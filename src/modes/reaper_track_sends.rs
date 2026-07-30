use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::vec::Vec;

use crossbeam_channel::{Receiver, Sender};
use uuid::Uuid;

use crate::midi::v1m;
use crate::midi::v1m::{
    ChannelFaderMsg, DownstreamMsg, EncoderRingMode, EncoderRingMsg, UpstreamMsg,
};
use crate::modes::mode_manager::{Barrier, Mode, ModeHandler, ModeState, State};
use crate::track::track;
use crate::track::track::{TrackMsg, TrackQuery};

pub fn map_to_0xb(x: f32) -> u8 {
    let clamped = x.clamp(-1.0, 1.0) as f64;
    ((clamped + 1.0) * 0.5 * 0xb as f64).round() as u8
}

#[derive(Clone, Default)]
pub struct TrackSendInfo {
    pub guid: String,
    pub level: f32,
    pub pan: f32,
}

pub struct TrackSendsMode {
    // Maps track send index to send guid
    hw_assignments: Arc<Mutex<Vec<Option<Uuid>>>>,
    // Maps guid to info about the send it designates
    track_send_states: Arc<Mutex<BTreeMap<Uuid, TrackSendInfo>>>,
    selected_track_guid: Option<Uuid>,
    //a mode transition!
    to_reaper: Sender<TrackMsg>,
    _from_reaper: Receiver<TrackMsg>,
    to_v1m: Sender<DownstreamMsg>,
    _from_v1m: Receiver<UpstreamMsg>,
}

impl TrackSendsMode {
    pub fn new(
        num_channels: usize,
        from_reaper: Receiver<TrackMsg>,
        to_reaper: Sender<TrackMsg>,
        from_v1m: Receiver<UpstreamMsg>,
        to_v1m: Sender<DownstreamMsg>,
    ) -> Self {
        TrackSendsMode {
            hw_assignments: Arc::new(Mutex::new(vec![None; num_channels])),
            track_send_states: Arc::new(Mutex::new(BTreeMap::new())),
            selected_track_guid: None,
            to_reaper,
            _from_reaper: from_reaper,
            to_v1m,
            _from_v1m: from_v1m,
        }
    }

    pub fn reset(&mut self, to_v1m: Sender<DownstreamMsg>) {
        self.track_send_states.lock().unwrap().clear();
        let mut assignments = self.hw_assignments.lock().unwrap();
        for slot in assignments.iter_mut() {
            *slot = None;
        }
        for i in 0..assignments.len() {
            // Zero faders
            to_v1m
                .send(v1m::DownstreamMsg::ChannelFader(v1m::ChannelFaderMsg {
                    idx: i as i32,
                    value: 0.0,
                }))
                .unwrap();
            // Zero encoder LEDs
            to_v1m
                .send(v1m::DownstreamMsg::EncoderRingLED(v1m::EncoderRingMsg {
                    idx: i as i32,
                    mode: v1m::EncoderRingMode::Point,
                    val: 0x06,
                }))
                .unwrap();
            // Turn off Mute/Solo/Arm LEDs
            to_v1m
                .send(v1m::DownstreamMsg::MuteLED(v1m::MuteLEDMsg {
                    idx: i as i32,
                    state: v1m::LEDState::Off,
                }))
                .unwrap();
            to_v1m
                .send(v1m::DownstreamMsg::SoloLED(v1m::SoloLEDMsg {
                    idx: i as i32,
                    state: v1m::LEDState::Off,
                }))
                .unwrap();
            to_v1m
                .send(v1m::DownstreamMsg::ArmLED(v1m::ArmLEDMsg {
                    idx: i as i32,
                    state: v1m::LEDState::Off,
                }))
                .unwrap();
            // Clear scribble strip text and colors
            to_v1m
                .send(v1m::DownstreamMsg::ScribbleStripLine1Text(
                    v1m::ScribbleStripLine1TextMsg {
                        idx: i as i32,
                        text: String::new(),
                    },
                ))
                .unwrap();
            to_v1m
                .send(v1m::DownstreamMsg::ScribbleStripLine2Text(
                    v1m::ScribbleStripLine2TextMsg {
                        idx: i as i32,
                        text: String::new(),
                    },
                ))
                .unwrap();
            to_v1m
                .send(v1m::DownstreamMsg::ScribbleStripBackgroundColor(
                    v1m::ScribbleStripBackgroundColorMsg {
                        idx: i as i32,
                        color: v1m::Color { r: 0, g: 0, b: 0 },
                    },
                ))
                .unwrap();
            to_v1m
                .send(v1m::DownstreamMsg::BottomScribbleStripLine1Text(
                    v1m::BottomScribbleStripLine1TextMsg {
                        idx: i as i32,
                        text: String::new(),
                    },
                ))
                .unwrap();
            to_v1m
                .send(v1m::DownstreamMsg::BottomScribbleStripLine2Text(
                    v1m::BottomScribbleStripLine2TextMsg {
                        idx: i as i32,
                        text: String::new(),
                    },
                ))
                .unwrap();
        }
    }

