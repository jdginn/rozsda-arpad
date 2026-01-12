// Integration tests for VolumePanMode
//
// These tests verify the behavior of the VolumePanMode, which manages the mapping
// between Reaper tracks and XTouch controller hardware (faders, buttons, LEDs).
//
// This comprehensive test suite implements all 18 test cases from the test plan,
// covering mapping, state accumulation, message flow, mode transitions, ordering,
// and threshold testing.
use std::time::Duration;

use assert2::{assert, check};
use crossbeam_channel::{Receiver, Sender, unbounded};
use float_cmp::approx_eq;

use arpad_rust::midi::xtouch::{
    ArmPress, EncoderTurnCW, FaderAbsMsg, LEDState, MutePress, SoloPress, XTouchDownstreamMsg,
    XTouchUpstreamMsg,
};
use arpad_rust::modes::mode_manager::{Barrier, Mode, ModeHandler, ModeState, State};
use arpad_rust::modes::reaper_vol_pan::{FADER_0DB, VolumePanMode};
use arpad_rust::track::track::{DataPayload, Direction, TrackDataMsg, TrackMsg};

// EPSILON constant for floating-point threshold testing
const EPSILON: f32 = 0.01;

/// Helper to create a VolumePanMode instance for testing
fn setup_vol_pan_mode() -> (
    VolumePanMode,
    Sender<TrackMsg>,
    Receiver<TrackMsg>,
    Sender<XTouchUpstreamMsg>,
    Receiver<XTouchDownstreamMsg>,
) {
    let (from_reaper_tx, from_reaper_rx) = unbounded();
    let (to_reaper_tx, to_reaper_rx) = unbounded();
    let (from_xtouch_tx, from_xtouch_rx) = unbounded();
    let (to_xtouch_tx, to_xtouch_rx) = unbounded();

    let mode = VolumePanMode::new(
        8, // num_channels
        from_reaper_rx,
        to_reaper_tx,
        from_xtouch_rx,
        to_xtouch_tx,
    );

    (
        mode,
        from_reaper_tx,
        to_reaper_rx,
        from_xtouch_tx,
        to_xtouch_rx,
    )
}

// ============================================================================
// Helper Functions for Asserting Messages
// ============================================================================

const FLOAT_EPSILON: f64 = 0.0001;

/// Helper to assert a FaderAbs message is received with the expected values
#[macro_export]
macro_rules! assert_downstream_fader_abs_msg {
    ($rx:expr, $expected_idx:expr, $expected_value:expr) => {{
        let msg = $rx
            .recv_timeout(Duration::from_millis(100))
            .expect("Expected to receive a FaderAbs message.");

        if let XTouchDownstreamMsg::FaderAbs(fader_msg) = msg {
            check!(fader_msg.idx == $expected_idx);
            check!(
                approx_eq!(
                    f64,
                    fader_msg.value,
                    $expected_value,
                    epsilon = FLOAT_EPSILON
                ),
                "Fader value should match approximately\nExpected: {}, Got: {}",
                $expected_value,
                fader_msg.value
            );
        } else {
            panic!("Expected XTouchDownstreamMsg::FaderAbs, but got {:?}", msg);
        }
    }};
}

/// Macro to assert an EncoderRingLED message is received with the expected values
#[macro_export]
macro_rules! assert_downstream_encoder_ring_led_msg {
    ($rx:expr, $expected_idx:expr, $expected_pos:expr) => {{
        let result = $rx.recv_timeout(std::time::Duration::from_millis(100));
        check!(
            result.is_ok(),
            "Should receive XTouch encoder ring LED message"
        );

        match result {
            Ok(XTouchDownstreamMsg::EncoderRingLED(
                arpad_rust::midi::xtouch::EncoderRingLEDMsg::RangePoint(msg),
            )) => {
                check!(msg.idx == $expected_idx, "Encoder index should match");
                check!(
                    approx_eq!(f32, msg.pos, $expected_pos, epsilon = EPSILON),
                    "Encoder position should match approximately\nExpected: {}, Got: {}",
                    $expected_pos,
                    msg.pos
                );
            }
            _ => panic!(
                "Expected EncoderRingLED RangePoint message but got {:?}",
                result
            ),
        }
    }};
}

/// Macro to assert a MuteLED message is received
#[macro_export]
macro_rules! assert_downstream_mute_led_msg {
    ($rx:expr, $expected_idx:expr, $expected_state:expr) => {{
        let result = $rx.recv_timeout(std::time::Duration::from_millis(100));
        check!(result.is_ok(), "Should receive MuteLED message");

        match result {
            Ok(XTouchDownstreamMsg::MuteLED(msg)) => {
                check!(msg.idx == $expected_idx, "Mute LED index should match");
                check!(
                    &msg.state == &$expected_state,
                    "Mute LED state should match"
                );
            }
            _ => panic!("Expected MuteLED message but got {:?}", result),
        }
    }};
}

/// Macro to assert a SoloLED message is received
#[macro_export]
macro_rules! assert_downstream_solo_led_msg {
    ($rx:expr, $expected_idx:expr, $expected_state:expr) => {{
        let result = $rx.recv_timeout(std::time::Duration::from_millis(100));
        check!(result.is_ok(), "Should receive SoloLED message");

        match result {
            Ok(XTouchDownstreamMsg::SoloLED(msg)) => {
                check!(msg.idx == $expected_idx, "Solo LED index should match");
                check!(
                    &msg.state == &$expected_state,
                    "Solo LED state should match"
                );
            }
            _ => panic!("Expected SoloLED message but got {:?}", result),
        }
    }};
}

/// Macro to assert an ArmLED message is received
#[macro_export]
macro_rules! assert_downstream_arm_led_msg {
    ($rx:expr, $expected_idx:expr, $expected_state:expr) => {{
        let result = $rx.recv_timeout(std::time::Duration::from_millis(100));
        check!(result.is_ok(), "Should receive ArmLED message");

        match result {
            Ok(XTouchDownstreamMsg::ArmLED(msg)) => {
                check!(msg.idx == $expected_idx, "Arm LED index should match");
                check!(&msg.state == &$expected_state, "Arm LED state should match");
            }
            _ => panic!("Expected ArmLED message but got {:?}", result),
        }
    }};
}

