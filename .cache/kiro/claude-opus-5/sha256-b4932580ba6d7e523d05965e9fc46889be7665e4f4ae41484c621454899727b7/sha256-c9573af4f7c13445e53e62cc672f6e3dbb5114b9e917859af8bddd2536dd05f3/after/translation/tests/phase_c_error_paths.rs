//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md`. The C API returns `void` and contains no
//! `return`, `assert`, error enum or range check at all, so its entire
//! rejection surface is expressed through the loop bound
//! `(int)((int)(w * sizeof(cp_pixel_t)) * h)`. Every distinct way of reaching a
//! non-positive bound, plus the pointer preconditions, is asserted here against
//! BOTH shared objects.
//!
//! For the two rows where the C genuinely faults (null `img`, null `pix` with a
//! positive bound) the comparison is made on the *signal disposition* of a
//! forked child, so "C and Rust fail the same way" is asserted rather than
//! assumed.

mod support;

use std::os::unix::process::ExitStatusExt;
use std::process::Command;

use support::{c_loop_bound, load_pair, CPixel, Lib, Rng};

/// Assert that both libraries leave the whole buffer bit-identical, i.e. the
/// call was a silent no-op — the way this API rejects bad geometry.
fn assert_both_noop(c: &Lib, r: &Lib, label: &str, w: i32, h: i32) {
    // Precondition from the C source model: the loop bound must be <= 0.
    assert!(
        c_loop_bound(w, h) <= 0,
        "{label}: precondition failed, bound = {} is positive for w={w} h={h}",
        c_loop_bound(w, h)
    );

    let mut rng = Rng::new(0xC0FF_EE00 ^ (w as u64) << 32 ^ (h as u64 & 0xFFFF_FFFF));
    for it in 0..200 {
        // A generous buffer: if either implementation wrote anything at all we
        // will see it.
        let mut original = vec![0u8; 4096];
        rng.fill_bytes(&mut original);
        let mut cb = original.clone();
        let mut rb = original.clone();
        c.call_bytes(w, h, &mut cb);
        r.call_bytes(w, h, &mut rb);
        assert_eq!(
            cb, original,
            "{label}: C was expected to be a no-op for w={w} h={h} (iter {it})"
        );
        assert_eq!(
            rb, original,
            "{label}: Rust was expected to be a no-op for w={w} h={h} (iter {it})"
        );
        assert_eq!(cb, rb, "{label}: C/Rust divergence for w={w} h={h}");
    }
}

// ERRORS row 1 — w == 0.
#[test]
fn err01_width_zero() {
    let (c, r) = load_pair();
    for h in [1i32, 2, 7, 64, 1024, i32::MAX, -1, i32::MIN] {
        assert_both_noop(&c, &r, "err01", 0, h);
    }
}

// ERRORS row 2 — h == 0.
#[test]
fn err02_height_zero() {
    let (c, r) = load_pair();
    for w in [1i32, 2, 7, 64, 1024, i32::MAX, -1, i32::MIN] {
        assert_both_noop(&c, &r, "err02", w, 0);
    }
}

// ERRORS row 3 — both zero.
#[test]
fn err03_both_zero() {
    let (c, r) = load_pair();
    assert_both_noop(&c, &r, "err03", 0, 0);
}

// ERRORS row 4 — w < 0, h > 0.
#[test]
fn err04_negative_width_positive_height() {
    let (c, r) = load_pair();
    for w in [-1i32, -2, -3, -255, -4096] {
        for h in [1i32, 2, 3, 17, 1024] {
            assert_both_noop(&c, &r, "err04", w, h);
        }
    }
}

// ERRORS row 5 — w > 0, h < 0.
#[test]
fn err05_positive_width_negative_height() {
    let (c, r) = load_pair();
    for w in [1i32, 2, 3, 17, 1024] {
        for h in [-1i32, -2, -3, -255, -4096] {
            assert_both_noop(&c, &r, "err05", w, h);
        }
    }
}

