//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md`. `driver` returns `void`, so the error
//! channel is the exact byte stream on stdout: the specific `Error: …` line,
//! the `Operation failed` line, and the `Result: <code>` code. Each row asserts
//! the two `.so`s agree AND that they produce the specific code from the C
//! source — not merely "both failed somehow".

mod common;

use common::{Rng, SEED, assert_same, assert_same_sequence, expected};

/// Extract the trailing `Result: <n>` code — the internal `multi_stage` return
/// value, which is the only error code this API surfaces.
fn result_code(out: &[u8]) -> i32 {
    let s = String::from_utf8_lossy(out);
    let line = s
        .lines()
        .rev()
        .find(|l| l.starts_with("Result: "))
        .unwrap_or_else(|| panic!("no `Result:` line in output {s:?}"));
    line["Result: ".len()..]
        .trim()
        .parse()
        .unwrap_or_else(|_| panic!("unparsable result line {line:?}"))
}

/// ERRORS.md row 1 — `x != 1`: first guard, code 1, `y`/`z` never examined.
#[test]
fn err_row1_x_not_1() {
    let mut rng = Rng::new(SEED ^ 0x101);
    for _ in 0..256 {
        let x = rng.interesting_int_except(1);
        // y and z are deliberately arbitrary: the first guard must short-circuit.
        let (y, z) = (rng.interesting_int(), rng.interesting_int());
        let out = assert_same(x, y, z);
        let s = String::from_utf8_lossy(&out);
        assert_eq!(result_code(&out), 1, "driver({x}, {y}, {z})");
        assert_eq!(
            s,
            format!(
                "{}{}{}",
                expected::ERR_X,
                expected::FAILED,
                expected::result_line(1)
            )
        );
        assert!(!s.contains("but y != 2"), "y guard must not run");
        assert!(!s.contains("but z != 3"), "z guard must not run");
        assert!(!s.contains(expected::OK));
    }
}

/// ERRORS.md row 2 — `x == 1 && y != 2`: second guard, code 2, `z` never examined.
#[test]
fn err_row2_y_not_2() {
    let mut rng = Rng::new(SEED ^ 0x102);
    for _ in 0..256 {
        let y = rng.interesting_int_except(2);
        let z = rng.interesting_int();
        let out = assert_same(1, y, z);
        let s = String::from_utf8_lossy(&out);
        assert_eq!(result_code(&out), 2, "driver(1, {y}, {z})");
        assert_eq!(
            s,
            format!(
                "{}{}{}",
                expected::ERR_Y,
                expected::FAILED,
                expected::result_line(2)
            )
        );
        assert!(!s.contains("Error: x != 1"), "x guard must have passed");
        assert!(!s.contains("but z != 3"), "z guard must not run");
    }
}

/// ERRORS.md row 3 — `x == 1 && y == 2 && z != 3`: third guard, code 3.
#[test]
fn err_row3_z_not_3() {
    let mut rng = Rng::new(SEED ^ 0x103);
    for _ in 0..256 {
        let z = rng.interesting_int_except(3);
        let out = assert_same(1, 2, z);
        let s = String::from_utf8_lossy(&out);
        assert_eq!(result_code(&out), 3, "driver(1, 2, {z})");
        assert_eq!(
            s,
            format!(
                "{}{}{}",
                expected::ERR_Z,
                expected::FAILED,
                expected::result_line(3)
            )
        );
        assert!(!s.contains("Error: x != 1"));
        assert!(!s.contains("but y != 2"));
    }
}

