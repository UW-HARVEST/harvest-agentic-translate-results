// Phase B — valid-path differential tests.
//
// One test per row of CONFIGS.md. Every test drives BOTH the C `.so` and the
// Rust `.so` through `dlsym` and compares captured stdout byte-for-byte.
//
// Ordering follows the call hierarchy: the leaf entry points (`printIntLine`,
// `printLine`) first, then the mid-level `bad`/`good`, then the `driver`
// wrapper, and finally a randomized interleaving of all five.

mod common;

use common::{assert_same, cstr, libs, capture, Rng};
use std::ffi::c_int;

// ---------------------------------------------------------------------------
// printIntLine — rows 1-7
// ---------------------------------------------------------------------------

/// Row 1: the value `bad()`/`good()` actually print.
#[test]
fn cfg_01_print_int_line_zero() {
    assert_same("printIntLine(0)", |l| unsafe { (l.print_int_line)(0) });
}

/// Row 2: single-digit positives.
#[test]
fn cfg_02_print_int_line_single_digit() {
    for v in 1..=9 {
        assert_same(&format!("printIntLine({v})"), |l| unsafe {
            (l.print_int_line)(v)
        });
    }
}

/// Row 3: sign + single digit.
#[test]
fn cfg_03_print_int_line_small_negative() {
    for v in -9..=-1 {
        assert_same(&format!("printIntLine({v})"), |l| unsafe {
            (l.print_int_line)(v)
        });
    }
}

/// Row 4: digit-count carry boundaries, both signs.
#[test]
fn cfg_04_print_int_line_digit_boundaries() {
    let mut vals = Vec::new();
    for base in [9, 10, 99, 100, 999, 1000, 9999, 10_000, 99_999, 100_000] {
        vals.push(base);
        vals.push(-base);
    }
    for base in [999_999_999_i32, 1_000_000_000, 2_000_000_000] {
        vals.push(base);
        vals.push(-base);
    }
    for v in vals {
        assert_same(&format!("printIntLine({v})"), |l| unsafe {
            (l.print_int_line)(v)
        });
    }
}

/// Row 5: 32-bit extremes.
#[test]
fn cfg_05_print_int_line_extremes() {
    for v in [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX] {
        assert_same(&format!("printIntLine({v})"), |l| unsafe {
            (l.print_int_line)(v)
        });
    }
}

/// Row 6: randomized over the full 32-bit range.
#[test]
fn cfg_06_print_int_line_random() {
    let mut rng = Rng::new();
    for _ in 0..4096 {
        let v = rng.next_i32();
        assert_same(&format!("printIntLine({v})"), |l| unsafe {
            (l.print_int_line)(v)
        });
    }
}

/// Row 7: 1024 randomized values in ONE capture window, so the whole
/// accumulated stdio byte stream is compared rather than each call in isolation.
#[test]
fn cfg_07_print_int_line_batch_run() {
    let mut rng = Rng::new();
    let vals: Vec<i32> = (0..1024).map(|_| rng.next_i32()).collect();
    assert_same("printIntLine batch run (1024 values, one capture)", |l| {
        for &v in &vals {
            unsafe { (l.print_int_line)(v) }
        }
    });
}

// ---------------------------------------------------------------------------
// printLine — rows 8-15
// ---------------------------------------------------------------------------

/// Row 8: non-NULL but empty — passes the guard, emits only "\n".
#[test]
fn cfg_08_print_line_empty() {
    let s = cstr(b"");
    assert_same("printLine(\"\")", |l| unsafe {
        (l.print_line)(s.as_ptr().cast())
    });
}

/// Row 9: every single non-NUL byte value, including high/non-UTF-8 bytes.
#[test]
fn cfg_09_print_line_all_single_bytes() {
    for b in 1u8..=255 {
        let s = cstr(&[b]);
        assert_same(&format!("printLine(&[{b:#04x}])"), |l| unsafe {
            (l.print_line)(s.as_ptr().cast())
        });
    }
}

/// Row 10: plain ASCII of varying length.
#[test]
fn cfg_10_print_line_ascii() {
    let mut rng = Rng::new();
    for len in 2..=64 {
        let bytes: Vec<u8> = (0..len)
            .map(|_| b'!' + (rng.below(0x7e - 0x21) as u8))
            .collect();
        let s = cstr(&bytes);
        assert_same(&format!("printLine(ascii len {len})"), |l| unsafe {
            (l.print_line)(s.as_ptr().cast())
        });
    }
}