// ERRORS row 6 — w < 0 && h < 0: bound is POSITIVE, so this is NOT a no-op.
// The differential assertion is that both libraries process the same wrapped
// byte range identically (also covered by CONFIGS rows 19/20).
#[test]
fn err06_both_negative_is_not_a_noop() {
    let (c, r) = load_pair();
    let mut rng = Rng::new(0x0606_0606);
    for w in [-1i32, -2, -3, -5, -8] {
        for h in [-1i32, -2, -3, -5, -8] {
            let bound = c_loop_bound(w, h);
            assert!(bound > 0, "err06: precondition, bound must be positive");
            let touched = bound as usize;
            for it in 0..100 {
                let mut original = vec![0u8; touched + 64];
                rng.fill_bytes(&mut original);
                let mut cb = original.clone();
                let mut rb = original.clone();
                c.call_bytes(w, h, &mut cb);
                r.call_bytes(w, h, &mut rb);
                assert_eq!(
                    cb, rb,
                    "err06: divergence for w={w} h={h} (iter {it}, bound {bound})"
                );
                assert_eq!(
                    &cb[touched..],
                    &original[touched..],
                    "err06: C walked past the wrapped bound {touched}"
                );
                assert_eq!(
                    &rb[touched..],
                    &original[touched..],
                    "err06: Rust walked past the wrapped bound {touched}"
                );
            }
        }
    }
}

// ERRORS row 7 — w == INT_MIN: (size_t)INT_MIN * 4 truncates to 0.
#[test]
fn err07_width_int_min() {
    let (c, r) = load_pair();
    assert_eq!(
        c_loop_bound(i32::MIN, 1),
        0,
        "err07: precondition, INT_MIN*4 truncates to 0"
    );
    for h in [1i32, 2, 3, 1024, i32::MAX] {
        assert_both_noop(&c, &r, "err07", i32::MIN, h);
    }
}

// ERRORS row 8 — w == 2^29: w*4 truncates to INT_MIN (negative).
#[test]
fn err08_width_two_pow_29() {
    let (c, r) = load_pair();
    let w = 0x2000_0000i32;
    assert_eq!(w.wrapping_mul(4), i32::MIN, "err08: precondition");
    for h in [1i32, 3, 5, 1023] {
        // odd h keeps INT_MIN*h negative
        if c_loop_bound(w, h) <= 0 {
            assert_both_noop(&c, &r, "err08", w, h);
        }
    }
}

// ERRORS row 9 — w == 2^30: w*4 truncates to exactly 0.
#[test]
fn err09_width_two_pow_30() {
    let (c, r) = load_pair();
    let w = 0x4000_0000i32;
    assert_eq!(w.wrapping_mul(4), 0, "err09: precondition");
    for h in [1i32, 2, 7, i32::MAX, -1, i32::MIN] {
        assert_both_noop(&c, &r, "err09", w, h);
    }
}

// ERRORS row 10 — w == 2^29 + 1: one step past row 8.
#[test]
fn err10_width_two_pow_29_plus_one() {
    let (c, r) = load_pair();
    let w = 0x2000_0001i32;
    assert_eq!(w.wrapping_mul(4), -2_147_483_644, "err10: precondition");
    for h in [1i32, 3, 5] {
        if c_loop_bound(w, h) <= 0 {
            assert_both_noop(&c, &r, "err10", w, h);
        }
    }
}

// ERRORS row 11 — stride positive but stride*h overflows int to INT_MIN.
#[test]
fn err11_stride_times_height_overflows() {
    let (c, r) = load_pair();
    let w = 0x1000_0000i32; // 2^28 -> stride 2^30 (positive)
    assert_eq!(w.wrapping_mul(4), 0x4000_0000, "err11: stride is positive");
    let h = 2i32;
    assert_eq!(c_loop_bound(w, h), i32::MIN, "err11: bound wraps to INT_MIN");
    assert_both_noop(&c, &r, "err11", w, h);
    // A few more overflowing (stride>0, h>0) pairs.
    for (w, h) in [
        (0x1000_0000i32, 2i32),
        (0x0800_0000, 4),
        (0x0400_0000, 8),
        (0x2000_0000, 2),
    ] {
        if c_loop_bound(w, h) <= 0 {
            assert_both_noop(&c, &r, "err11", w, h);
        }
    }
}

