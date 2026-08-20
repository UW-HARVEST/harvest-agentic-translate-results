//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`.  Every test loads BOTH shared objects with
//! `libloading` and calls them only through their exported C symbols, then
//! compares the stdout bytes produced by each individual call.
//!
//! Order follows the call hierarchy: the lowest-level entry points first
//! (`printLine`, `printIntLine`), then `bad` / `good` (the latter drives the two
//! `static` helpers), then the composed `driver` pipeline, then sequencing,
//! statelessness, composition and randomised mixed-call programs.

mod common;
use common::*;

// ---------------------------------------------------------------------------
// Rows 1–8: printLine (lowest level)
// ---------------------------------------------------------------------------

/// CONFIGS row 1 — empty string.
#[test]
fn cfg_01_print_line_empty() {
    assert_same(&Op::PrintLine(vec![]));
}

/// CONFIGS row 2 — every single non-NUL byte value.
#[test]
fn cfg_02_print_line_single_byte_all_values() {
    let ops: Vec<Op> = (1u8..=255).map(|b| Op::PrintLine(vec![b])).collect();
    assert_same_batch(&ops);
}

/// CONFIGS row 3 — randomised printable ASCII, length 1..=64.
#[test]
fn cfg_03_print_line_random_ascii() {
    let mut rng = Rng::new(0x5EED_0003);
    let ops: Vec<Op> = (0..400)
        .map(|_| {
            let len = 1 + (rng.next_u32() % 64) as usize;
            Op::PrintLine((0..len).map(|_| 0x20 + rng.next_byte() % 0x5f).collect())
        })
        .collect();
    assert_same_batch(&ops);
}

/// CONFIGS row 4 — randomised arbitrary non-NUL bytes (invalid UTF-8 included).
#[test]
fn cfg_04_print_line_random_bytes() {
    let mut rng = Rng::new(0x5EED_0004);
    let ops: Vec<Op> = (0..400)
        .map(|_| {
            let len = 1 + (rng.next_u32() % 256) as usize;
            Op::PrintLine(
                (0..len)
                    .map(|_| {
                        let b = rng.next_byte();
                        if b == 0 { 1 } else { b }
                    })
                    .collect(),
            )
        })
        .collect();
    assert_same_batch(&ops);
}

/// CONFIGS row 5 — the payload looks like a printf format string.
#[test]
fn cfg_05_print_line_format_specifiers() {
    let ops: Vec<Op> = [
        &b"%s"[..],
        b"%d",
        b"%n",
        b"%%",
        b"%s%s%s%s%s%s%s%s",
        b"%n%n%n%n",
        b"100%% done: %p %x %lf %999999999d",
        b"ERROR: %s is negative.",
    ]
    .iter()
    .map(|s| Op::PrintLine(s.to_vec()))
    .collect();
    assert_same_batch(&ops);
}

/// CONFIGS row 6 — embedded newlines / CR / tabs / control bytes.
#[test]
fn cfg_06_print_line_embedded_newlines() {
    let ops: Vec<Op> = [
        &b"\n"[..],
        b"a\nb",
        b"\n\n\n",
        b"line1\nline2\n",
        b"\r\n",
        b"tab\there",
        b"trailing\n",
        b"\x0b\x0c\x1b[31m",
    ]
    .iter()
    .map(|s| Op::PrintLine(s.to_vec()))
    .collect();
    assert_same_batch(&ops);
}

/// CONFIGS row 7 — lengths straddling libc's stdout buffer size.
#[test]
fn cfg_07_print_line_buffer_boundary_lengths() {
    let ops: Vec<Op> = [
        1usize, 2, 63, 64, 127, 128, 511, 512, 1023, 1024, 4095, 4096, 4097, 8190, 8191, 8192,
        8193, 16384, 65536,
    ]
    .iter()
    .map(|&len| Op::PrintLine((0..len).map(|i| b'a' + (i % 26) as u8).collect()))
    .collect();
    assert_same_batch(&ops);
    // and again with stdout unbuffered, so the flush points differ
    assert_same_batch_buffered(&ops, Buffering::Unbuffered);
}

/// CONFIGS row 8 — interior pointer + embedded NUL truncation.
#[test]
fn cfg_08_print_line_interior_pointer() {
    let buf = b"HEAD\0TAIL\0more\0".to_vec();
    let mut ops: Vec<Op> = [0usize, 1, 4, 5, 9, 10, 14]
        .iter()
        .map(|&off| Op::PrintLineRaw(buf.clone(), off))
        .collect();
    // A long buffer whose NUL is far from the start, addressed from the middle.
    let mut long = vec![b'z'; 5000];
    long[4000] = 0;
    long.push(0);
    for off in [0usize, 1, 2500, 3999, 4000, 4001] {
        ops.push(Op::PrintLineRaw(long.clone(), off));
    }
    assert_same_batch(&ops);
}

