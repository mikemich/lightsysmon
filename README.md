# lightsysmon

A lightweight system monitor written in Rust. Displays real-time CPU and memory usage.

## Installation

### From GitHub Releases (macOS/Linux)

1. Go to [Releases](https://github.com/yourusername/lightsysmon/releases)
2. Download the latest binary for your platform
3. Make it executable: `chmod +x lightsysmon`
4. Run: `./lightsysmon watch --interval 1`

### From Source

```bash
git clone https://github.com/yourusername/lightsysmon.git
cd lightsysmon
cargo build --release
./target/release/lightsysmon watch --interval 1
```

## Usage

```bash
# Monitor system metrics with 1-second refresh interval
./lightsysmon watch --interval 1

# Monitor with custom interval (in seconds)
./lightsysmon watch --interval 2
```

## Output

```
CPU Usage: 12.5%
Memory Usage: 56.3%
CPU Usage: 14.2%
Memory Usage: 56.5%
```

## Requirements

- Rust 1.70+ (for building from source)
- macOS or Linux

## License

MIT
