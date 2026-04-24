use std::collections::HashMap;

use crossbeam_channel::{Receiver, Sender};
use uuid::Uuid;

use crate::midi::xtouch::{self, EncoderRingLEDRangePointMsg};
use crate::midi::xtouch::{XTouchDownstreamMsg, XTouchUpstreamMsg};
use crate::modes::mode_manager::{Barrier, Mode, ModeHandler, ModeState, State};
use crate::modes::reaper_faders_buttons_core::VolumeFadersCore;
use crate::track::track;
use crate::track::track::{TrackMsg, TrackQuery};

pub const FADER_0DB: f32 = 0.72; // Placeholder value for 0dB on fader scale

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
    to_xtouch: Sender<XTouchDownstreamMsg>,
    _from_xtouch: Receiver<XTouchUpstreamMsg>,
    pan_states: HashMap<Uuid, f32>,
}

impl VolumePanMode {
    pub fn new(
        num_channels: usize,
        from_reaper: Receiver<TrackMsg>,
        to_reaper: Sender<TrackMsg>,
        from_xtouch: Receiver<XTouchUpstreamMsg>,
        to_xtouch: Sender<XTouchDownstreamMsg>,
    ) -> Self {
        VolumePanMode {
            core: VolumeFadersCore::new(num_channels),
            pan_states: HashMap::new(),
            to_reaper,
            _from_reaper: from_reaper,
            to_xtouch,
            _from_xtouch: from_xtouch,
        }
    }

    pub fn find_hw_channel(&self, guid: Uuid) -> Option<usize> {
        self.core.find_hw_channel(guid)
    }
}

impl ModeHandler<TrackMsg, TrackMsg, XTouchDownstreamMsg, XTouchUpstreamMsg> for VolumePanMode {
    fn handle_messages_from_upstream(&mut self, msg: TrackMsg, curr_mode: ModeState) -> ModeState {
        match track::DataMsg::try_from(msg) {
            Err(TrackMsg::Barrier(barrier)) => {
                // Forward barriers downstream (they need to reflect back upstream for the mode to
                // transition)
                self.to_xtouch
                    .send(XTouchDownstreamMsg::Barrier(barrier))
                    .unwrap();
                match curr_mode.state {
                    // If we were already waiting on a barrier from upstream, check if this is the one
                    // we were waiting for. If yes, transition to waiting for the barrier to reflect back up from downstream.
                    State::WaitingBarrierFromUpstream(expected_barrier) => {
                        if barrier == expected_barrier {
                            ModeState {
                                mode: curr_mode.mode,
                                state: State::WaitingBarrierFromDownstream(barrier),
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
                    track::DataMsg::Pan(msg) => {
                        self.pan_states.insert(msg.track_guid, msg.pan);
                        if let Some(hw_channel) = self.core.find_hw_channel(msg.track_guid) {
                            // Send pan update to XTouch for the corresponding encoder
                            let pan_value = msg.pan; // TODO: scale appropriately
                            let _ = self.to_xtouch.send(
                                xtouch::EncoderRingLEDMsg::RangePoint(
                                    EncoderRingLEDRangePointMsg {
                                        idx: hw_channel as i32,
                                        pos: pan_value,
                                    },
                                )
                                .into(),
                            );
                        }
                        curr_mode
                    }
                    _ => {
                        // Handle common functionality across modes that put volume on the faders
                        // and mute/solo/arm on the buttons
                        self.core.handle_message_from_upstream(
                            msg,
                            self.to_xtouch.clone(),
                            |data| {
                                println!("Inside epilogue");
                                let pan_val = self.pan_states.entry(data.track_guid).or_insert(0.5); // Default center pan
                                println!("Sending");
                                self.to_xtouch
                                    .send(
                                        xtouch::EncoderRingLEDMsg::RangePoint(
                                            EncoderRingLEDRangePointMsg {
                                                idx: data.hw_channel as i32,
                                                pos: *pan_val, // TODO: scale appropriately
                                            },
                                        )
                                        .into(),
                                    )
                                    .unwrap();
                            },
                        );
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
        msg: XTouchUpstreamMsg,
        curr_mode: ModeState,
    ) -> ModeState {
        match msg {
            // GlobalPress maps to this mode!
            XTouchUpstreamMsg::GlobalPress => curr_mode,
            // MIDITracksPress maps to ReaperSends mode
            XTouchUpstreamMsg::MIDITracksPress => {
                // Request transition to ReaperSends mode
                ModeState {
                    mode: Mode::ReaperSends,
                    state: State::RequestingModeTransition,
                }
            }
            XTouchUpstreamMsg::EncoderTurnInc(encoder_msg) => {
                if let Some(guid) = self.core.get_guid_for_hw_channel(encoder_msg.idx as usize) {
                    // Get current pan value and increment it
                    let current_pan = self.pan_states.entry(guid).or_insert(0.5); // Default center pan
                    let new_pan = (*current_pan + 0.05).min(1.0); // Clamp to max 1.0
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
                    self.to_xtouch
                        .send(XTouchDownstreamMsg::EncoderRingLED(
                            xtouch::EncoderRingLEDMsg::RangePoint(EncoderRingLEDRangePointMsg {
                                idx: encoder_msg.idx,
                                pos: new_pan,
                            }),
                        ))
                        .unwrap();
                }
                curr_mode
            }
            XTouchUpstreamMsg::EncoderTurnDec(encoder_msg) => {
                if let Some(guid) = self.core.get_guid_for_hw_channel(encoder_msg.idx as usize) {
                    // Get current pan value and decrement it
                    let current_pan = self.pan_states.entry(guid).or_insert(0.5); // Default center pan
                    let new_pan = (*current_pan + 0.05).min(1.0); // Clamp to max 1.0
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
                    self.to_xtouch
                        .send(XTouchDownstreamMsg::EncoderRingLED(
                            xtouch::EncoderRingLEDMsg::RangePoint(EncoderRingLEDRangePointMsg {
                                idx: encoder_msg.idx,
                                pos: new_pan,
                            }),
                        ))
                        .unwrap();
                }
                curr_mode
            }
            _ => {
                self.core.handle_message_from_downstream(
                    msg,
                    self.to_reaper.clone(),
                    self.to_xtouch.clone(),
                );
                curr_mode
            }
        }
    }
}

impl VolumePanMode {
    pub fn initiate_mode_transition(
        &mut self,
        from_mode: Mode,
        upstream: Sender<TrackMsg>,
    ) -> ModeState {
        self.core
            .track_hw_assignments
            .lock()
            .unwrap()
            .iter()
            .for_each(|assignment| {
                if let Some(guid) = assignment {
                    // Request track data from Reaper for each assigned track
                    let _ = self
                        .to_reaper
                        .send(TrackMsg::Query(TrackQuery { guid: *guid }));
                }
            });
        let barrier = Barrier::new(from_mode, Mode::ReaperVolPan);
        upstream.send(TrackMsg::Barrier(barrier)).unwrap();
        ModeState {
            mode: Mode::ReaperVolPan,
            state: State::WaitingBarrierFromUpstream(barrier),
        }
    }
}
