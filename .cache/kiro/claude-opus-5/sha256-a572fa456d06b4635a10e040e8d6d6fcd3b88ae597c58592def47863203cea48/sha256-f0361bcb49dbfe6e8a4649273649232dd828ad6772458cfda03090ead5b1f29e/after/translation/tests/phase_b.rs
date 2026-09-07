//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md` (C1..C23). Every test drives BOTH the C
//! `.so` and the Rust `.so` through their exported `ima_parse` symbol (loaded
//! with `libloading`) and asserts the returned `int` and all 40 bytes of
//! `struct ima_info` are identical.

mod common;

use common::*;

fn pair() -> Pair {
    Pair::load()
}

/// Low-level chunk writer, for the rows that need a layout the builder's
/// linear chunk list cannot express (negative-size backwards walks).
fn write_chunk(buf: &mut AlignedBuf, off: usize, ty: &[u8; 4], size: i64, payload: &[u8]) {
    buf.write(off, ty);
    buf.write(off + 4, &[0xDE, 0xAD, 0xBE, 0xEF]); // ABI padding, never read
    buf.write(off + 8, &size.to_be_bytes());
    buf.write(off + 16, payload);
}

// ---------------------------------------------------------------------------
// C1 — canonical minimal valid file: desc -> pakt -> data
// ---------------------------------------------------------------------------

#[test]
fn cfg_c1_canonical_desc_pakt_data() {
    let p = pair();
    let mut rng = Rng::new(0xC0FFEE_0001);
    for i in 0..ITERS {
        let d = rng.desc();
        let k = rng.pakt();
        let mut b = CafBuilder::new();
        b.flags = rng.u16();
        b.filler = rng.u8();
        b.push(Chunk::desc(&d));
        b.push(Chunk::pakt(&k));
        b.push(Chunk::data(rng.u32(), 34 * rng.range(1, 8)));
        let buf = b.build();
        p.diff(&buf, &format!("C1 iter={i} desc={d:?} pakt={k:?}"));
        // model check: the derived blocks pointer
        let mut info = ImaInfo::prefilled(0);
        assert_eq!(unsafe { (p.c)(&mut info, buf.ptr() as *const _) }, 0);
        assert_eq!(info.blocks, b.expected_blocks(&buf).unwrap(), "C1 blocks addr");
    }
}

// ---------------------------------------------------------------------------
// C2 — order swapped: pakt -> desc -> data
// ---------------------------------------------------------------------------

#[test]
fn cfg_c2_pakt_before_desc() {
    let p = pair();
    let mut rng = Rng::new(0xC0FFEE_0002);
    for i in 0..ITERS {
        let d = rng.desc();
        let k = rng.pakt();
        let mut b = CafBuilder::new();
        b.filler = rng.u8();
        b.push(Chunk::pakt(&k));
        b.push(Chunk::desc(&d));
        b.push(Chunk::data(rng.u32(), 34));
        p.diff(&b.build(), &format!("C2 iter={i}"));
    }
}

// ---------------------------------------------------------------------------
// C3 — unknown chunk before desc, declared size == real payload size
// ---------------------------------------------------------------------------

#[test]
fn cfg_c3_unknown_chunk_exact_size() {
    let p = pair();
    let mut rng = Rng::new(0xC0FFEE_0003);
    for i in 0..ITERS {
        let d = rng.desc();
        let k = rng.pakt();
        let n = rng.below(257);
        let ty = rng.unknown_fourcc();
        let mut b = CafBuilder::new();
        b.filler = rng.u8();
        b.push(Chunk::new(&ty, rng.bytes(n)));
        b.push(Chunk::desc(&d));
        let ty2 = rng.unknown_fourcc();
        let n2 = rng.below(64);
        b.push(Chunk::new(&ty2, rng.bytes(n2)));
        b.push(Chunk::pakt(&k));
        b.push(Chunk::data(rng.u32(), 34 * 2));
        p.diff(&b.build(), &format!("C3 iter={i} unknown={ty:?} n={n}"));
    }
}

