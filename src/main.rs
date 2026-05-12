use clap::{Parser, Subcommand};
use sysinfo::System;
use tokio::time::{self, Duration};

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

fn memory_usage_percent(used_memory: u64, total_memory: u64) -> f64 {
    if total_memory == 0 {
        0.0
    } else {
        (used_memory as f64 / total_memory as f64) * 100.0
    }
}

fn metric_lines(cpu_usage: f64, mem_usage: f64) -> (String, String) {
    (
        format!("CPU Usage: {:.1}%", cpu_usage),
        format!("Memory Usage: {:.1}%", mem_usage),
    )
}

async fn watch_system_metrics(interval: u64) {
    let mut sys = System::new_all();

    loop {
        sys.refresh_all();

        let cpu_usage = sys.global_cpu_usage() as f64;
        let mem_usage = memory_usage_percent(sys.used_memory(), sys.total_memory());
        let (cpu_line, mem_line) = metric_lines(cpu_usage, mem_usage);

        println!("{cpu_line}");
        println!("{mem_line}");

        time::sleep(Duration::from_secs(interval)).await;
    }
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Watch(args) => watch_system_metrics(args.interval).await,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_watch_interval_default() {
        let cli = Cli::try_parse_from(["lightsysmon", "watch"]).expect("should parse");
        let Commands::Watch(args) = cli.command;
        assert_eq!(args.interval, 1);
    }

    #[test]
    fn parse_watch_interval_custom_short_flag() {
        let cli = Cli::try_parse_from(["lightsysmon", "watch", "-i", "5"]).expect("should parse");
        let Commands::Watch(args) = cli.command;
        assert_eq!(args.interval, 5);
    }

    #[test]
    fn parse_watch_interval_custom_long_flag() {
        let cli = Cli::try_parse_from(["lightsysmon", "watch", "--interval", "10"])
            .expect("should parse");
        let Commands::Watch(args) = cli.command;
        assert_eq!(args.interval, 10);
    }

    #[test]
    fn parse_fails_without_subcommand() {
        let result = Cli::try_parse_from(["lightsysmon"]);
        assert!(result.is_err());
    }

    #[test]
    fn memory_usage_percent_is_calculated() {
        assert!((memory_usage_percent(500, 1000) - 50.0).abs() < f64::EPSILON);
    }

    #[test]
    fn memory_usage_percent_handles_zero_total_memory() {
        assert!((memory_usage_percent(500, 0) - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn metric_lines_format_one_decimal() {
        let (cpu_line, mem_line) = metric_lines(12.54, 56.26);
        assert_eq!(cpu_line, "CPU Usage: 12.5%");
        assert_eq!(mem_line, "Memory Usage: 56.3%");
    }
}
