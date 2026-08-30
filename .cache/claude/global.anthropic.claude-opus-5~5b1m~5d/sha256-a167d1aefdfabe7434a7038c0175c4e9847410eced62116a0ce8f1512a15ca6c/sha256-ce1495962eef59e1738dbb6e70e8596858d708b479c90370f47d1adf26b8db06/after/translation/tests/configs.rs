// Phase B — valid-path differential tests.
//
// One test per row of CONFIGS.md. Every case drives BOTH `.so`s through their
// exported symbols and compares captured stdout byte-for-byte. Randomized rows
// use the fixed-seed splitmix64 in `common`, so failures reproduce exactly.
//
// `harness = false` — the cases must run sequentially; see
// `tests/common/mod.rs::Runner` for why.

mod common;

use common::*;
use std::ffi::c_char;

fn main() {
    let mut r = Runner::new("configs (Phase B / CONFIGS.md)");
    r.case("c1_print_line_single_byte_all_values", c1_print_line_single_byte_all_values);
    r.case("c2_print_line_empty", c2_print_line_empty);
    r.case("c3_print_line_short_ascii_random", c3_print_line_short_ascii_random);
    r.case("c4_print_line_arbitrary_bytes_random", c4_print_line_arbitrary_bytes_random);
    r.case("c5_print_line_format_metachars_random", c5_print_line_format_metachars_random);
    r.case("c6_print_line_control_bytes_random", c6_print_line_control_bytes_random);
    r.case("c7_print_line_buffer_boundary_lengths", c7_print_line_buffer_boundary_lengths);
    r.case("c8_print_line_interior_pointer_random", c8_print_line_interior_pointer_random);
    r.case("c9_print_line_stops_at_nul", c9_print_line_stops_at_nul);
    r.case("c10_print_line_null", c10_print_line_null);
    r.case("c11_good_single_call", c11_good_single_call);
    r.case("c12_good_repeated", c12_good_repeated);
    r.case("c13_bad_single_call", c13_bad_single_call);
    r.case("c14_bad_repeated", c14_bad_repeated);
    r.case("c15_driver_zero", c15_driver_zero);
    r.case("c16_driver_one", c16_driver_one);
    r.case("c17_driver_random_nonzero_positive", c17_driver_random_nonzero_positive);
    r.case("c18_driver_random_negative", c18_driver_random_negative);
    r.case("c19_driver_extremal_ints", c19_driver_extremal_ints);
    r.case("c20_driver_random_sequence", c20_driver_random_sequence);
    r.case("c21_mixed_entry_point_program", c21_mixed_entry_point_program);
    r.case("c22_print_line_around_good", c22_print_line_around_good);
    r.finish();
}

// ---------------------------------------------------------------------------
// C1 — printLine, length 1, every possible non-NUL byte value
// ---------------------------------------------------------------------------

fn c1_print_line_single_byte_all_values() {
    for b in 1u8..=255 {
        let payload = [b];
        with_cstr(&payload, |p| {
            assert_same_and_eq(&format!("C1 byte 0x{b:02x}"), &expected_line(&payload), |lib| {
                unsafe { lib.print_line_raw(p) }
            });
        });
    }
}

// ---------------------------------------------------------------------------
// C2 — printLine, empty string (length 0, non-NULL)
// ---------------------------------------------------------------------------

fn c2_print_line_empty() {
    with_cstr(b"", |p| {
        assert_same_and_eq("C2 empty string", b"\n", |lib| unsafe { lib.print_line_raw(p) });
    });
}

// ---------------------------------------------------------------------------
// C3 — printLine, random printable ASCII, lengths 2..=64
// ---------------------------------------------------------------------------

fn c3_print_line_short_ascii_random() {
    let mut rng = Rng::new(3);
    for i in 0..200 {
        let len = 2 + rng.below(63);
        let payload: Vec<u8> = (0..len).map(|_| 0x20 + rng.nonzero_byte() % 0x5f).collect();
        with_cstr(&payload, |p| {
            assert_same_and_eq(&format!("C3 #{i} len={len}"), &expected_line(&payload), |lib| {
                unsafe { lib.print_line_raw(p) }
            });
        });
    }
}

// ---------------------------------------------------------------------------
// C4 — printLine, arbitrary bytes 0x01..=0xFF, lengths 0..=512
//      (deliberately includes sequences that are invalid UTF-8)
// ---------------------------------------------------------------------------

