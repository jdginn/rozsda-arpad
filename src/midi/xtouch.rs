use std::sync::{Arc, Mutex};
use std::thread;

use crossbeam_channel::{Receiver, Sender};
use derive_more::From;
use helgoboss_midi::{Channel, RawShortMessage, ShortMessage};

use crate::midi::base::{
    NoteOff, NoteOffBuilder, NoteOn, NoteOnBuilder, PitchBend, PitchBendBuilder,
};
use crate::midi::{MidiDevice, MidiError};
use crate::modes::mode_manager::Barrier;
use crate::traits::{Bind, Set};

#[derive(Clone)]
pub struct FaderAbsMsg {
    pub idx: i32,
    pub value: f64, // Probably too much precision?
}

#[derive(Clone, Copy)]
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
    pub state: LEDState,
}

#[derive(From)]
pub enum XTouchUpstreamMsg {
    Barrier(Barrier),

    // Channel strip messages
    FaderAbs(FaderAbsMsg),
    MutePress(MutePress),
    MuteRelease(MuteRelease),
    SoloPress(SoloPress),
    SoloRelease(SoloRelease),
    ArmPress(ArmPress),
    ArmRelease(ArmRelease),

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

pub enum XTouchDownstreamMsg {
    Barrier(Barrier),

    // Channel strip messages
    FaderAbs(FaderAbsMsg),
    MuteLED(MuteLEDMsg),
    SoloLED(SoloLEDMsg),
    ArmLED(ArmLEDMsg),

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
                    .send(XTouchUpstreamMsg::from(MutePress {
                        idx: i as i32,
                        velocity,
                    }));
            });
            let upstream_release = upstream.clone();
            b.bind_release(move |velocity| {
                let _ = upstream_release
                    .clone()
                    .send(XTouchUpstreamMsg::from(MuteRelease {
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
                        _ => panic!("Not implemented yet!"),
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

#[cfg(test)]
mod tests {
    use super::*;

    // Tests for LEDState

    #[test]
    fn test_led_state_from_bool_false() {
        let state: LEDState = false.into();
        assert!(matches!(state, LEDState::Off));
    }

    #[test]
    fn test_led_state_from_bool_true() {
        let state: LEDState = true.into();
        assert!(matches!(state, LEDState::On));
    }

    #[test]
    fn test_led_state_copy() {
        let state = LEDState::Flash;
        let copied = state;
        assert!(matches!(copied, LEDState::Flash));
        // Verify original is still usable (copy trait)
        assert!(matches!(state, LEDState::Flash));
    }

    // Tests for message structures

    #[test]
    fn test_fader_abs_msg_creation() {
        let msg = FaderAbsMsg {
            idx: 3,
            value: 0.75,
        };
        assert_eq!(msg.idx, 3);
        assert_eq!(msg.value, 0.75);
    }

    #[test]
    fn test_fader_abs_msg_clone() {
        let msg = FaderAbsMsg {
            idx: 1,
            value: 0.5,
        };
        let cloned = msg.clone();
        assert_eq!(cloned.idx, 1);
        assert_eq!(cloned.value, 0.5);
    }

    #[test]
    fn test_mute_press_creation() {
        let msg = MutePress {
            idx: 2,
            velocity: 127,
        };
        assert_eq!(msg.idx, 2);
        assert_eq!(msg.velocity, 127);
    }

    #[test]
    fn test_mute_led_msg_creation() {
        let msg = MuteLEDMsg {
            idx: 5,
            state: LEDState::Flash,
        };
        assert_eq!(msg.idx, 5);
        assert!(matches!(msg.state, LEDState::Flash));
    }

    #[test]
    fn test_solo_press_creation() {
        let msg = SoloPress { idx: 4 };
        assert_eq!(msg.idx, 4);
    }

    #[test]
    fn test_solo_led_msg_creation() {
        let msg = SoloLEDMsg {
            idx: 7,
            state: LEDState::On,
        };
        assert_eq!(msg.idx, 7);
        assert!(matches!(msg.state, LEDState::On));
    }

    #[test]
    fn test_arm_press_creation() {
        let msg = ArmPress { idx: 6 };
        assert_eq!(msg.idx, 6);
    }

    #[test]
    fn test_arm_led_msg_creation() {
        let msg = ArmLEDMsg {
            idx: 1,
            state: LEDState::Off,
        };
        assert_eq!(msg.idx, 1);
        assert!(matches!(msg.state, LEDState::Off));
    }

    // Tests for message enums

    #[test]
    fn test_xtouch_upstream_msg_from_fader_abs() {
        let fader_msg = FaderAbsMsg {
            idx: 0,
            value: 0.5,
        };
        let msg: XTouchUpstreamMsg = fader_msg.into();
        assert!(matches!(msg, XTouchUpstreamMsg::FaderAbs(_)));
    }

    #[test]
    fn test_xtouch_downstream_msg_mute_led_variant() {
        let led_msg = MuteLEDMsg {
            idx: 2,
            state: LEDState::On,
        };
        let msg = XTouchDownstreamMsg::MuteLED(led_msg);
        assert!(matches!(msg, XTouchDownstreamMsg::MuteLED(_)));
    }

    #[test]
    fn test_xtouch_downstream_msg_fader_abs_variant() {
        let fader_msg = FaderAbsMsg {
            idx: 1,
            value: 0.8,
        };
        let msg = XTouchDownstreamMsg::FaderAbs(fader_msg);
        assert!(matches!(msg, XTouchDownstreamMsg::FaderAbs(_)));
    }

    // NOTE: Testing XTouch::start, Fader, and Button would require:
    // 1. Mock MIDI devices and connections
    // 2. Channel setup and thread synchronization
    // 3. Complex async message passing
    //
    // These components are better suited for integration tests.
    // For unit tests, we've focused on the data structures and simple
    // conversions that can be tested in isolation.
}
