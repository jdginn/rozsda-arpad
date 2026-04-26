use crossbeam_channel::{Receiver, Sender};

use crate::midi::xtouch;
use crate::modes::mode_manager::{Mode, ModeHandler, ModeState, State};
use crate::modes::reaper_channel_strip_router::{ChannelStripMsg, ChannelStripRouter};
use crate::modes::reaper_channel_strip_widgets as widgets;
use crate::modes::reaper_faders_buttons_core::VolumeFadersCore;
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
    fn new(to_downstream: Sender<xtouch::DownstreamMsg>) -> Self {
        Widgets {
            hp_filter: widgets::ChannelWidget::new(to_downstream.clone()),
            low_freq: widgets::ChannelWidget::new(to_downstream.clone()),
            low_gain: widgets::ChannelWidget::new(to_downstream.clone()),
            lm_freq: widgets::ChannelWidget::new(to_downstream.clone()),
            lm_gain: widgets::ChannelWidget::new(to_downstream.clone()),
            hm_freq: widgets::ChannelWidget::new(to_downstream.clone()),
            hm_gain: widgets::ChannelWidget::new(to_downstream.clone()),
            high_freq: widgets::ChannelWidget::new(to_downstream.clone()),
            high_gain: widgets::ChannelWidget::new(to_downstream.clone()),
            eq_pos: widgets::ChannelWidget::new(to_downstream.clone()),
            comp_thresh: widgets::ChannelWidget::new(to_downstream.clone()),
            comp_ratio: widgets::ChannelWidget::new(to_downstream.clone()),
            comp_makeup: widgets::ChannelWidget::new(to_downstream.clone()),
            comp_type: widgets::ChannelWidget::new(to_downstream.clone()),
            saturation: widgets::ChannelWidget::new(to_downstream.clone()),
            gain: widgets::ChannelWidget::new(to_downstream.clone()),
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

    fn handle_message_from_downstream(&mut self, msg: xtouch::UpstreamMsg) {
        self.hp_filter.handle_message_from_downstream(msg);
        self.low_freq.handle_message_from_downstream(msg);
        self.low_gain.handle_message_from_downstream(msg);
        self.lm_freq.handle_message_from_downstream(msg);
        self.lm_gain.handle_message_from_downstream(msg);
        self.hm_freq.handle_message_from_downstream(msg);
        self.hm_gain.handle_message_from_downstream(msg);
        self.high_freq.handle_message_from_downstream(msg);
        self.high_gain.handle_message_from_downstream(msg);
        self.eq_pos.handle_message_from_downstream(msg);
        self.comp_thresh.handle_message_from_downstream(msg);
        self.comp_ratio.handle_message_from_downstream(msg);
        self.comp_makeup.handle_message_from_downstream(msg);
        self.comp_type.handle_message_from_downstream(msg);
        self.saturation.handle_message_from_downstream(msg);
        self.gain.handle_message_from_downstream(msg);
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
    router: ChannelStripRouter,
    widgets: Widgets,
    to_reaper: Sender<TrackMsg>,
    _from_reaper: Receiver<TrackMsg>,
    to_xtouch: Sender<xtouch::DownstreamMsg>,
    _from_xtouch: Receiver<xtouch::UpstreamMsg>,
}

impl ChannelStripMode {
    pub fn new(
        num_channels: usize,
        from_reaper: Receiver<TrackMsg>,
        to_reaper: Sender<TrackMsg>,
        from_xtouch: Receiver<xtouch::UpstreamMsg>,
        to_xtouch: Sender<xtouch::DownstreamMsg>,
    ) -> Self {
        ChannelStripMode {
            core: VolumeFadersCore::new(num_channels),
            router: ChannelStripRouter::new(),
            widgets: Widgets::new(to_xtouch.clone()),
            to_reaper,
            _from_reaper: from_reaper,
            to_xtouch,
            _from_xtouch: from_xtouch,
        }
    }
}

impl ModeHandler<TrackMsg, TrackMsg, xtouch::DownstreamMsg, xtouch::UpstreamMsg>
    for ChannelStripMode
{
    fn handle_messages_from_upstream(&mut self, msg: TrackMsg, curr_mode: ModeState) -> ModeState {
        match TrackDataMsg::try_from(msg) {
            Err(TrackMsg::Barrier(barrier)) => {
                // Forward barriers downstream (they need to reflect back upstream for the mode to
                // transition)
                self.to_xtouch
                    .send(xtouch::DownstreamMsg::Barrier(barrier))
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
                self.core
                    .handle_message_from_upstream(msg.clone(), self.to_xtouch.clone(), |_| {});
                if let Ok(translated_msg) = self.router.translate_message_from_upstream(msg) {
                    self.widgets.handle_message_from_upstream(translated_msg);
                };
                // Ignore unhandled payloads (e.g., Selected, SendIndex, etc.)
                curr_mode
            }
            Err(_) => {
                // Ignore messages that fail to parse as TrackDataMsg (e.g., ModeTransition, etc.)
                curr_mode
            }
        }
    }
    fn handle_messages_from_downstream(
        &mut self,
        msg: xtouch::UpstreamMsg,
        curr_mode: ModeState,
    ) -> ModeState {
        match msg {
            // If we were already waiting on a barrier from downstream, check if this is the one
            // we were waiting for. If yes, the state transition is finished.
            //
            // Note, we do not need to forward this barrier onward, since the hardware is not
            // allowed to reflect barriers back upstream.
            xtouch::UpstreamMsg::Barrier(barrier) => {
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
            // GlobalPress maps to ReaperVolPan mode
            xtouch::UpstreamMsg::GlobalPress => ModeState {
                mode: Mode::ReaperVolPan,
                state: State::RequestingModeTransition,
            },
            // MIDITracksPress maps to ReaperSends mode
            xtouch::UpstreamMsg::MIDITracksPress => ModeState {
                mode: Mode::ReaperSends,
                state: State::RequestingModeTransition,
            },
            xtouch::UpstreamMsg::InputsPress => curr_mode, // Inputs maps to this mode!
            _ => {
                self.core.handle_message_from_downstream(
                    msg,
                    self.to_reaper.clone(),
                    self.to_xtouch.clone(),
                );
                self.widgets.handle_message_from_downstream(msg);
                if let Some(translated_msg) = self.router.translate_message_from_downstream(msg) {
                    self.to_reaper.send(translated_msg);
                }
                curr_mode
            }
        }
    }
}
