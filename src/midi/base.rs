use helgoboss_midi::{
    RawShortMessage, ShortMessage, ShortMessageFactory, StructuredShortMessage, U7,
};
use midir::{MidiInput, MidiInputPort, MidiOutputConnection};

pub enum MidiError {
    SendError(midir::SendError),
    ConnectError(midir::ConnectError<midir::MidiInput>),
    InitError(midir::InitError),
    FromBytesError(helgoboss_midi::FromBytesError),
}

pub struct MidiDevice {
    name: String,
    midi_in_port: MidiInputPort,
    pub midi_out: MidiOutputConnection,
}

impl MidiDevice {
    pub fn new(name: &str, midi_in_port: MidiInputPort, midi_out: MidiOutputConnection) -> Self {
        MidiDevice {
            name: name.to_string(),
            midi_in_port,
            midi_out,
        }
    }

    pub fn run(&self) -> Result<(), MidiError> {
        let midi_in = MidiInput::new(&self.name).map_err(MidiError::InitError)?;
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
                            println!(
                                "Note On: Channel {}, Key {}, Velocity {}",
                                channel, key_number, velocity
                            );
                        }
                        StructuredShortMessage::PitchBendChange {
                            channel,
                            pitch_bend_value,
                        } => {
                            println!(
                                "Pitch Bend Change: Channel {}, Value {}",
                                channel, pitch_bend_value
                            );
                        }
                        _ => {
                            println!("Received unexpected message: {:?}", structured);
                        }
                    }
                },
                (),
            )
            .map_err(MidiError::ConnectError)?;
        Ok(())
    }
}
