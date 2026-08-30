//! Differential tests for `my_pow`, the only symbol the C library exports.
//!
//! Every call goes through `dlopen`/`dlsym` on both shared objects, so the
//! `#[no_mangle]` export wrapper is part of what is being tested.
//!
//! Both sides are compared on three axes:
//!   * the returned `double`, bit-for-bit (so NaN payloads and signed zeros
//!     are not silently accepted as equal);
//!   * `errno` as observed by the caller after the call returns;
//!   * the exact bytes written to file descriptor 2.

mod common;

use common::{Observation, Pair, inputs, observe};

fn fmt(x: f64) -> String {
    format!("{x:?} (bits {:#018x})", x.to_bits())
}

fn assert_same(label: &str, base: f64, exponent: f64, c: &Observation, rust: &Observation) {
    assert_eq!(
        c.bits,
        rust.bits,
        "[{label}] return value mismatch for my_pow({}, {}):\n  C    = {} \n  Rust = {}",
        fmt(base),
        fmt(exponent),
        fmt(f64::from_bits(c.bits)),
        fmt(f64::from_bits(rust.bits)),
    );
    assert_eq!(
        c.stderr,
        rust.stderr,
        "[{label}] stderr mismatch for my_pow({}, {}):\n  C    = {:?}\n  Rust = {:?}",
        fmt(base),
        fmt(exponent),
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&rust.stderr),
    );
    assert_eq!(
        c.errno_after,
        rust.errno_after,
        "[{label}] errno mismatch for my_pow({}, {}): C = {}, Rust = {}",
        fmt(base),
        fmt(exponent),
        c.errno_after,
        rust.errno_after,
    );
}

/// Both libraries export `my_pow` and it is reachable via `dlsym`.
#[test]
fn exports_my_pow() {
    let _ = Pair::load();
}

/// Core sweep: identical results, `errno` and stderr for the whole input table.
#[test]
fn matches_c_for_all_inputs() {
    let p = Pair::load();
    for (base, exponent) in inputs() {
        let c = observe("c", p.c, base, exponent, 0);
        let rust = observe("rust", p.rust, base, exponent, 0);
        assert_same("clean-errno", base, exponent, &c, &rust);
    }
}

/// The callee resets `errno` to 0 first, so a dirty incoming `errno` must not
/// change anything. Seeds include EDOM (33) and ERANGE (34) themselves.
#[test]
fn matches_c_with_dirty_incoming_errno() {
    let p = Pair::load();
    for seed in [0, 1, 2, 22, 33, 34, 75, 12345] {
        for (base, exponent) in inputs() {
            let c = observe("c", p.c, base, exponent, seed);
            let rust = observe("rust", p.rust, base, exponent, seed);
            assert_same(&format!("errno-seed-{seed}"), base, exponent, &c, &rust);
        }
    }
}

/// Calling Rust first must not change the verdict: neither implementation may
/// depend on residue the other one left behind.
#[test]
fn matches_c_with_rust_called_first() {
    let p = Pair::load();
    for (base, exponent) in inputs() {
        let rust = observe("rust", p.rust, base, exponent, 0);
        let c = observe("c", p.c, base, exponent, 0);
        assert_same("rust-first", base, exponent, &c, &rust);
    }
}

/// Repeated calls on the same input are stable and still agree, which catches
/// state that leaks between invocations.
#[test]
fn repeated_calls_are_stable() {
    let p = Pair::load();
    for (base, exponent) in inputs() {
        let first_c = observe("c", p.c, base, exponent, 0);
        let first_rust = observe("rust", p.rust, base, exponent, 0);
        for _ in 0..3 {
            let c = observe("c", p.c, base, exponent, 0);
            let rust = observe("rust", p.rust, base, exponent, 0);
            assert_eq!(c, first_c, "C not stable for my_pow({base}, {exponent})");
            assert_eq!(
                rust, first_rust,
                "Rust not stable for my_pow({base}, {exponent})"
            );
            assert_same("repeat", base, exponent, &c, &rust);
        }
    }
}

/// Alternating between the two implementations without resetting `errno`
/// in between - each call must stand on its own.
#[test]
fn matches_c_when_interleaved_without_reset() {
    let p = Pair::load();
    let table = inputs();
    for w in table.windows(2) {
        let (a, b) = (w[0], w[1]);
        // Warm up each side with the neighbouring input first.
        let _ = observe("c", p.c, a.0, a.1, 0);
        let c = observe("c", p.c, b.0, b.1, 34);
        let _ = observe("rust", p.rust, a.0, a.1, 0);
        let rust = observe("rust", p.rust, b.0, b.1, 34);
        assert_same("interleaved", b.0, b.1, &c, &rust);
    }
}

