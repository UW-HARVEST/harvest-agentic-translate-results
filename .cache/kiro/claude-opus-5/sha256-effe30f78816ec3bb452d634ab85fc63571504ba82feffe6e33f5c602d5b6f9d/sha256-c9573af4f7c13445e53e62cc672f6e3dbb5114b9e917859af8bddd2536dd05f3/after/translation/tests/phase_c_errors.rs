// Phase C -- error-path differential tests.
//
// One test per row of ERRORS.md. Because neither public function returns a
// value, a "rejection" is observable only as (a) the absence of the guarded
// side effect, or (b) termination by signal for the unchecked negative `data`.
// Each test asserts the C `.so` and the Rust `.so` agree on the *specific*
// observable -- the exact byte stream, and the exact signal number -- not
// merely that "both failed somehow".

mod common;

use common::*;

/// `SIGSEGV`. The out-of-bounds `strncpy` in `driver` for negative `data`
/// terminates the process with this signal in both builds.
const SIGSEGV: i32 = 11;

// ===========================================================================
// Row 1 -- printLine(NULL): `if(line != NULL)` is false -> nothing printed
// ===========================================================================

#[test]
fn err_01_printline_null() {
    let pair = load_pair();

    // Exactly zero bytes, clean exit.
    assert_same_and_output(&pair, &[Call::PrintLineNull], b"", "err_01 single NULL");

    // Many NULLs still produce nothing.
    let calls: Vec<Call> = (0..64).map(|_| Call::PrintLineNull).collect();
    assert_same_and_output(&pair, &calls, b"", "err_01 NULL x64");

    // A NULL between two real calls must not perturb the surrounding output --
    // proves the rejection is a no-op rather than, say, an empty line.
    assert_same_and_output(
        &pair,
        &[
            Call::print_line(b"before"),
            Call::PrintLineNull,
            Call::print_line(b"after"),
        ],
        b"before\nafter\n",
        "err_01 NULL between real calls",
    );

    // Interleaved with driver, randomised.
    let mut rng = Rng::new(SEED ^ 0xC1);
    for i in 0..200 {
        let d = rng.range_i32(0, 99);
        let mut expected = Vec::new();
        expected.extend_from_slice(&oracle_driver(d));
        assert_same_and_output(
            &pair,
            &[
                Call::PrintLineNull,
                Call::Driver(d),
                Call::PrintLineNull,
                Call::PrintLineNull,
            ],
            &expected,
            &format!("err_01 iter={i} data={d}"),
        );
    }
}

// ===========================================================================
// Row 2 -- driver(100): the exact first value rejected by `data < 100`
// ===========================================================================

#[test]
fn err_02_driver_data_eq_100() {
    let pair = load_pair();
    // Copy block skipped; `dest` is still the zero-initialised "" -> lone \n.
    assert_same_and_output(&pair, &[Call::Driver(100)], b"\n", "err_02 data=100");

    // Contrast with the last accepted value, in one child, to pin the
    // boundary: 99 -> 99 'A's, 100 -> empty.
    let mut expected = vec![b'A'; 99];
    expected.push(b'\n');
    expected.push(b'\n');
    assert_same_and_output(
        &pair,
        &[Call::Driver(99), Call::Driver(100)],
        &expected,
        "err_02 boundary 99 then 100",
    );
}

// ===========================================================================
// Row 3 -- driver(101): one step past the boundary
// ===========================================================================

#[test]
fn err_03_driver_data_101() {
    let pair = load_pair();
    assert_same_and_output(&pair, &[Call::Driver(101)], b"\n", "err_03 data=101");
}

// ===========================================================================
// Row 4 -- driver(INT_MAX): maximum rejected value / oversized length
// ===========================================================================

#[test]
fn err_04_driver_data_int_max() {
    let pair = load_pair();
    assert_same_and_output(
        &pair,
        &[Call::Driver(i32::MAX)],
        b"\n",
        "err_04 data=INT_MAX",
    );

    // Nearby oversized values and power-of-two boundaries.
    for d in [
        i32::MAX,
        i32::MAX - 1,
        0x4000_0000,
        0x7FFF_FFFE,
        1 << 20,
        1 << 30,
        100_000,
        65536,
        256,
        128,
    ] {
        assert_same_and_output(&pair, &[Call::Driver(d)], b"\n", &format!("err_04 data={d}"));
    }
}

