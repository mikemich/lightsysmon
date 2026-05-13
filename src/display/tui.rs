use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Gauge, Paragraph, Row, Table},
    Terminal,
};
use std::io;
use std::time::{Duration, Instant};

use crate::cli::{SortKey, WatchArgs};
use crate::config::Config;
use crate::metrics::{AllMetrics, CollectArgs, Collector};

struct AppState {
    show_disk: bool,
    show_network: bool,
    show_processes: bool,
    show_cores: bool,
    sort_key: SortKey,
    interval_secs: u64,
    process_count: usize,
}

pub fn run_tui(args: &WatchArgs, config: &Config) {
    let state = AppState {
        show_disk: args.disk || config.show_disk.unwrap_or(false),
        show_network: args.network || config.show_network.unwrap_or(false),
        show_processes: args.processes || config.show_processes.unwrap_or(true),
        show_cores: args.cores,
        sort_key: args.sort.clone(),
        interval_secs: if args.interval == 1 {
            config.interval.unwrap_or(1)
        } else {
            args.interval
        },
        process_count: if args.process_count == 5 {
            config.process_count.unwrap_or(10)
        } else {
            args.process_count
        },
    };
    if let Err(e) = run_tui_inner(state) {
        eprintln!("TUI error: {}", e);
        std::process::exit(1);
    }
}

pub fn run_tui_default(config: &Config) {
    let state = AppState {
        show_disk: config.show_disk.unwrap_or(false),
        show_network: config.show_network.unwrap_or(false),
        show_processes: config.show_processes.unwrap_or(true),
        show_cores: false,
        sort_key: SortKey::Cpu,
        interval_secs: config.interval.unwrap_or(1),
        process_count: config.process_count.unwrap_or(10),
    };
    if let Err(e) = run_tui_inner(state) {
        eprintln!("TUI error: {}", e);
        std::process::exit(1);
    }
}

fn run_tui_inner(mut state: AppState) -> Result<(), Box<dyn std::error::Error>> {
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
    collector.refresh();

    let mut metrics = {
        let ca = CollectArgs {
            disk: state.show_disk,
            network: state.show_network,
            processes: state.show_processes,
            process_count: state.process_count,
            sort_key: &state.sort_key,
            include_timestamp: false,
        };
        collector.collect(&ca)
    };
    let mut last_refresh = Instant::now();

    loop {
        terminal.draw(|f| draw_ui(f, &metrics, &state))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                let mut needs_refresh = false;
                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => break,
                    KeyCode::Char('d') => {
                        state.show_disk = !state.show_disk;
                        needs_refresh = true;
                    }
                    KeyCode::Char('n') => {
                        state.show_network = !state.show_network;
                        needs_refresh = true;
                    }
                    KeyCode::Char('p') => {
                        state.show_processes = !state.show_processes;
                        needs_refresh = true;
                    }
                    KeyCode::Char('c') => {
                        state.show_cores = !state.show_cores;
                    }
                    KeyCode::Char('s') => {
                        state.sort_key = match state.sort_key {
                            SortKey::Cpu => SortKey::Mem,
                            SortKey::Mem => SortKey::Cpu,
                        };
                        if state.show_processes {
                            needs_refresh = true;
                        }
                    }
                    KeyCode::Char('+') | KeyCode::Char('=') => {
                        state.interval_secs = (state.interval_secs + 1).min(60);
                    }
                    KeyCode::Char('-') => {
                        state.interval_secs = state.interval_secs.saturating_sub(1).max(1);
                    }
                    _ => {}
                }
                if needs_refresh {
                    let ca = CollectArgs {
                        disk: state.show_disk,
                        network: state.show_network,
                        processes: state.show_processes,
                        process_count: state.process_count,
                        sort_key: &state.sort_key,
                        include_timestamp: false,
                    };
                    collector.refresh();
                    metrics = collector.collect(&ca);
                    last_refresh = Instant::now();
                }
            }
        }

        if last_refresh.elapsed() >= Duration::from_secs(state.interval_secs) {
            let ca = CollectArgs {
                disk: state.show_disk,
                network: state.show_network,
                processes: state.show_processes,
                process_count: state.process_count,
                sort_key: &state.sort_key,
                include_timestamp: false,
            };
            collector.refresh();
            metrics = collector.collect(&ca);
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

fn border_style() -> Style {
    Style::default().fg(Color::Rgb(60, 80, 130))
}

fn title_style() -> Style {
    Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD)
}

fn header_style() -> Style {
    Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD)
}

