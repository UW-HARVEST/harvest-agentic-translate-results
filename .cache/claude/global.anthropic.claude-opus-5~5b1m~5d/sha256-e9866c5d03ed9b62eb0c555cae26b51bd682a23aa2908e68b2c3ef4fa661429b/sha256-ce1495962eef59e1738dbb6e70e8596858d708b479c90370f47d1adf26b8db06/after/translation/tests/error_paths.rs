//! Phase C — error/rejection-path differential tests, one `#[test]` per row of
//! `ERRORS.md`.
//!
//! Rows whose expected C behaviour is process death are verified for *signal
//! parity*: the same test binary is re-executed in a child process that calls
//! only the C `.so` or only the Rust `.so`, and the two children must terminate
//! with the identical signal / exit code.

mod common;

use common::*;
use std::ffi::c_int;
use std::os::unix::process::ExitStatusExt;
use std::process::{Command, Stdio};

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

fn bits(v: &[u32]) -> Vec<f32> {
    v.iter().map(|&b| f32::from_bits(b)).collect()
}

/// A scenario with hand-picked pointer arguments (used for the NULL rows).
fn raw(a_len: usize, b_len: usize, dst: P, src: P, size: c_int, label: &str) -> Scenario {
    Scenario {
        a: vec![SENTINEL; a_len],
        b: vec![SENTINEL; b_len],
        dst,
        src,
        size,
        label: label.to_string(),
    }
}

const QNAN: u32 = 0x7FC0_0000;
const NEG_QNAN: u32 = 0xFFC0_0000;
const SNAN: u32 = 0x7FBF_FFFF;
const SNAN_MIN: u32 = 0x7F80_0001;
const QNAN_MAX: u32 = 0x7FFF_FFFF;
const PINF: u32 = 0x7F80_0000;
const NINF: u32 = 0xFF80_0000;

// ---------------------------------------------------------------------------
// rows 1-5: size == 0 and NULL pointers
// ---------------------------------------------------------------------------

#[test] // row 1
fn err_01_size_zero_disjoint() {
    let mut rng = Rng::new(0x1001);
    // empty live window
    assert_same(&Scenario::disjoint(&[], 0, 0));
    // non-empty buffers but size == 0: not one byte may be written
    for n in [1usize, 4, 17, 64] {
        for off in 0..4 {
            let data = gen_data(Dist::Unit, n, &mut rng);
            assert_same(&Scenario::disjoint(&data, off, 0));
            let data = gen_data(Dist::FiniteBits, n, &mut rng);
            assert_same(&Scenario::disjoint(&data, off, 0));
        }
    }
}

#[test] // row 2
fn err_02_size_zero_aliased() {
    let mut rng = Rng::new(0x1002);
    assert_same(&Scenario::in_place(&[], 0, 0));
    for n in [1usize, 4, 17, 64] {
        for off in 0..4 {
            let data = gen_data(Dist::Wide, n, &mut rng);
            assert_same(&Scenario::in_place(&data, off, 0));
        }
    }
}

#[test] // row 3
fn err_03_size_zero_both_null() {
    assert_same(&raw(0, 0, P::Null, P::Null, 0, "size=0 dest=NULL src=NULL"));
}

#[test] // row 4
fn err_04_size_zero_dest_null() {
    // dest == NULL != src  =>  guard #4 is TRUE  =>  memset(NULL, 0, 0)
    let mut s = raw(32, 0, P::Null, P::A(8), 0, "size=0 dest=NULL src=valid");
    let mut rng = Rng::new(0x1004);
    for i in 8..24 {
        s.a[i] = rng.next_u32();
    }
    assert_same(&s);
}

#[test] // row 5
fn err_05_size_zero_src_null() {
    let s = raw(0, 32, P::B(8), P::Null, 0, "size=0 dest=valid src=NULL");
    assert_same(&s);
}

// ---------------------------------------------------------------------------
// rows 6-7: negative size with dest == src (no memset -> safe)
// ---------------------------------------------------------------------------

