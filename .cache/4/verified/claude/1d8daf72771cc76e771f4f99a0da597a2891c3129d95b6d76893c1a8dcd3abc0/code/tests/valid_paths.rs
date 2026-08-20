//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every test drives BOTH shared objects
//! through the exported `decode_base64` symbol and compares the *entire*
//! destination allocation (`strlen(src)+14` bytes) byte for byte.

mod common;

use common::{b64_encode, diff, diff_buf, invalid_bytes, random_alphabet, Rng, ALPHABET};

/// Row 1 — exhaustive: every possible single-byte input.
#[test]
fn b01_exhaustive_single_byte() {
    for b in 1u16..=255 {
        diff(&[b as u8], &format!("single byte 0x{b:02x}"));
    }
}

/// Row 2 — exhaustive: all 65x65 alphabet pairs (`l % 4 == 2`).
#[test]
fn b02_exhaustive_alphabet_pairs() {
    for &x in ALPHABET.iter() {
        for &y in ALPHABET.iter() {
            diff(&[x, y], &format!("pair {:?}", [x as char, y as char]));
        }
    }
}

/// Row 3 — exhaustive: all 65^3 alphabet triples (`l % 4 == 3`, `'='` in every
/// position, every `decode` class in every group slot).
#[test]
fn b03_exhaustive_alphabet_triples() {
    let mut buf = [0u8; 4];
    for &x in ALPHABET.iter() {
        for &y in ALPHABET.iter() {
            for &z in ALPHABET.iter() {
                buf[0] = x;
                buf[1] = y;
                buf[2] = z;
                buf[3] = 0;
                diff_buf(&buf, "triple");
            }
        }
    }
}

/// Row 4 — randomized single 4-char groups, padding included.
#[test]
fn b04_random_alphabet_quads() {
    let mut rng = Rng::new(4);
    for _ in 0..20_000 {
        let v = random_alphabet(&mut rng, 4, true);
        diff(&v, "quad");
    }
    // and the systematic padding shapes of one group
    for &c3 in &[b'=', b'A', b'z', b'9', b'+', b'/'] {
        for &c4 in &[b'=', b'A', b'z', b'9', b'+', b'/'] {
            diff(&[b'Q', b'W', c3, c4], "quad padding matrix");
        }
    }
}

/// Row 5 — `l % 4 == 0`, many groups, no padding.
#[test]
fn b05_aligned_no_padding() {
    let mut rng = Rng::new(5);
    for _ in 0..3_000 {
        let groups = rng.range(1, 40);
        let v = random_alphabet(&mut rng, groups * 4, false);
        diff(&v, "aligned");
    }
}

fn mod4_case(seed: u64, rem: usize) {
    let mut rng = Rng::new(seed);
    for _ in 0..3_000 {
        let groups = rng.range(0, 40);
        let len = groups * 4 + rem;
        let v = random_alphabet(&mut rng, len, false);
        diff(&v, &format!("len % 4 == {rem}"));
    }
    // same but with '=' allowed in the tail
    let mut rng = Rng::new(seed ^ 0xabcd);
    for _ in 0..3_000 {
        let groups = rng.range(0, 40);
        let v = random_alphabet(&mut rng, groups * 4 + rem, true);
        diff(&v, &format!("len % 4 == {rem} (with '=')"));
    }
}

/// Row 6 — `l % 4 == 1`: c2, c3 and c4 keep their `'A'` default.
#[test]
fn b06_len_mod4_eq_1() {
    mod4_case(6, 1);
}

/// Row 7 — `l % 4 == 2`: c3 and c4 keep their `'A'` default.
#[test]
fn b07_len_mod4_eq_2() {
    mod4_case(7, 2);
}

/// Row 8 — `l % 4 == 3`: c4 keeps its `'A'` default.
#[test]
fn b08_len_mod4_eq_3() {
    mod4_case(8, 3);
}

/// Row 9 — canonical encodings of random binary payloads, with and without
/// padding, for every payload length class (`len % 3`).
#[test]
fn b09_canonical_roundtrip() {
    let mut rng = Rng::new(9);
    for _ in 0..4_000 {
        let n = rng.range(1, 90);
        let payload: Vec<u8> = (0..n).map(|_| rng.byte()).collect();
        for pad in [true, false] {
            let enc = b64_encode(&payload, pad);
            diff(&enc, "canonical base64");
        }
    }
    // payloads made of the extreme byte values
    for n in 1..=48usize {
        for fill in [0x00u8, 0xff, 0x80, 0x7f, 0x01] {
            let payload = vec![fill; n];
            diff(&b64_encode(&payload, true), "canonical fixed payload");
            diff(&b64_encode(&payload, false), "canonical fixed payload nopad");
        }
    }
}

