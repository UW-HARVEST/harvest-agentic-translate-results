// Phase B — valid-path differential tests.
//
// One test per row of CONFIGS.md. Both libraries are loaded from their `.so`
// and driven through their exported symbols only; every row with a data axis is
// driven with many randomized inputs from a fixed seed.

mod harness;

use harness::{diff, diff_exact, CBuf, Rng, SEED};

const GOOD_LINE: &[u8] = b"helperGood1 string\n";

// ---------------------------------------------------------------------------
// C1 — printLine, non-NULL, length 0
// ---------------------------------------------------------------------------

#[test]
fn c1_print_line_empty_payload() {
    let buf = CBuf::new(b"");
    diff_exact("C1 printLine(\"\")", b"\n", |lib| lib.print_line(buf.as_ptr()));
}

// ---------------------------------------------------------------------------
// C2 — printLine, length 1, exhaustive over every legal byte 0x01..=0xFF
// ---------------------------------------------------------------------------

#[test]
fn c2_print_line_every_single_byte() {
    for b in 1u8..=255 {
        let buf = CBuf::new(&[b]);
        let expected = [b, b'\n'];
        diff_exact(&format!("C2 printLine([{b:#04x}])"), &expected, |lib| {
            lib.print_line(buf.as_ptr())
        });
    }
}

// ---------------------------------------------------------------------------
// C3 — printLine, randomized printable ASCII, len 1..=64
// ---------------------------------------------------------------------------

#[test]
fn c3_print_line_random_ascii() {
    let mut rng = Rng::new(SEED ^ 0xC3);
    for i in 0..512 {
        let len = rng.range(1, 64);
        let payload = rng.bytes_ascii(len);
        let buf = CBuf::new(&payload);
        let mut expected = payload.clone();
        expected.push(b'\n');
        diff_exact(&format!("C3 iter {i} len {len}"), &expected, |lib| {
            lib.print_line(buf.as_ptr())
        });
    }
}

// ---------------------------------------------------------------------------
// C4 — printLine, randomized arbitrary non-NUL bytes (non-UTF-8 included)
// ---------------------------------------------------------------------------

#[test]
fn c4_print_line_random_arbitrary_bytes() {
    let mut rng = Rng::new(SEED ^ 0xC4);
    for i in 0..512 {
        let len = rng.range(1, 128);
        let payload = rng.bytes_nonzero(len);
        let buf = CBuf::new(&payload);
        let mut expected = payload.clone();
        expected.push(b'\n');
        diff_exact(&format!("C4 iter {i} len {len}"), &expected, |lib| {
            lib.print_line(buf.as_ptr())
        });
    }
}

// ---------------------------------------------------------------------------
// C5 — printLine, payloads laced with control bytes
// ---------------------------------------------------------------------------

#[test]
fn c5_print_line_embedded_control_bytes() {
    let mut rng = Rng::new(SEED ^ 0xC5);
    const CTRL: [u8; 6] = [b'\n', b'\r', b'\t', 0x0b, 0x0c, 0x07];
    for i in 0..256 {
        let len = rng.range(1, 64);
        let payload: Vec<u8> = (0..len)
            .map(|_| {
                if rng.below(3) == 0 {
                    CTRL[rng.below(CTRL.len())]
                } else {
                    rng.ascii_byte()
                }
            })
            .collect();
        let buf = CBuf::new(&payload);
        let mut expected = payload.clone();
        expected.push(b'\n');
        diff_exact(&format!("C5 iter {i} len {len}"), &expected, |lib| {
            lib.print_line(buf.as_ptr())
        });
    }
}

// ---------------------------------------------------------------------------
// C6 — printLine, printf format specifiers must be emitted verbatim
// ---------------------------------------------------------------------------

#[test]
fn c6_print_line_format_specifiers_verbatim() {
    let cases: [&[u8]; 12] = [
        b"%s",
        b"%n",
        b"%d",
        b"%%",
        b"%s%s%s%s",
        b"%1000000d",
        b"%.2147483647f",
        b"100%% sure",
        b"%p %x %o %u %c",
        b"%-*.*s",
        b"%",
        b"trailing %",
    ];
    for case in cases {
        let buf = CBuf::new(case);
        let mut expected = case.to_vec();
        expected.push(b'\n');
        diff_exact(
            &format!("C6 printLine({:?})", String::from_utf8_lossy(case)),
            &expected,
            |lib| lib.print_line(buf.as_ptr()),
        );
    }
}

