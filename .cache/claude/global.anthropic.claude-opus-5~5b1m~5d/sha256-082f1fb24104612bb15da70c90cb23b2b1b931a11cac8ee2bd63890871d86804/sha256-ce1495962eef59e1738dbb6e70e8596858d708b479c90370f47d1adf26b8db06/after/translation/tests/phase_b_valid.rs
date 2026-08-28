//! Phase B — valid-path differential tests, one test per row of `CONFIGS.md`.
//!
//! Every test drives BOTH shared objects through `dlopen`/`dlsym` with the same
//! input buffer and compares the `int` return value plus all 40 bytes of
//! `struct ima_info` (padding included) byte for byte.

mod support;

use support::*;

/// Runs `iters` seeded-random iterations of one `CONFIGS.md` row.
///
/// `f` returns `(buffer, misalignment offset)`.  `expect` (when given) is
/// asserted against the C return value so that a row can never pass vacuously
/// (e.g. because a "valid" file actually turned out to be rejected).
fn row(
    name: &str,
    seed: u64,
    iters: usize,
    expect: Option<i32>,
    mut f: impl FnMut(&mut Rng) -> (Vec<u8>, usize),
) {
    let mut rng = Rng::new(seed);
    for i in 0..iters {
        let (bytes, off) = f(&mut rng);
        let buf = AlignedBuf::new(&bytes, off);
        let o = assert_same(&format!("{name} iter={i}"), &bytes, buf.ptr());
        if let Some(e) = expect {
            assert_eq!(
                o.c_ret, e,
                "{name} iter={i}: C returned {} but the row expects {e} \
                 (the test input is not exercising the intended path)",
                o.c_ret
            );
        }
    }
}

/// The canonical minimal valid file: `desc`, `pakt`, `data`.
fn minimal_valid(rng: &mut Rng) -> Vec<u8> {
    let sr = rng.u64();
    let ch = rng.u32();
    let fc = rng.u64();
    let ds = rng.u64() as i64;
    let mut b = FileBuilder::valid_header(Rng::new(rng.u64()));
    b.desc(sr, FMT_IMA4, ch);
    b.pakt(fc);
    b.data(ds, rng.range_usize(4, 40));
    b.finish()
}

// ---------------------------------------------------------------------------
// Row 1 — invalid magic (axis A✗)
// ---------------------------------------------------------------------------
#[test]
fn cfg01_bad_magic_randomized() {
    row("cfg01", 0x1111_0001, 20_000, Some(-1), |rng| {
        let magic = loop {
            let m = rng.fourcc();
            if m != MAGIC_CAFF {
                break m;
            }
        };
        let ver = rng.u16();
        let mut b = FileBuilder::new(Rng::new(rng.u64()), magic, ver);
        b.raw(rng.range_usize(0, 64));
        (b.finish(), 0)
    });
}

// ---------------------------------------------------------------------------
// Row 2 — valid magic, invalid version (axis A✓ + B✗)
// ---------------------------------------------------------------------------
#[test]
fn cfg02_bad_version_randomized() {
    row("cfg02", 0x1111_0002, 20_000, Some(-2), |rng| {
        let ver = loop {
            let v = rng.u16();
            if v != 1 {
                break v;
            }
        };
        let mut b = FileBuilder::new(Rng::new(rng.u64()), MAGIC_CAFF, ver);
        b.raw(rng.range_usize(0, 64));
        (b.finish(), 0)
    });
}

// ---------------------------------------------------------------------------
// Row 3 — exhaustive over all 65 536 `version` values (also ERRORS.md row 11)
// ---------------------------------------------------------------------------
#[test]
fn cfg03_version_exhaustive() {
    let mut rng = Rng::new(0x1111_0003);
    for v in 0u32..=0xFFFF {
        let ver = v as u16;
        let sr = rng.u64();
        let ch = rng.u32();
        let fc = rng.u64();
        let ds = rng.u64() as i64;
        let mut b = FileBuilder::new(Rng::new(rng.u64()), MAGIC_CAFF, ver);
        b.desc(sr, FMT_IMA4, ch);
        b.pakt(fc);
        b.data(ds, 8);
        let bytes = b.finish();
        let buf = AlignedBuf::aligned(&bytes);
        let o = assert_same(&format!("cfg03 version={ver}"), &bytes, buf.ptr());
        let expect = if ver == 1 { 0 } else { -2 };
        assert_eq!(o.c_ret, expect, "cfg03 version={ver}");
    }
}

