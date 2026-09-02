//! Phase C — error-path differential tests, one test per `ERRORS.md` row.
//!
//! Every call goes through `dlsym` on both shared objects. Each test asserts
//! the *same* error code / sentinel, not merely that "both failed".

mod common;

use common::*;
use std::ffi::c_void;

const MODES: [i32; 3] = [0, 1, 2];

// ---------------------------------------------------------------------------
// E1 / E2 — invalid `iterations` => -1
// ---------------------------------------------------------------------------

#[test]
fn e1_iterations_negative_returns_minus_1() {
    let libs = Pair::load();
    let (cf, rf) = libs.gotomach();
    let mut rng = Rng::new(0xE001);

    let mut cases: Vec<i32> = vec![-1, -2, -10, -1000, -65535, -65536, i32::MIN, i32::MIN + 1];
    for _ in 0..300 {
        cases.push(rng.i32_in(i32::MIN, -1));
    }

    for &it in &cases {
        // Vary the other three arguments: the check must fire regardless, and
        // `iterations` is validated *before* `seed`, so even an invalid seed
        // must still yield -1.
        for &(s, m, t) in &[
            (0, 0, 0),
            (7, 1, i32::MAX),
            (65535, 2, i32::MIN),
            (-1, -1, 0),
            (65536, 99, 12345),
            (i32::MIN, i32::MIN, i32::MIN),
            (i32::MAX, i32::MAX, i32::MAX),
        ] {
            let r = assert_goto_eq(&cf, &rf, "E1", it, s, m, t);
            assert_eq!(
                r, -1,
                "[E1] expected -1 for iterations={it} (seed={s}, mode={m}, threshold={t}), got {r}"
            );
        }
    }
}

#[test]
fn e2_iterations_above_uint16_max_returns_minus_1() {
    let libs = Pair::load();
    let (cf, rf) = libs.gotomach();
    let mut rng = Rng::new(0xE002);

    let mut cases: Vec<i32> = vec![65536, 65537, 70000, 100000, i32::MAX - 1, i32::MAX];
    for _ in 0..300 {
        cases.push(rng.i32_in(65536, i32::MAX));
    }

    for &it in &cases {
        for &(s, m, t) in &[(0, 0, 0), (7, 1, i32::MAX), (-5, 3, i32::MIN)] {
            let r = assert_goto_eq(&cf, &rf, "E2", it, s, m, t);
            assert_eq!(r, -1, "[E2] expected -1 for iterations={it}, got {r}");
        }
    }

    // One step either side of the boundary.
    for &(it, want) in &[(65535i32, None), (65536i32, Some(-1))] {
        let r = assert_goto_eq(&cf, &rf, "E2", it, 0, 0, i32::MIN);
        match want {
            Some(code) => assert_eq!(r, code, "[E2] iterations={it} should be rejected"),
            None => assert_eq!(r, 0, "[E2] iterations={it} should be accepted"),
        }
    }
}

// ---------------------------------------------------------------------------
// E3 / E4 — invalid `seed` => -2
// ---------------------------------------------------------------------------

#[test]
fn e3_seed_negative_returns_minus_2() {
    let libs = Pair::load();
    let (cf, rf) = libs.gotomach();
    let mut rng = Rng::new(0xE003);

    let mut cases: Vec<i32> = vec![-1, -2, -1000, -65535, -65536, i32::MIN, i32::MIN + 1];
    for _ in 0..300 {
        cases.push(rng.i32_in(i32::MIN, -1));
    }

    for &s in &cases {
        for &it in &[0, 1, 2, 100, 65535] {
            for &m in &[0, 1, 2, -1, 7] {
                let r = assert_goto_eq(&cf, &rf, "E3", it, s, m, i32::MAX);
                assert_eq!(
                    r, -2,
                    "[E3] expected -2 for seed={s} (iterations={it}, mode={m}), got {r}"
                );
            }
        }
    }
}

