//! Phase B — valid-path differential tests, one test per row of `CONFIGS.md`.
//!
//! Every test drives BOTH `.so`s through their exported `hex2bin` symbol and
//! asserts the return value, the entire output window and the `hex_end_p`
//! pointer are byte-identical. Randomized rows use a fixed seed.

mod common;
use common::*;

const N: usize = 2000; // randomized iterations per row

/// Build `n` hex digits drawn from `charset`.
fn digits(rng: &mut Rng, n: usize, charset: &[u8]) -> Vec<u8> {
    (0..n).map(|_| *rng.pick(charset)).collect()
}

/// Interleave ignorable bytes into a valid hex stream, only at *even* nibble
/// positions (i.e. on byte boundaries), which is where the C allows skipping.
/// `max_run` consecutive ignorables are inserted at each boundary.
fn with_ignorables_even(rng: &mut Rng, nbytes: usize, set: &[u8], max_run: usize) -> Vec<u8> {
    let mut out = Vec::new();
    for _ in 0..nbytes {
        for _ in 0..rng.below(max_run + 1) {
            out.push(*rng.pick(set));
        }
        out.push(*rng.pick(MIXED));
        out.push(*rng.pick(MIXED));
    }
    out
}

// ---------------------------------------------------------------- row 1
#[test]
fn row01_empty_input_no_ignore_no_hex_end() {
    let c = Case::new(b"")
        .no_ignore()
        .hex_end(false)
        .bin_maxlen(0)
        .bin_cap(4);
    let out = assert_same("row01", &c);
    assert_eq!(out.ret, 0);
    // also with a non-empty output buffer available
    assert_same("row01/slack", &Case::new(b"").hex_end(false).bin_maxlen(8).bin_cap(8));
}

// ---------------------------------------------------------------- row 2
#[test]
fn row02_single_byte_lowercase() {
    let mut rng = Rng::new(0x0202);
    for _ in 0..N {
        let h = digits(&mut rng, 2, HEX_LOWER);
        let c = Case::new(&h).no_ignore().hex_end(false);
        let out = assert_same("row02", &c);
        assert_eq!(out.ret, 1);
    }
}

// ---------------------------------------------------------------- row 3
#[test]
fn row03_digits_only_even() {
    let mut rng = Rng::new(0x0303);
    for _ in 0..N {
        let n = 2 * rng.below(33);
        let h = digits(&mut rng, n, DIGITS);
        assert_same("row03", &Case::new(&h).no_ignore().hex_end(false));
    }
}

// ---------------------------------------------------------------- row 4
#[test]
fn row04_alpha_lowercase_only_even() {
    let mut rng = Rng::new(0x0404);
    for _ in 0..N {
        let n = 2 * rng.below(33);
        let h = digits(&mut rng, n, ALPHA_LOWER);
        assert_same("row04", &Case::new(&h).no_ignore().hex_end(false));
    }
}

// ---------------------------------------------------------------- row 5
#[test]
fn row05_alpha_uppercase_only_even() {
    let mut rng = Rng::new(0x0505);
    for _ in 0..N {
        let n = 2 * rng.below(33);
        let h = digits(&mut rng, n, ALPHA_UPPER);
        assert_same("row05", &Case::new(&h).no_ignore().hex_end(false));
    }
}

// ---------------------------------------------------------------- row 6
#[test]
fn row06_mixed_case_even() {
    let mut rng = Rng::new(0x0606);
    for _ in 0..N {
        let n = 2 * rng.below(33);
        let h = digits(&mut rng, n, MIXED);
        assert_same("row06", &Case::new(&h).no_ignore().hex_end(false));
    }
}

// ---------------------------------------------------------------- row 7
#[test]
fn row07_large_inputs() {
    let mut rng = Rng::new(0x0707);
    for _ in 0..200 {
        let n = 2 * (256 + rng.below(257)); // 512..1024 digits
        let h = digits(&mut rng, n, MIXED);
        let out = assert_same("row07", &Case::new(&h).no_ignore().hex_end(false));
        assert_eq!(out.ret as usize, n / 2);
    }
}

