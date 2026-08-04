use XOpt::snprintf;

#[test]
fn test_getexponent() {
    // From C: getexponent(123.456) returns 2 (since 1.23456e2)
    assert_eq!(snprintf::getexponent(123.456), 2);
    assert_eq!(snprintf::getexponent(0.00123), -3);
    assert_eq!(snprintf::getexponent(1.0), 0);
    assert_eq!(snprintf::getexponent(0.0), 0);
    assert_eq!(snprintf::getexponent(10.0), 1);
    assert_eq!(snprintf::getexponent(100.0), 2);
    assert_eq!(snprintf::getexponent(0.1), -1);
    assert_eq!(snprintf::getexponent(0.01), -2);
    assert_eq!(snprintf::getexponent(-123.456), 2);
}

#[test]
fn test_mypow10() {
    // Compare values from C: mypow10(3) = 1000.0, mypow10(-2) = 0.01
    assert_eq!(snprintf::mypow10(0), 1.0);
    assert_eq!(snprintf::mypow10(1), 10.0);
    assert_eq!(snprintf::mypow10(2), 100.0);
    assert_eq!(snprintf::mypow10(3), 1000.0);
    assert!((snprintf::mypow10(-2) - 0.01).abs() < 1e-12);
}

#[test]
fn test_getnumsep() {
    // Per C: separators = (digits - (digits%3==0 ? 1 : 0)) / 3
    // digits=1: (1-0)/3 = 0
    // digits=3: (3-1)/3 = 0
    // digits=4: (4-0)/3 = 1
    // digits=6: (6-1)/3 = 1
    // digits=7: (7-0)/3 = 2
    assert_eq!(snprintf::getnumsep(1), 0);
    assert_eq!(snprintf::getnumsep(2), 0);
    assert_eq!(snprintf::getnumsep(3), 0);
    assert_eq!(snprintf::getnumsep(4), 1);
    assert_eq!(snprintf::getnumsep(5), 1);
    assert_eq!(snprintf::getnumsep(6), 1);
    assert_eq!(snprintf::getnumsep(7), 2);
}

#[test]
fn test_convert() {
    // Note: C convert() returns digits in reverse order.
    let mut buf = String::new();
    snprintf::convert(0, &mut buf, 10, 0);
    assert_eq!(buf, "0");

    let mut buf = String::new();
    snprintf::convert(123, &mut buf, 10, 0);
    // Reversed: "321"
    assert_eq!(buf, "321");

    let mut buf = String::new();
    snprintf::convert(255, &mut buf, 16, 0);
    // 0xff -> "ff" -> reversed "ff"
    assert_eq!(buf, "ff");

    let mut buf = String::new();
    snprintf::convert(255, &mut buf, 16, 1);
    assert_eq!(buf, "FF");

    let mut buf = String::new();
    snprintf::convert(8, &mut buf, 8, 0);
    // 8 -> "10" -> reversed "01"
    assert_eq!(buf, "01");
}

#[test]
fn test_cast() {
    assert_eq!(snprintf::cast(0.0), 0);
    assert_eq!(snprintf::cast(1.5), 1);
    assert_eq!(snprintf::cast(2.9), 2);
    assert_eq!(snprintf::cast(100.0), 100);
}

#[test]
fn test_printsep() {
    let mut s = String::new();
    snprintf::printsep(&mut s, 10);
    // Default behavior emits ','
    assert_eq!(s, ",");
}

#[test]
fn test_fmtstr_basic() {
    // From C: rpl_snprintf "%s" "hello" -> "hello"
    let mut buf = String::new();
    snprintf::fmtstr(&mut buf, usize::MAX, "hello", 0, usize::MAX, 0);
    assert_eq!(buf, "hello");
}

#[test]
fn test_fmtstr_width() {
    // C: %10s "hi" -> "        hi"
    let mut buf = String::new();
    snprintf::fmtstr(&mut buf, usize::MAX, "hi", 10, usize::MAX, 0);
    assert_eq!(buf, "        hi");
}

#[test]
fn test_fmtstr_left_justify() {
    // C: %-10s "hi" -> "hi        "
    let mut buf = String::new();
    snprintf::fmtstr(&mut buf, usize::MAX, "hi", 10, usize::MAX, 1 /* PRINT_F_MINUS */);
    assert_eq!(buf, "hi        ");
}

#[test]
fn test_fmtstr_precision() {
    // C: %.5s "hello world" -> "hello"
    let mut buf = String::new();
    snprintf::fmtstr(&mut buf, usize::MAX, "hello world", 0, 5, 0);
    assert_eq!(buf, "hello");
}

#[test]
fn test_fmtint_basic() {
    // C: %d 42 -> "42"
    let mut buf = String::new();
    snprintf::fmtint(&mut buf, usize::MAX, 42, 0, usize::MAX, 0);
    assert_eq!(buf, "42");
}

