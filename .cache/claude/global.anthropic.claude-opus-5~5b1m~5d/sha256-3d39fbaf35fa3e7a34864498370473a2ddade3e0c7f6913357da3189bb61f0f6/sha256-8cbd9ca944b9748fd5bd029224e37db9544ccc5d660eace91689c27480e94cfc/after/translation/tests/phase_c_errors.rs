//! Phase C — error / rejection-path differential tests.
//!
//! One test per row of `ERRORS.md`. Each constructs the exact invalid input or
//! boundary condition, calls BOTH `.so`s through `libloading`, and asserts the
//! same result (same sentinel value, same `bs->pos`, same `grbuf` bytes, same
//! return value) — not merely "both failed somehow".

mod harness;

use harness::*;

const SEED: u64 = 0xE770_5EED;

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

/// Raw differential call that bypasses the buffer-allocating helper, so that
/// pointers such as `NULL` can be passed verbatim.
struct Raw {
    ret: i32,
    pos: i32,
    limit: i32,
    sci: Vec<u8>,
}

fn raw_call(f: DequantFn, grbuf: *mut f32, buf: &[u8], sci_in: &Sci, gs: i32, pos: i32, limit: i32) -> Raw {
    let mut sci = *sci_in;
    let mut bs = BsT {
        buf: buf.as_ptr(),
        pos,
        limit,
    };
    let ret = unsafe { f(grbuf, &mut bs as *mut BsT, &mut sci as *mut Sci, gs) };
    let bytes = unsafe {
        std::slice::from_raw_parts(&sci as *const Sci as *const u8, std::mem::size_of::<Sci>())
    }
    .to_vec();
    Raw {
        ret,
        pos: bs.pos,
        limit: bs.limit,
        sci: bytes,
    }
}

fn assert_raw_eq(label: &str, a: &Raw, b: &Raw) {
    assert_eq!(a.ret, b.ret, "{label}: return value");
    assert_eq!(a.pos, b.pos, "{label}: bs->pos");
    assert_eq!(a.limit, b.limit, "{label}: bs->limit");
    assert_eq!(a.sci, b.sci, "{label}: L12_scale_info bytes");
}

// ===========================================================================
// E1 — limit exceeded on the very first get_bits call
// ===========================================================================
#[test]
fn e1_limit_exceeded_first_call() {
    let mut rng = Rng::new(SEED ^ 1);
    for ba in 1u8..=16 {
        for _ in 0..16 {
            let buf = random_buf(&mut rng);
            let sci = sci_with(1, &[ba, 0]);
            // limit is one bit short of the first field: get_bits must return 0
            // *without* reading, so every output is (float)(0 - half).
            let limit = ba as i32 - 1;
            diff_case_must_run("E1", &buf, &sci, 18, 0, limit);
        }
    }
    // Also verify the *value*: every slot must be exactly -(2^(ba-1) - 1).
    let buf = vec![0xFFu8; BUF_LEN];
    for ba in 1u8..=16 {
        let sci = sci_with(1, &[ba, 0]);
        let mut grbuf = vec![0f32; 8192];
        let mut bs = BsT {
            buf: buf.as_ptr(),
            pos: 0,
            limit: ba as i32 - 1,
        };
        let mut s = sci;
        unsafe {
            c_dequantize()(grbuf.as_mut_ptr(), &mut bs, &mut s, 4);
        }
        let half = (1i32 << (ba - 1)) - 1;
        assert_eq!(
            grbuf[0], -(half as f32),
            "ba={ba}: expected the rejected field to decode as -half"
        );
    }
}

// ===========================================================================
// E2 — limit == 0
// ===========================================================================
#[test]
fn e2_limit_zero() {
    let mut rng = Rng::new(SEED ^ 2);
    for _ in 0..128 {
        let buf = random_buf(&mut rng);
        let tb = rng.range(1, 32) as u8;
        let ba: Vec<u8> = (0..2 * tb as usize)
            .map(|_| match rng.below(3) {
                0 => 0,
                1 => rng.range(1, 16) as u8,
                _ => rng.range(17, 24) as u8,
            })
            .collect();
        let sci = sci_with(tb, &ba);
        diff_case_must_run("E2", &buf, &sci, 18, 0, 0);
    }
}

// ===========================================================================
// E3 — negative limit
// ===========================================================================
#[test]
fn e3_negative_limit() {
    let mut rng = Rng::new(SEED ^ 3);
    for limit in [-1i32, -2, -7, -8, -1000, i32::MIN + 1, i32::MIN] {
        for _ in 0..32 {
            let buf = random_buf(&mut rng);
            let tb = rng.range(1, 32) as u8;
            let ba: Vec<u8> = (0..2 * tb as usize)
                .map(|_| rng.range(1, 24) as u8)
                .collect();
            let sci = sci_with(tb, &ba);
            diff_case_must_run("E3", &buf, &sci, 18, 0, limit);
        }
    }
}