/// Randomised sweep over a wide exponent range, including the negative-base
/// region that drives the EDOM branch and the extremes that drive ERANGE.
#[test]
fn matches_c_for_pseudorandom_inputs() {
    let p = Pair::load();

    // Deterministic xorshift64* so failures are reproducible.
    let mut state: u64 = 0x9E3779B97F4A7C15;
    let mut next = || {
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        state.wrapping_mul(0x2545F4914F6CDD1D)
    };

    for _ in 0..3000 {
        // Bases spanning roughly 1e-320 .. 1e320, both signs.
        let base_mag = 10f64.powf(((next() >> 11) as f64 / (1u64 << 53) as f64) * 640.0 - 320.0);
        let base = if next() & 1 == 0 { base_mag } else { -base_mag };
        // Exponents spanning -2000 .. 2000, integral about half the time.
        let mut exponent = ((next() >> 11) as f64 / (1u64 << 53) as f64) * 4000.0 - 2000.0;
        if next() & 1 == 0 {
            exponent = exponent.trunc();
        }

        let c = observe("c", p.c, base, exponent, 0);
        let rust = observe("rust", p.rust, base, exponent, 0);
        assert_same("random", base, exponent, &c, &rust);
    }
}

/// Raw bit patterns, including subnormals and non-canonical NaNs, fed straight
/// through as `double`s.
#[test]
fn matches_c_for_raw_bit_patterns() {
    let p = Pair::load();

    let patterns: [u64; 20] = [
        0x0000_0000_0000_0000, // +0
        0x8000_0000_0000_0000, // -0
        0x0000_0000_0000_0001, // smallest subnormal
        0x800f_ffff_ffff_ffff, // -largest subnormal
        0x0010_0000_0000_0000, // smallest normal
        0x7fef_ffff_ffff_ffff, // f64::MAX
        0xffef_ffff_ffff_ffff, // f64::MIN
        0x7ff0_0000_0000_0000, // +inf
        0xfff0_0000_0000_0000, // -inf
        0x7ff8_0000_0000_0000, // quiet NaN
        0xfff8_0000_0000_0000, // negative quiet NaN
        0x7ff0_0000_0000_0001, // signalling NaN
        0x7ff4_0000_0000_0000, // NaN with payload
        0x3ff0_0000_0000_0000, // 1.0
        0xbff0_0000_0000_0000, // -1.0
        0x4000_0000_0000_0000, // 2.0
        0xc000_0000_0000_0000, // -2.0
        0x3fe0_0000_0000_0000, // 0.5
        0xbfe0_0000_0000_0000, // -0.5
        0x4090_0000_0000_0000, // 1024.0
    ];

    for &bb in &patterns {
        for &eb in &patterns {
            let base = f64::from_bits(bb);
            let exponent = f64::from_bits(eb);
            let c = observe("c", p.c, base, exponent, 0);
            let rust = observe("rust", p.rust, base, exponent, 0);
            assert_same("bit-pattern", base, exponent, &c, &rust);
        }
    }
}

/// Sanity check that the suite really reaches both error branches; otherwise a
/// green run would prove very little about the `fprintf` paths.
#[test]
fn error_branches_are_exercised() {
    let p = Pair::load();

    let domain = observe("c", p.c, -8.0, 1.0 / 3.0, 0);
    assert_eq!(f64::from_bits(domain.bits), -1.0, "expected EDOM sentinel");
    assert!(
        String::from_utf8_lossy(&domain.stderr).starts_with("Domain error: pow(-8.00, 0.33)"),
        "unexpected EDOM message: {:?}",
        String::from_utf8_lossy(&domain.stderr)
    );

    let range = observe("c", p.c, 1e300, 2.0, 0);
    assert_eq!(f64::from_bits(range.bits), -1.0, "expected ERANGE sentinel");
    assert!(
        String::from_utf8_lossy(&range.stderr).starts_with("Range error: pow(1e+300, 2.00)")
            || String::from_utf8_lossy(&range.stderr).starts_with("Range error: pow("),
        "unexpected ERANGE message: {:?}",
        String::from_utf8_lossy(&range.stderr)
    );

    // And the happy path must not print anything at all.
    let ok = observe("c", p.c, 2.0, 10.0, 0);
    assert_eq!(f64::from_bits(ok.bits), 1024.0);
    assert!(ok.stderr.is_empty(), "happy path printed to stderr");
}
