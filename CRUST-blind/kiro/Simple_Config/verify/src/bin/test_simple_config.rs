use Simple_Config::simple_config::*;

fn parse(src: &str) -> Cfg {
    cfg_parse(src).unwrap()
}

fn parse_err(src: &str) -> CfgError {
    cfg_parse(src).unwrap_err()
}

// ============================================================
// cfg_parse: success cases
// ============================================================

#[test]
fn test_parse_empty() {
    let cfg = parse("");
    assert_eq!(cfg.count, 0);
}

#[test]
fn test_parse_whitespace() {
    let cfg = parse(" ");
    assert_eq!(cfg.count, 0);
}

#[test]
fn test_parse_comment_no_newline() {
    let cfg = parse("#");
    assert_eq!(cfg.count, 0);
}

#[test]
fn test_parse_comment_with_newline() {
    let cfg = parse("#\n");
    assert_eq!(cfg.count, 0);
}

#[test]
fn test_parse_string() {
    let cfg = parse("key: \"hello, world!\"");
    assert_eq!(cfg.count, 1);
    assert_eq!(cfg.entries[0].key, "key");
    assert_eq!(cfg.entries[0].val, CfgVal::String("hello, world!".into()));
}

#[test]
fn test_parse_int_with_inline_comment() {
    let cfg = parse("key: 10 # Inline comment");
    assert_eq!(cfg.count, 1);
    assert_eq!(cfg.entries[0].val, CfgVal::Int(10));
}

#[test]
fn test_parse_negative_int() {
    let cfg = parse("key: -1");
    assert_eq!(cfg.count, 1);
    assert_eq!(cfg.entries[0].val, CfgVal::Int(-1));
}

#[test]
fn test_parse_true() {
    let cfg = parse("key: true");
    assert_eq!(cfg.count, 1);
    assert_eq!(cfg.entries[0].val, CfgVal::Boolean(true));
}

#[test]
fn test_parse_false() {
    let cfg = parse("key: false");
    assert_eq!(cfg.count, 1);
    assert_eq!(cfg.entries[0].val, CfgVal::Boolean(false));
}

#[test]
fn test_parse_float() {
    let cfg = parse("key: 0.5");
    assert_eq!(cfg.count, 1);
    assert_eq!(cfg.entries[0].val, CfgVal::Float(0.5));
}

#[test]
fn test_parse_key_with_underscore() {
    let cfg = parse("key_: true");
    assert_eq!(cfg.entries[0].key, "key_");
}

#[test]
fn test_parse_key_with_dot() {
    let cfg = parse("key.: true");
    assert_eq!(cfg.entries[0].key, "key.");
}

#[test]
fn test_parse_two_entries() {
    let cfg = parse("a: true\nb:true");
    assert_eq!(cfg.count, 2);
    assert_eq!(cfg.entries[0].key, "a");
    assert_eq!(cfg.entries[0].val, CfgVal::Boolean(true));
    assert_eq!(cfg.entries[1].key, "b");
    assert_eq!(cfg.entries[1].val, CfgVal::Boolean(true));
}

#[test]
fn test_parse_float_trailing_dot() {
    // "1." parses as float 1.0
    let cfg = parse("key: 1.");
    assert_eq!(cfg.count, 1);
    assert_eq!(cfg.entries[0].val, CfgVal::Float(1.0));
}

#[test]
fn test_parse_negative_float() {
    let cfg = parse("key: -1.5");
    assert_eq!(cfg.count, 1);
    assert_eq!(cfg.entries[0].val, CfgVal::Float(-1.5));
}

#[test]
fn test_parse_rgba_int_alpha() {
    let cfg = parse("key: rgba(255, 255, 255, 1)");
    assert_eq!(cfg.count, 1);
    assert_eq!(cfg.entries[0].val, CfgVal::Color(CfgColor { r: 255, g: 255, b: 255, a: 255 }));
}