fn draw_ui(f: &mut ratatui::Frame, metrics: &AllMetrics, state: &AppState) {
    let area = f.area();
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    draw_title(f, main_chunks[0]);
    draw_content(f, main_chunks[1], metrics, state);
    draw_footer(f, main_chunks[2], state);
}

fn draw_title(f: &mut ratatui::Frame, area: Rect) {
    let version = env!("CARGO_PKG_VERSION");
    let now = chrono::Local::now().format("%Y-%m-%d  %H:%M:%S").to_string();
    let left = format!("  lightsysmon v{}", version);
    let right = format!("{}  ", now);
    let pad_len = area
        .width
        .saturating_sub((left.len() + right.len()) as u16) as usize;
    let pad = " ".repeat(pad_len);

    let line = Line::from(vec![
        Span::styled(left, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::raw(pad),
        Span::styled(right, Style::default().fg(Color::White)),
    ]);
    f.render_widget(
        Paragraph::new(line).style(Style::default().bg(Color::Rgb(18, 28, 58))),
        area,
    );
}

fn draw_content(f: &mut ratatui::Frame, area: Rect, metrics: &AllMetrics, state: &AppState) {
    let has_cores = state.show_cores && !metrics.cpu.per_core.is_empty();
    let has_disk = state.show_disk && !metrics.disk.is_empty();
    let has_network = state.show_network && !metrics.network.is_empty();
    let has_processes = state.show_processes && !metrics.processes.is_empty();

    let core_height = if has_cores {
        let rows = (metrics.cpu.per_core.len() + 1) / 2;
        (rows as u16).saturating_add(2)
    } else {
        0
    };

    let mut constraints: Vec<Constraint> = vec![Constraint::Length(5)];
    if has_cores {
        constraints.push(Constraint::Length(core_height));
    }
    if has_disk {
        constraints.push(Constraint::Length((metrics.disk.len() as u16).saturating_add(3)));
    }
    if has_network {
        constraints.push(Constraint::Length(
            (metrics.network.len() as u16).saturating_add(3),
        ));
    }
    if has_processes {
        constraints.push(Constraint::Min(6));
    }
    constraints.push(Constraint::Min(0));

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    let mut idx = 0;

    draw_cpu_mem(f, chunks[idx], metrics);
    idx += 1;

    if has_cores {
        draw_cores(f, chunks[idx], metrics);
        idx += 1;
    }
    if has_disk {
        draw_disk(f, chunks[idx], metrics);
        idx += 1;
    }
    if has_network {
        draw_network(f, chunks[idx], metrics);
        idx += 1;
    }
    if has_processes {
        draw_processes(f, chunks[idx], metrics, state);
        idx += 1;
    }

    let _ = idx;
}

fn draw_cpu_mem(f: &mut ratatui::Frame, area: Rect, metrics: &AllMetrics) {
    let h_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // --- CPU panel ---
    let cpu_block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(" CPU ", title_style()))
        .border_style(border_style());
    let cpu_inner = cpu_block.inner(h_chunks[0]);
    f.render_widget(cpu_block, h_chunks[0]);

    let cpu_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1), Constraint::Min(0)])
        .split(cpu_inner);

    let cpu_pct = metrics.cpu.global_usage.clamp(0.0, 100.0);
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                format!("  {} cores @ {} MHz  ", metrics.cpu.cpu_count, metrics.cpu.frequency_mhz),
                Style::default().fg(Color::White),
            ),
            Span::styled(
                metrics.cpu.brand.clone(),
                Style::default().fg(Color::Rgb(140, 160, 200)),
            ),
        ])),
        cpu_chunks[0],
    );
    f.render_widget(
        Gauge::default()
            .gauge_style(Style::default().fg(usage_color(cpu_pct)))
            .ratio(cpu_pct / 100.0)
            .label(format!("{:.1}%", cpu_pct)),
        cpu_chunks[1],
    );

    // --- Memory panel ---
    let mem_block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(" Memory ", title_style()))
        .border_style(border_style());
    let mem_inner = mem_block.inner(h_chunks[1]);
    f.render_widget(mem_block, h_chunks[1]);

    let has_swap = metrics.memory.swap_total > 0;
    let mem_constraints: Vec<Constraint> = if has_swap {
        vec![Constraint::Length(1), Constraint::Length(1), Constraint::Min(0)]
    } else {
        vec![Constraint::Length(1), Constraint::Min(0)]
    };

    let mem_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(mem_constraints)
        .split(mem_inner);

    let ram_pct = metrics.memory.usage_pct.clamp(0.0, 100.0);
    f.render_widget(
        Gauge::default()
            .gauge_style(Style::default().fg(usage_color(ram_pct)))
            .ratio(ram_pct / 100.0)
            .label(format!(
                "RAM  {:.1}%   {} / {}",
                ram_pct,
                crate::display::plain::format_bytes(metrics.memory.used),
                crate::display::plain::format_bytes(metrics.memory.total),
            )),
        mem_chunks[0],
    );

    if has_swap {
        let swap_pct = metrics.memory.swap_pct.clamp(0.0, 100.0);
        f.render_widget(
            Gauge::default()
                .gauge_style(Style::default().fg(usage_color(swap_pct)))
                .ratio(swap_pct / 100.0)
                .label(format!(
                    "Swap {:.1}%   {} / {}",
                    swap_pct,
                    crate::display::plain::format_bytes(metrics.memory.swap_used),
                    crate::display::plain::format_bytes(metrics.memory.swap_total),
                )),
            mem_chunks[1],
        );
    }
}

