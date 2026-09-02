//! Phase B (valid-path) and Phase C (error-path) differential tests.
//!
//! Every test loads BOTH `libdriver.so` builds through `libloading` and compares
//! the stdout bytes they emit. Row ids match `CONFIGS.md` / `ERRORS.md`.

mod common;

use common::{assert_same, assert_same_dirty, assert_same_sequence, pair, Rng, Sink};

/// Fixed seed for every property-style sweep, so failures are reproducible.
const SEED: u64 = 0x5DEE_CE66_D0C0_FFEE;

// ===========================================================================
// Phase A cross-check: the export the tests depend on really is exported by
// both objects (the Rust one via its `#[no_mangle]` wrapper).
// ===========================================================================

#[test]
fn symbols_driver_exported_by_both() {
    let p = pair();
    assert!(p.c.has_symbol(b"driver"), "C .so must export `driver`");
    assert!(
        p.rust.has_symbol(b"driver"),
        "Rust .so must export `driver` (check #[no_mangle] extern \"C\")"
    );
}

// ===========================================================================
// Phase B — CONFIGS.md rows
// ===========================================================================

/// C1 — exactly one iteration.
#[test]
fn cfg_c1_single() {
    let out = assert_same(1, Sink::RegularFile, "C1");
    assert_eq!(out, b"0 0\n", "C1 sanity: single line");
}

/// C2 — smallest "many"; `j` first becomes non-zero.
#[test]
fn cfg_c2_two() {
    let out = assert_same(2, Sink::RegularFile, "C2");
    assert_eq!(out, b"0 0\n1 2\n", "C2 sanity");
}

/// C3 — every small count exhaustively; `j` crosses 1→2 digits at `i == 5`.
#[test]
fn cfg_c3_small_exhaustive() {
    for x in 1..=16 {
        assert_same(x, Sink::RegularFile, "C3");
    }
}

/// C4 — randomized `x` in `1..=1000`.
#[test]
fn cfg_c4_random_small() {
    let mut rng = Rng::new(SEED);
    for _ in 0..200 {
        let x = rng.range_i32(1, 1000);
        assert_same(x, Sink::RegularFile, "C4");
    }
}

/// C5 — randomized `x` in `1..=100_000`; wider decimal fields.
#[test]
fn cfg_c5_random_medium() {
    let mut rng = Rng::new(SEED ^ 0xA5A5);
    for _ in 0..25 {
        let x = rng.range_i32(1, 100_000);
        assert_same(x, Sink::RegularFile, "C5");
    }
}

/// C6 — decimal-width boundaries of `i`.
#[test]
fn cfg_c6_i_width_boundaries() {
    const XS: &[i32] = &[
        1, 9, 10, 11, 99, 100, 101, 999, 1000, 1001, 9999, 10_000, 10_001, 99_999, 100_000,
        100_001,
    ];
    for &x in XS {
        assert_same(x, Sink::RegularFile, "C6");
    }
}

/// C7 — decimal-width boundaries of `j == 2*i`, which fall at different `i`
/// than `i`'s own boundaries.
#[test]
fn cfg_c7_j_width_boundaries() {
    const XS: &[i32] = &[5, 6, 7, 50, 51, 52, 500, 501, 502, 5000, 5001, 50_000, 50_001];
    for &x in XS {
        assert_same(x, Sink::RegularFile, "C7");
    }
}

/// C8 — one large call: 1e6 iterations, many stdout buffer refills.
#[test]
fn cfg_c8_large_one_million() {
    let out = assert_same(1_000_000, Sink::RegularFile, "C8");
    assert_eq!(
        out.iter().filter(|&&b| b == b'\n').count(),
        1_000_000,
        "C8 sanity: one line per iteration"
    );
}

/// C9 — many back-to-back calls, valid counts interleaved with rejecting ones.
/// Catches any retained state and checks the streams concatenate identically.
#[test]
fn cfg_c9_interleaved_sequence() {
    let mut rng = Rng::new(SEED ^ 0x1234_5678);
    for _ in 0..40 {
        let mut xs = Vec::new();
        for _ in 0..8 {
            // Mix valid counts with values the loop guard rejects.
            xs.push(match rng.next_u64() % 4 {
                0 => rng.range_i32(-1000, 0),
                1 => 0,
                _ => rng.range_i32(1, 300),
            });
        }
        assert_same_sequence(&xs, Sink::RegularFile, "C9");
    }
}

/// C12 — the same configurations through a pipe instead of a regular file, so
/// libc's default buffering mode differs.
#[test]
fn cfg_c12_pipe_and_file_buffering() {
    const XS: &[i32] = &[0, -1, 1, 2, 5, 10, 99, 100, 1000, 5000];
    for &x in XS {
        let file_out = assert_same(x, Sink::RegularFile, "C12/file");
        let pipe_out = assert_same(x, Sink::Pipe, "C12/pipe");
        // Buffering must not change the byte stream itself.
        common::compare(
            &file_out,
            &pipe_out,
            &format!("C12: driver({x}) differs between file and pipe sinks"),
        );
    }

    let mut rng = Rng::new(SEED ^ 0xBEEF);
    for _ in 0..40 {
        let x = rng.range_i32(-50, 2000);
        assert_same(x, Sink::Pipe, "C12/pipe-random");
    }

    // A payload larger than the 64 KiB pipe capacity, so the writer blocks and
    // resumes mid-stream.
    assert_same(40_000, Sink::Pipe, "C12/pipe-large");
}