#[test]
fn test_parse_rgba_float_alpha() {
    let cfg = parse("key: rgba(255, 255, 255, 0.5)");
    assert_eq!(cfg.count, 1);
    assert_eq!(cfg.entries[0].val, CfgVal::Color(CfgColor { r: 255, g: 255, b: 255, a: 127 }));
}

#[test]
fn test_parse_rgba_zero_alpha() {
    let cfg = parse("key: rgba(0, 0, 0, 0)");
    assert_eq!(cfg.count, 1);
    assert_eq!(cfg.entries[0].val, CfgVal::Color(CfgColor { r: 0, g: 0, b: 0, a: 0 }));
}

// ============================================================
// cfg_parse: error cases
// ============================================================

#[test]
fn test_parse_err_missing_key() {
    assert_eq!(parse_err("!").msg, "missing key");
}

#[test]
fn test_parse_err_key_too_long() {
    assert_eq!(parse_err("xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx").msg, "key too long");
}

#[test]
fn test_parse_err_colon_expected() {
    assert_eq!(parse_err("key").msg, "':' expected");
}

#[test]
fn test_parse_err_colon_expected_bang() {
    assert_eq!(parse_err("key!").msg, "':' expected");
}

#[test]
fn test_parse_err_missing_value_colon_space() {
    assert_eq!(parse_err("key  :").msg, "missing value");
}

#[test]
fn test_parse_err_missing_value_colon() {
    assert_eq!(parse_err("key:").msg, "missing value");
}

#[test]
fn test_parse_err_missing_value_colon_spaces() {
    assert_eq!(parse_err("key:  ").msg, "missing value");
}

#[test]
fn test_parse_err_missing_value_newline() {
    assert_eq!(parse_err("key:\n").msg, "missing value");
}

#[test]
fn test_parse_err_missing_value_spaces_newline() {
    assert_eq!(parse_err("key:  \n").msg, "missing value");
}

#[test]
fn test_parse_err_invalid_value() {
    assert_eq!(parse_err("key: @\n").msg, "invalid value");
}

#[test]
fn test_parse_err_closing_quote() {
    assert_eq!(parse_err("key: \"").msg, "closing '\"' expected");
}

#[test]
fn test_parse_err_closing_quote_hello() {
    assert_eq!(parse_err("key: \"hello").msg, "closing '\"' expected");
}

#[test]
fn test_parse_err_value_too_long() {
    let long_val = "x".repeat(65);
    let src = format!("key: \"{}\"", long_val);
    assert_eq!(parse_err(&src).msg, "value too long");
}

#[test]
fn test_parse_err_unexpected_char() {
    assert_eq!(parse_err("key: 10x").msg, "unexpected character 'x'");
}

#[test]
fn test_parse_err_invalid_value_dash() {
    assert_eq!(parse_err("key: -").msg, "invalid value");
}

#[test]
fn test_parse_err_invalid_literal_t() {
    assert_eq!(parse_err("key: t").msg, "invalid literal");
}

#[test]
fn test_parse_err_invalid_literal_f() {
    assert_eq!(parse_err("key: f").msg, "invalid literal");
}

#[test]
fn test_parse_err_invalid_literal_x() {
    assert_eq!(parse_err("key: x").msg, "invalid literal");
}

#[test]
fn test_parse_err_rgba_comma_expected() {
    assert_eq!(parse_err("key: rgba(").msg, "',' expected");
}

#[test]
fn test_parse_err_rgba_paren_expected() {
    assert_eq!(parse_err("key: rgba").msg, "'(' expected");
}

#[test]
fn test_parse_err_rgba_paren_expected_space() {
    assert_eq!(parse_err("key: rgba x").msg, "'(' expected");
}

#[test]
fn test_parse_err_rgba_r_invalid() {
    assert_eq!(parse_err("key: r").msg, "invalid literal");
}

