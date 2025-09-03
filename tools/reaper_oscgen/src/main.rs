use clap::Parser;
use regex::Regex;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::fmt::Write;
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
fn pascal_case(s: &str) -> String {
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

// OSC argument as represented in the YAML
#[derive(Debug, Deserialize, Clone)]
struct OscArgument {
    name: String,
    #[serde(rename = "type")]
    typ: String,
    description: Option<String>,
}

// OSC route as represented in the YAML
#[derive(Debug, Deserialize, Clone)]
struct OscRoute {
    osc_address: String,
    arguments: Vec<OscArgument>,
    direction: Option<String>,
}

/// Info for leaf endpoints
#[derive(Debug, Clone)]
struct LeafInfo {
    args: Vec<OscArgument>,
    osc_address: String,
    direction: Option<String>,
}

/// A node in the OSC hierarchy tree
#[derive(Debug, Clone)]
struct TreeNode {
    // how to access this node in the fluent API
    accessor_name: String,
    // type name in the generated source
    // NOTE: must represent its whole hierarchy to avoid name
    // collisions (e.g. "Pan" is not ennough because we may have both TrackPan" vs "TrackSendPan")
    struct_name: String,
    path_arg: Option<String>,            // e.g., "track_guid"
    children: HashMap<String, TreeNode>, // next level down
    leaf: Option<LeafInfo>,
    parents: Vec<PathStep>, // For convenience since linked lists are hard in Rust
}

#[derive(Debug, Clone)]
pub struct ParentArg {
    pub name: String,
    pub typ: String,
}

#[derive(Debug, Clone)]
pub struct PathStep {
    /// The accessor method name, e.g. "track_mut"
    pub accessor: String,
    /// The argument name for this accessor (None for leaf accessor)
    pub arg: Option<ParentArg>,
    /// The struct type at this step, e.g. "Track"
    pub struct_name: String,
}

/// Parse a single OSC address into a vector of (name, Option<arg_name>) pairs
/// Example: "/track/{track_guid}/index" => [("track", Some("track_guid")), ("index", None)]
fn parse_address(address: &str) -> Vec<(String, Option<String>)> {
    address
        .split('/')
        .filter(|s| !s.is_empty())
        .fold(Vec::new(), |mut acc, part| {
            if part.starts_with('{') && part.ends_with('}') {
                let arg = part.trim_matches(|c| c == '{' || c == '}').to_string();
                if let Some(last) = acc.pop() {
                    acc.push((last.0, Some(arg)));
                }
            } else {
                acc.push((part.to_string(), None));
            }
            acc
        })
}

#[derive(Debug)]
struct ContextParam {
    name: String,
    typ: String,
}

fn write_imports(code: &mut String) {
    code.push_str("// AUTO-GENERATED CODE. DO NOT EDIT!\n\n");
    code.push_str("use std::net::UdpSocket;\n");
    code.push_str("use std::collections::HashMap;\n");
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
            .arguments
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

/// Build hierarchy tree from all routes
fn build_tree(routes: &[OscRoute]) -> TreeNode {
    let mut root = TreeNode {
        accessor_name: "Reaper".to_string(),
        struct_name: "Reaper".to_string(),
        path_arg: None,
        children: HashMap::new(),
        leaf: None,
        parents: Vec::new(),
    };

    let re = Regex::new(r"\{([^\}]+)\}").unwrap();
    for route in routes {
        let mut path = Vec::new(); // Reset path for each route
        let parsed = parse_address(&route.osc_address);
        let mut node = &mut root;
        let mut parents = Vec::new();
        for (name, arg_name) in &parsed {
            let struct_name = full_path_struct_name(path.as_slice());
            parents.push(PathStep {
                accessor: name.clone(),
                arg: arg_name.clone().map(|a| ParentArg {
                    name: sanitize_path_level(&a),
                    typ: "String".to_string(),
                }),
                struct_name,
            });
            path.push((name.as_ref(), arg_name.as_deref()));
            let key = format!(
                "{}{}",
                name,
                arg_name
                    .as_ref()
                    .map_or(String::new(), |a| format!("${}", a))
            );

            node = node.children.entry(key.clone()).or_insert(TreeNode {
                accessor_name: sanitize_path_level(&name.clone()),
                struct_name: full_path_struct_name(path.as_slice()),
                path_arg: arg_name.clone(),
                children: HashMap::new(),
                leaf: None,
                parents: parents.clone(),
            });
        }

        // Get path arg names from address: e.g. "/track/{track_guid}/pan"
        let path_arg_names: std::collections::HashSet<_> = re
            .captures_iter(&route.osc_address)
            .map(|cap| cap[1].to_string())
            .collect();

        // Filter arguments: only those NOT in path_arg_names
        let endpoint_args: Vec<OscArgument> = route
            .arguments
            .iter()
            .filter(|arg| !path_arg_names.contains(&arg.name))
            .cloned()
            .collect();

        node.leaf = Some(LeafInfo {
            args: endpoint_args,
            osc_address: route.osc_address.clone(),
            direction: route.direction.clone(),
        });
    }
    root
}

/// Generate full-path struct name from hierarchy
fn full_path_struct_name(path: &[(&str, Option<&str>)]) -> String {
    let mut parts = Vec::new();
    for (seg, arg) in path {
        if !seg.is_empty() {
            parts.push(sanitize_path_level(seg));
        }
        // Don't include argument names in struct name unless segment is empty (anonymous node)
        if seg.is_empty() {
            if let Some(a) = arg {
                parts.push(sanitize_path_level(a));
            }
        }
    }
    if parts.is_empty() {
        "Root".to_string()
    } else {
        pascal_case(&parts.join("_"))
    }
}

fn write_node_struct_definition(code: &mut String, node: &TreeNode) {
    code.push_str(&format!("pub struct {} {{\n", node.struct_name));
    code.push_str("    socket: Arc<UdpSocket>,\n");

    if node.leaf.is_some() {
        code.push_str(&format!(
            "    handler: Option<{0}Handler>,\n",
            node.struct_name
        ));
    }

    for parent in &node.parents {
        if let Some(arg) = &parent.arg {
            code.push_str(&format!("    pub {}: {},\n", arg.name, arg.typ));
        }
    }

    for child in node.children.values() {
        if let Some(arg_name) = &child.path_arg {
            code.push_str(&format!(
                "    pub {0}_map: HashMap<String, {1}>,\n",
                sanitize_path_level(arg_name),
                child.struct_name,
            ));
        }
    }
    code.push_str("}\n\n");
}

fn write_node_constructor(code: &mut String, node: &TreeNode) {
    code.push_str(&format!("impl {} {{\n", node.struct_name));
    code.push_str("    pub fn new(socket: Arc<UdpSocket>");
    for parent in &node.parents {
        if let Some(arg) = &parent.arg {
            code.push_str(&format!(", {}: {}", arg.name, arg.typ));
        }
    }
    code.push_str(&format!(") -> {} {{\n", node.struct_name));
    code.push_str(&format!("        {} {{\n", node.struct_name));
    code.push_str("            socket,\n");
    if node.leaf.is_some() {
        code.push_str("            handler: None,\n");
    }
    for parent in &node.parents {
        if let Some(arg) = &parent.arg {
            code.push_str(&format!(
                "            {}: {}.clone(),\n",
                arg.name, arg.name
            ));
        }
    }
    for child in node.children.values() {
        if let Some(arg_name) = &child.path_arg {
            code.push_str(&format!(
                "            {0}_map: HashMap::new(),\n",
                sanitize_path_level(arg_name)
            ));
        }
    }
    code.push_str("        }\n    }\n");
}

fn write_child_fluent_api(code: &mut String, node: &TreeNode) {
    for child in node.children.values() {
        let method_name = if child.accessor_name.is_empty() {
            if let Some(arg_name) = &child.path_arg {
                sanitize_path_level(arg_name)
            } else {
                panic!("Anonymous node without arg_name: {:#?}", child);
            }
        } else {
            sanitize_path_level(&child.accessor_name)
        };

        if let Some(arg_name) = &child.path_arg {
            code.push_str(&format!(
                "    pub fn {0}(&mut self, {1}: String) -> &mut {2} {{\n",
                method_name,
                sanitize_path_level(arg_name),
                child.struct_name,
            ));
            code.push_str(&format!(
                "        self.{0}_map.entry({1}.clone()).or_insert_with(|| {2}::new(self.socket.clone(), ",
                sanitize_path_level(arg_name), sanitize_path_level(arg_name), child.struct_name
            ));
            for parent in &node.parents {
                if let Some(arg) = &parent.arg {
                    code.push_str(&format!("self.{}.clone(), ", arg.name));
                }
            }
            code.push_str(&format!("{0}.clone()))\n", sanitize_path_level(arg_name)));
            code.push_str("    }\n");
        } else {
            code.push_str(&format!(
                "    pub fn {0}(&self) -> {1} {{\n        {1}::new(self.socket.clone(), ",
                method_name, child.struct_name
            ));
            for parent in &node.parents {
                if let Some(arg) = &parent.arg {
                    code.push_str(&format!("self.{}.clone(), ", arg.name));
                }
            }

            code.push_str("     )\n");
            code.push_str("        }\n");
        }
    }
    code.push_str("}\n\n");
}

fn write_node_bind_trait(code: &mut String, node: &TreeNode) {
    if let Some(leaf) = &node.leaf {
        code.push_str(&format!("/// {}\n", leaf.osc_address));
        code.push_str(&format!(
            "impl Bind<{0}Args> for {1} {{\n    fn bind<F>(&mut self, callback: F)\n    where F: FnMut({0}Args) + 'static {{\n",
            node.struct_name, node.struct_name
        ));
        code.push_str("        self.handler = Some(Box::new(callback));\n");
        code.push_str("    }\n}\n\n");
    }
}

fn write_node_set_trait(code: &mut String, node: &TreeNode) {
    if let Some(leaf) = &node.leaf {
        code.push_str(&format!("/// {}\n", leaf.osc_address));
        code.push_str(&format!(
            "impl Set<{0}Args> for {1} {{\n    type Error = OscError;\n    fn set(&mut self, args: {0}Args) -> Result<(), Self::Error> {{\n",
            node.struct_name, node.struct_name
        ));
        let re = Regex::new(r"\{[^\}]+\}").unwrap();
        let osc_address_template = re.replace_all(&leaf.osc_address, "{}");
        code.push_str(&format!(
            "        let osc_address = format!(\"{}\"{});\n",
            osc_address_template,
            node.parents
                .iter()
                .map(|parent| if let Some(arg) = &parent.arg {
                    format!(", self.{}", arg.name)
                } else {
                    String::new() // TODO: suspicious
                })
                .collect::<String>()
        ));
        code.push_str("        let osc_msg = rosc::OscMessage {\n");
        code.push_str("            addr: osc_address,\n");
        code.push_str("            args: vec![\n");
        leaf.args.iter().for_each(|arg| {
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
}

fn write_node_query_trait(code: &mut String, node: &TreeNode) {
    if let Some(leaf) = &node.leaf {
        code.push_str(&format!("/// {}\n", leaf.osc_address));
        code.push_str(&format!(
            "impl Query for {0} {{\n    type Error = OscError;\n    fn query(&self) -> Result<(), Self::Error> {{\n",
            node.struct_name
        ));
        let re = Regex::new(r"\{[^\}]+\}").unwrap();
        let osc_address_template = re.replace_all(&leaf.osc_address, "{}");
        code.push_str(&format!(
            "        let osc_address = format!(\"{}\"{});\n",
            osc_address_template,
            node.parents
                .iter()
                .map(|parent| if let Some(arg) = &parent.arg {
                    format!(", self.{}", arg.name)
                } else {
                    String::new() // TODO: suspicious})
                })
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
}

fn write_node(code: &mut String, node: &TreeNode, generated_structs: &mut HashSet<String>) {
    if generated_structs.contains(&node.struct_name) {
        return;
    }
    generated_structs.insert(node.struct_name.clone());

    write_node_struct_definition(code, node);
    write_node_constructor(code, node);
    write_child_fluent_api(code, node);

    // Generate trait impls if this is a leaf node
    if let Some(leaf) = &node.leaf {
        // Generate Args struct and Handler type if needed
        let endpoint_args_struct = format!("{}Args", node.struct_name);
        if !generated_structs.contains(&endpoint_args_struct) {
            code.push_str("#[derive(Debug)]\n");
            code.push_str(&format!("pub struct {} {{\n", endpoint_args_struct));
            for arg in &leaf.args {
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

        code.push_str(&format!(
            "pub type {0}Handler = Box<dyn FnMut({0}Args) + 'static>;\n\n",
            node.struct_name
        ));

        if leaf.direction.as_deref() != Some("readonly") {
            write_node_set_trait(code, node);
        }
        if leaf.direction.as_deref() != Some("writeonly") {
            write_node_query_trait(code, node);
            write_node_bind_trait(code, node);
        }
    }

    for child in node.children.values() {
        write_node(code, child, generated_structs);
    }
}

impl TreeNode {
    pub fn iter_endpoints(&self) -> Vec<TreeNode> {
        let mut endpoints = Vec::new();
        self.collect_endpoints(&mut endpoints);
        endpoints
    }

    fn collect_endpoints(&self, endpoints: &mut Vec<TreeNode>) {
        if self.leaf.is_some() {
            endpoints.push(self.clone());
        }
        for child in self.children.values() {
            child.collect_endpoints(endpoints);
        }
    }
}

fn write_dispatcher(code: &mut String, api_tree: &TreeNode) {
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
    for node in api_tree.iter_endpoints() {
        // Begin arm
        code.push_str(&format!(
            "    if let Some(args) = match_addr(addr, \"{}\") {{\n",
            &node.leaf.clone().unwrap().osc_address,
        ));

        // Extract path args
        for (i, parent) in node.parents.iter().rev().enumerate() {
            if let Some(arg) = &parent.arg {
                code.push_str(&format!("        let {} = &args[{}];\n", arg.name, i));
            }
        }

        let mut cursor = "reaper".to_string();
        for parent in node.parents.iter() {
            if let Some(arg) = &parent.arg {
                code.push_str(&format!(
                    "        let {} = {}.{}({}.clone());\n",
                    parent.accessor.trim_end_matches("_mut"),
                    cursor,
                    parent.accessor,
                    arg.name,
                ));
                cursor = parent.accessor.trim_end_matches("_mut").to_string();
            }
        }
        // Last accessor is the endpoint
        code.push_str(&format!(
            "        let mut endpoint = {}.{}();\n",
            cursor, node.accessor_name,
        ));

        // Handler check
        code.push_str("        if let Some(handler) = &mut endpoint.handler {\n");

        // OSC arg decoding
        for (j, osc_arg) in node.leaf.clone().unwrap().args.iter().enumerate() {
            code.push_str(&format!(
                "            if let Some({}) = msg.args.get({}) {{\n",
                osc_arg.name, j
            ));
            match osc_arg.typ.as_str() {
                "int" => {
                    code.push_str(&format!(
                        "                handler({}Args {{ {}: {}.clone().int().unwrap()}});\n",
                        node.struct_name, osc_arg.name, osc_arg.name
                    ));
                }
                "float" => {
                    code.push_str(&format!(
                        "                handler({}Args {{ {}: {}.clone().float().unwrap()}});\n",
                        node.struct_name, osc_arg.name, osc_arg.name
                    ));
                }
                "bool" => {
                    code.push_str(&format!(
                        "                handler({}Args {{ {}: {}.clone().bool().unwrap()}});\n",
                        node.struct_name, osc_arg.name, osc_arg.name
                    ));
                }
                "string" => {
                    code.push_str(&format!(
                        "                handler({}Args {{ {}: {}.clone().string().unwrap().clone()}});\n",
                        node.struct_name, osc_arg.name, osc_arg.name
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
        .arg("--emit")
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

    let tree = build_tree(&routes);
    let mut code = String::new();
    write_imports(&mut code);
    write_context_struct_types(&mut code, &routes);
    write_node(&mut code, &tree, &mut HashSet::new());
    write_dispatcher(&mut code, &tree);

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
