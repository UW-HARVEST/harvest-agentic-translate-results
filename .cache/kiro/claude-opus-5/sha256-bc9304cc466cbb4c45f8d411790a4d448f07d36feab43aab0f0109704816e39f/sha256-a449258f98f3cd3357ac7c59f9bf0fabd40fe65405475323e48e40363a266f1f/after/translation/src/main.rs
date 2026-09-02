// Rust translation of c_src/src/container_of.c
//
// The original program:
//   * reads two command line arguments with atoi()
//   * stores them in a `struct test { int a; int b; }`
//   * recovers the containing struct from a pointer to each member using the
//     classic `container_of` macro
//   * prints the sum of the two members with printf("%d\n", ...)
//
// Behaviour that is reproduced exactly (not "fixed"):
//   * glibc's atoi(): leading whitespace is skipped, an optional sign is
//     accepted, digits are accumulated as a `long` which saturates at
//     LONG_MIN/LONG_MAX, and the result is then truncated to `int`.
//     e.g. "99999999999" -> 1215752191, "-99999999999999999999" -> 0
//   * non-numeric input yields 0 (no error, no diagnostic)
//   * signed overflow of `a + b` wraps like gcc does at -O2
//     e.g. 2147483647 + 1 -> -2147483648
//   * a missing argv[1] / argv[2] dereferences a NULL pointer in atoi(),
//     which terminates the process with SIGSEGV and no output

use std::io::Write;

#[repr(C)]
struct Test {
    a: i32,
    b: i32,
}

/// `container_of(ptr, struct test, a)`
///
/// Mirrors the C macro: back up from the member pointer by the member's
/// offset to obtain a pointer to the enclosing struct. The offset for `a`
/// is 0, so the resulting pointer is the struct itself.
fn find_container_of_a(i: &i32) -> &Test {
    container_of(i, std::mem::offset_of!(Test, a))
}

/// `container_of(ptr, struct test, b)`
fn find_container_of_b(i: &i32) -> &Test {
    container_of(i, std::mem::offset_of!(Test, b))
}

fn container_of(member: &i32, offset: usize) -> &Test {
    // The pointer arithmetic below stays inside the original allocation,
    // exactly like the C macro does for these two call sites.
    unsafe {
        let base = (member as *const i32 as *const u8).sub(offset) as *const Test;
        &*base
    }
}

/// Faithful re-implementation of glibc's `atoi` (i.e. `(int) strtol(s, NULL, 10)`).
fn atoi(s: &[u8]) -> i32 {
    let mut idx = 0usize;

    // isspace() in the C locale
    while idx < s.len() && matches!(s[idx], b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r') {
        idx += 1;
    }

    let mut negative = false;
    if idx < s.len() && (s[idx] == b'+' || s[idx] == b'-') {
        negative = s[idx] == b'-';
        idx += 1;
    }

    // Accumulate as a long, saturating at the long boundaries like strtol.
    let mut acc: i64 = 0;
    let mut saturated = false;
    while idx < s.len() && s[idx].is_ascii_digit() {
        let digit = i64::from(s[idx] - b'0');
        if !saturated {
            match acc.checked_mul(10).and_then(|v| v.checked_add(digit)) {
                Some(v) => acc = v,
                None => saturated = true,
            }
        }
        idx += 1;
    }

    let value: i64 = if saturated {
        if negative {
            i64::MIN
        } else {
            i64::MAX
        }
    } else if negative {
        // -acc cannot overflow here: acc <= i64::MAX
        -acc
    } else {
        acc
    };

    // strtol's `long` result is truncated to `int` by atoi.
    value as i32
}

/// Reproduce the NULL-pointer dereference that glibc's atoi performs when
/// handed a missing argv entry: the process dies with SIGSEGV, no output.
fn null_deref() -> ! {
    let _ = std::io::stdout().flush();
    unsafe {
        // read_volatile is never optimized away, so this really does load
        // from address 0 and raise SIGSEGV.
        std::ptr::read_volatile(std::ptr::null::<u8>());
    }
    // Unreachable in practice; keep the signature honest.
    std::process::abort();
}

fn arg(argv: &[Vec<u8>], index: usize) -> &[u8] {
    match argv.get(index) {
        Some(v) => v.as_slice(),
        None => null_deref(),
    }
}

fn main() {
    let argv: Vec<Vec<u8>> = {
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStrExt;
            std::env::args_os()
                .map(|a| a.as_os_str().as_bytes().to_vec())
                .collect()
        }
        #[cfg(not(unix))]
        {
            std::env::args_os()
                .map(|a| a.to_string_lossy().into_owned().into_bytes())
                .collect()
        }
    };

    // Same evaluation order as the C: argv[1] first, then argv[2].
    let a = atoi(arg(&argv, 1));
    let b = atoi(arg(&argv, 2));

    let mut t = Test { a: 0, b: 0 }; // memset(&t, 0, sizeof(t))
    t.a = a;
    t.b = b;

    let sum = find_container_of_a(&t.a)
        .a
        .wrapping_add(find_container_of_b(&t.b).b);

    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let _ = write!(out, "{}\n", sum);
    let _ = out.flush();
}
