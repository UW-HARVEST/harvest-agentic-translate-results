// Phase B -- valid-path differential tests.
//
// One test per row of CONFIGS.md. Every row is driven with MANY randomised
// inputs from a fixed seed (`common::SEED`), and both the C `.so` and the Rust
// `.so` are exercised through their exported symbols only.
//
// The low-level entry point `printLine` (exported but absent from driver.h) is
// driven directly, not only through the `driver` wrapper.

mod common;

use common::*;

// ===========================================================================
// Row 1 -- driver, randomised sweep of the accepted range [0, 99]
// ===========================================================================

#[test]
fn cfg_01_driver_accepted_range_random() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 0x01);

    for i in 0..1000 {
        let data = rng.range_i32(0, 99);
        let calls = [Call::Driver(data)];
        assert_same_and_output(
            &pair,
            &calls,
            &oracle_driver(data),
            &format!("cfg_01 iter={i} data={data}"),
        );
    }
}

// ===========================================================================
// Row 2 -- driver, exhaustive over the accepted range
// ===========================================================================

#[test]
fn cfg_02_driver_accepted_range_exhaustive() {
    let pair = load_pair();

    for data in 0..=99i32 {
        assert_same_and_output(
            &pair,
            &[Call::Driver(data)],
            &oracle_driver(data),
            &format!("cfg_02 data={data}"),
        );
    }
}

// ===========================================================================
// Row 3 -- driver, shape "empty": data == 0
// ===========================================================================

#[test]
fn cfg_03_driver_data_zero() {
    let pair = load_pair();
    // strncpy(dest, source, 0) copies nothing; dest[0] = '\0'; -> lone newline.
    assert_same_and_output(&pair, &[Call::Driver(0)], b"\n", "cfg_03 data=0");

    // Repeated, to confirm the zero case is stable across invocations.
    let calls: Vec<Call> = (0..32).map(|_| Call::Driver(0)).collect();
    let expected: Vec<u8> = std::iter::repeat(b'\n').take(32).collect();
    assert_same_and_output(&pair, &calls, &expected, "cfg_03 data=0 x32");
}

// ===========================================================================
// Row 4 -- driver, shape "one": data == 1
// ===========================================================================

#[test]
fn cfg_04_driver_data_one() {
    let pair = load_pair();
    assert_same_and_output(&pair, &[Call::Driver(1)], b"A\n", "cfg_04 data=1");
}

// ===========================================================================
// Row 5 -- driver, shape "many": randomised data in [2, 98]
// ===========================================================================

#[test]
fn cfg_05_driver_data_many_random() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 0x05);

    for i in 0..800 {
        let data = rng.range_i32(2, 98);
        assert_same_and_output(
            &pair,
            &[Call::Driver(data)],
            &oracle_driver(data),
            &format!("cfg_05 iter={i} data={data}"),
        );
    }
}

// ===========================================================================
// Row 6 -- driver, shape "exactly-full": data == 99
//
// The copy consumes all 99 'A' bytes of `source` (so strncpy contributes NO
// terminator) and `dest[99]` -- the last in-bounds element -- is the NUL.
// ===========================================================================

#[test]
fn cfg_06_driver_data_99_full() {
    let pair = load_pair();

    let mut expected = vec![b'A'; 99];
    expected.push(b'\n');
    assert_same_and_output(&pair, &[Call::Driver(99)], &expected, "cfg_06 data=99");

    // 98 vs 99 vs 100 in one child, to pin the boundary transition and prove
    // the full-buffer case leaves no residue for the next call.
    let mut expected3 = vec![b'A'; 98];
    expected3.push(b'\n');
    expected3.extend(std::iter::repeat(b'A').take(99));
    expected3.push(b'\n');
    expected3.push(b'\n'); // data == 100 -> rejected -> empty line
    assert_same_and_output(
        &pair,
        &[Call::Driver(98), Call::Driver(99), Call::Driver(100)],
        &expected3,
        "cfg_06 boundary 98/99/100",
    );
}

// ===========================================================================
// Row 7 -- driver, repeated invocation on one handle (statelessness)
// ===========================================================================

