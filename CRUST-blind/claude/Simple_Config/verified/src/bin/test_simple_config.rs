use Simple_Config::simple_config::{
    cfg_get_bool, cfg_get_color, cfg_get_float, cfg_get_float_max, cfg_get_float_min,
    cfg_get_float_range, cfg_get_int, cfg_get_int_max, cfg_get_int_min, cfg_get_int_range,
    cfg_get_string, cfg_parse, cfg_parse_file, CfgColor, CfgVal, CFG_FILE_EXT, CFG_MAX_ERR,
    CFG_MAX_KEY, CFG_MAX_VAL,
};

// ---- Constants ----

#[test]
fn test_constants() {
    assert_eq!(CFG_FILE_EXT, ".cfg");
    assert_eq!(CFG_MAX_KEY, 32);
    assert_eq!(CFG_MAX_VAL, 64);
    assert_eq!(CFG_MAX_ERR, 64);
}

// ---- cfg_parse: success cases ----

#[test]
fn test_parse_simple_string() {
    let cfg = cfg_parse("name: \"hello\"\n").expect("parse");
    assert_eq!(cfg.count, 1);
    assert_eq!(cfg.entries.len(), 1);
    assert_eq!(cfg.entries[0].key, "name");
    assert_eq!(cfg.entries[0].val, CfgVal::String("hello".to_string()));
}

#[test]
fn test_parse_simple_int() {
    let cfg = cfg_parse("x: 42\n").expect("parse");
    assert_eq!(cfg.count, 1);
    assert_eq!(cfg.entries[0].key, "x");
    assert_eq!(cfg.entries[0].val, CfgVal::Int(42));
}

#[test]
fn test_parse_negative_int() {
    let cfg = cfg_parse("x: -42\n").expect("parse");
    assert_eq!(cfg.count, 1);
    assert_eq!(cfg.entries[0].val, CfgVal::Int(-42));
}

#[test]
fn test_parse_simple_float() {
    let cfg = cfg_parse("x: 3.14\n").expect("parse");
    assert_eq!(cfg.count, 1);
    match cfg.entries[0].val {
        CfgVal::Float(f) => {
            // Match the exact C arithmetic: 3 + 14/100 in float
            let expected = 3i32 as f32 + (14i32 as f32 / 100i32 as f32);
            assert_eq!(f, expected);
        }
        ref v => panic!("expected Float, got {:?}", v),
    }
}

#[test]
fn test_parse_negative_float() {
    let cfg = cfg_parse("x: -2.5\n").expect("parse");
    assert_eq!(cfg.count, 1);
    match cfg.entries[0].val {
        CfgVal::Float(f) => assert_eq!(f, -2.5_f32),
        ref v => panic!("expected Float, got {:?}", v),
    }
}

#[test]
fn test_parse_bool_true() {
    let cfg = cfg_parse("flag: true\n").expect("parse");
    assert_eq!(cfg.entries[0].key, "flag");
    assert_eq!(cfg.entries[0].val, CfgVal::Boolean(true));
}

#[test]
fn test_parse_bool_false() {
    let cfg = cfg_parse("flag: false\n").expect("parse");
    assert_eq!(cfg.entries[0].val, CfgVal::Boolean(false));
}

#[test]
fn test_parse_rgba_int_alpha() {
    let cfg = cfg_parse("color: rgba(255, 128, 64, 1)\n").expect("parse");
    assert_eq!(cfg.entries[0].key, "color");
    assert_eq!(
        cfg.entries[0].val,
        CfgVal::Color(CfgColor {
            r: 255,
            g: 128,
            b: 64,
            a: 255,
        })
    );
}

#[test]
fn test_parse_rgba_float_alpha() {
    let cfg = cfg_parse("color: rgba(255, 128, 64, 0.5)\n").expect("parse");
    assert_eq!(
        cfg.entries[0].val,
        CfgVal::Color(CfgColor {
            r: 255,
            g: 128,
            b: 64,
            a: 127,
        })
    );
}

#[test]
fn test_parse_rgba_alpha_zero() {
    let cfg = cfg_parse("color: rgba(0, 0, 0, 0)\n").expect("parse");
    assert_eq!(
        cfg.entries[0].val,
        CfgVal::Color(CfgColor {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        })
    );
}

