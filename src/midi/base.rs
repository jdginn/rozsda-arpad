use std::sync::{Arc, Mutex};

use helgoboss_midi::{
    Channel, ControllerNumber, RawShortMessage, ShortMessage, ShortMessageFactory,
    StructuredShortMessage, U7,
};
use midir::{MidiInput, MidiInputPort, MidiOutputConnection};

use crate::traits::{Bind, Set};

fn byte_slice(msg: RawShortMessage) -> [u8; 3] {
    let bytes = msg.to_bytes();
    [bytes.0, bytes.1.get(), bytes.2.get()]
}

#[derive(Debug)]
pub enum MidiError {
    Send(midir::SendError),
    Connect(midir::ConnectError<midir::MidiInput>),
    Init(midir::InitError),
    FromBytes(helgoboss_midi::FromBytesError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NoteOn {
    pub channel: u8,
    pub key_number: u8,
}

pub struct NoteOnBuilder<'a> {
    pub device: &'a mut MidiDevice,
    pub spec: NoteOn,
}

impl Bind<u8> for NoteOnBuilder<'_> {
    fn bind<F>(&mut self, _callback: F)
    where
        F: FnMut(u8) + Send + 'static,
    {
        self.device
            .note_on_callbacks
            .lock()
            .unwrap()
            .push((self.spec, Box::new(_callback)));
    }
}

impl Set<u8> for NoteOnBuilder<'_> {
    type Error = MidiError;

    fn set(&mut self, value: u8) -> Result<(), Self::Error> {
        let message: RawShortMessage = ShortMessageFactory::note_on(
            Channel::new(self.spec.channel),
            helgoboss_midi::KeyNumber::new(self.spec.key_number),
            U7::new(value),
        );
        self.device
            .midi_out
            .send(&byte_slice(message))
            .map_err(MidiError::Send)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NoteOff {
    pub channel: u8,
    pub key_number: u8,
}

pub struct NoteOffBuilder<'a> {
    pub device: &'a mut MidiDevice,
    pub spec: NoteOff,
}

impl Bind<u8> for NoteOffBuilder<'_> {
    fn bind<F>(&mut self, _callback: F)
    where
        F: FnMut(u8) + Send + 'static,
    {
        self.device
            .note_off_callbacks
            .lock()
            .unwrap()
            .push((self.spec, Box::new(_callback)));
    }
}

impl Set<u8> for NoteOffBuilder<'_> {
    type Error = MidiError;

    fn set(&mut self, value: u8) -> Result<(), Self::Error> {
        let message: RawShortMessage = ShortMessageFactory::note_off(
            Channel::new(self.spec.channel),
            helgoboss_midi::KeyNumber::new(self.spec.key_number),
            U7::new(value),
        );
        self.device
            .midi_out
            .send(&byte_slice(message))
            .map_err(MidiError::Send)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ControlChange {
    channel: u8,
    controller_number: u8,
}

pub struct ControlChangeBuilder<'a> {
    device: &'a mut MidiDevice,
    spec: ControlChange,
}

impl Bind<u8> for ControlChangeBuilder<'_> {
    fn bind<F>(&mut self, _callback: F)
    where
        F: FnMut(u8) + Send + 'static,
    {
        self.device
            .cc_callbacks
            .lock()
            .unwrap()
            .push((self.spec, Box::new(_callback)));
    }
}

impl Set<u8> for ControlChangeBuilder<'_> {
    type Error = MidiError;

    fn set(&mut self, value: u8) -> Result<(), Self::Error> {
        let message: RawShortMessage = ShortMessageFactory::control_change(
            Channel::new(self.spec.channel),
            ControllerNumber::new(self.spec.controller_number),
            U7::new(value),
        );
        self.device
            .midi_out
            .send(&byte_slice(message))
            .map_err(MidiError::Send)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PitchBend {
    pub channel: u8,
}

pub struct PitchBendBuilder<'a> {
    pub device: &'a mut MidiDevice,
    pub spec: PitchBend,
}

impl Bind<u16> for PitchBendBuilder<'_> {
    fn bind<F>(&mut self, _callback: F)
    where
        F: FnMut(u16) + Send + 'static,
    {
        self.device
            .pitch_bend_callbacks
            .lock()
            .unwrap()
            .push((self.spec, Box::new(_callback)));
    }
}

