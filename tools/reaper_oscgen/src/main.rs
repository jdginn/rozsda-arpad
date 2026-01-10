use clap::Parser;
use regex::Regex;
use serde::Deserialize;
use std::collections::{BTreeMap, HashSet};
use std::fmt::{Display, Write};
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};

#[derive(Parser)]
struct Cli {
    /// Path to the OSC YAML spec file
    spec: PathBuf,
    /// Output Rust file
    #[clap(short, long, default_value = "generated_osc.rs")]
    out: PathBuf,
}

/// Convert "int" and "string" to Rust types
fn rust_type(yaml_type: &str) -> &str {
    match yaml_type {
        "int" => "i32",
        "string" => "String",
        "float" => "f32",
        "bool" => "bool",
        _ => "String", // fallback
    }
}

/// Sanitize a path segment to be a valid Rust identifier
fn sanitize_path_level(s: &str) -> String {
    s.replace("-", "_")
        .replace(" ", "_")
        .replace(".", "_")
        .replace("/", "_")
        .replace("?", "_")
        .replace("$", "_")
}

/// PascalCase a sanitized identifier (for struct names)
fn pascal_case(s: String) -> String {
    s.split('_')
        .filter(|p| !p.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
                None => "".to_string(),
            }
        })
        .collect::<String>()
}

// OSC param as represented in the YAML
#[derive(Debug, Deserialize, Clone)]
struct OscParam {
    name: String,
    #[serde(rename = "type")]
    typ: String,
    description: Option<String>,
}

impl Display for OscParam {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "OscParam {{ name: {}, type: {} }}", self.name, self.typ)
    }
}

// OSC argument as represented in the YAML
#[derive(Debug, Deserialize, Clone)]
struct OscArgument {
    name: String,
    #[serde(rename = "type")]
    typ: String,
    description: Option<String>,
}

impl Display for OscArgument {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "OscArgument {{ name: {}, type: {} }}",
            self.name, self.typ
        )
    }
}

#[derive(Debug, Deserialize, Clone, Eq, PartialEq, Hash)]
#[serde(rename_all = "lowercase")]
enum AccessTag {
    Readable,
    Writeable,
    Queryable,
}

impl Display for AccessTag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AccessTag::Readable => write!(f, "readable"),
            AccessTag::Writeable => write!(f, "writeable"),
            AccessTag::Queryable => write!(f, "queryable"),
        }
    }
}

// OSC route as represented in the YAML
#[derive(Debug, Deserialize, Clone)]
struct OscRoute {
    osc_address: String,
    params: Vec<OscParam>,
    arguments: Vec<OscArgument>,
    access_tags: HashSet<AccessTag>,
}

impl Display for OscRoute {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "OscRoute {{ osc_address: {}, arguments: {:?}, access_tags: {:?} }}",
            self.osc_address, self.arguments, self.access_tags
        )
    }
}

impl OscRoute {
    fn struct_name(&self) -> String {
        let mut name = String::new();
        let parts: Vec<_> = self
            .osc_address
            .split('/')
            .filter(|s| !s.is_empty())
            .map(sanitize_path_level)
            .collect();
        let mut i = 0;
        while i < parts.len() {
            let part = parts[i].clone();
            // If this segment is not a wildcard, and the next segment exists and is a wildcard, include it
            if !part.starts_with('{') && !part.ends_with('}') {
                name.push_str(&part[..1].to_uppercase());
                name.push_str(&part[1..]);
            }
            i += 1;
        }
        pascal_case(name)
    }

    fn accessor_name(&self) -> String {
        let mut name = String::new();
        let parts: Vec<_> = self
            .osc_address
            .split('/')
            .filter(|s| !s.is_empty())
            .map(sanitize_path_level)
            .collect();
        let mut i = 0;
        while i < parts.len() {
            let part = parts[i].clone();
            // If this segment is not a wildcard, and the next segment exists and is a wildcard, include it
            if !part.starts_with('{') && !part.ends_with('}') {
                if i > 0 {
                    name.push('_');
                }
                name.push_str(&part[..1].to_uppercase());
                name.push_str(&part[1..]);
            }
            i += 1;
        }
        name.to_lowercase()
    }
}

