use std::any::Any;
use std::cell::RefCell;
use std::io::{self, Write};
// Constants
pub const HEXALOW: &str = "0123456789abcdef";
pub const DECIMAL: &str = "0123456789";
pub const HEXAUP: &str = "0123456789ABCDEF";
pub const LOCATION: i32 = 2;

// Thread-local state used by the `format` function to track which argument
// from the slice supplied by the `ft_printf!` macro is currently being
// consumed. Because the macro hands the entire `&[Box<dyn Any>]` slice to
// `format` on every call, we need an external counter to walk through the
// arguments in order. The state stores `(slice_pointer, index)` so that we
// can detect when a new `ft_printf!` invocation begins (different slice
// pointer) and reset the index.
thread_local! {
    static ARG_STATE: RefCell<(usize, usize)> = RefCell::new((0, 0));
}

fn next_arg_index(args: &[Box<dyn Any>]) -> usize {
    ARG_STATE.with(|s| {
        let mut state = s.borrow_mut();
        let ptr = args.as_ptr() as usize;
        // Reset the counter if we appear to be in a fresh invocation, or if
        // we have already consumed every argument.
        if state.0 != ptr || state.1 >= args.len() {
            state.0 = ptr;
            state.1 = 0;
        }
        let idx = state.1;
        state.1 += 1;
        idx
    })
}

// Function Declarations
pub fn writeuint(n: u64, len: &mut i32) -> i32 {
    if n == 0 {
        return writechar('0', len);
    }
    let dec = DECIMAL.as_bytes();
    let mut arr = [0u8; 32];
    let mut i: usize = 0;
    let mut n = n;
    while n != 0 {
        arr[i] = dec[(n % 10) as usize];
        i += 1;
        n /= 10;
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
    // '%' does not consume an argument.
    if c == '%' {
        return writechar('%', len);
    }

    let idx = next_arg_index(args);
    if idx >= args.len() {
        return -1;
    }
    let arg: &dyn Any = args[idx].as_ref();

    if c == 'c' {
        if let Some(ch) = arg.downcast_ref::<char>() {
            return writechar(*ch, len);
        }
        if let Some(b) = arg.downcast_ref::<u8>() {
            return writechar(*b as char, len);
        }
        if let Some(i) = arg.downcast_ref::<i32>() {
            return writechar((*i as u8) as char, len);
        }
        if let Some(u) = arg.downcast_ref::<u32>() {
            return writechar((*u as u8) as char, len);
        }
        return -1;
    }
    if c == 's' {
        if let Some(s) = arg.downcast_ref::<&str>() {
            return writestring(s, len);
        }
        if let Some(s) = arg.downcast_ref::<String>() {
            return writestring(s.as_str(), len);
        }
        return -1;
    }
    if c == 'd' || c == 'i' {
        if let Some(n) = arg.downcast_ref::<i32>() {
            return writeint(*n, len);
        }
        if let Some(n) = arg.downcast_ref::<i64>() {
            return writeint(*n as i32, len);
        }
        if let Some(n) = arg.downcast_ref::<isize>() {
            return writeint(*n as i32, len);
        }
        return -1;
    }
    if c == 'u' {
        if let Some(n) = arg.downcast_ref::<u32>() {
            return writeuint(*n as u64, len);
        }
        if let Some(n) = arg.downcast_ref::<u64>() {
            return writeuint(*n, len);
        }
        if let Some(n) = arg.downcast_ref::<usize>() {
            return writeuint(*n as u64, len);
        }
        if let Some(n) = arg.downcast_ref::<i32>() {
            return writeuint(*n as u32 as u64, len);
        }
        return -1;
    }
    if c == 'p' {
        if let Some(p) = arg.downcast_ref::<*const std::ffi::c_void>() {
            return writepoint(*p, len);
        }
        if let Some(p) = arg.downcast_ref::<*mut std::ffi::c_void>() {
            return writepoint(*p as *const std::ffi::c_void, len);
        }
        if let Some(addr) = arg.downcast_ref::<usize>() {
            return writepoint(*addr as *const std::ffi::c_void, len);
        }
        return -1;
    }
    if c == 'x' || c == 'X' {
        if let Some(n) = arg.downcast_ref::<u32>() {
            return writehex(*n as u64, c, len);
        }
        if let Some(n) = arg.downcast_ref::<u64>() {
            return writehex(*n, c, len);
        }
        if let Some(n) = arg.downcast_ref::<usize>() {
            return writehex(*n as u64, c, len);
        }
        if let Some(n) = arg.downcast_ref::<i32>() {
            return writehex(*n as u32 as u64, c, len);
        }
        return -1;
    }
    -1
}

pub fn writechar(c: char, len: &mut i32) -> i32 {
    *len += 1;
    // The C version performs `write(1, &c, 1)` which writes a single byte and
    // returns the number of bytes written (1) on success or -1 on failure.
    // To preserve that semantic we write the low byte of the char to stdout.
    let byte = [c as u32 as u8];
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    match handle.write(&byte) {
        Ok(n) => n as i32,
        Err(_) => -1,
    }
}

pub fn writepoint(n: *const std::ffi::c_void, len: &mut i32) -> i32 {
    if LOCATION == 2 && n.is_null() {
        return writestring("(nil)", len);
    }
    let mut nb = n as usize;
    if writestring("0x", len) == -1 {
        return -1;
    }
    if nb == 0 {
        return writechar('0', len);
    }
    let hex = HEXALOW.as_bytes();
    let mut arr = [0u8; 32];
    let mut i: usize = 0;
    while nb != 0 {
        arr[i] = hex[nb % 16];
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
    let mut n = n;
    if n < 0 {
        if writechar('-', len) == -1 {
            return -1;
        }
        n = -n;
    }
    let dec = DECIMAL.as_bytes();
    let mut arr = [0u8; 10];
    let mut i: usize = 0;
    while n != 0 {
        arr[i] = dec[(n % 10) as usize];
        i += 1;
        n /= 10;
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
    // The original C function checks for a NULL pointer and substitutes
    // "(null)". In Rust, `&str` cannot be null, so the substitution is
    // unnecessary; the type system enforces a valid string slice.
    for b in s.bytes() {
        if writechar(b as char, len) == -1 {
            return -1;
        }
    }
    1
}

pub fn writehex(n: u64, c: char, len: &mut i32) -> i32 {
    let hex = if c == 'x' {
        HEXALOW.as_bytes()
    } else {
        HEXAUP.as_bytes()
    };
    let mut arr = [0u8; 32];
    let mut i: usize = 0;
    let mut n = n;
    if n == 0 {
        arr[i] = b'0';
        i += 1;
    }
    while n != 0 {
        arr[i] = hex[(n % 16) as usize];
        i += 1;
        n /= 16;
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
