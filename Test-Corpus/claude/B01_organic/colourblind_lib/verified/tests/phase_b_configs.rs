//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every test loads BOTH the C `.so` and the
//! Rust `.so` via `libloading` and compares the three output `f32`s
//! bit-for-bit. Inputs are generated from a fixed-seed SplitMix64 PRNG so runs
//! are reproducible.

mod common;

use common::*;

// ---------------------------------------------------------------------------
// Input generators — one per value-shape class (Axis II of CONFIGS.md)
// ---------------------------------------------------------------------------

/// V1: random normals in `[0, 1)` — the intended colour range.
fn v1(seed: u64, n: usize) -> Vec<[f32; 3]> {
    let mut rng = Rng::new(seed);
    (0..n)
        .map(|_| [rng.unit(), rng.unit(), rng.unit()])
        .collect()
}

/// V2: random normals in `[-1, 1)`.
fn v2(seed: u64, n: usize) -> Vec<[f32; 3]> {
    let mut rng = Rng::new(seed);
    (0..n)
        .map(|_| [rng.signed_unit(), rng.signed_unit(), rng.signed_unit()])
        .collect()
}

/// V3: random normals spanning the whole exponent range.
fn v3(seed: u64, n: usize) -> Vec<[f32; 3]> {
    let mut rng = Rng::new(seed);
    (0..n)
        .map(|_| {
            [
                rng.normal_full_range(),
                rng.normal_full_range(),
                rng.normal_full_range(),
            ]
        })
        .collect()
}

/// V4: all exact `+0.0`.
fn v4() -> Vec<[f32; 3]> {
    vec![[0.0, 0.0, 0.0]]
}

/// V5: all eight signed-zero combinations.
fn v5() -> Vec<[f32; 3]> {
    cube(&[0.0f32, -0.0f32])
}

/// V6: subnormals, including the smallest positive/negative subnormal.
fn v6(seed: u64, n: usize) -> Vec<[f32; 3]> {
    let mut rng = Rng::new(seed);
    let tiny = f32::from_bits(1); // 1e-45
    let mut out = cube(&[tiny, -tiny]);
    out.extend((0..n).map(|_| [rng.subnormal(), rng.subnormal(), rng.subnormal()]));
    out
}

/// V7: `±FLT_MAX` — the weighted sums overflow to `±inf`.
fn v7() -> Vec<[f32; 3]> {
    cube(&[f32::MAX, -f32::MAX])
}

/// V8: `±FLT_MIN` (smallest positive normal).
fn v8() -> Vec<[f32; 3]> {
    cube(&[f32::MIN_POSITIVE, -f32::MIN_POSITIVE])
}

/// V9: all eight `±inf` combinations (produces `inf - inf` -> NaN).
fn v9() -> Vec<[f32; 3]> {
    cube(&[f32::INFINITY, f32::NEG_INFINITY])
}

/// V10: NaN payload / sign propagation — all 4^3 = 64 combinations.
fn v10() -> Vec<[f32; 3]> {
    let pool: Vec<f32> = NAN_VARIANTS.iter().map(|&b| f32::from_bits(b)).collect();
    cube(&pool)
}

/// V11: `±inf` mixed with finite operands.
fn v11(seed: u64, n: usize) -> Vec<[f32; 3]> {
    let mut rng = Rng::new(seed);
    (0..n)
        .map(|_| {
            let mut t = [
                rng.normal_full_range(),
                rng.normal_full_range(),
                rng.normal_full_range(),
            ];
            // Force between one and three lanes to an infinity.
            let count = 1 + rng.below(3) as usize;
            for _ in 0..count {
                let lane = rng.below(3) as usize;
                t[lane] = if rng.bool() {
                    f32::INFINITY
                } else {
                    f32::NEG_INFINITY
                };
            }
            t
        })
        .collect()
}

/// V12: `G == B` exactly — maximises cancellation in Tritanopia's
/// `0.1273...*G - 0.1273...*B`.
fn v12(seed: u64, n: usize) -> Vec<[f32; 3]> {
    let mut rng = Rng::new(seed);
    (0..n)
        .map(|_| {
            let v = if rng.bool() {
                rng.signed_unit()
            } else {
                rng.normal_full_range()
            };
            [rng.signed_unit(), v, v]
        })
        .collect()
}