// ERRORS row 12 — h == INT_MIN.
#[test]
fn err12_height_int_min() {
    let (c, r) = load_pair();
    for w in [1i32, 2, 3, 7, 1024, -1, -3] {
        if c_loop_bound(w, i32::MIN) <= 0 {
            assert_both_noop(&c, &r, "err12", w, i32::MIN);
        }
    }
    // `stride` is always `w*4`, i.e. always even, and any even multiple of
    // INT_MIN wraps to exactly 0 in 32-bit arithmetic. So for h == INT_MIN the
    // bound is 0 for EVERY w, and the call is always a no-op.
    for w in [1i32, 2, 3, 5, 7, 1024, -1, -3, i32::MAX, i32::MIN, 0] {
        assert_eq!(
            c_loop_bound(w, i32::MIN),
            0,
            "err12: precondition — (w*4) * INT_MIN wraps to 0 for every w (w={w})"
        );
    }
}

// ERRORS row 13 — bound wraps to a small POSITIVE value (+12).
// Only 3 pixels may be touched even though w*h is ~3.2 billion.
#[test]
fn err13_bound_wraps_to_small_positive() {
    let (c, r) = load_pair();
    let (w, h) = (3i32, 0x4000_0001i32);
    assert_eq!(c_loop_bound(w, h), 12, "err13: precondition");
    let mut rng = Rng::new(0x1313_1313);
    for it in 0..500 {
        let mut original = vec![0u8; 4096];
        rng.fill_bytes(&mut original);
        let mut cb = original.clone();
        let mut rb = original.clone();
        c.call_bytes(w, h, &mut cb);
        r.call_bytes(w, h, &mut rb);
        assert_eq!(cb, rb, "err13: divergence (iter {it})");
        assert_eq!(&cb[12..], &original[12..], "err13: C over-walked");
        assert_eq!(&rb[12..], &original[12..], "err13: Rust over-walked");
    }
}

// ERRORS row 14 — pix == NULL with a non-positive bound: never dereferenced.
#[test]
fn err14_null_pix_with_nonpositive_bound() {
    let (c, r) = load_pair();
    for (w, h) in [
        (0i32, 0i32),
        (0, 5),
        (5, 0),
        (-1, 1),
        (1, -1),
        (i32::MIN, 1),
        (0x4000_0000, 3),
        (0x2000_0000, 1),
        (1, i32::MIN),
    ] {
        assert!(
            c_loop_bound(w, h) <= 0,
            "err14: precondition for w={w} h={h}"
        );
        // If either implementation dereferenced `pix` here the process would
        // die with SIGSEGV and the test would fail by crashing.
        unsafe {
            c.call_raw(w, h, std::ptr::null_mut());
            r.call_raw(w, h, std::ptr::null_mut());
        }
    }
}

// ERRORS rows 15 & 16 — genuine faults. Compared via child processes.
//
// The parent re-executes this test binary with `PHASE_C_CRASH_LIB` /
// `PHASE_C_CRASH_CASE` set; the child runs `crash_probe` (an `#[ignore]`d test)
// which performs the faulting call. The parent then compares the termination
// signal of the C child and the Rust child.

#[test]
#[ignore = "internal child-process probe, driven by err15/err16"]
fn crash_probe() {
    let which = std::env::var("PHASE_C_CRASH_LIB").expect("PHASE_C_CRASH_LIB");
    let case = std::env::var("PHASE_C_CRASH_CASE").expect("PHASE_C_CRASH_CASE");
    let (c, r) = load_pair();
    let lib: &Lib = match which.as_str() {
        "c" => &c,
        "rust" => &r,
        other => panic!("unknown lib selector {other}"),
    };
    match case.as_str() {
        // img == NULL: dereferenced immediately at `img->w`.
        "null_img" => unsafe { lib.call_null_img() },
        // pix == NULL with a positive bound: dereferenced in the loop body.
        "null_pix" => unsafe {
            lib.call_raw(1, 1, std::ptr::null_mut::<CPixel>());
        },
        other => panic!("unknown case {other}"),
    }
    // If we get here the call did NOT fault. Signal that distinctly.
    std::process::exit(77);
}

fn run_crash_probe(which: &str, case: &str) -> (Option<i32>, Option<i32>) {
    let exe = std::env::current_exe().expect("current_exe");
    let out = Command::new(exe)
        .args(["--exact", "crash_probe", "--ignored", "--nocapture", "--test-threads=1"])
        .env("PHASE_C_CRASH_LIB", which)
        .env("PHASE_C_CRASH_CASE", case)
        .env("RUST_BACKTRACE", "0")
        .output()
        .expect("spawn crash probe");
    (out.status.code(), out.status.signal())
}

