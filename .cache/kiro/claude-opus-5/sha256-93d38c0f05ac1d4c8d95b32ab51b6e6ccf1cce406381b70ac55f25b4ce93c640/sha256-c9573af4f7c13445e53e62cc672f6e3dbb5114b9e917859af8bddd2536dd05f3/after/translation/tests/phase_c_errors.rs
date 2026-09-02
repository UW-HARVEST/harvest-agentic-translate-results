//! Phase C — error / rejection-path differential tests.
//!
//! One test per row of `ERRORS.md` (rows 1–2 live in `phase_c_abort.rs`,
//! rows 3–5 in `phase_c_alloc_failure.rs`, rows 34–35 in
//! `phase_b_exhaustive.rs`). Each test asserts the *same specific* sentinel
//! from both shared objects, not merely that both "failed somehow".

mod common;

use common::*;

fn cstr(bytes: &[u8]) -> Vec<u8> {
    let mut v = bytes.to_vec();
    v.push(0);
    v
}

/// Assert both implementations return the SAME offset AND that it equals the
/// offset the C source dictates.
fn assert_drop_offset(p: &Pair, input: &[u8], expected: usize, ctx: &str) {
    let a = cstr(input);
    let b = cstr(input);
    let c = call_drop(p.c.drop_fn, &a);
    let r = call_drop(p.rs.drop_fn, &b);
    assert_eq!(c, r, "[{ctx}] C/Rust offset differ on {input:02X?}: {c} vs {r}");
    assert_eq!(
        c, expected,
        "[{ctx}] offset is {c}, the C source dictates {expected} for {input:02X?}"
    );
}

/// Assert the byte sequence is REJECTED at offset `at` by both.
fn assert_rejected_at(p: &Pair, input: &[u8], at: usize, ctx: &str) {
    assert_drop_offset(p, input, at, ctx);
    // and the filter must agree in every mode
    for m in MODES {
        assert_filter_eq(p, input, m, ctx);
    }
}

/// Assert the whole sequence is ACCEPTED (scanner runs to the terminator).
fn assert_accepted(p: &Pair, input: &[u8], ctx: &str) {
    assert_drop_offset(p, input, input.len(), ctx);
    for m in MODES {
        assert_filter_eq(p, input, m, ctx);
    }
}

// ===========================================================================
// Group 3 — w_utf8_drop rejection sentinel
// ===========================================================================

#[test]
fn err06_first_byte_invalid_returns_ptr_at_zero() {
    let p = pair();
    for b in [0x80u8, 0x9F, 0xA0, 0xBF, 0xC0, 0xC1, 0xF5, 0xF6, 0xF7, 0xF8, 0xFE, 0xFF] {
        assert_rejected_at(&p, &[b], 0, "err06 lead alone");
        assert_rejected_at(&p, &[b, b'A'], 0, "err06 lead + ascii");
    }
}

#[test]
fn err07_invalid_after_k_valid_bytes() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 7);
    for _ in 0..3000 {
        let prefix = valid_mixed_n(&mut rng, 0, 24, &[1, 2, 3, 4]);
        let k = prefix.len();
        let mut s = prefix;
        s.push(definitely_invalid_byte(&mut rng));
        let tail = rng.below(8);
        s.extend(random_bytes(&mut rng, tail));
        assert_drop_offset(&p, &s, k, "err07 invalid after k");
    }
}

#[test]
fn err08_no_invalid_byte_returns_terminator() {
    let p = pair();
    assert_drop_offset(&p, b"", 0, "err08 empty");
    let mut rng = Rng::new(SEED ^ 8);
    for _ in 0..3000 {
        let s = valid_mixed_n(&mut rng, 0, 40, &[1, 2, 3, 4]);
        let n = s.len();
        assert_drop_offset(&p, &s, n, "err08 all valid");
    }
}

// ===========================================================================
// Group 4 — valid_1
// ===========================================================================

#[test]
fn err09_high_bit_set_is_not_a_one_byte_form() {
    let p = pair();
    // Every 0x80..0xFF byte alone must be refused at offset 0 (nothing can make
    // a complete sequence out of a single high byte).
    for b in 0x80u8..=0xFF {
        assert_drop_offset(&p, &[b], 0, "err09 single high byte");
    }
    // Every 0x01..0x7F byte alone must be accepted.
    for b in 0x01u8..=0x7F {
        assert_drop_offset(&p, &[b], 1, "err09 single ascii byte");
    }
}

// ===========================================================================
// Group 5 — valid_2
// ===========================================================================

