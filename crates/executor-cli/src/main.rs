use anyhow::Result;
use clap::{Parser, Subcommand};

mod commands;
mod dispatch;

#[derive(Parser)]
#[command(name = "openclaw-agent")]
#[command(about = "Manage coding agent tasks", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start a new task
    Start {
        /// Executor name (ssh, local, etc.)
        executor: String,
        
        /// Task payload (prompt or command)
        payload: String,
        
        /// Payload type: claude_code or shell_command
        #[arg(short, long, default_value = "claude_code")]
        type: String,
        
        /// Detach and return immediately
        #[arg(short, long)]
        detach: bool,
    },
    
    /// Show task status
    Status {
        /// Task ID
        task_id: String,
        
        /// Output as JSON
        #[arg(short, long)]
        json: bool,
    },
    
    /// Show task logs
    Logs {
        /// Task ID
        task_id: String,
        
        /// Number of lines to show
        #[arg(short, long, default_value = "50")]
        lines: usize,
    },
    
    /// Kill a running task
    Kill {
        /// Task ID
        task_id: String,
    },
    
    /// Clean up task artifacts
    Cleanup {
        /// Task ID
        task_id: String,
    },
    
    /// Cleanup stale tasks (no heartbeat for >5 min)
    CleanupStale {},
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    use Commands::*;
    
    match cli.command {
        Start { executor, payload, type: payload_type, detach } => {
            let config = commands::start::load_config()?;
            commands::start::run(&config, &executor, &payload, &payload_type, detach).await?;
        }
        Status { task_id, json } => {
            let config = commands::start::load_config()?;
            commands::status::run(&config, &task_id, json).await?;
        }
        Logs { task_id, lines } => {
            let config = commands::start::load_config()?;
            commands::logs::run(&config, &task_id, lines).await?;
        }
        Kill { task_id } => {
            let config = commands::start::load_config()?;
            commands::kill::run(&config, &task_id).await?;
        }
        Cleanup { task_id } => {
            let config = commands::start::load_config()?;
            commands::cleanup::run(&config, &task_id).await?;
        }
        CleanupStale {} => {
            let config = commands::start::load_config()?;
            commands::cleanup_stale::run(&config).await?;
        }
    }
    
    Ok(())
}
