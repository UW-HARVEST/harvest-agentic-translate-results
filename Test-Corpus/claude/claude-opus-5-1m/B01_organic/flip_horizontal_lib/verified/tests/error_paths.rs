//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md`. The library has no error-return mechanism
//! (`flip_horizontal` is `void`, with no asserts, no null checks and no range
//! checks), so every rejection is either
//!
//!   * (S) a silent no-op — a loop guard is false on entry, and the buffer plus
//!     the `cp_image_t` must come back byte-identical, or
//!   * (V) a fatal memory fault — an unconditional dereference of an invalid
//!     pointer.
//!
//! (S) rows are asserted with `assert_same_and_noop`. (V) rows are asserted by
//! re-executing this very test binary as a child process for each
//! implementation and comparing the *terminating signal*, so "both rejected the
//! input the same way" is a real equality check and not merely "both failed".

mod common;

use common::{assert_same, assert_same_and_noop, both, Case, Lib, Rng};

use std::os::unix::process::ExitStatusExt;
use std::process::Command;

// ===========================================================================
// (V) fatal rows — ERRORS 1 and 2, compared across subprocesses
// ===========================================================================

const CRASH_ENV: &str = "DIFFTEST_CRASH";
/// Exit code used by the child when the call unexpectedly *returned*.
const NO_FAULT_EXIT: i32 = 42;

#[derive(Debug, PartialEq, Eq)]
enum Outcome {
    /// Terminated by signal N (e.g. 11 = SIGSEGV).
    Signal(i32),
    /// Exited normally with this code.
    Exit(i32),
}

/// Hidden child entry point. Runs one fatal case in its own process.
///
/// Invoked as: `<this-test-binary> --ignored --exact crash_child_entry`
/// with `DIFFTEST_CRASH=<case>` in the environment.
#[test]
#[ignore = "internal child-process entry point; driven by the (V) row tests"]
fn crash_child_entry() {
    let which = match std::env::var(CRASH_ENV) {
        Ok(v) => v,
        // Someone ran `cargo test -- --ignored` directly: nothing to do.
        Err(_) => return,
    };

    let lib = match which.as_str() {
        "c_null_img" | "c_null_pix" => Lib::c(),
        "rust_null_img" | "rust_null_pix" => Lib::rust(),
        other => panic!("unknown {CRASH_ENV} value {other:?}"),
    };

    match which.as_str() {
        // ERRORS 1: img == NULL.
        "c_null_img" | "rust_null_img" => unsafe { lib.flip(std::ptr::null_mut()) },
        // ERRORS 2: img->pix == NULL with w >= 1 and h >= 2 (both loops entered).
        "c_null_pix" | "rust_null_pix" => {
            let mut img = common::CpImage { w: 4, h: 4, pix: std::ptr::null_mut() };
            unsafe { lib.flip(&mut img) };
        }
        _ => unreachable!(),
    }

    // Reached only if no fault occurred.
    std::process::exit(NO_FAULT_EXIT);
}

fn run_crash_child(which: &str) -> Outcome {
    let exe = std::env::current_exe().expect("current_exe");
    let out = Command::new(&exe)
        .args(["--exact", "crash_child_entry", "--ignored", "--test-threads=1", "--nocapture"])
        .env(CRASH_ENV, which)
        .output()
        .unwrap_or_else(|e| panic!("failed to spawn child {}: {e}", exe.display()));

    match out.status.signal() {
        Some(sig) => Outcome::Signal(sig),
        None => Outcome::Exit(out.status.code().unwrap_or(-1)),
    }
}

/// ERRORS 1: `img == NULL`. The C dereferences `img->pix` with no null check.
/// Both implementations must die with the same signal.
#[test]
fn err01_null_img_segv() {
    let c = run_crash_child("c_null_img");
    let r = run_crash_child("rust_null_img");
    assert_eq!(
        c, r,
        "img==NULL rejected differently: C={c:?} Rust={r:?}"
    );
    assert_eq!(
        c,
        Outcome::Signal(libc_sigsegv()),
        "expected both to fault with SIGSEGV, got {c:?}"
    );
}

