//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Each row drives BOTH the C `.so` and the
//! Rust `.so` through `libloading` with many randomized inputs (fixed seed) and
//! asserts the return value, the entire `bin` buffer (including the sentinel
//! tail) and `*hex_end_p` match byte-for-byte.

mod common;

use common::*;

/// Number of randomized inputs per row (kept modest so the whole suite stays
/// well under the time budget while still sweeping many code paths).
const N: usize = 400;

fn buf_call<'a>(
    hex: &'a [u8],
    bin_maxlen: usize,
    ignore: Option<&'a [u8]>,
    want_hex_end: bool,
) -> Call<'a> {
    Call {
        bin: BinArg::Buf(bin_maxlen + 8),
        bin_maxlen,
        hex: HexArg::Bytes(hex),
        hex_len: hex.len(),
        ignore,
        want_hex_end,
    }
}

// ---------------------------------------------------------------------------
// Rows 1..4 — ignore=NULL, hex_end_p=NULL, exact bin_maxlen, digit classes
// ---------------------------------------------------------------------------

fn exact_alphabet_row(label: &str, alphabet: &[u8], seed_salt: u64) {
    let mut rng = Rng::new(SEED ^ seed_salt);
    for i in 0..N {
        let nibbles = 2 * rng.range(1, 32);
        let hex = rand_hex(&mut rng, nibbles, alphabet);
        let call = buf_call(&hex, nibbles / 2, None, false);
        assert_same(&format!("{label}#{i}"), &call);
    }
}

#[test]
fn configs_md_row_01_null_ignore_null_end_exact_lowercase() {
    let __before = comparisons();
    exact_alphabet_row("row01", LOWER, 1);
    assert_did_work("configs_md_row_01_null_ignore_null_end_exact_lowercase", __before, 400);
}

#[test]
fn configs_md_row_02_null_ignore_null_end_exact_uppercase() {
    let __before = comparisons();
    exact_alphabet_row("row02", UPPER, 2);
    assert_did_work("configs_md_row_02_null_ignore_null_end_exact_uppercase", __before, 400);
}

#[test]
fn configs_md_row_03_null_ignore_null_end_exact_decimal() {
    let __before = comparisons();
    exact_alphabet_row("row03", DEC, 3);
    assert_did_work("configs_md_row_03_null_ignore_null_end_exact_decimal", __before, 400);
}

#[test]
fn configs_md_row_04_null_ignore_null_end_exact_mixed_case() {
    let __before = comparisons();
    exact_alphabet_row("row04", MIXED, 4);
    assert_did_work("configs_md_row_04_null_ignore_null_end_exact_mixed_case", __before, 400);
}

// ---------------------------------------------------------------------------
// Row 5 — slack in bin_maxlen (tail must stay untouched)
// ---------------------------------------------------------------------------

#[test]
fn configs_md_row_05_slack_bin_maxlen() {
    let __before = comparisons();
    let mut rng = Rng::new(SEED ^ 5);
    for i in 0..N {
        let nibbles = 2 * rng.range(0, 32);
        let hex = rand_hex(&mut rng, nibbles, MIXED);
        let maxlen = nibbles / 2 + rng.range(1, 16);
        assert_same(&format!("row05#{i}"), &buf_call(&hex, maxlen, None, false));
    }
    assert_did_work("configs_md_row_05_slack_bin_maxlen", __before, 400);
}

// ---------------------------------------------------------------------------
// Row 6 — bin_maxlen == SIZE_MAX
// ---------------------------------------------------------------------------

#[test]
fn configs_md_row_06_bin_maxlen_size_max() {
    let __before = comparisons();
    let mut rng = Rng::new(SEED ^ 6);
    for i in 0..N {
        let nibbles = 2 * rng.range(0, 32);
        let hex = rand_hex(&mut rng, nibbles, MIXED);
        // The real allocation is small; bin_maxlen is a lie the C also believes,
        // but it never writes more than nibbles/2 bytes, which fits.
        let call = Call {
            bin: BinArg::Buf(nibbles / 2 + 8),
            bin_maxlen: usize::MAX,
            hex: HexArg::Bytes(&hex),
            hex_len: hex.len(),
            ignore: None,
            want_hex_end: false,
        };
        assert_same(&format!("row06#{i}"), &call);
    }
    assert_did_work("configs_md_row_06_bin_maxlen_size_max", __before, 400);
}

