//! Voice-to-marine bridge.
//!
//! Takes parsed voice commands and translates them to CoCapn marine actions:
//!   - Heading commands → Autopilot PID targets
//!   - Depth commands → Sensor queries
//!   - Deadband commands → Marine deadband config
//!   - Escalate commands → CoCapn core escalation (stripe rebalance / handoff)
//!
//! This is the glue between the three projects:
//!   Handy (voice-in) → MarineVoiceBridge → cocapn-marine + cocapn-core

use crate::grammar::{
    AutopilotState, DepthAlarmType, MarineCommand, TurnDirection,
};

/// Target tier for escalation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EscalationTarget {
    Cloud,
    Cortex,
    Backbone,
    Reflex,
}

impl std::str::FromStr for EscalationTarget {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().trim() {
            "cloud" | "api" | "server" | "aws" | "gcp" | "azure" => Ok(EscalationTarget::Cloud),
            "cortex" | "jetson" | "gpu" | "workstation" | "desktop" => Ok(EscalationTarget::Cortex),
            "backbone" | "pi" | "raspberry" | "raspberry pi" | "rpi" => Ok(EscalationTarget::Backbone),
            "reflex" | "esp32" | "esp" | "arduino" | "micro" => Ok(EscalationTarget::Reflex),
            _ => Err(format!("unknown escalation target: {}", s)),
        }
    }
}

/// A resolved marine action that can be dispatched.
#[derive(Debug, Clone, PartialEq)]
pub enum MarineAction {
    /// Set target heading on the PID autopilot (cocapn-marine autopilot::Autopilot.set_heading)
    SetHeading { target: f64 },

    /// Steer relative to current heading (synthesises a new target)
    TurnRelative { degrees: f64 },

    /// Hold current heading as target
    SteadyAsSheGoes,

    /// Set speed target (knots)
    SetSpeed { knots: f64 },

    /// Adjust speed by delta knots
    SpeedAdjust { delta: i32 },

    /// Kill throttle
    AllStop,

    /// Query depth sensor (cocapn-marine sensor::DepthSensor)
    QueryDepth,

    /// Set depth alarm threshold
    SetDepthAlarm { depth: f64, alarm_type: DepthAlarmType },

    /// Escalate to higher-tier compute (cocapn-core handoff/stripe)
    EscalateTo { target: EscalationTarget, reason: String },

    /// Toggle autopilot
    ToggleAutopilot { engage: bool },

    /// Set deadband tolerance (cocapn-marine deadband::HeadingDeadband)
    SetDeadband { tolerance_degrees: f64 },

    /// Report current GPS position (cocapn-marine sensor::GpsSensor)
    QueryPosition,

    /// Report system status (all sensors + autopilot state)
    QueryStatus,

    /// Voice feedback — read back data to the user
    ReadBack { message: String },
}

impl std::fmt::Display for MarineAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MarineAction::SetHeading { target } => write!(f, "SET_HEADING {}°", target),
            MarineAction::TurnRelative { degrees } => write!(f, "TURN_RELATIVE {}°", degrees),
            MarineAction::SteadyAsSheGoes => write!(f, "STEADY"),
            MarineAction::SetSpeed { knots } => write!(f, "SET_SPEED {} kn", knots),
            MarineAction::SpeedAdjust { delta } => write!(f, "SPEED_ADJUST {:+}", delta),
            MarineAction::AllStop => write!(f, "ALL_STOP"),
            MarineAction::QueryDepth => write!(f, "QUERY_DEPTH"),
            MarineAction::SetDepthAlarm { depth, alarm_type } => {
                write!(f, "SET_DEPTH_ALARM {:?} {}m", alarm_type, depth)
            }
            MarineAction::EscalateTo { target, reason } => {
                write!(f, "ESCALATE {:?}: {}", target, reason)
            }
            MarineAction::ToggleAutopilot { engage } => {
                write!(f, "TOGGLE_AUTOPILOT {}", if *engage { "ON" } else { "OFF" })
            }
            MarineAction::SetDeadband { tolerance_degrees } => {
                write!(f, "SET_DEADBAND ±{}°", tolerance_degrees)
            }
            MarineAction::QueryPosition => write!(f, "QUERY_POSITION"),
            MarineAction::QueryStatus => write!(f, "QUERY_STATUS"),
            MarineAction::ReadBack { message } => write!(f, "READBACK: {}", message),
        }
    }
}

