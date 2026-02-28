use executor_core::config::Config;

pub async fn run(config: &Config) -> Result<(), anyhow::Error> {
    use executor_core::metadata::cleanup_stale_tasks;
    
    eprintln!("🔍 Scanning for stale tasks (no heartbeat for >5 minutes)...");
    
    match cleanup_stale_tasks() {
        Ok(fixed) if !fixed.is_empty() => {
            eprintln!("✅ Fixed {} stale task(s):", fixed.len());
            for task_id in fixed {
                eprintln!("   - {}", task_id);
            }
            eprintln!("💡 Run 'openclaw-agent status <task_id>' to see details");
        }
        Ok(_) => {
            eprintln!("✅ No stale tasks found - all tasks are healthy!");
        }
        Err(e) => {
            eprintln!("❌ Error scanning tasks: {}", e);
        }
    }
    
    Ok(())
}
