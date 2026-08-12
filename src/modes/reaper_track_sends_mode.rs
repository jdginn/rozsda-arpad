use std::collections::BTreeMap;
use std::vec::Vec;

use uuid::Uuid;

use crate::midi::v1m;
use crate::modes::mode_manager::{
    DownstreamIo, ModeAction, ModeHandler, TransitionRequest, UpstreamIo,
};
use crate::track::track;
use crate::track::track::TrackMsg;

#[derive(Clone, Default)]
pub struct TrackSendInfo {
    pub guid: String,
    pub level: f32,
    pub pan: f32,
}

pub struct ReaperTrackSendsMode {
    // Number of channels in the hardware controller
    num_channels: usize,
    // Track index in Reaper that corresponds to the first channel on the hardware controller
    channel_offset: usize,
    // Maps track send index to send guid
    hw_assignments: Vec<Option<Uuid>>,
    // Maps guid to info about the send it designates
    track_send_states: BTreeMap<Uuid, TrackSendInfo>,
    selected_track_guid: Uuid,
}

impl ReaperTrackSendsMode {
    pub fn new(num_channels: usize, channel_offset: usize, selected_track_guid: Uuid) -> Self {
        ReaperTrackSendsMode {
            num_channels,
            channel_offset,
            hw_assignments: vec![None; num_channels],
            track_send_states: BTreeMap::new(),
            selected_track_guid,
        }
    }

    pub fn init(self, io: &mut dyn DownstreamIo) -> Self {
        for i in 0..self.num_channels {
            io.send_to_v1m(v1m::DownstreamMsg::ChannelFader(v1m::ChannelFaderMsg {
                idx: i as i32,
                value: 0.0,
            }));
            io.send_to_v1m(v1m::DownstreamMsg::MuteLED(v1m::MuteLEDMsg {
                idx: i as i32,
                state: v1m::LEDState::Off,
            }));
            io.send_to_v1m(v1m::DownstreamMsg::SoloLED(v1m::SoloLEDMsg {
                idx: i as i32,
                state: v1m::LEDState::Off,
            }));
            io.send_to_v1m(v1m::DownstreamMsg::ArmLED(v1m::ArmLEDMsg {
                idx: i as i32,
                state: v1m::LEDState::Off,
            }));
            io.send_to_v1m(v1m::DownstreamMsg::EncoderRingLED(v1m::EncoderRingMsg {
                idx: i as i32,
                mode: v1m::EncoderRingMode::Point,
                val: v1m::ENCODER_CENTER_MODE_CENTER,
            }));
            io.send_to_v1m(v1m::DownstreamMsg::TopScribbleStripLine1Text(
                v1m::TopScribbleStripLine1TextMsg {
                    idx: i as i32,
                    text: String::new(),
                },
            ));
            io.send_to_v1m(v1m::DownstreamMsg::TopScribbleStripLine2Text(
                v1m::TopScribbleStripLine2TextMsg {
                    idx: i as i32,
                    text: String::new(),
                },
            ));
            io.send_to_v1m(v1m::DownstreamMsg::TopScribbleStripBackgroundColor(
                v1m::TopScribbleStripColorMsg {
                    idx: i as i32,
                    color: v1m::Color { r: 0, g: 0, b: 0 },
                },
            ));
            io.send_to_v1m(v1m::DownstreamMsg::BottomScribbleStripLine1Text(
                v1m::BottomScribbleStripLine1TextMsg {
                    idx: i as i32,
                    text: String::new(),
                },
            ));
            io.send_to_v1m(v1m::DownstreamMsg::BottomScribbleStripLine2Text(
                v1m::BottomScribbleStripLine2TextMsg {
                    idx: i as i32,
                    text: String::new(),
                },
            ));
        }
        self
    }

    fn get_guid_for_hw_channel(&self, hw_channel: usize) -> Option<Uuid> {
        self.hw_assignments[hw_channel]
    }

    pub fn find_hw_channel(&self, guid: Uuid) -> Option<usize> {
        for (hw_channel, &assigned_guid) in self.hw_assignments.iter().enumerate() {
            println!(
                "Checking hw_channel {} with assigned_guid {:?}",
                hw_channel, assigned_guid
            );
            if let Some(assigned_guid) = assigned_guid {
                if assigned_guid == guid {
                    return Some(hw_channel);
                }
            }
        }
        None
    }
}

