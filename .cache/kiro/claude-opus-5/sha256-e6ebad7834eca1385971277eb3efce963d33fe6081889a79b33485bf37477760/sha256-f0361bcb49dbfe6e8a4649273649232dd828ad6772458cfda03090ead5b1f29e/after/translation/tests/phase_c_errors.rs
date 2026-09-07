//! Phase C -- error-path differential tests, one test per row of `ERRORS.md`.
//!
//! `dequantize_granule` has no error return, so the "error surface" is the
//! `get_bits` exhaustion guard, the guards that skip work, and the
//! out-of-nominal-range inputs the C accepts anyway. Each test builds the exact
//! triggering condition, calls both `.so`s, and asserts the same result --
//! including asserting *which* rejection happened (guard fired / no write
//! occurred / same fatal signal), not merely "both did something".

mod common;

use common::*;

const BUF_MED: usize = 65536;
const BUF_BIG: usize = 512 * 1024;

/// The sentinel `Inputs::new` writes into `grbuf`; used to prove a row really
/// did skip all writes rather than coincidentally writing matching values.
fn sentinel(i: usize) -> u32 {
    0x7FA0_0000 ^ (i as u32).wrapping_mul(0x9E37_79B9)
}

fn assert_grbuf_untouched(o: &Outcome, what: &str) {
    for (i, w) in o.grbuf.iter().enumerate() {
        assert_eq!(*w, sentinel(i), "{what}: grbuf word {i} was written but should not have been");
    }
}

// --------------------------------------------------------------------------
// E1 -- guard trips on the very first get_bits call (limit == 0)
// --------------------------------------------------------------------------
#[test]
fn e01_exhausted_immediately() {
    let libs = Libs::load();
    let mut rng = Rng::new(0xE001);
    for it in 0..300 {
        let ba = rng.range(1, 16) as u8;
        let mut inp = Inputs::new(&mut rng, BUF_MED);
        inp.ba_all(|_| ba).total_bands(4).group_size(3).pos(0).limit(0);
        let (c, _) = assert_same_ret(&libs, "E1", it, &inp.done());
        // The guard fired every time: get_bits returned 0, so every sample is
        // `0 - half`, and pos advanced by the full bit demand anyway.
        let half = (1i32 << (ba as i32 - 1)) - 1;
        assert_eq!(c.grbuf[GR_PREFIX], ((0 - half) as f32).to_bits(), "E1: expected 0-half sample");
        assert_eq!(c.pos, 4 * 8 * 3 * ba as i32, "E1: pos must advance past the limit");
        assert_eq!(c.ret, 12);
    }
}

// --------------------------------------------------------------------------
// E2 -- guard trips mid-run: prefix real, suffix all zero bits
// --------------------------------------------------------------------------
#[test]
fn e02_exhausted_mid_run() {
    let libs = Libs::load();
    let mut rng = Rng::new(0xE002);
    for it in 0..300 {
        let mut inp = Inputs::new(&mut rng, BUF_MED);
        let ba = rng.range(1, 16) as u8;
        let total = 4 * 32 * 12 * ba as i32;
        let lim = rng.range(1, total as i64 - 1) as i32;
        inp.ba_all(|_| ba).total_bands(16).group_size(12).pos(0).limit(lim);
        let (c, _) = assert_same_ret(&libs, "E2", it, &inp.done());
        assert_eq!(c.pos, 4 * 32 * 12 * ba as i32, "E2: pos advances for every call");
    }
}

// --------------------------------------------------------------------------
// E3 / E4 -- the guard is `>` : exactly-at-limit reads, one-past does not
// --------------------------------------------------------------------------
#[test]
fn e03_e04_boundary_exactly_and_one_past() {
    let libs = Libs::load();
    let mut rng = Rng::new(0xE003);
    for it in 0..300 {
        // A single band, a single sample: the whole run is one get_bits(ba).
        let ba = rng.range(1, 16) as u8;
        let half = (1i32 << (ba as i32 - 1)) - 1;

        // E3: pos + n == limit -> guard NOT taken.
        let mut at = Inputs::new(&mut rng, BUF_MED);
        at.ba_all(|i| if i == 0 { ba } else { 0 })
            .total_bands(1)
            .group_size(1)
            .pos(0)
            .limit(ba as i32);
        let (c_at, _) = assert_same_ret(&libs, "E3", it, &at.done());

        // E4: pos + n == limit + 1 -> guard taken, all bits zero.
        let mut past = at.clone();
        past.limit(ba as i32 - 1);
        let (c_past, _) = assert_same_ret(&libs, "E4", it, &past.done());

        assert_eq!(
            c_past.grbuf[GR_PREFIX],
            ((0 - half) as f32).to_bits(),
            "E4: one past the limit must yield 0 bits"
        );
        // And the two must actually differ for some input, otherwise the
        // boundary test is vacuous -- checked in aggregate below.
        if c_at.grbuf[GR_PREFIX] != c_past.grbuf[GR_PREFIX] {
            return;
        }
    }
}

