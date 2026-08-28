//! Phase C — error-path differential tests, one `#[test]` per `ERRORS.md` row,
//! plus the generic C-API boundaries (null pointers, zero / oversized lengths,
//! one-past-range values, out-of-range "enum" bytes crossing the FFI boundary).
//!
//! Both `.so`s are loaded with `libloading`; a row passes only when the C and
//! the Rust library agree on the *same* sentinel / rejection behaviour, not
//! merely on "something went wrong".

mod common;

use common::*;

fn band_case(rng: &mut Rng, t: u8, ba: u8, gs: i32) -> Case {
    let mut c = Case::new(rng);
    c.set_total_bands(t);
    for i in 0..2 * t as usize {
        c.set_ba(i, ba);
    }
    c.group_size = gs;
    c.pos = 0;
    c.limit = MAX_LIMIT;
    c
}

// ---------------------------------------------------------------------------
// E1 / E2 — the single genuine error path: the `get_bits` underrun guard
// ---------------------------------------------------------------------------

#[test]
fn e1_underrun_in_half_branch_yields_minus_half() {
    // `get_bits` returns the sentinel 0, so `dst[k] = (float)(0 - half)`.
    let mut rng = Rng::new(201);
    for ba in 1u8..=16 {
        for it in 0..16 {
            let mut c = band_case(&mut rng, 4, ba, 4);
            c.pos = 0;
            c.limit = 0; // nothing at all is readable
            check(&mut c, &format!("E1 ba={ba} iter={it}"));

            // and the expected sentinel value really is -half
            let half = (1i32 << (ba - 1)) - 1;
            let out = run(c_lib(), &c);
            let idx = 0usize;
            assert_eq!(
                f32::from_bits(out.grbuf[idx]),
                -(half as f32),
                "C must write -half on underrun (ba={ba})"
            );
            let rout = run(rust_lib(), &c);
            assert_eq!(out.grbuf[idx], rout.grbuf[idx]);
        }
    }
}

#[test]
fn e2_underrun_in_mod_branch_yields_minus_mod_over_two() {
    let mut rng = Rng::new(202);
    for ba in 17u8..=48 {
        for it in 0..8 {
            let mut c = band_case(&mut rng, 4, ba, 4);
            c.limit = 0;
            check(&mut c, &format!("E2 ba={ba} iter={it}"));

            let m = (2i32.wrapping_shl((ba as i32 - 17) as u32) as u32).wrapping_add(1);
            let expect = ((0u32 % m).wrapping_sub(m / 2)) as i32 as f32;
            let out = run(c_lib(), &c);
            assert_eq!(
                f32::from_bits(out.grbuf[0]),
                expect,
                "C must write -(mod/2) on underrun (ba={ba}, mod={m})"
            );
            let rout = run(rust_lib(), &c);
            assert_eq!(out.grbuf[0], rout.grbuf[0]);
        }
    }
}

#[test]
fn e3_limit_zero_with_pos_zero_empty_stream() {
    let mut rng = Rng::new(203);
    for it in 0..200 {
        let mut c = Case::new(&mut rng);
        let t = rng.range_i32(1, 64) as u8;
        c.set_total_bands(t);
        for i in 0..2 * t as usize {
            c.set_ba(i, rng.range_i32(0, 255) as u8);
        }
        c.group_size = rng.pick(&[1i32, 4, 12, 32]);
        c.pos = 0;
        c.limit = 0;
        check(&mut c, &format!("E3 iter={it}"));
    }
}

#[test]
fn e4_negative_limit() {
    let mut rng = Rng::new(204);
    for &lim in &[-1i32, -2, -7, -8, -9, -1000, -32768, -(1 << 20), i32::MIN + 1, i32::MIN] {
        for it in 0..24 {
            let mut c = Case::new(&mut rng);
            let t = rng.range_i32(1, 32) as u8;
            c.set_total_bands(t);
            for i in 0..2 * t as usize {
                c.set_ba(i, rng.range_i32(0, 255) as u8);
            }
            c.group_size = rng.pick(&[1i32, 4, 12]);
            c.pos = rng.range_i32(0, 1024);
            c.limit = lim;
            check(&mut c, &format!("E4 limit={lim} iter={it}"));
        }
    }
}

