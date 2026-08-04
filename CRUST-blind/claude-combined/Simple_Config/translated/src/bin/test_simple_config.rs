use Simple_Config::simple_config::*;

// ============================================================
// Successful parsing
// ============================================================

#[test]
fn test_parse_empty() {
    let cfg = cfg_parse("").unwrap();
    assert_eq!(cfg.count, 0);
    assert_eq!(cfg.entries.len(), 0);
}

#[test]
fn test_parse_only_whitespace() {
    let cfg = cfg_parse(" ").unwrap();
    assert_eq!(cfg.count, 0);
    assert_eq!(cfg.entries.len(), 0);
}

#[test]
fn test_parse_only_comment_no_newline() {
    let cfg = cfg_parse("#").unwrap();
    assert_eq!(cfg.count, 0);
    assert_eq!(cfg.entries.len(), 0);
}

#[test]
fn test_parse_only_comment_with_newline() {
    let cfg = cfg_parse("#\n").unwrap();
    assert_eq!(cfg.count, 0);
}

#[test]
fn test_parse_comment_then_entry() {
    let cfg = cfg_parse("# comment\nkey: 10").unwrap();
    assert_eq!(cfg.count, 1);
    assert_eq!(cfg.entries[0].key, "key");
    assert_eq!(cfg.entries[0].val, CfgVal::Int(10));
}

#[test]
fn test_parse_string_value() {
    let cfg = cfg_parse("key: \"hello, world!\"").unwrap();
    assert_eq!(cfg.count, 1);
    assert_eq!(cfg.entries[0].key, "key");
    assert_eq!(cfg.entries[0].val, CfgVal::String("hello, world!".to_string()));
}

