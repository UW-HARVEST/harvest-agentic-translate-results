use std::any::Any;
use std::io::Write;

// Constants
pub const HEXALOW: &str = "0123456789abcdef";
pub const DECIMAL: &str = "0123456789";
pub const HEXAUP: &str = "0123456789ABCDEF";
pub const LOCATION: i32 = 2;

// Function Declarations
pub fn writeuint(n: u64, len: &mut i32) -> i32 {
    let mut arr: [u8; 32] = [0; 32];
    let mut i: usize = 0;
    if n == 0 {
        return writechar('0', len);
    }
    let decimal_bytes = DECIMAL.as_bytes();
    let mut nb = n;
    while nb != 0 {
        arr[i] = decimal_bytes[(nb % 10) as usize];
        i += 1;
        nb /= 10;
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
    // Use a static counter to track which arg we are on per-call.
    // Since this is called repeatedly within the macro and we don't have
    // mutable state, we keep an internal index using a thread-local.
    use std::cell::Cell;
    thread_local! {
        static ARG_INDEX: Cell<usize> = Cell::new(0);
    }

    let idx = ARG_INDEX.with(|i| {
        let v = i.get();
        i.set(v + 1);
        v
    });

    // Reset index if we go past the args (handle simple case for new macro invocations)
    if idx >= args.len() && c != '%' {
        ARG_INDEX.with(|i| i.set(0));
        return -1;
    }

    let result = if c == 'c' {
        let arg = args.get(idx).and_then(|a| {
            a.downcast_ref::<char>()
                .copied()
                .or_else(|| a.downcast_ref::<i32>().map(|v| *v as u8 as char))
                .or_else(|| a.downcast_ref::<u8>().map(|v| *v as char))
        });
        match arg {
            Some(ch) => writechar(ch, len),
            None => -1,
        }
    } else if c == 's' {
        let arg = args.get(idx).and_then(|a| {
            a.downcast_ref::<&str>()
                .map(|s| s.to_string())
                .or_else(|| a.downcast_ref::<String>().cloned())
        });
        match arg {
            Some(s) => writestring(&s, len),
            None => -1,
        }
    } else if c == 'd' || c == 'i' {
        let arg = args.get(idx).and_then(|a| {
            a.downcast_ref::<i32>()
                .copied()
                .or_else(|| a.downcast_ref::<i64>().map(|v| *v as i32))
        });
        match arg {
            Some(n) => writeint(n, len),
            None => -1,
        }
    } else if c == 'u' {
        let arg = args.get(idx).and_then(|a| {
            a.downcast_ref::<u32>()
                .map(|v| *v as u64)
                .or_else(|| a.downcast_ref::<u64>().copied())
                .or_else(|| a.downcast_ref::<i32>().map(|v| *v as u32 as u64))
        });
        match arg {
            Some(n) => writeuint(n, len),
            None => -1,
        }
    } else if c == 'p' {
        let arg = args.get(idx).and_then(|a| {
            a.downcast_ref::<*const std::ffi::c_void>()
                .copied()
                .or_else(|| a.downcast_ref::<usize>().map(|v| *v as *const std::ffi::c_void))
        });
        match arg {
            Some(p) => writepoint(p, len),
            None => -1,
        }
    } else if c == 'x' || c == 'X' {
        let arg = args.get(idx).and_then(|a| {
            a.downcast_ref::<u32>()
                .map(|v| *v as u64)
                .or_else(|| a.downcast_ref::<u64>().copied())
                .or_else(|| a.downcast_ref::<i32>().map(|v| *v as u32 as u64))
        });
        match arg {
            Some(n) => writehex(n, c, len),
            None => -1,
        }
    } else if c == '%' {
        // Percent: rewind index, since '%' doesn't consume an argument
        ARG_INDEX.with(|i| i.set(idx));
        writechar('%', len)
    } else {
        -1
    };

    result
}

pub fn writechar(c: char, len: &mut i32) -> i32 {
    *len += 1;
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    let mut buf = [0u8; 4];
    let s = c.encode_utf8(&mut buf);
    match handle.write_all(s.as_bytes()) {
        Ok(_) => 1,
        Err(_) => -1,
    }
}

pub fn writepoint(n: *const std::ffi::c_void, len: &mut i32) -> i32 {
    if LOCATION == 2 && n.is_null() {
        return writestring("(nil)", len);
    }
    let mut arr: [u8; 32] = [0; 32];
    let mut i: usize = 0;
    let mut nb = n as u64;
    if writestring("0x", len) == -1 {
        return -1;
    }
    if nb == 0 {
        return writechar('0', len);
    }
    let hex_bytes = HEXALOW.as_bytes();
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
    let mut arr: [u8; 16] = [0; 16];
    let mut i: usize = 0;
    if n == 0 {
        return writechar('0', len);
    }
    if n == i32::MIN {
        return writestring("-2147483648", len);
    }
    let mut nb = n;
    if nb < 0 {
        if writechar('-', len) == -1 {
            return -1;
        }
        nb = -nb;
    }
    let decimal_bytes = DECIMAL.as_bytes();
    while nb != 0 {
        arr[i] = decimal_bytes[(nb % 10) as usize];
        i += 1;
        nb /= 10;
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
    // The C version replaces NULL with "(null)". In Rust, we receive &str directly,
    // so we don't have a NULL case for normal usage; we just iterate.
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
    let mut nb = n;
    if nb == 0 {
        arr[i] = b'0';
        i += 1;
    }
    let hex_bytes = if c == 'x' {
        HEXALOW.as_bytes()
    } else {
        HEXAUP.as_bytes()
    };
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
