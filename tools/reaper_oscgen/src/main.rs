use clap::Parser;
use regex::Regex;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
struct Cli {
    /// Path to the OSC YAML spec file
    spec: PathBuf,
    /// Output Rust file
    #[clap(short, long, default_value = "generated_osc.rs")]
    out: PathBuf,
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
    name: String,                        // e.g., "track", "index"
    arg_name: Option<String>,            // e.g., "track_guid"
    children: HashMap<String, TreeNode>, // next level down
    leaf: Option<LeafInfo>,
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
        arg_name: None,
        children: HashMap::new(),
        leaf: None,
    };

    let re = Regex::new(r"\{([^\}]+)\}").unwrap();
    for route in routes {
        let parsed = parse_address(&route.osc_address);
        let mut node = &mut root;
        for (name, arg_name) in &parsed {
            let key = format!(
                "{}{}",
                name,
                arg_name
                    .as_ref()
                    .map_or(String::new(), |a| format!("${}", a))
            );
            node = node.children.entry(key.clone()).or_insert(TreeNode {
                name: name.clone(),
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

/// Generate the code from the tree
fn generate_code(root: &TreeNode) -> String {
    let mut code = String::new();
    code.push_str("// AUTO-GENERATED CODE. DO NOT EDIT!\n\n");
    code.push_str("use std::net::UdpSocket;\n");
    code.push_str("use std::sync::Arc;\n");

    code.push_str("#[derive(Debug)]\npub struct OscError;\n\n");
    code.push_str("use crate::traits::{Bind, Set, Query};\n\n");

    code.push_str("#[derive(Debug)]\n");
    code.push_str("pub struct Reaper{\n");
    code.push_str("    socket: Arc<UdpSocket>,\n");
    code.push_str("}\n\n");
    code.push_str("impl Reaper {\n");
    code.push_str("    pub fn new(socket: UdpSocket) -> Result<Self, OscError> {\n");
    code.push_str("        Ok(Reaper { socket: Arc::new(socket) })\n");
    code.push_str("    }\n");
    code.push_str("}\n\n");

    let mut generated_structs = HashSet::new();

    // Recurse into children, since we handwrote the root struct above
    for child in root.children.values() {
        write_node(&mut code, child, &[], &mut generated_structs);
    }
    code
}

/// Recursively write struct and impls for each node
fn write_node(
    code: &mut String,
    node: &TreeNode,
    path: &[(&str, Option<&str>)],
    generated_structs: &mut HashSet<String>,
) {
    // Build struct name from path
    let mut current_path = path.to_vec();
    current_path.push((&node.name, node.arg_name.as_ref().map(|s| s.as_str())));
    let struct_name = full_path_struct_name(&current_path);

    // Collect all args in the hierarchy up to this node
    let mut args = Vec::new();
    for (_seg, arg_opt) in &current_path {
        if let Some(arg) = arg_opt {
            // Always string for guids
            args.push((sanitize_path_level(arg), "String".to_string())); // TODO: This seems wrong
        }
    }

    // Avoid duplicate struct generation
    if !generated_structs.contains(&struct_name) {
        code.push_str(&format!("pub struct {} {{\n", struct_name));
        code.push_str("    socket: Arc<UdpSocket>,\n");
        for (arg, typ) in &args {
            code.push_str(&format!("    pub {}: {},\n", arg, typ));
        }
        code.push_str("}\n\n");
        generated_structs.insert(struct_name.clone());
    }

    // Fluent methods for children
    code.push_str(&format!("impl {} {{\n", struct_name));
    for child in node.children.values() {
        let mut child_path = current_path.clone();
        child_path.push((&child.name, child.arg_name.as_ref().map(|s| s.as_str())));
        let child_struct_name = full_path_struct_name(&child_path);

        let method_name = if child.name.is_empty() {
            if let Some(arg_name) = &child.arg_name {
                sanitize_path_level(arg_name)
            } else {
                "unnamed_child".to_string()
            }
        } else {
            sanitize_path_level(&child.name)
        };

        if let Some(arg_name) = &child.arg_name {
            code.push_str(&format!(
                "    pub fn {0}(&self, {1}: String) -> {2} {{\n        {2} {{\n",
                method_name,
                sanitize_path_level(arg_name),
                child_struct_name
            ));
            code.push_str("            socket: self.socket.clone(),\n");
            for (parent_arg, _typ) in &args {
                code.push_str(&format!(
                    "            {}: self.{}.clone(),\n",
                    parent_arg, parent_arg
                ));
            }
            code.push_str(&format!(
                "            {}: {0}.clone(),\n",
                sanitize_path_level(arg_name)
            ));
            code.push_str("        }\n    }\n");
        } else {
            code.push_str(&format!(
                "    pub fn {0}(&self) -> {1} {{\n        {1} {{\n",
                method_name, child_struct_name
            ));
            code.push_str("            socket: self.socket.clone(),\n");
            for (parent_arg, _typ) in &args {
                code.push_str(&format!(
                    "            {}: self.{}.clone(),\n",
                    parent_arg, parent_arg
                ));
            }
            code.push_str("        }\n    }\n");
        }
    }
    code.push_str("}\n\n");

    // If this node is a leaf, implement endpoint traits
    if let Some(leaf) = &node.leaf {
        // Args struct for endpoint
        let endpoint_args_struct = format!("{}Args", struct_name);
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

        // Handler type
        code.push_str(&format!(
            "pub type {0}Handler = Box<dyn FnMut({0}Args) + 'static>;\n\n",
            struct_name
        ));

        // Implement Bind trait
        code.push_str(&format!("/// {}\n", leaf.osc_address));
        code.push_str(&format!(
            "impl Bind<{0}Args> for {1} {{\n    fn bind<F>(&mut self, _callback: F)\n    where F: FnMut({0}Args) + 'static {{\n",
            struct_name, struct_name
        ));
        code.push_str("         // store callback for endpoint\n");
        code.push_str("     }\n}\n\n");

        // Implement Set trait
        code.push_str(&format!("/// {}\n", leaf.osc_address));
        code.push_str(&format!(
            "impl Set<{0}Args> for {1} {{\n    type Error = OscError;\n    fn set(&mut self, args: {0}Args) -> Result<(), Self::Error> {{\n",
            struct_name, struct_name
        ));
        // Construct the OSC address by replacing placeholders with struct fields
        let re = Regex::new(r"\{[^\}]+\}").unwrap();
        let osc_address_template = re.replace_all(&leaf.osc_address, "{}");
        // Only path arguments (from the struct), not endpoint args (from Args struct)
        let path_args = path
            .iter()
            .filter_map(|(_seg, arg_opt)| arg_opt.as_ref())
            .map(|arg| sanitize_path_level(arg))
            .collect::<Vec<_>>();
        code.push_str(&format!(
            "        let osc_address = format!(\"{}\"{});\n",
            osc_address_template,
            path_args
                .iter()
                .map(|arg| format!(", self.{}", arg))
                .collect::<String>()
        ));
        // Build the OSC message args
        code.push_str("        let osc_msg = rosc::OscMessage {\n");
        code.push_str("            addr: osc_address,\n");
        code.push_str("            args: vec![\n");
        // for arg in &leaf.args {
        //     let arg_name = sanitize_path_level(&arg.name);
        //     match arg.typ.as_str() {
        //         "int" => code.push_str(&format!(
        //             "                rosc::OscType::Int(args.{}) ,\n",
        //             arg_name
        //         )),
        //         "float" => code.push_str(&format!(
        //             "                rosc::OscType::Float(args.{}) ,\n",
        //             arg_name
        //         )),
        //         "string" => code.push_str(&format!(
        //             "                rosc::OscType::String(args.{}.clone()) ,\n",
        //             arg_name
        //         )),
        //         "bool" => code.push_str(&format!(
        //             "                rosc::OscType::Bool(args.{}) ,\n",
        //             arg_name
        //         )),
        //         _ => code.push_str(&format!(
        //             "                /* Unknown type for {} */\n",
        //             arg_name
        //         )),
        //     }
        // }
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

        // Implement Query trait
        code.push_str(&format!("/// {}\n", leaf.osc_address));
        code.push_str(&format!(
            "impl Query for {0} {{\n    type Error = OscError;\n    fn query(&self) -> Result<(), Self::Error> {{\n",
            struct_name
        ));
        // Construct the OSC address by replacing placeholders with struct fields
        let re = Regex::new(r"\{[^\}]+\}").unwrap();
        let osc_address_template = re.replace_all(&leaf.osc_address, "{}");
        // Only path arguments (from the struct), not endpoint args (from Args struct)
        let path_args = path
            .iter()
            .filter_map(|(_seg, arg_opt)| arg_opt.as_ref())
            .map(|arg| sanitize_path_level(arg))
            .collect::<Vec<_>>();
        code.push_str(&format!(
            "        let osc_address = format!(\"{}\"{});\n",
            osc_address_template,
            path_args
                .iter()
                .map(|arg| format!(", self.{}", arg))
                .collect::<String>()
        ));
        // Build the OSC message args
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

    // Recurse into children
    for child in node.children.values() {
        write_node(code, child, &current_path, generated_structs);
    }
}

fn main() {
    let cli = Cli::parse();
    let yaml = fs::read_to_string(&cli.spec).expect("Failed to read input YAML");
    let routes: Vec<OscRoute> = serde_yaml::from_str(&yaml).expect("Failed to parse YAML");

    let tree = build_tree(&routes);
    let code = generate_code(&tree);

    fs::write(&cli.out, code).expect("Failed to write output Rust file");
}
