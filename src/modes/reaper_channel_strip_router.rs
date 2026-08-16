use std::sync::Mutex;

use uuid::Uuid;

use crate::modes::generated_fx_param as fx;
use crate::modes::reaper_channel_strip_widgets::EncoderTurn;
use crate::track::track;

use crate::modes::generated_fx_param::rea_eq;

macro_rules! rotary_enum {
    (
        $(#[$meta:meta])*
        $vis:vis enum $Name:ident {
            $($Variant:ident => $label:expr),+ $(,)?
        }
    ) => {
        $(#[$meta])*
        $vis enum $Name { $($Variant),+ }

        impl $Name {
            const ALL: &'static [Self] = &[$(Self::$Variant),+];

            pub fn next(self) -> Self {
                let i = Self::ALL.iter().position(|v| *v == self).unwrap();
                Self::ALL[(i + 1) % Self::ALL.len()]
            }

            pub fn prev(self) -> Self {
                let i = Self::ALL.iter().position(|v| *v == self).unwrap();
                Self::ALL[(i + Self::ALL.len() - 1) % Self::ALL.len()]
            }

            pub fn step(self, dir: EncoderTurn) -> Self {
                match dir {
                    EncoderTurn::Inc {..} => self.next(),
                    EncoderTurn::Dec {..} => self.prev(),
                }
            }

            pub fn as_str(self) -> &'static str {
                match self {
                    $(Self::$Variant => $label),+
                }
            }
        }
    }
}

// | #  | Normal      | Pressed                          | Shift            | Shift+Pressed  | Click          | Shift+Click     |
// |----|-------------|----------------------------------|------------------|----------------|--------------- |-----------------|
// | 1  | HP filter   | slope                            | EQ type          |                |                |                 |
// | 2  | Low freq    | Low Q (bell) / slope (shelf)     | bell/shelf       |                |                |                 |
// | 3  | Low gain    |                                  |                  |                | zero Low gain  |                 |
// | 4  | LM freq     | LM Q                             |                  |                |                |                 |
// | 5  | LM gain     |                                  |                  |                | zero LM gain   |                 |
// | 6  | HM freq     | HM Q                             |                  |                |                |                 |
// | 7  | HM gain     |                                  |                  |                | zero HM gain   |                 |
// | 8  | High freq   | High Q (bell) / slope (slope)    | bell/shelf       |                |                |                 |
// | 9  | High gain   |                                  | sides gain       |                | zero High gain | zero sides gain |
// | 10 | EQ pos      |                                  | Comp order       |                | bypass EQ      |                 |
// | 11 | Comp thresh | Comp SC filter                   | Comp2  thresh    | Comp2 SC filt  |                |                 |
// | 12 | Comp ratio  | Comp attack                      | Comp2  ratio     | Comp2 attack   |                |                 |
// | 13 | Comp makeup | Comp release                     | Comp2  makeup    | Comp2 release  |                |                 |
// | 14 | Comp type   |                                  | Comp2  type      |                | bypass Comp    | bypass Comp2    |
// | 15 | Saturation  |                                  | Saturation type  |                | bypass Sat     |                 |
// | 16 | Gain        | Interface gain (only if armed)   | Trim             |                |                |                 |

rotary_enum! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum BandMode {
        Bell => "bell",
        Shelf => "shelf",
    }
}

rotary_enum! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum EqType {
        Digital => "Digital",
        SSL => "SSL",
        Neve => "Neve",
    }
}

rotary_enum! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum EqPosition {
        First => "E>C>C>S",
        Middle => "C>C>E>S",
        Last => "C>C>S>E",
    }
}

rotary_enum! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum CompOrder {
        C1toC2 => "Cmp1->2",
        C2toC1 => "Cmp2->1",
    }
}

rotary_enum! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum CompType {
        Digital => "Digital",
        Eleven76 => "1176",
        LA2A => "LA2A",
        Distressor => "Distrsr",
        LA3A => "LA3A",
        VariMu => "VariMu",
    }
}

rotary_enum! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum EqBypass {
        IN => "EqIN",
        OUT => "EqOUT",
    }
}

rotary_enum! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum CompBypass {
        IN => "CompIN",
        OUT => "CompOUT",
    }
}

