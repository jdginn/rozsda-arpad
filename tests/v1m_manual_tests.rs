// Semi-manual test suite for v1m hardware
//
// This test suite provides two types of tests:
// 1. Output tests: Send messages to v1m and prompt user to verify hardware behavior
// 2. Input tests: Prompt user to interact with hardware and verify messages received
//
// Run with: cargo test --test v1m_manual_tests -- --nocapture --test-threads=1

use std::collections::HashMap;
use std::io::{self, Write};
use std::time::Duration;

use crossbeam_channel::{Receiver, Sender, bounded};
use midir::{Ignore, MidiInput, MidiInputPort, MidiOutput, MidiOutputConnection};

use arpad_rust::midi::v1m::{
    ArmLEDMsg, BottomScribbleStripLine1TextMsg, BottomScribbleStripLine2TextMsg, Color, DawId,
    DownstreamMsg, EncoderRingMode, EncoderRingMsg, FaderAbsMsg, LEDState, MeterMsg, MuteLEDMsg,
    ScribbleStripBackgroundColorMsg, ScribbleStripLine1TextMsg, ScribbleStripLine2TextMsg, Slot,
    SoloLEDMsg, TouchScreenLayer, TouchScreenUpdateMsg, UpstreamMsg, V1mBuilder,
};

// ============================================================================
// Helper functions for connecting to the hardware
// ===========================================================================

fn find_v1m_ports() -> Result<
    (
        MidiInputPort,
        MidiOutputConnection,
        MidiInputPort,
        MidiOutputConnection,
    ),
    Box<dyn std::error::Error>,
> {
    let mut midi_in = MidiInput::new("midir input port sniff")?;
    midi_in.ignore(Ignore::None);
    let midi_out_1 = MidiOutput::new("midir output port sniff")?;
    let midi_out_4 = MidiOutput::new("midir output port sniff")?;

    let mut input_port_1 = None;
    let mut output_port_1 = None;
    let mut input_port_4 = None;
    let mut output_port_4 = None;

    for (i, p) in midi_in.ports().iter().enumerate() {
        if input_port_1.is_none() && midi_in.port_name(p)? == "iCON V1-M Port 1" {
            input_port_1 = Some(p.clone());
            break;
        }
    }
    for (i, p) in midi_out_1.ports().iter().enumerate() {
        if output_port_1.is_none() && midi_out_1.port_name(p)? == "iCON V1-M Port 1" {
            output_port_1 = Some(p.clone());
            break;
        }
    }
    for (i, p) in midi_in.ports().iter().enumerate() {
        if input_port_4.is_none() && midi_in.port_name(p)? == "iCON V1-M Port 4" {
            input_port_4 = Some(p.clone());
            break;
        }
    }
    for (i, p) in midi_out_1.ports().iter().enumerate() {
        if output_port_4.is_none() && midi_out_1.port_name(p)? == "iCON V1-M Port 4" {
            output_port_4 = Some(p.clone());
            break;
        }
    }

    if input_port_1.is_none() {
        return Err("Could not find X-Touch MIDI input port".into());
    }

    println!("Found input port {}", input_port_1.clone().unwrap().id());

    if let Some(output_port_1) = output_port_1 {
        if let Some(output_port_4) = output_port_4 {
            println!(
                "Connecting to output port '{}' ...",
                midi_out_1.port_name(&output_port_1)?
            );
            let output_connection_1 = midi_out_1.connect(&output_port_1, "midir-test")?;
            let output_connection_4 = midi_out_4.connect(&output_port_4, "midir-test")?;
            Ok((
                input_port_1.unwrap(),
                output_connection_1,
                input_port_4.unwrap(),
                output_connection_4,
            ))
        } else {
            Err("Could not find X-Touch MIDI output port".into())
        }
    } else {
        Err("Could not find X-Touch MIDI output port".into())
    }
}

fn find_imap_ports() -> Result<(MidiInputPort, MidiOutputConnection), Box<dyn std::error::Error>> {
    let mut midi_in = MidiInput::new("midir input port sniff")?;
    midi_in.ignore(Ignore::None);
    let midi_out = MidiOutput::new("midir output port sniff")?;

    let mut input_port = None;
    let mut output_port = None;

    for (i, p) in midi_in.ports().iter().enumerate() {
        if input_port.is_none() && midi_in.port_name(p)? == "iCON V1-M Port 4" {
            input_port = Some(p.clone());
            break;
        }
    }
    for (i, p) in midi_out.ports().iter().enumerate() {
        if output_port.is_none() && midi_out.port_name(p)? == "iCON V1-M Port 4" {
            output_port = Some(p.clone());
            break;
        }
    }

    if input_port.is_none() {
        return Err("Could not find X-Touch MIDI input port".into());
    }

    println!("Found input port {}", input_port.clone().unwrap().id());

    if let Some(output_port) = output_port {
        println!(
            "Connecting to output port '{}' ...",
            midi_out.port_name(&output_port)?
        );
        let output_connection = midi_out.connect(&output_port, "midir-test")?;
        Ok((input_port.unwrap(), output_connection))
    } else {
        Err("Could not find X-Touch MIDI output port".into())
    }
}

