//! Phase B — valid-path differential tests, one `#[test]` per `CONFIGS.md` row.
//!
//! Every row is driven with many randomized inputs from a fixed-seed PRNG and
//! compares the C `.so` against the Rust `.so` through their exported
//! `dequantize_granule` symbol.

mod common;

use common::*;

/// `total_bands = 1` with `bitalloc[1] = 0`, `group_size = 1`: the whole call
/// reduces to four sequential `get_bits(bs, n(ba))` reads (one per `j`), which
/// is how the lowest-level routine is exercised in isolation.
fn one_band(rng: &mut Rng, ba: u8) -> Case {
    let mut c = Case::new(rng);
    c.set_total_bands(1);
    c.set_ba(0, ba);
    c.set_ba(1, 0);
    c.group_size = 1;
    c.pos = 0;
    c.limit = MAX_LIMIT;
    c
}

/// A `bitalloc` value that is guaranteed to perform a real read with any
/// buffer-safe `limit` (i.e. `n(ba) <= MAX_LIMIT`), or 0 (skipped band).
/// Note this set still spans the whole 1..=255 byte range, because for
/// `ba >= 17` the shift count is masked to 5 bits: `(ba-17)%32 <= 14` or
/// `== 31` are all narrow.
fn narrow_or_zero(rng: &mut Rng, narrow: &[u8]) -> u8 {
    if rng.below(5) == 0 { 0 } else { rng.pick(narrow) }
}

// ===========================================================================
// G* — the lowest-level entry point (`get_bits`) in isolation
// ===========================================================================

#[test]
fn g1_get_bits_single_bit() {
    let mut rng = Rng::new(1);
    for it in 0..256 {
        let mut c = one_band(&mut rng, 1);
        check(&mut c, &format!("G1 iter={it}"));
    }
}

#[test]
fn g2_get_bits_every_half_branch_width_aligned() {
    let mut rng = Rng::new(2);
    for ba in 1u8..=16 {
        for it in 0..48 {
            let mut c = one_band(&mut rng, ba);
            check(&mut c, &format!("G2 ba={ba} iter={it}"));
        }
    }
}

#[test]
fn g3_get_bits_every_width_times_every_unaligned_start() {
    let mut rng = Rng::new(3);
    for ba in 1u8..=16 {
        for s in 1..8 {
            for it in 0..12 {
                let mut c = one_band(&mut rng, ba);
                // arbitrary byte offset plus the sub-byte phase `s`
                c.pos = 8 * rng.range_i32(0, 400) + s;
                check(&mut c, &format!("G3 ba={ba} s={s} iter={it}"));
            }
        }
    }
}

#[test]
fn g4_get_bits_shl_lands_exactly_on_zero() {
    // `shl = n + s` reaching exactly 0 after the loop means the final
    // `next >> -shl` is a shift by 0 (must not be treated as a shift by 32).
    let mut rng = Rng::new(4);
    let mut combos = Vec::new();
    for ba in 1u8..=16 {
        for s in 0..8i32 {
            if (ba as i32 + s) % 8 == 0 {
                combos.push((ba, s));
            }
        }
    }
    assert!(!combos.is_empty());
    for (ba, s) in combos {
        for it in 0..24 {
            let mut c = one_band(&mut rng, ba);
            c.pos = 8 * rng.range_i32(0, 400) + s;
            check(&mut c, &format!("G4 ba={ba} s={s} iter={it}"));
        }
    }
}

#[test]
fn g5_get_bits_mod_branch_widths() {
    let mut rng = Rng::new(5);
    for ba in 17u8..=46 {
        for s in 0..8i32 {
            for it in 0..4 {
                let mut c = one_band(&mut rng, ba);
                c.pos = 8 * rng.range_i32(0, 64) + s;
                check(&mut c, &format!("G5 ba={ba} s={s} iter={it}"));
            }
        }
    }
}

#[test]
fn g6_get_bits_ba47_signed_shift_overflow() {
    // ba == 47 => `2 << 30` overflows `int` to 0x80000000, mod = 0x80000001,
    // n = 0x70000003.  The underrun guard fires, so `code == 0` and every
    // sample becomes -(mod/2) = -1073741824; `bs->pos` still advances by n.
    let mut rng = Rng::new(6);
    for it in 0..128 {
        let mut c = one_band(&mut rng, 47);
        c.group_size = rng.pick(&[1, 2, 3, 4, 12, 18, 32]);
        check(&mut c, &format!("G6 iter={it}"));
    }
}