#[test]
fn err16_null_img_faults_identically() {
    // Make sure both libraries are loadable in-process first.
    let _ = load_pair();
    let c = run_crash_probe("c", "null_img");
    let r = run_crash_probe("rust", "null_img");
    assert_eq!(
        c, r,
        "err16: C and Rust must fail identically on img == NULL \
         (code, signal): C={c:?} Rust={r:?}"
    );
    assert_eq!(
        c.1,
        Some(11),
        "err16: expected SIGSEGV from the null-`img` dereference, got {c:?}"
    );
}

#[test]
fn err15_null_pix_with_positive_bound_faults_identically() {
    let _ = load_pair();
    let c = run_crash_probe("c", "null_pix");
    let r = run_crash_probe("rust", "null_pix");
    assert_eq!(
        c, r,
        "err15: C and Rust must fail identically on pix == NULL with a \
         positive bound (code, signal): C={c:?} Rust={r:?}"
    );
    assert_eq!(
        c.1,
        Some(11),
        "err15: expected SIGSEGV from the null-`pix` dereference, got {c:?}"
    );
}

// ERRORS row 17 — bound exceeds the caller's logical array. The walk is
// unchecked in C; with an over-allocated backing buffer the exact overrun is
// observable and must match.
#[test]
fn err17_bound_exceeds_logical_array() {
    let (c, r) = load_pair();
    let mut rng = Rng::new(0x1717_1717);
    for it in 0..1_000 {
        // Caller claims a bigger image than it "allocated" logically, but the
        // real allocation is large enough that the overrun stays mapped.
        let logical = rng.range(1, 32) as i32;
        let claimed_w = logical + rng.range(1, 32) as i32;
        let claimed_h = 1i32;
        let touched = support::c_touched_bytes(claimed_w, claimed_h);
        let mut original = vec![0u8; touched + 128];
        rng.fill_bytes(&mut original);
        let mut cb = original.clone();
        let mut rb = original.clone();
        c.call_bytes(claimed_w, claimed_h, &mut cb);
        r.call_bytes(claimed_w, claimed_h, &mut rb);
        assert_eq!(
            cb, rb,
            "err17: divergence on overrun (w={claimed_w} h={claimed_h} iter {it})"
        );
        assert_eq!(
            &cb[touched..],
            &original[touched..],
            "err17: C walked past {touched}"
        );
        assert_eq!(
            &rb[touched..],
            &original[touched..],
            "err17: Rust walked past {touched}"
        );
    }
}

// ERRORS row 18 — misaligned `pix`. Byte-wise access, must be accepted by both.
// (Also CONFIGS row 15; repeated here as the explicit error-surface assertion.)
#[test]
fn err18_misaligned_pix_accepted() {
    let (c, r) = load_pair();
    let mut rng = Rng::new(0x1818_0018);
    for offset in 1usize..=3 {
        for it in 0..200 {
            let w = rng.range(1, 16) as i32;
            let h = rng.range(1, 16) as i32;
            let px = (w * h) as usize;
            let mut original = vec![0u8; px * 4 + 8];
            rng.fill_bytes(&mut original);
            let mut cb = original.clone();
            let mut rb = original.clone();
            unsafe {
                c.call_raw(w, h, cb.as_mut_ptr().add(offset) as *mut CPixel);
                r.call_raw(w, h, rb.as_mut_ptr().add(offset) as *mut CPixel);
            }
            assert_eq!(
                cb, rb,
                "err18: divergence at +{offset} (w={w} h={h} iter {it})"
            );
        }
    }
}

