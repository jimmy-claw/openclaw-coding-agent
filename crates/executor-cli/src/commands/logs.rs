use crate::dispatch;
use executor_core::config::Config;
use executor_core::metadata::{metadata_dir, TaskMetadata};
use executor_core::task::TaskId;

pub async fn run(
    config: &Config,
    task_id_str: &str,
    lines: usize,
    follow: Option<u64>,
) -> anyhow::Result<()> {
    let task_id = TaskId::from_string(task_id_str.to_string());
    let meta = load_local_meta(&task_id)?;
    let executor = dispatch::create_executor(config, &meta.executor_name)?;

    match follow {
        Some(interval) => {
            // Poll mode
            loop {
                let log_lines = executor.logs(&task_id, lines).await?;
                // Clear screen and print
                print!("\x1B[2J\x1B[H");
                for line in &log_lines {
                    println!("{}", line);
                }
                tokio::time::sleep(std::time::Duration::from_secs(interval)).await;
            }
        }
        None => {
            let log_lines = executor.logs(&task_id, lines).await?;
            for line in &log_lines {
                println!("{}", line);
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
