//! Phase C — one generator per `ERRORS.md` row, plus the generic FFI boundary
//! rows and an exhaustive/randomized small-input sweep.

#![allow(dead_code)]

use super::super::deflate::*;
use super::super::{Case, Expect, Rng, Tbl};

pub const IDS: &[&str] = &[
    "c_e1_len_nlen",
    "c_e2_stored_beyond",
    "c_e3_out_symbol",
    "c_e4_back_dist",
    "c_e5_out_string",
    "c_e6_unknown_btype",
    "c_a1_cp_ptr",
    "c_a3_consume_bits",
    "c_a4_read_bits_width",
    "c_a6_bits_left",
    "c_a8_would_overflow",
    "c_a9_build_len",
    "c_a10_decode_key",
    "c_boundaries",
    "c_sweep_tiny",
    "c_sweep_two",
    "c_sweep_random",
];

pub fn build(id: &str) -> Vec<Case> {
    match id {
        "c_e1_len_nlen" => e1(),
        "c_e2_stored_beyond" => e2(),
        "c_e3_out_symbol" => e3(),
        "c_e4_back_dist" => e4(),
        "c_e5_out_string" => e5(),
        "c_e6_unknown_btype" => e6(),
        "c_a1_cp_ptr" => a1(),
        "c_a3_consume_bits" => a3(),
        "c_a4_read_bits_width" => a4(),
        "c_a6_bits_left" => a6(),
        "c_a8_would_overflow" => a8(),
        "c_a9_build_len" => a9(),
        "c_a10_decode_key" => a10(),
        "c_boundaries" => boundaries(),
        "c_sweep_tiny" => sweep_tiny(),
        "c_sweep_two" => sweep_two(),
        "c_sweep_random" => sweep_random(),
        _ => unreachable!(),
    }
}

pub const E1: &str =
    "Failed to find LEN and NLEN as complements within stored (uncompressed) stream.";
pub const E2: &str = "Stored block extends beyond end of input stream.";
pub const E3: &str = "Attempted to overwrite out buffer while outputting a symbol.";
pub const E4: &str = "Attempted to write before out buffer (invalid backwards distance).";
pub const E5: &str = "Attempted to overwrite out buffer while outputting a string.";
pub const E6: &str = "Detected unknown block type within input stream.";

// ---------------------------------------------------------------------------
// E1 — LEN is not the complement of NLEN
// ---------------------------------------------------------------------------

fn e1() -> Vec<Case> {
    let mut cases = Vec::new();
    for (i, (len, nlen)) in
        [(0u16, 0u16), (1, 0), (5, 5), (0xFFFF, 0xFFFF), (0x1234, 0x1234), (7, 0xFFF7)]
            .into_iter()
            .enumerate()
    {
        for align in 0..4usize {
            let fb = first_bytes_for(align);
            let mut rng = Rng::new(0x2000_0000 + i as u64 * 8 + align as u64);
            let data: Vec<u8> = (0..8).map(|_| rng.byte()).collect();
            let mut s = Stream::new();
            s.stored_block_raw(true, len, nlen, &data, false);
            let (input, _, _) = s.finish(fb, true, 0);
            cases.push(
                Case::new(format!("LEN={len:#06x} NLEN={nlen:#06x} fb={fb}"), input, 64)
                    .in_align(align)
                    .preset_reason()
                    .expect(Expect::Ret { ret: 0, reason: Some(E1) }),
            );
        }
    }
    cases
}

// ---------------------------------------------------------------------------
// E2 — the stored block does not extend to the end of the input
// ---------------------------------------------------------------------------