#[test]
fn err10_lead_outside_c0_df_falls_through() {
    let p = pair();
    // 0xE0..0xFF are not 2-byte leads: with a single continuation byte they must
    // still be refused (they need 3 or 4 bytes, or are illegal).
    for lead in 0xE0u8..=0xFF {
        assert_rejected_at(&p, &[lead, 0x80], 0, "err10 non-2-byte lead");
    }
}

#[test]
fn err11_overlong_two_byte_leads_c0_c1() {
    let p = pair();
    for lead in [0xC0u8, 0xC1] {
        for cont in 0x80u8..=0xBF {
            assert_rejected_at(&p, &[lead, cont], 0, "err11 overlong 2-byte");
        }
        // also mid-string
        assert_rejected_at(&p, &[b'a', b'b', lead, 0x80, b'c'], 2, "err11 mid-string");
    }
}

#[test]
fn err12_second_byte_not_continuation() {
    let p = pair();
    for lead in 0xC2u8..=0xDF {
        // truncated: the terminator doubles as a non-continuation byte
        assert_rejected_at(&p, &[lead], 0, "err12 truncated 2-byte");
        for c in 0x01u8..=0x7F {
            assert_drop_offset(&p, &[lead, c], 0, "err12 ascii after 2-byte lead");
        }
        for c in 0xC0u8..=0xFF {
            assert_drop_offset(&p, &[lead, c], 0, "err12 lead after 2-byte lead");
        }
        // the only accepted continuations
        for c in 0x80u8..=0xBF {
            assert_drop_offset(&p, &[lead, c], 2, "err12 valid 2-byte");
        }
    }
}

// ===========================================================================
// Group 6 — valid_3
// ===========================================================================

#[test]
fn err13_lead_outside_e0_ef_falls_through() {
    let p = pair();
    for lead in 0xF0u8..=0xFF {
        // 3 continuation bytes: only 0xF0..0xF4 can succeed (as a 4-byte form,
        // and only with the right second byte), everything else is refused.
        let s = [lead, 0x90, 0x80, 0x80];
        if (0xF0..=0xF4).contains(&lead) {
            let expect = if lead == 0xF4 { 0 } else { 4 };
            assert_drop_offset(&p, &s, expect, "err13 4-byte lead");
        } else {
            assert_rejected_at(&p, &s, 0, "err13 illegal lead");
        }
    }
}

#[test]
fn err14_three_byte_second_not_continuation() {
    let p = pair();
    for lead in 0xE0u8..=0xEF {
        assert_rejected_at(&p, &[lead], 0, "err14 truncated after lead");
        for c1 in 0x01u8..=0x7F {
            assert_drop_offset(&p, &[lead, c1, 0x80], 0, "err14 ascii 2nd byte");
        }
        for c1 in 0xC0u8..=0xFF {
            assert_drop_offset(&p, &[lead, c1, 0x80], 0, "err14 lead as 2nd byte");
        }
    }
}

#[test]
fn err15_three_byte_third_not_continuation() {
    let p = pair();
    for lead in 0xE0u8..=0xEF {
        let (lo, hi) = match lead {
            0xE0 => (0xA0u8, 0xBFu8),
            0xED => (0x80, 0x9F),
            _ => (0x80, 0xBF),
        };
        for c1 in lo..=hi {
            // truncated after two bytes
            assert_rejected_at(&p, &[lead, c1], 0, "err15 truncated after 2");
            for c2 in [0x01u8, 0x41, 0x7F, 0xC0, 0xE0, 0xFF] {
                assert_drop_offset(&p, &[lead, c1, c2], 0, "err15 bad 3rd byte");
            }
            for c2 in [0x80u8, 0x9F, 0xBF] {
                assert_drop_offset(&p, &[lead, c1, c2], 3, "err15 good 3rd byte");
            }
        }
    }
}

#[test]
fn err16_overlong_three_byte_e0() {
    let p = pair();
    for c1 in 0x80u8..=0x9F {
        for c2 in [0x80u8, 0xA0, 0xBF] {
            assert_rejected_at(&p, &[0xE0, c1, c2], 0, "err16 E0 overlong");
        }
    }
    // one step past: 0xA0 is accepted
    for c1 in 0xA0u8..=0xBF {
        assert_accepted(&p, &[0xE0, c1, 0x80], "err16 E0 accepted");
    }
}

#[test]
fn err17_surrogate_three_byte_ed() {
    let p = pair();
    for c1 in 0xA0u8..=0xBF {
        for c2 in [0x80u8, 0xA0, 0xBF] {
            assert_rejected_at(&p, &[0xED, c1, c2], 0, "err17 ED surrogate");
        }
    }
    for c1 in 0x80u8..=0x9F {
        assert_accepted(&p, &[0xED, c1, 0x80], "err17 ED accepted");
    }
}