// ---------------------------------------------------------------------------
// Row 7 — hex_end_p set, fully consumed input
// ---------------------------------------------------------------------------

#[test]
fn configs_md_row_07_hex_end_set_full_consume() {
    let __before = comparisons();
    let mut rng = Rng::new(SEED ^ 7);
    for i in 0..N {
        let nibbles = 2 * rng.range(1, 32);
        let hex = rand_hex(&mut rng, nibbles, MIXED);
        assert_same(
            &format!("row07#{i}"),
            &buf_call(&hex, nibbles / 2, None, true),
        );
    }
    assert_did_work("configs_md_row_07_hex_end_set_full_consume", __before, 400);
}

// ---------------------------------------------------------------------------
// Row 8 — hex_len == 0
// ---------------------------------------------------------------------------

#[test]
fn configs_md_row_08_hex_len_zero() {
    let __before = comparisons();
    let mut rng = Rng::new(SEED ^ 8);
    for i in 0..N {
        let maxlen = rng.range(0, 16);
        let nh = rng.range(0, 8);
        let hex: Vec<u8> = rand_hex(&mut rng, nh, MIXED);
        let call = Call {
            bin: BinArg::Buf(maxlen + 8),
            bin_maxlen: maxlen,
            hex: HexArg::Bytes(&hex),
            hex_len: 0,
            ignore: if rng.bool() { None } else { Some(b": ") },
            want_hex_end: rng.bool(),
        };
        assert_same(&format!("row08#{i}"), &call);
    }
    assert_did_work("configs_md_row_08_hex_len_zero", __before, 400);
}

// ---------------------------------------------------------------------------
// Row 9 — long inputs
// ---------------------------------------------------------------------------

#[test]
fn configs_md_row_09_long_inputs() {
    let __before = comparisons();
    let mut rng = Rng::new(SEED ^ 9);
    for i in 0..64 {
        let nibbles = 2 * rng.range(256, 1024);
        let hex = rand_hex(&mut rng, nibbles, MIXED);
        assert_same(
            &format!("row09#{i}"),
            &buf_call(&hex, nibbles / 2, None, true),
        );
    }
    assert_did_work("configs_md_row_09_long_inputs", __before, 64);
}

// ---------------------------------------------------------------------------
// Row 10 — bin_maxlen straddling the buffer-full boundary
// ---------------------------------------------------------------------------

#[test]
fn configs_md_row_10_bin_maxlen_straddles_boundary() {
    let __before = comparisons();
    let mut rng = Rng::new(SEED ^ 10);
    for i in 0..N {
        let nibbles = 2 * rng.range(1, 24);
        let hex = rand_hex(&mut rng, nibbles, MIXED);
        let maxlen = rng.range(0, nibbles);
        assert_same(&format!("row10#{i}"), &buf_call(&hex, maxlen, None, true));
    }
    assert_did_work("configs_md_row_10_bin_maxlen_straddles_boundary", __before, 400);
}

// ---------------------------------------------------------------------------
// Row 11 — odd hex_len
// ---------------------------------------------------------------------------

#[test]
fn configs_md_row_11_odd_hex_len() {
    let __before = comparisons();
    let mut rng = Rng::new(SEED ^ 11);
    for i in 0..N {
        let nibbles = 2 * rng.range(0, 24) + 1;
        let hex = rand_hex(&mut rng, nibbles, MIXED);
        let maxlen = rng.range(0, nibbles / 2 + 2);
        assert_same_both_end_modes(&format!("row11#{i}"), &buf_call(&hex, maxlen, None, true));
    }
    assert_did_work("configs_md_row_11_odd_hex_len", __before, 800);
}

// ---------------------------------------------------------------------------
// Row 12 — ignore = "" (differs from NULL only for the 0x00 byte)
// ---------------------------------------------------------------------------

