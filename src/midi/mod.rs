use std::any::type_name;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use midi_msg::{ChannelVoiceMsg, MidiMsg};

// Helper for debugging unimplemented arms of a match statement
fn type_of<T>(_: &T) -> &'static str {
    type_name::<T>()
}

pub struct Mode {
    // channel[control][callback]
    cc_callbacks: HashMap<u8, HashMap<u8, Box<dyn Fn(u8) -> Result<(), String> + Send>>>,
    note_callbacks: HashMap<u8, Box<dyn Fn(u8, u8) -> Result<(), String> + Send>>,
}

impl Mode {
    pub fn new() -> Mode {
        Mode {
            cc_callbacks: HashMap::new(),
            note_callbacks: HashMap::new(),
        }
    }

    pub fn bind_note<F>(&mut self, channel: midi_msg::Channel, callback: F)
    where
        F: Fn(u8, u8) -> Result<(), String> + Send + 'static,
    {
        self.note_callbacks
            .insert(channel as u8, Box::new(callback));
    }

    fn call_note(&self, channel: midi_msg::Channel, note: u8, velocity: u8) -> Result<(), String> {
        match self.note_callbacks.get(&(channel as u8)) {
            Some(callback) => callback(note, velocity),
            None => Result::Err("Not implemented".to_string()),
        }
    }

    pub fn bind_cc(
        &mut self,
        channel: midi_msg::Channel,
        control: midi_msg::ControlNumber,
        callback: fn(u8) -> Result<(), String>,
    ) {
        match self.cc_callbacks.get_mut(&(channel as u8)) {
            Some(chan) => {
                chan.insert(control as u8, Box::new(callback));
            }
            None => {
                self.cc_callbacks.insert(control as u8, HashMap::new());
                self.bind_cc(channel, control, callback);
            }
        }
    }

    pub fn call_cc(
        &self,
        channel: midi_msg::Channel,
        control: u8,
        value: u8,
    ) -> Result<(), String> {
        match self.cc_callbacks.get(&(channel as u8)) {
            Some(chan) => match chan.get(&(control as u8)) {
                Some(callback) => callback(value),
                None => Result::Err("Not implemented".to_string()),
            },
            None => Result::Err("Not implemented".to_string()),
        }
    }
}

pub struct ModeManager {
    current_mode: Arc<Mutex<i32>>,
    modes: HashMap<i32, Mode>,
}

impl ModeManager {
    pub fn new(mode: Arc<Mutex<i32>>) -> ModeManager {
        let mut mm = ModeManager {
            current_mode: mode,
            modes: HashMap::new(),
        };
        mm.modes.insert(0, Mode::new());
        mm
    }

    fn bind_cc(
        &mut self,
        mode: i32,
        channel: midi_msg::Channel,
        control: midi_msg::ControlNumber,
        callback: fn(u8) -> Result<(), String>,
    ) {
        match self.modes.get_mut(&mode) {
            Some(mode) => {
                mode.bind_cc(channel, control, callback);
            }
            None => {
                self.modes.insert(mode, Mode::new());
                self.bind_cc(mode, channel, control, callback);
            }
        }
    }
    fn call_cc(&self, channel: midi_msg::Channel, control: u8, value: u8) -> Result<(), String> {
        let curr_mode = {
            let guard = self.current_mode.lock().unwrap();
            *guard
        };
        self.modes
            .get(&curr_mode)
            .unwrap()
            .call_cc(channel, control, value)
    }
    pub fn bind_note<F>(&mut self, mode: i32, channel: midi_msg::Channel, callback: F)
    where
        F: Fn(u8, u8) -> Result<(), String> + Send + 'static,
    {
        println!("Binding mode {mode}");
        println!("modes: {:?}", self.modes.keys());
        self.modes
            .entry(mode)
            .or_insert(Mode::new())
            .bind_note(channel, callback);
        println!("modes: {:?}", self.modes.keys());
    }
    pub fn call_note(
        &self,
        channel: midi_msg::Channel,
        note: u8,
        velocity: u8,
    ) -> Result<(), String> {
        let curr_mode = {
            let guard = self.current_mode.lock().unwrap();
            *guard
        };
        println!("curr_mode: {}", curr_mode);
        println!("modes: {:?}", self.modes.keys());
        self.modes
            .get(&curr_mode)
            .unwrap()
            .call_note(channel, note, velocity)
    }
    // fn bind_pitch_bend(&mut self, mode: u8, callback: fn()) {}
    // fn bind_aftertouch(&mut self, mode: u8, callback: fn()) {}

    pub fn run(&self, message: &[u8]) -> Result<(), String> {
        let (msg, _) = MidiMsg::from_midi(message).expect("Not an error");
        match msg {
            MidiMsg::ChannelVoice { channel, msg } => match msg {
                ChannelVoiceMsg::NoteOn { note, velocity } => {
                    self.call_note(channel, note, velocity)
                }
                // ChannelVoiceMsg::NoteOff { note, velocity } => {
                //     panic!("Note off not implemented");
                // }
                ChannelVoiceMsg::ControlChange { control: cc } => {
                    self.call_cc(channel, cc.control(), cc.value())
                }
                _ => {
                    panic!("Not implemented msg type {}", type_of(&msg))
                }
            },
            _ => {
                panic!("Not implemented msg type {}", type_of(&msg))
            }
        }
    }
}