/// The `0xEF` clause is dead: `(x[1] & 0xC0) == 0x80` already forces
/// `x[1] <= 0xBF`. Prove it never rejects anything by accepting every
/// `EF <cont> <cont>`.
#[test]
fn err18_ef_clause_is_unreachable() {
    let p = pair();
    for c1 in 0x80u8..=0xBF {
        for c2 in 0x80u8..=0xBF {
            assert_drop_offset(&p, &[0xEF, c1, c2], 3, "err18 EF always accepted");
        }
    }
    for m in CANONICAL_MODES {
        assert_filter_eq(&p, &[0xEF, 0xBF, 0xBD], m, "err18 EF BF BD roundtrip");
        assert_filter_eq(&p, &[0xEF, 0xBF, 0xBF], m, "err18 EF BF BF");
    }
}

// ===========================================================================
// Group 7 — valid_4
// ===========================================================================

#[test]
fn err19_lead_outside_f0_f7() {
    let p = pair();
    for lead in [0xF8u8, 0xF9, 0xFA, 0xFB, 0xFC, 0xFD, 0xFE, 0xFF] {
        assert_rejected_at(&p, &[lead, 0x80, 0x80, 0x80], 0, "err19 F8+ lead");
        assert_rejected_at(&p, &[lead], 0, "err19 F8+ lead alone");
    }
}

#[test]
fn err20_lead_above_f4() {
    let p = pair();
    for lead in [0xF5u8, 0xF6, 0xF7] {
        for c1 in [0x80u8, 0x90, 0xBF] {
            assert_rejected_at(&p, &[lead, c1, 0x80, 0x80], 0, "err20 lead > F4");
        }
    }
    // one step below the max constant: 0xF4 with a legal second byte is accepted
    assert_accepted(&p, &[0xF4, 0x8F, 0x80, 0x80], "err20 F4 accepted");
}

#[test]
fn err21_four_byte_second_not_continuation() {
    let p = pair();
    for lead in 0xF0u8..=0xF4 {
        assert_rejected_at(&p, &[lead], 0, "err21 truncated after lead");
        for c1 in [0x01u8, 0x7F, 0xC0, 0xF0, 0xFF] {
            assert_drop_offset(&p, &[lead, c1, 0x80, 0x80], 0, "err21 bad 2nd byte");
        }
    }
}

#[test]
fn err22_four_byte_third_not_continuation() {
    let p = pair();
    for (lead, c1) in [(0xF0u8, 0x90u8), (0xF1, 0x80), (0xF3, 0xBF), (0xF4, 0x8F)] {
        assert_rejected_at(&p, &[lead, c1], 0, "err22 truncated after 2");
        for c2 in [0x01u8, 0x7F, 0xC2, 0xE0, 0xF0, 0xFF] {
            assert_drop_offset(&p, &[lead, c1, c2, 0x80], 0, "err22 bad 3rd byte");
        }
    }
}

#[test]
fn err23_four_byte_fourth_not_continuation() {
    let p = pair();
    for (lead, c1) in [(0xF0u8, 0x90u8), (0xF1, 0x80), (0xF3, 0xBF), (0xF4, 0x8F)] {
        assert_rejected_at(&p, &[lead, c1, 0x80], 0, "err23 truncated after 3");
        for c3 in [0x01u8, 0x7F, 0xC2, 0xE0, 0xF0, 0xFF] {
            assert_drop_offset(&p, &[lead, c1, 0x80, c3], 0, "err23 bad 4th byte");
        }
        for c3 in [0x80u8, 0xBF] {
            assert_drop_offset(&p, &[lead, c1, 0x80, c3], 4, "err23 good 4th byte");
        }
    }
}

#[test]
fn err24_overlong_four_byte_f0() {
    let p = pair();
    for c1 in 0x80u8..=0x8F {
        assert_rejected_at(&p, &[0xF0, c1, 0x80, 0x80], 0, "err24 F0 overlong");
    }
    for c1 in 0x90u8..=0xBF {
        assert_accepted(&p, &[0xF0, c1, 0x80, 0x80], "err24 F0 accepted");
    }
}

#[test]
fn err25_beyond_max_codepoint_f4() {
    let p = pair();
    for c1 in 0x90u8..=0xBF {
        assert_rejected_at(&p, &[0xF4, c1, 0x80, 0x80], 0, "err25 F4 > 10FFFF");
    }
    for c1 in 0x80u8..=0x8F {
        assert_accepted(&p, &[0xF4, c1, 0x80, 0x80], "err25 F4 accepted");
    }
}

