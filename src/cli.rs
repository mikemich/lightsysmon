use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[clap(author, version, about = "A lightweight system monitor", long_about = None)]
pub struct Cli {
    /// Path to config file (default: ~/.config/lightsysmon/config.toml)
    #[clap(short, long)]
    pub config: Option<String>,
    #[clap(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Watch system metrics continuously
    Watch(WatchArgs),
    /// Show top processes
    Top(TopArgs),
    /// Show a one-shot system information summary
    Info,
    /// Take a single snapshot of current metrics
    Snapshot(SnapshotArgs),
    /// Generate shell completions
    Completions(CompletionsArgs),
}

#[derive(Parser, Clone)]
pub struct WatchArgs {
    /// Refresh interval in seconds
    #[clap(short, long, default_value_t = 1)]
    pub interval: u64,
    /// Show per-core CPU usage
    #[clap(long)]
    pub cores: bool,
    /// Show disk usage
    #[clap(long)]
    pub disk: bool,
    /// Show network throughput
    #[clap(long, alias = "net")]
    pub network: bool,
    /// Show top processes
    #[clap(long)]
    pub processes: bool,
    /// Number of top processes to show
    #[clap(long, default_value_t = 5)]
    pub process_count: usize,
    /// Sort processes by field
    #[clap(long, default_value = "cpu")]
    pub sort: SortKey,
    /// Add timestamp to each output line
    #[clap(long)]
    pub timestamp: bool,
    /// Output as JSON lines
    #[clap(long)]
    pub json: bool,
    /// Log metrics to CSV file
    #[clap(long)]
    pub log: Option<String>,
    /// Use interactive TUI mode
    #[clap(long)]
    pub tui: bool,
    /// Alert when CPU usage exceeds threshold (%)
    #[clap(long)]
    pub alert_cpu: Option<f64>,
    /// Alert when memory usage exceeds threshold (%)
    #[clap(long)]
    pub alert_mem: Option<f64>,
}

#[derive(Parser, Clone)]
pub struct TopArgs {
    /// Sort by field
    #[clap(short, long, default_value = "cpu")]
    pub sort: SortKey,
    /// Number of processes to show
    #[clap(short, long, default_value_t = 10)]
    pub count: usize,
    /// Output as JSON
    #[clap(long)]
    pub json: bool,
}

#[derive(ValueEnum, Clone, Debug, serde::Deserialize, serde::Serialize, Default)]
pub enum SortKey {
    #[default]
    Cpu,
    Mem,
}

#[derive(Parser, Clone)]
pub struct SnapshotArgs {
    /// Output as JSON
    #[clap(long)]
    pub json: bool,
    /// Output as CSV
    #[clap(long)]
    pub csv: bool,
    /// Include disk metrics
    #[clap(long)]
    pub disk: bool,
    /// Include network metrics
    #[clap(long, alias = "net")]
    pub network: bool,
}

#[derive(Parser, Clone)]
pub struct CompletionsArgs {
    pub shell: clap_complete::Shell,
}