/// ERRORS.md row 4 — the `fail:` label. `Operation failed` must appear exactly
/// when the code is non-zero, and never on the success path (the C `return`s
/// before the label).
#[test]
fn err_row4_fail_label_only_on_failure() {
    let mut rng = Rng::new(SEED ^ 0x104);
    let mut seen_success = false;
    let mut seen_failure = false;

    for _ in 0..1024 {
        let (x, y, z) = if rng.next_u64() % 5 == 0 {
            (1, 2, 3)
        } else {
            (
                rng.interesting_int(),
                rng.interesting_int(),
                rng.interesting_int(),
            )
        };
        let out = assert_same(x, y, z);
        let s = String::from_utf8_lossy(&out);
        let code = result_code(&out);
        let has_failed_line = s.contains(expected::FAILED);
        assert_eq!(
            has_failed_line,
            code != 0,
            "`Operation failed` presence must track code != 0; driver({x}, {y}, {z}) -> {s:?}"
        );
        assert_eq!(
            s.contains(expected::OK),
            code == 0,
            "`Ok!` presence must track code == 0"
        );
        if code == 0 {
            seen_success = true;
        } else {
            seen_failure = true;
        }
    }
    assert!(seen_success && seen_failure, "row 4 covered both outcomes");
}

/// ERRORS.md rows 5 and 6 — guard precedence when several conditions are
/// simultaneously invalid. Only the first failing guard may report.
#[test]
fn err_row5_guard_precedence() {
    // Row 5: all three invalid -> code 1 only.
    let mut rng = Rng::new(SEED ^ 0x105);
    for _ in 0..256 {
        let x = rng.interesting_int_except(1);
        let y = rng.interesting_int_except(2);
        let z = rng.interesting_int_except(3);
        let out = assert_same(x, y, z);
        let s = String::from_utf8_lossy(&out);
        assert_eq!(result_code(&out), 1, "x guard has highest precedence");
        assert!(s.contains(expected::ERR_X));
        assert!(!s.contains("but y != 2"));
        assert!(!s.contains("but z != 3"));
    }

    // Row 6: x ok, y and z invalid -> code 2 only.
    for _ in 0..256 {
        let y = rng.interesting_int_except(2);
        let z = rng.interesting_int_except(3);
        let out = assert_same(1, y, z);
        let s = String::from_utf8_lossy(&out);
        assert_eq!(result_code(&out), 2, "y guard outranks z guard");
        assert!(s.contains(expected::ERR_Y));
        assert!(!s.contains("but z != 3"));
    }
}

/// ERRORS.md row 7 — `INT_MIN` / `INT_MAX` in each parameter slot.
#[test]
fn err_boundary_int_extremes() {
    let extremes = [i32::MIN, i32::MAX];
    for &v in &extremes {
        // In the x slot -> row 1.
        assert_eq!(result_code(&assert_same(v, 2, 3)), 1);
        assert_eq!(result_code(&assert_same(v, v, v)), 1);
        // In the y slot (x valid) -> row 2.
        assert_eq!(result_code(&assert_same(1, v, 3)), 2);
        assert_eq!(result_code(&assert_same(1, v, v)), 2);
        // In the z slot (x, y valid) -> row 3.
        assert_eq!(result_code(&assert_same(1, 2, v)), 3);
    }
    // Both extremes together, in every slot arrangement.
    for &a in &extremes {
        for &b in &extremes {
            for &c in &extremes {
                let out = assert_same(a, b, c);
                assert_eq!(
                    String::from_utf8_lossy(&out),
                    expected::transcript(a, b, c),
                    "driver({a}, {b}, {c})"
                );
            }
        }
    }
}

/// ERRORS.md rows 8 and 9 — one step past each magic constant, plus zero.
#[test]
fn err_boundary_off_by_one() {
    // Row 8: x one step either side of 1.
    for x in [0, 2] {
        assert_eq!(result_code(&assert_same(x, 2, 3)), 1, "x = {x}");
    }
    // Row 8: y one step either side of 2.
    for y in [1, 3] {
        assert_eq!(result_code(&assert_same(1, y, 3)), 2, "y = {y}");
    }
    // Row 8: z one step either side of 3.
    for z in [2, 4] {
        assert_eq!(result_code(&assert_same(1, 2, z)), 3, "z = {z}");
    }
    // Row 9: zero in each slot.
    assert_eq!(result_code(&assert_same(0, 2, 3)), 1);
    assert_eq!(result_code(&assert_same(1, 0, 3)), 2);
    assert_eq!(result_code(&assert_same(1, 2, 0)), 3);
    assert_eq!(result_code(&assert_same(0, 0, 0)), 1);
    // The exact "valid" values are the only ones that pass.
    assert_eq!(result_code(&assert_same(1, 2, 3)), 0);
}

