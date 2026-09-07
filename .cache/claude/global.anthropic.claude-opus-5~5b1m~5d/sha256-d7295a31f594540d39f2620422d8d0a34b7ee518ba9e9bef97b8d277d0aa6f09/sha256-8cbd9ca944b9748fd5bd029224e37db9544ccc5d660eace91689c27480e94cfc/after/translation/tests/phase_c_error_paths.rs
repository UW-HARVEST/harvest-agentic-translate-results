// Phase C — error-path differential tests, GATED on ERRORS.md.
//
// One test per ERRORS.md row (E1..E10). `driver` returns `void` and has no
// pointer/length/enum parameters, so the "same error/rejection" being asserted
// is the exact rejection MESSAGE plus the internal `result` sentinel surfaced by
// the trailing `Result: %d` line — not merely "both failed somehow".

mod common;

use common::*;

const SEED: u64 = 0xC0FF_EE00_BADD_0001;
const N: usize = 400;

/// Extracts the `result` sentinel from the trailing `Result: <n>\n` line.
#[track_caller]
fn result_code(out: &[u8]) -> i32 {
    let s = std::str::from_utf8(out).expect("output is ASCII");
    let line = s
        .lines()
        .last()
        .unwrap_or_else(|| panic!("no output at all: {s:?}"));
    let n = line
        .strip_prefix("Result: ")
        .unwrap_or_else(|| panic!("last line is not a Result line: {line:?}"));
    n.trim().parse().expect("Result code parses")
}

/// The Phase C workhorse: asserts C and Rust reject identically — same message
/// bytes AND the same numeric sentinel.
#[track_caller]
fn assert_same_rejection(x: i32, y: i32, z: i32, want_msg: &str, want_code: i32) {
    let c = c_output(x, y, z);
    let r = rust_output(x, y, z);
    assert_eq!(
        c,
        r,
        "C/Rust rejection differs for driver({x}, {y}, {z}):\n  C    = {:?}\n  Rust = {:?}",
        String::from_utf8_lossy(&c),
        String::from_utf8_lossy(&r)
    );
    assert_eq!(
        c,
        want_msg.as_bytes(),
        "unexpected rejection bytes for driver({x}, {y}, {z})"
    );
    assert_eq!(
        result_code(&c),
        want_code,
        "wrong error sentinel for driver({x}, {y}, {z})"
    );
    assert_eq!(
        result_code(&r),
        want_code,
        "wrong Rust error sentinel for driver({x}, {y}, {z})"
    );
}

// ---------------------------------------------------------------------------
// E1 — x != 1 (first check, short-circuits everything after it) => result 1
// ---------------------------------------------------------------------------

#[test]
fn e1_x_not_one_rejects_with_code_1() {
    let mut rng = Rng::new(SEED ^ 1);

    // Deterministic neighbours of the single valid value.
    for x in [0, 2, -1, -2, 1_000_000, -1_000_000, i32::MIN, i32::MAX] {
        assert_same_rejection(x, 2, 3, ERR_X, 1);
    }

    // Randomized, with every combination of validity for the later parameters,
    // to prove `x` always wins.
    for _ in 0..N {
        let x = rng.i32_not(1);
        let y = if rng.below(2) == 0 { 2 } else { rng.i32_not(2) };
        let z = if rng.below(2) == 0 { 3 } else { rng.i32_not(3) };
        assert_same_rejection(x, y, z, ERR_X, 1);
    }
}

// ---------------------------------------------------------------------------
// E2 — x == 1 && y != 2 => result 2
// ---------------------------------------------------------------------------

#[test]
fn e2_y_not_two_rejects_with_code_2() {
    let mut rng = Rng::new(SEED ^ 2);

    for y in [0, 1, 3, -1, -2, 123, i32::MIN, i32::MAX] {
        assert_same_rejection(1, y, 3, ERR_Y, 2);
    }

    for _ in 0..N {
        let y = rng.i32_not(2);
        let z = if rng.below(2) == 0 { 3 } else { rng.i32_not(3) };
        assert_same_rejection(1, y, z, ERR_Y, 2);
    }
}

// ---------------------------------------------------------------------------
// E3 — x == 1 && y == 2 && z != 3 => result 3
// ---------------------------------------------------------------------------

#[test]
fn e3_z_not_three_rejects_with_code_3() {
    let mut rng = Rng::new(SEED ^ 3);

    for z in [0, 1, 2, 4, -1, -3, i32::MIN, i32::MAX] {
        assert_same_rejection(1, 2, z, ERR_Z, 3);
    }

    for _ in 0..N {
        let z = rng.i32_not(3);
        assert_same_rejection(1, 2, z, ERR_Z, 3);
    }
}

// ---------------------------------------------------------------------------
// E4 — the shared `fail:` epilogue: "Operation failed" iff result != 0.
// ---------------------------------------------------------------------------

