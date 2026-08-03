use std::sync::Mutex;

use uuid::Uuid;

use crate::modes::generated_fx_param as fx;
use crate::track::track;

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

#[derive(Debug, Clone, Copy)]
pub enum BandMode {
    Bell,
    Shelf,
}

#[derive(Debug, Clone, Copy)]
pub enum EqType {
    Digital,
}

#[derive(Debug, Clone, Copy)]
pub enum EqPosition {
    First,
    Middle,
    Last,
}

#[derive(Debug, Clone, Copy)]
pub enum CompOrder {
    FtoS,
    StoF,
}

#[derive(Debug, Clone, Copy)]
pub enum CompType {
    Digital,
}

#[derive(Debug, Clone, Copy)]
pub enum BypassMode {
    Engaged,
    Bypassed,
}

#[derive(Debug, Clone, Copy)]
pub enum SaturationType {
    Console,
}

#[derive(Debug, Clone, Copy)]
pub enum ChannelStripMsg {
    EnableEq,
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
    EqBypass(BypassMode),
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
    CompBypass(BypassMode),
    Comp2Bypass(BypassMode),
    Saturation(f32),
    SaturationType(SaturationType),
    SaturationBypass(BypassMode),
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
    hp_filter: Option<FXParamIdent>,
    hp_slope: Option<FXParamIdent>,
    low_freq: Option<FXParamIdent>,
    low_q: Option<FXParamIdent>,
    low_slope: Option<FXParamIdent>,
    /// Chooses between bell and shelf for low band
    low_bell_shelf: Option<FXParamIdent>,
    low_gain: Option<FXParamIdent>,
    lm_freq: Option<FXParamIdent>,
    lm_q: Option<FXParamIdent>,
    lm_gain: Option<FXParamIdent>,
    hm_freq: Option<FXParamIdent>,
    hm_q: Option<FXParamIdent>,
    hm_gain: Option<FXParamIdent>,
    high_freq: Option<FXParamIdent>,
    high_q: Option<FXParamIdent>,
    high_slope: Option<FXParamIdent>,
    /// Chooses between bell and shelf for high band
    high_bell_shelf: Option<FXParamIdent>,
    high_gain: Option<FXParamIdent>,
    /// Gain for the "sides" channel in a mid-side EQ (if applicable)
    high_sides_gain: Option<FXParamIdent>,
    /// Toggles between various EQ plugins
    eq_type: Option<FXParamIdent>,
    eq_bypass: Option<FXParamIdent>,
    /// EQ before or after comprssion
    // eq_position: Option<FXParamIdent>,
    comp1_thresh: Option<FXParamIdent>,
    comp1_sc_filter: Option<FXParamIdent>,
    comp1_ratio: Option<FXParamIdent>,
    comp1_attack: Option<FXParamIdent>,
    comp1_release: Option<FXParamIdent>,
    comp1_makeup: Option<FXParamIdent>,
    /// Toggles between various compressor plugins
    comp1_type: Option<FXParamIdent>,
    comp1_bypass: Option<FXParamIdent>,
    comp2_thresh: Option<FXParamIdent>,
    comp2_sc_filter: Option<FXParamIdent>,
    comp2_ratio: Option<FXParamIdent>,
    comp2_attack: Option<FXParamIdent>,
    comp2_release: Option<FXParamIdent>,
    comp2_makeup: Option<FXParamIdent>,
    /// Toggles between various compressor plugins
    comp2_type: Option<FXParamIdent>,
    comp2_bypass: Option<FXParamIdent>,
    /// Comp 1 -> Comp 2 or Comp 2 -> Comp 1
    // comp_position: Option<FXParamIdent>,
    saturation: Option<FXParamIdent>,
    saturation_bypass: Option<FXParamIdent>,
    /// Toggles between various saturation plugins
    saturation_type: Option<FXParamIdent>,
    gain: Option<FXParamIdent>,
    /// Toggles between various gain plugins (e.g. preamp models)
    gain_type: Option<FXParamIdent>,
    /// Only active if the track is armed
    interface_gain: Option<FXParamIdent>,
}

