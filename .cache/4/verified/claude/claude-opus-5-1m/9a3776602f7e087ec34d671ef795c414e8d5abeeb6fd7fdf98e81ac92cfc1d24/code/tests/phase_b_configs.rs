//! Phase B — valid-path differential tests, one test per row of `CONFIGS.md`.
//!
//! Every test loads BOTH shared objects through `libloading` (via
//! `examples/so_runner.rs`) and compares stdout/exit-status byte for byte.
//! Randomized rows use a fixed seed so failures are reproducible.

mod harness;
use harness::*;

// ===========================================================================
// A. low-level entry point: driver(s1, s2)
// ===========================================================================

/// C1 — both operands empty.
#[test]
fn cfg_c1_both_empty() {
    assert_driver_batch("c1", &[(vec![], vec![])]);
}

/// C2 — empty reject set: result must be strlen(s1).
#[test]
fn cfg_c2_empty_reject() {
    let mut rng = Rng::new(0xC2);
    let mut cases = vec![(b"a".to_vec(), vec![]), (b"abcdef".to_vec(), vec![])];
    for _ in 0..100 {
        let len = rng.below(130);
        cases.push((rng.bytes_nonzero(len), vec![]));
    }
    assert_driver_batch("c2", &cases);
}

/// C3 — empty s1 with a non-empty reject set: result must be 0.
#[test]
fn cfg_c3_empty_s1() {
    let mut rng = Rng::new(0xC3);
    let mut cases = vec![(vec![], b"abc".to_vec())];
    for _ in 0..100 {
        let len = 1 + rng.below(20);
        cases.push((vec![], rng.bytes_nonzero(len)));
    }
    assert_driver_batch("c3", &cases);
}

/// C4 — every single-byte s1 (0x01..0xFF) against matching and non-matching
/// single-byte reject sets.
#[test]
fn cfg_c4_single_bytes() {
    let mut cases = Vec::new();
    for b in 1u16..=255 {
        let b = b as u8;
        cases.push((vec![b], vec![b])); // match  -> 0
        let other = if b == 0xFF { 0x01 } else { b + 1 };
        cases.push((vec![b], vec![other])); // no match -> 1
        cases.push((vec![b], vec![other, b])); // match, 2nd position in s2
    }
    assert_driver_batch("c4", &cases);
}

/// C5 — the match is the first byte of s1 (result 0).
#[test]
fn cfg_c5_match_first() {
    let mut rng = Rng::new(0xC5);
    let mut cases = Vec::new();
    for _ in 0..120 {
        let len = 1 + rng.below(60);
        let s1 = rng.bytes_from(len, b"abcdefgh");
        let l2 = 1 + rng.below(4);
        let mut s2 = rng.bytes_from(l2, b"xyz");
        s2.push(s1[0]);
        cases.push((s1, s2));
    }
    assert_driver_batch("c5", &cases);
}

/// C6 — the match is somewhere in the middle of s1.
#[test]
fn cfg_c6_match_middle() {
    let mut rng = Rng::new(0xC6);
    let mut cases = Vec::new();
    for _ in 0..120 {
        let len = 3 + rng.below(60);
        // s1 over one alphabet, then a single foreign byte in the middle
        let mut s1 = rng.bytes_from(len, b"abcdefgh");
        let pos = 1 + rng.below(len - 2);
        s1[pos] = b'Z';
        cases.push((s1, b"Z".to_vec()));
    }
    assert_driver_batch("c6", &cases);
}

/// C7 — the match is the last byte of s1 (result strlen(s1)-1).
#[test]
fn cfg_c7_match_last() {
    let mut rng = Rng::new(0xC7);
    let mut cases = Vec::new();
    for _ in 0..120 {
        let len = 1 + rng.below(80);
        let mut s1 = rng.bytes_from(len, b"abcdefgh");
        s1[len - 1] = b'Q';
        cases.push((s1, b"Q".to_vec()));
    }
    assert_driver_batch("c7", &cases);
}

/// C8 — disjoint alphabets: no match at all (result = strlen(s1)).
#[test]
fn cfg_c8_no_match() {
    let mut rng = Rng::new(0xC8);
    let mut cases = Vec::new();
    for _ in 0..120 {
        let l1 = rng.below(90);
        let l2 = 1 + rng.below(10);
        cases.push((rng.bytes_from(l1, b"abcdef"), rng.bytes_from(l2, b"XYZ123")));
    }
    assert_driver_batch("c8", &cases);
}

