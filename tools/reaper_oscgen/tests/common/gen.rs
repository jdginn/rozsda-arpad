use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Generates Rust code from the test fixture YAML into:
///   <crate>/target/oscgen-tests/generated.rs
///
/// Returns the output path on success.
///
/// Panics with stdout/stderr included if generation fails.
pub fn generate_fixture_to_target() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    println!(
        "Generating OSC code from fixture at {}",
        manifest_dir.display()
    );

    let spec_path = manifest_dir
        .join("tests")
        .join("fixtures")
        .join("oscgen_test_routes.yaml");

    if !spec_path.exists() {
        panic!("OSC spec fixture not found at {}", spec_path.display());
    }

    let out_dir = manifest_dir.join("target").join("oscgen-tests");
    ensure_dir(&out_dir);

    let out_path = out_dir.join("generated.rs");

    // Run:
    //   cargo run --package reaper_oscgen -- <SPEC> -o <OUT>
    //
    // Note: set current_dir to manifest_dir so this works when tests are run from anywhere.
    let output = Command::new("cargo")
        .current_dir(&manifest_dir)
        .args([
            "run",
            "--package",
            "reaper_oscgen",
            spec_path.to_str().expect("spec_path not valid UTF-8"),
            "-o",
            out_path.to_str().expect("out_path not valid UTF-8"),
        ])
        .output()
        .expect("Failed to spawn `cargo run` to generate OSC code");

    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!(
            "Generator command failed.\n\
             status: {}\n\
             spec: {}\n\
             out: {}\n\
             --- stdout ---\n{}\n\
             --- stderr ---\n{}\n",
            output.status,
            spec_path.display(),
            out_path.display(),
            stdout,
            stderr
        );
    }

    // Extra sanity check: ensure the output file exists and is non-empty.
    let meta = fs::metadata(&out_path)
        .unwrap_or_else(|e| panic!("Generated file missing at {}: {}", out_path.display(), e));
    if meta.len() == 0 {
        panic!(
            "Generated file was created but is empty: {}",
            out_path.display()
        );
    }

    out_path
}

fn ensure_dir(dir: &Path) {
    fs::create_dir_all(dir)
        .unwrap_or_else(|e| panic!("Failed to create directory {}: {}", dir.display(), e));
}
