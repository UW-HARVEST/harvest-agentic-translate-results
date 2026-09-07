//! Phase C — error-path differential tests, one test per row of `ERRORS.md`.
//!
//! `premultiply` returns `void` and has no error codes, so "the same
//! error/rejection" means the same *observable* rejection:
//!   * for rows 1-11 and 15-16: the same "did nothing" sentinel — the pixel
//!     buffer is bit-identical to the input in BOTH libraries, `pix` is never
//!     dereferenced (proved by passing `NULL`), and both return normally;
//!   * for rows 12-14 (undefined behaviour in C): the same *process outcome*,
//!     compared by running each library's call in a forked child and asserting
//!     the children die with the same signal / exit the same way.

mod common;

use common::{bytes, diff, libs, CpImage, CpPixel, PremultiplyFn, Rng};

// ---------------------------------------------------------------------------
// Fork helper: run one call in a child process and report how the child ended.
// ---------------------------------------------------------------------------

extern "C" {
    fn fork() -> i32;
    fn waitpid(pid: i32, status: *mut i32, options: i32) -> i32;
    fn _exit(code: i32) -> !;
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Outcome {
    Exited(i32),
    Signalled(i32),
}

/// Call `f(img)` inside a forked child; return how the child terminated.
/// The child `_exit(0)`s if the call returns normally.
fn outcome_of(f: PremultiplyFn, img: *mut CpImage) -> Outcome {
    unsafe {
        let pid = fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            f(img);
            _exit(0);
        }
        let mut status: i32 = 0;
        let r = waitpid(pid, &mut status, 0);
        assert_eq!(r, pid, "waitpid failed");
        // WIFSIGNALED / WTERMSIG / WEXITSTATUS, glibc encoding.
        let termsig = status & 0x7f;
        if termsig != 0 && termsig != 0x7f {
            Outcome::Signalled(termsig)
        } else {
            Outcome::Exited((status >> 8) & 0xff)
        }
    }
}

/// Differential outcome check for the UB rows.
#[track_caller]
fn diff_outcome(make_img: impl Fn() -> (Box<CpImage>, Vec<CpPixel>), label: &str) {
    let l = libs();

    let (mut c_img, mut _c_keep) = make_img();
    let c_out = outcome_of(l.c_premultiply, &mut *c_img as *mut CpImage);

    let (mut r_img, mut _r_keep) = make_img();
    let r_out = outcome_of(l.rust_premultiply, &mut *r_img as *mut CpImage);

    assert_eq!(
        c_out, r_out,
        "[{label}] process outcome differs: C={c_out:?} Rust={r_out:?}"
    );
}

/// Rows where the trigger must be a no-op: assert C and Rust both leave the
/// buffer bit-identical, both leave the struct alone, and both return normally
/// even when `pix` is NULL (proving neither dereferences it).
#[track_caller]
fn assert_rejected_noop(w: i32, h: i32, label: &str) {
    let l = libs();

    // (a) with a real, pattern-filled buffer: must be untouched by both.
    let mut rng = Rng::new(0xC0DE_0000 ^ (w as u64) ^ ((h as u64) << 32));
    let px = rng.pixels(64);
    let out = diff(&px, w, h, label);
    assert_eq!(
        bytes(&out),
        bytes(&px),
        "[{label}] w={w} h={h}: buffer must be untouched (no work performed)"
    );

    // (b) with pix == NULL: must still return normally in both -> proves the
    //     rejection happens before any dereference. Run in-process; a fault
    //     here would abort the test binary, which is itself a failure signal,
    //     so also cross-check via fork for a precise outcome comparison.
    let mut c_img = CpImage { w, h, pix: std::ptr::null_mut() };
    let mut r_img = CpImage { w, h, pix: std::ptr::null_mut() };
    let c_out = outcome_of(l.c_premultiply, &mut c_img);
    let r_out = outcome_of(l.rust_premultiply, &mut r_img);
    assert_eq!(
        c_out,
        Outcome::Exited(0),
        "[{label}] C must return normally with pix=NULL (w={w} h={h})"
    );
    assert_eq!(
        r_out,
        Outcome::Exited(0),
        "[{label}] Rust must return normally with pix=NULL (w={w} h={h})"
    );
    assert_eq!(c_out, r_out, "[{label}] outcome mismatch with pix=NULL");

    unsafe {
        (l.c_premultiply)(&mut c_img);
        (l.rust_premultiply)(&mut r_img);
    }
    assert_eq!((c_img.w, c_img.h), (w, h));
    assert_eq!((r_img.w, r_img.h), (w, h));
}