/// Row 10 — `'='` at position c3/c4 of a *middle* group: the C keeps decoding
/// the following groups.
#[test]
fn b10_padding_mid_stream() {
    let mut rng = Rng::new(10);
    for _ in 0..5_000 {
        let groups = rng.range(2, 8);
        let mut v = random_alphabet(&mut rng, groups * 4, false);
        let g = rng.below(groups - 1); // not the last group
        let slot = rng.range(2, 3); // c3 or c4
        v[g * 4 + slot] = b'=';
        // sometimes also pad the very last group
        if rng.below(2) == 0 {
            v[(groups - 1) * 4 + 3] = b'=';
        }
        diff(&v, "mid-stream padding");
    }
    diff(b"QUJD=EFHSQ==", "explicit mid-stream padding");
    diff(b"QQ==QQ==QQ==", "repeated padded groups");
    diff(b"====QUJD", "leading padding group");
}

/// Row 11 — input made only of `'='`.
#[test]
fn b11_all_equals() {
    for n in 1..=64usize {
        diff(&vec![b'='; n], "all '='");
    }
}

/// Row 12 — `'='` sprinkled at random positions (including c1/c2 and more than
/// two of them).
#[test]
fn b12_random_equals_anywhere() {
    let mut rng = Rng::new(12);
    for _ in 0..10_000 {
        let len = rng.range(1, 32);
        let mut v = random_alphabet(&mut rng, len, false);
        let n_eq = rng.range(1, len);
        for _ in 0..n_eq {
            let i = rng.below(len);
            v[i] = b'=';
        }
        diff(&v, "random '=' positions");
    }
}

/// Row 13 — no alphabet byte at all: the decode loop never runs and the result
/// is a non-NULL, all-zero buffer.
#[test]
fn b13_no_alphabet_chars_at_all() {
    let bad = invalid_bytes();
    for &b in bad.iter() {
        diff(&[b], &format!("single invalid byte 0x{b:02x}"));
    }
    let mut rng = Rng::new(13);
    for _ in 0..5_000 {
        let len = rng.range(1, 40);
        let v: Vec<u8> = (0..len).map(|_| rng.pick(&bad)).collect();
        diff(&v, "only invalid bytes");
    }
    diff(b"...", "dots only");
    diff(b"\n\r\t ", "whitespace only");
    diff(b"----____", "url-safe alphabet only (rejected by is_base64)");
}

/// Row 14 — invalid *low* bytes interleaved with alphabet bytes.
#[test]
fn b14_interleaved_invalid_low_bytes() {
    let low: Vec<u8> = invalid_bytes().into_iter().filter(|&b| b < 0x80).collect();
    let mut rng = Rng::new(14);
    for _ in 0..15_000 {
        let len = rng.range(1, 48);
        let v: Vec<u8> = (0..len)
            .map(|_| {
                if rng.below(2) == 0 {
                    rng.pick(&low)
                } else {
                    rng.pick(ALPHABET)
                }
            })
            .collect();
        diff(&v, "interleaved low invalid bytes");
    }
    diff(b"Q!U#J$D%", "punctuation interleaved");
    diff(b"-QUJD_", "dash/underscore around payload");
}

/// Row 15 — bytes `0x80..=0xFF` (negative `char`) interleaved with alphabet.
#[test]
fn b15_interleaved_high_bytes() {
    let high: Vec<u8> = (0x80u16..=0xff).map(|b| b as u8).collect();
    let mut rng = Rng::new(15);
    for _ in 0..15_000 {
        let len = rng.range(1, 48);
        let v: Vec<u8> = (0..len)
            .map(|_| {
                if rng.below(2) == 0 {
                    rng.pick(&high)
                } else {
                    rng.pick(ALPHABET)
                }
            })
            .collect();
        diff(&v, "interleaved high bytes");
    }
    for &b in high.iter() {
        diff(&[b, b'Q', b, b'U', b, b'J', b, b'D', b], "high byte pattern");
    }
}

