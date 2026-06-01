use proj_42_Kocaeli_Printf::ft_printf;

#[test]
fn test_constants() {
    assert_eq!(ft_printf::HEXALOW, "0123456789abcdef");
    assert_eq!(ft_printf::HEXAUP, "0123456789ABCDEF");
    assert_eq!(ft_printf::DECIMAL, "0123456789");
    assert_eq!(ft_printf::LOCATION, 2);
}

// ---------------- writechar ----------------
#[test]
fn test_writechar_a() {
    let mut len: i32 = 0;
    let r = ft_printf::writechar('A', &mut len);
    assert_eq!(r, 1);
    assert_eq!(len, 1);
}

#[test]
fn test_writechar_b() {
    let mut len: i32 = 0;
    assert_eq!(ft_printf::writechar('B', &mut len), 1);
    assert_eq!(len, 1);
}

#[test]
fn test_writechar_z() {
    let mut len: i32 = 0;
    assert_eq!(ft_printf::writechar('z', &mut len), 1);
    assert_eq!(len, 1);
}

#[test]
fn test_writechar_multiple_increments_len() {
    let mut len: i32 = 0;
    ft_printf::writechar('a', &mut len);
    ft_printf::writechar('b', &mut len);
    ft_printf::writechar('c', &mut len);
    assert_eq!(len, 3);
}

#[test]
fn test_writechar_starting_len_increments() {
    let mut len: i32 = 7;
    let r = ft_printf::writechar('!', &mut len);
    assert_eq!(r, 1);
    assert_eq!(len, 8);
}

// ---------------- writestring ----------------
#[test]
fn test_writestring_hello() {
    let mut len: i32 = 0;
    let r = ft_printf::writestring("Hello", &mut len);
    assert_eq!(r, 1);
    assert_eq!(len, 5);
}

#[test]
fn test_writestring_world_bang() {
    let mut len: i32 = 0;
    let r = ft_printf::writestring("World!", &mut len);
    assert_eq!(r, 1);
    assert_eq!(len, 6);
}

#[test]
fn test_writestring_empty() {
    let mut len: i32 = 0;
    let r = ft_printf::writestring("", &mut len);
    assert_eq!(r, 1);
    assert_eq!(len, 0);
}

#[test]
fn test_writestring_long() {
    let mut len: i32 = 0;
    let r = ft_printf::writestring("1234567890", &mut len);
    assert_eq!(r, 1);
    assert_eq!(len, 10);
}

#[test]
fn test_writestring_with_special_chars() {
    let mut len: i32 = 0;
    // "abc\n\t" is 5 bytes
    let r = ft_printf::writestring("abc\n\t", &mut len);
    assert_eq!(r, 1);
    assert_eq!(len, 5);
}

#[test]
fn test_writestring_opt_null() {
    // C: writestring(NULL, &len) writes "(null)" -> len 6, ret 1
    let mut len: i32 = 0;
    let r = ft_printf::writestring_opt(None, &mut len);
    assert_eq!(r, 1);
    assert_eq!(len, 6);
}

#[test]
fn test_writestring_opt_some() {
    let mut len: i32 = 0;
    let r = ft_printf::writestring_opt(Some("Hi"), &mut len);
    assert_eq!(r, 1);
    assert_eq!(len, 2);
}

// ---------------- writeint ----------------
#[test]
fn test_writeint_42() {
    let mut len: i32 = 0;
    let r = ft_printf::writeint(42, &mut len);
    assert_eq!(r, 1);
    assert_eq!(len, 2);
}

#[test]
fn test_writeint_neg42() {
    let mut len: i32 = 0;
    let r = ft_printf::writeint(-42, &mut len);
    assert_eq!(r, 1);
    assert_eq!(len, 3);
}

#[test]
fn test_writeint_zero() {
    let mut len: i32 = 0;
    let r = ft_printf::writeint(0, &mut len);
    assert_eq!(r, 1);
    assert_eq!(len, 1);
}

#[test]
fn test_writeint_int_min() {
    let mut len: i32 = 0;
    let r = ft_printf::writeint(i32::MIN, &mut len);
    assert_eq!(r, 1);
    assert_eq!(len, 11);
}

#[test]
fn test_writeint_int_max() {
    let mut len: i32 = 0;
    let r = ft_printf::writeint(i32::MAX, &mut len);
    assert_eq!(r, 1);
    assert_eq!(len, 10);
}

#[test]
fn test_writeint_one() {
    let mut len: i32 = 0;
    let r = ft_printf::writeint(1, &mut len);
    assert_eq!(r, 1);
    assert_eq!(len, 1);
}

#[test]
fn test_writeint_neg_one() {
    let mut len: i32 = 0;
    let r = ft_printf::writeint(-1, &mut len);
    assert_eq!(r, 1);
    assert_eq!(len, 2);
}

#[test]
fn test_writeint_ten() {
    let mut len: i32 = 0;
    let r = ft_printf::writeint(10, &mut len);
    assert_eq!(r, 1);
    assert_eq!(len, 2);
}

#[test]
fn test_writeint_99999() {
    let mut len: i32 = 0;
    let r = ft_printf::writeint(99999, &mut len);
    assert_eq!(r, 1);
    assert_eq!(len, 5);
}

// ---------------- writeuint ----------------
#[test]
fn test_writeuint_42() {
    let mut len: i32 = 0;
    let r = ft_printf::writeuint(42, &mut len);
    assert_eq!(r, 1);
    assert_eq!(len, 2);
}

