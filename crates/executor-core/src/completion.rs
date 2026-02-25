use crate::metadata::TaskMetadata;
use crate::task::TaskStatus;
use std::path::PathBuf;

/// Directory for completion records: ~/.openclaw-agent/completions/
pub fn completions_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join(".openclaw-agent")
        .join("completions")
}

/// Write a completion record JSON file for a finished task.
/// Returns Ok(true) if written, Ok(false) if already exists or not terminal.
pub fn write_completion_record(meta: &TaskMetadata) -> Result<bool, std::io::Error> {
    if !meta.status.is_terminal() {
        return Ok(false);
    }

    let dir = completions_dir();
    let path = dir.join(format!("{}.json", meta.task_id));

    if path.exists() {
        return Ok(false);
    }

    std::fs::create_dir_all(&dir)?;

    let status_str = match meta.status {
        TaskStatus::Completed => "success",
        _ => "failure",
    };

    let record = serde_json::json!({
        "task_id": meta.task_id.0,
        "status": status_str,
        "exit_code": meta.exit_code.unwrap_or(-1),
        "completed_at": meta.finished_at.map(|t| t.to_rfc3339()).unwrap_or_default(),
        "executor": meta.executor_name,
    });

    let json = serde_json::to_string_pretty(&record)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    std::fs::write(&path, json)?;
    Ok(true)
}

/// POST the completion record to a webhook URL using curl.
/// Runs asynchronously via tokio::process::Command.
pub async fn post_webhook(meta: &TaskMetadata, webhook_url: &str) -> Result<(), String> {
    if !meta.status.is_terminal() {
        return Ok(());
    }

    let status_str = match meta.status {
        TaskStatus::Completed => "success",
        _ => "failure",
    };

    let record = serde_json::json!({
        "task_id": meta.task_id.0,
        "status": status_str,
        "exit_code": meta.exit_code.unwrap_or(-1),
        "completed_at": meta.finished_at.map(|t| t.to_rfc3339()).unwrap_or_default(),
        "executor": meta.executor_name,
    });

    let body = serde_json::to_string(&record).map_err(|e| e.to_string())?;

    let output = tokio::process::Command::new("curl")
        .args([
            "-s",
            "-X",
            "POST",
            "-H",
            "Content-Type: application/json",
            "-d",
            &body,
            "--max-time",
            "10",
            webhook_url,
        ])
        .output()
        .await
        .map_err(|e| format!("Failed to run curl: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Webhook POST failed: {}", stderr));
    }

    Ok(())
}