#[test]
fn test_parse_int_value() {
    let cfg = cfg_parse("key: 10").unwrap();
    assert_eq!(cfg.count, 1);
    assert_eq!(cfg.entries[0].key, "key");
    assert_eq!(cfg.entries[0].val, CfgVal::Int(10));
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
    assert_eq!(cfg.entries[0].key, "key");
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
fn test_parse_float() {
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
fn test_parse_three_int_entries() {
    let cfg = cfg_parse("a: 1\nb: 2\nc: 3").unwrap();
    assert_eq!(cfg.count, 3);
    assert_eq!(cfg.entries[0].key, "a");
    assert_eq!(cfg.entries[0].val, CfgVal::Int(1));
    assert_eq!(cfg.entries[1].key, "b");
    assert_eq!(cfg.entries[1].val, CfgVal::Int(2));
    assert_eq!(cfg.entries[2].key, "c");
    assert_eq!(cfg.entries[2].val, CfgVal::Int(3));
}

#[test]
fn test_parse_rgba_full_alpha() {
    let cfg = cfg_parse("key: rgba(255, 255, 255, 1)").unwrap();
    assert_eq!(cfg.count, 1);
    assert_eq!(
        cfg.entries[0].val,
        CfgVal::Color(CfgColor { r: 255, g: 255, b: 255, a: 255 })
    );
}

#[test]
fn test_parse_rgba_half_alpha() {
    let cfg = cfg_parse("key: rgba(0, 128, 64, 0.5)").unwrap();
    assert_eq!(cfg.count, 1);
    // 0.5 * 255 = 127.5 -> truncated to 127 in C
    assert_eq!(
        cfg.entries[0].val,
        CfgVal::Color(CfgColor { r: 0, g: 128, b: 64, a: 127 })
    );
}

#[test]
fn test_parse_rgba_zero_alpha() {
    let cfg = cfg_parse("key: rgba(0, 128, 64, 0)").unwrap();
    assert_eq!(cfg.count, 1);
    assert_eq!(
        cfg.entries[0].val,
        CfgVal::Color(CfgColor { r: 0, g: 128, b: 64, a: 0 })
    );
}

// ============================================================
// Parse errors
// ============================================================

#[test]
fn test_err_missing_key() {
    let err = cfg_parse("!").unwrap_err();
    assert_eq!(err.msg, "missing key");
    assert_eq!(err.off, 0);
    assert_eq!(err.row, 1);
    assert_eq!(err.col, 1);
}

#[test]
fn test_err_key_too_long() {
    let err = cfg_parse("xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx").unwrap_err();
    assert_eq!(err.msg, "key too long");
    assert_eq!(err.off, 33);
    assert_eq!(err.row, 1);
    assert_eq!(err.col, 34);
}

#[test]
fn test_err_colon_expected_eof() {
    let err = cfg_parse("key").unwrap_err();
    assert_eq!(err.msg, "':' expected");
    assert_eq!(err.off, 3);
    assert_eq!(err.row, 1);
    assert_eq!(err.col, 4);
}

#[test]
fn test_err_colon_expected_punct() {
    let err = cfg_parse("key!").unwrap_err();
    assert_eq!(err.msg, "':' expected");
    assert_eq!(err.off, 3);
    assert_eq!(err.row, 1);
    assert_eq!(err.col, 4);
}

#[test]
fn test_err_missing_value_with_blanks() {
    let err = cfg_parse("key  :").unwrap_err();
    assert_eq!(err.msg, "missing value");
    assert_eq!(err.off, 6);
    assert_eq!(err.row, 1);
    assert_eq!(err.col, 7);
}

#[test]
fn test_err_missing_value_eof() {
    let err = cfg_parse("key:").unwrap_err();
    assert_eq!(err.msg, "missing value");
    assert_eq!(err.off, 4);
    assert_eq!(err.row, 1);
    assert_eq!(err.col, 5);
}

#[test]
fn test_err_missing_value_blanks_eof() {
    let err = cfg_parse("key:  ").unwrap_err();
    assert_eq!(err.msg, "missing value");
    assert_eq!(err.off, 6);
    assert_eq!(err.row, 1);
    assert_eq!(err.col, 7);
}

#[test]
fn test_err_missing_value_newline() {
    let err = cfg_parse("key:\n").unwrap_err();
    assert_eq!(err.msg, "missing value");
    assert_eq!(err.off, 4);
    assert_eq!(err.row, 1);
    assert_eq!(err.col, 5);
}

#[test]
fn test_err_missing_value_blanks_newline() {
    let err = cfg_parse("key:  \n").unwrap_err();
    assert_eq!(err.msg, "missing value");
    assert_eq!(err.off, 6);
    assert_eq!(err.row, 1);
    assert_eq!(err.col, 7);
}

#[test]
fn test_err_invalid_value() {
    let err = cfg_parse("key: @\n").unwrap_err();
    assert_eq!(err.msg, "invalid value");
    assert_eq!(err.off, 5);
    assert_eq!(err.row, 1);
    assert_eq!(err.col, 6);
}

#[test]
fn test_err_unclosed_quote() {
    let err = cfg_parse("key: \"").unwrap_err();
    assert_eq!(err.msg, "closing '\"' expected");
    assert_eq!(err.off, 6);
    assert_eq!(err.row, 1);
    assert_eq!(err.col, 7);
}

#[test]
fn test_err_unclosed_string() {
    let err = cfg_parse("key: \"hello").unwrap_err();
    assert_eq!(err.msg, "closing '\"' expected");
    assert_eq!(err.off, 11);
    assert_eq!(err.row, 1);
    assert_eq!(err.col, 12);
}

#[test]
fn test_err_value_too_long() {
    let s = "key: \"".to_string()
        + &"x".repeat(CFG_MAX_VAL + 1)
        + "\"";
    let err = cfg_parse(&s).unwrap_err();
    assert_eq!(err.msg, "value too long");
}

#[test]
fn test_err_unexpected_after_int() {
    let err = cfg_parse("key: 10x").unwrap_err();
    assert_eq!(err.msg, "unexpected character 'x'");
    assert_eq!(err.off, 7);
    assert_eq!(err.row, 1);
    assert_eq!(err.col, 8);
}

#[test]
fn test_err_dash_alone() {
    let err = cfg_parse("key: -").unwrap_err();
    assert_eq!(err.msg, "invalid value");
    assert_eq!(err.off, 5);
    assert_eq!(err.row, 1);
    assert_eq!(err.col, 6);
}

#[test]
fn test_err_invalid_literal_t() {
    let err = cfg_parse("key: t").unwrap_err();
    assert_eq!(err.msg, "invalid literal");
}

#[test]
fn test_err_invalid_literal_f() {
    let err = cfg_parse("key: f").unwrap_err();
    assert_eq!(err.msg, "invalid literal");
}

#[test]
fn test_err_invalid_literal_x() {
    let err = cfg_parse("key: x").unwrap_err();
    assert_eq!(err.msg, "invalid literal");
}

#[test]
fn test_err_invalid_literal_r() {
    let err = cfg_parse("key: r").unwrap_err();
    assert_eq!(err.msg, "invalid literal");
}

#[test]
fn test_err_rgba_paren_then_eof() {
    let err = cfg_parse("key: rgba(").unwrap_err();
    assert_eq!(err.msg, "',' expected");
}

#[test]
fn test_err_rgba_no_paren() {
    let err = cfg_parse("key: rgba").unwrap_err();
    assert_eq!(err.msg, "'(' expected");
}

#[test]
fn test_err_rgba_no_paren_then_x() {
    let err = cfg_parse("key: rgba x").unwrap_err();
    assert_eq!(err.msg, "'(' expected");
}

#[test]
fn test_err_rgba_float_red() {
    let err = cfg_parse("key: rgba(0.5").unwrap_err();
    assert_eq!(err.msg, "red, blue and green must be integers in range [0, 255]");
}

#[test]
fn test_err_rgba_negative_red() {
    let err = cfg_parse("key: rgba(-1").unwrap_err();
    assert_eq!(err.msg, "red, blue and green must be integers in range [0, 255]");
}

#[test]
fn test_err_rgba_red_too_large() {
    let err = cfg_parse("key: rgba(256").unwrap_err();
    assert_eq!(err.msg, "red, blue and green must be integers in range [0, 255]");
}

#[test]
fn test_err_rgba_alpha_negative() {
    let err = cfg_parse("key: rgba(255, 255, 255, -1)").unwrap_err();
    assert_eq!(err.msg, "alpha must be in range [0, 1]");
}

#[test]
fn test_err_rgba_alpha_too_large() {
    let err = cfg_parse("key: rgba(255, 255, 255, 2)").unwrap_err();
    assert_eq!(err.msg, "alpha must be in range [0, 1]");
}

#[test]
fn test_err_rgba_no_close_paren() {
    let err = cfg_parse("key: rgba(255, 255, 255, 1").unwrap_err();
    assert_eq!(err.msg, "')' expected");
}

#[test]
fn test_err_rgba_alpha_not_a_number() {
    let err = cfg_parse("key: rgba(255, 255, 255, x").unwrap_err();
    assert_eq!(err.msg, "number expected");
}

#[test]
fn test_err_rgba_red_not_a_number() {
    let err = cfg_parse("key: rgba(x").unwrap_err();
    assert_eq!(err.msg, "number expected");
}

#[test]
fn test_err_rgba_missing_comma() {
    let err = cfg_parse("key: rgba(255 x").unwrap_err();
    assert_eq!(err.msg, "',' expected");
}

#[test]
fn test_err_rgba_missing_close_paren() {
    let err = cfg_parse("key: rgba(255, 255, 255, 1 x").unwrap_err();
    assert_eq!(err.msg, "')' expected");
}

#[test]
fn test_err_int_overflow() {
    let err = cfg_parse("key: 2147483648").unwrap_err();
    assert_eq!(err.msg, "number too large");
}

#[test]
fn test_err_float_overflow_via_div() {
    let err = cfg_parse("key: 0.0000000000").unwrap_err();
    assert_eq!(err.msg, "number too large");
}

#[test]
fn test_err_float_fract_overflow() {
    let err = cfg_parse("key: 1.33333333333333333").unwrap_err();
    assert_eq!(err.msg, "number too large");
}

#[test]
fn test_err_rgba_alpha_negative_float() {
    let err = cfg_parse("key: rgba(255, 255, 255, -0.5)").unwrap_err();
    assert_eq!(err.msg, "alpha must be in range [0, 1]");
}

#[test]
fn test_err_rgba_alpha_dash_paren() {
    let err = cfg_parse("key: rgba(255, 255, 255, -)").unwrap_err();
    assert_eq!(err.msg, "number expected");
}

#[test]
fn test_err_rgba_alpha_float_too_large() {
    let err = cfg_parse("key: rgba(255, 255, 255, 1.5)").unwrap_err();
    assert_eq!(err.msg, "alpha must be in range [0, 1]");
}

#[test]
fn test_err_row_col_multi_line() {
    // 'a: true\nb:'  -> error at col 3 of row 2 (after 'b:').
    let err = cfg_parse("a:true\nb:").unwrap_err();
    assert_eq!(err.msg, "missing value");
    assert_eq!(err.row, 2);
    assert_eq!(err.col, 3);
}

// ============================================================
// cfg_get_* functions
// ============================================================

fn build_cfg(entries: Vec<CfgEntry>) -> Cfg {
    let count = entries.len() as i32;
    let cap = entries.len();
    Cfg { entries, count, capacity: cap }
}

#[test]
fn test_get_string_present() {
    let cfg = build_cfg(vec![
        CfgEntry { key: "keyA".to_string(), val: CfgVal::String("foobar".to_string()) },
        CfgEntry { key: "keyC".to_string(), val: CfgVal::Boolean(true) },
    ]);
    assert_eq!(cfg_get_string(&cfg, "keyA", "barfoo"), "foobar");
}

#[test]
fn test_get_string_missing() {
    let cfg = build_cfg(vec![
        CfgEntry { key: "keyA".to_string(), val: CfgVal::String("foobar".to_string()) },
        CfgEntry { key: "keyC".to_string(), val: CfgVal::Boolean(true) },
    ]);
    assert_eq!(cfg_get_string(&cfg, "keyB", "barfoo"), "barfoo");
}

#[test]
fn test_get_string_wrong_type() {
    let cfg = build_cfg(vec![
        CfgEntry { key: "keyA".to_string(), val: CfgVal::String("foobar".to_string()) },
        CfgEntry { key: "keyC".to_string(), val: CfgVal::Boolean(true) },
    ]);
    assert_eq!(cfg_get_string(&cfg, "keyC", "barfoo"), "barfoo");
}

#[test]
fn test_get_bool_present() {
    let cfg = build_cfg(vec![CfgEntry {
        key: "key".to_string(),
        val: CfgVal::Boolean(true),
    }]);
    assert_eq!(cfg_get_bool(&cfg, "key", false), true);
}

#[test]
fn test_get_bool_missing() {
    let cfg = build_cfg(vec![]);
    assert_eq!(cfg_get_bool(&cfg, "key", false), false);
    assert_eq!(cfg_get_bool(&cfg, "key", true), true);
}

#[test]
fn test_get_int_present() {
    let cfg = build_cfg(vec![CfgEntry {
        key: "key".to_string(),
        val: CfgVal::Int(16),
    }]);
    assert_eq!(cfg_get_int(&cfg, "key", 64), 16);
}

#[test]
fn test_get_int_missing() {
    let cfg = build_cfg(vec![]);
    assert_eq!(cfg_get_int(&cfg, "key", 64), 64);
}

#[test]
fn test_get_int_min() {
    let cfg = build_cfg(vec![CfgEntry {
        key: "key".to_string(),
        val: CfgVal::Int(16),
    }]);
    assert_eq!(cfg_get_int_min(&cfg, "key", 64, 8), 16);
    assert_eq!(cfg_get_int_min(&cfg, "key", 64, 32), 64);
}

#[test]
fn test_get_int_max() {
    let cfg = build_cfg(vec![CfgEntry {
        key: "key".to_string(),
        val: CfgVal::Int(16),
    }]);
    assert_eq!(cfg_get_int_max(&cfg, "key", 64, 32), 16);
    assert_eq!(cfg_get_int_max(&cfg, "key", 4, 8), 4);
}

#[test]
fn test_get_int_range() {
    let cfg = build_cfg(vec![CfgEntry {
        key: "key".to_string(),
        val: CfgVal::Int(16),
    }]);
    assert_eq!(cfg_get_int_range(&cfg, "key", 64, 8, 32), 16);
    assert_eq!(cfg_get_int_range(&cfg, "key", 4, 4, 8), 4);
    assert_eq!(cfg_get_int_range(&cfg, "key", 32, 32, 64), 32);
}

#[test]
fn test_get_float_present() {
    let cfg = build_cfg(vec![CfgEntry {
        key: "key".to_string(),
        val: CfgVal::Float(16.0),
    }]);
    assert_eq!(cfg_get_float(&cfg, "key", 64.0), 16.0);
}

#[test]
fn test_get_float_missing() {
    let cfg = build_cfg(vec![]);
    assert_eq!(cfg_get_float(&cfg, "key", 64.0), 64.0);
}

#[test]
fn test_get_float_min() {
    let cfg = build_cfg(vec![CfgEntry {
        key: "key".to_string(),
        val: CfgVal::Float(16.0),
    }]);
    assert_eq!(cfg_get_float_min(&cfg, "key", 64.0, 8.0), 16.0);
    assert_eq!(cfg_get_float_min(&cfg, "key", 64.0, 32.0), 64.0);
}

#[test]
fn test_get_float_max() {
    let cfg = build_cfg(vec![CfgEntry {
        key: "key".to_string(),
        val: CfgVal::Float(16.0),
    }]);
    assert_eq!(cfg_get_float_max(&cfg, "key", 64.0, 32.0), 16.0);
    assert_eq!(cfg_get_float_max(&cfg, "key", 4.0, 8.0), 4.0);
}

#[test]
fn test_get_float_range() {
    let cfg = build_cfg(vec![CfgEntry {
        key: "key".to_string(),
        val: CfgVal::Float(16.0),
    }]);
    assert_eq!(cfg_get_float_range(&cfg, "key", 64.0, 8.0, 32.0), 16.0);
    assert_eq!(cfg_get_float_range(&cfg, "key", 4.0, 4.0, 8.0), 4.0);
    assert_eq!(cfg_get_float_range(&cfg, "key", 32.0, 32.0, 64.0), 32.0);
}

#[test]
fn test_get_color_present() {
    let c1 = CfgColor { r: 255, g: 255, b: 255, a: 255 };
    let c2 = CfgColor { r: 0, g: 0, b: 0, a: 0 };
    let cfg = build_cfg(vec![CfgEntry {
        key: "key".to_string(),
        val: CfgVal::Color(c1),
    }]);
    let actual = cfg_get_color(&cfg, "key", c2);
    assert_eq!(actual, c1);
    assert_eq!(actual.r, 255);
    assert_eq!(actual.g, 255);
    assert_eq!(actual.b, 255);
    assert_eq!(actual.a, 255);
}

#[test]
fn test_get_color_missing() {
    let c2 = CfgColor { r: 1, g: 2, b: 3, a: 4 };
    let cfg = build_cfg(vec![]);
    let actual = cfg_get_color(&cfg, "key", c2);
    assert_eq!(actual, c2);
}

// ============================================================
// Display impls (used for cfg_fprint and cfg_fprint_error)
// ============================================================

#[test]
fn test_display_color() {
    let c = CfgColor { r: 255, g: 255, b: 255, a: 255 };
    assert_eq!(format!("{}", c), "rgba(255, 255, 255, 255)");
}

#[test]
fn test_display_val_string() {
    let v = CfgVal::String("hello".to_string());
    assert_eq!(format!("{}", v), "\"hello\"");
}

#[test]
fn test_display_val_bool() {
    assert_eq!(format!("{}", CfgVal::Boolean(true)), "true");
    assert_eq!(format!("{}", CfgVal::Boolean(false)), "false");
}

#[test]
fn test_display_val_int() {
    assert_eq!(format!("{}", CfgVal::Int(14)), "14");
    assert_eq!(format!("{}", CfgVal::Int(-1)), "-1");
}

#[test]
fn test_display_val_float() {
    assert_eq!(format!("{}", CfgVal::Float(1.5)), "1.500000");
}

#[test]
fn test_display_val_color() {
    let v = CfgVal::Color(CfgColor { r: 1, g: 2, b: 3, a: 4 });
    assert_eq!(format!("{}", v), "rgba(1, 2, 3, 4)");
}

#[test]
fn test_display_entry() {
    let e = CfgEntry { key: "k".to_string(), val: CfgVal::Int(7) };
    assert_eq!(format!("{}", e), "k: 7");
}

#[test]
fn test_display_cfg_full() {
    // Mirrors c_src/test/test_print.c:run_print_test
    let src = "font: \"JetBrainsMono Nerd Font\"\n\
               font.size: 14\n\
               zoom: 1.5\n\
               line_numbers: true\n\
               ruler: false\n\
               bg.color: rgba(255, 255, 255, 1)";
    let cfg = cfg_parse(src).unwrap();
    let printed = format!("{}", cfg);
    let expected = "font: \"JetBrainsMono Nerd Font\"\n\
                    font.size: 14\n\
                    zoom: 1.500000\n\
                    line_numbers: true\n\
                    ruler: false\n\
                    bg.color: rgba(255, 255, 255, 255)\n";
    assert_eq!(printed, expected);
}

#[test]
fn test_display_error_no_position() {
    let e = CfgError {
        off: -1,
        row: -1,
        col: -1,
        msg: "invalid filename".to_string(),
    };
    assert_eq!(format!("{}", e), "Error: invalid filename\n");
}

#[test]
fn test_display_error_with_position() {
    // From C test: "a:true\nb:" -> "Error at 2:3 :: missing value\n"
    let err = cfg_parse("a:true\nb:").unwrap_err();
    assert_eq!(format!("{}", err), "Error at 2:3 :: missing value\n");
}

// ============================================================
// cfg_parse_file
// ============================================================

#[test]
fn test_parse_file_invalid_filename_short() {
    let err = cfg_parse_file("").unwrap_err();
    assert_eq!(err.msg, "invalid filename");
    assert_eq!(err.off, -1);
    assert_eq!(err.row, -1);
    assert_eq!(err.col, -1);
}

#[test]
fn test_parse_file_invalid_extension() {
    let err = cfg_parse_file("foo.txt").unwrap_err();
    assert_eq!(err.msg, "invalid file extension");
    assert_eq!(err.off, -1);
    assert_eq!(err.row, -1);
    assert_eq!(err.col, -1);
}

#[test]
fn test_parse_file_does_not_exist() {
    let err = cfg_parse_file("/tmp/__definitely_not_a_real_file__.cfg").unwrap_err();
    assert_eq!(err.msg, "failed to open file");
}

#[test]
fn test_parse_file_sample() {
    use std::io::Write;
    let path = "/tmp/_test_simple_config_sample.cfg";
    {
        let mut f = std::fs::File::create(path).unwrap();
        f.write_all(b"# A sample config file\nfont: \"JetBrainsMono Nerd Font\"\nfont.size: 14\nzoom: 1.5\nline_numbers: true\nbg.color: rgba(255, 255, 255, 1)\n").unwrap();
    }
    let cfg = cfg_parse_file(path).unwrap();
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
    let _ = std::fs::remove_file(path);
}

// ============================================================
// CfgVal From conversions
// ============================================================
#[test]
fn test_from_conversions() {
    let v: CfgVal = "x".into();
    assert_eq!(v, CfgVal::String("x".to_string()));
    let v: CfgVal = String::from("y").into();
    assert_eq!(v, CfgVal::String("y".to_string()));
    let v: CfgVal = true.into();
    assert_eq!(v, CfgVal::Boolean(true));
    let v: CfgVal = 7i32.into();
    assert_eq!(v, CfgVal::Int(7));
    let v: CfgVal = 1.5f32.into();
    assert_eq!(v, CfgVal::Float(1.5));
    let c: CfgColor = (1u8, 2u8, 3u8, 4u8).into();
    assert_eq!(c, CfgColor { r: 1, g: 2, b: 3, a: 4 });
    let v: CfgVal = c.into();
    assert_eq!(v, CfgVal::Color(c));
}

// ============================================================
// Constants
// ============================================================
#[test]
fn test_constants() {
    assert_eq!(CFG_FILE_EXT, ".cfg");
    assert_eq!(CFG_MAX_KEY, 32);
    assert_eq!(CFG_MAX_VAL, 64);
    assert_eq!(CFG_MAX_ERR, 64);
}

// ============================================================
// CfgError default
// ============================================================
#[test]
fn test_cfg_error_default() {
    let e = CfgError::default();
    assert_eq!(e.off, 0);
    assert_eq!(e.row, 0);
    assert_eq!(e.col, 0);
    assert_eq!(e.msg, "");
}

fn main() {}