#[test]
fn test_parse_multiple_entries_with_comments() {
    let cfg = cfg_parse("# a comment\nname: \"hello\"\n# another\nx: 100\n").expect("parse");
    assert_eq!(cfg.count, 2);
    assert_eq!(cfg.entries.len(), 2);
    assert_eq!(cfg.entries[0].key, "name");
    assert_eq!(cfg.entries[0].val, CfgVal::String("hello".to_string()));
    assert_eq!(cfg.entries[1].key, "x");
    assert_eq!(cfg.entries[1].val, CfgVal::Int(100));
}

#[test]
fn test_parse_empty_input() {
    let cfg = cfg_parse("").expect("parse");
    assert_eq!(cfg.count, 0);
    assert_eq!(cfg.entries.len(), 0);
}

#[test]
fn test_parse_whitespace_only() {
    let cfg = cfg_parse("   \n\t  \n  ").expect("parse");
    assert_eq!(cfg.count, 0);
    assert_eq!(cfg.entries.len(), 0);
}

#[test]
fn test_parse_comment_only() {
    let cfg = cfg_parse("# only a comment\n").expect("parse");
    assert_eq!(cfg.count, 0);
}

#[test]
fn test_parse_trailing_comment_after_value() {
    let cfg = cfg_parse("x: 42 # trailing\n").expect("parse");
    assert_eq!(cfg.count, 1);
    assert_eq!(cfg.entries[0].val, CfgVal::Int(42));
}

#[test]
fn test_parse_leading_whitespace() {
    let cfg = cfg_parse("\n\n  x: 42\n").expect("parse");
    assert_eq!(cfg.count, 1);
    assert_eq!(cfg.entries[0].key, "x");
    assert_eq!(cfg.entries[0].val, CfgVal::Int(42));
}

#[test]
fn test_parse_dotted_key() {
    let cfg = cfg_parse("font.size: 12\n").expect("parse");
    assert_eq!(cfg.entries[0].key, "font.size");
    assert_eq!(cfg.entries[0].val, CfgVal::Int(12));
}

#[test]
fn test_parse_underscore_key() {
    let cfg = cfg_parse("line_num: true\n").expect("parse");
    assert_eq!(cfg.entries[0].key, "line_num");
    assert_eq!(cfg.entries[0].val, CfgVal::Boolean(true));
}

#[test]
fn test_parse_max_length_string() {
    let val: String = std::iter::repeat('a').take(64).collect();
    let src = format!("name: \"{}\"\n", val);
    let cfg = cfg_parse(&src).expect("parse");
    assert_eq!(cfg.count, 1);
    assert_eq!(cfg.entries[0].val, CfgVal::String(val));
}

#[test]
fn test_parse_max_length_key() {
    let key: String = std::iter::repeat('a').take(32).collect();
    let src = format!("{}: 1\n", key);
    let cfg = cfg_parse(&src).expect("parse");
    assert_eq!(cfg.count, 1);
    assert_eq!(cfg.entries[0].key, key);
    assert_eq!(cfg.entries[0].val, CfgVal::Int(1));
}

#[test]
fn test_parse_rgba_with_whitespace() {
    let cfg = cfg_parse("c: rgba(  10 , 20 , 30 , 1  )\n").expect("parse");
    assert_eq!(
        cfg.entries[0].val,
        CfgVal::Color(CfgColor {
            r: 10,
            g: 20,
            b: 30,
            a: 255
        })
    );
}

#[test]
fn test_parse_float_zero() {
    let cfg = cfg_parse("x: 0.0\n").expect("parse");
    match cfg.entries[0].val {
        CfgVal::Float(f) => assert_eq!(f, 0.0_f32),
        ref v => panic!("expected Float, got {:?}", v),
    }
}

#[test]
fn test_parse_float_no_fractional_digits() {
    // "5." is valid; int_part=5, fract_part=0, div=1 -> 5.0
    let cfg = cfg_parse("x: 5.\n").expect("parse");
    match cfg.entries[0].val {
        CfgVal::Float(f) => assert_eq!(f, 5.0_f32),
        ref v => panic!("expected Float, got {:?}", v),
    }
}

