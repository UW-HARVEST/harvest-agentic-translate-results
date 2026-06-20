use std::any::Any;
use std::cell::RefCell;
use std::io::{self, Write};

// Constants
pub const HEXALOW: &str = "0123456789abcdef";
pub const DECIMAL: &str = "0123456789";
pub const HEXAUP: &str = "0123456789ABCDEF";
pub const LOCATION: i32 = 2;

thread_local! {
    static FORMAT_STATE: RefCell<FormatState> = RefCell::new(FormatState::default());
}

#[derive(Default)]
struct FormatState {
    len_ptr: usize,
    next_arg: usize,
}

fn decimal_digit(n: u64) -> char {
    DECIMAL.as_bytes()[n as usize] as char
}

fn hex_digit(n: u64, uppercase: bool) -> char {
    let table = if uppercase { HEXAUP } else { HEXALOW };
    table.as_bytes()[n as usize] as char
}

fn next_arg_index(len: &mut i32) -> usize {
    FORMAT_STATE.with(|state| {
        let mut state = state.borrow_mut();
        let len_ptr = (len as *mut i32) as usize;
        if state.len_ptr != len_ptr {
            state.len_ptr = len_ptr;
            state.next_arg = 0;
        }
        let index = state.next_arg;
        state.next_arg += 1;
        index
    })
}

