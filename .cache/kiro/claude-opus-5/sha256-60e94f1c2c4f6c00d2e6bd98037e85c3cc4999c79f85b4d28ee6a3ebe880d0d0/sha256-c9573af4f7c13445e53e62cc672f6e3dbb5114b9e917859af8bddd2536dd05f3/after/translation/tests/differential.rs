//! Differential tests: C `libdriver.so` vs Rust `libdriver.so`, both loaded
//! with `libloading` and driven only through their exported C symbols.
//!
//! * `phase_b_*` — one test per row of `CONFIGS.md` (valid paths).
//! * `phase_c_*` — one test per row of `ERRORS.md` (rejection paths).
//!
//! Every capture manipulates the process-global fd 1, so the harness serialises
//! them internally with a mutex; the suite is also safe under
//! `--test-threads=1`.

mod harness;

use harness::{assert_same, assert_same_and_eq, cstr, Api, Rng};
use std::ffi::{c_char, c_int};

/// Expected stdout of `bad()` — `data = CHAR_MAX (127)`, `127 * 2` overflows a
/// signed `char` to `-2`, promoted to `int` and printed with `%02x`.
const BAD_OUT: &[u8] = b"fffffffe\n";
/// Expected stdout of `good()` — `goodG2B` prints `2 * 2 = 4`, then `goodB2G`
/// rejects `127` because `127 < CHAR_MAX/2 (63)` is false.
const GOOD_OUT: &[u8] = b"04\ndata value is too large to perform arithmetic safely.\n";
const TOO_LARGE: &[u8] = b"data value is too large to perform arithmetic safely.";

// ===========================================================================
// Phase B — valid-path differential tests (gated on CONFIGS.md)
// ===========================================================================

/// C1: exhaustive sweep of the whole `char` domain.
#[test]
fn phase_b_c1_print_hex_char_line_exhaustive_domain() {
    for v in 0u16..=0xFF {
        let b = v as u8;
        assert_same(&format!("C1 printHexCharLine(0x{b:02x})"), &move |a: &Api| unsafe {
            (a.print_hex_char_line)(b as c_char)
        });
    }
}

/// C2: randomized draws over the full byte domain, fixed seed.
#[test]
fn phase_b_c2_print_hex_char_line_randomized() {
    let mut rng = Rng::new(0xC2_5EED);
    // Batch many values into one capture too, so buffering behaviour is
    // compared over a long stream and not only per single line.
    let vals: Vec<u8> = (0..4096).map(|_| rng.next_u8()).collect();
    for chunk in vals.chunks(64) {
        let c = chunk.to_vec();
        assert_same("C2 printHexCharLine randomized batch", &move |a: &Api| unsafe {
            for &b in &c {
                (a.print_hex_char_line)(b as c_char);
            }
        });
    }
}

/// C3: the 2-digit / 8-digit and zero-pad transition boundaries.
#[test]
fn phase_b_c3_print_hex_char_line_boundaries() {
    let cases: [(u8, &[u8]); 7] = [
        (0x00, b"00\n"),
        (0x01, b"01\n"),
        (0x0F, b"0f\n"),
        (0x10, b"10\n"),
        (0x7F, b"7f\n"),        // CHAR_MAX
        (0x80, b"ffffff80\n"),  // CHAR_MIN, sign-extended
        (0xFF, b"ffffffff\n"),
    ];
    for (b, expect) in cases {
        assert_same_and_eq(
            &format!("C3 printHexCharLine(0x{b:02x})"),
            expect,
            &move |a: &Api| unsafe { (a.print_hex_char_line)(b as c_char) },
        );
    }
}

/// C4: the `char` parameter reached with dirty upper argument-register bits.
#[test]
fn phase_b_c4_print_hex_char_line_dirty_upper_bits() {
    let cases: [i32; 8] = [
        0x1FF,
        0x100,
        0xDEADBE7Fu32 as i32,
        -256,
        i32::MIN,
        i32::MAX,
        0x7FFF_FF80u32 as i32,
        0x0000_FF00,
    ];
    for v in cases {
        assert_same(&format!("C4 printHexCharLine(int {v:#x})"), &move |a: &Api| unsafe {
            (a.print_hex_char_line_int)(v as c_int)
        });
    }
}

