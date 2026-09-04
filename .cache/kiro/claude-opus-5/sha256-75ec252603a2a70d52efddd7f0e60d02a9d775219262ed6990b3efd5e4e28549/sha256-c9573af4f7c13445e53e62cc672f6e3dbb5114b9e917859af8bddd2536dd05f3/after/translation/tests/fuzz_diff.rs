//! Broad randomized differential fuzzing.
//!
//! The enumerated rows in `CONFIGS.md` / `ERRORS.md` cover what the C source
//! *says* it does. This file covers what it actually does on inputs nobody
//! enumerated: mutated valid PNGs, random bytes, random deflate streams and
//! random exported-table states. Every case asserts C and Rust agree exactly.

mod common;

use common::deflate::{self, Tok};
use common::png::{self, ColorType, PngSpec};
use common::*;

/// Bit flips land in the IHDR dimension fields too, which can ask for hundreds
/// of megabytes. Capping the children's address space makes those fail at
/// `malloc` — identically in both libraries — instead of spending minutes
/// walking gigabytes at `-O0`. The size guards themselves are covered
/// deterministically by `phase_c_errors::err13/err14`.
fn cap_child_memory() {
    set_child_as_limit(192 << 20);
}

fn valid_png(rng: &mut Rng, ct: ColorType) -> Vec<u8> {
    let w = rng.range(1, 12) as usize;
    let h = rng.range(1, 12) as usize;
    let bpp = ct.bpp();
    let filters: Vec<u8> = (0..h).map(|_| rng.below(5) as u8).collect();
    let raw = png::raw_scanlines(rng, w, h, bpp, &filters);
    let def = deflate::stored_block(&raw, true);
    let mut spec = PngSpec::new(w as u32, h as u32, ct as u8, def, raw);
    if ct == ColorType::Indexed {
        spec.plte = Some(rng.bytes(256 * 3));
    }
    if rng.below(2) == 0 {
        let tl = rng.range(0, 256) as usize;
        spec.trns = Some(rng.bytes(tl));
    }
    spec.idat_chunks = rng.range(1, 4) as usize;
    spec.build()
}

#[test]
fn fuzz_bitflipped_pngs() {
    cap_child_memory();
    // Single- and multi-byte corruptions of otherwise valid files: hits the
    // chunk walker, the IHDR guards, the zlib header, the DEFLATE decoder and
    // the filter switch in proportions no hand-written test achieves.
    let mut rng = Rng::new(SEED ^ 0xF01);
    let mut stats = [0usize; 4]; // ok, rejected, signalled, other
    for iter in 0..1200 {
        let ct = ColorType::ALL[(iter % 5) as usize];
        let mut file = valid_png(&mut rng, ct);
        let nflips = rng.range(1, 4) as usize;
        for _ in 0..nflips {
            let i = rng.below(file.len() as u32) as usize;
            file[i] ^= 1u8 << rng.below(8);
        }
        let (c, r) = call_load_png(&file);
        assert_same(&format!("fuzz_bitflip iter={iter} ct={}", ct as u8), &c, &r);
        if c.signal.is_some() {
            stats[2] += 1;
        } else if c.pix_null {
            stats[1] += 1;
        } else {
            stats[0] += 1;
        }
    }
    eprintln!(
        "fuzz_bitflipped_pngs: {} decoded, {} rejected, {} died by signal",
        stats[0], stats[1], stats[2]
    );
    assert!(stats[0] > 0 && stats[1] > 0, "fuzz corpus too one-sided: {stats:?}");
}

#[test]
fn fuzz_truncated_pngs() {
    cap_child_memory();
    let mut rng = Rng::new(SEED ^ 0xF02);
    for iter in 0..300 {
        let ct = ColorType::ALL[(iter % 5) as usize];
        let file = valid_png(&mut rng, ct);
        let cut = rng.range(1, file.len() as u32) as usize;
        let (c, r) = call_load_png(&file[..cut]);
        assert_same(&format!("fuzz_trunc iter={iter} cut={cut}"), &c, &r);
        // Also keep the buffer intact but lie about its length.
        let (c, r) = call_load_png_len(&file, cut as i32);
        assert_same(&format!("fuzz_short_len iter={iter} len={cut}"), &c, &r);
    }
}

