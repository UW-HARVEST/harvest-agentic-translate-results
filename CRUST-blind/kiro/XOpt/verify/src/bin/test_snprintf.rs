use XOpt::snprintf;

// Helper to run rpl_vsnprintf and return (ret, output_string)
fn vsnprintf(fmt: &str, args: &[&str]) -> (i32, String) {
    let mut s = String::new();
    let ret = snprintf::rpl_vsnprintf(&mut s, usize::MAX, fmt, args);
    (ret, s)
}

#[test]
fn test_str() {
    let (ret, buf) = vsnprintf("hello %s", &["world"]);
    assert_eq!(ret, 11);
    assert_eq!(buf, "hello world");
}

#[test]
fn test_int() {
    let (ret, buf) = vsnprintf("num %d", &["42"]);
    assert_eq!(ret, 6);
    assert_eq!(buf, "num 42");
}

#[test]
fn test_neg_int() {
    let (ret, buf) = vsnprintf("neg %d", &["-7"]);
    assert_eq!(ret, 6);
    assert_eq!(buf, "neg -7");
}

#[test]
fn test_char() {
    let (ret, buf) = vsnprintf("char %c", &["A"]);
    assert_eq!(ret, 6);
    assert_eq!(buf, "char A");
}

#[test]
fn test_pct() {
    let (ret, buf) = vsnprintf("pct %%", &[]);
    assert_eq!(ret, 5);
    assert_eq!(buf, "pct %");
}

#[test]
fn test_float() {
    let (ret, buf) = vsnprintf("float %f", &["3.14"]);
    assert_eq!(ret, 14);
    assert_eq!(buf, "float 3.140000");
}

#[test]
fn test_exp() {
    let (ret, buf) = vsnprintf("exp %e", &["12345.6789"]);
    assert_eq!(ret, 16);
    assert_eq!(buf, "exp 1.234568e+04");
}

#[test]
fn test_exp_upper() {
    let (ret, buf) = vsnprintf("EXP %E", &["12345.6789"]);
    assert_eq!(ret, 16);
    assert_eq!(buf, "EXP 1.234568E+04");
}

#[test]
fn test_g() {
    let (ret, buf) = vsnprintf("g %g", &["12345.6789"]);
    assert_eq!(ret, 9);
    assert_eq!(buf, "g 12345.7");
}

#[test]
fn test_g_upper() {
    let (ret, buf) = vsnprintf("G %G", &["0.00012345"]);
    assert_eq!(ret, 12);
    assert_eq!(buf, "G 0.00012345");
}

#[test]
fn test_width_str() {
    let (ret, buf) = vsnprintf("[%10s]", &["hi"]);
    assert_eq!(ret, 12);
    assert_eq!(buf, "[        hi]");
}

#[test]
fn test_left_str() {
    let (ret, buf) = vsnprintf("[%-10s]", &["hi"]);
    assert_eq!(ret, 12);
    assert_eq!(buf, "[hi        ]");
}

#[test]
fn test_prec_str() {
    let (ret, buf) = vsnprintf("[%.3s]", &["hello"]);
    assert_eq!(ret, 5);
    assert_eq!(buf, "[hel]");
}

#[test]
fn test_zeropad() {
    let (ret, buf) = vsnprintf("[%05d]", &["42"]);
    assert_eq!(ret, 7);
    assert_eq!(buf, "[00042]");
}

#[test]
fn test_plus() {
    let (ret, buf) = vsnprintf("[%+d]", &["42"]);
    assert_eq!(ret, 5);
    assert_eq!(buf, "[+42]");
}

#[test]
fn test_space() {
    let (ret, buf) = vsnprintf("[% d]", &["42"]);
    assert_eq!(ret, 5);
    assert_eq!(buf, "[ 42]");
}

#[test]
fn test_width_prec_float() {
    let (ret, buf) = vsnprintf("[%10.3f]", &["3.14159"]);
    assert_eq!(ret, 12);
    assert_eq!(buf, "[     3.142]");
}

#[test]
fn test_zero() {
    let (ret, buf) = vsnprintf("%d", &["0"]);
    assert_eq!(ret, 1);
    assert_eq!(buf, "0");
}

