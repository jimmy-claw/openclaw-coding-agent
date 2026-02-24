use executor_core::metadata::list_all_metadata;

/// Dashboard integration command. Covers GitHub issue #4.
/// Outputs structured JSON/JSONL for external dashboard consumption.
pub async fn run(stream: bool, watch: Option<u64>) -> anyhow::Result<()> {
    match watch {
        Some(interval) => {
            // Watch mode: continuously output status
            loop {
                output_dashboard(stream)?;
                tokio::time::sleep(std::time::Duration::from_secs(interval)).await;
            }
        }
        None => {
            output_dashboard(stream)?;
        }
    }

    Ok(())
}

fn output_dashboard(stream: bool) -> anyhow::Result<()> {
    let tasks = list_all_metadata()?;

    if stream {
        // JSONL: one line per task
        for task in &tasks {
            println!("{}", task.to_jsonl_line());
        }
    } else {
        // Full JSON array
        let dashboard: Vec<serde_json::Value> = tasks
            .iter()
            .map(|t| t.to_dashboard_json())
            .collect();

        let output = serde_json::json!({
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "task_count": tasks.len(),
            "running": tasks.iter().filter(|t| t.status == executor_core::task::TaskStatus::Running).count(),
            "completed": tasks.iter().filter(|t| t.status == executor_core::task::TaskStatus::Completed).count(),
            "failed": tasks.iter().filter(|t| t.status == executor_core::task::TaskStatus::Failed).count(),
            "tasks": dashboard,
        });

        println!("{}", serde_json::to_string_pretty(&output)?);
    }

    Ok(())
}