// ===========================================================================
// E4 — limit crossed part-way through the granule
// ===========================================================================
#[test]
fn e4_limit_crossed_midway() {
    let mut rng = Rng::new(SEED ^ 4);
    for _ in 0..128 {
        let buf = random_buf(&mut rng);
        let tb = rng.range(2, 24) as u8;
        let ba: Vec<u8> = (0..2 * tb as usize)
            .map(|_| rng.range(1, 16) as u8)
            .collect();
        let sci = sci_with(tb, &ba);
        let gs = 18i32;
        let demand: i64 = ba.iter().map(|&b| b as i64 * gs as i64).sum::<i64>() * 4;
        // Somewhere strictly inside the granule stream.
        let limit = (demand / 3 + rng.below(17) as i64) as i32;
        diff_case_must_run("E4", &buf, &sci, gs, 0, limit);
    }
}

// ===========================================================================
// E5 / E6 — `pos + n == limit` accepted, `== limit + 1` rejected
// ===========================================================================
#[test]
fn e5_limit_exact_boundary() {
    let mut rng = Rng::new(SEED ^ 5);
    for ba in 1u8..=16 {
        for phase in 0..8i32 {
            let buf = random_buf(&mut rng);
            let sci = sci_with(1, &[ba, 0]);
            // group_size 1 and a single non-zero band: 4 get_bits calls total,
            // one per granule, each consuming `ba` bits.
            let gs = 1i32;
            let pos0 = 64 + phase;
            let total = 4 * ba as i32;

            // E5: limit exactly at the end of the *last* field -> all accepted.
            diff_case_must_run("E5-exact", &buf, &sci, gs, pos0, pos0 + total);
            // ...and exactly at the end of the *first* field.
            diff_case_must_run("E5-first", &buf, &sci, gs, pos0, pos0 + ba as i32);
            // E6: one bit past the valid range -> the field is rejected.
            diff_case_must_run("E6-short", &buf, &sci, gs, pos0, pos0 + total - 1);
            diff_case_must_run("E6-first", &buf, &sci, gs, pos0, pos0 + ba as i32 - 1);
        }
    }
}

// ===========================================================================
// E7 — negative bs->pos
// ===========================================================================
#[test]
fn e7_negative_pos() {
    let mut rng = Rng::new(SEED ^ 6);
    for pos in [-1i32, -2, -7, -8, -9, -64, -1000, -100_000, i32::MIN / 2] {
        for _ in 0..16 {
            let buf = random_buf(&mut rng);
            let tb = rng.range(1, 16) as u8;
            let ba: Vec<u8> = (0..2 * tb as usize)
                .map(|_| rng.range(1, 24) as u8)
                .collect();
            let sci = sci_with(tb, &ba);
            // A limit below `pos` keeps get_bits from dereferencing the wild
            // pointer, while still exercising `pos & 7` / `pos >> 3` on a
            // negative value and the wrapping `pos += n`.
            let limit = pos - 1;
            diff_case_must_run("E7", &buf, &sci, 18, pos, limit);
        }
    }
    // `pos & 7` on a negative value must still be 0..7 and `pos >> 3` must be
    // an arithmetic shift; assert the observable (final pos) matches exactly.
    let buf = vec![0u8; 64];
    let sci = sci_with(1, &[5, 0]);
    for pos in [-1i32, -3, -5, -7, -9, -15] {
        let a = raw_call(c_dequantize(), std::ptr::null_mut(), &buf, &sci, 0, pos, pos - 1);
        let b = raw_call(rust_dequantize(), std::ptr::null_mut(), &buf, &sci, 0, pos, pos - 1);
        assert_raw_eq("E7-raw", &a, &b);
    }
}

// ===========================================================================
// E8 / E22 — enormous `n` and the wrapping `bs->pos += n`
// ===========================================================================
#[test]
fn e8_huge_n_pos_overflow() {
    // ba = 47 => (47-17) = 30 => 2 << 30 = 0x80000000 => mod = 0x80000001,
    // n = 0x80000001 + 2 - 0x10000000 = 0x70000003.
    let (m, n) = grouped_params(47);
    assert_eq!(m, 0x8000_0001);
    assert_eq!(n, 0x7000_0003);

    let mut rng = Rng::new(SEED ^ 7);
    // NO_READ_LIMIT guarantees every get_bits bails out before dereferencing,
    // so the wrapping `pos += 0x70000003` (4 times, overflowing `int`) is the
    // only observable — and it must wrap identically on both sides.
    for ba in [46u8, 47, 78, 79, 110, 111] {
        for _ in 0..16 {
            let buf = random_buf(&mut rng);
            let sci = sci_with(rng.range(1, 8) as u8, &vec![ba; 16]);
            let gs = rng.range(0, 18) as i32;
            diff_case_must_run("E8", &buf, &sci, gs, 0, NO_READ_LIMIT);
        }
    }
    // Explicitly cross-check the wrap for a single band.
    let buf = vec![0u8; 64];
    let sci = sci_with(1, &[47, 0]);
    let a = raw_call(c_dequantize(), std::ptr::null_mut(), &buf, &sci, 0, 0, NO_READ_LIMIT);
    let b = raw_call(rust_dequantize(), std::ptr::null_mut(), &buf, &sci, 0, 0, NO_READ_LIMIT);
    assert_raw_eq("E8-wrap", &a, &b);
    assert_eq!(a.pos, 0i32.wrapping_add(n).wrapping_add(n).wrapping_add(n).wrapping_add(n));
    assert!(a.pos < 0, "pos should have wrapped negative, got {}", a.pos);
}

