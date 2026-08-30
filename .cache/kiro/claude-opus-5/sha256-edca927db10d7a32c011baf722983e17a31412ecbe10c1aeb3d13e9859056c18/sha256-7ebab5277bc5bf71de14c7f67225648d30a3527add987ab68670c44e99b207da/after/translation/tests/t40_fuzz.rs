//! Level 4: randomised differential testing.
//!
//! Three generators, all seeded deterministically so failures reproduce:
//!
//! 1. structurally valid streams (random block sequences, tables, tokens),
//! 2. bit-flip / byte-splice mutations of those streams,
//! 3. uniformly random bytes.
//!
//! For (2) and (3) most inputs are malformed and both implementations trip an
//! assertion; the harness only demands that they agree on the outcome.

mod harness;

use harness::deflate::*;
use harness::{fuzz_iters, Differ, Rng};

const PAD: usize = 16;

/// Builds a random, structurally valid DEFLATE stream.
/// Returns the stream, the bytes it decodes to, and whether it contains a
/// stored block (whose output the C code derives from bit-buffer bookkeeping,
/// so it is not necessarily the textbook result).
fn random_stream(rng: &mut Rng) -> (Vec<u8>, Vec<u8>, bool) {
    let nblocks = 1 + rng.below(3);
    let mut w = BitWriter::new();
    let mut expected: Vec<u8> = Vec::new();

    for b in 0..nblocks {
        let last = b + 1 == nblocks;
        // A stored block is only decodable as the very last thing in the input.
        let kind = if last && rng.below(4) == 0 {
            0
        } else if rng.bool() {
            1
        } else {
            2
        };

        if kind == 0 {
            let len = rng.below(48);
            let data: Vec<u8> = (0..len).map(|_| rng.byte()).collect();
            write_stored_block(&mut w, true, &data);
            expected.extend_from_slice(&data);
            return (w.finish(), expected, true);
        }

        // Pick the alphabet sizes up front, so only tokens that actually have a
        // code get generated.
        let (nlit, ndst) = if kind == 1 {
            (288usize, 32usize)
        } else {
            (257 + rng.below(32), 1 + rng.below(32))
        };
        let max_len = (3..=258u32)
            .filter(|&len| 257 + len_symbol(len).0 < nlit)
            .last()
            .unwrap_or(0);
        let max_dist = (1..=32768u32)
            .filter(|&dist| dist_symbol(dist).0 < ndst)
            .last()
            .unwrap_or(0);

        let mut tokens: Vec<Token> = Vec::new();
        let mut produced = expected.len();
        let ntok = rng.below(24);
        for _ in 0..ntok {
            let can_match = max_len >= 3 && max_dist >= 1 && produced >= 1;
            if can_match && rng.below(3) == 0 {
                let dist_cap = (produced as u32).min(max_dist);
                let dist = 1 + rng.below(dist_cap as usize) as u32;
                let len = (3 + rng.below(40) as u32).min(max_len);
                if len >= 3 {
                    tokens.push(Token::Match { len, dist });
                    produced += len as usize;
                    continue;
                }
            }
            let byte = rng.byte();
            if (byte as usize) < nlit {
                tokens.push(Token::Lit(byte));
                produced += 1;
            }
        }

        if kind == 1 {
            write_fixed_block(&mut w, last, &tokens);
        } else {
            let (lit_lens, dist_lens) = tables_for(&tokens, nlit, ndst);
            let enc = match rng.below(3) {
                0 => ClEncoding::Literal,
                1 => ClEncoding::ZeroRuns,
                _ => ClEncoding::Full,
            };
            write_dynamic_block(&mut w, last, &tokens, &lit_lens, &dist_lens, enc, rng.bool());
        }

        for t in &tokens {
            match *t {
                Token::Lit(byte) => expected.push(byte),
                Token::Match { len, dist } => {
                    let start = expected.len() - dist as usize;
                    for i in 0..len as usize {
                        let byte = expected[start + i];
                        expected.push(byte);
                    }
                }
            }
        }
    }
    (w.finish(), expected, false)
}

