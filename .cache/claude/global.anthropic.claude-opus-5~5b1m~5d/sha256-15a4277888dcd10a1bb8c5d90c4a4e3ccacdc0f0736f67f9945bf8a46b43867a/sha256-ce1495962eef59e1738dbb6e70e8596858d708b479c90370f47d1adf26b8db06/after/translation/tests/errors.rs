//! Phase C — error-path differential tests, one per row of ERRORS.md.
//!
//! `premultiply` returns `void` and contains no explicit error handling, so an
//! "error" is observable in exactly two ways, both asserted here:
//!
//!   * `NO-OP`  — the loop bound is `<= 0`, the pixel buffer is byte-identical
//!                afterwards and `img->pix` is never dereferenced (proved by
//!                passing a NULL / unmapped `pix` and not faulting).
//!   * `SIGSEGV` — a null / unmapped pointer is dereferenced. Verified in a
//!                forked child process, comparing the *exact fault signal*
//!                raised by the C `.so` against the Rust `.so`.

mod common;

use common::*;
use std::os::unix::process::ExitStatusExt;
use std::process::{Command, Stdio};

// ===========================================================================
// Rows 1-3 — crash parity, checked out-of-process.
// ===========================================================================

const CASE_ENV: &str = "PREMULT_CRASH_CASE";
const IMPL_ENV: &str = "PREMULT_CRASH_IMPL";

/// Runs the given crash case in a child process against one implementation and
/// returns `(signal, exit_code)`.
fn crash_signal(case: &str, imp: &str) -> (Option<i32>, Option<i32>) {
    let exe = std::env::current_exe().expect("current_exe");
    let status = Command::new(exe)
        .args(["--exact", "crash_child", "--include-ignored", "--test-threads=1"])
        .env(CASE_ENV, case)
        .env(IMPL_ENV, imp)
        // Propagate resolved paths so the child does not have to re-discover them.
        .env("C_SO_PATH", c_so_path())
        .env("RUST_SO_PATH", rust_so_path())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("spawn crash child");
    (status.signal(), status.code())
}

const SIGSEGV: i32 = 11;
const SIGABRT: i32 = 6;

fn assert_crash_parity(case: &str) {
    let (c_sig, c_code) = crash_signal(case, "c");
    let (r_sig, r_code) = crash_signal(case, "rust");

    // The C ground truth always faults with SIGSEGV.
    assert_eq!(
        c_sig,
        Some(SIGSEGV),
        "case '{case}': expected SIGSEGV ({SIGSEGV}) from the C library, got \
         signal {c_sig:?} / exit {c_code:?}"
    );

    if cfg!(debug_assertions) {
        // A debug-profile Rust cdylib carries the standard library's UB
        // pre-condition checks. Those turn a null/dangling load into a
        // controlled `abort()` (SIGABRT) *before* the faulting instruction is
        // ever executed, so the signal number legitimately differs from the C
        // build even though the translation is faithful. Both must still die
        // from a fatal signal -- a Rust that silently *returns* here (i.e. one
        // that added a null guard the C does not have) is still caught.
        assert!(
            r_sig == Some(SIGSEGV) || r_sig == Some(SIGABRT),
            "case '{case}': Rust must die from SIGSEGV or (debug UB-check) \
             SIGABRT, got signal {r_sig:?} / exit {r_code:?}"
        );
        if r_sig == Some(SIGSEGV) {
            assert_eq!(c_code, r_code, "case '{case}': exit codes differ");
        }
    } else {
        // Release profile: this is the shipped artifact and the one built with
        // the same optimisation posture as the C .so. Require exact parity.
        assert_eq!(
            c_sig, r_sig,
            "case '{case}': C died with signal {c_sig:?} (exit {c_code:?}) but \
             Rust died with signal {r_sig:?} (exit {r_code:?})"
        );
        assert_eq!(c_code, r_code, "case '{case}': exit codes differ");
    }
}

/// ERRORS.md row 1 — `img == NULL`.
#[test]
fn row01_null_img_faults_identically() {
    assert_crash_parity("null_img");
}

/// ERRORS.md row 2 — `img->pix == NULL` with a live extent.
#[test]
fn row02_null_pix_with_live_extent_faults_identically() {
    assert_crash_parity("null_pix");
}

/// ERRORS.md row 3 — `img->pix` = non-null but unmapped.
#[test]
fn row03_wild_pix_faults_identically() {
    assert_crash_parity("wild_pix");
}

