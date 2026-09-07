//! Phase C -- error-path differential tests, one test per ERRORS.md row.
//!
//! `normalize` returns `void` and validates nothing, so the "error surface" is
//! (a) the branch it selects for degenerate inputs and (b) how the process dies
//! for the inputs that are outright UB. Both are compared differentially: the
//! benign rows compare resulting memory byte-for-byte, the UB rows compare the
//! exact termination status of a forked child per side.

mod common;

use common::*;
use std::ffi::c_int;

// =========================================================== E1
// size == 0, dest != src -> memset(dest, 0, 0) -> strict no-op.
#[test]
fn e1_size_zero_dest_ne_src() {
    let l = build_layout(Placement::Separate, 0, &[], 0);
    let (c, r) = run_both(&l);
    assert_same("E1", &l, &c, &r);
    assert_eq!(c, l.words, "C: 0-length memset must not modify any byte");
    assert_eq!(r, l.words, "RUST: 0-length memset must not modify any byte");
}

// =========================================================== E2
// size == 0, dest == src -> both branches skipped.
#[test]
fn e2_size_zero_dest_eq_src() {
    let l = build_layout(Placement::Aliased, 0, &[], 0);
    let (c, r) = run_both(&l);
    assert_same("E2", &l, &c, &r);
    assert_eq!(c, l.words);
    assert_eq!(r, l.words);
}

// =========================================================== E3
// size < 0 with dest == src: loop skipped, memset guarded away -> no-op.
#[test]
fn e3_negative_size_dest_eq_src() {
    let f = libs();
    for &size in &[-1i32, -2, -7, -1000, -0x4000_0000, i32::MIN] {
        // A real, populated buffer so a stray write would be visible.
        let mut rng = Rng::new(SEED ^ size as u64);
        let vals = gen_values(ValueClass::Mixed, 32, &mut rng);
        let template = build_layout(Placement::Aliased, 32, &vals, 0).words;

        let cb = AlignedBuf::from_words(&template);
        let p = cb.word_ptr(GUARD_WORDS);
        unsafe { (f.c)(p, p as *const f32, size as c_int) };
        let cout = cb.words();

        let rb = AlignedBuf::from_words(&template);
        let p = rb.word_ptr(GUARD_WORDS);
        unsafe { (f.rust)(p, p as *const f32, size as c_int) };
        let rout = rb.words();

        assert_eq!(cout, rout, "E3 divergence for size={size}");
        assert_eq!(cout, template, "C: negative size with dest==src must be a no-op (size={size})");
        assert_eq!(rout, template, "RUST: negative size with dest==src must be a no-op (size={size})");
    }
}

// =========================================================== E4
// size < 0 with dest != src: memset length is (size_t)size * 4 wrapped
// -> ~2^64 bytes -> the process must fault, identically on both sides.
#[test]
fn e4_negative_size_dest_ne_src_faults() {
    if let Some(side) = fault_side() {
        let n = side_fn(&side);
        let dst = AlignedBuf::new(4096);
        let src = AlignedBuf::new(4096);
        // dest != src, sum stays 0 because the loop body never executes.
        unsafe { n(dst.word_ptr(0), src.word_ptr(0) as *const f32, -1 as c_int) };
        // If we somehow survive, exit with a distinctive code so the parent
        // still sees the SAME status from both sides or a real mismatch.
        eprintln!("survived");
        std::process::exit(77);
    }
    assert_fault_parity("e4_negative_size_dest_ne_src_faults");
}

// Same row, other negative magnitudes.
#[test]
fn e4b_int_min_size_dest_ne_src_faults() {
    if let Some(side) = fault_side() {
        let n = side_fn(&side);
        let dst = AlignedBuf::new(4096);
        let src = AlignedBuf::new(4096);
        unsafe { n(dst.word_ptr(0), src.word_ptr(0) as *const f32, i32::MIN as c_int) };
        eprintln!("survived");
        std::process::exit(77);
    }
    assert_fault_parity("e4b_int_min_size_dest_ne_src_faults");
}

