use std::collections::HashMap;

use uuid::Uuid;

use crate::midi::v1m;
use crate::modes::mode_manager::{
    DownstreamIo, ModeAction, ModeHandler, TransitionRequest, UpstreamIo,
};
use crate::modes::reaper_channel_strip_router::{ChannelStripMsg, ChannelStripRouter};
use crate::modes::reaper_channel_strip_widgets as widgets;
use crate::modes::reaper_channel_strip_widgets::{Dirty, Widget};
use crate::modes::reaper_faders_buttons_core::VolumeFadersCore;
use crate::track::track;

struct Widgets {
    hp_filter: widgets::HpfWidget,
    low_freq: widgets::LowFreqWidget,
    low_gain: widgets::LowGainWidget,
    lm_freq: widgets::LMFreqWidget,
    lm_gain: widgets::LMGainWidget,
    hm_freq: widgets::HMFreqWidget,
    hm_gain: widgets::HMGainWidget,
    high_freq: widgets::HiFreqWidget,
    high_gain: widgets::HiGainWidget,
    eq_pos: widgets::EqPosWidget,
    comp_thresh: widgets::CompThreshWidget,
    comp_ratio: widgets::CompRatWidget,
    comp_makeup: widgets::CompMkpWidget,
    comp_type: widgets::CompTypeWidget,
    saturation: widgets::SatWidget,
    gain: widgets::GainWidget,
}

impl Widgets {
    fn new() -> Self {
        Widgets {
            hp_filter: widgets::HpfWidget::new(0),
            low_freq: widgets::LowFreqWidget::new(1),
            low_gain: widgets::Widget::new(2),
            lm_freq: widgets::Widget::new(3),
            lm_gain: widgets::Widget::new(4),
            hm_freq: widgets::Widget::new(5),
            hm_gain: widgets::Widget::new(6),
            high_freq: widgets::Widget::new(7),
            high_gain: widgets::Widget::new(8),
            eq_pos: widgets::Widget::new(9),
            comp_thresh: widgets::Widget::new(10),
            comp_ratio: widgets::Widget::new(11),
            comp_makeup: widgets::Widget::new(12),
            comp_type: widgets::Widget::new(13),
            saturation: widgets::Widget::new(14),
            gain: widgets::Widget::new(15),
        }
    }

    fn at_index(&mut self, idx: usize) -> &mut dyn Widget {
        match idx {
            0 => &mut self.hp_filter,
            1 => &mut self.low_freq,
            2 => &mut self.low_gain,
            3 => &mut self.lm_freq,
            4 => &mut self.lm_gain,
            5 => &mut self.hm_freq,
            6 => &mut self.hm_gain,
            7 => &mut self.high_freq,
            8 => &mut self.high_gain,
            9 => &mut self.eq_pos,
            10 => &mut self.comp_thresh,
            11 => &mut self.comp_ratio,
            12 => &mut self.comp_makeup,
            13 => &mut self.comp_type,
            14 => &mut self.saturation,
            15 => &mut self.gain,
            _ => panic!("Invalid widget index: {}", idx),
        }
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut dyn Widget> {
        let Widgets {
            hp_filter,
            low_freq,
            low_gain,
            lm_freq,
            lm_gain,
            hm_freq,
            hm_gain,
            high_freq,
            high_gain,
            eq_pos,
            comp_thresh,
            comp_ratio,
            comp_makeup,
            comp_type,
            saturation,
            gain,
        } = self;

        [
            hp_filter as &mut dyn Widget,
            low_freq as &mut dyn Widget,
            low_gain as &mut dyn Widget,
            lm_freq as &mut dyn Widget,
            lm_gain as &mut dyn Widget,
            hm_freq as &mut dyn Widget,
            hm_gain as &mut dyn Widget,
            high_freq as &mut dyn Widget,
            high_gain as &mut dyn Widget,
            eq_pos as &mut dyn Widget,
            comp_thresh as &mut dyn Widget,
            comp_ratio as &mut dyn Widget,
            comp_makeup as &mut dyn Widget,
            comp_type as &mut dyn Widget,
            saturation as &mut dyn Widget,
            gain as &mut dyn Widget,
        ]
        .into_iter()
    }

    pub fn iter(&self) -> impl Iterator<Item = &dyn Widget> {
        let Widgets {
            hp_filter,
            low_freq,
            low_gain,
            lm_freq,
            lm_gain,
            hm_freq,
            hm_gain,
            high_freq,
            high_gain,
            eq_pos,
            comp_thresh,
            comp_ratio,
            comp_makeup,
            comp_type,
            saturation,
            gain,
        } = self;

        [
            hp_filter as &dyn Widget,
            low_freq as &dyn Widget,
            low_gain as &dyn Widget,
            lm_freq as &dyn Widget,
            lm_gain as &dyn Widget,
            hm_freq as &dyn Widget,
            hm_gain as &dyn Widget,
            high_freq as &dyn Widget,
            high_gain as &dyn Widget,
            eq_pos as &dyn Widget,
            comp_thresh as &dyn Widget,
            comp_ratio as &dyn Widget,
            comp_makeup as &dyn Widget,
            comp_type as &dyn Widget,
            saturation as &dyn Widget,
            gain as &dyn Widget,
        ]
        .into_iter()
    }
}

pub struct ChannelStripMode {
    core: VolumeFadersCore,
    num_channels: usize,
    channel_offset: usize,

