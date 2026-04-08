use XOpt::snprintf;

// ---- Helper utilities ----

fn vsnprintf_helper(size: usize, fmt: &str, args: &[&str]) -> (String, i32) {
    let mut s = String::new();
    let r = snprintf::rpl_vsnprintf(&mut s, size, fmt, args);
    (s, r)
}

fn snprintf_large(fmt: &str, args: &[&str]) -> (String, i32) {
    vsnprintf_helper(1024, fmt, args)
}

// ---- rpl_vsnprintf tests ----

#[test]
fn test_vsnprintf_string_basic() {
    let (s, r) = snprintf_large("%s", &["Hello"]);
    assert_eq!(s, "Hello");
    assert_eq!(r, 5);
}

#[test]
fn test_vsnprintf_string_right_padded() {
    let (s, r) = snprintf_large("%10s", &["Hello"]);
    assert_eq!(s, "     Hello");
    assert_eq!(r, 10);
}

#[test]
fn test_vsnprintf_string_left_padded() {
    let (s, r) = snprintf_large("%-10s", &["Hello"]);
    assert_eq!(s, "Hello     ");
    assert_eq!(r, 10);
}

#[test]
fn test_vsnprintf_string_precision() {
    let (s, r) = snprintf_large("%.3s", &["Hello"]);
    assert_eq!(s, "Hel");
    assert_eq!(r, 3);
}

#[test]
fn test_vsnprintf_string_width_and_precision() {
    let (s, r) = snprintf_large("%10.3s", &["Hello"]);
    assert_eq!(s, "       Hel");
    assert_eq!(r, 10);
}

#[test]
fn test_vsnprintf_string_empty() {
    let (s, r) = snprintf_large("%s", &[""]);
    assert_eq!(s, "");
    assert_eq!(r, 0);
}

#[test]
fn test_vsnprintf_string_width_short() {
    let (s, r) = snprintf_large("%5s", &["Hi"]);
    assert_eq!(s, "   Hi");
    assert_eq!(r, 5);
}

// ---- Integer formatting ----

#[test]
fn test_vsnprintf_int_basic() {
    let (s, r) = snprintf_large("%d", &["42"]);
    assert_eq!(s, "42");
    assert_eq!(r, 2);
}

#[test]
fn test_vsnprintf_int_width() {
    let (s, r) = snprintf_large("%10d", &["42"]);
    assert_eq!(s, "        42");
    assert_eq!(r, 10);
}

#[test]
fn test_vsnprintf_int_left_justify() {
    let (s, r) = snprintf_large("%-10d", &["42"]);
    assert_eq!(s, "42        ");
    assert_eq!(r, 10);
}

#[test]
fn test_vsnprintf_int_zero_pad() {
    let (s, r) = snprintf_large("%010d", &["42"]);
    assert_eq!(s, "0000000042");
    assert_eq!(r, 10);
}

#[test]
fn test_vsnprintf_int_plus_sign() {
    let (s, r) = snprintf_large("%+d", &["42"]);
    assert_eq!(s, "+42");
    assert_eq!(r, 3);
}

#[test]
fn test_vsnprintf_int_negative() {
    let (s, r) = snprintf_large("%+d", &["-42"]);
    assert_eq!(s, "-42");
    assert_eq!(r, 3);
}

#[test]
fn test_vsnprintf_int_space_sign() {
    let (s, r) = snprintf_large("% d", &["42"]);
    assert_eq!(s, " 42");
    assert_eq!(r, 3);
}

#[test]
fn test_vsnprintf_int_precision() {
    let (s, r) = snprintf_large("%.5d", &["42"]);
    assert_eq!(s, "00042");
    assert_eq!(r, 5);
}

#[test]
fn test_vsnprintf_int_zero_precision_zero() {
    // C rpl_vsnprintf: "%.0d" with value 0 produces "0", len=1
    // (the rpl_ implementation doesn't suppress zero with precision=0)
    let (s, r) = snprintf_large("%.0d", &["0"]);
    assert_eq!(s, "0");
    assert_eq!(r, 1);
}

