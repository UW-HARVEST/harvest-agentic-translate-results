//! Rust translation of `c_src/src/lib.c`.
//!
//! The C file defines a family of sample-prediction routines plus a
//! selector (`BTAC1C2_GetPredictFunc`) that hands back a function pointer,
//! and one exported entry point (`get_predict_func`) that checks whether the
//! selector returned the pointer expected for the given predictor number.
//!
//! The translation keeps the original structure -- real function pointers are
//! produced and compared by address -- so any quirk of the C control flow is
//! reproduced rather than folded into a constant. Arithmetic quirks/bugs in the
//! individual predictors (e.g. the `Pfn10`/`Pfn11` shift counts differing from
//! the corresponding `switch` arms) are preserved verbatim.

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use std::ffi::{c_int, c_void};

type btac1c_u16 = u16;
type btac1c_s16 = i16;
type btac1c_byte = u8;

#[repr(C)]
pub struct btac1c_idxstate {
    pub idx: btac1c_u16,
    pub lpred: btac1c_s16,
    pub rpred: btac1c_s16,
    pub tag: btac1c_byte,
    pub bcfcn: btac1c_byte,
    pub bsfcn: btac1c_byte,
    pub usefx: btac1c_byte,
    pub firfx: [[btac1c_s16; 8]; 4],
}

/// Signature shared by every predictor function in the C source.
type PredictFn =
    unsafe extern "C" fn(*mut c_int, c_int, c_int, *mut btac1c_idxstate) -> c_int;

/// `psamp[(base - off) & 7]`, matching C's two's-complement `&` on a possibly
/// negative index (which always lands in `0..=7`).
#[inline(always)]
unsafe fn ps(psamp: *const c_int, base: c_int, off: c_int) -> c_int {
    let i = base.wrapping_sub(off) & 7;
    unsafe { *psamp.offset(i as isize) }
}

// ---------------------------------------------------------------------------
// BTAC1C2_PredictSample -- the generic switch-based predictor.
// ---------------------------------------------------------------------------

