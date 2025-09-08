use std::sync::{Arc, Mutex};

use crate::midi::{MidiDevice, MidiError};
use crate::traits::{Bind, Query, Set};

use helgoboss_midi::{Channel, RawShortMessage, ShortMessage, ShortMessageFactory, U14};

fn byte_slice(msg: RawShortMessage) -> [u8; 3] {
    let bytes = msg.to_bytes();
    [bytes.0, bytes.1.get(), bytes.2.get()]
}

pub struct XTouch {
    // Placeholder for XTouch specific fields
}

pub struct Fader {
    base: Arc<Mutex<MidiDevice>>,
    channel: Channel,
}

impl Bind<i32> for Fader {
    fn bind<F>(&mut self, callback: F)
    where
        F: FnMut(i32) + 'static,
    {
        // Placeholder for binding logic
        println!("Binding Fader on channel {}", self.channel);
    }
}

impl Set<i32> for Fader {
    type Error = MidiError;
    fn set(&mut self, value: i32) -> Result<(), Self::Error> {
        let msg = RawShortMessage::pitch_bend_change(self.channel, U14::new(value as u16));
        let mut midi = self.base.lock().unwrap();
        midi.midi_out
            .send(byte_slice(msg).as_slice())
            .map_err(MidiError::Send)?;
        Ok(())
    }
}

pub struct Scribble {}
