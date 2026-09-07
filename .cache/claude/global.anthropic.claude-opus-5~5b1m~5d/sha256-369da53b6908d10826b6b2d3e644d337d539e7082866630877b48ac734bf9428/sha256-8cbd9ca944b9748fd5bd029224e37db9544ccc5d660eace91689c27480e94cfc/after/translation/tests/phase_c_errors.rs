//! Phase C — error-path differential tests.
//! One test per row of `ERRORS.md`.  The C performs no validation and always
//! returns 0, so each test asserts the *same* result value from both
//! implementations (never merely "both failed") plus full struct equality.

mod common;
use common::*;

const SEED: u64 = 0xE770_0000_1234_5678;

/// Helper: run the call through both libs and additionally pin down the exact
/// result value the C returns, so a Rust impl returning a different code
/// (e.g. -1) fails even if it somehow matched byte state.
#[track_caller]
fn check_rc_zero(state: Bitwriter, bits: u32, val: u64, ctx: &str) {
    let c = c_add();
    let r = rust_add();
    let mut cs = state;
    let mut rs = state;
    let rc_c = unsafe { c(&mut cs as *mut Bitwriter, bits, val) };
    let rc_r = unsafe { r(&mut rs as *mut Bitwriter, bits, val) };
    assert_eq!(rc_c, 0, "C sentinel changed [{ctx}]");
    assert_eq!(rc_r, rc_c, "rc differs [{ctx}] bits={bits} val={val:#x}");
    assert_eq!(cs.as_bytes(), rs.as_bytes(), "state differs [{ctx}] bits={bits} val={val:#x}");
}

/// Row 1 — no validation exists: the result is unconditionally 0 for a wide
/// randomized population of inputs.
#[test]
fn err01_result_always_zero() {
    let mut rng = Rng::new(SEED ^ 1);
    for _ in 0..20_000 {
        let st = rng.state();
        check_rc_zero(st, rng.interesting_bits(), rng.interesting_u64(), "err01");
    }
}

/// Row 2 — bits == 0 → `val <<= 64` (C UB; hardware masks the count to 0).
#[test]
fn err02_bits_zero_shift_by_width() {
    let mut rng = Rng::new(SEED ^ 2);
    for _ in 0..2000 {
        let st = rng.state();
        check_rc_zero(st, 0, rng.interesting_u64(), "err02");
    }
    // Explicit: from a clean writer, bits == 0 must leave `val`/`bits`/`tot`
    // exactly as the C leaves them (whatever that is) — compared, not assumed.
    check_rc_zero(Bitwriter::zeroed(), 0, u64::MAX, "err02-clean");
}

/// Row 3 — bits > 64 → `64 - bits` underflows u32.
#[test]
fn err03_bits_gt_width() {
    let mut rng = Rng::new(SEED ^ 3);
    for bits in [65u32, 66, 100, 1000, 0xFFFF, 0x7FFF_FFFF, 0x8000_0000, u32::MAX] {
        for _ in 0..500 {
            let st = rng.state();
            check_rc_zero(st, bits, rng.interesting_u64(), "err03");
        }
    }
}

/// Row 4 — bits == 64 exactly (boundary, loop always entered).
#[test]
fn err04_bits_eq_width() {
    let mut rng = Rng::new(SEED ^ 4);
    for _ in 0..2000 {
        let st = rng.state();
        check_rc_zero(st, 64, rng.interesting_u64(), "err04");
    }
}

/// Row 5 — bw->bits >= 64 on entry → `val >> bw->bits` shift count >= width.
#[test]
fn err05_state_bits_ge_width() {
    let mut rng = Rng::new(SEED ^ 5);
    for sb in [64u32, 65, 96, 128, 255, 1024, 0xFFFF_FFFF] {
        for _ in 0..500 {
            let mut st = rng.state();
            st.bits = sb;
            check_rc_zero(st, rng.interesting_bits(), rng.interesting_u64(), "err05");
        }
    }
}

/// Row 6 — bw->bits == 63 → b == 0, `bits` never decreases, loop stopped only
/// by the `i < 100` guard.
#[test]
fn err06_state_bits_63_iteration_cap() {
    let mut rng = Rng::new(SEED ^ 6);
    for _ in 0..2000 {
        let mut st = rng.state();
        st.bits = 63;
        // bits >= 1 guarantees the loop is entered (63 + bits >= 64)
        let bits = 1 + rng.below(63);
        check_rc_zero(st, bits, rng.interesting_u64(), "err06");
    }
}