fn draw_cores(f: &mut ratatui::Frame, area: Rect, metrics: &AllMetrics) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(" Per-Core CPU ", title_style()))
        .border_style(border_style());
    let inner = block.inner(area);
    f.render_widget(block, area);

    let cores = &metrics.cpu.per_core;
    let h_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(inner);

    let half = (cores.len() + 1) / 2;

    let mut left_lines: Vec<Line> = Vec::new();
    for (i, &u) in cores[..half.min(cores.len())].iter().enumerate() {
        let pct = u.clamp(0.0, 100.0);
        let filled = ((pct / 100.0) * 12.0) as usize;
        let bar = format!("{}{}", "█".repeat(filled), "░".repeat(12 - filled));
        left_lines.push(Line::from(vec![
            Span::styled(
                format!("Core {:2}  ", i),
                Style::default().fg(Color::Rgb(140, 160, 200)),
            ),
            Span::styled(bar, Style::default().fg(usage_color(pct))),
            Span::styled(
                format!("  {:5.1}%", pct),
                Style::default().fg(Color::White),
            ),
        ]));
    }

    let mut right_lines: Vec<Line> = Vec::new();
    for (i, &u) in cores[half..].iter().enumerate() {
        let pct = u.clamp(0.0, 100.0);
        let filled = ((pct / 100.0) * 12.0) as usize;
        let bar = format!("{}{}", "█".repeat(filled), "░".repeat(12 - filled));
        right_lines.push(Line::from(vec![
            Span::styled(
                format!("Core {:2}  ", i + half),
                Style::default().fg(Color::Rgb(140, 160, 200)),
            ),
            Span::styled(bar, Style::default().fg(usage_color(pct))),
            Span::styled(
                format!("  {:5.1}%", pct),
                Style::default().fg(Color::White),
            ),
        ]));
    }

    f.render_widget(Paragraph::new(left_lines), h_chunks[0]);
    f.render_widget(Paragraph::new(right_lines), h_chunks[1]);
}

fn draw_disk(f: &mut ratatui::Frame, area: Rect, metrics: &AllMetrics) {
    let rows: Vec<Row> = metrics
        .disk
        .iter()
        .map(|d| {
            let pct = d.used_pct.clamp(0.0, 100.0);
            Row::new(vec![
                Cell::from(d.mount_point.clone()),
                Cell::from(d.fs_type.clone()).style(Style::default().fg(Color::Rgb(140, 160, 200))),
                Cell::from(format!("{:.1}%", pct))
                    .style(Style::default().fg(usage_color(pct))),
                Cell::from(crate::display::plain::format_bytes(d.used)),
                Cell::from(crate::display::plain::format_bytes(d.total))
                    .style(Style::default().fg(Color::Rgb(140, 160, 200))),
            ])
        })
        .collect();

    let widths = [
        Constraint::Percentage(30),
        Constraint::Percentage(15),
        Constraint::Percentage(13),
        Constraint::Percentage(20),
        Constraint::Percentage(22),
    ];

    f.render_widget(
        Table::new(rows, widths)
            .header(
                Row::new(["Mount", "FS", "Used%", "Used", "Total"]).style(header_style()),
            )
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(Span::styled(" Disk ", title_style()))
                    .border_style(border_style()),
            ),
        area,
    );
}

