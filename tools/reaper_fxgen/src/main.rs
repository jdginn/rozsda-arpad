use clap::Parser;
use regex::Regex;
use serde::Deserialize;
use std::fmt::{Display, Write};
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};

#[derive(Parser)]
struct Cli {
    /// Path to the fx_dump yaml file
    spec: PathBuf,
    /// Output Rust file
    #[clap(short, long, default_value = "generated_fx_param.rs")]
    out: PathBuf,
}

// FX info as represented in the YAML
#[derive(Debug, Deserialize, Clone)]
struct YamlFx {
    fx_name: String,
    params: Vec<YamlFxParam>,
}

fn remove_unnamed_params(params: Vec<YamlFxParam>) -> Vec<YamlFxParam> {
    params
        .into_iter()
        .filter(|p| !(p.name.is_empty() || p.name == "-"))
        .collect()
}

// FX parameter info as represented in the YAML
#[derive(Debug, Deserialize, Clone)]
struct YamlFxParam {
    name: String,
    index: i32,
    min: f32,
    max: f32,
    // TODO: step size?
}

fn snake_case(s: &str) -> String {
    let re = Regex::new(r"([a-z0-9])([A-Z])").unwrap();
    re.replace_all(s, "$1_$2").to_lowercase()
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

fn write_module(code: &mut String, yaml_fx: YamlFx) {
    writeln!(code, "pub mod {} {{", sanitize(yaml_fx.fx_name.as_str())).unwrap();
    writeln!(code, "    use crate::track::track;").unwrap();
    writeln!(code, "    use uuid::Uuid;").unwrap();
    writeln!(code, "").unwrap();
    write_enum(code, yaml_fx.clone());
    writeln!(code, "").unwrap();
    write_encode_trackmsg(code, yaml_fx.clone());
    writeln!(code, "").unwrap();
    write_decode_trackmsg(code, yaml_fx.clone());
    writeln!(code, "}}").unwrap();
}

fn write_enum(code: &mut String, yaml_fx: YamlFx) {
    writeln!(code, "    pub enum Param {{").unwrap();
    for param in &yaml_fx.params {
        writeln!(
            code,
            "        {}(f32),",
            capitalize_first_letter(sanitize(&param.name).as_str())
        )
        .unwrap();
    }
    writeln!(code, "    }}").unwrap();
}

fn write_encode_trackmsg(code: &mut String, yaml_fx: YamlFx) {
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
            capitalize_first_letter(sanitize(&param.name).as_str())
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

fn write_decode_trackmsg(code: &mut String, yaml_fx: YamlFx) {
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
            param.index,
            capitalize_first_letter(sanitize(&param.name).as_str())
        )
        .unwrap();
    }
    writeln!(code, "            _ => None,").unwrap();
    writeln!(code, "        }}").unwrap();
    writeln!(code, "    }}").unwrap();
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

fn main() {
    let cli = Cli::parse();
    let yaml = fs::read_to_string(&cli.spec).expect("Failed to read input YAML");
    let mut fx_list: Vec<YamlFx> = serde_yaml::from_str(&yaml).expect("Failed to parse YAML");
    let mut code = String::new();
    for fx in &mut fx_list {
        fx.params = remove_unnamed_params(fx.params.clone());
        write_module(&mut code, fx.clone());
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
