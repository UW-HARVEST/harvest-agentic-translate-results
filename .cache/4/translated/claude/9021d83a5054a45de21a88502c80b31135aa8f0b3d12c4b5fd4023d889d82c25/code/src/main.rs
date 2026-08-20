// Rust translation of c_src/src/container_of.c
//
// The original C program:
//   * reads two integers from argv[1] / argv[2] using atoi()
//   * zero-initialises a `struct test { int a; int b; }` with memset
//   * stores the two values into the struct fields
//   * takes a pointer to each field, recovers the enclosing struct with the
//     classic `container_of` macro, reads the field back out and prints the
//     sum with printf("%d\n", ...)
//
// Because `container_of(&t.a, struct test, a)` and
// `container_of(&t.b, struct test, b)` both simply yield `&t`, the observable
// behaviour is `printf("%d\n", t.a + t.b)`. That is reproduced exactly here,
// including glibc's atoi() parsing rules and C's wrapping `int` arithmetic.

use std::io::Write;

/// Mirrors `struct test { int a; int b; }`.
struct Test {
    a: i32,
    b: i32,
}

impl Test {
    /// Equivalent of `memset(&t, 0, sizeof(t))` followed by nothing else.
    fn zeroed() -> Test {
        Test { a: 0, b: 0 }
    }
}

/// `struct test* find_container_of_a(int *i)` — `container_of` on member `a`.
///
/// The C code converts an interior pointer back into a pointer to the whole
/// struct. In safe Rust the equivalent operation is simply handing back the
/// borrow of the container that the field borrow came from, so the pointer
/// arithmetic (`ptr - offsetof(struct test, a)`, i.e. `ptr - 0`) is modelled by
/// returning the container reference itself.
fn find_container_of_a(t: &Test) -> &Test {
    t
}

/// `struct test* find_container_of_b(int *i)` — `container_of` on member `b`.
///
/// Here the C macro subtracts `offsetof(struct test, b)` (4 bytes) from the
/// pointer to `t.b`, which again lands exactly on `&t`.
fn find_container_of_b(t: &Test) -> &Test {
    t
}

/// Faithful re-implementation of glibc's `atoi`, which is `(int) strtol(s, NULL, 10)`:
///
///   * leading whitespace (as recognised by `isspace`) is skipped
///   * an optional `+` / `-` sign is consumed
///   * decimal digits are accumulated; a value that would overflow `long` is
///     clamped to `LONG_MAX` / `LONG_MIN` (glibc sets ERANGE and saturates)
///   * the resulting `long` is truncated to `int`
///   * anything unparsable yields 0
fn atoi(bytes: &[u8]) -> i32 {
    let mut i = 0usize;

    // isspace(): ' ', '\t', '\n', '\v', '\f', '\r'
    while i < bytes.len()
        && matches!(bytes[i], b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
    {
        i += 1;
    }

    let negative = if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        let neg = bytes[i] == b'-';
        i += 1;
        neg
    } else {
        false
    };

    // Accumulate in i64 (== C `long` on LP64) with saturation, like strtol.
    let mut acc: i64 = 0;
    let mut saturated = false;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        let digit = (bytes[i] - b'0') as i64;
        if !saturated {
            match acc.checked_mul(10).and_then(|v| v.checked_add(digit)) {
                Some(v) => acc = v,
                None => saturated = true,
            }
        }
        i += 1;
    }

    let value: i64 = if saturated {
        if negative {
            i64::MIN
        } else {
            i64::MAX
        }
    } else if negative {
        // `acc` is non-negative here, so negating it can never overflow.
        -acc
    } else {
        acc
    };

    // (int) truncation of the long result.
    value as i32
}

fn main() {
    let args: Vec<Vec<u8>> = std::env::args_os().map(os_string_to_bytes).collect();

    // The C code dereferences argv[1] and argv[2] unconditionally. When they
    // are absent they are NULL (argv is NULL-terminated), and atoi(NULL)
    // dereferences a null pointer, crashing the process with SIGSEGV before
    // anything is printed. Reproduce that failure mode rather than "fixing" it.
    if args.len() < 3 {
        null_dereference();
    }

    let a = atoi(&args[1]);
    let b = atoi(&args[2]);

    let mut t = Test::zeroed();
    t.a = a;
    t.b = b;

    // printf("%d\n", find_container_of_a(&t.a)->a + find_container_of_b(&t.b)->b);
    // C `int` addition wraps on the usual two's-complement hardware.
    let sum = find_container_of_a(&t).a.wrapping_add(find_container_of_b(&t).b);

    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let _ = write!(out, "{}\n", sum);
    let _ = out.flush();
}

#[cfg(unix)]
fn os_string_to_bytes(s: std::ffi::OsString) -> Vec<u8> {
    use std::os::unix::ffi::OsStringExt;
    s.into_vec()
}

#[cfg(not(unix))]
fn os_string_to_bytes(s: std::ffi::OsString) -> Vec<u8> {
    s.to_string_lossy().into_owned().into_bytes()
}

/// Emulates the null-pointer dereference performed by `atoi(NULL)` in the
/// original program. A volatile read is never optimised away, so this really
/// does fault, matching the C binary's exit status (killed by SIGSEGV) and its
/// complete lack of output.
fn null_dereference() -> ! {
    unsafe {
        let p: *const u8 = std::ptr::null();
        std::ptr::read_volatile(p);
    }
    // Unreachable in practice; kept so the function can be typed as `!`.
    std::process::abort();
}
