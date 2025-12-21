use std::collections::HashMap;
use std::thread;

use crossbeam_channel::{Receiver, Sender};

use crate::modes::mode_manager::Barrier;

// TODO: probably instead of having direction, make an enum of separate UpstreamTrackMsg and DownstreamTrackMsg like we do for XTouch? That seems cleaner
#[derive(Clone)]
pub enum Direction {
    Upstream,
    Downstream,
}

/// Set of messages that TrackManager can handle
#[derive(Clone)]
pub enum TrackMsg {
    Barrier(Barrier),
    TrackDataMsg(TrackDataMsg),
    TrackQuery(TrackQuery),
}

#[derive(Clone)]
pub struct TrackDataMsg {
    pub guid: String,
    pub direction: Direction,
    pub data: DataPayload,
}

#[derive(Clone)]
pub struct TrackQuery {
    pub guid: String,
    pub direction: Direction,
}

#[derive(Clone)]
pub struct SendIndex {
    pub send_index: i32,
    pub guid: String,
}

#[derive(Clone)]
pub struct SendLevel {
    pub send_index: i32,
    pub level: f32,
}

#[derive(Clone)]
pub struct SendPan {
    pub send_index: i32,
    pub pan: f32,
}

#[derive(Clone)]
pub enum DataPayload {
    Name(String),
    ReaperTrackIndex(Option<i32>),
    Selected(bool),
    Muted(bool),
    Soloed(bool),
    Armed(bool),
    Volume(f32),
    Pan(f32),
    SendIndex(SendIndex),
    SendLevel(SendLevel),
    SendPan(SendPan),
    TrackData(TrackData),
}

#[derive(Clone)]
pub struct SendData {
    pub target_guid: String,
    pub send_index: i32,
    pub level: f32,
    pub pan: f32,
}

/// Maintains state for a given track to the best of our knowledge
#[derive(Clone)]
pub struct TrackData {
    guid: String,
    name: String,
    reaper_track_index: Option<i32>,
    selected: bool,
    muted: bool,
    soloed: bool,
    armed: bool,
    volume: f32,
    pan: f32,
    sends: Vec<SendData>,
}

impl TrackData {
    fn new(guid: &str) -> Self {
        Self {
            guid: guid.to_string(),
            name: String::new(),
            reaper_track_index: None,
            selected: false,
            muted: false,
            soloed: false,
            armed: false,
            volume: 0.0,
            pan: 0.0,
            sends: Vec::new(),
        }
    }

    fn get_send_state(&mut self, index: i32) -> Option<&mut SendData> {
        self.sends.get_mut(index as usize)
    }

    fn set_send_index(&mut self, send_index: SendIndex) {
        // Ensure the sends vector is large enough
        while self.sends.len() <= send_index.send_index as usize {
            self.sends.push(SendData {
                target_guid: String::new(),
                send_index: self.sends.len() as i32,
                level: 0.0,
                pan: 0.0,
            });
        }
        self.sends[send_index.send_index as usize].target_guid = send_index.guid;
    }
}

pub struct TrackManager {
    tracks: HashMap<String, TrackData>,
    selected_track: Option<String>,
    input: Receiver<TrackMsg>,
    downstream: Sender<TrackMsg>,
    upstream: Sender<TrackMsg>,
}

impl TrackManager {
    pub fn start(
        input: Receiver<TrackMsg>,
        upstream: Sender<TrackMsg>,
        downstream: Sender<TrackMsg>,
    ) {
        thread::spawn(move || {
            let mut manager = Self {
                tracks: HashMap::new(),
                selected_track: None,
                input,
                downstream,
                upstream,
            };
            loop {
                manager.handle_messages();
            }
        });
    }

