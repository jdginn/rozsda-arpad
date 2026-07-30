use std::collections::HashMap;

use crossbeam_channel::{Receiver, Sender};
use uuid::Uuid;

use crate::midi::v1m::{self};
use crate::midi::v1m::{
    ChannelFaderMsg, DownstreamMsg, EncoderRingMsg, LEDState, SelectLEDMsg, UpstreamMsg,
};
use crate::modes::mode_manager::{Barrier, Mode, ModeHandler, ModeState, State};
use crate::modes::reaper_faders_buttons_core::VolumeFadersCore;
use crate::track::track;
use crate::track::track::{TrackMsg, TrackQuery};

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
pub struct VolumePanMode {
    core: VolumeFadersCore,
    to_reaper: Sender<TrackMsg>,
    _from_reaper: Receiver<TrackMsg>,
    to_v1m: Sender<v1m::DownstreamMsg>,
    _from_v1m: Receiver<v1m::UpstreamMsg>,
    pan_states: HashMap<Uuid, f32>,
}

impl VolumePanMode {
    pub fn new(
        num_channels: usize,
        from_reaper: Receiver<TrackMsg>,
        to_reaper: Sender<TrackMsg>,
        from_v1m: Receiver<v1m::UpstreamMsg>,
        to_v1m: Sender<v1m::DownstreamMsg>,
    ) -> Self {
        VolumePanMode {
            core: VolumeFadersCore::new(num_channels),
            pan_states: HashMap::new(),
            to_reaper,
            _from_reaper: from_reaper,
            to_v1m,
            _from_v1m: from_v1m,
        }
    }

    pub fn find_hw_channel(&self, guid: Uuid) -> Option<usize> {
        self.core.find_hw_channel(guid)
    }
}

