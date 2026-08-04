use std::time::Duration;

use assert2::{assert, check};
use crossbeam_channel::{Receiver, unbounded};
use float_cmp::approx_eq;

use arpad_rust::midi::v1m;
use arpad_rust::midi::v1m::{ChannelFaderMsg, DownstreamMsg, UpstreamMsg};
use arpad_rust::modes::mode_manager::{IoDirect, ModeAction, ModeHandler, TransitionRequest};
use arpad_rust::modes::reaper_track_sends_mode::ReaperTrackSendsMode;
use arpad_rust::track::track::{self as track, TrackMsg};

// EPSILON constant for floating-point threshold testing
const EPSILON: f32 = 0.01;

// fn recv_track(rx: &Receiver<TrackMsg>) -> TrackMsg {
//     rx.recv_timeout(Duration::from_millis(100))
//         .expect("expected TrackMsg")
// }
//
// fn recv_v1m(rx: &Receiver<v1m::DownstreamMsg>) -> v1m::DownstreamMsg {
//     rx.recv_timeout(Duration::from_millis(100))
//         .expect("expected v1m::DownstreamMsg")
// }

/// Helper to create a ReaperTrackSendsMode instance for testing
fn setup_track_sends_mode(
    selected_track_guid: uuid::Uuid,
) -> (
    ReaperTrackSendsMode,
    Receiver<TrackMsg>,
    Receiver<DownstreamMsg>,
    IoDirect,
) {
    let (to_reaper_tx, to_reaper_rx) = unbounded();
    let (to_v1m_tx, to_v1m_rx) = unbounded();

    let mode = ReaperTrackSendsMode::new(8, 0, selected_track_guid);

    let io_direct = IoDirect::new(to_reaper_tx, to_v1m_tx);

    (mode, to_reaper_rx, to_v1m_rx, io_direct)
}

// ============================================================================
// Helper Functions for Asserting Messages
// ============================================================================

const FLOAT_EPSILON: f64 = 0.0001;

pub fn drain<T>(rx: &Receiver<T>) {
    // Drops (flushes) all messages currently buffered at the time we start draining,
    // plus any that arrive before we hit Empty.
    for _msg in rx.try_iter() {
        // intentionally discard
    }
}

/// Helper to assert a ChannelFader message is received with the expected values
#[macro_export]
macro_rules! assert_downstream_fader_abs_msg {
    ($rx:expr, $expected_idx:expr, $expected_value:expr) => {{
        let msg = $rx
            .recv_timeout(Duration::from_millis(100))
            .expect("Expected to receive a ChannelFader message.");

        if let DownstreamMsg::ChannelFader(fader_msg) = msg {
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
            panic!("Expected DownstreamMsg::ChannelFader, but got {:?}", msg);
        }
    }};
}

