//! Differential tests: C `.so` vs Rust `.so`, both loaded with `libloading`.
//!
//! Nothing here calls a Rust function directly. Every call — including the ones
//! into the Rust crate — goes through `dlopen`/`dlsym` on a shared object, so
//! the `#[no_mangle] extern "C"` export wrappers are under test too.
//!
//! Row numbers in test names refer to `CONFIGS.md` (valid paths, Phase B) and
//! `ERRORS.md` (rejections, Phase C).
//!
//! Objects loaded:
//!   * `c_src/build/libtranslated_rust.so`   — the real C library (public ABI)
//!   * `target/diffshim/libcshim.so`         — C shim, `#include`s the untouched
//!                                             `c_src/src/lib.c` to reach its
//!                                             14 `static` routines
//!   * `target/difftest/debug/libget_predict_func_lib.so` — the Rust crate
//!
//! The shim pair only exists under the `diff_internals` feature; those tests
//! are `#[cfg]`-gated and skipped in the default configuration.

#![allow(clippy::too_many_arguments)]
// Much of the support layer (generators, the IdxState mirror, the shim paths)
// is only reachable from the `diff_internals`-gated modules, so in the default
// configuration those items are legitimately unused.
#![allow(dead_code)]

use libloading::Library;
use std::ffi::c_int;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

// ===========================================================================
// struct btac1c_idxstate_s — must mirror c_src/src/lib.c exactly.
// Row 19 proves this layout agrees with the C compiler's.
// ===========================================================================

#[repr(C)]
#[derive(Clone, Copy)]
struct IdxState {
    idx: u16,
    lpred: i16,
    rpred: i16,
    tag: u8,
    bcfcn: u8,
    bsfcn: u8,
    usefx: u8,
    firfx: [[i16; 8]; 4],
}

impl IdxState {
    fn zeroed() -> Self {
        IdxState {
            idx: 0,
            lpred: 0,
            rpred: 0,
            tag: 0,
            bcfcn: 0,
            bsfcn: 0,
            usefx: 0,
            firfx: [[0; 8]; 4],
        }
    }
}

// ===========================================================================
// Build the three shared objects (once per test binary).
// ===========================================================================

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn run(cmd: &mut Command, what: &str) {
    let out = cmd
        .output()
        .unwrap_or_else(|e| panic!("failed to spawn {what}: {e}"));
    if !out.status.success() {
        panic!(
            "{what} failed ({}):\n--- stdout ---\n{}\n--- stderr ---\n{}",
            out.status,
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
    }
}

struct Artifacts {
    c_main: PathBuf,
    c_shim: PathBuf,
    rust: PathBuf,
}

fn artifacts() -> &'static Artifacts {
    static A: OnceLock<Artifacts> = OnceLock::new();
    A.get_or_init(|| {
        let root = manifest_dir();

        // --- 1. the real C library, via CMake, exactly as documented ---------
        let c_build = root.join("c_src/build");
        let c_main = c_build.join("libtranslated_rust.so");
        if !c_main.exists() {
            std::fs::create_dir_all(&c_build).expect("mkdir c_src/build");
            run(
                Command::new("cmake")
                    .current_dir(&c_build)
                    .arg("..")
                    .arg("-DCMAKE_POSITION_INDEPENDENT_CODE=ON"),
                "cmake configure",
            );
            run(
                Command::new("cmake")
                    .current_dir(&c_build)
                    .args(["--build", "."]),
                "cmake build",
            );
        }
        assert!(c_main.exists(), "C library missing: {}", c_main.display());

        // --- 2. the C shim ---------------------------------------------------
        // No -O flag, matching CMake's default (empty CMAKE_BUILD_TYPE), so
        // signed-overflow behaviour is identical to the library above.
        let shim_dir = root.join("target/diffshim");
        std::fs::create_dir_all(&shim_dir).expect("mkdir target/diffshim");
        let c_shim = shim_dir.join("libcshim.so");
        run(
            Command::new("gcc").current_dir(&root).args([
                "-shared",
                "-fPIC",
                "-I",
                "c_src/include",
                "-o",
                c_shim.to_str().unwrap(),
                "tests/cshim/cshim.c",
            ]),
            "gcc (C shim)",
        );

        // --- 3. the Rust cdylib ---------------------------------------------
        // `cargo test` does not build cdylib targets, so build it here. A
        // separate --target-dir keeps this from deadlocking on the target lock
        // held by the outer `cargo test`.
        let rs_target = root.join("target/difftest");
        let mut c = Command::new(env!("CARGO"));
        c.current_dir(&root)
            .arg("build")
            .arg("--offline")
            .arg("--no-default-features")
            .arg("--target-dir")
            .arg(&rs_target);
        if cfg!(feature = "diff_internals") {
            c.arg("--features").arg("diff_internals");
        }
        run(&mut c, "cargo build (rust cdylib)");
        let rust = rs_target.join("debug/libget_predict_func_lib.so");
        assert!(rust.exists(), "Rust cdylib missing: {}", rust.display());

        Artifacts {
            c_main,
            c_shim,
            rust,
        }
    })
}

fn load(p: &Path) -> Library {
    unsafe { Library::new(p) }.unwrap_or_else(|e| panic!("dlopen {}: {e}", p.display()))
}

// ===========================================================================
// Public ABI pair: get_predict_func from the C .so and from the Rust .so.
// ===========================================================================

