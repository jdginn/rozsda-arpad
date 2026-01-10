use std::collections::{HashMap, VecDeque};
use std::fmt::Debug;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::marker::PhantomData;
use std::time::{Duration, Instant};

use rosc::{OscMessage, OscPacket};

fn hash_to_u64<T: std::hash::Hash>(hashable: T) -> u64 {
    let mut hasher = std::hash::DefaultHasher::new();
    hashable.hash(&mut hasher);
    hasher.finish()
}

/// Trait representing a specific OSC address context, such as a track or send instance.
/// Implementors should provide identity, cloning, and a way to extract parameter values.
pub trait ContextTrait: Debug + Eq + Clone + std::hash::Hash {}

/// Trait representing a shape for an OSC address context, such as a track or send.
/// Implementors should provide parsing, identity, and cloning.
pub trait ContextKindTrait: Debug + Eq + Clone + std::hash::Hash {
    type Context: ContextTrait + 'static;
    /// Attempt to parse this context from the given OSC address.
    /// Returns Some(context instance) if matched, else None.
    fn parse(osc_address: &str) -> Option<Self::Context>
    where
        Self: Sized;

    /// Returns a human-readable name for this context (for logging, debugging).
    fn context_name() -> &'static str;
}

pub trait ContextGateBuilderTrait {
    fn build_boxed(self: Box<Self>) -> Box<dyn ContextualDispatcher>;
}

// Builder for a single context gate layer
pub struct ContextGateBuilder<K: ContextKindTrait> {
    key_routes: Vec<String>,
    on_initialized: Option<Box<dyn FnMut(K::Context, &HashMap<String, OscMessage>)>>,

    _marker: PhantomData<K>,
}

impl<K: ContextKindTrait> ContextGateBuilder<K> {
    pub fn new() -> Self {
        Self {
            key_routes: Vec::new(),
            on_initialized: None,
            _marker: PhantomData,
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
        F: FnMut(K::Context, &HashMap<String, OscMessage>) + 'static,
    {
        self.on_initialized = Some(Box::new(callback));
        self
    }

    fn build(self) -> ContextGate<K> {
        ContextGate {
            key_routes: self.key_routes,
            initialized: HashMap::new(),
            on_initialized: self.on_initialized,
            key_messages: HashMap::new(),
            _marker: PhantomData,
        }
    }
}

impl<K: ContextKindTrait + 'static> ContextGateBuilderTrait for ContextGateBuilder<K> {
    fn build_boxed(self: Box<Self>) -> Box<dyn ContextualDispatcher> {
        Box::new(ContextGateBuilder::build(*self))
    }
}

enum InitializationState {
    Uninitialized,
    AlreadyInitialized,
    NewlyInitialized,
}

trait ContextualDispatcher {
    fn initialization_state(
        &mut self,
        msg: &OscMessage,
    ) -> Option<(InitializationState, Option<u64>)>;

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
struct ContextGate<K: ContextKindTrait + 'static> {
    // the OSC address that "unlocks" this layer
    // E.g. for TrackGUID, this might be "/track/{track_guid}/index"
    key_routes: Vec<String>,
    // We buffer messages if this is false. When it's true, we pass messages through.
    // At the moment we set it true, we also flush the buffer.
    initialized: HashMap<K::Context, bool>,
    // Called when a specific context is initialized
    on_initialized: Option<Box<dyn FnMut(K::Context, &HashMap<String, OscMessage>)>>,
    key_messages: HashMap<K::Context, HashMap<String, OscMessage>>,

