//! Phase C — error-path differential tests, one test per row of `ERRORS.md`.
//!
//! Each test constructs the exact invalid input/condition, calls BOTH `.so`s,
//! and asserts they return the SAME sentinel (`-1`) *and* the same `hex_end_p`
//! and the same output-window bytes — not merely "both failed somehow".

mod common;
use common::*;

const N: usize = 2000;

fn digits(rng: &mut Rng, n: usize, charset: &[u8]) -> Vec<u8> {
    (0..n).map(|_| *rng.pick(charset)).collect()
}

// ============================================================ ERRORS.md row 1
// Output buffer full mid-stream, hex_end_p != NULL.
#[test]
fn e01_buffer_full_midstream() {
    let mut rng = Rng::new(0xE001);
    for _ in 0..N {
        let nbytes = 2 + rng.below(16);
        let maxlen = rng.below(nbytes); // strictly fewer than needed
        let h = digits(&mut rng, nbytes * 2, MIXED);
        let c = Case::new(&h)
            .no_ignore()
            .hex_end(true)
            .bin_cap(maxlen + 4)
            .bin_maxlen(maxlen);
        let out = assert_same("e01", &c);
        assert_eq!(out.ret, -1, "buffer-full must yield the -1 sentinel");
        // the offending digit is the (2*maxlen)-th, i.e. the high nibble of the
        // byte that would not fit
        assert_eq!(out.hex_end, Some(2 * maxlen as isize));
        // bytes already written are NOT rolled back: check them exactly
        // (comparing against POISON is unsound, since 0xA5 is a legal output).
        let expect: Vec<u8> = (0..maxlen)
            .map(|i| {
                let hi = (h[2 * i] as char).to_digit(16).unwrap() as u8;
                let lo = (h[2 * i + 1] as char).to_digit(16).unwrap() as u8;
                hi * 16 + lo
            })
            .collect();
        assert_eq!(
            &out.bin[..maxlen],
            &expect[..],
            "the first {maxlen} decoded bytes must survive the rejection"
        );
        assert!(
            out.bin[maxlen..].iter().all(|&b| b == POISON),
            "nothing may be written past bin_maxlen: {:02x?}",
            out.bin
        );
    }
}

// ============================================================ ERRORS.md row 2
// bin_maxlen == 0 with at least one valid digit.
#[test]
fn e02_bin_maxlen_zero() {
    for &he in [false, true].iter() {
        for b in MIXED.iter() {
            let c = Case::new(&[*b, *b])
                .no_ignore()
                .hex_end(he)
                .bin_cap(4)
                .bin_maxlen(0);
            let out = assert_same(&format!("e02/{b} he={he}"), &c);
            assert_eq!(out.ret, -1);
            if he {
                assert_eq!(out.hex_end, Some(0));
            }
            assert!(out.bin.iter().all(|&x| x == POISON), "nothing may be written");
        }
    }
    // also with a genuinely NULL bin pointer
    let out = assert_same(
        "e02/null-bin",
        &Case::new(b"ab")
            .null_bin()
            .no_ignore()
            .hex_end(true)
            .bin_cap(0)
            .bin_maxlen(0),
    );
    assert_eq!(out.ret, -1);
    assert_eq!(out.hex_end, Some(0));
}

// ============================================================ ERRORS.md row 3
// Odd digit count => hex_pos-- and -1.
#[test]
fn e03_odd_digit_count_decrements_hex_pos() {
    let mut rng = Rng::new(0xE003);
    for _ in 0..N {
        let n = 2 * rng.below(17) + 1; // always odd
        let h = digits(&mut rng, n, MIXED);
        let c = Case::new(&h)
            .no_ignore()
            .hex_end(true)
            .bin_cap(n / 2 + 4)
            .bin_maxlen(n / 2 + 4);
        let out = assert_same("e03", &c);
        assert_eq!(out.ret, -1);
        // points AT the unpaired digit (hex_pos was n, decremented to n-1)
        assert_eq!(out.hex_end, Some(n as isize - 1));
    }
    // odd count followed by non-hex garbage
    let out = assert_same(
        "e03/garbage",
        &Case::new(b"aabbc!!!")
            .no_ignore()
            .hex_end(true)
            .bin_cap(8)
            .bin_maxlen(8),
    );
    assert_eq!(out.ret, -1);
    assert_eq!(out.hex_end, Some(4)); // broke at index 5, then --
}