#[test]
fn test_vsnprintf_unsigned() {
    let (s, r) = snprintf_large("%u", &["42"]);
    assert_eq!(s, "42");
    assert_eq!(r, 2);
}

// ---- Hex and Octal ----

#[test]
fn test_vsnprintf_hex_lower() {
    let (s, r) = snprintf_large("%x", &["255"]);
    assert_eq!(s, "ff");
    assert_eq!(r, 2);
}

#[test]
fn test_vsnprintf_hex_upper() {
    let (s, r) = snprintf_large("%X", &["255"]);
    assert_eq!(s, "FF");
    assert_eq!(r, 2);
}

#[test]
fn test_vsnprintf_hex_prefix() {
    let (s, r) = snprintf_large("%#x", &["255"]);
    assert_eq!(s, "0xff");
    assert_eq!(r, 4);
}

#[test]
fn test_vsnprintf_octal() {
    let (s, r) = snprintf_large("%o", &["255"]);
    assert_eq!(s, "377");
    assert_eq!(r, 3);
}

#[test]
fn test_vsnprintf_octal_prefix() {
    let (s, r) = snprintf_large("%#o", &["255"]);
    assert_eq!(s, "0377");
    assert_eq!(r, 4);
}

// ---- Float formatting ----

#[test]
fn test_vsnprintf_float_basic() {
    let (s, r) = snprintf_large("%f", &["3.14"]);
    assert_eq!(s, "3.140000");
    assert_eq!(r, 8);
}

#[test]
fn test_vsnprintf_float_precision() {
    let (s, r) = snprintf_large("%.2f", &["3.14"]);
    assert_eq!(s, "3.14");
    assert_eq!(r, 4);
}

#[test]
fn test_vsnprintf_float_width_precision() {
    let (s, r) = snprintf_large("%10.2f", &["3.14"]);
    assert_eq!(s, "      3.14");
    assert_eq!(r, 10);
}

#[test]
fn test_vsnprintf_float_negative() {
    let (s, r) = snprintf_large("%f", &["-3.14"]);
    assert_eq!(s, "-3.140000");
    assert_eq!(r, 9);
}

#[test]
fn test_vsnprintf_float_plus() {
    let (s, r) = snprintf_large("%+f", &["3.14"]);
    assert_eq!(s, "+3.140000");
    assert_eq!(r, 9);
}

#[test]
fn test_vsnprintf_float_zero_pad() {
    let (s, r) = snprintf_large("%010.2f", &["3.14"]);
    assert_eq!(s, "0000003.14");
    assert_eq!(r, 10);
}

#[test]
fn test_vsnprintf_float_neg_zero_pad() {
    let (s, r) = snprintf_large("%010.2f", &["-3.14"]);
    assert_eq!(s, "-000003.14");
    assert_eq!(r, 10);
}

// ---- Scientific notation ----

#[test]
fn test_vsnprintf_scientific() {
    let (s, r) = snprintf_large("%e", &["3.14"]);
    assert_eq!(s, "3.140000e+00");
    assert_eq!(r, 12);
}

#[test]
fn test_vsnprintf_scientific_upper() {
    let (s, r) = snprintf_large("%E", &["3.14"]);
    assert_eq!(s, "3.140000E+00");
    assert_eq!(r, 12);
}

// ---- %g formatting ----

#[test]
fn test_vsnprintf_g_normal() {
    let (s, r) = snprintf_large("%g", &["3.14"]);
    assert_eq!(s, "3.14");
    assert_eq!(r, 4);
}

#[test]
fn test_vsnprintf_g_small() {
    let (s, r) = snprintf_large("%g", &["0.00001"]);
    assert_eq!(s, "1e-05");
    assert_eq!(r, 5);
}

#[test]
fn test_vsnprintf_g_large() {
    let (s, r) = snprintf_large("%g", &["100000"]);
    assert_eq!(s, "100000");
    assert_eq!(r, 6);
}

