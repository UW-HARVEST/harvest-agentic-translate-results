//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Every test drives BOTH `.so` files through `libloading` and compares the
//! return value, the entire `bin` buffer (plus a guard region past
//! `bin_maxlen`), and the exact `*hex_end_p` offset.

mod harness;
use harness::*;

/// Number of randomized inputs per row.
const N: usize = 400;

fn ignore_variants() -> Vec<Option<Vec<u8>>> {
    vec![
        None,
        Some(b"".to_vec()),
        Some(b" ".to_vec()),
        Some(b": -".to_vec()),
    ]
}

/// Rows C1..C5: pure-alphabet even-length input, `bin_maxlen` exact,
/// `ignore=NULL`, `hex_end_p=NULL`.
fn pure_alphabet_row(seed: u64, alphabet: &[u8]) {
    let mut rng = Rng::new(seed);
    for _ in 0..N {
        let len = 2 * rng.below(33); // 0..=64, always even
        let hex = random_from(&mut rng, alphabet, len);
        assert_same(&Case::exact(&hex).hex_end(false));
    }
}

#[test]
fn c1_lowercase_exact_no_ignore_no_hexend() {
    pure_alphabet_row(0x0000_0001, LOWER);
}

#[test]
fn c2_uppercase_exact_no_ignore_no_hexend() {
    pure_alphabet_row(0x0000_0002, UPPER);
}

#[test]
fn c3_mixed_case_exact_no_ignore_no_hexend() {
    pure_alphabet_row(0x0000_0003, MIXED);
}

#[test]
fn c4_digits_only_exact() {
    pure_alphabet_row(0x0000_0004, DIGITS);
}

#[test]
fn c5_letters_only_exact() {
    pure_alphabet_row(0x0000_0005, LETTERS);
}

#[test]
fn c6_hex_end_p_non_null_full_consume() {
    let mut rng = Rng::new(0x0000_0006);
    for _ in 0..N {
        let len = 2 * rng.below(33);
        let hex = random_from(&mut rng, MIXED, len);
        assert_same(&Case::exact(&hex).hex_end(true));
    }
}

#[test]
fn c7_generous_bin_maxlen_guard_untouched() {
    let mut rng = Rng::new(0x0000_0007);
    for _ in 0..N {
        let len = 2 * rng.below(25);
        let hex = random_from(&mut rng, MIXED, len);
        let extra = rng.below(9);
        for want_end in [false, true] {
            assert_same(&Case::new(&hex, len / 2 + extra).hex_end(want_end));
        }
    }
}

#[test]
fn c8_bin_maxlen_size_max() {
    let mut rng = Rng::new(0x0000_0008);
    for _ in 0..N {
        let len = 2 * rng.below(17);
        let hex = random_from(&mut rng, MIXED, len);
        for want_end in [false, true] {
            assert_same(&Case::new(&hex, usize::MAX).hex_end(want_end));
        }
    }
}

#[test]
fn c9_empty_input_cross_product() {
    for ig in ignore_variants() {
        for want_end in [false, true] {
            for bml in [0usize, 1, 16, usize::MAX] {
                let mut c = Case::new(b"", bml).hex_end(want_end);
                c.ignore = ig.clone();
                assert_same(&c);
                // Same, but with a NULL hex pointer and zero length.
                let mut c2 = Case::new(b"", bml).hex_end(want_end).hex_null();
                c2.ignore = ig.clone();
                assert_same(&c2);
            }
        }
    }
}

#[test]
fn c10_single_byte_pair_cross_product() {
    let mut rng = Rng::new(0x0000_0010);
    for _ in 0..N {
        let hex = random_from(&mut rng, MIXED, 2);
        for ig in ignore_variants() {
            for want_end in [false, true] {
                for bml in [0usize, 1, 2] {
                    let mut c = Case::new(&hex, bml).hex_end(want_end);
                    c.ignore = ig.clone();
                    assert_same(&c);
                }
            }
        }
    }
}

#[test]
fn c11_long_inputs() {
    let mut rng = Rng::new(0x0000_0011);
    for _ in 0..60 {
        let len = 2 * (1 + rng.below(512)); // up to 1024
        let hex = random_from(&mut rng, MIXED, len);
        for want_end in [false, true] {
            assert_same(&Case::exact(&hex).hex_end(want_end));
        }
    }
}

