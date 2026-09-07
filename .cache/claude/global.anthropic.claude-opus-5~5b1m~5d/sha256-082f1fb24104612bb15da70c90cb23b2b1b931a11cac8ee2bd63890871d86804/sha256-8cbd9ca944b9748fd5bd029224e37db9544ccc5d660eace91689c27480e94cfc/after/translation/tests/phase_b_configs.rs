//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Every test loads BOTH the C `.so` and the Rust `.so` through `libloading`
//! and compares the `int` return value plus all 40 bytes of `struct ima_info`.

mod common;
use common::*;

/// Realistic sample rates, stored big-endian in the file the way a real CAF
/// writer would (the C then mis-reads them natively — that is the point).
const RATES: [f64; 8] = [
    8000.0, 11025.0, 22050.0, 32000.0, 44100.0, 48000.0, 96000.0, 192000.0,
];

// ---------------------------------------------------------------------------
// Row 1 — minimal valid document, all unread fields randomized.
// ---------------------------------------------------------------------------
#[test]
fn cfg_row01_minimal_valid() {
    let l = Libs::load();
    let rng = Rng::new(0x1111_1111);
    for i in 0..500 {
        let rate = RATES[i % RATES.len()];
        let buf = valid_doc(
            &rng,
            u64::from_be_bytes(rate.to_be_bytes()),
            rng.next_u32(),
            rng.next_u64(),
            rng.next_u64() as i64,
            rng.below(4),
        );
        l.assert_same(&format!("row01 i={i}"), &buf);
    }
}

// ---------------------------------------------------------------------------
// Row 2 — reordered: pakt before desc.
// ---------------------------------------------------------------------------
#[test]
fn cfg_row02_pakt_before_desc() {
    let l = Libs::load();
    let rng = Rng::new(0x2222_2222);
    for i in 0..500 {
        let mut d = Doc::new(&rng);
        let fc = rng.next_u64();
        let sr = rng.next_u64();
        let ch = rng.next_u32();
        d.pakt(&rng, fc);
        d.desc(&rng, sr, FOURCC_IMA4, ch);
        d.data(&rng, rng.next_u64() as i64, rng.below(3));
        l.assert_same(&format!("row02 i={i}"), &d.bytes);
    }
}

