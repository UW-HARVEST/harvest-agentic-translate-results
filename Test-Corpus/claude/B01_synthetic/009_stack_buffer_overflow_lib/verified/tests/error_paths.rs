//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md`.  Every function in this C library is `void`
//! and there is no `errno` / error code / error enum, so "the same rejection"
//! means "the same rejection message bytes on stdout and nothing else".  Each
//! test asserts that for the C `.so`, for the Rust `.so`, and against the message
//! text taken verbatim from `c_src/src/driver.c`.

mod common;
use common::*;

fn contains(hay: &[u8], needle: &[u8]) -> bool {
    hay.len() >= needle.len() && hay.windows(needle.len()).any(|w| w == needle)
}

/// goodG2B's fixed block, which `good()` always prints first.
fn g2b_block() -> Vec<u8> {
    ten_lines(Some(7))
}

/// What `good(data)` must print for an out-of-range `data`.
fn good_rejected() -> Vec<u8> {
    let mut v = g2b_block();
    v.extend_from_slice(ERR_OOB);
    v
}

/// ERRORS row 1 — `printLine(NULL)`: `if(line != NULL)` fails ⇒ nothing at all.
#[test]
fn err_01_print_line_null() {
    let op = Op::PrintLineNull;
    assert_same(&op);
    for lib in [c_lib(), rust_lib()] {
        let out = output(lib, &op);
        assert!(
            out.is_empty(),
            "{} lib printed {:?} for printLine(NULL)",
            lib.name,
            String::from_utf8_lossy(&out)
        );
    }
}

/// ERRORS row 2 — `bad(-1)` ⇒ "ERROR: Array index is negative.".
#[test]
fn err_02_bad_negative_one() {
    let op = Op::Bad(-1);
    assert_same(&op);
    for lib in [c_lib(), rust_lib()] {
        assert_eq!(output(lib, &op), ERR_NEGATIVE, "{} lib", lib.name);
    }
}

/// ERRORS row 3 — `bad(INT_MIN)`.
#[test]
fn err_03_bad_int_min() {
    let op = Op::Bad(i32::MIN);
    assert_same(&op);
    for lib in [c_lib(), rust_lib()] {
        assert_eq!(output(lib, &op), ERR_NEGATIVE, "{} lib", lib.name);
    }
}

/// ERRORS row 4 — randomised negative `data`, 200+ values.
#[test]
fn err_04_bad_negative_random() {
    let mut rng = Rng::new(0xE770_0004);
    let mut ops: Vec<Op> = [-1, -2, -10, i32::MIN, i32::MIN + 1, -1_000_000]
        .iter()
        .map(|&d| Op::Bad(d))
        .collect();
    for _ in 0..200 {
        ops.push(Op::Bad(rng.range_i32(i32::MIN, -1)));
    }
    assert_same_batch(&ops);
    for lib in [c_lib(), rust_lib()] {
        for (out, op) in outputs(lib, &ops).iter().zip(&ops) {
            assert_eq!(out, ERR_NEGATIVE, "{} lib {}", lib.name, op.describe());
        }
    }
}

/// ERRORS row 5 — `data >= 10` is *not* rejected (the injected flaw): the store
/// is out of bounds, yet the ten in-bounds elements are still printed unchanged.
#[test]
fn err_05_bad_oob_not_rejected() {
    for d in [10, 11, 13, 16, 20, 33, 64, 1000] {
        let op = Op::Bad(d);
        assert_same_stdout_ub(&op);
        let c = ub_stream(c_lib(), &op);
        assert_eq!(
            c,
            ten_lines(None),
            "C lib: bad({d}) should print ten zeros (the OOB store misses the array)"
        );
        assert!(
            !contains(&c, ERR_NEGATIVE) && !contains(&c, ERR_OOB),
            "C lib must NOT reject bad({d})"
        );
    }
}