// ---------------------------------------------------------------------------
// Rows 9–11: printIntLine (lowest level)
// ---------------------------------------------------------------------------

/// CONFIGS row 9 — fixed integer shapes.
#[test]
fn cfg_09_print_int_line_fixed_shapes() {
    let ops: Vec<Op> = [
        0i32,
        1,
        -1,
        9,
        -9,
        10,
        -10,
        99999,
        -99999,
        i32::MAX,
        i32::MIN,
        i32::MIN + 1,
        i32::MAX - 1,
    ]
    .iter()
    .map(|&n| Op::PrintIntLine(n))
    .collect();
    assert_same_batch(&ops);
}

/// CONFIGS row 10 — uniformly random ints across the whole 32-bit range.
#[test]
fn cfg_10_print_int_line_random_full_range() {
    let mut rng = Rng::new(0x5EED_0010);
    let ops: Vec<Op> = (0..3000).map(|_| Op::PrintIntLine(rng.next_i32())).collect();
    assert_same_batch(&ops);
}

/// CONFIGS row 11 — one value per decimal digit width, both signs.
#[test]
fn cfg_11_print_int_line_digit_widths() {
    let mut ops = Vec::new();
    let mut v = 1i64;
    for _ in 0..10 {
        let n = v.min(i32::MAX as i64) as i32;
        ops.push(Op::PrintIntLine(n));
        ops.push(Op::PrintIntLine(-n));
        ops.push(Op::PrintIntLine(n.wrapping_sub(1)));
        v *= 10;
    }
    assert_same_batch(&ops);
}

// ---------------------------------------------------------------------------
// Rows 12–15: bad()
// ---------------------------------------------------------------------------

/// CONFIGS row 12 — every in-bounds index, exhaustively.
#[test]
fn cfg_12_bad_all_in_bounds() {
    let ops: Vec<Op> = (0..10).map(Op::Bad).collect();
    assert_same_batch(&ops);
    // cross-check both libraries against the model of the C source
    for lib in [c_lib(), rust_lib()] {
        for (i, out) in outputs(lib, &ops).iter().enumerate() {
            assert_eq!(out, &ten_lines(Some(i)), "{} lib bad({i})", lib.name);
        }
    }
}

/// CONFIGS row 13 — randomised negative `data` (the rejecting branch).
#[test]
fn cfg_13_bad_random_negative() {
    let mut rng = Rng::new(0x5EED_0013);
    let ops: Vec<Op> = (0..300)
        .map(|_| Op::Bad(rng.range_i32(i32::MIN, -1)))
        .collect();
    assert_same_batch(&ops);
}

/// CONFIGS row 14 — out-of-bounds sweep 10..=64 (deliberate UB store).
#[test]
fn cfg_14_bad_oob_sweep() {
    for d in 10..=64 {
        assert_same_stdout_ub(&Op::Bad(d));
    }
}

/// CONFIGS row 15 — randomised large out-of-bounds indices (deliberate UB).
#[test]
fn cfg_15_bad_oob_random() {
    let mut rng = Rng::new(0x5EED_0015);
    for _ in 0..60 {
        assert_same_stdout_ub(&Op::Bad(rng.range_i32(10, i32::MAX)));
    }
    for d in [10, 11, 1 << 20, i32::MAX - 1, i32::MAX] {
        assert_same_stdout_ub(&Op::Bad(d));
    }
}

// ---------------------------------------------------------------------------
// Rows 16–18: good() (drives the two `static` helpers)
// ---------------------------------------------------------------------------

/// CONFIGS row 16 — every accepted index, exhaustively.
#[test]
fn cfg_16_good_all_in_bounds() {
    let ops: Vec<Op> = (0..10).map(Op::Good).collect();
    assert_same_batch(&ops);
    for lib in [c_lib(), rust_lib()] {
        for (i, out) in outputs(lib, &ops).iter().enumerate() {
            let mut expected = ten_lines(Some(7)); // goodG2B: data == 7
            expected.extend_from_slice(&ten_lines(Some(i))); // goodB2G: data == i
            assert_eq!(out, &expected, "{} lib good({i})", lib.name);
        }
    }
}

/// CONFIGS row 17 — rejected `data` values, fixed and randomised.
#[test]
fn cfg_17_good_out_of_range() {
    let mut ops: Vec<Op> = [
        -1,
        i32::MIN,
        i32::MIN + 1,
        10,
        11,
        12,
        100,
        i32::MAX - 1,
        i32::MAX,
    ]
    .iter()
    .map(|&d| Op::Good(d))
    .collect();
    let mut rng = Rng::new(0x5EED_0017);
    for _ in 0..300 {
        let d = if rng.next_u64() & 1 == 0 {
            rng.range_i32(i32::MIN, -1)
        } else {
            rng.range_i32(10, i32::MAX)
        };
        ops.push(Op::Good(d));
    }
    assert_same_batch(&ops);
}

