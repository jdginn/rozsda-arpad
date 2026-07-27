use std::time::Duration;

use assert2::{assert, check};
use crossbeam_channel::{Receiver, Sender, bounded};
use float_cmp::approx_eq;
use uuid::Uuid;

use arpad_rust::midi::v1m::{ChannelFaderMsg, DownstreamMsg, UpstreamMsg};
use arpad_rust::modes::mode_manager::{Barrier, Mode, ModeHandler, ModeManager, ModeState};
use arpad_rust::modes::reaper_vol_pan::VolumePanMode;
use arpad_rust::track::track;
use arpad_rust::track::track::TrackMsg;

// EPSILON constant for floating-point threshold testing
const EPSILON: f32 = 0.01;

/// Helper to assert a ChannelFader message is received with the expected values
#[macro_export]
macro_rules! assert_downstream_fader_abs_msg {
    ($rx:expr, $expected_idx:expr, $expected_value:expr) => {{
        let msg = $rx.recv_timeout(Duration::from_millis(1)).expect(
            "Expected to receive a ChannelFader message but received {:?}",
            result,
        );

        if let v1mDownstreamMsg::ChannelFader(fader_msg) = msg {
            check!(fader_msg.idx == $expected_idx);
            check!(
                approx_eq!(
                    f64,
                    fader_msg.value,
                    $expected_value,
                    epsilon = EPSILON as f64
                ),
                "Fader value should match approximately\nExpected: {}, Got: {}",
                $expected_value,
                fader_msg.value
            );
        } else {
            panic!("Expected v1mDownstreamMsg::ChannelFader, but got {:?}", msg);
        }
    }};
}

