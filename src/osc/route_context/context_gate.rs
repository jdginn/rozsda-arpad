use std::collections::{HashMap, VecDeque};
use std::fmt::Debug;
use std::time::{Duration, Instant};

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

// Builder for a single context gate layer
pub struct ContextGateBuilder<T: ContextTrait, K: ContextKindTrait<T>> {
    parameter_sequence: K,
    key_routes: Vec<String>,
    on_initialized: Option<Box<dyn Fn(T, &HashMap<String, OscMessage>)>>,
}

impl<T: ContextTrait, K: ContextKindTrait<T>> ContextGateBuilder<T, K> {
    pub fn new(context_kind: K) -> Self {
        Self {
            parameter_sequence: context_kind,
            key_routes: Vec::new(),
            on_initialized: None,
        }
    }

    pub fn add_key_route(mut self, key_route: impl Into<String>) -> Self {
        self.key_routes.push(key_route.into());
        self
    }

    pub fn add_key_routes(mut self, key_routes: Vec<impl Into<String>>) -> Self {
        for route in key_routes {
            self.key_routes.push(route.into());
        }
        self
    }

    pub fn with_initialization_callback<F>(mut self, callback: F) -> Self
    where
        F: Fn(T, &HashMap<String, OscMessage>) + 'static,
    {
        self.on_initialized = Some(Box::new(callback));
        self
    }

    fn build(self) -> ContextGate<T, K> {
        ContextGate {
            parameter_sequence: self.parameter_sequence,
            key_routes: self.key_routes,
            initialized: HashMap::new(),
            on_initialized: self.on_initialized,
            buffer: HashMap::new(),
            buffer_timestamps: HashMap::new(),
            key_messages: HashMap::new(),
        }
    }
}

trait ContextualDispatcher {
    fn dispatch_osc(
        &mut self,
        msg: OscMessage,
        dispatcher: &mut dyn FnMut(OscMessage),
    ) -> Option<()>;
    fn purge_stale_buffers(&mut self, timeout: Duration);

    #[cfg(test)]
    fn test_info(&self, ctx_str: &str) -> HashMap<String, usize>;
}

/// ContextGate manages all messages whose address is relevant to some particular OscContext, where
/// an OscContext defines some specific entity whose messages we either want to gate or propagate
/// depending on some initialization condition.
///
/// Each ContextGate handles any number of OscContexts with the same "shape", that is any number of
/// entities that live at the same layer of the hierarchy, encode the same set of identifiers into
/// their address, and depend on the same initialization criteria. Each concrete context is handled
/// individually.
struct ContextGate<T: ContextTrait + 'static, K: ContextKindTrait<T> + 'static> {
    // The shape of the OscContext that this layer is responsible for
    parameter_sequence: K,
    // the OSC address that "unlocks" this layer
    // E.g. for TrackGUID, this might be "/track/{track_guid}/index"
    key_routes: Vec<String>,
    // We buffer messages if this is false. When it's true, we pass messages through.
    // At the moment we set it true, we also flush the buffer.
    initialized: HashMap<T, bool>,
    // Called when a specific context is initialized
    on_initialized: Option<Box<dyn Fn(T, &HashMap<String, OscMessage>)>>,
    buffer: HashMap<T, VecDeque<OscMessage>>,
    buffer_timestamps: HashMap<T, Instant>,
    key_messages: HashMap<T, HashMap<String, OscMessage>>,
}

impl<T: ContextTrait, K: ContextKindTrait<T>> ContextGate<T, K> {
    /// Mark a specific concrete OscContext as initialized
    pub fn initialize(&mut self, context: T) {
        let key_messages = self.key_messages.get(&context).unwrap();

        if let Some(callback) = &self.on_initialized {
            callback(context.clone(), key_messages);
        }
        self.initialized.insert(context.clone(), true);
    }
}

