//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md`. Fatal-signal rows (null dereference) are
//! compared in a re-exec'd subprocess so the exact terminating signal of the C
//! and the Rust library can be compared rather than "both failed somehow".

mod common;

use std::os::unix::process::ExitStatusExt;
use std::process::Command;

use common::*;

const SEED: u64 = 0xC0FF_EE00_1234_5678;

// ---------------------------------------------------------------------------
// Rows 1 & 2 — unchecked null dereference: compare the terminating signal.
// ---------------------------------------------------------------------------

/// Child worker: re-exec'd by the tests below. Performs one deliberately fatal
/// call so the parent can observe how the process dies.
///
/// Ignored by default so a normal `cargo test` run never executes it.
#[test]
#[ignore]
fn crash_child() {
    let side = std::env::var("DIFF_CRASH_SIDE").expect("DIFF_CRASH_SIDE unset");
    let case = std::env::var("DIFF_CRASH_CASE").expect("DIFF_CRASH_CASE unset");
    let pair = load_pair();
    let lib = match side.as_str() {
        "c" => &pair.c,
        "rust" => &pair.rust,
        other => panic!("bad DIFF_CRASH_SIDE {other}"),
    };

    match case.as_str() {
        // Row 1: img itself is NULL -> img->w faults.
        "null_img" => unsafe {
            lib.premultiply(std::ptr::null_mut());
        },
        // Row 2: img->pix is NULL and the bound is positive -> data[3] faults.
        "null_pix" => unsafe {
            let mut img = CpImage {
                w: 4,
                h: 4,
                pix: std::ptr::null_mut(),
            };
            lib.premultiply(&mut img);
        },
        other => panic!("bad DIFF_CRASH_CASE {other}"),
    }

    // If we get here the call did NOT fault. Exit with a distinctive code so the
    // parent can tell "returned normally" apart from "died by signal".
    eprintln!("{side}/{case}: returned normally (no fault)");
    std::process::exit(42);
}

/// Outcome of a child run: either a normal exit code or a terminating signal.
#[derive(Debug, PartialEq, Eq)]
enum Outcome {
    Code(i32),
    Signal(i32),
}

fn run_crash_child(side: &str, case: &str) -> Outcome {
    let exe = std::env::current_exe().expect("current_exe");
    let out = Command::new(exe)
        .args(["--ignored", "--exact", "crash_child", "--test-threads=1"])
        .env("DIFF_CRASH_SIDE", side)
        .env("DIFF_CRASH_CASE", case)
        // Keep the child's own recursion impossible.
        .env_remove("RUST_BACKTRACE")
        .output()
        .expect("spawn crash child");

    match (out.status.code(), out.status.signal()) {
        (Some(c), _) => Outcome::Code(c),
        (None, Some(s)) => Outcome::Signal(s),
        _ => panic!("child neither exited nor signalled: {:?}", out.status),
    }
}

/// ERRORS.md row 1 — `img == NULL`: no null check in C, so both must die the
/// same way.
#[test]
fn err01_null_img_same_signal() {
    let c = run_crash_child("c", "null_img");
    let r = run_crash_child("rust", "null_img");
    assert_eq!(
        c, r,
        "row 1: C and Rust disagree on img==NULL (C: {c:?}, Rust: {r:?})"
    );
    assert_eq!(
        c,
        Outcome::Signal(libc_sigsegv()),
        "row 1: expected SIGSEGV, got {c:?}"
    );
}

/// ERRORS.md row 2 — `img->pix == NULL` with a positive bound.
#[test]
fn err02_null_pix_positive_bound_same_signal() {
    let c = run_crash_child("c", "null_pix");
    let r = run_crash_child("rust", "null_pix");
    assert_eq!(
        c, r,
        "row 2: C and Rust disagree on pix==NULL (C: {c:?}, Rust: {r:?})"
    );
    assert_eq!(
        c,
        Outcome::Signal(libc_sigsegv()),
        "row 2: expected SIGSEGV, got {c:?}"
    );
}

fn libc_sigsegv() -> i32 {
    11
}

// ---------------------------------------------------------------------------
// Rows 3 & 4 — NULL pix but a zero bound: the pointer is never dereferenced,
// so both libraries must return normally, in-process, without faulting.
// ---------------------------------------------------------------------------