// ---------------------------------------------------------------------------
// C4 — unknown chunk with size == 0
// ---------------------------------------------------------------------------

#[test]
fn cfg_c4_zero_size_unknown_chunk() {
    let p = pair();
    let mut rng = Rng::new(0xC0FFEE_0004);
    for i in 0..ITERS {
        let d = rng.desc();
        let k = rng.pakt();
        let mut b = CafBuilder::new();
        b.filler = rng.u8();
        for _ in 0..rng.range(1, 4) {
            b.push(Chunk::new(&rng.unknown_fourcc(), Vec::new()).with_declared_size(0));
        }
        b.push(Chunk::desc(&d));
        b.push(Chunk::pakt(&k));
        b.push(Chunk::data(rng.u32(), 34));
        p.diff(&b.build(), &format!("C4 iter={i}"));
    }
}

// ---------------------------------------------------------------------------
// C5 — unknown chunk whose declared size skips over trailing padding
// ---------------------------------------------------------------------------

#[test]
fn cfg_c5_unknown_chunk_oversized_declared_size() {
    let p = pair();
    let mut rng = Rng::new(0xC0FFEE_0005);
    for i in 0..ITERS {
        let d = rng.desc();
        let k = rng.pakt();
        let real = rng.below(33);
        let pad = rng.range(1, 200);
        let mut b = CafBuilder::new();
        b.filler = rng.u8();
        b.push(
            Chunk::new(&rng.unknown_fourcc(), rng.bytes(real))
                .with_declared_size((real + pad) as i64)
                .with_pad_after(pad),
        );
        b.push(Chunk::desc(&d));
        b.push(Chunk::pakt(&k));
        b.push(Chunk::data(rng.u32(), 34));
        p.diff(&b.build(), &format!("C5 iter={i} real={real} pad={pad}"));
    }
}

// ---------------------------------------------------------------------------
// C6 — desc/pakt chunks whose declared size exceeds the struct size
// ---------------------------------------------------------------------------

#[test]
fn cfg_c6_desc_pakt_padded_declared_size() {
    let p = pair();
    let mut rng = Rng::new(0xC0FFEE_0006);
    for i in 0..ITERS {
        let d = rng.desc();
        let k = rng.pakt();
        let dpad = rng.range(1, 128);
        let kpad = rng.range(1, 128);
        let mut b = CafBuilder::new();
        b.filler = rng.u8();
        b.push(
            Chunk::desc(&d)
                .with_declared_size((DESC_LEN + dpad) as i64)
                .with_pad_after(dpad),
        );
        b.push(
            Chunk::pakt(&k)
                .with_declared_size((PAKT_LEN + kpad) as i64)
                .with_pad_after(kpad),
        );
        b.push(Chunk::data(rng.u32(), 34));
        p.diff(&b.build(), &format!("C6 iter={i} dpad={dpad} kpad={kpad}"));
    }
}

// ---------------------------------------------------------------------------
// C7 / C23 — negative chunk size: the walk moves BACKWARDS
// ---------------------------------------------------------------------------