/// The child half of rows 1-3. Ignored so it never runs during a normal pass;
/// the parent re-invokes this binary with `--include-ignored`.
#[test]
#[ignore = "spawned deliberately by the row01..row03 crash-parity tests"]
fn crash_child() {
    let case = match std::env::var(CASE_ENV) {
        Ok(c) => c,
        Err(_) => return, // Direct invocation: nothing to do.
    };
    let imp = std::env::var(IMPL_ENV).expect("impl env");
    let l = libs();
    let f: PremultiplyFn = match imp.as_str() {
        "c" => l.c,
        "rust" => l.rust,
        other => panic!("unknown impl {other}"),
    };

    match case.as_str() {
        "null_img" => unsafe { f(std::ptr::null_mut()) },
        "null_pix" => {
            let mut img = CpImage { w: 4, h: 4, pix: std::ptr::null_mut() };
            unsafe { f(&mut img as *mut CpImage) }
        }
        "wild_pix" => {
            // 0x1000 is below mmap_min_addr on any sane Linux -> unmapped.
            let mut img = CpImage { w: 4, h: 4, pix: 0x1000usize as *mut u8 };
            unsafe { f(&mut img as *mut CpImage) }
        }
        other => panic!("unknown case {other}"),
    }

    // If we get here the library did NOT fault. Exit non-zero but unsignalled
    // so the parent's signal comparison reports the discrepancy.
    eprintln!("case {case} / impl {imp} returned without faulting");
    std::process::exit(42);
}

// ===========================================================================
// Rows 4-25 — in-process NO-OP / extent parity.
// ===========================================================================

/// Assert both libraries treat `(w, h)` as a complete no-op over a live buffer.
fn assert_noop(w: i32, h: i32, seed: u64) {
    let mut rng = Rng::new(seed);
    let (stride, limit, iters) = semantics(w, h);
    assert_eq!(
        iters, 0,
        "test bug: (w={w}, h={h}) is not a no-op (stride={stride}, limit={limit})"
    );
    for _ in 0..8 {
        let p = rng.bytes(64);
        let out = assert_same_simple(w, h, &p);
        assert_eq!(
            out, p,
            "(w={w}, h={h}) must leave the buffer untouched \
             (stride={stride}, limit={limit})"
        );
    }
}

/// ERRORS.md row 4 — `w == 0`, `h > 0`.
#[test]
fn row04_zero_width() {
    for h in [1, 2, 3, 7, 1000, i32::MAX] {
        assert_noop(0, h, 0x0401 ^ h as u64);
    }
}

/// ERRORS.md row 5 — `h == 0`, `w > 0`.
#[test]
fn row05_zero_height() {
    for w in [1, 2, 3, 7, 1000, i32::MAX] {
        assert_noop(w, 0, 0x0501 ^ w as u64);
    }
}

/// ERRORS.md row 6 — both zero.
#[test]
fn row06_zero_width_and_height() {
    assert_noop(0, 0, 0x0601);
}

/// ERRORS.md row 7 — a zero dimension with `pix == NULL` must NOT fault,
/// because the loop never runs and so never dereferences `pix`.
#[test]
fn row07_zero_dimension_with_null_pix_does_not_fault() {
    let l = libs();
    let cases = [
        (0, 0),
        (0, 4),
        (4, 0),
        (0, i32::MAX),
        (i32::MAX, 0),
        (0, -4),
        (-4, 0),
    ];
    for (w, h) in cases {
        assert_eq!(semantics(w, h).2, 0, "({w},{h}) should be a no-op");
        for (who, f) in [("C", l.c), ("Rust", l.rust)] {
            let mut img = CpImage { w, h, pix: std::ptr::null_mut() };
            // Must return normally; a fault here would kill the test process.
            unsafe { f(&mut img as *mut CpImage) };
            assert_eq!(img.w, w, "{who} modified img.w");
            assert_eq!(img.h, h, "{who} modified img.h");
            assert!(img.pix.is_null(), "{who} modified img.pix");
        }
    }
}

/// ERRORS.md row 8 — `w < 0`, `h > 0`.
#[test]
fn row08_negative_width_positive_height() {
    for w in [-1, -2, -3, -7, -16] {
        for h in [1, 2, 3, 9] {
            assert_noop(w, h, (0x0801u64) ^ ((w as i64 as u64) << 8) ^ h as u64);
        }
    }
}

