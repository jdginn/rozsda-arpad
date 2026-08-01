use std::collections::HashMap;

use crossbeam_channel::{Receiver, Sender};
use uuid::Uuid;

use crate::midi::v1m;
use crate::modes::mode_manager::{Mode, ModeAction, ModeHandler, Senders, TransitionRequest};
use crate::modes::reaper_channel_strip_router::{ChannelStripMsg, ChannelStripRouter};
use crate::modes::reaper_channel_strip_widgets as widgets;
use crate::modes::reaper_faders_buttons_core::VolumeFadersCore;
use crate::track::track;
use crate::track::track::{DataMsg as TrackDataMsg, TrackMsg};

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

/// Implements a mode where the faders and Arm/Mute/Solo/Select buttons behave the same as VolumePanMode
/// but the encoders and scribble strpes expose key tone-shaping functions like EQ, Compression, Saturation, etc.
///
/// The encoders ONLY control the selected track, and only control one track at a time. Track
/// selection still follows reaper and still responds to the select buttons on the surface.
///
/// Encoders support multiple behaviors, with each encoder supporting up to the following:
/// 1. Turn the encoder without pressing anything
/// 2. Turn the encoder WHILE holding it down
/// 3. Turn the encoder WHILE holding down a modifier button (e.g., Shift)
/// 4. Turn the encoder WHILE holding down both the modifier AND pressing down the encoder
/// 5. Click the encoder (turning does nothing)
/// 6. Click the encoder WHILE holding down a modifier button (e.g., Shift -- turning does nothing)
///
/// In each case, the behavior updates the scribble strip to indicate what parameter is being controlled and the encoder ring to indicate the current value.
///
/// This mode assumes 16 encoders are available. The encoders have the following functions:
/// | #  | Normal      | Pressed                          | Shift            | Shift+Pressed  | Click          | Shift+Click     |
/// |----|-------------|----------------------------------|------------------|----------------|--------------- |-----------------|
/// | 1  | HP filter   | slope                            | EQ type          |                |                |                 |
/// | 2  | Low freq    | Low Q (bell) / slope (shelf)     | bell/shelf       |                |                |                 |
/// | 3  | Low gain    |                                  |                  |                | zero Low gain  |                 |
/// | 4  | LM freq     | LM Q                             |                  |                |                |                 |
/// | 5  | LM gain     |                                  |                  |                | zero LM gain   |                 |
/// | 6  | HM freq     | HM Q                             |                  |                |                |                 |
/// | 7  | HM gain     |                                  |                  |                | zero HM gain   |                 |
/// | 8  | High freq   | High Q (bell) / slope (slope)    | bell/shelf       |                |                |                 |
/// | 9  | High gain   |                                  | sides gain       |                | zero High gain | zero sides gain |
/// | 10 | EQ pos      |                                  | Comp order       |                | bypass EQ      |                 |
/// | 11 | Comp thresh | Comp SC filter                   | Comp2  thresh    | Comp2 SC filt  |                |                 |
/// | 12 | Comp ratio  | Comp attack                      | Comp2  ratio     | Comp2 attack   |                |                 |
/// | 13 | Comp makeup | Comp release                     | Comp2  makeup    | Comp2 release  |                |                 |
/// | 14 | Comp type   |                                  | Comp2  type      |                | bypass Comp    | bypass Comp2    |
/// | 15 | Saturation  |                                  | Saturation type  |                | bypass Sat     |                 |
/// | 16 | Gain        | Interface gain (only if armed    | Trim             |                |                |                 |
///
/// Notes on specific controls:
/// - By default, EQ is engaged, both compressors and saturation are bypassed.
/// - EQ type selects between EQ plugins with EQUIVALENT features. It may allow e.g. colourless EQ, SSL-style, Neve-style, etc.
/// - Depending on EQ type, Q may or may not take effect.
/// - Sides gain applies the "High" band only to the sides in a mid-side EQ. This value is offset from the main high gain.
/// - EQ pos sets the position of EQ in the signal chain. Modes:
///    - "FIRST": Gain -> EQ -> Comp -> Comp -> Saturation -> Trim
///    - "MIDDLE": Gain -> Comp -> Comp -> EQ -> Saturation -> Trim
///    - "LAST": Gain -> Comp -> Comp -> Saturation -> EQ -> Trim
/// - Comp order sets the ordering of compressors. Modes:
///    - "F->S": Comp -> Comp2
///    - "S->F": Comp2 -> Comp1
/// - Comp and Comp2 are separate compressors and controlled fully independently.
/// - Comp is a "fast", FET-style compressor. Comp2 is a "slow" optical-style compressor.
/// - Comp type selects between comprssors germain to the two categories above. Examples:
///     - Comp1: 1176, Distressor, Digital, SSL, API
///     - Comp2: LA2A, LA3A, Vari-MU, etc.
/// - For compressor types that do not have a threshold control, Comp thresh maps to input gain.
/// - Some compressor tyeps do not have a ratio control.
/// - For compressor types that do not have a ratio control, Comp makeup maps to output gain.
/// - Some compressor types lack attack and release controls.
/// - Comp SC filter is a high-pass filter on the compressor sidechain.
/// - Saturation type selects between various console, tape simulators up to full-on distortion.
/// - Gain adjusts level entering the channel strip, before any processing.
/// - Trim adjust level leaving the channel strip.
/// - Interface gain adjusts the gain at the audio interface, if the selected tack is armed. This does not affect recorded material.
pub struct ChannelStripMode {
    core: VolumeFadersCore,
    routers: HashMap<Uuid, ChannelStripRouter>,
    widgets: Widgets,
    selected_track_guid: Uuid,
}

