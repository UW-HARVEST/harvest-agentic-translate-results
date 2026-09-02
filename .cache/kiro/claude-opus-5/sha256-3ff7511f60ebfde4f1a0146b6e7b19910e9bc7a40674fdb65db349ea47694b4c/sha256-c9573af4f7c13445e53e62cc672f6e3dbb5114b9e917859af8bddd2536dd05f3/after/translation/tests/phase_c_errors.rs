//! Phase C — error-path / rejection differential tests.
//!
//! One test per row of `ERRORS.md`, plus the generic C-API boundaries.
//!
//! ## Why the UB rows are tested out-of-process
//!
//! `pow43` has NO error-return surface (no sentinel, no `errno`, no error enum,
//! no pointer or length parameter — see `ERRORS.md` for the mechanical grep).
//! Its entire "rejection" surface is the two out-of-bounds table-read regions,
//! and C leaves those **undefined**. Empirically (see `ERRORS.md`) an OOB read
//! either returns whatever bytes happen to neighbour `g_pow43` in that
//! particular shared object — which necessarily differ between the C `.so` and
//! the Rust `.so`, so there is no ground truth to match — or faults with
//! SIGSEGV/SIGBUS once the computed offset leaves the mapping.
//!
//! The one property that IS well defined and IS a real correctness requirement
//! is therefore asserted here: **the Rust must never reject an input that the C
//! accepts.** A bounds-checked Rust index would panic (exit 101 / SIGABRT)
//! where the C merely reads adjacent memory; that is a genuine, observable
//! behavioural divergence and these tests catch it. Probes run in a child
//! process (`examples/probe.rs`) so that a faulting UB read does not take the
//! test binary down with it.

mod harness;
use harness::*;

use std::path::PathBuf;
use std::process::Command;

// ---------------------------------------------------------------------------
// out-of-process probe plumbing
// ---------------------------------------------------------------------------

fn probe_bin() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|d| d.parent())
        .expect("target/<profile>");
    let p = profile_dir.join("examples/probe");
    assert!(
        p.is_file(),
        "probe helper not built at {}. Run `cargo build --example probe` \
         (cargo test builds examples automatically).",
        p.display()
    );
    p
}

#[derive(Debug, PartialEq, Eq)]
enum Outcome {
    /// Returned normally with these result bits.
    Value(u32),
    /// Died by signal `n` (SIGSEGV = 11, SIGBUS = 7) — a UB memory read that
    /// left the mapping. This is what the C does too.
    Signal(i32),
    /// The Rust panicked / aborted — i.e. it REJECTED the input. Always a
    /// divergence, because the C never rejects.
    Rejected(String),
}

fn probe(so: &std::path::Path, x: i32) -> Outcome {
    let out = Command::new(probe_bin())
        .arg(so)
        .arg(x.to_string())
        .output()
        .expect("spawn probe");
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();

    if let Some(code) = out.status.code() {
        match code {
            0 => {
                let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
                Outcome::Value(u32::from_str_radix(&s, 16).unwrap_or_else(|e| {
                    panic!("probe printed {s:?}, not hex bits: {e}")
                }))
            }
            // 101 = Rust panic; 134 = SIGABRT surfaced as a code on some setups.
            101 | 134 => Outcome::Rejected(stderr),
            other => {
                if stderr.contains("panicked") {
                    Outcome::Rejected(stderr)
                } else {
                    Outcome::Signal(other)
                }
            }
        }
    } else {
        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;
            let sig = out.status.signal().unwrap_or(-1);
            if sig == libc_sigabrt() || stderr.contains("panicked") {
                Outcome::Rejected(stderr)
            } else {
                Outcome::Signal(sig)
            }
        }
        #[cfg(not(unix))]
        {
            Outcome::Signal(-1)
        }
    }
}

fn libc_sigabrt() -> i32 {
    6
}