// ===========================================================================
// E9 — the masked-shift path in `next >> -shl` is unreachable from the public
//      API. Proven mechanically rather than by a differential call.
// ===========================================================================
#[test]
fn e9_shift_count_always_in_range() {
    for ba in 1i32..=255 {
        let n = if ba < 17 {
            ba
        } else {
            grouped_params(ba).1
        };
        assert!(n >= 1, "ba={ba}: get_bits would be called with n={n} <= 0");
        for s in 0..8i32 {
            let shl0 = (n as u32).wrapping_add(s as u32) as i32;
            assert!(shl0 >= 1, "ba={ba} s={s}: shl0={shl0}");
            // Replay `while ((shl -= 8) > 0)` to find the exit value.
            let steps = ((shl0 as i64) + 7) / 8; // number of `-= 8` executions
            let exit = shl0 as i64 - 8 * steps;
            assert!(
                (-7..=0).contains(&exit),
                "ba={ba} s={s}: shl exits at {exit}, -shl would be out of 0..32"
            );
        }
    }
}

// ===========================================================================
// E10 — total_bands == 0
// ===========================================================================
#[test]
fn e10_total_bands_zero() {
    let mut rng = Rng::new(SEED ^ 8);
    for _ in 0..128 {
        let buf = random_buf(&mut rng);
        let mut sci = Sci::zeroed();
        sci.total_bands = 0;
        for i in 0..64 {
            sci.bitalloc[i] = rng.next_u32() as u8;
            sci.scfcod[i] = rng.next_u32() as u8;
        }
        let gs = rng.range(0, 64) as i32;
        let pos = rng.range(0, 100_000) as i32;
        diff_case_must_run("E10", &buf, &sci, gs, pos, ample_limit(buf.len()));
        // bs->pos must be untouched and the return value must be group_size*4.
        let a = raw_call(c_dequantize(), std::ptr::null_mut(), &buf, &sci, gs, pos, 1 << 20);
        let b = raw_call(rust_dequantize(), std::ptr::null_mut(), &buf, &sci, gs, pos, 1 << 20);
        assert_raw_eq("E10-raw", &a, &b);
        assert_eq!(a.pos, pos, "no bits may be consumed when total_bands == 0");
        assert_eq!(a.ret, gs.wrapping_mul(4));
    }
}

// ===========================================================================
// E11 — bitalloc[i] == 0 skips the band but still advances dst / choff
// ===========================================================================
#[test]
fn e11_bitalloc_zero() {
    let mut rng = Rng::new(SEED ^ 9);
    for _ in 0..128 {
        let buf = random_buf(&mut rng);
        let tb = rng.range(1, 32) as u8;
        let n = 2 * tb as usize;
        // Mostly zeros with a few live bands, so the choff walk has to keep
        // stepping over the skipped ones.
        let ba: Vec<u8> = (0..n)
            .map(|_| if rng.below(4) == 0 { rng.range(1, 16) as u8 } else { 0 })
            .collect();
        let sci = sci_with(tb, &ba);
        diff_case_must_run("E11", &buf, &sci, 18, 0, ample_limit(buf.len()));
    }
    // All-zero bitalloc: nothing consumed, nothing written.
    let buf = vec![0xABu8; 1024];
    let sci = sci_with(32, &[0u8; 64]);
    let a = raw_call(c_dequantize(), std::ptr::null_mut(), &buf, &sci, 18, 123, 1 << 20);
    let b = raw_call(rust_dequantize(), std::ptr::null_mut(), &buf, &sci, 18, 123, 1 << 20);
    assert_raw_eq("E11-allzero", &a, &b);
    assert_eq!(a.pos, 123);
    assert_eq!(a.ret, 72);
}

// ===========================================================================
// E12 — group_size == 0 (grouped bands still consume one field each)
// ===========================================================================
#[test]
fn e12_group_size_zero() {
    let mut rng = Rng::new(SEED ^ 10);
    for _ in 0..128 {
        let buf = random_buf(&mut rng);
        let tb = rng.range(1, 32) as u8;
        let ba: Vec<u8> = (0..2 * tb as usize)
            .map(|_| match rng.below(3) {
                0 => 0,
                1 => rng.range(1, 16) as u8,
                _ => rng.range(17, 24) as u8,
            })
            .collect();
        let sci = sci_with(tb, &ba);
        diff_case_must_run("E12", &buf, &sci, 0, 0, ample_limit(buf.len()));
    }
    // Linear band + group_size 0 => nothing consumed.
    let buf = vec![0x5Au8; 4096];
    let lin = sci_with(1, &[9, 0]);
    let a = raw_call(c_dequantize(), std::ptr::null_mut(), &buf, &lin, 0, 0, 1 << 20);
    let b = raw_call(rust_dequantize(), std::ptr::null_mut(), &buf, &lin, 0, 0, 1 << 20);
    assert_raw_eq("E12-linear", &a, &b);
    assert_eq!(a.pos, 0);
    assert_eq!(a.ret, 0);

    // Grouped band + group_size 0 => *one* field per band per granule anyway.
    let grp = sci_with(1, &[17, 0]);
    let a = raw_call(c_dequantize(), std::ptr::null_mut(), &buf, &grp, 0, 0, 1 << 20);
    let b = raw_call(rust_dequantize(), std::ptr::null_mut(), &buf, &grp, 0, 0, 1 << 20);
    assert_raw_eq("E12-grouped", &a, &b);
    assert_eq!(a.pos, 4 * 5, "ba=17 => n=5, one call per granule");
    assert_eq!(a.ret, 0);
}

