![Logo](https://i.imgur.com/jRecwhn.png)
# Symvea Daemon - Version 0.1

### Code is currently experimental, research only currently.

High-performance daemon for symbol analysis and code intelligence.

## Installation
```bash
git clone https://github.com/Symvea/symvead.git
cd symvead
```

## Build
```bash
cargo build --release
```

## Usage
```bash
# Run the daemon
cargo run

# Run with custom config
cargo run -- --config symvea.toml

# Run release build
./target/release/symvead
```

Default port: `24096`