/// Macro to assert a Volume TrackDataMsg is received upstream
#[macro_export]
macro_rules! assert_volume_track_msg {
    ($rx:expr, $expected_guid:expr, $expected_value:expr) => {{
        let result = $rx.recv_timeout(std::time::Duration::from_millis(100));
        check!(result.is_ok(), "Should receive volume message to Reaper");

        match result {
            Ok(TrackMsg::TrackDataMsg(msg)) => {
                check!(&msg.guid == $expected_guid, "Track GUID should match");
                check!(msg.direction == Direction::Upstream, "Should be upstream");
                match msg.data {
                    DataPayload::Volume(volume) => {
                        check!(
                            approx_eq!(f32, volume, $expected_value, epsilon = EPSILON),
                            "Volume should match approximately\nExpected: {}, Got: {}",
                            $expected_value,
                            volume
                        );
                    }
                    _ => panic!("Expected Volume payload"),
                }
            }
            _ => panic!("Expected TrackDataMsg but got {:?}", result),
        }
    }};
}

/// Macro to assert a Muted TrackDataMsg is received upstream
#[macro_export]
macro_rules! assert_upstream_muted_track_msg {
    ($rx:expr, $expected_guid:expr, $expected_muted:expr) => {{
        let result = $rx.recv_timeout(std::time::Duration::from_millis(100));
        check!(result.is_ok(), "Should receive muted message to Reaper");

        match result {
            Ok(TrackMsg::TrackDataMsg(msg)) => {
                check!(&msg.guid == $expected_guid, "Track GUID should match");
                match msg.data {
                    DataPayload::Muted(muted) => {
                        check!(muted == $expected_muted, "Muted state should match");
                    }
                    _ => panic!("Expected Muted payload"),
                }
            }
            _ => panic!("Expected TrackDataMsg but got {:?}", result),
        }
    }};
}

/// Macro to assert a Soloed TrackDataMsg is received upstream
#[macro_export]
macro_rules! assert_upstream_soloed_track_msg {
    ($rx:expr, $expected_guid:expr, $expected_soloed:expr) => {{
        let result = $rx.recv_timeout(std::time::Duration::from_millis(100));
        check!(result.is_ok(), "Should receive soloed message to Reaper");

        match result {
            Ok(TrackMsg::TrackDataMsg(msg)) => {
                check!(&msg.guid == $expected_guid, "Track GUID should match");
                match msg.data {
                    DataPayload::Soloed(soloed) => {
                        check!(soloed == $expected_soloed, "Soloed state should match");
                    }
                    _ => panic!("Expected Soloed payload"),
                }
            }
            _ => panic!("Expected TrackDataMsg but got {:?}", result),
        }
    }};
}

/// Macro to assert an Armed TrackDataMsg is received upstream
#[macro_export]
macro_rules! assert_upstream_armed_track_msg {
    ($rx:expr, $expected_guid:expr, $expected_armed:expr) => {{
        let result = $rx.recv_timeout(std::time::Duration::from_millis(100));
        check!(result.is_ok(), "Should receive armed message to Reaper");

        match result {
            Ok(TrackMsg::TrackDataMsg(msg)) => {
                check!(&msg.guid == $expected_guid, "Track GUID should match");
                match msg.data {
                    DataPayload::Armed(armed) => {
                        check!(armed == $expected_armed, "Armed state should match");
                    }
                    _ => panic!("Expected Armed payload"),
                }
            }
            _ => panic!("Expected TrackDataMsg but got {:?}", result),
        }
    }};
}

/// Macro to assert no message is received within timeout
#[macro_export]
macro_rules! check_no_message {
    ($rx:expr, $timeout_ms:expr) => {{
        let result = $rx.recv_timeout(std::time::Duration::from_millis($timeout_ms));
        check!(
            result.is_err(),
            "Should not receive any message, but got {:?}!",
            result
        );
    }};
}

/// Helper function to assign a track to a hardware channel
fn assign_track_to_channel(
    mode: &mut VolumePanMode,
    guid: &str,
    hw_channel: i32,
    curr_mode: ModeState,
) -> ModeState {
    mode.handle_downstream_messages(
        TrackMsg::TrackDataMsg(TrackDataMsg {
            guid: guid.to_string(),
            direction: Direction::Downstream,
            data: DataPayload::ReaperTrackIndex(Some(hw_channel)),
        }),
        curr_mode,
    )
}

#[test]
fn test_vol_pan_mode_assigns_tracks_by_reaper_index() {
    let (mut mode, _from_reaper_tx, _to_reaper_rx, _from_xtouch_tx, _to_xtouch_rx) =
        setup_vol_pan_mode();

    let track_guid = "track-guid-1".to_string();
    let reaper_index = 2;

    let curr_mode = ModeState {
        mode: Mode::ReaperVolPan,
        state: State::Active,
    };

    // Send a ReaperTrackIndex message to assign the track to hardware channel 2
    let msg = TrackMsg::TrackDataMsg(TrackDataMsg {
        guid: track_guid.clone(),
        direction: Direction::Downstream,
        data: DataPayload::ReaperTrackIndex(Some(reaper_index)),
    });

    let result_mode = mode.handle_downstream_messages(msg, curr_mode);

    // Mode should remain unchanged
    assert_eq!(result_mode, curr_mode);

    // Verify the track is now assigned to hardware channel 2
    let found_channel = mode.find_hw_channel(&track_guid);
    assert_eq!(
        found_channel,
        Some(reaper_index as usize),
        "Track should be assigned to hardware channel matching Reaper index"
    );
}

#[test]
fn test_vol_pan_mode_volume_updates_sent_to_faders() {
    let (mut mode, _from_reaper_tx, _to_reaper_rx, _from_xtouch_tx, to_xtouch_rx) =
        setup_vol_pan_mode();

    let track_guid = "track-guid-2".to_string();
    let hw_channel = 3;
    let test_volume = 0.65;

    let curr_mode = ModeState {
        mode: Mode::ReaperVolPan,
        state: State::Active,
    };

    // First, assign the track to a hardware channel
    mode.handle_downstream_messages(
        TrackMsg::TrackDataMsg(TrackDataMsg {
            guid: track_guid.clone(),
            direction: Direction::Downstream,
            data: DataPayload::ReaperTrackIndex(Some(hw_channel)),
        }),
        curr_mode,
    );

    assert_downstream_fader_abs_msg!(&to_xtouch_rx, hw_channel, FADER_0DB as f64);

    // Now send a volume update
    mode.handle_downstream_messages(
        TrackMsg::TrackDataMsg(TrackDataMsg {
            guid: track_guid.clone(),
            direction: Direction::Downstream,
            data: DataPayload::Volume(test_volume),
        }),
        curr_mode,
    );

    // Should receive a fader update on XTouch
    let result = to_xtouch_rx.recv_timeout(Duration::from_millis(100));
    assert!(result.is_ok(), "Should receive XTouch fader message");

    check!(result.is_ok(), "Should receive XTouch fader message");

    if let Ok(XTouchDownstreamMsg::FaderAbs(fader_msg)) = result {
        check!(fader_msg.idx == hw_channel, "Fader index should match");
        check!(
            approx_eq!(
                f64,
                fader_msg.value,
                test_volume as f64,
                epsilon = FLOAT_EPSILON
            ),
            "Fader value should match approximately\nExpected: {}, Got: {}",
            test_volume as f64,
            fader_msg.value
        );
    } else {
        // else case handled by check! above
    }
}

