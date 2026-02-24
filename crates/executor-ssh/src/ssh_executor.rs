use executor_core::config::ExecutorConfig;
use executor_core::error::ExecutorError;
use executor_core::metadata::{metadata_dir, TaskMetadata};
use executor_core::task::{TaskId, TaskRequest, TaskStatus};
use executor_core::Executor;
use ssh2::Session;
use std::io::Read;
use std::net::TcpStream;
use std::path::PathBuf;
use tracing::{debug, info, warn};

/// SSH executor: connects to a remote host, runs `claude` via nohup,
/// tracks PID, and tails logs. Covers GitHub issue #1.
pub struct SshExecutor {
    config: ExecutorConfig,
}

impl SshExecutor {
    pub fn new(config: ExecutorConfig) -> Self {
        Self { config }
    }

    /// Establish an SSH session to the configured host.
    fn connect(&self) -> Result<Session, ExecutorError> {
        let host = self
            .config
            .host
            .as_deref()
            .ok_or_else(|| ExecutorError::Config("SSH executor requires 'host'".into()))?;
        let user = self
            .config
            .user
            .as_deref()
            .ok_or_else(|| ExecutorError::Config("SSH executor requires 'user'".into()))?;
        let port = self.config.ssh_port();

        debug!("Connecting to {}@{}:{}", user, host, port);
        let tcp = TcpStream::connect(format!("{}:{}", host, port))
            .map_err(|e| ExecutorError::SshConnection(format!("TCP connect to {}:{}: {}", host, port, e)))?;

        let mut sess = Session::new()
            .map_err(|e| ExecutorError::SshConnection(format!("Session::new: {}", e)))?;
        sess.set_tcp_stream(tcp);
        sess.handshake()
            .map_err(|e| ExecutorError::SshConnection(format!("Handshake: {}", e)))?;

        // Try key-based auth first
        if let Some(key_path) = &self.config.key_path {
            sess.userauth_pubkey_file(user, None, std::path::Path::new(key_path), None)
                .map_err(|e| ExecutorError::SshConnection(format!("Pubkey auth: {}", e)))?;
        } else {
            // Try SSH agent
            sess.userauth_agent(user)
                .map_err(|e| ExecutorError::SshConnection(format!("Agent auth: {}", e)))?;
        }

        if !sess.authenticated() {
            return Err(ExecutorError::SshConnection("Authentication failed".into()));
        }

        info!("SSH connected to {}@{}:{}", user, host, port);
        Ok(sess)
    }

    /// Execute a command on the remote host and return stdout.
    fn exec_remote(&self, sess: &Session, cmd: &str) -> Result<String, ExecutorError> {
        debug!("Remote exec: {}", cmd);
        let mut channel = sess
            .channel_session()
            .map_err(|e| ExecutorError::SshCommand(format!("Channel: {}", e)))?;
        channel
            .exec(cmd)
            .map_err(|e| ExecutorError::SshCommand(format!("Exec '{}': {}", cmd, e)))?;

        let mut output = String::new();
        channel
            .read_to_string(&mut output)
            .map_err(|e| ExecutorError::SshCommand(format!("Read output: {}", e)))?;

        let mut stderr = String::new();
        channel
            .stderr()
            .read_to_string(&mut stderr)
            .map_err(|e| ExecutorError::SshCommand(format!("Read stderr: {}", e)))?;

        channel.wait_close().ok();
        let exit_status = channel.exit_status().unwrap_or(-1);

        if exit_status != 0 && !stderr.is_empty() {
            debug!("Remote command stderr: {}", stderr.trim());
        }

        Ok(output)
    }

    /// Remote directory for task metadata/logs.
    fn remote_task_dir(&self, task_id: &TaskId) -> String {
        format!("/tmp/openclaw-tasks/{}", task_id)
    }

    /// Local metadata directory for this task.
    fn local_meta_dir(&self) -> PathBuf {
        metadata_dir()
    }
}

#[async_trait::async_trait]
impl Executor for SshExecutor {
    fn name(&self) -> &str {
        &self.config.name
    }

    fn executor_type(&self) -> &str {
        "ssh"
    }

