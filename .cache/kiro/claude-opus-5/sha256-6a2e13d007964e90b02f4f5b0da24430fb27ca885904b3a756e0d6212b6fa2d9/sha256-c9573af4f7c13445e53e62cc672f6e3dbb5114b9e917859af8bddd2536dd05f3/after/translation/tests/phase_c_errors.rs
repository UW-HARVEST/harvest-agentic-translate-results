//! Phase C — error / rejection-path differential tests.
//!
//! One test per row of `ERRORS.md`. The C library has no error codes (its only
//! public function returns `void` and validates nothing), so each row asserts
//! the *observable* rejection result the C actually produces:
//!
//!   * a fatal signal (compared by exact signal number, via `fork` + `waitpid`),
//!     or
//!   * a normal return that leaves the buffer bit-identical (a silent no-op).
//!
//! "Both failed somehow" is never accepted: signal rows compare the concrete
//! `WTERMSIG`, no-op rows compare every byte of the backing allocation.

mod common;

use common::*;

fn rng_for(row: u64) -> Rng {
    Rng::new(SEED ^ (row.wrapping_mul(0x0000_0100_0000_00C5) | 1))
}

const I32_MIN: i32 = i32::MIN;
const I32_MAX: i32 = i32::MAX;

// --- row 1: img == NULL -----------------------------------------------------
#[test]
fn err_01_null_img_faults_identically() {
    // Resolve both symbols in the PARENT: a child forked from a multi-threaded
    // test process must not be the one to run the lazy `impls()` init.
    let im = impls();
    let out_c = run_in_child(move || unsafe { (im.c)(std::ptr::null_mut()) });
    let out_r = run_in_child(move || unsafe { (im.rust)(std::ptr::null_mut()) });
    assert_eq!(
        out_c, out_r,
        "err_01: NULL img must terminate identically (C={out_c:?}, Rust={out_r:?})"
    );
    // Pin down the expected shape too, so a change to "both exit 0" is noticed.
    assert_eq!(
        out_c,
        Outcome::Signaled(libc::SIGSEGV),
        "err_01: expected SIGSEGV from the C implementation, got {out_c:?}"
    );
    assert_same_outcome_null_img("err_01");
}

// --- row 2: pix == NULL with real work --------------------------------------
#[test]
fn err_02_null_pix_with_work_faults_identically() {
    let mut rng = rng_for(2);
    for _ in 0..8 {
        let w = rng.range_i32(1, 16);
        let h = rng.range_i32(2, 16);
        let case = Case::new(w, h).alloc(0).null_pix();
        assert_same_outcome("err_02", case);
    }
    // And the exact signal, for one representative instance.
    let im = impls();
    let mk = |f: FlipFn| {
        run_in_child(move || {
            let mut img = cp_image_t {
                w: 4,
                h: 4,
                pix: std::ptr::null_mut(),
            };
            unsafe { f(&mut img) };
        })
    };
    assert_eq!(mk(im.c), Outcome::Signaled(libc::SIGSEGV));
    assert_eq!(mk(im.rust), Outcome::Signaled(libc::SIGSEGV));
}

// --- row 3: pix == NULL, h == 0 ---------------------------------------------
#[test]
fn err_03_null_pix_h0_is_noop() {
    let mut rng = rng_for(3);
    for _ in 0..REPS {
        let w = rng.range_i32(-64, 64);
        let case = Case::new(w, 0).alloc(0).null_pix();
        // Must return normally (exit 0) in BOTH, not fault.
        assert_same_outcome("err_03", case);
        assert_null_pix_returns_ok("err_03", w, 0);
    }
}

// --- row 4: pix == NULL, h == 1 ---------------------------------------------
#[test]
fn err_04_null_pix_h1_is_noop() {
    let mut rng = rng_for(4);
    for _ in 0..REPS {
        let w = rng.range_i32(-64, 64);
        assert_same_outcome("err_04", Case::new(w, 1).alloc(0).null_pix());
        assert_null_pix_returns_ok("err_04", w, 1);
    }
}

// --- row 5: pix == NULL, w == 0, h >= 2 -------------------------------------
#[test]
fn err_05_null_pix_w0_is_noop() {
    let mut rng = rng_for(5);
    for _ in 0..REPS {
        let h = rng.range_i32(2, 64);
        assert_same_outcome("err_05", Case::new(0, h).alloc(0).null_pix());
        assert_null_pix_returns_ok("err_05", 0, h);
    }
}

// --- row 6: pix == NULL, w < 0, h >= 2 --------------------------------------
#[test]
fn err_06_null_pix_wneg_is_noop() {
    let mut rng = rng_for(6);
    for _ in 0..REPS {
        let w = rng.range_i32(-4096, -1);
        let h = rng.range_i32(2, 64);
        assert_same_outcome("err_06", Case::new(w, h).alloc(0).null_pix());
        assert_null_pix_returns_ok("err_06", w, h);
    }
    // Extreme negative width, where `w * (h - i - 1)` overflows `int`.
    assert_same_outcome("err_06/int_min", Case::new(I32_MIN, 4).alloc(0).null_pix());
    assert_null_pix_returns_ok("err_06/int_min", I32_MIN, 4);
}

