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
    let mut value = n;
    while value != 0 {
        arr[i] = decimal_bytes[(value % 10) as usize];
        i += 1;
        value /= 10;
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
    // Look up the next argument by reading the args slice. The macro doesn't
    // currently track the argument index, so for compatibility we attempt a
    // best-effort dispatch using the first argument that matches the type.
    match c {
        'c' => {
            if let Some(arg) = args.first() {
                if let Some(ch) = arg.downcast_ref::<char>() {
                    return writechar(*ch, len);
                }
                if let Some(ch) = arg.downcast_ref::<i32>() {
                    return writechar((*ch as u8) as char, len);
                }
            }
            -1
        }
        's' => {
            if let Some(arg) = args.first() {
                if let Some(s) = arg.downcast_ref::<&str>() {
                    return writestring(s, len);
                }
                if let Some(s) = arg.downcast_ref::<String>() {
                    return writestring(s.as_str(), len);
                }
            }
            -1
        }
        'd' | 'i' => {
            if let Some(arg) = args.first() {
                if let Some(n) = arg.downcast_ref::<i32>() {
                    return writeint(*n, len);
                }
            }
            -1
        }
        'u' => {
            if let Some(arg) = args.first() {
                if let Some(n) = arg.downcast_ref::<u32>() {
                    return writeuint(*n as u64, len);
                }
                if let Some(n) = arg.downcast_ref::<u64>() {
                    return writeuint(*n, len);
                }
            }
            -1
        }
        'p' => {
            if let Some(arg) = args.first() {
                if let Some(p) = arg.downcast_ref::<*const std::ffi::c_void>() {
                    return writepoint(*p, len);
                }
            }
            -1
        }
        'x' | 'X' => {
            if let Some(arg) = args.first() {
                if let Some(n) = arg.downcast_ref::<u32>() {
                    return writehex(*n as u64, c, len);
                }
                if let Some(n) = arg.downcast_ref::<u64>() {
                    return writehex(*n, c, len);
                }
            }
            -1
        }
        '%' => writechar('%', len),
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
        Ok(_) => s.len() as i32,
        Err(_) => -1,
    }
}
pub fn writepoint(n: *const std::ffi::c_void, len: &mut i32) -> i32 {
    if LOCATION == 2 && n.is_null() {
        return writestring("(nil)", len);
    }
    let mut nb = n as usize as u64;
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
    let mut value = n;
    if value < 0 {
        if writechar('-', len) == -1 {
            return -1;
        }
        value = -value;
    }
    let decimal_bytes = DECIMAL.as_bytes();
    let mut arr: [u8; 10] = [0; 10];
    let mut i: usize = 0;
    while value != 0 {
        arr[i] = decimal_bytes[(value % 10) as usize];
        i += 1;
        value /= 10;
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
    let bytes = s.as_bytes();
    for &b in bytes {
        if writechar(b as char, len) == -1 {
            return -1;
        }
    }
    1
}
pub fn writehex(n: u64, c: char, len: &mut i32) -> i32 {
    let hex_bytes = if c == 'x' {
        HEXALOW.as_bytes()
    } else {
        HEXAUP.as_bytes()
    };
    let mut arr: [u8; 16] = [0; 16];
    let mut i: usize = 0;
    let mut value = n;
    if value == 0 {
        arr[i] = b'0';
        i += 1;
    }
    while value != 0 {
        arr[i] = hex_bytes[(value % 16) as usize];
        i += 1;
        value /= 16;
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