/// Layout (all offsets from the buffer start):
/// ```text
///   0   header
///   8   U1  (unknown, declared_size = +N)      -> next = 24 + N = X
///   Y   desc / pakt / data   (Y < X, inside the region U1 jumped over)
///   X   U2  (unknown, declared_size = -(X+16-Y)) -> next = Y  (backwards!)
/// ```
/// So the walk is: `U1` -> forward to `U2` -> **backward** to `desc` -> `pakt`
/// -> `data`.
#[test]
fn cfg_c7_negative_size_walks_backwards() {
    let p = pair();
    let mut rng = Rng::new(0xC0FFEE_0007);
    for i in 0..ITERS {
        let d = rng.desc();
        let k = rng.pakt();
        let y = 24 + 16 * rng.range(1, 6); // desc starts here, after U1's header
        let desc_len = CHUNK_HDR + DESC_LEN;
        let pakt_len = CHUNK_HDR + PAKT_LEN;
        let data_len = CHUNK_HDR + CAF_DATA_LEN + 34;
        let x = y + desc_len + pakt_len + data_len + 16 * rng.below(4);
        let n = (x - 24) as i64; // U1 jumps from 24 to X
        let back = -((x + 16 - y) as i64); // U2 jumps from X+16 back to Y
        let len = x + 16 + 64;

        let mut buf = AlignedBuf::new(len, 0);
        let filler = vec![rng.u8(); len];
        buf.write(0, &filler);
        buf.write(0, b"caff");
        buf.write(4, &1u16.to_be_bytes());
        buf.write(6, &rng.u16().to_be_bytes());
        write_chunk(&mut buf, 8, &rng.unknown_fourcc(), n, &[]);
        write_chunk(&mut buf, y, b"desc", DESC_LEN as i64, &d.encode());
        write_chunk(&mut buf, y + desc_len, b"pakt", PAKT_LEN as i64, &k.encode());
        let data_off = y + desc_len + pakt_len;
        let mut dp = rng.u32().to_be_bytes().to_vec();
        dp.extend(rng.bytes(34));
        write_chunk(&mut buf, data_off, b"data", rng.i64(), &dp);
        write_chunk(&mut buf, x, &rng.unknown_fourcc(), back, &[]);

        p.diff(&buf, &format!("C7 iter={i} y={y} x={x} n={n} back={back}"));

        let mut info = ImaInfo::prefilled(0);
        assert_eq!(unsafe { (p.c)(&mut info, buf.ptr() as *const _) }, 0);
        assert_eq!(
            info.blocks,
            unsafe { buf.ptr().add(data_off + CHUNK_HDR + CAF_DATA_LEN) },
            "C7 blocks addr"
        );
    }
}

// ---------------------------------------------------------------------------
// C8 — duplicate desc chunks: last wins
// ---------------------------------------------------------------------------

#[test]
fn cfg_c8_duplicate_desc_last_wins() {
    let p = pair();
    let mut rng = Rng::new(0xC0FFEE_0008);
    for i in 0..ITERS {
        let k = rng.pakt();
        let count = rng.range(2, 4);
        let mut b = CafBuilder::new();
        b.filler = rng.u8();
        let mut last = DescFields::valid();
        for j in 0..count {
            let mut d = rng.desc();
            // earlier copies get a deliberately wrong format_id: only the last
            // one is read, so the parse must still succeed.
            if j + 1 < count {
                d.format_id = rng.u32().to_be_bytes();
            }
            last = d;
            b.push(Chunk::desc(&d));
        }
        b.push(Chunk::pakt(&k));
        b.push(Chunk::data(rng.u32(), 34));
        p.diff(&b.build(), &format!("C8 iter={i} count={count} last={last:?}"));
    }
}

// ---------------------------------------------------------------------------
// C9 — duplicate pakt chunks: last wins
// ---------------------------------------------------------------------------

#[test]
fn cfg_c9_duplicate_pakt_last_wins() {
    let p = pair();
    let mut rng = Rng::new(0xC0FFEE_0009);
    for i in 0..ITERS {
        let d = rng.desc();
        let count = rng.range(2, 4);
        let mut b = CafBuilder::new();
        b.filler = rng.u8();
        b.push(Chunk::desc(&d));
        for _ in 0..count {
            let k = rng.pakt();
            b.push(Chunk::pakt(&k));
        }
        b.push(Chunk::data(rng.u32(), 34));
        p.diff(&b.build(), &format!("C9 iter={i} count={count}"));
    }
}

// ---------------------------------------------------------------------------
// C10 — deep loop: many interleaved unknown chunks
// ---------------------------------------------------------------------------