#[derive(Debug)]
struct ContextParam {
    name: String,
    typ: String,
}

fn write_imports(code: &mut String) {
    code.push_str("// AUTO-GENERATED CODE. DO NOT EDIT!\n\n");
    code.push_str("use std::net::UdpSocket;\n");
    code.push_str("use std::sync::Arc;\n\n");

    code.push_str("use crate::traits::{Bind, Set, Query};\n\n");

    code.push_str("use crate::osc::route_context::{ContextTrait};\n\n");

    code.push_str("#[derive(Debug)]\npub struct OscError;\n\n");
}

// Helper to extract wildcard path segments as context keys
fn extract_context_params(route: &OscRoute) -> Vec<ContextParam> {
    let mut keys = Vec::new();
    let re = Regex::new(r"\{([^}]+)\}").unwrap();
    for cap in re.captures_iter(&route.osc_address) {
        let name = cap[1].to_string();
        let ty = route
            .params
            .iter()
            .find(|a| a.name == *name)
            .map(|a| rust_type(a.typ.as_str()))
            .unwrap_or("String");
        keys.push(ContextParam {
            name,
            typ: ty.to_string(),
        });
    }
    keys
}

/// Helper to build a context name from the OSC path, e.g.
/// "/track/{track_guid}/send/{send_index}/guid" -> "TrackSend"
/// "/track/{track_guid}/index" -> "Track"
/// "/track/{track_guid}/send/{send_index}/volume" -> "TrackSend"
fn build_context_name(osc_address: &str) -> String {
    let mut name = String::new();
    let parts: Vec<_> = osc_address.split('/').filter(|s| !s.is_empty()).collect();
    let mut i = 0;
    while i < parts.len() {
        let part = parts[i];
        // If this segment is not a wildcard, and the next segment exists and is a wildcard, include it
        let next_is_wildcard = parts
            .get(i + 1)
            .map(|p| p.starts_with('{') && p.ends_with('}'))
            .unwrap_or(false);
        if !part.starts_with('{') && !part.ends_with('}') && next_is_wildcard {
            name.push_str(&part[..1].to_uppercase());
            name.push_str(&part[1..]);
        }
        i += 1;
    }
    name
}