// ---------------------------------------------------------------------------
// Row 4 — minimal valid file (desc, pakt, data)
// ---------------------------------------------------------------------------
#[test]
fn cfg04_minimal_valid() {
    row("cfg04", 0x1111_0004, 5_000, Some(0), |rng| {
        (minimal_valid(rng), 0)
    });
}

// ---------------------------------------------------------------------------
// Row 5 — order pakt, desc, data (axis E)
// ---------------------------------------------------------------------------
#[test]
fn cfg05_order_pakt_desc_data() {
    row("cfg05", 0x1111_0005, 5_000, Some(0), |rng| {
        let sr = rng.u64();
        let ch = rng.u32();
        let fc = rng.u64();
        let ds = rng.u64() as i64;
        let mut b = FileBuilder::valid_header(Rng::new(rng.u64()));
        b.pakt(fc);
        b.desc(sr, FMT_IMA4, ch);
        b.data(ds, rng.range_usize(4, 40));
        (b.finish(), 0)
    });
}

// ---------------------------------------------------------------------------
// Row 6 — one unknown chunk before desc (axis G=1, axis D fall-through)
// ---------------------------------------------------------------------------
#[test]
fn cfg06_one_unknown_chunk_first() {
    row("cfg06", 0x1111_0006, 5_000, Some(0), |rng| {
        let t = rng.unknown_fourcc();
        let n = rng.range_usize(0, 48);
        let sr = rng.u64();
        let ch = rng.u32();
        let fc = rng.u64();
        let ds = rng.u64() as i64;
        let mut b = FileBuilder::valid_header(Rng::new(rng.u64()));
        b.unknown(t, n);
        b.desc(sr, FMT_IMA4, ch);
        b.pakt(fc);
        b.data(ds, rng.range_usize(4, 40));
        (b.finish(), 0)
    });
}

// ---------------------------------------------------------------------------
// Row 7 — 1..8 unknown chunks interleaved at random positions (axis G=many)
// ---------------------------------------------------------------------------
#[derive(Clone, Copy, PartialEq)]
enum Item {
    Desc,
    Pakt,
    Unknown,
}

#[test]
fn cfg07_many_unknown_chunks_interleaved() {
    row("cfg07", 0x1111_0007, 5_000, Some(0), |rng| {
        let k = rng.range_usize(1, 8);
        let mut items = vec![Item::Desc, Item::Pakt];
        for _ in 0..k {
            let at = rng.range_usize(0, items.len());
            items.insert(at, Item::Unknown);
        }
        let sr = rng.u64();
        let ch = rng.u32();
        let fc = rng.u64();
        let ds = rng.u64() as i64;
        let mut b = FileBuilder::valid_header(Rng::new(rng.u64()));
        for it in items {
            match it {
                Item::Desc => {
                    b.desc(sr, FMT_IMA4, ch);
                }
                Item::Pakt => {
                    b.pakt(fc);
                }
                Item::Unknown => {
                    let t = b.rng().unknown_fourcc();
                    let n = b.rng().range_usize(0, 48);
                    b.unknown(t, n);
                }
            }
        }
        b.data(ds, rng.range_usize(4, 40));
        (b.finish(), 0)
    });
}

// ---------------------------------------------------------------------------
// Row 8 — several desc chunks: the last one before `data` wins (axis F)
// ---------------------------------------------------------------------------
#[test]
fn cfg08_multiple_desc_last_wins() {
    row("cfg08", 0x1111_0008, 5_000, Some(0), |rng| {
        let k = rng.range_usize(2, 4);
        let fc = rng.u64();
        let ds = rng.u64() as i64;
        let mut b = FileBuilder::valid_header(Rng::new(rng.u64()));
        for _ in 0..k {
            let sr = rng.u64();
            let ch = rng.u32();
            // Every desc has a valid format_id so that whichever one wins the
            // parse still succeeds and the *values* are what differ.
            b.desc(sr, FMT_IMA4, ch);
        }
        b.pakt(fc);
        b.data(ds, rng.range_usize(4, 40));
        (b.finish(), 0)
    });
}

/// Same as row 8 but the *earlier* desc chunks carry an invalid `format_id`,
/// which proves that only the last one is consulted.
#[test]
fn cfg08b_multiple_desc_only_last_format_id_matters() {
    row("cfg08b", 0x1111_0018, 3_000, Some(0), |rng| {
        let k = rng.range_usize(2, 4);
        let fc = rng.u64();
        let ds = rng.u64() as i64;
        let mut b = FileBuilder::valid_header(Rng::new(rng.u64()));
        for i in 0..k {
            let sr = rng.u64();
            let ch = rng.u32();
            let fmt = if i + 1 == k {
                FMT_IMA4
            } else {
                loop {
                    let f = rng.fourcc();
                    if f != FMT_IMA4 {
                        break f;
                    }
                }
            };
            b.desc(sr, fmt, ch);
        }
        b.pakt(fc);
        b.data(ds, rng.range_usize(4, 40));
        (b.finish(), 0)
    });
}