// --- row 1: w == 0 ---------------------------------------------------------
#[test]
fn row01_w_zero() {
    for h in [0i32, 1, 2, 7, 1024, i32::MAX, -1, i32::MIN] {
        assert_rejected_noop(0, h, &format!("ERRORS row1 w=0 h={h}"));
    }
}

// --- row 2: h == 0 ---------------------------------------------------------
#[test]
fn row02_h_zero() {
    for w in [0i32, 1, 2, 7, 1024, i32::MAX, -1, i32::MIN] {
        assert_rejected_noop(w, 0, &format!("ERRORS row2 w={w} h=0"));
    }
}

// --- row 3: w < 0, h > 0 ---------------------------------------------------
#[test]
fn row03_negative_w_positive_h() {
    for (w, h) in [(-4i32, 3i32), (-1, 1), (-1, 64), (-7, 9), (-1000, 1000)] {
        assert_rejected_noop(w, h, &format!("ERRORS row3 {w}x{h}"));
    }
}

// --- row 4: h < 0, w > 0 ---------------------------------------------------
#[test]
fn row04_positive_w_negative_h() {
    for (w, h) in [(4i32, -3i32), (1, -1), (64, -1), (9, -7), (1000, -1000)] {
        assert_rejected_noop(w, h, &format!("ERRORS row4 {w}x{h}"));
    }
}

// --- row 5: both negative -> bound positive -> work IS done ----------------
#[test]
fn row05_both_negative_performs_work() {
    // Not a rejection at all: the double sign flip makes the bound positive and
    // the loop runs. Both libs must do the *same* work over the same range.
    let mut rng = Rng::new(0xC0DE_0005);
    for (w, h) in [(-1i32, -1i32), (-1, -2), (-2, -3), (-4, -5), (-8, -8), (-1, -100)] {
        let pixels = (w as i64 * h as i64) as usize;
        for i in 0..8 {
            let px = rng.pixels(pixels + 8);
            let out = diff(&px, w, h, &format!("ERRORS row5 {w}x{h} iter {i}"));
            assert_ne!(
                bytes(&out),
                bytes(&px[..]),
                "ERRORS row5 {w}x{h}: expected real work to happen \
                 (statistically certain with random alphas)"
            );
            assert_eq!(
                bytes(&out[pixels..]),
                bytes(&px[pixels..]),
                "ERRORS row5 {w}x{h}: must stop after {pixels} pixels"
            );
        }
    }
}

// --- row 6: w*4 overflows to exactly 0 ------------------------------------
#[test]
fn row06_stride_overflow_to_zero() {
    // 0x2000_0000 * 4 == 2^31 -> wraps; 0x4000_0000 * 4 == 2^32 -> 0.
    for w in [0x4000_0000i32, -0x4000_0000i32, i32::MIN] {
        assert_eq!(w.wrapping_mul(4), 0, "test bug: stride not 0 for w={w:#x}");
        for h in [1i32, 2, 7, 1024, i32::MAX, -1, i32::MIN] {
            assert_rejected_noop(w, h, &format!("ERRORS row6 w={w:#x} h={h}"));
        }
    }
}

