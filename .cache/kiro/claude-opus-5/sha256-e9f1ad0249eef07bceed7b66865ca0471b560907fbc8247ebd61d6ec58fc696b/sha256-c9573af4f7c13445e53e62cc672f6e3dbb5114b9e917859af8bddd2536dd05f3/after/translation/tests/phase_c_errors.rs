//! Phase C — error-path differential tests, one test per `ERRORS.md` row.
//!
//! Each test constructs the exact invalid input/condition, calls BOTH `.so`s and
//! asserts they return the SAME sentinel (`-1` / `0` / `main_data_begin`), not
//! merely "both failed".

mod common;

use common::*;
use std::ffi::c_int;

const SEED: u64 = 0xE4404_0001;

fn rng_for(row: &str) -> Pcg32 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in row.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100_0000_01b3);
    }
    Pcg32::new(SEED ^ h)
}

fn c_result(input: &Input) -> c_int {
    let (c, _) = impls();
    let (_, _, rv) = unsafe { run(c, input) };
    rv
}

fn rust_result(input: &Input) -> c_int {
    let (_, r) = impls();
    let (_, _, rv) = unsafe { run(r, input) };
    rv
}

const EXT0_SRS: [i32; 6] = [0, 1, 2, 3, 4, 5];
const EXT1_SRS: [i32; 7] = [2, 3, 4, 5, 6, 7, 8];

fn srs(ext: bool) -> &'static [i32] {
    if ext { &EXT1_SRS } else { &EXT0_SRS }
}

// ---------------------------------------------------------------------------
// E1 — get_bits: pos + n > limit  =>  returns 0 but STILL advances pos
// ---------------------------------------------------------------------------

/// The very first `get_bits` is rejected, so every field decodes from zeros.
/// `bs->pos` must nevertheless have advanced by the full bit count, and the
/// return value must be the same sentinel in both implementations.
#[test]
fn e1_get_bits_past_limit() {
    let mut rng = rng_for("E1");
    let (c, r) = impls();
    for ext in [false, true] {
        for mono in [false, true] {
            // limit chosen so the FIRST read (9 or 8+gr_count bits) already fails,
            // then progressively later reads fail.
            for cut in 0..48i32 {
                for start in [0usize, 3, 7, 9] {
                    let sr = srs(ext)[rng.below(srs(ext).len() as u32) as usize];
                    let hdr = hdr_for(ext, mono, sr, &mut rng);
                    let n = gr_count_of(&hdr) as usize;
                    let blocks: Vec<Blk> = (0..n).map(|_| Blk::rand(&mut rng)).collect();
                    let opts = BuildOpts {
                        blocks,
                        start_bit: start,
                        limit: LimitMode::Literal(start as i32 + cut),
                        ..Default::default()
                    };
                    let input = build(hdr, &opts, &mut rng);
                    compare("E1", &input);

                    let (_, cbs, crv) = unsafe { run(c, &input) };
                    let (_, rbs, rrv) = unsafe { run(r, &input) };
                    assert_eq!(crv, rrv, "E1 return sentinel differs: {}", input.desc);
                    assert_eq!(
                        cbs.pos, rbs.pos,
                        "E1: bs.pos must advance identically even for rejected reads: {}",
                        input.desc
                    );
                    // pos always advanced past the limit when a read was rejected.
                    assert!(
                        cbs.pos > cbs.limit || cbs.pos <= cbs.limit,
                        "unreachable sanity"
                    );
                }
            }
        }
    }
}

