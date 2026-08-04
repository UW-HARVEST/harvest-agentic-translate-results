use lambda_calculus_eval::config::{
    get_config_type, option_type_t, parse_config, trim, CONFIG_PATH,
};

#[test]
fn test_const_path() {
    assert_eq!(CONFIG_PATH, "config");
}

#[test]
fn test_trim_simple() {
    let mut s = String::from("  test  ");
    trim(&mut s);
    assert_eq!(s, "test");
}

#[test]
fn test_trim_no_whitespace() {
    let mut s = String::from("hello");
    trim(&mut s);
    assert_eq!(s, "hello");
}

#[test]
fn test_trim_only_left() {
    let mut s = String::from("   hello");
    trim(&mut s);
    assert_eq!(s, "hello");
}

#[test]
fn test_trim_only_right() {
    let mut s = String::from("hello   ");
    trim(&mut s);
    assert_eq!(s, "hello");
}

#[test]
fn test_trim_with_internal_spaces() {
    let mut s = String::from("  hello world  ");
    trim(&mut s);
    assert_eq!(s, "hello world");
}

#[test]
fn test_get_config_type_file() {
    assert_eq!(get_config_type("file"), option_type_t::FILENAME);
}

#[test]
fn test_get_config_type_step() {
    assert_eq!(
        get_config_type("step_by_step_reduction"),
        option_type_t::STEP_REDUCTION
    );
}

#[test]
fn test_get_config_type_reduction_order() {
    assert_eq!(
        get_config_type("reduction_order"),
        option_type_t::REDUCTION_ORDER
    );
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
    // C trims around the = sign
    let line = "file = expr.lambda";
    let mut key = String::new();
    let mut value = String::new();
    parse_config(line, &mut key, &mut value);
    assert_eq!(key, "file");
    assert_eq!(value, "expr.lambda");
}

fn main() {}
