// Translation of c_src/src/container_of.c
//
// The original program:
//   * reads argv[1] and argv[2] with atoi() -- without ever checking argc
//   * zero-fills a `struct test { int a; int b; }`, stores the two values
//   * uses the classic `container_of` macro to recover a pointer to the
//     enclosing struct from a pointer to one of its members
//   * prints `t.a + t.b` with "%d\n"
//
// Behaviour that is deliberately reproduced rather than "fixed":
//   * no argc validation: with fewer than two arguments the C code calls
//     atoi(NULL) and dies on a NULL dereference, having printed nothing.
//     We abort the process instead, which likewise emits nothing on stdout.
//   * glibc's atoi() is `(int)strtol(s, NULL, 10)`: leading whitespace is
//     skipped, an optional sign is honoured, parsing stops at the first
//     non-digit, out-of-range values saturate at LONG_MIN/LONG_MAX and are
//     then truncated to `int`.
//   * the final addition wraps on overflow, as it does in practice with gcc.

use std::io::Write;

/// The `int` size assumed by the original C code on the target platform.
const INT_SIZE: usize = 4;

/// `struct test { int a; int b; };` modelled as raw storage so that the
/// pointer arithmetic performed by `container_of` can be reproduced exactly
/// while staying inside safe Rust.
struct Test {
    storage: [u8; 2 * INT_SIZE],
}

/// Byte offsets matching `offsetof(struct test, MEMBER)`.
const OFFSET_A: usize = 0;
const OFFSET_B: usize = INT_SIZE;

/// A stand-in for a C pointer into `Test`: the object it points into plus a
/// byte offset. Offsets are signed so that the subtraction performed by
/// `container_of` can be represented even if it would go out of bounds.
#[derive(Clone, Copy)]
struct MemberPtr {
    offset: isize,
}

impl Test {
    /// `memset(&t, 0, sizeof(t));`
    fn zeroed() -> Test {
        Test {
            storage: [0u8; 2 * INT_SIZE],
        }
    }

    fn store(&mut self, offset: usize, value: i32) {
        self.storage[offset..offset + INT_SIZE].copy_from_slice(&value.to_ne_bytes());
    }

    fn load(&self, offset: usize) -> i32 {
        let mut bytes = [0u8; INT_SIZE];
        bytes.copy_from_slice(&self.storage[offset..offset + INT_SIZE]);
        i32::from_ne_bytes(bytes)
    }

    /// `&t.a`
    fn ptr_a(&self) -> MemberPtr {
        MemberPtr {
            offset: OFFSET_A as isize,
        }
    }

    /// `&t.b`
    fn ptr_b(&self) -> MemberPtr {
        MemberPtr {
            offset: OFFSET_B as isize,
        }
    }
}

/// `container_of(ptr, struct test, a)`: subtract offsetof(struct test, a).
fn find_container_of_a(i: MemberPtr) -> MemberPtr {
    MemberPtr {
        offset: i.offset - OFFSET_A as isize,
    }
}

/// `container_of(ptr, struct test, b)`: subtract offsetof(struct test, b).
fn find_container_of_b(i: MemberPtr) -> MemberPtr {
    MemberPtr {
        offset: i.offset - OFFSET_B as isize,
    }
}

/// Read member `a` through a `struct test *`.
fn deref_a(t: &Test, base: MemberPtr) -> i32 {
    t.load((base.offset + OFFSET_A as isize) as usize)
}

/// Read member `b` through a `struct test *`.
fn deref_b(t: &Test, base: MemberPtr) -> i32 {
    t.load((base.offset + OFFSET_B as isize) as usize)
}

/// glibc-compatible `atoi`.
fn atoi(bytes: &[u8]) -> i32 {
    let mut idx = 0usize;

    // isspace()
    while idx < bytes.len()
        && matches!(bytes[idx], b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
    {
        idx += 1;
    }

    let mut negative = false;
    if idx < bytes.len() && (bytes[idx] == b'+' || bytes[idx] == b'-') {
        negative = bytes[idx] == b'-';
        idx += 1;
    }

    // Accumulate with saturation at the long (i64) boundaries, mirroring
    // strtol, then truncate to int as the cast in atoi does.
    let mut acc: i64 = 0;
    let mut saturated = false;
    while idx < bytes.len() && bytes[idx].is_ascii_digit() {
        let digit = i64::from(bytes[idx] - b'0');
        if !saturated {
            match acc.checked_mul(10).and_then(|v| v.checked_add(digit)) {
                Some(next) => acc = next,
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
        -acc
    } else {
        acc
    };

    value as i32
}

fn main() {
    let argv: Vec<std::ffi::OsString> = std::env::args_os().collect();

    // The C code indexes argv[1] and argv[2] unconditionally.
    let a = atoi(&arg_bytes(&argv, 1));
    let b = atoi(&arg_bytes(&argv, 2));

    let mut t = Test::zeroed();
    t.store(OFFSET_A, a);
    t.store(OFFSET_B, b);

    let via_a = deref_a(&t, find_container_of_a(t.ptr_a()));
    let via_b = deref_b(&t, find_container_of_b(t.ptr_b()));

    let mut stdout = std::io::stdout();
    let _ = write!(stdout, "{}\n", via_a.wrapping_add(via_b));
    let _ = stdout.flush();
}

/// Fetch argv[index] as bytes. A missing argument is a NULL pointer in C and
/// atoi() dereferences it, so terminate without producing any output.
fn arg_bytes(argv: &[std::ffi::OsString], index: usize) -> Vec<u8> {
    match argv.get(index) {
        Some(arg) => os_str_bytes(arg),
        None => std::process::abort(),
    }
}

#[cfg(unix)]
fn os_str_bytes(s: &std::ffi::OsStr) -> Vec<u8> {
    use std::os::unix::ffi::OsStrExt;
    s.as_bytes().to_vec()
}

#[cfg(not(unix))]
fn os_str_bytes(s: &std::ffi::OsStr) -> Vec<u8> {
    s.to_string_lossy().into_owned().into_bytes()
}
