use std::any::Any;
use std::io::Write;

// Constants
pub const HEXALOW: &str = "0123456789abcdef";
pub const DECIMAL: &str = "0123456789";
pub const HEXAUP: &str = "0123456789ABCDEF";
pub const LOCATION: i32 = 2;

// Thread-local index used by `format` to walk through the args slice
// produced by the `ft_printf!` macro. Each call to `format` consumes
// one argument from `args`, mirroring how the C version pulls the
// next argument out of `va_list` on every conversion specifier.
thread_local! {
    static ARG_INDEX: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

fn next_arg_index() -> usize {
    ARG_INDEX.with(|c| {
        let i = c.get();
        c.set(i + 1);
        i
    })
}

fn reset_arg_index_if_done(args_len: usize) {
    ARG_INDEX.with(|c| {
        if c.get() >= args_len {
            c.set(0);
        }
    });
}

// Function Declarations
pub fn writeuint(n: u64, len: &mut i32) -> i32 {
    let mut arr: [u8; 32] = [0; 32];
    let mut i: usize = 0;
    if n == 0 {
        return writechar('0', len);
    }
    let mut nb = n;
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

pub fn format(args: &[Box<dyn Any>], c: char, len: &mut i32) -> i32 {
    // The `%%` specifier does not consume an argument, so handle it
    // up-front before pulling anything out of `args`.
    if c == '%' {
        return writechar('%', len);
    }

    let idx = next_arg_index();
    reset_arg_index_if_done(args.len());

    // If the format char demands an argument but none is available,
    // signal an error like the C version's default branch would.
    if idx >= args.len() {
        return -1;
    }
    let arg: &dyn Any = args[idx].as_ref();

    let result = match c {
        'c' => {
            if let Some(v) = arg.downcast_ref::<char>() {
                writechar(*v, len)
            } else if let Some(v) = arg.downcast_ref::<u8>() {
                writechar(*v as char, len)
            } else if let Some(v) = arg.downcast_ref::<i32>() {
                writechar((*v as u8) as char, len)
            } else {
                -1
            }
        }
        's' => {
            if let Some(v) = arg.downcast_ref::<&str>() {
                writestring(v, len)
            } else if let Some(v) = arg.downcast_ref::<String>() {
                writestring(v.as_str(), len)
            } else {
                -1
            }
        }
        'd' | 'i' => {
            if let Some(v) = arg.downcast_ref::<i32>() {
                writeint(*v, len)
            } else if let Some(v) = arg.downcast_ref::<i64>() {
                writeint(*v as i32, len)
            } else if let Some(v) = arg.downcast_ref::<u32>() {
                writeint(*v as i32, len)
            } else {
                -1
            }
        }
        'u' => {
            if let Some(v) = arg.downcast_ref::<u32>() {
                writeuint(*v as u64, len)
            } else if let Some(v) = arg.downcast_ref::<u64>() {
                writeuint(*v, len)
            } else if let Some(v) = arg.downcast_ref::<i32>() {
                writeuint(*v as u32 as u64, len)
            } else {
                -1
            }
        }
        'p' => {
            if let Some(v) = arg.downcast_ref::<*const std::ffi::c_void>() {
                writepoint(*v, len)
            } else if let Some(v) = arg.downcast_ref::<*mut std::ffi::c_void>() {
                writepoint(*v as *const std::ffi::c_void, len)
            } else if let Some(v) = arg.downcast_ref::<usize>() {
                writepoint(*v as *const std::ffi::c_void, len)
            } else {
                -1
            }
        }
        'x' | 'X' => {
            if let Some(v) = arg.downcast_ref::<u32>() {
                writehex(*v as u64, c, len)
            } else if let Some(v) = arg.downcast_ref::<u64>() {
                writehex(*v, c, len)
            } else if let Some(v) = arg.downcast_ref::<i32>() {
                writehex(*v as u32 as u64, c, len)
            } else {
                -1
            }
        }
        _ => -1,
    };
    result
}

pub fn writechar(c: char, len: &mut i32) -> i32 {
    *len += 1;
    let mut buf = [0u8; 4];
    let s = c.encode_utf8(&mut buf);
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    match handle.write(s.as_bytes()) {
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
    let mut nb = n as usize as u64;
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
    let mut arr: [u8; 10] = [0; 10];
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
    // Mirror the C behavior for NULL by treating an empty `&str`-style
    // sentinel separately. In Rust, `&str` cannot be null, so an empty
    // string is taken as the closest analogue of the NULL case.
    let to_write: &str = if s.is_empty() { "(null)" } else { s };
    for ch in to_write.chars() {
        if writechar(ch, len) == -1 {
            return -1;
        }
    }
    1
}

pub fn writehex(n: u64, c: char, len: &mut i32) -> i32 {
    let mut arr: [u8; 16] = [0; 16];
    let mut i: usize = 0;
    if n == 0 {
        arr[i] = b'0';
        i += 1;
    }
    let hex = if c == 'x' { HEXALOW } else { HEXAUP };
    let hex_bytes = hex.as_bytes();
    let mut nb = n;
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