// Test result tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TestResult {
    Pass,
    Fail,
    Skip,
}

struct TestSummary {
    name: String,
    result: TestResult,
}

impl TestSummary {
    fn new(name: &str, result: TestResult) -> Self {
        TestSummary {
            name: name.to_string(),
            result,
        }
    }
}

// Helper to prompt user for Y/N/X input
fn prompt_user(message: &str) -> TestResult {
    print!("{} [Y/N/X to skip]: ", message);
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();

    match input.trim().to_uppercase().as_str() {
        "Y" => TestResult::Pass,
        "N" => TestResult::Fail,
        "X" => TestResult::Skip,
        _ => {
            println!("Invalid input, treating as skip");
            TestResult::Skip
        }
    }
}

// Helper to wait for user to press Enter
fn wait_for_user_action(message: &str) {
    print!("{} [Press Enter when ready]", message);
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
}

// Print test results summary
fn print_summary(summaries: &[TestSummary]) {
    println!("\n========================================");
    println!("Test Results Summary");
    println!("========================================");

    let mut passed = 0;
    let mut failed = 0;
    let mut skipped = 0;

    for summary in summaries {
        let status = match summary.result {
            TestResult::Pass => {
                passed += 1;
                "✓ PASS"
            }
            TestResult::Fail => {
                failed += 1;
                "✗ FAIL"
            }
            TestResult::Skip => {
                skipped += 1;
                "- SKIP"
            }
        };
        println!("  {} ... {}", summary.name, status);
    }

    println!(
        "\ntest result: {}. {} passed; {} failed; {} skipped",
        if failed > 0 { "FAILED" } else { "ok" },
        passed,
        failed,
        skipped
    );
    println!("========================================\n");
}

// ============================================================================
// OUTPUT TESTS - Send messages to v1m and verify hardware behavior
// ============================================================================

/// Test suite for v1m downstream (output) messages
fn run_output_tests(tx: &Sender<DownstreamMsg>) -> Vec<TestSummary> {
    println!("\n========================================");
    println!("v1m Output Tests");
    println!("========================================");
    println!("These tests send messages to v1m hardware.");
    println!("Please verify the hardware behaves as expected.\n");

    let mut results = Vec::new();

    // Test fader movement
    for channel in 0..8 {
        let test_name = format!("fader_channel_{}_to_max", channel);
        println!("\nTest: {}", test_name);

        tx.send(DownstreamMsg::FaderAbs(FaderAbsMsg {
            idx: channel,
            value: 1.0,
        }))
        .unwrap();

        let result = prompt_user(&format!(
            "Did fader {} move to maximum position (+10dB)?",
            channel
        ));
        results.push(TestSummary::new(&test_name, result));
    }

    for channel in 0..8 {
        let test_name = format!("fader_channel_{}_to_min", channel);
        println!("\nTest: {}", test_name);

        tx.send(DownstreamMsg::FaderAbs(FaderAbsMsg {
            idx: channel,
            value: 0.0,
        }))
        .unwrap();

        let result = prompt_user(&format!(
            "Did fader {} move to minimum position (-Inf)?",
            channel
        ));
        results.push(TestSummary::new(&test_name, result));
    }

    for channel in 0..8 {
        let test_name = format!("fader_channel_{}_to_unity", channel);
        println!("\nTest: {}", test_name);

        tx.send(DownstreamMsg::FaderAbs(FaderAbsMsg {
            idx: channel,
            value: 0.75, // Approximate unity gain position
        }))
        .unwrap();

        let result = prompt_user(&format!(
            "Did fader {} move to approximately unity gain (0dB)?",
            channel
        ));
        results.push(TestSummary::new(&test_name, result));
    }

    // Test mute LEDs
    for channel in 0..8 {
        let test_name = format!("mute_led_channel_{}_on", channel);
        println!("\nTest: {}", test_name);

        tx.send(DownstreamMsg::MuteLED(MuteLEDMsg {
            idx: channel,
            state: LEDState::On,
        }))
        .unwrap();

        let result = prompt_user(&format!(
            "Did the mute LED for channel {} turn ON?",
            channel
        ));
        results.push(TestSummary::new(&test_name, result));
    }

    for channel in 0..8 {
        let test_name = format!("mute_led_channel_{}_off", channel);
        println!("\nTest: {}", test_name);

        tx.send(DownstreamMsg::MuteLED(MuteLEDMsg {
            idx: channel,
            state: LEDState::Off,
        }))
        .unwrap();

        let result = prompt_user(&format!(
            "Did the mute LED for channel {} turn OFF?",
            channel
        ));
        results.push(TestSummary::new(&test_name, result));
    }

    // Test solo LEDs
    for channel in 0..8 {
        let test_name = format!("solo_led_channel_{}_on", channel);
        println!("\nTest: {}", test_name);

        tx.send(DownstreamMsg::SoloLED(SoloLEDMsg {
            idx: channel,
            state: LEDState::On,
        }))
        .unwrap();

        let result = prompt_user(&format!(
            "Did the solo LED for channel {} turn ON?",
            channel
        ));
        results.push(TestSummary::new(&test_name, result));
    }

    for channel in 0..8 {
        let test_name = format!("solo_led_channel_{}_off", channel);
        println!("\nTest: {}", test_name);

        tx.send(DownstreamMsg::SoloLED(SoloLEDMsg {
            idx: channel,
            state: LEDState::Off,
        }))
        .unwrap();

        let result = prompt_user(&format!(
            "Did the solo LED for channel {} turn OFF?",
            channel
        ));
        results.push(TestSummary::new(&test_name, result));
    }

    // Test arm LEDs
    for channel in 0..8 {
        let test_name = format!("arm_led_channel_{}_on", channel);
        println!("\nTest: {}", test_name);

        tx.send(DownstreamMsg::ArmLED(ArmLEDMsg {
            idx: channel,
            state: LEDState::On,
        }))
        .unwrap();

        let result = prompt_user(&format!(
            "Did the arm/rec LED for channel {} turn ON?",
            channel
        ));
        results.push(TestSummary::new(&test_name, result));
    }

    for channel in 0..8 {
        let test_name = format!("arm_led_channel_{}_off", channel);
        println!("\nTest: {}", test_name);

        tx.send(DownstreamMsg::ArmLED(ArmLEDMsg {
            idx: channel,
            state: LEDState::Off,
        }))
        .unwrap();

        let result = prompt_user(&format!(
            "Did the arm/rec LED for channel {} turn OFF?",
            channel
        ));
        results.push(TestSummary::new(&test_name, result));
    }

    results
}