fn write_context_struct_types(code: &mut String, routes: &[OscRoute]) {
    use std::collections::BTreeMap;

    // Step 0: Gather all unique contexts with their keys and arguments
    #[derive(Debug)]
    struct ContextInfo {
        name: String,
        parameters: Vec<ContextParam>,
        regex: Regex,
    }
    let mut contexts: BTreeMap<String, ContextInfo> = BTreeMap::new();

    for route in routes {
        let keys = extract_context_params(route); // TODO: make this
                                                  // return an option
        if keys.is_empty() {
            continue; // No context, skip
        }
        let name = build_context_name(&route.osc_address);
        let regex = osc_address_template_to_regex(&route.osc_address);
        contexts.entry(name.clone()).or_insert(ContextInfo {
            name,
            parameters: keys,
            regex: Regex::new(&regex).unwrap(),
        });
    }

    // Step 1: put these structs in a module
    writeln!(code, "pub mod context {{").unwrap();
    writeln!(code, "    use crate::osc::generated_osc::ContextTrait;\n").unwrap();

    // Step 2: Generate context structs
    for ctx in contexts.values() {
        writeln!(code, "    #[derive(Clone, Debug, PartialEq, Eq, Hash)]").unwrap();
        writeln!(code, "    pub struct {} {{", ctx.name).unwrap();
        for param in &ctx.parameters {
            writeln!(code, "        pub {}: {},", param.name, param.typ).unwrap();
        }
        writeln!(code, "    }}\n\n").unwrap();
        writeln!(code, "    impl ContextTrait for {} {{}}\n", ctx.name).unwrap();
    }
    writeln!(code, "}}\n\n").unwrap();

    writeln!(code, "pub mod context_kind {{").unwrap();
    writeln!(code, "    use regex::Regex;").unwrap();
    writeln!(code, "    use super::context;").unwrap();
    writeln!(
        code,
        "    use crate::osc::route_context::{{ContextKindTrait}};\n"
    )
    .unwrap();
    for ctx in contexts.values() {
        writeln!(code, "    #[derive(Clone, Debug, PartialEq, Eq, Hash)]").unwrap();
        writeln!(code, "    pub struct {} {{}}\n\n", ctx.name).unwrap();
        writeln!(code, "    impl ContextKindTrait for {} {{\n", ctx.name).unwrap();
        writeln!(code, "        type Context = context::{};\n\n", ctx.name).unwrap();
        writeln!(code, "        fn context_name() -> &'static str {{").unwrap();
        writeln!(code, "            \"{}\"\n", ctx.name).unwrap();
        writeln!(code, "        }}\n\n").unwrap();

        writeln!(
            code,
            "    fn parse(osc_address: &str) -> Option<context::{}> {{\n",
            ctx.name
        )
        .unwrap();
        // Compose capture logic
        let mut capture_fields = String::new();
        println!("Context parameters: {:?}", ctx.parameters);
        for (i, param) in ctx.parameters.iter().enumerate() {
            println!("param {} rust_type: {}", param.name, param.typ.as_str());
            match param.typ.as_str() {
                "i32" => capture_fields.push_str(&format!(
                    "{}: caps[{}].parse().unwrap(), ",
                    param.name,
                    i + 1
                )),
                "f32" => capture_fields.push_str(&format!(
                    "{}: caps[{}].parse().ok()?, ",
                    param.name,
                    i + 1
                )),
                "bool" => capture_fields.push_str(&format!(
                    "{}: caps[{}] == \"true\", ",
                    param.name,
                    i + 1
                )),
                _ => capture_fields.push_str(&format!(
                    "{}: caps[{}].to_string(), ",
                    param.name,
                    i + 1
                )),
            }
        }
        writeln!(
            code,
            "            let re = Regex::new(r\"{}{}\").unwrap();",
            ctx.regex,
            if ctx.parameters.is_empty() { "" } else { "" } // No extra required
        )
        .unwrap();
        writeln!(
            code,
            "            re.captures(osc_address).map(|caps| context::{}{{ {} }})",
            ctx.name, capture_fields
        )
        .unwrap();
        writeln!(code, "        }}\n").unwrap();
        writeln!(code, "    }}\n").unwrap();
    }
    writeln!(code, "}}\n\n").unwrap();
}

/// Generates a regex string for an OSC address template.
/// E.g. "/track/{track_guid}/index" -> r"^/track/([^/]+)/index$"
pub fn osc_address_template_to_regex(osc_address: &str) -> String {
    let mut regex = String::from("^");
    let mut chars = osc_address.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '{' => {
                // Skip until closing brace
                for c2 in chars.by_ref() {
                    if c2 == '}' {
                        break;
                    }
                }
                regex.push_str("([^/]+)");
            }
            _ => {
                regex.push(c);
            }
        }
    }
    regex.push('$');
    regex
}

fn write_node_struct_definition(code: &mut String, node: &OscRoute) {
    code.push_str(&format!(
        "pub type {0}Handler = Box<dyn FnMut({0}Args) + 'static>;\n\n",
        node.struct_name()
    ));

    code.push_str(&format!("pub struct {} {{\n", node.struct_name()));
    code.push_str("    socket: Arc<UdpSocket>,\n");
    code.push_str(&format!(
        "    handler: Option<{0}Handler>,\n",
        node.struct_name()
    ));

    for param in &node.params {
        code.push_str(&format!(
            "    pub {}: {},\n",
            param.name,
            rust_type(&param.typ)
        ));
    }
    code.push_str("}\n\n");
}