// ===========================================================================
// Row 5 -- driver over the whole rejected range [100, INT_MAX], randomised
// ===========================================================================

#[test]
fn err_05_driver_rejected_range_random() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 0xC5);

    // Batched: every rejected value must emit exactly one empty line.
    for batch in 0..100 {
        let n = 1 + rng.below(20) as usize;
        let calls: Vec<Call> = (0..n)
            .map(|_| Call::Driver(rng.range_i32(100, i32::MAX)))
            .collect();
        let expected: Vec<u8> = std::iter::repeat(b'\n').take(n).collect();
        assert_same_and_output(
            &pair,
            &calls,
            &expected,
            &format!("err_05 batch={batch} n={n}"),
        );
    }

    // Unbatched, so a value-specific divergence cannot be masked.
    for i in 0..600 {
        let d = rng.range_i32(100, i32::MAX);
        assert_same_and_output(&pair, &[Call::Driver(d)], b"\n", &format!("err_05 iter={i} data={d}"));
    }

    // Dense sweep just above the boundary, where an off-by-one would live.
    for d in 100..=160i32 {
        assert_same_and_output(&pair, &[Call::Driver(d)], b"\n", &format!("err_05 dense data={d}"));
    }
}

// ===========================================================================
// Row 6 -- driver(-1): UNCHECKED negative passes the signed guard
//
// `data` sign-extends to size_t 0xFFFF_FFFF_FFFF_FFFF as strncpy's length, so
// the NUL padding runs off the end of `dest`. Both builds must die from the
// same signal.
// ===========================================================================

#[test]
fn err_06_driver_data_neg1() {
    let pair = load_pair();
    assert_same_and_signal(&pair, &[Call::Driver(-1)], SIGSEGV, "err_06 data=-1");
}

// ===========================================================================
// Row 7 -- driver(-2): confirms it is the sign, not the single value -1
// ===========================================================================

#[test]
fn err_07_driver_data_neg2() {
    let pair = load_pair();
    assert_same_and_signal(&pair, &[Call::Driver(-2)], SIGSEGV, "err_07 data=-2");

    for d in [-3i32, -4, -10, -99, -100, -255, -256, -1000] {
        assert_same_and_signal(&pair, &[Call::Driver(d)], SIGSEGV, &format!("err_07 data={d}"));
    }
}

// ===========================================================================
// Row 8 -- driver(INT_MIN): one step past the low end; worst-case sign
// extension to 0xFFFF_FFFF_8000_0000
// ===========================================================================

#[test]
fn err_08_driver_data_int_min() {
    let pair = load_pair();
    assert_same_and_signal(
        &pair,
        &[Call::Driver(i32::MIN)],
        SIGSEGV,
        "err_08 data=INT_MIN",
    );
    assert_same_and_signal(
        &pair,
        &[Call::Driver(i32::MIN + 1)],
        SIGSEGV,
        "err_08 data=INT_MIN+1",
    );
    for d in [-0x4000_0000i32, -(1 << 20), -(1 << 30), -65536] {
        assert_same_and_signal(&pair, &[Call::Driver(d)], SIGSEGV, &format!("err_08 data={d}"));
    }
}

// ===========================================================================
// Row 9 -- driver over the whole negative range [INT_MIN, -1], randomised
// ===========================================================================

#[test]
fn err_09_driver_negative_range_random() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 0xC9);

    for i in 0..800 {
        let d = rng.range_i32(i32::MIN, -1);
        assert_same_and_signal(
            &pair,
            &[Call::Driver(d)],
            SIGSEGV,
            &format!("err_09 iter={i} data={d}"),
        );
    }

    // The crash must occur on the FIRST negative call, so any output produced
    // by preceding valid calls is flushed identically (or identically lost) by
    // both builds. Compare the full outcome, whatever it is.
    for i in 0..250 {
        let good = rng.range_i32(0, 99);
        let bad = rng.range_i32(i32::MIN, -1);
        let calls = [Call::Driver(good), Call::Driver(bad), Call::Driver(good)];
        let out = assert_same(&pair, &calls, &format!("err_09 mixed iter={i}"));
        assert_eq!(
            out.signal,
            Some(SIGSEGV),
            "err_09 mixed iter={i}: expected SIGSEGV, got {out:?}"
        );
    }
}

