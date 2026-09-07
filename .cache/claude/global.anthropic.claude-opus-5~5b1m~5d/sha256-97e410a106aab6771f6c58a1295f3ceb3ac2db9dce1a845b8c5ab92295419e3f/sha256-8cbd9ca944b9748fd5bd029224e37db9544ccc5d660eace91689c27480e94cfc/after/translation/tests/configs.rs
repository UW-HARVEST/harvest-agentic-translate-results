//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Every test drives BOTH the C `.so` and the Rust `.so` through their
//! exported `hex2bin` symbol and compares the return value, the entire output
//! buffer, and `*hex_end_p` byte-for-byte.

mod harness;
use harness::*;

const IGNORE_SETS: &[&[u8]] = &[b"", b" ", b":", b" :\n\t-"];

/// Build `n` random bytes worth of hex digits and splice separators from
/// `sep_set` in at random *byte* boundaries (state == 0), so every separator
/// is skippable by the C.
fn hex_with_separators_at_byte_boundaries(
    rng: &mut Rng,
    nbytes: usize,
    sep_set: &[u8],
    leading: bool,
    trailing: bool,
) -> Vec<u8> {
    let mut out = Vec::new();
    if leading && !sep_set.is_empty() {
        for _ in 0..rng.range(1, 3) {
            out.push(*rng.pick(sep_set));
        }
    }
    for i in 0..nbytes {
        if i > 0 && !sep_set.is_empty() && rng.bool() {
            for _ in 0..rng.range(1, 2) {
                out.push(*rng.pick(sep_set));
            }
        }
        out.push(rng.hex_digit());
        out.push(rng.hex_digit());
    }
    if trailing && !sep_set.is_empty() {
        for _ in 0..rng.range(1, 3) {
            out.push(*rng.pick(sep_set));
        }
    }
    out
}

// --- row 1 ---------------------------------------------------------------
#[test]
fn row01_empty_input_no_ignore_no_hex_end() {
    for cap in [0usize, 1, 8] {
        let call = Call::new(vec![])
            .bin_cap(cap)
            .bin_maxlen(0)
            .no_ignore()
            .hex_end(false);
        assert_same(&call, "row01 empty");
        let call = call.bin_maxlen(cap);
        assert_same(&call, "row01 empty maxlen=cap");
    }
}

// --- row 2 ---------------------------------------------------------------
#[test]
fn row02_single_byte_exact() {
    let mut rng = Rng::new(SEED ^ 2);
    for _ in 0..ITERS {
        let hex = rng.hex_digits(2);
        let call = Call::new(hex).bin_maxlen(1).no_ignore().hex_end(false);
        assert_same(&call, "row02 one byte");
    }
}

// --- row 3 ---------------------------------------------------------------
#[test]
fn row03_random_even_len_exact() {
    let mut rng = Rng::new(SEED ^ 3);
    for _ in 0..ITERS {
        let n = rng.range(2, 32) * 2;
        let hex = rng.hex_digits(n);
        let call = Call::new(hex).bin_maxlen(n / 2).no_ignore().hex_end(false);
        assert_same(&call, "row03 even exact");
    }
}

// --- row 4 ---------------------------------------------------------------
#[test]
fn row04_large_even_input() {
    let mut rng = Rng::new(SEED ^ 4);
    for _ in 0..32 {
        let n = rng.range(512, 2048) * 2;
        let hex = rng.hex_digits(n);
        let call = Call::new(hex).bin_maxlen(n / 2).no_ignore().hex_end(false);
        assert_same(&call, "row04 large");
    }
}

// --- row 5 ---------------------------------------------------------------
#[test]
fn row05_oversized_bin_maxlen_tail_untouched() {
    let mut rng = Rng::new(SEED ^ 5);
    for _ in 0..ITERS {
        let n = rng.range(1, 24) * 2;
        let hex = rng.hex_digits(n);
        let extra = rng.range(1, 32);
        let call = Call::new(hex)
            .bin_cap(n / 2 + extra + 8)
            .bin_maxlen(n / 2 + extra)
            .poison(rng.u8())
            .no_ignore()
            .hex_end(false);
        assert_same(&call, "row05 oversized maxlen");
    }
}

