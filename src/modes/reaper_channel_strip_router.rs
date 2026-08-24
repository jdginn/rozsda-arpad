use std::collections::HashMap;
use uuid::Uuid;

use crate::modes::reaper_channel_strip_widgets::EncoderTurn;
use crate::modes::reaper_fx::{FxId, rea_eq};
use crate::modes::reaper_fx_adapters::{FxAdapter, rea_eq::ReaEqAdapter};
use crate::track::track;

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
// | 11 | ComGp thresh | Comp SC filter                   | Comp2  thresh    | Comp2 SC filt  |                |                 |
// | 12 | Comp ratio  | Comp attack                      | Comp2  ratio     | Comp2 attack   |                |                 |
// | 13 | Comp makeup | Comp release                     | Comp2  makeup    | Comp2 release  |                |                 |
// | 14 | Comp type   |                                  | Comp2  type      |                | bypass Comp    | bypass Comp2    |
// | 15 | Saturation  |                                  | Saturation type  |                | bypass Sat     |                 |
// | 16 | Gain        | Interface gain (only if armed)   | Trim             |                |                |                 |

pub enum FxKind {
    Eq,
    Comp,
    Saturation,
    Gain,
    Trim,
    InterfaceGain,
}

#[derive(Debug, Clone, Copy)]
pub enum FxUi {
    Eq(EqType),
    Comp(CompType),
    Saturation(SaturationType),
    Gain,
    Trim,
    InterfaceGain,
}

rotary_enum! {
    /// UI representation of the channel strip concepts, which are mapped to actual FX parameters by the router.
    /// By design, this does not need to have a 1:1 mapping to the internal logic.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum BandMode {
        Bell => "bell",
        Shelf => "shelf",
    }
}

rotary_enum! {
    /// UI representation of the channel strip concepts, which are mapped to actual FX parameters by the router.
    /// By design, this does not need to have a 1:1 mapping to the internal logic.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum EqType {
        Digital => "Digital",
        SSL => "SSL",
        Neve => "Neve",
    }
}

rotary_enum! {
    /// UI representation of the channel strip concepts, which are mapped to actual FX parameters by the router.
    /// By design, this does not need to have a 1:1 mapping to the internal logic.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum EqPosition {
        First => "E>C>C>S",
        Middle => "C>C>E>S",
        Last => "C>C>S>E",
    }
}

rotary_enum! {
    /// UI representation of the channel strip concepts, which are mapped to actual FX parameters by the router.
    /// By design, this does not need to have a 1:1 mapping to the internal logic.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum CompOrder {
        C1toC2 => "Cmp1->2",
        C2toC1 => "Cmp2->1",
    }
}

rotary_enum! {
    /// UI representation of the channel strip concepts, which are mapped to actual FX parameters by the router.
    /// By design, this does not need to have a 1:1 mapping to the internal logic.
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
    /// UI representation of the channel strip concepts, which are mapped to actual FX parameters by the router.
    /// By design, this does not need to have a 1:1 mapping to the internal logic.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum EqBypass {
        IN => "EqIN",
        OUT => "EqOUT",
    }
}

rotary_enum! {
    /// UI representation of the channel strip concepts, which are mapped to actual FX parameters by the router.
    /// By design, this does not need to have a 1:1 mapping to the internal logic.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum CompBypass {
        IN => "CompIN",
        OUT => "CompOUT",
    }
}

rotary_enum! {
    /// UI representation of the channel strip concepts, which are mapped to actual FX parameters by the router.
    /// By design, this does not need to have a 1:1 mapping to the internal logic.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Cmp2Bypass {
        Engaged => "Cmp2IN",
        Bypassed => "Cmp2OUT",
    }
}

rotary_enum! {
    /// UI representation of the channel strip concepts, which are mapped to actual FX parameters by the router.
    /// By design, this does not need to have a 1:1 mapping to the internal logic.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum SaturationBypass {
        IN => "SatIN",
        OUT => "SatOUT",
    }
}

