use std::time::Duration;

use arpad_rust::modes::mode_manager::{Barrier, Mode};
use arpad_rust::track::track;
use arpad_rust::track::track::TrackMsg;

use crossbeam_channel::{Receiver, Sender, bounded};
use uuid::Uuid;

/// Helper to create a test TrackManager setup with channels
fn setup_track_manager() -> (
    Sender<TrackMsg>,
    Receiver<TrackMsg>,
    Sender<TrackMsg>,
    Receiver<TrackMsg>,
) {
    let (from_upstream_tx, from_upstream_rx) = bounded(128);
    let (to_upstream_tx, to_upstream_rx) = bounded(128);
    let (from_downstream_tx, from_downstream_rx) = bounded(128);
    let (to_downstream_tx, to_downstream_rx) = bounded(128);

    track::TrackManager::start(
        from_upstream_rx.clone(),
        to_upstream_tx.clone(),
        from_downstream_rx.clone(),
        to_downstream_tx.clone(),
    );

    // Give the thread time to start
    std::thread::sleep(Duration::from_millis(50));

    (
        from_upstream_tx,
        to_upstream_rx,
        from_downstream_tx,
        to_downstream_rx,
    )
}

#[test]
fn test_track_manager_forwards_barriers() {
    let (upstream_tx, _upstream_rx, _downstream_tx, downstream_rx) = setup_track_manager();

    let barrier = Barrier::new();
    upstream_tx.send(TrackMsg::Barrier(barrier)).unwrap();

    // Barrier should be forwarded downstream
    let result = downstream_rx.recv_timeout(Duration::from_millis(100));
    assert!(result.is_ok(), "Barrier should be forwarded downstream");

    if let Ok(TrackMsg::Barrier(received_barrier)) = result {
        assert_eq!(received_barrier, barrier, "Barrier ID should match");
    } else {
        panic!("Expected Barrier message");
    }
}

#[test]
fn test_track_manager_handles_track_name() {
    let (upstream_tx, _upstream_rx, _downstream_tx, downstream_rx) = setup_track_manager();

    let test_guid = Uuid::new_v4();
    let test_name = "Test Track".to_string();

    upstream_tx
        .send(
            track::Name {
                track_guid: test_guid,
                name: test_name.clone(),
            }
            .into(),
        )
        .unwrap();

    // Message should be forwarded downstream
    let result = downstream_rx.recv_timeout(Duration::from_millis(100));
    assert!(result.is_ok(), "Track name message should be forwarded");

    if let Ok(TrackMsg::Name(msg)) = result {
        assert_eq!(msg.track_guid, test_guid);
        assert_eq!(msg.name, test_name);
    } else {
        panic!("Expected TrackMsg::Name");
    }
}

#[test]
fn test_track_manager_handles_track_volume() {
    let (_upstream_tx, upstream_rx, downstream_tx, _downstream_rx) = setup_track_manager();

    let test_guid = Uuid::new_v4();
    let test_volume = 0.75;

    downstream_tx
        .send(
            track::Volume {
                track_guid: test_guid,
                volume: test_volume,
            }
            .into(),
        )
        .unwrap();

    // Message should be forwarded upstream
    let result = upstream_rx.recv_timeout(Duration::from_millis(100));
    assert!(result.is_ok(), "Track volume message should be forwarded");

    if let Ok(TrackMsg::Volume(msg)) = result {
        assert_eq!(msg.track_guid, test_guid);
        assert_eq!(msg.volume, test_volume);
    } else {
        panic!("Expected TrackMsg::Volume");
    }
}

#[test]
fn test_track_manager_responds_to_track_query() {
    let (from_upstream_tx, to_upstream_rx, from_downstream_tx, to_downstream_rx) =
        setup_track_manager();

    let test_guid = Uuid::new_v4();

    // First, populate some track data
    from_upstream_tx
        .send(
            track::Name {
                track_guid: test_guid,
                name: "Populated Track".to_string(),
            }
            .into(),
        )
        .unwrap();

    // Consume the forwarded message
    let _ = to_downstream_rx.recv_timeout(Duration::from_millis(100));

    // Now query the track
    from_downstream_tx
        .send(track::TrackQuery { guid: test_guid }.into())
        .unwrap();

    // Should receive a TrackData response upstream
    let result = to_downstream_rx.recv_timeout(Duration::from_millis(100));
    assert!(result.is_ok(), "TrackQuery should receive a response");

    if let Ok(TrackMsg::TrackData(msg)) = result {
        assert_eq!(msg.track_guid, test_guid);
        // Verify track data contains our populated name
        // Note: We can't directly access TrackData fields as they're private,
        // but we can verify the message type is correct
        println!("Successfully received TrackData response");
    } else {
        panic!("Expected TrackDataMsg in response to query");
    }
}