fn e2() -> Vec<Case> {
    let mut cases = Vec::new();
    for (i, (len, datalen)) in
        [(0u16, 8usize), (1, 8), (1, 64), (5, 100), (0, 4), (7, 4096)].into_iter().enumerate()
    {
        for align in 0..4usize {
            let fb = first_bytes_for(align);
            let mut rng = Rng::new(0x2100_0000 + i as u64 * 8 + align as u64);
            let data: Vec<u8> = (0..datalen).map(|_| rng.byte()).collect();
            let mut s = Stream::new();
            s.stored_block_raw(true, len, !len, &data, false);
            let (input, _, _) = s.finish(fb, true, 0);
            // `bits_left/8` after the header is datalen (+ padding) > LEN
            cases.push(
                Case::new(format!("LEN={len} remaining={datalen} fb={fb}"), input, 64)
                    .in_align(align)
                    .preset_reason()
                    .expect(Expect::Ret { ret: 0, reason: Some(E2) }),
            );
        }
    }
    cases
}

// ---------------------------------------------------------------------------
// E3 — a literal does not fit in the out buffer
// ---------------------------------------------------------------------------

fn e3() -> Vec<Case> {
    let t = Tables::default();
    let mut cases = Vec::new();
    for (i, nlit) in [1usize, 2, 10, 300].into_iter().enumerate() {
        let mut rng = Rng::new(0x2200_0000 + i as u64);
        let mut toks: Vec<Tok> = (0..nlit).map(|_| Tok::Lit(rng.byte() as u16)).collect();
        toks.push(Tok::End);
        let mut s = Stream::new();
        s.fixed_block(true, &toks, &t);
        let (input, _, olen) = s.finish(0, false, 0);
        for out_bytes in [0i32, 1, (olen as i32) - 1, -1, -1000] {
            if out_bytes >= olen as i32 {
                continue;
            }
            cases.push(
                Case::new(format!("{nlit} literals, out_bytes={out_bytes}"), input.clone(), out_bytes)
                    .out_pad(olen + 4096)
                    .preset_reason()
                    .expect(Expect::Ret { ret: 0, reason: Some(E3) }),
            );
        }
    }
    // null out pointer, out_bytes = 0  (N3)
    {
        let mut s = Stream::new();
        s.fixed_block(true, &[Tok::Lit(65), Tok::End], &t);
        let (input, _, _) = s.finish(0, false, 0);
        cases.push(
            Case::new("out=NULL out_bytes=0, one literal", input, 0)
                .out_null()
                .preset_reason()
                .expect(Expect::Ret { ret: 0, reason: Some(E3) }),
        );
    }
    cases
}

// ---------------------------------------------------------------------------
// E4 — backwards distance reaches before the start of the out buffer
// ---------------------------------------------------------------------------

fn e4() -> Vec<Case> {
    let t = Tables::default();
    let mut cases = Vec::new();
    // produced bytes, distance symbol, extra  =>  distance > produced
    let combos: &[(usize, u16, u32)] = &[
        (0, 0, 0),   // nothing produced yet, distance 1
        (1, 1, 0),   // distance 2 > 1
        (2, 2, 0),   // distance 3 > 2
        (10, 5, 1),  // distance 7+1 = 8 <= 10 -> filtered out below
        (10, 6, 3),  // distance 9+3 = 12 > 10
        (100, 10, 15), // distance 33+15 = 48 <= 100 -> filtered out below
        (100, 13, 31), // distance 97+31 = 128 > 100
        (1, 29, 8191), // distance 32768 > 1
    ];
    for (i, &(produced, ds, dx)) in combos.iter().enumerate() {
        let dist = t.distance_of(ds, dx) as usize;
        if dist <= produced {
            continue;
        }
        let mut rng = Rng::new(0x2300_0000 + i as u64);
        let mut toks: Vec<Tok> = (0..produced).map(|_| Tok::Lit(rng.byte() as u16)).collect();
        toks.push(Tok::Match { ls: 257, lx: 0, ds, dx });
        toks.push(Tok::End);
        let mut s = Stream::new();
        s.fixed_block(true, &toks, &t);
        let (input, _, _) = s.finish(0, false, 0);
        cases.push(
            Case::new(format!("produced={produced} dist={dist}"), input, (produced + 300) as i32)
                .out_pad(produced + 8192)
                .preset_reason()
                .expect(Expect::Ret { ret: 0, reason: Some(E4) }),
        );
    }
    cases
}

