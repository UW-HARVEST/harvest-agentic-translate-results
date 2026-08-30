//! Level 3: the single public entry point from `include/driver.h`,
//! `void driver(int goodData, int badData)`.
//!
//! `badData` is kept in `bad()`'s defined domain (`<= 9`); see the note in
//! `level2_bad_good.rs`.

mod common;

use common::{FnVoidIntInt, assert_same, capture_stdout, libs, show, sym};

fn good_values() -> Vec<i32> {
    let mut v = vec![
        i32::MIN,
        i32::MIN + 1,
        -100,
        -11,
        -10,
        -1,
        0,
        1,
        5,
        7,
        8,
        9,
        10,
        11,
        1000,
        i32::MAX - 1,
        i32::MAX,
    ];
    let mut state: u32 = 0x9e37_79b9;
    for _ in 0..40 {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        v.push(state as i32);
    }
    v
}

fn bad_values() -> Vec<i32> {
    let mut v: Vec<i32> = (0..10).collect();
    v.extend_from_slice(&[-1, -2, -10, -1000, i32::MIN + 1, i32::MIN]);
    v
}

#[test]
fn driver_matches_over_the_argument_cross_product() {
    for good_data in good_values() {
        for bad_data in bad_values() {
            assert_same::<FnVoidIntInt, _>(
                "driver",
                |f| unsafe { f(good_data, bad_data) },
                &format!("goodData={good_data}, badData={bad_data}"),
            );
        }
    }
}

/// The two arguments must not be swapped: assert the banner/section ordering
/// and that `goodData` only affects the `good()` section.
#[test]
fn driver_argument_roles_are_not_swapped() {
    let l = libs();
    let c: libloading::Symbol<'static, FnVoidIntInt> = unsafe { sym(&l.c, "driver") };
    let r: libloading::Symbol<'static, FnVoidIntInt> = unsafe { sym(&l.rust, "driver") };

    // goodData=3 is in bounds for goodB2G; badData=-1 takes bad()'s error path.
    let expected: &[u8] = b"Calling good()...\n\
        0\n0\n0\n0\n0\n0\n0\n1\n0\n0\n\
        0\n0\n0\n1\n0\n0\n0\n0\n0\n0\n\
        Finished good()\n\
        Calling bad()...\n\
        ERROR: Array index is negative.\n\
        Finished bad()\n";

    let c_out = capture_stdout(|| unsafe { c(3, -1) });
    let r_out = capture_stdout(|| unsafe { r(3, -1) });
    assert_eq!(c_out, expected, "C baseline drifted: {}", show(&c_out));
    assert_eq!(r_out, expected, "Rust output: {}", show(&r_out));

    // Reversing the arguments must produce a different stream in both.
    let c_swapped = capture_stdout(|| unsafe { c(-1, 3) });
    let r_swapped = capture_stdout(|| unsafe { r(-1, 3) });
    assert_eq!(c_swapped, r_swapped, "swapped-args mismatch");
    assert_ne!(c_out, c_swapped, "argument roles are indistinguishable");
}

/// `driver` must be reentrant/stateless across calls.
#[test]
fn driver_repeated_calls_match() {
    for _ in 0..5 {
        for (g, b) in [(7, 7), (0, 0), (9, 9), (-5, -5), (12345, 4)] {
            assert_same::<FnVoidIntInt, _>(
                "driver",
                |f| unsafe { f(g, b) },
                &format!("repeat ({g}, {b})"),
            );
        }
    }
}
