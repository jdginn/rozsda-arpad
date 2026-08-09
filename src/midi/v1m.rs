use std::sync::{Arc, Mutex};
use std::thread;

use crossbeam_channel::{Receiver, Sender};
use helgoboss_midi::Channel;
use midir::{MidiInputPort, MidiOutputConnection};

use coalescible_derive::{Coalescible, CoalescibleEnum};
use derive_enum_from::EnumFrom;

use crate::midi::base::{
    ChannelPressure, ChannelPressureBuilder, ControlChange, ControlChangeBuilder, NoteOff,
    NoteOffBuilder, NoteOn, NoteOnBuilder, PitchBend, PitchBendBuilder,
};
use crate::midi::text::compact_to_7_bytes;
use crate::midi::{MidiDevice, MidiError};
use crate::modes::mode_manager::Barrier;
use crate::traits::{Bind, Set};

pub const FADER_0DB: f32 = 0.72; // Placeholder value for 0dB on fader scale

#[derive(Clone, Copy, Debug, PartialEq, Eq, std::hash::Hash)]
pub enum StereoChannel {
    Left,
    Right,
}

impl std::fmt::Display for StereoChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StereoChannel::Left => write!(f, "Left"),
            StereoChannel::Right => write!(f, "Right"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, std::hash::Hash)]
pub enum Slot {
    DAW1,
    DAW2,
    DAW3,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, std::hash::Hash)]
pub enum DawId {
    Bitwig,
    Cubase,
    ProTools,
    Logic,
    Live,
    Reaper,
    Reason,
    StudioOne,
    FLStudio,
    Cakewalk,
    DigitalPerformer,
    Samplitude,
    Harrison,
    Nuendo,
    Audition,
    Tracktion,
    Ability,
    Luna,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, std::hash::Hash)]
pub enum TouchScreenLayer {
    Blue,
    Green,
    Yellow,
    User1,
    User2,
}

impl std::fmt::Display for TouchScreenLayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TouchScreenLayer::Blue => write!(f, "Blue"),
            TouchScreenLayer::Green => write!(f, "Green"),
            TouchScreenLayer::Yellow => write!(f, "Yellow"),
            TouchScreenLayer::User1 => write!(f, "User1"),
            TouchScreenLayer::User2 => write!(f, "User2"),
        }
    }
}

#[derive(Clone, Debug, Copy, Coalescible)]
pub struct ChannelFaderMsg {
    pub idx: i32,
    #[data]
    pub value: f64, // Probably too much precision?
}

#[derive(Clone, Debug, Copy, Coalescible)]
pub struct MasterFaderMsg {
    #[data]
    pub value: f64, // Probably too much precision?
}

#[derive(Clone, Copy, Debug)]
pub struct EncoderTurnCW {
    pub idx: i32,
    pub accel: u8,
}

#[derive(Clone, Copy, Debug)]
pub struct EncoderTurnCCW {
    pub idx: i32,
    pub accel: u8,
}

#[derive(Clone, Copy, Debug)]
pub struct EncoderPressMsg {
    pub idx: i32,
}

#[derive(Clone, Copy, Debug)]
pub struct EncoderReleaseMsg {
    pub idx: i32,
}

#[derive(Clone, Copy, Debug, Coalescible)]
pub struct EncoderRingMsg {
    pub idx: i32,
    #[data]
    pub mode: EncoderRingMode,
    #[data]
    pub val: u8,
}

// Maps a float to an encoder u8
//
// u8 encodes setps from to one of 12 values representing positions along the encoder
pub fn map_to_encoder_ring(x: f32) -> u8 {
    let clamped = x.clamp(-1.0, 1.0) as f64;
    ((clamped + 1.0) * 0.5 * 0xb as f64).round() as u8
}

/// Val between -1.0 and 1.0
impl EncoderRingMsg {
    pub fn new(idx: i32, mode: EncoderRingMode, val: f32) -> Self {
        Self {
            idx,
            mode,
            val: map_to_encoder_ring(val),
        }
    }
}

#[derive(Clone, Copy, Debug, EnumFrom)]
pub enum EncoderRingMode {
    Point,
    FromCenter,
    FromLeft,
    Width,
}

pub const ENCODER_CENTER_MODE_CENTER: u8 = 0x06;

impl std::fmt::Display for EncoderRingMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EncoderRingMode::Point => write!(f, "Point"),
            EncoderRingMode::FromCenter => write!(f, "FromCenter"),
            EncoderRingMode::FromLeft => write!(f, "FromLeft"),
            EncoderRingMode::Width => write!(f, "Width"),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct EncoderRingLEDBlankMsg {
    pub idx: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, CoalescibleEnum)]
pub enum LEDState {
    Off,
    On,
    Flash,
}

impl From<bool> for LEDState {
    fn from(value: bool) -> Self {
        match value {
            false => LEDState::Off,
            true => LEDState::On,
        }
    }
}

#[derive(Clone, Debug, Copy)]
pub struct MutePress {
    pub idx: i32,
}

#[derive(Clone, Debug, Copy)]
pub struct MuteRelease {
    pub idx: i32,
}

#[derive(Clone, Debug, Copy, Coalescible)]
pub struct MuteLEDMsg {
    pub idx: i32,
    #[data]
    pub state: LEDState,
}

#[derive(Clone, Debug, Copy)]
pub struct SoloPress {
    pub idx: i32,
}

#[derive(Clone, Debug, Copy)]
pub struct SoloRelease {
    pub idx: i32,
}

#[derive(Clone, Debug, Copy, Coalescible)]
pub struct SoloLEDMsg {
    pub idx: i32,
    #[data]
    pub state: LEDState,
}

#[derive(Clone, Debug, Copy)]
pub struct ArmPress {
    pub idx: i32,
}

#[derive(Clone, Debug, Copy)]
pub struct ArmRelease {
    pub idx: i32,
}

#[derive(Clone, Debug, Copy, Coalescible)]
pub struct ArmLEDMsg {
    pub idx: i32,
    #[data]
    pub state: LEDState,
}

#[derive(Clone, Debug, Copy)]
pub struct SelectPress {
    pub idx: i32,
}

#[derive(Clone, Debug, Copy)]
pub struct SelectRelease {
    pub idx: i32,
}

#[derive(Clone, Debug, Copy, Coalescible)]
pub struct SelectLEDMsg {
    pub idx: i32,
    #[data]
    pub state: LEDState,
}

#[derive(Clone, Debug, Copy, Coalescible)]
pub struct ChannelMeterMsg {
    pub idx: i32,
    #[data]
    pub db: f64,
}

#[derive(Clone, Debug, Copy, Coalescible)]
pub struct MasterMeterMsg {
    pub channel: StereoChannel,
    #[data]
    pub db: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    fn scaled(self, scale: f32) -> Self {
        let scale = scale.clamp(0.0, 1.0);
        Self {
            r: (self.r as f32 * scale).round().clamp(0.0, 127.0) as u8,
            g: (self.g as f32 * scale).round().clamp(0.0, 127.0) as u8,
            b: (self.b as f32 * scale).round().clamp(0.0, 127.0) as u8,
        }
    }
}

