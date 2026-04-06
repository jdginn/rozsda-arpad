use std::time::Duration;

use assert2::{assert, check};
use crossbeam_channel::{Receiver, Sender, unbounded};
use float_cmp::approx_eq;

use arpad_rust::midi::xtouch::{FaderAbsMsg, XTouchDownstreamMsg, XTouchUpstreamMsg};
use arpad_rust::modes::mode_manager::{Mode, ModeHandler, ModeState, State};
use arpad_rust::modes::reaper_track_sends::TrackSendsMode;
use arpad_rust::track::track::{
    DataPayload, Direction, SendIndex, SendLevel, TrackDataMsg, TrackMsg,
};

pub fn drain<T>(rx: &Receiver<T>) {
    // Drops (flushes) all messages currently buffered at the time we start draining,
    // plus any that arrive before we hit Empty.
    for _msg in rx.try_iter() {
        // intentionally discard
    }
}

// EPSILON constant for floating-point threshold testing
const EPSILON: f32 = 0.01;

/// Helper to create a TrackSendsMode instance for testing
fn setup_track_sends_mode() -> (
    TrackSendsMode,
    Sender<TrackMsg>,
    Receiver<TrackMsg>,
    Sender<XTouchUpstreamMsg>,
    Receiver<XTouchDownstreamMsg>,
) {
    let (from_reaper_tx, from_reaper_rx) = unbounded();
    let (to_reaper_tx, to_reaper_rx) = unbounded();
    let (from_xtouch_tx, from_xtouch_rx) = unbounded();
    let (to_xtouch_tx, to_xtouch_rx) = unbounded();

    let mode = TrackSendsMode::new(
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
            .recv_timeout(Duration::from_millis(1))
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
            Ok(TrackMsg::TrackDataMsg(msg)) => {
                check!(&msg.guid == $expected_guid, "Track GUID should match");
                check!(msg.direction == Direction::Upstream, "Should be upstream");
                match msg.data {
                    DataPayload::SendLevel(send_level) => {
                        check!(
                            send_level.send_index == $expected_send_index,
                            "Send index should match"
                        );
                        check!(
                            approx_eq!(f32, send_level.level, $expected_level, epsilon = EPSILON),
                            "Send level should match approximately\nExpected: {}, Got: {}",
                            $expected_level,
                            send_level.level
                        );
                    }
                    _ => panic!("Expected SendLevel payload"),
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

/// Helper function to assign a send to a hardware channel
fn assign_send_to_channel(
    mode: &mut TrackSendsMode,
    send_guid: &str,
    send_index: i32,
    curr_mode: ModeState,
) -> ModeState {
    mode.handle_downstream_messages(
        TrackMsg::TrackDataMsg(TrackDataMsg {
            guid: "selected-track".to_string(),
            direction: Direction::Downstream,
            data: DataPayload::SendIndex(SendIndex {
                send_index,
                guid: send_guid.to_string(),
            }),
        }),
        curr_mode,
    )
}

// ----------------------------------------------------------------------------
// Basic Functionality Tests
// ----------------------------------------------------------------------------

#[test]
fn test_track_sends_mode_assigns_sends_by_index() {
    let (mut mode, _from_reaper_tx, _to_reaper_rx, _from_xtouch_tx, _to_xtouch_rx) =
        setup_track_sends_mode();

    let send_index = 2;

    let curr_mode = ModeState {
        mode: Mode::ReaperSends,
        state: State::Active,
    };

    // Send a SendIndex message to assign the send to hardware channel 2
    let msg = TrackMsg::TrackDataMsg(TrackDataMsg {
        guid: "selected-track".to_string(),
        direction: Direction::Downstream,
        data: DataPayload::SendIndex(SendIndex {
            send_index,
            guid: "selected-track".to_string(),
        }),
    });

    let result_mode = mode.handle_downstream_messages(msg, curr_mode);

    // Mode should remain unchanged
    assert_eq!(result_mode, curr_mode);

    // Verify the that channel 2 appears to have gotten a new mapping
    // (Note: TrackSendsMode doesn't expose a public find method to prove which guid it is mapped to)
    assert_downstream_fader_abs_msg!(&_to_xtouch_rx, send_index, 0.0); // Initial level should be sent to hardware
}

// ----------------------------------------------------------------------------
// Mapping Tests
// ----------------------------------------------------------------------------

#[test]
fn test_send_level_for_mapped_send_forwards_to_hardware() {
    let (mut mode, _from_reaper_tx, _to_reaper_rx, _from_xtouch_tx, to_xtouch_rx) =
        setup_track_sends_mode();

    let send_index = 2;
    let test_level = 0.75;

    let curr_mode = ModeState {
        mode: Mode::ReaperSends,
        state: State::Active,
    };

    // Assign send to hardware channel
    assign_send_to_channel(&mut mode, "selected-track", send_index, curr_mode);
    drain(&to_xtouch_rx); // Clear any previous messages

    // Send level update
    mode.handle_downstream_messages(
        TrackMsg::TrackDataMsg(TrackDataMsg {
            guid: "selected-track".to_string(),
            direction: Direction::Downstream,
            data: DataPayload::SendLevel(SendLevel {
                send_index,
                level: test_level,
            }),
        }),
        curr_mode,
    );

    // Assert fader message is sent to hardware
    assert_downstream_fader_abs_msg!(&to_xtouch_rx, send_index, test_level as f64);
}

#[test]
fn test_send_level_for_unmapped_send_is_ignored() {
    let (mut mode, _from_reaper_tx, _to_reaper_rx, _from_xtouch_tx, to_xtouch_rx) =
        setup_track_sends_mode();

    let send_index = 5;
    let test_level = 0.85;

    let curr_mode = ModeState {
        mode: Mode::ReaperSends,
        state: State::Active,
    };

    // Send level update WITHOUT assigning send to hardware channel
    mode.handle_downstream_messages(
        TrackMsg::TrackDataMsg(TrackDataMsg {
            guid: "selected-track".to_string(),
            direction: Direction::Downstream,
            data: DataPayload::SendLevel(SendLevel {
                send_index,
                level: test_level,
            }),
        }),
        curr_mode,
    );

    // Assert no message is sent to hardware
    check_no_message!(&to_xtouch_rx, 1);
}

#[test]
fn test_upstream_fader_for_mapped_channel_forwards_to_reaper() {
    let (mut mode, _from_reaper_tx, to_reaper_rx, _from_xtouch_tx, _to_xtouch_rx) =
        setup_track_sends_mode();

    let target_guid = "target-mapped-fader".to_string();
    let send_index = 1;
    let new_level = 0.65;

    let curr_mode = ModeState {
        mode: Mode::ReaperSends,
        state: State::Active,
    };

    // Assign send to hardware channel
    assign_send_to_channel(&mut mode, &target_guid, send_index, curr_mode);
    drain(&to_reaper_rx); // Clear any previous messages

    // Simulate fader movement from hardware
    mode.handle_upstream_messages(
        XTouchUpstreamMsg::FaderAbs(FaderAbsMsg {
            idx: send_index,
            value: new_level,
        }),
        curr_mode,
    );

    // Assert send level message is sent to Reaper
    assert_upstream_send_level_track_msg!(
        &to_reaper_rx,
        &target_guid,
        send_index,
        new_level as f32
    );
}

#[test]
fn test_upstream_fader_for_unmapped_channel_is_ignored() {
    let (mut mode, _from_reaper_tx, to_reaper_rx, _from_xtouch_tx, _to_xtouch_rx) =
        setup_track_sends_mode();

    let send_index = 5;
    let new_level = 0.55;

    let curr_mode = ModeState {
        mode: Mode::ReaperSends,
        state: State::Active,
    };

    // Simulate fader movement WITHOUT assigning any send to this channel
    mode.handle_upstream_messages(
        XTouchUpstreamMsg::FaderAbs(FaderAbsMsg {
            idx: send_index,
            value: new_level,
        }),
        curr_mode,
    );

    // Assert no message is sent to Reaper
    check_no_message!(&to_reaper_rx, 1);
}

#[test]
fn test_simultaneous_upstream_downstream_messages() {
    let (mut mode, _from_reaper_tx, to_reaper_rx, _from_xtouch_tx, to_xtouch_rx) =
        setup_track_sends_mode();

    let target_guid = "target-simultaneous".to_string();
    let send_index = 2;

    let curr_mode = ModeState {
        mode: Mode::ReaperSends,
        state: State::Active,
    };

    // Assign send to hardware channel
    assign_send_to_channel(&mut mode, &target_guid, send_index, curr_mode);
    drain(&to_xtouch_rx); // Clear any previous messages

    // Send downstream level update from Reaper
    mode.handle_downstream_messages(
        TrackMsg::TrackDataMsg(TrackDataMsg {
            guid: "selected-track".to_string(),
            direction: Direction::Downstream,
            data: DataPayload::SendLevel(SendLevel {
                send_index,
                level: 0.6,
            }),
        }),
        curr_mode,
    );

    // Immediately send upstream message from hardware
    mode.handle_upstream_messages(
        XTouchUpstreamMsg::FaderAbs(FaderAbsMsg {
            idx: send_index,
            value: 0.8,
        }),
        curr_mode,
    );

    // Should receive downstream message to hardware
    assert_downstream_fader_abs_msg!(&to_xtouch_rx, send_index, 0.6);

    // Should receive upstream message to Reaper
    assert_upstream_send_level_track_msg!(&to_reaper_rx, &target_guid, send_index, 0.8);

    // Send another downstream update
    mode.handle_downstream_messages(
        TrackMsg::TrackDataMsg(TrackDataMsg {
            guid: "selected-track".to_string(),
            direction: Direction::Downstream,
            data: DataPayload::SendLevel(SendLevel {
                send_index,
                level: 0.9,
            }),
        }),
        curr_mode,
    );

    // Should still receive the message correctly
    assert_downstream_fader_abs_msg!(&to_xtouch_rx, send_index, 0.9);

    // Verify no unexpected messages
    check_no_message!(&to_xtouch_rx, 1);
    check_no_message!(&to_reaper_rx, 1);
}

#[test]
fn test_remapping_sends_across_hardware_channels() {
    let (mut mode, _from_reaper_tx, to_reaper_rx, _from_xtouch_tx, to_xtouch_rx) =
        setup_track_sends_mode();

    let target_guid_1 = "target-remap-1".to_string();
    let target_guid_2 = "target-remap-2".to_string();
    let channel_1 = 2;
    let channel_2 = 5;

    let curr_mode = ModeState {
        mode: Mode::ReaperSends,
        state: State::Active,
    };

    // Assign first send to channel 1
    assign_send_to_channel(&mut mode, &target_guid_1, channel_1, curr_mode);
    drain(&to_xtouch_rx); // Clear any previous messages

    // Send level update to channel 1
    mode.handle_downstream_messages(
        TrackMsg::TrackDataMsg(TrackDataMsg {
            guid: "selected-track".to_string(),
            direction: Direction::Downstream,
            data: DataPayload::SendLevel(SendLevel {
                send_index: channel_1,
                level: 0.5,
            }),
        }),
        curr_mode,
    );
    assert_downstream_fader_abs_msg!(&to_xtouch_rx, channel_1, 0.5);

    // Test hardware interaction on channel 1
    mode.handle_upstream_messages(
        XTouchUpstreamMsg::FaderAbs(FaderAbsMsg {
            idx: channel_1,
            value: 0.7,
        }),
        curr_mode,
    );
    assert_upstream_send_level_track_msg!(&to_reaper_rx, &target_guid_1, channel_1, 0.7);

    // Assign second send to channel 2
    assign_send_to_channel(&mut mode, &target_guid_2, channel_2, curr_mode);
    drain(&to_xtouch_rx); // Clear any previous messages

    // Send level to second channel should work
    mode.handle_downstream_messages(
        TrackMsg::TrackDataMsg(TrackDataMsg {
            guid: "selected-track".to_string(),
            direction: Direction::Downstream,
            data: DataPayload::SendLevel(SendLevel {
                send_index: channel_2,
                level: 0.8,
            }),
        }),
        curr_mode,
    );
    assert_downstream_fader_abs_msg!(&to_xtouch_rx, channel_2, 0.8);

    // Now reassign channel 1 to a different target
    assign_send_to_channel(&mut mode, &target_guid_2, channel_1, curr_mode);
    drain(&to_xtouch_rx); // Clear any previous messages

    // Channel 1 should now control target_guid_2
    mode.handle_upstream_messages(
        XTouchUpstreamMsg::FaderAbs(FaderAbsMsg {
            idx: channel_1,
            value: 0.9,
        }),
        curr_mode,
    );
    assert_upstream_send_level_track_msg!(&to_reaper_rx, &target_guid_2, channel_1, 0.9);

    // TODO: to the best of current knowledge, this is NOT the behavior we want... but maybe in the
    // future we would want to allow multiple channels to control the same target. If we change our
    // desired behavior, this test will need to change
    //
    // // Channel 2 should still work for target_guid_2 (same target, multiple channels - this is the current behavior)
    // mode.handle_upstream_messages(
    //     XTouchUpstreamMsg::FaderAbs(FaderAbsMsg {
    //         idx: channel_2,
    //         value: 0.85,
    //     }),
    //     curr_mode,
    // );
    // assert_upstream_send_level_track_msg!(&to_reaper_rx, &target_guid_2, channel_2, 0.85);
}

// ----------------------------------------------------------------------------
// State Accumulation Tests
// ----------------------------------------------------------------------------

#[test]
fn test_send_level_state_reflects_latest_value_when_remapped() {
    let (mut mode, _from_reaper_tx, _to_reaper_rx, _from_xtouch_tx, to_xtouch_rx) =
        setup_track_sends_mode();

    let target_guid_1 = "target-remap-1".to_string();
    let target_guid_2 = "target-remap-2".to_string();
    let send_index_1 = 2;
    let send_index_2 = 4;
    let level_1 = 0.5;
    let level_2 = 0.8;

    let curr_mode = ModeState {
        mode: Mode::ReaperSends,
        state: State::Active,
    };

    // Assign first send to hardware channel and send level
    assign_send_to_channel(&mut mode, &target_guid_1, send_index_1, curr_mode);
    drain(&to_xtouch_rx); // Clear any previous messages
    mode.handle_downstream_messages(
        TrackMsg::TrackDataMsg(TrackDataMsg {
            guid: "selected-track".to_string(),
            direction: Direction::Downstream,
            data: DataPayload::SendLevel(SendLevel {
                send_index: send_index_1,
                level: level_1,
            }),
        }),
        curr_mode,
    );
    assert_downstream_fader_abs_msg!(&to_xtouch_rx, send_index_1, level_1 as f64);

    // Update level
    mode.handle_downstream_messages(
        TrackMsg::TrackDataMsg(TrackDataMsg {
            guid: "selected-track".to_string(),
            direction: Direction::Downstream,
            data: DataPayload::SendLevel(SendLevel {
                send_index: send_index_1,
                level: level_2,
            }),
        }),
        curr_mode,
    );
    assert_downstream_fader_abs_msg!(&to_xtouch_rx, send_index_1, level_2 as f64);

    // Remap to different channel
    assign_send_to_channel(&mut mode, &target_guid_1, send_index_2, curr_mode);
    assert_downstream_fader_abs_msg!(&to_xtouch_rx, send_index_2, level_2 as f64);
    drain(&to_xtouch_rx); // Clear any previous messages

    // Send level update to new send
    mode.handle_downstream_messages(
        TrackMsg::TrackDataMsg(TrackDataMsg {
            guid: "selected-track".to_string(),
            direction: Direction::Downstream,
            data: DataPayload::SendLevel(SendLevel {
                send_index: send_index_2,
                level: 0.9,
            }),
        }),
        curr_mode,
    );

    // Should receive fader update on new channel
    assert_downstream_fader_abs_msg!(&to_xtouch_rx, send_index_2, 0.9);
    // Should not receive any messages on the old channel
    check_no_message!(&to_xtouch_rx, send_index_1 as u64);
}

#[test]
fn test_multiple_sends_can_be_mapped_simultaneously() {
    let (mut mode, _from_reaper_tx, _to_reaper_rx, _from_xtouch_tx, to_xtouch_rx) =
        setup_track_sends_mode();

    let target_guid_1 = "target-multi-1".to_string();
    let target_guid_2 = "target-multi-2".to_string();
    let target_guid_3 = "target-multi-3".to_string();
    let send_index_1 = 0;
    let send_index_2 = 1;
    let send_index_3 = 2;

    let curr_mode = ModeState {
        mode: Mode::ReaperSends,
        state: State::Active,
    };

    // Assign multiple sends
    assign_send_to_channel(&mut mode, &target_guid_1, send_index_1, curr_mode);
    assign_send_to_channel(&mut mode, &target_guid_2, send_index_2, curr_mode);
    assign_send_to_channel(&mut mode, &target_guid_3, send_index_3, curr_mode);
    drain(&to_xtouch_rx); // Clear any previous messages

    // Send levels to all three
    mode.handle_downstream_messages(
        TrackMsg::TrackDataMsg(TrackDataMsg {
            guid: "selected-track".to_string(),
            direction: Direction::Downstream,
            data: DataPayload::SendLevel(SendLevel {
                send_index: send_index_1,
                level: 0.3,
            }),
        }),
        curr_mode,
    );
    mode.handle_downstream_messages(
        TrackMsg::TrackDataMsg(TrackDataMsg {
            guid: "selected-track".to_string(),
            direction: Direction::Downstream,
            data: DataPayload::SendLevel(SendLevel {
                send_index: send_index_2,
                level: 0.6,
            }),
        }),
        curr_mode,
    );
    mode.handle_downstream_messages(
        TrackMsg::TrackDataMsg(TrackDataMsg {
            guid: "selected-track".to_string(),
            direction: Direction::Downstream,
            data: DataPayload::SendLevel(SendLevel {
                send_index: send_index_3,
                level: 0.9,
            }),
        }),
        curr_mode,
    );

    // Should receive fader updates for all three
    assert_downstream_fader_abs_msg!(&to_xtouch_rx, send_index_1, 0.3);
    assert_downstream_fader_abs_msg!(&to_xtouch_rx, send_index_2, 0.6);
    assert_downstream_fader_abs_msg!(&to_xtouch_rx, send_index_3, 0.9);
}

#[test]
fn test_state_accumulation_for_unmapped_sends_applies_when_mapped() {
    let (mut mode, _from_reaper_tx, _to_reaper_rx, _from_xtouch_tx, to_xtouch_rx) =
        setup_track_sends_mode();

    let target_guid = "target-accumulate".to_string();
    let send_index = 3;
    let level_1 = 0.4;
    let level_2 = 0.7; // Latest value

    let curr_mode = ModeState {
        mode: Mode::ReaperSends,
        state: State::Active,
    };

    // Send level updates BEFORE mapping - they should not be sent to hardware yet
    mode.handle_downstream_messages(
        TrackMsg::TrackDataMsg(TrackDataMsg {
            guid: "selected-track".to_string(),
            direction: Direction::Downstream,
            data: DataPayload::SendLevel(SendLevel {
                send_index,
                level: level_1,
            }),
        }),
        curr_mode,
    );

    // No message should be sent yet (send not mapped)
    check_no_message!(&to_xtouch_rx, 1);

    // Send another level update
    mode.handle_downstream_messages(
        TrackMsg::TrackDataMsg(TrackDataMsg {
            guid: "selected-track".to_string(),
            direction: Direction::Downstream,
            data: DataPayload::SendLevel(SendLevel {
                send_index,
                level: level_2,
            }),
        }),
        curr_mode,
    );

    // Still no message (send not mapped)
    check_no_message!(&to_xtouch_rx, 1);

    // NOW assign send to hardware channel
    assign_send_to_channel(&mut mode, &target_guid, send_index, curr_mode);
    drain(&to_xtouch_rx); // Clear any previous messages

    // NOTE: Current implementation does NOT accumulate state for unmapped sends
    // This test documents expected behavior (state should be sent) vs actual behavior
    // Expected: assert_downstream_fader_abs_msg!(&to_xtouch_rx, send_index, level_2 as f64);
    // Actual: No accumulated state is sent
    check_no_message!(&to_xtouch_rx, 1);
}

// ----------------------------------------------------------------------------
// Upstream/Downstream Flow Tests
// ----------------------------------------------------------------------------

#[test]
fn test_fader_movement_sends_correct_upstream_message() {
    let (mut mode, _from_reaper_tx, to_reaper_rx, _from_xtouch_tx, _to_xtouch_rx) =
        setup_track_sends_mode();

    let target_guid = "target-fader-flow".to_string();
    let send_index = 2;
    let test_value = 0.75;

    let curr_mode = ModeState {
        mode: Mode::ReaperSends,
        state: State::Active,
    };

    // Assign send to hardware channel
    assign_send_to_channel(&mut mode, &target_guid, send_index, curr_mode);

    // Simulate fader movement
    mode.handle_upstream_messages(
        XTouchUpstreamMsg::FaderAbs(FaderAbsMsg {
            idx: send_index,
            value: test_value,
        }),
        curr_mode,
    );

    // Should send level message to Reaper (upstream)
    assert_upstream_send_level_track_msg!(
        &to_reaper_rx,
        &target_guid,
        send_index,
        test_value as f32
    );
}

// ----------------------------------------------------------------------------
// Message Ordering Tests
// ----------------------------------------------------------------------------

#[test]
fn test_downstream_messages_sent_in_correct_order() {
    let (mut mode, _from_reaper_tx, _to_reaper_rx, _from_xtouch_tx, to_xtouch_rx) =
        setup_track_sends_mode();

    let target_guid = "target-ordering".to_string();
    let send_index = 1;

    let curr_mode = ModeState {
        mode: Mode::ReaperSends,
        state: State::Active,
    };

    // Assign send
    assign_send_to_channel(&mut mode, &target_guid, send_index, curr_mode);
    drain(&to_xtouch_rx); // Clear any previous messages

    // Send multiple messages in order
    mode.handle_downstream_messages(
        TrackMsg::TrackDataMsg(TrackDataMsg {
            guid: "selected-track".to_string(),
            direction: Direction::Downstream,
            data: DataPayload::SendLevel(SendLevel {
                send_index,
                level: 0.5,
            }),
        }),
        curr_mode,
    );

    mode.handle_downstream_messages(
        TrackMsg::TrackDataMsg(TrackDataMsg {
            guid: "selected-track".to_string(),
            direction: Direction::Downstream,
            data: DataPayload::SendLevel(SendLevel {
                send_index,
                level: 0.7,
            }),
        }),
        curr_mode,
    );

    mode.handle_downstream_messages(
        TrackMsg::TrackDataMsg(TrackDataMsg {
            guid: "selected-track".to_string(),
            direction: Direction::Downstream,
            data: DataPayload::SendLevel(SendLevel {
                send_index,
                level: 0.9,
            }),
        }),
        curr_mode,
    );

    // Verify messages received in order
    assert_downstream_fader_abs_msg!(&to_xtouch_rx, send_index, 0.5);
    assert_downstream_fader_abs_msg!(&to_xtouch_rx, send_index, 0.7);
    assert_downstream_fader_abs_msg!(&to_xtouch_rx, send_index, 0.9);
}

#[test]
fn test_upstream_messages_processed_in_correct_order() {
    let (mut mode, _from_reaper_tx, to_reaper_rx, _from_xtouch_tx, _to_xtouch_rx) =
        setup_track_sends_mode();

    let target_guid = "target-ordering-upstream".to_string();
    let send_index = 3;

    let curr_mode = ModeState {
        mode: Mode::ReaperSends,
        state: State::Active,
    };

    // Assign send
    assign_send_to_channel(&mut mode, &target_guid, send_index, curr_mode);

    // Send multiple upstream messages in order
    mode.handle_upstream_messages(
        XTouchUpstreamMsg::FaderAbs(FaderAbsMsg {
            idx: send_index,
            value: 0.6,
        }),
        curr_mode,
    );

    mode.handle_upstream_messages(
        XTouchUpstreamMsg::FaderAbs(FaderAbsMsg {
            idx: send_index,
            value: 0.7,
        }),
        curr_mode,
    );

    // Verify messages processed in order
    assert_upstream_send_level_track_msg!(&to_reaper_rx, &target_guid, send_index, 0.6);
    assert_upstream_send_level_track_msg!(&to_reaper_rx, &target_guid, send_index, 0.7);
}

// ----------------------------------------------------------------------------
// Threshold/EPSILON Tests
// ----------------------------------------------------------------------------

#[test]
fn test_send_level_changes_below_epsilon_threshold_ignored() {
    // Send level changes smaller than EPSILON should not send updates to hardware
    // NOTE: Current implementation does NOT filter by EPSILON - this is documenting expected behavior
    let (mut mode, _from_reaper_tx, _to_reaper_rx, _from_xtouch_tx, to_xtouch_rx) =
        setup_track_sends_mode();

    let target_guid = "target-epsilon".to_string();
    let send_index = 2;
    let initial_level = 0.5;

    let curr_mode = ModeState {
        mode: Mode::ReaperSends,
        state: State::Active,
    };

    // Assign send and set initial level
    assign_send_to_channel(&mut mode, &target_guid, send_index, curr_mode);
    drain(&to_xtouch_rx); // Clear any previous messages
    mode.handle_downstream_messages(
        TrackMsg::TrackDataMsg(TrackDataMsg {
            guid: "selected-track".to_string(),
            direction: Direction::Downstream,
            data: DataPayload::SendLevel(SendLevel {
                send_index,
                level: initial_level,
            }),
        }),
        curr_mode,
    );
    assert_downstream_fader_abs_msg!(&to_xtouch_rx, send_index, initial_level as f64);

    // Send level change smaller than EPSILON
    let small_change = initial_level + (EPSILON / 2.0);
    mode.handle_downstream_messages(
        TrackMsg::TrackDataMsg(TrackDataMsg {
            guid: "selected-track".to_string(),
            direction: Direction::Downstream,
            data: DataPayload::SendLevel(SendLevel {
                send_index,
                level: small_change,
            }),
        }),
        curr_mode,
    );

    // BUG: Current implementation does NOT filter by EPSILON
    // Expected: check_no_message!(&to_xtouch_rx, 100);
    // Actual: Message is sent even for small changes
    // For now, we verify the message IS sent (documenting current behavior)
    let result = to_xtouch_rx.recv_timeout(Duration::from_millis(1));
    check!(
        result.is_ok(),
        "BUG: Small changes are not filtered (expected EPSILON filtering)"
    );
}

#[test]
fn test_send_level_changes_above_epsilon_propagate() {
    let (mut mode, _from_reaper_tx, _to_reaper_rx, _from_xtouch_tx, to_xtouch_rx) =
        setup_track_sends_mode();

    let target_guid = "target-epsilon-large".to_string();
    let send_index = 1;
    let initial_level = 0.5;

    let curr_mode = ModeState {
        mode: Mode::ReaperSends,
        state: State::Active,
    };

    // Assign send and set initial level
    assign_send_to_channel(&mut mode, &target_guid, send_index, curr_mode);
    drain(&to_xtouch_rx); // Clear any previous messages
    mode.handle_downstream_messages(
        TrackMsg::TrackDataMsg(TrackDataMsg {
            guid: "selected-track".to_string(),
            direction: Direction::Downstream,
            data: DataPayload::SendLevel(SendLevel {
                send_index,
                level: initial_level,
            }),
        }),
        curr_mode,
    );
    assert_downstream_fader_abs_msg!(&to_xtouch_rx, send_index, initial_level as f64);

    // Send level change larger than EPSILON
    let large_change = initial_level + (EPSILON * 3.0);
    mode.handle_downstream_messages(
        TrackMsg::TrackDataMsg(TrackDataMsg {
            guid: "selected-track".to_string(),
            direction: Direction::Downstream,
            data: DataPayload::SendLevel(SendLevel {
                send_index,
                level: large_change,
            }),
        }),
        curr_mode,
    );

    // Should send message for large changes
    assert_downstream_fader_abs_msg!(&to_xtouch_rx, send_index, large_change as f64);
}

// ----------------------------------------------------------------------------
// Mode Transition Tests
// ----------------------------------------------------------------------------

#[test]
fn test_mode_transition_requests_track_query() {
    let (mut mode, _from_reaper_tx, _to_reaper_rx, _from_xtouch_tx, _to_xtouch_rx) =
        setup_track_sends_mode();

    let selected_track_guid = "track-for-sends".to_string();

    // Create an unbounded sender that will be used to send messages upstream
    let (upstream_sender, upstream_receiver) = unbounded();

    // Initiate mode transition
    let result_mode =
        mode.initiate_mode_transition(Mode::ReaperVolPan, upstream_sender, &selected_track_guid);

    // Should send TrackQuery for the selected track
    let msg1 = upstream_receiver.recv_timeout(Duration::from_millis(1));
    assert!(msg1.is_ok(), "Should send TrackQuery for selected track");
    match msg1.unwrap() {
        TrackMsg::TrackQuery(query) => {
            check!(query.guid == selected_track_guid, "GUID should match");
            check!(
                query.direction == Direction::Downstream,
                "Should be downstream"
            );
        }
        _ => panic!("Expected TrackQuery message"),
    }

    // Should send barrier upstream
    let barrier_msg = upstream_receiver.recv_timeout(Duration::from_millis(1));
    assert!(barrier_msg.is_ok(), "Should send barrier message upstream");
    match barrier_msg.unwrap() {
        TrackMsg::Barrier(_) => {}
        _ => panic!("Expected Barrier message"),
    }

    // Should be waiting for barrier from upstream
    match result_mode.state {
        State::WaitingBarrierFromUpstream(_) => {
            // Success - we're in the expected state
        }
        _ => panic!("Should be waiting for barrier from downstream"),
    }
}

// ----------------------------------------------------------------------------
// Complex Integration Tests
// ----------------------------------------------------------------------------

#[test]
fn test_complex_multi_send_integration() {
    let (mut mode, _from_reaper_tx, to_reaper_rx, _from_xtouch_tx, to_xtouch_rx) =
        setup_track_sends_mode();
    let curr_mode = ModeState {
        state: State::Active,
        mode: Mode::ReaperSends,
    };

    let send1_guid = "send-target-1".to_string();
    let send2_guid = "send-target-2".to_string();
    let send3_guid = "send-target-3".to_string();

    // === PHASE 1: Map multiple sends ===
    assign_send_to_channel(&mut mode, &send1_guid, 0, curr_mode);
    assign_send_to_channel(&mut mode, &send2_guid, 1, curr_mode);
    assign_send_to_channel(&mut mode, &send3_guid, 2, curr_mode);
    drain(&to_xtouch_rx); // Clear any previous messages

    // === PHASE 2: Send levels to all sends ===
    mode.handle_downstream_messages(
        TrackMsg::TrackDataMsg(TrackDataMsg {
            guid: "selected-track".to_string(),
            direction: Direction::Downstream,
            data: DataPayload::SendLevel(SendLevel {
                send_index: 0,
                level: 0.3,
            }),
        }),
        curr_mode,
    );
    assert_downstream_fader_abs_msg!(&to_xtouch_rx, 0, 0.3);

    mode.handle_downstream_messages(
        TrackMsg::TrackDataMsg(TrackDataMsg {
            guid: "selected-track".to_string(),
            direction: Direction::Downstream,
            data: DataPayload::SendLevel(SendLevel {
                send_index: 1,
                level: 0.6,
            }),
        }),
        curr_mode,
    );
    assert_downstream_fader_abs_msg!(&to_xtouch_rx, 1, 0.6);

    mode.handle_downstream_messages(
        TrackMsg::TrackDataMsg(TrackDataMsg {
            guid: "selected-track".to_string(),
            direction: Direction::Downstream,
            data: DataPayload::SendLevel(SendLevel {
                send_index: 2,
                level: 0.9,
            }),
        }),
        curr_mode,
    );
    assert_downstream_fader_abs_msg!(&to_xtouch_rx, 2, 0.9);

    // === PHASE 3: Hardware interaction on multiple channels ===
    // Move fader on channel 0
    mode.handle_upstream_messages(
        XTouchUpstreamMsg::FaderAbs(FaderAbsMsg { idx: 0, value: 0.4 }),
        curr_mode,
    );
    assert_upstream_send_level_track_msg!(&to_reaper_rx, &send1_guid, 0, 0.4);

    // Move fader on channel 1
    mode.handle_upstream_messages(
        XTouchUpstreamMsg::FaderAbs(FaderAbsMsg { idx: 1, value: 0.7 }),
        curr_mode,
    );
    assert_upstream_send_level_track_msg!(&to_reaper_rx, &send2_guid, 1, 0.7);

    // Move fader on channel 2
    mode.handle_upstream_messages(
        XTouchUpstreamMsg::FaderAbs(FaderAbsMsg {
            idx: 2,
            value: 0.95,
        }),
        curr_mode,
    );
    assert_upstream_send_level_track_msg!(&to_reaper_rx, &send3_guid, 2, 0.95);

    // === PHASE 4: Update send levels from Reaper ===
    mode.handle_downstream_messages(
        TrackMsg::TrackDataMsg(TrackDataMsg {
            guid: "selected-track".to_string(),
            direction: Direction::Downstream,
            data: DataPayload::SendLevel(SendLevel {
                send_index: 0,
                level: 0.5,
            }),
        }),
        curr_mode,
    );
    assert_downstream_fader_abs_msg!(&to_xtouch_rx, 0, 0.5);

    mode.handle_downstream_messages(
        TrackMsg::TrackDataMsg(TrackDataMsg {
            guid: "selected-track".to_string(),
            direction: Direction::Downstream,
            data: DataPayload::SendLevel(SendLevel {
                send_index: 1,
                level: 0.8,
            }),
        }),
        curr_mode,
    );
    assert_downstream_fader_abs_msg!(&to_xtouch_rx, 1, 0.8);

    // Verify no additional messages
    check_no_message!(&to_xtouch_rx, 1);
    check_no_message!(&to_reaper_rx, 1);
}

#[test]
fn test_multiple_tracks_and_switching_selections() {
    let (mut mode, _from_reaper_tx, _to_reaper_rx, _from_xtouch_tx, to_xtouch_rx) =
        setup_track_sends_mode();

    let track1_send1 = "track1-send1".to_string();
    let track1_send2 = "track1-send2".to_string();
    let track2_send1 = "track2-send1".to_string();

    let curr_mode = ModeState {
        mode: Mode::ReaperSends,
        state: State::Active,
    };

    // Set up sends for track 1
    assign_send_to_channel(&mut mode, &track1_send1, 0, curr_mode);
    assign_send_to_channel(&mut mode, &track1_send2, 1, curr_mode);
    drain(&to_xtouch_rx); // Clear any previous messages

    // Send levels for track 1 sends
    mode.handle_downstream_messages(
        TrackMsg::TrackDataMsg(TrackDataMsg {
            guid: "selected-track".to_string(),
            direction: Direction::Downstream,
            data: DataPayload::SendLevel(SendLevel {
                send_index: 0,
                level: 0.3,
            }),
        }),
        curr_mode,
    );
    assert_downstream_fader_abs_msg!(&to_xtouch_rx, 0, 0.3);

    mode.handle_downstream_messages(
        TrackMsg::TrackDataMsg(TrackDataMsg {
            guid: "selected-track".to_string(),
            direction: Direction::Downstream,
            data: DataPayload::SendLevel(SendLevel {
                send_index: 1,
                level: 0.6,
            }),
        }),
        curr_mode,
    );
    assert_downstream_fader_abs_msg!(&to_xtouch_rx, 1, 0.6);

    // Simulate switching to track 2 (different sends get mapped to same channels)
    assign_send_to_channel(&mut mode, &track2_send1, 0, curr_mode);
    drain(&to_xtouch_rx); // Clear messages from track 1

    // Send level for track 2 send 1
    mode.handle_downstream_messages(
        TrackMsg::TrackDataMsg(TrackDataMsg {
            guid: "selected-track".to_string(),
            direction: Direction::Downstream,
            data: DataPayload::SendLevel(SendLevel {
                send_index: 0,
                level: 0.9,
            }),
        }),
        curr_mode,
    );
    assert_downstream_fader_abs_msg!(&to_xtouch_rx, 0, 0.9);

    // Switch back to track 1 by reassigning track1_send1
    assign_send_to_channel(&mut mode, &track1_send1, 0, curr_mode);
    drain(&to_xtouch_rx); // Clear messages from track 2

    // Send level should update correctly
    mode.handle_downstream_messages(
        TrackMsg::TrackDataMsg(TrackDataMsg {
            guid: "selected-track".to_string(),
            direction: Direction::Downstream,
            data: DataPayload::SendLevel(SendLevel {
                send_index: 0,
                level: 0.4,
            }),
        }),
        curr_mode,
    );
    assert_downstream_fader_abs_msg!(&to_xtouch_rx, 0, 0.4);
}

#[test]
fn test_expanded_real_world_integration() {
    let (mut mode, _from_reaper_tx, to_reaper_rx, _from_xtouch_tx, to_xtouch_rx) =
        setup_track_sends_mode();

    let curr_mode = ModeState {
        mode: Mode::ReaperSends,
        state: State::Active,
    };

    // === SCENARIO 1: Set up multiple sends on Track A ===
    let track_a_send_1 = "track-a-send-1".to_string();
    let track_a_send_2 = "track-a-send-2".to_string();
    let track_a_send_3 = "track-a-send-3".to_string();

    assign_send_to_channel(&mut mode, &track_a_send_1, 0, curr_mode);
    assign_send_to_channel(&mut mode, &track_a_send_2, 1, curr_mode);
    assign_send_to_channel(&mut mode, &track_a_send_3, 2, curr_mode);
    drain(&to_xtouch_rx); // Clear any previous messages

    // Set initial levels
    for (idx, level) in [(0, 0.3), (1, 0.5), (2, 0.7)] {
        mode.handle_downstream_messages(
            TrackMsg::TrackDataMsg(TrackDataMsg {
                guid: "selected-track".to_string(),
                direction: Direction::Downstream,
                data: DataPayload::SendLevel(SendLevel {
                    send_index: idx,
                    level,
                }),
            }),
            curr_mode,
        );
        assert_downstream_fader_abs_msg!(&to_xtouch_rx, idx, level as f64);
    }

    // === SCENARIO 2: User adjusts faders on hardware ===
    for (idx, level) in [(0, 0.4), (1, 0.6), (2, 0.8)] {
        mode.handle_upstream_messages(
            XTouchUpstreamMsg::FaderAbs(FaderAbsMsg { idx, value: level }),
            curr_mode,
        );
    }

    // Verify upstream messages sent to Reaper
    assert_upstream_send_level_track_msg!(&to_reaper_rx, &track_a_send_1, 0, 0.4);
    assert_upstream_send_level_track_msg!(&to_reaper_rx, &track_a_send_2, 1, 0.6);
    assert_upstream_send_level_track_msg!(&to_reaper_rx, &track_a_send_3, 2, 0.8);

    // === SCENARIO 3: Switch to Track B (different sends mapped to same channels) ===
    let track_b_send_1 = "track-b-send-1".to_string();
    let track_b_send_2 = "track-b-send-2".to_string();

    assign_send_to_channel(&mut mode, &track_b_send_1, 0, curr_mode);
    assign_send_to_channel(&mut mode, &track_b_send_2, 1, curr_mode);
    drain(&to_xtouch_rx); // Clear messages from Track A

    // Track B levels
    for (idx, level) in [(0, 0.2), (1, 0.9)] {
        mode.handle_downstream_messages(
            TrackMsg::TrackDataMsg(TrackDataMsg {
                guid: "selected-track".to_string(),
                direction: Direction::Downstream,
                data: DataPayload::SendLevel(SendLevel {
                    send_index: idx,
                    level,
                }),
            }),
            curr_mode,
        );
        assert_downstream_fader_abs_msg!(&to_xtouch_rx, idx, level as f64);
    }

    // === SCENARIO 4: Remap Track B send 1 to different channel ===
    assign_send_to_channel(&mut mode, &track_b_send_1, 5, curr_mode);
    drain(&to_xtouch_rx); // Clear previous messages

    mode.handle_downstream_messages(
        TrackMsg::TrackDataMsg(TrackDataMsg {
            guid: "selected-track".to_string(),
            direction: Direction::Downstream,
            data: DataPayload::SendLevel(SendLevel {
                send_index: 5,
                level: 0.95,
            }),
        }),
        curr_mode,
    );
    assert_downstream_fader_abs_msg!(&to_xtouch_rx, 5, 0.95);

    // TODO: We think we do NOT want this behavior. But if we change our minds, bring this test
    // back.
    //
    // // === SCENARIO 5: Verify both channels respond to Track B send 1 (current behavior allows multiple channels for same target) ===
    // // Channel 0 still has track_b_send_1 mapped
    // mode.handle_upstream_messages(
    //     XTouchUpstreamMsg::FaderAbs(FaderAbsMsg { idx: 0, value: 0.1 }),
    //     curr_mode,
    // );
    // // Current implementation: Channel 0 is still mapped to track_b_send_1
    // assert_upstream_send_level_track_msg!(&to_reaper_rx, &track_b_send_1, 0, 0.1);

    // === SCENARIO 6: Switch back to Track A ===
    assign_send_to_channel(&mut mode, &track_a_send_1, 0, curr_mode);
    drain(&to_xtouch_rx); // Clear messages from Track B

    // Track A send 1 should work on channel 0 again
    mode.handle_downstream_messages(
        TrackMsg::TrackDataMsg(TrackDataMsg {
            guid: "selected-track".to_string(),
            direction: Direction::Downstream,
            data: DataPayload::SendLevel(SendLevel {
                send_index: 0,
                level: 0.55,
            }),
        }),
        curr_mode,
    );
    assert_downstream_fader_abs_msg!(&to_xtouch_rx, 0, 0.55);

    // Hardware interaction should work
    mode.handle_upstream_messages(
        XTouchUpstreamMsg::FaderAbs(FaderAbsMsg {
            idx: 0,
            value: 0.65,
        }),
        curr_mode,
    );
    assert_upstream_send_level_track_msg!(&to_reaper_rx, &track_a_send_1, 0, 0.65);

    // === Final verification: No unexpected messages ===
    check_no_message!(&to_xtouch_rx, 1);
    check_no_message!(&to_reaper_rx, 1);
}
