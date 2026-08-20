//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md`. Each asserts C and Rust produce the SAME
//! error sentinel (exact value), not merely "both failed".

mod common;

use common::{assert_same, Rng, ARG_BOUNDARIES, DECIMAL_WIDTH_BOUNDARIES};

/// Sentinels from `lib.c`: `STATUS_ERROR` is `0002`.
const STATUS_ERROR: i32 = 0o2;
const ERR_MODE1_NOT_FOUND: i32 = STATUS_ERROR | 0o020; // 18
const ERR_MODE2_NOT_FOUND: i32 = STATUS_ERROR | 0o040; // 34
const ERR_MODE4_NOT_FOUND: i32 = STATUS_ERROR | 0o100; // 66
const ERR_BAD_MODE: i32 = STATUS_ERROR | 0o200; // 130

/// The translated library proper, i.e. `src/lib.rs` with the test-only
/// `shadow_probe` module stripped off. Structural checks must not be satisfied
/// (nor broken) by the probe wrappers, which exist only for the deep tests.
fn src() -> String {
    let all = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/lib.rs"))
        .expect("cannot read src/lib.rs");
    match all.find("#[cfg(feature = \"shadow_probe\")]") {
        Some(cut) => all[..cut].to_string(),
        None => all,
    }
}

/// Assert the Rust source still contains a guard that the public API cannot
/// reach, so it cannot silently drift away from the C on that path.
#[track_caller]
fn assert_guard_present(needles: &[&str], why: &str) {
    let s = src();
    let normalized: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    for n in needles {
        let nn: String = n.chars().filter(|c| !c.is_whitespace()).collect();
        assert!(
            normalized.contains(&nn),
            "structural guard missing from src/lib.rs ({why}): expected to find `{n}`"
        );
    }
}

// ---------------------------------------------------------------- row 1
#[test]
fn err_row01_default_mode_out_of_range_ints() {
    let mut rng = Rng::new(0x2001);
    // Every mode outside {1,2,3,4}, incl. out-of-range "enum" ints across FFI.
    let mut modes: Vec<i32> = vec![
        0,
        5,
        6,
        7,
        -1,
        -2,
        -3,
        -4,
        i32::MIN,
        i32::MIN + 1,
        i32::MAX,
        i32::MAX - 1,
        0x100,
        0x10000,
        0o1000,
        1 << 30,
        -(1 << 30),
        // Values whose low byte looks like a valid mode but the int does not.
        0x0000_0101,
        0x0001_0002,
        0x7fff_0003,
        -0x7fff_0004,
    ];
    modes.extend((8..40).map(|v| v as i32));
    modes.extend((-40..-4).map(|v| v as i32));

    for &m in &modes {
        for _ in 0..40 {
            let v = assert_same(m, rng.shaped_i32(), rng.shaped_i32(), rng.shaped_i32());
            assert_eq!(v, ERR_BAD_MODE, "mode {m} should hit `default:` -> 130, got {v}");
        }
    }

    // Unbiased fuzz over the whole i32 range: anything not in 1..=4 must be 130.
    for _ in 0..20_000 {
        let m = rng.i32();
        let v = assert_same(m, rng.shaped_i32(), rng.shaped_i32(), rng.shaped_i32());
        if !(1..=4).contains(&m) {
            assert_eq!(v, ERR_BAD_MODE, "mode {m} should be 130, got {v}");
        }
    }
}

// ---------------------------------------------------------------- row 2
#[test]
fn err_row02_mode1_node_not_found() {
    let mut rng = Rng::new(0x2002);
    // node_count is permanently 0, so EVERY node_id misses -> STATUS_ERROR|0020.
    for &id in &ARG_BOUNDARIES {
        for &d in &ARG_BOUNDARIES {
            let v = assert_same(1, id, d, rng.shaped_i32());
            assert_eq!(v, ERR_MODE1_NOT_FOUND, "mode 1 id={id} depth={d} -> {v}");
        }
    }
    // Ids that WOULD exist if initialize_test_data() had been called (1..=7).
    for id in -1..=8 {
        let v = assert_same(1, id, 5, 0);
        assert_eq!(
            v, ERR_MODE1_NOT_FOUND,
            "initialize_test_data is never called in C; id={id} must still miss"
        );
    }
    for _ in 0..5000 {
        let v = assert_same(1, rng.i32(), rng.i32(), rng.i32());
        assert_eq!(v, ERR_MODE1_NOT_FOUND);
    }
}