#[test]
fn cfg_c10_many_chunks_deep_loop() {
    let p = pair();
    let mut rng = Rng::new(0xC0FFEE_0010);
    for i in 0..(ITERS / 4) {
        let d = rng.desc();
        let k = rng.pakt();
        let n = rng.range(8, 64);
        let desc_at = rng.below(n);
        let pakt_at = rng.below(n);
        let mut b = CafBuilder::new();
        b.filler = rng.u8();
        for j in 0..n {
            if j == desc_at {
                b.push(Chunk::desc(&d));
            }
            if j == pakt_at {
                b.push(Chunk::pakt(&k));
            }
            let sz = rng.below(48);
            b.push(Chunk::new(&rng.unknown_fourcc(), rng.bytes(sz)));
        }
        if desc_at >= n {
            b.push(Chunk::desc(&d));
        }
        if pakt_at >= n {
            b.push(Chunk::pakt(&k));
        }
        b.push(Chunk::data(rng.u32(), 34));
        p.diff(&b.build(), &format!("C10 iter={i} n={n}"));
    }
}

// ---------------------------------------------------------------------------
// C11 — data-chunk `size` shape -> info->size
// ---------------------------------------------------------------------------

#[test]
fn cfg_c11_data_chunk_size_shapes() {
    let p = pair();
    let mut rng = Rng::new(0xC0FFEE_0011);
    let mut sizes: Vec<i64> = vec![
        0,
        1,
        34,
        -1,
        i64::MIN,
        i64::MAX,
        i64::MIN + 1,
        i64::MAX - 1,
        1 << 32,
        -(1i64 << 32),
        0x0000_0000_FFFF_FFFF,
        0x0100_0000_0000_0000,
    ];
    for _ in 0..ITERS {
        sizes.push(rng.i64());
    }
    for (i, sz) in sizes.iter().enumerate() {
        let d = rng.desc();
        let k = rng.pakt();
        let mut b = CafBuilder::new();
        b.filler = rng.u8();
        b.push(Chunk::desc(&d));
        b.push(Chunk::pakt(&k));
        b.push(Chunk::data(rng.u32(), 34).with_declared_size(*sz));
        p.diff(&b.build(), &format!("C11 iter={i} data_size={sz:#x}"));
    }
}

// ---------------------------------------------------------------------------
// C12 — sample_rate as a random raw u64 bit pattern
// ---------------------------------------------------------------------------

#[test]
fn cfg_c12_sample_rate_random_bit_patterns() {
    let p = pair();
    let mut rng = Rng::new(0xC0FFEE_0012);
    for i in 0..(ITERS * 6) {
        let mut d = rng.desc();
        d.sample_rate_raw = rng.u64();
        let k = rng.pakt();
        let mut b = CafBuilder::new();
        b.push(Chunk::desc(&d));
        b.push(Chunk::pakt(&k));
        b.push(Chunk::data(0, 34));
        p.diff_fill(
            &b.build(),
            0xAA,
            &format!("C12 iter={i} raw={:#018x} as_f64={}", d.sample_rate_raw, f64::from_bits(d.sample_rate_raw)),
        );
    }
}

// ---------------------------------------------------------------------------
// C13/C14/C15/C16 — the full `double`->`u64` conversion domain
// ---------------------------------------------------------------------------

fn sample_rate_row(seed: u64, label: &str, values: &[u64]) {
    let p = pair();
    let mut rng = Rng::new(seed);
    for (i, raw) in values.iter().enumerate() {
        let mut d = rng.desc();
        d.sample_rate_raw = *raw;
        let k = rng.pakt();
        let mut b = CafBuilder::new();
        b.push(Chunk::desc(&d));
        b.push(Chunk::pakt(&k));
        b.push(Chunk::data(0, 34));
        p.diff(
            &b.build(),
            &format!("{label} i={i} raw={raw:#018x} f64={}", f64::from_bits(*raw)),
        );
    }
}