/// The core UB assertion: for an out-of-bounds input the C never rejects, so
/// the Rust must not reject either. It may return garbage or fault, exactly as
/// the C does.
#[track_caller]
fn assert_rust_does_not_reject(p: &Pair, x: i32, row: &str) {
    let c = probe(&p.c_path, x);
    let r = probe(&p.rs_path, x);
    if let Outcome::Rejected(msg) = &r {
        panic!(
            "[{row}] x = {x}: the Rust REJECTED an input the C accepts \
             (C outcome: {c:?}). This is a real divergence — the C performs an \
             unchecked read here.\nRust stderr:\n{msg}"
        );
    }
    // Also assert the C itself never "rejects" — documents the premise.
    assert!(
        !matches!(c, Outcome::Rejected(_)),
        "[{row}] x = {x}: premise violated, the C aborted: {c:?}"
    );
}

// ---------------------------------------------------------------------------
// ERRORS.md rows
// ---------------------------------------------------------------------------

/// ERRORS.md row 1 — `x == -16` is the lowest *in-bounds* argument.
/// Defined behaviour: must match bit-for-bit, and must be `+0.0f`.
#[test]
fn errors_row01_lowest_in_bounds() {
    let p = Pair::load();
    assert_eq!(c_table_index(-16), 0);
    assert!(in_bounds(-16));
    p.assert_same(-16, "errors_row01");
    assert_eq!(
        p.c(-16).to_bits(),
        0x0000_0000,
        "errors_row01: C returned {:?}, expected +0.0f",
        p.c(-16)
    );
    assert_eq!(p.rs(-16).to_bits(), 0x0000_0000);
}

/// ERRORS.md row 2 — `x < -16`: negative index, OOB read before the table.
/// The C returns garbage or faults; it never rejects. Neither may the Rust.
#[test]
fn errors_row02_negative_index_oob_is_not_rejected() {
    let p = Pair::load();
    let mut r = Rng::new(SEED ^ 0xE02);
    let mut probes: Vec<i32> = vec![-17, -18, -19, -20, -24, -32, -48, -64, -128, -256, -1000];
    for _ in 0..40 {
        probes.push(r.range(i32::MIN + 17, -17));
    }
    for x in probes {
        assert!(!in_bounds(x), "probe {x} should be out of bounds");
        assert_rust_does_not_reject(&p, x, "errors_row02");
    }
}

/// ERRORS.md row 2 (continued) — `16 + x` itself overflows for
/// `x < INT_MIN + 16`. Signed-overflow UB; must not be rejected either side.
#[test]
fn errors_row02b_index_addition_overflow_is_not_rejected() {
    let p = Pair::load();
    for x in [i32::MIN, i32::MIN + 1, i32::MIN + 15, i32::MIN + 16] {
        assert_rust_does_not_reject(&p, x, "errors_row02b");
    }
}

/// ERRORS.md row 3 — `x == 8223` is the highest *in-bounds* argument
/// (table index 144, the very last element). Defined: must match exactly.
#[test]
fn errors_row03_highest_in_bounds() {
    let p = Pair::load();
    assert_eq!(c_table_index(8223), 144);
    assert!(in_bounds(8223));
    p.assert_same(8223, "errors_row03");
}

/// ERRORS.md row 4 — `x > 8223`: computed index exceeds 144, OOB read past the
/// end of the table. Same reasoning as row 2.
#[test]
fn errors_row04_high_index_oob_is_not_rejected() {
    let p = Pair::load();
    let mut r = Rng::new(SEED ^ 0xE04);
    let mut probes: Vec<i32> = vec![8224, 8225, 8255, 8256, 8287, 8288, 9000, 16_384, 65_536];
    for _ in 0..40 {
        probes.push(r.range(8224, i32::MAX));
    }
    for x in probes {
        assert!(!in_bounds(x), "probe {x} should be out of bounds");
        assert_rust_does_not_reject(&p, x, "errors_row04");
    }
}

