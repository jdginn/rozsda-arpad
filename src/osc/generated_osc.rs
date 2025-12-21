// AUTO-GENERATED CODE. DO NOT EDIT!

use std::collections::HashMap;
use std::net::UdpSocket;
use std::sync::Arc;

use crate::traits::{Bind, Query, Set};

use crate::osc::route_context::ContextTrait;

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
    use super::context;
    use crate::osc::route_context::ContextKindTrait;
    use regex::Regex;

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
    pub send_index_map: HashMap<i32, TrackSend>,
}

impl Track {
    pub fn new(socket: Arc<UdpSocket>, track_guid: String) -> Track {
        Track {
            socket,
            track_guid: track_guid.clone(),
            send_index_map: HashMap::new(),
        }
    }
    pub fn solo(&self) -> TrackSolo {
        TrackSolo::new(self.socket.clone(), self.track_guid.clone())
    }
    pub fn send(&mut self, send_index: i32) -> &mut TrackSend {
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
    pub fn color(&self) -> TrackColor {
        TrackColor::new(self.socket.clone(), self.track_guid.clone())
    }
    pub fn rec_arm(&self) -> TrackRecArm {
        TrackRecArm::new(self.socket.clone(), self.track_guid.clone())
    }
    pub fn selected(&self) -> TrackSelected {
        TrackSelected::new(self.socket.clone(), self.track_guid.clone())
    }
    pub fn name(&self) -> TrackName {
        TrackName::new(self.socket.clone(), self.track_guid.clone())
    }
    pub fn delete(&self) -> TrackDelete {
        TrackDelete::new(self.socket.clone(), self.track_guid.clone())
    }
    pub fn volume(&self) -> TrackVolume {
        TrackVolume::new(self.socket.clone(), self.track_guid.clone())
    }
    pub fn mute(&self) -> TrackMute {
        TrackMute::new(self.socket.clone(), self.track_guid.clone())
    }
    pub fn pan(&self) -> TrackPan {
        TrackPan::new(self.socket.clone(), self.track_guid.clone())
    }
    pub fn index(&self) -> TrackIndex {
        TrackIndex::new(self.socket.clone(), self.track_guid.clone())
    }
}

pub struct TrackSolo {
    socket: Arc<UdpSocket>,
    handler: Option<TrackSoloHandler>,
    pub track_guid: String,
}

impl TrackSolo {
    pub fn new(socket: Arc<UdpSocket>, track_guid: String) -> TrackSolo {
        TrackSolo {
            socket,
            handler: None,
            track_guid: track_guid.clone(),
        }
    }
}

#[derive(Debug)]
pub struct TrackSoloArgs {
    pub solo: bool, // true means track is soloed
}

pub type TrackSoloHandler = Box<dyn FnMut(TrackSoloArgs) + 'static>;

/// /track/{track_guid}/solo
impl Set<TrackSoloArgs> for TrackSolo {
    type Error = OscError;
    fn set(&mut self, args: TrackSoloArgs) -> Result<(), Self::Error> {
        let osc_address = format!("/track/{}/solo", self.track_guid);
        let osc_msg = rosc::OscMessage {
            addr: osc_address,
            args: vec![rosc::OscType::Bool(args.solo)],
        };
        let packet = rosc::OscPacket::Message(osc_msg);
        let buf = rosc::encoder::encode(&packet).map_err(|_| OscError)?;
        self.socket.send(&buf).map_err(|_| OscError)?;
        Ok(())
    }
}

/// /track/{track_guid}/solo
impl Query for TrackSolo {
    type Error = OscError;
    fn query(&self) -> Result<(), Self::Error> {
        let osc_address = format!("/track/{}/solo", self.track_guid);
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

/// /track/{track_guid}/solo
impl Bind<TrackSoloArgs> for TrackSolo {
    fn bind<F>(&mut self, callback: F)
    where
        F: FnMut(TrackSoloArgs) + 'static,
    {
        self.handler = Some(Box::new(callback));
    }
}

pub struct TrackSend {
    socket: Arc<UdpSocket>,
    pub track_guid: String,
    pub send_index: i32,
}

impl TrackSend {
    pub fn new(socket: Arc<UdpSocket>, track_guid: String, send_index: i32) -> TrackSend {
        TrackSend {
            socket,
            track_guid: track_guid.clone(),
            send_index: send_index.clone(),
        }
    }
    pub fn guid(&self) -> TrackSendGuid {
        TrackSendGuid::new(
            self.socket.clone(),
            self.track_guid.clone(),
            self.send_index.clone(),
        )
    }
    pub fn pan(&self) -> TrackSendPan {
        TrackSendPan::new(
            self.socket.clone(),
            self.track_guid.clone(),
            self.send_index.clone(),
        )
    }
    pub fn volume(&self) -> TrackSendVolume {
        TrackSendVolume::new(
            self.socket.clone(),
            self.track_guid.clone(),
            self.send_index.clone(),
        )
    }
}

pub struct TrackSendGuid {
    socket: Arc<UdpSocket>,
    handler: Option<TrackSendGuidHandler>,
    pub track_guid: String,
    pub send_index: i32,
}

impl TrackSendGuid {
    pub fn new(socket: Arc<UdpSocket>, track_guid: String, send_index: i32) -> TrackSendGuid {
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

pub struct TrackSendPan {
    socket: Arc<UdpSocket>,
    handler: Option<TrackSendPanHandler>,
    pub track_guid: String,
    pub send_index: i32,
}

impl TrackSendPan {
    pub fn new(socket: Arc<UdpSocket>, track_guid: String, send_index: i32) -> TrackSendPan {
        TrackSendPan {
            socket,
            handler: None,
            track_guid: track_guid.clone(),
            send_index: send_index.clone(),
        }
    }
}

#[derive(Debug)]
pub struct TrackSendPanArgs {
    pub pan: f32, // pan of the send, normalized to -1.0 to 1.0
}

pub type TrackSendPanHandler = Box<dyn FnMut(TrackSendPanArgs) + 'static>;

/// /track/{track_guid}/send/{send_index}/pan
impl Set<TrackSendPanArgs> for TrackSendPan {
    type Error = OscError;
    fn set(&mut self, args: TrackSendPanArgs) -> Result<(), Self::Error> {
        let osc_address = format!("/track/{}/send/{}/pan", self.track_guid, self.send_index);
        let osc_msg = rosc::OscMessage {
            addr: osc_address,
            args: vec![rosc::OscType::Float(args.pan)],
        };
        let packet = rosc::OscPacket::Message(osc_msg);
        let buf = rosc::encoder::encode(&packet).map_err(|_| OscError)?;
        self.socket.send(&buf).map_err(|_| OscError)?;
        Ok(())
    }
}

/// /track/{track_guid}/send/{send_index}/pan
impl Query for TrackSendPan {
    type Error = OscError;
    fn query(&self) -> Result<(), Self::Error> {
        let osc_address = format!("/track/{}/send/{}/pan", self.track_guid, self.send_index);
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

/// /track/{track_guid}/send/{send_index}/pan
impl Bind<TrackSendPanArgs> for TrackSendPan {
    fn bind<F>(&mut self, callback: F)
    where
        F: FnMut(TrackSendPanArgs) + 'static,
    {
        self.handler = Some(Box::new(callback));
    }
}

pub struct TrackSendVolume {
    socket: Arc<UdpSocket>,
    handler: Option<TrackSendVolumeHandler>,
    pub track_guid: String,
    pub send_index: i32,
}

impl TrackSendVolume {
    pub fn new(socket: Arc<UdpSocket>, track_guid: String, send_index: i32) -> TrackSendVolume {
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

pub struct TrackColor {
    socket: Arc<UdpSocket>,
    handler: Option<TrackColorHandler>,
    pub track_guid: String,
}

impl TrackColor {
    pub fn new(socket: Arc<UdpSocket>, track_guid: String) -> TrackColor {
        TrackColor {
            socket,
            handler: None,
            track_guid: track_guid.clone(),
        }
    }
}

#[derive(Debug)]
pub struct TrackColorArgs {
    pub color: i32, // color of the track, represented as an RGB integer
}

pub type TrackColorHandler = Box<dyn FnMut(TrackColorArgs) + 'static>;

/// /track/{track_guid}/color
impl Set<TrackColorArgs> for TrackColor {
    type Error = OscError;
    fn set(&mut self, args: TrackColorArgs) -> Result<(), Self::Error> {
        let osc_address = format!("/track/{}/color", self.track_guid);
        let osc_msg = rosc::OscMessage {
            addr: osc_address,
            args: vec![rosc::OscType::Int(args.color)],
        };
        let packet = rosc::OscPacket::Message(osc_msg);
        let buf = rosc::encoder::encode(&packet).map_err(|_| OscError)?;
        self.socket.send(&buf).map_err(|_| OscError)?;
        Ok(())
    }
}

/// /track/{track_guid}/color
impl Query for TrackColor {
    type Error = OscError;
    fn query(&self) -> Result<(), Self::Error> {
        let osc_address = format!("/track/{}/color", self.track_guid);
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

/// /track/{track_guid}/color
impl Bind<TrackColorArgs> for TrackColor {
    fn bind<F>(&mut self, callback: F)
    where
        F: FnMut(TrackColorArgs) + 'static,
    {
        self.handler = Some(Box::new(callback));
    }
}

pub struct TrackRecArm {
    socket: Arc<UdpSocket>,
    handler: Option<TrackRecArmHandler>,
    pub track_guid: String,
}

impl TrackRecArm {
    pub fn new(socket: Arc<UdpSocket>, track_guid: String) -> TrackRecArm {
        TrackRecArm {
            socket,
            handler: None,
            track_guid: track_guid.clone(),
        }
    }
}

#[derive(Debug)]
pub struct TrackRecArmArgs {
    pub rec_arm: bool, // true means track is armed for recording
}

pub type TrackRecArmHandler = Box<dyn FnMut(TrackRecArmArgs) + 'static>;

/// /track/{track_guid}/rec-arm
impl Set<TrackRecArmArgs> for TrackRecArm {
    type Error = OscError;
    fn set(&mut self, args: TrackRecArmArgs) -> Result<(), Self::Error> {
        let osc_address = format!("/track/{}/rec-arm", self.track_guid);
        let osc_msg = rosc::OscMessage {
            addr: osc_address,
            args: vec![rosc::OscType::Bool(args.rec_arm)],
        };
        let packet = rosc::OscPacket::Message(osc_msg);
        let buf = rosc::encoder::encode(&packet).map_err(|_| OscError)?;
        self.socket.send(&buf).map_err(|_| OscError)?;
        Ok(())
    }
}

/// /track/{track_guid}/rec-arm
impl Query for TrackRecArm {
    type Error = OscError;
    fn query(&self) -> Result<(), Self::Error> {
        let osc_address = format!("/track/{}/rec-arm", self.track_guid);
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

/// /track/{track_guid}/rec-arm
impl Bind<TrackRecArmArgs> for TrackRecArm {
    fn bind<F>(&mut self, callback: F)
    where
        F: FnMut(TrackRecArmArgs) + 'static,
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

pub struct TrackName {
    socket: Arc<UdpSocket>,
    handler: Option<TrackNameHandler>,
    pub track_guid: String,
}

impl TrackName {
    pub fn new(socket: Arc<UdpSocket>, track_guid: String) -> TrackName {
        TrackName {
            socket,
            handler: None,
            track_guid: track_guid.clone(),
        }
    }
}

#[derive(Debug)]
pub struct TrackNameArgs {
    pub name: String, // name of the track
}

pub type TrackNameHandler = Box<dyn FnMut(TrackNameArgs) + 'static>;

/// /track/{track_guid}/name
impl Set<TrackNameArgs> for TrackName {
    type Error = OscError;
    fn set(&mut self, args: TrackNameArgs) -> Result<(), Self::Error> {
        let osc_address = format!("/track/{}/name", self.track_guid);
        let osc_msg = rosc::OscMessage {
            addr: osc_address,
            args: vec![rosc::OscType::String(args.name.clone())],
        };
        let packet = rosc::OscPacket::Message(osc_msg);
        let buf = rosc::encoder::encode(&packet).map_err(|_| OscError)?;
        self.socket.send(&buf).map_err(|_| OscError)?;
        Ok(())
    }
}

/// /track/{track_guid}/name
impl Query for TrackName {
    type Error = OscError;
    fn query(&self) -> Result<(), Self::Error> {
        let osc_address = format!("/track/{}/name", self.track_guid);
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

/// /track/{track_guid}/name
impl Bind<TrackNameArgs> for TrackName {
    fn bind<F>(&mut self, callback: F)
    where
        F: FnMut(TrackNameArgs) + 'static,
    {
        self.handler = Some(Box::new(callback));
    }
}

pub struct TrackDelete {
    socket: Arc<UdpSocket>,
    handler: Option<TrackDeleteHandler>,
    pub track_guid: String,
}

impl TrackDelete {
    pub fn new(socket: Arc<UdpSocket>, track_guid: String) -> TrackDelete {
        TrackDelete {
            socket,
            handler: None,
            track_guid: track_guid.clone(),
        }
    }
}

#[derive(Debug)]
pub struct TrackDeleteArgs {}

pub type TrackDeleteHandler = Box<dyn FnMut(TrackDeleteArgs) + 'static>;

/// /track/{track_guid}/delete
impl Set<TrackDeleteArgs> for TrackDelete {
    type Error = OscError;
    fn set(&mut self, args: TrackDeleteArgs) -> Result<(), Self::Error> {
        let osc_address = format!("/track/{}/delete", self.track_guid);
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

/// /track/{track_guid}/delete
impl Query for TrackDelete {
    type Error = OscError;
    fn query(&self) -> Result<(), Self::Error> {
        let osc_address = format!("/track/{}/delete", self.track_guid);
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

/// /track/{track_guid}/delete
impl Bind<TrackDeleteArgs> for TrackDelete {
    fn bind<F>(&mut self, callback: F)
    where
        F: FnMut(TrackDeleteArgs) + 'static,
    {
        self.handler = Some(Box::new(callback));
    }
}

pub struct TrackVolume {
    socket: Arc<UdpSocket>,
    handler: Option<TrackVolumeHandler>,
    pub track_guid: String,
}

impl TrackVolume {
    pub fn new(socket: Arc<UdpSocket>, track_guid: String) -> TrackVolume {
        TrackVolume {
            socket,
            handler: None,
            track_guid: track_guid.clone(),
        }
    }
}

#[derive(Debug)]
pub struct TrackVolumeArgs {
    pub volume: f32, // volume of the track, normalized to 0 to 1.0
}

pub type TrackVolumeHandler = Box<dyn FnMut(TrackVolumeArgs) + 'static>;

/// /track/{track_guid}/volume
impl Set<TrackVolumeArgs> for TrackVolume {
    type Error = OscError;
    fn set(&mut self, args: TrackVolumeArgs) -> Result<(), Self::Error> {
        let osc_address = format!("/track/{}/volume", self.track_guid);
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

/// /track/{track_guid}/volume
impl Query for TrackVolume {
    type Error = OscError;
    fn query(&self) -> Result<(), Self::Error> {
        let osc_address = format!("/track/{}/volume", self.track_guid);
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

/// /track/{track_guid}/volume
impl Bind<TrackVolumeArgs> for TrackVolume {
    fn bind<F>(&mut self, callback: F)
    where
        F: FnMut(TrackVolumeArgs) + 'static,
    {
        self.handler = Some(Box::new(callback));
    }
}

pub struct TrackMute {
    socket: Arc<UdpSocket>,
    handler: Option<TrackMuteHandler>,
    pub track_guid: String,
}

impl TrackMute {
    pub fn new(socket: Arc<UdpSocket>, track_guid: String) -> TrackMute {
        TrackMute {
            socket,
            handler: None,
            track_guid: track_guid.clone(),
        }
    }
}

#[derive(Debug)]
pub struct TrackMuteArgs {
    pub mute: bool, // true means track is muted
}

pub type TrackMuteHandler = Box<dyn FnMut(TrackMuteArgs) + 'static>;

/// /track/{track_guid}/mute
impl Set<TrackMuteArgs> for TrackMute {
    type Error = OscError;
    fn set(&mut self, args: TrackMuteArgs) -> Result<(), Self::Error> {
        let osc_address = format!("/track/{}/mute", self.track_guid);
        let osc_msg = rosc::OscMessage {
            addr: osc_address,
            args: vec![rosc::OscType::Bool(args.mute)],
        };
        let packet = rosc::OscPacket::Message(osc_msg);
        let buf = rosc::encoder::encode(&packet).map_err(|_| OscError)?;
        self.socket.send(&buf).map_err(|_| OscError)?;
        Ok(())
    }
}

/// /track/{track_guid}/mute
impl Query for TrackMute {
    type Error = OscError;
    fn query(&self) -> Result<(), Self::Error> {
        let osc_address = format!("/track/{}/mute", self.track_guid);
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

/// /track/{track_guid}/mute
impl Bind<TrackMuteArgs> for TrackMute {
    fn bind<F>(&mut self, callback: F)
    where
        F: FnMut(TrackMuteArgs) + 'static,
    {
        self.handler = Some(Box::new(callback));
    }
}

pub struct TrackPan {
    socket: Arc<UdpSocket>,
    handler: Option<TrackPanHandler>,
    pub track_guid: String,
}

impl TrackPan {
    pub fn new(socket: Arc<UdpSocket>, track_guid: String) -> TrackPan {
        TrackPan {
            socket,
            handler: None,
            track_guid: track_guid.clone(),
        }
    }
}

#[derive(Debug)]
pub struct TrackPanArgs {
    pub pan: f32, // pan of the track, normalized to -1.0 to 1.0
}

pub type TrackPanHandler = Box<dyn FnMut(TrackPanArgs) + 'static>;

/// /track/{track_guid}/pan
impl Set<TrackPanArgs> for TrackPan {
    type Error = OscError;
    fn set(&mut self, args: TrackPanArgs) -> Result<(), Self::Error> {
        let osc_address = format!("/track/{}/pan", self.track_guid);
        let osc_msg = rosc::OscMessage {
            addr: osc_address,
            args: vec![rosc::OscType::Float(args.pan)],
        };
        let packet = rosc::OscPacket::Message(osc_msg);
        let buf = rosc::encoder::encode(&packet).map_err(|_| OscError)?;
        self.socket.send(&buf).map_err(|_| OscError)?;
        Ok(())
    }
}

/// /track/{track_guid}/pan
impl Query for TrackPan {
    type Error = OscError;
    fn query(&self) -> Result<(), Self::Error> {
        let osc_address = format!("/track/{}/pan", self.track_guid);
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

/// /track/{track_guid}/pan
impl Bind<TrackPanArgs> for TrackPan {
    fn bind<F>(&mut self, callback: F)
    where
        F: FnMut(TrackPanArgs) + 'static,
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

pub fn dispatch_osc<F>(reaper: &mut Reaper, msg: rosc::OscMessage, log_unknown: F)
where
    F: Fn(&str),
{
    let addr = msg.addr.as_str();
    if let Some(args) = match_addr(addr, "/track/{track_guid}/solo") {
        let track_guid = &args[1];
        let track = reaper.track(track_guid.clone());
        let mut endpoint = track.solo();
        if let Some(handler) = &mut endpoint.handler {
            if let Some(solo) = msg.args.get(0) {
                handler(TrackSoloArgs {
                    solo: solo.clone().bool().unwrap(),
                });
            }
        }
        return;
    }
    if let Some(args) = match_addr(addr, "/track/{track_guid}/send/{send_index}/guid") {
        let send_index = args[1].parse::<i32>().unwrap();
        let track_guid = &args[2];
        let track = reaper.track(track_guid.clone());
        let send = track.send(send_index);
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
    if let Some(args) = match_addr(addr, "/track/{track_guid}/send/{send_index}/pan") {
        let send_index = args[1].parse::<i32>().unwrap();
        let track_guid = &args[2];
        let track = reaper.track(track_guid.clone());
        let send = track.send(send_index);
        let mut endpoint = send.pan();
        if let Some(handler) = &mut endpoint.handler {
            if let Some(pan) = msg.args.get(0) {
                handler(TrackSendPanArgs {
                    pan: pan.clone().float().unwrap(),
                });
            }
        }
        return;
    }
    if let Some(args) = match_addr(addr, "/track/{track_guid}/send/{send_index}/volume") {
        let send_index = args[1].parse::<i32>().unwrap();
        let track_guid = &args[2];
        let track = reaper.track(track_guid.clone());
        let send = track.send(send_index);
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
    if let Some(args) = match_addr(addr, "/track/{track_guid}/color") {
        let track_guid = &args[1];
        let track = reaper.track(track_guid.clone());
        let mut endpoint = track.color();
        if let Some(handler) = &mut endpoint.handler {
            if let Some(color) = msg.args.get(0) {
                handler(TrackColorArgs {
                    color: color.clone().int().unwrap(),
                });
            }
        }
        return;
    }
    if let Some(args) = match_addr(addr, "/track/{track_guid}/rec-arm") {
        let track_guid = &args[1];
        let track = reaper.track(track_guid.clone());
        let mut endpoint = track.rec_arm();
        if let Some(handler) = &mut endpoint.handler {
            if let Some(rec_arm) = msg.args.get(0) {
                handler(TrackRecArmArgs {
                    rec_arm: rec_arm.clone().bool().unwrap(),
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
    if let Some(args) = match_addr(addr, "/track/{track_guid}/name") {
        let track_guid = &args[1];
        let track = reaper.track(track_guid.clone());
        let mut endpoint = track.name();
        if let Some(handler) = &mut endpoint.handler {
            if let Some(name) = msg.args.get(0) {
                handler(TrackNameArgs {
                    name: name.clone().string().unwrap().clone(),
                });
            }
        }
        return;
    }
    if let Some(args) = match_addr(addr, "/track/{track_guid}/delete") {
        let track_guid = &args[1];
        let track = reaper.track(track_guid.clone());
        let mut endpoint = track.delete();
        if let Some(handler) = &mut endpoint.handler {}
        return;
    }
    if let Some(args) = match_addr(addr, "/track/{track_guid}/volume") {
        let track_guid = &args[1];
        let track = reaper.track(track_guid.clone());
        let mut endpoint = track.volume();
        if let Some(handler) = &mut endpoint.handler {
            if let Some(volume) = msg.args.get(0) {
                handler(TrackVolumeArgs {
                    volume: volume.clone().float().unwrap(),
                });
            }
        }
        return;
    }
    if let Some(args) = match_addr(addr, "/track/{track_guid}/mute") {
        let track_guid = &args[1];
        let track = reaper.track(track_guid.clone());
        let mut endpoint = track.mute();
        if let Some(handler) = &mut endpoint.handler {
            if let Some(mute) = msg.args.get(0) {
                handler(TrackMuteArgs {
                    mute: mute.clone().bool().unwrap(),
                });
            }
        }
        return;
    }
    if let Some(args) = match_addr(addr, "/track/{track_guid}/pan") {
        let track_guid = &args[1];
        let track = reaper.track(track_guid.clone());
        let mut endpoint = track.pan();
        if let Some(handler) = &mut endpoint.handler {
            if let Some(pan) = msg.args.get(0) {
                handler(TrackPanArgs {
                    pan: pan.clone().float().unwrap(),
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
    log_unknown(addr);
}