#[test]
fn e03_boundary_is_not_vacuous() {
    // Prove `limit == pos + n` really does read the bitstream (guard is `>`),
    // by finding an input where at-limit and one-past-limit differ.
    let libs = Libs::load();
    let mut rng = Rng::new(0xE004);
    let mut differed = false;
    for it in 0..300 {
        let mut at = Inputs::new(&mut rng, BUF_MED);
        at.ba_all(|i| if i == 0 { 16 } else { 0 }).total_bands(1).group_size(1).pos(0).limit(16);
        let (c_at, _) = assert_same_ret(&libs, "E3v", it, &at.done());
        let mut past = at.clone();
        past.limit(15);
        let (c_past, _) = assert_same_ret(&libs, "E3v", it, &past.done());
        if c_at.grbuf[GR_PREFIX] != c_past.grbuf[GR_PREFIX] {
            differed = true;
            break;
        }
    }
    assert!(differed, "E3: at-limit and past-limit never differed; boundary test is vacuous");
}

// --------------------------------------------------------------------------
// E5 -- negative limit: every call trips the guard
// --------------------------------------------------------------------------
#[test]
fn e05_negative_limit() {
    let libs = Libs::load();
    let mut rng = Rng::new(0xE005);
    for (it, lim) in [-1i32, -7, -1000, i32::MIN].iter().enumerate() {
        let mut inp = Inputs::new(&mut rng, BUF_MED);
        let set: Vec<u8> = (0..=255u8).collect();
        let picks: Vec<u8> = (0..(SCI_REGION - OFF_BITALLOC)).map(|_| rng.pick(&set)).collect();
        let mut pk = picks.into_iter();
        inp.ba_all(|_| pk.next().unwrap()).total_bands(8).group_size(3).pos(0).limit(*lim);
        assert_same_ret(&libs, "E5", it, &inp.done());
    }
}

// --------------------------------------------------------------------------
// E6 -- unaligned pos: the first byte is masked with `255 >> s`
// --------------------------------------------------------------------------
#[test]
fn e06_unaligned_first_byte_mask() {
    let libs = Libs::load();
    let mut rng = Rng::new(0xE006);
    for s in 1..8i32 {
        let mut inp = Inputs::new(&mut rng, BUF_MED);
        inp.ba_all(|i| if i == 0 { 8 } else { 0 })
            .total_bands(1)
            .group_size(1)
            .pos(s)
            .buf_fill(0xFF);
        let (c, _) = assert_same_ret(&libs, "E6", s as usize, &inp.done());
        // With an all-0xFF stream, 8 bits starting at bit offset `s` still read
        // 0xFF -- but the mask discards the top `s` bits of the *first* byte and
        // the loop supplies them from the next byte, so the value is 255.
        assert_eq!(c.grbuf[GR_PREFIX], (255.0f32 - 127.0).to_bits());
    }
}

// --------------------------------------------------------------------------
// E7 -- pos already beyond limit before the first call
// --------------------------------------------------------------------------
#[test]
fn e07_pos_already_past_limit() {
    let libs = Libs::load();
    let mut rng = Rng::new(0xE007);
    for it in 0..300 {
        let mut inp = Inputs::new(&mut rng, BUF_MED);
        let pos = rng.range(101, 5000) as i32;
        inp.ba_all(|_| 12).total_bands(4).group_size(3).pos(pos).limit(100);
        let (c, _) = assert_same_ret(&libs, "E7", it, &inp.done());
        let half = (1i32 << 11) - 1;
        assert_eq!(c.grbuf[GR_PREFIX], ((0 - half) as f32).to_bits(), "E7: guard must fire");
    }
}

