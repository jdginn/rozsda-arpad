use std::collections::HashMap;

use uuid::Uuid;

use crate::midi::v1m;
use crate::modes::mode_manager::{
    DownstreamIo, ModeAction, ModeHandler, TransitionRequest, UpstreamIo,
};
use crate::modes::reaper_channel_strip_router::{ChannelStripMsg, ChannelStripRouter};
use crate::modes::reaper_channel_strip_widgets as widgets;
use crate::modes::reaper_faders_buttons_core::VolumeFadersCore;
use crate::track::track;

struct Widgets {
    hp_filter: widgets::ChannelWidget<widgets::HPWidgetBehavior>,
    low_freq: widgets::ChannelWidget<widgets::LowFreqWidgetBehavior>,
    low_gain: widgets::ChannelWidget<widgets::LowGainWidgetBehavior>,
    lm_freq: widgets::ChannelWidget<widgets::LmFreqWidgetBehavior>,
    lm_gain: widgets::ChannelWidget<widgets::LmGainWidgetBehavior>,
    hm_freq: widgets::ChannelWidget<widgets::HmFreqWidgetBehavior>,
    hm_gain: widgets::ChannelWidget<widgets::HmGainWidgetBehavior>,
    high_freq: widgets::ChannelWidget<widgets::HighFreqWidgetBehavior>,
    high_gain: widgets::ChannelWidget<widgets::HighGainWidgetBehavior>,
    eq_pos: widgets::ChannelWidget<widgets::EqPosWidgetBehavior>,
    comp_thresh: widgets::ChannelWidget<widgets::CompThreshWidgetBehavior>,
    comp_ratio: widgets::ChannelWidget<widgets::CompRatioWidgetBehavior>,
    comp_makeup: widgets::ChannelWidget<widgets::CompMakeupWidgetBehavior>,
    comp_type: widgets::ChannelWidget<widgets::CompTypeWidgetBehavior>,
    saturation: widgets::ChannelWidget<widgets::SaturationWidgetBehavior>,
    gain: widgets::ChannelWidget<widgets::GainWidgetBehavior>,
}

impl Widgets {
    fn new() -> Self {
        Widgets {
            hp_filter: widgets::ChannelWidget::new(),
            low_freq: widgets::ChannelWidget::new(),
            low_gain: widgets::ChannelWidget::new(),
            lm_freq: widgets::ChannelWidget::new(),
            lm_gain: widgets::ChannelWidget::new(),
            hm_freq: widgets::ChannelWidget::new(),
            hm_gain: widgets::ChannelWidget::new(),
            high_freq: widgets::ChannelWidget::new(),
            high_gain: widgets::ChannelWidget::new(),
            eq_pos: widgets::ChannelWidget::new(),
            comp_thresh: widgets::ChannelWidget::new(),
            comp_ratio: widgets::ChannelWidget::new(),
            comp_makeup: widgets::ChannelWidget::new(),
            comp_type: widgets::ChannelWidget::new(),
            saturation: widgets::ChannelWidget::new(),
            gain: widgets::ChannelWidget::new(),
        }
    }

    fn handle_message_from_upstream(&mut self, msg: ChannelStripMsg) {
        self.hp_filter.handle_message_from_upstream(msg);
        self.low_freq.handle_message_from_upstream(msg);
        self.low_gain.handle_message_from_upstream(msg);
        self.lm_freq.handle_message_from_upstream(msg);
        self.lm_gain.handle_message_from_upstream(msg);
        self.hm_freq.handle_message_from_upstream(msg);
        self.hm_gain.handle_message_from_upstream(msg);
        self.high_freq.handle_message_from_upstream(msg);
        self.high_gain.handle_message_from_upstream(msg);
        self.eq_pos.handle_message_from_upstream(msg);
        self.comp_thresh.handle_message_from_upstream(msg);
        self.comp_ratio.handle_message_from_upstream(msg);
        self.comp_makeup.handle_message_from_upstream(msg);
        self.comp_type.handle_message_from_upstream(msg);
        self.saturation.handle_message_from_upstream(msg);
        self.gain.handle_message_from_upstream(msg);
    }

    fn handle_message_from_downstream(&mut self, msg: v1m::UpstreamMsg) -> Vec<ChannelStripMsg> {
        let mut responses = Vec::new();

        responses.extend(self.hp_filter.handle_message_from_downstream(msg));
        responses.extend(self.low_freq.handle_message_from_downstream(msg));
        responses.extend(self.low_gain.handle_message_from_downstream(msg));
        responses.extend(self.lm_freq.handle_message_from_downstream(msg));
        responses.extend(self.lm_gain.handle_message_from_downstream(msg));
        responses.extend(self.hm_freq.handle_message_from_downstream(msg));
        responses.extend(self.hm_gain.handle_message_from_downstream(msg));
        responses.extend(self.high_freq.handle_message_from_downstream(msg));
        responses.extend(self.high_gain.handle_message_from_downstream(msg));
        responses.extend(self.eq_pos.handle_message_from_downstream(msg));
        responses.extend(self.comp_thresh.handle_message_from_downstream(msg));
        responses.extend(self.comp_ratio.handle_message_from_downstream(msg));
        responses.extend(self.comp_makeup.handle_message_from_downstream(msg));
        responses.extend(self.comp_type.handle_message_from_downstream(msg));
        responses.extend(self.saturation.handle_message_from_downstream(msg));
        responses.extend(self.gain.handle_message_from_downstream(msg));

        responses
    }
}

pub struct ChannelStripMode {
    core: VolumeFadersCore,
    routers: HashMap<Uuid, ChannelStripRouter>,
    widgets: Widgets,
    selected_track_guid: Uuid,
}

impl ChannelStripMode {
    pub fn new(num_channels: usize, channel_offset: usize, selected_track_guid: Uuid) -> Self {
        ChannelStripMode {
            core: VolumeFadersCore::new(num_channels, channel_offset),
            routers: HashMap::new(),
            widgets: Widgets::new(),
            selected_track_guid,
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
                                self.widgets.handle_message_from_upstream(translated_msg);
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
                let channel_strip_msgs = self.widgets.handle_message_from_downstream(msg);
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
