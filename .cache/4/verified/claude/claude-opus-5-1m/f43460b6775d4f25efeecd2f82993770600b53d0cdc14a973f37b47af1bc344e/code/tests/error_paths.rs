//! Phase C — error-path differential tests, one test per row of `ERRORS.md`
//! (library rows E1..E20; the executable rows E21..E26 live in `executable.rs`).

mod common;

use common::{assert_same, assert_same_full, assert_same_null, Rng, INT_EDGE_VALUES};

// ===========================================================================
// process_decisions guard clauses
// ===========================================================================

/// E1: `decision_string == NULL`, `length > 0` -> -1.
#[test]
fn err_e1_null_pointer() {
    for length in [1usize, 2, 3, 4, 31, 32, 33, 1023, usize::MAX] {
        for op in INT_EDGE_VALUES {
            for param in [0, 1, 2, 3, -1, 99] {
                let r = assert_same_null(length, op, param, "E1");
                assert_eq!(r, -1, "E1: op={op} param={param} length={length}");
            }
        }
    }
}

/// E2: `length == 0` -> -1, and it wins over the unknown-operation -3.
#[test]
fn err_e2_zero_length() {
    for op in INT_EDGE_VALUES {
        for param in INT_EDGE_VALUES {
            let r = assert_same(b"", 0, op, param, "E2");
            assert_eq!(r, -1, "E2: op={op} param={param}");
        }
    }
    // Non-empty buffer, but a claimed length of zero.
    for op in -2..6 {
        let r = assert_same(b"yyyyyyyy", 0, op, 0, "E2-nonempty-buffer");
        assert_eq!(r, -1, "E2: op={op}");
    }
}

/// E3: NULL pointer *and* zero length -> -1.
#[test]
fn err_e3_null_and_zero_length() {
    for op in INT_EDGE_VALUES {
        for param in INT_EDGE_VALUES {
            let r = assert_same_null(0, op, param, "E3");
            assert_eq!(r, -1, "E3: op={op} param={param}");
        }
    }
}

/// E4: operation 0 with 1 <= length <= 2 -> -2.
#[test]
fn err_e4_op0_short_length() {
    for length in [1usize, 2] {
        for buf in [b"y".as_slice(), b"n", b"yy", b"yn", b"ny", b"nn", b"\x00\x00", b"??"] {
            if buf.len() < length {
                continue;
            }
            for param in INT_EDGE_VALUES {
                let r = assert_same(buf, length, 0, param, "E4");
                assert_eq!(r, -2, "E4: length={length} buf={buf:02x?} param={param}");
            }
        }
    }
    // length 3 is the first accepted value: must NOT be -2.
    assert_ne!(assert_same_full(b"yyy", 0, 0, "E4-boundary"), -2);
}

/// E5: operation 1 with 1 <= length <= 2 -> -2.
#[test]
fn err_e5_op1_short_length() {
    for length in [1usize, 2] {
        for buf in [b"y".as_slice(), b"n", b"yy", b"yn", b"ny", b"nn", b"\xff\x80"] {
            if buf.len() < length {
                continue;
            }
            for param in INT_EDGE_VALUES {
                let r = assert_same(buf, length, 1, param, "E5");
                assert_eq!(r, -2, "E5: length={length} buf={buf:02x?} param={param}");
            }
        }
    }
    assert_ne!(assert_same_full(b"yyy", 1, 0, "E5-boundary"), -2);
}

/// E6: `operation` outside {0,1,2,3} -> -3, including out-of-range "enum" ints.
#[test]
fn err_e6_unknown_operation() {
    let invalid = [
        i32::MIN,
        i32::MIN + 1,
        -1000,
        -4,
        -3,
        -2,
        -1,
        4, // one past the top of the valid range
        5,
        6,
        1000,
        i32::MAX - 1,
        i32::MAX,
    ];

    for op in invalid {
        for param in INT_EDGE_VALUES {
            for buf in [b"y".as_slice(), b"yyy", b"nnnnnnnnnn", b"\x00\xff\x7f"] {
                let r = assert_same_full(buf, op, param, "E6");
                assert_eq!(r, -3, "E6: op={op} param={param} buf={buf:02x?}");
            }
        }
    }

    // Every value in the valid range must NOT be -3.
    for op in 0..4 {
        assert_ne!(assert_same_full(b"yyy", op, 0, "E6-valid"), -3, "op={op}");
    }
}