// ---------------------------------------------------------------- row 8
#[test]
fn row08_bin_maxlen_larger_than_needed() {
    let mut rng = Rng::new(0x0808);
    for _ in 0..N {
        let n = 2 * rng.below(17);
        let h = digits(&mut rng, n, MIXED);
        let slack = 1 + rng.below(16);
        let c = Case::new(&h)
            .no_ignore()
            .hex_end(false)
            .bin_cap(n / 2 + slack)
            .bin_maxlen(n / 2 + slack);
        assert_same("row08", &c);
    }
}

// ---------------------------------------------------------------- row 9
#[test]
fn row09_bin_maxlen_usize_max() {
    let mut rng = Rng::new(0x0909);
    for _ in 0..N {
        let n = 2 * rng.below(17);
        let h = digits(&mut rng, n, MIXED);
        // Real capacity is ample; only `bin_maxlen` is absurd, which is exactly
        // what a caller with a huge buffer would pass.
        let c = Case::new(&h)
            .no_ignore()
            .hex_end(true)
            .bin_cap(64)
            .bin_maxlen(usize::MAX);
        let out = assert_same("row09", &c);
        assert_eq!(out.ret as usize, n / 2);
    }
}

// ---------------------------------------------------------------- row 10
#[test]
fn row10_hex_end_full_consumption() {
    let mut rng = Rng::new(0x1010);
    for _ in 0..N {
        let n = 2 * rng.below(33);
        let h = digits(&mut rng, n, MIXED);
        let out = assert_same("row10", &Case::new(&h).no_ignore().hex_end(true));
        assert_eq!(out.hex_end, Some(n as isize));
        assert_eq!(out.ret as usize, n / 2);
    }
}

// ---------------------------------------------------------------- row 11
#[test]
fn row11_trailing_garbage_with_hex_end() {
    let mut rng = Rng::new(0x1111);
    for _ in 0..N {
        let n = 2 * rng.below(17);
        let mut h = digits(&mut rng, n, MIXED);
        let stop = h.len();
        // append 1..4 bytes that are definitely not hex digits
        let ngarbage = 1 + rng.below(4);
        for _ in 0..ngarbage {
            loop {
                let b = rng.byte();
                if !is_c_hex(b) {
                    h.push(b);
                    break;
                }
            }
        }
        let c = Case::new(&h)
            .no_ignore()
            .hex_end(true)
            .bin_cap(n / 2 + 4)
            .bin_maxlen(n / 2 + 4);
        let out = assert_same("row11", &c);
        assert_eq!(out.ret as usize, n / 2);
        assert_eq!(out.hex_end, Some(stop as isize));
    }
}

// ---------------------------------------------------------------- row 12
#[test]
fn row12_single_digit_odd_decrements_hex_end() {
    for b in HEX_LOWER.iter().chain(ALPHA_UPPER.iter()) {
        let c = Case::new(&[*b]).no_ignore().hex_end(true).bin_cap(4).bin_maxlen(4);
        let out = assert_same("row12", &c);
        assert_eq!(out.ret, -1);
        // hex_pos was 1, then decremented to 0
        assert_eq!(out.hex_end, Some(0));
    }
}

// ---------------------------------------------------------------- row 13
#[test]
fn row13_space_ignore_even_positions() {
    let mut rng = Rng::new(0x1313);
    for _ in 0..N {
        let nbytes = rng.below(17);
        let h = with_ignorables_even(&mut rng, nbytes, b" ", 2);
        let c = Case::new(&h)
            .ignore(b" ")
            .hex_end(false)
            .bin_cap(nbytes + 1)
            .bin_maxlen(nbytes);
        let out = assert_same("row13", &c);
        assert_eq!(out.ret as usize, nbytes);
    }
}

// ---------------------------------------------------------------- row 14
#[test]
fn row14_multichar_ignore_set_runs() {
    let mut rng = Rng::new(0x1414);
    let set: &[u8] = b" \t\n\r:-";
    for _ in 0..N {
        let nbytes = rng.below(17);
        let h = with_ignorables_even(&mut rng, nbytes, set, 3);
        let c = Case::new(&h)
            .ignore(set)
            .hex_end(false)
            .bin_cap(nbytes + 1)
            .bin_maxlen(nbytes);
        let out = assert_same("row14", &c);
        assert_eq!(out.ret as usize, nbytes);
    }
}