// --------------------------------------------------------------------------
// E8 -- `while ((shl -= 8) > 0)` body never executes (n + (pos&7) <= 8)
// --------------------------------------------------------------------------
#[test]
fn e08_shift_loop_never_runs() {
    let libs = Libs::load();
    let mut rng = Rng::new(0xE008);
    let mut it = 0;
    for s in 0..8i32 {
        for n in 1..=8i32 {
            if n + s > 8 {
                continue;
            }
            let mut inp = Inputs::new(&mut rng, BUF_MED);
            inp.ba_all(|i| if i == 0 { n as u8 } else { 0 })
                .total_bands(1)
                .group_size(1)
                .pos(s);
            assert_same_ret(&libs, "E8", it, &inp.done());
            it += 1;
        }
    }
    assert!(it > 0);
}

// --------------------------------------------------------------------------
// E9 -- total_bands == 0: no writes, bs untouched, return group_size*4
// --------------------------------------------------------------------------
#[test]
fn e09_total_bands_zero_no_effect() {
    let libs = Libs::load();
    let mut rng = Rng::new(0xE009);
    for (it, gs) in [-3i32, 0, 1, 3, 12, 64].iter().enumerate() {
        let mut inp = Inputs::new(&mut rng, BUF_MED);
        let set: Vec<u8> = (1..=255u8).collect();
        let picks: Vec<u8> = (0..(SCI_REGION - OFF_BITALLOC)).map(|_| rng.pick(&set)).collect();
        let mut pk = picks.into_iter();
        inp.ba_all(|_| pk.next().unwrap()).total_bands(0).group_size(*gs).pos(5).limit(1000);
        let (c, _) = assert_same_ret(&libs, "E9", it, &inp.done());
        assert_grbuf_untouched(&c, "E9");
        assert_eq!(c.pos, 5, "E9: bs->pos must be untouched");
        assert_eq!(c.ret, gs.wrapping_mul(4));
    }
}

// --------------------------------------------------------------------------
// E10 -- every ba == 0: no get_bits, no write, pos unchanged
// --------------------------------------------------------------------------
#[test]
fn e10_all_bands_skipped() {
    let libs = Libs::load();
    let mut rng = Rng::new(0xE00A);
    for (it, tb) in [1u8, 8, 32, 64, 255].iter().enumerate() {
        let mut inp = Inputs::new(&mut rng, BUF_MED);
        inp.ba_all(|_| 0).total_bands(*tb).group_size(12).pos(9).limit(100000);
        let (c, _) = assert_same_ret(&libs, "E10", it, &inp.done());
        assert_grbuf_untouched(&c, "E10");
        assert_eq!(c.pos, 9, "E10: no get_bits call may happen");
        assert_eq!(c.ret, 48);
    }
}

// --------------------------------------------------------------------------
// E11 -- group_size == 0: linear path consumes nothing, grouped path still
//        consumes exactly one get_bits per active band
// --------------------------------------------------------------------------
#[test]
fn e11_group_size_zero_still_consumes_grouped() {
    let libs = Libs::load();
    let mut rng = Rng::new(0xE00B);

    // Linear only -> nothing consumed.
    let mut lin = Inputs::new(&mut rng, BUF_MED);
    lin.ba_all(|_| 9).total_bands(8).group_size(0).pos(0).limit(100000);
    let (c_lin, _) = assert_same_ret(&libs, "E11-linear", 0, &lin.done());
    assert_grbuf_untouched(&c_lin, "E11-linear");
    assert_eq!(c_lin.pos, 0, "E11: `ba<17` with group_size 0 must not call get_bits");
    assert_eq!(c_lin.ret, 0);

    // Grouped -> one get_bits per band even though no sample is produced.
    let mut grp = Inputs::new(&mut rng, BUF_MED);
    grp.ba_all(|_| 17).total_bands(8).group_size(0).pos(0).limit(100000);
    let (c_grp, _) = assert_same_ret(&libs, "E11-grouped", 1, &grp.done());
    assert_grbuf_untouched(&c_grp, "E11-grouped");
    assert_eq!(c_grp.pos, 4 * 16 * 5, "E11: `ba>=17` consumes n bits per band regardless");
    assert_eq!(c_grp.ret, 0);
}