#[test]
fn g7_get_bits_ba48_mod_is_one() {
    // ba == 48 => `2 << 31` == 0 => mod == 1 => every sample is exactly 0.0,
    // and n == 3.
    let mut rng = Rng::new(7);
    for s in 0..8i32 {
        for it in 0..32 {
            let mut c = one_band(&mut rng, 48);
            c.pos = 8 * rng.range_i32(0, 400) + s;
            c.group_size = rng.pick(&[1, 2, 4, 12, 32]);
            check(&mut c, &format!("G7 s={s} iter={it}"));
        }
    }
}

#[test]
fn g8_get_bits_shift_count_aliasing_above_48() {
    let mut rng = Rng::new(8);
    for ba in 49u8..=255 {
        for it in 0..4 {
            let mut c = one_band(&mut rng, ba);
            c.pos = 8 * rng.range_i32(0, 64) + rng.range_i32(0, 7);
            check(&mut c, &format!("G8 ba={ba} iter={it}"));
        }
    }
}

#[test]
fn g9_limit_exactly_equals_pos_plus_n_last_legal_read() {
    // the guard is `>` not `>=`: `pos + n == limit` must still read.
    let mut rng = Rng::new(9);
    let narrow = narrow_bas();
    for &ba in &narrow {
        for it in 0..8 {
            let mut c = one_band(&mut rng, ba);
            let n = n_for_ba(ba as i32);
            let maxpos = (MAX_LIMIT - n).max(0);
            c.pos = if maxpos > 0 { rng.range_i32(0, maxpos) } else { 0 };
            c.limit = c.pos + n;
            assert!(c.limit <= MAX_LIMIT);
            check(&mut c, &format!("G9 ba={ba} n={n} iter={it}"));
        }
    }
}

#[test]
fn g10_limit_one_below_pos_plus_n_first_illegal_read() {
    let mut rng = Rng::new(10);
    for &ba in &narrow_bas() {
        for it in 0..8 {
            let mut c = one_band(&mut rng, ba);
            let n = n_for_ba(ba as i32);
            let maxpos = (MAX_LIMIT - n).max(0);
            c.pos = if maxpos > 0 { rng.range_i32(0, maxpos) } else { 0 };
            c.limit = c.pos + n - 1;
            check(&mut c, &format!("G10 ba={ba} n={n} iter={it}"));
        }
    }
}

#[test]
fn g11_all_zero_bitstream() {
    let mut rng = Rng::new(11);
    for ba in 1u8..=16 {
        for s in 0..8i32 {
            let mut c = one_band(&mut rng, ba);
            c.bits = vec![0u8; BUF_LEN];
            c.pos = 8 * rng.range_i32(0, 400) + s;
            check(&mut c, &format!("G11 ba={ba} s={s}"));
        }
    }
}

#[test]
fn g12_all_ones_bitstream() {
    let mut rng = Rng::new(12);
    for ba in 1u8..=16 {
        for s in 0..8i32 {
            let mut c = one_band(&mut rng, ba);
            c.bits = vec![0xFFu8; BUF_LEN];
            c.pos = 8 * rng.range_i32(0, 400) + s;
            check(&mut c, &format!("G12 ba={ba} s={s}"));
        }
    }
}

#[test]
fn g13_mod_branch_with_all_ones_bitstream() {
    let mut rng = Rng::new(13);
    for ba in 17u8..=31 {
        for s in 0..8i32 {
            let mut c = one_band(&mut rng, ba);
            c.bits = vec![0xFFu8; BUF_LEN];
            c.pos = 8 * rng.range_i32(0, 32) + s;
            c.group_size = rng.pick(&[1, 4, 12, 32]);
            check(&mut c, &format!("G13 ba={ba} s={s}"));
        }
    }
}

#[test]
fn g14_widest_legal_read_many_loop_steps() {
    // ba == 31 => n == 28675 => the `while ((shl -= 8) > 0)` loop runs ~3585
    // times, so only the low 32 bits of the accumulated `cache` survive.
    let mut rng = Rng::new(14);
    assert_eq!(n_for_ba(31), 28675);
    for s in 0..8i32 {
        for it in 0..16 {
            let mut c = one_band(&mut rng, 31);
            c.pos = 8 * rng.range_i32(0, 8) + s;
            c.group_size = rng.pick(&[1, 4, 12, 32]);
            check(&mut c, &format!("G14 s={s} iter={it}"));
        }
    }
}