#[test] // row 6
fn err_06_negative_size_aliased() {
    let mut rng = Rng::new(0x1006);
    for size in [-1i32, -2, -3, -4, -7, -8, -100, -4096, -65_537] {
        for off in 0..4 {
            let data = gen_data(Dist::FiniteBits, 32, &mut rng);
            assert_same(&Scenario::in_place(&data, off, size));
        }
    }
}

#[test] // row 7
fn err_07_int_min_aliased() {
    let mut rng = Rng::new(0x1007);
    for size in [i32::MIN, i32::MIN + 1, i32::MIN / 2] {
        let data = gen_data(Dist::Wide, 32, &mut rng);
        assert_same(&Scenario::in_place(&data, 0, size));
    }
    // ...and with NULL pointers, where dest == src keeps it a pure no-op
    assert_same(&raw(0, 0, P::Null, P::Null, i32::MIN, "size=INT_MIN both NULL"));
}

// ---------------------------------------------------------------------------
// rows 8-10, 29-30: rows that abort the process -> signal parity in a child
// ---------------------------------------------------------------------------

const CASE_ENV: &str = "NORM_CRASH_CASE";
const WHICH_ENV: &str = "NORM_CRASH_WHICH";

fn spawn_child(case: &str, which: &str) -> std::process::ExitStatus {
    let exe = std::env::current_exe().expect("current_exe");
    Command::new(exe)
        .args(["zz_crash_child", "--exact", "--test-threads=1", "--quiet"])
        .env(CASE_ENV, case)
        .env(WHICH_ENV, which)
        // make sure the child can find both .so files without rebuilding
        .env("NORM_C_SO", c_so_path())
        .env("NORM_RUST_SO", rust_so_path())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("spawn crash child")
}

fn describe(s: std::process::ExitStatus) -> String {
    format!("code={:?} signal={:?}", s.code(), s.signal())
}

/// Assert the C `.so` and the Rust `.so` terminate the process identically for
/// an input that the C code cannot survive.
fn assert_crash_parity(case: &str) {
    let c = spawn_child(case, "c");
    let r = spawn_child(case, "rust");
    assert_eq!(
        (c.code(), c.signal()),
        (r.code(), r.signal()),
        "case `{case}`: C child {} but Rust child {}",
        describe(c),
        describe(r)
    );
    // The C behaviour for every case routed here is death by signal; make sure
    // the test is actually observing that (and not silently passing because
    // both sides returned cleanly).
    assert!(
        c.signal().is_some(),
        "case `{case}`: expected the C child to die from a signal, got {}",
        describe(c)
    );
}

/// Executed only in the forked child (selected by `--exact zz_crash_child`).
/// A no-op unless `NORM_CRASH_CASE` is set.
#[test]
fn zz_crash_child() {
    let case = match std::env::var(CASE_ENV) {
        Ok(c) if !c.is_empty() => c,
        _ => return,
    };
    let which = std::env::var(WHICH_ENV).unwrap_or_default();
    let f = match which.as_str() {
        "c" => c_normalize(),
        "rust" => rust_normalize(),
        other => panic!("bad {WHICH_ENV}={other}"),
    };

    // Small live buffers; every case below deliberately runs off them.
    let mut dst = vec![0.0f32; 64];
    let src: Vec<f32> = (0..64).map(|i| (i as f32) + 1.0).collect();

    unsafe {
        match case.as_str() {
            // row 8: memset length = (size_t)(long)(-1) * 4
            "neg_size_disjoint" => f(dst.as_mut_ptr(), src.as_ptr(), -1),
            // row 9: memset length = (size_t)(long)INT_MIN * 4
            "int_min_disjoint" => f(dst.as_mut_ptr(), src.as_ptr(), i32::MIN),
            // row 10: accumulation loop reads 2^31-1 floats
            "int_max_read" => f(dst.as_mut_ptr(), src.as_ptr(), i32::MAX),
            // row 29: loop #2 writes through NULL
            "null_dest_positive" => f(std::ptr::null_mut(), src.as_ptr(), 8),
            // row 30: loop #1 reads through NULL
            "null_src_positive" => f(dst.as_mut_ptr(), std::ptr::null(), 8),
            other => panic!("unknown crash case `{other}`"),
        }
    }
    // Keep the buffers alive across the call.
    std::hint::black_box((&dst, &src));
    println!("case {case} ({which}) returned without crashing");
}