// --- row 6 ---------------------------------------------------------------
#[test]
fn row06_undersized_bin_maxlen() {
    let mut rng = Rng::new(SEED ^ 6);
    for _ in 0..ITERS {
        let n = rng.range(1, 24) * 2;
        let hex = rng.hex_digits(n);
        let maxlen = rng.below(n / 2 + 1); // 0 ..= n/2, often too small
        let call = Call::new(hex)
            .bin_maxlen(maxlen)
            .poison(rng.u8())
            .no_ignore()
            .hex_end(false);
        assert_same(&call, "row06 undersized maxlen");
    }
}

// --- row 7 ---------------------------------------------------------------
#[test]
fn row07_bin_maxlen_size_max() {
    let mut rng = Rng::new(SEED ^ 7);
    for _ in 0..ITERS {
        let n = rng.range(1, 24) * 2;
        let hex = rng.hex_digits(n);
        let call = Call::new(hex)
            .bin_cap(n / 2 + 8)
            .bin_maxlen(usize::MAX)
            .no_ignore()
            .hex_end(false);
        assert_same(&call, "row07 SIZE_MAX maxlen");
    }
}

// --- row 8 ---------------------------------------------------------------
#[test]
fn row08_odd_len_no_hex_end() {
    let mut rng = Rng::new(SEED ^ 8);
    for _ in 0..ITERS {
        let n = rng.range(0, 24) * 2 + 1; // 1, 3, 5, ...
        let hex = rng.hex_digits(n);
        let call = Call::new(hex)
            .bin_cap(n / 2 + 8)
            .bin_maxlen(n / 2 + 4)
            .no_ignore()
            .hex_end(false);
        assert_same(&call, "row08 odd len");
    }
}

// --- row 9 ---------------------------------------------------------------
#[test]
fn row09_hex_end_even_exact() {
    let mut rng = Rng::new(SEED ^ 9);
    for _ in 0..ITERS {
        let n = rng.range(1, 32) * 2;
        let hex = rng.hex_digits(n);
        let call = Call::new(hex).bin_maxlen(n / 2).no_ignore().hex_end(true);
        assert_same(&call, "row09 hex_end even");
    }
}

// --- row 10 --------------------------------------------------------------
#[test]
fn row10_hex_end_odd_len_rewind() {
    let mut rng = Rng::new(SEED ^ 10);
    for _ in 0..ITERS {
        let n = rng.range(0, 24) * 2 + 1;
        let hex = rng.hex_digits(n);
        let call = Call::new(hex)
            .bin_cap(n / 2 + 8)
            .bin_maxlen(n / 2 + 4)
            .no_ignore()
            .hex_end(true);
        assert_same(&call, "row10 hex_end odd rewind");
    }
}

// --- row 11 --------------------------------------------------------------
#[test]
fn row11_hex_end_truncated() {
    let mut rng = Rng::new(SEED ^ 11);
    for _ in 0..ITERS {
        let n = rng.range(1, 24) * 2;
        let hex = rng.hex_digits(n);
        let maxlen = rng.below(n / 2 + 1);
        let call = Call::new(hex)
            .bin_maxlen(maxlen)
            .poison(rng.u8())
            .no_ignore()
            .hex_end(true);
        assert_same(&call, "row11 hex_end truncated");
    }
}

// --- row 12 --------------------------------------------------------------
#[test]
fn row12_arbitrary_byte_injected() {
    let mut rng = Rng::new(SEED ^ 12);
    for _ in 0..(ITERS * 4) {
        let a = rng.range(0, 8) * 2;
        let b = rng.range(0, 8) * 2;
        let mut hex = rng.hex_digits(a);
        hex.push(rng.u8()); // ANY byte 0..=255
        hex.extend(rng.hex_digits(b));
        let n = hex.len();
        let call = Call::new(hex)
            .bin_cap(n + 8)
            .bin_maxlen(n / 2 + 4)
            .no_ignore()
            .hex_end(true);
        assert_same(&call, "row12 injected byte");
    }
}

