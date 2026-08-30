//! Differential tests: every call goes through the exported `driver` symbol of
//! the C `.so` and of the Rust `.so`, loaded with `libloading`. Nothing in the
//! Rust crate is called directly.
//!
//! Phase B rows come from `CONFIGS.md`, Phase C rows from `ERRORS.md`.

mod common;

use common::*;

// ---------------------------------------------------------------------------
// Phase B — valid-path rows from CONFIGS.md
// ---------------------------------------------------------------------------

/// Row 1: loop guard false on entry (`x <= 0 && y <= 0`) — empty output.
#[test]
fn configs_row01_guard_false_both_nonpositive() {
    let mut rng = Rng::new(1);
    for _ in 0..128 {
        let x = rng.range(-3000, 0);
        let y = rng.range(-3000, 0);
        assert_same("row01", x, y);
        // And the behaviour itself: nothing at all is printed.
        assert!(c_output(x, y).is_empty(), "row01: C printed for ({x},{y})");
        assert!(rust_output(x, y).is_empty(), "row01: Rust printed for ({x},{y})");
    }
    for &(x, y) in &[
        (0, 0),
        (i32::MIN, 0),
        (0, i32::MIN),
        (i32::MIN, i32::MIN),
        (-1, -1),
        (i32::MIN, -1),
        (-1, i32::MIN),
    ] {
        assert_same("row01-corners", x, y);
    }
}

/// Row 2: `y == 0, x > 0` — `if (y == 0) continue;` taken every iteration.
#[test]
fn configs_row02_y_zero_x_positive() {
    let mut rng = Rng::new(2);
    for _ in 0..96 {
        let x = rng.range(1, 3000);
        assert_same("row02", x, 0);
    }
    for x in 1..=12 {
        assert_same("row02-small", x, 0);
    }
}

/// Row 3: `x == 0, y > 0` — the `x > 0` block never runs; `x < 3` always true.
#[test]
fn configs_row03_x_zero_y_positive() {
    let mut rng = Rng::new(3);
    for _ in 0..96 {
        let y = rng.range(1, 3000);
        assert_same("row03", 0, y);
    }
    for y in 1..=12 {
        assert_same("row03-small", 0, y);
    }
}

/// Row 4: negative `x`, positive `y` — `x` is never decremented.
#[test]
fn configs_row04_x_negative_y_positive() {
    let mut rng = Rng::new(4);
    for _ in 0..96 {
        let x = rng.range(-3000, -1);
        let y = rng.range(1, 3000);
        assert_same("row04", x, y);
    }
    assert_same("row04-extreme", i32::MIN, 7);
}

/// Row 5: the unique `goto label2` input.
#[test]
fn configs_row05_special_skip_1_4() {
    assert_same("row05", 1, 4);

    // The skip must actually change the byte stream relative to its neighbours,
    // otherwise this row would be vacuous.
    let out = c_output(1, 4);
    assert_eq!(
        out, rust_output(1, 4),
        "row05: C/Rust differ on the goto label2 input"
    );
    assert!(
        out.starts_with(b"loop\ny\n"),
        "row05: expected label1 to be skipped, got {:?}",
        String::from_utf8_lossy(&out)
    );
    assert!(
        c_output(1, 5).starts_with(b"loop\nx\n"),
        "row05: sanity — (1,5) must not skip label1"
    );
}

/// Row 6: `x == 1` with `y != 4`.
#[test]
fn configs_row06_special_near_miss_on_y() {
    for y in [0, 1, 2, 3, 5, 6, 7, 8] {
        assert_same("row06", 1, y);
    }
}

/// Row 7: `y == 4` with `x != 1`.
#[test]
fn configs_row07_special_near_miss_on_x() {
    for x in [-1, 0, 2, 3, 4, 5] {
        assert_same("row07", x, 4);
    }
}

/// Row 8: `x == 2` — `x < 3` back-goto with a decrementing `x`.
#[test]
fn configs_row08_x_two_back_goto() {
    let mut rng = Rng::new(8);
    for _ in 0..64 {
        let y = rng.range(1, 3000);
        assert_same("row08", 2, y);
    }
}

/// Row 9: `x == 3` boundary of `x < 3` — falls through to the loop guard.
#[test]
fn configs_row09_x_three_boundary() {
    let mut rng = Rng::new(9);
    for _ in 0..64 {
        let y = rng.range(1, 3000);
        assert_same("row09", 3, y);
    }
    for y in 1..=10 {
        assert_same("row09-small", 3, y);
        assert_same("row09-small", 4, y);
    }
}

