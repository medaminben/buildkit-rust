use foo_core::Greeting;

fn main() {
    let arg = std::env::args().nth(1);
    match arg.as_deref() {
        Some("--version") => println!("foo-console {}", env!("BK_VERSION_FULL")),
        other => println!("{}", Greeting::new(other.unwrap_or("world"))),
    }
}