#[test]
fn cfg_07_driver_repeated_calls_random() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 0x07);

    for batch in 0..150 {
        let n = 1 + rng.below(24) as usize;
        let datas: Vec<i32> = (0..n).map(|_| rng.range_i32(0, 99)).collect();
        let calls: Vec<Call> = datas.iter().map(|d| Call::Driver(*d)).collect();
        let mut expected = Vec::new();
        for d in &datas {
            expected.extend_from_slice(&oracle_driver(*d));
        }
        assert_same_and_output(
            &pair,
            &calls,
            &expected,
            &format!("cfg_07 batch={batch} n={n}"),
        );
    }
}

// ===========================================================================
// Row 8 -- printLine (direct), shape "empty"
// ===========================================================================

#[test]
fn cfg_08_printline_empty() {
    let pair = load_pair();
    assert_same_and_output(&pair, &[Call::print_line(b"")], b"\n", "cfg_08 empty");

    let calls: Vec<Call> = (0..16).map(|_| Call::print_line(b"")).collect();
    let expected: Vec<u8> = std::iter::repeat(b'\n').take(16).collect();
    assert_same_and_output(&pair, &calls, &expected, "cfg_08 empty x16");
}

// ===========================================================================
// Row 9 -- printLine (direct), shape "one": every non-NUL byte value
// ===========================================================================

#[test]
fn cfg_09_printline_one_byte_all_values() {
    let pair = load_pair();

    // Exhaustive over all 255 legal single-byte strings, batched to keep the
    // fork count sane while still comparing byte-for-byte.
    let mut calls = Vec::new();
    let mut expected = Vec::new();
    for b in 1u8..=255 {
        calls.push(Call::print_line(&[b]));
        expected.extend_from_slice(&oracle_print_line(&[b]));
    }
    assert_same_and_output(&pair, &calls, &expected, "cfg_09 all single bytes");

    // Randomised order as well, one child per value, so a value-dependent
    // divergence cannot hide behind batching.
    let mut rng = Rng::new(SEED ^ 0x09);
    for i in 0..500 {
        let b = rng.byte_in(1, 255);
        assert_same_and_output(
            &pair,
            &[Call::print_line(&[b])],
            &oracle_print_line(&[b]),
            &format!("cfg_09 iter={i} byte=0x{b:02x}"),
        );
    }
}

// ===========================================================================
// Row 10 -- printLine (direct), shape "many": random length, random bytes
// ===========================================================================

#[test]
fn cfg_10_printline_many_random_bytes() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 0x10);

    for i in 0..400 {
        let len = 2 + rng.below(4095) as usize; // [2, 4096]
        let s: Vec<u8> = (0..len).map(|_| rng.byte_in(1, 255)).collect();
        assert_same_and_output(
            &pair,
            &[Call::PrintLine(s.clone())],
            &oracle_print_line(&s),
            &format!("cfg_10 iter={i} len={len}"),
        );
    }
}

// ===========================================================================
// Row 11 -- printLine (direct) with the exact buffer shape `driver` builds
// ===========================================================================

#[test]
fn cfg_11_printline_driver_shaped_buffer() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 0x11);

    for i in 0..400 {
        let n = rng.range_i32(0, 99) as usize;
        let s = vec![b'A'; n];
        assert_same_and_output(
            &pair,
            &[Call::PrintLine(s.clone())],
            &oracle_print_line(&s),
            &format!("cfg_11 iter={i} n={n}"),
        );
    }

    // And the composed equivalence: driver(n) must emit exactly what a direct
    // printLine("A"*n) emits, for every n in the accepted window.
    for n in 0..=99i32 {
        let s = vec![b'A'; n as usize];
        let via_wrapper = run_one(&pair.c, Call::Driver(n));
        let via_primitive = run_one(&pair.c, Call::PrintLine(s.clone()));
        assert_eq!(
            via_wrapper.stdout,
            via_primitive.stdout,
            "C: driver({n}) != printLine(\"A\"*{n})"
        );
        let r_wrapper = run_one(&pair.rust, Call::Driver(n));
        let r_primitive = run_one(&pair.rust, Call::PrintLine(s));
        assert_eq!(
            r_wrapper.stdout, r_primitive.stdout,
            "Rust: driver({n}) != printLine(\"A\"*{n})"
        );
        assert_eq!(
            via_wrapper.stdout, r_wrapper.stdout,
            "cfg_11 composed divergence at n={n}"
        );
    }
}