fn null_pix_no_fault(w: i32, h: i32, tag: &str) {
    let pair = load_pair();
    assert_eq!(
        expected_iterations(w, h),
        0,
        "{tag}: this row requires a zero trip count"
    );
    for lib in [&pair.c, &pair.rust] {
        let mut img = CpImage {
            w,
            h,
            pix: std::ptr::null_mut(),
        };
        unsafe { lib.premultiply(&mut img) };
        // Reaching this line at all is the assertion: no fault occurred.
        assert_eq!((img.w, img.h), (w, h), "{tag}: {} mutated the struct", lib.name);
        assert!(img.pix.is_null(), "{tag}: {} mutated pix", lib.name);
    }
}

/// ERRORS.md row 3 — `pix == NULL`, `w == 0`.
#[test]
fn err03_null_pix_zero_w_returns_normally() {
    null_pix_no_fault(0, 4, "row3");
}

/// ERRORS.md row 4 — `pix == NULL`, `h == 0`.
#[test]
fn err04_null_pix_zero_h_returns_normally() {
    null_pix_no_fault(4, 0, "row4");
    null_pix_no_fault(0, 0, "row4b");
    // Also the negative-bound shapes never dereference pix.
    null_pix_no_fault(-4, 4, "row4c");
    null_pix_no_fault(4, -4, "row4d");
    null_pix_no_fault(i32::MAX, 1, "row4e");
    null_pix_no_fault(0x4000_0000, 1, "row4f");
}

// ---------------------------------------------------------------------------
// Rows 5..22, 24 — silent no-op / wrapped-bound rows. The "same rejection" is
// "the same number of pixels processed and the same resulting bytes", which is
// asserted byte-for-byte against a poisoned arena.
// ---------------------------------------------------------------------------

/// Assert both libraries treat `(w,h)` as a complete no-op: buffer bit-identical
/// to the input, guards intact, across many random inputs.
fn assert_noop(w: i32, h: i32, tag: &str, salt: u64) {
    let pair = load_pair();
    assert_eq!(
        expected_iterations(w, h),
        0,
        "{tag}: model says this shape is NOT a no-op (w={w} h={h})"
    );
    let mut rng = Rng::new(SEED ^ salt);
    for _ in 0..200 {
        let data = random_bytes(&mut rng, 16);
        let out = diff_run(&pair, w, h, &data, tag);
        assert_eq!(
            out, data,
            "{tag}: expected a no-op for w={w} h={h} but the buffer changed"
        );
    }
}

/// Assert both libraries process exactly `expect_px` pixels and nothing beyond.
fn assert_exact_trip(w: i32, h: i32, expect_px: u32, tag: &str, salt: u64) {
    let pair = load_pair();
    assert_eq!(
        expected_iterations(w, h),
        expect_px,
        "{tag}: model trip count disagrees with the row"
    );
    let mut rng = Rng::new(SEED ^ salt);
    let n = expect_px as usize + 4;
    for _ in 0..200 {
        // rgb=0xFF, alpha=0 => every processed pixel becomes 00 00 00 00 and
        // every untouched pixel stays ff ff ff 00, making the boundary visible.
        let mut data = random_bytes(&mut rng, n);
        for i in 0..n {
            data[i * 4] = 0xFF;
            data[i * 4 + 1] = 0xFF;
            data[i * 4 + 2] = 0xFF;
            data[i * 4 + 3] = 0x00;
        }
        let out = diff_run(&pair, w, h, &data, tag);
        let span = expect_px as usize * PIXEL_SIZE;
        for i in 0..expect_px as usize {
            assert_eq!(
                &out[i * 4..i * 4 + 3],
                &[0, 0, 0],
                "{tag}: pixel {i} should have been processed"
            );
        }
        assert_eq!(
            &out[span..],
            &data[span..],
            "{tag}: bytes past the wrapped end were modified"
        );
    }
}

/// ERRORS.md row 5 — `w == 0`, `h > 0`.
#[test]
fn err05_zero_w() {
    assert_noop(0, 4, "row5", 5);
    assert_noop(0, 1, "row5b", 105);
    assert_noop(0, 1000, "row5c", 205);
}

/// ERRORS.md row 6 — `h == 0`, `w > 0`.
#[test]
fn err06_zero_h() {
    assert_noop(4, 0, "row6", 6);
    assert_noop(1, 0, "row6b", 106);
    assert_noop(1000, 0, "row6c", 206);
}

/// ERRORS.md row 7 — both zero.
#[test]
fn err07_both_zero() {
    assert_noop(0, 0, "row7", 7);
}

