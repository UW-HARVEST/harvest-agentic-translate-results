use jccc::parse::{parse_simple_main_func};

#[test]
fn test_parse_simple_main_func() {
    assert_eq!(parse_simple_main_func(), 0);
}

#[test]
fn test_parse_nonexistent_file() {
    let ret = jccc::parse::parse("/nonexistent/file.c");
    assert_eq!(ret, 1);
}

fn main() {}
