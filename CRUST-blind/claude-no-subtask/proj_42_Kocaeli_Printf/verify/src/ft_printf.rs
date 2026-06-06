use std::any::Any;
use std::io::Write;
use std::sync::atomic::{AtomicUsize, Ordering};

// Constants
pub const HEXALOW: &str = "0123456789abcdef";
pub const DECIMAL: &str = "0123456789";
pub const HEXAUP: &str = "0123456789ABCDEF";
pub const LOCATION: i32 = 2;

// Tracks which positional argument the next `%`-specifier should consume.
// The `ft_printf!` macro hands the same `args` slice to every call to
// `format`, so we use a static counter that wraps around once it reaches
// the end of the slice. This lets `format` advance through the arguments
// in order, mirroring the C `va_arg` behaviour, while still working for
// repeated invocations of the macro.
static ARG_INDEX: AtomicUsize = AtomicUsize::new(0);

fn next_arg_index(total: usize) -> usize {
    if total == 0 {
        return 0;
    }
    loop {
        let cur = ARG_INDEX.load(Ordering::SeqCst);
        let idx = if cur >= total { 0 } else { cur };
        let next = idx + 1;
        if ARG_INDEX
            .compare_exchange(cur, next, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            return idx;
        }
    }
}

// Function Declarations
pub fn writeuint(n: u64, len: &mut i32) -> i32 {
    let mut arr: [u8; 32] = [0; 32];
    let mut i: usize = 0;
    if n == 0 {
        return writechar('0', len);
    }
    let bytes = DECIMAL.as_bytes();
    let mut n = n;
    while n != 0 {
        arr[i] = bytes[(n % 10) as usize];
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
    if c == '%' {
        return writechar('%', len);
    }
    if args.is_empty() {
        return -1;
    }
    let idx = next_arg_index(args.len());
    let arg = &args[idx];

    match c {
        'c' => {
            if let Some(ch) = arg.downcast_ref::<char>() {
                writechar(*ch, len)
            } else if let Some(ch) = arg.downcast_ref::<i32>() {
                writechar((*ch as u8) as char, len)
            } else if let Some(ch) = arg.downcast_ref::<u8>() {
                writechar(*ch as char, len)
            } else if let Some(ch) = arg.downcast_ref::<u32>() {
                writechar((*ch as u8) as char, len)
            } else {
                -1
            }
        }
        's' => {
            if let Some(s) = arg.downcast_ref::<&str>() {
                writestring(*s, len)
            } else if let Some(s) = arg.downcast_ref::<String>() {
                writestring(s.as_str(), len)
            } else if let Some(s) = arg.downcast_ref::<&String>() {
                writestring(s.as_str(), len)
            } else {
                -1
            }
        }
        'd' | 'i' => {
            if let Some(n) = arg.downcast_ref::<i32>() {
                writeint(*n, len)
            } else if let Some(n) = arg.downcast_ref::<i64>() {
                writeint(*n as i32, len)
            } else if let Some(n) = arg.downcast_ref::<u32>() {
                writeint(*n as i32, len)
            } else if let Some(n) = arg.downcast_ref::<i16>() {
                writeint(*n as i32, len)
            } else if let Some(n) = arg.downcast_ref::<i8>() {
                writeint(*n as i32, len)
            } else if let Some(n) = arg.downcast_ref::<isize>() {
                writeint(*n as i32, len)
            } else if let Some(n) = arg.downcast_ref::<usize>() {
                writeint(*n as i32, len)
            } else {
                -1
            }
        }
        'u' => {
            if let Some(n) = arg.downcast_ref::<u32>() {
                writeuint(*n as u64, len)
            } else if let Some(n) = arg.downcast_ref::<u64>() {
                writeuint(*n, len)
            } else if let Some(n) = arg.downcast_ref::<i32>() {
                writeuint((*n as u32) as u64, len)
            } else if let Some(n) = arg.downcast_ref::<usize>() {
                writeuint(*n as u64, len)
            } else if let Some(n) = arg.downcast_ref::<u16>() {
                writeuint(*n as u64, len)
            } else if let Some(n) = arg.downcast_ref::<u8>() {
                writeuint(*n as u64, len)
            } else {
                -1
            }
        }
        'p' => {
            if let Some(p) = arg.downcast_ref::<*const std::ffi::c_void>() {
                writepoint(*p, len)
            } else if let Some(p) = arg.downcast_ref::<*mut std::ffi::c_void>() {
                writepoint(*p as *const std::ffi::c_void, len)
            } else if let Some(p) = arg.downcast_ref::<*const u8>() {
                writepoint(*p as *const std::ffi::c_void, len)
            } else if let Some(p) = arg.downcast_ref::<*mut u8>() {
                writepoint(*p as *const std::ffi::c_void, len)
            } else if let Some(p) = arg.downcast_ref::<usize>() {
                writepoint(*p as *const std::ffi::c_void, len)
            } else {
                -1
            }
        }
        'x' | 'X' => {
            if let Some(n) = arg.downcast_ref::<u32>() {
                writehex(*n as u64, c, len)
            } else if let Some(n) = arg.downcast_ref::<u64>() {
                writehex(*n, c, len)
            } else if let Some(n) = arg.downcast_ref::<i32>() {
                writehex((*n as u32) as u64, c, len)
            } else if let Some(n) = arg.downcast_ref::<usize>() {
                writehex(*n as u64, c, len)
            } else if let Some(n) = arg.downcast_ref::<u16>() {
                writehex(*n as u64, c, len)
            } else if let Some(n) = arg.downcast_ref::<u8>() {
                writehex(*n as u64, c, len)
            } else {
                -1
            }
        }
        _ => -1,
    }
}

pub fn writechar(c: char, len: &mut i32) -> i32 {
    // Mirror the C `(*len)++, write(1, &c, 1)` behaviour: bump the length
    // unconditionally and return the byte count written (1 on success,
    // -1 on failure).
    *len += 1;
    let mut buf = [0u8; 4];
    let encoded = c.encode_utf8(&mut buf);
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    match handle.write_all(encoded.as_bytes()) {
        Ok(()) => 1,
        Err(_) => -1,
    }
}

pub fn writepoint(n: *const std::ffi::c_void, len: &mut i32) -> i32 {
    let nb = n as usize as u64;
    if LOCATION == 2 && n.is_null() {
        return writestring("(nil)", len);
    }
    if writestring("0x", len) == -1 {
        return -1;
    }
    if nb == 0 {
        return writechar('0', len);
    }
    let mut arr: [u8; 32] = [0; 32];
    let mut i: usize = 0;
    let hex = HEXALOW.as_bytes();
    let mut v = nb;
    while v != 0 {
        arr[i] = hex[(v % 16) as usize];
        i += 1;
        v /= 16;
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
    let mut arr: [u8; 16] = [0; 16];
    let mut i: usize = 0;
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
    let bytes = DECIMAL.as_bytes();
    while n != 0 {
        arr[i] = bytes[(n % 10) as usize];
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
    // The original C accepts NULL and substitutes "(null)". In Rust the
    // signature only allows valid &str values, so the caller has to
    // pass "(null)" explicitly when desired. An empty string writes
    // nothing and succeeds, matching the C `while (*s)` loop.
    for ch in s.chars() {
        if writechar(ch, len) == -1 {
            return -1;
        }
    }
    1
}

pub fn writehex(n: u64, c: char, len: &mut i32) -> i32 {
    let mut arr: [u8; 32] = [0; 32];
    let mut i: usize = 0;
    if n == 0 {
        arr[i] = b'0';
        i += 1;
    }
    let hex = if c == 'x' {
        HEXALOW.as_bytes()
    } else {
        HEXAUP.as_bytes()
    };
    let mut v = n;
    while v != 0 {
        arr[i] = hex[(v % 16) as usize];
        i += 1;
        v /= 16;
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