// ---------------------------------------------------------------- row 15
#[test]
fn row15_leading_ignorables() {
    let mut rng = Rng::new(0x1515);
    let set: &[u8] = b" \t:";
    for _ in 0..N {
        let nbytes = rng.below(9);
        let mut h: Vec<u8> = (0..1 + rng.below(5)).map(|_| *rng.pick(set)).collect();
        h.extend(digits(&mut rng, nbytes * 2, MIXED));
        let c = Case::new(&h)
            .ignore(set)
            .hex_end(true)
            .bin_cap(nbytes + 1)
            .bin_maxlen(nbytes);
        let out = assert_same("row15", &c);
        assert_eq!(out.ret as usize, nbytes);
        assert_eq!(out.hex_end, Some(h.len() as isize));
    }
}

// ---------------------------------------------------------------- row 16
#[test]
fn row16_trailing_ignorables_consumed() {
    let mut rng = Rng::new(0x1616);
    let set: &[u8] = b" \t:";
    for _ in 0..N {
        let nbytes = rng.below(9);
        let mut h = digits(&mut rng, nbytes * 2, MIXED);
        let ntrail = 1 + rng.below(5);
        for _ in 0..ntrail {
            h.push(*rng.pick(set));
        }
        let c = Case::new(&h)
            .ignore(set)
            .hex_end(true)
            .bin_cap(nbytes + 1)
            .bin_maxlen(nbytes);
        let out = assert_same("row16", &c);
        assert_eq!(out.ret as usize, nbytes);
        // trailing ignorables are consumed, so hex_end is the very end
        assert_eq!(out.hex_end, Some(h.len() as isize));
    }
}

// ---------------------------------------------------------------- row 17
#[test]
fn row17_ignorable_at_odd_nibble_breaks() {
    let mut rng = Rng::new(0x1717);
    let set: &[u8] = b" \t:";
    for _ in 0..N {
        let nbytes = rng.below(9);
        let mut h = digits(&mut rng, nbytes * 2, MIXED);
        h.push(*rng.pick(MIXED)); // one lone high nibble -> state != 0
        let odd_at = h.len();
        h.push(*rng.pick(set)); // ignorable, but state != 0 => break
        h.extend(digits(&mut rng, 4, MIXED));
        let c = Case::new(&h)
            .ignore(set)
            .hex_end(true)
            .bin_cap(nbytes + 8)
            .bin_maxlen(nbytes + 8);
        let out = assert_same("row17", &c);
        assert_eq!(out.ret, -1);
        // broke at `odd_at`, then hex_pos-- because state != 0
        assert_eq!(out.hex_end, Some(odd_at as isize - 1));
    }
}

// ---------------------------------------------------------------- row 18
#[test]
fn row18_empty_ignore_string() {
    let mut rng = Rng::new(0x1818);
    for _ in 0..N {
        let n = 2 * rng.below(17);
        let h = digits(&mut rng, n, MIXED);
        assert_same("row18/valid", &Case::new(&h).ignore(b"").hex_end(true));
    }
    // an empty ignore set still matches a NUL byte (strchr terminator quirk)
    let out = assert_same(
        "row18/nul",
        &Case::new(b"aa\x00bb").ignore(b"").hex_end(true).bin_cap(8).bin_maxlen(8),
    );
    assert_eq!(out.ret, 2);
    assert_eq!(out.hex_end, Some(5));
}

// ---------------------------------------------------------------- row 19
#[test]
fn row19_ignore_set_containing_hex_chars() {
    let mut rng = Rng::new(0x1919);
    let set: &[u8] = b"abc0";
    for _ in 0..N {
        let n = 2 * rng.below(17);
        let h = digits(&mut rng, n, MIXED);
        // hex-digit members of `ignore` are decoded, never skipped
        let out = assert_same("row19", &Case::new(&h).ignore(set).hex_end(true));
        assert_eq!(out.ret as usize, n / 2);
        assert_eq!(out.hex_end, Some(n as isize));
    }
}