fn write_node_accessors(code: &mut String, routes: Vec<OscRoute>) {
    code.push_str("impl Reaper {\n");
    for route in routes {
        code.push_str(&format!("    pub fn {}(&self", route.accessor_name()));
        for param in &route.params {
            code.push_str(&format!(", {}: {}", param.name, rust_type(&param.typ)));
        }
        code.push_str(&format!(") -> {} {{\n", route.struct_name()));
        code.push_str(&format!("        {} {{\n", route.struct_name()));
        code.push_str("        socket: self.socket.clone(),\n");
        code.push_str("        handler: None,\n");
        for param in &route.params {
            code.push_str(&format!("        {}: {},\n", param.name, param.name));
        }
        code.push_str("        }\n");
        code.push_str("    }\n");
    }
    code.push_str("}\n\n");
}

fn write_node_bind_trait(code: &mut String, node: &OscRoute) {
    println!("Generating Bind trait for node: {}", node.struct_name());
    println!(
        "OscRoute {} with access tags: {:?}",
        node.struct_name(),
        node.access_tags,
    );
    code.push_str(&format!("/// {}\n", node.osc_address));
    code.push_str(&format!(
            "impl Bind<{0}Args> for {1} {{\n    fn bind<F>(&mut self, callback: F)\n    where F: FnMut({0}Args) + 'static {{\n",
            node.struct_name(), node.struct_name()
        ));
    code.push_str("        self.handler = Some(Box::new(callback));\n");
    code.push_str("    }\n}\n\n");
}

fn write_node_set_trait(code: &mut String, node: &OscRoute) {
    code.push_str(&format!("/// {}\n", node.osc_address));
    code.push_str(&format!(
            "impl Set<{0}Args> for {1} {{\n    type Error = OscError;\n    fn set(&mut self, args: {0}Args) -> Result<(), Self::Error> {{\n",
            node.struct_name(), node.struct_name()
        ));
    let re = Regex::new(r"\{[^\}]+\}").unwrap();
    let osc_address_template = re.replace_all(&node.osc_address, "{}");
    code.push_str(&format!(
        "        let osc_address = format!(\"{}\"{});\n",
        osc_address_template,
        node.params
            .iter()
            .map(|param| { format!(", self.{}", param.name) })
            .collect::<String>()
    ));
    code.push_str("        let osc_msg = rosc::OscMessage {\n");
    code.push_str("            addr: osc_address,\n");
    code.push_str("            args: vec![\n");
    node.arguments.iter().for_each(|arg| {
        let arg_name = sanitize_path_level(&arg.name);
        match arg.typ.as_str() {
            "int" => code.push_str(&format!(
                "                rosc::OscType::Int(args.{}) ,\n",
                arg_name
            )),
            "float" => code.push_str(&format!(
                "                rosc::OscType::Float(args.{}) ,\n",
                arg_name
            )),
            "string" => code.push_str(&format!(
                "                rosc::OscType::String(args.{}.clone()) ,\n",
                arg_name
            )),
            "bool" => code.push_str(&format!(
                "                rosc::OscType::Bool(args.{}) ,\n",
                arg_name
            )),
            _ => code.push_str(&format!(
                "                /* Unknown type for {} */\n",
                arg_name
            )),
        }
    });
    code.push_str("            ],\n");
    code.push_str("        };\n");
    code.push_str("        let packet = rosc::OscPacket::Message(osc_msg);\n");
    code.push_str("        let buf = rosc::encoder::encode(&packet).map_err(|_| OscError)?;\n");
    code.push_str("        self.socket.send(&buf).map_err(|_| OscError)?;\n");
    code.push_str("        Ok(())\n");
    code.push_str("    }\n}\n\n");
}

fn write_node_query_trait(code: &mut String, node: &OscRoute) {
    code.push_str(&format!("/// {}\n", node.osc_address));
    code.push_str(&format!(
            "impl Query for {0} {{\n    type Error = OscError;\n    fn query(&self) -> Result<(), Self::Error> {{\n",
            node.struct_name()
        ));
    let re = Regex::new(r"\{[^\}]+\}").unwrap();
    let osc_address_template = re.replace_all(&node.osc_address, "{}");
    code.push_str(&format!(
        "        let osc_address = format!(\"{}\"{});\n",
        osc_address_template,
        node.params
            .iter()
            .map(|param| { format!(", self.{}", param.name) })
            .collect::<String>()
    ));
    code.push_str("        let osc_msg = rosc::OscMessage {\n");
    code.push_str("            addr: osc_address,\n");
    code.push_str("            args: vec![],\n");
    code.push_str("        };\n");
    code.push_str("        let packet = rosc::OscPacket::Message(osc_msg);\n");
    code.push_str("        let buf = rosc::encoder::encode(&packet).map_err(|_| OscError)?;\n");
    code.push_str("        self.socket.send(&buf).map_err(|_| OscError)?;\n");
    code.push_str("        Ok(())\n");
    code.push_str("    }\n}\n\n");
}