/// ERRORS row 6 — `bad(10)`, the very first out-of-range index.
#[test]
fn err_06_bad_oob_first() {
    let op = Op::Bad(10);
    assert_same_stdout_ub(&op);
    assert_eq!(ub_stream(c_lib(), &op), ten_lines(None));
    assert_eq!(ub_stream(rust_lib(), &op), ten_lines(None));
}

/// ERRORS row 7 — `bad(INT_MAX)`: accepted by `data >= 0`, store far off-stack.
#[test]
fn err_07_bad_int_max() {
    assert_same_stdout_ub(&Op::Bad(i32::MAX));
    assert_same_stdout_ub(&Op::Bad(i32::MAX - 1));
}

/// ERRORS row 8 — `good(-1)` ⇒ goodG2B block + "out-of-bounds" message.
#[test]
fn err_08_good_negative() {
    let op = Op::Good(-1);
    assert_same(&op);
    for lib in [c_lib(), rust_lib()] {
        assert_eq!(output(lib, &op), good_rejected(), "{} lib", lib.name);
    }
}

/// ERRORS row 9 — `good(INT_MIN)`.
#[test]
fn err_09_good_int_min() {
    let op = Op::Good(i32::MIN);
    assert_same(&op);
    for lib in [c_lib(), rust_lib()] {
        assert_eq!(output(lib, &op), good_rejected(), "{} lib", lib.name);
    }
}

/// ERRORS row 10 — `good(10)`: the first value failing `data < (10)`; `good(9)`
/// (one step inside the range) is still accepted.
#[test]
fn err_10_good_ten() {
    let ops = [Op::Good(10), Op::Good(9)];
    assert_same_batch(&ops);
    for lib in [c_lib(), rust_lib()] {
        let out = outputs(lib, &ops);
        assert_eq!(out[0], good_rejected(), "{} lib good(10)", lib.name);
        assert!(
            !contains(&out[1], ERR_OOB),
            "{} lib rejected good(9): {:?}",
            lib.name,
            String::from_utf8_lossy(&out[1])
        );
    }
}

/// ERRORS row 11 — `good(INT_MAX)`.
#[test]
fn err_11_good_int_max() {
    let op = Op::Good(i32::MAX);
    assert_same(&op);
    for lib in [c_lib(), rust_lib()] {
        assert_eq!(output(lib, &op), good_rejected(), "{} lib", lib.name);
    }
}

/// ERRORS row 12 — randomised out-of-range `data` for `goodB2G`, 200 values.
#[test]
fn err_12_good_out_of_range_random() {
    let mut rng = Rng::new(0xE770_0012);
    let ops: Vec<Op> = (0..200)
        .map(|i| {
            Op::Good(if i % 2 == 0 {
                rng.range_i32(i32::MIN, -1)
            } else {
                rng.range_i32(10, i32::MAX)
            })
        })
        .collect();
    assert_same_batch(&ops);
    let expected = good_rejected();
    for lib in [c_lib(), rust_lib()] {
        for (out, op) in outputs(lib, &ops).iter().zip(&ops) {
            assert_eq!(out, &expected, "{} lib {}", lib.name, op.describe());
        }
    }
}

/// ERRORS row 13 — `good(9)` is the last accepted value.
#[test]
fn err_13_good_nine_accepted() {
    let op = Op::Good(9);
    assert_same(&op);
    let mut expected = g2b_block();
    expected.extend_from_slice(&ten_lines(Some(9)));
    for lib in [c_lib(), rust_lib()] {
        assert_eq!(output(lib, &op), expected, "{} lib", lib.name);
    }
}

/// ERRORS row 14 — `good(0)` is the lower boundary and is accepted.
#[test]
fn err_14_good_zero_accepted() {
    let op = Op::Good(0);
    assert_same(&op);
    let mut expected = g2b_block();
    expected.extend_from_slice(&ten_lines(Some(0)));
    for lib in [c_lib(), rust_lib()] {
        assert_eq!(output(lib, &op), expected, "{} lib", lib.name);
    }
}

