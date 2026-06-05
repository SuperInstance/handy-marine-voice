# PLUG_AND_PLAY — Handy Marine Voice

> **Voice-controlled marine autopilot. Handy hears you. CoCapn steers. No cloud required.**

## What Is This?

A Rust binary that bridges your voice (via Handy offline STT) to CoCapn marine actions. Speak commands like "hold heading 270" or "report depth" — it parses, resolves, and executes against the autopilot and sensors.

## Why Should You Care?

- **Voice-controlled navigation** — Hands-free helm commands while underway
- **Offline-first** — Transcription via Handy's local Whisper/Parakeet, no internet needed
- **Multi-tier compute** — Escalate from Reflex (ESP32) to Cloud with a single command
- **Full autopilot integration** — PID heading hold, speed control, deadband configuration

## Quick Start

```bash
echo "hold heading 270" | handy-marine-voice
```

## ✨ Key Features

- Heading control: hold, turn, steady, hard turns
- Speed control: set, adjust, all stop
- Navigation queries: depth, position, status
- Configuration: deadband, autopilot toggle
- Escalation: handoff between compute tiers (Reflex → Backbone → Cortex → Cloud)

## Next Steps

| Guide | What It Covers |
|-------|----------------|
| [`GETTING_STARTED.md`](./GETTING_STARTED.md) | Build, run, voice commands |
| [`ARCHITECTURE.md`](./ARCHITECTURE.md) | Grammar, bridge, execution pipeline |
| [`API_REFERENCE.md`](./API_REFERENCE.md) | Command grammar and action types |
| [`LOW_LEVEL.md`](./LOW_LEVEL.md) | Internal structure, testing |

## Status

**v0.1.0 — Pre-production.** Grammar and bridge are complete with 29+ tests. Real cocapn-marine and cocapn-core crate dependencies not yet wired.
