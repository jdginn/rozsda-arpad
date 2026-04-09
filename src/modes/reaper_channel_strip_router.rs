use std::sync::Mutex;

use crate::track::track;
use crate::track::track::TrackMsg;

use uuid::Uuid;

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
/// | 16 | Gain        | Interface gain (only if armed)   | Trim             |                |                |                 |

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
struct ChannelStripMap {
    mux: Mutex<()>,
    plugin_names_by_index: Vec<String>,
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

impl ChannelStripMap {
    fn new() -> Self {
        ChannelStripMap {
            mux: Mutex::new(()),
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

    fn update_plugin_state(&mut self, plugin_index: i32, plugin_name: &str) {
        let _lock = self.mux.lock().unwrap();
        if (plugin_index as usize) >= self.plugin_names_by_index.len() {
            self.plugin_names_by_index
                .resize((plugin_index + 1) as usize, String::new());
        }
        self.plugin_names_by_index[plugin_index as usize] = plugin_name.to_string();
    }

    fn update_mapping_locked(&mut self) {}

    fn translate_downstream_msg(&self, msg: TrackMsg) -> Result<ChannelStripMsg, String> {
        // FIXME: implement
        Ok(ChannelStripMsg::HpfFreq(0.0))
    }

    fn translate_upstream_msg(&self, msg: ChannelStripMsg) -> Result<TrackMsg, String> {
        // FIXME: implement
        Ok(track::Muted {
            track_guid: Uuid::new_v4(),
            muted: true,
        }
        .into())
    }
}