#[test]
fn g15_byte_aligned_nonzero_start_positions() {
    let mut rng = Rng::new(15);
    let narrow = narrow_bas();
    for it in 0..512 {
        let ba = rng.pick(&narrow);
        let mut c = one_band(&mut rng, ba);
        c.pos = 8 * rng.range_i32(1, 500);
        check(&mut c, &format!("G15 ba={ba} iter={it}"));
    }
}

// ===========================================================================
// B* — the exported entry point driven the way a real consumer does
// ===========================================================================

#[test]
fn b1_total_bands_zero() {
    let mut rng = Rng::new(101);
    for it in 0..256 {
        let mut c = Case::new(&mut rng);
        c.set_total_bands(0);
        c.group_size = rng.range_i32(-MAX_GROUP, MAX_GROUP);
        c.pos = rng.range_i32(0, MAX_LIMIT);
        c.limit = rng.range_i32(-1000, MAX_LIMIT);
        check(&mut c, &format!("B1 iter={it}"));
    }
}

#[test]
fn b2_all_bitalloc_zero() {
    let mut rng = Rng::new(102);
    for it in 0..256 {
        let mut c = Case::new(&mut rng);
        c.set_total_bands(rng.range_i32(1, 64) as u8);
        c.set_bitalloc_all(0);
        c.group_size = 4;
        check(&mut c, &format!("B2 iter={it}"));
    }
}

#[test]
fn b3_group_size_sweep_aligned() {
    let mut rng = Rng::new(103);
    for &gs in &[1i32, 2, 3, 4, 12, 18, 32] {
        for it in 0..64 {
            let mut c = Case::new(&mut rng);
            c.set_total_bands(1);
            c.set_ba(0, rng.range_i32(1, 16) as u8);
            c.set_ba(1, rng.range_i32(1, 16) as u8);
            c.group_size = gs;
            c.pos = 0;
            check(&mut c, &format!("B3 gs={gs} iter={it}"));
        }
    }
}

#[test]
fn b4_group_size_sweep_unaligned() {
    let mut rng = Rng::new(104);
    for &gs in &[1i32, 2, 3, 4, 12, 18, 32] {
        for s in 1..8i32 {
            for it in 0..8 {
                let mut c = Case::new(&mut rng);
                c.set_total_bands(1);
                c.set_ba(0, rng.range_i32(1, 16) as u8);
                c.set_ba(1, rng.range_i32(1, 16) as u8);
                c.group_size = gs;
                c.pos = 8 * rng.range_i32(0, 100) + s;
                check(&mut c, &format!("B4 gs={gs} s={s} iter={it}"));
            }
        }
    }
}

#[test]
fn b5_multi_band_choff_walk() {
    let mut rng = Rng::new(105);
    for t in 2u8..=8 {
        for &gs in &[4i32, 12] {
            for it in 0..24 {
                let mut c = Case::new(&mut rng);
                c.set_total_bands(t);
                for i in 0..2 * t as usize {
                    // deliberately mix skipped bands in with decoded ones
                    c.set_ba(i, if rng.below(4) == 0 { 0 } else { rng.range_i32(1, 16) as u8 });
                }
                c.group_size = gs;
                c.pos = rng.range_i32(0, 64);
                check(&mut c, &format!("B5 t={t} gs={gs} iter={it}"));
            }
        }
    }
}

#[test]
fn b6_total_bands_31() {
    let mut rng = Rng::new(106);
    for it in 0..96 {
        let mut c = Case::new(&mut rng);
        c.set_total_bands(31);
        for i in 0..62 {
            c.set_ba(i, rng.range_i32(1, 16) as u8);
        }
        c.group_size = 12;
        check(&mut c, &format!("B6 iter={it}"));
    }
}

#[test]
fn b7_total_bands_32_uses_all_64_bitalloc_bytes() {
    let mut rng = Rng::new(107);
    for it in 0..96 {
        let mut c = Case::new(&mut rng);
        c.set_total_bands(32);
        for i in 0..64 {
            c.set_ba(i, rng.range_i32(1, 16) as u8);
        }
        c.group_size = 12;
        check(&mut c, &format!("B7 iter={it}"));
    }
}

#[test]
fn b8_total_bands_33_to_64_reads_spill_into_scfcod() {
    let mut rng = Rng::new(108);
    for t in 33u8..=64 {
        for it in 0..6 {
            let mut c = Case::new(&mut rng);
            c.set_total_bands(t);
            // in-bounds bitalloc bytes are chosen; bytes >= 64 come from the
            // randomized `scfcod` image, i.e. genuinely out-of-bounds reads.
            for i in 0..64 {
                c.set_ba(i, rng.range_i32(0, 16) as u8);
            }
            c.group_size = 12;
            check(&mut c, &format!("B8 t={t} iter={it}"));
        }
    }
}

