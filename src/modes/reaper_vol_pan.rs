use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::vec::Vec;

use crossbeam_channel::{Receiver, Sender};

use crate::midi::xtouch::{self, EncoderRingLEDRangePointMsg, EncoderTurnCCW};
use crate::midi::xtouch::{FaderAbsMsg, LEDState, XTouchDownstreamMsg, XTouchUpstreamMsg};
use crate::modes::mode_manager::{Barrier, Mode, ModeHandler, ModeState, State};
use crate::track::track::{
    DataPayload as TrackDataPayload, Direction, TrackDataMsg, TrackMsg, TrackQuery,
};

// Threshold for filtering out insignificant volume/pan changes
const EPSILON: f32 = 0.01;

pub const FADER_0DB: f32 = 0.72; // Placeholder value for 0dB on fader scale

#[derive(Clone)]
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

// Collection of state for the buttons repeated for each channel on the hw controller
//
// TODO: this might be too implementation-specific to live here?
#[derive(Clone)]
struct ButtonState {
    mute: Button,
    solo: Button,
    arm: Button,
}

// Track the current pan value for each track to support encoder inc/dec
#[derive(Clone)]
struct TrackState {
    buttons: ButtonState,
    pan: f32,
    volume: f32,
}

/// Implements a mode where that "basic" reaper functionality is mapped to the channel strips on
/// the control surface, namely:
/// - Volume on faders
/// - Pan on rotary encoders
/// - Select/Mute/Solo/Arm on buttons
///
/// Button LED toggling is handled here (downstream does not need to worry about managing button
/// LEDS.)
pub struct VolumePanMode {
    // Maps each channel on the hardware controller to a Reaper track
    track_hw_assignments: Arc<Mutex<Vec<Option<String>>>>,
    // Store state for each track by track GUID
    track_states: HashMap<String, TrackState>,
    // Store last sent volume/pan values to avoid sending updates for tiny changes
    last_sent_volume: HashMap<String, f32>,
    last_sent_pan: HashMap<String, f32>,
    to_reaper: Sender<TrackMsg>,
    from_reaper: Receiver<TrackMsg>,
    to_xtouch: Sender<XTouchDownstreamMsg>,
    from_xtouch: Receiver<XTouchUpstreamMsg>,
}

impl VolumePanMode {
    pub fn new(
        num_channels: usize,
        from_reaper: Receiver<TrackMsg>,
        to_reaper: Sender<TrackMsg>,
        from_xtouch: Receiver<XTouchUpstreamMsg>,
        to_xtouch: Sender<XTouchDownstreamMsg>,
    ) -> Self {
        let track_hw_assignments = Arc::new(Mutex::new(vec![None; num_channels]));
        let button_states = HashMap::new();

        VolumePanMode {
            track_hw_assignments,
            track_states: button_states,
            last_sent_volume: HashMap::new(),
            last_sent_pan: HashMap::new(),
            to_reaper,
            from_reaper,
            to_xtouch,
            from_xtouch,
        }
    }

    fn get_track_state(&mut self, guid: String) -> &mut TrackState {
        self.track_states.entry(guid).or_insert(TrackState {
            buttons: ButtonState {
                mute: Button::new(),
                solo: Button::new(),
                arm: Button::new(),
            },
            pan: 0.5,          // Default center pan
            volume: FADER_0DB, // Default volume at 0dB
        })
    }

    fn get_guid_for_hw_channel(&self, hw_channel: usize) -> Option<String> {
        let assignments = self.track_hw_assignments.lock().unwrap();
        assignments[hw_channel].clone()
    }

    // For a given track GUID, find which hardware channel it's assigned to (if any)
    pub fn find_hw_channel(&self, guid: &str) -> Option<usize> {
        let assignments = self.track_hw_assignments.lock().unwrap();
        assignments
            .iter()
            .enumerate()
            .find(|(_, assigned_guid)| *assigned_guid == &Some(guid.to_string()))
            .map(|(hw_channel, _)| hw_channel)
    }
}

