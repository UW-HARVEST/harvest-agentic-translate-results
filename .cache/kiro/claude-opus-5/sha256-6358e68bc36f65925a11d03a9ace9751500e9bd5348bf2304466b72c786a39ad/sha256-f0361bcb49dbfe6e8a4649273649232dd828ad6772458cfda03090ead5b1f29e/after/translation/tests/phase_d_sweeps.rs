//! Phase D — high-volume adversarial sweeps and symbol parity.
//!
//! `CONFIGS.md` / `ERRORS.md` are hand-derived from the C source, so they can
//! only be as complete as that reading. These tests are the mechanical
//! backstop: hundreds of thousands of inputs drawn so that every float class,
//! every exponent decade and every `errno` outcome is hit many times.
//!
//! The bulk sweeps use a cheap comparison (return bits + `errno`, with `stderr`
//! sent to `/dev/null`) so the iteration count can be high; the smaller sweeps
//! additionally compare `stderr` byte-for-byte.

mod common;

use std::ffi::{c_int, c_void};

use common::{c_pow, diff_with_errno, rust_pow, PowFn, Rng};

unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn open(path: *const i8, flags: c_int, ...) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn __errno_location() -> *mut c_int;
}

const O_WRONLY: c_int = 1;

/// Silences fd 2 for the lifetime of the guard so a sweep can make millions of
/// `fprintf` calls without producing gigabytes of output.
struct DevNullStderr {
    saved: c_int,
    devnull: c_int,
    _guard: std::sync::MutexGuard<'static, ()>,
}

impl DevNullStderr {
    fn new() -> Self {
        // Held for the guard's whole lifetime: fd 2 is process-global and other
        // tests redirect it concurrently.
        let _guard = common::lock_stderr();
        unsafe {
            fflush(std::ptr::null_mut());
            let saved = dup(2);
            let devnull = open(c"/dev/null".as_ptr(), O_WRONLY);
            assert!(saved >= 0 && devnull >= 0, "could not open /dev/null");
            assert!(dup2(devnull, 2) >= 0, "dup2 onto fd 2 failed");
            DevNullStderr { saved, devnull, _guard }
        }
    }
}

impl Drop for DevNullStderr {
    fn drop(&mut self) {
        unsafe {
            fflush(std::ptr::null_mut());
            dup2(self.saved, 2);
            close(self.saved);
            close(self.devnull);
        }
    }
}

/// `(return bits, errno)` for one call, with `errno` pre-set to 0.
#[inline]
fn call_quiet(f: PowFn, base: f64, exponent: f64) -> (u64, c_int) {
    unsafe {
        *__errno_location() = 0;
        let v = f(base, exponent);
        (v.to_bits(), *__errno_location())
    }
}

/// Runs `gen` `n` times and compares both libraries. Returns a histogram of the
/// `errno` outcomes so the test can assert the sweep actually reached the error
/// branches instead of silently only testing the happy path.
fn sweep(name: &str, n: usize, seed: u64, mut make: impl FnMut(&mut Rng) -> (f64, f64)) -> [usize; 3] {
    let _silence = DevNullStderr::new();
    let c = c_pow();
    let r = rust_pow();
    let mut rng = Rng::new(seed);
    // [no error, EDOM, ERANGE]
    let mut hist = [0usize; 3];

    for i in 0..n {
        let (b, e) = make(&mut rng);
        let (cv, ce) = call_quiet(c, b, e);
        let (rv, re) = call_quiet(r, b, e);
        assert!(
            cv == rv && ce == re,
            "[{name}] divergence at iteration {i}: my_pow({b:?} /*{:#018x}*/, {e:?} /*{:#018x}*/) \
             -> C: bits={cv:#018x} errno={ce}, Rust: bits={rv:#018x} errno={re}",
            b.to_bits(),
            e.to_bits(),
        );
        match ce {
            common::EDOM => hist[1] += 1,
            common::ERANGE => hist[2] += 1,
            _ => hist[0] += 1,
        }
    }
    hist
}

#[test]
fn d1_sweep_full_random_bit_patterns() {
    let h = sweep("D1", 150_000, 0xD1_5EED, |rng| (rng.raw_f64(), rng.raw_f64()));
    assert!(h[0] > 0, "D1 never reached the no-error path: {h:?}");
}

#[test]
fn d2_sweep_random_signs_and_decades() {
    // base = ±m × 10^k with m in [1,10) and k in [-320, 308]; exponent likewise.
    // This guarantees a dense mix of overflow, underflow and normal results.
    let h = sweep("D2", 150_000, 0xD2_5EED, |rng| {
        let mk = |rng: &mut Rng, kmin: i64, kmax: i64| {
            let m = rng.range(1.0, 10.0);
            let k = rng.int(kmin, kmax) as i32;
            let s = if rng.next_u64() & 1 == 0 { 1.0 } else { -1.0 };
            s * m * 10f64.powi(k)
        };
        (mk(rng, -320, 308), mk(rng, -4, 4))
    });
    assert!(h[0] > 0 && h[1] > 0 && h[2] > 0, "D2 missed a branch: {h:?}");
}

