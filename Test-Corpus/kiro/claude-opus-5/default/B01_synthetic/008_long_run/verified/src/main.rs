// Rust translation of c_src/src/main.c
//
// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge,
// to any person obtaining a copy of this software
// and associated documentation files (the “Software”),
// to deal in the Software without restriction,
// including without limitation the rights to use, copy,
// modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software,
// and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED “AS IS”, WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

mod cstrtoul;
mod glibc_rand;

use std::ffi::OsString;
use std::io::Write;
use std::os::unix::ffi::OsStrExt;
use std::process::ExitCode;
use std::thread;

use cstrtoul::strtoul_base10;
use glibc_rand::GlibcRand;

/// `#define ARRAY_SIZE (256 * 1024)`
const ARRAY_SIZE: usize = 256 * 1024;
/// `#define ITERATIONS 2000`
const ITERATIONS: usize = 2000;
/// Length of the inner arithmetic loop in `perform_expensive_operations()`.
const INNER_STEPS: usize = 100;

/// `UINT_MAX` on the reference platform.
const UINT_MAX: u64 = u32::MAX as u64;

extern "C" {
    /// libc `signal()`; used only to restore the default SIGPIPE disposition.
    fn signal(signum: i32, handler: usize) -> usize;
}

/// One iteration of the inner loop of `perform_expensive_operations()`.
///
/// The C code relies on signed overflow, on an arithmetic right shift of a
/// negative value and on a left shift that overflows -- all of which are
/// undefined or implementation-defined in C but which gcc realises as plain
/// two's-complement wrap-around.  The wrapping operators below reproduce that
/// behaviour exactly, including truncating division/remainder toward zero.
#[inline(always)]
fn expensive_step(mut x: i32) -> i32 {
    // x = x * 3 + 7;
    x = x.wrapping_mul(3).wrapping_add(7);
    // x = x ^ (x >> 3);   (arithmetic shift, as gcc does for signed types)
    x ^= x >> 3;
    // x = x - (x << 1);   (the shift discards the bit shifted out)
    x = x.wrapping_sub(((x as u32) << 1) as i32);
    // x = x / 2 + x % 7;  (C truncates toward zero, same as Rust)
    x = (x / 2).wrapping_add(x % 7);
    x
}

/// Apply the full `ITERATIONS * INNER_STEPS` transformation to a single element.
#[inline(always)]
fn transform_element(mut x: i32) -> i32 {
    for _ in 0..ITERATIONS {
        for _ in 0..INNER_STEPS {
            x = expensive_step(x);
        }
    }
    x
}

/// The C program calls `perform_expensive_operations()` `ITERATIONS` times over
/// the whole array.  Each element evolves independently of every other one, so
/// running the complete per-element transformation is equivalent to the C loop
/// nesting while allowing the work to be spread over several threads.  The
/// final XOR reduction is order-independent, so the result is identical.
fn perform_all_iterations(array: &mut [i32]) {
    let threads = thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
        .min(array.len().max(1));

    let chunk_len = array.len().div_ceil(threads);

    thread::scope(|scope| {
        for chunk in array.chunks_mut(chunk_len) {
            scope.spawn(move || {
                for slot in chunk.iter_mut() {
                    *slot = transform_element(*slot);
                }
            });
        }
    });
}

