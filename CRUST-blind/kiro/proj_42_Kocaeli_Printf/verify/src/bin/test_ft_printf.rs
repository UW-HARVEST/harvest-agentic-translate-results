use proj_42_Kocaeli_Printf::ft_printf::*;
use std::any::Any;
use std::ptr;

// === writechar tests ===

#[test]
fn test_writechar_basic() {
    let mut len = 0;
    assert_eq!(writechar('A', &mut len), 1);
    assert_eq!(len, 1);
}

#[test]
fn test_writechar_null_byte() {
    let mut len = 0;
    assert_eq!(writechar('\0', &mut len), 1);
    assert_eq!(len, 1);
}

#[test]
fn test_writechar_accumulates_len() {
    let mut len = 5;
    assert_eq!(writechar('Z', &mut len), 1);
    assert_eq!(len, 6);
}

// === writestring tests ===

#[test]
fn test_writestring_hello() {
    let mut len = 0;
    assert_eq!(writestring("Hello", &mut len), 1);
    assert_eq!(len, 5);
}

#[test]
fn test_writestring_empty() {
    let mut len = 0;
    assert_eq!(writestring("", &mut len), 1);
    assert_eq!(len, 0);
}

#[test]
fn test_writestring_null_literal() {
    let mut len = 0;
    assert_eq!(writestring("(null)", &mut len), 1);
    assert_eq!(len, 6);
}

#[test]
fn test_writestring_two_chars() {
    let mut len = 0;
    assert_eq!(writestring("ab", &mut len), 1);
    assert_eq!(len, 2);
}

#[test]
fn test_writestring_ten_chars() {
    let mut len = 0;
    assert_eq!(writestring("1234567890", &mut len), 1);
    assert_eq!(len, 10);
}

// === writeint tests ===

#[test]
fn test_writeint_zero() {
    let mut len = 0;
    assert_eq!(writeint(0, &mut len), 1);
    assert_eq!(len, 1);
}

#[test]
fn test_writeint_positive() {
    let mut len = 0;
    assert_eq!(writeint(42, &mut len), 1);
    assert_eq!(len, 2);
}

#[test]
fn test_writeint_negative() {
    let mut len = 0;
    assert_eq!(writeint(-42, &mut len), 1);
    assert_eq!(len, 3);
}

#[test]
fn test_writeint_min() {
    let mut len = 0;
    assert_eq!(writeint(-2147483648, &mut len), 1);
    assert_eq!(len, 11);
}

#[test]
fn test_writeint_max() {
    let mut len = 0;
    assert_eq!(writeint(2147483647, &mut len), 1);
    assert_eq!(len, 10);
}

#[test]
fn test_writeint_one() {
    let mut len = 0;
    assert_eq!(writeint(1, &mut len), 1);
    assert_eq!(len, 1);
}

#[test]
fn test_writeint_neg_one() {
    let mut len = 0;
    assert_eq!(writeint(-1, &mut len), 1);
    assert_eq!(len, 2);
}

// === writeuint tests ===

#[test]
fn test_writeuint_zero() {
    let mut len = 0;
    assert_eq!(writeuint(0, &mut len), 1);
    assert_eq!(len, 1);
}

#[test]
fn test_writeuint_42() {
    let mut len = 0;
    assert_eq!(writeuint(42, &mut len), 1);
    assert_eq!(len, 2);
}

#[test]
fn test_writeuint_max() {
    let mut len = 0;
    assert_eq!(writeuint(4294967295u64, &mut len), 1);
    assert_eq!(len, 10);
}

#[test]
fn test_writeuint_one() {
    let mut len = 0;
    assert_eq!(writeuint(1, &mut len), 1);
    assert_eq!(len, 1);
}

// === writehex tests ===

#[test]
fn test_writehex_zero_lower() {
    let mut len = 0;
    assert_eq!(writehex(0, 'x', &mut len), 1);
    assert_eq!(len, 1);
}

#[test]
fn test_writehex_255_lower() {
    let mut len = 0;
    assert_eq!(writehex(255, 'x', &mut len), 1);
    assert_eq!(len, 2);
}

#[test]
fn test_writehex_255_upper() {
    let mut len = 0;
    assert_eq!(writehex(255, 'X', &mut len), 1);
    assert_eq!(len, 2);
}