// ============================================================================
// INPUT TESTS - Prompt user to interact with hardware and verify messages
// ============================================================================

/// Test suite for v1m upstream (input) messages
fn run_input_tests(rx: &Receiver<UpstreamMsg>) -> Vec<TestSummary> {
    println!("\n========================================");
    println!("v1m Input Tests");
    println!("========================================");
    println!("These tests require you to interact with v1m hardware.");
    println!("Please follow the prompts and perform the requested actions.\n");

    let mut results = Vec::new();

    // for channel in 0..8 {
    //     let test_name = format!("encoder_click_channel_{}", channel);
    //     println!("\nTest: {}", test_name);
    //
    //     wait_for_user_action(&format!("Click the encoder for channel {}", channel));
    //
    //     let mut received_press = false;
    //     let mut received_release = false;
    //
    //     let timeout = std::time::Instant::now();
    //     while timeout.elapsed() < Duration::from_secs(2) {
    //         if let Ok(msg) = rx.recv_timeout(Duration::from_millis(100)) {
    //             match msg {
    //                 UpstreamMsg::EncoderPress(click) if click.idx == channel => {
    //                     received_press = true;
    //                     println!("  ✓ Received EncoderPress{{idx: {}}}", click.idx);
    //                 }
    //                 UpstreamMsg::EncoderRelease(click) if click.idx == channel => {
    //                     received_release = true;
    //                     println!("  ✓ Received EncoderRelease{{idx: {}}}", click.idx);
    //                 }
    //                 _ => {}
    //             }
    //         }
    //
    //         if received_press && received_release {
    //             break;
    //         }
    //     }
    //
    //     let result = if received_press && received_release {
    //         TestResult::Pass
    //     } else {
    //         println!(
    //             "  ✗ Did not receive expected messages (press: {}, release: {})",
    //             received_press, received_release
    //         );
    //         TestResult::Fail
    //     };
    //     results.push(TestSummary::new(&test_name, result));
    // }

    for channel in 0..8 {
        let test_name = format!("encoder_turn_cw_channel_{}", channel);
        println!("\nTest: {}", test_name);

        wait_for_user_action(&format!(
            "Turn the encoder for channel {} clockwise",
            channel
        ));

        let timeout = std::time::Instant::now();
        while timeout.elapsed() < Duration::from_secs(2) {
            if let Ok(UpstreamMsg::EncoderTurnInc(msg)) =
                rx.recv_timeout(Duration::from_millis(100))
            {
                if msg.idx == channel {
                    println!("  ✓ Received EncoderTurn{{idx: {}}}", msg.idx);
                    results.push(TestSummary::new(&test_name, TestResult::Pass));
                    break;
                }
            }
        }
        println!(
            "  ✗ Did not receive EncoderTurn{{idx: {}}} message",
            channel
        );
        results.push(TestSummary::new(&test_name, TestResult::Fail));
    }

    for channel in 0..8 {
        let test_name = format!("encoder_turn_ccw_channel_{}", channel);
        println!("\nTest: {}", test_name);

        wait_for_user_action(&format!(
            "Turn the encoder for channel {} counter-clockwise",
            channel
        ));

        let timeout = std::time::Instant::now();
        while timeout.elapsed() < Duration::from_secs(2) {
            if let Ok(UpstreamMsg::EncoderTurnDec(msg)) =
                rx.recv_timeout(Duration::from_millis(100))
            {
                if msg.idx == channel {
                    println!("  ✓ Received EncoderTurn{{idx: {}}}", msg.idx);
                    results.push(TestSummary::new(&test_name, TestResult::Pass));
                    break;
                }
            }
        }
        println!(
            "  ✗ Did not receive EncoderTurn{{idx: {}}} message",
            channel
        );
        results.push(TestSummary::new(&test_name, TestResult::Fail));
    }

    // Test mute button press/release
    for channel in 0..8 {
        let test_name = format!("mute_button_channel_{}", channel);
        println!("\nTest: {}", test_name);

        wait_for_user_action(&format!("Press the MUTE button for channel {}", channel));

        // Check for MutePress message
        let mut received_press = false;
        let mut received_release = false;

        let timeout = std::time::Instant::now();
        while timeout.elapsed() < Duration::from_secs(2) {
            if let Ok(msg) = rx.recv_timeout(Duration::from_millis(100)) {
                match msg {
                    UpstreamMsg::MutePress(press) if press.idx == channel => {
                        received_press = true;
                        println!("  ✓ Received MutePress{{idx: {}}}", press.idx);
                    }
                    UpstreamMsg::MuteRelease(release) if release.idx == channel => {
                        received_release = true;
                        println!("  ✓ Received MuteRelease{{idx: {}}}", release.idx);
                    }
                    _ => {}
                }
            }

            if received_press && received_release {
                break;
            }
        }

        let result = if received_press && received_release {
            TestResult::Pass
        } else {
            println!(
                "  ✗ Did not receive expected messages (press: {}, release: {})",
                received_press, received_release
            );
            TestResult::Fail
        };

        results.push(TestSummary::new(&test_name, result));
    }

    // Test solo button press/release
    for channel in 0..8 {
        let test_name = format!("solo_button_channel_{}", channel);
        println!("\nTest: {}", test_name);

        wait_for_user_action(&format!("Press the SOLO button for channel {}", channel));

        let mut received_press = false;
        let mut received_release = false;

        let timeout = std::time::Instant::now();
        while timeout.elapsed() < Duration::from_secs(2) {
            if let Ok(msg) = rx.recv_timeout(Duration::from_millis(100)) {
                match msg {
                    UpstreamMsg::SoloPress(press) if press.idx == channel => {
                        received_press = true;
                        println!("  ✓ Received SoloPress{{idx: {}}}", press.idx);
                    }
                    UpstreamMsg::SoloRelease(release) if release.idx == channel => {
                        received_release = true;
                        println!("  ✓ Received SoloRelease{{idx: {}}}", release.idx);
                    }
                    _ => {}
                }
            }

            if received_press && received_release {
                break;
            }
        }

        let result = if received_press && received_release {
            TestResult::Pass
        } else {
            println!(
                "  ✗ Did not receive expected messages (press: {}, release: {})",
                received_press, received_release
            );
            TestResult::Fail
        };

        results.push(TestSummary::new(&test_name, result));
    }

    // Test arm button press/release
    for channel in 0..8 {
        let test_name = format!("arm_button_channel_{}", channel);
        println!("\nTest: {}", test_name);

        wait_for_user_action(&format!("Press the ARM/REC button for channel {}", channel));

        let mut received_press = false;
        let mut received_release = false;

        let timeout = std::time::Instant::now();
        while timeout.elapsed() < Duration::from_secs(2) {
            if let Ok(msg) = rx.recv_timeout(Duration::from_millis(100)) {
                match msg {
                    UpstreamMsg::ArmPress(press) if press.idx == channel => {
                        received_press = true;
                        println!("  ✓ Received ArmPress{{idx: {}}}", press.idx);
                    }
                    UpstreamMsg::ArmRelease(release) if release.idx == channel => {
                        received_release = true;
                        println!("  ✓ Received ArmRelease{{idx: {}}}", release.idx);
                    }
                    _ => {}
                }
            }

            if received_press && received_release {
                break;
            }
        }

        let result = if received_press && received_release {
            TestResult::Pass
        } else {
            println!(
                "  ✗ Did not receive expected messages (press: {}, release: {})",
                received_press, received_release
            );
            TestResult::Fail
        };

        results.push(TestSummary::new(&test_name, result));
    }

    // Test select button press/release
    for channel in 0..8 {
        let test_name = format!("select_button_channel_{}", channel);
        println!("\nTest: {}", test_name);

        wait_for_user_action(&format!("Press the SELECT button for channel {}", channel));

        let mut received_press = false;
        let mut received_release = false;

        let timeout = std::time::Instant::now();
        while timeout.elapsed() < Duration::from_secs(2) {
            if let Ok(msg) = rx.recv_timeout(Duration::from_millis(100)) {
                match msg {
                    UpstreamMsg::SelectPress(press) if press.idx == channel => {
                        received_press = true;
                        println!("  ✓ Received SelectPress{{idx: {}}}", press.idx);
                    }
                    UpstreamMsg::SelectRelease(release) if release.idx == channel => {
                        received_release = true;
                        println!("  ✓ Received SelectRelease{{idx: {}}}", release.idx);
                    }
                    _ => {}
                }
            }

            if received_press && received_release {
                break;
            }
        }

        let result = if received_press && received_release {
            TestResult::Pass
        } else {
            println!(
                "  ✗ Did not receive expected messages (press: {}, release: {})",
                received_press, received_release
            );
            TestResult::Fail
        };

        results.push(TestSummary::new(&test_name, result));
    }

    // Test fader movement
    for channel in 0..2 {
        // Just test first 2 channels to keep it reasonable
        let test_name = format!("fader_movement_channel_{}", channel);
        println!("\nTest: {}", test_name);

        wait_for_user_action(&format!("Move fader {} from bottom to top", channel));

        let mut received_message = false;

        let timeout = std::time::Instant::now();
        while timeout.elapsed() < Duration::from_secs(3) {
            if let Ok(msg) = rx.recv_timeout(Duration::from_millis(100)) {
                if let UpstreamMsg::FaderAbs(fader) = msg {
                    if fader.idx == channel {
                        received_message = true;
                        println!(
                            "  ✓ Received FaderAbs{{idx: {}, value: {:.3}}}",
                            fader.idx, fader.value
                        );
                    }
                }
            }
        }

        let result = if received_message {
            TestResult::Pass
        } else {
            println!("  ✗ Did not receive FaderAbs message");
            TestResult::Fail
        };

        results.push(TestSummary::new(&test_name, result));
    }

    // Test view buttons
    let test_name = "global_view_button";
    println!("\nTest: {}", test_name);
    wait_for_user_action("Press the 'Global View' button");

    let mut received_press = false;
    let mut received_release = false;

    let timeout = std::time::Instant::now();
    while timeout.elapsed() < Duration::from_secs(2) {
        if let Ok(msg) = rx.recv_timeout(Duration::from_millis(100)) {
            match msg {
                UpstreamMsg::GlobalPress => {
                    received_press = true;
                    println!("  ✓ Received GlobalPress");
                }
                UpstreamMsg::GlobalRelease => {
                    received_release = true;
                    println!("  ✓ Received GlobalRelease");
                }
                _ => {}
            }
        }

        if received_press && received_release {
            break;
        }
    }

    let result = if received_press && received_release {
        TestResult::Pass
    } else {
        println!(
            "  ✗ Did not receive expected messages (press: {}, release: {})",
            received_press, received_release
        );
        TestResult::Fail
    };

    results.push(TestSummary::new(test_name, result));

    let test_name = "midi_tracks_button";
    println!("\nTest: {}", test_name);
    wait_for_user_action("Press the 'MIDI Tracks' button");

    let mut received_press = false;
    let mut received_release = false;

    let timeout = std::time::Instant::now();
    while timeout.elapsed() < Duration::from_secs(2) {
        if let Ok(msg) = rx.recv_timeout(Duration::from_millis(100)) {
            match msg {
                UpstreamMsg::MIDITracksPress => {
                    received_press = true;
                    println!("  ✓ Received MIDITracksPress");
                }
                UpstreamMsg::MIDITracksRelease => {
                    received_release = true;
                    println!("  ✓ Received MIDITracksRelease");
                }
                _ => {}
            }
        }

        if received_press && received_release {
            break;
        }
    }

    let result = if received_press && received_release {
        TestResult::Pass
    } else {
        println!(
            "  ✗ Did not receive expected messages (press: {}, release: {})",
            received_press, received_release
        );
        TestResult::Fail
    };

    results.push(TestSummary::new(test_name, result));

    results
}