fn draw_network(f: &mut ratatui::Frame, area: Rect, metrics: &AllMetrics) {
    let rows: Vec<Row> = metrics
        .network
        .iter()
        .map(|n| {
            Row::new(vec![
                Cell::from(n.interface.clone()),
                Cell::from(crate::display::plain::format_bytes(n.rx_bytes))
                    .style(Style::default().fg(Color::Green)),
                Cell::from(crate::display::plain::format_bytes(n.tx_bytes))
                    .style(Style::default().fg(Color::Cyan)),
                Cell::from(crate::display::plain::format_bytes(n.total_rx))
                    .style(Style::default().fg(Color::Rgb(140, 160, 200))),
                Cell::from(crate::display::plain::format_bytes(n.total_tx))
                    .style(Style::default().fg(Color::Rgb(140, 160, 200))),
            ])
        })
        .collect();

    let widths = [
        Constraint::Percentage(28),
        Constraint::Percentage(16),
        Constraint::Percentage(16),
        Constraint::Percentage(20),
        Constraint::Percentage(20),
    ];

    f.render_widget(
        Table::new(rows, widths)
            .header(
                Row::new(["Interface", "RX/s", "TX/s", "Total RX", "Total TX"])
                    .style(header_style()),
            )
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(Span::styled(" Network ", title_style()))
                    .border_style(border_style()),
            ),
        area,
    );
}

fn draw_processes(f: &mut ratatui::Frame, area: Rect, metrics: &AllMetrics, state: &AppState) {
    let sort_label = match state.sort_key {
        SortKey::Cpu => "CPU",
        SortKey::Mem => "Mem",
    };
    let title = format!(" Processes  by {} ", sort_label);

    let rows: Vec<Row> = metrics
        .processes
        .iter()
        .map(|p| {
            let cpu_color = usage_color(p.cpu_usage as f64);
            let mem_color = usage_color(p.memory_pct as f64);
            Row::new(vec![
                Cell::from(p.pid.to_string())
                    .style(Style::default().fg(Color::Rgb(140, 160, 200))),
                Cell::from(p.name.clone()),
                Cell::from(format!("{:.1}%", p.cpu_usage))
                    .style(Style::default().fg(cpu_color)),
                Cell::from(crate::display::plain::format_bytes(p.memory_bytes)),
                Cell::from(format!("{:.1}%", p.memory_pct))
                    .style(Style::default().fg(mem_color)),
            ])
        })
        .collect();

    let widths = [
        Constraint::Length(8),
        Constraint::Percentage(42),
        Constraint::Percentage(14),
        Constraint::Percentage(24),
        Constraint::Percentage(14),
    ];

    f.render_widget(
        Table::new(rows, widths)
            .header(Row::new(["PID", "Name", "CPU%", "Memory", "Mem%"]).style(header_style()))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(Span::styled(title, title_style()))
                    .border_style(border_style()),
            ),
        area,
    );
}

fn draw_footer(f: &mut ratatui::Frame, area: Rect, state: &AppState) {
    let key_s = Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD);
    let on_s = Style::default().fg(Color::Green).add_modifier(Modifier::BOLD);
    let off_s = Style::default().fg(Color::Rgb(80, 80, 80));
    let val_s = Style::default().fg(Color::Cyan);
    let sep = Span::raw("   ");

    let sort_label = match state.sort_key {
        SortKey::Cpu => "CPU",
        SortKey::Mem => "Mem",
    };

    let spans: Vec<Span> = vec![
        Span::raw(" "),
        Span::styled("[q]", key_s),
        Span::styled("Quit", Style::default().fg(Color::Red)),
        sep.clone(),
        Span::styled("[d]", key_s),
        Span::styled("Disk", if state.show_disk { on_s } else { off_s }),
        sep.clone(),
        Span::styled("[n]", key_s),
        Span::styled("Net", if state.show_network { on_s } else { off_s }),
        sep.clone(),
        Span::styled("[p]", key_s),
        Span::styled("Procs", if state.show_processes { on_s } else { off_s }),
        sep.clone(),
        Span::styled("[c]", key_s),
        Span::styled("Cores", if state.show_cores { on_s } else { off_s }),
        sep.clone(),
        Span::styled("[s]", key_s),
        Span::styled("Sort:", val_s),
        Span::styled(sort_label, val_s.add_modifier(Modifier::BOLD)),
        sep.clone(),
        Span::styled("[-/+]", key_s),
        Span::styled(format!("{}s", state.interval_secs), val_s),
    ];

    f.render_widget(
        Paragraph::new(Line::from(spans))
            .style(Style::default().bg(Color::Rgb(18, 24, 42))),
        area,
    );
}
