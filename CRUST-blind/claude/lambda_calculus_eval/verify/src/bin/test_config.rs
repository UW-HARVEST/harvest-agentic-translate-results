use lambda_calculus_eval::config::{
    get_config_type, option_type_t, parse_config, trim,
};

#[test]
fn test_trim_basic() {
    let mut s = String::from("  hello  ");
    trim(&mut s);
    assert_eq!(s, "hello");
}

#[test]
fn test_trim_with_internal_whitespace() {
    // C: trim only removes leading/trailing whitespace, not internal.
    let mut s = String::from("  test  with spaces  ");
    trim(&mut s);
    assert_eq!(s, "test  with spaces");
}

#[test]
fn test_trim_already_trimmed() {
    let mut s = String::from("hello");
    trim(&mut s);
    assert_eq!(s, "hello");
}

#[test]
fn test_trim_empty() {
    let mut s = String::from("");
    trim(&mut s);
    assert_eq!(s, "");
}

#[test]
fn test_get_config_type_file() {
    let opt = get_config_type("file");
    assert!(matches!(opt, option_type_t::FILENAME));
}

#[test]
fn test_get_config_type_step_reduction() {
    let opt = get_config_type("step_by_step_reduction");
    assert!(matches!(opt, option_type_t::STEP_REDUCTION));
}

#[test]
fn test_get_config_type_reduction_order() {
    let opt = get_config_type("reduction_order");
    assert!(matches!(opt, option_type_t::REDUCTION_ORDER));
}

#[test]
fn test_parse_config_basic() {
    let line = "file=expr.lambda";
    let mut key = String::new();
    let mut value = String::new();
    parse_config(line, &mut key, &mut value);
    assert_eq!(key, "file");
    assert_eq!(value, "expr.lambda");
}

#[test]
fn test_parse_config_with_spaces() {
    let line = "key = value";
    let mut key = String::new();
    let mut value = String::new();
    parse_config(line, &mut key, &mut value);
    assert_eq!(key, "key");
    assert_eq!(value, "value");
}

#[test]
fn test_parse_config_step_reduction() {
    let line = "step_by_step_reduction=true";
    let mut key = String::new();
    let mut value = String::new();
    parse_config(line, &mut key, &mut value);
    assert_eq!(key, "step_by_step_reduction");
    assert_eq!(value, "true");
}

fn main() {}