// Function Declarations
pub fn writeuint(n: u64, len: &mut i32) -> i32 {
    let mut digits = ['\0'; 20];
    let mut value = n;
    let mut i = 0usize;

    if value == 0 {
        return writechar('0', len);
    }

    while value != 0 {
        digits[i] = decimal_digit(value % 10);
        value /= 10;
        i += 1;
    }

    while i > 0 {
        i -= 1;
        if writechar(digits[i], len) == -1 {
            return -1;
        }
    }

    1
}
pub fn format(args:&[Box<dyn Any>], c: char, len: &mut i32) -> i32 {
    match c {
        '%' => writechar('%', len),
        'c' => {
            let Some(arg) = args.get(next_arg_index(len)) else {
                return -1;
            };
            if let Some(value) = arg.downcast_ref::<char>() {
                writechar(*value, len)
            } else if let Some(value) = arg.downcast_ref::<u8>() {
                writechar(*value as char, len)
            } else if let Some(value) = arg.downcast_ref::<i8>() {
                writechar((*value as u8) as char, len)
            } else if let Some(value) = arg.downcast_ref::<i32>() {
                char::from_u32(*value as u32).map_or(-1, |ch| writechar(ch, len))
            } else if let Some(value) = arg.downcast_ref::<u32>() {
                char::from_u32(*value).map_or(-1, |ch| writechar(ch, len))
            } else {
                -1
            }
        }
        's' => {
            let Some(arg) = args.get(next_arg_index(len)) else {
                return -1;
            };
            if let Some(value) = arg.downcast_ref::<String>() {
                writestring(value, len)
            } else if let Some(value) = arg.downcast_ref::<&'static str>() {
                writestring(value, len)
            } else {
                -1
            }
        }
        'd' | 'i' => {
            let Some(arg) = args.get(next_arg_index(len)) else {
                return -1;
            };
            if let Some(value) = arg.downcast_ref::<i32>() {
                writeint(*value, len)
            } else if let Some(value) = arg.downcast_ref::<i16>() {
                writeint((*value).into(), len)
            } else if let Some(value) = arg.downcast_ref::<i8>() {
                writeint((*value).into(), len)
            } else if let Some(value) = arg.downcast_ref::<u8>() {
                writeint((*value).into(), len)
            } else if let Some(value) = arg.downcast_ref::<u16>() {
                writeint((*value).into(), len)
            } else {
                -1
            }
        }
        'u' => {
            let Some(arg) = args.get(next_arg_index(len)) else {
                return -1;
            };
            if let Some(value) = arg.downcast_ref::<u64>() {
                writeuint(*value, len)
            } else if let Some(value) = arg.downcast_ref::<u32>() {
                writeuint((*value).into(), len)
            } else if let Some(value) = arg.downcast_ref::<u16>() {
                writeuint((*value).into(), len)
            } else if let Some(value) = arg.downcast_ref::<u8>() {
                writeuint((*value).into(), len)
            } else {
                -1
            }
        }
        'p' => {
            let Some(arg) = args.get(next_arg_index(len)) else {
                return -1;
            };
            if let Some(value) = arg.downcast_ref::<*const std::ffi::c_void>() {
                writepoint(*value, len)
            } else if let Some(value) = arg.downcast_ref::<*mut std::ffi::c_void>() {
                writepoint((*value).cast_const(), len)
            } else if let Some(value) = arg.downcast_ref::<usize>() {
                writepoint(*value as *const std::ffi::c_void, len)
            } else {
                -1
            }
        }
        'x' => {
            let Some(arg) = args.get(next_arg_index(len)) else {
                return -1;
            };
            if let Some(value) = arg.downcast_ref::<u64>() {
                writehex(*value, 'x', len)
            } else if let Some(value) = arg.downcast_ref::<u32>() {
                writehex((*value).into(), 'x', len)
            } else if let Some(value) = arg.downcast_ref::<u16>() {
                writehex((*value).into(), 'x', len)
            } else if let Some(value) = arg.downcast_ref::<u8>() {
                writehex((*value).into(), 'x', len)
            } else {
                -1
            }
        }
        'X' => {
            let Some(arg) = args.get(next_arg_index(len)) else {
                return -1;
            };
            if let Some(value) = arg.downcast_ref::<u64>() {
                writehex(*value, 'X', len)
            } else if let Some(value) = arg.downcast_ref::<u32>() {
                writehex((*value).into(), 'X', len)
            } else if let Some(value) = arg.downcast_ref::<u16>() {
                writehex((*value).into(), 'X', len)
            } else if let Some(value) = arg.downcast_ref::<u8>() {
                writehex((*value).into(), 'X', len)
            } else {
                -1
            }
        }
        _ => -1,
    }
}
pub fn writechar(c: char, len: &mut i32) -> i32 {
    *len += 1;
    let mut stdout = io::stdout().lock();
    let mut buf = [0u8; 4];
    let encoded = c.encode_utf8(&mut buf);
    if stdout.write_all(encoded.as_bytes()).is_err() {
        return -1;
    }
    1
}
pub fn writepoint(n: *const std::ffi::c_void, len: &mut i32) -> i32 {
    if LOCATION == 2 && n.is_null() {
        return writestring("(nil)", len);
    }

    if writestring("0x", len) == -1 {
        return -1;
    }

    let mut digits = ['\0'; 16];
    let mut value = n as usize as u64;
    let mut i = 0usize;

    if value == 0 {
        return writechar('0', len);
    }

    while value != 0 {
        digits[i] = hex_digit(value % 16, false);
        value /= 16;
        i += 1;
    }

    while i > 0 {
        i -= 1;
        if writechar(digits[i], len) == -1 {
            return -1;
        }
    }

    1
}
pub fn writeint(n: i32, len: &mut i32) -> i32 {
    let mut digits = ['\0'; 10];
    let mut i = 0usize;

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

    while value != 0 {
        digits[i] = decimal_digit((value % 10) as u64);
        value /= 10;
        i += 1;
    }

    while i > 0 {
        i -= 1;
        if writechar(digits[i], len) == -1 {
            return -1;
        }
    }

    1
}
pub fn writestring(s: &str, len: &mut i32) -> i32 {
    for c in s.chars() {
        if writechar(c, len) == -1 {
            return -1;
        }
    }
    1
}
pub fn writehex(n: u64, c: char, len: &mut i32) -> i32 {
    let mut digits = ['\0'; 16];
    let mut value = n;
    let mut i = 0usize;
    let uppercase = c == 'X';

    if value == 0 {
        digits[i] = '0';
        i += 1;
    }

    while value != 0 {
        digits[i] = hex_digit(value % 16, uppercase);
        value /= 16;
        i += 1;
    }

    while i > 0 {
        i -= 1;
        if writechar(digits[i], len) == -1 {
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