/// Both implementations must survive a NULL `pix` and return normally.
#[track_caller]
fn assert_null_pix_returns_ok(label: &str, w: i32, h: i32) {
    let im = impls();
    for (name, f) in [("C", im.c), ("Rust", im.rust)] {
        let out = run_in_child(move || {
            let mut img = cp_image_t {
                w,
                h,
                pix: std::ptr::null_mut(),
            };
            unsafe { f(&mut img) };
        });
        assert_eq!(
            out,
            Outcome::Exited(0),
            "{label}: {name} did not return normally for w={w} h={h} pix=NULL"
        );
    }
}

// --- row 7: h == 0 ----------------------------------------------------------
#[test]
fn err_07_h_zero_noop() {
    let mut rng = rng_for(7);
    for _ in 0..REPS {
        let w = rng.range_i32(1, 64);
        assert_same_and_noop("err_07", &Case::new(w, 0).alloc(w as usize), &mut rng);
    }
}

// --- row 8: h == 1 ----------------------------------------------------------
#[test]
fn err_08_h_one_noop() {
    let mut rng = rng_for(8);
    for _ in 0..REPS {
        let w = rng.range_i32(1, 64);
        assert_same_and_noop("err_08", &Case::new(w, 1), &mut rng);
    }
}

// --- row 9: h == -1 ---------------------------------------------------------
#[test]
fn err_09_h_neg_one_noop() {
    let mut rng = rng_for(9);
    for _ in 0..REPS {
        let w = rng.range_i32(1, 64);
        let case = Case::new(w, -1).alloc(w as usize * 4);
        assert_same_and_noop("err_09", &case, &mut rng);
    }
}

// --- row 10: h == -2 --------------------------------------------------------
#[test]
fn err_10_h_neg_two_noop() {
    let mut rng = rng_for(10);
    for _ in 0..REPS {
        let w = rng.range_i32(1, 64);
        let case = Case::new(w, -2).alloc(w as usize * 4);
        assert_same_and_noop("err_10", &case, &mut rng);
    }
}

// --- row 11: h arbitrary negative ------------------------------------------
#[test]
fn err_11_h_negative_random_noop() {
    let mut rng = rng_for(11);
    for _ in 0..REPS {
        let w = rng.range_i32(0, 64);
        let h = rng.range_i32(-100_000, -1);
        let case = Case::new(w, h).alloc((w as usize).max(1) * 4);
        assert_same_and_noop("err_11", &case, &mut rng);
    }
}

// --- row 12: h == INT_MIN ---------------------------------------------------
#[test]
fn err_12_h_int_min_noop() {
    let mut rng = rng_for(12);
    for _ in 0..REPS {
        let w = rng.range_i32(0, 64);
        let case = Case::new(w, I32_MIN).alloc((w as usize).max(1) * 4);
        assert_same_and_noop("err_12", &case, &mut rng);
    }
    // ...and one step "past" it on the other side of the boundary.
    let case = Case::new(8, I32_MIN + 1).alloc(64);
    assert_same_and_noop("err_12/int_min_plus_1", &case, &mut rng);
}

// --- row 13: w == 0, h >= 2 -------------------------------------------------
#[test]
fn err_13_w_zero_noop() {
    let mut rng = rng_for(13);
    for _ in 0..REPS {
        let h = rng.range_i32(2, 64);
        assert_same_and_noop("err_13", &Case::new(0, h).alloc(32), &mut rng);
    }
}

// --- row 14: w == -1, h >= 2 ------------------------------------------------
#[test]
fn err_14_w_neg_one_noop() {
    let mut rng = rng_for(14);
    for _ in 0..REPS {
        let h = rng.range_i32(2, 64);
        assert_same_and_noop("err_14", &Case::new(-1, h).alloc(32), &mut rng);
    }
}

// --- row 15: w arbitrary negative ------------------------------------------
#[test]
fn err_15_w_negative_random_noop() {
    let mut rng = rng_for(15);
    for _ in 0..REPS {
        let w = rng.range_i32(-100_000, -1);
        let h = rng.range_i32(2, 64);
        assert_same_and_noop("err_15", &Case::new(w, h).alloc(32), &mut rng);
    }
}

// --- row 16: w == INT_MIN, h >= 2 ------------------------------------------
#[test]
fn err_16_w_int_min_noop() {
    let mut rng = rng_for(16);
    for _ in 0..REPS {
        let h = rng.range_i32(2, 64);
        assert_same_and_noop("err_16", &Case::new(I32_MIN, h).alloc(32), &mut rng);
    }
    assert_same_and_noop(
        "err_16/int_min_plus_1",
        &Case::new(I32_MIN + 1, 6).alloc(32),
        &mut rng,
    );
}

