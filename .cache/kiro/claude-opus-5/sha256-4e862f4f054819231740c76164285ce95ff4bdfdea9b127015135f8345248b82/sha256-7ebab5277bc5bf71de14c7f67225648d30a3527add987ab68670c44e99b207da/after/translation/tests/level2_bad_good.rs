//! Level 2: `bad(int)` and `good(int)`.
//!
//! `good()` internally calls the two `static` helpers `goodG2B()` and
//! `goodB2G(int)`, which have no external linkage in the C build (confirmed via
//! `nm -D`), so they are exercised through `good()`.
//!
//! Domain note for `bad()`: the C writes `buffer[data] = 1` into a
//! ten-element stack array with no upper-bound check, so `data >= 10` is
//! undefined behaviour. Measured against the compiled C .so, those inputs
//! either segfault or silently scribble on the frame depending on the index
//! (e.g. 12-15 and 20-23 crash, 16-19 and 24-39 do not). There is no
//! well-defined byte stream to match there, so the equivalence tests cover the
//! defined domain `data <= 9`; `bad_out_of_bounds_is_undefined_in_c`
//! documents the boundary instead of asserting on it.

mod common;

use common::{FnVoidInt, assert_same};

/// Every index that is in bounds for `int buffer[10]`.
#[test]
fn bad_matches_for_in_bounds_indices() {
    for data in 0..10 {
        assert_same::<FnVoidInt, _>("bad", |f| unsafe { f(data) }, &format!("data={data}"));
    }
}

/// The `data < 0` branch prints the "negative" diagnostic and touches nothing.
#[test]
fn bad_matches_for_negative_indices() {
    let cases = [-1, -2, -9, -10, -11, -100, -1000, -65536, i32::MIN + 1, i32::MIN];
    for data in cases {
        assert_same::<FnVoidInt, _>("bad", |f| unsafe { f(data) }, &format!("data={data}"));
    }
}

/// `good()` validates both bounds, so the whole `int` range is well defined.
#[test]
fn good_matches_across_the_int_range() {
    let mut cases: Vec<i32> = vec![
        // Around the `data >= 0 && data < 10` boundary in goodB2G.
        i32::MIN,
        i32::MIN + 1,
        -1000,
        -11,
        -10,
        -2,
        -1,
        0,
        1,
        2,
        3,
        4,
        5,
        6,
        7,
        8,
        9,
        10,
        11,
        12,
        100,
        1000,
        65536,
        i32::MAX - 1,
        i32::MAX,
    ];

    let mut state: u32 = 0x1234_5678;
    for _ in 0..200 {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        cases.push(state as i32);
    }

    for data in cases {
        assert_same::<FnVoidInt, _>("good", |f| unsafe { f(data) }, &format!("data={data}"));
    }
}

/// Repeated calls must not accumulate state: `buffer` is re-zeroed each time.
#[test]
fn repeated_calls_are_stateless_and_match() {
    for _round in 0..3 {
        for data in 0..10 {
            assert_same::<FnVoidInt, _>(
                "bad",
                |f| unsafe { f(data) },
                &format!("repeat data={data}"),
            );
            assert_same::<FnVoidInt, _>(
                "good",
                |f| unsafe { f(data) },
                &format!("repeat data={data}"),
            );
        }
    }
}

/// Documents (without asserting equivalence) that `data >= 10` leaves the
/// defined domain of the C implementation.
#[test]
fn bad_out_of_bounds_is_undefined_in_c() {
    // Highest in-bounds index still produces the all-zeros-but-one listing.
    let l = common::libs();
    let c: libloading::Symbol<'static, FnVoidInt> = unsafe { common::sym(&l.c, "bad") };
    let out = common::capture_stdout(|| unsafe { c(9) });
    assert_eq!(
        out, b"0\n0\n0\n0\n0\n0\n0\n0\n0\n1\n",
        "unexpected C output for bad(9): {}",
        common::show(&out)
    );
}