// ---------------------------------------------------------------- row 20
#[test]
fn row20_ignore_set_high_bit_bytes() {
    let mut rng = Rng::new(0x2020);
    let set: &[u8] = b"\x80\xC3\xFF";
    for _ in 0..N {
        let nbytes = rng.below(13);
        let h = with_ignorables_even(&mut rng, nbytes, set, 2);
        let c = Case::new(&h)
            .ignore(set)
            .hex_end(true)
            .bin_cap(nbytes + 1)
            .bin_maxlen(nbytes);
        let out = assert_same("row20", &c);
        assert_eq!(out.ret as usize, nbytes);
        assert_eq!(out.hex_end, Some(h.len() as isize));
    }
}

// ---------------------------------------------------------------- row 21
#[test]
fn row21_embedded_nul_skipped_when_ignore_non_null() {
    let mut rng = Rng::new(0x2121);
    for _ in 0..N {
        let nbytes = rng.below(9);
        let h = with_ignorables_even(&mut rng, nbytes, b"\x00", 2);
        let c = Case::new(&h)
            .ignore(b" \t")
            .hex_end(true)
            .bin_cap(nbytes + 1)
            .bin_maxlen(nbytes);
        let out = assert_same("row21", &c);
        // NUL matches strchr's own terminator => treated as ignorable
        assert_eq!(out.ret as usize, nbytes);
        assert_eq!(out.hex_end, Some(h.len() as isize));
    }
}

// ---------------------------------------------------------------- row 22
#[test]
fn row22_embedded_nul_breaks_when_ignore_null() {
    let out = assert_same(
        "row22",
        &Case::new(b"aabb\x00ccdd")
            .no_ignore()
            .hex_end(true)
            .bin_cap(8)
            .bin_maxlen(8),
    );
    assert_eq!(out.ret, 2);
    assert_eq!(out.hex_end, Some(4));
    // `hex_len` shorter than the buffer: the scan must stop at hex_len, not NUL
    let out = assert_same(
        "row22/short-len",
        &Case::new(b"aabbccdd")
            .hex_len(4)
            .no_ignore()
            .hex_end(true)
            .bin_cap(8)
            .bin_maxlen(8),
    );
    assert_eq!(out.ret, 2);
    assert_eq!(out.hex_end, Some(4));
}

// ---------------------------------------------------------------- row 23
#[test]
fn row23_buffer_one_short_composed_with_ignore() {
    let mut rng = Rng::new(0x2323);
    let set: &[u8] = b" :";
    for _ in 0..N {
        let nbytes = 1 + rng.below(12);
        let h = with_ignorables_even(&mut rng, nbytes, set, 2);
        let c = Case::new(&h)
            .ignore(set)
            .hex_end(true)
            .bin_cap(nbytes) // real capacity: enough for the short maxlen
            .bin_maxlen(nbytes - 1);
        let out = assert_same("row23", &c);
        assert_eq!(out.ret, -1);
    }
}

// ---------------------------------------------------------------- row 24
#[test]
fn row24_all_256_bytes_across_ignore_and_hex_end() {
    let ignores: [Option<&[u8]>; 3] = [None, Some(b""), Some(b" \t")];
    for b in 0u16..256 {
        let b = b as u8;
        for ig in ignores.iter() {
            for &he in [false, true].iter() {
                for prefix in [None, Some(b'a')] {
                    let mut h = Vec::new();
                    if let Some(p) = prefix {
                        h.push(p);
                    }
                    h.push(b);
                    let mut c = Case::new(&h).hex_end(he).bin_cap(8).bin_maxlen(8);
                    c = match ig {
                        None => c.no_ignore(),
                        Some(s) => c.ignore(s),
                    };
                    assert_same(&format!("row24/b={b:#04x} ig={ig:?} he={he} pre={prefix:?}"), &c);
                }
            }
        }
    }
}

