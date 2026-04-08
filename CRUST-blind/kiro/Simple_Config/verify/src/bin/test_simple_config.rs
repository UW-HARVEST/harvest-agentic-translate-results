use Simple_Config::simple_config::*;

// ===== PARSE SUCCESS TESTS =====

#[test]
fn test_parse_empty() {
    let cfg = cfg_parse("").unwrap();
    assert_eq!(cfg.count, 0);
    assert_eq!(cfg.entries.len(), 0);
}

#[test]
fn test_parse_whitespace() {
    let cfg = cfg_parse(" ").unwrap();
    assert_eq!(cfg.count, 0);
}

#[test]
fn test_parse_comment_no_newline() {
    let cfg = cfg_parse("#").unwrap();
    assert_eq!(cfg.count, 0);
}

#[test]
fn test_parse_comment_with_newline() {
    let cfg = cfg_parse("#\n").unwrap();
    assert_eq!(cfg.count, 0);
}

#[test]
fn test_parse_string_value() {
    let cfg = cfg_parse("key: \"hello, world!\"").unwrap();
    assert_eq!(cfg.count, 1);
    assert_eq!(cfg.entries[0].key, "key");
    assert_eq!(cfg.entries[0].val, CfgVal::String("hello, world!".to_string()));
}

#[test]
fn test_parse_int_with_inline_comment() {
    let cfg = cfg_parse("key: 10 # Inline comment").unwrap();
    assert_eq!(cfg.count, 1);
    assert_eq!(cfg.entries[0].key, "key");
    assert_eq!(cfg.entries[0].val, CfgVal::Int(10));
}

#[test]
fn test_parse_negative_int() {
    let cfg = cfg_parse("key: -1").unwrap();
    assert_eq!(cfg.count, 1);
    assert_eq!(cfg.entries[0].val, CfgVal::Int(-1));
}

#[test]
fn test_parse_true() {
    let cfg = cfg_parse("key: true").unwrap();
    assert_eq!(cfg.count, 1);
    assert_eq!(cfg.entries[0].val, CfgVal::Boolean(true));
}

#[test]
fn test_parse_false() {
    let cfg = cfg_parse("key: false").unwrap();
    assert_eq!(cfg.count, 1);
    assert_eq!(cfg.entries[0].val, CfgVal::Boolean(false));
}

#[test]
fn test_parse_float_half() {
    let cfg = cfg_parse("key: 0.5").unwrap();
    assert_eq!(cfg.count, 1);
    assert_eq!(cfg.entries[0].val, CfgVal::Float(0.5));
}

#[test]
fn test_parse_float_trailing_dot() {
    let cfg = cfg_parse("key: 1.").unwrap();
    assert_eq!(cfg.count, 1);
    assert_eq!(cfg.entries[0].val, CfgVal::Float(1.0));
}

#[test]
fn test_parse_key_with_underscore() {
    let cfg = cfg_parse("key_: true").unwrap();
    assert_eq!(cfg.count, 1);
    assert_eq!(cfg.entries[0].key, "key_");
    assert_eq!(cfg.entries[0].val, CfgVal::Boolean(true));
}

#[test]
fn test_parse_key_with_dot() {
    let cfg = cfg_parse("key.: true").unwrap();
    assert_eq!(cfg.count, 1);
    assert_eq!(cfg.entries[0].key, "key.");
    assert_eq!(cfg.entries[0].val, CfgVal::Boolean(true));
}

#[test]
fn test_parse_two_entries() {
    let cfg = cfg_parse("a: true\nb:true").unwrap();
    assert_eq!(cfg.count, 2);
    assert_eq!(cfg.entries[0].key, "a");
    assert_eq!(cfg.entries[0].val, CfgVal::Boolean(true));
    assert_eq!(cfg.entries[1].key, "b");
    assert_eq!(cfg.entries[1].val, CfgVal::Boolean(true));
}