// --------------------------------------------------------------------------
// E12 -- negative group_size: no writes, dst starts before grbuf
// --------------------------------------------------------------------------
#[test]
fn e12_negative_group_size() {
    let libs = Libs::load();
    let mut rng = Rng::new(0xE00C);
    for (it, gs) in [-1i32, -7, -33, -100].iter().enumerate() {
        let mut inp = Inputs::new(&mut rng, BUF_MED);
        let mut set: Vec<u8> = (1..=21u8).collect();
        set.push(0);
        let picks: Vec<u8> = (0..(SCI_REGION - OFF_BITALLOC)).map(|_| rng.pick(&set)).collect();
        let mut pk = picks.into_iter();
        inp.ba_all(|_| pk.next().unwrap()).total_bands(8).group_size(*gs).pos(0).limit(1000000);
        let (c, _) = assert_same_ret(&libs, "E12", it, &inp.done());
        assert_grbuf_untouched(&c, "E12");
        assert_eq!(c.ret, gs.wrapping_mul(4));
    }
}

// --------------------------------------------------------------------------
// E13 -- oversized group_size, no clamp exists
// --------------------------------------------------------------------------
#[test]
fn e13_oversized_group_size() {
    let libs = Libs::load();
    let mut rng = Rng::new(0xE00D);
    for (it, gs) in [64i32, 128, 256].iter().enumerate() {
        for rep in 0..40 {
            let mut inp = Inputs::new(&mut rng, BUF_BIG);
            inp.ba_all(|_| 6).total_bands(2).group_size(*gs).pos(0);
            let (c, _) = assert_same_ret(&libs, "E13", it * 4 + rep, &inp.done());
            assert_eq!(c.ret, gs * 4);
        }
    }
}

// --------------------------------------------------------------------------
// E14 / E15 -- the ba == 16 / ba == 17 branch boundary
// --------------------------------------------------------------------------
#[test]
fn e14_e15_linear_grouped_boundary() {
    let libs = Libs::load();
    let mut rng = Rng::new(0xE00E);
    for it in 0..200 {
        for ba in [16u8, 17] {
            let mut inp = Inputs::new(&mut rng, BUF_MED);
            inp.ba_all(|_| ba).total_bands(4).group_size(12).pos(0);
            let (c, _) = assert_same_ret(&libs, "E14/E15", it, &inp.done());
            if ba == 16 {
                // 16 bits per sample, half = 32767, one get_bits per sample.
                assert_eq!(c.pos, 4 * 8 * 12 * 16);
            } else {
                // mod == 3 -> n == 5, one get_bits per band.
                assert_eq!(c.pos, 4 * 8 * 5);
            }
        }
    }
}

// --------------------------------------------------------------------------
// E16 -- grouped path asking for more than 32 bits (shift-count UB)
// --------------------------------------------------------------------------
#[test]
fn e16_grouped_over_32_bits() {
    let libs = Libs::load();
    let mut rng = Rng::new(0xE00F);
    for ba in 22..=31u8 {
        for rep in 0..40 {
            assert!(grouped_n(ba as i32) > 32, "row premise: n must exceed 32 for ba={ba}");
            let mut inp = Inputs::new(&mut rng, BUF_BIG);
            inp.ba_all(|_| ba).total_bands(2).group_size(3).pos(rep);
            assert_same_ret(&libs, "E16", (ba as usize) * 4 + rep as usize, &inp.done());
        }
    }
}

// --------------------------------------------------------------------------
// E17 -- ba >= 49: `2 << (ba-17)` shift count >= 32, wraps mod 32
// --------------------------------------------------------------------------
#[test]
fn e17_shift_count_wraps_mod_32() {
    let libs = Libs::load();
    let mut rng = Rng::new(0xE010);
    // ba and ba+32 must behave identically if the shift count is masked to 5
    // bits; assert the C really does that, then require Rust to match.
    for ba in 17..=21u8 {
        let mut a = Inputs::new(&mut rng, BUF_MED);
        a.ba_all(|_| ba).total_bands(2).group_size(3).pos(0);
        let (c_a, _) = assert_same_ret(&libs, "E17", ba as usize, &a.done());

        let mut b = a.clone();
        b.ba_all(|_| ba + 32);
        let (c_b, _) = assert_same_ret(&libs, "E17", ba as usize + 100, &b.done());

        assert_eq!(c_a.pos, c_b.pos, "E17: ba={ba} and ba={} must consume the same", ba + 32);
        assert!(c_a.grbuf == c_b.grbuf, "E17: ba={ba} and ba={} must decode the same", ba + 32);
    }
}