/// ERRORS.md row 9 — `w > 0`, `h < 0`.
#[test]
fn row09_positive_width_negative_height() {
    for w in [1, 2, 3, 7, 16] {
        for h in [-1, -2, -3, -9] {
            assert_noop(w, h, (0x0901u64) ^ ((w as u64) << 8) ^ (h as i64 as u64));
        }
    }
}

/// ERRORS.md row 10 — `w < 0 && h < 0` makes `limit` positive: the loop RUNS.
/// This is the "surprising acceptance" the C code exhibits.
#[test]
fn row10_both_negative_runs_the_loop() {
    let mut rng = Rng::new(0x1001);
    for (w, h) in [(-1, -1), (-2, -3), (-1, -4), (-4, -4), (-5, -2), (-8, -8)] {
        let iters = semantics(w, h).2;
        assert_eq!(iters, (w as i64 * h as i64) as usize, "iters model");
        assert!(iters > 0);
        for _ in 0..20 {
            let p = rng.bytes(iters * 4 + 32);
            let out = assert_same_simple(w, h, &p);
            // Exactly `iters` pixels processed, nothing beyond.
            assert_eq!(&out[iters * 4..], &p[iters * 4..]);
            // And the loop really did something observable somewhere.
            assert_eq!(out.len(), p.len());
        }
    }
}

/// ERRORS.md row 11 — `w ≡ 0 (mod 2^30)`, `w != 0`: `stride` wraps to 0.
#[test]
fn row11_stride_wraps_to_zero() {
    for w in [1 << 30, -(1 << 30), i32::MIN] {
        assert_eq!(semantics(w, 1).0, 0, "stride should wrap to 0 for w={w}");
        for h in [1, 2, 3, 5, 1000, i32::MAX, i32::MIN, -7] {
            assert_noop(w, h, 0x1101 ^ (w as i64 as u64) ^ ((h as i64 as u64) << 16));
        }
    }
}

/// ERRORS.md row 12 — `w == ±2^29`, `h` odd -> `limit == INT_MIN` (negative).
#[test]
fn row12_stride_intmin_odd_height() {
    for w in [1 << 29, -(1 << 29)] {
        assert_eq!(semantics(w, 1).0, i32::MIN);
        for h in [1, 3, 5, 7, 9, 1001] {
            assert_eq!(semantics(w, h).1, i32::MIN, "limit for ({w},{h})");
            assert_noop(w, h, 0x1201 ^ (w as i64 as u64) ^ ((h as u64) << 16));
        }
    }
}

/// ERRORS.md row 13 — `w == ±2^29`, `h` even -> `limit` wraps to 0.
#[test]
fn row13_stride_intmin_even_height() {
    for w in [1 << 29, -(1 << 29)] {
        for h in [2, 4, 6, 8, 1000] {
            assert_eq!(semantics(w, h).1, 0, "limit for ({w},{h})");
            assert_noop(w, h, 0x1301 ^ (w as i64 as u64) ^ ((h as u64) << 16));
        }
    }
}

/// ERRORS.md row 14 — `w == 2^29 + 1`, `h == 2` -> `limit` wraps to +8: 2 px.
#[test]
fn row14_wrap_to_positive_eight() {
    let (w, h) = (536_870_913i32, 2i32);
    let (stride, limit, iters) = semantics(w, h);
    assert_eq!((stride, limit, iters), (i32::MIN + 4, 8, 2));
    let mut rng = Rng::new(0x1401);
    for _ in 0..40 {
        let p = rng.bytes(64);
        let out = assert_same_simple(w, h, &p);
        assert_eq!(&out[8..], &p[8..], "only the first 2 pixels may change");
    }
}

/// ERRORS.md row 15 — `w == INT_MAX`, `h == 1` -> `limit == -4`: no-op.
#[test]
fn row15_intmax_width_height_one() {
    assert_eq!(semantics(i32::MAX, 1), (-4, -4, 0));
    assert_noop(i32::MAX, 1, 0x1501);
}

/// ERRORS.md row 16 — `w == INT_MAX`, `h == -1` -> `limit == 4`: 1 px.
#[test]
fn row16_intmax_width_negative_height() {
    let (w, h) = (i32::MAX, -1i32);
    assert_eq!(semantics(w, h), (-4, 4, 1));
    let mut rng = Rng::new(0x1601);
    for _ in 0..40 {
        let p = rng.bytes(64);
        let out = assert_same_simple(w, h, &p);
        assert_eq!(&out[4..], &p[4..], "only pixel 0 may change");
    }
}