#[test]
fn b9_total_bands_above_64_reads_past_the_struct() {
    let mut rng = Rng::new(109);
    for t in [65u8, 66, 80, 100, 127, 128, 200, 254, 255] {
        for it in 0..12 {
            let mut c = Case::new(&mut rng);
            c.set_total_bands(t);
            for i in 0..2 * t as usize {
                // covers the declared array, `scfcod`, the tail padding and the
                // bytes past `sizeof(L12_scale_info)` entirely
                c.set_ba(i, rng.range_i32(0, 16) as u8);
            }
            c.group_size = 4;
            check(&mut c, &format!("B9 t={t} iter={it}"));
        }
    }
}

#[test]
fn b10_total_bands_255_full_value_range() {
    let mut rng = Rng::new(110);
    for it in 0..64 {
        let mut c = Case::new(&mut rng);
        c.set_total_bands(255);
        let narrow = narrow_bas();
        for i in 0..510 {
            c.set_ba(i, narrow_or_zero(&mut rng, &narrow));
        }
        c.group_size = 4;
        c.limit = rng.range_i32(0, MAX_LIMIT);
        check(&mut c, &format!("B10 iter={it}"));
    }
}

#[test]
fn b11_mixed_half_and_mod_branches_in_one_call() {
    let mut rng = Rng::new(111);
    let narrow = narrow_bas();
    for it in 0..400 {
        let mut c = Case::new(&mut rng);
        let t = rng.range_i32(2, 64) as u8;
        c.set_total_bands(t);
        for i in 0..2 * t as usize {
            c.set_ba(i, narrow_or_zero(&mut rng, &narrow));
        }
        c.group_size = if it % 2 == 0 { 4 } else { 12 };
        c.pos = rng.range_i32(0, 512);
        check(&mut c, &format!("B11 iter={it}"));
    }
}

#[test]
fn b12_mod_branch_only_division_chains() {
    let mut rng = Rng::new(112);
    let bas = [17u8, 18, 19, 20, 48];
    for it in 0..200 {
        let mut c = Case::new(&mut rng);
        let t = rng.range_i32(2, 64) as u8;
        c.set_total_bands(t);
        for i in 0..2 * t as usize {
            c.set_ba(i, rng.pick(&bas));
        }
        c.group_size = 12;
        check(&mut c, &format!("B12 iter={it}"));
    }
}

#[test]
fn b13_limit_mid_stream_partial_underrun() {
    let mut rng = Rng::new(113);
    for it in 0..400 {
        let mut c = Case::new(&mut rng);
        c.set_total_bands(8);
        for i in 0..16 {
            c.set_ba(i, rng.range_i32(1, 16) as u8);
        }
        c.group_size = 4;
        c.pos = 0;
        // 16 bands * 4 granules * 4 samples * <=16 bits = <=4096 bits total
        let lim = rng.range_i32(0, 4096);
        c.limit = lim;
        check(&mut c, &format!("B13 iter={it} limit={lim}"));
    }
}

#[test]
fn b14_limit_zero() {
    let mut rng = Rng::new(114);
    for it in 0..128 {
        let mut c = Case::new(&mut rng);
        c.set_total_bands(8);
        for i in 0..16 {
            c.set_ba(i, rng.range_i32(1, 255) as u8);
        }
        c.group_size = 4;
        c.pos = 0;
        c.limit = 0;
        check(&mut c, &format!("B14 iter={it}"));
    }
}

#[test]
fn b15_negative_limit() {
    let mut rng = Rng::new(115);
    for it in 0..128 {
        let mut c = Case::new(&mut rng);
        c.set_total_bands(8);
        for i in 0..16 {
            c.set_ba(i, rng.range_i32(1, 255) as u8);
        }
        c.group_size = 4;
        c.pos = rng.range_i32(0, 64);
        c.limit = -rng.range_i32(1, 1_000_000);
        check(&mut c, &format!("B15 iter={it}"));
    }
}

