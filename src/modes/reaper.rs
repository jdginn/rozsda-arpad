use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::vec::Vec;

use crossbeam_channel::{Receiver, Sender, bounded};

use crate::midi::xtouch;
use crate::midi::xtouch::{FaderAbsMsg, LEDState, XTouchDownstreamMsg, XTouchUpstreamMsg};
use crate::track::track::{DataPayload as TrackDataPayload, Direction, TrackDataMsg, TrackMsg};

struct ButtonState {
    mute: bool,
    solo: bool,
    arm: bool,
}

pub struct VolumePanMode {
    // Maps each channel on the hardware controller to a Reaper track
    track_hw_assignments: Arc<Mutex<Vec<Option<String>>>>,
    track_states: HashMap<String, ButtonState>,
    to_reaper: Sender<TrackMsg>,
    from_reaper: Receiver<TrackMsg>,
    to_xtouch: Sender<XTouchDownstreamMsg>,
    from_xtouch: Receiver<XTouchUpstreamMsg>,
}

impl VolumePanMode {
    pub fn start(
        to_reaper: Sender<TrackMsg>,
        from_reaper: Receiver<TrackMsg>,
        to_xtouch: Sender<XTouchDownstreamMsg>,
        from_xtouch: Receiver<XTouchUpstreamMsg>,
    ) {
        let track_hw_assignments = Arc::new(Mutex::new(vec![None; 8])); // Assuming 8 channels
        let button_states = HashMap::new();

        let mut mode = VolumePanMode {
            track_hw_assignments,
            track_states: button_states,
            to_reaper,
            from_reaper,
            to_xtouch,
            from_xtouch,
        };

        thread::spawn(move || {
            loop {
                mode.handle_messages();
            }
        });
    }

    pub fn find_hw_channel(&self, guid: &str) -> Option<usize> {
        let assignments = self.track_hw_assignments.lock().unwrap();
        assignments
            .iter()
            .enumerate()
            .find(|(_, assigned_guid)| *assigned_guid == &Some(guid.to_string()))
            .map(|(hw_channel, _)| hw_channel)
    }

