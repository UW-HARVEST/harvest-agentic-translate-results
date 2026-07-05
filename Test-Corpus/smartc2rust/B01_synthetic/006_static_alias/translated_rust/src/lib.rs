
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::os::raw::{c_char, c_int};

use std::sync::Mutex;
use std::ffi::CStr;

/// The inner static state that was `static int inner = 1;` inside `static_alias`.
/// Wrapped in a `Mutex` for safe interior mutability across the FFI boundary.
static INNER: Mutex<i32> = Mutex::new(1);

/// Result of the `static_alias` operation.
///
/// In the original C code, the function returned either a pointer to the
/// static `inner` variable or a pointer to the caller's `outer` variable.
/// We model this distinction as an enum so no raw pointers/aliasing is needed.
enum AliasResult {
    /// The static inner value was updated; caller should adopt this value.
    Inner(i32),
    /// The caller's value was updated in place; this is the new value.
    Outer(i32),
}

fn static_alias(outer: i32) -> AliasResult {
    let mut inner = INNER.lock().expect("INNER mutex poisoned");
    if outer >= *inner {
        *inner = inner.wrapping_add(outer);
        AliasResult::Inner(*inner)
    } else {
        AliasResult::Outer(outer.wrapping_add(*inner))
    }
}

/// Parse an integer from a string using semantics comparable to C's `strtol`
/// with base 10: skip leading ASCII whitespace, optional sign, then digits.
/// Returns `Some(value)` if at least one digit was parsed, otherwise `None`.
fn parse_int_c_style(s: &str) -> Option<i64> {
    let bytes = s.as_bytes();
    let mut idx = 0;

    while idx < bytes.len() && bytes[idx].is_ascii_whitespace() {
        idx += 1;
    }

    let mut neg = false;
    if idx < bytes.len() && (bytes[idx] == b'+' || bytes[idx] == b'-') {
        neg = bytes[idx] == b'-';
        idx += 1;
    }

    let digits_start = idx;
    let mut val: i64 = 0;
    while idx < bytes.len() && bytes[idx].is_ascii_digit() {
        val = val.saturating_mul(10).saturating_add((bytes[idx] - b'0') as i64);
        idx += 1;
    }

    if idx == digits_start {
        return None;
    }

    Some(if neg { -val } else { val })
}

/// Collect C-style `argv` into a `Vec<String>` in a single, isolated `unsafe`
/// block at the FFI boundary. After this, all further logic is in safe Rust.
fn collect_args(argc: c_int, argv: *mut *mut c_char) -> Vec<String> {
    if argv.is_null() || argc <= 0 {
        return Vec::new();
    }
    let mut args = Vec::with_capacity(argc as usize);
    for i in 0..argc as isize {
        // SAFETY: The C caller guarantees `argv` points to at least `argc`
        // valid, NUL-terminated C strings (or null pointers). This is the
        // standard FFI contract for `main(argc, argv)`.
        let s = unsafe {
            let ptr = *argv.offset(i);
            if ptr.is_null() {
                String::new()
            } else {
                CStr::from_ptr(ptr).to_string_lossy().into_owned()
            }
        };
        args.push(s);
    }
    args
}

fn run(args: &[String]) -> c_int {
    if args.len() != 3 {
        println!("Error: should only be two (integer) arguments!");
        return 1;
    }

    let initial_value: i32 = match parse_int_c_style(&args[1]) {
        Some(v) => v as i32,
        None => {
            println!("Error: first argument must be an integer!");
            return 1;
        }
    };

    let iterations: i32 = match parse_int_c_style(&args[2]) {
        Some(v) => v as i32,
        None => {
            println!("Error: second argument must be an integer!");
            return 1;
        }
    };

    let mut running_sum: i32 = initial_value;
    for _ in 0..iterations {
        running_sum = match static_alias(running_sum) {
            AliasResult::Inner(v) | AliasResult::Outer(v) => v,
        };
        println!("{}", running_sum);
    }

    0
}

#[unsafe(no_mangle)]
pub extern "C" fn main_main(argc: c_int, argv: *mut *mut c_char) -> c_int {
    // Reset the static state so repeated invocations from tests behave
    // like a fresh program run (matching the C program's fresh-process
    // semantics for the `static int inner = 1;` initialization).
    if let Ok(mut inner) = INNER.lock() {
        *inner = 1;
    }

    let args = collect_args(argc, argv);
    run(&args)
}