/// ERRORS.md row 4 (boundary) — the exact first out-of-bounds argument is
/// 8224, not 8256: bit 5 of 8224 is set so `sign == 64` pushes the index to
/// 145. Documents the derivation of DOMAIN_HI.
#[test]
fn errors_row04b_first_oob_is_8224() {
    let p = Pair::load();
    assert_eq!(c_table_index(8223), 144, "8223 must still be in bounds");
    assert_eq!(c_table_index(8224), 145, "8224 must be the first OOB input");
    assert_eq!(8224i32.wrapping_mul(2) & 64, 64, "sign must be 64 at 8224");
    assert_eq!(DOMAIN_HI, 8223);
    p.assert_same(8223, "errors_row04b");
    assert_rust_does_not_reject(&p, 8224, "errors_row04b");
}

/// ERRORS.md row 5 — the `x <<= 3` shift can never overflow, because it is
/// guarded by `129 <= x < 1024`. Verified by construction over the whole guard
/// range, so no divergence is possible on that path.
#[test]
fn errors_row05_shift_overflow_unreachable() {
    for x in 129..1024i32 {
        let shifted = (x as i64) << 3;
        assert!(
            shifted <= i32::MAX as i64 && shifted >= i32::MIN as i64,
            "x = {x}: `x <<= 3` would overflow"
        );
        assert!(shifted <= 8184, "x = {x}: post-shift value {shifted} > 8184");
    }
}

/// ERRORS.md row 6 — division by zero at `(x & ~63) + sign` is unreachable.
///
/// Checked over the entire reachable `x >= 129` range that this expression is
/// never 0, and that no defined-domain input produces `inf`/`NaN` from either
/// library.
#[test]
fn errors_row06_denominator_never_zero() {
    let p = Pair::load();

    for x0 in 129..=DOMAIN_HI {
        let x = if x0 < 1024 { x0 << 3 } else { x0 };
        let sign = x.wrapping_mul(2) & 64;
        let den = (x & !63i32).wrapping_add(sign);
        assert_ne!(den, 0, "x = {x0}: denominator is zero");
    }

    // Spot-check the whole (mostly UB) upper range for a zero denominator.
    let mut r = Rng::new(SEED ^ 0xE06);
    for _ in 0..200_000 {
        let x = r.range(1024, i32::MAX);
        let sign = x.wrapping_mul(2) & 64;
        let den = (x & !63i32).wrapping_add(sign);
        assert_ne!(den, 0, "x = {x}: denominator is zero");
    }

    // No defined-domain input may yield a non-finite result from either side.
    for x in DOMAIN_LO..=DOMAIN_HI {
        let cv = p.c(x);
        let rv = p.rs(x);
        assert!(cv.is_finite(), "C produced non-finite {cv:?} at x = {x}");
        assert!(rv.is_finite(), "Rust produced non-finite {rv:?} at x = {x}");
        assert_eq!(cv.to_bits(), rv.to_bits(), "divergence at x = {x}");
    }
}

/// ERRORS.md row 7 — signed-overflow UB in `2 * x`, `(x & ~63) + sign` and
/// `x + sign` for `x` near `INT_MAX`. gcc at `-O0` wraps two's-complement; the
/// Rust uses `wrapping_*`. Reachable only together with row 4, so the assertion
/// is (a) the wrapping premise holds and (b) the Rust does not reject.
#[test]
fn errors_row07_signed_overflow_wraps_without_rejecting() {
    let p = Pair::load();
    for x in [
        i32::MAX / 2,
        i32::MAX / 2 + 1,
        i32::MAX - 64,
        i32::MAX - 1,
        i32::MAX,
        0x4000_0000,
        0x7FFF_FFC0,
    ] {
        if x > i32::MAX / 2 {
            assert!(x.wrapping_mul(2) < 0, "premise: 2*{x} should wrap negative");
        }
        assert_rust_does_not_reject(&p, x, "errors_row07");
    }
}