// ============================================================================
// Meters tests
// ============================================================================
fn run_meters_tests(tx: &Sender<DownstreamMsg>) -> Vec<TestSummary> {
    println!("\n========================================");
    println!("v1m Meters Tests");
    println!("========================================");
    println!("These tests send messages to v1m hardware.");
    println!("Please verify the meters display the expected levels.\n");

    let mut results = Vec::new();

    // Test meter levels for each channel
    for idx in 0..8 {
        let test_name = format!("meter_channel_{}_levels", idx);
        println!("\nTest: {}", test_name);

        // Send a series of meter levels from -60dB to 10dB
        let levels_db: [i32; 14] = [
            -80, -60, -50, -40, -30, -20, -14, -10, -8, -6, -4, -2, 0, 10,
        ];
        for db in levels_db {
            tx.send(DownstreamMsg::Meter(MeterMsg { idx, db: db as f64 }))
                .unwrap();
            std::thread::sleep(std::time::Duration::from_millis(100));
            let result = prompt_user(&format!(
                "Did the meter for channel {} display {}db?",
                idx, db
            ));
            results.push(TestSummary::new(&test_name, result));
        }
    }

    results
}

#[test]
#[ignore] // Must be run manually with --ignored flag
fn v1m_meters_tests() {
    println!("\n");
    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║          v1m Meters Test Suite                                ║");
    println!("╚════════════════════════════════════════════════════════════════╝");
    println!("\nNOTE: This test requires v1m hardware to be connected.");
    println!("Messages will be sent to the hardware for manual verification.\n");

    print!("Is v1m hardware connected and ready? [Y/N]: ");
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();

    if !input.trim().eq_ignore_ascii_case("y") {
        println!("Test aborted. Please connect v1m hardware and try again.");
        return;
    }

    let (input_port_1, output_connection_1, input_port_4, output_connection_4) =
        match find_v1m_ports() {
            Ok(ports) => ports,
            Err(err) => {
                println!("Error finding v1m MIDI ports: {}", err);
                println!("Test aborted. Please ensure v1m is connected and try again.");
                return;
            }
        };
    let (upstream_tx, upstream_rx) = bounded::<UpstreamMsg>(128);
    let (downstream_tx, downstream_rx) = bounded::<DownstreamMsg>(128);
    V1mBuilder::new(
        input_port_1,        // Use default MIDI input port
        output_connection_1, // Use default MIDI output port
        input_port_4,
        output_connection_4,
        8,
    )
    .build(downstream_rx, upstream_tx);

    let results = run_meters_tests(&downstream_tx);
}

