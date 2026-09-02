//! Phase C — error-path differential tests.
//!
//! `ERRORS.md` has ZERO rows: the C contains no `return`, no `if`, no `assert`,
//! no null check and no range check, so there is no reported-error behavior to
//! match. `errors_table_is_empty_by_construction` re-derives that fact from the
//! C source at test time so the empty table cannot silently go stale.
//!
//! What remains — and what this file covers — is the generic-boundary table
//! (rows G1..G10 of `ERRORS.md`). The undefined-behavior boundaries (null
//! pointers) are compared by *fault signal*: each library is called in a forked
//! child and the two must die the same way. That is the only observable
//! "rejection" this API has, and it verifies the Rust has not added a check the
//! C lacks (which would make it return normally instead of faulting).

mod common;

use common::*;
use std::os::unix::process::ExitStatusExt;
use std::process::Command;

const SIGSEGV: i32 = 11;
const SIGBUS: i32 = 7;

// ---------------------------------------------------------------------------
// The empty-table justification, re-checked mechanically.
// ---------------------------------------------------------------------------

/// Re-derive the ERRORS.md row count from the C source: if anyone adds a
/// rejection path to the C, this fails and the table must be regenerated.
#[test]
fn errors_table_is_empty_by_construction() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let mut src = String::new();
    for f in ["c_src/src/lib.c", "c_src/include/lib.h"] {
        src.push_str(&std::fs::read_to_string(root.join(f)).expect("read C source"));
    }
    // Strip the one declaration/definition line noise we expect, then look for
    // any rejection construct.
    for needle in [
        "return", "assert", "NULL", "ERROR", "errno", "if (", "if(", "switch", "#if",
        "goto", "exit(", "abort", "MAX", "MIN", "<=", ">=", "-1",
    ] {
        assert!(
            !src.contains(needle),
            "C source now contains {needle:?} — it has gained a rejection/branch path. \
             Regenerate ERRORS.md and add a differential row for it."
        );
    }
}

// ---------------------------------------------------------------------------
// G1..G3 — null pointers. Compared by fault signal across a fork.
// ---------------------------------------------------------------------------

/// Run one ignored death-test in a child process and return its termination
/// signal (or `None` if it exited normally).
fn death_signal(test_name: &str) -> Option<i32> {
    let exe = std::env::current_exe().expect("current_exe");
    let out = Command::new(exe)
        .args(["--exact", test_name, "--ignored", "--test-threads=1", "--nocapture"])
        .env("DEATH_CHILD", "1")
        .output()
        .expect("spawn death-test child");
    out.status.signal()
}

fn assert_same_fault(label: &str, c_test: &str, rust_test: &str) {
    let c = death_signal(c_test);
    let r = death_signal(rust_test);
    assert!(
        c == Some(SIGSEGV) || c == Some(SIGBUS),
        "[{label}] C was expected to fault on the null pointer, got signal {c:?}"
    );
    assert_eq!(
        c, r,
        "[{label}] C and Rust do not reject identically: C signal {c:?}, Rust signal {r:?}. \
         A differing/absent signal means the Rust added or omitted a check the C does not have."
    );
}

/// G1 — `m == NULL`.
#[test]
fn g1_null_m_faults_in_both() {
    assert_same_fault("G1", "death_c_null_m", "death_rust_null_m");
}

/// G2 — `out == NULL`.
#[test]
fn g2_null_out_faults_in_both() {
    assert_same_fault("G2", "death_c_null_out", "death_rust_null_out");
}

/// G3 — both pointers NULL.
#[test]
fn g3_null_both_faults_in_both() {
    assert_same_fault("G3", "death_c_null_both", "death_rust_null_both");
}

// The death payloads. `#[ignore]`d so a normal run never executes them; the
// parent tests above invoke them explicitly with `--ignored --exact`.

fn null_m(which: Impl) {
    let f = digest(which);
    let mut out = [0u8; 16];
    unsafe { f(std::ptr::null(), out.as_mut_ptr()) };
    std::process::exit(0); // reached only if the impl silently tolerated NULL
}

fn null_out(which: Impl) {
    let f = digest(which);
    let m = Md5::new(0x1122_3344, 0x5566_7788, 0x99AA_BBCC, 0xDDEE_FF00);
    unsafe { f(&m as *const Md5, std::ptr::null_mut()) };
    std::process::exit(0);
}

fn null_both(which: Impl) {
    let f = digest(which);
    unsafe { f(std::ptr::null(), std::ptr::null_mut()) };
    std::process::exit(0);
}

#[test]
#[ignore = "death test: invoked as a child by g1_null_m_faults_in_both"]
fn death_c_null_m() {
    null_m(Impl::C);
}

#[test]
#[ignore = "death test: invoked as a child by g1_null_m_faults_in_both"]
fn death_rust_null_m() {
    null_m(Impl::Rust);
}

#[test]
#[ignore = "death test: invoked as a child by g2_null_out_faults_in_both"]
fn death_c_null_out() {
    null_out(Impl::C);
}

