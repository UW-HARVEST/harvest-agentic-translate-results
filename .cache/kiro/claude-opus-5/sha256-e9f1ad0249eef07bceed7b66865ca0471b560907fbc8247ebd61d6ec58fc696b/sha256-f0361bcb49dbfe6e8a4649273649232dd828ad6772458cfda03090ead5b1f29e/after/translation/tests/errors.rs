//! Phase C — error-path differential tests, one per `ERRORS.md` row (E1..E11).
//!
//! Every test constructs the exact rejecting condition, calls BOTH libraries
//! through their `.so` exports, and asserts they return the same code/sentinel
//! (not merely "both failed").

mod harness;
use harness::*;

/// Call one library and return `(ret, bs_after, granules)`.
unsafe fn call(
    f: ReadSideInfoFn,
    buf: &[u8],
    pos: i32,
    limit: i32,
    hdr: &[u8; 4],
) -> (i32, bs_t, [L3_gr_info_t; MAX_GR]) {
    let mut bs = bs_t { buf: buf.as_ptr(), pos, limit };
    let mut gr = [L3_gr_info_t {
        sfbtab: std::ptr::null(),
        part_23_length: 0xA5A5,
        big_values: 0xA5A5,
        scalefac_compress: 0xA5A5,
        global_gain: 0xA5,
        block_type: 0xA5,
        mixed_block_flag: 0xA5,
        n_long_sfb: 0xA5,
        n_short_sfb: 0xA5,
        table_select: [0xA5; 3],
        region_count: [0xA5; 3],
        subblock_gain: [0xA5; 3],
        preflag: 0xA5,
        scalefac_scale: 0xA5,
        count1_table: 0xA5,
        scfsi: 0xA5,
    }; MAX_GR];
    let ret = unsafe { f(&mut bs, gr.as_mut_ptr(), hdr.as_ptr()) };
    (ret, bs, gr)
}

/// Assert both libraries agree AND that the C produced `want_ret`.
fn expect_ret(p: &Pair, label: &str, buf: &[u8], pos: i32, limit: i32, hdr: &[u8; 4], want: i32) {
    let (cr, cb, _) = unsafe { call(p.c, buf, pos, limit, hdr) };
    let (rr, rb, _) = unsafe { call(p.r, buf, pos, limit, hdr) };
    assert_eq!(cr, rr, "{label}: return diverges (C={cr} Rust={rr})");
    assert_eq!(cb.pos, rb.pos, "{label}: bs.pos diverges");
    assert_eq!(
        cr, want,
        "{label}: C returned {cr}, test expected {want} (ERRORS.md row is wrong)"
    );
}

/// Build a one-configuration side-info stream and return the packed buffer.
fn build(
    mpeg1: bool,
    mono: bool,
    sr: i32,
    choices: &[GrChoice; MAX_GR],
    mdb: u32,
    align: usize,
    rng: &mut Rng,
) -> ([u8; 4], Packed) {
    let hdr = make_hdr(mpeg1, mono, sr, rng).expect("reachable hdr");
    let bw = build_side_info(&hdr, mdb, rng.next_u32(), choices, rng);
    let packed = pack_with_pad(&bw, PAD, align, rng);
    (hdr, packed)
}

fn all(c: GrChoice) -> [GrChoice; MAX_GR] {
    [c; MAX_GR]
}

// ===========================================================================
// E1 — get_bits limit sentinel: returns 0 and STILL advances bs->pos by n
// ===========================================================================

#[test]
fn err_e1_get_bits_limit_sentinel() {
    let p = Pair::load();
    let mut rng = Rng::new(101);
    // limit == pos: the very first get_bits already trips, so every field
    // becomes the 0 sentinel and no byte is dereferenced.
    for (mpeg1, mono) in [(false, false), (false, true), (true, false), (true, true)] {
        let sr = reachable_sr(mpeg1)[0];
        let c = GrChoice { w: false, bt: 0, mb: false, part_23_length: 0xFFF, big_values: 0x1FF, scalefac_compress: Some(0) };
        let (hdr, pk) = build(mpeg1, mono, sr, &all(c), 511, 3, &mut rng);
        let label = format!("E1 limit==pos mpeg1={mpeg1} mono={mono}");

        let (cr, cb, cg) = unsafe { call(p.c, &pk.buf, pk.pos, pk.pos, &hdr) };
        let (rr, rb, rg) = unsafe { call(p.r, &pk.buf, pk.pos, pk.pos, &hdr) };
        assert_eq!(cr, rr, "{label}: return diverges");
        assert_eq!(cb.pos, rb.pos, "{label}: bs.pos diverges");
        for i in 0..MAX_GR {
            assert_eq!(dump(&cg[i]), dump(&rg[i]), "{label}: granule {i} diverges");
        }
        // The C semantics we are pinning down: sentinel 0 for every field,
        // pos advanced past the limit (NOT clamped), and -1 from the final check.
        assert_eq!(cr, -1, "{label}: C should reject via the final bounds check");
        assert!(cb.pos > cb.limit, "{label}: pos must be advanced past limit, got {}", cb.pos);
        let gr_count = hdr_gr_count(&hdr) as usize;
        for i in 0..gr_count {
            assert_eq!(cg[i].part_23_length, 0, "{label}: granule {i} should be all-sentinel");
            assert_eq!(cg[i].big_values, 0);
            assert_eq!(cg[i].global_gain, 0);
            assert_eq!(cg[i].block_type, 0);
        }
    }
}

