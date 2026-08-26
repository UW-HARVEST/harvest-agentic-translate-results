//! Phase C — error-path differential tests, one test per row of `ERRORS.md`.
//!
//! `premultiply` returns `void` and contains no `if`, no `assert`, no `return`
//! and no null check (see `ERRORS.md` for the mechanical grep), so every
//! rejection is either
//!   * implicit — the loop guard is false and the call is a bitwise no-op, or
//!   * fatal    — an unchecked dereference raises `SIGSEGV`.
//!
//! The fatal rows are compared by re-executing this test binary as a child
//! process and asserting that the C child and the Rust child die with the
//! **same signal**.

mod harness;

use harness::*;
use std::os::unix::process::ExitStatusExt;
use std::process::Command;

const GUARD_BYTE: u8 = 0x5A;
const CRASH_ENV: &str = "PREMULT_CRASH_CASE";

fn payload(px: usize, seed: u64) -> Vec<u8> {
    let mut rng = Rng::new(seed);
    let mut v = vec![GUARD_BYTE; 64 * 2 + 4 * px];
    rng.fill(&mut v[64..64 + 4 * px]);
    v
}

// ===========================================================================
// Fatal-row plumbing (rows 1 and 2)
// ===========================================================================

/// The worker that actually performs a faulting call. In a normal test run the
/// environment variable is absent and this is a no-op, so `cargo test` never
/// crashes; the parent tests set the variable on the children they spawn.
#[test]
fn zz_crash_worker() {
    let spec = match std::env::var(CRASH_ENV) {
        Ok(s) => s,
        Err(_) => return, // normal run: nothing to do
    };
    let (case, which) = spec.split_once(':').expect("spec must be `case:which`");
    let f = match which {
        "c" => c_fn(),
        "rust" => rust_fn(),
        other => panic!("unknown library selector {other:?}"),
    };
    // Make sure the fault is reported promptly and not buffered away.
    eprintln!("crash worker: case={case} lib={which}");
    unsafe {
        match case {
            // ERRORS.md row 1: img == NULL
            "null_img" => f(std::ptr::null_mut()),
            // ERRORS.md row 2: img->pix == NULL with end > 0
            "null_pix_work" => {
                let mut img = CpImage {
                    w: 1,
                    h: 1,
                    pix: std::ptr::null_mut(),
                };
                f(&mut img);
            }
            other => panic!("unknown crash case {other:?}"),
        }
    }
    // Reaching here means no fault occurred.
    eprintln!("crash worker: survived");
    std::process::exit(7);
}

#[derive(Debug, PartialEq, Eq)]
struct Outcome {
    signal: Option<i32>,
    code: Option<i32>,
}