// ---------------------------------------------------------------- row 3
#[test]
fn err_row03_mode2_node_not_found() {
    let mut rng = Rng::new(0x2003);
    for &id in &ARG_BOUNDARIES {
        for &d in &ARG_BOUNDARIES {
            let v = assert_same(2, id, d, rng.shaped_i32());
            assert_eq!(v, ERR_MODE2_NOT_FOUND, "mode 2 id={id} depth={d} -> {v}");
        }
    }
    for id in -1..=8 {
        let v = assert_same(2, id, 5, 0);
        assert_eq!(v, ERR_MODE2_NOT_FOUND);
    }
    for _ in 0..5000 {
        let v = assert_same(2, rng.i32(), rng.i32(), rng.i32());
        assert_eq!(v, ERR_MODE2_NOT_FOUND);
    }
}

// ---------------------------------------------------------------- row 4
#[test]
fn err_row04_mode4_node_not_found() {
    let mut rng = Rng::new(0x2004);
    for &id in &ARG_BOUNDARIES {
        for &d in &ARG_BOUNDARIES {
            let v = assert_same(4, id, d, rng.shaped_i32());
            assert_eq!(v, ERR_MODE4_NOT_FOUND, "mode 4 id={id} depth={d} -> {v}");
        }
    }
    for id in -1..=8 {
        let v = assert_same(4, id, 5, 0);
        assert_eq!(v, ERR_MODE4_NOT_FOUND);
    }
    for _ in 0..5000 {
        let v = assert_same(4, rng.i32(), rng.i32(), rng.i32());
        assert_eq!(v, ERR_MODE4_NOT_FOUND);
    }
}

// ---------------------------------------------------------------- row 5
#[test]
fn err_row05_find_node_by_id_never_matches() {
    // find_node_by_id returns NULL for every id; observable via all three
    // callers returning their distinct sentinel for the same id.
    let mut rng = Rng::new(0x2005);
    for _ in 0..3000 {
        let id = rng.shaped_i32();
        assert_eq!(assert_same(1, id, 3, 0), ERR_MODE1_NOT_FOUND);
        assert_eq!(assert_same(2, id, 3, 0), ERR_MODE2_NOT_FOUND);
        assert_eq!(assert_same(4, id, 3, 0), ERR_MODE4_NOT_FOUND);
    }
    // The three sentinels must be distinct, proving the right branch fired.
    assert_ne!(ERR_MODE1_NOT_FOUND, ERR_MODE2_NOT_FOUND);
    assert_ne!(ERR_MODE2_NOT_FOUND, ERR_MODE4_NOT_FOUND);
    assert_ne!(ERR_MODE1_NOT_FOUND, ERR_MODE4_NOT_FOUND);
    assert_guard_present(
        &["ptr::null_mut()"],
        "find_node_by_id must return NULL when no id matches",
    );
}

// ---------------------------------------------------------------- row 6
#[test]
fn err_row06_add_node_capacity_guard() {
    assert_guard_present(
        &[
            "const MAX_NODES: usize = 100;",
            "if count as usize >= MAX_NODES {",
            "return STATUS_ERROR;",
        ],
        "row 6: add_node capacity limit",
    );
}

// ---------------------------------------------------------------- rows 7/8/9
#[test]
fn err_row07_safe_double_to_int_upper_clamp() {
    assert_guard_present(
        &["if value > 2147483647.0 {", "value = 2147483647.0;"],
        "row 7: upper clamp",
    );
}

#[test]
fn err_row08_safe_double_to_int_lower_clamp() {
    assert_guard_present(
        &["if value < -2147483648.0 {", "value = -2147483648.0;"],
        "row 8: lower clamp",
    );
}

#[test]
fn err_row09_safe_double_to_int_nan() {
    // (int)NaN on x86-64 is the "integer indefinite" value INT_MIN.
    assert_guard_present(
        &["if value.is_nan() {", "return c_int::MIN;"],
        "row 9: NaN cast must yield INT_MIN like cvttsd2si",
    );
}

// ---------------------------------------------------------------- row 10
#[test]
fn err_row10_mode1_parent_sentinel() {
    assert_guard_present(
        &["(*current_node).parent_id != -1"],
        "row 10: parent_id == -1 terminates the walk",
    );
}

// ---------------------------------------------------------------- row 11
#[test]
fn err_row11_mode1_dangling_parent() {
    assert_guard_present(
        &["if parent_node.is_null() {", "break;"],
        "row 11: dangling parent id breaks the walk",
    );
}

// ---------------------------------------------------------------- row 12
#[test]
fn err_row12_mode1_nonpositive_depth() {
    // Observable at the boundary: depth <= 0 must not change the sentinel.
    let mut rng = Rng::new(0x200c);
    for &d in &[i32::MIN, i32::MIN + 1, -1000, -1, 0] {
        for _ in 0..200 {
            let v = assert_same(1, rng.shaped_i32(), d, rng.shaped_i32());
            assert_eq!(v, ERR_MODE1_NOT_FOUND, "mode 1 depth={d} -> {v}");
        }
    }
    assert_guard_present(&["while i < depth &&"], "row 12: depth is the loop bound");
}