#[test]
fn test_parse_err_rgba_float_rgb() {
    assert_eq!(parse_err("key: rgba(0.5").msg, "red, blue and green must be integers in range [0, 255]");
}

#[test]
fn test_parse_err_rgba_negative_rgb() {
    assert_eq!(parse_err("key: rgba(-1").msg, "red, blue and green must be integers in range [0, 255]");
}

#[test]
fn test_parse_err_rgba_over_255() {
    assert_eq!(parse_err("key: rgba(256").msg, "red, blue and green must be integers in range [0, 255]");
}

#[test]
fn test_parse_err_rgba_alpha_negative() {
    assert_eq!(parse_err("key: rgba(255, 255, 255, -1)").msg, "alpha must be in range [0, 1]");
}

#[test]
fn test_parse_err_rgba_alpha_over_1() {
    assert_eq!(parse_err("key: rgba(255, 255, 255, 2)").msg, "alpha must be in range [0, 1]");
}

#[test]
fn test_parse_err_rgba_close_paren() {
    assert_eq!(parse_err("key: rgba(255, 255, 255, 1").msg, "')' expected");
}

#[test]
fn test_parse_err_rgba_alpha_number_expected() {
    assert_eq!(parse_err("key: rgba(255, 255, 255, x").msg, "number expected");
}

#[test]
fn test_parse_err_rgba_rgb_number_expected() {
    assert_eq!(parse_err("key: rgba(x").msg, "number expected");
}

#[test]
fn test_parse_err_rgba_comma_after_rgb() {
    assert_eq!(parse_err("key: rgba(255 x").msg, "',' expected");
}

#[test]
fn test_parse_err_rgba_close_paren_after_alpha() {
    assert_eq!(parse_err("key: rgba(255, 255, 255, 1 x").msg, "')' expected");
}

#[test]
fn test_parse_err_int_too_large() {
    assert_eq!(parse_err("key: 2147483648").msg, "number too large");
}

#[test]
fn test_parse_err_float_too_many_decimals() {
    assert_eq!(parse_err("key: 0.0000000000").msg, "number too large");
}

#[test]
fn test_parse_err_float_fract_too_large() {
    assert_eq!(parse_err("key: 1.33333333333333333").msg, "number too large");
}

#[test]
fn test_parse_err_rgba_negative_float_alpha() {
    assert_eq!(parse_err("key: rgba(255, 255, 255, -0.5)").msg, "alpha must be in range [0, 1]");
}

#[test]
fn test_parse_err_rgba_alpha_dash_paren() {
    assert_eq!(parse_err("key: rgba(255, 255, 255, -)").msg, "number expected");
}

#[test]
fn test_parse_err_rgba_alpha_float_over_1() {
    assert_eq!(parse_err("key: rgba(255, 255, 255, 1.5)").msg, "alpha must be in range [0, 1]");
}

// ============================================================
// Error position
// ============================================================

#[test]
fn test_error_position() {
    let err = parse_err("a:true\nb:");
    assert_eq!(err.row, 2);
    assert_eq!(err.col, 3);
    assert_eq!(err.msg, "missing value");
}

// ============================================================
// cfg_parse_file
// ============================================================

#[test]
fn test_parse_file_invalid_filename() {
    let err = cfg_parse_file("").unwrap_err();
    assert_eq!(err.msg, "invalid filename");
}

#[test]
fn test_parse_file_invalid_extension() {
    let err = cfg_parse_file("sample.txt").unwrap_err();
    assert_eq!(err.msg, "invalid file extension");
}

#[test]
fn test_parse_file_not_found() {
    let err = cfg_parse_file("sample2.cfg").unwrap_err();
    assert_eq!(err.msg, "failed to open file");
}

#[test]
fn test_parse_file_success() {
    // Write a temp .cfg file
    let dir = std::env::temp_dir();
    let path = dir.join("test_sc.cfg");
    std::fs::write(&path, "key: 42\n").unwrap();
    let cfg = cfg_parse_file(path.to_str().unwrap()).unwrap();
    assert_eq!(cfg.count, 1);
    assert_eq!(cfg.entries[0].val, CfgVal::Int(42));
    std::fs::remove_file(&path).ok();
}