/// Convert a parsed marine voice command into a resolved marine action.
///
/// This is the bridge between the voice grammar and the CoCapn execution layer.
/// The action can then be dispatched against cocapn-marine (autopilot, sensors)
/// and cocapn-core (stripe rebalance, handoff).
pub fn resolve_command(cmd: MarineCommand) -> MarineAction {
    match cmd {
        MarineCommand::HoldHeading(target) => {
            MarineAction::SetHeading {
                target: target.rem_euclid(360.0),
            }
        }
        MarineCommand::TurnRelative { direction, degrees } => {
            let signed = match direction {
                TurnDirection::Port => -degrees,
                TurnDirection::Starboard => degrees,
            };
            MarineAction::TurnRelative { degrees: signed }
        }
        MarineCommand::TurnHard(direction) => {
            let signed = match direction {
                TurnDirection::Port => -45.0,
                TurnDirection::Starboard => 45.0,
            };
            MarineAction::TurnRelative { degrees: signed }
        }
        MarineCommand::SteadyHeading => {
            MarineAction::SteadyAsSheGoes
        }
        MarineCommand::SetSpeed(knots) => {
            MarineAction::SetSpeed { knots: knots.max(0.0) }
        }
        MarineCommand::SpeedAdjust(delta) => {
            MarineAction::SpeedAdjust { delta }
        }
        MarineCommand::AllStop => {
            MarineAction::AllStop
        }
        MarineCommand::ReportDepth => {
            MarineAction::QueryDepth
        }
        MarineCommand::SetDepthAlarm { depth, alarm_type } => {
            MarineAction::SetDepthAlarm {
                depth: depth.max(0.0),
                alarm_type,
            }
        }
        MarineCommand::Escalate(target) => {
            let target = target.parse::<EscalationTarget>()
                .unwrap_or(EscalationTarget::Cloud);
            MarineAction::EscalateTo {
                target,
                reason: "voice command".into(),
            }
        }
        MarineCommand::ToggleAutopilot(state) => {
            MarineAction::ToggleAutopilot {
                engage: state == AutopilotState::Engage,
            }
        }
        MarineCommand::SetDeadband(tolerance) => {
            MarineAction::SetDeadband {
                tolerance_degrees: tolerance.max(0.5),
            }
        }
        MarineCommand::ReportPosition => {
            MarineAction::QueryPosition
        }
        MarineCommand::SystemStatus => {
            MarineAction::QueryStatus
        }
        MarineCommand::Unknown(text) => {
            MarineAction::ReadBack {
                message: format!("I didn't understand: {}", text),
            }
        }
    }
}

