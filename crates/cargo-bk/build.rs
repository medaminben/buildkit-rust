use std::path::Path;

/// `[workspace.metadata.buildkit] version` of buildkit-rust's root manifest.
/// It moves with each release tag, so a `cargo install --git … --tag vX.Y.Z`
/// checkout, which carries no tags, still reports `X.Y.Z`.
fn root_metadata_version(manifest: &Path) -> String {
    let text = std::fs::read_to_string(manifest)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", manifest.display()));
    let mut in_table = false;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_table = line == "[workspace.metadata.buildkit]";
        } else if in_table
            && let Some(value) = line
                .strip_prefix("version")
                .and_then(|r| r.trim_start().strip_prefix('='))
        {
            return value.trim().trim_matches('"').to_string();
        }
    }
    panic!(
        "{} has no [workspace.metadata.buildkit] version",
        manifest.display()
    );
}

fn main() {
    let dir = std::env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is not set");
    let manifest = Path::new(&dir).join("../../Cargo.toml");
    let fallback = root_metadata_version(&manifest);
    buildkit::emit_version(&fallback);
    println!("cargo:rustc-env=BK_FALLBACK={fallback}");
    println!("cargo:rerun-if-changed=../../Cargo.toml");
    println!("cargo:rerun-if-changed=../../templates");
    println!("cargo:rerun-if-changed=../../tools");
}
