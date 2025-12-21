use arpad_rust::track::track::{
    DataPayload, Direction, SendIndex, SendLevel, TrackDataMsg, TrackManager, TrackMsg,
    TrackQuery,
};
use arpad_rust::modes::mode_manager::Barrier;
use crossbeam_channel::{bounded, Receiver, Sender};
use std::time::Duration;

/// Helper to create a test TrackManager setup with channels
fn setup_track_manager() -> (
    Sender<TrackMsg>,
    Receiver<TrackMsg>,
    Receiver<TrackMsg>,
) {
    let (input_tx, input_rx) = bounded(128);
    let (upstream_tx, upstream_rx) = bounded(128);
    let (downstream_tx, downstream_rx) = bounded(128);

    TrackManager::start(input_rx, upstream_tx, downstream_tx);

    // Give the thread time to start
    std::thread::sleep(Duration::from_millis(50));

    (input_tx, upstream_rx, downstream_rx)
}

#[test]
fn test_track_manager_forwards_barriers() {
    let (input_tx, _upstream_rx, downstream_rx) = setup_track_manager();

    let barrier = Barrier::new();
    input_tx.send(TrackMsg::Barrier(barrier)).unwrap();

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
    let (input_tx, _upstream_rx, downstream_rx) = setup_track_manager();

    let test_guid = "test-track-guid-1".to_string();
    let test_name = "Test Track".to_string();

    input_tx
        .send(TrackMsg::TrackDataMsg(TrackDataMsg {
            guid: test_guid.clone(),
            direction: Direction::Downstream,
            data: DataPayload::Name(test_name.clone()),
        }))
        .unwrap();

    // Message should be forwarded downstream
    let result = downstream_rx.recv_timeout(Duration::from_millis(100));
    assert!(result.is_ok(), "Track name message should be forwarded");

    if let Ok(TrackMsg::TrackDataMsg(msg)) = result {
        assert_eq!(msg.guid, test_guid);
        if let DataPayload::Name(name) = msg.data {
            assert_eq!(name, test_name);
        } else {
            panic!("Expected Name payload");
        }
    } else {
        panic!("Expected TrackDataMsg");
    }
}

#[test]
fn test_track_manager_handles_track_volume() {
    let (input_tx, upstream_rx, _downstream_rx) = setup_track_manager();

    let test_guid = "test-track-guid-2".to_string();
    let test_volume = 0.75;

    input_tx
        .send(TrackMsg::TrackDataMsg(TrackDataMsg {
            guid: test_guid.clone(),
            direction: Direction::Upstream,
            data: DataPayload::Volume(test_volume),
        }))
        .unwrap();

    // Message should be forwarded upstream
    let result = upstream_rx.recv_timeout(Duration::from_millis(100));
    assert!(result.is_ok(), "Track volume message should be forwarded");

    if let Ok(TrackMsg::TrackDataMsg(msg)) = result {
        assert_eq!(msg.guid, test_guid);
        if let DataPayload::Volume(volume) = msg.data {
            assert_eq!(volume, test_volume);
        } else {
            panic!("Expected Volume payload");
        }
    } else {
        panic!("Expected TrackDataMsg");
    }
}

#[test]
fn test_track_manager_responds_to_track_query() {
    let (input_tx, upstream_rx, downstream_rx) = setup_track_manager();

    let test_guid = "test-track-guid-3".to_string();

    // First, populate some track data
    input_tx
        .send(TrackMsg::TrackDataMsg(TrackDataMsg {
            guid: test_guid.clone(),
            direction: Direction::Downstream,
            data: DataPayload::Name("Populated Track".to_string()),
        }))
        .unwrap();

    // Consume the forwarded message
    let _ = downstream_rx.recv_timeout(Duration::from_millis(100));

    // Now query the track
    input_tx
        .send(TrackMsg::TrackQuery(TrackQuery {
            guid: test_guid.clone(),
            direction: Direction::Upstream,
        }))
        .unwrap();

    // Should receive a TrackData response upstream
    let result = upstream_rx.recv_timeout(Duration::from_millis(100));
    assert!(result.is_ok(), "TrackQuery should receive a response");

    if let Ok(TrackMsg::TrackDataMsg(msg)) = result {
        assert_eq!(msg.guid, test_guid);
        if let DataPayload::TrackData(track_data) = msg.data {
            // Verify track data contains our populated name
            // Note: We can't directly access TrackData fields as they're private,
            // but we can verify the message type is correct
            println!("Successfully received TrackData response");
        } else {
            panic!("Expected TrackData payload in response to query");
        }
    } else {
        panic!("Expected TrackDataMsg in response to query");
    }
}

