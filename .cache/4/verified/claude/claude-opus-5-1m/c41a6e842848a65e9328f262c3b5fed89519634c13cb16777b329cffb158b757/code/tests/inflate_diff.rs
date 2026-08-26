//! Phase B — valid-path differential tests for `cp_inflate`
//! (rows I1…I24 and I30 of `CONFIGS.md`).

mod common;

use common::deflate::*;
use common::{expect_output, InflateHarness, Rng};
use flate2::write::DeflateEncoder;
use flate2::Compression;
use std::io::Write;

fn raw_deflate(data: &[u8], level: u32) -> Vec<u8> {
    let mut e = DeflateEncoder::new(Vec::new(), Compression::new(level));
    e.write_all(data).unwrap();
    e.finish().unwrap()
}

// ---------------------------------------------------------------------------
// I1 / I2 — stored blocks
// ---------------------------------------------------------------------------

/// I1 — `BTYPE=0`, `BFINAL=1`, `LEN == in_bytes - 5`: the only stored shape the
/// C code accepts (`bits_left/8 <= LEN`).
#[test]
fn i1_stored_block() {
    let h = InflateHarness::new("i1", 1 << 16, 1 << 16);
    let mut rng = Rng::new(0x3001);
    for it in 0..250 {
        let len = match it {
            0 => 3,
            1 => 4,
            2 => 5,
            _ => rng.range(3, 2000) as usize,
        };
        let payload = rng.bytes(len);
        let mut bw = BitWriter::new();
        emit_stored_block(&mut bw, true, &payload, None);
        let stream = bw.finish();
        assert_eq!(stream.len(), len + 5);
        let ctx = format!("I1 len={len} #{it}");
        let o = h.call(&ctx, &stream, 0, len as i32);
        expect_output(&ctx, &o, &payload, len as i32);
    }
}

/// I2 — stored blocks with `LEN` 0/1/2, where the final-partial-word load makes
/// `cp_ptr()` return a *desynchronised* source pointer. Purely a
/// C-vs-Rust comparison: the C behaviour here is a quirk, not a spec.
#[test]
fn i2_stored_tiny_len() {
    let h = InflateHarness::new("i2", 1 << 16, 1 << 16);
    let mut rng = Rng::new(0x3002);
    for len in 0..=6usize {
        for it in 0..20 {
            let payload = rng.bytes(len);
            let mut bw = BitWriter::new();
            emit_stored_block(&mut bw, true, &payload, None);
            let stream = bw.finish();
            h.call(&format!("I2 len={len} #{it}"), &stream, 0, 64);
        }
    }
}

// ---------------------------------------------------------------------------
// I3…I8, I13, I14, I30 — static (fixed) blocks
// ---------------------------------------------------------------------------

/// I3 — literals only, random values (covers the 8- and 9-bit code classes).
#[test]
fn i3_fixed_literals() {
    let h = InflateHarness::new("i3", 1 << 14, 1 << 14);
    let mut rng = Rng::new(0x3003);
    for it in 0..250 {
        let n = rng.range(1, 500) as usize;
        let payload = rng.bytes(n);
        let items: Vec<Item> = payload.iter().map(|&b| Item::Lit(b as u16)).collect();
        let mut bw = BitWriter::new();
        emit_fixed_block(&mut bw, true, &items);
        let stream = bw.finish();
        let ctx = format!("I3 n={n} #{it}");
        let o = h.call(&ctx, &stream, 0, n as i32);
        expect_output(&ctx, &o, &payload, n as i32);
    }
}

/// I4 — completely empty static block (only the end-of-block symbol).
#[test]
fn i4_fixed_empty() {
    let h = InflateHarness::new("i4", 1 << 12, 1 << 12);
    for out_bytes in [0i32, 1, 16] {
        let mut bw = BitWriter::new();
        emit_fixed_block(&mut bw, true, &[]);
        let stream = bw.finish();
        let ctx = format!("I4 out_bytes={out_bytes}");
        let o = h.call(&ctx, &stream, 0, out_bytes);
        expect_output(&ctx, &o, &[], out_bytes);
    }
}