#[test]
fn test_vol_pan_mode_fader_sends_volume_upstream() {
    let (mut mode, _from_reaper_tx, to_reaper_rx, _from_xtouch_tx, _to_xtouch_rx) =
        setup_vol_pan_mode();

    let track_guid = "track-guid-4".to_string();
    let hw_channel = 0;
    let new_volume = 0.85;

    let curr_mode = ModeState {
        mode: Mode::ReaperVolPan,
        state: State::Active,
    };

    // Assign track to hardware channel
    mode.handle_downstream_messages(
        TrackMsg::TrackDataMsg(TrackDataMsg {
            guid: track_guid.clone(),
            direction: Direction::Downstream,
            data: DataPayload::ReaperTrackIndex(Some(hw_channel)),
        }),
        curr_mode,
    );

    // Simulate fader movement
    let msg = XTouchUpstreamMsg::FaderAbs(FaderAbsMsg {
        idx: hw_channel,
        value: new_volume,
    });

    mode.handle_upstream_messages(msg, curr_mode);

    // Should send volume update to Reaper
    let result = to_reaper_rx.recv_timeout(Duration::from_millis(100));
    assert!(result.is_ok(), "Should send volume message to Reaper");

    if let Ok(TrackMsg::TrackDataMsg(msg)) = result {
        check!(msg.guid == track_guid, "Track GUID should match");
        check!(msg.direction == Direction::Upstream, "Should be upstream");
        if let DataPayload::Volume(volume) = msg.data {
            assert!(
                approx_eq!(f32, volume, new_volume as f32, epsilon = EPSILON),
                "Volume should match approximately\nExpected: {}, Got: {}",
                volume,
                new_volume,
            );
        } else {
            assert!(false, "Expected Volume payload");
        }
    } else {
        assert!(false, "Expected TrackDataMsg");
    }
}

#[test]
fn test_vol_pan_mode_barrier_forwarding() {
    let (mut mode, _from_reaper_tx, _to_reaper_rx, _from_xtouch_tx, to_xtouch_rx) =
        setup_vol_pan_mode();

    let barrier = Barrier::new();
    let curr_mode = ModeState {
        mode: Mode::ReaperVolPan,
        state: State::WaitingBarrierFromUpstream(barrier),
    };

    // Send barrier from upstream
    let result_mode = mode.handle_downstream_messages(TrackMsg::Barrier(barrier), curr_mode);

    // Should transition to WaitingBarrierFromDownstream
    assert_eq!(
        result_mode.state,
        State::WaitingBarrierFromDownstream(barrier)
    );

    // Barrier should be forwarded to XTouch
    let result = to_xtouch_rx.recv_timeout(Duration::from_millis(100));
    assert!(result.is_ok(), "Barrier should be forwarded to XTouch");

    if let Ok(XTouchDownstreamMsg::Barrier(received_barrier)) = result {
        assert!(received_barrier == barrier, "Barrier should match");
    } else {
        assert!(false, "Expected Barrier message");
    }
}

#[test]
fn test_vol_pan_mode_barrier_reflection() {
    let (mut mode, _from_reaper_tx, _to_reaper_rx, _from_xtouch_tx, _to_xtouch_rx) =
        setup_vol_pan_mode();

    let barrier = Barrier::new();
    let curr_mode = ModeState {
        mode: Mode::ReaperVolPan,
        state: State::WaitingBarrierFromDownstream(barrier),
    };

    // Simulate barrier reflecting back from XTouch
    let result_mode = mode.handle_upstream_messages(XTouchUpstreamMsg::Barrier(barrier), curr_mode);

    // Should transition back to Active state
    assert_eq!(
        result_mode.state,
        State::Active,
        "Should return to Active state after barrier completes"
    );
}

// ============================================================================
// COMPREHENSIVE TEST SUITE - 18 Test Cases
// ============================================================================

// ----------------------------------------------------------------------------
// Mapping Tests (Tests 1-4)
// ----------------------------------------------------------------------------

#[test]
fn test_01_volume_message_for_mapped_track_forwards_to_hardware() {
    let (mut mode, _from_reaper_tx, _to_reaper_rx, _from_xtouch_tx, to_xtouch_rx) =
        setup_vol_pan_mode();

    let track_guid = "track-guid-mapped-vol".to_string();
    let hw_channel = 2;
    let test_volume = 0.75;

    let curr_mode = ModeState {
        mode: Mode::ReaperVolPan,
        state: State::Active,
    };

    // Assign track to hardware channel
    assign_track_to_channel(&mut mode, &track_guid, hw_channel, curr_mode);
    assert_downstream_fader_abs_msg!(&to_xtouch_rx, hw_channel, FADER_0DB as f64);
    assert_downstream_mute_led_msg!(&to_xtouch_rx, hw_channel, LEDState::Off);
    assert_downstream_solo_led_msg!(&to_xtouch_rx, hw_channel, LEDState::Off);
    assert_downstream_arm_led_msg!(&to_xtouch_rx, hw_channel, LEDState::Off);
    assert_downstream_encoder_ring_led_msg!(&to_xtouch_rx, hw_channel, 0.5);

    // Send volume update
    mode.handle_downstream_messages(
        TrackMsg::TrackDataMsg(TrackDataMsg {
            guid: track_guid.clone(),
            direction: Direction::Downstream,
            data: DataPayload::Volume(test_volume),
        }),
        curr_mode,
    );

    // Assert fader message is sent to hardware
    assert_downstream_fader_abs_msg!(&to_xtouch_rx, hw_channel, test_volume as f64);
}

#[test]
fn test_02_volume_message_for_unmapped_track_is_ignored() {
    let (mut mode, _from_reaper_tx, _to_reaper_rx, _from_xtouch_tx, to_xtouch_rx) =
        setup_vol_pan_mode();

    let track_guid = "track-guid-unmapped-vol".to_string();
    let test_volume = 0.85;

    let curr_mode = ModeState {
        mode: Mode::ReaperVolPan,
        state: State::Active,
    };

    // Send volume update WITHOUT assigning track to hardware channel
    mode.handle_downstream_messages(
        TrackMsg::TrackDataMsg(TrackDataMsg {
            guid: track_guid.clone(),
            direction: Direction::Downstream,
            data: DataPayload::Volume(test_volume),
        }),
        curr_mode,
    );

    // Assert no message is sent to hardware
    check_no_message!(&to_xtouch_rx, 100);
}