fn run_crash_child(case: &str, which: &str) -> Outcome {
    let exe = std::env::current_exe().expect("current_exe");
    let out = Command::new(exe)
        .args([
            "--exact",
            "zz_crash_worker",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(CRASH_ENV, format!("{case}:{which}"))
        .env("PREMULT_C_SO", c_so_path())
        .env("PREMULT_RUST_SO", rust_so_path())
        .output()
        .expect("spawn crash child");
    Outcome {
        signal: out.status.signal(),
        code: out.status.code(),
    }
}

fn assert_same_fatal(row: &str, case: &str) {
    let c = run_crash_child(case, "c");
    let r = run_crash_child(case, "rust");
    assert_eq!(
        c, r,
        "{row}: C and Rust did not fail identically for case `{case}` \
         (C = {c:?}, Rust = {r:?})"
    );
    assert_eq!(
        c.signal,
        Some(11),
        "{row}: expected both to die with SIGSEGV (11) for case `{case}`, got {c:?}"
    );
}

// ---------------------------------------------------------------------------
// Row 1 — img == NULL
// ---------------------------------------------------------------------------

#[test]
fn err01_null_img_segv_parity() {
    if std::env::var(CRASH_ENV).is_ok() {
        return;
    }
    assert_same_fatal("ERRORS.md row 1", "null_img");
}

// ---------------------------------------------------------------------------
// Row 2 — img->pix == NULL with end > 0
// ---------------------------------------------------------------------------

#[test]
fn err02_null_pix_with_work_segv_parity() {
    if std::env::var(CRASH_ENV).is_ok() {
        return;
    }
    assert!(c_end(1, 1) > 0, "row 2 precondition: end must be > 0");
    assert_same_fatal("ERRORS.md row 2", "null_pix_work");
}

// ---------------------------------------------------------------------------
// Row 3 — img->pix == NULL with end <= 0  (returns normally)
// ---------------------------------------------------------------------------

#[test]
fn err03_null_pix_no_work_ok() {
    for &(w, h) in &[(0i32, 0i32), (0, 5), (5, 0), (-1, 1), (1, -1), (i32::MAX, 1)] {
        assert!(c_end(w, h) <= 0, "row 3 precondition (w={w},h={h})");
        let mut ci = CpImage {
            w,
            h,
            pix: std::ptr::null_mut(),
        };
        let mut ri = CpImage {
            w,
            h,
            pix: std::ptr::null_mut(),
        };
        unsafe {
            (c_fn())(&mut ci);
            (rust_fn())(&mut ri);
        }
        assert_eq!(
            (ci.w, ci.h, ci.pix as usize),
            (ri.w, ri.h, ri.pix as usize),
            "row 3: struct diverged for w={w},h={h}"
        );
        assert_eq!((ci.w, ci.h, ci.pix as usize), (w, h, 0), "row 3: struct mutated");
    }
}

// ---------------------------------------------------------------------------
// Row 4 — wild non-null `pix` with end <= 0 (never dereferenced)
// ---------------------------------------------------------------------------

#[test]
fn err04_wild_pix_no_work_ok() {
    let wild: usize = 0xDEAD_BEEF_0000_0001;
    for &(w, h) in &[
        (0i32, 0i32),
        (0, 1),
        (1, 0),
        (-3, 4),
        (4, -3),
        (0x4000_0000, 1000),
        (0x2000_0000, 2),
        (0x2000_0000, 3),
        (1, i32::MAX),
        (2, i32::MIN),
        (i32::MIN, 1),
    ] {
        assert!(c_end(w, h) <= 0, "row 4 precondition (w={w},h={h})");
        let mut ci = CpImage {
            w,
            h,
            pix: wild as *mut CpPixel,
        };
        let mut ri = CpImage {
            w,
            h,
            pix: wild as *mut CpPixel,
        };
        unsafe {
            (c_fn())(&mut ci);
            (rust_fn())(&mut ri);
        }
        assert_eq!(
            (ci.w, ci.h, ci.pix as usize),
            (ri.w, ri.h, ri.pix as usize),
            "row 4: struct diverged for w={w},h={h}"
        );
        assert_eq!(
            (ci.w, ci.h, ci.pix as usize),
            (w, h, wild),
            "row 4: struct mutated"
        );
    }
}

// ---------------------------------------------------------------------------
// Row 5 — w == 0 (zero length) with h > 0
// ---------------------------------------------------------------------------

#[test]
fn err05_zero_w_noop() {
    for h in [1i32, 2, 3, 7, 64, 1000, 1_000_000] {
        assert_eq!(c_end(0, h), 0, "row 5 precondition");
        for s in 0..4u64 {
            let buf = payload(32, SEED ^ 0x105 ^ s);
            assert_noop("ERRORS.md row 5 (w=0)", 0, h, &buf, 64);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 6 — h == 0 (zero length) with w > 0
// ---------------------------------------------------------------------------

#[test]
fn err06_zero_h_noop() {
    for w in [1i32, 2, 3, 7, 64, 1000, 1_000_000] {
        assert_eq!(c_end(w, 0), 0, "row 6 precondition");
        for s in 0..4u64 {
            let buf = payload(32, SEED ^ 0x106 ^ s);
            assert_noop("ERRORS.md row 6 (h=0)", w, 0, &buf, 64);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 7 — w < 0, h > 0 with |4*w*h| < 2^31 (no wrap)  ->  end < 0  ->  no-op
// ---------------------------------------------------------------------------

#[test]
fn err07_neg_w_pos_h_noop() {
    for w in [-1i32, -2, -3, -7, -64, -1000, -100_000] {
        for h in [1i32, 2, 3, 7, 64, 1000] {
            // Only the non-wrapping magnitudes belong to this row.
            if (w as i64) * (h as i64) * 4 < i32::MIN as i64 {
                continue;
            }
            assert!(c_end(w, h) < 0, "row 7 precondition (w={w},h={h})");
            let buf = payload(32, SEED ^ 0x107 ^ (w as u64) ^ (h as u64));
            assert_noop("ERRORS.md row 7 (w<0,h>0)", w, h, &buf, 64);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 8 — w > 0, h < 0 with |4*w*h| < 2^31 (no wrap)  ->  end < 0  ->  no-op
// ---------------------------------------------------------------------------

#[test]
fn err08_pos_w_neg_h_noop() {
    for w in [1i32, 2, 3, 7, 64, 1000, 100_000] {
        for h in [-1i32, -2, -3, -7, -64, -1000] {
            if (w as i64) * (h as i64) * 4 < i32::MIN as i64 {
                continue;
            }
            assert!(c_end(w, h) < 0, "row 8 precondition (w={w},h={h})");
            let buf = payload(32, SEED ^ 0x108 ^ (w as u64) ^ (h as u64));
            assert_noop("ERRORS.md row 8 (w>0,h<0)", w, h, &buf, 64);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 20 — mixed-sign dimensions whose 32-bit wrap makes `end` POSITIVE, so
// the "obviously invalid" negative dimension is silently accepted and pixels
// really are processed.
// ---------------------------------------------------------------------------

#[test]
fn err20_mixed_sign_wraps_to_positive() {
    let mut rng = Rng::new(SEED ^ 0x120);
    // w < 0, h > 0 with end > 0
    let neg_pos: &[(i32, i32)] = &[
        (-0x3FFF_FFFF, 1),   // stride wraps to +4  -> end = 4
        (-0x3FFF_FFFF, 3),   // end = 12
        (-0x3FFF_FF00, 1),   // stride wraps to +1024 -> end = 1024
        (-0x3FFF_FF00, 2),   // end = 2048
        (-1_000_000, 1000),  // 4*w*h wraps to +294_967_296 (skipped: too big)
    ];
    // w > 0, h < 0 with end > 0
    let pos_neg: &[(i32, i32)] = &[
        (3, -357_913_941),   // 12 * -357913941 == -(2^32 - 4) -> end = 4
        (1, -0x3FFF_FFFF),   // stride 4, end = 4
        (2, -0x3FFF_FFFF),   // stride 8, end = 8
        (4, -0x1FFF_FFFF),   // stride 16, end = 16
    ];

    let mut worked = 0usize;
    let mut skipped = 0usize;
    for &(w, h) in neg_pos.iter().chain(pos_neg.iter()) {
        let end = c_end(w, h);
        assert!(
            end > 0,
            "row 20 precondition: end must wrap positive (w={w},h={h},end={end})"
        );
        for _ in 0..4 {
            match run_combo("ERRORS.md row 20 (mixed sign, wrapped positive)", w, h, 64, &mut rng)
            {
                None => {
                    skipped += 1;
                    break;
                }
                Some(did) => {
                    if did {
                        worked += 1;
                    }
                }
            }
        }
    }
    assert!(
        worked > 0,
        "row 20: expected mixed-sign wrapped-positive combinations to process pixels"
    );
    assert_eq!(skipped, 1, "row 20: exactly the >8 MiB combination may be skipped");
}

// ---------------------------------------------------------------------------
// Row 9 — w < 0 AND h < 0  ->  NOT rejected: |w*h| pixels processed
// ---------------------------------------------------------------------------

#[test]
fn err09_neg_w_neg_h_processes() {
    let mut any_work = false;
    for w in [-1i32, -2, -3, -7, -32] {
        for h in [-1i32, -2, -3, -7, -32] {
            let end = c_end(w, h);
            assert!(end > 0, "row 9 precondition (w={w},h={h})");
            let px = (end / 4) as usize;
            assert_eq!(px, (w * h) as usize, "row 9: pixel count");
            for s in 0..4u64 {
                let buf = payload(px, SEED ^ 0x109 ^ (w as u64) ^ (h as u64) ^ s);
                let out = assert_same("ERRORS.md row 9 (w<0,h<0)", w, h, &buf, 64);
                if out != buf {
                    any_work = true;
                }
                // Bytes outside [64, 64+4*px) are untouched.
                assert_eq!(&out[..64], &buf[..64], "row 9: leading guard modified");
                assert_eq!(
                    &out[64 + 4 * px..],
                    &buf[64 + 4 * px..],
                    "row 9: trailing guard modified"
                );
            }
        }
    }
    assert!(any_work, "row 9: expected real work to happen");
}

// ---------------------------------------------------------------------------
// Row 10 — w == INT_MAX
// ---------------------------------------------------------------------------

#[test]
fn err10_w_int_max_noop() {
    assert_eq!(c_end(i32::MAX, 1), -4, "row 10 precondition: stride wraps to -4");
    for h in [1i32, 2, 3, 1000] {
        assert!(c_end(i32::MAX, h) < 0, "row 10 precondition (h={h})");
        let buf = payload(32, SEED ^ 0x110 ^ (h as u64));
        assert_noop("ERRORS.md row 10 (w=INT_MAX)", i32::MAX, h, &buf, 64);
    }
}

// ---------------------------------------------------------------------------
// Row 11 — w == INT_MIN
// ---------------------------------------------------------------------------

#[test]
fn err11_w_int_min_noop() {
    for h in [-2i32, -1, 0, 1, 2, 3, 1000, i32::MAX, i32::MIN] {
        assert_eq!(c_end(i32::MIN, h), 0, "row 11 precondition (h={h})");
        let buf = payload(32, SEED ^ 0x111 ^ (h as u64));
        assert_noop("ERRORS.md row 11 (w=INT_MIN)", i32::MIN, h, &buf, 64);
    }
}

// ---------------------------------------------------------------------------
// Row 12 — w == 0x40000000: w*4 overflows to exactly 0
// ---------------------------------------------------------------------------

#[test]
fn err12_w_stride_overflow_to_zero_noop() {
    let w = 0x4000_0000i32;
    assert_eq!(w.wrapping_mul(4), 0, "row 12 precondition: stride == 0");
    for h in [-1000i32, -2, -1, 0, 1, 2, 3, 7, 1000, i32::MAX, i32::MIN] {
        assert_eq!(c_end(w, h), 0, "row 12 precondition (h={h})");
        let buf = payload(32, SEED ^ 0x112 ^ (h as u64));
        assert_noop("ERRORS.md row 12 (stride wraps to 0)", w, h, &buf, 64);
    }
}

// ---------------------------------------------------------------------------
// Row 13 — w == 0x40000001: w*4 wraps to 4 -> exactly 1 pixel processed
// ---------------------------------------------------------------------------

#[test]
fn err13_w_stride_wrap_to_four_processes_one() {
    let w = 0x4000_0001i32;
    assert_eq!(w.wrapping_mul(4), 4, "row 13 precondition: stride == 4");
    assert_eq!(c_end(w, 1), 4, "row 13 precondition: end == 4");

    let mut any_work = false;
    for s in 0..256u64 {
        let buf = payload(2, SEED ^ 0x113 ^ s);
        let out = assert_same("ERRORS.md row 13 (stride wraps to 4)", w, 1, &buf, 64);
        // Exactly one pixel processed: bytes 64..68 may change, 68.. must not.
        assert_eq!(&out[68..], &buf[68..], "row 13: touched more than 1 pixel");
        assert_eq!(&out[..64], &buf[..64], "row 13: leading guard modified");
        assert_eq!(out[67], buf[67], "row 13: alpha modified");
        if out != buf {
            any_work = true;
        }
    }
    assert!(any_work, "row 13: expected exactly one pixel to be processed");

    // And with h>1 the wrapped stride yields end = 4*h pixels.
    for h in [2i32, 3, 7, 100] {
        let px = h as usize;
        assert_eq!(c_end(w, h), 4 * h, "row 13 precondition (h={h})");
        let buf = payload(px + 1, SEED ^ 0x1130 ^ (h as u64));
        let out = assert_same("ERRORS.md row 13 (stride=4, h>1)", w, h, &buf, 64);
        assert_eq!(
            &out[64 + 4 * px..],
            &buf[64 + 4 * px..],
            "row 13: touched more than {px} pixels"
        );
    }
}

// ---------------------------------------------------------------------------
// Row 14 — w == 0x20000000, h == 2: end overflows to 0
// ---------------------------------------------------------------------------

#[test]
fn err14_end_overflow_to_zero_noop() {
    let w = 0x2000_0000i32;
    assert_eq!(w.wrapping_mul(4), i32::MIN, "row 14 precondition: stride == INT_MIN");
    assert_eq!(c_end(w, 2), 0, "row 14 precondition: end == 0");
    for s in 0..8u64 {
        let buf = payload(32, SEED ^ 0x114 ^ s);
        assert_noop("ERRORS.md row 14 (end wraps to 0)", w, 2, &buf, 64);
    }
    // Every even h gives end == 0 as well.
    for h in [4i32, 6, 8, 16, 1000, i32::MIN] {
        assert_eq!(c_end(w, h), 0, "row 14 precondition (h={h})");
        let buf = payload(32, SEED ^ 0x1140 ^ (h as u64));
        assert_noop("ERRORS.md row 14 (end wraps to 0)", w, h, &buf, 64);
    }
}

// ---------------------------------------------------------------------------
// Row 15 — w == 0x20000000, h == 3: end overflows to INT_MIN
// ---------------------------------------------------------------------------

#[test]
fn err15_end_overflow_to_int_min_noop() {
    let w = 0x2000_0000i32;
    assert_eq!(c_end(w, 3), i32::MIN, "row 15 precondition: end == INT_MIN");
    for s in 0..8u64 {
        let buf = payload(32, SEED ^ 0x115 ^ s);
        assert_noop("ERRORS.md row 15 (end wraps to INT_MIN)", w, 3, &buf, 64);
    }
    // Every odd h gives end == INT_MIN.
    for h in [1i32, 5, 7, 9, 1001, i32::MAX] {
        assert_eq!(c_end(w, h), i32::MIN, "row 15 precondition (h={h})");
        let buf = payload(32, SEED ^ 0x1150 ^ (h as u64));
        assert_noop("ERRORS.md row 15 (end wraps to INT_MIN)", w, h, &buf, 64);
    }
}

// ---------------------------------------------------------------------------
// Row 16 — h == INT_MAX with w == 1
// ---------------------------------------------------------------------------

#[test]
fn err16_h_int_max_noop() {
    assert_eq!(c_end(1, i32::MAX), -4, "row 16 precondition: end == -4");
    for w in [1i32, 2, 3, 5, 1000] {
        assert!(c_end(w, i32::MAX) < 0, "row 16 precondition (w={w})");
        let buf = payload(32, SEED ^ 0x116 ^ (w as u64));
        assert_noop("ERRORS.md row 16 (h=INT_MAX)", w, i32::MAX, &buf, 64);
    }
}

// ---------------------------------------------------------------------------
// Row 17 — h == INT_MIN with w == 2
// ---------------------------------------------------------------------------

#[test]
fn err17_h_int_min_noop() {
    assert_eq!(c_end(2, i32::MIN), 0, "row 17 precondition: end == 0");
    for w in [1i32, 2, 3, 4, 5, 1000, -1, -7] {
        assert_eq!(c_end(w, i32::MIN), 0, "row 17 precondition (w={w})");
        let buf = payload(32, SEED ^ 0x117 ^ (w as u64));
        assert_noop("ERRORS.md row 17 (h=INT_MIN)", w, i32::MIN, &buf, 64);
    }
}

// ---------------------------------------------------------------------------
// Row 18 — touched extent is exactly [0, 4*w*h) and alpha is never written
// ---------------------------------------------------------------------------

#[test]
fn err18_touched_extent_is_exact() {
    let mut rng = Rng::new(SEED ^ 0x118);
    for &(w, h) in &[
        (1i32, 1i32),
        (2, 1),
        (1, 2),
        (3, 5),
        (16, 16),
        (127, 3),
        (1, 1000),
    ] {
        let px = (w * h) as usize;
        for _ in 0..8 {
            // Buffer holds 4*px + 256 bytes; only the first 4*px may change.
            let n = 4 * px;
            let mut buf = vec![GUARD_BYTE; 64 + n + 256];
            rng.fill(&mut buf[64..64 + n + 256]);
            let snapshot = buf.clone();
            let out = assert_same("ERRORS.md row 18 (extent)", w, h, &buf, 64);
            assert_eq!(&out[..64], &snapshot[..64], "row 18: bytes before pix changed");
            assert_eq!(
                &out[64 + n..],
                &snapshot[64 + n..],
                "row 18: bytes at/after 4*w*h changed ({w}x{h})"
            );
            for k in 0..px {
                assert_eq!(
                    out[64 + 4 * k + 3],
                    snapshot[64 + 4 * k + 3],
                    "row 18: alpha of pixel {k} was written ({w}x{h})"
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 19 — "out-of-range enum across FFI": N/A, but exercise every extreme
// int value of the only two scalar fields as a belt-and-braces sweep.
// ---------------------------------------------------------------------------

#[test]
fn err19_extreme_scalar_field_sweep() {
    let mut rng = Rng::new(SEED ^ 0x119);
    const BUF_PX: usize = 4096;
    let extremes: &[i32] = &[
        i32::MIN,
        i32::MIN + 1,
        i32::MIN + 2,
        -0x4000_0001,
        -0x4000_0000,
        -0x3FFF_FFFF,
        -0x2000_0001,
        -0x2000_0000,
        -1000,
        -3,
        -2,
        -1,
        0,
        1,
        2,
        3,
        1000,
        0x1FFF_FFFF,
        0x2000_0000,
        0x2000_0001,
        0x3FFF_FFFF,
        0x4000_0000,
        0x4000_0001,
        0x7FFF_FFFE,
        i32::MAX,
    ];
    let mut checked = 0usize;
    let mut skipped = 0usize;
    for &w in extremes {
        for &h in extremes {
            let end = c_end(w, h);
            let px = if end > 0 { (end / 4) as usize } else { 0 };
            if px > BUF_PX {
                skipped += 1;
                continue;
            }
            checked += 1;
            let n = 4 * px.max(1);
            let mut buf = vec![GUARD_BYTE; 64 + n + 64];
            rng.fill(&mut buf[64..64 + n]);
            let out = assert_same("ERRORS.md row 19 (extreme scalars)", w, h, &buf, 64);
            assert_eq!(&out[..64], &buf[..64], "row 19: leading guard modified");
            assert_eq!(
                &out[64 + n..],
                &buf[64 + n..],
                "row 19: trailing guard modified (w={w},h={h})"
            );
        }
    }
    assert!(
        checked > 500,
        "row 19: expected most of the {}x{} matrix to be checked, only {checked} were",
        extremes.len(),
        extremes.len()
    );
    eprintln!("row 19: checked {checked}, skipped {skipped} (buffer > {BUF_PX} px)");
}