type GetPredictFunc = unsafe extern "C" fn(c_int) -> c_int;

struct PublicPair {
    _c: Library,
    _rs: Library,
    c: GetPredictFunc,
    rs: GetPredictFunc,
}

impl PublicPair {
    fn new() -> Self {
        let a = artifacts();
        let c_lib = load(&a.c_main);
        let rs_lib = load(&a.rust);
        let c = unsafe { *c_lib.get::<GetPredictFunc>(b"get_predict_func\0").unwrap() };
        let rs = unsafe { *rs_lib.get::<GetPredictFunc>(b"get_predict_func\0").unwrap() };
        PublicPair {
            _c: c_lib,
            _rs: rs_lib,
            c,
            rs,
        }
    }

    /// Call both and assert byte-identical results.
    fn check(&self, pfcn: c_int) -> c_int {
        let cv = unsafe { (self.c)(pfcn) };
        let rv = unsafe { (self.rs)(pfcn) };
        assert_eq!(
            cv, rv,
            "get_predict_func({pfcn}) mismatch: C returned {cv}, Rust returned {rv}"
        );
        cv
    }
}

fn public() -> &'static PublicPair {
    static P: OnceLock<PublicPair> = OnceLock::new();
    P.get_or_init(PublicPair::new)
}
// Library handles are only ever read from here on.
unsafe impl Sync for PublicPair {}
unsafe impl Send for PublicPair {}

// ===========================================================================
// Deterministic PRNG (SplitMix64) — fixed seed, reproducible.
// ===========================================================================

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed)
    }
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    /// Uniform-ish in `0..n`.
    fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
}

/// The seed every row starts from (rows offset it so they explore different
/// streams while staying reproducible).
const SEED: u64 = 0x5DEE_CE66_D9E3_779B;

fn rng_for(row: u64) -> Rng {
    Rng::new(SEED ^ row.wrapping_mul(0x9E37_79B9_7F4A_7C15))
}

/// Iterations per randomized row.
const ITERS: u32 = 512;

// ===========================================================================
// Input generators — one per axis in CONFIGS.md.
// ===========================================================================

/// Number of A4 (`psamp` contents) tiers.
const PSAMP_TIERS: u32 = 8;

/// A4: `psamp[]` contents.
///
/// * 0 — all zero (degenerate)
/// * 1 — small mixed signs
/// * 2 — moderate mixed signs
/// * 3 — "safe large": `|v| <= 2^20`, so no intermediate product/sum can
///       overflow (max coefficient magnitude sum is 126)
/// * 4 — full `i32` range: `72*x` etc. *do* overflow
/// * 5 — boundary constants (`INT_MIN`, `INT_MAX`, `0`, `±1`, …)
/// * 6 — all negative (arithmetic `>>` vs truncating `/` diverge here)
/// * 7 — negative and deliberately *not* divisible by 16/64/256
fn gen_psamp(rng: &mut Rng, tier: u32) -> [c_int; 8] {
    const BOUNDS: [i32; 8] = [
        i32::MIN,
        i32::MAX,
        0,
        -1,
        1,
        i32::MIN + 1,
        i32::MAX - 1,
        2,
    ];
    let mut a = [0 as c_int; 8];
    for slot in a.iter_mut() {
        *slot = match tier {
            0 => 0,
            1 => rng.below(201) as i32 - 100,
            2 => rng.next_i32() % 100_001,
            3 => rng.next_i32() >> 11,
            4 => rng.next_i32(),
            5 => BOUNDS[rng.below(8) as usize],
            6 => -(rng.below(1_000_000) as i32) - 1,
            _ => -((rng.below(4096) as i32) * 256 + 1 + rng.below(255) as i32),
        };
    }
    a
}

/// Number of A3 (`idx`) variants.
const IDX_VARIANTS: u32 = 6;

/// A3: `idx`. Only `(idx - n) & 7` is ever used, so the interesting cases are
/// in-range, negative, and values where `idx - n` overflows `int`.
fn gen_idx(rng: &mut Rng, variant: u32) -> c_int {
    match variant {
        0 => rng.below(8) as i32,
        1 => -(rng.below(16) as i32) - 1,
        2 => i32::MIN.wrapping_add(rng.below(9) as i32),
        3 => i32::MAX - rng.below(9) as i32,
        4 => rng.next_i32(),
        _ => rng.below(65_536) as i32,
    }
}

/// Number of A5 (`firfx`) tiers.
const FIRFX_TIERS: u32 = 4;

/// A5: `ridx->firfx`. Every row is filled so row-selection bugs (A6) show up.
fn gen_firfx(rng: &mut Rng, tier: u32) -> [[i16; 8]; 4] {
    let mut f = [[0i16; 8]; 4];
    for row in f.iter_mut() {
        for v in row.iter_mut() {
            *v = match tier {
                0 => 0,
                1 => rng.below(513) as i16 - 256,
                2 => rng.next_u32() as u16 as i16,
                _ => {
                    if rng.next_u32() & 1 == 0 {
                        i16::MIN
                    } else {
                        i16::MAX
                    }
                }
            };
        }
    }
    f
}

// ===========================================================================
// PHASE B — rows 1..14: the public entry point `get_predict_func`.
// ===========================================================================