#[test]
fn configs_md_row_12_empty_ignore() {
    let __before = comparisons();
    let mut rng = Rng::new(SEED ^ 12);
    for i in 0..N {
        let nibbles = 2 * rng.range(1, 32);
        let mut hex = rand_hex(&mut rng, nibbles, MIXED);
        // occasionally splice in a NUL to hit the strchr-terminator quirk
        if rng.bool() && !hex.is_empty() {
            let at = rng.below(hex.len());
            hex[at] = 0;
        }
        assert_same_both_end_modes(
            &format!("row12#{i}"),
            &buf_call(&hex, nibbles / 2, Some(b""), true),
        );
    }
    assert_did_work("configs_md_row_12_empty_ignore", __before, 800);
}

// ---------------------------------------------------------------------------
// Rows 13..19 — separator placement
// ---------------------------------------------------------------------------

/// Build a hex string with separators from `seps` inserted at byte boundaries.
fn hex_with_byte_boundary_seps(
    rng: &mut Rng,
    bytes: usize,
    seps: &[u8],
    leading: bool,
    trailing: bool,
) -> Vec<u8> {
    let mut out = Vec::new();
    if leading {
        for _ in 0..rng.range(1, 4) {
            out.push(*rng.pick(seps));
        }
    }
    for b in 0..bytes {
        if b > 0 {
            for _ in 0..rng.range(0, 3) {
                out.push(*rng.pick(seps));
            }
        }
        out.push(*rng.pick(MIXED));
        out.push(*rng.pick(MIXED));
    }
    if trailing {
        for _ in 0..rng.range(1, 4) {
            out.push(*rng.pick(seps));
        }
    }
    out
}

#[test]
fn configs_md_row_13_single_char_separator_between_bytes() {
    let __before = comparisons();
    let mut rng = Rng::new(SEED ^ 13);
    for i in 0..N {
        let bytes = rng.range(1, 24);
        let hex = hex_with_byte_boundary_seps(&mut rng, bytes, b":", false, false);
        assert_same(
            &format!("row13#{i}"),
            &buf_call(&hex, bytes, Some(b":"), true),
        );
    }
    assert_did_work("configs_md_row_13_single_char_separator_between_bytes", __before, 400);
}

#[test]
fn configs_md_row_14_multi_char_separator_runs() {
    let __before = comparisons();
    let mut rng = Rng::new(SEED ^ 14);
    for i in 0..N {
        let bytes = rng.range(1, 24);
        let hex = hex_with_byte_boundary_seps(&mut rng, bytes, b": -", false, false);
        assert_same(
            &format!("row14#{i}"),
            &buf_call(&hex, bytes, Some(b": -"), true),
        );
    }
    assert_did_work("configs_md_row_14_multi_char_separator_runs", __before, 400);
}

#[test]
fn configs_md_row_15_leading_separator_run() {
    let __before = comparisons();
    let mut rng = Rng::new(SEED ^ 15);
    for i in 0..N {
        let bytes = rng.range(1, 24);
        let hex = hex_with_byte_boundary_seps(&mut rng, bytes, b": -", true, false);
        assert_same_both_end_modes(
            &format!("row15#{i}"),
            &buf_call(&hex, bytes, Some(b": -"), true),
        );
    }
    assert_did_work("configs_md_row_15_leading_separator_run", __before, 800);
}

#[test]
fn configs_md_row_16_trailing_separator_run() {
    let __before = comparisons();
    let mut rng = Rng::new(SEED ^ 16);
    for i in 0..N {
        let bytes = rng.range(1, 24);
        let hex = hex_with_byte_boundary_seps(&mut rng, bytes, b": -", false, true);
        assert_same_both_end_modes(
            &format!("row16#{i}"),
            &buf_call(&hex, bytes, Some(b": -"), true),
        );
    }
    assert_did_work("configs_md_row_16_trailing_separator_run", __before, 800);
}