// ===========================================================================
// apply_permissions (operation 0)
// ===========================================================================

/// E7: write-only -> -10.
#[test]
fn err_e7_write_only() {
    // read=false, write=true, execute=false.
    for r0 in [b'n', b'N', b'?', 0u8, 0xffu8] {
        for w in [b'y', b'Y'] {
            for x in [b'n', b'N', b'?', 0u8, 0xffu8] {
                let res = assert_same_full(&[r0, w, x], 0, 0, "E7");
                assert_eq!(res, -10, "E7: {:02x?}", [r0, w, x]);
            }
        }
    }
}

/// E8: execute-only -> -20.
#[test]
fn err_e8_execute_only() {
    for r0 in [b'n', b'N', b'?', 0u8, 0xffu8] {
        for w in [b'n', b'N', b'?', 0u8, 0xffu8] {
            for x in [b'y', b'Y'] {
                let res = assert_same_full(&[r0, w, x], 0, 0, "E8");
                assert_eq!(res, -20, "E8: {:02x?}", [r0, w, x]);
            }
        }
    }
}

/// E9: the read&&write&&!execute branch never falls through to `return 0`
/// (`permission_value` is provably 6 there) — verify C and Rust agree that the
/// answer is 56 and never 0.
#[test]
fn err_e9_read_write_no_execute_never_falls_through() {
    for r0 in [b'y', b'Y'] {
        for w in [b'y', b'Y'] {
            for x in [b'n', b'N', b'?', 0u8, 0xffu8, b'0'] {
                let res = assert_same_full(&[r0, w, x], 0, 0, "E9");
                assert_eq!(res, 56, "E9: {:02x?}", [r0, w, x]);
            }
        }
    }
    // And the genuine "no permissions" zero.
    assert_eq!(assert_same_full(b"nnn", 0, 0, "E9-zero"), 0);
}

// ===========================================================================
// evaluate_conditions (operation 1)
// ===========================================================================

/// E10: `param` (logic_op) outside {0,1,2,3} -> -1, for every condition triple.
#[test]
fn err_e10_unknown_logic_op() {
    let invalid = [
        i32::MIN,
        i32::MIN + 1,
        -1000,
        -2,
        -1,
        4, // one past the top of the valid range
        5,
        99,
        1000,
        i32::MAX - 1,
        i32::MAX,
    ];

    for param in invalid {
        for bits in 0u32..8 {
            let input = common::pattern_to_bytes(bits, 3);
            let r = assert_same_full(&input, 1, param, "E10");
            assert_eq!(r, -1, "E10: param={param} bits={bits}");
        }
        // Longer inputs take the same path.
        let r = assert_same_full(b"ynynynynyn", 1, param, "E10-long");
        assert_eq!(r, -1, "E10-long: param={param}");
    }

    // Valid logic ops must NOT be -1.
    for param in 0..4 {
        assert_ne!(assert_same_full(b"yyy", 1, param, "E10-valid"), -1);
    }
}

// ===========================================================================
// validate_sequence (operation 3)
// ===========================================================================

/// E11: `len == 0` — internally `return 0`, but unreachable through the public
/// entry point, which reports -1 first.
#[test]
fn err_e11_validate_zero_len() {
    for param in INT_EDGE_VALUES {
        let r = assert_same(b"", 0, 3, param, "E11");
        assert_eq!(r, -1, "E11: param={param}");
        let r = assert_same(b"yyyy", 0, 3, param, "E11-nonempty");
        assert_eq!(r, -1, "E11-nonempty: param={param}");
    }
    assert_eq!(assert_same_null(0, 3, 0, "E11-null"), -1);
}