/// E1 again, but with the limit placed so that it trips at EVERY individual
/// field boundary in turn — one distinct `get_bits` call rejected per case.
#[test]
fn err_e1_limit_sweep_every_field() {
    let p = Pair::load();
    let mut rng = Rng::new(102);
    let mut n = 0;
    for (mpeg1, mono) in [(false, false), (false, true), (true, false), (true, true)] {
        for (w, bt, mb) in [(false, 0u32, false), (true, 1, false), (true, 2, false), (true, 2, true), (true, 3, false)] {
            let sr = reachable_sr(mpeg1)[1];
            let c = GrChoice { w, bt, mb, part_23_length: rng.below(4096), big_values: rng.below(289), scalefac_compress: None };
            let (hdr, pk) = build(mpeg1, mono, sr, &all(c), rng.below(256), 5, &mut rng);
            for k in 0..=pk.nbits as i32 {
                diff(&p, &Case {
                    label: format!("E1 sweep mpeg1={mpeg1} mono={mono} w={w} bt={bt} mb={mb} k={k}"),
                    buf: &pk.buf,
                    pos: pk.pos,
                    limit: pk.pos + k,
                    hdr,
                    compare_sfb_contents: sr <= 7,
                });
                n += 1;
            }
        }
    }
    eprintln!("E1: {n} per-field truncation points OK");
}

// ===========================================================================
// E2 — big_values > 288  =>  return -1
// ===========================================================================

#[test]
fn err_e2_big_values_over_288() {
    let p = Pair::load();
    let mut rng = Rng::new(201);
    for bv in [289u32, 290, 300, 400, 511] {
        for (mpeg1, mono) in [(false, false), (false, true), (true, false), (true, true)] {
            let sr = reachable_sr(mpeg1)[2];
            let c = GrChoice { w: rng.bool(), bt: 1 + rng.below(3), mb: rng.bool(), part_23_length: rng.below(4096), big_values: bv, scalefac_compress: None };
            let (hdr, pk) = build(mpeg1, mono, sr, &all(c), rng.below(256), 0, &mut rng);
            let label = format!("E2 bv={bv} mpeg1={mpeg1} mono={mono}");
            // ample limit so the rejection is due to big_values, not the limit
            let limit = (pk.buf.len() * 8) as i32;
            expect_ret(&p, &label, &pk.buf, pk.pos, limit, &hdr, -1);
            diff(&p, &Case { label, buf: &pk.buf, pos: pk.pos, limit, hdr, compare_sfb_contents: sr <= 7 });
        }
    }
}

#[test]
fn err_e2_boundary_288_vs_289() {
    let p = Pair::load();
    let mut rng = Rng::new(202);
    for (mpeg1, mono) in [(false, false), (false, true), (true, false), (true, true)] {
        let sr = reachable_sr(mpeg1)[0];
        for (bv, want) in [(287u32, 0i32), (288, 0), (289, -1)] {
            // part_23_length 0 and main_data_begin max so the FINAL check passes
            // and the only possible rejection is E2.
            let c = GrChoice { w: false, bt: 0, mb: false, part_23_length: 0, big_values: bv, scalefac_compress: Some(0) };
            let mdb = if mpeg1 { 511 } else { 255 };
            let (hdr, pk) = build(mpeg1, mono, sr, &all(c), mdb, 0, &mut rng);
            let limit = pk.pos + pk.nbits as i32;
            let label = format!("E2 boundary bv={bv} mpeg1={mpeg1} mono={mono}");
            let (cr, _, _) = unsafe { call(p.c, &pk.buf, pk.pos, limit, &hdr) };
            // want==0 means "not the E2 rejection"; the C may still return a
            // non-negative main_data_begin.
            if want == -1 {
                assert_eq!(cr, -1, "{label}: expected E2 rejection");
            } else {
                assert!(cr >= 0, "{label}: expected acceptance, got {cr}");
            }
            diff(&p, &Case { label, buf: &pk.buf, pos: pk.pos, limit, hdr, compare_sfb_contents: sr <= 7 });
        }
    }
}

