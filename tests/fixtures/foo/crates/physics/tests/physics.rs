use foo_physics::Greeting;

#[test]
fn greets_by_name() {
    assert_eq!(Greeting::new("Ada").to_string(), "Hello, Ada!");
}
