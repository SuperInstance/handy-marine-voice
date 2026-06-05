# API Reference — Handy Marine Voice

> *Voice command grammar and action types. MSRV: Rust edition 2021.*

---

## `MarineCommand` (grammar output)

```rust
pub enum MarineCommand {
    HoldHeading(f64),
    TurnRelative { direction: TurnDirection, degrees: f64 },
    TurnHard(TurnDirection),
    SteadyHeading,
    SetSpeed(f64),
    SpeedAdjust(i32),
    AllStop,
    ReportDepth,
    SetDepthAlarm { depth: f64, alarm_type: DepthAlarmType },
    Escalate(String),
    ToggleAutopilot(AutopilotState),
    SetDeadband(f64),
    ReportPosition,
    SystemStatus,
    Unknown(String),
}
```

### Command patterns (input → command)

| Input | Result |
|-------|--------|
| `"hold heading 270"`, `"steer 180"` | `HoldHeading(270.0)` |
| `"turn port 10"`, `"come right 15"` | `TurnRelative { direction, degrees }` |
| `"hard to starboard"`, `"come left"` | `TurnHard(direction)` |
| `"steady"`, `"steady as she goes"` | `SteadyHeading` |
| `"speed 12 knots"`, `"set speed 8"` | `SetSpeed(12.0)` |
| `"speed up 5"`, `"throttle down 3"` | `SpeedAdjust(5)` |
| `"all stop"`, `"stop engine"` | `AllStop` |
| `"report depth"`, `"how deep"` | `ReportDepth` |
| `"set shallow alarm 3"` | `SetDepthAlarm { depth: 3.0, shallow }` |
| `"escalate to cloud"`, `"handoff to jetson"` | `Escalate("cloud")` |
| `"engage autopilot"`, `"disengage autopilot"` | `ToggleAutopilot(state)` |
| `"set deadband 3"`, `"deadband 2.5"` | `SetDeadband(3.0)` |
| `"where am I"`, `"report position"` | `ReportPosition` |
| `"status"`, `"system status"` | `SystemStatus` |

---

## `MarineAction` (bridge output)

```rust
pub enum MarineAction {
    SetHeading { target: f64 },              // → cocapn-marine autopilot
    TurnRelative { degrees: f64 },
    SteadyAsSheGoes,
    SetSpeed { knots: f64 },                  // → cocapn-marine speed control
    SpeedAdjust { delta: i32 },
    AllStop,
    QueryDepth,                                // → cocapn-marine depth sensor
    SetDepthAlarm { depth: f64, alarm_type: DepthAlarmType },
    EscalateTo { target: EscalationTarget, reason: String },  // → cocapn-core
    ToggleAutopilot { engage: bool },
    SetDeadband { tolerance_degrees: f64 },    // → cocapn-marine deadband
    QueryPosition,                              // → cocapn-marine GPS
    QueryStatus,
    ReadBack { message: String },              // → TTS
}
```

---

## `MarineState` (runtime state)

```rust
pub struct MarineState {
    pub autopilot_engaged: bool,
    pub target_heading: Option<f64>,
    pub current_heading: Option<f64>,
    pub target_speed_knots: Option<f64>,
    pub last_lat: Option<f64>,
    pub last_lon: Option<f64>,
    pub last_depth_reading: Option<f64>,
    pub depth_alarm: Option<f64>,
    pub deadband_tolerance: Option<f64>,
    pub escalation: Option<EscalationTarget>,
}
```

---

## `EscalationTarget`

```rust
pub enum EscalationTarget {
    Cloud,     // API tier — heavy compute
    Cortex,    // Jetson, GPU workstation
    Backbone,  // Raspberry Pi
    Reflex,    // ESP32, microcontrollers
}
```

## CLI Usage

```bash
# Single command
echo "hold heading 270" | handy-marine-voice

# Daemon mode (continuous)
handy-marine-voice --daemon

# Simulate sensor data
SIM_HEADING=270 SIM_LAT=48.1 SIM_LON=-122.5 SIM_DEPTH=15 \
    handy-marine-voice --daemon
```

## Output Format

JSON on stdout:
```json
{
  "command": "hold heading 270",
  "action": "SET_HEADING 270°",
  "result": "Heading set to 270°",
  "state": { "autopilot_engaged": false, "target_heading": 270.0, ... }
}
```

Human-readable summary on stderr.

## Minimum Supported Rust Version

Rust edition 2021.
