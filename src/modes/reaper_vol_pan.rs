use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::vec::Vec;

use crossbeam_channel::{Receiver, Sender};
use uuid::Uuid;

use crate::midi::xtouch::{self};
use crate::midi::xtouch::{
    EncoderRingMsg, FaderAbsMsg, LEDState, XTouchDownstreamMsg, XTouchUpstreamMsg,
};
use crate::modes::mode_manager::{Barrier, Mode, ModeHandler, ModeState, State};
use crate::track::track;
use crate::track::track::{TrackMsg, TrackQuery};

// Threshold for filtering out insignificant volume/pan changes
const EPSILON: f32 = 0.01;

pub const FADER_0DB: f32 = 0.72; // Placeholder value for 0dB on fader scale

pub fn map_to_0xb(x: f32) -> u8 {
    let clamped = x.clamp(-1.0, 1.0) as f64;
    ((clamped + 1.0) * 0.5 * 0xb as f64).round() as u8
}

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
    track_hw_assignments: Arc<Mutex<Vec<Option<Uuid>>>>,
    // Store state for each track by track GUID
    track_states: HashMap<Uuid, TrackState>,
    // Store last sent volume/pan values to avoid sending updates for tiny changes
    last_sent_volume: HashMap<Uuid, f32>,
    last_sent_pan: HashMap<Uuid, f32>,
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

    fn get_track_state(&mut self, guid: Uuid) -> &mut TrackState {
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

    fn get_guid_for_hw_channel(&self, hw_channel: usize) -> Option<Uuid> {
        let assignments = self.track_hw_assignments.lock().unwrap();
        assignments[hw_channel]
    }

    // For a given track GUID, find which hardware channel it's assigned to (if any)
    pub fn find_hw_channel(&self, guid: Uuid) -> Option<usize> {
        let assignments = self.track_hw_assignments.lock().unwrap();
        assignments
            .iter()
            .enumerate()
            .find(|(_, assigned_guid)| **assigned_guid == Some(guid))
            .map(|(hw_channel, _)| hw_channel)
    }
}