// --- row 7: w*4 wraps negative, h > 0 -------------------------------------
#[test]
fn row07_stride_wraps_negative() {
    for w in [0x2000_0000i32, 0x3000_0000, 0x2000_0001, 0x7FFF_FFFF] {
        let stride = w.wrapping_mul(4);
        if stride >= 0 {
            // Not this row's trigger for that particular w; skip.
            continue;
        }
        for h in [1i32, 2, 3, 7, 1024] {
            let bound = stride.wrapping_mul(h);
            if bound > 0 {
                continue; // covered by row 10
            }
            assert_rejected_noop(w, h, &format!("ERRORS row7 w={w:#x} h={h}"));
        }
    }
    // Explicit documented case: w = 0x3000_0000 -> stride = 0xC000_0000 < 0.
    assert!(0x3000_0000i32.wrapping_mul(4) < 0);
    assert_rejected_noop(0x3000_0000, 1, "ERRORS row7 documented case");
}

// --- row 8: stride*h overflows to exactly 0 -------------------------------
#[test]
fn row08_bound_overflow_to_zero() {
    // w = 0x10000 -> stride = 0x40000 ; h = 0x4000 -> 0x40000 * 0x4000 == 2^32.
    let (w, h) = (0x10000i32, 0x4000i32);
    assert_eq!(w.wrapping_mul(4).wrapping_mul(h), 0, "test bug");
    assert_rejected_noop(w, h, "ERRORS row8 documented case");

    for (w, h) in [(0x10000i32, 0x4000i32), (0x8000, 0x8000), (0x20000, 0x2000), (0x40000000, 4)] {
        if w.wrapping_mul(4).wrapping_mul(h) != 0 {
            continue;
        }
        assert_rejected_noop(w, h, &format!("ERRORS row8 {w:#x}x{h:#x}"));
    }
}

// --- row 9: stride*h overflows negative ------------------------------------
#[test]
fn row09_bound_overflow_negative() {
    let (w, h) = (1000i32, 1_000_000i32);
    assert!(
        w.wrapping_mul(4).wrapping_mul(h) < 0,
        "test bug: bound not negative"
    );
    assert_rejected_noop(w, h, "ERRORS row9 documented case");

    for (w, h) in [
        (1000i32, 1_000_000i32),
        (0x10000, 0x2000),
        (65535, 65535),
        (0x1FFF_FFFF, 3),
    ] {
        if w.wrapping_mul(4).wrapping_mul(h) >= 0 {
            continue;
        }
        assert_rejected_noop(w, h, &format!("ERRORS row9 {w}x{h}"));
    }
}

// --- row 10: stride*h wraps to a small POSITIVE value ----------------------
#[test]
fn row10_bound_overflow_small_positive() {
    // w = 0x10000 -> stride = 0x40000 ; h = 0x4001 -> bound wraps to 0x40000,
    // i.e. 0x40000/4 = 65536 pixels get processed instead of 0x10000*0x4001.
    let (w, h) = (0x10000i32, 0x4001i32);
    let bound = w.wrapping_mul(4).wrapping_mul(h);
    assert_eq!(bound, 0x40000, "test bug: unexpected wrapped bound");
    let expected_pixels = (bound / 4) as usize;

    let mut rng = Rng::new(0xC0DE_000A);
    let px = rng.pixels(expected_pixels + 8);
    let out = diff(&px, w, h, "ERRORS row10 documented case");
    assert_eq!(
        bytes(&out[expected_pixels..]),
        bytes(&px[expected_pixels..]),
        "ERRORS row10: exactly {expected_pixels} pixels must be processed"
    );
    assert_ne!(bytes(&out), bytes(&px), "ERRORS row10: work must happen");

    // A second, smaller wrap: stride = 0x40000, h = 0x4002 -> bound 0x80000.
    let (w2, h2) = (0x10000i32, 0x4002i32);
    let bound2 = w2.wrapping_mul(4).wrapping_mul(h2);
    assert_eq!(bound2, 0x80000);
    let n2 = (bound2 / 4) as usize;
    let px2 = rng.pixels(n2 + 8);
    let out2 = diff(&px2, w2, h2, "ERRORS row10 second wrap");
    assert_eq!(bytes(&out2[n2..]), bytes(&px2[n2..]));
}