/// Macro to assert a Volume TrackDataMsg is received upstream
#[macro_export]
macro_rules! assert_upstream_volume_track_msg {
    ($rx:expr, $expected_guid:expr, $expected_value:expr) => {{
        let result = $rx.recv_timeout(std::time::Duration::from_millis(1));
        check!(
            result.is_ok(),
            "Should receive volume message to Reaper but received {:?}",
            result
        );

        match result {
            Ok(TrackMsg::Volume(msg)) => {
                check!(&msg.track_guid == $expected_guid, "Track GUID should match");
                check!(
                    approx_eq!(f32, msg.volume, $expected_value, epsilon = EPSILON),
                    "Volume should match approximately\nExpected: {}, Got: {}",
                    $expected_value,
                    msg.volume
                );
            }
            _ => panic!("Expected Volume msg but got {:?}", result),
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

// Macro to assert a Barrier message is received by v1m with the expected from/to modes
#[macro_export]
macro_rules! assert_downstream_barrier_msg {
    ($rx:expr, $expected_from_mode:expr, $expected_to_mode:expr) => {{
        let result = $rx.recv_timeout(std::time::Duration::from_millis(1));
        check!(
            result.is_ok(),
            "Should receive barrier message but received {:?}",
            result
        );

        match result {
            Ok(v1mDownstreamMsg::Barrier(msg)) => {
                check!(msg.from == $expected_from_mode, "'From mode' should match");
                check!(msg.to == $expected_to_mode, "'To mode' should match");
            }
            _ => panic!("Expected Barrier message but got {:?}", result),
        }
    }};
}

// Macro to assert a Barrier message is received by reaper with the expected from/to modes
#[macro_export]
macro_rules! assert_upstream_barrier_msg {
    ($rx:expr, $expected_from_mode:expr, $expected_to_mode:expr) => {{
        let result = $rx.recv_timeout(std::time::Duration::from_millis(1));
        check!(
            result.is_ok(),
            "Should receive barrier message but received {:?}",
            result
        );

        match result {
            Ok(TrackMsg::Barrier(msg)) => {
                check!(msg.from == $expected_from_mode, "'From mode' should match");
                check!(msg.to == $expected_to_mode, "'To mode' should match");
                msg
            }
            _ => panic!("Expected Barrier message but got {:?}", result),
        }
    }};
}

/// Macro to assert no message is received within timeout
#[macro_export]
macro_rules! assert_no_message {
    ($rx:expr, $timeout_ms:expr) => {{
        let result = $rx.recv_timeout(std::time::Duration::from_millis($timeout_ms));
        check!(
            result.is_err(),
            "Should not receive any message, but got {:?}!",
            result
        );
    }};
}

fn drain_downstream_until_barrier(rx: &Receiver<DownstreamMsg>) -> Option<Barrier> {
    let deadline = std::time::Instant::now() + Duration::from_millis(1); // pick a test-friendly bound

    loop {
        // Drain anything currently queued without waiting.
        while let Ok(msg) = rx.try_recv() {
            if let DownstreamMsg::Barrier(b) = msg {
                return Some(b);
            }
        }

        // If not found yet, wait a bit for the next message (or until deadline).
        let now = std::time::Instant::now();
        if now >= deadline {
            return None;
        }
        let remaining = deadline - now;

        match rx.recv_timeout(remaining.min(Duration::from_millis(1))) {
            Ok(DownstreamMsg::Barrier(b)) => return Some(b),
            Ok(_) => continue, // got something else; loop and keep draining
            Err(crossbeam_channel::RecvTimeoutError::Timeout) => continue,
            Err(crossbeam_channel::RecvTimeoutError::Disconnected) => return None,
        }
    }
}

fn drain<T>(rx: &Receiver<T>) {
    // Drops (flushes) all messages currently buffered at the time we start draining,
    // plus any that arrive before we hit Empty.
    for _msg in rx.try_iter() {
        // intentionally discard
    }
}

fn drain_and_print<T: std::fmt::Debug>(rx: &Receiver<T>) {
    // Drops (flushes) all messages currently buffered at the time we start draining,
    // plus any that arrive before we hit Empty.
    for msg in rx.try_iter() {
        println!("Drained message: {:?}", msg);
    }
}

/// Helper to set up channels for mode manager testing
fn setup_mode_manager_channels() -> (
    Sender<TrackMsg>,
    Receiver<TrackMsg>,
    Sender<UpstreamMsg>,
    Receiver<DownstreamMsg>,
) {
    let (reaper_tx, reaper_rx) = bounded(128);
    let (v1m_tx, v1m_rx) = bounded(128);
    let (to_reaper_tx, to_reaper_rx) = bounded(128);
    let (to_v1m_tx, to_v1m_rx) = bounded(128);

    // Start the mode manager
    ModeManager::start(reaper_rx, to_reaper_tx, v1m_rx, to_v1m_tx);

    // Give the thread time to start
    std::thread::sleep(Duration::from_millis(50));

    (reaper_tx, to_reaper_rx, v1m_tx, to_v1m_rx)
}

/// Helper function to assign a track to a hardware channel
fn assign_track_to_channel(
    mode: &mut VolumePanMode,
    guid: Uuid,
    hw_channel: i32,
    curr_mode: ModeState,
) -> ModeState {
    mode.handle_messages_from_upstream(
        track::ReaperTrackIndex {
            track_guid: guid,
            track_index: Some(hw_channel),
        }
        .into(),
        curr_mode,
    )
}

#[test]
fn test_mode_transition_vol_pan_to_sends_initiated_by_hardware() {
    let (reaper_tx, to_reaper_rx, v1m_tx, to_v1m_rx) = setup_mode_manager_channels();

    // We start in VolPan mode

    // Try to initiate transition from VolPan to Sends by simulating a MIDITracksPress
    v1m_tx.send(UpstreamMsg::MIDITracksPress {}).unwrap();

    // We should not transition modes if no track is selected
    assert_no_message!(to_reaper_rx, 1);

    let track1_guid = Uuid::new_v4();
    let track2_guid = Uuid::new_v4();

    // Register two tracks and select a track
    reaper_tx
        .send(
            track::ReaperTrackIndex {
                track_guid: track1_guid,
                track_index: Some(1),
            }
            .into(),
        )
        .unwrap();
    reaper_tx
        .send(
            track::ReaperTrackIndex {
                track_guid: track2_guid,
                track_index: Some(2),
            }
            .into(),
        )
        .unwrap();
    reaper_tx
        .send(
            track::Selected {
                track_guid: track1_guid,
                selected: true,
            }
            .into(),
        )
        .unwrap();
    // assert_upstream_volume_track_msg!(to_reaper_rx, &track1_guid, 0.75);

    // TODO: is there a better way to do this than sleeping?
    std::thread::sleep(Duration::from_millis(100));

    // v1m messages should be forwarded upstream to reaper
    v1m_tx
        .send(UpstreamMsg::ChannelFader(ChannelFaderMsg {
            idx: 0,
            value: 0.75,
        }))
        .unwrap();
    std::thread::sleep(Duration::from_millis(100));
    assert_upstream_volume_track_msg!(to_reaper_rx, &track1_guid, 0.75);

    // Initiate mode transition
    v1m_tx.send(UpstreamMsg::MIDITracksPress {}).unwrap();

    // Swallow the track data query message
    to_reaper_rx.recv().unwrap();

    let barrier = assert_upstream_barrier_msg!(to_reaper_rx, Mode::ReaperVolPan, Mode::ReaperSends);

    // From here on out, any messages from v1m should be blocked until the mode transition is complete and the barrier is reflected back.
    // We will periodicially send more messages from v1m and verify that they are blocked until we reflect the barrier back.
    v1m_tx
        .send(UpstreamMsg::ChannelFader(ChannelFaderMsg {
            idx: 0,
            value: 0.75,
        }))
        .unwrap();
    assert_no_message!(to_reaper_rx, 1);

    // Mock the response to the track data query
    reaper_tx
        .send(
            track::SendIndex {
                track_guid: track1_guid,
                send_index: 1,
                send_guid: track2_guid,
            }
            .into(),
        )
        .unwrap();

    // v1m messages should still be blocked...
    v1m_tx
        .send(UpstreamMsg::ChannelFader(ChannelFaderMsg {
            idx: 0,
            value: 0.75,
        }))
        .unwrap();
    assert_no_message!(to_reaper_rx, 1);

    // There should be no barrier sent to v1m until it reflects from reaper
    drain_downstream_until_barrier(&to_v1m_rx);
    assert_no_message!(to_v1m_rx, 1);

    // Reflect the barrier back from the reaper side indicating that we are done responding with
    // the queried data
    reaper_tx.send(TrackMsg::Barrier(barrier)).unwrap();

    // v1m messages should still be blocked...
    v1m_tx
        .send(UpstreamMsg::ChannelFader(ChannelFaderMsg {
            idx: 0,
            value: 0.75,
        }))
        .unwrap();
    assert_no_message!(to_reaper_rx, 1);

    // Now we should see the barrier forwarded to v1m indicating the mode transition is complete
    let barrier = drain_downstream_until_barrier(&to_v1m_rx);
    assert!(
        barrier.unwrap().from == Mode::ReaperVolPan && barrier.unwrap().to == Mode::ReaperSends,
        "Barrier should indicate transition from VolPan to Sends"
    );

    // v1m messages should still be blocked...
    v1m_tx
        .send(UpstreamMsg::ChannelFader(ChannelFaderMsg {
            idx: 0,
            value: 0.75,
        }))
        .unwrap();
    assert_no_message!(to_reaper_rx, 1);

    // Once v1m reflects the barrier, the mode transition should be complete and messages should flow again
    v1m_tx.send(UpstreamMsg::Barrier(barrier.unwrap())).unwrap();

    // Messages should now be forwarded
    v1m_tx
        .send(UpstreamMsg::ChannelFader(ChannelFaderMsg {
            idx: 1,
            value: 0.75,
        }))
        .unwrap();
    drain_and_print(&to_reaper_rx);
    assert_upstream_send_level_track_msg!(to_reaper_rx, &track2_guid, 1, 0.75);
}

#[test]
fn test_mode_transition_sends_to_vol_pan_initiated_by_hardware() {
    let (reaper_tx, to_reaper_rx, v1m_tx, to_v1m_rx) = setup_mode_manager_channels();

    // We start in VolPan mode. We need to get into Sends mode first before we can test transitioning back to VolPan, so we'll repeat some of the same steps as the previous test to get there.

    // Register two tracks and select a track
    let track1_guid = Uuid::new_v4();
    let track2_guid = Uuid::new_v4();
    reaper_tx
        .send(
            track::ReaperTrackIndex {
                track_guid: track1_guid,
                track_index: Some(1),
            }
            .into(),
        )
        .unwrap();
    reaper_tx
        .send(
            track::ReaperTrackIndex {
                track_guid: track2_guid,
                track_index: Some(2),
            }
            .into(),
        )
        .unwrap();
    reaper_tx
        .send(
            track::Selected {
                track_guid: track1_guid,
                selected: true,
            }
            .into(),
        )
        .unwrap();
    reaper_tx
        .send(
            track::SendIndex {
                track_guid: track1_guid,
                send_index: 1,
                send_guid: track2_guid,
            }
            .into(),
        )
        .unwrap();

    // TODO: is there a better way to do this than sleeping?
    std::thread::sleep(Duration::from_millis(1));

    // Initiate mode transition
    v1m_tx.send(UpstreamMsg::MIDITracksPress {}).unwrap();

    // Swallow the track data query message
    to_reaper_rx.recv().unwrap();

    let barrier = assert_upstream_barrier_msg!(to_reaper_rx, Mode::ReaperVolPan, Mode::ReaperSends);

    // Mock the response to the track data query
    reaper_tx
        .send(
            track::SendIndex {
                track_guid: track1_guid,
                send_index: 1,
                send_guid: track2_guid,
            }
            .into(),
        )
        .unwrap();

    // Reflect the barrier back from the reaper side indicating that we are done responding with
    // the queried data
    reaper_tx.send(TrackMsg::Barrier(barrier)).unwrap();

    // Now we should see the barrier forwarded to v1m indicating the mode transition is complete
    let barrier = drain_downstream_until_barrier(&to_v1m_rx);
    assert!(
        barrier.is_some(),
        "Should receive barrier message indicating mode transition is complete"
    );
    assert!(
        barrier.unwrap().from == Mode::ReaperVolPan && barrier.unwrap().to == Mode::ReaperSends,
        "Barrier should indicate transition from VolPan to Sends"
    );

    // Once v1m reflects the barrier, the mode transition should be complete and messages should flow again
    v1m_tx.send(UpstreamMsg::Barrier(barrier.unwrap())).unwrap();

    // Messages should now be forwarded
    v1m_tx
        .send(UpstreamMsg::ChannelFader(ChannelFaderMsg {
            idx: 1,
            value: 0.75,
        }))
        .unwrap();
    drain_and_print(&to_reaper_rx);
    assert_upstream_send_level_track_msg!(to_reaper_rx, &track2_guid, 1, 0.75);

    // Now we are in Sends mode. Transition back to VolPan.
    v1m_tx.send(UpstreamMsg::GlobalPress {}).unwrap();

    // Swallow the track data query messages (one per track)
    to_reaper_rx.recv().unwrap();
    to_reaper_rx.recv().unwrap();

    let barrier = assert_upstream_barrier_msg!(to_reaper_rx, Mode::ReaperSends, Mode::ReaperVolPan);

    // Reflect the barrier back from the reaper side indicating that we are done responding with
    // the queried data
    reaper_tx.send(TrackMsg::Barrier(barrier)).unwrap();

    // Now we should see the barrier forwarded to v1m indicating the mode transition is complete
    let barrier = drain_downstream_until_barrier(&to_v1m_rx);
    assert!(
        barrier.unwrap().from == Mode::ReaperSends && barrier.unwrap().to == Mode::ReaperVolPan,
        "Barrier should indicate transition from VolPan to Sends"
    );

    // Once v1m reflects the barrier, the mode transition should be complete and messages should flow again
    v1m_tx.send(UpstreamMsg::Barrier(barrier.unwrap())).unwrap();

    // Transition complete; messages should now be forwarded to the correct track parameter again
    v1m_tx
        .send(UpstreamMsg::ChannelFader(ChannelFaderMsg {
            idx: 0,
            value: 0.11,
        }))
        .unwrap();
    v1m_tx
        .send(UpstreamMsg::ChannelFader(ChannelFaderMsg {
            idx: 1,
            value: 0.33,
        }))
        .unwrap();
    assert_upstream_volume_track_msg!(to_reaper_rx, &track1_guid, 0.11);
    assert_upstream_volume_track_msg!(to_reaper_rx, &track2_guid, 0.33);
}