#[test]
fn test_track_manager_handles_send_data() {
    let (input_tx, _upstream_rx, downstream_rx) = setup_track_manager();

    let test_guid = "test-track-guid-4".to_string();
    let send_index = 2;
    let target_guid = "target-track-guid".to_string();

    // Set send index (maps send to target track)
    input_tx
        .send(TrackMsg::TrackDataMsg(TrackDataMsg {
            guid: test_guid.clone(),
            direction: Direction::Downstream,
            data: DataPayload::SendIndex(SendIndex {
                send_index,
                guid: target_guid.clone(),
            }),
        }))
        .unwrap();

    // Consume the forwarded message
    let _ = downstream_rx.recv_timeout(Duration::from_millis(100));

    // Set send level
    let send_level = 0.8;
    input_tx
        .send(TrackMsg::TrackDataMsg(TrackDataMsg {
            guid: test_guid.clone(),
            direction: Direction::Downstream,
            data: DataPayload::SendLevel(SendLevel {
                send_index,
                level: send_level,
            }),
        }))
        .unwrap();

    // Message should be forwarded
    let result = downstream_rx.recv_timeout(Duration::from_millis(100));
    assert!(result.is_ok(), "Send level message should be forwarded");
}

#[test]
fn test_track_manager_message_ordering() {
    // Test that messages are processed in the order they're sent
    let (input_tx, _upstream_rx, downstream_rx) = setup_track_manager();

    let test_guid = "test-track-ordering".to_string();

    // Send multiple messages in sequence
    let messages = vec![
        DataPayload::Name("Track 1".to_string()),
        DataPayload::Volume(0.5),
        DataPayload::Pan(0.2),
        DataPayload::Muted(true),
    ];

    for payload in messages.iter() {
        input_tx
            .send(TrackMsg::TrackDataMsg(TrackDataMsg {
                guid: test_guid.clone(),
                direction: Direction::Downstream,
                data: payload.clone(),
            }))
            .unwrap();
    }

    // Verify messages are received in order
    for (idx, expected_payload) in messages.iter().enumerate() {
        let result = downstream_rx.recv_timeout(Duration::from_millis(100));
        assert!(
            result.is_ok(),
            "Message {} should be received in order",
            idx
        );

        if let Ok(TrackMsg::TrackDataMsg(msg)) = result {
            // Verify the message type matches
            match (expected_payload, &msg.data) {
                (DataPayload::Name(_), DataPayload::Name(_)) => {}
                (DataPayload::Volume(_), DataPayload::Volume(_)) => {}
                (DataPayload::Pan(_), DataPayload::Pan(_)) => {}
                (DataPayload::Muted(_), DataPayload::Muted(_)) => {}
                _ => panic!("Message type mismatch at position {}", idx),
            }
        }
    }
}

#[test]
fn test_track_manager_concurrent_tracks() {
    // Test that TrackManager can handle messages for multiple tracks concurrently
    let (input_tx, _upstream_rx, downstream_rx) = setup_track_manager();

    let track1 = "track-1".to_string();
    let track2 = "track-2".to_string();
    let track3 = "track-3".to_string();

    // Send messages for multiple tracks
    input_tx
        .send(TrackMsg::TrackDataMsg(TrackDataMsg {
            guid: track1.clone(),
            direction: Direction::Downstream,
            data: DataPayload::Name("Track 1".to_string()),
        }))
        .unwrap();

    input_tx
        .send(TrackMsg::TrackDataMsg(TrackDataMsg {
            guid: track2.clone(),
            direction: Direction::Downstream,
            data: DataPayload::Name("Track 2".to_string()),
        }))
        .unwrap();

    input_tx
        .send(TrackMsg::TrackDataMsg(TrackDataMsg {
            guid: track3.clone(),
            direction: Direction::Downstream,
            data: DataPayload::Name("Track 3".to_string()),
        }))
        .unwrap();

    // All messages should be forwarded
    for i in 0..3 {
        let result = downstream_rx.recv_timeout(Duration::from_millis(100));
        assert!(
            result.is_ok(),
            "Message {} for concurrent tracks should be forwarded",
            i
        );
    }
}

// TODO: Test edge case where TrackQuery is sent for a track that doesn't exist
// Unclear behavior: Should it return an empty TrackData, return nothing, or error?
#[test]
#[ignore] // Ignoring until behavior is clarified
fn test_track_manager_query_nonexistent_track() {
    let (input_tx, upstream_rx, _downstream_rx) = setup_track_manager();

    let nonexistent_guid = "nonexistent-track".to_string();

    input_tx
        .send(TrackMsg::TrackQuery(TrackQuery {
            guid: nonexistent_guid.clone(),
            direction: Direction::Upstream,
        }))
        .unwrap();

    // TODO: What should happen here?
    // Option 1: Receive nothing (timeout)
    // Option 2: Receive a TrackData with default/empty values
    // Option 3: Receive an error message
    let result = upstream_rx.recv_timeout(Duration::from_millis(100));
    
    // Currently, the implementation doesn't send anything if track doesn't exist
    assert!(result.is_err(), "Query for nonexistent track currently returns nothing");
}