#[test]
fn test_vsnprintf_g_very_large() {
    let (s, r) = snprintf_large("%g", &["1000000"]);
    assert_eq!(s, "1e+06");
    assert_eq!(r, 5);
}

// ---- Special values ----

#[test]
fn test_vsnprintf_inf() {
    let (s, _) = snprintf_large("%f", &["inf"]);
    assert_eq!(s, "inf");
}

#[test]
fn test_vsnprintf_neg_inf() {
    let (s, _) = snprintf_large("%f", &["-inf"]);
    assert_eq!(s, "-inf");
}

#[test]
fn test_vsnprintf_nan() {
    let (s, _) = snprintf_large("%f", &["NaN"]);
    assert_eq!(s, "nan");
}

// ---- Percent and char ----

#[test]
fn test_vsnprintf_percent() {
    let (s, r) = snprintf_large("%%", &[]);
    assert_eq!(s, "%");
    assert_eq!(r, 1);
}

#[test]
fn test_vsnprintf_char() {
    let (s, r) = snprintf_large("%c", &["A"]);
    assert_eq!(s, "A");
    assert_eq!(r, 1);
}

// ---- Mixed format ----

#[test]
fn test_vsnprintf_mixed() {
    let (s, r) = snprintf_large("Hello %s, you are %d years old", &["World", "25"]);
    assert_eq!(s, "Hello World, you are 25 years old");
    assert_eq!(r, 33);
}

// ---- Truncation ----

#[test]
fn test_vsnprintf_truncation() {
    let (s, r) = vsnprintf_helper(10, "Hello, World!", &[]);
    assert_eq!(s, "Hello, Wo");
    assert_eq!(r, 13);
}

#[test]
fn test_vsnprintf_zero_size() {
    let (_, r) = vsnprintf_helper(0, "Hello", &[]);
    assert_eq!(r, 5);
}

// ---- Public helper function tests ----

#[test]
fn test_mypow10_positive() {
    assert_eq!(snprintf::mypow10(0), 1.0);
    assert_eq!(snprintf::mypow10(3), 1000.0);
    assert_eq!(snprintf::mypow10(1), 10.0);
}

#[test]
fn test_mypow10_negative() {
    let v = snprintf::mypow10(-2);
    assert!((v - 0.01).abs() < 1e-15);
}

#[test]
fn test_getexponent() {
    assert_eq!(snprintf::getexponent(3.14), 0);
    assert_eq!(snprintf::getexponent(31.4), 1);
    assert_eq!(snprintf::getexponent(0.314), -1);
    assert_eq!(snprintf::getexponent(100.0), 2);
    assert_eq!(snprintf::getexponent(0.001), -3);
    assert_eq!(snprintf::getexponent(-3.14), 0);
    assert_eq!(snprintf::getexponent(1.0), 0);
    assert_eq!(snprintf::getexponent(9.99), 0);
    assert_eq!(snprintf::getexponent(10.0), 1);
}

#[test]
fn test_getnumsep() {
    assert_eq!(snprintf::getnumsep(1), 0);
    assert_eq!(snprintf::getnumsep(3), 0);
    assert_eq!(snprintf::getnumsep(4), 1);
    assert_eq!(snprintf::getnumsep(7), 2);
    assert_eq!(snprintf::getnumsep(6), 1);
    assert_eq!(snprintf::getnumsep(9), 2);
    assert_eq!(snprintf::getnumsep(10), 3);
}

#[test]
fn test_cast() {
    assert_eq!(snprintf::cast(3.7), 3);
    assert_eq!(snprintf::cast(0.0), 0);
    assert_eq!(snprintf::cast(1.0), 1);
    assert_eq!(snprintf::cast(9.999), 9);
}

#[test]
fn test_convert() {
    let mut buf = String::new();
    snprintf::convert(255, &mut buf, 16, 0);
    assert_eq!(buf, "ff"); // reversed digits

    buf.clear();
    snprintf::convert(255, &mut buf, 16, 1);
    assert_eq!(buf, "FF");

    buf.clear();
    snprintf::convert(0, &mut buf, 10, 0);
    assert_eq!(buf, "0");

    buf.clear();
    snprintf::convert(123, &mut buf, 10, 0);
    assert_eq!(buf, "321"); // reversed
}