/// C9 — reject set with duplicated bytes and/or longer than s1.
#[test]
fn cfg_c9_dup_reject_long_s2() {
    let mut rng = Rng::new(0xC9);
    let mut cases = vec![
        (b"abc".to_vec(), b"cccccccc".to_vec()),
        (b"a".to_vec(), b"aaaaaaaaaaaaaaaa".to_vec()),
    ];
    for _ in 0..120 {
        let l1 = rng.below(20);
        let l2 = 20 + rng.below(200);
        let s1 = rng.bytes_from(l1, b"abcde");
        let mut s2 = rng.bytes_from(l2, b"abcdeXY");
        // force duplicates
        for i in 0..s2.len() / 2 {
            s2[i] = s2[0];
        }
        cases.push((s1, s2));
    }
    assert_driver_batch("c9", &cases);
}

/// C10 — randomized printable-ASCII operands, random lengths 0..120.
#[test]
fn cfg_c10_random_ascii() {
    let mut rng = Rng::new(0x10);
    let mut cases = Vec::new();
    for _ in 0..300 {
        let l1 = rng.below(121);
        let l2 = rng.below(9);
        cases.push((rng.bytes_from(l1, ASCII), rng.bytes_from(l2, ASCII)));
    }
    assert_driver_batch("c10", &cases);
}

/// C11 — full byte range 0x01..0xFF, including bytes >= 0x80 (signed-char
/// hazard) and control characters.
#[test]
fn cfg_c11_random_full_bytes() {
    let mut rng = Rng::new(0x11);
    let mut cases = Vec::new();
    for _ in 0..300 {
        let l1 = rng.below(80);
        let l2 = rng.below(7);
        cases.push((rng.bytes_nonzero(l1), rng.bytes_nonzero(l2)));
    }
    // explicit high-byte matches
    cases.push((vec![0x80, 0x81, 0xff], vec![0xff]));
    cases.push((vec![0xff, 0xfe, 0x80, b'a'], vec![0x80]));
    cases.push((vec![0x7f, 0x80], vec![0x80]));
    assert_driver_batch("c11", &cases);
}

/// C12 — tiny alphabet: matches are dense, exercises early exits.
#[test]
fn cfg_c12_small_alphabet() {
    let mut rng = Rng::new(0x12);
    let mut cases = Vec::new();
    for _ in 0..200 {
        let l1 = rng.below(41);
        let l2 = rng.below(3);
        cases.push((rng.bytes_from(l1, b"ab"), rng.bytes_from(l2, b"ab")));
    }
    assert_driver_batch("c12", &cases);
}

/// C13 — s1 contains every byte 0x01..0xFF exactly once, s2 sweeps a single
/// byte over the whole range: the result sweeps 0..254.
#[test]
fn cfg_c13_index_sweep() {
    let s1: Vec<u8> = (1u16..=255).map(|b| b as u8).collect();
    let mut cases = Vec::new();
    for b in 1u16..=255 {
        cases.push((s1.clone(), vec![b as u8]));
    }
    assert_driver_batch("c13", &cases);
}

/// C14 — oversized operands (1 KiB / 4 KiB / 64 KiB), far past the 100-byte
/// buffers `main` uses.
#[test]
fn cfg_c14_oversized() {
    let mut rng = Rng::new(0x14);
    let mut cases = Vec::new();
    for size in [1024usize, 4096, 65536] {
        // no match at all
        let base = rng.bytes_from(size, b"abcdef");
        cases.push((base.clone(), b"XYZ".to_vec()));
        // match at the very end
        let mut last = base.clone();
        last[size - 1] = b'X';
        cases.push((last, b"X".to_vec()));
        // match in the middle
        let mut mid = base.clone();
        mid[size / 2] = b'X';
        cases.push((mid, b"X".to_vec()));
        // match at the very beginning
        let mut first = base.clone();
        first[0] = b'X';
        cases.push((first, b"X".to_vec()));
        // empty reject set over a huge string
        cases.push((base, vec![]));
    }
    assert_driver_batch("c14", &cases);
}

/// C15 — operands with an interior NUL: the C string ends there.
#[test]
fn cfg_c15_interior_nul() {
    let mut rng = Rng::new(0x15);
    let mut cases = vec![
        (b"abc\0def".to_vec(), b"d".to_vec()),
        (b"abc\0def".to_vec(), b"c".to_vec()),
        (b"\0abc".to_vec(), b"a".to_vec()),
        (b"abc".to_vec(), b"\0c".to_vec()),
        (b"abc".to_vec(), b"c\0a".to_vec()),
    ];
    for _ in 0..150 {
        let l1 = 1 + rng.below(40);
        let l2 = 1 + rng.below(8);
        let mut s1 = rng.bytes_from(l1, b"abcde");
        let mut s2 = rng.bytes_from(l2, b"abcde");
        s1[rng.below(l1)] = 0;
        if rng.below(2) == 0 {
            s2[rng.below(l2)] = 0;
        }
        cases.push((s1, s2));
    }
    assert_driver_batch("c15", &cases);
}

