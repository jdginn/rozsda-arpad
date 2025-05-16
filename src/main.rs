use std::any::type_name;
use std::collections::HashMap;
use std::error::Error;
use std::io::{Write, stdin, stdout};
use std::thread::sleep;
use std::thread::spawn;
use std::time::Duration;

use midi_msg::{ChannelVoiceMsg, MidiMsg};
use midir::{Ignore, MidiInput, MidiOutput};

// Helper for debugging unimplemented arms of a match statement
fn type_of<T>(_: &T) -> &'static str {
    type_name::<T>()
}

fn main() {
    let th = spawn(|| {
        sleep(Duration::new(1, 0));
        match make_events() {
            Ok(_) => (),
            Err(err) => println!("Error: {}", err),
        };
    });

    match listen() {
        Ok(_) => (),
        Err(err) => println!("Error: {}", err),
    }

    match th.join() {
        Ok(_) => (),
        Err(_) => panic!("Failed to join event thread"),
    };
}

fn make_events() -> Result<(), Box<dyn Error>> {
    let midi_out = match MidiOutput::new("midir output") {
        Ok(midi_out) => midi_out,
        Err(error) => panic!("Problem opening MIDI out port: {error:?}"),
    };

    let out_ports = midi_out.ports();
    let out_port = &out_ports[0];
    let mut conn = midi_out.connect(out_port, "testcase-midi-generator")?;
    {
        // Define a new scope in which the closure `play_note` borrows conn_out, so it can be called easily
        let mut play_note = |note: u8, duration: u64| {
            const NOTE_ON_MSG: u8 = 0x90;
            const NOTE_OFF_MSG: u8 = 0x80;
            const VELOCITY: u8 = 0x64;
            // We're ignoring errors in here
            let _ = conn.send(&[NOTE_ON_MSG, note, VELOCITY]);
            sleep(Duration::from_millis(duration * 150));
            // let _ = conn.send(&[NOTE_OFF_MSG, note, VELOCITY]);
        };

        sleep(Duration::from_millis(4 * 150));

        play_note(66, 4);
        play_note(65, 3);
        play_note(63, 1);
        play_note(61, 6);
        play_note(59, 2);
        play_note(58, 4);
        play_note(56, 4);
        play_note(54, 4);
    }

    return Ok(());
}

struct Mode {
    // channel[control][callback]
    cc_callbacks: HashMap<u8, HashMap<u8, Box<dyn Fn(u8) -> Result<(), String> + Send>>>,
    note_callbacks: HashMap<u8, Box<dyn Fn(u8, u8) -> Result<(), String> + Send>>,
}

impl Mode {
    fn new() -> Mode {
        Mode {
            cc_callbacks: HashMap::new(),
            note_callbacks: HashMap::new(),
        }
    }

    fn bind_note(
        &mut self,
        channel: midi_msg::Channel,
        callback: fn(note: u8, velocity: u8) -> Result<(), String>,
    ) {
        self.note_callbacks
            .insert(channel as u8, Box::new(callback));
    }

    fn call_note(&self, channel: midi_msg::Channel, note: u8, velocity: u8) -> Result<(), String> {
        match self.note_callbacks.get(&(channel as u8)) {
            Some(callback) => callback(note, velocity),
            None => Result::Err("Not implemented".to_string()),
        }
    }

    fn bind_cc(
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

    fn call_cc(&self, channel: midi_msg::Channel, control: u8, value: u8) -> Result<(), String> {
        match self.cc_callbacks.get(&(channel as u8)) {
            Some(chan) => match chan.get(&(control as u8)) {
                Some(callback) => callback(value),
                None => Result::Err("Not implemented".to_string()),
            },
            None => Result::Err("Not implemented".to_string()),
        }
    }
}

struct ModeManager {
    current_mode: u8,
    modes: HashMap<u8, Mode>,
}

impl ModeManager {
    fn new() -> ModeManager {
        let mut mm = ModeManager {
            current_mode: 0,
            modes: HashMap::new(),
        };
        mm.modes.insert(0, Mode::new());
        mm
    }

