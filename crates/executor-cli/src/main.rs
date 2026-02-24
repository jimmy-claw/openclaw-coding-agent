mod commands;
mod dispatch;

use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(
    name = "openclaw-agent",
    about = "OpenClaw Coding Agent — Executor Framework",
    version
)]
struct Cli {
    /// Path to config file (default: ~/.config/openclaw/coding-agent.yaml)
    #[arg(long, short)]
    config: Option<String>,

    /// Enable verbose logging
    #[arg(long, short)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start a new coding task on an executor
    Start {
        /// Executor name (from config)
        #[arg(long, short)]
        executor: String,

        /// Task prompt
        #[arg(long, short)]
        prompt: String,

        /// Workspace directory on the executor
        #[arg(long, short)]
        workspace: Option<String>,

        /// Maximum turns for claude
        #[arg(long)]
        max_turns: Option<u32>,

        /// Allowed tools (can be repeated)
        #[arg(long)]
        allowed_tools: Vec<String>,
    },

    /// Check status of a task
    Status {
        /// Task ID
        #[arg(long, short)]
        task_id: String,

        /// Output as JSON for dashboard integration
        #[arg(long)]
        json: bool,
    },

    /// Fetch logs from a task
    Logs {
        /// Task ID
        #[arg(long, short)]
        task_id: String,

        /// Number of lines to fetch
        #[arg(long, short, default_value = "50")]
        lines: usize,

        /// Follow log output (poll every N seconds)
        #[arg(long, short)]
        follow: Option<u64>,
    },

    /// Kill a running task
    Kill {
        /// Task ID
        #[arg(long, short)]
        task_id: String,
    },

    /// Cleanup task artifacts
    Cleanup {
        /// Task ID
        #[arg(long, short)]
        task_id: String,
    },

    /// List all tasks (from local metadata)
    List {
        /// Output as JSON for dashboard integration
        #[arg(long)]
        json: bool,

        /// Output as JSONL (one JSON object per line) for streaming
        #[arg(long)]
        jsonl: bool,

        /// Filter by status
        #[arg(long)]
        status: Option<String>,

        /// Filter by executor name
        #[arg(long)]
        executor: Option<String>,
    },

    /// List configured executors
    Executors {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Show or initialize the config file
    Config {
        /// Print the default config path
        #[arg(long)]
        path: bool,

        /// Initialize a sample config file
        #[arg(long)]
        init: bool,
    },

    /// Output task status as structured JSON for dashboards (issue #4)
    Dashboard {
        /// Stream mode: output JSONL for all tasks, then exit
        #[arg(long)]
        stream: bool,

        /// Watch mode: poll every N seconds
        #[arg(long)]
        watch: Option<u64>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Init tracing
    let filter = if cli.verbose {
        "debug"
    } else {
        "info"
    };
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(filter))
        .with_target(false)
        .init();

    // Load config
    let config = if let Some(ref path) = cli.config {
        executor_core::Config::load_from(std::path::Path::new(path))?
    } else {
        executor_core::Config::load_default()?
    };

    match cli.command {
        Commands::Start {
            executor,
            prompt,
            workspace,
            max_turns,
            allowed_tools,
        } => {
            commands::start::run(&config, &executor, prompt, workspace, max_turns, allowed_tools)
                .await
        }
        Commands::Status { task_id, json } => {
            commands::status::run(&config, &task_id, json).await
        }
        Commands::Logs {
            task_id,
            lines,
            follow,
        } => commands::logs::run(&config, &task_id, lines, follow).await,
        Commands::Kill { task_id } => commands::kill::run(&config, &task_id).await,
        Commands::Cleanup { task_id } => commands::cleanup::run(&config, &task_id).await,
        Commands::List {
            json,
            jsonl,
            status,
            executor,
        } => commands::list::run(json, jsonl, status, executor).await,
        Commands::Executors { json } => commands::executors::run(&config, json).await,
        Commands::Config { path, init } => commands::config::run(path, init).await,
        Commands::Dashboard { stream, watch } => {
            commands::dashboard::run(stream, watch).await
        }
    }
}