/// E12: rule 1 — first byte does not parse as true -> -10.
#[test]
fn err_e12_rule1_must_start_true() {
    // Every byte other than y/Y must trip rule 1.
    for b in 0u16..=255 {
        let b = b as u8;
        if b == b'y' || b == b'Y' {
            continue;
        }
        assert_eq!(assert_same_full(&[b], 3, 0, "E12-len1"), -10, "byte {b:#04x}");
        assert_eq!(
            assert_same_full(&[b, b'n'], 3, 0, "E12-len2"),
            -10,
            "byte {b:#04x}"
        );
        assert_eq!(
            assert_same_full(&[b, b'y', b'n', b'y', b'n'], 3, 0, "E12-len5"),
            -10,
            "byte {b:#04x}"
        );
    }

    // Randomized: any sequence starting with a non-true byte is -10.
    let mut rng = Rng::new(0xE12);
    for _ in 0..20_000 {
        let len = 1 + rng.below(50);
        let mut input: Vec<u8> = (0..len).map(|_| rng.yn_byte()).collect();
        input[0] = if rng.bool() { b'n' } else { b'N' };
        assert_eq!(assert_same_full(&input, 3, 0, "E12-rand"), -10);
    }
}

/// E13: rule 2 — `len > 1` and the last byte parses as true -> -11.
#[test]
fn err_e13_rule2_must_end_false() {
    let mut rng = Rng::new(0xE13);
    for _ in 0..20_000 {
        let len = 2 + rng.below(49);
        let mut input: Vec<u8> = (0..len).map(|_| rng.yn_byte()).collect();
        input[0] = if rng.bool() { b'y' } else { b'Y' };
        input[len - 1] = if rng.bool() { b'y' } else { b'Y' };
        assert_eq!(
            assert_same_full(&input, 3, 0, "E13"),
            -11,
            "E13: {:?}",
            String::from_utf8_lossy(&input)
        );
    }

    // len == 1 is exempt from rule 2 (the `len > 1` guard).
    assert_eq!(assert_same_full(b"y", 3, 0, "E13-len1"), 1);
    assert_eq!(assert_same_full(b"Y", 3, 0, "E13-len1-upper"), 1);
    // Exactly at the boundary len == 2.
    assert_eq!(assert_same_full(b"yy", 3, 0, "E13-len2"), -11);
    assert_eq!(assert_same_full(b"yn", 3, 0, "E13-len2-ok"), 2);
}

