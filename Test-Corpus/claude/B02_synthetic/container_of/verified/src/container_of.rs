//! Faithful translation of `c_src/src/container_of.c`.
//!
//! The C translation unit is tiny, but every observable detail matters:
//!
//! ```c
//! #define offsetof(TYPE, MEMBER)  ((size_t) (&(((TYPE *)(0))->MEMBER)))
//!
//! #define container_of(ptr, type, member) ({         \
//!     (type *)( (char *)ptr - offsetof(type, member) );})
//!
//! struct test {
//!     int a;
//!     int b;
//! };
//!
//! struct test* find_container_of_a(int *i) { return container_of(i, struct test, a); }
//! struct test* find_container_of_b(int *i) { return container_of(i, struct test, b); }
//!
//! int main(int argc, char** argv) {
//!     int a = atoi(argv[1]);
//!     int b = atoi(argv[2]);
//!     struct test t;
//!     memset(&t, 0, sizeof(t));
//!     t.a = a;
//!     t.b = b;
//!     printf("%d\n", find_container_of_a(&t.a)->a + find_container_of_b(&t.b)->b);
//! }
//! ```
//!
//! Notable behaviours that are reproduced bit-for-bit:
//!
//! * `find_container_of_a` subtracts `offsetof(struct test, a) == 0`, so it is
//!   the identity on the pointer value. `find_container_of_b` subtracts
//!   `offsetof(struct test, b) == 4`. Neither performs any validation, so a
//!   `NULL` argument to `find_container_of_b` produces the wrapped-around
//!   pointer `(char *)0 - 4`.
//! * `argc` is never inspected: `argv[1]` and `argv[2]` are dereferenced
//!   unconditionally, in that order. With too few arguments they are `NULL`
//!   (or past the terminator) and glibc's `atoi` faults, killing the process
//!   with `SIGSEGV` before anything is printed.
//! * `atoi` is glibc's `(int) strtol(nptr, NULL, 10)`: leading whitespace,
//!   optional sign, decimal digits, saturation at `LONG_MAX` / `LONG_MIN`, then
//!   truncation to `int`.
//! * `main` falls off its closing brace, which in C99 and later means
//!   `return 0`.

use core::ffi::{c_char, c_int};

/// Mirrors `struct test { int a; int b; }`. `repr(C)` guarantees the same
/// layout, so `offset_of!(Test, b)` equals the C `offsetof(struct test, b)`.
#[repr(C)]
pub struct Test {
    pub a: c_int,
    pub b: c_int,
}

/// `offsetof(struct test, a)` -- zero.
pub const OFFSET_OF_A: usize = core::mem::offset_of!(Test, a);

/// `offsetof(struct test, b)` -- `sizeof(int)`, i.e. 4 on every target the C
/// code supports.
pub const OFFSET_OF_B: usize = core::mem::offset_of!(Test, b);

/// `struct test* find_container_of_a(int *i)`
///
/// `container_of(i, struct test, a)` == `(struct test *)((char *)i - 0)`.
///
/// The subtraction is performed on the integer representation of the pointer so
/// that it wraps exactly like the C pointer arithmetic does, without Rust
/// imposing any provenance or in-bounds requirement on the caller's pointer.
#[inline]
pub fn find_container_of_a(i: *mut c_int) -> *mut Test {
    (i as usize).wrapping_sub(OFFSET_OF_A) as *mut Test
}

/// `struct test* find_container_of_b(int *i)`
///
/// `container_of(i, struct test, b)` == `(struct test *)((char *)i - 4)`.
#[inline]
pub fn find_container_of_b(i: *mut c_int) -> *mut Test {
    (i as usize).wrapping_sub(OFFSET_OF_B) as *mut Test
}