// --- row 13 --------------------------------------------------------------
#[test]
fn row13_full_fuzz_with_hex_end() {
    let mut rng = Rng::new(SEED ^ 13);
    for _ in 0..(ITERS * 8) {
        let n = rng.below(65);
        let hex: Vec<u8> = (0..n).map(|_| rng.u8()).collect();
        let call = Call::new(hex)
            .bin_cap(48)
            .bin_maxlen(rng.below(41))
            .poison(rng.u8())
            .no_ignore()
            .hex_end(true);
        assert_same(&call, "row13 fuzz hex_end");
    }
}

// --- row 14 --------------------------------------------------------------
#[test]
fn row14_full_fuzz_no_hex_end() {
    let mut rng = Rng::new(SEED ^ 14);
    for _ in 0..(ITERS * 8) {
        let n = rng.below(65);
        let hex: Vec<u8> = (0..n).map(|_| rng.u8()).collect();
        let call = Call::new(hex)
            .bin_cap(48)
            .bin_maxlen(rng.below(41))
            .poison(rng.u8())
            .no_ignore()
            .hex_end(false);
        assert_same(&call, "row14 fuzz no hex_end");
    }
}

// --- row 15 --------------------------------------------------------------
#[test]
fn row15_empty_ignore_set() {
    let mut rng = Rng::new(SEED ^ 15);
    for _ in 0..(ITERS * 4) {
        let na = rng.range(0, 8) * 2;
        let mut hex = rng.hex_digits(na);
        if rng.bool() {
            let b = rng.u8();
            hex.push(b);
        }
        let nb = rng.range(0, 8) * 2;
        let tail = rng.hex_digits(nb);
        hex.extend(tail);
        let n = hex.len();
        let call = Call::new(hex)
            .bin_cap(n + 8)
            .bin_maxlen(n / 2 + 4)
            .ignore(b"")
            .hex_end(rng.bool());
        assert_same(&call, "row15 empty ignore");
    }
}

// --- row 16 --------------------------------------------------------------
#[test]
fn row16_space_ignore_at_byte_boundaries() {
    let mut rng = Rng::new(SEED ^ 16);
    for _ in 0..(ITERS * 2) {
        let nbytes = rng.range(0, 16);
        let hex = hex_with_separators_at_byte_boundaries(&mut rng, nbytes, b" ", false, false);
        let call = Call::new(hex)
            .bin_cap(nbytes + 8)
            .bin_maxlen(nbytes)
            .ignore(b" ")
            .hex_end(true);
        assert_same(&call, "row16 space at byte boundary");
    }
}

// --- row 17 --------------------------------------------------------------
#[test]
fn row17_space_ignore_at_nibble_boundary() {
    let mut rng = Rng::new(SEED ^ 17);
    for _ in 0..(ITERS * 2) {
        let nbytes = rng.range(1, 12);
        let mut hex = rng.hex_digits(nbytes * 2);
        // Insert a separator at an ODD index => state != 0 => NOT skipped.
        let odd_slots = nbytes; // indices 1, 3, 5, ...
        let idx = rng.below(odd_slots) * 2 + 1;
        hex.insert(idx, b' ');
        let n = hex.len();
        for he in [true, false] {
            let call = Call::new(hex.clone())
                .bin_cap(n + 8)
                .bin_maxlen(n / 2 + 4)
                .ignore(b" ")
                .hex_end(he);
            assert_same(&call, "row17 space at nibble boundary");
        }
    }
}

// --- row 18 --------------------------------------------------------------
#[test]
fn row18_colon_separated_pairs() {
    let mut rng = Rng::new(SEED ^ 18);
    for _ in 0..(ITERS * 2) {
        let nbytes = rng.range(1, 20);
        let mut hex = Vec::new();
        for i in 0..nbytes {
            if i > 0 {
                hex.push(b':');
            }
            hex.push(rng.hex_digit());
            hex.push(rng.hex_digit());
        }
        let call = Call::new(hex)
            .bin_cap(nbytes + 8)
            .bin_maxlen(nbytes)
            .ignore(b":")
            .hex_end(true);
        assert_same(&call, "row18 colon pairs");
    }
}