// ---------------------------------------------------------------------------
// E5 — the copied string does not fit in the out buffer
// ---------------------------------------------------------------------------

fn e5() -> Vec<Case> {
    let t = Tables::default();
    let mut cases = Vec::new();
    // (prefix literals, length symbol, length extra, distance symbol)
    let combos: &[(usize, u16, u32, u16)] = &[
        (1, 257, 0, 0),   // len 3, dist 1
        (4, 285, 0, 0),   // len 258, dist 1
        (4, 269, 3, 1),   // len 19+3 = 22, dist 2
        (20, 280, 15, 3), // len 115+15 = 130, dist 4
    ];
    for (i, &(prefix, ls, lx, ds)) in combos.iter().enumerate() {
        let length = t.length_of(ls, lx) as usize;
        let mut rng = Rng::new(0x2400_0000 + i as u64);
        let mut toks: Vec<Tok> = (0..prefix).map(|_| Tok::Lit(rng.byte() as u16)).collect();
        toks.push(Tok::Match { ls, lx, ds, dx: 0 });
        toks.push(Tok::End);
        let mut s = Stream::new();
        s.fixed_block(true, &toks, &t);
        let (input, _, _) = s.finish(0, false, 0);
        // one byte short of the exact fit, and much shorter
        for out_bytes in [(prefix + length - 1) as i32, prefix as i32] {
            cases.push(
                Case::new(
                    format!("prefix={prefix} len={length} out_bytes={out_bytes}"),
                    input.clone(),
                    out_bytes,
                )
                .out_pad(prefix + length + 4096)
                .preset_reason()
                .expect(Expect::Ret { ret: 0, reason: Some(E5) }),
            );
        }
    }
    cases
}

// ---------------------------------------------------------------------------
// E6 — reserved block type 3
// ---------------------------------------------------------------------------

fn e6() -> Vec<Case> {
    let mut cases = Vec::new();
    for align in 0..4usize {
        for nbytes in [1usize, 2, 4, 5, 8] {
            let fb = first_bytes_for(align);
            let mut s = Stream::new();
            s.raw_bits(1, 1); // bfinal
            s.raw_bits(3, 2); // btype = 3
            let (mut input, _, _) = s.finish(fb, false, 0);
            while input.len() < nbytes {
                input.push(0);
            }
            cases.push(
                Case::new(format!("btype=3 fb={fb} in_bytes={nbytes}"), input, 64)
                    .in_align(align)
                    .preset_reason()
                    .expect(Expect::Ret { ret: 0, reason: Some(E6) }),
            );
        }
    }
    // btype 3 as the second block
    {
        let t = Tables::default();
        let mut s = Stream::new();
        s.fixed_block(false, &[Tok::Lit(1), Tok::Lit(2), Tok::End], &t);
        s.raw_bits(1, 1);
        s.raw_bits(3, 2);
        let (input, _, _) = s.finish(0, false, 0);
        cases.push(
            Case::new("fixed then btype=3", input, 64)
                .preset_reason()
                .expect(Expect::Ret { ret: 0, reason: Some(E6) }),
        );
    }
    cases
}

// ---------------------------------------------------------------------------
// A1 — cp_ptr: !(s->bits_left & 7)
//
// Reachable when `cp_peak_bits` folds the final partial word at a bit position
// that is not a multiple of 8: from then on `bits_left` and the real byte
// position disagree, and the stored-block path asserts.
// ---------------------------------------------------------------------------