#[test]
fn fuzz_random_bytes_as_png() {
    cap_child_memory();
    let mut rng = Rng::new(SEED ^ 0xF03);
    for iter in 0..400 {
        let n = rng.range(8, 400) as usize;
        let mut buf = rng.bytes(n);
        // Half the time keep a valid signature so the walker is reached.
        if iter % 2 == 0 && n >= 8 {
            buf[..8].copy_from_slice(&png::SIG);
        }
        let (c, r) = call_load_png(&buf);
        assert_same(&format!("fuzz_random_png iter={iter} n={n}"), &c, &r);
    }
}

#[test]
fn fuzz_random_ihdr_fields() {
    cap_child_memory();
    // The five IHDR bytes after the dimensions are all `int`-typed switches in
    // the C; sweep them jointly with random dimensions.
    let mut rng = Rng::new(SEED ^ 0xF04);
    for iter in 0..600 {
        let mut spec = {
            let w = rng.range(1, 8) as usize;
            let h = rng.range(1, 8) as usize;
            let filters = vec![0u8; h];
            let raw = png::raw_scanlines(&mut rng, w, h, 1, &filters);
            let def = deflate::stored_block(&raw, true);
            let mut s = PngSpec::new(w as u32, h as u32, 0, def, raw);
            s.plte = Some(rng.bytes(256 * 3));
            s
        };
        spec.bit_depth = if rng.below(3) == 0 { 8 } else { rng.u8() };
        spec.color_type = match rng.below(3) {
            0 => [0u8, 2, 3, 4, 6][rng.below(5) as usize],
            _ => rng.u8(),
        };
        spec.compression = if rng.below(3) == 0 { 0 } else { rng.u8() };
        spec.filter = if rng.below(3) == 0 { 0 } else { rng.u8() };
        spec.interlace = if rng.below(3) == 0 { 0 } else { rng.u8() };
        if rng.below(4) == 0 {
            spec.w = rng.u32();
        }
        if rng.below(4) == 0 {
            spec.h = rng.u32();
        }
        let file = spec.build();
        let (c, r) = call_load_png(&file);
        assert_same(
            &format!(
                "fuzz_ihdr iter={iter} bd={} ct={} comp={} filt={} il={} w={} h={}",
                spec.bit_depth,
                spec.color_type,
                spec.compression,
                spec.filter,
                spec.interlace,
                spec.w,
                spec.h
            ),
            &c,
            &r,
        );
    }
}

#[test]
fn fuzz_random_chunk_streams() {
    cap_child_memory();
    // Random chunk types / lengths after a valid IHDR, driving cp_find and
    // cp_chunk with declared lengths that overflow, sign-extend or point back.
    let mut rng = Rng::new(SEED ^ 0xF05);
    let types: [&[u8; 4]; 8] = [
        b"IDAT", b"PLTE", b"tRNS", b"IEND", b"gAMA", b"IDAt", b"idat", b"\x00\x00\x00\x00",
    ];
    for iter in 0..500 {
        let mut buf = png::SIG.to_vec();
        buf.extend_from_slice(&png::chunk(b"IHDR", &png::ihdr_data(4, 3, 8, 0)));
        let n = rng.range(1, 6) as usize;
        for _ in 0..n {
            let ty = types[rng.below(types.len() as u32) as usize];
            let payload_len = rng.range(0, 24) as usize;
            let payload = rng.bytes(payload_len);
            if rng.below(3) == 0 {
                let declared = match rng.below(4) {
                    0 => rng.u32(),
                    1 => 0x8000_0000u32.wrapping_add(rng.below(64)),
                    2 => 0xFFFF_FFFFu32 - rng.below(16),
                    _ => payload_len as u32,
                };
                buf.extend_from_slice(&png::chunk_raw_len(ty, declared, &payload));
            } else {
                buf.extend_from_slice(&png::chunk(ty, &payload));
            }
        }
        let (c, r) = call_load_png(&buf);
        assert_same(&format!("fuzz_chunks iter={iter}"), &c, &r);
    }
}