#[test]
fn fuzz_valid_streams() {
    let mut d = Differ::new();
    let mut rng = Rng::new(0xC0FFEE);
    let mut exact = 0usize;
    for i in 0..fuzz_iters(3000) {
        let (stream, expected, has_stored) = random_stream(&mut rng);
        let stream = with_padding(stream, if has_stored { 0 } else { PAD });
        let offset = rng.below(8);
        let slack = rng.below(16);
        let out_bytes = expected.len() + slack;
        let name = format!("valid#{i}");
        if has_stored {
            // A stored block can only be the last thing in the input, which
            // leaves no padding for the blocks in front of it; the C code then
            // sometimes over-reads and asserts. Its output also comes from the
            // bit-buffer bookkeeping rather than the payload (see t10_stored),
            // so these streams are compared without an expected value.
            d.check(&name, &stream, offset, out_bytes.max(1));
        } else {
            exact += 1;
            d.check_ok(&name, &stream, offset, out_bytes, &expected);
        }
    }
    assert!(
        exact * 10 > fuzz_iters(3000) * 6,
        "only {exact} of {} generated streams are checked against an expected \
         decoding; the generator has drifted",
        fuzz_iters(3000)
    );
    d.finish("fuzz valid streams");
}

#[test]
fn fuzz_valid_streams_tight_output_buffers() {
    // Same streams, but with an output buffer that is too small, so the three
    // `cp_block` guards fire at arbitrary points.
    let mut d = Differ::new();
    let mut rng = Rng::new(0x1234_5678);
    for i in 0..fuzz_iters(3000) {
        let (stream, expected, has_stored) = random_stream(&mut rng);
        let stream = with_padding(stream, if has_stored { 0 } else { PAD });
        let cap = if expected.is_empty() {
            0
        } else {
            rng.below(expected.len() + 1)
        };
        d.check(&format!("tight#{i}"), &stream, rng.below(8), cap);
    }
    d.finish("fuzz tight output buffers");
}

#[test]
fn fuzz_mutated_streams() {
    let mut d = Differ::new();
    let mut rng = Rng::new(0xDEAD_BEEF);
    // A corpus of valid streams to mutate.
    let mut corpus: Vec<Vec<u8>> = Vec::new();
    let mut seed_rng = Rng::new(99);
    for _ in 0..fuzz_iters(60) {
        let (s, _, _) = random_stream(&mut seed_rng);
        corpus.push(with_padding(s, PAD));
    }

    for i in 0..fuzz_iters(8000) {
        let base = &corpus[rng.below(corpus.len())];
        let mut bytes = base.clone();
        if bytes.is_empty() {
            continue;
        }
        match rng.below(4) {
            0 => {
                // single bit flip
                let bit = rng.below(bytes.len() * 8);
                bytes[bit / 8] ^= 1 << (bit % 8);
            }
            1 => {
                // a few bit flips
                for _ in 0..1 + rng.below(5) {
                    let bit = rng.below(bytes.len() * 8);
                    bytes[bit / 8] ^= 1 << (bit % 8);
                }
            }
            2 => {
                // random byte overwrite
                let at = rng.below(bytes.len());
                bytes[at] = rng.byte();
            }
            _ => {
                // truncation
                let keep = rng.below(bytes.len());
                bytes.truncate(keep);
            }
        }
        d.check(
            &format!("mutant#{i}"),
            &bytes,
            rng.below(8),
            rng.below(256),
        );
    }
    d.finish("fuzz mutated streams");
}

#[test]
fn fuzz_random_bytes() {
    let mut d = Differ::new();
    let mut rng = Rng::new(0xABCD_1234);
    for i in 0..fuzz_iters(6000) {
        let len = rng.below(64);
        let bytes: Vec<u8> = (0..len).map(|_| rng.byte()).collect();
        d.check(
            &format!("random#{i}"),
            &bytes,
            rng.below(8),
            rng.below(128),
        );
    }
    // Longer random inputs, which reach deeper into the header parsing.
    for i in 0..fuzz_iters(2000) {
        let len = 64 + rng.below(512);
        let bytes: Vec<u8> = (0..len).map(|_| rng.byte()).collect();
        d.check(
            &format!("random-long#{i}"),
            &bytes,
            rng.below(8),
            rng.below(2048),
        );
    }
    d.finish("fuzz random bytes");
}

