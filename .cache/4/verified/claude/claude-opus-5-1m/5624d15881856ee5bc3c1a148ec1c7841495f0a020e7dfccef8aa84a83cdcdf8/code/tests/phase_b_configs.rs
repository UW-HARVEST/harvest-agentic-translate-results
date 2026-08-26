//! Phase B — valid-path differential tests, one test per row of CONFIGS.md.
//!
//! Both libraries are loaded as shared objects and driven only through their
//! exported C symbols. Rows with a value domain use many seeded-random inputs
//! rather than one hand-picked value.

mod harness;

use harness::*;
use std::ffi::c_char;

// ===========================================================================
// C1 — printHexCharLine over the exhaustive `char` domain.
// ===========================================================================

#[test]
fn c1_print_hex_char_line_exhaustive_char_domain() {
    // The whole domain of the platform `char`: every one of the 256 bit
    // patterns, which covers zero, the 1-hex-digit range, the 2-hex-digit
    // range, CHAR_MIN, CHAR_MAX and every sign-extended negative value.
    for raw in 0u16..=255 {
        let v = raw as u8 as c_char;
        assert_same(&format!("C1 printHexCharLine({v})"), |api| unsafe {
            (api.print_hex_char_line)(v)
        });
    }
}

#[test]
fn c1b_print_hex_char_line_domain_boundaries_are_exactly_as_c_formats_them() {
    // Pin the actual bytes so "both agree" cannot mean "both wrong the same
    // way". Values verified against the compiled C library.
    let cases: &[(i32, &[u8])] = &[
        (0, b"00\n"),
        (1, b"01\n"),
        (9, b"09\n"),
        (15, b"0f\n"),
        (16, b"10\n"),
        (127, b"7f\n"), // CHAR_MAX
        (-1, b"ffffffff\n"),
        (-2, b"fffffffe\n"),
        (-16, b"fffffff0\n"),
        (-128, b"ffffff80\n"), // CHAR_MIN
    ];
    for &(v, expected) in cases {
        let v = v as i8 as c_char;
        assert_same_and_eq(
            &format!("C1b printHexCharLine({v})"),
            expected,
            |api| unsafe { (api.print_hex_char_line)(v) },
        );
    }
}

// ===========================================================================
// C2 — printHexCharLine, randomized.
// ===========================================================================

#[test]
fn c2_print_hex_char_line_randomized() {
    let mut rng = Rng::new(SEED ^ 0xC002);
    for i in 0..4096 {
        let v = rng.next_c_char();
        assert_same(&format!("C2 #{i} printHexCharLine({v})"), |api| unsafe {
            (api.print_hex_char_line)(v)
        });
    }
}

#[test]
fn c2b_print_hex_char_line_batched_without_intermediate_flush() {
    // Many calls inside a single capture window: exercises the stdio buffer
    // being filled and refilled by repeated small writes.
    let mut rng = Rng::new(SEED ^ 0xC002_B);
    let vals: Vec<c_char> = (0..2000).map(|_| rng.next_c_char()).collect();
    assert_same("C2b printHexCharLine x2000 batched", |api| {
        for &v in &vals {
            unsafe { (api.print_hex_char_line)(v) }
        }
    });
}

// ===========================================================================
// C3 — printLine, printable ASCII, lengths 0..=64.
// ===========================================================================

#[test]
fn c3_print_line_random_printable_ascii() {
    let mut rng = Rng::new(SEED ^ 0xC003);
    for i in 0..512 {
        let len = rng.below(65) as usize; // includes the empty string
        let payload = rng.next_ascii(len);
        assert_same(&format!("C3 #{i} printLine(len={len})"), |api| {
            call_print_line(api, &payload)
        });
    }
}

// ===========================================================================
// C4 — printLine, arbitrary NUL-free bytes (high-bit / non-UTF-8 / `%` / \n).
// ===========================================================================

#[test]
fn c4_print_line_random_arbitrary_bytes() {
    let mut rng = Rng::new(SEED ^ 0xC004);
    for i in 0..512 {
        let len = 1 + rng.below(256) as usize;
        let payload = rng.next_bytes(len);
        assert_same(&format!("C4 #{i} printLine(len={len}, raw bytes)"), |api| {
            call_print_line(api, &payload)
        });
    }
}