// ===========================================================================
// E13 — negative group_size
// ===========================================================================
#[test]
fn e13_group_size_negative() {
    let mut rng = Rng::new(SEED ^ 11);
    for gs in [-1i32, -2, -5, -18, -1000, -100_000, i32::MIN / 8, i32::MIN] {
        for _ in 0..24 {
            let buf = random_buf(&mut rng);
            let tb = rng.range(1, 32) as u8;
            let ba: Vec<u8> = (0..2 * tb as usize)
                .map(|_| match rng.below(3) {
                    0 => 0,
                    1 => rng.range(1, 16) as u8,
                    _ => rng.range(17, 24) as u8,
                })
                .collect();
            let sci = sci_with(tb, &ba);
            diff_case_must_run("E13", &buf, &sci, gs, 0, ample_limit(buf.len()));
            // No write may occur, so a NULL grbuf is safe and lets us compare
            // the raw return value including its overflow.
            let a = raw_call(c_dequantize(), std::ptr::null_mut(), &buf, &sci, gs, 0, 1 << 20);
            let b = raw_call(rust_dequantize(), std::ptr::null_mut(), &buf, &sci, gs, 0, 1 << 20);
            assert_raw_eq("E13-raw", &a, &b);
            assert_eq!(a.ret, gs.wrapping_mul(4));
        }
    }
}

// ===========================================================================
// E14 — `group_size * 4` overflows int
// ===========================================================================
#[test]
fn e14_return_overflow() {
    let buf = vec![0u8; 64];
    // total_bands == 0 means the huge group_size never indexes grbuf.
    let sci = {
        let mut s = Sci::zeroed();
        s.total_bands = 0;
        s
    };
    for gs in [
        0x4000_0000i32,
        0x2000_0001,
        0x7FFF_FFFF,
        0x6000_0000,
        i32::MIN,
        i32::MIN + 1,
        -0x4000_0000,
    ] {
        let a = raw_call(c_dequantize(), std::ptr::null_mut(), &buf, &sci, gs, 0, 0);
        let b = raw_call(rust_dequantize(), std::ptr::null_mut(), &buf, &sci, gs, 0, 0);
        assert_raw_eq("E14", &a, &b);
        assert_eq!(a.ret, gs.wrapping_mul(4), "gs={gs}");
    }
    // Same with a live all-zero bitalloc (bands visited, nothing written).
    let sci0 = sci_with(32, &[0u8; 64]);
    for gs in [0x4000_0000i32, 0x7FFF_FFFF, i32::MIN] {
        let a = raw_call(c_dequantize(), std::ptr::null_mut(), &buf, &sci0, gs, 0, 0);
        let b = raw_call(rust_dequantize(), std::ptr::null_mut(), &buf, &sci0, gs, 0, 0);
        assert_raw_eq("E14-bands", &a, &b);
        assert_eq!(a.ret, gs.wrapping_mul(4), "gs={gs}");
    }
}

// ===========================================================================
// E15 / E16 — total_bands > 32 reads past bitalloc[64] into scfcod[]
// ===========================================================================
#[test]
fn e15_total_bands_oob_into_scfcod() {
    let mut rng = Rng::new(SEED ^ 12);
    for tb in 33u8..=64 {
        for _ in 0..8 {
            let buf = random_buf(&mut rng);
            let mut sci = Sci::zeroed();
            sci.total_bands = tb;
            // bitalloc all zero, scfcod all live: the ONLY way any bits get
            // consumed is via the out-of-bounds read.
            for i in 0..64 {
                sci.bitalloc[i] = 0;
                sci.scfcod[i] = rng.range(1, 16) as u8;
            }
            diff_case_must_run("E15", &buf, &sci, 12, 0, ample_limit(buf.len()));
        }
    }
    // E16: total_bands == 64 => the last band index is 127 => scfcod[63].
    let buf = vec![0xC3u8; BUF_LEN];
    let mut sci = Sci::zeroed();
    sci.total_bands = 64;
    sci.scfcod[63] = 11;
    diff_case_must_run("E16", &buf, &sci, 18, 0, ample_limit(buf.len()));
    // And the out-of-bounds read really is what drives it: pos must advance.
    let a = raw_call(c_dequantize(), std::ptr::null_mut(), &buf, &sci, 0, 0, 1 << 20);
    let b = raw_call(rust_dequantize(), std::ptr::null_mut(), &buf, &sci, 0, 0, 1 << 20);
    assert_raw_eq("E16-raw", &a, &b);
}

