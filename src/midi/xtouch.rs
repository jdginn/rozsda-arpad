use std::sync::{Arc, Mutex};
use std::thread;

use crossbeam_channel::{Receiver, Sender};
use derive_more::From;
use helgoboss_midi::{Channel, RawShortMessage, ShortMessage};

use crate::midi::base::{
    ControlChange, ControlChangeBuilder, NoteOff, NoteOffBuilder, NoteOn, NoteOnBuilder, PitchBend,
    PitchBendBuilder,
};
use crate::midi::encoder_led_mappings;
use crate::midi::{MidiDevice, MidiError};
use crate::modes::mode_manager::Barrier;
use crate::traits::{Bind, Set};

#[derive(Clone, Debug)]
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
pub enum EncoderRingLEDMsg {
    Blank(EncoderRingLEDBlankMsg),
    AllSegments(EncoderRingLEDAllSegmentsMsg),
    RangePoint(EncoderRingLEDRangePointMsg),
    RangeFill(EncoderRingLEDRangeFillMsg),
    Edges(EncoderRingLEDEdges),
}

#[derive(Clone, Copy, Debug)]
pub struct EncoderRingLEDBlankMsg {
    pub idx: i32,
}

#[derive(Clone, Copy, Debug)]
pub struct EncoderRingLEDAllSegmentsMsg {
    pub idx: i32,
}

#[derive(Clone, Copy, Debug)]
pub struct EncoderRingLEDRangePointMsg {
    pub idx: i32,
    pub pos: f32, // 0.0 to 1.0
}

#[derive(Clone, Copy, Debug)]
pub struct EncoderRingLEDRangeFillMsg {
    pub idx: i32,
    pub pos: f32, // 0.0 to 1.0
}

#[derive(Clone, Copy, Debug)]
pub struct EncoderRingLEDEdges {
    pub idx: i32,
}

#[derive(Clone, Copy, Debug)]
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

#[derive(Clone)]
pub struct MutePress {
    pub idx: i32,
}

#[derive(Clone)]
pub struct MuteRelease {
    pub idx: i32,
}

#[derive(Clone, Debug)]
pub struct MuteLEDMsg {
    pub idx: i32,
    pub state: LEDState,
}

#[derive(Clone)]
pub struct SoloPress {
    pub idx: i32,
}

#[derive(Clone)]
pub struct SoloRelease {
    pub idx: i32,
}

#[derive(Clone, Debug)]
pub struct SoloLEDMsg {
    pub idx: i32,
    pub state: LEDState,
}

#[derive(Clone)]
pub struct ArmPress {
    pub idx: i32,
}

#[derive(Clone)]
pub struct ArmRelease {
    pub idx: i32,
}

#[derive(Clone, Debug)]
pub struct ArmLEDMsg {
    pub idx: i32,
    pub state: LEDState,
}

#[derive(Clone)]
pub struct SelectPress {
    pub idx: i32,
}

#[derive(Clone)]
pub struct SelectRelease {
    pub idx: i32,
}

#[derive(Clone, Debug)]
pub struct SelectLEDMsg {
    pub idx: i32,
    pub state: LEDState,
}

#[derive(From)]
pub enum XTouchUpstreamMsg {
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

#[derive(Debug)]
pub enum XTouchDownstreamMsg {
    Barrier(Barrier),

    // Channel strip messages
    FaderAbs(FaderAbsMsg),
    EncoderRingLED(EncoderRingLEDMsg),
    MuteLED(MuteLEDMsg),
    SoloLED(SoloLEDMsg),
    ArmLED(ArmLEDMsg),
    SelectLED(SelectLEDMsg),

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

fn byte_slice(msg: RawShortMessage) -> [u8; 3] {
    let bytes = msg.to_bytes();
    [bytes.0, bytes.1.get(), bytes.2.get()]
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
    channel: Channel,
    knob_cc: u8,
    button_note: u8,
    led_cc_1: u8,
    led_cc_2: u8,
}

impl Encoder {
    fn bind_turn<F>(&mut self, mut callback: F)
    where
        F: FnMut(u8) + 'static + std::marker::Send,
    {
        ControlChangeBuilder {
            device: &mut self.base.lock().unwrap(),
            spec: ControlChange {
                channel: self.channel.get(),
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
                channel: self.channel.get(),
                key_number: self.button_note,
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
                channel: self.channel.get(),
                key_number: self.button_note,
            },
        }
        .bind(move |value| {
            callback(value);
        })
    }