/// Build a hex string with separators inserted only at even nibble boundaries.
fn with_even_separators(rng: &mut Rng, pairs: usize, seps: &[u8], max_run: usize) -> Vec<u8> {
    let mut out = Vec::new();
    for _ in 0..pairs {
        let run = rng.below(max_run + 1);
        for _ in 0..run {
            out.push(*rng.pick(seps));
        }
        out.push(*rng.pick(MIXED));
        out.push(*rng.pick(MIXED));
    }
    out
}

#[test]
fn c12_ignore_space_even_boundaries() {
    let mut rng = Rng::new(0x0000_0012);
    for _ in 0..N {
        let pairs = rng.below(17);
        let hex = with_even_separators(&mut rng, pairs, b" ", 1);
        assert_same(&Case::new(&hex, pairs).ignore(Some(b" ")).hex_end(true));
        assert_same(&Case::new(&hex, pairs).ignore(Some(b" ")).hex_end(false));
    }
}

#[test]
fn c13_ignore_multichar_set_runs() {
    let mut rng = Rng::new(0x0000_0013);
    for _ in 0..N {
        let pairs = rng.below(13);
        let hex = with_even_separators(&mut rng, pairs, b": -", 4);
        for want_end in [false, true] {
            assert_same(&Case::new(&hex, pairs).ignore(Some(b": -")).hex_end(want_end));
            // Also a short bin_maxlen to cross A1 x A4.
            assert_same(
                &Case::new(&hex, if pairs == 0 { 0 } else { rng.below(pairs) })
                    .ignore(Some(b": -"))
                    .hex_end(want_end),
            );
        }
    }
}

#[test]
fn c14_leading_separators() {
    let mut rng = Rng::new(0x0000_0014);
    for _ in 0..N {
        let lead = rng.below(6);
        let pairs = rng.below(9);
        let mut hex: Vec<u8> = (0..lead).map(|_| *rng.pick(b": -")).collect();
        hex.extend(random_from(&mut rng, MIXED, pairs * 2));
        for want_end in [false, true] {
            assert_same(&Case::new(&hex, pairs).ignore(Some(b": -")).hex_end(want_end));
        }
    }
}

#[test]
fn c15_trailing_separators() {
    let mut rng = Rng::new(0x0000_0015);
    for _ in 0..N {
        let pairs = rng.below(9);
        let trail = 1 + rng.below(5);
        let mut hex = random_from(&mut rng, MIXED, pairs * 2);
        hex.extend((0..trail).map(|_| *rng.pick(b": -")));
        for want_end in [false, true] {
            assert_same(&Case::new(&hex, pairs).ignore(Some(b": -")).hex_end(want_end));
        }
    }
}

#[test]
fn c16_separator_at_odd_boundary() {
    let mut rng = Rng::new(0x0000_0016);
    for _ in 0..N {
        let pairs = 1 + rng.below(8);
        let mut hex = random_from(&mut rng, MIXED, pairs * 2);
        // Splice a separator at an ODD offset so `state != 0` bypasses `ignore`.
        let odd = 1 + 2 * rng.below(pairs);
        hex.insert(odd, *rng.pick(b": -"));
        for want_end in [false, true] {
            assert_same(&Case::new(&hex, pairs).ignore(Some(b": -")).hex_end(want_end));
        }
    }
}

#[test]
fn c17_empty_ignore_set() {
    let mut rng = Rng::new(0x0000_0017);
    for _ in 0..N {
        let len = 2 * rng.below(17);
        let hex = random_from(&mut rng, MIXED, len);
        for want_end in [false, true] {
            assert_same(&Case::exact(&hex).ignore(Some(b"")).hex_end(want_end));
        }
    }
}

#[test]
fn c18_ignore_set_contains_hex_digits() {
    let mut rng = Rng::new(0x0000_0018);
    for _ in 0..N {
        let len = 2 * rng.below(17);
        let hex = random_from(&mut rng, MIXED, len);
        for set in [&b"0123abc"[..], &b"abcdefABCDEF0123456789"[..]] {
            for want_end in [false, true] {
                assert_same(&Case::exact(&hex).ignore(Some(set)).hex_end(want_end));
            }
        }
    }
}

