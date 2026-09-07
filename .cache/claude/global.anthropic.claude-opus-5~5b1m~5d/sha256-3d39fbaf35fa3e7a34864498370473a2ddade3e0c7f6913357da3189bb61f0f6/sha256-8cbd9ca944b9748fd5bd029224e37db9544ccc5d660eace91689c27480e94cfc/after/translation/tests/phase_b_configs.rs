//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`, each driven with many randomized inputs
//! (fixed seeds). Both the C `.so` and the Rust `.so` are loaded with
//! `libloading` and `dequantize_granule` is called through the FFI boundary on
//! each; every observable (`grbuf` bit patterns, `bs->pos`, `bs->limit`, the
//! whole `L12_scale_info`, and the return value) must match byte-for-byte.

mod harness;

use harness::*;

const SEED: u64 = 0x5EED_1234;

fn sci_with(total_bands: u8, ba: &[u8]) -> Sci {
    let mut s = Sci::zeroed();
    s.total_bands = total_bands;
    for (i, &v) in ba.iter().enumerate() {
        if i < 64 {
            s.bitalloc[i] = v;
        } else {
            s.scfcod[i - 64] = v;
        }
    }
    s
}

fn sci_uniform(total_bands: u8, ba: u8) -> Sci {
    let n = (2 * total_bands as usize).min(128);
    sci_with(total_bands, &vec![ba; n])
}

/// Randomize the untouched parts of `sci` (scf, stereo_bands, unused bitalloc /
/// scfcod tail) so any accidental read of them would show up.
fn fuzz_unused(sci: &mut Sci, rng: &mut Rng) {
    for v in sci.scf.iter_mut() {
        *v = f32::from_bits(rng.next_u32());
    }
    sci.stereo_bands = rng.next_u32() as u8;
}

// ---------------------------------------------------------------------------
// C1 — no bands at all
// ---------------------------------------------------------------------------
#[test]
fn c1_no_bands() {
    let mut rng = Rng::new(SEED ^ 1);
    for _ in 0..64 {
        let buf = random_buf(&mut rng);
        let mut sci = Sci::zeroed();
        sci.total_bands = 0;
        // bitalloc is deliberately non-zero: nothing may be consumed anyway.
        for i in 0..64 {
            sci.bitalloc[i] = rng.next_u32() as u8;
            sci.scfcod[i] = rng.next_u32() as u8;
        }
        fuzz_unused(&mut sci, &mut rng);
        let gs = rng.range(0, 32) as i32;
        let pos = rng.range(0, 4096) as i32;
        diff_case_must_run("C1", &buf, &sci, gs, pos, ample_limit(buf.len()));
    }
}

// ---------------------------------------------------------------------------
// C2 — ba = 1 (half = 0), minimal linear path
// ---------------------------------------------------------------------------
#[test]
fn c2_ba1_minimal() {
    let mut rng = Rng::new(SEED ^ 2);
    for _ in 0..64 {
        let buf = random_buf(&mut rng);
        let mut sci = sci_uniform(1, 1);
        fuzz_unused(&mut sci, &mut rng);
        diff_case_must_run("C2", &buf, &sci, 18, 0, ample_limit(buf.len()));
    }
}

// ---------------------------------------------------------------------------
// C3 — sweep ba = 2..15
// ---------------------------------------------------------------------------
#[test]
fn c3_ba_linear_sweep() {
    let mut rng = Rng::new(SEED ^ 3);
    for ba in 2u8..=15 {
        for _ in 0..32 {
            let buf = random_buf(&mut rng);
            let mut sci = sci_uniform(1, ba);
            fuzz_unused(&mut sci, &mut rng);
            diff_case_must_run("C3", &buf, &sci, 18, 0, ample_limit(buf.len()));
        }
    }
}