// ============================================================
// Getters
// ============================================================

#[test]
fn test_get_string_found() {
    let cfg = parse("keyA: \"foobar\"");
    assert_eq!(cfg_get_string(&cfg, "keyA", "barfoo"), "foobar");
}

#[test]
fn test_get_string_not_found() {
    let cfg = parse("keyA: \"foobar\"");
    assert_eq!(cfg_get_string(&cfg, "keyB", "barfoo"), "barfoo");
}

#[test]
fn test_get_string_type_mismatch() {
    let cfg = parse("keyC: true");
    assert_eq!(cfg_get_string(&cfg, "keyC", "barfoo"), "barfoo");
}

#[test]
fn test_get_bool() {
    let cfg = parse("key: true");
    assert_eq!(cfg_get_bool(&cfg, "key", false), true);
    assert_eq!(cfg_get_bool(&cfg, "missing", false), false);
}

#[test]
fn test_get_int() {
    let cfg = parse("key: 16");
    assert_eq!(cfg_get_int(&cfg, "key", 64), 16);
    assert_eq!(cfg_get_int(&cfg, "missing", 64), 64);
}

#[test]
fn test_get_int_min() {
    let cfg = parse("key: 16");
    assert_eq!(cfg_get_int_min(&cfg, "key", 64, 8), 16);   // 16 >= 8, ok
    assert_eq!(cfg_get_int_min(&cfg, "key", 64, 32), 64);  // 16 < 32, fallback
}

#[test]
fn test_get_int_max() {
    let cfg = parse("key: 16");
    assert_eq!(cfg_get_int_max(&cfg, "key", 64, 32), 16);  // 16 <= 32, ok
    assert_eq!(cfg_get_int_max(&cfg, "key", 4, 8), 4);     // 16 > 8, fallback
}

#[test]
fn test_get_int_range() {
    let cfg = parse("key: 16");
    assert_eq!(cfg_get_int_range(&cfg, "key", 64, 8, 32), 16);   // in range
    assert_eq!(cfg_get_int_range(&cfg, "key", 4, 4, 8), 4);      // 16 > 8, fallback
    assert_eq!(cfg_get_int_range(&cfg, "key", 32, 32, 64), 32);  // 16 < 32, fallback
}

#[test]
fn test_get_float() {
    let cfg = parse("key: 16.0");
    assert_eq!(cfg_get_float(&cfg, "key", 64.0), 16.0);
    assert_eq!(cfg_get_float(&cfg, "missing", 64.0), 64.0);
}

#[test]
fn test_get_float_min() {
    let cfg = parse("key: 16.0");
    assert_eq!(cfg_get_float_min(&cfg, "key", 64.0, 8.0), 16.0);
    assert_eq!(cfg_get_float_min(&cfg, "key", 64.0, 32.0), 64.0);
}

#[test]
fn test_get_float_max() {
    let cfg = parse("key: 16.0");
    assert_eq!(cfg_get_float_max(&cfg, "key", 64.0, 32.0), 16.0);
    assert_eq!(cfg_get_float_max(&cfg, "key", 4.0, 8.0), 4.0);
}

#[test]
fn test_get_float_range() {
    let cfg = parse("key: 16.0");
    assert_eq!(cfg_get_float_range(&cfg, "key", 64.0, 8.0, 32.0), 16.0);
    assert_eq!(cfg_get_float_range(&cfg, "key", 4.0, 4.0, 8.0), 4.0);
    assert_eq!(cfg_get_float_range(&cfg, "key", 32.0, 32.0, 64.0), 32.0);
}

