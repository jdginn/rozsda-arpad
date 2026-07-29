use std::sync::{Arc, Mutex};
use std::thread;

use crossbeam_channel::{Receiver, Sender};
use derive_more::From;
use helgoboss_midi::{Channel, RawShortMessage, ShortMessage};
use midir::{MidiInputPort, MidiOutputConnection};

use derive_enum_from::EnumFrom;

use crate::midi::base::{
    ChannelPressure, ChannelPressureBuilder, ControlChange, ControlChangeBuilder, NoteOff,
    NoteOffBuilder, NoteOn, NoteOnBuilder, PitchBend, PitchBendBuilder,
};
use crate::midi::{MidiDevice, MidiError};
use crate::modes::mode_manager::Barrier;
use crate::traits::{Bind, Set};

#[derive(Clone, Copy, Debug)]
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

#[derive(Clone, Copy, Debug)]
pub enum Slot {
    DAW1,
    DAW2,
    DAW3,
}

#[derive(Clone, Copy, Debug)]
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

#[derive(Clone, Copy, Debug)]
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

#[derive(Clone, Debug, Copy)]
pub struct ChannelFaderMsg {
    pub idx: i32,
    pub value: f64, // Probably too much precision?
}

#[derive(Clone, Debug, Copy)]
pub struct MasterFaderMsg {
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

#[derive(Clone, Copy, Debug)]
pub struct EncoderRingMsg {
    pub idx: i32,
    pub mode: EncoderRingMode,
    pub val: u8,
}

#[derive(Clone, Copy, Debug, EnumFrom)]
pub enum EncoderRingMode {
    Point,
    FromCenter,
    FromLeft,
    Width,
}

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

#[derive(Clone, Debug, Copy)]
pub struct MuteLEDMsg {
    pub idx: i32,
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

#[derive(Clone, Debug, Copy)]
pub struct SoloLEDMsg {
    pub idx: i32,
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

#[derive(Clone, Debug, Copy)]
pub struct ArmLEDMsg {
    pub idx: i32,
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

#[derive(Clone, Debug, Copy)]
pub struct SelectLEDMsg {
    pub idx: i32,
    pub state: LEDState,
}

#[derive(Clone, Debug, Copy)]
pub struct ChannelMeterMsg {
    pub idx: i32,
    pub db: f64,
}

#[derive(Clone, Debug, Copy)]
pub struct MasterMeterMsg {
    pub channel: StereoChannel,
    pub db: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Clone, Debug)]
pub struct ScribbleStripLine1TextMsg {
    pub idx: i32,
    pub text: String,
}

#[derive(Clone, Debug)]
pub struct ScribbleStripLine2TextMsg {
    pub idx: i32,
    pub text: String,
}

#[derive(Clone, Debug)]
pub struct BottomScribbleStripLine1TextMsg {
    pub idx: i32,
    pub text: String,
}

#[derive(Clone, Debug)]
pub struct BottomScribbleStripLine2TextMsg {
    pub idx: i32,
    pub text: String,
}

#[derive(Clone, Debug)]
pub struct ScribbleStripBackgroundColorMsg {
    pub idx: i32,
    pub color: Color,
}

#[derive(Clone, Debug)]
pub struct SevenSegmentDisplayMsg {
    pub text: String,
}

#[derive(Clone, Debug)]
pub struct TouchScreenSetTextMsg {
    pub slot: Slot,
    pub daw_id: DawId,
    pub row: usize,
    pub column: usize,
    pub layer: TouchScreenLayer,
    pub text: String,
}

#[derive(Clone, Debug)]
pub struct TouchScreenSetButtonBehaviorMsg {
    pub slot: Slot,
    pub daw_id: DawId,
    pub row: usize,
    pub column: usize,
    pub layer: TouchScreenLayer,
    pub midi_channel: u8,
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
}

#[derive(Debug, EnumFrom)]
pub enum DownstreamMsg {
    #[enum_from]
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
    ScribbleStripLine1Text(ScribbleStripLine1TextMsg),
    #[enum_from]
    ScribbleStripLine2Text(ScribbleStripLine2TextMsg),
    #[enum_from]
    BottomScribbleStripLine1Text(BottomScribbleStripLine1TextMsg),
    #[enum_from]
    BottomScribbleStripLine2Text(BottomScribbleStripLine2TextMsg),
    #[enum_from]
    ScribbleStripBackgroundColor(ScribbleStripBackgroundColorMsg),

