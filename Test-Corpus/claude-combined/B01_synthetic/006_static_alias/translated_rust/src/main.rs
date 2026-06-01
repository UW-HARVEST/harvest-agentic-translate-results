// Translation of c_src/src/main.c into Rust.
//
// The original C program leverages aliasing between a caller-supplied pointer
// and a `static` local inside `static_alias` to maintain a running sum across
// calls. We faithfully reproduce that behavior here.

use std::env;
use std::process::ExitCode;
use std::ptr::addr_of_mut;

static mut INNER: i32 = 1;

/// Faithful translation of:
///
/// ```c
/// int *static_alias(int *outer) {
///   static int inner = 1;
///   if(*outer >= inner) {
///     inner += *outer;
///     return &inner;
///   } else {
///     *outer += inner;
///     return outer;
///   }
/// }
/// ```
unsafe fn static_alias(outer: *mut i32) -> *mut i32 {
    // Safety: caller guarantees `outer` points to a valid `i32`. When `outer`
    // aliases `&INNER`, both the read of `*outer` and the read/write of
    // `INNER` go through the same memory location, exactly matching the C
    // behavior.
    let outer_val = unsafe { *outer };
    let inner_val = unsafe { *addr_of_mut!(INNER) };
    if outer_val >= inner_val {
        // inner += *outer;
        // When outer == &INNER, this is equivalent to inner += inner.
        unsafe {
            *addr_of_mut!(INNER) = inner_val + outer_val;
        }
        addr_of_mut!(INNER)
    } else {
        // *outer += inner;
        unsafe {
            *outer = outer_val + inner_val;
        }
        outer
    }
}

/// Mimics C's `strtol(s, &end, 10)` on the success/failure axis we need:
/// returns `(parsed_value_truncated_to_i32, end_equals_start)`.
///
/// Following POSIX `strtol`, leading whitespace is skipped, an optional `+`/`-`
/// sign is consumed, then base-10 digits are read. If no digits were parsed,
/// the C code observes `end == argv[i]`, which we signal by returning `true`
/// in the second tuple element.
fn strtol_like(s: &str) -> (i32, bool) {
    let bytes = s.as_bytes();
    let mut i = 0usize;

    // Skip whitespace (matches C `isspace` for the default "C" locale).
    while i < bytes.len() {
        let c = bytes[i];
        if c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' || c == 0x0b || c == 0x0c {
            i += 1;
        } else {
            break;
        }
    }

    let mut negative = false;
    if i < bytes.len() {
        if bytes[i] == b'+' {
            i += 1;
        } else if bytes[i] == b'-' {
            negative = true;
            i += 1;
        }
    }

    let digits_start = i;
    // Use i64 so we can mirror typical 64-bit `long` accumulation before
    // truncation to `int` (i32) per the C source's implicit narrowing
    // assignment.
    let mut value: i64 = 0;
    let mut overflow = false;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        let d = (bytes[i] - b'0') as i64;
        match value.checked_mul(10).and_then(|v| {
            if negative {
                v.checked_sub(d)
            } else {
                v.checked_add(d)
            }
        }) {
            Some(v) => value = v,
            None => {
                overflow = true;
                value = if negative { i64::MIN } else { i64::MAX };
            }
        }
        i += 1;
    }

    if i == digits_start {
        // No digits were consumed -> in C `end` is set to the start of the
        // string, so `end == argv[i]` holds true. Signal "no digits parsed".
        return (0, true);
    }

    // If overflow occurred, keep the saturated i64 value; truncation to i32
    // below mirrors what assigning a 64-bit `long` to an `int` does in C.
    let _ = overflow;
    (value as i32, false)
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    let argc = args.len();

    if argc != 3 {
        println!("Error: should only be two (integer) arguments!");
        return ExitCode::from(1);
    }

    let (initial_value_parsed, no_digits_first) = strtol_like(&args[1]);
    if no_digits_first {
        println!("Error: first argument must be an integer!");
        return ExitCode::from(1);
    }

    let (iterations, no_digits_second) = strtol_like(&args[2]);
    if no_digits_second {
        println!("Error: second argument must be an integer!");
        return ExitCode::from(1);
    }

    let mut initial_value: i32 = initial_value_parsed;
    let mut running_sum: *mut i32 = &mut initial_value as *mut i32;

    // Safety: `running_sum` always points to either `initial_value` (a live
    // local) or `INNER` (a `static mut`). Both remain valid for the duration
    // of the loop, and we only access them through the single `running_sum`
    // pointer, mirroring the C program's pointer usage.
    unsafe {
        let mut i: i32 = 0;
        while i < iterations {
            running_sum = static_alias(running_sum);
            println!("{}", *running_sum);
            i += 1;
        }
    }

    ExitCode::from(0)
}
