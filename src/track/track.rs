use std::collections::HashMap;
use std::thread;

use crossbeam_channel::{Receiver, Sender, select};
use uuid::Uuid;

use coalescible_derive::{Coalescible, CoalescibleEnum};
use derive_enum_from::EnumFrom;

use crate::{modes::mode_manager::Barrier, track};

/// Set of messages that TrackManager can handle
#[derive(Clone, Debug, EnumFrom)]
pub enum TrackMsg {
    Barrier(Barrier),
    QueryAll,
    Query(TrackQuery),
    Delete(Delete),
    Name(Name),
    ReaperTrackIndex(ReaperTrackIndex),
    Selected(Selected),
    Muted(Muted),
    Soloed(Soloed),
    Armed(Armed),
    Volume(Volume),
    Pan(Pan),
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

// DataMsg doesn't need From impls because we expect to only be converting structs into TrackMsg.
// DataMsg exists to let us inspect whether a TrackMsg falls into this category of messages.
#[derive(Clone, Debug)]
pub enum DataMsg {
    Delete(Delete),
    Name(Name),
    ReaperTrackIndex(ReaperTrackIndex),
    Selected(Selected),
    Muted(Muted),
    Soloed(Soloed),
    Armed(Armed),
    Volume(Volume),
    Pan(Pan),
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

impl TryFrom<TrackMsg> for DataMsg {
    type Error = TrackMsg; // “give me back what you couldn’t convert”