    pub fn handle_messages(&mut self) {
        // Handle messages from Reaper and XTouch here
        // Update track_hw_assignments as needed
        if let Ok(TrackMsg::TrackDataMsg(msg)) = self.from_reaper.recv() {
            match msg.data {
                TrackDataPayload::ReaperTrackIndex(Some(index)) => {
                    self.track_hw_assignments.lock().unwrap()[index as usize] =
                        Some(msg.guid.clone());
                }
                TrackDataPayload::Volume(value) => {
                    if let Some(hw_channel) = self.find_hw_channel(&msg.guid) {
                        // Send volume update to XTouch for the corresponding fader
                        let fader_value = value; // TODO: scale appropriately
                        let _ = self
                            .to_xtouch
                            .send(XTouchDownstreamMsg::FaderAbs(FaderAbsMsg {
                                direction: Direction::Downstream,
                                idx: hw_channel as i32,
                                value: fader_value as f64,
                            }));
                    }
                }
                TrackDataPayload::Muted(muted) => {
                    if let Some(hw_channel) = self.find_hw_channel(&msg.guid) {
                        self.track_states
                            .entry(msg.guid)
                            .or_insert(ButtonState {
                                mute: false,
                                solo: false,
                                arm: false,
                            })
                            .mute = muted;
                        // Send mute LED update to XTouch
                        let _ =
                            self.to_xtouch
                                .send(XTouchDownstreamMsg::MuteLED(xtouch::MuteLEDMsg {
                                    direction: Direction::Downstream,
                                    idx: hw_channel as i32,
                                    state: match muted {
                                        true => LEDState::On,
                                        false => LEDState::Off,
                                    },
                                }));
                    }
                }
                TrackDataPayload::Soloed(soloed) => {
                    if let Some(hw_channel) = self.find_hw_channel(&msg.guid) {
                        self.track_states
                            .entry(msg.guid)
                            .or_insert(ButtonState {
                                mute: false,
                                solo: false,
                                arm: false,
                            })
                            .solo = soloed;
                        // Send solo LED update to XTouch
                        let _ =
                            self.to_xtouch
                                .send(XTouchDownstreamMsg::SoloLED(xtouch::SoloLEDMsg {
                                    direction: Direction::Downstream,
                                    idx: hw_channel as i32,
                                    state: match soloed {
                                        true => LEDState::On,
                                        false => LEDState::Off,
                                    },
                                }));
                    }
                }
                TrackDataPayload::Armed(armed) => {
                    if let Some(hw_channel) = self.find_hw_channel(&msg.guid) {
                        self.track_states
                            .entry(msg.guid)
                            .or_insert(ButtonState {
                                mute: false,
                                solo: false,
                                arm: false,
                            })
                            .arm = armed;
                        // Send arm LED update to XTouch
                        let _ =
                            self.to_xtouch
                                .send(XTouchDownstreamMsg::ArmLED(xtouch::ArmLEDMsg {
                                    direction: Direction::Downstream,
                                    idx: hw_channel as i32,
                                    state: match armed {
                                        true => LEDState::On,
                                        false => LEDState::Off,
                                    },
                                }));
                    }
                }
                _ => {}
            }
            // Update track_hw_assignments based on data_msg
            // For example, if data_msg contains track index info, map it to a hardware channel
        }
        if let Ok(msg) = self.from_xtouch.recv() {
            match msg {
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
                }
                XTouchUpstreamMsg::MutePress(mute_msg) => {
                    if let Some(guid) =
                        &self.track_hw_assignments.lock().unwrap()[mute_msg.idx as usize]
                    {
                        let curr = self
                            .track_states
                            .get(guid)
                            .map_or(false, |state| state.mute);
                        self.track_states
                            .entry(guid.clone())
                            .or_insert(ButtonState {
                                mute: false,
                                solo: false,
                                arm: false,
                            })
                            .mute = curr;
                        // Send mute toggle to Reaper for the corresponding track
                        self.to_reaper.send(TrackMsg::TrackDataMsg(TrackDataMsg {
                            direction: Direction::Upstream,
                            guid: guid.clone(),
                            data: TrackDataPayload::Muted(curr),
                        }));
                        // Update the toggle on the hardware
                        self.to_xtouch
                            .send(XTouchDownstreamMsg::MuteLED(xtouch::MuteLEDMsg {
                                direction: Direction::Downstream,
                                idx: mute_msg.idx,
                                state: match curr {
                                    true => LEDState::On,
                                    false => LEDState::Off,
                                },
                            }));
                    }
                }
                XTouchUpstreamMsg::SoloPress(solo_msg) => {
                    if let Some(guid) =
                        &self.track_hw_assignments.lock().unwrap()[solo_msg.idx as usize]
                    {
                        let curr = self
                            .track_states
                            .get(guid)
                            .map_or(false, |state| state.solo);
                        self.track_states
                            .entry(guid.clone())
                            .or_insert(ButtonState {
                                mute: false,
                                solo: false,
                                arm: false,
                            })
                            .solo = curr;
                        // Send solo toggle to Reaper for the corresponding track
                        self.to_reaper.send(TrackMsg::TrackDataMsg(TrackDataMsg {
                            direction: Direction::Upstream,
                            guid: guid.clone(),
                            data: TrackDataPayload::Soloed(curr),
                        }));
                        self.to_xtouch
                            .send(XTouchDownstreamMsg::SoloLED(xtouch::SoloLEDMsg {
                                direction: Direction::Downstream,
                                idx: solo_msg.idx,
                                state: match curr {
                                    true => LEDState::On,
                                    false => LEDState::Off,
                                },
                            }));
                    }
                }
                XTouchUpstreamMsg::ArmPress(arm_msg) => {
                    if let Some(guid) =
                        &self.track_hw_assignments.lock().unwrap()[arm_msg.idx as usize]
                    {
                        let curr = self.track_states.get(guid).map_or(false, |state| state.arm);
                        self.track_states
                            .entry(guid.clone())
                            .or_insert(ButtonState {
                                mute: false,
                                solo: false,
                                arm: false,
                            })
                            .arm = curr;
                        // Send arm toggle to Reaper for the corresponding track
                        self.to_reaper.send(TrackMsg::TrackDataMsg(TrackDataMsg {
                            direction: Direction::Upstream,
                            guid: guid.clone(),
                            data: TrackDataPayload::Armed(curr),
                        }));
                        self.to_xtouch
                            .send(XTouchDownstreamMsg::ArmLED(xtouch::ArmLEDMsg {
                                direction: Direction::Downstream,
                                idx: arm_msg.idx,
                                state: match curr {
                                    true => LEDState::On,
                                    false => LEDState::Off,
                                },
                            }));
                    }
                }
                _ => {}
            }
        }
    }
}