#[test] // row 8
fn err_08_negative_size_disjoint_crashes() {
    assert_crash_parity("neg_size_disjoint");
}

#[test] // row 9
fn err_09_int_min_disjoint_crashes() {
    assert_crash_parity("int_min_disjoint");
}

#[test] // row 10
fn err_10_int_max_reads_oob_crashes() {
    assert_crash_parity("int_max_read");
}

#[test] // row 29
fn err_29_null_dest_positive_size_crashes() {
    assert_crash_parity("null_dest_positive");
}

#[test] // row 30
fn err_30_null_src_positive_size_crashes() {
    assert_crash_parity("null_src_positive");
}

// ---------------------------------------------------------------------------
// rows 11-14: sum == +0.0f  ->  the zero-fill branch
// ---------------------------------------------------------------------------

#[test] // row 11
fn err_11_all_plus_zero() {
    for &sz in SIZES {
        let data = vec![0.0f32; sz as usize];
        for off in 0..4 {
            assert_same(&Scenario::disjoint(&data, off, sz));
        }
    }
}

#[test] // row 12
fn err_12_all_minus_zero() {
    for &sz in SIZES {
        let data = vec![-0.0f32; sz as usize];
        for off in 0..4 {
            assert_same(&Scenario::disjoint(&data, off, sz));
            assert_same(&Scenario::in_place(&data, off, sz));
        }
        if sz >= 2 {
            assert_same(&Scenario::overlap(&data, 1, sz));
            assert_same(&Scenario::overlap(&data, -1, sz));
        }
    }
}

#[test] // row 13
fn err_13_zero_sum_aliased_no_write() {
    // guard #2 false AND guard #4 false: the buffer must be left byte-identical
    for &sz in SIZES {
        for pat in [0.0f32, -0.0f32] {
            let data = vec![pat; sz as usize];
            for off in 0..4 {
                assert_same(&Scenario::in_place(&data, off, sz));
            }
        }
    }
    // mixture of +0.0 / -0.0, in place
    let mut rng = Rng::new(0x1013);
    for &sz in SIZES {
        let data: Vec<f32> = (0..sz as usize)
            .map(|_| if rng.bool() { 0.0f32 } else { -0.0f32 })
            .collect();
        assert_same(&Scenario::in_place(&data, 0, sz));
    }
}

#[test] // row 14
fn err_14_underflow_to_zero_sum() {
    let mut rng = Rng::new(0x1014);
    for &sz in SIZES {
        for mag in [1e-30f32, 1e-25f32, 1e-24f32, f32::MIN_POSITIVE, 1e-45f32] {
            let data: Vec<f32> = (0..sz as usize)
                .map(|_| if rng.bool() { mag } else { -mag })
                .collect();
            assert_same(&Scenario::disjoint(&data, 0, sz));
            assert_same(&Scenario::in_place(&data, 0, sz));
        }
    }
}

// ---------------------------------------------------------------------------
// rows 15-18: NaN in the input  ->  guard #2 is false (unordered compare)
// ---------------------------------------------------------------------------

fn nan_row(nan_bits: &[u32], seed: u64) {
    let mut rng = Rng::new(seed);
    for &sz in SIZES {
        let n = sz as usize;
        for &nb in nan_bits {
            for _ in 0..4 {
                let mut raw_bits: Vec<u32> = gen_data(Dist::Unit, n, &mut rng)
                    .iter()
                    .map(|v| v.to_bits())
                    .collect();
                let idx = rng.below(n.max(1));
                if idx < raw_bits.len() {
                    raw_bits[idx] = nb;
                }
                let data = bits(&raw_bits);
                assert_same(&Scenario::disjoint(&data, 0, sz));
                assert_same(&Scenario::in_place(&data, 0, sz));
                if n >= 2 {
                    assert_same(&Scenario::overlap(&data, 1, sz));
                    assert_same(&Scenario::overlap(&data, -1, sz));
                }
            }
        }
    }
}