impl ModeHandler<TrackMsg, TrackMsg, XTouchDownstreamMsg, XTouchUpstreamMsg> for VolumePanMode {
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
                // We use track index according to reaper to assign tracks to hardware channels
                TrackDataPayload::ReaperTrackIndex(Some(index)) => {
                    // First, check if the assignment is changing. If not changing, do nothing.
                    if let Some(current_guid) =
                        &self.track_hw_assignments.lock().unwrap()[index as usize]
                    {
                        if current_guid == &msg.guid {
                            return curr_mode; // No change in assignment
                        }
                    }
                    {
                        // Clear any existing assignment for this track GUID before setting the new one
                        let mut assignments = self.track_hw_assignments.lock().unwrap();
                        for slot in assignments.iter_mut() {
                            if let Some(guid) = slot {
                                if guid == &msg.guid {
                                    *slot = None;
                                }
                            }
                        }
                        // Now set the new assignment
                        assignments[index as usize] = Some(msg.guid.clone());
                    }
                    // Now, send the current state of the track to the hardware for this channel
                    if let Some(hw_channel) = self.find_hw_channel(&msg.guid) {
                        let track_state = self.get_track_state(msg.guid.clone()).clone();
                        // Send volume
                        let _ = self
                            .to_xtouch
                            .send(XTouchDownstreamMsg::FaderAbs(FaderAbsMsg {
                                idx: hw_channel as i32,
                                value: track_state.volume as f64,
                            }));
                        // Send mute LED
                        let _ =
                            self.to_xtouch
                                .send(XTouchDownstreamMsg::MuteLED(xtouch::MuteLEDMsg {
                                    idx: hw_channel as i32,
                                    state: LEDState::from(track_state.buttons.mute.is_on()),
                                }));
                        // Send solo LED
                        let _ =
                            self.to_xtouch
                                .send(XTouchDownstreamMsg::SoloLED(xtouch::SoloLEDMsg {
                                    idx: hw_channel as i32,
                                    state: LEDState::from(track_state.buttons.solo.is_on()),
                                }));
                        // Send arm LED
                        let _ =
                            self.to_xtouch
                                .send(XTouchDownstreamMsg::ArmLED(xtouch::ArmLEDMsg {
                                    idx: hw_channel as i32,
                                    state: LEDState::from(track_state.buttons.arm.is_on()),
                                }));
                        // Send pan
                        let _ = self.to_xtouch.send(XTouchDownstreamMsg::EncoderRingLED(
                            xtouch::EncoderRingLEDMsg::RangePoint(EncoderRingLEDRangePointMsg {
                                idx: hw_channel as i32,
                                pos: track_state.pan,
                            }),
                        ));
                    }
                    return curr_mode;
                }
                TrackDataPayload::Volume(value) => {
                    self.get_track_state(msg.guid.clone()).volume = value;
                    if let Some(hw_channel) = self.find_hw_channel(&msg.guid) {
                        // Check if the change is significant enough to send
                        let should_send =
                            if let Some(&last_value) = self.last_sent_volume.get(&msg.guid) {
                                (value - last_value).abs() >= EPSILON
                            } else {
                                true // Always send if we haven't sent before
                            };

                        if should_send {
                            // Store the value we're sending
                            self.last_sent_volume.insert(msg.guid.clone(), value);

                            // Send volume update to XTouch for the corresponding fader
                            let fader_value = value; // TODO: scale appropriately
                            let _ =
                                self.to_xtouch
                                    .send(XTouchDownstreamMsg::FaderAbs(FaderAbsMsg {
                                        idx: hw_channel as i32,
                                        value: fader_value as f64,
                                    }));
                        }
                    }
                    return curr_mode;
                }
                TrackDataPayload::Muted(muted) => {
                    self.get_track_state(msg.guid.clone())
                        .buttons
                        .mute
                        .set(muted);
                    if let Some(hw_channel) = self.find_hw_channel(&msg.guid) {
                        // Send mute LED update to XTouch
                        let _ =
                            self.to_xtouch
                                .send(XTouchDownstreamMsg::MuteLED(xtouch::MuteLEDMsg {
                                    idx: hw_channel as i32,
                                    state: LEDState::from(muted),
                                }));
                    }
                    return curr_mode;
                }
                TrackDataPayload::Soloed(soloed) => {
                    self.get_track_state(msg.guid.clone())
                        .buttons
                        .solo
                        .set(soloed);
                    if let Some(hw_channel) = self.find_hw_channel(&msg.guid) {
                        // Send solo LED update to XTouch
                        let _ =
                            self.to_xtouch
                                .send(XTouchDownstreamMsg::SoloLED(xtouch::SoloLEDMsg {
                                    idx: hw_channel as i32,
                                    state: LEDState::from(soloed),
                                }));
                    }
                    return curr_mode;
                }
                TrackDataPayload::Armed(armed) => {
                    self.get_track_state(msg.guid.clone())
                        .buttons
                        .arm
                        .set(armed);
                    if let Some(hw_channel) = self.find_hw_channel(&msg.guid) {
                        // Send arm LED update to XTouch
                        let _ =
                            self.to_xtouch
                                .send(XTouchDownstreamMsg::ArmLED(xtouch::ArmLEDMsg {
                                    idx: hw_channel as i32,
                                    state: LEDState::from(armed),
                                }));
                    }
                    return curr_mode;
                }
                TrackDataPayload::Pan(value) => {
                    self.get_track_state(msg.guid.clone()).pan = value;
                    if let Some(hw_channel) = self.find_hw_channel(&msg.guid) {
                        // Check if the change is significant enough to send
                        let should_send =
                            if let Some(&last_value) = self.last_sent_pan.get(&msg.guid) {
                                (value - last_value).abs() >= EPSILON
                            } else {
                                true // Always send if we haven't sent before
                            };

                        if should_send {
                            // Store the value we're sending
                            self.last_sent_pan.insert(msg.guid.clone(), value);

                            // Send pan update to XTouch for the corresponding encoder
                            let pan_value = value; // TODO: scale appropriately
                            let _ = self.to_xtouch.send(XTouchDownstreamMsg::EncoderRingLED(
                                xtouch::EncoderRingLEDMsg::RangePoint(
                                    EncoderRingLEDRangePointMsg {
                                        idx: hw_channel as i32,
                                        pos: pan_value,
                                    },
                                ),
                            ));
                        }
                    }
                    return curr_mode;
                }
                _ => {
                    // Ignore unhandled payloads (e.g., Selected, SendIndex, etc.)
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
            XTouchUpstreamMsg::GlobalPress => curr_mode, // GlobalPress maps to this mode!
            // MIDITracksPress maps to ReaperSends mode
            XTouchUpstreamMsg::MIDITracksPress => {
                // Request transition to ReaperSends mode
                ModeState {
                    mode: Mode::ReaperSends,
                    state: State::RequestingModeTransition,
                }
            }
            XTouchUpstreamMsg::FaderAbs(fader_msg) => {
                if let Some(guid) =
                    &self.track_hw_assignments.lock().unwrap()[fader_msg.idx as usize]
                {
                    // Send volume update to Reaper for the corresponding track
                    let _ = self.to_reaper.send(TrackMsg::TrackDataMsg(TrackDataMsg {
                        direction: Direction::Upstream,
                        guid: guid.clone(),
                        data: TrackDataPayload::Volume(fader_msg.value as f32), // TODO: Need to scale appropriately
                    }));
                }
                curr_mode
            }
            XTouchUpstreamMsg::MutePress(mute_msg) => {
                if let Some(guid) = self.get_guid_for_hw_channel(mute_msg.idx as usize) {
                    let new_state = self.get_track_state(guid.clone()).buttons.mute.toggle();
                    // Send mute toggle to Reaper for the corresponding track
                    self.to_reaper
                        .send(TrackMsg::TrackDataMsg(TrackDataMsg {
                            direction: Direction::Upstream,
                            guid: guid.clone(),
                            data: TrackDataPayload::Muted(new_state),
                        }))
                        .unwrap();
                    // Update the toggle on the hardware
                    self.to_xtouch
                        .send(XTouchDownstreamMsg::MuteLED(xtouch::MuteLEDMsg {
                            idx: mute_msg.idx,
                            state: LEDState::from(new_state),
                        }))
                        .unwrap();
                }
                curr_mode
            }
            XTouchUpstreamMsg::SoloPress(solo_msg) => {
                if let Some(guid) = self.get_guid_for_hw_channel(solo_msg.idx as usize) {
                    let new_state = self.get_track_state(guid.clone()).buttons.solo.toggle();
                    // Send solo toggle to Reaper for the corresponding track
                    self.to_reaper
                        .send(TrackMsg::TrackDataMsg(TrackDataMsg {
                            direction: Direction::Upstream,
                            guid: guid.clone(),
                            data: TrackDataPayload::Soloed(new_state),
                        }))
                        .unwrap();
                    self.to_xtouch
                        .send(XTouchDownstreamMsg::SoloLED(xtouch::SoloLEDMsg {
                            idx: solo_msg.idx,
                            state: LEDState::from(new_state),
                        }))
                        .unwrap();
                }
                curr_mode
            }
            XTouchUpstreamMsg::ArmPress(arm_msg) => {
                if let Some(guid) = self.get_guid_for_hw_channel(arm_msg.idx as usize) {
                    let new_state = self.get_track_state(guid.clone()).buttons.arm.toggle();
                    // Send arm toggle to Reaper for the corresponding track
                    self.to_reaper
                        .send(TrackMsg::TrackDataMsg(TrackDataMsg {
                            direction: Direction::Upstream,
                            guid: guid.clone(),
                            data: TrackDataPayload::Armed(new_state),
                        }))
                        .unwrap();
                    self.to_xtouch
                        .send(XTouchDownstreamMsg::ArmLED(xtouch::ArmLEDMsg {
                            idx: arm_msg.idx,
                            state: LEDState::from(new_state),
                        }))
                        .unwrap();
                }
                curr_mode
            }
            XTouchUpstreamMsg::EncoderTurnInc(encoder_msg) => {
                if let Some(guid) = self.get_guid_for_hw_channel(encoder_msg.idx as usize) {
                    // Get current pan value and increment it
                    let current_pan = self.get_track_state(guid.clone()).pan;
                    let new_pan = (current_pan + 0.05).min(1.0); // Clamp to max 1.0

                    // Update stored pan value
                    self.get_track_state(guid.clone()).pan = new_pan;

                    // Send pan update upstream to Reaper
                    self.to_reaper
                        .send(TrackMsg::TrackDataMsg(TrackDataMsg {
                            direction: Direction::Upstream,
                            guid: guid.clone(),
                            data: TrackDataPayload::Pan(new_pan),
                        }))
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
                if let Some(guid) = self.get_guid_for_hw_channel(encoder_msg.idx as usize) {
                    // Get current pan value and decrement it
                    let current_pan = self.get_track_state(guid.clone()).pan;
                    let new_pan = (current_pan - 0.05).max(0.0); // Clamp to min 0.0

                    // Update stored pan value
                    self.get_track_state(guid.clone()).pan = new_pan;

                    // Send pan update upstream to Reaper
                    self.to_reaper
                        .send(TrackMsg::TrackDataMsg(TrackDataMsg {
                            direction: Direction::Upstream,
                            guid: guid.clone(),
                            data: TrackDataPayload::Pan(new_pan),
                        }))
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
            _ => curr_mode,
        }
    }
}

impl VolumePanMode {
    pub fn initiate_mode_transition(&mut self, upstream: Sender<TrackMsg>) -> ModeState {
        self.track_hw_assignments
            .lock()
            .unwrap()
            .iter()
            .for_each(|assignment| {
                if let Some(guid) = assignment {
                    // Request track data from Reaper for each assigned track
                    let _ = self.to_reaper.send(TrackMsg::TrackQuery(TrackQuery {
                        guid: guid.clone(),
                        direction: Direction::Upstream,
                    }));
                }
            });
        let barrier = Barrier::new();
        upstream.send(TrackMsg::Barrier(barrier)).unwrap();
        ModeState {
            mode: Mode::ReaperVolPan,
            state: State::WaitingBarrierFromDownstream(barrier),
        }
    }
}
