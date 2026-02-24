use executor_core::metadata::list_all_metadata;
use executor_core::task::TaskStatus;

pub async fn run(
    json: bool,
    jsonl: bool,
    status_filter: Option<String>,
    executor_filter: Option<String>,
) -> anyhow::Result<()> {
    let mut tasks = list_all_metadata()?;

    // Apply filters
    if let Some(ref status_str) = status_filter {
        let target = parse_status(status_str);
        tasks.retain(|t| t.status == target);
    }
    if let Some(ref exec_name) = executor_filter {
        tasks.retain(|t| t.executor_name == *exec_name);
    }

    if jsonl {
        // JSONL output for dashboard streaming (issue #4)
        for task in &tasks {
            println!("{}", task.to_jsonl_line());
        }
    } else if json {
        let json_tasks: Vec<_> = tasks.iter().map(|t| t.to_dashboard_json()).collect();
        println!("{}", serde_json::to_string_pretty(&json_tasks)?);
    } else {
        if tasks.is_empty() {
            println!("No tasks found.");
            return Ok(());
        }
        println!(
            "{:<38} {:<12} {:<12} {:<10} {:<8}",
            "TASK ID", "EXECUTOR", "TYPE", "STATUS", "PID"
        );
        println!("{}", "-".repeat(80));
        for task in &tasks {
            println!(
                "{:<38} {:<12} {:<12} {:<10} {:<8}",
                task.task_id,
                task.executor_name,
                task.executor_type,
                task.status,
                task.pid.map(|p| p.to_string()).unwrap_or_else(|| "-".into()),
            );
        }
    }

    Ok(())
}

fn parse_status(s: &str) -> TaskStatus {
    match s.to_lowercase().as_str() {
        "starting" => TaskStatus::Starting,
        "running" => TaskStatus::Running,
        "completed" => TaskStatus::Completed,
        "failed" => TaskStatus::Failed,
        "killed" => TaskStatus::Killed,
        _ => TaskStatus::Unknown,
    }
}
