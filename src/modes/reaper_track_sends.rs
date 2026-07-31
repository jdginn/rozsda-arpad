use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::vec::Vec;

use uuid::Uuid;

use crate::midi::v1m;
use crate::midi::v1m::{
    ChannelFaderMsg, DownstreamMsg, EncoderRingMode, EncoderRingMsg, UpstreamMsg,
};
use crate::modes::mode_manager::{Mode, ModeAction, ModeHandler, Senders};
use crate::track::track;
use crate::track::track::TrackMsg;

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
}

impl TrackSendsMode {
    pub fn new(num_channels: usize) -> Self {
        TrackSendsMode {
            hw_assignments: Arc::new(Mutex::new(vec![None; num_channels])),
            track_send_states: Arc::new(Mutex::new(BTreeMap::new())),
            selected_track_guid: None,
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

impl ModeHandler for TrackSendsMode {
    fn handle_msg_from_upstream(&mut self, msg: TrackMsg, senders: &Senders) -> ModeAction {
        match track::DataMsg::try_from(msg) {
            Ok(msg) => {
                match msg {
                    track::DataMsg::Name(msg) => {
                        senders.send_to_v1m(
                            v1m::ScribbleStripLine1TextMsg {
                                idx: self.find_hw_channel_for_guid(msg.track_guid).unwrap_or(0)
                                    as i32,
                                text: msg.name.clone(),
                            }
                            .into(),
                        );
                        senders.send_to_v1m(
                            v1m::BottomScribbleStripLine2TextMsg {
                                idx: self.find_hw_channel_for_guid(msg.track_guid).unwrap_or(0)
                                    as i32,
                                text: msg.name,
                            }
                            .into(),
                        );
                        ModeAction::None
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
                                return ModeAction::SelectedTrackChanged(Some(msg.track_guid));
                            }
                            self.selected_track_guid = Some(msg.track_guid);
                            let state = match msg.selected {
                                true => v1m::LEDState::On,
                                false => v1m::LEDState::Off,
                            };
                            senders.send_to_v1m(
                                v1m::SelectLEDMsg {
                                    idx: self.find_hw_channel_for_guid(msg.track_guid).unwrap_or(0)
                                        as i32,
                                    state,
                                }
                                .into(),
                            );
                        }
                        ModeAction::None
                    }
                    track::DataMsg::SendIndex(msg) => {
                        if msg.track_guid == self.selected_track_guid.unwrap_or_default() {
                            // Only process send index messages for the currently selected track
                        } else {
                            return ModeAction::None;
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
                        senders.send_to_v1m(DownstreamMsg::ChannelFader(ChannelFaderMsg {
                            idx: msg.send_index,
                            value: state.level as f64, // TODO: scale appropriately
                        }));
                        senders.send_to_v1m(DownstreamMsg::EncoderRingLED(
                            // EncoderRingMsg::RangePoint(EncoderRingLEDRangePointMsg {
                            //     idx: msg.send_index,
                            //     pos: (state.pan + 1.0) / 2.0, // Scale -1.0 to 1.0 into 0.0 to 1.0
                            // }),
                            EncoderRingMsg {
                                idx: msg.send_index,
                                mode: EncoderRingMode::FromCenter,
                                val: map_to_0xb(state.pan),
                            },
                        ));
                        ModeAction::None
                    }
                    track::DataMsg::SendLevel(msg) => {
                        if msg.track_guid == self.selected_track_guid.unwrap_or_default() {
                            // Only process send index messages for the currently selected track
                        } else {
                            return ModeAction::None;
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
                            senders.send_to_v1m(DownstreamMsg::ChannelFader(ChannelFaderMsg {
                                idx: msg.send_index,
                                value: fader_value as f64,
                            }))
                        }
                        ModeAction::None
                    }
                    track::DataMsg::SendPan(msg) => {
                        if msg.track_guid == self.selected_track_guid.unwrap_or_default() {
                            // Only process send index messages for the currently selected track
                        } else {
                            return ModeAction::None;
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

                            senders.send_to_v1m(DownstreamMsg::EncoderRingLED(EncoderRingMsg {
                                idx: msg.send_index,
                                mode: EncoderRingMode::FromCenter,
                                val: map_to_0xb(msg.pan),
                            }))
                        }
                        ModeAction::None
                    }
                    // TODO: pan
                    _ => {
                        // Ignore unhandled payloads
                        ModeAction::None
                    }
                }
            }
            Err(_) => {
                // Ignore unhandled messages
                panic!("Code bug: VolumePanMode should only receive TrackDataMsg messages.");
            }
        }
    }

    fn handle_msg_from_downstream(&mut self, msg: UpstreamMsg, senders: &Senders) -> ModeAction {
        match msg {
            UpstreamMsg::GlobalPress => ModeAction::Transition(Mode::ReaperVolPan),
            UpstreamMsg::MIDITracksPress => ModeAction::None,
            UpstreamMsg::InputsPress => ModeAction::Transition(Mode::ReaperChannelStrip),
            // If a new track is selected, we need to initiate a mode transition so that the
            // widgets are controlling the new track
            //
            // TODO: do we need to handle this case separately or do we simply expect a reflected
            // message back from Reaper?
            UpstreamMsg::SelectPress(msg) => {
                self.selected_track_guid = self.get_guid_for_hw_channel(msg.idx as usize);
                if let Some(guid) = self.get_guid_for_hw_channel(msg.idx as usize) {
                    senders.send_to_reaper(
                        track::Selected {
                            track_guid: guid,
                            selected: true,
                        }
                        .into(),
                    );
                }
                //FIXME: what if we're transitioning MODE AND SELECTED TRACK AT THE SAME TIME?
                ModeAction::Transition(Mode::ReaperSends)
            }
            UpstreamMsg::ChannelFader(fader_msg) => {
                // FIXME: seems like this is the issuer here V
                // No?
                if let Some(guid) = self.get_guid_for_hw_channel(fader_msg.idx as usize) {
                    senders.send_to_reaper(
                        track::SendLevel {
                            track_guid: guid,
                            send_index: fader_msg.idx,
                            level: fader_msg.value as f32,
                        }
                        .into(),
                    )
                }
                ModeAction::None
            }
            _ => ModeAction::None, // For now, the buttons and encoder do nothing
        }
    }
}