#[inline(never)]
unsafe extern "C" fn BTAC1C2_PredictSample(
    psamp: *mut c_int,
    idx: c_int,
    pfcn: c_int,
    ridx: *mut btac1c_idxstate,
) -> c_int {
    let i = idx;
    let pred: c_int;
    unsafe {
        match pfcn {
            0 => {
                pred = ps(psamp, i, 1);
            }
            1 => {
                pred = 2i32
                    .wrapping_mul(ps(psamp, i, 1))
                    .wrapping_sub(ps(psamp, i, 2));
            }
            2 => {
                pred = 3i32
                    .wrapping_mul(ps(psamp, i, 1))
                    .wrapping_sub(ps(psamp, i, 2))
                    >> 1;
            }
            3 => {
                pred = 5i32
                    .wrapping_mul(ps(psamp, i, 1))
                    .wrapping_sub(ps(psamp, i, 2))
                    >> 2;
            }
            4 => {
                let p0 = ps(psamp, i, 1).wrapping_add(ps(psamp, i, 2));
                let p1 = ps(psamp, i, 2).wrapping_add(ps(psamp, i, 3));
                pred = p0.wrapping_sub(p1 >> 1);
            }
            5 => {
                let p0 = ps(psamp, i, 1).wrapping_add(ps(psamp, i, 2));
                let p1 = ps(psamp, i, 2).wrapping_add(ps(psamp, i, 3));
                pred = 3i32.wrapping_mul(p0).wrapping_sub(p1) >> 2;
            }
            6 => {
                let p0 = ps(psamp, i, 1).wrapping_add(ps(psamp, i, 2));
                let p1 = ps(psamp, i, 2).wrapping_add(ps(psamp, i, 3));
                pred = 5i32.wrapping_mul(p0).wrapping_sub(p1) >> 3;
            }
            7 => {
                let acc = 18i32
                    .wrapping_mul(ps(psamp, i, 1))
                    .wrapping_sub(4i32.wrapping_mul(ps(psamp, i, 2)))
                    .wrapping_add(3i32.wrapping_mul(ps(psamp, i, 3)))
                    .wrapping_sub(2i32.wrapping_mul(ps(psamp, i, 4)))
                    .wrapping_add(ps(psamp, i, 5));
                pred = acc.wrapping_div(16);
            }
            8 => {
                let acc = 72i32
                    .wrapping_mul(ps(psamp, i, 1))
                    .wrapping_sub(16i32.wrapping_mul(ps(psamp, i, 2)))
                    .wrapping_add(12i32.wrapping_mul(ps(psamp, i, 3)))
                    .wrapping_sub(8i32.wrapping_mul(ps(psamp, i, 4)))
                    .wrapping_add(5i32.wrapping_mul(ps(psamp, i, 5)))
                    .wrapping_sub(3i32.wrapping_mul(ps(psamp, i, 6)))
                    .wrapping_add(3i32.wrapping_mul(ps(psamp, i, 7)))
                    .wrapping_sub(ps(psamp, i, 8));
                pred = acc.wrapping_div(64);
            }
            9 => {
                let acc = 76i32
                    .wrapping_mul(ps(psamp, i, 1))
                    .wrapping_sub(17i32.wrapping_mul(ps(psamp, i, 2)))
                    .wrapping_add(10i32.wrapping_mul(ps(psamp, i, 3)))
                    .wrapping_sub(7i32.wrapping_mul(ps(psamp, i, 4)))
                    .wrapping_add(5i32.wrapping_mul(ps(psamp, i, 5)))
                    .wrapping_sub(4i32.wrapping_mul(ps(psamp, i, 6)))
                    .wrapping_add(4i32.wrapping_mul(ps(psamp, i, 7)))
                    .wrapping_sub(3i32.wrapping_mul(ps(psamp, i, 8)));
                pred = acc.wrapping_div(64);
            }
            10 => {
                let p0 = ps(psamp, i, 1)
                    .wrapping_add(ps(psamp, i, 2))
                    .wrapping_add(ps(psamp, i, 3))
                    .wrapping_add(ps(psamp, i, 4));
                let p1 = ps(psamp, i, 5)
                    .wrapping_add(ps(psamp, i, 6))
                    .wrapping_add(ps(psamp, i, 7))
                    .wrapping_add(ps(psamp, i, 8));
                pred = 5i32.wrapping_mul(p0).wrapping_sub(p1) >> 4;
            }
            11 => {
                let p0 = ps(psamp, i, 1)
                    .wrapping_add(ps(psamp, i, 2))
                    .wrapping_add(ps(psamp, i, 3))
                    .wrapping_add(ps(psamp, i, 4));
                let p1 = ps(psamp, i, 5)
                    .wrapping_add(ps(psamp, i, 6))
                    .wrapping_add(ps(psamp, i, 7))
                    .wrapping_add(ps(psamp, i, 8));
                pred = p0.wrapping_add(p1) >> 3;
            }
            12 | 13 | 14 | 15 => {
                let row = &(*ridx).firfx[(pfcn - 12) as usize];
                let mut acc: c_int = 0;
                for k in 0..8usize {
                    acc = acc.wrapping_add(
                        (row[k] as c_int).wrapping_mul(ps(psamp, i, (k + 1) as c_int)),
                    );
                }
                pred = acc.wrapping_div(256);
            }
            _ => {
                pred = 0;
            }
        }
    }
    pred
}

// ---------------------------------------------------------------------------
// Specialized predictors. Each must stay a distinct symbol: `get_predict_func`
// compares addresses, so folding two of these together would change behavior.
// ---------------------------------------------------------------------------

#[inline(never)]
unsafe extern "C" fn BTAC1C2_PredictSample_Pfn0(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    unsafe { ps(psamp, idx, 1) }
}

#[inline(never)]
unsafe extern "C" fn BTAC1C2_PredictSample_Pfn1(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    unsafe {
        2i32.wrapping_mul(ps(psamp, idx, 1))
            .wrapping_sub(ps(psamp, idx, 2))
    }
}

#[inline(never)]
unsafe extern "C" fn BTAC1C2_PredictSample_Pfn2(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    unsafe {
        3i32.wrapping_mul(ps(psamp, idx, 1))
            .wrapping_sub(ps(psamp, idx, 2))
            >> 1
    }
}

#[inline(never)]
unsafe extern "C" fn BTAC1C2_PredictSample_Pfn3(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    unsafe {
        5i32.wrapping_mul(ps(psamp, idx, 1))
            .wrapping_sub(ps(psamp, idx, 2))
            >> 2
    }
}

#[inline(never)]
unsafe extern "C" fn BTAC1C2_PredictSample_Pfn4(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    unsafe {
        let p0 = ps(psamp, idx, 1).wrapping_add(ps(psamp, idx, 2));
        let p1 = ps(psamp, idx, 2).wrapping_add(ps(psamp, idx, 3));
        p0.wrapping_sub(p1 >> 1)
    }
}