#[test]
fn test_parse_duplicate_keys_count() {
    let cfg = cfg_parse("x: 1\nx: 2\nx: 3\n").expect("parse");
    assert_eq!(cfg.count, 3);
    assert_eq!(cfg.entries.len(), 3);
    assert_eq!(cfg.entries[0].val, CfgVal::Int(1));
    assert_eq!(cfg.entries[1].val, CfgVal::Int(2));
    assert_eq!(cfg.entries[2].val, CfgVal::Int(3));
}

#[test]
fn test_parse_full_sample_like() {
    let src = "# A sample config file\n\
               font: \"JetBrainsMono Nerd Font\"\n\
               font.size: 14\n\
               zoom: 1.5\n\
               line_numbers: true\n\
               bg.color: rgba(255, 255, 255, 1)\n";
    let cfg = cfg_parse(src).expect("parse");
    assert_eq!(cfg.count, 5);
    assert_eq!(cfg.entries[0].key, "font");
    assert_eq!(
        cfg.entries[0].val,
        CfgVal::String("JetBrainsMono Nerd Font".to_string())
    );
    assert_eq!(cfg.entries[1].key, "font.size");
    assert_eq!(cfg.entries[1].val, CfgVal::Int(14));
    assert_eq!(cfg.entries[2].key, "zoom");
    match cfg.entries[2].val {
        CfgVal::Float(f) => assert_eq!(f, 1.5_f32),
        ref v => panic!("expected Float, got {:?}", v),
    }
    assert_eq!(cfg.entries[3].key, "line_numbers");
    assert_eq!(cfg.entries[3].val, CfgVal::Boolean(true));
    assert_eq!(cfg.entries[4].key, "bg.color");
    assert_eq!(
        cfg.entries[4].val,
        CfgVal::Color(CfgColor {
            r: 255,
            g: 255,
            b: 255,
            a: 255,
        })
    );
}

// ---- cfg_parse: error cases ----

#[test]
fn test_parse_missing_colon() {
    let err = cfg_parse("name \"hello\"\n").unwrap_err();
    assert_eq!(err.off, 5);
    assert_eq!(err.row, 1);
    assert_eq!(err.col, 6);
    assert_eq!(err.msg, "':' expected");
}

#[test]
fn test_parse_missing_key() {
    let err = cfg_parse(": \"hello\"\n").unwrap_err();
    assert_eq!(err.off, 0);
    assert_eq!(err.row, 1);
    assert_eq!(err.col, 1);
    assert_eq!(err.msg, "missing key");
}

#[test]
fn test_parse_missing_value() {
    let err = cfg_parse("name:\n").unwrap_err();
    assert_eq!(err.off, 5);
    assert_eq!(err.row, 1);
    assert_eq!(err.col, 6);
    assert_eq!(err.msg, "missing value");
}

#[test]
fn test_parse_missing_closing_quote() {
    let err = cfg_parse("name: \"hello\n").unwrap_err();
    assert_eq!(err.off, 12);
    assert_eq!(err.row, 1);
    assert_eq!(err.col, 13);
    assert_eq!(err.msg, "closing '\"' expected");
}

#[test]
fn test_parse_value_too_long() {
    let val: String = std::iter::repeat('a').take(65).collect();
    let src = format!("name: \"{}\"\n", val);
    let err = cfg_parse(&src).unwrap_err();
    assert_eq!(err.off, 72);
    assert_eq!(err.row, 1);
    assert_eq!(err.col, 73);
    assert_eq!(err.msg, "value too long");
}

#[test]
fn test_parse_key_too_long() {
    let key: String = std::iter::repeat('a').take(33).collect();
    let src = format!("{}: 1\n", key);
    let err = cfg_parse(&src).unwrap_err();
    assert_eq!(err.off, 33);
    assert_eq!(err.row, 1);
    assert_eq!(err.col, 34);
    assert_eq!(err.msg, "key too long");
}

#[test]
fn test_parse_invalid_value_char() {
    let err = cfg_parse("name: ?\n").unwrap_err();
    assert_eq!(err.off, 6);
    assert_eq!(err.row, 1);
    assert_eq!(err.col, 7);
    assert_eq!(err.msg, "invalid value");
}