    _marker: PhantomData<K>,
}

impl<K: ContextKindTrait> ContextGate<K> {
    /// Mark a specific concrete OscContext as initialized
    pub fn initialize(&mut self, context: K::Context) {
        let key_messages = self.key_messages.get(&context).unwrap();

        if let Some(callback) = &mut self.on_initialized {
            callback(context.clone(), key_messages);
        }
        self.initialized.insert(context.clone(), true);
    }
}

impl<K: ContextKindTrait + 'static> ContextualDispatcher for ContextGate<K> {
    fn initialization_state(
        &mut self,
        msg: &OscMessage,
    ) -> Option<(InitializationState, Option<u64>)> {
        match K::parse(&msg.addr) {
            None => None,
            Some(context) => {
                // If this message is relevant to this layer...
                match self.initialized.get(&context) {
                    Some(true) => {
                        // context is already initialized, just dispatch
                        Some((
                            InitializationState::AlreadyInitialized,
                            Some(hash_to_u64(&context)),
                        ))
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

                            if has_all_key_messages {
                                // Initialize the context
                                self.initialize(context.clone());
                                Some((
                                    InitializationState::NewlyInitialized,
                                    Some(hash_to_u64(&context)),
                                ))
                            } else {
                                Some((
                                    InitializationState::Uninitialized,
                                    Some(hash_to_u64(&context)),
                                ))
                            }
                        } else {
                            Some((
                                InitializationState::Uninitialized,
                                Some(hash_to_u64(&context)),
                            ))
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

        info
    }
}

pub type Dispatcher = Box<dyn FnMut(OscMessage)>;

// Main builder for the router
pub struct OscGatedRouterBuilder {
    layers: Vec<Box<dyn ContextGateBuilderTrait>>,
    dispatcher: Dispatcher,
    buffer_timeout: Duration,
}

impl OscGatedRouterBuilder {
    pub fn new<F>(dispatcher: F) -> Self
    where
        F: FnMut(OscMessage) + 'static,
    {
        Self {
            layers: Vec::new(),
            dispatcher: Box::new(dispatcher),
            buffer_timeout: Duration::from_secs(60), // Default 1 minute timeout
        }
    }

    pub fn with_buffer_timeout(mut self, timeout: Duration) -> Self {
        self.buffer_timeout = timeout;
        self
    }

    pub fn add_layer(mut self, layer: Box<dyn ContextGateBuilderTrait>) -> Self {
        self.layers.push(layer);
        self
    }

    pub fn build(self) -> Result<OscGatedRouter, RouterBuildError> {
        // Instead of collecting directly, create the vector and push each element
        let mut layers: Vec<Box<dyn ContextualDispatcher>> = Vec::with_capacity(self.layers.len());

        for layer_builder in self.layers {
            // Explicitly cast each built layer as a Box<dyn ContextualDispatcher>
            // let layer: Box<dyn ContextualDispatcher> = Box::new(layer_builder.build());
            layers.push(layer_builder.build_boxed());
        }

        Ok(OscGatedRouter {
            layers,
            dispatcher: self.dispatcher,
            buffer_timeout: self.buffer_timeout,
            buffer: HashMap::new(),
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
    buffer: HashMap<u64, VecDeque<(OscMessage, Instant)>>,
}

impl OscGatedRouter {
    pub fn purge_stale_buffers(&mut self) {
        let now = Instant::now();
        // TODO: this needs to take timestamps on the keys in buffer and update those when
        // messages get buffered inside dispatch_osc
        for (_, messages) in self.buffer.iter_mut() {
            messages.retain(|(_, timestamp)| now.duration_since(*timestamp) <= self.buffer_timeout);
        }
    }

    /// dispatch_osc gates messages until their initialization condition is met and then passes
    /// messages through to self.dispatcher.
    pub fn dispatch_osc(&mut self, packet: OscPacket) {
        let msg = match &packet {
            OscPacket::Message(msg) => msg,
            _ => return,
        };

        let mut hasher = DefaultHasher::new();
        let mut gated = false;
        self.layers.iter_mut().for_each(|layer| {
            if let Some(res) = layer.initialization_state(msg) {
                if let Some(hash) = res.1 {
                    hash.hash(&mut hasher)
                }
                match res.0 {
                    InitializationState::Uninitialized => gated = true,
                    InitializationState::AlreadyInitialized => {}
                    InitializationState::NewlyInitialized => {}
                }
            }
        });
        let hash = hasher.finish();
        if gated {
            // Buffer the message
            let buffer = self.buffer.entry(hash).or_default();
            buffer.push_back((msg.to_owned(), Instant::now()));
        } else {
            // First, flush any buffered messages for this hash to preserve ordering
            if let Some(buffered_messages) = self.buffer.get(&hash) {
                for (buffered_msg, _) in buffered_messages {
                    (self.dispatcher)(buffered_msg.to_owned());
                }
                self.buffer.remove(&hash);
            }
            // Then, dispatch the current message
            (self.dispatcher)(msg.to_owned());
        }
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
    pub fn get_buffered_messages_count(&self, ctxs: Vec<impl std::hash::Hash>) -> usize {
        let mut hasher = DefaultHasher::new();
        let mut hashed_once = 0;
        for ctx in ctxs {
            hashed_once = hash_to_u64(ctx);
            hashed_once.hash(&mut hasher);
        }
        let hash = hasher.finish();
        self.buffer.get(&hash).map_or(0, |buf| buf.len())
    }
}