/// Rows 1-12: each of the twelve valid `case` arms returns 1 on both sides.
macro_rules! public_row {
    ($name:ident, $row:expr, $pfcn:expr) => {
        #[test]
        fn $name() {
            let p = public();
            let got = p.check($pfcn);
            // The C selects _Pfn$pfcn and then compares against _Pfn$pfcn, so
            // the pointer identity holds and the result is 1.
            assert_eq!(
                got, 1,
                "CONFIGS row {}: get_predict_func({}) should be 1",
                $row, $pfcn
            );
        }
    };
}

public_row!(row01_get_predict_func_pfcn0, 1, 0);
public_row!(row02_get_predict_func_pfcn1, 2, 1);
public_row!(row03_get_predict_func_pfcn2, 3, 2);
public_row!(row04_get_predict_func_pfcn3, 4, 3);
public_row!(row05_get_predict_func_pfcn4, 5, 4);
public_row!(row06_get_predict_func_pfcn5, 6, 5);
public_row!(row07_get_predict_func_pfcn6, 7, 6);
public_row!(row08_get_predict_func_pfcn7, 8, 7);
public_row!(row09_get_predict_func_pfcn8, 9, 8);
public_row!(row10_get_predict_func_pfcn9, 10, 9);
public_row!(row11_get_predict_func_pfcn10, 11, 10);
public_row!(row12_get_predict_func_pfcn11, 12, 11);

/// Row 13: exhaustive sweep of the whole near range, both boundaries included.
#[test]
fn row13_get_predict_func_exhaustive_near_range() {
    let p = public();
    for pfcn in -4096..=4096 {
        let got = p.check(pfcn);
        let want = if (0..=11).contains(&pfcn) { 1 } else { 0 };
        assert_eq!(got, want, "CONFIGS row 13: get_predict_func({pfcn})");
    }
}

/// Row 14: randomized over the full `i32` range.
#[test]
fn row14_get_predict_func_randomized_full_i32() {
    let p = public();
    let mut rng = rng_for(14);
    for _ in 0..100_000 {
        let pfcn = rng.next_i32();
        let got = p.check(pfcn);
        let want = if (0..=11).contains(&pfcn) { 1 } else { 0 };
        assert_eq!(got, want, "CONFIGS row 14: get_predict_func({pfcn})");
    }
}

// ===========================================================================
// PHASE C — ERRORS.md rows 3..8 reachable through the public ABI.
// (Rows 1 and 2 need the internal surface; see the gated section below.)
// ===========================================================================

/// ERRORS row 3: `pfcn` outside 0..=11 hits `default:`, so `result` keeps its
/// initial 0 and no pointer comparison happens.
#[test]
fn row3_get_predict_func_default_arm() {
    let p = public();
    for pfcn in [16, 17, 100, -5, -100, 1_000_000, -1_000_000] {
        let got = p.check(pfcn);
        assert_eq!(got, 0, "ERRORS row 3: get_predict_func({pfcn}) must be 0");
    }
}

/// ERRORS row 4: one step below the valid range.
#[test]
fn row4_minus_one_boundary() {
    let got = public().check(-1);
    assert_eq!(got, 0, "ERRORS row 4: get_predict_func(-1) must be 0");
}

/// ERRORS row 5: one step past the valid range. 12 is a real `case` in
/// `BTAC1C2_PredictSample` but *not* in `BTAC1C2_GetPredictFunc`, so it must
/// still be rejected by `get_predict_func`.
#[test]
fn row5_twelve_boundary() {
    let got = public().check(12);
    assert_eq!(got, 0, "ERRORS row 5: get_predict_func(12) must be 0");
}

/// ERRORS row 6: the remaining `BTAC1C2_PredictSample`-only cases.
#[test]
fn row6_thirteen_to_fifteen() {
    let p = public();
    for pfcn in 13..=15 {
        let got = p.check(pfcn);
        assert_eq!(got, 0, "ERRORS row 6: get_predict_func({pfcn}) must be 0");
    }
}

/// ERRORS row 7.
#[test]
fn row7_int_min() {
    let got = public().check(i32::MIN);
    assert_eq!(got, 0, "ERRORS row 7: get_predict_func(INT_MIN) must be 0");
}

/// ERRORS row 8.
#[test]
fn row8_int_max() {
    let got = public().check(i32::MAX);
    assert_eq!(got, 0, "ERRORS row 8: get_predict_func(INT_MAX) must be 0");
}

/// Generic boundary: `pfcn` is an `int`, so out-of-range "enum" values are
/// ordinary inputs. Sweep the near band exhaustively and the rest randomly.
#[test]
fn generic_out_of_range_enum_values_full_i32() {
    let p = public();
    for pfcn in -4096..=4096 {
        if !(0..=11).contains(&pfcn) {
            assert_eq!(p.check(pfcn), 0, "out-of-range pfcn {pfcn} must yield 0");
        }
    }
    let mut rng = rng_for(9001);
    for _ in 0..50_000 {
        let pfcn = rng.next_i32();
        if !(0..=11).contains(&pfcn) {
            assert_eq!(p.check(pfcn), 0, "out-of-range pfcn {pfcn} must yield 0");
        }
    }
}

/// Generic boundary: the extremes of the argument's type.
#[test]
fn generic_extreme_values() {
    let p = public();
    for pfcn in [
        i32::MIN,
        i32::MIN + 1,
        -2,
        -1,
        0,
        1,
        11,
        12,
        i32::MAX - 1,
        i32::MAX,
    ] {
        let got = p.check(pfcn);
        let want = if (0..=11).contains(&pfcn) { 1 } else { 0 };
        assert_eq!(got, want, "extreme pfcn {pfcn}");
    }
}