#[test]
fn d3_sweep_integer_exponents_both_parities() {
    let h = sweep("D3", 150_000, 0xD3_5EED, |rng| {
        let s = if rng.next_u64() & 1 == 0 { 1.0 } else { -1.0 };
        let b = s * rng.range(0.0, 1000.0);
        let e = rng.int(-2100, 2100) as f64;
        (b, e)
    });
    assert!(h[0] > 0 && h[2] > 0, "D3 missed a branch: {h:?}");
}

#[test]
fn d4_sweep_near_overflow_underflow_boundaries() {
    // Exponents chosen so |result| lands within a few ULPs of DBL_MAX or of the
    // smallest normal — the region where the ERANGE decision flips.
    let h = sweep("D4", 120_000, 0xD4_5EED, |rng| {
        let b = rng.range(1.0000001, 3.0) * if rng.next_u64() & 1 == 0 { 1.0 } else { -1.0 };
        let target = if rng.next_u64() & 1 == 0 { 709.78 } else { -708.4 };
        // e ~ target / ln|b|, jittered across the boundary.
        let e = target / b.abs().ln() * rng.range(0.9995, 1.0005);
        (b, e)
    });
    assert!(h[1] > 0 && h[2] > 0, "D4 missed a branch: {h:?}");
}

#[test]
fn d5_sweep_around_unity_and_zero() {
    // |base| within a few ULPs of 1.0 and of 0.0, paired with huge exponents:
    // the region where pow's argument reduction is most delicate.
    let h = sweep("D5", 120_000, 0xD5_5EED, |rng| {
        let b = match rng.next_u64() % 4 {
            0 => 1.0 + rng.range(-8.0, 8.0) * f64::EPSILON,
            1 => -1.0 + rng.range(-8.0, 8.0) * f64::EPSILON,
            2 => rng.range(0.0, 16.0) * 5e-324,
            _ => -rng.range(0.0, 16.0) * 5e-324,
        };
        let e = match rng.next_u64() % 3 {
            0 => rng.int(-1000000, 1000000) as f64,
            1 => rng.range(-1e18, 1e18),
            _ => rng.range(-4.0, 4.0),
        };
        (b, e)
    });
    assert!(h[0] > 0, "D5 never reached the no-error path: {h:?}");
}

#[test]
fn d6_sweep_special_values_mixed_with_randoms() {
    let h = sweep("D6", 120_000, 0xD6_5EED, |rng| {
        let pick = |rng: &mut Rng| {
            let pool = common::SPECIALS;
            if rng.next_u64() % 3 == 0 {
                rng.raw_f64()
            } else {
                pool[(rng.next_u64() as usize) % pool.len()]
            }
        };
        (pick(rng), pick(rng))
    });
    assert!(h[0] > 0, "D6 never reached the no-error path: {h:?}");
}

/// Same idea as the sweeps above but with full `stderr` byte comparison, so the
/// diagnostic text is also covered by randomized inputs rather than only by the
/// hand-written `ERRORS.md` cases. Fewer iterations because each one costs two
/// fd redirections.
#[test]
fn d7_randomized_stderr_text_comparison() {
    let mut rng = Rng::new(0xD7_5EED);
    let mut edom = 0usize;
    let mut erange = 0usize;
    for _ in 0..1500 {
        // Bias hard towards the two error branches so the %.2f rendering is
        // exercised across many magnitudes and both signs of zero.
        let (b, e) = match rng.next_u64() % 4 {
            // EDOM: negative base, non-integer exponent.
            0 => (
                -rng.range(0.0, 1e6) * 10f64.powi(rng.int(-300, 300) as i32),
                rng.range(-50.0, 50.0) + 0.5,
            ),
            // ERANGE overflow / underflow.
            1 => (
                rng.range(1.5, 10.0) * if rng.next_u64() & 1 == 0 { 1.0 } else { -1.0 },
                rng.int(-4000, 4000) as f64,
            ),
            // ERANGE pole.
            2 => (
                if rng.next_u64() & 1 == 0 { 0.0 } else { -0.0 },
                -(rng.int(1, 200) as f64) - if rng.next_u64() & 1 == 0 { 0.0 } else { 0.5 },
            ),
            // Anything at all.
            _ => (rng.raw_f64(), rng.raw_f64()),
        };
        diff_with_errno(b, e, 0, "D7");
        // Track coverage using the C library as the oracle.
        let (_, ce) = {
            let _s = DevNullStderr::new();
            call_quiet(c_pow(), b, e)
        };
        match ce {
            common::EDOM => edom += 1,
            common::ERANGE => erange += 1,
            _ => {}
        }
    }
    assert!(
        edom > 100 && erange > 100,
        "D7 did not exercise both diagnostics enough: EDOM={edom}, ERANGE={erange}"
    );
}