/// ERRORS 2: `img->pix == NULL` with `w >= 1, h >= 2`; `*a` at address 0.
#[test]
fn err02_null_pix_with_work_segv() {
    let c = run_crash_child("c_null_pix");
    let r = run_crash_child("rust_null_pix");
    assert_eq!(
        c, r,
        "img->pix==NULL (with work to do) rejected differently: C={c:?} Rust={r:?}"
    );
    assert_eq!(
        c,
        Outcome::Signal(libc_sigsegv()),
        "expected both to fault with SIGSEGV, got {c:?}"
    );
}

/// SIGSEGV on Linux.
fn libc_sigsegv() -> i32 {
    11
}

// ===========================================================================
// (S) silent-no-op rows — ERRORS 3-19
// ===========================================================================

/// ERRORS 3: no bounds check exists. Proven by CONFIGS 20 (`cfg_padding_canary`
/// in `differential.rs`); repeated here so the row has a test in this file too:
/// with an oversized buffer both implementations write the identical range.
#[test]
fn err03_no_bounds_check_same_footprint() {
    let (c, r) = both();
    let mut rng = Rng::new(0xE003);
    for i in 0..100 {
        let w = rng.range_i32(1, 16);
        let h = rng.range_i32(0, 16);
        let used = w as usize * h as usize;
        let mut data = rng.bytes(used * common::PIXEL_SIZE);
        data.extend(std::iter::repeat_n(0x5Au8, 32 * common::PIXEL_SIZE));
        let case = Case { w, h, data, null_pix: false, calls: 1 };
        assert_same(&c, &r, &case, &format!("err03[{i}] w={w} h={h}"));
    }
}

/// ERRORS 4: `h == 0` -> `flips == 0`.
#[test]
fn err04_h_zero() {
    let (c, r) = both();
    let mut rng = Rng::new(0xE004);
    for &w in &[1i32, 2, 3, 64, 0, -1, i32::MAX, i32::MIN] {
        let case = Case::sized(&mut rng, w, 0, 64);
        assert_same_and_noop(&c, &r, &case, &format!("err04 w={w}"));
    }
}

/// ERRORS 5: `h == 1` -> `flips == 1/2 == 0`.
#[test]
fn err05_h_one() {
    let (c, r) = both();
    let mut rng = Rng::new(0xE005);
    for &w in &[1i32, 2, 3, 64, 0, -1, i32::MAX, i32::MIN] {
        let case = Case::sized(&mut rng, w, 1, 64);
        assert_same_and_noop(&c, &r, &case, &format!("err05 w={w}"));
    }
}

/// ERRORS 6: `h == -1` -> `flips == -1/2 == 0` (C truncates toward zero).
#[test]
fn err06_h_neg_one() {
    let (c, r) = both();
    let mut rng = Rng::new(0xE006);
    for &w in &[1i32, 2, 64, 0, -1, i32::MAX, i32::MIN] {
        let case = Case::sized(&mut rng, w, -1, 64);
        assert_same_and_noop(&c, &r, &case, &format!("err06 w={w}"));
    }
}

/// ERRORS 7: `h == -2` -> `flips == -1`.
#[test]
fn err07_h_neg_two() {
    let (c, r) = both();
    let mut rng = Rng::new(0xE007);
    for &w in &[1i32, 2, 64, 0, -1, i32::MAX, i32::MIN] {
        let case = Case::sized(&mut rng, w, -2, 64);
        assert_same_and_noop(&c, &r, &case, &format!("err07 w={w}"));
    }
}