fn main() -> ExitCode {
    // The C program is an ordinary C process, so SIGPIPE is left at its default
    // (terminate) disposition.  The Rust runtime ignores SIGPIPE before `main`
    // runs, which would make the two programs disagree if stdout were a closed
    // pipe, so restore the C behaviour.
    unsafe {
        // SIGPIPE == 13, SIG_DFL == 0 on Linux.
        signal(13, 0);
    }

    let args: Vec<OsString> = std::env::args_os().collect();

    // if (argc != 2) { fprintf(stderr, "Usage: %s <seed>\n", argv[0]); return 1; }
    if args.len() != 2 {
        let argv0: &[u8] = args.first().map(|a| a.as_bytes()).unwrap_or(b"");
        let mut stderr = std::io::stderr();
        let _ = stderr.write_all(b"Usage: ");
        let _ = stderr.write_all(argv0);
        let _ = stderr.write_all(b" <seed>\n");
        return ExitCode::from(1);
    }

    let arg = args[1].as_bytes();

    // errno = 0;
    // unsigned long temp_seed = strtoul(argv[1], &endptr, 10);
    let parsed = strtoul_base10(arg);

    // if (*endptr != '\0' || errno != 0 || temp_seed > UINT_MAX)
    //
    // A C string ends at its NUL terminator, and argv strings cannot contain
    // embedded NULs, so `*endptr == '\0'` is exactly "endptr reached the end".
    let end_is_nul = parsed.end_offset == arg.len();
    if !end_is_nul || parsed.erange || parsed.value > UINT_MAX {
        let mut stderr = std::io::stderr();
        let _ = stderr.write_all(b"Invalid seed: '");
        let _ = stderr.write_all(arg);
        let _ = stderr.write_all(b"'\n");
        return ExitCode::from(1);
    }

    // unsigned int seed = (unsigned int)temp_seed;
    // srand(seed);
    let seed = parsed.value as u32;
    let mut rng = GlibcRand::new(seed);

    // The C array is a zero-initialised global; every slot is overwritten with
    // rand() before it is read.
    let mut array: Vec<i32> = vec![0; ARRAY_SIZE];
    for slot in array.iter_mut() {
        *slot = rng.next_i32();
    }

    perform_all_iterations(&mut array);

    // int xor_result = 0; for (...) xor_result ^= array[i];
    let mut xor_result: i32 = 0;
    for &value in array.iter() {
        xor_result ^= value;
    }

    // printf("%d\n", xor_result);
    let mut stdout = std::io::stdout();
    let _ = write!(stdout, "{}\n", xor_result);
    let _ = stdout.flush();

    ExitCode::from(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Reference values captured from glibc's own `srand()`/`rand()`.
    #[test]
    fn matches_glibc_rand_stream() {
        let cases: &[(u32, [i32; 8], [i32; 4])] = &[
            (
                0,
                [
                    1804289383, 846930886, 1681692777, 1714636915, 1957747793, 424238335,
                    719885386, 1649760492,
                ],
                [1489001354, 953691761, 507578762, 1402492972],
            ),
            (
                1,
                [
                    1804289383, 846930886, 1681692777, 1714636915, 1957747793, 424238335,
                    719885386, 1649760492,
                ],
                [1489001354, 953691761, 507578762, 1402492972],
            ),
            (
                42,
                [
                    71876166, 708592740, 1483128881, 907283241, 442951012, 537146758, 1366999021,
                    1854614940,
                ],
                [953844426, 1600182443, 1245716983, 93014782],
            ),
            (
                12345,
                [
                    383100999, 858300821, 357768173, 455528251, 133005921, 116285904, 591987137,
                    102557902,
                ],
                [73389321, 102937240, 294230873, 642804576],
            ),
            (
                2147483647,
                [
                    1065668062, 2142264300, 1066566375, 1064012770, 2141034222, 1065509725,
                    2135810236, 2139491828,
                ],
                [1738459046, 1700097590, 1518965010, 217494216],
            ),
            (
                2147483648,
                [
                    1336741213, 1210407648, 1447044896, 337392383, 82502902, 538660432, 1313908778,
                    370221063,
                ],
                [1932232815, 1100906073, 2127327884, 1278847013],
            ),
            (
                4294967295,
                [
                    254925627, 1205188300, 366127624, 1401405153, 76053476, 1604170158, 1302235366,
                    362229243,
                ],
                [1523208214, 653520015, 1498809246, 1496341229],
            ),
            (
                3000000000,
                [
                    2058147116, 854483408, 922419988, 286396165, 2068523933, 1172167191, 573677598,
                    1899216469,
                ],
                [241592599, 270862027, 1380345186, 1385378635],
            ),
        ];

        for &(seed, ref first, ref later) in cases {
            let mut rng = GlibcRand::new(seed);
            for (i, &expected) in first.iter().enumerate() {
                assert_eq!(rng.next_i32(), expected, "seed {seed}, draw {i}");
            }
            for _ in 0..1000 {
                rng.next_i32();
            }
            for (i, &expected) in later.iter().enumerate() {
                assert_eq!(rng.next_i32(), expected, "seed {seed}, draw {} (late)", 1008 + i);
            }
        }
    }

    /// Values produced by the real C kernel for the first 24 steps.
    #[test]
    fn matches_c_kernel_trajectory() {
        let expected: [i32; 24] = [
            -628815138, -1061118560, -621081090, -829917089, -863314434, -881476770, -924224053,
            -687208098, -987688919, -591899471, -846655728, -851715145, -899870884, -712697330,
            -944722945, -788013638, -1052423437, -635113091, -1070696313, -609026651, -817755125,
            -809463897, -827641239, -817943255,
        ];
        let mut x = 1804289383i32;
        for (i, &want) in expected.iter().enumerate() {
            x = expensive_step(x);
            assert_eq!(x, want, "step {i}");
        }

        // i32::MIN start, exercising the extreme wrap-around cases.
        let expected_min: [i32; 6] = [
            -939524099, -780140547, -1031274497, -666206213, -1021462535, -540606855,
        ];
        let mut y = i32::MIN;
        for (i, &want) in expected_min.iter().enumerate() {
            y = expensive_step(y);
            assert_eq!(y, want, "min step {i}");
        }

        // Further trajectories captured from the C kernel compiled as-is
        // (`cc -O0`), covering INT_MAX, small magnitudes and both signs.
        let more: &[(i32, [i32; 8])] = &[
            (
                i32::MAX,
                [
                    -939524102, -780140555, -1031274514, -666009623, -1023064097, -538384043,
                    -908522953, -723336896,
                ],
            ),
            (0, [-3, -1, -6, -9, -11, -18, -22, -35]),
            (-1, [-6, -9, -11, -18, -22, -35, -58, -92]),
            (1, [-9, -11, -18, -22, -35, -58, -92, -150]),
            (7, [-18, -22, -35, -58, -92, -150, -203, -270]),
            (-7, [-11, -18, -22, -35, -58, -92, -150, -203]),
            (2, [-11, -18, -22, -35, -58, -92, -150, -203]),
            (-2, [-1, -6, -9, -11, -18, -22, -35, -58]),
            (
                1000000007,
                [
                    -574958323, -889927784, -912444939, -732345262, -961891382, -792726532,
                    -1044123715, -653351792,
                ],
            ),
            (
                -1000000007,
                [
                    -574958316, -889927771, -912444952, -732345223, -961891571, -792726792,
                    -1044124157, -653351341,
                ],
            ),
        ];
        for &(start, ref want) in more {
            let mut z = start;
            for (i, &expected) in want.iter().enumerate() {
                z = expensive_step(z);
                assert_eq!(z, expected, "start {start}, step {i}");
            }
        }
    }

    #[test]
    fn strtoul_edge_cases() {
        // (input, accepted_by_program, seed_if_accepted)
        let cases: &[(&str, bool, u32)] = &[
            ("", true, 0),
            (" ", false, 0),
            ("12abc", false, 0),
            ("abc", false, 0),
            ("-1", false, 0),
            ("-0", true, 0),
            ("+7", true, 7),
            (" 42", true, 42),
            ("4294967296", false, 0),
            ("4294967295", true, 4294967295),
            ("99999999999999999999999999", false, 0),
            ("0x10", false, 0),
            ("42 ", false, 0),
            ("1.5", false, 0),
            ("  +0000000042", true, 42),
        ];

        for &(input, accepted, seed) in cases {
            let bytes = input.as_bytes();
            let r = strtoul_base10(bytes);
            let ok = r.end_offset == bytes.len() && !r.erange && r.value <= UINT_MAX;
            assert_eq!(ok, accepted, "input {input:?}");
            if accepted {
                assert_eq!(r.value as u32, seed, "input {input:?}");
            }
        }
    }
}
