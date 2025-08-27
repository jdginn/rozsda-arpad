use std::collections::{HashMap, VecDeque};

use rosc::{OscMessage, OscPacket};

// This is auto-generated based on the osc spec yaml
//
// Each of these enums corresponds to a field in the OSC message that encodes a GUID, meaning they
// do not encode the actual endpoints.
#[derive(Eq, Hash, PartialEq, Clone)]
enum PathParameterType {
    TrackGUID,
    TrackSendGUID,
}

// These should also be auto-generated from the spec. Each variant's parse() method can use a
// regex to extract the relevant parameters from the OSC address.
pub enum ParameterSequence {
    TrackGUID,
    TrackSendGUID,
}

type ParameterValueSequence = Vec<(PathParameterType, String)>;

impl ParameterSequence {
    // TODO
    fn parse(&self, osc_address: &str) -> Option<ParameterValueSequence> {
        None // TODO
    }
}

// Hopefully, these enums are the only thing that need to be generated from the spec, and the rest
// of this can be handwritten once.

struct FilterLayer {
    parameter_sequence: ParameterSequence,
    // the OSC address that "unlocks" this layer
    // E.g. for TrackGUID, this might be "/track/{track_guid}/index"
    key_route: String, // TODO: consider supporting multiple
    // We buffer messages if this is false. When it's true, we pass messages through.
    // At the moment we set it true, we also flush the buffer.
    initialized: HashMap<ParameterValueSequence, bool>,
    on_initialized: Option<Box<dyn Fn(ParameterValueSequence)>>, // called when a guid is initialized
    buffer: HashMap<ParameterValueSequence, VecDeque<OscMessage>>,
}

impl FilterLayer {
    pub fn initialize(&mut self, values: ParameterValueSequence) {
        if let Some(callback) = &self.on_initialized {
            callback(values.clone());
        }
        self.initialized.insert(values.clone(), true);
    }
}

struct MessageFilter {
    // Each layer represents some field in the OSC address we may need to filter on
    layers: Vec<FilterLayer>,
    dispatcher: Box<dyn Fn(OscMessage)>,
}

impl MessageFilter {
    /// dispatcher is just another dispatcher
    pub fn dispatch_osc<L>(&mut self, packet: OscPacket, log_unknown: L)
    where
        L: Fn(&str),
    {
        fn matches_key_route(osc_addr: &str, key_route: &str) -> bool {
            let osc_parts: Vec<&str> = osc_addr.split('/').filter(|s| !s.is_empty()).collect();
            let key_parts: Vec<&str> = key_route.split('/').filter(|s| !s.is_empty()).collect();

            if osc_parts.len() != key_parts.len() {
                return false;
            }

            for (osc, key) in osc_parts.iter().zip(key_parts.iter()) {
                if key.starts_with('{') && key.ends_with('}') {
                    // Wildcard segment, always matches
                    continue;
                }
                if osc != key {
                    return false;
                }
            }
            true
        }

        let msg = match &packet {
            OscPacket::Message(msg) => msg,
            _ => return,
        };

        // Pseudocode:
        // - Iterate on layers
        // - For each layer, check whether this message's address matches the regex for that layer
        // - If so, extract the guid from the address
        // - Check whether the guid is initialized for the layer. If so, call self.dispatcher(msg)
        // and return.
        // - If not, check whether the message is the "key" message for this filter <- TODO: still
        // need to define how this works.
        //      - If so, mark the guid as initialized
        //      - flush the buffer by calling self.dispatcher(msg) for each message
        //      - call self.dispatcher(msg) for this message
        //      - Possibly, also call an arbitrary callback that fires on initialization? <- TODO
        // - If this is not the key message, buffer it and return.
        self.layers.iter_mut().for_each(|layer| {
            if let Some(value_sequence) = layer.parameter_sequence.parse(&msg.addr) {
                match layer.initialized.get(&value_sequence) {
                    Some(true) => {
                        // already initialized, just dispatch
                        (self.dispatcher)(msg.to_owned());
                    }
                    Some(false) => {
                        // not initialized, buffer the message
                        let buffer = layer
                            .buffer
                            .entry(value_sequence.clone())
                            .or_insert_with(VecDeque::new);
                        buffer.push_back(msg.clone());
                    }
                    None => {
                        // Not seen before, check if this is the key message
                        if matches_key_route(&msg.addr, &layer.key_route) {
                            // This is the key message, initialize
                            layer.initialize(value_sequence.clone());
                            // Dispatch the buffered messages
                            if let Some(buffer) = layer.buffer.get_mut(&value_sequence) {
                                while let Some(buffered_msg) = buffer.pop_front() {
                                    (self.dispatcher)(buffered_msg);
                                }
                            }
                            // Dispatch this message
                            (self.dispatcher)(msg.clone());
                        } else {
                            // Not the key message, buffer it
                            let buffer = layer
                                .buffer
                                .entry(value_sequence.clone())
                                .or_insert_with(VecDeque::new);
                            buffer.push_back(msg.clone());
                        }
                    }
                }
            }
        });
    }
}