fn write_node(code: &mut String, node: &OscRoute, generated_structs: &mut HashSet<String>) {
    if generated_structs.contains(&node.struct_name()) {
        return;
    }
    generated_structs.insert(node.struct_name().clone());
    // Generate Args struct and Handler type if needed
    let endpoint_args_struct = format!("{}Args", node.struct_name());
    if !generated_structs.contains(&endpoint_args_struct) {
        code.push_str("#[derive(Debug)]\n");
        code.push_str(&format!("pub struct {} {{\n", endpoint_args_struct));
        for arg in &node.arguments {
            code.push_str(&format!(
                "    pub {}: {}, // {}\n",
                sanitize_path_level(&arg.name),
                rust_type(&arg.typ),
                arg.description.as_deref().unwrap_or("")
            ));
        }
        code.push_str("}\n\n");
        generated_structs.insert(endpoint_args_struct.clone());
    }

    write_node_struct_definition(code, node);

    println!(
        "OscRoute {} is leaf with access tags: {:?}",
        node.struct_name(),
        node.access_tags,
    );
    if node.access_tags.contains(&AccessTag::Writeable) {
        write_node_set_trait(code, node);
    }
    if node.access_tags.contains(&AccessTag::Readable) {
        write_node_bind_trait(code, node);
    }
    if node.access_tags.contains(&AccessTag::Queryable) {
        write_node_query_trait(code, node);
    }
}

fn write_reaper(code: &mut String, routes: Vec<OscRoute>) {
    code.push_str("pub struct Reaper {\n");
    code.push_str("    socket: Arc<UdpSocket>,\n");
    code.push_str("}\n\n");
    code.push_str("impl Reaper {\n");
    code.push_str("    pub fn new(socket: Arc<UdpSocket>) -> Self {\n");
    code.push_str("        Self {\n");
    code.push_str("            socket,\n");
    code.push_str("        }\n");
    code.push_str("    }\n");
    // for route in routes.iter() {
    //     code.push_str(&format!(
    //         "    pub fn {}(&self",
    //         route.struct_name().to_lowercase(),
    //     ));
    //     for param in &route.params {
    //         code.push_str(&format!(", {}: {}", param.name, rust_type(&param.typ)));
    //     }
    //     code.push_str(&format!(") -> {} {{\n", route.struct_name()));
    //     code.push_str("        ");
    //     code.push_str(&format!("{} {{\n", route.struct_name()));
    //     code.push_str("            socket: self.socket.clone(),\n");
    //     for param in &route.params {
    //         code.push_str(&format!("            {}: {},\n", param.name, param.name));
    //     }
    //     code.push_str("        }\n");
    //     code.push_str("    }\n");
    // }
    code.push_str("}\n\n");

    write_node_accessors(code, routes);
}