#[test]
fn b16_pos_already_past_limit() {
    let mut rng = Rng::new(116);
    for it in 0..128 {
        let mut c = Case::new(&mut rng);
        c.set_total_bands(8);
        for i in 0..16 {
            c.set_ba(i, rng.range_i32(1, 255) as u8);
        }
        c.group_size = 4;
        c.limit = rng.range_i32(0, 2048);
        c.pos = c.limit + rng.range_i32(1, 4096);
        check(&mut c, &format!("B16 iter={it}"));
    }
}

#[test]
fn b17_group_size_zero() {
    let mut rng = Rng::new(117);
    for it in 0..256 {
        let mut c = Case::new(&mut rng);
        let t = rng.range_i32(0, 64) as u8;
        c.set_total_bands(t);
        for i in 0..2 * t as usize {
            c.set_ba(i, rng.range_i32(0, 255) as u8);
        }
        c.group_size = 0;
        check(&mut c, &format!("B17 iter={it}"));
    }
}

#[test]
fn b18_negative_group_size() {
    let mut rng = Rng::new(118);
    for it in 0..256 {
        let mut c = Case::new(&mut rng);
        let t = rng.range_i32(0, 64) as u8;
        c.set_total_bands(t);
        for i in 0..2 * t as usize {
            c.set_ba(i, rng.range_i32(0, 255) as u8);
        }
        c.group_size = -rng.range_i32(1, MAX_GROUP);
        check(&mut c, &format!("B18 iter={it}"));
    }
}

#[test]
fn b19_group_size_one() {
    let mut rng = Rng::new(119);
    for it in 0..256 {
        let mut c = Case::new(&mut rng);
        let t = rng.range_i32(1, 64) as u8;
        c.set_total_bands(t);
        let narrow = narrow_bas();
        for i in 0..2 * t as usize {
            c.set_ba(i, narrow_or_zero(&mut rng, &narrow));
        }
        c.group_size = 1;
        check(&mut c, &format!("B19 iter={it}"));
    }
}

#[test]
fn b20_widest_band_set_times_largest_stride() {
    let mut rng = Rng::new(120);
    for it in 0..48 {
        let mut c = Case::new(&mut rng);
        c.set_total_bands(64);
        let narrow = narrow_bas();
        for i in 0..128 {
            c.set_ba(i, rng.pick(&narrow));
        }
        c.group_size = 32;
        check(&mut c, &format!("B20 iter={it}"));
    }
}

#[test]
fn b21_zero_bitstream_full_range() {
    let mut rng = Rng::new(121);
    for it in 0..200 {
        let mut c = Case::new(&mut rng);
        c.bits = vec![0u8; BUF_LEN];
        let t = rng.range_i32(1, 64) as u8;
        c.set_total_bands(t);
        let narrow = narrow_bas();
        for i in 0..2 * t as usize {
            c.set_ba(i, narrow_or_zero(&mut rng, &narrow));
        }
        c.group_size = 12;
        check(&mut c, &format!("B21 iter={it}"));
    }
}

#[test]
fn b22_ones_bitstream_full_range() {
    let mut rng = Rng::new(122);
    for it in 0..200 {
        let mut c = Case::new(&mut rng);
        c.bits = vec![0xFFu8; BUF_LEN];
        let t = rng.range_i32(1, 64) as u8;
        c.set_total_bands(t);
        let narrow = narrow_bas();
        for i in 0..2 * t as usize {
            c.set_ba(i, narrow_or_zero(&mut rng, &narrow));
        }
        c.group_size = 12;
        check(&mut c, &format!("B22 iter={it}"));
    }
}

#[test]
fn b23_full_random_fuzz_over_all_axes() {
    let mut rng = Rng::new(0xC0FF_EE23);
    for it in 0..4000 {
        let mut c = Case::new(&mut rng);
        let t = match rng.below(5) {
            0 => 0,
            1 => rng.range_i32(1, 8) as u8,
            2 => rng.range_i32(9, 32) as u8,
            3 => rng.range_i32(33, 64) as u8,
            _ => rng.range_i32(65, 255) as u8,
        };
        c.set_total_bands(t);
        let narrow = narrow_bas();
        let force_narrow = rng.below(2) == 0;
        for i in 0..2 * t as usize {
            let v = match rng.below(4) {
                0 => 0,
                1 => rng.range_i32(1, 16) as u8,
                2 => rng.range_i32(17, 48) as u8,
                _ => rng.range_i32(1, 255) as u8,
            };
            c.set_ba(i, if force_narrow { narrow_or_zero(&mut rng, &narrow) } else { v });
        }
        c.group_size = match rng.below(6) {
            0 => 0,
            1 => -rng.range_i32(1, MAX_GROUP),
            2 => 1,
            3 => rng.range_i32(2, 4),
            4 => 12,
            _ => rng.range_i32(1, MAX_GROUP),
        };
        c.pos = match rng.below(4) {
            0 => 0,
            1 => rng.range_i32(0, 7),
            2 => rng.range_i32(0, MAX_LIMIT),
            _ => rng.range_i32(-4096, 4096),
        };
        c.limit = match rng.below(5) {
            0 => MAX_LIMIT,
            1 => 0,
            2 => -rng.range_i32(1, 1 << 20),
            3 => rng.range_i32(0, 4096),
            _ => rng.range_i32(-64, MAX_LIMIT),
        };
        match rng.below(4) {
            0 => c.bits = vec![0u8; BUF_LEN],
            1 => c.bits = vec![0xFFu8; BUF_LEN],
            _ => {}
        }
        check(&mut c, &format!("B23 iter={it}"));
    }
}