/// E2 triggered on the SECOND / THIRD / FOURTH granule: earlier granules are
/// fully written, the failing one only partially. Both libraries must agree on
/// exactly how much was written.
#[test]
fn err_e2_second_granule() {
    let p = Pair::load();
    let mut rng = Rng::new(203);
    for (mpeg1, mono) in [(false, false), (true, false), (true, true)] {
        let sr = reachable_sr(mpeg1)[3];
        let hdr = make_hdr(mpeg1, mono, sr, &mut rng).unwrap();
        let gr_count = hdr_gr_count(&hdr) as usize;
        for bad in 0..gr_count {
            let mut choices = [GrChoice::random(&mut rng); MAX_GR];
            for g in 0..MAX_GR {
                let mut c = GrChoice::random(&mut rng);
                c.big_values = if g == bad { 289 + rng.below(223) } else { rng.below(289) };
                choices[g] = c;
            }
            let bw = build_side_info(&hdr, rng.below(256), rng.next_u32(), &choices, &mut rng);
            let pk = pack_with_pad(&bw, PAD, 2, &mut rng);
            let limit = (pk.buf.len() * 8) as i32;
            let label = format!("E2 gr#{bad} of {gr_count} mpeg1={mpeg1} mono={mono}");
            expect_ret(&p, &label, &pk.buf, pk.pos, limit, &hdr, -1);
            diff(&p, &Case { label, buf: &pk.buf, pos: pk.pos, limit, hdr, compare_sfb_contents: sr <= 7 });
        }
    }
}

// ===========================================================================
// E3 — window switching set and block_type == 0  =>  return -1
// ===========================================================================

#[test]
fn err_e3_block_type_zero() {
    let p = Pair::load();
    let mut rng = Rng::new(301);
    for (mpeg1, mono) in [(false, false), (false, true), (true, false), (true, true)] {
        let sr = reachable_sr(mpeg1)[1];
        let hdr = make_hdr(mpeg1, mono, sr, &mut rng).unwrap();
        let gr_count = hdr_gr_count(&hdr) as usize;
        for bad in 0..gr_count {
            let mut choices = [GrChoice::random(&mut rng); MAX_GR];
            for g in 0..MAX_GR {
                let mut c = GrChoice::random(&mut rng);
                c.big_values = rng.below(289);
                c.w = true;
                c.bt = if g == bad { 0 } else { 1 + rng.below(3) };
                choices[g] = c;
            }
            let bw = build_side_info(&hdr, rng.below(256), rng.next_u32(), &choices, &mut rng);
            let pk = pack_with_pad(&bw, PAD, 1, &mut rng);
            let limit = (pk.buf.len() * 8) as i32;
            let label = format!("E3 bt=0 at gr#{bad}/{gr_count} mpeg1={mpeg1} mono={mono}");
            expect_ret(&p, &label, &pk.buf, pk.pos, limit, &hdr, -1);
            diff(&p, &Case { label: label.clone(), buf: &pk.buf, pos: pk.pos, limit, hdr, compare_sfb_contents: sr <= 7 });

            // pin the documented partial-write behaviour
            let (_, _, cg) = unsafe { call(p.c, &pk.buf, pk.pos, limit, &hdr) };
            assert_eq!(cg[bad].block_type, 0, "{label}: block_type must be stored as 0");
            assert_eq!(cg[bad].n_long_sfb, 22, "{label}: n_long_sfb was set before the check");
            assert_eq!(cg[bad].mixed_block_flag, 0xA5, "{label}: mixed_block_flag must stay untouched");
        }
    }
}

/// All four `block_type` values (0 rejects, 1/2/3 accept) x both
/// `mixed_block_flag` values — the full 2-bit field, including the value with
/// no valid meaning.
#[test]
fn err_e3_all_block_types() {
    let p = Pair::load();
    let mut rng = Rng::new(302);
    for (mpeg1, mono) in [(false, false), (false, true), (true, false), (true, true)] {
        for sr in reachable_sr(mpeg1) {
            for bt in 0..4u32 {
                for mb in [false, true] {
                    let c = GrChoice { w: true, bt, mb, part_23_length: 0, big_values: 0, scalefac_compress: Some(0) };
                    let mdb = if mpeg1 { 511 } else { 255 };
                    let (hdr, pk) = build(mpeg1, mono, *sr, &all(c), mdb, 4, &mut rng);
                    let limit = (pk.buf.len() * 8) as i32;
                    let label = format!("E3 bt={bt} mb={mb} sr={sr} mpeg1={mpeg1} mono={mono}");
                    if bt == 0 {
                        expect_ret(&p, &label, &pk.buf, pk.pos, limit, &hdr, -1);
                    }
                    diff(&p, &Case { label, buf: &pk.buf, pos: pk.pos, limit, hdr, compare_sfb_contents: *sr <= 7 });
                }
            }
        }
    }
}

