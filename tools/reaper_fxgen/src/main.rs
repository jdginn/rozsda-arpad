use clap::Parser;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::fmt::Write;
use std::fs;
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Parser)]
struct Cli {
    /// Path to the fx_dump yaml file
    spec: PathBuf,
    /// Optional path to a text file containing an allow‑list of FX base names to include
    #[clap(short, long)]
    allow_list: Option<PathBuf>,
    /// Output directory for generated Rust modules
    #[clap(short, long, default_value = "generated_fx")]
    out_dir: PathBuf,
}

#[derive(Debug, Deserialize, Clone)]
struct RawFx {
    fx_name: String,
    #[serde(default)]
    params: Vec<RawFxParam>,
}

#[derive(Debug, Deserialize, Clone)]
struct RawEnumValue {
    index: usize,
    name: String,
}

#[derive(Debug, Deserialize, Clone)]
struct RawFxParam {
    name: String,
    index: i32,
    min: f32,
    max: f32,
    #[serde(default)]
    is_toggle: bool,
    #[serde(default)]
    presumed_enum_values: Vec<RawEnumValue>,
}

#[derive(Debug, Clone)]
struct Fx {
    fx_name: String,
    repr: String,
    params: Vec<FxParam>,
    plugin_type: PluginType,
    developer: DeveloperName,
}

#[derive(Debug, Clone, PartialEq)]
struct FxParam {
    name: String,
    repr: String,
    index: i32,
    min: f32,
    max: f32,
    kind: ParamKind,
}

#[derive(Debug, Clone, PartialEq)]
enum ParamKind {
    Continuous,
    Toggle,
    Enum {
        enum_name: String,
        variants: Vec<EnumVariant>,
    },
}

#[derive(Debug, Clone, PartialEq)]
struct EnumVariant {
    repr: String,
    raw_index: usize,
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
        "VST" | "VST3" => PluginType::VST,
        "AU" | "AUi" => PluginType::AU,
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

    if s == "Container" {
        return (
            "Container".to_string(),
            PluginType::Container,
            DeveloperName::Unknown,
        );
    }

    let (plugin_type, rest) = if let Some((prefix, rest)) = s.split_once(':') {
        (plugin_type_from_prefix(prefix.trim()), rest.trim())
    } else {
        (PluginType::Other(String::new()), s)
    };

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

fn params_equal(p1: &[FxParam], p2: &[RawFxParam]) -> bool {
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

fn rust_keywords() -> &'static [&'static str] {
    &[
        "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn",
        "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref",
        "return", "self", "Self", "static", "struct", "super", "trait", "true", "type", "unsafe",
        "use", "where", "while", "async", "await", "dyn", "abstract", "become", "box", "do",
        "final", "macro", "override", "priv", "try", "typeof", "unsized", "virtual", "yield",
    ]
}

fn apply_keyword_suffix(ident: &str) -> String {
    if rust_keywords().contains(&ident) {
        format!("{ident}_")
    } else {
        ident.to_string()
    }
}

fn make_unique(base: String, used: &mut HashSet<String>) -> String {
    if used.insert(base.clone()) {
        return base;
    }
    let mut n = 2usize;
    loop {
        let candidate = format!("{base}_{n}");
        if used.insert(candidate.clone()) {
            return candidate;
        }
        n += 1;
    }
}

fn sanitize_enum(input: &str) -> String {
    let s = input.trim();

    if s == "-" {
        return "Minus".to_string();
    }
    if s == "+" {
        return "Plus".to_string();
    }

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

    let mut out = String::new();
    let mut new_word = true;

    // FIX: tokenize from no_parens (not s)
    for ch in no_parens.chars() {
        if ch.is_ascii_alphanumeric() {
            if new_word {
                out.extend(ch.to_uppercase());
                new_word = false;
            } else {
                out.push(ch);
            }
        } else {
            new_word = true;
        }
    }

    if out.is_empty() {
        out = "Unnamed".to_string();
    }

    if out.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        out.insert(0, '_');
    }

    apply_keyword_suffix(&out)
}

fn pascal_to_snake(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len() + 8);

    for i in 0..chars.len() {
        let c = chars[i];
        let prev = i.checked_sub(1).map(|j| chars[j]);
        let next = if i + 1 < chars.len() {
            Some(chars[i + 1])
        } else {
            None
        };

        let is_boundary = c.is_uppercase()
            && i > 0
            && match (prev, next) {
                (Some(p), Some(n)) => p.is_lowercase() || (p.is_uppercase() && n.is_lowercase()),
                (Some(p), None) => p.is_lowercase(),
                _ => false,
            };

        if is_boundary {
            out.push('_');
        }
        out.extend(c.to_lowercase());
    }

    out
}