// ERRORS row 19 — the alpha byte is never written. Must hold in both.
#[test]
fn err19_alpha_never_written() {
    let (c, r) = load_pair();
    let mut rng = Rng::new(0x1919_0019);
    for it in 0..1_000 {
        let w = rng.range(1, 32) as i32;
        let h = rng.range(1, 32) as i32;
        let px = (w * h) as usize;
        let mut original = vec![0u8; px * 4];
        rng.fill_bytes(&mut original);
        let mut cb = original.clone();
        let mut rb = original.clone();
        c.call_bytes(w, h, &mut cb);
        r.call_bytes(w, h, &mut rb);
        assert_eq!(cb, rb, "err19: divergence (iter {it})");
        for k in 0..px {
            assert_eq!(
                cb[k * 4 + 3],
                original[k * 4 + 3],
                "err19: C modified alpha of pixel {k}"
            );
            assert_eq!(
                rb[k * 4 + 3],
                original[k * 4 + 3],
                "err19: Rust modified alpha of pixel {k}"
            );
        }
    }
}

// --------------------------------------------------------------------------
// Generic boundaries required by Phase C regardless of the table.
// --------------------------------------------------------------------------

// B6 — "out-of-range enum across the FFI boundary". `lib.h` declares no enum
// and no mode/flag parameter; the only scalars the caller controls are the two
// `int`s. The equivalent test is therefore: every bit pattern of `w`/`h` is a
// legal C value and must be handled identically. This sweeps the full i32
// domain by structured sampling plus randomized draws.
#[test]
fn boundary_full_i32_domain_sampling() {
    let (c, r) = load_pair();
    let mut rng = Rng::new(0xB006_B006);

    let mut candidates: Vec<i32> = Vec::new();
    // every power of two and its neighbours, both signs
    for k in 0..32 {
        let v = 1i32.wrapping_shl(k);
        for d in [-2i32, -1, 0, 1, 2] {
            candidates.push(v.wrapping_add(d));
            candidates.push(v.wrapping_neg().wrapping_add(d));
        }
    }
    candidates.extend_from_slice(&[0, i32::MIN, i32::MAX, i32::MIN + 1, i32::MAX - 1]);
    for _ in 0..256 {
        candidates.push(rng.next_i32());
    }
    candidates.sort_unstable();
    candidates.dedup();

    const CAP: usize = 1 << 14;
    let mut checked = 0usize;
    for &w in &candidates {
        for &h in &[
            -8i32,
            -3,
            -2,
            -1,
            0,
            1,
            2,
            3,
            8,
            i32::MIN,
            i32::MAX,
            0x4000_0000,
        ] {
            let touched = support::c_touched_bytes(w, h);
            if touched > CAP {
                continue;
            }
            let mut original = vec![0u8; touched + 64];
            rng.fill_bytes(&mut original);
            let mut cb = original.clone();
            let mut rb = original.clone();
            c.call_bytes(w, h, &mut cb);
            r.call_bytes(w, h, &mut rb);
            assert_eq!(cb, rb, "boundary sweep: divergence at w={w} h={h}");
            assert_eq!(
                &cb[touched..],
                &original[touched..],
                "boundary sweep: C over-walked at w={w} h={h}"
            );
            assert_eq!(
                &rb[touched..],
                &original[touched..],
                "boundary sweep: Rust over-walked at w={w} h={h}"
            );
            checked += 1;
        }
    }
    assert!(
        checked > 2_000,
        "boundary sweep covered only {checked} (w,h) pairs"
    );
}

// Zero-length buffer with a zero bound: nothing is read, nothing is written.
#[test]
fn boundary_zero_length_buffer() {
    let (c, r) = load_pair();
    let mut empty_c: [u8; 0] = [];
    let mut empty_r: [u8; 0] = [];
    for (w, h) in [(0i32, 0i32), (0, 1), (1, 0), (-1, 1), (1, -1)] {
        c.call_bytes(w, h, &mut empty_c);
        r.call_bytes(w, h, &mut empty_r);
    }
    assert_eq!(empty_c, empty_r);
}

// A dangling-but-never-dereferenced non-null pointer with a zero bound: both
// must ignore it entirely.
#[test]
fn boundary_dangling_pix_with_zero_bound() {
    let (c, r) = load_pair();
    let bogus = 0x1usize as *mut CPixel; // guaranteed unmapped
    for (w, h) in [(0i32, 0i32), (0, 7), (7, 0), (0x4000_0000, 9)] {
        assert!(c_loop_bound(w, h) <= 0);
        unsafe {
            c.call_raw(w, h, bogus);
            r.call_raw(w, h, bogus);
        }
    }
}