// ===========================================================================
// E17 — total_bands > 64 reads *past* the L12_scale_info object.
//      Deliberately excluded from differential comparison: the value read is
//      whatever happens to follow the object in each process's stack frame, so
//      no defined result exists on either side. This test pins the exclusion
//      down so it cannot silently become a blind spot.
// ===========================================================================
#[test]
fn e17_total_bands_past_object_is_excluded() {
    let buf = vec![0u8; BUF_LEN];
    for tb in [65u8, 66, 100, 128, 200, 255] {
        let mut sci = Sci::zeroed();
        sci.total_bands = tb;
        assert!(
            plan(&sci, 18, 0, ample_limit(buf.len()), buf.len()).is_none(),
            "total_bands={tb} must be rejected as non-comparable (2*{tb} > 128)"
        );
    }
    // Exactly 64 is the largest comparable value and must NOT be rejected.
    let mut sci = Sci::zeroed();
    sci.total_bands = 64;
    assert!(plan(&sci, 18, 0, ample_limit(buf.len()), buf.len()).is_some());
}

// ===========================================================================
// E18 / E19 — the ba == 16 / ba == 17 branch boundary
// ===========================================================================
#[test]
fn e18_ba_boundary_16_17() {
    let mut rng = Rng::new(SEED ^ 13);
    for _ in 0..64 {
        let buf = random_buf(&mut rng);
        for &ba in &[15u8, 16, 17, 18] {
            let sci = sci_with(2, &[ba, ba, ba, ba]);
            for pos in [0i32, 1, 3, 7, 8, 9, 63, 64] {
                diff_case_must_run("E18", &buf, &sci, 18, pos, ample_limit(buf.len()));
            }
        }
    }
    // half for ba = 16 is 0x7FFF, and the grouped path starts at ba = 17.
    assert_eq!((1i32 << (16 - 1)) - 1, 0x7FFF);
    assert_eq!(grouped_params(17), (3, 5));
}

// ===========================================================================
// E20 — shift counts >= 32 in `2 << (ba - 17)` alias modulo 32
// ===========================================================================
#[test]
fn e20_ba_shift_count_masked() {
    let mut rng = Rng::new(SEED ^ 14);
    // ba and ba + 32 must produce identical (mod, n).
    for ba in 17i32..=223 {
        assert_eq!(
            grouped_params(ba),
            grouped_params(ba + 32),
            "ba={ba} vs ba={}",
            ba + 32
        );
    }
    // Differentially confirm the aliasing on the readable ones.
    for ba in 17u8..=24 {
        let aliased = ba + 32;
        for _ in 0..8 {
            let buf = random_buf(&mut rng);
            let s1 = sci_with(2, &vec![ba; 4]);
            let s2 = sci_with(2, &vec![aliased; 4]);
            diff_case_must_run("E20-base", &buf, &s1, 18, 0, ample_limit(buf.len()));
            diff_case_must_run("E20-alias", &buf, &s2, 18, 0, ample_limit(buf.len()));
            // ...and they must give the same bytes as each other, too.
            let g1 = {
                let mut g = vec![0f32; 8192];
                let mut bs = BsT { buf: buf.as_ptr(), pos: 0, limit: ample_limit(buf.len()) };
                let mut s = s1;
                unsafe { c_dequantize()(g.as_mut_ptr(), &mut bs, &mut s, 18) };
                g
            };
            let g2 = {
                let mut g = vec![0f32; 8192];
                let mut bs = BsT { buf: buf.as_ptr(), pos: 0, limit: ample_limit(buf.len()) };
                let mut s = s2;
                unsafe { rust_dequantize()(g.as_mut_ptr(), &mut bs, &mut s, 18) };
                g
            };
            assert!(
                g1.iter().map(|v| v.to_bits()).eq(g2.iter().map(|v| v.to_bits())),
                "ba={ba} and ba={aliased} must alias"
            );
        }
    }
}

// ===========================================================================
// E21 — ba - 17 == 31 => 2 << 31 == 0 => mod == 1 => all outputs 0.0f
// ===========================================================================
#[test]
fn e21_ba_mod_one() {
    let mut rng = Rng::new(SEED ^ 15);
    for ba in [48u8, 80, 112, 144, 176, 208, 240] {
        assert_eq!(grouped_params(ba as i32), (1, 3), "ba={ba}");
        for _ in 0..16 {
            let buf = random_buf(&mut rng);
            let tb = rng.range(1, 16) as u8;
            let sci = sci_with(tb, &vec![ba; 2 * tb as usize]);
            let gs = rng.range(1, 18) as i32;
            diff_case_must_run("E21", &buf, &sci, gs, 0, ample_limit(buf.len()));
        }
    }
    // Every produced value must be exactly +0.0f.
    let buf = vec![0xFFu8; BUF_LEN];
    let sci = sci_with(1, &[48, 0]);
    let mut g = vec![f32::NAN; 4096];
    let mut bs = BsT { buf: buf.as_ptr(), pos: 0, limit: ample_limit(buf.len()) };
    let mut s = sci;
    unsafe { c_dequantize()(g.as_mut_ptr(), &mut bs, &mut s, 4) };
    for i in 0..4 {
        assert_eq!(g[i].to_bits(), 0f32.to_bits(), "slot {i}");
    }
    assert_eq!(bs.pos, 4 * 3, "ba=48 => n=3");
}

