use std::any::Any;
use std::cell::RefCell;
use std::io::{self, Write};
// Constants
pub const HEXALOW: &str = "0123456789abcdef";
pub const DECIMAL: &str = "0123456789";
pub const HEXAUP: &str = "0123456789ABCDEF";
pub const LOCATION: i32 = 2;

thread_local! {
    static FORMAT_STATE: RefCell<(usize, usize, usize)> = const { RefCell::new((0, 0, 0)) };
}

fn write_bytes(bytes: &[u8], len: &mut i32) -> i32 {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    for &byte in bytes {
        *len += 1;
        if handle.write_all(&[byte]).is_err() {
            return -1;
        }
    }
    1
}

fn next_arg<'a>(args: &'a [Box<dyn Any>]) -> Option<&'a Box<dyn Any>> {
    let key = (args.as_ptr() as usize, args.len());
    FORMAT_STATE.with(|state| {
        let mut state = state.borrow_mut();
        if state.0 != key.0 || state.1 != key.1 {
            *state = (key.0, key.1, 0);
        }

        let index = state.2;
        let arg = args.get(index);
        if arg.is_some() {
            state.2 += 1;
        }
        arg
    })
}

fn as_char(arg: &Box<dyn Any>) -> Option<char> {
    arg.downcast_ref::<char>().copied().or_else(|| {
        arg.downcast_ref::<u8>().map(|c| char::from(*c)).or_else(|| {
            arg.downcast_ref::<i8>()
                .and_then(|c| u8::try_from(*c).ok())
                .map(char::from)
        })
    })
}

fn as_i32(arg: &Box<dyn Any>) -> Option<i32> {
    arg.downcast_ref::<i32>()
        .copied()
        .or_else(|| arg.downcast_ref::<i16>().map(|n| i32::from(*n)))
        .or_else(|| arg.downcast_ref::<i8>().map(|n| i32::from(*n)))
        .or_else(|| arg.downcast_ref::<u8>().map(|n| i32::from(*n)))
        .or_else(|| arg.downcast_ref::<u16>().map(|n| i32::from(*n)))
        .or_else(|| arg.downcast_ref::<u32>().and_then(|n| i32::try_from(*n).ok()))
        .or_else(|| arg.downcast_ref::<i64>().and_then(|n| i32::try_from(*n).ok()))
        .or_else(|| arg.downcast_ref::<u64>().and_then(|n| i32::try_from(*n).ok()))
        .or_else(|| arg.downcast_ref::<isize>().and_then(|n| i32::try_from(*n).ok()))
        .or_else(|| arg.downcast_ref::<usize>().and_then(|n| i32::try_from(*n).ok()))
}

fn as_u64(arg: &Box<dyn Any>) -> Option<u64> {
    arg.downcast_ref::<u64>()
        .copied()
        .or_else(|| arg.downcast_ref::<u32>().map(|n| u64::from(*n)))
        .or_else(|| arg.downcast_ref::<u16>().map(|n| u64::from(*n)))
        .or_else(|| arg.downcast_ref::<u8>().map(|n| u64::from(*n)))
        .or_else(|| arg.downcast_ref::<usize>().and_then(|n| u64::try_from(*n).ok()))
        .or_else(|| arg.downcast_ref::<i32>().and_then(|n| u64::try_from(*n).ok()))
        .or_else(|| arg.downcast_ref::<i16>().and_then(|n| u64::try_from(*n).ok()))
        .or_else(|| arg.downcast_ref::<i8>().and_then(|n| u64::try_from(*n).ok()))
        .or_else(|| arg.downcast_ref::<i64>().and_then(|n| u64::try_from(*n).ok()))
        .or_else(|| arg.downcast_ref::<isize>().and_then(|n| u64::try_from(*n).ok()))
}

fn as_str(arg: &Box<dyn Any>) -> Option<&str> {
    arg.downcast_ref::<&str>()
        .copied()
        .or_else(|| arg.downcast_ref::<String>().map(String::as_str))
}

