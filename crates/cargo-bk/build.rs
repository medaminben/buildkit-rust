use buildkit::Fallback;

fn main() {
    buildkit::emit_version(Fallback("0.1.0"));
    println!("cargo:rerun-if-changed=../../templates");
}
