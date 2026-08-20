//! Core of the C program (`c_src/src/main.c`), shared verbatim by
//! * the `driver` binary (`src/main.rs`), and
//! * the `libdriver.so` C-ABI shim (`src/lib.rs`).
//!
//! Everything here mirrors the C one-to-one: the same global `array` in .bss,
//! the same `perform_expensive_operations()` over it, the same validation
//! order, and the same stdout/stderr byte streams.

use crate::rng;
use crate::strtoul;

use std::io::Write;

pub const ARRAY_SIZE: usize = 256 * 1024; // 1MB assuming sizeof(int) = 4
pub const ITERATIONS: usize = 2000;

/// `UINT_MAX` from <limits.h> (32-bit `unsigned int` on this target).
pub const UINT_MAX: u64 = u32::MAX as u64;

/// `int array[ARRAY_SIZE];` — the C global, exported with the same name so an
/// external caller (or a differential test) sees exactly the same object.
#[allow(non_upper_case_globals)]
#[no_mangle]
pub static mut array: [i32; ARRAY_SIZE] = [0; ARRAY_SIZE];

#[inline(always)]
fn array_mut() -> &'static mut [i32; ARRAY_SIZE] {
    // Single-threaded, exactly like the C program's use of its global.
    unsafe { &mut *std::ptr::addr_of_mut!(array) }
}

/// One iteration of the inner (`j`) loop body of `perform_expensive_operations`.
///
/// The C code relies on wrap-around signed arithmetic (`x * 3 + 7`,
/// `x - (x << 1)`), an arithmetic right shift for `x >> 3`, and C's
/// truncate-toward-zero division/remainder; all of that is reproduced here.
#[inline(always)]
fn expensive_step(mut x: i32) -> i32 {
    x = x.wrapping_mul(3).wrapping_add(7);
    x ^= x >> 3;
    x = x.wrapping_sub(x.wrapping_shl(1));
    x = x / 2 + x % 7;
    x
}

/// `void perform_expensive_operations(void)` — perform expensive arithmetic on
/// each element of the global `array`.
#[no_mangle]
pub extern "C" fn perform_expensive_operations() {
    for slot in array_mut().iter_mut() {
        let mut x = *slot;
        for _ in 0..100 {
            x = expensive_step(x);
        }
        *slot = x;
    }
}

/// The C validation block:
///
/// ```c
/// errno = 0;
/// unsigned long temp_seed = strtoul(argv[1], &endptr, 10);
/// if (*endptr != '\0' || errno != 0 || temp_seed > UINT_MAX) -> reject
/// unsigned int seed = (unsigned int)temp_seed;
/// ```
///
/// `arg` is the NUL-terminated `argv[1]` without its terminator, so
/// `*endptr != '\0'` is equivalent to "strtoul did not consume every byte".
pub fn parse_seed(arg: &[u8]) -> Option<u32> {
    let parsed = strtoul::strtoul_base10(arg);

    let endptr_not_nul = parsed.consumed != arg.len();
    if endptr_not_nul || parsed.erange || parsed.value > UINT_MAX {
        return None;
    }

    Some(parsed.value as u32)
}

fn write_stderr(bytes: &[u8]) {
    // C's stderr is unbuffered; Rust's is too.
    let mut stderr = std::io::stderr();
    let _ = stderr.write_all(bytes);
}

/// `fprintf(stderr, "Usage: %s <seed>\n", argv[0]); return 1;`
///
/// `argv0 == None` models a NULL `argv[0]`, which glibc's `printf` renders as
/// `(null)`.
pub fn usage(argv0: Option<&[u8]>) -> i32 {
    write_stderr(b"Usage: ");
    write_stderr(argv0.unwrap_or(b"(null)"));
    write_stderr(b" <seed>\n");
    1
}

/// Everything the C `main` does once it knows `argc == 2`.
pub fn run_with_seed_arg(arg: &[u8]) -> i32 {
    let seed = match parse_seed(arg) {
        Some(seed) => seed,
        None => {
            // fprintf(stderr, "Invalid seed: '%s'\n", argv[1]); return 1;
            write_stderr(b"Invalid seed: '");
            write_stderr(arg);
            write_stderr(b"'\n");
            return 1;
        }
    };

    // srand(seed); for (i...) array[i] = rand();
    let mut generator = rng::GlibcRand::new(seed);
    for slot in array_mut().iter_mut() {
        *slot = generator.next_i32();
    }

    for _ in 0..ITERATIONS {
        perform_expensive_operations();
    }

    let mut xor_result: i32 = 0;
    for &value in array_mut().iter() {
        xor_result ^= value;
    }

    let mut stdout = std::io::stdout();
    let _ = writeln!(stdout, "{}", xor_result);
    let _ = stdout.flush();
    0
}

/// The body of C `main`: `argv` is modelled as the list of argument byte
/// strings (`None` = NULL pointer).
///
/// Used by the `driver` binary; `lib.rs` splits the same two branches apart so
/// that it can mirror C's lazy `argv[0]` / `argv[1]` dereferences.
#[allow(dead_code)]
pub fn run(argv: &[Option<&[u8]>]) -> i32 {
    if argv.len() != 2 {
        return usage(argv.first().copied().flatten());
    }
    // argc == 2 with a NULL argv[1] would fault inside C's strtoul; there is no
    // observable C behaviour to mirror, so treat it as the empty string.
    run_with_seed_arg(argv[1].unwrap_or(b""))
}
