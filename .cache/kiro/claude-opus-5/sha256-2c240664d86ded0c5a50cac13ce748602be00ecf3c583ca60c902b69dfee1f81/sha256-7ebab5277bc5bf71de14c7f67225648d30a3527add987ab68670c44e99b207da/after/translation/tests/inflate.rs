//! Level 1: `cp_inflate` — the lowest-level exported entry point.
//!
//! Every case is driven through both `.so` files with an identical input
//! pointer (the 4-byte alignment of `in` changes `cp_inflate`'s word split, so
//! it has to be the same for both) and separate output buffers that are then
//! compared byte for byte, slack included.

mod common;

use common::*;

fn compare(label: &str, data: &[u8], out_bytes: usize, align: usize) {
    compare_backed(label, data, out_bytes, align, out_bytes + 4096 + data.len() * 2)
}

fn compare_backed(label: &str, data: &[u8], out_bytes: usize, align: usize, backing: usize) {
    let p = pair();
    let input = AlignedInput::new(data, align);
    let c = run_inflate_backed(&p.c, &input, out_bytes, backing);
    let r = run_inflate_backed(&p.rs, &input, out_bytes, backing);
    assert_inflate_eq(label, &c, &r);
}

/// All fixtures, all four input alignments, output buffer exactly the right
/// size.
#[test]
fn fixtures_exact_out_all_alignments() {
    for (name, data, ulen) in deflate_fixtures() {
        for align in 0..4 {
            compare(&format!("{name} align={align} out=exact"), &data, ulen, align);
        }
    }
}

/// Output buffer larger than required: nothing may be written past the real
/// output length, and the trailing slack must stay untouched in both.
#[test]
fn fixtures_oversized_out() {
    for (name, data, ulen) in deflate_fixtures() {
        for extra in [1usize, 7, 64, 1000] {
            for align in 0..4 {
                compare(
                    &format!("{name} align={align} out=+{extra}"),
                    &data,
                    ulen + extra,
                    align,
                );
            }
        }
    }
}

/// Output buffer too small: exercises the three "out buffer" error paths and
/// checks that both implementations bail at exactly the same byte.
#[test]
fn fixtures_undersized_out() {
    for (name, data, ulen) in deflate_fixtures() {
        if ulen == 0 {
            continue;
        }
        let mut sizes = vec![0usize, 1, ulen / 2];
        if ulen > 3 {
            sizes.push(ulen - 1);
            sizes.push(ulen - 3);
        }
        sizes.sort();
        sizes.dedup();
        for out_bytes in sizes {
            for align in 0..4 {
                compare(
                    &format!("{name} align={align} out={out_bytes}"),
                    &data,
                    out_bytes,
                    align,
                );
            }
        }
    }
}

/// A hand-built stored (BTYPE=00) block, plus the "LEN/NLEN are not
/// complements" and "unknown block type" error paths.
#[test]
fn stored_blocks_and_block_type_errors() {
    let payloads: [&[u8]; 6] = [
        b"",
        b"x",
        b"hello",
        b"0123456789abcdef",
        &[0u8; 37],
        &[0xFFu8; 100],
    ];
    for payload in payloads {
        let len = payload.len() as u16;
        let mut s = vec![0x01u8]; // BFINAL=1, BTYPE=00
        s.extend_from_slice(&len.to_le_bytes());
        s.extend_from_slice(&(!len).to_le_bytes());
        s.extend_from_slice(payload);
        for align in 0..4 {
            for out in [payload.len(), payload.len() + 32] {
                compare(
                    &format!("stored len={len} align={align} out={out}"),
                    &s,
                    out,
                    align,
                );
            }
        }
        // broken NLEN
        let mut bad = s.clone();
        bad[3] ^= 0xFF;
        for align in 0..4 {
            compare(
                &format!("stored_bad_nlen len={len} align={align}"),
                &bad,
                payload.len() + 32,
                align,
            );
        }
    }

    // BTYPE=11 is invalid: 0b111 -> byte 0x07
    for align in 0..4 {
        compare(
            &format!("btype3 align={align}"),
            &[0x07u8, 0, 0, 0, 0, 0, 0, 0],
            32,
            align,
        );
    }
}

/// The single "empty fixed-Huffman block" stream, in all alignments and with a
/// variety of trailing padding lengths, so that `first_bytes`/`last_bytes` and
/// the `final_word` path get every combination.
#[test]
fn empty_fixed_block_padding_matrix() {
    // BFINAL=1, BTYPE=01, then the 7-bit end-of-block code 0000000.
    let base = [0x03u8, 0x00];
    for pad in 0..9usize {
        let mut s = base.to_vec();
        s.extend(std::iter::repeat(0u8).take(pad));
        for align in 0..4 {
            compare(&format!("empty_fixed pad={pad} align={align}"), &s, 16, align);
        }
    }
}

/// Minimal fixed-Huffman DEFLATE writer so that arbitrary literal/back-
/// reference sequences can be constructed exactly.
struct BitW {
    bytes: Vec<u8>,
    nbits: u32,
}

const LEN_BASE: [u32; 29] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258,
];
const LEN_EXTRA: [u32; 29] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
];
const DIST_BASE: [u32; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
];
const DIST_EXTRA: [u32; 30] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13,
];

impl BitW {
    fn new() -> BitW {
        BitW {
            bytes: Vec::new(),
            nbits: 0,
        }
    }

    /// LSB-first: used for headers and extra bits.
    fn bits(&mut self, val: u32, n: u32) {
        for i in 0..n {
            let bit = (val >> i) & 1;
            if self.nbits % 8 == 0 {
                self.bytes.push(0);
            }
            let last = self.bytes.len() - 1;
            self.bytes[last] |= (bit as u8) << (self.nbits % 8);
            self.nbits += 1;
        }
    }

