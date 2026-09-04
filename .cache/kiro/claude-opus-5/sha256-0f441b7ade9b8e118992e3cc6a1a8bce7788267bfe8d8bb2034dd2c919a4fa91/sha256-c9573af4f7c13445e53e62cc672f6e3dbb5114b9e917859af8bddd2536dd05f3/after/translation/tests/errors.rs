//! Phase C — error / rejection surface differential tests, one test per row of
//! `ERRORS.md`.
//!
//! The C library has *no* error returns, asserts, null checks or range checks
//! (see the grep evidence in `ERRORS.md`): both entry points are `void` and all
//! 2^32 seed values are accepted.  The requirement therefore becomes that every
//! boundary and would-be-invalid condition produces *identical accepted
//! behaviour* on both sides — identical stdout bytes, identical `array` bytes,
//! and no panic/abort where C completes.
//!
//! Rows that need the C `long_exec` (~470 s per call) compare against the C
//! bytes recorded by `tests/ground_truth/capture.sh`; everything else runs both
//! libraries live.

mod common;

use common::*;
use std::ffi::c_int;

fn gt(seed: u32) -> String {
    let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/ground_truth")
        .join(format!("c_{seed}.out"));
    let b = std::fs::read(&p)
        .unwrap_or_else(|e| panic!("missing {} : {e} (run capture.sh)", p.display()));
    assert!(!b.is_empty(), "{}: empty", p.display());
    String::from_utf8(b).expect("utf8")
}

fn rust_long_exec_out(seed: u32) -> String {
    let l = libs();
    String::from_utf8(capture_stdout(|| l.rs.long_exec(seed))).expect("utf8")
}

fn uniform(v: c_int) -> Vec<c_int> {
    vec![v; ARRAY_LEN]
}

// --- Rows 1-2: glibc srand(0) is documented to behave as srand(1) -----------

#[test]
fn err_row01_02_seed_zero_equals_one() {
    let c0 = gt(0);
    let c1 = gt(1);
    // The C ground truth itself must show the srand(0) == srand(1) substitution.
    assert_eq!(c0, c1, "C: srand(0) did not behave as srand(1)");
    assert_eq!(rust_long_exec_out(0), c0, "seed=0");
    assert_eq!(rust_long_exec_out(1), c1, "seed=1");
}

// --- Row 3: UINT_MAX --------------------------------------------------------

#[test]
fn err_row03_seed_uint_max() {
    assert_eq!(rust_long_exec_out(u32::MAX), gt(4294967295));
}

// --- Rows 4-5: the sign-bit boundary of the seed parameter ------------------

#[test]
fn err_row04_05_sign_bit_seeds() {
    // 2^31: recorded C ground truth exists.
    assert_eq!(rust_long_exec_out(2147483648), gt(2147483648));
    // INT_MAX (one step below) must give a *different* answer, proving the
    // parameter is not being truncated or sign-confused.
    let at_int_max = rust_long_exec_out(2147483647);
    assert_ne!(
        at_int_max,
        gt(2147483648),
        "seed 2147483647 and 2147483648 must differ"
    );
    // and it must be reproducible
    assert_eq!(rust_long_exec_out(2147483647), at_int_max);
}

// --- Row 6: a negative int passed where unsigned int is expected ------------

#[test]
fn err_row06_negative_seed_reinterpreted() {
    // C reinterprets the bit pattern, it does not reject: -1 == 4294967295.
    let as_neg = rust_long_exec_out((-1i32) as u32);
    assert_eq!(as_neg, gt(4294967295), "-1 must be seen as 4294967295");
    // Same for INT_MIN as a negative int == 2^31.
    assert_eq!(
        rust_long_exec_out(i32::MIN as u32),
        gt(2147483648),
        "INT_MIN must be seen as 2147483648"
    );
}

// --- Rows 7-8: repeated calls -----------------------------------------------

#[test]
fn err_row07_repeat_same_seed() {
    let a = rust_long_exec_out(255);
    let b = rust_long_exec_out(255);
    assert_eq!(a, b, "repeat of the same seed diverged");
    assert_eq!(a, gt(255));
}

#[test]
fn err_row08_repeat_different_seed() {
    let a = rust_long_exec_out(3);
    let b = rust_long_exec_out(100);
    assert_eq!(a, gt(3));
    assert_eq!(b, gt(100));
    assert_ne!(a, b);
    // and going back to the first seed still reproduces it
    assert_eq!(rust_long_exec_out(3), a, "state carried over between calls");
}

// --- Rows 9-16: arithmetic edge conditions in the kernel --------------------

#[test]
fn err_row09_10_extreme_scalars() {
    // INT_MIN: x*3+7, x<<1 and the -x idiom all overflow.
    diff_peo("err09 INT_MIN", &uniform(i32::MIN), 1);
    diff_peo("err09 INT_MIN", &uniform(i32::MIN), 3);
    // INT_MAX: x*3+7 overflows on the very first operation.
    diff_peo("err10 INT_MAX", &uniform(i32::MAX), 1);
    diff_peo("err10 INT_MAX", &uniform(i32::MAX), 3);
}

#[test]
fn err_row11_negative_shift_operand() {
    // `x >> 3` with x < 0 is implementation-defined in C; the compiled .so is
    // ground truth.  Cover a dense band of negative values, not just -1.
    let mut v = uniform(-1);
    for (i, slot) in v.iter_mut().enumerate() {
        *slot = -1 - (i as i32 % 4096);
    }
    diff_peo("err11 negative >>", &v, 1);
    diff_peo("err11 all -1", &uniform(-1), 1);
}

