// AUTO-GENERATED CODE. DO NOT EDIT!

use std::collections::HashMap;
use std::net::UdpSocket;
use std::sync::Arc;

use uuid::Uuid;

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
        let osc_address = format!("/num_tracks?");
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
        let osc_address = format!("/track/all_guids?");
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
    pub track_guid: Uuid,
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
        let osc_address = format!("/track/{}/index?", self.track_guid);
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
    pub track_guid: Uuid,
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

/// /track/{track_guid}/delete
impl Bind<TrackDeleteArgs> for TrackDelete {
    fn bind<F>(&mut self, callback: F)
    where
        F: FnMut(TrackDeleteArgs) + 'static,
    {
        self.handler = Some(Box::new(callback));
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
    pub track_guid: Uuid,
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
        let osc_address = format!("/track/{}/name?", self.track_guid);
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
    pub track_guid: Uuid,
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
        let osc_address = format!("/track/{}/selected?", self.track_guid);
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
    pub track_guid: Uuid,
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
        let osc_address = format!("/track/{}/volume?", self.track_guid);
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
    pub track_guid: Uuid,
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
        let osc_address = format!("/track/{}/pan?", self.track_guid);
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
    pub track_guid: Uuid,
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
        let osc_address = format!("/track/{}/mute?", self.track_guid);
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
    pub track_guid: Uuid,
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
        let osc_address = format!("/track/{}/solo?", self.track_guid);
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
    pub track_guid: Uuid,
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
        let osc_address = format!("/track/{}/rec-arm?", self.track_guid);
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
    pub guid: Uuid, // unique identifier for the send
}

pub type TrackSendGuidHandler = Box<dyn FnMut(TrackSendGuidArgs) + 'static>;

pub struct TrackSendGuid {
    socket: Arc<UdpSocket>,
    handler: Option<TrackSendGuidHandler>,
    pub track_guid: Uuid,
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
        let osc_address = format!("/track/{}/send/{}/guid?", self.track_guid, self.send_index);
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
    pub track_guid: Uuid,
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
        let osc_address = format!(
            "/track/{}/send/{}/volume?",
            self.track_guid, self.send_index
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
pub struct TrackSendPanArgs {
    pub pan: f32, // pan of the send, normalized to -1.0 to 1.0
}

pub type TrackSendPanHandler = Box<dyn FnMut(TrackSendPanArgs) + 'static>;

pub struct TrackSendPan {
    socket: Arc<UdpSocket>,
    handler: Option<TrackSendPanHandler>,
    pub track_guid: Uuid,
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
        let osc_address = format!("/track/{}/send/{}/pan?", self.track_guid, self.send_index);
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
    pub r: i32, // encoded from u8
    pub g: i32, // encoded from u8
    pub b: i32, // encoded from u8
}

pub type TrackColorHandler = Box<dyn FnMut(TrackColorArgs) + 'static>;

pub struct TrackColor {
    socket: Arc<UdpSocket>,
    handler: Option<TrackColorHandler>,
    pub track_guid: Uuid,
}

/// /track/{track_guid}/color
impl Set<TrackColorArgs> for TrackColor {
    type Error = OscError;
    fn set(&mut self, args: TrackColorArgs) -> Result<(), Self::Error> {
        let osc_address = format!("/track/{}/color", self.track_guid);
        let osc_msg = rosc::OscMessage {
            addr: osc_address,
            args: vec![
                rosc::OscType::Int(args.r),
                rosc::OscType::Int(args.g),
                rosc::OscType::Int(args.b),
            ],
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
        let osc_address = format!("/track/{}/color?", self.track_guid);
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
    pub guid: Uuid, // unique identifier for the FX
}

pub type TrackFxGuidHandler = Box<dyn FnMut(TrackFxGuidArgs) + 'static>;

pub struct TrackFxGuid {
    socket: Arc<UdpSocket>,
    handler: Option<TrackFxGuidHandler>,
    pub track_guid: Uuid,
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
        let osc_address = format!("/track/{}/fx/{}/guid?", self.track_guid, self.fx_idx);
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
    pub track_guid: Uuid,
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
        let osc_address = format!("/track/{}/fx/{}/name?", self.track_guid, self.fx_idx);
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
    pub track_guid: Uuid,
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
        let osc_address = format!("/track/{}/fx/{}/enabled?", self.track_guid, self.fx_idx);
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
    pub track_guid: Uuid,
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
        let osc_address = format!("/track/{}/fx/{}/param_count?", self.track_guid, self.fx_idx);
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
    pub track_guid: Uuid,
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
            "/track/{}/fx/{}/param/{}/name?",
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
    pub track_guid: Uuid,
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
            "/track/{}/fx/{}/param/{}/value?",
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
    pub track_guid: Uuid,
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
            "/track/{}/fx/{}/param/{}/min?",
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
    pub track_guid: Uuid,
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
            "/track/{}/fx/{}/param/{}/max?",
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
    pub track_guid: Uuid,
    pub fx_idx: i32,
}

/// /track/{track_guid}/fx/{fx_idx}/info
impl Query for TrackFxInfo {
    type Error = OscError;
    fn query(&self) -> Result<(), Self::Error> {
        let osc_address = format!("/track/{}/fx/{}/info?", self.track_guid, self.fx_idx);
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
        let osc_address = format!("/fxinfo/{}/param_count?", self.ident);
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
        let osc_address = format!("/fxinfo/{}/param/{}/name?", self.ident, self.param_idx);
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
        let osc_address = format!("/fxinfo/{}/param/{}/min?", self.ident, self.param_idx);
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
        let osc_address = format!("/fxinfo/{}/param/{}/max?", self.ident, self.param_idx);
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
        let osc_address = format!("/fxinfo?");
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
    use uuid::Uuid;

    use crate::osc::route_context::ContextTrait;

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
        pub track_guid: Uuid,
    }

    impl ContextTrait for Track {}

    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    pub struct TrackFx {
        pub track_guid: Uuid,
        pub fx_idx: i32,
    }

    impl ContextTrait for TrackFx {}

    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    pub struct TrackFxParam {
        pub track_guid: Uuid,
        pub fx_idx: i32,
        pub param_idx: i32,
    }

    impl ContextTrait for TrackFxParam {}

    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    pub struct TrackSend {
        pub track_guid: Uuid,
        pub send_index: i32,
    }

    impl ContextTrait for TrackSend {}
}

pub mod context_kind {
    use super::context;
    use crate::osc::route_context::ContextKindTrait;
    use regex::Regex;
    use uuid::Uuid;

    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    pub struct Fxinfo {}

    impl ContextKindTrait for Fxinfo {
        type Context = context::Fxinfo;

        fn context_name() -> &'static str {
            "Fxinfo"
        }

        fn parse(osc_address: &str) -> Option<context::Fxinfo> {
            let re = Regex::new(r"^/fxinfo/([^/]+)/name$").unwrap();
            re.captures(osc_address).and_then(|caps| {
                Some(context::Fxinfo {
                    ident: caps[1].to_string(),
                })
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
            re.captures(osc_address).and_then(|caps| {
                Some(context::FxinfoParam {
                    ident: caps[1].to_string(),
                    param_idx: caps[2].parse().ok()?,
                })
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
            re.captures(osc_address).and_then(|caps| {
                Some(context::Track {
                    track_guid: Uuid::parse_str(&caps[1]).ok()?,
                })
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
            re.captures(osc_address).and_then(|caps| {
                Some(context::TrackFx {
                    track_guid: Uuid::parse_str(&caps[1]).ok()?,
                    fx_idx: caps[2].parse().ok()?,
                })
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
            re.captures(osc_address).and_then(|caps| {
                Some(context::TrackFxParam {
                    track_guid: Uuid::parse_str(&caps[1]).ok()?,
                    fx_idx: caps[2].parse().ok()?,
                    param_idx: caps[3].parse().ok()?,
                })
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
            re.captures(osc_address).and_then(|caps| {
                Some(context::TrackSend {
                    track_guid: Uuid::parse_str(&caps[1]).ok()?,
                    send_index: caps[2].parse().ok()?,
                })
            })
        }
    }
}

pub struct Reaper {
    socket: Arc<UdpSocket>,
    num_tracks_endpoint: NumTracks,
    track_all_guids_endpoint: TrackAllGuids,
    track_index_endpoints: HashMap<Uuid, TrackIndex>,
    track_delete_endpoints: HashMap<Uuid, TrackDelete>,
    track_name_endpoints: HashMap<Uuid, TrackName>,
    track_selected_endpoints: HashMap<Uuid, TrackSelected>,
    track_volume_endpoints: HashMap<Uuid, TrackVolume>,
    track_pan_endpoints: HashMap<Uuid, TrackPan>,
    track_mute_endpoints: HashMap<Uuid, TrackMute>,
    track_solo_endpoints: HashMap<Uuid, TrackSolo>,
    track_rec_arm_endpoints: HashMap<Uuid, TrackRecArm>,
    track_send_guid_endpoints: HashMap<Uuid, HashMap<i32, TrackSendGuid>>,
    track_send_volume_endpoints: HashMap<Uuid, HashMap<i32, TrackSendVolume>>,
    track_send_pan_endpoints: HashMap<Uuid, HashMap<i32, TrackSendPan>>,
    track_color_endpoints: HashMap<Uuid, TrackColor>,
    track_fx_guid_endpoints: HashMap<Uuid, HashMap<i32, TrackFxGuid>>,
    track_fx_name_endpoints: HashMap<Uuid, HashMap<i32, TrackFxName>>,
    track_fx_enabled_endpoints: HashMap<Uuid, HashMap<i32, TrackFxEnabled>>,
    track_fx_param_count_endpoints: HashMap<Uuid, HashMap<i32, TrackFxParamCount>>,
    track_fx_param_name_endpoints: HashMap<Uuid, HashMap<i32, HashMap<i32, TrackFxParamName>>>,
    track_fx_param_value_endpoints: HashMap<Uuid, HashMap<i32, HashMap<i32, TrackFxParamValue>>>,
    track_fx_param_min_endpoints: HashMap<Uuid, HashMap<i32, HashMap<i32, TrackFxParamMin>>>,
    track_fx_param_max_endpoints: HashMap<Uuid, HashMap<i32, HashMap<i32, TrackFxParamMax>>>,
    track_fx_info_endpoints: HashMap<Uuid, HashMap<i32, TrackFxInfo>>,
    fxinfo_name_endpoints: HashMap<String, FxinfoName>,
    fxinfo_param_count_endpoints: HashMap<String, FxinfoParamCount>,
    fxinfo_param_name_endpoints: HashMap<String, HashMap<i32, FxinfoParamName>>,
    fxinfo_param_min_endpoints: HashMap<String, HashMap<i32, FxinfoParamMin>>,
    fxinfo_param_max_endpoints: HashMap<String, HashMap<i32, FxinfoParamMax>>,
    fxinfo_endpoint: Fxinfo,
}

impl Reaper {
    pub fn new(socket: Arc<UdpSocket>) -> Self {
        Self {
            socket: socket.clone(),
            num_tracks_endpoint: NumTracks {
                socket: socket.clone(),
                handler: None,
            },
            track_all_guids_endpoint: TrackAllGuids {
                socket: socket.clone(),
                handler: None,
            },
            track_index_endpoints: HashMap::new(),
            track_delete_endpoints: HashMap::new(),
            track_name_endpoints: HashMap::new(),
            track_selected_endpoints: HashMap::new(),
            track_volume_endpoints: HashMap::new(),
            track_pan_endpoints: HashMap::new(),
            track_mute_endpoints: HashMap::new(),
            track_solo_endpoints: HashMap::new(),
            track_rec_arm_endpoints: HashMap::new(),
            track_send_guid_endpoints: HashMap::new(),
            track_send_volume_endpoints: HashMap::new(),
            track_send_pan_endpoints: HashMap::new(),
            track_color_endpoints: HashMap::new(),
            track_fx_guid_endpoints: HashMap::new(),
            track_fx_name_endpoints: HashMap::new(),
            track_fx_enabled_endpoints: HashMap::new(),
            track_fx_param_count_endpoints: HashMap::new(),
            track_fx_param_name_endpoints: HashMap::new(),
            track_fx_param_value_endpoints: HashMap::new(),
            track_fx_param_min_endpoints: HashMap::new(),
            track_fx_param_max_endpoints: HashMap::new(),
            track_fx_info_endpoints: HashMap::new(),
            fxinfo_name_endpoints: HashMap::new(),
            fxinfo_param_count_endpoints: HashMap::new(),
            fxinfo_param_name_endpoints: HashMap::new(),
            fxinfo_param_min_endpoints: HashMap::new(),
            fxinfo_param_max_endpoints: HashMap::new(),
            fxinfo_endpoint: Fxinfo {
                socket: socket.clone(),
                handler: None,
            },
        }
    }
}

impl Reaper {
    pub fn num_tracks(&mut self) -> &mut NumTracks {
        &mut self.num_tracks_endpoint
    }
    pub fn track_all_guids(&mut self) -> &mut TrackAllGuids {
        &mut self.track_all_guids_endpoint
    }
    pub fn track_index(&mut self, track_guid: Uuid) -> &mut TrackIndex {
        self.track_index_endpoints
            .entry(track_guid.clone())
            .or_insert_with(|| TrackIndex {
                socket: self.socket.clone(),
                track_guid: track_guid,
                handler: None,
            })
    }
    pub fn track_delete(&mut self, track_guid: Uuid) -> &mut TrackDelete {
        self.track_delete_endpoints
            .entry(track_guid.clone())
            .or_insert_with(|| TrackDelete {
                socket: self.socket.clone(),
                track_guid: track_guid,
                handler: None,
            })
    }
    pub fn track_name(&mut self, track_guid: Uuid) -> &mut TrackName {
        self.track_name_endpoints
            .entry(track_guid.clone())
            .or_insert_with(|| TrackName {
                socket: self.socket.clone(),
                track_guid: track_guid,
                handler: None,
            })
    }
    pub fn track_selected(&mut self, track_guid: Uuid) -> &mut TrackSelected {
        self.track_selected_endpoints
            .entry(track_guid.clone())
            .or_insert_with(|| TrackSelected {
                socket: self.socket.clone(),
                track_guid: track_guid,
                handler: None,
            })
    }
    pub fn track_volume(&mut self, track_guid: Uuid) -> &mut TrackVolume {
        self.track_volume_endpoints
            .entry(track_guid.clone())
            .or_insert_with(|| TrackVolume {
                socket: self.socket.clone(),
                track_guid: track_guid,
                handler: None,
            })
    }
    pub fn track_pan(&mut self, track_guid: Uuid) -> &mut TrackPan {
        self.track_pan_endpoints
            .entry(track_guid.clone())
            .or_insert_with(|| TrackPan {
                socket: self.socket.clone(),
                track_guid: track_guid,
                handler: None,
            })
    }
    pub fn track_mute(&mut self, track_guid: Uuid) -> &mut TrackMute {
        self.track_mute_endpoints
            .entry(track_guid.clone())
            .or_insert_with(|| TrackMute {
                socket: self.socket.clone(),
                track_guid: track_guid,
                handler: None,
            })
    }
    pub fn track_solo(&mut self, track_guid: Uuid) -> &mut TrackSolo {
        self.track_solo_endpoints
            .entry(track_guid.clone())
            .or_insert_with(|| TrackSolo {
                socket: self.socket.clone(),
                track_guid: track_guid,
                handler: None,
            })
    }
    pub fn track_rec_arm(&mut self, track_guid: Uuid) -> &mut TrackRecArm {
        self.track_rec_arm_endpoints
            .entry(track_guid.clone())
            .or_insert_with(|| TrackRecArm {
                socket: self.socket.clone(),
                track_guid: track_guid,
                handler: None,
            })
    }
    pub fn track_send_guid(&mut self, track_guid: Uuid, send_index: i32) -> &mut TrackSendGuid {
        self.track_send_guid_endpoints
            .entry(track_guid.clone())
            .or_insert_with(|| HashMap::new())
            .entry(send_index.clone())
            .or_insert_with(|| TrackSendGuid {
                socket: self.socket.clone(),
                track_guid: track_guid,
                send_index: send_index,
                handler: None,
            })
    }
    pub fn track_send_volume(&mut self, track_guid: Uuid, send_index: i32) -> &mut TrackSendVolume {
        self.track_send_volume_endpoints
            .entry(track_guid.clone())
            .or_insert_with(|| HashMap::new())
            .entry(send_index.clone())
            .or_insert_with(|| TrackSendVolume {
                socket: self.socket.clone(),
                track_guid: track_guid,
                send_index: send_index,
                handler: None,
            })
    }
    pub fn track_send_pan(&mut self, track_guid: Uuid, send_index: i32) -> &mut TrackSendPan {
        self.track_send_pan_endpoints
            .entry(track_guid.clone())
            .or_insert_with(|| HashMap::new())
            .entry(send_index.clone())
            .or_insert_with(|| TrackSendPan {
                socket: self.socket.clone(),
                track_guid: track_guid,
                send_index: send_index,
                handler: None,
            })
    }
    pub fn track_color(&mut self, track_guid: Uuid) -> &mut TrackColor {
        self.track_color_endpoints
            .entry(track_guid.clone())
            .or_insert_with(|| TrackColor {
                socket: self.socket.clone(),
                track_guid: track_guid,
                handler: None,
            })
    }
    pub fn track_fx_guid(&mut self, track_guid: Uuid, fx_idx: i32) -> &mut TrackFxGuid {
        self.track_fx_guid_endpoints
            .entry(track_guid.clone())
            .or_insert_with(|| HashMap::new())
            .entry(fx_idx.clone())
            .or_insert_with(|| TrackFxGuid {
                socket: self.socket.clone(),
                track_guid: track_guid,
                fx_idx: fx_idx,
                handler: None,
            })
    }
    pub fn track_fx_name(&mut self, track_guid: Uuid, fx_idx: i32) -> &mut TrackFxName {
        self.track_fx_name_endpoints
            .entry(track_guid.clone())
            .or_insert_with(|| HashMap::new())
            .entry(fx_idx.clone())
            .or_insert_with(|| TrackFxName {
                socket: self.socket.clone(),
                track_guid: track_guid,
                fx_idx: fx_idx,
                handler: None,
            })
    }
    pub fn track_fx_enabled(&mut self, track_guid: Uuid, fx_idx: i32) -> &mut TrackFxEnabled {
        self.track_fx_enabled_endpoints
            .entry(track_guid.clone())
            .or_insert_with(|| HashMap::new())
            .entry(fx_idx.clone())
            .or_insert_with(|| TrackFxEnabled {
                socket: self.socket.clone(),
                track_guid: track_guid,
                fx_idx: fx_idx,
                handler: None,
            })
    }
    pub fn track_fx_param_count(
        &mut self,
        track_guid: Uuid,
        fx_idx: i32,
    ) -> &mut TrackFxParamCount {
        self.track_fx_param_count_endpoints
            .entry(track_guid.clone())
            .or_insert_with(|| HashMap::new())
            .entry(fx_idx.clone())
            .or_insert_with(|| TrackFxParamCount {
                socket: self.socket.clone(),
                track_guid: track_guid,
                fx_idx: fx_idx,
                handler: None,
            })
    }
    pub fn track_fx_param_name(
        &mut self,
        track_guid: Uuid,
        fx_idx: i32,
        param_idx: i32,
    ) -> &mut TrackFxParamName {
        self.track_fx_param_name_endpoints
            .entry(track_guid.clone())
            .or_insert_with(|| HashMap::new())
            .entry(fx_idx.clone())
            .or_insert_with(|| HashMap::new())
            .entry(param_idx.clone())
            .or_insert_with(|| TrackFxParamName {
                socket: self.socket.clone(),
                track_guid: track_guid,
                fx_idx: fx_idx,
                param_idx: param_idx,
                handler: None,
            })
    }
    pub fn track_fx_param_value(
        &mut self,
        track_guid: Uuid,
        fx_idx: i32,
        param_idx: i32,
    ) -> &mut TrackFxParamValue {
        self.track_fx_param_value_endpoints
            .entry(track_guid.clone())
            .or_insert_with(|| HashMap::new())
            .entry(fx_idx.clone())
            .or_insert_with(|| HashMap::new())
            .entry(param_idx.clone())
            .or_insert_with(|| TrackFxParamValue {
                socket: self.socket.clone(),
                track_guid: track_guid,
                fx_idx: fx_idx,
                param_idx: param_idx,
                handler: None,
            })
    }
    pub fn track_fx_param_min(
        &mut self,
        track_guid: Uuid,
        fx_idx: i32,
        param_idx: i32,
    ) -> &mut TrackFxParamMin {
        self.track_fx_param_min_endpoints
            .entry(track_guid.clone())
            .or_insert_with(|| HashMap::new())
            .entry(fx_idx.clone())
            .or_insert_with(|| HashMap::new())
            .entry(param_idx.clone())
            .or_insert_with(|| TrackFxParamMin {
                socket: self.socket.clone(),
                track_guid: track_guid,
                fx_idx: fx_idx,
                param_idx: param_idx,
                handler: None,
            })
    }
    pub fn track_fx_param_max(
        &mut self,
        track_guid: Uuid,
        fx_idx: i32,
        param_idx: i32,
    ) -> &mut TrackFxParamMax {
        self.track_fx_param_max_endpoints
            .entry(track_guid.clone())
            .or_insert_with(|| HashMap::new())
            .entry(fx_idx.clone())
            .or_insert_with(|| HashMap::new())
            .entry(param_idx.clone())
            .or_insert_with(|| TrackFxParamMax {
                socket: self.socket.clone(),
                track_guid: track_guid,
                fx_idx: fx_idx,
                param_idx: param_idx,
                handler: None,
            })
    }
    pub fn track_fx_info(&mut self, track_guid: Uuid, fx_idx: i32) -> &mut TrackFxInfo {
        self.track_fx_info_endpoints
            .entry(track_guid.clone())
            .or_insert_with(|| HashMap::new())
            .entry(fx_idx.clone())
            .or_insert_with(|| TrackFxInfo {
                socket: self.socket.clone(),
                track_guid: track_guid,
                fx_idx: fx_idx,
                handler: None,
            })
    }
    pub fn fxinfo_name(&mut self, ident: String) -> &mut FxinfoName {
        self.fxinfo_name_endpoints
            .entry(ident.clone())
            .or_insert_with(|| FxinfoName {
                socket: self.socket.clone(),
                ident: ident,
                handler: None,
            })
    }
    pub fn fxinfo_param_count(&mut self, ident: String) -> &mut FxinfoParamCount {
        self.fxinfo_param_count_endpoints
            .entry(ident.clone())
            .or_insert_with(|| FxinfoParamCount {
                socket: self.socket.clone(),
                ident: ident,
                handler: None,
            })
    }
    pub fn fxinfo_param_name(&mut self, ident: String, param_idx: i32) -> &mut FxinfoParamName {
        self.fxinfo_param_name_endpoints
            .entry(ident.clone())
            .or_insert_with(|| HashMap::new())
            .entry(param_idx.clone())
            .or_insert_with(|| FxinfoParamName {
                socket: self.socket.clone(),
                ident: ident,
                param_idx: param_idx,
                handler: None,
            })
    }
    pub fn fxinfo_param_min(&mut self, ident: String, param_idx: i32) -> &mut FxinfoParamMin {
        self.fxinfo_param_min_endpoints
            .entry(ident.clone())
            .or_insert_with(|| HashMap::new())
            .entry(param_idx.clone())
            .or_insert_with(|| FxinfoParamMin {
                socket: self.socket.clone(),
                ident: ident,
                param_idx: param_idx,
                handler: None,
            })
    }
    pub fn fxinfo_param_max(&mut self, ident: String, param_idx: i32) -> &mut FxinfoParamMax {
        self.fxinfo_param_max_endpoints
            .entry(ident.clone())
            .or_insert_with(|| HashMap::new())
            .entry(param_idx.clone())
            .or_insert_with(|| FxinfoParamMax {
                socket: self.socket.clone(),
                ident: ident,
                param_idx: param_idx,
                handler: None,
            })
    }
    pub fn fxinfo(&mut self) -> &mut Fxinfo {
        &mut self.fxinfo_endpoint
    }
}

#[derive(Debug)]
pub enum DispatchError {
    MissingArgument {
        arg_index: usize,
    },
    WrongArgumentType {
        expected: &'static str,
        got: &'static str,
    },
    ParamParseError {
        param: &'static str,
        value: String,
    },
}

fn osc_type_name(t: &rosc::OscType) -> &'static str {
    match t {
        rosc::OscType::Int(_) => "int",
        rosc::OscType::Float(_) => "float",
        rosc::OscType::String(_) => "string",
        rosc::OscType::Bool(_) => "bool",
        rosc::OscType::Double(_) => "double",
        rosc::OscType::Long(_) => "long",
        _ => "unknown",
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
        if p.starts_with('{') && p.ends_with('}') {
            args.push((*a).to_string());
        } else if *p != *a {
            return None;
        }
    }
    Some(args)
}

pub fn dispatch_osc<F, G>(
    reaper: &mut Reaper,
    msg: rosc::OscMessage,
    mut log_unknown_route: F,
    mut log_decode_error: G,
) where
    F: FnMut(&str),
    G: FnMut(&str, DispatchError),
{
    let addr = msg.addr.as_str();
    if let Some(_args) = match_addr(addr, "/num_tracks") {
        let endpoint = reaper.num_tracks();
        if let Some(handler) = &mut endpoint.handler {
            let _decoded_num_tracks = match msg.args.get(0) {
                Some(_raw_arg_0) => match _raw_arg_0.clone().int() {
                    Some(v) => v,
                    None => {
                        log_decode_error(
                            addr,
                            DispatchError::WrongArgumentType {
                                expected: "int",
                                got: osc_type_name(_raw_arg_0),
                            },
                        );
                        return;
                    }
                },
                None => {
                    log_decode_error(addr, DispatchError::MissingArgument { arg_index: 0 });
                    return;
                }
            };
            handler(NumTracksArgs {
                num_tracks: _decoded_num_tracks,
            });
        }
        return;
    }
    if let Some(_args) = match_addr(addr, "/track/all_guids") {
        let endpoint = reaper.track_all_guids();
        if let Some(handler) = &mut endpoint.handler {
            handler(TrackAllGuidsArgs {});
        }
        return;
    }
    if let Some(args) = match_addr(addr, "/track/{track_guid}/index") {
        let track_guid: Uuid = match Uuid::parse_str(&args[0]) {
            Ok(v) => v,
            Err(_) => {
                log_decode_error(
                    addr,
                    DispatchError::ParamParseError {
                        param: "track_guid",
                        value: args[0].clone(),
                    },
                );
                return;
            }
        };
        let endpoint = reaper.track_index(track_guid);
        if let Some(handler) = &mut endpoint.handler {
            let _decoded_index = match msg.args.get(0) {
                Some(_raw_arg_0) => match _raw_arg_0.clone().int() {
                    Some(v) => v,
                    None => {
                        log_decode_error(
                            addr,
                            DispatchError::WrongArgumentType {
                                expected: "int",
                                got: osc_type_name(_raw_arg_0),
                            },
                        );
                        return;
                    }
                },
                None => {
                    log_decode_error(addr, DispatchError::MissingArgument { arg_index: 0 });
                    return;
                }
            };
            handler(TrackIndexArgs {
                index: _decoded_index,
            });
        }
        return;
    }
    if let Some(args) = match_addr(addr, "/track/{track_guid}/delete") {
        let track_guid: Uuid = match Uuid::parse_str(&args[0]) {
            Ok(v) => v,
            Err(_) => {
                log_decode_error(
                    addr,
                    DispatchError::ParamParseError {
                        param: "track_guid",
                        value: args[0].clone(),
                    },
                );
                return;
            }
        };
        let endpoint = reaper.track_delete(track_guid);
        if let Some(handler) = &mut endpoint.handler {
            handler(TrackDeleteArgs {});
        }
        return;
    }
    if let Some(args) = match_addr(addr, "/track/{track_guid}/name") {
        let track_guid: Uuid = match Uuid::parse_str(&args[0]) {
            Ok(v) => v,
            Err(_) => {
                log_decode_error(
                    addr,
                    DispatchError::ParamParseError {
                        param: "track_guid",
                        value: args[0].clone(),
                    },
                );
                return;
            }
        };
        let endpoint = reaper.track_name(track_guid);
        if let Some(handler) = &mut endpoint.handler {
            let _decoded_name = match msg.args.get(0) {
                Some(_raw_arg_0) => match _raw_arg_0.clone().string() {
                    Some(v) => v,
                    None => {
                        log_decode_error(
                            addr,
                            DispatchError::WrongArgumentType {
                                expected: "string",
                                got: osc_type_name(_raw_arg_0),
                            },
                        );
                        return;
                    }
                },
                None => {
                    log_decode_error(addr, DispatchError::MissingArgument { arg_index: 0 });
                    return;
                }
            };
            handler(TrackNameArgs {
                name: _decoded_name,
            });
        }
        return;
    }
    if let Some(args) = match_addr(addr, "/track/{track_guid}/selected") {
        let track_guid: Uuid = match Uuid::parse_str(&args[0]) {
            Ok(v) => v,
            Err(_) => {
                log_decode_error(
                    addr,
                    DispatchError::ParamParseError {
                        param: "track_guid",
                        value: args[0].clone(),
                    },
                );
                return;
            }
        };
        let endpoint = reaper.track_selected(track_guid);
        if let Some(handler) = &mut endpoint.handler {
            let _decoded_selected = match msg.args.get(0) {
                Some(_raw_arg_0) => match _raw_arg_0.clone().bool() {
                    Some(v) => v,
                    None => {
                        log_decode_error(
                            addr,
                            DispatchError::WrongArgumentType {
                                expected: "bool",
                                got: osc_type_name(_raw_arg_0),
                            },
                        );
                        return;
                    }
                },
                None => {
                    log_decode_error(addr, DispatchError::MissingArgument { arg_index: 0 });
                    return;
                }
            };
            handler(TrackSelectedArgs {
                selected: _decoded_selected,
            });
        }
        return;
    }
    if let Some(args) = match_addr(addr, "/track/{track_guid}/volume") {
        let track_guid: Uuid = match Uuid::parse_str(&args[0]) {
            Ok(v) => v,
            Err(_) => {
                log_decode_error(
                    addr,
                    DispatchError::ParamParseError {
                        param: "track_guid",
                        value: args[0].clone(),
                    },
                );
                return;
            }
        };
        let endpoint = reaper.track_volume(track_guid);
        if let Some(handler) = &mut endpoint.handler {
            let _decoded_volume = match msg.args.get(0) {
                Some(_raw_arg_0) => match _raw_arg_0.clone().float() {
                    Some(v) => v,
                    None => {
                        log_decode_error(
                            addr,
                            DispatchError::WrongArgumentType {
                                expected: "float",
                                got: osc_type_name(_raw_arg_0),
                            },
                        );
                        return;
                    }
                },
                None => {
                    log_decode_error(addr, DispatchError::MissingArgument { arg_index: 0 });
                    return;
                }
            };
            handler(TrackVolumeArgs {
                volume: _decoded_volume,
            });
        }
        return;
    }
    if let Some(args) = match_addr(addr, "/track/{track_guid}/pan") {
        let track_guid: Uuid = match Uuid::parse_str(&args[0]) {
            Ok(v) => v,
            Err(_) => {
                log_decode_error(
                    addr,
                    DispatchError::ParamParseError {
                        param: "track_guid",
                        value: args[0].clone(),
                    },
                );
                return;
            }
        };
        let endpoint = reaper.track_pan(track_guid);
        if let Some(handler) = &mut endpoint.handler {
            let _decoded_pan = match msg.args.get(0) {
                Some(_raw_arg_0) => match _raw_arg_0.clone().float() {
                    Some(v) => v,
                    None => {
                        log_decode_error(
                            addr,
                            DispatchError::WrongArgumentType {
                                expected: "float",
                                got: osc_type_name(_raw_arg_0),
                            },
                        );
                        return;
                    }
                },
                None => {
                    log_decode_error(addr, DispatchError::MissingArgument { arg_index: 0 });
                    return;
                }
            };
            handler(TrackPanArgs { pan: _decoded_pan });
        }
        return;
    }
    if let Some(args) = match_addr(addr, "/track/{track_guid}/mute") {
        let track_guid: Uuid = match Uuid::parse_str(&args[0]) {
            Ok(v) => v,
            Err(_) => {
                log_decode_error(
                    addr,
                    DispatchError::ParamParseError {
                        param: "track_guid",
                        value: args[0].clone(),
                    },
                );
                return;
            }
        };
        let endpoint = reaper.track_mute(track_guid);
        if let Some(handler) = &mut endpoint.handler {
            let _decoded_mute = match msg.args.get(0) {
                Some(_raw_arg_0) => match _raw_arg_0.clone().bool() {
                    Some(v) => v,
                    None => {
                        log_decode_error(
                            addr,
                            DispatchError::WrongArgumentType {
                                expected: "bool",
                                got: osc_type_name(_raw_arg_0),
                            },
                        );
                        return;
                    }
                },
                None => {
                    log_decode_error(addr, DispatchError::MissingArgument { arg_index: 0 });
                    return;
                }
            };
            handler(TrackMuteArgs {
                mute: _decoded_mute,
            });
        }
        return;
    }
    if let Some(args) = match_addr(addr, "/track/{track_guid}/solo") {
        let track_guid: Uuid = match Uuid::parse_str(&args[0]) {
            Ok(v) => v,
            Err(_) => {
                log_decode_error(
                    addr,
                    DispatchError::ParamParseError {
                        param: "track_guid",
                        value: args[0].clone(),
                    },
                );
                return;
            }
        };
        let endpoint = reaper.track_solo(track_guid);
        if let Some(handler) = &mut endpoint.handler {
            let _decoded_solo = match msg.args.get(0) {
                Some(_raw_arg_0) => match _raw_arg_0.clone().bool() {
                    Some(v) => v,
                    None => {
                        log_decode_error(
                            addr,
                            DispatchError::WrongArgumentType {
                                expected: "bool",
                                got: osc_type_name(_raw_arg_0),
                            },
                        );
                        return;
                    }
                },
                None => {
                    log_decode_error(addr, DispatchError::MissingArgument { arg_index: 0 });
                    return;
                }
            };
            handler(TrackSoloArgs {
                solo: _decoded_solo,
            });
        }
        return;
    }
    if let Some(args) = match_addr(addr, "/track/{track_guid}/rec-arm") {
        let track_guid: Uuid = match Uuid::parse_str(&args[0]) {
            Ok(v) => v,
            Err(_) => {
                log_decode_error(
                    addr,
                    DispatchError::ParamParseError {
                        param: "track_guid",
                        value: args[0].clone(),
                    },
                );
                return;
            }
        };
        let endpoint = reaper.track_rec_arm(track_guid);
        if let Some(handler) = &mut endpoint.handler {
            let _decoded_rec_arm = match msg.args.get(0) {
                Some(_raw_arg_0) => match _raw_arg_0.clone().bool() {
                    Some(v) => v,
                    None => {
                        log_decode_error(
                            addr,
                            DispatchError::WrongArgumentType {
                                expected: "bool",
                                got: osc_type_name(_raw_arg_0),
                            },
                        );
                        return;
                    }
                },
                None => {
                    log_decode_error(addr, DispatchError::MissingArgument { arg_index: 0 });
                    return;
                }
            };
            handler(TrackRecArmArgs {
                rec_arm: _decoded_rec_arm,
            });
        }
        return;
    }
    if let Some(args) = match_addr(addr, "/track/{track_guid}/send/{send_index}/guid") {
        let track_guid: Uuid = match Uuid::parse_str(&args[0]) {
            Ok(v) => v,
            Err(_) => {
                log_decode_error(
                    addr,
                    DispatchError::ParamParseError {
                        param: "track_guid",
                        value: args[0].clone(),
                    },
                );
                return;
            }
        };
        let send_index: i32 = match args[1].parse::<i32>() {
            Ok(v) => v,
            Err(_) => {
                log_decode_error(
                    addr,
                    DispatchError::ParamParseError {
                        param: "send_index",
                        value: args[1].clone(),
                    },
                );
                return;
            }
        };
        let endpoint = reaper.track_send_guid(track_guid, send_index);
        if let Some(handler) = &mut endpoint.handler {
            let _decoded_guid = match msg.args.get(0) {
                Some(_raw_arg_0) => match _raw_arg_0.clone().string() {
                    Some(v) => Uuid::parse_str(&v).expect("Invalid UUID string"),
                    None => {
                        log_decode_error(
                            addr,
                            DispatchError::WrongArgumentType {
                                expected: "uuid (as string)",
                                got: osc_type_name(_raw_arg_0),
                            },
                        );
                        return;
                    }
                },
                None => {
                    log_decode_error(addr, DispatchError::MissingArgument { arg_index: 0 });
                    return;
                }
            };
            handler(TrackSendGuidArgs {
                guid: _decoded_guid,
            });
        }
        return;
    }
    if let Some(args) = match_addr(addr, "/track/{track_guid}/send/{send_index}/volume") {
        let track_guid: Uuid = match Uuid::parse_str(&args[0]) {
            Ok(v) => v,
            Err(_) => {
                log_decode_error(
                    addr,
                    DispatchError::ParamParseError {
                        param: "track_guid",
                        value: args[0].clone(),
                    },
                );
                return;
            }
        };
        let send_index: i32 = match args[1].parse::<i32>() {
            Ok(v) => v,
            Err(_) => {
                log_decode_error(
                    addr,
                    DispatchError::ParamParseError {
                        param: "send_index",
                        value: args[1].clone(),
                    },
                );
                return;
            }
        };
        let endpoint = reaper.track_send_volume(track_guid, send_index);
        if let Some(handler) = &mut endpoint.handler {
            let _decoded_volume = match msg.args.get(0) {
                Some(_raw_arg_0) => match _raw_arg_0.clone().float() {
                    Some(v) => v,
                    None => {
                        log_decode_error(
                            addr,
                            DispatchError::WrongArgumentType {
                                expected: "float",
                                got: osc_type_name(_raw_arg_0),
                            },
                        );
                        return;
                    }
                },
                None => {
                    log_decode_error(addr, DispatchError::MissingArgument { arg_index: 0 });
                    return;
                }
            };
            handler(TrackSendVolumeArgs {
                volume: _decoded_volume,
            });
        }
        return;
    }
    if let Some(args) = match_addr(addr, "/track/{track_guid}/send/{send_index}/pan") {
        let track_guid: Uuid = match Uuid::parse_str(&args[0]) {
            Ok(v) => v,
            Err(_) => {
                log_decode_error(
                    addr,
                    DispatchError::ParamParseError {
                        param: "track_guid",
                        value: args[0].clone(),
                    },
                );
                return;
            }
        };
        let send_index: i32 = match args[1].parse::<i32>() {
            Ok(v) => v,
            Err(_) => {
                log_decode_error(
                    addr,
                    DispatchError::ParamParseError {
                        param: "send_index",
                        value: args[1].clone(),
                    },
                );
                return;
            }
        };
        let endpoint = reaper.track_send_pan(track_guid, send_index);
        if let Some(handler) = &mut endpoint.handler {
            let _decoded_pan = match msg.args.get(0) {
                Some(_raw_arg_0) => match _raw_arg_0.clone().float() {
                    Some(v) => v,
                    None => {
                        log_decode_error(
                            addr,
                            DispatchError::WrongArgumentType {
                                expected: "float",
                                got: osc_type_name(_raw_arg_0),
                            },
                        );
                        return;
                    }
                },
                None => {
                    log_decode_error(addr, DispatchError::MissingArgument { arg_index: 0 });
                    return;
                }
            };
            handler(TrackSendPanArgs { pan: _decoded_pan });
        }
        return;
    }
    if let Some(args) = match_addr(addr, "/track/{track_guid}/color") {
        println!("MATCHED COLOR PATTERN");
        let track_guid: Uuid = match Uuid::parse_str(&args[0]) {
            Ok(v) => v,
            Err(_) => {
                log_decode_error(
                    addr,
                    DispatchError::ParamParseError {
                        param: "track_guid",
                        value: args[0].clone(),
                    },
                );
                return;
            }
        };
        let endpoint = reaper.track_color(track_guid);
        if let Some(handler) = &mut endpoint.handler {
            println!("HAVE HANDLER");
            let _decoded_r = match msg.args.get(0) {
                Some(_raw_arg_0) => match _raw_arg_0.clone().int() {
                    Some(v) => v,
                    None => {
                        log_decode_error(
                            addr,
                            DispatchError::WrongArgumentType {
                                expected: "int",
                                got: osc_type_name(_raw_arg_0),
                            },
                        );
                        return;
                    }
                },
                None => {
                    log_decode_error(addr, DispatchError::MissingArgument { arg_index: 0 });
                    return;
                }
            };
            let _decoded_g = match msg.args.get(1) {
                Some(_raw_arg_1) => match _raw_arg_1.clone().int() {
                    Some(v) => v,
                    None => {
                        log_decode_error(
                            addr,
                            DispatchError::WrongArgumentType {
                                expected: "int",
                                got: osc_type_name(_raw_arg_1),
                            },
                        );
                        return;
                    }
                },
                None => {
                    log_decode_error(addr, DispatchError::MissingArgument { arg_index: 1 });
                    return;
                }
            };
            let _decoded_b = match msg.args.get(2) {
                Some(_raw_arg_2) => match _raw_arg_2.clone().int() {
                    Some(v) => v,
                    None => {
                        log_decode_error(
                            addr,
                            DispatchError::WrongArgumentType {
                                expected: "int",
                                got: osc_type_name(_raw_arg_2),
                            },
                        );
                        return;
                    }
                },
                None => {
                    log_decode_error(addr, DispatchError::MissingArgument { arg_index: 2 });
                    return;
                }
            };
            handler(TrackColorArgs {
                r: _decoded_r,
                g: _decoded_g,
                b: _decoded_b,
            });
        }
        return;
    }
    if let Some(args) = match_addr(addr, "/track/{track_guid}/fx/{fx_idx}/guid") {
        let track_guid: Uuid = match Uuid::parse_str(&args[0]) {
            Ok(v) => v,
            Err(_) => {
                log_decode_error(
                    addr,
                    DispatchError::ParamParseError {
                        param: "track_guid",
                        value: args[0].clone(),
                    },
                );
                return;
            }
        };
        let fx_idx: i32 = match args[1].parse::<i32>() {
            Ok(v) => v,
            Err(_) => {
                log_decode_error(
                    addr,
                    DispatchError::ParamParseError {
                        param: "fx_idx",
                        value: args[1].clone(),
                    },
                );
                return;
            }
        };
        let endpoint = reaper.track_fx_guid(track_guid, fx_idx);
        if let Some(handler) = &mut endpoint.handler {
            let _decoded_guid = match msg.args.get(0) {
                Some(_raw_arg_0) => match _raw_arg_0.clone().string() {
                    Some(v) => Uuid::parse_str(&v).expect("Invalid UUID string"),
                    None => {
                        log_decode_error(
                            addr,
                            DispatchError::WrongArgumentType {
                                expected: "uuid (as string)",
                                got: osc_type_name(_raw_arg_0),
                            },
                        );
                        return;
                    }
                },
                None => {
                    log_decode_error(addr, DispatchError::MissingArgument { arg_index: 0 });
                    return;
                }
            };
            handler(TrackFxGuidArgs {
                guid: _decoded_guid,
            });
        }
        return;
    }
    if let Some(args) = match_addr(addr, "/track/{track_guid}/fx/{fx_idx}/name") {
        let track_guid: Uuid = match Uuid::parse_str(&args[0]) {
            Ok(v) => v,
            Err(_) => {
                log_decode_error(
                    addr,
                    DispatchError::ParamParseError {
                        param: "track_guid",
                        value: args[0].clone(),
                    },
                );
                return;
            }
        };
        let fx_idx: i32 = match args[1].parse::<i32>() {
            Ok(v) => v,
            Err(_) => {
                log_decode_error(
                    addr,
                    DispatchError::ParamParseError {
                        param: "fx_idx",
                        value: args[1].clone(),
                    },
                );
                return;
            }
        };
        let endpoint = reaper.track_fx_name(track_guid, fx_idx);
        if let Some(handler) = &mut endpoint.handler {
            let _decoded_name = match msg.args.get(0) {
                Some(_raw_arg_0) => match _raw_arg_0.clone().string() {
                    Some(v) => v,
                    None => {
                        log_decode_error(
                            addr,
                            DispatchError::WrongArgumentType {
                                expected: "string",
                                got: osc_type_name(_raw_arg_0),
                            },
                        );
                        return;
                    }
                },
                None => {
                    log_decode_error(addr, DispatchError::MissingArgument { arg_index: 0 });
                    return;
                }
            };
            handler(TrackFxNameArgs {
                name: _decoded_name,
            });
        }
        return;
    }
    if let Some(args) = match_addr(addr, "/track/{track_guid}/fx/{fx_idx}/enabled") {
        let track_guid: Uuid = match Uuid::parse_str(&args[0]) {
            Ok(v) => v,
            Err(_) => {
                log_decode_error(
                    addr,
                    DispatchError::ParamParseError {
                        param: "track_guid",
                        value: args[0].clone(),
                    },
                );
                return;
            }
        };
        let fx_idx: i32 = match args[1].parse::<i32>() {
            Ok(v) => v,
            Err(_) => {
                log_decode_error(
                    addr,
                    DispatchError::ParamParseError {
                        param: "fx_idx",
                        value: args[1].clone(),
                    },
                );
                return;
            }
        };
        let endpoint = reaper.track_fx_enabled(track_guid, fx_idx);
        if let Some(handler) = &mut endpoint.handler {
            let _decoded_enabled = match msg.args.get(0) {
                Some(_raw_arg_0) => match _raw_arg_0.clone().bool() {
                    Some(v) => v,
                    None => {
                        log_decode_error(
                            addr,
                            DispatchError::WrongArgumentType {
                                expected: "bool",
                                got: osc_type_name(_raw_arg_0),
                            },
                        );
                        return;
                    }
                },
                None => {
                    log_decode_error(addr, DispatchError::MissingArgument { arg_index: 0 });
                    return;
                }
            };
            handler(TrackFxEnabledArgs {
                enabled: _decoded_enabled,
            });
        }
        return;
    }
    if let Some(args) = match_addr(addr, "/track/{track_guid}/fx/{fx_idx}/param_count") {
        let track_guid: Uuid = match Uuid::parse_str(&args[0]) {
            Ok(v) => v,
            Err(_) => {
                log_decode_error(
                    addr,
                    DispatchError::ParamParseError {
                        param: "track_guid",
                        value: args[0].clone(),
                    },
                );
                return;
            }
        };
        let fx_idx: i32 = match args[1].parse::<i32>() {
            Ok(v) => v,
            Err(_) => {
                log_decode_error(
                    addr,
                    DispatchError::ParamParseError {
                        param: "fx_idx",
                        value: args[1].clone(),
                    },
                );
                return;
            }
        };
        let endpoint = reaper.track_fx_param_count(track_guid, fx_idx);
        if let Some(handler) = &mut endpoint.handler {
            let _decoded_param_count = match msg.args.get(0) {
                Some(_raw_arg_0) => match _raw_arg_0.clone().int() {
                    Some(v) => v,
                    None => {
                        log_decode_error(
                            addr,
                            DispatchError::WrongArgumentType {
                                expected: "int",
                                got: osc_type_name(_raw_arg_0),
                            },
                        );
                        return;
                    }
                },
                None => {
                    log_decode_error(addr, DispatchError::MissingArgument { arg_index: 0 });
                    return;
                }
            };
            handler(TrackFxParamCountArgs {
                param_count: _decoded_param_count,
            });
        }
        return;
    }
    if let Some(args) = match_addr(
        addr,
        "/track/{track_guid}/fx/{fx_idx}/param/{param_idx}/name",
    ) {
        let track_guid: Uuid = match Uuid::parse_str(&args[0]) {
            Ok(v) => v,
            Err(_) => {
                log_decode_error(
                    addr,
                    DispatchError::ParamParseError {
                        param: "track_guid",
                        value: args[0].clone(),
                    },
                );
                return;
            }
        };
        let fx_idx: i32 = match args[1].parse::<i32>() {
            Ok(v) => v,
            Err(_) => {
                log_decode_error(
                    addr,
                    DispatchError::ParamParseError {
                        param: "fx_idx",
                        value: args[1].clone(),
                    },
                );
                return;
            }
        };
        let param_idx: i32 = match args[2].parse::<i32>() {
            Ok(v) => v,
            Err(_) => {
                log_decode_error(
                    addr,
                    DispatchError::ParamParseError {
                        param: "param_idx",
                        value: args[2].clone(),
                    },
                );
                return;
            }
        };
        let endpoint = reaper.track_fx_param_name(track_guid, fx_idx, param_idx);
        if let Some(handler) = &mut endpoint.handler {
            let _decoded_param_name = match msg.args.get(0) {
                Some(_raw_arg_0) => match _raw_arg_0.clone().string() {
                    Some(v) => v,
                    None => {
                        log_decode_error(
                            addr,
                            DispatchError::WrongArgumentType {
                                expected: "string",
                                got: osc_type_name(_raw_arg_0),
                            },
                        );
                        return;
                    }
                },
                None => {
                    log_decode_error(addr, DispatchError::MissingArgument { arg_index: 0 });
                    return;
                }
            };
            handler(TrackFxParamNameArgs {
                param_name: _decoded_param_name,
            });
        }
        return;
    }
    if let Some(args) = match_addr(
        addr,
        "/track/{track_guid}/fx/{fx_idx}/param/{param_idx}/value",
    ) {
        let track_guid: Uuid = match Uuid::parse_str(&args[0]) {
            Ok(v) => v,
            Err(_) => {
                log_decode_error(
                    addr,
                    DispatchError::ParamParseError {
                        param: "track_guid",
                        value: args[0].clone(),
                    },
                );
                return;
            }
        };
        let fx_idx: i32 = match args[1].parse::<i32>() {
            Ok(v) => v,
            Err(_) => {
                log_decode_error(
                    addr,
                    DispatchError::ParamParseError {
                        param: "fx_idx",
                        value: args[1].clone(),
                    },
                );
                return;
            }
        };
        let param_idx: i32 = match args[2].parse::<i32>() {
            Ok(v) => v,
            Err(_) => {
                log_decode_error(
                    addr,
                    DispatchError::ParamParseError {
                        param: "param_idx",
                        value: args[2].clone(),
                    },
                );
                return;
            }
        };
        let endpoint = reaper.track_fx_param_value(track_guid, fx_idx, param_idx);
        if let Some(handler) = &mut endpoint.handler {
            let _decoded_value = match msg.args.get(0) {
                Some(_raw_arg_0) => match _raw_arg_0.clone().float() {
                    Some(v) => v,
                    None => {
                        log_decode_error(
                            addr,
                            DispatchError::WrongArgumentType {
                                expected: "float",
                                got: osc_type_name(_raw_arg_0),
                            },
                        );
                        return;
                    }
                },
                None => {
                    log_decode_error(addr, DispatchError::MissingArgument { arg_index: 0 });
                    return;
                }
            };
            handler(TrackFxParamValueArgs {
                value: _decoded_value,
            });
        }
        return;
    }
    if let Some(args) = match_addr(
        addr,
        "/track/{track_guid}/fx/{fx_idx}/param/{param_idx}/min",
    ) {
        let track_guid: Uuid = match Uuid::parse_str(&args[0]) {
            Ok(v) => v,
            Err(_) => {
                log_decode_error(
                    addr,
                    DispatchError::ParamParseError {
                        param: "track_guid",
                        value: args[0].clone(),
                    },
                );
                return;
            }
        };
        let fx_idx: i32 = match args[1].parse::<i32>() {
            Ok(v) => v,
            Err(_) => {
                log_decode_error(
                    addr,
                    DispatchError::ParamParseError {
                        param: "fx_idx",
                        value: args[1].clone(),
                    },
                );
                return;
            }
        };
        let param_idx: i32 = match args[2].parse::<i32>() {
            Ok(v) => v,
            Err(_) => {
                log_decode_error(
                    addr,
                    DispatchError::ParamParseError {
                        param: "param_idx",
                        value: args[2].clone(),
                    },
                );
                return;
            }
        };
        let endpoint = reaper.track_fx_param_min(track_guid, fx_idx, param_idx);
        if let Some(handler) = &mut endpoint.handler {
            let _decoded_min = match msg.args.get(0) {
                Some(_raw_arg_0) => match _raw_arg_0.clone().float() {
                    Some(v) => v,
                    None => {
                        log_decode_error(
                            addr,
                            DispatchError::WrongArgumentType {
                                expected: "float",
                                got: osc_type_name(_raw_arg_0),
                            },
                        );
                        return;
                    }
                },
                None => {
                    log_decode_error(addr, DispatchError::MissingArgument { arg_index: 0 });
                    return;
                }
            };
            handler(TrackFxParamMinArgs { min: _decoded_min });
        }
        return;
    }
    if let Some(args) = match_addr(
        addr,
        "/track/{track_guid}/fx/{fx_idx}/param/{param_idx}/max",
    ) {
        let track_guid: Uuid = match Uuid::parse_str(&args[0]) {
            Ok(v) => v,
            Err(_) => {
                log_decode_error(
                    addr,
                    DispatchError::ParamParseError {
                        param: "track_guid",
                        value: args[0].clone(),
                    },
                );
                return;
            }
        };
        let fx_idx: i32 = match args[1].parse::<i32>() {
            Ok(v) => v,
            Err(_) => {
                log_decode_error(
                    addr,
                    DispatchError::ParamParseError {
                        param: "fx_idx",
                        value: args[1].clone(),
                    },
                );
                return;
            }
        };
        let param_idx: i32 = match args[2].parse::<i32>() {
            Ok(v) => v,
            Err(_) => {
                log_decode_error(
                    addr,
                    DispatchError::ParamParseError {
                        param: "param_idx",
                        value: args[2].clone(),
                    },
                );
                return;
            }
        };
        let endpoint = reaper.track_fx_param_max(track_guid, fx_idx, param_idx);
        if let Some(handler) = &mut endpoint.handler {
            let _decoded_max = match msg.args.get(0) {
                Some(_raw_arg_0) => match _raw_arg_0.clone().float() {
                    Some(v) => v,
                    None => {
                        log_decode_error(
                            addr,
                            DispatchError::WrongArgumentType {
                                expected: "float",
                                got: osc_type_name(_raw_arg_0),
                            },
                        );
                        return;
                    }
                },
                None => {
                    log_decode_error(addr, DispatchError::MissingArgument { arg_index: 0 });
                    return;
                }
            };
            handler(TrackFxParamMaxArgs { max: _decoded_max });
        }
        return;
    }
    if let Some(args) = match_addr(addr, "/track/{track_guid}/fx/{fx_idx}/info") {
        let track_guid: Uuid = match Uuid::parse_str(&args[0]) {
            Ok(v) => v,
            Err(_) => {
                log_decode_error(
                    addr,
                    DispatchError::ParamParseError {
                        param: "track_guid",
                        value: args[0].clone(),
                    },
                );
                return;
            }
        };
        let fx_idx: i32 = match args[1].parse::<i32>() {
            Ok(v) => v,
            Err(_) => {
                log_decode_error(
                    addr,
                    DispatchError::ParamParseError {
                        param: "fx_idx",
                        value: args[1].clone(),
                    },
                );
                return;
            }
        };
        let endpoint = reaper.track_fx_info(track_guid, fx_idx);
        if let Some(handler) = &mut endpoint.handler {
            handler(TrackFxInfoArgs {});
        }
        return;
    }
    if let Some(args) = match_addr(addr, "/fxinfo/{ident}/name") {
        let ident = args[0].clone();
        let endpoint = reaper.fxinfo_name(ident);
        if let Some(handler) = &mut endpoint.handler {
            let _decoded_name = match msg.args.get(0) {
                Some(_raw_arg_0) => match _raw_arg_0.clone().string() {
                    Some(v) => v,
                    None => {
                        log_decode_error(
                            addr,
                            DispatchError::WrongArgumentType {
                                expected: "string",
                                got: osc_type_name(_raw_arg_0),
                            },
                        );
                        return;
                    }
                },
                None => {
                    log_decode_error(addr, DispatchError::MissingArgument { arg_index: 0 });
                    return;
                }
            };
            handler(FxinfoNameArgs {
                name: _decoded_name,
            });
        }
        return;
    }
    if let Some(args) = match_addr(addr, "/fxinfo/{ident}/param_count") {
        let ident = args[0].clone();
        let endpoint = reaper.fxinfo_param_count(ident);
        if let Some(handler) = &mut endpoint.handler {
            let _decoded_param_count = match msg.args.get(0) {
                Some(_raw_arg_0) => match _raw_arg_0.clone().int() {
                    Some(v) => v,
                    None => {
                        log_decode_error(
                            addr,
                            DispatchError::WrongArgumentType {
                                expected: "int",
                                got: osc_type_name(_raw_arg_0),
                            },
                        );
                        return;
                    }
                },
                None => {
                    log_decode_error(addr, DispatchError::MissingArgument { arg_index: 0 });
                    return;
                }
            };
            handler(FxinfoParamCountArgs {
                param_count: _decoded_param_count,
            });
        }
        return;
    }
    if let Some(args) = match_addr(addr, "/fxinfo/{ident}/param/{param_idx}/name") {
        let ident = args[0].clone();
        let param_idx: i32 = match args[1].parse::<i32>() {
            Ok(v) => v,
            Err(_) => {
                log_decode_error(
                    addr,
                    DispatchError::ParamParseError {
                        param: "param_idx",
                        value: args[1].clone(),
                    },
                );
                return;
            }
        };
        let endpoint = reaper.fxinfo_param_name(ident, param_idx);
        if let Some(handler) = &mut endpoint.handler {
            let _decoded_param_name = match msg.args.get(0) {
                Some(_raw_arg_0) => match _raw_arg_0.clone().string() {
                    Some(v) => v,
                    None => {
                        log_decode_error(
                            addr,
                            DispatchError::WrongArgumentType {
                                expected: "string",
                                got: osc_type_name(_raw_arg_0),
                            },
                        );
                        return;
                    }
                },
                None => {
                    log_decode_error(addr, DispatchError::MissingArgument { arg_index: 0 });
                    return;
                }
            };
            handler(FxinfoParamNameArgs {
                param_name: _decoded_param_name,
            });
        }
        return;
    }
    if let Some(args) = match_addr(addr, "/fxinfo/{ident}/param/{param_idx}/min") {
        let ident = args[0].clone();
        let param_idx: i32 = match args[1].parse::<i32>() {
            Ok(v) => v,
            Err(_) => {
                log_decode_error(
                    addr,
                    DispatchError::ParamParseError {
                        param: "param_idx",
                        value: args[1].clone(),
                    },
                );
                return;
            }
        };
        let endpoint = reaper.fxinfo_param_min(ident, param_idx);
        if let Some(handler) = &mut endpoint.handler {
            let _decoded_param_min = match msg.args.get(0) {
                Some(_raw_arg_0) => match _raw_arg_0.clone().float() {
                    Some(v) => v,
                    None => {
                        log_decode_error(
                            addr,
                            DispatchError::WrongArgumentType {
                                expected: "float",
                                got: osc_type_name(_raw_arg_0),
                            },
                        );
                        return;
                    }
                },
                None => {
                    log_decode_error(addr, DispatchError::MissingArgument { arg_index: 0 });
                    return;
                }
            };
            handler(FxinfoParamMinArgs {
                param_min: _decoded_param_min,
            });
        }
        return;
    }
    if let Some(args) = match_addr(addr, "/fxinfo/{ident}/param/{param_idx}/max") {
        let ident = args[0].clone();
        let param_idx: i32 = match args[1].parse::<i32>() {
            Ok(v) => v,
            Err(_) => {
                log_decode_error(
                    addr,
                    DispatchError::ParamParseError {
                        param: "param_idx",
                        value: args[1].clone(),
                    },
                );
                return;
            }
        };
        let endpoint = reaper.fxinfo_param_max(ident, param_idx);
        if let Some(handler) = &mut endpoint.handler {
            let _decoded_param_max = match msg.args.get(0) {
                Some(_raw_arg_0) => match _raw_arg_0.clone().float() {
                    Some(v) => v,
                    None => {
                        log_decode_error(
                            addr,
                            DispatchError::WrongArgumentType {
                                expected: "float",
                                got: osc_type_name(_raw_arg_0),
                            },
                        );
                        return;
                    }
                },
                None => {
                    log_decode_error(addr, DispatchError::MissingArgument { arg_index: 0 });
                    return;
                }
            };
            handler(FxinfoParamMaxArgs {
                param_max: _decoded_param_max,
            });
        }
        return;
    }
    if let Some(_args) = match_addr(addr, "/fxinfo") {
        let endpoint = reaper.fxinfo();
        if let Some(handler) = &mut endpoint.handler {
            handler(FxinfoArgs {});
        }
        return;
    }
    log_unknown_route(addr);
}