// --- row 19 --------------------------------------------------------------
#[test]
fn row19_multichar_ignore_set_hex_end() {
    let mut rng = Rng::new(SEED ^ 19);
    for _ in 0..(ITERS * 2) {
        let nbytes = rng.range(0, 16);
        let lead = rng.bool();
        let trail = rng.bool();
        let hex = hex_with_separators_at_byte_boundaries(
            &mut rng,
            nbytes,
            b" :\n\t-",
            lead,
            trail,
        );
        let n = hex.len();
        let call = Call::new(hex)
            .bin_cap(n + 8)
            .bin_maxlen(nbytes)
            .ignore(b" :\n\t-")
            .hex_end(true);
        assert_same(&call, "row19 multichar ignore hex_end");
    }
}

// --- row 20 --------------------------------------------------------------
#[test]
fn row20_multichar_ignore_set_no_hex_end() {
    let mut rng = Rng::new(SEED ^ 20);
    for _ in 0..(ITERS * 2) {
        let nbytes = rng.range(0, 16);
        let lead = rng.bool();
        let hex = hex_with_separators_at_byte_boundaries(
            &mut rng,
            nbytes,
            b" :\n\t-",
            lead,
            true, // trailing separators => hex_pos may still equal hex_len
        );
        let n = hex.len();
        let call = Call::new(hex)
            .bin_cap(n + 8)
            .bin_maxlen(nbytes)
            .ignore(b" :\n\t-")
            .hex_end(false);
        assert_same(&call, "row20 multichar ignore no hex_end");
    }
}

// --- row 21 --------------------------------------------------------------
#[test]
fn row21_embedded_nul_with_ignore_set() {
    let mut rng = Rng::new(SEED ^ 21);
    for _ in 0..(ITERS * 2) {
        let nbytes = rng.range(1, 12);
        let mut hex = rng.hex_digits(nbytes * 2);
        let idx = rng.below(hex.len() + 1);
        hex.insert(idx, 0u8);
        let n = hex.len();
        for set in IGNORE_SETS {
            for he in [true, false] {
                let call = Call::new(hex.clone())
                    .bin_cap(n + 8)
                    .bin_maxlen(n / 2 + 4)
                    .ignore(set)
                    .hex_end(he);
                assert_same(&call, "row21 embedded NUL, ignore != NULL");
            }
        }
    }
}

// --- row 22 --------------------------------------------------------------
#[test]
fn row22_embedded_nul_without_ignore_set() {
    let mut rng = Rng::new(SEED ^ 22);
    for _ in 0..(ITERS * 2) {
        let nbytes = rng.range(1, 12);
        let mut hex = rng.hex_digits(nbytes * 2);
        let idx = rng.below(hex.len() + 1);
        hex.insert(idx, 0u8);
        let n = hex.len();
        for he in [true, false] {
            let call = Call::new(hex.clone())
                .bin_cap(n + 8)
                .bin_maxlen(n / 2 + 4)
                .no_ignore()
                .hex_end(he);
            assert_same(&call, "row22 embedded NUL, ignore == NULL");
        }
    }
}

// --- row 23 --------------------------------------------------------------
#[test]
fn row23_undersized_maxlen_with_separators() {
    let mut rng = Rng::new(SEED ^ 23);
    for _ in 0..(ITERS * 2) {
        let nbytes = rng.range(1, 16);
        let hex = hex_with_separators_at_byte_boundaries(&mut rng, nbytes, b" :-", false, false);
        let n = hex.len();
        let maxlen = rng.below(nbytes + 1);
        for he in [true, false] {
            let call = Call::new(hex.clone())
                .bin_cap(n + 8)
                .bin_maxlen(maxlen)
                .poison(rng.u8())
                .ignore(b" :-")
                .hex_end(he);
            assert_same(&call, "row23 undersized maxlen + separators");
        }
    }
}

// --- row 24 --------------------------------------------------------------
#[test]
fn row24_odd_digits_with_separators() {
    let mut rng = Rng::new(SEED ^ 24);
    for _ in 0..(ITERS * 2) {
        let nbytes = rng.range(0, 12);
        let mut hex =
            hex_with_separators_at_byte_boundaries(&mut rng, nbytes, b" :-", false, false);
        // One extra digit => odd total digit count => state != 0 at the end.
        if !hex.is_empty() && rng.bool() {
            hex.push(*rng.pick(b" :-"));
        }
        hex.push(rng.hex_digit());
        let n = hex.len();
        for he in [true, false] {
            let call = Call::new(hex.clone())
                .bin_cap(n + 8)
                .bin_maxlen(n / 2 + 4)
                .ignore(b" :-")
                .hex_end(he);
            assert_same(&call, "row24 odd digits + separators");
        }
    }
}

