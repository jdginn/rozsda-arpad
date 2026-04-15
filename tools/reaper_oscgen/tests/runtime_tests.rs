// Runtime integration tests for the reaper_oscgen generated code.
//
// The generated code imports `crate::traits::{Bind, Query, Set}` and
// `crate::osc::route_context::ContextTrait`.  In this integration-test crate
// `crate` refers to the anonymous test crate, so we define those modules here
// before including the generated file.

// ---------------------------------------------------------------------------
// Trait stubs – mirror what arpad-rust/src/traits.rs exports but without the
// `Send` bound so the generated Bind impls compile.
// ---------------------------------------------------------------------------
mod traits {
    pub trait Bind<Args> {
        fn bind<F>(&mut self, callback: F)
        where
            F: FnMut(Args) + 'static;
    }

    pub trait Set<Args> {
        type Error;
        fn set(&mut self, args: Args) -> Result<(), Self::Error>;
    }

    pub trait Query {
        type Error;
        fn query(&self) -> Result<(), Self::Error>;
    }
}

mod osc {
    pub mod route_context {
        pub trait ContextTrait: std::fmt::Debug + Eq + Clone + std::hash::Hash {}

        pub trait ContextKindTrait: std::fmt::Debug + Eq + Clone + std::hash::Hash {
            type Context: ContextTrait + 'static;
            fn parse(osc_address: &str) -> Option<Self::Context>
            where
                Self: Sized;
            fn context_name() -> &'static str;
        }
    }
}

// ---------------------------------------------------------------------------
// Include the pre-generated code.  This file is produced by running:
//   cargo run -p reaper_oscgen -- tests/fixtures/oscgen_test_routes.yaml \
//     -o tests/generated/test_generated.rs
// ---------------------------------------------------------------------------
include!("generated/test_generated.rs");

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

/// Create a connected UDP socket pair for testing outbound OSC messages.
/// Returns `(sender_arc, receiver)`.
fn make_udp_pair() -> (Arc<UdpSocket>, UdpSocket) {
    let receiver = UdpSocket::bind("127.0.0.1:0").expect("bind receiver");
    let addr = receiver.local_addr().expect("local_addr");
    let sender = UdpSocket::bind("127.0.0.1:0").expect("bind sender");
    sender.connect(addr).expect("connect sender");
    receiver
        .set_read_timeout(Some(Duration::from_millis(500)))
        .expect("set_read_timeout");
    (Arc::new(sender), receiver)
}

/// Receive one OSC message from `receiver`, panic on timeout or decode failure.
fn recv_osc(receiver: &UdpSocket) -> rosc::OscMessage {
    let mut buf = [0u8; 4096];
    let (size, _) = receiver.recv_from(&mut buf).expect("recv_from");
    let (_, packet) = rosc::decoder::decode_udp(&buf[..size]).expect("decode_udp");
    match packet {
        rosc::OscPacket::Message(msg) => msg,
        other => panic!("Expected OscMessage, got {:?}", other),
    }
}

/// Build a `Reaper` with the sender socket.
fn make_reaper(socket: Arc<UdpSocket>) -> Reaper {
    Reaper::new(socket)
}

// ---------------------------------------------------------------------------
// B) Query tests – outbound UDP: address ends with '?', no args
// ---------------------------------------------------------------------------

#[test]
fn query_appends_question_mark_no_args_num() {
    let (sender, receiver) = make_udp_pair();
    let mut reaper = make_reaper(sender);
    reaper._test_oscgen_num().query().unwrap();
    let msg = recv_osc(&receiver);
    assert_eq!(msg.addr, "/_test/oscgen/num?");
    assert!(msg.args.is_empty(), "query must send no args");
}

#[test]
fn query_appends_question_mark_no_args_ping() {
    let (sender, receiver) = make_udp_pair();
    let mut reaper = make_reaper(sender);
    reaper._test_oscgen_ping().query().unwrap();
    let msg = recv_osc(&receiver);
    assert_eq!(msg.addr, "/_test/oscgen/ping?");
    assert!(msg.args.is_empty(), "query must send no args");
}