#[test]
fn c19_early_stop_reported_via_hex_end() {
    let mut rng = Rng::new(0x0000_0019);
    for _ in 0..N {
        let prefix = 2 * rng.below(9);
        let mut hex = random_from(&mut rng, MIXED, prefix);
        // A byte guaranteed not to be a hex digit.
        loop {
            let b = rng.byte();
            let is_hex = b.is_ascii_digit() || (b | 32).is_ascii_lowercase() && (b | 32) <= b'f';
            if !is_hex {
                hex.push(b);
                break;
            }
        }
        let tail = rng.below(5);
        hex.extend(random_from(&mut rng, MIXED, tail));
        for ig in ignore_variants() {
            for want_end in [false, true] {
                let mut c = Case::new(&hex, hex.len()).hex_end(want_end);
                c.ignore = ig.clone();
                assert_same(&c);
            }
        }
    }
}

#[test]
fn c20_class_boundary_bytes() {
    let mut rng = Rng::new(0x0000_0020);
    for _ in 0..N {
        let pairs = rng.below(9);
        let mut hex = random_from(&mut rng, MIXED, pairs * 2);
        let pos = rng.below(hex.len() + 1);
        hex.insert(pos, *rng.pick(BOUNDARY));
        for ig in ignore_variants() {
            for want_end in [false, true] {
                let mut c = Case::new(&hex, hex.len()).hex_end(want_end);
                c.ignore = ig.clone();
                assert_same(&c);
            }
        }
    }
}

/// Row C21 — every byte value, alone and after a valid digit, under four
/// `ignore` variants (incl. an ignore set that *is* the byte itself).
#[test]
fn c21_full_byte_sweep() {
    for b in 0u8..=255 {
        let one = vec![b];
        let two = vec![b'a', b];
        let three = vec![b'a', b'b', b];
        for hex in [one, two, three] {
            let sets: [Option<Vec<u8>>; 5] = [
                None,
                Some(b"".to_vec()),
                Some(b" ".to_vec()),
                Some(vec![b]),
                Some(vec![b, b'x']),
            ];
            for ig in sets {
                for want_end in [false, true] {
                    for bml in [0usize, 1, 2, 8] {
                        let mut c = Case::new(&hex, bml).hex_end(want_end);
                        c.ignore = ig.clone();
                        assert_same(&c);
                    }
                }
            }
        }
    }
}

#[test]
fn c22_embedded_nul() {
    let mut rng = Rng::new(0x0000_0022);
    for _ in 0..N {
        let pairs = rng.below(7);
        let mut hex = random_from(&mut rng, MIXED, pairs * 2);
        let pos = rng.below(hex.len() + 1);
        hex.insert(pos, 0u8);
        for ig in ignore_variants() {
            for want_end in [false, true] {
                let mut c = Case::new(&hex, hex.len()).hex_end(want_end);
                c.ignore = ig.clone();
                assert_same(&c);
            }
        }
    }
    // Deterministic spot checks of the `strchr` NUL-terminator quirk.
    for ig in [None, Some(&b""[..]), Some(&b" "[..])] {
        for want_end in [false, true] {
            assert_same(&Case::new(b"aa\0bb", 4).ignore(ig).hex_end(want_end));
            assert_same(&Case::new(b"\0aabb", 4).ignore(ig).hex_end(want_end));
            assert_same(&Case::new(b"a\0abb", 4).ignore(ig).hex_end(want_end));
        }
    }
}

#[test]
fn c23_high_bytes_spliced() {
    let mut rng = Rng::new(0x0000_0023);
    for _ in 0..N {
        let pairs = rng.below(9);
        let mut hex = random_from(&mut rng, MIXED, pairs * 2);
        let b = 0x80 | rng.byte() & 0x7f;
        let pos = rng.below(hex.len() + 1);
        hex.insert(pos, b);
        for ig in [None, Some(&b" "[..]), Some(&[b][..])] {
            for want_end in [false, true] {
                assert_same(&Case::new(&hex, hex.len()).ignore(ig).hex_end(want_end));
            }
        }
    }
}

#[test]
fn c24_short_bin_maxlen_partial_writes() {
    let mut rng = Rng::new(0x0000_0024);
    for _ in 0..N {
        let pairs = 1 + rng.below(12);
        let hex = random_from(&mut rng, MIXED, pairs * 2);
        let bml = rng.below(pairs); // strictly short: 0..pairs-1
        for want_end in [false, true] {
            assert_same(&Case::new(&hex, bml).hex_end(want_end));
            assert_same(&Case::new(&hex, bml).ignore(Some(b" ")).hex_end(want_end));
        }
    }
}