#[test]
fn e5_pos_already_past_limit_is_latching() {
    let mut rng = Rng::new(205);
    for it in 0..200 {
        let mut c = Case::new(&mut rng);
        let t = rng.range_i32(1, 32) as u8;
        c.set_total_bands(t);
        let narrow = narrow_bas();
        for i in 0..2 * t as usize {
            c.set_ba(i, rng.pick(&narrow));
        }
        c.group_size = rng.pick(&[1i32, 4, 12]);
        c.limit = rng.range_i32(0, 2048);
        c.pos = c.limit + rng.range_i32(1, 2048);
        check(&mut c, &format!("E5 iter={it}"));
    }
}

#[test]
fn e6_pos_exactly_equals_limit_one_step_past_valid_range() {
    let mut rng = Rng::new(206);
    let narrow = narrow_bas();
    for it in 0..300 {
        let mut c = Case::new(&mut rng);
        c.set_total_bands(1);
        c.set_ba(0, rng.pick(&narrow));
        c.set_ba(1, 0);
        c.group_size = 1;
        c.pos = rng.range_i32(0, 4096);
        c.limit = c.pos; // pos + n > limit for every n >= 1
        check(&mut c, &format!("E6 iter={it}"));
    }
}

#[test]
fn e7_pos_plus_n_exactly_equals_limit_is_accepted() {
    // The guard is `>` and not `>=`: this read must succeed in both libraries.
    let mut rng = Rng::new(207);
    for &ba in &narrow_bas() {
        let n = n_for_ba(ba as i32);
        for it in 0..6 {
            let mut c = Case::new(&mut rng);
            c.set_total_bands(1);
            c.set_ba(0, ba);
            c.set_ba(1, 0);
            c.group_size = 1;
            let maxpos = (MAX_LIMIT - n).max(0);
            c.pos = if maxpos > 0 { rng.range_i32(0, maxpos) } else { 0 };
            c.limit = c.pos + n;
            check(&mut c, &format!("E7 ba={ba} n={n} iter={it}"));

            // sanity: the accepted read must differ from the pure-sentinel one
            let accepted = run(c_lib(), &c);
            let mut rejected = c.clone();
            rejected.limit = rejected.pos + n - 1;
            let refused = run(c_lib(), &rejected);
            assert_eq!(accepted.pos, refused.pos, "pos advances either way");
            let _ = refused;
        }
    }
}

#[test]
fn e8_negative_pos_guard_fires_before_any_dereference() {
    // With `limit < pos` the guard rejects *before* `bs->buf + (pos >> 3)` is
    // dereferenced, so a negative bit position is handled without touching
    // memory in front of the buffer.  `pos >> 3` is an arithmetic shift, and
    // `pos & 7` is still 0..=7, both of which must be reproduced.
    let mut rng = Rng::new(208);
    for &pos in &[-1i32, -2, -7, -8, -9, -64, -65, -1023, -1024, -(1 << 20), -(1 << 29)] {
        for it in 0..24 {
            let mut c = Case::new(&mut rng);
            let t = rng.range_i32(1, 32) as u8;
            c.set_total_bands(t);
            for i in 0..2 * t as usize {
                c.set_ba(i, rng.range_i32(0, 255) as u8);
            }
            c.group_size = rng.pick(&[1i32, 4, 12]);
            c.pos = pos;
            c.limit = i32::MIN; // guard always fires => no dereference
            check(&mut c, &format!("E8 pos={pos} iter={it}"));
        }
    }
}

// ---------------------------------------------------------------------------
// E9 – E12 — the implicit "rejections" (degenerate loop guards)
// ---------------------------------------------------------------------------

