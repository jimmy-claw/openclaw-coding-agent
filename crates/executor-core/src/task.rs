use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct TaskId(pub String);

impl TaskId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    pub fn from_string(s: String) -> Self {
        Self(s)
    }
}

impl fmt::Display for TaskId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Default for TaskId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRequest {
    pub prompt: String,
    pub workspace: Option<String>,
    pub max_turns: Option<u32>,
    pub allowed_tools: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Starting,
    Running,
    Completed,
    Failed,
    Killed,
    Unknown,
}

impl fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TaskStatus::Starting => write!(f, "starting"),
            TaskStatus::Running => write!(f, "running"),
            TaskStatus::Completed => write!(f, "completed"),
            TaskStatus::Failed => write!(f, "failed"),
            TaskStatus::Killed => write!(f, "killed"),
            TaskStatus::Unknown => write!(f, "unknown"),
        }
    }
}
