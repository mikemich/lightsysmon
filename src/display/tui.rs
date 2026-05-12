use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Cell, Gauge, Paragraph, Row, Table},
    Terminal,
};
use std::io;
use std::time::{Duration, Instant};

use crate::cli::WatchArgs;
use crate::config::Config;
use crate::metrics::{AllMetrics, CollectArgs, Collector};

pub fn run_tui(args: &WatchArgs, config: &Config) {
    if let Err(e) = run_tui_inner(args, config) {
        eprintln!("TUI error: {}", e);
        std::process::exit(1);
    }
}

fn run_tui_inner(args: &WatchArgs, config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        original_hook(info);
    }));

    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    let mut collector = Collector::new();
    let sort_key = args.sort.clone();
    let show_disk = args.disk || config.show_disk.unwrap_or(false);
    let show_network = args.network || config.show_network.unwrap_or(false);
    let show_processes = args.processes || config.show_processes.unwrap_or(false);
    let process_count = if args.process_count == 5 {
        config.process_count.unwrap_or(args.process_count)
    } else {
        args.process_count
    };
    let interval_secs = if args.interval == 1 {
        config.interval.unwrap_or(args.interval)
    } else {
        args.interval
    };
    let collect_args = CollectArgs {
        disk: show_disk,
        network: show_network,
        processes: show_processes,
        process_count,
        sort_key: &sort_key,
        include_timestamp: false,
    };

    collector.refresh();
    let mut metrics = collector.collect(&collect_args);
    let mut last_refresh = Instant::now();
    let refresh_interval = Duration::from_secs(interval_secs);

    loop {
        terminal.draw(|f| draw_ui(f, &metrics))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Char('c')
                        if key.modifiers.contains(KeyModifiers::CONTROL) =>
                    {
                        break
                    }
                    _ => {}
                }
            }
        }

        if last_refresh.elapsed() >= refresh_interval {
            collector.refresh();
            metrics = collector.collect(&collect_args);
            last_refresh = Instant::now();
        }
    }

    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    disable_raw_mode()?;

    Ok(())
}

fn usage_color(pct: f64) -> Color {
    if pct < 60.0 {
        Color::Green
    } else if pct < 80.0 {
        Color::Yellow
    } else {
        Color::Red
    }
}