#[test]
fn test_parse_bad_literal() {
    let err = cfg_parse("name: nope\n").unwrap_err();
    assert_eq!(err.off, 6);
    assert_eq!(err.row, 1);
    assert_eq!(err.col, 7);
    assert_eq!(err.msg, "invalid literal");
}

#[test]
fn test_parse_rgba_missing_paren() {
    let err = cfg_parse("c: rgba 255, 0, 0, 1)\n").unwrap_err();
    assert_eq!(err.off, 8);
    assert_eq!(err.row, 1);
    assert_eq!(err.col, 9);
    assert_eq!(err.msg, "'(' expected");
}

#[test]
fn test_parse_rgba_red_out_of_range() {
    let err = cfg_parse("c: rgba(256, 0, 0, 1)\n").unwrap_err();
    assert_eq!(err.off, 11);
    assert_eq!(err.row, 1);
    assert_eq!(err.col, 12);
    assert_eq!(
        err.msg,
        "red, blue and green must be integers in range [0, 255]"
    );
}

#[test]
fn test_parse_rgba_alpha_out_of_range() {
    let err = cfg_parse("c: rgba(0, 0, 0, 2)\n").unwrap_err();
    assert_eq!(err.off, 18);
    assert_eq!(err.row, 1);
    assert_eq!(err.col, 19);
    assert_eq!(err.msg, "alpha must be in range [0, 1]");
}

#[test]
fn test_parse_rgba_negative_red() {
    let err = cfg_parse("c: rgba(-1, 0, 0, 1)\n").unwrap_err();
    assert_eq!(err.off, 10);
    assert_eq!(err.row, 1);
    assert_eq!(err.col, 11);
    assert_eq!(
        err.msg,
        "red, blue and green must be integers in range [0, 255]"
    );
}

#[test]
fn test_parse_rgba_float_red() {
    let err = cfg_parse("c: rgba(1.0, 0, 0, 1)\n").unwrap_err();
    assert_eq!(err.off, 8);
    assert_eq!(err.row, 1);
    assert_eq!(err.col, 9);
    assert_eq!(
        err.msg,
        "red, blue and green must be integers in range [0, 255]"
    );
}

#[test]
fn test_parse_rgba_alpha_float_negative() {
    let err = cfg_parse("c: rgba(0, 0, 0, -0.5)\n").unwrap_err();
    assert_eq!(err.off, 21);
    assert_eq!(err.row, 1);
    assert_eq!(err.col, 22);
    assert_eq!(err.msg, "alpha must be in range [0, 1]");
}

#[test]
fn test_parse_rgba_alpha_float_over_1() {
    let err = cfg_parse("c: rgba(0, 0, 0, 1.5)\n").unwrap_err();
    assert_eq!(err.off, 20);
    assert_eq!(err.row, 1);
    assert_eq!(err.col, 21);
    assert_eq!(err.msg, "alpha must be in range [0, 1]");
}

#[test]
fn test_parse_int_overflow() {
    let err = cfg_parse("x: 99999999999999999999\n").unwrap_err();
    assert_eq!(err.off, 13);
    assert_eq!(err.row, 1);
    assert_eq!(err.col, 14);
    assert_eq!(err.msg, "number too large");
}

#[test]
fn test_parse_unexpected_after_value() {
    let err = cfg_parse("x: 42 y: 100\n").unwrap_err();
    assert_eq!(err.off, 6);
    assert_eq!(err.row, 1);
    assert_eq!(err.col, 7);
    assert_eq!(err.msg, "unexpected character 'y'");
}

#[test]
fn test_parse_row_col_tracking_multiline_error() {
    let err = cfg_parse("a: 1\nb: 2\nbad ?\n").unwrap_err();
    assert_eq!(err.off, 14);
    assert_eq!(err.row, 3);
    assert_eq!(err.col, 5);
    assert_eq!(err.msg, "':' expected");
}

#[test]
fn test_parse_rgba_close_paren_missing() {
    let err = cfg_parse("c: rgba(255, 0, 0, 1\n").unwrap_err();
    assert_eq!(err.off, 20);
    assert_eq!(err.row, 1);
    assert_eq!(err.col, 21);
    assert_eq!(err.msg, "')' expected");
}