// ---------------------------------------------------------------------------
// C4 — ba = 16, the largest value on the linear path (half = 0x7FFF)
// ---------------------------------------------------------------------------
#[test]
fn c4_ba16_max_linear() {
    let mut rng = Rng::new(SEED ^ 4);
    for _ in 0..64 {
        let buf = random_buf(&mut rng);
        let mut sci = sci_uniform(2, 16);
        fuzz_unused(&mut sci, &mut rng);
        diff_case_must_run("C4", &buf, &sci, 18, 0, ample_limit(buf.len()));
    }
}

// ---------------------------------------------------------------------------
// C5 — all eight bit phases (bs->pos & 7 == 0..7) crossed with ba = 1..16
// ---------------------------------------------------------------------------
#[test]
fn c5_all_bit_phases() {
    let mut rng = Rng::new(SEED ^ 5);
    for phase in 0..8i32 {
        for ba in 1u8..=16 {
            for _ in 0..4 {
                let buf = random_buf(&mut rng);
                let mut sci = sci_uniform(2, ba);
                fuzz_unused(&mut sci, &mut rng);
                let pos = (rng.range(0, 500) as i32) * 8 + phase;
                diff_case_must_run("C5", &buf, &sci, 12, pos, ample_limit(buf.len()));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C6 — single-byte fields: n + s <= 8, so the `while` body never runs
// ---------------------------------------------------------------------------
#[test]
fn c6_single_byte_fields() {
    let mut rng = Rng::new(SEED ^ 6);
    let mut ran = 0usize;
    for phase in 0..8i32 {
        for ba in 1u8..=8 {
            if (ba as i32) + phase > 8 {
                continue;
            }
            for _ in 0..8 {
                let buf = random_buf(&mut rng);
                // A single band + group_size 1 keeps every call at the same phase.
                let mut sci = sci_with(1, &[ba, 0]);
                fuzz_unused(&mut sci, &mut rng);
                let pos = (rng.range(0, 500) as i32) * 8 + phase;
                diff_case_must_run("C6", &buf, &sci, 1, pos, ample_limit(buf.len()));
                ran += 1;
            }
        }
    }
    assert!(ran >= 8, "C6 executed too few cases: {ran}");
}

// ---------------------------------------------------------------------------
// C7 — multi-iteration `while`: ba = 16 at phase 7 => shl = 23
// ---------------------------------------------------------------------------
#[test]
fn c7_multi_byte_span() {
    let mut rng = Rng::new(SEED ^ 7);
    for _ in 0..64 {
        let buf = random_buf(&mut rng);
        let mut sci = sci_with(1, &[16, 0]);
        fuzz_unused(&mut sci, &mut rng);
        let pos = (rng.range(0, 500) as i32) * 8 + 7;
        diff_case_must_run("C7", &buf, &sci, 1, pos, ample_limit(buf.len()));
    }
}

// ---------------------------------------------------------------------------
// C8 — grouped path, ba = 17 (mod = 3, n = 5), all bit phases
// ---------------------------------------------------------------------------
#[test]
fn c8_grouped_ba17() {
    let mut rng = Rng::new(SEED ^ 8);
    assert_eq!(grouped_params(17), (3, 5));
    for phase in 0..8i32 {
        for _ in 0..16 {
            let buf = random_buf(&mut rng);
            let mut sci = sci_uniform(2, 17);
            fuzz_unused(&mut sci, &mut rng);
            let pos = (rng.range(0, 500) as i32) * 8 + phase;
            diff_case_must_run("C8", &buf, &sci, 18, pos, ample_limit(buf.len()));
        }
    }
}

// ---------------------------------------------------------------------------
// C9 — grouped path sweep ba = 18..24 (mod = 5,9,17,33,65,129,257)
// ---------------------------------------------------------------------------
#[test]
fn c9_grouped_ba_sweep() {
    let mut rng = Rng::new(SEED ^ 9);
    let expected_mod = [5u32, 9, 17, 33, 65, 129, 257];
    for (idx, ba) in (18u8..=24).enumerate() {
        assert_eq!(grouped_params(ba as i32).0, expected_mod[idx], "ba={ba}");
        for _ in 0..32 {
            let buf = random_buf(&mut rng);
            let mut sci = sci_uniform(2, ba);
            fuzz_unused(&mut sci, &mut rng);
            let pos = rng.range(0, 4000) as i32;
            diff_case_must_run("C9", &buf, &sci, 18, pos, ample_limit(buf.len()));
        }
    }
}

// ---------------------------------------------------------------------------
// C10 — ba = 48: 2 << 31 overflows to 0 => mod = 1 => every output is 0.0f
// ---------------------------------------------------------------------------
#[test]
fn c10_grouped_mod_one() {
    let mut rng = Rng::new(SEED ^ 10);
    assert_eq!(grouped_params(48), (1, 3));
    for ba in [48u8, 80, 112, 144, 176, 208, 240] {
        assert_eq!(grouped_params(ba as i32), (1, 3), "ba={ba}");
        for _ in 0..16 {
            let buf = random_buf(&mut rng);
            let mut sci = sci_uniform(4, ba);
            fuzz_unused(&mut sci, &mut rng);
            let pos = rng.range(0, 4000) as i32;
            diff_case_must_run("C10", &buf, &sci, 18, pos, ample_limit(buf.len()));
        }
    }
}

// ---------------------------------------------------------------------------
// C11 — masked-shift aliasing for large ba; NO_READ_LIMIT keeps the enormous
//       field widths from dereferencing anything.
// ---------------------------------------------------------------------------
#[test]
fn c11_grouped_shift_alias() {
    let mut rng = Rng::new(SEED ^ 11);
    // (ba - 17) & 31 aliasing: ba and ba+32 must behave identically.
    for ba in [
        25u8, 26, 30, 33, 40, 46, 47, 49, 50, 51, 55, 64, 65, 79, 81, 100, 128, 200, 254, 255,
    ] {
        for _ in 0..8 {
            let buf = random_buf(&mut rng);
            let mut sci = sci_uniform(3, ba);
            fuzz_unused(&mut sci, &mut rng);
            let gs = rng.range(1, 18) as i32;
            diff_case_must_run("C11", &buf, &sci, gs, 0, NO_READ_LIMIT);
        }
    }
    // Also exercise the ones whose field width really is readable.
    for ba in [25u8, 26, 27, 28, 29, 30, 31, 32] {
        let (_m, n) = grouped_params(ba as i32);
        if n > 32_768 {
            continue;
        }
        for _ in 0..8 {
            let buf = random_buf(&mut rng);
            let mut sci = sci_with(1, &[ba, 0]);
            fuzz_unused(&mut sci, &mut rng);
            diff_case_must_run("C11r", &buf, &sci, 4, 0, ample_limit(buf.len()));
        }
    }
}

// ---------------------------------------------------------------------------
// C12 — mixed bitalloc across 2..16 bands, including zeros
// ---------------------------------------------------------------------------
#[test]
fn c12_mixed_bands_random() {
    let mut rng = Rng::new(SEED ^ 12);
    for _ in 0..256 {
        let buf = random_buf(&mut rng);
        let tb = rng.range(2, 16) as u8;
        let n = 2 * tb as usize;
        let ba: Vec<u8> = (0..n)
            .map(|_| match rng.below(4) {
                0 => 0,
                1 => rng.range(1, 16) as u8,
                2 => rng.range(17, 24) as u8,
                _ => rng.range(1, 16) as u8,
            })
            .collect();
        let mut sci = sci_with(tb, &ba);
        fuzz_unused(&mut sci, &mut rng);
        let pos = rng.range(0, 2000) as i32;
        diff_case_must_run("C12", &buf, &sci, 18, pos, ample_limit(buf.len()));
    }
}

// ---------------------------------------------------------------------------
// C13 — total_bands = 32: `i` walks exactly bitalloc[0..63]
// ---------------------------------------------------------------------------
#[test]
fn c13_total_bands_32_full() {
    let mut rng = Rng::new(SEED ^ 13);
    for _ in 0..64 {
        let buf = random_buf(&mut rng);
        let ba: Vec<u8> = (0..64)
            .map(|_| match rng.below(3) {
                0 => 0,
                1 => rng.range(1, 16) as u8,
                _ => rng.range(17, 22) as u8,
            })
            .collect();
        let mut sci = sci_with(32, &ba);
        fuzz_unused(&mut sci, &mut rng);
        diff_case_must_run("C13", &buf, &sci, 18, 0, ample_limit(buf.len()));
    }
}

// ---------------------------------------------------------------------------
// C14 — total_bands 33..64: `i` runs off bitalloc into scfcod (still in-object)
// ---------------------------------------------------------------------------
#[test]
fn c14_total_bands_into_scfcod() {
    let mut rng = Rng::new(SEED ^ 14);
    for tb in [33u8, 34, 40, 48, 63, 64] {
        for _ in 0..24 {
            let buf = random_buf(&mut rng);
            let n = 2 * tb as usize; // up to 128
            let ba: Vec<u8> = (0..n)
                .map(|_| match rng.below(3) {
                    0 => 0,
                    1 => rng.range(1, 16) as u8,
                    _ => rng.range(17, 21) as u8,
                })
                .collect();
            let mut sci = sci_with(tb, &ba);
            fuzz_unused(&mut sci, &mut rng);
            diff_case_must_run("C14", &buf, &sci, 12, 0, ample_limit(buf.len()));
        }
    }
}

// ---------------------------------------------------------------------------
// C15 — group_size sweep
// ---------------------------------------------------------------------------
#[test]
fn c15_group_size_sweep() {
    let mut rng = Rng::new(SEED ^ 15);
    for gs in [1i32, 2, 3, 4, 5, 6, 7, 8, 12, 16, 18, 24, 32, 64] {
        for _ in 0..24 {
            let buf = random_buf(&mut rng);
            let ba: Vec<u8> = (0..4)
                .map(|_| match rng.below(3) {
                    0 => 0,
                    1 => rng.range(1, 16) as u8,
                    _ => rng.range(17, 22) as u8,
                })
                .collect();
            let mut sci = sci_with(2, &ba);
            fuzz_unused(&mut sci, &mut rng);
            let pos = rng.range(0, 1000) as i32;
            diff_case_must_run("C15", &buf, &sci, gs, pos, ample_limit(buf.len()));
        }
    }
}

// ---------------------------------------------------------------------------
// C16 — group_size = 1 on the grouped path (code divided exactly once)
// ---------------------------------------------------------------------------
#[test]
fn c16_group_size_one_grouped() {
    let mut rng = Rng::new(SEED ^ 16);
    for ba in 17u8..=24 {
        for _ in 0..16 {
            let buf = random_buf(&mut rng);
            let mut sci = sci_uniform(4, ba);
            fuzz_unused(&mut sci, &mut rng);
            diff_case_must_run("C16", &buf, &sci, 1, 0, ample_limit(buf.len()));
        }
    }
}

// ---------------------------------------------------------------------------
// C17/C18/C19 — fixed bitstream byte patterns
// ---------------------------------------------------------------------------
fn patterned_case(label: &str, pattern: &[u8], seed: u64) {
    let mut rng = Rng::new(seed);
    for _ in 0..24 {
        let mut buf = vec![0u8; BUF_LEN];
        for (i, b) in buf.iter_mut().enumerate() {
            *b = pattern[i % pattern.len()];
        }
        let ba: Vec<u8> = (0..64).map(|_| rng.range(1, 16) as u8).collect();
        let mut sci = sci_with(32, &ba);
        fuzz_unused(&mut sci, &mut rng);
        let pos = rng.range(0, 64) as i32;
        diff_case_must_run(label, &buf, &sci, 18, pos, ample_limit(buf.len()));
    }
}

#[test]
fn c17_buf_all_zeros() {
    patterned_case("C17", &[0x00], SEED ^ 17);
}

#[test]
fn c18_buf_all_ones() {
    patterned_case("C18", &[0xFF], SEED ^ 18);
}

#[test]
fn c19_buf_patterns() {
    patterned_case("C19a", &[0xAA, 0x55], SEED ^ 19);
    patterned_case("C19b", &[0x0F, 0xF0], SEED ^ 20);
    patterned_case("C19c", &[0x80, 0x00, 0x00, 0x01], SEED ^ 21);
    patterned_case("C19d", &[0x7F, 0xFF, 0x80, 0x00], SEED ^ 22);
}

// ---------------------------------------------------------------------------
// C20 — limit lands exactly on the end of the last field (accepted, `>` not `>=`)
// ---------------------------------------------------------------------------
#[test]
fn c20_limit_exact() {
    let mut rng = Rng::new(SEED ^ 23);
    for _ in 0..96 {
        let buf = random_buf(&mut rng);
        let tb = rng.range(1, 8) as u8;
        let ba: Vec<u8> = (0..2 * tb as usize)
            .map(|_| rng.range(1, 16) as u8)
            .collect();
        let mut sci = sci_with(tb, &ba);
        fuzz_unused(&mut sci, &mut rng);
        let gs = rng.range(1, 18) as i32;
        let pos0 = rng.range(0, 32) as i32;
        // Total demand: every band is linear, so ba bits per k per granule.
        let per_granule: i64 = ba.iter().map(|&b| b as i64 * gs as i64).sum();
        let demand = per_granule * 4;
        let exact = pos0 as i64 + demand;
        assert!(exact < ample_limit(buf.len()) as i64);
        // exact => last field just fits
        diff_case_must_run("C20-exact", &buf, &sci, gs, pos0, exact as i32);
        // one bit short => the last field is rejected
        if demand > 0 {
            diff_case_must_run("C20-short", &buf, &sci, gs, pos0, (exact - 1) as i32);
        }
    }
}

// ---------------------------------------------------------------------------
// C21 — limit crossed part-way through the granule
// ---------------------------------------------------------------------------
#[test]
fn c21_limit_mid() {
    let mut rng = Rng::new(SEED ^ 24);
    for _ in 0..128 {
        let buf = random_buf(&mut rng);
        let ba: Vec<u8> = (0..32)
            .map(|_| match rng.below(3) {
                0 => 0,
                1 => rng.range(1, 16) as u8,
                _ => rng.range(17, 22) as u8,
            })
            .collect();
        let mut sci = sci_with(16, &ba);
        fuzz_unused(&mut sci, &mut rng);
        let gs = 18;
        let per_granule: i64 = ba
            .iter()
            .map(|&b| {
                let b = b as i32;
                if b == 0 {
                    0
                } else if b < 17 {
                    b as i64 * gs as i64
                } else {
                    grouped_params(b).1 as i64
                }
            })
            .sum();
        let demand = per_granule * 4;
        for pct in [10u32, 30, 50, 60, 75, 90, 99] {
            let limit = (demand * pct as i64 / 100) as i32;
            diff_case_must_run("C21", &buf, &sci, gs, 0, limit);
        }
    }
}

// ---------------------------------------------------------------------------
// C22 — choff carry across the four granules
// ---------------------------------------------------------------------------
#[test]
fn c22_choff_carry_across_granules() {
    let mut rng = Rng::new(SEED ^ 25);
    // total_bands = 1 => 2 bands per granule => 8 dst steps overall, so the
    // +576 / -558 alternation has to be preserved from one `j` to the next.
    for tb in [1u8, 2, 3, 5] {
        for _ in 0..32 {
            let buf = random_buf(&mut rng);
            let ba: Vec<u8> = (0..2 * tb as usize)
                .map(|_| rng.range(1, 16) as u8)
                .collect();
            let mut sci = sci_with(tb, &ba);
            fuzz_unused(&mut sci, &mut rng);
            for gs in [1i32, 2, 18] {
                diff_case_must_run("C22", &buf, &sci, gs, 0, ample_limit(buf.len()));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C23 — full randomized cross-product of every axis
// ---------------------------------------------------------------------------
#[test]
fn c23_full_random_fuzz() {
    let mut rng = Rng::new(SEED ^ 26);
    let mut ran = 0usize;
    let mut skipped = 0usize;
    for _ in 0..4000 {
        // F: bitstream contents
        let mut buf = vec![0u8; BUF_LEN];
        match rng.below(5) {
            0 => {}                                    // all zero
            1 => buf.iter_mut().for_each(|b| *b = 0xFF), // all ones
            2 => {
                for (i, b) in buf.iter_mut().enumerate() {
                    *b = if i & 1 == 0 { 0xAA } else { 0x55 };
                }
            }
            _ => rng.fill(&mut buf),
        }

        // A: total_bands
        let tb = match rng.below(6) {
            0 => 0u8,
            1 => 1,
            2 => rng.range(2, 16) as u8,
            3 => 32,
            4 => rng.range(33, 64) as u8,
            _ => rng.range(1, 64) as u8,
        };

        // B: bitalloc / scfcod (the whole 128-byte alloc region)
        let mut sci = Sci::zeroed();
        sci.total_bands = tb;
        let mut huge = false;
        for i in 0..128usize {
            let v = match rng.below(6) {
                0 => 0u8,
                1 => rng.range(1, 16) as u8,
                2 => rng.range(17, 24) as u8,
                3 => rng.range(25, 255) as u8,
                4 => [16u8, 17, 47, 48, 49, 255][rng.below(6) as usize],
                _ => rng.next_u32() as u8,
            };
            if i < 64 {
                sci.bitalloc[i] = v;
            } else {
                sci.scfcod[i - 64] = v;
            }
            if i < (2 * tb as usize).min(128) {
                let ba = v as i32;
                if ba >= 17 && grouped_params(ba).1 > 32_768 {
                    huge = true;
                }
            }
        }
        fuzz_unused(&mut sci, &mut rng);

        // C: group_size
        let gs = match rng.below(6) {
            0 => 0i32,
            1 => 1,
            2 => rng.range(2, 8) as i32,
            3 => 18,
            4 => rng.range(1, 64) as i32,
            _ => -(rng.range(1, 32) as i32),
        };

        // D: starting bit phase / position
        let pos0 = match rng.below(4) {
            0 => 0i32,
            1 => rng.below(8) as i32,
            2 => rng.range(0, 4000) as i32,
            _ => (rng.range(0, 500) as i32) * 8 + rng.below(8) as i32,
        };

        // E: limit — forced to NO_READ_LIMIT whenever a field width is absurd.
        let limit = if huge || !all_ba_readable(&sci) {
            NO_READ_LIMIT
        } else {
            match rng.below(6) {
                0 => ample_limit(buf.len()),
                1 => 0,
                2 => -(rng.range(1, 1000) as i32),
                3 => rng.range(1, 5000) as i32,
                4 => rng.range(1, 200_000) as i32,
                _ => NO_READ_LIMIT,
            }
        };

        if diff_case("C23", &buf, &sci, gs, pos0, limit) {
            ran += 1;
        } else {
            skipped += 1;
        }
    }
    eprintln!("C23: ran={ran} skipped={skipped}");
    assert!(ran >= 1500, "C23 ran only {ran} comparable cases");
}

// ---------------------------------------------------------------------------
// C24 — non-zero / large starting bit position
// ---------------------------------------------------------------------------
#[test]
fn c24_nonzero_start_pos() {
    let mut rng = Rng::new(SEED ^ 27);
    for _ in 0..256 {
        let buf = random_buf(&mut rng);
        let tb = rng.range(1, 16) as u8;
        let ba: Vec<u8> = (0..2 * tb as usize)
            .map(|_| match rng.below(3) {
                0 => 0,
                1 => rng.range(1, 16) as u8,
                _ => rng.range(17, 22) as u8,
            })
            .collect();
        let mut sci = sci_with(tb, &ba);
        fuzz_unused(&mut sci, &mut rng);
        // Start deep inside the buffer, at every possible phase.
        let pos = (rng.range(1000, 200_000)) as i32;
        diff_case_must_run("C24", &buf, &sci, 18, pos, ample_limit(buf.len()));
    }
}