// =========================================================== E5
// Zero-norm vector, dest != src -> memset -> every element becomes +0.0
// (never -0.0), regardless of the input's sign bits.
#[test]
fn e5_zero_norm_memset() {
    let mut rng = Rng::new(SEED ^ 0xE5);
    for &size in SIZES {
        if size < 1 {
            continue;
        }
        for class in [ValueClass::PositiveZero, ValueClass::NegativeZero, ValueClass::MixedZero] {
            for _ in 0..16 {
                let vals = gen_values(class, size as usize, &mut rng);
                let l = build_layout(Placement::Separate, size, &vals, 0);
                let (c, r) = run_both(&l);
                assert_same("E5", &l, &c, &r);
                for i in 0..size as usize {
                    assert_eq!(
                        c[l.dest_word + i], 0u32,
                        "C: memset branch must produce +0.0 bits (size={size}, i={i})"
                    );
                }
            }
        }
    }
}

// =========================================================== E6
// Zero-norm vector, dest == src -> no-op, -0.0 signs survive in place.
#[test]
fn e6_zero_norm_aliased_noop() {
    let mut rng = Rng::new(SEED ^ 0xE6);
    for &size in SIZES {
        if size < 1 {
            continue;
        }
        for class in [ValueClass::NegativeZero, ValueClass::MixedZero] {
            for _ in 0..16 {
                let vals = gen_values(class, size as usize, &mut rng);
                let l = build_layout(Placement::Aliased, size, &vals, 0);
                let (c, r) = run_both(&l);
                assert_same("E6", &l, &c, &r);
                assert_eq!(c, l.words, "C: aliased zero-norm must be a strict no-op");
                assert_eq!(r, l.words, "RUST: aliased zero-norm must be a strict no-op");
            }
        }
    }
}

// =========================================================== E7
// NaN in src -> sum is NaN -> `NaN > 0.0f` false -> memset branch.
#[test]
fn e7_nan_sum_memset() {
    for &nan in &[
        0x7FC0_0000u32,
        0xFFC0_0000,
        0x7F80_0001,
        0xFF80_0001,
        0x7FFF_FFFF,
        0xFFFF_FFFF,
        0x7FBF_FFFF,
    ] {
        for &size in &[1i32, 2, 3, 4, 5, 8, 16, 17, 33, 64, 1000] {
            for pos in [0usize, (size as usize) / 2, size as usize - 1] {
                let mut vals: Vec<u32> =
                    (0..size as usize).map(|i| (i as f32 + 1.25).to_bits()).collect();
                vals[pos] = nan;
                let l = build_layout(Placement::Separate, size, &vals, 0);
                let (c, r) = run_both(&l);
                assert_same(&format!("E7 nan=0x{nan:08X} size={size} pos={pos}"), &l, &c, &r);
                for i in 0..size as usize {
                    assert_eq!(
                        c[l.dest_word + i], 0u32,
                        "C: NaN sum must take the memset branch (nan=0x{nan:08X}, i={i})"
                    );
                }
            }
        }
    }
}

// =========================================================== E8
// NaN in src, dest == src -> no-op, NaN payload preserved.
#[test]
fn e8_nan_sum_aliased_noop() {
    for &nan in &[0x7FC0_0000u32, 0xFF80_0001, 0x7FFF_FFFF] {
        for &size in &[1i32, 2, 4, 7, 16, 33, 1000] {
            for pos in [0usize, size as usize - 1] {
                let mut vals: Vec<u32> =
                    (0..size as usize).map(|i| (i as f32 - 3.5).to_bits()).collect();
                vals[pos] = nan;
                let l = build_layout(Placement::Aliased, size, &vals, 0);
                let (c, r) = run_both(&l);
                assert_same("E8", &l, &c, &r);
                assert_eq!(c, l.words, "C: aliased NaN must be a strict no-op");
                assert_eq!(r, l.words, "RUST: aliased NaN must be a strict no-op");
            }
        }
    }
}