fn as_ptr(arg: &Box<dyn Any>) -> Option<*const std::ffi::c_void> {
    arg.downcast_ref::<*const std::ffi::c_void>()
        .copied()
        .or_else(|| {
            arg.downcast_ref::<*mut std::ffi::c_void>()
                .map(|ptr| *ptr as *const std::ffi::c_void)
        })
        .or_else(|| {
            arg.downcast_ref::<usize>()
                .map(|ptr| *ptr as *const std::ffi::c_void)
        })
}
// Function Declarations
pub fn writeuint(n: u64, len: &mut i32) -> i32 {
    if n == 0 {
        return writechar('0', len);
    }

    let mut value = n;
    let mut buf = [0u8; 20];
    let mut i = 0usize;
    let decimal = DECIMAL.as_bytes();

    while value != 0 {
        buf[i] = decimal[(value % 10) as usize];
        value /= 10;
        i += 1;
    }

    while i > 0 {
        i -= 1;
        if writechar(char::from(buf[i]), len) == -1 {
            return -1;
        }
    }

    1
}
pub fn format(args:&[Box<dyn Any>], c: char, len: &mut i32) -> i32 {
    match c {
        'c' => next_arg(args)
            .and_then(as_char)
            .map_or(-1, |value| writechar(value, len)),
        's' => next_arg(args)
            .and_then(as_str)
            .map_or(-1, |value| writestring(value, len)),
        'd' | 'i' => next_arg(args)
            .and_then(as_i32)
            .map_or(-1, |value| writeint(value, len)),
        'u' => next_arg(args)
            .and_then(as_u64)
            .map_or(-1, |value| writeuint(value, len)),
        'p' => next_arg(args)
            .and_then(as_ptr)
            .map_or(-1, |value| writepoint(value, len)),
        'x' | 'X' => next_arg(args)
            .and_then(as_u64)
            .map_or(-1, |value| writehex(value, c, len)),
        '%' => writechar('%', len),
        _ => -1,
    }
}
pub fn writechar(c: char, len: &mut i32) -> i32 {
    let mut buf = [0u8; 4];
    let encoded = c.encode_utf8(&mut buf);
    write_bytes(encoded.as_bytes(), len)
}
pub fn writepoint(n: *const std::ffi::c_void, len: &mut i32) -> i32 {
    if LOCATION == 2 && n.is_null() {
        return writestring("(nil)", len);
    }

    if writestring("0x", len) == -1 {
        return -1;
    }

    let mut value = n as usize as u64;
    if value == 0 {
        return writechar('0', len);
    }

    let mut buf = [0u8; 16];
    let mut i = 0usize;
    let hex = HEXALOW.as_bytes();

    while value != 0 {
        buf[i] = hex[(value % 16) as usize];
        value /= 16;
        i += 1;
    }

    while i > 0 {
        i -= 1;
        if writechar(char::from(buf[i]), len) == -1 {
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

    let mut buf = [0u8; 10];
    let mut i = 0usize;
    let decimal = DECIMAL.as_bytes();

    while value != 0 {
        buf[i] = decimal[(value % 10) as usize];
        value /= 10;
        i += 1;
    }

    while i > 0 {
        i -= 1;
        if writechar(char::from(buf[i]), len) == -1 {
            return -1;
        }
    }

    1
}
pub fn writestring(s: &str, len: &mut i32) -> i32 {
    write_bytes(s.as_bytes(), len)
}
pub fn writehex(n: u64, c: char, len: &mut i32) -> i32 {
    let mut buf = [0u8; 16];
    let mut i = 0usize;
    let mut value = n;
    let hex = if c == 'x' {
        HEXALOW.as_bytes()
    } else {
        HEXAUP.as_bytes()
    };

    if value == 0 {
        buf[i] = b'0';
        i += 1;
    }

    while value != 0 {
        buf[i] = hex[(value % 16) as usize];
        value /= 16;
        i += 1;
    }

    while i > 0 {
        i -= 1;
        if writechar(char::from(buf[i]), len) == -1 {
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