// ===========================================================================
// Low-level pair (feature `diff_internals`).
//
// The 14 routines below are `static` in the C, so each side is reached through
// a name-identical shim: tests/cshim/cshim.c on the C side (it #includes the
// untouched c_src/src/lib.c) and the crate's `diff_internals` exports on the
// Rust side. Both are still called via dlopen/dlsym.
// ===========================================================================

#[cfg(feature = "diff_internals")]
mod internals {
    use super::*;

    type PredictSampleFn =
        unsafe extern "C" fn(*mut c_int, c_int, c_int, *mut IdxState) -> c_int;
    type PfnFn =
        unsafe extern "C" fn(c_int, *mut c_int, c_int, c_int, *mut IdxState) -> c_int;
    type IntToInt = unsafe extern "C" fn(c_int) -> c_int;

    /// `diffshim_pfn` returns this when `which` names no routine.
    const PFN_SENTINEL: c_int = 0x5EED_BAD;

    struct Side {
        _lib: Library,
        predict_sample: PredictSampleFn,
        pfn: PfnFn,
        gpf_index: IntToInt,
        layout: IntToInt,
    }

    impl Side {
        fn new(path: &Path) -> Self {
            let lib = load(path);
            unsafe {
                let predict_sample = *lib
                    .get::<PredictSampleFn>(b"diffshim_predict_sample\0")
                    .unwrap_or_else(|e| panic!("{}: diffshim_predict_sample: {e}", path.display()));
                let pfn = *lib
                    .get::<PfnFn>(b"diffshim_pfn\0")
                    .unwrap_or_else(|e| panic!("{}: diffshim_pfn: {e}", path.display()));
                let gpf_index = *lib
                    .get::<IntToInt>(b"diffshim_getpredictfunc_index\0")
                    .unwrap_or_else(|e| {
                        panic!("{}: diffshim_getpredictfunc_index: {e}", path.display())
                    });
                let layout = *lib
                    .get::<IntToInt>(b"diffshim_idxstate_layout\0")
                    .unwrap_or_else(|e| panic!("{}: diffshim_idxstate_layout: {e}", path.display()));
                Side {
                    _lib: lib,
                    predict_sample,
                    pfn,
                    gpf_index,
                    layout,
                }
            }
        }
    }

    pub struct ShimPair {
        c: Side,
        rs: Side,
    }
    unsafe impl Sync for ShimPair {}
    unsafe impl Send for ShimPair {}

    fn state_bytes(s: &IdxState) -> &[u8] {
        // IdxState is 74 bytes with no padding (2+2+2+1+1+1+1 then [[i16;8];4]),
        // and every instance here is fully initialised, so this is sound.
        unsafe {
            std::slice::from_raw_parts(
                s as *const IdxState as *const u8,
                std::mem::size_of::<IdxState>(),
            )
        }
    }

    impl ShimPair {
        fn new() -> Self {
            let a = artifacts();
            ShimPair {
                c: Side::new(&a.c_shim),
                rs: Side::new(&a.rust),
            }
        }

        /// `BTAC1C2_PredictSample`. Each side gets its own copy of the inputs;
        /// afterwards the return value **and** both buffers must agree.
        pub fn predict(
            &self,
            psamp: &[c_int; 8],
            idx: c_int,
            pfcn: c_int,
            ridx: Option<&IdxState>,
        ) -> c_int {
            let mut cbuf = *psamp;
            let mut rbuf = *psamp;
            let mut cst = ridx.copied();
            let mut rst = ridx.copied();
            let cp = cst
                .as_mut()
                .map_or(std::ptr::null_mut(), |s| s as *mut IdxState);
            let rp = rst
                .as_mut()
                .map_or(std::ptr::null_mut(), |s| s as *mut IdxState);
            let cv = unsafe { (self.c.predict_sample)(cbuf.as_mut_ptr(), idx, pfcn, cp) };
            let rv = unsafe { (self.rs.predict_sample)(rbuf.as_mut_ptr(), idx, pfcn, rp) };
            assert_eq!(
                cv, rv,
                "PredictSample(idx={idx}, pfcn={pfcn}, psamp={psamp:?}, \
                 firfx={:?}) -> C {cv} vs Rust {rv}",
                ridx.map(|s| s.firfx)
            );
            assert_eq!(cbuf, rbuf, "PredictSample mutated psamp differently");
            match (cst.as_ref(), rst.as_ref()) {
                (Some(a), Some(b)) => assert_eq!(
                    state_bytes(a),
                    state_bytes(b),
                    "PredictSample mutated *ridx differently"
                ),
                _ => {}
            }
            cv
        }