/// I5 — `backwards_distance == 1`: the `memset` fast path, every length 3…258.
#[test]
fn i5_fixed_dist1_memset() {
    let h = InflateHarness::new("i5", 1 << 14, 1 << 14);
    let mut rng = Rng::new(0x3005);
    for length in 3..=258u32 {
        let seed = rng.u8();
        let items = vec![Item::Lit(seed as u16), Item::Match(length, 1)];
        let expected = expected_output(&items);
        let mut bw = BitWriter::new();
        emit_fixed_block(&mut bw, true, &items);
        let stream = bw.finish();
        let ctx = format!("I5 length={length}");
        let o = h.call(&ctx, &stream, 0, expected.len() as i32);
        expect_output(&ctx, &o, &expected, expected.len() as i32);
        assert!(expected.iter().all(|&b| b == seed));
    }
}

/// I6 — `backwards_distance > 1`, overlapping (`distance < length`) and
/// non-overlapping copies.
#[test]
fn i6_fixed_dist_gt1() {
    let h = InflateHarness::new("i6", 1 << 14, 1 << 14);
    let mut rng = Rng::new(0x3006);
    for it in 0..400 {
        let pre = rng.range(2, 64) as usize;
        let dist = rng.range(2, pre as i32) as u32;
        let length = rng.range(3, 258) as u32;
        let mut items: Vec<Item> = (0..pre).map(|_| Item::Lit(rng.u8() as u16)).collect();
        items.push(Item::Match(length, dist));
        // a second, overlapping match on top of the first
        if rng.below(2) == 0 {
            let d2 = rng.range(1, (pre as u32 + length) as i32) as u32;
            items.push(Item::Match(rng.range(3, 100) as u32, d2));
        }
        let expected = expected_output(&items);
        let mut bw = BitWriter::new();
        emit_fixed_block(&mut bw, true, &items);
        let stream = bw.finish();
        let ctx = format!("I6 pre={pre} dist={dist} length={length} #{it}");
        let o = h.call(&ctx, &stream, 0, expected.len() as i32);
        expect_output(&ctx, &o, &expected, expected.len() as i32);
    }
}

/// I7 — every length symbol 257…285 combined with every distance symbol 0…29,
/// i.e. all extra-bit widths (0…5 for lengths, 0…13 for distances).
#[test]
fn i7_fixed_all_len_dist_symbols() {
    let h = InflateHarness::new("i7", 1 << 16, 40960 + 4096);
    let mut rng = Rng::new(0x3007);
    for lsym in 0..29usize {
        for dsym in 0..30usize {
            let length = LEN_BASE[lsym] + rng.below(1 << LEN_EXTRA[lsym]);
            let dist = DIST_BASE[dsym] + rng.below(1 << DIST_EXTRA[dsym]);
            if dist as usize + length as usize + 16 > 40960 {
                continue;
            }
            let mut items: Vec<Item> = Vec::new();
            for i in 0..dist {
                items.push(Item::Lit((i % 251) as u16));
            }
            items.push(Item::Match(length, dist));
            let expected = expected_output(&items);
            let mut bw = BitWriter::new();
            emit_fixed_block(&mut bw, true, &items);
            let stream = bw.finish();
            let ctx = format!("I7 lsym={} dsym={dsym} len={length} dist={dist}", lsym + 257);
            let o = h.call(&ctx, &stream, 0, expected.len() as i32);
            expect_output(&ctx, &o, &expected, expected.len() as i32);
        }
    }
}

/// I8 — every literal value 0…255 (8-bit and 9-bit fixed codes) plus the
/// length symbols 280…287 region of the table.
#[test]
fn i8_fixed_all_literals() {
    let h = InflateHarness::new("i8", 1 << 14, 1 << 14);
    let payload: Vec<u8> = (0..=255u8).collect();
    let items: Vec<Item> = payload.iter().map(|&b| Item::Lit(b as u16)).collect();
    let mut bw = BitWriter::new();
    emit_fixed_block(&mut bw, true, &items);
    let stream = bw.finish();
    let o = h.call("I8 all literals", &stream, 0, 256);
    expect_output("I8 all literals", &o, &payload, 256);

    // length symbols 280..285 use the 8-bit code block at the end of the table
    for lsym in 280..=285usize {
        let length = LEN_BASE[lsym - 257];
        let dist = 3u32;
        let mut items: Vec<Item> = (0..dist).map(|i| Item::Lit(i as u16 + 65)).collect();
        items.push(Item::Match(length, dist));
        let expected = expected_output(&items);
        let mut bw = BitWriter::new();
        emit_fixed_block(&mut bw, true, &items);
        let stream = bw.finish();
        let ctx = format!("I8 lsym={lsym}");
        let o = h.call(&ctx, &stream, 0, expected.len() as i32);
        expect_output(&ctx, &o, &expected, expected.len() as i32);
    }
}