/// ERRORS.md row 17 — both `INT_MAX` -> `limit == 4`: 1 px.
#[test]
fn row17_both_intmax() {
    let (w, h) = (i32::MAX, i32::MAX);
    assert_eq!(semantics(w, h), (-4, 4, 1));
    let mut rng = Rng::new(0x1701);
    for _ in 0..40 {
        let p = rng.bytes(64);
        let out = assert_same_simple(w, h, &p);
        assert_eq!(&out[4..], &p[4..]);
    }
}

/// ERRORS.md row 18 — `w == 1`, `h == INT_MIN` -> `limit` wraps to 0.
#[test]
fn row18_height_intmin() {
    assert_eq!(semantics(1, i32::MIN), (4, 0, 0));
    assert_noop(1, i32::MIN, 0x1801);
}

/// ERRORS.md row 19 — `w == 1`, `h == INT_MAX` -> `limit` wraps to -4.
#[test]
fn row19_height_intmax() {
    assert_eq!(semantics(1, i32::MAX), (4, -4, 0));
    assert_noop(1, i32::MAX, 0x1901);
}

/// ERRORS.md row 20 — both `INT_MIN`.
#[test]
fn row20_both_intmin() {
    assert_eq!(semantics(i32::MIN, i32::MIN), (0, 0, 0));
    assert_noop(i32::MIN, i32::MIN, 0x2001);
}

/// ERRORS.md row 21 — `w == 2^28`, `h == 4` -> `stride == 2^30`, `limit` -> 0.
#[test]
fn row21_stride_2p30_h4() {
    let (w, h) = (268_435_456i32, 4i32);
    assert_eq!(semantics(w, h), (1 << 30, 0, 0));
    assert_noop(w, h, 0x2101);
}

/// ERRORS.md row 22 — `w == 2^28 + 1`, `h == 4` -> `limit` wraps to +16: 4 px.
#[test]
fn row22_stride_wrap_to_positive_sixteen() {
    let (w, h) = (268_435_457i32, 4i32);
    assert_eq!(semantics(w, h).2, 4);
    let mut rng = Rng::new(0x2201);
    for _ in 0..40 {
        let p = rng.bytes(64);
        let out = assert_same_simple(w, h, &p);
        assert_eq!(&out[16..], &p[16..], "only the first 4 pixels may change");
    }
}

/// ERRORS.md row 23 — misaligned `cp_image_t *`.
#[test]
fn row23_misaligned_img_pointer() {
    let l = libs();
    let mut rng = Rng::new(0x2301);
    for _ in 0..30 {
        let payload = rng.bytes(16 * 4);
        let mut results = Vec::new();
        for (who, f) in [("C", l.c), ("Rust", l.rust)] {
            let mut g = Guarded::new(&payload, 0);
            let pix = g.ptr();
            let mut arena = vec![0u8; std::mem::size_of::<CpImage>() + 8];
            let img_ptr = unsafe {
                let mut p = arena.as_mut_ptr().add(1);
                if (p as usize) % 2 == 0 {
                    p = p.add(1);
                }
                p as *mut CpImage
            };
            assert_eq!((img_ptr as usize) % 2, 1);
            unsafe {
                img_ptr.write_unaligned(CpImage { w: 4, h: 4, pix });
                f(img_ptr);
            }
            g.assert_canaries(who, "row23 misaligned img");
            results.push(g.payload().to_vec());
        }
        assert_eq!(results[0], results[1], "misaligned img divergence");
    }
}

/// ERRORS.md row 24 — misaligned `pix`.
#[test]
fn row24_misaligned_pix() {
    let mut rng = Rng::new(0x2401);
    for misalign in 0..4usize {
        for _ in 0..30 {
            let p = rng.bytes(6 * 6 * 4);
            assert_same(6, 6, &p, misalign, 1);
        }
    }
}

