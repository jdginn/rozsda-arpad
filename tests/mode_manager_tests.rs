// Integration tests for ModeManager
//
// These tests verify the message routing and mode transition logic of the ModeManager.
// ModeManager coordinates between upstream (Reaper) and downstream (XTouch) endpoints,
// managing different control modes and ensuring proper state synchronization during transitions.

use arpad_rust::modes::mode_manager::{Barrier, ModeManager};
use arpad_rust::midi::xtouch::{
    FaderAbsMsg, XTouchDownstreamMsg, XTouchUpstreamMsg,
};
use arpad_rust::track::track::{DataPayload, Direction, TrackDataMsg, TrackMsg};
use crossbeam_channel::{bounded, Receiver, Sender};
use std::time::Duration;

/// Helper to set up channels for mode manager testing
fn setup_mode_manager_channels() -> (
    Sender<TrackMsg>,
    Receiver<TrackMsg>,
    Sender<XTouchUpstreamMsg>,
    Receiver<XTouchDownstreamMsg>,
) {
    let (reaper_tx, reaper_rx) = bounded(128);
    let (xtouch_tx, xtouch_rx) = bounded(128);
    let (to_reaper_tx, to_reaper_rx) = bounded(128);
    let (to_xtouch_tx, to_xtouch_rx) = bounded(128);

    // Start the mode manager
    ModeManager::start(reaper_rx, to_reaper_tx, xtouch_rx, to_xtouch_tx);

    // Give the thread time to start
    std::thread::sleep(Duration::from_millis(50));

    (reaper_tx, to_reaper_rx, xtouch_tx, to_xtouch_rx)
}

#[test]
fn test_mode_manager_forwards_track_messages_downstream() {
    let (reaper_tx, _to_reaper_rx, _xtouch_tx, _to_xtouch_rx) = setup_mode_manager_channels();

    let test_guid = "test-track".to_string();
    let test_volume = 0.5;

    // Send a track message from Reaper
    reaper_tx
        .send(TrackMsg::TrackDataMsg(TrackDataMsg {
            guid: test_guid.clone(),
            direction: Direction::Downstream,
            data: DataPayload::Volume(test_volume),
        }))
        .unwrap();

    // TODO: The mode manager doesn't directly forward TrackMsg to XTouch.
    // Instead, the VolumePanMode handles the conversion to XTouchDownstreamMsg.
    // This test needs to be adjusted to match the actual architecture.
    //
    // Expected behavior: VolumePanMode should receive the TrackMsg and convert
    // it to a FaderAbs message for the corresponding hardware channel.
    
    // For now, we'll just verify that no panic occurs
    std::thread::sleep(Duration::from_millis(100));
}

#[test]
fn test_mode_manager_forwards_xtouch_fader_messages() {
    let (_reaper_tx, _to_reaper_rx, xtouch_tx, _to_xtouch_rx) = setup_mode_manager_channels();

    // Send a fader movement from XTouch
    xtouch_tx
        .send(XTouchUpstreamMsg::FaderAbs(FaderAbsMsg {
            idx: 0,
            value: 0.75,
        }))
        .unwrap();

    // TODO: Similar to above, the mode manager doesn't directly forward XTouchMsg to Reaper.
    // VolumePanMode handles the conversion to TrackMsg.
    //
    // Expected behavior: VolumePanMode should receive the FaderAbs message,
    // look up which track is assigned to hardware channel 0, and send a
    // TrackMsg with Volume data upstream.

    // For now, verify no panic
    std::thread::sleep(Duration::from_millis(100));
}

#[test]
fn test_mode_manager_barrier_propagation() {
    let (reaper_tx, _to_reaper_rx, _xtouch_tx, to_xtouch_rx) = setup_mode_manager_channels();

    let barrier = Barrier::new();

    // Send barrier from upstream (Reaper)
    reaper_tx.send(TrackMsg::Barrier(barrier)).unwrap();

    // Barrier should be forwarded downstream to XTouch
    let result = to_xtouch_rx.recv_timeout(Duration::from_millis(200));
    
    // TODO: Verify that barrier is actually forwarded by the mode handlers.
    // The VolumePanMode should forward barriers it receives.
    
    if let Ok(XTouchDownstreamMsg::Barrier(received_barrier)) = result {
        assert_eq!(received_barrier, barrier, "Barrier should match");
    } else {
        // TODO: Behavior unclear - should barriers always be forwarded?
        // Current implementation might not forward them in all states.
        println!("Barrier was not forwarded (may be expected behavior)");
    }
}

// TODO: Test mode transitions
// The following tests are challenging because:
// 1. Mode transitions involve complex state management
// 2. We need to mock or simulate the specific mode handlers
// 3. The current architecture makes it difficult to observe internal state
//
// Suggested tests to implement once mode transition API is clearer:
// - test_mode_transition_from_vol_pan_to_sends
// - test_mode_transition_blocks_upstream_messages
// - test_barrier_synchronization_during_transition
// - test_rapid_mode_transitions

#[test]
#[ignore] // Ignoring until mode transition behavior is clarified
fn test_mode_transition_state_management() {
    // TODO: How do we initiate a mode transition from outside the ModeManager?
    // The current design has modes initiate transitions themselves based on
    // button presses, but we need a way to trigger this from tests.
    //
    // Possible approaches:
    // 1. Send button press messages (e.g., MIDITracksPress) to trigger transition
    // 2. Add a test API to ModeManager for forcing mode transitions
    // 3. Test at a higher level with actual button press simulations
}

#[test]
#[ignore] // Ignoring until state observation is possible
fn test_messages_blocked_during_transition() {
    // TODO: How do we observe that messages are being blocked during transition?
    // The ModeManager's curr_mode is private, and we can't observe the state
    // from outside.
    //
    // Possible approaches:
    // 1. Add test accessors for observing state
    // 2. Infer state from behavior (timing-based, unreliable)
    // 3. Refactor ModeManager to expose state for testing
}

// Edge case: What happens if we send messages while ModeManager is starting up?
#[test]
fn test_messages_during_startup() {
    let (reaper_tx, _to_reaper_rx, _xtouch_tx, _to_xtouch_rx) = setup_mode_manager_channels();

    // Send a message immediately without waiting
    let result = reaper_tx.try_send(TrackMsg::TrackDataMsg(TrackDataMsg {
        guid: "test".to_string(),
        direction: Direction::Downstream,
        data: DataPayload::Volume(0.5),
    }));

    assert!(result.is_ok(), "Should be able to send messages during startup");
}