/// I13 — three chained static blocks (`BFINAL` 0,0,1).
#[test]
fn i13_multi_fixed_blocks() {
    let h = InflateHarness::new("i13", 1 << 14, 1 << 14);
    let mut rng = Rng::new(0x3013);
    for it in 0..150 {
        let mut all: Vec<u8> = Vec::new();
        let mut bw = BitWriter::new();
        let nblocks = rng.range(2, 5) as usize;
        for b in 0..nblocks {
            let n = rng.range(0, 40) as usize;
            let items: Vec<Item> = (0..n)
                .map(|_| {
                    let v = rng.u8();
                    Item::Lit(v as u16)
                })
                .collect();
            all.extend(expected_output(&items));
            emit_fixed_block(&mut bw, b + 1 == nblocks, &items);
        }
        let stream = bw.finish();
        let ctx = format!("I13 nblocks={nblocks} #{it}");
        let o = h.call(&ctx, &stream, 0, all.len() as i32);
        expect_output(&ctx, &o, &all, all.len() as i32);
    }
}

/// I30 — payload lengths 0…64 so that the static block ends at every possible
/// phase relative to the 32-bit word loads of `cp_peak_bits`.
#[test]
fn i30_word_boundary_sweep() {
    let h = InflateHarness::new("i30", 1 << 14, 1 << 14);
    let mut rng = Rng::new(0x3030);
    for n in 0..=64usize {
        for align in 0..4usize {
            let payload = rng.bytes(n);
            let items: Vec<Item> = payload.iter().map(|&b| Item::Lit(b as u16)).collect();
            let mut bw = BitWriter::new();
            emit_fixed_block(&mut bw, true, &items);
            let stream = bw.finish();
            let ctx = format!("I30 n={n} align={align}");
            let o = h.call(&ctx, &stream, align, n as i32);
            expect_output(&ctx, &o, &payload, n as i32);
        }
    }
}

// ---------------------------------------------------------------------------
// I9…I12, I14…I16 — dynamic blocks
// ---------------------------------------------------------------------------

/// Random literal alphabet (always including 256) for a dynamic block.
fn random_lit_alphabet(rng: &mut Rng, nlit: usize, k: usize) -> Vec<usize> {
    let mut used = vec![256usize];
    while used.len() < k {
        let s = rng.below(nlit.min(256) as u32) as usize;
        if !used.contains(&s) {
            used.push(s);
        }
    }
    used.sort_unstable();
    used
}

/// I9 — dynamic block, literals only, randomized `HLIT`/`HDIST`/`HCLEN`.
#[test]
fn i9_dynamic_literals() {
    let h = InflateHarness::new("i9", 1 << 15, 1 << 14);
    let mut rng = Rng::new(0x3009);
    for it in 0..200 {
        let nlit = rng.range(257, 288) as usize;
        let ndst = rng.range(1, 32) as usize;
        let k = rng.range(2, 20) as usize;
        let used = random_lit_alphabet(&mut rng, nlit, k);
        let litlens = lengths_for(nlit, &used);
        let dstlens = lengths_for(ndst, &[0]);
        let n = rng.range(0, 200) as usize;
        let items: Vec<Item> = (0..n)
            .map(|_| {
                let mut s = used[rng.below(used.len() as u32) as usize];
                if s == 256 {
                    s = used[0];
                }
                Item::Lit(s as u16)
            })
            .filter(|it| matches!(it, Item::Lit(v) if *v != 256))
            .collect();
        let mut bw = BitWriter::new();
        let (lit, dst) = emit_dynamic_header(
            &mut bw,
            true,
            &litlens,
            &dstlens,
            ClMode::Literal,
            None,
        );
        emit_items(&mut bw, &lit, &dst, &items);
        let stream = bw.finish();
        let expected = expected_output(&items);
        let ctx = format!("I9 nlit={nlit} ndst={ndst} k={k} n={n} #{it}");
        let o = h.call(&ctx, &stream, 0, expected.len() as i32);
        expect_output(&ctx, &o, &expected, expected.len() as i32);
    }
}