/// ERRORS.md row 8 — `w < 0`, `h > 0`.
#[test]
fn err08_negative_w() {
    for (i, (w, h)) in [(-1, 1), (-4, 4), (-1, 1000), (-9, 3)].iter().enumerate() {
        assert_noop(*w, *h, "row8", 800 + i as u64);
    }
}

/// ERRORS.md row 9 — `w > 0`, `h < 0`.
#[test]
fn err09_negative_h() {
    for (i, (w, h)) in [(1, -1), (4, -4), (1000, -1), (3, -9)].iter().enumerate() {
        assert_noop(*w, *h, "row9", 900 + i as u64);
    }
}

/// ERRORS.md row 10 — `w < 0` AND `h < 0`: the bound goes positive and the loop
/// RUNS. This quirk must be preserved, not "fixed".
#[test]
fn err10_both_negative_loop_runs() {
    assert_exact_trip(-1, -1, 1, "row10a", 10);
    assert_exact_trip(-2, -3, 6, "row10b", 110);
    assert_exact_trip(-4, -4, 16, "row10c", 210);
    assert_exact_trip(-3, -5, 15, "row10d", 310);
}

/// ERRORS.md row 11 — `w == 2^30`: `w << 2` wraps to 0, no OOB access.
#[test]
fn err11_w_2pow30_stride_wraps_to_zero() {
    for (i, h) in [1, 2, 3, 1000, -1, i32::MAX, i32::MIN].iter().enumerate() {
        assert_noop(0x4000_0000, *h, "row11", 1100 + i as u64);
    }
}

/// ERRORS.md row 12 — `w == 2^29`, `h == 1`: stride becomes INT_MIN.
#[test]
fn err12_w_2pow29_h1_stride_intmin() {
    assert_noop(0x2000_0000, 1, "row12", 12);
}

/// ERRORS.md row 13 — `w == 2^29`, `h == 2`: bound wraps to 0.
#[test]
fn err13_w_2pow29_h2_bound_zero() {
    assert_noop(0x2000_0000, 2, "row13", 13);
}

/// ERRORS.md row 14 — `w == 2^29`, `h == 3`: bound is INT_MIN.
#[test]
fn err14_w_2pow29_h3_bound_intmin() {
    assert_noop(0x2000_0000, 3, "row14", 14);
    // h = 4 wraps back to 0, h = 5 back to INT_MIN: both still no-ops.
    assert_noop(0x2000_0000, 4, "row14b", 114);
    assert_noop(0x2000_0000, 5, "row14c", 214);
}

/// ERRORS.md row 15 — `w == INT_MAX`, `h == 1`: stride wraps to -4.
#[test]
fn err15_intmax_w_h1() {
    assert_noop(i32::MAX, 1, "row15", 15);
    assert_noop(i32::MAX, 2, "row15b", 115);
}

/// ERRORS.md row 16 — `w == INT_MAX`, `h == -1`: bound is +4 → exactly 1 pixel.
#[test]
fn err16_intmax_w_h_neg1_one_pixel() {
    assert_exact_trip(i32::MAX, -1, 1, "row16", 16);
}

/// ERRORS.md row 17 — `w == INT_MIN`, `h == 1`: stride wraps to 0.
#[test]
fn err17_intmin_w_h1() {
    assert_noop(i32::MIN, 1, "row17", 17);
}

/// ERRORS.md row 18 — `w == INT_MIN` with negative h.
#[test]
fn err18_intmin_w_negative_h() {
    assert_noop(i32::MIN, -1, "row18", 18);
    assert_noop(i32::MIN, i32::MIN, "row18b", 118);
    assert_noop(i32::MIN, i32::MAX, "row18c", 218);
}

/// ERRORS.md row 19 — `w == 1`, `h == INT_MAX`: bound wraps to -4.
#[test]
fn err19_w1_h_intmax() {
    assert_noop(1, i32::MAX, "row19", 19);
}

/// ERRORS.md row 20 — `w == 1`, `h == INT_MIN`: bound wraps to 0.
#[test]
fn err20_w1_h_intmin() {
    assert_noop(1, i32::MIN, "row20", 20);
}

/// ERRORS.md row 21 — `stride * h` overflows to a negative bound. A
/// non-wrapping implementation would run ~10^9 iterations and segfault here.
#[test]
fn err21_bound_overflow_to_negative() {
    assert_noop(1000, 1_000_000, "row21", 21);
    assert_noop(1_000_000, 1000, "row21b", 121);
    assert_noop(123_456, 7_890, "row21c", 221);
    // 65536 * 16384 wraps to exactly 0 (one step below the row-22 case).
    assert_noop(65_536, 16_384, "row21d", 321);
}

