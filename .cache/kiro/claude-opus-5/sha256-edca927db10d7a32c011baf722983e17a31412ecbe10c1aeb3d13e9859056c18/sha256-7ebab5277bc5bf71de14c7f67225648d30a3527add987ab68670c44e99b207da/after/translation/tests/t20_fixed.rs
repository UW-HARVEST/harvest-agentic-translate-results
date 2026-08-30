//! Level 2: `cp_fixed` / `cp_build` / `cp_decode` / `cp_block`.
//!
//! Fixed-Huffman blocks pin down the table built from `cp_fixed_table`, the
//! binary search in `cp_decode`, and every branch of `cp_block`: single
//! literals, the `memset` fast path for distance 1, the byte-by-byte
//! overlapping copy, and the three failure guards.

mod harness;

use harness::deflate::*;
use harness::Differ;

/// Trailing zero bytes: `cp_read_bits` asserts it never over-reads, so streams
/// must not end exactly on the last bit they need.
const PAD: usize = 16;

fn fixed_stream(tokens: &[Token]) -> Vec<u8> {
    let mut w = BitWriter::new();
    write_fixed_block(&mut w, true, tokens);
    with_padding(w.finish(), PAD)
}

#[test]
fn fixed_empty_block() {
    let mut d = Differ::new();
    let stream = fixed_stream(&[]);
    for offset in 0..8usize {
        for out_bytes in [0usize, 1, 16] {
            d.check(&format!("empty off={offset} out={out_bytes}"), &stream, offset, out_bytes);
        }
    }
    // With a non-zero output buffer this must succeed and write nothing.
    d.check_ok("empty", &stream, 0, 16, &[]);
    d.finish("fixed empty block");
}

#[test]
fn fixed_every_literal_byte() {
    // All 256 literal symbols, i.e. both halves of the fixed table (8-bit codes
    // for 0..=143, 9-bit codes for 144..=255).
    let mut d = Differ::new();
    let tokens: Vec<Token> = (0..=255u8).map(Token::Lit).collect();
    let expected = expand(&tokens);
    let stream = fixed_stream(&tokens);
    for offset in 0..8usize {
        d.check_ok(&format!("all literals off={offset}"), &stream, offset, 300, &expected);
    }
    // One case per single literal, to catch any single mis-assigned code.
    for b in 0..=255u8 {
        let t = [Token::Lit(b)];
        d.check_ok(&format!("literal {b}"), &fixed_stream(&t), 0, 4, &expand(&t));
    }
    d.finish("fixed literals");
}

#[test]
fn fixed_all_length_symbols() {
    // Every length symbol and every extra-bit boundary, at a distance that
    // takes the byte-by-byte copy path.
    let mut d = Differ::new();
    for len in 3..=258u32 {
        let mut tokens: Vec<Token> = (0..8u8).map(|i| Token::Lit(b'0' + i)).collect();
        tokens.push(Token::Match { len, dist: 8 });
        let expected = expand(&tokens);
        d.check_ok(
            &format!("len={len} dist=8"),
            &fixed_stream(&tokens),
            0,
            expected.len(),
            &expected,
        );
    }
    d.finish("fixed length symbols");
}

#[test]
fn fixed_distance_one_memset_path() {
    // `case 1: memset(dst, *src, length)` -- the only branch that is not a
    // straight byte loop.
    let mut d = Differ::new();
    for len in 3..=258u32 {
        let tokens = vec![Token::Lit(0xA5), Token::Match { len, dist: 1 }];
        let expected = expand(&tokens);
        d.check_ok(
            &format!("dist=1 len={len}"),
            &fixed_stream(&tokens),
            0,
            expected.len(),
            &expected,
        );
    }
    d.finish("fixed distance 1");
}

#[test]
fn fixed_overlapping_copies() {
    // dist < len: the copy must read bytes it has just written.
    let mut d = Differ::new();
    for dist in 1..=16u32 {
        for len in [3u32, 4, 5, 7, 11, 17, 31, 64, 258] {
            let seed: Vec<Token> = (0..dist as u8).map(|i| Token::Lit(b'a' + i)).collect();
            let mut tokens = seed;
            tokens.push(Token::Match { len, dist });
            tokens.push(Token::Match { len: 3, dist: 2 });
            let expected = expand(&tokens);
            d.check_ok(
                &format!("overlap dist={dist} len={len}"),
                &fixed_stream(&tokens),
                0,
                expected.len(),
                &expected,
            );
        }
    }
    d.finish("fixed overlapping copies");
}

