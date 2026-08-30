// Phase C — error-path differential tests.
//
// One test per row of ERRORS.md (E1..E6, B1..B4, B6). `driver` returns
// `void`, so the "error code" travelled to the caller is the `Result: <n>`
// line plus the specific rejection message; every assertion pins those exact
// bytes rather than merely checking that "both failed somehow".

use crate::common::*;
use crate::Case;

const MSG_X: &str = "Error: x != 1\nOperation failed\nResult: 1\n";
const MSG_Y: &str = "Error: x == 1 but y != 2\nOperation failed\nResult: 2\n";
const MSG_Z: &str = "Error: x == 1 and y == 2, but z != 3\nOperation failed\nResult: 3\n";
const MSG_OK: &str = "Ok!\nResult: 0\n";

// --- E1: x != 1 -> result 1 ------------------------------------------------

fn e1_x_not_one() {
    let mut rng = Rng::new(SEED ^ 0xE1);
    // Hand-picked plus randomized invalid `x`.
    for x in [0, -1, 2, 3, -1000, i32::MIN, i32::MAX] {
        assert_same_and_eq(x, 2, 3, MSG_X);
        assert_same_and_eq(x, 0, 0, MSG_X);
        assert_same_and_eq(x, i32::MIN, i32::MAX, MSG_X);
    }
    for _ in 0..1500 {
        let x = rng.i32_except(1);
        let y = rng.interesting_i32();
        let z = rng.interesting_i32();
        assert_same_and_eq(x, y, z, MSG_X);
    }
}

// --- E2: x == 1 && y != 2 -> result 2 --------------------------------------

fn e2_y_not_two() {
    let mut rng = Rng::new(SEED ^ 0xE2);
    for y in [0, -1, 1, 3, 123, -1000, i32::MIN, i32::MAX] {
        assert_same_and_eq(1, y, 3, MSG_Y);
        assert_same_and_eq(1, y, 0, MSG_Y);
        assert_same_and_eq(1, y, i32::MIN, MSG_Y);
    }
    for _ in 0..1500 {
        let y = rng.i32_except(2);
        let z = rng.interesting_i32();
        assert_same_and_eq(1, y, z, MSG_Y);
    }
}

// --- E3: x == 1 && y == 2 && z != 3 -> result 3 ----------------------------

fn e3_z_not_three() {
    let mut rng = Rng::new(SEED ^ 0xE3);
    for z in [0, -1, 1, 2, 4, 123, -1000, i32::MIN, i32::MAX] {
        assert_same_and_eq(1, 2, z, MSG_Z);
    }
    for _ in 0..1500 {
        let z = rng.i32_except(3);
        assert_same_and_eq(1, 2, z, MSG_Z);
    }
}

// --- E4: the shared `fail:` epilogue -------------------------------------

fn e4_fail_epilogue_only_on_error() {
    // Present on every error path...
    for (x, y, z) in [(0, 0, 0), (1, 0, 0), (1, 2, 0)] {
        let out = assert_same(x, y, z);
        let s = String::from_utf8(out).unwrap();
        assert!(
            s.contains("Operation failed\n"),
            "error path must run the `fail:` epilogue, got {s:?}"
        );
        // ...and exactly once, between the message and the Result line.
        assert_eq!(s.matches("Operation failed").count(), 1, "got {s:?}");
    }
    // ...and absent on the success path.
    let out = assert_same(1, 2, 3);
    let s = String::from_utf8(out).unwrap();
    assert!(
        !s.contains("Operation failed"),
        "success path must not run the `fail:` epilogue, got {s:?}"
    );
    assert_eq!(s, MSG_OK);
}

// --- E5: first failing check short-circuits the rest ----------------------

fn e5_first_check_wins_all_invalid() {
    let mut rng = Rng::new(SEED ^ 0xE5);
    // All three invalid: only the `x` message may appear.
    for _ in 0..1000 {
        let x = rng.i32_except(1);
        let y = rng.i32_except(2);
        let z = rng.i32_except(3);
        let out = assert_same(x, y, z);
        let s = String::from_utf8(out).unwrap();
        assert_eq!(s, MSG_X, "for driver({x}, {y}, {z})");
        assert!(!s.contains("but y != 2"));
        assert!(!s.contains("but z != 3"));
    }
    assert_same_and_eq(i32::MIN, i32::MIN, i32::MIN, MSG_X);
    assert_same_and_eq(i32::MAX, i32::MAX, i32::MAX, MSG_X);
    assert_same_and_eq(0, 0, 0, MSG_X);
}

// --- E6: y check beats z check --------------------------------------------

fn e6_y_check_beats_z_check() {
    let mut rng = Rng::new(SEED ^ 0xE6);
    for _ in 0..1000 {
        let y = rng.i32_except(2);
        let z = rng.i32_except(3);
        let out = assert_same(1, y, z);
        let s = String::from_utf8(out).unwrap();
        assert_eq!(s, MSG_Y, "for driver(1, {y}, {z})");
        assert!(!s.contains("but z != 3"));
    }
}

// --- B1: extreme sentinels in every position -----------------------------

