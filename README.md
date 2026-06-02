# Handy-Marine-Voice 🎤⛵

**Voice-controlled marine autopilot. Handy hears you. CoCapn steers. No cloud required.**

Handy-Marine-Voice is an integration between [Handy](https://github.com/cjpais/Handy) (offline speech-to-text) and the CoCapn distributed agent framework ([cocapn-marine](https://github.com/SuperInstance/cocapn-marine) + [cocapn-core](https://github.com/SuperInstance/cocapn-core)). It translates spoken commands into marine autopilot actions — heading holds, turns, speed changes, depth queries, deadband configuration, and compute tier escalation.

## How It Works

```
You say "hold heading 270"
        │
        ▼
  ┌─────────────────┐
  │     Handy       │  Offline Whisper/Parakeet STT on your laptop
  │  (cjpais/Handy) │  No internet. No cloud. Just your voice.
  └───────┬─────────┘
          │ transcribed text: "hold heading 270"
          ▼
  ┌─────────────────┐
  │ Marine Voice    │  Voice command grammar → structured action
  │  Grammar        │  Regex patterns for navigation language
  └───────┬─────────┘
          │ MarineCommand::HoldHeading(270.0)
          ▼
  ┌─────────────────┐
  │  Voice Bridge   │  Resolve to MarineAction::SetHeading{target:270}
  │  (this crate)   │  Dispatch against CoCapn
  └───────┬─────────┘
          │
          ├──────────────► cocapn-marine  — PID autopilot setpoint change
          │
          ├──────────────► cocapn-marine  — NMEA sensor queries
          │
          └──────────────► cocapn-core    — Tier escalation / handoff
```

## Voice Commands

### Heading Control
| Say this | Result |
|----------|--------|
| `"hold heading 270"` | Set PID target to 270° |
| `"steer 180"` | Set target to 180° |
| `"turn port 10"` | Turn left 10° |
| `"come right 15"` | Turn right 15° |
| `"hard to starboard"` | Hard turn right (45°) |
| `"steady"` | Hold current heading |

### Speed Control
| Say this | Result |
|----------|--------|
| `"speed 12 knots"` | Set speed to 12 kn |
| `"speed up 5"` | Increase by 5 kn |
| `"throttle down 3"` | Decrease by 3 kn |
| `"all stop"` | Kill engines |

### Navigation
| Say this | Result |
|----------|--------|
| `"what's my depth"` | Read depth sounder |
| `"report position"` | Read GPS coordinates |
| `"status"` | Full system overview |

### Configuration
| Say this | Result |
|----------|--------|
| `"engage autopilot"` | Enable PID heading hold |
| `"disengage autopilot"` | Release to manual |
| `"set deadband 3"` | Set heading deadband ±3° |
| `"set shallow alarm 5"` | Depth alarm at 5m |

### Escalation
| Say this | Result |
|----------|--------|
| `"escalate to cloud"` | Push computation to cloud tier |
| `"handoff to jetson"` | Handoff to Cortex-tier compute |
| `"escalate"` | Auto-escalate to next available tier |

## Architecture

### Grammar Layer (`grammar.rs`)
- Regex-based parser for marine navigation language
- Normalizes variations: "hold heading", "steer", "set course"
- Returns structured `MarineCommand` enum values

### Bridge Layer (`bridge.rs`)
- Maps voice commands to CoCapn marine actions
- Maintains `MarineState` mirroring sensor/autopilot state
- Dispatch targets:

  | Action | Target |
  |--------|--------|
  | Heading/PID | `cocapn-marine::autopilot::Autopilot` |
  | Depth/GPS | `cocapn-marine::sensor::*` |
  | Deadband | `cocapn-marine::deadband::HeadingDeadband` |
  | Escalation | `cocapn-core::handoff` / `cocapn-core::stripe` |
  | Position | `cocapn-marine::sensor::GpsSensor` |

  Also maps to CoCapn core concepts:
  - **Deadband monitoring** → `cocapn-core::deadband::Deadband` (trigger when heading deviates beyond tolerance)
  - **Compute striping** → `cocapn-core::stripe::Stripe::rebalance()` (redistribute work across tiers)
  - **Handoff** → `cocapn-core::handoff` (escalate to higher tier with crossfade fallback)
  - **Push-down** → `cocapn-core::pushdown::PushDown` (de-escalate: "reflex can handle this, don't bother the cloud")
  - **Device tiers** → `cocapn-core::device::DeviceTier` (Reflex → Backbone → Cortex → Cloud)

## Integration with Handy

Handy is a Tauri desktop app written in Rust. It provides:

1. **Keyboard-shortcut recording** — tap your hotkey, speak, release
2. **Offline transcription** — Whisper or Parakeet models, entirely local
3. **Actions system** — Handy fires an action when a phrase is transcribed

To integrate, pipe Handy's transcription output (via its CLI or IPC) into `handy-marine-voice`:

```bash
# Pipe a single transcription
echo "hold heading 270" | handy-marine-voice

# Or run in daemon mode (reads from stdin)
handy-marine-voice --daemon
```

In production, Handy's actions system would be configured to invoke `handy-marine-voice` with each transcribed utterance. The `MarineCommand::ReadBack` action provides text that Handy can speak back to the user via TTS.

## Integration with CoCapn

This crate is designed as a drop-in voice layer for a CoCapn marine deployment:

```
Reflex tier (ESP32): bare-minimum steer + deadband
    │
Backbone tier (RPi):  Handy STT + Marine Voice Bridge + PID autopilot
    │
Cortex tier (Jetson): bathymetric mapping, sensor fusion
    │
Cloud tier:           chart databases, heavy training
```

The `escalate` command triggers `cocapn-core::stripe::rebalance()` or `cocapn-core::handoff::crossfade()` to push computation up the tier chain.

## Running

```bash
# Build
cargo build --release

# Run interactively (one command mode)
echo "engage autopilot" | ./target/release/handy-marine-voice

# Run in daemon mode (continuous stdin listening)
./target/release/handy-marine-voice --daemon

# Simulate sensor data
SIM_HEADING=270 SIM_LAT=48.1173 SIM_LON=-122.5167 SIM_DEPTH=15 \
    ./target/release/handy-marine-voice --daemon
```

## Testing

```bash
# All unit and integration tests
cargo test

# 29 tests covering:
#   - Grammar parsing (all command variants)
#   - Action resolution (voice → CoCapn action)
#   - Execution (state changes, readbacks)
#   - Full pipeline (transcription → autopilot action)
```

## Development Status

**Phase 1: Complete** ✅
- [x] Voice command grammar (heading, speed, depth, deadband, escalation)
- [x] Action resolution bridge to CoCapn
- [x] Execution simulation and state machine
- [x] 29 passing tests

**Phase 2: Production** 🚧
- [ ] WebSocket/TCP listener for Handy IPC
- [ ] Real cocapn-marine crate dependency
- [ ] Real cocapn-core crate dependency
- [ ] NMEA live data via serial/network
- [ ] PID autopilot feedback to voice TTS
- [ ] ESP32 (Reflex-tier) deployment config

## License

MIT