    fn set(&mut self, val1: u8, val2: u8) -> Result<(), MidiError> {
        ControlChangeBuilder {
            device: &mut self.base.lock().unwrap(),
            spec: ControlChange {
                channel: self.channel.get(),
                controller_number: self.led_cc_1,
            },
        }
        .set(val1)?;
        ControlChangeBuilder {
            device: &mut self.base.lock().unwrap(),
            spec: ControlChange {
                channel: self.channel.get(),
                controller_number: self.led_cc_2,
            },
        }
        .set(val2)
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

pub struct XTouchBuilder {
    pub base: Arc<Mutex<MidiDevice>>,
    pub num_channels: usize,
}

impl XTouchBuilder {
    pub fn build(self, input: Receiver<XTouchDownstreamMsg>, upstream: Sender<XTouchUpstreamMsg>) {
        let mut faders = Vec::with_capacity(self.num_channels);
        for i in 0..self.num_channels {
            let mut f = Fader {
                base: self.base.clone(),
                channel: Channel::new(i as u8),
            };
            let upstream_fader = upstream.clone();
            f.bind(move |value| {
                let _ = upstream_fader.send(XTouchUpstreamMsg::from(FaderAbsMsg {
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
                channel: Channel::new(i as u8),
                knob_cc: 0x16 + i as u8,
                button_note: 0x32 + i as u8,
                led_cc_1: 0x48 + i as u8,
                led_cc_2: 0x56 + i as u8,
            };
            let upstream_turn = upstream.clone();
            e.bind_turn(move |value| match value {
                1 => upstream_turn
                    .send(XTouchUpstreamMsg::from(EncoderTurnCW { idx: i as i32 }))
                    .unwrap(),
                65 => upstream_turn
                    .send(XTouchUpstreamMsg::from(EncoderTurnCCW { idx: i as i32 }))
                    .unwrap(),
                _ => panic!("Unexpected encoder turn value: {}", value),
            });
            let upstream_press = upstream.clone();
            e.bind_press(move |_value| {
                upstream_press
                    .send(XTouchUpstreamMsg::from(EncoderPressMsg { idx: i as i32 }))
                    .unwrap();
            });
            let upstream_release = upstream.clone();
            e.bind_release(move |_value| {
                upstream_release
                    .send(XTouchUpstreamMsg::from(EncoderReleaseMsg { idx: i as i32 }))
                    .unwrap();
            });
            encoders.push(e);
        }
        let mut mutes = Vec::with_capacity(self.num_channels);
        for i in 0..self.num_channels {
            // TODO: repeat this for the other button types
            let mut b = Button {
                base: self.base.clone(),
                channel: Channel::new(i as u8),
                midi_note: 0x16 + i as u8,
            };
            let upstream_press = upstream.clone();
            b.bind_press(move |_velocity| {
                let _ = upstream_press.send(XTouchUpstreamMsg::from(MutePress { idx: i as i32 }));
            });
            let upstream_release = upstream.clone();
            b.bind_release(move |_velocity| {
                let _ =
                    upstream_release.send(XTouchUpstreamMsg::from(MuteRelease { idx: i as i32 }));
            });
            mutes.push(b);
        }
        let mut solos = Vec::with_capacity(self.num_channels);
        for i in 0..self.num_channels {
            let mut b = Button {
                base: self.base.clone(),
                channel: Channel::new(i as u8),
                midi_note: 0x08 + i as u8,
            };
            let upstream_press = upstream.clone();
            b.bind_press(move |_velocity| {
                let _ = upstream_press.send(XTouchUpstreamMsg::from(SoloPress { idx: i as i32 }));
            });
            let upstream_release = upstream.clone();
            b.bind_release(move |_velocity| {
                let _ =
                    upstream_release.send(XTouchUpstreamMsg::from(SoloRelease { idx: i as i32 }));
            });
            solos.push(b);
        }
        let mut arms = Vec::with_capacity(self.num_channels);
        for i in 0..self.num_channels {
            let mut b = Button {
                base: self.base.clone(),
                channel: Channel::new(i as u8),
                midi_note: i as u8,
            };
            let upstream_press = upstream.clone();
            b.bind_press(move |_velocity| {
                let _ = upstream_press.send(XTouchUpstreamMsg::from(ArmPress { idx: i as i32 }));
            });
            let upstream_release = upstream.clone();
            b.bind_release(move |_velocity| {
                let _ =
                    upstream_release.send(XTouchUpstreamMsg::from(ArmRelease { idx: i as i32 }));
            });
            arms.push(b);
        }
        let mut selects = Vec::with_capacity(self.num_channels);
        for i in 0..self.num_channels {
            let mut b = Button {
                base: self.base.clone(),
                channel: Channel::new(i as u8),
                midi_note: 0x24 + i as u8,
            };
            let upstream_press = upstream.clone();
            b.bind_press(move |_velocity| {
                let _ = upstream_press.send(XTouchUpstreamMsg::from(ArmPress { idx: i as i32 }));
            });
            let upstream_release = upstream.clone();
            b.bind_release(move |_velocity| {
                let _ =
                    upstream_release.send(XTouchUpstreamMsg::from(ArmRelease { idx: i as i32 }));
            });
            selects.push(b);
        }

        let mut xtouch = XTouch {
            input,
            upstream,
            faders,
            encoders,
            mutes,
            solos,
            arms,
            selects,
        };

        thread::spawn(move || {
            loop {
                if let Ok(msg) = xtouch.input.recv() {
                    match msg {
                        XTouchDownstreamMsg::Barrier(barrier_msg) => {
                            let _ = xtouch
                                .upstream
                                .send(XTouchUpstreamMsg::Barrier(barrier_msg));
                        }
                        XTouchDownstreamMsg::FaderAbs(fader_msg) => {
                            xtouch.faders[fader_msg.idx as usize]
                                .set((fader_msg.value * 16383.0) as i32) // TODO: check this...
                                .unwrap();
                        }
                        XTouchDownstreamMsg::EncoderRingLED(encoder_led_msg) => {
                            match encoder_led_msg {
                                EncoderRingLEDMsg::Blank(blank_msg) => {
                                    xtouch.encoders[blank_msg.idx as usize].set(0, 0).unwrap();
                                }
                                EncoderRingLEDMsg::AllSegments(all_msg) => {
                                    xtouch.encoders[all_msg.idx as usize].set(127, 127).unwrap();
                                }
                                EncoderRingLEDMsg::RangePoint(range_msg) => {
                                    let (val1, val2) =
                                        encoder_led_mappings::range_point(range_msg.pos);
                                    xtouch.encoders[range_msg.idx as usize]
                                        .set(val1, val2)
                                        .unwrap();
                                }
                                EncoderRingLEDMsg::RangeFill(fill_msg) => {
                                    let (val1, val2) =
                                        encoder_led_mappings::range_fill(fill_msg.pos);
                                    xtouch.encoders[fill_msg.idx as usize]
                                        .set(val1, val2)
                                        .unwrap();
                                }
                                EncoderRingLEDMsg::Edges(edges_msg) => {
                                    xtouch.encoders[edges_msg.idx as usize].set(1, 32).unwrap();
                                }
                            }
                        }
                        XTouchDownstreamMsg::MuteLED(mute_msg) => {
                            xtouch.mutes[mute_msg.idx as usize]
                                .set(mute_msg.state)
                                .unwrap();
                        }
                        XTouchDownstreamMsg::SoloLED(solo_msg) => {
                            xtouch.solos[solo_msg.idx as usize]
                                .set(solo_msg.state)
                                .unwrap();
                        }
                        XTouchDownstreamMsg::ArmLED(arm_msg) => {
                            xtouch.arms[arm_msg.idx as usize]
                                .set(arm_msg.state)
                                .unwrap();
                        }
                        XTouchDownstreamMsg::SelectLED(select_msg) => {
                            xtouch.selects[select_msg.idx as usize]
                                .set(select_msg.state)
                                .unwrap();
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
    input: Receiver<XTouchDownstreamMsg>,
    upstream: Sender<XTouchUpstreamMsg>,
}