/// V13: arbitrary random `u32` bit patterns reinterpreted as `f32`.
fn v13(seed: u64, n: usize) -> Vec<[f32; 3]> {
    let mut rng = Rng::new(seed);
    (0..n)
        .map(|_| [rng.any_bits(), rng.any_bits(), rng.any_bits()])
        .collect()
}

/// V14: exact powers of two `±2^k` for every representable `k`, plus small
/// exact integers.
fn v14() -> Vec<[f32; 3]> {
    let mut out = Vec::new();
    for k in -149i32..=127 {
        let p = pow2(k);
        let q = pow2((-k).clamp(-149, 127));
        out.push([p, -p, q]);
        out.push([-p, q, p]);
        out.push([p, p, p]);
    }
    for i in 0..=256i32 {
        let f = i as f32;
        out.push([f, -f, f + 1.0]);
    }
    out
}

/// V15: triples of values one ULP apart around many magnitudes.
fn v15(seed: u64, n: usize) -> Vec<[f32; 3]> {
    let mut rng = Rng::new(seed);
    (0..n)
        .map(|_| {
            let v = if rng.bool() {
                rng.signed_unit()
            } else {
                rng.normal_full_range()
            };
            [v, next_up(v), next_down(v)]
        })
        .collect()
}

/// V16 seeds: starting points for the iterated in-place application.
fn v16_seeds(seed: u64, n: usize) -> Vec<[f32; 3]> {
    let mut rng = Rng::new(seed);
    (0..n)
        .map(|_| {
            if rng.bool() {
                [rng.unit(), rng.unit(), rng.unit()]
            } else {
                [
                    rng.normal_full_range(),
                    rng.normal_full_range(),
                    rng.normal_full_range(),
                ]
            }
        })
        .collect()
}