/// ERRORS.md row 22 — `stride * h` overflows to a *small positive* bound: only
/// 65536 pixels are processed, not 2^30.
#[test]
fn err22_bound_overflow_to_small_positive() {
    let pair = load_pair();
    let w = 65536i32;
    let h = 16385i32;
    let iters = expected_iterations(w, h);
    assert_eq!(iters, 65536);
    let mut rng = Rng::new(SEED ^ 22);
    for _ in 0..3 {
        let n = iters as usize + 64;
        let mut data = random_bytes(&mut rng, n);
        for i in 0..n {
            data[i * 4] = 0xFF;
            data[i * 4 + 1] = 0xFF;
            data[i * 4 + 2] = 0xFF;
            data[i * 4 + 3] = 0x00;
        }
        let out = diff_run(&pair, w, h, &data, "row22");
        let span = iters as usize * PIXEL_SIZE;
        assert_eq!(&out[..span], &vec![0u8; span][..], "row22 processed span");
        assert_eq!(&out[span..], &data[span..], "row22 wrote past wrapped end");
    }
}

/// ERRORS.md row 23 — undersized buffer: the C has no bounds check and walks
/// past the logical end. Both libraries must touch the identical byte span with
/// identical values. Run against an over-allocated arena so the access stays
/// mapped.
#[test]
fn err23_undersized_buffer_identical_oob_span() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 23);
    // Logical image is 1 pixel, but the struct claims 3x5 = 15 pixels.
    let w = 3i32;
    let h = 5i32;
    let iters = expected_iterations(w, h) as usize;
    assert_eq!(iters, 15);
    for _ in 0..300 {
        // 64 live pixels: 1 "logical" + 63 pixels of arena the C will walk into.
        let data = random_bytes(&mut rng, 64);
        let out = diff_run(&pair, w, h, &data, "row23");
        // Both agree byte-for-byte (diff_run asserts it); additionally confirm
        // the overrun span is exactly 15 pixels and stops there.
        assert_eq!(
            &out[iters * PIXEL_SIZE..],
            &data[iters * PIXEL_SIZE..],
            "row23 overran past pixel 15"
        );
        for i in 0..iters {
            let px = [data[i * 4], data[i * 4 + 1], data[i * 4 + 2], data[i * 4 + 3]];
            assert_eq!(
                &out[i * 4..i * 4 + 4],
                &model_pixel(px)[..],
                "row23 pixel {i} value"
            );
        }
    }
}

/// ERRORS.md row 24 — both dimensions oversized: bound wraps to 16 → 4 pixels.
#[test]
fn err24_both_dims_oversized_four_pixels() {
    assert_exact_trip(0x7FFF_FFFE, 0x7FFF_FFFE, 4, "row24", 24);
}

// ---------------------------------------------------------------------------
// Generic boundary coverage required regardless of the table.
// ---------------------------------------------------------------------------

/// Every `int` is a legal value for `w`/`h` across the FFI boundary (the API has
/// no enums to overflow, so the dimension fields are the equivalent surface).
/// Sweep every boundary value and one step past each, in both fields.
/// `boundary_values()` / `INPROC_MAX_PX` live in `common` so that
/// `phase_c_large.rs` sweeps the identical value set and the two tests' coverage
/// accounting cannot drift apart.
#[test]
fn generic_every_dimension_boundary_and_one_past() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 0xB0DE);
    let values = boundary_values();
    assert_eq!(values.len(), 49, "boundary value set changed unexpectedly");

    let mut checked = 0usize;
    let mut skipped = 0usize;
    let mut zero_bound = 0usize;
    for &w in &values {
        for &h in &values {
            let iters = expected_iterations(w, h) as usize;
            if iters > INPROC_MAX_PX {
                // Covered exhaustively (by distinct trip count) in phase_c_large.
                skipped += 1;
                continue;
            }
            if iters == 0 {
                zero_bound += 1;
            }
            let n = iters + 4;
            let data = random_bytes(&mut rng, n);
            let out = diff_run(&pair, w, h, &data, "generic-boundary");
            assert_eq!(
                &out[iters * PIXEL_SIZE..],
                &data[iters * PIXEL_SIZE..],
                "generic: wrote past end for w={w} h={h}"
            );
            checked += 1;
        }
    }
    assert_eq!(
        checked + skipped,
        values.len() * values.len(),
        "sweep accounting"
    );
    assert_eq!(checked, 2229, "in-process boundary combos checked");
    assert_eq!(skipped, 172, "combos deferred to phase_c_large");
    assert_eq!(zero_bound, 1476, "zero-bound combos in the sweep");
}