#[test]
fn c25_odd_length_valid_hex() {
    let mut rng = Rng::new(0x0000_0025);
    for _ in 0..N {
        let len = 2 * rng.below(17) + 1; // always odd
        let hex = random_from(&mut rng, MIXED, len);
        for ig in ignore_variants() {
            for want_end in [false, true] {
                for bml in [len / 2, len / 2 + 1, 0] {
                    let mut c = Case::new(&hex, bml).hex_end(want_end);
                    c.ignore = ig.clone();
                    assert_same(&c);
                }
            }
        }
    }
}

/// Row C26 — unstructured cross-product fuzz over every axis at once.
#[test]
fn c26_unstructured_fuzz() {
    let mut rng = Rng::new(0x0000_0026);
    for _ in 0..20_000 {
        let hex_len = rng.below(65);
        // Mix truly random bytes with hex-ish bytes so we hit deep paths often.
        let hex: Vec<u8> = (0..hex_len)
            .map(|_| {
                if rng.below(4) == 0 {
                    rng.byte()
                } else {
                    *rng.pick(b"0123456789abcdefABCDEF :-\0/@Gg`\x80\xff")
                }
            })
            .collect();
        let bin_maxlen = rng.below(41);
        let ignore = match rng.below(5) {
            0 => None,
            1 => Some(Vec::new()),
            2 => Some(b" ".to_vec()),
            3 => Some(b": -\n\t".to_vec()),
            _ => {
                let n = rng.below(5);
                Some((0..n).map(|_| rng.byte()).filter(|b| *b != 0).collect())
            }
        };
        let mut c = Case::new(&hex, bin_maxlen).hex_end(rng.bool());
        c.ignore = ignore;
        // Occasionally shorten hex_len below the buffer length (still in bounds).
        if rng.below(4) == 0 && hex_len > 0 {
            c = c.hex_len(rng.below(hex_len + 1));
        }
        assert_same(&c);
    }
}

/// Row C27 — in-place decoding: `bin` and `hex` are the SAME buffer. The C
/// interleaves `bin[bin_pos] = ...` writes with `hex[hex_pos]` reads (always
/// `bin_pos < hex_pos`), so the exact read/write ordering is observable.
#[test]
fn c27_in_place_decoding_bin_aliases_hex() {
    let mut rng = Rng::new(0x0000_0027);
    for _ in 0..N {
        let len = 2 * rng.below(25);
        let hex = random_from(&mut rng, MIXED, len);
        for want_end in [false, true] {
            assert_same(&Case::exact(&hex).in_place().hex_end(want_end));
            // Short output buffer -> overflow while aliasing.
            assert_same(&Case::new(&hex, len / 4).in_place().hex_end(want_end));
            // Generous output buffer.
            assert_same(&Case::new(&hex, len).in_place().hex_end(want_end));
        }
    }
    // Aliased + separators, odd lengths, and boundary bytes.
    let mut rng = Rng::new(0x1000_0027);
    for _ in 0..N {
        let pairs = rng.below(13);
        let mut hex = with_even_separators(&mut rng, pairs, b": -", 2);
        if rng.bool() && !hex.is_empty() {
            let pos = rng.below(hex.len());
            hex.insert(pos, *rng.pick(BOUNDARY));
        }
        if rng.bool() {
            hex.push(*rng.pick(MIXED)); // make the digit count odd
        }
        for ig in [None, Some(&b": -"[..]), Some(&b""[..])] {
            for want_end in [false, true] {
                assert_same(
                    &Case::new(&hex, hex.len()).in_place().ignore(ig).hex_end(want_end),
                );
            }
        }
    }
}

/// Row C28 — `ignore` aliasing the `hex` buffer (a caller may legitimately point
/// `ignore` at a substring of its own input). Only matters via `strchr`.
#[test]
fn c28_ignore_aliases_nothing_but_shares_content() {
    let mut rng = Rng::new(0x0000_0028);
    for _ in 0..N {
        let pairs = rng.below(9);
        let mut hex = with_even_separators(&mut rng, pairs, b": -", 2);
        if rng.bool() {
            hex.push(0);
        }
        // ignore set built from the very bytes present in hex
        let mut set: Vec<u8> = hex.iter().copied().filter(|b| *b != 0).collect();
        set.truncate(6);
        for want_end in [false, true] {
            assert_same(&Case::new(&hex, hex.len()).ignore(Some(&set)).hex_end(want_end));
            assert_same(
                &Case::new(&hex, hex.len()).in_place().ignore(Some(&set)).hex_end(want_end),
            );
        }
    }
}