#[test]
fn test_parse_trailing_junk_after_value() {
    let err = cfg_parse("x: 1 garbage\n").unwrap_err();
    assert_eq!(err.off, 5);
    assert_eq!(err.row, 1);
    assert_eq!(err.col, 6);
    assert_eq!(err.msg, "unexpected character 'g'");
}

// ---- cfg_get_string ----

#[test]
fn test_cfg_get_string_basic() {
    let cfg = cfg_parse("a: \"first\"\nb: 5\n").expect("parse");
    assert_eq!(cfg_get_string(&cfg, "a", "x"), "first");
}

#[test]
fn test_cfg_get_string_missing_returns_fallback() {
    let cfg = cfg_parse("a: \"first\"\nb: 5\n").expect("parse");
    assert_eq!(cfg_get_string(&cfg, "missing", "default"), "default");
}

#[test]
fn test_cfg_get_string_type_mismatch_returns_fallback() {
    // 'b' is an Int, not a String; should fall back
    let cfg = cfg_parse("a: \"first\"\nb: 5\n").expect("parse");
    assert_eq!(cfg_get_string(&cfg, "b", "x"), "x");
}

// ---- cfg_get_bool ----

#[test]
fn test_cfg_get_bool_basic() {
    let cfg = cfg_parse("flag: true\n").expect("parse");
    assert_eq!(cfg_get_bool(&cfg, "flag", false), true);
}

#[test]
fn test_cfg_get_bool_missing_returns_fallback() {
    let cfg = cfg_parse("flag: true\n").expect("parse");
    assert_eq!(cfg_get_bool(&cfg, "missing", false), false);
    assert_eq!(cfg_get_bool(&cfg, "missing", true), true);
}

#[test]
fn test_cfg_get_bool_type_mismatch_returns_fallback() {
    let cfg = cfg_parse("x: 1\n").expect("parse");
    assert_eq!(cfg_get_bool(&cfg, "x", true), true);
    assert_eq!(cfg_get_bool(&cfg, "x", false), false);
}

// ---- cfg_get_int and friends ----

#[test]
fn test_cfg_get_int_basic() {
    let cfg = cfg_parse("x: 42\n").expect("parse");
    assert_eq!(cfg_get_int(&cfg, "x", -1), 42);
}

#[test]
fn test_cfg_get_int_missing() {
    let cfg = cfg_parse("x: 42\n").expect("parse");
    assert_eq!(cfg_get_int(&cfg, "missing", 99), 99);
}

#[test]
fn test_cfg_get_int_type_mismatch() {
    // Float value, requesting Int -> should fall back
    let cfg = cfg_parse("x: 3.14\n").expect("parse");
    assert_eq!(cfg_get_int(&cfg, "x", 99), 99);
}

#[test]
fn test_cfg_get_int_duplicates_last_wins() {
    let cfg = cfg_parse("x: 1\nx: 2\nx: 3\n").expect("parse");
    assert_eq!(cfg_get_int(&cfg, "x", -1), 3);
}

#[test]
fn test_cfg_get_int_min() {
    let cfg = cfg_parse("x: 10\n").expect("parse");
    assert_eq!(cfg_get_int_min(&cfg, "x", 99, 5), 10);
    assert_eq!(cfg_get_int_min(&cfg, "x", 99, 50), 99);
}

#[test]
fn test_cfg_get_int_max() {
    let cfg = cfg_parse("x: 10\n").expect("parse");
    assert_eq!(cfg_get_int_max(&cfg, "x", 99, 5), 99);
    assert_eq!(cfg_get_int_max(&cfg, "x", 99, 50), 10);
}

#[test]
fn test_cfg_get_int_range() {
    let cfg = cfg_parse("x: 10\n").expect("parse");
    assert_eq!(cfg_get_int_range(&cfg, "x", 99, 0, 100), 10);
    assert_eq!(cfg_get_int_range(&cfg, "x", 99, 100, 200), 99);
    assert_eq!(cfg_get_int_range(&cfg, "x", 99, 0, 5), 99);
}

// ---- cfg_get_float and friends ----

#[test]
fn test_cfg_get_float_basic() {
    let cfg = cfg_parse("x: 1.5\n").expect("parse");
    assert_eq!(cfg_get_float(&cfg, "x", -1.0), 1.5_f32);
}

