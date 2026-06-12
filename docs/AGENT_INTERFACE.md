# Agent Interface — handy-marine-voice

> **Role:** Voice-controlled marine autopilot — Handy hears you, CoCapn steers. No cloud required.
> **Language:** Rust
> **Build:** `cargo build --release`

---

## What an Agent Can Do Here

### Primary Actions

| Action | Entry Point | Description |
|--------|-------------|-------------|
| Build | `cargo build --release` | Compile voice autopilot stack |
| Run tests | `cargo test` | Voice command parsing + autopilot integration tests |
| Lint | `cargo clippy -- -D warnings` | Strict lint pass |
| Format | `cargo fmt` | Rust formatting |
| Start listener | `cargo run -- listen` | Start voice command listener on default audio device |
| Test command | `cargo run -- parse "turn to heading 045"` | Parse a voice command without hardware |
| Status | `cargo run -- status` | Check audio device + autopilot connection status |

### Source Layout

```
src/
├── voice/       — Speech-to-command pipeline
├── autopilot/   — Autopilot interface (NMEA over serial)
├── audio/       — Audio capture/playback
└── main.rs      — Entry point
```

---

## Environment Variables Required

```bash
# Required
GITHUB_TOKEN=<ghp_...>           # GitHub API access
DEEPINFRA_API_KEY=<key>          # LLM inference for voice parsing
OPENAI_API_KEY=<key>             # Fallback LLM

# Runtime
CARGO_TERM_COLOR=always
RUST_LOG=info                    # Log level

# Autopilot interface
AUTOPILOT_SERIAL=/dev/ttyACM0    # Serial port to marine autopilot
AUTOPILOT_BAUD=4800              # NMEA 0183 baud rate

# Audio
AUDIO_INPUT_DEVICE=default       # Microphone device
```

---

## Entry Points

### CLI
```bash
cargo build --release && ./target/release/handy-marine-voice [subcommand]

# Available subcommands:
#   listen          — Start continuous voice command listener
#   parse <phrase>  — Parse a text phrase as if spoken
#   status          — Check hardware and connection status
#   configure       — Configure audio and autopilot devices
```

### Library (crate)
```rust
use handy_marine_voice::parse::parse_command;
use handy_marine_voice::autopilot::Autopilot;

let command = parse_command("turn to heading 270")?;
let mut ap = Autopilot::new("/dev/ttyACM0", 4800)?;
ap.send_heading(270.0)?;
```

### Tests
```bash
cargo test                        # All tests
cargo test voice                  # Voice command parsing
cargo test autopilot              # Autopilot interface tests
```

---

## How to Report Back Results

1. **Write a Nail** in construct-coordination `memory/` with:
   - Voice command recognition accuracy metrics
   - Autopilot integration test results
   - Configuration changes
2. **Commit & push** changes
3. **Log to daily memory**

---

## Inter-repo Communication

| Repo | Dialogue |
|------|----------|
| **cocapn-marine** | Parsed voice commands → autopilot PID steering |
| **construct-coordination** | Command logs, calibration data |

---

## Dev Container

This repo includes a `.devcontainer/` with Rust toolchain + audio features pre-installed.

```bash
gh codespace create --repo SuperInstance/handy-marine-voice
```
