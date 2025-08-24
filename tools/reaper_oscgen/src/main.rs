use clap::Parser;
use regex::Regex;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
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
    let s = s.replace("-", "_");
    let s = s.replace(" ", "_");
    let s = s.replace(".", "_");
    let s = s.replace("/", "_");
    let s = s.replace("?", "_");
    let s = s.replace("$", "_");
    s
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
#[derive(Debug)]
struct TreeNode {
    name: String,                       // e.g., "track", "index"
    struct_name: String, // unique name for this node that reflects the full path to get to it e.g. "TrackIndex"
    parent_args: Vec<(String, String)>, // (arg_name, arg_type) pairs from parent nodes, used to
    // initialize structs in the fluent API
    accessor_name: Option<String>, // name of the method that accesses this node, e.g. "track_mut"
    arg_name: Option<String>,      // e.g., "track_guid"
    children: HashMap<String, TreeNode>, // next level down
    leaf: Option<LeafInfo>,
}

// Helper to convert "/track/{track_guid}/pan" -> "/track/{}/pan"
fn to_pattern(address: &str) -> String {
    let re = regex::Regex::new(r"\{[^{}]+\}").unwrap();
    re.replace_all(address, "{}").to_string()
}

// Helper to extract ["track_guid"] from "/track/{track_guid}/pan"
fn extract_path_args(address: &str) -> Vec<String> {
    let mut args = Vec::new();
    let re = regex::Regex::new(r"\{([^{}]+)\}").unwrap();
    for cap in re.captures_iter(address) {
        args.push(cap[1].to_string());
    }
    args
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

/// Build hierarchy tree from all routes
fn build_tree(routes: &[OscRoute]) -> TreeNode {
    let mut root = TreeNode {
        name: "Reaper".to_string(),
        struct_name: "Reaper".to_string(),
        parent_args: Vec::new(),
        accessor_name: None,
        arg_name: None,
        children: HashMap::new(),
        leaf: None,
    };

    let re = Regex::new(r"\{([^\}]+)\}").unwrap();
    for route in routes {
        let mut path = Vec::new(); // Reset path for each route
        let parsed = parse_address(&route.osc_address);
        let mut node = &mut root;
        let mut parent_args = Vec::new();
        for (name, arg_name) in &parsed {
            if let Some(arg) = arg_name {
                parent_args.push((sanitize_path_level(arg), "String".to_string()));
            }
            path.push((name.as_ref(), arg_name.as_deref()));
            let key = format!(
                "{}{}",
                name,
                arg_name
                    .as_ref()
                    .map_or(String::new(), |a| format!("${}", a))
            );
            let method_name = sanitize_path_level(name);

            node = node.children.entry(key.clone()).or_insert(TreeNode {
                name: name.clone(),
                struct_name: full_path_struct_name(path.as_slice()),
                parent_args: parent_args.clone(),
                accessor_name: Some(method_name),
                arg_name: arg_name.clone(),
                children: HashMap::new(),
                leaf: None,
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

    for (arg, typ) in &node.parent_args {
        code.push_str(&format!("    pub {}: {},\n", arg, typ));
    }

    for child in node.children.values() {
        if let Some(arg_name) = &child.arg_name {
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
    for (arg, typ) in &node.parent_args {
        code.push_str(&format!(", {}: {}", arg, typ));
    }
    code.push_str(&format!(") -> {} {{\n", node.struct_name));
    code.push_str(&format!("        {} {{\n", node.struct_name));
    code.push_str("            socket,\n");
    if node.leaf.is_some() {
        code.push_str("            handler: None,\n");
    }
    for (arg, _) in &node.parent_args {
        code.push_str(&format!("            {}: {}.clone(),\n", arg, arg));
    }
    for child in node.children.values() {
        if let Some(arg_name) = &child.arg_name {
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
        let method_name = if child.name.is_empty() {
            if let Some(arg_name) = &child.arg_name {
                sanitize_path_level(arg_name)
            } else {
                panic!("Anonymous node without arg_name: {:#?}", child);
            }
        } else {
            sanitize_path_level(&child.name)
        };

        if let Some(arg_name) = &child.arg_name {
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
            for (parent_arg, _) in &node.parent_args {
                code.push_str(&format!("self.{}.clone(), ", parent_arg));
            }
            code.push_str(&format!("{0}.clone()))\n", sanitize_path_level(arg_name)));
            code.push_str("    }\n");
        } else {
            code.push_str(&format!(
                "    pub fn {0}(&self) -> {1} {{\n        {1}::new(self.socket.clone(), ",
                method_name, child.struct_name
            ));
            for (parent_arg, _) in &node.parent_args {
                code.push_str(&format!("            self.{}.clone(),\n", parent_arg));
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
            node.parent_args
                .iter()
                .map(|arg| format!(", self.{}", arg.0))
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
            node.parent_args
                .iter()
                .map(|arg| format!(", self.{}", arg.0))
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

#[derive(Clone)]
pub struct PathStep {
    /// The accessor method name, e.g. "track_mut"
    pub accessor: String,
    /// The argument name for this accessor (None for leaf accessor)
    pub arg_name: Option<String>,
    /// The struct type at this step, e.g. "Track"
    pub struct_name: String,
    pub is_mut: bool,
}

pub struct EndpointMeta {
    pub osc_pattern: String,
    pub path_args: Vec<String>,
    pub osc_args: Vec<(String, String)>,
    pub struct_name: String,
    pub args_struct_name: String,
    pub path_chain: Vec<PathStep>,
}

impl TreeNode {
    pub fn iter_endpoints(&self) -> Vec<EndpointMeta> {
        let mut endpoints = Vec::new();
        self.collect_endpoints(&mut endpoints, Vec::new());
        endpoints
    }

    fn collect_endpoints(&self, endpoints: &mut Vec<EndpointMeta>, mut chain: Vec<PathStep>) {
        // If this node is not the root, add its accessor to the chain
        // println!("\tnode: {:?}\n", self);
        if let Some(accessor_name) = &self.accessor_name {
            chain.push(PathStep {
                accessor: accessor_name.clone(),
                arg_name: self.arg_name.clone(),
                struct_name: self.struct_name.clone(),
                is_mut: true, // TODO
            });
        }
        if let Some(leaf) = &self.leaf {
            // Determine path args by finding {name} in osc_address
            let path_args = extract_path_args(&leaf.osc_address);
            let osc_args = leaf
                .args
                .iter()
                .map(|a| (a.name.clone(), a.typ.clone()))
                .collect();
            endpoints.push(EndpointMeta {
                osc_pattern: to_pattern(&leaf.osc_address), // e.g. "/track/{}/pan"
                path_args,
                osc_args,
                struct_name: self.struct_name.clone(),
                args_struct_name: format!("{}Args", self.struct_name),
                path_chain: chain.clone(),
            });
        }
        for child in self.children.values() {
            child.collect_endpoints(endpoints, chain.clone());
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
    code.push_str("pub fn dispatch_osc<F>(reaper: &mut Reaper, packet: rosc::OscPacket, log_unknown: F)\nwhere F: Fn(&str) {\n");
    code.push_str(
        "    let msg = match packet { rosc::OscPacket::Message(msg) => msg, _ => return, };\n",
    );
    code.push_str("    let addr = msg.addr.as_str();\n");

    // Emit match arms for each endpoint
    for endpoint in api_tree.iter_endpoints() {
        let osc_addr_pattern = endpoint.osc_pattern; // e.g. "/track/{}/pan"
        let path_arg_names = endpoint.path_args; // e.g. vec!["track_guid"]
        let osc_arg_types = endpoint.osc_args; // e.g. vec![("pan", "Float")]

        // Begin arm
        code.push_str(&format!(
            "    if let Some(args) = match_addr(addr, \"{}\") {{\n",
            osc_addr_pattern
        ));

        // Extract path args
        for (i, name) in path_arg_names.iter().enumerate() {
            code.push_str(&format!("        let {} = &args[{}];\n", name, i));
        }

        let mut cursor = "reaper".to_string();
        for path_step in &endpoint.path_chain[..endpoint.path_chain.len() - 1] {
            if path_step.is_mut {
                code.push_str(&format!(
                    "        let {} = {}.{}({}.clone());\n",
                    path_step.accessor.trim_end_matches("_mut"),
                    cursor,
                    path_step.accessor,
                    path_step.arg_name.clone().unwrap(),
                ));
                cursor = path_step.accessor.trim_end_matches("_mut").to_string();
            } else {
                // code.push_str(&format!(
                //     "        let {} = {}.{}();\n",
                //     path_step.accessor, cursor, path_step.accessor
                // ));
                // cursor = path_step.accessor.to_string();
            }
        }
        // Last accessor is the endpoint
        let last = endpoint.path_chain.last().unwrap();
        code.push_str(&format!(
            "        let mut endpoint = {}.{}();\n",
            cursor, last.accessor
        ));

        // Handler check
        code.push_str("        if let Some(handler) = &mut endpoint.handler {\n");

        // OSC arg decoding
        for (j, (osc_arg, osc_type)) in osc_arg_types.iter().enumerate() {
            let rust_type = match osc_type.as_str() {
                "Float" => "f32",
                "Int" => "i32",
                "String" => "String",
                "Bool" => "bool",
                _ => "UNKNOWN",
            };
            code.push_str(&format!(
                "            if let Some({}) = msg.args.get({}) {{\n",
                osc_arg, j
            ));
            match osc_type.as_str() {
                "int" => {
                    code.push_str(&format!(
                        "                handler({}Args {{ {}: {}.clone().int().unwrap() as i32 }});\n",
                        endpoint.struct_name, osc_arg, osc_arg
                    ));
                }
                "float" => {
                    code.push_str(&format!(
                        "                handler({}Args {{ {}: {}.clone().float().unwrap() as f32 }});\n",
                        endpoint.struct_name, osc_arg, osc_arg
                    ));
                }
                "bool" => {
                    code.push_str(&format!(
                        "                handler({}Args {{ {}: {}.clone().bool().unwrap() }});\n",
                        endpoint.struct_name, osc_arg, osc_arg
                    ));
                }
                "string" => {
                    code.push_str(&format!(
                        "                handler({}Args {{ {}: {}.clone().string().unwrap().clone() }});\n",
                        endpoint.struct_name, osc_arg, osc_arg
                    ));
                }
                _ => {
                    code.push_str(&format!(
                        "                // Unsupported arg type: {}\n",
                        osc_type
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

/// Generate the code from the tree
fn generate_code(root: &TreeNode) -> String {
    let mut code = String::new();
    code.push_str("// AUTO-GENERATED CODE. DO NOT EDIT!\n\n");
    code.push_str("use std::net::UdpSocket;\n");
    code.push_str("use std::collections::HashMap;\n");
    code.push_str("use std::sync::Arc;\n\n");

    code.push_str("#[derive(Debug)]\npub struct OscError;\n\n");
    code.push_str("use crate::traits::{Bind, Set, Query};\n\n");

    write_node(&mut code, root, &mut HashSet::new());

    write_dispatcher(&mut code, root);

    code
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
    let code = generate_code(&tree);

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