// =========================================================== E9
// sum overflows to +inf -> scale branch with 1/inf == +0.0.
#[test]
fn e9_inf_sum_scale_to_zero() {
    let big = f32::MAX.to_bits();
    for &size in SIZES {
        if size < 1 {
            continue;
        }
        let vals = vec![big; size as usize];
        let l = build_layout(Placement::Separate, size, &vals, 0);
        let (c, r) = run_both(&l);
        assert_same("E9-max-pos", &l, &c, &r);
        for i in 0..size as usize {
            assert_eq!(
                c[l.dest_word + i], 0u32,
                "C: FLT_MAX * (1/inf) must be +0.0 (i={i})"
            );
        }

        let vals = vec![big | 0x8000_0000; size as usize];
        let l = build_layout(Placement::Separate, size, &vals, 0);
        let (c, r) = run_both(&l);
        assert_same("E9-max-neg", &l, &c, &r);
        for i in 0..size as usize {
            assert_eq!(
                c[l.dest_word + i], 0x8000_0000u32,
                "C: -FLT_MAX * (1/inf) must be -0.0 (i={i})"
            );
        }
    }
}

// =========================================================== E10
// +-inf element -> sum == +inf -> inf * 0.0 == NaN in that lane.
#[test]
fn e10_infinite_element() {
    for &inf in &[0x7F80_0000u32, 0xFF80_0000] {
        for &size in &[1i32, 2, 3, 4, 5, 8, 16, 17, 33, 64, 1000] {
            for pos in [0usize, (size as usize) / 2, size as usize - 1] {
                let mut vals: Vec<u32> =
                    (0..size as usize).map(|i| (i as f32 + 2.5).to_bits()).collect();
                vals[pos] = inf;
                let l = build_layout(Placement::Separate, size, &vals, 0);
                let (c, r) = run_both(&l);
                assert_same(&format!("E10 inf=0x{inf:08X} size={size} pos={pos}"), &l, &c, &r);
                // The inf lane becomes NaN, the finite lanes signed zeros.
                assert!(
                    f32::from_bits(c[l.dest_word + pos]).is_nan(),
                    "C: inf lane must become NaN"
                );
                for i in 0..size as usize {
                    if i == pos {
                        continue;
                    }
                    assert_eq!(
                        c[l.dest_word + i] & 0x7FFF_FFFF,
                        0u32,
                        "C: finite lane must become a signed zero (i={i})"
                    );
                }
            }
        }
        // Also the aliased variant (still takes the scale branch since sum > 0).
        for &size in &[1i32, 4, 17, 64] {
            let mut vals: Vec<u32> = (0..size as usize).map(|i| (i as f32 + 2.5).to_bits()).collect();
            vals[0] = inf;
            let l = build_layout(Placement::Aliased, size, &vals, 0);
            let (c, r) = run_both(&l);
            assert_same("E10-aliased", &l, &c, &r);
        }
    }
}

// =========================================================== E11
// Denormal / underflowing sums: whichever branch `> 0.0f` picks, both sides
// must pick the same one.
#[test]
fn e11_denormal_sum() {
    let probes: &[u32] = &[
        1u32,                             // 1e-45, smallest subnormal
        2,
        0x0000_00FF,
        0x007F_FFFF,                      // largest subnormal
        0x0080_0000,                      // FLT_MIN
        0x0080_0001,
        (1e-20f32).to_bits(),             // square == 1e-40 (subnormal)
        (1e-22f32).to_bits(),             // square == 1e-44 (subnormal)
        (1e-23f32).to_bits(),             // square underflows to +0
        (3e-23f32).to_bits(),
        (1e-30f32).to_bits(),
        (f32::MIN_POSITIVE * 2.0).to_bits(),
    ];
    for &v in probes {
        for &size in SIZES {
            if size < 1 {
                continue;
            }
            for &sign in &[0u32, 0x8000_0000u32] {
                let vals = vec![v | sign; size as usize];
                let l = build_layout(Placement::Separate, size, &vals, 0);
                let (c, r) = run_both(&l);
                assert_same(
                    &format!("E11 v=0x{v:08X} sign=0x{sign:08X} size={size}"),
                    &l,
                    &c,
                    &r,
                );
                let l = build_layout(Placement::Aliased, size, &vals, 0);
                let (c, r) = run_both(&l);
                assert_same("E11-aliased", &l, &c, &r);
            }
        }
    }

    // Randomized subnormal sweep.
    let mut rng = Rng::new(SEED ^ 0xE11);
    for &size in SIZES {
        for _ in 0..64 {
            let vals = gen_values(ValueClass::Denormal, size as usize, &mut rng);
            check("E11-random", Placement::Separate, size, &vals, 0);
        }
    }
}