#[test]
fn configs_md_row_17_mid_byte_separator() {
    let __before = comparisons();
    let mut rng = Rng::new(SEED ^ 17);
    for i in 0..N {
        let bytes = rng.range(1, 24);
        let mut hex = hex_with_byte_boundary_seps(&mut rng, bytes, b": -", false, false);
        // Force a separator into an odd position (mid-byte) so the
        // `state == 0U` conjunct on line 23 fails.
        if hex.len() >= 3 {
            let mut at = rng.range(1, hex.len() - 1);
            // find an index whose parity, counting only hex digits before it, is odd
            let mut digits = hex[..at].iter().filter(|b| is_hex_digit(**b)).count();
            while digits % 2 == 0 && at + 1 < hex.len() {
                at += 1;
                if is_hex_digit(hex[at - 1]) {
                    digits += 1;
                }
            }
            hex[at] = *rng.pick(b": -");
        }
        assert_same_both_end_modes(
            &format!("row17#{i}"),
            &buf_call(&hex, bytes, Some(b": -"), true),
        );
    }
    assert_did_work("configs_md_row_17_mid_byte_separator", __before, 800);
}

#[test]
fn configs_md_row_18_separators_with_null_hex_end() {
    let __before = comparisons();
    let mut rng = Rng::new(SEED ^ 18);
    for i in 0..N {
        let bytes = rng.range(1, 24);
        let hex = hex_with_byte_boundary_seps(&mut rng, bytes, b": -", false, false);
        assert_same(
            &format!("row18#{i}"),
            &buf_call(&hex, bytes, Some(b": -"), false),
        );
    }
    assert_did_work("configs_md_row_18_separators_with_null_hex_end", __before, 400);
}

#[test]
fn configs_md_row_19_input_is_only_separators() {
    let __before = comparisons();
    let mut rng = Rng::new(SEED ^ 19);
    for i in 0..N {
        let n = rng.range(0, 32);
        let hex: Vec<u8> = (0..n).map(|_| *rng.pick(b": -\t")).collect();
        assert_same_both_end_modes(
            &format!("row19#{i}"),
            &buf_call(&hex, rng.range(0, 8), Some(b": -\t"), true),
        );
    }
    assert_did_work("configs_md_row_19_input_is_only_separators", __before, 800);
}

// ---------------------------------------------------------------------------
// Row 20 — ignore contains valid hex digits (inert)
// ---------------------------------------------------------------------------

#[test]
fn configs_md_row_20_ignore_contains_hex_digits() {
    let __before = comparisons();
    let mut rng = Rng::new(SEED ^ 20);
    for i in 0..N {
        let nibbles = 2 * rng.range(1, 32);
        let hex = rand_hex(&mut rng, nibbles, MIXED);
        assert_same_both_end_modes(
            &format!("row20#{i}"),
            &buf_call(&hex, nibbles / 2, Some(b"abc0"), true),
        );
    }
    assert_did_work("configs_md_row_20_ignore_contains_hex_digits", __before, 800);
}

// ---------------------------------------------------------------------------
// Row 21 — high bytes (>= 0x80) in both `ignore` and `hex`
// ---------------------------------------------------------------------------

#[test]
fn configs_md_row_21_high_bytes_in_ignore_and_hex() {
    let __before = comparisons();
    let mut rng = Rng::new(SEED ^ 21);
    let ignore: &[u8] = &[0x80, 0xff, 0xa5];
    for i in 0..N {
        let bytes = rng.range(1, 20);
        let (lead, trail) = (rng.bool(), rng.bool());
        let hex = hex_with_byte_boundary_seps(&mut rng, bytes, ignore, lead, trail);
        assert_same_both_end_modes(
            &format!("row21#{i}"),
            &buf_call(&hex, bytes, Some(ignore), true),
        );
    }
    assert_did_work("configs_md_row_21_high_bytes_in_ignore_and_hex", __before, 800);
}

// ---------------------------------------------------------------------------
// Row 22 — ignore = all 255 non-NUL bytes, fully random hex
// ---------------------------------------------------------------------------