#[test]
fn cfg_c13_sample_rate_in_range_positive() {
    let mut rng = Rng::new(0xC0FFEE_0013);
    let mut vals: Vec<u64> = [8000.0f64, 22050.0, 44100.0, 48000.0, 96000.0, 192000.0, 1.0, 0.0, 0.5]
        .iter()
        .map(|x| x.to_bits())
        .collect();
    for _ in 0..ITERS {
        // uniform magnitudes inside [0, 2^63)
        let e = rng.below(63) as i32;
        let m = (rng.u64() >> 11) as f64 / (1u64 << 53) as f64;
        vals.push(((1.0 + m) * 2f64.powi(e)).to_bits());
    }
    sample_rate_row(0xB13, "C13", &vals);
}

#[test]
fn cfg_c14_sample_rate_in_range_negative() {
    let mut rng = Rng::new(0xC0FFEE_0014);
    let mut vals: Vec<u64> = [-1.0f64, -0.5, -0.0, -44100.0, -1e17]
        .iter()
        .map(|x| x.to_bits())
        .collect();
    for _ in 0..ITERS {
        let e = rng.below(63) as i32;
        let m = (rng.u64() >> 11) as f64 / (1u64 << 53) as f64;
        vals.push((-(1.0 + m) * 2f64.powi(e)).to_bits());
    }
    sample_rate_row(0xB14, "C14", &vals);
}

#[test]
fn cfg_c15_sample_rate_at_or_above_two_pow_63() {
    let mut rng = Rng::new(0xC0FFEE_0015);
    let mut vals: Vec<u64> = [
        9223372036854775808.0f64,  // 2^63
        9223372036854777856.0,     // 2^63 + 2048
        18446744073709549568.0,    // largest f64 < 2^64
        18446744073709551616.0,    // 2^64
        1e300,
        f64::MAX,
        f64::INFINITY,
    ]
    .iter()
    .map(|x| x.to_bits())
    .collect();
    for _ in 0..ITERS {
        let e = 63 + rng.below(200) as i32;
        let m = (rng.u64() >> 11) as f64 / (1u64 << 53) as f64;
        vals.push(((1.0 + m) * 2f64.powi(e)).to_bits());
    }
    sample_rate_row(0xB15, "C15", &vals);
}

#[test]
fn cfg_c16_sample_rate_edge_domain() {
    let mut rng = Rng::new(0xC0FFEE_0016);
    let mut vals = interesting_f64_bits();
    for _ in 0..ITERS {
        // below -2^63
        let e = 63 + rng.below(200) as i32;
        let m = (rng.u64() >> 11) as f64 / (1u64 << 53) as f64;
        vals.push((-(1.0 + m) * 2f64.powi(e)).to_bits());
        // subnormals
        vals.push(rng.u64() & 0x000F_FFFF_FFFF_FFFF);
        // NaNs with random payloads
        vals.push(0x7FF0_0000_0000_0000 | (rng.u64() & 0x000F_FFFF_FFFF_FFFF).max(1));
        vals.push(0xFFF0_0000_0000_0000 | (rng.u64() & 0x000F_FFFF_FFFF_FFFF).max(1));
    }
    sample_rate_row(0xB16, "C16", &vals);
}

// ---------------------------------------------------------------------------
// C17 — channel_count axis
// ---------------------------------------------------------------------------

#[test]
fn cfg_c17_channel_count_axis() {
    let p = pair();
    let mut rng = Rng::new(0xC0FFEE_0017);
    let mut vals: Vec<u32> = vec![0, 1, 2, 3, 8, 0x7F, 0x80, 0xFF, 0x100, 0xFFFF, 0x7FFF_FFFF, 0x8000_0000, 0xFFFF_FFFF];
    for _ in 0..ITERS {
        vals.push(rng.u32());
    }
    for (i, ch) in vals.iter().enumerate() {
        let mut d = rng.desc();
        d.channels_per_frame = *ch;
        let k = rng.pakt();
        let mut b = CafBuilder::new();
        b.push(Chunk::desc(&d));
        b.push(Chunk::pakt(&k));
        b.push(Chunk::data(0, 34));
        p.diff(&b.build(), &format!("C17 i={i} channels={ch:#x}"));
    }
}

