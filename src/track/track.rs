use std::collections::HashMap;
use std::thread;

use crossbeam_channel::{Receiver, Sender};

use crate::modes::mode_manager::Barrier;

// TODO: probably instead of having direction, make an enum of separate UpstreamTrackMsg and DownstreamTrackMsg like we do for XTouch? That seems cleaner
#[derive(Clone, Debug, PartialEq, Eq)]
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
pub struct FXName {
    pub fx_index: i32,
    pub name: String,
}

#[derive(Clone)]
pub struct FXGuid {
    pub fx_index: i32,
    pub guid: String,
}

#[derive(Clone)]
pub struct FXEnabled {
    pub fx_index: i32,
    pub enabled: bool,
}

#[derive(Clone)]
pub struct FXParamName {
    pub fx_index: i32,
    pub param_index: i32,
    pub name: String,
}

#[derive(Clone)]
pub struct FXParamValue {
    pub fx_index: i32,
    pub param_index: i32,
    pub value: f32,
}

#[derive(Clone)]
pub struct FXParamMin {
    pub fx_index: i32,
    pub param_index: i32,
    pub min: f32,
}

#[derive(Clone)]
pub struct FXParamMax {
    pub fx_index: i32,
    pub param_index: i32,
    pub max: f32,
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
    FXGuid(FXGuid),
    FXName(FXName),
    FXEnabled(FXEnabled),
    FXParamName(FXParamName),
    FXParamValue(FXParamValue),
    FXParamMin(FXParamMin),
    FXParamMax(FXParamMax),
    TrackData(TrackData),
}

#[derive(Clone)]
pub struct SendData {
    pub target_guid: String,
    pub send_index: i32,
    pub level: f32,
    pub pan: f32,
}

#[derive(Clone)]
pub struct FXData {
    pub fx_index: i32,
    pub guid: String,
    pub name: String,
    pub enabled: bool,
    pub params: Vec<FXParamData>,
}

impl FXData {
    fn get_param_data(&mut self, param_index: i32) -> Option<&mut FXParamData> {
        // Ensure the params vector is large enough
        while self.params.len() <= param_index as usize {
            self.params.push(FXParamData {
                param_index: self.params.len() as i32,
                value: 0.0,
                min: 0.0,
                max: 1.0,
            });
        }
        self.params.get_mut(param_index as usize)
    }
}

#[derive(Clone)]
pub struct FXParamData {
    pub param_index: i32,
    pub value: f32,
    pub min: f32,
    pub max: f32,
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
    fx: Vec<FXData>,
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
            fx: Vec::new(),
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

    fn get_fx_data(&mut self, fx_index: i32) -> Option<&mut FXData> {
        // Ensure the fx vector is large enough
        while self.fx.len() <= fx_index as usize {
            self.fx.push(FXData {
                guid: String::new(),
                fx_index: self.fx.len() as i32,
                name: String::new(),
                enabled: false,
                params: Vec::new(),
            });
        }
        self.fx.get_mut(fx_index as usize)
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
                        DataPayload::FXGuid(fx_guid) => {
                            if let Some(fx) = track.get_fx_data(fx_guid.fx_index) {
                                fx.guid = fx_guid.guid.clone();
                                println!(
                                    "Track {} FX {} GUID set to {}",
                                    msg.guid, fx_guid.fx_index, fx_guid.guid
                                );
                            }
                        }
                        DataPayload::FXName(fx_name) => {
                            if let Some(fx) = track.get_fx_data(fx_name.fx_index) {
                                fx.name = fx_name.name.clone();
                                println!(
                                    "Track {} FX {} name set to {}",
                                    msg.guid, fx_name.fx_index, fx_name.name
                                );
                            }
                        }
                        DataPayload::FXEnabled(fx_enabled) => {
                            if let Some(fx) = track.get_fx_data(fx_enabled.fx_index) {
                                fx.enabled = fx_enabled.enabled;
                                println!(
                                    "Track {} FX {} enabled set to {}",
                                    msg.guid, fx_enabled.fx_index, fx_enabled.enabled
                                );
                            }
                        }
                        DataPayload::FXParamName(fx_param_name) => {
                            if let Some(fx) = track.get_fx_data(fx_param_name.fx_index) {
                                if let Some(param) = fx.get_param_data(fx_param_name.param_index) {
                                    // We don't store the name in FXParamData currently
                                    println!(
                                        "Track {} FX {} Param {} name set to {}",
                                        msg.guid,
                                        fx_param_name.fx_index,
                                        fx_param_name.param_index,
                                        fx_param_name.name
                                    );
                                }
                            }
                        }
                        DataPayload::FXParamValue(fx_param_value) => {
                            if let Some(fx) = track.get_fx_data(fx_param_value.fx_index) {
                                if let Some(param) = fx.get_param_data(fx_param_value.param_index) {
                                    param.value = fx_param_value.value;
                                    println!(
                                        "Track {} FX {} Param {} value set to {}",
                                        msg.guid,
                                        fx_param_value.fx_index,
                                        fx_param_value.param_index,
                                        fx_param_value.value
                                    );
                                }
                            }
                        }
                        DataPayload::FXParamMin(fx_param_min) => {
                            if let Some(fx) = track.get_fx_data(fx_param_min.fx_index) {
                                if let Some(param) = fx.get_param_data(fx_param_min.param_index) {
                                    param.min = fx_param_min.min;
                                    println!(
                                        "Track {} FX {} Param {} min set to {}",
                                        msg.guid,
                                        fx_param_min.fx_index,
                                        fx_param_min.param_index,
                                        fx_param_min.min
                                    );
                                }
                            }
                        }
                        DataPayload::FXParamMax(fx_param_max) => {
                            if let Some(fx) = track.get_fx_data(fx_param_max.fx_index) {
                                if let Some(param) = fx.get_param_data(fx_param_max.param_index) {
                                    param.max = fx_param_max.max;
                                    println!(
                                        "Track {} FX {} Param {} max set to {}",
                                        msg.guid,
                                        fx_param_max.fx_index,
                                        fx_param_max.param_index,
                                        fx_param_max.max
                                    );
                                }
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
