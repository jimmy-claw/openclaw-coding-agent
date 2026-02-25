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

/// Payload type: either a Claude Code prompt or an arbitrary shell command.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TaskPayload {
    ClaudeCode {
        prompt: String,
        max_turns: Option<u32>,
        #[serde(default)]
        allowed_tools: Vec<String>,
    },
    ShellCommand {
        command: String,
    },
}

impl TaskPayload {
    /// Human-readable description (the prompt or command).
    pub fn description(&self) -> &str {
        match self {
            TaskPayload::ClaudeCode { prompt, .. } => prompt,
            TaskPayload::ShellCommand { command } => command,
        }
    }

    /// Type identifier string.
    pub fn type_str(&self) -> &str {
        match self {
            TaskPayload::ClaudeCode { .. } => "claude_code",
            TaskPayload::ShellCommand { .. } => "shell_command",
        }
    }

    /// Icon for display.
    pub fn icon(&self) -> &str {
        match self {
            TaskPayload::ClaudeCode { .. } => "\u{1F916}",
            TaskPayload::ShellCommand { .. } => "\u{2699}\u{FE0F}",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRequest {
    pub payload: TaskPayload,
    pub workspace: Option<String>,
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

impl TaskStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(self, TaskStatus::Completed | TaskStatus::Failed | TaskStatus::Killed)
    }
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
