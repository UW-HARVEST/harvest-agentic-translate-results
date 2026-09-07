//! Phase C — error/rejection-path differential tests, one test per `ERRORS.md`
//! row. Both libraries are loaded from their `.so` files with `libloading`.

mod common;

use common::*;

use std::os::unix::process::ExitStatusExt;
use std::process::{Command, Stdio};

const SEED: u64 = 0xE770_0000_1234_5678;

/// Offsets returned by `w_utf8_drop` must agree, and both `w_utf8_filter`
/// results must agree, for both flag values.
fn assert_rejection(pair: &Pair, payload: &[u8], expected_offset: usize, ctx: &str) {
    let buf = cstring(payload);
    let c = call_drop(&pair.c, &buf);
    let r = call_drop(&pair.rs, &buf);
    assert_eq!(
        c, expected_offset,
        "{ctx}: C rejected at {c}, table says {expected_offset}; input {:02X?}",
        payload
    );
    assert_eq!(
        r, c,
        "{ctx}: Rust rejected at {r} but C at {c}; input {:02X?}",
        payload
    );
    for flag in [0u8, 1] {
        let oc = call_filter(&pair.c, &buf, flag);
        let or = call_filter(&pair.rs, &buf, flag);
        assert_eq!(
            oc, or,
            "{ctx}: filter mismatch flag={flag}; input {:02X?}",
            payload
        );
    }
}

// ===========================================================================
// Rows 1 & 2 — NULL pointer. The C's `assert(string != NULL)` is compiled in
// (the CMake build passes no -DNDEBUG; `__assert_fail` is an undefined symbol
// of libdriver.so), so the C aborts with SIGABRT. The Rust must abort with the
// same signal. Each case runs in a fresh child process (this very test binary,
// re-executed) so a crash does not take the suite down.
// ===========================================================================

const ENV_KEY: &str = "NULL_CRASH_TARGET";

fn child_signal(target: &str) -> Option<i32> {
    let exe = std::env::current_exe().expect("current_exe");
    let status = Command::new(exe)
        .arg("--exact")
        .arg("nullcrash::harness")
        .arg("--nocapture")
        .env(ENV_KEY, target)
        .env("RUST_BACKTRACE", "0")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("re-exec test binary");
    status.signal()
}

pub mod nullcrash {
    //! Re-entrant harness: when `NULL_CRASH_TARGET` is set this test calls the
    //! selected `.so` export with a NULL pointer and is expected to die.
    use super::*;

    #[test]
    fn harness() {
        let target = match std::env::var(ENV_KEY) {
            Ok(t) => t,
            Err(_) => return, // normal run: nothing to do
        };
        let pair = load_pair();
        unsafe {
            match target.as_str() {
                "c_drop" => {
                    let _ = (pair.c.drop_fn)(std::ptr::null());
                }
                "rs_drop" => {
                    let _ = (pair.rs.drop_fn)(std::ptr::null());
                }
                "c_filter0" => {
                    let _ = (pair.c.filter_fn)(std::ptr::null(), 0);
                }
                "rs_filter0" => {
                    let _ = (pair.rs.filter_fn)(std::ptr::null(), 0);
                }
                "c_filter1" => {
                    let _ = (pair.c.filter_fn)(std::ptr::null(), 1);
                }
                "rs_filter1" => {
                    let _ = (pair.rs.filter_fn)(std::ptr::null(), 1);
                }
                other => panic!("unknown target {other}"),
            }
        }
        // If we get here neither implementation crashed; exit 0 so the parent
        // sees "no signal" and the differential assert still compares equal.
        std::process::exit(0);
    }
}

#[test]
fn row01_null_pointer_to_w_utf8_drop() {
    let c = child_signal("c_drop");
    let r = child_signal("rs_drop");
    assert_eq!(
        c,
        Some(libc_sigabrt()),
        "expected C w_utf8_drop(NULL) to die with SIGABRT (assert), got {c:?}"
    );
    assert_eq!(
        r, c,
        "w_utf8_drop(NULL): Rust died with signal {r:?}, C with {c:?}"
    );
}

#[test]
fn row02_null_pointer_to_w_utf8_filter() {
    for (cn, rn) in [("c_filter0", "rs_filter0"), ("c_filter1", "rs_filter1")] {
        let c = child_signal(cn);
        let r = child_signal(rn);
        assert_eq!(
            c,
            Some(libc_sigabrt()),
            "expected C {cn}(NULL) to die with SIGABRT (assert), got {c:?}"
        );
        assert_eq!(r, c, "{rn}: Rust signal {r:?} vs C signal {c:?}");
    }
}