fn a1() -> Vec<Case> {
    let lit = canonical(&fixed_lit_lens());
    let ll = fixed_lit_lens();
    let mut cases = Vec::new();

    // 6 bytes, 4-byte aligned: word 0 is loaded, then the 2-byte final word is
    // folded after 19 bits (not a multiple of 8), and the following stored block
    // reaches cp_ptr with bits_left % 8 == 3.
    for lit_sym in [0u16, 1, 100] {
        let mut s = Stream::new();
        s.raw_bits(0, 1); // bfinal = 0
        s.raw_bits(1, 2); // btype = 1 (fixed)
        s.bw.huff(lit[lit_sym as usize], ll[lit_sym as usize] as usize);
        s.bw.huff(lit[lit_sym as usize], ll[lit_sym as usize] as usize);
        s.bw.huff(lit[256], ll[256] as usize); // end of block (7 bits)
        s.raw_bits(1, 1); // bfinal = 1
        s.raw_bits(0, 2); // btype = 0 (stored)
        s.raw_bits(0xFFFF, 16); // LEN  = 0xFFFF
        s.raw_bits(0, 3); // NLEN low bits; the rest reads as 0 => ~NLEN == LEN
        let (input, _, _) = s.finish(0, false, 0);
        assert_eq!(input.len(), 6, "hand-built A1 stream must be 6 bytes");
        cases.push(
            Case::new(format!("fold at bit 19, lit={lit_sym}"), input, 2)
                .in_pad(8192)
                .out_pad(8192)
                .expect(Expect::Assert {
                    line: 95,
                    func: "cp_ptr",
                    expr: "!(s->bits_left & 7)",
                }),
        );
    }
    cases
}

// ---------------------------------------------------------------------------
// A3 — cp_consume_bits: s->count >= num_bits_to_read
//
// `cp_decode` consumes the code length without any bits_left check, so a stream
// that ends in the middle of a Huffman code aborts here.
// ---------------------------------------------------------------------------

fn a3() -> Vec<Case> {
    let t = Tables::default();
    let mut cases = Vec::new();
    for align in 0..4usize {
        for nlit in [1usize, 5, 40] {
            let fb = first_bytes_for(align);
            let mut rng = Rng::new(0x2600_0000 + align as u64 * 16 + nlit as u64);
            // Literals, no end-of-block, then all-ones filler: 0b111111111 is a
            // valid 9-bit code (symbol 255), so the decoder keeps consuming until
            // the bits run out.  Two things must be avoided:
            //   * zero bits at the end — 7 zero bits decode as end-of-block, so
            //     every padding bit is a 1;
            //   * `last_bytes != 0` — folding the final partial word inflates
            //     `count` by `bits_left`, which supplies phantom zero bits (and
            //     therefore an end-of-block) beyond the real data.
            let toks: Vec<Tok> = (0..nlit).map(|_| Tok::Lit(rng.byte() as u16)).collect();
            let mut s = Stream::new();
            s.fixed_block(true, &toks, &t);
            while s.bw.nbits % 8 != 0 {
                s.raw_bits(1, 1);
            }
            let (mut input, _, _) = s.finish(fb, false, 0);
            for _ in 0..4 {
                input.push(0xFF);
            }
            while (input.len() + 4 - fb) % 4 != 0 {
                input.push(0xFF);
            }
            let n = input.len();
            cases.push(
                Case::new(format!("truncated fb={fb} nlit={nlit}"), input, (n * 2 + 64) as i32)
                    .in_align(align)
                    .out_pad(n * 2 + 4096)
                    .expect(Expect::Assert {
                        line: 115,
                        func: "cp_consume_bits",
                        expr: "s->count >= num_bits_to_read",
                    }),
            );
        }
    }
    cases
}

// ---------------------------------------------------------------------------
// A4 — cp_read_bits: num_bits_to_read <= 32  (and A5's `>= 0`, same argument)
//
// Only reachable through the writable extra-bit tables.
// ---------------------------------------------------------------------------