/// Row 7 — bw->bits > 63 → `64 - bw->bits - 1` underflows, then clamps to bits.
#[test]
fn err07_state_bits_underflow_clamp() {
    let mut rng = Rng::new(SEED ^ 7);
    for sb in [64u32, 65, 70, 200, 0xFFFF_FFFE, 0xFFFF_FFFF] {
        for bits in [1u32, 2, 63, 64, 65, 0xFFFF] {
            for _ in 0..64 {
                let mut st = rng.state();
                st.bits = sb;
                check_rc_zero(st, bits, rng.interesting_u64(), "err07");
            }
        }
    }
}

/// Row 8 — `bw->bits + bits` overflows u32 back below 64 → loop skipped.
#[test]
fn err08_bits_sum_overflow() {
    let mut rng = Rng::new(SEED ^ 8);
    for (sb, bits) in [
        (u32::MAX, 1u32),
        (u32::MAX, 63),
        (u32::MAX, 64),
        (0xFFFF_FFC0, 0x41),
        (0x8000_0000, 0x8000_0000),
        (0x8000_0001, 0x7FFF_FFFF),
    ] {
        for _ in 0..200 {
            let mut st = rng.state();
            st.bits = sb;
            check_rc_zero(st, bits, rng.interesting_u64(), "err08");
        }
    }
}

/// Row 9 — bw->tot overflow (wrapping u32 add).
#[test]
fn err09_tot_overflow() {
    let mut rng = Rng::new(SEED ^ 9);
    for tot in [u32::MAX, 0xFFFF_FFFE, 0xFFFF_0000, 0x8000_0000] {
        for bits in [1u32, 2, 64, 65, 0xFFFF, u32::MAX] {
            for _ in 0..64 {
                let mut st = rng.state();
                st.tot = tot;
                check_rc_zero(st, bits, rng.interesting_u64(), "err09");
            }
        }
    }
}

/// Row 10 — null buffer / zero len / pos > len: never dereferenced, no error.
#[test]
fn err10_null_buffer_zero_len() {
    let mut rng = Rng::new(SEED ^ 10);
    for (pos, len) in [(0u32, 0u32), (1, 0), (u32::MAX, 0), (5, 4), (u32::MAX, u32::MAX)] {
        for _ in 0..400 {
            let st = Bitwriter {
                val: rng.interesting_u64(),
                bits: rng.interesting_state_bits(),
                pos,
                len,
                tot: rng.next_u32(),
                buffer: std::ptr::null_mut(),
            };
            check_rc_zero(st, rng.interesting_bits(), rng.interesting_u64(), "err10");
        }
    }
}

/// Row 11 — `bw == NULL` is identical UB (unconditional deref) in both impls.
/// Not executed: it would SIGSEGV and take the harness down.  Documented and
/// asserted structurally instead — the C emits no null check, so neither may
/// the Rust "helpfully" return an error code.
#[test]
fn err11_null_bw_documented_not_executed() {
    let c_src = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("c_src/src/lib.c"),
    )
    .expect("read c_src/src/lib.c");
    assert!(
        !c_src.contains("NULL") && !c_src.contains("assert"),
        "C gained a null/assert check — ERRORS.md row 11 must be revisited"
    );
    // And the only `return` in the C is `return 0`.
    let returns: Vec<&str> = c_src
        .lines()
        .map(str::trim)
        .filter(|l| l.starts_with("return"))
        .collect();
    assert_eq!(returns, vec!["return 0;"], "C error surface changed");
}

/// Row 12 — no enums exist in the header, so every u32 `bits` value is valid
/// input; exhaustively cover the boundary neighbourhood one step past each
/// documented-ish limit plus a dense sweep of 0..=130.
#[test]
fn err12_no_enum_exhaustive_bits_neighbourhood() {
    let mut rng = Rng::new(SEED ^ 12);
    for bits in 0u32..=130 {
        for sb in [0u32, 1, 62, 63, 64, 65, 127, 128] {
            let st = Bitwriter {
                val: rng.interesting_u64(),
                bits: sb,
                pos: 3,
                len: 4,
                tot: 5,
                buffer: std::ptr::null_mut(),
            };
            check_rc_zero(st, bits, rng.interesting_u64(), "err12");
        }
    }
    // one step past the u32 range boundaries
    for bits in [u32::MAX - 1, u32::MAX, 0x8000_0000, 0x7FFF_FFFF] {
        check_rc_zero(Bitwriter::zeroed(), bits, 0xDEAD_BEEF_CAFE_BABE, "err12-max");
    }
}
