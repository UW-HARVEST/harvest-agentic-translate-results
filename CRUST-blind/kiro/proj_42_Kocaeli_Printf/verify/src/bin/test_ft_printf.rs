use proj_42_Kocaeli_Printf::ft_printf::*;
use std::any::Any;

// --- writechar ---

#[test]
fn test_writechar_normal() {
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
fn test_writechar_tilde() {
    let mut len = 0;
    assert_eq!(writechar('~', &mut len), 1);
    assert_eq!(len, 1);
}

// --- writestring ---

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
fn test_writestring_null_equivalent() {
    // C writestring(NULL) writes "(null)" -> len=6
    let mut len = 0;
    assert_eq!(writestring("(null)", &mut len), 1);
    assert_eq!(len, 6);
}

// --- writeint ---

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
    assert_eq!(writeint(i32::MIN, &mut len), 1);
    assert_eq!(len, 11);
}

#[test]
fn test_writeint_max() {
    let mut len = 0;
    assert_eq!(writeint(i32::MAX, &mut len), 1);
    assert_eq!(len, 10);
}

#[test]
fn test_writeint_neg_one() {
    let mut len = 0;
    assert_eq!(writeint(-1, &mut len), 1);
    assert_eq!(len, 2);
}

// --- writeuint ---

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
fn test_writeuint_max_u32() {
    let mut len = 0;
    assert_eq!(writeuint(4294967295, &mut len), 1);
    assert_eq!(len, 10);
}

// --- writehex ---

#[test]
fn test_writehex_zero() {
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
fn test_writehex_deadbeef() {
    let mut len = 0;
    assert_eq!(writehex(0xdeadbeef, 'x', &mut len), 1);
    assert_eq!(len, 8);
}

#[test]
fn test_writehex_16() {
    let mut len = 0;
    assert_eq!(writehex(16, 'x', &mut len), 1);
    assert_eq!(len, 2);
}

// --- writepoint ---

#[test]
fn test_writepoint_null() {
    let mut len = 0;
    assert_eq!(writepoint(std::ptr::null(), &mut len), 1);
    assert_eq!(len, 5); // "(nil)" = 5 chars
}

#[test]
fn test_writepoint_0x1234() {
    let mut len = 0;
    assert_eq!(writepoint(0x1234 as *const std::ffi::c_void, &mut len), 1);
    assert_eq!(len, 6); // "0x" + "1234" = 6
}

#[test]
fn test_writepoint_0x1() {
    let mut len = 0;
    assert_eq!(writepoint(0x1 as *const std::ffi::c_void, &mut len), 1);
    assert_eq!(len, 3); // "0x" + "1" = 3
}

// --- format ---

#[test]
fn test_format_char() {
    reset_arg_index();
    let args: Vec<Box<dyn Any>> = vec![Box::new('A' as i32)];
    let mut len = 0;
    assert_eq!(format(&args, 'c', &mut len), 1);
    assert_eq!(len, 1);
}

#[test]
fn test_format_string() {
    reset_arg_index();
    let args: Vec<Box<dyn Any>> = vec![Box::new("test" as &str)];
    let mut len = 0;
    assert_eq!(format(&args, 's', &mut len), 1);
    assert_eq!(len, 4);
}

#[test]
fn test_format_int() {
    reset_arg_index();
    let args: Vec<Box<dyn Any>> = vec![Box::new(42i32)];
    let mut len = 0;
    assert_eq!(format(&args, 'd', &mut len), 1);
    assert_eq!(len, 2);
}

#[test]
fn test_format_uint() {
    reset_arg_index();
    let args: Vec<Box<dyn Any>> = vec![Box::new(42u32)];
    let mut len = 0;
    assert_eq!(format(&args, 'u', &mut len), 1);
    assert_eq!(len, 2);
}

#[test]
fn test_format_hex_lower() {
    reset_arg_index();
    let args: Vec<Box<dyn Any>> = vec![Box::new(255u32)];
    let mut len = 0;
    assert_eq!(format(&args, 'x', &mut len), 1);
    assert_eq!(len, 2);
}

#[test]
fn test_format_hex_upper() {
    reset_arg_index();
    let args: Vec<Box<dyn Any>> = vec![Box::new(255u32)];
    let mut len = 0;
    assert_eq!(format(&args, 'X', &mut len), 1);
    assert_eq!(len, 2);
}

#[test]
fn test_format_pointer() {
    reset_arg_index();
    let args: Vec<Box<dyn Any>> = vec![Box::new(0x1234usize)];
    let mut len = 0;
    assert_eq!(format(&args, 'p', &mut len), 1);
    assert_eq!(len, 6);
}

#[test]
fn test_format_percent() {
    reset_arg_index();
    let args: Vec<Box<dyn Any>> = vec![];
    let mut len = 0;
    assert_eq!(format(&args, '%', &mut len), 1);
    assert_eq!(len, 1);
}

#[test]
fn test_format_int_i_specifier() {
    reset_arg_index();
    let args: Vec<Box<dyn Any>> = vec![Box::new(-1i32)];
    let mut len = 0;
    assert_eq!(format(&args, 'i', &mut len), 1);
    assert_eq!(len, 2);
}

fn main() {}