/// Row 11: high bytes must pass through untouched (no UTF-8 validation or
/// lossy conversion may creep into the Rust translation).
#[test]
fn cfg_11_print_line_non_utf8() {
    let cases: Vec<Vec<u8>> = vec![
        vec![0x80],
        vec![0xff, 0xfe, 0xfd],
        vec![0xc3],                         // truncated 2-byte UTF-8 lead
        vec![0xe2, 0x82],                   // truncated 3-byte UTF-8 lead
        vec![0xf0, 0x9f, 0x92],             // truncated 4-byte UTF-8 lead
        vec![b'a', 0x80, b'b', 0xff, b'c'], // interleaved with ASCII
        (0x80u8..=0xff).collect(),          // every high byte at once
    ];
    for (i, bytes) in cases.iter().enumerate() {
        let s = cstr(bytes);
        assert_same(&format!("printLine(non-utf8 case {i})"), |l| unsafe {
            (l.print_line)(s.as_ptr().cast())
        });
    }
}

/// Row 12: C appends exactly one '\n' no matter what the payload contains.
#[test]
fn cfg_12_print_line_embedded_newlines() {
    let cases: Vec<&[u8]> = vec![
        b"\n",
        b"\n\n\n",
        b"a\nb",
        b"trailing\n",
        b"\rcarriage",
        b"tab\there",
        b"mixed\r\n\tline\n",
    ];
    for (i, bytes) in cases.iter().enumerate() {
        let s = cstr(bytes);
        assert_same(&format!("printLine(newlines case {i})"), |l| unsafe {
            (l.print_line)(s.as_ptr().cast())
        });
    }
}

/// Row 13: format specifiers are DATA, not format. The C does
/// `printf("%s\n", line)`, so a translation that did `printf(line)` would
/// diverge (or crash) here.
#[test]
fn cfg_13_print_line_format_specifiers() {
    let cases: Vec<&[u8]> = vec![
        b"%s",
        b"%d",
        b"%%",
        b"%n",
        b"%p",
        b"%1000000d",
        b"%s %s %s %s %s",
        b"%n%n%n%n",
        b"100%% done: %d of %s",
    ];
    for (i, bytes) in cases.iter().enumerate() {
        let s = cstr(bytes);
        assert_same(&format!("printLine(format case {i})"), |l| unsafe {
            (l.print_line)(s.as_ptr().cast())
        });
    }
}

/// Row 14: randomized byte strings, random lengths.
#[test]
fn cfg_14_print_line_random_bytes() {
    let mut rng = Rng::new();
    for _ in 0..2048 {
        let len = rng.below(257) as usize;
        let bytes: Vec<u8> = (0..len).map(|_| rng.nonzero_byte()).collect();
        let s = cstr(&bytes);
        assert_same(&format!("printLine(random len {len})"), |l| unsafe {
            (l.print_line)(s.as_ptr().cast())
        });
    }
}

/// Row 15: lengths straddling stdio buffer and page boundaries.
#[test]
fn cfg_15_print_line_long_buffer_boundaries() {
    let mut rng = Rng::new();
    for len in [1023usize, 1024, 1025, 4095, 4096, 4097, 8191, 8192, 8193, 65536, 1 << 20] {
        let bytes: Vec<u8> = (0..len).map(|_| rng.nonzero_byte()).collect();
        let s = cstr(&bytes);
        assert_same(&format!("printLine(len {len})"), |l| unsafe {
            (l.print_line)(s.as_ptr().cast())
        });
    }
}

// ---------------------------------------------------------------------------
// bad / good — rows 16-19
// ---------------------------------------------------------------------------

/// Row 16: the undersized-`alloca` path (CWE-131), called directly.
#[test]
fn cfg_16_bad_single() {
    assert_same("bad()", |l| unsafe { (l.bad)() });
}

/// Row 17: the correctly-sized-`alloca` path, called directly.
#[test]
fn cfg_17_good_single() {
    assert_same("good()", |l| unsafe { (l.good)() });
}

/// Row 18: many calls in one capture — each C call gets a fresh `alloca` frame,
/// so a Rust translation backed by a `static` buffer would show up here.
#[test]
fn cfg_18_bad_repeated() {
    assert_same("bad() x256 (one capture)", |l| {
        for _ in 0..256 {
            unsafe { (l.bad)() }
        }
    });
}

/// Row 19: same for `good()`.
#[test]
fn cfg_19_good_repeated() {
    assert_same("good() x256 (one capture)", |l| {
        for _ in 0..256 {
            unsafe { (l.good)() }
        }
    });
}

// ---------------------------------------------------------------------------
// driver — rows 20-23
// ---------------------------------------------------------------------------

/// Row 20: `useGood = 0` selects `bad()`.
#[test]
fn cfg_20_driver_false() {
    assert_same("driver(0)", |l| unsafe { (l.driver)(0) });
}

/// Row 21: `useGood = 1` selects `good()`.
#[test]
fn cfg_21_driver_true() {
    assert_same("driver(1)", |l| unsafe { (l.driver)(1) });
}