/// C5: `printLine(NULL)`.
#[test]
fn phase_b_c5_print_line_null() {
    assert_same_and_eq("C5 printLine(NULL)", b"", &|a: &Api| unsafe {
        (a.print_line)(std::ptr::null())
    });
}

/// C6: `printLine("")`.
#[test]
fn phase_b_c6_print_line_empty() {
    assert_same_and_eq("C6 printLine(\"\")", b"\n", &|a: &Api| unsafe {
        (a.print_line)(b"\0".as_ptr() as *const c_char)
    });
}

/// C7: exhaustive single-byte strings, all 255 non-NUL values.
#[test]
fn phase_b_c7_print_line_all_single_bytes() {
    for v in 1u16..=0xFF {
        let buf = cstr(&[v as u8]);
        assert_same(&format!("C7 printLine([{v:#04x}])"), &move |a: &Api| unsafe {
            (a.print_line)(buf.as_ptr() as *const c_char)
        });
    }
}

/// C8: randomized strings, length 0..64, bytes 0x01..=0xFF, fixed seed.
#[test]
fn phase_b_c8_print_line_randomized() {
    let mut rng = Rng::new(0xC8_5EED);
    for _ in 0..512 {
        let len = rng.below(65) as usize;
        let bytes: Vec<u8> = (0..len)
            .map(|_| {
                // avoid NUL so the whole string survives
                1 + (rng.next_u64() % 255) as u8
            })
            .collect();
        let buf = cstr(&bytes);
        assert_same("C8 printLine randomized", &move |a: &Api| unsafe {
            (a.print_line)(buf.as_ptr() as *const c_char)
        });
    }
}

/// C9: interior NUL — C truncates at the first NUL.
#[test]
fn phase_b_c9_print_line_interior_nul() {
    let cases: [(&[u8], &[u8]); 4] = [
        (b"ab\0cd\0", b"ab\n"),
        (b"\0trailing\0", b"\n"),
        (b"x\0\0\0y\0", b"x\n"),
        (b"long prefix here\0hidden tail\0", b"long prefix here\n"),
    ];
    for (raw, expect) in cases {
        let buf = raw.to_vec();
        assert_same_and_eq("C9 printLine interior NUL", expect, &move |a: &Api| unsafe {
            (a.print_line)(buf.as_ptr() as *const c_char)
        });
    }
}

/// C10: `printf` conversion specifiers carried as *data*.
#[test]
fn phase_b_c10_print_line_format_specifiers() {
    let cases: [&[u8]; 8] = [
        b"%s",
        b"%d %i %u %x",
        b"%n",
        b"%p",
        b"%%",
        b"%s%s%s%s%s%s%s%s",
        b"100%",
        b"%1$s %2$n",
    ];
    for raw in cases {
        let buf = cstr(raw);
        let mut expect = raw.to_vec();
        expect.push(b'\n');
        assert_same_and_eq(
            &format!("C10 printLine({:?})", String::from_utf8_lossy(raw)),
            &expect,
            &move |a: &Api| unsafe { (a.print_line)(buf.as_ptr() as *const c_char) },
        );
    }
}

/// C11: the exact literal `goodB2G` passes to `printLine`.
#[test]
fn phase_b_c11_print_line_goodb2g_literal() {
    let buf = cstr(TOO_LARGE);
    let mut expect = TOO_LARGE.to_vec();
    expect.push(b'\n');
    assert_same_and_eq("C11 printLine(goodB2G literal)", &expect, &move |a: &Api| unsafe {
        (a.print_line)(buf.as_ptr() as *const c_char)
    });
}

