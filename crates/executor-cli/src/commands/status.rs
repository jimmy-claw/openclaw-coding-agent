use crate::dispatch;
use executor_core::completion;
use executor_core::config::Config;
use executor_core::metadata::TaskMetadata;
use executor_core::task::TaskId;

pub async fn run(config: &Config, task_id_str: &str, json: bool) -> anyhow::Result<()> {
    let task_id = TaskId::from_string(task_id_str.to_string());

    // Read local metadata to find the executor
    let meta = load_local_meta(&task_id)?;
    let executor_name = meta.executor_name.clone();

    let executor = dispatch::create_executor(config, &executor_name)?;
    let updated_meta = executor.status(&task_id).await?;

    // Write completion record if task reached a terminal state
    if updated_meta.status.is_terminal() {
        if let Ok(true) = completion::write_completion_record(&updated_meta) {
            // Fire webhook if configured
            if let Some(ref webhook_url) = config.defaults.webhook_url {
                if let Err(e) = completion::post_webhook(&updated_meta, webhook_url).await {
                    eprintln!("Warning: webhook POST failed: {}", e);
                }
            }
        }
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&updated_meta.to_dashboard_json())?);
    } else {
        print_status(&updated_meta);
    }

    Ok(())
}

fn print_status(meta: &TaskMetadata) {
    println!("{}  Task:     {}", meta.task_icon(), meta.task_id);
    println!("   Type:     {}", meta.task_type);
    println!("   Executor: {} ({})", meta.executor_name, meta.executor_type);
    println!("   Status:   {}", meta.status);
    println!("   PID:      {}", meta.pid.map(|p| p.to_string()).unwrap_or_else(|| "N/A".into()));
    println!("   Started:  {}", meta.started_at);
    println!("   Updated:  {}", meta.updated_at);
    if let Some(finished) = meta.finished_at {
        println!("   Finished: {}", finished);
    }
    if let Some(code) = meta.exit_code {
        println!("   Exit:     {}", code);
    }
    if let Some(ref err) = meta.error {
        println!("   Error:    {}", err);
    }
}

fn load_local_meta(task_id: &TaskId) -> anyhow::Result<TaskMetadata> {
    let dir = executor_core::metadata::metadata_dir();
    let path = dir.join(format!("{}.meta.json", task_id));
    if path.exists() {
        Ok(TaskMetadata::read_from_file(&path)?)
    } else {
        anyhow::bail!("No local metadata for task {}", task_id)
    }
}
