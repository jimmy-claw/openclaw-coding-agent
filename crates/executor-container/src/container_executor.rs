use executor_core::config::{ContainerRuntime, ExecutorConfig};
use executor_core::error::ExecutorError;
use executor_core::metadata::{metadata_dir, TaskMetadata};
use executor_core::task::{TaskId, TaskRequest, TaskStatus};
use executor_core::Executor;
use std::path::PathBuf;
use tokio::process::Command;
use tracing::{debug, info, warn};

/// Container executor: runs claude in Docker/Podman containers.
/// Covers GitHub issue #2.
pub struct ContainerExecutor {
    config: ExecutorConfig,
}

impl ContainerExecutor {
    pub fn new(config: ExecutorConfig) -> Self {
        Self { config }
    }

    /// Get the container runtime command ("docker" or "podman").
    fn runtime_cmd(&self) -> &str {
        match self.config.runtime.as_ref().unwrap_or(&ContainerRuntime::Docker) {
            ContainerRuntime::Docker => "docker",
            ContainerRuntime::Podman => "podman",
        }
    }

    /// Container name for a given task.
    fn container_name(&self, task_id: &TaskId) -> String {
        format!("openclaw-{}-{}", self.config.name, &task_id.0[..8])
    }

    fn local_meta_dir(&self) -> PathBuf {
        metadata_dir()
    }

    /// Run a container runtime command and return stdout.
    async fn run_cmd(&self, args: &[&str]) -> Result<String, ExecutorError> {
        let runtime = self.runtime_cmd();
        debug!("Running: {} {}", runtime, args.join(" "));

        let output = Command::new(runtime)
            .args(args)
            .output()
            .await
            .map_err(|e| {
                ExecutorError::ContainerRuntime(format!("Failed to run {}: {}", runtime, e))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ExecutorError::ContainerRuntime(format!(
                "{} {} failed: {}",
                runtime,
                args.first().unwrap_or(&""),
                stderr.trim()
            )));
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }
}

#[async_trait::async_trait]
impl Executor for ContainerExecutor {
    fn name(&self) -> &str {
        &self.config.name
    }

    fn executor_type(&self) -> &str {
        "container"
    }

    async fn start(&self, request: TaskRequest) -> Result<TaskMetadata, ExecutorError> {
        let task_id = TaskId::new();
        let container_name = self.container_name(&task_id);
        let image = self
            .config
            .image
            .as_deref()
            .ok_or_else(|| ExecutorError::Config("Container executor requires 'image'".into()))?;

        let claude_bin = self.config.claude_binary();

        // Build docker/podman run command
        let mut args: Vec<String> = vec![
            "run".to_string(),
            "-d".to_string(),
            "--name".to_string(),
            container_name.clone(),
        ];

        // Mount volumes
        for vol in &self.config.volumes {
            args.push("-v".to_string());
            args.push(vol.clone());
        }

        // Set environment variables
        for (key, val) in &self.config.env {
            args.push("-e".to_string());
            args.push(format!("{}={}", key, val));
        }

        // Set workspace directory
        if let Some(ref workspace) = request.workspace {
            args.push("-w".to_string());
            args.push(workspace.clone());
        }

        args.push(image.to_string());

        // Build the claude command inside the container
        let mut claude_cmd = format!(
            "{} --print --output-format json -p {}",
            claude_bin,
            shell_escape(&request.prompt)
        );

        if let Some(max_turns) = request.max_turns {
            claude_cmd.push_str(&format!(" --max-turns {}", max_turns));
        }

        for tool in &request.allowed_tools {
            claude_cmd.push_str(&format!(" --allowedTools {}", shell_escape(tool)));
        }

        args.push("sh".to_string());
        args.push("-c".to_string());
        args.push(claude_cmd);

        let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        let container_id = self.run_cmd(&args_refs).await?;

        info!(
            "Task {} started in container {} ({})",
            task_id, container_name, &container_id[..12]
        );

        // Get the PID of the main process inside the container
        let pid_str = self
            .run_cmd(&["inspect", "--format", "{{.State.Pid}}", &container_name])
            .await
            .unwrap_or_else(|_| "0".to_string());
        let pid: u32 = pid_str.trim().parse().unwrap_or(0);

        let mut meta = TaskMetadata::new(
            task_id.clone(),
            self.config.name.clone(),
            "container".to_string(),
            request.prompt,
            request.workspace,
        );
        meta.mark_running(pid);

        let local_dir = self.local_meta_dir();
        std::fs::create_dir_all(&local_dir)?;
        meta.write_to_dir(&local_dir)?;

        Ok(meta)
    }

    async fn status(&self, task_id: &TaskId) -> Result<TaskMetadata, ExecutorError> {
        let local_dir = self.local_meta_dir();
        let local_path = local_dir.join(format!("{}.meta.json", task_id));

        let mut meta = if local_path.exists() {
            TaskMetadata::read_from_file(&local_path)?
        } else {
            return Err(ExecutorError::TaskNotFound(task_id.to_string()));
        };

        if meta.status == TaskStatus::Running {
            let container_name = self.container_name(task_id);
            let state = self
                .run_cmd(&["inspect", "--format", "{{.State.Status}}", &container_name])
                .await
                .unwrap_or_else(|_| "unknown".to_string());

            match state.trim() {
                "running" => {} // still running
                "exited" => {
                    let exit_str = self
                        .run_cmd(&[
                            "inspect",
                            "--format",
                            "{{.State.ExitCode}}",
                            &container_name,
                        ])
                        .await
                        .unwrap_or_else(|_| "1".to_string());
                    let exit_code: i32 = exit_str.trim().parse().unwrap_or(1);
                    meta.mark_completed(exit_code);
                    meta.write_to_dir(&local_dir)?;
                }
                _ => {
                    meta.mark_failed(format!("Container in unexpected state: {}", state.trim()));
                    meta.write_to_dir(&local_dir)?;
                }
            }
        }

        Ok(meta)
    }

    async fn logs(&self, task_id: &TaskId, lines: usize) -> Result<Vec<String>, ExecutorError> {
        let container_name = self.container_name(task_id);
        let output = self
            .run_cmd(&["logs", "--tail", &lines.to_string(), &container_name])
            .await?;

        Ok(output.lines().map(|l| l.to_string()).collect())
    }

    async fn kill(&self, task_id: &TaskId) -> Result<(), ExecutorError> {
        let container_name = self.container_name(task_id);
        warn!("Killing container {} for task {}", container_name, task_id);
        self.run_cmd(&["kill", &container_name]).await?;

        let local_dir = self.local_meta_dir();
        let local_path = local_dir.join(format!("{}.meta.json", task_id));
        if local_path.exists() {
            let mut meta = TaskMetadata::read_from_file(&local_path)?;
            meta.mark_killed();
            meta.write_to_dir(&local_dir)?;
        }

        Ok(())
    }

    async fn cleanup(&self, task_id: &TaskId) -> Result<(), ExecutorError> {
        let container_name = self.container_name(task_id);
        info!("Cleaning up container {} for task {}", container_name, task_id);

        // Stop + remove, ignore errors if already stopped/removed
        let _ = self.run_cmd(&["rm", "-f", &container_name]).await;

        let local_path = self
            .local_meta_dir()
            .join(format!("{}.meta.json", task_id));
        if local_path.exists() {
            std::fs::remove_file(local_path)?;
        }

        Ok(())
    }
}

fn shell_escape(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}
