use super::available_space;

#[test]
fn current_directory_has_space_information() {
    let result = available_space(std::path::Path::new("."));
    assert!(result.is_ok());
}