fn b1_extreme_sentinels() {
    const EX: [i32; 4] = [i32::MIN, i32::MIN + 1, i32::MAX - 1, i32::MAX];
    for &v in &EX {
        // In the x slot -> E1.
        assert_same_and_eq(v, 2, 3, MSG_X);
        // In the y slot -> E2 (must not wrap into 2).
        assert_same_and_eq(1, v, 3, MSG_Y);
        // In the z slot -> E3 (must not wrap into 3).
        assert_same_and_eq(1, 2, v, MSG_Z);
    }
    // Full cross-product of the extremes.
    for &x in &EX {
        for &y in &EX {
            for &z in &EX {
                assert_same_and_eq(x, y, z, MSG_X);
            }
        }
    }
}

// --- B2: 0 and -1 sentinels ----------------------------------------------

fn b2_zero_and_minus_one() {
    for &v in &[0i32, -1i32] {
        assert_same_and_eq(v, 2, 3, MSG_X);
        assert_same_and_eq(1, v, 3, MSG_Y);
        assert_same_and_eq(1, 2, v, MSG_Z);
    }
    assert_same_and_eq(0, 0, 0, MSG_X);
    assert_same_and_eq(-1, -1, -1, MSG_X);
}

// --- B3: one step past each valid value ----------------------------------

fn b3_one_step_past_valid() {
    // x valid only at 1
    assert_same_and_eq(0, 2, 3, MSG_X);
    assert_same_and_eq(2, 2, 3, MSG_X);
    // y valid only at 2
    assert_same_and_eq(1, 1, 3, MSG_Y);
    assert_same_and_eq(1, 3, 3, MSG_Y);
    // z valid only at 3
    assert_same_and_eq(1, 2, 2, MSG_Z);
    assert_same_and_eq(1, 2, 4, MSG_Z);
    // and the accepting point itself, for contrast
    assert_same_and_eq(1, 2, 3, MSG_OK);
}

// --- B4: "out-of-range enum" ints, incl. the static's initialiser --------

fn b4_out_of_range_enum_like_ints() {
    // 123 is the C `static int y = 123;` initialiser. It must be treated as
    // an ordinary invalid value, never as "already initialised / valid".
    assert_same_and_eq(1, 123, 3, MSG_Y);
    assert_same_and_eq(123, 2, 3, MSG_X);
    assert_same_and_eq(1, 2, 123, MSG_Z);
    assert_same_and_eq(123, 123, 123, MSG_X);

    // Values a C enum would never define but an `int` parameter accepts.
    let mut rng = Rng::new(SEED ^ 0xB4);
    for _ in 0..1000 {
        let v = rng.next_i32();
        // Whatever v is, the classification must agree between C and Rust.
        assert_same(v, 2, 3);
        assert_same(1, v, 3);
        assert_same(1, 2, v);
    }
    for v in [4i32, 5, 99, 1 << 30, -(1 << 30), 0x7FFF_FFFE, -0x7FFF_FFFF] {
        assert_same(v, v, v);
        assert_same(1, v, v);
    }
}

// --- B6: repeated / interleaved calls, persistent `static y` -------------

fn b6_repeated_and_interleaved_calls() {
    // Failing call must not poison a later success, and vice versa.
    let seqs: [&[(i32, i32, i32)]; 6] = [
        &[(1, 2, 3), (1, 5, 3), (1, 2, 3)],
        &[(1, 5, 3), (1, 5, 3), (1, 2, 3)],
        &[(0, 0, 0), (1, 2, 3)],
        &[(1, 2, 3), (0, 0, 0)],
        &[(1, 2, 3); 10],
        &[(1, 123, 3), (1, 2, 3), (1, 123, 3)],
    ];
    for s in seqs {
        let out = assert_same_seq(s);
        let model: String = s.iter().map(|&(x, y, z)| expected_output(x, y, z)).collect();
        assert_eq!(String::from_utf8_lossy(&out), model, "sequence {s:?}");
    }

    // Long randomized interleaving.
    let mut rng = Rng::new(SEED ^ 0xB6);
    let calls: Vec<(i32, i32, i32)> = (0..500)
        .map(|i| {
            if i % 3 == 0 {
                (1, 2, 3)
            } else {
                (
                    rng.interesting_i32(),
                    rng.interesting_i32(),
                    rng.interesting_i32(),
                )
            }
        })
        .collect();
    let out = assert_same_seq(&calls);
    let model: String = calls
        .iter()
        .map(|&(x, y, z)| expected_output(x, y, z))
        .collect();
    assert_eq!(String::from_utf8_lossy(&out), model);
}

/// Registry of this module's cases, in execution order.
pub fn cases() -> Vec<Case> {
    vec![
        ("e1_x_not_one", e1_x_not_one as fn()),
        ("e2_y_not_two", e2_y_not_two as fn()),
        ("e3_z_not_three", e3_z_not_three as fn()),
        ("e4_fail_epilogue_only_on_error", e4_fail_epilogue_only_on_error as fn()),
        ("e5_first_check_wins_all_invalid", e5_first_check_wins_all_invalid as fn()),
        ("e6_y_check_beats_z_check", e6_y_check_beats_z_check as fn()),
        ("b1_extreme_sentinels", b1_extreme_sentinels as fn()),
        ("b2_zero_and_minus_one", b2_zero_and_minus_one as fn()),
        ("b3_one_step_past_valid", b3_one_step_past_valid as fn()),
        ("b4_out_of_range_enum_like_ints", b4_out_of_range_enum_like_ints as fn()),
        ("b6_repeated_and_interleaved_calls", b6_repeated_and_interleaved_calls as fn()),
    ]
}