fn build_param_kind(param: &RawFxParam, used_type_names: &mut HashSet<String>) -> ParamKind {
    if param.is_toggle {
        return ParamKind::Toggle;
    }

    if !param.presumed_enum_values.is_empty() {
        let base_enum_name = sanitize_enum(&param.name);
        let enum_name = make_unique(base_enum_name, used_type_names);

        let mut used_variants = HashSet::new();
        let mut values = param.presumed_enum_values.clone();
        values.sort_by_key(|v| v.index);

        let variants = values
            .into_iter()
            .map(|v| {
                let variant_base = sanitize_enum(&v.name);
                let repr = make_unique(variant_base, &mut used_variants);
                EnumVariant {
                    repr,
                    raw_index: v.index,
                }
            })
            .collect();

        return ParamKind::Enum {
            enum_name,
            variants,
        };
    }

    ParamKind::Continuous
}

fn process_yaml_fx(raw_yaml_fx_list: Vec<RawFx>, allow_names: Option<HashSet<String>>) -> Vec<Fx> {
    let mut plugin_names: HashMap<String, HashMap<PluginType, Fx>> = HashMap::new();

    for raw_fx in &raw_yaml_fx_list {
        let (base_name, plugin_type, developer) = split_fx_name(&raw_fx.fx_name);

        if let Some(allowed) = &allow_names {
            if !allowed.contains(&base_name) {
                continue;
            }
        }

        let base_repr = sanitize_enum(&base_name);
        let per_name = plugin_names.entry(base_name.clone()).or_default();

        let needs_disambiguation = per_name
            .values()
            .any(|existing| !params_equal(&existing.params, &raw_fx.params));

        if per_name.get(&plugin_type).is_some() {
            panic!(
                "Duplicate plugin type for same base name: {} with plugin type {:?} already exists.",
                base_name, plugin_type
            );
        }

        if needs_disambiguation {
            for fx in per_name.values_mut() {
                fx.repr = format!("{}_{}", base_repr, plugin_type_suffix(&fx.plugin_type));
            }
        }

        use std::collections::hash_map::Entry;
        match per_name.entry(plugin_type.clone()) {
            Entry::Vacant(v) => {
                let repr = if needs_disambiguation {
                    format!("{}_{}", base_repr, plugin_type_suffix(&plugin_type))
                } else {
                    base_repr.clone()
                };

                let mut used_param_names = HashSet::new();
                let mut used_type_names = HashSet::new();

                let mut raw_params = raw_fx.params.clone();
                raw_params.sort_by_key(|p| p.index);

                let params = raw_params
                    .into_iter()
                    .map(|p| {
                        let param_base = sanitize_enum(&p.name);
                        let param_repr = make_unique(param_base, &mut used_param_names);
                        let kind = build_param_kind(&p, &mut used_type_names);
                        FxParam {
                            name: p.name,
                            repr: param_repr,
                            index: p.index,
                            min: p.min,
                            max: p.max,
                            kind,
                        }
                    })
                    .collect();

                v.insert(Fx {
                    fx_name: raw_fx.fx_name.clone(),
                    repr,
                    params,
                    plugin_type,
                    developer,
                });
            }
            Entry::Occupied(_) => {}
        }
    }

    let mut out: Vec<Fx> = plugin_names
        .into_values()
        .flat_map(|type_map| type_map.into_values())
        .collect();

    out.sort_by(|a, b| a.repr.cmp(&b.repr));
    out
}

fn write_fx_mod(code: &mut String, yaml_fx: &Fx) {
    writeln!(code, "pub mod {} {{", pascal_to_snake(&yaml_fx.repr)).unwrap();
    writeln!(code, "use super::*;\n").unwrap();

    write_name_fn(code, yaml_fx);
    writeln!(code).unwrap();

    write_param_enums_for_presumed_enums(code, yaml_fx);
    write_param_enum(code, yaml_fx);
    write_encode_trackmsg(code, yaml_fx);
    write_decode_trackmsg(code, yaml_fx);

    writeln!(code, "}}").unwrap();
}

fn write_name_fn(code: &mut String, yaml_fx: &Fx) {
    writeln!(code, "pub const FX_NAME: &str = {:?};\n", yaml_fx.fx_name).unwrap();
}

