use thiserror::Error;

#[derive(Error, Debug)]
pub enum ExecutorError {
    #[error("SSH connection failed: {0}")]
    SshConnection(String),

    #[error("SSH command failed: {0}")]
    SshCommand(String),

    #[error("Container runtime error: {0}")]
    ContainerRuntime(String),

    #[error("Task not found: {0}")]
    TaskNotFound(String),

    #[error("Task already running: {0}")]
    TaskAlreadyRunning(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Process error: {0}")]
    Process(String),

    #[error("Executor not found: {0}")]
    ExecutorNotFound(String),
}