        /// `BTAC1C2_PredictSample_Pfn<which>`.
        pub fn pfn(
            &self,
            which: c_int,
            psamp: &[c_int; 8],
            idx: c_int,
            pfcn: c_int,
            ridx: Option<&IdxState>,
        ) -> c_int {
            let mut cbuf = *psamp;
            let mut rbuf = *psamp;
            let mut cst = ridx.copied();
            let mut rst = ridx.copied();
            let cp = cst
                .as_mut()
                .map_or(std::ptr::null_mut(), |s| s as *mut IdxState);
            let rp = rst
                .as_mut()
                .map_or(std::ptr::null_mut(), |s| s as *mut IdxState);
            let cv = unsafe { (self.c.pfn)(which, cbuf.as_mut_ptr(), idx, pfcn, cp) };
            let rv = unsafe { (self.rs.pfn)(which, rbuf.as_mut_ptr(), idx, pfcn, rp) };
            assert_eq!(
                cv, rv,
                "Pfn{which}(idx={idx}, pfcn={pfcn}, psamp={psamp:?}) -> C {cv} vs Rust {rv}"
            );
            assert_eq!(cbuf, rbuf, "Pfn{which} mutated psamp differently");
            cv
        }

        /// `BTAC1C2_GetPredictFunc`, reported as a dispatch-table index.
        pub fn gpf_index(&self, pfcn: c_int) -> c_int {
            let cv = unsafe { (self.c.gpf_index)(pfcn) };
            let rv = unsafe { (self.rs.gpf_index)(pfcn) };
            assert_eq!(
                cv, rv,
                "GetPredictFunc({pfcn}) identity -> C index {cv} vs Rust index {rv}"
            );
            cv
        }

        pub fn layout(&self, what: c_int) -> c_int {
            let cv = unsafe { (self.c.layout)(what) };
            let rv = unsafe { (self.rs.layout)(what) };
            assert_eq!(
                cv, rv,
                "btac1c_idxstate layout probe {what} -> C {cv} vs Rust {rv}"
            );
            cv
        }
    }

    pub fn shims() -> &'static ShimPair {
        static S: OnceLock<ShimPair> = OnceLock::new();
        S.get_or_init(ShimPair::new)
    }

    /// Build an `IdxState` whose `firfx` is `f` and whose other members carry
    /// recognisable non-zero junk (the C never reads them — if it did, we would
    /// see it).
    pub fn state_with(f: [[i16; 8]; 4]) -> IdxState {
        IdxState {
            idx: 0xBEEF,
            lpred: -12345,
            rpred: 12345,
            tag: 0xAB,
            bcfcn: 0xCD,
            bsfcn: 0xEF,
            usefx: 0x42,
            firfx: f,
        }
    }

    // =======================================================================
    // Rows 15-18: the dispatch table itself (BTAC1C2_GetPredictFunc).
    // =======================================================================

    /// Row 15: `pfcn` 0..=11 must select `_Pfn<pfcn>` — the *identity*, not a bool.
    #[test]
    fn row15_getpredictfunc_selects_matching_pfn() {
        let s = shims();
        for pfcn in 0..=11 {
            let got = s.gpf_index(pfcn);
            assert_eq!(
                got, pfcn,
                "CONFIGS row 15: GetPredictFunc({pfcn}) must select _Pfn{pfcn}"
            );
        }
    }

    /// Row 16 / ERRORS row 2: 12..=15 exist in `BTAC1C2_PredictSample` but not
    /// in `BTAC1C2_GetPredictFunc`, so they fall to `default:`.
    #[test]
    fn row16_getpredictfunc_twelve_to_fifteen_fall_through() {
        let s = shims();
        for pfcn in 12..=15 {
            assert_eq!(
                s.gpf_index(pfcn),
                12,
                "CONFIGS row 16: GetPredictFunc({pfcn}) must fall through to PredictSample"
            );
        }
    }

    /// Row 17 / ERRORS row 2: the out-of-range extremes.
    #[test]
    fn row2_getpredictfunc_default_arm() {
        let s = shims();
        for pfcn in [
            -1,
            -2,
            16,
            17,
            i32::MIN,
            i32::MIN + 1,
            i32::MAX - 1,
            i32::MAX,
        ] {
            assert_eq!(
                s.gpf_index(pfcn),
                12,
                "ERRORS row 2: GetPredictFunc({pfcn}) must return &BTAC1C2_PredictSample"
            );
        }
    }

    /// Row 18: randomized over the full `i32` range.
    #[test]
    fn row18_getpredictfunc_randomized() {
        let s = shims();
        let mut rng = rng_for(18);
        for _ in 0..20_000 {
            let pfcn = rng.next_i32();
            let got = s.gpf_index(pfcn);
            let want = if (0..=11).contains(&pfcn) { pfcn } else { 12 };
            assert_eq!(got, want, "CONFIGS row 18: GetPredictFunc({pfcn})");
        }
    }

    /// Row 19: the `struct btac1c_idxstate_s` ABI must match, or every test
    /// above that passes a `*mut IdxState` would be meaningless.
    #[test]
    fn row19_idxstate_layout_matches() {
        let s = shims();
        let names = [
            "sizeof", "alignof", "idx", "lpred", "rpred", "tag", "bcfcn", "bsfcn", "usefx",
            "firfx",
        ];
        for (what, name) in names.iter().enumerate() {
            let v = s.layout(what as c_int);
            assert!(v >= 0, "CONFIGS row 19: probe {name} returned {v}");
        }
        // sizeof/alignof/offsetof must also equal what the Rust test's own
        // #[repr(C)] mirror computes, so the harness itself is validated.
        assert_eq!(
            s.layout(0) as usize,
            std::mem::size_of::<IdxState>(),
            "row 19: sizeof(btac1c_idxstate)"
        );
        assert_eq!(
            s.layout(1) as usize,
            std::mem::align_of::<IdxState>(),
            "row 19: alignof(btac1c_idxstate)"
        );
        assert_eq!(s.layout(9), 10, "row 19: offsetof(firfx) should be 10");
        // Out-of-range probe.
        for what in [-1, 10, 11, 999, i32::MIN, i32::MAX] {
            assert_eq!(s.layout(what), -1, "row 19: probe {what} must be -1");
        }
    }
}