#[test]
fn test_03_upstream_fader_for_mapped_channel_forwards_to_reaper() {
    let (mut mode, _from_reaper_tx, to_reaper_rx, _from_xtouch_tx, _to_xtouch_rx) =
        setup_vol_pan_mode();

    let track_guid = "track-guid-mapped-fader".to_string();
    let hw_channel = 1;
    let new_volume = 0.65;

    let curr_mode = ModeState {
        mode: Mode::ReaperVolPan,
        state: State::Active,
    };

    // Assign track to hardware channel
    assign_track_to_channel(&mut mode, &track_guid, hw_channel, curr_mode);

    // Simulate fader movement from hardware
    mode.handle_upstream_messages(
        XTouchUpstreamMsg::FaderAbs(FaderAbsMsg {
            idx: hw_channel,
            value: new_volume,
        }),
        curr_mode,
    );

    // Assert volume message is sent to Reaper
    assert_volume_track_msg!(&to_reaper_rx, &track_guid, new_volume as f32);
}

#[test]
fn test_04_upstream_fader_for_unmapped_channel_is_ignored() {
    let (mut mode, _from_reaper_tx, to_reaper_rx, _from_xtouch_tx, _to_xtouch_rx) =
        setup_vol_pan_mode();

    let hw_channel = 5;
    let new_volume = 0.55;

    let curr_mode = ModeState {
        mode: Mode::ReaperVolPan,
        state: State::Active,
    };

    // Simulate fader movement WITHOUT assigning any track to this channel
    mode.handle_upstream_messages(
        XTouchUpstreamMsg::FaderAbs(FaderAbsMsg {
            idx: hw_channel,
            value: new_volume,
        }),
        curr_mode,
    );

    // Assert no message is sent to Reaper
    check_no_message!(&to_reaper_rx, 100);
}

// ----------------------------------------------------------------------------
// State Accumulation Tests (Tests 5-7)
// ----------------------------------------------------------------------------

#[test]
fn test_05_volume_state_reflects_latest_value_when_remapped() {
    let (mut mode, _from_reaper_tx, _to_reaper_rx, _from_xtouch_tx, to_xtouch_rx) =
        setup_vol_pan_mode();

    let track_guid = "track-guid-remap".to_string();
    let hw_channel_1 = 2;
    let hw_channel_2 = 4;
    let volume_1 = 0.5;
    let volume_2 = 0.8;

    let curr_mode = ModeState {
        mode: Mode::ReaperVolPan,
        state: State::Active,
    };

    // Assign track to first hardware channel and send volume
    assign_track_to_channel(&mut mode, &track_guid, hw_channel_1, curr_mode);
    assert_downstream_fader_abs_msg!(&to_xtouch_rx, hw_channel_1, FADER_0DB as f64);
    assert_downstream_mute_led_msg!(&to_xtouch_rx, hw_channel_1, LEDState::Off);
    assert_downstream_solo_led_msg!(&to_xtouch_rx, hw_channel_1, LEDState::Off);
    assert_downstream_arm_led_msg!(&to_xtouch_rx, hw_channel_1, LEDState::Off);
    assert_downstream_encoder_ring_led_msg!(&to_xtouch_rx, hw_channel_1, 0.5);
    mode.handle_downstream_messages(
        TrackMsg::TrackDataMsg(TrackDataMsg {
            guid: track_guid.clone(),
            direction: Direction::Downstream,
            data: DataPayload::Volume(volume_1),
        }),
        curr_mode,
    );
    assert_downstream_fader_abs_msg!(&to_xtouch_rx, hw_channel_1, volume_1 as f64);

    // Update volume
    mode.handle_downstream_messages(
        TrackMsg::TrackDataMsg(TrackDataMsg {
            guid: track_guid.clone(),
            direction: Direction::Downstream,
            data: DataPayload::Volume(volume_2),
        }),
        curr_mode,
    );
    assert_downstream_fader_abs_msg!(&to_xtouch_rx, hw_channel_1, volume_2 as f64);

    // Remap to different channel - old mapping should be cleared
    assign_track_to_channel(&mut mode, &track_guid, hw_channel_2, curr_mode);
    assert_downstream_fader_abs_msg!(&to_xtouch_rx, hw_channel_2, volume_2 as f64);
    assert_downstream_mute_led_msg!(&to_xtouch_rx, hw_channel_2, LEDState::Off);
    assert_downstream_solo_led_msg!(&to_xtouch_rx, hw_channel_2, LEDState::Off);
    assert_downstream_arm_led_msg!(&to_xtouch_rx, hw_channel_2, LEDState::Off);
    assert_downstream_encoder_ring_led_msg!(&to_xtouch_rx, hw_channel_2, 0.5);

    // Verify the track can be found via find_hw_channel
    let found_channel = mode.find_hw_channel(&track_guid);
    assert!(
        found_channel.is_some(),
        "Track should be found after remapping"
    );
    // Should return the new channel (hw_channel_2)
    assert_eq!(
        found_channel.unwrap(),
        hw_channel_2 as usize,
        "find_hw_channel returns the remapped channel"
    );

    // Send another volume update - should go to new channel (hw_channel_2)
    let volume_3 = 0.9;
    mode.handle_downstream_messages(
        TrackMsg::TrackDataMsg(TrackDataMsg {
            guid: track_guid.clone(),
            direction: Direction::Downstream,
            data: DataPayload::Volume(volume_3),
        }),
        curr_mode,
    );

    // Volume update should go to the new channel (hw_channel_2)
    assert_downstream_fader_abs_msg!(&to_xtouch_rx, hw_channel_2, volume_3 as f64);
}