// ============================================================================
// Scribble strip tests
// ============================================================================
//
fn run_scribble_strip_tests(tx: &Sender<DownstreamMsg>) -> Vec<TestSummary> {
    println!("\n========================================");
    println!("v1m Scribble Strip Tests");
    println!("========================================");
    println!("These tests send messages to v1m hardware.");
    println!("Please verify the scribble strip displays the expected text.\n");

    let mut results = Vec::new();

    // Case 1: Line 1 text "FaderN"
    for channel in 0..8 {
        let test_name = format!("scribble_channel_{}_line1", channel);
        println!("\nTest: {}", test_name);
        tx.send(DownstreamMsg::ScribbleStripLine1Text(
            ScribbleStripLine1TextMsg {
                idx: channel,
                text: format!("Fader{}", channel),
            },
        ))
        .unwrap();

        let result = prompt_user(&format!(
            "Did channel {} scribble strip line 1 display \"Fader{}\"?",
            channel, channel
        ));
        results.push(TestSummary::new(&test_name, result));
    }

    // Case 2: Line 2 text "Displ1"
    for channel in 0..8 {
        let test_name = format!("scribble_channel_{}_line2", channel);
        println!("\nTest: {}", test_name);
        tx.send(DownstreamMsg::ScribbleStripLine2Text(
            ScribbleStripLine2TextMsg {
                idx: channel,
                text: format!("Displ{}", channel),
            },
        ))
        .unwrap();

        let result = prompt_user(&format!(
            "Did channel {} scribble strip line 2 display \"Displ{}\"?",
            channel, channel
        ));
        results.push(TestSummary::new(&test_name, result));
    }

    // Case 3: Light/Light mode with distinct background colors (wrapping)
    let colors = [
        ("red", Color { r: 127, g: 0, b: 0 }),
        ("green", Color { r: 0, g: 127, b: 0 }),
        ("blue", Color { r: 0, g: 0, b: 127 }),
        (
            "yellow",
            Color {
                r: 127,
                g: 127,
                b: 0,
            },
        ),
        (
            "cyan",
            Color {
                r: 0,
                g: 127,
                b: 127,
            },
        ),
        (
            "magenta",
            Color {
                r: 127,
                g: 0,
                b: 127,
            },
        ),
        (
            "white",
            Color {
                r: 127,
                g: 127,
                b: 127,
            },
        ),
        (
            "orange",
            Color {
                r: 127,
                g: 63,
                b: 0,
            },
        ),
    ];
    for channel in 0..8 {
        let test_name = format!("scribble_channel_{}_light_light_color", channel);
        println!("\nTest: {}", test_name);

        tx.send(DownstreamMsg::ScribbleStripBackgroundColor(
            ScribbleStripBackgroundColorMsg {
                idx: channel,
                color: colors[channel as usize % colors.len()].1,
            },
        ))
        .unwrap();

        let expected_color = colors[channel as usize % colors.len()].0;
        let result = prompt_user(&format!(
            "Did channel {} display in Light/Light mode with background color {:?}?",
            channel, expected_color
        ));
        results.push(TestSummary::new(&test_name, result));
    }

    // Case 4: Bottom line 1 text "BottomN"
    for channel in 0..8 {
        let test_name = format!("scribble_channel_{}_line1", channel);
        println!("\nTest: {}", test_name);
        tx.send(DownstreamMsg::BottomScribbleStripLine1Text(
            BottomScribbleStripLine1TextMsg {
                idx: channel,
                text: format!("Bottom{}", channel),
            },
        ))
        .unwrap();

        let result = prompt_user(&format!(
            "Did channel {} lower scribble strip line 1 display \"Bottom{}\"?",
            channel, channel
        ));
        results.push(TestSummary::new(&test_name, result));
    }

    // Case 5: Bottom line2 text "BelowN"
    for channel in 0..8 {
        let test_name = format!("scribble_channel_{}_line2", channel);
        println!("\nTest: {}", test_name);
        tx.send(DownstreamMsg::BottomScribbleStripLine2Text(
            BottomScribbleStripLine2TextMsg {
                idx: channel,
                text: format!("Below{}", channel),
            },
        ))
        .unwrap();

        let result = prompt_user(&format!(
            "Did channel {} lower scribble strip line 2 display \"Below{}\"?",
            channel, channel
        ));
        results.push(TestSummary::new(&test_name, result));
    }

    results
}

