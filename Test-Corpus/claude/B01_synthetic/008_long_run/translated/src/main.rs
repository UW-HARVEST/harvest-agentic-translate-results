// Rust translation of c_src/src/main.c
//
// Behaviour-preserving port: the same argument validation order, the same
// glibc pseudo-random number stream, the same (wrapping) signed integer
// arithmetic, and the same stdout/stderr text.

mod rng;
mod strtoul;

use std::io::Write;
use std::os::unix::ffi::OsStrExt;

const ARRAY_SIZE: usize = 256 * 1024; // 1MB assuming sizeof(int) = 4
const ITERATIONS: usize = 2000;

const UINT_MAX: u64 = u32::MAX as u64;

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

/// Perform expensive arithmetic on each element.
fn perform_expensive_operations(array: &mut [i32]) {
    for slot in array.iter_mut() {
        let mut x = *slot;
        for _ in 0..100 {
            x = expensive_step(x);
        }
        *slot = x;
    }
}

fn main() {
    let args: Vec<std::ffi::OsString> = std::env::args_os().collect();

    // if (argc != 2) { fprintf(stderr, "Usage: %s <seed>\n", argv[0]); return 1; }
    if args.len() != 2 {
        let argv0: &[u8] = match args.first() {
            Some(a) => a.as_bytes(),
            // argc == 0: glibc's printf renders a NULL "%s" as "(null)".
            None => b"(null)",
        };
        let mut stderr = std::io::stderr();
        let _ = stderr.write_all(b"Usage: ");
        let _ = stderr.write_all(argv0);
        let _ = stderr.write_all(b" <seed>\n");
        std::process::exit(1);
    }

    let arg1 = args[1].as_bytes();

    // errno = 0; strtoul(argv[1], &endptr, 10);
    let parsed = strtoul::strtoul_base10(arg1);

    // if (*endptr != '\0' || errno != 0 || temp_seed > UINT_MAX)
    let endptr_not_nul = parsed.consumed != arg1.len();
    if endptr_not_nul || parsed.erange || parsed.value > UINT_MAX {
        let mut stderr = std::io::stderr();
        let _ = stderr.write_all(b"Invalid seed: '");
        let _ = stderr.write_all(arg1);
        let _ = stderr.write_all(b"'\n");
        std::process::exit(1);
    }

    let seed = parsed.value as u32;
    let mut generator = rng::GlibcRand::new(seed);

    let mut array = vec![0i32; ARRAY_SIZE];
    for slot in array.iter_mut() {
        *slot = generator.next_i32();
    }

    for _ in 0..ITERATIONS {
        perform_expensive_operations(&mut array);
    }

    let mut xor_result: i32 = 0;
    for &value in array.iter() {
        xor_result ^= value;
    }

    let mut stdout = std::io::stdout();
    let _ = writeln!(stdout, "{}", xor_result);
    let _ = stdout.flush();
}