#[test]
fn test_parse_full_config() {
    let src = "font: \"JetBrainsMono Nerd Font\"\n\
               font.size: 14\n\
               zoom: 1.5\n\
               line_numbers: true\n\
               ruler: false\n\
               bg.color: rgba(255, 255, 255, 1)";
    let cfg = cfg_parse(src).unwrap();
    assert_eq!(cfg.count, 6);
    assert_eq!(cfg.entries[0].key, "font");
    assert_eq!(cfg.entries[0].val, CfgVal::String("JetBrainsMono Nerd Font".to_string()));
    assert_eq!(cfg.entries[1].key, "font.size");
    assert_eq!(cfg.entries[1].val, CfgVal::Int(14));
    assert_eq!(cfg.entries[2].key, "zoom");
    assert_eq!(cfg.entries[2].val, CfgVal::Float(1.5));
    assert_eq!(cfg.entries[3].key, "line_numbers");
    assert_eq!(cfg.entries[3].val, CfgVal::Boolean(true));
    assert_eq!(cfg.entries[4].key, "ruler");
    assert_eq!(cfg.entries[4].val, CfgVal::Boolean(false));
    assert_eq!(cfg.entries[5].key, "bg.color");
    assert_eq!(cfg.entries[5].val, CfgVal::Color(CfgColor { r: 255, g: 255, b: 255, a: 255 }));
}

#[test]
fn test_parse_rgba_float_alpha() {
    let cfg = cfg_parse("key: rgba(100, 200, 50, 0.5)").unwrap();
    assert_eq!(cfg.count, 1);
    assert_eq!(cfg.entries[0].val, CfgVal::Color(CfgColor { r: 100, g: 200, b: 50, a: 127 }));
}

#[test]
fn test_parse_rgba_int_alpha_zero() {
    let cfg = cfg_parse("key: rgba(100, 200, 50, 0)").unwrap();
    assert_eq!(cfg.count, 1);
    assert_eq!(cfg.entries[0].val, CfgVal::Color(CfgColor { r: 100, g: 200, b: 50, a: 0 }));
}

#[test]
fn test_parse_rgba_int_alpha_one() {
    let cfg = cfg_parse("key: rgba(100, 200, 50, 1)").unwrap();
    assert_eq!(cfg.count, 1);
    assert_eq!(cfg.entries[0].val, CfgVal::Color(CfgColor { r: 100, g: 200, b: 50, a: 255 }));
}

// ===== PARSE ERROR TESTS =====

fn assert_parse_err(src: &str, expected_msg: &str, expected_off: i32, expected_row: i32, expected_col: i32) {
    let err = cfg_parse(src).unwrap_err();
    assert_eq!(err.msg, expected_msg, "src={:?}", src);
    assert_eq!(err.off, expected_off, "src={:?} off", src);
    assert_eq!(err.row, expected_row, "src={:?} row", src);
    assert_eq!(err.col, expected_col, "src={:?} col", src);
}

#[test]
fn test_err_missing_key() {
    assert_parse_err("!", "missing key", 0, 1, 1);
}

#[test]
fn test_err_key_too_long() {
    assert_parse_err("xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx", "key too long", 33, 1, 34);
}

#[test]
fn test_err_colon_expected() {
    assert_parse_err("key", "':' expected", 3, 1, 4);
}

#[test]
fn test_err_colon_expected_bang() {
    assert_parse_err("key!", "':' expected", 3, 1, 4);
}

#[test]
fn test_err_missing_value_after_colon_space() {
    assert_parse_err("key  :", "missing value", 6, 1, 7);
}

#[test]
fn test_err_missing_value_after_colon() {
    assert_parse_err("key:", "missing value", 4, 1, 5);
}

#[test]
fn test_err_missing_value_trailing_spaces() {
    assert_parse_err("key:  ", "missing value", 6, 1, 7);
}

#[test]
fn test_err_missing_value_newline() {
    assert_parse_err("key:\n", "missing value", 4, 1, 5);
}

#[test]
fn test_err_missing_value_spaces_newline() {
    assert_parse_err("key:  \n", "missing value", 6, 1, 7);
}

#[test]
fn test_err_invalid_value() {
    assert_parse_err("key: @\n", "invalid value", 5, 1, 6);
}

#[test]
fn test_err_unclosed_quote_empty() {
    assert_parse_err("key: \"", "closing '\"' expected", 6, 1, 7);
}

#[test]
fn test_err_unclosed_quote_hello() {
    assert_parse_err("key: \"hello", "closing '\"' expected", 11, 1, 12);
}

#[test]
fn test_err_unexpected_char() {
    assert_parse_err("key: 10x", "unexpected character 'x'", 7, 1, 8);
}

#[test]
fn test_err_invalid_value_dash() {
    assert_parse_err("key: -", "invalid value", 5, 1, 6);
}

#[test]
fn test_err_invalid_literal_t() {
    assert_parse_err("key: t", "invalid literal", 5, 1, 6);
}

#[test]
fn test_err_invalid_literal_f() {
    assert_parse_err("key: f", "invalid literal", 5, 1, 6);
}

