# XTouch Manual Test Suite

This directory contains semi-manual test suites for testing XTouch hardware integration.

## Overview

The manual test suite provides two types of tests:

1. **Output Tests** (`xtouch_manual_output_tests`): Send messages to XTouch hardware and prompt the user to verify the hardware behaves correctly
2. **Input Tests** (`xtouch_manual_input_tests`): Prompt the user to interact with the hardware and verify that the correct messages are received

## Running the Tests

### Prerequisites

- XTouch hardware must be connected to your computer
- The hardware must be powered on and ready

### Running Output Tests

Output tests send messages to the XTouch and ask you to verify the hardware behavior:

```bash
cargo test --test xtouch_manual_tests xtouch_manual_output_tests -- --ignored --nocapture --test-threads=1
```

Example interaction:
```
Test: fader_channel_3_to_min
Did fader 3 move to minimum position (-Inf)? [Y/N/X to skip]: Y
```

### Running Input Tests

Input tests prompt you to interact with the hardware and verify the messages received:

```bash
cargo test --test xtouch_manual_tests xtouch_manual_input_tests -- --ignored --nocapture --test-threads=1
```

Example interaction:
```
Test: mute_button_channel_7
Press the MUTE button for channel 7 [Press Enter when ready]
  ✓ Received MutePress{idx: 7, velocity: 127}
  ✓ Received MuteRelease{idx: 7}
```

### Running All Manual Tests

To run all manual tests at once:

```bash
cargo test --test xtouch_manual_tests -- --ignored --nocapture --test-threads=1
```

**Important**: Always use `--test-threads=1` to ensure tests run sequentially and don't interfere with each other.

## Test Results

Test results are displayed in a format similar to `cargo test`:

```
========================================
Test Results Summary
========================================
  fader_channel_0_to_min ... ✓ PASS
  fader_channel_0_to_max ... ✓ PASS
  fader_channel_1_to_min ... - SKIP
  mute_led_channel_0_on ... ✗ FAIL
  ...

test result: FAILED. 42 passed; 1 failed; 1 skipped
========================================
```

## Response Options

When prompted for verification:
- `Y` or `y`: Test passed - hardware behaved as expected
- `N` or `n`: Test failed - hardware did not behave as expected
- `X` or `x`: Skip this test

## Adding New Test Cases

### Adding Output Tests

To add a new output test, edit `tests/xtouch_manual_tests.rs` and add to the `run_output_tests` function:

```rust
let test_name = "my_custom_output_test";
println!("\nTest: {}", test_name);

// Send message to hardware
tx.send(XTouchDownstreamMsg::SomeMessage(...)).unwrap();

// Prompt user for verification
let result = prompt_user("Did the expected behavior occur?");
results.push(TestSummary::new(test_name, result));
```

### Adding Input Tests

To add a new input test, edit `tests/xtouch_manual_tests.rs` and add to the `run_input_tests` function:

```rust
let test_name = "my_custom_input_test";
println!("\nTest: {}", test_name);

// Prompt user to interact with hardware
wait_for_user_action("Perform some action on the hardware");

// Wait for and verify message
let mut received = false;
let timeout = std::time::Instant::now();
while timeout.elapsed() < Duration::from_secs(2) {
    if let Ok(msg) = rx.recv_timeout(Duration::from_millis(100)) {
        if matches!(msg, XTouchUpstreamMsg::ExpectedMessage(_)) {
            received = true;
            println!("  ✓ Received expected message");
            break;
        }
    }
}

let result = if received {
    TestResult::Pass
} else {
    println!("  ✗ Did not receive expected message");
    TestResult::Fail
};

results.push(TestSummary::new(test_name, result));
```

## Current Test Coverage

### Output Tests (Downstream Messages)
- Fader movements (min, max, unity gain) for all 8 channels
- Mute LED (on/off) for all 8 channels
- Solo LED (on/off) for all 8 channels
- Arm/Rec LED (on/off) for all 8 channels
- View button LEDs (Global, MIDI Tracks)

### Input Tests (Upstream Messages)
- Mute button press/release for all 8 channels
- Solo button press/release for all 8 channels
- Arm/Rec button press/release for all 8 channels
- Fader movements (channels 0-1)
- View buttons (Global, MIDI Tracks)

## Architecture Notes

### Current Implementation

The current implementation uses mock channels that are **not** connected to actual XTouch hardware. This serves as a framework/template for the test structure.

### Production Implementation

To connect to real XTouch hardware, you would need to:

1. Initialize the XTouch MIDI device
2. Connect the test channels to the actual hardware channels
3. Ensure proper MIDI message routing

Example integration:
```rust
// Initialize XTouch device
let xtouch_device = XTouch::new("XTouch MIDI Device Name")?;

// Get actual channels from device
let (tx, rx) = xtouch_device.get_channels();

// Run tests with real hardware
let results = run_output_tests(&tx);
```

## Troubleshooting

### Hardware Not Responding

- Verify XTouch is powered on
- Check USB/MIDI connection
- Ensure correct MIDI device is selected
- Check MIDI driver installation

### Tests Timeout

- Input tests have a 2-3 second timeout for user actions
- If tests timeout, the action wasn't detected - check hardware connection

### Incorrect Test Results

- Make sure you're interacting with the correct channel/button
- Verify hardware is in the expected initial state
- Check for stuck buttons or faders

## Future Enhancements

Potential improvements to the test suite:
- Add tests for encoders and encoder assignment buttons
- Add tests for transport controls
- Add tests for flashing LED states
- Add tests for fader touch sensitivity
- Add batch test modes for faster testing
- Add automated initialization/reset sequences