#[test] // row 15
fn err_15_quiet_nan_zero_fill() {
    nan_row(&[QNAN], 0x1015);
}

#[test] // row 16
fn err_16_quiet_nan_aliased_untouched() {
    // dest == src and sum is NaN: guard #4 false, so the NaN payload and sign
    // bit must survive untouched.
    for &sz in SIZES {
        let n = sz as usize;
        for &nb in &[QNAN, NEG_QNAN, SNAN, SNAN_MIN, QNAN_MAX] {
            let data = bits(&vec![nb; n]);
            for off in 0..4 {
                assert_same(&Scenario::in_place(&data, off, sz));
            }
        }
    }
}

#[test] // row 17
fn err_17_signaling_nan() {
    nan_row(&[SNAN, SNAN_MIN], 0x1017);
}

#[test] // row 18
fn err_18_nan_payload_variants() {
    nan_row(&[NEG_QNAN, QNAN_MAX, 0x7F80_00FF, 0xFF80_0001, 0xFFFF_FFFF], 0x1018);
    // several distinct NaNs at once
    let mut rng = Rng::new(0x1118);
    for &sz in SIZES {
        let n = sz as usize;
        let mut b: Vec<u32> = gen_data(Dist::Wide, n, &mut rng).iter().map(|v| v.to_bits()).collect();
        for slot in b.iter_mut() {
            if rng.below(3) == 0 {
                *slot = 0x7F80_0000 | (rng.next_u32() & 0x007F_FFFF).max(1);
            }
        }
        let data = bits(&b);
        assert_same(&Scenario::disjoint(&data, 0, sz));
        assert_same(&Scenario::in_place(&data, 0, sz));
    }
}

// ---------------------------------------------------------------------------
// rows 19-23: infinities and overflow  ->  guard #2 TRUE, scale == +0.0f
// ---------------------------------------------------------------------------

fn inf_row(inf_bits: &[u32], seed: u64) {
    let mut rng = Rng::new(seed);
    for &sz in SIZES {
        let n = sz as usize;
        for &ib in inf_bits {
            for _ in 0..4 {
                let mut b: Vec<u32> = gen_data(Dist::Unit, n, &mut rng)
                    .iter()
                    .map(|v| v.to_bits())
                    .collect();
                let idx = rng.below(n.max(1));
                if idx < b.len() {
                    b[idx] = ib;
                }
                let data = bits(&b);
                assert_same(&Scenario::disjoint(&data, 0, sz));
                assert_same(&Scenario::in_place(&data, 0, sz));
                if n >= 2 {
                    assert_same(&Scenario::overlap(&data, 1, sz));
                    assert_same(&Scenario::overlap(&data, -1, sz));
                }
            }
        }
    }
}

#[test] // row 19
fn err_19_plus_inf() {
    inf_row(&[PINF], 0x1019);
    // all-inf buffers too
    for &sz in SIZES {
        let data = bits(&vec![PINF; sz as usize]);
        assert_same(&Scenario::disjoint(&data, 0, sz));
        assert_same(&Scenario::in_place(&data, 0, sz));
    }
}

#[test] // row 20
fn err_20_minus_inf() {
    inf_row(&[NINF], 0x1020);
    for &sz in SIZES {
        let data = bits(&vec![NINF; sz as usize]);
        assert_same(&Scenario::disjoint(&data, 0, sz));
        assert_same(&Scenario::in_place(&data, 0, sz));
    }
}

#[test] // row 21
fn err_21_both_infs() {
    let mut rng = Rng::new(0x1021);
    for &sz in SIZES {
        let n = sz as usize;
        if n < 2 {
            continue;
        }
        for _ in 0..8 {
            let mut b: Vec<u32> = gen_data(Dist::Unit, n, &mut rng)
                .iter()
                .map(|v| v.to_bits())
                .collect();
            let i = rng.below(n);
            let mut j = rng.below(n);
            if j == i {
                j = (i + 1) % n;
            }
            b[i] = PINF;
            b[j] = NINF;
            let data = bits(&b);
            assert_same(&Scenario::disjoint(&data, 0, sz));
            assert_same(&Scenario::in_place(&data, 0, sz));
            assert_same(&Scenario::overlap(&data, 1, sz));
        }
    }
}