#[test]
fn test_06_multiple_button_state_updates_accumulate_correctly() {
    let (mut mode, _from_reaper_tx, _to_reaper_rx, _from_xtouch_tx, to_xtouch_rx) =
        setup_vol_pan_mode();

    let track_guid = "track-guid-buttons".to_string();
    let hw_channel = 3;

    let curr_mode = ModeState {
        mode: Mode::ReaperVolPan,
        state: State::Active,
    };

    // Assign track to hardware channel
    assign_track_to_channel(&mut mode, &track_guid, hw_channel, curr_mode);
    assert_downstream_fader_abs_msg!(&to_xtouch_rx, hw_channel, FADER_0DB as f64);
    assert_downstream_mute_led_msg!(&to_xtouch_rx, hw_channel, LEDState::Off);
    assert_downstream_solo_led_msg!(&to_xtouch_rx, hw_channel, LEDState::Off);
    assert_downstream_arm_led_msg!(&to_xtouch_rx, hw_channel, LEDState::Off);
    assert_downstream_encoder_ring_led_msg!(&to_xtouch_rx, hw_channel, 0.5);

    // Send mute state
    mode.handle_downstream_messages(
        TrackMsg::TrackDataMsg(TrackDataMsg {
            guid: track_guid.clone(),
            direction: Direction::Downstream,
            data: DataPayload::Muted(true),
        }),
        curr_mode,
    );
    assert_downstream_mute_led_msg!(&to_xtouch_rx, hw_channel, LEDState::On);

    // Send solo state
    mode.handle_downstream_messages(
        TrackMsg::TrackDataMsg(TrackDataMsg {
            guid: track_guid.clone(),
            direction: Direction::Downstream,
            data: DataPayload::Soloed(true),
        }),
        curr_mode,
    );
    assert_downstream_solo_led_msg!(&to_xtouch_rx, hw_channel, LEDState::On);

    // Send armed state
    mode.handle_downstream_messages(
        TrackMsg::TrackDataMsg(TrackDataMsg {
            guid: track_guid.clone(),
            direction: Direction::Downstream,
            data: DataPayload::Armed(true),
        }),
        curr_mode,
    );
    assert_downstream_arm_led_msg!(&to_xtouch_rx, hw_channel, LEDState::On);
}

#[test]
fn test_pan_state_accumulates_and_applies_on_mapping() {
    // NOTE: Current implementation limitation - pan state is only stored for mapped tracks.
    // Ideally, state should accumulate for unmapped tracks and be sent when they're mapped.
    // This test documents current behavior.

    let (mut mode, _from_reaper_tx, _to_reaper_rx, _from_xtouch_tx, to_xtouch_rx) =
        setup_vol_pan_mode();

    let track_guid = "track-guid-pan".to_string();
    let hw_channel = 1;
    let pan_value_1 = 0.3;
    let pan_value_2 = 0.7; // Most recent value

    let curr_mode = ModeState {
        mode: Mode::ReaperVolPan,
        state: State::Active,
    };

    // First assign track to hardware channel
    assign_track_to_channel(&mut mode, &track_guid, hw_channel, curr_mode);
    assert_downstream_fader_abs_msg!(&to_xtouch_rx, hw_channel, FADER_0DB as f64);
    assert_downstream_mute_led_msg!(&to_xtouch_rx, hw_channel, LEDState::Off);
    assert_downstream_solo_led_msg!(&to_xtouch_rx, hw_channel, LEDState::Off);
    assert_downstream_arm_led_msg!(&to_xtouch_rx, hw_channel, LEDState::Off);
    assert_downstream_encoder_ring_led_msg!(&to_xtouch_rx, hw_channel, 0.5);

    // Send pan values - they should accumulate
    mode.handle_downstream_messages(
        TrackMsg::TrackDataMsg(TrackDataMsg {
            guid: track_guid.clone(),
            direction: Direction::Downstream,
            data: DataPayload::Pan(pan_value_1),
        }),
        curr_mode,
    );

    // First value should be sent
    assert_downstream_encoder_ring_led_msg!(&to_xtouch_rx, hw_channel, pan_value_1);

    mode.handle_downstream_messages(
        TrackMsg::TrackDataMsg(TrackDataMsg {
            guid: track_guid.clone(),
            direction: Direction::Downstream,
            data: DataPayload::Pan(pan_value_2),
        }),
        curr_mode,
    );

    // Updated value should be sent
    assert_downstream_encoder_ring_led_msg!(&to_xtouch_rx, hw_channel, pan_value_2);
}

#[test]
fn test_pan_state_accumulates_before_mapping() {
    // This test demonstrates IDEAL behavior: state should accumulate for unmapped tracks
    // and be sent downstream when the track is mapped.

    let (mut mode, _from_reaper_tx, _to_reaper_rx, _from_xtouch_tx, to_xtouch_rx) =
        setup_vol_pan_mode();

    let track_guid = "track-guid-pan-accumulate".to_string();
    let hw_channel = 1;
    let pan_value_1 = 0.3;
    let pan_value_2 = 0.7; // Most recent value should be sent

    let curr_mode = ModeState {
        mode: Mode::ReaperVolPan,
        state: State::Active,
    };

    // Send pan values BEFORE mapping - they should be accumulated but not sent downstream yet
    mode.handle_downstream_messages(
        TrackMsg::TrackDataMsg(TrackDataMsg {
            guid: track_guid.clone(),
            direction: Direction::Downstream,
            data: DataPayload::Pan(pan_value_1),
        }),
        curr_mode,
    );

    // No message should be sent yet (track not mapped)
    check_no_message!(&to_xtouch_rx, 100);

    mode.handle_downstream_messages(
        TrackMsg::TrackDataMsg(TrackDataMsg {
            guid: track_guid.clone(),
            direction: Direction::Downstream,
            data: DataPayload::Pan(pan_value_2),
        }),
        curr_mode,
    );

    // Still no message (track not mapped)
    check_no_message!(&to_xtouch_rx, 100);

    // NOW assign track to hardware channel
    assign_track_to_channel(&mut mode, &track_guid, hw_channel, curr_mode);
    assert_downstream_fader_abs_msg!(&to_xtouch_rx, hw_channel, FADER_0DB as f64);
    assert_downstream_mute_led_msg!(&to_xtouch_rx, hw_channel, LEDState::Off);
    assert_downstream_solo_led_msg!(&to_xtouch_rx, hw_channel, LEDState::Off);
    assert_downstream_arm_led_msg!(&to_xtouch_rx, hw_channel, LEDState::Off);
    assert_downstream_encoder_ring_led_msg!(&to_xtouch_rx, hw_channel, pan_value_2);
}

// ----------------------------------------------------------------------------
// Upstream/Downstream Flow Tests (Tests 8-11)
// ----------------------------------------------------------------------------

