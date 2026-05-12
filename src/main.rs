use clap::{Parser, Subcommand};
use tokio::time::{self, Duration};
use sysinfo::System;

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Watch(WatchArgs),
}

#[derive(Parser)]
struct WatchArgs {
    #[clap(short, long, default_value_t = 1)]
    interval: u64,
}

async fn watch_system_metrics(interval: u64) {
    let mut sys = System::new_all();

    loop {
        sys.refresh_all();

        let cpu_usage = sys.global_cpu_usage() as f64;
        let mem_usage = (sys.used_memory() as f64 / sys.total_memory() as f64) * 100.0;

        println!("CPU Usage: {:.1}%", cpu_usage);
        println!("Memory Usage: {:.1}%", mem_usage);

        time::sleep(Duration::from_secs(interval)).await;
    }
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Watch(args) =>
watch_system_metrics(args.interval).await,
    }
}