/// Row 16 — PEM-style wrapped base64 (whitespace must be ignored).
#[test]
fn b16_pem_style_wrapping() {
    let mut rng = Rng::new(16);
    for _ in 0..2_000 {
        let n = rng.range(1, 200);
        let payload: Vec<u8> = (0..n).map(|_| rng.byte()).collect();
        let enc = b64_encode(&payload, true);
        let mut wrapped = Vec::new();
        for (i, &c) in enc.iter().enumerate() {
            if i > 0 && i % 16 == 0 {
                wrapped.extend_from_slice(rng.pick(&[&b"\n"[..], &b"\r\n"[..], &b" "[..], &b"\t"[..]]));
            }
            wrapped.push(c);
        }
        wrapped.extend_from_slice(b"\n");
        diff(&wrapped, "PEM wrapped");
    }
    diff(b"QUJD\nRUZH\n", "two wrapped lines");
    diff(b"  QUJD  RUZH  ", "space separated");
}

/// Row 17 — fully random byte soup, short lengths.
#[test]
fn b17_random_bytes_small() {
    let mut rng = Rng::new(17);
    for _ in 0..40_000 {
        let len = rng.range(1, 64);
        let v: Vec<u8> = (0..len).map(|_| rng.nonzero_byte()).collect();
        diff(&v, "random bytes");
    }
}

/// Row 18 — random byte soup sweeping every length 1..=200.
#[test]
fn b18_length_sweep_1_to_200() {
    let mut rng = Rng::new(18);
    for len in 1..=200usize {
        for _ in 0..40 {
            let v: Vec<u8> = (0..len).map(|_| rng.nonzero_byte()).collect();
            diff(&v, &format!("random len {len}"));
        }
    }
}

/// Row 19 — alphabet-only sweep 1..=200 (worst case for the destination buffer:
/// nothing is filtered out, so `3*ceil(l/4)` bytes are written).
#[test]
fn b19_alphabet_length_sweep() {
    let mut rng = Rng::new(19);
    for len in 1..=200usize {
        for pad in [false, true] {
            for _ in 0..20 {
                let v = random_alphabet(&mut rng, len, pad);
                diff(&v, &format!("alphabet len {len} pad={pad}"));
            }
        }
        // deterministic extremes
        diff(&vec![b'A'; len], "all 'A'");
        diff(&vec![b'/'; len], "all '/'");
        diff(&vec![b'z'; len], "all 'z'");
        diff(&vec![b'9'; len], "all '9'");
        diff(&vec![b'+'; len], "all '+'");
    }
}

/// Row 20 — large inputs.
#[test]
fn b20_large_inputs() {
    let mut rng = Rng::new(20);
    for &len in &[4096usize, 4097, 4098, 4099, 65536, 65537, 1_000_003] {
        let v = random_alphabet(&mut rng, len, false);
        diff(&v, &format!("large alphabet {len}"));
        let v = random_alphabet(&mut rng, len, true);
        diff(&v, &format!("large alphabet+pad {len}"));
        let v: Vec<u8> = (0..len).map(|_| rng.nonzero_byte()).collect();
        diff(&v, &format!("large random bytes {len}"));
        let payload: Vec<u8> = (0..len / 2).map(|_| rng.byte()).collect();
        diff(&b64_encode(&payload, true), &format!("large canonical {len}"));
    }
}

/// Row 21 — every alphabet character in every group position.
#[test]
fn b21_alphabet_position_matrix() {
    let mut rng = Rng::new(21);
    for &c in ALPHABET.iter() {
        for slot in 0..4usize {
            for _ in 0..40 {
                let groups = rng.range(1, 4);
                let mut v = random_alphabet(&mut rng, groups * 4, false);
                let g = rng.below(groups);
                v[g * 4 + slot] = c;
                diff(&v, &format!("char {:?} at slot {slot}", c as char));
                // also with a truncated tail so the defaults kick in
                let cut = rng.range(1, v.len());
                diff(&v[..cut], &format!("char {:?} slot {slot} truncated", c as char));
            }
        }
    }
}