fn draw_ui(f: &mut ratatui::Frame, metrics: &AllMetrics) {
    let has_cores = !metrics.cpu.per_core.is_empty();
    let has_disk = !metrics.disk.is_empty();
    let has_network = !metrics.network.is_empty();
    let has_processes = !metrics.processes.is_empty();

    // Build constraint list dynamically
    let mut constraints: Vec<Constraint> = vec![
        Constraint::Length(3), // title bar
        Constraint::Length(3), // CPU global gauge
    ];
    if has_cores {
        constraints.push(Constraint::Length(
            (metrics.cpu.per_core.len() as u16).saturating_add(2),
        ));
    }
    constraints.push(Constraint::Length(5)); // memory section
    if has_disk {
        constraints.push(Constraint::Length(
            (metrics.disk.len() as u16).saturating_add(3),
        ));
    }
    if has_network {
        constraints.push(Constraint::Length(
            (metrics.network.len() as u16).saturating_add(3),
        ));
    }
    if has_processes {
        constraints.push(Constraint::Length(
            (metrics.processes.len() as u16).saturating_add(4),
        ));
    }
    constraints.push(Constraint::Min(0));

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(f.area());

    let mut idx: usize = 0;

    // Title bar
    let version = env!("CARGO_PKG_VERSION");
    f.render_widget(
        Paragraph::new(format!(
            "lightsysmon v{}  |  Press q to quit",
            version
        ))
        .block(Block::default().borders(Borders::ALL)),
        chunks[idx],
    );
    idx += 1;

    // CPU global gauge
    let cpu_pct = metrics.cpu.global_usage.clamp(0.0, 100.0);
    f.render_widget(
        Gauge::default()
            .block(Block::default().borders(Borders::ALL).title("CPU"))
            .gauge_style(Style::default().fg(usage_color(cpu_pct)))
            .ratio(cpu_pct / 100.0)
            .label(format!(
                "{:.1}% - {} cores @ {} MHz  {}",
                cpu_pct,
                metrics.cpu.cpu_count,
                metrics.cpu.frequency_mhz,
                metrics.cpu.brand,
            )),
        chunks[idx],
    );
    idx += 1;

    // Per-core text
    if has_cores {
        let core_text: String = metrics
            .cpu
            .per_core
            .iter()
            .enumerate()
            .map(|(i, u)| format!("Core {:2}: {:6.1}%", i, u))
            .collect::<Vec<_>>()
            .join("\n");
        f.render_widget(
            Paragraph::new(core_text)
                .block(Block::default().borders(Borders::ALL).title("Per-Core CPU")),
            chunks[idx],
        );
        idx += 1;
    }

    // Memory section
    let mem_block = Block::default().borders(Borders::ALL).title("Memory");
    let mem_inner = mem_block.inner(chunks[idx]);
    f.render_widget(mem_block, chunks[idx]);
    idx += 1;

    let mem_sub = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(mem_inner);

    let ram_pct = metrics.memory.usage_pct.clamp(0.0, 100.0);
    f.render_widget(
        Gauge::default()
            .gauge_style(Style::default().fg(usage_color(ram_pct)))
            .ratio(ram_pct / 100.0)
            .label(format!(
                "RAM: {:.1}%  ({} / {})",
                ram_pct,
                crate::display::plain::format_bytes(metrics.memory.used),
                crate::display::plain::format_bytes(metrics.memory.total),
            )),
        mem_sub[0],
    );
    if metrics.memory.swap_total > 0 {
        let swap_pct = metrics.memory.swap_pct.clamp(0.0, 100.0);
        f.render_widget(
            Gauge::default()
                .gauge_style(Style::default().fg(usage_color(swap_pct)))
                .ratio(swap_pct / 100.0)
                .label(format!(
                    "Swap: {:.1}%  ({} / {})",
                    swap_pct,
                    crate::display::plain::format_bytes(metrics.memory.swap_used),
                    crate::display::plain::format_bytes(metrics.memory.swap_total),
                )),
            mem_sub[1],
        );
    }

    // Disk table
    if has_disk {
        let rows: Vec<Row> = metrics
            .disk
            .iter()
            .map(|d| {
                Row::new(vec![
                    Cell::from(d.mount_point.clone()),
                    Cell::from(d.fs_type.clone()),
                    Cell::from(format!("{:.1}%", d.used_pct)),
                    Cell::from(crate::display::plain::format_bytes(d.used)),
                    Cell::from(crate::display::plain::format_bytes(d.total)),
                ])
            })
            .collect();
        let widths = [
            Constraint::Percentage(30),
            Constraint::Percentage(15),
            Constraint::Percentage(15),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
        ];
        f.render_widget(
            Table::new(rows, widths)
                .header(Row::new(["Mount", "FS", "Used%", "Used", "Total"]))
                .block(Block::default().borders(Borders::ALL).title("Disk")),
            chunks[idx],
        );
        idx += 1;
    }

    // Network table
    if has_network {
        let rows: Vec<Row> = metrics
            .network
            .iter()
            .map(|n| {
                Row::new(vec![
                    Cell::from(n.interface.clone()),
                    Cell::from(crate::display::plain::format_bytes(n.rx_bytes)),
                    Cell::from(crate::display::plain::format_bytes(n.tx_bytes)),
                    Cell::from(crate::display::plain::format_bytes(n.total_rx)),
                    Cell::from(crate::display::plain::format_bytes(n.total_tx)),
                ])
            })
            .collect();
        let widths = [
            Constraint::Percentage(30),
            Constraint::Percentage(15),
            Constraint::Percentage(15),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
        ];
        f.render_widget(
            Table::new(rows, widths)
                .header(Row::new(["Interface", "RX/s", "TX/s", "Total RX", "Total TX"]))
                .block(Block::default().borders(Borders::ALL).title("Network")),
            chunks[idx],
        );
        idx += 1;
    }

    // Processes table
    if has_processes {
        let rows: Vec<Row> = metrics
            .processes
            .iter()
            .map(|p| {
                Row::new(vec![
                    Cell::from(p.pid.to_string()),
                    Cell::from(p.name.clone()),
                    Cell::from(format!("{:.1}%", p.cpu_usage)),
                    Cell::from(crate::display::plain::format_bytes(p.memory_bytes)),
                    Cell::from(format!("{:.1}%", p.memory_pct)),
                ])
            })
            .collect();
        let widths = [
            Constraint::Length(8),
            Constraint::Percentage(35),
            Constraint::Percentage(15),
            Constraint::Percentage(25),
            Constraint::Percentage(15),
        ];
        f.render_widget(
            Table::new(rows, widths)
                .header(Row::new(["PID", "Name", "CPU%", "Memory", "Mem%"]))
                .block(Block::default().borders(Borders::ALL).title("Processes")),
            chunks[idx],
        );
        idx += 1;
    }

    // Consume idx to avoid unused-variable warning when all optional sections are absent
    let _ = idx;
}