// ---------------------------------------------------------------------------
// C7 — printLine, lengths straddling libc's 4096-byte stdout buffer
// ---------------------------------------------------------------------------

#[test]
fn c7_print_line_stdio_buffer_boundaries() {
    let mut rng = Rng::new(SEED ^ 0xC7);
    for len in [4094usize, 4095, 4096, 4097, 4098, 8191, 8192, 8193] {
        let payload = rng.bytes_ascii(len);
        let buf = CBuf::new(&payload);
        let mut expected = payload.clone();
        expected.push(b'\n');
        diff_exact(&format!("C7 len {len}"), &expected, |lib| {
            lib.print_line(buf.as_ptr())
        });
    }
}

// ---------------------------------------------------------------------------
// C8 — printLine, long randomized payloads
// ---------------------------------------------------------------------------

#[test]
fn c8_print_line_long_random_payloads() {
    let mut rng = Rng::new(SEED ^ 0xC8);
    for i in 0..64 {
        let len = rng.range(1, 20_000);
        let payload = rng.bytes_nonzero(len);
        let buf = CBuf::new(&payload);
        let mut expected = payload.clone();
        expected.push(b'\n');
        diff_exact(&format!("C8 iter {i} len {len}"), &expected, |lib| {
            lib.print_line(buf.as_ptr())
        });
    }
}

// ---------------------------------------------------------------------------
// C9 — printLine, terminator is the final byte of the allocation
// ---------------------------------------------------------------------------

#[test]
fn c9_print_line_no_slack_after_terminator() {
    let mut rng = Rng::new(SEED ^ 0xC9);
    for i in 0..256 {
        let len = rng.range(0, 300);
        let payload = rng.bytes_nonzero(len);
        // CBuf is a boxed slice of exactly len+1 bytes: nothing readable
        // follows the NUL terminator.
        let buf = CBuf::new(&payload);
        let mut expected = payload.clone();
        expected.push(b'\n');
        diff_exact(&format!("C9 iter {i} len {len}"), &expected, |lib| {
            lib.print_line(buf.as_ptr())
        });
    }
}

// ---------------------------------------------------------------------------
// C10 — printLine, same pointer N times
// ---------------------------------------------------------------------------

#[test]
fn c10_print_line_repeated_same_pointer() {
    let mut rng = Rng::new(SEED ^ 0x10);
    for i in 0..128 {
        let n = rng.range(2, 16);
        let len = rng.range(1, 40);
        let payload = rng.bytes_ascii(len);
        let buf = CBuf::new(&payload);
        let mut expected = Vec::new();
        for _ in 0..n {
            expected.extend_from_slice(&payload);
            expected.push(b'\n');
        }
        diff_exact(&format!("C10 iter {i} n {n}"), &expected, |lib| {
            for _ in 0..n {
                lib.print_line(buf.as_ptr());
            }
        });
    }
}

// ---------------------------------------------------------------------------
// C11 — printLine, randomized sequence of distinct payloads in one capture
// ---------------------------------------------------------------------------

#[test]
fn c11_print_line_sequence_of_distinct_payloads() {
    let mut rng = Rng::new(SEED ^ 0x11);
    for i in 0..128 {
        let n = rng.range(1, 12);
        let payloads: Vec<Vec<u8>> = (0..n)
            .map(|_| {
                let len = rng.range(0, 80);
                rng.bytes_nonzero(len)
            })
            .collect();
        let bufs: Vec<CBuf> = payloads.iter().map(|p| CBuf::new(p)).collect();
        let mut expected = Vec::new();
        for p in &payloads {
            expected.extend_from_slice(p);
            expected.push(b'\n');
        }
        diff_exact(&format!("C11 iter {i} n {n}"), &expected, |lib| {
            for b in &bufs {
                lib.print_line(b.as_ptr());
            }
        });
    }
}