// ---------------------------------------------------------------------------
// Row 9 — several pakt chunks: the last one before `data` wins (axis F)
// ---------------------------------------------------------------------------
#[test]
fn cfg09_multiple_pakt_last_wins() {
    row("cfg09", 0x1111_0009, 5_000, Some(0), |rng| {
        let k = rng.range_usize(2, 4);
        let sr = rng.u64();
        let ch = rng.u32();
        let ds = rng.u64() as i64;
        let mut b = FileBuilder::valid_header(Rng::new(rng.u64()));
        b.desc(sr, FMT_IMA4, ch);
        for _ in 0..k {
            let fc = rng.u64();
            b.pakt(fc);
        }
        b.data(ds, rng.range_usize(4, 40));
        (b.finish(), 0)
    });
}

// ---------------------------------------------------------------------------
// Row 10 — two `data` chunks: the scan breaks on the first (axis E/F)
// ---------------------------------------------------------------------------
#[test]
fn cfg10_two_data_chunks_first_wins() {
    row("cfg10", 0x1111_000A, 2_000, Some(0), |rng| {
        let sr = rng.u64();
        let ch = rng.u32();
        let fc = rng.u64();
        let ds1 = rng.u64() as i64;
        let ds2 = rng.u64() as i64;
        let mut b = FileBuilder::valid_header(Rng::new(rng.u64()));
        b.desc(sr, FMT_IMA4, ch);
        b.pakt(fc);
        b.data(ds1, 24);
        // Never reached: a second desc/pakt/data with completely different
        // values that must NOT influence the result.
        let sr2 = rng.u64();
        let ch2 = rng.u32();
        let fc2 = rng.u64();
        b.desc(sr2, FMT_IMA4, ch2);
        b.pakt(fc2);
        b.data(ds2, 24);
        (b.finish(), 0)
    });
}

// ---------------------------------------------------------------------------
// Row 11 — non-zero positive skipped-chunk sizes: stride is 16 + size (axis H/I)
// ---------------------------------------------------------------------------
#[test]
fn cfg11_positive_skip_sizes() {
    row("cfg11", 0x1111_000B, 5_000, Some(0), |rng| {
        let sr = rng.u64();
        let ch = rng.u32();
        let fc = rng.u64();
        let ds = rng.u64() as i64;
        let mut b = FileBuilder::valid_header(Rng::new(rng.u64()));
        // A run of skipped chunks with sizes 0..=96, including 0 (chunks packed
        // back-to-back at the bare 16-byte stride).
        for _ in 0..rng.range_usize(1, 6) {
            let t = rng.unknown_fourcc();
            let n = rng.range_usize(0, 96);
            b.unknown(t, n);
        }
        b.desc(sr, FMT_IMA4, ch);
        for _ in 0..rng.range_usize(0, 3) {
            let t = rng.unknown_fourcc();
            let n = rng.range_usize(0, 96);
            b.unknown(t, n);
        }
        b.pakt(fc);
        b.data(ds, rng.range_usize(4, 40));
        (b.finish(), 0)
    });
}

// ---------------------------------------------------------------------------
// Row 12 — NEGATIVE chunk size: the scan walks backwards (axis H)
//
// Layout (offsets are exact):
//     0    header (8)
//     8    desc   hdr(16) + payload(32), declared 32   -> next = 56
//    56    jmpf   hdr(16),               declared 80   -> next = 152
//    72    pakt   hdr(16) + payload(24), declared 24   -> next = 112
//   112    data   hdr(16) + payload(24), declared ds
//   152    jmpb   hdr(16),               declared -96  -> next = 72
//
// Scan order: 8 (desc) -> 56 (jmpf) -> 152 (jmpb) -> 72 (pakt) -> 112 (data).
// ---------------------------------------------------------------------------
#[test]
fn cfg12_negative_chunk_size_walks_backwards() {
    row("cfg12", 0x1111_000C, 3_000, Some(0), |rng| {
        let sr = rng.u64();
        let ch = rng.u32();
        let fc = rng.u64();
        let ds = rng.u64() as i64;
        let t1 = rng.unknown_fourcc();
        let t2 = rng.unknown_fourcc();
        let mut b = FileBuilder::valid_header(Rng::new(rng.u64()));
        assert_eq!(b.offset(), 8);
        b.desc(sr, FMT_IMA4, ch);
        assert_eq!(b.offset(), 56);
        b.unknown_sized(t1, 80, 0);
        assert_eq!(b.offset(), 72);
        b.pakt(fc);
        assert_eq!(b.offset(), 112);
        b.data(ds, 24);
        assert_eq!(b.offset(), 152);
        b.unknown_sized(t2, -96, 0);
        assert_eq!(b.offset(), 168);
        let data_off = b.data_off.unwrap();
        assert_eq!(data_off, 112);
        let bytes = b.finish();
        (bytes, 0)
    });
}