/// True for the characters glibc's `isspace` accepts in the default "C"
/// locale, which is the locale in force because the program never calls
/// `setlocale`.
#[inline]
fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// glibc's `atoi`, which is literally `(int) strtol (nptr, NULL, 10)`.
///
/// * leading `isspace` characters are skipped,
/// * an optional `+` / `-` sign is consumed,
/// * decimal digits are accumulated; a value outside `long` saturates to
///   `LONG_MAX` / `LONG_MIN` (and sets `ERANGE`, which the caller ignores),
/// * a subject sequence that is empty converts to 0,
/// * the resulting `long` is truncated to `int`.
///
/// # Safety
///
/// `s` must be a NUL-terminated string, exactly as glibc requires. Passing
/// `NULL` reproduces the C program's crash: the first byte is read with a
/// volatile load, so the read is really performed and the process dies with
/// `SIGSEGV` just as glibc's `strtol` does.
pub unsafe fn atoi(s: *const c_char) -> c_int {
    // `long` on the LP64 targets this code is built for.
    let mut p = s as *const u8;

    // Volatile reads keep every byte access -- including the faulting one for a
    // NULL argument -- and keep them ordered with respect to the rest of the
    // program, mirroring the C call.
    let mut cur = core::ptr::read_volatile(p);

    while is_c_space(cur) {
        p = p.wrapping_add(1);
        cur = core::ptr::read_volatile(p);
    }

    let negative = if cur == b'+' || cur == b'-' {
        let neg = cur == b'-';
        p = p.wrapping_add(1);
        cur = core::ptr::read_volatile(p);
        neg
    } else {
        false
    };

    let mut acc: i64 = 0;
    let mut saturated = false;
    while cur.is_ascii_digit() {
        let digit = (cur - b'0') as i64;
        if !saturated {
            match acc.checked_mul(10).and_then(|v| v.checked_add(digit)) {
                Some(v) => acc = v,
                // Note: for the single value `-9223372036854775808` glibc does
                // *not* report a range error, but it still returns `LONG_MIN`,
                // which is what the saturating branch below produces, so the
                // returned value is identical either way.
                None => saturated = true,
            }
        }
        p = p.wrapping_add(1);
        cur = core::ptr::read_volatile(p);
    }

    let value: i64 = if saturated {
        if negative {
            i64::MIN
        } else {
            i64::MAX
        }
    } else if negative {
        // `acc >= 0` here, so the negation cannot overflow.
        -acc
    } else {
        acc
    };

    // `(int)` truncation of the `long` result.
    value as c_int
}

/// `int main(int argc, char** argv)`
///
/// `argc` is accepted but, exactly like the C code, never examined.
///
/// # Safety
///
/// `argv` must be a valid, NUL-pointer-terminated `char *` array as supplied by
/// a C runtime. Reads of `argv[1]` / `argv[2]` are performed unconditionally
/// and in that order, matching the C statement order, so a short `argv`
/// reproduces the original crash instead of avoiding it.
pub unsafe fn c_main(_argc: c_int, argv: *mut *mut c_char) -> c_int {
    // `int a = atoi(argv[1]);`
    let arg1 = core::ptr::read_volatile(argv.wrapping_add(1));
    let a = atoi(arg1);

    // `int b = atoi(argv[2]);`
    let arg2 = core::ptr::read_volatile(argv.wrapping_add(2));
    let b = atoi(arg2);

    // `struct test t; memset(&t, 0, sizeof(t)); t.a = a; t.b = b;`
    let mut t = Test { a: 0, b: 0 };
    t.a = a;
    t.b = b;

    // `printf("%d\n", find_container_of_a(&t.a)->a + find_container_of_b(&t.b)->b);`
    //
    // Both `container_of` recoveries land back on `&t`; the arithmetic is done
    // through the real translated helpers so the pointer maths is exercised.
    let pa = find_container_of_a(&mut t.a as *mut c_int);
    let pb = find_container_of_b(&mut t.b as *mut c_int);
    // Signed overflow wraps on the two's-complement hardware this targets.
    let sum = (*pa).a.wrapping_add((*pb).b);

    print_int_line(sum);

    // `main` falls off the end of its body: C99 onwards specifies `return 0`.
    0
}

/// `printf("%d\n", value)` -- writes the shortest decimal representation of a
/// 32-bit signed integer followed by a single newline, and nothing else.
///
/// Errors are discarded because `printf`'s return value is discarded by the C
/// program too (a failing write does not change its exit status).
fn print_int_line(value: c_int) {
    use std::io::Write;

    let mut buf = [0u8; 12];
    let text = format_int(value, &mut buf);

    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let _ = out.write_all(text);
    let _ = out.flush();
}

/// Renders `value` exactly as `printf("%d\n")` would, including the trailing
/// newline, into `buf`, and returns the populated slice.
///
/// `i32::MIN` needs 11 characters plus the newline, hence the 12-byte buffer.
fn format_int(value: c_int, buf: &mut [u8; 12]) -> &[u8] {
    let mut pos = buf.len();

    // Write the newline first, then the digits right-to-left.
    pos -= 1;
    buf[pos] = b'\n';

    // Work in the unsigned domain so that `i32::MIN` needs no special case.
    let negative = value < 0;
    let mut magnitude = (value as i64).unsigned_abs();

    loop {
        pos -= 1;
        buf[pos] = b'0' + (magnitude % 10) as u8;
        magnitude /= 10;
        if magnitude == 0 {
            break;
        }
    }

    if negative {
        pos -= 1;
        buf[pos] = b'-';
    }

    &buf[pos..]
}