// ---------------------------------------------------------------------------
// C12 / C13 — good()
// ---------------------------------------------------------------------------

#[test]
fn c12_good_single_call() {
    diff_exact("C12 good()", GOOD_LINE, |lib| lib.call_good());
}

#[test]
fn c13_good_repeated_static_storage_stability() {
    let mut rng = Rng::new(SEED ^ 0x13);
    for i in 0..64 {
        let n = rng.range(2, 32);
        let expected: Vec<u8> = GOOD_LINE.repeat(n);
        diff_exact(&format!("C13 iter {i} n {n}"), &expected, |lib| {
            for _ in 0..n {
                lib.call_good();
            }
        });
    }
}

// ---------------------------------------------------------------------------
// C14 / C15 — bad()
// ---------------------------------------------------------------------------

#[test]
fn c14_bad_single_call() {
    diff_exact("C14 bad()", b"", |lib| lib.call_bad());
}

#[test]
fn c15_bad_repeated() {
    let mut rng = Rng::new(SEED ^ 0x15);
    for i in 0..64 {
        let n = rng.range(2, 32);
        diff_exact(&format!("C15 iter {i} n {n}"), b"", |lib| {
            for _ in 0..n {
                lib.call_bad();
            }
        });
    }
}

// ---------------------------------------------------------------------------
// C16..C20 — driver() branch selection
// ---------------------------------------------------------------------------

#[test]
fn c16_driver_zero_takes_bad_branch() {
    diff_exact("C16 driver(0)", b"", |lib| lib.call_driver(0));
}

#[test]
fn c17_driver_one_takes_good_branch() {
    diff_exact("C17 driver(1)", GOOD_LINE, |lib| lib.call_driver(1));
}

#[test]
fn c18_driver_minus_one() {
    diff_exact("C18 driver(-1)", GOOD_LINE, |lib| lib.call_driver(-1));
}

#[test]
fn c19_driver_int_extremes() {
    diff_exact("C19 driver(INT_MAX)", GOOD_LINE, |lib| {
        lib.call_driver(i32::MAX)
    });
    diff_exact("C19 driver(INT_MIN)", GOOD_LINE, |lib| {
        lib.call_driver(i32::MIN)
    });
}

/// The trap row: these values are non-zero but their low byte is 0, so any
/// translation that tested `useGood as u8 != 0` (or compared against 1) would
/// diverge here.
#[test]
fn c20_driver_low_byte_zero_but_nonzero() {
    for v in [
        0x100i32,
        0x10000,
        0x0100_0000,
        0x7f00,
        -256,
        -65536,
        i32::MIN,
        0x4000_0000,
        0x0000_FF00,
    ] {
        diff_exact(&format!("C20 driver({v:#x})"), GOOD_LINE, |lib| {
            lib.call_driver(v)
        });
    }
}

// ---------------------------------------------------------------------------
// C21 — driver, randomized over the full i32 range
// ---------------------------------------------------------------------------

#[test]
fn c21_driver_random_i32() {
    let mut rng = Rng::new(SEED ^ 0x21);
    for i in 0..2048 {
        // Deliberately mix in zero so both branches are hit.
        let v = if rng.below(8) == 0 { 0 } else { rng.next_i32() };
        let expected: &[u8] = if v != 0 { GOOD_LINE } else { b"" };
        diff_exact(&format!("C21 iter {i} driver({v})"), expected, |lib| {
            lib.call_driver(v)
        });
    }
}

// ---------------------------------------------------------------------------
// C22 — driver, repeated calls on both branches
// ---------------------------------------------------------------------------

#[test]
fn c22_driver_repeated_both_branches() {
    let mut rng = Rng::new(SEED ^ 0x22);
    for i in 0..64 {
        let n = rng.range(2, 24);
        let v = rng.next_i32();
        let expected: Vec<u8> = if v != 0 {
            GOOD_LINE.repeat(n)
        } else {
            Vec::new()
        };
        diff_exact(&format!("C22 iter {i} n {n} v {v}"), &expected, |lib| {
            for _ in 0..n {
                lib.call_driver(v);
            }
        });
    }
}