impl ChannelStripMode {
    pub fn new(num_channels: usize, selected_track_guid: Uuid) -> Self {
        ChannelStripMode {
            core: VolumeFadersCore::new(num_channels),
            routers: HashMap::new(),
            widgets: Widgets::new(),
            selected_track_guid,
        }
    }
}

impl ModeHandler for ChannelStripMode {
    fn handle_msg_from_upstream(&mut self, msg: TrackMsg, senders: &Senders) -> ModeAction {
        match TrackDataMsg::try_from(msg) {
            Ok(msg) => {
                match msg {
                    // If a new track is selected, initiate a mode transition to make widgets now
                    // point to that new track.
                    TrackDataMsg::Selected(msg) => {
                        if msg.selected {
                            ModeAction::Transition(TransitionRequest::ToReaperChannelStrip {
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
                            .handle_msg_from_upstream(msg.clone(), senders, |_| {});
                        let router = self
                            .routers
                            .entry(self.selected_track_guid)
                            .or_insert(ChannelStripRouter::new(self.selected_track_guid));
                        // Each message from upstream may cause one or more ChannelStripMsgs
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
                // Ignore messages that fail to parse as TrackDataMsg (e.g., ModeTransition, etc.)
                panic!("Failed to parse TrackMsg as TrackDataMsg:");
            }
        }
    }
    fn handle_msg_from_downstream(
        &mut self,
        msg: v1m::UpstreamMsg,
        senders: &Senders,
    ) -> ModeAction {
        match msg {
            // GlobalPress maps to ReaperVolPan mode
            v1m::UpstreamMsg::GlobalPress => {
                ModeAction::Transition(TransitionRequest::ToReaperVolumePan {
                    selected_track_guid: Some(self.selected_track_guid),
                })
            }
            // MIDITracksPress maps to ReaperSends mode
            v1m::UpstreamMsg::MIDITracksPress => {
                ModeAction::Transition(TransitionRequest::ToReaperSends {
                    selected_track_guid: self.selected_track_guid,
                })
            }
            v1m::UpstreamMsg::InputsPress => ModeAction::None,
            v1m::UpstreamMsg::SelectPress(msg) => {
                if let Some(guid) = self.core.get_guid_for_hw_channel(msg.idx as usize) {
                    if guid != self.selected_track_guid {
                        senders.send_to_reaper(
                            track::Selected {
                                track_guid: guid,
                                selected: true,
                            }
                            .into(),
                        );
                        return ModeAction::Transition(TransitionRequest::ToReaperSends {
                            selected_track_guid: guid,
                        });
                    }
                }
                ModeAction::None
            }
            _ => {
                // Handle messages not specific to ChannelStripMode (e.g. faders, mute/arm/solo)
                self.core.handle_msg_from_downstream(msg, senders);
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
                            senders.send_to_reaper(translated_msg);
                        }
                    }
                }
                ModeAction::None
            }
        }
    }
}
