use crate::task::{TaskId, TaskStatus};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::Path;

fn default_task_type() -> String {
    "claude_code".to_string()
}

fn default_heartbeat_interval() -> u64 {
    30
}

/// Task metadata stored as .meta.json alongside task artifacts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskMetadata {
    pub task_id: TaskId,
    pub executor_name: String,
    pub executor_type: String,
    #[serde(default = "default_task_type")]
    pub task_type: String,
    pub pid: Option<u32>,
    pub status: TaskStatus,
    pub prompt: String,
    pub workspace: Option<String>,
    pub started_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub exit_code: Option<i32>,
    pub error: Option<String>,
    pub last_heartbeat: Option<u64>,
    #[serde(default = "default_heartbeat_interval")]
    pub heartbeat_interval: Option<u64>,
}

impl TaskMetadata {
    pub fn new(
        task_id: TaskId,
        executor_name: String,
        executor_type: String,
        task_type: String,
        prompt: String,
        workspace: Option<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            task_id,
            executor_name,
            executor_type,
            task_type,
            pid: None,
            status: TaskStatus::Starting,
            prompt,
            workspace,
            started_at: now,
            updated_at: now,
            finished_at: None,
            exit_code: None,
            error: None,
            last_heartbeat: None,
            heartbeat_interval: Some(default_heartbeat_interval()),
        }
    }

    pub fn mark_running(&mut self, pid: u32) {
        self.pid = Some(pid);
        self.status = TaskStatus::Running;
        self.updated_at = Utc::now();
    }

    pub fn mark_completed(&mut self, exit_code: i32) {
        let now = Utc::now();
        self.status = if exit_code == 0 {
            TaskStatus::Completed
        } else {
            TaskStatus::Failed
        };
        self.exit_code = Some(exit_code);
        self.finished_at = Some(now);
        self.updated_at = now;
    }

    pub fn mark_killed(&mut self) {
        let now = Utc::now();
        self.status = TaskStatus::Killed;
        self.finished_at = Some(now);
        self.updated_at = now;
    }

    pub fn mark_failed(&mut self, error: String) {
        let now = Utc::now();
        self.status = TaskStatus::Failed;
        self.error = Some(error);
        self.finished_at = Some(now);
        self.updated_at = now;
    }

    pub fn mark_heartbeat(&mut self) {
        self.last_heartbeat = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        );
        self.updated_at = Utc::now();
    }

    /// Write metadata to a .meta.json file in the given directory.
    pub fn write_to_dir(&self, dir: &Path) -> Result<(), std::io::Error> {
        let path = dir.join(format!("{}.meta.json", self.task_id.0));
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        std::fs::write(path, json)
    }

    /// Read metadata from a .meta.json file.
    pub fn read_from_file(path: &Path) -> Result<Self, std::io::Error> {
        let data = std::fs::read_to_string(path)?;
        serde_json::from_str(&data)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    /// Produce structured JSON for dashboard integration.
    pub fn to_dashboard_json(&self) -> serde_json::Value {
        serde_json::json!({
            "task_id": self.task_id.0,
            "executor": self.executor_name,
            "executor_type": self.executor_type,
            "task_type": self.task_type,
            "status": self.status,
            "pid": self.pid,
            "started_at": self.started_at.to_rfc3339(),
            "updated_at": self.updated_at.to_rfc3339(),
            "finished_at": self.finished_at.map(|t| t.to_rfc3339()),
            "exit_code": self.exit_code,
            "error": self.error,
            "last_heartbeat": self.last_heartbeat,
            "heartbeat_interval": self.heartbeat_interval,
        })
    }

    /// Produce a JSONL line for dashboard streaming.
    pub fn to_jsonl_line(&self) -> String {
        serde_json::to_string(&self.to_dashboard_json()).unwrap_or_default()
    }

    /// Icon for display based on task type.
    pub fn task_icon(&self) -> &str {
        match self.task_type.as_str() {
            "shell_command" => "\u{2699}\u{FE0F}",
            _ => "\u{1F916}",
        }
    }
}

/// Get the default metadata storage directory.
pub fn metadata_dir() -> std::path::PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
        .join("openclaw")
        .join("tasks")
}

/// List all task metadata files in the metadata directory.
pub fn list_all_metadata() -> Result<Vec<TaskMetadata>, std::io::Error> {
    let dir = metadata_dir();
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut results = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "json")
            && path
                .file_name()
                .is_some_and(|n| n.to_string_lossy().ends_with(".meta.json"))
        {
            if let Ok(meta) = TaskMetadata::read_from_file(&path) {
                results.push(meta);
            }
        }
    }
    results.sort_by(|a, b| b.started_at.cmp(&a.started_at));
    Ok(results)
}

/// Clean up tasks that are stuck in Running state due to SSH disconnection
/// (no heartbeat updates for > 5 minutes). Updates their status to HeartbeatTimeout.
pub fn cleanup_stale_tasks() -> Result<Vec<TaskId>, std::io::Error> {
    use std::time::{SystemTime, UNIX_EPOCH};
    
    let metadata_dir = metadata_dir();
    if !metadata_dir.exists() {
        return Ok(Vec::new());
    }
    
    let mut stale_tasks = Vec::new();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    
    for entry in std::fs::read_dir(metadata_dir)? {
        let entry = entry?;
        let path = entry.path();
        
        if !path.extension().is_some_and(|ext| ext == "json") {
            continue;
        }
        
        if path.file_name().is_some_and(|n| !n.to_string_lossy().ends_with(".meta.json")) {
            continue;
        }
        
        // Read metadata
        let mut meta = match TaskMetadata::read_from_file(&path) {
            Ok(m) => m,
            Err(_) => continue,
        };
        
        // Only check Running tasks with heartbeat interval set
        if meta.status == executor_core::task::TaskStatus::Running {
            if let Some(interval) = meta.heartbeat_interval {
                let stale_after = interval * 10; // 10 intervals = 5 minutes
                
                // If no heartbeat recorded, or last heartbeat is stale
                let is_stale = match meta.last_heartbeat {
                    None => false, // Haven't started heartbeat yet
                    Some(last_heartbeat) => now - last_heartbeat > stale_after,
                };
                
                if is_stale {
                    warn!("Marking task {} as heartbeat_timeout (stale for {}s)", meta.task_id, now - last_heartbeat.unwrap_or(now));
                    meta.status = executor_core::task::TaskStatus::HeartbeatTimeout;
                    meta.updated_at = Utc::now();
                    
                    if let Err(e) = meta.write_to_dir(&path.parent().unwrap()) {
                        warn!("Failed to update task {}: {}", meta.task_id, e);
                        continue;
                    }
                    
                    stale_tasks.push(meta.task_id);
                }
            }
        }
    }
    
    Ok(stale_tasks)
}
