# lightsysmon

A lightweight system monitor written in Rust. Monitors CPU, memory, disk, network, and processes in real time.

## Installation

### From GitHub Releases (macOS/Linux)

1. Go to [Releases](https://github.com/mikemich/lightsysmon/releases)
2. Download the latest binary for your platform
3. Make it executable: `chmod +x lightsysmon`
4. Run: `./lightsysmon watch`

### From Source

```bash
git clone https://github.com/mikemich/lightsysmon.git
cd lightsysmon
cargo build --release
./target/release/lightsysmon watch
```

## Usage

### Subcommands

```bash
# Continuously watch system metrics (CPU + memory by default)
lightsysmon watch

# Watch with disk, network, and process info
lightsysmon watch --disk --network --processes

# Watch with per-core CPU, 2-second refresh, and TUI dashboard
lightsysmon watch --cores --interval 2 --tui

# Watch with JSON output and CSV logging
lightsysmon watch --json --log metrics.csv

# Watch with threshold alerts
lightsysmon watch --alert-cpu 80 --alert-mem 90

# One-shot snapshot of current metrics
lightsysmon snapshot
lightsysmon snapshot --disk --network --json

# Show top processes (sorted by CPU by default)
lightsysmon top
lightsysmon top --sort mem --count 20

# Show static system information
lightsysmon info

# Generate shell completions
lightsysmon completions bash   # or zsh, fish, powershell
```

### Config File

Persistent defaults can be set in `~/.config/lightsysmon/config.toml`:

```toml
interval      = 2
show_disk     = true
show_network  = true
show_processes = true
process_count = 10
alert_cpu     = 80.0
alert_mem     = 90.0
```

## Sample Output

```
CPU Usage: 12.5%  (4 cores @ 2400 MHz)
Memory Usage: 56.3%  (8.9 GB / 16.0 GB)
```

With `--tui`, an interactive terminal dashboard is shown. Press **q** to quit.

## Requirements

- Rust 1.70+ (for building from source)
- macOS or Linux

## Building & Testing

```bash
cargo build
cargo test
```

## License

MIT