#[test]
fn test_writeuint_zero() {
    let mut len: i32 = 0;
    let r = ft_printf::writeuint(0, &mut len);
    assert_eq!(r, 1);
    assert_eq!(len, 1);
}

#[test]
fn test_writeuint_4294967295() {
    let mut len: i32 = 0;
    let r = ft_printf::writeuint(4294967295u64, &mut len);
    assert_eq!(r, 1);
    assert_eq!(len, 10);
}

#[test]
fn test_writeuint_100() {
    let mut len: i32 = 0;
    let r = ft_printf::writeuint(100, &mut len);
    assert_eq!(r, 1);
    assert_eq!(len, 3);
}

// ---------------- writehex ----------------
#[test]
fn test_writehex_42_lower() {
    let mut len: i32 = 0;
    let r = ft_printf::writehex(42, 'x', &mut len);
    assert_eq!(r, 1);
    assert_eq!(len, 2);
}

#[test]
fn test_writehex_42_upper() {
    let mut len: i32 = 0;
    let r = ft_printf::writehex(42, 'X', &mut len);
    assert_eq!(r, 1);
    assert_eq!(len, 2);
}

#[test]
fn test_writehex_zero_lower() {
    let mut len: i32 = 0;
    let r = ft_printf::writehex(0, 'x', &mut len);
    assert_eq!(r, 1);
    assert_eq!(len, 1);
}

#[test]
fn test_writehex_zero_upper() {
    let mut len: i32 = 0;
    let r = ft_printf::writehex(0, 'X', &mut len);
    assert_eq!(r, 1);
    assert_eq!(len, 1);
}

#[test]
fn test_writehex_255_lower() {
    let mut len: i32 = 0;
    let r = ft_printf::writehex(255, 'x', &mut len);
    assert_eq!(r, 1);
    assert_eq!(len, 2);
}

#[test]
fn test_writehex_255_upper() {
    let mut len: i32 = 0;
    let r = ft_printf::writehex(255, 'X', &mut len);
    assert_eq!(r, 1);
    assert_eq!(len, 2);
}

#[test]
fn test_writehex_4096_lower() {
    let mut len: i32 = 0;
    let r = ft_printf::writehex(4096, 'x', &mut len);
    assert_eq!(r, 1);
    assert_eq!(len, 4);
}

#[test]
fn test_writehex_deadbeef_lower() {
    let mut len: i32 = 0;
    let r = ft_printf::writehex(0xdeadbeef, 'x', &mut len);
    assert_eq!(r, 1);
    assert_eq!(len, 8);
}

// ---------------- writepoint ----------------
#[test]
fn test_writepoint_basic() {
    let mut len: i32 = 0;
    let ptr = 0x1234usize as *const std::ffi::c_void;
    let r = ft_printf::writepoint(ptr, &mut len);
    assert_eq!(r, 1);
    // "0x" + "1234" = 6 chars
    assert_eq!(len, 6);
}

#[test]
fn test_writepoint_null() {
    let mut len: i32 = 0;
    let ptr = std::ptr::null::<std::ffi::c_void>();
    let r = ft_printf::writepoint(ptr, &mut len);
    assert_eq!(r, 1);
    // "(nil)" = 5 chars
    assert_eq!(len, 5);
}

#[test]
fn test_writepoint_0xff() {
    let mut len: i32 = 0;
    let ptr = 0xffusize as *const std::ffi::c_void;
    let r = ft_printf::writepoint(ptr, &mut len);
    assert_eq!(r, 1);
    // "0x" + "ff" = 4 chars
    assert_eq!(len, 4);
}

// ---------------- format ----------------
#[test]
fn test_format_percent_literal() {
    use std::any::Any;
    let args: Vec<Box<dyn Any>> = vec![Box::new(0i32)];
    let mut len: i32 = 0;
    let r = ft_printf::format(&args, '%', &mut len);
    assert_eq!(r, 1);
    assert_eq!(len, 1);
}

#[test]
fn test_format_unknown_specifier_returns_neg1() {
    use std::any::Any;
    let args: Vec<Box<dyn Any>> = vec![Box::new(0i32)];
    let mut len: i32 = 0;
    let r = ft_printf::format(&args, 'q', &mut len);
    assert_eq!(r, -1);
}

#[test]
fn test_format_d_specifier() {
    use std::any::Any;
    let args: Vec<Box<dyn Any>> = vec![Box::new(42i32)];
    let mut len: i32 = 0;
    let r = ft_printf::format(&args, 'd', &mut len);
    assert_eq!(r, 1);
    assert_eq!(len, 2);
}

#[test]
fn test_format_s_specifier() {
    use std::any::Any;
    let args: Vec<Box<dyn Any>> = vec![Box::new("World!")];
    let mut len: i32 = 0;
    let r = ft_printf::format(&args, 's', &mut len);
    assert_eq!(r, 1);
    assert_eq!(len, 6);
}

#[test]
fn test_format_x_specifier() {
    use std::any::Any;
    let args: Vec<Box<dyn Any>> = vec![Box::new(255u32)];
    let mut len: i32 = 0;
    let r = ft_printf::format(&args, 'x', &mut len);
    assert_eq!(r, 1);
    assert_eq!(len, 2);
}

fn main() {}