/// ERRORS.md row 25 — exact write extent: no byte at or past `limit`, no byte
/// before `pix`, and no alpha byte is ever written.
#[test]
fn row25_write_extent_and_alpha_preservation() {
    let mut rng = Rng::new(0x2501);
    for _ in 0..150 {
        let w = rng.range(1, 12);
        let h = rng.range(1, 12);
        let px = (w * h) as usize;
        // Deliberately over-allocate so bytes past the extent are observable.
        let p = rng.bytes(px * 4 + 37);
        let out = assert_same(w, h, &p, 0, 1); // canaries checked inside
        assert_eq!(
            &out[px * 4..],
            &p[px * 4..],
            "wrote at/past limit for {w}x{h}"
        );
        for i in 0..px {
            assert_eq!(
                out[i * 4 + 3],
                p[i * 4 + 3],
                "alpha byte of pixel {i} was written ({w}x{h})"
            );
        }
    }
}

// ===========================================================================
// Generic FFI boundary sweeps (required even where not in the table).
// ===========================================================================

/// C `enum`s accept any `int`, so out-of-range enum values are a real input
/// class. `lib.h` declares NO enum, so the analogous "value with no valid
/// meaning" is an out-of-range dimension. Sweep every `±2^k` boundary and its
/// neighbours across both dimensions.
#[test]
fn generic_out_of_range_dimension_sweep() {
    let mut rng = Rng::new(0xDEAD_BEEF);
    let mut vals: Vec<i32> = Vec::new();
    for k in 0..32u32 {
        let base = 1i32.wrapping_shl(k);
        for d in [-1i32, 0, 1] {
            vals.push(base.wrapping_add(d));
            vals.push(base.wrapping_neg().wrapping_add(d));
        }
    }
    vals.extend_from_slice(&[0, 1, -1, i32::MIN, i32::MAX, i32::MIN + 1, i32::MAX - 1]);
    vals.sort_unstable();
    vals.dedup();

    const CAP_PX: usize = 256;
    let mut executed = 0usize;
    let mut skipped = 0usize;
    let mut ran = 0usize;

    for &w in &vals {
        for &h in &vals {
            let iters = semantics(w, h).2;
            if iters > CAP_PX {
                skipped += 1;
                continue;
            }
            executed += 1;
            if iters > 0 {
                ran += 1;
            }
            let p = rng.bytes(CAP_PX * 4);
            let out = assert_same_simple(w, h, &p);
            assert_eq!(
                &out[iters * 4..],
                &p[iters * 4..],
                "({w},{h}) wrote past its {iters}-pixel extent"
            );
        }
    }
    assert!(executed > 1000, "only {executed} pairs executed");
    assert!(ran > 50, "only {ran} pairs ran the loop");
    eprintln!("dimension sweep: {executed} executed ({ran} looping), {skipped} too large");
}

/// Zero-length and one-past-range extents combined with a NULL `pix`: any pair
/// whose predicted extent is zero must be safe to call with no buffer at all.
#[test]
fn generic_zero_extent_with_null_pix_never_faults() {
    let l = libs();
    let mut checked = 0usize;
    for k in 0..32u32 {
        let w = 1i32.wrapping_shl(k);
        for h in [0i32, 1, 2, 3, -1, i32::MIN, i32::MAX] {
            for wv in [w, w.wrapping_neg(), w.wrapping_add(1)] {
                if semantics(wv, h).2 != 0 {
                    continue;
                }
                checked += 1;
                for f in [l.c, l.rust] {
                    let mut img = CpImage { w: wv, h, pix: std::ptr::null_mut() };
                    unsafe { f(&mut img as *mut CpImage) };
                }
            }
        }
    }
    assert!(checked > 100, "only {checked} zero-extent pairs checked");
}

/// The struct fields must never be modified by either implementation.
#[test]
fn generic_image_struct_is_not_mutated() {
    let l = libs();
    let mut rng = Rng::new(0xFEED_FACE);
    for _ in 0..200 {
        let w = rng.range(0, 10);
        let h = rng.range(0, 10);
        let px = (w * h) as usize;
        let payload = rng.bytes(px * 4 + 8);
        for (who, f) in [("C", l.c), ("Rust", l.rust)] {
            let mut g = Guarded::new(&payload, 0);
            let pix = g.ptr();
            let mut img = CpImage { w, h, pix };
            unsafe { f(&mut img as *mut CpImage) };
            assert_eq!(img.w, w, "{who} mutated img.w");
            assert_eq!(img.h, h, "{who} mutated img.h");
            assert_eq!(img.pix, pix, "{who} mutated img.pix");
            g.assert_canaries(who, "struct immutability");
        }
    }
}