/// ERRORS 8: randomized negative `h`.
#[test]
fn err08_h_negative_random() {
    let (c, r) = both();
    let mut rng = Rng::new(0xE008);
    for i in 0..200 {
        let h = rng.range_i32(i32::MIN, -1);
        let w = rng.range_i32(-8, 64);
        let case = Case::sized(&mut rng, w, h, 64);
        assert_same_and_noop(&c, &r, &case, &format!("err08[{i}] w={w} h={h}"));
    }
}

/// ERRORS 9: `h == INT_MIN` -> `flips == -1073741824` (note `INT_MIN / 2` is
/// representable, so this is not the `INT_MIN / -1` overflow trap).
#[test]
fn err09_h_int_min() {
    let (c, r) = both();
    let mut rng = Rng::new(0xE009);
    for &w in &[1i32, 2, 64, 0, -1, i32::MAX, i32::MIN] {
        let case = Case::sized(&mut rng, w, i32::MIN, 64);
        assert_same_and_noop(&c, &r, &case, &format!("err09 w={w}"));
    }
}

/// ERRORS 10: `w == 0` -> inner guard false; `pix` is never dereferenced, so
/// even `pix == NULL` is accepted silently.
#[test]
fn err10_w_zero() {
    let (c, r) = both();
    let mut rng = Rng::new(0xE00A);
    for &h in &[2i32, 3, 4, 5, 64, 255, 1024] {
        let case = Case::sized(&mut rng, 0, h, 64);
        assert_same_and_noop(&c, &r, &case, &format!("err10 h={h}"));
        let case_null = Case::null_pix(0, h);
        assert_same_and_noop(&c, &r, &case_null, &format!("err10-null h={h}"));
    }
}

/// ERRORS 11: `w == -1` -> inner guard `0 < -1` false; an out-of-range pointer
/// is computed but never dereferenced.
#[test]
fn err11_w_neg_one() {
    let (c, r) = both();
    let mut rng = Rng::new(0xE00B);
    for &h in &[2i32, 3, 4, 5, 64, 255] {
        let case = Case::sized(&mut rng, -1, h, 64);
        assert_same_and_noop(&c, &r, &case, &format!("err11 h={h}"));
        let case_null = Case::null_pix(-1, h);
        assert_same_and_noop(&c, &r, &case_null, &format!("err11-null h={h}"));
    }
}

/// ERRORS 12: large negative `w` -> the address calculation wraps far below the
/// allocation. This is exactly the input on which Rust's `ptr::offset`
/// precondition check would abort while C proceeds silently.
#[test]
fn err12_w_large_negative() {
    let (c, r) = both();
    let mut rng = Rng::new(0xE00C);
    for &w in &[-(1i32 << 10), -(1 << 20), -(1 << 24), -(1 << 28), -(1 << 30), i32::MIN + 1] {
        for &h in &[2i32, 3, 8, 64] {
            let case = Case::sized(&mut rng, w, h, 64);
            assert_same_and_noop(&c, &r, &case, &format!("err12 w={w} h={h}"));
            let case_null = Case::null_pix(w, h);
            assert_same_and_noop(&c, &r, &case_null, &format!("err12-null w={w} h={h}"));
        }
    }
}

/// ERRORS 13: `w == INT_MIN` -> `w * i` signed-overflows in `int`.
#[test]
fn err13_w_int_min() {
    let (c, r) = both();
    let mut rng = Rng::new(0xE00D);
    for &h in &[2i32, 3, 4, 5, 6, 7, 8, 64, 255] {
        let case = Case::sized(&mut rng, i32::MIN, h, 64);
        assert_same_and_noop(&c, &r, &case, &format!("err13 h={h}"));
    }
}

/// ERRORS 14: `pix == NULL` while `h == 1` — the guard short-circuits before
/// `pix` is ever used, so this must NOT fault.
#[test]
fn err14_null_pix_h_one() {
    let (c, r) = both();
    for &h in &[i32::MIN, -3, -1, 0, 1] {
        for &w in &[1i32, 2, 64, i32::MAX, 0, -1, i32::MIN] {
            let case = Case::null_pix(w, h);
            assert_same_and_noop(&c, &r, &case, &format!("err14 w={w} h={h}"));
        }
    }
}