impl ModeHandler for ReaperTrackSendsMode {
    fn handle_msg_from_upstream(&mut self, msg: TrackMsg, io: &mut dyn UpstreamIo) -> ModeAction {
        match track::DataMsg::try_from(msg) {
            Ok(msg) => {
                println!("ReaperTrackSendsMode received msg: {:?}", msg);
                println!("hw_channel_mapping {:?}", self.hw_assignments);
                match msg {
                    track::DataMsg::Name(msg) => {
                        if let Some(hw_channel) = self.find_hw_channel(msg.track_guid) {
                            io.send_to_v1m(
                                v1m::TopScribbleStripLine1TextMsg {
                                    idx: hw_channel as i32,
                                    text: msg.name.clone(),
                                }
                                .into(),
                            );
                            io.send_to_v1m(
                                v1m::BottomScribbleStripLine2TextMsg {
                                    idx: hw_channel as i32,
                                    text: msg.name,
                                }
                                .into(),
                            );
                        }
                        ModeAction::None
                    }
                    track::DataMsg::RgbColor(msg) => {
                        println!("Rgb msg {:?}", msg);
                        println!("hw_channel_mapping {:?}", self.hw_assignments);
                        if let Some(hw_channel) = self.find_hw_channel(msg.track_guid) {
                            io.send_to_v1m(
                                v1m::TopScribbleStripColorMsg {
                                    idx: hw_channel as i32,
                                    color: v1m::Color {
                                        r: msg.r,
                                        g: msg.g,
                                        b: msg.b,
                                    },
                                }
                                .into(),
                            );
                        }
                        ModeAction::None
                    }
                    // If a new track is selected, we need to initiate a mode transition so that we
                    // are controlling sends for that new track
                    //
                    // FIXME: this behavior doesn't seem quite right in terms of intent
                    track::DataMsg::Selected(msg) => {
                        if msg.track_guid == self.selected_track_guid && msg.selected {
                            // No change, skip
                            return ModeAction::None;
                        }
                        // TODO: what happens if we deselect the currently selected track? Nothing?
                        if msg.selected {
                            self.selected_track_guid = msg.track_guid;
                            let state = match msg.selected {
                                true => v1m::LEDState::On,
                                false => v1m::LEDState::Off,
                            };
                            io.send_to_v1m(
                                v1m::SelectLEDMsg {
                                    idx: self.find_hw_channel(msg.track_guid).unwrap_or(0) as i32,
                                    state,
                                }
                                .into(),
                            );
                        }
                        ModeAction::None
                    }
                    track::DataMsg::SendIndex(msg) => {
                        if msg.track_guid == self.selected_track_guid {
                            // Only process send index messages for the currently selected track
                        } else {
                            return ModeAction::None;
                        }

                        println!("SendIndex msg: {:?}", msg);

                        println!("msg: {:?}", msg);

                        // If the send was previously mapped to a hw_channel, zero that channel
                        if let Some(index) = self.find_hw_channel(msg.send_guid) {
                            println!(
                                "Adjusted send index: {}",
                                msg.send_index - self.channel_offset as i32
                            );
                            if index as i32 == msg.send_index {
                                // No change, skip
                                return ModeAction::None;
                            }
                            // TODO: is this pointless?
                            self.hw_assignments[index - self.channel_offset] = None;
                            io.send_to_v1m(
                                v1m::ChannelFaderMsg {
                                    idx: index as i32,
                                    value: 0.0,
                                }
                                .into(),
                            );
                            io.send_to_v1m(
                                v1m::EncoderRingMsg {
                                    idx: index as i32,
                                    mode: v1m::EncoderRingMode::Point,
                                    val: v1m::ENCODER_CENTER_MODE_CENTER,
                                }
                                .into(),
                            );
                        }

                        // Add bounds checking to prevent panic on invalid send_index
                        // If out of bounds, silently ignore (could log error in production)
                        if (msg.send_index as usize) < self.channel_offset
                            || (msg.send_index as usize) >= self.channel_offset + self.num_channels
                        {
                            // Out of bounds, skip
                            return ModeAction::None;
                        };
                        self.hw_assignments[msg.send_index as usize] = Some(msg.send_guid);
                        // Insert default state into self.track_send_states if not already present
                        let state = self
                            .track_send_states
                            .entry(msg.send_guid)
                            .or_default()
                            .clone();
                        println!(
                            "Setting send index {} to guid {} ",
                            msg.send_index, msg.send_guid
                        );
                        println!(
                            "Adjusted send index: {}",
                            msg.send_index - self.channel_offset as i32
                        );
                        // Send current state to hardware for this send index
                        io.send_to_v1m(
                            v1m::ChannelFaderMsg {
                                idx: msg.send_index - self.channel_offset as i32,
                                value: state.level as f64, // TODO: scale appropriately
                            }
                            .into(),
                        );
                        io.send_to_v1m(
                            v1m::EncoderRingMsg::new(
                                msg.send_index - self.channel_offset as i32,
                                v1m::EncoderRingMode::FromCenter,
                                state.pan,
                            )
                            .into(),
                        );
                        ModeAction::None
                    }
                    track::DataMsg::SendLevel(msg) => {
                        if msg.track_guid == self.selected_track_guid {
                            // Only process send index messages for the currently selected track
                        } else {
                            return ModeAction::None;
                        }
                        // Only send fader update if the send index is mapped to a target
                        if let Some(Some(guid)) = self
                            .hw_assignments
                            .get(msg.send_index as usize - self.channel_offset)
                        {
                            self.track_send_states.entry(*guid).or_default().level = msg.level;

                            let fader_value = msg.level; // TODO: scale appropriately
                            io.send_to_v1m(
                                v1m::ChannelFaderMsg {
                                    idx: msg.send_index,
                                    value: fader_value as f64,
                                }
                                .into(),
                            )
                        }
                        ModeAction::None
                    }
                    track::DataMsg::SendPan(msg) => {
                        if msg.track_guid == self.selected_track_guid {
                            // Only process send index messages for the currently selected track
                        } else {
                            return ModeAction::None;
                        }
                        // Only send encoder update if the send index is mapped to a target
                        if let Some(Some(guid)) = self
                            .hw_assignments
                            .get(msg.send_index as usize - self.channel_offset)
                        {
                            self.track_send_states.entry(*guid).or_default().pan = msg.pan;

                            io.send_to_v1m(
                                v1m::EncoderRingMsg::new(
                                    msg.send_index,
                                    v1m::EncoderRingMode::FromCenter,
                                    msg.pan,
                                )
                                .into(),
                            )
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

    fn handle_msg_from_downstream(
        &mut self,
        msg: v1m::UpstreamMsg,
        io: &mut dyn DownstreamIo,
    ) -> ModeAction {
        match msg {
            v1m::UpstreamMsg::GlobalPress => {
                ModeAction::Transition(TransitionRequest::ToReaperVolumePan {
                    // TODO: is there something smarter to do here?
                    offset: 1,
                    selected_track_guid: Some(self.selected_track_guid),
                })
            }
            v1m::UpstreamMsg::MIDITracksPress => ModeAction::None,
            v1m::UpstreamMsg::InputsPress => {
                ModeAction::Transition(TransitionRequest::ToReaperChannelStrip {
                    // TODO: is there something smarter to do here?
                    offset: 0,
                    selected_track_guid: self.selected_track_guid,
                })
            }
            // If a new track is selected, we need to initiate a mode transition so that the
            // widgets are controlling the new track
            //
            // TODO: do we need to handle this case separately or do we simply expect a reflected
            // message back from Reaper?
            v1m::UpstreamMsg::SelectPress(msg) => {
                if let Some(guid) = self.get_guid_for_hw_channel(msg.idx as usize) {
                    if guid != self.selected_track_guid {
                        io.send_to_reaper(
                            track::Selected {
                                track_guid: guid,
                                selected: true,
                            }
                            .into(),
                        );
                        return ModeAction::Transition(TransitionRequest::ToReaperSends {
                            offset: self.channel_offset,
                            selected_track_guid: guid,
                        });
                    }
                }
                ModeAction::None
            }
            v1m::UpstreamMsg::ChannelFader(fader_msg) => {
                if self
                    .get_guid_for_hw_channel(fader_msg.idx as usize)
                    .is_some()
                {
                    io.send_to_reaper(
                        track::SendLevel {
                            track_guid: self.selected_track_guid,
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