/// I10 — dynamic block whose code-length stream really uses symbols 16, 17
/// **and** 18.
#[test]
fn i10_dynamic_cl_repeats() {
    let h = InflateHarness::new("i10", 1 << 15, 1 << 14);
    let mut rng = Rng::new(0x3010);
    let mut saw = [false; 3];
    for it in 0..120 {
        let nlit = 288usize;
        let ndst = 32usize;
        // 0..15 consecutive + a far-away symbol: gives long zero runs (17/18)
        // and a long run of equal non-zero lengths (16).
        let mut used: Vec<usize> = (0..16).collect();
        used.push(256);
        used.push(20 + rng.below(200) as usize);
        used.sort_unstable();
        used.dedup();
        let litlens = lengths_for(nlit, &used);
        let dstlens = lengths_for(ndst, &[0, 1, 2]);
        let all: Vec<u8> = litlens.iter().chain(dstlens.iter()).copied().collect();
        for (sym, _, _) in cl_encode(&all, ClMode::Repeats) {
            match sym {
                16 => saw[0] = true,
                17 => saw[1] = true,
                18 => saw[2] = true,
                _ => {}
            }
        }
        let n = rng.range(0, 120) as usize;
        let items: Vec<Item> = (0..n)
            .map(|_| {
                let mut s = used[rng.below(used.len() as u32) as usize];
                if s == 256 {
                    s = used[0];
                }
                Item::Lit(s as u16)
            })
            .collect();
        let mut bw = BitWriter::new();
        let (lit, dst) = emit_dynamic_header(
            &mut bw,
            true,
            &litlens,
            &dstlens,
            ClMode::Repeats,
            None,
        );
        emit_items(&mut bw, &lit, &dst, &items);
        let stream = bw.finish();
        let expected = expected_output(&items);
        let ctx = format!("I10 n={n} #{it}");
        let o = h.call(&ctx, &stream, 0, expected.len() as i32);
        expect_output(&ctx, &o, &expected, expected.len() as i32);
    }
    assert_eq!(
        saw,
        [true, true, true],
        "code-length symbols 16/17/18 were not all exercised"
    );
}

/// I11 — distance symbols 30/31 (`cp_dist_base == 0` ⇒
/// `backwards_distance == 0`, i.e. the byte-copy loop with `src == dst`).
#[test]
fn i11_dynamic_dist_sym_30_31() {
    let h = InflateHarness::new("i11", 1 << 15, 1 << 14);
    let mut rng = Rng::new(0x3011);
    for dsym in [30u16, 31] {
        for it in 0..40 {
            let nlit = 288usize;
            let ndst = 32usize;
            let mut used: Vec<usize> = vec![65, 66, 67, 256, 257 + rng.below(20) as usize];
            used.sort_unstable();
            used.dedup();
            let litlens = lengths_for(nlit, &used);
            let dstlens = lengths_for(ndst, &[0, 30, 31]);
            let lsym = *used.iter().find(|&&s| s > 256).unwrap();
            let lx = LEN_EXTRA[lsym - 257];
            let lv = rng.below(1 << lx);
            let mut items: Vec<Item> = vec![Item::Lit(65), Item::Lit(66), Item::Lit(67)];
            items.push(Item::RawMatch(lsym as u16, lv, dsym, 0));
            let mut bw = BitWriter::new();
            let (lit, dst) = emit_dynamic_header(
                &mut bw,
                true,
                &litlens,
                &dstlens,
                ClMode::Literal,
                None,
            );
            emit_items(&mut bw, &lit, &dst, &items);
            let stream = bw.finish();
            let ctx = format!("I11 dsym={dsym} lsym={lsym} lv={lv} #{it}");
            let o = h.call(&ctx, &stream, 0, 1024);
            assert_eq!(o.signal, None, "[{ctx}] {o:?}");
            assert_eq!(o.ret, 1, "[{ctx}] {o:?}");
        }
    }
}