/// C12: oversized strings straddling the stdio buffer size.
#[test]
fn phase_b_c12_print_line_oversized() {
    for &len in &[1024usize, 4095, 4096, 4097, 65536] {
        let bytes: Vec<u8> = (0..len).map(|i| b'A' + (i % 26) as u8).collect();
        let buf = cstr(&bytes);
        let mut expect = bytes.clone();
        expect.push(b'\n');
        assert_same_and_eq(
            &format!("C12 printLine len={len}"),
            &expect,
            &move |a: &Api| unsafe { (a.print_line)(buf.as_ptr() as *const c_char) },
        );
    }
}

/// C13: `bad()` called directly (low-level entry point, not via `driver`).
#[test]
fn phase_b_c13_bad_direct() {
    assert_same_and_eq("C13 bad()", BAD_OUT, &|a: &Api| unsafe { (a.bad)() });
}

/// C14: `bad()` repeated — no state may carry between calls.
#[test]
fn phase_b_c14_bad_repeated() {
    let expect: Vec<u8> = BAD_OUT.repeat(16);
    assert_same_and_eq("C14 bad() x16", &expect, &|a: &Api| unsafe {
        for _ in 0..16 {
            (a.bad)();
        }
    });
}

/// C15: `good()` called directly — the composed `goodG2B` then `goodB2G`
/// pipeline, order-sensitive.
#[test]
fn phase_b_c15_good_direct() {
    assert_same_and_eq("C15 good()", GOOD_OUT, &|a: &Api| unsafe { (a.good)() });
}

/// C16: `good()` repeated.
#[test]
fn phase_b_c16_good_repeated() {
    let expect: Vec<u8> = GOOD_OUT.repeat(16);
    assert_same_and_eq("C16 good() x16", &expect, &|a: &Api| unsafe {
        for _ in 0..16 {
            (a.good)();
        }
    });
}

/// C17: `driver(0)` — mode "bad".
#[test]
fn phase_b_c17_driver_zero() {
    assert_same_and_eq("C17 driver(0)", BAD_OUT, &|a: &Api| unsafe { (a.driver)(0) });
}

/// C18: `driver(1)` — mode "good".
#[test]
fn phase_b_c18_driver_one() {
    assert_same_and_eq("C18 driver(1)", GOOD_OUT, &|a: &Api| unsafe { (a.driver)(1) });
}

/// C19: other truthy selector shapes, including low-byte-zero values.
#[test]
fn phase_b_c19_driver_other_truthy() {
    let cases: [i32; 10] = [
        -1,
        2,
        42,
        0x100,
        0x10000,
        0x0100_0000,
        i32::MAX,
        i32::MIN,
        0xFFFF_FF00u32 as i32,
        0x7FFF_FFFF,
    ];
    for v in cases {
        assert_same_and_eq(
            &format!("C19 driver({v:#x})"),
            GOOD_OUT,
            &move |a: &Api| unsafe { (a.driver)(v as c_int) },
        );
    }
}

/// C20: randomized selectors over the whole `i32` domain, fixed seed, with
/// zero deliberately mixed in.
#[test]
fn phase_b_c20_driver_randomized() {
    let mut rng = Rng::new(0x20_5EED);
    let mut vals: Vec<i32> = (0..2048).map(|_| rng.next_u32() as i32).collect();
    // guarantee the falsy path and small values appear
    for extra in [0i32, 0, 1, -1, 0x100, 0] {
        vals.push(extra);
    }
    for chunk in vals.chunks(32) {
        let c = chunk.to_vec();
        let expect: Vec<u8> = c
            .iter()
            .flat_map(|&v| if v == 0 { BAD_OUT.to_vec() } else { GOOD_OUT.to_vec() })
            .collect();
        assert_same_and_eq("C20 driver randomized batch", &expect, &move |a: &Api| unsafe {
            for &v in &c {
                (a.driver)(v as c_int);
            }
        });
    }
}

// --- C21 / C22: mixed pipelines across every entry point --------------------