#[test]
fn test_writehex_42_lower() {
    let mut len = 0;
    assert_eq!(writehex(42, 'x', &mut len), 1);
    assert_eq!(len, 2);
}

#[test]
fn test_writehex_42_upper() {
    let mut len = 0;
    assert_eq!(writehex(42, 'X', &mut len), 1);
    assert_eq!(len, 2);
}

#[test]
fn test_writehex_uint_max_lower() {
    let mut len = 0;
    assert_eq!(writehex(4294967295u64, 'x', &mut len), 1);
    assert_eq!(len, 8);
}

#[test]
fn test_writehex_16_lower() {
    let mut len = 0;
    assert_eq!(writehex(16, 'x', &mut len), 1);
    assert_eq!(len, 2);
}

// === writepoint tests ===

#[test]
fn test_writepoint_null() {
    let mut len = 0;
    assert_eq!(writepoint(ptr::null(), &mut len), 1);
    assert_eq!(len, 5);
}

#[test]
fn test_writepoint_0x1234() {
    let mut len = 0;
    assert_eq!(writepoint(0x1234 as *const std::ffi::c_void, &mut len), 1);
    assert_eq!(len, 6);
}

#[test]
fn test_writepoint_0x1() {
    let mut len = 0;
    assert_eq!(writepoint(0x1 as *const std::ffi::c_void, &mut len), 1);
    assert_eq!(len, 3);
}

// === format tests ===
// Each #[test] runs on its own thread, so thread-local ARG_INDEX starts at 0.

#[test]
fn test_format_percent() {
    let mut len = 0;
    let args: &[Box<dyn Any>] = &[];
    assert_eq!(format(args, '%', &mut len), 1);
    assert_eq!(len, 1);
}

#[test]
fn test_format_char() {
    let mut len = 0;
    let args: &[Box<dyn Any>] = &[Box::new('A')];
    assert_eq!(format(args, 'c', &mut len), 1);
    assert_eq!(len, 1);
}

#[test]
fn test_format_string() {
    let mut len = 0;
    let args: &[Box<dyn Any>] = &[Box::new("world" as &str)];
    assert_eq!(format(args, 's', &mut len), 1);
    assert_eq!(len, 5);
}

#[test]
fn test_format_int() {
    let mut len = 0;
    let args: &[Box<dyn Any>] = &[Box::new(42i32)];
    assert_eq!(format(args, 'd', &mut len), 1);
    assert_eq!(len, 2);
}

#[test]
fn test_format_uint() {
    let mut len = 0;
    let args: &[Box<dyn Any>] = &[Box::new(42u32)];
    assert_eq!(format(args, 'u', &mut len), 1);
    assert_eq!(len, 2);
}

#[test]
fn test_format_hex_lower() {
    let mut len = 0;
    let args: &[Box<dyn Any>] = &[Box::new(255u32)];
    assert_eq!(format(args, 'x', &mut len), 1);
    assert_eq!(len, 2);
}

#[test]
fn test_format_hex_upper() {
    let mut len = 0;
    let args: &[Box<dyn Any>] = &[Box::new(255u32)];
    assert_eq!(format(args, 'X', &mut len), 1);
    assert_eq!(len, 2);
}

#[test]
fn test_format_pointer_null() {
    let mut len = 0;
    let args: &[Box<dyn Any>] = &[Box::new(0usize)];
    assert_eq!(format(args, 'p', &mut len), 1);
    assert_eq!(len, 5);
}

#[test]
fn test_format_unknown_specifier() {
    let mut len = 0;
    let args: &[Box<dyn Any>] = &[];
    assert_eq!(format(args, 'z', &mut len), -1);
}

#[test]
fn test_format_null_string() {
    let mut len = 0;
    let args: &[Box<dyn Any>] = &[Box::new(None::<&str>)];
    assert_eq!(format(args, 's', &mut len), 1);
    assert_eq!(len, 6);
}

// === constants tests ===

#[test]
fn test_constants() {
    assert_eq!(HEXALOW, "0123456789abcdef");
    assert_eq!(HEXAUP, "0123456789ABCDEF");
    assert_eq!(DECIMAL, "0123456789");
    assert_eq!(LOCATION, 2);
}

fn main() {}
