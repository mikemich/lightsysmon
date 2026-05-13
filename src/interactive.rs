use clap_complete::Shell;
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};

use crate::cli::{CompletionsArgs, Commands, SnapshotArgs, SortKey, TopArgs, WatchArgs};
use crate::config::Config;

pub fn prompt_command(config: &Config) -> Option<Commands> {
    let theme = ColorfulTheme::default();
    let actions = [
        "Watch metrics (plain)",
        "Watch metrics (TUI dashboard)",
        "Top processes",
        "Snapshot",
        "System info",
        "Generate shell completions",
        "Quit",
    ];

    let action = Select::with_theme(&theme)
        .with_prompt("lightsysmon - choose an action")
        .items(actions)
        .default(0)
        .interact()
        .ok()?;

    match action {
        0 => Some(Commands::Watch(prompt_watch_args(config, false))),
        1 => Some(Commands::Watch(prompt_watch_args(config, true))),
        2 => Some(Commands::Top(prompt_top_args(config))),
        3 => Some(Commands::Snapshot(prompt_snapshot_args(config))),
        4 => Some(Commands::Info),
        5 => Some(Commands::Completions(prompt_completions_args())),
        _ => None,
    }
}

fn prompt_watch_args(config: &Config, tui: bool) -> WatchArgs {
    let theme = ColorfulTheme::default();
    let interval_default = config.interval.unwrap_or(1);
    let process_default = config.process_count.unwrap_or(5);

    let interval = Input::<u64>::with_theme(&theme)
        .with_prompt("Refresh interval in seconds")
        .default(interval_default)
        .interact_text()
        .unwrap_or(interval_default);

    let cores = Confirm::with_theme(&theme)
        .with_prompt("Show per-core CPU usage?")
        .default(false)
        .interact()
        .unwrap_or(false);

    let disk = Confirm::with_theme(&theme)
        .with_prompt("Include disk usage?")
        .default(config.show_disk.unwrap_or(false))
        .interact()
        .unwrap_or(config.show_disk.unwrap_or(false));

    let network = Confirm::with_theme(&theme)
        .with_prompt("Include network throughput?")
        .default(config.show_network.unwrap_or(false))
        .interact()
        .unwrap_or(config.show_network.unwrap_or(false));

    let processes = Confirm::with_theme(&theme)
        .with_prompt("Include top processes?")
        .default(config.show_processes.unwrap_or(false))
        .interact()
        .unwrap_or(config.show_processes.unwrap_or(false));

    let process_count = if processes {
        Input::<usize>::with_theme(&theme)
            .with_prompt("Number of processes to show")
            .default(process_default)
            .interact_text()
            .unwrap_or(process_default)
    } else {
        process_default
    };

    let sort = prompt_sort_key("Sort processes by");
    let timestamp = Confirm::with_theme(&theme)
        .with_prompt("Add timestamps?")
        .default(false)
        .interact()
        .unwrap_or(false);
    let json = Confirm::with_theme(&theme)
        .with_prompt("Output metrics as JSON lines?")
        .default(false)
        .interact()
        .unwrap_or(false);

    WatchArgs {
        interval,
        cores,
        disk,
        network,
        processes,
        process_count,
        sort,
        timestamp,
        json,
        log: None,
        tui,
        alert_cpu: None,
        alert_mem: None,
    }
}

fn prompt_top_args(config: &Config) -> TopArgs {
    let theme = ColorfulTheme::default();
    let count_default = config.process_count.unwrap_or(10);

    let sort = prompt_sort_key("Sort processes by");
    let count = Input::<usize>::with_theme(&theme)
        .with_prompt("Number of processes to show")
        .default(count_default)
        .interact_text()
        .unwrap_or(count_default);
    let json = Confirm::with_theme(&theme)
        .with_prompt("Output as JSON?")
        .default(false)
        .interact()
        .unwrap_or(false);

    TopArgs { sort, count, json }
}

fn prompt_snapshot_args(config: &Config) -> SnapshotArgs {
    let theme = ColorfulTheme::default();
    let disk = Confirm::with_theme(&theme)
        .with_prompt("Include disk metrics?")
        .default(config.show_disk.unwrap_or(false))
        .interact()
        .unwrap_or(config.show_disk.unwrap_or(false));
    let network = Confirm::with_theme(&theme)
        .with_prompt("Include network metrics?")
        .default(config.show_network.unwrap_or(false))
        .interact()
        .unwrap_or(config.show_network.unwrap_or(false));

    let output_modes = ["Pretty output", "JSON", "CSV"];
    let mode = Select::with_theme(&theme)
        .with_prompt("Snapshot output format")
        .items(output_modes)
        .default(0)
        .interact()
        .unwrap_or(0);

    SnapshotArgs {
        json: mode == 1,
        csv: mode == 2,
        disk,
        network,
    }
}

fn prompt_completions_args() -> CompletionsArgs {
    let theme = ColorfulTheme::default();
    let shells = ["bash", "zsh", "fish", "powershell", "elvish"];
    let idx = Select::with_theme(&theme)
        .with_prompt("Choose shell")
        .items(shells)
        .default(0)
        .interact()
        .unwrap_or(0);

    let shell = match idx {
        0 => Shell::Bash,
        1 => Shell::Zsh,
        2 => Shell::Fish,
        3 => Shell::PowerShell,
        _ => Shell::Elvish,
    };

    CompletionsArgs { shell }
}

fn prompt_sort_key(prompt: &str) -> SortKey {
    let theme = ColorfulTheme::default();
    let options = ["cpu", "mem"];
    let idx = Select::with_theme(&theme)
        .with_prompt(prompt)
        .items(options)
        .default(0)
        .interact()
        .unwrap_or(0);
    if idx == 1 {
        SortKey::Mem
    } else {
        SortKey::Cpu
    }
}
