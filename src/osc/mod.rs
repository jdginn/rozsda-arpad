use std::collections::HashMap;
use std::net::{SocketAddrV4, UdpSocket};

use rosc::{OscMessage, OscPacket, OscType, encoder};

use crate::traits::{Bind, Query, Set};

mod generated_osc;

pub struct OscDevice {
    addr: SocketAddrV4,
    socket: UdpSocket,
    // ...
    track_volume_handlers: HashMap<String, Box<dyn FnMut(String, f32)>>,
    track_mute_handlers: HashMap<String, Box<dyn FnMut(String, bool)>>,
    track_name_handlers: HashMap<String, Box<dyn FnMut(String, String)>>,
}

impl Bind<i32> for OscDevice {
    fn bind<F>(&mut self, callback: F) {
        // This is a placeholder for the actual binding logic
        // In a real implementation, you would set up the socket to listen for incoming OSC messages
        // and call the callback with each received message.
        println!("Binding OSC device at {}", self.addr);
    }
}
