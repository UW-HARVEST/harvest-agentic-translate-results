//! Phase C — error-path differential tests, GATED on `ERRORS.md`.
//!
//! The error-surface table in `ERRORS.md` has ZERO rows: `c_src/src/lib.c`
//! contains no `if`, `switch`, `assert`, error enum, sentinel return, null
//! check or range check, and `half2float` is total over all 65 536 `uint16_t`
//! inputs (both index expressions are provably in bounds).
//!
//! So instead of matching error codes, these tests verify that the *absence* of
//! any rejection path is faithfully reproduced: for every generic boundary a C
//! API can have, both implementations must succeed and agree bit-for-bit, and
//! neither may trap, abort or panic. Rows B1-B6 of `ERRORS.md`.

mod common;

use common::{Pair, Rng, assert_bits_eq};

// ---------------------------------------------------------------------------
// B1 / B2 — extremes of the declared domain
// ---------------------------------------------------------------------------

#[test]
fn b1_minimum_domain_value() {
    let pair = Pair::load();
    let c = pair.c_half2float();
    let r = pair.rust_half2float();
    let h = 0x0000u16;
    let (cv, rv) = unsafe { (c(h), r(h)) };
    assert_bits_eq(h, cv, rv, "B1: h = 0x0000 (minimum of domain)");
}

#[test]
fn b2_maximum_domain_value() {
    let pair = Pair::load();
    let c = pair.c_half2float();
    let r = pair.rust_half2float();
    let h = 0xFFFFu16;
    let (cv, rv) = unsafe { (c(h), r(h)) };
    assert_bits_eq(h, cv, rv, "B2: h = 0xFFFF (maximum of domain)");
}

// ---------------------------------------------------------------------------
// B3 — one step past every internal region edge
// ---------------------------------------------------------------------------

#[test]
fn b3_one_step_past_each_region_edge() {
    let pair = Pair::load();
    let c = pair.c_half2float();
    let r = pair.rust_half2float();

    // Each pair straddles a boundary in m__offset / m__exponent, i.e. the last
    // value of one region and the first value of the next.
    const EDGES: &[u16] = &[
        0x03FF, 0x0400, // n=0 -> n=1   (m__offset 0x0000 -> 0x0400)
        0x7BFF, 0x7C00, // n=30 -> n=31 (finite -> Inf/NaN exponent 0x47800000)
        0x7C01, // first NaN payload past +Inf
        0x7FFF, 0x8000, // n=31 -> n=32 (m__offset 0x0400 -> 0x0000, sign flips)
        0x83FF, 0x8400, // n=32 -> n=33 (m__offset 0x0000 -> 0x0400)
        0xFBFF, 0xFC00, // n=62 -> n=63 (finite -> Inf/NaN exponent 0xC7800000)
        0xFC01, // first NaN payload past -Inf
    ];

    for &h in EDGES {
        let (cv, rv) = unsafe { (c(h), r(h)) };
        assert_bits_eq(h, cv, rv, "B3: region edge");
    }

    // Also step one past every single one of the 64 exponent-field boundaries,
    // mechanically rather than by hand-picked list.
    for n in 0u32..64 {
        for m in [0x000u32, 0x3FF] {
            let h = ((n << 10) | m) as u16;
            let (cv, rv) = unsafe { (c(h), r(h)) };
            assert_bits_eq(h, cv, rv, "B3: exponent-field boundary");
        }
    }
}

// ---------------------------------------------------------------------------
// B4 — exhaustive sweep: no input can hide an unmatched rejection
// ---------------------------------------------------------------------------

#[test]
fn b4_exhaustive_no_input_is_rejected_differently() {
    let pair = Pair::load();
    let c = pair.c_half2float();
    let r = pair.rust_half2float();

    let mut diverged = 0usize;
    let mut first: Option<(u16, u32, u32)> = None;

    for h in 0u16..=u16::MAX {
        let (cb, rb) = unsafe { (c(h).to_bits(), r(h).to_bits()) };
        if cb != rb {
            diverged += 1;
            first.get_or_insert((h, cb, rb));
        }
        if h == u16::MAX {
            break;
        }
    }

    assert_eq!(
        diverged, 0,
        "exhaustive sweep diverged on {diverged} inputs; first = {first:?}"
    );
}

// ---------------------------------------------------------------------------
// B5 — out-of-range value across the FFI boundary (the invalid-enum analogue)
// ---------------------------------------------------------------------------
//
// `half2float` takes a `uint16_t`. A caller that mis-declares the prototype as
// taking a wider integer can leave garbage in the high bits of the argument
// register. The x86-64 SysV ABI leaves those bits unspecified, so this is a
// caller-side contract violation -- but it is still a real input the C handles
// somehow, and the Rust must handle it identically. These tests pin down that
// both implementations narrow to 16 bits the same way, so no divergence is
// reachable even from a mis-declared caller.