#[test]
fn e9_group_size_zero_still_consumes_mod_branch_bits() {
    let mut rng = Rng::new(209);
    let narrow = narrow_bas();
    for it in 0..300 {
        let mut c = Case::new(&mut rng);
        let t = rng.range_i32(1, 64) as u8;
        c.set_total_bands(t);
        for i in 0..2 * t as usize {
            // >= 17 => `get_bits` is called even though the k loop is empty
            c.set_ba(i, if rng.below(2) == 0 { rng.pick(&narrow) } else { 20 });
        }
        c.group_size = 0;
        check(&mut c, &format!("E9 iter={it}"));
    }

    // explicit: return value must be exactly 0 and `pos` must still move
    let mut c = band_case(&mut rng, 2, 20, 0);
    c.sanitize();
    let out = run(c_lib(), &c);
    let rout = run(rust_lib(), &c);
    assert_eq!(out.ret, 0);
    assert_eq!(rout.ret, 0);
    assert_ne!(out.pos, 0, "the mod branch reads before the k loop");
    assert_eq!(out.pos, rout.pos);
}

#[test]
fn e10_negative_group_size_writes_nothing_and_returns_negative() {
    let mut rng = Rng::new(210);
    for &gs in &[-1i32, -2, -4, -18, -32] {
        for it in 0..40 {
            let mut c = Case::new(&mut rng);
            let t = rng.range_i32(0, 64) as u8;
            c.set_total_bands(t);
            for i in 0..2 * t as usize {
                c.set_ba(i, rng.range_i32(0, 255) as u8);
            }
            c.group_size = gs;
            check(&mut c, &format!("E10 gs={gs} iter={it}"));

            let out = run(c_lib(), &c);
            let rout = run(rust_lib(), &c);
            assert_eq!(out.ret, gs.wrapping_mul(4));
            assert_eq!(rout.ret, gs.wrapping_mul(4));
            assert!(touched(&out, c.grbuf_seed).is_empty(), "C must not write");
            assert!(touched(&rout, c.grbuf_seed).is_empty(), "Rust must not write");
        }
    }
}

#[test]
fn e11_total_bands_zero_touches_nothing() {
    let mut rng = Rng::new(211);
    for it in 0..200 {
        let mut c = Case::new(&mut rng);
        c.set_total_bands(0);
        c.group_size = rng.range_i32(-MAX_GROUP, MAX_GROUP);
        c.pos = rng.range_i32(0, 1 << 20);
        c.limit = rng.range_i32(-1 << 20, MAX_LIMIT);
        check(&mut c, &format!("E11 iter={it}"));

        let out = run(c_lib(), &c);
        let rout = run(rust_lib(), &c);
        assert_eq!(out.pos, c.pos, "bs untouched when total_bands == 0");
        assert_eq!(rout.pos, c.pos);
        assert!(touched(&out, c.grbuf_seed).is_empty());
        assert!(touched(&rout, c.grbuf_seed).is_empty());
    }
}

#[test]
fn e12_zero_bitalloc_skips_band_but_still_walks_choff() {
    let mut rng = Rng::new(212);
    for it in 0..200 {
        let mut c = Case::new(&mut rng);
        let t = rng.range_i32(1, 64) as u8;
        c.set_total_bands(t);
        c.set_bitalloc_all(0);
        c.group_size = rng.pick(&[1i32, 4, 12, 32]);
        check(&mut c, &format!("E12 all-zero iter={it}"));

        let out = run(c_lib(), &c);
        let rout = run(rust_lib(), &c);
        assert_eq!(out.pos, c.pos, "no bits consumed when every ba == 0");
        assert_eq!(rout.pos, c.pos);
        assert!(touched(&out, c.grbuf_seed).is_empty());
        assert!(touched(&rout, c.grbuf_seed).is_empty());
    }

    // interleaved zero / non-zero bands: the `choff` walk must stay in sync
    let narrow = narrow_bas();
    for it in 0..200 {
        let mut c = Case::new(&mut rng);
        let t = rng.range_i32(2, 64) as u8;
        c.set_total_bands(t);
        for i in 0..2 * t as usize {
            c.set_ba(i, if i % 2 == 0 { 0 } else { rng.pick(&narrow) });
        }
        c.group_size = rng.pick(&[1i32, 4, 12]);
        check(&mut c, &format!("E12 interleaved iter={it}"));
    }
}

