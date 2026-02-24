use crate::error::ExecutorError;
use crate::metadata::TaskMetadata;
use crate::task::{TaskId, TaskRequest};

/// Core executor trait. Every executor type implements this.
/// Maps to the interface described in the README:
///   start, status, logs (fetch_logs), kill, cleanup
#[async_trait::async_trait]
pub trait Executor: Send + Sync {
    /// Name of this executor instance (from config).
    fn name(&self) -> &str;

    /// Executor type string ("ssh", "container", "local").
    fn executor_type(&self) -> &str;

    /// Start a task. Returns task ID and initial metadata.
    async fn start(&self, request: TaskRequest) -> Result<TaskMetadata, ExecutorError>;

    /// Get current status/metadata for a task.
    async fn status(&self, task_id: &TaskId) -> Result<TaskMetadata, ExecutorError>;

    /// Fetch recent log lines from the task.
    async fn logs(&self, task_id: &TaskId, lines: usize) -> Result<Vec<String>, ExecutorError>;

    /// Kill a running task.
    async fn kill(&self, task_id: &TaskId) -> Result<(), ExecutorError>;

    /// Cleanup task artifacts (containers, temp files, etc.).
    async fn cleanup(&self, task_id: &TaskId) -> Result<(), ExecutorError>;
}