/// ERRORS.md row 8 — `x == 128` / `x == 129`, the `x < 129` selector.
#[test]
fn errors_row08_selector_129() {
    let p = Pair::load();
    p.assert_same(128, "errors_row08");
    p.assert_same(129, "errors_row08");
}

/// ERRORS.md row 9 — `x == 1023` / `x == 1024`, the `x < 1024` selector
/// (`mult` flips between 16 and 256).
#[test]
fn errors_row09_selector_1024() {
    let p = Pair::load();
    p.assert_same(1023, "errors_row09");
    p.assert_same(1024, "errors_row09");
}

/// ERRORS.md row 10 — `x == 0`: the scalar analogue of a zero length; must
/// return `+0.0f` with the same sign bit from both libraries.
#[test]
fn errors_row10_zero_input() {
    let p = Pair::load();
    p.assert_same(0, "errors_row10");
    assert_eq!(p.c(0).to_bits(), p.rs(0).to_bits());
}

/// ERRORS.md row 11 — `INT_MIN` and `INT_MAX`, one step past every range.
#[test]
fn errors_row11_extreme_ints() {
    let p = Pair::load();
    for x in [i32::MIN, i32::MAX] {
        assert_rust_does_not_reject(&p, x, "errors_row11");
    }
}

/// ERRORS.md row 12 — "out-of-range enum across FFI".
///
/// `pow43` declares no enum, flag or mode parameter, so the closest analogue is
/// that every one of the 2^32 `int` bit patterns is a legal argument. The
/// defined sub-range is covered exhaustively by `errors_row12a`; this test
/// samples the *undefined* remainder and asserts the Rust never rejects.
#[test]
fn errors_row12a_defined_domain_exhaustive() {
    let p = Pair::load();
    let n = p.assert_same_all(DOMAIN_LO..=DOMAIN_HI, "errors_row12a");
    assert_eq!(n, 8240);
}

#[test]
fn errors_row12b_undefined_remainder_never_rejected() {
    let p = Pair::load();
    let mut r = Rng::new(SEED ^ 0xE12);
    let mut checked = 0usize;
    // Draw from the whole int space; keep only the UB values.
    while checked < 60 {
        let x = r.next_i32();
        if in_bounds(x) {
            p.assert_same(x, "errors_row12b");
            continue;
        }
        assert_rust_does_not_reject(&p, x, "errors_row12b");
        checked += 1;
    }
}

/// Generic boundary: every input one step past each derived range edge.
#[test]
fn errors_generic_one_step_past_every_edge() {
    let p = Pair::load();
    for x in [
        -1, 0, 1, 127, 128, 129, 130, 1022, 1023, 1024, 1025, DOMAIN_LO, DOMAIN_LO + 1,
        DOMAIN_HI - 1, DOMAIN_HI,
    ] {
        assert!(in_bounds(x));
        p.assert_same(x, "errors_generic");
    }
    for x in [DOMAIN_LO - 1, DOMAIN_HI + 1] {
        assert!(!in_bounds(x));
        assert_rust_does_not_reject(&p, x, "errors_generic");
    }
}

/// Repeated invocation with the same argument must be idempotent on both sides
/// (no lazily-initialised state, no accumulator).
#[test]
fn errors_generic_idempotent() {
    let p = Pair::load();
    for x in [-16, 0, 128, 129, 1023, 1024, 4096, 8223] {
        let c0 = p.c(x).to_bits();
        let r0 = p.rs(x).to_bits();
        for _ in 0..1000 {
            assert_eq!(p.c(x).to_bits(), c0, "C not idempotent at {x}");
            assert_eq!(p.rs(x).to_bits(), r0, "Rust not idempotent at {x}");
        }
        assert_eq!(c0, r0, "divergence at {x}");
    }
}
