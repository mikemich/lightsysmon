use std::io::{IsTerminal, Write};

use crate::cli::{SnapshotArgs, SortKey, TopArgs, WatchArgs};
use crate::config::Config;
use crate::metrics::{CollectArgs, Collector};

pub fn format_bytes(bytes: u64) -> String {
    const KIB: u64 = 1024;
    const MIB: u64 = 1024 * KIB;
    const GIB: u64 = 1024 * MIB;

    if bytes >= GIB {
        format!("{:.2} GiB", bytes as f64 / GIB as f64)
    } else if bytes >= MIB {
        format!("{:.2} MiB", bytes as f64 / MIB as f64)
    } else if bytes >= KIB {
        format!("{:.2} KiB", bytes as f64 / KIB as f64)
    } else {
        format!("{} B", bytes)
    }
}

fn colorize(value: f64, text: &str, use_color: bool) -> String {
    if !use_color {
        return text.to_string();
    }
    let color = if value < 60.0 {
        "\x1b[32m"
    } else if value < 80.0 {
        "\x1b[33m"
    } else {
        "\x1b[31m"
    };
    format!("{}{}\x1b[0m", color, text)
}

pub async fn run_watch(args: &WatchArgs, config: &Config) {
    let use_color = std::io::stdout().is_terminal();

    // Config provides defaults; explicit CLI flags take precedence where detectable.
    let interval = if args.interval == 1 {
        config.interval.unwrap_or(args.interval)
    } else {
        args.interval
    };
    let show_disk = args.disk || config.show_disk.unwrap_or(false);
    let show_network = args.network || config.show_network.unwrap_or(false);
    let show_processes = args.processes || config.show_processes.unwrap_or(false);
    let process_count = if args.process_count == 5 {
        config.process_count.unwrap_or(args.process_count)
    } else {
        args.process_count
    };
    let alert_cpu = args.alert_cpu.or(config.alert_cpu);
    let alert_mem = args.alert_mem.or(config.alert_mem);

    let mut collector = Collector::new();
    let sort_key = args.sort.clone();
    let collect_args = CollectArgs {
        disk: show_disk,
        network: show_network,
        processes: show_processes,
        process_count,
        sort_key: &sort_key,
        include_timestamp: args.timestamp,
    };

    let mut csv_file: Option<std::fs::File> = None;
    if let Some(ref log_path) = args.log {
        let file_is_new = !std::path::Path::new(log_path).exists()
            || std::fs::metadata(log_path).map_or(true, |m| m.len() == 0);
        let file = match std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(log_path)
        {
            Ok(f) => f,
            Err(e) => {
                eprintln!("Failed to open log file '{}': {}", log_path, e);
                std::process::exit(1);
            }
        };
        if file_is_new {
            let mut f = file;
            if let Err(e) = writeln!(f, "timestamp,cpu_pct,mem_pct,mem_used_bytes,mem_total_bytes")
            {
                eprintln!("Failed to write CSV header: {}", e);
            }
            csv_file = Some(f);
        } else {
            csv_file = Some(file);
        }
    }

    let mut interval_timer =
        tokio::time::interval(tokio::time::Duration::from_secs(interval));

    loop {
        interval_timer.tick().await;
        collector.refresh();
        let metrics = collector.collect(&collect_args);

        if args.json {
            match serde_json::to_string(&metrics) {
                Ok(json) => println!("{}", json),
                Err(e) => eprintln!("JSON serialization error: {}", e),
            }
        } else {
            if let Some(ref ts) = metrics.timestamp {
                println!("[{}]", ts);
            }

            let cpu_text = format!("{:.1}%", metrics.cpu.global_usage);
            println!(
                "CPU:    {} ({} cores, {} MHz, {})",
                colorize(metrics.cpu.global_usage, &cpu_text, use_color),
                metrics.cpu.cpu_count,
                metrics.cpu.frequency_mhz,
                metrics.cpu.brand,
            );

            if args.cores {
                for (i, usage) in metrics.cpu.per_core.iter().enumerate() {
                    let core_text = format!("{:.1}%", usage);
                    println!("  Core {}: {}", i, colorize(*usage, &core_text, use_color));
                }
            }

            let mem_text = format!("{:.1}%", metrics.memory.usage_pct);
            println!(
                "Memory: {} ({} / {})",
                colorize(metrics.memory.usage_pct, &mem_text, use_color),
                format_bytes(metrics.memory.used),
                format_bytes(metrics.memory.total),
            );

            if metrics.memory.swap_total > 0 {
                let swap_text = format!("{:.1}%", metrics.memory.swap_pct);
                println!(
                    "Swap:   {} ({} / {})",
                    colorize(metrics.memory.swap_pct, &swap_text, use_color),
                    format_bytes(metrics.memory.swap_used),
                    format_bytes(metrics.memory.swap_total),
                );
            }

            if !metrics.disk.is_empty() {
                println!("Disks:");
                for disk in &metrics.disk {
                    let pct_text = format!("{:.1}%", disk.used_pct);
                    println!(
                        "  {} ({}): {} used of {}  [{}]",
                        disk.mount_point,
                        disk.fs_type,
                        colorize(disk.used_pct, &pct_text, use_color),
                        format_bytes(disk.total),
                        disk.name,
                    );
                }
            }

            if !metrics.network.is_empty() {
                println!("Network:");
                for net in &metrics.network {
                    println!(
                        "  {}: \u{2193} {} \u{2191} {}",
                        net.interface,
                        format_bytes(net.rx_bytes),
                        format_bytes(net.tx_bytes),
                    );
                }
            }

            if !metrics.processes.is_empty() {
                println!("Top Processes:");
                println!(
                    "  {:>7}  {:<25} {:>8}  {:>10}  {:>7}",
                    "PID", "Name", "CPU%", "Memory", "Mem%"
                );
                for proc in &metrics.processes {
                    println!(
                        "  {:>7}  {:<25} {:>7.1}%  {:>10}  {:>6.1}%",
                        proc.pid,
                        proc.name,
                        proc.cpu_usage,
                        format_bytes(proc.memory_bytes),
                        proc.memory_pct,
                    );
                }
            }

            println!("---");
        }

        if let Some(threshold) = alert_cpu {
            if metrics.cpu.global_usage > threshold {
                eprintln!(
                    "ALERT: CPU usage {:.1}% exceeds threshold {:.1}%",
                    metrics.cpu.global_usage, threshold
                );
            }
        }
        if let Some(threshold) = alert_mem {
            if metrics.memory.usage_pct > threshold {
                eprintln!(
                    "ALERT: Memory usage {:.1}% exceeds threshold {:.1}%",
                    metrics.memory.usage_pct, threshold
                );
            }
        }

        if let Some(ref mut file) = csv_file {
            let ts = chrono::Local::now().to_rfc3339();
            if let Err(e) = writeln!(
                file,
                "{},{:.1},{:.1},{},{}",
                ts,
                metrics.cpu.global_usage,
                metrics.memory.usage_pct,
                metrics.memory.used,
                metrics.memory.total,
            ) {
                eprintln!("Failed to write to log file: {}", e);
            }
        }
    }
}