#[test] // row 22
fn err_22_inf_and_nan() {
    let mut rng = Rng::new(0x1022);
    for &sz in SIZES {
        let n = sz as usize;
        if n < 2 {
            continue;
        }
        for &nb in &[QNAN, SNAN, NEG_QNAN] {
            for &ib in &[PINF, NINF] {
                let mut b: Vec<u32> = gen_data(Dist::Unit, n, &mut rng)
                    .iter()
                    .map(|v| v.to_bits())
                    .collect();
                let i = rng.below(n);
                let mut j = rng.below(n);
                if j == i {
                    j = (i + 1) % n;
                }
                b[i] = ib;
                b[j] = nb;
                let data = bits(&b);
                assert_same(&Scenario::disjoint(&data, 0, sz));
                assert_same(&Scenario::in_place(&data, 0, sz));
            }
        }
    }
}

#[test] // row 23
fn err_23_sum_overflow_to_inf() {
    let mut rng = Rng::new(0x1023);
    for &sz in SIZES {
        let n = sz as usize;
        for mag in [1e30f32, 1e38f32, f32::MAX, 1e20f32, 3.0e19f32] {
            let data: Vec<f32> = (0..n).map(|_| if rng.bool() { mag } else { -mag }).collect();
            assert_same(&Scenario::disjoint(&data, 0, sz));
            assert_same(&Scenario::in_place(&data, 0, sz));
            if n >= 2 {
                assert_same(&Scenario::overlap(&data, 1, sz));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// row 24: sum lands in the subnormal range
// ---------------------------------------------------------------------------

#[test] // row 24
fn err_24_subnormal_sum() {
    // x^2 subnormal  <=>  |x| roughly in [1.1e-23, 1.1e-19]
    let mags = [
        1.5e-22f32, 1.0e-23f32, 3.0e-23f32, 1.1e-22f32, 5.0e-22f32, 9.9e-23f32, 2.0e-21f32,
        1.0e-20f32,
    ];
    let mut rng = Rng::new(0x1024);
    for &sz in SIZES {
        let n = sz as usize;
        for &m in &mags {
            let data: Vec<f32> = (0..n).map(|_| if rng.bool() { m } else { -m }).collect();
            assert_same(&Scenario::disjoint(&data, 0, sz));
            assert_same(&Scenario::in_place(&data, 0, sz));
        }
        // random subnormal-producing magnitudes
        for _ in 0..8 {
            let data: Vec<f32> = (0..n).map(|_| rng.scaled(-76, -64)).collect();
            assert_same(&Scenario::disjoint(&data, 0, sz));
        }
    }
    // the single-element case, where the scale is exactly 1/sqrt(x^2)
    for &m in &mags {
        assert_same(&Scenario::disjoint(&[m], 0, 1));
        assert_same(&Scenario::disjoint(&[-m], 0, 1));
        assert_same(&Scenario::in_place(&[m], 0, 1));
    }
    // every subnormal-square magnitude by raw bit pattern
    let mut rng = Rng::new(0x1124);
    for _ in 0..2000 {
        // exponent field 0x2C..0x3F keeps x^2 subnormal
        let e = 0x28u32 + rng.below(0x18) as u32;
        let b = (rng.next_u32() & 0x8000_0000) | (e << 23) | (rng.next_u32() & 0x007F_FFFF);
        let x = f32::from_bits(b);
        assert_same(&Scenario::disjoint(&[x], 0, 1));
        assert_same(&Scenario::disjoint(&[x, x], 0, 2));
    }
}

// ---------------------------------------------------------------------------
// row 25: the `size` int has no valid-variant restriction - sweep it
// ---------------------------------------------------------------------------

#[test] // row 25
fn err_25_random_int_sweep() {
    // There is no enum in this API; the only scalar parameter is `int size`.
    // Sweep the whole non-positive half of `int` (safe: guard #1 skips both
    // loops and, with dest == src, guard #4 skips the memset), plus every
    // valid positive size up to the buffer length.
    let mut rng = Rng::new(0x1025);
    let data = gen_data(Dist::FiniteBits, 32, &mut rng);

    // hand-picked boundaries
    for size in [
        i32::MIN,
        i32::MIN + 1,
        -65_537,
        -65_536,
        -1024,
        -33,
        -32,
        -2,
        -1,
        0,
    ] {
        assert_same(&Scenario::in_place(&data, 0, size));
        assert_same(&raw(0, 0, P::Null, P::Null, size, "null/null sweep"));
    }
    // random negatives
    for _ in 0..3000 {
        let v = (rng.next_u32() as i32).min(0);
        let size = if v == 0 { i32::MIN } else { v };
        assert_same(&Scenario::in_place(&data, 0, size));
    }
    // random valid positives, with the buffer sized to match
    for _ in 0..1500 {
        let n = 1 + rng.below(96);
        let d = gen_data(Dist::FiniteBits, n, &mut rng);
        assert_same(&Scenario::disjoint(&d, rng.below(4), n as c_int));
    }
    // one step past a valid size, staying inside the allocation: `size` may
    // legally reach into the trailing guard region, so both sides must read and
    // write exactly the same extended window.
    for _ in 0..1500 {
        let n = 1 + rng.below(64);
        let d = gen_data(Dist::Unit, n, &mut rng);
        let extra = 1 + rng.below(GUARD);
        assert_same(&Scenario::disjoint(&d, 0, (n + extra) as c_int));
    }
}

// ---------------------------------------------------------------------------
// rows 26-28: partial overlap
// ---------------------------------------------------------------------------

#[test] // row 26
fn err_26_overlap_forward() {
    let mut rng = Rng::new(0x1026);
    for &sz in SIZES {
        let n = sz as usize;
        if n < 2 {
            continue;
        }
        for dist in [Dist::Unit, Dist::Wide, Dist::FiniteBits, Dist::Pow2] {
            for _ in 0..6 {
                let d = gen_data(dist, n, &mut rng);
                let k = 1 + rng.below(n - 1);
                assert_same(&Scenario::overlap(&d, k as isize, sz));
            }
        }
    }
}

#[test] // row 27
fn err_27_overlap_backward() {
    let mut rng = Rng::new(0x1027);
    for &sz in SIZES {
        let n = sz as usize;
        if n < 2 {
            continue;
        }
        for dist in [Dist::Unit, Dist::Wide, Dist::FiniteBits, Dist::Pow2] {
            for _ in 0..6 {
                let d = gen_data(dist, n, &mut rng);
                let k = 1 + rng.below(n - 1);
                assert_same(&Scenario::overlap(&d, -(k as isize), sz));
            }
        }
    }
}

#[test] // row 28
fn err_28_overlap_zero_fill() {
    // sum <= 0 with overlapping (but unequal) pointers: guard #4 is TRUE, so
    // the memset stomps part of `src`.
    let mut rng = Rng::new(0x1028);
    for &sz in SIZES {
        let n = sz as usize;
        if n < 2 {
            continue;
        }
        let zeros = vec![0.0f32; n];
        let neg_zeros = vec![-0.0f32; n];
        let nans = bits(&vec![QNAN; n]);
        let snans = bits(&vec![SNAN; n]);
        for d in [&zeros, &neg_zeros, &nans, &snans] {
            for _ in 0..4 {
                let k = 1 + rng.below(n - 1);
                assert_same(&Scenario::overlap(d, k as isize, sz));
                assert_same(&Scenario::overlap(d, -(k as isize), sz));
            }
        }
        // NaN in a random slot with an overlap
        for _ in 0..6 {
            let mut b: Vec<u32> = gen_data(Dist::Unit, n, &mut rng)
                .iter()
                .map(|v| v.to_bits())
                .collect();
            b[rng.below(n)] = QNAN;
            let d = bits(&b);
            let k = 1 + rng.below(n - 1);
            assert_same(&Scenario::overlap(&d, k as isize, sz));
            assert_same(&Scenario::overlap(&d, -(k as isize), sz));
        }
    }
}