fn a4() -> Vec<Case> {
    let t = Tables::default();
    let mut cases = Vec::new();
    let mut rng = Rng::new(0x2700_0000);
    let mut toks: Vec<Tok> = (0..8).map(|_| Tok::Lit(rng.byte() as u16)).collect();
    toks.push(Tok::Match { ls: 257, lx: 0, ds: 0, dx: 0 });
    toks.push(Tok::End);
    let mut s = Stream::new();
    s.fixed_block(true, &toks, &t);
    let (input, _, olen) = s.finish(0, false, 0);

    for val in [33u32, 64, 127, 255] {
        cases.push(
            Case::new(format!("cp_len_extra_bits[0]={val}"), input.clone(), (olen + 64) as i32)
                .out_pad(olen + 4096)
                .patch(Tbl::LenExtra, 0, val)
                .expect(Expect::Assert {
                    line: 123,
                    func: "cp_read_bits",
                    expr: "num_bits_to_read <= 32",
                }),
        );
        cases.push(
            Case::new(format!("cp_dist_extra_bits[0]={val}"), input.clone(), (olen + 64) as i32)
                .out_pad(olen + 4096)
                .patch(Tbl::DistExtra, 0, val)
                .expect(Expect::Assert {
                    line: 123,
                    func: "cp_read_bits",
                    expr: "num_bits_to_read <= 32",
                }),
        );
    }
    cases
}

// ---------------------------------------------------------------------------
// A6 — cp_read_bits: s->bits_left > 0
// ---------------------------------------------------------------------------

fn a6() -> Vec<Case> {
    let assert6 =
        Expect::Assert { line: 125, func: "cp_read_bits", expr: "s->bits_left > 0" };
    let mut cases = Vec::new();

    // in_bytes == 0 at every alignment
    for align in 0..4usize {
        cases.push(
            Case::new(format!("in_bytes=0 align={align}"), vec![], 64)
                .in_align(align)
                .in_pad(8192)
                .expect(assert6.clone()),
        );
    }
    // NULL input, in_bytes == 0
    cases.push(Case::new("in=NULL in_bytes=0", vec![], 64).in_null().expect(assert6.clone()));

    // negative / wrapping in_bytes: `bits_left = in_bytes * 8` is <= 0, so the
    // very first cp_read_bits aborts — *unless*
    // `last_bytes = (in_bytes - first_bytes) & 3` is non-zero, in which case
    // `pinflate`'s final-word loop first reads `in[in_bytes-last_bytes ..]`,
    // which for a large |in_bytes| is a wild address and faults instead.
    for (name, n) in [
        ("in_bytes=-1", -1i32),
        ("in_bytes=-4", -4),
        ("in_bytes=-8", -8),
        ("in_bytes=-3", -3),
        ("in_bytes=INT_MIN", i32::MIN),
        ("in_bytes=0x20000000", 0x2000_0000),
        ("in_bytes=0x40000000", 0x4000_0000),
    ] {
        let mut rng = Rng::new(0x2800_0000 + (n as u32) as u64);
        let data: Vec<u8> = (0..16).map(|_| rng.byte()).collect();
        for align in 0..4usize {
            let fb = first_bytes_for(align) as i32;
            let last_bytes = n.wrapping_sub(fb) & 3;
            let first_read = n.wrapping_sub(last_bytes);
            // 8 KiB of padding on both sides keeps modest out-of-range reads
            // inside a real allocation
            let in_range = last_bytes == 0 || (first_read.unsigned_abs() as usize) < 4096;
            let e = if in_range { assert6.clone() } else { Expect::Signal(11) };
            cases.push(
                Case::new(format!("{name} align={align} last_bytes={last_bytes}"), data.clone(), 64)
                    .in_bytes(n)
                    .in_align(align)
                    .in_pad(8192)
                    .expect(e),
            );
        }
    }

    // a 1-byte stored-block header exhausts the stream inside cp_stored
    for align in 0..4usize {
        let fb = first_bytes_for(align);
        let mut s = Stream::new();
        s.raw_bits(0, 1);
        s.raw_bits(0, 2);
        let (input, _, _) = s.finish(fb, false, 0);
        assert_eq!(input.len(), 1);
        cases.push(
            Case::new(format!("1-byte stored header align={align}"), input, 64)
                .in_align(align)
                .in_pad(8192)
                .expect(assert6.clone()),
        );
    }
    cases
}