// --------------------------------------------------------------------------
// E18 -- ba == 48: mod == 1, every sample is exactly 0.0, n == 3
// --------------------------------------------------------------------------
#[test]
fn e18_ba48_mod_one_all_zero() {
    let libs = Libs::load();
    let mut rng = Rng::new(0xE011);
    assert_eq!(grouped_n(48), 3, "row premise: ba=48 asks for 3 bits");
    for it in 0..200 {
        let mut inp = Inputs::new(&mut rng, BUF_MED);
        inp.ba_all(|_| 48).total_bands(2).group_size(12).pos(0);
        let (c, _) = assert_same_ret(&libs, "E18", it, &inp.done());
        assert_eq!(c.pos, 4 * 4 * 3, "E18: 3 bits per band");
        assert_eq!(c.grbuf[GR_PREFIX], 0.0f32.to_bits(), "E18: mod==1 makes every sample 0");
    }
}

// --------------------------------------------------------------------------
// E19 -- ba whose bit demand dwarfs the buffer: the guard fires before any
//        dereference, so there is no out-of-bounds read
// --------------------------------------------------------------------------
#[test]
fn e19_huge_bit_demand_guard_first() {
    let libs = Libs::load();
    let mut rng = Rng::new(0xE012);
    for ba in [38u8, 39, 40] {
        let n = grouped_n(ba as i32);
        assert!(n > 1_000_000, "row premise: ba={ba} asks for {n} bits");
        let mut inp = Inputs::new(&mut rng, BUF_MED);
        // Only bitalloc[0] is active -> 4 calls total, so `pos` cannot overflow.
        inp.ba_all(|i| if i == 0 { ba } else { 0 })
            .total_bands(1)
            .group_size(3)
            .pos(0)
            .limit(1000);
        let (c, _) = assert_same_ret(&libs, "E19", ba as usize, &inp.done());
        assert!(!c.crashed(), "E19: the guard must prevent the wild read");
        assert_eq!(c.pos, n.wrapping_mul(4), "E19: pos still advances by n each call");
    }
}

// --------------------------------------------------------------------------
// E20 -- `code % mod - mod / 2` wraps in unsigned before the cast to int
// --------------------------------------------------------------------------
#[test]
fn e20_unsigned_wrap_before_int_cast() {
    let libs = Libs::load();
    let mut rng = Rng::new(0xE013);
    let mut saw_negative = false;
    for it in 0..300 {
        let mut inp = Inputs::new(&mut rng, BUF_MED);
        // mod == 17 (ba == 20) -> mod/2 == 8, so codes 0..7 must come out
        // negative via the unsigned wrap.
        inp.ba_all(|_| 20).total_bands(8).group_size(12).pos(0);
        let (c, _) = assert_same_ret(&libs, "E20", it, &inp.done());
        for w in &c.grbuf {
            let f = f32::from_bits(*w);
            if f < 0.0 && f >= -8.0 {
                saw_negative = true;
            }
        }
    }
    assert!(saw_negative, "E20: never produced a negative sample; the wrap path was not reached");
}

// --------------------------------------------------------------------------
// E21 / E22 -- bitalloc index leaving its array, then leaving the struct
// --------------------------------------------------------------------------
#[test]
fn e21_index_reads_scfcod() {
    let libs = Libs::load();
    let mut rng = Rng::new(0xE014);
    for it in 0..200 {
        let mut inp = Inputs::new(&mut rng, BUF_BIG);
        // In-array slots inert, `scfcod` slots active: any decoding at all
        // proves `ba` was fetched from beyond `bitalloc[64]`.
        inp.ba_all(|i| if i < 64 { 0 } else { 7 }).total_bands(40).group_size(3).pos(0);
        let (c, _) = assert_same_ret(&libs, "E21", it, &inp.done());
        assert_ne!(c.pos, 0, "E21: premise -- ba must have been read out of `scfcod`");
    }
}

