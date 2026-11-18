use foo_core::Greeting;

#[test]
fn greets_by_name() {
    assert_eq!(Greeting::new("Ada").to_string(), "Hello, Ada!");
}