// ===========================================================================
// Row 12 -- printLine (direct), content = printf format specifiers
//
// Inert in the C (driver.c:34 uses a fixed format string), but a translation
// that passed `line` AS the format would diverge loudly here.
// ===========================================================================

#[test]
fn cfg_12_printline_percent_content() {
    let pair = load_pair();

    let fixed: &[&[u8]] = &[
        b"%s",
        b"%n",
        b"%d",
        b"%%",
        b"%",
        b"%s%s%s%s%s%s%s%s",
        b"%n%n%n%n",
        b"100%% sure",
        b"%p %x %o %c %e %f %g",
        b"%1$s %2$s",
        b"%.*s",
        b"%999999999d",
        b"%hhn%hn%lln",
        b"A%sB%nC",
    ];
    let mut calls = Vec::new();
    let mut expected = Vec::new();
    for s in fixed {
        calls.push(Call::print_line(s));
        expected.extend_from_slice(&oracle_print_line(s));
    }
    assert_same_and_output(&pair, &calls, &expected, "cfg_12 fixed specifiers");

    // Randomised strings densely seeded with '%' and specifier letters.
    let mut rng = Rng::new(SEED ^ 0x12);
    const LETTERS: &[u8] = b"sdnxXopcefgiu%*.$hlLqjzt0123456789 ";
    for i in 0..500 {
        let len = 1 + rng.below(120) as usize;
        let s: Vec<u8> = (0..len)
            .map(|_| {
                if rng.below(3) == 0 {
                    b'%'
                } else {
                    LETTERS[rng.below(LETTERS.len() as u64) as usize]
                }
            })
            .collect();
        assert_same_and_output(
            &pair,
            &[Call::PrintLine(s.clone())],
            &oracle_print_line(&s),
            &format!("cfg_12 iter={i} len={len}"),
        );
    }
}

// ===========================================================================
// Row 13 -- printLine (direct), embedded control bytes
// ===========================================================================

#[test]
fn cfg_13_printline_control_bytes() {
    let pair = load_pair();

    let fixed: &[&[u8]] = &[
        b"\n",
        b"\n\n\n",
        b"a\nb",
        b"\t",
        b"a\tb\tc",
        b"\r",
        b"a\r\nb",
        b"\x0b\x0c",
        b"\x7f",
        b"\x01\x02\x03\x04\x05\x06\x07\x08",
        b"line1\nline2\nline3",
        b"\x1b[31mred\x1b[0m",
    ];
    let mut calls = Vec::new();
    let mut expected = Vec::new();
    for s in fixed {
        calls.push(Call::print_line(s));
        expected.extend_from_slice(&oracle_print_line(s));
    }
    assert_same_and_output(&pair, &calls, &expected, "cfg_13 fixed control bytes");

    // Randomised: control bytes sprinkled at random positions in ASCII text.
    let mut rng = Rng::new(SEED ^ 0x13);
    const CTRL: &[u8] = &[b'\n', b'\t', b'\r', 0x0b, 0x0c, 0x7f, 0x01, 0x1b];
    for i in 0..500 {
        let len = 1 + rng.below(200) as usize;
        let s: Vec<u8> = (0..len)
            .map(|_| {
                if rng.below(4) == 0 {
                    CTRL[rng.below(CTRL.len() as u64) as usize]
                } else {
                    rng.byte_in(b'a', b'z')
                }
            })
            .collect();
        assert_same_and_output(
            &pair,
            &[Call::PrintLine(s.clone())],
            &oracle_print_line(&s),
            &format!("cfg_13 iter={i} len={len}"),
        );
    }
}

// ===========================================================================
// Row 14 -- printLine (direct), payload larger than the stdio buffer
// ===========================================================================

