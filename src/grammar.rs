//! Voice command grammar for marine navigation.
//!
//! This module defines all voice commands that Handy can recognise
//! and maps them to CoCapn marine actions.
//!
//! Handy transcribes speech locally via Whisper/Parakeet → we parse
//! the text against these patterns → CoCapn steers.

use regex::Regex;

/// A parsed marine voice command.
#[derive(Debug, Clone, PartialEq)]
pub enum MarineCommand {
    // ── Heading commands ──
    /// "hold heading 270" / "steer 270" / "set course 180"
    HoldHeading(f64),

    /// "turn left 10" / "turn starboard 5" / "come right 15"
    TurnRelative { direction: TurnDirection, degrees: f64 },

    /// "come left" / "hard to starboard" (turn 45° standard)
    TurnHard(TurnDirection),

    /// "steady" / "steady as she goes" (hold current heading)
    SteadyHeading,

    // ── Speed commands ──
    /// "set speed 12 knots" / "speed 8"
    SetSpeed(f64),

    /// "throttle up 5" / "speed up 2"
    SpeedAdjust(i32),

    /// "stop engine" / "all stop"
    AllStop,

    // ── Depth commands ──
    /// "what's my depth" / "depth report" / "how deep is the water"
    ReportDepth,

    /// "set depth alarm 5" / "shallow alarm 3 meters"
    SetDepthAlarm { depth: f64, alarm_type: DepthAlarmType },

    // ── System commands ──
    /// "escalate to cloud" / "handoff to jetson"
    Escalate(String),

    /// "engage autopilot" / "disengage autopilot"
    ToggleAutopilot(AutopilotState),

    /// "set deadband 2" / "deadband 5 degrees"
    SetDeadband(f64),

    /// "report position" / "where am I"
    ReportPosition,

    /// "status" / "how are we doing"
    SystemStatus,

    /// Unknown or unparseable command
    Unknown(String),
}

/// Turn direction relative to current heading.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TurnDirection {
    Port,    // left
    Starboard, // right
}

/// Depth alarm type.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DepthAlarmType {
    Shallow,
    Deep,
}

/// Autopilot state toggle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AutopilotState {
    Engage,
    Disengage,
}