    #[enum_from]
    SevenSegmentDisplay(SevenSegmentDisplayMsg),

    #[enum_from]
    TouchScreenSetText(TouchScreenSetTextMsg),
    #[enum_from]
    TouchScreenBatchSetText(Vec<TouchScreenSetTextMsg>),
    #[enum_from]
    TouchScreenSetButtonBehavior(TouchScreenSetButtonBehaviorMsg),
    #[enum_from]
    TouchScreenBatchSetButtonBehavior(Vec<TouchScreenSetButtonBehaviorMsg>),

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

/// Compacts a `&str` into exactly 7 ASCII bytes using camelCase conventions,
/// vowel stripping, double-consonant compression, and null-padding.
pub fn compact_to_7_bytes(input: &str) -> Vec<u8> {
    const VOWELS: &[u8] = b"aeiouAEIOU";
    const SEPARATORS: &[u8] = b" _.,-";
    const NUMBERS: &[u8] = b"0123456789";

    // 1. Filter to ASCII alphanumerics + recognized separators
    let cleaned: Vec<u8> = input
        .bytes()
        .filter(|b| b.is_ascii_alphanumeric() || SEPARATORS.contains(b))
        .collect();

    if cleaned.is_empty() {
        return vec![32; 7];
    }

    // 2. Split into words by separators
    // We also treat EACH number as its own word. This keeps us from truncating numbers (we assume
    // that numbers are always important).
    let words: Vec<&[u8]> = cleaned
        .split(|b| SEPARATORS.contains(b))
        .filter(|w| !w.is_empty())
        .flat_map(|w| w.chunk_by(|a, b| a.is_ascii_digit() == b.is_ascii_digit()))
        .flat_map(|w| -> Vec<&[u8]> {
            if w[0].is_ascii_digit() {
                w.chunks(1).collect()
            } else {
                vec![w]
            }
        })
        .filter(|w| !w.is_empty())
        .collect();

    if words.is_empty() {
        let mut result: Vec<u8> = input.bytes().collect();
        result.truncate(7);
        result.resize(7, 32);
        return result;
    }

    // 3. Process each word
    let mut processed: Vec<Vec<u8>> = Vec::with_capacity(words.len());
    for (i, word) in words.iter().enumerate() {
        let mut w = word.to_vec();

        // --- Casing Rules ---
        if i == 0 {
            // First word: preserve original first char case, lowercase the rest
            if !w.is_empty() {
                for j in 1..w.len() {
                    w[j] = w[j].to_ascii_lowercase();
                }
            }
        } else {
            // Subsequent words: camelCase
            // Lowercase all-caps words before casing (Rule 5)
            if w.iter()
                .all(|b| b.is_ascii_alphabetic() && b.is_ascii_uppercase())
            {
                for b in w.iter_mut() {
                    *b = b.to_ascii_lowercase();
                }
            }

            if !w.is_empty() {
                w[0] = w[0].to_ascii_uppercase();
                for j in 1..w.len() {
                    w[j] = w[j].to_ascii_lowercase();
                }
            }
        }

        processed.push(w);
    }

    const WORD_REPLACEMENTS: &[(&[u8], &[u8])] = &[
        (b"one", b"1"),
        (b"two", b"2"),
        (b"three", b"3"),
        (b"third", b"3rd"),
        (b"four", b"4"),
        (b"five", b"5"),
        (b"six", b"6"),
        (b"seven", b"7"),
        (b"eighth", b"8th"),
        (b"eight", b"8"),
        (b"nine", b"9"),
        (b"zero", b"0"),
        (b"left", b"L"),
        (b"right", b"R"),
        (b"center", b"C"),
    ];

    fn apply_word_replacement(s: &[u8]) -> (Vec<u8>, bool) {
        let lower: Vec<u8> = s.iter().map(|b| b.to_ascii_lowercase()).collect();
        for (from, to) in WORD_REPLACEMENTS {
            if lower == *from {
                return (to.to_vec(), true);
            }
        }
        (s.to_vec(), false)
    }

    const SUBSTR_REPLACEMENTS: &[(&[u8], &[u8])] = &[
        (b"two", b"2"),
        (b"three", b"3"),
        (b"third", b"3rd"),
        (b"four", b"4"),
        (b"five", b"5"),
        (b"six", b"6"),
        (b"seven", b"7"),
        (b"eighth", b"8th"),
        (b"eight", b"8"),
        (b"nine", b"9"),
        (b"zero", b"0"),
        (b"harmony", b"harm"),
        (b"background", b"bg"),
        (b"backup", b"bk"),
        (b"overhead", b"OH"),
        (b"reverse", b"rev"),
        (b"reverb", b"rvb"),
        (b"channel", b"chan"),
        (b"duplicate", b"dup"),
        (b"lead", b"ld"),
        (b"clean", b"cln"),
        (b"kick", b"kik"),
        (b"floor", b"flr"),
        (b"hats", b"hh"),
        (b"snare", b"snr"),
        (b"click", b"clk"),
    ];

    fn apply_substr_replacement(s: &[u8]) -> (Vec<u8>, bool) {
        let mut result = s.to_vec();
        let mut found = false;
        for (from, to) in SUBSTR_REPLACEMENTS {
            let lower: Vec<u8> = result.iter().map(|b| b.to_ascii_lowercase()).collect();
            if let Some(pos) = lower.windows(from.len()).position(|w| w == *from) {
                let mut replacement = to.to_vec();
                if result[pos].is_ascii_uppercase() {
                    replacement[0] = replacement[0].to_ascii_uppercase();
                }
                result.splice(pos..pos + from.len(), replacement);
                found = true;
            }
        }
        (result, found)
    }

    // Truncate words down to 7 bytes
    //
    // Each step shortens the string by one byte until we fit within 7 bytes
    //
    // For each step, find the longest word and truncate according to a set of rules.
    'outer: while processed.concat().len() > 7 {
        // Find the longest word
        let (i, w) = processed
            .iter_mut()
            .enumerate()
            .max_by_key(|(_, w)| w.len())
            .unwrap();

        // Some predefined replacements for common words/phrases
        let (compressed, did_replace) = apply_word_replacement(w);
        if did_replace {
            *w = compressed;
            continue;
        }
        let (compressed, did_replace) = apply_substr_replacement(w);
        if did_replace {
            *w = compressed;
            continue;
        }

        let mut did_truncate = false;

        // Remove double consonants first
        let mut compressed = Vec::with_capacity(w.len());
        let mut prev_consonant: Option<u8> = None;
        for b in w.iter() {
            let is_consonant = b.is_ascii_alphabetic() && !VOWELS.contains(b);
            if is_consonant && prev_consonant == Some(b.to_ascii_lowercase()) {
                did_truncate = true;
                continue; // Skip duplicate consonant
            }
            prev_consonant = Some(b.to_ascii_lowercase());
            compressed.push(*b);
        }
        *w = compressed;
        if did_truncate {
            continue;
        }

        // Remove double vowels
        let mut compressed = Vec::with_capacity(w.len());
        let mut prev_consonant: Option<u8> = None;
        for b in w.iter() {
            let is_consonant = b.is_ascii_alphabetic() && VOWELS.contains(b);
            if is_consonant && prev_consonant == Some(b.to_ascii_lowercase()) {
                did_truncate = true;
                continue; // Skip duplicate consonant
            }
            prev_consonant = Some(b.to_ascii_lowercase());
            compressed.push(*b);
        }
        *w = compressed;
        if did_truncate {
            continue;
        }

        // Remove the last single vowel UNLESS it's the first character of the word
        for (i, b) in w.iter().enumerate().rev() {
            if VOWELS.contains(b) && i != 0 {
                w.remove(i);
                continue 'outer;
            }
        }

        // As a last resort, truncate the last character of the longest word
        if !w.is_empty() {
            w.pop();
        }

        processed[i] = w.clone();
    }