// --- row 11: pix == NULL with a zero bound --------------------------------
#[test]
fn row11_null_pix_with_zero_bound() {
    let l = libs();
    // Every zero/negative-bound trigger from rows 1-4, 6-9, 15-16 with NULL pix.
    let cases: &[(i32, i32)] = &[
        (0, 0),
        (0, 5),
        (5, 0),
        (-4, 3),
        (4, -3),
        (0x4000_0000, 7),
        (i32::MIN, 7),
        (0x3000_0000, 1),
        (0x10000, 0x4000),
        (1000, 1_000_000),
        (i32::MIN, i32::MIN),
        (4, i32::MIN),
        (i32::MAX, 0),
        (0, i32::MIN),
    ];
    for &(w, h) in cases {
        let bound = w.wrapping_mul(4).wrapping_mul(h);
        assert!(bound <= 0, "test bug: case {w}x{h} has positive bound {bound}");
        let mut c_img = CpImage { w, h, pix: std::ptr::null_mut() };
        let mut r_img = CpImage { w, h, pix: std::ptr::null_mut() };
        let c_out = outcome_of(l.c_premultiply, &mut c_img);
        let r_out = outcome_of(l.rust_premultiply, &mut r_img);
        assert_eq!(
            c_out,
            Outcome::Exited(0),
            "ERRORS row11: C must not fault for {w}x{h} with NULL pix"
        );
        assert_eq!(c_out, r_out, "ERRORS row11: outcome mismatch for {w}x{h}");
        // In-process too, to be sure the symbol really returned.
        unsafe {
            (l.c_premultiply)(&mut c_img);
            (l.rust_premultiply)(&mut r_img);
        }
    }
}

// --- row 12: pix == NULL with a POSITIVE bound (UB: SIGSEGV) --------------
#[test]
fn row12_null_pix_with_positive_bound_same_signal() {
    for (w, h) in [(1i32, 1i32), (4, 4), (16, 16), (-1, -1)] {
        assert!(w.wrapping_mul(4).wrapping_mul(h) > 0, "test bug");
        diff_outcome(
            || {
                (
                    Box::new(CpImage { w, h, pix: std::ptr::null_mut() }),
                    Vec::new(),
                )
            },
            &format!("ERRORS row12 {w}x{h} NULL pix"),
        );
    }
    // And assert it really is a fault, not a silent return, in both.
    let l = libs();
    let mut img = CpImage { w: 4, h: 4, pix: std::ptr::null_mut() };
    let c = outcome_of(l.c_premultiply, &mut img);
    let r = outcome_of(l.rust_premultiply, &mut img);
    assert!(
        matches!(c, Outcome::Signalled(_)),
        "ERRORS row12: expected C to fault, got {c:?}"
    );
    assert_eq!(c, r, "ERRORS row12: C={c:?} Rust={r:?}");
}

// --- row 13: img == NULL (UB: SIGSEGV) ------------------------------------
#[test]
fn row13_null_img_same_signal() {
    let l = libs();
    let c = outcome_of(l.c_premultiply, std::ptr::null_mut());
    let r = outcome_of(l.rust_premultiply, std::ptr::null_mut());
    assert!(
        matches!(c, Outcome::Signalled(_)),
        "ERRORS row13: expected C to fault on img=NULL, got {c:?}"
    );
    assert_eq!(
        c, r,
        "ERRORS row13: img=NULL outcome differs: C={c:?} Rust={r:?}"
    );
}

// --- row 14: w/h describe more pixels than the buffer holds ----------------
#[test]
fn row14_out_of_range_index() {
    // Over-declare the image against a *padded* allocation so both libraries
    // walk the same, still-mapped bytes and must agree on all of them.
    let mut rng = Rng::new(0xC0DE_000E);
    // Logical 8x8 = 64 pixels, but the caller "owns" only 4; allocate 64+8 so
    // the identical out-of-range walk is observable rather than fatal.
    for (dw, dh, alloc) in [
        (8i32, 8i32, 64usize + 8),
        (4, 4, 16 + 8),
        (10, 10, 100 + 8),
        (1, 100, 100 + 8),
    ] {
        for i in 0..8 {
            let px = rng.pixels(alloc);
            let declared = (dw as usize) * (dh as usize);
            let out = diff(
                &px,
                dw,
                dh,
                &format!("ERRORS row14 declared {dw}x{dh} alloc {alloc} iter {i}"),
            );
            assert_eq!(
                bytes(&out[declared..]),
                bytes(&px[declared..]),
                "ERRORS row14: exactly {declared} pixels must be walked"
            );
        }
    }
    // Truly-too-small buffers: the read/write past the end is UB, so compare at
    // the process-outcome level with a page-sized declared region.
    diff_outcome(
        || {
            let mut buf = vec![CpPixel::default(); 4];
            let img = Box::new(CpImage {
                w: 4096,
                h: 4096,
                pix: buf.as_mut_ptr(),
            });
            (img, buf)
        },
        "ERRORS row14 far out-of-range",
    );
}