#[test]
fn fuzz_random_deflate_streams() {
    cap_child_memory();
    let mut rng = Rng::new(SEED ^ 0xF06);
    let mut stats = [0usize; 3];
    for iter in 0..1500 {
        let n = rng.range(1, 60) as usize;
        let mut buf = rng.bytes(n);
        // Bias the first byte towards a legal block header so the decoder
        // actually gets going.
        if iter % 3 != 0 {
            buf[0] = (buf[0] & 0xF8) | (rng.below(3) as u8) << 1 | (rng.below(2) as u8);
        }
        let out_bytes = [0i32, 1, 16, 256, 4096][rng.below(5) as usize];
        let shift = rng.below(4) as usize;
        let (c, r) = call_inflate_cfg(&buf, buf.len() as i32, out_bytes, shift, |_| {});
        assert_same(
            &format!("fuzz_deflate iter={iter} n={n} out={out_bytes} shift={shift}"),
            &c,
            &r,
        );
        if c.signal.is_some() {
            stats[2] += 1;
        } else if c.ret == 0 {
            stats[1] += 1;
        } else {
            stats[0] += 1;
        }
    }
    eprintln!(
        "fuzz_random_deflate_streams: {} ok, {} rejected, {} died by signal",
        stats[0], stats[1], stats[2]
    );
    assert!(stats[1] > 0 && stats[2] > 0, "fuzz corpus too one-sided: {stats:?}");
}

#[test]
fn fuzz_mutated_valid_deflate_streams() {
    cap_child_memory();
    // Start from streams that *are* valid, then flip a few bits: keeps the
    // decoder deep inside cp_block / cp_dynamic when things go wrong.
    let mut rng = Rng::new(SEED ^ 0xF07);
    let mut alarms = 0usize;
    for iter in 0..800 {
        let n = rng.range(4, 120) as usize;
        let data = rng.bytes(n);
        let toks = if rng.below(2) == 0 {
            data.iter().map(|&b| Tok::Lit(b)).collect::<Vec<_>>()
        } else {
            deflate::lz77(&data, 32768, 16)
        };
        let mut def = if rng.below(2) == 0 {
            deflate::fixed_stream(&toks)
        } else {
            let (ll, dl) = deflate::tables_for(&toks, 15, 288, 30);
            let mut bw = deflate::BitWriter::new();
            deflate::dynamic_block(&mut bw, &ll, &dl, &toks, true, rng.below(2) == 0);
            bw.finish()
        };
        for _ in 0..rng.range(1, 3) {
            let i = rng.below(def.len() as u32) as usize;
            def[i] ^= 1u8 << rng.below(8);
        }
        let out_bytes = (n as i32) + (rng.below(64) as i32);
        let (c, r) = call_inflate_cfg(&def, def.len() as i32, out_bytes, rng.below(4) as usize, |_| {});
        assert_same(&format!("fuzz_mutated_deflate iter={iter} n={n}"), &c, &r);
        if c.signal == Some(14) { alarms += 1; }
    }
    eprintln!("fuzz_mutated_valid_deflate_streams: {alarms} cases livelocked (SIGALRM)");
}