// ===========================================================================
// Rows 20-63: the arithmetic. Every branch of BTAC1C2_PredictSample and all
// twelve _Pfn* routines, across the psamp / idx / firfx axes.
// ===========================================================================

#[cfg(feature = "diff_internals")]
mod internals_arith {
    use super::internals::{shims, state_with};
    use super::*;

    /// Every `(psamp tier, idx variant, firfx tier)` combination is visited
    /// (8*6*4 = 192 combos, and `ITERS` = 512 revisits each with fresh values).
    fn sweep_predict(row: u64, pfcn: c_int, with_ridx: bool) {
        let s = shims();
        let mut rng = rng_for(row);
        for i in 0..ITERS {
            let tier = i % PSAMP_TIERS;
            let idxv = (i / PSAMP_TIERS) % IDX_VARIANTS;
            let ftier = (i / (PSAMP_TIERS * IDX_VARIANTS)) % FIRFX_TIERS;
            let psamp = gen_psamp(&mut rng, tier);
            let idx = gen_idx(&mut rng, idxv);
            let st = state_with(gen_firfx(&mut rng, ftier));
            s.predict(&psamp, idx, pfcn, if with_ridx { Some(&st) } else { None });
        }
    }

    fn sweep_pfn(row: u64, which: c_int) {
        let s = shims();
        let mut rng = rng_for(row);
        for i in 0..ITERS {
            let tier = i % PSAMP_TIERS;
            let idxv = (i / PSAMP_TIERS) % IDX_VARIANTS;
            let psamp = gen_psamp(&mut rng, tier);
            let idx = gen_idx(&mut rng, idxv);
            // A8: the `pfcn` argument is ignored by every _Pfn*; vary it anyway.
            let pfcn = rng.next_i32();
            // A7: `ridx` is never dereferenced by any _Pfn*; alternate NULL.
            let st = state_with(gen_firfx(&mut rng, tier % FIRFX_TIERS));
            let ridx = if i % 2 == 0 { Some(&st) } else { None };
            s.pfn(which, &psamp, idx, pfcn, ridx);
        }
    }

    macro_rules! predict_row {
        ($name:ident, $row:expr, $pfcn:expr) => {
            #[test]
            fn $name() {
                sweep_predict($row, $pfcn, true);
            }
        };
    }

    macro_rules! pfn_row {
        ($name:ident, $row:expr, $which:expr) => {
            #[test]
            fn $name() {
                sweep_pfn($row, $which);
            }
        };
    }

    // --- Rows 20-31: the twelve closed-form arms of the big switch ----------
    predict_row!(row20_predict_sample_pfcn0, 20, 0);
    predict_row!(row21_predict_sample_pfcn1, 21, 1);
    predict_row!(row22_predict_sample_pfcn2, 22, 2);
    predict_row!(row23_predict_sample_pfcn3, 23, 3);
    predict_row!(row24_predict_sample_pfcn4, 24, 4);
    predict_row!(row25_predict_sample_pfcn5, 25, 5);
    predict_row!(row26_predict_sample_pfcn6, 26, 6);
    predict_row!(row27_predict_sample_pfcn7, 27, 7);
    predict_row!(row28_predict_sample_pfcn8, 28, 8);
    predict_row!(row29_predict_sample_pfcn9, 29, 9);
    predict_row!(row30_predict_sample_pfcn10, 30, 10);
    predict_row!(row31_predict_sample_pfcn11, 31, 11);

    // --- Rows 32-35: the four FIR arms, one per firfx row -------------------
    predict_row!(row32_predict_sample_pfcn12_firfx_row0, 32, 12);
    predict_row!(row33_predict_sample_pfcn13_firfx_row1, 33, 13);
    predict_row!(row34_predict_sample_pfcn14_firfx_row2, 34, 14);
    predict_row!(row35_predict_sample_pfcn15_firfx_row3, 35, 15);

    /// Row 36: degenerate FIR taps — the numerator is exactly 0.
    #[test]
    fn row36_fir_all_zero_taps() {
        let s = shims();
        let mut rng = rng_for(36);
        let st = state_with([[0i16; 8]; 4]);
        for i in 0..ITERS {
            let psamp = gen_psamp(&mut rng, i % PSAMP_TIERS);
            let idx = gen_idx(&mut rng, (i / PSAMP_TIERS) % IDX_VARIANTS);
            for pfcn in 12..=15 {
                let v = s.predict(&psamp, idx, pfcn, Some(&st));
                assert_eq!(v, 0, "CONFIGS row 36: zero taps must give 0 (pfcn={pfcn})");
            }
        }
    }

    /// Row 37: taps saturated to INT16_MIN/INT16_MAX — maximum-magnitude
    /// products, so the 8-term sum overflows `int`.
    #[test]
    fn row37_fir_saturated_taps() {
        let s = shims();
        let mut rng = rng_for(37);
        for i in 0..ITERS {
            let psamp = gen_psamp(&mut rng, i % PSAMP_TIERS);
            let idx = gen_idx(&mut rng, (i / PSAMP_TIERS) % IDX_VARIANTS);
            // tier 3 of gen_firfx is the saturated one.
            let st = state_with(gen_firfx(&mut rng, 3));
            for pfcn in 12..=15 {
                s.predict(&psamp, idx, pfcn, Some(&st));
            }
        }
    }

