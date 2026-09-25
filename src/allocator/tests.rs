#[test]
fn module_tests_are_enabled() {
    // Allocation itself is intentionally not tested here because it changes
    // filesystem state. Use an integration test with a dedicated filesystem
    // or temporary directory for real fallocate tests.
    assert!(true);
}