#[inline(never)]
unsafe extern "C" fn BTAC1C2_PredictSample_Pfn5(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    unsafe {
        let p0 = ps(psamp, idx, 1).wrapping_add(ps(psamp, idx, 2));
        let p1 = ps(psamp, idx, 2).wrapping_add(ps(psamp, idx, 3));
        3i32.wrapping_mul(p0).wrapping_sub(p1) >> 2
    }
}

#[inline(never)]
unsafe extern "C" fn BTAC1C2_PredictSample_Pfn6(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    unsafe {
        let p0 = ps(psamp, idx, 1).wrapping_add(ps(psamp, idx, 2));
        let p1 = ps(psamp, idx, 2).wrapping_add(ps(psamp, idx, 3));
        5i32.wrapping_mul(p0).wrapping_sub(p1) >> 3
    }
}

#[inline(never)]
unsafe extern "C" fn BTAC1C2_PredictSample_Pfn7(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    unsafe {
        18i32
            .wrapping_mul(ps(psamp, idx, 1))
            .wrapping_sub(4i32.wrapping_mul(ps(psamp, idx, 2)))
            .wrapping_add(3i32.wrapping_mul(ps(psamp, idx, 3)))
            .wrapping_sub(2i32.wrapping_mul(ps(psamp, idx, 4)))
            .wrapping_add(ps(psamp, idx, 5))
            .wrapping_div(16)
    }
}

#[inline(never)]
unsafe extern "C" fn BTAC1C2_PredictSample_Pfn8(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    unsafe {
        72i32
            .wrapping_mul(ps(psamp, idx, 1))
            .wrapping_sub(16i32.wrapping_mul(ps(psamp, idx, 2)))
            .wrapping_add(12i32.wrapping_mul(ps(psamp, idx, 3)))
            .wrapping_sub(8i32.wrapping_mul(ps(psamp, idx, 4)))
            .wrapping_add(5i32.wrapping_mul(ps(psamp, idx, 5)))
            .wrapping_sub(3i32.wrapping_mul(ps(psamp, idx, 6)))
            .wrapping_add(3i32.wrapping_mul(ps(psamp, idx, 7)))
            .wrapping_sub(ps(psamp, idx, 8))
            .wrapping_div(64)
    }
}

#[inline(never)]
unsafe extern "C" fn BTAC1C2_PredictSample_Pfn9(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    unsafe {
        76i32
            .wrapping_mul(ps(psamp, idx, 1))
            .wrapping_sub(17i32.wrapping_mul(ps(psamp, idx, 2)))
            .wrapping_add(10i32.wrapping_mul(ps(psamp, idx, 3)))
            .wrapping_sub(7i32.wrapping_mul(ps(psamp, idx, 4)))
            .wrapping_add(5i32.wrapping_mul(ps(psamp, idx, 5)))
            .wrapping_sub(4i32.wrapping_mul(ps(psamp, idx, 6)))
            .wrapping_add(4i32.wrapping_mul(ps(psamp, idx, 7)))
            .wrapping_sub(3i32.wrapping_mul(ps(psamp, idx, 8)))
            .wrapping_div(64)
    }
}

// NOTE: the C `Pfn10` shifts by 3 while the `switch` arm for pfcn == 10 shifts
// by 4. Reproduced as-is.
#[inline(never)]
unsafe extern "C" fn BTAC1C2_PredictSample_Pfn10(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    unsafe {
        let p0 = ps(psamp, idx, 1)
            .wrapping_add(ps(psamp, idx, 2))
            .wrapping_add(ps(psamp, idx, 3))
            .wrapping_add(ps(psamp, idx, 4));
        let p1 = ps(psamp, idx, 5)
            .wrapping_add(ps(psamp, idx, 6))
            .wrapping_add(ps(psamp, idx, 7))
            .wrapping_add(ps(psamp, idx, 8));
        5i32.wrapping_mul(p0).wrapping_sub(p1) >> 3
    }
}

// NOTE: the C `Pfn11` shifts by 1 while the `switch` arm for pfcn == 11 shifts
// by 3. Reproduced as-is.
#[inline(never)]
unsafe extern "C" fn BTAC1C2_PredictSample_Pfn11(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    unsafe {
        let p0 = ps(psamp, idx, 1)
            .wrapping_add(ps(psamp, idx, 2))
            .wrapping_add(ps(psamp, idx, 3))
            .wrapping_add(ps(psamp, idx, 4));
        let p1 = ps(psamp, idx, 5)
            .wrapping_add(ps(psamp, idx, 6))
            .wrapping_add(ps(psamp, idx, 7))
            .wrapping_add(ps(psamp, idx, 8));
        p0.wrapping_add(p1) >> 1
    }
}

