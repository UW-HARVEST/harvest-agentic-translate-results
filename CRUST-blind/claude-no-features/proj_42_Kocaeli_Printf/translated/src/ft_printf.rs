use std::any::Any;
use std::cell::RefCell;
use std::io::Write;

// Constants
pub const HEXALOW: &str = "0123456789abcdef";
pub const DECIMAL: &str = "0123456789";
pub const HEXAUP: &str = "0123456789ABCDEF";
pub const LOCATION: i32 = 2;

thread_local! {
    static ARG_INDEX: RefCell<usize> = const { RefCell::new(0) };
}

// Helper to get nth byte from a constant string of digits.
fn digit_at(s: &str, idx: usize) -> char {
    s.as_bytes()[idx] as char
}

// Function Declarations
pub fn writeuint(n: u64, len: &mut i32) -> i32 {
    let mut arr: [char; 32] = ['\0'; 32];
    let mut i: usize = 0;

    if n == 0 {
        return writechar('0', len);
    }
    let mut n = n;
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
    // Pull next argument from the args slice using a thread-local index.
    let idx = ARG_INDEX.with(|i| {
        let cur = *i.borrow();
        *i.borrow_mut() = cur + 1;
        cur
    });

    // Reset index if we've consumed all args (so subsequent calls of the macro
    // with fresh args still work). We do this by resetting whenever idx is out
    // of range below.
    let arg_opt: Option<&Box<dyn Any>> = if idx < args.len() {
        Some(&args[idx])
    } else {
        None
    };

    let result = match c {
        'c' => {
            if let Some(arg) = arg_opt {
                if let Some(ch) = arg.downcast_ref::<char>() {
                    writechar(*ch, len)
                } else if let Some(b) = arg.downcast_ref::<u8>() {
                    writechar(*b as char, len)
                } else if let Some(i) = arg.downcast_ref::<i32>() {
                    writechar((*i as u8) as char, len)
                } else {
                    -1
                }
            } else {
                -1
            }
        }
        's' => {
            if let Some(arg) = arg_opt {
                if let Some(s) = arg.downcast_ref::<&str>() {
                    writestring(s, len)
                } else if let Some(s) = arg.downcast_ref::<String>() {
                    writestring(s.as_str(), len)
                } else {
                    -1
                }
            } else {
                -1
            }
        }
        'd' | 'i' => {
            if let Some(arg) = arg_opt {
                if let Some(n) = arg.downcast_ref::<i32>() {
                    writeint(*n, len)
                } else if let Some(n) = arg.downcast_ref::<i64>() {
                    writeint(*n as i32, len)
                } else if let Some(n) = arg.downcast_ref::<u32>() {
                    writeint(*n as i32, len)
                } else {
                    -1
                }
            } else {
                -1
            }
        }
        'u' => {
            if let Some(arg) = arg_opt {
                if let Some(n) = arg.downcast_ref::<u32>() {
                    writeuint(*n as u64, len)
                } else if let Some(n) = arg.downcast_ref::<u64>() {
                    writeuint(*n, len)
                } else if let Some(n) = arg.downcast_ref::<i32>() {
                    writeuint(*n as u32 as u64, len)
                } else {
                    -1
                }
            } else {
                -1
            }
        }
        'p' => {
            if let Some(arg) = arg_opt {
                if let Some(p) = arg.downcast_ref::<*const std::ffi::c_void>() {
                    writepoint(*p, len)
                } else if let Some(p) = arg.downcast_ref::<*mut std::ffi::c_void>() {
                    writepoint(*p as *const std::ffi::c_void, len)
                } else if let Some(n) = arg.downcast_ref::<usize>() {
                    writepoint(*n as *const std::ffi::c_void, len)
                } else {
                    -1
                }
            } else {
                -1
            }
        }
        'x' | 'X' => {
            if let Some(arg) = arg_opt {
                if let Some(n) = arg.downcast_ref::<u32>() {
                    writehex(*n as u64, c, len)
                } else if let Some(n) = arg.downcast_ref::<u64>() {
                    writehex(*n, c, len)
                } else if let Some(n) = arg.downcast_ref::<i32>() {
                    writehex(*n as u32 as u64, c, len)
                } else {
                    -1
                }
            } else {
                -1
            }
        }
        '%' => {
            // The literal '%' character does not consume an argument. Roll back
            // the index we just incremented above.
            ARG_INDEX.with(|i| {
                let cur = *i.borrow();
                if cur > 0 {
                    *i.borrow_mut() = cur - 1;
                }
            });
            writechar('%', len)
        }
        _ => -1,
    };

    // If we've consumed all args, reset the counter so the next ft_printf!
    // invocation starts at 0 again.
    ARG_INDEX.with(|i| {
        if *i.borrow() >= args.len() {
            *i.borrow_mut() = 0;
        }
    });

    result
}

pub fn writechar(c: char, len: &mut i32) -> i32 {
    *len += 1;
    let mut buf = [0u8; 4];
    let s = c.encode_utf8(&mut buf);
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    match handle.write_all(s.as_bytes()) {
        Ok(_) => s.len() as i32,
        Err(_) => -1,
    }
}

pub fn writepoint(n: *const std::ffi::c_void, len: &mut i32) -> i32 {
    let mut arr: [char; 32] = ['\0'; 32];
    let mut i: usize = 0;
    let mut nb: u64 = n as usize as u64;

    if LOCATION == 2 && n.is_null() {
        return writestring("(nil)", len);
    }
    if writestring("0x", len) == -1 {
        return -1;
    }
    if nb == 0 {
        return writechar('0', len);
    }
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
    let mut arr: [char; 16] = ['\0'; 16];
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
    // The Rust signature uses &str so the value cannot be null. Callers wishing
    // to mimic the C behavior of printing "(null)" for a NULL pointer should
    // pass the literal "(null)" string explicitly.
    for ch in s.chars() {
        if writechar(ch, len) == -1 {
            return -1;
        }
    }
    1
}

pub fn writehex(n: u64, c: char, len: &mut i32) -> i32 {
    let mut arr: [char; 32] = ['\0'; 32];
    let mut i: usize = 0;
    let hex = if c == 'x' { HEXALOW } else { HEXAUP };

    let mut n = n;
    if n == 0 {
        arr[i] = '0';
        i += 1;
    }
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