fn write_dispatcher(code: &mut String, routes: Vec<OscRoute>) {
    code.push_str("/// Try to match an OSC address against a pattern, extracting arguments.\n");
    code.push_str("/// E.g. addr: \"/track/abc123/pan\", pattern: \"/track/{}/pan\" -> Some(vec![\"abc123\"])\n");
    code.push_str("fn match_addr(addr: &str, pattern: &str) -> Option<Vec<String>> {\n");
    code.push_str(
        "    let addr_parts: Vec<&str> = addr.split('/').filter(|s| !s.is_empty()).collect();\n",
    );
    code.push_str(
        "    let pat_parts: Vec<&str> = pattern.split('/').filter(|s| !s.is_empty()).collect();\n",
    );
    code.push_str("    if addr_parts.len() != pat_parts.len() {\n");
    code.push_str("        return None;\n");
    code.push_str("    }\n");
    code.push_str("    let mut args = Vec::new();\n");
    code.push_str("    for (a, p) in addr_parts.iter().zip(pat_parts.iter()) {\n");
    code.push_str("        if *p == \"{}\" {\n");
    code.push_str("            args.push((*a).to_string());\n");
    code.push_str("        } else if *p != *a {\n");
    code.push_str("            return None;\n");
    code.push_str("        }\n");
    code.push_str("    }\n");
    code.push_str("    Some(args)\n");
    code.push_str("}\n\n");
    code.push_str("pub fn dispatch_osc<F>(reaper: &mut Reaper, msg: rosc::OscMessage, log_unknown: F)\nwhere F: Fn(&str) {\n");
    code.push_str("    let addr = msg.addr.as_str();\n");

    // Emit match arms for each endpoint
    for node in routes.iter() {
        // Begin arm
        code.push_str(&format!(
            "    if let Some(args) = match_addr(addr, \"{}\") {{\n",
            &node.osc_address,
        ));

        // Extract path args
        for (i, param) in node.params.iter().rev().enumerate() {
            match param.typ.as_str() {
                "int" => {
                    code.push_str(&format!(
                        "        let {}: i32 = args[{}].parse().unwrap();\n",
                        param.name, i
                    ));
                }
                "float" => {
                    code.push_str(&format!(
                        "        let {}: f32 = args[{}].parse().unwrap();\n",
                        param.name, i
                    ));
                }
                "bool" => {
                    code.push_str(&format!(
                        "        let {}: bool = args[{}] == \"true\";\n",
                        param.name, i
                    ));
                }
                "string" => {
                    code.push_str(&format!(
                        "        let {} = args[{}].clone();\n",
                        param.name, i
                    ));
                }
                _ => {
                    panic!(
                        "Unsupported path argument type '{}' in node {:?}",
                        param.typ, node
                    );
                }
            }
        }

        code.push_str(&format!(
            "        let mut endpoint = reaper.{}(",
            node.accessor_name(),
        ));
        if !node.params.is_empty() {
            for param in &node.params {
                code.push_str(&format!("{}, ", param.name));
            }
        }
        code.push_str(");\n");

        // Handler check
        code.push_str("        if let Some(handler) = &mut endpoint.handler {\n");

        // OSC arg decoding
        for (j, osc_arg) in node.clone().arguments.iter().enumerate() {
            code.push_str(&format!(
                "            if let Some({}) = msg.args.get({}) {{\n",
                osc_arg.name, j
            ));
            match osc_arg.typ.as_str() {
                "int" => {
                    code.push_str(&format!(
                        "                handler({}Args {{ {}: {}.clone().int().unwrap()}});\n",
                        node.struct_name(),
                        osc_arg.name,
                        osc_arg.name
                    ));
                }
                "float" => {
                    code.push_str(&format!(
                        "                handler({}Args {{ {}: {}.clone().float().unwrap()}});\n",
                        node.struct_name(),
                        osc_arg.name,
                        osc_arg.name
                    ));
                }
                "bool" => {
                    code.push_str(&format!(
                        "                handler({}Args {{ {}: {}.clone().bool().unwrap()}});\n",
                        node.struct_name(),
                        osc_arg.name,
                        osc_arg.name
                    ));
                }
                "string" => {
                    code.push_str(&format!(
                        "                handler({}Args {{ {}: {}.clone().string().unwrap().clone()}});\n",
                        node.struct_name(), osc_arg.name, osc_arg.name
                    ));
                }
                _ => {
                    code.push_str(&format!(
                        "                // Unsupported arg type: {}\n",
                        osc_arg.typ
                    ));
                }
            }
            code.push_str("                }\n");
        }
        code.push_str("            }\n        return;\n    }\n");
    }

    // Unknown fallback
    code.push_str("    log_unknown(addr);\n}\n");

    // Add match_addr helper here
}

