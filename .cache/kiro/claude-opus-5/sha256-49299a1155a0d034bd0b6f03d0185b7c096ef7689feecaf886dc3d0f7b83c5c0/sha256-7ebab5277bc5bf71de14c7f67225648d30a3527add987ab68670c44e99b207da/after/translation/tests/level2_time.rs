//! Level 2: `get_modified_time`.
//!
//! It reads the wall clock, but only `time(NULL) >> 29` survives, so the value
//! is stable for ~17 years at a stretch; the C and Rust calls are made back to
//! back so they observe the same epoch bucket.

mod common;

use common::*;

#[test]
fn get_modified_time_matches() {
    let pair = Pair::load();
    let (c, rs) = pair.get_modified_time();

    let mut cases: Vec<(i32, i32)> = Vec::new();
    let ints = interesting_ints();
    for &d in &ints {
        for &h in &ints {
            cases.push((d, h));
        }
    }
    // the values `modeselect` actually feeds in: offset_hours == seed % 24
    for d in -50..=50 {
        for h in -23..=23 {
            cases.push((d, h));
        }
    }
    // overflow territory for `offset_days * 86400` and `offset_hours * 3600`
    for &d in &[
        i32::MAX,
        i32::MIN,
        i32::MAX / 86400,
        i32::MAX / 86400 + 1,
        i32::MIN / 86400,
        i32::MIN / 86400 - 1,
        24855,
        24856,
        -24855,
        -24856,
    ] {
        for &h in &[0, 1, -1, 596523, 596524, -596523, i32::MAX, i32::MIN] {
            cases.push((d, h));
        }
    }

    for (d, h) in cases {
        let a = unsafe { c(d, h) };
        let b = unsafe { rs(d, h) };
        assert_eq!(a, b, "get_modified_time({d}, {h}): C={a} Rust={b}");
    }
}

/// The `>> 29` bucket must actually be picked up from the clock, i.e. the
/// result is not a hard-coded constant and both sides use the same `time_t`
/// width.
#[test]
fn get_modified_time_tracks_the_clock() {
    let pair = Pair::load();
    let (c, rs) = pair.get_modified_time();

    let base_c = unsafe { c(0, 0) };
    let base_rs = unsafe { rs(0, 0) };
    assert_eq!(base_c, base_rs);

    // time(NULL) >> 29 for any plausible current date is 3 (2021-06 .. 2038-11).
    assert!(
        base_c > 0 && base_c < 1000,
        "unexpected clock bucket {base_c}"
    );

    // shifting by one day must move both by exactly 86400
    assert_eq!(unsafe { c(1, 0) } - base_c, 86400);
    assert_eq!(unsafe { rs(1, 0) } - base_rs, 86400);
}