#[test]
fn b24_untouched_slots_and_touched_offset_pattern() {
    let mut rng = Rng::new(124);
    for it in 0..64 {
        let mut c = Case::new(&mut rng);
        let t = rng.range_i32(1, 64) as u8;
        c.set_total_bands(t);
        for i in 0..2 * t as usize {
            c.set_ba(i, rng.range_i32(1, 16) as u8);
        }
        c.group_size = rng.pick(&[1i32, 4, 12, 32]);
        c.sanitize();
        assert_same(&c, &format!("B24 iter={it}"));

        // Compare the *set of written offsets* explicitly: this is the
        // `dst += choff; choff = 18 - choff;` walk, including the fact that
        // `choff` is carried across the `j` loop.
        let co = run(c_lib(), &c);
        let ro = run(rust_lib(), &c);
        let ct = touched(&co, c.grbuf_seed);
        let rt = touched(&ro, c.grbuf_seed);
        assert_eq!(ct, rt, "written-offset pattern differs (B24 iter={it})");
        assert!(!ct.is_empty(), "expected some writes (B24 iter={it})");

        // and the walk really is the alternating +576 / -558 one
        let gs = c.group_size as usize;
        let mut expected: Vec<usize> = Vec::new();
        for j in 0..4usize {
            let mut off: i64 = (c.group_size as i64) * (j as i64);
            let mut choff: i64 = 576;
            for _ in 0..2 * t as usize {
                for k in 0..gs {
                    expected.push((off + k as i64) as usize);
                }
                off += choff;
                choff = 18 - choff;
            }
        }
        expected.sort_unstable();
        expected.dedup();
        // every written slot must be one the walk predicts (values may coincide
        // with the pre-fill pattern, so `expected` is a superset)
        for i in &ct {
            assert!(
                expected.binary_search(i).is_ok(),
                "unexpected write at grbuf[{i}] (B24 iter={it})"
            );
        }
    }
}

#[test]
fn b25_chained_calls_share_one_bit_reader() {
    let mut rng = Rng::new(125);
    for it in 0..200 {
        let mut c = Case::new(&mut rng);
        let t = rng.range_i32(1, 32) as u8;
        c.set_total_bands(t);
        let narrow = narrow_bas();
        for i in 0..2 * t as usize {
            c.set_ba(i, narrow_or_zero(&mut rng, &narrow));
        }
        c.group_size = rng.pick(&[1i32, 4, 12]);
        c.pos = 0;
        c.sanitize_n(5);
        assert_same_seq(&c, 5, &format!("B25 iter={it}"));
    }
}

#[test]
fn b26_large_group_size_full_granule_strides() {
    // `group_size == 576` is the real MPEG granule width; 18/64/128 sit between
    // the small SCFSI group sizes and it.  These strides make `grbuf` writes
    // from different `j` iterations and different bands overlap differently.
    let mut rng = Rng::new(126);
    let narrow = narrow_bas();
    for &gs in &[18i32, 64, 128, 576] {
        for &t in &[1u8, 2, 8, 32, 64, 255] {
            for it in 0..3 {
                let mut c = Case::new(&mut rng);
                c.set_total_bands(t);
                for i in 0..2 * t as usize {
                    c.set_ba(i, narrow_or_zero(&mut rng, &narrow));
                }
                c.group_size = gs;
                c.pos = rng.range_i32(0, 63);
                check(&mut c, &format!("B26 gs={gs} t={t} iter={it}"));
                assert_eq!(c.group_size, gs, "sanitize must not clamp gs={gs}");
            }
        }
    }
}