    routers: HashMap<Uuid, ChannelStripRouter>,
    selected_track_guid: Uuid,

    widgets: Widgets,
    dirty: widgets::Dirty,
}

impl ChannelStripMode {
    pub fn new(num_channels: usize, channel_offset: usize, selected_track_guid: Uuid) -> Self {
        ChannelStripMode {
            core: VolumeFadersCore::new(num_channels, channel_offset),
            num_channels,
            channel_offset,
            routers: HashMap::new(),
            widgets: Widgets::new(),
            selected_track_guid,
            dirty: widgets::Dirty::default()
                | widgets::Dirty::LINE1
                | widgets::Dirty::LINE2
                | widgets::Dirty::COLOR,
        }
    }

    pub fn init(mut self, io: &mut dyn DownstreamIo) -> Self {
        self.core = self.core.init(io);
        // Query full track state from Reaper to get fx info
        io.send_to_reaper(
            track::TrackQuery {
                guid: self.selected_track_guid,
            }
            .into(),
        );
        self
    }

    fn apply_downstream_outcome(
        &mut self,
        outcome: widgets::HandledDownstreamOutcome,
        io: &mut dyn DownstreamIo,
    ) {
        if let Some(snapshot) = outcome.snapshot {
            for w in self.widgets.iter_mut() {
                if let Some(o) = w.handle_snapshot(&snapshot) {
                    self.dirty |= o.dirty;
                    for m in o.downstream_msgs {
                        io.send_to_v1m(m);
                    }
                }
            }
        }
        self.dirty |= outcome.dirty;
        for m in outcome.downstream_msgs {
            match m {
                v1m::DownstreamMsg::EncoderRingLED(msg) => {
                    let mut new_msg = msg;
                    new_msg.idx -= self.channel_offset as i32;
                    io.send_to_v1m(new_msg.into());
                }
                _ => io.send_to_v1m(m),
            }
        }
        for m in outcome.upstream_msgs {
            println!("ChannelStripMode: sending upstream msg: {:?}", m);
            let translated_messages = self
                .routers
                .entry(self.selected_track_guid)
                .or_insert(ChannelStripRouter::new(self.selected_track_guid))
                .translate_message_from_downstream(m)
                .unwrap();
            for tm in translated_messages {
                println!("Sending translated message to Reaper: {:?}", tm);
                io.send_to_reaper(tm);
            }
        }
    }
}

impl ModeHandler for ChannelStripMode {
    fn on_tick(&mut self, io: &mut dyn DownstreamIo) -> ModeAction {
        if self.dirty.intersects(Dirty::LINE1) {
            let mut msgs = vec![];
            for (i, w) in self.widgets.iter().enumerate() {
                if i < self.channel_offset {
                    continue;
                }
                if i > self.channel_offset + self.num_channels {
                    break;
                }
                msgs.push(
                    v1m::TopScribbleStripLine1TextMsg {
                        idx: (i - self.channel_offset) as i32,
                        text: w.line1_text(),
                    }
                    .into(),
                );
            }
            io.send_to_v1m(v1m::DownstreamMsg::TopScribbleStripBatch(msgs));
        }
        if self.dirty.intersects(Dirty::LINE2) {
            let mut msgs = vec![];
            for (i, w) in self.widgets.iter().enumerate() {
                if i < self.channel_offset {
                    continue;
                }
                if i > self.channel_offset + self.num_channels {
                    break;
                }
                msgs.push(
                    v1m::TopScribbleStripLine2TextMsg {
                        idx: (i - self.channel_offset) as i32,
                        text: w.line2_text(),
                    }
                    .into(),
                );
            }
            io.send_to_v1m(v1m::DownstreamMsg::TopScribbleStripBatch(msgs));
        }
        if self.dirty.intersects(Dirty::COLOR) {
            let mut msgs = vec![];
            for (i, w) in self.widgets.iter().enumerate() {
                if i < self.channel_offset {
                    continue;
                }
                if i > self.channel_offset + self.num_channels {
                    break;
                }
                msgs.push(
                    v1m::TopScribbleStripColorMsg {
                        idx: (i - self.channel_offset) as i32,
                        color: v1m::Color {
                            r: w.color().red,
                            g: w.color().green,
                            b: w.color().blue,
                        },
                    }
                    .into(),
                );
            }
            io.send_to_v1m(v1m::DownstreamMsg::TopScribbleStripBatch(msgs));
        }
        self.dirty = widgets::Dirty::default();
        ModeAction::None
    }

