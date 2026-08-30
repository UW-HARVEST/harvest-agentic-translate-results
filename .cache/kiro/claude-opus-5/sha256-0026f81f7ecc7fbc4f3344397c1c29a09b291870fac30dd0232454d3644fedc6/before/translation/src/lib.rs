// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge,
// to any person obtaining a copy of this software
// and associated documentation files (the "Software"),
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
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

//! Rust translation of `c_src/src/long.c`.
//!
//! The translation is byte-for-byte output compatible with the C original:
//!   * `srand`/`rand` are reimplemented as glibc's TYPE_3 additive-feedback
//!     generator (degree 31, separation 3), which is what the C code observes
//!     when linked against glibc.
//!   * The arithmetic kernel uses wrapping / truncating operations that match
//!     what a two's-complement C compiler emits for the original expressions.
//!   * The result is emitted with C `printf` so buffering and formatting are
//!     identical.

use std::ffi::{c_char, c_int, c_uint};

const ARRAY_SIZE: usize = 256 * 1024; // 1MB assuming sizeof(int) = 4
const ITERATIONS: c_int = 2000;

unsafe extern "C" {
    #[link_name = "printf"]
    unsafe fn c_printf(fmt: *const c_char, ...) -> c_int;
}

// Global array (mirrors the C global of the same name, including its symbol).
#[allow(non_upper_case_globals)]
#[unsafe(no_mangle)]
pub static mut array: [c_int; ARRAY_SIZE] = [0; ARRAY_SIZE];

/// Reimplementation of glibc's `random()` state machine, as used by
/// `srand()` / `rand()` (TYPE_3: degree 31, separation 3).
struct GlibcRand {
    state: [u32; DEG],
    fptr: usize,
    rptr: usize,
}

const DEG: usize = 31;
const SEP: usize = 3;

impl GlibcRand {
    fn new(seed: c_uint) -> Self {
        let mut rng = GlibcRand {
            state: [0; DEG],
            fptr: SEP,
            rptr: 0,
        };
        rng.srand(seed);
        rng
    }

    fn srand(&mut self, seed: c_uint) {
        // "We must make sure the seed is not 0.  Take arbitrarily 1 in this case."
        let seed = if seed == 0 { 1 } else { seed };
        self.state[0] = seed;

        // state[i] = (16807 * state[i - 1]) % 2147483647, computed without
        // overflowing 31 bits (Schrage's method). glibc keeps `word` in an
        // `int32_t` while the intermediate products are `long int`, so seeds
        // above INT32_MAX are first wrapped to a negative value.
        let mut word: i32 = seed as i32;
        for i in 1..DEG {
            let w = word as i64;
            let hi = w / 127773;
            let lo = w % 127773;
            let mut next = 16807 * lo - 2836 * hi;
            if next < 0 {
                next += 2147483647;
            }
            word = next as i32;
            self.state[i] = word as u32;
        }

        self.fptr = SEP;
        self.rptr = 0;

        // Discard the first 10 * DEG outputs.
        for _ in 0..(DEG * 10) {
            self.next_i32();
        }
    }

    fn next_i32(&mut self) -> i32 {
        let val = self.state[self.fptr].wrapping_add(self.state[self.rptr]);
        self.state[self.fptr] = val;

        // Chucking least random bit.
        let result = ((val >> 1) & 0x7fff_ffff) as i32;

        self.fptr += 1;
        if self.fptr >= DEG {
            self.fptr = 0;
            self.rptr += 1;
        } else {
            self.rptr += 1;
            if self.rptr >= DEG {
                self.rptr = 0;
            }
        }

        result
    }

    /// Equivalent of C `rand()`.
    fn rand(&mut self) -> c_int {
        self.next_i32()
    }
}

/// Body of the C inner loop:
/// ```c
/// for (int j = 0; j < 100; j++) {
///     x = x * 3 + 7;
///     x = x ^ (x >> 3);
///     x = x - (x << 1);
///     x = x / 2 + x % 7;
/// }
/// ```
#[inline]
fn expensive_element(mut x: i32) -> i32 {
    for _ in 0..100 {
        x = x.wrapping_mul(3).wrapping_add(7);
        x ^= x >> 3;
        x = x.wrapping_sub(x.wrapping_shl(1));
        x = (x / 2).wrapping_add(x % 7);
    }
    x
}

/// Perform expensive arithmetic on each element.
#[unsafe(no_mangle)]
pub extern "C" fn perform_expensive_operations() {
    let arr: &mut [c_int] = unsafe { &mut *(&raw mut array) };
    for slot in arr.iter_mut() {
        *slot = expensive_element(*slot);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn long_exec(seed: c_uint) {
    let mut rng = GlibcRand::new(seed);

    let arr: &mut [c_int] = unsafe { &mut *(&raw mut array) };
    for slot in arr.iter_mut() {
        *slot = rng.rand();
    }

    for _ in 0..ITERATIONS {
        perform_expensive_operations();
    }

    let arr: &[c_int] = unsafe { &*(&raw const array) };
    let mut xor_result: c_int = 0;
    for &v in arr.iter() {
        xor_result ^= v;
    }

    unsafe {
        c_printf(c"%d\n".as_ptr(), xor_result);
    }
}

#[cfg(test)]
mod tests {
    use super::{GlibcRand, expensive_element};

    #[test]
    fn matches_glibc_rand_long_sequence() {
        // Reference: glibc srand(4294967295) then 262144 rand() calls.
        let mut rng = GlibcRand::new(4294967295);
        let mut xor: i32 = 0;
        let mut last3 = [0i32; 3];
        for i in 0..262144usize {
            let r = rng.rand();
            xor ^= r;
            if i >= 262141 {
                last3[i - 262141] = r;
            }
        }
        assert_eq!(last3, [396216925, 2081837121, 1154106855]);
        assert_eq!(xor, 271341985);
    }

    #[test]
    fn matches_c_kernel() {
        // Reference values from the original C inner loop (gcc, -O0 and -O2 agree).
        let cases: [(i32, i32); 18] = [
            (0, -626538949),
            (1, -1057168239),
            (-1, -626500583),
            (2, -626277382),
            (-2, -1057283197),
            (7, -822186310),
            (-7, -626277382),
            (3, -626277382),
            (i32::MAX, -627633746),
            (i32::MIN, -756415197),
            (i32::MAX - 1, -988934373),
            (i32::MIN + 1, -627633746),
            (1804289383, -806169092),
            (846930886, -650520112),
            (-1073741824, -951240585),
            (123456789, -860622313),
            (-987654321, -855633797),
            (1431655765, -1057168239),
        ];

        for (x, expected) in cases {
            assert_eq!(expensive_element(x), expected, "x = {x}");
        }
    }

    #[test]
    fn matches_glibc_rand() {
        // Values produced by glibc: srand(S); rand() x 5
        let cases: [(u32, [i32; 5]); 4] = [
            (0, [1804289383, 846930886, 1681692777, 1714636915, 1957747793]),
            (1, [1804289383, 846930886, 1681692777, 1714636915, 1957747793]),
            (42, [71876166, 708592740, 1483128881, 907283241, 442951012]),
            (
                4294967295,
                [254925627, 1205188300, 366127624, 1401405153, 76053476],
            ),
        ];

        for (seed, expected) in cases {
            let mut rng = GlibcRand::new(seed);
            let got: [i32; 5] = std::array::from_fn(|_| rng.rand());
            assert_eq!(got, expected, "seed {seed}");
        }
    }
}