// ===========================================================================
// E4 — part_23_sum + pos > limit + main_data_begin*8  =>  return -1
// ===========================================================================

#[test]
fn err_e4_main_data_bounds() {
    let p = Pair::load();
    let mut rng = Rng::new(401);
    for (mpeg1, mono) in [(false, false), (false, true), (true, false), (true, true)] {
        let sr = reachable_sr(mpeg1)[0];
        // huge part_23_length, zero main_data_begin => guaranteed overrun
        let c = GrChoice { w: false, bt: 0, mb: false, part_23_length: 4095, big_values: 0, scalefac_compress: Some(0) };
        let (hdr, pk) = build(mpeg1, mono, sr, &all(c), 0, 0, &mut rng);
        let limit = pk.pos + pk.nbits as i32;
        let label = format!("E4 overrun mpeg1={mpeg1} mono={mono}");
        expect_ret(&p, &label, &pk.buf, pk.pos, limit, &hdr, -1);
        diff(&p, &Case { label, buf: &pk.buf, pos: pk.pos, limit, hdr, compare_sfb_contents: sr <= 7 });
    }
}

/// Sweep `part_23_length` across the whole 12-bit range so the final
/// comparison is crossed from "just inside" to "just outside" one bit at a
/// time. Both libraries must flip from success to -1 at the same value.
#[test]
fn err_e4_bounds_boundary() {
    let p = Pair::load();
    let mut rng = Rng::new(402);
    for (mpeg1, mono) in [(false, false), (false, true), (true, false), (true, true)] {
        let sr = reachable_sr(mpeg1)[0];
        let mdb = if mpeg1 { 100 } else { 100 };
        let mut flips = 0;
        let mut prev: Option<bool> = None;
        for p23 in 0..4096u32 {
            let c = GrChoice { w: false, bt: 0, mb: false, part_23_length: p23, big_values: 0, scalefac_compress: Some(0) };
            let (hdr, pk) = build(mpeg1, mono, sr, &all(c), mdb, 0, &mut rng);
            let limit = pk.pos + pk.nbits as i32;
            let label = format!("E4 boundary p23={p23} mpeg1={mpeg1} mono={mono}");
            let (cr, _, _) = unsafe { call(p.c, &pk.buf, pk.pos, limit, &hdr) };
            let (rr, _, _) = unsafe { call(p.r, &pk.buf, pk.pos, limit, &hdr) };
            assert_eq!(cr, rr, "{label}: return diverges (C={cr} Rust={rr})");
            let rejected = cr == -1;
            if prev.is_some_and(|q| q != rejected) {
                flips += 1;
            }
            prev = Some(rejected);
            diff(&p, &Case { label, buf: &pk.buf, pos: pk.pos, limit, hdr, compare_sfb_contents: sr <= 7 });
        }
        assert!(flips >= 1, "E4 boundary mpeg1={mpeg1} mono={mono}: sweep never crossed the threshold");
    }
    eprintln!("E4: part_23_length threshold crossed identically in all 4 shapes");
}

// ===========================================================================
// E5 — sr_idx == 8: one past the end of every table, unchecked by the C
// ===========================================================================

#[test]
fn err_e5_sr_idx_out_of_range() {
    let p = Pair::load();
    let mut rng = Rng::new(501);
    // sr_idx 8 needs hdr[1] bits 3 and 4 set and (hdr[2]>>2)&3 == 3
    for mono in [false, true] {
        for (w, bt, mb) in [(false, 0u32, false), (true, 1, false), (true, 2, false), (true, 2, true), (true, 3, false)] {
            let c = GrChoice { w, bt, mb, part_23_length: 0, big_values: 0, scalefac_compress: Some(0) };
            let (hdr, pk) = build(true, mono, 8, &all(c), 511, 0, &mut rng);
            assert_eq!(hdr_sr_idx(&hdr), 8);
            let limit = (pk.buf.len() * 8) as i32;
            // No rejection: sr_idx is never range-checked.
            let (cr, _, cg) = unsafe { call(p.c, &pk.buf, pk.pos, limit, &hdr) };
            assert!(cr >= 0, "E5: C unexpectedly rejected sr_idx=8 (ret={cr})");
            assert!(!cg[0].sfbtab.is_null(), "E5: sfbtab must still be written");
            // Everything except the pointed-to bytes must match.
            diff(&p, &Case {
                label: format!("E5 sr_idx=8 mono={mono} w={w} bt={bt} mb={mb}"),
                buf: &pk.buf, pos: pk.pos, limit, hdr,
                compare_sfb_contents: false,
            });
        }
    }
    // and the in-range neighbour, 7, where contents ARE comparable
    for mono in [false, true] {
        let c = GrChoice { w: true, bt: 2, mb: false, part_23_length: 0, big_values: 0, scalefac_compress: Some(0) };
        let (hdr, pk) = build(true, mono, 7, &all(c), 511, 0, &mut rng);
        let limit = (pk.buf.len() * 8) as i32;
        diff(&p, &Case { label: format!("E5 sr_idx=7 mono={mono}"), buf: &pk.buf, pos: pk.pos, limit, hdr, compare_sfb_contents: true });
    }
}