/// Row 10: `x ∈ [4,60]`, `y ∈ [1,60]` — mixes fall-through and back-goto.
#[test]
fn configs_row10_mixed_back_goto() {
    let mut rng = Rng::new(10);
    for _ in 0..256 {
        let x = rng.range(4, 60);
        let y = rng.range(1, 60);
        assert_same("row10", x, y);
    }
}

/// Row 11: large `x`, small `y`.
#[test]
fn configs_row11_large_x_small_y() {
    let mut rng = Rng::new(11);
    for _ in 0..24 {
        let x = rng.range(500, 20000);
        let y = rng.range(0, 5);
        assert_same("row11", x, y);
    }
}

/// Row 12: small `x`, large `y`.
#[test]
fn configs_row12_small_x_large_y() {
    let mut rng = Rng::new(12);
    for _ in 0..24 {
        let x = rng.range(0, 5);
        let y = rng.range(500, 20000);
        assert_same("row12", x, y);
    }
}

/// Row 13: both arguments large.
#[test]
fn configs_row13_both_large() {
    let mut rng = Rng::new(13);
    for _ in 0..24 {
        let x = rng.range(200, 4000);
        let y = rng.range(200, 4000);
        assert_same("row13", x, y);
    }
}

/// Row 14: the three ordering relations between `x` and `y`.
#[test]
fn configs_row14_ordering_relations() {
    let mut rng = Rng::new(14);
    for _ in 0..64 {
        let x = rng.range(1, 400);
        // y > x
        assert_same("row14-gt", x, x + rng.range(1, 400));
        // y == x
        assert_same("row14-eq", x, x);
        // y < x
        assert_same("row14-lt", x, rng.range(0, x as i64 - 1).max(0));
    }
}

/// Row 15: exhaustive small grid, skipping only the C-side UB region.
#[test]
fn configs_row15_exhaustive_small_grid() {
    let mut checked = 0usize;
    for x in -4..=12 {
        for y in -4..=12 {
            if is_excluded(x, y) {
                continue;
            }
            assert_same("row15", x, y);
            checked += 1;
        }
    }
    assert_eq!(checked, 17 * 17 - 12 * 4, "row15: unexpected case count");
}

/// Row 15b: near-exhaustive medium grid — every `(x, y) ∈ [-6,40]²` outside the
/// C-side UB region. This covers every interaction of the guard, the `x == 1 &&
/// y == 4` skip, the `y == 0` continue and the `x < 3` back-goto for small
/// magnitudes, rather than sampling them.
#[test]
fn configs_row15b_exhaustive_medium_grid() {
    let mut checked = 0usize;
    for x in -6..=40 {
        for y in -6..=40 {
            if is_excluded(x, y) {
                continue;
            }
            assert_same("row15b", x, y);
            checked += 1;
        }
    }
    assert_eq!(checked, 47 * 47 - 40 * 6, "row15b: unexpected case count");
}

/// Row 16: broad randomized sweep.
#[test]
fn configs_row16_broad_random_sweep() {
    let mut rng = Rng::new(16);
    let mut done = 0;
    while done < 768 {
        let x = rng.range(-2000, 2000);
        let y = rng.range(-2000, 2000);
        if is_excluded(x, y) {
            continue;
        }
        assert_same("row16", x, y);
        done += 1;
    }
}