rotary_enum! {
    /// UI representation of the channel strip concepts, which are mapped to actual FX parameters by the router.
    /// By design, this does not need to have a 1:1 mapping to the internal logic.
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
    EnableComp1(CompType),
    DisableComp1,
    EnableComp2(CompType),
    DisableComp2,
    EnableSaturation(SaturationType),
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

impl ChannelStripMsg {
    fn kind(&self) -> Option<FxKind> {
        use ChannelStripMsg::*;
        Some(match self {
            LowFreq(_) | LowGain(_) | HighFreq(_) | EqType(_) | EqBypass(_) => FxKind::Eq,
            CompThresh(_) | CompRatio(_) | CompType(_) | CompBypass(_) => FxKind::Comp,
            Saturation(_) | SaturationType(_) | SaturationBypass(_) => FxKind::Saturation,
            Gain(_) => FxKind::Gain,
            Trim(_) => FxKind::Trim,
            InterfaceGain(_) => FxKind::InterfaceGain,
            _ => return None, // non-routing/system msgs
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub enum TranslationErr {
    Dummy,
}

// ----
// These structs define actual routing logic
// ----

struct FXParamIdent {
    fx_index: i32,
    param_index: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FxCategory {
    Eq,
    Comp,
    Saturation,
    Gain,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SlotRole {
    Eq,
    Comp1,
    Comp2,
    Saturation,
    Gain,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SlotSpec {
    role: SlotRole,
    accepts: &'static [FxCategory],
}

#[derive(Clone, Copy)]
struct FxSpec {
    id: FxId,
    ui_type: FxUi,
    categories: &'static [FxCategory],
    adapter: &'static dyn FxAdapter,
}

pub struct RoutingRegistry {
    pub slots: &'static [SlotSpec],
    pub fx_specs: &'static [FxSpec],

    pub by_fx_id: HashMap<FxId, &'static FxSpec>,
    pub fx_to_slots: HashMap<FxId, Vec<SlotRole>>, // derived
    pub name_to_fx_id: HashMap<String, FxId>,      // derived
}

impl RoutingRegistry {
    pub fn new() -> Self {
        let slots: &'static [SlotSpec] = &[
            SlotSpec {
                role: SlotRole::Eq,
                accepts: &[FxCategory::Eq],
            },
            SlotSpec {
                role: SlotRole::Comp1,
                accepts: &[FxCategory::Comp],
            },
            SlotSpec {
                role: SlotRole::Comp2,
                accepts: &[FxCategory::Comp],
            },
            SlotSpec {
                role: SlotRole::Saturation,
                accepts: &[FxCategory::Saturation],
            },
            SlotSpec {
                role: SlotRole::Gain,
                accepts: &[FxCategory::Gain],
            },
        ];

        let fx_specs: &'static [FxSpec] = &[FxSpec {
            id: FxId::ReaEQ,
            ui_type: FxUi::Eq(EqType::Digital),
            categories: &[FxCategory::Eq],
            adapter: &ReaEqAdapter {},
        }];

        let by_fx_id = fx_specs
            .iter()
            .map(|spec| (spec.id, spec))
            .collect::<HashMap<_, _>>();

        let mut fx_to_slots = HashMap::new();
        for spec in fx_specs.iter() {
            for slot in slots.iter() {
                if spec.categories.iter().any(|cat| slot.accepts.contains(cat)) {
                    fx_to_slots
                        .entry(spec.id)
                        .or_insert_with(Vec::new)
                        .push(slot.role);
                }
            }
        }

        let mut name_to_fx_id = HashMap::new();
        for spec in fx_specs.iter() {
            for name in spec.adapter.fx_names() {
                name_to_fx_id.insert(name.to_string(), spec.id);
            }
        }

        RoutingRegistry {
            slots,
            fx_specs,
            by_fx_id,
            fx_to_slots,
            name_to_fx_id,
        }
    }

    fn get_spec_by_name(&self, name: &str) -> Option<&FxSpec> {
        self.name_to_fx_id
            .get(name)
            .and_then(|fx_id| self.by_fx_id.get(fx_id).copied())
    }
}

// TODO: might need to keep one cache by guid and another by position to resolve in the case we
// haven't gotten the GUID yet - only do this if we realize it's actually necessary for robustness
//
// NOTE: as of 8/22, we no longer believe this is necessary but we could be wrong

#[derive(Clone)]
struct FxInstance {
    spec: FxSpec,
    guid: Uuid,
}

struct SlotBinding {
    role: SlotRole,
    active_instance: Option<FxInstance>,
    owned_instances: Vec<FxInstance>,
}

impl Default for SlotBinding {
    fn default() -> Self {
        SlotBinding {
            role: SlotRole::Eq,
            active_instance: None,
            owned_instances: Vec::new(),
        }
    }
}

struct Slots {
    eq: SlotBinding,
    comp1: SlotBinding,
    comp2: SlotBinding,
    saturation: SlotBinding,
    gain: SlotBinding,
}

impl Slots {
    fn new() -> Self {
        Slots {
            eq: SlotBinding {
                role: SlotRole::Eq,
                ..Default::default()
            },
            comp1: SlotBinding {
                role: SlotRole::Comp1,
                ..Default::default()
            },
            comp2: SlotBinding {
                role: SlotRole::Comp2,
                ..Default::default()
            },
            saturation: SlotBinding {
                role: SlotRole::Saturation,
                ..Default::default()
            },
            gain: SlotBinding {
                role: SlotRole::Gain,
                ..Default::default()
            },
        }
    }

    fn get(&self, role: SlotRole) -> &SlotBinding {
        match role {
            SlotRole::Eq => &self.eq,
            SlotRole::Comp1 => &self.comp1,
            SlotRole::Comp2 => &self.comp2,
            SlotRole::Saturation => &self.saturation,
            SlotRole::Gain => &self.gain,
        }
    }

    fn get_mut(&mut self, role: SlotRole) -> &mut SlotBinding {
        match role {
            SlotRole::Eq => &mut self.eq,
            SlotRole::Comp1 => &mut self.comp1,
            SlotRole::Comp2 => &mut self.comp2,
            SlotRole::Saturation => &mut self.saturation,
            SlotRole::Gain => &mut self.gain,
        }
    }

    fn iter(&self) -> impl Iterator<Item = &SlotBinding> {
        [
            &self.eq,
            &self.comp1,
            &self.comp2,
            &self.saturation,
            &self.gain,
        ]
        .into_iter()
    }

    fn iter_mut(&mut self) -> impl Iterator<Item = &mut SlotBinding> {
        [
            &mut self.eq,
            &mut self.comp1,
            &mut self.comp2,
            &mut self.saturation,
            &mut self.gain,
        ]
        .into_iter()
    }
}

struct PluginInfo {
    name: String,
    fx_idx: usize,
    enabled: bool,
}

/// Maps named, high-level channel strip concepts to their respective parameters
///
/// NOTE: we have one of these *PER TRACK*
///
/// TODO: the hard part will be getting this to update dynamically based on the actual FX chain on the track
pub struct ChannelStripRouter {
    track_guid: Uuid,

    routing_registry: RoutingRegistry,

    slots: Slots,
    plugins: HashMap<Uuid, PluginInfo>, // Maps Fx GUID to PluginInfo
}

impl ChannelStripRouter {
    pub fn new(track_guid: Uuid) -> Self {
        ChannelStripRouter {
            track_guid,
            routing_registry: RoutingRegistry::new(),
            slots: Slots::new(),
            plugins: HashMap::new(),
        }
    }

    pub fn set_plugin_enable(&mut self, plugin_guid: Uuid, enabled: bool) {
        if let Some(plugin_info) = self.plugins.get_mut(&plugin_guid) {
            plugin_info.enabled = enabled;
        }
    }

    fn get_fx_guid_from_index(&self, fx_index: i32) -> Option<Uuid> {
        self.plugins
            .iter()
            .find(|(_, info)| info.fx_idx == fx_index as usize)
            .map(|(guid, _)| *guid)
    }

    pub fn update_plugin_state(&mut self, plugin_guid: Uuid, plugin_name: &str, fx_index: i32) {
        self.plugins
            .entry(plugin_guid)
            .and_modify(|info| {
                info.name = plugin_name.to_string();
                info.fx_idx = fx_index as usize;
            })
            .or_insert(PluginInfo {
                name: plugin_name.to_string(),
                fx_idx: fx_index as usize,
                enabled: true,
            });
    }

    pub fn translate_message_from_upstream(
        &mut self,
        msg: track::DataMsg,
    ) -> Result<Vec<ChannelStripMsg>, TranslationErr> {
        match msg {
            track::DataMsg::FXName(msg) => {
                self.update_plugin_state(msg.fx_guid, &msg.name, msg.fx_index);

                if let Some(fx_id) = self.routing_registry.name_to_fx_id.get(&msg.name) {
                    if let Some(applicable_roles) = self.routing_registry.fx_to_slots.get(fx_id) {
                        for role in applicable_roles {
                            let instance = FxInstance {
                                spec: **self.routing_registry.by_fx_id.get(fx_id).unwrap(),
                                guid: msg.fx_guid,
                            };

                            let slot = self.slots.get_mut(*role);
                            // TODO: this just blindly assumes that the next valid FX we get is the
                            // one we activate. There is also no policy for purging any old owned FX.
                            // These assumptions are probably faulty and need to be revisited.
                            slot.owned_instances.push(instance.clone());
                            slot.active_instance = Some(instance);
                            match role {
                                SlotRole::Eq => {
                                    return Ok(vec![ChannelStripMsg::EnableEq(EqType::Digital)]);
                                }
                                SlotRole::Comp1 => {
                                    return Ok(vec![ChannelStripMsg::EnableComp1(
                                        CompType::Digital,
                                    )]);
                                }
                                SlotRole::Comp2 => {
                                    return Ok(vec![ChannelStripMsg::EnableComp2(
                                        CompType::Digital,
                                    )]);
                                }
                                SlotRole::Saturation => {
                                    return Ok(vec![ChannelStripMsg::EnableSaturation(
                                        SaturationType::Console,
                                    )]);
                                }
                                SlotRole::Gain => return Ok(vec![ChannelStripMsg::EnableGain]),
                            }
                        }
                    }
                }
                Ok(vec![])
            }
            track::DataMsg::FXParamValue(msg) => {
                if let Some(fx_guid) = self.get_fx_guid_from_index(msg.fx_index) {
                    for slot in self.slots.iter() {
                        if let Some(active_instance) = &slot.active_instance {
                            if active_instance.guid == fx_guid {
                                if let Some(msg) = active_instance.spec.adapter.from_track(msg) {
                                    return Ok(vec![msg]);
                                }
                            }
                        }
                    }
                }
                Ok(vec![])
            }
            track::DataMsg::FXEnabled(msg) => {
                if msg.enabled {
                    for slot in self.slots.iter_mut() {
                        for instance in &slot.owned_instances {
                            if instance.guid == msg.fx_guid {
                                slot.active_instance = Some(instance.clone());
                            }
                            match instance.spec.ui_type {
                                FxUi::Eq(eq_type) => {
                                    return Ok(vec![ChannelStripMsg::EnableEq(eq_type)]);
                                }
                                FxUi::Comp(comp_type) => {
                                    if slot.role == SlotRole::Comp1 {
                                        return Ok(vec![ChannelStripMsg::EnableComp1(comp_type)]);
                                    } else if slot.role == SlotRole::Comp2 {
                                        return Ok(vec![ChannelStripMsg::EnableComp2(comp_type)]);
                                    }
                                }
                                FxUi::Saturation(sat_type) => {
                                    return Ok(vec![ChannelStripMsg::EnableSaturation(sat_type)]);
                                }
                                FxUi::Gain => {
                                    return Ok(vec![ChannelStripMsg::EnableGain]);
                                }
                                _ => {}
                            }
                        }
                    }
                    Ok(vec![])
                } else {
                    for slot in self.slots.iter_mut() {
                        if let Some(active_instance) = slot.active_instance.clone() {
                            if active_instance.guid == msg.fx_guid {
                                slot.active_instance = None;
                                match slot.role {
                                    SlotRole::Eq => return Ok(vec![ChannelStripMsg::DisableEq]),
                                    SlotRole::Comp1 => {
                                        return Ok(vec![ChannelStripMsg::DisableComp1]);
                                    }
                                    SlotRole::Comp2 => {
                                        return Ok(vec![ChannelStripMsg::DisableComp2]);
                                    }
                                    SlotRole::Saturation => {
                                        return Ok(vec![ChannelStripMsg::DisableSaturation]);
                                    }
                                    SlotRole::Gain => {
                                        return Ok(vec![ChannelStripMsg::DisableGain]);
                                    }
                                }
                            }
                        }
                    }
                    Ok(vec![])
                }
            }
            _ => Ok(vec![]),
        }
    }

    pub fn translate_message_from_downstream(
        &self,
        msg: ChannelStripMsg,
    ) -> Result<Vec<track::TrackMsg>, TranslationErr> {
        match msg.kind() {
            Some(FxKind::Eq) => {
                if let Some(active_instance) = &self.slots.eq.active_instance {
                    if let Some(track_msg) =
                        active_instance
                            .spec
                            .adapter
                            .to_track(self.track_guid, 0, msg)
                    {
                        return Ok(vec![track_msg]);
                    }
                }
                Ok(vec![])
            }
            _ => Ok(vec![]), // TODO: implement other kinds
        }
    }
}
