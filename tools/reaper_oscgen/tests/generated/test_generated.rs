// AUTO-GENERATED CODE. DO NOT EDIT!

use std::collections::HashMap;
use std::net::UdpSocket;
use std::sync::Arc;

use crate::traits::{Bind, Query, Set};

use crate::osc::route_context::ContextTrait;

#[derive(Debug)]
pub struct OscError;

#[derive(Debug)]
pub struct TestOscgenNumArgs {
    pub num: i32, // test-only integer payload
}

pub type TestOscgenNumHandler = Box<dyn FnMut(TestOscgenNumArgs) + 'static>;

pub struct TestOscgenNum {
    socket: Arc<UdpSocket>,
    handler: Option<TestOscgenNumHandler>,
}

/// /_test/oscgen/num
impl Bind<TestOscgenNumArgs> for TestOscgenNum {
    fn bind<F>(&mut self, callback: F)
    where
        F: FnMut(TestOscgenNumArgs) + 'static,
    {
        self.handler = Some(Box::new(callback));
    }
}

/// /_test/oscgen/num
impl Query for TestOscgenNum {
    type Error = OscError;
    fn query(&self) -> Result<(), Self::Error> {
        let osc_address = format!("/_test/oscgen/num?");
        let osc_msg = rosc::OscMessage {
            addr: osc_address,
            args: vec![],
        };
        let packet = rosc::OscPacket::Message(osc_msg);
        let buf = rosc::encoder::encode(&packet).map_err(|_| OscError)?;
        self.socket.send(&buf).map_err(|_| OscError)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct TestOscgenPingArgs {}

pub type TestOscgenPingHandler = Box<dyn FnMut(TestOscgenPingArgs) + 'static>;

pub struct TestOscgenPing {
    socket: Arc<UdpSocket>,
    handler: Option<TestOscgenPingHandler>,
}

/// /_test/oscgen/ping
impl Bind<TestOscgenPingArgs> for TestOscgenPing {
    fn bind<F>(&mut self, callback: F)
    where
        F: FnMut(TestOscgenPingArgs) + 'static,
    {
        self.handler = Some(Box::new(callback));
    }
}

/// /_test/oscgen/ping
impl Query for TestOscgenPing {
    type Error = OscError;
    fn query(&self) -> Result<(), Self::Error> {
        let osc_address = format!("/_test/oscgen/ping?");
        let osc_msg = rosc::OscMessage {
            addr: osc_address,
            args: vec![],
        };
        let packet = rosc::OscPacket::Message(osc_msg);
        let buf = rosc::encoder::encode(&packet).map_err(|_| OscError)?;
        self.socket.send(&buf).map_err(|_| OscError)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct TestOscgenSetNameArgs {
    pub name: String, // test-only string payload
}

pub type TestOscgenSetNameHandler = Box<dyn FnMut(TestOscgenSetNameArgs) + 'static>;

pub struct TestOscgenSetName {
    socket: Arc<UdpSocket>,
    handler: Option<TestOscgenSetNameHandler>,
}

/// /_test/oscgen/set_name
impl Set<TestOscgenSetNameArgs> for TestOscgenSetName {
    type Error = OscError;
    fn set(&mut self, args: TestOscgenSetNameArgs) -> Result<(), Self::Error> {
        let osc_address = format!("/_test/oscgen/set_name");
        let osc_msg = rosc::OscMessage {
            addr: osc_address,
            args: vec![rosc::OscType::String(args.name.clone())],
        };
        let packet = rosc::OscPacket::Message(osc_msg);
        let buf = rosc::encoder::encode(&packet).map_err(|_| OscError)?;
        self.socket.send(&buf).map_err(|_| OscError)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct TestOscgenItemEnabledArgs {
    pub enabled: bool, // test-only boolean payload
}

pub type TestOscgenItemEnabledHandler = Box<dyn FnMut(TestOscgenItemEnabledArgs) + 'static>;

pub struct TestOscgenItemEnabled {
    socket: Arc<UdpSocket>,
    handler: Option<TestOscgenItemEnabledHandler>,
    pub id_1: String,
}

/// /_test/oscgen/item/{id_1}/enabled
impl Set<TestOscgenItemEnabledArgs> for TestOscgenItemEnabled {
    type Error = OscError;
    fn set(&mut self, args: TestOscgenItemEnabledArgs) -> Result<(), Self::Error> {
        let osc_address = format!("/_test/oscgen/item/{}/enabled", self.id_1);
        let osc_msg = rosc::OscMessage {
            addr: osc_address,
            args: vec![rosc::OscType::Bool(args.enabled)],
        };
        let packet = rosc::OscPacket::Message(osc_msg);
        let buf = rosc::encoder::encode(&packet).map_err(|_| OscError)?;
        self.socket.send(&buf).map_err(|_| OscError)?;
        Ok(())
    }
}

/// /_test/oscgen/item/{id_1}/enabled
impl Bind<TestOscgenItemEnabledArgs> for TestOscgenItemEnabled {
    fn bind<F>(&mut self, callback: F)
    where
        F: FnMut(TestOscgenItemEnabledArgs) + 'static,
    {
        self.handler = Some(Box::new(callback));
    }
}

#[derive(Debug)]
pub struct TestOscgenItemSlotGainArgs {
    pub gain: f32, // test-only float payload
}

pub type TestOscgenItemSlotGainHandler = Box<dyn FnMut(TestOscgenItemSlotGainArgs) + 'static>;

pub struct TestOscgenItemSlotGain {
    socket: Arc<UdpSocket>,
    handler: Option<TestOscgenItemSlotGainHandler>,
    pub id_1: String,
    pub slot: i32,
}

/// /_test/oscgen/item/{id_1}/slot/{slot}/gain
impl Set<TestOscgenItemSlotGainArgs> for TestOscgenItemSlotGain {
    type Error = OscError;
    fn set(&mut self, args: TestOscgenItemSlotGainArgs) -> Result<(), Self::Error> {
        let osc_address = format!("/_test/oscgen/item/{}/slot/{}/gain", self.id_1, self.slot);
        let osc_msg = rosc::OscMessage {
            addr: osc_address,
            args: vec![rosc::OscType::Float(args.gain)],
        };
        let packet = rosc::OscPacket::Message(osc_msg);
        let buf = rosc::encoder::encode(&packet).map_err(|_| OscError)?;
        self.socket.send(&buf).map_err(|_| OscError)?;
        Ok(())
    }
}

/// /_test/oscgen/item/{id_1}/slot/{slot}/gain
impl Bind<TestOscgenItemSlotGainArgs> for TestOscgenItemSlotGain {
    fn bind<F>(&mut self, callback: F)
    where
        F: FnMut(TestOscgenItemSlotGainArgs) + 'static,
    {
        self.handler = Some(Box::new(callback));
    }
}

/// /_test/oscgen/item/{id_1}/slot/{slot}/gain
impl Query for TestOscgenItemSlotGain {
    type Error = OscError;
    fn query(&self) -> Result<(), Self::Error> {
        let osc_address = format!("/_test/oscgen/item/{}/slot/{}/gain?", self.id_1, self.slot);
        let osc_msg = rosc::OscMessage {
            addr: osc_address,
            args: vec![],
        };
        let packet = rosc::OscPacket::Message(osc_msg);
        let buf = rosc::encoder::encode(&packet).map_err(|_| OscError)?;
        self.socket.send(&buf).map_err(|_| OscError)?;
        Ok(())
    }
}

pub mod context {
    use crate::osc::route_context::ContextTrait;

    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    pub struct Item {
        pub id_1: String,
    }

    impl ContextTrait for Item {}

    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    pub struct ItemSlot {
        pub id_1: String,
        pub slot: i32,
    }

    impl ContextTrait for ItemSlot {}
}

pub mod context_kind {
    use super::context;
    use crate::osc::route_context::ContextKindTrait;
    use regex::Regex;

    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    pub struct Item {}

    impl ContextKindTrait for Item {
        type Context = context::Item;

        fn context_name() -> &'static str {
            "Item"
        }

        fn parse(osc_address: &str) -> Option<context::Item> {
            let re = Regex::new(r"^/_test/oscgen/item/([^/]+)/enabled$").unwrap();
            re.captures(osc_address).map(|caps| context::Item {
                id_1: caps[1].to_string(),
            })
        }
    }

    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    pub struct ItemSlot {}

    impl ContextKindTrait for ItemSlot {
        type Context = context::ItemSlot;

        fn context_name() -> &'static str {
            "ItemSlot"
        }

        fn parse(osc_address: &str) -> Option<context::ItemSlot> {
            let re = Regex::new(r"^/_test/oscgen/item/([^/]+)/slot/([^/]+)/gain$").unwrap();
            re.captures(osc_address).map(|caps| context::ItemSlot {
                id_1: caps[1].to_string(),
                slot: caps[2].parse().unwrap(),
            })
        }
    }
}

pub struct Reaper {
    socket: Arc<UdpSocket>,
    _test_oscgen_num_endpoint: TestOscgenNum,
    _test_oscgen_ping_endpoint: TestOscgenPing,
    _test_oscgen_set_name_endpoint: TestOscgenSetName,
    _test_oscgen_item_enabled_endpoints: HashMap<String, TestOscgenItemEnabled>,
    _test_oscgen_item_slot_gain_endpoints: HashMap<String, HashMap<i32, TestOscgenItemSlotGain>>,
}

impl Reaper {
    pub fn new(socket: Arc<UdpSocket>) -> Self {
        Self {
            socket: socket.clone(),
            _test_oscgen_num_endpoint: TestOscgenNum {
                socket: socket.clone(),
                handler: None,
            },
            _test_oscgen_ping_endpoint: TestOscgenPing {
                socket: socket.clone(),
                handler: None,
            },
            _test_oscgen_set_name_endpoint: TestOscgenSetName {
                socket: socket.clone(),
                handler: None,
            },
            _test_oscgen_item_enabled_endpoints: HashMap::new(),
            _test_oscgen_item_slot_gain_endpoints: HashMap::new(),
        }
    }
}

impl Reaper {
    pub fn _test_oscgen_num(&mut self) -> &mut TestOscgenNum {
        &mut self._test_oscgen_num_endpoint
    }
    pub fn _test_oscgen_ping(&mut self) -> &mut TestOscgenPing {
        &mut self._test_oscgen_ping_endpoint
    }
    pub fn _test_oscgen_set_name(&mut self) -> &mut TestOscgenSetName {
        &mut self._test_oscgen_set_name_endpoint
    }
    pub fn _test_oscgen_item_enabled(&mut self, id_1: String) -> &mut TestOscgenItemEnabled {
        self._test_oscgen_item_enabled_endpoints
            .entry(id_1.clone())
            .or_insert_with(|| TestOscgenItemEnabled {
                socket: self.socket.clone(),
                id_1: id_1,
                handler: None,
            })
    }
    pub fn _test_oscgen_item_slot_gain(
        &mut self,
        id_1: String,
        slot: i32,
    ) -> &mut TestOscgenItemSlotGain {
        self._test_oscgen_item_slot_gain_endpoints
            .entry(id_1.clone())
            .or_insert_with(|| HashMap::new())
            .entry(slot.clone())
            .or_insert_with(|| TestOscgenItemSlotGain {
                socket: self.socket.clone(),
                id_1: id_1,
                slot: slot,
                handler: None,
            })
    }
}

#[derive(Debug)]
pub enum DispatchError {
    MissingArgument {
        arg_index: usize,
    },
    WrongArgumentType {
        expected: &'static str,
        got: &'static str,
    },
    ParamParseError {
        param: &'static str,
        value: String,
    },
}

fn osc_type_name(t: &rosc::OscType) -> &'static str {
    match t {
        rosc::OscType::Int(_) => "int",
        rosc::OscType::Float(_) => "float",
        rosc::OscType::String(_) => "string",
        rosc::OscType::Bool(_) => "bool",
        rosc::OscType::Double(_) => "double",
        rosc::OscType::Long(_) => "long",
        _ => "unknown",
    }
}

/// Try to match an OSC address against a pattern, extracting arguments.
/// E.g. addr: "/track/abc123/pan", pattern: "/track/{}/pan" -> Some(vec!["abc123"])
fn match_addr(addr: &str, pattern: &str) -> Option<Vec<String>> {
    let addr_parts: Vec<&str> = addr.split('/').filter(|s| !s.is_empty()).collect();
    let pat_parts: Vec<&str> = pattern.split('/').filter(|s| !s.is_empty()).collect();
    if addr_parts.len() != pat_parts.len() {
        return None;
    }
    let mut args = Vec::new();
    for (a, p) in addr_parts.iter().zip(pat_parts.iter()) {
        if p.starts_with('{') && p.ends_with('}') {
            args.push((*a).to_string());
        } else if *p != *a {
            return None;
        }
    }
    Some(args)
}

pub fn dispatch_osc<F, G>(
    reaper: &mut Reaper,
    msg: rosc::OscMessage,
    mut log_unknown_route: F,
    mut log_decode_error: G,
) where
    F: FnMut(&str),
    G: FnMut(&str, DispatchError),
{
    let addr = msg.addr.as_str();
    if let Some(_args) = match_addr(addr, "/_test/oscgen/num") {
        let endpoint = reaper._test_oscgen_num();
        if let Some(handler) = &mut endpoint.handler {
            let _decoded_num = match msg.args.get(0) {
                Some(_raw_arg_0) => match _raw_arg_0.clone().int() {
                    Some(v) => v,
                    None => {
                        log_decode_error(
                            addr,
                            DispatchError::WrongArgumentType {
                                expected: "int",
                                got: osc_type_name(_raw_arg_0),
                            },
                        );
                        return;
                    }
                },
                None => {
                    log_decode_error(addr, DispatchError::MissingArgument { arg_index: 0 });
                    return;
                }
            };
            handler(TestOscgenNumArgs { num: _decoded_num });
        }
        return;
    }
    if let Some(_args) = match_addr(addr, "/_test/oscgen/ping") {
        let endpoint = reaper._test_oscgen_ping();
        if let Some(handler) = &mut endpoint.handler {
            handler(TestOscgenPingArgs {});
        }
        return;
    }
    if let Some(_args) = match_addr(addr, "/_test/oscgen/set_name") {
        let endpoint = reaper._test_oscgen_set_name();
        if let Some(handler) = &mut endpoint.handler {
            let _decoded_name = match msg.args.get(0) {
                Some(_raw_arg_0) => match _raw_arg_0.clone().string() {
                    Some(v) => v,
                    None => {
                        log_decode_error(
                            addr,
                            DispatchError::WrongArgumentType {
                                expected: "string",
                                got: osc_type_name(_raw_arg_0),
                            },
                        );
                        return;
                    }
                },
                None => {
                    log_decode_error(addr, DispatchError::MissingArgument { arg_index: 0 });
                    return;
                }
            };
            handler(TestOscgenSetNameArgs {
                name: _decoded_name,
            });
        }
        return;
    }
    if let Some(args) = match_addr(addr, "/_test/oscgen/item/{id_1}/enabled") {
        let id_1 = args[0].clone();
        let endpoint = reaper._test_oscgen_item_enabled(id_1);
        if let Some(handler) = &mut endpoint.handler {
            let _decoded_enabled = match msg.args.get(0) {
                Some(_raw_arg_0) => match _raw_arg_0.clone().bool() {
                    Some(v) => v,
                    None => {
                        log_decode_error(
                            addr,
                            DispatchError::WrongArgumentType {
                                expected: "bool",
                                got: osc_type_name(_raw_arg_0),
                            },
                        );
                        return;
                    }
                },
                None => {
                    log_decode_error(addr, DispatchError::MissingArgument { arg_index: 0 });
                    return;
                }
            };
            handler(TestOscgenItemEnabledArgs {
                enabled: _decoded_enabled,
            });
        }
        return;
    }
    if let Some(args) = match_addr(addr, "/_test/oscgen/item/{id_1}/slot/{slot}/gain") {
        let id_1 = args[0].clone();
        let slot: i32 = match args[1].parse::<i32>() {
            Ok(v) => v,
            Err(_) => {
                log_decode_error(
                    addr,
                    DispatchError::ParamParseError {
                        param: "slot",
                        value: args[1].clone(),
                    },
                );
                return;
            }
        };
        let endpoint = reaper._test_oscgen_item_slot_gain(id_1, slot);
        if let Some(handler) = &mut endpoint.handler {
            let _decoded_gain = match msg.args.get(0) {
                Some(_raw_arg_0) => match _raw_arg_0.clone().float() {
                    Some(v) => v,
                    None => {
                        log_decode_error(
                            addr,
                            DispatchError::WrongArgumentType {
                                expected: "float",
                                got: osc_type_name(_raw_arg_0),
                            },
                        );
                        return;
                    }
                },
                None => {
                    log_decode_error(addr, DispatchError::MissingArgument { arg_index: 0 });
                    return;
                }
            };
            handler(TestOscgenItemSlotGainArgs {
                gain: _decoded_gain,
            });
        }
        return;
    }
    log_unknown_route(addr);
}