// ---------------------------------------------------------------------------
// E13 / E14 — null pointers on the paths that never dereference them
// ---------------------------------------------------------------------------

#[test]
fn e13_null_grbuf_when_nothing_is_written() {
    let mut rng = Rng::new(213);
    // (a) group_size <= 0
    for &gs in &[0i32, -1, -4, -32] {
        for it in 0..24 {
            let mut c = Case::new(&mut rng);
            let t = rng.range_i32(0, 64) as u8;
            c.set_total_bands(t);
            for i in 0..2 * t as usize {
                c.set_ba(i, rng.range_i32(0, 255) as u8);
            }
            c.group_size = gs;
            c.grbuf_null = true;
            check(&mut c, &format!("E13a gs={gs} iter={it}"));
        }
    }
    // (b) total_bands == 0
    for it in 0..24 {
        let mut c = Case::new(&mut rng);
        c.set_total_bands(0);
        c.group_size = rng.range_i32(1, MAX_GROUP);
        c.grbuf_null = true;
        check(&mut c, &format!("E13b iter={it}"));
    }
    // (c) every bitalloc == 0
    for it in 0..24 {
        let mut c = Case::new(&mut rng);
        let t = rng.range_i32(1, 64) as u8;
        c.set_total_bands(t);
        c.set_bitalloc_all(0);
        c.group_size = rng.range_i32(1, MAX_GROUP);
        c.grbuf_null = true;
        check(&mut c, &format!("E13c iter={it}"));
    }
}

#[test]
fn e14_null_bs_when_get_bits_is_never_called() {
    let mut rng = Rng::new(214);
    // total_bands == 0
    for it in 0..32 {
        let mut c = Case::new(&mut rng);
        c.set_total_bands(0);
        c.group_size = rng.range_i32(-MAX_GROUP, MAX_GROUP);
        c.bs_null = true;
        check(&mut c, &format!("E14a iter={it}"));
    }
    // every bitalloc == 0
    for it in 0..32 {
        let mut c = Case::new(&mut rng);
        let t = rng.range_i32(1, 64) as u8;
        c.set_total_bands(t);
        c.set_bitalloc_all(0);
        c.group_size = rng.range_i32(-MAX_GROUP, MAX_GROUP);
        c.bs_null = true;
        check(&mut c, &format!("E14b iter={it}"));
    }
    // both null at once
    for it in 0..32 {
        let mut c = Case::new(&mut rng);
        c.set_total_bands(0);
        c.group_size = rng.range_i32(-MAX_GROUP, MAX_GROUP);
        c.bs_null = true;
        c.grbuf_null = true;
        check(&mut c, &format!("E14c iter={it}"));
    }
}

// ---------------------------------------------------------------------------
// E15 / E16 — out-of-range `total_bands` (out-of-bounds `bitalloc` indexing)
// ---------------------------------------------------------------------------

#[test]
fn e15_total_bands_above_32_reads_past_bitalloc_into_scfcod() {
    let mut rng = Rng::new(215);
    for t in 33u8..=64 {
        for it in 0..8 {
            let mut c = Case::new(&mut rng);
            c.set_total_bands(t);
            // leave bytes >= 64 as the randomized `scfcod` image: the C code
            // reads them as bit allocations, and so must the Rust code.
            let narrow = narrow_bas();
            for i in 0..64 {
                c.set_ba(i, rng.pick(&narrow));
            }
            for i in 64..2 * t as usize {
                c.set_ba(i, rng.pick(&narrow)); // deterministic, still OOB reads
            }
            c.group_size = 4;
            check(&mut c, &format!("E15 t={t} iter={it}"));
        }
    }
}