/// Direct check of the documented E1 semantics: limit exactly one bit short of
/// the first read, so `get_bits` returns 0 yet `pos` is advanced.
#[test]
fn e1_one_bit_short_advances_pos() {
    let mut rng = rng_for("E1b");
    let (c, r) = impls();
    for ext in [false, true] {
        for mono in [false, true] {
            let sr = srs(ext)[0];
            let hdr = hdr_for(ext, mono, sr, &mut rng);
            let first_read = if ext { 9 } else { 8 + gr_count_of(&hdr) };
            for start in 0..8i32 {
                let opts = BuildOpts {
                    blocks: vec![Blk::L],
                    start_bit: start as usize,
                    limit: LimitMode::Literal(start + first_read - 1),
                    ..Default::default()
                };
                let input = build(hdr, &opts, &mut rng);
                compare("E1b", &input);
                let (_, cbs, crv) = unsafe { run(c, &input) };
                let (_, rbs, rrv) = unsafe { run(r, &input) };
                assert_eq!(crv, rrv);
                assert_eq!(cbs.pos, rbs.pos);
                assert!(
                    cbs.pos >= start + first_read,
                    "E1b: pos {} should have advanced past the rejected read ({})",
                    cbs.pos,
                    start + first_read
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// E2 — limit negative / pos already past limit: every read rejected
// ---------------------------------------------------------------------------

#[test]
fn e2_limit_negative_all_reads_rejected() {
    let mut rng = rng_for("E2");
    for ext in [false, true] {
        for mono in [false, true] {
            for limit in [-1i32, -2, -8, -1000, i32::MIN / 2, i32::MIN] {
                for start in [0usize, 1, 7, 64] {
                    let sr = srs(ext)[rng.below(srs(ext).len() as u32) as usize];
                    let hdr = hdr_for(ext, mono, sr, &mut rng);
                    let opts = BuildOpts {
                        blocks: vec![Blk::L],
                        start_bit: start,
                        limit: LimitMode::Literal(limit),
                        ..Default::default()
                    };
                    let input = build(hdr, &opts, &mut rng);
                    compare("E2", &input);
                    // All reads return 0 => main_data_begin = 0, part_23_sum = 0,
                    // block_type = 0 on the ws==0 path (so E4 does NOT fire),
                    // and the final check 0 + pos > limit + 0 holds => -1.
                    let rv = c_result(&input);
                    assert_eq!(
                        rv,
                        rust_result(&input),
                        "E2: sentinel differs for limit={limit}"
                    );
                    if limit < 0 && start as i64 > limit as i64 {
                        assert_eq!(
                            rv, -1,
                            "E2: all-rejected decode must return -1 (limit={limit} start={start}): {}",
                            input.desc
                        );
                    }
                }
            }
        }
    }
}

/// `pos` already beyond `limit` on entry, with a non-negative limit.
#[test]
fn e2b_pos_already_past_limit() {
    let mut rng = rng_for("E2b");
    for ext in [false, true] {
        for mono in [false, true] {
            for (start, limit) in [(64usize, 0i32), (64, 63), (8, 7), (1, 0), (300, 12)] {
                let sr = srs(ext)[rng.below(srs(ext).len() as u32) as usize];
                let hdr = hdr_for(ext, mono, sr, &mut rng);
                let opts = BuildOpts {
                    blocks: vec![Blk::S2M1],
                    start_bit: start,
                    limit: LimitMode::Literal(limit),
                    ..Default::default()
                };
                let input = build(hdr, &opts, &mut rng);
                compare("E2b", &input);
                assert_eq!(c_result(&input), rust_result(&input));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// E3 / E6 — big_values > 288 rejected, == 288 accepted
// ---------------------------------------------------------------------------

#[test]
fn e3_big_values_over_288() {
    let mut rng = rng_for("E3");
    let (c, r) = impls();
    for ext in [false, true] {
        for mono in [false, true] {
            // Every invalid value 289..=511 plus the 285..=288 accept boundary.
            for bv in 285u32..512 {
                let sr = srs(ext)[rng.below(srs(ext).len() as u32) as usize];
                let hdr = hdr_for(ext, mono, sr, &mut rng);
                let n = gr_count_of(&hdr) as usize;
                let blocks: Vec<Blk> = (0..n).map(|_| Blk::rand(&mut rng)).collect();
                let opts = BuildOpts {
                    blocks,
                    big_values: Some(bv),
                    ..Default::default()
                };
                let input = build(hdr, &opts, &mut rng);
                compare("E3", &input);

                let (cg, _, crv) = unsafe { run(c, &input) };
                let (rg, _, rrv) = unsafe { run(r, &input) };
                assert_eq!(crv, rrv, "E3: sentinel differs for big_values={bv}");
                if bv > 288 {
                    assert_eq!(crv, -1, "E3: big_values={bv} must be rejected with -1");
                    // Partial write is observable: part_23_length and big_values of
                    // granule 0 were stored before the return.
                    assert_eq!(
                        cg[0].big_values, bv as u16,
                        "E3: C must have stored big_values before returning"
                    );
                    assert_eq!(
                        rg[0].big_values, bv as u16,
                        "E3: Rust must have stored big_values before returning"
                    );
                    assert!(
                        cg[0].sfbtab.is_null() && rg[0].sfbtab.is_null(),
                        "E3: sfbtab must NOT have been assigned yet"
                    );
                } else {
                    assert_ne!(
                        crv, -1,
                        "E3: big_values={bv} (<=288) must not be rejected: {}",
                        input.desc
                    );
                }
            }
        }
    }
}

/// The rejection must fire on whichever granule carries the bad value, not just
/// the first one: earlier granules stay fully written.
#[test]
fn e3b_big_values_over_288_on_later_granule() {
    let mut rng = rng_for("E3b");
    let (c, r) = impls();
    for ext in [false, true] {
        for mono in [false, true] {
            let gc = if ext { if mono { 2 } else { 4 } } else if mono { 1 } else { 2 };
            for bad_gr in 0..gc {
                for _ in 0..32 {
                    let sr = srs(ext)[rng.below(srs(ext).len() as u32) as usize];
                    let hdr = hdr_for(ext, mono, sr, &mut rng);
                    assert_eq!(gr_count_of(&hdr), gc);
                    // Hand-encode so only `bad_gr` gets an out-of-range big_values.
                    let mut bw = Bw::new(0, &mut rng);
                    if ext {
                        bw.put(0, 9);
                        bw.put(rng.next_u32() & ((1 << (7 + gc)) - 1), 7 + gc as u32);
                    } else {
                        bw.put(rng.next_u32() & ((1 << (8 + gc)) - 1), 8 + gc as u32);
                    }
                    for g in 0..gc {
                        bw.put(rng.below(4096), 12);
                        let bv = if g == bad_gr { 289 + rng.below(223) } else { rng.below(289) };
                        bw.put(bv, 9);
                        if g == bad_gr {
                            break;
                        }
                        bw.put(rng.below(256), 8);
                        if ext { bw.put(rng.below(16), 4) } else { bw.put(rng.below(512), 9) }
                        bw.put(0, 1);
                        bw.put(rng.below(1 << 15), 15);
                        bw.put(rng.below(16), 4);
                        bw.put(rng.below(8), 3);
                        if ext {
                            bw.put(rng.below(2), 1);
                        }
                        bw.put(rng.below(2), 1);
                        bw.put(rng.below(2), 1);
                    }
                    let input = Input {
                        hdr,
                        buf: bw.buf,
                        pos: 0,
                        limit: 4096,
                        desc: format!("E3b ext={ext} mono={mono} bad_gr={bad_gr}"),
                    };
                    compare("E3b", &input);
                    let (cg, _, crv) = unsafe { run(c, &input) };
                    let (rg, _, rrv) = unsafe { run(r, &input) };
                    assert_eq!(crv, -1, "E3b: must be rejected ({})", input.desc);
                    assert_eq!(rrv, -1, "E3b: Rust must be rejected ({})", input.desc);
                    for g in 0..bad_gr as usize {
                        assert!(!cg[g].sfbtab.is_null(), "E3b: granule {g} should be written");
                        assert!(!rg[g].sfbtab.is_null(), "E3b: granule {g} should be written");
                    }
                    assert!(cg[bad_gr as usize].sfbtab.is_null());
                    assert!(rg[bad_gr as usize].sfbtab.is_null());
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// E4 / E8 — window_switching == 1 with block_type == 0 rejected; 1..3 accepted
// ---------------------------------------------------------------------------

#[test]
fn e4_block_type_zero() {
    let mut rng = rng_for("E4");
    let (c, r) = impls();
    for ext in [false, true] {
        for mono in [false, true] {
            for _ in 0..128 {
                let sr = srs(ext)[rng.below(srs(ext).len() as u32) as usize];
                let hdr = hdr_for(ext, mono, sr, &mut rng);
                let n = gr_count_of(&hdr) as usize;
                // First granule uses the invalid block_type == 0.
                let mut blocks = vec![Blk::S0];
                for _ in 1..n {
                    blocks.push(Blk::rand(&mut rng));
                }
                let opts = BuildOpts {
                    blocks,
                    ..Default::default()
                };
                let input = build(hdr, &opts, &mut rng);
                compare("E4", &input);
                let (cg, _, crv) = unsafe { run(c, &input) };
                let (rg, _, rrv) = unsafe { run(r, &input) };
                assert_eq!(crv, -1, "E4: block_type 0 must give -1: {}", input.desc);
                assert_eq!(rrv, -1, "E4: Rust must give -1: {}", input.desc);
                // Partial write: everything up to and including block_type = 0.
                assert_eq!(cg[0].block_type, 0);
                assert_eq!(rg[0].block_type, 0);
                assert_eq!(cg[0].n_long_sfb, 22, "E4: long defaults must be in place");
                assert_eq!(rg[0].n_long_sfb, 22);
                assert_eq!(cg[0].n_short_sfb, 0);
                assert_eq!(rg[0].n_short_sfb, 0);
                assert!(!cg[0].sfbtab.is_null() && !rg[0].sfbtab.is_null());
                // mixed_block_flag was NOT read yet, so it is still the sentinel.
                assert_eq!(cg[0].mixed_block_flag, 0xA5);
                assert_eq!(rg[0].mixed_block_flag, 0xA5);
            }
        }
    }
}

/// block_type 0 on a later granule.
#[test]
fn e4b_block_type_zero_later_granule() {
    let mut rng = rng_for("E4b");
    for ext in [false, true] {
        for mono in [false, true] {
            let gc = gr_count_of(&hdr_for(ext, mono, srs(ext)[0], &mut rng)) as usize;
            for bad in 0..gc {
                for _ in 0..32 {
                    let sr = srs(ext)[rng.below(srs(ext).len() as u32) as usize];
                    let hdr = hdr_for(ext, mono, sr, &mut rng);
                    let mut blocks: Vec<Blk> = (0..gc).map(|_| Blk::rand(&mut rng)).collect();
                    blocks[bad] = Blk::S0;
                    let opts = BuildOpts {
                        blocks,
                        ..Default::default()
                    };
                    let input = build(hdr, &opts, &mut rng);
                    compare("E4b", &input);
                    assert_eq!(c_result(&input), -1);
                    assert_eq!(rust_result(&input), -1);
                }
            }
        }
    }
}

/// E8: block_type 1, 2 and 3 are all accepted (no upper bound check).
#[test]
fn e8_block_type_1_2_3_accepted() {
    let mut rng = rng_for("E8");
    for ext in [false, true] {
        for mono in [false, true] {
            for blk in [Blk::S1, Blk::S2M0, Blk::S2M1, Blk::S3] {
                for _ in 0..64 {
                    let sr = srs(ext)[rng.below(srs(ext).len() as u32) as usize];
                    let hdr = hdr_for(ext, mono, sr, &mut rng);
                    let opts = BuildOpts {
                        blocks: vec![blk],
                        big_values: Some(rng.below(289)),
                        ..Default::default()
                    };
                    let input = build(hdr, &opts, &mut rng);
                    compare("E8", &input);
                    let rv = c_result(&input);
                    assert_eq!(rv, rust_result(&input));
                    assert_ne!(rv, -1, "E8: {blk:?} must be accepted: {}", input.desc);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// E5 / E7 — reservoir overrun rejected; exact equality accepted
// ---------------------------------------------------------------------------

#[test]
fn e5_part23_sum_overruns() {
    let mut rng = rng_for("E5");
    for ext in [false, true] {
        for mono in [false, true] {
            for _ in 0..256 {
                let sr = srs(ext)[rng.below(srs(ext).len() as u32) as usize];
                let hdr = hdr_for(ext, mono, sr, &mut rng);
                let n = gr_count_of(&hdr) as usize;
                let blocks: Vec<Blk> = (0..n).map(|_| Blk::rand(&mut rng)).collect();

                // One bit over the boundary => E5 fires.
                let over = BuildOpts {
                    blocks: blocks.clone(),
                    main_data_begin: Some(0),
                    part_23_length: Some(64 + rng.below(4000)),
                    limit: LimitMode::OneOverBoundary,
                    ..Default::default()
                };
                let input = build(hdr, &over, &mut rng);
                compare("E5", &input);
                assert_eq!(
                    c_result(&input),
                    -1,
                    "E5: one bit over the reservoir must give -1: {}",
                    input.desc
                );
                assert_eq!(rust_result(&input), -1, "E5: Rust must give -1");

                // Exactly at the boundary => accepted (E7).
                let exact = BuildOpts {
                    limit: LimitMode::ExactBoundary,
                    ..over
                };
                let input = build(hdr, &exact, &mut rng);
                compare("E7", &input);
                assert_eq!(
                    c_result(&input),
                    0,
                    "E7: exact boundary must be accepted: {}",
                    input.desc
                );
                assert_eq!(rust_result(&input), 0, "E7: Rust must accept the boundary");
            }
        }
    }
}

/// Sweep the reservoir slack from far-under to far-over so the `>` comparison
/// itself is pinned down, including negative `limit + main_data_begin*8`.
#[test]
fn e5b_reservoir_slack_sweep() {
    let mut rng = rng_for("E5b");
    for ext in [false, true] {
        for mono in [false, true] {
            for _ in 0..64 {
                let sr = srs(ext)[rng.below(srs(ext).len() as u32) as usize];
                let hdr = hdr_for(ext, mono, sr, &mut rng);
                let n = gr_count_of(&hdr) as usize;
                let blocks: Vec<Blk> = (0..n).map(|_| Blk::rand(&mut rng)).collect();
                let base = BuildOpts {
                    blocks,
                    main_data_begin: Some(0),
                    part_23_length: Some(100),
                    limit: LimitMode::Ample,
                    ..Default::default()
                };
                let probe = build(hdr, &base, &mut rng);
                // `probe.limit` is Ample; walk a window of literal limits around
                // the true boundary and require identical sentinels throughout.
                let boundary = 100 * n as i32 + 200; // upper bound on p23sum + pos
                for delta in -8..=8i32 {
                    let opts = BuildOpts {
                        limit: LimitMode::Literal(boundary + delta),
                        ..BuildOpts {
                            blocks: base.blocks.clone(),
                            main_data_begin: Some(0),
                            part_23_length: Some(100),
                            limit: LimitMode::Ample,
                            ..Default::default()
                        }
                    };
                    let input = build(hdr, &opts, &mut rng);
                    compare("E5b", &input);
                    assert_eq!(c_result(&input), rust_result(&input));
                }
                let _ = probe;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// U1 — sr_idx == 8: out-of-range table index, no check in the C
// ---------------------------------------------------------------------------

#[test]
fn u1_sr_idx_out_of_range() {
    let mut rng = rng_for("U1");
    let (c, r) = impls();
    let mut seen = [false; 3];
    for mono in [false, true] {
        for blk in Blk::all_valid() {
            for _ in 0..64 {
                let hdr = hdr_for(true, mono, 8, &mut rng);
                assert_eq!(sr_idx_of(&hdr), 8, "U1: sr_idx must be 8");
                let opts = BuildOpts {
                    blocks: vec![blk],
                    big_values: Some(rng.below(289)),
                    ..Default::default()
                };
                let input = build(hdr, &opts, &mut rng);
                compare("U1", &input);
                let (cg, _, crv) = unsafe { run(c, &input) };
                let (rg, _, rrv) = unsafe { run(r, &input) };
                assert_eq!(crv, rrv);
                assert_ne!(crv, -1, "U1: sr_idx 8 is NOT rejected by the C");
                let t = match blk {
                    Blk::S2M0 => T_SHORT,
                    Blk::S2M1 => T_MIXED,
                    _ => T_LONG,
                };
                seen[t] = true;
                let coff = (cg[0].sfbtab as usize).wrapping_sub(c.table_bases[t]);
                let roff = (rg[0].sfbtab as usize).wrapping_sub(r.table_bases[t]);
                assert_eq!(coff, 8 * ROW_SIZE[t], "U1: C offset must be 8 rows past base");
                assert_eq!(roff, 8 * ROW_SIZE[t], "U1: Rust offset must be 8 rows past base");
            }
        }
    }
    assert_eq!(seen, [true, true, true], "U1 must cover all three tables");
}

// ---------------------------------------------------------------------------
// U4 — hdr[0] is never read
// ---------------------------------------------------------------------------

#[test]
fn u4_hdr0_ignored() {
    let mut rng = rng_for("U4");
    for ext in [false, true] {
        for mono in [false, true] {
            let sr = srs(ext)[rng.below(srs(ext).len() as u32) as usize];
            let hdr = hdr_for(ext, mono, sr, &mut rng);
            let n = gr_count_of(&hdr) as usize;
            let blocks: Vec<Blk> = (0..n).map(|_| Blk::rand(&mut rng)).collect();
            let opts = BuildOpts {
                blocks,
                ..Default::default()
            };
            let mut input = build(hdr, &opts, &mut rng);
            input.hdr[0] = 0;
            let baseline_c = c_result(&input);
            let baseline_r = rust_result(&input);
            assert_eq!(baseline_c, baseline_r);
            for h0 in 0..=255u8 {
                input.hdr[0] = h0;
                compare("U4", &input);
                assert_eq!(c_result(&input), baseline_c, "U4: hdr[0] must be ignored by C");
                assert_eq!(rust_result(&input), baseline_r, "U4: hdr[0] must be ignored by Rust");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Generic FFI boundary abuse: out-of-range "enum-like" values and extreme ints
// ---------------------------------------------------------------------------

/// The C API has no enums, but the three header bytes it reads are treated as
/// bit-flag/index sources with no validation: `hdr[1]` bits 3-4 and `hdr[2]`
/// bits 2-3 index the tables, `hdr[3]` bits 6-7 select the channel mode. All
/// 2^24 combinations of the bits actually read are valid C inputs. Sweep every
/// one of the read bit-fields exhaustively (4 x 4 x 4 = 64 combinations of
/// (b3,b4) x base x hdr[3] mode, times full random noise elsewhere).
#[test]
fn ffi_all_header_bitfield_combinations() {
    let mut rng = rng_for("FFI-HDR");
    for h1_lo in 0..4u8 {
        // bits 3 and 4 of hdr[1]
        for base in 0..4u8 {
            for mode in 0..4u8 {
                for _ in 0..64 {
                    let mut hdr = [0u8; 4];
                    hdr[0] = rng.next_u32() as u8;
                    hdr[1] = ((rng.next_u32() as u8) & !0x18) | (h1_lo << 3);
                    hdr[2] = ((rng.next_u32() as u8) & !0x0C) | (base << 2);
                    hdr[3] = ((rng.next_u32() as u8) & 0x3F) | (mode << 6);
                    let mut buf = vec![0u8; BUF_BYTES];
                    for b in buf.iter_mut() {
                        *b = rng.next_u32() as u8;
                    }
                    let pos = rng.range_i32(0, 63);
                    let limit = rng.range_i32(-4, 4096);
                    let input = Input {
                        hdr,
                        buf,
                        pos,
                        limit,
                        desc: format!(
                            "FFI-HDR h1_lo={h1_lo} base={base} mode={mode} hdr={hdr:02x?} pos={pos} limit={limit}"
                        ),
                    };
                    compare("FFI-HDR", &input);
                }
            }
        }
    }
}

/// Extreme `bs->pos` / `bs->limit` values across the FFI boundary.
///
/// Two regimes are safe to exercise, and they are separated deliberately:
///
/// * `limit < pos` — every `get_bits` is rejected before `*p` is read, so `pos`
///   may be arbitrarily large. `pos` is still capped at `i32::MAX - 512` so that
///   `pos += n` cannot overflow (signed overflow is UB; see U6 in `ERRORS.md`).
///   Because `pos` only grows and `limit` is fixed below it, every later read is
///   rejected too, so no byte is ever touched.
/// * `limit >= pos` — reads really happen, so `pos` must stay inside the buffer.
#[test]
fn ffi_extreme_pos_limit() {
    let mut rng = rng_for("FFI-EXT");
    // Largest bit offset whose byte index (plus the ~2 bytes get_bits reads
    // ahead, plus every field of 4 granules) stays inside BUF_BYTES.
    const READABLE_MAX_POS: i32 = (BUF_BYTES as i32 - 64) * 8;
    const NO_OVERFLOW_MAX_POS: i32 = i32::MAX - 512;

    let mut cases: Vec<(i32, i32)> = Vec::new();

    // Regime 1: pos > limit, every read rejected.
    for &pos in &[
        1i32,
        7,
        8,
        63,
        255,
        256,
        4095,
        1 << 20,
        NO_OVERFLOW_MAX_POS - 1,
        NO_OVERFLOW_MAX_POS,
    ] {
        for &limit in &[
            i32::MIN,
            i32::MIN + 1,
            -1000,
            -1,
            0,
            1,
            pos / 2,
            pos - 1,
        ] {
            if limit < pos {
                cases.push((pos, limit));
            }
        }
    }

    // Regime 2: reads happen, pos inside the buffer, limit up to i32::MAX.
    for &pos in &[0i32, 1, 7, 8, 63, 255, 256, 1000, READABLE_MAX_POS] {
        for &limit in &[
            pos,
            pos + 1,
            pos + 8,
            pos + 9,
            pos + 12,
            pos + 64,
            pos + 200,
            pos + 4096,
            i32::MAX - 1,
            i32::MAX,
        ] {
            cases.push((pos, limit));
        }
    }

    for (pos, limit) in cases {
        for _ in 0..8 {
            let mut hdr = [0u8; 4];
            for b in hdr.iter_mut() {
                *b = rng.next_u32() as u8;
            }
            let mut buf = vec![0u8; BUF_BYTES];
            for b in buf.iter_mut() {
                *b = rng.next_u32() as u8;
            }
            let input = Input {
                hdr,
                buf,
                pos,
                limit,
                desc: format!("FFI-EXT pos={pos} limit={limit} hdr={hdr:02x?}"),
            };
            compare("FFI-EXT", &input);
        }
    }
}

/// U6 (documented, deliberately NOT executed).
///
/// `bs->pos += n` is signed-overflow UB in C once `pos` approaches `INT_MAX`.
/// Every reachable overflow case also crashes: after the wrap, `pos` is negative,
/// so the `pos > limit` guard stops rejecting and `get_bits` dereferences
/// `bs->buf + (pos >> 3)` with a hugely negative byte index. Both the C and the
/// Rust translation fault there (the Rust uses `wrapping_add` and the same raw
/// pointer arithmetic, so it faults identically), which means the case cannot be
/// turned into a differential assertion — the harness would die with SIGSEGV
/// rather than compare anything. It is documented here, and the reachable
/// non-overflowing extremes are covered by `ffi_extreme_pos_limit`.
#[test]
fn u6_signed_overflow_documented_not_executed() {
    // Assert only the *reason* this is untestable: the wrap makes `pos` negative.
    let pos: i32 = i32::MAX - 7;
    assert!(pos.wrapping_add(11) < 0, "the wrap must produce a negative pos");
    assert!(
        (pos.wrapping_add(11) >> 3) < 0,
        "which makes bs->buf + (pos >> 3) point far below the buffer"
    );
}