#[test]
fn configs_md_row_22_ignore_all_bytes_random_hex() {
    let __before = comparisons();
    let ignore: Vec<u8> = (1u8..=255).collect();
    let mut rng = Rng::new(SEED ^ 22);
    for i in 0..N {
        let n = rng.range(0, 40);
        let hex: Vec<u8> = (0..n).map(|_| rng.next_u8()).collect();
        let maxlen = rng.range(0, 24);
        assert_same_both_end_modes(
            &format!("row22#{i}"),
            &buf_call(&hex, maxlen, Some(&ignore), true),
        );
    }
    assert_did_work("configs_md_row_22_ignore_all_bytes_random_hex", __before, 800);
}

// ---------------------------------------------------------------------------
// Row 23 — ignore=NULL, fully random bytes
// ---------------------------------------------------------------------------

#[test]
fn configs_md_row_23_null_ignore_random_bytes() {
    let __before = comparisons();
    let mut rng = Rng::new(SEED ^ 23);
    for i in 0..2000 {
        let n = rng.range(0, 48);
        let hex: Vec<u8> = (0..n).map(|_| rng.next_u8()).collect();
        let maxlen = rng.range(0, 32);
        let hex_len = if n == 0 { 0 } else { rng.range(0, n) };
        let call = Call {
            bin: BinArg::Buf(maxlen + 8),
            bin_maxlen: maxlen,
            hex: HexArg::Bytes(&hex),
            hex_len,
            ignore: None,
            want_hex_end: true,
        };
        assert_same_both_end_modes(&format!("row23#{i}"), &call);
    }
    assert_did_work("configs_md_row_23_null_ignore_random_bytes", __before, 4000);
}

// ---------------------------------------------------------------------------
// Row 24 — random ignore set, mixed alphabet
// ---------------------------------------------------------------------------

#[test]
fn configs_md_row_24_random_ignore_mixed_alphabet() {
    let __before = comparisons();
    let mut rng = Rng::new(SEED ^ 24);
    for i in 0..2000 {
        // random ignore set of 1..8 non-NUL bytes
        let ilen = rng.range(1, 8);
        let ignore: Vec<u8> = (0..ilen)
            .map(|_| {
                let b = rng.next_u8();
                if b == 0 { 1 } else { b }
            })
            .collect();
        // alphabet = hex digits ∪ the ignore set ∪ junk
        let mut alphabet: Vec<u8> = MIXED.to_vec();
        alphabet.extend_from_slice(&ignore);
        alphabet.extend_from_slice(ADJACENT);
        alphabet.push(0);
        let n = rng.range(0, 40);
        let hex: Vec<u8> = (0..n).map(|_| *rng.pick(&alphabet)).collect();
        let maxlen = rng.range(0, 24);
        assert_same_both_end_modes(
            &format!("row24#{i}"),
            &buf_call(&hex, maxlen, Some(&ignore), true),
        );
    }
    assert_did_work("configs_md_row_24_random_ignore_mixed_alphabet", __before, 4000);
}

// ---------------------------------------------------------------------------
// Row 25 — bin == NULL with bin_maxlen == 0
// ---------------------------------------------------------------------------

#[test]
fn configs_md_row_25_null_bin_zero_maxlen() {
    let __before = comparisons();
    let mut rng = Rng::new(SEED ^ 25);
    for i in 0..N {
        let n = rng.range(0, 16);
        let hex = rand_hex(&mut rng, n, MIXED);
        for ignore in [None, Some(&b": "[..])] {
            for want_hex_end in [false, true] {
                let call = Call {
                    bin: BinArg::Null,
                    bin_maxlen: 0,
                    hex: HexArg::Bytes(&hex),
                    hex_len: hex.len(),
                    ignore,
                    want_hex_end,
                };
                assert_same(&format!("row25#{i}"), &call);
            }
        }
    }
    assert_did_work("configs_md_row_25_null_bin_zero_maxlen", __before, 1600);
}

// ---------------------------------------------------------------------------
// Row 26 — hex == NULL with hex_len == 0
// ---------------------------------------------------------------------------

#[test]
fn configs_md_row_26_null_hex_zero_len() {
    let __before = comparisons();
    for ignore in [None, Some(&b""[..]), Some(&b": "[..])] {
        for want_hex_end in [false, true] {
            for bin in [BinArg::Null, BinArg::Buf(8)] {
                let call = Call {
                    bin,
                    bin_maxlen: if bin == BinArg::Null { 0 } else { 8 },
                    hex: HexArg::Null,
                    hex_len: 0,
                    ignore,
                    want_hex_end,
                };
                assert_same("row26", &call);
            }
        }
    }
    assert_did_work("configs_md_row_26_null_hex_zero_len", __before, 12);
}

