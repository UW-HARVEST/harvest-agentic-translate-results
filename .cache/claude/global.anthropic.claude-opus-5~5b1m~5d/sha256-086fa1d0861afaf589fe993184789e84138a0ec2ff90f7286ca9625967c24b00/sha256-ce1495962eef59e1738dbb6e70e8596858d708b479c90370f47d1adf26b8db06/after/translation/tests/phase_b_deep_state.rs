//! Phase B (continued) — deep-global-state differential tests.
//!
//! `CONFIGS.md` rows 6/7 cover moderate state accumulation. This separate test
//! binary (so it gets a freshly loaded pair of `.so`s) pushes
//! `the_house.bathrooms` past the point where an `f32` can no longer represent
//! a half-integer exactly (2^23 = 8388608) and `the_house.floors` into the
//! millions.
//!
//! Rationale: `bathrooms` is a `double` passed as a `printf` vararg. Any
//! narrowing of that vararg (or of the accumulator) is invisible while
//! `bathrooms` stays small, because every value it takes is `k + 0.5`, which
//! `f32` represents exactly below 2^23. Mutation testing showed the shallow
//! rows could NOT distinguish an `f32`-narrowed `bathrooms`; these tests can.

mod common;

use common::*;
use std::ffi::c_int;

/// bathrooms starts at 2.5 and gains 1.0 per `run`; 2^23 is the first
/// magnitude at which `k + 0.5` is no longer exactly representable in `f32`.
const F32_HALF_INTEGER_LIMIT: u64 = 1 << 23; // 8_388_608

fn bathrooms_of(out: &[u8]) -> f64 {
    let text = String::from_utf8_lossy(out);
    let line = text.lines().next().expect("at least one line");
    line.rsplit(' ')
        .nth(1)
        .unwrap_or_else(|| panic!("cannot find bathrooms in {line:?}"))
        .parse()
        .unwrap_or_else(|e| panic!("cannot parse bathrooms in {line:?}: {e}"))
}

fn floors_of(out: &[u8]) -> i64 {
    let text = String::from_utf8_lossy(out);
    let line = text.lines().next().expect("at least one line");
    line.split(' ').nth(3).unwrap().parse().unwrap()
}

/// Rows 6/7 extended: drive `bathrooms` past 2^23 and confirm `%.1f` still
/// agrees byte-for-byte, then keep going past 2^24.
#[test]
fn deep_state_bathrooms_past_f32_precision() {
    // Checkpoint at the start so we know where we are.
    let out = assert_same(&Call::Run(0));
    let start = bathrooms_of(&out);
    assert!(start < 100.0, "expected a freshly loaded pair, got {start}");

    // --- just below the f32 half-integer limit -----------------------------
    advance_both_silently(F32_HALF_INTEGER_LIMIT - 16, 0);
    let out = assert_same(&Call::Run(0));
    let b = bathrooms_of(&out);
    assert!(
        b > (F32_HALF_INTEGER_LIMIT as f64) - 32.0 && b < F32_HALF_INTEGER_LIMIT as f64,
        "expected to be just below 2^23, got {b}"
    );
    assert_eq!(b.fract(), 0.5, "bathrooms must still be a half-integer: {b}");

    // --- step across the limit one call at a time -------------------------
    for _ in 0..64 {
        let out = assert_same(&Call::Run(0));
        let b = bathrooms_of(&out);
        assert_eq!(
            b.fract(),
            0.5,
            "the C prints a half-integer here; got {b} — an f32-narrowed \
             accumulator would have printed {:.1}",
            b as f32
        );
    }

    // --- and well past it, including past 2^24 ---------------------------
    advance_both_silently(F32_HALF_INTEGER_LIMIT + 1000, 0);
    for _ in 0..32 {
        let out = assert_same(&Call::Run(0));
        let b = bathrooms_of(&out);
        assert!(b > (1u64 << 24) as f64, "expected past 2^24, got {b}");
        assert_eq!(b.fract(), 0.5, "bathrooms must still be a half-integer: {b}");
    }

    // floors has been incremented once per run and must have tracked exactly.
    let out = assert_same(&Call::Run(0));
    let f = floors_of(&out);
    assert!(f > (1i64 << 24), "expected floors past 2^24, got {f}");
    // floors and bathrooms differ by exactly the initial 0.5 offset.
    assert_eq!(
        bathrooms_of(&out) - f as f64,
        0.5,
        "floors and bathrooms must stay in step"
    );
}

/// Same deep state, but reached through `driver` and with a non-zero
/// `extra_bedrooms`, so `bedrooms` random-walks and wraps many times on the
/// way. Confirms the composed pipeline agrees at depth too.
#[test]
fn deep_state_via_driver_and_wrapping_bedrooms() {
    // Push bedrooms through many wraps while also deepening floors/bathrooms.
    advance_both_silently(200_000, c_int::MAX / 3);
    let mut rng = Rng::with_seed(Rng::SEED ^ 0xdeed);

    for _ in 0..60 {
        let v = rng.next_i32();
        let out = assert_same(&Call::Driver(v.to_string().into_bytes()));
        assert_eq!(line_count(&out), DRIVER_OK_LINES);
    }
    for _ in 0..60 {
        assert_same(&Call::Run(rng.next_i32()));
    }
    // Rejections still behave at depth.
    for bad in ["", "abc", "2147483648", "-2147483649", "9".repeat(300).as_str()] {
        let out = assert_same(&Call::Driver(bad.as_bytes().to_vec()));
        assert_eq!(out, ERR_MSG, "input {bad:?}");
    }
}