    fn next_wake_deadline(&self) -> Option<std::time::Instant> {
        Some(std::time::Instant::now() + std::time::Duration::from_millis(50))
    }

    fn handle_msg_from_upstream(
        &mut self,
        msg: track::TrackMsg,
        io: &mut dyn UpstreamIo,
    ) -> ModeAction {
        match track::DataMsg::try_from(msg) {
            Ok(msg) => {
                // println!(
                //     "ChannelStripMode: received upstream msg: {:?}, selected_track_guid: {:?}",
                //     msg, self.selected_track_guid
                // );
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
                                offset: 1,
                                selected_track_guid: msg.track_guid,
                            })
                        } else {
                            ModeAction::None
                        }
                    }
                    track::DataMsg::FXName(msg) => {
                        println!("GOT FX NAME MSG: {:?}\n\n\n", msg);
                        self.routers
                            .entry(self.selected_track_guid)
                            .or_insert(ChannelStripRouter::new(self.selected_track_guid))
                            .update_plugin_state(msg.fx_index, &msg.name);
                        ModeAction::None
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
            v1m::UpstreamMsg::FlipPress => {
                let mut outcomes = Vec::new();
                for w in self.widgets.iter_mut() {
                    if let Some(outcome) = w.handle_shift_press() {
                        outcomes.push(outcome);
                    }
                }
                for outcome in outcomes {
                    self.apply_downstream_outcome(outcome, io);
                }
                ModeAction::None
            }
            v1m::UpstreamMsg::FlipRelease => {
                let mut outcomes = Vec::new();
                for w in self.widgets.iter_mut() {
                    if let Some(outcome) = w.handle_shift_release() {
                        outcomes.push(outcome);
                    }
                }
                for outcome in outcomes {
                    self.apply_downstream_outcome(outcome, io);
                }
                ModeAction::None
            }
            // TODO: add banking for 8 widgets at a time
            v1m::UpstreamMsg::EncoderClick(msg) => {
                let widget = self
                    .widgets
                    .at_index(msg.idx as usize + self.channel_offset);
                if let Some(outcome) = widget.handle_encoder_click() {
                    self.apply_downstream_outcome(outcome, io);
                }
                ModeAction::None
            }
            v1m::UpstreamMsg::EncoderTurnInc(msg) => {
                let widget = self
                    .widgets
                    .at_index(msg.idx as usize + self.channel_offset);
                if let Some(outcome) =
                    widget.handle_encoder_turn(widgets::EncoderTurn::Inc { accel: msg.accel })
                {
                    self.apply_downstream_outcome(outcome, io);
                }
                ModeAction::None
            }
            v1m::UpstreamMsg::EncoderTurnDec(msg) => {
                let widget = self
                    .widgets
                    .at_index(msg.idx as usize + self.channel_offset);
                if let Some(outcome) =
                    widget.handle_encoder_turn(widgets::EncoderTurn::Dec { accel: msg.accel })
                {
                    self.apply_downstream_outcome(outcome, io);
                }
                ModeAction::None
            }
            // Banking
            v1m::UpstreamMsg::BankLeft8 => {
                if self.channel_offset > 7 {
                    self.channel_offset -= 8;
                    self.dirty |= Dirty::LINE1 | Dirty::LINE2 | Dirty::COLOR;
                }
                ModeAction::None
            }

            v1m::UpstreamMsg::BankRight8 => {
                self.channel_offset += 8;
                self.dirty |= Dirty::LINE1 | Dirty::LINE2 | Dirty::COLOR;
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
