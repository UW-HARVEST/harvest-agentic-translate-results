//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every test drives BOTH the C `.so` and the
//! Rust `.so` through `ima_parse` on the *same* buffer and compares the return
//! value plus all 40 bytes of `struct ima_info`. Inputs are randomized from a
//! fixed-seed SplitMix64 so failures reproduce exactly.

mod common;

use common::*;

/// Number of random inputs per property-style row.
const ITERS: usize = 20_000;

/// Random filler chunk type that is guaranteed not to be `desc`/`pakt`/`data`.
fn unknown_type(rng: &Rng) -> [u8; 4] {
    loop {
        let t = rng.arr4();
        if !is_known_type(t) {
            return t;
        }
    }
}

/// Appends `n` filler chunks whose `size` fields match their payloads, so the
/// walk lands exactly on whatever comes next.
fn push_fillers(caf: &mut Caf, rng: &Rng, n: usize) {
    for _ in 0..n {
        let len = rng.below(40) as usize;
        let payload = rng.bytes(len);
        let t = if rng.below(4) == 0 {
            rng.pick(UNKNOWN_TYPES)
        } else {
            unknown_type(rng)
        };
        caf.chunk(t, rng.arr4(), &payload);
    }
}

/// A complete, well-formed stream: header, `desc`, `pakt`, `data`, with random
/// filler chunks interleaved at every position.
fn random_valid(rng: &Rng, desc: &Desc, pakt: &Pakt, data_size: i64) -> Caf {
    let mut caf = Caf::valid_header(rng);
    push_fillers(&mut caf, rng, rng.below(3) as usize);
    caf.desc(rng.arr4(), desc);
    push_fillers(&mut caf, rng, rng.below(3) as usize);
    caf.pakt(rng.arr4(), pakt);
    push_fillers(&mut caf, rng, rng.below(3) as usize);
    let trailing = rng.bytes(rng.below(80) as usize);
    caf.data(rng.arr4(), data_size, rng.next_u32(), &trailing);
    caf
}

// ---------------------------------------------------------------------------
// Row 1 — canonical order, everything random
// ---------------------------------------------------------------------------

#[test]
fn row01_desc_pakt_data_random_fields() {
    let rng = Rng::new(0x0101);
    for i in 0..ITERS {
        let desc = Desc::random_ima4(&rng);
        let pakt = Pakt::random(&rng);
        let size = rng.below(1 << 20) as i64;
        let caf = random_valid(&rng, &desc, &pakt, size);
        let out = assert_same(&format!("row01/{i}"), &caf.buf);
        assert_eq!(out.ret, 0);
        assert_eq!(out.info.size(), size as u64);
        assert_eq!(out.info.frame_count(), pakt.frame_count as u64);
        assert_eq!(out.info.channel_count(), desc.channels_per_frame);
    }
}

// ---------------------------------------------------------------------------
// Row 2 — pakt before desc
// ---------------------------------------------------------------------------

#[test]
fn row02_pakt_before_desc() {
    let rng = Rng::new(0x0202);
    for i in 0..ITERS {
        let desc = Desc::random_ima4(&rng);
        let pakt = Pakt::random(&rng);
        let mut caf = Caf::valid_header(&rng);
        caf.pakt(rng.arr4(), &pakt);
        push_fillers(&mut caf, &rng, rng.below(3) as usize);
        caf.desc(rng.arr4(), &desc);
        caf.data(rng.arr4(), rng.next_u64() as i64, rng.next_u32(), &[1, 2, 3, 4]);
        let out = assert_same(&format!("row02/{i}"), &caf.buf);
        assert_eq!(out.ret, 0);
    }
}

// ---------------------------------------------------------------------------
// Row 3 — no filler chunks at all
// ---------------------------------------------------------------------------

#[test]
fn row03_no_filler_chunks() {
    let rng = Rng::new(0x0303);
    for i in 0..ITERS {
        let desc = Desc::random_ima4(&rng);
        let pakt = Pakt::random(&rng);
        let mut caf = Caf::valid_header(&rng);
        caf.desc(rng.arr4(), &desc);
        caf.pakt(rng.arr4(), &pakt);
        caf.data(rng.arr4(), rng.below(1 << 30) as i64, rng.next_u32(), &[]);
        assert_eq!(assert_same(&format!("row03/{i}"), &caf.buf).ret, 0);
    }
}

// ---------------------------------------------------------------------------
// Row 4 — exactly one filler chunk between desc and pakt
// ---------------------------------------------------------------------------

#[test]
fn row04_one_filler_between_desc_and_pakt() {
    let rng = Rng::new(0x0404);
    for i in 0..ITERS {
        let desc = Desc::random_ima4(&rng);
        let pakt = Pakt::random(&rng);
        let mut caf = Caf::valid_header(&rng);
        caf.desc(rng.arr4(), &desc);
        push_fillers(&mut caf, &rng, 1);
        caf.pakt(rng.arr4(), &pakt);
        caf.data(rng.arr4(), rng.next_u64() as i64, rng.next_u32(), &[0xAA; 16]);
        assert_eq!(assert_same(&format!("row04/{i}"), &caf.buf).ret, 0);
    }
}

// ---------------------------------------------------------------------------
// Row 5 — many filler chunks
// ---------------------------------------------------------------------------