// ---------------------------------------------------------------- row 13
#[test]
fn err_row13_mode4_backward_scan_guard() {
    assert_guard_present(
        &["if count > 2 {", "while i < 3 && iter > base {"],
        "row 13: backward-scan guards",
    );
}

// ---------------------------------------------------------------- row 14
#[test]
fn err_row14_mode2_depth_out_of_bounds() {
    // depth outside [0,16] would put process_backward's `start` outside
    // temp_array. Unreachable publicly (mode 2 errors first) -> must be 34.
    let mut rng = Rng::new(0x200e);
    for &d in &[
        i32::MIN,
        i32::MIN + 1,
        -1_000_000,
        -17,
        -16,
        -1,
        0,
        16,
        17,
        18,
        1_000_000,
        i32::MAX,
    ] {
        for _ in 0..200 {
            let v = assert_same(2, rng.shaped_i32(), d, rng.shaped_i32());
            assert_eq!(v, ERR_MODE2_NOT_FOUND, "mode 2 depth={d} -> {v}");
        }
    }
    assert_guard_present(&["while p > start {"], "row 14: process_backward guard");
}

// ---------------------------------------------------------------- row 15
#[test]
fn err_row15_mode3_sprintf_buffer_boundary() {
    // Widest possible formatting: both ints are INT_MIN ("-2147483648", 11
    // chars each) -> "Node_" 5 + 11 + "_Depth_" 7 + 11 + NUL = 35 <= 50.
    let widest = [i32::MIN, i32::MIN + 1, i32::MAX, -1_000_000_000, 1_000_000_000];
    for &a in &widest {
        for &b in &widest {
            for &f in &[0, 1, 127, 128, -1, i32::MIN, i32::MAX] {
                let v = assert_same(3, a, b, f);
                // strlen == 34 for the two 11-char extremes: 34*2+8 = 76.
                assert!(v >= 8, "unexpectedly small metric {v}");
            }
        }
    }
    let v = assert_same(3, i32::MIN, i32::MIN, 0);
    assert_eq!(v, 34 * 2 + 8, "widest sprintf metric must be 76, got {v}");
    assert_guard_present(&["let mut buffer: [u8; 50] = [0; 50];"], "row 15: buffer[50]");
}

// ---------------------------------------------------------------- row 16
#[test]
fn err_row16_no_pointer_parameters_zero_is_mode0() {
    // jumpnode takes four ints by value: the "null pointer" analogue is 0.
    let v = assert_same(0, 0, 0, 0);
    assert_eq!(v, ERR_BAD_MODE, "operation_mode 0 hits `default:` -> 130");
    // Zero in every other argument position is a valid input, not an error.
    for &m in &[1, 2, 3, 4] {
        assert_same(m, 0, 0, 0);
    }
}

// ---------------------------------------------------------------- generic
#[test]
fn err_generic_boundary_matrix() {
    // Every argument position independently driven to its extremes, plus one
    // step past each documented range boundary.
    let vals = [
        i32::MIN,
        i32::MIN + 1,
        -2,
        -1,
        0,
        1,
        2,
        3,
        4,
        5,
        15,
        16,
        17,
        126,
        127,
        128,
        i32::MAX - 1,
        i32::MAX,
    ];
    for &m in &vals {
        for &id in &vals {
            for &d in &vals {
                for &f in &[i32::MIN, -1, 0, 1, 127, 128, i32::MAX] {
                    assert_same(m, id, d, f);
                }
            }
        }
    }
    // Decimal-width extremes crossed with every mode class.
    for &m in &[1, 2, 3, 4, 0, i32::MIN, i32::MAX] {
        for &id in &DECIMAL_WIDTH_BOUNDARIES {
            assert_same(m, id, id, id);
        }
    }
}

/// Umbrella check that every structurally-unreachable ERRORS.md row still has
/// its guard in the Rust source.
#[test]
fn structural_unreachable_error_paths_documented() {
    assert_guard_present(
        &[
            "return STATUS_ERROR | 0o020;",
            "return STATUS_ERROR | 0o040;",
            "return STATUS_ERROR | 0o100;",
            "result = STATUS_ERROR | 0o200;",
        ],
        "all four jumpnode error sentinels",
    );
    // initialize_test_data must NOT be invoked anywhere outside its own body,
    // mirroring the C where it is a never-called `static` function.
    // Only look at non-comment code lines, and only the *definition* may name it.
    let s = src();
    let mentions: Vec<&str> = s
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.starts_with("//"))
        .filter(|l| l.contains("initialize_test_data"))
        .collect();
    assert_eq!(
        mentions.len(),
        1,
        "initialize_test_data must be mentioned by exactly one code line (its definition) and \
         never called, because the C never calls it; found: {mentions:?}"
    );
    assert!(
        mentions[0].contains("fn initialize_test_data"),
        "the single mention of initialize_test_data must be its definition, not a call: {:?}",
        mentions[0]
    );
}