#[test]
fn fuzz_low_entropy_bytes() {
    // Random bytes drawn from a small alphabet hit valid-looking headers far
    // more often than uniform noise does.
    let mut d = Differ::new();
    let mut rng = Rng::new(0x5555_AAAA);
    for i in 0..fuzz_iters(6000) {
        let len = 1 + rng.below(96);
        let alphabet: [u8; 6] = [0x00, 0x01, 0x02, 0x03, 0xFF, 0xED];
        let bytes: Vec<u8> = (0..len)
            .map(|_| alphabet[rng.below(alphabet.len())])
            .collect();
        d.check(
            &format!("low-entropy#{i}"),
            &bytes,
            rng.below(8),
            rng.below(512),
        );
    }
    d.finish("fuzz low-entropy bytes");
}

/// Random *structurally parseable* dynamic headers: a real code-length alphabet
/// followed by an arbitrary symbol stream. This is the highest-risk area of the
/// decoder -- it drives `cp_build` with arbitrary length tables and reaches the
/// `lens` overrun far more often than byte-level noise does.
#[test]
fn fuzz_dynamic_headers() {
    let mut d = Differ::new();
    let mut rng = Rng::new(0x0BAD_F00D);
    for i in 0..fuzz_iters(6000) {
        // A complete code over a random subset of the 19 code-length symbols.
        let nused = 2 + rng.below(8);
        let mut used: Vec<usize> = Vec::new();
        while used.len() < nused {
            let s = rng.below(19);
            if !used.contains(&s) {
                used.push(s);
            }
        }
        // Make sure at least one plain length symbol is available.
        if !used.iter().any(|&s| s <= 15) {
            used.push(rng.below(16));
        }
        let cl_lens = complete_lengths(19, &used);

        let hlit = 257 + rng.below(32);
        let hdist = 1 + rng.below(32);

        let nsyms = rng.below(400);
        let mut syms: Vec<(usize, u32, u32)> = Vec::new();
        for _ in 0..nsyms {
            let sym = used[rng.below(used.len())];
            let (extra, nextra) = match sym {
                16 => (rng.below(4) as u32, 2),
                17 => (rng.below(8) as u32, 3),
                18 => (rng.below(128) as u32, 7),
                _ => (0, 0),
            };
            syms.push((sym, extra, nextra));
        }

        let mut w = BitWriter::new();
        write_dynamic_header_raw(&mut w, rng.bool(), hlit, hdist, &cl_lens, &syms);
        let stream = with_padding(w.finish(), 64);
        d.check(
            &format!("dynhdr#{i}"),
            &stream,
            rng.below(8),
            rng.below(1024),
        );
    }
    d.finish("fuzz dynamic headers");
}

#[test]
fn fuzz_structural_mutations() {
    // Insertions, deletions and splices, which shift every bit offset after the
    // edit point rather than perturbing a single field.
    let mut d = Differ::new();
    let mut rng = Rng::new(0x7E57_5EED);
    let mut corpus: Vec<Vec<u8>> = Vec::new();
    let mut seed_rng = Rng::new(4242);
    for _ in 0..fuzz_iters(60) {
        let (s, _, _) = random_stream(&mut seed_rng);
        corpus.push(with_padding(s, PAD));
    }

    for i in 0..fuzz_iters(6000) {
        let base = &corpus[rng.below(corpus.len())];
        if base.is_empty() {
            continue;
        }
        let mut bytes = base.clone();
        match rng.below(4) {
            0 => {
                let at = rng.below(bytes.len() + 1);
                bytes.insert(at, rng.byte());
            }
            1 => {
                let at = rng.below(bytes.len());
                bytes.remove(at);
            }
            2 => {
                let other = &corpus[rng.below(corpus.len())];
                if !other.is_empty() {
                    let cut = rng.below(bytes.len());
                    let take = rng.below(other.len());
                    bytes.truncate(cut);
                    bytes.extend_from_slice(&other[..take]);
                }
            }
            _ => {
                let at = rng.below(bytes.len());
                let n = 1 + rng.below(4);
                for k in 0..n {
                    if at + k < bytes.len() {
                        bytes[at + k] = rng.byte();
                    }
                }
            }
        }
        d.check(
            &format!("structmut#{i}"),
            &bytes,
            rng.below(8),
            rng.below(512),
        );
    }
    d.finish("fuzz structural mutations");
}