#[derive(Clone, Debug, Coalescible)]
pub struct TopScribbleStripLine1TextMsg {
    pub idx: i32,
    #[data]
    pub text: String,
}

#[derive(Clone, Debug, Coalescible)]
pub struct TopScribbleStripLine2TextMsg {
    pub idx: i32,
    #[data]
    pub text: String,
}

#[derive(Clone, Debug, Coalescible)]
pub struct BottomScribbleStripLine1TextMsg {
    pub idx: i32,
    #[data]
    pub text: String,
}

#[derive(Clone, Debug, Coalescible)]
pub struct BottomScribbleStripLine2TextMsg {
    pub idx: i32,
    #[data]
    pub text: String,
}

#[derive(Clone, Debug, Coalescible)]
pub struct TopScribbleStripColorMsg {
    pub idx: i32,
    #[data]
    pub color: Color,
}

#[derive(Clone, Debug, Coalescible)]
pub struct SevenSegmentDisplayMsg {
    #[data]
    pub text: String,
}

#[derive(Clone, Debug, Coalescible)]
pub struct TouchScreenSetTextMsg {
    pub slot: Slot,
    pub daw_id: DawId,
    pub row: usize,
    pub column: usize,
    pub layer: TouchScreenLayer,
    #[data]
    pub text: String,
}

#[derive(Clone, Debug, Coalescible)]
pub struct TouchScreenSetButtonBehaviorMsg {
    pub slot: Slot,
    pub daw_id: DawId,
    pub row: usize,
    pub column: usize,
    pub layer: TouchScreenLayer,
    #[data]
    pub midi_channel: u8,
    #[data]
    pub note: u8,
}

#[derive(Clone, Copy, Debug, EnumFrom)]
pub enum UpstreamMsg {
    Barrier(Barrier),

    // Channel strip messages
    ChannelFader(ChannelFaderMsg),
    MasterFader(MasterFaderMsg),
    EncoderTurnInc(EncoderTurnCW),
    EncoderTurnDec(EncoderTurnCCW),
    EncoderPress(EncoderPressMsg),
    EncoderRelease(EncoderReleaseMsg),
    MutePress(MutePress),
    MuteRelease(MuteRelease),
    SoloPress(SoloPress),
    SoloRelease(SoloRelease),
    ArmPress(ArmPress),
    ArmRelease(ArmRelease),
    SelectPress(SelectPress),
    SelectRelease(SelectRelease),

    // Encoder assign messages
    TrackPress,
    TrackRelease,
    PanPress,
    PanRelease,
    EQPress,
    EQRelease,
    SendPress,
    SendRelease,
    PluginPress,
    PluginRelease,
    InstPress,
    InstRelease,

    // View messages
    GlobalPress,
    GlobalRelease,
    MIDITracksPress,
    MIDITracksRelease,
    InputsPress,
    InputsRelease,
    AudioTracksPress,
    AudioTracksRelease,
    AudioInstPress,
    AudioInstRelease,
    AuxPress,
    AuxRelease,
    BusesPress,
    BusesRelease,
    OutputsPress,
    OutputsRelease,
    UserPress,
    UserRelease,
    ShiftPress,   //TODO: what does this map to?
    ShiftRelease, //TODO: what does this map to?
}

#[derive(Clone, Debug, EnumFrom)]
pub enum TopScribbleStripMsg {
    Line1Text(TopScribbleStripLine1TextMsg),
    Line2Text(TopScribbleStripLine2TextMsg),
    BackgroundColor(TopScribbleStripColorMsg),
}

#[derive(Clone, Debug, EnumFrom)]
pub enum BottomScribbleStripMsg {
    Line1Text(BottomScribbleStripLine1TextMsg),
    Line2Text(BottomScribbleStripLine2TextMsg),
}

#[derive(Clone, Debug, EnumFrom, CoalescibleEnum)]
pub enum DownstreamMsg {
    #[enum_from]
    #[nocoalesce]
    Barrier(Barrier),

    // Channel strip messages
    #[enum_from]
    ChannelFader(ChannelFaderMsg),
    #[enum_from]
    MasterFader(MasterFaderMsg),
    #[enum_from]
    EncoderRingLED(EncoderRingMsg),
    #[enum_from]
    MuteLED(MuteLEDMsg),
    #[enum_from]
    SoloLED(SoloLEDMsg),
    #[enum_from]
    ArmLED(ArmLEDMsg),
    #[enum_from]
    SelectLED(SelectLEDMsg),
    #[enum_from]
    ChannelMeter(ChannelMeterMsg),
    #[enum_from]
    MasterMeter(MasterMeterMsg),

    // Scribble strip messages
    #[enum_from]
    TopScribbleStripLine1Text(TopScribbleStripLine1TextMsg),
    #[enum_from]
    TopScribbleStripLine2Text(TopScribbleStripLine2TextMsg),
    #[enum_from]
    TopScribbleStripBackgroundColor(TopScribbleStripColorMsg),
    #[enum_from]
    #[nocoalesce]
    TopScribbleStripBatch(Vec<TopScribbleStripMsg>),

    #[enum_from]
    BottomScribbleStripLine1Text(BottomScribbleStripLine1TextMsg),
    #[enum_from]
    BottomScribbleStripLine2Text(BottomScribbleStripLine2TextMsg),
    #[enum_from]
    #[nocoalesce]
    BottomScribbleStripBatch(Vec<BottomScribbleStripMsg>),

    #[enum_from]
    SevenSegmentDisplay(SevenSegmentDisplayMsg),

    #[enum_from]
    TouchScreenSetText(TouchScreenSetTextMsg),
    #[enum_from]
    #[nocoalesce]
    TouchScreenTextBatch(Vec<TouchScreenSetTextMsg>),

    #[enum_from]
    TouchScreenButtonBehavior(TouchScreenSetButtonBehaviorMsg),
    #[enum_from]
    #[nocoalesce]
    TouchScreenButtonBehaviorBatch(Vec<TouchScreenSetButtonBehaviorMsg>),

    // Encoder assign messages
    Track(LEDState),
    Pan(LEDState),
    EQ(LEDState),
    Send(LEDState),
    Plugin(LEDState),
    Inst(LEDState),

    // View messages
    Global(LEDState),
    MIDITracks(LEDState),
    Inputs(LEDState),
    AudioTracks(LEDState),
    AudioInst(LEDState),
    Aux(LEDState),
    Buses(LEDState),
    Outputs(LEDState),
    User(LEDState),
}

pub struct Fader {
    base: Arc<Mutex<MidiDevice>>,
    channel: Channel,
}