    if processed.concat().len() > 7 {
        panic!("Processed string is still longer than 7 bytes after all compression steps");
    }

    // Safety truncate & null-padding to exactly 7 bytes
    let mut result = processed.concat();
    result.truncate(7);
    result.resize(7, 32);

    result
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
            msg_bytes.push(self.background_color[i].r.clamp(0, 0x7f));
            msg_bytes.push(self.background_color[i].g.clamp(0, 0x7f));
            msg_bytes.push(self.background_color[i].b.clamp(0, 0x7f));
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

impl Set<ScribbleStripLine1TextMsg> for TopScribbleStrips {
    type Error = ScribbleStripError;
    fn set(&mut self, value: ScribbleStripLine1TextMsg) -> Result<(), Self::Error> {
        self.line_1[value.idx as usize] = value.text;
        self.write()
    }
}

impl Set<ScribbleStripLine2TextMsg> for TopScribbleStrips {
    type Error = ScribbleStripError;
    fn set(&mut self, value: ScribbleStripLine2TextMsg) -> Result<(), Self::Error> {
        self.line_2[value.idx as usize] = value.text;
        self.write()
    }
}

impl Set<ScribbleStripBackgroundColorMsg> for TopScribbleStrips {
    type Error = ScribbleStripError;
    fn set(&mut self, value: ScribbleStripBackgroundColorMsg) -> Result<(), Self::Error> {
        self.background_color[value.idx as usize] = value.color;
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
                        DownstreamMsg::ScribbleStripLine1Text(scribble_msg) => {
                            v1m.top_scribbles.set(scribble_msg).unwrap();
                        }
                        DownstreamMsg::ScribbleStripLine2Text(scribble_msg) => {
                            v1m.top_scribbles.set(scribble_msg).unwrap();
                        }
                        DownstreamMsg::BottomScribbleStripLine1Text(scribble_msg) => {
                            v1m.bottom_scribbles.set(scribble_msg).unwrap();
                        }
                        DownstreamMsg::BottomScribbleStripLine2Text(scribble_msg) => {
                            v1m.bottom_scribbles.set(scribble_msg).unwrap();
                        }
                        DownstreamMsg::ScribbleStripBackgroundColor(scribble_msg) => {
                            v1m.top_scribbles.set(scribble_msg).unwrap();
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
                        DownstreamMsg::TouchScreenBatchSetText(touch_msgs) => {
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
                        _ => panic!("Message {:?} not implemented yet!", msg),
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
macro_rules! assert_eq_str {
    ($left:expr, $right:expr) => {
        assert_eq!(
            $left,
            $right,
            "\n  left:  {}\n  right: {}",
            String::from_utf8_lossy($left),
            String::from_utf8_lossy($right)
        );
    };
}

mod tests {
    use super::*;

    #[test]
    fn test_exact_7_byte_length() {
        // Rule: Output must always be exactly 7 bytes
        for input in &["hi", "hello", "longphrasethatshouldbeshort", "___"] {
            let res = compact_to_7_bytes(input);
            assert_eq!(
                res.len(),
                7,
                "Output must be exactly 7 bytes for: {}",
                input
            );
        }
    }

    #[test]
    fn test_null_padding_for_short_strings() {
        let res = compact_to_7_bytes("hi");
        // assert_eq!(&res, b"hi\0\0\0\0\0");
        assert_eq!(&res[..2], b"hi");
        assert_eq!(&res[2..7], [32, 32, 32, 32, 32]);
    }

    #[test]
    fn test_first_char_case_preservation() {
        // Rule: First character should match original capitalization
        assert_eq!(compact_to_7_bytes("hello world")[0], b'h');
        assert_eq!(compact_to_7_bytes("Hello world")[0], b'H');
        assert_eq!(compact_to_7_bytes("HELLO world")[0], b'H');
    }

    #[test]
    fn test_separators_trigger_camelcase() {
        // Rule: Spaces, underscores, periods, commas, hyphens replace with forcedCamelCase
        let res = compact_to_7_bytes("hello_world.test,word");
        let s = String::from_utf8_lossy(&res);
        assert!(
            s.starts_with("hl"),
            "First word should be lowercased (except first char preserved)"
        );
        // CamelCase boundaries should be uppercase
        assert!(
            !s.contains("_") && !s.contains(".") && !s.contains(",") && !s.contains("-"),
            "Separators should be removed and replaced by camelCase boundaries"
        );
    }

    #[test]
    fn test_all_caps_word_lowercasing_rule5() {
        // Rule 5: All-caps words next to a space get lowercased (unless first word needs uppercase)
        let res = compact_to_7_bytes("hello XML world");
        let s = String::from_utf8_lossy(&res);
        assert!(
            !s.contains("XML"),
            "All-caps word 'XML' should be lowercased to 'xml' -> 'Xml'"
        );
        // Verify camelCase transition happens
        assert!(
            s.contains("X"),
            "First letter of non-first words should be uppercase for camelCase"
        );
    }

    #[test]
    fn test_double_consonant_compression() {
        // Heuristic: `battle` -> `battl` -> `batl`
        let res = compact_to_7_bytes("battle fi");
        let s = String::from_utf8_lossy(&res);
        println!("Want: battle field -> batlField, got: {}", s);
        assert!(
            s.contains("batl"),
            "Double 't' in 'battle' should be compressed to single 't'"
        );
    }

    #[test]
    fn test_vowel_removal_priority() {
        // Rule 3 & 4: Remove vowels first (end-first), then truncate consonants if still >7
        let res = compact_to_7_bytes("beautiful reason");
        let s = String::from_utf8_lossy(&res);
        // Should be <= 7 chars (excluding nulls)
        let non_null_len = res.iter().take_while(|b| **b != 0).count();
        assert_eq!(
            non_null_len, 7,
            "After vowel removal, string should fit in 7 chars"
        );
        // Vowels should be stripped from the end first
        assert!(
            !s.ends_with(|c: char| "aeiouAEIOU".contains(c)),
            "End vowels should be removed first"
        );
    }

    #[test]
    fn test_numbers_preserved() {
        // Heuristic: Keep numbers
        let res = compact_to_7_bytes("user123 login");
        let s = String::from_utf8_lossy(&res);
        println!("Want to preserve numbers, got: {}", s);
        assert!(
            s.contains("123"),
            "Numbers should be preserved during compression"
        );
    }

    #[test]
    fn test_non_ascii_stripping() {
        // Heuristic: Skip non-ASCII
        let res = compact_to_7_bytes("naïve résumé");
        let s = String::from_utf8_lossy(&res);
        assert!(
            !s.contains("ï") && !s.contains("é"),
            "Non-ASCII characters should be stripped"
        );
    }

    #[test]
    fn test_all_separator_and_empty_inputs() {
        // Rule 11: All-separator input leaves it exactly as-is (padded with zeros)
        let res_sep = compact_to_7_bytes("  _ . ,  ");
        assert_eq!(&res_sep, "  _ . ,".as_bytes());

        let res_empty = compact_to_7_bytes("");
        assert_eq!(&res_empty, &[32; 7]);
    }

    #[test]
    fn test_deterministic_output() {
        // Rule 13: Same input always produces same output
        let input = "fetchXML_data_v2";
        let r1 = compact_to_7_bytes(input);
        let r2 = compact_to_7_bytes(input);
        assert_eq!(r1, r2, "Output must be fully deterministic");
    }

    #[test]
    fn test_exact_byte_array_for_known_case() {
        // Regression test for predictable output
        assert_eq_str!(&compact_to_7_bytes("hello world"), b"heloWrl");
        assert_eq_str!(&compact_to_7_bytes("drum bus 1"), b"drmBus1");
        assert_eq_str!(&compact_to_7_bytes("Lead Vox"), b"LeadVox");
        assert_eq_str!(&compact_to_7_bytes("funky guitar"), b"fnkyGtr");
        assert_eq_str!(&compact_to_7_bytes("rack tom 1"), b"rckTom1");
        assert_eq_str!(&compact_to_7_bytes("Stereo Chug"), b"SterChg");
        assert_eq_str!(&compact_to_7_bytes("Stacked Harmonies 2"), b"StcHrm2");
        assert_eq_str!(&compact_to_7_bytes("Hall reverb throw"), b"HalRvTh");
        assert_eq_str!(&compact_to_7_bytes("Hall reverb throw 2"), b"HlRvTh2");
        assert_eq_str!(&compact_to_7_bytes("Dotted eighth L"), b"Dtd8thL");
    }
}
