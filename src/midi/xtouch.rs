use std::sync::{Arc, Mutex};
use std::thread;

use crossbeam_channel::{Receiver, Sender};
use derive_more::From;
use helgoboss_midi::{Channel, RawShortMessage, ShortMessage};
use midir::{MidiInputPort, MidiOutputConnection};

use derive_enum_from::EnumFrom;

use crate::midi::base::{
    ControlChange, ControlChangeBuilder, NoteOff, NoteOffBuilder, NoteOn, NoteOnBuilder, PitchBend,
    PitchBendBuilder,
};
use crate::midi::{MidiDevice, MidiError};
use crate::modes::mode_manager::Barrier;
use crate::traits::{Bind, Set};

#[derive(Clone, Debug, Copy)]
pub struct FaderAbsMsg {
    pub idx: i32,
    pub value: f64, // Probably too much precision?
}

#[derive(Clone, Copy, Debug)]
pub struct EncoderTurnCW {
    pub idx: i32,
}

#[derive(Clone, Copy, Debug)]
pub struct EncoderTurnCCW {
    pub idx: i32,
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

#[derive(Clone, Copy, Debug)]
pub enum Color {
    Off,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    Grey,
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
pub struct ScribbleStripBackgroundColorMsg {
    pub idx: i32,
    pub color: Color,
}

#[derive(Clone, Copy, Debug, EnumFrom)]
pub enum UpstreamMsg {
    Barrier(Barrier),

    // Channel strip messages
    FaderAbs(FaderAbsMsg),
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
    FaderAbs(FaderAbsMsg),
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

    // Scribble strip messages
    ScribbleStripLine1Text(ScribbleStripLine1TextMsg),
    ScribbleStripLine2Text(ScribbleStripLine2TextMsg),
    ScribbleStripBackgroundColor(ScribbleStripBackgroundColorMsg),

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
        println!(
            "Setting fader on channel {} to value {}\n",
            self.channel.get(),
            value
        );
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

        println!(
            "Setting encoder LED ring for CC {} to mode {:?} with value {:b} (raw value {})\n",
            self.led_cc, mode, new_val, val
        );

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

pub struct ScribbleStrips {
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
        return vec![0; 7];
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
        return vec![0; 7];
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
    result.resize(7, 0);

    result
}

type ScribbleStripError = String;

impl ScribbleStrips {
    fn new(base: Arc<Mutex<MidiDevice>>, num_channels: usize) -> Self {
        Self {
            base,
            line_1: vec![String::new(); num_channels],
            line_2: vec![String::new(); num_channels],
            background_color: vec![Color::Blue; num_channels],
        }
    }

    fn write(&self) -> Result<(), ScribbleStripError> {
        // First, we send the text
        // Sysex message is made of a header + text formatted as ascii
        // Each scribble stip consumes 7 ascii bytes
        // Since we are updating everything, we can just send the full text for both lines of all 8
        // strips in one message.

        let mut msg_bytes = vec![0xf0, 0x00, 0x00, 0x66, 0x14, 0x12, 0x00];
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
            .map_err(|e| format!("Failed to send SysEx message: {}", e))?;

        fn color_to_byte(background: Color) -> u8 {
            let background_byte = match background {
                Color::Off => 0x00,
                Color::Red => 0x01,
                Color::Green => 0x02,
                Color::Yellow => 0x03,
                Color::Blue => 0x04,
                Color::Magenta => 0x05,
                Color::Cyan => 0x06,
                Color::Grey => 0x07,
            };
            // background_byte | (line1_mode_byte << 4) | (line2_mode_byte << 5)
            // background_byte | 0x50
            0x4b
        }

        // Now we do the same for the background color and line modes, which are sent in a separate message
        let mut msg_bytes = vec![0xf0, 0x00, 0x00, 0x66, 0x14, 0x72];
        // let mut msg_bytes = vec![0xf0, 0x00, 0x20, 0x32, 0x14, 0x72];
        for i in 0..self.line_1.len() {
            let color_byte = color_to_byte(self.background_color[i]);
            msg_bytes.push(color_byte);
        }
        msg_bytes.push(0xf7);

        println!(
            "\nSending SysEx message for scribble strip colors and modes: {:02x?}\n",
            msg_bytes
        );

        self.base
            .lock()
            .unwrap()
            .midi_out
            .send(&msg_bytes)
            .map_err(|e| format!("Failed to send SysEx message: {}", e))
    }
}

impl Set<ScribbleStripLine1TextMsg> for ScribbleStrips {
    type Error = ScribbleStripError;
    fn set(&mut self, value: ScribbleStripLine1TextMsg) -> Result<(), Self::Error> {
        self.line_1[value.idx as usize] = value.text;
        self.write()
    }
}

impl Set<ScribbleStripLine2TextMsg> for ScribbleStrips {
    type Error = ScribbleStripError;
    fn set(&mut self, value: ScribbleStripLine2TextMsg) -> Result<(), Self::Error> {
        self.line_2[value.idx as usize] = value.text;
        self.write()
    }
}

impl Set<ScribbleStripBackgroundColorMsg> for ScribbleStrips {
    type Error = ScribbleStripError;
    fn set(&mut self, value: ScribbleStripBackgroundColorMsg) -> Result<(), Self::Error> {
        self.background_color[value.idx as usize] = value.color;
        self.write()
    }
}

pub struct XTouchBuilder {
    pub base: Arc<Mutex<MidiDevice>>,
    pub num_channels: usize,
}

impl XTouchBuilder {
    pub fn new(
        midi_in_port: MidiInputPort,
        midi_out: MidiOutputConnection,
        num_channels: usize,
    ) -> Self {
        Self {
            base: Arc::new(Mutex::new(MidiDevice::new(
                "xtouch",
                midi_in_port,
                midi_out,
            ))),
            num_channels,
        }
    }