#[test]
fn test_cfg_get_float_missing() {
    let cfg = cfg_parse("x: 1.5\n").expect("parse");
    assert_eq!(cfg_get_float(&cfg, "missing", 9.9_f32), 9.9_f32);
}

#[test]
fn test_cfg_get_float_type_mismatch() {
    // Int value, requesting Float -> should fall back
    let cfg = cfg_parse("x: 5\n").expect("parse");
    assert_eq!(cfg_get_float(&cfg, "x", 9.9_f32), 9.9_f32);
}

#[test]
fn test_cfg_get_float_min() {
    let cfg = cfg_parse("x: 1.5\n").expect("parse");
    assert_eq!(cfg_get_float_min(&cfg, "x", 9.9_f32, 0.5_f32), 1.5_f32);
    assert_eq!(cfg_get_float_min(&cfg, "x", 9.9_f32, 5.5_f32), 9.9_f32);
}

#[test]
fn test_cfg_get_float_max() {
    let cfg = cfg_parse("x: 1.5\n").expect("parse");
    assert_eq!(cfg_get_float_max(&cfg, "x", 9.9_f32, 0.5_f32), 9.9_f32);
    assert_eq!(cfg_get_float_max(&cfg, "x", 9.9_f32, 5.5_f32), 1.5_f32);
}

#[test]
fn test_cfg_get_float_range() {
    let cfg = cfg_parse("x: 1.5\n").expect("parse");
    assert_eq!(cfg_get_float_range(&cfg, "x", 9.9_f32, 0.0_f32, 10.0_f32), 1.5_f32);
    assert_eq!(cfg_get_float_range(&cfg, "x", 9.9_f32, 5.0_f32, 10.0_f32), 9.9_f32);
}

// ---- cfg_get_color ----

#[test]
fn test_cfg_get_color_basic() {
    let cfg = cfg_parse("c: rgba(10, 20, 30, 1)\n").expect("parse");
    let fallback = CfgColor { r: 0, g: 0, b: 0, a: 0 };
    let got = cfg_get_color(&cfg, "c", fallback);
    assert_eq!(got.r, 10);
    assert_eq!(got.g, 20);
    assert_eq!(got.b, 30);
    assert_eq!(got.a, 255);
}

#[test]
fn test_cfg_get_color_missing() {
    let cfg = cfg_parse("c: rgba(10, 20, 30, 1)\n").expect("parse");
    let fallback = CfgColor { r: 1, g: 2, b: 3, a: 4 };
    let got = cfg_get_color(&cfg, "missing", fallback);
    assert_eq!(got.r, 1);
    assert_eq!(got.g, 2);
    assert_eq!(got.b, 3);
    assert_eq!(got.a, 4);
}

#[test]
fn test_cfg_get_color_type_mismatch() {
    let cfg = cfg_parse("x: 1\n").expect("parse");
    let fallback = CfgColor { r: 9, g: 8, b: 7, a: 6 };
    let got = cfg_get_color(&cfg, "x", fallback);
    assert_eq!(got.r, 9);
    assert_eq!(got.g, 8);
    assert_eq!(got.b, 7);
    assert_eq!(got.a, 6);
}

// ---- cfg_parse_file ----

#[test]
fn test_cfg_parse_file_invalid_extension() {
    let err = cfg_parse_file("foo.txt").unwrap_err();
    assert_eq!(err.off, -1);
    assert_eq!(err.row, -1);
    assert_eq!(err.col, -1);
    assert_eq!(err.msg, "invalid file extension");
}

#[test]
fn test_cfg_parse_file_short_name() {
    let err = cfg_parse_file("ab").unwrap_err();
    assert_eq!(err.off, -1);
    assert_eq!(err.row, -1);
    assert_eq!(err.col, -1);
    assert_eq!(err.msg, "invalid filename");
}

#[test]
fn test_cfg_parse_file_missing() {
    let err = cfg_parse_file("does_not_exist.cfg").unwrap_err();
    assert_eq!(err.off, -1);
    assert_eq!(err.row, -1);
    assert_eq!(err.col, -1);
    assert_eq!(err.msg, "failed to open file");
}

