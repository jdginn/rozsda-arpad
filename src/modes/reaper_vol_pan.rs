use std::collections::HashMap;

use uuid::Uuid;

use crate::midi::v1m::{self};
use crate::midi::v1m::{DownstreamMsg, EncoderRingMsg, LEDState, SelectLEDMsg, UpstreamMsg};
use crate::modes::mode_manager::{
    DownstreamIo, ModeAction, ModeHandler, TransitionRequest, UpstreamIo,
};
use crate::modes::reaper_faders_buttons_core::VolumeFadersCore;
use crate::track::track;
use crate::track::track::TrackMsg;

pub const FADER_0DB: f32 = 0.72; // Placeholder value for 0dB on fader scale

pub fn map_to_0xb(x: f32) -> u8 {
    let clamped = x.clamp(-1.0, 1.0) as f64;
    ((clamped + 1.0) * 0.5 * 0xb as f64).round() as u8
}

/// Implements a mode where that "basic" reaper functionality is mapped to the channel strips on
/// the control surface, namely:
/// - Volume on faders
/// - Select/Mute/Solo/Arm on buttons
/// - Pan on rotary encoders
///
/// Button LED toggling is handled here (downstream does not need to worry about managing button
/// LEDS.)
///
pub struct VolumePanMode {
    core: VolumeFadersCore,
    pan_states: HashMap<Uuid, f32>,
    selected_track_guid: Option<Uuid>,
}

impl VolumePanMode {
    pub fn new(num_channels: usize, selected_track_guid: Option<Uuid>) -> Self {
        VolumePanMode {
            core: VolumeFadersCore::new(num_channels),
            pan_states: HashMap::new(),
            selected_track_guid,
        }
    }
}