impl ModeHandler<TrackMsg, TrackMsg, XTouchDownstreamMsg, XTouchUpstreamMsg> for VolumePanMode {
    fn handle_downstream_messages(&mut self, msg: TrackMsg, curr_mode: ModeState) -> ModeState {
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
                    // We use track index according to reaper to assign tracks to hardware channels
                    track::DataMsg::ReaperTrackIndex(msg) => {
                        if let Some(index) = msg.track_index {
                            if index < 1 {
                                // Invalid index, ignore
                                // Reaper starts indexing from 1
                                return curr_mode;
                            }
                            // First, check if the assignment is changing. If not changing, do nothing.
                            if let Some(current_guid) =
                                &self.track_hw_assignments.lock().unwrap()[index as usize]
                            {
                                if current_guid == &msg.track_guid {
                                    return curr_mode; // No change in assignment
                                }
                            }
                            // Clear any existing assignment for this track GUID before setting the new one
                            let mut assignments = self.track_hw_assignments.lock().unwrap();
                            for slot in assignments.iter_mut() {
                                if let Some(guid) = slot {
                                    if guid == &msg.track_guid {
                                        // Clear EPSILON tracking for this track since it's being unmapped
                                        self.last_sent_volume.remove(guid);
                                        self.last_sent_pan.remove(guid);
                                        *slot = None;
                                    }
                                }
                            }
                            // Now set the new assignment
                            assignments[index as usize - 1] = Some(msg.track_guid);
                        }
                        // Now, send the current state of the track to the hardware for this channel
                        if let Some(hw_channel) = self.find_hw_channel(msg.track_guid) {
                            let track_state = self.get_track_state(msg.track_guid).clone();
                            // Send volume
                            let _ =
                                self.to_xtouch
                                    .send(XTouchDownstreamMsg::FaderAbs(FaderAbsMsg {
                                        idx: hw_channel as i32,
                                        value: track_state.volume as f64,
                                    }));
                            // Update EPSILON tracking for volume since we just sent it
                            self.last_sent_volume
                                .insert(msg.track_guid, track_state.volume);

                            // Send mute LED
                            let _ = self.to_xtouch.send(
                                xtouch::MuteLEDMsg {
                                    idx: hw_channel as i32,
                                    state: LEDState::from(track_state.buttons.mute.is_on()),
                                }
                                .into(),
                            );
                            // Send solo LED
                            let _ = self.to_xtouch.send(
                                xtouch::SoloLEDMsg {
                                    idx: hw_channel as i32,
                                    state: LEDState::from(track_state.buttons.solo.is_on()),
                                }
                                .into(),
                            );
                            // Send arm LED
                            let _ = self.to_xtouch.send(
                                xtouch::ArmLEDMsg {
                                    idx: hw_channel as i32,
                                    state: LEDState::from(track_state.buttons.arm.is_on()),
                                }
                                .into(),
                            );
                            // Send pan
                            let _ = self.to_xtouch.send(XTouchDownstreamMsg::EncoderRingLED(
                                EncoderRingMsg {
                                    idx: hw_channel as i32,
                                    mode: xtouch::EncoderRingMode::Point,
                                    val: map_to_0xb(track_state.pan),
                                }
                                .into(),
                            ));
                            // Update EPSILON tracking for pan since we just sent it
                            self.last_sent_pan.insert(msg.track_guid, track_state.pan);
                        }
                        curr_mode
                    }
                    track::DataMsg::Volume(msg) => {
                        self.get_track_state(msg.track_guid).volume = msg.volume;
                        if let Some(hw_channel) = self.find_hw_channel(msg.track_guid) {
                            // Check if the change is significant enough to send
                            let should_send = if let Some(&last_value) =
                                self.last_sent_volume.get(&msg.track_guid)
                            {
                                (msg.volume - last_value).abs() >= EPSILON
                            } else {
                                true // Always send if we haven't sent before
                            };

                            if should_send {
                                // Store the value we're sending
                                self.last_sent_volume.insert(msg.track_guid, msg.volume);

                                // Send volume update to XTouch for the corresponding fader
                                let fader_value = msg.volume; // TODO: scale appropriately
                                let _ = self.to_xtouch.send(
                                    FaderAbsMsg {
                                        idx: hw_channel as i32,
                                        value: fader_value as f64,
                                    }
                                    .into(),
                                );
                            }
                        }
                        curr_mode
                    }
                    track::DataMsg::Muted(msg) => {
                        self.get_track_state(msg.track_guid)
                            .buttons
                            .mute
                            .set(msg.muted);
                        if let Some(hw_channel) = self.find_hw_channel(msg.track_guid) {
                            // Send mute LED update to XTouch
                            let _ = self.to_xtouch.send(XTouchDownstreamMsg::MuteLED(
                                xtouch::MuteLEDMsg {
                                    idx: hw_channel as i32,
                                    state: LEDState::from(msg.muted),
                                },
                            ));
                        }
                        curr_mode
                    }
                    track::DataMsg::Soloed(msg) => {
                        self.get_track_state(msg.track_guid)
                            .buttons
                            .solo
                            .set(msg.soloed);
                        if let Some(hw_channel) = self.find_hw_channel(msg.track_guid) {
                            // Send solo LED update to XTouch
                            let _ = self.to_xtouch.send(XTouchDownstreamMsg::SoloLED(
                                xtouch::SoloLEDMsg {
                                    idx: hw_channel as i32,
                                    state: LEDState::from(msg.soloed),
                                },
                            ));
                        }
                        curr_mode
                    }
                    track::DataMsg::Armed(msg) => {
                        self.get_track_state(msg.track_guid)
                            .buttons
                            .arm
                            .set(msg.armed);
                        if let Some(hw_channel) = self.find_hw_channel(msg.track_guid) {
                            // Send arm LED update to XTouch
                            let _ = self.to_xtouch.send(XTouchDownstreamMsg::ArmLED(
                                xtouch::ArmLEDMsg {
                                    idx: hw_channel as i32,
                                    state: LEDState::from(msg.armed),
                                },
                            ));
                        }
                        curr_mode
                    }
                    track::DataMsg::Pan(msg) => {
                        self.get_track_state(msg.track_guid).pan = msg.pan;
                        if let Some(hw_channel) = self.find_hw_channel(msg.track_guid) {
                            // Check if the change is significant enough to send
                            let should_send = if let Some(&last_value) =
                                self.last_sent_pan.get(&msg.track_guid)
                            {
                                (msg.pan - last_value).abs() >= EPSILON
                            } else {
                                true // Always send if we haven't sent before
                            };

                            if should_send {
                                // Store the value we're sending
                                self.last_sent_pan.insert(msg.track_guid, msg.pan);

                                // Send pan update to XTouch for the corresponding encoder
                                let pan_value = msg.pan; // TODO: scale appropriately
                                let _ = self.to_xtouch.send(
                                    XTouchDownstreamMsg::EncoderRingLED(EncoderRingMsg {
                                        idx: hw_channel as i32,
                                        mode: xtouch::EncoderRingMode::Point,
                                        val: map_to_0xb(pan_value),
                                    })
                                    .into(),
                                );
                            }
                        }
                        curr_mode
                    }
                    _ => {
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
    fn handle_upstream_messages(
        &mut self,
        msg: XTouchUpstreamMsg,
        curr_mode: ModeState,
    ) -> ModeState {
        match msg {
            // GlobalPress maps to this mode!
            XTouchUpstreamMsg::GlobalPress => curr_mode,
            // MIDITracksPress maps to ReaperSends mode
            XTouchUpstreamMsg::MIDITracksPress => {
                println!("Requesting transition to ReaperSends mode");
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
                    let _ = self.to_reaper.send(
                        track::Volume {
                            track_guid: *guid,
                            volume: fader_msg.value as f32, // TODO: Need to scale appropriately
                        }
                        .into(),
                    );
                }
                curr_mode
            }
            XTouchUpstreamMsg::MutePress(mute_msg) => {
                if let Some(guid) = self.get_guid_for_hw_channel(mute_msg.idx as usize) {
                    let new_state = self.get_track_state(guid).buttons.mute.toggle();
                    // Send mute toggle to Reaper for the corresponding track
                    self.to_reaper
                        .send(
                            track::Muted {
                                track_guid: guid,
                                muted: new_state,
                            }
                            .into(),
                        )
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
                    let new_state = self.get_track_state(guid).buttons.solo.toggle();
                    // Send solo toggle to Reaper for the corresponding track
                    self.to_reaper
                        .send(
                            track::Soloed {
                                track_guid: guid,
                                soloed: new_state,
                            }
                            .into(),
                        )
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
                    let new_state = self.get_track_state(guid).buttons.arm.toggle();
                    // Send arm toggle to Reaper for the corresponding track
                    self.to_reaper
                        .send(
                            track::Armed {
                                track_guid: guid,
                                armed: new_state,
                            }
                            .into(),
                        )
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
                        .send(
                            track::Pan {
                                track_guid: guid.clone(),
                                pan: new_pan,
                            }
                            .into(),
                        )
                        .unwrap();

                    // Send encoder LED update downstream to hardware
                    self.to_xtouch
                        .send(XTouchDownstreamMsg::EncoderRingLED(EncoderRingMsg {
                            idx: encoder_msg.idx,
                            mode: xtouch::EncoderRingMode::Point,
                            val: map_to_0xb(new_pan),
                        }))
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
                        .send(
                            track::Pan {
                                track_guid: guid.clone(),
                                pan: new_pan,
                            }
                            .into(),
                        )
                        .unwrap();

                    // Send encoder LED update downstream to hardware
                    self.to_xtouch
                        .send(XTouchDownstreamMsg::EncoderRingLED(EncoderRingMsg {
                            idx: encoder_msg.idx,
                            mode: xtouch::EncoderRingMode::Point,
                            val: map_to_0xb(new_pan),
                        }))
                        .unwrap();
                }
                curr_mode
            }
            _ => curr_mode,
        }
    }
}

impl VolumePanMode {
    pub fn initiate_mode_transition(
        &mut self,
        from_mode: Mode,
        upstream: Sender<TrackMsg>,
    ) -> ModeState {
        self.track_hw_assignments
            .lock()
            .unwrap()
            .iter()
            .for_each(|assignment| {
                if let Some(guid) = assignment {
                    // Request track data from Reaper for each assigned track
                    let _ = self
                        .to_reaper
                        .send(TrackMsg::Query(TrackQuery { guid: guid.clone() }));
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