#[test]
fn e22_index_reads_past_struct() {
    let libs = Libs::load();
    let mut rng = Rng::new(0xE015);
    for it in 0..200 {
        let mut inp = Inputs::new(&mut rng, BUF_BIG);
        // Everything inside the struct inert; only the padding past byte 900 is
        // active, i.e. index >= 130.
        inp.ba_all(|i| if OFF_BITALLOC + i < SCI_SIZE { 0 } else { 5 })
            .total_bands(255)
            .group_size(3)
            .pos(0);
        let (c, _) = assert_same_ret(&libs, "E22", it, &inp.done());
        assert_ne!(c.pos, 0, "E22: premise -- ba must have been read past the struct end");
    }
}

// --------------------------------------------------------------------------
// E23 -- total_bands == 255 (max uint8_t): 510 bands, choff phase preserved
// --------------------------------------------------------------------------
#[test]
fn e23_total_bands_max() {
    let libs = Libs::load();
    let mut rng = Rng::new(0xE016);
    for it in 0..200 {
        let mut inp = Inputs::new(&mut rng, BUF_BIG);
        inp.ba_all(|_| 4).total_bands(255).group_size(3).pos(0);
        let (c, _) = assert_same_ret(&libs, "E23", it, &inp.done());
        assert_eq!(c.pos, 4 * 510 * 3 * 4, "E23: 510 bands per granule");
    }
}

// --------------------------------------------------------------------------
// E24 -- out-of-range "enum" ints across the FFI boundary. C enums accept any
//        int and this API has no valid-value check at all: sweep every ba byte
//        and a spread of extreme group_size / pos / limit values.
// --------------------------------------------------------------------------
#[test]
fn e24_all_ba_values_no_rejection() {
    let libs = Libs::load();
    let mut rng = Rng::new(0xE017);
    let mut crashes = 0usize;
    for ba in 0..=255u8 {
        let mut inp = Inputs::new(&mut rng, BUF_MED);
        inp.ba_all(|_| ba).total_bands(3).group_size(3).pos(0).limit(64);
        let (c, r) = assert_same_ret(&libs, "E24-ba", ba as usize, &inp.done());
        assert_eq!(c.crashed(), r.crashed());
        if c.crashed() {
            crashes += 1;
        }
    }
    // Not a vacuous sweep: the huge-`n` values really do make `bs->pos`
    // overflow `int`, the guard miss, and both libraries fault identically.
    assert!(crashes > 0, "E24: expected some ba values to fault in BOTH libraries");
    assert!(crashes < 256, "E24: expected most ba values to complete normally");
}

#[test]
fn e24_extreme_scalar_arguments() {
    let libs = Libs::load();
    let mut rng = Rng::new(0xE018);
    let gss = [i32::MIN, i32::MIN + 1, -1000000, -1, 0, 1, 1000000, i32::MAX];
    let poss = [i32::MIN, -1, 0, 1, i32::MAX];
    let lims = [i32::MIN, -1, 0, 1, i32::MAX];
    let mut it = 0;
    for gs in gss {
        for pos in poss {
            for lim in lims {
                let mut inp = Inputs::new(&mut rng, BUF_MED);
                // `ba == 0` everywhere: no get_bits, no grbuf write, so these
                // extremes exercise only the loop bounds and the return value.
                inp.ba_all(|_| 0).total_bands(8).group_size(gs).pos(pos).limit(lim);
                let (c, _) = assert_same_ret(&libs, "E24-scalar", it, &inp.done());
                assert_eq!(c.ret, gs.wrapping_mul(4));
                it += 1;
            }
        }
    }
    // And once more with an active band, but only where no wild pointer can
    // form: group_size <= 0 means the linear path never calls get_bits.
    for gs in [i32::MIN, -1, 0] {
        for pos in poss {
            for lim in lims {
                let mut inp = Inputs::new(&mut rng, BUF_MED);
                inp.ba_all(|_| 9).total_bands(8).group_size(gs).pos(pos).limit(lim);
                assert_same_ret(&libs, "E24-scalar-active", it, &inp.done());
                it += 1;
            }
        }
    }
}