fn write_param_enums_for_presumed_enums(code: &mut String, yaml_fx: &Fx) {
    for param in &yaml_fx.params {
        if let ParamKind::Enum {
            enum_name,
            variants,
        } = &param.kind
        {
            writeln!(code, "#[derive(Debug, Clone, Copy, PartialEq)]").unwrap();
            writeln!(code, "pub enum {} {{", enum_name).unwrap();
            for v in variants {
                writeln!(code, "    {},", v.repr).unwrap();
            }
            writeln!(code, "}}").unwrap();

            writeln!(code, "impl {} {{", enum_name).unwrap();
            writeln!(code, "pub fn to_raw(self) -> f32 {{").unwrap();
            writeln!(code, "    match self {{").unwrap();
            for v in variants {
                writeln!(code, "        Self::{} => {}f32,", v.repr, v.raw_index).unwrap();
            }
            writeln!(code, "    }}").unwrap();
            writeln!(code, "}}").unwrap();

            writeln!(code, "pub fn from_raw(value: f32) -> Option<Self> {{").unwrap();
            writeln!(code, "    let rounded = value.round() as isize;").unwrap();
            writeln!(code, "    match rounded {{").unwrap();
            for v in variants {
                writeln!(
                    code,
                    "        {} => Some(Self::{}),",
                    v.raw_index as isize, v.repr
                )
                .unwrap();
            }
            writeln!(code, "        _ => None,").unwrap();
            writeln!(code, "    }}").unwrap();
            writeln!(code, "}}").unwrap();
            writeln!(code, "}}\n").unwrap();
        }
    }
}

fn write_param_enum(code: &mut String, yaml_fx: &Fx) {
    writeln!(code, "#[derive(Debug, Clone, Copy, PartialEq)]").unwrap();
    writeln!(code, "pub enum Param {{").unwrap();

    for param in &yaml_fx.params {
        let ty = match &param.kind {
            ParamKind::Toggle => "bool".to_string(),
            ParamKind::Enum { enum_name, .. } => enum_name.clone(),
            ParamKind::Continuous => "f32".to_string(),
        };
        writeln!(code, "{}({}),", param.repr, ty).unwrap();
    }

    writeln!(code, "}}").unwrap();
}

fn write_encode_trackmsg(code: &mut String, yaml_fx: &Fx) {
    writeln!(
        code,
        "pub fn encode_trackmsg(track_guid: Uuid, fx_index: i32, param: Param) -> track::TrackMsg {{"
    )
    .unwrap();
    writeln!(code, "match param {{").unwrap();

    for param in &yaml_fx.params {
        match &param.kind {
            ParamKind::Continuous => {
                writeln!(
                    code,
                    "        Param::{}(value) => track::FXParamValue {{",
                    param.repr
                )
                .unwrap();
                writeln!(code, "            track_guid,").unwrap();
                writeln!(code, "            fx_index,").unwrap();
                writeln!(code, "            param_index: {},", param.index).unwrap();
                writeln!(code, "            value,").unwrap();
                writeln!(code, "        }}").unwrap();
                writeln!(code, "        .into(),").unwrap();
            }
            ParamKind::Toggle => {
                writeln!(
                    code,
                    "        Param::{}(value) => track::FXParamValue {{",
                    param.repr
                )
                .unwrap();
                writeln!(code, "            track_guid,").unwrap();
                writeln!(code, "            fx_index,").unwrap();
                writeln!(code, "            param_index: {},", param.index).unwrap();
                writeln!(
                    code,
                    "            value: if value {{ {}f32 }} else {{ {}f32 }},",
                    param.max, param.min
                )
                .unwrap();
                writeln!(code, "        }}").unwrap();
                writeln!(code, "        .into(),").unwrap();
            }
            ParamKind::Enum { .. } => {
                writeln!(
                    code,
                    "        Param::{}(value) => track::FXParamValue {{",
                    param.repr
                )
                .unwrap();
                writeln!(code, "            track_guid,").unwrap();
                writeln!(code, "            fx_index,").unwrap();
                writeln!(code, "            param_index: {},", param.index).unwrap();
                writeln!(code, "            value: value.to_raw(),").unwrap();
                writeln!(code, "        }}").unwrap();
                writeln!(code, "        .into(),").unwrap();
            }
        }
    }

    writeln!(code, "    }}").unwrap();
    writeln!(code, "}}").unwrap();
}

fn write_decode_trackmsg(code: &mut String, yaml_fx: &Fx) {
    writeln!(
        code,
        "pub fn decode_trackmsg(msg: track::FXParamValue) -> Option<Param> {{"
    )
    .unwrap();
    writeln!(code, "    match msg.param_index {{").unwrap();

    for param in &yaml_fx.params {
        match &param.kind {
            ParamKind::Continuous => {
                writeln!(
                    code,
                    "        {} => Some(Param::{}(msg.value)),",
                    param.index, param.repr
                )
                .unwrap();
            }
            ParamKind::Toggle => {
                let midpoint = (param.min + param.max) / 2.0;
                writeln!(
                    code,
                    "        {} => Some(Param::{}(msg.value >= {}f32)),",
                    param.index, param.repr, midpoint
                )
                .unwrap();
            }
            ParamKind::Enum { enum_name, .. } => {
                writeln!(
                    code,
                    "        {} => {}::from_raw(msg.value).map(Param::{}),",
                    param.index, enum_name, param.repr
                )
                .unwrap();
            }
        }
    }

    writeln!(code, "        _ => None,").unwrap();
    writeln!(code, "    }}").unwrap();
    writeln!(code, "}}").unwrap();
}