/// ERRORS row 15 — `bad(0)` / `bad(9)`: the boundaries the missing upper-bound
/// check would have used are accepted and stay in bounds.
#[test]
fn err_15_bad_boundaries_accepted() {
    let ops = [Op::Bad(0), Op::Bad(9)];
    assert_same_batch(&ops);
    for lib in [c_lib(), rust_lib()] {
        let out = outputs(lib, &ops);
        assert_eq!(out[0], ten_lines(Some(0)), "{} lib bad(0)", lib.name);
        assert_eq!(out[1], ten_lines(Some(9)), "{} lib bad(9)", lib.name);
    }
}

/// ERRORS row 16 — `goodG2B`'s "negative" branch is unreachable (`data` is the
/// literal `7`), so neither library may ever print that message from
/// `good`/`driver`, and the fixed block always comes first.
#[test]
fn err_16_goodg2b_error_branch_unreachable() {
    let mut rng = Rng::new(0xE770_0016);
    let mut ops: Vec<Op> = [-1, 0, 7, 9, 10, i32::MIN, i32::MAX]
        .iter()
        .map(|&d| Op::Good(d))
        .collect();
    for _ in 0..100 {
        ops.push(Op::Good(rng.next_i32()));
    }
    assert_same_batch(&ops);
    let block = g2b_block();
    for lib in [c_lib(), rust_lib()] {
        for (out, op) in outputs(lib, &ops).iter().zip(&ops) {
            assert!(
                !contains(out, ERR_NEGATIVE),
                "{} lib printed goodG2B's 'negative' message for {}",
                lib.name,
                op.describe()
            );
            assert!(
                out.starts_with(&block),
                "{} lib: {} did not start with goodG2B's fixed block",
                lib.name,
                op.describe()
            );
        }
    }
}

/// The full seven-part `driver` transcript for the given inner blocks.
fn driver_transcript(good_block: &[u8], bad_block: &[u8]) -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(b"Calling good()...\n");
    v.extend_from_slice(good_block);
    v.extend_from_slice(b"Finished good()\n");
    v.extend_from_slice(b"Calling bad()...\n");
    v.extend_from_slice(bad_block);
    v.extend_from_slice(b"Finished bad()\n");
    v
}

/// ERRORS row 17 — `driver` with both error branches taken at once.
#[test]
fn err_17_driver_both_error_branches() {
    let op = Op::Driver(-5, -9);
    assert_same(&op);
    let expected = driver_transcript(&good_rejected(), ERR_NEGATIVE);
    for lib in [c_lib(), rust_lib()] {
        assert_eq!(output(lib, &op), expected, "{} lib", lib.name);
    }
}

/// ERRORS row 18 — only `good`'s check rejects.
#[test]
fn err_18_driver_only_good_rejects() {
    for (g, b) in [(10i32, 4i32), (-3, 0), (i32::MAX, 9), (i32::MIN, 5)] {
        let op = Op::Driver(g, b);
        assert_same(&op);
        let expected = driver_transcript(&good_rejected(), &ten_lines(Some(b as usize)));
        for lib in [c_lib(), rust_lib()] {
            assert_eq!(output(lib, &op), expected, "{} lib driver({g},{b})", lib.name);
        }
    }
}

/// ERRORS row 19 — only `bad`'s check rejects.
#[test]
fn err_19_driver_only_bad_rejects() {
    for (g, b) in [(0i32, -1i32), (7, i32::MIN), (9, -12345)] {
        let op = Op::Driver(g, b);
        assert_same(&op);
        let mut good_block = g2b_block();
        good_block.extend_from_slice(&ten_lines(Some(g as usize)));
        let expected = driver_transcript(&good_block, ERR_NEGATIVE);
        for lib in [c_lib(), rust_lib()] {
            assert_eq!(output(lib, &op), expected, "{} lib driver({g},{b})", lib.name);
        }
    }
}