#[test]
fn cfg_14_printline_over_bufsiz() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 0x14);

    // Exact multiples of / offsets around common BUFSIZ values, where a
    // partial-flush bug would show up.
    for len in [4095usize, 4096, 4097, 8191, 8192, 8193, 16384, 65535, 65536] {
        let s = vec![b'Z'; len];
        assert_same_and_output(
            &pair,
            &[Call::PrintLine(s.clone())],
            &oracle_print_line(&s),
            &format!("cfg_14 len={len}"),
        );
    }

    for i in 0..60 {
        let len = 8193 + rng.below(40960 - 8193 + 1) as usize;
        let s: Vec<u8> = (0..len).map(|_| rng.byte_in(1, 255)).collect();
        assert_same_and_output(
            &pair,
            &[Call::PrintLine(s.clone())],
            &oracle_print_line(&s),
            &format!("cfg_14 iter={i} len={len}"),
        );
    }
}

// ===========================================================================
// Row 15 -- printLine (direct), high-bit / invalid-UTF-8 payloads
//
// A translation that round-tripped the pointer through `str`/`String` would
// panic or lossily replace bytes here; raw `*const c_char` must not.
// ===========================================================================

#[test]
fn cfg_15_printline_high_bit_bytes() {
    let pair = load_pair();

    // Exhaustive: every high byte on its own, plus notorious sequences.
    let mut calls = Vec::new();
    let mut expected = Vec::new();
    for b in 0x80u8..=0xFF {
        calls.push(Call::print_line(&[b]));
        expected.extend_from_slice(&oracle_print_line(&[b]));
    }
    let fixed: &[&[u8]] = &[
        &[0xff],
        &[0xff, 0xfe],
        &[0xc0, 0x80],             // overlong NUL encoding
        &[0xed, 0xa0, 0x80],       // UTF-16 surrogate half
        &[0xf4, 0x90, 0x80, 0x80], // > U+10FFFF
        &[0x80, 0x81, 0x82],       // lone continuation bytes
        &[0xfe, 0xff, 0xfe, 0xff],
    ];
    for s in fixed {
        calls.push(Call::print_line(s));
        expected.extend_from_slice(&oracle_print_line(s));
    }
    assert_same_and_output(&pair, &calls, &expected, "cfg_15 all high bytes");

    let mut rng = Rng::new(SEED ^ 0x15);
    for i in 0..500 {
        let len = 1 + rng.below(300) as usize;
        let s: Vec<u8> = (0..len).map(|_| rng.byte_in(0x80, 0xFF)).collect();
        assert_same_and_output(
            &pair,
            &[Call::PrintLine(s.clone())],
            &oracle_print_line(&s),
            &format!("cfg_15 iter={i} len={len}"),
        );
    }
}

// ===========================================================================
// Row 16 -- composed pipeline: interleaved driver / printLine / printLine(NULL)
//
// Exercises output ordering and stdio buffering across the whole pipeline on a
// single loaded handle -- invisible to per-function tests.
// ===========================================================================

#[test]
fn cfg_16_interleaved_pipeline_random() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 0x16);

    for batch in 0..200 {
        let n = 1 + rng.below(30) as usize;
        let mut calls = Vec::new();
        let mut expected = Vec::new();

        for _ in 0..n {
            match rng.below(4) {
                0 => {
                    // driver in the accepted window
                    let d = rng.range_i32(0, 99);
                    calls.push(Call::Driver(d));
                    expected.extend_from_slice(&oracle_driver(d));
                }
                1 => {
                    // driver in the rejected window -> lone newline
                    let d = rng.range_i32(100, i32::MAX);
                    calls.push(Call::Driver(d));
                    expected.push(b'\n');
                }
                2 => {
                    let len = rng.below(200) as usize;
                    let s: Vec<u8> = (0..len).map(|_| rng.byte_in(1, 255)).collect();
                    calls.push(Call::PrintLine(s.clone()));
                    expected.extend_from_slice(&oracle_print_line(&s));
                }
                _ => {
                    // NULL -> emits nothing at all
                    calls.push(Call::PrintLineNull);
                }
            }
        }

        assert_same_and_output(
            &pair,
            &calls,
            &expected,
            &format!("cfg_16 batch={batch} n={n}"),
        );
    }
}