/// CONFIGS row 18 — randomised over the whole int range (mixes both classes).
#[test]
fn cfg_18_good_random_full_range() {
    let mut rng = Rng::new(0x5EED_0018);
    let ops: Vec<Op> = (0..600)
        .map(|_| {
            // bias towards the interesting window while still covering everything
            Op::Good(match rng.next_u64() % 4 {
                0 => rng.range_i32(-12, 12),
                1 => rng.range_i32(-1000, 1000),
                _ => rng.next_i32(),
            })
        })
        .collect();
    assert_same_batch(&ops);
}

// ---------------------------------------------------------------------------
// Rows 19–22: driver() (composed pipeline)
// ---------------------------------------------------------------------------

const GOOD_VALUES: [i32; 8] = [-1, i32::MIN, 0, 1, 7, 9, 10, i32::MAX];
const BAD_SAFE_VALUES: [i32; 8] = [-1, i32::MIN, i32::MIN + 1, 0, 1, 5, 7, 9];

/// CONFIGS row 19 — full cross product over the well-defined domain.
#[test]
fn cfg_19_driver_cross_product() {
    let mut ops = Vec::new();
    for g in GOOD_VALUES {
        for b in BAD_SAFE_VALUES {
            ops.push(Op::Driver(g, b));
        }
    }
    assert_same_batch(&ops);
}

/// CONFIGS row 20 — cross product with out-of-bounds `badData` (UB store).
#[test]
fn cfg_20_driver_cross_product_oob_bad() {
    for g in GOOD_VALUES {
        for b in [10, 11, 12, 20, 40, i32::MAX] {
            assert_same_stdout_ub(&Op::Driver(g, b));
        }
    }
}

/// CONFIGS row 21 — randomised, both arguments in the accepted range.
#[test]
fn cfg_21_driver_random_valid() {
    let mut rng = Rng::new(0x5EED_0021);
    let ops: Vec<Op> = (0..300)
        .map(|_| Op::Driver(rng.range_i32(0, 9), rng.range_i32(0, 9)))
        .collect();
    assert_same_batch(&ops);
}

/// CONFIGS row 22 — randomised over the whole int range for `goodData`, and over
/// the well-defined domain for `badData` (`badData >= 10` is row 20).
#[test]
fn cfg_22_driver_random_full_range() {
    let mut rng = Rng::new(0x5EED_0022);
    let ops: Vec<Op> = (0..400)
        .map(|_| {
            let g = match rng.next_u64() % 3 {
                0 => rng.range_i32(-12, 12),
                1 => rng.range_i32(-100_000, 100_000),
                _ => rng.next_i32(),
            };
            let b = match rng.next_u64() % 3 {
                0 => rng.range_i32(-12, 9),
                1 => rng.range_i32(i32::MIN, -1),
                _ => rng.range_i32(0, 9),
            };
            Op::Driver(g, b)
        })
        .collect();
    assert_same_batch(&ops);
}

// ---------------------------------------------------------------------------
// Rows 23–27: sequencing, buffering, statelessness, composition, fuzz
// ---------------------------------------------------------------------------

fn mixed_sequence() -> Vec<Op> {
    vec![
        Op::PrintLine(b"header".to_vec()),
        Op::PrintIntLine(-42),
        Op::Bad(3),
        Op::Good(9),
        Op::PrintLineNull,
        Op::Bad(-7),
        Op::Good(-1),
        Op::PrintIntLine(i32::MIN),
        Op::Driver(4, 8),
        Op::PrintLine(b"footer".to_vec()),
    ]
}

/// CONFIGS row 23 — interleaved calls, one stdout stream, block buffered.
#[test]
fn cfg_23_interleaved_sequence_block_buffered() {
    assert_same_batch_buffered(&mixed_sequence(), Buffering::Block);
}

/// CONFIGS row 24 — the same sequence with stdout unbuffered.
#[test]
fn cfg_24_interleaved_sequence_unbuffered() {
    assert_same_batch_buffered(&mixed_sequence(), Buffering::Unbuffered);
}