// ---------------------------------------------------------------------------
// C18 — frame_count axis
// ---------------------------------------------------------------------------

#[test]
fn cfg_c18_frame_count_axis() {
    let p = pair();
    let mut rng = Rng::new(0xC0FFEE_0018);
    let mut vals: Vec<i64> = vec![0, 1, -1, 2, i64::MIN, i64::MAX, i64::MIN + 1, i64::MAX - 1, 1 << 32, -(1i64 << 32), 0xFF];
    for _ in 0..ITERS {
        vals.push(rng.i64());
    }
    for (i, fc) in vals.iter().enumerate() {
        let d = rng.desc();
        let mut k = rng.pakt();
        k.frame_count = *fc;
        let mut b = CafBuilder::new();
        b.push(Chunk::desc(&d));
        b.push(Chunk::pakt(&k));
        b.push(Chunk::data(0, 34));
        p.diff(&b.build(), &format!("C18 i={i} frame_count={fc:#x}"));
    }
}

// ---------------------------------------------------------------------------
// C19 — unaligned `data` base pointer
// ---------------------------------------------------------------------------

#[test]
fn cfg_c19_unaligned_data_pointer() {
    let p = pair();
    let mut rng = Rng::new(0xC0FFEE_0019);
    for skew in 0..16usize {
        for i in 0..(ITERS / 8) {
            let d = rng.desc();
            let k = rng.pakt();
            let mut b = CafBuilder::new();
            b.skew = skew;
            b.filler = rng.u8();
            b.push(Chunk::desc(&d));
            let ty2 = rng.unknown_fourcc();
            let n2 = rng.below(17);
            b.push(Chunk::new(&ty2, rng.bytes(n2)));
            b.push(Chunk::pakt(&k));
            b.push(Chunk::data(rng.u32(), 34));
            p.diff(&b.build(), &format!("C19 skew={skew} iter={i}"));
        }
    }
}

// ---------------------------------------------------------------------------
// C20 — `info->blocks` output pointer parity at arbitrary data-chunk offsets
// ---------------------------------------------------------------------------

#[test]
fn cfg_c20_blocks_pointer_parity() {
    let p = pair();
    let mut rng = Rng::new(0xC0FFEE_0020);
    for i in 0..ITERS {
        let d = rng.desc();
        let k = rng.pakt();
        let mut b = CafBuilder::new();
        b.skew = rng.below(8);
        b.filler = rng.u8();
        b.push(Chunk::desc(&d));
        b.push(Chunk::pakt(&k));
        // shove the data chunk to a random offset with a filler chunk
        let ty2 = rng.unknown_fourcc();
        let n2 = rng.below(97);
        b.push(Chunk::new(&ty2, rng.bytes(n2)));
        b.push(Chunk::data(rng.u32(), 34));
        let buf = b.build();
        p.diff(&buf, &format!("C20 iter={i}"));

        let want = b.expected_blocks(&buf).unwrap();
        let mut ci = ImaInfo::prefilled(0x5A);
        let mut ri = ImaInfo::prefilled(0x5A);
        assert_eq!(unsafe { (p.c)(&mut ci, buf.ptr() as *const _) }, 0);
        assert_eq!(unsafe { (p.r)(&mut ri, buf.ptr() as *const _) }, 0);
        assert_eq!(ci.blocks, want, "C20 C blocks addr");
        assert_eq!(ri.blocks, want, "C20 Rust blocks addr");
    }
}

// ---------------------------------------------------------------------------
// C21 — never-read fields filled with noise must not affect the output
// ---------------------------------------------------------------------------