#[test]
fn test_err_invalid_literal_x() {
    assert_parse_err("key: x", "invalid literal", 5, 1, 6);
}

#[test]
fn test_err_rgba_comma_expected() {
    assert_parse_err("key: rgba(", "',' expected", 10, 1, 11);
}

#[test]
fn test_err_rgba_paren_expected() {
    assert_parse_err("key: rgba", "'(' expected", 9, 1, 10);
}

#[test]
fn test_err_rgba_paren_expected_space() {
    assert_parse_err("key: rgba x", "'(' expected", 10, 1, 11);
}

#[test]
fn test_err_invalid_literal_r() {
    assert_parse_err("key: r", "invalid literal", 5, 1, 6);
}

#[test]
fn test_err_rgba_float_rgb() {
    assert_parse_err("key: rgba(0.5", "red, blue and green must be integers in range [0, 255]", 10, 1, 11);
}

#[test]
fn test_err_rgba_negative_rgb() {
    assert_parse_err("key: rgba(-1", "red, blue and green must be integers in range [0, 255]", 12, 1, 13);
}

#[test]
fn test_err_rgba_rgb_too_large() {
    assert_parse_err("key: rgba(256", "red, blue and green must be integers in range [0, 255]", 13, 1, 14);
}

#[test]
fn test_err_rgba_alpha_negative() {
    assert_parse_err("key: rgba(255, 255, 255, -1)", "alpha must be in range [0, 1]", 27, 1, 28);
}

#[test]
fn test_err_rgba_alpha_too_large() {
    assert_parse_err("key: rgba(255, 255, 255, 2)", "alpha must be in range [0, 1]", 26, 1, 27);
}

#[test]
fn test_err_rgba_close_paren_expected() {
    assert_parse_err("key: rgba(255, 255, 255, 1", "')' expected", 26, 1, 27);
}

#[test]
fn test_err_rgba_alpha_number_expected() {
    assert_parse_err("key: rgba(255, 255, 255, x", "number expected", 25, 1, 26);
}

#[test]
fn test_err_rgba_rgb_number_expected() {
    assert_parse_err("key: rgba(x", "number expected", 10, 1, 11);
}

#[test]
fn test_err_rgba_comma_expected_after_num() {
    assert_parse_err("key: rgba(255 x", "',' expected", 14, 1, 15);
}

#[test]
fn test_err_rgba_close_paren_expected_after_alpha() {
    assert_parse_err("key: rgba(255, 255, 255, 1 x", "')' expected", 27, 1, 28);
}

#[test]
fn test_err_int_too_large() {
    assert_parse_err("key: 2147483648", "number too large", 15, 1, 16);
}

#[test]
fn test_err_float_fract_too_many_zeros() {
    assert_parse_err("key: 0.0000000000", "number too large", 17, 1, 18);
}

#[test]
fn test_err_float_fract_too_long() {
    assert_parse_err("key: 1.33333333333333333", "number too large", 17, 1, 18);
}

#[test]
fn test_err_rgba_alpha_neg_float() {
    assert_parse_err("key: rgba(255, 255, 255, -0.5)", "alpha must be in range [0, 1]", 29, 1, 30);
}

#[test]
fn test_err_rgba_alpha_dash_paren() {
    assert_parse_err("key: rgba(255, 255, 255, -)", "number expected", 25, 1, 26);
}

#[test]
fn test_err_rgba_alpha_float_too_large() {
    assert_parse_err("key: rgba(255, 255, 255, 1.5)", "alpha must be in range [0, 1]", 28, 1, 29);
}

#[test]
fn test_err_multiline_position() {
    let err = cfg_parse("a:true\nb:").unwrap_err();
    assert_eq!(err.msg, "missing value");
    assert_eq!(err.off, 9);
    assert_eq!(err.row, 2);
    assert_eq!(err.col, 3);
}

#[test]
fn test_err_value_too_long() {
    let long_val = format!("key: \"{}\"", "x".repeat(65));
    let err = cfg_parse(&long_val).unwrap_err();
    assert_eq!(err.msg, "value too long");
    assert_eq!(err.off, 71);
    assert_eq!(err.row, 1);
    assert_eq!(err.col, 72);
}

// ===== GET FUNCTION TESTS =====

fn make_cfg(src: &str) -> Cfg {
    cfg_parse(src).unwrap()
}

#[test]
fn test_get_string_found() {
    let cfg = make_cfg("key: \"foobar\"");
    assert_eq!(cfg_get_string(&cfg, "key", "default"), "foobar");
}