#[test]
fn c4b_print_line_shapes_the_c_special_cases() {
    // Every distinct payload SHAPE the `%s` conversion could plausibly treat
    // differently, hand-enumerated in addition to the random sweep.
    let cases: Vec<Vec<u8>> = vec![
        b"".to_vec(),
        b"a".to_vec(),
        b" ".to_vec(),
        b"\n".to_vec(),
        b"\n\n\n".to_vec(),
        b"line1\nline2".to_vec(),
        b"trailing\n".to_vec(),
        b"%s".to_vec(),
        b"%n".to_vec(),
        b"%d %i %x %p %%".to_vec(),
        b"%s%s%s%s%s%s%s%s".to_vec(),
        b"100%".to_vec(),
        b"\t\r\x0b\x0c\x07\x1b".to_vec(),
        b"\x01\x02\x03".to_vec(),
        b"\x7f".to_vec(),
        b"\x80".to_vec(),
        b"\xff".to_vec(),
        b"\xff\xfe\x80\x81".to_vec(),
        b"\xc3\x28".to_vec(),         // invalid UTF-8
        b"\xe2\x82".to_vec(),         // truncated UTF-8
        b"caf\xc3\xa9".to_vec(),      // valid UTF-8
        b"\xed\xa0\x80".to_vec(),     // UTF-8-encoded surrogate
        (1u8..=255).collect(),        // every non-NUL byte, in order
        (1u8..=255).rev().collect(),  // ... and reversed
        b"data value is too large to perform arithmetic safely.".to_vec(),
    ];
    for (i, payload) in cases.iter().enumerate() {
        // The C is `printf("%s\n", line)`: the payload is an *argument*, so it
        // must appear literally, with exactly one newline appended.
        let mut expected = payload.clone();
        expected.push(b'\n');
        assert_same_and_eq(&format!("C4b #{i} printLine shape"), &expected, |api| {
            call_print_line(api, payload)
        });
    }
}

// ===========================================================================
// C5 — printLine at and around the stdio buffering boundary.
// ===========================================================================

#[test]
fn c5_print_line_lengths_across_stdio_buffer_boundary() {
    let mut rng = Rng::new(SEED ^ 0xC005);
    for &len in &[
        1usize, 2, 3, 127, 128, 255, 256, 511, 512, 1023, 1024, 1025, 4095, 4096, 4097, 8191, 8192,
        8193, 16384, 65536,
    ] {
        let payload = rng.next_ascii(len);
        let mut expected = payload.clone();
        expected.push(b'\n');
        assert_same_and_eq(&format!("C5 printLine(len={len})"), &expected, |api| {
            call_print_line(api, &payload)
        });
    }
}

// ===========================================================================
// C6/C7/C8 — bad(): nullary, stateless, must not trap on the CWE-190 overflow.
// ===========================================================================

#[test]
fn c6_bad_single_and_repeated() {
    // data = CHAR_MAX (127) -> 127 > 0 -> (char)(127*2) == (char)254 == -2
    // -> printf("%02x\n", (int)-2) -> "fffffffe\n".
    assert_same_and_eq("C6 bad() once", b"fffffffe\n", |api| unsafe { (api.bad)() });

    for n in [2usize, 3, 8, 64] {
        let expected: Vec<u8> = b"fffffffe\n".repeat(n);
        assert_same_and_eq(&format!("C6 bad() x{n}"), &expected, |api| {
            for _ in 0..n {
                unsafe { (api.bad)() }
            }
        });
    }
}

#[test]
fn c7_c8_bad_overflow_semantics_in_this_profile() {
    // Row C7 (release: panic="abort", overflow checks off) and row C8 (dev:
    // overflow checks ON) are the same assertion run under the two profiles;
    // `profile()` records which one this execution covers. A Rust translation
    // that used checked/saturating arithmetic instead of a wrapping `as` cast
    // would abort or print "7f"/"fe" here.
    let out = assert_same(&format!("C7/C8 bad() in {} profile", profile()), |api| {
        unsafe { (api.bad)() }
    });
    assert_eq!(
        out, b"fffffffe\n",
        "signed-char truncation must wrap in the {} profile",
        profile()
    );
}

// ===========================================================================
// C9 — good(): the composed goodG2B + goodB2G pipeline.
// ===========================================================================