/// Mixed pool used by the aliasing rows: half plain normals, half arbitrary
/// bit patterns (so NaN/inf/subnormal aliasing is covered too).
fn alias_inputs(seed: u64, n: usize) -> Vec<[f32; 3]> {
    let mut rng = Rng::new(seed);
    (0..n)
        .map(|_| match rng.below(3) {
            0 => [rng.unit(), rng.unit(), rng.unit()],
            1 => [
                rng.normal_full_range(),
                rng.normal_full_range(),
                rng.normal_full_range(),
            ],
            _ => [rng.any_bits(), rng.any_bits(), rng.any_bits()],
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Rows C1..C48 — value-shape x Impairment cross-product
// ---------------------------------------------------------------------------

/// Generates the three per-impairment tests for one value-shape class.
macro_rules! value_rows {
    ($( $class:ident : [$r0:ident, $r1:ident, $r2:ident] = $gen:expr; )*) => {
        $(
            #[test]
            fn $r0() { run_row(stringify!($r0), CB_PROTANOPIA, &$gen); }
            #[test]
            fn $r1() { run_row(stringify!($r1), CB_DEUTERANOPIA, &$gen); }
            #[test]
            fn $r2() { run_row(stringify!($r2), CB_TRITANOPIA, &$gen); }
        )*
    };
}

value_rows! {
    // C1  C2  C3
    v1:  [cfg_v1_imp0,  cfg_v1_imp1,  cfg_v1_imp2]  = v1(0x1111_1111, 1000);
    // C4  C5  C6
    v2:  [cfg_v2_imp0,  cfg_v2_imp1,  cfg_v2_imp2]  = v2(0x2222_2222, 1000);
    // C7  C8  C9
    v3:  [cfg_v3_imp0,  cfg_v3_imp1,  cfg_v3_imp2]  = v3(0x3333_3333, 1000);
    // C10 C11 C12
    v4:  [cfg_v4_imp0,  cfg_v4_imp1,  cfg_v4_imp2]  = v4();
    // C13 C14 C15
    v5:  [cfg_v5_imp0,  cfg_v5_imp1,  cfg_v5_imp2]  = v5();
    // C16 C17 C18
    v6:  [cfg_v6_imp0,  cfg_v6_imp1,  cfg_v6_imp2]  = v6(0x6666_6666, 500);
    // C19 C20 C21
    v7:  [cfg_v7_imp0,  cfg_v7_imp1,  cfg_v7_imp2]  = v7();
    // C22 C23 C24
    v8:  [cfg_v8_imp0,  cfg_v8_imp1,  cfg_v8_imp2]  = v8();
    // C25 C26 C27
    v9:  [cfg_v9_imp0,  cfg_v9_imp1,  cfg_v9_imp2]  = v9();
    // C28 C29 C30
    v10: [cfg_v10_imp0, cfg_v10_imp1, cfg_v10_imp2] = v10();
    // C31 C32 C33
    v11: [cfg_v11_imp0, cfg_v11_imp1, cfg_v11_imp2] = v11(0xAAAA_AAAA, 500);
    // C34 C35 C36
    v12: [cfg_v12_imp0, cfg_v12_imp1, cfg_v12_imp2] = v12(0xBBBB_BBBB, 500);
    // C37 C38 C39
    v13: [cfg_v13_imp0, cfg_v13_imp1, cfg_v13_imp2] = v13(0xCCCC_CCCC, 4000);
    // C40 C41 C42
    v14: [cfg_v14_imp0, cfg_v14_imp1, cfg_v14_imp2] = v14();
    // C43 C44 C45
    v15: [cfg_v15_imp0, cfg_v15_imp1, cfg_v15_imp2] = v15(0xDDDD_DDDD, 1000);
}

// --- C46..C48: iterated in-place application (output fed back 32 times) -----

fn run_iterated(row: &str, impairment: i32) {
    let seeds = v16_seeds(0xEEEE_EEEE, 300);
    for start in seeds {
        let mut c = start;
        let mut r = start;
        for step in 0..32 {
            c = c_lib().call(impairment, c);
            r = rust_lib().call(impairment, r);
            assert!(
                bits_eq(c, r),
                "[{row}] divergence at iteration {step} for Impairment={impairment} \
                 start={}\n  C   : {}\n  Rust: {}",
                fmt3(start),
                fmt3(c),
                fmt3(r)
            );
        }
    }
    eprintln!("[{row}] OK  Impairment={impairment}  300 starts x 32 iterations");
}

#[test]
fn cfg_v16_imp0() {
    run_iterated("cfg_v16_imp0", CB_PROTANOPIA);
}
#[test]
fn cfg_v16_imp1() {
    run_iterated("cfg_v16_imp1", CB_DEUTERANOPIA);
}
#[test]
fn cfg_v16_imp2() {
    run_iterated("cfg_v16_imp2", CB_TRITANOPIA);
}

// ---------------------------------------------------------------------------
// Rows C49..C60 — pointer aliasing x Impairment
//
// The C signature has no `restrict`, and each kernel reads all three inputs
// into locals BEFORE storing, so aliasing is well defined in C: every read
// observes the original value and the last store to an address wins.
// ---------------------------------------------------------------------------

/// Which argument slots share storage.
#[derive(Copy, Clone, PartialEq, Debug)]
enum Alias {
    /// R and G point at the same float.
    Rg,
    /// R and B point at the same float.
    Rb,
    /// G and B point at the same float.
    Gb,
    /// All three point at the same float.
    All,
}

/// Perform an aliased call against one library and return the full storage
/// state afterwards (2 cells for the pairwise cases, 1 for the all-aliased
/// case, padded to 3 for uniform reporting).
fn aliased_call(lib: &Lib, impairment: i32, alias: Alias, input: [f32; 3]) -> [f32; 3] {
    match alias {
        Alias::Rg => {
            // slot0 shared by R and G, slot1 holds B
            let mut cell = [input[0], input[2], 0.0f32];
            unsafe {
                let p0 = cell.as_mut_ptr();
                let p1 = cell.as_mut_ptr().add(1);
                lib.call_raw(impairment, p0, p0, p1);
            }
            [cell[0], cell[1], 0.0]
        }
        Alias::Rb => {
            // slot0 shared by R and B, slot1 holds G
            let mut cell = [input[0], input[1], 0.0f32];
            unsafe {
                let p0 = cell.as_mut_ptr();
                let p1 = cell.as_mut_ptr().add(1);
                lib.call_raw(impairment, p0, p1, p0);
            }
            [cell[0], cell[1], 0.0]
        }
        Alias::Gb => {
            // slot0 holds R, slot1 shared by G and B
            let mut cell = [input[0], input[1], 0.0f32];
            unsafe {
                let p0 = cell.as_mut_ptr();
                let p1 = cell.as_mut_ptr().add(1);
                lib.call_raw(impairment, p0, p1, p1);
            }
            [cell[0], cell[1], 0.0]
        }
        Alias::All => {
            let mut cell = [input[0], 0.0f32, 0.0f32];
            unsafe {
                let p0 = cell.as_mut_ptr();
                lib.call_raw(impairment, p0, p0, p0);
            }
            [cell[0], 0.0, 0.0]
        }
    }
}

fn run_alias_row(row: &str, impairment: i32, alias: Alias) {
    let inputs = alias_inputs(0x5A5A_5A5A, 1000);
    for input in inputs.iter().copied() {
        let c = aliased_call(c_lib(), impairment, alias, input);
        let r = aliased_call(rust_lib(), impairment, alias, input);
        assert!(
            bits_eq(c, r),
            "[{row}] aliasing divergence ({alias:?}) for Impairment={impairment} \
             input={}\n  C   : {}\n  Rust: {}",
            fmt3(input),
            fmt3(c),
            fmt3(r)
        );
    }
    eprintln!(
        "[{row}] OK  Impairment={impairment}  alias={alias:?}  {} inputs",
        inputs.len()
    );
}

macro_rules! alias_rows {
    ($( $alias:ident : [$r0:ident, $r1:ident, $r2:ident]; )*) => {
        $(
            #[test]
            fn $r0() { run_alias_row(stringify!($r0), CB_PROTANOPIA,   Alias::$alias); }
            #[test]
            fn $r1() { run_alias_row(stringify!($r1), CB_DEUTERANOPIA, Alias::$alias); }
            #[test]
            fn $r2() { run_alias_row(stringify!($r2), CB_TRITANOPIA,   Alias::$alias); }
        )*
    };
}

alias_rows! {
    // C49 C50 C51
    Rg:  [cfg_a2_imp0, cfg_a2_imp1, cfg_a2_imp2];
    // C52 C53 C54
    Rb:  [cfg_a3_imp0, cfg_a3_imp1, cfg_a3_imp2];
    // C55 C56 C57
    Gb:  [cfg_a4_imp0, cfg_a4_imp1, cfg_a4_imp2];
    // C58 C59 C60
    All: [cfg_a5_imp0, cfg_a5_imp1, cfg_a5_imp2];
}

// ---------------------------------------------------------------------------
// C61 — unaligned `float*`
//
// GCC emits `movss`, which has no alignment requirement, so a byte-offset
// `float*` is honoured by the C build. The Rust side must behave identically.
// ---------------------------------------------------------------------------

fn unaligned_call(lib: &Lib, impairment: i32, input: [f32; 3], offset: usize) -> [f32; 3] {
    // 16 bytes of scratch + up to 3 bytes of misalignment.
    let mut buf = [0u8; 32];
    for (i, v) in input.iter().enumerate() {
        let at = offset + i * 4;
        buf[at..at + 4].copy_from_slice(&v.to_ne_bytes());
    }
    unsafe {
        let base = buf.as_mut_ptr().add(offset);
        lib.call_raw(
            impairment,
            base as *mut f32,
            base.add(4) as *mut f32,
            base.add(8) as *mut f32,
        );
    }
    let mut out = [0f32; 3];
    for (i, o) in out.iter_mut().enumerate() {
        let at = offset + i * 4;
        *o = f32::from_ne_bytes(buf[at..at + 4].try_into().unwrap());
    }
    out
}

#[test]
fn cfg_unaligned_all_imps() {
    let row = "cfg_unaligned_all_imps";
    let inputs = v1(0x0F0F_0F0F, 300);
    for &imp in &VALID_IMPAIRMENTS {
        for offset in 1..=3usize {
            for input in inputs.iter().copied() {
                let c = unaligned_call(c_lib(), imp, input, offset);
                let r = unaligned_call(rust_lib(), imp, input, offset);
                assert!(
                    bits_eq(c, r),
                    "[{row}] unaligned divergence (offset={offset}) for Impairment={imp} \
                     input={}\n  C   : {}\n  Rust: {}",
                    fmt3(input),
                    fmt3(c),
                    fmt3(r)
                );
            }
        }
    }
    eprintln!("[{row}] OK  3 impairments x 3 offsets x 300 inputs");
}

// ---------------------------------------------------------------------------
// C62 — chained calls across all three impairments on the same buffer
// ---------------------------------------------------------------------------

#[test]
fn cfg_chain_all_imps() {
    let row = "cfg_chain_all_imps";
    let inputs = v16_seeds(0x1234_5678, 500);
    for start in inputs {
        let mut c = start;
        let mut r = start;
        for &imp in &VALID_IMPAIRMENTS {
            c = c_lib().call(imp, c);
            r = rust_lib().call(imp, r);
            assert!(
                bits_eq(c, r),
                "[{row}] chain divergence after Impairment={imp} start={}\n  \
                 C   : {}\n  Rust: {}",
                fmt3(start),
                fmt3(c),
                fmt3(r)
            );
        }
    }
    eprintln!("[{row}] OK  500 starts x chain(0,1,2)");
}

// ---------------------------------------------------------------------------
// C63 — determinism: the same input replayed must give the same output
// (there is no global or `static` mutable state in the C).
// ---------------------------------------------------------------------------

#[test]
fn cfg_replay_determinism() {
    let row = "cfg_replay_determinism";
    let inputs = v13(0x9999_9999, 500);
    for &imp in &VALID_IMPAIRMENTS {
        for input in inputs.iter().copied() {
            let c1 = c_lib().call(imp, input);
            let c2 = c_lib().call(imp, input);
            let r1 = rust_lib().call(imp, input);
            let r2 = rust_lib().call(imp, input);
            assert!(bits_eq(c1, c2), "[{row}] C not deterministic");
            assert!(bits_eq(r1, r2), "[{row}] Rust not deterministic");
            assert!(
                bits_eq(c1, r1),
                "[{row}] divergence for Impairment={imp} input={}\n  C   : {}\n  Rust: {}",
                fmt3(input),
                fmt3(c1),
                fmt3(r1)
            );
        }
    }
    eprintln!("[{row}] OK  3 impairments x 500 inputs x 2 replays");
}

// ---------------------------------------------------------------------------
// C64 — NaN mixed with finite operands, exhaustive 3-cube.
//
// Hardens the per-channel NaN-payload priority: when only *some* operands are
// NaN, the surviving payload depends on which term GCC placed in the
// destination register, and that differs per output channel. A pool of 4 NaN
// variants + 4 finite values gives 8^3 = 512 combinations per impairment.
// ---------------------------------------------------------------------------

#[test]
fn cfg_v17_nan_finite_cube_all_imps() {
    let row = "cfg_v17_nan_finite_cube_all_imps";
    let mut pool: Vec<f32> = NAN_VARIANTS.iter().map(|&b| f32::from_bits(b)).collect();
    pool.extend_from_slice(&[0.25f32, -3.5f32, 0.0f32, f32::INFINITY]);
    let inputs = cube(&pool);
    assert_eq!(inputs.len(), 512);
    for &imp in &VALID_IMPAIRMENTS {
        run_row(row, imp, &inputs);
    }
}

// ---------------------------------------------------------------------------
// C65 — high-volume randomized soak over arbitrary bit patterns.
// ---------------------------------------------------------------------------

#[test]
fn cfg_soak_random_bit_patterns() {
    let row = "cfg_soak_random_bit_patterns";
    const N: usize = 200_000;
    let mut rng = Rng::new(0x5EED_5EED);
    let mut checked = 0usize;
    for _ in 0..N {
        let input = [rng.any_bits(), rng.any_bits(), rng.any_bits()];
        let imp = VALID_IMPAIRMENTS[(rng.below(3)) as usize];
        assert_same(row, imp, input);
        checked += 1;
    }
    eprintln!("[{row}] OK  {checked} randomized triples across all 3 impairments");
}

// ---------------------------------------------------------------------------
// Sanity: both libraries were really loaded from two distinct .so files.
// ---------------------------------------------------------------------------

#[test]
fn harness_loads_two_distinct_shared_objects() {
    let c = c_so_path();
    let r = rust_so_path();
    assert!(c.exists(), "C .so missing: {}", c.display());
    assert!(r.exists(), "Rust .so missing: {}", r.display());
    assert_ne!(
        std::fs::canonicalize(&c).unwrap(),
        std::fs::canonicalize(&r).unwrap(),
        "the harness must load two different shared objects"
    );
    assert_ne!(
        c_lib().raw_fn() as usize,
        rust_lib().raw_fn() as usize,
        "`colourblind` resolved to the same address in both libraries"
    );
    eprintln!("C   .so: {}", c.display());
    eprintln!("Rust.so: {}", r.display());
}