impl ModeHandler for VolumePanMode {
    fn handle_msg_from_upstream(&mut self, msg: TrackMsg, io: &mut dyn UpstreamIo) -> ModeAction {
        match track::DataMsg::try_from(msg) {
            Ok(msg) => {
                match msg {
                    track::DataMsg::Selected(msg) => {
                        let state = match msg.selected {
                            true => v1m::LEDState::On,
                            false => v1m::LEDState::Off,
                        };
                        io.send_to_v1m(
                            v1m::SelectLEDMsg {
                                idx: self.core.find_hw_channel(msg.track_guid).unwrap_or(0) as i32,
                                state,
                            }
                            .into(),
                        );
                        ModeAction::None
                    }
                    track::DataMsg::Name(msg) => {
                        io.send_to_v1m(
                            v1m::ScribbleStripLine1TextMsg {
                                idx: self.core.find_hw_channel(msg.track_guid).unwrap_or(0) as i32,
                                text: msg.name.clone(),
                            }
                            .into(),
                        );
                        io.send_to_v1m(
                            v1m::BottomScribbleStripLine2TextMsg {
                                idx: self.core.find_hw_channel(msg.track_guid).unwrap_or(0) as i32,
                                text: msg.name,
                            }
                            .into(),
                        );
                        ModeAction::None
                    }
                    _ => {
                        // Handle common functionality across modes that put volume on the faders
                        // and mute/solo/arm on the buttons
                        self.core.handle_msg_from_upstream(msg, io, |data| {
                            // This closure defines what happens when a track's index changes.
                            // Each mode that builds from VolumeFadersCore may need to define its own behavior here.
                            let pan_val = self.pan_states.entry(data.track_guid).or_insert(0.5); // Default center pan
                            Some(vec![
                                v1m::EncoderRingMsg {
                                    idx: data.hw_channel as i32,
                                    mode: v1m::EncoderRingMode::Point,
                                    val: map_to_0xb(*pan_val),
                                }
                                .into(),
                            ])
                        });
                        // In addition to the common functionality, handle a few edge casess:
                        // Ignore unhandled payloads (e.g., Selected, SendIndex, etc.)
                        ModeAction::None
                    }
                }
            }
            Err(_) => {
                panic!("Code bug: VolumePanMode should only receive TrackDataMsg messages.");
            }
        }
    }
    fn handle_msg_from_downstream(
        &mut self,
        msg: v1m::UpstreamMsg,
        io: &mut dyn DownstreamIo,
    ) -> ModeAction {
        match msg {
            // GlobalPress maps to this mode!
            v1m::UpstreamMsg::GlobalPress => ModeAction::None,
            // MIDITracksPress maps to ReaperSends mode
            UpstreamMsg::MIDITracksPress => {
                println!("Requesting transition to ReaperSends mode");
                if let Some(guid) = self.selected_track_guid {
                    ModeAction::Transition(TransitionRequest::ToReaperSends {
                        selected_track_guid: guid,
                    })
                } else {
                    ModeAction::None
                }
            }
            v1m::UpstreamMsg::InputsPress => {
                println!("Requesting transition to ReaperChannelStrip mode");
                if let Some(guid) = self.selected_track_guid {
                    ModeAction::Transition(TransitionRequest::ToReaperChannelStrip {
                        selected_track_guid: guid,
                    })
                } else {
                    ModeAction::None
                }
            }
            v1m::UpstreamMsg::SelectPress(select_msg) => {
                io.send_to_v1m(DownstreamMsg::SelectLED(SelectLEDMsg {
                    idx: select_msg.idx,
                    state: LEDState::On,
                }));
                if let Some(guid) = self.core.get_guid_for_hw_channel(select_msg.idx as usize) {
                    io.send_to_reaper(
                        track::Selected {
                            track_guid: guid,
                            selected: true,
                        }
                        .into(),
                    );
                }
                // TODO: what do we do with this?
                let new_selected_track_guid =
                    self.core.get_guid_for_hw_channel(select_msg.idx as usize);
                self.selected_track_guid = new_selected_track_guid;
                ModeAction::None
            }
            v1m::UpstreamMsg::EncoderTurnInc(encoder_msg) => {
                if let Some(guid) = self.core.get_guid_for_hw_channel(encoder_msg.idx as usize) {
                    // Get current pan value and increment it
                    let current_pan = self.pan_states.entry(guid).or_insert(0.5); // Default center pan
                    let new_pan = match encoder_msg.accel {
                        1 => (*current_pan + 0.05).min(1.0), // Increment by 0.05, clamp to max 1.0
                        2 => (*current_pan + 0.1).min(1.0),  // Increment by 0.07, clamp to max 1.0
                        3 => (*current_pan + 0.2).min(1.0),  // Increment by 0.1, clamp to max 1.0
                        4 => (*current_pan + 0.4).min(1.0),  // Increment by 0.2, clamp to max 1.0
                        _ => (*current_pan + 0.4).min(1.0),  // Increment by 0.2, clamp to max 1.0
                    };
                    self.pan_states.insert(guid, new_pan);

                    // Send pan update upstream to Reaper
                    io.send_to_reaper(
                        track::Pan {
                            track_guid: guid,
                            pan: new_pan,
                        }
                        .into(),
                    );

                    // Send encoder LED update downstream to hardware
                    io.send_to_v1m(DownstreamMsg::EncoderRingLED(EncoderRingMsg {
                        idx: encoder_msg.idx,
                        mode: v1m::EncoderRingMode::FromCenter,
                        val: map_to_0xb(new_pan),
                    }));
                }
                ModeAction::None
            }
            v1m::UpstreamMsg::EncoderTurnDec(encoder_msg) => {
                if let Some(guid) = self.core.get_guid_for_hw_channel(encoder_msg.idx as usize) {
                    // Get current pan value and decrement it
                    let current_pan = self.pan_states.entry(guid).or_insert(0.5); // Default center pan
                    let new_pan = match encoder_msg.accel {
                        1 => (*current_pan - 0.05).max(-1.0), // Decrement by 0.05, clamp to min -1.0
                        2 => (*current_pan - 0.1).max(-1.0), // Decrement by 0.05, clamp to min -1.0
                        3 => (*current_pan - 0.2).max(-1.0), // Decrement by 0.05, clamp to min -1.0
                        4 => (*current_pan - 0.4).max(-1.0), // Decrement by 0.05, clamp to min -1.0
                        _ => (*current_pan - 0.4).max(-1.0), // Decrement by 0.05, clamp to min -1.0
                    };
                    self.pan_states.insert(guid, new_pan);

                    // Send pan update upstream to Reaper
                    io.send_to_reaper(
                        track::Pan {
                            track_guid: guid,
                            pan: new_pan,
                        }
                        .into(),
                    );

                    // Send encoder LED update downstream to hardware
                    io.send_to_v1m(DownstreamMsg::EncoderRingLED(EncoderRingMsg {
                        idx: encoder_msg.idx,
                        mode: v1m::EncoderRingMode::FromCenter,
                        val: map_to_0xb(new_pan),
                    }));
                }
                ModeAction::None
            }
            _ => {
                self.core.handle_msg_from_downstream(msg, io);
                ModeAction::None
            }
        }
    }
}