#[test]
fn query_appends_question_mark_no_args_gain_with_params() {
    let (sender, receiver) = make_udp_pair();
    let mut reaper = make_reaper(sender);
    reaper
        ._test_oscgen_item_slot_gain("abc".to_string(), 2)
        .query()
        .unwrap();
    let msg = recv_osc(&receiver);
    assert_eq!(msg.addr, "/_test/oscgen/item/abc/slot/2/gain?");
    assert!(msg.args.is_empty(), "query must send no args");
}

// ---------------------------------------------------------------------------
// C) Set tests – outbound UDP: correct address and OSC type
// ---------------------------------------------------------------------------

#[test]
fn set_sends_string_arg_set_name() {
    let (sender, receiver) = make_udp_pair();
    let mut reaper = make_reaper(sender);
    reaper
        ._test_oscgen_set_name()
        .set(TestOscgenSetNameArgs {
            name: "hello".to_string(),
        })
        .unwrap();
    let msg = recv_osc(&receiver);
    assert_eq!(msg.addr, "/_test/oscgen/set_name");
    assert_eq!(msg.args.len(), 1);
    assert_eq!(msg.args[0], rosc::OscType::String("hello".to_string()));
}

#[test]
fn set_sends_bool_arg_item_enabled() {
    let (sender, receiver) = make_udp_pair();
    let mut reaper = make_reaper(sender);
    reaper
        ._test_oscgen_item_enabled("myid".to_string())
        .set(TestOscgenItemEnabledArgs { enabled: true })
        .unwrap();
    let msg = recv_osc(&receiver);
    assert_eq!(msg.addr, "/_test/oscgen/item/myid/enabled");
    assert_eq!(msg.args.len(), 1);
    assert_eq!(msg.args[0], rosc::OscType::Bool(true));
}

#[test]
fn set_sends_float_arg_item_slot_gain() {
    let (sender, receiver) = make_udp_pair();
    let mut reaper = make_reaper(sender);
    reaper
        ._test_oscgen_item_slot_gain("abc".to_string(), 2)
        .set(TestOscgenItemSlotGainArgs { gain: 0.5 })
        .unwrap();
    let msg = recv_osc(&receiver);
    assert_eq!(msg.addr, "/_test/oscgen/item/abc/slot/2/gain");
    assert_eq!(msg.args.len(), 1);
    assert_eq!(msg.args[0], rosc::OscType::Float(0.5));
}

// ---------------------------------------------------------------------------
// D) Dispatch routing tests
// ---------------------------------------------------------------------------

#[test]
fn dispatch_calls_bound_handler_for_num_with_int() {
    let (sender, _rx) = make_udp_pair();
    let mut reaper = make_reaper(sender);

    let called = Rc::new(RefCell::new(false));
    let received = Rc::new(RefCell::new(0i32));
    let called_c = Rc::clone(&called);
    let received_c = Rc::clone(&received);

    reaper._test_oscgen_num().bind(move |args: TestOscgenNumArgs| {
        *called_c.borrow_mut() = true;
        *received_c.borrow_mut() = args.num;
    });

    let msg = rosc::OscMessage {
        addr: "/_test/oscgen/num".to_string(),
        args: vec![rosc::OscType::Int(42)],
    };
    let mut unknown_called = false;
    let mut decode_error_called = false;
    dispatch_osc(
        &mut reaper,
        msg,
        |_| unknown_called = true,
        |_, _| decode_error_called = true,
    );

    assert!(*called.borrow(), "handler should have been called");
    assert_eq!(*received.borrow(), 42);
    assert!(!unknown_called, "unknown-route logger must not fire");
    assert!(!decode_error_called, "decode-error logger must not fire");
}

#[test]
fn dispatch_calls_bound_handler_for_ping_with_no_args() {
    let (sender, _rx) = make_udp_pair();
    let mut reaper = make_reaper(sender);

    let called = Rc::new(RefCell::new(false));
    let called_c = Rc::clone(&called);

    reaper
        ._test_oscgen_ping()
        .bind(move |_args: TestOscgenPingArgs| {
            *called_c.borrow_mut() = true;
        });

    let msg = rosc::OscMessage {
        addr: "/_test/oscgen/ping".to_string(),
        args: vec![],
    };
    let mut unknown_called = false;
    let mut decode_error_called = false;
    dispatch_osc(
        &mut reaper,
        msg,
        |_| unknown_called = true,
        |_, _| decode_error_called = true,
    );

    assert!(*called.borrow(), "handler must be called for 0-arg route");
    assert!(!unknown_called);
    assert!(!decode_error_called);
}