// ============================================================ ERRORS.md row 4
// hex_end_p == NULL and the scan stopped early on a non-ignorable byte.
#[test]
fn e04_null_hex_end_early_stop() {
    let mut rng = Rng::new(0xE004);
    let set: &[u8] = b" \t";
    for _ in 0..N {
        let nbytes = rng.below(9);
        let mut h = digits(&mut rng, nbytes * 2, MIXED);
        // a byte that is neither hex nor in the ignore set
        loop {
            let b = rng.byte();
            if !is_c_hex(b) && !set.contains(&b) && b != 0 {
                h.push(b);
                break;
            }
        }
        h.extend(digits(&mut rng, 2, MIXED));
        let c = Case::new(&h)
            .ignore(set)
            .hex_end(false)
            .bin_cap(nbytes + 4)
            .bin_maxlen(nbytes + 4);
        let out = assert_same("e04", &c);
        assert_eq!(out.ret, -1, "unconsumed input with hex_end_p==NULL must be -1");
    }
}

// ============================================================ ERRORS.md row 5
// hex_end_p == NULL, ignore == NULL, any non-hex byte anywhere.
#[test]
fn e05_null_hex_end_null_ignore_nonhex() {
    let mut rng = Rng::new(0xE005);
    for _ in 0..N {
        let nbytes = rng.below(9);
        let mut h = digits(&mut rng, nbytes * 2, MIXED);
        loop {
            let b = rng.byte();
            if !is_c_hex(b) {
                h.push(b);
                break;
            }
        }
        h.extend(digits(&mut rng, 2, MIXED));
        let c = Case::new(&h)
            .no_ignore()
            .hex_end(false)
            .bin_cap(nbytes + 4)
            .bin_maxlen(nbytes + 4);
        let out = assert_same("e05", &c);
        assert_eq!(out.ret, -1);
    }
    // exhaustive: every non-hex byte value, at a byte boundary, ignore == NULL
    for b in 0u16..256 {
        let b = b as u8;
        if is_c_hex(b) {
            continue;
        }
        let c = Case::new(&[b'a', b'a', b])
            .no_ignore()
            .hex_end(false)
            .bin_cap(8)
            .bin_maxlen(8);
        let out = assert_same(&format!("e05/exhaustive b={b:#04x}"), &c);
        assert_eq!(out.ret, -1);
    }
}

// ============================================================ ERRORS.md row 6
// Ignorable byte at an ODD nibble => break (ignore is only honoured at state==0).
#[test]
fn e06_ignorable_at_odd_nibble() {
    let mut rng = Rng::new(0xE006);
    let set: &[u8] = b" \t:-";
    for _ in 0..N {
        let nbytes = rng.below(9);
        let mut h = digits(&mut rng, nbytes * 2, MIXED);
        h.push(*rng.pick(MIXED)); // now state != 0
        h.push(*rng.pick(set)); // ignorable, but not honoured here
        h.extend(digits(&mut rng, 2, MIXED));
        let c = Case::new(&h)
            .ignore(set)
            .hex_end(false)
            .bin_cap(nbytes + 8)
            .bin_maxlen(nbytes + 8);
        let out = assert_same("e06/no-hex-end", &c);
        assert_eq!(out.ret, -1);

        let c2 = c.clone().hex_end(true);
        let out2 = assert_same("e06/hex-end", &c2);
        assert_eq!(out2.ret, -1, "state != 0 alone is already an error");
    }
    // exhaustive over the ignore set members
    for b in b" \t:-".iter() {
        let out = assert_same(
            &format!("e06/exhaustive {b:#04x}"),
            &Case::new(&[b'a', b'a', b'b', *b, b'c'])
                .ignore(b" \t:-")
                .hex_end(true)
                .bin_cap(8)
                .bin_maxlen(8),
        );
        assert_eq!(out.ret, -1);
        assert_eq!(out.hex_end, Some(2)); // broke at 3, then hex_pos--
    }
}

// ============================================================ ERRORS.md row 7
// hex_end_p == NULL AND odd digit count (both L45 and L52 fire).
#[test]
fn e07_null_hex_end_and_odd_count() {
    let mut rng = Rng::new(0xE007);
    for _ in 0..N {
        let n = 2 * rng.below(17) + 1;
        let h = digits(&mut rng, n, MIXED);
        let c = Case::new(&h)
            .no_ignore()
            .hex_end(false)
            .bin_cap(n / 2 + 4)
            .bin_maxlen(n / 2 + 4);
        let out = assert_same("e07", &c);
        assert_eq!(out.ret, -1, "must be exactly -1, not a compounded value");
    }
}