/// Row 17: integer extremes / arbitrary bit patterns.
///
/// `INT_MAX` (or any large positive value) can only be *observed* when it is the
/// argument that is never iterated over: as `x` it needs `INT_MAX` iterations,
/// and as `y` it needs `INT_MAX` `"y\n"` lines, so those two combinations are
/// `ERRORS.md` row 15 (intractable) and are skipped, not asserted.
#[test]
fn configs_row17_integer_extremes() {
    // INT_MIN as x: guard depends solely on y, and x is never decremented.
    for y in 0..=10 {
        assert_same("row17-intmin-x", i32::MIN, y);
    }
    // INT_MIN as y: guard depends solely on x; y<0 forces x<=0 to stay defined.
    for x in [-1, 0, i32::MIN, -12345] {
        assert_same("row17-intmin-y", x, i32::MIN);
    }
    // INT_MAX paired with a positive partner is intractable, so assert only that
    // the harness refuses it rather than executing it.
    assert!(is_excluded(i32::MAX, 0));
    assert!(is_excluded(0, i32::MAX));
    assert!(is_ub(i32::MAX, i32::MIN));

    let patterns: &[i32] = &[
        0x5A5A_5A5Au32 as i32,
        0xFFFF_FFFFu32 as i32, // -1
        0x8000_0000u32 as i32, // INT_MIN
        0x7FFF_FFFFu32 as i32, // INT_MAX
        0xDEAD_BEEFu32 as i32,
        0x0000_0001,
        0x0000_0004,
    ];
    let mut executed = 0;
    let mut skipped = 0;
    for &v in patterns {
        for &partner in &[-1i32, 0, 1, 4, 7] {
            for &(x, y) in &[(v, partner), (partner, v)] {
                if is_excluded(x, y) {
                    skipped += 1;
                    continue;
                }
                assert_same("row17-pattern", x, y);
                executed += 1;
            }
        }
    }
    assert!(executed > 0 && skipped > 0, "row17: filter sanity check");
}

/// Row 18: statelessness across repeated invocations in one capture.
#[test]
fn configs_row18_repeated_invocations() {
    for stream in [18u64, 1800] {
        let mut rng = Rng::new(stream);
        let mut calls = Vec::new();
        while calls.len() < 40 {
            let x = rng.range(-6, 40);
            let y = rng.range(-6, 40);
            if is_excluded(x, y) {
                continue;
            }
            calls.push((x, y));
        }
        assert_same_sequence("row18", &calls);
    }
}

/// Row 19: interleaved C/Rust calls — neither library may perturb the other's
/// use of the shared `stdout` buffer.
#[test]
fn configs_row19_interleaved_libraries() {
    let cf = c_driver();
    let rf = rust_driver();
    let cases: &[(i32, i32)] = &[(1, 4), (3, 3), (0, 5), (7, 0), (2, 9), (-3, 4), (0, 0), (5, 1)];

    for &(x, y) in cases {
        let interleaved = capture_stdout(|| unsafe {
            cf(x, y);
            rf(x, y);
            cf(x, y);
            rf(x, y);
        });
        let single = c_output(x, y);
        let mut expected = Vec::new();
        for _ in 0..4 {
            expected.extend_from_slice(&single);
        }
        assert_eq!(
            interleaved, expected,
            "row19: interleaved C/Rust output differs for ({x},{y})"
        );
    }
}

// ---------------------------------------------------------------------------
// Phase C — error/rejection rows from ERRORS.md
// ---------------------------------------------------------------------------

/// ERRORS rows 1-8: every "rejection" the library has — the loop guard being
/// false on entry, i.e. return with exactly zero bytes written.
#[test]
fn errors_rows01_08_guard_false_returns_silently() {
    let cases: &[(&str, i32, i32)] = &[
        ("row01 x==0 && y==0", 0, 0),
        ("row02 x<0 && y==0", -1, 0),
        ("row02 x<0 && y==0", -9999, 0),
        ("row03 x==0 && y<0", 0, -1),
        ("row03 x==0 && y<0", 0, -9999),
        ("row04 x<0 && y<0", -7, -3),
        ("row05 x==INT_MIN && y==0", i32::MIN, 0),
        ("row06 x==0 && y==INT_MIN", 0, i32::MIN),
        ("row07 both INT_MIN", i32::MIN, i32::MIN),
        ("row08 INT_MIN/-1", i32::MIN, -1),
        ("row08 -1/INT_MIN", -1, i32::MIN),
    ];
    for &(row, x, y) in cases {
        let c = c_output(x, y);
        let r = rust_output(x, y);
        assert_eq!(c, r, "[{row}] C/Rust diverge for driver({x},{y})");
        assert!(
            c.is_empty(),
            "[{row}] expected the C library to reject silently, got {:?}",
            String::from_utf8_lossy(&c)
        );
        assert!(
            r.is_empty(),
            "[{row}] expected the Rust library to reject silently, got {:?}",
            String::from_utf8_lossy(&r)
        );
    }
}