// --- row 15: w == INT_MIN -------------------------------------------------
#[test]
fn row15_w_int_min() {
    assert_eq!(i32::MIN.wrapping_mul(4), 0, "test bug");
    for h in [0i32, 1, 2, 7, -1, 1024, i32::MAX, i32::MIN] {
        assert_rejected_noop(i32::MIN, h, &format!("ERRORS row15 w=INT_MIN h={h}"));
    }
}

// --- row 16: h == INT_MIN with w > 0 --------------------------------------
#[test]
fn row16_h_int_min() {
    for w in [1i32, 2, 3, 4, 7, 1024, i32::MAX] {
        let bound = w.wrapping_mul(4).wrapping_mul(i32::MIN);
        assert_eq!(
            bound, 0,
            "test bug: stride is a multiple of 4 so stride*INT_MIN must wrap to 0 (w={w})"
        );
        assert_rejected_noop(w, i32::MIN, &format!("ERRORS row16 w={w} h=INT_MIN"));
    }
}

// --- row 17: out-of-range enum values ------------------------------------
#[test]
fn row17_no_enum_in_api_full_int_range_instead() {
    // The C API declares no enum, so the analogue is "any int is a legal FFI
    // value for w/h". Sweep the full i32 range with NULL pix (safe whenever the
    // bound is non-positive) and additionally with a padded real buffer whenever
    // the wrapped bound is small and positive.
    let l = libs();
    let mut rng = Rng::new(0xC0DE_0011);
    let mut null_cases = 0usize;
    let mut real_cases = 0usize;

    for _ in 0..20_000 {
        let w = rng.next_i32();
        let h = rng.next_i32();
        let bound = w.wrapping_mul(4).wrapping_mul(h);
        if bound <= 0 {
            let mut c_img = CpImage { w, h, pix: std::ptr::null_mut() };
            let mut r_img = CpImage { w, h, pix: std::ptr::null_mut() };
            let c = outcome_of(l.c_premultiply, &mut c_img);
            let r = outcome_of(l.rust_premultiply, &mut r_img);
            assert_eq!(c, Outcome::Exited(0), "row17: C faulted on {w}x{h}");
            assert_eq!(c, r, "row17: outcome mismatch on {w}x{h}");
            null_cases += 1;
            if null_cases > 400 {
                break;
            }
        }
    }
    assert!(null_cases > 100, "row17: too few NULL cases ({null_cases})");

    // Small-positive wrapped bounds with a real buffer. Rejection sampling over
    // the full i32 range essentially never lands here (probability ~2^-19), so
    // construct such (w, h) pairs algebraically instead: pick an odd `h` and a
    // target bound `b = 4k`, then solve `4*w*h == b (mod 2^32)` using the
    // modular inverse of `h`. The resulting `w`/`h` are arbitrary large-magnitude
    // (often negative) i32 values whose wrapped bound is exactly `b`.
    let mut rng2 = Rng::new(0xC0DE_0012);
    let mut tried = 0usize;
    while real_cases < 200 && tried < 100_000 {
        tried += 1;
        let h_u = rng2.next_u32() | 1; // odd -> invertible mod 2^32
        let k = rng2.range(1, 4096) as u32; // pixel count
        let b = k.wrapping_mul(4); // target bound, divisible by 4

        // Newton iteration for the inverse of an odd number mod 2^32.
        let mut inv: u32 = 1;
        for _ in 0..5 {
            inv = inv.wrapping_mul(2u32.wrapping_sub(h_u.wrapping_mul(inv)));
        }
        debug_assert_eq!(h_u.wrapping_mul(inv), 1);

        let s = b.wrapping_mul(inv); // desired value of (4*w) mod 2^32
        if s % 4 != 0 {
            continue;
        }
        // 4*w == s (mod 2^32)  =>  w == s/4 (mod 2^30); the top two bits are
        // free, so randomize them to reach both signs and large magnitudes.
        let w_u = (s / 4) | ((rng2.next_u32() & 0b11) << 30);
        let (w, h) = (w_u as i32, h_u as i32);

        let bound = w.wrapping_mul(4).wrapping_mul(h);
        assert_eq!(
            bound, b as i32,
            "test bug: constructed bound mismatch for w={w:#x} h={h:#x}"
        );
        if bound <= 0 {
            continue; // b >= 2^31 cannot happen here, but be defensive
        }

        let n = (bound / 4) as usize;
        let px = rng2.pixels(n + 8);
        let out = diff(&px, w, h, &format!("row17 real w={w} h={h} bound={bound}"));
        assert_eq!(
            bytes(&out[n..]),
            bytes(&px[n..]),
            "row17: exactly {n} pixels must be walked for w={w} h={h}"
        );
        real_cases += 1;
    }
    assert!(real_cases > 50, "row17: too few real-buffer cases ({real_cases})");
    eprintln!("row17: null_cases={null_cases} real_cases={real_cases}");
}