#[test]
fn test_get_color() {
    let cfg = parse("key: rgba(255, 255, 255, 1)");
    let fallback = CfgColor { r: 0, g: 0, b: 0, a: 0 };
    let c = cfg_get_color(&cfg, "key", fallback);
    assert_eq!(c, CfgColor { r: 255, g: 255, b: 255, a: 255 });
}

#[test]
fn test_get_color_fallback() {
    let cfg = parse("key: 42");
    let fallback = CfgColor { r: 1, g: 2, b: 3, a: 4 };
    assert_eq!(cfg_get_color(&cfg, "key", fallback), fallback);
}

// Last match wins (search backwards)
#[test]
fn test_get_last_match_wins() {
    let cfg = parse("key: 1\nkey: 2");
    assert_eq!(cfg_get_int(&cfg, "key", 0), 2);
}

// ============================================================
// cfg_fprint
// ============================================================

#[test]
fn test_fprint() {
    let cfg = parse(
        "font: \"JetBrainsMono Nerd Font\"\n\
         font.size: 14\n\
         zoom: 1.5\n\
         line_numbers: true\n\
         ruler: false\n\
         bg.color: rgba(255, 255, 255, 1)"
    );

    let expected = "font: \"JetBrainsMono Nerd Font\"\n\
                    font.size: 14\n\
                    zoom: 1.500000\n\
                    line_numbers: true\n\
                    ruler: false\n\
                    bg.color: rgba(255, 255, 255, 255)\n";

    let dir = std::env::temp_dir();
    let path = dir.join("test_fprint_out.txt");
    {
        let mut f = std::fs::File::create(&path).unwrap();
        cfg_fprint(&mut f, &cfg);
    }
    let output = std::fs::read_to_string(&path).unwrap();
    assert_eq!(output, expected);
    std::fs::remove_file(&path).ok();
}

// ============================================================
// cfg_fprint_error
// ============================================================

#[test]
fn test_fprint_error_no_position() {
    let err = CfgError { off: -1, col: -1, row: -1, msg: "invalid filename".into() };
    let dir = std::env::temp_dir();
    let path = dir.join("test_fprint_err1.txt");
    {
        let mut f = std::fs::File::create(&path).unwrap();
        cfg_fprint_error(&mut f, &err);
    }
    let output = std::fs::read_to_string(&path).unwrap();
    assert_eq!(output, "Error: invalid filename\n");
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_fprint_error_with_position() {
    let err = parse_err("a:true\nb:");
    let dir = std::env::temp_dir();
    let path = dir.join("test_fprint_err2.txt");
    {
        let mut f = std::fs::File::create(&path).unwrap();
        cfg_fprint_error(&mut f, &err);
    }
    let output = std::fs::read_to_string(&path).unwrap();
    assert_eq!(output, "Error at 2:3 :: missing value\n");
    std::fs::remove_file(&path).ok();
}

// ============================================================
// Display trait (matches cfg_fprint output format)
// ============================================================

#[test]
fn test_display_cfg() {
    let cfg = parse("key: 1.5\nflag: true");
    let s = format!("{}", cfg);
    assert!(s.contains("key: 1.500000"));
    assert!(s.contains("flag: true"));
}

#[test]
fn test_display_error_no_pos() {
    let err = CfgError { off: -1, col: -1, row: -1, msg: "test".into() };
    assert_eq!(format!("{}", err), "Error: test\n");
}

#[test]
fn test_display_error_with_pos() {
    let err = CfgError { off: 5, col: 3, row: 2, msg: "oops".into() };
    assert_eq!(format!("{}", err), "Error at 2:3 :: oops\n");
}

// ============================================================
// Multi-line with comments and whitespace
// ============================================================

#[test]
fn test_parse_multiline_with_comments() {
    let src = "# header\n\nkey: 42\n\n# footer\n";
    let cfg = parse(src);
    assert_eq!(cfg.count, 1);
    assert_eq!(cfg.entries[0].val, CfgVal::Int(42));
}

fn main() {}