#[derive(Clone, Debug)]
enum Op {
    Driver(i32),
    Bad,
    Good,
    PrintLine(Option<Vec<u8>>),
    PrintHex(u8),
    PrintHexInt(i32),
}

fn run_program(a: &Api, prog: &[Op]) {
    unsafe {
        for op in prog {
            match op {
                Op::Driver(v) => (a.driver)(*v as c_int),
                Op::Bad => (a.bad)(),
                Op::Good => (a.good)(),
                Op::PrintLine(None) => (a.print_line)(std::ptr::null()),
                Op::PrintLine(Some(buf)) => (a.print_line)(buf.as_ptr() as *const c_char),
                Op::PrintHex(b) => (a.print_hex_char_line)(*b as c_char),
                Op::PrintHexInt(v) => (a.print_hex_char_line_int)(*v as c_int),
            }
        }
    }
}

/// C21: hand-ordered interleaving hitting all five exported symbols in one
/// capture.
#[test]
fn phase_b_c21_mixed_pipeline_all_entry_points() {
    let prog = vec![
        Op::Driver(0),
        Op::PrintLine(Some(cstr(b"between"))),
        Op::PrintHex(0x7F),
        Op::Bad,
        Op::PrintLine(None),
        Op::Good,
        Op::Driver(1),
        Op::PrintHex(0x80),
        Op::PrintHexInt(0x1FF),
        Op::PrintLine(Some(cstr(TOO_LARGE))),
        Op::Driver(-1),
        Op::PrintLine(Some(cstr(b""))),
        Op::Bad,
        Op::Driver(0),
    ];
    let mut expect: Vec<u8> = Vec::new();
    expect.extend_from_slice(BAD_OUT);
    expect.extend_from_slice(b"between\n");
    expect.extend_from_slice(b"7f\n");
    expect.extend_from_slice(BAD_OUT);
    // printLine(NULL) contributes nothing
    expect.extend_from_slice(GOOD_OUT);
    expect.extend_from_slice(GOOD_OUT);
    expect.extend_from_slice(b"ffffff80\n");
    expect.extend_from_slice(b"ffffffff\n"); // 0x1FF -> low byte 0xFF -> -1
    expect.extend_from_slice(TOO_LARGE);
    expect.push(b'\n');
    expect.extend_from_slice(GOOD_OUT);
    expect.extend_from_slice(b"\n");
    expect.extend_from_slice(BAD_OUT);
    expect.extend_from_slice(BAD_OUT);

    assert_same_and_eq("C21 mixed pipeline", &expect, &move |a: &Api| {
        run_program(a, &prog)
    });
}

/// C22: randomized 256-step programs, compared as a single byte stream.
#[test]
fn phase_b_c22_mixed_pipeline_randomized() {
    let mut rng = Rng::new(0x22_5EED);
    for round in 0..8 {
        let mut prog: Vec<Op> = Vec::with_capacity(256);
        for _ in 0..256 {
            prog.push(match rng.below(6) {
                0 => Op::Driver(if rng.below(4) == 0 { 0 } else { rng.next_u32() as i32 }),
                1 => Op::Bad,
                2 => Op::Good,
                3 => {
                    if rng.below(8) == 0 {
                        Op::PrintLine(None)
                    } else {
                        let len = rng.below(24) as usize;
                        let bytes: Vec<u8> = (0..len)
                            .map(|_| 1 + (rng.next_u64() % 255) as u8)
                            .collect();
                        Op::PrintLine(Some(cstr(&bytes)))
                    }
                }
                4 => Op::PrintHex(rng.next_u8()),
                _ => Op::PrintHexInt(rng.next_u32() as i32),
            });
        }
        assert_same(&format!("C22 randomized program round {round}"), &move |a: &Api| {
            run_program(a, &prog)
        });
    }
}

// ===========================================================================
// Phase C — error / rejection-path differential tests (gated on ERRORS.md)
// ===========================================================================