/// Parse a transcribed utterance into a marine command.
/// Returns `MarineCommand::Unknown` when no pattern matches.
pub fn parse_command(text: &str) -> MarineCommand {
    let text = text.trim().to_lowercase();

    // ── Heading: hold/steer/set heading ──
    let re_hold = Regex::new(r"^(hold|steer|set|go to?)\s*(heading|course)?\s*(\d{1,3})(\.\d+)?\s*(degrees?)?$").unwrap();
    if let Some(caps) = re_hold.captures(&text) {
        let heading: f64 = format!("{}{}", &caps[3], caps.get(4).map(|m| m.as_str()).unwrap_or(""))
            .parse()
            .unwrap_or(0.0);
        return MarineCommand::HoldHeading(heading);
    }

    // ── Heading: "heading 270" (short form) ──
    let re_heading = Regex::new(r"^heading\s+(\d{1,3})(\.\d+)?$").unwrap();
    if let Some(caps) = re_heading.captures(&text) {
        let heading: f64 = format!("{}{}", &caps[1], caps.get(2).map(|m| m.as_str()).unwrap_or(""))
            .parse()
            .unwrap_or(0.0);
        return MarineCommand::HoldHeading(heading);
    }

    // ── Turn relative ──
    let re_turn = Regex::new(r"^(turn|come)\s+(port|left|starboard|right)\s*(\d{1,3})?\s*(degrees?)?$").unwrap();
    if let Some(caps) = re_turn.captures(&text) {
        let dir_str = &caps[2];
        let direction = match dir_str {
            "port" | "left" => TurnDirection::Port,
            "starboard" | "right" => TurnDirection::Starboard,
            _ => unreachable!(),
        };
        if let Some(deg) = caps.get(3) {
            let degrees: f64 = deg.as_str().parse().unwrap_or(10.0);
            return MarineCommand::TurnRelative { direction, degrees };
        } else {
            // "come left" without degrees → hard turn
            return MarineCommand::TurnHard(direction);
        }
    }

    // ── Hard turns ──
    let re_hard = Regex::new(r"^hard\s+(a|to)?\s*(port|starboard|left|right|larboard)$").unwrap();
    if let Some(caps) = re_hard.captures(&text) {
        let dir_str = &caps[2];
        let direction = match dir_str {
            "port" | "left" | "larboard" => TurnDirection::Port,
            "starboard" | "right" => TurnDirection::Starboard,
            _ => unreachable!(),
        };
        return MarineCommand::TurnHard(direction);
    }

    // ── Steady ──
    let re_steady = Regex::new(r"^(steady|steady as she goes|hold steady)$").unwrap();
    if re_steady.is_match(&text) {
        return MarineCommand::SteadyHeading;
    }

    // ── Speed: set speed ──
    let re_speed = Regex::new(r"^speed\s+(\d{1,3}(\.\d+)?)\s*(knots?)?$").unwrap();
    if let Some(caps) = re_speed.captures(&text) {
        let speed: f64 = caps[1].parse().unwrap_or(0.0);
        return MarineCommand::SetSpeed(speed);
    }
    let re_set_speed = Regex::new(r"^(set|make)\s+speed\s+(\d{1,3}(\.\d+)?)\s*(knots?)?$").unwrap();
    if let Some(caps) = re_set_speed.captures(&text) {
        let speed: f64 = caps[2].parse().unwrap_or(0.0);
        return MarineCommand::SetSpeed(speed);
    }

    // ── Speed adjust ──
    let re_speed_up = Regex::new(r"^(speed|throttle)\s*(up|increase|down|decrease)\s*(\d{1,2})$").unwrap();
    if let Some(caps) = re_speed_up.captures(&text) {
        let amount: i32 = caps[3].parse().unwrap_or(1);
        let dir = caps.get(2).map(|m| m.as_str()).unwrap_or("");
        let multiplier = if dir == "up" || dir == "increase" { 1 } else { -1 };
        return MarineCommand::SpeedAdjust(amount * multiplier);
    }

    // ── All stop ──
    let re_stop = Regex::new(r"^(all stop|stop engine|kill engine|stop the boat)$").unwrap();
    if re_stop.is_match(&text) {
        return MarineCommand::AllStop;
    }

    // ── Depth report ──
    let re_depth = Regex::new(r"^(what'?s? my depth|depth report|how deep|what'?s? the depth|report depth)$").unwrap();
    if re_depth.is_match(&text) {
        return MarineCommand::ReportDepth;
    }

    // ── Depth alarm ──
    let re_shallow = Regex::new(r"^(set|make)\s+(shallow|deep)\s+alarm\s+(\d{1,3}(\.\d+)?)\s*(meters?|m|feet?|ft)?$").unwrap();
    if let Some(caps) = re_shallow.captures(&text) {
        let depth: f64 = caps[3].parse().unwrap_or(0.0);
        let alarm_type = match &caps[2] {
            "shallow" => DepthAlarmType::Shallow,
            "deep" => DepthAlarmType::Deep,
            _ => DepthAlarmType::Shallow,
        };
        return MarineCommand::SetDepthAlarm { depth, alarm_type };
    }

    // ── Escalate ──
    if text == "escalate" {
        return MarineCommand::Escalate("next available tier".to_string());
    }
    let re_escalate = Regex::new(r"^escalate\s+(to\s+)?(.+)$").unwrap();
    if let Some(caps) = re_escalate.captures(&text) {
        let target = caps[2].trim().to_string();
        return MarineCommand::Escalate(target);
    }
    let re_handoff = Regex::new(r"^handoff\s+(to\s+)?(.+)$").unwrap();
    if let Some(caps) = re_handoff.captures(&text) {
        let target = caps[2].trim().to_string();
        return MarineCommand::Escalate(target);
    }

    // ── Toggle autopilot ──
    let re_engage = Regex::new(r"^engage\s+autopilot$").unwrap();
    if re_engage.is_match(&text) {
        return MarineCommand::ToggleAutopilot(AutopilotState::Engage);
    }
    let re_disengage = Regex::new(r"^(disengage|disable|turn off)\s+autopilot$").unwrap();
    if re_disengage.is_match(&text) {
        return MarineCommand::ToggleAutopilot(AutopilotState::Disengage);
    }

    // ── Set deadband ──
    let re_deadband = Regex::new(r"^(set\s+)?deadband\s*(\d{1,3}(\.\d+)?)\s*(degrees?)?$").unwrap();
    if let Some(caps) = re_deadband.captures(&text) {
        let val: f64 = caps[2].parse().unwrap_or(3.0);
        return MarineCommand::SetDeadband(val);
    }

    // ── Position report ──
    let re_pos = Regex::new(r"^(report\s+)?(position|location|where am i|where are we)$").unwrap();
    if re_pos.is_match(&text) {
        return MarineCommand::ReportPosition;
    }

    // ── System status ──
    let re_status = Regex::new(r"^(system\s+)?(status|how are we|report)$").unwrap();
    if re_status.is_match(&text) {
        return MarineCommand::SystemStatus;
    }

    MarineCommand::Unknown(text.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Heading ──
    #[test]
    fn test_hold_heading() {
        assert_eq!(parse_command("hold heading 270"), MarineCommand::HoldHeading(270.0));
        assert_eq!(parse_command("steer 180"), MarineCommand::HoldHeading(180.0));
        assert_eq!(parse_command("set course 45"), MarineCommand::HoldHeading(45.0));
        assert_eq!(parse_command("heading 270"), MarineCommand::HoldHeading(270.0));
        assert_eq!(parse_command("hold heading 270 degrees"), MarineCommand::HoldHeading(270.0));
    }

    #[test]
    fn test_turn_relative() {
        assert_eq!(
            parse_command("turn port 10"),
            MarineCommand::TurnRelative { direction: TurnDirection::Port, degrees: 10.0 }
        );
        assert_eq!(
            parse_command("come right 15"),
            MarineCommand::TurnRelative { direction: TurnDirection::Starboard, degrees: 15.0 }
        );
        assert_eq!(
            parse_command("turn starboard 5 degrees"),
            MarineCommand::TurnRelative { direction: TurnDirection::Starboard, degrees: 5.0 }
        );
    }

    #[test]
    fn test_turn_hard() {
        assert_eq!(
            parse_command("come left"),
            MarineCommand::TurnHard(TurnDirection::Port)
        );
        assert_eq!(
            parse_command("hard to starboard"),
            MarineCommand::TurnHard(TurnDirection::Starboard)
        );
        assert_eq!(
            parse_command("hard a port"),
            MarineCommand::TurnHard(TurnDirection::Port)
        );
    }

    #[test]
    fn test_steady() {
        assert_eq!(parse_command("steady"), MarineCommand::SteadyHeading);
        assert_eq!(parse_command("steady as she goes"), MarineCommand::SteadyHeading);
    }

    // ── Speed ──
    #[test]
    fn test_set_speed() {
        assert_eq!(parse_command("speed 12"), MarineCommand::SetSpeed(12.0));
        assert_eq!(parse_command("speed 8 knots"), MarineCommand::SetSpeed(8.0));
        assert_eq!(parse_command("set speed 6.5"), MarineCommand::SetSpeed(6.5));
    }

    #[test]
    fn test_speed_adjust() {
        assert_eq!(parse_command("speed up 5"), MarineCommand::SpeedAdjust(5));
        assert_eq!(parse_command("throttle down 3"), MarineCommand::SpeedAdjust(-3));
        assert_eq!(parse_command("throttle up 2"), MarineCommand::SpeedAdjust(2));
    }

    #[test]
    fn test_all_stop() {
        assert_eq!(parse_command("all stop"), MarineCommand::AllStop);
        assert_eq!(parse_command("stop engine"), MarineCommand::AllStop);
    }

    // ── Depth ──
    #[test]
    fn test_depth_report() {
        assert_eq!(parse_command("what's my depth"), MarineCommand::ReportDepth);
        assert_eq!(parse_command("depth report"), MarineCommand::ReportDepth);
        assert_eq!(parse_command("how deep"), MarineCommand::ReportDepth);
        assert_eq!(parse_command("report depth"), MarineCommand::ReportDepth);
    }

    #[test]
    fn test_depth_alarm() {
        assert_eq!(
            parse_command("set shallow alarm 3"),
            MarineCommand::SetDepthAlarm { depth: 3.0, alarm_type: DepthAlarmType::Shallow }
        );
        assert_eq!(
            parse_command("set deep alarm 15 meters"),
            MarineCommand::SetDepthAlarm { depth: 15.0, alarm_type: DepthAlarmType::Deep }
        );
    }

    // ── Escalate ──
    #[test]
    fn test_escalate() {
        assert_eq!(
            parse_command("escalate to cloud"),
            MarineCommand::Escalate("cloud".to_string())
        );
        assert_eq!(
            parse_command("handoff to jetson"),
            MarineCommand::Escalate("jetson".to_string())
        );
        assert_eq!(
            parse_command("escalate"),
            MarineCommand::Escalate("next available tier".to_string())
        );
    }

    // ── Autopilot ──
    #[test]
    fn test_toggle_autopilot() {
        assert_eq!(
            parse_command("engage autopilot"),
            MarineCommand::ToggleAutopilot(AutopilotState::Engage)
        );
        assert_eq!(
            parse_command("disengage autopilot"),
            MarineCommand::ToggleAutopilot(AutopilotState::Disengage)
        );
    }

    // ── Deadband ──
    #[test]
    fn test_deadband() {
        assert_eq!(parse_command("deadband 5"), MarineCommand::SetDeadband(5.0));
        assert_eq!(parse_command("set deadband 2.5"), MarineCommand::SetDeadband(2.5));
        assert_eq!(parse_command("deadband 2 degrees"), MarineCommand::SetDeadband(2.0));
    }

    // ── Position ──
    #[test]
    fn test_position() {
        assert_eq!(parse_command("where am I"), MarineCommand::ReportPosition);
        assert_eq!(parse_command("report position"), MarineCommand::ReportPosition);
        assert_eq!(parse_command("position"), MarineCommand::ReportPosition);
    }

    // ── Status ──
    #[test]
    fn test_status() {
        assert_eq!(parse_command("status"), MarineCommand::SystemStatus);
        assert_eq!(parse_command("system status"), MarineCommand::SystemStatus);
        assert_eq!(parse_command("how are we"), MarineCommand::SystemStatus);
    }

    // ── Unknown ──
    #[test]
    fn test_unknown() {
        match parse_command("what's the weather like") {
            MarineCommand::Unknown(s) => assert_eq!(s, "what's the weather like"),
            _ => panic!("Expected Unknown"),
        }
    }
}