// ---------------------------------------------------------------------------
// A8 — cp_read_bits: !cp_would_overflow(s, num_bits_to_read)
//
// 6 bytes, 4-byte aligned, btype = 2, HCLEN >= 11: the 11th 3-bit code-length
// read happens with bits_left == count == 1.
// ---------------------------------------------------------------------------

fn a8() -> Vec<Case> {
    let mut cases = Vec::new();
    for hclen_field in [7u32, 8, 15] {
        for (hlit_field, hdist_field) in [(0u32, 0u32), (31, 31), (5, 9)] {
            let mut s = Stream::new();
            s.raw_bits(1, 1); // bfinal
            s.raw_bits(2, 2); // btype = 2 (dynamic)
            s.raw_bits(hlit_field, 5);
            s.raw_bits(hdist_field, 5);
            s.raw_bits(hclen_field, 4); // HCLEN = 4 + field >= 11
            for i in 0..10 {
                s.raw_bits((i % 8) as u32, 3);
            }
            let (input, _, _) = s.finish(0, false, 0);
            assert_eq!(input.len(), 6, "hand-built A8 stream must be 6 bytes");
            cases.push(
                Case::new(
                    format!("hclen={} hlit={hlit_field} hdist={hdist_field}", 4 + hclen_field),
                    input,
                    4096,
                )
                .in_pad(8192)
                .out_pad(8192)
                .expect(Expect::Assert {
                    line: 127,
                    func: "cp_read_bits",
                    expr: "!cp_would_overflow(s, num_bits_to_read)",
                }),
            );
        }
    }
    cases
}

// ---------------------------------------------------------------------------
// A9 — cp_build: len < 16   (via the writable cp_fixed_table)
// ---------------------------------------------------------------------------

fn a9() -> Vec<Case> {
    let t = Tables::default();
    let mut rng = Rng::new(0x2900_0000);
    let mut toks: Vec<Tok> = (0..8).map(|_| Tok::Lit(rng.byte() as u16)).collect();
    toks.push(Tok::End);
    let mut s = Stream::new();
    s.fixed_block(true, &toks, &t);
    let (input, _, olen) = s.finish(0, false, 0);

    let mut cases = Vec::new();
    for idx in [0usize, 1, 143, 287, 288, 300, 319] {
        for val in [16u32, 17, 56, 255] {
            cases.push(
                Case::new(
                    format!("cp_fixed_table[{idx}]={val}"),
                    input.clone(),
                    (olen + 64) as i32,
                )
                .out_pad(olen + 4096)
                .patch(Tbl::Fixed, idx, val)
                .expect(Expect::Assert { line: 154, func: "cp_build", expr: "len < 16" }),
            );
        }
    }
    cases
}

// ---------------------------------------------------------------------------
// A10 — cp_decode: (search >> len) == (key >> len)
// ---------------------------------------------------------------------------

fn a10() -> Vec<Case> {
    let t = Tables::default();
    let expect = Expect::Assert {
        line: 217,
        func: "cp_decode",
        expr: "(search >> len) == (key >> len)",
    };
    let mut cases = Vec::new();

    // (a) every code-length code length is 0  =>  nlen == 0  =>  tree[-1] read
    for hclen_field in [0u32, 3, 15] {
        for nbytes in [4usize, 8, 16] {
            let nlen = 4 + hclen_field as usize;
            let mut s = Stream::new();
            s.raw_bits(1, 1);
            s.raw_bits(2, 2);
            s.raw_bits(0, 5);
            s.raw_bits(0, 5);
            s.raw_bits(hclen_field, 4);
            for _ in 0..nlen {
                s.raw_bits(0, 3);
            }
            let (mut input, _, _) = s.finish(0, false, 0);
            while input.len() < nbytes {
                input.push(0);
            }
            if input.len() > nbytes {
                continue;
            }
            cases.push(
                Case::new(format!("nlen=0 hclen={nlen} in_bytes={nbytes}"), input, 4096)
                    .in_pad(8192)
                    .out_pad(8192)
                    .expect(expect.clone()),
            );
        }
    }

    // (b) a valid code-length tree, but every literal/distance length is 0
    //     =>  nlit == 0  =>  cp_block's first cp_decode reads lit[-1]
    {
        let mut cl_lens = [0u8; 19];
        cl_lens[0] = 1;
        cl_lens[18] = 1;
        let spec = DynSpec {
            hlit: 257,
            hdist: 1,
            hclen: hclen_for(&cl_lens, &PERM),
            cl_lens,
            ops: vec![ClOp::Rep18(127), ClOp::Rep18(109)],
            lit_lens: vec![0u8; 257],
            dist_lens: vec![0u8; 1],
        };
        assert_eq!(expand_ops(&spec.ops).len(), 258);
        for pad in [0usize, 1, 2, 3] {
            let mut s = Stream::new();
            s.dynamic_block(true, &spec, &[], &t);
            let (mut input, _, _) = s.finish(0, false, 0);
            for _ in 0..pad {
                input.push(0);
            }
            cases.push(
                Case::new(format!("nlit=0 pad={pad}"), input, 4096)
                    .in_pad(8192)
                    .out_pad(8192)
                    .expect(expect.clone()),
            );
        }
    }
    cases
}