/// Row 22 — `'+'`/`'/'` heavy inputs (`decode` -> 62 / 63).
#[test]
fn b22_plus_slash_heavy() {
    let mut rng = Rng::new(22);
    let set = b"+/=";
    for _ in 0..20_000 {
        let len = rng.range(1, 32);
        let v: Vec<u8> = (0..len).map(|_| rng.pick(set)).collect();
        diff(&v, "+/= only");
    }
    for len in 1..=64usize {
        diff(&vec![b'+'; len], "all '+'");
        diff(&vec![b'/'; len], "all '/'");
        let v: Vec<u8> = (0..len).map(|i| if i % 2 == 0 { b'+' } else { b'/' }).collect();
        diff(&v, "alternating +/");
    }
}

/// Row 23 — the source buffer has bytes past the terminating NUL; the result
/// must equal that of the truncated string.
#[test]
fn b23_interior_nul_buffer() {
    let mut rng = Rng::new(23);
    for _ in 0..5_000 {
        let head = rng.range(1, 24);
        let tail = rng.range(1, 24);
        let mut buf: Vec<u8> = (0..head).map(|_| rng.pick(ALPHABET)).collect();
        buf.push(0);
        buf.extend((0..tail).map(|_| rng.nonzero_byte()));
        buf.push(0);
        diff_buf(&buf, "interior NUL buffer");
    }
    let mut b = b"QUJD\0IGNORED".to_vec();
    b.push(0);
    diff_buf(&b, "explicit interior NUL");
}

/// Row 24 — decoded payloads containing NUL bytes: the comparison covers the
/// whole allocation, so the write count and the `calloc` zero fill are checked
/// past the C-string terminator.
#[test]
fn b24_nul_bytes_in_payload() {
    let mut rng = Rng::new(24);
    for _ in 0..5_000 {
        let n = rng.range(1, 40);
        let mut payload: Vec<u8> = (0..n).map(|_| rng.byte()).collect();
        // force NUL bytes at the start, middle and end of the payload
        payload[0] = 0;
        payload[n / 2] = 0;
        payload[n - 1] = 0;
        for pad in [true, false] {
            diff(&b64_encode(&payload, pad), "payload with NULs");
        }
    }
    for n in 1..=40usize {
        diff(&b64_encode(&vec![0u8; n], true), "all-zero payload");
        diff(&b64_encode(&vec![0u8; n], false), "all-zero payload nopad");
    }
    // "AAAA..." decodes to zero bytes only
    for n in 1..=64usize {
        diff(&vec![b'A'; n], "'A' run decodes to NULs");
    }
}

/// Row 25 — no state is carried between calls (interleaved C/Rust invocations
/// in a long sequence, results must stay identical and reproducible).
#[test]
fn b25_no_cross_call_state() {
    let mut rng = Rng::new(25);
    let inputs: Vec<Vec<u8>> = (0..500)
        .map(|_| {
            let len = rng.range(1, 40);
            (0..len).map(|_| rng.nonzero_byte()).collect()
        })
        .collect();
    // forwards, backwards, then twice again - same expectations every time
    for round in 0..2 {
        for v in inputs.iter() {
            diff(v, &format!("round {round} fwd"));
        }
        for v in inputs.iter().rev() {
            diff(v, &format!("round {round} rev"));
        }
    }
    // long run of the *same* input
    for _ in 0..1_000 {
        diff(b"SGVsbG8sIFdvcmxkIQ==", "repeated identical call");
    }
}

/// Extra: known-answer sanity (documents what the C actually produces, so a
/// regression in *both* implementations would still be visible).
#[test]
fn b26_known_answers() {
    let cases: &[(&[u8], &[u8])] = &[
        (b"SGVsbG8sIFdvcmxkIQ==", b"Hello, World!"),
        (b"QQ==", b"A"),
        (b"QUI=", b"AB"),
        (b"QUJD", b"ABC"),
        (b"QUJDRA==", b"ABCD"),
        (b"aGVsbG8gd29ybGQ=", b"hello world"),
        (b"/w==", b"\xff"),
        (b"+w==", b"\xfb"),
    ];
    let a = common::api();
    for (enc, want) in cases {
        let mut src = enc.to_vec();
        src.push(0);
        diff_buf(&src, "known answer");
        let p = unsafe { (a.c)(src.as_ptr() as *const std::ffi::c_char) };
        assert!(!p.is_null());
        let got = unsafe { std::ffi::CStr::from_ptr(p) }.to_bytes().to_vec();
        unsafe { common::free(p as *mut std::ffi::c_void) };
        assert_eq!(
            got, *want,
            "C reference output changed for {:?}",
            String::from_utf8_lossy(enc)
        );
    }
}
