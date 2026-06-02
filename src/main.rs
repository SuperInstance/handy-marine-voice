//! handy-marine-voice
//!
//! Voice-controlled boat navigation.
//! Handy hears you. CoCapn steers. No cloud required.
//!
//! This binary provides the integration layer:
//!   1. Receives transcribed speech from Handy (via stdin, WebSocket, or Tauri IPC)
//!   2. Parses against the marine voice grammar
//!   3. Resolves to CoCapn marine actions
//!   4. Dispatches to cocapn-marine (autopilot PID, NMEA sensors) and cocapn-core (stripe/handoff)
//!
//! # Example
//!
//! ```bash
//! # Pipe a brief transcription through Handy's CLI
//! echo "hold heading 270" | handy-marine-voice
//!
//! # Or run in daemon mode listening on stdin for continuous transcription
//! handy-marine-voice --daemon
//! ```

mod grammar;
mod bridge;

use std::io::{self, BufRead, Write};
use bridge::{execute_action, MarineState, resolve_command};
use grammar::parse_command;

/// Available runtime modes.
enum Mode {
    /// Process a single command from stdin and print result
    Once,
    /// Continuous mode — read lines from stdin forever
    Daemon,
}

fn print_usage() {
    eprintln!(
        r#"Handy-Marine-Voice — Speech-to-Stars (Voice-Controlled Autopilot)

Usage:
    handy-marine-voice [--daemon]

Commands (say these into Handy):
    Heading:
        "hold heading 270"          Set target heading
        "turn port 10"              Turn left 10 degrees
        "come right 15"             Turn right 15 degrees
        "hard to port"              45° port turn
        "steady"                    Hold current heading
        "steady as she goes"

    Speed:
        "speed 12 knots"            Set speed
        "speed up 5"                Increase speed
        "throttle down 3"           Decrease speed
        "all stop"                  Kill engines

    Navigation:
        "report depth"              Read depth sounder
        "report position"           Read GPS position
        "where am I"
        "status"                    Full system status

    Configuration:
        "set deadband 3"            Set autopilot deadband (degrees)
        "engage autopilot"          Engage PID heading hold
        "disengage autopilot"       Release to manual

    Escalation:
        "escalate to cloud"         Push computation to cloud tier
        "handoff to jetson"         Handoff to cortex-tier compute
        "escalate"                  Auto-escalate to next tier
"#
    );
}

fn process_line(line: &str, state: &mut MarineState, output: &mut dyn Write) -> io::Result<()> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return Ok(());
    }

    // Special commands
    match trimmed {
        "help" | "--help" | "-h" => {
            print_usage();
            return Ok(());
        }
        "quit" | "exit" => {
            writeln!(output, "BYE")?;
            return Ok(());
        }
        _ => {}
    }

    // Parse → resolve → execute
    let parsed = parse_command(trimmed);
    let action = resolve_command(parsed);
    let result = execute_action(&action, state);

    // Output as JSON for machine consumers, or plain text
    let json_output = serde_json::json!({
        "command": trimmed,
        "action": format!("{}", action),
        "result": result,
        "state": {
            "autopilot_engaged": state.autopilot_engaged,
            "target_heading": state.target_heading,
            "current_heading": state.current_heading,
            "target_speed_knots": state.target_speed_knots,
            "depth_alarm": state.depth_alarm,
            "deadband_tolerance": state.deadband_tolerance,
        }
    });

    writeln!(output, "{}", serde_json::to_string_pretty(&json_output)?)?;
    // Also print a human-readable summary on stderr
    eprintln!("🎤 {}  →  {}", trimmed, result);

    Ok(())
}

fn run(mode: Mode, state: &mut MarineState) -> io::Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut stdout_lock = stdout.lock();

    match mode {
        Mode::Once => {
            for line in stdin.lock().lines() {
                process_line(&line?, state, &mut stdout_lock)?;
            }
        }
        Mode::Daemon => {
            eprintln!("🔊 handy-marine-voice daemon ready. Speak into Handy.");
            for line in stdin.lock().lines() {
                process_line(&line?, state, &mut stdout_lock)?;
            }
        }
    }

    Ok(())
}