// --- row 17: both negative --------------------------------------------------
#[test]
fn err_17_both_negative_noop() {
    let mut rng = rng_for(17);
    for _ in 0..REPS {
        let w = rng.range_i32(-100_000, -1);
        let h = rng.range_i32(-100_000, -1);
        assert_same_and_noop("err_17", &Case::new(w, h).alloc(32), &mut rng);
    }
    for (w, h) in [
        (I32_MIN, I32_MIN),
        (I32_MIN, -1),
        (-1, I32_MIN),
        (-1, -1),
        (-1, -2),
        (-2, -1),
    ] {
        assert_same_and_noop(
            &format!("err_17/{w}x{h}"),
            &Case::new(w, h).alloc(32),
            &mut rng,
        );
    }
}

// --- row 18: w == INT_MAX with h <= 1 --------------------------------------
#[test]
fn err_18_w_int_max_h_le1_noop() {
    let mut rng = rng_for(18);
    for h in [0i32, 1, -1, I32_MIN] {
        for _ in 0..8 {
            assert_same_and_noop(
                &format!("err_18/h={h}"),
                &Case::new(I32_MAX, h).alloc(32),
                &mut rng,
            );
        }
    }
    // One step below INT_MAX, same reasoning.
    assert_same_and_noop(
        "err_18/int_max_minus_1",
        &Case::new(I32_MAX - 1, 1).alloc(32),
        &mut rng,
    );
}

// --- row 19: huge h with w == 0 --------------------------------------------
#[test]
fn err_19_h_int_max_w0_noop() {
    let mut rng = rng_for(19);
    // Bounded surrogate: same `flips = h/2` no-memory-access path, odd height,
    // but a trip count that finishes quickly.
    for h in [4_000_001i32, 4_000_000, 8_000_003] {
        assert_same_and_noop(
            &format!("err_19/h={h}"),
            &Case::new(0, h).alloc(32),
            &mut rng,
        );
    }
}

/// The exact `h == INT_MAX` instance: ~2^30 empty outer iterations per
/// implementation. Correct but slow, so it is opt-in via `--ignored`.
#[test]
#[ignore = "2^30 empty iterations per implementation; run explicitly"]
fn err_19_h_int_max_w0_noop_exact() {
    let mut rng = rng_for(1019);
    assert_same_and_noop("err_19/exact", &Case::new(0, I32_MAX).alloc(32), &mut rng);
}

// ---------------------------------------------------------------------------
// Generic FFI boundary sweep (belt and braces on top of the table above)
// ---------------------------------------------------------------------------

/// Cross-product of the interesting integer boundary values for `w` and `h`,
/// restricted to the combinations where the C behavior is well-defined
/// (i.e. no in-bounds inner loop against an undersized buffer).
#[test]
fn err_20_boundary_value_cross_product() {
    let mut rng = rng_for(20);
    let vals = [I32_MIN, I32_MIN + 1, -100_000, -2, -1, 0, 1];
    for &w in &vals {
        for &h in &vals {
            // Any combination here has either w <= 0 (inner loop never runs)
            // or h <= 1 (outer loop never runs), so no memory is touched.
            assert!(w <= 0 || h <= 1);
            assert_same_and_noop(
                &format!("err_20/{w}x{h}"),
                &Case::new(w, h).alloc(64),
                &mut rng,
            );
        }
    }
    // w == 1 crossed with the non-positive heights.
    for &h in &vals[..6] {
        assert_same_and_noop(
            &format!("err_20/1x{h}"),
            &Case::new(1, h).alloc(64),
            &mut rng,
        );
    }
}

/// `INT_MAX` / large positive values for the field that does *not* drive memory
/// access, so the C stays well-defined.
#[test]
fn err_21_oversized_dimension_without_access() {
    let mut rng = rng_for(21);
    for &w in &[I32_MAX, I32_MAX - 1, 1 << 20, 1 << 30] {
        for &h in &[0i32, 1, -1, -2, I32_MIN] {
            assert_same_and_noop(
                &format!("err_21/{w}x{h}"),
                &Case::new(w, h).alloc(64),
                &mut rng,
            );
        }
    }
    for &h in &[1 << 20, 1 << 24] {
        assert_same_and_noop(
            &format!("err_21/0x{h}"),
            &Case::new(0, h).alloc(64),
            &mut rng,
        );
        assert_same_and_noop(
            &format!("err_21/-1x{h}"),
            &Case::new(-1, h).alloc(64),
            &mut rng,
        );
    }
}

/// There is no enum anywhere in the public API, so there is no out-of-range
/// enum variant to feed across FFI. This test documents and enforces that fact:
/// the only exported symbol takes a single pointer argument.
#[test]
fn err_22_no_enum_or_mode_parameter_exists() {
    let header = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/include/lib.h"),
    )
    .expect("read lib.h");
    assert!(
        !header.contains("enum"),
        "lib.h grew an enum — ERRORS.md must gain out-of-range-variant rows"
    );
    let src = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/src/lib.c"),
    )
    .expect("read lib.c");
    assert!(
        !src.contains("enum"),
        "lib.c grew an enum — ERRORS.md must gain out-of-range-variant rows"
    );
    // The single entry point resolves in both .so files.
    let _ = impls();
}