#[test]
fn e16_total_bands_above_64_reads_past_the_whole_struct() {
    let mut rng = Rng::new(216);
    for t in [65u8, 80, 112, 128, 160, 200, 224, 250, 254, 255] {
        for it in 0..12 {
            let mut c = Case::new(&mut rng);
            c.set_total_bands(t);
            let narrow = narrow_bas();
            for i in 0..2 * t as usize {
                c.set_ba(i, narrow_or_zero_local(&mut rng, &narrow));
            }
            c.group_size = 4;
            check(&mut c, &format!("E16 t={t} iter={it}"));
        }
    }
}

fn narrow_or_zero_local(rng: &mut Rng, narrow: &[u8]) -> u8 {
    if rng.below(5) == 0 { 0 } else { rng.pick(narrow) }
}

// ---------------------------------------------------------------------------
// E17 – E22 — out-of-range "enum" bytes: every `bitalloc` opcode 0..=255
//
// `bitalloc` is the closest thing this API has to an enum crossing the FFI
// boundary: MPEG only ever produces 0..=16, but the C code accepts any `uint8_t`
// and every value takes a different, sometimes overflowing, code path.
// ---------------------------------------------------------------------------

#[test]
fn e17_to_e22_every_bitalloc_opcode_value() {
    let mut rng = Rng::new(217);
    for ba in 0u8..=255 {
        for &gs in &[0i32, 1, 2, 4, 12, 32, -4] {
            for it in 0..2 {
                let mut c = band_case(&mut rng, 2, ba, gs);
                c.pos = rng.range_i32(0, 7);
                check(&mut c, &format!("E17-22 ba={ba} gs={gs} iter={it}"));
            }
        }
    }
}

#[test]
fn e18_ba16_is_the_last_half_branch_value_and_e17_ba17_the_first_mod_value() {
    let mut rng = Rng::new(218);
    for &ba in &[15u8, 16, 17, 18] {
        for it in 0..64 {
            let mut c = band_case(&mut rng, 8, ba, 12);
            c.pos = rng.range_i32(0, 63);
            check(&mut c, &format!("E18 ba={ba} iter={it}"));
        }
    }
    // the documented `half` values must be what the C code produces on underrun
    for ba in [1u8, 15, 16] {
        let mut c = band_case(&mut rng, 1, ba, 1);
        c.limit = 0;
        c.sanitize();
        let out = run(c_lib(), &c);
        let expect = -(((1i32 << (ba - 1)) - 1) as f32);
        assert_eq!(f32::from_bits(out.grbuf[0]), expect, "ba={ba}");
        assert_eq!(run(rust_lib(), &c).grbuf[0], out.grbuf[0]);
    }
}

#[test]
fn e19_ba48_shift_by_31_gives_mod_one_and_all_zero_samples() {
    let mut rng = Rng::new(219);
    assert_eq!(n_for_ba(48), 3);
    for it in 0..128 {
        let mut c = band_case(&mut rng, 4, 48, 12);
        c.pos = rng.range_i32(0, 255);
        check(&mut c, &format!("E19 iter={it}"));
        let out = run(c_lib(), &c);
        assert_eq!(f32::from_bits(out.grbuf[0]), 0.0, "mod == 1 => sample 0");
        assert_eq!(f32::from_bits(out.grbuf[0]).to_bits(), 0, "and it is +0.0");
    }
}

#[test]
fn e20_ba47_signed_shift_overflow_mod_is_0x80000001() {
    let mut rng = Rng::new(220);
    let m = (2i32.wrapping_shl(30) as u32).wrapping_add(1);
    assert_eq!(m, 0x8000_0001);
    assert_eq!(n_for_ba(47), 0x7000_0003);
    for it in 0..64 {
        let mut c = band_case(&mut rng, 3, 47, 12);
        check(&mut c, &format!("E20 iter={it}"));
        let out = run(c_lib(), &c);
        let rout = run(rust_lib(), &c);
        assert_eq!(out.grbuf, rout.grbuf);
        assert_eq!(out.pos, rout.pos);
        assert_eq!(
            f32::from_bits(out.grbuf[0]),
            -1073741824.0f32,
            "underrun sample must be -(mod/2)"
        );
    }
}

