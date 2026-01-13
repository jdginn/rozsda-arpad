use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::vec::Vec;

use crossbeam_channel::{Receiver, Sender};

use crate::midi::xtouch::{
    EncoderRingLEDMsg, EncoderRingLEDRangePointMsg, FaderAbsMsg, XTouchDownstreamMsg,
    XTouchUpstreamMsg,
};
use crate::modes::mode_manager::{Barrier, Mode, ModeHandler, ModeState, State};
use crate::track::track::{
    DataPayload as TrackDataPayload, Direction, SendLevel, TrackDataMsg, TrackMsg, TrackQuery,
};

#[derive(Clone, Default)]
pub struct TrackSendInfo {
    pub guid: String,
    pub level: f32,
    pub pan: f32,
}

pub struct TrackSendsMode {
    // Maps track send index to send guid
    hw_assignments: Arc<Mutex<Vec<Option<String>>>>,
    // Maps guid to info about the send it designates
    track_send_states: Arc<Mutex<BTreeMap<String, TrackSendInfo>>>,
    selected_track_guid: Option<String>,
    to_reaper: Sender<TrackMsg>,
    from_reaper: Receiver<TrackMsg>,
    to_xtouch: Sender<XTouchDownstreamMsg>,
    from_xtouch: Receiver<XTouchUpstreamMsg>,
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
            from_reaper,
            to_xtouch,
            from_xtouch,
        }
    }

    fn get_guid_for_hw_channel(&self, hw_channel: usize) -> Option<String> {
        let assignments = self.hw_assignments.lock().unwrap();
        assignments[hw_channel].clone()
    }

    fn find_hw_channel_for_guid(guid: &str, assignments: Vec<Option<String>>) -> Option<usize> {
        for (hw_channel, assigned_guid) in assignments.iter().enumerate() {
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
    fn handle_downstream_messages(&mut self, msg: TrackMsg, curr_mode: ModeState) -> ModeState {
        if let TrackMsg::Barrier(barrier) = msg {
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
        if let TrackMsg::TrackDataMsg(msg) = msg {
            match msg.data {
                TrackDataPayload::SendIndex(msg) => {
                    let mut assignments = self.hw_assignments.lock().unwrap();

                    if let Some(index) =
                        TrackSendsMode::find_hw_channel_for_guid(&msg.guid, assignments.to_vec())
                    {
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
                        assignments[msg.send_index as usize] = Some(msg.guid.clone());
                    }
                    // Insert default state into self.track_send_states if not already present
                    let state = self
                        .track_send_states
                        .lock()
                        .unwrap()
                        .entry(msg.guid.clone())
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
                            EncoderRingLEDMsg::RangePoint(EncoderRingLEDRangePointMsg {
                                idx: msg.send_index,
                                pos: (state.pan + 1.0) / 2.0, // Scale -1.0 to 1.0 into 0.0 to 1.0
                            }),
                        ))
                        .unwrap();
                }
                TrackDataPayload::SendLevel(msg) => {
                    // Only send fader update if the send index is mapped to a target
                    let assignments = self.hw_assignments.lock().unwrap();
                    if let Some(Some(guid)) = assignments.get(msg.send_index as usize) {
                        self.track_send_states
                            .lock()
                            .unwrap()
                            .entry(guid.clone())
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
                TrackDataPayload::SendPan(msg) => {
                    // Only send encoder update if the send index is mapped to a target
                    let assignments = self.hw_assignments.lock().unwrap();
                    if let Some(Some(guid)) = assignments.get(msg.send_index as usize) {
                        self.track_send_states
                            .lock()
                            .unwrap()
                            .entry(guid.clone())
                            .or_default()
                            .pan = msg.pan;

                        let encoder_pos = (msg.pan + 1.0) / 2.0; // Scale -1.0 to 1.0 into 0.0 to 1.0
                        self.to_xtouch
                            .send(XTouchDownstreamMsg::EncoderRingLED(
                                EncoderRingLEDMsg::RangePoint(EncoderRingLEDRangePointMsg {
                                    idx: msg.send_index,
                                    pos: encoder_pos,
                                }),
                            ))
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
        curr_mode
    }

    fn handle_upstream_messages(
        &mut self,
        msg: XTouchUpstreamMsg,
        curr_mode: ModeState,
    ) -> ModeState {
        match msg {
            // If we were already waiting on a barrier from downstream, check if this is the one
            // we were waiting for. If yes, the state transition is finished.
            //
            // Note, we do not need to forward this barrier onward, since the hardware is not
            // allowed to reflect barriers back upstream.
            XTouchUpstreamMsg::Barrier(barrier) => {
                match curr_mode.state {
                    State::WaitingBarrierFromDownstream(expected_barrier) => {
                        if barrier == expected_barrier {
                            ModeState {
                                mode: curr_mode.mode,
                                state: State::Active,
                            }
                        } else {
                            curr_mode
                        }
                    }
                    _ => {
                        // TODO: This is a barrier message we don't care about. Do we need to do
                        // anything with it?
                        //
                        // Presumably if a barrier comes back that we weren't looking for, it's for
                        // some old irrelevant state transition that has already been superseded.
                        curr_mode
                    }
                }
                // Handle barrier messages if needed
            }
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
                        .send(TrackMsg::TrackDataMsg(TrackDataMsg {
                            direction: Direction::Upstream,
                            guid,
                            data: TrackDataPayload::SendLevel(SendLevel {
                                send_index: fader_msg.idx,
                                level: fader_msg.value as f32, // TODO: scale appropriately
                            }),
                        }))
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
        upstream: Sender<TrackMsg>,
        selected_track_guid: &str,
    ) -> ModeState {
        self.selected_track_guid = Some(selected_track_guid.to_string());
        upstream
            .send(TrackMsg::TrackQuery(TrackQuery {
                direction: Direction::Downstream,
                guid: selected_track_guid.to_string(),
            }))
            .unwrap();
        let barrier = Barrier::new();
        upstream.send(TrackMsg::Barrier(barrier)).unwrap();
        ModeState {
            mode: Mode::ReaperSends,
            state: State::WaitingBarrierFromDownstream(barrier),
        }
    }
}