/// Execute a marine action against the CoCapn system.
/// In production, this would dispatch to the actual cocapn-marine and cocapn-core crates.
/// For now, it logs and returns a textual result.
pub fn execute_action(action: &MarineAction, state: &mut MarineState) -> String {
    match action {
        MarineAction::SetHeading { target } => {
            state.target_heading = Some(*target);
            format!("Heading set to {}°", target)
        }
        MarineAction::TurnRelative { degrees } => {
            let new_heading = state.current_heading.map(|h| (h + degrees).rem_euclid(360.0));
            state.target_heading = new_heading;
            match state.current_heading {
                Some(h) => format!("Turning {}° to new heading {:.1}°", degrees, (h + degrees).rem_euclid(360.0)),
                None => format!("Turn {}° (no position fix)", degrees),
            }
        }
        MarineAction::SteadyAsSheGoes => {
            if let Some(h) = state.current_heading {
                state.target_heading = Some(h);
                format!("Steady as she goes — holding {:.1}°", h)
            } else {
                "No heading data, cannot steady".to_string()
            }
        }
        MarineAction::SetSpeed { knots } => {
            state.target_speed_knots = Some(*knots);
            format!("Speed set to {} knots", knots)
        }
        MarineAction::SpeedAdjust { delta } => {
            let current = state.target_speed_knots.unwrap_or(6.0);
            let new_speed = (current + *delta as f64).max(0.0);
            state.target_speed_knots = Some(new_speed);
            if *delta > 0 {
                format!("Throttle up — speed now {:.1} knots", new_speed)
            } else if *delta < 0 {
                format!("Throttle down — speed now {:.1} knots", new_speed)
            } else {
                format!("Speed unchanged at {:.1} knots", new_speed)
            }
        }
        MarineAction::AllStop => {
            state.target_speed_knots = Some(0.0);
            "All stop — engines off".to_string()
        }
        MarineAction::QueryDepth => {
            let depth_text = match state.last_depth_reading {
                Some(d) => format!("Depth: {:.1} meters", d),
                None => "No depth reading available".to_string(),
            };
            depth_text
        }
        MarineAction::SetDepthAlarm { depth, alarm_type } => {
            state.depth_alarm = Some(*depth);
            match alarm_type {
                DepthAlarmType::Shallow => format!("Shallow alarm set at {:.1} meters", depth),
                DepthAlarmType::Deep => format!("Deep alarm set at {:.1} meters", depth),
            }
        }
        MarineAction::EscalateTo { target, reason } => {
            state.escalation = Some(*target);
            format!("Escalating to {:?}: {}", target, reason)
        }
        MarineAction::ToggleAutopilot { engage } => {
            state.autopilot_engaged = *engage;
            if *engage {
                "Autopilot engaged".to_string()
            } else {
                "Autopilot disengaged".to_string()
            }
        }
        MarineAction::SetDeadband { tolerance_degrees } => {
            state.deadband_tolerance = Some(*tolerance_degrees);
            format!("Deadband set to ±{}°", tolerance_degrees)
        }
        MarineAction::QueryPosition => {
            match (state.last_lat, state.last_lon) {
                (Some(lat), Some(lon)) => {
                    let lat_dir = if lat >= 0.0 { "N" } else { "S" };
                    let lon_dir = if lon >= 0.0 { "E" } else { "W" };
                    format!("Position: {:.4}°{}, {:.4}°{}", lat.abs(), lat_dir, lon.abs(), lon_dir)
                }
                _ => "No GPS fix".to_string(),
            }
        }
        MarineAction::QueryStatus => {
            let mut parts = vec![];
            if state.autopilot_engaged {
                if let Some(h) = state.target_heading {
                    parts.push(format!("Autopilot engaged, target {:.0}°", h));
                } else {
                    parts.push("Autopilot engaged, no target".to_string());
                }
            } else {
                parts.push("Autopilot disengaged".to_string());
            }
            if let Some(h) = state.current_heading {
                parts.push(format!("Heading {:.0}°", h));
            }
            if let Some(d) = state.last_depth_reading {
                parts.push(format!("Depth {:.1}m", d));
            }
            if let Some((lat, lon)) = state.last_lat.zip(state.last_lon) {
                parts.push(format!("Position: {:.4}/{:.4}", lat, lon));
            }
            if let Some(spd) = state.target_speed_knots {
                parts.push(format!("Speed target {:.1}kn", spd));
            }
            if parts.is_empty() {
                "All systems nominal. No sensor data yet.".to_string()
            } else {
                parts.join(". ")
            }
        }
        MarineAction::ReadBack { message } => {
            message.clone()
        }
    }
}

/// Runtime state for the marine voice bridge.
/// Mirrors the relevant state of cocapn-marine sensors and autopilot.
#[derive(Debug, Clone, Default)]
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