#[test]
fn e4_operation_failed_epilogue_only_on_error_paths() {
    let mut rng = Rng::new(SEED ^ 4);

    // Success must NOT print the epilogue.
    let c = c_output(1, 2, 3);
    let r = rust_output(1, 2, 3);
    assert_eq!(c, r);
    assert!(
        !contains(&c, b"Operation failed"),
        "success path must not reach the fail: epilogue"
    );

    // Every failure must print it exactly once.
    for &(x, y, z) in &[(0, 2, 3), (1, 0, 3), (1, 2, 0)] {
        let c = c_output(x, y, z);
        let r = rust_output(x, y, z);
        assert_eq!(c, r, "divergence for driver({x}, {y}, {z})");
        assert_eq!(
            count(&c, b"Operation failed\n"),
            1,
            "fail: epilogue must appear exactly once for driver({x}, {y}, {z})"
        );
    }

    // Randomized invariant sweep over all four outcomes.
    for _ in 0..(N * 2) {
        let (x, y, z) = (
            rng.biased_i32_favouring(1),
            rng.biased_i32_favouring(2),
            rng.biased_i32_favouring(3),
        );
        let c = c_output(x, y, z);
        let r = rust_output(x, y, z);
        assert_eq!(c, r, "divergence for driver({x}, {y}, {z})");
        let code = result_code(&c);
        assert_eq!(
            count(&c, b"Operation failed\n"),
            usize::from(code != 0),
            "epilogue/result-code mismatch for driver({x}, {y}, {z})"
        );
        assert_eq!(
            count(&c, b"Ok!\n"),
            usize::from(code == 0),
            "Ok!/result-code mismatch for driver({x}, {y}, {z})"
        );
        assert_eq!(code, expected_code(x, y, z));
    }
}

// ---------------------------------------------------------------------------
// E5 — check ORDER / short-circuit: only the FIRST failing stage is reported.
// ---------------------------------------------------------------------------

#[test]
fn e5_only_first_failing_check_is_reported() {
    // Two invalid at once.
    assert_same_rejection(0, 0, 3, ERR_X, 1); // x before y
    assert_same_rejection(0, 2, 0, ERR_X, 1); // x before z
    assert_same_rejection(1, 0, 0, ERR_Y, 2); // y before z
    // All three invalid.
    assert_same_rejection(0, 0, 0, ERR_X, 1);
    assert_same_rejection(i32::MIN, i32::MIN, i32::MIN, ERR_X, 1);
    assert_same_rejection(i32::MAX, i32::MAX, i32::MAX, ERR_X, 1);

    // Never more than one `Error: ` line, for any input.
    let mut rng = Rng::new(SEED ^ 5);
    for _ in 0..(N * 2) {
        let (x, y, z) = (
            rng.biased_i32_favouring(1),
            rng.biased_i32_favouring(2),
            rng.biased_i32_favouring(3),
        );
        let c = c_output(x, y, z);
        let r = rust_output(x, y, z);
        assert_eq!(c, r, "divergence for driver({x}, {y}, {z})");
        assert!(
            count(&c, b"Error: ") <= 1,
            "more than one Error: line for driver({x}, {y}, {z})"
        );
    }
}

// ---------------------------------------------------------------------------
// E6/E7/E8 — extreme and one-past-valid values per parameter.
// ---------------------------------------------------------------------------

#[test]
fn e6_x_extremes_and_off_by_one() {
    for x in [i32::MIN, i32::MIN + 1, -1, 0, 2, i32::MAX - 1, i32::MAX] {
        assert_same_rejection(x, 2, 3, ERR_X, 1);
    }
    // The single valid value must NOT be rejected.
    assert_same_and_eq(1, 2, 3, OK);
}

#[test]
fn e7_y_extremes_and_off_by_one() {
    for y in [i32::MIN, i32::MIN + 1, -2, -1, 0, 1, 3, 123, i32::MAX - 1, i32::MAX] {
        assert_same_rejection(1, y, 3, ERR_Y, 2);
    }
    assert_same_and_eq(1, 2, 3, OK);
}

#[test]
fn e8_z_extremes_and_off_by_one() {
    for z in [i32::MIN, i32::MIN + 1, -3, -1, 0, 2, 4, i32::MAX - 1, i32::MAX] {
        assert_same_rejection(1, 2, z, ERR_Z, 3);
    }
    assert_same_and_eq(1, 2, 3, OK);
}

// ---------------------------------------------------------------------------
// E9 — "out-of-range enum value" analogue. `driver` takes plain `int`s, so any
// of the 2^32 bit patterns is a legal FFI input with no "valid variant". Both
// implementations must classify all of them identically. This includes values
// that would trap only under a signed/unsigned or narrowing mistranslation:
// 0x8000_0000, 0xFFFF_FFFF, and values whose low 32 bits look like 1/2/3.
// ---------------------------------------------------------------------------