// =========================================================== E12
// Null pointers with size > 0 -> immediate dereference -> fault on both sides.
#[test]
fn e12_null_pointer_size_positive_faults() {
    if let Some(side) = fault_side() {
        let n = side_fn(&side);
        unsafe { n(std::ptr::null_mut(), std::ptr::null(), 8 as c_int) };
        eprintln!("survived");
        std::process::exit(77);
    }
    assert_fault_parity("e12_null_pointer_size_positive_faults");
}

#[test]
fn e12b_null_src_only_faults() {
    if let Some(side) = fault_side() {
        let n = side_fn(&side);
        let dst = AlignedBuf::new(4096);
        unsafe { n(dst.word_ptr(0), std::ptr::null(), 8 as c_int) };
        eprintln!("survived");
        std::process::exit(77);
    }
    assert_fault_parity("e12b_null_src_only_faults");
}

#[test]
fn e12c_null_dest_scale_branch_faults() {
    if let Some(side) = fault_side() {
        let n = side_fn(&side);
        // src has a positive norm -> the scale branch writes through NULL.
        let src = AlignedBuf::from_words(&[1.0f32.to_bits(); 8]);
        unsafe { n(std::ptr::null_mut(), src.word_ptr(0) as *const f32, 8 as c_int) };
        eprintln!("survived");
        std::process::exit(77);
    }
    assert_fault_parity("e12c_null_dest_scale_branch_faults");
}

#[test]
fn e12d_null_dest_memset_branch_faults() {
    if let Some(side) = fault_side() {
        let n = side_fn(&side);
        // src is all zeros -> sum == 0 -> memset(NULL, 0, 32).
        let src = AlignedBuf::from_words(&[0u32; 8]);
        unsafe { n(std::ptr::null_mut(), src.word_ptr(0) as *const f32, 8 as c_int) };
        eprintln!("survived");
        std::process::exit(77);
    }
    assert_fault_parity("e12d_null_dest_memset_branch_faults");
}

// =========================================================== E13
// Both pointers null with size == 0: `dest != src` is false (NULL == NULL),
// so nothing is dereferenced and nothing is memset -> returns normally.
#[test]
fn e13_null_pointers_size_zero_ok() {
    let f = libs();
    unsafe { (f.c)(std::ptr::null_mut(), std::ptr::null(), 0 as c_int) };
    unsafe { (f.rust)(std::ptr::null_mut(), std::ptr::null(), 0 as c_int) };
    // Reaching here at all is the assertion: both returned without faulting.
}

// Both pointers null with a NEGATIVE size: still `dest == src`, still benign.
#[test]
fn e13b_null_pointers_negative_size_ok() {
    let f = libs();
    for &size in &[-1i32, -1000, i32::MIN] {
        unsafe { (f.c)(std::ptr::null_mut(), std::ptr::null(), size as c_int) };
        unsafe { (f.rust)(std::ptr::null_mut(), std::ptr::null(), size as c_int) };
    }
}

