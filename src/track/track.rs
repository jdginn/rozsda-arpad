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
pub enum DataPayload {
    Name(String),
    ReaperTrackIndex(Option<i32>),
    Selected(bool),
    Muted(bool),
    Soloed(bool),
    Armed(bool),
    Volume(f32),
    Pan(f32),
    TrackData(TrackData),
}

/// Maintains stae for a given track to the best of our knowledge
#[derive(Clone)]
struct TrackData {
    guid: String,
    name: String,
    reaper_track_index: Option<i32>,
    selected: bool,
    muted: bool,
    soloed: bool,
    armed: bool,
    volume: f32,
    pan: f32,
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
        }
    }
}

pub struct TrackManager {
    tracks: HashMap<String, TrackData>,
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
        while let Ok(msg) = self.input.try_recv() {
            match msg {
                TrackMsg::Barrier(barrier) => {
                    self.downstream
                        .send(TrackMsg::Barrier(barrier.clone()))
                        .unwrap();
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