#[test]
fn row05_many_filler_chunks() {
    let rng = Rng::new(0x0505);
    for i in 0..500 {
        let desc = Desc::random_ima4(&rng);
        let pakt = Pakt::random(&rng);
        let mut caf = Caf::valid_header(&rng);
        push_fillers(&mut caf, &rng, 8 + rng.below(25) as usize);
        caf.desc(rng.arr4(), &desc);
        push_fillers(&mut caf, &rng, 8 + rng.below(25) as usize);
        caf.pakt(rng.arr4(), &pakt);
        push_fillers(&mut caf, &rng, 8 + rng.below(25) as usize);
        caf.data(rng.arr4(), rng.next_u64() as i64, rng.next_u32(), &[]);
        let out = assert_same(&format!("row05/{i}"), &caf.buf);
        assert_eq!(out.ret, 0);
        assert_eq!(out.info.blocks(), caf.expected_blocks(caf.buf.as_ptr()));
    }
}

// ---------------------------------------------------------------------------
// Row 6 — duplicate desc chunks: the last one before `data` wins
// ---------------------------------------------------------------------------

#[test]
fn row06_duplicate_desc_last_wins() {
    let rng = Rng::new(0x0606);
    for i in 0..ITERS {
        let first = Desc::random_ima4(&rng);
        let mut second = Desc::random_ima4(&rng);
        // Make the two distinguishable in the output.
        second.channels_per_frame = first.channels_per_frame ^ 0x5555_5555;
        let pakt = Pakt::random(&rng);

        let mut caf = Caf::valid_header(&rng);
        caf.desc(rng.arr4(), &first);
        push_fillers(&mut caf, &rng, rng.below(3) as usize);
        caf.desc(rng.arr4(), &second);
        caf.pakt(rng.arr4(), &pakt);
        caf.data(rng.arr4(), 7, rng.next_u32(), &[]);

        let out = assert_same(&format!("row06/{i}"), &caf.buf);
        assert_eq!(out.ret, 0);
        // Latching semantics: the *second* desc is the one reported.
        assert_eq!(
            out.info.channel_count(),
            second.channels_per_frame,
            "later desc chunk must win"
        );
    }
}

// ---------------------------------------------------------------------------
// Row 7 — duplicate pakt chunks: the last one before `data` wins
// ---------------------------------------------------------------------------

#[test]
fn row07_duplicate_pakt_last_wins() {
    let rng = Rng::new(0x0707);
    for i in 0..ITERS {
        let desc = Desc::random_ima4(&rng);
        let first = Pakt::random(&rng);
        let mut second = Pakt::random(&rng);
        second.frame_count = first.frame_count ^ 0x1234_5678_9abc_def0u64 as i64;

        let mut caf = Caf::valid_header(&rng);
        caf.desc(rng.arr4(), &desc);
        caf.pakt(rng.arr4(), &first);
        push_fillers(&mut caf, &rng, rng.below(3) as usize);
        caf.pakt(rng.arr4(), &second);
        caf.data(rng.arr4(), 9, rng.next_u32(), &[]);

        let out = assert_same(&format!("row07/{i}"), &caf.buf);
        assert_eq!(out.ret, 0);
        assert_eq!(
            out.info.frame_count(),
            second.frame_count as u64,
            "later pakt chunk must win"
        );
    }
}

// ---------------------------------------------------------------------------
// Row 8 — desc/pakt after the data chunk are never seen
// ---------------------------------------------------------------------------

#[test]
fn row08_chunks_after_data_ignored() {
    let rng = Rng::new(0x0808);
    for i in 0..ITERS {
        let desc = Desc::random_ima4(&rng);
        let pakt = Pakt::random(&rng);
        let mut caf = Caf::valid_header(&rng);
        caf.desc(rng.arr4(), &desc);
        caf.pakt(rng.arr4(), &pakt);
        caf.data(rng.arr4(), 123, rng.next_u32(), &[]);
        // Everything below is behind the `break` and must be invisible.
        let mut decoy_desc = Desc::random_ima4(&rng);
        decoy_desc.channels_per_frame = !desc.channels_per_frame;
        caf.desc(rng.arr4(), &decoy_desc);
        let mut decoy_pakt = Pakt::random(&rng);
        decoy_pakt.frame_count = !pakt.frame_count;
        caf.pakt(rng.arr4(), &decoy_pakt);

        let out = assert_same(&format!("row08/{i}"), &caf.buf);
        assert_eq!(out.ret, 0);
        assert_eq!(out.info.channel_count(), desc.channels_per_frame);
        assert_eq!(out.info.frame_count(), pakt.frame_count as u64);
        assert_eq!(out.info.size(), 123);
    }
}

// ---------------------------------------------------------------------------
// Rows 9-11 — the data chunk's `size` field becomes `info->size` unvalidated
// ---------------------------------------------------------------------------

#[test]
fn row09_data_size_zero() {
    let rng = Rng::new(0x0909);
    for i in 0..ITERS {
        let caf = random_valid(
            &rng,
            &Desc::random_ima4(&rng),
            &Pakt::random(&rng),
            0,
        );
        let out = assert_same(&format!("row09/{i}"), &caf.buf);
        assert_eq!(out.ret, 0);
        assert_eq!(out.info.size(), 0);
    }
}

#[test]
fn row10_data_size_random_full_range() {
    let rng = Rng::new(0x0A0A);
    for i in 0..ITERS {
        let size = rng.next_u64() as i64;
        let caf = random_valid(
            &rng,
            &Desc::random_ima4(&rng),
            &Pakt::random(&rng),
            size,
        );
        let out = assert_same(&format!("row10/{i}"), &caf.buf);
        assert_eq!(out.ret, 0);
        // s64 -> u64 is a bit-preserving reinterpretation, not a clamp.
        assert_eq!(out.info.size(), size as u64);
    }
}

