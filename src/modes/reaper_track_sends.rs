use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::vec::Vec;

use crossbeam_channel::{Receiver, Sender};
use uuid::Uuid;

use crate::midi::xtouch::{
    EncoderRingMode, EncoderRingMsg, FaderAbsMsg, XTouchDownstreamMsg, XTouchUpstreamMsg,
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
    selected_track_guid: Option<String>,
    to_reaper: Sender<TrackMsg>,
    _from_reaper: Receiver<TrackMsg>,
    to_xtouch: Sender<XTouchDownstreamMsg>,
    _from_xtouch: Receiver<XTouchUpstreamMsg>,
}

impl TrackSendsMode {
    pub fn new(
        num_channels: usize,
        from_reaper: Receiver<TrackMsg>,
        to_reaper: Sender<TrackMsg>,
        from_xtouch: Receiver<XTouchUpstreamMsg>,
        to_xtouch: Sender<XTouchDownstreamMsg>,
    ) -> Self {
        TrackSendsMode {
            hw_assignments: Arc::new(Mutex::new(vec![None; num_channels])),
            track_send_states: Arc::new(Mutex::new(BTreeMap::new())),
            selected_track_guid: None,
            to_reaper,
            _from_reaper: from_reaper,
            to_xtouch,
            _from_xtouch: from_xtouch,
        }
    }

    fn get_guid_for_hw_channel(&self, hw_channel: usize) -> Option<Uuid> {
        let assignments = self.hw_assignments.lock().unwrap();
        assignments[hw_channel]
    }

    fn find_hw_channel_for_guid(guid: Uuid, assignments: Vec<Option<Uuid>>) -> Option<usize> {
        for (hw_channel, &assigned_guid) in assignments.iter().enumerate() {
            if let Some(assigned_guid) = assigned_guid {
                if assigned_guid == guid {
                    return Some(hw_channel);
                }
            }
        }
        None
    }
}

impl ModeHandler<TrackMsg, TrackMsg, XTouchDownstreamMsg, XTouchUpstreamMsg> for TrackSendsMode {
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
                            return ModeState {
                                mode: curr_mode.mode,
                                state: State::WaitingBarrierFromDownstream(barrier),
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
                    track::DataMsg::SendIndex(msg) => {
                        let mut assignments = self.hw_assignments.lock().unwrap();

                        if let Some(index) = TrackSendsMode::find_hw_channel_for_guid(
                            msg.send_guid,
                            assignments.to_vec(),
                        ) {
                            if index as i32 == msg.send_index {
                                // No change, skip
                                return curr_mode;
                            }
                            // Clear previous assignment
                            //
                            // TODO: are we sure this is the correct behavior?
                            assignments[index] = None;
                        }
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
                        self.to_xtouch
                            .send(XTouchDownstreamMsg::FaderAbs(FaderAbsMsg {
                                idx: msg.send_index,
                                value: state.level as f64, // TODO: scale appropriately
                            }))
                            .unwrap();
                        self.to_xtouch
                            .send(XTouchDownstreamMsg::EncoderRingLED(
                                // EncoderRingMsg::RangePoint(EncoderRingLEDRangePointMsg {
                                //     idx: msg.send_index,
                                //     pos: (state.pan + 1.0) / 2.0, // Scale -1.0 to 1.0 into 0.0 to 1.0
                                // }),
                                EncoderRingMsg {
                                    idx: msg.send_index,
                                    mode: EncoderRingMode::Point,
                                    val: map_to_0xb(state.pan),
                                },
                            ))
                            .unwrap();
                    }
                    track::DataMsg::SendLevel(msg) => {
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
                            self.to_xtouch
                                .send(XTouchDownstreamMsg::FaderAbs(FaderAbsMsg {
                                    idx: msg.send_index,
                                    value: fader_value as f64,
                                }))
                                .unwrap();
                        }
                    }
                    track::DataMsg::SendPan(msg) => {
                        // Only send encoder update if the send index is mapped to a target
                        let assignments = self.hw_assignments.lock().unwrap();
                        if let Some(Some(guid)) = assignments.get(msg.send_index as usize) {
                            self.track_send_states
                                .lock()
                                .unwrap()
                                .entry(*guid)
                                .or_default()
                                .pan = msg.pan;

                            self.to_xtouch
                                .send(XTouchDownstreamMsg::EncoderRingLED(EncoderRingMsg {
                                    idx: msg.send_index,
                                    mode: EncoderRingMode::Point,
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
        msg: XTouchUpstreamMsg,
        curr_mode: ModeState,
    ) -> ModeState {
        match msg {
            XTouchUpstreamMsg::GlobalPress => {
                // Request transition to ReaperVolPan mode
                ModeState {
                    mode: Mode::ReaperVolPan,
                    state: State::RequestingModeTransition,
                }
            }
            XTouchUpstreamMsg::MIDITracksPress => curr_mode, //MIDITracksPress maps to this mode!
            XTouchUpstreamMsg::FaderAbs(fader_msg) => {
                if let Some(guid) = self.get_guid_for_hw_channel(fader_msg.idx as usize) {
                    self.to_reaper
                        .send(
                            track::SendLevel {
                                track_guid: guid,
                                send_index: fader_msg.idx,
                                level: fader_msg.value as f32, // TODO: scale appropriately
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
        self.selected_track_guid = Some(selected_track_guid.to_string());
        upstream
            .send(TrackMsg::Query(TrackQuery {
                guid: selected_track_guid,
            }))
            .unwrap();
        let barrier = Barrier::new(from_mode, Mode::ReaperSends);
        upstream.send(TrackMsg::Barrier(barrier)).unwrap();
        ModeState {
            mode: Mode::ReaperSends,
            state: State::WaitingBarrierFromUpstream(barrier),
        }
    }
}
