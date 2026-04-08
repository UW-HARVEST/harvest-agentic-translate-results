use lambda_calculus_eval::config;

#[test]
fn test_trim_spaces() {
    let mut s = "  test  ".to_string();
    config::trim(&mut s);
    assert_eq!(s, "test");
}

#[test]
fn test_trim_no_spaces() {
    let mut s = "test".to_string();
    config::trim(&mut s);
    assert_eq!(s, "test");
}

#[test]
fn test_trim_only_leading() {
    let mut s = "  test".to_string();
    config::trim(&mut s);
    assert_eq!(s, "test");
}

#[test]
fn test_trim_only_trailing() {
    let mut s = "test  ".to_string();
    config::trim(&mut s);
    assert_eq!(s, "test");
}

#[test]
fn test_get_config_type_file() {
    assert_eq!(config::get_config_type("file"), config::option_type_t::FILENAME);
}

#[test]
fn test_get_config_type_step() {
    assert_eq!(config::get_config_type("step_by_step_reduction"), config::option_type_t::STEP_REDUCTION);
}

#[test]
fn test_get_config_type_reduction() {
    assert_eq!(config::get_config_type("reduction_order"), config::option_type_t::REDUCTION_ORDER);
}

#[test]
fn test_parse_config_basic() {
    let mut key = String::new();
    let mut value = String::new();
    config::parse_config("file=expr.lambda", &mut key, &mut value);
    assert_eq!(key, "file");
    assert_eq!(value, "expr.lambda");
}

#[test]
fn test_parse_config_with_spaces() {
    let mut key = String::new();
    let mut value = String::new();
    config::parse_config("  file  =  expr.lambda  ", &mut key, &mut value);
    assert_eq!(key, "file");
    assert_eq!(value, "expr.lambda");
}

fn main() {}
