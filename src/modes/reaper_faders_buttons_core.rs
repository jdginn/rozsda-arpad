use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crossbeam_channel::Sender;
use uuid::Uuid;

use crate::midi::xtouch;
use crate::modes::button::Button;
use crate::track::track;

// FIXME:
// KNOWN BUGS:
// FIXME:
//
// When 8 or more channels are configured, all faders stop working
//
// Encoders don't pick up the faster speeds. Also, they appear to always increase Reaper rather than
// either increasing OR decreasing it based on their direction of rotation.
//
// Reaper's Select doesn't seem to be respected
//
// We have only implemented naming on scribbles
//
// FIXME:

// This functionality is common enough to warrant making it reusable.
//
// This is the core functionality for VolumePanMode but it shows up in other modes as well e.g.
// ChannelStripMode.
//
// Note that in all situations, this functionality will be extended to connect additional controls.
// As such, this does not implement the ModeHandler interface, and cannot be used on its own.

// Threshold for filtering out insignificant volume changes
const FADER_EPSILON: f32 = 0.01;
// FIXME: find the real value
pub const FADER_0DB: f32 = 0.72; // Placeholder value for 0dB on fader reusable

#[derive(Clone, Copy)]
struct MuteSoloArmButtonState {
    mute: Button,
    solo: Button,
    arm: Button,
}

#[derive(Clone, Copy)]
struct TrackState {
    buttons: MuteSoloArmButtonState,
    volume: f32,
}

#[derive(Clone, Copy)]
pub struct TrackIndexUpdateEpilogue {
    pub track_guid: Uuid,
    pub hw_channel: usize,
}

/// Implements a mode where that "basic" reaper functionality is mapped to the channel strips on
/// the control surface, namely:
/// - Volume on faders
/// - Select/Mute/Solo/Arm on buttons
///
/// Button LED toggling is handled here (downstream does not need to worry about managing button
/// LEDS.)
pub struct VolumeFadersCore {
    // Maps each channel on the hardware controller to a Reaper track
    pub track_hw_assignments: Arc<Mutex<Vec<Option<Uuid>>>>,
    // Store state for each track by track GUID
    track_states: HashMap<Uuid, TrackState>,
    // Store last volume sent downstream by track GUID for de-jitter
    last_sent_volume: Vec<f32>,
}

impl VolumeFadersCore {
    pub fn new(num_channels: usize) -> Self {
        let track_hw_assignments = Arc::new(Mutex::new(vec![None; num_channels]));
        let track_states = HashMap::new();

        VolumeFadersCore {
            track_hw_assignments,
            track_states,
            last_sent_volume: vec![0.0; num_channels], // Initialize with default volume values
        }
    }

    fn get_track_state(&mut self, guid: Uuid) -> &mut TrackState {
        self.track_states.entry(guid).or_insert(TrackState {
            buttons: MuteSoloArmButtonState {
                mute: Button::new(),
                solo: Button::new(),
                arm: Button::new(),
            },
            volume: FADER_0DB, // Default volume at 0dB
        })
    }

