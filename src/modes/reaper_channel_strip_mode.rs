use std::collections::HashMap;

use uuid::Uuid;

use crate::midi::v1m;
use crate::modes::mode_manager::{
    DownstreamIo, ModeAction, ModeHandler, TransitionRequest, UpstreamIo,
};
use crate::modes::reaper_channel_strip_router::{ChannelStripMsg, ChannelStripRouter};
use crate::modes::reaper_channel_strip_widgets as widgets;
use crate::modes::reaper_channel_strip_widgets::Widget;
use crate::modes::reaper_faders_buttons_core::VolumeFadersCore;
use crate::track::track;

struct Widgets {
    hp_filter: widgets::HpfWidget,
    low_freq: widgets::LowFreqWidget,
    // low_gain: widgets::LowGainWidget,
    // lm_freq: widgets::LmFreqWidget,
    // lm_gain: widgets::LmGainWidget,
    // hm_freq: widgets::HmFreqWidget,
    // hm_gain: widgets::HmGainWidget,
    // high_freq: widgets::HighFreqWidget,
    // high_gain: widgets::HighGainWidget,
    // eq_pos: widgets::EqPosWidget,
    // comp_thresh: widgets::CompThreshWidget,
    // comp_ratio: widgets::CompRatioWidget,
    // comp_makeup: widgets::CompMakeupWidget,
    // comp_type: widgets::CompTypeWidget,
    // saturation: widgets::SaturationWidget,
    // gain: widgets::GainWidget,
}

impl Widgets {
    fn new() -> Self {
        Widgets {
            hp_filter: widgets::HpfWidget::new(),
            low_freq: widgets::LowFreqWidget::new(),
            // low_gain: widgets::Widget::new(),
            // lm_freq: widgets::Widget::new(),
            // lm_gain: widgets::Widget::new(),
            // hm_freq: widgets::Widget::new(),
            // hm_gain: widgets::Widget::new(),
            // high_freq: widgets::Widget::new(),
            // high_gain: widgets::Widget::new(),
            // eq_pos: widgets::Widget::new(),
            // comp_thresh: widgets::Widget::new(),
            // comp_ratio: widgets::Widget::new(),
            // comp_makeup: widgets::Widget::new(),
            // comp_type: widgets::Widget::new(),
            // saturation: widgets::Widget::new(),
            // gain: widgets::Widget::new(),
        }
    }

    fn at_index(&mut self, idx: usize) -> &mut dyn Widget {
        match idx {
            0 => &mut self.hp_filter,
            1 => &mut self.low_freq,
            // 2 => &mut self.low_gain,
            // 3 => &mut self.lm_freq,
            // 4 => &mut self.lm_gain,
            // 5 => &mut self.hm_freq,
            // 6 => &mut self.hm_gain,
            // 7 => &mut self.high_freq,
            // 8 => &mut self.high_gain,
            // 9 => &mut self.eq_pos,
            // 10 => &mut self.comp_thresh,
            // 11 => &mut self.comp_ratio,
            // 12 => &mut self.comp_makeup,
            // 13 => &mut self.comp_type,
            // 14 => &mut self.saturation,
            // 15 => &mut self.gain,
            _ => panic!("Invalid widget index: {}", idx),
        }
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut dyn Widget> {
        let Widgets {
            hp_filter,
            low_freq,
        } = self;

        [hp_filter as &mut dyn Widget, low_freq as &mut dyn Widget].into_iter()
    }

    pub fn iter(&self) -> impl Iterator<Item = &dyn Widget> {
        let Widgets {
            hp_filter,
            low_freq,
        } = self;

        [hp_filter as &dyn Widget, low_freq as &dyn Widget].into_iter()
    }
}
//
//     fn handle_message_from_upstream(&mut self, msg: ChannelStripMsg) {
//         self.hp_filter.handle_message_from_upstream(msg);
//         self.low_freq.handle_message_from_upstream(msg);
//         self.low_gain.handle_message_from_upstream(msg);
//         self.lm_freq.handle_message_from_upstream(msg);
//         self.lm_gain.handle_message_from_upstream(msg);
//         self.hm_freq.handle_message_from_upstream(msg);
//         self.hm_gain.handle_message_from_upstream(msg);
//         self.high_freq.handle_message_from_upstream(msg);
//         self.high_gain.handle_message_from_upstream(msg);
//         self.eq_pos.handle_message_from_upstream(msg);
//         self.comp_thresh.handle_message_from_upstream(msg);
//         self.comp_ratio.handle_message_from_upstream(msg);
//         self.comp_makeup.handle_message_from_upstream(msg);
//         self.comp_type.handle_message_from_upstream(msg);
//         self.saturation.handle_message_from_upstream(msg);
//         self.gain.handle_message_from_upstream(msg);
//     }
//
//     fn handle_message_from_downstream(&mut self, msg: v1m::UpstreamMsg) -> Vec<ChannelStripMsg> {
//         let mut responses = Vec::new();
//
//         responses.extend(self.hp_filter.handle_message_from_downstream(msg));
//         responses.extend(self.low_freq.handle_message_from_downstream(msg));
//         responses.extend(self.low_gain.handle_message_from_downstream(msg));
//         responses.extend(self.lm_freq.handle_message_from_downstream(msg));
//         responses.extend(self.lm_gain.handle_message_from_downstream(msg));
//         responses.extend(self.hm_freq.handle_message_from_downstream(msg));
//         responses.extend(self.hm_gain.handle_message_from_downstream(msg));
//         responses.extend(self.high_freq.handle_message_from_downstream(msg));
//         responses.extend(self.high_gain.handle_message_from_downstream(msg));
//         responses.extend(self.eq_pos.handle_message_from_downstream(msg));
//         responses.extend(self.comp_thresh.handle_message_from_downstream(msg));
//         responses.extend(self.comp_ratio.handle_message_from_downstream(msg));
//         responses.extend(self.comp_makeup.handle_message_from_downstream(msg));
//         responses.extend(self.comp_type.handle_message_from_downstream(msg));
//         responses.extend(self.saturation.handle_message_from_downstream(msg));
//         responses.extend(self.gain.handle_message_from_downstream(msg));
//
//         responses
//     }
// }

pub struct ChannelStripMode {
    core: VolumeFadersCore,