fn libc_sigabrt() -> i32 {
    6
}

// ===========================================================================
// Rows 3-18 — every distinct byte pattern the `valid_1..4` ladder rejects.
// Each row is tested at offset 0, and again after a valid prefix so the
// `w_utf8_filter` `memcpy` path is exercised too.
// ===========================================================================

fn with_prefix(pair: &Pair, bad: &[u8], ctx: &str) {
    let mut rng = Rng::new(SEED ^ bad.len() as u64 ^ bad[0] as u64);
    assert_rejection(pair, bad, 0, ctx);
    for _ in 0..25 {
        let prefix = gen_valid_mixed_range(&mut rng, 1, 10);
        let mut v = prefix.clone();
        v.extend_from_slice(bad);
        assert_rejection(pair, &v, prefix.len(), ctx);
        // and with a valid suffix after the bad bytes
        let suffix = gen_valid_mixed_range(&mut rng, 1, 6);
        let mut v2 = v.clone();
        v2.extend_from_slice(&suffix);
        assert_rejection(pair, &v2, prefix.len(), ctx);
    }
}

#[test]
fn row03_lone_continuation_bytes() {
    let pair = load_pair();
    for b in 0x80u8..=0xBF {
        with_prefix(&pair, &[b], "row03 lone continuation");
    }
}

#[test]
fn row04_overlong_two_byte_leads_c0_c1() {
    let pair = load_pair();
    for lead in [0xC0u8, 0xC1] {
        with_prefix(&pair, &[lead], "row04 C0/C1 lead alone");
        for &next in INTERESTING.iter() {
            assert_rejection(&pair, &[lead, next], 0, "row04 C0/C1 + byte");
        }
    }
}

#[test]
fn row05_two_byte_lead_bad_continuation() {
    let pair = load_pair();
    for lead in [0xC2u8, 0xC3, 0xD0, 0xDE, 0xDF] {
        // every non-continuation second byte
        for next in 1u16..=0xFF {
            let n = next as u8;
            if (n & 0xC0) == 0x80 {
                continue; // that would be valid
            }
            assert_rejection(&pair, &[lead, n], 0, "row05 2-byte bad continuation");
        }
        with_prefix(&pair, &[lead, 0x41], "row05 2-byte lead + ASCII");
    }
}

#[test]
fn row06_two_byte_lead_truncated_by_nul() {
    let pair = load_pair();
    for lead in 0xC2u8..=0xDF {
        with_prefix(&pair, &[lead], "row06 truncated 2-byte");
    }
}

#[test]
fn row07_overlong_three_byte_e0() {
    let pair = load_pair();
    for b1 in 0x80u8..=0x9F {
        for b2 in [0x80u8, 0xA0, 0xBF] {
            assert_rejection(&pair, &[0xE0, b1, b2], 0, "row07 E0 overlong");
        }
    }
    with_prefix(&pair, &[0xE0, 0x80, 0x80], "row07 E0 overlong prefixed");
    with_prefix(&pair, &[0xE0, 0x9F, 0xBF], "row07 E0 overlong boundary");
}

#[test]
fn row08_surrogate_halves_ed() {
    let pair = load_pair();
    for b1 in 0xA0u8..=0xBF {
        for b2 in [0x80u8, 0x90, 0xBF] {
            assert_rejection(&pair, &[0xED, b1, b2], 0, "row08 ED surrogate");
        }
    }
    with_prefix(&pair, &[0xED, 0xA0, 0x80], "row08 ED surrogate prefixed");
    with_prefix(&pair, &[0xED, 0xBF, 0xBF], "row08 ED surrogate boundary");
}

#[test]
fn row09_three_byte_bad_second_byte() {
    let pair = load_pair();
    for lead in 0xE0u8..=0xEF {
        for next in 1u16..=0xFF {
            let n = next as u8;
            if (n & 0xC0) == 0x80 {
                continue;
            }
            assert_rejection(&pair, &[lead, n, 0x80], 0, "row09 3-byte bad 2nd");
        }
        // absent second byte (NUL truncation)
        with_prefix(&pair, &[lead], "row09 3-byte truncated at 2nd");
    }
}