#[test]
fn cfg_c21_unread_fields_are_ignored() {
    let p = pair();
    let mut rng = Rng::new(0xC0FFEE_0021);
    for i in 0..ITERS {
        // fixed read fields...
        let base_desc = DescFields {
            sample_rate_raw: 0x1122_3344_5566_7788,
            format_id: *b"ima4",
            channels_per_frame: 0xDEAD_BEEF,
            format_flags: 0,
            bytes_per_packet: 0,
            frames_per_packet: 0,
            bits_per_channel: 0,
        };
        let base_pakt = PaktFields {
            frame_count: 0x0123_4567_89AB_CDEF,
            packet_count: 0,
            priming_frames: 0,
            remainder_frames: 0,
        };

        let quiet = {
            let mut b = CafBuilder::new();
            b.push(Chunk::desc(&base_desc));
            b.push(Chunk::pakt(&base_pakt));
            b.push(Chunk::data(0, 34));
            b
        };

        // ...and randomized never-read fields
        let mut noisy_desc = base_desc;
        noisy_desc.format_flags = rng.u32();
        noisy_desc.bytes_per_packet = rng.u32();
        noisy_desc.frames_per_packet = rng.u32();
        noisy_desc.bits_per_channel = rng.u32();
        let mut noisy_pakt = base_pakt;
        noisy_pakt.packet_count = rng.i64();
        noisy_pakt.priming_frames = rng.i32();
        noisy_pakt.remainder_frames = rng.i32();

        let mut noisy = CafBuilder::new();
        noisy.flags = rng.u16();
        noisy.push(Chunk::desc(&noisy_desc).with_hdr_pad(rng.u32().to_be_bytes()));
        noisy.push(Chunk::pakt(&noisy_pakt).with_hdr_pad(rng.u32().to_be_bytes()));
        noisy.push(Chunk::data(rng.u32(), 34).with_hdr_pad(rng.u32().to_be_bytes()));

        let qbuf = quiet.build();
        let nbuf = noisy.build();
        p.diff(&qbuf, &format!("C21 quiet iter={i}"));
        p.diff(&nbuf, &format!("C21 noisy iter={i}"));

        // the noise must not change anything except the derived blocks pointer
        let mut qi = ImaInfo::prefilled(0);
        let mut ni = ImaInfo::prefilled(0);
        assert_eq!(unsafe { (p.r)(&mut qi, qbuf.ptr() as *const _) }, 0);
        assert_eq!(unsafe { (p.r)(&mut ni, nbuf.ptr() as *const _) }, 0);
        assert_eq!(qi.size, ni.size, "C21 size changed by noise");
        assert_eq!(qi.frame_count, ni.frame_count, "C21 frame_count changed by noise");
        assert_eq!(qi.channel_count, ni.channel_count, "C21 channel_count changed by noise");
        assert_eq!(
            qi.sample_rate.to_bits(),
            ni.sample_rate.to_bits(),
            "C21 sample_rate changed by noise"
        );
    }
}

// ---------------------------------------------------------------------------
// C22 — full structural fuzz
// ---------------------------------------------------------------------------

#[test]
fn cfg_c22_structural_fuzz() {
    let p = pair();
    let mut rng = Rng::new(0xF0F0_1234_5678_9ABC);
    let mut cases = 0usize;
    for i in 0..10_000usize {
        let mut b = CafBuilder::new();
        b.skew = rng.below(8);
        b.filler = rng.u8();
        b.flags = rng.u16();

        let nlead = rng.below(6);
        let d = rng.desc();
        let k = rng.pakt();
        let mut have_desc = false;
        let mut have_pakt = false;

        for _ in 0..nlead {
            match rng.below(4) {
                0 => {
                    let mut dd = rng.desc();
                    if rng.below(4) == 0 {
                        dd.format_id = rng.u32().to_be_bytes();
                    }
                    b.push(Chunk::desc(&dd));
                    have_desc = true;
                }
                1 => {
                    let kk = rng.pakt();
                    b.push(Chunk::pakt(&kk));
                    have_pakt = true;
                }
                _ => {
                    let n = rng.below(65);
                    let pad = rng.below(33);
                    b.push(
                        Chunk::new(&rng.unknown_fourcc(), rng.bytes(n))
                            .with_declared_size((n + pad) as i64)
                            .with_pad_after(pad)
                            .with_hdr_pad(rng.u32().to_be_bytes()),
                    );
                }
            }
        }
        // guarantee a desc and a pakt precede the data chunk (otherwise the C
        // dereferences NULL -- that case lives in the crash-parity harness)
        if !have_desc || rng.below(2) == 0 {
            b.push(Chunk::desc(&d));
        }
        if !have_pakt || rng.below(2) == 0 {
            b.push(Chunk::pakt(&k));
        }
        b.push(
            Chunk::data(rng.u32(), 34 * rng.range(1, 4))
                .with_declared_size(rng.i64())
                .with_hdr_pad(rng.u32().to_be_bytes()),
        );
        // trailing garbage that must never be reached
        b.tail = rng.below(64);

        p.diff_fill(&b.build(), 0xAA, &format!("C22 iter={i}"));
        cases += 1;
    }
    assert_eq!(cases, 10_000);
}