#[test]
fn dispatch_extracts_param_id_1_and_calls_correct_item_enabled_handler() {
    let (sender, _rx) = make_udp_pair();
    let mut reaper = make_reaper(sender);

    let called = Rc::new(RefCell::new(false));
    let got_value = Rc::new(RefCell::new(false));
    let called_c = Rc::clone(&called);
    let value_c = Rc::clone(&got_value);

    reaper
        ._test_oscgen_item_enabled("abc".to_string())
        .bind(move |args: TestOscgenItemEnabledArgs| {
            *called_c.borrow_mut() = true;
            *value_c.borrow_mut() = args.enabled;
        });

    let msg = rosc::OscMessage {
        addr: "/_test/oscgen/item/abc/enabled".to_string(),
        args: vec![rosc::OscType::Bool(true)],
    };
    let mut unknown_called = false;
    let mut decode_error_called = false;
    dispatch_osc(
        &mut reaper,
        msg,
        |_| unknown_called = true,
        |_, _| decode_error_called = true,
    );

    assert!(*called.borrow(), "handler should have been called");
    assert!(*got_value.borrow(), "handler should have received true");
    assert!(!unknown_called);
    assert!(!decode_error_called);
}

#[test]
fn dispatch_extracts_two_params_in_correct_order_and_calls_gain_handler() {
    // This test specifically guards against the placeholder-order swap bug:
    // args[0] must be id_1 (string) and args[1] must be slot (int).
    let (sender, _rx) = make_udp_pair();
    let mut reaper = make_reaper(sender);

    let called = Rc::new(RefCell::new(false));
    let got_gain = Rc::new(RefCell::new(0.0f32));
    let called_c = Rc::clone(&called);
    let gain_c = Rc::clone(&got_gain);

    reaper
        ._test_oscgen_item_slot_gain("abc".to_string(), 2)
        .bind(move |args: TestOscgenItemSlotGainArgs| {
            *called_c.borrow_mut() = true;
            *gain_c.borrow_mut() = args.gain;
        });

    let msg = rosc::OscMessage {
        addr: "/_test/oscgen/item/abc/slot/2/gain".to_string(),
        args: vec![rosc::OscType::Float(0.75)],
    };
    let mut unknown_called = false;
    let mut decode_error_called = false;
    dispatch_osc(
        &mut reaper,
        msg,
        |_| unknown_called = true,
        |_, _| decode_error_called = true,
    );

    assert!(*called.borrow(), "handler should have been called");
    assert!(
        (*got_gain.borrow() - 0.75).abs() < 1e-6,
        "gain should be 0.75, got {}",
        *got_gain.borrow()
    );
    assert!(!unknown_called);
    assert!(!decode_error_called);
}

// ---------------------------------------------------------------------------
// E) Unknown route / decode-error structured logging
// ---------------------------------------------------------------------------

#[test]
fn dispatch_unknown_route_calls_unknown_logger_only() {
    let (sender, _rx) = make_udp_pair();
    let mut reaper = make_reaper(sender);

    let mut unknown_addr: Option<String> = None;
    let mut decode_error_called = false;

    let msg = rosc::OscMessage {
        addr: "/_test/oscgen/does_not_exist".to_string(),
        args: vec![],
    };
    dispatch_osc(
        &mut reaper,
        msg,
        |a| unknown_addr = Some(a.to_string()),
        |_, _| decode_error_called = true,
    );

    assert_eq!(
        unknown_addr.as_deref(),
        Some("/_test/oscgen/does_not_exist")
    );
    assert!(!decode_error_called);
}