/// E1: `printLine(NULL)` — the explicit null check rejects; 0 bytes written.
#[test]
fn phase_c_e1_print_line_null_rejected() {
    assert_same_and_eq("E1 printLine(NULL)", b"", &|a: &Api| unsafe {
        (a.print_line)(std::ptr::null())
    });
    // and it must still be a no-op when surrounded by real output
    assert_same_and_eq("E1 printLine(NULL) in sequence", b"a\nb\n", &|a: &Api| unsafe {
        (a.print_line)(b"a\0".as_ptr() as *const c_char);
        (a.print_line)(std::ptr::null());
        (a.print_line)(b"b\0".as_ptr() as *const c_char);
    });
}

/// E2: empty string passes the null check.
#[test]
fn phase_c_e2_print_line_empty() {
    assert_same_and_eq("E2 printLine(\"\")", b"\n", &|a: &Api| unsafe {
        (a.print_line)(b"\0".as_ptr() as *const c_char)
    });
}

/// E3: format specifiers are data, never interpreted.
#[test]
fn phase_c_e3_print_line_format_string_is_data() {
    let raw: &[u8] = b"%s %n %d %p";
    let buf = cstr(raw);
    let mut expect = raw.to_vec();
    expect.push(b'\n');
    assert_same_and_eq("E3 printLine format specifiers", &expect, &move |a: &Api| unsafe {
        (a.print_line)(buf.as_ptr() as *const c_char)
    });
}

/// E4: non-UTF-8 bytes pass through verbatim (no validation in C, and the Rust
/// must not add any).
#[test]
fn phase_c_e4_print_line_invalid_utf8() {
    let cases: [&[u8]; 5] = [
        &[0x80, 0xFF, 0xFE],
        &[0xC3],                   // truncated 2-byte sequence
        &[0xE2, 0x82],             // truncated 3-byte sequence
        &[0xF0, 0x9F, 0x92],       // truncated 4-byte sequence
        &[0xFF, 0xFE, 0xFD, 0xFC, 0x80, 0x81],
    ];
    for raw in cases {
        let buf = cstr(raw);
        let mut expect = raw.to_vec();
        expect.push(b'\n');
        assert_same_and_eq("E4 printLine non-UTF8", &expect, &move |a: &Api| unsafe {
            (a.print_line)(buf.as_ptr() as *const c_char)
        });
    }
}

/// E5: 64 KiB input — no length cap in C.
#[test]
fn phase_c_e5_print_line_oversized() {
    let bytes: Vec<u8> = (0..65536usize).map(|i| b'!' + (i % 90) as u8).collect();
    let buf = cstr(&bytes);
    let mut expect = bytes.clone();
    expect.push(b'\n');
    assert_same_and_eq("E5 printLine 64KiB", &expect, &move |a: &Api| unsafe {
        (a.print_line)(buf.as_ptr() as *const c_char)
    });
}

/// E6: interior NUL truncates.
#[test]
fn phase_c_e6_print_line_interior_nul_truncates() {
    let buf = b"ab\0cd\0".to_vec();
    assert_same_and_eq("E6 printLine interior NUL", b"ab\n", &move |a: &Api| unsafe {
        (a.print_line)(buf.as_ptr() as *const c_char)
    });
}

/// E7: negative `char` renders as 8 hex digits.
#[test]
fn phase_c_e7_print_hex_negative() {
    let cases: [(i8, &[u8]); 4] = [
        (-1, b"ffffffff\n"),
        (-2, b"fffffffe\n"),
        (-128, b"ffffff80\n"),
        (-127, b"ffffff81\n"),
    ];
    for (v, expect) in cases {
        assert_same_and_eq(
            &format!("E7 printHexCharLine({v})"),
            expect,
            &move |a: &Api| unsafe { (a.print_hex_char_line)(v as c_char) },
        );
    }
}

/// E8: zero hits the `%02x` zero-pad path.
#[test]
fn phase_c_e8_print_hex_zero() {
    assert_same_and_eq("E8 printHexCharLine(0)", b"00\n", &|a: &Api| unsafe {
        (a.print_hex_char_line)(0)
    });
}