    async fn start(&self, request: TaskRequest) -> Result<TaskMetadata, ExecutorError> {
        let task_id = TaskId::new();
        let sess = self.connect()?;

        let task_dir = self.remote_task_dir(&task_id);
        self.exec_remote(&sess, &format!("mkdir -p {}", task_dir))?;

        // Build the claude command
        let claude_bin = self.config.claude_binary();
        let mut cmd_parts = vec![
            format!("cd {}", request.workspace.as_deref().unwrap_or("~")),
            "&&".to_string(),
            format!(
                "nohup {} --print --output-format json -p {}",
                claude_bin,
                shell_escape(&request.prompt)
            ),
        ];

        if let Some(max_turns) = request.max_turns {
            cmd_parts.push(format!("--max-turns {}", max_turns));
        }

        for tool in &request.allowed_tools {
            cmd_parts.push(format!("--allowedTools {}", shell_escape(tool)));
        }

        // Redirect output to log file, run in background
        let log_file = format!("{}/claude.log", task_dir);
        let pid_file = format!("{}/claude.pid", task_dir);
        cmd_parts.push(format!("> {} 2>&1 & echo $! > {}", log_file, pid_file));

        let full_cmd = cmd_parts.join(" ");
        info!("Starting task {} on {}: {}", task_id, self.name(), full_cmd);
        self.exec_remote(&sess, &full_cmd)?;

        // Read the PID
        let pid_str = self
            .exec_remote(&sess, &format!("cat {}", pid_file))?
            .trim()
            .to_string();
        let pid: u32 = pid_str
            .parse()
            .map_err(|_| ExecutorError::Process(format!("Invalid PID: '{}'", pid_str)))?;

        info!("Task {} started with PID {} on {}", task_id, pid, self.name());

        // Create and save metadata locally
        let mut meta = TaskMetadata::new(
            task_id.clone(),
            self.config.name.clone(),
            "ssh".to_string(),
            request.prompt,
            request.workspace,
        );
        meta.mark_running(pid);

        // Write .meta.json locally
        let local_dir = self.local_meta_dir();
        std::fs::create_dir_all(&local_dir)?;
        meta.write_to_dir(&local_dir)?;

        // Write .meta.json on remote too
        let meta_json = serde_json::to_string_pretty(&meta)
            .map_err(|e| ExecutorError::SshCommand(format!("Serialize meta: {}", e)))?;
        self.exec_remote(
            &sess,
            &format!(
                "cat > {}/{}.meta.json << 'METAEOF'\n{}\nMETAEOF",
                task_dir, task_id, meta_json
            ),
        )?;

        Ok(meta)
    }

    async fn status(&self, task_id: &TaskId) -> Result<TaskMetadata, ExecutorError> {
        // Try reading local metadata first
        let local_dir = self.local_meta_dir();
        let local_path = local_dir.join(format!("{}.meta.json", task_id));

        let mut meta = if local_path.exists() {
            TaskMetadata::read_from_file(&local_path)?
        } else {
            return Err(ExecutorError::TaskNotFound(task_id.to_string()));
        };

        // Check if the process is still running on remote
        if meta.status == TaskStatus::Running {
            if let Some(pid) = meta.pid {
                let sess = self.connect()?;
                let check = self.exec_remote(&sess, &format!("kill -0 {} 2>/dev/null && echo running || echo stopped", pid))?;
                let check = check.trim();

                if check == "stopped" {
                    // Process finished, check exit code
                    let exit_output = self
                        .exec_remote(&sess, &format!("wait {} 2>/dev/null; echo $?", pid))
                        .unwrap_or_else(|_| "1".to_string());
                    let exit_code: i32 = exit_output.trim().parse().unwrap_or(1);
                    meta.mark_completed(exit_code);

                    // Update local metadata
                    meta.write_to_dir(&local_dir)?;
                }
            }
        }

        Ok(meta)
    }

    async fn logs(&self, task_id: &TaskId, lines: usize) -> Result<Vec<String>, ExecutorError> {
        let sess = self.connect()?;
        let task_dir = self.remote_task_dir(task_id);
        let log_file = format!("{}/claude.log", task_dir);

        let output = self.exec_remote(&sess, &format!("tail -n {} {}", lines, log_file))?;

        Ok(output.lines().map(|l| l.to_string()).collect())
    }

    async fn kill(&self, task_id: &TaskId) -> Result<(), ExecutorError> {
        let local_dir = self.local_meta_dir();
        let local_path = local_dir.join(format!("{}.meta.json", task_id));

        let mut meta = if local_path.exists() {
            TaskMetadata::read_from_file(&local_path)?
        } else {
            return Err(ExecutorError::TaskNotFound(task_id.to_string()));
        };

        if let Some(pid) = meta.pid {
            let sess = self.connect()?;
            warn!("Killing task {} (PID {}) on {}", task_id, pid, self.name());
            self.exec_remote(&sess, &format!("kill {} 2>/dev/null || true", pid))?;

            meta.mark_killed();
            meta.write_to_dir(&local_dir)?;
        }

        Ok(())
    }

    async fn cleanup(&self, task_id: &TaskId) -> Result<(), ExecutorError> {
        let sess = self.connect()?;
        let task_dir = self.remote_task_dir(task_id);

        info!("Cleaning up task {} on {}", task_id, self.name());
        self.exec_remote(&sess, &format!("rm -rf {}", task_dir))?;

        // Remove local metadata
        let local_path = self
            .local_meta_dir()
            .join(format!("{}.meta.json", task_id));
        if local_path.exists() {
            std::fs::remove_file(local_path)?;
        }

        Ok(())
    }
}

/// Shell-escape a string for safe use in remote commands.
fn shell_escape(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}