/// Row 22: randomized flags across the full `int` range.
#[test]
fn cfg_22_driver_random_flag() {
    let mut rng = Rng::new();
    for _ in 0..1024 {
        let v: c_int = rng.next_i32();
        assert_same(&format!("driver({v})"), |l| unsafe { (l.driver)(v) });
    }
}

/// Row 23: mode switching under accumulation — alternating truthy/falsey flags,
/// all in one capture window.
#[test]
fn cfg_23_driver_alternating() {
    let mut rng = Rng::new();
    let flags: Vec<c_int> = (0..512)
        .map(|i| if i % 2 == 0 { 0 } else { rng.next_i32() | 1 })
        .collect();
    assert_same("driver() alternating 0/nonzero x512", |l| {
        for &f in &flags {
            unsafe { (l.driver)(f) }
        }
    });
}

// ---------------------------------------------------------------------------
// composed pipeline — row 24
// ---------------------------------------------------------------------------

/// Row 24: a randomized 1024-step program mixing all five entry points in a
/// single capture — the way a real consumer composes the library. Bugs in the
/// composed pipeline are invisible to the per-wrapper tests above.
#[test]
fn cfg_24_interleaved_all_entry_points() {
    #[derive(Clone)]
    enum Step {
        Line(Vec<u8>),
        IntLine(i32),
        Bad,
        Good,
        Driver(c_int),
    }

    let mut rng = Rng::new();
    let mut steps = Vec::with_capacity(1024);
    for _ in 0..1024 {
        steps.push(match rng.below(5) {
            0 => {
                let len = rng.below(48) as usize;
                let bytes: Vec<u8> = (0..len).map(|_| rng.nonzero_byte()).collect();
                Step::Line(cstr(&bytes))
            }
            1 => Step::IntLine(rng.next_i32()),
            2 => Step::Bad,
            3 => Step::Good,
            _ => Step::Driver(rng.next_i32()),
        });
    }

    assert_same("interleaved program over all 5 entry points (1024 steps)", |l| {
        for s in &steps {
            unsafe {
                match s {
                    Step::Line(b) => (l.print_line)(b.as_ptr().cast()),
                    Step::IntLine(v) => (l.print_int_line)(*v),
                    Step::Bad => (l.bad)(),
                    Step::Good => (l.good)(),
                    Step::Driver(v) => (l.driver)(*v),
                }
            }
        }
    });
}

// ---------------------------------------------------------------------------
// sanity: the harness itself must be able to observe a difference
// ---------------------------------------------------------------------------

/// Documents the limit of what ANY stdout-based differential test can prove
/// about this library, established from the C side alone.
///
/// In the C ground truth `bad()`, `good()`, `driver(0)` and `driver(1)` all emit
/// exactly `"0\n"`: `bad`/`good` differ only in their `alloca` size (10 vs 40
/// bytes) and both print `data[0]`, which the copy loop always sets to 0. So the
/// `useGood` routing is *unobservable* through the library's only output channel.
///
/// Consequence, recorded here so it is not mistaken for test coverage: a mutation
/// that sends `driver` to the wrong branch cannot be caught behaviourally (see
/// MUTATION.md). Routing is instead pinned by source inspection — the Rust
/// `if useGood != 0 { good() } else { bad() }` matches the C `if (useGood)`.
/// If this test ever FAILS, the premise broke and routing became observable, at
/// which point `err_07b` becomes a real routing check.
#[test]
fn cfg_equivalence_premise_bad_and_good_are_output_identical() {
    let l = libs();
    for lib in [&l.c, &l.rust] {
        let bad = capture(|| unsafe { (lib.bad)() });
        let good = capture(|| unsafe { (lib.good)() });
        assert_eq!(bad, b"0\n", "{}: bad() must emit 0\\n", lib.name);
        assert_eq!(
            bad, good,
            "{}: bad()/good() are output-identical in the C ground truth",
            lib.name
        );
    }
}

/// Guards against a false-negative harness: if `capture` silently returned
/// nothing, every `assert_same` above would pass vacuously.
#[test]
fn cfg_harness_actually_observes_output() {
    let l = libs();
    let c = capture(|| unsafe { (l.c.print_int_line)(1234) });
    let r = capture(|| unsafe { (l.rust.print_int_line)(1234) });
    assert_eq!(c, b"1234\n", "C capture wrong; harness is broken");
    assert_eq!(r, b"1234\n", "Rust capture wrong; harness is broken");

    // Different inputs must produce different captures.
    let other = capture(|| unsafe { (l.c.print_int_line)(5678) });
    assert_ne!(c, other, "capture is not input-sensitive; harness is broken");
}