#[test]
fn dispatch_missing_arg_logs_decode_error_not_unknown() {
    let (sender, _rx) = make_udp_pair();
    let mut reaper = make_reaper(sender);

    // Bind a handler so the decode path is exercised.
    reaper._test_oscgen_num().bind(|_: TestOscgenNumArgs| {});

    let mut unknown_called = false;
    let mut got_error: Option<String> = None;

    let msg = rosc::OscMessage {
        addr: "/_test/oscgen/num".to_string(),
        args: vec![], // missing the required int arg
    };
    dispatch_osc(
        &mut reaper,
        msg,
        |_| unknown_called = true,
        |_addr, err| match err {
            DispatchError::MissingArgument { arg_index: 0 } => {
                got_error = Some("MissingArgument".to_string())
            }
            other => got_error = Some(format!("unexpected: {:?}", other)),
        },
    );

    assert!(!unknown_called, "unknown-route logger must not fire");
    assert_eq!(
        got_error.as_deref(),
        Some("MissingArgument"),
        "expected MissingArgument decode error"
    );
}

#[test]
fn dispatch_wrong_arg_type_logs_decode_error_not_unknown() {
    let (sender, _rx) = make_udp_pair();
    let mut reaper = make_reaper(sender);

    reaper._test_oscgen_num().bind(|_: TestOscgenNumArgs| {});

    let mut unknown_called = false;
    let mut got_error: Option<String> = None;

    let msg = rosc::OscMessage {
        addr: "/_test/oscgen/num".to_string(),
        args: vec![rosc::OscType::String("x".to_string())], // wrong type
    };
    dispatch_osc(
        &mut reaper,
        msg,
        |_| unknown_called = true,
        |_addr, err| match err {
            DispatchError::WrongArgumentType {
                expected: "int",
                got: "string",
            } => got_error = Some("WrongArgumentType".to_string()),
            other => got_error = Some(format!("unexpected: {:?}", other)),
        },
    );

    assert!(!unknown_called);
    assert_eq!(got_error.as_deref(), Some("WrongArgumentType"));
}

#[test]
fn dispatch_param_parse_error_logs_decode_error_not_unknown() {
    let (sender, _rx) = make_udp_pair();
    let mut reaper = make_reaper(sender);

    // Bind a handler for slot=0 (doesn't matter, we're testing the param parse).
    reaper
        ._test_oscgen_item_slot_gain("abc".to_string(), 0)
        .bind(|_: TestOscgenItemSlotGainArgs| {});

    let mut unknown_called = false;
    let mut got_error: Option<String> = None;

    let msg = rosc::OscMessage {
        addr: "/_test/oscgen/item/abc/slot/not_an_int/gain".to_string(),
        args: vec![rosc::OscType::Float(0.5)],
    };
    dispatch_osc(
        &mut reaper,
        msg,
        |_| unknown_called = true,
        |_addr, err| match err {
            DispatchError::ParamParseError {
                param: "slot",
                value,
            } => got_error = Some(format!("ParamParseError({})", value)),
            other => got_error = Some(format!("unexpected: {:?}", other)),
        },
    );

    assert!(!unknown_called);
    assert_eq!(
        got_error.as_deref(),
        Some("ParamParseError(not_an_int)"),
        "expected ParamParseError for slot"
    );
}

#[test]
fn dispatch_when_handler_not_bound_does_not_log_or_panic() {
    let (sender, _rx) = make_udp_pair();
    let mut reaper = make_reaper(sender);

    // Do NOT bind any handler.
    let mut unknown_called = false;
    let mut decode_error_called = false;

    let msg = rosc::OscMessage {
        addr: "/_test/oscgen/num".to_string(),
        args: vec![rosc::OscType::Int(1)],
    };
    dispatch_osc(
        &mut reaper,
        msg,
        |_| unknown_called = true,
        |_, _| decode_error_called = true,
    );

    assert!(!unknown_called, "known route must not call unknown logger");
    assert!(!decode_error_called, "unbound handler must not log decode errors");
}

// ---------------------------------------------------------------------------
// F) Endpoint instance separation
// ---------------------------------------------------------------------------