    pub fn handle_messages(&mut self) {
        while let Ok(msg) = self.input.recv() {
            match msg {
                TrackMsg::Barrier(barrier) => {
                    self.downstream.send(TrackMsg::Barrier(barrier)).unwrap();
                }
                TrackMsg::TrackDataMsg(msg) => {
                    let msg_cloned = msg.clone();
                    // If we've never seen this track before, create a new entry
                    let track = self
                        .tracks
                        .entry(msg.guid.to_string())
                        .or_insert_with(|| TrackData::new(&msg.guid));
                    // TODO: this really should also be forwarding all messages downstream as well
                    // as accumulating state internally
                    match msg.data {
                        DataPayload::Name(name) => {
                            track.name = name.clone();
                            println!("Track {} name set to {}", msg.guid, name);
                        }
                        DataPayload::ReaperTrackIndex(index) => {
                            track.reaper_track_index = index;
                            println!("Track {} Reaper index set to {:?}", msg.guid, index);
                        }
                        DataPayload::Selected(selected) => {
                            track.selected = selected;
                            if selected {
                                self.selected_track = Some(msg.guid.clone());
                            }
                            println!("Track {} selected set to {}", msg.guid, selected);
                        }
                        DataPayload::Muted(muted) => {
                            track.muted = muted;
                            println!("Track {} muted set to {}", msg.guid, muted);
                        }
                        DataPayload::Soloed(soloed) => {
                            track.soloed = soloed;
                            println!("Track {} soloed set to {}", msg.guid, soloed);
                        }
                        DataPayload::Armed(armed) => {
                            track.armed = armed;
                            println!("Track {} armed set to {}", msg.guid, armed);
                        }
                        DataPayload::Volume(volume) => {
                            track.volume = volume;
                            println!("Track {} volume set to {}", msg.guid, volume);
                        }
                        DataPayload::Pan(pan) => {
                            track.pan = pan;
                            println!("Track {} pan set to {}", msg.guid, pan);
                        }
                        // Update everything!
                        DataPayload::TrackData(track_data) => {
                            *track = track_data;
                        }
                        DataPayload::SendIndex(send_index) => {
                            track.set_send_index(send_index.clone());
                            println!(
                                "Track {} send {} target GUID set to {}",
                                msg.guid, send_index.send_index, send_index.guid
                            );
                        }
                        DataPayload::SendLevel(send_level) => {
                            if let Some(send) = track.get_send_state(send_level.send_index) {
                                send.level = send_level.level;
                                println!(
                                    "Track {} send {} level set to {}",
                                    msg.guid, send_level.send_index, send_level.level
                                );
                            }
                        }
                        DataPayload::SendPan(send_pan) => {
                            if let Some(send) = track.get_send_state(send_pan.send_index) {
                                send.pan = send_pan.pan;
                                println!(
                                    "Track {} send {} pan set to {}",
                                    msg.guid, send.send_index, send_pan.pan
                                );
                            }
                        }
                    }
                    // Forward the message to the appropriate place
                    match msg.direction {
                        Direction::Upstream => {
                            self.upstream
                                .send(TrackMsg::TrackDataMsg(msg_cloned))
                                .unwrap();
                        }
                        Direction::Downstream => {
                            self.downstream
                                .send(TrackMsg::TrackDataMsg(msg_cloned))
                                .unwrap();
                        }
                    }
                }
                TrackMsg::TrackQuery(msg) => match msg.direction {
                    // Respond with ALL of the current track data
                    Direction::Upstream => {
                        if let Some(track) = self.tracks.get(&msg.guid) {
                            let response = TrackMsg::TrackDataMsg(TrackDataMsg {
                                guid: msg.guid.clone(),
                                direction: Direction::Upstream, // Don't care?
                                data: DataPayload::TrackData(track.clone()),
                            });
                            self.upstream.send(response).unwrap();
                        }
                    }
                    Direction::Downstream => {
                        if let Some(track) = self.tracks.get(&msg.guid) {
                            let response = TrackMsg::TrackDataMsg(TrackDataMsg {
                                guid: msg.guid.clone(),
                                direction: Direction::Downstream, // Don't care?
                                data: DataPayload::TrackData(track.clone()),
                            });
                            self.downstream.send(response).unwrap();
                        }
                    }
                },
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests for TrackData

    #[test]
    fn test_track_data_new() {
        let track = TrackData::new("test-guid");
        assert_eq!(track.guid, "test-guid");
        assert_eq!(track.name, "");
        assert_eq!(track.reaper_track_index, None);
        assert_eq!(track.selected, false);
        assert_eq!(track.muted, false);
        assert_eq!(track.soloed, false);
        assert_eq!(track.armed, false);
        assert_eq!(track.volume, 0.0);
        assert_eq!(track.pan, 0.0);
        assert!(track.sends.is_empty());
    }

    #[test]
    fn test_set_send_index_creates_send() {
        let mut track = TrackData::new("test-guid");
        let send_index = SendIndex {
            send_index: 0,
            guid: "target-guid".to_string(),
        };
        
        track.set_send_index(send_index);
        
        assert_eq!(track.sends.len(), 1);
        assert_eq!(track.sends[0].target_guid, "target-guid");
        assert_eq!(track.sends[0].send_index, 0);
    }

    #[test]
    fn test_set_send_index_expands_vector() {
        let mut track = TrackData::new("test-guid");
        let send_index = SendIndex {
            send_index: 3,
            guid: "target-guid".to_string(),
        };
        
        track.set_send_index(send_index);
        
        // Should create sends at indices 0, 1, 2, and 3
        assert_eq!(track.sends.len(), 4);
        assert_eq!(track.sends[3].target_guid, "target-guid");
    }

    #[test]
    fn test_set_send_index_updates_existing() {
        let mut track = TrackData::new("test-guid");
        
        // Create first send
        track.set_send_index(SendIndex {
            send_index: 0,
            guid: "first-guid".to_string(),
        });
        
        // Update the same send
        track.set_send_index(SendIndex {
            send_index: 0,
            guid: "second-guid".to_string(),
        });
        
        assert_eq!(track.sends.len(), 1);
        assert_eq!(track.sends[0].target_guid, "second-guid");
    }

    #[test]
    fn test_get_send_state_returns_none_for_invalid_index() {
        let mut track = TrackData::new("test-guid");
        assert!(track.get_send_state(0).is_none());
    }

    #[test]
    fn test_get_send_state_returns_send() {
        let mut track = TrackData::new("test-guid");
        track.set_send_index(SendIndex {
            send_index: 0,
            guid: "target-guid".to_string(),
        });
        
        let send = track.get_send_state(0);
        assert!(send.is_some());
        assert_eq!(send.unwrap().target_guid, "target-guid");
    }

    #[test]
    fn test_get_send_state_allows_mutation() {
        let mut track = TrackData::new("test-guid");
        track.set_send_index(SendIndex {
            send_index: 0,
            guid: "target-guid".to_string(),
        });
        
        if let Some(send) = track.get_send_state(0) {
            send.level = 0.5;
            send.pan = 0.25;
        }
        
        let send = track.get_send_state(0).unwrap();
        assert_eq!(send.level, 0.5);
        assert_eq!(send.pan, 0.25);
    }

    // Tests for SendData

    #[test]
    fn test_send_data_clone() {
        let send = SendData {
            target_guid: "test-guid".to_string(),
            send_index: 1,
            level: 0.7,
            pan: 0.3,
        };
        
        let cloned = send.clone();
        assert_eq!(cloned.target_guid, send.target_guid);
        assert_eq!(cloned.send_index, send.send_index);
        assert_eq!(cloned.level, send.level);
        assert_eq!(cloned.pan, send.pan);
    }

    // Tests for message structures

    #[test]
    fn test_track_data_msg_clone() {
        let msg = TrackDataMsg {
            guid: "test-guid".to_string(),
            direction: Direction::Upstream,
            data: DataPayload::Volume(0.5),
        };
        
        let cloned = msg.clone();
        assert_eq!(cloned.guid, "test-guid");
    }

    #[test]
    fn test_track_query_creation() {
        let query = TrackQuery {
            guid: "test-guid".to_string(),
            direction: Direction::Downstream,
        };
        
        assert_eq!(query.guid, "test-guid");
    }

    // NOTE: Testing TrackManager::start and handle_messages would require more complex
    // integration testing with actual channels and thread synchronization. These tests
    // would be better suited for integration tests rather than unit tests, as they
    // involve spawning threads and channel communication across thread boundaries.
    //
    // For now, we've focused on testing the data structures and helper methods
    // that can be tested in isolation.
}