/// ERRORS.md row 10 — out-of-range "enum-like" ints across the FFI boundary.
/// C `int` parameters accept any bit pattern, so values with no meaningful
/// variant are real inputs that both implementations must handle identically.
#[test]
fn err_out_of_range_enum_values() {
    // Values chosen to have no valid meaning for x/y/z, including the ones a
    // C enum would never define, and the sign-boundary bit patterns.
    const NONSENSE: [i32; 12] = [
        -1,
        4,
        5,
        -2,
        100,
        -100,
        i32::MAX,
        i32::MIN,
        0x7FFF_FFFE,
        -0x7FFF_FFFF,
        0x0001_0000,
        i32::MIN + 1, // == -0x7fffffff, the negation-overflow neighbour
    ];

    for &v in &NONSENSE {
        for slot in 0..3 {
            let (x, y, z) = match slot {
                0 => (v, 2, 3),
                1 => (1, v, 3),
                _ => (1, 2, v),
            };
            let out = assert_same(x, y, z);
            assert_eq!(
                String::from_utf8_lossy(&out),
                expected::transcript(x, y, z),
                "out-of-range value {v} in slot {slot}"
            );
            assert_ne!(result_code(&out), 0, "{v} must never be accepted");
        }
    }

    // Every nonsense value in every slot simultaneously.
    for &a in &NONSENSE {
        for &b in &NONSENSE {
            let out = assert_same(a, b, a);
            assert_eq!(String::from_utf8_lossy(&out), expected::transcript(a, b, a));
        }
    }
}

/// ERRORS.md rows 11 and 12 — documented as not applicable, asserted
/// structurally so the claim cannot silently rot: the exported signature is
/// `void driver(int, int, int)`, which has no pointer and no length parameter,
/// so there is no null-pointer or zero/oversized-length input to construct.
#[test]
fn err_rows_11_12_not_applicable_by_signature() {
    let header = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../c_src/include/driver.h"
    ))
    .expect("read driver.h");
    assert!(
        header.contains("void driver(int x, int y, int z);"),
        "driver.h signature changed — re-evaluate ERRORS.md rows 11/12"
    );
    let decls: Vec<&str> = header
        .lines()
        .filter(|l| l.contains('(') && l.trim_end().ends_with(");"))
        .collect();
    assert_eq!(decls.len(), 1, "public API is a single function: {decls:?}");
    assert!(
        !decls[0].contains('*'),
        "a pointer parameter appeared; null-pointer rows are now required"
    );
    assert!(
        !decls[0].contains("size_t") && !decls[0].contains("len"),
        "a length parameter appeared; length rows are now required"
    );
}

/// ERRORS.md row 13 — a failing call still commits `y = local_y` before the
/// guards run, so the mutation must survive into the next call identically in
/// both implementations.
#[test]
fn err_state_persists_after_failure() {
    // A failing call writes y, then the next call writes it again — so the
    // observable behaviour is that `local_y` of the *current* call always wins.
    assert_same_sequence(&[
        (0, 999, 0),   // fails at the x guard, but y := 999 is committed
        (1, 2, 3),     // must still succeed: y := 2 overwrites 999
        (1, 999, 3),   // fails at the y guard, y := 999
        (1, 2, 3),     // succeeds again
        (1, 2, 0),     // fails at the z guard, y := 2
        (1, 2, 3),     // succeeds
        (i32::MIN, i32::MIN, i32::MIN),
        (1, 2, 3),
    ]);

    // And the same idea driven randomly: every failing configuration followed
    // by the success triple must still succeed.
    let mut rng = Rng::new(SEED ^ 0x113);
    for _ in 0..256 {
        let bad = (
            rng.interesting_int_except(1),
            rng.interesting_int(),
            rng.interesting_int(),
        );
        assert_same_sequence(&[bad, (1, 2, 3)]);
        let out = assert_same(1, 2, 3);
        assert_eq!(result_code(&out), 0, "state leaked from {bad:?}");
    }
}
