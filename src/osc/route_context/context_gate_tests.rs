use std::collections::HashMap;
use std::time::Duration;

use super::context_gate::{
    ContextGateBuilder, ContextKindTrait, ContextTrait, OscGatedRouter, OscGatedRouterBuilder,
};

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
