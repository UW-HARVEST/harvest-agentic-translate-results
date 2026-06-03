use std::any::Any;
use std::io::Write;
// Constants
pub const HEXALOW: &str = "0123456789abcdef";
pub const DECIMAL: &str = "0123456789";
pub const HEXAUP: &str = "0123456789ABCDEF";
pub const LOCATION: i32 = 2;

// Thread-local index used by `format` to walk through the args slice in order
// when the ft_printf! macro processes successive format specifiers.
thread_local! {
    static FORMAT_IDX: std::cell::Cell<usize> = std::cell::Cell::new(0);
}

// Function Declarations
pub fn writeuint(n: u64, len: &mut i32) -> i32 {
    if n == 0 {
        return writechar('0', len);
    }
    let mut arr: [u8; 32] = [0; 32];
    let mut i: usize = 0;
    let mut n = n;
    let decimal_bytes = DECIMAL.as_bytes();
    while n > 0 {
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
    // Pull the next argument index for this format specifier.
    let idx = FORMAT_IDX.with(|f| {
        let v = f.get();
        f.set(v + 1);
        v
    });
    // For literal '%', no argument is consumed; rewind the index.
    if c == '%' {
        FORMAT_IDX.with(|f| f.set(idx));
        return writechar('%', len);
    }
    if idx >= args.len() {
        return -1;
    }
    let arg: &dyn Any = &*args[idx];
    match c {
        'c' => {
            if let Some(ch) = arg.downcast_ref::<char>() {
                writechar(*ch, len)
            } else if let Some(b) = arg.downcast_ref::<u8>() {
                writechar(*b as char, len)
            } else if let Some(i) = arg.downcast_ref::<i32>() {
                writechar(*i as u8 as char, len)
            } else {
                -1
            }
        }
        's' => {
            if let Some(s) = arg.downcast_ref::<&str>() {
                writestring(s, len)
            } else if let Some(s) = arg.downcast_ref::<String>() {
                writestring(s.as_str(), len)
            } else {
                -1
            }
        }
        'd' | 'i' => {
            if let Some(i) = arg.downcast_ref::<i32>() {
                writeint(*i, len)
            } else if let Some(i) = arg.downcast_ref::<i64>() {
                writeint(*i as i32, len)
            } else if let Some(u) = arg.downcast_ref::<u32>() {
                writeint(*u as i32, len)
            } else {
                -1
            }
        }
        'u' => {
            if let Some(u) = arg.downcast_ref::<u32>() {
                writeuint(*u as u64, len)
            } else if let Some(u) = arg.downcast_ref::<u64>() {
                writeuint(*u, len)
            } else if let Some(i) = arg.downcast_ref::<i32>() {
                writeuint(*i as u32 as u64, len)
            } else {
                -1
            }
        }
        'p' => {
            if let Some(p) = arg.downcast_ref::<*const std::ffi::c_void>() {
                writepoint(*p, len)
            } else if let Some(p) = arg.downcast_ref::<*mut std::ffi::c_void>() {
                writepoint(*p as *const std::ffi::c_void, len)
            } else if let Some(u) = arg.downcast_ref::<usize>() {
                writepoint(*u as *const std::ffi::c_void, len)
            } else {
                -1
            }
        }
        'x' | 'X' => {
            if let Some(u) = arg.downcast_ref::<u32>() {
                writehex(*u as u64, c, len)
            } else if let Some(u) = arg.downcast_ref::<u64>() {
                writehex(*u, c, len)
            } else if let Some(i) = arg.downcast_ref::<i32>() {
                writehex(*i as u32 as u64, c, len)
            } else {
                -1
            }
        }
        _ => -1,
    }
}

pub fn writechar(c: char, len: &mut i32) -> i32 {
    *len += 1;
    let mut buf = [0u8; 4];
    let s = c.encode_utf8(&mut buf);
    let bytes = s.as_bytes();
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    match handle.write(bytes) {
        Ok(_) => 1,
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
    let mut arr: [u8; 32] = [0; 32];
    let mut i: usize = 0;
    let hex = HEXALOW.as_bytes();
    while nb > 0 {
        arr[i] = hex[(nb % 16) as usize];
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
    let mut arr: [u8; 16] = [0; 16];
    let mut i: usize = 0;
    let decimal_bytes = DECIMAL.as_bytes();
    while n > 0 {
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
    // In Rust, &str cannot be null, so the C `if (!s) s = "(null)"` check is
    // not applicable here.
    for c in s.chars() {
        if writechar(c, len) == -1 {
            return -1;
        }
    }
    1
}

pub fn writehex(n: u64, c: char, len: &mut i32) -> i32 {
    let mut arr: [u8; 32] = [0; 32];
    let mut i: usize = 0;
    let mut n = n;
    if n == 0 {
        arr[i] = b'0';
        i += 1;
    }
    let hex = if c == 'x' { HEXALOW.as_bytes() } else { HEXAUP.as_bytes() };
    while n > 0 {
        arr[i] = hex[(n % 16) as usize];
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