// --- generic boundary sweep (required by Phase C regardless of the table) ---
#[test]
fn generic_boundaries() {
    let l = libs();

    // Zero and oversized lengths, and one step past every boundary value.
    let interesting: &[i32] = &[
        i32::MIN,
        i32::MIN + 1,
        -0x4000_0001,
        -0x4000_0000,
        -0x3FFF_FFFF,
        -0x1_0001,
        -0x1_0000,
        -257,
        -256,
        -255,
        -2,
        -1,
        0,
        1,
        2,
        3,
        4,
        5,
        254,
        255,
        256,
        257,
        0xFFFF,
        0x1_0000,
        0x1_0001,
        0x1FFF_FFFF,
        0x2000_0000,
        0x2000_0001,
        0x3FFF_FFFF,
        0x4000_0000,
        0x4000_0001,
        i32::MAX - 1,
        i32::MAX,
    ];

    let mut rng = Rng::new(0xC0DE_BEEF);
    let mut checked_null = 0usize;
    let mut checked_real = 0usize;

    for &w in interesting {
        for &h in interesting {
            let bound = w.wrapping_mul(4).wrapping_mul(h);
            if bound <= 0 {
                let mut c_img = CpImage { w, h, pix: std::ptr::null_mut() };
                let mut r_img = CpImage { w, h, pix: std::ptr::null_mut() };
                unsafe {
                    (l.c_premultiply)(&mut c_img);
                    (l.rust_premultiply)(&mut r_img);
                }
                assert_eq!((c_img.w, c_img.h, c_img.pix), (w, h, std::ptr::null_mut()));
                assert_eq!((r_img.w, r_img.h, r_img.pix), (w, h, std::ptr::null_mut()));

                // Non-NULL buffer: must be left untouched by both.
                let px = rng.pixels(8);
                let out = diff(&px, w, h, &format!("generic noop {w}x{h}"));
                assert_eq!(bytes(&out), bytes(&px), "generic: {w}x{h} must be a no-op");
                checked_null += 1;
            } else if bound <= 4 * 8192 {
                let n = (bound / 4) as usize;
                let px = rng.pixels(n + 8);
                let out = diff(&px, w, h, &format!("generic real {w}x{h} bound={bound}"));
                assert_eq!(bytes(&out[n..]), bytes(&px[n..]));
                checked_real += 1;
            }
        }
    }
    eprintln!("generic_boundaries: noop={checked_null} real={checked_real}");
    assert!(checked_null > 500);
    assert!(checked_real > 10);
}