#[test]
fn test_999() {
    let (ret, buf) = vsnprintf("%d", &["999"]);
    assert_eq!(ret, 3);
    assert_eq!(buf, "999");
}

#[test]
fn test_exp_1() {
    let (ret, buf) = vsnprintf("%e", &["1.0"]);
    assert_eq!(ret, 12);
    assert_eq!(buf, "1.000000e+00");
}

#[test]
fn test_exp_small() {
    let (ret, buf) = vsnprintf("%e", &["0.001"]);
    assert_eq!(ret, 12);
    assert_eq!(buf, "1.000000e-03");
}

#[test]
fn test_exp_100() {
    let (ret, buf) = vsnprintf("%e", &["100.0"]);
    assert_eq!(ret, 12);
    assert_eq!(buf, "1.000000e+02");
}

#[test]
fn test_prec0_float() {
    let (ret, buf) = vsnprintf("%.0f", &["3.7"]);
    assert_eq!(ret, 1);
    assert_eq!(buf, "4");
}

#[test]
fn test_prec10_float() {
    let (ret, buf) = vsnprintf("%.10f", &["1.23"]);
    assert_eq!(ret, 12);
    assert_eq!(buf, "1.2300000000");
}

#[test]
fn test_unsigned() {
    let (ret, buf) = vsnprintf("%u", &["42"]);
    assert_eq!(ret, 2);
    assert_eq!(buf, "42");
}

#[test]
fn test_neg_float() {
    let (ret, buf) = vsnprintf("%f", &["-2.5"]);
    assert_eq!(ret, 9);
    assert_eq!(buf, "-2.500000");
}

#[test]
fn test_nan() {
    let (ret, buf) = vsnprintf("%f", &["nan"]);
    assert_eq!(ret, 3);
    assert_eq!(buf, "nan");
}

#[test]
fn test_inf() {
    let (ret, buf) = vsnprintf("%f", &["inf"]);
    assert_eq!(ret, 3);
    assert_eq!(buf, "inf");
}

#[test]
fn test_empty() {
    let (ret, buf) = vsnprintf("", &[]);
    assert_eq!(ret, 0);
    assert_eq!(buf, "");
}

#[test]
fn test_multi() {
    let (ret, buf) = vsnprintf("%s=%d", &["key", "10"]);
    assert_eq!(ret, 6);
    assert_eq!(buf, "key=10");
}

#[test]
fn test_g_small() {
    let (ret, buf) = vsnprintf("%g", &["0.0001"]);
    assert_eq!(ret, 6);
    assert_eq!(buf, "0.0001");
}

#[test]
fn test_g_vsmall() {
    let (ret, buf) = vsnprintf("%g", &["0.00001"]);
    assert_eq!(ret, 5);
    assert_eq!(buf, "1e-05");
}

#[test]
fn test_g_1() {
    let (ret, buf) = vsnprintf("%g", &["1.0"]);
    assert_eq!(ret, 1);
    assert_eq!(buf, "1");
}

#[test]
fn test_g_100k() {
    let (ret, buf) = vsnprintf("%g", &["100000.0"]);
    assert_eq!(ret, 6);
    assert_eq!(buf, "100000");
}

#[test]
fn test_g_1m() {
    let (ret, buf) = vsnprintf("%g", &["1000000.0"]);
    assert_eq!(ret, 5);
    assert_eq!(buf, "1e+06");
}

#[test]
fn test_neg_inf() {
    let (ret, buf) = vsnprintf("%f", &["-inf"]);
    assert_eq!(ret, 4);
    assert_eq!(buf, "-inf");
}

#[test]
fn test_nan_upper() {
    let (ret, buf) = vsnprintf("%F", &["nan"]);
    assert_eq!(ret, 3);
    assert_eq!(buf, "NAN");
}

#[test]
fn test_inf_upper() {
    let (ret, buf) = vsnprintf("%F", &["inf"]);
    assert_eq!(ret, 3);
    assert_eq!(buf, "INF");
}

// Test helper functions directly

