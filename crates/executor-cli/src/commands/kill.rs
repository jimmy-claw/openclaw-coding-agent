use crate::dispatch;
use executor_core::completion;
use executor_core::config::Config;
use executor_core::metadata::{metadata_dir, TaskMetadata};
use executor_core::task::TaskId;

pub async fn run(config: &Config, task_id_str: &str) -> anyhow::Result<()> {
    let task_id = TaskId::from_string(task_id_str.to_string());
    let meta = load_local_meta(&task_id)?;
    let executor = dispatch::create_executor(config, &meta.executor_name)?;

    executor.kill(&task_id).await?;
    println!("Task {} killed.", task_id);

    // Write completion record for the killed task
    let updated_meta = load_local_meta(&task_id)?;
    if updated_meta.status.is_terminal() {
        if let Ok(true) = completion::write_completion_record(&updated_meta) {
            if let Some(ref webhook_url) = config.defaults.webhook_url {
                if let Err(e) = completion::post_webhook(&updated_meta, webhook_url).await {
                    eprintln!("Warning: webhook POST failed: {}", e);
                }
            }
        }
    }

    Ok(())
}

fn load_local_meta(task_id: &TaskId) -> anyhow::Result<TaskMetadata> {
    let dir = metadata_dir();
    let path = dir.join(format!("{}.meta.json", task_id));
    if path.exists() {
        Ok(TaskMetadata::read_from_file(&path)?)
    } else {
        anyhow::bail!("No local metadata for task {}", task_id)
    }
}