impl<T: ContextTrait + 'static, K: ContextKindTrait<T> + 'static> ContextualDispatcher
    for ContextGate<T, K>
{
    fn purge_stale_buffers(&mut self, timeout: Duration) {
        let now = Instant::now();
        let stale_contexts: Vec<T> = self
            .buffer_timestamps
            .iter()
            .filter(|(ctx, timestamp)| {
                now.duration_since(**timestamp) > timeout && !self.initialized.contains_key(ctx)
            })
            .map(|(ctx, _)| ctx.clone())
            .collect();

        for ctx in stale_contexts {
            self.buffer.remove(&ctx);
            self.buffer_timestamps.remove(&ctx);
            self.key_messages.remove(&ctx);
        }
    }
    fn dispatch_osc(
        &mut self,
        msg: OscMessage,
        dispatcher: &mut dyn FnMut(OscMessage),
    ) -> Option<()> {
        match self.parameter_sequence.parse(&msg.addr) {
            None => None,
            Some(context) => {
                // Update timestamp for this context
                self.buffer_timestamps
                    .insert(context.clone(), Instant::now());

                // If this message is relevant to this layer...
                match self.initialized.get(&context) {
                    Some(true) => {
                        // context is already initialized, just dispatch
                        (dispatcher)(msg.to_owned());
                        Some(())
                    }
                    Some(false) | None => {
                        // Check if this is the key message
                        let mut is_key_message = false;
                        let mut matched_key_route = String::new();
                        for key_route in &self.key_routes {
                            if matches_key_pattern(&msg.addr, key_route) {
                                is_key_message = true;
                                matched_key_route = key_route.clone();
                                break;
                            }
                        }

                        if is_key_message {
                            // Store the key message
                            let key_msgs = self.key_messages.entry(context.clone()).or_default();
                            key_msgs.insert(matched_key_route.clone(), msg.to_owned());

                            // Check if we have all required key messages
                            let has_all_key_messages = self
                                .key_routes
                                .iter()
                                .all(|route| key_msgs.contains_key(route));

                            println!(
                                "ContextGate: Received key message for context {:?} on route {}. Has all key messages: {}, Key messages: {:?}",
                                context,
                                matched_key_route,
                                has_all_key_messages,
                                key_msgs.keys().collect::<Vec<_>>()
                            );
                            if has_all_key_messages {
                                // Initialize the context
                                self.initialize(context.clone());

                                // Process buffered messages
                                if let Some(buffer) = self.buffer.get_mut(&context) {
                                    while let Some(buffered_msg) = buffer.pop_front() {
                                        (dispatcher)(buffered_msg);
                                    }
                                }
                                // Dispatch this message
                                (dispatcher)(msg.clone());
                                Some(())
                            } else {
                                // Not all key messages received yet; buffer this one
                                let buffer = self.buffer.entry(context.clone()).or_default();
                                buffer.push_back(msg.clone());
                                Some(())
                            }
                        } else {
                            // Not the key message; buffer it
                            let buffer = self.buffer.entry(context.clone()).or_default();
                            buffer.push_back(msg.clone());
                            Some(())
                        }
                    }
                }
            }
        }
    }

    #[cfg(test)]
    fn test_info(&self, ctx_str: &str) -> HashMap<String, usize> {
        let mut info = HashMap::new();

        // Find the context with the matching string representation
        for (ctx, initialized) in &self.initialized {
            if format!("{:?}", ctx) == ctx_str {
                info.insert("initialized".to_string(), if *initialized { 1 } else { 0 });
                break;
            }
        }

        for (ctx, buffer) in &self.buffer {
            if format!("{:?}", ctx) == ctx_str {
                info.insert("buffered_count".to_string(), buffer.len());
                break;
            }
        }

        info
    }
}

// Main builder for the router
pub struct OscGatedRouterBuilder<T: ContextTrait + 'static, K: ContextKindTrait<T> + 'static> {
    layers: Vec<ContextGateBuilder<T, K>>,
    dispatcher: Option<Box<dyn FnMut(OscMessage)>>,
    buffer_timeout: Duration,
}

impl<T: ContextTrait, K: ContextKindTrait<T>> OscGatedRouterBuilder<T, K> {
    pub fn new() -> Self {
        Self {
            layers: Vec::new(),
            dispatcher: None,
            buffer_timeout: Duration::from_secs(60), // Default 1 minute timeout
        }
    }

    pub fn with_dispatcher<F>(mut self, dispatcher: F) -> Self
    where
        F: FnMut(OscMessage) + 'static,
    {
        self.dispatcher = Some(Box::new(dispatcher));
        self
    }

    pub fn with_buffer_timeout(mut self, timeout: Duration) -> Self {
        self.buffer_timeout = timeout;
        self
    }

    pub fn add_layer(mut self, layer: ContextGateBuilder<T, K>) -> Self {
        self.layers.push(layer);
        self
    }

    pub fn build(self) -> Result<OscGatedRouter, RouterBuildError> {
        let dispatcher = self
            .dispatcher
            .ok_or(RouterBuildError::NoDispatcherProvided)?;

        // Instead of collecting directly, create the vector and push each element
        let mut layers: Vec<Box<dyn ContextualDispatcher>> = Vec::with_capacity(self.layers.len());

        for layer_builder in self.layers {
            // Explicitly cast each built layer as a Box<dyn ContextualDispatcher>
            let layer: Box<dyn ContextualDispatcher> = Box::new(layer_builder.build());
            layers.push(layer);
        }

        Ok(OscGatedRouter {
            layers,
            dispatcher,
            buffer_timeout: self.buffer_timeout,
        })
    }
}

#[derive(Debug)]
pub enum RouterBuildError {
    NoDispatcherProvided,
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
pub struct OscGatedRouter {
    // Each layer represents some field in the OSC address we may need to filter on
    layers: Vec<Box<dyn ContextualDispatcher>>,
    dispatcher: Box<dyn FnMut(OscMessage)>,
    buffer_timeout: Duration,
}

impl OscGatedRouter {
    pub fn purge_stale_buffers(&mut self) {
        for layer in &mut self.layers {
            layer.purge_stale_buffers(self.buffer_timeout);
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
            if layer
                .dispatch_osc(msg.to_owned(), &mut *self.dispatcher)
                .is_some()
            {
                // Message was handled by this layer, stop processing
                return;
            };
        });
    }

    #[cfg(test)]
    pub fn test_context(&self, ctx: impl Debug) -> HashMap<String, usize> {
        let ctx_str = format!("{:?}", ctx);
        let mut merged_info = HashMap::new();

        for layer in &self.layers {
            let info = layer.test_info(&ctx_str);
            for (k, v) in info {
                merged_info.insert(k, v);
            }
        }

        merged_info
    }

    #[cfg(test)]
    pub fn is_context_initialized(&self, ctx: impl Debug) -> bool {
        self.test_context(ctx)
            .get("initialized")
            .copied()
            .unwrap_or(0)
            > 0
    }

    #[cfg(test)]
    pub fn get_buffered_messages_count(&self, ctx: impl Debug) -> usize {
        self.test_context(ctx)
            .get("buffered_count")
            .copied()
            .unwrap_or(0)
    }
}