impl Bind<u16> for Fader {
    fn bind<F>(&mut self, callback: F)
    where
        F: FnMut(u16) + 'static + std::marker::Send,
    {
        PitchBendBuilder {
            device: &mut self.base.lock().unwrap(),
            spec: PitchBend {
                channel: self.channel.get(),
            },
        }
        .bind(callback)
    }
}

impl Set<i32> for Fader {
    type Error = MidiError;
    fn set(&mut self, value: i32) -> Result<(), Self::Error> {
        PitchBendBuilder {
            device: &mut self.base.lock().unwrap(),
            spec: PitchBend {
                channel: self.channel.get(),
            },
        }
        .set(value as u16)
    }
}

pub struct Encoder {
    base: Arc<Mutex<MidiDevice>>,
    knob_cc: u8,
    click_note: u8,
    led_cc: u8,
}

impl Encoder {
    fn bind_turn<F>(&mut self, mut callback: F)
    where
        F: FnMut(u8) + 'static + std::marker::Send,
    {
        ControlChangeBuilder {
            device: &mut self.base.lock().unwrap(),
            spec: ControlChange {
                channel: 0,
                controller_number: self.knob_cc,
            },
        }
        .bind(move |value| {
            callback(value);
        })
    }

    fn bind_press<F>(&mut self, mut callback: F)
    where
        F: FnMut(u8) + 'static + std::marker::Send,
    {
        NoteOnBuilder {
            device: &mut self.base.lock().unwrap(),
            spec: NoteOn {
                channel: 0,
                key_number: self.click_note,
            },
        }
        .bind(move |value| {
            callback(value);
        })
    }

    fn bind_release<F>(&mut self, mut callback: F)
    where
        F: FnMut(u8) + 'static + std::marker::Send,
    {
        NoteOffBuilder {
            device: &mut self.base.lock().unwrap(),
            spec: NoteOff {
                channel: 0,
                key_number: self.click_note,
            },
        }
        .bind(move |value| {
            callback(value);
        })
    }

    fn set(&mut self, mode: EncoderRingMode, val: u8) -> Result<(), MidiError> {
        let new_val = match mode {
            EncoderRingMode::Point => val & 0xf,
            EncoderRingMode::FromCenter => 0x01 << 4 | (val & 0xf),
            EncoderRingMode::FromLeft => 0x02 << 4 | (val & 0xf),
            EncoderRingMode::Width => 0x03 << 4 | (val & 0xf),
        };

        ControlChangeBuilder {
            device: &mut self.base.lock().unwrap(),
            spec: ControlChange {
                channel: 0,
                controller_number: self.led_cc,
            },
        }
        .set(new_val)
    }
}

pub struct Button {
    base: Arc<Mutex<MidiDevice>>,
    channel: Channel,
    midi_note: u8,
}

impl Button {
    fn bind_press<F>(&mut self, mut callback: F)
    where
        F: FnMut(u8) + 'static + std::marker::Send,
    {
        NoteOnBuilder {
            device: &mut self.base.lock().unwrap(),
            spec: NoteOn {
                channel: self.channel.get(),
                key_number: self.midi_note,
            },
        }
        .bind(move |velocity| {
            callback(velocity);
        })
    }

    fn bind_release<F>(&mut self, mut callback: F)
    where
        F: FnMut(u8) + 'static + std::marker::Send,
    {
        NoteOffBuilder {
            device: &mut self.base.lock().unwrap(),
            spec: NoteOff {
                channel: self.channel.get(),
                key_number: self.midi_note,
            },
        }
        .bind(move |velocity| {
            callback(velocity);
        })
    }
}