    fn get_guid_for_hw_channel(&self, hw_channel: usize) -> Option<Uuid> {
        let assignments = self.hw_assignments.lock().unwrap();
        assignments[hw_channel]
    }

    fn find_hw_channel_for_guid(&self, guid: Uuid) -> Option<usize> {
        for (hw_channel, &assigned_guid) in self.hw_assignments.lock().unwrap().iter().enumerate() {
            if let Some(assigned_guid) = assigned_guid {
                if assigned_guid == guid {
                    return Some(hw_channel);
                }
            }
        }
        None
    }
}

impl ModeHandler<TrackMsg, TrackMsg, DownstreamMsg, UpstreamMsg> for TrackSendsMode {
    fn handle_messages_from_upstream(&mut self, msg: TrackMsg, curr_mode: ModeState) -> ModeState {
        match track::DataMsg::try_from(msg) {
            Err(TrackMsg::Barrier(barrier)) => {
                // Forward barriers downstream (they need to reflect back upstream for the mode to
                // transition)
                self.to_v1m.send(DownstreamMsg::Barrier(barrier)).unwrap();
                match curr_mode.state {
                    // If we were already waiting on a barrier from upstream, check if this is the one
                    // we were waiting for. If yes, transition to waiting for the barrier to reflect back up from downstream.
                    State::WaitingBarrierFromUpstream(expected_barrier) => {
                        if barrier == expected_barrier {
                            return ModeState {
                                mode: curr_mode.mode,
                                state: State::WaitingBarrierFromDownstream(barrier),
                                new_selected_track_guid: None,
                            };
                        } else {
                            return curr_mode;
                        }
                    }
                    _ => return curr_mode,
                }
            }
            Ok(msg) => {
                match msg {
                    track::DataMsg::Name(msg) => {
                        self.to_v1m
                            .send(
                                v1m::ScribbleStripLine1TextMsg {
                                    idx: self.find_hw_channel_for_guid(msg.track_guid).unwrap_or(0)
                                        as i32,
                                    text: msg.name.clone(),
                                }
                                .into(),
                            )
                            .unwrap();
                        self.to_v1m
                            .send(
                                v1m::BottomScribbleStripLine2TextMsg {
                                    idx: self.find_hw_channel_for_guid(msg.track_guid).unwrap_or(0)
                                        as i32,
                                    text: msg.name,
                                }
                                .into(),
                            )
                            .unwrap();
                    }
                    // If a new track is selected, we need to initiate a mode transition so that we
                    // are controlling sends for that new track
                    track::DataMsg::Selected(msg) => {
                        // FIXME
                        // - There is some kind of double selection or initiation behavior at least with TrackSendsMode
                        // - Send naming has an off by one error (should say 2 but says 3)
                        // - Scribble strip messages are jittery
                        // - Still some fader jitter to sort through

                        if msg.selected {
                            if self.selected_track_guid != Some(msg.track_guid) {
                                return ModeState {
                                    mode: curr_mode.mode,
                                    state: State::RequestingModeTransition,
                                    new_selected_track_guid: Some(msg.track_guid),
                                };
                            }
                            self.selected_track_guid = Some(msg.track_guid);
                            let state = match msg.selected {
                                true => v1m::LEDState::On,
                                false => v1m::LEDState::Off,
                            };
                            self.to_v1m
                                .send(
                                    v1m::SelectLEDMsg {
                                        idx: self
                                            .find_hw_channel_for_guid(msg.track_guid)
                                            .unwrap_or(0)
                                            as i32,
                                        state,
                                    }
                                    .into(),
                                )
                                .unwrap();
                            return curr_mode;
                        }
                    }
                    track::DataMsg::SendIndex(msg) => {
                        if msg.track_guid == self.selected_track_guid.unwrap_or_default() {
                            // Only process send index messages for the currently selected track
                        } else {
                            return curr_mode;
                        }
                        let mut assignments = self.hw_assignments.lock().unwrap();

                        assignments[msg.send_index as usize] = Some(msg.send_guid);

                        // if let Some(index) = TrackSendsMode::find_hw_channel_for_guid(
                        //     msg.send_guid,
                        //     assignments.to_vec(),
                        // ) {
                        //     if index as i32 == msg.send_index {
                        //         // No change, skip
                        //         return curr_mode;
                        //     }
                        //     // Clear previous assignment
                        //     //
                        //     // TODO: are we sure this is the correct behavior?
                        //     assignments[index] = None;
                        // }
                        // Add bounds checking to prevent panic on invalid send_index
                        // If out of bounds, silently ignore (could log error in production)
                        if (msg.send_index as usize) < assignments.len() {
                            assignments[msg.send_index as usize] = Some(msg.send_guid);
                        }
                        // Insert default state into self.track_send_states if not already present
                        let state = self
                            .track_send_states
                            .lock()
                            .unwrap()
                            .entry(msg.send_guid)
                            .or_default()
                            .clone();
                        // Send current state to hardware for this send index
                        self.to_v1m
                            .send(DownstreamMsg::ChannelFader(ChannelFaderMsg {
                                idx: msg.send_index,
                                value: state.level as f64, // TODO: scale appropriately
                            }))
                            .unwrap();
                        self.to_v1m
                            .send(DownstreamMsg::EncoderRingLED(
                                // EncoderRingMsg::RangePoint(EncoderRingLEDRangePointMsg {
                                //     idx: msg.send_index,
                                //     pos: (state.pan + 1.0) / 2.0, // Scale -1.0 to 1.0 into 0.0 to 1.0
                                // }),
                                EncoderRingMsg {
                                    idx: msg.send_index,
                                    mode: EncoderRingMode::FromCenter,
                                    val: map_to_0xb(state.pan),
                                },
                            ))
                            .unwrap();
                    }
                    track::DataMsg::SendLevel(msg) => {
                        if msg.track_guid == self.selected_track_guid.unwrap_or_default() {
                            // Only process send index messages for the currently selected track
                        } else {
                            return curr_mode;
                        }
                        // Only send fader update if the send index is mapped to a target
                        let assignments = self.hw_assignments.lock().unwrap();
                        if let Some(Some(guid)) = assignments.get(msg.send_index as usize) {
                            self.track_send_states
                                .lock()
                                .unwrap()
                                .entry(*guid)
                                .or_default()
                                .level = msg.level;

                            let fader_value = msg.level; // TODO: scale appropriately
                            self.to_v1m
                                .send(DownstreamMsg::ChannelFader(ChannelFaderMsg {
                                    idx: msg.send_index,
                                    value: fader_value as f64,
                                }))
                                .unwrap();
                        }
                    }
                    track::DataMsg::SendPan(msg) => {
                        if msg.track_guid == self.selected_track_guid.unwrap_or_default() {
                            // Only process send index messages for the currently selected track
                        } else {
                            return curr_mode;
                        }
                        // Only send encoder update if the send index is mapped to a target
                        let assignments = self.hw_assignments.lock().unwrap();
                        if let Some(Some(guid)) = assignments.get(msg.send_index as usize) {
                            self.track_send_states
                                .lock()
                                .unwrap()
                                .entry(*guid)
                                .or_default()
                                .pan = msg.pan;

                            self.to_v1m
                                .send(DownstreamMsg::EncoderRingLED(EncoderRingMsg {
                                    idx: msg.send_index,
                                    mode: EncoderRingMode::FromCenter,
                                    val: map_to_0xb(msg.pan),
                                }))
                                .unwrap();
                        }
                    }
                    // TODO: pan
                    _ => {
                        // Ignore unhandled payloads
                        return curr_mode;
                    }
                }
            }
            Err(_) => {
                // Ignore unhandled messages
                return curr_mode;
            }
        }

        curr_mode
    }