#[test]
fn row10_three_byte_bad_third_byte() {
    let pair = load_pair();
    for lead in 0xE0u8..=0xEF {
        let b1 = if lead == 0xE0 {
            0xA0
        } else if lead == 0xED {
            0x9F
        } else {
            0x80
        };
        for next in 1u16..=0xFF {
            let n = next as u8;
            if (n & 0xC0) == 0x80 {
                continue;
            }
            assert_rejection(&pair, &[lead, b1, n], 0, "row10 3-byte bad 3rd");
        }
        // absent third byte (NUL truncation)
        with_prefix(&pair, &[lead, b1], "row10 3-byte truncated at 3rd");
    }
}

#[test]
fn row11_ef_second_byte_guard_is_a_noop() {
    let pair = load_pair();
    // `(x[0] != (char)0xEF || (unsigned char)x[1] <= 0xBF)` can never fire on
    // its own because the preceding `(x[1] & 0xC0) == 0x80` already forces
    // x[1] into 0x80..=0xBF. Prove it: for EVERY second byte, the C's accept /
    // reject decision for lead 0xEF matches the same decision with the guard
    // removed, and the Rust agrees with the C byte-for-byte.
    for b1 in 1u16..=0xFF {
        for b2 in 1u16..=0xFF {
            let payload = [0xEFu8, b1 as u8, b2 as u8];
            let buf = cstring(&payload);
            let c = call_drop(&pair.c, &buf);
            let r = call_drop(&pair.rs, &buf);
            assert_eq!(c, r, "row11 EF guard: drop differs for {:02X?}", payload);
            let guard_relevant = (b1 as u8) > 0xBF && ((b1 as u8) & 0xC0) == 0x80;
            assert!(
                !guard_relevant,
                "row11: the 0xEF guard turned out to be reachable for b1={b1:#04X}"
            );
            let expect_accept = ((b1 as u8) & 0xC0) == 0x80 && ((b2 as u8) & 0xC0) == 0x80;
            assert_eq!(
                c == 3,
                expect_accept,
                "row11 EF guard: unexpected C decision for {:02X?}",
                payload
            );
            for flag in [0u8, 1] {
                let oc = call_filter(&pair.c, &buf, flag);
                let or = call_filter(&pair.rs, &buf, flag);
                assert_eq!(oc, or, "row11 filter differs for {:02X?}", payload);
            }
        }
    }
}

#[test]
fn row12_overlong_four_byte_f0() {
    let pair = load_pair();
    for b1 in 0x80u8..=0x8F {
        assert_rejection(&pair, &[0xF0, b1, 0x80, 0x80], 0, "row12 F0 overlong");
    }
    with_prefix(&pair, &[0xF0, 0x80, 0x80, 0x80], "row12 F0 overlong prefixed");
    with_prefix(&pair, &[0xF0, 0x8F, 0xBF, 0xBF], "row12 F0 overlong boundary");
}

#[test]
fn row13_beyond_max_codepoint_f4() {
    let pair = load_pair();
    for b1 in 0x90u8..=0xBF {
        assert_rejection(&pair, &[0xF4, b1, 0x80, 0x80], 0, "row13 F4 > U+10FFFF");
    }
    with_prefix(&pair, &[0xF4, 0x90, 0x80, 0x80], "row13 F4 too big prefixed");
    with_prefix(&pair, &[0xF4, 0xBF, 0xBF, 0xBF], "row13 F4 too big boundary");
}

#[test]
fn row14_leads_f5_f6_f7() {
    let pair = load_pair();
    for lead in [0xF5u8, 0xF6, 0xF7] {
        with_prefix(&pair, &[lead], "row14 lead > F4 alone");
        assert_rejection(&pair, &[lead, 0x80, 0x80, 0x80], 0, "row14 lead > F4 full");
    }
}

#[test]
fn row15_leads_f8_through_ff() {
    let pair = load_pair();
    for lead in 0xF8u8..=0xFF {
        with_prefix(&pair, &[lead], "row15 invalid lead alone");
        assert_rejection(&pair, &[lead, 0x80, 0x80, 0x80], 0, "row15 invalid lead full");
    }
}

#[test]
fn row16_four_byte_bad_second_byte() {
    let pair = load_pair();
    for lead in 0xF0u8..=0xF4 {
        for next in 1u16..=0xFF {
            let n = next as u8;
            if (n & 0xC0) == 0x80 {
                continue;
            }
            assert_rejection(&pair, &[lead, n, 0x80, 0x80], 0, "row16 4-byte bad 2nd");
        }
        with_prefix(&pair, &[lead], "row16 4-byte truncated at 2nd");
    }
}