/// C16 — many `driver` calls in a single process: output ordering / buffering.
#[test]
fn cfg_c16_repeated_calls() {
    let mut rng = Rng::new(0x16);
    let mut cases = Vec::new();
    for _ in 0..1000 {
        let l1 = rng.below(30);
        let l2 = rng.below(4);
        cases.push((rng.bytes_from(l1, b"abcXY"), rng.bytes_from(l2, b"abcXY")));
    }
    assert_driver_batch("c16", &cases);
}

// ===========================================================================
// B. whole-program entry point: main() over stdin
// ===========================================================================

/// C17 — no input at all.
#[test]
fn cfg_c17_zero_lines() {
    assert_main_bytes("c17", b"");
}

/// C18 — a single line terminated with '\n'.
#[test]
fn cfg_c18_one_line_nl() {
    for s in [&b"a\n"[..], b"abc\n", b"abcdef\n", b"aaaaaa\n"] {
        assert_main_bytes("c18", s);
    }
}

/// C19 — a single line **without** a trailing newline.
#[test]
fn cfg_c19_one_line_no_nl() {
    for s in [&b"a"[..], b"abc", b"abcdef", b"xyz"] {
        assert_main_bytes("c19", s);
    }
}

/// C20 — the nominal case: two newline-terminated lines, randomized.
#[test]
fn cfg_c20_two_lines_random() {
    let mut rng = Rng::new(0x20);
    for i in 0..200 {
        let l1 = rng.below(40);
        let l2 = rng.below(10);
        let mut input = rng.bytes_from(l1, b"abcdefXY");
        input.push(b'\n');
        input.extend_from_slice(&rng.bytes_from(l2, b"abcdefXY"));
        input.push(b'\n');
        assert_main_bytes(&format!("c20_{i}"), &input);
    }
}

/// C21 — two lines, the second without a trailing newline.
#[test]
fn cfg_c21_second_no_nl() {
    let mut rng = Rng::new(0x21);
    for i in 0..100 {
        let l1 = rng.below(40);
        let l2 = rng.below(10);
        let mut input = rng.bytes_from(l1, b"abcdefXY");
        input.push(b'\n');
        input.extend_from_slice(&rng.bytes_from(l2, b"abcdefXY"));
        assert_main_bytes(&format!("c21_{i}"), &input);
    }
}

/// C22 — more than two lines: the surplus must be ignored.
#[test]
fn cfg_c22_surplus_lines() {
    let mut rng = Rng::new(0x22);
    for i in 0..60 {
        let mut input = Vec::new();
        let lines = 3 + rng.below(5);
        for _ in 0..lines {
            let ll = rng.below(20);
            input.extend_from_slice(&rng.bytes_from(ll, b"abcXY"));
            input.push(b'\n');
        }
        assert_main_bytes(&format!("c22_{i}"), &input);
    }
}

/// C23 — first line empty.
#[test]
fn cfg_c23_empty_first_line() {
    for s in [&b"\nabc\n"[..], b"\n\nabc\n", b"\nabc", b"\n"] {
        assert_main_bytes("c23", s);
    }
}

/// C24 — both lines empty.
#[test]
fn cfg_c24_both_lines_empty() {
    for s in [&b"\n\n"[..], b"\n\n\n", b"\n\nabc\n"] {
        assert_main_bytes("c24", s);
    }
}

/// C25 — sweep line 1 around the `fgets` cap (`sizeof(s1)-1 == 99`).
#[test]
fn cfg_c25_cap_sweep() {
    let mut rng = Rng::new(0x25);
    for len in 90usize..=110 {
        for with_nl in [false, true] {
            let mut input = rng.bytes_from(len, b"abcdef");
            if with_nl {
                input.push(b'\n');
            }
            input.extend_from_slice(b"cX\n");
            assert_main_bytes(&format!("c25_{len}_{with_nl}"), &input);
        }
    }
    // exactly-at-the-cap variants with a match-bearing tail
    for len in [98usize, 99, 100, 101] {
        let mut input = vec![b'a'; len];
        input.push(b'\n');
        input.extend_from_slice(b"a\n");
        assert_main_bytes(&format!("c25b_{len}"), &input);
    }
}

