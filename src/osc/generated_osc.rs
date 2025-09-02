// AUTO-GENERATED CODE. DO NOT EDIT!

use std::collections::HashMap;
use std::net::UdpSocket;
use std::sync::Arc;

use crate::traits::{Bind, Query, Set};

use crate::osc::route_context::{ContextKindTrait, ContextTrait};

#[derive(Debug)]
pub struct OscError;

pub mod context {
    use crate::osc::generated_osc::ContextTrait;

    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    pub struct Track {
        pub track_guid: String,
    }

    impl ContextTrait for Track {}

    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    pub struct TrackSend {
        pub track_guid: String,
        pub send_index: i32,
    }

    impl ContextTrait for TrackSend {}
}

pub mod context_kind {
    use regex::Regex;

    use crate::osc::generated_osc::context;
    use crate::osc::route_context::{ContextKindTrait, ContextTrait};

    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    pub struct Track {}

    impl ContextKindTrait for Track {
        type Context = context::Track;
        fn context_name() -> &'static str {
            "Track"
        }

        fn parse(osc_address: &str) -> Option<context::Track> {
            let re = Regex::new(r"^/track/([^/]+)/index$").unwrap();
            re.captures(osc_address).map(|caps| context::Track {
                track_guid: caps[1].to_string(),
            })
        }
    }

    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    pub struct TrackSend {}

    impl ContextKindTrait for TrackSend {
        type Context = context::TrackSend;
        fn context_name() -> &'static str {
            "TrackSend"
        }

        fn parse(osc_address: &str) -> Option<context::TrackSend> {
            let re = Regex::new(r"^/track/([^/]+)/send/([^/]+)/guid$").unwrap();
            re.captures(osc_address).map(|caps| context::TrackSend {
                track_guid: caps[1].to_string(),
                send_index: caps[2].parse().unwrap(),
            })
        }
    }
}

pub struct Reaper {
    socket: Arc<UdpSocket>,
    pub track_guid_map: HashMap<String, Track>,
}

impl Reaper {
    pub fn new(socket: Arc<UdpSocket>) -> Reaper {
        Reaper {
            socket,
            track_guid_map: HashMap::new(),
        }
    }
    pub fn track(&mut self, track_guid: String) -> &mut Track {
        self.track_guid_map
            .entry(track_guid.clone())
            .or_insert_with(|| Track::new(self.socket.clone(), track_guid.clone()))
    }
}

pub struct Track {
    socket: Arc<UdpSocket>,
    pub track_guid: String,
    pub send_index_map: HashMap<String, TrackSend>,
}

impl Track {
    pub fn new(socket: Arc<UdpSocket>, track_guid: String) -> Track {
        Track {
            socket,
            track_guid: track_guid.clone(),
            send_index_map: HashMap::new(),
        }
    }
    pub fn send(&mut self, send_index: String) -> &mut TrackSend {
        self.send_index_map
            .entry(send_index.clone())
            .or_insert_with(|| {
                TrackSend::new(
                    self.socket.clone(),
                    self.track_guid.clone(),
                    send_index.clone(),
                )
            })
    }
    pub fn index(&self) -> TrackIndex {
        TrackIndex::new(self.socket.clone(), self.track_guid.clone())
    }
    pub fn selected(&self) -> TrackSelected {
        TrackSelected::new(self.socket.clone(), self.track_guid.clone())
    }
}

pub struct TrackSend {
    socket: Arc<UdpSocket>,
    pub track_guid: String,
    pub send_index: String,
}

impl TrackSend {
    pub fn new(socket: Arc<UdpSocket>, track_guid: String, send_index: String) -> TrackSend {
        TrackSend {
            socket,
            track_guid: track_guid.clone(),
            send_index: send_index.clone(),
        }
    }
    pub fn volume(&self) -> TrackSendVolume {
        TrackSendVolume::new(
            self.socket.clone(),
            self.track_guid.clone(),
            self.send_index.clone(),
        )
    }
    pub fn guid(&self) -> TrackSendGuid {
        TrackSendGuid::new(
            self.socket.clone(),
            self.track_guid.clone(),
            self.send_index.clone(),
        )
    }
}

pub struct TrackSendVolume {
    socket: Arc<UdpSocket>,
    handler: Option<TrackSendVolumeHandler>,
    pub track_guid: String,
    pub send_index: String,
}

impl TrackSendVolume {
    pub fn new(socket: Arc<UdpSocket>, track_guid: String, send_index: String) -> TrackSendVolume {
        TrackSendVolume {
            socket,
            handler: None,
            track_guid: track_guid.clone(),
            send_index: send_index.clone(),
        }
    }
}

#[derive(Debug)]
pub struct TrackSendVolumeArgs {
    pub volume: f32, // volume of the send, normalized to 0 to 1.
}

pub type TrackSendVolumeHandler = Box<dyn FnMut(TrackSendVolumeArgs) + 'static>;