#[test]
fn test_get_string_missing() {
    let cfg = make_cfg("key: \"foobar\"");
    assert_eq!(cfg_get_string(&cfg, "missing", "fallback"), "fallback");
}

#[test]
fn test_get_string_wrong_type() {
    let cfg = make_cfg("key: true");
    assert_eq!(cfg_get_string(&cfg, "key", "fallback"), "fallback");
}

#[test]
fn test_get_bool_found() {
    let cfg = make_cfg("key: true");
    assert_eq!(cfg_get_bool(&cfg, "key", false), true);
}

#[test]
fn test_get_bool_missing() {
    let cfg = make_cfg("key: true");
    assert_eq!(cfg_get_bool(&cfg, "missing", false), false);
}

#[test]
fn test_get_int_found() {
    let cfg = make_cfg("key: 14");
    assert_eq!(cfg_get_int(&cfg, "key", 0), 14);
}

#[test]
fn test_get_int_missing() {
    let cfg = make_cfg("key: 14");
    assert_eq!(cfg_get_int(&cfg, "missing", 42), 42);
}

#[test]
fn test_get_float_found() {
    let cfg = make_cfg("key: 1.5");
    assert_eq!(cfg_get_float(&cfg, "key", 0.0), 1.5);
}

#[test]
fn test_get_float_missing() {
    let cfg = make_cfg("key: 1.5");
    assert_eq!(cfg_get_float(&cfg, "missing", 3.14), 3.14);
}

#[test]
fn test_get_color_found() {
    let cfg = make_cfg("key: rgba(255, 255, 255, 1)");
    let c = cfg_get_color(&cfg, "key", CfgColor { r: 0, g: 0, b: 0, a: 0 });
    assert_eq!(c.r, 255);
    assert_eq!(c.g, 255);
    assert_eq!(c.b, 255);
    assert_eq!(c.a, 255);
}

#[test]
fn test_get_color_missing() {
    let cfg = make_cfg("key: rgba(255, 255, 255, 1)");
    let fb = CfgColor { r: 0, g: 0, b: 0, a: 0 };
    let c = cfg_get_color(&cfg, "missing", fb);
    assert_eq!(c.r, 0);
    assert_eq!(c.g, 0);
    assert_eq!(c.b, 0);
    assert_eq!(c.a, 0);
}

#[test]
fn test_get_int_duplicate_key_last_wins() {
    let cfg = make_cfg("key: 10\nkey: 20");
    assert_eq!(cfg_get_int(&cfg, "key", 0), 20);
}

// ===== INT RANGE TESTS =====

#[test]
fn test_get_int_min_in_range() {
    let cfg = make_cfg("key: 16");
    assert_eq!(cfg_get_int_min(&cfg, "key", 64, 8), 16);
}

#[test]
fn test_get_int_min_below() {
    let cfg = make_cfg("key: 16");
    assert_eq!(cfg_get_int_min(&cfg, "key", 64, 32), 64);
}

#[test]
fn test_get_int_max_in_range() {
    let cfg = make_cfg("key: 16");
    assert_eq!(cfg_get_int_max(&cfg, "key", 64, 32), 16);
}

#[test]
fn test_get_int_max_above() {
    let cfg = make_cfg("key: 16");
    assert_eq!(cfg_get_int_max(&cfg, "key", 4, 8), 4);
}

#[test]
fn test_get_int_range_in() {
    let cfg = make_cfg("key: 16");
    assert_eq!(cfg_get_int_range(&cfg, "key", 64, 8, 32), 16);
}

#[test]
fn test_get_int_range_below() {
    let cfg = make_cfg("key: 16");
    assert_eq!(cfg_get_int_range(&cfg, "key", 4, 4, 8), 4);
}

#[test]
fn test_get_int_range_above() {
    let cfg = make_cfg("key: 16");
    assert_eq!(cfg_get_int_range(&cfg, "key", 32, 32, 64), 32);
}

// ===== FLOAT RANGE TESTS =====

#[test]
fn test_get_float_min_in_range() {
    let cfg = make_cfg("key: 16.0");
    assert_eq!(cfg_get_float_min(&cfg, "key", 64.0, 8.0), 16.0);
}

#[test]
fn test_get_float_min_below() {
    let cfg = make_cfg("key: 16.0");
    assert_eq!(cfg_get_float_min(&cfg, "key", 64.0, 32.0), 64.0);
}