/// ERRORS 15: `h == INT_MAX`, `w == 0` -> `flips == 1073741823` outer
/// iterations, every inner guard false. Must be a no-op.
///
/// The C build is unoptimised, so this genuinely spins ~2^30 times; it is the
/// slowest test in the suite by design (it is the only way to reach the
/// `INT_MAX` boundary of the outer guard).
#[test]
fn err15_h_int_max_w_zero() {
    let (c, r) = both();
    let mut rng = Rng::new(0xE00F);
    let case = Case::sized(&mut rng, 0, i32::MAX, 64);
    assert_same_and_noop(&c, &r, &case, "err15 w=0 h=INT_MAX");

    // Same guard boundary, negative `w` (still no dereference).
    let case = Case::sized(&mut rng, -1, i32::MAX, 64);
    assert_same_and_noop(&c, &r, &case, "err15 w=-1 h=INT_MAX");
}

/// ERRORS 16: `w == INT_MAX`, `h == 0` -> `w` is never used.
#[test]
fn err16_w_int_max_h_zero() {
    let (c, r) = both();
    let mut rng = Rng::new(0xE010);
    for &h in &[0i32, 1, -1, -2, i32::MIN] {
        let case = Case::sized(&mut rng, i32::MAX, h, 64);
        assert_same_and_noop(&c, &r, &case, &format!("err16 h={h}"));
    }
}

/// ERRORS 17: `w == 1, h == 2` — one step past "empty", the smallest input that
/// does real work. Must NOT be a no-op, and must match.
#[test]
fn err17_smallest_working_input() {
    let (c, r) = both();
    let mut rng = Rng::new(0xE011);
    for i in 0..100 {
        // Force the two pixels to differ so a no-op would be detected.
        let mut data = rng.bytes(8);
        if data[..4] == data[4..] {
            data[0] ^= 0xFF;
        }
        let case = Case { w: 1, h: 2, data: data.clone(), null_pix: false, calls: 1 };
        assert_same(&c, &r, &case, &format!("err17[{i}]"));

        let mut buf = data.clone();
        let mut img = common::CpImage { w: 1, h: 2, pix: buf.as_mut_ptr().cast() };
        unsafe { r.flip(&mut img) };
        assert_ne!(buf, data, "err17: expected work to be done");
        assert_eq!(&buf[..4], &data[4..], "err17: pixel 0 should be old pixel 1");
        assert_eq!(&buf[4..], &data[..4], "err17: pixel 1 should be old pixel 0");
    }
}

/// ERRORS 18: the `h == 1` / `h == 2` boundary where `flips` becomes non-zero.
#[test]
fn err18_h_boundary_one_vs_two() {
    let (c, r) = both();
    let mut rng = Rng::new(0xE012);
    for i in 0..100 {
        let w = rng.range_i32(1, 16);
        // h == 1: no-op.
        let case1 = Case::sized(&mut rng, w, 1, (w as usize).max(1) * 2);
        assert_same_and_noop(&c, &r, &case1, &format!("err18-h1[{i}] w={w}"));
        // h == 2: exactly one swap.
        let case2 = Case::exact(&mut rng, w, 2);
        assert_same(&c, &r, &case2, &format!("err18-h2[{i}] w={w}"));
    }
}