    fn try_from(msg: TrackMsg) -> Result<Self, Self::Error> {
        match msg {
            TrackMsg::Delete(x) => Ok(DataMsg::Delete(x)),
            TrackMsg::Name(x) => Ok(DataMsg::Name(x)),
            TrackMsg::ReaperTrackIndex(x) => Ok(DataMsg::ReaperTrackIndex(x)),
            TrackMsg::Selected(x) => Ok(DataMsg::Selected(x)),
            TrackMsg::Muted(x) => Ok(DataMsg::Muted(x)),
            TrackMsg::Soloed(x) => Ok(DataMsg::Soloed(x)),
            TrackMsg::Armed(x) => Ok(DataMsg::Armed(x)),
            TrackMsg::Volume(x) => Ok(DataMsg::Volume(x)),
            TrackMsg::Pan(x) => Ok(DataMsg::Pan(x)),
            TrackMsg::SendIndex(x) => Ok(DataMsg::SendIndex(x)),
            TrackMsg::SendLevel(x) => Ok(DataMsg::SendLevel(x)),
            TrackMsg::SendPan(x) => Ok(DataMsg::SendPan(x)),
            TrackMsg::FXGuid(x) => Ok(DataMsg::FXGuid(x)),
            TrackMsg::FXName(x) => Ok(DataMsg::FXName(x)),
            TrackMsg::FXEnabled(x) => Ok(DataMsg::FXEnabled(x)),
            TrackMsg::FXParamName(x) => Ok(DataMsg::FXParamName(x)),
            TrackMsg::FXParamValue(x) => Ok(DataMsg::FXParamValue(x)),
            TrackMsg::FXParamMin(x) => Ok(DataMsg::FXParamMin(x)),
            TrackMsg::FXParamMax(x) => Ok(DataMsg::FXParamMax(x)),
            TrackMsg::TrackData(x) => Ok(DataMsg::TrackData(x)),

            other @ (TrackMsg::Barrier(_) | TrackMsg::Query(_) | TrackMsg::QueryAll) => Err(other),
        }
    }
}

#[derive(Clone, Debug)]
pub struct QueryAll {}

#[derive(Copy, Clone, Debug)]
pub struct TrackQuery {
    pub guid: Uuid,
}

#[derive(Clone, Debug, Coalescible)]
pub struct Delete {
    pub guid: Uuid,
}

#[derive(Clone, Debug, Coalescible)]
pub struct Name {
    pub track_guid: Uuid,
    #[data]
    pub name: String,
}

#[derive(Copy, Clone, Debug, Coalescible)]
pub struct ReaperTrackIndex {
    pub track_guid: Uuid,
    #[data]
    pub track_index: Option<i32>,
}

#[derive(Copy, Clone, Debug, Coalescible)]
pub struct Selected {
    pub track_guid: Uuid,
    #[data]
    pub selected: bool,
}

#[derive(Copy, Clone, Debug, Coalescible)]
pub struct Muted {
    pub track_guid: Uuid,
    #[data]
    pub muted: bool,
}

#[derive(Copy, Clone, Debug, Coalescible)]
pub struct Soloed {
    pub track_guid: Uuid,
    #[data]
    pub soloed: bool,
}

#[derive(Copy, Clone, Debug, Coalescible)]
pub struct Armed {
    pub track_guid: Uuid,
    #[data]
    pub armed: bool,
}

#[derive(Copy, Clone, Debug, Coalescible)]
pub struct Volume {
    pub track_guid: Uuid,
    #[data]
    pub volume: f32,
}

#[derive(Copy, Clone, Debug, Coalescible)]
pub struct Pan {
    pub track_guid: Uuid,
    #[data]
    pub pan: f32,
}

#[derive(Copy, Clone, Debug, Coalescible)]
pub struct SendIndex {
    pub track_guid: Uuid,
    pub send_index: i32,
    #[data]
    pub send_guid: Uuid,
}

#[derive(Copy, Clone, Debug, Coalescible)]
pub struct SendLevel {
    pub track_guid: Uuid,
    pub send_index: i32,
    #[data]
    pub level: f32,
}

#[derive(Copy, Clone, Debug, Coalescible)]
pub struct SendPan {
    pub track_guid: Uuid,
    pub send_index: i32,
    #[data]
    pub pan: f32,
}

#[derive(Clone, Debug, Coalescible)]
pub struct FXName {
    pub track_guid: Uuid,
    pub fx_index: i32,
    #[data]
    pub name: String,
}

#[derive(Copy, Clone, Debug, Coalescible)]
pub struct FXGuid {
    pub track_guid: Uuid,
    pub fx_index: i32,
    #[data]
    pub guid: Uuid,
}

#[derive(Copy, Clone, Debug, Coalescible)]
pub struct FXEnabled {
    pub track_guid: Uuid,
    pub fx_index: i32,
    #[data]
    pub enabled: bool,
}

#[derive(Clone, Debug, Coalescible)]
pub struct FXParamName {
    pub track_guid: Uuid,
    pub fx_index: i32,
    pub param_index: i32,
    #[data]
    pub name: String,
}

#[derive(Copy, Clone, Debug, Coalescible)]
pub struct FXParamValue {
    pub track_guid: Uuid,
    pub fx_index: i32,
    pub param_index: i32,
    #[data]
    pub value: f32,
}

#[derive(Copy, Clone, Debug, Coalescible)]
pub struct FXParamMin {
    pub track_guid: Uuid,
    pub fx_index: i32,
    pub param_index: i32,
    #[data]
    pub min: f32,
}

#[derive(Copy, Clone, Debug, Coalescible)]
pub struct FXParamMax {
    pub track_guid: Uuid,
    pub fx_index: i32,
    pub param_index: i32,
    #[data]
    pub max: f32,
}

#[derive(Copy, Clone, Debug, Coalescible)]
pub struct SendData {
    pub track_guid: Uuid,
    pub target_guid: Uuid,
    pub send_index: i32,
    #[data]
    pub level: f32,
    #[data]
    pub pan: f32,
}

#[derive(Clone, Debug, Coalescible)]
pub struct FXData {
    pub track_guid: Uuid,
    pub fx_index: i32,
    #[data]
    pub guid: Uuid,
    #[data]
    pub name: String,
    #[data]
    pub enabled: bool,
    #[data]
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

#[derive(Clone, Debug)]
pub struct FXParamData {
    pub param_index: i32,
    pub value: f32,
    pub min: f32,
    pub max: f32,
}

/// Maintains state for a given track to the best of our knowledge
#[derive(Clone, Debug)]
pub struct TrackData {
    pub track_guid: Uuid,
    pub name: String,
    pub reaper_track_index: Option<i32>,
    pub selected: bool,
    pub muted: bool,
    pub soloed: bool,
    pub armed: bool,
    pub volume: f32,
    pub pan: f32,
    pub sends: Vec<SendData>,
    pub fx: Vec<FXData>,
}

impl TrackData {
    fn new(guid: Uuid) -> Self {
        Self {
            track_guid: guid,
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

    fn set_send_index(&mut self, msg: SendIndex) {
        // Ensure the sends vector is large enough
        while self.sends.len() <= msg.send_index as usize {
            self.sends.push(SendData {
                track_guid: self.track_guid,
                target_guid: msg.send_guid,
                send_index: msg.send_index as i32,
                level: 0.0,
                pan: 0.0,
            });
        }
        self.sends[msg.send_index as usize].target_guid = msg.track_guid;
    }

    fn get_fx_data(&mut self, fx_index: i32) -> Option<&mut FXData> {
        // Ensure the fx vector is large enough
        while self.fx.len() <= fx_index as usize {
            self.fx.push(FXData {
                track_guid: self.track_guid,
                guid: Uuid::nil(),
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
    tracks: HashMap<Uuid, TrackData>,
    selected_track: Option<Uuid>,
    from_upstream: Receiver<TrackMsg>,
    to_upstream: Sender<TrackMsg>,
    from_downstream: Receiver<TrackMsg>,
    to_downstream: Sender<TrackMsg>,
}

impl TrackManager {
    fn send_queryall(&self) {
        let tracks = self.tracks.values().collect::<Vec<&TrackData>>();
        for track in &tracks {
            self.to_downstream
                .send(
                    ReaperTrackIndex {
                        track_guid: track.track_guid,
                        track_index: track.reaper_track_index,
                    }
                    .into(),
                )
                .unwrap();
        }
        for track in &tracks {
            self.to_downstream
                .send(
                    Volume {
                        track_guid: track.track_guid,
                        volume: track.volume,
                    }
                    .into(),
                )
                .unwrap();
        }
        for track in &tracks {
            self.to_downstream
                .send(
                    Pan {
                        track_guid: track.track_guid,
                        pan: track.pan,
                    }
                    .into(),
                )
                .unwrap();
        }
        for track in &tracks {
            self.to_downstream
                .send(
                    Name {
                        track_guid: track.track_guid,
                        name: track.name.clone(),
                    }
                    .into(),
                )
                .unwrap();
        }
        for track in &tracks {
            self.to_downstream
                .send(
                    Selected {
                        track_guid: track.track_guid,
                        selected: track.selected,
                    }
                    .into(),
                )
                .unwrap();
        }
        for track in &tracks {
            self.to_downstream
                .send(
                    Muted {
                        track_guid: track.track_guid,
                        muted: track.muted,
                    }
                    .into(),
                )
                .unwrap();
        }
        for track in &tracks {
            self.to_downstream
                .send(
                    Soloed {
                        track_guid: track.track_guid,
                        soloed: track.soloed,
                    }
                    .into(),
                )
                .unwrap();
        }
        for track in &tracks {
            self.to_downstream
                .send(
                    Armed {
                        track_guid: track.track_guid,
                        armed: track.armed,
                    }
                    .into(),
                )
                .unwrap();
        }
        for track in &tracks {
            for (send_index, send) in track.sends.iter().enumerate() {
                self.to_downstream
                    .send(
                        SendIndex {
                            track_guid: send.track_guid,
                            send_index: send_index as i32,
                            send_guid: send.target_guid,
                        }
                        .into(),
                    )
                    .unwrap();
                self.to_downstream
                    .send(
                        SendLevel {
                            track_guid: send.track_guid,
                            send_index: send_index as i32,
                            level: send.level,
                        }
                        .into(),
                    )
                    .unwrap();
                self.to_downstream
                    .send(
                        SendPan {
                            track_guid: send.track_guid,
                            send_index: send_index as i32,
                            pan: send.pan,
                        }
                        .into(),
                    )
                    .unwrap();
            }
        }
        for track in &tracks {
            for (fx_index, fx) in track.fx.iter().enumerate() {
                self.to_downstream
                    .send(
                        FXGuid {
                            track_guid: fx.track_guid,
                            fx_index: fx_index as i32,
                            guid: fx.guid,
                        }
                        .into(),
                    )
                    .unwrap();
                self.to_downstream
                    .send(
                        FXName {
                            track_guid: fx.track_guid,
                            fx_index: fx_index as i32,
                            name: fx.name.clone(),
                        }
                        .into(),
                    )
                    .unwrap();
                self.to_downstream
                    .send(
                        FXEnabled {
                            track_guid: fx.track_guid,
                            fx_index: fx_index as i32,
                            enabled: fx.enabled,
                        }
                        .into(),
                    )
                    .unwrap();
                for (param_index, param) in fx.params.iter().enumerate() {
                    self.to_downstream
                        .send(
                            FXParamName {
                                track_guid: fx.track_guid,
                                fx_index: fx_index as i32,
                                param_index: param_index as i32,
                                name: format!("Param {}", param_index),
                            }
                            .into(),
                        )
                        .unwrap();
                    self.to_downstream
                        .send(
                            FXParamValue {
                                track_guid: fx.track_guid,
                                fx_index: fx_index as i32,
                                param_index: param_index as i32,
                                value: param.value,
                            }
                            .into(),
                        )
                        .unwrap();
                    self.to_downstream
                        .send(
                            FXParamMin {
                                track_guid: fx.track_guid,
                                fx_index: fx_index as i32,
                                param_index: param_index as i32,
                                min: param.min,
                            }
                            .into(),
                        )
                        .unwrap();
                    self.to_downstream
                        .send(
                            FXParamMax {
                                track_guid: fx.track_guid,
                                fx_index: fx_index as i32,
                                param_index: param_index as i32,
                                max: param.max,
                            }
                            .into(),
                        )
                        .unwrap();
                }
            }
        }
    }

    fn send_track_query(&self, guid: &Uuid) {
        if let Some(track) = self.tracks.get(guid) {
            let response = TrackMsg::TrackData(track.clone());
            self.to_downstream.send(response).unwrap();
            self.to_downstream
                .send(TrackMsg::ReaperTrackIndex(ReaperTrackIndex {
                    track_guid: track.track_guid,
                    track_index: track.reaper_track_index,
                }))
                .unwrap();
            self.to_downstream
                .send(TrackMsg::Name(Name {
                    track_guid: track.track_guid,
                    name: track.name.clone(),
                }))
                .unwrap();
            self.to_downstream
                .send(TrackMsg::Muted(Muted {
                    track_guid: track.track_guid,
                    muted: track.muted,
                }))
                .unwrap();
            self.to_downstream
                .send(TrackMsg::Soloed(Soloed {
                    track_guid: track.track_guid,
                    soloed: track.soloed,
                }))
                .unwrap();
            self.to_downstream
                .send(TrackMsg::Armed(Armed {
                    track_guid: track.track_guid,
                    armed: track.armed,
                }))
                .unwrap();
            self.to_downstream
                .send(TrackMsg::Volume(Volume {
                    track_guid: track.track_guid,
                    volume: track.volume,
                }))
                .unwrap();
            self.to_downstream
                .send(TrackMsg::Pan(Pan {
                    track_guid: track.track_guid,
                    pan: track.pan,
                }))
                .unwrap();
            for (send_index, send) in track.sends.iter().enumerate() {
                self.to_downstream
                    .send(TrackMsg::SendIndex(SendIndex {
                        track_guid: send.track_guid,
                        send_index: send_index as i32,
                        send_guid: send.target_guid,
                    }))
                    .unwrap();
                self.to_downstream
                    .send(TrackMsg::SendLevel(SendLevel {
                        track_guid: send.track_guid,
                        send_index: send_index as i32,
                        level: send.level,
                    }))
                    .unwrap();
                self.to_downstream
                    .send(TrackMsg::SendPan(SendPan {
                        track_guid: send.track_guid,
                        send_index: send_index as i32,
                        pan: send.pan,
                    }))
                    .unwrap();
            }
            for (fx_index, fx) in track.fx.iter().enumerate() {
                self.to_downstream
                    .send(TrackMsg::FXGuid(FXGuid {
                        track_guid: fx.track_guid,
                        fx_index: fx_index as i32,
                        guid: fx.guid,
                    }))
                    .unwrap();
                self.to_downstream
                    .send(TrackMsg::FXName(FXName {
                        track_guid: fx.track_guid,
                        fx_index: fx_index as i32,
                        name: fx.name.clone(),
                    }))
                    .unwrap();
                self.to_downstream
                    .send(TrackMsg::FXEnabled(FXEnabled {
                        track_guid: fx.track_guid,
                        fx_index: fx_index as i32,
                        enabled: fx.enabled,
                    }))
                    .unwrap();
                for (param_index, param) in fx.params.iter().enumerate() {
                    self.to_downstream
                        .send(TrackMsg::FXParamName(FXParamName {
                            track_guid: fx.track_guid,
                            fx_index: fx_index as i32,
                            param_index: param_index as i32,
                            name: format!("Param {}", param_index),
                        }))
                        .unwrap();
                    self.to_downstream
                        .send(TrackMsg::FXParamValue(FXParamValue {
                            track_guid: fx.track_guid,
                            fx_index: fx_index as i32,
                            param_index: param_index as i32,
                            value: param.value,
                        }))
                        .unwrap();
                    self.to_downstream
                        .send(TrackMsg::FXParamMin(FXParamMin {
                            track_guid: fx.track_guid,
                            fx_index: fx_index as i32,
                            param_index: param_index as i32,
                            min: param.min,
                        }))
                        .unwrap();
                    self.to_downstream
                        .send(TrackMsg::FXParamMax(FXParamMax {
                            track_guid: fx.track_guid,
                            fx_index: fx_index as i32,
                            param_index: param_index as i32,
                            max: param.max,
                        }))
                        .unwrap();
                }
            }
            self.to_downstream
                .send(TrackMsg::Selected(Selected {
                    track_guid: track.track_guid,
                    selected: track.selected,
                }))
                .unwrap();
        }
    }

    pub fn start(
        from_uptream: Receiver<TrackMsg>,
        to_upstream: Sender<TrackMsg>,
        from_downstream: Receiver<TrackMsg>,
        to_downstream: Sender<TrackMsg>,
    ) {
        thread::spawn(move || {
            let mut manager = Self {
                tracks: HashMap::new(),
                selected_track: None,
                from_upstream: from_uptream,
                to_upstream,
                from_downstream,
                to_downstream,
            };
            loop {
                select! {
                    recv(manager.from_upstream) -> result=> {
                        match result{
                            Ok(msg) => {
                                match DataMsg::try_from(msg.clone()){
                                    Ok(data_msg) =>  {
                                        manager.handle_track_data_msg(data_msg.clone());
                                        // Forward message after we process it
                                        manager.to_downstream.send(msg).unwrap();
                                    }
                                    Err(TrackMsg::Barrier(barrier_msg)) => {
                                        // Simply forward barriers
                                        manager.to_downstream.send(TrackMsg::Barrier(barrier_msg)).unwrap();
                                    }
                                    Err(other) => {
                                        println!("Received unsupported message type from upstream (this should never happen): {:?}", other);
                                    }
                                }
                            }
                            Err(_) => {
                                // Upstream channel closed, we should probably exit
                                break;
                            }
                        }
                    },
                    recv(manager.from_downstream) -> result=> {
                        match result{
                            Ok(msg) => {
                                match DataMsg::try_from(msg.clone()){
                                    Ok(data_msg) =>  {
                                        manager.handle_track_data_msg(data_msg.clone());
                                        // Forward message after we process it
                                        manager.to_upstream.send(msg).unwrap();
                                    }
                                    Err(TrackMsg::Barrier(barrier_msg)) => {
                                        // Simply forward barriers
                                        manager.to_upstream.send(TrackMsg::Barrier(barrier_msg)).unwrap();
                                    }
                                    Err(TrackMsg::Query(query_msg)) => {
                                        println!("Received track query for GUID: {}", query_msg.guid);
                                            manager.send_track_query(&query_msg.guid);
                                    }
                                    Err(TrackMsg::QueryAll) => {
                                        println!("Received track query for all tracks");
                                        manager.send_queryall();
                                    }
                                    Err(other) => {
                                        println!("Received unsupported message type from downstream (this should never happen): {:?}", other);
                                    }
                                }
                            }
                            Err(_) => {
                                // Upstream channel closed, we should probably exit
                                break;
                            }
                        }
                    },
                }
            }
        });
    }

    fn get_or_create_track(&mut self, guid: Uuid) -> &mut TrackData {
        // If we've never seen this track before, create a new entry
        self.tracks
            .entry(guid)
            .or_insert_with(|| TrackData::new(guid))
    }

    // Even though this is a method, we define it in terms of in/out/reflect channels so that we
    // can reuse the same code to handle upstream and downstream messages. DRY
    pub fn handle_track_data_msg(&mut self, msg: DataMsg) {
        match msg {
            DataMsg::Delete(msg) => {
                println!("Deleting track with GUID: {}", msg.guid);
                self.tracks.remove(&msg.guid);
                if self.selected_track == Some(msg.guid) {
                    self.selected_track = None;
                }
            }
            DataMsg::Name(msg) => {
                self.get_or_create_track(msg.track_guid).name = msg.name.clone();
            }
            DataMsg::ReaperTrackIndex(msg) => {
                self.get_or_create_track(msg.track_guid).reaper_track_index = msg.track_index;
            }
            DataMsg::Selected(msg) => {
                self.get_or_create_track(msg.track_guid).selected = msg.selected;
                if msg.selected {
                    self.selected_track = Some(msg.track_guid);
                }
            }
            DataMsg::Muted(msg) => {
                self.get_or_create_track(msg.track_guid).muted = msg.muted;
            }
            DataMsg::Soloed(msg) => {
                self.get_or_create_track(msg.track_guid).soloed = msg.soloed;
            }
            DataMsg::Armed(msg) => {
                self.get_or_create_track(msg.track_guid).armed = msg.armed;
            }
            DataMsg::Volume(msg) => {
                self.get_or_create_track(msg.track_guid).volume = msg.volume;
            }
            DataMsg::Pan(msg) => {
                self.get_or_create_track(msg.track_guid).pan = msg.pan;
            }
            // Update everything!
            DataMsg::TrackData(track_data) => {
                *self.get_or_create_track(track_data.track_guid) = track_data.clone();
            }
            DataMsg::SendIndex(msg) => {
                self.get_or_create_track(msg.track_guid).set_send_index(msg);
            }
            DataMsg::SendLevel(msg) => {
                if let Some(send) = self
                    .get_or_create_track(msg.track_guid)
                    .get_send_state(msg.send_index)
                {
                    send.level = msg.level;
                }
            }
            DataMsg::SendPan(msg) => {
                if let Some(send) = self
                    .get_or_create_track(msg.track_guid)
                    .get_send_state(msg.send_index)
                {
                    send.pan = msg.pan;
                }
            }
            DataMsg::FXGuid(msg) => {
                if let Some(fx) = self
                    .get_or_create_track(msg.track_guid)
                    .get_fx_data(msg.fx_index)
                {
                    fx.guid = msg.guid;
                    println!(
                        "Track {} FX {} GUID set to {}",
                        msg.track_guid, msg.fx_index, msg.guid
                    );
                }
            }
            DataMsg::FXName(msg) => {
                if let Some(fx) = self
                    .get_or_create_track(msg.track_guid)
                    .get_fx_data(msg.fx_index)
                {
                    fx.name = msg.name.clone();
                    println!(
                        "Track {} FX {} name set to {}",
                        msg.track_guid, msg.fx_index, msg.name
                    );
                }
            }
            DataMsg::FXEnabled(msg) => {
                if let Some(fx) = self
                    .get_or_create_track(msg.track_guid)
                    .get_fx_data(msg.fx_index)
                {
                    fx.enabled = msg.enabled;
                    println!(
                        "Track {} FX {} enabled set to {}",
                        msg.track_guid, msg.fx_index, msg.enabled
                    );
                }
            }
            DataMsg::FXParamName(msg) => {
                if let Some(fx) = self
                    .get_or_create_track(msg.track_guid)
                    .get_fx_data(msg.fx_index)
                {
                    if let Some(_param) = fx.get_param_data(msg.param_index) {
                        // We don't store the name in FXParamData currently
                        println!(
                            "Track {} FX {} Param {} name set to {}",
                            msg.track_guid, msg.fx_index, msg.param_index, msg.name
                        );
                    }
                }
            }
            DataMsg::FXParamValue(msg) => {
                if let Some(fx) = self
                    .get_or_create_track(msg.track_guid)
                    .get_fx_data(msg.fx_index)
                {
                    if let Some(param) = fx.get_param_data(msg.param_index) {
                        param.value = msg.value;
                        println!(
                            "Track {} FX {} Param {} value set to {}",
                            msg.track_guid, msg.fx_index, msg.param_index, msg.value
                        );
                    }
                }
            }
            DataMsg::FXParamMin(msg) => {
                if let Some(fx) = self
                    .get_or_create_track(msg.track_guid)
                    .get_fx_data(msg.fx_index)
                {
                    if let Some(param) = fx.get_param_data(msg.param_index) {
                        param.min = msg.min;
                        println!(
                            "Track {} FX {} Param {} min set to {}",
                            msg.track_guid, msg.fx_index, msg.param_index, msg.min
                        );
                    }
                }
            }
            DataMsg::FXParamMax(msg) => {
                if let Some(fx) = self
                    .get_or_create_track(msg.track_guid)
                    .get_fx_data(msg.fx_index)
                {
                    if let Some(param) = fx.get_param_data(msg.param_index) {
                        param.max = msg.max;
                        println!(
                            "Track {} FX {} Param {} max set to {}",
                            msg.track_guid, msg.fx_index, msg.param_index, msg.max
                        );
                    }
                }
            }
        }
    }
}