#[test]
fn row17_four_byte_bad_third_byte() {
    let pair = load_pair();
    for lead in 0xF0u8..=0xF4 {
        let b1 = if lead == 0xF0 { 0x90 } else { 0x80 };
        for next in 1u16..=0xFF {
            let n = next as u8;
            if (n & 0xC0) == 0x80 {
                continue;
            }
            assert_rejection(&pair, &[lead, b1, n, 0x80], 0, "row17 4-byte bad 3rd");
        }
        with_prefix(&pair, &[lead, b1], "row17 4-byte truncated at 3rd");
    }
}

#[test]
fn row18_four_byte_bad_fourth_byte() {
    let pair = load_pair();
    for lead in 0xF0u8..=0xF4 {
        let b1 = if lead == 0xF0 { 0x90 } else { 0x80 };
        for next in 1u16..=0xFF {
            let n = next as u8;
            if (n & 0xC0) == 0x80 {
                continue;
            }
            assert_rejection(&pair, &[lead, b1, 0x80, n], 0, "row18 4-byte bad 4th");
        }
        with_prefix(&pair, &[lead, b1, 0x80], "row18 4-byte truncated at 4th");
    }
}

// ===========================================================================
// Rows 19 & 20 — what `w_utf8_filter` does with a rejected byte under each
// flag value. Asserted against the C, and additionally against the documented
// intent (dropped vs. EF BF BD) so a "both wrong the same way" pass is ruled
// out for the C side.
// ===========================================================================

#[test]
fn row19_row20_rejected_byte_dropped_or_replaced() {
    let pair = load_pair();
    for bad in ALWAYS_INVALID {
        let payload = [b'A', bad, b'B'];
        let buf = cstring(&payload);

        let c0 = call_filter(&pair.c, &buf, 0).expect("C filter NULL");
        let r0 = call_filter(&pair.rs, &buf, 0).expect("Rust filter NULL");
        assert_eq!(c0, r0, "row19 mismatch for bad={bad:#04X}");
        assert_eq!(c0, b"AB".to_vec(), "row19 C did not drop bad={bad:#04X}");

        let c1 = call_filter(&pair.c, &buf, 1).expect("C filter NULL");
        let r1 = call_filter(&pair.rs, &buf, 1).expect("Rust filter NULL");
        assert_eq!(c1, r1, "row20 mismatch for bad={bad:#04X}");
        assert_eq!(
            c1,
            vec![b'A', 0xEF, 0xBF, 0xBD, b'B'],
            "row20 C did not substitute U+FFFD for bad={bad:#04X}"
        );
    }
}

// ===========================================================================
// Rows 21 & 22 — allocation failure. Not inducible without interposing on
// libc; documented as n/a in ERRORS.md. What IS asserted here is that both
// implementations always return non-NULL for the successful case, i.e. the
// NULL channel is never taken spuriously by one and not the other. Row 35 of
// CONFIGS.md and every `assert_filter_eq` also compare NULL-ness explicitly.
// ===========================================================================

#[test]
fn row21_row22_null_return_channel_never_diverges() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 21);
    for _ in 0..500 {
        let n = rng.range(1, 4096);
        let v: Vec<u8> = (0..n).map(|_| rng.nonzero_byte()).collect();
        let buf = cstring(&v);
        for flag in [0u8, 1] {
            let c = call_filter(&pair.c, &buf, flag);
            let r = call_filter(&pair.rs, &buf, flag);
            assert_eq!(
                c.is_none(),
                r.is_none(),
                "row21/22 NULL-return divergence flag={flag} len={n}"
            );
            assert!(c.is_some(), "row21/22 C unexpectedly returned NULL");
        }
    }
}

// ===========================================================================
// Row 23 — out-of-range `_Bool` across the FFI boundary.
// ===========================================================================

