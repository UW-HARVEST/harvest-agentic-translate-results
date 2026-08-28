//! Level 5: the public entry point `hatch`.
//!
//! `hatch` mutates both statics on every call, so its result depends on the
//! whole call history. Both libraries are driven with an identical sequence in a
//! single test inside a dedicated test binary, keeping their hidden state in
//! lockstep.

mod common;

use common::*;
use std::ffi::c_int;

#[test]
fn hatch_matches() {
    let libs = load();
    let (hatch_c, hatch_r) = libs.pair::<FnHatch>("hatch");

    let mut checked: u64 = 0;
    let mut check = |p1: c_int, p2: c_int, p3: c_int, p4: c_int| {
        let ec = unsafe { hatch_c(p1, p2, p3, p4) };
        let er = unsafe { hatch_r(p1, p2, p3, p4) };
        assert_eq!(ec, er, "hatch({p1}, {p2}, {p3}, {p4})");
        checked += 1;
    };

    // First call from the pristine zero state, then a few repeats of the same
    // arguments to pin down the state carried between calls.
    for _ in 0..8 {
        check(1, 2, 3, 4);
    }
    for _ in 0..8 {
        check(0, 0, 0, 0);
    }

    // Full cross product over a reduced but boundary-heavy value set.
    let small: &[c_int] = &[
        0,
        1,
        -1,
        2,
        -3,
        7,
        -10,
        100,
        -1000,
        65_536,
        -46_341,
        1_000_000,
        i32::MAX,
        i32::MIN,
    ];
    for &p1 in small {
        for &p2 in small {
            for &p3 in small {
                for &p4 in small {
                    check(p1, p2, p3, p4);
                }
            }
        }
    }

    // Each parameter swept over the full interesting set with the others fixed.
    for &v in INTS {
        check(v, 3, 5, 7);
        check(3, v, 5, 7);
        check(3, 5, v, 7);
        check(3, 5, 7, v);
    }

    // Deterministic pseudo-random sweep over the whole int range.
    let mut state: u32 = 0xDEAD_BEEF;
    let mut next = || {
        state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        state as c_int
    };
    for _ in 0..20_000 {
        let (a, b, c, d) = (next(), next(), next(), next());
        check(a, b, c, d);
    }

    assert!(checked > 50_000, "expected a broad sweep, only ran {checked}");
}