// ---------------------------------------------------------------------------
// C23 — randomized interleaving of the two mid-level entry points
// ---------------------------------------------------------------------------

#[test]
fn c23_interleave_bad_and_good() {
    let mut rng = Rng::new(SEED ^ 0x23);
    for i in 0..128 {
        let n = rng.range(1, 20);
        let plan: Vec<bool> = (0..n).map(|_| rng.below(2) == 0).collect();
        let mut expected = Vec::new();
        for &is_good in &plan {
            if is_good {
                expected.extend_from_slice(GOOD_LINE);
            }
        }
        diff_exact(&format!("C23 iter {i} n {n}"), &expected, |lib| {
            for &is_good in &plan {
                if is_good {
                    lib.call_good()
                } else {
                    lib.call_bad()
                }
            }
        });
    }
}

// ---------------------------------------------------------------------------
// C24 — randomized programs over all four entry points (composed pipeline)
// ---------------------------------------------------------------------------

enum Step {
    PrintLine(CBuf),
    PrintNull,
    Bad,
    Good,
    Driver(i32),
}

#[test]
fn c24_random_programs_over_all_entry_points() {
    let mut rng = Rng::new(SEED ^ 0x24);
    for prog in 0..256 {
        let steps_n = rng.range(1, 24);
        let mut steps = Vec::with_capacity(steps_n);
        let mut expected = Vec::new();
        for _ in 0..steps_n {
            match rng.below(5) {
                0 => {
                    let len = rng.range(0, 100);
                    let payload = rng.bytes_nonzero(len);
                    expected.extend_from_slice(&payload);
                    expected.push(b'\n');
                    steps.push(Step::PrintLine(CBuf::new(&payload)));
                }
                1 => steps.push(Step::PrintNull),
                2 => steps.push(Step::Bad),
                3 => {
                    expected.extend_from_slice(GOOD_LINE);
                    steps.push(Step::Good);
                }
                _ => {
                    let v = if rng.below(4) == 0 { 0 } else { rng.next_i32() };
                    if v != 0 {
                        expected.extend_from_slice(GOOD_LINE);
                    }
                    steps.push(Step::Driver(v));
                }
            }
        }

        diff_exact(
            &format!("C24 program {prog} ({steps_n} steps)"),
            &expected,
            |lib| {
                for step in &steps {
                    match step {
                        Step::PrintLine(buf) => lib.print_line(buf.as_ptr()),
                        Step::PrintNull => lib.print_line(std::ptr::null()),
                        Step::Bad => lib.call_bad(),
                        Step::Good => lib.call_good(),
                        Step::Driver(v) => lib.call_driver(*v),
                    }
                }
            },
        );
    }
}

// ---------------------------------------------------------------------------
// C25 — wrapper/callee equivalence, cross-checked in both libraries
// ---------------------------------------------------------------------------

#[test]
fn c25_driver_equivalent_to_the_branch_it_dispatches() {
    let (c, r) = harness::libs();
    let mut rng = Rng::new(SEED ^ 0x25);

    // driver(0) must be indistinguishable from bad(), and driver(nz) from
    // good() -- verified inside each library as well as across them.
    for lib in [c, r] {
        let via_driver = harness::capture(|| lib.call_driver(0));
        let via_bad = harness::capture(|| lib.call_bad());
        assert_eq!(
            via_driver, via_bad,
            "[{}] driver(0) != bad()",
            lib.which
        );

        for _ in 0..64 {
            let v = rng.next_i32() | 1; // guaranteed non-zero
            let via_driver = harness::capture(|| lib.call_driver(v));
            let via_good = harness::capture(|| lib.call_good());
            assert_eq!(
                via_driver, via_good,
                "[{}] driver({v}) != good()",
                lib.which
            );
        }
    }

    // And the cross-library differential form of the same claim.
    diff("C25 driver(0) vs bad", |lib| {
        lib.call_driver(0);
        lib.call_bad();
    });
    diff("C25 driver(nz) vs good", |lib| {
        lib.call_driver(7);
        lib.call_good();
    });
}