/// ERRORS row 20 — extreme ints for both parameters across the FFI boundary.
#[test]
fn err_20_driver_int_min_both() {
    let ops: Vec<Op> = [
        (i32::MIN, i32::MIN),
        (i32::MIN + 1, i32::MIN + 1),
        (i32::MAX, i32::MIN),
    ]
    .iter()
    .map(|&(g, b)| Op::Driver(g, b))
    .collect();
    assert_same_batch(&ops);
    let expected = driver_transcript(&good_rejected(), ERR_NEGATIVE);
    for lib in [c_lib(), rust_lib()] {
        for (out, op) in outputs(lib, &ops).iter().zip(&ops) {
            assert_eq!(out, &expected, "{} lib {}", lib.name, op.describe());
        }
    }
}

/// ERRORS row 21 — non-NULL pointer to an immediate NUL: prints just "\n".
#[test]
fn err_21_print_line_empty() {
    let op = Op::PrintLine(vec![]);
    assert_same(&op);
    for lib in [c_lib(), rust_lib()] {
        assert_eq!(output(lib, &op), b"\n", "{} lib", lib.name);
    }
}

/// ERRORS row 22 — interior pointer / embedded NUL truncation.
#[test]
fn err_22_print_line_embedded_nul() {
    let buf = b"abc\0def\0".to_vec();
    let cases: [(usize, &[u8]); 5] = [
        (0, b"abc\n"),
        (1, b"bc\n"),
        (3, b"\n"),
        (4, b"def\n"),
        (7, b"\n"),
    ];
    let ops: Vec<Op> = cases
        .iter()
        .map(|&(off, _)| Op::PrintLineRaw(buf.clone(), off))
        .collect();
    assert_same_batch(&ops);
    for lib in [c_lib(), rust_lib()] {
        for (out, (off, want)) in outputs(lib, &ops).iter().zip(cases) {
            assert_eq!(out, want, "{} lib printLine(buf+{off})", lib.name);
        }
    }
}

/// ERRORS row 23 — a format-specifier-looking payload is data, not a format.
#[test]
fn err_23_print_line_format_like_data() {
    let strings: [&[u8]; 6] = [b"%s", b"%n", b"%d %d %d", b"%%%%", b"%1000000000d", b"%hn%p"];
    let ops: Vec<Op> = strings.iter().map(|s| Op::PrintLine(s.to_vec())).collect();
    assert_same_batch(&ops);
    for lib in [c_lib(), rust_lib()] {
        for (out, s) in outputs(lib, &ops).iter().zip(strings) {
            let mut want = s.to_vec();
            want.push(b'\n');
            assert_eq!(out, &want, "{} lib printLine({:?})", lib.name, s);
        }
    }
}

/// ERRORS row 24 — extreme / "out-of-range" integer inputs.  This API has no
/// enums, so the invalid-enum-variant class degenerates to arbitrary `int`s:
/// they are all valid `%d` inputs and must format identically.
#[test]
fn err_24_print_int_line_extremes() {
    let values = [0, -1, 1, i32::MIN, i32::MIN + 1, i32::MAX, i32::MAX - 1];
    let ops: Vec<Op> = values.iter().map(|&n| Op::PrintIntLine(n)).collect();
    assert_same_batch(&ops);
    for lib in [c_lib(), rust_lib()] {
        for (out, n) in outputs(lib, &ops).iter().zip(values) {
            assert_eq!(out, &format!("{n}\n").into_bytes(), "{} lib", lib.name);
        }
    }
    // bit patterns that are only "positive" when reinterpreted as unsigned
    let ops2: Vec<Op> = [0x8000_0000u32, 0xFFFF_FFFF, 0x7FFF_FFFF, 0xDEAD_BEEF]
        .iter()
        .map(|&u| Op::PrintIntLine(u as i32))
        .collect();
    assert_same_batch(&ops2);
}