    // Return the track_guid associated with the hardware channel
    pub fn get_guid_for_hw_channel(&self, hw_channel: usize) -> Option<Uuid> {
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

    pub fn handle_message_from_upstream<F>(
        &mut self,
        msg: track::DataMsg,
        to_downstream: Sender<xtouch::DownstreamMsg>,
        mut track_index_epilogue: F,
    ) where
        F: FnMut(TrackIndexUpdateEpilogue),
    {
        match msg {
            // We use track index according to reaper to assign tracks to hardware channels
            track::DataMsg::ReaperTrackIndex(msg) => {
                if let Some(index) = msg.track_index {
                    if index < 1 {
                        // Invalid index, ignore
                        // Reaper starts indexing from 1
                        return;
                    }
                    // First, check if the assignment is changing. If not changing, do nothing.
                    if let Some(current_guid) =
                        &self.track_hw_assignments.lock().unwrap()[index as usize]
                    {
                        if current_guid == &msg.track_guid {
                            return; // No change in assignment
                        }
                    }
                    // Clear any existing assignment for this track GUID before setting the new one
                    let mut assignments = self.track_hw_assignments.lock().unwrap();
                    for slot in assignments.iter_mut() {
                        if let Some(guid) = slot {
                            if guid == &msg.track_guid {
                                *slot = None; // FIXME: seems sketchy
                            }
                        }
                    }
                    // Now set the new assignment
                    assignments[index as usize - 1] = Some(msg.track_guid);
                }
                // Now, send the current state of the track to the hardware for this channel
                if let Some(hw_channel) = self.find_hw_channel(msg.track_guid) {
                    let track_state = *self.get_track_state(msg.track_guid);
                    // Send volume
                    let _ =
                        to_downstream.send(xtouch::DownstreamMsg::FaderAbs(xtouch::FaderAbsMsg {
                            idx: hw_channel as i32,
                            value: track_state.volume as f64,
                        }));
                    // Update EPSILON tracking for volume since we just sent it
                    self.last_sent_volume[hw_channel] = track_state.volume;

                    // Send mute LED
                    let _ =
                        to_downstream.send(xtouch::DownstreamMsg::MuteLED(xtouch::MuteLEDMsg {
                            idx: hw_channel as i32,
                            state: xtouch::LEDState::from(track_state.buttons.mute.is_on()),
                        }));
                    // Send solo LED
                    let _ =
                        to_downstream.send(xtouch::DownstreamMsg::SoloLED(xtouch::SoloLEDMsg {
                            idx: hw_channel as i32,
                            state: xtouch::LEDState::from(track_state.buttons.solo.is_on()),
                        }));
                    // Send arm LED
                    let _ = to_downstream.send(xtouch::DownstreamMsg::ArmLED(xtouch::ArmLEDMsg {
                        idx: hw_channel as i32,
                        state: xtouch::LEDState::from(track_state.buttons.arm.is_on()),
                    }));
                    track_index_epilogue(TrackIndexUpdateEpilogue {
                        track_guid: msg.track_guid,
                        hw_channel,
                    });
                }
            }
            track::DataMsg::Volume(msg) => {
                self.get_track_state(msg.track_guid).volume = msg.volume;
                if let Some(hw_channel) = self.find_hw_channel(msg.track_guid) {
                    // Check if the change is significant enough to send
                    let should_send = true;
                    // (self.last_sent_volume[hw_channel] - msg.volume).abs() >= FADER_EPSILON;

                    if should_send {
                        // Send volume update to XTouch for the corresponding fader
                        let fader_value = msg.volume; // TODO: scale appropriately
                        let _ = to_downstream.send(xtouch::DownstreamMsg::FaderAbs(
                            xtouch::FaderAbsMsg {
                                idx: hw_channel as i32,
                                value: fader_value as f64,
                            },
                        ));
                        // Store the value we just sent
                        self.last_sent_volume[hw_channel] = msg.volume;
                    }
                }
            }
            track::DataMsg::Muted(msg) => {
                self.get_track_state(msg.track_guid)
                    .buttons
                    .mute
                    .set(msg.muted);
                if let Some(hw_channel) = self.find_hw_channel(msg.track_guid) {
                    // Send mute LED update to XTouch
                    let _ =
                        to_downstream.send(xtouch::DownstreamMsg::MuteLED(xtouch::MuteLEDMsg {
                            idx: hw_channel as i32,
                            state: xtouch::LEDState::from(msg.muted),
                        }));
                }
            }
            track::DataMsg::Soloed(msg) => {
                self.get_track_state(msg.track_guid)
                    .buttons
                    .solo
                    .set(msg.soloed);
                if let Some(hw_channel) = self.find_hw_channel(msg.track_guid) {
                    // Send solo LED update to XTouch
                    let _ =
                        to_downstream.send(xtouch::DownstreamMsg::SoloLED(xtouch::SoloLEDMsg {
                            idx: hw_channel as i32,
                            state: xtouch::LEDState::from(msg.soloed),
                        }));
                }
            }
            track::DataMsg::Armed(msg) => {
                self.get_track_state(msg.track_guid)
                    .buttons
                    .arm
                    .set(msg.armed);
                if let Some(hw_channel) = self.find_hw_channel(msg.track_guid) {
                    // Send arm LED update to XTouch
                    let _ = to_downstream.send(xtouch::DownstreamMsg::ArmLED(xtouch::ArmLEDMsg {
                        idx: hw_channel as i32,
                        state: xtouch::LEDState::from(msg.armed),
                    }));
                }
            }
            _ => {
                // Ignore unhandled payloads (e.g., Selected, SendIndex, etc.)
            }
        }
    }

    pub fn handle_message_from_downstream(
        &mut self,
        msg: xtouch::UpstreamMsg,
        to_upstream: Sender<track::TrackMsg>,
        to_downstream: Sender<xtouch::DownstreamMsg>,
    ) {
        match msg {
            xtouch::UpstreamMsg::FaderAbs(fader_msg) => {
                if let Some(guid) =
                    &self.track_hw_assignments.lock().unwrap()[fader_msg.idx as usize]
                {
                    // Send volume update to Reaper for the corresponding track
                    let _ = to_upstream.send(
                        track::Volume {
                            track_guid: *guid,
                            volume: fader_msg.value as f32, // TODO: Need to scale appropriately
                        }
                        .into(),
                    );
                    // Convince the xtouch to leave its faders where we put them
                    let _ =
                        to_downstream.send(xtouch::DownstreamMsg::FaderAbs(xtouch::FaderAbsMsg {
                            idx: fader_msg.idx,
                            value: fader_msg.value,
                        }));
                }
            }
            xtouch::UpstreamMsg::MutePress(mute_msg) => {
                if let Some(guid) = self.get_guid_for_hw_channel(mute_msg.idx as usize) {
                    let new_state = self.get_track_state(guid).buttons.mute.toggle();
                    // Send mute toggle to Reaper for the corresponding track
                    to_upstream
                        .send(
                            track::Muted {
                                track_guid: guid,
                                muted: new_state,
                            }
                            .into(),
                        )
                        .unwrap();
                    // Update the toggle on the hardware
                    to_downstream
                        .send(xtouch::DownstreamMsg::MuteLED(xtouch::MuteLEDMsg {
                            idx: mute_msg.idx,
                            state: xtouch::LEDState::from(new_state),
                        }))
                        .unwrap();
                }
            }
            xtouch::UpstreamMsg::SoloPress(solo_msg) => {
                if let Some(guid) = self.get_guid_for_hw_channel(solo_msg.idx as usize) {
                    let new_state = self.get_track_state(guid).buttons.solo.toggle();
                    // Send solo toggle to Reaper for the corresponding track
                    to_upstream
                        .send(
                            track::Soloed {
                                track_guid: guid,
                                soloed: new_state,
                            }
                            .into(),
                        )
                        .unwrap();
                    to_downstream
                        .send(xtouch::DownstreamMsg::SoloLED(xtouch::SoloLEDMsg {
                            idx: solo_msg.idx,
                            state: xtouch::LEDState::from(new_state),
                        }))
                        .unwrap();
                }
            }
            xtouch::UpstreamMsg::ArmPress(arm_msg) => {
                if let Some(guid) = self.get_guid_for_hw_channel(arm_msg.idx as usize) {
                    let new_state = self.get_track_state(guid).buttons.arm.toggle();
                    // Send arm toggle to Reaper for the corresponding track
                    to_upstream
                        .send(
                            track::Armed {
                                track_guid: guid,
                                armed: new_state,
                            }
                            .into(),
                        )
                        .unwrap();
                    to_downstream
                        .send(xtouch::DownstreamMsg::ArmLED(xtouch::ArmLEDMsg {
                            idx: arm_msg.idx,
                            state: xtouch::LEDState::from(new_state),
                        }))
                        .unwrap();
                }
            }
            _ => {
                // Ignore other messages
            }
        }
    }
}