// ---------------------------------------------------------------------------
// N1-N7 — generic FFI boundaries
// ---------------------------------------------------------------------------

fn boundaries() -> Vec<Case> {
    let t = Tables::default();
    let mut cases = Vec::new();

    // N2: out == NULL, out_bytes == 0, empty block => success
    {
        let mut s = Stream::new();
        s.fixed_block(true, &[Tok::End], &t);
        let (input, _, _) = s.finish(0, false, 0);
        cases.push(
            Case::new("out=NULL, empty fixed block", input.clone(), 0)
                .out_null()
                .expect(Expect::Ret { ret: 1, reason: None }),
        );
        // and a stored block with LEN == 0
        let mut s2 = Stream::new();
        s2.stored_block(true, &[]);
        let (input2, _, olen2) = s2.finish(1, false, 0);
        assert_eq!(olen2, 0);
        cases.push(
            Case::new("out=NULL, stored LEN=0", input2, 0)
                .in_align(3)
                .out_null()
                .expect(Expect::Ret { ret: 1, reason: None }),
        );
    }

    // N4: negative out_bytes with a match (E5) and with a literal (E3)
    {
        let mut rng = Rng::new(0x2A00_0000);
        let mut toks: Vec<Tok> = (0..4).map(|_| Tok::Lit(rng.byte() as u16)).collect();
        toks.push(Tok::Match { ls: 257, lx: 0, ds: 0, dx: 0 });
        toks.push(Tok::End);
        let mut s = Stream::new();
        s.fixed_block(true, &toks, &t);
        let (input, _, olen) = s.finish(0, false, 0);
        for ob in [-1i32, -1000, i32::MIN] {
            cases.push(
                Case::new(format!("out_bytes={ob} literal first"), input.clone(), ob)
                    .out_pad(olen + 4096)
                    .preset_reason()
                    .expect(Expect::Ret { ret: 0, reason: Some(E3) }),
            );
        }
        // a stored block ignores out_end entirely, so a negative out_bytes still
        // copies LEN bytes and returns 1
        let mut s2 = Stream::new();
        let data: Vec<u8> = (0..32).map(|_| rng.byte()).collect();
        s2.stored_block(true, &data);
        let (input2, _, olen2) = s2.finish(0, true, 0);
        for ob in [-1i32, -1000] {
            cases.push(
                Case::new(format!("stored, out_bytes={ob}"), input2.clone(), ob)
                    .out_pad(olen2 + 8192)
                    .expect(Expect::Ret { ret: 1, reason: None }),
            );
        }
    }

    // N6: in_bytes = INT_MAX  =>  last_bytes = 3  =>  wild read  =>  SIGSEGV
    {
        let mut rng = Rng::new(0x2B00_0000);
        let data: Vec<u8> = (0..16).map(|_| rng.byte()).collect();
        cases.push(
            Case::new("in_bytes=INT_MAX", data.clone(), 64)
                .in_bytes(i32::MAX)
                .expect(Expect::Signal(11)),
        );
        cases.push(
            Case::new("in_bytes=INT_MAX-1", data, 64)
                .in_bytes(i32::MAX - 1)
                .expect(Expect::Signal(11)),
        );
    }

    // N7: huge out_bytes with a short valid stream
    {
        let mut rng = Rng::new(0x2C00_0000);
        let (toks, _) = super::random_fixed_stream(&mut rng, 20);
        let mut s = Stream::new();
        s.fixed_block(true, &toks, &t);
        let (input, _, olen) = s.finish(0, false, 0);
        for ob in [i32::MAX, i32::MAX - 1, 1 << 30] {
            cases.push(
                Case::new(format!("out_bytes={ob}"), input.clone(), ob)
                    .out_pad(olen + 4096)
                    .expect(Expect::Ret { ret: 1, reason: None }),
            );
        }
    }

    // in_bytes shorter than the real buffer (truncation at every offset)
    {
        let mut rng = Rng::new(0x2D00_0000);
        let (toks, _) = super::random_fixed_stream(&mut rng, 30);
        let mut s = Stream::new();
        s.fixed_block(true, &toks, &t);
        let (input, _, olen) = s.finish(0, false, 0);
        for k in 1..input.len() {
            cases.push(
                Case::new(format!("in_bytes={k} of {}", input.len()), input.clone(), (olen + 64) as i32)
                    .in_bytes(k as i32)
                    .in_pad(8192)
                    .out_pad(olen + 8192)
                    .expect(Expect::Any),
            );
        }
    }
    cases
}