// ---------------------------------------------------------------------------
// Row 3 — unknown chunks interleaved (0, 1, 2, 8 of them, random FourCCs).
// ---------------------------------------------------------------------------
#[test]
fn cfg_row03_unknown_chunks() {
    let l = Libs::load();
    let rng = Rng::new(0x3333_3333);
    for &n in &[0usize, 1, 2, 3, 8] {
        for i in 0..200 {
            let mut d = Doc::new(&rng);
            for _ in 0..n {
                let body = rng.below(48);
                d.unknown(&rng, body, body as i64);
            }
            d.desc(&rng, rng.next_u64(), FOURCC_IMA4, rng.next_u32());
            for _ in 0..n {
                let body = rng.below(40);
                d.unknown(&rng, body, body as i64);
            }
            d.pakt(&rng, rng.next_u64());
            for _ in 0..n {
                let body = rng.below(24);
                d.unknown(&rng, body, body as i64);
            }
            d.data(&rng, rng.next_u64() as i64, rng.below(3));
            l.assert_same(&format!("row03 n={n} i={i}"), &d.bytes);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 4 — duplicate `desc` chunks: the later one must win.
// ---------------------------------------------------------------------------
#[test]
fn cfg_row04_duplicate_desc() {
    let l = Libs::load();
    let rng = Rng::new(0x4444_4444);
    for i in 0..400 {
        let mut d = Doc::new(&rng);
        // First desc has a deliberately WRONG format id and different channels;
        // if the C kept the first one the result would be -3.
        d.desc(&rng, rng.next_u64(), rng.next_u32(), rng.next_u32());
        d.pakt(&rng, rng.next_u64());
        d.desc(&rng, rng.next_u64(), FOURCC_IMA4, rng.next_u32());
        d.data(&rng, rng.next_u64() as i64, rng.below(3));
        l.assert_same(&format!("row04 i={i}"), &d.bytes);

        // ...and the mirror case: valid first, garbage second -> must be -3.
        let mut d2 = Doc::new(&rng);
        d2.desc(&rng, rng.next_u64(), FOURCC_IMA4, rng.next_u32());
        d2.desc(&rng, rng.next_u64(), rng.next_u32(), rng.next_u32());
        d2.pakt(&rng, rng.next_u64());
        d2.data(&rng, rng.next_u64() as i64, 1);
        l.assert_same(&format!("row04b i={i}"), &d2.bytes);
    }
}

// ---------------------------------------------------------------------------
// Row 5 — duplicate `pakt` chunks: the later one must win.
// ---------------------------------------------------------------------------
#[test]
fn cfg_row05_duplicate_pakt() {
    let l = Libs::load();
    let rng = Rng::new(0x5555_5555);
    for i in 0..400 {
        let mut d = Doc::new(&rng);
        d.pakt(&rng, rng.next_u64());
        d.desc(&rng, rng.next_u64(), FOURCC_IMA4, rng.next_u32());
        d.pakt(&rng, rng.next_u64());
        d.pakt(&rng, rng.next_u64());
        d.data(&rng, rng.next_u64() as i64, rng.below(3));
        l.assert_same(&format!("row05 i={i}"), &d.bytes);
    }
}

// ---------------------------------------------------------------------------
// Row 6 — unknown chunk with size 0: cursor advances by exactly 16 bytes.
//          (This is what proves the chunk stride is sizeof(caf_chunk)==16.)
// ---------------------------------------------------------------------------
#[test]
fn cfg_row06_zero_size_unknown_chunk() {
    let l = Libs::load();
    let rng = Rng::new(0x6666_6666);
    for i in 0..300 {
        let mut d = Doc::new(&rng);
        d.unknown(&rng, 0, 0);
        d.unknown(&rng, 0, 0);
        d.desc(&rng, rng.next_u64(), FOURCC_IMA4, rng.next_u32());
        d.unknown(&rng, 0, 0);
        d.pakt(&rng, rng.next_u64());
        d.unknown(&rng, 0, 0);
        d.data(&rng, rng.next_u64() as i64, rng.below(3));
        l.assert_same(&format!("row06 i={i}"), &d.bytes);
    }
}

// ---------------------------------------------------------------------------
// Row 7 — unknown chunk with a large positive size (skips a big region).
// ---------------------------------------------------------------------------
#[test]
fn cfg_row07_large_positive_skip() {
    let l = Libs::load();
    let rng = Rng::new(0x7777_7777);
    for &skip in &[64usize, 256, 1024, 4096, 65536] {
        for i in 0..40 {
            let mut d = Doc::new(&rng);
            d.desc(&rng, rng.next_u64(), FOURCC_IMA4, rng.next_u32());
            d.pakt(&rng, rng.next_u64());
            d.unknown(&rng, skip, skip as i64);
            d.data(&rng, rng.next_u64() as i64, rng.below(3));
            l.assert_same(&format!("row07 skip={skip} i={i}"), &d.bytes);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 8 — NEGATIVE chunk size: cursor walks backwards and finds `data`
//          in a region an earlier forward-skip jumped over.
//
//  off   8 : desc  (size 32)            -> next  56
//  off  56 : pakt  (size 24)            -> next  96
//  off  96 : JUMP_FWD (size 32)         -> next 144
//  off 112 : 16 bytes filler (skipped)
//  off 128 : data chunk header          <- reached by the back-jump
//  off 144 : JUMP_BACK (size -32)       -> next 128  == data  => break
// ---------------------------------------------------------------------------
fn build_negative_skip_doc(rng: &Rng, data_size: i64) -> (Vec<u8>, usize) {
    let mut d = Doc::new(rng);
    assert_eq!(d.len(), 8);
    d.desc(rng, rng.next_u64(), FOURCC_IMA4, rng.next_u32());
    assert_eq!(d.len(), 56);
    d.pakt(rng, rng.next_u64());
    assert_eq!(d.len(), 96);
    d.chunk_hdr(rng, 0x4a_4d_50_31, 32); // JUMP_FWD, size 32 -> 96+16+32 = 144
    assert_eq!(d.len(), 112);
    d.pad(rng, 16); // 112..128 filler
    let data_off = d.len();
    assert_eq!(data_off, 128);
    d.chunk_hdr(rng, FOURCC_DATA, data_size); // 128..144
    assert_eq!(d.len(), 144);
    d.chunk_hdr(rng, 0x4a_4d_50_32, -32); // JUMP_BACK -> 144+16-32 = 128
    assert_eq!(d.len(), 160);
    d.pad(rng, 4 + 34 * 2); // edit_count + blocks living past 160
    (d.bytes, data_off)
}

#[test]
fn cfg_row08_negative_chunk_size() {
    let l = Libs::load();
    let rng = Rng::new(0x8888_8888);
    for i in 0..400 {
        let ds = rng.next_u64() as i64;
        let (buf, _) = build_negative_skip_doc(&rng, ds);
        l.assert_same(&format!("row08 i={i} data_size={ds:#x}"), &buf);
    }
}

// ---------------------------------------------------------------------------
// Row 9 — `data` chunk size drives `info->size` (incl. negative / extremes).
// ---------------------------------------------------------------------------
#[test]
fn cfg_row09_data_chunk_size_values() {
    let l = Libs::load();
    let rng = Rng::new(0x9999_9999);
    let mut sizes: Vec<i64> = vec![
        0,
        1,
        16,
        34,
        4096,
        i64::MAX,
        i64::MIN,
        -1,
        -16,
        -4096,
        0x7fff_ffff,
        -0x8000_0000,
        0xffff_ffffu32 as i64,
    ];
    for _ in 0..200 {
        sizes.push(rng.next_u64() as i64);
    }
    for (i, &s) in sizes.iter().enumerate() {
        let buf = valid_doc(&rng, rng.next_u64(), rng.next_u32(), rng.next_u64(), s, 2);
        l.assert_same(&format!("row09 i={i} size={s}"), &buf);
    }
}

// ---------------------------------------------------------------------------
// Row 10 — realistic sample rates stored big-endian (the ordinary case).
// ---------------------------------------------------------------------------
#[test]
fn cfg_row10_realistic_sample_rates_be() {
    let l = Libs::load();
    let rng = Rng::new(0xAAAA_AAAA);
    for (i, &r) in RATES.iter().enumerate() {
        for j in 0..40 {
            let raw = u64::from_be_bytes(r.to_be_bytes());
            let buf = valid_doc(
                &rng,
                raw,
                rng.next_u32(),
                rng.next_u64(),
                rng.next_u64() as i64,
                1,
            );
            l.assert_same(&format!("row10 rate={r} i={i} j={j}"), &buf);
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 11-14 — the `double -> unsigned long long` conversion sub-ranges.
//
// `sample_rate_raw` is written in *native* order, so the value below is exactly
// the `double` the C loads with `movsd`.
// ---------------------------------------------------------------------------
fn run_sample_rate_values(name: &str, seed: u64, values: &[f64]) {
    let l = Libs::load();
    let rng = Rng::new(seed);
    for (i, &v) in values.iter().enumerate() {
        let raw = v.to_bits();
        let buf = valid_doc(
            &rng,
            raw,
            rng.next_u32(),
            rng.next_u64(),
            rng.next_u64() as i64,
            1,
        );
        l.assert_same(&format!("{name} i={i} v={v:e} bits={raw:#018x}"), &buf);
    }
}

#[test]
fn cfg_row11_sample_rate_in_range_truncation() {
    let rng = Rng::new(0xB0B0_B0B0);
    let mut v = vec![
        1.0,
        1.5,
        1.9999999999,
        2.0,
        3.0,
        255.5,
        44100.25,
        1e6,
        1e15,
        (1u64 << 52) as f64,
        (1u64 << 52) as f64 + 0.5,
        (1u64 << 62) as f64,
        9223372036854774784.0, // largest double < 2^63
    ];
    for _ in 0..300 {
        // uniform-ish in [1, 2^63)
        let x = (rng.next_u64() >> 1) as f64;
        v.push(x.max(1.0));
    }
    run_sample_rate_values("row11", 0xB1B1_B1B1, &v);
}

#[test]
fn cfg_row12_sample_rate_ge_two_pow_63() {
    let rng = Rng::new(0xC0C0_C0C0);
    let two63 = 9223372036854775808.0f64;
    let mut v = vec![
        two63,
        two63 + 4096.0,
        two63 * 1.5,
        two63 * 1.99999,
        two63 * 2.0 - 2048.0,       // just under 2^64
        18446744073709549568.0,     // largest double < 2^64
        two63 * 2.0,                // exactly 2^64 -> out of range after subsd
        two63 * 4.0,
        1e30,
        f64::MAX,
    ];
    for _ in 0..300 {
        let m = 1.0 + (rng.next_u64() >> 11) as f64 / (1u64 << 53) as f64;
        v.push(two63 * m);
    }
    run_sample_rate_values("row12", 0xC1C1_C1C1, &v);
}

#[test]
fn cfg_row13_sample_rate_negative() {
    let rng = Rng::new(0xD0D0_D0D0);
    let two63 = 9223372036854775808.0f64;
    let mut v = vec![
        -0.0,
        -1e-300,
        -0.5,
        -0.9999999,
        -1.0,
        -1.5,
        -2.0,
        -44100.0,
        -1e18,
        -9223372036854774784.0, // > -2^63
        -two63,                 // exactly -2^63 (representable)
        -two63 - 4096.0,        // below -2^63
        -1e30,
        f64::MIN,
    ];
    for _ in 0..300 {
        let x = -((rng.next_u64() >> 1) as f64);
        v.push(x);
    }
    run_sample_rate_values("row13", 0xD1D1_D1D1, &v);
}

#[test]
fn cfg_row14_sample_rate_specials() {
    let v = vec![
        0.0,
        -0.0,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
        -f64::NAN,
        f64::from_bits(0x7ff0_0000_0000_0001), // signalling NaN
        f64::from_bits(0xfff0_0000_0000_0001),
        f64::from_bits(0x7ff8_0000_dead_beef), // quiet NaN with payload
        f64::MIN_POSITIVE,
        f64::from_bits(1),        // smallest subnormal
        f64::from_bits(0x000f_ffff_ffff_ffff), // largest subnormal
        f64::EPSILON,
        0.5,
        0.9999999999999999,
        f64::MAX,
        f64::MIN,
    ];
    run_sample_rate_values("row14", 0xE1E1_E1E1, &v);
}

// ---------------------------------------------------------------------------
// Row 15 — fully random 64-bit sample-rate patterns.
// ---------------------------------------------------------------------------
#[test]
fn cfg_row15_sample_rate_random_bits() {
    let l = Libs::load();
    let rng = Rng::new(0xF00D_F00D);
    for i in 0..10_000 {
        let raw = rng.next_u64();
        let buf = valid_doc(
            &rng,
            raw,
            rng.next_u32(),
            rng.next_u64(),
            rng.next_u64() as i64,
            0,
        );
        l.assert_same(&format!("row15 i={i} bits={raw:#018x}"), &buf);
    }
}

// ---------------------------------------------------------------------------
// Row 16 — channels_per_frame values.
// ---------------------------------------------------------------------------
#[test]
fn cfg_row16_channel_counts() {
    let l = Libs::load();
    let rng = Rng::new(0x1616_1616);
    let mut chans: Vec<u32> = vec![0, 1, 2, 3, 6, 8, 0x7fff_ffff, 0x8000_0000, 0xffff_ffff, 0xffff];
    for _ in 0..300 {
        chans.push(rng.next_u32());
    }
    for (i, &c) in chans.iter().enumerate() {
        let buf = valid_doc(&rng, rng.next_u64(), c, rng.next_u64(), 34, 1);
        l.assert_same(&format!("row16 i={i} ch={c}"), &buf);
    }
}

// ---------------------------------------------------------------------------
// Row 17 — frame_count values.
// ---------------------------------------------------------------------------
#[test]
fn cfg_row17_frame_counts() {
    let l = Libs::load();
    let rng = Rng::new(0x1717_1717);
    let mut fcs: Vec<u64> = vec![
        0,
        1,
        i64::MAX as u64,
        u64::MAX,
        i64::MIN as u64,
        0x8000_0000_0000_0000,
        0x0000_0000_ffff_ffff,
        0xffff_ffff_0000_0000,
    ];
    for _ in 0..300 {
        fcs.push(rng.next_u64());
    }
    for (i, &f) in fcs.iter().enumerate() {
        let buf = valid_doc(&rng, rng.next_u64(), rng.next_u32(), f, 34, 1);
        l.assert_same(&format!("row17 i={i} fc={f:#x}"), &buf);
    }
}

// ---------------------------------------------------------------------------
// Row 18 — `blocks` output pointer identity: must be data_chunk_offset + 20.
// ---------------------------------------------------------------------------
#[test]
fn cfg_row18_blocks_pointer_identity() {
    let l = Libs::load();
    let rng = Rng::new(0x1818_1818);
    for i in 0..200 {
        let n_unknown = rng.below(5);
        let mut d = Doc::new(&rng);
        d.desc(&rng, rng.next_u64(), FOURCC_IMA4, rng.next_u32());
        d.pakt(&rng, rng.next_u64());
        for _ in 0..n_unknown {
            let body = rng.below(33);
            d.unknown(&rng, body, body as i64);
        }
        let data_off = d.data(&rng, 34, 2);
        let buf = d.bytes;

        let ((crc, cb), (rrc, rb)) = l.call_both(&buf, 0x5A);
        assert_eq!(crc, 0, "row18 i={i}: C should accept");
        assert_eq!(rrc, 0, "row18 i={i}: Rust should accept");
        assert_eq!(cb, rb, "row18 i={i}: info mismatch");
        let expect = buf.as_ptr() as usize + data_off + CHUNK_HDR + 4;
        let got = u64::from_le_bytes(cb[0..8].try_into().unwrap()) as usize;
        assert_eq!(
            got, expect,
            "row18 i={i}: blocks pointer should be data_chunk+20"
        );
    }
}

// ---------------------------------------------------------------------------
// Row 19 — misaligned `data` buffers (offsets 0..7 from a 16-aligned base).
// ---------------------------------------------------------------------------
#[test]
fn cfg_row19_misaligned_buffers() {
    let l = Libs::load();
    let rng = Rng::new(0x1919_1919);
    for off in 0..8usize {
        for i in 0..100 {
            let doc = valid_doc(
                &rng,
                rng.next_u64(),
                rng.next_u32(),
                rng.next_u64(),
                rng.next_u64() as i64,
                2,
            );
            let m = Misaligned::new(&doc, off);
            assert_eq!(m.addr_mod8(), off % 8, "misalignment setup failed");
            l.assert_same(&format!("row19 off={off} i={i}"), m.as_slice());
        }
    }
}

// ---------------------------------------------------------------------------
// Row 20 — block payload region contents are irrelevant; 0 / 1 / many blocks.
// ---------------------------------------------------------------------------
#[test]
fn cfg_row20_block_payload_ignored() {
    let l = Libs::load();
    let rng = Rng::new(0x2020_2020);
    for &n in &[0usize, 1, 2, 7, 64] {
        for i in 0..60 {
            let buf = valid_doc(
                &rng,
                rng.next_u64(),
                rng.next_u32(),
                rng.next_u64(),
                (n * 34) as i64,
                n,
            );
            l.assert_same(&format!("row20 n={n} i={i}"), &buf);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 21 — structural fuzz: random valid documents, all axes at once.
// ---------------------------------------------------------------------------
#[test]
fn cfg_row21_structural_fuzz() {
    let l = Libs::load();
    let rng = Rng::new(0x2121_2121);
    for i in 0..5_000 {
        let mut d = Doc::new(&rng);
        // Random ordering of desc/pakt with random unknown chunks sprinkled in.
        let desc_first = rng.next_u64() & 1 == 0;
        let sprinkle = |d: &mut Doc, rng: &Rng| {
            for _ in 0..rng.below(4) {
                let body = rng.below(40);
                d.unknown(rng, body, body as i64);
            }
        };
        sprinkle(&mut d, &rng);
        if desc_first {
            d.desc(&rng, rng.next_u64(), FOURCC_IMA4, rng.next_u32());
            sprinkle(&mut d, &rng);
            d.pakt(&rng, rng.next_u64());
        } else {
            d.pakt(&rng, rng.next_u64());
            sprinkle(&mut d, &rng);
            d.desc(&rng, rng.next_u64(), FOURCC_IMA4, rng.next_u32());
        }
        sprinkle(&mut d, &rng);
        d.data(&rng, rng.next_u64() as i64, rng.below(4));
        l.assert_same(&format!("row21 i={i}"), &d.bytes);
    }
}

// ---------------------------------------------------------------------------
// Row 22 — `data` chunk physically BEFORE `desc`, reached by a back-jump so
//          that `desc`/`pakt` are still populated first.
//
//  off   8 : JUMP_FWD (size 48)  -> next 72
//  off  24 : data chunk header (skipped over on the way out)
//  off  72 : desc (size 32)      -> next 120
//  off 120 : pakt (size 24)      -> next 160
//  off 160 : JUMP_BACK (size -152) -> next 24 == data => break
// ---------------------------------------------------------------------------
#[test]
fn cfg_row22_data_before_desc_via_backjump() {
    let l = Libs::load();
    let rng = Rng::new(0x2222_0022);
    for i in 0..300 {
        let ds = rng.next_u64() as i64;
        let mut d = Doc::new(&rng);
        d.chunk_hdr(&rng, 0x4a_4d_50_41, 48); // 8..24, -> 8+16+48 = 72
        assert_eq!(d.len(), 24);
        let data_off = d.len();
        d.chunk_hdr(&rng, FOURCC_DATA, ds); // 24..40
        d.pad(&rng, 32); // 40..72 (part of the skipped 48 bytes)
        assert_eq!(d.len(), 72);
        d.desc(&rng, rng.next_u64(), FOURCC_IMA4, rng.next_u32()); // 72..120
        assert_eq!(d.len(), 120);
        d.pakt(&rng, rng.next_u64()); // 120..160
        assert_eq!(d.len(), 160);
        d.chunk_hdr(&rng, 0x4a_4d_50_42, -152); // 160..176, -> 160+16-152 = 24
        assert_eq!(d.len(), 176);

        let buf = d.bytes;
        let ((crc, cb), (rrc, rb)) = l.call_both(&buf, 0x3C);
        assert_eq!(crc, rrc, "row22 i={i}: rc mismatch {crc} vs {rrc}");
        assert_eq!(cb, rb, "row22 i={i}: info mismatch");
        assert_eq!(crc, 0, "row22 i={i}: expected success");
        let expect = buf.as_ptr() as usize + data_off + CHUNK_HDR + 4;
        let got = u64::from_le_bytes(cb[0..8].try_into().unwrap()) as usize;
        assert_eq!(got, expect, "row22 i={i}: blocks pointer");
        assert_eq!(
            u64::from_le_bytes(cb[8..16].try_into().unwrap()),
            ds as u64,
            "row22 i={i}: info->size"
        );
    }
}

// ---------------------------------------------------------------------------
// Row 23 — on the error paths `*info` must be left exactly as the caller had it.
// ---------------------------------------------------------------------------
#[test]
fn cfg_row23_info_untouched_on_error() {
    let l = Libs::load();
    let rng = Rng::new(0x2323_2323);
    for i in 0..300 {
        // bad magic
        let mut d = Doc::with_header(&rng, rng.next_u32() | 0x8000_0000, 1);
        d.pad(&rng, 64);
        for poison in [0x00u8, 0xFF, 0xA5, 0x5A] {
            let ((crc, cb), (rrc, rb)) = l.call_both(&d.bytes, poison);
            assert_eq!((crc, cb), (rrc, rb), "row23 magic i={i} poison={poison:#x}");
            assert_eq!(crc, -1);
            assert_eq!(cb, ImaInfo::poisoned(poison).raw(), "info was modified");
        }
        // bad version
        let mut d = Doc::with_header(&rng, FOURCC_CAFF, rng.next_u16() | 2);
        d.pad(&rng, 64);
        for poison in [0x00u8, 0xFF, 0xA5] {
            let ((crc, cb), (rrc, rb)) = l.call_both(&d.bytes, poison);
            assert_eq!((crc, cb), (rrc, rb), "row23 version i={i}");
            assert_eq!(crc, -2);
            assert_eq!(cb, ImaInfo::poisoned(poison).raw(), "info was modified");
        }
        // bad format id
        let mut bad = rng.next_u32();
        if bad == FOURCC_IMA4 {
            bad ^= 1;
        }
        let mut d = Doc::new(&rng);
        d.desc(&rng, rng.next_u64(), bad, rng.next_u32());
        d.pakt(&rng, rng.next_u64());
        d.data(&rng, 34, 1);
        for poison in [0x00u8, 0xFF, 0xA5] {
            let ((crc, cb), (rrc, rb)) = l.call_both(&d.bytes, poison);
            assert_eq!((crc, cb), (rrc, rb), "row23 fmt i={i}");
            assert_eq!(crc, -3);
            assert_eq!(cb, ImaInfo::poisoned(poison).raw(), "info was modified");
        }
    }
}

// ---------------------------------------------------------------------------
// Row 24 — statelessness: alternating valid/invalid calls reusing one `info`.
// ---------------------------------------------------------------------------
#[test]
fn cfg_row24_repeated_calls_stateless() {
    let l = Libs::load();
    let rng = Rng::new(0x2424_2424);
    let mut ci = ImaInfo::poisoned(0x11);
    let mut ri = ImaInfo::poisoned(0x11);
    for i in 0..300 {
        let buf: Vec<u8> = match i % 4 {
            0 => valid_doc(&rng, rng.next_u64(), rng.next_u32(), rng.next_u64(), 34, 1),
            1 => {
                let mut d = Doc::with_header(&rng, rng.next_u32() | 1, 1);
                d.pad(&rng, 32);
                d.bytes
            }
            2 => {
                let mut d = Doc::with_header(&rng, FOURCC_CAFF, 0);
                d.pad(&rng, 32);
                d.bytes
            }
            _ => {
                let mut d = Doc::new(&rng);
                d.desc(&rng, rng.next_u64(), 0, rng.next_u32());
                d.pakt(&rng, rng.next_u64());
                d.data(&rng, 34, 1);
                d.bytes
            }
        };
        let p = buf.as_ptr() as *const std::ffi::c_void;
        let crc = unsafe { (l.c_parse)(&mut ci, p) };
        let rrc = unsafe { (l.r_parse)(&mut ri, p) };
        assert_eq!(crc, rrc, "row24 i={i}: rc mismatch");
        assert_eq!(ci.raw(), ri.raw(), "row24 i={i}: info mismatch");
    }
}