/// I12 — length symbols 286/287 (`cp_len_base == 0` ⇒ `length == 0`).
#[test]
fn i12_dynamic_len_sym_286_287() {
    let h = InflateHarness::new("i12", 1 << 15, 1 << 14);
    let mut rng = Rng::new(0x3012);
    for lsym in [286usize, 287] {
        for dsym in [0usize, 1, 3, 5] {
            let nlit = 288usize;
            let ndst = 32usize;
            let used: Vec<usize> = vec![65, 66, 67, 68, 69, 70, 71, 72, 256, lsym];
            let litlens = lengths_for(nlit, &used);
            let dstlens = lengths_for(ndst, &[0, 1, 3, 5]);
            let dv = rng.below(1 << DIST_EXTRA[dsym]);
            let items = vec![
                Item::Lit(65),
                Item::Lit(66),
                Item::Lit(67),
                Item::Lit(68),
                Item::Lit(69),
                Item::Lit(70),
                Item::Lit(71),
                Item::Lit(72),
                Item::RawMatch(lsym as u16, 0, dsym as u16, dv),
            ];
            let mut bw = BitWriter::new();
            let (lit, dst) = emit_dynamic_header(
                &mut bw,
                true,
                &litlens,
                &dstlens,
                ClMode::Literal,
                None,
            );
            emit_items(&mut bw, &lit, &dst, &items);
            let stream = bw.finish();
            let ctx = format!("I12 lsym={lsym} dsym={dsym} dv={dv}");
            let o = h.call(&ctx, &stream, 0, 1024);
            assert_eq!(o.signal, None, "[{ctx}] {o:?}");
            assert_eq!(o.ret, 1, "[{ctx}] {o:?}");
            assert_eq!(&o.out[..8], b"ABCDEFGH", "[{ctx}] literals wrong");
        }
    }
}

/// I14 — blocks of different `BTYPE` chained together.
#[test]
fn i14_multi_mixed_blocks() {
    let h = InflateHarness::new("i14", 1 << 15, 1 << 14);
    let mut rng = Rng::new(0x3014);
    for it in 0..120 {
        let nlit = 257usize;
        let ndst = 1usize;
        let used = random_lit_alphabet(&mut rng, nlit, 8);
        let litlens = lengths_for(nlit, &used);
        let dstlens = lengths_for(ndst, &[0]);
        let dyn_items: Vec<Item> = (0..rng.range(0, 30) as usize)
            .map(|_| {
                let mut s = used[rng.below(used.len() as u32) as usize];
                if s == 256 {
                    s = used[0];
                }
                Item::Lit(s as u16)
            })
            .collect();
        let fixed_items: Vec<Item> = (0..rng.range(0, 30) as usize)
            .map(|_| Item::Lit(rng.u8() as u16))
            .collect();

        // dynamic -> fixed(final)
        let mut bw = BitWriter::new();
        let (lit, dst) =
            emit_dynamic_header(&mut bw, false, &litlens, &dstlens, ClMode::Repeats, None);
        emit_items(&mut bw, &lit, &dst, &dyn_items);
        emit_fixed_block(&mut bw, true, &fixed_items);
        let mut expected = expected_output(&dyn_items);
        expected.extend(expected_output(&fixed_items));
        let ctx = format!("I14a #{it}");
        let o = h.call(&ctx, &bw.clone().finish(), 0, expected.len() as i32);
        expect_output(&ctx, &o, &expected, expected.len() as i32);

        // fixed -> dynamic(final)
        let mut bw = BitWriter::new();
        emit_fixed_block(&mut bw, false, &fixed_items);
        let (lit, dst) =
            emit_dynamic_header(&mut bw, true, &litlens, &dstlens, ClMode::Literal, None);
        emit_items(&mut bw, &lit, &dst, &dyn_items);
        let mut expected = expected_output(&fixed_items);
        expected.extend(expected_output(&dyn_items));
        let ctx = format!("I14b #{it}");
        let o = h.call(&ctx, &bw.finish(), 0, expected.len() as i32);
        expect_output(&ctx, &o, &expected, expected.len() as i32);
    }
}