#[test]
fn test_convert_decimal() {
    let mut buf = String::new();
    snprintf::convert(0, &mut buf, 10, 0);
    // convert stores digits in reverse; for 0 it's just "0"
    assert_eq!(buf, "0");
}

#[test]
fn test_convert_123() {
    let mut buf = String::new();
    snprintf::convert(123, &mut buf, 10, 0);
    // Stored reversed: "321"
    assert_eq!(buf, "321");
}

#[test]
fn test_convert_base16() {
    let mut buf = String::new();
    snprintf::convert(255, &mut buf, 16, 0);
    // 255 = 0xff, reversed: "ff"
    assert_eq!(buf, "ff");
}

#[test]
fn test_convert_base16_caps() {
    let mut buf = String::new();
    snprintf::convert(255, &mut buf, 16, 1);
    assert_eq!(buf, "FF");
}

#[test]
fn test_cast_normal() {
    assert_eq!(snprintf::cast(3.7), 3);
}

#[test]
fn test_cast_zero() {
    assert_eq!(snprintf::cast(0.0), 0);
}

#[test]
fn test_cast_large() {
    // cast of a value >= u64::MAX should return i32::MAX
    assert_eq!(snprintf::cast(1e30), i32::MAX);
}

#[test]
fn test_mypow10_positive() {
    assert_eq!(snprintf::mypow10(0), 1.0);
    assert_eq!(snprintf::mypow10(1), 10.0);
    assert_eq!(snprintf::mypow10(3), 1000.0);
}

#[test]
fn test_mypow10_negative() {
    assert!((snprintf::mypow10(-1) - 0.1).abs() < 1e-15);
    assert!((snprintf::mypow10(-3) - 0.001).abs() < 1e-15);
}

#[test]
fn test_getexponent() {
    assert_eq!(snprintf::getexponent(1.0), 0);
    assert_eq!(snprintf::getexponent(10.0), 1);
    assert_eq!(snprintf::getexponent(100.0), 2);
    assert_eq!(snprintf::getexponent(0.1), -1);
    assert_eq!(snprintf::getexponent(0.001), -3);
    assert_eq!(snprintf::getexponent(12345.6789), 4);
}

#[test]
fn test_getnumsep() {
    assert_eq!(snprintf::getnumsep(1), 0);
    assert_eq!(snprintf::getnumsep(3), 0);
    assert_eq!(snprintf::getnumsep(4), 1);
    assert_eq!(snprintf::getnumsep(6), 1);
    assert_eq!(snprintf::getnumsep(7), 2);
}

#[test]
fn test_rpl_asprintf() {
    let mut s = String::new();
    let ret = snprintf::rpl_asprintf(&mut s, "hello %s %d", &["world", "42"]);
    assert_eq!(ret, 14);
    assert_eq!(s, "hello world 42");
}

#[test]
fn test_fmtstr_direct() {
    let mut s = String::new();
    snprintf::fmtstr(&mut s, usize::MAX, "abc", 0, usize::MAX, 0);
    assert_eq!(s, "abc");
}

#[test]
fn test_fmtstr_width() {
    let mut s = String::new();
    snprintf::fmtstr(&mut s, usize::MAX, "hi", 6, usize::MAX, 0);
    assert_eq!(s, "    hi");
}

#[test]
fn test_fmtstr_left_align() {
    let mut s = String::new();
    // PRINT_F_MINUS = 1
    snprintf::fmtstr(&mut s, usize::MAX, "hi", 6, usize::MAX, 1);
    assert_eq!(s, "hi    ");
}

#[test]
fn test_fmtint_direct() {
    let mut s = String::new();
    snprintf::fmtint(&mut s, usize::MAX, 42, 0, usize::MAX, 0);
    assert_eq!(s, "42");
}

#[test]
fn test_fmtint_negative() {
    let mut s = String::new();
    snprintf::fmtint(&mut s, usize::MAX, -7, 0, usize::MAX, 0);
    assert_eq!(s, "-7");
}

#[test]
fn test_fmtflt_direct() {
    let mut s = String::new();
    snprintf::fmtflt(&mut s, usize::MAX, 3.14, 0, usize::MAX, 0);
    assert_eq!(s, "3.140000");
}

fn main() {}