#[test]
fn e4_seed_above_uint16_max_returns_minus_2() {
    let libs = Pair::load();
    let (cf, rf) = libs.gotomach();
    let mut rng = Rng::new(0xE004);

    let mut cases: Vec<i32> = vec![65536, 65537, 100000, i32::MAX - 1, i32::MAX];
    for _ in 0..300 {
        cases.push(rng.i32_in(65536, i32::MAX));
    }

    for &s in &cases {
        for &it in &[0, 1, 65535] {
            let r = assert_goto_eq(&cf, &rf, "E4", it, s, 0, i32::MAX);
            assert_eq!(r, -2, "[E4] expected -2 for seed={s}, got {r}");
        }
    }

    // One step either side of the boundary.
    let r = assert_goto_eq(&cf, &rf, "E4", 4, 65535, 0, i32::MIN);
    assert_eq!(r, 0, "[E4] seed=65535 must be accepted");
    let r = assert_goto_eq(&cf, &rf, "E4", 4, 65536, 0, i32::MIN);
    assert_eq!(r, -2, "[E4] seed=65536 must be rejected");
}

#[test]
fn e1_e3_precedence_iterations_checked_before_seed() {
    let libs = Pair::load();
    let (cf, rf) = libs.gotomach();
    // Both invalid: the C source tests `iterations` first, so the result is -1.
    for &(it, s) in &[
        (-1, -1),
        (-1, 65536),
        (65536, -1),
        (65536, 65536),
        (i32::MIN, i32::MIN),
        (i32::MAX, i32::MAX),
    ] {
        let r = assert_goto_eq(&cf, &rf, "E1/E3", it, s, 0, 0);
        assert_eq!(
            r, -1,
            "[E1/E3] iterations must be validated first: iterations={it}, seed={s} gave {r}"
        );
    }
}

// ---------------------------------------------------------------------------
// E8 / E9 — statically unreachable rejections (-5 and -6).
//
// Both branches exist in both shared objects (verified by the presence of their
// log strings, see tests/phase_d_symbols.rs) but can never be taken:
//   * `init_processor` always sets `status = 1`, so `check_char_flag` is true.
//   * `count` grows at most once per iteration, so `count <= i < capacity`.
// The differential requirement is therefore that NEITHER library ever produces
// -5 or -6, across a wide sweep of the whole accepted input domain.
// ---------------------------------------------------------------------------

#[test]
fn e8_e9_minus5_and_minus6_never_observed_in_either_library() {
    let libs = Pair::load();
    let (cf, rf) = libs.gotomach();
    let mut rng = Rng::new(0xE089);

    let mut check = |it: i32, s: i32, m: i32, t: i32| {
        let r = assert_goto_eq(&cf, &rf, "E8/E9", it, s, m, t);
        assert!(
            r != -5 && r != -6,
            "[E8/E9] unexpected {r} for (iterations={it}, seed={s}, mode={m}, threshold={t})"
        );
    };

    // Deterministic sweep of the shapes most likely to trip the count ceiling.
    for &m in &MODES {
        for &it in &[0, 1, 2, 3, 65533, 65534, 65535] {
            for &s in &[0, 1, 999, 1000, 65535] {
                for &t in &[i32::MIN, 0, 3000, i32::MAX] {
                    check(it, s, m, t);
                }
            }
        }
    }

    // Randomized sweep of the accepted domain.
    for _ in 0..600 {
        let m = if rng.next_u64() % 4 == 0 {
            rng.invalid_mode()
        } else {
            MODES[(rng.next_u64() % 3) as usize]
        };
        let it = if rng.next_u64() % 8 == 0 {
            rng.i32_in(0, 65535)
        } else {
            rng.i32_in(0, 256)
        };
        check(it, rng.i32_in(0, 65535), m, rng.i32_any());
    }
}

// ---------------------------------------------------------------------------
// E10 — `mode` outside {0,1,2}: the `switch` default. Soft rejection: logs a
// warning and falls back to `process_value`, so the result must equal mode 0.
// This is the out-of-range-enum-over-FFI case.
// ---------------------------------------------------------------------------