fn format_code(code: &str) -> String {
    let mut rustfmt = Command::new("rustfmt")
        .arg("stdout")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to start rustfmt");

    let stdin = rustfmt.stdin.as_mut().expect("Failed to open stdin");
    use std::io::Write;
    stdin
        .write_all(code.as_bytes())
        .expect("Failed to write to rustfmt stdin");

    let output = rustfmt
        .wait_with_output()
        .expect("Failed to read rustfmt output");
    String::from_utf8(output.stdout).expect("rustfmt output not valid UTF-8")
}

fn main() {
    let cli = Cli::parse();
    let yaml = fs::read_to_string(&cli.spec).expect("Failed to read input YAML");
    let routes: Vec<OscRoute> = serde_yaml::from_str(&yaml).expect("Failed to parse YAML");

    let mut code = String::new();
    write_imports(&mut code);
    for route in &routes {
        let mut generated_structs = HashSet::new();
        write_node(&mut code, route, &mut generated_structs);
    }
    write_context_struct_types(&mut code, &routes);
    write_reaper(&mut code, routes.clone());
    write_dispatcher(&mut code, routes);

    let formatted_code = match std::panic::catch_unwind(|| format_code(&code)) {
        Ok(formatted) => {
            if formatted.trim().is_empty() {
                // rustfmt output was empty, fallback to unformatted
                &code
            } else {
                &formatted.clone()
            }
        }
        Err(_) => &code,
    };
    fs::write(&cli.out, formatted_code).expect("Failed to write output Rust file");
}

#[cfg(test)]
mod test_osc_address_template_to_regex {
    use super::*;

    #[test]
    fn test_track_index() {
        let regex_str = osc_address_template_to_regex("/track/{track_guid}/index");
        let re = regex::Regex::new(&regex_str).unwrap();
        let caps = re.captures("/track/1234/index").unwrap();
        assert_eq!(&caps[1], "1234");
    }

    #[test]
    fn test_track_selected() {
        let regex_str = osc_address_template_to_regex("/track/{track_guid}/selected");
        let re = regex::Regex::new(&regex_str).unwrap();
        let caps = re.captures("/track/abcd/selected").unwrap();
        assert_eq!(&caps[1], "abcd");
    }

    #[test]
    fn test_track_send_guid() {
        let regex_str = osc_address_template_to_regex("/track/{track_guid}/send/{send_index}/guid");
        let re = regex::Regex::new(&regex_str).unwrap();
        let caps = re.captures("/track/abcd/send/5/guid").unwrap();
        assert_eq!(&caps[1], "abcd");
        assert_eq!(&caps[2], "5");
    }

    #[test]
    fn test_track_send_volume() {
        let regex_str =
            osc_address_template_to_regex("/track/{track_guid}/send/{send_index}/volume");
        let re = regex::Regex::new(&regex_str).unwrap();
        let caps = re.captures("/track/abcd/send/3/volume").unwrap();
        assert_eq!(&caps[1], "abcd");
        assert_eq!(&caps[2], "3");
    }
}

#[cfg(test)]
mod test_build_context_name {
    use super::*;

    #[test]
    fn test_track_index() {
        assert_eq!(build_context_name("/track/{track_guid}/index"), "Track");
    }

    #[test]
    fn test_track_selected() {
        assert_eq!(build_context_name("/track/{track_guid}/selected"), "Track");
    }

    #[test]
    fn test_track_send_guid() {
        assert_eq!(
            build_context_name("/track/{track_guid}/send/{send_index}/guid"),
            "TrackSend"
        );
    }

    #[test]
    fn test_track_send_volume() {
        assert_eq!(
            build_context_name("/track/{track_guid}/send/{send_index}/volume"),
            "TrackSend"
        );
    }

    #[test]
    fn test_nested_example() {
        assert_eq!(
            build_context_name("/track/{track_guid}/fx/{fx_guid}/param/{param_guid}/value"),
            "TrackFxParam"
        );
    }

    #[test]
    fn test_single_path() {
        assert_eq!(
            build_context_name("/project/{project_guid}/name"),
            "Project"
        );
    }
}
