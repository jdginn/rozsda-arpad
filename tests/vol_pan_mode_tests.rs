// Integration tests for VolumePanMode
//
// These tests verify the behavior of the VolumePanMode, which manages the mapping
// between Reaper tracks and XTouch controller hardware (faders, buttons, LEDs).

use arpad_rust::midi::xtouch::{
    ArmLEDMsg, ArmPress, FaderAbsMsg, LEDState, MuteLEDMsg, MutePress, SoloLEDMsg, SoloPress,
    XTouchDownstreamMsg, XTouchUpstreamMsg,
};
use arpad_rust::modes::mode_manager::{Barrier, Mode, ModeHandler, ModeState, State};
use arpad_rust::modes::reaper_vol_pan::VolumePanMode;
use arpad_rust::track::track::{DataPayload, Direction, TrackDataMsg, TrackMsg};
use crossbeam_channel::{Receiver, Sender, bounded};
use std::time::Duration;

/// Helper to create a VolumePanMode instance for testing
fn setup_vol_pan_mode() -> (
    VolumePanMode,
    Sender<TrackMsg>,
    Receiver<TrackMsg>,
    Sender<XTouchUpstreamMsg>,
    Receiver<XTouchDownstreamMsg>,
) {
    let (from_reaper_tx, from_reaper_rx) = bounded(128);
    let (to_reaper_tx, to_reaper_rx) = bounded(128);
    let (from_xtouch_tx, from_xtouch_rx) = bounded(128);
    let (to_xtouch_tx, to_xtouch_rx) = bounded(128);

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

#[test]
fn test_vol_pan_mode_assigns_tracks_by_reaper_index() {
    let (mut mode, from_reaper_tx, _to_reaper_rx, _from_xtouch_tx, _to_xtouch_rx) =
        setup_vol_pan_mode();

    let test_guid = "track-guid-1".to_string();
    let reaper_index = 2;

    let curr_mode = ModeState {
        mode: Mode::ReaperVolPan,
        state: State::Active,
    };

    // Send a ReaperTrackIndex message to assign the track to hardware channel 2
    let msg = TrackMsg::TrackDataMsg(TrackDataMsg {
        guid: test_guid.clone(),
        direction: Direction::Downstream,
        data: DataPayload::ReaperTrackIndex(Some(reaper_index)),
    });

    let result_mode = mode.handle_downstream_messages(msg, curr_mode);

    // Mode should remain unchanged
    assert_eq!(result_mode, curr_mode);

    // Verify the track is now assigned to hardware channel 2
    let found_channel = mode.find_hw_channel(&test_guid);
    assert_eq!(
        found_channel,
        Some(reaper_index as usize),
        "Track should be assigned to hardware channel matching Reaper index"
    );
}

#[test]
fn test_vol_pan_mode_volume_updates_sent_to_faders() {
    let (mut mode, from_reaper_tx, _to_reaper_rx, _from_xtouch_tx, to_xtouch_rx) =
        setup_vol_pan_mode();

    let test_guid = "track-guid-2".to_string();
    let hw_channel = 3;
    let test_volume = 0.65;

    let curr_mode = ModeState {
        mode: Mode::ReaperVolPan,
        state: State::Active,
    };

    // First, assign the track to a hardware channel
    mode.handle_downstream_messages(
        TrackMsg::TrackDataMsg(TrackDataMsg {
            guid: test_guid.clone(),
            direction: Direction::Downstream,
            data: DataPayload::ReaperTrackIndex(Some(hw_channel)),
        }),
        curr_mode,
    );

    // Now send a volume update
    mode.handle_downstream_messages(
        TrackMsg::TrackDataMsg(TrackDataMsg {
            guid: test_guid.clone(),
            direction: Direction::Downstream,
            data: DataPayload::Volume(test_volume),
        }),
        curr_mode,
    );

    // Should receive a fader update on XTouch
    let result = to_xtouch_rx.recv_timeout(Duration::from_millis(100));
    assert!(result.is_ok(), "Should receive XTouch fader message");

    if let Ok(XTouchDownstreamMsg::FaderAbs(fader_msg)) = result {
        assert_eq!(fader_msg.idx, hw_channel, "Fader index should match");
        assert_eq!(
            fader_msg.value, test_volume as f64,
            "Fader value should match volume"
        );
    } else {
        panic!("Expected FaderAbs message");
    }
}

#[test]
fn test_vol_pan_mode_mute_button_toggles() {
    let (mut mode, _from_reaper_tx, to_reaper_rx, _from_xtouch_tx, to_xtouch_rx) =
        setup_vol_pan_mode();

    let test_guid = "track-guid-3".to_string();
    let hw_channel = 1;

    let curr_mode = ModeState {
        mode: Mode::ReaperVolPan,
        state: State::Active,
    };

    // Assign track to hardware channel
    mode.handle_downstream_messages(
        TrackMsg::TrackDataMsg(TrackDataMsg {
            guid: test_guid.clone(),
            direction: Direction::Downstream,
            data: DataPayload::ReaperTrackIndex(Some(hw_channel)),
        }),
        curr_mode,
    );

    // Simulate mute button press
    let msg = XTouchUpstreamMsg::MutePress(MutePress { idx: hw_channel });

    mode.handle_upstream_messages(msg, curr_mode);

    // Should receive:
    // 1. TrackMsg to Reaper setting mute to true
    // 2. LED update to XTouch showing mute is on

    let track_msg_result = to_reaper_rx.recv_timeout(Duration::from_millis(100));
    assert!(
        track_msg_result.is_ok(),
        "Should send mute message to Reaper"
    );

    if let Ok(TrackMsg::TrackDataMsg(msg)) = track_msg_result {
        assert_eq!(msg.guid, test_guid);
        if let DataPayload::Muted(muted) = msg.data {
            assert!(muted, "First toggle should mute the track");
        } else {
            panic!("Expected Muted payload");
        }
    } else {
        panic!("Expected TrackDataMsg");
    }

    let led_msg_result = to_xtouch_rx.recv_timeout(Duration::from_millis(100));
    assert!(led_msg_result.is_ok(), "Should send LED update to XTouch");

    if let Ok(XTouchDownstreamMsg::MuteLED(led_msg)) = led_msg_result {
        assert_eq!(led_msg.idx, hw_channel);
        assert!(
            matches!(led_msg.state, LEDState::On),
            "LED should be on after mute"
        );
    } else {
        panic!("Expected MuteLED message");
    }
}

#[test]
fn test_vol_pan_mode_fader_sends_volume_upstream() {
    let (mut mode, _from_reaper_tx, to_reaper_rx, _from_xtouch_tx, _to_xtouch_rx) =
        setup_vol_pan_mode();

    let test_guid = "track-guid-4".to_string();
    let hw_channel = 0;
    let new_volume = 0.85;

    let curr_mode = ModeState {
        mode: Mode::ReaperVolPan,
        state: State::Active,
    };

    // Assign track to hardware channel
    mode.handle_downstream_messages(
        TrackMsg::TrackDataMsg(TrackDataMsg {
            guid: test_guid.clone(),
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
        assert_eq!(msg.guid, test_guid);
        assert_eq!(msg.direction, Direction::Upstream);
        if let DataPayload::Volume(volume) = msg.data {
            assert_eq!(volume, new_volume as f32, "Volume should match fader value");
        } else {
            panic!("Expected Volume payload");
        }
    } else {
        panic!("Expected TrackDataMsg");
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
        assert_eq!(received_barrier, barrier);
    } else {
        panic!("Expected Barrier message");
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

// TODO: Test solo and arm buttons (similar to mute test)
// TODO: Test behavior when fader is moved for unassigned hardware channel
// TODO: Test LED updates from downstream (e.g., Reaper sends mute state change)
// TODO: Test mode transition initiation
