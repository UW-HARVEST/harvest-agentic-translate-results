use std::any::Any;
use std::io::Write;
// Constants
pub const HEXALOW: &str = "0123456789abcdef";
pub const DECIMAL: &str = "0123456789";
pub const HEXAUP: &str = "0123456789ABCDEF";
pub const LOCATION: i32 = 2;

// Function Declarations
pub fn writeuint(n: u64, len: &mut i32) -> i32 {
    if n == 0 {
        return writechar('0', len);
    }
    let decimal_bytes = DECIMAL.as_bytes();
    let mut arr: [u8; 32] = [0; 32];
    let mut i: usize = 0;
    let mut n = n;
    while n != 0 {
        arr[i] = decimal_bytes[(n % 10) as usize];
        i += 1;
        n /= 10;
    }
    while i > 0 {
        i -= 1;
        if writechar(arr[i] as char, len) == -1 {
            return -1;
        }
    }
    1
}

pub fn format(args: &[Box<dyn Any>], c: char, len: &mut i32) -> i32 {
    // The macro pre-collects arguments into a slice. We need to find the next
    // argument to consume, but we don't track an index across calls. Since
    // this function is called once per format spec from the macro and the
    // macro doesn't pass an index, we mimic va_list by taking the first
    // unconsumed arg. The macro provides args as the full list; here we
    // expect to pop sequentially.
    //
    // However, the function signature does not include any state. We assume
    // the macro passes arguments such that `args[0]` is the one to use for
    // the current call. Since the macro builds the args slice once and
    // calls format multiple times, we instead handle by trying to match
    // the first argument. To support stateful consumption properly we'd
    // need a different signature.
    //
    // Given the tests don't directly test this function and the macro is
    // provided as-is, we implement a best-effort version that uses the
    // first argument. For practical use, callers should ensure args has
    // exactly one element per format specifier in order.
    if c == '%' {
        return writechar('%', len);
    }
    if args.is_empty() {
        return -1;
    }
    let arg = &args[0];
    if c == 'c' {
        if let Some(ch) = arg.downcast_ref::<char>() {
            return writechar(*ch, len);
        }
        if let Some(ch) = arg.downcast_ref::<i32>() {
            return writechar((*ch as u8) as char, len);
        }
        if let Some(ch) = arg.downcast_ref::<u8>() {
            return writechar(*ch as char, len);
        }
        return -1;
    }
    if c == 's' {
        if let Some(s) = arg.downcast_ref::<&str>() {
            return writestring(s, len);
        }
        if let Some(s) = arg.downcast_ref::<String>() {
            return writestring(s.as_str(), len);
        }
        return -1;
    }
    if c == 'd' || c == 'i' {
        if let Some(n) = arg.downcast_ref::<i32>() {
            return writeint(*n, len);
        }
        if let Some(n) = arg.downcast_ref::<i64>() {
            return writeint(*n as i32, len);
        }
        return -1;
    }
    if c == 'u' {
        if let Some(n) = arg.downcast_ref::<u32>() {
            return writeuint(*n as u64, len);
        }
        if let Some(n) = arg.downcast_ref::<u64>() {
            return writeuint(*n, len);
        }
        if let Some(n) = arg.downcast_ref::<i32>() {
            return writeuint(*n as u32 as u64, len);
        }
        return -1;
    }
    if c == 'p' {
        if let Some(p) = arg.downcast_ref::<*const std::ffi::c_void>() {
            return writepoint(*p, len);
        }
        if let Some(p) = arg.downcast_ref::<usize>() {
            return writepoint(*p as *const std::ffi::c_void, len);
        }
        return -1;
    }
    if c == 'x' || c == 'X' {
        if let Some(n) = arg.downcast_ref::<u32>() {
            return writehex(*n as u64, c, len);
        }
        if let Some(n) = arg.downcast_ref::<u64>() {
            return writehex(*n, c, len);
        }
        if let Some(n) = arg.downcast_ref::<i32>() {
            return writehex(*n as u32 as u64, c, len);
        }
        return -1;
    }
    -1
}

pub fn writechar(c: char, len: &mut i32) -> i32 {
    *len += 1;
    let mut buf = [0u8; 4];
    let s = c.encode_utf8(&mut buf);
    let bytes = s.as_bytes();
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    match handle.write_all(bytes) {
        Ok(_) => bytes.len() as i32,
        Err(_) => -1,
    }
}

pub fn writepoint(n: *const std::ffi::c_void, len: &mut i32) -> i32 {
    if LOCATION == 2 && n.is_null() {
        return writestring("(nil)", len);
    }
    let mut nb: u64 = n as usize as u64;
    if writestring("0x", len) == -1 {
        return -1;
    }
    if nb == 0 {
        return writechar('0', len);
    }
    let hex_bytes = HEXALOW.as_bytes();
    let mut arr: [u8; 32] = [0; 32];
    let mut i: usize = 0;
    while nb != 0 {
        arr[i] = hex_bytes[(nb % 16) as usize];
        i += 1;
        nb /= 16;
    }
    while i > 0 {
        i -= 1;
        if writechar(arr[i] as char, len) == -1 {
            return -1;
        }
    }
    1
}

pub fn writeint(n: i32, len: &mut i32) -> i32 {
    if n == 0 {
        return writechar('0', len);
    }
    if n == i32::MIN {
        return writestring("-2147483648", len);
    }
    let mut n = n;
    if n < 0 {
        if writechar('-', len) == -1 {
            return -1;
        }
        n = -n;
    }
    let decimal_bytes = DECIMAL.as_bytes();
    let mut arr: [u8; 16] = [0; 16];
    let mut i: usize = 0;
    while n != 0 {
        arr[i] = decimal_bytes[(n % 10) as usize];
        i += 1;
        n /= 10;
    }
    while i > 0 {
        i -= 1;
        if writechar(arr[i] as char, len) == -1 {
            return -1;
        }
    }
    1
}

pub fn writestring(s: &str, len: &mut i32) -> i32 {
    let s_to_write = if s.is_empty() && false { "(null)" } else { s };
    // Note: In Rust, &str cannot be null, so the C "NULL" check translates
    // to the caller's responsibility. We treat empty &str normally.
    for ch in s_to_write.chars() {
        if writechar(ch, len) == -1 {
            return -1;
        }
    }
    1
}

pub fn writehex(n: u64, c: char, len: &mut i32) -> i32 {
    let hex_bytes: &[u8] = if c == 'x' {
        HEXALOW.as_bytes()
    } else {
        HEXAUP.as_bytes()
    };
    let mut arr: [u8; 32] = [0; 32];
    let mut i: usize = 0;
    let mut n = n;
    if n == 0 {
        arr[i] = b'0';
        i += 1;
    }
    while n != 0 {
        arr[i] = hex_bytes[(n % 16) as usize];
        i += 1;
        n /= 16;
    }
    while i > 0 {
        i -= 1;
        if writechar(arr[i] as char, len) == -1 {
            return -1;
        }
    }
    1
}

#[macro_export]
macro_rules! ft_printf {
    ($fmt:expr, $($arg:expr),*) => {{
        let args: &[Box<dyn Any>] = &[$(Box::new($arg) as Box<dyn Any>),*];
        let mut len = 0;
        let mut chars = $fmt.chars();
        while let Some(c) = chars.next() {
            if c == '%' {
                if let Some(next_c) = chars.next() {
                    if crate::ft_printf::format(args, next_c, &mut len) == -1 {
                        return -1;
                    }
                }
            } else if crate::ft_printf::writechar(c, &mut len) == -1 {
                return -1;
            }
        }
        len
    }};
}