impl Set<u16> for PitchBendBuilder<'_> {
    type Error = MidiError;

    fn set(&mut self, value: u16) -> Result<(), Self::Error> {
        let message: RawShortMessage = ShortMessageFactory::pitch_bend_change(
            Channel::new(self.spec.channel),
            helgoboss_midi::U14::new(value),
        );
        self.device
            .midi_out
            .send(&byte_slice(message))
            .map_err(MidiError::Send)
    }
}

pub struct MidiDevice {
    name: String,
    midi_in_port: MidiInputPort,
    pub midi_out: MidiOutputConnection,

    note_on_callbacks: Arc<Mutex<Vec<(NoteOn, Box<dyn FnMut(u8) + Send>)>>>,
    note_off_callbacks: Arc<Mutex<Vec<(NoteOff, Box<dyn FnMut(u8) + Send>)>>>,
    cc_callbacks: Arc<Mutex<Vec<(ControlChange, Box<dyn FnMut(u8) + Send>)>>>,
    pitch_bend_callbacks: Arc<Mutex<Vec<(PitchBend, Box<dyn FnMut(u16) + Send>)>>>,
}

impl MidiDevice {
    pub fn new(name: &str, midi_in_port: MidiInputPort, midi_out: MidiOutputConnection) -> Self {
        MidiDevice {
            name: name.to_string(),
            midi_in_port,
            midi_out,
            note_on_callbacks: Arc::new(Mutex::new(Vec::new())),
            note_off_callbacks: Arc::new(Mutex::new(Vec::new())),
            cc_callbacks: Arc::new(Mutex::new(Vec::new())),
            pitch_bend_callbacks: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn run(&self) -> Result<(), MidiError> {
        let midi_in = MidiInput::new(&self.name).map_err(MidiError::Init)?;
        let cc_callbacks_clone = self.cc_callbacks.clone();
        let note_on_callbacks_clone = self.note_on_callbacks.clone();
        let note_off_callbacks_clone = self.note_off_callbacks.clone();
        let pitch_bend_callbacks_clone = self.pitch_bend_callbacks.clone();
        midi_in
            .connect(
                &self.midi_in_port,
                "MidiDevice",
                move |_, message, _| {
                    let structured = RawShortMessage::from_bytes((
                        message[0],
                        U7::new(message[1]),
                        U7::new(message[2]),
                    ))
                    .unwrap()
                    .to_structured();
                    match structured {
                        StructuredShortMessage::NoteOn {
                            channel,
                            key_number,
                            velocity,
                        } => {
                            let mut callbacks = note_on_callbacks_clone.lock().unwrap();
                            for (spec, callback) in callbacks.iter_mut() {
                                if Channel::new(spec.channel) == channel
                                    && u8::from(key_number) == spec.key_number
                                {
                                    callback(u8::from(velocity));
                                }
                            }
                        }
                        StructuredShortMessage::NoteOff {
                            channel,
                            key_number,
                            velocity,
                        } => {
                            let mut callbacks = note_off_callbacks_clone.lock().unwrap();
                            for (spec, callback) in callbacks.iter_mut() {
                                if Channel::new(spec.channel) == channel
                                    && u8::from(key_number) == spec.key_number
                                {
                                    callback(u8::from(velocity));
                                }
                            }
                        }
                        StructuredShortMessage::ControlChange {
                            channel,
                            controller_number,
                            control_value,
                        } => {
                            let mut callbacks = cc_callbacks_clone.lock().unwrap();
                            for (spec, callback) in callbacks.iter_mut() {
                                if Channel::new(spec.channel) == channel
                                    && ControllerNumber::new(spec.controller_number)
                                        == controller_number
                                {
                                    callback(u8::from(control_value));
                                }
                            }
                        }
                        StructuredShortMessage::PitchBendChange {
                            channel,
                            pitch_bend_value,
                        } => {
                            let mut callbacks = pitch_bend_callbacks_clone.lock().unwrap();
                            for (spec, callback) in callbacks.iter_mut() {
                                if Channel::new(spec.channel) == channel {
                                    callback(u16::from(pitch_bend_value));
                                }
                            }
                        }
                        _ => {
                            println!("Received unexpected message: {:?}", structured);
                        }
                    }
                },
                (),
            )
            .map_err(MidiError::Connect)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests for byte_slice helper function
    #[test]
    fn test_byte_slice_converts_raw_message() {
        let message = ShortMessageFactory::note_on(
            Channel::new(0),
            helgoboss_midi::KeyNumber::new(60),
            U7::new(100),
        );
        let bytes = byte_slice(message);
        assert_eq!(bytes.len(), 3);
        // First byte should be status byte (0x90 for Note On channel 0)
        assert_eq!(bytes[0], 0x90);
        // Second byte should be note number (60)
        assert_eq!(bytes[1], 60);
        // Third byte should be velocity (100)
        assert_eq!(bytes[2], 100);
    }

    // Tests for NoteOn structure
    #[test]
    fn test_note_on_creation() {
        let note_on = NoteOn {
            channel: 1,
            key_number: 60,
        };
        assert_eq!(note_on.channel, 1);
        assert_eq!(note_on.key_number, 60);
    }

    #[test]
    fn test_note_on_equality() {
        let note1 = NoteOn {
            channel: 1,
            key_number: 60,
        };
        let note2 = NoteOn {
            channel: 1,
            key_number: 60,
        };
        let note3 = NoteOn {
            channel: 2,
            key_number: 60,
        };
        assert_eq!(note1, note2);
        assert_ne!(note1, note3);
    }

    #[test]
    fn test_note_on_clone() {
        let note_on = NoteOn {
            channel: 1,
            key_number: 60,
        };
        let cloned = note_on.clone();
        assert_eq!(note_on, cloned);
    }

    // Tests for NoteOff structure
    #[test]
    fn test_note_off_creation() {
        let note_off = NoteOff {
            channel: 2,
            key_number: 72,
        };
        assert_eq!(note_off.channel, 2);
        assert_eq!(note_off.key_number, 72);
    }

    #[test]
    fn test_note_off_equality() {
        let note1 = NoteOff {
            channel: 2,
            key_number: 72,
        };
        let note2 = NoteOff {
            channel: 2,
            key_number: 72,
        };
        let note3 = NoteOff {
            channel: 2,
            key_number: 73,
        };
        assert_eq!(note1, note2);
        assert_ne!(note1, note3);
    }

    // Tests for ControlChange structure
    #[test]
    fn test_control_change_creation() {
        let cc = ControlChange {
            channel: 0,
            controller_number: 7,
        };
        assert_eq!(cc.channel, 0);
        assert_eq!(cc.controller_number, 7);
    }

    #[test]
    fn test_control_change_equality() {
        let cc1 = ControlChange {
            channel: 0,
            controller_number: 7,
        };
        let cc2 = ControlChange {
            channel: 0,
            controller_number: 7,
        };
        let cc3 = ControlChange {
            channel: 1,
            controller_number: 7,
        };
        assert_eq!(cc1, cc2);
        assert_ne!(cc1, cc3);
    }

    // Tests for PitchBend structure
    #[test]
    fn test_pitch_bend_creation() {
        let pb = PitchBend { channel: 3 };
        assert_eq!(pb.channel, 3);
    }

    #[test]
    fn test_pitch_bend_equality() {
        let pb1 = PitchBend { channel: 3 };
        let pb2 = PitchBend { channel: 3 };
        let pb3 = PitchBend { channel: 4 };
        assert_eq!(pb1, pb2);
        assert_ne!(pb1, pb3);
    }

    // Tests for MidiError
    #[test]
    fn test_midi_error_debug() {
        // Just verify that MidiError implements Debug
        let error = format!("{:?}", MidiError::Init(midir::InitError));
        assert!(error.contains("Init"));
    }

    // NOTE: Testing MidiDevice::new(), MidiDevice::run(), and the builder pattern
    // implementations (NoteOnBuilder, NoteOffBuilder, etc.) would require:
    // 1. Mock MIDI ports and connections
    // 2. Complex setup of MIDI input/output infrastructure
    // 3. Thread synchronization for callback testing
    //
    // These are better suited for integration tests with actual or mock MIDI devices.
    // For unit tests, we've focused on the data structures and simple helper functions
    // that can be tested in isolation.
}