/// ERRORS row 9: `x == 0, y > 0` — the `x > 0` branch is never taken, so no
/// `"x\n"` line may ever appear.
#[test]
fn errors_row09_x_zero_never_emits_x_line() {
    let mut rng = Rng::new(109);
    for _ in 0..64 {
        let y = rng.range(1, 500);
        assert_same("errors-row09", 0, y);
        let c = c_output(0, y);
        assert!(!c.contains(&b'x'), "errors-row09: C emitted an x line for (0,{y})");
        assert_eq!(c, rust_output(0, y));
    }
}

/// ERRORS row 10: negative `x` with positive `y` — one `"loop\n"` then `y`
/// `"y\n"` lines, `x` untouched.
#[test]
fn errors_row10_negative_x_positive_y() {
    for &x in &[-1, -2, -1000, i32::MIN] {
        for y in 1..=8 {
            assert_same("errors-row10", x, y);
            let c = c_output(x, y);
            let expected: Vec<u8> = {
                let mut v = b"loop\n".to_vec();
                for _ in 0..y {
                    v.extend_from_slice(b"y\n");
                }
                v
            };
            assert_eq!(
                c, expected,
                "errors-row10: unexpected C shape for ({x},{y})"
            );
            assert_eq!(c, rust_output(x, y));
        }
    }
}

/// ERRORS row 11: `y == 0, x > 0` — `continue` on every iteration, so the
/// `y--` underflow path is never reached and output is `x`×`"loop\nx\n"`.
#[test]
fn errors_row11_y_zero_positive_x_uses_continue() {
    for x in 1..=20 {
        assert_same("errors-row11", x, 0);
        let c = c_output(x, 0);
        let expected: Vec<u8> = b"loop\nx\n".repeat(x as usize);
        assert_eq!(c, expected, "errors-row11: unexpected C shape for ({x},0)");
        assert_eq!(c, rust_output(x, 0));
    }
}

/// ERRORS row 12: the `goto label2` input is the only one that skips `label1`.
#[test]
fn errors_row12_goto_label2_only_for_1_4() {
    assert_same("errors-row12", 1, 4);
    assert!(c_output(1, 4).starts_with(b"loop\ny\n"));
    assert_eq!(c_output(1, 4), rust_output(1, 4));

    // Every other entry point into the `while` body must run `label1` first.
    for x in -2..=8 {
        for y in 0..=8 {
            if (x, y) == (1, 4) || !(x > 0 || y > 0) {
                continue;
            }
            let c = c_output(x, y);
            assert_eq!(c, rust_output(x, y), "errors-row12: diverge at ({x},{y})");
            if x > 0 {
                assert!(
                    c.starts_with(b"loop\nx\n"),
                    "errors-row12: ({x},{y}) unexpectedly skipped label1"
                );
            }
        }
    }
}

/// ERRORS row 13: values with no special meaning, including "enum-shaped"
/// out-of-range integers crossing the FFI boundary. There is no validation in
/// the C code, so both libraries must behave identically and never reject.
#[test]
fn errors_row13_arbitrary_out_of_range_values() {
    let patterns: &[i32] = &[
        i32::MIN,
        i32::MIN + 1,
        -0x5A5A_5A5A,
        -12345,
        -5,
        -2,
        -1,
        0,
        1,
        2,
        3,
        4,
        5,
        6,
        99,
        1 << 20,
        i32::MAX - 1,
        i32::MAX,
    ];
    // Cross product, restricted to combinations that terminate quickly:
    // huge magnitudes are only paired with a non-positive partner.
    for &x in patterns {
        for &y in patterns {
            // ERRORS.md row 14 (C-side UB) and row 15 (intractable output).
            if is_excluded(x, y) {
                continue;
            }
            assert_same("errors-row13", x, y);
        }
    }
}

/// The Rust side must not panic (which, with `panic = "abort"`, would kill the
/// process) anywhere the C side is well-defined. Rows 14/15 of `ERRORS.md` are
/// the only inputs excluded, and they are excluded because *C* is UB there.
#[test]
fn errors_no_panic_on_any_defined_input() {
    let mut rng = Rng::new(1415);
    for _ in 0..512 {
        let x = rng.pick(&[i32::MIN, -1000, -3, -1, 0, 1, 2, 3, 4, 30, 500]);
        let y = rng.pick(&[i32::MIN, -1000, -3, -1, 0, 1, 2, 3, 4, 30, 500]);
        if is_excluded(x, y) {
            continue;
        }
        assert_same("errors-nopanic", x, y);
    }
}