/// A second negative-size shape: the backwards jump lands directly on the
/// `data` chunk, which physically precedes the jump chunk in the buffer.
///
///     0    header (8)
///     8    desc  hdr(16) + payload(32), declared 32  -> next = 56
///    56    pakt  hdr(16) + payload(24), declared 88  -> next = 56+16+88 = 160
///    96    data  hdr(16) (patched into the filler)   -> break, blocks = 116
///   160    jmpb  hdr(16),               declared -80 -> next = 160+16-80 = 96
///
/// Scan order: 8 (desc) -> 56 (pakt) -> 160 (jmpb) -> 96 (data).
#[test]
fn cfg12b_negative_chunk_size_jumps_back_onto_data() {
    row("cfg12b", 0x1111_001C, 3_000, Some(0), |rng| {
        let sr = rng.u64();
        let ch = rng.u32();
        let fc = rng.u64();
        let ds = rng.u64() as i64;
        let t = rng.unknown_fourcc();
        let mut b = FileBuilder::valid_header(Rng::new(rng.u64()));
        assert_eq!(b.offset(), 8);
        b.desc(sr, FMT_IMA4, ch);
        assert_eq!(b.offset(), 56);
        b.pakt(fc);
        assert_eq!(b.offset(), 96);
        b.raw(64); // filler 96..160, the data chunk header is patched in below
        assert_eq!(b.offset(), 160);
        b.unknown_sized(t, -80, 0);
        assert_eq!(b.offset(), 176);
        let mut bytes = b.finish();
        // pakt declares 88 instead of 24, so its stride jumps *over* the data
        // chunk to the backwards-jump chunk at 160.
        bytes[56 + 8..56 + 16].copy_from_slice(&88u64.to_be_bytes());
        // The data chunk header at 96 (type BE fourcc, size at +8 BE).
        bytes[96..100].copy_from_slice(&T_DATA);
        bytes[104..112].copy_from_slice(&(ds as u64).to_be_bytes());
        (bytes, 0)
    });
}

// ---------------------------------------------------------------------------
// Row 13 — `data` chunk size extremes -> info->size (axis H/Q)
// ---------------------------------------------------------------------------
#[test]
fn cfg13_data_chunk_size_extremes() {
    const SIZES: &[i64] = &[
        0,
        1,
        -1,
        2,
        -2,
        16,
        -16,
        i64::MIN,
        i64::MAX,
        i64::MIN + 1,
        i64::MAX - 1,
        -1i64 << 32,
        1i64 << 32,
        0x00FF_00FF_00FF_00FF,
        0x7F00_0000_0000_0000,
        -0x7F00_0000_0000_0000,
    ];
    let mut rng = Rng::new(0x1111_000D);
    for i in 0..5_000usize {
        let ds = if i < SIZES.len() {
            SIZES[i]
        } else {
            rng.u64() as i64
        };
        let sr = rng.u64();
        let ch = rng.u32();
        let fc = rng.u64();
        let mut b = FileBuilder::valid_header(Rng::new(rng.u64()));
        b.desc(sr, FMT_IMA4, ch);
        b.pakt(fc);
        b.data(ds, 8);
        let bytes = b.finish();
        let buf = AlignedBuf::aligned(&bytes);
        let o = assert_same(&format!("cfg13 i={i} ds={ds}"), &bytes, buf.ptr());
        assert_eq!(o.c_ret, 0);
        assert_eq!(
            o.c_info.size(),
            ds as u64,
            "cfg13 i={i}: info->size must be the s64->u64 reinterpretation of {ds}"
        );
    }
}