// ===========================================================================
// Phase C — ERRORS.md rows
//
// `driver` returns `void` and has no error channel, so "same rejection" means
// both implementations return normally and emit the identical (empty) stream.
// ===========================================================================

/// E1 — `x == 0`: loop guard false on first evaluation.
#[test]
fn err_e1_zero() {
    let out = assert_same(0, Sink::RegularFile, "E1");
    assert!(out.is_empty(), "E1: x==0 must emit nothing, got {out:?}");
}

/// E2 — `x == -1`: one step past the bottom of the productive range.
#[test]
fn err_e2_minus_one() {
    let out = assert_same(-1, Sink::RegularFile, "E2");
    assert!(out.is_empty(), "E2: x==-1 must emit nothing");
}

/// E3 — randomized negative values.
#[test]
fn err_e3_negative_sweep() {
    let mut rng = Rng::new(SEED ^ 0xDEAD);
    for _ in 0..300 {
        let x = rng.range_i32(i32::MIN, -1);
        let out = assert_same(x, Sink::RegularFile, "E3");
        assert!(out.is_empty(), "E3: driver({x}) must emit nothing");
    }
}

/// E4 — `x == INT_MIN`.
#[test]
fn err_e4_int_min() {
    let out = assert_same(i32::MIN, Sink::RegularFile, "E4");
    assert!(out.is_empty(), "E4: INT_MIN must emit nothing");
    // Also through a pipe, to rule out sink-dependent behaviour.
    let out = assert_same(i32::MIN, Sink::Pipe, "E4/pipe");
    assert!(out.is_empty(), "E4: INT_MIN must emit nothing via pipe");
}

/// E5 — `x == INT_MIN + 1`, one step past the extreme boundary.
#[test]
fn err_e5_int_min_plus_one() {
    let out = assert_same(i32::MIN + 1, Sink::RegularFile, "E5");
    assert!(out.is_empty(), "E5: INT_MIN+1 must emit nothing");
}

/// E6 — arbitrary `int` bit patterns with no "valid variant". The C prototype
/// is `int`, so any 32-bit pattern is a legal argument and the C validates
/// none of them; this is the enum-style out-of-range case for this API.
#[test]
fn err_e6_out_of_range_int_patterns() {
    const PATTERNS: &[u32] = &[
        0x8000_0000, // INT_MIN
        0x8000_0001,
        0xC000_0000,
        0xDEAD_BEEF,
        0xFFFF_FFFF, // -1
        0xFFFF_FFFE,
        0x7FFF_FFFF, // INT_MAX — valid but extreme; see the prefix test
        0x0000_FFFF,
        0xCAFE_0000,
        0xA5A5_A5A5,
    ];
    for &bits in PATTERNS {
        let x = bits as i32;
        if x == i32::MAX {
            // Running INT_MAX to completion is infeasible; covered separately
            // by the prefix comparison in `tests/int_max_prefix.rs`.
            continue;
        }
        if x > 2_000_000 {
            continue; // keep the test bounded
        }
        assert_same(x, Sink::RegularFile, "E6");
    }
}

/// E7 — argument register carries garbage in its upper 32 bits; both callees
/// must truncate to `int` identically.
#[test]
fn err_e7_dirty_upper_half() {
    const PACKED: &[i64] = &[
        0x1234_5678_0000_0000u64 as i64, // low half 0
        0x1234_5678_0000_0003u64 as i64, // low half 3
        0xFFFF_FFFF_0000_0005u64 as i64, // low half 5
        0x0000_0001_FFFF_FFFFu64 as i64, // low half -1
        0x7FFF_FFFF_8000_0000u64 as i64, // low half INT_MIN
        0xDEAD_BEEF_0000_000Au64 as i64, // low half 10
    ];
    for &p in PACKED {
        assert_same_dirty(p, "E7");
    }
}

/// E8 — a rejecting call must leave no sticky state behind.
#[test]
fn err_e8_no_sticky_state() {
    assert_same_sequence(&[0, 3], Sink::RegularFile, "E8");
    assert_same_sequence(&[-1, 3], Sink::RegularFile, "E8");
    assert_same_sequence(&[i32::MIN, 3], Sink::RegularFile, "E8");
    assert_same_sequence(&[0, 0, 0, 5, 0, 5], Sink::RegularFile, "E8");
    assert_same_sequence(&[-7, 4, -7, 4], Sink::Pipe, "E8/pipe");

    // And the post-rejection call must still produce the full expected output.
    let p = pair();
    let _g = common::fd_lock().lock().unwrap();
    let out = common::capture(Sink::RegularFile, || {
        p.rust.driver(-100);
        p.rust.driver(3);
    });
    drop(_g);
    assert_eq!(out, b"0 0\n1 2\n2 4\n", "E8: state leaked across calls");
}