/// /track/{track_guid}/send/{send_index}/volume
impl Set<TrackSendVolumeArgs> for TrackSendVolume {
    type Error = OscError;
    fn set(&mut self, args: TrackSendVolumeArgs) -> Result<(), Self::Error> {
        let osc_address = format!("/track/{}/send/{}/volume", self.track_guid, self.send_index);
        let osc_msg = rosc::OscMessage {
            addr: osc_address,
            args: vec![rosc::OscType::Float(args.volume)],
        };
        let packet = rosc::OscPacket::Message(osc_msg);
        let buf = rosc::encoder::encode(&packet).map_err(|_| OscError)?;
        self.socket.send(&buf).map_err(|_| OscError)?;
        Ok(())
    }
}

/// /track/{track_guid}/send/{send_index}/volume
impl Query for TrackSendVolume {
    type Error = OscError;
    fn query(&self) -> Result<(), Self::Error> {
        let osc_address = format!("/track/{}/send/{}/volume", self.track_guid, self.send_index);
        let osc_msg = rosc::OscMessage {
            addr: osc_address,
            args: vec![],
        };
        let packet = rosc::OscPacket::Message(osc_msg);
        let buf = rosc::encoder::encode(&packet).map_err(|_| OscError)?;
        self.socket.send(&buf).map_err(|_| OscError)?;
        Ok(())
    }
}

/// /track/{track_guid}/send/{send_index}/volume
impl Bind<TrackSendVolumeArgs> for TrackSendVolume {
    fn bind<F>(&mut self, callback: F)
    where
        F: FnMut(TrackSendVolumeArgs) + 'static,
    {
        self.handler = Some(Box::new(callback));
    }
}

pub struct TrackSendGuid {
    socket: Arc<UdpSocket>,
    handler: Option<TrackSendGuidHandler>,
    pub track_guid: String,
    pub send_index: String,
}

impl TrackSendGuid {
    pub fn new(socket: Arc<UdpSocket>, track_guid: String, send_index: String) -> TrackSendGuid {
        TrackSendGuid {
            socket,
            handler: None,
            track_guid: track_guid.clone(),
            send_index: send_index.clone(),
        }
    }
}

#[derive(Debug)]
pub struct TrackSendGuidArgs {
    pub guid: String, // unique identifier for the send
}

pub type TrackSendGuidHandler = Box<dyn FnMut(TrackSendGuidArgs) + 'static>;

/// /track/{track_guid}/send/{send_index}/guid
impl Query for TrackSendGuid {
    type Error = OscError;
    fn query(&self) -> Result<(), Self::Error> {
        let osc_address = format!("/track/{}/send/{}/guid", self.track_guid, self.send_index);
        let osc_msg = rosc::OscMessage {
            addr: osc_address,
            args: vec![],
        };
        let packet = rosc::OscPacket::Message(osc_msg);
        let buf = rosc::encoder::encode(&packet).map_err(|_| OscError)?;
        self.socket.send(&buf).map_err(|_| OscError)?;
        Ok(())
    }
}

/// /track/{track_guid}/send/{send_index}/guid
impl Bind<TrackSendGuidArgs> for TrackSendGuid {
    fn bind<F>(&mut self, callback: F)
    where
        F: FnMut(TrackSendGuidArgs) + 'static,
    {
        self.handler = Some(Box::new(callback));
    }
}

pub struct TrackIndex {
    socket: Arc<UdpSocket>,
    handler: Option<TrackIndexHandler>,
    pub track_guid: String,
}

impl TrackIndex {
    pub fn new(socket: Arc<UdpSocket>, track_guid: String) -> TrackIndex {
        TrackIndex {
            socket,
            handler: None,
            track_guid: track_guid.clone(),
        }
    }
}

#[derive(Debug)]
pub struct TrackIndexArgs {
    pub index: i32, // index of the track in the project according to reaper's mixer view
}

pub type TrackIndexHandler = Box<dyn FnMut(TrackIndexArgs) + 'static>;

/// /track/{track_guid}/index
impl Query for TrackIndex {
    type Error = OscError;
    fn query(&self) -> Result<(), Self::Error> {
        let osc_address = format!("/track/{}/index", self.track_guid);
        let osc_msg = rosc::OscMessage {
            addr: osc_address,
            args: vec![],
        };
        let packet = rosc::OscPacket::Message(osc_msg);
        let buf = rosc::encoder::encode(&packet).map_err(|_| OscError)?;
        self.socket.send(&buf).map_err(|_| OscError)?;
        Ok(())
    }
}

/// /track/{track_guid}/index
impl Bind<TrackIndexArgs> for TrackIndex {
    fn bind<F>(&mut self, callback: F)
    where
        F: FnMut(TrackIndexArgs) + 'static,
    {
        self.handler = Some(Box::new(callback));
    }
}

pub struct TrackSelected {
    socket: Arc<UdpSocket>,
    handler: Option<TrackSelectedHandler>,
    pub track_guid: String,
}

