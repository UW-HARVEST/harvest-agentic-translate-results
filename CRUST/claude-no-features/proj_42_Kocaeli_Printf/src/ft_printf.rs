use std::any::Any;
use std::io::Write;
use std::sync::atomic::{AtomicUsize, Ordering};

// Constants
pub const HEXALOW: &str = "0123456789abcdef";
pub const DECIMAL: &str = "0123456789";
pub const HEXAUP: &str = "0123456789ABCDEF";
pub const LOCATION: i32 = 2;

// Internal: index counter used by `format` to walk through the args slice
// passed in by the `ft_printf!` macro.
static FORMAT_ARG_INDEX: AtomicUsize = AtomicUsize::new(0);

// Helper to fetch a byte from a constant ASCII digit string by index.
fn digit_at(table: &str, idx: usize) -> char {
    table.as_bytes()[idx] as char
}

// Function Declarations
pub fn writeuint(n: u64, len: &mut i32) -> i32 {
    let mut arr: [char; 32] = ['\0'; 32];
    let mut i: usize = 0;
    let mut n = n;
    if n == 0 {
        return writechar('0', len);
    }
    while n != 0 {
        arr[i] = digit_at(DECIMAL, (n % 10) as usize);
        i += 1;
        n /= 10;
    }
    while i > 0 {
        i -= 1;
        if writechar(arr[i], len) == -1 {
            return -1;
        }
    }
    1
}

pub fn format(args: &[Box<dyn Any>], c: char, len: &mut i32) -> i32 {
    // Pick the next argument. The `ft_printf!` macro doesn't track an index
    // itself, so we maintain a process-global cursor here. It is reset to 0
    // by the macro implicitly via this counter being read each time.
    let idx = FORMAT_ARG_INDEX.fetch_add(1, Ordering::SeqCst);

    if c == '%' {
        // '%%' doesn't consume an argument, so reverse the increment.
        FORMAT_ARG_INDEX.fetch_sub(1, Ordering::SeqCst);
        return writechar('%', len);
    }

    let arg = match args.get(idx) {
        Some(a) => a,
        None => return -1,
    };

    match c {
        'c' => {
            if let Some(ch) = arg.downcast_ref::<char>() {
                writechar(*ch, len)
            } else if let Some(b) = arg.downcast_ref::<u8>() {
                writechar(*b as char, len)
            } else if let Some(i) = arg.downcast_ref::<i32>() {
                writechar((*i as u8) as char, len)
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
            if let Some(n) = arg.downcast_ref::<i32>() {
                writeint(*n, len)
            } else if let Some(n) = arg.downcast_ref::<i64>() {
                writeint(*n as i32, len)
            } else if let Some(n) = arg.downcast_ref::<u32>() {
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
                writeuint(*n as u32 as u64, len)
            } else {
                -1
            }
        }
        'p' => {
            if let Some(p) = arg.downcast_ref::<*const std::ffi::c_void>() {
                writepoint(*p, len)
            } else if let Some(p) = arg.downcast_ref::<*mut std::ffi::c_void>() {
                writepoint(*p as *const std::ffi::c_void, len)
            } else if let Some(n) = arg.downcast_ref::<usize>() {
                writepoint(*n as *const std::ffi::c_void, len)
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
                writehex(*n as u32 as u64, c, len)
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
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    match handle.write_all(s.as_bytes()) {
        Ok(_) => 1,
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
    let mut arr: [char; 32] = ['\0'; 32];
    let mut i: usize = 0;
    let mut nb = nb;
    while nb != 0 {
        arr[i] = digit_at(HEXALOW, (nb % 16) as usize);
        i += 1;
        nb /= 16;
    }
    while i > 0 {
        i -= 1;
        if writechar(arr[i], len) == -1 {
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
    let mut arr: [char; 16] = ['\0'; 16];
    let mut i: usize = 0;
    while n != 0 {
        arr[i] = digit_at(DECIMAL, (n % 10) as usize);
        i += 1;
        n /= 10;
    }
    while i > 0 {
        i -= 1;
        if writechar(arr[i], len) == -1 {
            return -1;
        }
    }
    1
}

pub fn writestring(s: &str, len: &mut i32) -> i32 {
    // The Rust `&str` cannot be null, but we match the C semantics: an empty
    // sentinel means we write `"(null)"`. There's no equivalent here, so we
    // just write the given string.
    for ch in s.chars() {
        if writechar(ch, len) == -1 {
            return -1;
        }
    }
    1
}

pub fn writehex(n: u64, c: char, len: &mut i32) -> i32 {
    let hex = if c == 'x' { HEXALOW } else { HEXAUP };
    let mut arr: [char; 32] = ['\0'; 32];
    let mut i: usize = 0;
    if n == 0 {
        arr[i] = '0';
        i += 1;
    }
    let mut n = n;
    while n != 0 {
        arr[i] = digit_at(hex, (n % 16) as usize);
        i += 1;
        n /= 16;
    }
    while i > 0 {
        i -= 1;
        if writechar(arr[i], len) == -1 {
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