// ---------------------------------------------------------------------------
// C23 — data chunk reached only after a backwards jump (C7 + C1 combination)
// ---------------------------------------------------------------------------

#[test]
fn cfg_c23_data_after_backwards_jump() {
    let p = pair();
    let mut rng = Rng::new(0xC0FFEE_0023);
    let desc_len = CHUNK_HDR + DESC_LEN;
    let pakt_len = CHUNK_HDR + PAKT_LEN;
    let data_len = CHUNK_HDR + CAF_DATA_LEN + 34;
    for i in 0..ITERS {
        // 0:   header
        // 8:   U1  (unknown, forward jump to X)
        // Y:   desc -> pakt -> data      (Y < X, skipped over by U1)
        // X:   U2  (unknown, backwards jump to Y)
        //
        // Combines C7's backwards arithmetic with C1's canonical chunk trio and
        // a randomly skewed base pointer, so the `data` chunk (and hence the
        // derived `blocks` pointer) is only reached after a negative walk.
        let gap = 16 * rng.below(4);
        let y = 24 + 16 * rng.range(1, 4);
        let x = y + desc_len + pakt_len + data_len + gap;
        let len = x + 16 + 64;

        let mut d = rng.desc();
        d.format_id = *b"ima4";
        let k = rng.pakt();
        let data_size = rng.i64();
        let mut dp = rng.u32().to_be_bytes().to_vec();
        dp.extend(rng.bytes(34));

        let mut buf = AlignedBuf::new(len, rng.below(8));
        let filler = vec![rng.u8(); len];
        buf.write(0, &filler);
        buf.write(0, b"caff");
        buf.write(4, &1u16.to_be_bytes());
        buf.write(6, &rng.u16().to_be_bytes());
        write_chunk(&mut buf, 8, &rng.unknown_fourcc(), (x - 24) as i64, &[]);
        write_chunk(&mut buf, y, b"desc", DESC_LEN as i64, &d.encode());
        write_chunk(&mut buf, y + desc_len, b"pakt", PAKT_LEN as i64, &k.encode());
        let doff = y + desc_len + pakt_len;
        write_chunk(&mut buf, doff, b"data", data_size, &dp);
        write_chunk(&mut buf, x, &rng.unknown_fourcc(), -((x + 16 - y) as i64), &[]);

        p.diff(&buf, &format!("C23 iter={i} y={y} x={x} gap={gap}"));

        let mut ci = ImaInfo::prefilled(0x11);
        let mut ri = ImaInfo::prefilled(0x11);
        assert_eq!(unsafe { (p.c)(&mut ci, buf.ptr() as *const _) }, 0);
        assert_eq!(unsafe { (p.r)(&mut ri, buf.ptr() as *const _) }, 0);
        let want = unsafe { buf.ptr().add(doff + CHUNK_HDR + CAF_DATA_LEN) };
        assert_eq!(ci.blocks, want, "C23 C blocks addr");
        assert_eq!(ri.blocks, want, "C23 Rust blocks addr");
    }
}