fn c4_print_line_arbitrary_bytes_random() {
    let mut rng = Rng::new(4);
    for i in 0..2000 {
        let len = rng.below(513);
        let payload = rng.nonzero_bytes(len);
        with_cstr(&payload, |p| {
            assert_same_and_eq(&format!("C4 #{i} len={len}"), &expected_line(&payload), |lib| {
                unsafe { lib.print_line_raw(p) }
            });
        });
    }
}

// ---------------------------------------------------------------------------
// C5 — printLine, printf conversion specifiers embedded in the payload
// ---------------------------------------------------------------------------

const METACHARS: &[&[u8]] = &[
    b"%s", b"%d", b"%n", b"%%", b"%p", b"%x", b"%1$s", b"%99999d", b"%.2147483647f", b"%*d",
    b"%hhn", b"%ln", b"%-+ #0", b"%c", b"%S", b"%99999999s",
];

fn c5_print_line_format_metachars_random() {
    let mut rng = Rng::new(5);
    for i in 0..300 {
        let mut payload: Vec<u8> = Vec::new();
        let chunks = 1 + rng.below(8);
        for _ in 0..chunks {
            payload.extend_from_slice(METACHARS[rng.below(METACHARS.len())]);
            let filler = rng.below(6);
            for _ in 0..filler {
                payload.push(b'a' + (rng.below(26) as u8));
            }
        }
        with_cstr(&payload, |p| {
            assert_same_and_eq(&format!("C5 #{i}"), &expected_line(&payload), |lib| unsafe {
                lib.print_line_raw(p)
            });
        });
    }
}

// ---------------------------------------------------------------------------
// C6 — printLine, control / whitespace bytes
// ---------------------------------------------------------------------------

fn c6_print_line_control_bytes_random() {
    const CTRL: &[u8] = &[b'\n', b'\r', b'\t', 0x0b, 0x0c, 0x1b, 0x07, 0x08, 0x7f];
    let mut rng = Rng::new(6);
    for i in 0..300 {
        let len = rng.below(128);
        let payload: Vec<u8> = (0..len)
            .map(|_| {
                if rng.below(2) == 0 {
                    CTRL[rng.below(CTRL.len())]
                } else {
                    b'a' + (rng.below(26) as u8)
                }
            })
            .collect();
        with_cstr(&payload, |p| {
            assert_same_and_eq(&format!("C6 #{i} len={len}"), &expected_line(&payload), |lib| {
                unsafe { lib.print_line_raw(p) }
            });
        });
    }
}

// ---------------------------------------------------------------------------
// C7 — printLine, lengths that straddle stdio buffer boundaries
// ---------------------------------------------------------------------------

fn c7_print_line_buffer_boundary_lengths() {
    const LENS: &[usize] = &[
        0, 1, 2, 3, 7, 15, 16, 17, 31, 32, 33, 63, 64, 65, 127, 128, 129, 255, 256, 257, 511, 512,
        513, 1023, 1024, 1025, 2047, 2048, 2049, 4095, 4096, 4097, 8191, 8192, 8193, 16383, 16384,
        16385, 65535, 65536, 65537, 1 << 20,
    ];
    let mut rng = Rng::new(7);
    for &len in LENS {
        // Non-constant content so a length-only bug cannot hide behind a run of
        // identical bytes.
        let payload: Vec<u8> = (0..len).map(|k| 1 + ((k as u64 + rng.next_u64() % 7) % 255) as u8).collect();
        with_cstr(&payload, |p| {
            assert_same_and_eq(&format!("C7 len={len}"), &expected_line(&payload), |lib| unsafe {
                lib.print_line_raw(p)
            });
        });
    }
}

// ---------------------------------------------------------------------------
// C8 — printLine with an interior (possibly unaligned) pointer
// ---------------------------------------------------------------------------

fn c8_print_line_interior_pointer_random() {
    let mut rng = Rng::new(8);
    for i in 0..200 {
        let total = 1 + rng.below(300);
        let mut buf = rng.nonzero_bytes(total);
        buf.push(0);
        let k = rng.below(total); // 0..total-1, includes odd offsets
        let tail = buf[k..total].to_vec();
        let p = unsafe { buf.as_ptr().add(k) } as *const c_char;
        assert_same_and_eq(
            &format!("C8 #{i} total={total} offset={k}"),
            &expected_line(&tail),
            |lib| unsafe { lib.print_line_raw(p) },
        );
        // Keep `buf` alive across the calls above.
        assert_eq!(buf.len(), total + 1);
    }
}

// ---------------------------------------------------------------------------
// C9 — printLine must stop at the first NUL, ignoring trailing garbage
// ---------------------------------------------------------------------------

