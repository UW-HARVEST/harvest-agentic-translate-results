use std::any::Any;
use std::cell::Cell;
use std::io::Write;

// Constants
pub const HEXALOW: &str = "0123456789abcdef";
pub const DECIMAL: &str = "0123456789";
pub const HEXAUP: &str = "0123456789ABCDEF";
pub const LOCATION: i32 = 2;

thread_local! {
    static ARG_INDEX: Cell<usize> = Cell::new(0);
}

pub fn reset_arg_index() {
    ARG_INDEX.with(|idx| idx.set(0));
}

fn next_arg<'a>(args: &'a [Box<dyn Any>]) -> Option<&'a Box<dyn Any>> {
    ARG_INDEX.with(|idx| {
        let i = idx.get();
        if i < args.len() {
            idx.set(i + 1);
            Some(&args[i])
        } else {
            None
        }
    })
}

// Function Declarations
pub fn writeuint(n: u64, len: &mut i32) -> i32 {
    if n == 0 {
        return writechar('0', len);
    }
    let mut arr = [0u8; 20];
    let mut i = 0usize;
    let mut n = n;
    while n > 0 {
        arr[i] = DECIMAL.as_bytes()[(n % 10) as usize];
        n /= 10;
        i += 1;
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
    match c {
        'c' => {
            if let Some(arg) = next_arg(args) {
                if let Some(&v) = arg.downcast_ref::<i32>() {
                    writechar(v as u8 as char, len)
                } else if let Some(&v) = arg.downcast_ref::<char>() {
                    writechar(v, len)
                } else if let Some(&v) = arg.downcast_ref::<u8>() {
                    writechar(v as char, len)
                } else {
                    -1
                }
            } else {
                -1
            }
        }
        's' => {
            if let Some(arg) = next_arg(args) {
                if let Some(v) = arg.downcast_ref::<&str>() {
                    writestring(v, len)
                } else if let Some(v) = arg.downcast_ref::<String>() {
                    writestring(v.as_str(), len)
                } else if let Some(v) = arg.downcast_ref::<Option<&str>>() {
                    match v {
                        Some(s) => writestring(s, len),
                        None => writestring("(null)", len),
                    }
                } else {
                    -1
                }
            } else {
                -1
            }
        }
        'd' | 'i' => {
            if let Some(arg) = next_arg(args) {
                if let Some(&v) = arg.downcast_ref::<i32>() {
                    writeint(v, len)
                } else {
                    -1
                }
            } else {
                -1
            }
        }
        'u' => {
            if let Some(arg) = next_arg(args) {
                if let Some(&v) = arg.downcast_ref::<u32>() {
                    writeuint(v as u64, len)
                } else if let Some(&v) = arg.downcast_ref::<u64>() {
                    writeuint(v, len)
                } else {
                    -1
                }
            } else {
                -1
            }
        }
        'p' => {
            if let Some(arg) = next_arg(args) {
                if let Some(&v) = arg.downcast_ref::<*const std::ffi::c_void>() {
                    writepoint(v, len)
                } else if let Some(&v) = arg.downcast_ref::<usize>() {
                    writepoint(v as *const std::ffi::c_void, len)
                } else {
                    -1
                }
            } else {
                -1
            }
        }
        'x' | 'X' => {
            if let Some(arg) = next_arg(args) {
                if let Some(&v) = arg.downcast_ref::<u32>() {
                    writehex(v as u64, c, len)
                } else if let Some(&v) = arg.downcast_ref::<u64>() {
                    writehex(v, c, len)
                } else {
                    -1
                }
            } else {
                -1
            }
        }
        '%' => writechar('%', len),
        _ => -1,
    }
}

pub fn writechar(c: char, len: &mut i32) -> i32 {
    let buf = [c as u8];
    match std::io::stdout().write_all(&buf) {
        Ok(_) => {
            *len += 1;
            1
        }
        Err(_) => -1,
    }
}

pub fn writepoint(n: *const std::ffi::c_void, len: &mut i32) -> i32 {
    if LOCATION == 2 && n.is_null() {
        return writestring("(nil)", len);
    }
    if writestring("0x", len) == -1 {
        return -1;
    }
    let nb = n as u64;
    if nb == 0 {
        return writechar('0', len);
    }
    let mut arr = [0u8; 32];
    let mut i = 0usize;
    let mut nb = nb;
    while nb > 0 {
        arr[i] = HEXALOW.as_bytes()[(nb % 16) as usize];
        nb /= 16;
        i += 1;
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
    if n == -2147483648 {
        return writestring("-2147483648", len);
    }
    let mut n = n;
    if n < 0 {
        if writechar('-', len) == -1 {
            return -1;
        }
        n = -n;
    }
    let mut arr = [0u8; 10];
    let mut i = 0usize;
    while n > 0 {
        arr[i] = DECIMAL.as_bytes()[(n % 10) as usize];
        n /= 10;
        i += 1;
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
    let s = if s.is_empty() { s } else { s };
    for c in s.bytes() {
        if writechar(c as char, len) == -1 {
            return -1;
        }
    }
    1
}

pub fn writehex(n: u64, c: char, len: &mut i32) -> i32 {
    let hex = if c == 'x' { HEXALOW } else { HEXAUP };
    let mut arr = [0u8; 16];
    let mut i = 0usize;
    if n == 0 {
        arr[i] = b'0';
        i += 1;
    }
    let mut n = n;
    while n > 0 {
        arr[i] = hex.as_bytes()[(n % 16) as usize];
        n /= 16;
        i += 1;
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
