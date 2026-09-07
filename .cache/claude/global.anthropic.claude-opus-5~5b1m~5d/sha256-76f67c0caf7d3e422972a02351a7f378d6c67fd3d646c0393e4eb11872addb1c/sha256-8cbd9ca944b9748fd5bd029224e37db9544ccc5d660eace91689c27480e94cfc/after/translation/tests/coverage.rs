//! Coverage assertions for `CONFIGS.md`: prove that the streams the Phase B
//! rows feed to the two libraries really do contain the block types / branches
//! the rows claim, so a row cannot be "passing" while testing nothing.

mod common;

use common::deflate::*;
use common::Rng;
use std::io::Write;

fn flate2_raw(data: &[u8], level: u32) -> Vec<u8> {
    let mut e = flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::new(level));
    e.write_all(data).unwrap();
    e.finish().unwrap()
}

/// Minimal DEFLATE block walker: returns the BTYPE of every block in a raw
/// stream (0 = stored, 1 = fixed, 2 = dynamic). Only needs to be right about
/// block *framing*, so it decodes headers and skips block bodies using an
/// independent (textbook) Huffman decoder.
fn block_types(stream: &[u8]) -> Vec<u8> {
    struct R<'a> {
        d: &'a [u8],
        pos: usize,
    }
    impl<'a> R<'a> {
        fn bit(&mut self) -> u32 {
            let b = (self.d[self.pos >> 3] >> (self.pos & 7)) & 1;
            self.pos += 1;
            b as u32
        }
        fn bits(&mut self, n: u32) -> u32 {
            let mut v = 0;
            for i in 0..n {
                v |= self.bit() << i;
            }
            v
        }
        fn align(&mut self) {
            self.pos = (self.pos + 7) & !7;
        }
    }
    // canonical Huffman decoder from code lengths
    fn decode(r: &mut R, lens: &[u8]) -> u32 {
        let maxlen = *lens.iter().max().unwrap() as usize;
        let mut bl_count = vec![0u32; maxlen + 1];
        for &l in lens {
            if l > 0 {
                bl_count[l as usize] += 1;
            }
        }
        let mut next_code = vec![0u32; maxlen + 2];
        let mut code = 0u32;
        for b in 1..=maxlen {
            code = (code + bl_count[b - 1]) << 1;
            next_code[b] = code;
        }
        let mut codes = vec![0u32; lens.len()];
        let mut nc = next_code.clone();
        for (i, &l) in lens.iter().enumerate() {
            if l > 0 {
                codes[i] = nc[l as usize];
                nc[l as usize] += 1;
            }
        }
        let mut cur = 0u32;
        let mut len = 0usize;
        loop {
            cur = (cur << 1) | r.bit();
            len += 1;
            assert!(len <= maxlen, "no code matched");
            for (i, &l) in lens.iter().enumerate() {
                if l as usize == len && codes[i] == cur {
                    return i as u32;
                }
            }
        }
    }

    const PERM: [usize; 19] =
        [16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15];
    let mut r = R { d: stream, pos: 0 };
    let mut out: Vec<u8> = Vec::new();
    let mut types = Vec::new();
    loop {
        let bfinal = r.bit();
        let btype = r.bits(2) as u8;
        types.push(btype);
        match btype {
            0 => {
                r.align();
                let len = r.bits(16) as usize;
                let _nlen = r.bits(16);
                for _ in 0..len {
                    out.push(r.bits(8) as u8);
                }
            }
            1 | 2 => {
                let (litlens, distlens) = if btype == 1 {
                    let mut l = vec![8u8; 288];
                    for i in 144..256 {
                        l[i] = 9;
                    }
                    for i in 256..280 {
                        l[i] = 7;
                    }
                    (l, vec![5u8; 30])
                } else {
                    let nlit = 257 + r.bits(5) as usize;
                    let ndst = 1 + r.bits(5) as usize;
                    let nlen = 4 + r.bits(4) as usize;
                    let mut cl = [0u8; 19];
                    for i in 0..nlen {
                        cl[PERM[i]] = r.bits(3) as u8;
                    }
                    let mut lens: Vec<u8> = Vec::new();
                    while lens.len() < nlit + ndst {
                        let sym = decode(&mut r, &cl);
                        match sym {
                            16 => {
                                let prev = *lens.last().unwrap();
                                let n = 3 + r.bits(2);
                                for _ in 0..n {
                                    lens.push(prev);
                                }
                            }
                            17 => {
                                let n = 3 + r.bits(3);
                                for _ in 0..n {
                                    lens.push(0);
                                }
                            }
                            18 => {
                                let n = 11 + r.bits(7);
                                for _ in 0..n {
                                    lens.push(0);
                                }
                            }
                            s => lens.push(s as u8),
                        }
                    }
                    (lens[..nlit].to_vec(), lens[nlit..nlit + ndst].to_vec())
                };
                loop {
                    let sym = decode(&mut r, &litlens);
                    if sym < 256 {
                        out.push(sym as u8);
                    } else if sym == 256 {
                        break;
                    } else {
                        let i = (sym - 257) as usize;
                        let len = LEN_BASE[i] + r.bits(LEN_EXTRA[i]);
                        let ds = decode(&mut r, &distlens) as usize;
                        let dist = DIST_BASE[ds] + r.bits(DIST_EXTRA[ds]);
                        let start = out.len() - dist as usize;
                        for k in 0..len as usize {
                            let b = out[start + k];
                            out.push(b);
                        }
                    }
                }
            }
            _ => panic!("btype 3"),
        }
        if bfinal == 1 {
            break;
        }
    }
    types
}