fn c9_print_line_stops_at_nul() {
    let mut rng = Rng::new(9);
    for i in 0..200 {
        let visible_len = rng.below(64);
        let visible = rng.nonzero_bytes(visible_len);
        let mut buf = visible.clone();
        buf.push(0);
        // Garbage after the terminator, including more NULs.
        let junk = 1 + rng.below(32);
        for _ in 0..junk {
            buf.push(if rng.below(4) == 0 { 0 } else { rng.nonzero_byte() });
        }
        let p = buf.as_ptr() as *const c_char;
        assert_same_and_eq(
            &format!("C9 #{i} visible={visible_len}"),
            &expected_line(&visible),
            |lib| unsafe { lib.print_line_raw(p) },
        );
        assert!(buf.len() > visible_len);
    }
}

// ---------------------------------------------------------------------------
// C10 — printLine(NULL): the other side of the `if (line != NULL)` guard
// ---------------------------------------------------------------------------

fn c10_print_line_null() {
    assert_same_and_eq("C10 NULL", b"", |lib| unsafe {
        lib.print_line_raw(std::ptr::null())
    });
}

// ---------------------------------------------------------------------------
// C11 / C12 — good()
// ---------------------------------------------------------------------------

fn c11_good_single_call() {
    assert_same_and_eq("C11 good once", GOOD_OUTPUT, |lib| unsafe { lib.good_raw() });
}

fn c12_good_repeated() {
    let expected: Vec<u8> = GOOD_OUTPUT.repeat(256);
    assert_same_and_eq("C12 good x256", &expected, |lib| unsafe {
        for _ in 0..256 {
            lib.good_raw();
        }
    });
}

// ---------------------------------------------------------------------------
// C13 / C14 — bad()
// ---------------------------------------------------------------------------

fn c13_bad_single_call() {
    assert_same_and_eq("C13 bad once", BAD_OUTPUT, |lib| unsafe { lib.bad_raw() });
}

fn c14_bad_repeated() {
    assert_same_and_eq("C14 bad x256", b"", |lib| unsafe {
        for _ in 0..256 {
            lib.bad_raw();
        }
    });
}

// ---------------------------------------------------------------------------
// C15 / C16 — driver(0) and driver(1)
// ---------------------------------------------------------------------------

fn c15_driver_zero() {
    assert_same_and_eq("C15 driver(0)", BAD_OUTPUT, |lib| unsafe { lib.driver_raw(0) });
}

fn c16_driver_one() {
    assert_same_and_eq("C16 driver(1)", GOOD_OUTPUT, |lib| unsafe { lib.driver_raw(1) });
}

// ---------------------------------------------------------------------------
// C17 / C18 — random non-zero ints, positive and negative
// ---------------------------------------------------------------------------

fn c17_driver_random_nonzero_positive() {
    let mut rng = Rng::new(17);
    for i in 0..500 {
        let v = (rng.next_u32() % (i32::MAX as u32)) as i32 + 1; // 1..=i32::MAX
        assert!(v > 0);
        assert_same_and_eq(&format!("C17 #{i} driver({v})"), GOOD_OUTPUT, |lib| unsafe {
            lib.driver_raw(v)
        });
    }
}

fn c18_driver_random_negative() {
    let mut rng = Rng::new(18);
    for i in 0..500 {
        let v = -((rng.next_u32() % (i32::MAX as u32)) as i32) - 1; // i32::MIN..=-1
        assert!(v < 0);
        assert_same_and_eq(&format!("C18 #{i} driver({v})"), GOOD_OUTPUT, |lib| unsafe {
            lib.driver_raw(v)
        });
    }
}

// ---------------------------------------------------------------------------
// C19 — extremal ints, plus values whose low 32 bits are zero
// ---------------------------------------------------------------------------

fn c19_driver_extremal_ints() {
    let cases: &[(i32, &[u8])] = &[
        (i32::MIN, GOOD_OUTPUT),
        (i32::MIN + 1, GOOD_OUTPUT),
        (-2, GOOD_OUTPUT),
        (-1, GOOD_OUTPUT),
        (0, BAD_OUTPUT),
        (1, GOOD_OUTPUT),
        (2, GOOD_OUTPUT),
        (i32::MAX - 1, GOOD_OUTPUT),
        (i32::MAX, GOOD_OUTPUT),
        (0x0000_FFFF, GOOD_OUTPUT),
        (0x0001_0000, GOOD_OUTPUT),
        (0x7FFF_FFFE, GOOD_OUTPUT),
    ];
    for &(v, exp) in cases {
        assert_same_and_eq(&format!("C19 driver({v})"), exp, |lib| unsafe { lib.driver_raw(v) });
    }

    // A 64-bit value whose low 32 bits are zero must be seen as 0 by an `int`
    // parameter: the upper half of the argument register carries no meaning in
    // the SysV ABI. Both libraries must agree on that truncation.
    let truncated = 0x0000_0001_0000_0000u64 as u32 as i32;
    assert_eq!(truncated, 0);
    assert_same_and_eq("C19 driver(0x1_0000_0000 truncated)", BAD_OUTPUT, |lib| unsafe {
        lib.driver_raw(truncated)
    });
}