/// ERRORS 19: the full sign matrix of `(w, h)`, including both negative. Every
/// combination with `w <= 0` or `h <= 1` must be a no-op in both.
#[test]
fn err19_sign_matrix() {
    let (c, r) = both();
    let mut rng = Rng::new(0xE013);
    let vals = [i32::MIN, -1000, -2, -1, 0, 1, 2, 1000, i32::MAX];
    for &w in &vals {
        for &h in &vals {
            let noop = w <= 0 || h <= 1;
            if !noop {
                // Both positive and > 1: real work, but INT_MAX-sized shapes
                // cannot be allocated. Covered by the CONFIGS rows instead.
                continue;
            }
            // Skip the two combinations that would spin ~2^30 empty outer
            // iterations (covered once, deliberately, by err15).
            if w <= 0 && h == i32::MAX {
                continue;
            }
            let case = Case::sized(&mut rng, w, h, 64);
            assert_same_and_noop(&c, &r, &case, &format!("err19 w={w} h={h}"));
            let case_null = Case::null_pix(w, h);
            assert_same_and_noop(&c, &r, &case_null, &format!("err19-null w={w} h={h}"));
        }
    }
}

// ===========================================================================
// Generic FFI-boundary boundaries (required even though not in the table)
// ===========================================================================

/// There are no enums in this API, so there is no invalid enum variant. The
/// nearest equivalent is an `int` field with no meaningful value: every such
/// value is swept here across the whole `i32` range (restricted to shapes that
/// do not dereference, so the sweep is safe to run in-process).
#[test]
fn generic_full_int_range_sweep() {
    let (c, r) = both();
    let mut rng = Rng::new(0xE100);
    for i in 0..1000 {
        // h <= 1 => zero outer trips, so any `w` bit pattern is safe.
        let w = rng.i32_any();
        let h = rng.range_i32(i32::MIN, 1);
        let case = Case::sized(&mut rng, w, h, 16);
        assert_same_and_noop(&c, &r, &case, &format!("generic-int[{i}] w={w} h={h}"));
    }
    for i in 0..1000 {
        // w <= 0 => zero inner trips, so any `h` in a bounded range is safe.
        let w = rng.range_i32(i32::MIN, 0);
        let h = rng.range_i32(2, 512);
        let case = Case::sized(&mut rng, w, h, 16);
        assert_same_and_noop(&c, &r, &case, &format!("generic-int2[{i}] w={w} h={h}"));
    }
}

/// Zero-length buffer with a non-null (dangling but aligned) pointer, which is
/// what a real caller gets from an empty allocation.
#[test]
fn generic_zero_length_buffer() {
    let (c, r) = both();
    for &(w, h) in &[(0i32, 0i32), (0, 1), (0, 2), (1, 0), (1, 1), (-1, 0), (-1, 2), (0, -1)] {
        let case = Case { w, h, data: Vec::new(), null_pix: false, calls: 1 };
        assert_same_and_noop(&c, &r, &case, &format!("generic-zerolen w={w} h={h}"));
    }
}