#[test]
fn e10_out_of_range_mode_falls_back_to_process_value() {
    let libs = Pair::load();
    let (cf, rf) = libs.gotomach();
    let mut rng = Rng::new(0xE010);

    let mut modes: Vec<i32> = vec![
        -1,
        -2,
        3,
        4,
        5,
        100,
        255,
        256,
        65535,
        65536,
        i32::MIN,
        i32::MIN + 1,
        i32::MAX,
        i32::MAX - 1,
        // Values that would alias to 0/1/2 only under a narrowing cast — they
        // must NOT be treated as valid modes.
        0x1_0000,
        0x1_0001,
        0x1_0002,
        -0x1_0000,
    ];
    for _ in 0..300 {
        modes.push(rng.invalid_mode());
    }

    for &m in &modes {
        for &(it, s, t) in &[
            (0, 0, 0),
            (1, 7, i32::MAX),
            (5, 999, 1000),
            (64, 65535, 2000),
            (257, 1, i32::MIN),
        ] {
            let r = assert_goto_eq(&cf, &rf, "E10", it, s, m, t);
            let baseline = assert_goto_eq(&cf, &rf, "E10", it, s, 0, t);
            assert_eq!(
                r, baseline,
                "[E10] invalid mode {m} must behave like mode 0 for \
                 (iterations={it}, seed={s}, threshold={t}): got {r}, mode 0 gives {baseline}"
            );
            // It is a *warning*, not an error: never an error sentinel unless
            // the inputs themselves are invalid.
            assert!(
                !(-6..=-1).contains(&r),
                "[E10] invalid mode {m} must not produce an error code, got {r}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// E11 — `state->count >= UINT16_MAX` early break.
// ---------------------------------------------------------------------------

#[test]
fn e11_count_ceiling_break_matches() {
    let libs = Pair::load();
    let (cf, rf) = libs.gotomach();
    let mut rng = Rng::new(0xE011);

    for &m in &MODES {
        for &s in &[0, 1, 2, 999, 1000, 65535] {
            let r = assert_goto_eq(&cf, &rf, "E11", 65535, s, m, i32::MAX);
            // Not an error code: the ceiling break still falls through to the
            // summation and returns the sum.
            assert!(
                !(-6..=-1).contains(&r),
                "[E11] ceiling break must not return an error code, got {r}"
            );
        }
        for _ in 0..16 {
            assert_goto_eq(&cf, &rf, "E11", 65535, rng.i32_in(0, 65535), m, i32::MAX);
        }
    }
    for _ in 0..16 {
        let m = rng.invalid_mode();
        assert_goto_eq(&cf, &rf, "E11", 65535, rng.i32_in(0, 65535), m, i32::MAX);
    }
}

// ---------------------------------------------------------------------------
// E12 / B7 — the op functions must never touch `unused_param` or
// `unused_context`, including a null and a garbage non-null pointer.
// ---------------------------------------------------------------------------

#[test]
fn e12_ops_ignore_unused_param_and_context() {
    let libs = Pair::load();
    let mut rng = Rng::new(0xE012);
    let contexts: [*mut c_void; 5] = [
        std::ptr::null_mut(),
        1usize as *mut c_void,
        usize::MAX as *mut c_void,
        0xdead_beef_usize as *mut c_void,
        0x7fff_ffff_ffff_ffffusize as *mut c_void,
    ];

    for name in OP_NAMES {
        let (cf, rf) = libs.op(name);
        let n = std::str::from_utf8(name).unwrap();
        for _ in 0..200 {
            let v = rng.i32_any();
            let mut baseline: Option<i32> = None;
            for &ctx in &contexts {
                for &p in &[i32::MIN, -1, 0, 1, i32::MAX, rng.i32_any()] {
                    let r = assert_op_eq(&cf, &rf, "E12", n, v, p, ctx);
                    match baseline {
                        None => baseline = Some(r),
                        Some(b) => assert_eq!(
                            r, b,
                            "[E12] {n}(value={v}) changed with unused_param={p}, ctx={ctx:p}"
                        ),
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// B1..B6 — generic boundaries required by Phase C.
// ---------------------------------------------------------------------------

#[test]
fn b1_iterations_zero_is_accepted_and_returns_zero() {
    let libs = Pair::load();
    let (cf, rf) = libs.gotomach();
    let mut rng = Rng::new(0xB001);
    // malloc(0) is called twice; glibc returns a unique non-NULL pointer, so
    // this must NOT be an allocation failure.
    for &m in &[0, 1, 2, -1, 3, i32::MIN, i32::MAX] {
        for &s in &[0, 1, 65535] {
            for &t in &[i32::MIN, 0, i32::MAX] {
                let r = assert_goto_eq(&cf, &rf, "B1", 0, s, m, t);
                assert_eq!(r, 0, "[B1] iterations=0 must return 0, got {r}");
            }
        }
    }
    for _ in 0..200 {
        let r = assert_goto_eq(
            &cf,
            &rf,
            "B1",
            0,
            rng.i32_in(0, 65535),
            rng.i32_any(),
            rng.i32_any(),
        );
        assert_eq!(r, 0, "[B1] iterations=0 must return 0, got {r}");
    }
}

#[test]
fn b2_b3_one_past_range_pairs() {
    let libs = Pair::load();
    let (cf, rf) = libs.gotomach();

    // iterations: last valid vs first invalid.
    assert_ne!(
        assert_goto_eq(&cf, &rf, "B2", 65535, 0, 0, i32::MIN),
        -1,
        "[B2] iterations=65535 must be accepted"
    );
    assert_eq!(
        assert_goto_eq(&cf, &rf, "B2", 65536, 0, 0, i32::MIN),
        -1,
        "[B2] iterations=65536 must be rejected"
    );
    assert_eq!(
        assert_goto_eq(&cf, &rf, "B2", -1, 0, 0, i32::MIN),
        -1,
        "[B2] iterations=-1 must be rejected"
    );
    assert_eq!(
        assert_goto_eq(&cf, &rf, "B2", 0, 0, 0, i32::MIN),
        0,
        "[B2] iterations=0 must be accepted"
    );

    // seed: last valid vs first invalid.
    for &it in &[0, 1, 64] {
        assert_ne!(
            assert_goto_eq(&cf, &rf, "B3", it, 65535, 0, i32::MIN),
            -2,
            "[B3] seed=65535 must be accepted"
        );
        assert_eq!(
            assert_goto_eq(&cf, &rf, "B3", it, 65536, 0, i32::MIN),
            -2,
            "[B3] seed=65536 must be rejected"
        );
        assert_eq!(
            assert_goto_eq(&cf, &rf, "B3", it, -1, 0, i32::MIN),
            -2,
            "[B3] seed=-1 must be rejected"
        );
        assert_ne!(
            assert_goto_eq(&cf, &rf, "B3", it, 0, 0, i32::MIN),
            -2,
            "[B3] seed=0 must be accepted"
        );
    }
}

#[test]
fn b4_b5_b6_extreme_arguments() {
    let libs = Pair::load();
    let (cf, rf) = libs.gotomach();
    let extremes = [i32::MIN, i32::MIN + 1, -1, 0, 1, 65535, 65536, i32::MAX - 1, i32::MAX];

    // Full cross-product of extremes over all four parameters (9^4 = 6561).
    for &it in &extremes {
        for &s in &extremes {
            for &m in &extremes {
                for &t in &extremes {
                    assert_goto_eq(&cf, &rf, "B4/B5/B6", it, s, m, t);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// B8 / B9 — signed-overflow boundaries of the three op functions.
// ---------------------------------------------------------------------------

#[test]
fn b8_b9_op_overflow_boundaries() {
    let libs = Pair::load();
    let cases: &[(&[u8], &[i32])] = &[
        (
            b"process_value",
            &[
                i32::MAX,
                i32::MAX - 1,
                i32::MAX - 9,
                i32::MAX - 10,
                i32::MAX - 11,
                i32::MIN,
                i32::MIN + 9,
                i32::MIN + 10,
                -10,
                -9,
                0,
            ],
        ),
        (
            b"double_value",
            &[
                i32::MAX,
                i32::MAX - 1,
                i32::MAX / 2,
                i32::MAX / 2 + 1,
                i32::MIN,
                i32::MIN + 1,
                i32::MIN / 2,
                i32::MIN / 2 - 1,
                0,
                1,
                -1,
            ],
        ),
        (
            b"triple_value",
            &[
                i32::MAX,
                i32::MAX - 1,
                i32::MAX / 3,
                i32::MAX / 3 + 1,
                i32::MAX / 3 + 2,
                i32::MIN,
                i32::MIN + 1,
                i32::MIN / 3,
                i32::MIN / 3 - 1,
                0,
                1,
                -1,
            ],
        ),
    ];

    for &(name, values) in cases {
        let (cf, rf) = libs.op(name);
        let n = std::str::from_utf8(name).unwrap();
        for &v in values {
            assert_op_eq(&cf, &rf, "B8/B9", n, v, 0, std::ptr::null_mut());
        }
    }
}
