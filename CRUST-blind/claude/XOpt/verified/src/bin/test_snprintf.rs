use XOpt::snprintf;

#[test]
fn test_rpl_vsnprintf_string() {
    let mut s = String::new();
    let r = snprintf::rpl_vsnprintf(&mut s, 1024, "%s", &["hello"]);
    assert_eq!(s, "hello");
    assert_eq!(r, 5);
}

#[test]
fn test_rpl_vsnprintf_string_width_pad() {
    let mut s = String::new();
    let r = snprintf::rpl_vsnprintf(&mut s, 1024, "%10s", &["hi"]);
    assert_eq!(s, "        hi");
    assert_eq!(r, 10);
}

#[test]
fn test_rpl_vsnprintf_string_left_justify() {
    let mut s = String::new();
    let r = snprintf::rpl_vsnprintf(&mut s, 1024, "%-10s|", &["hi"]);
    assert_eq!(s, "hi        |");
    assert_eq!(r, 11);
}

#[test]
fn test_rpl_vsnprintf_string_precision() {
    let mut s = String::new();
    let r = snprintf::rpl_vsnprintf(&mut s, 1024, "%.3s", &["hello"]);
    assert_eq!(s, "hel");
    assert_eq!(r, 3);
}

#[test]
fn test_rpl_vsnprintf_int() {
    let mut s = String::new();
    let r = snprintf::rpl_vsnprintf(&mut s, 1024, "%d", &["42"]);
    assert_eq!(s, "42");
    assert_eq!(r, 2);
}

#[test]
fn test_rpl_vsnprintf_int_negative() {
    let mut s = String::new();
    let r = snprintf::rpl_vsnprintf(&mut s, 1024, "%d", &["-42"]);
    assert_eq!(s, "-42");
    assert_eq!(r, 3);
}

#[test]
fn test_rpl_vsnprintf_int_width() {
    let mut s = String::new();
    let r = snprintf::rpl_vsnprintf(&mut s, 1024, "%5d", &["42"]);
    assert_eq!(s, "   42");
    assert_eq!(r, 5);
}

#[test]
fn test_rpl_vsnprintf_int_zero_pad() {
    let mut s = String::new();
    let r = snprintf::rpl_vsnprintf(&mut s, 1024, "%05d", &["42"]);
    assert_eq!(s, "00042");
    assert_eq!(r, 5);
}

#[test]
fn test_rpl_vsnprintf_int_zero_pad_negative() {
    let mut s = String::new();
    let r = snprintf::rpl_vsnprintf(&mut s, 1024, "%05d", &["-42"]);
    assert_eq!(s, "-0042");
    assert_eq!(r, 5);
}

#[test]
fn test_rpl_vsnprintf_int_left_justify() {
    let mut s = String::new();
    let r = snprintf::rpl_vsnprintf(&mut s, 1024, "%-5d|", &["42"]);
    assert_eq!(s, "42   |");
    assert_eq!(r, 6);
}

#[test]
fn test_rpl_vsnprintf_char() {
    let mut s = String::new();
    let r = snprintf::rpl_vsnprintf(&mut s, 1024, "%c", &["A"]);
    assert_eq!(s, "A");
    assert_eq!(r, 1);
}

#[test]
fn test_rpl_vsnprintf_octal() {
    let mut s = String::new();
    let r = snprintf::rpl_vsnprintf(&mut s, 1024, "%o", &["8"]);
    assert_eq!(s, "10");
    assert_eq!(r, 2);
}

#[test]
fn test_rpl_vsnprintf_hex_lower() {
    let mut s = String::new();
    let r = snprintf::rpl_vsnprintf(&mut s, 1024, "%x", &["255"]);
    assert_eq!(s, "ff");
    assert_eq!(r, 2);
}

#[test]
fn test_rpl_vsnprintf_hex_upper() {
    let mut s = String::new();
    let r = snprintf::rpl_vsnprintf(&mut s, 1024, "%X", &["255"]);
    assert_eq!(s, "FF");
    assert_eq!(r, 2);
}