#[test]
fn test_cfg_parse_file_ok() {
    // Write a temporary .cfg file and parse it.
    let tmp_path = std::env::temp_dir().join("test_simple_config_sample.cfg");
    std::fs::write(
        &tmp_path,
        "# A sample config file\n\
         font: \"JetBrainsMono Nerd Font\"\n\
         font.size: 14\n\
         zoom: 1.5\n\
         line_numbers: true\n\
         bg.color: rgba(255, 255, 255, 1)\n",
    )
    .expect("write tmp file");

    let path_str = tmp_path.to_str().expect("path utf8");
    let cfg = cfg_parse_file(path_str).expect("parse_file");
    assert_eq!(cfg.count, 5);
    assert_eq!(cfg.entries[0].key, "font");
    assert_eq!(
        cfg.entries[0].val,
        CfgVal::String("JetBrainsMono Nerd Font".to_string())
    );
    assert_eq!(cfg.entries[1].key, "font.size");
    assert_eq!(cfg.entries[1].val, CfgVal::Int(14));
    assert_eq!(cfg.entries[2].key, "zoom");
    match cfg.entries[2].val {
        CfgVal::Float(f) => assert_eq!(f, 1.5_f32),
        ref v => panic!("expected Float, got {:?}", v),
    }
    assert_eq!(cfg.entries[3].key, "line_numbers");
    assert_eq!(cfg.entries[3].val, CfgVal::Boolean(true));
    assert_eq!(cfg.entries[4].key, "bg.color");
    assert_eq!(
        cfg.entries[4].val,
        CfgVal::Color(CfgColor {
            r: 255,
            g: 255,
            b: 255,
            a: 255,
        })
    );

    let _ = std::fs::remove_file(&tmp_path);
}

// ---- Display / formatting ----

#[test]
fn test_cfg_display_format() {
    let cfg = cfg_parse(
        "a: \"hi\"\n\
         b: true\n\
         c: 42\n\
         d: 1.5\n\
         e: rgba(1, 2, 3, 1)\n",
    )
    .expect("parse");
    let s = format!("{}", cfg);
    let expected = "a: \"hi\"\n\
                    b: true\n\
                    c: 42\n\
                    d: 1.500000\n\
                    e: rgba(1, 2, 3, 255)\n";
    assert_eq!(s, expected);
}

#[test]
fn test_cfg_error_display_with_position() {
    let err = cfg_parse(": x\n").unwrap_err();
    let s = format!("{}", err);
    assert_eq!(s, "Error at 1:1 :: missing key");
}

#[test]
fn test_cfg_error_display_no_position() {
    let err = cfg_parse_file("foo.txt").unwrap_err();
    let s = format!("{}", err);
    assert_eq!(s, "Error: invalid file extension");
}

#[test]
fn test_cfg_color_display() {
    let c = CfgColor { r: 1, g: 2, b: 3, a: 4 };
    assert_eq!(format!("{}", c), "rgba(1, 2, 3, 4)");
}

// ---- From conversions ----

#[test]
fn test_cfgval_from_str() {
    let v: CfgVal = "abc".into();
    assert_eq!(v, CfgVal::String("abc".to_string()));
}

#[test]
fn test_cfgval_from_string() {
    let v: CfgVal = String::from("xyz").into();
    assert_eq!(v, CfgVal::String("xyz".to_string()));
}

#[test]
fn test_cfgval_from_bool() {
    let v: CfgVal = true.into();
    assert_eq!(v, CfgVal::Boolean(true));
}

#[test]
fn test_cfgval_from_i32() {
    let v: CfgVal = 7i32.into();
    assert_eq!(v, CfgVal::Int(7));
}

#[test]
fn test_cfgval_from_f32() {
    let v: CfgVal = 1.25_f32.into();
    assert_eq!(v, CfgVal::Float(1.25_f32));
}

#[test]
fn test_cfgval_from_color() {
    let c = CfgColor { r: 1, g: 2, b: 3, a: 4 };
    let v: CfgVal = c.into();
    assert_eq!(v, CfgVal::Color(c));
}

#[test]
fn test_cfgcolor_from_tuple() {
    let c: CfgColor = (10, 20, 30, 40).into();
    assert_eq!(c, CfgColor { r: 10, g: 20, b: 30, a: 40 });
}

fn main() {}