fn write_imports(code: &mut String) {
    writeln!(code, "use uuid::Uuid;").unwrap();
    writeln!(code, "use crate::track::track;").unwrap();
}

fn write_fx_enum(code: &mut String, effects: &[Fx]) {
    writeln!(code, "#[derive(Debug, Clone, Copy, PartialEq)]").unwrap();
    writeln!(code, "pub enum FX {{").unwrap();
    for fx in effects {
        writeln!(code, "{},", fx.repr).unwrap();
    }
    writeln!(code, "}}").unwrap();
}

fn write_fx_file_content(yaml_fx: &Fx) -> String {
    let mut code = String::new();
    write_imports(&mut code);
    writeln!(code).unwrap();

    // same body you currently emit inside module, but now at file scope
    write_name_fn(&mut code, yaml_fx);
    writeln!(code).unwrap();
    write_param_enums_for_presumed_enums(&mut code, yaml_fx);
    write_param_enum(&mut code, yaml_fx);
    write_encode_trackmsg(&mut code, yaml_fx);
    write_decode_trackmsg(&mut code, yaml_fx);

    format_code(&code)
}

fn write_mod_rs(out_dir: &Path, effects: &[Fx]) -> io::Result<()> {
    let mut code = String::new();
    write_imports(&mut code);
    writeln!(code).unwrap();

    let mut module_names: Vec<String> =
        effects.iter().map(|fx| pascal_to_snake(&fx.repr)).collect();
    module_names.sort();
    module_names.dedup();

    for m in &module_names {
        writeln!(code, "pub mod {};", m).unwrap();
    }
    writeln!(code).unwrap();

    writeln!(code, "#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]").unwrap();
    writeln!(code, "pub enum FxId {{").unwrap();
    for fx in effects {
        writeln!(code, "    {},", fx.repr).unwrap();
    }
    writeln!(code, "}}").unwrap();

    let mod_rs = format_code(&code);
    fs::write(out_dir.join("mod.rs"), mod_rs)
}

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
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (name, _, _) = split_fx_name(line);
        allow_names.insert(name);
    }

    Some(allow_names)
}

fn collect_files_recursively(dir: &Path) -> io::Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    if !dir.exists() {
        return Ok(out);
    }

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let meta = entry.metadata()?;
        if meta.is_dir() {
            out.extend(collect_files_recursively(&path)?);
        } else if meta.is_file() {
            out.push(path);
        }
    }
    Ok(out)
}

fn prompt_yes_no(prompt: &str) -> io::Result<bool> {
    print!("{prompt} [y/N]: ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let s = input.trim().to_ascii_lowercase();
    Ok(s == "y" || s == "yes")
}

fn prepare_output_directory(out_dir: &Path) -> io::Result<()> {
    fs::create_dir_all(out_dir)?;

    let existing_files = collect_files_recursively(out_dir)?;
    if existing_files.is_empty() {
        return Ok(());
    }

    println!("Warning: about to delete the following files:");
    for p in &existing_files {
        println!("  {}", p.display());
    }

    if !prompt_yes_no("Are you sure you wish to continue?")? {
        return Err(io::Error::new(
            io::ErrorKind::Interrupted,
            "Aborted by user.",
        ));
    }

    for p in existing_files {
        fs::remove_file(&p)?;
    }

    // optionally remove empty subdirs
    fn remove_empty_dirs(dir: &Path) -> io::Result<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let p = entry.path();
            if p.is_dir() {
                remove_empty_dirs(&p)?;
                if fs::read_dir(&p)?.next().is_none() {
                    fs::remove_dir(&p)?;
                }
            }
        }
        Ok(())
    }
    remove_empty_dirs(out_dir)?;

    Ok(())
}

fn main() {
    let cli = Cli::parse();

    let yaml = fs::read_to_string(&cli.spec).expect("Failed to read input YAML");
    let allow_names = if let Some(path) = cli.allow_list {
        read_allow_list(&path)
    } else {
        None
    };

    let raw_fx_list: Vec<RawFx> = serde_yaml::from_str(&yaml).expect("Failed to parse YAML");
    let processed_fx_list = process_yaml_fx(raw_fx_list, allow_names);

    prepare_output_directory(&cli.out_dir).expect("Failed preparing output directory");

    for fx in &processed_fx_list {
        let module_name = pascal_to_snake(&fx.repr);
        let file_path = cli.out_dir.join(format!("{module_name}.rs"));
        let code = write_fx_file_content(fx);
        fs::write(file_path, code).expect("Failed writing FX module");
    }

    write_mod_rs(&cli.out_dir, &processed_fx_list).expect("Failed writing mod.rs");
}