#[test]
fn fixed_all_distance_symbols() {
    // Every distance symbol, including the 13-extra-bit ones, which needs up to
    // 32 KiB of history.
    let mut d = Differ::new();
    for sym in 0..30usize {
        let base = DIST_BASE[sym];
        let max = base + ((1u32 << DIST_EXTRA[sym]) - 1);
        for dist in [base, max] {
            if dist > 32768 {
                continue;
            }
            let mut tokens: Vec<Token> = Vec::new();
            // Build history cheaply: a few literals then long matches.
            tokens.push(Token::Lit(b'Z'));
            let mut have = 1u32;
            while have < dist {
                let n = (dist - have).min(258);
                if n < 3 {
                    for _ in 0..n {
                        tokens.push(Token::Lit(b'Y'));
                        have += 1;
                    }
                } else {
                    tokens.push(Token::Match { len: n, dist: 1 });
                    have += n;
                }
            }
            tokens.push(Token::Match { len: 3, dist });
            let expected = expand(&tokens);
            d.check_ok(
                &format!("dist sym={sym} dist={dist}"),
                &fixed_stream(&tokens),
                0,
                expected.len(),
                &expected,
            );
        }
    }
    d.finish("fixed distance symbols");
}

#[test]
fn fixed_output_overflow_guards() {
    let mut d = Differ::new();

    // "Attempted to overwrite out buffer while outputting a symbol."
    let lits: Vec<Token> = (0..16u8).map(|i| Token::Lit(b'A' + i)).collect();
    let stream = fixed_stream(&lits);
    for out_bytes in 0..20usize {
        d.check(&format!("symbol overflow out={out_bytes}"), &stream, 0, out_bytes);
    }

    // "Attempted to overwrite out buffer while outputting a string."
    let tokens = vec![
        Token::Lit(b'a'),
        Token::Lit(b'b'),
        Token::Lit(b'c'),
        Token::Match { len: 32, dist: 3 },
    ];
    let stream = fixed_stream(&tokens);
    for out_bytes in 0..40usize {
        d.check(&format!("string overflow out={out_bytes}"), &stream, 0, out_bytes);
    }

    // "Attempted to write before out buffer (invalid backwards distance)."
    for dist in [1u32, 2, 3, 8, 300] {
        let tokens = vec![Token::Lit(b'q'), Token::Match { len: 3, dist }];
        let stream = fixed_stream(&tokens);
        for out_bytes in [1usize, 2, 4, 16, 64] {
            d.check(
                &format!("bad distance dist={dist} out={out_bytes}"),
                &stream,
                0,
                out_bytes,
            );
        }
    }
    d.finish("fixed overflow guards");
}

#[test]
fn fixed_multiple_blocks_and_btype3() {
    let mut d = Differ::new();

    // Several non-final fixed blocks in a row.
    for nblocks in 1..6usize {
        let mut w = BitWriter::new();
        let mut expected: Vec<u8> = Vec::new();
        for i in 0..nblocks {
            let tokens: Vec<Token> = (0..4u8).map(|k| Token::Lit(b'0' + k + i as u8)).collect();
            write_fixed_block(&mut w, i + 1 == nblocks, &tokens);
            expected.extend(expand(&tokens));
        }
        let stream = with_padding(w.finish(), PAD);
        for offset in 0..4usize {
            d.check_ok(
                &format!("{nblocks} fixed blocks off={offset}"),
                &stream,
                offset,
                expected.len() + 8,
                &expected,
            );
        }
    }

    // BTYPE == 3 -> "Detected unknown block type within input stream."
    for bfinal in [0u32, 1] {
        let mut w = BitWriter::new();
        w.bits(bfinal, 1);
        w.bits(3, 2);
        let stream = with_padding(w.finish(), PAD);
        d.check(&format!("btype3 bfinal={bfinal}"), &stream, 0, 16);
    }

    // A fixed block whose error surfaces only in the *second* block.
    let mut w = BitWriter::new();
    write_fixed_block(&mut w, false, &[Token::Lit(b'x')]);
    w.bits(1, 1);
    w.bits(3, 2);
    let stream = with_padding(w.finish(), PAD);
    d.check("btype3 after fixed", &stream, 0, 16);

    d.finish("block sequencing");
}

#[test]
fn fixed_zero_and_max_out_bytes() {
    // `out_bytes` is an `int`; odd values are the caller's problem, but both
    // sides must agree on how they misbehave.
    let mut d = Differ::new();
    let tokens = vec![Token::Lit(b'a'), Token::Lit(b'b')];
    let stream = fixed_stream(&tokens);
    for out_bytes in [0usize, 1, 2, 3] {
        d.check(&format!("out={out_bytes}"), &stream, 0, out_bytes);
    }
    // in_bytes deliberately smaller/larger than the real input.
    for in_bytes in [1i32, 2, 3, 4, 5, stream.len() as i32, stream.len() as i32 + 8] {
        d.check_raw(&format!("in_bytes={in_bytes}"), &stream, 0, 32, in_bytes);
    }
    d.finish("odd sizes");
}