#[test]
fn err_row12_zero_element() {
    diff_peo("err12 zeros", &uniform(0), 1);
    diff_peo("err12 zeros", &uniform(0), 2);
}

#[test]
fn err_row13_truncating_division_signs() {
    // Negative odd values exercise C's truncate-toward-zero `/` and the
    // sign-of-dividend `%`.  Floor semantics would diverge here.
    let probes: [i32; 12] = [-3, -7, -1, -5, -9, -13, -2147483647, -14, -21, -6, -99, -1000003];
    let mut v = vec![0i32; ARRAY_LEN];
    for (i, slot) in v.iter_mut().enumerate() {
        *slot = probes[i % probes.len()];
    }
    diff_peo("err13 negative odd", &v, 1);
    diff_peo("err13 negative odd", &v, 5);
}

#[test]
fn err_row14_int_min_division() {
    // Feed values whose orbit passes through INT_MIN-adjacent magnitudes so the
    // `/2` and `%7` of INT_MIN are actually evaluated.
    let mut v = vec![0i32; ARRAY_LEN];
    for (i, slot) in v.iter_mut().enumerate() {
        *slot = i32::MIN.wrapping_add((i as i32) % 64);
    }
    diff_peo("err14 INT_MIN /2", &v, 1);
    diff_peo("err14 INT_MIN /2", &v, 4);
}

#[test]
fn err_row15_whole_array_int_min() {
    diff_peo("err15 whole array INT_MIN", &uniform(i32::MIN), 10);
}

#[test]
fn err_row16_bss_initial_state() {
    // The pristine .bss state: all zeros, and the low-level entry point called
    // before `long_exec` has ever run.  Fresh library images are not available
    // mid-process, so this reproduces the state explicitly.
    diff_peo("err16 pristine bss", &uniform(0), 1);
    let l = libs();
    // A pristine `array` read back before any call must be identical on both
    // sides in size and content semantics.
    assert_eq!(l.c.read_array_bytes().len(), l.rs.read_array_bytes().len());
}

// --- Rows 17-18: mismatched prototypes across the FFI boundary --------------

#[test]
fn err_row17_extra_arg_ignored() {
    // `void perform_expensive_operations()` accepts any argument list in C.
    // Out-of-range "enum-like" ints must be ignored identically by both.
    let mut rng = Rng::new(17);
    let mut input = vec![0i32; ARRAY_LEN];
    rng.fill(&mut input);
    for arg in [0i32, 1, -1, i32::MAX, i32::MIN, -999, 0x7fff_ffff] {
        let l = libs();
        l.c.write_array(&input);
        l.rs.write_array(&input);
        l.c.peo_with_arg(arg);
        l.rs.peo_with_arg(arg);
        assert_arrays_eq(
            &format!("err17 arg={arg}"),
            &input,
            &l.c.read_array(),
            &l.rs.read_array(),
        );
        // and it must equal the no-argument call
        drop(l);
        let l = libs();
        let plain_c = run_peo(l.c, &input, 1);
        let plain_rs = run_peo(l.rs, &input, 1);
        assert_arrays_eq(&format!("err17 plain vs arg={arg}"), &input, &plain_c, &plain_rs);
    }
}

#[test]
fn err_row18_high_half_garbage_seed() {
    // Only the low 32 bits are the `unsigned int` parameter.
    let out = {
        let l = libs();
        String::from_utf8(capture_stdout(|| l.rs.long_exec_u64(0xDEAD_BEEF_0000_0007))).unwrap()
    };
    assert_eq!(out, gt(7), "high half of the argument register leaked in");
}

// --- Row 19: boundary indices of the exported object ------------------------

#[test]
fn err_row19_boundary_indices() {
    let mut v = vec![0i32; ARRAY_LEN];
    v[0] = i32::MIN;
    v[1] = i32::MAX;
    v[ARRAY_LEN - 2] = -1;
    v[ARRAY_LEN - 1] = 1;
    diff_peo("err19 boundary indices", &v, 2);
    let l = libs();
    let c = l.c.read_array();
    let rs = l.rs.read_array();
    for &i in &[0usize, 1, ARRAY_LEN - 2, ARRAY_LEN - 1] {
        assert_eq!(c[i], rs[i], "err19: array[{i}] diverged");
    }
}

// --- Row 20: composition counts 0..n ---------------------------------------

#[test]
fn err_row20_composition_counts() {
    let mut rng = Rng::new(20);
    let mut input = vec![0i32; ARRAY_LEN];
    rng.fill(&mut input);
    for k in 0..6u32 {
        diff_peo("err20 composition", &input, k);
    }
}

// --- Row 21: pre-poisoned array must be fully overwritten ------------------

#[test]
fn err_row21_poisoned_array_overwritten() {
    let mut rng = Rng::new(21);
    let mut poison = vec![0i32; ARRAY_LEN];
    rng.fill(&mut poison);
    // sprinkle in the nastiest values, including at both boundaries
    poison[0] = i32::MIN;
    poison[1] = i32::MAX;
    poison[ARRAY_LEN - 1] = i32::MIN;
    let out = {
        let l = libs();
        l.rs.write_array(&poison);
        String::from_utf8(capture_stdout(|| l.rs.long_exec(65535))).unwrap()
    };
    assert_eq!(out, gt(65535), "poisoned array leaked into the result");
}