#[test]
fn test_fmtint_negative() {
    // C: %d -42 -> "-42"
    let mut buf = String::new();
    snprintf::fmtint(&mut buf, usize::MAX, -42, 0, usize::MAX, 0);
    assert_eq!(buf, "-42");
}

#[test]
fn test_fmtint_width() {
    // C: %5d 42 -> "   42"
    let mut buf = String::new();
    snprintf::fmtint(&mut buf, usize::MAX, 42, 5, usize::MAX, 0);
    assert_eq!(buf, "   42");
}

#[test]
fn test_fmtint_zero_pad() {
    // C: %05d 42 -> "00042"
    let mut buf = String::new();
    let print_f_zero = 1 << 4;
    snprintf::fmtint(&mut buf, usize::MAX, 42, 5, usize::MAX, print_f_zero);
    assert_eq!(buf, "00042");
}

#[test]
fn test_rpl_vsnprintf_int() {
    // C: snprintf "%d" 42 -> "42", n=2
    let mut s = String::new();
    let n = snprintf::rpl_vsnprintf(&mut s, 256, "%d", &["42"]);
    assert_eq!(s, "42");
    assert_eq!(n, 2);
}

#[test]
fn test_rpl_vsnprintf_neg() {
    // C: snprintf "%d" -42 -> "-42", n=3
    let mut s = String::new();
    let n = snprintf::rpl_vsnprintf(&mut s, 256, "%d", &["-42"]);
    assert_eq!(s, "-42");
    assert_eq!(n, 3);
}

#[test]
fn test_rpl_vsnprintf_string() {
    // C: snprintf "%s world" "hello" -> "hello world"
    let mut s = String::new();
    let n = snprintf::rpl_vsnprintf(&mut s, 256, "%s world", &["hello"]);
    assert_eq!(s, "hello world");
    assert_eq!(n, 11);
}

#[test]
fn test_rpl_vsnprintf_multi() {
    // C: snprintf "%d %d %d" 1,2,3 -> "1 2 3"
    let mut s = String::new();
    let n = snprintf::rpl_vsnprintf(&mut s, 256, "%d %d %d", &["1", "2", "3"]);
    assert_eq!(s, "1 2 3");
    assert_eq!(n, 5);
}

#[test]
fn test_rpl_vsnprintf_percent() {
    // C: snprintf "%%" -> "%"
    let mut s = String::new();
    let n = snprintf::rpl_vsnprintf(&mut s, 256, "%%", &[]);
    assert_eq!(s, "%");
    assert_eq!(n, 1);
}

#[test]
fn test_rpl_vsnprintf_hex() {
    // C: snprintf "%x" 255 -> "ff"
    let mut s = String::new();
    let n = snprintf::rpl_vsnprintf(&mut s, 256, "%x", &["255"]);
    assert_eq!(s, "ff");
    assert_eq!(n, 2);
}

#[test]
fn test_rpl_vsnprintf_HEX() {
    let mut s = String::new();
    let n = snprintf::rpl_vsnprintf(&mut s, 256, "%X", &["255"]);
    assert_eq!(s, "FF");
    assert_eq!(n, 2);
}

#[test]
fn test_rpl_vsnprintf_oct() {
    // C: snprintf "%o" 8 -> "10"
    let mut s = String::new();
    let n = snprintf::rpl_vsnprintf(&mut s, 256, "%o", &["8"]);
    assert_eq!(s, "10");
    assert_eq!(n, 2);
}

#[test]
fn test_rpl_vsnprintf_char() {
    let mut s = String::new();
    let n = snprintf::rpl_vsnprintf(&mut s, 256, "%c", &["A"]);
    assert_eq!(s, "A");
    assert_eq!(n, 1);
}

#[test]
fn test_rpl_vsnprintf_literal() {
    // No %s, just literal
    let mut s = String::new();
    let n = snprintf::rpl_vsnprintf(&mut s, 256, "hello world", &[]);
    assert_eq!(s, "hello world");
    assert_eq!(n, 11);
}

#[test]
fn test_rpl_asprintf() {
    let mut s = String::new();
    let n = snprintf::rpl_asprintf(&mut s, "x=%d y=%s", &["7", "hi"]);
    assert_eq!(s, "x=7 y=hi");
    assert_eq!(n, 8);
}

#[test]
fn test_rpl_vasprintf() {
    let v: Vec<String> = Vec::new();
    let n = snprintf::rpl_vasprintf(v, "hi", &[]);
    assert_eq!(n, 2);
}

#[test]
fn test_main_does_nothing() {
    // Just make sure main() can be called and doesn't panic.
    snprintf::main();
}

fn main() {}