/// Phase D symbol parity, asserted from inside the test suite so it cannot be
/// forgotten: every symbol the C `.so` defines must also be defined by the Rust
/// `.so`, and `my_pow` must be resolvable through `dlsym` in both (already
/// implied by every other test, since all calls go through `dlsym`).
#[test]
fn d8_symbol_parity() {
    use std::process::Command;

    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_so = std::env::var("POW_C_SO")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| manifest.join("../c_src/build/libpow.so"));
    let rust_so = std::env::var("POW_RUST_SO")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            let p = manifest.join("target/debug/libpow.so");
            if p.exists() { p } else { manifest.join("target/release/libpow.so") }
        });

    let defined = |p: &std::path::Path| -> Vec<String> {
        let out = Command::new("nm")
            .arg("-D")
            .arg(p)
            .output()
            .expect("running `nm -D` failed");
        assert!(out.status.success(), "nm -D {p:?} failed");
        let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|l| {
                let f: Vec<&str> = l.split_whitespace().collect();
                let (kind, name) = match f.len() {
                    2 => (f[0], f[1]),
                    3 => (f[1], f[2]),
                    _ => return None,
                };
                // Keep defined symbols only ("U" = undefined import).
                if kind == "U" {
                    return None;
                }
                Some(name.to_string())
            })
            .collect();
        v.sort();
        v.dedup();
        v
    };

    let c_syms = defined(&c_so);
    let r_syms = defined(&rust_so);

    assert!(
        c_syms.iter().any(|s| s == "my_pow"),
        "the C .so does not define my_pow; wrong library? {c_so:?}"
    );

    let missing: Vec<&String> = c_syms.iter().filter(|s| !r_syms.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "symbols defined by the C .so but missing from the Rust .so: {missing:?}"
    );
}

// ------------------------------------------------- structured exhaustive grids
//
// The randomized sweeps above sample the input space; these two walk it
// systematically so that *every* one of the 2048 possible biased-exponent
// fields of a `double` is exercised, in both signs, for both parameters. That
// covers every float class (zero, subnormal, all normal decades, infinity, NaN)
// by construction rather than by luck.

const MANTISSAS: [u64; 4] = [
    0x0_0000_0000_0000,
    0x8_0000_0000_0000,
    0xF_FFFF_FFFF_FFFF,
    0x5_5555_5555_5555,
];

fn build(sign: u64, exp_field: u64, mantissa: u64) -> f64 {
    f64::from_bits((sign << 63) | (exp_field << 52) | mantissa)
}

#[test]
fn d13_every_exponent_field_of_the_base() {
    let _silence = DevNullStderr::new();
    let c = c_pow();
    let r = rust_pow();

    let exponents: [f64; 16] = [
        0.0, -0.0, 1.0, -1.0, 2.0, -2.0, 3.0, -3.0, 0.5, -0.5, 2.5, -2.5, 1e300, -1e300,
        f64::INFINITY, f64::NAN,
    ];

    for exp_field in 0u64..=0x7FF {
        for &mantissa in &MANTISSAS {
            for sign in [0u64, 1] {
                let b = build(sign, exp_field, mantissa);
                for &e in &exponents {
                    let (cv, ce) = call_quiet(c, b, e);
                    let (rv, re) = call_quiet(r, b, e);
                    assert!(
                        cv == rv && ce == re,
                        "[D13] divergence for my_pow({b:?} /*{:#018x}*/, {e:?}) -> \
                         C: bits={cv:#018x} errno={ce}, Rust: bits={rv:#018x} errno={re}",
                        b.to_bits(),
                    );
                }
            }
        }
    }
}

#[test]
fn d14_every_exponent_field_of_the_exponent() {
    let _silence = DevNullStderr::new();
    let c = c_pow();
    let r = rust_pow();

    let bases: [f64; 16] = [
        0.0, -0.0, 1.0, -1.0, 2.0, -2.0, 0.5, -0.5, 1e300, -1e300, 5e-324, -5e-324,
        f64::INFINITY, f64::NEG_INFINITY, f64::NAN, 1.0000000000000002,
    ];

    for exp_field in 0u64..=0x7FF {
        for &mantissa in &MANTISSAS {
            for sign in [0u64, 1] {
                let e = build(sign, exp_field, mantissa);
                for &b in &bases {
                    let (cv, ce) = call_quiet(c, b, e);
                    let (rv, re) = call_quiet(r, b, e);
                    assert!(
                        cv == rv && ce == re,
                        "[D14] divergence for my_pow({b:?}, {e:?} /*{:#018x}*/) -> \
                         C: bits={cv:#018x} errno={ce}, Rust: bits={rv:#018x} errno={re}",
                        e.to_bits(),
                    );
                }
            }
        }
    }
}