// ---------------------------------------------------------------- row 25
#[test]
fn row25_full_fuzz() {
    let mut rng = Rng::new(0xDEAD_BEEF_CAFE_F00D);
    let ignores: [Option<&[u8]>; 6] = [
        None,
        Some(b""),
        Some(b" "),
        Some(b" \t\n:"),
        Some(b"0aF"),
        Some(b"\x80\xFF"),
    ];
    // Bias the alphabet so plenty of cases are long valid runs, while still
    // covering the whole 0x00..0xFF byte space.
    for i in 0..200_000usize {
        let hex_len = rng.below(65);
        let biased = rng.below(4) != 0;
        let hex: Vec<u8> = (0..hex_len)
            .map(|_| {
                if biased && rng.below(8) != 0 {
                    *rng.pick(MIXED)
                } else {
                    rng.byte()
                }
            })
            .collect();
        let bin_maxlen = rng.below(41);
        let mut c = Case::new(&hex)
            .bin_cap(bin_maxlen.max(1))
            .bin_maxlen(bin_maxlen)
            .hex_end(rng.below(2) == 1);
        c = match rng.pick(&ignores) {
            None => c.no_ignore(),
            Some(s) => c.ignore(s),
        };
        assert_same(&format!("row25/#{i}"), &c);
    }
}

// ---------------------------------------------------------------- row 26
#[test]
fn row26_nibble_boundary_sweep() {
    let mut rng = Rng::new(0x2626);
    for n in 0..18usize {
        for bin_maxlen in 0..11usize {
            for &he in [false, true].iter() {
                for _ in 0..40 {
                    let h = digits(&mut rng, n, MIXED);
                    let c = Case::new(&h)
                        .no_ignore()
                        .hex_end(he)
                        .bin_cap(bin_maxlen.max(1))
                        .bin_maxlen(bin_maxlen);
                    assert_same(&format!("row26/n={n} maxlen={bin_maxlen} he={he}"), &c);
                }
            }
        }
    }
}

// ---------------------------------------------------------------- row 27
#[test]
fn row27_null_pointers_with_zero_lengths() {
    for &he in [false, true].iter() {
        let out = assert_same(
            &format!("row27/null-hex he={he}"),
            &Case::new(b"")
                .null_hex()
                .null_bin()
                .hex_len(0)
                .bin_cap(0)
                .bin_maxlen(0)
                .hex_end(he)
                .no_ignore(),
        );
        assert_eq!(out.ret, 0);
        if he {
            // &hex[0] on a NULL base is NULL => offset 0
            assert_eq!(out.hex_end, Some(0));
        }
        // null bin with a real (but unread-past) hex, nothing decodable
        assert_same(
            &format!("row27/null-bin he={he}"),
            &Case::new(b"zz")
                .null_bin()
                .bin_cap(0)
                .bin_maxlen(0)
                .hex_end(he)
                .no_ignore(),
        );
        // null hex, non-NULL ignore
        assert_same(
            &format!("row27/null-hex+ignore he={he}"),
            &Case::new(b"")
                .null_hex()
                .null_bin()
                .hex_len(0)
                .bin_cap(0)
                .bin_maxlen(0)
                .hex_end(he)
                .ignore(b" \t"),
        );
    }
}

// ---------------------------------------------------------------- row 28
#[test]
fn row28_output_window_witness_on_error_paths() {
    let mut rng = Rng::new(0x2828);
    // `Outcome.bin` always spans the full allocated capacity and starts life
    // poisoned, so `assert_same` already proves both implementations wrote the
    // same bytes and left the same bytes untouched. Here we force the error
    // paths specifically, with slack capacity beyond `bin_maxlen`.
    for _ in 0..N {
        let n = 1 + rng.below(24);
        let h = digits(&mut rng, n, MIXED);
        let maxlen = rng.below(n / 2 + 1); // usually too small
        let c = Case::new(&h)
            .no_ignore()
            .hex_end(rng.below(2) == 1)
            .bin_cap(maxlen + 4) // slack: poison beyond maxlen must stay poison
            .bin_maxlen(maxlen);
        let out = assert_same("row28", &c);
        assert!(
            out.bin[maxlen..].iter().all(|&b| b == POISON),
            "wrote past bin_maxlen: {:02x?}",
            out.bin
        );
    }
}