#[test]
#[ignore] // Must be run manually with --ignored flag
fn v1m_scribble_strip_tests() {
    println!("\n");
    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║          v1m Scribble Strip Test Suite                      ║");
    println!("╚════════════════════════════════════════════════════════════════╝");
    println!("\nNOTE: This test requires v1m hardware to be connected.");
    println!("Messages will be sent to the hardware for manual verification.\n");

    print!("Is v1m hardware connected and ready? [Y/N]: ");
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();

    if !input.trim().eq_ignore_ascii_case("y") {
        println!("Test aborted. Please connect v1m hardware and try again.");
        return;
    }

    let (input_port_1, output_connection_1, input_port_4, output_connection_4) =
        match find_v1m_ports() {
            Ok(ports) => ports,
            Err(err) => {
                println!("Error finding v1m MIDI ports: {}", err);
                println!("Test aborted. Please ensure v1m is connected and try again.");
                return;
            }
        };
    let (upstream_tx, upstream_rx) = bounded::<UpstreamMsg>(128);
    let (downstream_tx, downstream_rx) = bounded::<DownstreamMsg>(128);
    V1mBuilder::new(
        input_port_1,        // Use default MIDI input port
        output_connection_1, // Use default MIDI output port
        input_port_4,
        output_connection_4,
        8,
    )
    .build(downstream_rx, upstream_tx);

    let results = run_scribble_strip_tests(&downstream_tx);
}