// ---------------------------------------------------------------------------
// Row 27 — exhaustive single-byte sweep over the full 0x00..=0xFF domain
// ---------------------------------------------------------------------------

#[test]
fn configs_md_row_27_exhaustive_single_byte() {
    let __before = comparisons();
    for b in 0u8..=255 {
        let hex = [b];
        let self_set = if b == 0 { vec![b'x'] } else { vec![b] };
        let ignore_variants: [Option<&[u8]>; 4] =
            [None, Some(b""), Some(&self_set), Some(b"zZ~\x01")];
        for ignore in ignore_variants {
            for want_hex_end in [false, true] {
                for bin_maxlen in [0usize, 1] {
                    let call = Call {
                        bin: BinArg::Buf(bin_maxlen + 8),
                        bin_maxlen,
                        hex: HexArg::Bytes(&hex),
                        hex_len: 1,
                        ignore,
                        want_hex_end,
                    };
                    assert_same(&format!("row27/byte=0x{b:02x}"), &call);
                }
            }
        }
    }
    assert_did_work("configs_md_row_27_exhaustive_single_byte", __before, 4096);
}

// ---------------------------------------------------------------------------
// Row 28 — exhaustive two-byte sweep (65 536 pairs)
// ---------------------------------------------------------------------------

#[test]
fn configs_md_row_28_exhaustive_two_bytes() {
    let __before = comparisons();
    for a in 0u8..=255 {
        for b in 0u8..=255 {
            let hex = [a, b];
            let call = Call {
                bin: BinArg::Buf(9),
                bin_maxlen: 1,
                hex: HexArg::Bytes(&hex),
                hex_len: 2,
                ignore: None,
                want_hex_end: true,
            };
            assert_same(&format!("row28/{a:02x}{b:02x}"), &call);
        }
    }
    assert_did_work("configs_md_row_28_exhaustive_two_bytes", __before, 65536);
}

/// Same exhaustive pair sweep, but with a non-NULL `ignore` so the
/// `strchr`-terminator quirk and the `state == 0U` conjunct are both crossed
/// with every byte pair.
#[test]
fn configs_md_row_28b_exhaustive_two_bytes_with_ignore() {
    let __before = comparisons();
    for a in 0u8..=255 {
        for b in 0u8..=255 {
            let hex = [a, b];
            let call = Call {
                bin: BinArg::Buf(9),
                bin_maxlen: 1,
                hex: HexArg::Bytes(&hex),
                hex_len: 2,
                ignore: Some(b": -"),
                want_hex_end: true,
            };
            assert_same(&format!("row28b/{a:02x}{b:02x}"), &call);
        }
    }
    assert_did_work("configs_md_row_28b_exhaustive_two_bytes_with_ignore", __before, 65536);
}

// ---------------------------------------------------------------------------
// Row 29 — hex_len shorter than the backing buffer
// ---------------------------------------------------------------------------

#[test]
fn configs_md_row_29_hex_len_shorter_than_buffer() {
    let __before = comparisons();
    let mut rng = Rng::new(SEED ^ 29);
    for i in 0..N {
        let used = 2 * rng.range(1, 16);
        let mut hex = rand_hex(&mut rng, used, MIXED);
        // Guard bytes past hex_len: sometimes hex digits, sometimes junk.
        let guard_hex = rng.bool();
        for _ in 0..rng.range(1, 8) {
            hex.push(if guard_hex {
                *rng.pick(MIXED)
            } else {
                *rng.pick(ADJACENT)
            });
        }
        let call = Call {
            bin: BinArg::Buf(used / 2 + 8),
            bin_maxlen: used / 2,
            hex: HexArg::Bytes(&hex),
            hex_len: used,
            ignore: if rng.bool() { None } else { Some(b": ") },
            want_hex_end: rng.bool(),
        };
        assert_same(&format!("row29#{i}"), &call);
    }
    assert_did_work("configs_md_row_29_hex_len_shorter_than_buffer", __before, 400);
}

