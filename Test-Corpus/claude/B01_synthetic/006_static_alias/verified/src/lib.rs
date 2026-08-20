// Rust translation of c_src/src/main.c (StaticAlias driver).
//
// The C translation unit exports exactly two symbols, `static_alias` and
// `main`; both are reproduced here with the same C ABI signatures and the same
// observable behaviour:
//
//   * `static_alias()` keeps a function-local `static int inner = 1;`.  The
//     static lives for the whole lifetime of the loaded image, so repeated
//     calls (and repeated calls of `main`) accumulate state.  This is modelled
//     by the `INNER` mutable static below.
//   * the pointer aliasing between the caller's object and the function-local
//     static is reproduced with raw pointers, so passing the returned pointer
//     back in (`outer == &inner`) behaves exactly like C.
//   * `int` arithmetic wraps (what gcc/clang emit for `inner += *outer` at the
//     optimisation level used by c_src/CMakeLists.txt, which sets no
//     -O flags).
//   * the order of `main`'s argument-count / parse validation checks,
//   * C's `strtol` semantics (leading whitespace, optional sign, saturation at
//     LONG_MAX/LONG_MIN on overflow, "end == start" when nothing is parsed),
//   * the implicit `long` -> `int` narrowing conversion (two's-complement
//     truncation, as performed by gcc/clang),
//   * the exact printf output ("%d\n") and the returned exit statuses.

use std::ffi::{c_char, c_int, c_long, CStr};
use std::io::Write;

/// The function-local `static int inner = 1;` of C's `static_alias()`.
///
/// A function-local static in C has static storage duration, i.e. exactly the
/// lifetime of the loaded image, and is shared by every call.
static mut INNER: c_int = 1;

/// ```c
/// int*
/// static_alias(int *outer) {
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
///
/// # Safety
/// `outer` must be a valid, aligned, dereferenceable and writable pointer to an
/// `int`, exactly as required by the C function.
#[no_mangle]
pub unsafe extern "C" fn static_alias(outer: *mut c_int) -> *mut c_int {
    let inner: *mut c_int = std::ptr::addr_of_mut!(INNER);

    if *outer >= *inner {
        // `inner += *outer`; when `outer` aliases `inner` this doubles it.
        *inner = (*inner).wrapping_add(*outer);
        inner
    } else {
        *outer = (*outer).wrapping_add(*inner);
        outer
    }
}

/// True for the characters `isspace()` accepts in the "C" locale, which is the
/// locale in effect because the program never calls `setlocale()`.
fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// Faithful replica of `strtol(s, &end, 10)`.
///
/// Returns `(value, consumed)`, where `consumed` is how far `end` was advanced
/// past the start of `s`. `consumed == 0` corresponds to `end == s` in the C
/// code, i.e. no conversion could be performed.
pub fn c_strtol_base10(s: &[u8]) -> (c_long, usize) {
    let mut i = 0usize;

    // Skip leading white space.
    while i < s.len() && is_c_space(s[i]) {
        i += 1;
    }

    // Optional sign.
    let mut negative = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        negative = s[i] == b'-';
        i += 1;
    }

    // Digit sequence, accumulated as an exact magnitude so that the
    // LONG_MIN/LONG_MAX saturation boundaries match strtol precisely.
    let digits_start = i;
    let limit: u128 = if negative {
        u128::from(c_long::MIN.unsigned_abs())
    } else {
        c_long::MAX as u128
    };
    let mut acc: u128 = 0;
    let mut saturated = false;
    while i < s.len() && s[i].is_ascii_digit() {
        if !saturated {
            acc = acc * 10 + u128::from(s[i] - b'0');
            if acc > limit {
                saturated = true;
            }
        }
        i += 1;
    }

    if i == digits_start {
        // No conversion performed: strtol stores the original pointer in
        // *endptr and returns 0.
        return (0, 0);
    }

    if saturated {
        // ERANGE: strtol returns LONG_MAX / LONG_MIN.
        return (if negative { c_long::MIN } else { c_long::MAX }, i);
    }

    let value = if negative {
        (acc as c_long).wrapping_neg()
    } else {
        acc as c_long
    };
    (value, i)
}

/// ```c
/// int
/// main(int argc, char **argv) { ... }
/// ```
///
/// Maintain a sum leveraging multiple references to a static variable.
///
/// # Safety
/// `argv` must be a valid `argc`-element array of NUL-terminated C strings, as
/// required by the C function.
pub unsafe extern "C" fn c_main(argc: c_int, argv: *mut *mut c_char) -> c_int {
    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::new(stdout.lock());

    if argc != 3 {
        let _ = writeln!(out, "Error: should only be two (integer) arguments!");
        let _ = out.flush();
        return 1;
    }

    let arg1: *mut c_char = *argv.add(1);
    let (raw1, consumed1) = c_strtol_base10(CStr::from_ptr(arg1).to_bytes());
    // Implicit `long` -> `int` conversion of strtol()'s result.
    let mut initial_value: c_int = raw1 as c_int;
    if consumed1 == 0 {
        // end is set to start of string if nothing parsed
        let _ = writeln!(out, "Error: first argument must be an integer!");
        let _ = out.flush();
        return 1;
    }

    let arg2: *mut c_char = *argv.add(2);
    let (raw2, consumed2) = c_strtol_base10(CStr::from_ptr(arg2).to_bytes());
    let iterations: c_int = raw2 as c_int;
    if consumed2 == 0 {
        // end is set to start of string if nothing parsed
        let _ = writeln!(out, "Error: second argument must be an integer!");
        let _ = out.flush();
        return 1;
    }

    let mut running_sum: *mut c_int = &mut initial_value;
    let mut i: c_int = 0;
    while i < iterations {
        running_sum = static_alias(running_sum);
        let _ = writeln!(out, "{}", *running_sum);
        i = i.wrapping_add(1);
    }

    let _ = out.flush();
    0
}
