//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Both implementations are driven exclusively through the `driver` symbol
//! exported by their respective `.so`, loaded with `libloading`.

mod common;

use common::{assert_same, expected, Impl, Libs, Rng, SEED};

/// How many randomized inputs each randomized row draws.
const N: usize = 64;

fn diff(libs: &Libs, label: &str, x: i32) {
    let c = libs.run(Impl::C, x);
    let r = libs.run(Impl::Rust, x);
    assert_same(label, x, &c, &r);
    // Independent model of the C loop, so a harness that captured nothing for
    // both sides cannot silently "pass".
    assert_eq!(
        c,
        expected(x),
        "[{label}] C output does not match the independent model for x={x}"
    );
}

/// Byte length of the output for a given `x`, computed independently.
fn out_len(x: i32) -> usize {
    let mut n = 0usize;
    let mut i: i32 = 0;
    let mut j: i32 = 0;
    while i < x {
        n += format!("{i} {j}\n").len();
        i += 1;
        j += 2;
    }
    n
}

// ---------------------------------------------------------------------------
// C1 — exhaustive small range across the sign transition
// ---------------------------------------------------------------------------
#[test]
fn c1_exhaustive_small_range() {
    let libs = Libs::load();
    for x in -8..=64 {
        diff(&libs, "C1", x);
    }
    // Sanity: the boundary really is at 0/1.
    assert!(libs.run(Impl::C, 0).is_empty());
    assert_eq!(libs.run(Impl::C, 1), b"0 0\n");
}

// ---------------------------------------------------------------------------
// C2..C8 — randomized magnitude bands (decimal-width axis)
// ---------------------------------------------------------------------------
fn band(libs: &Libs, label: &str, lo: i32, hi: i32, n: usize, seed_salt: u64) {
    let mut rng = Rng::new(SEED ^ seed_salt);
    // Always include both endpoints of the band, then randomize.
    diff(libs, label, lo);
    diff(libs, label, hi);
    for _ in 0..n {
        let x = rng.range_i32(lo, hi);
        diff(libs, label, x);
    }
}

#[test]
fn c2_single_digit() {
    band(&Libs::load(), "C2", 1, 9, N, 2);
}

#[test]
fn c3_two_digits() {
    band(&Libs::load(), "C3", 10, 99, N, 3);
}

#[test]
fn c4_three_digits() {
    band(&Libs::load(), "C4", 100, 999, N, 4);
}

#[test]
fn c5_four_digits() {
    band(&Libs::load(), "C5", 1_000, 9_999, N, 5);
}

#[test]
fn c6_five_digits() {
    band(&Libs::load(), "C6", 10_000, 99_999, 24, 6);
}

#[test]
fn c7_six_digits() {
    band(&Libs::load(), "C7", 100_000, 999_999, 6, 7);
}

#[test]
fn c8_seven_digits_multi_megabyte() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 8);
    for _ in 0..3 {
        let x = rng.range_i32(1_000_000, 4_000_000);
        let c = libs.run(Impl::C, x);
        let r = libs.run(Impl::Rust, x);
        assert_same("C8", x, &c, &r);
        assert_eq!(c, expected(x), "[C8] C output diverges from model, x={x}");
        assert!(c.len() > 14_000_000, "[C8] expected a multi-MiB capture");
    }
}

// ---------------------------------------------------------------------------
// C10 — powers of ten and their neighbours (decimal width of `i`)
// ---------------------------------------------------------------------------
#[test]
fn c10_power_of_ten_boundaries_for_i() {
    let libs = Libs::load();
    let mut xs = Vec::new();
    let mut p: i64 = 1;
    while p <= 1_000_000 {
        for d in [-1i64, 0, 1] {
            let v = p + d;
            if v >= 0 {
                xs.push(v as i32);
            }
        }
        p *= 10;
    }
    xs.sort_unstable();
    xs.dedup();
    for x in xs {
        diff(&libs, "C10", x);
    }
}

// ---------------------------------------------------------------------------
// C11 — `j`-digit boundaries: j == 2*i gains a digit before i does
// ---------------------------------------------------------------------------
#[test]
fn c11_digit_boundaries_for_j() {
    let libs = Libs::load();
    let mut xs = Vec::new();
    let mut p: i64 = 10;
    while p <= 1_000_000 {
        // j first reaches `p` at i == p/2, i.e. x must exceed p/2.
        let base = p / 2;
        for d in [-1i64, 0, 1, 2] {
            let v = base + d;
            if v >= 0 {
                xs.push(v as i32);
            }
        }
        p *= 10;
    }
    xs.sort_unstable();
    xs.dedup();
    for x in xs {
        diff(&libs, "C11", x);
    }
    // Explicitly assert the skew really exists in the C output (guards against
    // the test degenerating into a no-op).
    let five = libs.run(Impl::C, 6);
    assert_eq!(five, b"0 0\n1 2\n2 4\n3 6\n4 8\n5 10\n");
}