// =========================================================== E14
// dest == NULL, src valid, size == 0 -> `dest != src` true -> memset(NULL,0,0).
#[test]
fn e14_null_dest_size_zero() {
    let f = libs();
    let src = AlignedBuf::from_words(&[0u32; 8]);
    unsafe { (f.c)(std::ptr::null_mut(), src.word_ptr(0) as *const f32, 0 as c_int) };
    unsafe { (f.rust)(std::ptr::null_mut(), src.word_ptr(0) as *const f32, 0 as c_int) };

    // Mirror: src == NULL, dest valid, size == 0.
    let dst = AlignedBuf::from_words(&[0xAAAA_AAAAu32; 8]);
    let before = dst.words();
    unsafe { (f.c)(dst.word_ptr(0), std::ptr::null(), 0 as c_int) };
    let after_c = dst.words();
    assert_eq!(before, after_c, "C: size==0 must not write to dest");

    let dst = AlignedBuf::from_words(&[0xAAAA_AAAAu32; 8]);
    unsafe { (f.rust)(dst.word_ptr(0), std::ptr::null(), 0 as c_int) };
    assert_eq!(before, dst.words(), "RUST: size==0 must not write to dest");
}

// =========================================================== E15
// Extreme / out-of-domain `int` values in the only scalar parameter.
// (This API has no enum type; `int size` is the whole scalar domain, and C
// accepts every one of the 2^32 values.)
#[test]
fn e15_extreme_int_sizes() {
    let f = libs();
    // Benign half: every negative/extreme value with dest == src is a no-op.
    for &size in &[
        i32::MIN,
        i32::MIN + 1,
        -0x7FFF_FFFF,
        -65536,
        -256,
        -2,
        -1,
        0,
    ] {
        let mut rng = Rng::new(SEED ^ 0xE15 ^ size as u64);
        let vals = gen_values(ValueClass::Mixed, 64, &mut rng);
        let template = build_layout(Placement::Aliased, 64, &vals, 0).words;

        let cb = AlignedBuf::from_words(&template);
        let p = cb.word_ptr(GUARD_WORDS);
        unsafe { (f.c)(p, p as *const f32, size as c_int) };
        let cout = cb.words();

        let rb = AlignedBuf::from_words(&template);
        let p = rb.word_ptr(GUARD_WORDS);
        unsafe { (f.rust)(p, p as *const f32, size as c_int) };
        let rout = rb.words();

        assert_eq!(cout, rout, "E15 divergence for size={size}");
        assert_eq!(cout, template, "C: size={size} with dest==src must be a no-op");
    }

    // Null pointers with every extreme value: dest == src == NULL, benign.
    for &size in &[i32::MIN, -1, 0] {
        unsafe { (f.c)(std::ptr::null_mut(), std::ptr::null(), size as c_int) };
        unsafe { (f.rust)(std::ptr::null_mut(), std::ptr::null(), size as c_int) };
    }
}

// INT_MAX with dest == src: 2^31-1 out-of-bounds reads -> must fault the same.
#[test]
fn e15b_int_max_size_faults() {
    if let Some(side) = fault_side() {
        let n = side_fn(&side);
        let buf = AlignedBuf::new(4096);
        let p = buf.word_ptr(0);
        unsafe { n(p, p as *const f32, i32::MAX as c_int) };
        eprintln!("survived");
        std::process::exit(77);
    }
    assert_fault_parity("e15b_int_max_size_faults");
}

// One step past a valid range: size == actual_len + 1 (reads one element OOB).
// Both sides must read the same adjacent word and therefore agree exactly.
#[test]
fn e15c_size_one_past_the_buffer() {
    let f = libs();
    for &len in &[1usize, 2, 3, 4, 7, 8, 15, 16, 17, 63, 64] {
        // Over-allocate so the +1 read is defined for the test process, but the
        // library is told a size one larger than the "logical" buffer.
        let mut words: Vec<u32> = (0..len + 8).map(|i| (i as f32 + 1.0).to_bits()).collect();
        words[len] = (0.25f32).to_bits(); // the one-past element both must read

        let cb = AlignedBuf::from_words(&words);
        unsafe { (f.c)(cb.word_ptr(0), cb.word_ptr(0) as *const f32, (len + 1) as c_int) };
        let cout = cb.words();

        let rb = AlignedBuf::from_words(&words);
        unsafe { (f.rust)(rb.word_ptr(0), rb.word_ptr(0) as *const f32, (len + 1) as c_int) };
        assert_eq!(cout, rb.words(), "E15c divergence at len={len}");
    }
}