// ============================================================ ERRORS.md row 8
// Any rejection forces bin_pos = 0, so the return is exactly -1, never partial.
#[test]
fn e08_no_partial_count_ever() {
    let mut rng = Rng::new(0xE008);
    let ignores: [Option<&[u8]>; 3] = [None, Some(b""), Some(b" \t:")];
    let mut saw_error = 0usize;
    for _ in 0..N * 4 {
        let hex_len = rng.below(33);
        let hex: Vec<u8> = (0..hex_len)
            .map(|_| if rng.below(4) == 0 { rng.byte() } else { *rng.pick(MIXED) })
            .collect();
        let maxlen = rng.below(20);
        let mut c = Case::new(&hex)
            .bin_cap(maxlen + 2)
            .bin_maxlen(maxlen)
            .hex_end(rng.below(2) == 1);
        c = match rng.pick(&ignores) {
            None => c.no_ignore(),
            Some(s) => c.ignore(s),
        };
        let out = assert_same("e08", &c);
        assert!(out.ret == -1 || out.ret >= 0);
        if out.ret < 0 {
            saw_error += 1;
            assert_eq!(out.ret, -1, "the only negative return value is -1");
        }
    }
    assert!(saw_error > 100, "expected the fuzz to reach the error paths often");
}

// ============================================================ ERRORS.md row 9
// bin_maxlen exhausted AND hex_end_p == NULL.
#[test]
fn e09_buffer_full_with_null_hex_end() {
    let mut rng = Rng::new(0xE009);
    for _ in 0..N {
        let nbytes = 2 + rng.below(16);
        let maxlen = rng.below(nbytes);
        let h = digits(&mut rng, nbytes * 2, MIXED);
        let c = Case::new(&h)
            .no_ignore()
            .hex_end(false)
            .bin_cap(maxlen + 2)
            .bin_maxlen(maxlen);
        let out = assert_same("e09", &c);
        assert_eq!(out.ret, -1);
    }
}

// ============================================================ ERRORS.md row 10
// The bin_pos >= bin_maxlen check also runs for HIGH nibbles: 2*maxlen+1 valid
// digits must fail rather than "succeed with truncation".
#[test]
fn e10_buffer_full_on_high_nibble() {
    let mut rng = Rng::new(0xE010);
    for maxlen in 0..12usize {
        for _ in 0..100 {
            let n = 2 * maxlen + 1; // one extra high nibble
            let h = digits(&mut rng, n, MIXED);
            for &he in [false, true].iter() {
                let c = Case::new(&h)
                    .no_ignore()
                    .hex_end(he)
                    .bin_cap(maxlen + 2)
                    .bin_maxlen(maxlen);
                let out = assert_same(&format!("e10/maxlen={maxlen} he={he}"), &c);
                assert_eq!(out.ret, -1);
                if he {
                    // broke on the digit at index 2*maxlen, state == 0 so no --
                    assert_eq!(out.hex_end, Some(2 * maxlen as isize));
                }
            }
        }
    }
    // and exactly 2*maxlen digits must succeed (the boundary on the other side)
    for maxlen in 0..12usize {
        let h = digits(&mut rng, 2 * maxlen, MIXED);
        let out = assert_same(
            &format!("e10/exact maxlen={maxlen}"),
            &Case::new(&h)
                .no_ignore()
                .hex_end(true)
                .bin_cap(maxlen + 2)
                .bin_maxlen(maxlen),
        );
        assert_eq!(out.ret as usize, maxlen);
    }
}

// ==================================================== generic boundary cases
// ERRORS.md G1/G2/G3: null pointers and zero lengths.
#[test]
fn g01_g03_null_and_zero_lengths() {
    for &he in [false, true].iter() {
        let out = assert_same(
            &format!("g01 he={he}"),
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
            assert_eq!(out.hex_end, Some(0));
        }
        let out = assert_same(
            &format!("g03 he={he}"),
            &Case::new(b"aabb")
                .hex_len(0)
                .no_ignore()
                .hex_end(he)
                .bin_cap(4)
                .bin_maxlen(4),
        );
        assert_eq!(out.ret, 0);
        if he {
            assert_eq!(out.hex_end, Some(0));
        }
    }
}

// ERRORS.md G4/G5/G6: the strchr-NUL-terminator quirk.
#[test]
fn g04_g06_nul_byte_quirk() {
    // ignore == "" still "contains" NUL
    let out = assert_same(
        "g04",
        &Case::new(b"\x00\x00aa")
            .ignore(b"")
            .hex_end(true)
            .bin_cap(8)
            .bin_maxlen(8),
    );
    assert_eq!(out.ret, 1);
    assert_eq!(out.hex_end, Some(4));

    // NUL at an ODD nibble is NOT skipped (state != 0)
    let out = assert_same(
        "g05/odd",
        &Case::new(b"a\x00a")
            .ignore(b" ")
            .hex_end(true)
            .bin_cap(8)
            .bin_maxlen(8),
    );
    assert_eq!(out.ret, -1);
    assert_eq!(out.hex_end, Some(0));

    // ignore == NULL: NUL just breaks
    let out = assert_same(
        "g06",
        &Case::new(b"aa\x00aa")
            .no_ignore()
            .hex_end(true)
            .bin_cap(8)
            .bin_maxlen(8),
    );
    assert_eq!(out.ret, 1);
    assert_eq!(out.hex_end, Some(2));
    // same input, hex_end_p == NULL => error
    let out = assert_same(
        "g06/null-end",
        &Case::new(b"aa\x00aa")
            .no_ignore()
            .hex_end(false)
            .bin_cap(8)
            .bin_maxlen(8),
    );
    assert_eq!(out.ret, -1);
}