// ---------------------------------------------------------------------------
// Row 14 — unread bytes (axis C/I/O) must not influence either implementation
// ---------------------------------------------------------------------------
#[test]
fn cfg14_unread_bytes_are_ignored() {
    let mut rng = Rng::new(0x1111_000E);
    for i in 0..5_000usize {
        let sr = rng.u64();
        let ch = rng.u32();
        let fc = rng.u64();
        let ds = rng.u64() as i64;
        // Two files that are semantically identical but differ in every byte
        // the C code never reads (header->flags, chunk padding, format_flags,
        // bytes/frames_per_packet, bits_per_channel, packet_count,
        // priming/remainder_frames, caf_data->edit_count, block payloads).
        let mk = |noise_seed: u64| {
            let mut b = FileBuilder::valid_header(Rng::new(noise_seed));
            b.desc(sr, FMT_IMA4, ch);
            b.pakt(fc);
            b.data(ds, 34);
            b.finish()
        };
        let a = mk(rng.u64());
        let c = mk(rng.u64());
        assert_ne!(a, c, "cfg14 i={i}: the two noise fillings should differ");
        assert_eq!(a.len(), c.len());

        let ba = AlignedBuf::aligned(&a);
        let bc = AlignedBuf::aligned(&c);
        let oa = assert_same(&format!("cfg14 i={i} noiseA"), &a, ba.ptr());
        let oc = assert_same(&format!("cfg14 i={i} noiseB"), &c, bc.ptr());
        assert_eq!(oa.c_ret, 0);
        assert_eq!(oc.c_ret, 0);
        // Everything except the `blocks` pointer (different allocations) must
        // be identical between the two noise fillings, in BOTH libraries.
        for (x, y) in [(&oa.c_info, &oc.c_info), (&oa.r_info, &oc.r_info)] {
            assert_eq!(x.size(), y.size(), "cfg14 i={i} size");
            assert_eq!(
                x.sample_rate_bits(),
                y.sample_rate_bits(),
                "cfg14 i={i} sample_rate"
            );
            assert_eq!(x.frame_count(), y.frame_count(), "cfg14 i={i} frame_count");
            assert_eq!(
                x.channel_count(),
                y.channel_count(),
                "cfg14 i={i} channel_count"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Row 15 — pakt->frame_count (axis M, exercises ima_btoh64)
// ---------------------------------------------------------------------------
#[test]
fn cfg15_frame_count_values() {
    const FCS: &[u64] = &[
        0,
        1,
        2,
        u64::MAX,
        u64::MAX - 1,
        i64::MAX as u64,
        i64::MIN as u64,
        0x00FF_00FF_00FF_00FF,
        0xFF00_FF00_FF00_FF00,
        0x0102_0304_0506_0708,
        0x8080_8080_8080_8080,
        1 << 63,
        (1 << 63) | 1,
        0xDEAD_BEEF_CAFE_BABE,
    ];
    let mut rng = Rng::new(0x1111_000F);
    for i in 0..5_000usize {
        let fc = if i < FCS.len() { FCS[i] } else { rng.u64() };
        let sr = rng.u64();
        let ch = rng.u32();
        let ds = rng.u64() as i64;
        let mut b = FileBuilder::valid_header(Rng::new(rng.u64()));
        b.desc(sr, FMT_IMA4, ch);
        b.pakt(fc);
        b.data(ds, 8);
        let bytes = b.finish();
        let buf = AlignedBuf::aligned(&bytes);
        let o = assert_same(&format!("cfg15 i={i} fc=0x{fc:016x}"), &bytes, buf.ptr());
        assert_eq!(o.c_ret, 0);
        assert_eq!(o.c_info.frame_count(), fc, "cfg15 i={i}");
    }
}

// ---------------------------------------------------------------------------
// Row 16 — misaligned `data` pointer, offsets 1..7 (axis N)
// ---------------------------------------------------------------------------
#[test]
fn cfg16_misaligned_buffer() {
    for off in 0..8usize {
        row(
            &format!("cfg16 off={off}"),
            0x1111_0010 + off as u64,
            2_000,
            Some(0),
            |rng| (minimal_valid(rng), off),
        );
    }
}

// ---------------------------------------------------------------------------
// Row 17 — invalid format_id (axis J✗) -> -3, info untouched
// ---------------------------------------------------------------------------
#[test]
fn cfg17_bad_format_id_randomized() {
    let mut rng = Rng::new(0x1111_0011);
    let curated: &[[u8; 4]] = &[
        *b"ima3", *b"ima5", *b"IMA4", *b"4ami", *b"Ima4", *b"ima\0", *b"\0ima", *b"    ",
    ];
    for i in 0..20_000usize {
        let fmt = if i < curated.len() {
            curated[i]
        } else {
            loop {
                let f = rng.fourcc();
                if f != FMT_IMA4 {
                    break f;
                }
            }
        };
        let sr = rng.u64();
        let ch = rng.u32();
        let fc = rng.u64();
        let ds = rng.u64() as i64;
        let mut b = FileBuilder::valid_header(Rng::new(rng.u64()));
        b.desc(sr, fmt, ch);
        b.pakt(fc);
        b.data(ds, 8);
        let bytes = b.finish();
        let buf = AlignedBuf::aligned(&bytes);
        let o = assert_same(&format!("cfg17 i={i} fmt={fmt:?}"), &bytes, buf.ptr());
        assert_eq!(o.c_ret, -3, "cfg17 i={i}");
        assert_eq!(
            o.c_info,
            InfoBytes::sentinel(),
            "cfg17 i={i}: C must not write to *info on the -3 path"
        );
    }
}

// ---------------------------------------------------------------------------
// Row 18 — desc->channels_per_frame (axis L, exercises ima_btoh32)
// ---------------------------------------------------------------------------
#[test]
fn cfg18_channel_count_values() {
    const CHS: &[u32] = &[
        0,
        1,
        2,
        3,
        4,
        6,
        8,
        0xFFFF_FFFF,
        0xFFFF_FFFE,
        0x7FFF_FFFF,
        0x8000_0000,
        0x0000_00FF,
        0xFF00_0000,
        0x0102_0304,
        0xDEAD_BEEF,
    ];
    let mut rng = Rng::new(0x1111_0012);
    for i in 0..5_000usize {
        let ch = if i < CHS.len() { CHS[i] } else { rng.u32() };
        let sr = rng.u64();
        let fc = rng.u64();
        let ds = rng.u64() as i64;
        let mut b = FileBuilder::valid_header(Rng::new(rng.u64()));
        b.desc(sr, FMT_IMA4, ch);
        b.pakt(fc);
        b.data(ds, 8);
        let bytes = b.finish();
        let buf = AlignedBuf::aligned(&bytes);
        let o = assert_same(&format!("cfg18 i={i} ch=0x{ch:08x}"), &bytes, buf.ptr());
        assert_eq!(o.c_ret, 0);
        assert_eq!(o.c_info.channel_count(), ch, "cfg18 i={i}");
    }
}

// ---------------------------------------------------------------------------
// Row 19 — sample_rate: arbitrary random double bit patterns (axis K)
//
// This is the row that exercises the `(ima_u64_t)double` *value* conversion in
// `lib.c:127` (comisd / jae / subsd / cvttsd2si / xor 2^63) followed by the
// byte swap and the reinterpretation back to `double`.
// ---------------------------------------------------------------------------
#[test]
fn cfg19_sample_rate_random_bit_patterns() {
    row("cfg19", 0x1111_0013, 30_000, Some(0), |rng| {
        let sr = rng.u64();
        let ch = rng.u32();
        let fc = rng.u64();
        let ds = rng.u64() as i64;
        let mut b = FileBuilder::valid_header(Rng::new(rng.u64()));
        b.desc(sr, FMT_IMA4, ch);
        b.pakt(fc);
        b.data(ds, 8);
        (b.finish(), 0)
    });
}

/// Random doubles biased towards the *interesting* magnitudes for the
/// `double -> u64` conversion (around and across 2^63, negative values,
/// fractional values), which uniform random bit patterns almost never produce.
#[test]
fn cfg19b_sample_rate_biased_magnitudes() {
    row("cfg19b", 0x1111_0023, 30_000, Some(0), |rng| {
        let pick = rng.below(8);
        let m = rng.u64();
        let f = match pick {
            0 => (m % 2_000_000) as f64 + (m as f64 / 1e19), // small positive w/ fraction
            1 => -((m % 2_000_000) as f64) - (m as f64 / 1e19), // small negative
            2 => 9223372036854775808.0 * (1.0 + (m as i32 as f64) / 2.147e12), // straddle 2^63
            3 => -9223372036854775808.0 * (1.0 + (m as i32 as f64) / 2.147e12), // straddle -2^63
            4 => m as f64, // huge, usually above 2^63
            5 => -(m as f64),
            6 => (m % 1024) as f64 / 1024.0, // in (0,1): truncates to 0
            _ => -((m % 1024) as f64) / 1024.0, // in (-1,0): truncates to 0/-0
        };
        let sr = f.to_bits();
        let ch = rng.u32();
        let fc = rng.u64();
        let ds = rng.u64() as i64;
        let mut b = FileBuilder::valid_header(Rng::new(rng.u64()));
        b.desc(sr, FMT_IMA4, ch);
        b.pakt(fc);
        b.data(ds, 8);
        (b.finish(), 0)
    });
}

// ---------------------------------------------------------------------------
// Row 20 — sample_rate: curated hard doubles (axis K, all three HW paths)
// ---------------------------------------------------------------------------
#[test]
fn cfg20_sample_rate_hard_doubles() {
    let mut rng = Rng::new(0x1111_0014);
    let bits: Vec<u64> = HARD_DOUBLES
        .iter()
        .map(|d| d.to_bits())
        .chain(HARD_DOUBLE_BITS.iter().copied())
        .collect();
    for (i, &sr) in bits.iter().enumerate() {
        // Repeat each value with several different noise fillings and channel /
        // frame-count values.
        for rep in 0..8 {
            let ch = rng.u32();
            let fc = rng.u64();
            let ds = rng.u64() as i64;
            let mut b = FileBuilder::valid_header(Rng::new(rng.u64()));
            b.desc(sr, FMT_IMA4, ch);
            b.pakt(fc);
            b.data(ds, 8);
            let bytes = b.finish();
            let buf = AlignedBuf::new(&bytes, rep % 8);
            let o = assert_same(
                &format!(
                    "cfg20 i={i} rep={rep} sr_bits=0x{sr:016x} ({})",
                    f64::from_bits(sr)
                ),
                &bytes,
                buf.ptr(),
            );
            assert_eq!(o.c_ret, 0);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 21 — K x L x M x Q cross-product in randomly shaped chunk streams
// ---------------------------------------------------------------------------
#[test]
fn cfg21_cross_product() {
    row("cfg21", 0x1111_0015, 20_000, Some(0), |rng| {
        let sr = match rng.below(4) {
            0 => rng.u64(),
            1 => HARD_DOUBLES[rng.below(HARD_DOUBLES.len() as u64) as usize].to_bits(),
            2 => HARD_DOUBLE_BITS[rng.below(HARD_DOUBLE_BITS.len() as u64) as usize],
            _ => ((rng.below(200_000) as f64) + 0.5).to_bits(),
        };
        let ch = match rng.below(3) {
            0 => rng.u32(),
            1 => rng.below(9) as u32,
            _ => 0xFFFF_FFFF - rng.below(4) as u32,
        };
        let fc = match rng.below(3) {
            0 => rng.u64(),
            1 => rng.below(1 << 20),
            _ => u64::MAX - rng.below(4),
        };
        let ds = match rng.below(4) {
            0 => rng.u64() as i64,
            1 => rng.below(1 << 20) as i64,
            2 => i64::MIN + rng.below(4) as i64,
            _ => i64::MAX - rng.below(4) as i64,
        };
        let desc_first = rng.below(2) == 0;
        let mut b = FileBuilder::valid_header(Rng::new(rng.u64()));
        for _ in 0..rng.below(3) {
            let t = b.rng().unknown_fourcc();
            let n = b.rng().range_usize(0, 32);
            b.unknown(t, n);
        }
        if desc_first {
            b.desc(sr, FMT_IMA4, ch);
            b.pakt(fc);
        } else {
            b.pakt(fc);
            b.desc(sr, FMT_IMA4, ch);
        }
        for _ in 0..rng.below(3) {
            let t = b.rng().unknown_fourcc();
            let n = b.rng().range_usize(0, 32);
            b.unknown(t, n);
        }
        b.data(ds, rng.range_usize(4, 64));
        (b.finish(), rng.range_usize(0, 7))
    });
}

// ---------------------------------------------------------------------------
// Row 22 — `info->blocks` == data-chunk address + 20 (axis P)
// ---------------------------------------------------------------------------
#[test]
fn cfg22_blocks_pointer_identity() {
    let mut rng = Rng::new(0x1111_0016);
    for i in 0..5_000usize {
        let sr = rng.u64();
        let ch = rng.u32();
        let fc = rng.u64();
        let ds = rng.u64() as i64;
        let align_off = rng.range_usize(0, 7);
        let mut b = FileBuilder::valid_header(Rng::new(rng.u64()));
        // A random number of skipped chunks of random size shifts the `data`
        // chunk to a different offset every iteration.
        for _ in 0..rng.range_usize(0, 6) {
            let t = b.rng().unknown_fourcc();
            let n = b.rng().range_usize(0, 64);
            b.unknown(t, n);
        }
        b.desc(sr, FMT_IMA4, ch);
        b.pakt(fc);
        b.data(ds, 34);
        let data_off = b.data_off.unwrap();
        let bytes = b.finish();
        let buf = AlignedBuf::new(&bytes, align_off);
        let o = assert_same(&format!("cfg22 i={i}"), &bytes, buf.ptr());
        assert_eq!(o.c_ret, 0);
        let expect = buf.ptr() as u64 + data_off as u64 + (CHUNK_HDR + CAF_DATA_LEN) as u64;
        assert_eq!(
            o.c_info.blocks(),
            expect,
            "cfg22 i={i}: blocks must be &data_chunk + 16 + 4"
        );
        assert_eq!(o.r_info.blocks(), expect, "cfg22 i={i}");
    }
}

// ---------------------------------------------------------------------------
// Row 23 — whole-buffer fuzz of the composed pipeline
// ---------------------------------------------------------------------------
#[test]
fn cfg23_whole_file_fuzz() {
    row("cfg23", 0x1111_0017, 30_000, None, |rng| {
        // Occasionally break the header so the fuzz also revisits the -1/-2
        // paths from arbitrary starting states.
        let magic = if rng.below(20) == 0 {
            rng.fourcc()
        } else {
            MAGIC_CAFF
        };
        let version = if rng.below(20) == 0 { rng.u16() } else { 1 };
        let fmt = if rng.below(8) == 0 {
            rng.fourcc()
        } else {
            FMT_IMA4
        };
        let sr = rng.u64();
        let ch = rng.u32();
        let fc = rng.u64();
        let ds = rng.u64() as i64;

        let mut b = FileBuilder::new(Rng::new(rng.u64()), magic, version);
        // Random ordering of desc / pakt with unknown chunks sprinkled in.
        let mut items = vec![Item::Desc, Item::Pakt];
        if rng.below(2) == 0 {
            items.swap(0, 1);
        }
        for _ in 0..rng.below(6) {
            let at = rng.range_usize(0, items.len());
            items.insert(at, Item::Unknown);
        }
        for it in items {
            match it {
                Item::Desc => {
                    b.desc(sr, fmt, ch);
                }
                Item::Pakt => {
                    b.pakt(fc);
                }
                Item::Unknown => {
                    let t = b.rng().unknown_fourcc();
                    let n = b.rng().range_usize(0, 80);
                    b.unknown(t, n);
                }
            }
        }
        b.data(ds, rng.range_usize(4, 68));
        (b.finish(), rng.range_usize(0, 7))
    });
}

// ---------------------------------------------------------------------------
// Row 24 — bad format_id AND no pakt chunk: -3 is returned *before* the NULL
// `pakt` dereference (interaction of axes E and J).  Also ERRORS.md row 3.
// ---------------------------------------------------------------------------
#[test]
fn cfg24_bad_format_id_without_pakt() {
    row("cfg24", 0x1111_0019, 3_000, Some(-3), |rng| {
        let fmt = loop {
            let f = rng.fourcc();
            if f != FMT_IMA4 {
                break f;
            }
        };
        let sr = rng.u64();
        let ch = rng.u32();
        let ds = rng.u64() as i64;
        let mut b = FileBuilder::valid_header(Rng::new(rng.u64()));
        b.desc(sr, fmt, ch);
        // deliberately NO pakt chunk
        b.data(ds, 8);
        (b.finish(), rng.range_usize(0, 7))
    });
}

// ---------------------------------------------------------------------------
// Row 25 — OVERLAPPING chunks: the desc payload is also parsed as a chunk
// header, because the desc chunk declares a size (8) smaller than its payload
// (32).  Verifies the exact 16-byte chunk stride (axis I) under aliasing.
//
//     8    desc  hdr(16), declared 8  -> desc = 24, next = 32
//    24    desc payload (32 bytes, ends at 56)
//    32    (overlaps desc payload) type = desc->format_id = "ima4" (unknown),
//          size = BE u64 at payload[16..24] = 8            -> next = 56
//    56    pakt  hdr(16) + payload(24)                     -> next = 96
//    96    data
// ---------------------------------------------------------------------------
#[test]
fn cfg25_overlapping_chunk_headers() {
    row("cfg25", 0x1111_001A, 3_000, Some(0), |rng| {
        let sr = rng.u64();
        let ch = rng.u32();
        let fc = rng.u64();
        let ds = rng.u64() as i64;
        let mut b = FileBuilder::valid_header(Rng::new(rng.u64()));
        assert_eq!(b.offset(), 8);
        b.desc_sized(sr, FMT_IMA4, ch, 8, Some(8));
        assert_eq!(b.offset(), 56);
        b.pakt(fc);
        assert_eq!(b.offset(), 96);
        b.data(ds, 34);
        (b.finish(), rng.range_usize(0, 7))
    });
}

// ---------------------------------------------------------------------------
// Sanity: both libraries were actually loaded from two different files.
// ---------------------------------------------------------------------------
#[test]
fn cfg00_libraries_are_distinct_shared_objects() {
    let l = libs();
    println!("C    .so: {}", l.c_path.display());
    println!("Rust .so: {}", l.rust_path.display());
    assert_ne!(l.c_path, l.rust_path);
    assert_ne!(l.c as usize, l.rust as usize);
    assert!(l.c_path.exists());
    assert!(l.rust_path.exists());
}