#[test]
fn test_08_mute_button_sends_correct_upstream_and_downstream_messages() {
    let (mut mode, _from_reaper_tx, to_reaper_rx, _from_xtouch_tx, to_xtouch_rx) =
        setup_vol_pan_mode();

    let track_guid = "track-guid-mute-flow".to_string();
    let hw_channel = 2;

    let curr_mode = ModeState {
        mode: Mode::ReaperVolPan,
        state: State::Active,
    };

    // Assign track to hardware channel
    assign_track_to_channel(&mut mode, &track_guid, hw_channel, curr_mode);
    assert_downstream_fader_abs_msg!(&to_xtouch_rx, hw_channel, FADER_0DB as f64);
    assert_downstream_mute_led_msg!(&to_xtouch_rx, hw_channel, LEDState::Off);
    assert_downstream_solo_led_msg!(&to_xtouch_rx, hw_channel, LEDState::Off);
    assert_downstream_arm_led_msg!(&to_xtouch_rx, hw_channel, LEDState::Off);
    assert_downstream_encoder_ring_led_msg!(&to_xtouch_rx, hw_channel, 0.5);

    // Simulate mute button press
    mode.handle_upstream_messages(
        XTouchUpstreamMsg::MutePress(MutePress { idx: hw_channel }),
        curr_mode,
    );

    // Should send mute message to Reaper (upstream)
    assert_upstream_muted_track_msg!(&to_reaper_rx, &track_guid, true);

    // Should send LED update to hardware (downstream)
    assert_downstream_mute_led_msg!(&to_xtouch_rx, hw_channel, LEDState::On);
}

#[test]
fn test_09_solo_button_sends_correct_messages() {
    let (mut mode, _from_reaper_tx, to_reaper_rx, _from_xtouch_tx, to_xtouch_rx) =
        setup_vol_pan_mode();

    let track_guid = "track-guid-solo-flow".to_string();
    let hw_channel = 4;

    let curr_mode = ModeState {
        mode: Mode::ReaperVolPan,
        state: State::Active,
    };

    // Assign track to hardware channel
    assign_track_to_channel(&mut mode, &track_guid, hw_channel, curr_mode);

    assert_downstream_fader_abs_msg!(&to_xtouch_rx, hw_channel, FADER_0DB as f64);
    assert_downstream_mute_led_msg!(&to_xtouch_rx, hw_channel, LEDState::Off);
    assert_downstream_solo_led_msg!(&to_xtouch_rx, hw_channel, LEDState::Off);
    assert_downstream_arm_led_msg!(&to_xtouch_rx, hw_channel, LEDState::Off);
    assert_downstream_encoder_ring_led_msg!(&to_xtouch_rx, hw_channel, 0.5);

    // Simulate solo button press
    mode.handle_upstream_messages(
        XTouchUpstreamMsg::SoloPress(SoloPress { idx: hw_channel }),
        curr_mode,
    );

    // Should send solo message to Reaper
    assert_upstream_soloed_track_msg!(&to_reaper_rx, &track_guid, true);

    // Should send LED update to hardware
    assert_downstream_solo_led_msg!(&to_xtouch_rx, hw_channel, LEDState::On);
}

#[test]
fn test_10_arm_button_sends_correct_messages() {
    let (mut mode, _from_reaper_tx, to_reaper_rx, _from_xtouch_tx, to_xtouch_rx) =
        setup_vol_pan_mode();

    let track_guid = "track-guid-arm-flow".to_string();
    let hw_channel = 0;

    let curr_mode = ModeState {
        mode: Mode::ReaperVolPan,
        state: State::Active,
    };

    // Assign track to hardware channel
    assign_track_to_channel(&mut mode, &track_guid, hw_channel, curr_mode);
    assert_downstream_fader_abs_msg!(&to_xtouch_rx, hw_channel, FADER_0DB as f64);
    assert_downstream_mute_led_msg!(&to_xtouch_rx, hw_channel, LEDState::Off);
    assert_downstream_solo_led_msg!(&to_xtouch_rx, hw_channel, LEDState::Off);
    assert_downstream_arm_led_msg!(&to_xtouch_rx, hw_channel, LEDState::Off);
    assert_downstream_encoder_ring_led_msg!(&to_xtouch_rx, hw_channel, 0.5);

    // Simulate arm button press
    mode.handle_upstream_messages(
        XTouchUpstreamMsg::ArmPress(ArmPress { idx: hw_channel }),
        curr_mode,
    );

    // Should send arm message to Reaper
    assert_upstream_armed_track_msg!(&to_reaper_rx, &track_guid, true);

    // Should send LED update to hardware
    assert_downstream_arm_led_msg!(&to_xtouch_rx, hw_channel, LEDState::On);
}

#[test]
fn test_11_pan_encoder_changes_forward_correctly() {
    // Encoder inc/dec messages should adjust pan and send updates to Reaper
    let (mut mode, _from_reaper_tx, to_reaper_rx, _from_xtouch_tx, to_xtouch_rx) =
        setup_vol_pan_mode();

    let track_guid = "track-guid-encoder".to_string();
    let hw_channel = 5;
    let initial_pan = 0.5;

    let curr_mode = ModeState {
        mode: Mode::ReaperVolPan,
        state: State::Active,
    };

    // Assign track to hardware channel and set initial pan
    assign_track_to_channel(&mut mode, &track_guid, hw_channel, curr_mode);
    assert_downstream_fader_abs_msg!(&to_xtouch_rx, hw_channel, FADER_0DB as f64);
    assert_downstream_mute_led_msg!(&to_xtouch_rx, hw_channel, LEDState::Off);
    assert_downstream_solo_led_msg!(&to_xtouch_rx, hw_channel, LEDState::Off);
    assert_downstream_arm_led_msg!(&to_xtouch_rx, hw_channel, LEDState::Off);
    assert_downstream_encoder_ring_led_msg!(&to_xtouch_rx, hw_channel, 0.5);

    mode.handle_downstream_messages(
        TrackMsg::TrackDataMsg(TrackDataMsg {
            guid: track_guid.clone(),
            direction: Direction::Downstream,
            data: DataPayload::Pan(initial_pan),
        }),
        curr_mode,
    );
    // Clear the initial pan message
    let _ = to_xtouch_rx.recv_timeout(Duration::from_millis(100));

    // Simulate encoder turn clockwise
    let result_mode = mode.handle_upstream_messages(
        XTouchUpstreamMsg::EncoderTurnInc(EncoderTurnCW { idx: hw_channel }),
        curr_mode,
    );

    // Mode should remain active and send pan update to Reaper
    assert_eq!(result_mode.state, State::Active);

    // Should receive a pan update message sent to Reaper
    let msg = to_reaper_rx.recv_timeout(Duration::from_millis(100));
    assert!(msg.is_ok(), "Should send pan update to Reaper");

    // Should receive an encoder LED update showing new pan position
    let led_msg = to_xtouch_rx.recv_timeout(Duration::from_millis(100));
    assert!(led_msg.is_ok(), "Should send encoder LED update");
}

// ----------------------------------------------------------------------------
// Mode Transition Tests (Tests 12-14)
// ----------------------------------------------------------------------------

