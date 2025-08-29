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
    fn dispatch_osc(&mut self, msg: OscMessage, dispatcher: &mut dyn FnMut(OscMessage));
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
    fn dispatch_osc(&mut self, msg: OscMessage, dispatcher: &mut dyn FnMut(OscMessage)) {
        if let Some(context) = self.parameter_sequence.parse(&msg.addr) {
            // Update timestamp for this context
            self.buffer_timestamps
                .insert(context.clone(), Instant::now());

            // If this message is relevant to this layer...
            match self.initialized.get(&context) {
                Some(true) => {
                    // context is already initialized, just dispatch
                    (dispatcher)(msg.to_owned());
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

                            // Process buffered messages
                            if let Some(buffer) = self.buffer.get_mut(&context) {
                                while let Some(buffered_msg) = buffer.pop_front() {
                                    (dispatcher)(buffered_msg);
                                }
                            }
                            // Dispatch this message
                            (dispatcher)(msg.clone());
                        }
                    } else {
                        // Not the key message; buffer it
                        let buffer = self.buffer.entry(context.clone()).or_default();
                        buffer.push_back(msg.clone());
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
            layer.dispatch_osc(msg.to_owned(), &mut *self.dispatcher);
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

#[cfg(test)]
mod tests {
    use super::*;
    use rosc::{OscMessage, OscPacket, OscType};
    use std::cell::RefCell;
    use std::rc::Rc;

    // Test-specific context implementation
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    struct TrackContext {
        track_guid: String,
    }

    impl ContextTrait for TrackContext {}

    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    struct TrackContextKind {}

    // Define a second context type for sends
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    struct SendContext {
        track_guid: String,
        send_index: String,
    }

    impl ContextTrait for SendContext {}

    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    struct SendContextKind {}

    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    enum RouterContext {
        Track(TrackContext),
        Send(SendContext),
        // Add more context types as needed
    }

    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    enum RouterContextKind {
        Track(TrackContextKind),
        Send(SendContextKind),
    }

    impl ContextTrait for RouterContext {}

    impl ContextKindTrait<RouterContext> for RouterContextKind {
        fn parse(&self, osc_address: &str) -> Option<RouterContext> {
            match self {
                RouterContextKind::Track(kind) => kind.parse(osc_address).map(RouterContext::Track),
                RouterContextKind::Send(kind) => kind.parse(osc_address).map(RouterContext::Send),
            }
        }

        fn context_name(&self) -> &'static str {
            match self {
                RouterContextKind::Track(kind) => kind.context_name(),
                RouterContextKind::Send(kind) => kind.context_name(),
            }
        }
    }

    impl ContextKindTrait<TrackContext> for TrackContextKind {
        fn parse(&self, osc_address: &str) -> Option<TrackContext> {
            let parts: Vec<&str> = osc_address.split('/').collect();
            if parts.len() >= 3 && parts[1] == "track" {
                Some(TrackContext {
                    track_guid: parts[2].to_string(),
                })
            } else {
                None
            }
        }

        fn context_name(&self) -> &'static str {
            "Track"
        }
    }

    // Test helper functions
    fn create_test_message(address: &str, args: Vec<OscType>) -> OscPacket {
        OscPacket::Message(OscMessage {
            addr: address.to_string(),
            args,
        })
    }

    fn create_test_router() -> (OscGatedRouter, Rc<RefCell<Vec<OscMessage>>>) {
        let received_messages = Rc::new(RefCell::new(Vec::new()));
        let received_messages_clone = received_messages.clone();

        let dispatcher = move |msg: OscMessage| {
            received_messages.borrow_mut().push(msg);
        };

        let router = OscGatedRouterBuilder::<TrackContext, TrackContextKind>::new()
            .with_dispatcher(dispatcher)
            .add_layer(
                ContextGateBuilder::new(TrackContextKind {})
                    .add_key_route("/track/{track_guid}/index")
                    .with_initialization_callback(|ctx, _| {
                        // In a real test you might want to capture this in another Rc<RefCell>
                        // to assert initialization happened
                    }),
            )
            .build()
            .unwrap();

        (router, received_messages_clone)
    }

    #[test]
    fn test_basic_routing() {
        let (mut router, received) = create_test_router();
        let context = TrackContext {
            track_guid: "12345".to_string(),
        };

        // Send a non-key message first (should be buffered)
        router.dispatch_osc(create_test_message(
            "/track/12345/volume",
            vec![OscType::Float(0.75)],
        ));

        // No messages should be received yet
        assert_eq!(received.borrow().len(), 0);
        assert_eq!(router.get_buffered_messages_count(&context), 1);
        assert!(!router.is_context_initialized(&context));

        // Send the key message (should unlock processing)
        router.dispatch_osc(create_test_message(
            "/track/12345/index",
            vec![OscType::Int(42)],
        ));

        // Both messages should now be received
        assert_eq!(received.borrow().len(), 2);
        assert_eq!(received.borrow()[0].addr, "/track/12345/volume");
        assert_eq!(received.borrow()[1].addr, "/track/12345/index");

        // Buffer should be empty and context initialized
        assert_eq!(router.get_buffered_messages_count(&context), 0);
        assert!(router.is_context_initialized(&context));
    }

    // Table-driven testing for multiple scenarios
    #[test]
    fn test_multiple_scenarios() {
        // Define test scenarios
        struct TestScenario {
            name: &'static str,
            messages: Vec<(&'static str, Vec<OscType>)>,
            expected_dispatched_count: usize,
            expected_initialized: bool,
        }

        let scenarios = vec![
            TestScenario {
                name: "key_first_then_others",
                messages: vec![
                    ("/track/abc/index", vec![OscType::Int(1)]),
                    ("/track/abc/volume", vec![OscType::Float(0.5)]),
                ],
                expected_dispatched_count: 2,
                expected_initialized: true,
            },
            TestScenario {
                name: "others_first_then_key",
                messages: vec![
                    ("/track/def/volume", vec![OscType::Float(0.7)]),
                    ("/track/def/pan", vec![OscType::Float(0.2)]),
                    ("/track/def/index", vec![OscType::Int(2)]),
                ],
                expected_dispatched_count: 3,
                expected_initialized: true,
            },
            TestScenario {
                name: "no_key_message",
                messages: vec![
                    ("/track/xyz/volume", vec![OscType::Float(0.3)]),
                    ("/track/xyz/pan", vec![OscType::Float(0.1)]),
                ],
                expected_dispatched_count: 0,
                expected_initialized: false,
            },
        ];

        // Run each scenario
        for scenario in scenarios {
            println!("Running scenario: {}", scenario.name);

            let (mut router, received) = create_test_router();

            // Extract the track_guid for this scenario
            let track_guid = scenario.messages[0].0.split('/').nth(2).unwrap();
            let context = TrackContext {
                track_guid: track_guid.to_string(),
            };

            // Dispatch all messages in this scenario
            for (addr, args) in &scenario.messages {
                router.dispatch_osc(create_test_message(addr, args.clone()));
            }

            // Check results
            assert_eq!(
                received.borrow().len(),
                scenario.expected_dispatched_count,
                "Scenario '{}' dispatched count mismatch",
                scenario.name
            );

            assert_eq!(
                router.is_context_initialized(&context),
                scenario.expected_initialized,
                "Scenario '{}' initialization status mismatch",
                scenario.name
            );
        }
    }

    #[test]
    fn test_timeout_purging() {
        use std::thread::sleep;

        // Create router with short timeout
        let received_messages = Rc::new(RefCell::new(Vec::new()));
        let router = OscGatedRouterBuilder::<TrackContext, TrackContextKind>::new()
            .with_dispatcher(move |msg| {
                received_messages.borrow_mut().push(msg);
            })
            .with_buffer_timeout(Duration::from_millis(10))
            .add_layer(
                ContextGateBuilder::new(TrackContextKind {})
                    .add_key_route("/track/{track_guid}/index"),
            )
            .build()
            .unwrap();

        let mut router = router;
        let context = TrackContext {
            track_guid: "timeout".to_string(),
        };

        // Send a non-key message
        router.dispatch_osc(create_test_message(
            "/track/timeout/volume",
            vec![OscType::Float(0.5)],
        ));

        // Wait longer than timeout
        sleep(Duration::from_millis(20));

        // Purge stale buffers
        router.purge_stale_buffers();

        // // Buffer should be empty
        assert_eq!(router.get_buffered_messages_count(&context), 0);
    }

    #[test]
    fn test_multiple_key_routes() {
        let (mut router, received) = create_test_router_with_multiple_keys();
        let context = TrackContext {
            track_guid: "multi123".to_string(),
        };

        // Send first key route
        router.dispatch_osc(create_test_message(
            "/track/multi123/index",
            vec![OscType::Int(42)],
        ));

        // // Check that context is NOT yet initialized
        assert!(!router.is_context_initialized(&context));
        assert_eq!(received.borrow().len(), 0);

        // Send second key route
        router.dispatch_osc(create_test_message(
            "/track/multi123/name",
            vec![OscType::String("Track 1".to_string())],
        ));

        // // Now context should be initialized and both messages processed
        assert!(router.is_context_initialized(&context));
        assert_eq!(received.borrow().len(), 2);
    }

    fn create_test_router_with_multiple_keys() -> (OscGatedRouter, Rc<RefCell<Vec<OscMessage>>>) {
        let received_messages = Rc::new(RefCell::new(Vec::new()));
        let received_messages_clone = received_messages.clone();

        let dispatcher = move |msg: OscMessage| {
            received_messages.borrow_mut().push(msg);
        };

        let router = OscGatedRouterBuilder::<TrackContext, TrackContextKind>::new()
            .with_dispatcher(dispatcher)
            .add_layer(
                ContextGateBuilder::new(TrackContextKind {})
                    .add_key_route("/track/{track_guid}/index")
                    .add_key_route("/track/{track_guid}/name"),
            )
            .build()
            .unwrap();

        (router, received_messages_clone)
    }

    #[test]
    fn test_multiple_contexts() {
        let (mut router, received) = create_test_router();

        // Send messages for track1
        router.dispatch_osc(create_test_message(
            "/track/track1/volume",
            vec![OscType::Float(0.5)],
        ));

        // Send messages for track2
        router.dispatch_osc(create_test_message(
            "/track/track2/volume",
            vec![OscType::Float(0.7)],
        ));

        // Initialize track1
        router.dispatch_osc(create_test_message(
            "/track/track1/index",
            vec![OscType::Int(1)],
        ));

        // Only track1's messages should be processed
        assert_eq!(received.borrow().len(), 2);
        assert!(router.is_context_initialized(&TrackContext {
            track_guid: "track1".to_string()
        }));
        assert!(!router.is_context_initialized(&TrackContext {
            track_guid: "track2".to_string()
        }));

        // Initialize track2
        router.dispatch_osc(create_test_message(
            "/track/track2/index",
            vec![OscType::Int(2)],
        ));

        // Now track2's messages should also be processed
        assert_eq!(received.borrow().len(), 4);
        assert!(router.is_context_initialized(&TrackContext {
            track_guid: "track2".to_string()
        }));
    }

    #[test]
    fn test_multiple_layers() {
        impl ContextKindTrait<SendContext> for SendContextKind {
            fn parse(&self, osc_address: &str) -> Option<SendContext> {
                let parts: Vec<&str> = osc_address.split('/').collect();
                if parts.len() >= 5 && parts[1] == "track" && parts[3] == "send" {
                    Some(SendContext {
                        track_guid: parts[2].to_string(),
                        send_index: parts[4].to_string(),
                    })
                } else {
                    None
                }
            }

            fn context_name(&self) -> &'static str {
                "Send"
            }
        }

        // Create a multi-layer router
        let received_messages = Rc::new(RefCell::new(Vec::new()));
        let initialized_contexts = Rc::new(RefCell::new(Vec::new()));

        let received_messages_clone = received_messages.clone();
        let dispatcher = move |msg: OscMessage| {
            received_messages_clone.borrow_mut().push(msg);
        };

        let mut router = OscGatedRouterBuilder::new()
            .with_dispatcher(dispatcher)
            .add_layer({
                let contexts = initialized_contexts.clone();
                ContextGateBuilder::new(RouterContextKind::Track(TrackContextKind {}))
                    .add_key_route("/track/{track_guid}/index")
                    .with_initialization_callback(move |ctx, _| {
                        if let RouterContext::Track(t_ctx) = ctx {
                            contexts
                                .borrow_mut()
                                .push(format!("Track:{}", t_ctx.track_guid));
                        }
                    })
            })
            .add_layer({
                let contexts = initialized_contexts.clone();
                ContextGateBuilder::new(RouterContextKind::Send(SendContextKind {}))
                    .add_key_route("/track/{track_guid}/send/{send_index}/guid")
                    .with_initialization_callback(move |ctx, _| {
                        if let RouterContext::Send(s_ctx) = ctx {
                            contexts
                                .borrow_mut()
                                .push(format!("Send:{}:{}", s_ctx.track_guid, s_ctx.send_index));
                        }
                    })
            })
            .build()
            .unwrap();

        // Test track messages
        router.dispatch_osc(create_test_message(
            "/track/track1/volume",
            vec![OscType::Float(0.5)],
        ));
        router.dispatch_osc(create_test_message(
            "/track/track1/index",
            vec![OscType::Int(1)],
        ));

        // Test send messages
        router.dispatch_osc(create_test_message(
            "/track/track1/send/0/volume",
            vec![OscType::Float(0.3)],
        ));
        router.dispatch_osc(create_test_message(
            "/track/track1/send/0/guid",
            vec![OscType::String("send-guid-123".to_string())],
        ));

        // Check results
        assert_eq!(received_messages.borrow().len(), 4);
        assert_eq!(initialized_contexts.borrow().len(), 2);
        assert!(
            initialized_contexts
                .borrow()
                .contains(&"Track:track1".to_string())
        );
        assert!(
            initialized_contexts
                .borrow()
                .contains(&"Send:track1:0".to_string())
        );
    }

    #[test]
    fn test_key_route_order_independence() {
        let scenarios = vec![
            vec!["/track/order1/index", "/track/order1/name"],
            vec!["/track/order2/name", "/track/order2/index"],
        ];

        for (i, scenario) in scenarios.iter().enumerate() {
            let (mut router, received) = create_test_router_with_multiple_keys();
            let track_guid = format!("order{}", i + 1);
            let context = TrackContext {
                track_guid: track_guid.clone(),
            };

            // Send key routes in the order specified by this scenario
            for &route in scenario {
                let args = if route.ends_with("/index") {
                    vec![OscType::Int(i as i32)]
                } else {
                    vec![OscType::String(format!("Track {}", i + 1))]
                };

                router.dispatch_osc(create_test_message(route, args));
            }

            // Context should be initialized regardless of order
            assert!(router.is_context_initialized(&context));
            assert_eq!(received.borrow().len(), 2);
        }
    }

    #[test]
    fn test_key_message_access_in_callback() {
        let key_message_values = Rc::new(RefCell::new(HashMap::new()));
        let key_message_values_clone = key_message_values.clone();

        let received_messages = Rc::new(RefCell::new(Vec::new()));

        let dispatcher = move |msg: OscMessage| {
            received_messages.borrow_mut().push(msg);
        };

        let mut router = OscGatedRouterBuilder::<TrackContext, TrackContextKind>::new()
            .with_dispatcher(dispatcher)
            .add_layer(
                ContextGateBuilder::new(TrackContextKind {})
                    .add_key_route("/track/{track_guid}/index")
                    .with_initialization_callback(move |ctx, key_msgs| {
                        // Extract the index value from the key message
                        if let Some(index_msg) = key_msgs.get("/track/{track_guid}/index") {
                            if let Some(OscType::Int(index)) = index_msg.args.get(0) {
                                key_message_values
                                    .borrow_mut()
                                    .insert(ctx.track_guid.clone(), *index);
                            }
                        }
                    }),
            )
            .build()
            .unwrap();

        // Send index message
        router.dispatch_osc(create_test_message(
            "/track/callback/index",
            vec![OscType::Int(42)],
        ));

        // Check that callback extracted the value
        assert_eq!(key_message_values_clone.borrow().get("callback"), Some(&42));
    }

    #[test]
    fn test_timeout_and_recovery() {
        use std::thread::sleep;

        // Create router with short timeout
        let received_messages = Rc::new(RefCell::new(Vec::new()));
        let received_messages_clone = received_messages.clone();

        let mut router = OscGatedRouterBuilder::<TrackContext, TrackContextKind>::new()
            .with_dispatcher(move |msg| {
                received_messages.borrow_mut().push(msg);
            })
            .with_buffer_timeout(Duration::from_millis(10))
            .add_layer(
                ContextGateBuilder::new(TrackContextKind {})
                    .add_key_route("/track/{track_guid}/index"),
            )
            .build()
            .unwrap();

        let context = TrackContext {
            track_guid: "recovery".to_string(),
        };

        // Send a non-key message
        router.dispatch_osc(create_test_message(
            "/track/recovery/volume",
            vec![OscType::Float(0.5)],
        ));

        // Wait longer than timeout
        sleep(Duration::from_millis(20));

        // Purge stale buffers
        router.purge_stale_buffers();

        // Buffer should be empty
        assert_eq!(router.get_buffered_messages_count(&context), 0);

        // Now send messages again for the same context
        router.dispatch_osc(create_test_message(
            "/track/recovery/pan",
            vec![OscType::Float(0.2)],
        ));

        router.dispatch_osc(create_test_message(
            "/track/recovery/index",
            vec![OscType::Int(5)],
        ));

        // Should process both messages
        assert_eq!(received_messages_clone.borrow().len(), 2);
        assert!(router.is_context_initialized(&context));
    }

    #[test]
    fn test_non_matching_messages() {
        let (mut router, received) = create_test_router();

        // Send a message that doesn't match any layer's context pattern
        router.dispatch_osc(create_test_message(
            "/unrelated/message",
            vec![OscType::String("hello".to_string())],
        ));

        // Message should pass through
        assert_eq!(received.borrow().len(), 1);
    }

    #[test]
    fn test_bulk_messages() {
        let (mut router, received) = create_test_router();

        // Generate 100 contexts
        for i in 0..100 {
            let track_guid = format!("bulk{}", i);

            // Send 5 non-key messages per context
            for j in 0..5 {
                router.dispatch_osc(create_test_message(
                    &format!("/track/{}/param{}", track_guid, j),
                    vec![OscType::Float(j as f32 / 10.0)],
                ));
            }
        }

        // No messages should be processed yet
        assert_eq!(received.borrow().len(), 0);

        // Initialize all contexts
        for i in 0..100 {
            let track_guid = format!("bulk{}", i);
            router.dispatch_osc(create_test_message(
                &format!("/track/{}/index", track_guid),
                vec![OscType::Int(i)],
            ));
        }

        // Should have processed all messages (100 contexts × 6 messages each)
        assert_eq!(received.borrow().len(), 100 * 6);
    }

    #[test]
    fn test_real_osc_packets() {
        let (mut router, received) = create_test_router();

        // Create a more complex OSC packet with multiple arguments
        let complex_msg = OscPacket::Message(OscMessage {
            addr: "/track/complex/volume".to_string(),
            args: vec![
                OscType::Float(0.75),              // volume level
                OscType::String("dB".to_string()), // unit
                OscType::Int(1),                   // enabled flag
                OscType::Bool(true),               // automation enabled
            ],
        });

        router.dispatch_osc(complex_msg);

        // Initialize the context
        router.dispatch_osc(create_test_message(
            "/track/complex/index",
            vec![OscType::Int(10)],
        ));

        // Check all arguments were preserved
        assert_eq!(received.borrow().len(), 2);
        assert_eq!(received.borrow()[0].args.len(), 4);
        match &received.borrow()[0].args[0] {
            OscType::Float(v) => assert_eq!(*v, 0.75),
            _ => panic!("Expected Float"),
        }
    }

    #[test]
    fn test_resource_usage() {
        use std::thread::sleep;

        let (mut router, _) = create_test_router();

        // Send many messages for many contexts without key routes
        for i in 0..1000 {
            let track_guid = format!("resource{}", i);

            router.dispatch_osc(create_test_message(
                &format!("/track/{}/volume", track_guid),
                vec![OscType::Float(0.5)],
            ));
        }

        // // Verify buffers are populated
        assert!(
            router.get_buffered_messages_count(&TrackContext {
                track_guid: "resource0".to_string()
            }) > 0
        );

        // Wait and purge
        sleep(Duration::from_millis(100));
        router.purge_stale_buffers();

        // // Verify buffers are cleared
        assert_eq!(
            router.get_buffered_messages_count(&TrackContext {
                track_guid: "resource0".to_string()
            }),
            0
        );

        // Memory usage should now be minimal
        // Note: In real tests you might want to use a memory profiler here
    }
}