// ERRORS.md G7: one step past every accepted range.
#[test]
fn g07_one_past_each_range_boundary() {
    // '/' ':' '@' 'G' '`' 'g' plus the inclusive endpoints for contrast
    let probes: [(u8, bool); 12] = [
        (0x2F, false), // '/'
        (0x30, true),  // '0'
        (0x39, true),  // '9'
        (0x3A, false), // ':'
        (0x40, false), // '@'
        (0x41, true),  // 'A'
        (0x46, true),  // 'F'
        (0x47, false), // 'G'
        (0x60, false), // '`'
        (0x61, true),  // 'a'
        (0x66, true),  // 'f'
        (0x67, false), // 'g'
    ];
    for (b, accepted) in probes {
        assert_eq!(is_c_hex(b), accepted, "test oracle wrong for {b:#04x}");
        for &he in [false, true].iter() {
            let out = assert_same(
                &format!("g07/{b:#04x} he={he}"),
                &Case::new(&[b'0', b])
                    .no_ignore()
                    .hex_end(he)
                    .bin_cap(8)
                    .bin_maxlen(8),
            );
            if accepted {
                assert_eq!(out.ret, 1, "{b:#04x} should decode");
            } else {
                assert_eq!(out.ret, -1, "{b:#04x} should be rejected (odd nibble)");
            }
        }
    }
}

// ERRORS.md G8: high-bit bytes, incl. the ones that would alias to A-F/a-f if
// bit 7 were dropped by the `c & ~32U` trick.
#[test]
fn g08_high_bit_bytes_rejected() {
    for b in 0x80u16..0x100 {
        let b = b as u8;
        assert!(!is_c_hex(b));
        let out = assert_same(
            &format!("g08/{b:#04x}"),
            &Case::new(&[b'a', b'a', b])
                .no_ignore()
                .hex_end(true)
                .bin_cap(8)
                .bin_maxlen(8),
        );
        assert_eq!(out.ret, 1, "{b:#04x} must not be decoded as a hex digit");
        assert_eq!(out.hex_end, Some(2));
    }
}

// ERRORS.md G9: oversized bin_maxlen.
#[test]
fn g09_oversized_bin_maxlen() {
    let mut rng = Rng::new(0xE0_09_00);
    for _ in 0..500 {
        let n = 2 * rng.below(17);
        let h = digits(&mut rng, n, MIXED);
        for maxlen in [usize::MAX, usize::MAX - 1, isize::MAX as usize] {
            let out = assert_same(
                &format!("g09/{maxlen}"),
                &Case::new(&h).no_ignore().hex_end(true).bin_cap(64).bin_maxlen(maxlen),
            );
            assert_eq!(out.ret as usize, n / 2);
        }
    }
}

// ERRORS.md G10: hex_len must bound the scan, not a NUL or the allocation.
#[test]
fn g10_hex_len_bounds_the_scan() {
    let mut rng = Rng::new(0xE010_0000);
    for _ in 0..N {
        let full = digits(&mut rng, 32, MIXED);
        let used = 2 * rng.below(17);
        let out = assert_same(
            "g10",
            &Case::new(&full)
                .hex_len(used)
                .no_ignore()
                .hex_end(true)
                .bin_cap(32)
                .bin_maxlen(32),
        );
        assert_eq!(out.ret as usize, used / 2);
        assert_eq!(out.hex_end, Some(used as isize));
    }
}

// The API takes no enums; the equivalent "any bit pattern is legal" input class
// is the hex byte space, driven exhaustively here in every nibble position and
// across the ignore / hex_end_p axes.
#[test]
fn g11_exhaustive_byte_space_all_positions() {
    let ignores: [Option<&[u8]>; 4] = [None, Some(b""), Some(b" \t"), Some(b"\x00\xFF")];
    for b in 0u16..256 {
        let b = b as u8;
        for pad in 0..3usize {
            let mut h = vec![b'a'; pad];
            h.push(b);
            h.push(b'b');
            for ig in ignores.iter() {
                for &he in [false, true].iter() {
                    for maxlen in [0usize, 1, 8] {
                        let mut c = Case::new(&h).hex_end(he).bin_cap(maxlen + 1).bin_maxlen(maxlen);
                        c = match ig {
                            None => c.no_ignore(),
                            Some(s) => c.ignore(s),
                        };
                        assert_same(
                            &format!("g11/b={b:#04x} pad={pad} ig={ig:?} he={he} max={maxlen}"),
                            &c,
                        );
                    }
                }
            }
        }
    }
}