#[test]
fn row23_out_of_range_bool_argument() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 23);
    let payload = [b'A', 0xFF, b'B', 0x80, b'C'];
    let buf = cstring(&payload);
    for flag in 0u16..=0xFF {
        let f = flag as u8;
        let c = call_filter(&pair.c, &buf, f).expect("C filter NULL");
        let r = call_filter(&pair.rs, &buf, f).expect("Rust filter NULL");
        assert_eq!(c, r, "row23 mismatch for _Bool byte value {f:#04X}");
        // Every non-zero byte must behave like `true` (the C tests the whole byte).
        let expected: Vec<u8> = if f == 0 {
            b"ABC".to_vec()
        } else {
            vec![b'A', 0xEF, 0xBF, 0xBD, b'B', 0xEF, 0xBF, 0xBD, b'C']
        };
        assert_eq!(c, expected, "row23 C behaved unexpectedly for {f:#04X}");
    }
    // random payloads with weird flag bytes
    for _ in 0..300 {
        let n = rng.range(1, 48);
        let v: Vec<u8> = (0..n).map(|_| rng.nonzero_byte()).collect();
        let f = rng.nonzero_byte();
        let buf = cstring(&v);
        let c = call_filter(&pair.c, &buf, f);
        let r = call_filter(&pair.rs, &buf, f);
        assert_eq!(c, r, "row23 random mismatch flag={f:#04X} input={:02X?}", v);
    }
}

// ===========================================================================
// Rows 24-27 — zero length, invalid-at-offset-0, realloc boundary, all-valid.
// ===========================================================================

#[test]
fn row24_zero_length_input() {
    let pair = load_pair();
    let buf = cstring(b"");
    assert_eq!(call_drop(&pair.c, &buf), 0);
    assert_eq!(call_drop(&pair.rs, &buf), 0);
    for flag in 0u16..=0xFF {
        let f = flag as u8;
        let c = call_filter(&pair.c, &buf, f);
        let r = call_filter(&pair.rs, &buf, f);
        assert_eq!(c, r, "row24 mismatch flag={f:#04X}");
        assert_eq!(c, Some(Vec::new()), "row24 C did not return an empty string");
    }
}

#[test]
fn row25_first_byte_invalid_memcpy_length_zero() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 25);
    for bad in ALWAYS_INVALID {
        for _ in 0..50 {
            let mut v = vec![bad];
            v.extend_from_slice(&gen_valid_mixed_range(&mut rng, 0, 12));
            let buf = cstring(&v);
            assert_eq!(call_drop(&pair.c, &buf), 0);
            assert_eq!(call_drop(&pair.rs, &buf), 0);
            for flag in [0u8, 1] {
                let c = call_filter(&pair.c, &buf, flag);
                let r = call_filter(&pair.rs, &buf, flag);
                assert_eq!(c, r, "row25 mismatch flag={flag} input={:02X?}", v);
            }
        }
    }
}

#[test]
fn row26_realloc_boundary_1366th_replacement() {
    let pair = load_pair();
    // 4096 % 3 == 1, so after the first realloc repl walks 4093, 4090, ... 1;
    // the 1366th replacement sees repl == 1 < 3 and reallocs again.
    for n in 1360usize..=1375 {
        let v = vec![0xFFu8; n];
        let buf = cstring(&v);
        for flag in [0u8, 1] {
            let c = call_filter(&pair.c, &buf, flag).expect("C filter NULL");
            let r = call_filter(&pair.rs, &buf, flag).expect("Rust filter NULL");
            assert_eq!(c, r, "row26 mismatch n={n} flag={flag}");
            let expect_len = if flag == 0 { 0 } else { n * 3 };
            assert_eq!(c.len(), expect_len, "row26 C length surprise n={n}");
        }
    }
    // deeper: several realloc rounds
    for n in [2731usize, 2732, 4096, 4097, 8192] {
        let v = vec![0x80u8; n];
        let buf = cstring(&v);
        let c = call_filter(&pair.c, &buf, 1).expect("C filter NULL");
        let r = call_filter(&pair.rs, &buf, 1).expect("Rust filter NULL");
        assert_eq!(c, r, "row26 deep mismatch n={n}");
        assert_eq!(c.len(), n * 3);
    }
}

#[test]
fn row27_fully_valid_strdup_fast_path() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 27);
    for _ in 0..400 {
        let v = gen_valid_mixed_range(&mut rng, 1, 40);
        let buf = cstring(&v);
        let dc = call_drop(&pair.c, &buf);
        let dr = call_drop(&pair.rs, &buf);
        assert_eq!(dc, v.len(), "row27 C did not reach the NUL for {:02X?}", v);
        assert_eq!(dr, dc, "row27 drop mismatch");
        for flag in [0u8, 1] {
            let c = call_filter(&pair.c, &buf, flag).expect("C filter NULL");
            let r = call_filter(&pair.rs, &buf, flag).expect("Rust filter NULL");
            assert_eq!(c, r, "row27 filter mismatch flag={flag}");
            assert_eq!(c, v, "row27 C did not return an identical copy");
        }
    }
}