#[test]
#[ignore] // Must be run manually with --ignored flag
fn v1m_touchscreen_tests() {
    println!("\n");
    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║          v1m Touchscreen Test Suite                      ║");
    println!("╚════════════════════════════════════════════════════════════════╝");
    println!("\nNOTE: This test requires v1m hardware to be connected.");
    println!("Messages wisll be sent to the hardware for manual verification.\n");

    print!("Is v1m hardware connected and ready? [Y/N]: ");
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();

    if !input.trim().eq_ignore_ascii_case("y") {
        println!("Test aborted. Please connect v1m hardware and try again.");
        return;
    }

    let (input_port_1, output_connection_1, input_port_4, output_connection_4) =
        match find_v1m_ports() {
            Ok(ports) => ports,
            Err(err) => {
                println!("Error finding v1m MIDI ports: {}", err);
                println!("Test aborted. Please ensure v1m is connected and try again.");
                return;
            }
        };
    let (upstream_tx, upstream_rx) = bounded::<UpstreamMsg>(128);
    let (downstream_tx, downstream_rx) = bounded::<DownstreamMsg>(128);
    V1mBuilder::new(
        input_port_1,
        output_connection_1,
        input_port_4,
        output_connection_4,
        8,
    )
    .build(downstream_rx, upstream_tx);

    downstream_tx.send(DownstreamMsg::TouchScreenUpdate(TouchScreenUpdateMsg {
        slot: Slot::DAW1,
        daw_id: DawId::Reaper,
        column: 0x0,
        row: 0x0,
        layer: TouchScreenLayer::Blue,
        text: "foo",
    }));
}

// ============================================================================
// Encoder tests
// ============================================================================
//
fn run_encoder_tests(tx: &Sender<DownstreamMsg>) -> Vec<TestSummary> {
    println!("\n========================================");
    println!("v1m Encoder Output Tests");
    println!("========================================");
    println!("These tests send messages to v1m hardware.");
    println!("Please verify the encoders show the expected behavior.\n");

    let mut results = Vec::new();

    // Case 1: illuminate all segments
    for channel in 0..8 {
        let test_name = format!("encoder_channel_{}_all_segments", channel);
        println!("\nTest: {}", test_name);
        tx.send(DownstreamMsg::EncoderRingLED(EncoderRingMsg {
            idx: channel,
            mode: EncoderRingMode::FromLeft,
            val: 0xb,
        }))
        .unwrap();

        let result = prompt_user(&format!(
            "Did channel {} enoder display all segments illuminated?",
            channel
        ));
        results.push(TestSummary::new(&test_name, result));
    }

    results
}