/// E14: rule 3 — a run of more than 3 equal parsed values -> -12.
#[test]
fn err_e14_rule3_max_consecutive() {
    // Runs of true.
    assert_eq!(assert_same_full(b"yyyyn", 3, 0, "E14-true-run"), -12);
    // Exactly 3 in a row is still allowed (`consecutive > 3`, not `>= 3`).
    assert_ne!(assert_same_full(b"yyyn", 3, 0, "E14-run-of-3-ok"), -12);

    // Runs of false.
    assert_eq!(assert_same_full(b"ynnnn", 3, 0, "E14-false-run"), -12);
    assert_ne!(assert_same_full(b"ynnn", 3, 0, "E14-false-run3"), -12);

    // Run appearing in the middle / at the end.
    assert_eq!(assert_same_full(b"ynynnnnyn", 3, 0, "E14-middle"), -12);
    assert_eq!(assert_same_full(b"ynyyyyn", 3, 0, "E14-middle-true"), -12);

    // Every run length 4..=20 at every offset in a 24-byte sequence.
    for run in 4..=20usize {
        for offset in 0..(24 - run) {
            let mut input = vec![b'n'; 24];
            input[0] = b'y';
            for i in offset..offset + run {
                input[i] = b'y';
            }
            input[23] = b'n';
            if input[0] != b'y' {
                continue;
            }
            // A y-run of >= 4 anywhere, or the surrounding n-run, must be -12.
            let r = assert_same_full(&input, 3, 0, "E14-sweep");
            assert_eq!(r, -12, "E14-sweep run={run} offset={offset}");
        }
    }

    // Randomized: rule 3 must fire iff some run length exceeds 3 (and rules 1
    // and 2 pass first).
    let mut rng = Rng::new(0xE14);
    for _ in 0..30_000 {
        let len = 2 + rng.below(40);
        let mut input: Vec<u8> = (0..len).map(|_| rng.yn_byte()).collect();
        input[0] = b'y';
        input[len - 1] = b'n';
        let r = assert_same_full(&input, 3, 0, "E14-rand");

        let bools: Vec<bool> = input.iter().map(|&b| b == b'y' || b == b'Y').collect();
        let mut worst = 1usize;
        let mut run = 1usize;
        for i in 1..len {
            if bools[i] == bools[i - 1] {
                run += 1;
                worst = worst.max(run);
            } else {
                run = 1;
            }
        }
        if worst > 3 {
            assert_eq!(r, -12, "E14-rand expected -12 for {input:02x?}");
        } else {
            assert_ne!(r, -12, "E14-rand unexpected -12 for {input:02x?}");
        }
    }
}

// ===========================================================================
// Bounds / min-max constants
// ===========================================================================

/// E15: operation 2 clamps `count` at 32; bytes 32.. are ignored.
#[test]
fn err_e15_op2_count_clamped_to_32() {
    let mut rng = Rng::new(0xE15);
    for len in 33..=200usize {
        for _ in 0..30 {
            let input: Vec<u8> = (0..len).map(|_| rng.yn_byte()).collect();
            let full = assert_same_full(&input, 2, 0, "E15");
            let clamped = assert_same_full(&input[..32], 2, 0, "E15-prefix");
            assert_eq!(full, clamped, "E15: len={len} tail influenced the result");
        }
    }
    // Extreme: 1023 bytes where only the tail differs.
    let mut a = vec![b'y'; 1023];
    let mut b = vec![b'y'; 1023];
    a[500] = b'n';
    b[900] = b'n';
    assert_eq!(
        assert_same_full(&a, 2, 0, "E15-a"),
        assert_same_full(&b, 2, 0, "E15-b")
    );
}

/// E16: `1u << i` never shifts by >= 32 (`i < count && i < 32`).
#[test]
fn err_e16_configure_flags_shift_bound() {
    // Only the bit at index 31 can be the highest one.
    for len in [31usize, 32, 33, 63, 64, 1023] {
        let mut only_last = vec![b'n'; len];
        only_last[31.min(len - 1)] = b'y';
        assert_same_full(&only_last, 2, 0, "E16-bit31");

        let mut past_32 = vec![b'n'; len];
        if len > 32 {
            past_32[32] = b'y';
            past_32[len - 1] = b'y';
            // Every true bit lives past the clamp, so this must look all-false.
            assert_eq!(assert_same_full(&past_32, 2, 0, "E16-past32"), 0);
        }

        assert_same_full(&vec![b'y'; len], 2, 0, "E16-all-true");
    }
}

/// E17: `special_count == count - 1` is an unsigned comparison.
#[test]
fn err_e17_configure_flags_count_minus_one_unsigned() {
    // Sweep every count in 1..=32 with every possible number of true values.
    for count in 1..=32usize {
        for trues in 0..=count {
            // A few different placements per (count, trues).
            let mut rng = Rng::new(0xE17 ^ ((count as u64) << 8) ^ trues as u64);
            for _ in 0..20 {
                let mut input = vec![b'n'; count];
                let mut placed = 0;
                while placed < trues {
                    let i = rng.below(count);
                    if input[i] == b'n' {
                        input[i] = b'y';
                        placed += 1;
                    }
                }
                assert_same_full(&input, 2, 0, "E17");
            }
        }
    }
}