#[test]
fn test_rpl_vsnprintf_unsigned() {
    let mut s = String::new();
    let r = snprintf::rpl_vsnprintf(&mut s, 1024, "%u", &["42"]);
    assert_eq!(s, "42");
    assert_eq!(r, 2);
}

#[test]
fn test_rpl_vsnprintf_percent() {
    let mut s = String::new();
    let r = snprintf::rpl_vsnprintf(&mut s, 1024, "100%% done", &[]);
    assert_eq!(s, "100% done");
    assert_eq!(r, 9);
}

#[test]
fn test_rpl_vsnprintf_float_default() {
    let mut s = String::new();
    let r = snprintf::rpl_vsnprintf(&mut s, 1024, "%f", &["3.14"]);
    assert_eq!(s, "3.140000");
    assert_eq!(r, 8);
}

#[test]
fn test_rpl_vsnprintf_float_precision() {
    let mut s = String::new();
    let r = snprintf::rpl_vsnprintf(&mut s, 1024, "%.2f", &["3.14159"]);
    assert_eq!(s, "3.14");
    assert_eq!(r, 4);
}

#[test]
fn test_rpl_vsnprintf_no_args() {
    let mut s = String::new();
    let r = snprintf::rpl_vsnprintf(&mut s, 1024, "no args", &[]);
    assert_eq!(s, "no args");
    assert_eq!(r, 7);
}

#[test]
fn test_rpl_vsnprintf_empty_format() {
    let mut s = String::new();
    let r = snprintf::rpl_vsnprintf(&mut s, 1024, "", &[]);
    assert_eq!(s, "");
    assert_eq!(r, 0);
}

#[test]
fn test_rpl_vsnprintf_int_width_precision() {
    let mut s = String::new();
    let r = snprintf::rpl_vsnprintf(&mut s, 1024, "%5.3d", &["42"]);
    assert_eq!(s, "  042");
    assert_eq!(r, 5);
}

#[test]
fn test_rpl_vsnprintf_int_left_with_precision() {
    let mut s = String::new();
    let r = snprintf::rpl_vsnprintf(&mut s, 1024, "%-5.3d|", &["42"]);
    assert_eq!(s, "042  |");
    assert_eq!(r, 6);
}

#[test]
fn test_rpl_vsnprintf_int_zero() {
    let mut s = String::new();
    let r = snprintf::rpl_vsnprintf(&mut s, 1024, "%d", &["0"]);
    assert_eq!(s, "0");
    assert_eq!(r, 1);
}

#[test]
fn test_rpl_vsnprintf_hex_zero() {
    let mut s = String::new();
    let r = snprintf::rpl_vsnprintf(&mut s, 1024, "%x", &["0"]);
    assert_eq!(s, "0");
    assert_eq!(r, 1);
}

#[test]
fn test_rpl_vsnprintf_int_precision_only() {
    let mut s = String::new();
    let r = snprintf::rpl_vsnprintf(&mut s, 1024, "%.5d", &["42"]);
    assert_eq!(s, "00042");
    assert_eq!(r, 5);
}

#[test]
fn test_rpl_vsnprintf_string_width_and_precision() {
    let mut s = String::new();
    let r = snprintf::rpl_vsnprintf(&mut s, 1024, "%6.3s", &["hello"]);
    assert_eq!(s, "   hel");
    assert_eq!(r, 6);
}

#[test]
fn test_rpl_vsnprintf_truncation() {
    let mut s = String::new();
    // n=5 means buffer holds at most 4 chars (last byte for null terminator)
    let r = snprintf::rpl_vsnprintf(&mut s, 5, "1234567890", &[]);
    assert_eq!(s, "1234");
    assert_eq!(r, 10);
}

#[test]
fn test_rpl_vsnprintf_string_with_brackets() {
    let mut s = String::new();
    let r = snprintf::rpl_vsnprintf(&mut s, 1024, "[%5s]", &["hi"]);
    assert_eq!(s, "[   hi]");
    assert_eq!(r, 7);
}