impl Set<LEDState> for Button {
    type Error = MidiError;
    fn set(&mut self, value: LEDState) -> Result<(), Self::Error> {
        NoteOnBuilder {
            device: &mut self.base.lock().unwrap(),
            spec: NoteOn {
                channel: self.channel.get(),
                key_number: self.midi_note,
            },
        }
        .set(match value {
            LEDState::Off => 0,
            LEDState::On => 127,
            LEDState::Flash => 1,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MeterLevelCode {
    L0 = 0x0, // < -60 dB
    L1 = 0x1, // >= -60
    L2 = 0x2, // >= -50
    L3 = 0x3, // >= -40
    L4 = 0x4, // >= -30
    L5 = 0x5, // >= -20
    L6 = 0x6, // >= -14
    L7 = 0x7, // >= -10
    L8 = 0x8, // >= -8
    L9 = 0x9, // >= -6
    LA = 0xA, // >= -4
    LB = 0xB, // >= -2
    LC = 0xC, // >=  0
    LD = 0xD, // >   0 (100% / over 0 dB)
}

impl MeterLevelCode {
    #[inline]
    pub fn as_nibble(self) -> u8 {
        self as u8
    }
}

/// Map dB value to MCU meter nibble (0x0..0xD).
/// Ignores 0xE/0xF overload control codes as requested.
fn db_to_meter_code(db: f64) -> MeterLevelCode {
    if !db.is_finite() {
        return MeterLevelCode::L0;
    }

    if db > 0.0 {
        MeterLevelCode::LD
    } else if db >= 0.0 {
        MeterLevelCode::LC
    } else if db >= -2.0 {
        MeterLevelCode::LB
    } else if db >= -4.0 {
        MeterLevelCode::LA
    } else if db >= -6.0 {
        MeterLevelCode::L9
    } else if db >= -8.0 {
        MeterLevelCode::L8
    } else if db >= -10.0 {
        MeterLevelCode::L7
    } else if db >= -14.0 {
        MeterLevelCode::L6
    } else if db >= -20.0 {
        MeterLevelCode::L5
    } else if db >= -30.0 {
        MeterLevelCode::L4
    } else if db >= -40.0 {
        MeterLevelCode::L3
    } else if db >= -50.0 {
        MeterLevelCode::L2
    } else if db >= -60.0 {
        MeterLevelCode::L1
    } else {
        MeterLevelCode::L0
    }
}

/// Build full channel-pressure value `0xsv`:
/// - `strip` is channel strip index 0..=7
/// - low nibble is meter code
pub fn meter_byte(idx: u8, db: f64) -> u8 {
    let s = (idx & 0x0F) << 4;
    let v = db_to_meter_code(db).as_nibble() & 0x0F;
    s | v
}

pub struct ChannelMeters {
    base: Arc<Mutex<MidiDevice>>,
}

impl ChannelMeters {
    fn new(base: Arc<Mutex<MidiDevice>>) -> Self {
        Self { base }
    }
}

// NOTE: channel meters retain their last setting for about 300ms
impl Set<ChannelMeterMsg> for ChannelMeters {
    type Error = MidiError;

    fn set(&mut self, msg: ChannelMeterMsg) -> Result<(), Self::Error> {
        ChannelPressureBuilder {
            device: &mut self.base.lock().unwrap(),
            spec: ChannelPressure { channel: 0 },
        }
        .set(meter_byte(msg.idx as u8, msg.db))
    }
}

pub struct MasterMeters {
    base: Arc<Mutex<MidiDevice>>,
}

impl MasterMeters {
    fn new(base: Arc<Mutex<MidiDevice>>) -> Self {
        Self { base }
    }
}

// NOTE: master meters retaing their last setting forever
impl Set<MasterMeterMsg> for MasterMeters {
    type Error = MidiError;

    fn set(&mut self, msg: MasterMeterMsg) -> Result<(), Self::Error> {
        let idx = match msg.channel {
            StereoChannel::Left => 0,
            StereoChannel::Right => 1,
        };
        ChannelPressureBuilder {
            device: &mut self.base.lock().unwrap(),
            spec: ChannelPressure { channel: 1 },
        }
        .set(meter_byte(idx, msg.db))
    }
}

pub struct TopScribbleStrips {
    base: Arc<Mutex<MidiDevice>>,
    line_1: Vec<String>,
    line_2: Vec<String>,
    background_color: Vec<Color>,
}

type ScribbleStripError = String;

impl TopScribbleStrips {
    fn new(base: Arc<Mutex<MidiDevice>>, num_channels: usize) -> Self {
        Self {
            base,
            line_1: vec![String::new(); num_channels],
            line_2: vec![String::new(); num_channels],
            background_color: vec![Color { r: 0, g: 0, b: 0 }; num_channels],
        }
    }

    // TODO: need to stop writing the whole thing: only write the parts that have changed
    fn write(&self) -> Result<(), ScribbleStripError> {
        // First, we send the text
        // Sysex message is made of a header + text formatted as ascii
        // Each scribble stip consumes 7 ascii bytes
        // Since we are updating everything, we can just send the full text for both lines of all 8
        // strips in one message.

        let mut msg_bytes = vec![0xf0, 0x00, 0x00, 0x66, 0x14, 0x12, 0x00];
        for i in 0..self.line_2.len() {
            let line2_bytes = compact_to_7_bytes(&self.line_2[i]);
            // Append line1 and line2 bytes to the message
            msg_bytes.extend_from_slice(&line2_bytes);
        }
        for i in 0..self.line_1.len() {
            let line1_bytes = compact_to_7_bytes(&self.line_1[i]);
            // Append line1 and line2 bytes to the message
            msg_bytes.extend_from_slice(&line1_bytes);
        }
        // Finally, append the sysex end byte
        msg_bytes.push(0xf7);

        self.base
            .lock()
            .unwrap()
            .midi_out
            .send(&msg_bytes)
            .map_err(|e| format!("Failed to send SysEx message: {}", e))?;

        // Now we do the same for the background color and line modes, which are sent in a separate message
        let mut msg_bytes = vec![0xf0, 0x00, 0x02, 0x4e, 0x16, 0x14];
        for i in 0..self.line_1.len() {
            let color = self.background_color[i].scaled(0.2);
            msg_bytes.push(color.r.clamp(0, 0x7f));
            msg_bytes.push(color.g.clamp(0, 0x7f));
            msg_bytes.push(color.b.clamp(0, 0x7f));
        }
        msg_bytes.push(0xf7);

        self.base
            .lock()
            .unwrap()
            .midi_out
            .send(&msg_bytes)
            .map_err(|e| format!("Failed to send SysEx message: {}", e))
    }
}

impl Set<TopScribbleStripLine1TextMsg> for TopScribbleStrips {
    type Error = ScribbleStripError;
    fn set(&mut self, value: TopScribbleStripLine1TextMsg) -> Result<(), Self::Error> {
        self.line_1[value.idx as usize] = value.text;
        self.write()
    }
}

impl Set<TopScribbleStripLine2TextMsg> for TopScribbleStrips {
    type Error = ScribbleStripError;
    fn set(&mut self, value: TopScribbleStripLine2TextMsg) -> Result<(), Self::Error> {
        self.line_2[value.idx as usize] = value.text;
        self.write()
    }
}

impl Set<TopScribbleStripColorMsg> for TopScribbleStrips {
    type Error = ScribbleStripError;
    fn set(&mut self, value: TopScribbleStripColorMsg) -> Result<(), Self::Error> {
        self.background_color[value.idx as usize] = value.color;
        self.write()
    }
}

impl Set<Vec<TopScribbleStripMsg>> for TopScribbleStrips {
    type Error = ScribbleStripError;
    fn set(&mut self, value: Vec<TopScribbleStripMsg>) -> Result<(), Self::Error> {
        for msg in value {
            match msg {
                TopScribbleStripMsg::Line1Text(msg) => {
                    self.line_1[msg.idx as usize] = msg.text;
                }
                TopScribbleStripMsg::Line2Text(msg) => {
                    self.line_2[msg.idx as usize] = msg.text;
                }
                TopScribbleStripMsg::BackgroundColor(msg) => {
                    self.background_color[msg.idx as usize] = msg.color;
                }
            }
        }
        self.write()
    }
}

pub struct BottomScribbleStrips {
    base: Arc<Mutex<MidiDevice>>,
    line_1: Vec<String>,
    line_2: Vec<String>,
}

impl BottomScribbleStrips {
    fn new(base: Arc<Mutex<MidiDevice>>, num_channels: usize) -> Self {
        Self {
            base,
            line_1: vec![String::new(); num_channels],
            line_2: vec![String::new(); num_channels],
        }
    }

    // TODO: need to stop writing the whole thing: only write the parts that have changed
    fn write(&self) -> Result<(), ScribbleStripError> {
        // First, we send the text
        // Sysex message is made of a header + text formatted as ascii
        // Each scribble stip consumes 7 ascii bytes
        // Since we are updating everything, we can just send the full text for both lines of all 8
        // strips in one message.

        let mut msg_bytes = vec![0xf0, 0x00, 0x02, 0x4e, 0x15, 0x13, 0x00];
        for i in 0..self.line_1.len() {
            let line1_bytes = compact_to_7_bytes(&self.line_1[i]);
            // Append line1 and line2 bytes to the message
            msg_bytes.extend_from_slice(&line1_bytes);
        }
        for i in 0..self.line_2.len() {
            let line2_bytes = compact_to_7_bytes(&self.line_2[i]);
            // Append line1 and line2 bytes to the message
            msg_bytes.extend_from_slice(&line2_bytes);
        }
        // Finally, append the sysex end byte
        msg_bytes.push(0xf7);

        self.base
            .lock()
            .unwrap()
            .midi_out
            .send(&msg_bytes)
            .map_err(|e| format!("Failed to send SysEx message: {}", e))
    }
}

impl Set<BottomScribbleStripLine1TextMsg> for BottomScribbleStrips {
    type Error = ScribbleStripError;
    fn set(&mut self, value: BottomScribbleStripLine1TextMsg) -> Result<(), Self::Error> {
        self.line_1[value.idx as usize] = value.text;
        self.write()
    }
}

impl Set<BottomScribbleStripLine2TextMsg> for BottomScribbleStrips {
    type Error = ScribbleStripError;
    fn set(&mut self, value: BottomScribbleStripLine2TextMsg) -> Result<(), Self::Error> {
        self.line_2[value.idx as usize] = value.text;
        self.write()
    }
}

impl Set<Vec<BottomScribbleStripMsg>> for BottomScribbleStrips {
    type Error = ScribbleStripError;
    fn set(&mut self, value: Vec<BottomScribbleStripMsg>) -> Result<(), Self::Error> {
        for msg in value {
            match msg {
                BottomScribbleStripMsg::Line1Text(msg) => {
                    self.line_1[msg.idx as usize] = msg.text;
                }
                BottomScribbleStripMsg::Line2Text(msg) => {
                    self.line_2[msg.idx as usize] = msg.text;
                }
            }
        }
        self.write()
    }
}

pub struct SevenSegmentDisplay {
    base: Arc<Mutex<MidiDevice>>,
}

impl SevenSegmentDisplay {
    fn new(base: Arc<Mutex<MidiDevice>>, num_digits: usize) -> Self {
        Self { base }
    }

    fn set(&self, text: &str) -> Result<(), String> {
        let compacted = compact_to_7_bytes(text);
        println!("SEVEN SEG: sending text: {:?} -> {:?}", text, compacted);
        for (i, b) in compacted.iter().enumerate() {
            // if i < self.digits.len() {
            //     self.digits[i] = *b;
            // }
            println!("NEW VAL: {:x}", b - 40);
            ControlChangeBuilder {
                device: &mut self.base.lock().unwrap(),
                spec: ControlChange {
                    channel: 0,
                    controller_number: 0x49 - i as u8,
                },
            }
            // .set(*b - 40)
            .set(0x3d + i as u8)
            .map_err(|e| format!("Failed to send 7-segment display message"))?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct TouchScreenButton {
    text: String,
    behavior: Option<(u8, u8)>, // (midi_channel, note)
}

impl TouchScreenButton {
    fn new() -> Self {
        Self {
            text: "".to_string(),
            behavior: None,
        }
    }
}

const TOUCHSCREEN_COLUMNS: usize = 6;
const TOUCHSCREEN_ROWS: usize = 4;
const TOUCHSCREEN_LAYERS: usize = 5;
const TOUCHSCREEN_BUTTONS: usize = TOUCHSCREEN_COLUMNS * TOUCHSCREEN_ROWS * TOUCHSCREEN_LAYERS;

struct TouchScreenSetTextContainer {
    slot: Slot,
    daw_id: DawId,
    row: usize,
    column: usize,
    layer: TouchScreenLayer,
    text: String,
}

impl TouchScreenSetTextContainer {
    fn button_idx(&self) -> usize {
        let layer_idx = match self.layer {
            TouchScreenLayer::Blue => 0,
            TouchScreenLayer::Green => 1,
            TouchScreenLayer::Yellow => 2,
            TouchScreenLayer::User1 => 3,
            TouchScreenLayer::User2 => 4,
        };

        layer_idx * TOUCHSCREEN_ROWS * TOUCHSCREEN_COLUMNS
            + self.row * TOUCHSCREEN_COLUMNS
            + self.column
    }

    fn part(&self) -> usize {
        match self.layer {
            TouchScreenLayer::Blue => self.row / 2,
            TouchScreenLayer::Green => 2 + self.row / 2,
            TouchScreenLayer::Yellow => 4 + self.row / 2,
            TouchScreenLayer::User1 => 6 + self.row / 2,
            TouchScreenLayer::User2 => 8 + self.row / 2,
        }
    }
}

struct TouchScreenSetButtonBehaviorContainer {
    slot: Slot,
    daw_id: DawId,
    row: usize,
    column: usize,
    layer: TouchScreenLayer,
    midi_channel: u8,
    note: u8,
}

impl TouchScreenSetButtonBehaviorContainer {
    fn button_idx(&self) -> usize {
        let layer_idx = match self.layer {
            TouchScreenLayer::Blue => 0,
            TouchScreenLayer::Green => 1,
            TouchScreenLayer::Yellow => 2,
            TouchScreenLayer::User1 => 3,
            TouchScreenLayer::User2 => 4,
        };

        layer_idx * TOUCHSCREEN_ROWS * TOUCHSCREEN_COLUMNS
            + self.row * TOUCHSCREEN_COLUMNS
            + self.column
    }

    fn part(&self) -> usize {
        match self.layer {
            TouchScreenLayer::Blue => self.row / 2,
            TouchScreenLayer::Green => 2 + self.row / 2,
            TouchScreenLayer::Yellow => 4 + self.row / 2,
            TouchScreenLayer::User1 => 6 + self.row / 2,
            TouchScreenLayer::User2 => 8 + self.row / 2,
        }
    }
}

type TouchScreenError = String;

pub struct TouchScreen {
    midi: Arc<Mutex<MidiDevice>>,
    buttons: Vec<TouchScreenButton>,
}

impl TouchScreen {
    fn new(midi: Arc<Mutex<MidiDevice>>) -> Self {
        Self {
            midi,
            buttons: vec![TouchScreenButton::new(); TOUCHSCREEN_BUTTONS],
        }
    }

    fn write_button_text(
        &mut self,
        messages: Vec<TouchScreenSetTextContainer>,
    ) -> Result<(), TouchScreenError> {
        let mut parts_need_set: [bool; 10] = [false; 10]; // Placeholder for parts that need to be set

        // Set text for each message in self.butons
        for message in messages.iter() {
            let idx = message.button_idx();
            self.buttons[idx].text = message.text.clone();
            parts_need_set[message.part()] = true;
        }

        for part in 0..parts_need_set.len() {
            if parts_need_set[part] {
                // We don't really know what this message does but iMAP always sends it.
                const INIT_BYTES: [u8; 4] = [0xef, 0x7f, 0x7f, 0xf7];
                self.midi
                    .lock()
                    .unwrap()
                    .midi_out
                    .send(&INIT_BYTES)
                    .map_err(|e| format!("Failed to send touch screen init message: {}", e))?;

                // Inform V1 that we are macos. Do we need this? Who knows!
                const MACOS_BYTES: [u8; 4] = [0xec, 0x2a, 0x01, 0xf7];
                self.midi
                    .lock()
                    .unwrap()
                    .midi_out
                    .send(&MACOS_BYTES)
                    .map_err(|e| format!("Failed to send touchscreen macos message: {}", e))?;

                // Touchscreen text message is formatted as:
                // [6-byte header] [slot] [daw id] [part #] [n buttons] [n lines] [bytes per line] [text...]
                const TOUCHSCREEN_TEXT_HEADER: [u8; 6] = [0xf0, 0x1d, 0x03, 0x10, 0x09, 0x26];
                let mut touchscreen_text_sysex = TOUCHSCREEN_TEXT_HEADER.to_vec();
                let slot_id: u8 = match messages[0].slot {
                    Slot::DAW1 => 0x01,
                    Slot::DAW2 => 0x02,
                    Slot::DAW3 => 0x03,
                };
                touchscreen_text_sysex.push(slot_id);
                let daw_id: u8 = match messages[0].daw_id {
                    DawId::Bitwig => 0x01,
                    DawId::Cubase => 0x02,
                    DawId::ProTools => 0x03,
                    DawId::Logic => 0x04,
                    DawId::Live => 0x05,
                    DawId::Reaper => 0x06,
                    DawId::Reason => 0x07,
                    DawId::StudioOne => 0x08,
                    DawId::FLStudio => 0x09,
                    DawId::Cakewalk => 0x0a,
                    DawId::DigitalPerformer => 0x0b,
                    DawId::Samplitude => 0x0c,
                    DawId::Harrison => 0x0d,
                    DawId::Nuendo => 0x0e,
                    DawId::Audition => 0x0f,
                    DawId::Tracktion => 0x10,
                    DawId::Ability => 0x11,
                    DawId::Luna => 0x12,
                };
                touchscreen_text_sysex.push(daw_id);
                touchscreen_text_sysex.push(part as u8); // part #
                touchscreen_text_sysex.push(0x0c); // # buttons
                touchscreen_text_sysex.push(0x02); // lines
                touchscreen_text_sysex.push(0x08); // bytes per line
                for i in part * 12..(part + 1) * 12 {
                    let mut padded = [0x20u8; 16];
                    let src = self.buttons[i].text.as_bytes();
                    let n = src.len().min(16);
                    padded[..n].copy_from_slice(&src[..n]);
                    touchscreen_text_sysex.extend_from_slice(&padded);
                }
                touchscreen_text_sysex.push(0xf7); // end of sysex

                self.midi
                    .lock()
                    .unwrap()
                    .midi_out
                    .send(&touchscreen_text_sysex)
                    .map_err(|e| format!("Failed to send touchscreen text message: {}", e))?;

                // Send switch slot message to cause touchscreen to refresh showing updated text
                let switch_slot_sysex = vec![0xec, 0x22, slot_id << 5 | daw_id];
                self.midi
                    .lock()
                    .unwrap()
                    .midi_out
                    .send(&switch_slot_sysex)
                    .map_err(|e| format!("Failed to send touchscreen switch slot message: {}", e))?
            }
        }
        Ok(())
    }

    fn write_button_behavior(
        &mut self,
        messages: Vec<TouchScreenSetButtonBehaviorContainer>,
    ) -> Result<(), TouchScreenError> {
        let mut parts_need_set: [bool; 10] = [false; 10]; // Placeholder for parts that need to be set

        // Set text for each message in self.butons
        for message in messages.iter() {
            let idx = message.button_idx();
            self.buttons[idx].behavior = (message.midi_channel, message.note).into();
            parts_need_set[message.part()] = true;
        }

        for part in 0..parts_need_set.len() {
            if parts_need_set[part] {
                // We don't really know what this message does but iMAP always sends it.
                const INIT_BYTES: [u8; 4] = [0xef, 0x7f, 0x7f, 0xf7];
                self.midi
                    .lock()
                    .unwrap()
                    .midi_out
                    .send(&INIT_BYTES)
                    .map_err(|e| format!("Failed to send touch screen init message: {}", e))?;

                // Inform V1 that we are macos. Do we need this? Who knows!
                const MACOS_BYTES: [u8; 4] = [0xec, 0x2a, 0x01, 0xf7];
                self.midi
                    .lock()
                    .unwrap()
                    .midi_out
                    .send(&MACOS_BYTES)
                    .map_err(|e| format!("Failed to send touchscreen macos message: {}", e))?;

                // Touchscreen text message is formatted as:
                // [6-byte header] [slot_daw] [part] [record_size] | [2-byte behavior type] [midi channel] [note] 7f 00 00 00 |<per button> 7f
                // 28 records per message
                const TOUCHSCREEN_BEHAVIOR_HEADER: [u8; 6] = [0xf0, 0x1d, 0x03, 0x10, 0x09, 0x25];
                let mut touchscreen_behavior_sysex = TOUCHSCREEN_BEHAVIOR_HEADER.to_vec();
                let slot_id: u8 = match messages[0].slot {
                    Slot::DAW1 => 0x01,
                    Slot::DAW2 => 0x02,
                    Slot::DAW3 => 0x03,
                };
                touchscreen_behavior_sysex.push(slot_id);
                let daw_id: u8 = match messages[0].daw_id {
                    DawId::Bitwig => 0x01,
                    DawId::Cubase => 0x02,
                    DawId::ProTools => 0x03,
                    DawId::Logic => 0x04,
                    DawId::Live => 0x05,
                    DawId::Reaper => 0x06,
                    DawId::Reason => 0x07,
                    DawId::StudioOne => 0x08,
                    DawId::FLStudio => 0x09,
                    DawId::Cakewalk => 0x0a,
                    DawId::DigitalPerformer => 0x0b,
                    DawId::Samplitude => 0x0c,
                    DawId::Harrison => 0x0d,
                    DawId::Nuendo => 0x0e,
                    DawId::Audition => 0x0f,
                    DawId::Tracktion => 0x10,
                    DawId::Ability => 0x11,
                    DawId::Luna => 0x12,
                };
                touchscreen_behavior_sysex.push(slot_id << 5 | daw_id);
                touchscreen_behavior_sysex.push(part as u8); // part #
                touchscreen_behavior_sysex.push(0x08); // record_size
                const EMPTY_BUTTON: [u8; 8] = [0x20, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
                for i in part * 28..(part + 1) * 28 {
                    match self.buttons[i].behavior {
                        Some((midi_channel, note)) => {
                            touchscreen_behavior_sysex.push(0x09); // This may need DAW_id muxed in
                            touchscreen_behavior_sysex.push(0x09);
                            touchscreen_behavior_sysex.push(midi_channel + 1);
                            touchscreen_behavior_sysex.push(note);
                            touchscreen_behavior_sysex.push(0x7f);
                            touchscreen_behavior_sysex.push(0x00);
                            touchscreen_behavior_sysex.push(0x00);
                            touchscreen_behavior_sysex.push(0x00);
                        }
                        None => {
                            touchscreen_behavior_sysex.extend_from_slice(EMPTY_BUTTON.as_slice())
                        }
                    }
                }
                touchscreen_behavior_sysex.push(0xf7); // end of sysex

                self.midi
                    .lock()
                    .unwrap()
                    .midi_out
                    .send(&touchscreen_behavior_sysex)
                    .map_err(|e| format!("Failed to send touchscreen text message: {}", e))?;

                // Send switch slot message to cause touchscreen to refresh showing updated text
                let switch_slot_sysex = vec![0xec, 0x22, slot_id << 5 | daw_id];
                self.midi
                    .lock()
                    .unwrap()
                    .midi_out
                    .send(&switch_slot_sysex)
                    .map_err(|e| format!("Failed to send touchscreen switch slot message: {}", e))?
            }
        }
        Ok(())
    }
}

pub struct V1mBuilder {
    pub main_midi: Arc<Mutex<MidiDevice>>,
    pub config_midi: Arc<Mutex<MidiDevice>>,
    pub num_channels: usize,
}

impl V1mBuilder {
    pub fn new(
        main_midi_in_port: MidiInputPort,
        main_midi_out: MidiOutputConnection,
        config_midi_in_port: MidiInputPort,
        config_midi_out: MidiOutputConnection,
        num_channels: usize,
    ) -> Self {
        Self {
            main_midi: Arc::new(Mutex::new(MidiDevice::new(
                "v1m",
                main_midi_in_port,
                main_midi_out,
            ))),
            config_midi: Arc::new(Mutex::new(MidiDevice::new(
                "v1m-config",
                config_midi_in_port,
                config_midi_out,
            ))),
            num_channels,
        }
    }

    pub fn build(self, input: Receiver<DownstreamMsg>, upstream: Sender<UpstreamMsg>) {
        let mut faders = Vec::with_capacity(self.num_channels);
        for i in 0..self.num_channels {
            let mut f = Fader {
                base: self.main_midi.clone(),
                channel: Channel::new(i as u8),
            };
            let upstream_fader = upstream.clone();
            f.bind(move |value| {
                let _ = upstream_fader.send(UpstreamMsg::from(ChannelFaderMsg {
                    idx: i as i32,
                    value: value as f64 / 16383.0, // TODO: check this...
                }));
            });
            faders.push(f);
        }
        let mut master_fader = Fader {
            base: self.main_midi.clone(),
            channel: Channel::new(8),
        };
        {
            let upstream_fader = upstream.clone();
            master_fader.bind(move |value| {
                let _ = upstream_fader.send(UpstreamMsg::from(MasterFaderMsg {
                    value: value as f64 / 16383.0, // TODO: check this...
                }));
            });
        }
        let mut encoders = Vec::with_capacity(self.num_channels);
        for i in 0..self.num_channels {
            let mut e = Encoder {
                base: self.main_midi.clone(),
                knob_cc: 0x10 + i as u8,
                click_note: 0x20 + i as u8,
                led_cc: 0x30 + i as u8,
            };
            let upstream_turn = upstream.clone();
            e.bind_turn(move |value| match value {
                // TODO: 1 means CW slow all the way up to at least 5 is CW fast(er)
                1..6 => upstream_turn
                    .send(UpstreamMsg::from(EncoderTurnCW {
                        idx: i as i32,
                        accel: value,
                    }))
                    .unwrap(),
                // Similarly, 65 seems to mean slow but we can go all the way up to at least 68
                65..69 => upstream_turn
                    .send(UpstreamMsg::from(EncoderTurnCCW {
                        idx: i as i32,
                        accel: (value - 64),
                    }))
                    .unwrap(),
                _ => println!("HERE: Unexpected encoder turn value: {}\n", value),
            });
            let upstream_press = upstream.clone();
            e.bind_press(move |_value| {
                upstream_press
                    .send(UpstreamMsg::from(EncoderPressMsg { idx: i as i32 }))
                    .unwrap();
            });
            let upstream_release = upstream.clone();
            e.bind_release(move |_value| {
                upstream_release
                    .send(EncoderReleaseMsg { idx: i as i32 }.into())
                    .unwrap();
            });
            encoders.push(e);
        }
        let mut mutes = Vec::with_capacity(self.num_channels);
        for i in 0..self.num_channels {
            // TODO: repeat this for the other button types
            let mut b = Button {
                base: self.main_midi.clone(),
                channel: Channel::new(0),
                midi_note: 16 + i as u8,
            };
            let upstream_press = upstream.clone();
            b.bind_press(move |velocity| match velocity {
                0 => upstream_press
                    .send(UpstreamMsg::from(MuteRelease { idx: i as i32 }))
                    .unwrap(),
                127 => upstream_press
                    .send(UpstreamMsg::from(MutePress { idx: i as i32 }))
                    .unwrap(),
                _ => panic!("Unexpected mute button velocity: {}", velocity),
            });
            mutes.push(b);
        }
        let mut solos = Vec::with_capacity(self.num_channels);
        for i in 0..self.num_channels {
            let mut b = Button {
                base: self.main_midi.clone(),
                channel: Channel::new(0),
                midi_note: 8 + i as u8,
            };
            let upstream_press = upstream.clone();
            b.bind_press(move |velocity| match velocity {
                0 => upstream_press
                    .send(SoloRelease { idx: i as i32 }.into())
                    .unwrap(),
                127 => upstream_press
                    .send(SoloPress { idx: i as i32 }.into())
                    .unwrap(),
                _ => panic!("Unexpected solo button velocity: {}", velocity),
            });
            solos.push(b);
        }
        let mut arms = Vec::with_capacity(self.num_channels);
        for i in 0..self.num_channels {
            let mut b = Button {
                base: self.main_midi.clone(),
                channel: Channel::new(0),
                midi_note: i as u8,
            };
            let upstream_press = upstream.clone();
            b.bind_press(move |velocity| match velocity {
                0 => upstream_press
                    .send(UpstreamMsg::from(ArmRelease { idx: i as i32 }))
                    .unwrap(),
                127 => upstream_press
                    .send(UpstreamMsg::from(ArmPress { idx: i as i32 }))
                    .unwrap(),
                _ => panic!("Unexpected arm button velocity: {}", velocity),
            });
            arms.push(b);
        }
        let mut selects = Vec::with_capacity(self.num_channels);
        for i in 0..self.num_channels {
            let mut b = Button {
                base: self.main_midi.clone(),
                channel: Channel::new(0),
                midi_note: 24 + i as u8,
            };
            let upstream_press = upstream.clone();
            b.bind_press(move |velocity| match velocity {
                0 => upstream_press
                    .send(UpstreamMsg::from(SelectRelease { idx: i as i32 }))
                    .unwrap(),
                127 => upstream_press
                    .send(UpstreamMsg::from(SelectPress { idx: i as i32 }))
                    .unwrap(),
                _ => panic!("Unexpected select button velocity: {}", velocity),
            });
            selects.push(b);
        }
        let channel_meters = ChannelMeters::new(self.main_midi.clone());
        let master_meters = MasterMeters::new(self.main_midi.clone());
        let top_scribbles = TopScribbleStrips::new(self.main_midi.clone(), self.num_channels);
        let bottom_scribbles = BottomScribbleStrips::new(self.main_midi.clone(), self.num_channels);
        let seven_segment_display = SevenSegmentDisplay::new(self.main_midi.clone(), 7);
        let touchscreen = TouchScreen::new(self.config_midi.clone());
        // Global view
        let mut b = Button {
            base: self.main_midi.clone(),
            channel: Channel::new(0),
            midi_note: 51,
        };
        let upstream_press = upstream.clone();
        b.bind_press(move |velocity| match velocity {
            0 => upstream_press.send(UpstreamMsg::GlobalPress).unwrap(),
            127 => upstream_press.send(UpstreamMsg::GlobalRelease).unwrap(),
            _ => panic!("Unexpected global button velocity: {}", velocity),
        });
        // MIDITracks view
        let mut b = Button {
            base: self.main_midi.clone(),
            channel: Channel::new(0),
            midi_note: 62,
        };
        let upstream_press = upstream.clone();
        b.bind_press(move |velocity| match velocity {
            0 => upstream_press.send(UpstreamMsg::MIDITracksPress).unwrap(),
            127 => upstream_press.send(UpstreamMsg::MIDITracksRelease).unwrap(),
            _ => panic!("Unexpected MIDITracks button velocity: {}", velocity),
        });

        self.main_midi.lock().unwrap().run();

        let mut v1m = V1m {
            input,
            upstream,
            channel_faders: faders,
            master_fader,
            encoders,
            mutes,
            solos,
            arms,
            selects,
            channel_meters,
            master_meters,
            top_scribbles,
            bottom_scribbles,
            seven_segment_display,
            touchscreen,
        };

        thread::spawn(move || {
            loop {
                if let Ok(msg) = v1m.input.recv() {
                    match msg {
                        DownstreamMsg::Barrier(barrier_msg) => {
                            let _ = v1m.upstream.send(UpstreamMsg::Barrier(barrier_msg));
                        }
                        DownstreamMsg::ChannelFader(fader_msg) => {
                            v1m.channel_faders[fader_msg.idx as usize]
                                .set((fader_msg.value * 16383.0) as i32) // TODO: check this...
                                .unwrap();
                        }
                        DownstreamMsg::MasterFader(fader_msg) => {
                            v1m.master_fader
                                .set((fader_msg.value * 16383.0) as i32) // TODO: check this...
                                .unwrap();
                        }
                        DownstreamMsg::EncoderRingLED(encoder_led_msg) => {
                            v1m.encoders[encoder_led_msg.idx as usize]
                                .set(encoder_led_msg.mode, encoder_led_msg.val)
                                .unwrap();
                        }
                        DownstreamMsg::MuteLED(mute_msg) => {
                            v1m.mutes[mute_msg.idx as usize]
                                .set(mute_msg.state)
                                .unwrap();
                        }
                        DownstreamMsg::SoloLED(solo_msg) => {
                            v1m.solos[solo_msg.idx as usize]
                                .set(solo_msg.state)
                                .unwrap();
                        }
                        DownstreamMsg::ArmLED(arm_msg) => {
                            v1m.arms[arm_msg.idx as usize].set(arm_msg.state).unwrap();
                        }
                        DownstreamMsg::SelectLED(select_msg) => {
                            v1m.selects[select_msg.idx as usize]
                                .set(select_msg.state)
                                .unwrap();
                        }
                        DownstreamMsg::ChannelMeter(meter_msg) => {
                            v1m.channel_meters.set(meter_msg).unwrap();
                        }
                        DownstreamMsg::MasterMeter(meter_msg) => {
                            v1m.master_meters.set(meter_msg).unwrap();
                        }
                        DownstreamMsg::TopScribbleStripLine1Text(scribble_msg) => {
                            v1m.top_scribbles.set(scribble_msg).unwrap();
                        }
                        DownstreamMsg::TopScribbleStripLine2Text(scribble_msg) => {
                            v1m.top_scribbles.set(scribble_msg).unwrap();
                        }
                        DownstreamMsg::TopScribbleStripBackgroundColor(scribble_msg) => {
                            v1m.top_scribbles.set(scribble_msg).unwrap();
                        }
                        DownstreamMsg::TopScribbleStripBatch(scribble_msgs) => {
                            v1m.top_scribbles.set(scribble_msgs).unwrap();
                        }
                        DownstreamMsg::BottomScribbleStripLine1Text(scribble_msg) => {
                            v1m.bottom_scribbles.set(scribble_msg).unwrap();
                        }
                        DownstreamMsg::BottomScribbleStripLine2Text(scribble_msg) => {
                            v1m.bottom_scribbles.set(scribble_msg).unwrap();
                        }
                        DownstreamMsg::BottomScribbleStripBatch(scribble_msgs) => {
                            v1m.bottom_scribbles.set(scribble_msgs).unwrap();
                        }
                        DownstreamMsg::SevenSegmentDisplay(seven_segment_msg) => {
                            v1m.seven_segment_display
                                .set(&seven_segment_msg.text)
                                .unwrap();
                        }
                        DownstreamMsg::TouchScreenSetText(touch_msg) => {
                            v1m.touchscreen
                                .write_button_text(vec![TouchScreenSetTextContainer {
                                    slot: touch_msg.slot,
                                    daw_id: touch_msg.daw_id,
                                    row: touch_msg.row,
                                    column: touch_msg.column,
                                    layer: touch_msg.layer,
                                    text: touch_msg.text,
                                }])
                                .unwrap();
                        }
                        DownstreamMsg::TouchScreenTextBatch(touch_msgs) => {
                            let containers: Vec<TouchScreenSetTextContainer> = touch_msgs
                                .into_iter()
                                .map(|touch_msg| TouchScreenSetTextContainer {
                                    slot: touch_msg.slot,
                                    daw_id: touch_msg.daw_id,
                                    row: touch_msg.row,
                                    column: touch_msg.column,
                                    layer: touch_msg.layer,
                                    text: touch_msg.text,
                                })
                                .collect();
                            v1m.touchscreen.write_button_text(containers).unwrap();
                        }
                        _ => println!("Unhandled downstream message: {:?}", msg),
                    }
                }
            }
        });
    }
}

pub struct V1m {
    pub channel_faders: Vec<Fader>,
    pub master_fader: Fader,
    pub encoders: Vec<Encoder>,
    pub mutes: Vec<Button>,
    pub solos: Vec<Button>,
    pub arms: Vec<Button>,
    pub selects: Vec<Button>,
    pub channel_meters: ChannelMeters,
    pub master_meters: MasterMeters,
    pub top_scribbles: TopScribbleStrips,
    pub bottom_scribbles: BottomScribbleStrips,
    pub seven_segment_display: SevenSegmentDisplay,
    pub touchscreen: TouchScreen,
    input: Receiver<DownstreamMsg>,
    upstream: Sender<UpstreamMsg>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encoder_center_mode_center_const() {
        assert_eq!(map_to_encoder_ring(0.0), ENCODER_CENTER_MODE_CENTER);
    }
}