/// E9: 0x80..=0xFF are not representable as positive in a signed `char`.
#[test]
fn phase_c_e9_print_hex_past_signed_range() {
    for v in 0x80u16..=0xFF {
        let b = v as u8;
        let expect = format!("ffffff{b:02x}\n").into_bytes();
        assert_same_and_eq(
            &format!("E9 printHexCharLine(0x{b:02x})"),
            &expect,
            &move |a: &Api| unsafe { (a.print_hex_char_line)(b as c_char) },
        );
    }
}

/// E10: only the low byte of an over-wide argument may be considered.
#[test]
fn phase_c_e10_print_hex_ignores_upper_bits() {
    let cases: [(i32, &[u8]); 6] = [
        (0x1FF, b"ffffffff\n"),
        (0x100, b"00\n"),
        (0x0000_FF7F, b"7f\n"),
        (0xDEADBE7Fu32 as i32, b"7f\n"),
        (-256, b"00\n"),
        (i32::MIN, b"00\n"),
    ];
    for (v, expect) in cases {
        assert_same_and_eq(
            &format!("E10 printHexCharLine(int {v:#x})"),
            expect,
            &move |a: &Api| unsafe { (a.print_hex_char_line_int)(v as c_int) },
        );
    }
}

/// E11: `bad()`'s `if(data > 0)` guard is never false — the overflow always
/// happens.
#[test]
fn phase_c_e11_bad_guard_always_true() {
    assert_same_and_eq("E11 bad() overflow path", BAD_OUT, &|a: &Api| unsafe {
        (a.bad)()
    });
}

/// E12: `goodG2B`'s guard is never false — reachable only via `good()`.
/// `good()`'s first line is `goodG2B`'s output.
#[test]
fn phase_c_e12_goodg2b_guard_always_true() {
    let ca = harness::c_api();
    let out = harness::capture(&mut || unsafe { (ca.good)() });
    let first = out.split(|&b| b == b'\n').next().unwrap().to_vec();
    assert_eq!(first, b"04".to_vec(), "goodG2B must print 04 (2*2)");
    assert_same_and_eq("E12 good() (goodG2B first)", GOOD_OUT, &|a: &Api| unsafe {
        (a.good)()
    });
}

/// E13: the library's one real range rejection — `127 < 63` is false, so
/// `goodB2G` takes the `else` arm and does NOT multiply.
#[test]
fn phase_c_e13_goodb2g_range_rejection() {
    let mut expect = b"04\n".to_vec();
    expect.extend_from_slice(TOO_LARGE);
    expect.push(b'\n');
    assert_same_and_eq("E13 goodB2G range rejection", &expect, &|a: &Api| unsafe {
        (a.good)()
    });
    // Explicitly: no multiplication result (`fe`/`fffffffe`) may appear in the
    // second line of good()'s output.
    let ra = harness::rust_api();
    let out = harness::capture(&mut || unsafe { (ra.good)() });
    assert!(
        !out.windows(8).any(|w| w == b"fffffffe"),
        "goodB2G must not perform the multiplication, got {:?}",
        String::from_utf8_lossy(&out)
    );
}

/// E14: the dead store `data = ' '` (32) must have no effect. If it leaked, the
/// `32 < 63` check would pass and `40` (0x40 = 64) would be printed instead of
/// the diagnostic.
#[test]
fn phase_c_e14_goodb2g_dead_store_has_no_effect() {
    let (ca, ra) = (harness::c_api(), harness::rust_api());
    let c = harness::capture(&mut || unsafe { (ca.good)() });
    let r = harness::capture(&mut || unsafe { (ra.good)() });
    assert_eq!(c, r, "good() diverged");
    assert!(
        c.ends_with(&{
            let mut v = TOO_LARGE.to_vec();
            v.push(b'\n');
            v
        }),
        "expected the rejection diagnostic, got {:?}",
        String::from_utf8_lossy(&c)
    );
    assert!(
        !r.contains(&b'4') || !r.windows(3).any(|w| w == b"40\n"),
        "dead store leaked: saw the 32*2 result"
    );
}