// ===========================================================================
// Generic boundaries the task asks for beyond the table.
// ===========================================================================

#[test]
fn generic_single_byte_strings_every_value() {
    let pair = load_pair();
    for b in 1u16..=0xFF {
        let payload = [b as u8];
        let buf = cstring(&payload);
        let c = call_drop(&pair.c, &buf);
        let r = call_drop(&pair.rs, &buf);
        assert_eq!(c, r, "generic: drop differs for byte {b:#04X}");
        for flag in 0u16..=0xFF {
            let f = flag as u8;
            let oc = call_filter(&pair.c, &buf, f);
            let or = call_filter(&pair.rs, &buf, f);
            assert_eq!(oc, or, "generic: filter differs byte={b:#04X} flag={f:#04X}");
        }
    }
}

#[test]
fn generic_oversized_inputs() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 0xB16_u64);
    for len in [65_536usize, 262_144] {
        // all-invalid (worst case for the replacement growth accounting)
        let v = vec![0xFEu8; len];
        let buf = cstring(&v);
        for flag in [0u8, 1] {
            let c = call_filter(&pair.c, &buf, flag).expect("C filter NULL");
            let r = call_filter(&pair.rs, &buf, flag).expect("Rust filter NULL");
            assert_eq!(c.len(), r.len(), "oversized len mismatch len={len} flag={flag}");
            assert_eq!(c, r, "oversized content mismatch len={len} flag={flag}");
        }
        // random mix
        let mut w = Vec::with_capacity(len);
        while w.len() < len {
            if rng.below(3) == 0 {
                w.push(invalid_byte(&mut rng));
            } else {
                w.extend_from_slice(&gen_valid_mixed(&mut rng, 1));
            }
        }
        let buf = cstring(&w);
        let dc = call_drop(&pair.c, &buf);
        let dr = call_drop(&pair.rs, &buf);
        assert_eq!(dc, dr, "oversized drop mismatch len={len}");
        for flag in [0u8, 1] {
            let c = call_filter(&pair.c, &buf, flag);
            let r = call_filter(&pair.rs, &buf, flag);
            assert_eq!(c, r, "oversized mixed mismatch len={len} flag={flag}");
        }
    }
}

#[test]
fn generic_one_past_every_documented_range_boundary() {
    let pair = load_pair();
    // For each range check in the C, test the last accepted value and the
    // first rejected value on both sides.
    let cases: &[(&[u8], &str)] = &[
        (&[0xC1, 0x80], "one below the C2 lead floor"),
        (&[0xC2, 0x80], "the C2 lead floor itself"),
        (&[0xC2, 0x7F], "one below the continuation floor"),
        (&[0xC2, 0xC0], "one past the continuation ceiling"),
        (&[0xE0, 0x9F, 0x80], "one below the E0 second-byte floor"),
        (&[0xE0, 0xA0, 0x80], "the E0 second-byte floor"),
        (&[0xED, 0x9F, 0x80], "last non-surrogate after ED"),
        (&[0xED, 0xA0, 0x80], "first surrogate after ED"),
        (&[0xEF, 0xBF, 0xBF], "the EF/BF ceiling"),
        (&[0xF0, 0x8F, 0x80, 0x80], "one below the F0 second-byte floor"),
        (&[0xF0, 0x90, 0x80, 0x80], "the F0 second-byte floor"),
        (&[0xF4, 0x8F, 0xBF, 0xBF], "U+10FFFF, the last code point"),
        (&[0xF4, 0x90, 0x80, 0x80], "one past U+10FFFF"),
        (&[0xF4, 0x80, 0x80, 0x80], "F4 floor"),
        (&[0xF5, 0x80, 0x80, 0x80], "one past the F4 lead ceiling"),
        (&[0xEF, 0xBF, 0xBD], "U+FFFD itself, the replacement character"),
    ];
    for (payload, what) in cases {
        let buf = cstring(payload);
        let c = call_drop(&pair.c, &buf);
        let r = call_drop(&pair.rs, &buf);
        assert_eq!(c, r, "boundary '{what}': drop differs for {:02X?}", payload);
        for flag in [0u8, 1] {
            let oc = call_filter(&pair.c, &buf, flag);
            let or = call_filter(&pair.rs, &buf, flag);
            assert_eq!(
                oc, or,
                "boundary '{what}': filter differs flag={flag} for {:02X?}",
                payload
            );
        }
    }
}