#[test]
fn test_rpl_vsnprintf_multiple_args() {
    let mut s = String::new();
    let r = snprintf::rpl_vsnprintf(&mut s, 1024, "name=%s, value=%d", &["foo", "42"]);
    assert_eq!(s, "name=foo, value=42");
    assert_eq!(r, 18);
}

#[test]
fn test_rpl_vsnprintf_dot_star_string() {
    let mut s = String::new();
    let r = snprintf::rpl_vsnprintf(&mut s, 1024, "%.*s", &["3", "hello"]);
    assert_eq!(s, "hel");
    assert_eq!(r, 3);
}

#[test]
fn test_rpl_vsnprintf_percent_only() {
    let mut s = String::new();
    let r = snprintf::rpl_vsnprintf(&mut s, 1024, "%%", &[]);
    assert_eq!(s, "%");
    assert_eq!(r, 1);
}

#[test]
fn test_rpl_vsnprintf_xopt_invalid_option_format() {
    let mut s = String::new();
    let r = snprintf::rpl_vsnprintf(&mut s, 1024, "invalid option: --%.*s", &["4", "abcdef"]);
    assert_eq!(s, "invalid option: --abcd");
    assert_eq!(r, 22);
}

#[test]
fn test_rpl_vsnprintf_xopt_missing_value_format() {
    let mut s = String::new();
    let r = snprintf::rpl_vsnprintf(&mut s, 1024, "missing option value: --%s", &["some-int"]);
    assert_eq!(s, "missing option value: --some-int");
    assert_eq!(r, 32);
}

#[test]
fn test_rpl_asprintf() {
    let mut s = String::new();
    let r = snprintf::rpl_asprintf(&mut s, "test: %d", &["100"]);
    assert_eq!(s, "test: 100");
    assert_eq!(r, 9);
}

#[test]
fn test_fmtstr_basic() {
    let mut s = String::new();
    snprintf::fmtstr(&mut s, 1024, "hello", 0, usize::MAX, 0);
    assert_eq!(s, "hello");
}

#[test]
fn test_fmtstr_width() {
    let mut s = String::new();
    snprintf::fmtstr(&mut s, 1024, "hi", 5, usize::MAX, 0);
    assert_eq!(s, "   hi");
}

#[test]
fn test_fmtstr_precision() {
    let mut s = String::new();
    snprintf::fmtstr(&mut s, 1024, "hello", 0, 3, 0);
    assert_eq!(s, "hel");
}

#[test]
fn test_fmtstr_left_justify() {
    let mut s = String::new();
    // PRINT_F_MINUS = 0x1
    snprintf::fmtstr(&mut s, 1024, "hi", 5, usize::MAX, 0x1);
    assert_eq!(s, "hi   ");
}

#[test]
fn test_fmtint_basic() {
    let mut s = String::new();
    snprintf::fmtint(&mut s, 1024, 42, 0, usize::MAX, 0);
    assert_eq!(s, "42");
}

#[test]
fn test_fmtint_negative() {
    let mut s = String::new();
    snprintf::fmtint(&mut s, 1024, -42, 0, usize::MAX, 0);
    assert_eq!(s, "-42");
}

#[test]
fn test_fmtint_width() {
    let mut s = String::new();
    snprintf::fmtint(&mut s, 1024, 42, 5, usize::MAX, 0);
    assert_eq!(s, "   42");
}

#[test]
fn test_fmtflt_basic() {
    let mut s = String::new();
    snprintf::fmtflt(&mut s, 1024, 3.14, 0, 6, 0);
    assert_eq!(s, "3.140000");
}

#[test]
fn test_fmtflt_negative() {
    let mut s = String::new();
    snprintf::fmtflt(&mut s, 1024, -1.5, 0, 3, 0);
    assert_eq!(s, "-1.500");
}

#[test]
fn test_fmtflt_precision_two() {
    let mut s = String::new();
    snprintf::fmtflt(&mut s, 1024, 3.14159, 0, 2, 0);
    assert_eq!(s, "3.14");
}

#[test]
fn test_printsep() {
    let mut s = String::new();
    snprintf::printsep(&mut s, 1024);
    assert_eq!(s, ",");
}

