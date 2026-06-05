# GETTING STARTED — Handy Marine Voice

> *Estimated time: 5 minutes*

## Prerequisites

- Rust edition 2021+

## Installation

```bash
git clone https://github.com/SuperInstance/handy-marine-voice.git
cd handy-marine-voice
cargo build --release
```

## Your First 5 Minutes

### 1. Single Command Mode

```bash
echo "hold heading 270" | ./target/release/handy-marine-voice
```

Output shows the parsed command, resolved action, execution result, and current state.

### 2. Continuous Mode

```bash
./target/release/handy-marine-voice --daemon
```

Then type commands:
```
hold heading 180
turn port 10
steady
report depth
status
help
quit
```

### 3. Simulate Sensor Data

```bash
SIM_HEADING=270 SIM_LAT=48.1173 SIM_LON=-122.5167 SIM_DEPTH=15 \
    ./target/release/handy-marine-voice --daemon
```

### 4. Pipe from Handy

In production, Handy's action system pipes transcriptions to `handy-marine-voice`:

```bash
# Configure Handy to call:
# handy-marine-voice "$TRANSCRIPTION"
echo "engage autopilot" | handy-marine-voice
```

## Common Voice Commands

| Say | Result |
|-----|--------|
| `"hold heading 270"` | Set autopilot target |
| `"turn port 10"` | Turn left 10° |
| `"steady"` | Hold current heading |
| `"speed 12 knots"` | Set speed to 12 kn |
| `"all stop"` | Kill engines |
| `"report depth"` | Read depth sounder |
| `"where am I"` | Read GPS position |
| `"status"` | Full system overview |
| `"engage autopilot"` | Enable PID heading hold |
| `"set deadband 3"` | Set heading tolerance |
| `"escalate to cloud"` | Push to cloud compute |

## Next Steps

- [ARCHITECTURE.md](./ARCHITECTURE.md) — Design overview
- [API_REFERENCE.md](./API_REFERENCE.md) — Command grammar reference
- [LOW_LEVEL.md](./LOW_LEVEL.md) — Internal structure