// ---------------------------------------------------------------------------
// Selector
// ---------------------------------------------------------------------------

#[inline(never)]
fn BTAC1C2_GetPredictFunc(pfcn: c_int) -> *const c_void {
    let fcn: PredictFn = match pfcn {
        0 => BTAC1C2_PredictSample_Pfn0,
        1 => BTAC1C2_PredictSample_Pfn1,
        2 => BTAC1C2_PredictSample_Pfn2,
        3 => BTAC1C2_PredictSample_Pfn3,
        4 => BTAC1C2_PredictSample_Pfn4,
        5 => BTAC1C2_PredictSample_Pfn5,
        6 => BTAC1C2_PredictSample_Pfn6,
        7 => BTAC1C2_PredictSample_Pfn7,
        8 => BTAC1C2_PredictSample_Pfn8,
        9 => BTAC1C2_PredictSample_Pfn9,
        10 => BTAC1C2_PredictSample_Pfn10,
        11 => BTAC1C2_PredictSample_Pfn11,
        _ => BTAC1C2_PredictSample,
    };
    fcn as *const c_void
}

#[unsafe(no_mangle)]
pub extern "C" fn get_predict_func(pfcn: c_int) -> c_int {
    let mut result: c_int = 0;
    let fcn = BTAC1C2_GetPredictFunc(pfcn);
    match pfcn {
        0 => result = (fcn == BTAC1C2_PredictSample_Pfn0 as *const c_void) as c_int,
        1 => result = (fcn == BTAC1C2_PredictSample_Pfn1 as *const c_void) as c_int,
        2 => result = (fcn == BTAC1C2_PredictSample_Pfn2 as *const c_void) as c_int,
        3 => result = (fcn == BTAC1C2_PredictSample_Pfn3 as *const c_void) as c_int,
        4 => result = (fcn == BTAC1C2_PredictSample_Pfn4 as *const c_void) as c_int,
        5 => result = (fcn == BTAC1C2_PredictSample_Pfn5 as *const c_void) as c_int,
        6 => result = (fcn == BTAC1C2_PredictSample_Pfn6 as *const c_void) as c_int,
        7 => result = (fcn == BTAC1C2_PredictSample_Pfn7 as *const c_void) as c_int,
        8 => result = (fcn == BTAC1C2_PredictSample_Pfn8 as *const c_void) as c_int,
        9 => result = (fcn == BTAC1C2_PredictSample_Pfn9 as *const c_void) as c_int,
        10 => result = (fcn == BTAC1C2_PredictSample_Pfn10 as *const c_void) as c_int,
        11 => result = (fcn == BTAC1C2_PredictSample_Pfn11 as *const c_void) as c_int,
        _ => {}
    }
    result
}

// ---------------------------------------------------------------------------
// Differential tests against the C source.
//
// `c_src/src/lib.c` keeps every predictor `static`, so the only symbol the real
// C shared library exports is `get_predict_func` (covered by the integration
// test in `tests/`, which drives both `.so` files through `libloading`).
//
// The predictor arithmetic has no exported entry point in either library, so it
// is compared here against an auxiliary C harness (`tests/c_harness/harness.c`)
// that `#include`s the untouched C source and publishes wrappers. This module is
// `#[cfg(test)]`, so it contributes nothing to the shipped `cdylib` and leaves
// its exported symbol set identical to the C library's.
// ---------------------------------------------------------------------------
#[cfg(test)]
mod c_differential {
    use super::*;
    use libloading::{Library, Symbol};
    use std::path::PathBuf;
    use std::process::Command;
    use std::sync::OnceLock;

    type PredictAbi = unsafe extern "C" fn(*mut c_int, c_int, c_int, *mut btac1c_idxstate) -> c_int;
    type IntFn = unsafe extern "C" fn() -> c_int;
    type IdFn = unsafe extern "C" fn(c_int) -> c_int;