pub fn run_snapshot(args: &SnapshotArgs, config: &Config) {
    let mut collector = Collector::new();
    collector.refresh();

    let sort_key = SortKey::default();
    let show_disk = args.disk || config.show_disk.unwrap_or(false);
    let show_network = args.network || config.show_network.unwrap_or(false);
    let collect_args = CollectArgs {
        disk: show_disk,
        network: show_network,
        processes: false,
        process_count: 0,
        sort_key: &sort_key,
        include_timestamp: true,
    };

    let metrics = collector.collect(&collect_args);

    if args.json {
        match serde_json::to_string_pretty(&metrics) {
            Ok(json) => println!("{}", json),
            Err(e) => eprintln!("JSON error: {}", e),
        }
    } else if args.csv {
        println!("timestamp,cpu_pct,mem_pct,mem_used_bytes,mem_total_bytes");
        println!(
            "{},{:.1},{:.1},{},{}",
            chrono::Local::now().to_rfc3339(),
            metrics.cpu.global_usage,
            metrics.memory.usage_pct,
            metrics.memory.used,
            metrics.memory.total,
        );
    } else {
        let use_color = std::io::stdout().is_terminal();
        if let Some(ref ts) = metrics.timestamp {
            println!("Snapshot at {}", ts);
        }
        let cpu_text = format!("{:.1}%", metrics.cpu.global_usage);
        println!(
            "CPU:    {}",
            colorize(metrics.cpu.global_usage, &cpu_text, use_color)
        );
        let mem_text = format!("{:.1}%", metrics.memory.usage_pct);
        println!(
            "Memory: {} ({} / {})",
            colorize(metrics.memory.usage_pct, &mem_text, use_color),
            format_bytes(metrics.memory.used),
            format_bytes(metrics.memory.total),
        );
        if !metrics.disk.is_empty() {
            println!("Disks:");
            for disk in &metrics.disk {
                println!(
                    "  {}: {:.1}% used of {}",
                    disk.mount_point,
                    disk.used_pct,
                    format_bytes(disk.total)
                );
            }
        }
        if !metrics.network.is_empty() {
            println!("Network:");
            for net in &metrics.network {
                println!(
                    "  {}: \u{2193} {} \u{2191} {}",
                    net.interface,
                    format_bytes(net.rx_bytes),
                    format_bytes(net.tx_bytes)
                );
            }
        }
    }
}