/// A MISALIGNED `cp_image_t *`. C loads the `int` fields with plain unaligned
/// loads (fine on x86-64); Rust must not trip an alignment check. This is the
/// alignment counterpart of the null-pointer row.
#[test]
fn generic_misaligned_image_struct() {
    let (c, r) = both();
    let mut rng = Rng::new(0xE102);
    let img_size = std::mem::size_of::<common::CpImage>();

    for skew in 1usize..8 {
        for &(w, h) in &[(1i32, 2i32), (3, 4), (4, 5), (0, 8), (-1, 8), (7, 1), (5, 0)] {
            let pixels = ((w.max(0) as usize) * (h.max(0) as usize)).max(64);
            // Returns (pixel buffer, w, h, pix-unchanged, trailing-padding
            // bytes of the struct allocation). The raw struct bytes themselves
            // are NOT compared across runs because they embed the `pix`
            // pointer, which legitimately differs between the two allocations.
            let run = |lib: &Lib| -> (Vec<u8>, i32, i32, bool, Vec<u8>) {
                let mut buf = vec![0u8; pixels * common::PIXEL_SIZE];
                for b in buf.iter_mut() {
                    *b = 0x3C;
                }
                let mut raw = vec![0xEEu8; img_size + 16];
                let img_ptr = unsafe { raw.as_mut_ptr().add(skew) } as *mut common::CpImage;
                let pix: *mut common::CpPixel = buf.as_mut_ptr().cast();
                let stored = common::CpImage { w, h, pix };
                unsafe { std::ptr::write_unaligned(img_ptr, stored) };
                unsafe { lib.flip(img_ptr) };
                // Read the struct back (unaligned) to confirm it was untouched.
                let after = unsafe { std::ptr::read_unaligned(img_ptr) };
                // Bytes of the allocation outside the struct must be pristine.
                let mut outside = raw[..skew].to_vec();
                outside.extend_from_slice(&raw[skew + img_size..]);
                (buf, after.w, after.h, after.pix == pix, outside)
            };
            let (bc, cw, ch, cpix, cout) = run(&c);
            let (br, rw, rh, rpix, rout) = run(&r);
            assert_eq!(bc, br, "misaligned struct: buffer diverged (skew={skew} w={w} h={h})");
            assert_eq!((cw, ch, cpix), (w, h, true), "C modified the struct (skew={skew})");
            assert_eq!(
                (cw, ch, cpix),
                (rw, rh, rpix),
                "misaligned struct: struct fields diverged (skew={skew} w={w} h={h})"
            );
            assert_eq!(cout, rout, "misaligned struct: bytes outside the struct diverged");
            assert!(cout.iter().all(|&b| b == 0xEE), "struct allocation overrun");
            let _ = &mut rng;
        }
    }
}

/// `img->pix` pointing into the MIDDLE of a larger allocation, with poison on
/// both sides. Verifies neither implementation walks outside `[pix, pix+w*h)`.
#[test]
fn generic_pix_offset_into_buffer() {
    let (c, r) = both();
    let mut rng = Rng::new(0xE103);
    for i in 0..100 {
        let w = rng.range_i32(1, 16);
        let h = rng.range_i32(0, 16);
        let used = w as usize * h as usize;
        let front = 1 + rng.below(16) as usize;
        let back = 1 + rng.below(16) as usize;
        let payload = rng.bytes(used * common::PIXEL_SIZE);

        let run = |lib: &Lib| -> Vec<u8> {
            let mut buf = vec![0u8; (front + used + back) * common::PIXEL_SIZE];
            for b in buf.iter_mut() {
                *b = 0xC3;
            }
            buf[front * common::PIXEL_SIZE..(front + used) * common::PIXEL_SIZE]
                .copy_from_slice(&payload);
            let pix = unsafe { buf.as_mut_ptr().add(front * common::PIXEL_SIZE) };
            let mut img = common::CpImage { w, h, pix: pix.cast() };
            unsafe { lib.flip(&mut img) };
            buf
        };
        let bc = run(&c);
        let br = run(&r);
        assert_eq!(bc, br, "offset-pix buffer diverged [{i}] (w={w} h={h})");
        // Poison on both sides must be intact.
        assert!(
            br[..front * common::PIXEL_SIZE].iter().all(|&b| b == 0xC3)
                && br[(front + used) * common::PIXEL_SIZE..].iter().all(|&b| b == 0xC3),
            "Rust wrote outside [pix, pix+w*h) [{i}] (w={w} h={h})"
        );
    }
}

/// Repeated calls on error-path shapes must stay no-ops.
#[test]
fn generic_repeated_calls_on_noop_shapes() {
    let (c, r) = both();
    let mut rng = Rng::new(0xE101);
    for &(w, h) in &[(0i32, 8i32), (-1, 8), (5, 1), (5, 0), (5, -5), (i32::MIN, 4)] {
        for calls in [1usize, 2, 3, 7] {
            let case = Case::sized(&mut rng, w, h, 64).with_calls(calls);
            assert_same_and_noop(&c, &r, &case, &format!("generic-rep w={w} h={h} calls={calls}"));
        }
    }
}