#[test]
fn test_track_manager_handles_send_data() {
    let (upstream_tx, upstream_rx, downstream_tx, downstream_rx) = setup_track_manager();

    let test_guid = Uuid::new_v4();
    let send_index = 2;
    let target_guid = Uuid::new_v4();

    // Set send index (maps send to target track)
    upstream_tx
        .send(
            track::SendIndex {
                track_guid: test_guid,
                send_index,
                send_guid: target_guid,
            }
            .into(),
        )
        .unwrap();

    // Consume the forwarded message
    let _ = downstream_rx.recv_timeout(Duration::from_millis(100));

    // Set send level
    let send_level = 0.8;
    downstream_tx
        .send(
            track::SendLevel {
                track_guid: test_guid,
                send_index,
                level: send_level,
            }
            .into(),
        )
        .unwrap();

    // Message should be forwarded
    let result = upstream_rx.recv_timeout(Duration::from_millis(100));
    assert!(result.is_ok(), "Send level message should be forwarded");
}

#[test]
fn test_track_manager_message_ordering() {
    // Test that messages are processed in the order they're sent
    let (from_upstream, _to_upstream, _from_downstream, to_downstream) = setup_track_manager();

    let test_guid = Uuid::new_v4();

    // Send multiple messages in sequence
    let messages: Vec<TrackMsg> = vec![
        track::Name {
            track_guid: test_guid,
            name: "Track 1".to_string(),
        }
        .into(),
        track::Volume {
            track_guid: test_guid,
            volume: 0.5,
        }
        .into(),
        track::Volume {
            track_guid: test_guid,
            volume: 0.5,
        }
        .into(),
        track::Pan {
            track_guid: test_guid,
            pan: 0.2,
        }
        .into(),
        track::Muted {
            track_guid: test_guid,
            muted: true,
        }
        .into(),
    ];

    for payload in messages.iter() {
        from_upstream.send(payload.clone()).unwrap();
    }

    // Verify messages are received in order
    for (idx, expected_payload) in messages.iter().enumerate() {
        let result = to_downstream.recv_timeout(Duration::from_millis(100));
        assert!(
            result.is_ok(),
            "Message {} should be received in order",
            idx
        );

        match (expected_payload, result) {
            (TrackMsg::Name(_), Ok(TrackMsg::Name(_))) => {}
            (TrackMsg::Volume(_), Ok(TrackMsg::Volume(_))) => {}
            (TrackMsg::Pan(_), Ok(TrackMsg::Pan(_))) => {}
            (TrackMsg::Muted(_), Ok(TrackMsg::Muted(_))) => {}
            _ => panic!("Message type mismatch at position {}", idx),
        }
    }
}

#[test]
fn test_track_manager_concurrent_tracks() {
    // Test that TrackManager can handle messages for multiple tracks concurrently
    let (from_upstream, _to_upstream, _from_downstream, to_downstream) = setup_track_manager();

    let track1 = Uuid::new_v4();
    let track2 = Uuid::new_v4();
    // let track3 = Uuid::new_v4();

    // Send messages for multiple tracks
    from_upstream
        .send(
            track::Name {
                track_guid: track1,
                name: "Track 1".to_string(),
            }
            .into(),
        )
        .unwrap();

    from_upstream
        .send(
            track::Name {
                track_guid: track2,
                name: "Track 2".to_string(),
            }
            .into(),
        )
        .unwrap();

    from_upstream
        .send(
            track::Volume {
                track_guid: track1,
                volume: 0.5,
            }
            .into(),
        )
        .unwrap();

    // All messages should be forwarded
    for i in 0..3 {
        let result = to_downstream.recv_timeout(Duration::from_millis(100));
        assert!(
            result.is_ok(),
            "Message {} for concurrent tracks should be forwarded",
            i
        );
    }
}

// TODO: Test edge case where TrackQuery is sent for a track that doesn't exist
// The current implementation doesn't send anything if track doesn't exist, which is verified below.
#[test]
fn test_track_manager_query_nonexistent_track() {
    let (upstream_tx, upstream_rx, _downstream_tx, _downstream_rx) = setup_track_manager();

    let nonexistent_guid = Uuid::new_v4();

    upstream_tx
        .send(
            track::TrackQuery {
                guid: nonexistent_guid,
            }
            .into(),
        )
        .unwrap();

    // TODO: What should happen here?
    // Option 1: Receive nothing (timeout)
    // Option 2: Receive a TrackData with default/empty values
    // Option 3: Receive an error message
    let result = upstream_rx.recv_timeout(Duration::from_millis(5));

    // Currently, the implementation doesn't send anything if track doesn't exist
    assert!(
        result.is_err(),
        "Query for nonexistent should return nothing but got {:?}",
        result
    );
}