    pub fn build(self, input: Receiver<DownstreamMsg>, upstream: Sender<UpstreamMsg>) {
        let mut faders = Vec::with_capacity(self.num_channels);
        for i in 0..self.num_channels {
            let mut f = Fader {
                base: self.base.clone(),
                channel: Channel::new(i as u8),
            };
            let upstream_fader = upstream.clone();
            f.bind(move |value| {
                let _ = upstream_fader.send(UpstreamMsg::from(FaderAbsMsg {
                    idx: i as i32,
                    value: value as f64 / 16383.0, // TODO: check this...
                }));
            });
            faders.push(f);
        }
        let mut encoders = Vec::with_capacity(self.num_channels);
        for i in 0..self.num_channels {
            let mut e = Encoder {
                base: self.base.clone(),
                knob_cc: 0x10 + i as u8,
                click_note: 0x20 + i as u8,
                led_cc: 0x30 + i as u8,
            };
            let upstream_turn = upstream.clone();
            e.bind_turn(move |value| match value {
                1 => upstream_turn
                    .send(UpstreamMsg::from(EncoderTurnCW { idx: i as i32 }))
                    .unwrap(),
                65 => upstream_turn
                    .send(UpstreamMsg::from(EncoderTurnCCW { idx: i as i32 }))
                    .unwrap(),
                _ => panic!("Unexpected encoder turn value: {}", value),
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
                base: self.base.clone(),
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
                base: self.base.clone(),
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
                base: self.base.clone(),
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
                base: self.base.clone(),
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
        let scribbles = ScribbleStrips::new(self.base.clone(), self.num_channels);
        // Global view
        let mut b = Button {
            base: self.base.clone(),
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
            base: self.base.clone(),
            channel: Channel::new(0),
            midi_note: 62,
        };
        let upstream_press = upstream.clone();
        b.bind_press(move |velocity| match velocity {
            0 => upstream_press.send(UpstreamMsg::MIDITracksPress).unwrap(),
            127 => upstream_press.send(UpstreamMsg::MIDITracksRelease).unwrap(),
            _ => panic!("Unexpected MIDITracks button velocity: {}", velocity),
        });

        self.base.lock().unwrap().run();

        let mut xtouch = XTouch {
            input,
            upstream,
            faders,
            encoders,
            mutes,
            solos,
            arms,
            selects,
            scribbles,
        };

        thread::spawn(move || {
            loop {
                if let Ok(msg) = xtouch.input.recv() {
                    match msg {
                        DownstreamMsg::Barrier(barrier_msg) => {
                            let _ = xtouch.upstream.send(UpstreamMsg::Barrier(barrier_msg));
                        }
                        DownstreamMsg::FaderAbs(fader_msg) => {
                            println!(
                                "Setting fader {} to value {} (raw value {})\n",
                                fader_msg.idx,
                                fader_msg.value,
                                (fader_msg.value * 16383.0) as i32
                            );
                            xtouch.faders[fader_msg.idx as usize]
                                .set((fader_msg.value * 16383.0) as i32) // TODO: check this...
                                .unwrap();
                        }
                        DownstreamMsg::EncoderRingLED(encoder_led_msg) => {
                            xtouch.encoders[encoder_led_msg.idx as usize]
                                .set(encoder_led_msg.mode, encoder_led_msg.val)
                                .unwrap();
                        }
                        DownstreamMsg::MuteLED(mute_msg) => {
                            xtouch.mutes[mute_msg.idx as usize]
                                .set(mute_msg.state)
                                .unwrap();
                        }
                        DownstreamMsg::SoloLED(solo_msg) => {
                            xtouch.solos[solo_msg.idx as usize]
                                .set(solo_msg.state)
                                .unwrap();
                        }
                        DownstreamMsg::ArmLED(arm_msg) => {
                            xtouch.arms[arm_msg.idx as usize]
                                .set(arm_msg.state)
                                .unwrap();
                        }
                        DownstreamMsg::SelectLED(select_msg) => {
                            xtouch.selects[select_msg.idx as usize]
                                .set(select_msg.state)
                                .unwrap();
                        }
                        DownstreamMsg::ScribbleStripLine1Text(scribble_msg) => {
                            xtouch.scribbles.set(scribble_msg).unwrap();
                        }
                        DownstreamMsg::ScribbleStripLine2Text(scribble_msg) => {
                            xtouch.scribbles.set(scribble_msg).unwrap();
                        }
                        DownstreamMsg::ScribbleStripBackgroundColor(scribble_msg) => {
                            xtouch.scribbles.set(scribble_msg).unwrap();
                        }
                        _ => panic!("Message {:?} implemented yet!", msg),
                    }
                }
            }
        });
    }
}

pub struct XTouch {
    pub faders: Vec<Fader>,
    pub encoders: Vec<Encoder>,
    pub mutes: Vec<Button>,
    pub solos: Vec<Button>,
    pub arms: Vec<Button>,
    pub selects: Vec<Button>,
    pub scribbles: ScribbleStrips,
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
        assert_eq!(&res, b"hi\0\0\0\0\0");
        // assert_eq!(&res[..2], b"hi");
        // assert_eq!(&res[2..7], [0, 0, 0, 0, 0]);
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
        assert_eq!(&res_sep, &[0; 7]);

        let res_empty = compact_to_7_bytes("");
        assert_eq!(&res_empty, &[0; 7]);
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