pub fn run_top(args: &TopArgs, config: &Config) {
    let mut collector = Collector::new();
    collector.refresh();

    let count = if args.count == 10 {
        config.process_count.unwrap_or(args.count)
    } else {
        args.count
    };
    let collect_args = CollectArgs {
        disk: false,
        network: false,
        processes: true,
        process_count: count,
        sort_key: &args.sort,
        include_timestamp: false,
    };

    let metrics = collector.collect(&collect_args);

    if args.json {
        match serde_json::to_string_pretty(&metrics.processes) {
            Ok(json) => println!("{}", json),
            Err(e) => eprintln!("JSON error: {}", e),
        }
    } else {
        println!(
            "{:>7}  {:<25} {:>8}  {:>10}  {:>7}",
            "PID", "Name", "CPU%", "Memory", "Mem%"
        );
        for proc in &metrics.processes {
            println!(
                "{:>7}  {:<25} {:>7.1}%  {:>10}  {:>6.1}%",
                proc.pid,
                proc.name,
                proc.cpu_usage,
                format_bytes(proc.memory_bytes),
                proc.memory_pct,
            );
        }
    }
}

pub fn print_info() {
    let sys = sysinfo::System::new_all();
    let hostname = sysinfo::System::host_name().unwrap_or_else(|| "unknown".to_string());
    let os = sysinfo::System::os_version().unwrap_or_else(|| "unknown".to_string());
    let kernel = sysinfo::System::kernel_version().unwrap_or_else(|| "unknown".to_string());
    let uptime = sysinfo::System::uptime();
    let cpu_count = sys.cpus().len();
    let brand = sys
        .cpus()
        .first()
        .map_or("unknown", |c| c.brand());
    let total_ram = sys.total_memory();

    println!("System Information");
    println!("==================");
    println!("Hostname:    {}", hostname);
    println!("OS:          {}", os);
    println!("Kernel:      {}", kernel);
    println!(
        "Uptime:      {}h {}m {}s",
        uptime / 3600,
        (uptime % 3600) / 60,
        uptime % 60
    );
    println!("CPU:         {} ({} cores)", brand, cpu_count);
    println!("Total RAM:   {}", format_bytes(total_ram));
}
