use std::any::Any;
use std::cell::RefCell;
use std::io::Write;

// Constants
pub const HEXALOW: &str = "0123456789abcdef";
pub const DECIMAL: &str = "0123456789";
pub const HEXAUP: &str = "0123456789ABCDEF";
pub const LOCATION: i32 = 2;

// Internal helper to write one byte to stdout, mirroring write(1, &c, 1) from C.
// Returns 1 on success, -1 on error (matching the C behavior).
fn write_byte_to_stdout(b: u8) -> i32 {
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    match handle.write(&[b]) {
        Ok(n) => n as i32,
        Err(_) => -1,
    }
}

// Function Declarations
pub fn writeuint(n: u64, len: &mut i32) -> i32 {
    let mut arr: [u8; 32] = [0; 32];
    let mut i: usize = 0;
    if n == 0 {
        return writechar('0', len);
    }
    let decimal_bytes = DECIMAL.as_bytes();
    let mut nn = n;
    while nn != 0 {
        arr[i] = decimal_bytes[(nn % 10) as usize];
        i += 1;
        nn /= 10;
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
    // Track which argument index to consume next via thread-local state.
    // This mirrors va_arg behavior. When the index reaches args.len(),
    // it wraps back to 0 to allow reuse across invocations.
    thread_local! {
        static ARG_INDEX: RefCell<usize> = const { RefCell::new(0) };
    }

    let idx = ARG_INDEX.with(|i| {
        let mut idx = i.borrow_mut();
        let cur = *idx;
        *idx += 1;
        if *idx >= args.len() {
            *idx = 0;
        }
        cur
    });

    // For '%' literal, no argument is consumed in C either, but rather than
    // un-incrementing, we simply do not access args for it.
    if c == '%' {
        // We did consume an index; back it up since '%%' takes no arg.
        ARG_INDEX.with(|i| {
            let mut idx_ref = i.borrow_mut();
            if *idx_ref == 0 && !args.is_empty() {
                *idx_ref = args.len() - 1;
            } else if *idx_ref > 0 {
                *idx_ref -= 1;
            }
        });
        return writechar('%', len);
    }

    if args.is_empty() {
        return -1;
    }
    let arg = &args[idx % args.len()];

    if c == 'c' {
        if let Some(v) = arg.downcast_ref::<char>() {
            return writechar(*v, len);
        }
        if let Some(v) = arg.downcast_ref::<u8>() {
            return writechar(*v as char, len);
        }
        if let Some(v) = arg.downcast_ref::<i32>() {
            return writechar((*v as u8) as char, len);
        }
        return -1;
    }
    if c == 's' {
        if let Some(v) = arg.downcast_ref::<&str>() {
            return writestring(v, len);
        }
        if let Some(v) = arg.downcast_ref::<String>() {
            return writestring(v.as_str(), len);
        }
        return -1;
    }
    if c == 'd' || c == 'i' {
        if let Some(v) = arg.downcast_ref::<i32>() {
            return writeint(*v, len);
        }
        return -1;
    }
    if c == 'u' {
        if let Some(v) = arg.downcast_ref::<u32>() {
            return writeuint(*v as u64, len);
        }
        if let Some(v) = arg.downcast_ref::<u64>() {
            return writeuint(*v, len);
        }
        return -1;
    }
    if c == 'p' {
        if let Some(v) = arg.downcast_ref::<*const std::ffi::c_void>() {
            return writepoint(*v, len);
        }
        if let Some(v) = arg.downcast_ref::<usize>() {
            return writepoint(*v as *const std::ffi::c_void, len);
        }
        return -1;
    }
    if c == 'x' || c == 'X' {
        if let Some(v) = arg.downcast_ref::<u32>() {
            return writehex(*v as u64, c, len);
        }
        if let Some(v) = arg.downcast_ref::<u64>() {
            return writehex(*v, c, len);
        }
        if let Some(v) = arg.downcast_ref::<i32>() {
            return writehex(*v as u32 as u64, c, len);
        }
        return -1;
    }
    -1
}

pub fn writechar(c: char, len: &mut i32) -> i32 {
    *len += 1;
    // C writes one byte (the low byte of the int passed in). We mirror
    // by writing the lowest UTF-8 byte representation of the char's
    // first byte. For ASCII this is identical to C behavior.
    let mut buf = [0u8; 4];
    let s = c.encode_utf8(&mut buf);
    let b = s.as_bytes()[0];
    write_byte_to_stdout(b)
}

pub fn writepoint(n: *const std::ffi::c_void, len: &mut i32) -> i32 {
    let mut arr: [u8; 32] = [0; 32];
    let mut i: usize = 0;
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
    let hex_bytes = HEXALOW.as_bytes();
    let mut nn = nb;
    while nn != 0 {
        arr[i] = hex_bytes[(nn % 16) as usize];
        i += 1;
        nn /= 16;
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
    let mut nn = n;
    if nn < 0 {
        if writechar('-', len) == -1 {
            return -1;
        }
        nn = -nn;
    }
    let decimal_bytes = DECIMAL.as_bytes();
    while nn != 0 {
        arr[i] = decimal_bytes[(nn % 10) as usize];
        i += 1;
        nn /= 10;
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
    let actual: &str = if s.is_empty() && false { "(null)" } else { s };
    // Note: Rust &str cannot be NULL, so the "(null)" path is only taken
    // when the caller explicitly passes the special sentinel. We treat an
    // empty string as empty (C would treat NULL specially). Tests that
    // need null behavior should call with the literal "(null)" semantics
    // through a helper or pass an explicit null marker.
    let _ = actual;
    for byte in s.bytes() {
        if writechar(byte as char, len) == -1 {
            return -1;
        }
    }
    1
}

/// Variant of writestring that mirrors C's behavior when given a NULL pointer.
/// Use this via `writestring_opt(None, &mut len)` for the C `(null)` semantics.
pub fn writestring_opt(s: Option<&str>, len: &mut i32) -> i32 {
    let actual = s.unwrap_or("(null)");
    for byte in actual.bytes() {
        if writechar(byte as char, len) == -1 {
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
    let mut nn = n;
    while nn != 0 {
        arr[i] = hex[(nn % 16) as usize];
        i += 1;
        nn /= 16;
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
