use std::sync::{Arc, Mutex};
use std::thread;

use crossbeam_channel::{Receiver, Sender};
use helgoboss_midi::{Channel, RawShortMessage, ShortMessage};

use crate::midi::base::{
    NoteOff, NoteOffBuilder, NoteOn, NoteOnBuilder, PitchBend, PitchBendBuilder,
};
use crate::midi::{MidiDevice, MidiError};
use crate::track::track::Direction; // TODO: probably hoist this out of track
use crate::traits::{Bind, Set};

#[derive(Clone)]
pub struct FaderAbsMsg {
    pub idx: i32,
    pub direction: Direction,
    pub value: f64, // Probably too much precision?
}

#[derive(Clone, Copy)]
pub enum LEDState {
    Off,
    On,
    Flash,
}

#[derive(Clone)]
pub struct MutePress {
    pub idx: i32,
    velocity: u8,
}

#[derive(Clone)]
pub struct MuteRelease {
    pub idx: i32,
    velocity: u8,
}

#[derive(Clone)]
pub struct MuteLEDMsg {
    pub idx: i32,
    pub direction: Direction,
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

#[derive(Clone)]
pub struct SoloLEDMsg {
    pub idx: i32,
    pub direction: Direction,
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

#[derive(Clone)]
pub struct ArmLEDMsg {
    pub idx: i32,
    pub direction: Direction,
    pub state: LEDState,
}

pub enum XTouchDownstreamMsg {
    FaderAbs(FaderAbsMsg),
    MuteLED(MuteLEDMsg),
    SoloLED(SoloLEDMsg),
    ArmLED(ArmLEDMsg),
}

pub enum XTouchUpstreamMsg {
    FaderAbs(FaderAbsMsg),
    MutePress(MutePress),
    MuteRelease(MuteRelease),
    SoloPress(SoloPress),
    SoloRelease(SoloRelease),
    ArmPress(ArmPress),
    ArmRelease(ArmRelease),
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

pub struct Button {
    base: Arc<Mutex<MidiDevice>>,
    channel: Channel,
    midi_note: u8,
}

// YOLO, who needs bind/set traits anyway?
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
            faders.push(Fader {
                base: self.base.clone(),
                channel: Channel::new(i as u8), // TODO: This has some offset
            });
        }
        let mut mutes = Vec::with_capacity(self.num_channels);
        for i in 0..self.num_channels {
            // TODO: repeat this for the other button types
            let mut b = Button {
                base: self.base.clone(),
                channel: Channel::new(i as u8),
                midi_note: 0x10, // TODO: This has some offset
            };
            let upstream_press = upstream.clone();
            b.bind_press(move |velocity| {
                let _ = upstream_press
                    .clone()
                    .send(XTouchUpstreamMsg::MutePress(MutePress {
                        idx: i as i32,
                        velocity,
                    }));
            });
            let upstream_release = upstream.clone();
            b.bind_release(move |velocity| {
                let _ = upstream_release
                    .clone()
                    .send(XTouchUpstreamMsg::MuteRelease(MuteRelease {
                        idx: i as i32,
                        velocity,
                    }));
            });
            mutes.push(b);
        }
        let mut solos = Vec::with_capacity(self.num_channels);
        for i in 0..self.num_channels {
            solos.push(Button {
                base: self.base.clone(),
                channel: Channel::new(i as u8),
                midi_note: 0x11, // TODO: This has some offset
            });
        }
        let mut arms = Vec::with_capacity(self.num_channels);
        for i in 0..self.num_channels {
            arms.push(Button {
                base: self.base.clone(),
                channel: Channel::new(i as u8),
                midi_note: 0x12, // TODO: This has some offset
            });
        }

        let mut xtouch = XTouch {
            input,
            upstream,
            faders,
            mutes,
            solos,
            arms,
        };

        thread::spawn(move || {
            loop {
                if let Ok(msg) = xtouch.input.recv() {
                    match msg {
                        XTouchDownstreamMsg::FaderAbs(fader_msg) => {
                            xtouch.faders[fader_msg.idx as usize]
                                .set((fader_msg.value * 16383.0) as i32) // TODO: check this...
                                .unwrap();
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
                    }
                }
            }
        });
    }
}

pub struct XTouch {
    pub faders: Vec<Fader>,
    pub mutes: Vec<Button>,
    pub solos: Vec<Button>,
    pub arms: Vec<Button>,
    input: Receiver<XTouchDownstreamMsg>,
    upstream: Sender<XTouchUpstreamMsg>,
}