// ===========================================================================
// E6 — limit == 0 and limit < 0
// ===========================================================================

#[test]
fn err_e6_zero_and_negative_limit() {
    let p = Pair::load();
    let mut rng = Rng::new(601);
    for limit in [0i32, -1, -8, -1000, i32::MIN, i32::MIN + 1] {
        for (mpeg1, mono) in [(false, false), (false, true), (true, false), (true, true)] {
            let sr = reachable_sr(mpeg1)[0];
            let c = GrChoice::random(&mut rng);
            let (hdr, pk) = build(mpeg1, mono, sr, &all(c), 7, 0, &mut rng);
            let label = format!("E6 limit={limit} mpeg1={mpeg1} mono={mono}");
            // No byte is dereferenced; every field is the 0 sentinel.
            diff(&p, &Case { label: label.clone(), buf: &pk.buf, pos: pk.pos, limit, hdr, compare_sfb_contents: sr <= 7 });
            let (cr, _, _) = unsafe { call(p.c, &pk.buf, pk.pos, limit, &hdr) };
            let (rr, _, _) = unsafe { call(p.r, &pk.buf, pk.pos, limit, &hdr) };
            assert_eq!(cr, rr, "{label}: return diverges");
        }
    }
}

// ===========================================================================
// E7 — pos already past limit on entry; oversized pos; oversized limit
// ===========================================================================

#[test]
fn err_e7_pos_past_limit_on_entry() {
    let p = Pair::load();
    let mut rng = Rng::new(701);
    for (mpeg1, mono) in [(false, false), (false, true), (true, false), (true, true)] {
        let sr = reachable_sr(mpeg1)[0];
        let c = GrChoice::random(&mut rng);
        let (hdr, pk) = build(mpeg1, mono, sr, &all(c), 3, 0, &mut rng);
        let nb = (pk.buf.len() * 8) as i32;
        // pos > limit: `get_bits` trips before any dereference, so even absurd
        // pos values are safe -- as long as `pos += n` does not overflow `int`
        // (see err_e7_pos_overflow_faults for that case, which really does
        // fault in the C).
        for (pos, limit) in [
            (pk.pos, pk.pos - 1),
            (nb, nb - 1),
            (1 << 28, 1 << 10),
            (1 << 30, 0),
            (i32::MAX / 2, 0),
            (i32::MAX - 4096, 0),
        ] {
            let label = format!("E7 pos={pos} limit={limit} mpeg1={mpeg1} mono={mono}");
            diff(&p, &Case { label, buf: &pk.buf, pos, limit, hdr, compare_sfb_contents: false });
        }
    }
}

const POS_ENV: &str = "SIDEINFO_POS_OVERFLOW_TEST";

/// `bs->pos` so close to `INT_MAX` that `bs->pos += n` overflows. The guard
/// `(bs->pos += n) > bs->limit` then compares a *negative* value and lets the
/// read through, so the C dereferences `buf + (pos >> 3)` far out of bounds and
/// faults. Verified out-of-process, comparing termination signals.
#[test]
fn err_e7_pos_overflow_faults() {
    use std::os::unix::process::ExitStatusExt;

    if let Ok(spec) = std::env::var(POS_ENV) {
        // ---- child mode ----
        let p = Pair::load();
        let (which, pos) = spec.split_once(':').unwrap();
        let f = if which == "c" { p.c } else { p.r };
        let pos: i32 = pos.parse().unwrap();
        let mut buf = vec![0u8; 4096];
        let hdr = [0u8, 0x08, 0, 0];
        let mut bs = bs_t { buf: buf.as_mut_ptr(), pos, limit: 0 };
        let mut gr = [unsafe { std::mem::zeroed::<L3_gr_info_t>() }; MAX_GR];
        let ret = unsafe { f(&mut bs, gr.as_mut_ptr(), hdr.as_ptr()) };
        println!("NOFAULT ret={ret} pos={}", bs.pos);
        std::process::exit(0);
    }

    // ---- parent mode ----
    let exe = std::env::current_exe().expect("current_exe");
    for pos in [i32::MAX, i32::MAX - 1, i32::MAX - 8, i32::MAX - 64] {
        let mut out = Vec::new();
        for which in ["c", "r"] {
            let o = std::process::Command::new(&exe)
                .args(["--exact", "err_e7_pos_overflow_faults", "--nocapture"])
                .env(POS_ENV, format!("{which}:{pos}"))
                .output()
                .expect("spawn child");
            let stdout = String::from_utf8_lossy(&o.stdout).to_string();
            let nofault = stdout.lines().find(|l| l.starts_with("NOFAULT")).map(str::to_string);
            out.push((o.status.signal(), nofault));
        }
        assert_eq!(
            out[0], out[1],
            "E7 pos={pos}: C and Rust disagree (C={:?} Rust={:?})",
            out[0], out[1]
        );
        eprintln!("E7 pos overflow pos={pos}: C={:?} Rust={:?}", out[0], out[1]);
    }
}