    /// Build (once) and return the path to the auxiliary C harness.
    fn harness_path() -> &'static PathBuf {
        static ONCE: OnceLock<PathBuf> = OnceLock::new();
        ONCE.get_or_init(|| {
            let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            let root = manifest.parent().expect("workspace root").to_path_buf();
            let c_src = root.join("c_src");
            let out_dir = manifest.join("target").join("c_harness");
            std::fs::create_dir_all(&out_dir).expect("create harness out dir");
            let out = out_dir.join("libharness.so");

            let cc = std::env::var("CC").unwrap_or_else(|_| "cc".to_string());
            let status = Command::new(cc)
                .arg("-shared")
                .arg("-fPIC")
                .arg("-O2")
                .arg("-I")
                .arg(c_src.join("include"))
                .arg("-I")
                .arg(c_src.join("src"))
                .arg("-o")
                .arg(&out)
                .arg(manifest.join("tests").join("c_harness").join("harness.c"))
                .output()
                .expect("spawn C compiler for harness");
            assert!(
                status.status.success(),
                "failed to build C harness:\n{}\n{}",
                String::from_utf8_lossy(&status.stdout),
                String::from_utf8_lossy(&status.stderr),
            );
            out
        })
    }

    fn harness() -> &'static Library {
        static ONCE: OnceLock<Library> = OnceLock::new();
        ONCE.get_or_init(|| unsafe {
            Library::new(harness_path()).expect("load C harness .so")
        })
    }

    fn sym<T: Copy>(name: &str) -> T {
        let lib = harness();
        unsafe {
            let s: Symbol<T> = lib
                .get(name.as_bytes())
                .unwrap_or_else(|e| panic!("missing harness symbol {name}: {e}"));
            *s
        }
    }

    /// The 12 specialized predictors, paired C-side wrapper name with the Rust
    /// function it was translated from.
    fn pairs() -> Vec<(&'static str, PredictAbi)> {
        vec![
            ("harness_predict_pfn0\0", BTAC1C2_PredictSample_Pfn0 as PredictAbi),
            ("harness_predict_pfn1\0", BTAC1C2_PredictSample_Pfn1 as PredictAbi),
            ("harness_predict_pfn2\0", BTAC1C2_PredictSample_Pfn2 as PredictAbi),
            ("harness_predict_pfn3\0", BTAC1C2_PredictSample_Pfn3 as PredictAbi),
            ("harness_predict_pfn4\0", BTAC1C2_PredictSample_Pfn4 as PredictAbi),
            ("harness_predict_pfn5\0", BTAC1C2_PredictSample_Pfn5 as PredictAbi),
            ("harness_predict_pfn6\0", BTAC1C2_PredictSample_Pfn6 as PredictAbi),
            ("harness_predict_pfn7\0", BTAC1C2_PredictSample_Pfn7 as PredictAbi),
            ("harness_predict_pfn8\0", BTAC1C2_PredictSample_Pfn8 as PredictAbi),
            ("harness_predict_pfn9\0", BTAC1C2_PredictSample_Pfn9 as PredictAbi),
            ("harness_predict_pfn10\0", BTAC1C2_PredictSample_Pfn10 as PredictAbi),
            ("harness_predict_pfn11\0", BTAC1C2_PredictSample_Pfn11 as PredictAbi),
        ]
    }

    /// Small deterministic PRNG so the sweep is reproducible.
    struct Rng(u64);
    impl Rng {
        fn next_u32(&mut self) -> u32 {
            // xorshift64*
            let mut x = self.0;
            x ^= x >> 12;
            x ^= x << 25;
            x ^= x >> 27;
            self.0 = x;
            (x.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 32) as u32
        }
        /// Value in `-bound ..= bound`.
        fn sample(&mut self, bound: i32) -> c_int {
            let span = (bound as i64) * 2 + 1;
            ((self.next_u32() as i64 % span) - bound as i64) as c_int
        }
    }

    /// Coefficient magnitudes are bounded by 120 (predictor 8) and 256 (the FIR
    /// arms), so keeping samples within this range guarantees the C side never
    /// signed-overflows -- which would be undefined behaviour and make any
    /// comparison meaningless.
    const SAFE_BOUND: i32 = 1 << 20;

    fn fresh_state(seed: u64) -> btac1c_idxstate {
        let mut rng = Rng(seed);
        let mut firfx = [[0i16; 8]; 4];
        for row in firfx.iter_mut() {
            for c in row.iter_mut() {
                *c = rng.sample(i16::MAX as i32 / 2) as i16;
            }
        }
        btac1c_idxstate {
            idx: rng.next_u32() as u16,
            lpred: rng.sample(i16::MAX as i32) as i16,
            rpred: rng.sample(i16::MAX as i32) as i16,
            tag: rng.next_u32() as u8,
            bcfcn: rng.next_u32() as u8,
            bsfcn: rng.next_u32() as u8,
            usefx: rng.next_u32() as u8,
            firfx,
        }
    }

    #[test]
    fn struct_layout_matches_c() {
        let size: IntFn = sym("harness_idxstate_size\0");
        let align: IntFn = sym("harness_idxstate_align\0");
        let firfx_off: IntFn = sym("harness_idxstate_firfx_offset\0");
        unsafe {
            assert_eq!(
                size() as usize,
                std::mem::size_of::<btac1c_idxstate>(),
                "sizeof(btac1c_idxstate) differs"
            );
            assert_eq!(
                align() as usize,
                std::mem::align_of::<btac1c_idxstate>(),
                "alignof(btac1c_idxstate) differs"
            );
            let rust_off = {
                let s = fresh_state(1);
                (&s.firfx as *const _ as usize) - (&s as *const _ as usize)
            };
            assert_eq!(firfx_off() as usize, rust_off, "offsetof(firfx) differs");
        }
    }

    /// Every specialized predictor, over a randomized sweep of sample buffers
    /// and `idx` values (including negative `idx`, which C masks with `& 7`).
    #[test]
    fn specialized_predictors_match() {
        let cases = pairs();
        let mut rng = Rng(0x1234_5678_9abc_def0);
        for (name, rust_fn) in &cases {
            let c_fn: PredictAbi = sym(name);
            for trial in 0..2000 {
                let mut psamp: [c_int; 8] = [0; 8];
                let bound = match trial % 4 {
                    0 => 1,
                    1 => i16::MAX as i32,
                    2 => SAFE_BOUND,
                    _ => 1 << 10,
                };
                for s in psamp.iter_mut() {
                    *s = rng.sample(bound);
                }
                let idx = rng.sample(1 << 20);
                let mut st = fresh_state(trial as u64 + 7);
                let mut a = psamp;
                let mut b = psamp;
                let c = unsafe { c_fn(a.as_mut_ptr(), idx, 0, &mut st) };
                let r = unsafe { rust_fn(b.as_mut_ptr(), idx, 0, &mut st) };
                assert_eq!(
                    c, r,
                    "{} mismatch: psamp={:?} idx={} -> C {} vs Rust {}",
                    name.trim_end_matches('\0'),
                    psamp,
                    idx,
                    c,
                    r
                );
                assert_eq!(a, b, "predictor must not write to psamp");
            }
        }
    }

    /// Hand-picked edge inputs: all-zero, constant, alternating, and monotonic
    /// buffers with `idx` walking every residue class mod 8.
    #[test]
    fn specialized_predictors_match_on_edge_inputs() {
        let cases = pairs();
        let buffers: Vec<[c_int; 8]> = vec![
            [0; 8],
            [1; 8],
            [-1; 8],
            [SAFE_BOUND; 8],
            [-SAFE_BOUND; 8],
            [0, 1, 2, 3, 4, 5, 6, 7],
            [7, 6, 5, 4, 3, 2, 1, 0],
            [1, -1, 1, -1, 1, -1, 1, -1],
            [-1, 1, -1, 1, -1, 1, -1, 1],
            [i16::MAX as c_int; 8],
            [i16::MIN as c_int; 8],
            [SAFE_BOUND, -SAFE_BOUND, SAFE_BOUND, -SAFE_BOUND, 0, 0, 1, -1],
            [3, 3, 3, 3, -3, -3, -3, -3],
            [1, 2, 4, 8, 16, 32, 64, 128],
        ];
        let idxs: Vec<c_int> = (-16..=16).chain([-1_000_001, 1_000_001]).collect();
        for (name, rust_fn) in &cases {
            let c_fn: PredictAbi = sym(name);
            let mut st = fresh_state(99);
            for buf in &buffers {
                for &idx in &idxs {
                    let mut a = *buf;
                    let mut b = *buf;
                    let c = unsafe { c_fn(a.as_mut_ptr(), idx, 0, &mut st) };
                    let r = unsafe { rust_fn(b.as_mut_ptr(), idx, 0, &mut st) };
                    assert_eq!(
                        c, r,
                        "{} mismatch: psamp={:?} idx={} -> C {} vs Rust {}",
                        name.trim_end_matches('\0'),
                        buf,
                        idx,
                        c,
                        r
                    );
                }
            }
        }
    }

    /// The generic switch-based `BTAC1C2_PredictSample`, across every `pfcn`
    /// arm including the FIR arms (12..=15) and the `default` fallthrough.
    #[test]
    fn generic_predictor_matches() {
        let c_fn: PredictAbi = sym("harness_predict_generic\0");
        let mut rng = Rng(0x0fed_cba9_8765_4321);
        let pfcns: Vec<c_int> = (-4..=20)
            .chain([-1000, 1000, i32::MIN, i32::MAX])
            .collect();
        for &pfcn in &pfcns {
            for trial in 0..400 {
                let mut psamp: [c_int; 8] = [0; 8];
                let bound = match trial % 4 {
                    0 => 1,
                    1 => i16::MAX as i32,
                    2 => SAFE_BOUND,
                    _ => 1 << 12,
                };
                for s in psamp.iter_mut() {
                    *s = rng.sample(bound);
                }
                let idx = rng.sample(1 << 20);
                let mut st = fresh_state(trial as u64 * 31 + 5);
                let mut a = psamp;
                let mut b = psamp;
                let c = unsafe { c_fn(a.as_mut_ptr(), idx, pfcn, &mut st) };
                let r = unsafe { BTAC1C2_PredictSample(b.as_mut_ptr(), idx, pfcn, &mut st) };
                assert_eq!(
                    c, r,
                    "generic predictor mismatch: pfcn={} psamp={:?} idx={} firfx={:?} -> C {} vs Rust {}",
                    pfcn, psamp, idx, st.firfx, c, r
                );
            }
        }
    }

    /// The FIR arms read `ridx->firfx[pfcn - 12]`; exercise them with extreme
    /// `i16` coefficients as well as randomized ones.
    #[test]
    fn generic_predictor_fir_arms_match() {
        let c_fn: PredictAbi = sym("harness_predict_generic\0");
        let coeff_sets: [[i16; 8]; 6] = [
            [0; 8],
            [1; 8],
            [-1; 8],
            [i16::MAX; 8],
            [i16::MIN; 8],
            [i16::MAX, i16::MIN, 1, -1, 0, 256, -256, 32767],
        ];
        // 8 * 32767 * bound must stay inside i32: bound <= 8191.
        let buffers: [[c_int; 8]; 5] = [
            [0; 8],
            [1, -1, 1, -1, 1, -1, 1, -1],
            [8191; 8],
            [-8191; 8],
            [8191, -8191, 4096, -4096, 1, -1, 0, 8190],
        ];
        for pfcn in 12..=15 {
            for coeffs in &coeff_sets {
                let mut st = fresh_state(3);
                for row in st.firfx.iter_mut() {
                    *row = *coeffs;
                }
                for buf in &buffers {
                    for idx in -9..=9 {
                        let mut a = *buf;
                        let mut b = *buf;
                        let c = unsafe { c_fn(a.as_mut_ptr(), idx, pfcn, &mut st) };
                        let r =
                            unsafe { BTAC1C2_PredictSample(b.as_mut_ptr(), idx, pfcn, &mut st) };
                        assert_eq!(
                            c, r,
                            "FIR arm mismatch: pfcn={pfcn} coeffs={coeffs:?} psamp={buf:?} idx={idx} -> C {c} vs Rust {r}"
                        );
                    }
                }
            }
        }
    }

    /// Each FIR arm must read its *own* row of `firfx`, so give the four rows
    /// distinguishable coefficients and confirm the results differ per arm in
    /// the same way on both sides.
    #[test]
    fn generic_predictor_fir_row_selection_matches() {
        let c_fn: PredictAbi = sym("harness_predict_generic\0");
        let mut st = fresh_state(11);
        for (row_i, row) in st.firfx.iter_mut().enumerate() {
            for (k, coeff) in row.iter_mut().enumerate() {
                *coeff = ((row_i + 1) * 100 + k) as i16;
            }
        }
        let buf: [c_int; 8] = [5, -7, 11, -13, 17, -19, 23, -29];
        for pfcn in 12..=15 {
            for idx in 0..8 {
                let mut a = buf;
                let mut b = buf;
                let c = unsafe { c_fn(a.as_mut_ptr(), idx, pfcn, &mut st) };
                let r = unsafe { BTAC1C2_PredictSample(b.as_mut_ptr(), idx, pfcn, &mut st) };
                assert_eq!(c, r, "FIR row selection mismatch at pfcn={pfcn} idx={idx}");
            }
        }
    }

    /// `BTAC1C2_GetPredictFunc` must select the same specialization on both
    /// sides. The C harness reports an id; the Rust side is identified by
    /// comparing the returned pointer against each candidate.
    #[test]
    fn selector_chooses_same_function() {
        let c_id: IdFn = sym("harness_selector_id\0");
        for pfcn in -8..=32 {
            let expect = unsafe { c_id(pfcn) };
            let got = rust_selector_id(pfcn);
            assert_eq!(
                expect, got,
                "selector mismatch for pfcn={pfcn}: C id {expect} vs Rust id {got}"
            );
        }
        for pfcn in [i32::MIN, i32::MIN + 1, -1000, 1000, i32::MAX - 1, i32::MAX] {
            let expect = unsafe { c_id(pfcn) };
            let got = rust_selector_id(pfcn);
            assert_eq!(expect, got, "selector mismatch for pfcn={pfcn}");
        }
    }

    fn rust_selector_id(pfcn: c_int) -> c_int {
        let f = BTAC1C2_GetPredictFunc(pfcn);
        let candidates: [(c_int, PredictAbi); 13] = [
            (0, BTAC1C2_PredictSample_Pfn0),
            (1, BTAC1C2_PredictSample_Pfn1),
            (2, BTAC1C2_PredictSample_Pfn2),
            (3, BTAC1C2_PredictSample_Pfn3),
            (4, BTAC1C2_PredictSample_Pfn4),
            (5, BTAC1C2_PredictSample_Pfn5),
            (6, BTAC1C2_PredictSample_Pfn6),
            (7, BTAC1C2_PredictSample_Pfn7),
            (8, BTAC1C2_PredictSample_Pfn8),
            (9, BTAC1C2_PredictSample_Pfn9),
            (10, BTAC1C2_PredictSample_Pfn10),
            (11, BTAC1C2_PredictSample_Pfn11),
            (100, BTAC1C2_PredictSample),
        ];
        for (id, cand) in candidates {
            if f == cand as *const c_void {
                return id;
            }
        }
        -1
    }

    /// Calling through the pointer the selector returned must agree end to end.
    #[test]
    fn calling_selected_function_matches() {
        let c_fn: PredictAbi = sym("harness_call_selected\0");
        let mut rng = Rng(0xdead_beef_cafe_babe);
        for pfcn in -4..=20 {
            for trial in 0..200 {
                let mut psamp: [c_int; 8] = [0; 8];
                for s in psamp.iter_mut() {
                    *s = rng.sample(if trial % 2 == 0 { 8191 } else { i16::MAX as i32 });
                }
                let idx = rng.sample(1 << 16);
                let mut st = fresh_state(trial as u64 + 1234);
                for row in st.firfx.iter_mut() {
                    for c in row.iter_mut() {
                        *c = (rng.sample(4096)) as i16;
                    }
                }
                let mut a = psamp;
                let mut b = psamp;
                let c = unsafe { c_fn(a.as_mut_ptr(), idx, pfcn, &mut st) };
                let rust_fn: PredictAbi = unsafe {
                    std::mem::transmute::<*const c_void, PredictAbi>(BTAC1C2_GetPredictFunc(pfcn))
                };
                let r = unsafe { rust_fn(b.as_mut_ptr(), idx, pfcn, &mut st) };
                assert_eq!(
                    c, r,
                    "selected-call mismatch: pfcn={pfcn} psamp={psamp:?} idx={idx} -> C {c} vs Rust {r}"
                );
            }
        }
    }

    /// The C source deliberately disagrees with itself: `Pfn10` shifts by 3
    /// where the `switch` arm for 10 shifts by 4, and `Pfn11` shifts by 1 where
    /// the arm for 11 shifts by 3. Pin that down so a future "cleanup" of the
    /// translation cannot silently normalize it.
    #[test]
    fn specialization_and_switch_arm_disagree_identically() {
        let generic: PredictAbi = sym("harness_predict_generic\0");
        let pfn10: PredictAbi = sym("harness_predict_pfn10\0");
        let pfn11: PredictAbi = sym("harness_predict_pfn11\0");
        let buf: [c_int; 8] = [16, 32, 48, 64, 80, 96, 112, 128];
        let mut st = fresh_state(5);
        for idx in 0..8 {
            for (arm, spec, rust_spec) in [
                (10, pfn10, BTAC1C2_PredictSample_Pfn10 as PredictAbi),
                (11, pfn11, BTAC1C2_PredictSample_Pfn11 as PredictAbi),
            ] {
                let mut a = buf;
                let mut b = buf;
                let mut d = buf;
                let c_generic = unsafe { generic(a.as_mut_ptr(), idx, arm, &mut st) };
                let c_spec = unsafe { spec(b.as_mut_ptr(), idx, arm, &mut st) };
                let r_generic =
                    unsafe { BTAC1C2_PredictSample(d.as_mut_ptr(), idx, arm, &mut st) };
                let mut e = buf;
                let r_spec = unsafe { rust_spec(e.as_mut_ptr(), idx, arm, &mut st) };
                assert_eq!(c_generic, r_generic, "generic arm {arm} at idx={idx}");
                assert_eq!(c_spec, r_spec, "specialization {arm} at idx={idx}");
                assert_ne!(
                    c_generic, c_spec,
                    "expected the C switch arm {arm} and its specialization to disagree"
                );
            }
        }
    }
}