    fn handle_messages_from_downstream(
        &mut self,
        msg: UpstreamMsg,
        curr_mode: ModeState,
    ) -> ModeState {
        match msg {
            UpstreamMsg::GlobalPress => {
                // Request transition to ReaperVolPan mode
                ModeState {
                    mode: Mode::ReaperVolPan,
                    state: State::RequestingModeTransition,
                    new_selected_track_guid: None,
                }
            }
            UpstreamMsg::MIDITracksPress => curr_mode, //MIDITracksPress maps to this mode!
            UpstreamMsg::InputsPress => {
                // Request transition to ReaperChannelStrip mode
                ModeState {
                    mode: Mode::ReaperChannelStrip,
                    state: State::RequestingModeTransition,
                    new_selected_track_guid: None,
                }
            }
            // If a new track is selected, we need to initiate a mode transition so that the
            // widgets are controlling the new track
            //
            // TODO: do we need to handle this case separately or do we simply expect a reflected
            // message back from Reaper?
            UpstreamMsg::SelectPress(msg) => {
                self.selected_track_guid = self.get_guid_for_hw_channel(msg.idx as usize);
                if let Some(guid) = self.get_guid_for_hw_channel(msg.idx as usize) {
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
                ModeState {
                    mode: Mode::ReaperSends,
                    state: State::RequestingModeTransition,
                    new_selected_track_guid: self.selected_track_guid,
                }
            }
            UpstreamMsg::ChannelFader(fader_msg) => {
                // FIXME: seems like this is the issuer here V
                // No?
                if let Some(guid) = self.get_guid_for_hw_channel(fader_msg.idx as usize) {
                    self.to_reaper
                        .send(
                            track::SendLevel {
                                track_guid: guid,
                                send_index: fader_msg.idx,
                                level: fader_msg.value as f32,
                            }
                            .into(),
                        )
                        .unwrap();
                }
                curr_mode
            }
            _ => curr_mode, // For now, the buttons and encoder do nothing
        }
    }
}

impl TrackSendsMode {
    pub fn initiate_mode_transition(
        &mut self,
        from_mode: Mode,
        upstream: Sender<TrackMsg>,
        selected_track_guid: Uuid,
    ) -> ModeState {
        println!(
            "TrackSendsMode: initiating mode transition from {:?} to ReaperSends for track {:?}",
            from_mode, selected_track_guid
        );
        self.reset(self.to_v1m.clone());
        self.selected_track_guid = Some(selected_track_guid);
        upstream.send(TrackMsg::QueryAll);
        let barrier = Barrier::new(from_mode, Mode::ReaperSends);
        upstream.send(TrackMsg::Barrier(barrier)).unwrap();

        ModeState {
            mode: Mode::ReaperSends,
            state: State::WaitingBarrierFromUpstream(barrier),
            new_selected_track_guid: None,
        }
    }
}