#[test]
#[ignore = "death test: invoked as a child by g2_null_out_faults_in_both"]
fn death_rust_null_out() {
    null_out(Impl::Rust);
}

#[test]
#[ignore = "death test: invoked as a child by g3_null_both_faults_in_both"]
fn death_c_null_both() {
    null_both(Impl::C);
}

#[test]
#[ignore = "death test: invoked as a child by g3_null_both_faults_in_both"]
fn death_rust_null_both() {
    null_both(Impl::Rust);
}

// ---------------------------------------------------------------------------
// G4 / G5 — no length parameter exists; the write width is fixed at 16.
// ---------------------------------------------------------------------------

/// G4 — exactly 16 bytes are written, never 15 and never 17. The output window
/// sits inside a poisoned arena with guard bytes on both sides; both libraries
/// must leave the guards untouched and identical. This is the only "oversized /
/// undersized length" check the API admits, since it takes no length argument
/// (G5 is vacuous for that reason).
#[test]
fn g4_writes_exactly_16_bytes_no_overrun() {
    let cf = digest(Impl::C);
    let rf = digest(Impl::Rust);
    let mut rng = Rng::new(0x60_0004);
    const GUARD: usize = 16;
    for _ in 0..ITERS {
        let m = rng.next_md5();
        let mut arena_c = [0x5Au8; GUARD + 16 + GUARD];
        let mut arena_r = [0x5Au8; GUARD + 16 + GUARD];
        unsafe {
            cf(&m as *const Md5, arena_c.as_mut_ptr().add(GUARD));
            rf(&m as *const Md5, arena_r.as_mut_ptr().add(GUARD));
        }
        assert_eq!(arena_c, arena_r, "[G4] arena divergence for {m:?}");
        assert!(
            arena_c[..GUARD].iter().all(|&b| b == 0x5A) && arena_c[GUARD + 16..].iter().all(|&b| b == 0x5A),
            "[G4] C wrote outside the 16-byte window: {arena_c:02x?}"
        );
        assert!(
            arena_r[..GUARD].iter().all(|&b| b == 0x5A) && arena_r[GUARD + 16..].iter().all(|&b| b == 0x5A),
            "[G4] Rust wrote outside the 16-byte window: {arena_r:02x?}"
        );
        assert!(
            arena_c[GUARD..GUARD + 16].iter().any(|&b| b != 0x5A) || m == Md5::new(0x5A5A5A5A, 0x5A5A5A5A, 0x5A5A5A5A, 0x5A5A5A5A),
            "[G4] window was not written at all"
        );
    }
}

// ---------------------------------------------------------------------------
// G6 — out-of-range enum / invalid bit patterns crossing the FFI boundary.
// ---------------------------------------------------------------------------

/// G6 — the API has no enum, mode or flag parameter, so an "out-of-range enum
/// variant" is not constructible. The equivalent real input class is an
/// arbitrary 16-byte struct image with no "valid" interpretation: the C accepts
/// every bit pattern, so the Rust must too (no niche-optimized field, no
/// validity assumption, no panic). All 2^128 images are sampled randomly plus
/// the adversarial patterns that would trip a `bool`/`NonZero`/enum field.
#[test]
fn g6_arbitrary_struct_bit_patterns() {
    let mut adversarial: Vec<[u8; 16]> = Vec::new();
    // Patterns that are invalid for bool (>1), for NonZero (0), for a
    // 4-variant enum (>=4), and for char (surrogates / >0x10FFFF).
    for &w in &[
        0x0000_0000u32,
        0x0000_0002,
        0x0000_00FF,
        0x0000_0004,
        0xFFFF_FFFF,
        0x0011_0000,
        0x0000_D800,
        0x8000_0000,
    ] {
        let mut img = [0u8; 16];
        for i in 0..4 {
            img[i * 4..i * 4 + 4].copy_from_slice(&w.to_le_bytes());
        }
        adversarial.push(img);
    }
    // Mixed: a different adversarial word in each field.
    adversarial.push({
        let mut img = [0u8; 16];
        img[0..4].copy_from_slice(&0u32.to_le_bytes());
        img[4..8].copy_from_slice(&2u32.to_le_bytes());
        img[8..12].copy_from_slice(&0xFFFF_FFFFu32.to_le_bytes());
        img[12..16].copy_from_slice(&0x0000_D800u32.to_le_bytes());
        img
    });

    for img in &adversarial {
        let m = Md5::from_image(img);
        assert_same("G6/adversarial", &m);
        assert_eq!(call(Impl::C, &m), *img);
        assert_eq!(call(Impl::Rust, &m), *img);
    }

    let mut rng = Rng::new(0x60_0006);
    for _ in 0..(ITERS * 4) {
        let img = rng.next_image();
        let m = Md5::from_image(&img);
        assert_same("G6/random-image", &m);
        assert_eq!(call(Impl::Rust, &m), img, "Rust rejected/altered a valid bit pattern");
    }
}

// ---------------------------------------------------------------------------
// G7 — value extremes and their wrap neighbours.
// ---------------------------------------------------------------------------

