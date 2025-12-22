use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::vec::Vec;

use crossbeam_channel::{Receiver, Sender};

use crate::midi::xtouch;
use crate::midi::xtouch::{FaderAbsMsg, LEDState, XTouchDownstreamMsg, XTouchUpstreamMsg};
use crate::modes::mode_manager::{Barrier, Mode, ModeHandler, ModeState, State};
use crate::modes::reaper_vol_pan::VolumePanMode;
use crate::track::track::{
    DataPayload as TrackDataPayload, Direction, SendLevel, TrackDataMsg, TrackMsg, TrackQuery,
};

struct Button {
    state: bool,
}

impl Button {
    fn new() -> Self {
        Button { state: false }
    }

    fn is_on(&self) -> bool {
        self.state
    }

    fn set(&mut self, new_state: bool) {
        self.state = new_state;
    }

    fn toggle(&mut self) -> bool {
        self.state = !self.state;
        self.state
    }
}

pub struct TrackSendState {}

pub struct TrackSendsMode {
    // Maps track send index to trackg guid
    track_sends: Arc<Mutex<Vec<Option<String>>>>,
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
            track_sends: Arc::new(Mutex::new(vec![None; num_channels])),
            selected_track_guid: None,
            to_reaper,
            from_reaper,
            to_xtouch,
            from_xtouch,
        }
    }

    fn get_guid_for_hw_channel(&self, hw_channel: usize) -> Option<String> {
        let assignments = self.track_sends.lock().unwrap();
        assignments[hw_channel].clone()
    }

    fn find_hw_channel_for_guid(&self, guid: &str) -> Option<usize> {
        let assignments = self.track_sends.lock().unwrap();
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
                    let mut assignments = self.track_sends.lock().unwrap();
                    assignments[msg.send_index as usize] = Some(msg.guid);
                }
                TrackDataPayload::SendLevel(msg) => {
                    let fader_value = msg.level; // TODO: scale appropriately
                    self.to_xtouch
                        .send(XTouchDownstreamMsg::FaderAbs(FaderAbsMsg {
                            idx: msg.send_index,
                            value: fader_value as f64,
                        }));
                }
                // TODO: pan
                _ => panic!("Unhandled track data payload in VolumePanMode"),
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