/// A misaligned `cp_image_t *` is something a C caller can hand over (UB per the
/// C standard, but gcc emits plain `mov`s that work fine on x86-64). The Rust
/// side must not be stricter than the hardware: both libraries must read the
/// same `w`/`h`/`pix` and produce identical output.
#[test]
fn generic_misaligned_image_struct_pointer() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 0xA11C);

    for shift in 1usize..=7 {
        for _ in 0..100 {
            let w = 3i32;
            let h = 2i32;
            let iters = expected_iterations(w, h) as usize;
            let data = random_bytes(&mut rng, iters + 2);

            // Two independent pixel arenas, one per library.
            let mut c_pix = data.clone();
            let mut r_pix = data.clone();

            // Backing storage for a deliberately misaligned cp_image_t.
            // Layout: { i32 w; i32 h; ptr pix; } = 16 bytes.
            let mut c_raw = vec![0u8; 16 + 8];
            let mut r_raw = vec![0u8; 16 + 8];

            unsafe {
                for (raw, pix) in [
                    (&mut c_raw, c_pix.as_mut_ptr()),
                    (&mut r_raw, r_pix.as_mut_ptr()),
                ] {
                    let p = raw.as_mut_ptr().add(shift);
                    // Write the fields byte-wise so the misaligned store is legal.
                    std::ptr::copy_nonoverlapping(w.to_ne_bytes().as_ptr(), p, 4);
                    std::ptr::copy_nonoverlapping(h.to_ne_bytes().as_ptr(), p.add(4), 4);
                    let pv = (pix as usize).to_ne_bytes();
                    std::ptr::copy_nonoverlapping(pv.as_ptr(), p.add(8), 8);
                }

                let c_img = c_raw.as_mut_ptr().add(shift) as *mut CpImage;
                let r_img = r_raw.as_mut_ptr().add(shift) as *mut CpImage;
                assert_ne!(c_img as usize % 8, 0, "pointer should be misaligned");
                pair.c.premultiply(c_img);
                pair.rust.premultiply(r_img);
            }

            assert_eq!(
                c_pix, r_pix,
                "misaligned img (shift={shift}): pixel output diverged"
            );
            // And the transform is still correct for the processed span.
            for i in 0..iters {
                let input = [data[i * 4], data[i * 4 + 1], data[i * 4 + 2], data[i * 4 + 3]];
                assert_eq!(
                    &c_pix[i * 4..i * 4 + 4],
                    &model_pixel(input)[..],
                    "misaligned img (shift={shift}): pixel {i}"
                );
            }
            // The w/h fields must be untouched by both (the C only reads them).
            assert_eq!(
                &c_raw[shift..shift + 8],
                &r_raw[shift..shift + 8],
                "misaligned img: w/h fields diverged"
            );
            assert_eq!(
                i32::from_ne_bytes(c_raw[shift..shift + 4].try_into().unwrap()),
                w,
                "misaligned img: w was modified"
            );
            assert_eq!(
                i32::from_ne_bytes(c_raw[shift + 4..shift + 8].try_into().unwrap()),
                h,
                "misaligned img: h was modified"
            );
        }
    }
}

/// Zero-length and oversized-length variants combined with a NULL `pix`: any
/// shape with a zero/negative bound must be safe even with no buffer at all.
#[test]
fn generic_null_pix_across_all_zero_bound_shapes() {
    let pair = load_pair();
    let mut safe = 0usize;
    for w in [
        i32::MIN,
        -1,
        0,
        1,
        0x2000_0000,
        0x4000_0000,
        i32::MAX,
        1000,
        1_000_000,
    ] {
        for h in [
            i32::MIN,
            -1,
            0,
            1,
            2,
            3,
            0x2000_0000,
            i32::MAX,
            1000,
            1_000_000,
        ] {
            if expected_iterations(w, h) != 0 {
                continue;
            }
            for lib in [&pair.c, &pair.rust] {
                let mut img = CpImage {
                    w,
                    h,
                    pix: std::ptr::null_mut(),
                };
                unsafe { lib.premultiply(&mut img) };
            }
            safe += 1;
        }
    }
    assert!(safe > 50, "expected many zero-bound shapes, got {safe}");
}