/// C26 — line 1 far longer than the cap, so its tail becomes line 2.
#[test]
fn cfg_c26_spill_into_s2() {
    let mut rng = Rng::new(0x26);
    for len in [150usize, 198, 199, 200, 201, 1000, 10240] {
        let mut input = rng.bytes_from(len, b"abcdef");
        input.push(b'\n');
        input.extend_from_slice(b"zzz\n");
        assert_main_bytes(&format!("c26_{len}"), &input);
    }
    // 99 a's followed by 51 b's: s1 = "a"*98, s2 = "b"*51 -> 98
    let mut input = vec![b'a'; 99];
    input.extend_from_slice(&vec![b'b'; 51]);
    input.push(b'\n');
    assert_main_bytes("c26_99a51b", &input);
}

/// C27 — NUL bytes at many positions in line 1 and line 2.
#[test]
fn cfg_c27_nul_positions() {
    let base: &[u8] = b"abcdefghij";
    for pos in 0..base.len() {
        let mut l1 = base.to_vec();
        l1[pos] = 0;
        let mut input = l1.clone();
        input.extend_from_slice(b"\ncdj\n");
        assert_main_bytes(&format!("c27_l1_{pos}"), &input);

        let mut input = base.to_vec();
        input.push(b'\n');
        let mut l2 = base.to_vec();
        l2[pos] = 0;
        input.extend_from_slice(&l2);
        input.push(b'\n');
        assert_main_bytes(&format!("c27_l2_{pos}"), &input);
    }
    // NUL as the very first byte of stdin / of line 2
    assert_main_bytes("c27_first", b"\0abc\nxyz\n");
    assert_main_bytes("c27_l2first", b"abc\n\0xyz\n");
    assert_main_bytes("c27_only", b"\0\n\0\n");
    // NUL past the fgets cap
    let mut input = vec![b'a'; 120];
    input[105] = 0;
    input.push(b'\n');
    assert_main_bytes("c27_past_cap", &input);
}

/// C28 — CRLF line endings: the '\r' survives the chop.
#[test]
fn cfg_c28_crlf() {
    for s in [
        &b"abc\r\nc\r\n"[..],
        b"abc\r\n\r\n",
        b"\r\n\r\n",
        b"abc\r\nx\r\n",
        b"abc\r",
    ] {
        assert_main_bytes("c28", s);
    }
}

/// C29 — non-UTF-8 / high-byte payloads on both lines.
#[test]
fn cfg_c29_high_bytes() {
    let mut rng = Rng::new(0x29);
    assert_main_bytes("c29_fixed", b"\xff\xfe\x80abc\n\x80\n");
    for i in 0..150 {
        let l1 = rng.below(60);
        let l2 = rng.below(10);
        let mut input: Vec<u8> = (0..l1).map(|_| 0x80 | (rng.byte() & 0x7f)).collect();
        input.push(b'\n');
        input.extend((0..l2).map(|_| 0x80 | (rng.byte() & 0x7f)));
        input.push(b'\n');
        assert_main_bytes(&format!("c29_{i}"), &input);
    }
    // mixed ASCII / high bytes / invalid UTF-8 sequences
    for s in [
        &b"\xc3\x28abc\n\x28\n"[..],
        b"\xe2\x82\xac\n\x82\n",
        b"\xf0\x9f\x92\xa9\n\x9f\n",
    ] {
        assert_main_bytes("c29_utf8", s);
    }
}

/// C30 — stdin is a pipe (non-seekable) rather than a regular file.
#[test]
fn cfg_c30_pipe_stdin() {
    let mut rng = Rng::new(0x30);
    for s in [&b""[..], b"abc\n", b"abc", b"abcdef\ncd\n", b"\n\n"] {
        assert_main_bytes_pipe("c30", s);
    }
    for i in 0..60 {
        let l1 = rng.below(140);
        let l2 = rng.below(20);
        let mut input = rng.bytes_from(l1, b"abcdefXY");
        input.push(b'\n');
        input.extend_from_slice(&rng.bytes_from(l2, b"abcdefXY"));
        input.push(b'\n');
        assert_main_bytes_pipe(&format!("c30_{i}"), &input);
    }
    // /dev/null stdin (immediate EOF, character device)
    assert_main(
        "c30_devnull",
        StdinKind::DevNull,
        StdinKind::DevNull,
    );
}