#[test]
fn test_getnumsep() {
    // C formula: (digits - (digits % 3 == 0 ? 1 : 0)) / 3
    assert_eq!(snprintf::getnumsep(0), 0);
    assert_eq!(snprintf::getnumsep(1), 0);
    assert_eq!(snprintf::getnumsep(2), 0);
    assert_eq!(snprintf::getnumsep(3), 0);
    assert_eq!(snprintf::getnumsep(4), 1);
    assert_eq!(snprintf::getnumsep(5), 1);
    assert_eq!(snprintf::getnumsep(6), 1);
    assert_eq!(snprintf::getnumsep(7), 2);
    assert_eq!(snprintf::getnumsep(9), 2);
    assert_eq!(snprintf::getnumsep(10), 3);
}

#[test]
fn test_getexponent_zero() {
    assert_eq!(snprintf::getexponent(0.0), 0);
}

#[test]
fn test_getexponent_one() {
    assert_eq!(snprintf::getexponent(1.0), 0);
}

#[test]
fn test_getexponent_large() {
    assert_eq!(snprintf::getexponent(12345.0), 4);
}

#[test]
fn test_getexponent_small() {
    assert_eq!(snprintf::getexponent(0.005), -3);
}

#[test]
fn test_getexponent_negative_value() {
    assert_eq!(snprintf::getexponent(-12345.0), 4);
}

#[test]
fn test_convert_decimal() {
    let mut buf = String::new();
    // 42 base 10 -> reverse-order "24"
    snprintf::convert(42, &mut buf, 10, 0);
    assert_eq!(buf, "24");
}

#[test]
fn test_convert_hex_lower() {
    let mut buf = String::new();
    // 255 base 16 -> "ff" (palindrome)
    snprintf::convert(255, &mut buf, 16, 0);
    assert_eq!(buf, "ff");
}

#[test]
fn test_convert_hex_upper() {
    let mut buf = String::new();
    snprintf::convert(255, &mut buf, 16, 1);
    assert_eq!(buf, "FF");
}

#[test]
fn test_convert_octal() {
    let mut buf = String::new();
    // 8 base 8 -> "10" reversed = "01"
    snprintf::convert(8, &mut buf, 8, 0);
    assert_eq!(buf, "01");
}

#[test]
fn test_convert_zero() {
    let mut buf = String::new();
    // 0 base 10 -> "0"
    snprintf::convert(0, &mut buf, 10, 0);
    assert_eq!(buf, "0");
}

#[test]
fn test_convert_large_decimal() {
    let mut buf = String::new();
    // 1234 base 10 -> "4321" (reversed)
    snprintf::convert(1234, &mut buf, 10, 0);
    assert_eq!(buf, "4321");
}

#[test]
fn test_cast_positive() {
    // floor(3.99) = 3
    assert_eq!(snprintf::cast(3.99), 3);
}

#[test]
fn test_cast_negative() {
    // floor(-3.99) = -4
    assert_eq!(snprintf::cast(-3.99), -4);
}

#[test]
fn test_cast_exact_integer() {
    assert_eq!(snprintf::cast(3.0), 3);
}

#[test]
fn test_cast_zero() {
    assert_eq!(snprintf::cast(0.0), 0);
}

#[test]
fn test_cast_overflow_high() {
    assert_eq!(snprintf::cast(1e20), i32::MAX);
}

#[test]
fn test_cast_overflow_low() {
    assert_eq!(snprintf::cast(-1e20), i32::MIN);
}

#[test]
fn test_mypow10_zero() {
    assert_eq!(snprintf::mypow10(0), 1.0);
}

#[test]
fn test_mypow10_positive() {
    assert_eq!(snprintf::mypow10(3), 1000.0);
    assert_eq!(snprintf::mypow10(1), 10.0);
}

#[test]
fn test_mypow10_negative() {
    let r = snprintf::mypow10(-3);
    // Approx 0.001
    assert!((r - 0.001).abs() < 1e-9);
}

fn main() {}