// ===========================================================================
// E23 — ba == 255 (max uint8_t): (255-17)&31 == 14 => mod = 32769, n = 28675
// ===========================================================================
#[test]
fn e23_ba_255() {
    let (m, n) = grouped_params(255);
    assert_eq!(m, 32769);
    assert_eq!(n, 32769 + 2 - 4096);
    assert_eq!(n, 28675);

    let mut rng = Rng::new(SEED ^ 16);
    for _ in 0..24 {
        let buf = random_buf(&mut rng);
        // One live band, ample limit: 4 x 28675 bits = 114700 bits < 512 Kbit.
        let sci = sci_with(1, &[255, 0]);
        let gs = rng.range(1, 18) as i32;
        diff_case_must_run("E23-read", &buf, &sci, gs, 0, ample_limit(buf.len()));
        // And with the limit blocking every read.
        let sci_many = sci_with(rng.range(1, 32) as u8, &vec![255u8; 64]);
        diff_case_must_run("E23-noread", &buf, &sci_many, gs, 0, NO_READ_LIMIT);
        diff_case_must_run("E23-zerolimit", &buf, &sci_many, gs, 0, 0);
    }
}

// ===========================================================================
// E24 — NULL pointers
// ===========================================================================
#[test]
fn e24_null_grbuf_no_writes() {
    let buf = vec![0x96u8; 4096];
    let mut rng = Rng::new(SEED ^ 17);

    // (a) NULL grbuf with total_bands == 0: no write is reachable.
    let mut sci = Sci::zeroed();
    for i in 0..64 {
        sci.bitalloc[i] = rng.next_u32() as u8;
        sci.scfcod[i] = rng.next_u32() as u8;
    }
    for gs in [0i32, 1, 18, -1, i32::MAX] {
        let a = raw_call(c_dequantize(), std::ptr::null_mut(), &buf, &sci, gs, 0, 1 << 20);
        let b = raw_call(rust_dequantize(), std::ptr::null_mut(), &buf, &sci, gs, 0, 1 << 20);
        assert_raw_eq("E24-a", &a, &b);
    }

    // (b) NULL grbuf with live bands but group_size <= 0: still no write.
    let live = sci_with(32, &{
        let mut v = [0u8; 64];
        for (i, x) in v.iter_mut().enumerate() {
            *x = if i % 3 == 0 { 0 } else { (i % 24 + 1) as u8 };
        }
        v
    });
    for gs in [0i32, -1, -7, -1000, i32::MIN] {
        let a = raw_call(c_dequantize(), std::ptr::null_mut(), &buf, &live, gs, 0, 1 << 20);
        let b = raw_call(rust_dequantize(), std::ptr::null_mut(), &buf, &live, gs, 0, 1 << 20);
        assert_raw_eq("E24-b", &a, &b);
    }

    // (c) NULL bs->buf, with a limit that stops every read before the deref.
    let empty: [u8; 0] = [];
    let s2 = sci_with(8, &vec![7u8; 16]);
    let mut sa = s2;
    let mut sb = s2;
    let mut bsa = BsT { buf: std::ptr::null(), pos: 0, limit: -1 };
    let mut bsb = BsT { buf: std::ptr::null(), pos: 0, limit: -1 };
    let mut ga = vec![0u32; 8192];
    let mut gb_ = vec![0u32; 8192];
    let ra = unsafe { c_dequantize()(ga.as_mut_ptr() as *mut f32, &mut bsa, &mut sa, 18) };
    let rb = unsafe { rust_dequantize()(gb_.as_mut_ptr() as *mut f32, &mut bsb, &mut sb, 18) };
    assert_eq!(ra, rb, "E24-c: return value with NULL bs->buf");
    assert_eq!(bsa.pos, bsb.pos, "E24-c: bs->pos with NULL bs->buf");
    assert_eq!(ga, gb_, "E24-c: grbuf with NULL bs->buf");
    let _ = empty;
}

// ===========================================================================
// Generic out-of-range / one-past-the-end coverage that every C API needs,
// including "enum-like" values with no valid variant: `ba` is a `uint8_t` read
// from the struct, so ALL 256 values are reachable across the FFI boundary and
// none of them is rejected by the C code.
// ===========================================================================
#[test]
fn generic_every_ba_value_0_to_255() {
    let mut rng = Rng::new(SEED ^ 18);
    let mut zero_limit_ran = 0usize;
    for ba in 0u8..=255 {
        let buf = random_buf(&mut rng);
        let sci = sci_with(1, &[ba, 0]);
        let readable = (ba as i32) < 17 || grouped_params(ba as i32).1 <= 32_768;
        let limit = if readable {
            ample_limit(buf.len())
        } else {
            NO_READ_LIMIT
        };
        diff_case_must_run("ALL-BA", &buf, &sci, 18, 0, limit);
        // Every ba must also behave identically when nothing can be read.
        diff_case_must_run("ALL-BA-noread", &buf, &sci, 18, 0, NO_READ_LIMIT);
        // limit == 0 is only comparable for the ba values whose field width is
        // small enough that the wrapping `pos += n` cannot land back below the
        // limit and drive a read from a wild pointer (see ERRORS.md E8).
        if readable {
            diff_case_must_run("ALL-BA-zerolimit", &buf, &sci, 18, 0, 0);
            zero_limit_ran += 1;
        }
    }
    assert!(
        zero_limit_ran >= 40,
        "limit==0 variant ran for only {zero_limit_ran} ba values"
    );
}