// --- row 25 --------------------------------------------------------------
#[test]
fn row25_exhaustive_single_char_sweep() {
    for c in 0u16..=255 {
        let hex = vec![c as u8];
        for set in [None, Some(&b""[..]), Some(&b" "[..]), Some(&b" :\n\t-"[..])] {
            for he in [true, false] {
                for maxlen in [0usize, 1] {
                    let mut call = Call::new(hex.clone())
                        .bin_cap(8)
                        .bin_maxlen(maxlen)
                        .hex_end(he);
                    call = match set {
                        None => call.no_ignore(),
                        Some(s) => call.ignore(s),
                    };
                    assert_same(&call, &format!("row25 single char {c:#04x}"));
                }
            }
        }
    }
}

// --- row 26 --------------------------------------------------------------
#[test]
fn row26_exhaustive_two_char_sweep() {
    for c0 in 0u16..=255 {
        for c1 in 0u16..=255 {
            let call = Call::new(vec![c0 as u8, c1 as u8])
                .bin_cap(8)
                .bin_maxlen(1)
                .no_ignore()
                .hex_end(true);
            assert_same(&call, &format!("row26 pair {c0:#04x},{c1:#04x}"));
        }
    }
}

// --- row 27 --------------------------------------------------------------
#[test]
fn row27_bin_null_with_zero_maxlen() {
    for hex in [&b"a"[..], b"ab", b"00", b"ff", b"zz", b""] {
        for he in [true, false] {
            let call = Call::new(hex.to_vec())
                .bin_cap(0)
                .bin_maxlen(0)
                .bin_null()
                .no_ignore()
                .hex_end(he);
            assert_same(&call, "row27 bin=NULL, bin_maxlen=0");
        }
    }
}

// --- row 28 --------------------------------------------------------------
#[test]
fn row28_hex_null_with_zero_len() {
    for he in [true, false] {
        for set in [None, Some(&b" "[..])] {
            let mut call = Call::new(vec![])
                .bin_cap(8)
                .bin_maxlen(4)
                .hex_len(0)
                .hex_null()
                .hex_end(he);
            call = match set {
                None => call.no_ignore(),
                Some(s) => call.ignore(s),
            };
            assert_same(&call, "row28 hex=NULL, hex_len=0");
        }
    }
}

// --- row 29 --------------------------------------------------------------
#[test]
fn row29_all_degenerate_zero() {
    let call = Call::new(vec![])
        .bin_cap(0)
        .bin_maxlen(0)
        .hex_len(0)
        .ignore(b" :\n\t-")
        .hex_end(true);
    assert_same(&call, "row29 degenerate zeros");
    let call = call.hex_end(false);
    assert_same(&call, "row29 degenerate zeros, no hex_end");
}

// --- row 30 --------------------------------------------------------------
#[test]
fn row30_round_trip_case_variants() {
    const LOWER: &[u8; 16] = b"0123456789abcdef";
    const UPPER: &[u8; 16] = b"0123456789ABCDEF";
    let mut rng = Rng::new(SEED ^ 30);
    for _ in 0..ITERS {
        let n = rng.range(0, 40);
        let blob: Vec<u8> = (0..n).map(|_| rng.u8()).collect();

        for variant in 0..3 {
            let mut hex = Vec::with_capacity(n * 2);
            for &b in &blob {
                let tbl: &[u8; 16] = match variant {
                    0 => LOWER,
                    1 => UPPER,
                    _ => {
                        if rng.bool() {
                            LOWER
                        } else {
                            UPPER
                        }
                    }
                };
                hex.push(tbl[(b >> 4) as usize]);
                hex.push(tbl[(b & 0x0f) as usize]);
            }
            let call = Call::new(hex).bin_cap(n + 8).bin_maxlen(n).hex_end(true);
            assert_same(&call, "row30 round trip");

            // And confirm the shared result really is the original blob.
            let c = call.run_c();
            assert_eq!(c.ret, n as i32, "row30 expected {n} decoded bytes");
            assert_eq!(&c.bin[..n], &blob[..], "row30 round-trip mismatch");
        }
    }
}
