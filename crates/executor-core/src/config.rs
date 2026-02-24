use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Top-level configuration. Covers GitHub issue #5.
/// Loaded from ~/.config/openclaw/coding-agent.yaml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub executors: Vec<ExecutorConfig>,
    #[serde(default)]
    pub defaults: Defaults,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutorConfig {
    pub name: String,
    #[serde(rename = "type")]
    pub executor_type: ExecutorType,
    #[serde(default)]
    pub host: Option<String>,
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default)]
    pub user: Option<String>,
    #[serde(default)]
    pub key_path: Option<String>,
    #[serde(default)]
    pub claude_path: Option<String>,
    #[serde(default)]
    pub image: Option<String>,
    #[serde(default)]
    pub runtime: Option<ContainerRuntime>,
    #[serde(default)]
    pub volumes: Vec<String>,
    #[serde(default)]
    pub labels: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExecutorType {
    Ssh,
    Container,
    Local,
}

impl std::fmt::Display for ExecutorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecutorType::Ssh => write!(f, "ssh"),
            ExecutorType::Container => write!(f, "container"),
            ExecutorType::Local => write!(f, "local"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContainerRuntime {
    Docker,
    Podman,
}

impl Default for ContainerRuntime {
    fn default() -> Self {
        Self::Docker
    }
}

impl std::fmt::Display for ContainerRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContainerRuntime::Docker => write!(f, "docker"),
            ContainerRuntime::Podman => write!(f, "podman"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Defaults {
    #[serde(default = "default_max_turns")]
    pub max_turns: u32,
    #[serde(default = "default_claude_path")]
    pub claude_path: String,
}

impl Default for Defaults {
    fn default() -> Self {
        Self {
            max_turns: default_max_turns(),
            claude_path: default_claude_path(),
        }
    }
}

fn default_max_turns() -> u32 {
    100
}

fn default_claude_path() -> String {
    "claude".to_string()
}

impl Config {
    /// Load config from the default path (~/.config/openclaw/coding-agent.yaml).
    pub fn load_default() -> anyhow::Result<Self> {
        let path = Self::default_path();
        if path.exists() {
            Self::load_from(&path)
        } else {
            Ok(Self::empty())
        }
    }

    /// Load config from a specific path.
    pub fn load_from(path: &Path) -> anyhow::Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        let config: Config = serde_yaml::from_str(&contents)?;
        Ok(config)
    }

    /// Default config file path.
    pub fn default_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("/etc"))
            .join("openclaw")
            .join("coding-agent.yaml")
    }

    /// Empty config with no executors.
    pub fn empty() -> Self {
        Self {
            executors: Vec::new(),
            defaults: Defaults::default(),
        }
    }

    /// Find an executor config by name.
    pub fn find_executor(&self, name: &str) -> Option<&ExecutorConfig> {
        self.executors.iter().find(|e| e.name == name)
    }

    /// Find executors matching all given labels.
    pub fn find_by_labels(&self, labels: &[String]) -> Vec<&ExecutorConfig> {
        self.executors
            .iter()
            .filter(|e| labels.iter().all(|l| e.labels.contains(l)))
            .collect()
    }
}

impl ExecutorConfig {
    /// Get the claude binary path, falling back to "claude".
    pub fn claude_binary(&self) -> &str {
        self.claude_path.as_deref().unwrap_or("claude")
    }

    /// Get the SSH port, falling back to 22.
    pub fn ssh_port(&self) -> u16 {
        self.port.unwrap_or(22)
    }
}