#[test]
#[ignore] // Must be run manually with --ignored flag
fn v1m_encoder_tests() {
    println!("\n");
    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║          v1m Encoder Output Test Suite                      ║");
    println!("╚════════════════════════════════════════════════════════════════╝");
    println!("\nNOTE: This test requires v1m hardware to be connected.");
    println!("Messages will be sent to the hardware for manual verification.\n");

    print!("Is v1m hardware connected and ready? [Y/N]: ");
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();

    if !input.trim().eq_ignore_ascii_case("y") {
        println!("Test aborted. Please connect v1m hardware and try again.");
        return;
    }

    let (input_port_1, output_connection_1, input_port_4, output_connection_4) =
        match find_v1m_ports() {
            Ok(ports) => ports,
            Err(err) => {
                println!("Error finding v1m MIDI ports: {}", err);
                println!("Test aborted. Please ensure v1m is connected and try again.");
                return;
            }
        };
    let (upstream_tx, upstream_rx) = bounded::<UpstreamMsg>(128);
    let (downstream_tx, downstream_rx) = bounded::<DownstreamMsg>(128);
    V1mBuilder::new(
        input_port_1,        // Use default MIDI input port
        output_connection_1, // Use default MIDI output port
        input_port_4,
        output_connection_4,
        8,
    )
    .build(downstream_rx, upstream_tx);

    let results = run_encoder_tests(&downstream_tx);
}

// ============================================================================
// TEST ENTRY POINTS
// ============================================================================

#[test]
#[ignore] // Must be run manually with --ignored flag
fn v1m_manual_output_tests() {
    println!("\n");
    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║          v1m Manual Output Test Suite                      ║");
    println!("╚════════════════════════════════════════════════════════════════╝");
    println!("\nNOTE: This test requires v1m hardware to be connected.");
    println!("Messages will be sent to the hardware for manual verification.\n");

    print!("Is v1m hardware connected and ready? [Y/N]: ");
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();

    if !input.trim().eq_ignore_ascii_case("y") {
        println!("Test aborted. Please connect v1m hardware and try again.");
        return;
    }

    let (input_port_1, output_connection_1, input_port_4, output_connection_4) =
        match find_v1m_ports() {
            Ok(ports) => ports,
            Err(err) => {
                println!("Error finding v1m MIDI ports: {}", err);
                println!("Test aborted. Please ensure v1m is connected and try again.");
                return;
            }
        };

    let (upstream_tx, upstream_rx) = bounded::<UpstreamMsg>(128);
    let (downstream_tx, downstream_rx) = bounded::<DownstreamMsg>(128);
    V1mBuilder::new(
        input_port_1,        // Use default MIDI input port
        output_connection_1, // Use default MIDI output port
        input_port_4,
        output_connection_4,
        8,
    )
    .build(downstream_rx, upstream_tx);

    let results = run_output_tests(&downstream_tx);
    print_summary(&results);
}

#[test]
#[ignore] // Must be run manually with --ignored flag
fn v1m_manual_input_tests() {
    println!("\n");
    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║          v1m Manual Input Test Suite                       ║");
    println!("╚════════════════════════════════════════════════════════════════╝");
    println!("\nNOTE: This test requires v1m hardware to be connected.");
    println!("You will be prompted to interact with the hardware.\n");

    // print!("Is v1m hardware connected and ready? [Y/N]: ");
    // io::stdout().flush().unwrap();
    //
    // let mut input = String::new();
    // io::stdin().read_line(&mut input).unwrap();

    // if !input.trim().eq_ignore_ascii_case("y") {
    //     println!("Test aborted. Please connect v1m hardware and try again.");
    //     return;
    // }

    let (input_port_1, output_connection_1, input_port_4, output_connection_4) =
        match find_v1m_ports() {
            Ok(ports) => ports,
            Err(err) => {
                println!("Error finding v1m MIDI ports: {}", err);
                println!("Test aborted. Please ensure v1m is connected and try again.");
                return;
            }
        };

    let (upstream_tx, upstream_rx) = bounded::<UpstreamMsg>(128);
    let (downstream_tx, downstream_rx) = bounded::<DownstreamMsg>(128);
    V1mBuilder::new(
        input_port_1,        // Use default MIDI input port
        output_connection_1, // Use default MIDI output port
        input_port_4,
        output_connection_4,
        8,
    )
    .build(downstream_rx, upstream_tx);

    let results = run_input_tests(&upstream_rx);
    print_summary(&results);
}

// ============================================================================
// EXAMPLE: How to add new test cases
// ============================================================================

/*
To add new output test cases, add them to the run_output_tests function:

    let test_name = "my_new_output_test";
    println!("\nTest: {}", test_name);

    tx.send(DownstreamMsg::SomeMessage(...)).unwrap();

    let result = prompt_user("Did the expected thing happen?");
    results.push(TestSummary::new(test_name, result));

To add new input test cases, add them to the run_input_tests function:

    let test_name = "my_new_input_test";
    println!("\nTest: {}", test_name);

    wait_for_user_action("Do something with the hardware");

    let mut received = false;
    let timeout = std::time::Instant::now();
    while timeout.elapsed() < Duration::from_secs(2) {
        if let Ok(msg) = rx.recv_timeout(Duration::from_millis(100)) {
            // Check for expected message
            if matches!(msg, UpstreamMsg::SomeMessage(_)) {
                received = true;
                println!("  ✓ Received expected message");
                break;
            }
        }
    }

    let result = if received {
        TestResult::Pass
    } else {
        TestResult::Fail
    };

    results.push(TestSummary::new(test_name, result));
*/