impl ModeHandler<TrackMsg, TrackMsg, v1m::DownstreamMsg, v1m::UpstreamMsg> for VolumePanMode {
    fn handle_messages_from_upstream(&mut self, msg: TrackMsg, curr_mode: ModeState) -> ModeState {
        match track::DataMsg::try_from(msg) {
            Err(TrackMsg::Barrier(barrier)) => {
                // Forward barriers downstream (they need to reflect back upstream for the mode to
                // transition)
                self.to_v1m
                    .send(v1m::DownstreamMsg::Barrier(barrier))
                    .unwrap();
                match curr_mode.state {
                    // If we were already waiting on a barrier from upstream, check if this is the one
                    // we were waiting for. If yes, transition to waiting for the barrier to reflect back up from downstream.
                    State::WaitingBarrierFromUpstream(expected_barrier) => {
                        if barrier == expected_barrier {
                            ModeState {
                                mode: curr_mode.mode,
                                state: State::WaitingBarrierFromDownstream(barrier),
                                new_selected_track_guid: None,
                            }
                        } else {
                            curr_mode
                        }
                    }
                    _ => curr_mode,
                }
            }
            Ok(msg) => {
                match msg {
                    track::DataMsg::Selected(msg) => {
                        let state = match msg.selected {
                            true => v1m::LEDState::On,
                            false => v1m::LEDState::Off,
                        };
                        self.to_v1m
                            .send(
                                v1m::SelectLEDMsg {
                                    idx: self.core.find_hw_channel(msg.track_guid).unwrap_or(0)
                                        as i32,
                                    state,
                                }
                                .into(),
                            )
                            .unwrap();
                        curr_mode
                        // ModeState {
                        //     mode: curr_mode.mode,
                        //     state: curr_mode.state,
                        //     new_selected_track_guid: Some(msg.track_guid),
                        // }
                    }
                    track::DataMsg::Name(msg) => {
                        self.to_v1m
                            .send(
                                v1m::ScribbleStripLine1TextMsg {
                                    idx: self.core.find_hw_channel(msg.track_guid).unwrap_or(0)
                                        as i32,
                                    text: msg.name.clone(),
                                }
                                .into(),
                            )
                            .unwrap();
                        self.to_v1m
                            .send(
                                v1m::BottomScribbleStripLine2TextMsg {
                                    idx: self.core.find_hw_channel(msg.track_guid).unwrap_or(0)
                                        as i32,
                                    text: msg.name,
                                }
                                .into(),
                            )
                            .unwrap();
                        curr_mode
                    }
                    _ => {
                        // Handle common functionality across modes that put volume on the faders
                        // and mute/solo/arm on the buttons
                        self.core
                            .handle_message_from_upstream(msg, self.to_v1m.clone(), |data| {
                                let pan_val = self.pan_states.entry(data.track_guid).or_insert(0.5); // Default center pan
                                self.to_v1m
                                    .send(
                                        v1m::EncoderRingMsg {
                                            idx: data.hw_channel as i32,
                                            mode: v1m::EncoderRingMode::Point,
                                            val: map_to_0xb(*pan_val),
                                        }
                                        .into(),
                                    )
                                    .unwrap();
                            });
                        // In addition to the common functionality, handle a few edge casess:
                        // Ignore unhandled payloads (e.g., Selected, SendIndex, etc.)
                        curr_mode
                    }
                }
            }
            Err(_) => {
                // Ignore messages that aren't TrackDataMsg or Barrier
                curr_mode
            }
        }
    }
    fn handle_messages_from_downstream(
        &mut self,
        msg: v1m::UpstreamMsg,
        curr_mode: ModeState,
    ) -> ModeState {
        match msg {
            // GlobalPress maps to this mode!
            v1m::UpstreamMsg::GlobalPress => curr_mode,
            // MIDITracksPress maps to ReaperSends mode
            UpstreamMsg::MIDITracksPress => {
                println!("Requesting transition to ReaperSends mode");
                // Request transition to ReaperSends mode
                ModeState {
                    mode: Mode::ReaperSends,
                    state: State::RequestingModeTransition,
                    new_selected_track_guid: None,
                }
            }
            v1m::UpstreamMsg::InputsPress => {
                // Request transition to ReaperChannelStrip mode
                ModeState {
                    mode: Mode::ReaperChannelStrip,
                    state: State::RequestingModeTransition,
                    new_selected_track_guid: None,
                }
            }
            v1m::UpstreamMsg::SelectPress(select_msg) => {
                self.to_v1m
                    .send(DownstreamMsg::SelectLED(SelectLEDMsg {
                        idx: select_msg.idx,
                        state: LEDState::On,
                    }))
                    .unwrap();
                if let Some(guid) = self.core.get_guid_for_hw_channel(select_msg.idx as usize) {
                    self.to_reaper
                        .send(
                            track::Selected {
                                track_guid: guid,
                                selected: true,
                            }
                            .into(),
                        )
                        .unwrap();
                }
                let new_selected_track_guid =
                    self.core.get_guid_for_hw_channel(select_msg.idx as usize);
                ModeState {
                    mode: curr_mode.mode,
                    state: State::RequestingModeTransition,
                    new_selected_track_guid,
                }
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
                    self.to_reaper
                        .send(
                            track::Pan {
                                track_guid: guid,
                                pan: new_pan,
                            }
                            .into(),
                        )
                        .unwrap();

                    // Send encoder LED update downstream to hardware
                    self.to_v1m
                        .send(DownstreamMsg::EncoderRingLED(EncoderRingMsg {
                            idx: encoder_msg.idx,
                            mode: v1m::EncoderRingMode::FromCenter,
                            val: map_to_0xb(new_pan),
                        }))
                        .unwrap();
                }
                curr_mode
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
                    self.to_reaper
                        .send(
                            track::Pan {
                                track_guid: guid,
                                pan: new_pan,
                            }
                            .into(),
                        )
                        .unwrap();

                    // Send encoder LED update downstream to hardware
                    self.to_v1m
                        .send(DownstreamMsg::EncoderRingLED(EncoderRingMsg {
                            idx: encoder_msg.idx,
                            mode: v1m::EncoderRingMode::FromCenter,
                            val: map_to_0xb(new_pan),
                        }))
                        .unwrap();
                }
                curr_mode
            }
            _ => {
                self.core.handle_message_from_downstream(
                    msg,
                    self.to_reaper.clone(),
                    self.to_v1m.clone(),
                );
                curr_mode
            }
        }
    }
}

impl VolumePanMode {
    pub fn initiate_mode_transition(
        &mut self,
        from_mode: ModeState,
        upstream: Sender<TrackMsg>,
        selected_track_guid: Option<Uuid>,
    ) -> ModeState {
        self.core.reset(self.to_v1m.clone());
        upstream.send(TrackMsg::QueryAll).unwrap();
        let barrier = Barrier::new(from_mode.mode, Mode::ReaperVolPan);
        upstream.send(TrackMsg::Barrier(barrier)).unwrap();
        ModeState {
            mode: Mode::ReaperVolPan,
            state: State::WaitingBarrierFromUpstream(barrier),
            new_selected_track_guid: None,
        }
    }
}