#[test]
fn fuzz_random_table_states() {
    cap_child_memory();
    // Randomize the exported tables (within the ranges that avoid the C's own
    // out-of-bounds `counts[]` write) and run a stream through both libraries.
    let mut rng = Rng::new(SEED ^ 0xF08);
    for iter in 0..300 {
        let mut fixed = [0u8; 320];
        // Valid complete codes so the trees are well-formed.
        let lf: Vec<u32> = (0..288).map(|_| rng.range(1, 64)).collect();
        let df: Vec<u32> = (0..32).map(|_| rng.range(1, 64)).collect();
        fixed[..288].copy_from_slice(&deflate::huff_lengths(&lf, 15));
        fixed[288..].copy_from_slice(&deflate::huff_lengths(&df, 15));

        let mut perm = [0u8; 19];
        for i in 0..19 {
            perm[i] = i as u8;
        }
        for i in (1..19).rev() {
            let j = rng.below(i as u32 + 1) as usize;
            perm.swap(i, j);
        }
        let mut lex = [0u8; 31];
        let mut lba = [0u32; 31];
        for i in 0..31 {
            lex[i] = rng.below(6) as u8;
            lba[i] = rng.range(1, 64);
        }
        let mut dex = [0u8; 32];
        let mut dba = [0u32; 32];
        for i in 0..32 {
            dex[i] = rng.below(6) as u8;
            dba[i] = rng.range(1, 64);
        }

        let n = rng.range(4, 80) as usize;
        let data = rng.bytes(n);
        let toks = deflate::lz77(&data, 512, 16);
        let def = if iter % 2 == 0 {
            deflate::fixed_stream(&toks)
        } else {
            let (ll, dl) = deflate::tables_for(&toks, 15, 288, 30);
            let mut bw = deflate::BitWriter::new();
            deflate::dynamic_block(&mut bw, &ll, &dl, &toks, true, true);
            bw.finish()
        };
        let (c, r) = call_inflate_cfg(&def, def.len() as i32, 4096, 0, move |lib| unsafe {
            std::ptr::copy_nonoverlapping(fixed.as_ptr(), lib.cp_fixed_table, 320);
            std::ptr::copy_nonoverlapping(perm.as_ptr(), lib.cp_permutation_order, 19);
            std::ptr::copy_nonoverlapping(lex.as_ptr(), lib.cp_len_extra_bits, 31);
            std::ptr::copy_nonoverlapping(lba.as_ptr(), lib.cp_len_base, 31);
            std::ptr::copy_nonoverlapping(dex.as_ptr(), lib.cp_dist_extra_bits, 32);
            std::ptr::copy_nonoverlapping(dba.as_ptr(), lib.cp_dist_base, 32);
        });
        assert_same(&format!("fuzz_tables iter={iter}"), &c, &r);
    }
}

#[test]
fn fuzz_random_filter_and_palette_shapes() {
    cap_child_memory();
    let mut rng = Rng::new(SEED ^ 0xF09);
    for iter in 0..600 {
        let ct = ColorType::ALL[rng.below(5) as usize];
        let w = rng.range(1, 20) as usize;
        let h = rng.range(1, 20) as usize;
        let bpp = ct.bpp();
        // Filter bytes drawn from the whole 0..=255 range, weighted towards
        // valid ones so most rows still decode.
        let filters: Vec<u8> = (0..h)
            .map(|_| if rng.below(4) == 0 { rng.u8() } else { rng.below(5) as u8 })
            .collect();
        let raw = png::raw_scanlines(&mut rng, w, h, bpp, &filters);
        let def = deflate::stored_block(&raw, true);
        let mut spec = PngSpec::new(w as u32, h as u32, ct as u8, def, raw);
        if rng.below(6) != 0 {
            let entries = rng.range(0, 256) as usize;
            spec.plte = Some(rng.bytes(entries * 3));
        }
        if rng.below(2) == 0 {
            let tl = rng.range(0, 300) as usize;
            spec.trns = Some(rng.bytes(tl));
        }
        spec.trns_first = rng.below(2) == 0;
        let file = spec.build();
        let (c, r) = call_load_png(&file);
        assert_same(
            &format!("fuzz_shapes iter={iter} ct={} {w}x{h}", ct as u8),
            &c,
            &r,
        );
    }
}