rotary_enum! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Cmp2Bypass {
        Engaged => "Cmp2IN",
        Bypassed => "Cmp2OUT",
    }
}

rotary_enum! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum SaturationBypass {
        IN => "SatIN",
        OUT => "SatOUT",
    }
}

rotary_enum! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum SaturationType {
        Console => "Console",
        Tape => "Tape",
        Decapitator => "Decap",
        BitCrush => "BitCrsh",
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ChannelStripMsg {
    EnableEq(EqType),
    DisableEq,
    EnableComp1,
    DisableComp1,
    EnableComp2,
    DisableComp2,
    EnableSaturation,
    DisableSaturation,
    EnableGain,
    DisableGain,
    EnableTrim,
    DisableTrim,
    EnableInterfaceGain,
    DisableInterfaceGain,
    // InstantiateEq(EqFx)
    // InstantiateComp1(CompFx)
    // InstantiateComp2(CompFx)
    // InstantiateSaturation(SaturationFx)
    // InstantiateGain(GainFx)
    // InstantiateTrim(TrimRx)
    // InstantiateInterface(Interface)
    HpfFreq(f32),
    HpfSlope(f32),
    EqType(EqType),
    LowFreq(f32),
    LowQ(f32),
    LowSlope(f32),
    LowBandMode(BandMode),
    LowGain(f32),
    LmFreq(f32),
    LmQ(f32),
    LmGain(f32),
    HmFreq(f32),
    HmQ(f32),
    HmGain(f32),
    HighFreq(f32),
    HighQ(f32),
    HighSlope(f32),
    HighBandMode(BandMode),
    HighGain(f32),
    HighSidesGain(f32),
    EqPos(EqPosition),
    CompOrder(CompOrder),
    EqBypass(EqBypass),
    CompThresh(f32),
    CompScFilter(f32),
    Comp2Thresh(f32),
    Comp2ScFilter(f32),
    CompRatio(f32),
    CompAttack(f32),
    Comp2Ratio(f32),
    Comp2Attack(f32),
    CompMakeup(f32),
    CompRelease(f32),
    Comp2Makeup(f32),
    Comp2Release(f32),
    CompType(CompType),
    Comp2Type(CompType),
    CompBypass(CompBypass),
    Comp2Bypass(Cmp2Bypass),
    Saturation(f32),
    SaturationType(SaturationType),
    SaturationBypass(SaturationBypass),
    Gain(f32),
    Trim(f32),
    InterfaceGain(f32),
}

#[derive(Debug, Clone, Copy)]
pub enum TranslationErr {
    Dummy,
}

// ----
//
// ----

struct FXParamIdent {
    fx_index: i32,
    param_index: i32,
}

/// Maps named, high-level channel strip concepts to their respective parameters
///
/// NOTE: we have one of these *PER TRACK*
///
/// TODO: the hard part will be getting this to update dynamically based on the actual FX chain on the track
pub struct ChannelStripRouter {
    mux: Mutex<()>,
    track_guid: Uuid,
    plugin_names_by_index: Vec<String>,
    plugins_by_index: Vec<fx::FX>,
}

// TODO: FIXME: URGENT: need to initialize this when we transition into the new mode and think about
// barriers, etc. This should work the same as all the other mode transitions, we just need to be sure we process the messages.
//
// Actually, I think we don't need to do anything special here since the way barriers work means we
// will get all the state updates though the "normal" handle_msg_from_upstream "channel", and the
// way we designed ModeManager should make it all just work.

impl ChannelStripRouter {
    pub fn new(track_guid: Uuid) -> Self {
        ChannelStripRouter {
            mux: Mutex::new(()),
            track_guid,
            plugins_by_index: Vec::new(),
            plugin_names_by_index: Vec::new(),
        }
    }

    pub fn update_plugin_state(&mut self, plugin_index: i32, plugin_name: &str) {
        println!(
            "Updating plugin state for index {}: {}",
            plugin_index, plugin_name
        );
        let _lock = self.mux.lock().unwrap();
        if (plugin_index as usize) >= self.plugin_names_by_index.len() {
            self.plugin_names_by_index
                .resize((plugin_index + 1) as usize, String::new());
        }
        self.plugin_names_by_index[plugin_index as usize] = plugin_name.to_string();
    }

    fn update_mapping_locked(&mut self) {}

    pub fn translate_message_from_upstream(
        &self,
        msg: track::DataMsg,
    ) -> Result<Vec<ChannelStripMsg>, TranslationErr> {
        match msg {
            track::DataMsg::FXParamValue(msg) => {
                if let Some(msg) = fx::rea_eq::decode_trackmsg(msg) {
                    match msg {
                        fx::rea_eq::Param::FreqLowShelf(val) => {
                            Ok(vec![ChannelStripMsg::LowFreq(val)])
                        }
                        fx::rea_eq::Param::GainLowShelf(val) => {
                            Ok(vec![ChannelStripMsg::LowGain(val)])
                        }
                        fx::rea_eq::Param::BWLowShelf(val) => Ok(vec![ChannelStripMsg::LowQ(val)]),
                        fx::rea_eq::Param::FreqBand2(val) => Ok(vec![ChannelStripMsg::LmFreq(val)]),
                        fx::rea_eq::Param::GainBand2(val) => Ok(vec![ChannelStripMsg::LmGain(val)]),
                        fx::rea_eq::Param::BWBand2(val) => Ok(vec![ChannelStripMsg::LmQ(val)]),
                        fx::rea_eq::Param::FreqBand3(val) => Ok(vec![ChannelStripMsg::HmFreq(val)]),
                        fx::rea_eq::Param::GainBand3(val) => Ok(vec![ChannelStripMsg::HmGain(val)]),
                        fx::rea_eq::Param::BWBand3(val) => Ok(vec![ChannelStripMsg::HmQ(val)]),
                        fx::rea_eq::Param::FreqHighShelf4(val) => {
                            Ok(vec![ChannelStripMsg::HighFreq(val)])
                        }
                        fx::rea_eq::Param::GainHighShelf4(val) => {
                            Ok(vec![ChannelStripMsg::HighGain(val)])
                        }
                        fx::rea_eq::Param::BWHighShelf4(val) => {
                            Ok(vec![ChannelStripMsg::HighQ(val)])
                        }
                        _ => Ok(vec![]),
                    }
                } else {
                    Ok(vec![])
                }
            }
            track::DataMsg::FXEnabled(msg) => {
                if let Some(fx_name) = self.plugin_names_by_index.get(msg.fx_index as usize) {
                    if fx_name == rea_eq::name() && self.track_guid == msg.track_guid {
                        if msg.enabled {
                            Ok(vec![ChannelStripMsg::EnableEq(EqType::Digital)])
                        } else {
                            Ok(vec![ChannelStripMsg::DisableEq])
                        }
                    } else {
                        Ok(vec![])
                    }
                } else {
                    Ok(vec![])
                }
            }
            _ => Ok(vec![]),
        }
    }

    fn get_first_fx_index(&self, needle: fx::FX) -> Option<i32> {
        self.plugins_by_index
            .iter()
            .position(|haystack| *haystack == needle)
            .map(|index| index as i32)
    }

    fn get_first_fx_index_string(&self, needle: &str) -> Option<i32> {
        self.plugin_names_by_index
            .iter()
            .position(|haystack| haystack == needle)
            .map(|index| index as i32)
    }

    pub fn translate_message_from_downstream(
        &self,
        msg: ChannelStripMsg,
    ) -> Result<Vec<track::TrackMsg>, TranslationErr> {
        match msg {
            ChannelStripMsg::EnableEq(eq_type) => match eq_type {
                EqType::Digital => {
                    Ok(vec![
                        track::InstantiateFX {
                            track_guid: self.track_guid,
                            fx_index: 0, // TODO:
                            fx_name: rea_eq::name().to_string(),
                        }
                        .into(),
                    ])
                }
                _ => Ok(vec![]),
            },
            ChannelStripMsg::DisableEq => match self.get_first_fx_index_string(rea_eq::name()) {
                Some(fx_index) => Ok(vec![
                    track::FXEnabled {
                        track_guid: self.track_guid,
                        fx_index,
                        enabled: false,
                    }
                    .into(),
                ]),
                None => Ok(vec![]),
            },
            ChannelStripMsg::LowFreq(val) => match self.get_first_fx_index_string(rea_eq::name()) {
                Some(fx_index) => Ok(vec![fx::rea_eq::encode_trackmsg(
                    self.track_guid,
                    fx_index,
                    fx::rea_eq::Param::FreqLowShelf(val),
                )]),
                None => Ok(vec![]),
            },
            ChannelStripMsg::LowGain(val) => match self.get_first_fx_index_string(rea_eq::name()) {
                Some(fx_index) => Ok(vec![fx::rea_eq::encode_trackmsg(
                    self.track_guid,
                    fx_index,
                    fx::rea_eq::Param::GainLowShelf(val),
                )]),
                None => Ok(vec![]),
            },
            ChannelStripMsg::LowQ(val) => match self.get_first_fx_index_string(rea_eq::name()) {
                Some(fx_index) => Ok(vec![fx::rea_eq::encode_trackmsg(
                    self.track_guid,
                    fx_index,
                    fx::rea_eq::Param::BWLowShelf(val),
                )]),
                None => Ok(vec![]),
            },
            ChannelStripMsg::LmFreq(val) => match self.get_first_fx_index_string(rea_eq::name()) {
                Some(fx_index) => Ok(vec![fx::rea_eq::encode_trackmsg(
                    self.track_guid,
                    fx_index,
                    fx::rea_eq::Param::FreqBand2(val),
                )]),
                None => Ok(vec![]),
            },
            ChannelStripMsg::LmGain(val) => match self.get_first_fx_index_string(rea_eq::name()) {
                Some(fx_index) => Ok(vec![fx::rea_eq::encode_trackmsg(
                    self.track_guid,
                    fx_index,
                    fx::rea_eq::Param::GainBand2(val),
                )]),
                None => Ok(vec![]),
            },
            ChannelStripMsg::HmFreq(val) => match self.get_first_fx_index_string(rea_eq::name()) {
                Some(fx_index) => Ok(vec![fx::rea_eq::encode_trackmsg(
                    self.track_guid,
                    fx_index,
                    fx::rea_eq::Param::FreqBand3(val),
                )]),
                None => Ok(vec![]),
            },
            ChannelStripMsg::HmGain(val) => match self.get_first_fx_index_string(rea_eq::name()) {
                Some(fx_index) => Ok(vec![fx::rea_eq::encode_trackmsg(
                    self.track_guid,
                    fx_index,
                    fx::rea_eq::Param::GainBand3(val),
                )]),
                None => Ok(vec![]),
            },
            ChannelStripMsg::HmQ(val) => match self.get_first_fx_index_string(rea_eq::name()) {
                Some(fx_index) => Ok(vec![fx::rea_eq::encode_trackmsg(
                    self.track_guid,
                    fx_index,
                    fx::rea_eq::Param::BWBand3(val),
                )]),
                None => Ok(vec![]),
            },
            ChannelStripMsg::HighFreq(val) => {
                match self.get_first_fx_index_string(rea_eq::name()) {
                    Some(fx_index) => Ok(vec![fx::rea_eq::encode_trackmsg(
                        self.track_guid,
                        fx_index,
                        fx::rea_eq::Param::FreqHighShelf4(val),
                    )]),
                    None => Ok(vec![]),
                }
            }
            ChannelStripMsg::HighGain(val) => {
                match self.get_first_fx_index_string(rea_eq::name()) {
                    Some(fx_index) => Ok(vec![fx::rea_eq::encode_trackmsg(
                        self.track_guid,
                        fx_index,
                        fx::rea_eq::Param::GainHighShelf4(val),
                    )]),
                    None => Ok(vec![]),
                }
            }
            ChannelStripMsg::HighQ(val) => match self.get_first_fx_index_string(rea_eq::name()) {
                Some(fx_index) => Ok(vec![fx::rea_eq::encode_trackmsg(
                    self.track_guid,
                    fx_index,
                    fx::rea_eq::Param::BWHighShelf4(val),
                )]),
                None => Ok(vec![]),
            },
            _ => todo!(),
        }
    }
}
