// Phase B -- valid-path differential tests.
//
// One test per row of CONFIGS.md. Every test drives BOTH shared objects through
// their exported C symbols and compares the captured stdout byte-for-byte.
// Randomized rows use the fixed seed from `common::SEED` so failures reproduce.

mod common;

use common::{
    assert_same, bad, diff_print_line, driver, good, print_line, print_line_raw, run_sequence,
    Impl, Rng, Step, SEED,
};

use std::ffi::c_char;

// ---------------------------------------------------------------------------
// Row 1 -- printLine, short random printable ASCII
// ---------------------------------------------------------------------------

#[test]
fn cfg_01_print_line_short_ascii() {
    let mut rng = Rng::new(SEED ^ 1);
    for i in 0..256 {
        let len = rng.range(1, 64);
        let s = rng.bytes_printable(len);
        diff_print_line(&format!("row1 iter {i} len {len}"), &s);
    }
}

// ---------------------------------------------------------------------------
// Row 2 -- printLine, arbitrary non-zero bytes (high/non-UTF-8, control bytes)
// ---------------------------------------------------------------------------

#[test]
fn cfg_02_print_line_random_bytes() {
    let mut rng = Rng::new(SEED ^ 2);
    for i in 0..256 {
        let len = rng.range(1, 4096);
        let s = rng.bytes_nonzero(len);
        diff_print_line(&format!("row2 iter {i} len {len}"), &s);
    }
}

// ---------------------------------------------------------------------------
// Row 3 -- printLine, long enough to cross libc's BUFSIZ stdout buffer
// ---------------------------------------------------------------------------

#[test]
fn cfg_03_print_line_crosses_bufsiz() {
    let mut rng = Rng::new(SEED ^ 3);
    for i in 0..64 {
        let len = rng.range(4000, 20000);
        let s = rng.bytes_nonzero(len);
        diff_print_line(&format!("row3 iter {i} len {len}"), &s);
    }

    // Pin the exact BUFSIZ-adjacent boundaries as well.
    for len in [4094usize, 4095, 4096, 4097, 8191, 8192, 8193] {
        let s = rng.bytes_printable(len);
        diff_print_line(&format!("row3 boundary len {len}"), &s);
    }
}

// ---------------------------------------------------------------------------
// Row 4 -- printLine, format-specifier lookalikes
//
// The C calls printf("%s\n", line): `line` is an *argument*, so a `%` inside it
// must be emitted literally and never interpreted. This row pins that.
// ---------------------------------------------------------------------------

#[test]
fn cfg_04_print_line_format_lookalikes() {
    const ALPHABET: &[u8] = b"%sdn\\\"'";

    let mut rng = Rng::new(SEED ^ 4);
    for i in 0..128 {
        let len = rng.range(1, 96);
        let s: Vec<u8> = (0..len).map(|_| *rng.pick(ALPHABET)).collect();
        diff_print_line(&format!("row4 iter {i} len {len}"), &s);
    }

    // Hand-picked specifiers that would crash or print garbage if either side
    // ever passed `line` as the format string itself.
    for s in [
        &b"%s"[..],
        b"%d",
        b"%n",
        b"%p",
        b"%99999999d",
        b"%s%s%s%s%s%s%s%s",
        b"%n%n%n%n",
        b"100%% done",
        b"\\n not a newline",
    ] {
        diff_print_line(
            &format!("row4 literal {:?}", String::from_utf8_lossy(s)),
            s,
        );
    }
}

// ---------------------------------------------------------------------------
// Rows 5-7 -- the argument-less entry points
// ---------------------------------------------------------------------------

#[test]
fn cfg_05_bad() {
    // Repeated, to catch any first-call-only state divergence.
    for i in 0..16 {
        assert_same(&format!("row5 bad() iter {i}"), &bad(Impl::C), &bad(Impl::Rust));
    }
}

#[test]
fn cfg_06_good() {
    for i in 0..16 {
        assert_same(
            &format!("row6 good() iter {i}"),
            &good(Impl::C),
            &good(Impl::Rust),
        );
    }
}

#[test]
fn cfg_07_driver() {
    for i in 0..16 {
        assert_same(
            &format!("row7 driver() iter {i}"),
            &driver(Impl::C),
            &driver(Impl::Rust),
        );
    }
}