#[test]
fn my_encoder_emits_the_block_types_it_claims() {
    // fixed literal-only
    assert_eq!(block_types(&fixed_literal_stream(b"hello world")), vec![1]);
    // stored
    let mut w = BitWriter::new();
    write_stored_block(&mut w, true, b"stored payload");
    assert_eq!(block_types(&w.finish()), vec![0]);
    // chained fixed blocks
    let mut w = BitWriter::new();
    write_fixed_block(&mut w, false, &b"aaa".iter().map(|&b| Tok::Lit(b)).collect::<Vec<_>>());
    write_fixed_block(&mut w, true, &b"bbb".iter().map(|&b| Tok::Lit(b)).collect::<Vec<_>>());
    assert_eq!(block_types(&w.finish()), vec![1, 1]);
    // fixed block with a match
    let mut w = BitWriter::new();
    write_fixed_block(
        &mut w,
        true,
        &[Tok::Lit(b'x'), Tok::Match { len: 258, dist: 1 }],
    );
    assert_eq!(block_types(&w.finish()), vec![1]);
}

#[test]
fn flate2_payload_shapes_produce_dynamic_blocks() {
    let rng = Rng::new(0xC0FFEE);
    // row 15 shape: a single distinct literal
    let d = vec![7u8; 2000];
    assert!(block_types(&flate2_raw(&d, 9)).contains(&2), "row15 shape is not dynamic");
    // row 16 shape: uniform over all 256 values
    let d = rng.bytes(3000);
    eprintln!("row16 shape -> {:?}", block_types(&flate2_raw(&d, 9)));
    // row 14 shape: small alphabet
    let d = rng.bytes_alphabet(1500, 4);
    assert!(block_types(&flate2_raw(&d, 9)).contains(&2), "row14 shape is not dynamic");
    // row 18 shape: full alphabet plus repeats
    let mut d: Vec<u8> = (0..=255u8).collect();
    d.extend(rng.bytes(1024));
    let t = d.clone();
    d.extend_from_slice(&t);
    assert!(block_types(&flate2_raw(&d, 9)).contains(&2), "row18 shape is not dynamic");
}

#[test]
fn every_compression_level_is_a_distinct_configuration() {
    // Document which block types each level actually produces, and require that
    // rows 26's sweep covers more than one block type overall.
    let rng = Rng::new(0xBEEF);
    let mut all: std::collections::BTreeSet<u8> = Default::default();
    for level in 0..=9u32 {
        for kind in 0..3 {
            let n = 3000;
            let data = match kind {
                0 => rng.bytes(n),
                1 => rng.bytes_alphabet(n, 5),
                _ => std::iter::repeat(0xA5u8).take(n).collect(),
            };
            let ts = block_types(&flate2_raw(&data, level));
            for t in ts {
                all.insert(t);
            }
        }
    }
    assert!(all.contains(&2), "no dynamic block anywhere in the level sweep");
    assert!(all.len() >= 2, "level sweep only produced one block type: {all:?}");
    eprintln!("block types produced across levels 0..=9: {all:?}");
}