    routers: HashMap<Uuid, ChannelStripRouter>,
    selected_track_guid: Uuid,

    widgets: Widgets,
    scribble_line_1_dirty: bool,
    scribble_line_2_dirty: bool,
    color_dirty: bool,
}

impl ChannelStripMode {
    pub fn new(num_channels: usize, channel_offset: usize, selected_track_guid: Uuid) -> Self {
        ChannelStripMode {
            core: VolumeFadersCore::new(num_channels, channel_offset),
            routers: HashMap::new(),
            widgets: Widgets::new(),
            selected_track_guid,
            scribble_line_1_dirty: false,
            scribble_line_2_dirty: false,
            color_dirty: false,
        }
    }

    pub fn init(mut self, io: &mut dyn UpstreamIo) -> Self {
        self.core = self.core.init(io);
        self
    }
}

impl ModeHandler for ChannelStripMode {
    fn handle_msg_from_upstream(
        &mut self,
        msg: track::TrackMsg,
        io: &mut dyn UpstreamIo,
    ) -> ModeAction {
        match track::DataMsg::try_from(msg) {
            Ok(msg) => {
                match msg {
                    // If a new track is selected, initiate a mode transition to make widgets now
                    // point to that new track.
                    //
                    // TODO: not sure if this is true. Maybe we only want to change what we control
                    // from the control surface and not chase whatever is selected in Reaper;
                    // especially since Reaper can have multiple things selected, while the control
                    // surface is one-hot by design
                    track::DataMsg::Selected(msg) => {
                        if msg.selected {
                            ModeAction::Transition(TransitionRequest::ToReaperChannelStrip {
                                offset: self.core.offset(),
                                selected_track_guid: msg.track_guid,
                            })
                        } else {
                            ModeAction::None
                        }
                    }
                    _ => {
                        // First handle the functionality that is not unique to ChannelStripMode
                        // (e.g. volume on faders, mute/arm/solo buttons)
                        self.core
                            .handle_msg_from_upstream(msg.clone(), io, |_| None);
                        let router = self
                            .routers
                            .entry(self.selected_track_guid)
                            .or_insert(ChannelStripRouter::new(self.selected_track_guid));
                        // Each message from upstream may cause one or more ChannelStripMsgs
                        //
                        // ChannelStripMsgs are handled by the widgets themselves
                        if let Ok(translated_msgs) = router.translate_message_from_upstream(msg) {
                            for translated_msg in translated_msgs {
                                // self.widgets.handle_message_from_upstream(translated_msg);
                            }
                        };
                        // Ignore unhandled payloads (e.g., Selected, SendIndex, etc.)
                        ModeAction::None
                    }
                }
            }
            Err(_) => {
                // Ignore messages that fail to parse as track::DataMsg (e.g., ModeTransition, etc.)
                panic!("Failed to parse TrackMsg as track::DataMsg:");
            }
        }
    }
    fn handle_msg_from_downstream(
        &mut self,
        msg: v1m::UpstreamMsg,
        io: &mut dyn DownstreamIo,
    ) -> ModeAction {
        match msg {
            // Messages that touch widgets
            //
            // FIXME: Shifts are placeholders on v1m, since we don't have a native shfit button.
            // Replace with something else (Master/Assign?)
            v1m::UpstreamMsg::ShiftPress => {
                for w in self.widgets.iter_mut() {
                    w.set_mode(widgets::ModeEvent::ShiftPress);
                }
                self.scribble_line_1_dirty = true;
                self.scribble_line_2_dirty = true;
                // TODO: should we just assume color needs to change?
                ModeAction::None
            }
            v1m::UpstreamMsg::ShiftRelease => {
                for w in self.widgets.iter_mut() {
                    w.set_mode(widgets::ModeEvent::ShiftRelease);
                }
                self.scribble_line_1_dirty = true;
                self.scribble_line_2_dirty = true;
                // TODO: should we just assume color needs to change?
                ModeAction::None
            }
            v1m::UpstreamMsg::EncoderPress(msg) => {
                self.widgets
                    .at_index(msg.idx as usize)
                    .set_mode(widgets::ModeEvent::EncoderPress);
                self.scribble_line_1_dirty = true;
                self.scribble_line_2_dirty = true;
                // TODO: should we just assume color needs to change?
                ModeAction::None
            }
            v1m::UpstreamMsg::EncoderRelease(msg) => {
                self.widgets
                    .at_index(msg.idx as usize)
                    .set_mode(widgets::ModeEvent::EncoderRelease);
                self.scribble_line_1_dirty = true;
                self.scribble_line_2_dirty = true;
                // TODO: should we just assume color needs to change?
                ModeAction::None
            }
            v1m::UpstreamMsg::EncoderTurnInc(msg) => {
                self.widgets
                    .at_index(msg.idx as usize)
                    .handle_encoder_event(widgets::EncoderEvent::EncoderTurn(
                        widgets::EncoderTurn::Inc { accel: msg.accel },
                    ));
                ModeAction::None
            }
            v1m::UpstreamMsg::EncoderTurnDec(msg) => {
                self.widgets
                    .at_index(msg.idx as usize)
                    .handle_encoder_event(widgets::EncoderEvent::EncoderTurn(
                        widgets::EncoderTurn::Dec { accel: msg.accel },
                    ));
                ModeAction::None
            }
            // Messages switch modes
            v1m::UpstreamMsg::GlobalPress => {
                ModeAction::Transition(TransitionRequest::ToReaperVolumePan {
                    offset: self.core.offset(),
                    selected_track_guid: Some(self.selected_track_guid),
                })
            }
            v1m::UpstreamMsg::MIDITracksPress => {
                ModeAction::Transition(TransitionRequest::ToReaperSends {
                    offset: 0,
                    selected_track_guid: self.selected_track_guid,
                })
            }
            v1m::UpstreamMsg::InputsPress => ModeAction::None,
            v1m::UpstreamMsg::SelectPress(msg) => {
                if let Some(guid) = self.core.get_guid_for_hw_channel(msg.idx as usize) {
                    if guid != self.selected_track_guid {
                        io.send_to_reaper(
                            track::Selected {
                                track_guid: guid,
                                selected: true,
                            }
                            .into(),
                        );
                        return ModeAction::Transition(TransitionRequest::ToReaperChannelStrip {
                            offset: self.core.offset(),
                            selected_track_guid: guid,
                        });
                    }
                }
                ModeAction::None
            }
            _ => {
                // Handle messages not specific to ChannelStripMode (e.g. faders, mute/arm/solo)
                self.core.handle_msg_from_downstream(msg, io);
                let router = self
                    .routers
                    .entry(self.selected_track_guid)
                    .or_insert(ChannelStripRouter::new(self.selected_track_guid));
                // Handle messages to the widgets
                // let channel_strip_msgs = self.widgets.handle_message_from_downstream(msg);
                let channel_strip_msgs = vec![];
                // Each upstream message may generate one or more ChannelStripMsgs
                for channel_strip_msg in channel_strip_msgs {
                    // Each channel_strip_msg may be translated into one or more reaper TrackMsgs
                    if let Ok(translated_msgs) =
                        router.translate_message_from_downstream(channel_strip_msg)
                    {
                        for translated_msg in translated_msgs {
                            // FIXME: unwrap
                            io.send_to_reaper(translated_msg);
                        }
                    }
                }
                ModeAction::None
            }
        }
    }
}
