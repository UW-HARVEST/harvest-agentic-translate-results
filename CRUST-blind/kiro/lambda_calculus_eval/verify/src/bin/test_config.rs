use lambda_calculus_eval::config::*;

#[test]
fn test_trim_both_sides() {
    let mut s = "  test  ".to_string();
    trim(&mut s);
    assert_eq!(s, "test");
}

#[test]
fn test_trim_no_whitespace() {
    let mut s = "hello".to_string();
    trim(&mut s);
    assert_eq!(s, "hello");
}

#[test]
fn test_trim_leading() {
    let mut s = "   leading".to_string();
    trim(&mut s);
    assert_eq!(s, "leading");
}

#[test]
fn test_trim_trailing() {
    let mut s = "trailing   ".to_string();
    trim(&mut s);
    assert_eq!(s, "trailing");
}

#[test]
fn test_get_config_type_file() {
    assert_eq!(get_config_type("file"), option_type_t::FILENAME);
}

#[test]
fn test_get_config_type_step() {
    assert_eq!(get_config_type("step_by_step_reduction"), option_type_t::STEP_REDUCTION);
}

#[test]
fn test_get_config_type_reduction() {
    assert_eq!(get_config_type("reduction_order"), option_type_t::REDUCTION_ORDER);
}

#[test]
fn test_parse_config_simple() {
    let mut key = String::new();
    let mut value = String::new();
    parse_config("file=expr.lambda", &mut key, &mut value);
    assert_eq!(key, "file");
    assert_eq!(value, "expr.lambda");
}

#[test]
fn test_parse_config_with_spaces() {
    let mut key = String::new();
    let mut value = String::new();
    parse_config("reduction_order = normal", &mut key, &mut value);
    assert_eq!(key, "reduction_order");
    assert_eq!(value, "normal");
}

fn main() {}
