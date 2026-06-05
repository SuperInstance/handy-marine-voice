# LOW LEVEL — Handy Marine Voice

> *For contributors extending the voice-controlled autopilot.*

## Internal Architecture

```
handy-marine-voice/
├── src/
│   ├── main.rs        # Binary entry — stdin processing, daemon mode, CLI
│   ├── grammar.rs     # Voice command parser (regex patterns → MarineCommand)
│   └── bridge.rs      # Command → Action resolver + executor + MarineState
├── Cargo.toml
└── README.md
```

### Module Map

| Module | Responsibility | Key Types |
|--------|---------------|-----------|
| `grammar.rs` | Parse text → structured command | `MarineCommand` enum, `parse_command()` |
| `bridge.rs` | Resolve + execute marine actions | `MarineAction`, `MarineState`, `resolve_command()`, `execute_action()` |
| `main.rs` | CLI, stdin processing, init | `Mode` enum, `process_line()` |

## Key Internal Patterns

### Parse → Resolve → Execute Pipeline

```
process_line(line)
  → parse_command(text) → MarineCommand
  → resolve_command(cmd) → MarineAction
  → execute_action(action, &mut state) → String
  → JSON output to stdout + human-readable to stderr
```

### Grammar Pattern Design

Regexes are ordered by specificity (longest/full patterns first, short forms last). Each pattern captures groups for heading value, direction, speed, etc.

```rust
// Most specific first:
"hold heading 270" → exact match
"heading 270"     → short form
// Fall through to Unknown
```

### Heading Normalization

Headings are normalized to `[0, 360)` using `rem_euclid(360.0)`. Port turns are negative, starboard positive.

### Daemon Mode

In daemon mode, the binary reads lines from stdin forever. Each line is processed independently. State persists across commands for chaining (e.g., turn port 10 → steady).

## Testing

```bash
cargo test
```

29 tests covering: grammar parsing (all commands and variants), action resolution (heading, speed, escalate), execution (state updates, sensor reads, full pipeline).

## Debugging

- Set `RUST_LOG=handy_marine_voice=debug` for verbose logging
- `handy-marine-voice --daemon` shows state after each command on stderr
- JSON output on stdout shows full state for machine parsing

## Integration with Handy

Handy (cjpais/Handy) is a Tauri-based desktop STT app. Integration:
1. Handy transcribes speech → pipes text to `handy-marine-voice` via stdin
2. `handy-marine-voice` parses → resolves → executes
3. Returns JSON result; Handy reads it via TTS

In production: Handy actions system configured to invoke `handy-marine-voice`.

## Integration with CoCapn

In production:
- `MarineAction::SetHeading` dispatches to `cocapn-marine::autopilot::Autopilot::set_target_heading()`
- `MarineAction::QueryDepth` queries `cocapn-marine::sensor::DepthSensor`
- `MarineAction::EscalateTo` triggers `cocapn-core::stripe::rebalance()` or `cocapn-core::handoff::crossfade()`

## Future Work

- WebSocket/TCP listener for Handy IPC
- Real cocapn-marine/cocapn-core crate dependencies
- TTS readback of sensor data via Handy
- NMEA live data via serial/network
- ESP32 (Reflex-tier) deployment config