#[test]
fn b5_dirty_high_bits_u32_argument() {
    let pair = Pair::load();
    let c32 = pair.c_half2float_u32();
    let r32 = pair.rust_half2float_u32();
    let c16 = pair.c_half2float();

    // Explicit values just past the top of the uint16_t range, plus patterns
    // with every high bit set.
    let explicit: &[u32] = &[
        0x0001_0000,
        0x0001_0001,
        0x0000_FFFF,
        0xFFFF_0000,
        0xFFFF_FFFF,
        0xDEAD_0000,
        0xDEAD_BEEF,
        0x8000_0000,
        0x7FFF_FFFF,
    ];
    for &wide in explicit {
        let (cv, rv) = unsafe { (c32(wide), r32(wide)) };
        assert_eq!(
            cv.to_bits(),
            rv.to_bits(),
            "B5: C and Rust diverged for mis-declared u32 argument {wide:#010x}:\n  \
             C = {:#010x}  Rust = {:#010x}",
            cv.to_bits(),
            rv.to_bits(),
        );
    }

    // Randomized: for many h, set arbitrary garbage in the high 16 bits and
    // check the result still equals the correctly-declared 16-bit call.
    let mut rng = Rng::new(0x0000_00B5);
    for _ in 0..8192 {
        let h = rng.next_u16();
        let garbage = rng.next_u32() & 0xFFFF_0000;
        let wide = garbage | h as u32;

        let (cv, rv) = unsafe { (c32(wide), r32(wide)) };
        assert_eq!(
            cv.to_bits(),
            rv.to_bits(),
            "B5: divergence for wide arg {wide:#010x} (low16 = {h:#06x})"
        );

        // Both should also match the clean 16-bit call, i.e. the high bits are
        // ignored consistently by both.
        let clean = unsafe { c16(h) };
        assert_eq!(
            cv.to_bits(),
            clean.to_bits(),
            "B5: C changed behaviour when high bits were dirty ({wide:#010x})"
        );
        assert_eq!(
            rv.to_bits(),
            clean.to_bits(),
            "B5: Rust changed behaviour when high bits were dirty ({wide:#010x})"
        );
    }
}

#[test]
fn b5_dirty_high_bits_u64_argument() {
    let pair = Pair::load();
    let c64 = pair.c_half2float_u64();
    let r64 = pair.rust_half2float_u64();
    let c16 = pair.c_half2float();

    let explicit: &[u64] = &[
        0x0000_0000_0001_0000,
        0xFFFF_FFFF_FFFF_0000,
        0xFFFF_FFFF_FFFF_FFFF,
        0xDEAD_BEEF_CAFE_0000,
        0x8000_0000_0000_0000,
    ];
    for &wide in explicit {
        let (cv, rv) = unsafe { (c64(wide), r64(wide)) };
        assert_eq!(
            cv.to_bits(),
            rv.to_bits(),
            "B5(u64): divergence for {wide:#018x}"
        );
    }

    let mut rng = Rng::new(0x0000_0B54);
    for _ in 0..8192 {
        let h = rng.next_u16();
        let garbage = rng.next_u64() & 0xFFFF_FFFF_FFFF_0000;
        let wide = garbage | h as u64;

        let (cv, rv) = unsafe { (c64(wide), r64(wide)) };
        assert_eq!(
            cv.to_bits(),
            rv.to_bits(),
            "B5(u64): divergence for {wide:#018x} (low16 = {h:#06x})"
        );
        let clean = unsafe { c16(h) };
        assert_eq!(
            cv.to_bits(),
            clean.to_bits(),
            "B5(u64): C behaviour changed with dirty high bits"
        );
        assert_eq!(
            rv.to_bits(),
            clean.to_bits(),
            "B5(u64): Rust behaviour changed with dirty high bits"
        );
    }
}

// ---------------------------------------------------------------------------
// B6 — no pointer/length surface exists
// ---------------------------------------------------------------------------

#[test]
fn b6_signature_has_no_pointer_or_length_surface() {
    // `float half2float(uint16_t h)` -- one scalar in, one scalar out. There is
    // no pointer to pass as NULL and no length to pass as 0 or oversized, so
    // there is nothing to fuzz here. This test asserts the fact the reasoning
    // rests on: the C .so exports exactly one symbol and it is `half2float`.
    let pair = Pair::load();

    let out = std::process::Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(&pair.c_path)
        .output();

    let Ok(out) = out else {
        eprintln!("B6: `nm` unavailable, skipping symbol assertion");
        return;
    };
    if !out.status.success() {
        eprintln!("B6: `nm` failed, skipping symbol assertion");
        return;
    }

    let text = String::from_utf8_lossy(&out.stdout);
    let symbols: Vec<&str> = text
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2))
        .collect();

    assert_eq!(
        symbols,
        vec!["half2float"],
        "C .so must export exactly one symbol, `half2float`; got {symbols:?}"
    );
}