/// CONFIGS row 25 — no residual state between calls: the same call repeated 50×
/// produces 50 identical blocks in both libraries.
#[test]
fn cfg_25_repeated_invocation_no_state() {
    for op in [
        Op::Bad(5),
        Op::Good(2),
        Op::Driver(7, 7),
        Op::PrintIntLine(123),
        Op::PrintLine(b"x".to_vec()),
    ] {
        let seq: Vec<Op> = std::iter::repeat_n(op.clone(), 50).collect();
        assert_same_batch(&seq);
        for lib in [c_lib(), rust_lib()] {
            let blocks = outputs(lib, &seq);
            let first = blocks[0].clone();
            for (i, b) in blocks.iter().enumerate() {
                assert_eq!(
                    b,
                    &first,
                    "{} lib: call #{i} of {} differs from the first — state leaked",
                    lib.name,
                    op.describe()
                );
            }
        }
    }
}

/// CONFIGS row 26 — `driver()` must equal the hand-composed pipeline in *both*
/// libraries (verifies the composed order of the low-level calls, not just the
/// wrapper's own output).
#[test]
fn cfg_26_driver_equals_composition() {
    let mut rng = Rng::new(0x5EED_0026);
    let mut cases: Vec<(i32, i32)> = vec![(7, 3), (-1, -1), (10, 0), (0, 9), (i32::MIN, i32::MIN)];
    for _ in 0..40 {
        cases.push((rng.range_i32(-20, 20), rng.range_i32(-20, 9)));
    }
    for (g, b) in cases {
        let whole = vec![Op::Driver(g, b)];
        let composed = vec![
            Op::PrintLine(b"Calling good()...".to_vec()),
            Op::Good(g),
            Op::PrintLine(b"Finished good()".to_vec()),
            Op::PrintLine(b"Calling bad()...".to_vec()),
            Op::Bad(b),
            Op::PrintLine(b"Finished bad()".to_vec()),
        ];
        for lib in [c_lib(), rust_lib()] {
            let a = output(lib, &whole[0]);
            let parts = outputs(lib, &composed).concat();
            assert_eq!(
                a, parts,
                "{} lib: driver({g},{b}) != composition of its parts",
                lib.name
            );
        }
        assert_same_batch(&whole);
        assert_same_batch(&composed);
    }
}

/// CONFIGS row 29 — both `.so`s loaded in one process, calls interleaved into a
/// single stdout stream.  Each call must produce exactly the bytes it produces
/// when its library runs alone, which also proves that neither library's
/// internal `printLine`/`printIntLine` calls get interposed by the other
/// library's identically named exports.
#[test]
fn cfg_29_both_libraries_interleaved_in_one_process() {
    let ops = mixed_sequence();
    let c_alone = outputs(c_lib(), &ops);
    let r_alone = outputs(rust_lib(), &ops);

    for flip in [false, true] {
        let pairs: Vec<(&Driver, &Op)> = ops
            .iter()
            .enumerate()
            .map(|(i, op)| {
                let take_c = (i % 2 == 0) != flip;
                (if take_c { c_lib() } else { rust_lib() }, op)
            })
            .collect();
        let mixed = run_batch_pairs(&pairs, Buffering::Block);
        assert_eq!(mixed.exit, Exit::Code(0), "interleaved run terminated badly");
        assert_eq!(mixed.completed(), ops.len(), "interleaved run stopped early");
        let mut expected_stream = Vec::new();
        for (i, op) in ops.iter().enumerate() {
            let take_c = (i % 2 == 0) != flip;
            let expected = if take_c { &c_alone[i] } else { &r_alone[i] };
            assert_eq!(
                &mixed.outputs[i],
                expected,
                "{} lib produced different bytes for {} when both libraries are loaded",
                if take_c { "C" } else { "Rust" },
                op.describe()
            );
            expected_stream.extend_from_slice(expected);
        }
        assert_eq!(mixed.full, expected_stream, "interleaved transcript differs");
    }
}

/// CONFIGS row 27 — property-style fuzz: random programs of mixed calls over all
/// five entry points.
#[test]
fn cfg_27_random_call_program() {
    let mut rng = Rng::new(0x5EED_0027);
    for _ in 0..150 {
        let n = 1 + (rng.next_u32() % 12) as usize;
        let mut ops = Vec::with_capacity(n);
        for _ in 0..n {
            ops.push(match rng.next_u64() % 6 {
                0 => Op::PrintLineNull,
                1 => {
                    let len = (rng.next_u32() % 40) as usize;
                    Op::PrintLine(
                        (0..len)
                            .map(|_| {
                                let b = rng.next_byte();
                                if b == 0 { b'.' } else { b }
                            })
                            .collect(),
                    )
                }
                2 => Op::PrintIntLine(rng.next_i32()),
                3 => Op::Bad(rng.range_i32(i32::MIN, 9)),
                4 => Op::Good(rng.next_i32()),
                _ => Op::Driver(rng.next_i32(), rng.range_i32(-20, 9)),
            });
        }
        assert_same_batch(&ops);
    }
}