#[test]
fn test_12_state_propagates_correctly_during_mode_entry() {
    let (mut mode, _from_reaper_tx, to_reaper_rx, _from_xtouch_tx, _to_xtouch_rx) =
        setup_vol_pan_mode();

    let track_guid_1 = "track-guid-transition-1".to_string();
    let track_guid_2 = "track-guid-transition-2".to_string();
    let hw_channel_1 = 0;
    let hw_channel_2 = 1;

    let curr_mode = ModeState {
        mode: Mode::ReaperVolPan,
        state: State::Active,
    };

    // Assign tracks to hardware channels
    assign_track_to_channel(&mut mode, &track_guid_1, hw_channel_1, curr_mode);
    assign_track_to_channel(&mut mode, &track_guid_2, hw_channel_2, curr_mode);

    // Create an unbounded sender that will be used to send the barrier upstream
    let (upstream_sender, upstream_receiver) = unbounded();

    // Initiate mode transition
    let _result_mode = mode.initiate_mode_transition(upstream_sender);

    // Should send TrackQuery for each assigned track
    let msg1 = to_reaper_rx.recv_timeout(Duration::from_millis(100));
    assert!(msg1.is_ok(), "Should send TrackQuery for first track");

    let msg2 = to_reaper_rx.recv_timeout(Duration::from_millis(100));
    assert!(msg2.is_ok(), "Should send TrackQuery for second track");

    // Should send barrier upstream
    let barrier_msg = upstream_receiver.recv_timeout(Duration::from_millis(100));
    assert!(barrier_msg.is_ok(), "Should send barrier message upstream");
    match barrier_msg.unwrap() {
        TrackMsg::Barrier(_) => {}
        _ => assert!(false, "Expected Barrier message"),
    }
}

#[test]
fn test_13_barrier_messages_handled_during_transitions() {
    let (mut mode, _from_reaper_tx, _to_reaper_rx, _from_xtouch_tx, to_xtouch_rx) =
        setup_vol_pan_mode();

    let barrier = Barrier::new();

    // Test WaitingBarrierFromUpstream -> WaitingBarrierFromDownstream
    let curr_mode = ModeState {
        mode: Mode::ReaperVolPan,
        state: State::WaitingBarrierFromUpstream(barrier),
    };

    let result_mode = mode.handle_downstream_messages(TrackMsg::Barrier(barrier), curr_mode);

    assert_eq!(
        result_mode.state,
        State::WaitingBarrierFromDownstream(barrier),
        "Should transition to waiting for downstream barrier"
    );

    // Barrier should be forwarded to hardware
    let xtouch_msg = to_xtouch_rx.recv_timeout(Duration::from_millis(100));
    assert!(xtouch_msg.is_ok(), "Barrier should be forwarded to XTouch");

    // Test WaitingBarrierFromDownstream -> Active
    let result_mode =
        mode.handle_upstream_messages(XTouchUpstreamMsg::Barrier(barrier), result_mode);

    assert_eq!(
        result_mode.state,
        State::Active,
        "Should transition to Active after downstream barrier returns"
    );
}

#[test]
fn test_14_wrong_barrier_during_transition_maintains_state() {
    let (mut mode, _from_reaper_tx, _to_reaper_rx, _from_xtouch_tx, _to_xtouch_rx) =
        setup_vol_pan_mode();

    let expected_barrier = Barrier::new();
    let wrong_barrier = Barrier::new();

    let curr_mode = ModeState {
        mode: Mode::ReaperVolPan,
        state: State::WaitingBarrierFromUpstream(expected_barrier),
    };

    // Send wrong barrier
    let result_mode = mode.handle_downstream_messages(TrackMsg::Barrier(wrong_barrier), curr_mode);

    // Should remain in same state
    assert_eq!(
        result_mode.state,
        State::WaitingBarrierFromUpstream(expected_barrier),
        "Should remain waiting for correct barrier"
    );
}

// ----------------------------------------------------------------------------
// Message Ordering Tests (Tests 15-16)
// ----------------------------------------------------------------------------

#[test]
fn test_15_downstream_messages_sent_in_correct_order() {
    let (mut mode, _from_reaper_tx, _to_reaper_rx, _from_xtouch_tx, to_xtouch_rx) =
        setup_vol_pan_mode();

    let track_guid = "track-guid-ordering-downstream".to_string();
    let hw_channel = 1;

    let curr_mode = ModeState {
        mode: Mode::ReaperVolPan,
        state: State::Active,
    };

    // Assign track
    assign_track_to_channel(&mut mode, &track_guid, hw_channel, curr_mode);
    assert_downstream_fader_abs_msg!(&to_xtouch_rx, hw_channel, FADER_0DB as f64);
    assert_downstream_mute_led_msg!(&to_xtouch_rx, hw_channel, LEDState::Off);
    assert_downstream_solo_led_msg!(&to_xtouch_rx, hw_channel, LEDState::Off);
    assert_downstream_arm_led_msg!(&to_xtouch_rx, hw_channel, LEDState::Off);
    assert_downstream_encoder_ring_led_msg!(&to_xtouch_rx, hw_channel, 0.5);

    // Send multiple messages in order
    mode.handle_downstream_messages(
        TrackMsg::TrackDataMsg(TrackDataMsg {
            guid: track_guid.clone(),
            direction: Direction::Downstream,
            data: DataPayload::Volume(0.5),
        }),
        curr_mode,
    );

    mode.handle_downstream_messages(
        TrackMsg::TrackDataMsg(TrackDataMsg {
            guid: track_guid.clone(),
            direction: Direction::Downstream,
            data: DataPayload::Pan(0.3),
        }),
        curr_mode,
    );

    mode.handle_downstream_messages(
        TrackMsg::TrackDataMsg(TrackDataMsg {
            guid: track_guid.clone(),
            direction: Direction::Downstream,
            data: DataPayload::Muted(true),
        }),
        curr_mode,
    );

    // Verify messages received in order
    let msg1 = to_xtouch_rx.recv_timeout(Duration::from_millis(100));
    assert!(
        matches!(msg1, Ok(XTouchDownstreamMsg::FaderAbs(_))),
        "First should be fader"
    );

    let msg2 = to_xtouch_rx.recv_timeout(Duration::from_millis(100));
    assert!(
        matches!(msg2, Ok(XTouchDownstreamMsg::EncoderRingLED(_))),
        "Second should be encoder"
    );

    let msg3 = to_xtouch_rx.recv_timeout(Duration::from_millis(100));
    assert!(
        matches!(msg3, Ok(XTouchDownstreamMsg::MuteLED(_))),
        "Third should be mute LED"
    );
}