/// Oversized limit: `bs->limit` so large that `get_bits` never trips. The reads
/// themselves stay near `pos`, but the final check `limit + main_data_begin*8`
/// overflows `int` — signed overflow in the C, which the Rust reproduces with
/// wrapping arithmetic. Both must agree.
#[test]
fn err_e7_oversized_limit_overflow() {
    let p = Pair::load();
    let mut rng = Rng::new(702);
    for (mpeg1, mono) in [(false, false), (false, true), (true, false), (true, true)] {
        let sr = reachable_sr(mpeg1)[0];
        for mdb in [0u32, 1, 255, 511] {
            let c = GrChoice { w: false, bt: 0, mb: false, part_23_length: 4095, big_values: 0, scalefac_compress: Some(0) };
            let (hdr, pk) = build(mpeg1, mono, sr, &all(c), mdb, 0, &mut rng);
            for limit in [i32::MAX, i32::MAX - 1, i32::MAX - 4096, 0x7FFF_0000] {
                let label = format!("E7 oversized limit={limit} mdb={mdb} mpeg1={mpeg1} mono={mono}");
                diff(&p, &Case { label, buf: &pk.buf, pos: pk.pos, limit, hdr, compare_sfb_contents: sr <= 7 });
            }
        }
    }
}

// ===========================================================================
// E8 — negative pos: out-of-bounds pointer, unchecked. Both libraries get the
// SAME buffer pointer, so the bytes read are identical and results must match.
// ===========================================================================

#[test]
fn err_e8_negative_pos() {
    let p = Pair::load();
    let mut rng = Rng::new(801);
    // 256 bytes of real, mapped slack in front of the pointer we hand to the
    // libraries, so `buf + (pos >> 3)` for pos down to -2048 bits stays inside
    // the same allocation. Both libraries get the SAME pointer, so the
    // out-of-bounds bytes they read are identical.
    const SLACK: usize = 256;
    for (mpeg1, mono) in [(false, false), (false, true), (true, false), (true, true)] {
        let sr = reachable_sr(mpeg1)[0];
        let c = GrChoice::random(&mut rng);
        let (hdr, pk) = build(mpeg1, mono, sr, &all(c), 3, 0, &mut rng);
        let mut full = vec![0u8; SLACK];
        rng.fill(&mut full);
        full.extend_from_slice(&pk.buf);
        let view = &full[SLACK..];
        for off in 1..=(SLACK as i32 * 8) {
            let pos = -off;
            let label = format!("E8 pos={pos} mpeg1={mpeg1} mono={mono}");
            diff(&p, &Case {
                buf: view,
                pos,
                // limit generous enough that reads actually happen
                limit: (view.len() * 8) as i32,
                hdr,
                label,
                compare_sfb_contents: sr <= 7,
            });
        }
    }
    eprintln!("E8: negative pos (all {} bit offsets x 4 shapes) OK", SLACK * 8);
}

// ===========================================================================
// E9 — every hdr byte value is a legal input (no validity check, no enum).
//      Exhaustive over hdr[1] x hdr[2] x hdr[3] = 2^24.
// ===========================================================================