#[test]
fn test_printsep() {
    let mut s = String::new();
    snprintf::printsep(&mut s, 1024);
    assert_eq!(s, ",");
}

// ---- fmtstr direct tests ----

#[test]
fn test_fmtstr_basic() {
    let mut s = String::new();
    snprintf::fmtstr(&mut s, 1024, "Hello", 0, usize::MAX, 0);
    assert_eq!(s, "Hello");
}

#[test]
fn test_fmtstr_width() {
    let mut s = String::new();
    snprintf::fmtstr(&mut s, 1024, "Hi", 10, usize::MAX, 0);
    assert_eq!(s, "        Hi");
}

#[test]
fn test_fmtstr_left_justify() {
    let mut s = String::new();
    snprintf::fmtstr(&mut s, 1024, "Hi", 10, usize::MAX, 1); // PRINT_F_MINUS = 1
    assert_eq!(s, "Hi        ");
}

#[test]
fn test_fmtstr_precision() {
    let mut s = String::new();
    snprintf::fmtstr(&mut s, 1024, "Hello", 0, 3, 0);
    assert_eq!(s, "Hel");
}

// ---- fmtint direct tests ----

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
fn test_fmtint_zero() {
    let mut s = String::new();
    snprintf::fmtint(&mut s, 1024, 0, 0, usize::MAX, 0);
    assert_eq!(s, "0");
}

#[test]
fn test_fmtint_width() {
    let mut s = String::new();
    snprintf::fmtint(&mut s, 1024, 42, 10, usize::MAX, 0);
    assert_eq!(s, "        42");
}

// ---- fmtflt direct tests ----

#[test]
fn test_fmtflt_basic() {
    let mut s = String::new();
    snprintf::fmtflt(&mut s, 1024, 3.14, 0, usize::MAX, 0);
    assert_eq!(s, "3.140000");
}

#[test]
fn test_fmtflt_precision_2() {
    let mut s = String::new();
    snprintf::fmtflt(&mut s, 1024, 3.14, 0, 2, 0);
    assert_eq!(s, "3.14");
}

#[test]
fn test_fmtflt_nan() {
    let mut s = String::new();
    snprintf::fmtflt(&mut s, 1024, f64::NAN, 0, usize::MAX, 0);
    assert_eq!(s, "nan");
}

#[test]
fn test_fmtflt_inf() {
    let mut s = String::new();
    snprintf::fmtflt(&mut s, 1024, f64::INFINITY, 0, usize::MAX, 0);
    assert_eq!(s, "inf");
}

#[test]
fn test_fmtflt_neg_inf() {
    let mut s = String::new();
    snprintf::fmtflt(&mut s, 1024, f64::NEG_INFINITY, 0, usize::MAX, 0);
    assert_eq!(s, "-inf");
}

// ---- rpl_asprintf tests ----

#[test]
fn test_rpl_asprintf_basic() {
    let mut s = String::new();
    let r = snprintf::rpl_asprintf(&mut s, "Hello %s", &["World"]);
    assert_eq!(s, "Hello World");
    assert_eq!(r, 11);
}

#[test]
fn test_rpl_asprintf_int() {
    let mut s = String::new();
    let r = snprintf::rpl_asprintf(&mut s, "%d items", &["42"]);
    assert_eq!(s, "42 items");
    assert_eq!(r, 8);
}

// ---- Length modifier skipping ----

#[test]
fn test_vsnprintf_length_modifiers_skipped() {
    // The Rust version skips length modifiers (h, l, L, etc.)
    // %ld should behave same as %d for string-based args
    let (s, r) = snprintf_large("%ld", &["42"]);
    assert_eq!(s, "42");
    assert_eq!(r, 2);
}

#[test]
fn test_vsnprintf_lld() {
    let (s, r) = snprintf_large("%lld", &["42"]);
    assert_eq!(s, "42");
    assert_eq!(r, 2);
}

fn main() {}
