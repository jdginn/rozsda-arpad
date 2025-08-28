use std::collections::{HashMap, VecDeque};
use std::fmt::Debug;

use rosc::{OscMessage, OscPacket};

/// Trait representing a specific OSC address context, such as a track or send instance.
/// Implementors should provide identity, cloning, and a way to extract parameter values.
pub trait ContextTrait: Debug + Eq + Clone + std::hash::Hash {}

/// Trait representing a shape for an OSC address context, such as a track or send.
/// Implementors should provide parsing, identity, and cloning.
pub trait ContextKindTrait<T: ContextTrait>: Debug + Eq + Clone + std::hash::Hash {
    /// Attempt to parse this context from the given OSC address.
    /// Returns Some(context instance) if matched, else None.
    fn parse(&self, osc_address: &str) -> Option<T>
    where
        Self: Sized;

    /// Returns a human-readable name for this context (for logging, debugging).
    fn context_name(&self) -> &'static str;
}

/// ContextGate manages all messages whose address is relevant to some particular OscContext, where
/// an OscContext defines some specific entity whose messages we either want to gate or propagate
/// depending on some initialization condition.
///
/// Each ContextGate handles any number of OscContexts with the same "shape", that is any number of
/// entities that live at the same layer of the hierarchy, encode the same set of identifiers into
/// their address, and depend on the same initialization criteria. Each concrete context is handled
/// individually.
struct ContextGate<T: ContextTrait, K: ContextKindTrait<T>> {
    // The shape of the OscContext that this layer is responsible for
    parameter_sequence: K,
    // the OSC address that "unlocks" this layer
    // E.g. for TrackGUID, this might be "/track/{track_guid}/index"
    key_route: String, // TODO: consider supporting multiple
    // We buffer messages if this is false. When it's true, we pass messages through.
    // At the moment we set it true, we also flush the buffer.
    initialized: HashMap<T, bool>,
    // Called when a specific context is initialized
    on_initialized: Option<Box<dyn Fn(T)>>,
    buffer: HashMap<T, VecDeque<OscMessage>>,
}

impl<T: ContextTrait, K: ContextKindTrait<T>> ContextGate<T, K> {
    /// Mark a specific concrete OscContext as initialized
    pub fn initialize(&mut self, values: T) {
        if let Some(callback) = &self.on_initialized {
            callback(values.clone());
        }
        self.initialized.insert(values.clone(), true);
    }
}

/// Returns true if the OSC address matches a pattern expressed as a pattern.
///
/// E.g. for "/track/{track_guid}/index", this will match "/track/1234567890/index" but not
/// "/track/1234567890/
fn matches_key_pattern(osc_addr: &str, key_route: &str) -> bool {
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

/// OscGatedRouter allows gating a set of OSC messages until certain conditions are met.
///
/// Specifically, our messages encode various IDs into the OSC address that tie a message to some
/// specific entity. In situatitions where we need to know certain information about that entity
/// before we can process messages pertaining to it, ContextGate will buffer all messages until we
/// have received a "key" message, which provides us the information we need to successfully
/// process the rest of the messages. This is important if we have no guarantee tha the "key"
/// message will arrive before the others.
///
/// Once the gate's initialization condition is met, all messages will be passed through.
pub struct OscGatedRouter<T: ContextTrait, K: ContextKindTrait<T>> {
    // Each layer represents some field in the OSC address we may need to filter on
    layers: Vec<ContextGate<T, K>>,
    dispatcher: Box<dyn Fn(OscMessage)>,
}

impl<T: ContextTrait, K: ContextKindTrait<T>> OscGatedRouter<T, K> {
    pub fn new(dispatcher: Box<dyn Fn(OscMessage)>) -> Self {
        Self {
            layers: vec![],
            dispatcher,
        }
    }

    /// dispatch_osc gates messages until their initialization condition is met and then passes
    /// messages through to self.dispatcher.
    pub fn dispatch_osc(&mut self, packet: OscPacket) {
        let msg = match &packet {
            OscPacket::Message(msg) => msg,
            _ => return,
        };

        self.layers.iter_mut().for_each(|layer| {
            if let Some(value_sequence) = layer.parameter_sequence.parse(&msg.addr) {
                // If this message is relevant to this layer...
                match layer.initialized.get(&value_sequence) {
                    Some(true) => {
                        // context is already initialized, just dispatch
                        (self.dispatcher)(msg.to_owned());
                    }
                    Some(false) | None => {
                        // Check if this is the key message
                        if matches_key_pattern(&msg.addr, &layer.key_route) {
                            // This is the key message, initialize
                            //
                            // TODO: if this IS the key message, the on_initialized callback probably
                            // needs to know its value. How do we allow that? Especially in the case
                            // that we have multiple key messages and we need all of their values?
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
                            // Not the key message; buffer it
                            let buffer = layer.buffer.entry(value_sequence.clone()).or_default();
                            buffer.push_back(msg.clone());
                        }
                    }
                }
            }
        });
    }
}