/// C31 — unstructured fuzz over the whole stdin shape space.
#[test]
fn cfg_c31_fuzz_stdin() {
    let mut rng = Rng::new(0x31);
    for i in 0..400 {
        let len = rng.below(261);
        let mut input = Vec::with_capacity(len);
        for _ in 0..len {
            let r = rng.below(16);
            let b = match r {
                0 | 1 => b'\n',
                2 => 0u8,
                3 => 0x80 | (rng.byte() & 0x7f),
                4 => b'\r',
                _ => ASCII[rng.below(ASCII.len())],
            };
            input.push(b);
        }
        assert_main_bytes(&format!("c31_{i}"), &input);
    }
}

/// C32 — 1-byte and 2-byte inputs, exhaustively over the interesting bytes.
#[test]
fn cfg_c32_tiny_inputs() {
    for b in [0u8, b'\n', b'\r', b'a', 0x7f, 0x80, 0xff] {
        assert_main_bytes(&format!("c32_{b}"), &[b]);
        for c in [0u8, b'\n', b'a', 0xff] {
            assert_main_bytes(&format!("c32_{b}_{c}"), &[b, c]);
        }
    }
}

// ===========================================================================
// C. executable parity (the CMake target vs the Rust bin)
// ===========================================================================

/// C33 — the two standalone executables over the same fuzz corpus.
#[test]
fn cfg_c33_executables() {
    let mut rng = Rng::new(0x33);
    for s in [
        &b""[..],
        b"abc\n",
        b"abc",
        b"abcdef\ncd\n",
        b"\n\n",
        b"\0abc\nxyz\n",
        b"\xff\xfe\x80abc\n\x80\n",
    ] {
        assert_exe_bytes("c33", s);
    }
    for i in 0..150 {
        let len = rng.below(261);
        let mut input = Vec::with_capacity(len);
        for _ in 0..len {
            let r = rng.below(16);
            let b = match r {
                0 | 1 => b'\n',
                2 => 0u8,
                3 => 0x80 | (rng.byte() & 0x7f),
                4 => b'\r',
                _ => ASCII[rng.below(ASCII.len())],
            };
            input.push(b);
        }
        assert_exe_bytes(&format!("c33_{i}"), &input);
    }
}

/// C34 — stdin is a pipe delivered in small chunks with pauses, so `read(2)`
/// returns short reads in the middle of a line (the `fgets` refill path).
#[test]
fn cfg_c34_chunked_pipe_stdin() {
    let payloads: Vec<Vec<u8>> = vec![
        b"abcdef\ncd\n".to_vec(),
        b"abcdef\ncd".to_vec(),
        b"abc\n".to_vec(),
        b"\n\n".to_vec(),
        {
            let mut v = vec![b'a'; 150];
            v.push(b'\n');
            v.extend_from_slice(b"a\n");
            v
        },
        b"ab\0cd\ncb\n".to_vec(),
    ];
    for (i, p) in payloads.iter().enumerate() {
        for chunk in [1usize, 3, 7] {
            let label = format!("c34_{i}_{chunk}");
            assert_main(
                &label,
                StdinKind::PipeChunked(p, chunk),
                StdinKind::PipeChunked(p, chunk),
            );
        }
    }
}

/// C35 — the exported `main()` called repeatedly in one process: C's `FILE*`
/// stdin keeps its buffered remainder between calls, so the translation must
/// not discard buffered input either.
#[test]
fn cfg_c35_repeated_main_calls() {
    let mut rng = Rng::new(0x35);
    let payloads: Vec<Vec<u8>> = vec![
        b"abc\ndef\nghi\njkl\n".to_vec(),
        b"abcdef\ncd\nxyz\nz\n".to_vec(),
        b"a\nb\nc\nd\ne\nf\n".to_vec(),
        b"abc\ndef\n".to_vec(),
        b"".to_vec(),
        {
            let mut v = Vec::new();
            for _ in 0..40 {
                let l = rng.below(30);
                v.extend_from_slice(&rng.bytes_from(l, b"abcXY"));
                v.push(b'\n');
            }
            v
        },
        {
            // lines crossing the 4 KiB stdio buffer boundary
            let mut v = Vec::new();
            for _ in 0..80 {
                v.extend_from_slice(&vec![b'q'; 60]);
                v.push(b'\n');
            }
            v
        },
    ];
    for (i, p) in payloads.iter().enumerate() {
        for n in [1usize, 2, 3, 5] {
            assert_main_repeat(&format!("c35_{i}_{n}"), p, n);
        }
    }
}