#[test]
fn c9_good_composed_pipeline() {
    // goodG2B: data = 2 -> "04\n"
    // goodB2G: data = CHAR_MAX -> 127 < 63 is false -> the diagnostic line.
    let expected: &[u8] = b"04\ndata value is too large to perform arithmetic safely.\n";
    assert_same_and_eq("C9 good() once", expected, |api| unsafe { (api.good)() });

    for n in [2usize, 3, 16] {
        let rep = expected.repeat(n);
        assert_same_and_eq(&format!("C9 good() x{n}"), &rep, |api| {
            for _ in 0..n {
                unsafe { (api.good)() }
            }
        });
    }
}

// ===========================================================================
// C10 — driver(0) -> bad().
// ===========================================================================

#[test]
fn c10_driver_zero_selects_bad() {
    assert_same_and_eq("C10 driver(0)", b"fffffffe\n", |api| unsafe {
        (api.driver)(0)
    });
}

// ===========================================================================
// C11 — driver(nonzero) -> good(), incl. zero-low-byte and extreme values.
// ===========================================================================

#[test]
fn c11_driver_nonzero_selects_good_interesting_values() {
    let expected: &[u8] = b"04\ndata value is too large to perform arithmetic safely.\n";
    let vals: &[i32] = &[
        1,
        -1,
        2,
        3,
        42,
        0x7f,
        0x80,
        0xff,
        // Nonzero ints whose LOW BYTE is zero: `if (useGood)` tests the whole
        // int, so these must all pick good().
        256,
        512,
        0x1_0000,
        0x0100_0000,
        0xFFFF_FF00u32 as i32,
        -256,
        -65536,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
        -7,
    ];
    for &v in vals {
        assert_same_and_eq(&format!("C11 driver({v})"), expected, |api| unsafe {
            (api.driver)(v)
        });
    }
}

#[test]
fn c11b_driver_randomized_nonzero() {
    let mut rng = Rng::new(SEED ^ 0xC011);
    let expected: &[u8] = b"04\ndata value is too large to perform arithmetic safely.\n";
    for i in 0..1024 {
        let mut v = rng.next_i32();
        if v == 0 {
            v = 1;
        }
        assert_same_and_eq(&format!("C11b #{i} driver({v})"), expected, |api| unsafe {
            (api.driver)(v)
        });
    }
}

// ===========================================================================
// C12 — driver over unconstrained random i32 (both truthiness classes mixed).
// ===========================================================================

#[test]
fn c12_driver_randomized_unconstrained() {
    let mut rng = Rng::new(SEED ^ 0xC012);
    let mut saw_zero = false;
    for i in 0..2048 {
        // Bias 1-in-8 towards 0 so the false branch is genuinely hit by the
        // random stream rather than only by the hand-written row C10.
        let v = if rng.below(8) == 0 { 0 } else { rng.next_i32() };
        saw_zero |= v == 0;
        assert_same(&format!("C12 #{i} driver({v})"), |api| unsafe {
            (api.driver)(v)
        });
    }
    assert!(saw_zero, "the random stream must exercise driver(0) too");
}

// ===========================================================================
// C13 — dispatch equivalence: driver(0) == bad(), driver(k!=0) == good().
// ===========================================================================

#[test]
fn c13_driver_dispatch_equivalence() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 0xC013);

    for lib in [&p.c, &p.rust] {
        let via_bad = capture(|| unsafe { (lib.bad)() });
        let via_driver0 = capture(|| unsafe { (lib.driver)(0) });
        assert_eq!(
            via_bad, via_driver0,
            "[{}] driver(0) must be exactly bad()",
            lib.name
        );

        let via_good = capture(|| unsafe { (lib.good)() });
        for _ in 0..64 {
            let mut k = rng.next_i32();
            if k == 0 {
                k = 1;
            }
            let via_driver_k = capture(|| unsafe { (lib.driver)(k) });
            assert_eq!(
                via_good, via_driver_k,
                "[{}] driver({k}) must be exactly good()",
                lib.name
            );
        }
    }
}

// ===========================================================================
// C14 — good() decomposes into the low-level entry points.
// ===========================================================================

#[test]
fn c14_good_decomposes_into_low_level_entry_points() {
    let p = pair();
    for lib in [&p.c, &p.rust] {
        let composed = capture(|| unsafe { (lib.good)() });
        let by_hand = capture(|| {
            // goodG2B: data = 2, result = (char)(2*2)
            unsafe { (lib.print_hex_char_line)(4) };
            // goodB2G: rejects the arithmetic and emits this exact text
            call_print_line(lib, b"data value is too large to perform arithmetic safely.");
        });
        assert_eq!(
            composed, by_hand,
            "[{}] good() must equal printHexCharLine(4) then the diagnostic printLine",
            lib.name
        );
    }
}