// ===========================================================================
// Generic FFI boundaries required beyond the table
// ===========================================================================

/// Full-`int` sweep of `driver`'s only parameter. `int` is the analogue of the
/// "out-of-range enum value" case here: `driver.h` declares no enum, so the
/// widest smuggle-anything-across-the-FFI surface is the raw `int`, including
/// values that carry no meaningful interpretation. Every value in `i32` is
/// either accepted (`[0,99]`), silently rejected (`[100,INT_MAX]`), or fatal
/// (`[INT_MIN,-1]`), and C and Rust must agree on which.
#[test]
fn err_10_full_int_domain_random() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 0xCA);

    for i in 0..2000 {
        let d = rng.range_i32(i32::MIN, i32::MAX);
        let out = assert_same(&pair, &[Call::Driver(d)], &format!("err_10 iter={i} data={d}"));

        // Classify and pin the absolute behaviour too.
        if d < 0 {
            assert_eq!(out.signal, Some(SIGSEGV), "err_10 data={d}: expected SIGSEGV, got {out:?}");
        } else if d < 100 {
            assert!(out.exited_ok(), "err_10 data={d}: expected clean exit, got {out:?}");
            assert_eq!(out.stdout, oracle_driver(d), "err_10 data={d}");
        } else {
            assert!(out.exited_ok(), "err_10 data={d}: expected clean exit, got {out:?}");
            assert_eq!(out.stdout, b"\n", "err_10 data={d}");
        }
    }
}

/// Every distinguished boundary of the `int` domain, exhaustively enumerated:
/// the two ends, the two guard edges, and one step either side of each.
#[test]
fn err_11_all_int_boundaries() {
    let pair = load_pair();

    let boundaries: &[i32] = &[
        i32::MIN,
        i32::MIN + 1,
        -2,
        -1, // one step below the valid window
        0,  // zero length
        1,
        98,
        99,  // last accepted
        100, // one step past the documented range
        101,
        i32::MAX - 1,
        i32::MAX,
    ];

    for &d in boundaries {
        let out = assert_same(&pair, &[Call::Driver(d)], &format!("err_11 data={d}"));
        if d < 0 {
            assert_eq!(out.signal, Some(SIGSEGV), "err_11 data={d}: {out:?}");
        } else {
            assert!(out.exited_ok(), "err_11 data={d}: {out:?}");
            let expected: Vec<u8> = if d < 100 {
                oracle_driver(d)
            } else {
                b"\n".to_vec()
            };
            assert_eq!(out.stdout, expected, "err_11 data={d}");
        }
    }
}

/// `printLine` boundary shapes: NULL (row 1), the empty string (zero length),
/// and a payload far larger than any internal buffer (oversized length). The
/// NULL case is the only pointer rejection in the library; the other two are
/// accepted and must round-trip byte-for-byte.
#[test]
fn err_12_printline_pointer_and_length_boundaries() {
    let pair = load_pair();

    // NULL -> nothing; "" -> one newline. Both in one child so the difference
    // between "no output" and "empty line" is unambiguous.
    assert_same_and_output(
        &pair,
        &[Call::PrintLineNull, Call::print_line(b""), Call::PrintLineNull],
        b"\n",
        "err_12 NULL vs empty",
    );

    // Oversized payload: 1 MiB, well beyond any stdio buffer.
    let big = vec![b'q'; 1 << 20];
    assert_same_and_output(
        &pair,
        &[Call::PrintLine(big.clone())],
        &oracle_print_line(&big),
        "err_12 1MiB payload",
    );

    // A payload whose only byte is 0xFF, immediately followed by the
    // terminator -- the shortest non-ASCII string.
    assert_same_and_output(
        &pair,
        &[Call::print_line(&[0xff])],
        b"\xff\n",
        "err_12 single 0xFF",
    );
}
