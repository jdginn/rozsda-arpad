// AUTO-GENERATED CODE. DO NOT EDIT!

use std::net::UdpSocket;
use std::sync::Arc;

use crate::traits::{Bind, Query, Set};

use crate::osc::route_context::ContextTrait;

#[derive(Debug)]
pub struct OscError;

#[derive(Debug)]
pub struct NumTracksArgs {
    pub num_tracks: i32, // number of tracks in the current project
}

pub type NumTracksHandler = Box<dyn FnMut(NumTracksArgs) + 'static>;

pub struct NumTracks {
    socket: Arc<UdpSocket>,
    handler: Option<NumTracksHandler>,
}

/// /num_tracks
impl Bind<NumTracksArgs> for NumTracks {
    fn bind<F>(&mut self, callback: F)
    where
        F: FnMut(NumTracksArgs) + 'static,
    {
        self.handler = Some(Box::new(callback));
    }
}

/// /num_tracks
impl Query for NumTracks {
    type Error = OscError;
    fn query(&self) -> Result<(), Self::Error> {
        let osc_address = format!("/num_tracks");
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

#[derive(Debug)]
pub struct TrackAllGuidsArgs {}

pub type TrackAllGuidsHandler = Box<dyn FnMut(TrackAllGuidsArgs) + 'static>;

pub struct TrackAllGuids {
    socket: Arc<UdpSocket>,
    handler: Option<TrackAllGuidsHandler>,
}

/// /track/all_guids
impl Bind<TrackAllGuidsArgs> for TrackAllGuids {
    fn bind<F>(&mut self, callback: F)
    where
        F: FnMut(TrackAllGuidsArgs) + 'static,
    {
        self.handler = Some(Box::new(callback));
    }
}

/// /track/all_guids
impl Query for TrackAllGuids {
    type Error = OscError;
    fn query(&self) -> Result<(), Self::Error> {
        let osc_address = format!("/track/all_guids");
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

#[derive(Debug)]
pub struct TrackIndexArgs {
    pub index: i32, // index of the track in the project according to reaper's mixer view
}

pub type TrackIndexHandler = Box<dyn FnMut(TrackIndexArgs) + 'static>;

pub struct TrackIndex {
    socket: Arc<UdpSocket>,
    handler: Option<TrackIndexHandler>,
    pub track_guid: String,
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

#[derive(Debug)]
pub struct TrackDeleteArgs {}

pub type TrackDeleteHandler = Box<dyn FnMut(TrackDeleteArgs) + 'static>;

pub struct TrackDelete {
    socket: Arc<UdpSocket>,
    handler: Option<TrackDeleteHandler>,
    pub track_guid: String,
}

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

#[derive(Debug)]
pub struct TrackNameArgs {
    pub name: String, // name of the track
}

pub type TrackNameHandler = Box<dyn FnMut(TrackNameArgs) + 'static>;

pub struct TrackName {
    socket: Arc<UdpSocket>,
    handler: Option<TrackNameHandler>,
    pub track_guid: String,
}

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
impl Bind<TrackNameArgs> for TrackName {
    fn bind<F>(&mut self, callback: F)
    where
        F: FnMut(TrackNameArgs) + 'static,
    {
        self.handler = Some(Box::new(callback));
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

#[derive(Debug)]
pub struct TrackSelectedArgs {
    pub selected: bool, // true means track is selected
}

pub type TrackSelectedHandler = Box<dyn FnMut(TrackSelectedArgs) + 'static>;

pub struct TrackSelected {
    socket: Arc<UdpSocket>,
    handler: Option<TrackSelectedHandler>,
    pub track_guid: String,
}

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
impl Bind<TrackSelectedArgs> for TrackSelected {
    fn bind<F>(&mut self, callback: F)
    where
        F: FnMut(TrackSelectedArgs) + 'static,
    {
        self.handler = Some(Box::new(callback));
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

#[derive(Debug)]
pub struct TrackVolumeArgs {
    pub volume: f32, // volume of the track, normalized to 0 to 1.0
}

pub type TrackVolumeHandler = Box<dyn FnMut(TrackVolumeArgs) + 'static>;

pub struct TrackVolume {
    socket: Arc<UdpSocket>,
    handler: Option<TrackVolumeHandler>,
    pub track_guid: String,
}

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
impl Bind<TrackVolumeArgs> for TrackVolume {
    fn bind<F>(&mut self, callback: F)
    where
        F: FnMut(TrackVolumeArgs) + 'static,
    {
        self.handler = Some(Box::new(callback));
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

#[derive(Debug)]
pub struct TrackPanArgs {
    pub pan: f32, // pan of the track, normalized to -1.0 to 1.0
}

pub type TrackPanHandler = Box<dyn FnMut(TrackPanArgs) + 'static>;

pub struct TrackPan {
    socket: Arc<UdpSocket>,
    handler: Option<TrackPanHandler>,
    pub track_guid: String,
}

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
impl Bind<TrackPanArgs> for TrackPan {
    fn bind<F>(&mut self, callback: F)
    where
        F: FnMut(TrackPanArgs) + 'static,
    {
        self.handler = Some(Box::new(callback));
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

#[derive(Debug)]
pub struct TrackMuteArgs {
    pub mute: bool, // true means track is muted
}

pub type TrackMuteHandler = Box<dyn FnMut(TrackMuteArgs) + 'static>;

pub struct TrackMute {
    socket: Arc<UdpSocket>,
    handler: Option<TrackMuteHandler>,
    pub track_guid: String,
}

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
impl Bind<TrackMuteArgs> for TrackMute {
    fn bind<F>(&mut self, callback: F)
    where
        F: FnMut(TrackMuteArgs) + 'static,
    {
        self.handler = Some(Box::new(callback));
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

#[derive(Debug)]
pub struct TrackSoloArgs {
    pub solo: bool, // true means track is soloed
}

pub type TrackSoloHandler = Box<dyn FnMut(TrackSoloArgs) + 'static>;

pub struct TrackSolo {
    socket: Arc<UdpSocket>,
    handler: Option<TrackSoloHandler>,
    pub track_guid: String,
}

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
impl Bind<TrackSoloArgs> for TrackSolo {
    fn bind<F>(&mut self, callback: F)
    where
        F: FnMut(TrackSoloArgs) + 'static,
    {
        self.handler = Some(Box::new(callback));
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

#[derive(Debug)]
pub struct TrackRecArmArgs {
    pub rec_arm: bool, // true means track is armed for recording
}

pub type TrackRecArmHandler = Box<dyn FnMut(TrackRecArmArgs) + 'static>;

pub struct TrackRecArm {
    socket: Arc<UdpSocket>,
    handler: Option<TrackRecArmHandler>,
    pub track_guid: String,
}

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
impl Bind<TrackRecArmArgs> for TrackRecArm {
    fn bind<F>(&mut self, callback: F)
    where
        F: FnMut(TrackRecArmArgs) + 'static,
    {
        self.handler = Some(Box::new(callback));
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

#[derive(Debug)]
pub struct TrackSendGuidArgs {
    pub guid: String, // unique identifier for the send
}

pub type TrackSendGuidHandler = Box<dyn FnMut(TrackSendGuidArgs) + 'static>;

pub struct TrackSendGuid {
    socket: Arc<UdpSocket>,
    handler: Option<TrackSendGuidHandler>,
    pub track_guid: String,
    pub send_index: i32,
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

#[derive(Debug)]
pub struct TrackSendVolumeArgs {
    pub volume: f32, // volume of the send, normalized to 0 to 1.
}

pub type TrackSendVolumeHandler = Box<dyn FnMut(TrackSendVolumeArgs) + 'static>;

pub struct TrackSendVolume {
    socket: Arc<UdpSocket>,
    handler: Option<TrackSendVolumeHandler>,
    pub track_guid: String,
    pub send_index: i32,
}

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
impl Bind<TrackSendVolumeArgs> for TrackSendVolume {
    fn bind<F>(&mut self, callback: F)
    where
        F: FnMut(TrackSendVolumeArgs) + 'static,
    {
        self.handler = Some(Box::new(callback));
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

#[derive(Debug)]
pub struct TrackSendPanArgs {
    pub pan: f32, // pan of the send, normalized to -1.0 to 1.0
}

pub type TrackSendPanHandler = Box<dyn FnMut(TrackSendPanArgs) + 'static>;

pub struct TrackSendPan {
    socket: Arc<UdpSocket>,
    handler: Option<TrackSendPanHandler>,
    pub track_guid: String,
    pub send_index: i32,
}

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
impl Bind<TrackSendPanArgs> for TrackSendPan {
    fn bind<F>(&mut self, callback: F)
    where
        F: FnMut(TrackSendPanArgs) + 'static,
    {
        self.handler = Some(Box::new(callback));
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

#[derive(Debug)]
pub struct TrackColorArgs {
    pub color: i32, // color of the track, represented as an RGB integer
}

pub type TrackColorHandler = Box<dyn FnMut(TrackColorArgs) + 'static>;

pub struct TrackColor {
    socket: Arc<UdpSocket>,
    handler: Option<TrackColorHandler>,
    pub track_guid: String,
}

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
impl Bind<TrackColorArgs> for TrackColor {
    fn bind<F>(&mut self, callback: F)
    where
        F: FnMut(TrackColorArgs) + 'static,
    {
        self.handler = Some(Box::new(callback));
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

#[derive(Debug)]
pub struct TrackFxGuidArgs {
    pub guid: String, // unique identifier for the FX
}

pub type TrackFxGuidHandler = Box<dyn FnMut(TrackFxGuidArgs) + 'static>;

pub struct TrackFxGuid {
    socket: Arc<UdpSocket>,
    handler: Option<TrackFxGuidHandler>,
    pub track_guid: String,
    pub fx_idx: i32,
}

/// /track/{track_guid}/fx/{fx_idx}/guid
impl Bind<TrackFxGuidArgs> for TrackFxGuid {
    fn bind<F>(&mut self, callback: F)
    where
        F: FnMut(TrackFxGuidArgs) + 'static,
    {
        self.handler = Some(Box::new(callback));
    }
}

/// /track/{track_guid}/fx/{fx_idx}/guid
impl Query for TrackFxGuid {
    type Error = OscError;
    fn query(&self) -> Result<(), Self::Error> {
        let osc_address = format!("/track/{}/fx/{}/guid", self.track_guid, self.fx_idx);
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

#[derive(Debug)]
pub struct TrackFxNameArgs {
    pub name: String, // name of the FX
}

pub type TrackFxNameHandler = Box<dyn FnMut(TrackFxNameArgs) + 'static>;

pub struct TrackFxName {
    socket: Arc<UdpSocket>,
    handler: Option<TrackFxNameHandler>,
    pub track_guid: String,
    pub fx_idx: i32,
}

/// /track/{track_guid}/fx/{fx_idx}/name
impl Bind<TrackFxNameArgs> for TrackFxName {
    fn bind<F>(&mut self, callback: F)
    where
        F: FnMut(TrackFxNameArgs) + 'static,
    {
        self.handler = Some(Box::new(callback));
    }
}

/// /track/{track_guid}/fx/{fx_idx}/name
impl Query for TrackFxName {
    type Error = OscError;
    fn query(&self) -> Result<(), Self::Error> {
        let osc_address = format!("/track/{}/fx/{}/name", self.track_guid, self.fx_idx);
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

#[derive(Debug)]
pub struct TrackFxEnabledArgs {
    pub enabled: bool, // true if the FX is enabled
}

pub type TrackFxEnabledHandler = Box<dyn FnMut(TrackFxEnabledArgs) + 'static>;

pub struct TrackFxEnabled {
    socket: Arc<UdpSocket>,
    handler: Option<TrackFxEnabledHandler>,
    pub track_guid: String,
    pub fx_idx: i32,
}

/// /track/{track_guid}/fx/{fx_idx}/enabled
impl Set<TrackFxEnabledArgs> for TrackFxEnabled {
    type Error = OscError;
    fn set(&mut self, args: TrackFxEnabledArgs) -> Result<(), Self::Error> {
        let osc_address = format!("/track/{}/fx/{}/enabled", self.track_guid, self.fx_idx);
        let osc_msg = rosc::OscMessage {
            addr: osc_address,
            args: vec![rosc::OscType::Bool(args.enabled)],
        };
        let packet = rosc::OscPacket::Message(osc_msg);
        let buf = rosc::encoder::encode(&packet).map_err(|_| OscError)?;
        self.socket.send(&buf).map_err(|_| OscError)?;
        Ok(())
    }
}

/// /track/{track_guid}/fx/{fx_idx}/enabled
impl Bind<TrackFxEnabledArgs> for TrackFxEnabled {
    fn bind<F>(&mut self, callback: F)
    where
        F: FnMut(TrackFxEnabledArgs) + 'static,
    {
        self.handler = Some(Box::new(callback));
    }
}

/// /track/{track_guid}/fx/{fx_idx}/enabled
impl Query for TrackFxEnabled {
    type Error = OscError;
    fn query(&self) -> Result<(), Self::Error> {
        let osc_address = format!("/track/{}/fx/{}/enabled", self.track_guid, self.fx_idx);
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

#[derive(Debug)]
pub struct TrackFxParamCountArgs {
    pub param_count: i32, // number of parameters for the FX
}

pub type TrackFxParamCountHandler = Box<dyn FnMut(TrackFxParamCountArgs) + 'static>;

pub struct TrackFxParamCount {
    socket: Arc<UdpSocket>,
    handler: Option<TrackFxParamCountHandler>,
    pub track_guid: String,
    pub fx_idx: i32,
}

/// /track/{track_guid}/fx/{fx_idx}/param_count
impl Bind<TrackFxParamCountArgs> for TrackFxParamCount {
    fn bind<F>(&mut self, callback: F)
    where
        F: FnMut(TrackFxParamCountArgs) + 'static,
    {
        self.handler = Some(Box::new(callback));
    }
}

/// /track/{track_guid}/fx/{fx_idx}/param_count
impl Query for TrackFxParamCount {
    type Error = OscError;
    fn query(&self) -> Result<(), Self::Error> {
        let osc_address = format!("/track/{}/fx/{}/param_count", self.track_guid, self.fx_idx);
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

#[derive(Debug)]
pub struct TrackFxParamNameArgs {
    pub param_name: String, // name of the parameter
}

pub type TrackFxParamNameHandler = Box<dyn FnMut(TrackFxParamNameArgs) + 'static>;

pub struct TrackFxParamName {
    socket: Arc<UdpSocket>,
    handler: Option<TrackFxParamNameHandler>,
    pub track_guid: String,
    pub fx_idx: i32,
    pub param_idx: i32,
}

/// /track/{track_guid}/fx/{fx_idx}/param/{param_idx}/name
impl Bind<TrackFxParamNameArgs> for TrackFxParamName {
    fn bind<F>(&mut self, callback: F)
    where
        F: FnMut(TrackFxParamNameArgs) + 'static,
    {
        self.handler = Some(Box::new(callback));
    }
}

/// /track/{track_guid}/fx/{fx_idx}/param/{param_idx}/name
impl Query for TrackFxParamName {
    type Error = OscError;
    fn query(&self) -> Result<(), Self::Error> {
        let osc_address = format!(
            "/track/{}/fx/{}/param/{}/name",
            self.track_guid, self.fx_idx, self.param_idx
        );
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

#[derive(Debug)]
pub struct TrackFxParamValueArgs {
    pub value: f32, // value of the parameter
}

pub type TrackFxParamValueHandler = Box<dyn FnMut(TrackFxParamValueArgs) + 'static>;

pub struct TrackFxParamValue {
    socket: Arc<UdpSocket>,
    handler: Option<TrackFxParamValueHandler>,
    pub track_guid: String,
    pub fx_idx: i32,
    pub param_idx: i32,
}

/// /track/{track_guid}/fx/{fx_idx}/param/{param_idx}/value
impl Set<TrackFxParamValueArgs> for TrackFxParamValue {
    type Error = OscError;
    fn set(&mut self, args: TrackFxParamValueArgs) -> Result<(), Self::Error> {
        let osc_address = format!(
            "/track/{}/fx/{}/param/{}/value",
            self.track_guid, self.fx_idx, self.param_idx
        );
        let osc_msg = rosc::OscMessage {
            addr: osc_address,
            args: vec![rosc::OscType::Float(args.value)],
        };
        let packet = rosc::OscPacket::Message(osc_msg);
        let buf = rosc::encoder::encode(&packet).map_err(|_| OscError)?;
        self.socket.send(&buf).map_err(|_| OscError)?;
        Ok(())
    }
}

/// /track/{track_guid}/fx/{fx_idx}/param/{param_idx}/value
impl Bind<TrackFxParamValueArgs> for TrackFxParamValue {
    fn bind<F>(&mut self, callback: F)
    where
        F: FnMut(TrackFxParamValueArgs) + 'static,
    {
        self.handler = Some(Box::new(callback));
    }
}

/// /track/{track_guid}/fx/{fx_idx}/param/{param_idx}/value
impl Query for TrackFxParamValue {
    type Error = OscError;
    fn query(&self) -> Result<(), Self::Error> {
        let osc_address = format!(
            "/track/{}/fx/{}/param/{}/value",
            self.track_guid, self.fx_idx, self.param_idx
        );
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

#[derive(Debug)]
pub struct TrackFxParamMinArgs {
    pub min: f32, // minimum value of the parameter
}

pub type TrackFxParamMinHandler = Box<dyn FnMut(TrackFxParamMinArgs) + 'static>;

pub struct TrackFxParamMin {
    socket: Arc<UdpSocket>,
    handler: Option<TrackFxParamMinHandler>,
    pub track_guid: String,
    pub fx_idx: i32,
    pub param_idx: i32,
}

/// /track/{track_guid}/fx/{fx_idx}/param/{param_idx}/min
impl Bind<TrackFxParamMinArgs> for TrackFxParamMin {
    fn bind<F>(&mut self, callback: F)
    where
        F: FnMut(TrackFxParamMinArgs) + 'static,
    {
        self.handler = Some(Box::new(callback));
    }
}

/// /track/{track_guid}/fx/{fx_idx}/param/{param_idx}/min
impl Query for TrackFxParamMin {
    type Error = OscError;
    fn query(&self) -> Result<(), Self::Error> {
        let osc_address = format!(
            "/track/{}/fx/{}/param/{}/min",
            self.track_guid, self.fx_idx, self.param_idx
        );
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

#[derive(Debug)]
pub struct TrackFxParamMaxArgs {
    pub max: f32, // maximum value of the parameter
}

pub type TrackFxParamMaxHandler = Box<dyn FnMut(TrackFxParamMaxArgs) + 'static>;

pub struct TrackFxParamMax {
    socket: Arc<UdpSocket>,
    handler: Option<TrackFxParamMaxHandler>,
    pub track_guid: String,
    pub fx_idx: i32,
    pub param_idx: i32,
}

/// /track/{track_guid}/fx/{fx_idx}/param/{param_idx}/max
impl Bind<TrackFxParamMaxArgs> for TrackFxParamMax {
    fn bind<F>(&mut self, callback: F)
    where
        F: FnMut(TrackFxParamMaxArgs) + 'static,
    {
        self.handler = Some(Box::new(callback));
    }
}

/// /track/{track_guid}/fx/{fx_idx}/param/{param_idx}/max
impl Query for TrackFxParamMax {
    type Error = OscError;
    fn query(&self) -> Result<(), Self::Error> {
        let osc_address = format!(
            "/track/{}/fx/{}/param/{}/max",
            self.track_guid, self.fx_idx, self.param_idx
        );
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

#[derive(Debug)]
pub struct TrackFxInfoArgs {}

pub type TrackFxInfoHandler = Box<dyn FnMut(TrackFxInfoArgs) + 'static>;

pub struct TrackFxInfo {
    socket: Arc<UdpSocket>,
    handler: Option<TrackFxInfoHandler>,
    pub track_guid: String,
    pub fx_idx: i32,
}

/// /track/{track_guid}/fx/{fx_idx}/info
impl Query for TrackFxInfo {
    type Error = OscError;
    fn query(&self) -> Result<(), Self::Error> {
        let osc_address = format!("/track/{}/fx/{}/info", self.track_guid, self.fx_idx);
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

#[derive(Debug)]
pub struct FxinfoNameArgs {
    pub name: String, // name of the FX
}

pub type FxinfoNameHandler = Box<dyn FnMut(FxinfoNameArgs) + 'static>;

pub struct FxinfoName {
    socket: Arc<UdpSocket>,
    handler: Option<FxinfoNameHandler>,
    pub ident: String,
}

/// /fxinfo/{ident}/name
impl Bind<FxinfoNameArgs> for FxinfoName {
    fn bind<F>(&mut self, callback: F)
    where
        F: FnMut(FxinfoNameArgs) + 'static,
    {
        self.handler = Some(Box::new(callback));
    }
}

#[derive(Debug)]
pub struct FxinfoParamCountArgs {
    pub param_count: i32, // number of parameters for the FX
}

pub type FxinfoParamCountHandler = Box<dyn FnMut(FxinfoParamCountArgs) + 'static>;

pub struct FxinfoParamCount {
    socket: Arc<UdpSocket>,
    handler: Option<FxinfoParamCountHandler>,
    pub ident: String,
}

/// /fxinfo/{ident}/param_count
impl Bind<FxinfoParamCountArgs> for FxinfoParamCount {
    fn bind<F>(&mut self, callback: F)
    where
        F: FnMut(FxinfoParamCountArgs) + 'static,
    {
        self.handler = Some(Box::new(callback));
    }
}

/// /fxinfo/{ident}/param_count
impl Query for FxinfoParamCount {
    type Error = OscError;
    fn query(&self) -> Result<(), Self::Error> {
        let osc_address = format!("/fxinfo/{}/param_count", self.ident);
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

#[derive(Debug)]
pub struct FxinfoParamNameArgs {
    pub param_name: String, // name of the parameter
}

pub type FxinfoParamNameHandler = Box<dyn FnMut(FxinfoParamNameArgs) + 'static>;

pub struct FxinfoParamName {
    socket: Arc<UdpSocket>,
    handler: Option<FxinfoParamNameHandler>,
    pub ident: String,
    pub param_idx: i32,
}

/// /fxinfo/{ident}/param/{param_idx}/name
impl Bind<FxinfoParamNameArgs> for FxinfoParamName {
    fn bind<F>(&mut self, callback: F)
    where
        F: FnMut(FxinfoParamNameArgs) + 'static,
    {
        self.handler = Some(Box::new(callback));
    }
}

/// /fxinfo/{ident}/param/{param_idx}/name
impl Query for FxinfoParamName {
    type Error = OscError;
    fn query(&self) -> Result<(), Self::Error> {
        let osc_address = format!("/fxinfo/{}/param/{}/name", self.ident, self.param_idx);
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

#[derive(Debug)]
pub struct FxinfoParamMinArgs {
    pub param_min: f32, // minimum raw value of the parameter
}

pub type FxinfoParamMinHandler = Box<dyn FnMut(FxinfoParamMinArgs) + 'static>;

pub struct FxinfoParamMin {
    socket: Arc<UdpSocket>,
    handler: Option<FxinfoParamMinHandler>,
    pub ident: String,
    pub param_idx: i32,
}

/// /fxinfo/{ident}/param/{param_idx}/min
impl Bind<FxinfoParamMinArgs> for FxinfoParamMin {
    fn bind<F>(&mut self, callback: F)
    where
        F: FnMut(FxinfoParamMinArgs) + 'static,
    {
        self.handler = Some(Box::new(callback));
    }
}

/// /fxinfo/{ident}/param/{param_idx}/min
impl Query for FxinfoParamMin {
    type Error = OscError;
    fn query(&self) -> Result<(), Self::Error> {
        let osc_address = format!("/fxinfo/{}/param/{}/min", self.ident, self.param_idx);
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

#[derive(Debug)]
pub struct FxinfoParamMaxArgs {
    pub param_max: f32, // maximum raw value of the parameter
}

pub type FxinfoParamMaxHandler = Box<dyn FnMut(FxinfoParamMaxArgs) + 'static>;

pub struct FxinfoParamMax {
    socket: Arc<UdpSocket>,
    handler: Option<FxinfoParamMaxHandler>,
    pub ident: String,
    pub param_idx: i32,
}

/// /fxinfo/{ident}/param/{param_idx}/max
impl Bind<FxinfoParamMaxArgs> for FxinfoParamMax {
    fn bind<F>(&mut self, callback: F)
    where
        F: FnMut(FxinfoParamMaxArgs) + 'static,
    {
        self.handler = Some(Box::new(callback));
    }
}

/// /fxinfo/{ident}/param/{param_idx}/max
impl Query for FxinfoParamMax {
    type Error = OscError;
    fn query(&self) -> Result<(), Self::Error> {
        let osc_address = format!("/fxinfo/{}/param/{}/max", self.ident, self.param_idx);
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

#[derive(Debug)]
pub struct FxinfoArgs {}

pub type FxinfoHandler = Box<dyn FnMut(FxinfoArgs) + 'static>;

pub struct Fxinfo {
    socket: Arc<UdpSocket>,
    handler: Option<FxinfoHandler>,
}

/// /fxinfo
impl Query for Fxinfo {
    type Error = OscError;
    fn query(&self) -> Result<(), Self::Error> {
        let osc_address = format!("/fxinfo");
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

pub mod context {
    use crate::osc::generated_osc::ContextTrait;

    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    pub struct Fxinfo {
        pub ident: String,
    }

    impl ContextTrait for Fxinfo {}

    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    pub struct FxinfoParam {
        pub ident: String,
        pub param_idx: i32,
    }

    impl ContextTrait for FxinfoParam {}

    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    pub struct Track {
        pub track_guid: String,
    }

    impl ContextTrait for Track {}

    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    pub struct TrackFx {
        pub track_guid: String,
        pub fx_idx: i32,
    }

    impl ContextTrait for TrackFx {}

    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    pub struct TrackFxParam {
        pub track_guid: String,
        pub fx_idx: i32,
        pub param_idx: i32,
    }

    impl ContextTrait for TrackFxParam {}

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
    pub struct Fxinfo {}

    impl ContextKindTrait for Fxinfo {
        type Context = context::Fxinfo;

        fn context_name() -> &'static str {
            "Fxinfo"
        }

        fn parse(osc_address: &str) -> Option<context::Fxinfo> {
            let re = Regex::new(r"^/fxinfo/([^/]+)/name$").unwrap();
            re.captures(osc_address).map(|caps| context::Fxinfo {
                ident: caps[1].to_string(),
            })
        }
    }

    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    pub struct FxinfoParam {}

    impl ContextKindTrait for FxinfoParam {
        type Context = context::FxinfoParam;

        fn context_name() -> &'static str {
            "FxinfoParam"
        }

        fn parse(osc_address: &str) -> Option<context::FxinfoParam> {
            let re = Regex::new(r"^/fxinfo/([^/]+)/param/([^/]+)/name$").unwrap();
            re.captures(osc_address).map(|caps| context::FxinfoParam {
                ident: caps[1].to_string(),
                param_idx: caps[2].parse().unwrap(),
            })
        }
    }

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
    pub struct TrackFx {}

    impl ContextKindTrait for TrackFx {
        type Context = context::TrackFx;

        fn context_name() -> &'static str {
            "TrackFx"
        }

        fn parse(osc_address: &str) -> Option<context::TrackFx> {
            let re = Regex::new(r"^/track/([^/]+)/fx/([^/]+)/guid$").unwrap();
            re.captures(osc_address).map(|caps| context::TrackFx {
                track_guid: caps[1].to_string(),
                fx_idx: caps[2].parse().unwrap(),
            })
        }
    }

    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    pub struct TrackFxParam {}

    impl ContextKindTrait for TrackFxParam {
        type Context = context::TrackFxParam;

        fn context_name() -> &'static str {
            "TrackFxParam"
        }

        fn parse(osc_address: &str) -> Option<context::TrackFxParam> {
            let re = Regex::new(r"^/track/([^/]+)/fx/([^/]+)/param/([^/]+)/name$").unwrap();
            re.captures(osc_address).map(|caps| context::TrackFxParam {
                track_guid: caps[1].to_string(),
                fx_idx: caps[2].parse().unwrap(),
                param_idx: caps[3].parse().unwrap(),
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
}

impl Reaper {
    pub fn new(socket: Arc<UdpSocket>) -> Self {
        Self { socket }
    }
}

impl Reaper {
    pub fn num_tracks(&self) -> NumTracks {
        NumTracks {
            socket: self.socket.clone(),
            handler: None,
        }
    }
    pub fn track_all_guids(&self) -> TrackAllGuids {
        TrackAllGuids {
            socket: self.socket.clone(),
            handler: None,
        }
    }
    pub fn track_index(&self, track_guid: String) -> TrackIndex {
        TrackIndex {
            socket: self.socket.clone(),
            handler: None,
            track_guid: track_guid,
        }
    }
    pub fn track_delete(&self, track_guid: String) -> TrackDelete {
        TrackDelete {
            socket: self.socket.clone(),
            handler: None,
            track_guid: track_guid,
        }
    }
    pub fn track_name(&self, track_guid: String) -> TrackName {
        TrackName {
            socket: self.socket.clone(),
            handler: None,
            track_guid: track_guid,
        }
    }
    pub fn track_selected(&self, track_guid: String) -> TrackSelected {
        TrackSelected {
            socket: self.socket.clone(),
            handler: None,
            track_guid: track_guid,
        }
    }
    pub fn track_volume(&self, track_guid: String) -> TrackVolume {
        TrackVolume {
            socket: self.socket.clone(),
            handler: None,
            track_guid: track_guid,
        }
    }
    pub fn track_pan(&self, track_guid: String) -> TrackPan {
        TrackPan {
            socket: self.socket.clone(),
            handler: None,
            track_guid: track_guid,
        }
    }
    pub fn track_mute(&self, track_guid: String) -> TrackMute {
        TrackMute {
            socket: self.socket.clone(),
            handler: None,
            track_guid: track_guid,
        }
    }
    pub fn track_solo(&self, track_guid: String) -> TrackSolo {
        TrackSolo {
            socket: self.socket.clone(),
            handler: None,
            track_guid: track_guid,
        }
    }
    pub fn track_rec_arm(&self, track_guid: String) -> TrackRecArm {
        TrackRecArm {
            socket: self.socket.clone(),
            handler: None,
            track_guid: track_guid,
        }
    }
    pub fn track_send_guid(&self, track_guid: String, send_index: i32) -> TrackSendGuid {
        TrackSendGuid {
            socket: self.socket.clone(),
            handler: None,
            track_guid: track_guid,
            send_index: send_index,
        }
    }
    pub fn track_send_volume(&self, track_guid: String, send_index: i32) -> TrackSendVolume {
        TrackSendVolume {
            socket: self.socket.clone(),
            handler: None,
            track_guid: track_guid,
            send_index: send_index,
        }
    }
    pub fn track_send_pan(&self, track_guid: String, send_index: i32) -> TrackSendPan {
        TrackSendPan {
            socket: self.socket.clone(),
            handler: None,
            track_guid: track_guid,
            send_index: send_index,
        }
    }
    pub fn track_color(&self, track_guid: String) -> TrackColor {
        TrackColor {
            socket: self.socket.clone(),
            handler: None,
            track_guid: track_guid,
        }
    }
    pub fn track_fx_guid(&self, track_guid: String, fx_idx: i32) -> TrackFxGuid {
        TrackFxGuid {
            socket: self.socket.clone(),
            handler: None,
            track_guid: track_guid,
            fx_idx: fx_idx,
        }
    }
    pub fn track_fx_name(&self, track_guid: String, fx_idx: i32) -> TrackFxName {
        TrackFxName {
            socket: self.socket.clone(),
            handler: None,
            track_guid: track_guid,
            fx_idx: fx_idx,
        }
    }
    pub fn track_fx_enabled(&self, track_guid: String, fx_idx: i32) -> TrackFxEnabled {
        TrackFxEnabled {
            socket: self.socket.clone(),
            handler: None,
            track_guid: track_guid,
            fx_idx: fx_idx,
        }
    }
    pub fn track_fx_param_count(&self, track_guid: String, fx_idx: i32) -> TrackFxParamCount {
        TrackFxParamCount {
            socket: self.socket.clone(),
            handler: None,
            track_guid: track_guid,
            fx_idx: fx_idx,
        }
    }
    pub fn track_fx_param_name(
        &self,
        track_guid: String,
        fx_idx: i32,
        param_idx: i32,
    ) -> TrackFxParamName {
        TrackFxParamName {
            socket: self.socket.clone(),
            handler: None,
            track_guid: track_guid,
            fx_idx: fx_idx,
            param_idx: param_idx,
        }
    }
    pub fn track_fx_param_value(
        &self,
        track_guid: String,
        fx_idx: i32,
        param_idx: i32,
    ) -> TrackFxParamValue {
        TrackFxParamValue {
            socket: self.socket.clone(),
            handler: None,
            track_guid: track_guid,
            fx_idx: fx_idx,
            param_idx: param_idx,
        }
    }
    pub fn track_fx_param_min(
        &self,
        track_guid: String,
        fx_idx: i32,
        param_idx: i32,
    ) -> TrackFxParamMin {
        TrackFxParamMin {
            socket: self.socket.clone(),
            handler: None,
            track_guid: track_guid,
            fx_idx: fx_idx,
            param_idx: param_idx,
        }
    }
    pub fn track_fx_param_max(
        &self,
        track_guid: String,
        fx_idx: i32,
        param_idx: i32,
    ) -> TrackFxParamMax {
        TrackFxParamMax {
            socket: self.socket.clone(),
            handler: None,
            track_guid: track_guid,
            fx_idx: fx_idx,
            param_idx: param_idx,
        }
    }
    pub fn track_fx_info(&self, track_guid: String, fx_idx: i32) -> TrackFxInfo {
        TrackFxInfo {
            socket: self.socket.clone(),
            handler: None,
            track_guid: track_guid,
            fx_idx: fx_idx,
        }
    }
    pub fn fxinfo_name(&self, ident: String) -> FxinfoName {
        FxinfoName {
            socket: self.socket.clone(),
            handler: None,
            ident: ident,
        }
    }
    pub fn fxinfo_param_count(&self, ident: String) -> FxinfoParamCount {
        FxinfoParamCount {
            socket: self.socket.clone(),
            handler: None,
            ident: ident,
        }
    }
    pub fn fxinfo_param_name(&self, ident: String, param_idx: i32) -> FxinfoParamName {
        FxinfoParamName {
            socket: self.socket.clone(),
            handler: None,
            ident: ident,
            param_idx: param_idx,
        }
    }
    pub fn fxinfo_param_min(&self, ident: String, param_idx: i32) -> FxinfoParamMin {
        FxinfoParamMin {
            socket: self.socket.clone(),
            handler: None,
            ident: ident,
            param_idx: param_idx,
        }
    }
    pub fn fxinfo_param_max(&self, ident: String, param_idx: i32) -> FxinfoParamMax {
        FxinfoParamMax {
            socket: self.socket.clone(),
            handler: None,
            ident: ident,
            param_idx: param_idx,
        }
    }
    pub fn fxinfo(&self) -> Fxinfo {
        Fxinfo {
            socket: self.socket.clone(),
            handler: None,
        }
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
    if let Some(args) = match_addr(addr, "/num_tracks") {
        let mut endpoint = reaper.num_tracks();
        if let Some(handler) = &mut endpoint.handler {
            if let Some(num_tracks) = msg.args.get(0) {
                handler(NumTracksArgs {
                    num_tracks: num_tracks.clone().int().unwrap(),
                });
            }
        }
        return;
    }
    if let Some(args) = match_addr(addr, "/track/all_guids") {
        let mut endpoint = reaper.track_all_guids();
        if let Some(handler) = &mut endpoint.handler {}
        return;
    }
    if let Some(args) = match_addr(addr, "/track/{track_guid}/index") {
        let track_guid = args[0].clone();
        let mut endpoint = reaper.track_index(track_guid);
        if let Some(handler) = &mut endpoint.handler {
            if let Some(index) = msg.args.get(0) {
                handler(TrackIndexArgs {
                    index: index.clone().int().unwrap(),
                });
            }
        }
        return;
    }
    if let Some(args) = match_addr(addr, "/track/{track_guid}/delete") {
        let track_guid = args[0].clone();
        let mut endpoint = reaper.track_delete(track_guid);
        if let Some(handler) = &mut endpoint.handler {}
        return;
    }
    if let Some(args) = match_addr(addr, "/track/{track_guid}/name") {
        let track_guid = args[0].clone();
        let mut endpoint = reaper.track_name(track_guid);
        if let Some(handler) = &mut endpoint.handler {
            if let Some(name) = msg.args.get(0) {
                handler(TrackNameArgs {
                    name: name.clone().string().unwrap().clone(),
                });
            }
        }
        return;
    }
    if let Some(args) = match_addr(addr, "/track/{track_guid}/selected") {
        let track_guid = args[0].clone();
        let mut endpoint = reaper.track_selected(track_guid);
        if let Some(handler) = &mut endpoint.handler {
            if let Some(selected) = msg.args.get(0) {
                handler(TrackSelectedArgs {
                    selected: selected.clone().bool().unwrap(),
                });
            }
        }
        return;
    }
    if let Some(args) = match_addr(addr, "/track/{track_guid}/volume") {
        let track_guid = args[0].clone();
        let mut endpoint = reaper.track_volume(track_guid);
        if let Some(handler) = &mut endpoint.handler {
            if let Some(volume) = msg.args.get(0) {
                handler(TrackVolumeArgs {
                    volume: volume.clone().float().unwrap(),
                });
            }
        }
        return;
    }
    if let Some(args) = match_addr(addr, "/track/{track_guid}/pan") {
        let track_guid = args[0].clone();
        let mut endpoint = reaper.track_pan(track_guid);
        if let Some(handler) = &mut endpoint.handler {
            if let Some(pan) = msg.args.get(0) {
                handler(TrackPanArgs {
                    pan: pan.clone().float().unwrap(),
                });
            }
        }
        return;
    }
    if let Some(args) = match_addr(addr, "/track/{track_guid}/mute") {
        let track_guid = args[0].clone();
        let mut endpoint = reaper.track_mute(track_guid);
        if let Some(handler) = &mut endpoint.handler {
            if let Some(mute) = msg.args.get(0) {
                handler(TrackMuteArgs {
                    mute: mute.clone().bool().unwrap(),
                });
            }
        }
        return;
    }
    if let Some(args) = match_addr(addr, "/track/{track_guid}/solo") {
        let track_guid = args[0].clone();
        let mut endpoint = reaper.track_solo(track_guid);
        if let Some(handler) = &mut endpoint.handler {
            if let Some(solo) = msg.args.get(0) {
                handler(TrackSoloArgs {
                    solo: solo.clone().bool().unwrap(),
                });
            }
        }
        return;
    }
    if let Some(args) = match_addr(addr, "/track/{track_guid}/rec-arm") {
        let track_guid = args[0].clone();
        let mut endpoint = reaper.track_rec_arm(track_guid);
        if let Some(handler) = &mut endpoint.handler {
            if let Some(rec_arm) = msg.args.get(0) {
                handler(TrackRecArmArgs {
                    rec_arm: rec_arm.clone().bool().unwrap(),
                });
            }
        }
        return;
    }
    if let Some(args) = match_addr(addr, "/track/{track_guid}/send/{send_index}/guid") {
        let send_index: i32 = args[0].parse().unwrap();
        let track_guid = args[1].clone();
        let mut endpoint = reaper.track_send_guid(track_guid, send_index);
        if let Some(handler) = &mut endpoint.handler {
            if let Some(guid) = msg.args.get(0) {
                handler(TrackSendGuidArgs {
                    guid: guid.clone().string().unwrap().clone(),
                });
            }
        }
        return;
    }
    if let Some(args) = match_addr(addr, "/track/{track_guid}/send/{send_index}/volume") {
        let send_index: i32 = args[0].parse().unwrap();
        let track_guid = args[1].clone();
        let mut endpoint = reaper.track_send_volume(track_guid, send_index);
        if let Some(handler) = &mut endpoint.handler {
            if let Some(volume) = msg.args.get(0) {
                handler(TrackSendVolumeArgs {
                    volume: volume.clone().float().unwrap(),
                });
            }
        }
        return;
    }
    if let Some(args) = match_addr(addr, "/track/{track_guid}/send/{send_index}/pan") {
        let send_index: i32 = args[0].parse().unwrap();
        let track_guid = args[1].clone();
        let mut endpoint = reaper.track_send_pan(track_guid, send_index);
        if let Some(handler) = &mut endpoint.handler {
            if let Some(pan) = msg.args.get(0) {
                handler(TrackSendPanArgs {
                    pan: pan.clone().float().unwrap(),
                });
            }
        }
        return;
    }
    if let Some(args) = match_addr(addr, "/track/{track_guid}/color") {
        let track_guid = args[0].clone();
        let mut endpoint = reaper.track_color(track_guid);
        if let Some(handler) = &mut endpoint.handler {
            if let Some(color) = msg.args.get(0) {
                handler(TrackColorArgs {
                    color: color.clone().int().unwrap(),
                });
            }
        }
        return;
    }
    if let Some(args) = match_addr(addr, "/track/{track_guid}/fx/{fx_idx}/guid") {
        let fx_idx: i32 = args[0].parse().unwrap();
        let track_guid = args[1].clone();
        let mut endpoint = reaper.track_fx_guid(track_guid, fx_idx);
        if let Some(handler) = &mut endpoint.handler {
            if let Some(guid) = msg.args.get(0) {
                handler(TrackFxGuidArgs {
                    guid: guid.clone().string().unwrap().clone(),
                });
            }
        }
        return;
    }
    if let Some(args) = match_addr(addr, "/track/{track_guid}/fx/{fx_idx}/name") {
        let fx_idx: i32 = args[0].parse().unwrap();
        let track_guid = args[1].clone();
        let mut endpoint = reaper.track_fx_name(track_guid, fx_idx);
        if let Some(handler) = &mut endpoint.handler {
            if let Some(name) = msg.args.get(0) {
                handler(TrackFxNameArgs {
                    name: name.clone().string().unwrap().clone(),
                });
            }
        }
        return;
    }
    if let Some(args) = match_addr(addr, "/track/{track_guid}/fx/{fx_idx}/enabled") {
        let fx_idx: i32 = args[0].parse().unwrap();
        let track_guid = args[1].clone();
        let mut endpoint = reaper.track_fx_enabled(track_guid, fx_idx);
        if let Some(handler) = &mut endpoint.handler {
            if let Some(enabled) = msg.args.get(0) {
                handler(TrackFxEnabledArgs {
                    enabled: enabled.clone().bool().unwrap(),
                });
            }
        }
        return;
    }
    if let Some(args) = match_addr(addr, "/track/{track_guid}/fx/{fx_idx}/param_count") {
        let fx_idx: i32 = args[0].parse().unwrap();
        let track_guid = args[1].clone();
        let mut endpoint = reaper.track_fx_param_count(track_guid, fx_idx);
        if let Some(handler) = &mut endpoint.handler {
            if let Some(param_count) = msg.args.get(0) {
                handler(TrackFxParamCountArgs {
                    param_count: param_count.clone().int().unwrap(),
                });
            }
        }
        return;
    }
    if let Some(args) = match_addr(
        addr,
        "/track/{track_guid}/fx/{fx_idx}/param/{param_idx}/name",
    ) {
        let param_idx: i32 = args[0].parse().unwrap();
        let fx_idx: i32 = args[1].parse().unwrap();
        let track_guid = args[2].clone();
        let mut endpoint = reaper.track_fx_param_name(track_guid, fx_idx, param_idx);
        if let Some(handler) = &mut endpoint.handler {
            if let Some(param_name) = msg.args.get(0) {
                handler(TrackFxParamNameArgs {
                    param_name: param_name.clone().string().unwrap().clone(),
                });
            }
        }
        return;
    }
    if let Some(args) = match_addr(
        addr,
        "/track/{track_guid}/fx/{fx_idx}/param/{param_idx}/value",
    ) {
        let param_idx: i32 = args[0].parse().unwrap();
        let fx_idx: i32 = args[1].parse().unwrap();
        let track_guid = args[2].clone();
        let mut endpoint = reaper.track_fx_param_value(track_guid, fx_idx, param_idx);
        if let Some(handler) = &mut endpoint.handler {
            if let Some(value) = msg.args.get(0) {
                handler(TrackFxParamValueArgs {
                    value: value.clone().float().unwrap(),
                });
            }
        }
        return;
    }
    if let Some(args) = match_addr(
        addr,
        "/track/{track_guid}/fx/{fx_idx}/param/{param_idx}/min",
    ) {
        let param_idx: i32 = args[0].parse().unwrap();
        let fx_idx: i32 = args[1].parse().unwrap();
        let track_guid = args[2].clone();
        let mut endpoint = reaper.track_fx_param_min(track_guid, fx_idx, param_idx);
        if let Some(handler) = &mut endpoint.handler {
            if let Some(min) = msg.args.get(0) {
                handler(TrackFxParamMinArgs {
                    min: min.clone().float().unwrap(),
                });
            }
        }
        return;
    }
    if let Some(args) = match_addr(
        addr,
        "/track/{track_guid}/fx/{fx_idx}/param/{param_idx}/max",
    ) {
        let param_idx: i32 = args[0].parse().unwrap();
        let fx_idx: i32 = args[1].parse().unwrap();
        let track_guid = args[2].clone();
        let mut endpoint = reaper.track_fx_param_max(track_guid, fx_idx, param_idx);
        if let Some(handler) = &mut endpoint.handler {
            if let Some(max) = msg.args.get(0) {
                handler(TrackFxParamMaxArgs {
                    max: max.clone().float().unwrap(),
                });
            }
        }
        return;
    }
    if let Some(args) = match_addr(addr, "/track/{track_guid}/fx/{fx_idx}/info") {
        let fx_idx: i32 = args[0].parse().unwrap();
        let track_guid = args[1].clone();
        let mut endpoint = reaper.track_fx_info(track_guid, fx_idx);
        if let Some(handler) = &mut endpoint.handler {}
        return;
    }
    if let Some(args) = match_addr(addr, "/fxinfo/{ident}/name") {
        let ident = args[0].clone();
        let mut endpoint = reaper.fxinfo_name(ident);
        if let Some(handler) = &mut endpoint.handler {
            if let Some(name) = msg.args.get(0) {
                handler(FxinfoNameArgs {
                    name: name.clone().string().unwrap().clone(),
                });
            }
        }
        return;
    }
    if let Some(args) = match_addr(addr, "/fxinfo/{ident}/param_count") {
        let ident = args[0].clone();
        let mut endpoint = reaper.fxinfo_param_count(ident);
        if let Some(handler) = &mut endpoint.handler {
            if let Some(param_count) = msg.args.get(0) {
                handler(FxinfoParamCountArgs {
                    param_count: param_count.clone().int().unwrap(),
                });
            }
        }
        return;
    }
    if let Some(args) = match_addr(addr, "/fxinfo/{ident}/param/{param_idx}/name") {
        let param_idx: i32 = args[0].parse().unwrap();
        let ident = args[1].clone();
        let mut endpoint = reaper.fxinfo_param_name(ident, param_idx);
        if let Some(handler) = &mut endpoint.handler {
            if let Some(param_name) = msg.args.get(0) {
                handler(FxinfoParamNameArgs {
                    param_name: param_name.clone().string().unwrap().clone(),
                });
            }
        }
        return;
    }
    if let Some(args) = match_addr(addr, "/fxinfo/{ident}/param/{param_idx}/min") {
        let param_idx: i32 = args[0].parse().unwrap();
        let ident = args[1].clone();
        let mut endpoint = reaper.fxinfo_param_min(ident, param_idx);
        if let Some(handler) = &mut endpoint.handler {
            if let Some(param_min) = msg.args.get(0) {
                handler(FxinfoParamMinArgs {
                    param_min: param_min.clone().float().unwrap(),
                });
            }
        }
        return;
    }
    if let Some(args) = match_addr(addr, "/fxinfo/{ident}/param/{param_idx}/max") {
        let param_idx: i32 = args[0].parse().unwrap();
        let ident = args[1].clone();
        let mut endpoint = reaper.fxinfo_param_max(ident, param_idx);
        if let Some(handler) = &mut endpoint.handler {
            if let Some(param_max) = msg.args.get(0) {
                handler(FxinfoParamMaxArgs {
                    param_max: param_max.clone().float().unwrap(),
                });
            }
        }
        return;
    }
    if let Some(args) = match_addr(addr, "/fxinfo") {
        let mut endpoint = reaper.fxinfo();
        if let Some(handler) = &mut endpoint.handler {}
        return;
    }
    log_unknown(addr);
}