// ---------------------------------------------------------------------------
// Row 30 — tiny bin_maxlen with long input
// ---------------------------------------------------------------------------

#[test]
fn configs_md_row_30_tiny_bin_maxlen_long_input() {
    let __before = comparisons();
    let mut rng = Rng::new(SEED ^ 30);
    for i in 0..N {
        let nibbles = 2 * rng.range(8, 64);
        let hex = rand_hex(&mut rng, nibbles, MIXED);
        let maxlen = rng.range(0, 2);
        assert_same_both_end_modes(&format!("row30#{i}"), &buf_call(&hex, maxlen, None, true));
    }
    assert_did_work("configs_md_row_30_tiny_bin_maxlen_long_input", __before, 800);
}

// ---------------------------------------------------------------------------
// Whole-surface fuzz: every axis of CONFIGS.md randomized simultaneously.
//
// The per-row tests above pin down individual configurations; this sweep
// explores their unconstrained cross-product, which is where interaction bugs
// live. Fixed seed, so a failure is reproducible.
// ---------------------------------------------------------------------------

#[test]
fn fuzz_all_axes_simultaneously() {
    let __before = comparisons();
    let mut rng = Rng::new(SEED ^ 0xF1122);
    const ITERS: usize = 200_000;

    let all_bytes: Vec<u8> = (1u8..=255).collect();

    for i in 0..ITERS {
        // --- ignore axis -----------------------------------------------------
        let ignore_owned: Option<Vec<u8>> = match rng.below(6) {
            0 => None,                                  // NULL
            1 => Some(Vec::new()),                      // ""
            2 => Some(b": -".to_vec()),                 // typical separators
            3 => Some(MIXED.to_vec()),                  // hex digits (inert)
            4 => Some(all_bytes.clone()),               // everything
            _ => {
                let n = rng.range(1, 6);
                Some((0..n).map(|_| *rng.pick(&all_bytes)).collect())
            }
        };

        // --- input-shape axis -----------------------------------------------
        let mut alphabet: Vec<u8> = MIXED.to_vec();
        alphabet.extend_from_slice(ADJACENT);
        alphabet.push(0);
        if let Some(ig) = &ignore_owned {
            alphabet.extend_from_slice(ig);
        }
        if rng.bool() {
            alphabet.extend((0x80u8..=0xff).step_by(17));
        }

        let buf_len = rng.range(0, 40);
        let hex: Vec<u8> = if rng.below(8) == 0 {
            // fully random bytes
            (0..buf_len).map(|_| rng.next_u8()).collect()
        } else {
            (0..buf_len).map(|_| *rng.pick(&alphabet)).collect()
        };
        let hex_len = if buf_len == 0 { 0 } else { rng.range(0, buf_len) };

        // --- bin_maxlen axis -------------------------------------------------
        let need = hex_len; // upper bound on bytes the callee can write
        let bin_maxlen = match rng.below(6) {
            0 => 0,
            1 => need / 2,
            2 => need,
            3 => usize::MAX,
            4 => rng.range(0, need + 2),
            _ => rng.range(0, 4),
        };
        // The real allocation must always cover what the callee could write.
        let alloc = need.max(bin_maxlen.min(need)) + 8;

        // --- pointer axes ----------------------------------------------------
        let bin = if bin_maxlen == 0 && rng.below(4) == 0 {
            BinArg::Null
        } else {
            BinArg::Buf(alloc)
        };
        let hexarg = if hex_len == 0 && rng.below(4) == 0 {
            HexArg::Null
        } else {
            HexArg::Bytes(&hex)
        };

        let call = Call {
            bin,
            bin_maxlen,
            hex: hexarg,
            hex_len,
            ignore: ignore_owned.as_deref(),
            want_hex_end: rng.bool(),
        };
        assert_same(&format!("fuzz#{i}"), &call);
    }
    assert_did_work("fuzz_all_axes_simultaneously", __before, ITERS as u64);
}