    /// Row 38: all four `firfx` rows hold different taps, so a row-selection
    /// (`pfcn - 12`) bug would change the result.
    #[test]
    fn row38_fir_row_selection() {
        let s = shims();
        let mut f = [[0i16; 8]; 4];
        for (k, row) in f.iter_mut().enumerate() {
            for (j, v) in row.iter_mut().enumerate() {
                *v = (k as i16 + 1) * 1000 + j as i16;
            }
        }
        let st = state_with(f);
        // psamp all 256 => result is exactly the sum of that row's taps.
        let psamp = [256 as c_int; 8];
        let mut seen = Vec::new();
        for pfcn in 12..=15 {
            let v = s.predict(&psamp, 0, pfcn, Some(&st));
            let k = (pfcn - 12) as i16;
            let want: c_int = (0..8).map(|j| ((k + 1) * 1000 + j) as c_int).sum();
            assert_eq!(
                v, want,
                "CONFIGS row 38: pfcn={pfcn} must read firfx row {}",
                pfcn - 12
            );
            seen.push(v);
        }
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(seen.len(), 4, "CONFIGS row 38: the four rows must differ");
    }

    /// Row 39 / ERRORS row 1: the big switch's `default:` arm yields 0.
    #[test]
    fn row1_predictsample_default_via_pointer_identity() {
        let s = shims();
        let mut rng = rng_for(39);
        let st = state_with(gen_firfx(&mut rng, 2));
        for pfcn in [16, 17, 100, -1, -2, -100, i32::MIN, i32::MAX] {
            for tier in 0..PSAMP_TIERS {
                let psamp = gen_psamp(&mut rng, tier);
                let idx = gen_idx(&mut rng, tier % IDX_VARIANTS);
                let v = s.predict(&psamp, idx, pfcn, Some(&st));
                assert_eq!(
                    v, 0,
                    "ERRORS row 1: PredictSample(pfcn={pfcn}) default arm must give 0"
                );
            }
        }
    }

    /// Row 40 (A7): `ridx == NULL` is legal for every `pfcn` that is not 12-15,
    /// because the C only dereferences it in those four cases.
    #[test]
    fn row40_predict_sample_null_ridx() {
        for pfcn in 0..=11 {
            sweep_predict(4000 + pfcn as u64, pfcn, false);
        }
        // The `default:` arm does not touch ridx either.
        for (n, pfcn) in [16, -1, i32::MIN, i32::MAX].into_iter().enumerate() {
            sweep_predict(4100 + n as u64, pfcn, false);
        }
    }

    // --- Rows 41-52: the twelve standalone _Pfn* routines -------------------
    pfn_row!(row41_pfn0, 41, 0);
    pfn_row!(row42_pfn1, 42, 1);
    pfn_row!(row43_pfn2, 43, 2);
    pfn_row!(row44_pfn3, 44, 3);
    pfn_row!(row45_pfn4, 45, 4);
    pfn_row!(row46_pfn5, 46, 5);
    pfn_row!(row47_pfn6, 47, 6);
    pfn_row!(row48_pfn7, 48, 7);
    pfn_row!(row49_pfn8, 49, 8);
    pfn_row!(row50_pfn9, 50, 9);
    pfn_row!(row51_pfn10, 51, 10);
    pfn_row!(row52_pfn11, 52, 11);

    /// Row 53 (A8): the `pfcn` parameter is accepted but never read by any
    /// `_Pfn*`, so sweeping it must not change the result — on either side.
    #[test]
    fn row53_pfn_ignores_pfcn_argument() {
        let s = shims();
        let mut rng = rng_for(53);
        for which in 0..12 {
            for tier in 0..PSAMP_TIERS {
                let psamp = gen_psamp(&mut rng, tier);
                let idx = gen_idx(&mut rng, tier % IDX_VARIANTS);
                let base = s.pfn(which, &psamp, idx, 0, None);
                for pfcn in [
                    1,
                    2,
                    11,
                    12,
                    15,
                    16,
                    -1,
                    i32::MIN,
                    i32::MAX,
                ] {
                    let v = s.pfn(which, &psamp, idx, pfcn, None);
                    assert_eq!(
                        v, base,
                        "CONFIGS row 53: _Pfn{which} must ignore pfcn (got {v} for pfcn={pfcn}, \
                         {base} for pfcn=0)"
                    );
                }
            }
        }
    }

    /// Row 54: `_Pfn10` shifts by 3 while `case 10:` shifts by 4. Both sides
    /// must reproduce that discrepancy identically — this is the bug-for-bug
    /// check, so it asserts the two really do differ.
    #[test]
    fn row54_pfn10_differs_from_case10() {
        let s = shims();
        let mut rng = rng_for(54);
        let st = state_with([[0i16; 8]; 4]);
        let mut differed = 0;
        for _ in 0..ITERS {
            // "safe large" tier: no overflow, so the shift is the only variable.
            let psamp = gen_psamp(&mut rng, 3);
            let idx = gen_idx(&mut rng, 0);
            let a = s.pfn(10, &psamp, idx, 0, None);
            let b = s.predict(&psamp, idx, 10, Some(&st));
            if a != b {
                differed += 1;
            }
        }
        assert!(
            differed > 0,
            "CONFIGS row 54: _Pfn10 (>>3) and case 10 (>>4) must differ somewhere"
        );
    }

