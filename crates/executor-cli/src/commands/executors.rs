use executor_core::config::Config;

pub async fn run(config: &Config, json: bool) -> anyhow::Result<()> {
    if config.executors.is_empty() {
        println!("No executors configured.");
        println!("Run `openclaw-agent config --init` to create a sample config.");
        return Ok(());
    }

    if json {
        let entries: Vec<serde_json::Value> = config
            .executors
            .iter()
            .map(|e| {
                serde_json::json!({
                    "name": e.name,
                    "type": e.executor_type.to_string(),
                    "host": e.host,
                    "labels": e.labels,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&entries)?);
    } else {
        println!("{:<15} {:<12} {:<20} {}", "NAME", "TYPE", "HOST", "LABELS");
        println!("{}", "-".repeat(60));
        for e in &config.executors {
            println!(
                "{:<15} {:<12} {:<20} {}",
                e.name,
                e.executor_type,
                e.host.as_deref().unwrap_or("-"),
                e.labels.join(", "),
            );
        }
    }

    Ok(())
}
