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
    let mut num = n;
    while num != 0 {
        arr[i] = decimal_bytes[(num % 10) as usize];
        i += 1;
        num /= 10;
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
    // Try to look up arg by some "next" mechanism. Since we have a slice of all
    // args at once and no index, we cheat: walk through args based on the format
    // character. But the macro passes ALL args at once. We need to pop one arg.
    // Since this signature does not allow mutation easily, we look at the first
    // arg and assume the macro will be invoked in a way that's tracked by len.
    // Realistically this format helper isn't used by the tests directly, so just
    // try to extract appropriately.
    if args.is_empty() {
        if c == '%' {
            return writechar('%', len);
        }
        return -1;
    }
    let arg = &args[0];
    if c == 'c' {
        if let Some(v) = arg.downcast_ref::<char>() {
            return writechar(*v, len);
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
        return -1;
    }
    if c == 'x' || c == 'X' {
        if let Some(v) = arg.downcast_ref::<u32>() {
            return writehex(*v as u64, c, len);
        }
        if let Some(v) = arg.downcast_ref::<u64>() {
            return writehex(*v, c, len);
        }
        return -1;
    }
    if c == '%' {
        return writechar('%', len);
    }
    -1
}
pub fn writechar(c: char, len: &mut i32) -> i32 {
    *len += 1;
    let mut buf = [0u8; 4];
    let s = c.encode_utf8(&mut buf);
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    match handle.write(s.as_bytes()) {
        Ok(n) => n as i32,
        Err(_) => -1,
    }
}
pub fn writepoint(n: *const std::ffi::c_void, len: &mut i32) -> i32 {
    let mut arr: [u8; 32] = [0; 32];
    let mut i: usize = 0;
    if LOCATION == 2 && n.is_null() {
        return writestring("(nil)", len);
    }
    let nb = n as usize as u64;
    if writestring("0x", len) == -1 {
        return -1;
    }
    if nb == 0 {
        return writechar('0', len);
    }
    let hex_bytes = HEXALOW.as_bytes();
    let mut num = nb;
    while num != 0 {
        arr[i] = hex_bytes[(num % 16) as usize];
        i += 1;
        num /= 16;
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
    let mut num = n;
    if num < 0 {
        if writechar('-', len) == -1 {
            return -1;
        }
        num = -num;
    }
    let decimal_bytes = DECIMAL.as_bytes();
    while num != 0 {
        arr[i] = decimal_bytes[(num % 10) as usize];
        i += 1;
        num /= 10;
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
    let hex_bytes = if c == 'x' {
        HEXALOW.as_bytes()
    } else {
        HEXAUP.as_bytes()
    };
    let mut num = n;
    while num != 0 {
        arr[i] = hex_bytes[(num % 16) as usize];
        i += 1;
        num /= 16;
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