// ---------------------------------------------------------------------------
// C12 — stdout buffer boundary straddling
// ---------------------------------------------------------------------------
#[test]
fn c12_stdout_buffer_boundaries() {
    let libs = Libs::load();
    let mut xs = Vec::new();
    for target in [4096usize, 8192, 65536, 131_072] {
        // Find the smallest x whose output reaches `target` bytes.
        let mut x = 0i32;
        let mut n = 0usize;
        let mut i: i32 = 0;
        let mut j: i32 = 0;
        while n < target {
            n += format!("{i} {j}\n").len();
            i += 1;
            j += 2;
            x += 1;
        }
        for d in [-2i32, -1, 0, 1, 2] {
            let v = x + d;
            if v >= 0 {
                xs.push(v);
            }
        }
    }
    xs.sort_unstable();
    xs.dedup();
    for x in &xs {
        diff(&libs, "C12", *x);
    }
    // Confirm at least one candidate really straddles a 4 KiB boundary.
    assert!(
        xs.iter().any(|&x| out_len(x) >= 4096 && out_len(x - 1) < 4096),
        "[C12] no x found that crosses the 4 KiB boundary"
    );
}

// ---------------------------------------------------------------------------
// C14 — repeated invocation, no residual state
// ---------------------------------------------------------------------------
#[test]
fn c14_repeated_invocation() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 14);
    for _ in 0..8 {
        let x = rng.range_i32(0, 300);
        let times = 50;
        let c = libs.run_repeated(Impl::C, x, times);
        let r = libs.run_repeated(Impl::Rust, x, times);
        assert_same("C14", x, &c, &r);
        let mut model = Vec::new();
        for _ in 0..times {
            model.extend_from_slice(&expected(x));
        }
        assert_eq!(c, model, "[C14] repeated C output diverges from model, x={x}");
    }
}

// ---------------------------------------------------------------------------
// C15 — interleaved C/Rust invocation on the one shared libc stdout
// ---------------------------------------------------------------------------
#[test]
fn c15_interleaved_invocation() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 15);
    for _ in 0..200 {
        let x = rng.range_i32(-50, 400);
        let c1 = libs.run(Impl::C, x);
        let r1 = libs.run(Impl::Rust, x);
        let c2 = libs.run(Impl::C, x);
        let r2 = libs.run(Impl::Rust, x);
        assert_same("C15 c1/r1", x, &c1, &r1);
        assert_same("C15 c2/r2", x, &c2, &r2);
        assert_eq!(c1, c2, "[C15] C not idempotent for x={x}");
        assert_eq!(r1, r2, "[C15] Rust not idempotent for x={x}");
    }
}

// ---------------------------------------------------------------------------
// C16 — stdout buffering modes
// ---------------------------------------------------------------------------
#[test]
fn c16_stdout_buffering_modes() {
    let libs = Libs::load();
    let modes: [(&str, i32, usize); 4] = [
        ("fully-buffered/4096", common::IOFBF, 4096),
        ("fully-buffered/64", common::IOFBF, 64),
        ("line-buffered", common::IOLBF, 4096),
        ("unbuffered", common::IONBF, 0),
    ];
    let mut rng = Rng::new(SEED ^ 16);
    for (name, mode, size) in modes {
        for x in [0i32, 1, 7, 129, rng.range_i32(200, 3000)] {
            let c = libs.run_with_bufmode(Impl::C, x, mode, size);
            let r = libs.run_with_bufmode(Impl::Rust, x, mode, size);
            assert_same(&format!("C16 {name}"), x, &c, &r);
            assert_eq!(
                c,
                expected(x),
                "[C16 {name}] C output diverges from model, x={x}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// C17 — randomized full negative range
// ---------------------------------------------------------------------------
#[test]
fn c17_randomized_negatives() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 17);
    for x in [i32::MIN, i32::MIN + 1, -1, -2] {
        diff(&libs, "C17", x);
    }
    for _ in 0..256 {
        let x = rng.range_i32(i32::MIN, -1);
        let c = libs.run(Impl::C, x);
        let r = libs.run(Impl::Rust, x);
        assert_same("C17", x, &c, &r);
        assert!(c.is_empty(), "[C17] C emitted output for negative x={x}");
    }
}

// ---------------------------------------------------------------------------
// C18 — positives and negatives interleaved in one process
// ---------------------------------------------------------------------------
#[test]
fn c18_mixed_sign_interleaved() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 18);
    for _ in 0..300 {
        let pos = rng.range_i32(1, 2000);
        let neg = rng.range_i32(i32::MIN, -1);
        for x in [pos, neg, 0, pos, neg] {
            diff(&libs, "C18", x);
        }
    }
}

// ---------------------------------------------------------------------------
// Broad randomized sweep over the whole i32 domain, restricted to magnitudes
// that terminate quickly. Catches value-dependent formatting bugs that the
// banded rows above might miss.
// ---------------------------------------------------------------------------
#[test]
fn broad_randomized_sweep() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 0xFFFF);
    for _ in 0..400 {
        // Any 32-bit pattern; negatives short-circuit, positives are clamped so
        // the test stays fast.
        let raw = rng.any_i32();
        let x = if raw > 0 { raw % 5_000 } else { raw };
        diff(&libs, "sweep", x);
    }
}