impl ChannelStripRouter {
    pub fn new(track_guid: Uuid) -> Self {
        ChannelStripRouter {
            mux: Mutex::new(()),
            track_guid,
            plugins_by_index: Vec::new(),
            plugin_names_by_index: Vec::new(),
            hp_filter: None,
            hp_slope: None,
            low_freq: None,
            low_q: None,
            low_slope: None,
            low_bell_shelf: None,
            low_gain: None,
            lm_freq: None,
            lm_q: None,
            lm_gain: None,
            hm_freq: None,
            hm_q: None,
            hm_gain: None,
            high_freq: None,
            high_q: None,
            high_slope: None,
            high_bell_shelf: None,
            high_gain: None,
            high_sides_gain: None,
            eq_type: None,
            eq_bypass: None,
            comp1_thresh: None,
            comp1_sc_filter: None,
            comp1_ratio: None,
            comp1_attack: None,
            comp1_release: None,
            comp1_makeup: None,
            comp1_type: None,
            comp1_bypass: None,
            comp2_thresh: None,
            comp2_sc_filter: None,
            comp2_ratio: None,
            comp2_attack: None,
            comp2_release: None,
            comp2_makeup: None,
            comp2_type: None,
            comp2_bypass: None,
            saturation: None,
            saturation_bypass: None,
            saturation_type: None,
            gain: None,
            gain_type: None,
            interface_gain: None,
        }
    }

    pub fn update_plugin_state(&mut self, plugin_index: i32, plugin_name: &str) {
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
            _ => Ok(vec![]),
        }
    }

    fn get_first_fx_index(&self, needle: fx::FX) -> Option<i32> {
        self.plugins_by_index
            .iter()
            .position(|haystack| *haystack == needle)
            .map(|index| index as i32)
    }

    pub fn translate_message_from_downstream(
        &self,
        msg: ChannelStripMsg,
    ) -> Result<Vec<track::TrackMsg>, TranslationErr> {
        match msg {
            ChannelStripMsg::LowFreq(val) => match self.get_first_fx_index(fx::FX::ReaEQ) {
                Some(fx_index) => Ok(vec![fx::rea_eq::encode_trackmsg(
                    self.track_guid,
                    fx_index,
                    fx::rea_eq::Param::FreqLowShelf(val),
                )]),
                None => Ok(vec![]),
            },
            ChannelStripMsg::LowGain(val) => match self.get_first_fx_index(fx::FX::ReaEQ) {
                Some(fx_index) => Ok(vec![fx::rea_eq::encode_trackmsg(
                    self.track_guid,
                    fx_index,
                    fx::rea_eq::Param::GainLowShelf(val),
                )]),
                None => Ok(vec![]),
            },
            ChannelStripMsg::LowQ(val) => match self.get_first_fx_index(fx::FX::ReaEQ) {
                Some(fx_index) => Ok(vec![fx::rea_eq::encode_trackmsg(
                    self.track_guid,
                    fx_index,
                    fx::rea_eq::Param::BWLowShelf(val),
                )]),
                None => Ok(vec![]),
            },
            ChannelStripMsg::LmFreq(val) => match self.get_first_fx_index(fx::FX::ReaEQ) {
                Some(fx_index) => Ok(vec![fx::rea_eq::encode_trackmsg(
                    self.track_guid,
                    fx_index,
                    fx::rea_eq::Param::FreqBand2(val),
                )]),
                None => Ok(vec![]),
            },
            ChannelStripMsg::LmGain(val) => match self.get_first_fx_index(fx::FX::ReaEQ) {
                Some(fx_index) => Ok(vec![fx::rea_eq::encode_trackmsg(
                    self.track_guid,
                    fx_index,
                    fx::rea_eq::Param::GainBand2(val),
                )]),
                None => Ok(vec![]),
            },
            _ => Ok(vec![]),
        }
    }
}