#[test]
fn test_get_float_max_in_range() {
    let cfg = make_cfg("key: 16.0");
    assert_eq!(cfg_get_float_max(&cfg, "key", 64.0, 32.0), 16.0);
}

#[test]
fn test_get_float_max_above() {
    let cfg = make_cfg("key: 16.0");
    assert_eq!(cfg_get_float_max(&cfg, "key", 4.0, 8.0), 4.0);
}

#[test]
fn test_get_float_range_in() {
    let cfg = make_cfg("key: 16.0");
    assert_eq!(cfg_get_float_range(&cfg, "key", 64.0, 8.0, 32.0), 16.0);
}

#[test]
fn test_get_float_range_below() {
    let cfg = make_cfg("key: 16.0");
    assert_eq!(cfg_get_float_range(&cfg, "key", 4.0, 4.0, 8.0), 4.0);
}

#[test]
fn test_get_float_range_above() {
    let cfg = make_cfg("key: 16.0");
    assert_eq!(cfg_get_float_range(&cfg, "key", 32.0, 32.0, 64.0), 32.0);
}

// ===== PARSE FILE TESTS =====

#[test]
fn test_parse_file_invalid_filename() {
    let err = cfg_parse_file("").unwrap_err();
    assert_eq!(err.msg, "invalid filename");
    assert_eq!(err.off, -1);
    assert_eq!(err.row, -1);
    assert_eq!(err.col, -1);
}

#[test]
fn test_parse_file_invalid_extension() {
    let err = cfg_parse_file("sample.txt").unwrap_err();
    assert_eq!(err.msg, "invalid file extension");
    assert_eq!(err.off, -1);
    assert_eq!(err.row, -1);
    assert_eq!(err.col, -1);
}

#[test]
fn test_parse_file_nonexistent() {
    let err = cfg_parse_file("nonexistent.cfg").unwrap_err();
    assert_eq!(err.msg, "failed to open file");
    assert_eq!(err.off, -1);
    assert_eq!(err.row, -1);
    assert_eq!(err.col, -1);
}

#[test]
fn test_parse_file_valid() {
    let cfg = cfg_parse_file("c_src/sample.cfg").unwrap();
    assert_eq!(cfg.count, 5);
    assert_eq!(cfg.entries[0].key, "font");
    assert_eq!(cfg.entries[0].val, CfgVal::String("JetBrainsMono Nerd Font".to_string()));
    assert_eq!(cfg.entries[1].key, "font.size");
    assert_eq!(cfg.entries[1].val, CfgVal::Int(14));
    assert_eq!(cfg.entries[2].key, "zoom");
    assert_eq!(cfg.entries[2].val, CfgVal::Float(1.5));
    assert_eq!(cfg.entries[3].key, "line_numbers");
    assert_eq!(cfg.entries[3].val, CfgVal::Boolean(true));
    assert_eq!(cfg.entries[4].key, "bg.color");
    assert_eq!(cfg.entries[4].val, CfgVal::Color(CfgColor { r: 255, g: 255, b: 255, a: 255 }));
}

// ===== DISPLAY / FPRINT TESTS =====

#[test]
fn test_display_cfg() {
    let src = "font: \"JetBrainsMono Nerd Font\"\n\
               font.size: 14\n\
               zoom: 1.5\n\
               line_numbers: true\n\
               ruler: false\n\
               bg.color: rgba(255, 255, 255, 1)";
    let cfg = cfg_parse(src).unwrap();
    let output = format!("{}", cfg);
    assert!(output.contains("font: \"JetBrainsMono Nerd Font\""));
    assert!(output.contains("font.size: 14"));
    assert!(output.contains("zoom: 1.500000"));
    assert!(output.contains("line_numbers: true"));
    assert!(output.contains("ruler: false"));
    assert!(output.contains("bg.color: rgba(255, 255, 255, 255)"));
}

#[test]
fn test_display_error_with_position() {
    let err = cfg_parse("a:true\nb:").unwrap_err();
    let output = format!("{}", err);
    assert_eq!(output, "Error at 2:3 :: missing value\n");
}

#[test]
fn test_display_error_without_position() {
    let err = cfg_parse_file("").unwrap_err();
    let output = format!("{}", err);
    assert_eq!(output, "Error: invalid filename\n");
}

// ===== CFGERROR DEFAULT TEST =====

#[test]
fn test_cfg_error_default() {
    let err = CfgError::default();
    assert_eq!(err.off, 0);
    assert_eq!(err.col, 0);
    assert_eq!(err.row, 0);
    assert!(err.msg.is_empty());
}

fn main() {}