#[test]
fn test_16_upstream_messages_processed_in_correct_order() {
    let (mut mode, _from_reaper_tx, to_reaper_rx, _from_xtouch_tx, _to_xtouch_rx) =
        setup_vol_pan_mode();

    let track_guid = "track-guid-ordering-upstream".to_string();
    let hw_channel = 3;

    let curr_mode = ModeState {
        mode: Mode::ReaperVolPan,
        state: State::Active,
    };

    // Assign track
    assign_track_to_channel(&mut mode, &track_guid, hw_channel, curr_mode);
    assert_downstream_fader_abs_msg!(&_to_xtouch_rx, hw_channel, FADER_0DB as f64);
    assert_downstream_mute_led_msg!(&_to_xtouch_rx, hw_channel, LEDState::Off);
    assert_downstream_solo_led_msg!(&_to_xtouch_rx, hw_channel, LEDState::Off);
    assert_downstream_arm_led_msg!(&_to_xtouch_rx, hw_channel, LEDState::Off);
    assert_downstream_encoder_ring_led_msg!(&_to_xtouch_rx, hw_channel, 0.5);

    // Send multiple upstream messages in order
    mode.handle_upstream_messages(
        XTouchUpstreamMsg::FaderAbs(FaderAbsMsg {
            idx: hw_channel,
            value: 0.6,
        }),
        curr_mode,
    );

    mode.handle_upstream_messages(
        XTouchUpstreamMsg::MutePress(MutePress { idx: hw_channel }),
        curr_mode,
    );

    // Verify messages processed in order (volume then mute)
    let msg1 = to_reaper_rx.recv_timeout(Duration::from_millis(100));
    assert!(msg1.is_ok(), "Should receive first message");
    if let Ok(TrackMsg::TrackDataMsg(msg)) = msg1 {
        assert!(
            matches!(msg.data, DataPayload::Volume(_)),
            "First should be volume"
        );
    }

    let msg2 = to_reaper_rx.recv_timeout(Duration::from_millis(100));
    assert!(msg2.is_ok(), "Should receive second message");
    if let Ok(TrackMsg::TrackDataMsg(msg)) = msg2 {
        assert!(
            matches!(msg.data, DataPayload::Muted(_)),
            "Second should be muted"
        );
    }
}

// ----------------------------------------------------------------------------
// Threshold/EPSILON Tests (Tests 17-18)
// ----------------------------------------------------------------------------

#[test]
fn test_17_volume_changes_below_epsilon_threshold_ignored() {
    // Volume changes smaller than EPSILON should not send updates to hardware
    let (mut mode, _from_reaper_tx, _to_reaper_rx, _from_xtouch_tx, to_xtouch_rx) =
        setup_vol_pan_mode();

    let track_guid = "track-guid-epsilon-vol".to_string();
    let hw_channel = 2;
    let initial_volume = 0.5;

    let curr_mode = ModeState {
        mode: Mode::ReaperVolPan,
        state: State::Active,
    };

    // Assign track and set initial volume
    assign_track_to_channel(&mut mode, &track_guid, hw_channel, curr_mode);
    assert_downstream_fader_abs_msg!(&to_xtouch_rx, hw_channel, FADER_0DB as f64);
    assert_downstream_mute_led_msg!(&to_xtouch_rx, hw_channel, LEDState::Off);
    assert_downstream_solo_led_msg!(&to_xtouch_rx, hw_channel, LEDState::Off);
    assert_downstream_arm_led_msg!(&to_xtouch_rx, hw_channel, LEDState::Off);
    assert_downstream_encoder_ring_led_msg!(&to_xtouch_rx, hw_channel, 0.5);

    mode.handle_downstream_messages(
        TrackMsg::TrackDataMsg(TrackDataMsg {
            guid: track_guid.clone(),
            direction: Direction::Downstream,
            data: DataPayload::Volume(initial_volume),
        }),
        curr_mode,
    );
    assert_downstream_fader_abs_msg!(&to_xtouch_rx, hw_channel, initial_volume as f64);

    // Send volume change smaller than EPSILON
    let small_change = initial_volume + (EPSILON / 2.0);
    mode.handle_downstream_messages(
        TrackMsg::TrackDataMsg(TrackDataMsg {
            guid: track_guid.clone(),
            direction: Direction::Downstream,
            data: DataPayload::Volume(small_change),
        }),
        curr_mode,
    );

    // Should NOT send message for changes smaller than EPSILON
    check_no_message!(&to_xtouch_rx, 100);
}

#[test]
fn test_18_pan_changes_below_epsilon_threshold_ignored() {
    // Pan changes smaller than EPSILON should not send updates to hardware
    let (mut mode, _from_reaper_tx, _to_reaper_rx, _from_xtouch_tx, to_xtouch_rx) =
        setup_vol_pan_mode();

    let track_guid = "track-guid-epsilon-pan".to_string();
    let hw_channel = 1;
    let initial_pan = 0.5;

    let curr_mode = ModeState {
        mode: Mode::ReaperVolPan,
        state: State::Active,
    };

    // Assign track and set initial pan
    assign_track_to_channel(&mut mode, &track_guid, hw_channel, curr_mode);
    assert_downstream_fader_abs_msg!(&to_xtouch_rx, hw_channel, FADER_0DB as f64);
    assert_downstream_mute_led_msg!(&to_xtouch_rx, hw_channel, LEDState::Off);
    assert_downstream_solo_led_msg!(&to_xtouch_rx, hw_channel, LEDState::Off);
    assert_downstream_arm_led_msg!(&to_xtouch_rx, hw_channel, LEDState::Off);
    assert_downstream_encoder_ring_led_msg!(&to_xtouch_rx, hw_channel, 0.5);

    mode.handle_downstream_messages(
        TrackMsg::TrackDataMsg(TrackDataMsg {
            guid: track_guid.clone(),
            direction: Direction::Downstream,
            data: DataPayload::Pan(initial_pan),
        }),
        curr_mode,
    );
    assert_downstream_encoder_ring_led_msg!(&to_xtouch_rx, hw_channel, initial_pan);

    // Send pan change smaller than EPSILON
    let small_change = initial_pan + (EPSILON / 2.0);
    mode.handle_downstream_messages(
        TrackMsg::TrackDataMsg(TrackDataMsg {
            guid: track_guid.clone(),
            direction: Direction::Downstream,
            data: DataPayload::Pan(small_change),
        }),
        curr_mode,
    );

    // Should NOT send message for changes smaller than EPSILON
    check_no_message!(&to_xtouch_rx, 100);
}