/// E15: `driver(0)` selects `bad()`.
#[test]
fn phase_c_e15_driver_zero_selects_bad() {
    assert_same_and_eq("E15 driver(0)", BAD_OUT, &|a: &Api| unsafe { (a.driver)(0) });
}

/// E16: out-of-range / non-boolean selector values across the FFI boundary.
/// C `if (useGood)` accepts any `int`; there is no valid-variant check, so
/// every non-zero value must behave as "good".
#[test]
fn phase_c_e16_driver_out_of_range_enum_values() {
    let cases: [i32; 12] = [
        -1,
        2,
        3,
        127,
        128,
        255,
        256,
        i32::MIN,
        i32::MAX,
        0xFFFF_FF00u32 as i32,
        0x8000_0001u32 as i32,
        -2147483647,
    ];
    for v in cases {
        assert_same_and_eq(
            &format!("E16 driver({v:#x})"),
            GOOD_OUT,
            &move |a: &Api| unsafe { (a.driver)(v as c_int) },
        );
    }
    // exhaustive over the low byte plus a non-zero high byte
    for lo in 0u16..=0xFF {
        let v = 0x0100 | lo as i32;
        assert_same(&format!("E16 driver({v:#x})"), &move |a: &Api| unsafe {
            (a.driver)(v as c_int)
        });
    }
}

/// E17: selectors whose LOW BYTE is zero but which are non-zero overall — a
/// translation that truncated to `u8`/`bool` would wrongly pick `bad()`.
#[test]
fn phase_c_e17_driver_low_byte_zero_still_truthy() {
    let cases: [i32; 7] = [
        0x0000_0100,
        0x0001_0000,
        0x0100_0000,
        i32::MIN, // 0x80000000
        0xFFFF_FF00u32 as i32,
        0x7FFF_FF00,
        0x0000_FF00,
    ];
    for v in cases {
        assert_same_and_eq(
            &format!("E17 driver({v:#x}) must be truthy"),
            GOOD_OUT,
            &move |a: &Api| unsafe { (a.driver)(v as c_int) },
        );
    }
}

// ===========================================================================
// Generic boundary sweeps required by Phase C beyond the table
// ===========================================================================

/// Exhaustive cross-check of the entire reachable input domain of both leaf
/// entry points, plus the full 16-bit low range of `driver`.
#[test]
fn phase_c_generic_exhaustive_leaf_domains() {
    // printHexCharLine: all 256 values, one capture each already covered by C1;
    // here compare them as one long stream.
    let all: Vec<u8> = (0u16..=0xFF).map(|v| v as u8).collect();
    let a2 = all.clone();
    assert_same("generic printHexCharLine full-domain stream", &move |a: &Api| unsafe {
        for &b in &a2 {
            (a.print_hex_char_line)(b as c_char);
        }
    });

    // printLine: every 2-byte combination of a small alphabet incl. high bytes.
    let alphabet: [u8; 8] = [0x01, 0x09, 0x0A, 0x20, 0x41, 0x7F, 0x80, 0xFF];
    for &x in &alphabet {
        for &y in &alphabet {
            let buf = cstr(&[x, y]);
            assert_same(
                &format!("generic printLine([{x:#04x},{y:#04x}])"),
                &move |a: &Api| unsafe { (a.print_line)(buf.as_ptr() as *const c_char) },
            );
        }
    }
}

/// `driver` over a contiguous range around zero — the only boundary it has.
#[test]
fn phase_c_generic_driver_range_around_zero() {
    for v in -64i32..=64 {
        assert_same(&format!("generic driver({v})"), &move |a: &Api| unsafe {
            (a.driver)(v as c_int)
        });
    }
    for v in [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX] {
        assert_same(&format!("generic driver({v})"), &move |a: &Api| unsafe {
            (a.driver)(v as c_int)
        });
    }
}
