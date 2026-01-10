// Semi-manual test suite for XTouch hardware
//
// This test suite provides two types of tests:
// 1. Output tests: Send messages to XTouch and prompt user to verify hardware behavior
// 2. Input tests: Prompt user to interact with hardware and verify messages received
//
// Run with: cargo test --test xtouch_manual_tests -- --nocapture --test-threads=1

use arpad_rust::midi::xtouch::{
    ArmLEDMsg, ArmPress, ArmRelease, FaderAbsMsg, LEDState, MuteLEDMsg, MutePress, MuteRelease,
    SoloLEDMsg, SoloPress, SoloRelease, XTouchDownstreamMsg, XTouchUpstreamMsg,
};
use crossbeam_channel::{Receiver, Sender, bounded};
use std::io::{self, Write};
use std::time::Duration;

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
// OUTPUT TESTS - Send messages to XTouch and verify hardware behavior
// ============================================================================

/// Test suite for XTouch downstream (output) messages
fn run_output_tests(tx: &Sender<XTouchDownstreamMsg>) -> Vec<TestSummary> {
    println!("\n========================================");
    println!("XTouch Output Tests");
    println!("========================================");
    println!("These tests send messages to XTouch hardware.");
    println!("Please verify the hardware behaves as expected.\n");

    let mut results = Vec::new();

    // Test fader movement
    for channel in 0..8 {
        let test_name = format!("fader_channel_{}_to_min", channel);
        println!("\nTest: {}", test_name);

        tx.send(XTouchDownstreamMsg::FaderAbs(FaderAbsMsg {
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
        let test_name = format!("fader_channel_{}_to_max", channel);
        println!("\nTest: {}", test_name);

        tx.send(XTouchDownstreamMsg::FaderAbs(FaderAbsMsg {
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
        let test_name = format!("fader_channel_{}_to_unity", channel);
        println!("\nTest: {}", test_name);

        tx.send(XTouchDownstreamMsg::FaderAbs(FaderAbsMsg {
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

        tx.send(XTouchDownstreamMsg::MuteLED(MuteLEDMsg {
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

        tx.send(XTouchDownstreamMsg::MuteLED(MuteLEDMsg {
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

        tx.send(XTouchDownstreamMsg::SoloLED(SoloLEDMsg {
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

        tx.send(XTouchDownstreamMsg::SoloLED(SoloLEDMsg {
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

        tx.send(XTouchDownstreamMsg::ArmLED(ArmLEDMsg {
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

        tx.send(XTouchDownstreamMsg::ArmLED(ArmLEDMsg {
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

    // Test view button LEDs
    let test_name = "global_view_led_on";
    println!("\nTest: {}", test_name);
    tx.send(XTouchDownstreamMsg::Global(LEDState::On)).unwrap();
    let result = prompt_user("Did the 'Global View' LED turn ON?");
    results.push(TestSummary::new(test_name, result));

    let test_name = "global_view_led_off";
    println!("\nTest: {}", test_name);
    tx.send(XTouchDownstreamMsg::Global(LEDState::Off)).unwrap();
    let result = prompt_user("Did the 'Global View' LED turn OFF?");
    results.push(TestSummary::new(test_name, result));

    let test_name = "midi_tracks_led_on";
    println!("\nTest: {}", test_name);
    tx.send(XTouchDownstreamMsg::MIDITracks(LEDState::On))
        .unwrap();
    let result = prompt_user("Did the 'MIDI Tracks' LED turn ON?");
    results.push(TestSummary::new(test_name, result));

    let test_name = "midi_tracks_led_off";
    println!("\nTest: {}", test_name);
    tx.send(XTouchDownstreamMsg::MIDITracks(LEDState::Off))
        .unwrap();
    let result = prompt_user("Did the 'MIDI Tracks' LED turn OFF?");
    results.push(TestSummary::new(test_name, result));

    results
}

// ============================================================================
// INPUT TESTS - Prompt user to interact with hardware and verify messages
// ============================================================================

/// Test suite for XTouch upstream (input) messages
fn run_input_tests(rx: &Receiver<XTouchUpstreamMsg>) -> Vec<TestSummary> {
    println!("\n========================================");
    println!("XTouch Input Tests");
    println!("========================================");
    println!("These tests require you to interact with XTouch hardware.");
    println!("Please follow the prompts and perform the requested actions.\n");

    let mut results = Vec::new();

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
                    XTouchUpstreamMsg::MutePress(press) if press.idx == channel => {
                        received_press = true;
                        println!("  ✓ Received MutePress{{idx: {}}}", press.idx);
                    }
                    XTouchUpstreamMsg::MuteRelease(release) if release.idx == channel => {
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
                    XTouchUpstreamMsg::SoloPress(press) if press.idx == channel => {
                        received_press = true;
                        println!("  ✓ Received SoloPress{{idx: {}}}", press.idx);
                    }
                    XTouchUpstreamMsg::SoloRelease(release) if release.idx == channel => {
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
                    XTouchUpstreamMsg::ArmPress(press) if press.idx == channel => {
                        received_press = true;
                        println!("  ✓ Received ArmPress{{idx: {}}}", press.idx);
                    }
                    XTouchUpstreamMsg::ArmRelease(release) if release.idx == channel => {
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
                if let XTouchUpstreamMsg::FaderAbs(fader) = msg {
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
                XTouchUpstreamMsg::GlobalPress => {
                    received_press = true;
                    println!("  ✓ Received GlobalPress");
                }
                XTouchUpstreamMsg::GlobalRelease => {
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
                XTouchUpstreamMsg::MIDITracksPress => {
                    received_press = true;
                    println!("  ✓ Received MIDITracksPress");
                }
                XTouchUpstreamMsg::MIDITracksRelease => {
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
// TEST ENTRY POINTS
// ============================================================================

#[test]
#[ignore] // Must be run manually with --ignored flag
fn xtouch_manual_output_tests() {
    println!("\n");
    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║          XTouch Manual Output Test Suite                      ║");
    println!("╚════════════════════════════════════════════════════════════════╝");
    println!("\nNOTE: This test requires XTouch hardware to be connected.");
    println!("Messages will be sent to the hardware for manual verification.\n");

    print!("Is XTouch hardware connected and ready? [Y/N]: ");
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();

    if !input.trim().eq_ignore_ascii_case("y") {
        println!("Test aborted. Please connect XTouch hardware and try again.");
        return;
    }

    // Create channels for testing
    // In a real implementation, these would be connected to actual XTouch hardware
    let (tx, _rx) = bounded::<XTouchDownstreamMsg>(128);

    println!("\nWARNING: This is a mock test - channels are not connected to real hardware.");
    println!("In production, these channels would be connected to XTouch MIDI device.\n");

    let results = run_output_tests(&tx);
    print_summary(&results);
}

#[test]
#[ignore] // Must be run manually with --ignored flag
fn xtouch_manual_input_tests() {
    println!("\n");
    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║          XTouch Manual Input Test Suite                       ║");
    println!("╚════════════════════════════════════════════════════════════════╝");
    println!("\nNOTE: This test requires XTouch hardware to be connected.");
    println!("You will be prompted to interact with the hardware.\n");

    print!("Is XTouch hardware connected and ready? [Y/N]: ");
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();

    if !input.trim().eq_ignore_ascii_case("y") {
        println!("Test aborted. Please connect XTouch hardware and try again.");
        return;
    }

    // Create channels for testing
    // In a real implementation, these would be connected to actual XTouch hardware
    let (_tx, rx) = bounded::<XTouchUpstreamMsg>(128);

    println!("\nWARNING: This is a mock test - channels are not connected to real hardware.");
    println!("In production, these channels would be connected to XTouch MIDI device.\n");

    let results = run_input_tests(&rx);
    print_summary(&results);
}

// ============================================================================
// EXAMPLE: How to add new test cases
// ============================================================================

/*
To add new output test cases, add them to the run_output_tests function:

    let test_name = "my_new_output_test";
    println!("\nTest: {}", test_name);

    tx.send(XTouchDownstreamMsg::SomeMessage(...)).unwrap();

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
            if matches!(msg, XTouchUpstreamMsg::SomeMessage(_)) {
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
