use foo_core::Greeting;

fn main() {
    let arg = std::env::args().nth(1);
    match arg.as_deref() {
        Some("--version") => println!("foo-console {}", env!("BK_VERSION_FULL")),
        Some(name) => println!("{}", Greeting::new(name)),
        None => println!("{}", Greeting::new("world")),
    }
}
