# Architecture — Handy Marine Voice

> *Voice pipeline: speech → transcription → grammar parse → action resolution → CoCapn execution.*

## Design Goals

1. **Offline-first** — All voice processing is local. Handy's Whisper/Parakeet runs on your machine.
2. **Simple pipeline** — Transcribe → Parse → Resolve → Execute. Each stage is testable independently.
3. **Tiered compute** — Voice commands can escalate computation between Reflex → Backbone → Cortex → Cloud tiers.

## High-Level Overview

```
"You say 'hold heading 270'"
         │
         ▼
  ┌──────────────┐
  │   Handy STT  │  Offline Whisper/Parakeet (local, no cloud)
  └──────┬───────┘
         │ text: "hold heading 270"
         ▼
  ┌──────────────┐
  │   grammar    │  Regex patterns → MarineCommand::HoldHeading(270)
  │   parser     │  validate heading range, direction keywords
  └──────┬───────┘
         │
         ▼
  ┌──────────────┐
  │   bridge     │  MarineCommand → MarineAction::SetHeading{target:270}
  │   resolver   │  Maps voice intent to CoCapn action
  └──────┬───────┘
         │
         ▼
  ┌──────────────┐
  │   executor   │  Updates MarineState, returns result text
  │   (sim)      │  In production: dispatches to cocapn-marine/cocapn-core
  └──────────────┘
         │
         ▼
  "Heading set to 270°"
```

## Core Components

### Grammar Parser (`grammar.rs`)
Regex-based parsing of marine navigation language. 15+ command patterns covering heading, speed, depth, escalation, autopilot toggling, and deadband. Returns typed `MarineCommand` enum.

### Bridge Resolver (`bridge.rs`)
Maps parsed voice commands to CoCapn marine actions. `MarineCommand` → `MarineAction` resolution. Handles direction normalization (port/starboard), input validation, and default values.

### Executor (`bridge.rs`)
Simulates execution against `MarineState` (in production: dispatches to cocapn-marine autopilot and sensors). Returns human-readable results for TTS readback.

### MarineState (`bridge.rs`)
Mirrors relevant CoCapn sensor and autopilot state. Tracks heading, position (lat/lon), depth, speed, autopilot engagement, deadband, escalation target.

## Data Flow

```
voice → parse_command(text) → MarineCommand
     → resolve_command(cmd) → MarineAction
     → execute_action(action, &mut state) → String result
     → JSON output + stderr log
```

## Key Design Decisions

### Regex-Based Grammar
Lightweight and deterministic. No NLP, no ML. Just regex patterns for each command variant. Trade-off: less flexible than NLU, but predictable and testable (29 tests).

### Simulation-First
Bridge executes against in-memory `MarineState`. In production, it would dispatch to real cocapn-marine and cocapn-core crates. Currently those aren't wired as dependencies.

## Dependencies

| Dependency | Why |
|-----------|-----|
| `regex` | Command pattern matching |
| `serde` / `serde_json` | JSON output for machine consumers |
| `thiserror` | Error types |
| `log` / `env_logger` | Runtime logging |

## Extension Points

- **New command patterns** — Add regex to `grammar.rs` + variant to `MarineCommand`
- **New marine actions** — Add variant to `MarineAction` + resolution in `resolve_command`
- **Real CoCapn dispatch** — Replace `execute_action` mock with actual crate calls

## See Also

- [GETTING_STARTED.md](./GETTING_STARTED.md) — Quick start
- [API_REFERENCE.md](./API_REFERENCE.md) — Command grammar
- [LOW_LEVEL.md](./LOW_LEVEL.md) — Internal details