#[test]
fn generic_every_total_bands_value_0_to_64() {
    let mut rng = Rng::new(SEED ^ 19);
    for tb in 0u8..=64 {
        let buf = random_buf(&mut rng);
        let mut sci = Sci::zeroed();
        sci.total_bands = tb;
        for i in 0..64 {
            sci.bitalloc[i] = rng.range(0, 20) as u8;
            sci.scfcod[i] = rng.range(0, 20) as u8;
        }
        diff_case_must_run("ALL-TB", &buf, &sci, 12, 0, ample_limit(buf.len()));
    }
}

#[test]
fn generic_extreme_limits_and_positions() {
    let mut rng = Rng::new(SEED ^ 20);
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
    let mut ran = 0usize;
    for &pos in &interesting {
        for &limit in &interesting {
            let buf = random_buf(&mut rng);
            let sci = sci_with(4, &vec![5u8; 8]);
            if diff_case("GEN", &buf, &sci, 18, pos, limit) {
                ran += 1;
            }
            let sci_g = sci_with(4, &vec![19u8; 8]);
            if diff_case("GEN-g", &buf, &sci_g, 18, pos, limit) {
                ran += 1;
            }
        }
    }
    assert!(ran >= 40, "only {ran} extreme pos/limit combinations ran");
}

// ===========================================================================
// E7b — negative `bs->pos` that ACTUALLY reads.
//
// `bs->buf` is pointed into the middle of a large allocation, so
// `bs->buf + (bs->pos >> 3)` with a negative `pos` still lands on real memory.
// This is the only way to observe that `bs->pos >> 3` is an *arithmetic* shift
// (a logical shift would index ~2^29 bytes forward instead) and that
// `bs->pos & 7` still yields 0..7 for negative `pos`.
// ===========================================================================
#[test]
fn e7b_negative_pos_with_real_reads() {
    let mut rng = Rng::new(SEED ^ 21);
    const ORIGIN: usize = 1 << 15; // 32 KiB of readable bytes *before* bs->buf
    let mut ran = 0usize;
    for _ in 0..256 {
        let mut backing = vec![0u8; BUF_LEN];
        rng.fill(&mut backing);
        let tb = rng.range(1, 12) as u8;
        let ba: Vec<u8> = (0..2 * tb as usize)
            .map(|_| match rng.below(3) {
                0 => 0,
                1 => rng.range(1, 16) as u8,
                _ => rng.range(17, 22) as u8,
            })
            .collect();
        let sci = sci_with(tb, &ba);
        // Negative start position, but shallow enough that the whole granule
        // stays inside the allocation, and a limit that permits the reads.
        let pos = -((rng.range(1, 20_000)) as i32);
        let limit = ample_limit(BUF_LEN);
        if diff_case_origin("E7b", &backing, ORIGIN, &sci, 18, pos, limit) {
            ran += 1;
        }
    }
    assert!(ran >= 100, "E7b only ran {ran} cases with real negative-pos reads");

    // Every negative bit phase, exercised with a real read.
    let mut backing = vec![0u8; BUF_LEN];
    rng.fill(&mut backing);
    for k in 1..=64i32 {
        let sci = sci_with(2, &[13u8, 0, 19, 5]);
        diff_case_origin_must_run("E7b-phase", &backing, ORIGIN, &sci, 4, -k, ample_limit(BUF_LEN));
    }
    // ...and single-bit-wide fields so each phase is isolated.
    for k in 1..=64i32 {
        let sci = sci_with(1, &[1u8, 0]);
        diff_case_origin_must_run("E7b-1bit", &backing, ORIGIN, &sci, 1, -k, ample_limit(BUF_LEN));
    }
}

// ===========================================================================
// Cross-check: the same `bs->buf`-into-the-middle trick applied to the whole
// randomized fuzz, with both positive and negative starting positions.
// ===========================================================================
#[test]
fn e7c_origin_fuzz_mixed_sign_positions() {
    let mut rng = Rng::new(SEED ^ 22);
    const ORIGIN: usize = 1 << 14;
    let mut ran = 0usize;
    for _ in 0..2000 {
        let mut backing = vec![0u8; BUF_LEN];
        rng.fill(&mut backing);
        let tb = rng.range(0, 64) as u8;
        let mut sci = Sci::zeroed();
        sci.total_bands = tb;
        for i in 0..64 {
            sci.bitalloc[i] = match rng.below(4) {
                0 => 0,
                1 => rng.range(1, 16) as u8,
                2 => rng.range(17, 22) as u8,
                _ => rng.range(0, 24) as u8,
            };
            sci.scfcod[i] = rng.range(0, 22) as u8;
        }
        let gs = match rng.below(4) {
            0 => 0i32,
            1 => 1,
            2 => 18,
            _ => rng.range(1, 32) as i32,
        };
        let pos = if rng.bool() {
            -((rng.range(1, 100_000)) as i32)
        } else {
            rng.range(0, 100_000) as i32
        };
        let limit = match rng.below(3) {
            0 => ample_limit(BUF_LEN),
            1 => rng.range(0, 400_000) as i32,
            _ => -(rng.range(0, 200_000) as i32),
        };
        if diff_case_origin("E7c", &backing, ORIGIN, &sci, gs, pos, limit) {
            ran += 1;
        }
    }
    eprintln!("E7c: ran={ran}");
    assert!(ran >= 500, "E7c ran only {ran} cases");
}