/// Macro to assert an EncoderRingLED message is received with the expected values
#[macro_export]
macro_rules! assert_downstream_encoder_ring_led_msg {
    ($rx:expr, $expected_idx:expr, $expected_val:expr) => {{
        let result = $rx.recv_timeout(std::time::Duration::from_millis(100));
        check!(
            result.is_ok(),
            "Should receive v1m encoder ring LED message"
        );

        match result {
            Ok(DownstreamMsg::EncoderRingLED(msg)) => {
                check!(msg.idx == $expected_idx, "Encoder index should match");
                check!(msg.val == $expected_val, "Encoder value should match");
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
            Ok(DownstreamMsg::MuteLED(msg)) => {
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
            Ok(DownstreamMsg::SoloLED(msg)) => {
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
            Ok(DownstreamMsg::ArmLED(msg)) => {
                check!(msg.idx == $expected_idx, "Arm LED index should match");
                check!(&msg.state == &$expected_state, "Arm LED state should match");
            }
            _ => panic!("Expected ArmLED message but got {:?}", result),
        }
    }};
}

/// Macro to assert a SendLevel TrackDataMsg is received upstream
#[macro_export]
macro_rules! assert_upstream_send_level_track_msg {
    ($rx:expr, $expected_guid:expr, $expected_send_index:expr, $expected_level:expr) => {{
        let result = $rx.recv_timeout(std::time::Duration::from_millis(1));
        check!(
            result.is_ok(),
            "Should receive send level message to Reaper"
        );

        match result {
            Ok(TrackMsg::SendLevel(msg)) => {
                check!(
                    msg.send_index == $expected_send_index,
                    "Send index should match"
                );
                check!(
                    approx_eq!(f32, msg.level, $expected_level, epsilon = EPSILON),
                    "Send level should match approximately\nExpected: {}, Got: {}",
                    $expected_level,
                    msg.level
                );
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
            Ok(TrackMsg::Muted(msg)) => {
                check!(msg.track_guid == *$expected_guid, "Track GUID should match");
                check!(msg.muted == $expected_muted, "Muted state should match");
            }
            _ => panic!("Expected Muted TrackMsg but got {:?}", result),
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
            Ok(TrackMsg::Soloed(msg)) => {
                check!(msg.track_guid == *$expected_guid, "Track GUID should match");
                check!(msg.soloed == $expected_soloed, "Soloed state should match");
            }
            _ => panic!("Expected Soloed TrackMsg but got {:?}", result),
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
            Ok(TrackMsg::Armed(msg)) => {
                check!(msg.track_guid == *$expected_guid, "Track GUID should match");
                check!(msg.armed == $expected_armed, "Armed state should match");
            }
            _ => panic!("Expected Armed TrackMsg but got {:?}", result),
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

#[test]
#[cfg(test)]
fn test_track_send_mode_assigns_tracks_by_send_index() {
    let track_guid = uuid::Uuid::new_v4();
    let send_guid = uuid::Uuid::new_v4();
    let send_index = 0;

    let (mut mode, _to_reaper_rx, _to_v1m_rx, mut io_direct) = setup_track_sends_mode(track_guid);

    // Send a ReaperTrackIndex message to assign the track to hardware channel 2
    let msg = track::SendIndex {
        track_guid,
        send_guid,
        send_index,
    }
    .into();

    let mode_action = mode.handle_msg_from_upstream(msg, &mut io_direct);

    // Mode should remain unchanged
    assert_eq!(mode_action, ModeAction::None);

    // Verify the track is now assigned to hardware channel 2
    let found_channel = mode.find_hw_channel(send_guid);
    assert_eq!(
        found_channel,
        Some(send_index as usize),
        "Track should be assigned to hardware channel matching Reaper index"
    );
}

#[test]
fn test_track_send_mode_level_updates_sent_to_faders() {
    let track_guid = uuid::Uuid::new_v4();
    let (mut mode, _to_reaper_rx, to_v1m_rx, mut io_direct) = setup_track_sends_mode(track_guid);

    let send_guid = uuid::Uuid::new_v4();
    let send_index = 2;
    let level = 0.65;

    mode.handle_msg_from_upstream(
        track::SendIndex {
            track_guid,
            send_guid,
            send_index,
        }
        .into(),
        &mut io_direct,
    );

    assert_downstream_fader_abs_msg!(&to_v1m_rx, send_index, 0 as f64);
    assert_downstream_encoder_ring_led_msg!(
        &to_v1m_rx,
        send_index,
        v1m::ENCODER_CENTER_MODE_CENTER
    );

    mode.handle_msg_from_upstream(
        track::SendLevel {
            track_guid,
            send_index,
            level,
        }
        .into(),
        &mut io_direct,
    );

    assert_downstream_fader_abs_msg!(&to_v1m_rx, send_index, level as f64);
}

#[test]
fn test_track_send_mode_fader_sends_volume_upstream() {
    let track_guid = uuid::Uuid::new_v4();
    let (mut mode, to_reaper_rx, _to_v1m_rx, mut io_direct) = setup_track_sends_mode(track_guid);

    let send_guid = uuid::Uuid::new_v4();
    let send_index = 2;
    let level = 0.65;

    mode.handle_msg_from_upstream(
        track::SendIndex {
            track_guid,
            send_guid,
            send_index,
        }
        .into(),
        &mut io_direct,
    );

    // Simulate fader movement
    mode.handle_msg_from_downstream(
        ChannelFaderMsg {
            idx: send_index,
            value: level,
        }
        .into(),
        &mut io_direct,
    );

    // Should send send level update to Reaper
    let result = to_reaper_rx.recv_timeout(Duration::from_millis(100));
    assert!(result.is_ok(), "Should send send level message to Reaper");

    if let Ok(TrackMsg::SendLevel(msg)) = result {
        check!(msg.track_guid == track_guid, "Track GUID should match");
        assert!(
            approx_eq!(f32, msg.level, level as f32, epsilon = EPSILON),
            "Volume should match approximately\nExpected: {}, Got: {}",
            msg.level,
            level,
        );
    } else {
        assert!(false, "Expected SendLevel TrackMsg");
    }
}

// ============================================================================
// COMPREHENSIVE TEST SUITE
// ============================================================================

// ----------------------------------------------------------------------------
// Mapping Tests
// ----------------------------------------------------------------------------

#[test]
fn test_message_for_unselected_track_is_ignored() {
    let track_guid = uuid::Uuid::new_v4();
    let (mut mode, _to_reaper_rx, to_v1m_rx, mut io_direct) = setup_track_sends_mode(track_guid);

    // Send a message for a different track
    let other_track_guid = uuid::Uuid::new_v4();
    let mode_action = mode.handle_msg_from_upstream(
        track::SendLevel {
            track_guid: other_track_guid,
            send_index: 0,
            level: 0.5,
        }
        .into(),
        &mut io_direct,
    );

    assert_eq!(
        mode_action,
        ModeAction::None,
        "Mode should remain unchanged"
    );

    // Assert no message is sent to hardware
    check_no_message!(&to_v1m_rx, 100);
}

#[test]
fn test_send_level_message_for_unmapped_send_is_ignored() {
    let track_guid = uuid::Uuid::new_v4();
    let level = 0.85;
    let (mut mode, _to_reaper_rx, to_v1m_rx, mut io_direct) = setup_track_sends_mode(track_guid);

    // Send volume update WITHOUT assigning track to hardware channel
    mode.handle_msg_from_upstream(
        track::SendLevel {
            track_guid,
            send_index: 0,
            level,
        }
        .into(),
        &mut io_direct,
    );

    // Assert no message is sent to hardware
    check_no_message!(&to_v1m_rx, 100);
}

#[test]
fn test_upstream_fader_for_unmapped_channel_is_ignored() {
    let track_guid = uuid::Uuid::new_v4();
    let (mut mode, to_reaper_rx, _to_v1m_rx, mut io_direct) = setup_track_sends_mode(track_guid);

    let hw_channel = 5;
    let new_level = 0.55;

    // Simulate fader movement WITHOUT assigning any send to this channel
    mode.handle_msg_from_downstream(
        UpstreamMsg::ChannelFader(ChannelFaderMsg {
            idx: hw_channel,
            value: new_level,
        }),
        &mut io_direct,
    );
    // Assert no message is sent to Reaper
    check_no_message!(&to_reaper_rx, 100);

    // Assign a send
    let send_guid = uuid::Uuid::new_v4();
    let send_index = 2;
    mode.handle_msg_from_upstream(
        track::SendIndex {
            track_guid,
            send_guid,
            send_index,
        }
        .into(),
        &mut io_direct,
    );

    // Simulate fader movement from an unassigned hw_channel
    mode.handle_msg_from_downstream(
        UpstreamMsg::ChannelFader(ChannelFaderMsg {
            idx: 0,
            value: new_level,
        }),
        &mut io_direct,
    );
    // Assert no message is sent to Reaper
    check_no_message!(&to_reaper_rx, 100);
}

// ----------------------------------------------------------------------------
// State Accumulation Tests
// ----------------------------------------------------------------------------

#[test]
fn test_send_state_reflects_latest_value_when_moved() {
    let track_guid = uuid::Uuid::new_v4();
    let (mut mode, _to_reaper_rx, to_v1m_rx, mut io_direct) = setup_track_sends_mode(track_guid);

    let send_guid_1 = uuid::Uuid::new_v4();
    let send_idx_1 = 2;
    let send_idx_2 = 4;
    let level_1 = 0.5;
    let level_2 = 0.8;
    let pan_1 = 0.1;
    let pan_2 = 0.3;

    // Assign track to first hardware channel
    mode.handle_msg_from_upstream(
        track::SendIndex {
            track_guid,
            send_guid: send_guid_1,
            send_index: send_idx_1,
        }
        .into(),
        &mut io_direct,
    );
    assert_downstream_fader_abs_msg!(&to_v1m_rx, send_idx_1, 0 as f64);
    // TODO: what is the expected behavior of mute/solo/arm? Do they do anything?
    // assert_downstream_mute_led_msg!(&to_v1m_rx, send_idx_1, LEDState::Off);
    // assert_downstream_solo_led_msg!(&to_v1m_rx, send_idx_1, LEDState::Off);
    // assert_downstream_arm_led_msg!(&to_v1m_rx, send_idx_1, LEDState::Off);
    assert_downstream_encoder_ring_led_msg!(
        &to_v1m_rx,
        send_idx_1,
        v1m::ENCODER_CENTER_MODE_CENTER
    );
    // Update level
    mode.handle_msg_from_upstream(
        track::SendLevel {
            track_guid,
            send_index: send_idx_1,
            level: level_1,
        }
        .into(),
        &mut io_direct,
    );
    assert_downstream_fader_abs_msg!(&to_v1m_rx, send_idx_1, level_1 as f64);
    mode.handle_msg_from_upstream(
        track::SendPan {
            track_guid,
            send_index: send_idx_1,
            pan: pan_1,
        }
        .into(),
        &mut io_direct,
    );
    assert_downstream_encoder_ring_led_msg!(
        &to_v1m_rx,
        send_idx_1,
        v1m::map_to_encoder_ring(pan_1)
    );

    // Remap to different channel - old mapping should be cleared
    mode.handle_msg_from_upstream(
        track::SendIndex {
            track_guid,
            send_guid: send_guid_1,
            send_index: send_idx_2,
        }
        .into(),
        &mut io_direct,
    );
    assert_downstream_fader_abs_msg!(&to_v1m_rx, send_idx_1, 0 as f64);
    // TODO:
    // assert_downstream_mute_led_msg!(&to_v1m_rx, send_idx_1, LEDState::Off);
    // assert_downstream_solo_led_msg!(&to_v1m_rx, send_idx_1, LEDState::Off);
    // assert_downstream_arm_led_msg!(&to_v1m_rx, send_idx_1, LEDState::Off);
    assert_downstream_encoder_ring_led_msg!(
        &to_v1m_rx,
        send_idx_1,
        v1m::ENCODER_CENTER_MODE_CENTER
    );
    assert_downstream_fader_abs_msg!(&to_v1m_rx, send_idx_2, level_1 as f64);
    // TODO:
    // assert_downstream_mute_led_msg!(&to_v1m_rx, send_idx_2, LEDState::Off);
    // assert_downstream_solo_led_msg!(&to_v1m_rx, send_idx_2, LEDState::Off);
    // assert_downstream_arm_led_msg!(&to_v1m_rx, send_idx_2, LEDState::Off);
    assert_downstream_encoder_ring_led_msg!(
        &to_v1m_rx,
        send_idx_2,
        v1m::map_to_encoder_ring(pan_1)
    );

    // Verify the send can be found via find_hw_channel
    let found_channel = mode.find_hw_channel(send_guid_1);
    assert!(
        found_channel.is_some(),
        "Track should be found after remapping"
    );
    // Should return the new channel (hw_channel_2)
    assert_eq!(
        found_channel.unwrap(),
        send_idx_2 as usize,
        "find_hw_channel returns the remapped channel"
    );

    // Send another volume update - should go to new channel (hw_channel_2)
    mode.handle_msg_from_upstream(
        track::SendLevel {
            track_guid,
            send_index: send_idx_2,
            level: level_2,
        }
        .into(),
        &mut io_direct,
    );

    // Volume update should go to the new channel (hw_channel_2)
    assert_downstream_fader_abs_msg!(&to_v1m_rx, send_idx_2, level_2 as f64);

    // Send another pan update - should go to new channel (hw_channel_2)
    mode.handle_msg_from_upstream(
        track::SendPan {
            track_guid,
            send_index: send_idx_2,
            pan: pan_2,
        }
        .into(),
        &mut io_direct,
    );
    assert_downstream_encoder_ring_led_msg!(
        &to_v1m_rx,
        send_idx_2,
        v1m::map_to_encoder_ring(pan_2)
    );
}

#[test]
fn test_send_state_reflects_latest_value_new_send_replaces_old_send_at_index() {
    let track_guid = uuid::Uuid::new_v4();
    let (mut mode, _to_reaper_rx, to_v1m_rx, mut io_direct) = setup_track_sends_mode(track_guid);

    let send_guid_1 = uuid::Uuid::new_v4();
    let send_guid_2 = uuid::Uuid::new_v4();
    let send_idx_1 = 2;
    let level_1 = 0.5;
    let level_2 = 0.8;
    let pan_1 = 0.1;
    let pan_2 = 0.3;

    // Assign track to first hardware channel
    mode.handle_msg_from_upstream(
        track::SendIndex {
            track_guid,
            send_guid: send_guid_1,
            send_index: send_idx_1,
        }
        .into(),
        &mut io_direct,
    );
    assert_downstream_fader_abs_msg!(&to_v1m_rx, send_idx_1, 0 as f64);
    // TODO: what is the expected behavior of mute/solo/arm? Do they do anything?
    // assert_downstream_mute_led_msg!(&to_v1m_rx, send_idx_1, LEDState::Off);
    // assert_downstream_solo_led_msg!(&to_v1m_rx, send_idx_1, LEDState::Off);
    // assert_downstream_arm_led_msg!(&to_v1m_rx, send_idx_1, LEDState::Off);
    assert_downstream_encoder_ring_led_msg!(
        &to_v1m_rx,
        send_idx_1,
        v1m::ENCODER_CENTER_MODE_CENTER
    );
    // Update level
    mode.handle_msg_from_upstream(
        track::SendLevel {
            track_guid,
            send_index: send_idx_1,
            level: level_1,
        }
        .into(),
        &mut io_direct,
    );
    assert_downstream_fader_abs_msg!(&to_v1m_rx, send_idx_1, level_1 as f64);
    mode.handle_msg_from_upstream(
        track::SendPan {
            track_guid,
            send_index: send_idx_1,
            pan: pan_1,
        }
        .into(),
        &mut io_direct,
    );
    assert_downstream_encoder_ring_led_msg!(
        &to_v1m_rx,
        send_idx_1,
        v1m::map_to_encoder_ring(pan_1)
    );

    // Map new send to this index
    mode.handle_msg_from_upstream(
        track::SendIndex {
            track_guid,
            send_guid: send_guid_2,
            send_index: send_idx_1,
        }
        .into(),
        &mut io_direct,
    );
    assert_downstream_fader_abs_msg!(&to_v1m_rx, send_idx_1, 0 as f64);
    // TODO:
    // assert_downstream_mute_led_msg!(&to_v1m_rx, send_idx_1, LEDState::Off);
    // assert_downstream_solo_led_msg!(&to_v1m_rx, send_idx_1, LEDState::Off);
    // assert_downstream_arm_led_msg!(&to_v1m_rx, send_idx_1, LEDState::Off);
    assert_downstream_encoder_ring_led_msg!(
        &to_v1m_rx,
        send_idx_1,
        v1m::ENCODER_CENTER_MODE_CENTER
    );

    // Verify the old guid is unmapped
    assert!(
        mode.find_hw_channel(send_guid_1).is_none(),
        "Old send guid should be unmapped after new send is assigned to same index"
    );
    // Verify the send can be found via find_hw_channel
    let found_channel = mode.find_hw_channel(send_guid_2);
    assert!(
        found_channel.is_some(),
        "Track should be found after remapping"
    );
    // Should return the same hw channel index
    assert_eq!(
        found_channel.unwrap(),
        send_idx_1 as usize,
        "find_hw_channel returns the remapped channel"
    );

    // Send another volume update - should go to new channel (hw_channel_2)
    mode.handle_msg_from_upstream(
        track::SendLevel {
            track_guid,
            send_index: send_idx_1,
            level: level_2,
        }
        .into(),
        &mut io_direct,
    );

    // Volume update should go to the new channel (hw_channel_2)
    assert_downstream_fader_abs_msg!(&to_v1m_rx, send_idx_1, level_2 as f64);

    // Send another pan update - should go to new channel (hw_channel_2)
    mode.handle_msg_from_upstream(
        track::SendPan {
            track_guid,
            send_index: send_idx_1,
            pan: pan_2,
        }
        .into(),
        &mut io_direct,
    );
    assert_downstream_encoder_ring_led_msg!(
        &to_v1m_rx,
        send_idx_1,
        v1m::map_to_encoder_ring(pan_2)
    );
}

// TODO: might need this once we decide what buttons do in this mode
// #[test]
// fn test_multiple_button_state_updates_accumulate_correctly() {
//     let (mut mode, _to_reaper_rx, to_v1m_rx, mut io_direct) = setup_track_sends_mode();
//
//     let track_guid = uuid::Uuid::new_v4();
//     let hw_channel = 3;
//
//     // Assign track to hardware channel
//     mode.handle_msg_from_upstream(
//         track::ReaperTrackIndex {
//             track_guid,
//             track_index: Some(hw_channel + 1),
//         }
//         .into(),
//         &mut io_direct,
//     );
//     assert_downstream_default_track_mapping(&to_v1m_rx, hw_channel);
//
//     // Send mute state
//     mode.handle_msg_from_upstream(
//         track::Muted {
//             track_guid,
//             muted: true,
//         }
//         .into(),
//         &mut io_direct,
//     );
//     assert_downstream_mute_led_msg!(&to_v1m_rx, hw_channel, LEDState::On);
//
//     // Send solo state
//     mode.handle_msg_from_upstream(
//         track::Soloed {
//             track_guid,
//             soloed: true,
//         }
//         .into(),
//         &mut io_direct,
//     );
//     assert_downstream_solo_led_msg!(&to_v1m_rx, hw_channel, LEDState::On);
//
//     // Send armed state
//     mode.handle_msg_from_upstream(
//         track::Armed {
//             track_guid,
//             armed: true,
//         }
//         .into(),
//         &mut io_direct,
//     );
//     assert_downstream_arm_led_msg!(&to_v1m_rx, hw_channel, LEDState::On);
// }

// // ----------------------------------------------------------------------------
// // Upstream/Downstream Flow Tests
// // ----------------------------------------------------------------------------
//
// // TODO: may need once we decide what buttons do in this mode
// #[test]
// fn test_mute_button_sends_correct_upstream_and_downstream_messages() {
//     let (mut mode, to_reaper_rx, to_v1m_rx, mut io_direct) = setup_track_sends_mode();
//
//     let track_guid = uuid::Uuid::new_v4();
//     let hw_channel = 2;
//
//     // Assign track to hardware channel
//     mode.handle_msg_from_upstream(
//         track::ReaperTrackIndex {
//             track_guid,
//             track_index: Some(hw_channel + 1),
//         }
//         .into(),
//         &mut io_direct,
//     );
//     assert_downstream_default_track_mapping(&to_v1m_rx, hw_channel);
//
//     // Simulate mute button press
//     mode.handle_msg_from_downstream(
//         UpstreamMsg::MutePress(MutePress { idx: hw_channel }),
//         &mut io_direct,
//     );
//
//     // Should send mute message to Reaper (upstream)
//     assert_upstream_muted_track_msg!(&to_reaper_rx, &track_guid, true);
//
//     // Should send LED update to hardware (downstream)
//     assert_downstream_mute_led_msg!(&to_v1m_rx, hw_channel, LEDState::On);
// }
//
// #[test]
// fn test_solo_button_sends_correct_messages() {
//     let (mut mode, to_reaper_rx, to_v1m_rx, mut io_direct) = setup_track_sends_mode();
//
//     let track_guid = uuid::Uuid::new_v4();
//     let hw_channel = 4;
//
//     // Assign track to hardware channel
//     mode.handle_msg_from_upstream(
//         track::ReaperTrackIndex {
//             track_guid,
//             track_index: Some(hw_channel + 1),
//         }
//         .into(),
//         &mut io_direct,
//     );
//     assert_downstream_default_track_mapping(&to_v1m_rx, hw_channel);
//
//     // Simulate solo button press
//     mode.handle_msg_from_downstream(
//         UpstreamMsg::SoloPress(SoloPress { idx: hw_channel }),
//         &mut io_direct,
//     );
//
//     // Should send solo message to Reaper
//     assert_upstream_soloed_track_msg!(&to_reaper_rx, &track_guid, true);
//
//     // Should send LED update to hardware
//     assert_downstream_solo_led_msg!(&to_v1m_rx, hw_channel, LEDState::On);
// }
//
// #[test]
// fn test_arm_button_sends_correct_messages() {
//     let (mut mode, to_reaper_rx, to_v1m_rx, mut io_direct) = setup_track_sends_mode();
//
//     let track_guid = uuid::Uuid::new_v4();
//     let hw_channel = 0;
//
//     // Assign track to hardware channel
//     mode.handle_msg_from_upstream(
//         track::ReaperTrackIndex {
//             track_guid,
//             track_index: Some(hw_channel + 1),
//         }
//         .into(),
//         &mut io_direct,
//     );
//     assert_downstream_default_track_mapping(&to_v1m_rx, hw_channel);
//
//     // Simulate arm button press
//     mode.handle_msg_from_downstream(
//         UpstreamMsg::ArmPress(ArmPress { idx: hw_channel }),
//         &mut io_direct,
//     );
//
//     // Should send arm message to Reaper
//     assert_upstream_armed_track_msg!(&to_reaper_rx, &track_guid, true);
//
//     // Should send LED update to hardware
//     assert_downstream_arm_led_msg!(&to_v1m_rx, hw_channel, LEDState::On);
// }
//
//
// // ----------------------------------------------------------------------------
// // Mode Transition Tests
// // ----------------------------------------------------------------------------
//
#[test]
fn test_transition_to_reaperchannelstrip() {
    let track_guid = uuid::Uuid::new_v4();
    let (mut mode, _to_reaper_rx, _to_v1m_rx, mut io_direct) = setup_track_sends_mode(track_guid);

    let action = mode.handle_msg_from_downstream(UpstreamMsg::InputsPress, &mut io_direct);
    assert!(
        matches!(
            action,
            ModeAction::Transition(TransitionRequest::ToReaperChannelStrip {
                offset: 1,
                selected_track_guid: _
            })
        ),
        "Should transition to ReaperSends, got {:?}",
        action
    );
    let selected_track_guid = match action {
        ModeAction::Transition(TransitionRequest::ToReaperChannelStrip {
            offset: 1,
            selected_track_guid,
        }) => selected_track_guid,
        _ => panic!("Expected transition to ReaperSends"),
    };
    assert_eq!(
        selected_track_guid, track_guid,
        "Selected track GUID should match"
    );
}

#[test]
fn test_transition_to_reapervolpan() {
    let track_guid = uuid::Uuid::new_v4();
    let (mut mode, _to_reaper_rx, _to_v1m_rx, mut io_direct) = setup_track_sends_mode(track_guid);
    let action = mode.handle_msg_from_downstream(UpstreamMsg::GlobalPress, &mut io_direct);
    assert!(
        matches!(
            action,
            ModeAction::Transition(TransitionRequest::ToReaperVolumePan {
                offset: 1,
                selected_track_guid: _
            })
        ),
        "Should transition to ReaperSends, got {:?}",
        action
    );
    let selected_track_guid = match action {
        ModeAction::Transition(TransitionRequest::ToReaperVolumePan {
            offset: 1,
            selected_track_guid,
        }) => selected_track_guid,
        _ => panic!("Expected transition to ReaperSends"),
    };
    assert_eq!(
        selected_track_guid,
        Some(track_guid),
        "Selected track GUID should match"
    );
}

#[test]
fn test_requested_transition_to_self_is_noop() {
    let track_guid = uuid::Uuid::new_v4();
    let (mut mode, _to_reaper_rx, _to_v1m_rx, mut io_direct) = setup_track_sends_mode(track_guid);

    // Assert no transition if we send the message to transition to the current state
    let action = mode.handle_msg_from_downstream(UpstreamMsg::MIDITracksPress, &mut io_direct);
    assert!(
        matches!(action, ModeAction::None),
        "Message requesting transition to self should be no-op, got {:?}",
        action
    );
}

#[test]
fn test_selecting_currently_selected_is_noop() {
    let track_guid = uuid::Uuid::new_v4();
    let (mut mode, to_reaper_rx, to_v1m_rx, mut io_direct) = setup_track_sends_mode(track_guid);
    mode.handle_msg_from_upstream(
        track::Selected {
            track_guid,
            selected: true,
        }
        .into(),
        &mut io_direct,
    );
    // Assert selection of the same track from upstream is a no-op
    assert!(
        matches!(
            mode.handle_msg_from_upstream(
                track::Selected {
                    track_guid,
                    selected: true,
                }
                .into(),
                &mut io_direct
            ),
            ModeAction::None
        ),
        "Selecting the currently selected track from upstream should be a no-op"
    );
    check_no_message!(&to_reaper_rx, 100);
    check_no_message!(&to_v1m_rx, 100);
}

// TODO: we think this is a no-op but we're not sure
// #[test]
// fn test_selection_of_different_track_from_downstream() {
//     let (mut mode, _to_reaper_rx, _to_v1m_rx, mut io_direct) = setup_track_sends_mode();
//     let track_guid = uuid::Uuid::new_v4();
//     let track_idx = 0;
//     mode.handle_msg_from_upstream(
//         track::ReaperTrackIndex {
//             track_guid,
//             track_index: Some(track_idx + 1), // because reaper's counting starts at 1
//         }
//         .into(),
//         &mut io_direct,
//     );
//     mode.handle_msg_from_upstream(
//         track::Selected {
//             track_guid,
//             selected: true,
//         }
//         .into(),
//         &mut io_direct,
//     );
//     // Assert selection of a different track from downsteram
//     let track2_guid = uuid::Uuid::new_v4();
//     let track2_idx = 1;
//     mode.handle_msg_from_upstream(
//         track::ReaperTrackIndex {
//             track_guid: track2_guid,
//             track_index: Some(track2_idx + 1), // because reaper's counting starts at 1
//         }
//         .into(),
//         &mut io_direct,
//     );
//     mode.handle_msg_from_downstream(SelectPress { idx: track2_idx }.into(), &mut io_direct);
//     let action = mode.handle_msg_from_downstream(UpstreamMsg::MIDITracksPress, &mut io_direct);
//     assert!(
//         matches!(
//             action,
//             ModeAction::Transition(TransitionRequest::ToReaperSends {
//                 selected_track_guid: _
//             })
//         ),
//         "Should transition to ReaperSends, got {:?}",
//         action
//     );
//     let selected_track_guid = match action {
//         ModeAction::Transition(TransitionRequest::ToReaperSends {
//             selected_track_guid,
//         }) => selected_track_guid,
//         _ => panic!("Expected transition to ReaperSends"),
//     };
//     assert_eq!(
//         selected_track_guid, track2_guid,
//         "Selected track GUID should match"
//     );
// }
//
// // ----------------------------------------------------------------------------
// // Message Ordering Tests
// // ----------------------------------------------------------------------------
//
// #[test]
// fn test_downstream_messages_sent_in_correct_order() {
//     let (mut mode, _to_reaper_rx, to_v1m_rx, mut io_direct) = setup_track_sends_mode();
//
//     let track_guid = uuid::Uuid::new_v4();
//     let hw_channel = 1;
//
//     // Assign track
//     mode.handle_msg_from_upstream(
//         track::ReaperTrackIndex {
//             track_guid,
//             track_index: Some(hw_channel + 1),
//         }
//         .into(),
//         &mut io_direct,
//     );
//     assert_downstream_fader_abs_msg!(&to_v1m_rx, hw_channel, FADER_0DB as f64);
//     assert_downstream_mute_led_msg!(&to_v1m_rx, hw_channel, LEDState::Off);
//     assert_downstream_solo_led_msg!(&to_v1m_rx, hw_channel, LEDState::Off);
//     assert_downstream_arm_led_msg!(&to_v1m_rx, hw_channel, LEDState::Off);
//     assert_downstream_encoder_ring_led_msg!(&to_v1m_rx, hw_channel, 8);
//
//     // Send multiple messages in order
//     mode.handle_msg_from_upstream(
//         track::Volume {
//             track_guid,
//             volume: 0.5,
//         }
//         .into(),
//         &mut io_direct,
//     );
//
//     mode.handle_msg_from_upstream(
//         track::Pan {
//             track_guid,
//             pan: 0.3,
//         }
//         .into(),
//         &mut io_direct,
//     );
//
//     mode.handle_msg_from_upstream(
//         track::Muted {
//             track_guid,
//             muted: true,
//         }
//         .into(),
//         &mut io_direct,
//     );
//
//     // Verify messages received in order
//     let msg = to_v1m_rx.recv_timeout(Duration::from_millis(100));
//     assert!(
//         matches!(msg, Ok(DownstreamMsg::ChannelFader(_))),
//         " First should be fader, got {:?}",
//         msg
//     );
//
//     let msg = to_v1m_rx.recv_timeout(Duration::from_millis(100));
//     assert!(
//         matches!(msg, Ok(DownstreamMsg::EncoderRingLED(_))),
//         "Second should be encoder, got {:?}",
//         msg
//     );
//
//     let msg = to_v1m_rx.recv_timeout(Duration::from_millis(100));
//     assert!(
//         matches!(msg, Ok(DownstreamMsg::MuteLED(_))),
//         "Third should be mute LED, got {:?}",
//         msg
//     );
// }
//
// #[test]
// fn test_upstream_messages_processed_in_correct_order() {
//     let (mut mode, to_reaper_rx, to_v1m_rx, mut io_direct) = setup_track_sends_mode();
//
//     let track_guid = uuid::Uuid::new_v4();
//     let hw_channel = 3;
//
//     // Assign track
//     mode.handle_msg_from_upstream(
//         track::ReaperTrackIndex {
//             track_guid,
//             track_index: Some(hw_channel + 1),
//         }
//         .into(),
//         &mut io_direct,
//     );
//     assert_downstream_fader_abs_msg!(&to_v1m_rx, hw_channel, FADER_0DB as f64);
//     assert_downstream_mute_led_msg!(&to_v1m_rx, hw_channel, LEDState::Off);
//     assert_downstream_solo_led_msg!(&to_v1m_rx, hw_channel, LEDState::Off);
//     assert_downstream_arm_led_msg!(&to_v1m_rx, hw_channel, LEDState::Off);
//     // assert_downstream_encoder_ring_led_msg!(&_to_v1m_rx, hw_channel, 0.5);
//
//     // Send multiple upstream messages in order
//     mode.handle_msg_from_downstream(
//         UpstreamMsg::ChannelFader(ChannelFaderMsg {
//             idx: hw_channel,
//             value: 0.6,
//         }),
//         &mut io_direct,
//     );
//
//     mode.handle_msg_from_downstream(
//         UpstreamMsg::MutePress(MutePress { idx: hw_channel }),
//         &mut io_direct,
//     );
//
//     // Verify messages processed in order (volume then mute)
//     let msg1 = to_reaper_rx.recv_timeout(Duration::from_millis(100));
//     assert!(msg1.is_ok(), "Should receive first message");
//     assert!(
//         matches!(msg1, Ok(TrackMsg::Volume(_))),
//         "First should be volume"
//     );
//
//     let msg2 = to_reaper_rx.recv_timeout(Duration::from_millis(100));
//     assert!(msg2.is_ok(), "Should receive second message");
//     assert!(
//         matches!(msg2, Ok(TrackMsg::Muted(_))),
//         "Second should be muted"
//     );
// }
//
#[test]
fn test_complex_multi_send_integration() {
    let track_guid = uuid::Uuid::new_v4();
    let (mut mode, to_reaper_rx, to_v1m_rx, mut io_direct) = setup_track_sends_mode(track_guid);

    let send1_guid = uuid::Uuid::new_v4();
    let send2_guid = uuid::Uuid::new_v4();
    let send3_guid = uuid::Uuid::new_v4();

    // === PHASE 1: Map multiple sends ===
    mode.handle_msg_from_upstream(
        track::SendIndex {
            track_guid,
            send_guid: send1_guid,
            send_index: 0,
        }
        .into(),
        &mut io_direct,
    );
    mode.handle_msg_from_upstream(
        track::SendIndex {
            track_guid,
            send_guid: send2_guid,
            send_index: 1,
        }
        .into(),
        &mut io_direct,
    );
    mode.handle_msg_from_upstream(
        track::SendIndex {
            track_guid,
            send_guid: send3_guid,
            send_index: 2,
        }
        .into(),
        &mut io_direct,
    );
    drain(&to_v1m_rx); // Clear any previous messages

    // === PHASE 2: Send levels to all sends ===
    mode.handle_msg_from_upstream(
        track::SendLevel {
            track_guid,
            send_index: 0,
            level: 0.3,
        }
        .into(),
        &mut io_direct,
    );
    assert_downstream_fader_abs_msg!(&to_v1m_rx, 0, 0.3);

    mode.handle_msg_from_upstream(
        track::SendLevel {
            track_guid,
            send_index: 1,
            level: 0.6,
        }
        .into(),
        &mut io_direct,
    );
    assert_downstream_fader_abs_msg!(&to_v1m_rx, 1, 0.6);

    mode.handle_msg_from_upstream(
        track::SendLevel {
            track_guid,
            send_index: 2,
            level: 0.9,
        }
        .into(),
        &mut io_direct,
    );
    assert_downstream_fader_abs_msg!(&to_v1m_rx, 2, 0.9);

    // === PHASE 3: Hardware interaction on multiple channels ===
    // Move fader on channel 0
    mode.handle_msg_from_downstream(
        UpstreamMsg::ChannelFader(ChannelFaderMsg { idx: 0, value: 0.4 }),
        &mut io_direct,
    );
    assert_upstream_send_level_track_msg!(&to_reaper_rx, &send1_guid, 0, 0.4);

    // Move fader on channel 1
    mode.handle_msg_from_downstream(
        UpstreamMsg::ChannelFader(ChannelFaderMsg { idx: 1, value: 0.7 }),
        &mut io_direct,
    );
    assert_upstream_send_level_track_msg!(&to_reaper_rx, &send2_guid, 1, 0.7);

    // Move fader on channel 2
    mode.handle_msg_from_downstream(
        UpstreamMsg::ChannelFader(ChannelFaderMsg {
            idx: 2,
            value: 0.95,
        }),
        &mut io_direct,
    );
    assert_upstream_send_level_track_msg!(&to_reaper_rx, &send3_guid, 2, 0.95);

    // === PHASE 4: Update send levels from Reaper ===
    mode.handle_msg_from_upstream(
        track::SendLevel {
            track_guid,
            send_index: 0,
            level: 0.5,
        }
        .into(),
        &mut io_direct,
    );
    assert_downstream_fader_abs_msg!(&to_v1m_rx, 0, 0.5);

    mode.handle_msg_from_upstream(
        track::SendLevel {
            track_guid,
            send_index: 1,
            level: 0.8,
        }
        .into(),
        &mut io_direct,
    );
    assert_downstream_fader_abs_msg!(&to_v1m_rx, 1, 0.8);

    // Verify no additional messages
    check_no_message!(&to_v1m_rx, 1);
    check_no_message!(&to_reaper_rx, 1);
}

#[test]
fn test_multiple_tracks_and_switching_send_mapping() {
    let selected_track_guid = uuid::Uuid::new_v4();
    let (mut mode, _to_reaper_rx, to_v1m_rx, mut io_direct) =
        setup_track_sends_mode(selected_track_guid);

    let track1_send1 = uuid::Uuid::new_v4();
    let track1_send2 = uuid::Uuid::new_v4();
    let track2_send1 = uuid::Uuid::new_v4();

    // Set up sends for track 1
    mode.handle_msg_from_upstream(
        track::SendIndex {
            track_guid: selected_track_guid,
            send_guid: track1_send1,
            send_index: 0,
        }
        .into(),
        &mut io_direct,
    );
    mode.handle_msg_from_upstream(
        track::SendIndex {
            track_guid: selected_track_guid,
            send_guid: track1_send2,
            send_index: 1,
        }
        .into(),
        &mut io_direct,
    );
    drain(&to_v1m_rx); // Clear any previous messages

    // Send levels for track 1 sends
    mode.handle_msg_from_upstream(
        track::SendLevel {
            track_guid: selected_track_guid,
            send_index: 0,
            level: 0.3,
        }
        .into(),
        &mut io_direct,
    );
    assert_downstream_fader_abs_msg!(&to_v1m_rx, 0, 0.3);

    mode.handle_msg_from_upstream(
        track::SendLevel {
            track_guid: selected_track_guid,
            send_index: 1,
            level: 0.6,
        }
        .into(),
        &mut io_direct,
    );
    assert_downstream_fader_abs_msg!(&to_v1m_rx, 1, 0.6);

    // Simulate switching to track 2 (different sends get mapped to same channels)
    // TODO: this comment doesn't make sense to me...
    mode.handle_msg_from_upstream(
        track::SendIndex {
            track_guid: selected_track_guid,
            send_guid: track2_send1,
            send_index: 0,
        }
        .into(),
        &mut io_direct,
    );
    drain(&to_v1m_rx); // Clear messages from track 1

    // Send level for track 2 send 1
    mode.handle_msg_from_upstream(
        track::SendLevel {
            track_guid: selected_track_guid,
            send_index: 0,
            level: 0.9,
        }
        .into(),
        &mut io_direct,
    );
    assert_downstream_fader_abs_msg!(&to_v1m_rx, 0, 0.9);

    // Switch back to track 1 by reassigning track1_send1
    mode.handle_msg_from_upstream(
        track::SendIndex {
            track_guid: selected_track_guid,
            send_guid: track1_send1,
            send_index: 0,
        }
        .into(),
        &mut io_direct,
    );
    drain(&to_v1m_rx); // Clear messages from track 2

    // Send level should update correctly
    mode.handle_msg_from_upstream(
        track::SendLevel {
            track_guid: selected_track_guid,
            send_index: 0,
            level: 0.4,
        }
        .into(),
        &mut io_direct,
    );
    assert_downstream_fader_abs_msg!(&to_v1m_rx, 0, 0.4);
}

#[test]
fn test_smoke_and_churn() {
    let selected_track_guid = uuid::Uuid::new_v4();
    let (mut mode, to_reaper_rx, to_v1m_rx, mut io_direct) =
        setup_track_sends_mode(selected_track_guid);

    // === SCENARIO 1: Set up multiple sends
    let send_1_guid = uuid::Uuid::new_v4();
    let send_2_guid = uuid::Uuid::new_v4();
    let send_3_guid = uuid::Uuid::new_v4();

    mode.handle_msg_from_upstream(
        track::SendIndex {
            track_guid: selected_track_guid,
            send_guid: send_1_guid,
            send_index: 0,
        }
        .into(),
        &mut io_direct,
    );
    mode.handle_msg_from_upstream(
        track::SendIndex {
            track_guid: selected_track_guid,
            send_guid: send_2_guid,
            send_index: 1,
        }
        .into(),
        &mut io_direct,
    );
    mode.handle_msg_from_upstream(
        track::SendIndex {
            track_guid: selected_track_guid,
            send_guid: send_3_guid,
            send_index: 2,
        }
        .into(),
        &mut io_direct,
    );
    drain(&to_v1m_rx); // Clear any previous messages

    // Set initial levels
    for (idx, level) in [(0, 0.3), (1, 0.5), (2, 0.7)] {
        mode.handle_msg_from_upstream(
            track::SendLevel {
                track_guid: selected_track_guid,
                send_index: idx,
                level,
            }
            .into(),
            &mut io_direct,
        );
        assert_downstream_fader_abs_msg!(&to_v1m_rx, idx, level as f64);
    }

    // === SCENARIO 2: User adjusts faders on hardware ===
    for (idx, level) in [(0, 0.4), (1, 0.6), (2, 0.8)] {
        mode.handle_msg_from_downstream(
            UpstreamMsg::ChannelFader(ChannelFaderMsg { idx, value: level }),
            &mut io_direct,
        );
    }

    // Verify upstream messages sent to Reaper
    assert_upstream_send_level_track_msg!(&to_reaper_rx, &track_a_send_1, 0, 0.4);
    assert_upstream_send_level_track_msg!(&to_reaper_rx, &track_a_send_2, 1, 0.6);
    assert_upstream_send_level_track_msg!(&to_reaper_rx, &track_a_send_3, 2, 0.8);

    // === SCENARIO 3: Switch both sends to new guids
    let send_4_guid = uuid::Uuid::new_v4();
    let send_5_guid = uuid::Uuid::new_v4();

    mode.handle_msg_from_upstream(
        track::SendIndex {
            track_guid: selected_track_guid,
            send_guid: send_4_guid,
            send_index: 0,
        }
        .into(),
        &mut io_direct,
    );
    mode.handle_msg_from_upstream(
        track::SendIndex {
            track_guid: selected_track_guid,
            send_guid: send_5_guid,
            send_index: 1,
        }
        .into(),
        &mut io_direct,
    );
    drain(&to_v1m_rx); // Clear messages from Track A

    for (idx, level) in [(0, 0.2), (1, 0.9)] {
        mode.handle_msg_from_upstream(
            track::SendLevel {
                track_guid: selected_track_guid,
                send_index: idx,
                level,
            }
            .into(),
            &mut io_direct,
        );
        assert_downstream_fader_abs_msg!(&to_v1m_rx, idx, level as f64);
    }

    // === SCENARIO 4: send 1 to different channel ===
    mode.handle_msg_from_upstream(
        track::SendIndex {
            track_guid: selected_track_guid,
            send_guid: send_4_guid,
            send_index: 5,
        }
        .into(),
        &mut io_direct,
    );
    drain(&to_v1m_rx); // Clear previous messages

    mode.handle_msg_from_upstream(
        track::SendLevel {
            track_guid: selected_track_guid,
            send_index: 5,
            level: 0.95,
        }
        .into(),
        &mut io_direct,
    );
    assert_downstream_fader_abs_msg!(&to_v1m_rx, 5, 0.95);

    // === SCENARIO 5: switch mapping again
    mode.handle_msg_from_upstream(
        track::SendIndex {
            track_guid: selected_track_guid,
            send_guid: send_4_guid,
            send_index: 0,
        }
        .into(),
        &mut io_direct,
    );
    drain(&to_v1m_rx); // Clear messages from Track B

    mode.handle_msg_from_upstream(
        track::SendLevel {
            track_guid: selected_track_guid,
            send_index: 0,
            level: 0.55,
        }
        .into(),
        &mut io_direct,
    );
    assert_downstream_fader_abs_msg!(&to_v1m_rx, 0, 0.55);

    // Hardware interaction should work
    mode.handle_msg_from_downstream(
        UpstreamMsg::ChannelFader(ChannelFaderMsg {
            idx: 0,
            value: 0.65,
        }),
        &mut io_direct,
    );
    assert_upstream_send_level_track_msg!(&to_reaper_rx, &track_a_send_1, 0, 0.65);

    // === Final verification: No unexpected messages ===
    check_no_message!(&to_v1m_rx, 1);
    check_no_message!(&to_reaper_rx, 1);
}