#[test]
fn dispatch_routes_to_correct_endpoint_instance_between_two_ids() {
    let (sender, _rx) = make_udp_pair();
    let mut reaper = make_reaper(sender);

    let called_a = Rc::new(RefCell::new(false));
    let called_b = Rc::new(RefCell::new(false));
    let a_c = Rc::clone(&called_a);
    let b_c = Rc::clone(&called_b);

    reaper
        ._test_oscgen_item_enabled("a".to_string())
        .bind(move |_: TestOscgenItemEnabledArgs| {
            *a_c.borrow_mut() = true;
        });
    reaper
        ._test_oscgen_item_enabled("b".to_string())
        .bind(move |_: TestOscgenItemEnabledArgs| {
            *b_c.borrow_mut() = true;
        });

    // Dispatch to "a"
    dispatch_osc(
        &mut reaper,
        rosc::OscMessage {
            addr: "/_test/oscgen/item/a/enabled".to_string(),
            args: vec![rosc::OscType::Bool(false)],
        },
        |_| {},
        |_, _| {},
    );

    assert!(*called_a.borrow(), "handler for 'a' should fire");
    assert!(!*called_b.borrow(), "handler for 'b' must NOT fire");

    // Dispatch to "b"
    dispatch_osc(
        &mut reaper,
        rosc::OscMessage {
            addr: "/_test/oscgen/item/b/enabled".to_string(),
            args: vec![rosc::OscType::Bool(true)],
        },
        |_| {},
        |_, _| {},
    );

    assert!(*called_b.borrow(), "handler for 'b' should fire");
}

#[test]
fn dispatch_routes_to_correct_endpoint_instance_between_two_slots_same_id() {
    let (sender, _rx) = make_udp_pair();
    let mut reaper = make_reaper(sender);

    let called_1 = Rc::new(RefCell::new(false));
    let called_2 = Rc::new(RefCell::new(false));
    let c1 = Rc::clone(&called_1);
    let c2 = Rc::clone(&called_2);

    reaper
        ._test_oscgen_item_slot_gain("abc".to_string(), 1)
        .bind(move |_: TestOscgenItemSlotGainArgs| {
            *c1.borrow_mut() = true;
        });
    reaper
        ._test_oscgen_item_slot_gain("abc".to_string(), 2)
        .bind(move |_: TestOscgenItemSlotGainArgs| {
            *c2.borrow_mut() = true;
        });

    // Dispatch to slot 1
    dispatch_osc(
        &mut reaper,
        rosc::OscMessage {
            addr: "/_test/oscgen/item/abc/slot/1/gain".to_string(),
            args: vec![rosc::OscType::Float(0.1)],
        },
        |_| {},
        |_, _| {},
    );

    assert!(*called_1.borrow(), "slot-1 handler should fire");
    assert!(!*called_2.borrow(), "slot-2 handler must NOT fire");

    // Dispatch to slot 2
    dispatch_osc(
        &mut reaper,
        rosc::OscMessage {
            addr: "/_test/oscgen/item/abc/slot/2/gain".to_string(),
            args: vec![rosc::OscType::Float(0.9)],
        },
        |_| {},
        |_, _| {},
    );

    assert!(*called_2.borrow(), "slot-2 handler should fire");
}

// ---------------------------------------------------------------------------
// G) Generator regression: verify the generated file is up to date
// ---------------------------------------------------------------------------
#[test]
fn generated_file_matches_current_generator_output() {
    use std::process::Command;
    // Run the generator binary against the fixture YAML and capture output.
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let fixture = format!("{}/tests/fixtures/oscgen_test_routes.yaml", manifest_dir);
    let tmp_out = std::env::temp_dir().join("reaper_oscgen_test_regen.rs");

    let status = Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--manifest-path",
            &format!("{}/Cargo.toml", manifest_dir),
            "--bin",
            "reaper_oscgen",
            "--",
            &fixture,
            "-o",
            tmp_out.to_str().unwrap(),
        ])
        .status()
        .expect("failed to run reaper_oscgen generator");

    assert!(status.success(), "generator must exit successfully");

    let expected = std::fs::read_to_string(&tmp_out).expect("read regenerated file");
    let committed = std::fs::read_to_string(format!(
        "{}/tests/generated/test_generated.rs",
        manifest_dir
    ))
    .expect("read committed generated file");

    assert_eq!(
        committed, expected,
        "tests/generated/test_generated.rs is out of date – regenerate with:\n  \
         cd tools/reaper_oscgen && \\\n  \
         cargo run --bin reaper_oscgen -- tests/fixtures/oscgen_test_routes.yaml \\\n    \
         -o tests/generated/test_generated.rs"
    );
}