// ===========================================================================
// Group 8 — generic FFI boundaries
// ===========================================================================

#[test]
fn err26_out_of_range_bool_byte() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 26);
    // Every non-zero byte must behave exactly like `true`, and 0 like `false`.
    let cases: Vec<Vec<u8>> = vec![
        b"".to_vec(),
        b"plain ascii".to_vec(),
        vec![0x80],
        vec![b'a', 0xC0, b'b'],
        invalid_run(&mut rng, 1400),
        biased_bytes(&mut rng, 500),
    ];
    for s in &cases {
        let sc = cstr(s);
        let want_true = call_filter(p.c.filter_fn, &sc, 1);
        let want_false = call_filter(p.c.filter_fn, &sc, 0);
        for m in 0u16..=255 {
            let m = m as u8;
            let c = call_filter(p.c.filter_fn, &sc, m);
            let r = call_filter(p.rs.filter_fn, &sc, m);
            assert_eq!(c, r, "err26 C/Rust differ for bool byte {m:#04X}");
            let want = if m == 0 { &want_false } else { &want_true };
            assert_eq!(&c, want, "err26 bool byte {m:#04X} did not act like {}", m != 0);
        }
    }
}

#[test]
fn err27_zero_length() {
    let p = pair();
    assert_drop_offset(&p, b"", 0, "err27 empty drop");
    for m in MODES {
        let out = call_filter(p.c.filter_fn, &cstr(b""), m);
        assert_eq!(out.as_deref(), Some(&[][..]), "err27 C must return \"\"");
        assert_filter_eq(&p, b"", m, "err27 empty filter");
    }
}

#[test]
fn err28_oversized_input_repeated_realloc() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 28);
    for count in [1365usize, 1366, 2731, 5000, 20000] {
        let s = invalid_run(&mut rng, count);
        assert_drop_offset(&p, &s, 0, "err28 drop");
        for m in CANONICAL_MODES {
            assert_filter_eq(&p, &s, m, "err28 filter");
        }
        assert_filter_eq(&p, &s, 0xFF, "err28 filter noncanonical");
    }
}

#[test]
fn err29_c2_min_constant_boundary() {
    let p = pair();
    assert_rejected_at(&p, &[0xC1, 0x80], 0, "err29 C1 rejected");
    assert_accepted(&p, &[0xC2, 0x80], "err29 C2 accepted");
    assert_accepted(&p, &[0xDF, 0xBF], "err29 DF accepted");
    assert_rejected_at(&p, &[0xC0, 0xBF], 0, "err29 C0 rejected");
}

#[test]
fn err30_f4_max_constant_boundary() {
    let p = pair();
    assert_accepted(&p, &[0xF4, 0x8F, 0xBF, 0xBF], "err30 F4 8F accepted");
    assert_rejected_at(&p, &[0xF4, 0x90, 0x80, 0x80], 0, "err30 F4 90 rejected");
    assert_rejected_at(&p, &[0xF5, 0x80, 0x80, 0x80], 0, "err30 F5 rejected");
    assert_accepted(&p, &[0xF3, 0xBF, 0xBF, 0xBF], "err30 F3 accepted");
}

#[test]
fn err31_a0_boundaries() {
    let p = pair();
    assert_rejected_at(&p, &[0xE0, 0x9F, 0xBF], 0, "err31 E0 9F rejected");
    assert_accepted(&p, &[0xE0, 0xA0, 0x80], "err31 E0 A0 accepted");
    assert_accepted(&p, &[0xED, 0x9F, 0xBF], "err31 ED 9F accepted");
    assert_rejected_at(&p, &[0xED, 0xA0, 0x80], 0, "err31 ED A0 rejected");
}

#[test]
fn err32_90_boundary() {
    let p = pair();
    assert_rejected_at(&p, &[0xF0, 0x8F, 0xBF, 0xBF], 0, "err32 F0 8F rejected");
    assert_accepted(&p, &[0xF0, 0x90, 0x80, 0x80], "err32 F0 90 accepted");
}

#[test]
fn err33_bare_continuation_bytes() {
    let p = pair();
    for b in 0x80u8..=0xBF {
        assert_rejected_at(&p, &[b], 0, "err33 bare continuation");
        assert_rejected_at(&p, &[b, b, b, b], 0, "err33 continuation run");
        assert_drop_offset(&p, &[b'x', b], 1, "err33 continuation after ascii");
    }
}