// ---------------------------------------------------------------------------
// exhaustive tiny inputs
// ---------------------------------------------------------------------------

fn sweep_tiny() -> Vec<Case> {
    let mut cases = Vec::new();
    for align in 0..4usize {
        for b in 0..256u16 {
            cases.push(
                // a 1-byte input can never reach cp_stored's memcpy (the
                // LEN/NLEN reads abort first), so modest padding is enough
                Case::new(format!("1 byte {b:#04x} align={align}"), vec![b as u8], 4096)
                    .in_align(align)
                    .in_pad(8192)
                    .out_pad(8192)
                    .expect(Expect::Any),
            );
        }
    }
    cases
}

/// Every 2-byte input, at `first_bytes == 0`.
///
/// The full 65536-case sweep takes ~4 minutes; by default only every 8th value
/// is used.  Set `PINFLATE_FULL_SWEEP=1` for the exhaustive run (both the driver
/// and the workers read the same variable, so the case lists stay identical).
fn sweep_two() -> Vec<Case> {
    let full = std::env::var("PINFLATE_FULL_SWEEP").is_ok();
    let stride = if full { 1u32 } else { 8 };
    let mut cases = Vec::with_capacity(1 << 16);
    for v in (0..(1u32 << 16)).step_by(stride as usize) {
        cases.push(
            Case::new(
                format!("2 bytes {v:#06x}"),
                vec![(v & 0xFF) as u8, (v >> 8) as u8],
                4096,
            )
            .in_pad(8192)
            .out_pad(8192)
            .expect(Expect::Any),
        );
    }
    cases
}

fn sweep_random() -> Vec<Case> {
    let mut cases = Vec::new();
    let mut rng = Rng::new(0x2E00_0000);
    for n in [2usize, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 16, 20, 24, 32, 48, 64, 96] {
        for _ in 0..160 {
            let data: Vec<u8> = (0..n).map(|_| rng.byte()).collect();
            let align = rng.below(4);
            let out_bytes = [0i32, 1, 64, 4096][rng.below(4)];
            cases.push(
                Case::new(format!("{n} random bytes align={align} out={out_bytes}"), data, out_bytes)
                    .in_align(align)
                    .in_pad(70_000)
                    .out_pad(70_000)
                    .expect(Expect::Any),
            );
        }
    }
    cases
}