/// I15 — deep dynamic tree: code lengths 1…14 (lengths > 9 get no `s->lookup`
/// entry), with `HCLEN` forced to both extremes.
#[test]
fn i15_dynamic_deep_tree() {
    let h = InflateHarness::new("i15", 1 << 15, 1 << 14);
    let mut rng = Rng::new(0x3015);
    for depth in 2..=14usize {
        for force in [None, Some(19usize)] {
            let nlit = 288usize;
            let ndst = 1usize;
            let mut used: Vec<usize> = vec![256];
            let mut s = 1usize;
            while used.len() < depth + 1 {
                if !used.contains(&s) {
                    used.push(s);
                }
                s += 7;
            }
            used.sort_unstable();
            let litlens = deep_lengths(nlit, &used, depth);
            let dstlens = lengths_for(ndst, &[0]);
            let enc = HuffEnc::new(litlens.clone());
            assert_eq!(enc.kraft(), 1 << 15, "depth={depth} tree is not complete");
            let n = rng.range(0, 150) as usize;
            let items: Vec<Item> = (0..n)
                .map(|_| {
                    let mut x = used[rng.below(used.len() as u32) as usize];
                    if x == 256 {
                        x = used[0].max(1);
                        if x == 256 {
                            x = used[1];
                        }
                    }
                    Item::Lit(x as u16)
                })
                .filter(|it| matches!(it, Item::Lit(v) if *v != 256))
                .collect();
            let mut bw = BitWriter::new();
            let (lit, dst) = emit_dynamic_header(
                &mut bw,
                true,
                &litlens,
                &dstlens,
                ClMode::Repeats,
                force,
            );
            emit_items(&mut bw, &lit, &dst, &items);
            let stream = bw.finish();
            let expected = expected_output(&items);
            let ctx = format!("I15 depth={depth} force={force:?} n={n}");
            let o = h.call(&ctx, &stream, 0, expected.len() as i32);
            expect_output(&ctx, &o, &expected, expected.len() as i32);
        }
    }
}

