use crate::dispatch;
use executor_core::config::Config;
use executor_core::task::{TaskPayload, TaskRequest};

pub async fn run(
    config: &Config,
    executor_name: &str,
    prompt: String,
    workspace: Option<String>,
    max_turns: Option<u32>,
    allowed_tools: Vec<String>,
    detach: bool,
) -> anyhow::Result<()> {
    let executor = dispatch::create_executor(config, executor_name)?;

    let request = TaskRequest {
        payload: TaskPayload::ClaudeCode {
            prompt,
            max_turns,
            allowed_tools,
        },
        workspace,
        detach,
    };

    let meta = executor.start(request).await?;

    println!("{} Task started:", meta.task_icon());
    println!("  ID:       {}", meta.task_id);
    println!("  Type:     {}", meta.task_type);
    println!("  Executor: {} ({})", meta.executor_name, meta.executor_type);
    println!("  PID:      {}", meta.pid.map(|p| p.to_string()).unwrap_or_else(|| "N/A".into()));
    println!("  Status:   {}", meta.status);
    if detach {
        println!("  Mode:     detached (fire-and-forget)");
    }

    Ok(())
}
