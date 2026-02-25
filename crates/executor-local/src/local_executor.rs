use executor_core::config::ExecutorConfig;
use executor_core::error::ExecutorError;
use executor_core::metadata::{metadata_dir, TaskMetadata};
use executor_core::task::{TaskId, TaskPayload, TaskRequest, TaskStatus};
use executor_core::Executor;
use std::path::PathBuf;
use tokio::process::Command;
use tracing::{debug, info, warn};

/// Local executor: runs claude or shell commands directly on the host machine.
pub struct LocalExecutor {
    config: ExecutorConfig,
}

impl LocalExecutor {
    pub fn new(config: ExecutorConfig) -> Self {
        Self { config }
    }

    fn local_meta_dir(&self) -> PathBuf {
        metadata_dir()
    }

    fn task_dir(&self, task_id: &TaskId) -> PathBuf {
        PathBuf::from("/tmp/openclaw-tasks").join(task_id.to_string())
    }
}

#[async_trait::async_trait]
impl Executor for LocalExecutor {
    fn name(&self) -> &str {
        &self.config.name
    }

    fn executor_type(&self) -> &str {
        "local"
    }

    async fn start(&self, request: TaskRequest) -> Result<TaskMetadata, ExecutorError> {
        let task_id = TaskId::new();
        let task_dir = self.task_dir(&task_id);
        std::fs::create_dir_all(&task_dir)?;

        let log_file = task_dir.join("claude.log");
        let pid_file = task_dir.join("claude.pid");

        let workspace = request.workspace.as_deref().unwrap_or(".");

        // Build env var prefix from config.env (exported before the command)
        let env_prefix: String = self
            .config
            .env
            .iter()
            .map(|(k, v)| format!("{}='{}' ", k, v.replace('\'', "'\\''")))
            .collect();

        let shell_cmd = match &request.payload {
            TaskPayload::ClaudeCode {
                prompt,
                max_turns,
                allowed_tools,
            } => {
                let claude_bin = self.config.claude_binary();
                let mut claude_args = format!(
                    "{} --print --output-format json -p {}",
                    claude_bin,
                    shell_escape(prompt)
                );

                if let Some(turns) = max_turns {
                    claude_args.push_str(&format!(" --max-turns {}", turns));
                }

                for tool in allowed_tools {
                    claude_args.push_str(&format!(" --allowedTools {}", shell_escape(tool)));
                }

                format!(
                    "cd {} && nohup {}{}> {} 2>&1 & echo $! > {}",
                    shell_escape(workspace),
                    env_prefix,
                    claude_args,
                    log_file.display(),
                    pid_file.display(),
                )
            }
            TaskPayload::ShellCommand { command } => {
                format!(
                    "cd {} && nohup {}sh -c {} > {} 2>&1 & echo $! > {}",
                    shell_escape(workspace),
                    env_prefix,
                    shell_escape(command),
                    log_file.display(),
                    pid_file.display(),
                )
            }
        };

        debug!("Local exec: {}", shell_cmd);

        Command::new("sh")
            .arg("-c")
            .arg(&shell_cmd)
            .output()
            .await
            .map_err(|e| ExecutorError::Process(format!("Failed to spawn: {}", e)))?;

        // Read PID
        let pid_str = tokio::fs::read_to_string(&pid_file)
            .await
            .map_err(|e| ExecutorError::Process(format!("Failed to read PID file: {}", e)))?;
        let pid: u32 = pid_str
            .trim()
            .parse()
            .map_err(|_| ExecutorError::Process(format!("Invalid PID: '{}'", pid_str.trim())))?;

        info!("Task {} started locally with PID {}", task_id, pid);

        let mut meta = TaskMetadata::new(
            task_id.clone(),
            self.config.name.clone(),
            "local".to_string(),
            request.payload.type_str().to_string(),
            request.payload.description().to_string(),
            request.workspace,
        );
        meta.mark_running(pid);

        let meta_dir = self.local_meta_dir();
        std::fs::create_dir_all(&meta_dir)?;
        meta.write_to_dir(&meta_dir)?;

        Ok(meta)
    }

    async fn status(&self, task_id: &TaskId) -> Result<TaskMetadata, ExecutorError> {
        let meta_dir = self.local_meta_dir();
        let meta_path = meta_dir.join(format!("{}.meta.json", task_id));

        let mut meta = if meta_path.exists() {
            TaskMetadata::read_from_file(&meta_path)?
        } else {
            return Err(ExecutorError::TaskNotFound(task_id.to_string()));
        };

        if meta.status == TaskStatus::Running {
            if let Some(pid) = meta.pid {
                // Check if process is alive
                let output = Command::new("kill")
                    .args(["-0", &pid.to_string()])
                    .output()
                    .await;

                match output {
                    Ok(o) if !o.status.success() => {
                        // Process no longer running
                        meta.mark_completed(0);
                        meta.write_to_dir(&meta_dir)?;
                    }
                    Err(_) => {
                        meta.mark_completed(1);
                        meta.write_to_dir(&meta_dir)?;
                    }
                    _ => {} // still running
                }
            }
        }

        Ok(meta)
    }

    async fn logs(&self, task_id: &TaskId, lines: usize) -> Result<Vec<String>, ExecutorError> {
        let task_dir = self.task_dir(task_id);
        let log_file = task_dir.join("claude.log");

        if !log_file.exists() {
            return Ok(Vec::new());
        }

        let output = Command::new("tail")
            .args(["-n", &lines.to_string()])
            .arg(&log_file)
            .output()
            .await
            .map_err(|e| ExecutorError::Process(format!("tail failed: {}", e)))?;

        let text = String::from_utf8_lossy(&output.stdout);
        Ok(text.lines().map(|l| l.to_string()).collect())
    }

    async fn kill(&self, task_id: &TaskId) -> Result<(), ExecutorError> {
        let meta_dir = self.local_meta_dir();
        let meta_path = meta_dir.join(format!("{}.meta.json", task_id));

        let mut meta = if meta_path.exists() {
            TaskMetadata::read_from_file(&meta_path)?
        } else {
            return Err(ExecutorError::TaskNotFound(task_id.to_string()));
        };

        if let Some(pid) = meta.pid {
            warn!("Killing local task {} (PID {})", task_id, pid);
            let _ = Command::new("kill")
                .arg(pid.to_string())
                .output()
                .await;

            meta.mark_killed();
            meta.write_to_dir(&meta_dir)?;
        }

        Ok(())
    }

    async fn cleanup(&self, task_id: &TaskId) -> Result<(), ExecutorError> {
        let task_dir = self.task_dir(task_id);
        if task_dir.exists() {
            info!("Cleaning up local task dir: {}", task_dir.display());
            std::fs::remove_dir_all(task_dir)?;
        }

        let meta_path = self
            .local_meta_dir()
            .join(format!("{}.meta.json", task_id));
        if meta_path.exists() {
            std::fs::remove_file(meta_path)?;
        }

        Ok(())
    }
}

fn shell_escape(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}