#[test]
fn err_e9_exhaustive_hdr_bytes() {
    let p = Pair::load();
    let mut rng = Rng::new(901);
    let mut buf = vec![0u8; 1024];
    rng.fill(&mut buf);
    let pos = 64 * 8 + 3;
    let limit = (buf.len() * 8) as i32;
    // stride 1 = fully exhaustive; override for a quick run
    let stride: u32 = std::env::var("HDR_STRIDE").ok().and_then(|v| v.parse().ok()).unwrap_or(1);

    let mut seen_sr = [0u64; 9];
    let mut checked = 0u64;
    let mut mism = 0u64;
    let mut h1 = 0u32;
    while h1 < 256 {
        let mut h2 = 0u32;
        while h2 < 256 {
            let mut h3 = 0u32;
            while h3 < 256 {
                let hdr = [0u8, h1 as u8, h2 as u8, h3 as u8];
                let (cr, cb, cg) = unsafe { call(p.c, &buf, pos, limit, &hdr) };
                let (rr, rb, rg) = unsafe { call(p.r, &buf, pos, limit, &hdr) };
                if cr != rr || cb.pos != rb.pos || cb.limit != rb.limit {
                    mism += 1;
                    assert_eq!(
                        (cr, cb.pos), (rr, rb.pos),
                        "E9 hdr={hdr:02X?}: ret/pos diverge"
                    );
                }
                for i in 0..MAX_GR {
                    let cb2: [u8; 32] = unsafe { std::mem::transmute(cg[i]) };
                    let rb2: [u8; 32] = unsafe { std::mem::transmute(rg[i]) };
                    // skip the 8-byte sfbtab pointer, which differs by design
                    if cb2[8..] != rb2[8..] {
                        mism += 1;
                        assert_eq!(
                            dump(&cg[i]), dump(&rg[i]),
                            "E9 hdr={hdr:02X?}: granule {i} diverges"
                        );
                    }
                    // sfbtab written-ness must agree
                    assert_eq!(
                        cg[i].sfbtab.is_null(), rg[i].sfbtab.is_null(),
                        "E9 hdr={hdr:02X?}: granule {i} sfbtab written-ness diverges"
                    );
                }
                let sr = hdr_sr_idx(&hdr);
                seen_sr[sr as usize] += 1;
                checked += 1;
                h3 += stride;
            }
            h2 += stride;
        }
        h1 += stride;
    }
    eprintln!("E9: {checked} hdr combinations checked (stride={stride}), mismatches={mism}");
    eprintln!("E9: sr_idx histogram = {seen_sr:?}");
    assert_eq!(mism, 0);
    if stride == 1 {
        for (sr, n) in seen_sr.iter().enumerate() {
            assert!(*n > 0, "E9: sr_idx {sr} never produced by any hdr value");
        }
    }
}

// ===========================================================================
// E10 — granule-count boundary: the number of granules written must match.
//       (`diff` compares all MAX_GR granules including the untouched tail.)
// ===========================================================================

#[test]
fn err_e10_granule_count_written() {
    let p = Pair::load();
    let mut rng = Rng::new(1001);
    for (mpeg1, mono, expect) in [
        (false, true, 1),
        (false, false, 2),
        (true, true, 2),
        (true, false, 4),
    ] {
        let sr = reachable_sr(mpeg1)[0];
        let c = GrChoice { w: false, bt: 0, mb: false, part_23_length: 0, big_values: 0, scalefac_compress: Some(0) };
        let (hdr, pk) = build(mpeg1, mono, sr, &all(c), 255, 0, &mut rng);
        assert_eq!(hdr_gr_count(&hdr), expect, "granule count model wrong");
        let limit = (pk.buf.len() * 8) as i32;
        let (_, _, cg) = unsafe { call(p.c, &pk.buf, pk.pos, limit, &hdr) };
        let (_, _, rg) = unsafe { call(p.r, &pk.buf, pk.pos, limit, &hdr) };
        for i in 0..MAX_GR {
            let touched_c = cg[i].global_gain != 0xA5 || cg[i].part_23_length != 0xA5A5;
            let touched_r = rg[i].global_gain != 0xA5 || rg[i].part_23_length != 0xA5A5;
            assert_eq!(touched_c, touched_r, "granule {i} written-ness diverges (mpeg1={mpeg1} mono={mono})");
            assert_eq!(
                touched_c, i < expect as usize,
                "granule {i}: C wrote it = {touched_c}, expected {} (mpeg1={mpeg1} mono={mono})",
                i < expect as usize
            );
        }
        diff(&p, &Case { label: format!("E10 mpeg1={mpeg1} mono={mono}"), buf: &pk.buf, pos: pk.pos, limit, hdr, compare_sfb_contents: sr <= 7 });
    }
}

// ===========================================================================
// E11 — NULL pointers are unchecked: both libraries must fault identically.
//       Run out-of-process so the harness survives.
// ===========================================================================

const NULL_ENV: &str = "SIDEINFO_NULL_TEST";

