use super::parse_size;

#[test]
fn parses_sizes() {
    assert_eq!(parse_size("1K").expect("valid size"), 1024);
    assert_eq!(
        parse_size("2M").expect("valid size"),
        2 * 1024 * 1024
    );
    assert_eq!(
        parse_size("3G").expect("valid size"),
        3 * 1024 * 1024 * 1024
    );
}

#[test]
fn rejects_invalid_sizes() {
    assert!(parse_size("").is_err());
    assert!(parse_size("abc").is_err());
    assert!(parse_size("10X").is_err());
}