// --------------------------------------------------------------------------
// E25 -- degenerate bitstream content
// --------------------------------------------------------------------------
#[test]
fn e25_degenerate_buffer_content() {
    let libs = Libs::load();
    let mut rng = Rng::new(0xE019);
    for (it, fill) in [0x00u8, 0xFF].iter().enumerate() {
        let mut inp = Inputs::new(&mut rng, BUF_MED);
        inp.ba_all(|_| 10).total_bands(8).group_size(12).pos(0).buf_fill(*fill);
        let (c, _) = assert_same_ret(&libs, "E25", it, &inp.done());
        let half = (1i32 << 9) - 1;
        let expect =
            if *fill == 0 { (0 - half) as f32 } else { (1023 - half) as f32 };
        assert_eq!(c.grbuf[GR_PREFIX], expect.to_bits(), "E25: fill=0x{fill:02X}");
    }
}

// --------------------------------------------------------------------------
// E26 -- NULL pointers. The C has no null check; both libraries must agree on
//        which combinations survive (because a guard makes the deref
//        unreachable) and which fault.
// --------------------------------------------------------------------------
#[test]
fn e26_null_pointers() {
    let libs = Libs::load();
    let mut rng = Rng::new(0xE01A);

    struct Case {
        name: &'static str,
        null_gr: bool,
        null_bs: bool,
        null_sci: bool,
        tb: u8,
        ba: u8,
        gs: i32,
        expect_crash: bool,
    }
    let cases = [
        // sci is always dereferenced (total_bands) -> always fatal.
        Case { name: "sci null", null_gr: false, null_bs: false, null_sci: true, tb: 0, ba: 0, gs: 3, expect_crash: true },
        Case { name: "all null", null_gr: true, null_bs: true, null_sci: true, tb: 0, ba: 0, gs: 3, expect_crash: true },
        // total_bands == 0 -> neither grbuf nor bs is ever touched.
        Case { name: "grbuf+bs null, tb=0", null_gr: true, null_bs: true, null_sci: false, tb: 0, ba: 9, gs: 3, expect_crash: false },
        // every band skipped -> still no grbuf write and no get_bits.
        Case { name: "grbuf+bs null, ba=0", null_gr: true, null_bs: true, null_sci: false, tb: 8, ba: 0, gs: 3, expect_crash: false },
        // group_size <= 0 with a linear band -> k loop empty, no get_bits.
        Case { name: "grbuf+bs null, gs=0", null_gr: true, null_bs: true, null_sci: false, tb: 8, ba: 9, gs: 0, expect_crash: false },
        Case { name: "grbuf+bs null, gs=-4", null_gr: true, null_bs: true, null_sci: false, tb: 8, ba: 9, gs: -4, expect_crash: false },
        // active linear band with group_size > 0 -> get_bits derefs bs.
        Case { name: "bs null, active", null_gr: false, null_bs: true, null_sci: false, tb: 4, ba: 9, gs: 3, expect_crash: true },
        // active band, valid bs, NULL grbuf -> the store faults.
        Case { name: "grbuf null, active", null_gr: true, null_bs: false, null_sci: false, tb: 4, ba: 9, gs: 3, expect_crash: true },
        // grouped band with group_size == 0 -> one get_bits, so bs matters,
        // but grbuf is never written.
        Case { name: "grbuf null, grouped gs=0", null_gr: true, null_bs: false, null_sci: false, tb: 4, ba: 17, gs: 0, expect_crash: false },
    ];

    for (it, cs) in cases.iter().enumerate() {
        let mut inp = Inputs::new(&mut rng, BUF_MED);
        inp.ba_all(|_| cs.ba)
            .total_bands(cs.tb)
            .group_size(cs.gs)
            .pos(0)
            .limit(1000000)
            .nulls(cs.null_gr, cs.null_bs, cs.null_sci);
        let (c, r) = assert_same_ret(&libs, "E26", it, &inp.done());
        assert_eq!(
            c.crashed(),
            cs.expect_crash,
            "E26 case `{}`: C outcome {} contradicts the row's premise",
            cs.name,
            c.describe()
        );
        assert_eq!(c.crashed(), r.crashed(), "E26 case `{}`", cs.name);
    }
}