// ---------------------------------------------------------------------------
// C20 — long random sequence of driver() calls through one entry point
// ---------------------------------------------------------------------------

fn c20_driver_random_sequence() {
    let mut rng = Rng::new(20);
    let seq: Vec<i32> = (0..1000)
        .map(|_| match rng.below(4) {
            0 => 0,
            1 => rng.next_u32() as i32,
            2 => -(rng.below(1000) as i32) - 1,
            _ => rng.below(3) as i32, // 0, 1 or 2
        })
        .collect();
    let mut expected: Vec<u8> = Vec::new();
    for &v in &seq {
        expected.extend_from_slice(if v != 0 { GOOD_OUTPUT } else { BAD_OUTPUT });
    }
    assert_same_and_eq("C20 driver sequence x1000", &expected, |lib| unsafe {
        for &v in &seq {
            lib.driver_raw(v);
        }
    });
}

// ---------------------------------------------------------------------------
// C21 — composed pipeline: all four entry points interleaved in one capture
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
enum Op {
    PrintLine(Vec<u8>),
    PrintNull,
    Bad,
    Good,
    Driver(i32),
}

fn c21_mixed_entry_point_program() {
    let mut rng = Rng::new(21);
    let program: Vec<Op> = (0..600)
        .map(|_| match rng.below(5) {
            0 => {
                let n = rng.below(80);
                Op::PrintLine(rng.nonzero_bytes(n))
            }
            1 => Op::PrintNull,
            2 => Op::Bad,
            3 => Op::Good,
            _ => Op::Driver(if rng.below(3) == 0 { 0 } else { rng.next_u32() as i32 }),
        })
        .collect();

    let mut expected: Vec<u8> = Vec::new();
    for op in &program {
        match op {
            Op::PrintLine(p) => expected.extend_from_slice(&expected_line(p)),
            Op::PrintNull => {}
            Op::Bad => expected.extend_from_slice(BAD_OUTPUT),
            Op::Good => expected.extend_from_slice(GOOD_OUTPUT),
            Op::Driver(v) => {
                expected.extend_from_slice(if *v != 0 { GOOD_OUTPUT } else { BAD_OUTPUT })
            }
        }
    }

    assert_same_and_eq("C21 mixed program x600", &expected, |lib| {
        for op in &program {
            unsafe {
                match op {
                    Op::PrintLine(p) => with_cstr(p, |q| lib.print_line_raw(q)),
                    Op::PrintNull => lib.print_line_raw(std::ptr::null()),
                    Op::Bad => lib.bad_raw(),
                    Op::Good => lib.good_raw(),
                    Op::Driver(v) => lib.driver_raw(*v),
                }
            }
        }
    });
}

// ---------------------------------------------------------------------------
// C22 — printLine immediately before/after good(): the `.data` static that
//       helperGood1 hands out must not be aliased or clobbered by caller data
// ---------------------------------------------------------------------------

fn c22_print_line_around_good() {
    let mut rng = Rng::new(22);
    for i in 0..200 {
        // Same length as "helperGood1 string" for some cases, so an aliasing bug
        // would show up as the caller's bytes leaking into good()'s output.
        let len = if i % 3 == 0 { 18 } else { rng.below(40) };
        let payload = rng.nonzero_bytes(len);
        let mut expected = expected_line(&payload);
        expected.extend_from_slice(GOOD_OUTPUT);
        expected.extend_from_slice(&expected_line(&payload));
        expected.extend_from_slice(GOOD_OUTPUT);
        expected.extend_from_slice(BAD_OUTPUT);
        expected.extend_from_slice(&expected_line(&payload));

        with_cstr(&payload, |p| {
            assert_same_and_eq(&format!("C22 #{i} len={len}"), &expected, |lib| unsafe {
                lib.print_line_raw(p);
                lib.good_raw();
                lib.print_line_raw(p);
                lib.driver_raw(1);
                lib.driver_raw(0);
                lib.print_line_raw(p);
            });
        });
    }
}