#[test]
fn e9_arbitrary_int_bit_patterns_classified_identically() {
    // Values chosen to catch sign/width mistranslations around the constants
    // 1, 2 and 3.
    let tricky: Vec<i32> = vec![
        0,
        1,
        2,
        3,
        -1,
        -2,
        -3,
        i32::MIN,
        i32::MAX,
        i32::MIN + 1,
        i32::MAX - 1,
        0x0000_0001u32 as i32,
        0x8000_0001u32 as i32, // low bits look like 1, sign bit set
        0x8000_0002u32 as i32,
        0x8000_0003u32 as i32,
        0xFFFF_FFFFu32 as i32,
        0x0001_0001u32 as i32, // 1 in the low 16 bits
        0x0001_0002u32 as i32,
        0x0001_0003u32 as i32,
        0x0000_FF01u32 as i32,
        1 << 16,
        1 << 30,
        -(1 << 30),
    ];

    // Full cross product over the tricky set, batched per-x into one capture.
    for &x in &tricky {
        let mut calls = Vec::new();
        let mut want = String::new();
        for &y in &tricky {
            for &z in &tricky {
                calls.push((x, y, z));
                want.push_str(expected(x, y, z));
            }
        }
        let got = assert_same_seq(&calls);
        assert_eq!(
            got,
            want.as_bytes(),
            "tricky-bit-pattern slice x={x} does not match the driver.c model"
        );
    }

    // And a broad randomized sweep of the full i32 range.
    let mut rng = Rng::new(SEED ^ 9);
    let mut calls = Vec::with_capacity(1000);
    let mut want = String::new();
    for _ in 0..1000 {
        let (x, y, z) = (rng.next_i32(), rng.next_i32(), rng.next_i32());
        calls.push((x, y, z));
        want.push_str(expected(x, y, z));
    }
    let got = assert_same_seq(&calls);
    assert_eq!(got, want.as_bytes());
}

// ---------------------------------------------------------------------------
// E10 — no state can leak across calls (the file-scope `static int y`).
// ---------------------------------------------------------------------------

#[test]
fn e10_error_then_success_and_interleavings() {
    // Every ordered pair of the four representative outcomes.
    let reps = [(1, 2, 3), (0, 2, 3), (1, 0, 3), (1, 2, 0)];
    for &a in &reps {
        for &b in &reps {
            let calls = [a, b];
            let got = assert_same_seq(&calls);
            let want = format!("{}{}", expected(a.0, a.1, a.2), expected(b.0, b.1, b.2));
            assert_eq!(
                got,
                want.as_bytes(),
                "sequence {calls:?} diverges from the per-call model"
            );
        }
    }

    // Poison the static with many different values, then require a clean success.
    let mut rng = Rng::new(SEED ^ 10);
    for _ in 0..N {
        let poison = rng.next_i32();
        let calls = [(1, poison, 0), (1, 2, 3)];
        let got = assert_same_seq(&calls);
        let want = format!("{}{}", expected(1, poison, 0), OK);
        assert_eq!(got, want.as_bytes(), "static y leaked after poison={poison}");
    }
}

// ---------------------------------------------------------------------------
// Generic boundary cases that have no representable form in this API — asserted
// as source facts so the claim in ERRORS.md cannot silently rot.
// ---------------------------------------------------------------------------

#[test]
fn no_pointer_length_or_enum_parameters_exist_in_the_api() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let header = std::fs::read_to_string(root.parent().unwrap().join("c_src/include/driver.h"))
        .expect("read driver.h");
    // Strip comments so the licence text does not trip the checks.
    let decls: String = header
        .lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");

    assert!(
        decls.contains("void driver(int x, int y, int z);"),
        "public API changed; ERRORS.md must be regenerated. Saw:\n{decls}"
    );
    assert!(
        !decls.contains('*'),
        "the API now has pointer parameters — null-pointer rows are required"
    );
    assert!(
        !decls.contains("enum"),
        "the API now has enum parameters — out-of-range-enum rows are required"
    );
    assert!(
        !decls.contains("size_t") && !decls.contains("len"),
        "the API now has length parameters — zero/oversized-length rows are required"
    );

    let src = std::fs::read_to_string(root.parent().unwrap().join("c_src/src/driver.c"))
        .expect("read driver.c");
    assert!(!src.contains("assert"), "the C source now asserts — abort rows are required");
    assert!(
        !src.contains("malloc") && !src.contains("calloc") && !src.contains("realloc"),
        "the C source now allocates — allocation-failure rows are required"
    );
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

fn contains(hay: &[u8], needle: &[u8]) -> bool {
    count(hay, needle) > 0
}

fn count(hay: &[u8], needle: &[u8]) -> usize {
    if needle.is_empty() || hay.len() < needle.len() {
        return 0;
    }
    hay.windows(needle.len()).filter(|w| *w == needle).count()
}