/// G7 — there is no index or length argument, so "one step past a valid range"
/// only exists in the value domain, whose valid range is all of `uint32_t`.
/// The extremes and their neighbours (`0`/`1`, `MAX`/`MAX-1`) plus the signed
/// boundary (`0x7FFFFFFF`/`0x80000000`, where a signed-vs-unsigned shift
/// mistranslation would show) are checked in every field position.
#[test]
fn g7_extreme_word_values() {
    const EXTREMES: [u32; 6] = [0, 1, 0x7FFF_FFFF, 0x8000_0000, 0xFFFF_FFFE, 0xFFFF_FFFF];
    for &v in &EXTREMES {
        for field in 0..4usize {
            let mut m = Md5::default();
            match field {
                0 => m.a = v,
                1 => m.b = v,
                2 => m.c = v,
                _ => m.d = v,
            }
            assert_same("G7/single", &m);
        }
        // And all fields simultaneously at the extreme.
        assert_same("G7/all", &Md5::new(v, v, v, v));
    }
    // Cross-product of the extremes over all four fields.
    for &a in &EXTREMES {
        for &b in &EXTREMES {
            for &c in &EXTREMES {
                for &d in &EXTREMES {
                    assert_same("G7/cross", &Md5::new(a, b, c, d));
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// G8 — aliasing (also Phase B row 17; asserted here as a rejection boundary).
// ---------------------------------------------------------------------------

/// G8 — the C declares no `restrict`, so a caller may legally alias `out` onto
/// `m`. Neither library may reject it; both must produce the same bytes.
#[test]
fn g8_aliased_out_over_m() {
    let cf = digest(Impl::C);
    let rf = digest(Impl::Rust);
    let mut rng = Rng::new(0x60_0008);
    for _ in 0..ITERS {
        let img = rng.next_image();
        let mut mc = Md5::from_image(&img);
        let mut mr = Md5::from_image(&img);
        unsafe {
            cf(&mc as *const Md5, (&mut mc as *mut Md5).cast::<u8>());
            rf(&mr as *const Md5, (&mut mr as *mut Md5).cast::<u8>());
        }
        let ic = unsafe { *(&mc as *const Md5).cast::<[u8; 16]>() };
        let ir = unsafe { *(&mr as *const Md5).cast::<[u8; 16]>() };
        assert_eq!(ic, ir, "[G8] aliased divergence for {img:02x?}");
    }

    // Partial overlap: out starts 1..15 bytes into the struct image.
    for shift in 1..16usize {
        for _ in 0..64 {
            let img = rng.next_image();
            let mut arena_c = [0u8; 48];
            let mut arena_r = [0u8; 48];
            arena_c[..16].copy_from_slice(&img);
            arena_r[..16].copy_from_slice(&img);
            unsafe {
                cf(arena_c.as_ptr().cast::<Md5>(), arena_c.as_mut_ptr().add(shift));
                rf(arena_r.as_ptr().cast::<Md5>(), arena_r.as_mut_ptr().add(shift));
            }
            assert_eq!(
                arena_c, arena_r,
                "[G8] partial overlap shift {shift} divergence for {img:02x?}\n  C   : {arena_c:02x?}\n  Rust: {arena_r:02x?}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// G9 / G10 — pointer alignment boundaries.
// ---------------------------------------------------------------------------

/// G9 — `out` is `tflac_u8*` (alignment 1); every offset is valid and neither
/// library may reject an odd address.
#[test]
fn g9_unaligned_out_offsets() {
    let cf = digest(Impl::C);
    let rf = digest(Impl::Rust);
    let mut rng = Rng::new(0x60_0009);
    for _ in 0..(ITERS / 2) {
        let m = rng.next_md5();
        for off in 0..16usize {
            let mut ac = [0xC3u8; 48];
            let mut ar = [0xC3u8; 48];
            unsafe {
                cf(&m as *const Md5, ac.as_mut_ptr().add(off));
                rf(&m as *const Md5, ar.as_mut_ptr().add(off));
            }
            assert_eq!(ac, ar, "[G9] out offset {off} divergence for {m:?}");
        }
    }
}

/// G10 — `m` reached through an under-aligned address.
#[test]
fn g10_unaligned_m_pointer() {
    let cf = digest(Impl::C);
    let rf = digest(Impl::Rust);
    let mut rng = Rng::new(0x60_0010);
    for _ in 0..(ITERS / 2) {
        let img = rng.next_image();
        for off in 1..8usize {
            let mut arena = [0u8; 32];
            arena[off..off + 16].copy_from_slice(&img);
            let mp = unsafe { arena.as_ptr().add(off) }.cast::<Md5>();
            let mut oc = [0xAAu8; 16];
            let mut or = [0xAAu8; 16];
            unsafe {
                cf(mp, oc.as_mut_ptr());
                rf(mp, or.as_mut_ptr());
            }
            assert_eq!(oc, or, "[G10] m offset {off} divergence for {img:02x?}");
            assert_eq!(oc, img, "[G10] C read the wrong bytes");
        }
    }
}
