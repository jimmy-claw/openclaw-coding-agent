use crate::dispatch;
use executor_core::config::Config;
use executor_core::task::TaskRequest;

pub async fn run(
    config: &Config,
    executor_name: &str,
    prompt: String,
    workspace: Option<String>,
    max_turns: Option<u32>,
    allowed_tools: Vec<String>,
) -> anyhow::Result<()> {
    let executor = dispatch::create_executor(config, executor_name)?;

    let request = TaskRequest {
        prompt,
        workspace,
        max_turns,
        allowed_tools,
    };

    let meta = executor.start(request).await?;

    println!("Task started:");
    println!("  ID:       {}", meta.task_id);
    println!("  Executor: {} ({})", meta.executor_name, meta.executor_type);
    println!("  PID:      {}", meta.pid.map(|p| p.to_string()).unwrap_or_else(|| "N/A".into()));
    println!("  Status:   {}", meta.status);

    Ok(())
}
