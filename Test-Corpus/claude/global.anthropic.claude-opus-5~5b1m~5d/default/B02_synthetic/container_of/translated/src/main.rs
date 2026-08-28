// Rust translation of c_src/src/container_of.c
//
// The original program:
//   * reads two decimal integers from argv[1] and argv[2] using atoi()
//   * stores them in a `struct test { int a; int b; }` (after memset()ing it)
//   * recovers the address of the enclosing struct from the address of each
//     member using the classic `container_of()` macro
//   * prints `t.a + t.b` with printf("%d\n", ...)
//
// The C code performs no argument-count validation, so accessing argv[1] or
// argv[2] when they are absent dereferences a NULL pointer inside atoi() and
// the process dies with SIGSEGV.  That behaviour is reproduced here so that the
// observable output (nothing on stdout, death by SIGSEGV) matches.

use std::io::Write;
use std::os::unix::ffi::OsStrExt;

/// struct test { int a; int b; };
#[repr(C)]
struct Test {
    a: i32,
    b: i32,
}

/// container_of(ptr, struct test, a)
fn find_container_of_a(i: *const i32) -> *const Test {
    let offset = std::mem::offset_of!(Test, a) as isize;
    // (struct test *)((char *)ptr - offsetof(struct test, a))
    unsafe { (i as *const u8).offset(-offset) as *const Test }
}

/// container_of(ptr, struct test, b)
fn find_container_of_b(i: *const i32) -> *const Test {
    let offset = std::mem::offset_of!(Test, b) as isize;
    // (struct test *)((char *)ptr - offsetof(struct test, b))
    unsafe { (i as *const u8).offset(-offset) as *const Test }
}

/// glibc's `atoi`, which is `(int) strtol(nptr, NULL, 10)`:
///   * leading whitespace is skipped
///   * an optional '+' / '-' sign is accepted
///   * decimal digits are accumulated; on `long` overflow the value saturates
///     at LONG_MAX / LONG_MIN
///   * the resulting `long` is truncated to `int`
///   * a string with no digits yields 0
fn c_atoi(bytes: &[u8]) -> i32 {
    let mut idx = 0usize;

    while idx < bytes.len()
        && matches!(
            bytes[idx],
            b' ' | b'\t' | b'\n' | 0x0b /* \v */ | 0x0c /* \f */ | b'\r'
        )
    {
        idx += 1;
    }

    let mut negative = false;
    if idx < bytes.len() && (bytes[idx] == b'+' || bytes[idx] == b'-') {
        negative = bytes[idx] == b'-';
        idx += 1;
    }

    let mut acc: i64 = 0;
    let mut overflowed = false;
    while idx < bytes.len() && bytes[idx].is_ascii_digit() {
        let digit = i64::from(bytes[idx] - b'0');
        if !overflowed {
            match acc.checked_mul(10).and_then(|v| v.checked_add(digit)) {
                Some(v) => acc = v,
                None => overflowed = true,
            }
        }
        idx += 1;
    }

    let value: i64 = if overflowed {
        if negative {
            i64::MIN
        } else {
            i64::MAX
        }
    } else if negative {
        -acc
    } else {
        acc
    };

    // (int) truncation of the long result
    value as i32
}

/// Reproduce the NULL-pointer dereference that atoi() performs when the
/// requested argv slot does not exist.
fn null_dereference() -> ! {
    unsafe {
        std::ptr::read_volatile(std::ptr::null::<u8>());
    }
    // Unreachable in practice: the volatile load above faults.
    loop {
        std::hint::spin_loop();
    }
}

fn main() {
    let args: Vec<Vec<u8>> = std::env::args_os()
        .map(|arg| arg.as_bytes().to_vec())
        .collect();

    // int a = atoi(argv[1]);
    let a = match args.get(1) {
        Some(arg) => c_atoi(arg),
        None => null_dereference(),
    };
    // int b = atoi(argv[2]);
    let b = match args.get(2) {
        Some(arg) => c_atoi(arg),
        None => null_dereference(),
    };

    // struct test t; memset(&t, 0, sizeof(t));
    let mut t = Test { a: 0, b: 0 };
    t.a = a;
    t.b = b;

    // printf("%d\n", find_container_of_a(&t.a)->a + find_container_of_b(&t.b)->b);
    let left = unsafe { (*find_container_of_a(&t.a)).a };
    let right = unsafe { (*find_container_of_b(&t.b)).b };
    let sum = left.wrapping_add(right);

    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let _ = write!(out, "{}\n", sum);
    let _ = out.flush();
}