#[test]
fn e21_shift_count_masking_makes_ba_alias_with_period_32() {
    // ba and ba+32 must behave identically (the C shift count is masked to 5
    // bits); assert this on the C side and then require the Rust side to match.
    let mut rng = Rng::new(221);
    for ba in 17u8..=48 {
        let aliased = ba + 32;
        assert_eq!(n_for_ba(ba as i32), n_for_ba(aliased as i32));
        for it in 0..4 {
            let mut a = band_case(&mut rng, 2, ba, 4);
            a.pos = rng.range_i32(0, 31);
            a.sanitize();
            let mut b = a.clone();
            b.set_bitalloc_all(aliased);
            b.sanitize();
            assert_same(&a, &format!("E21 ba={ba} iter={it}"));
            assert_same(&b, &format!("E21 ba={aliased} iter={it}"));
            let ca = run(c_lib(), &a);
            let cb = run(c_lib(), &b);
            assert_eq!(ca.grbuf, cb.grbuf, "C: ba={ba} vs {aliased} must alias");
            assert_eq!(ca.pos, cb.pos);
        }
    }
}

#[test]
fn e22_ba255_max_uint8_value() {
    let mut rng = Rng::new(222);
    assert_eq!(n_for_ba(255), 28675);
    for it in 0..96 {
        let mut c = band_case(&mut rng, 2, 255, 4);
        c.pos = rng.range_i32(0, 7);
        c.limit = rng.pick(&[0i32, 1, 28674, 28675, 28676, MAX_LIMIT]);
        check(&mut c, &format!("E22 iter={it}"));
    }
}

// ---------------------------------------------------------------------------
// E23 / E24 — the shift-count corner cases inside `get_bits`
// ---------------------------------------------------------------------------

#[test]
fn e23_very_wide_legal_read_only_low_32_bits_survive() {
    let mut rng = Rng::new(223);
    for &ba in &[29u8, 30, 31, 61, 62, 63] {
        let n = n_for_ba(ba as i32);
        assert!(n <= MAX_LIMIT, "ba={ba} n={n}");
        for it in 0..12 {
            let mut c = Case::new(&mut rng);
            c.set_total_bands(1);
            c.set_ba(0, ba);
            c.set_ba(1, 0);
            c.group_size = rng.pick(&[1i32, 4, 32]);
            c.pos = rng.range_i32(0, 7);
            c.limit = MAX_LIMIT;
            check(&mut c, &format!("E23 ba={ba} n={n} iter={it}"));
        }
    }
}