fn main() -> io::Result<()> {
    // Initialise logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();

    let args: Vec<String> = std::env::args().collect();
    let daemon = args.iter().any(|a| a == "--daemon");

    if args.iter().any(|a| a == "--help" || a == "-h") {
        print_usage();
        return Ok(());
    }

    let sim_heading: Option<f64> = std::env::var("SIM_HEADING").ok().and_then(|v| v.parse().ok());
    let sim_lat: Option<f64> = std::env::var("SIM_LAT").ok().and_then(|v| v.parse().ok());
    let sim_lon: Option<f64> = std::env::var("SIM_LON").ok().and_then(|v| v.parse().ok());
    let sim_depth: Option<f64> = std::env::var("SIM_DEPTH").ok().and_then(|v| v.parse().ok());

    let mut state = MarineState::new();

    // Simulate some NMEA sensor input for demo/testing
    if sim_heading.is_some() || sim_lat.is_some() || sim_lon.is_some() || sim_depth.is_some() {
        state.update_from_nmea(sim_heading, sim_lat, sim_lon, sim_depth);
    } else {
        // Default simulation data
        state.update_from_nmea(Some(45.0), Some(48.1173), Some(-122.5167), Some(15.0));
    }

    if daemon {
        eprintln!("🌊 Marine Voice Bridge ready");
        eprintln!("   Heading: {:.1}°", state.current_heading.unwrap_or(0.0));
        eprintln!("   Position: {:.4}, {:.4}", state.last_lat.unwrap_or(0.0), state.last_lon.unwrap_or(0.0));
        eprintln!("   Depth: {:.1}m", state.last_depth_reading.unwrap_or(0.0));
        eprintln!("   Say 'help' for command list.");
        run(Mode::Daemon, &mut state)
    } else {
        run(Mode::Once, &mut state)
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use grammar::MarineCommand;

    #[test]
    fn test_parse_and_execute_pipeline() {
        let mut state = MarineState::new();
        state.update_from_nmea(Some(90.0), Some(48.0), Some(-122.0), Some(10.0));

        // Test the full pipeline for each major command category
        let test_cases = vec![
            ("hold heading 180", "Heading set to 180°"),
            ("turn port 30", "Turning -30° to new heading 60.0°"),
            ("steady", "Steady as she goes — holding 90.0°"),
            ("speed 12", "Speed set to 12 knots"),
            ("speed up 5", "Throttle up — speed now 11.0 knots"),
            ("all stop", "All stop — engines off"),
            ("report depth", "Depth: 10.0 meters"),
            ("set shallow alarm 3", "Shallow alarm set at 3.0 meters"),
            ("escalate to cloud", "Escalating to Cloud: voice command"),
            ("disengage autopilot", "Autopilot disengaged"),
            ("engage autopilot", "Autopilot engaged"),
            ("deadband 5", "Deadband set to ±5°"),
            ("report position", "Position: 48.0000°N, 122.0000°W"),
            ("status", "Autopilot disengaged. Heading 90°"),
        ];

        // Reset state for each test to avoid chaining dependencies
        for (input, expected) in test_cases {
            let mut test_state = MarineState::new();
            test_state.update_from_nmea(Some(90.0), Some(48.0), Some(-122.0), Some(10.0));

            let mut buf: Vec<u8> = Vec::new();
            process_line(input, &mut test_state, &mut buf).unwrap();
            let output = String::from_utf8(buf).unwrap();
            assert!(
                output.contains(expected),
                "Input: '{}' — expected output containing '{}', got: {}",
                input, expected, output
            );
        }
    }

    #[test]
    fn test_unknown_command() {
        let mut state = MarineState::new();
        let mut buf: Vec<u8> = Vec::new();
        process_line("what's the weather", &mut state, &mut buf).unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("I didn't understand"));
    }
}