impl TrackSelected {
    pub fn new(socket: Arc<UdpSocket>, track_guid: String) -> TrackSelected {
        TrackSelected {
            socket,
            handler: None,
            track_guid: track_guid.clone(),
        }
    }
}

#[derive(Debug)]
pub struct TrackSelectedArgs {
    pub selected: bool, // true means track is selected
}

pub type TrackSelectedHandler = Box<dyn FnMut(TrackSelectedArgs) + 'static>;

/// /track/{track_guid}/selected
impl Set<TrackSelectedArgs> for TrackSelected {
    type Error = OscError;
    fn set(&mut self, args: TrackSelectedArgs) -> Result<(), Self::Error> {
        let osc_address = format!("/track/{}/selected", self.track_guid);
        let osc_msg = rosc::OscMessage {
            addr: osc_address,
            args: vec![rosc::OscType::Bool(args.selected)],
        };
        let packet = rosc::OscPacket::Message(osc_msg);
        let buf = rosc::encoder::encode(&packet).map_err(|_| OscError)?;
        self.socket.send(&buf).map_err(|_| OscError)?;
        Ok(())
    }
}

/// /track/{track_guid}/selected
impl Query for TrackSelected {
    type Error = OscError;
    fn query(&self) -> Result<(), Self::Error> {
        let osc_address = format!("/track/{}/selected", self.track_guid);
        let osc_msg = rosc::OscMessage {
            addr: osc_address,
            args: vec![],
        };
        let packet = rosc::OscPacket::Message(osc_msg);
        let buf = rosc::encoder::encode(&packet).map_err(|_| OscError)?;
        self.socket.send(&buf).map_err(|_| OscError)?;
        Ok(())
    }
}

/// /track/{track_guid}/selected
impl Bind<TrackSelectedArgs> for TrackSelected {
    fn bind<F>(&mut self, callback: F)
    where
        F: FnMut(TrackSelectedArgs) + 'static,
    {
        self.handler = Some(Box::new(callback));
    }
}

/// Try to match an OSC address against a pattern, extracting arguments.
/// E.g. addr: "/track/abc123/pan", pattern: "/track/{}/pan" -> Some(vec!["abc123"])
fn match_addr(addr: &str, pattern: &str) -> Option<Vec<String>> {
    let addr_parts: Vec<&str> = addr.split('/').filter(|s| !s.is_empty()).collect();
    let pat_parts: Vec<&str> = pattern.split('/').filter(|s| !s.is_empty()).collect();
    if addr_parts.len() != pat_parts.len() {
        return None;
    }
    let mut args = Vec::new();
    for (a, p) in addr_parts.iter().zip(pat_parts.iter()) {
        if *p == "{}" {
            args.push((*a).to_string());
        } else if *p != *a {
            return None;
        }
    }
    Some(args)
}

pub fn dispatch_osc<F>(reaper: &mut Reaper, packet: rosc::OscPacket, log_unknown: F)
where
    F: Fn(&str),
{
    let msg = match packet {
        rosc::OscPacket::Message(msg) => msg,
        _ => return,
    };
    let addr = msg.addr.as_str();
    if let Some(args) = match_addr(addr, "/track/{track_guid}/send/{send_index}/volume") {
        let send_index = &args[1];
        let track_guid = &args[2];
        let track = reaper.track(track_guid.clone());
        let send = track.send(send_index.clone());
        let mut endpoint = send.volume();
        if let Some(handler) = &mut endpoint.handler {
            if let Some(volume) = msg.args.get(0) {
                handler(TrackSendVolumeArgs {
                    volume: volume.clone().float().unwrap(),
                });
            }
        }
        return;
    }
    if let Some(args) = match_addr(addr, "/track/{track_guid}/send/{send_index}/guid") {
        let send_index = &args[1];
        let track_guid = &args[2];
        let track = reaper.track(track_guid.clone());
        let send = track.send(send_index.clone());
        let mut endpoint = send.guid();
        if let Some(handler) = &mut endpoint.handler {
            if let Some(guid) = msg.args.get(0) {
                handler(TrackSendGuidArgs {
                    guid: guid.clone().string().unwrap().clone(),
                });
            }
        }
        return;
    }
    if let Some(args) = match_addr(addr, "/track/{track_guid}/index") {
        let track_guid = &args[1];
        let track = reaper.track(track_guid.clone());
        let mut endpoint = track.index();
        if let Some(handler) = &mut endpoint.handler {
            if let Some(index) = msg.args.get(0) {
                handler(TrackIndexArgs {
                    index: index.clone().int().unwrap(),
                });
            }
        }
        return;
    }
    if let Some(args) = match_addr(addr, "/track/{track_guid}/selected") {
        let track_guid = &args[1];
        let track = reaper.track(track_guid.clone());
        let mut endpoint = track.selected();
        if let Some(handler) = &mut endpoint.handler {
            if let Some(selected) = msg.args.get(0) {
                handler(TrackSelectedArgs {
                    selected: selected.clone().bool().unwrap(),
                });
            }
        }
        return;
    }
    log_unknown(addr);
}