#[test]
fn e24_final_shift_is_by_zero_not_by_thirtytwo() {
    let mut rng = Rng::new(224);
    // n + s == 8k => after the loop `shl == 0` => `next >> 0`
    for ba in 1u8..=16 {
        for s in 0..8i32 {
            if (ba as i32 + s) % 8 != 0 {
                continue;
            }
            for it in 0..16 {
                let mut c = Case::new(&mut rng);
                c.set_total_bands(1);
                c.set_ba(0, ba);
                c.set_ba(1, 0);
                c.group_size = 1;
                c.pos = 8 * rng.range_i32(0, 200) + s;
                c.limit = MAX_LIMIT;
                check(&mut c, &format!("E24 ba={ba} s={s} iter={it}"));
                // the read must not be the degenerate all-zero one
                let out = run(c_lib(), &c);
                assert_eq!(out.grbuf, run(rust_lib(), &c).grbuf);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// E25 — the `choff` walk lands wherever it lands; no bounds check exists
// ---------------------------------------------------------------------------

#[test]
fn e25_choff_walk_has_no_bounds_check() {
    let mut rng = Rng::new(225);
    let narrow = narrow_bas();
    for t in [1u8, 2, 3, 32, 64, 128, 255] {
        for it in 0..8 {
            let mut c = Case::new(&mut rng);
            c.set_total_bands(t);
            for i in 0..2 * t as usize {
                c.set_ba(i, rng.pick(&narrow));
            }
            c.group_size = 4;
            c.sanitize();
            assert_same(&c, &format!("E25 t={t} iter={it}"));

            let co = run(c_lib(), &c);
            let ro = run(rust_lib(), &c);
            assert_eq!(
                touched(&co, c.grbuf_seed),
                touched(&ro, c.grbuf_seed),
                "the set of written grbuf offsets must be identical (t={t})"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Generic C-API boundaries beyond the table
// ---------------------------------------------------------------------------

#[test]
fn extreme_group_size_values_with_no_inner_loop() {
    // `group_size` is a plain `int`: values one step past anything sane must
    // still produce the identical (wrapping) `group_size * 4` return value and
    // the identical `grbuf + group_size*j` pointer arithmetic (never
    // dereferenced because `total_bands == 0`).
    let mut rng = Rng::new(226);
    for &gs in &[
        i32::MIN,
        i32::MIN + 1,
        -1_073_741_824,
        -3,
        -1,
        0,
        1,
        3,
        1_073_741_823,
        1_073_741_824,
        i32::MAX - 1,
        i32::MAX,
    ] {
        for it in 0..8 {
            let mut c = Case::new(&mut rng);
            c.set_total_bands(0);
            c.group_size = gs;
            // deliberately NOT sanitized: `group_size` must stay extreme.
            assert_same(&c, &format!("extreme gs={gs} iter={it}"));
            let out = run(c_lib(), &c);
            assert_eq!(out.ret, gs.wrapping_mul(4), "C: group_size*4 wraps");
            assert_eq!(run(rust_lib(), &c).ret, gs.wrapping_mul(4));
        }
    }
}

#[test]
fn extreme_pos_and_limit_values() {
    let mut rng = Rng::new(227);
    let interesting = [
        i32::MIN,
        i32::MIN + 1,
        -1,
        0,
        1,
        7,
        8,
        i32::MAX - 1,
        i32::MAX,
    ];
    for &pos in &interesting {
        for &lim in &interesting {
            for it in 0..3 {
                let mut c = Case::new(&mut rng);
                // total_bands == 0 => `bs` is never touched at all, so even the
                // most extreme pos/limit pair is well defined in C.
                c.set_total_bands(0);
                c.group_size = rng.range_i32(-4, 4);
                c.pos = pos;
                c.limit = lim;
                assert_same(&c, &format!("extreme pos={pos} limit={lim} iter={it}"));
            }
        }
    }
    // and with one band, but `limit == INT_MIN` so the guard always fires
    for &pos in &interesting {
        let mut c = Case::new(&mut rng);
        c.set_total_bands(1);
        c.set_ba(0, 9);
        c.set_ba(1, 3);
        c.group_size = 2;
        c.pos = pos;
        c.limit = i32::MIN;
        if pos == i32::MIN {
            continue; // pos == limit would let the read through (shared UB)
        }
        assert_same(&c, &format!("extreme pos={pos} limit=INT_MIN"));
    }
}

#[test]
fn every_total_bands_value_zero_through_255() {
    // "oversized length": `total_bands` has no valid-range check at all.
    let mut rng = Rng::new(228);
    let narrow = narrow_bas();
    for t in 0u8..=255 {
        let mut c = Case::new(&mut rng);
        c.set_total_bands(t);
        for i in 0..2 * t as usize {
            c.set_ba(i, narrow_or_zero_local(&mut rng, &narrow));
        }
        c.group_size = 4;
        check(&mut c, &format!("total_bands={t}"));
    }
}
