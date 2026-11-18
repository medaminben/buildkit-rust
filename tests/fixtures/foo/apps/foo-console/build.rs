fn main() {
    buildkit::emit_version(buildkit::Fallback(env!("CARGO_PKG_VERSION")));
}