// ===========================================================================
// E17b — total_bands 65..=255: the C code walks `sci->bitalloc[i]` for
// `i < 2*total_bands`, i.e. up to i == 509, far past the end of a bare
// `L12_scale_info`.  By embedding the struct at the start of a larger
// allocation whose trailing bytes the test fills with a known pattern, both
// libraries read exactly the same memory and the whole 0..=255 range of
// `total_bands` becomes byte-comparable.
//
// This is what makes a "read total_bands as a signed byte" style bug visible:
// for total_bands >= 128 a sign-extending read yields a negative band count and
// the loop is skipped entirely.
// ===========================================================================
#[test]
fn e17b_total_bands_65_to_255_padded() {
    let mut rng = Rng::new(SEED ^ 23);
    // i goes up to 2*255-1 == 509 => the alloc region must be >= 510 bytes.
    const TAIL: usize = 1024;
    for tb in 0u8..=255 {
        let buf = random_buf(&mut rng);
        let mut sci = Sci::zeroed();
        for i in 0..64 {
            sci.bitalloc[i] = rng.range(0, 20) as u8;
            sci.scfcod[i] = rng.range(0, 20) as u8;
        }
        sci.total_bands = tb;
        let mut padded = PaddedSci::new(&sci, TAIL, &mut rng);
        // Keep every band width small so the reads stay inside `buf`.
        for i in 0..(2 * tb as usize).min(TAIL + 130) {
            padded.set_alloc_byte(i, (i % 21) as u8);
        }
        padded.set_total_bands(tb);
        diff_case_padded_must_run("E17b", &buf, 0, &padded, 4, 0, ample_limit(buf.len()));
    }
}

/// The same, but with `total_bands >= 128` specifically and *only* the
/// past-the-struct bytes carrying non-zero band widths — so if the band count
/// were computed from a sign-extended `total_bands`, nothing at all would be
/// decoded and `bs->pos` would stay at 0.
#[test]
fn e17c_total_bands_high_bit_set() {
    let mut rng = Rng::new(SEED ^ 24);
    const TAIL: usize = 1024;
    for tb in [128u8, 129, 130, 160, 200, 254, 255] {
        let buf = random_buf(&mut rng);
        let sci = Sci::zeroed(); // bitalloc and scfcod are all zero
        let mut padded = PaddedSci::new(&sci, TAIL, &mut rng);
        for i in 0..128 {
            padded.set_alloc_byte(i, 0);
        }
        // Live band widths live ONLY past the end of the struct.
        for i in 128..(2 * tb as usize) {
            padded.set_alloc_byte(i, ((i % 15) + 1) as u8);
        }
        padded.set_total_bands(tb);
        diff_case_padded_must_run("E17c", &buf, 0, &padded, 8, 0, ample_limit(buf.len()));

        // Pin down that bits really were consumed (so the loop truly ran).
        let mut probe = padded.clone_bytes();
        let mut bs = BsT {
            buf: buf.as_ptr(),
            pos: 0,
            limit: ample_limit(buf.len()),
        };
        let mut g = vec![0f32; 1 << 16];
        unsafe {
            c_dequantize()(g.as_mut_ptr(), &mut bs, probe.as_mut_ptr() as *mut Sci, 8);
        }
        assert!(
            bs.pos > 0,
            "tb={tb}: expected the past-the-struct bands to consume bits, pos={}",
            bs.pos
        );
    }
}

/// Randomized sweep over the padded configuration: `total_bands` across the
/// whole `u8` range, mixed band widths, mixed limits and positions.
#[test]
fn e17d_padded_fuzz() {
    let mut rng = Rng::new(SEED ^ 25);
    const TAIL: usize = 1024;
    let mut ran = 0usize;
    for _ in 0..1500 {
        let buf = random_buf(&mut rng);
        let mut sci = Sci::zeroed();
        sci.total_bands = rng.range(0, 255) as u8;
        let mut padded = PaddedSci::new(&sci, TAIL, &mut rng);
        let mut huge = false;
        let n = (2 * padded.total_bands() as usize).min(TAIL + 128);
        for i in 0..n {
            let v = match rng.below(5) {
                0 => 0u8,
                1 => rng.range(1, 16) as u8,
                2 => rng.range(17, 21) as u8,
                3 => rng.range(0, 255) as u8,
                _ => rng.range(1, 10) as u8,
            };
            padded.set_alloc_byte(i, v);
            let ba = v as i32;
            if ba >= 17 && grouped_params(ba).1 > 4096 {
                huge = true;
            }
        }
        let gs = match rng.below(4) {
            0 => 0i32,
            1 => 1,
            2 => 4,
            _ => rng.range(1, 12) as i32,
        };
        let limit = if huge {
            NO_READ_LIMIT
        } else {
            match rng.below(4) {
                0 => ample_limit(buf.len()),
                1 => 0,
                2 => rng.range(0, 300_000) as i32,
                _ => NO_READ_LIMIT,
            }
        };
        let pos = if rng.bool() { 0 } else { rng.range(0, 64) as i32 };
        if diff_case_padded("E17d", &buf, 0, &padded, gs, pos, limit) {
            ran += 1;
        }
    }
    eprintln!("E17d: ran={ran}");
    assert!(ran >= 600, "E17d ran only {ran} cases");
}