impl MarineState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Simulate updating from NMEA data (in production: from cocapn-marine sensors)
    pub fn update_from_nmea(&mut self, heading: Option<f64>, lat: Option<f64>, lon: Option<f64>, depth: Option<f64>) {
        if let Some(h) = heading {
            self.current_heading = Some(h);
        }
        if let (Some(lat), Some(lon)) = (lat, lon) {
            self.last_lat = Some(lat);
            self.last_lon = Some(lon);
        }
        if let Some(d) = depth {
            self.last_depth_reading = Some(d);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grammar::parse_command;

    #[test]
    fn test_heading_resolution() {
        let cmd = parse_command("hold heading 270");
        let action = resolve_command(cmd);
        assert_eq!(action, MarineAction::SetHeading { target: 270.0 });
    }

    #[test]
    fn test_turn_port_resolution() {
        let cmd = parse_command("turn port 10");
        let action = resolve_command(cmd);
        assert_eq!(action, MarineAction::TurnRelative { degrees: -10.0 });
    }

    #[test]
    fn test_turn_starboard_resolution() {
        let cmd = parse_command("come right 15");
        let action = resolve_command(cmd);
        assert_eq!(action, MarineAction::TurnRelative { degrees: 15.0 });
    }

    #[test]
    fn test_steady_resolution() {
        let cmd = parse_command("steady as she goes");
        let action = resolve_command(cmd);
        assert_eq!(action, MarineAction::SteadyAsSheGoes);
    }

    #[test]
    fn test_escalate_resolution() {
        let cmd = parse_command("escalate to cloud");
        let action = resolve_command(cmd);
        assert_eq!(action, MarineAction::EscalateTo {
            target: EscalationTarget::Cloud,
            reason: "voice command".into(),
        });
    }

    #[test]
    fn test_execute_set_heading() {
        let mut state = MarineState::new();
        let result = execute_action(&MarineAction::SetHeading { target: 180.0 }, &mut state);
        assert_eq!(result, "Heading set to 180°");
        assert_eq!(state.target_heading, Some(180.0));
    }

    #[test]
    fn test_execute_query_depth() {
        let mut state = MarineState::new();
        state.last_depth_reading = Some(12.5);
        let result = execute_action(&MarineAction::QueryDepth, &mut state);
        assert_eq!(result, "Depth: 12.5 meters");
    }

    #[test]
    fn test_execute_query_depth_no_data() {
        let mut state = MarineState::new();
        let result = execute_action(&MarineAction::QueryDepth, &mut state);
        assert_eq!(result, "No depth reading available");
    }

    #[test]
    fn test_execute_position() {
        let mut state = MarineState::new();
        state.update_from_nmea(None, Some(48.1173), Some(-122.5167), None);
        let result = execute_action(&MarineAction::QueryPosition, &mut state);
        assert!(result.contains("48.1173"));
        assert!(result.contains("N"));
        assert!(result.contains("122.5167"));
        assert!(result.contains("W"));
    }

    #[test]
    fn test_execute_all_stop() {
        let mut state = MarineState::new();
        state.target_speed_knots = Some(12.0);
        let result = execute_action(&MarineAction::AllStop, &mut state);
        assert_eq!(result, "All stop — engines off");
        assert_eq!(state.target_speed_knots, Some(0.0));
    }

    #[test]
    fn test_execute_escalate() {
        let mut state = MarineState::new();
        let result = execute_action(&MarineAction::EscalateTo {
            target: EscalationTarget::Cortex,
            reason: "voice command".into(),
        }, &mut state);
        assert_eq!(result, "Escalating to Cortex: voice command");
        assert_eq!(state.escalation, Some(EscalationTarget::Cortex));
    }

    #[test]
    fn test_full_pipeline() {
        // Voice → grammar → bridge → execution
        let mut state = MarineState::new();
        state.update_from_nmea(Some(45.0), Some(48.0), Some(-122.0), Some(10.0));

        let cmds = vec![
            "hold heading 270",
            "turn port 15",
            "steady",
            "report depth",
            "escalate to cloud",
            "deadband 3",
        ];

        let results: Vec<String> = cmds.iter().map(|&c| {
            let parsed = parse_command(c);
            let action = resolve_command(parsed);
            execute_action(&action, &mut state)
        }).collect();

        assert_eq!(results[0], "Heading set to 270°");
        assert_eq!(results[1], "Turning -15° to new heading 30.0°");
        assert_eq!(results[2], "Steady as she goes — holding 45.0°");
        assert_eq!(results[3], "Depth: 10.0 meters");
        assert_eq!(results[4], "Escalating to Cloud: voice command");
        assert_eq!(results[5], "Deadband set to ±3°");
    }
}