// ---------------------------------------------------------------------------
// Row 8 -- printLine with an interior pointer into a larger buffer
// ---------------------------------------------------------------------------

#[test]
fn cfg_08_print_line_interior_pointer() {
    let mut rng = Rng::new(SEED ^ 8);
    for i in 0..128 {
        let offset = rng.range(1, 32);
        let tail = rng.range(0, 64);

        let mut buf = rng.bytes_nonzero(offset);
        let payload = rng.bytes_nonzero(tail);
        buf.extend_from_slice(&payload);
        buf.push(0);
        // Trailing garbage after the terminator, which must be ignored.
        buf.extend_from_slice(&rng.bytes_nonzero(8));

        let ptr = unsafe { buf.as_ptr().add(offset) } as *const c_char;
        let c_out = print_line_raw(Impl::C, ptr);
        let rust_out = print_line_raw(Impl::Rust, ptr);
        assert_same(
            &format!("row8 iter {i} offset {offset} tail {tail}"),
            &c_out,
            &rust_out,
        );
    }
}

// ---------------------------------------------------------------------------
// Row 9 -- composed pipeline: random interleavings of all four entry points
//
// This is the row that per-wrapper tests cannot cover. All calls in a sequence
// share one capture window and one buffered stdout, so divergence in output
// ORDERING or flush timing shows up here even when each individual call is
// correct in isolation.
// ---------------------------------------------------------------------------

#[test]
fn cfg_09_random_call_sequence() {
    let mut rng = Rng::new(SEED ^ 9);

    for i in 0..64 {
        let n = rng.range(1, 32);
        let steps: Vec<Step> = (0..n)
            .map(|_| match rng.range(0, 4) {
                0 => {
                    let len = rng.range(0, 80);
                    Step::PrintLine(rng.bytes_nonzero(len))
                }
                1 => Step::PrintLineNull,
                2 => Step::Bad,
                3 => Step::Good,
                _ => Step::Driver,
            })
            .collect();

        let c_out = run_sequence(Impl::C, &steps);
        let rust_out = run_sequence(Impl::Rust, &steps);
        assert_same(
            &format!("row9 iter {i} ({n} steps)"),
            &c_out,
            &rust_out,
        );
    }
}

// ---------------------------------------------------------------------------
// Row 10 -- exhaustive single-byte sweep
// ---------------------------------------------------------------------------

#[test]
fn cfg_10_print_line_every_single_byte() {
    for b in 1u8..=255 {
        let c_out = print_line(Impl::C, &[b]);
        let rust_out = print_line(Impl::Rust, &[b]);
        assert_same(&format!("row10 byte 0x{b:02x}"), &c_out, &rust_out);
    }
}

// ---------------------------------------------------------------------------
// Cross-check: the captured output is actually non-empty and shaped as the C
// source says. Guards against a harness bug that would make every comparison
// trivially pass by capturing nothing at all.
// ---------------------------------------------------------------------------

#[test]
fn cfg_harness_self_check_captures_real_bytes() {
    // printLine appends exactly one '\n' and nothing else.
    assert_eq!(print_line(Impl::C, b"xyz").ok_stdout(), b"xyz\n");
    assert_eq!(print_line(Impl::Rust, b"xyz").ok_stdout(), b"xyz\n");

    // From driver.c: bad() prints one line, good() prints two.
    assert_eq!(bad(Impl::C).ok_stdout(), b"bad()\n");
    assert_eq!(good(Impl::C).ok_stdout(), b"good()\nhelperGood()\n");

    // driver() prints the full 7-line sequence, in this exact order.
    let expected: &[u8] = b"Calling good()...\n\
                            good()\n\
                            helperGood()\n\
                            Finished good()\n\
                            Calling bad()...\n\
                            bad()\n\
                            Finished bad()\n";
    // 7 lines: driver()'s own 4 printLine literals, plus good()'s 2 and bad()'s 1.
    assert_eq!(
        driver(Impl::C).ok_stdout(),
        expected,
        "C driver() output does not match the C source reading"
    );
    assert_eq!(driver(Impl::Rust).ok_stdout(), expected);
}