#[test]
fn row11_data_size_boundaries() {
    let rng = Rng::new(0x0B0B);
    let sizes: [i64; 15] = [
        0,
        1,
        -1,
        2,
        -2,
        i64::MIN,
        i64::MAX,
        i64::MIN + 1,
        i64::MAX - 1,
        1 << 31,
        -(1i64 << 31),
        1 << 32,
        1 << 47,
        -(1i64 << 47),
        -16,
    ];
    for (i, &size) in sizes.iter().enumerate() {
        for rep in 0..40 {
            let caf = random_valid(
                &rng,
                &Desc::random_ima4(&rng),
                &Pakt::random(&rng),
                size,
            );
            let out = assert_same(&format!("row11/size={size}/{i}/{rep}"), &caf.buf);
            assert_eq!(out.ret, 0);
            assert_eq!(out.info.size(), size as u64);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 12 — filler chunk with size 0 (stride exactly sizeof(caf_chunk))
// ---------------------------------------------------------------------------

#[test]
fn row12_filler_chunk_size_zero() {
    let rng = Rng::new(0x0C0C);
    for i in 0..ITERS {
        let mut caf = Caf::valid_header(&rng);
        // A run of empty chunks: each advances by exactly 16 bytes.
        for _ in 0..(1 + rng.below(6)) {
            caf.chunk_raw(unknown_type(&rng), rng.arr4(), 0, &[]);
        }
        caf.desc(rng.arr4(), &Desc::random_ima4(&rng));
        caf.chunk_raw(unknown_type(&rng), rng.arr4(), 0, &[]);
        caf.pakt(rng.arr4(), &Pakt::random(&rng));
        caf.data(rng.arr4(), 55, rng.next_u32(), &[]);
        assert_eq!(assert_same(&format!("row12/{i}"), &caf.buf).ret, 0);
    }
}

// ---------------------------------------------------------------------------
// Row 13 — negative chunk size makes the walk run *backwards*
// ---------------------------------------------------------------------------

#[test]
fn row13_backward_walk_onto_data_chunk() {
    let rng = Rng::new(0x0D0D);
    for i in 0..ITERS {
        let desc = Desc::random_ima4(&rng);
        let pakt = Pakt::random(&rng);
        let mut caf = Caf::valid_header(&rng);
        caf.desc(rng.arr4(), &desc);
        caf.pakt(rng.arr4(), &pakt);

        // Forward jump: skip over the data chunk to the backward-jump chunk.
        let fwd = caf.chunk_raw(unknown_type(&rng), rng.arr4(), 0, &[]);
        let data_off = caf.offset();
        let trailing = rng.bytes(rng.below(32) as usize);
        caf.data(rng.arr4(), 4242, rng.next_u32(), &trailing);
        let back = caf.offset();
        caf.chunk_raw(unknown_type(&rng), rng.arr4(), 0, &[]);

        // chunk_next = chunk + 16 + size
        caf.set_chunk_size(fwd, (back as i64) - (fwd as i64 + 16));
        caf.set_chunk_size(back, (data_off as i64) - (back as i64 + 16));

        let out = assert_same(&format!("row13/{i}"), &caf.buf);
        assert_eq!(out.ret, 0, "backward walk should reach the data chunk");
        assert_eq!(out.info.size(), 4242);
        assert_eq!(out.info.blocks(), caf.expected_blocks(caf.buf.as_ptr()));
    }
}

// ---------------------------------------------------------------------------
// Row 14 — chunk tail padding is struct padding and must be inert
// ---------------------------------------------------------------------------

#[test]
fn row14_chunk_padding_inert() {
    let rng = Rng::new(0x0E0E);
    let desc = Desc::ima4();
    let pakt = Pakt::new(0x1234);

    // Baseline with zeroed padding.
    let mut base = Caf::new(*b"caff", 1, 0);
    base.desc([0; 4], &desc);
    base.pakt([0; 4], &pakt);
    base.data([0; 4], 64, 0, &[]);
    let baseline = assert_same("row14/baseline", &base.buf);

    for i in 0..ITERS {
        let mut caf = Caf::new(*b"caff", 1, 0);
        caf.desc(rng.arr4(), &desc);
        caf.pakt(rng.arr4(), &pakt);
        caf.data(rng.arr4(), 64, 0, &[]);
        let out = assert_same(&format!("row14/{i}"), &caf.buf);
        // Same layout, same addresses (equal-length buffers are irrelevant:
        // compare everything except the interior `blocks` pointer).
        assert_eq!(out.ret, baseline.ret);
        assert_eq!(out.info.size(), baseline.info.size());
        assert_eq!(out.info.frame_count(), baseline.info.frame_count());
        assert_eq!(out.info.channel_count(), baseline.info.channel_count());
        assert_eq!(
            out.info.sample_rate_bits(),
            baseline.info.sample_rate_bits(),
            "chunk struct padding must not be observable"
        );
    }
}

// ---------------------------------------------------------------------------
// Row 15 — header->flags is never read
// ---------------------------------------------------------------------------

#[test]
fn row15_header_flags_inert() {
    let rng = Rng::new(0x0F0F);
    let desc = Desc::ima4();
    let pakt = Pakt::new(99);

    let mut template = Caf::new(*b"caff", 1, 0);
    template.desc([0; 4], &desc);
    template.pakt([0; 4], &pakt);
    template.data([0; 4], 17, 0, &[]);

    let mut flags: Vec<u16> = vec![0, 1, 0xffff, 0x8000, 0x7fff, 0x00ff, 0xff00];
    for _ in 0..ITERS {
        flags.push(rng.next_u16());
    }

    let mut buf = template.buf.clone();
    let mut baseline: Option<Outcome> = None;
    for f in flags {
        buf[6..8].copy_from_slice(&f.to_be_bytes());
        let out = assert_same(&format!("row15/flags={f:#06x}"), &buf);
        assert_eq!(out.ret, 0);
        match baseline {
            None => baseline = Some(out),
            Some(b) => assert_eq!(out, b, "flags={f:#06x} changed the result"),
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 16-23 — the `sample_rate` double -> u64 -> bswap -> bitcast pipeline
// ---------------------------------------------------------------------------

/// Builds a valid stream whose `desc.sample_rate` raw bytes are exactly `raw`,
/// runs both implementations, and checks the reference model agrees too.
#[track_caller]
fn check_rate_raw(label: &str, rng: &Rng, raw: [u8; 8]) -> Outcome {
    let mut desc = Desc::random_ima4(rng);
    desc.sample_rate_raw = raw;
    let caf = random_valid(rng, &desc, &Pakt::random(rng), 1);
    let out = assert_same(label, &caf.buf);
    assert_eq!(out.ret, 0);

    // Cross-check against the documented model of the C's conversion.
    let native = f64::from_bits(u64::from_le_bytes(raw));
    let expected = model_f64_to_u64(native).swap_bytes();
    assert_eq!(
        out.info.sample_rate_bits(),
        expected,
        "[{label}] native double {native:?} (bits {:#018x}) -> expected sample_rate bits {expected:#018x}",
        native.to_bits()
    );
    out
}

#[test]
fn row16_realistic_big_endian_sample_rates() {
    let rng = Rng::new(0x1010);
    for &hz in &[8000.0f64, 11025.0, 16000.0, 22050.0, 44100.0, 48000.0, 96000.0, 192000.0] {
        for rep in 0..40 {
            let out = check_rate_raw(
                &format!("row16/{hz}/{rep}"),
                &rng,
                hz.to_be_bytes(),
            );
            // A big-endian rate read as a native double is a tiny subnormal, so
            // the C's truncation yields 0 and the reported rate is +0.0.
            assert_eq!(
                out.info.sample_rate_bits(),
                0,
                "realistic BE rate {hz} should degrade to +0.0"
            );
        }
    }
}

#[test]
fn row17_sample_rate_uniform_random_bytes() {
    let rng = Rng::new(0x1111);
    for i in 0..ITERS * 3 {
        let raw = rng.arr8();
        check_rate_raw(&format!("row17/{i}"), &rng, raw);
    }
}

#[test]
fn row18_sample_rate_in_well_defined_range() {
    let rng = Rng::new(0x1212);
    for i in 0..ITERS {
        // Uniform in [0, 2^63): the only range C actually defines.
        let v = (rng.next_u64() >> 1) as f64;
        check_rate_raw(&format!("row18/{i}"), &rng, v.to_bits().to_le_bytes());
    }
    // Plus small integral and fractional values.
    for i in 0..ITERS {
        let v = (rng.below(1 << 20) as f64) + (rng.below(1000) as f64) / 1000.0;
        check_rate_raw(&format!("row18b/{i}"), &rng, v.to_bits().to_le_bytes());
    }
}

#[test]
fn row19_sample_rate_in_subsd_branch() {
    let rng = Rng::new(0x1313);
    const TWO63: f64 = 9_223_372_036_854_775_808.0;
    for i in 0..ITERS {
        // Uniform in [2^63, 2^64): takes the `subsd`/`xor` path.
        let v = TWO63 + (rng.next_u64() >> 1) as f64;
        check_rate_raw(&format!("row19/{i}"), &rng, v.to_bits().to_le_bytes());
    }
}

#[test]
fn row20_sample_rate_at_or_above_two_pow_64() {
    let rng = Rng::new(0x1414);
    const TWO64: f64 = 18_446_744_073_709_551_616.0;
    let mut cases: Vec<f64> = vec![
        TWO64,
        TWO64 * 2.0,
        TWO64 * 1024.0,
        f64::MAX,
        f64::INFINITY,
        1e300,
        1e30,
    ];
    for _ in 0..ITERS {
        // Random large magnitudes, mostly well past 2^64.
        let e = 64 + rng.below(900) as i32;
        cases.push(2.0f64.powi(e) * (1.0 + (rng.below(1000) as f64) / 1000.0));
    }
    for (i, v) in cases.into_iter().enumerate() {
        check_rate_raw(&format!("row20/{i}/{v:e}"), &rng, v.to_bits().to_le_bytes());
    }
}

#[test]
fn row21_sample_rate_negative() {
    let rng = Rng::new(0x1515);
    const TWO63: f64 = 9_223_372_036_854_775_808.0;
    let mut cases: Vec<f64> = vec![
        -0.0,
        -1.0,
        -1.5,
        -0.5,
        -f64::MIN_POSITIVE,
        -f64::MAX,
        f64::NEG_INFINITY,
        -TWO63,
        -TWO63 + 1024.0,
        -TWO63 - 4096.0,
        -TWO63 * 2.0,
        -(1i64 << 62) as f64,
        -1e300,
    ];
    for _ in 0..ITERS {
        cases.push(-((rng.next_u64() >> 1) as f64));
    }
    for _ in 0..200 {
        cases.push(-((rng.below(1 << 30) as f64) / 7.0));
    }
    for (i, v) in cases.into_iter().enumerate() {
        check_rate_raw(&format!("row21/{i}/{v:e}"), &rng, v.to_bits().to_le_bytes());
    }
}

#[test]
fn row22_sample_rate_nan() {
    let rng = Rng::new(0x1616);
    let mut cases: Vec<u64> = vec![
        0x7ff8_0000_0000_0000, // canonical quiet NaN
        0xfff8_0000_0000_0000, // negative quiet NaN
        0x7ff0_0000_0000_0001, // signalling NaN
        0xfff0_0000_0000_0001, // negative signalling NaN
        0x7ff7_ffff_ffff_ffff,
        0x7fff_ffff_ffff_ffff,
        0xffff_ffff_ffff_ffff,
    ];
    for _ in 0..ITERS {
        // Random NaN payloads: exponent all-ones, non-zero mantissa.
        let payload = (rng.next_u64() & 0x000f_ffff_ffff_ffff) | 1;
        let sign = (rng.below(2)) << 63;
        cases.push(sign | 0x7ff0_0000_0000_0000 | payload);
    }
    for (i, bits) in cases.into_iter().enumerate() {
        let raw = bits.to_le_bytes();
        assert!(f64::from_bits(bits).is_nan(), "case {i} must be NaN");
        check_rate_raw(&format!("row22/{i}/{bits:#018x}"), &rng, raw);
    }
}

#[test]
fn row23_sample_rate_exact_conversion_boundaries() {
    let rng = Rng::new(0x1717);
    const TWO63: f64 = 9_223_372_036_854_775_808.0;
    const TWO64: f64 = 18_446_744_073_709_551_616.0;
    // 2^63 and 2^64 have 1024/2048 ULP spacing, so these are the true
    // neighbours of the branch boundaries.
    let cases: Vec<f64> = vec![
        0.0,
        -0.0,
        f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE,
        f64::from_bits(1),  // smallest subnormal
        f64::from_bits(0x8000_0000_0000_0001),
        0.5,
        -0.5,
        1.0,
        -1.0,
        (1u64 << 52) as f64,
        (1u64 << 53) as f64,
        (1u64 << 62) as f64,
        TWO63 - 2048.0,
        TWO63 - 1024.0,
        TWO63,
        TWO63 + 2048.0,
        TWO63 * 1.5,
        TWO64 - 4096.0,
        TWO64 - 2048.0,
        TWO64,
        TWO64 + 4096.0,
        -TWO63 + 1024.0,
        -TWO63,
        -TWO63 - 2048.0,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::MAX,
        -f64::MAX,
    ];
    for (i, v) in cases.into_iter().enumerate() {
        for rep in 0..8 {
            check_rate_raw(
                &format!("row23/{i}/{v:e}/{rep}"),
                &rng,
                v.to_bits().to_le_bytes(),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Row 24 — channels_per_frame is copied out via bswap32, unvalidated
// ---------------------------------------------------------------------------

#[test]
fn row24_channel_count_values() {
    let rng = Rng::new(0x1818);
    let mut cases: Vec<u32> = vec![
        0,
        1,
        2,
        3,
        8,
        0x0000_00ff,
        0x0000_ff00,
        0x00ff_0000,
        0xff00_0000,
        0x7fff_ffff,
        0x8000_0000,
        0xffff_ffff,
        0x0102_0304,
    ];
    for _ in 0..ITERS {
        cases.push(rng.next_u32());
    }
    for (i, ch) in cases.into_iter().enumerate() {
        let mut desc = Desc::random_ima4(&rng);
        desc.channels_per_frame = ch;
        let caf = random_valid(&rng, &desc, &Pakt::random(&rng), 3);
        let out = assert_same(&format!("row24/{i}/{ch:#010x}"), &caf.buf);
        assert_eq!(out.ret, 0);
        // Direct oracle for ima_btoh32/ima_bswap32: the field is stored
        // big-endian, so the C's swap must reproduce `ch` exactly.
        assert_eq!(out.info.channel_count(), ch);
    }
}

// ---------------------------------------------------------------------------
// Row 25 — frame_count is copied out via bswap64, unvalidated
// ---------------------------------------------------------------------------

#[test]
fn row25_frame_count_values() {
    let rng = Rng::new(0x1919);
    let mut cases: Vec<i64> = vec![
        0,
        1,
        -1,
        2,
        -2,
        i64::MIN,
        i64::MAX,
        i64::MIN + 1,
        i64::MAX - 1,
        1 << 31,
        1 << 32,
        1 << 62,
        -(1i64 << 62),
        0x0102_0304_0506_0708,
        u64::MAX as i64,
    ];
    for _ in 0..ITERS {
        cases.push(rng.next_u64() as i64);
    }
    for (i, fc) in cases.into_iter().enumerate() {
        let mut pakt = Pakt::random(&rng);
        pakt.frame_count = fc;
        let caf = random_valid(&rng, &Desc::random_ima4(&rng), &pakt, 5);
        let out = assert_same(&format!("row25/{i}/{fc}"), &caf.buf);
        assert_eq!(out.ret, 0);
        // Direct oracle for ima_btoh64/ima_bswap64: the field is stored
        // big-endian, so the C's swap must reproduce `fc` exactly.
        assert_eq!(out.info.frame_count(), fc as u64);
    }
}

// ---------------------------------------------------------------------------
// Row 26 — every field the C never reads must be inert
// ---------------------------------------------------------------------------

#[test]
fn row26_ignored_fields_inert() {
    let rng = Rng::new(0x1A1A);

    // Fixed values for everything the C *does* read.
    let rate = 12345.678f64.to_bits().to_le_bytes();
    let channels = 2u32;
    let frames = 4321i64;

    let mk = |rng: &Rng, garbage: bool| -> Caf {
        let mut desc = Desc {
            sample_rate_raw: rate,
            format_id: *b"ima4",
            format_flags: 0,
            bytes_per_packet: 0,
            frames_per_packet: 0,
            channels_per_frame: channels,
            bits_per_channel: 0,
        };
        let mut pakt = Pakt {
            packet_count: 0,
            frame_count: frames,
            priming_frames: 0,
            remainder_frames: 0,
        };
        let mut edit = 0u32;
        if garbage {
            desc.format_flags = rng.next_u32();
            desc.bytes_per_packet = rng.next_u32();
            desc.frames_per_packet = rng.next_u32();
            desc.bits_per_channel = rng.next_u32();
            pakt.packet_count = rng.next_u64() as i64;
            pakt.priming_frames = rng.next_u32() as i32;
            pakt.remainder_frames = rng.next_u32() as i32;
            edit = rng.next_u32();
        }
        let mut caf = Caf::new(*b"caff", 1, 0);
        caf.desc([0; 4], &desc);
        caf.pakt([0; 4], &pakt);
        caf.data([0; 4], 31, edit, &[]);
        caf
    };

    let clean = mk(&rng, false);
    let baseline = assert_same("row26/clean", &clean.buf);
    assert_eq!(baseline.ret, 0);

    for i in 0..ITERS {
        let caf = mk(&rng, true);
        let out = assert_same(&format!("row26/{i}"), &caf.buf);
        assert_eq!(out.ret, baseline.ret);
        assert_eq!(out.info.size(), baseline.info.size());
        assert_eq!(out.info.frame_count(), baseline.info.frame_count());
        assert_eq!(out.info.channel_count(), baseline.info.channel_count());
        assert_eq!(out.info.sample_rate_bits(), baseline.info.sample_rate_bits());
    }
}

// ---------------------------------------------------------------------------
// Row 27 — misaligned buffer base (C uses aligned struct types, Rust uses
// read_unaligned)
// ---------------------------------------------------------------------------

#[test]
fn row27_misaligned_buffer_base() {
    let rng = Rng::new(0x1B1B);
    for i in 0..250 {
        let desc = Desc::random_ima4(&rng);
        let pakt = Pakt::random(&rng);
        let caf = random_valid(&rng, &desc, &pakt, rng.next_u64() as i64);

        for residue in 0..8usize {
            let (backing, k) = caf.aligned_copy(residue);
            let slice = &backing[k..];
            assert_eq!(slice.as_ptr() as usize % 8, residue);
            let out = assert_same(&format!("row27/{i}/residue={residue}"), slice);
            assert_eq!(out.ret, 0);
            assert_eq!(
                out.info.blocks(),
                caf.expected_blocks(slice.as_ptr()),
                "blocks pointer must track the misaligned base"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Row 28 — full random cross-product
// ---------------------------------------------------------------------------

#[test]
fn row28_full_random_cross_product() {
    let rng = Rng::new(0x1C1C);
    for i in 0..200_000 {
        // Random sample_rate strategy, covering all conversion branches.
        let raw = match rng.below(6) {
            0 => rng.arr8(),
            1 => rng.pick(&[44100.0f64, 48000.0, 8000.0]).to_be_bytes(),
            2 => ((rng.next_u64() >> 1) as f64).to_bits().to_le_bytes(),
            3 => (-((rng.next_u64() >> 1) as f64)).to_bits().to_le_bytes(),
            4 => (9_223_372_036_854_775_808.0f64 + (rng.next_u64() >> 1) as f64)
                .to_bits()
                .to_le_bytes(),
            _ => (0x7ff0_0000_0000_0000u64 | (rng.next_u64() & 0xf_ffff_ffff_ffff) | 1)
                .to_le_bytes(),
        };
        let mut desc = Desc::random_ima4(&rng);
        desc.sample_rate_raw = raw;
        let pakt = Pakt::random(&rng);
        let size = rng.pick(&[
            0i64,
            1,
            -1,
            i64::MIN,
            i64::MAX,
            rng.next_u64() as i64,
            1 << 40,
        ]);

        // Random chunk-list shape.
        let mut caf = Caf::valid_header(&rng);
        let desc_first = rng.below(2) == 0;
        push_fillers(&mut caf, &rng, rng.below(4) as usize);
        if desc_first {
            caf.desc(rng.arr4(), &desc);
            push_fillers(&mut caf, &rng, rng.below(4) as usize);
            caf.pakt(rng.arr4(), &pakt);
        } else {
            caf.pakt(rng.arr4(), &pakt);
            push_fillers(&mut caf, &rng, rng.below(4) as usize);
            caf.desc(rng.arr4(), &desc);
        }
        push_fillers(&mut caf, &rng, rng.below(4) as usize);
        let trailing = rng.bytes(rng.below(64) as usize);
        caf.data(rng.arr4(), size, rng.next_u32(), &trailing);

        let out = assert_same(&format!("row28/{i}"), &caf.buf);
        assert_eq!(out.ret, 0);
        assert_eq!(out.info.size(), size as u64);
        assert_eq!(out.info.frame_count(), pakt.frame_count as u64);
        assert_eq!(out.info.channel_count(), desc.channels_per_frame);
        assert_eq!(out.info.blocks(), caf.expected_blocks(caf.buf.as_ptr()));
    }
}

// ---------------------------------------------------------------------------
// Row 29 — the blocks pointer is data_chunk + 20
// ---------------------------------------------------------------------------

#[test]
fn row29_blocks_pointer_offset() {
    let rng = Rng::new(0x1D1D);
    for i in 0..ITERS {
        let caf = random_valid(
            &rng,
            &Desc::random_ima4(&rng),
            &Pakt::random(&rng),
            1,
        );
        let out = assert_same(&format!("row29/{i}"), &caf.buf);
        assert_eq!(out.ret, 0);
        let data_off = caf.data_chunk_off.unwrap();
        assert_eq!(
            out.info.blocks(),
            caf.buf.as_ptr() as u64 + (data_off + BLOCKS_OFF_FROM_CHUNK) as u64,
            "blocks must be data_chunk + sizeof(caf_chunk) + sizeof(caf_data) = +20"
        );
    }
}

// ---------------------------------------------------------------------------
// Row 30 — exhaustive sweep of all 65536 header versions
// ---------------------------------------------------------------------------

#[test]
fn row30_sweep_all_65536_version_values() {
    let rng = Rng::new(0x1E1E);
    let mut caf = Caf::new(*b"caff", 1, 0);
    caf.desc([0; 4], &Desc::random_ima4(&rng));
    caf.pakt([0; 4], &Pakt::random(&rng));
    caf.data([0; 4], 8, 0, &[]);
    let mut buf = caf.buf.clone();

    let mut accepted = 0usize;
    for v in 0..=u16::MAX {
        buf[4..6].copy_from_slice(&v.to_be_bytes());
        let (c, r) = run_both(&buf);
        assert_eq!(c, r, "version sweep diverged at v={v:#06x}");
        if c.ret != -2 {
            accepted += 1;
            assert_eq!(v, 1, "only version 1 may be accepted");
            assert_eq!(c.ret, 0);
        }
    }
    assert_eq!(accepted, 1, "exactly one version value must be accepted");
}

// ---------------------------------------------------------------------------
// Row 31 — header type values, incl. every single-byte mutation of "caff"
// ---------------------------------------------------------------------------

#[test]
fn row31_header_type_values_and_mutations() {
    let rng = Rng::new(0x1F1F);
    let mut caf = Caf::new(*b"caff", 1, 0);
    caf.desc([0; 4], &Desc::random_ima4(&rng));
    caf.pakt([0; 4], &Pakt::random(&rng));
    caf.data([0; 4], 8, 0, &[]);
    let mut buf = caf.buf.clone();

    // All 4 * 256 single-byte mutations of the magic.
    for pos in 0..4usize {
        for b in 0..=255u8 {
            let mut t = *b"caff";
            t[pos] = b;
            buf[0..4].copy_from_slice(&t);
            let (c, r) = run_both(&buf);
            assert_eq!(c, r, "type sweep diverged at pos={pos} b={b:#04x}");
            if t == *b"caff" {
                assert_eq!(c.ret, 0);
            } else {
                assert_eq!(c.ret, -1, "type {t:?} must be rejected with -1");
            }
        }
    }
    // Plus random 4-byte types.
    for _ in 0..ITERS {
        let t = rng.arr4();
        buf[0..4].copy_from_slice(&t);
        let (c, r) = run_both(&buf);
        assert_eq!(c, r, "random type diverged at {t:?}");
        assert_eq!(c.ret, if t == *b"caff" { 0 } else { -1 });
    }
}

// ---------------------------------------------------------------------------
// Row 32 — chunk type values, incl. single-byte mutations of the known codes
// (the closest analogue of an out-of-range enum discriminant in this ABI)
// ---------------------------------------------------------------------------

#[test]
fn row32_chunk_type_values_and_mutations() {
    let rng = Rng::new(0x2020);

    // A stream where the chunk under test sits between a valid desc/pakt pair
    // and the terminating data chunk. Its payload is 32 bytes so that even if a
    // mutation turns it into a `desc`, every field read stays in bounds.
    let build = |rng: &Rng, t: [u8; 4]| -> Caf {
        let mut caf = Caf::new(*b"caff", 1, 0);
        caf.desc([0; 4], &Desc::ima4());
        caf.pakt([0; 4], &Pakt::new(7));
        let payload = {
            // Valid ima4 desc bytes, so a `desc` mutation still returns 0.
            let mut d = Desc::ima4();
            d.channels_per_frame = 9;
            d.encode()
        };
        caf.chunk(t, rng.arr4(), &payload);
        caf.data([0; 4], 21, 0, &[]);
        caf
    };

    for known in [*b"desc", *b"pakt", *b"data"] {
        for pos in 0..4usize {
            for b in 0..=255u8 {
                let mut t = known;
                t[pos] = b;
                let caf = build(&rng, t);
                let (c, r) = run_both(&caf.buf);
                assert_eq!(
                    c, r,
                    "chunk-type mutation diverged: known={:?} pos={pos} b={b:#04x} -> {t:?}",
                    std::str::from_utf8(&known)
                );
            }
        }
    }

    // Random 32-bit chunk types: the open-discriminant space.
    for i in 0..ITERS {
        let t = rng.arr4();
        let caf = build(&rng, t);
        let (c, r) = run_both(&caf.buf);
        assert_eq!(c, r, "random chunk type {t:?} diverged (iter {i})");
        if !is_known_type(t) {
            assert_eq!(c.ret, 0, "unknown chunk type must be skipped");
        }
    }
}

// ---------------------------------------------------------------------------
// Row 33 — all 40 bytes of ima_info, incl. tail padding, on every exit path
// ---------------------------------------------------------------------------

#[test]
fn row33_full_info_struct_including_padding() {
    let rng = Rng::new(0x2121);

    // Success path.
    for i in 0..500 {
        let caf = random_valid(
            &rng,
            &Desc::random_ima4(&rng),
            &Pakt::random(&rng),
            rng.next_u64() as i64,
        );
        let (c, r) = run_both(&caf.buf);
        assert_eq!(c, r, "row33/success/{i}");
        assert_eq!(c.ret, 0);
        assert_eq!(
            c.info.tail_padding(),
            [POISON; 4],
            "tail padding after channel_count must never be written"
        );
    }

    // Every error path must leave `info` entirely untouched.
    let poisoned = InfoBuf::poisoned();

    let mut bad_magic = Caf::new(*b"xxxx", 1, 0);
    bad_magic.desc([0; 4], &Desc::ima4());
    bad_magic.pakt([0; 4], &Pakt::new(1));
    bad_magic.data([0; 4], 0, 0, &[]);
    let (c, r) = run_both(&bad_magic.buf);
    assert_eq!(c, r);
    assert_eq!(c.ret, -1);
    assert_eq!(c.info, poisoned, "-1 path must not write info");

    let mut bad_ver = Caf::new(*b"caff", 0, 0);
    bad_ver.desc([0; 4], &Desc::ima4());
    bad_ver.pakt([0; 4], &Pakt::new(1));
    bad_ver.data([0; 4], 0, 0, &[]);
    let (c, r) = run_both(&bad_ver.buf);
    assert_eq!(c, r);
    assert_eq!(c.ret, -2);
    assert_eq!(c.info, poisoned, "-2 path must not write info");

    let mut bad_fmt = Caf::new(*b"caff", 1, 0);
    let mut d = Desc::ima4();
    d.format_id = *b"lpcm";
    bad_fmt.desc([0; 4], &d);
    bad_fmt.pakt([0; 4], &Pakt::new(1));
    bad_fmt.data([0; 4], 0, 0, &[]);
    let (c, r) = run_both(&bad_fmt.buf);
    assert_eq!(c, r);
    assert_eq!(c.ret, -3);
    assert_eq!(c.info, poisoned, "-3 path must not write info");
}

// ---------------------------------------------------------------------------
// High-volume fuzz of the sample_rate pipeline
//
// `desc->sample_rate` is by far the riskiest field: the C reads 8 raw bytes as a
// *native* double and then performs a `double` -> `unsigned long long` value
// conversion, which C leaves undefined for negative, NaN, infinite, and
// >= 2^64 inputs. The Rust reproduces the x86-64 lowering instead of using
// Rust's saturating `as` cast, so this path gets an order of magnitude more
// randomized inputs than any other.
// ---------------------------------------------------------------------------

#[test]
fn fuzz_sample_rate_pipeline() {
    let rng = Rng::new(0xF00D_5EED);

    // One fixed stream, with only the 8 sample_rate bytes rewritten per
    // iteration, so the whole loop is just two FFI calls and a compare.
    let mut caf = Caf::new(*b"caff", 1, 0);
    let desc_off = caf.desc([0; 4], &Desc::ima4());
    caf.pakt([0; 4], &Pakt::new(1234));
    caf.data([0; 4], 99, 0, &[]);
    let rate_at = desc_off + CHUNK_HEADER_LEN; // sample_rate is at desc + 0
    let mut buf = caf.buf.clone();

    let mut branch_hits = [0usize; 4]; // [in range, subsd branch, negative, nan]

    for i in 0..1_000_000u32 {
        // Mix of fully random bytes and targeted exponent ranges, so the rare
        // interesting classes are hit densely rather than by chance.
        let bits = match i % 8 {
            0 | 1 => u64::from_le_bytes(rng.arr8()),
            2 => ((rng.next_u64() >> 1) as f64).to_bits(),
            3 => (-((rng.next_u64() >> 1) as f64)).to_bits(),
            4 => (9_223_372_036_854_775_808.0f64 + (rng.next_u64() >> 1) as f64).to_bits(),
            5 => {
                // Random value with an exponent straddling the 2^63/2^64 edges.
                let exp = 1000 + rng.below(80); // biased exponent ~ 2^-23..2^57
                (exp << 52) | (rng.next_u64() & 0x000f_ffff_ffff_ffff)
            }
            6 => 0x7ff0_0000_0000_0000 | (rng.next_u64() & 0x000f_ffff_ffff_ffff),
            _ => {
                let sign = rng.below(2) << 63;
                sign | ((1086 - rng.below(8)) << 52) | (rng.next_u64() & 0xffff_ffff_ffff)
            }
        };

        buf[rate_at..rate_at + 8].copy_from_slice(&bits.to_le_bytes());
        let (c, r) = run_both(&buf);
        if c != r {
            panic!(
                "DIVERGENCE fuzz_sample_rate/{i}: raw bits {bits:#018x} \
                 (native double {:?})\n  C    {:?}\n  Rust {:?}",
                f64::from_bits(bits),
                c,
                r
            );
        }
        assert_eq!(c.ret, 0);

        // Independent cross-check against the documented model.
        let native = f64::from_bits(bits);
        assert_eq!(
            c.info.sample_rate_bits(),
            model_f64_to_u64(native).swap_bytes(),
            "model disagrees at bits {bits:#018x} (native {native:?})"
        );

        const TWO63: f64 = 9_223_372_036_854_775_808.0;
        if native.is_nan() {
            branch_hits[3] += 1;
        } else if native < 0.0 {
            branch_hits[2] += 1;
        } else if native >= TWO63 {
            branch_hits[1] += 1;
        } else {
            branch_hits[0] += 1;
        }
    }

    // Prove the fuzzer actually reached every branch of the conversion rather
    // than hammering one easy path.
    let names = ["[0,2^63)", ">=2^63 (subsd)", "negative", "NaN"];
    for (n, &hits) in names.iter().zip(branch_hits.iter()) {
        assert!(hits > 1000, "conversion branch {n} only hit {hits} times");
    }
    println!("sample_rate conversion branch coverage: {branch_hits:?} ({names:?})");
}
