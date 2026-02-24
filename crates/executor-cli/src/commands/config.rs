use executor_core::Config;

const SAMPLE_CONFIG: &str = r#"# OpenClaw Coding Agent Configuration
# See: https://github.com/openclaw/coding-agent

executors:
  - name: crib
    type: ssh
    host: 192.168.0.152
    user: jimmy
    claude_path: ~/.npm-global/bin/claude
    labels:
      - rust
      - heavy-compute

  - name: builder
    type: container
    image: claude-code:latest
    runtime: docker
    volumes:
      - /home/jimmy/repos:/work
    labels:
      - isolated
      - reproducible

  - name: local
    type: local
    claude_path: claude
    labels:
      - quick-tasks
      - lightweight

defaults:
  max_turns: 100
  claude_path: claude
"#;

pub async fn run(path: bool, init: bool) -> anyhow::Result<()> {
    if path {
        println!("{}", Config::default_path().display());
        return Ok(());
    }

    if init {
        let config_path = Config::default_path();
        if config_path.exists() {
            println!("Config already exists at: {}", config_path.display());
            println!("Remove it first if you want to reinitialize.");
            return Ok(());
        }

        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&config_path, SAMPLE_CONFIG)?;
        println!("Sample config written to: {}", config_path.display());
        return Ok(());
    }

    // Default: show current config path and status
    let config_path = Config::default_path();
    println!("Config path: {}", config_path.display());
    if config_path.exists() {
        let config = Config::load_from(&config_path)?;
        println!("Executors:   {}", config.executors.len());
        for e in &config.executors {
            println!("  - {} ({})", e.name, e.executor_type);
        }
    } else {
        println!("Status:      not found");
        println!("Run `openclaw-agent config --init` to create one.");
    }

    Ok(())
}
