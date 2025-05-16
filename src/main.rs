use std::error::Error;
use std::io::{Write, stdin, stdout};
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::thread::spawn;
use std::time::Duration;

use midir::{Ignore, MidiInput, MidiOutput};

use crate::midi::{Mode, ModeManager};

pub mod midi;

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

    let mode = Arc::new(Mutex::new(0));
    let mut mode_manager = ModeManager::new(Arc::clone(&mode));
    mode_manager.bind_note(0, midi_msg::Channel::Ch1, {
        let mode = Arc::clone(&mode);
        move |note, velocity| {
            *mode.lock().unwrap() = 1;
            let _ =
                stdout().write_all(format!("MODE 0: Note {} ({})\n", note, velocity).as_bytes());
            Ok(())
        }
    });
    mode_manager.bind_note(1, midi_msg::Channel::Ch1, {
        let mode = Arc::clone(&mode);
        move |note, velocity| {
            *mode.lock().unwrap() = 0;
            let _ =
                stdout().write_all(format!("MODE 1: Note {} ({})\n", note, velocity).as_bytes());
            Ok(())
        }
    });

    // _conn_in needs to be a named parameter, because it needs to be kept alive until the end of the scope
    let _conn_in = midi_in.connect(
        in_port,
        "midir-read-input",
        move |_, message, _| {
            mode_manager.run(message).unwrap();
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