/// I16 — `HLIT`/`HDIST` extremes.
#[test]
fn i16_dynamic_header_extremes() {
    let h = InflateHarness::new("i16", 1 << 15, 1 << 14);
    let mut rng = Rng::new(0x3016);
    for nlit in [257usize, 258, 287, 288] {
        for ndst in [1usize, 2, 31, 32] {
            for mode in [ClMode::Literal, ClMode::Repeats] {
                let used = random_lit_alphabet(&mut rng, nlit, 6);
                let litlens = lengths_for(nlit, &used);
                let dstlens = lengths_for(ndst, &[0]);
                let items: Vec<Item> = (0..40)
                    .map(|_| {
                        let mut s = used[rng.below(used.len() as u32) as usize];
                        if s == 256 {
                            s = used[0];
                        }
                        Item::Lit(s as u16)
                    })
                    .filter(|it| matches!(it, Item::Lit(v) if *v != 256))
                    .collect();
                let mut bw = BitWriter::new();
                let (lit, dst) =
                    emit_dynamic_header(&mut bw, true, &litlens, &dstlens, mode, None);
                emit_items(&mut bw, &lit, &dst, &items);
                let stream = bw.finish();
                let expected = expected_output(&items);
                let ctx = format!("I16 nlit={nlit} ndst={ndst}");
                let o = h.call(&ctx, &stream, 0, expected.len() as i32);
                expect_output(&ctx, &o, &expected, expected.len() as i32);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// I17…I20 — real zlib output
// ---------------------------------------------------------------------------

fn flate2_row(ctx: &str, seed: u64, levels: &[u32], mk: impl Fn(&mut Rng, usize) -> Vec<u8>) {
    let h = InflateHarness::new(ctx, 1 << 16, 1 << 16);
    let mut rng = Rng::new(seed);
    let mut ok = 0usize;
    let mut total = 0usize;
    for &level in levels {
        for &n in &[0usize, 1, 2, 3, 5, 17, 64, 255, 700, 3000] {
            let data = mk(&mut rng, n);
            let stream = raw_deflate(&data, level);
            if stream.len() + 8 >= h.inbuf.usable() || data.len() + 8 >= h.out_c.usable() {
                continue;
            }
            let c = format!("{ctx} level={level} n={n}");
            let o = h.call(&c, &stream, 0, data.len() as i32);
            total += 1;
            if o.signal.is_none() && o.ret == 1 {
                assert_eq!(
                    &o.out[..data.len()],
                    &data[..],
                    "[{c}] round-trip payload mismatch"
                );
                ok += 1;
            }
        }
    }
    assert!(
        ok * 2 >= total,
        "[{ctx}] only {ok}/{total} real zlib streams decoded successfully — \
         the row is not exercising the success path"
    );
}

/// I17 — level 0 (all stored blocks).
#[test]
fn i17_flate2_level0() {
    // A stored block only satisfies the C code's `bits_left/8 <= LEN` check when
    // it is the last thing in the stream, which is the case for level 0 with a
    // single block.
    let h = InflateHarness::new("i17", 1 << 16, 1 << 16);
    let mut rng = Rng::new(0x3017);
    let mut ok = 0;
    let mut total = 0;
    for n in [0usize, 1, 2, 3, 4, 5, 16, 100, 1000, 2048] {
        let data = rng.bytes(n);
        let stream = raw_deflate(&data, 0);
        let ctx = format!("I17 n={n}");
        let o = h.call(&ctx, &stream, 0, data.len() as i32);
        total += 1;
        if o.signal.is_none() && o.ret == 1 && n >= 3 {
            assert_eq!(&o.out[..data.len()], &data[..], "[{ctx}] round-trip");
            ok += 1;
        }
    }
    assert!(ok >= 5, "only {ok}/{total} level-0 streams decoded");
}

/// I18 — levels 1…9 on random (literal-heavy) payloads.
#[test]
fn i18_flate2_random() {
    flate2_row("I18", 0x3018, &[1, 2, 3, 4, 5, 6, 7, 8, 9], |rng, n| {
        rng.bytes(n)
    });
}

/// I19 — levels 1…9 on highly repetitive payloads (long matches, distance 1).
#[test]
fn i19_flate2_repetitive() {
    flate2_row("I19", 0x3019, &[1, 2, 3, 4, 5, 6, 7, 8, 9], |rng, n| {
        let period = 1 + rng.below(7) as usize;
        let base: Vec<u8> = (0..period).map(|_| rng.u8()).collect();
        (0..n).map(|i| base[i % period]).collect()
    });
}

/// I20 — levels 1…9 on text-like payloads (small alphabet, mixed matches).
#[test]
fn i20_flate2_textlike() {
    flate2_row("I20", 0x3020, &[1, 2, 3, 4, 5, 6, 7, 8, 9], |rng, n| {
        const WORDS: [&str; 8] = [
            "the ", "quick ", "brown ", "fox ", "jumps ", "over ", "lazy ", "dog. ",
        ];
        let mut v = Vec::new();
        while v.len() < n {
            v.extend_from_slice(WORDS[rng.below(8) as usize].as_bytes());
        }
        v.truncate(n);
        v
    });
}

// ---------------------------------------------------------------------------
// I21…I24 — pointer alignment, input tail length, output sizing
// ---------------------------------------------------------------------------

fn corpus(rng: &mut Rng) -> Vec<(String, Vec<u8>, Vec<u8>)> {
    let mut v = Vec::new();

    // static block, literals only
    let payload = rng.bytes(37);
    let items: Vec<Item> = payload.iter().map(|&b| Item::Lit(b as u16)).collect();
    let mut bw = BitWriter::new();
    emit_fixed_block(&mut bw, true, &items);
    v.push(("fixed-lit".to_string(), bw.finish(), payload));

    // static block with matches
    let mut items: Vec<Item> = (0..8).map(|i| Item::Lit(i as u16 + 97)).collect();
    items.push(Item::Match(20, 3));
    items.push(Item::Match(258, 1));
    let expected = expected_output(&items);
    let mut bw = BitWriter::new();
    emit_fixed_block(&mut bw, true, &items);
    v.push(("fixed-match".to_string(), bw.finish(), expected));

    // dynamic block
    let used: Vec<usize> = vec![10, 65, 66, 200, 256];
    let litlens = lengths_for(280, &used);
    let dstlens = lengths_for(4, &[0, 1]);
    let items: Vec<Item> = (0..25)
        .map(|i| Item::Lit(used[i % 4] as u16))
        .filter(|it| matches!(it, Item::Lit(x) if *x != 256))
        .collect();
    let mut bw = BitWriter::new();
    let (lit, dst) = emit_dynamic_header(&mut bw, true, &litlens, &dstlens, ClMode::Repeats, None);
    emit_items(&mut bw, &lit, &dst, &items);
    v.push(("dynamic".to_string(), bw.finish(), expected_output(&items)));

    // stored block (LEN == remaining)
    let payload = rng.bytes(64);
    let mut bw = BitWriter::new();
    emit_stored_block(&mut bw, true, &payload, None);
    v.push(("stored".to_string(), bw.finish(), payload));

    // real zlib stream
    let data: Vec<u8> = (0..300u32).map(|i| (i % 41) as u8).collect();
    v.push(("zlib-l6".to_string(), raw_deflate(&data, 6), data));

    // multi-block
    let mut bw = BitWriter::new();
    let a: Vec<Item> = (0..10).map(|i| Item::Lit(i as u16 + 48)).collect();
    let b: Vec<Item> = (0..10).map(|i| Item::Lit(i as u16 + 65)).collect();
    emit_fixed_block(&mut bw, false, &a);
    emit_fixed_block(&mut bw, true, &b);
    let mut expected = expected_output(&a);
    expected.extend(expected_output(&b));
    v.push(("two-fixed".to_string(), bw.finish(), expected));

    v
}

/// I21 — the whole corpus at every input-pointer alignment (`first_bytes`
/// 0/3/2/1).
#[test]
fn i21_input_alignment() {
    let h = InflateHarness::new("i21", 1 << 16, 1 << 16);
    let mut rng = Rng::new(0x3021);
    for (name, stream, expected) in corpus(&mut rng) {
        for align in 0..4usize {
            let ctx = format!("I21 {name} align={align}");
            let o = h.call(&ctx, &stream, align, expected.len() as i32);
            // Alignment must not change the result for non-stored streams; the
            // stored path's `cp_ptr()` is alignment-independent too because the
            // pre-loaded head bytes are accounted for in `count`.
            expect_output(&ctx, &o, &expected, expected.len() as i32);
        }
    }
}

/// I22 — trailing padding so that `(in_bytes - first_bytes) & 3` takes all 4
/// values (`final_word_available`, `count += bits_left`).
#[test]
fn i22_input_tail_length() {
    let h = InflateHarness::new("i22", 1 << 16, 1 << 16);
    let mut rng = Rng::new(0x3022);
    for (name, stream, expected) in corpus(&mut rng) {
        if name == "stored" {
            continue; // padding breaks the stored `LEN >= remaining` invariant
        }
        for pad in 0..8usize {
            for align in 0..4usize {
                let mut s = stream.clone();
                s.extend(std::iter::repeat(0).take(pad));
                let ctx = format!("I22 {name} pad={pad} align={align}");
                let o = h.call(&ctx, &s, align, expected.len() as i32);
                expect_output(&ctx, &o, &expected, expected.len() as i32);
            }
        }
    }
}

/// I23 — `out_bytes` exactly the decompressed size vs. plenty of slack.
#[test]
fn i23_out_bytes_exact_vs_slack() {
    let h = InflateHarness::new("i23", 1 << 16, 1 << 16);
    let mut rng = Rng::new(0x3023);
    for (name, stream, expected) in corpus(&mut rng) {
        for extra in [0i32, 1, 7, 1000] {
            let ob = expected.len() as i32 + extra;
            let ctx = format!("I23 {name} out_bytes={ob}");
            let o = h.call(&ctx, &stream, 0, ob);
            expect_output(&ctx, &o, &expected, ob);
        }
    }
}

/// I24 — trailing garbage after the final block must be ignored.
#[test]
fn i24_trailing_garbage() {
    let h = InflateHarness::new("i24", 1 << 16, 1 << 16);
    let mut rng = Rng::new(0x3024);
    for (name, stream, expected) in corpus(&mut rng) {
        if name == "stored" {
            continue;
        }
        for extra in [1usize, 2, 3, 4, 9, 33] {
            let mut s = stream.clone();
            let g = rng.bytes(extra);
            s.extend_from_slice(&g);
            let ctx = format!("I24 {name} garbage={extra}");
            let o = h.call(&ctx, &s, 0, expected.len() as i32);
            expect_output(&ctx, &o, &expected, expected.len() as i32);
        }
    }
}