#[test]
fn err_e11_null_pointers() {
    use std::os::unix::process::ExitStatusExt;

    if let Ok(spec) = std::env::var(NULL_ENV) {
        // ---- child mode ----
        let p = Pair::load();
        let (which, arg) = spec.split_once(':').unwrap();
        let f = if which == "c" { p.c } else { p.r };
        let mut buf = [0u8; 64];
        let hdr = [0u8; 4];
        let mut bs = bs_t { buf: buf.as_mut_ptr(), pos: 0, limit: 512 };
        let mut gr = [unsafe { std::mem::zeroed::<L3_gr_info_t>() }; MAX_GR];
        let ret = unsafe {
            match arg {
                "bs" => f(std::ptr::null_mut(), gr.as_mut_ptr(), hdr.as_ptr()),
                "gr" => f(&mut bs, std::ptr::null_mut(), hdr.as_ptr()),
                "hdr" => f(&mut bs, gr.as_mut_ptr(), std::ptr::null()),
                _ => f(std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null()),
            }
        };
        // If we get here the library did not fault: report the return value.
        println!("NOFAULT {ret}");
        std::process::exit(0);
    }

    // ---- parent mode ----
    // The artifact under verification is the RELEASE cdylib, which faults with
    // SIGSEGV exactly like the C. A *debug* cdylib is compiled with
    // -C debug-assertions, which makes rustc insert an explicit null-pointer
    // check that aborts (SIGABRT) instead. That is a build-profile artifact, not
    // a translation divergence, so when the harness is pointed at a debug .so we
    // only require that both libraries fault.
    let debug_so = rust_so_path().to_string_lossy().contains("/debug/");
    let exe = std::env::current_exe().expect("current_exe");
    for arg in ["bs", "gr", "hdr", "all"] {
        let mut out = Vec::new();
        for which in ["c", "r"] {
            let o = std::process::Command::new(&exe)
                .args(["--exact", "err_e11_null_pointers", "--nocapture"])
                .env(NULL_ENV, format!("{which}:{arg}"))
                .output()
                .expect("spawn child");
            let stdout = String::from_utf8_lossy(&o.stdout).to_string();
            let nofault = stdout
                .lines()
                .find(|l| l.starts_with("NOFAULT"))
                .map(|s| s.to_string());
            out.push((o.status.signal(), o.status.code(), nofault));
        }
        // Both must fault (no silent success on either side).
        assert!(
            out[0].0.is_some() && out[1].0.is_some(),
            "E11 null {arg}: one side did not fault (C={:?} Rust={:?})",
            out[0],
            out[1]
        );
        assert_eq!(
            out[0].2, out[1].2,
            "E11 null {arg}: non-faulting return diverges (C={:?} Rust={:?})",
            out[0], out[1]
        );
        if debug_so {
            eprintln!(
                "E11 null {arg} (debug .so): C={:?} Rust={:?} -- both faulted; \
                 exact-signal check applies to the release cdylib only",
                out[0], out[1]
            );
        } else {
            assert_eq!(
                out[0].0, out[1].0,
                "E11 null {arg}: termination signal diverges (C={:?} Rust={:?})",
                out[0], out[1]
            );
            eprintln!("E11 null {arg}: C={:?} Rust={:?}", out[0], out[1]);
        }
    }
}

// ===========================================================================
// Generic boundaries every C API has, beyond the table above.
// ===========================================================================

/// Zero-length / degenerate buffer combined with a limit that forbids any read.
#[test]
fn err_generic_zero_length_buffer() {
    let p = Pair::load();
    let buf = [0u8; 1];
    let hdr_set: [[u8; 4]; 4] = [
        [0, 0x00, 0, 0x00], // mpeg2 stereo
        [0, 0x00, 0, 0xC0], // mpeg2 mono
        [0, 0x08, 0, 0x00], // mpeg1 stereo
        [0, 0x08, 0, 0xC0], // mpeg1 mono
    ];
    for hdr in hdr_set {
        for (pos, limit) in [(0, 0), (0, -1), (8, 0), (0, i32::MIN)] {
            diff(&p, &Case {
                label: format!("generic zero-len hdr={hdr:02X?} pos={pos} limit={limit}"),
                buf: &buf, pos, limit, hdr,
                compare_sfb_contents: hdr_sr_idx(&hdr) <= 7,
            });
        }
    }
}

/// Values one step past each documented range: block_type 4 is impossible (2
/// bits), so the closest analogues are `big_values` 289 (E2+1), `sr_idx` 8
/// (E5), `part_23_length` 4096-wrap and `scalefac_compress` 512-wrap. All are
/// verified by construction to wrap the same way in both libraries.
#[test]
fn err_generic_one_past_range() {
    let p = Pair::load();
    let mut rng = Rng::new(1101);
    for (mpeg1, mono) in [(false, false), (true, false)] {
        let sr = reachable_sr(mpeg1)[0];
        // part_23_length is a 12-bit field: 4096 wraps to 0, 4097 to 1
        for p23 in [4095u32, 4096, 4097] {
            // scalefac_compress is 9 bits (mpeg2) / 4 bits (mpeg1)
            for sfc in [0u32, 499, 500, 511, 512, 513] {
                let c = GrChoice { w: false, bt: 0, mb: false, part_23_length: p23, big_values: 289, scalefac_compress: Some(sfc) };
                let (hdr, pk) = build(mpeg1, mono, sr, &all(c), 0, 0, &mut rng);
                let limit = (pk.buf.len() * 8) as i32;
                diff(&p, &Case {
                    label: format!("generic one-past p23={p23} sfc={sfc} mpeg1={mpeg1}"),
                    buf: &pk.buf, pos: pk.pos, limit, hdr,
                    compare_sfb_contents: sr <= 7,
                });
            }
        }
    }
}