/// E18: `size_t` subtractions / mixed-signedness comparisons in
/// `validate_sequence` (`len - 1`, `len - 3`, `transitions < len/3`).
#[test]
fn err_e18_validate_size_t_comparisons() {
    // len == 1 -> `len - 1 == 0`; the `transitions == 0` check fires first.
    assert_eq!(assert_same_full(b"y", 3, 0, "E18-len1"), 1);

    // Exhaustive around each branch boundary.
    for len in [1usize, 2, 3, 4, 9, 10, 11, 12, 13] {
        for bits in 0u32..(1u32 << len) {
            assert_same_full(&common::pattern_to_bytes(bits, len), 3, 0, "E18");
        }
    }

    // Long tier: `transitions > len - 3`.
    for len in [11usize, 12, 13, 14, 20, 33, 64] {
        let alt: Vec<u8> = (0..len)
            .map(|i| if i % 2 == 0 { b'y' } else { b'n' })
            .collect();
        assert_same_full(&alt, 3, 0, "E18-alt");
        let mut few = vec![b'n'; len];
        few[0] = b'y';
        assert_same_full(&few, 3, 0, "E18-few");
    }
}

/// E19: `length == 1023` (`MAX_INPUT_SIZE - 1`) for every operation.
#[test]
fn err_e19_max_length_1023() {
    let mut rng = Rng::new(0xE19);
    for op in -1..5 {
        for param in [0, 1, 2, 3, -1, 42] {
            for _ in 0..20 {
                let input: Vec<u8> = (0..1023).map(|_| rng.yn_byte()).collect();
                assert_same_full(&input, op, param, "E19");
            }
            assert_same_full(&vec![b'y'; 1023], op, param, "E19-all-true");
            assert_same_full(&vec![b'n'; 1023], op, param, "E19-all-false");
            let alt: Vec<u8> = (0..1023)
                .map(|i| if i % 2 == 0 { b'y' } else { b'n' })
                .collect();
            assert_same_full(&alt, op, param, "E19-alt");
        }
    }
}

/// E20: `parse_bool` silently maps every unrecognised byte to false, including
/// bytes with the sign bit set (negative `char` on x86-64) and NUL.
#[test]
fn err_e20_parse_bool_invalid_bytes() {
    for b in 0u16..=255 {
        let b = b as u8;
        let is_true = b == b'y' || b == b'Y';
        let is_false = b == b'n' || b == b'N';

        // Operation 0: a triple of the same byte.
        let r = assert_same_full(&[b, b, b], 0, 0, "E20-op0");
        if !is_true {
            assert_eq!(r, 0, "E20: byte {b:#04x} should behave like all-false");
        }

        // Operation 1, every logic op.
        for param in 0..4 {
            let r1 = assert_same_full(&[b, b, b], 1, param, "E20-op1");
            if !is_true {
                let all_false = assert_same_full(b"nnn", 1, param, "E20-op1-ref");
                assert_eq!(r1, all_false, "E20: byte {b:#04x} param={param}");
            }
        }

        // Operation 2.
        let r2 = assert_same_full(&[b, b, b, b, b], 2, 0, "E20-op2");
        if !is_true {
            assert_eq!(r2, 0, "E20: byte {b:#04x} op2");
        }

        // Operation 3.
        let r3 = assert_same_full(&[b, b, b], 3, 0, "E20-op3");
        if !is_true {
            assert_eq!(r3, -10, "E20: byte {b:#04x} op3");
        }
        let _ = is_false;
    }

    // Mixed: a valid prefix followed by junk.
    let mut rng = Rng::new(0xE20);
    for _ in 0..30_000 {
        let len = 1 + rng.below(40);
        let input: Vec<u8> = (0..len).map(|_| rng.byte()).collect();
        for op in 0..4 {
            assert_same_full(&input, op, rng.range(-2, 5) as i32, "E20-mixed");
        }
    }
}