    /// MSB-first: used for Huffman codes.
    fn code(&mut self, code: u32, n: u32) {
        for i in (0..n).rev() {
            self.bits((code >> i) & 1, 1);
        }
    }

    fn literal(&mut self, sym: u32) {
        if sym <= 143 {
            self.code(0x30 + sym, 8);
        } else if sym <= 255 {
            self.code(0x190 + (sym - 144), 9);
        } else if sym <= 279 {
            self.code(sym - 256, 7);
        } else {
            self.code(0xC0 + (sym - 280), 8);
        }
    }

    fn end_of_block(&mut self) {
        self.literal(256);
    }

    fn back_ref(&mut self, length: u32, dist: u32) {
        let li = (0..29)
            .rev()
            .find(|&i| LEN_BASE[i] <= length)
            .expect("length out of range");
        assert!(length - LEN_BASE[li] < (1 << LEN_EXTRA[li]) || LEN_EXTRA[li] == 0);
        self.literal(257 + li as u32);
        self.bits(length - LEN_BASE[li], LEN_EXTRA[li]);
        let di = (0..30)
            .rev()
            .find(|&i| DIST_BASE[i] <= dist)
            .expect("dist out of range");
        self.code(di as u32, 5);
        self.bits(dist - DIST_BASE[di], DIST_EXTRA[di]);
    }
}

/// Builds a single final fixed-Huffman block from a literal prefix followed by
/// one back-reference.
fn fixed_block(prefix: &[u8], refs: &[(u32, u32)]) -> (Vec<u8>, usize) {
    let mut w = BitW::new();
    w.bits(1, 1); // BFINAL
    w.bits(1, 2); // BTYPE = 01 (fixed)
    let mut outlen = 0usize;
    for &b in prefix {
        w.literal(b as u32);
        outlen += 1;
    }
    for &(len, dist) in refs {
        w.back_ref(len, dist);
        outlen += len as usize;
    }
    w.end_of_block();
    (w.bytes, outlen)
}

/// Distance-1 back references drive the `memset` special case; longer
/// distances drive the byte-copy loop. This walks the interesting boundaries
/// of both, including overlapping copies.
#[test]
fn back_reference_boundaries() {
    for &dist in &[1u32, 2, 3, 4, 5, 8, 15, 16, 17, 31, 32, 33, 100, 257, 258, 300] {
        let prefix: Vec<u8> = (0..dist).map(|i| ((i * 37 + 11) & 0xFF) as u8).collect();
        for &len in &[3u32, 4, 5, 17, 18, 19, 257, 258] {
            let (stream, outlen) = fixed_block(&prefix, &[(len, dist)]);
            for align in 0..4 {
                compare(
                    &format!("dist{dist}_len{len}_align{align}"),
                    &stream,
                    outlen,
                    align,
                );
                // one byte short of what is needed: error path
                compare(
                    &format!("dist{dist}_len{len}_align{align}_short"),
                    &stream,
                    outlen - 1,
                    align,
                );
            }
        }
    }
}

/// Back-reference reaching before the start of the output buffer must fail
/// identically in both.
#[test]
fn back_reference_underflow() {
    for &dist in &[1u32, 2, 5, 100] {
        for &prefix_len in &[0u32, 1, 3] {
            if prefix_len >= dist {
                continue;
            }
            let prefix: Vec<u8> = (0..prefix_len).map(|i| (i as u8) ^ 0x5A).collect();
            let (stream, outlen) = fixed_block(&prefix, &[(3, dist)]);
            for align in 0..4 {
                compare(
                    &format!("underflow_dist{dist}_pfx{prefix_len}_align{align}"),
                    &stream,
                    outlen + 64,
                    align,
                );
            }
        }
    }
}

/// Every fixed-Huffman literal symbol, including the 9-bit range and the
/// length symbols 280..287.
#[test]
fn all_fixed_literals() {
    let all: Vec<u8> = (0..=255u8).collect();
    let (stream, outlen) = fixed_block(&all, &[]);
    for align in 0..4 {
        compare(&format!("all_literals_align{align}"), &stream, outlen, align);
        compare(
            &format!("all_literals_align{align}_slack"),
            &stream,
            outlen + 128,
            align,
        );
    }
    // long chains of back references
    let prefix: Vec<u8> = (0..64u8).collect();
    let refs: Vec<(u32, u32)> = (0..40).map(|i| (3 + (i % 200), 1 + (i % 64))).collect();
    let (stream, outlen) = fixed_block(&prefix, &refs);
    for align in 0..4 {
        compare(&format!("ref_chain_align{align}"), &stream, outlen, align);
    }
}

/// Multiple non-final blocks in one stream (fixed + stored mixtures are not
/// expressible here because of the `cp_stored` length check, so this uses
/// several fixed blocks).
#[test]
fn multiple_fixed_blocks() {
    for nblocks in 1..6usize {
        let mut w = BitW::new();
        let mut outlen = 0usize;
        for b in 0..nblocks {
            w.bits(if b + 1 == nblocks { 1 } else { 0 }, 1);
            w.bits(1, 2);
            for i in 0..(7 * (b + 1)) {
                w.literal(((i * 13 + b * 5) & 0xFF) as u32);
                outlen += 1;
            }
            if b > 0 {
                w.back_ref(5, 3);
                outlen += 5;
            }
            w.end_of_block();
        }
        for align in 0..4 {
            compare(
                &format!("multiblock{nblocks}_align{align}"),
                &w.bytes,
                outlen,
                align,
            );
        }
    }
}