#[test]
fn c14b_bad_decomposes_into_low_level_entry_point() {
    let p = pair();
    for lib in [&p.c, &p.rust] {
        let composed = capture(|| unsafe { (lib.bad)() });
        let by_hand = capture(|| unsafe { (lib.print_hex_char_line)(-2i32 as i8 as _) });
        assert_eq!(
            composed, by_hand,
            "[{}] bad() must equal printHexCharLine((char)(CHAR_MAX*2))",
            lib.name
        );
    }
}

// ===========================================================================
// C15 — long random interleaved sequence over all five exports, one capture.
// ===========================================================================

/// One step of a random call sequence over the whole public surface.
#[derive(Clone, Debug)]
enum Op {
    PrintLine(Vec<u8>),
    PrintLineNull,
    PrintHex(c_char),
    Bad,
    Good,
    Driver(i32),
}

fn gen_ops(rng: &mut Rng, n: usize) -> Vec<Op> {
    (0..n)
        .map(|_| match rng.below(6) {
            0 => {
                let len = rng.below(48) as usize;
                Op::PrintLine(rng.next_bytes(len))
            }
            1 => Op::PrintLineNull,
            2 => Op::PrintHex(rng.next_c_char()),
            3 => Op::Bad,
            4 => Op::Good,
            _ => Op::Driver(if rng.below(4) == 0 { 0 } else { rng.next_i32() }),
        })
        .collect()
}

fn run_op(api: &Api, op: &Op) {
    unsafe {
        match op {
            Op::PrintLine(b) => call_print_line(api, b),
            Op::PrintLineNull => (api.print_line)(std::ptr::null()),
            Op::PrintHex(v) => (api.print_hex_char_line)(*v),
            Op::Bad => (api.bad)(),
            Op::Good => (api.good)(),
            Op::Driver(v) => (api.driver)(*v),
        }
    }
}

#[test]
fn c15_random_interleaved_sequence_single_stream() {
    let mut rng = Rng::new(SEED ^ 0xC015);
    let ops = gen_ops(&mut rng, 3000);
    // One contiguous stdout stream per library, with no intermediate flush:
    // catches divergence in buffering or residual stream state that per-call
    // tests cannot see.
    assert_same("C15 3000-op random sequence, one stream", |api| {
        for op in &ops {
            run_op(api, op)
        }
    });
}

#[test]
fn c15b_many_shorter_random_sequences() {
    let mut rng = Rng::new(SEED ^ 0xC015_B);
    for i in 0..128 {
        let n = 1 + rng.below(24) as usize;
        let ops = gen_ops(&mut rng, n);
        assert_same(&format!("C15b #{i} sequence of {} ops", ops.len()), |api| {
            for op in &ops {
                run_op(api, op)
            }
        });
    }
}

// ===========================================================================
// C16 — C and Rust interleaved into ONE shared stdout stream.
// ===========================================================================

#[test]
fn c16_c_and_rust_interleaved_into_one_shared_stream() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 0xC016);
    let ops = gen_ops(&mut rng, 400);

    // Per-op reference output, taken from the C library alone.
    let per_op: Vec<Vec<u8>> = ops.iter().map(|op| capture(|| run_op(&p.c, op))).collect();

    // Now drive both libraries alternately through the SAME libc `FILE`
    // inside a single capture window. If both agree and neither leaves the
    // stream in a different state, the result is each op's bytes twice.
    let interleaved = capture(|| {
        for op in &ops {
            run_op(&p.c, op);
            run_op(&p.rust, op);
        }
    });

    let mut expected = Vec::new();
    for chunk in &per_op {
        expected.extend_from_slice(chunk);
        expected.extend_from_slice(chunk);
    }
    assert_eq!(
        interleaved.len(),
        expected.len(),
        "C16 interleaved stream length mismatch"
    );
    assert!(
        interleaved == expected,
        "C16 interleaved C/Rust stream diverged from the doubled C reference"
    );

    // And the mirror ordering (Rust first), to rule out order sensitivity.
    let interleaved_rev = capture(|| {
        for op in &ops {
            run_op(&p.rust, op);
            run_op(&p.c, op);
        }
    });
    assert!(
        interleaved_rev == expected,
        "C16 reversed interleaving diverged"
    );
}