    fn bind_cc(
        &mut self,
        mode: u8,
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
        self.modes
            .get(&self.current_mode)
            .unwrap()
            .call_cc(channel, control, value)
    }
    fn bind_note(
        &mut self,
        mode: u8,
        channel: midi_msg::Channel,
        callback: fn(note: u8, velocity: u8) -> Result<(), String>,
    ) {
        self.modes
            .get_mut(&mode)
            .get_or_insert(&mut Mode::new())
            .bind_note(channel, callback);
    }
    fn call_note(&self, channel: midi_msg::Channel, note: u8, velocity: u8) -> Result<(), String> {
        self.modes
            .get(&self.current_mode)
            .unwrap()
            .call_note(channel, note, velocity)
    }
    // fn bind_pitch_bend(&mut self, mode: u8, callback: fn()) {}
    // fn bind_aftertouch(&mut self, mode: u8, callback: fn()) {}

    fn run(&self, message: &[u8]) -> Result<(), String> {
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

fn listen() -> Result<(), Box<dyn Error>> {
    let mut input = String::new();

    let mut midi_in = MidiInput::new("midir reading input")?;
    midi_in.ignore(Ignore::None);

    // Get an input port (read from console if multiple are available)
    let in_ports = midi_in.ports();
    let in_port = match in_ports.len() {
        0 => return Err("no input port found".into()),
        1 => {
            println!(
                "Choosing the only available input port: {}",
                midi_in.port_name(&in_ports[0]).unwrap()
            );
            &in_ports[0]
        }
        _ => {
            println!("\nAvailable input ports:");
            for (i, p) in in_ports.iter().enumerate() {
                println!("{}: {}", i, midi_in.port_name(p).unwrap());
            }
            print!("Please select input port: ");
            stdout().flush()?;
            let mut input = String::new();
            stdin().read_line(&mut input)?;
            in_ports
                .get(input.trim().parse::<usize>()?)
                .ok_or("invalid input port selected")?
        }
    };

    println!("\nOpening connection");
    let in_port_name = midi_in.port_name(in_port)?;

    let mut mode_manager = ModeManager::new();
    mode_manager.bind_note(0, midi_msg::Channel::Ch1, |note, velocity| {
        let _ = stdout().write_all(format!("Note {} ({})\n", note, velocity).as_bytes());
        Ok(())
    });

    // _conn_in needs to be a named parameter, because it needs to be kept alive until the end of the scope
    let _conn_in = midi_in.connect(
        in_port,
        "midir-read-input",
        move |_, message, _| {
            mode_manager.run(message);

            // println!("{}: {:?} (len = {})", stamp, message, message.len());
            // let (msg, len) = MidiMsg::from_midi(message).expect("Not an error");
            // match msg {
            //     MidiMsg::ChannelVoice { channel, msg } => match msg {
            //         ChannelVoiceMsg::NoteOn { note, velocity } => {
            //             panic!("Note on not implemented");
            //         }
            //         ChannelVoiceMsg::NoteOff { note, velocity } => {
            //             panic!("Note off not implemented");
            //         }
            //         ChannelVoiceMsg::ControlChange { control } => {
            //             panic!("Control change not implemented");
            //         }
            //         _ => {
            //             panic!("Not implemented msg type {}", type_of(&msg))
            //         }
            //     },
            //     _ => {
            //         panic!("Not implemented msg type {}", type_of(&msg))
            //     }
            // }
        },
        (),
    )?;

    println!(
        "Connection open, reading input from '{}' (press enter to exit) ...",
        in_port_name
    );

    input.clear();
    stdin().read_line(&mut input)?; // wait for next enter key press

    println!("Closing connection");
    Ok(())
}