    /// Row 55: `_Pfn11` shifts by 1 while `case 11:` shifts by 3.
    #[test]
    fn row55_pfn11_differs_from_case11() {
        let s = shims();
        let mut rng = rng_for(55);
        let st = state_with([[0i16; 8]; 4]);
        let mut differed = 0;
        for _ in 0..ITERS {
            let psamp = gen_psamp(&mut rng, 3);
            let idx = gen_idx(&mut rng, 0);
            let a = s.pfn(11, &psamp, idx, 0, None);
            let b = s.predict(&psamp, idx, 11, Some(&st));
            if a != b {
                differed += 1;
            }
        }
        assert!(
            differed > 0,
            "CONFIGS row 55: _Pfn11 (>>1) and case 11 (>>3) must differ somewhere"
        );
    }

    // --- Rows 56-58: the idx axis, across every entry point ----------------

    /// Every `pfcn` (including the `default:` band) and every `_Pfn*`, at the
    /// given `idx` values, over all psamp tiers.
    fn sweep_all_entry_points_at_idx(row: u64, idxs: &[c_int]) {
        let s = shims();
        let mut rng = rng_for(row);
        for &idx in idxs {
            for tier in 0..PSAMP_TIERS {
                let psamp = gen_psamp(&mut rng, tier);
                let st = state_with(gen_firfx(&mut rng, tier % FIRFX_TIERS));
                for pfcn in -2..=17 {
                    s.predict(&psamp, idx, pfcn, Some(&st));
                }
                for which in 0..12 {
                    s.pfn(which, &psamp, idx, 0, Some(&st));
                }
            }
        }
    }

    /// Row 56: `idx` in range — `(idx - n) & 7` needs no wrapping for n <= idx.
    #[test]
    fn row56_idx_in_range() {
        sweep_all_entry_points_at_idx(56, &[0, 1, 2, 3, 4, 5, 6, 7]);
    }

    /// Row 57: negative `idx` — masking a negative `int`.
    #[test]
    fn row57_idx_negative() {
        let idxs: Vec<c_int> = (1..=16).map(|k| -k).collect();
        sweep_all_entry_points_at_idx(57, &idxs);
    }

    /// Row 58: `idx` where `idx - n` overflows `int`.
    #[test]
    fn row58_idx_overflow_extremes() {
        let mut idxs: Vec<c_int> = Vec::new();
        for k in 0..=8 {
            idxs.push(i32::MIN.wrapping_add(k));
            idxs.push(i32::MAX - k);
        }
        sweep_all_entry_points_at_idx(58, &idxs);
    }

    // --- Rows 59-63: the psamp-contents axis, across every entry point ------

    fn sweep_all_entry_points_at_tier(row: u64, tier: u32) {
        let s = shims();
        let mut rng = rng_for(row);
        for _ in 0..48 {
            let psamp = gen_psamp(&mut rng, tier);
            for idxv in 0..IDX_VARIANTS {
                let idx = gen_idx(&mut rng, idxv);
                let ftier = rng.below(FIRFX_TIERS);
                let st = state_with(gen_firfx(&mut rng, ftier));
                for pfcn in -2..=17 {
                    s.predict(&psamp, idx, pfcn, Some(&st));
                }
                for which in 0..12 {
                    s.pfn(which, &psamp, idx, 0, Some(&st));
                }
            }
        }
    }

    /// Row 59: all-zero samples.
    #[test]
    fn row59_psamp_all_zero() {
        sweep_all_entry_points_at_tier(59, 0);
    }

    /// Row 60: all-negative samples. Arithmetic `>>` floors while `/` truncates
    /// toward zero, so this separates the two families of arms.
    #[test]
    fn row60_psamp_all_negative() {
        sweep_all_entry_points_at_tier(60, 6);
    }

    /// Row 61: negative numerators that are deliberately *not* multiples of
    /// 16/64/256 — pins down the sign of the remainder.
    #[test]
    fn row61_psamp_negative_not_divisible() {
        sweep_all_entry_points_at_tier(61, 7);
    }

    /// Row 62: full-`i32` extremes — `72*x`, `5*p0`, … overflow here.
    #[test]
    fn row62_psamp_full_extremes() {
        sweep_all_entry_points_at_tier(62, 4);
        sweep_all_entry_points_at_tier(620, 5);
    }

    /// Row 63: "safe large" (`|v| <= 2^20`) — no intermediate overflow, so this
    /// row is meaningful regardless of overflow semantics.
    #[test]
    fn row63_psamp_safe_large() {
        sweep_all_entry_points_at_tier(63, 3);
    }

    /// Generic boundary: `which` outside 0..=11 must hit the shim sentinel on
    /// both sides (guards the dispatch in the shim itself).
    #[test]
    fn generic_pfn_dispatch_out_of_range() {
        let s = shims();
        let psamp = [1 as c_int, 2, 3, 4, 5, 6, 7, 8];
        for which in [-1, 12, 13, 100, i32::MIN, i32::MAX] {
            let v = s.pfn(which, &psamp, 0, 0, None);
            assert_eq!(
                v, 0x5EED_BAD,
                "out-of-range _Pfn selector {which} must give the sentinel"
            );
        }
    }
}
