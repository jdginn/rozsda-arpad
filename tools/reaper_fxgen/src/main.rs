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
    /// Path to the fx_dump yaml file
    spec: PathBuf,
    /// Optional path to a text file containing an allow‑list of FX base names to include (one per line, supports comments with #). If not provided, all FX in the YAML will be processed.
    #[clap(short, long)]
    allow_list: Option<PathBuf>,
    /// Output Rust file
    #[clap(short, long, default_value = "generated_fx_param.rs")]
    out: PathBuf,
}

// FX info as represented in the YAML
#[derive(Debug, Deserialize, Clone)]
struct RawFx {
    fx_name: String,
    params: Vec<RawFxParam>,
}

#[derive(Debug, Clone)]
struct Fx {
    fx_name: String,
    repr: String,
    params: Vec<FxParam>,
    plugin_type: PluginType,
    developer: DeveloperName,
}

// FX parameter info as represented in the YAML
#[derive(Debug, Deserialize, Clone)]
struct RawFxParam {
    name: String,
    index: i32,
    min: f32,
    max: f32,
    // TODO: step size?
}

#[derive(Debug, Clone, PartialEq)]
struct FxParam {
    name: String,
    repr: String,
    index: i32,
    min: f32,
    max: f32,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum PluginType {
    VST,
    AU,
    AAX,
    LV2,
    CLAP,
    Container,
    Other(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum DeveloperName {
    Apple,
    Airwindows,
    Cockos,
    Unknown,
}

fn plugin_type_from_prefix(prefix: &str) -> PluginType {
    match prefix {
        "VST" => PluginType::VST,
        "AU" | "AUi" => PluginType::AU, // treat "AUi:" as AU (adjust if you want a distinct variant)
        "AAX" => PluginType::AAX,
        "LV2" => PluginType::LV2,
        "CLAP" => PluginType::CLAP,
        "Container" => PluginType::Container,
        other => PluginType::Other(other.to_string()),
    }
}

fn developer_from_str(s: &str) -> Option<DeveloperName> {
    Some(match s {
        "Apple" => DeveloperName::Apple,
        "Airwindows" => DeveloperName::Airwindows,
        "Cockos" => DeveloperName::Cockos,
        _ => return None,
    })
}

fn split_fx_name(input: &str) -> (String, PluginType, DeveloperName) {
    let s = input.trim();

    // Special-case: exactly "Container"
    if s == "Container" {
        return (
            "Container".to_string(),
            PluginType::Container,
            DeveloperName::Unknown,
        );
    }

    // 1) Prefix / plugin type: look for "<prefix>: <rest>"
    let (plugin_type, rest) = if let Some((prefix, rest)) = s.split_once(':') {
        (plugin_type_from_prefix(prefix.trim()), rest.trim())
    } else {
        (PluginType::Other(String::new()), s) // or pick a default PluginType if you prefer
    };

    // 2) Developer at end: only strip if it's a known developer in final "(...)"
    //    If unknown, leave the parentheses in the name.
    let (name, developer) = if let Some(stripped) = rest.strip_suffix(')') {
        if let Some(open_paren) = stripped.rfind('(') {
            let dev_candidate = stripped[open_paren + 1..].trim();
            if let Some(dev) = developer_from_str(dev_candidate) {
                let name = stripped[..open_paren].trim_end().to_string();
                (name, dev)
            } else {
                (rest.to_string(), DeveloperName::Unknown)
            }
        } else {
            (rest.to_string(), DeveloperName::Unknown)
        }
    } else {
        (rest.to_string(), DeveloperName::Unknown)
    };

    (name, plugin_type, developer)
}

type PluginMap = HashMap<String, HashMap<PluginType, Fx>>;

fn params_equal(p1: Vec<FxParam>, p2: Vec<RawFxParam>) -> bool {
    if p1.len() != p2.len() {
        return false;
    }
    for (param1, param2) in p1.iter().zip(p2.iter()) {
        if param1.name != param2.name
            || param1.index != param2.index
            || (param1.min - param2.min).abs() > f32::EPSILON
            || (param1.max - param2.max).abs() > f32::EPSILON
        {
            return false;
        }
    }
    true
}

fn plugin_type_suffix(pt: &PluginType) -> &'static str {
    match pt {
        PluginType::VST => "VST",
        PluginType::AU => "AU",
        PluginType::AAX => "AAX",
        PluginType::LV2 => "LV2",
        PluginType::CLAP => "CLAP",
        PluginType::Container => "Container",
        PluginType::Other(_) => "Other",
    }
}

fn process_yaml_fx(raw_yaml_fx_list: Vec<RawFx>, allow_names: Option<HashSet<String>>) -> Vec<Fx> {
    let mut plugin_names: HashMap<String, HashMap<PluginType, Fx>> = HashMap::new();

    for raw_fx in &raw_yaml_fx_list {
        let (base_name, plugin_type, developer) = split_fx_name(&raw_fx.fx_name);

        // If an allow‑list is supplied, skip FX not in the list
        if let Some(allowed) = &allow_names {
            if !allowed.contains(&base_name) {
                continue;
            }
        }

        let base_repr = sanitize_enum(&base_name);

        let per_name = plugin_names
            .entry(base_name.clone())
            .or_insert_with(HashMap::new);

        // Do we already have any entry under this base name whose params differ?
        let needs_disambiguation = per_name
            .values()
            .any(|existing| !params_equal(existing.params.clone(), raw_fx.params.clone()));

        // Also: if this exact plugin_type already exists but with different params,
        // disambiguation is definitely needed (and you may want to treat as error).
        if let Some(existing_same_type) = per_name.get(&plugin_type) {
            panic!("Duplicate plugin type for same base name: {} with plugin type {:?} already exists. Consider disambiguating the name or checking for duplicates in the input YAML.", base_name, plugin_type);
        }

        // If disambiguation is needed, rename all existing reprs for this base name
        // to include their plugin type suffix.
        if needs_disambiguation {
            for fx in per_name.values_mut() {
                fx.repr = format!("{}_{}", base_repr, plugin_type_suffix(&fx.plugin_type));
            }
        }

        // Insert/update this (name, plugin_type).
        // If already present, keep the first one (or replace; your choice).
        use std::collections::hash_map::Entry;
        match per_name.entry(plugin_type.clone()) {
            Entry::Vacant(v) => {
                let repr = if needs_disambiguation {
                    format!("{}_{}", base_repr, plugin_type_suffix(&plugin_type))
                } else {
                    base_repr.clone()
                };

                let params = raw_fx
                    .params
                    .iter()
                    .map(|p| FxParam {
                        name: p.name.clone(),
                        repr: sanitize_enum(&p.name),
                        index: p.index,
                        min: p.min,
                        max: p.max,
                    })
                    .collect();

                v.insert(Fx {
                    fx_name: raw_fx.fx_name.clone(),
                    repr,
                    params: params,
                    plugin_type,
                    developer,
                });
            }
            Entry::Occupied(mut o) => {
                // Already have this plugin_type for this name.
                // Decide what you want here:
                // - keep existing
                // - or replace if new has "better" params
                // For now: keep existing.
                let _ = o.get_mut();
            }
        }
    }

    plugin_names
        .into_values()
        .flat_map(|type_map| type_map.into_values())
        .collect()
}

// fn process_yaml_fx(raw_yaml_fx_list: Vec<RawFx>) -> Vec<Fx> {
//     let mut plugin_names = HashMap::new();
//
//     for raw_fx in &raw_yaml_fx_list {
//         let (name, plugin_type, developer) = split_fx_name(&raw_fx.fx_name);
//         let per_name = plugin_names
//             .entry(name.clone())
//             .or_insert_with(HashMap::new);
//
//         // If we have a hit on plugin name, check whether we have differences in params across plugin types. If so, we need to disambiguate by plugin type in the enum variant name (e.g. "ReaEQ_AU" vs "ReaEQ_VST"). If not, we can just use the plugin name as the enum variant name.
//         per_name.entry(plugin_type.clone()).or_insert_with(|| Fx {
//             fx_name: raw_fx.fx_name.clone(),
//             repr: sanitize(&name),
//             params: raw_fx.params.clone(),
//             plugin_type,
//             developer,
//         });
//     }
//     return plugin_names
//         .into_iter()
//         .flat_map(|(_, type_map)| type_map.into_values())
//         .collect();
// }

fn snake_case(s: &str) -> String {
    let re = Regex::new(r"([a-z0-9])([A-Z])").unwrap();
    re.replace_all(s, "$1_$2").to_lowercase()
}

fn sanitize_enum(input: &str) -> String {
    let s = input.trim();

    if s == "-" {
        return "Minus".to_string();
    }
    if s == "+" {
        return "Plus".to_string();
    }

    // Remove anything in parentheses (supports multiple groups).
    // Example: "Foo (Bar) Baz (Qux)" -> "Foo  Baz "
    let mut no_parens = String::with_capacity(s.len());
    let mut depth: usize = 0;
    for ch in s.chars() {
        match ch {
            '(' => depth = depth.saturating_add(1),
            ')' => depth = depth.saturating_sub(1),
            _ if depth == 0 => no_parens.push(ch),
            _ => {}
        }
    }

    // Build PascalCase by splitting on any non-alphanumeric char.
    let mut out = String::new();
    let mut new_word = true;

    for ch in s.chars() {
        if ch.is_ascii_alphanumeric() {
            if new_word {
                out.extend(ch.to_uppercase());
                new_word = false;
            } else {
                // keep digits as-is; letters lowercased for stable casing
                if ch.is_ascii_digit() {
                    out.push(ch);
                } else {
                    out.extend(ch.to_lowercase());
                }
            }
        } else {
            new_word = true;
        }
    }

    // If nothing left after sanitizing, pick a fallback.
    if out.is_empty() {
        out = "Unnamed".to_string();
    }

    // Rust identifiers can't start with a digit. If it does (including "123"),
    // prefix with underscore.
    if out.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        out.insert(0, '_');
    }

    out
}

fn sanitize(name: &str) -> String {
    if name == "-" {
        return "minus".to_string();
    }
    if name == "+" {
        return "plus".to_string();
    }
    let prepend_numeric = Regex::new(r"^(\d)")
        .unwrap()
        .replace_all(name, "_$1")
        .to_string();
    let remove_spaces = Regex::new(r" ")
        .unwrap()
        .replace_all(prepend_numeric.as_str(), "_")
        .to_string();
    let insert_underscores = Regex::new(r"[^a-zA-Z0-9]")
        .unwrap()
        .replace_all(remove_spaces.as_str(), "_")
        .to_lowercase()
        .to_string();
    let remove_au_prefix = Regex::new(r"^au:")
        .unwrap()
        .replace_all(insert_underscores.as_str(), "")
        .to_string();
    remove_au_prefix
}

fn capitalize_first_letter(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

fn write_fx_enum(code: &mut String, effects: Vec<Fx>) {
    writeln!(code, "pub enum FX {{").unwrap();
    for fx in &effects {
        writeln!(code, "    {0}({0}),", fx.repr).unwrap();
    }
    writeln!(code, "}}").unwrap();
}

fn write_fx_struct(code: &mut String, yaml_fx: Fx) {
    writeln!(code, "pub struct {} {{}}\n", yaml_fx.repr).unwrap();
    writeln!(code, "impl {} {{\n", yaml_fx.repr).unwrap();
    write_encode_trackmsg(code, yaml_fx.clone());
    write_decode_trackmsg(code, yaml_fx.clone());
    writeln!(code, "}}").unwrap();
}

fn write_encode_trackmsg(code: &mut String, yaml_fx: Fx) {
    writeln!(
        code,
        "    pub fn encode_trackmsg(track_guid: Uuid, fx_index: i32, param: i32) -> track::TrackMsg {{"
    )
    .unwrap();
    writeln!(code, "        match param(value) {{").unwrap();
    for param in &yaml_fx.params {
        writeln!(
            code,
            "            Param::{}(value) => track::FXParamValue{{",
            &param.repr
        )
        .unwrap();
        writeln!(code, "                track_guid,").unwrap();
        writeln!(code, "                fx_index,").unwrap();
        writeln!(code, "                param_index: {},", param.index).unwrap();
        writeln!(code, "                value,").unwrap();
        writeln!(code, "            }}").unwrap();
        writeln!(code, "            .into(),").unwrap();
    }
    writeln!(code, "        }}").unwrap();
    writeln!(code, "    }}").unwrap();
}

fn write_decode_trackmsg(code: &mut String, yaml_fx: Fx) {
    writeln!(
        code,
        "    pub fn decode_trackmsg(msg: track::FXParamValue) -> Option<Param> {{"
    )
    .unwrap();
    writeln!(code, "        match msg.param_index {{").unwrap();
    for param in &yaml_fx.params {
        writeln!(
            code,
            "            {} => Some(Param::{}(msg.value)),",
            param.index, param.repr
        )
        .unwrap();
    }
    writeln!(code, "            _ => None,").unwrap();
    writeln!(code, "        }}").unwrap();
    writeln!(code, "    }}").unwrap();
}

fn write_imports(code: &mut String) {
    writeln!(code, "use crate::track::track;").unwrap();
    writeln!(code, "use uuid::Uuid;").unwrap();
}

// fn write_fx_enum(code: &mut String, fx_list: &[YamlFx]) {
//     writeln!(code, "pub enum FxType {{").unwrap();
//     for fx in fx_list {
//         writeln!(code, "    {},", fx.fx_name).unwrap();
//     }
//     writeln!(code, "}}").unwrap();
//     writeln!(code, "impl FxType {{").unwrap();
//     writeln!(code, "    fn_name(&self) -> &str {{").unwrap();
//     writeln!(code, "        match self {{").unwrap();
//     for fx in fx_list {
//         writeln!(
//             code,
//             "            FxType::{} => \"{}\",",
//             fx.fx_name, fx.fx_name
//         )
//         .unwrap();
//     }
//     writeln!(code, "        }}").unwrap();
//     writeln!(code, "    }}").unwrap();
//     writeln!(code, "}}").unwrap();
// }
//
// fn write_fx_struct(code: &mut String, fx: &YamlFx) {
//     writeln!(code, "pub struct {} {{", fx.fx_name).unwrap();
//     writeln!(code, "    fx_name: i32,").unwrap();
//     writeln!(code, "}}").unwrap();
// }
//
// fn write_fx_param_struct(code: &mut String, param: &YamlFxParam) {
//     writeln!(code, "pub struct {} {{", param.name).unwrap();
//     writeln!(code, "    value: f32,").unwrap();
//     writeln!(code, "}}").unwrap();
// }
//
fn format_code(code: &str) -> String {
    let mut rustfmt = Command::new("rustfmt")
        .arg("--emit")
        .arg("stdout")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to start rustfmt");

    {
        use std::io::Write;
        rustfmt
            .stdin
            .as_mut()
            .expect("Failed to open stdin")
            .write_all(code.as_bytes())
            .expect("Failed to write to rustfmt stdin");
    }

    let output = rustfmt
        .wait_with_output()
        .expect("Failed to read rustfmt output");

    // Optional but recommended: surface rustfmt errors
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        panic!("rustfmt failed: {}", err);
    }

    String::from_utf8(output.stdout).expect("rustfmt output not valid UTF-8")
}

fn read_allow_list(path: &PathBuf) -> Option<HashSet<String>> {
    if !path.exists() {
        println!(
            "Warning: allow‑list file {} does not exist – falling back to processing all FX.",
            path.display()
        );
        return None;
    }

    let contents = fs::read_to_string(path).ok()?;
    let mut allow_names = HashSet::new();

    for line in contents.lines() {
        // Trim whitespace and ignore empty / comment lines
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        // The list contains just the base name (e.g. "AUAudioFilePlayer").
        // We already strip prefixes/suffixes in `split_fx_name`, so we do the same here.
        let (name, _, _) = split_fx_name(line);
        allow_names.insert(name);
    }

    Some(allow_names)
}

fn main() {
    let cli = Cli::parse();
    let yaml = fs::read_to_string(&cli.spec).expect("Failed to read input YAML");
    let mut raw_fx_list: Vec<RawFx> = serde_yaml::from_str(&yaml).expect("Failed to parse YAML");
    let allow_names = if let Some(path) = cli.allow_list {
        read_allow_list(&path)
    } else {
        None
    };
    let mut raw_fx_list: Vec<RawFx> = serde_yaml::from_str(&yaml).expect("Failed to parse YAML");
    let processed_fx_list = process_yaml_fx(raw_fx_list, allow_names);
    let mut code = String::new();
    write_imports(&mut code);
    write_fx_enum(&mut code, processed_fx_list.clone());
    writeln!(code, "\n").unwrap();
    for fx in &processed_fx_list {
        write_fx_struct(&mut code, fx.clone());
        writeln!(code, "\n").unwrap();
    }

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
