//! Phase B, part 2 — `CONFIGS.md` rows 34-55: the backend hash primitives and
//! the NIST AES-256-CTR DRBG, all reached through `dlopen`/`dlsym`.
//!
//! Rows 34-39 apply to `HASH_BACKEND=blake`, 40-43 to `sha2`, 44-46 to `shake`,
//! 47-50 to `haraka`.  Each test asserts the active backend from `params.txt`
//! and returns early otherwise, so a mis-wired configuration cannot make a row
//! pass by accident.

mod common;

use common::*;
use std::ffi::{c_ulong, c_ulonglong};

/// `true` when the active backend is `want`; otherwise the row does not apply.
fn backend_is(want: &str) -> bool {
    env().1.backend() == want
}

/* ================================================================== */
/* rows 34-39 — BLAKE-256 / BLAKE-512                                  */
/* ================================================================== */

/// Input lengths that straddle every branch in `blake256_final` /
/// `blake512_final` (the 440-bit and 888-bit padding boundaries live at
/// 55 and 111 bytes) plus generic multi-block cases.
fn blake_lengths(rng: &mut Rng) -> Vec<usize> {
    let mut v: Vec<usize> = vec![
        0, 1, 2, 53, 54, 55, 56, 57, 63, 64, 65, 110, 111, 112, 113, 119, 120, 127, 128, 129, 191,
        192, 255, 256, 1000,
    ];
    for _ in 0..32 {
        v.push(rng.below(4096));
    }
    v.sort_unstable();
    v.dedup();
    v
}

#[test]
fn cfg34_37_blake_one_shot() {
    if !backend_is("blake") {
        return;
    }
    let (l, p) = env();
    let mut rng = Rng::new(0x3434);
    unsafe {
        for (name, outlen) in [
            ("blake256", p.n("SPX_BLAKE256_OUTPUT_BYTES")),
            ("blake512", p.n("SPX_BLAKE512_OUTPUT_BYTES")),
        ] {
            let cf: FnBlakeOneShot = *l.c(name);
            let rf: FnBlakeOneShot = *l.r(name);
            for inlen in blake_lengths(&mut rng) {
                let input = rng.bytes(inlen);
                let mut co = vec![0xA5u8; outlen + 32];
                let mut ro = co.clone();
                let cr = cf(co.as_mut_ptr(), input.as_ptr(), inlen as c_ulonglong);
                let rr = rf(ro.as_mut_ptr(), input.as_ptr(), inlen as c_ulonglong);
                same_i(&format!("{name}(inlen={inlen}) ret"), cr, rr);
                assert_eq!(cr, 0, "{name} is documented to always return 0");
                same(&format!("{name}(inlen={inlen})"), &co, &ro);
            }
        }
    }
}

#[test]
fn cfg35_37_blake_incremental() {
    if !backend_is("blake") {
        return;
    }
    let (l, p) = env();
    let mut rng = Rng::new(0x3535);
    unsafe {
        for (bits, state_size, outlen) in [
            (256usize, p.n("sizeof_blakestate256"), p.n("SPX_BLAKE256_OUTPUT_BYTES")),
            (512, p.n("sizeof_blakestate512"), p.n("SPX_BLAKE512_OUTPUT_BYTES")),
        ] {
            let ci: FnBlakeInit = *l.c(&format!("blake{bits}_init"));
            let ri: FnBlakeInit = *l.r(&format!("blake{bits}_init"));
            let cu: FnBlakeUpdate = *l.c(&format!("blake{bits}_update"));
            let ru: FnBlakeUpdate = *l.r(&format!("blake{bits}_update"));
            let cfin: FnBlakeFinal = *l.c(&format!("blake{bits}_final"));
            let rfin: FnBlakeFinal = *l.r(&format!("blake{bits}_final"));

            // A prefix of exactly 54/55/56 (and 110/111/112) bytes lands
            // buflen just below / at / just above the one-padding-byte case.
            let mut plans: Vec<Vec<usize>> = vec![
                vec![],
                vec![0],
                vec![1],
                vec![54],
                vec![55],
                vec![56],
                vec![110],
                vec![111],
                vec![112],
                vec![64],
                vec![128],
                vec![1, 1, 1],
                vec![63, 1],
                vec![64, 1],
                vec![32, 32, 32],
                vec![127, 1, 1],
            ];
            for _ in 0..24 {
                let k = 1 + rng.below(6);
                plans.push((0..k).map(|_| rng.below(300)).collect());
            }

            for plan in plans {
                let mut cs = vec![0xA5u8; state_size];
                let mut rs = cs.clone();
                ci(cs.as_mut_ptr());
                ri(rs.as_mut_ptr());
                same(&format!("blake{bits}_init"), &cs, &rs);
                for &chunk in &plan {
                    let data = rng.bytes(chunk);
                    // NOTE: blake*_update takes the length in BITS
                    cu(cs.as_mut_ptr(), data.as_ptr(), (chunk * 8) as c_ulonglong);
                    ru(rs.as_mut_ptr(), data.as_ptr(), (chunk * 8) as c_ulonglong);
                    same(&format!("blake{bits}_update({chunk}B) state, plan={plan:?}"), &cs, &rs);
                }
                let mut co = vec![0xA5u8; outlen + 32];
                let mut ro = co.clone();
                cfin(cs.as_mut_ptr(), co.as_mut_ptr());
                rfin(rs.as_mut_ptr(), ro.as_mut_ptr());
                same(&format!("blake{bits}_final digest, plan={plan:?}"), &co, &ro);
                same(&format!("blake{bits}_final state, plan={plan:?}"), &cs, &rs);
            }
        }
    }
}

#[test]
fn cfg36_37_blake_compress() {
    if !backend_is("blake") {
        return;
    }
    let (l, p) = env();
    let mut rng = Rng::new(0x3636);
    unsafe {
        for (bits, state_size, block) in [
            (256usize, p.n("sizeof_blakestate256"), 64usize),
            (512, p.n("sizeof_blakestate512"), 128),
        ] {
            let cf: FnBlakeCompress = *l.c(&format!("blake{bits}_compress"));
            let rf: FnBlakeCompress = *l.r(&format!("blake{bits}_compress"));
            for i in 0..64 {
                let mut cs = rng.bytes(state_size);
                // `nullt` is read as a boolean; drive both 0 and non-0.  It sits
                // right after `buflen`, i.e. at the two ints before `buf`.
                let ints_at = state_size - block - 8;
                let nullt = if i % 2 == 0 { 0u32 } else { 1 };
                cs[ints_at + 4..ints_at + 8].copy_from_slice(&nullt.to_le_bytes());
                // exercise the t[0] wrap that bumps t[1]
                if i % 3 == 0 {
                    let t_at = if bits == 256 { 48 } else { 96 };
                    cs[t_at..t_at + if bits == 256 { 4 } else { 8 }].fill(0xFF);
                }
                let rs0 = cs.clone();
                let mut rs = rs0;
                let blk = rng.bytes(block);
                cf(cs.as_mut_ptr(), blk.as_ptr());
                rf(rs.as_mut_ptr(), blk.as_ptr());
                same(&format!("blake{bits}_compress state (i={i})"), &cs, &rs);
            }
        }
    }
}

#[test]
fn cfg38_blake_mgf1() {
    if !backend_is("blake") {
        return;
    }
    let (l, _p) = env();
    let mut rng = Rng::new(0x3838);
    unsafe {
        for name in ["SPX_blake256_mgf1", "SPX_blake512_mgf1"] {
            let cf: FnMgf1 = *l.c(name);
            let rf: FnMgf1 = *l.r(name);
            for outlen in [0usize, 1, 2, 31, 32, 33, 63, 64, 65, 100, 128, 129, 256] {
                for inlen in [0usize, 1, 4, 32, 48, 64, 100] {
                    let input = rng.bytes(inlen);
                    let mut co = vec![0xA5u8; outlen + 32];
                    let mut ro = co.clone();
                    cf(
                        co.as_mut_ptr(),
                        outlen as c_ulong,
                        input.as_ptr(),
                        inlen as c_ulong,
                    );
                    rf(
                        ro.as_mut_ptr(),
                        outlen as c_ulong,
                        input.as_ptr(),
                        inlen as c_ulong,
                    );
                    same(&format!("{name}(outlen={outlen}, inlen={inlen})"), &co, &ro);
                }
            }
        }
    }
}

#[test]
fn cfg39_blake_cst_global() {
    if !backend_is("blake") {
        return;
    }
    let (l, _p) = env();
    unsafe {
        // `const u64 cst[16]` in blake512.c -- 128 bytes of read-only data.
        let c = std::slice::from_raw_parts(l.c_data("cst"), 128);
        let r = std::slice::from_raw_parts(l.r_data("cst"), 128);
        same("exported `cst` table", c, r);
        // sanity: first word is the BLAKE-512 constant 0x243F6A8885A308D3
        assert_eq!(
            u64::from_le_bytes(c[0..8].try_into().unwrap()),
            0x243F_6A88_85A3_08D3
        );
    }
}

/* ================================================================== */
/* rows 40-43 — SHA-256 / SHA-512                                      */
/* ================================================================== */

fn sha_lengths(rng: &mut Rng, block: usize) -> Vec<usize> {
    let mut v: Vec<usize> = vec![0, 1, 2, 63, 64, 65, 127, 128, 129, 1000];
    // the "needs one more block" padding boundary is at block-9 / block-8
    for k in 0..=2usize {
        for d in [-2i64, -1, 0, 1, 2] {
            let x = (k * block) as i64 + block as i64 - 9 + d;
            if x >= 0 {
                v.push(x as usize);
            }
            let y = (k * block) as i64 + d;
            if y >= 0 {
                v.push(y as usize);
            }
        }
    }
    for _ in 0..32 {
        v.push(rng.below(1024));
    }
    v.sort_unstable();
    v.dedup();
    v
}

#[test]
fn cfg40_sha_one_shot() {
    if !backend_is("sha2") {
        return;
    }
    let (l, _p) = env();
    let mut rng = Rng::new(0x4040);
    unsafe {
        for (name, outlen, block) in [("sha256", 32usize, 64usize), ("sha512", 64, 128)] {
            let cf: FnShaOneShot = *l.c(name);
            let rf: FnShaOneShot = *l.r(name);
            for inlen in sha_lengths(&mut rng, block) {
                let input = rng.bytes(inlen);
                let mut co = vec![0xA5u8; outlen + 32];
                let mut ro = co.clone();
                cf(co.as_mut_ptr(), input.as_ptr(), inlen);
                rf(ro.as_mut_ptr(), input.as_ptr(), inlen);
                same(&format!("{name}(inlen={inlen})"), &co, &ro);
            }
        }
    }
}

#[test]
fn cfg41_sha_incremental() {
    if !backend_is("sha2") {
        return;
    }
    let (l, _p) = env();
    let mut rng = Rng::new(0x4141);
    unsafe {
        for (bits, state_size, outlen, block) in
            [(256usize, 40usize, 32usize, 64usize), (512, 72, 64, 128)]
        {
            let ci: FnShaIncInit = *l.c(&format!("sha{bits}_inc_init"));
            let ri: FnShaIncInit = *l.r(&format!("sha{bits}_inc_init"));
            let cb: FnShaIncBlocks = *l.c(&format!("sha{bits}_inc_blocks"));
            let rb: FnShaIncBlocks = *l.r(&format!("sha{bits}_inc_blocks"));
            let cfin: FnShaIncFinalize = *l.c(&format!("sha{bits}_inc_finalize"));
            let rfin: FnShaIncFinalize = *l.r(&format!("sha{bits}_inc_finalize"));

            for nblocks in [0usize, 1, 2, 5] {
                for final_len in sha_lengths(&mut rng, block).into_iter().take(24) {
                    let mut cs = vec![0xA5u8; state_size];
                    let mut rs = cs.clone();
                    ci(cs.as_mut_ptr());
                    ri(rs.as_mut_ptr());
                    same(&format!("sha{bits}_inc_init"), &cs, &rs);

                    let data = rng.bytes(nblocks * block);
                    cb(cs.as_mut_ptr(), data.as_ptr(), nblocks);
                    rb(rs.as_mut_ptr(), data.as_ptr(), nblocks);
                    same(
                        &format!("sha{bits}_inc_blocks({nblocks}) state"),
                        &cs,
                        &rs,
                    );

                    let tail = rng.bytes(final_len);
                    let mut co = vec![0xA5u8; outlen + 32];
                    let mut ro = co.clone();
                    cfin(co.as_mut_ptr(), cs.as_mut_ptr(), tail.as_ptr(), final_len);
                    rfin(ro.as_mut_ptr(), rs.as_mut_ptr(), tail.as_ptr(), final_len);
                    same(
                        &format!("sha{bits}_inc_finalize({nblocks} blk + {final_len} B)"),
                        &co,
                        &ro,
                    );
                }
            }
        }
    }
}

#[test]
fn cfg42_sha_mgf1() {
    if !backend_is("sha2") {
        return;
    }
    let (l, _p) = env();
    let mut rng = Rng::new(0x4242);
    unsafe {
        for name in ["SPX_mgf1_256", "SPX_mgf1_512"] {
            let cf: FnMgf1 = *l.c(name);
            let rf: FnMgf1 = *l.r(name);
            for outlen in [0usize, 1, 2, 31, 32, 33, 63, 64, 65, 100, 128, 129, 256] {
                for inlen in [0usize, 1, 4, 32, 48, 64, 100] {
                    let input = rng.bytes(inlen);
                    let mut co = vec![0xA5u8; outlen + 32];
                    let mut ro = co.clone();
                    cf(co.as_mut_ptr(), outlen as c_ulong, input.as_ptr(), inlen as c_ulong);
                    rf(ro.as_mut_ptr(), outlen as c_ulong, input.as_ptr(), inlen as c_ulong);
                    same(&format!("{name}(outlen={outlen}, inlen={inlen})"), &co, &ro);
                }
            }
        }
    }
}

#[test]
fn cfg43_seed_state() {
    if !backend_is("sha2") {
        return;
    }
    let (l, p) = env();
    let mut rng = Rng::new(0x4343);
    unsafe {
        let cf: FnSeedState = *l.c("SPX_seed_state");
        let rf: FnSeedState = *l.r("SPX_seed_state");
        for _ in 0..64 {
            let mut cs = vec![0u8; p.ctx_size() + 32];
            rng.fill(&mut cs[..2 * p.n_()]);
            let mut rs = cs.clone();
            cf(cs.as_mut_ptr());
            rf(rs.as_mut_ptr());
            same("SPX_seed_state ctx", &cs, &rs);
        }
    }
}

/* ================================================================== */
/* rows 44-46 — SHAKE-256                                              */
/* ================================================================== */

const SHAKE256_RATE: usize = 136;

#[test]
fn cfg44_shake256_one_shot() {
    if !backend_is("shake") {
        return;
    }
    let (l, _p) = env();
    let mut rng = Rng::new(0x4444);
    unsafe {
        let cf: FnShake = *l.c("shake256");
        let rf: FnShake = *l.r("shake256");
        let lens: Vec<usize> = vec![
            0,
            1,
            2,
            32,
            SHAKE256_RATE - 1,
            SHAKE256_RATE,
            SHAKE256_RATE + 1,
            2 * SHAKE256_RATE - 1,
            2 * SHAKE256_RATE,
            2 * SHAKE256_RATE + 1,
            1000,
        ];
        for &outlen in &lens {
            for &inlen in &lens {
                let input = rng.bytes(inlen);
                let mut co = vec![0xA5u8; outlen + 32];
                let mut ro = co.clone();
                cf(co.as_mut_ptr(), outlen, input.as_ptr(), inlen);
                rf(ro.as_mut_ptr(), outlen, input.as_ptr(), inlen);
                same(&format!("shake256(outlen={outlen}, inlen={inlen})"), &co, &ro);
            }
        }
    }
}

#[test]
fn cfg45_shake256_incremental() {
    if !backend_is("shake") {
        return;
    }
    let (l, _p) = env();
    let mut rng = Rng::new(0x4545);
    // uint64_t s_inc[26]
    let state_words = 26usize;
    unsafe {
        let ci: FnShakeIncInit = *l.c("shake256_inc_init");
        let ri: FnShakeIncInit = *l.r("shake256_inc_init");
        let ca: FnShakeIncAbsorb = *l.c("shake256_inc_absorb");
        let ra: FnShakeIncAbsorb = *l.r("shake256_inc_absorb");
        let cfin: FnShakeIncFinalize = *l.c("shake256_inc_finalize");
        let rfin: FnShakeIncFinalize = *l.r("shake256_inc_finalize");
        let csq: FnShakeIncSqueeze = *l.c("shake256_inc_squeeze");
        let rsq: FnShakeIncSqueeze = *l.r("shake256_inc_squeeze");

        let mut plans: Vec<Vec<usize>> = vec![
            vec![],
            vec![0],
            vec![1],
            vec![SHAKE256_RATE - 1],
            vec![SHAKE256_RATE],
            vec![SHAKE256_RATE + 1],
            vec![1, SHAKE256_RATE - 1],
            vec![SHAKE256_RATE, SHAKE256_RATE],
            vec![17, 17, 17, 17, 17, 17, 17, 17],
        ];
        for _ in 0..20 {
            let k = 1 + rng.below(6);
            plans.push((0..k).map(|_| rng.below(300)).collect());
        }
        let squeeze_plans: Vec<Vec<usize>> = vec![
            vec![0],
            vec![1],
            vec![SHAKE256_RATE - 1],
            vec![SHAKE256_RATE],
            vec![SHAKE256_RATE + 1],
            vec![1, 1, 1],
            vec![100, 100, 100],
            vec![SHAKE256_RATE, 1, SHAKE256_RATE],
        ];

        for plan in &plans {
            for sq in &squeeze_plans {
                let mut cs = vec![0u64; state_words];
                let mut rs = vec![0u64; state_words];
                ci(cs.as_mut_ptr());
                ri(rs.as_mut_ptr());
                same("shake256_inc_init", words(&cs), words(&rs));
                for &chunk in plan {
                    let data = rng.bytes(chunk);
                    ca(cs.as_mut_ptr(), data.as_ptr(), chunk);
                    ra(rs.as_mut_ptr(), data.as_ptr(), chunk);
                    same(
                        &format!("shake256_inc_absorb({chunk}) plan={plan:?}"),
                        words(&cs),
                        words(&rs),
                    );
                }
                cfin(cs.as_mut_ptr());
                rfin(rs.as_mut_ptr());
                same("shake256_inc_finalize", words(&cs), words(&rs));
                for &outlen in sq {
                    let mut co = vec![0xA5u8; outlen + 32];
                    let mut ro = co.clone();
                    csq(co.as_mut_ptr(), outlen, cs.as_mut_ptr());
                    rsq(ro.as_mut_ptr(), outlen, rs.as_mut_ptr());
                    same(
                        &format!("shake256_inc_squeeze({outlen}) plan={plan:?} sq={sq:?}"),
                        &co,
                        &ro,
                    );
                    same("shake256_inc_squeeze state", words(&cs), words(&rs));
                }
            }
        }
    }
}

fn words(v: &[u64]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(v.as_ptr() as *const u8, v.len() * 8) }
}

#[test]
fn cfg46_shake256_absorb_squeezeblocks() {
    if !backend_is("shake") {
        return;
    }
    let (l, _p) = env();
    let mut rng = Rng::new(0x4646);
    unsafe {
        let ca: FnShakeAbsorb = *l.c("shake256_absorb");
        let ra: FnShakeAbsorb = *l.r("shake256_absorb");
        let cs_: FnShakeSqueezeBlocks = *l.c("shake256_squeezeblocks");
        let rs_: FnShakeSqueezeBlocks = *l.r("shake256_squeezeblocks");
        for inlen in [
            0usize,
            1,
            SHAKE256_RATE - 1,
            SHAKE256_RATE,
            SHAKE256_RATE + 1,
            3 * SHAKE256_RATE,
            500,
        ] {
            for nblocks in [0usize, 1, 2, 5] {
                let input = rng.bytes(inlen);
                let mut cs = vec![0u64; 25];
                let mut rs = vec![0u64; 25];
                ca(cs.as_mut_ptr(), input.as_ptr(), inlen);
                ra(rs.as_mut_ptr(), input.as_ptr(), inlen);
                same(
                    &format!("shake256_absorb({inlen}) state"),
                    words(&cs),
                    words(&rs),
                );
                let mut co = vec![0xA5u8; nblocks * SHAKE256_RATE + 32];
                let mut ro = co.clone();
                cs_(co.as_mut_ptr(), nblocks, cs.as_mut_ptr());
                rs_(ro.as_mut_ptr(), nblocks, rs.as_mut_ptr());
                same(
                    &format!("shake256_squeezeblocks(in={inlen}, nblocks={nblocks})"),
                    &co,
                    &ro,
                );
                same("squeezeblocks state", words(&cs), words(&rs));
            }
        }
    }
}

/* ================================================================== */
/* rows 47-50 — Haraka                                                 */
/* ================================================================== */

/// Builds a tweaked Haraka context in both libraries and asserts equality.
fn haraka_ctx(rng: &mut Rng) -> Vec<u8> {
    let (l, p) = env();
    let mut cs = vec![0u8; p.ctx_size() + 32];
    rng.fill(&mut cs[..2 * p.n_()]);
    let mut rs = cs.clone();
    unsafe {
        let cf: FnTweakConstants = *l.c("SPX_tweak_constants");
        let rf: FnTweakConstants = *l.r("SPX_tweak_constants");
        cf(cs.as_mut_ptr());
        rf(rs.as_mut_ptr());
    }
    same("SPX_tweak_constants ctx", &cs, &rs);
    cs.truncate(p.ctx_size());
    cs
}

#[test]
fn cfg47_tweak_constants() {
    if !backend_is("haraka") {
        return;
    }
    let mut rng = Rng::new(0x4747);
    for _ in 0..64 {
        haraka_ctx(&mut rng);
    }
    // extreme seeds
    let (l, p) = env();
    for fill in [0x00u8, 0xFF] {
        let mut cs = vec![0u8; p.ctx_size() + 32];
        cs[..2 * p.n_()].fill(fill);
        let mut rs = cs.clone();
        unsafe {
            let cf: FnTweakConstants = *l.c("SPX_tweak_constants");
            let rf: FnTweakConstants = *l.r("SPX_tweak_constants");
            cf(cs.as_mut_ptr());
            rf(rs.as_mut_ptr());
        }
        same(&format!("SPX_tweak_constants(seed={fill:#x})"), &cs, &rs);
    }
}

#[test]
fn cfg48_haraka_permutations() {
    if !backend_is("haraka") {
        return;
    }
    let (l, _p) = env();
    let mut rng = Rng::new(0x4848);
    unsafe {
        for (name, inlen, outlen) in [
            ("SPX_haraka512", 64usize, 32usize),
            ("SPX_haraka512_perm", 64, 64),
            ("SPX_haraka256", 32, 32),
        ] {
            let cf: FnHaraka512 = *l.c(name);
            let rf: FnHaraka512 = *l.r(name);
            for i in 0..64 {
                let ctx = haraka_ctx(&mut rng);
                let input = if i == 0 {
                    vec![0u8; inlen]
                } else if i == 1 {
                    vec![0xFFu8; inlen]
                } else {
                    rng.bytes(inlen)
                };
                let mut co = vec![0xA5u8; outlen + 32];
                let mut ro = co.clone();
                cf(co.as_mut_ptr(), input.as_ptr(), ctx.as_ptr());
                rf(ro.as_mut_ptr(), input.as_ptr(), ctx.as_ptr());
                same(&format!("{name}(i={i})"), &co, &ro);
            }
        }
    }
}

#[test]
fn cfg49_haraka_s_one_shot() {
    if !backend_is("haraka") {
        return;
    }
    let (l, _p) = env();
    let mut rng = Rng::new(0x4949);
    // the Haraka sponge rate is 32 bytes
    let lens: Vec<usize> = vec![0, 1, 2, 31, 32, 33, 63, 64, 65, 200, 500];
    unsafe {
        let cf: FnHarakaS = *l.c("SPX_haraka_S");
        let rf: FnHarakaS = *l.r("SPX_haraka_S");
        for &outlen in &lens {
            for &inlen in &lens {
                let ctx = haraka_ctx(&mut rng);
                let input = rng.bytes(inlen);
                let mut co = vec![0xA5u8; outlen + 32];
                let mut ro = co.clone();
                cf(
                    co.as_mut_ptr(),
                    outlen as c_ulonglong,
                    input.as_ptr(),
                    inlen as c_ulonglong,
                    ctx.as_ptr(),
                );
                rf(
                    ro.as_mut_ptr(),
                    outlen as c_ulonglong,
                    input.as_ptr(),
                    inlen as c_ulonglong,
                    ctx.as_ptr(),
                );
                same(
                    &format!("SPX_haraka_S(outlen={outlen}, inlen={inlen})"),
                    &co,
                    &ro,
                );
            }
        }
    }
}

#[test]
fn cfg50_haraka_s_incremental() {
    if !backend_is("haraka") {
        return;
    }
    let (l, _p) = env();
    let mut rng = Rng::new(0x5050);
    unsafe {
        let ci: FnHarakaSIncInit = *l.c("SPX_haraka_S_inc_init");
        let ri: FnHarakaSIncInit = *l.r("SPX_haraka_S_inc_init");
        let ca: FnHarakaSIncAbsorb = *l.c("SPX_haraka_S_inc_absorb");
        let ra: FnHarakaSIncAbsorb = *l.r("SPX_haraka_S_inc_absorb");
        let cfin: FnHarakaSIncFinalize = *l.c("SPX_haraka_S_inc_finalize");
        let rfin: FnHarakaSIncFinalize = *l.r("SPX_haraka_S_inc_finalize");
        let csq: FnHarakaSIncSqueeze = *l.c("SPX_haraka_S_inc_squeeze");
        let rsq: FnHarakaSIncSqueeze = *l.r("SPX_haraka_S_inc_squeeze");

        let mut plans: Vec<Vec<usize>> = vec![
            vec![],
            vec![0],
            vec![1],
            vec![31],
            vec![32],
            vec![33],
            vec![1, 31],
            vec![32, 32],
            vec![7, 7, 7, 7, 7],
        ];
        for _ in 0..16 {
            let k = 1 + rng.below(5);
            plans.push((0..k).map(|_| rng.below(100)).collect());
        }
        let squeezes: Vec<Vec<usize>> =
            vec![vec![0], vec![1], vec![31], vec![32], vec![33], vec![1, 1, 1], vec![50, 50]];

        for plan in &plans {
            for sq in &squeezes {
                let ctx = haraka_ctx(&mut rng);
                // uint8_t s_inc[65]
                let mut cs = vec![0xA5u8; 65];
                let mut rs = cs.clone();
                ci(cs.as_mut_ptr());
                ri(rs.as_mut_ptr());
                same("SPX_haraka_S_inc_init", &cs, &rs);
                for &chunk in plan {
                    let data = rng.bytes(chunk);
                    ca(cs.as_mut_ptr(), data.as_ptr(), chunk, ctx.as_ptr());
                    ra(rs.as_mut_ptr(), data.as_ptr(), chunk, ctx.as_ptr());
                    same(
                        &format!("SPX_haraka_S_inc_absorb({chunk}) plan={plan:?}"),
                        &cs,
                        &rs,
                    );
                }
                cfin(cs.as_mut_ptr());
                rfin(rs.as_mut_ptr());
                same("SPX_haraka_S_inc_finalize", &cs, &rs);
                for &outlen in sq {
                    let mut co = vec![0xA5u8; outlen + 32];
                    let mut ro = co.clone();
                    csq(co.as_mut_ptr(), outlen, cs.as_mut_ptr(), ctx.as_ptr());
                    rsq(ro.as_mut_ptr(), outlen, rs.as_mut_ptr(), ctx.as_ptr());
                    same(
                        &format!("SPX_haraka_S_inc_squeeze({outlen}) plan={plan:?}"),
                        &co,
                        &ro,
                    );
                    same("SPX_haraka_S_inc_squeeze state", &cs, &rs);
                }
            }
        }
    }
}

/* ================================================================== */
/* rows 51-55 — the NIST AES-256-CTR DRBG (rng.c)                      */
/* ================================================================== */

#[test]
fn cfg51_aes256_ecb() {
    let (l, _p) = env();
    let mut rng = Rng::new(0x5151);
    unsafe {
        let cf: FnAes256Ecb = *l.c("AES256_ECB");
        let rf: FnAes256Ecb = *l.r("AES256_ECB");
        for i in 0..64 {
            let mut key = if i == 0 { vec![0u8; 32] } else { rng.bytes(32) };
            let mut ctr = if i == 0 { vec![0u8; 16] } else { rng.bytes(16) };
            let mut ck = key.clone();
            let mut cc = ctr.clone();
            let mut co = vec![0xA5u8; 16 + 32];
            let mut ro = co.clone();
            cf(ck.as_mut_ptr(), cc.as_mut_ptr(), co.as_mut_ptr());
            rf(key.as_mut_ptr(), ctr.as_mut_ptr(), ro.as_mut_ptr());
            same(&format!("AES256_ECB(i={i})"), &co, &ro);
            // neither may modify key or ctr
            same("AES256_ECB key untouched", &ck, &key);
            same("AES256_ECB ctr untouched", &cc, &ctr);
        }
        // known answer: AES-256-ECB of an all-zero block under an all-zero key
        let mut key = vec![0u8; 32];
        let mut ctr = vec![0u8; 16];
        let mut out = vec![0u8; 16];
        rf(key.as_mut_ptr(), ctr.as_mut_ptr(), out.as_mut_ptr());
        assert_eq!(
            out,
            hex("dc95c078a2408989ad48a21492842087"),
            "AES-256-ECB KAT failed"
        );
    }
}

fn hex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

#[test]
fn cfg52_drbg_update() {
    let (l, _p) = env();
    let mut rng = Rng::new(0x5252);
    unsafe {
        let cf: FnDrbgUpdate = *l.c("AES256_CTR_DRBG_Update");
        let rf: FnDrbgUpdate = *l.r("AES256_CTR_DRBG_Update");
        // V values chosen to exercise the byte-wise carry loop in full
        let vs: Vec<Vec<u8>> = vec![
            vec![0u8; 16],
            vec![0xFFu8; 16],
            {
                let mut v = vec![0u8; 16];
                v[15] = 0xFF;
                v
            },
            {
                let mut v = vec![0u8; 16];
                v[14..16].fill(0xFF);
                v
            },
            {
                let mut v = vec![0xFFu8; 16];
                v[0] = 0x00;
                v
            },
            rng.bytes(16),
            rng.bytes(16),
        ];
        for v in vs {
            for with_data in [false, true] {
                for _ in 0..4 {
                    let key = rng.bytes(32);
                    let pd = rng.bytes(48);
                    let (mut ck, mut cv) = (key.clone(), v.clone());
                    let (mut rk, mut rv) = (key.clone(), v.clone());
                    let mut cpd = pd.clone();
                    let mut rpd = pd.clone();
                    if with_data {
                        cf(cpd.as_mut_ptr(), ck.as_mut_ptr(), cv.as_mut_ptr());
                        rf(rpd.as_mut_ptr(), rk.as_mut_ptr(), rv.as_mut_ptr());
                    } else {
                        cf(std::ptr::null_mut(), ck.as_mut_ptr(), cv.as_mut_ptr());
                        rf(std::ptr::null_mut(), rk.as_mut_ptr(), rv.as_mut_ptr());
                    }
                    let what = format!("AES256_CTR_DRBG_Update(provided_data={with_data})");
                    same(&format!("{what} Key"), &ck, &rk);
                    same(&format!("{what} V"), &cv, &rv);
                    same(&format!("{what} provided_data untouched"), &cpd, &rpd);
                }
            }
        }
    }
}

/// The whole `AES256_CTR_DRBG_struct`: `Key[32] ‖ V[16] ‖ int reseed_counter`.
const DRBG_STRUCT_BYTES: usize = 32 + 16 + 4;

#[test]
fn cfg53_randombytes_init() {
    let (l, _p) = env();
    let _g = drbg_lock();
    let mut rng = Rng::new(0x5353);
    unsafe {
        let ci: FnRandombytesInit = *l.c("randombytes_init");
        let ri: FnRandombytesInit = *l.r("randombytes_init");
        let cd = l.c_data("DRBG_ctx");
        let rd = l.r_data("DRBG_ctx");
        for with_pers in [false, true] {
            for _ in 0..16 {
                let mut e = rng.bytes(48);
                let mut ps = rng.bytes(48);
                let mut e2 = e.clone();
                let mut ps2 = ps.clone();
                if with_pers {
                    ci(e.as_mut_ptr(), ps.as_mut_ptr());
                    ri(e2.as_mut_ptr(), ps2.as_mut_ptr());
                } else {
                    ci(e.as_mut_ptr(), std::ptr::null_mut());
                    ri(e2.as_mut_ptr(), std::ptr::null_mut());
                }
                let what = format!("randombytes_init(pers={with_pers})");
                same(
                    &format!("{what} DRBG_ctx"),
                    std::slice::from_raw_parts(cd, DRBG_STRUCT_BYTES),
                    std::slice::from_raw_parts(rd, DRBG_STRUCT_BYTES),
                );
                same(&format!("{what} entropy untouched"), &e, &e2);
                same(&format!("{what} pers untouched"), &ps, &ps2);
            }
        }
    }
}

#[test]
fn cfg54_randombytes_stream() {
    let (l, _p) = env();
    let _g = drbg_lock();
    let mut rng = Rng::new(0x5454);
    unsafe {
        let ci: FnRandombytesInit = *l.c("randombytes_init");
        let ri: FnRandombytesInit = *l.r("randombytes_init");
        let cr: FnRandombytes = *l.c("randombytes");
        let rr: FnRandombytes = *l.r("randombytes");
        let cd = l.c_data("DRBG_ctx");
        let rd = l.r_data("DRBG_ctx");

        for xlen in [0usize, 1, 2, 15, 16, 17, 31, 32, 33, 47, 48, 64, 1000] {
            let mut e = rng.bytes(48);
            let mut e2 = e.clone();
            ci(e.as_mut_ptr(), std::ptr::null_mut());
            ri(e2.as_mut_ptr(), std::ptr::null_mut());
            // a chain of successive draws: state carries over, so a divergence
            // in the counter/reseed logic compounds
            for step in 0..10 {
                let mut co = vec![0xA5u8; xlen + 32];
                let mut ro = co.clone();
                let c = cr(co.as_mut_ptr(), xlen as c_ulonglong);
                let r = rr(ro.as_mut_ptr(), xlen as c_ulonglong);
                same_i(&format!("randombytes(xlen={xlen}) ret"), c, r);
                assert_eq!(c, 0, "randombytes always returns RNG_SUCCESS");
                same(&format!("randombytes(xlen={xlen}, step={step})"), &co, &ro);
                same(
                    &format!("DRBG_ctx after randombytes(xlen={xlen}, step={step})"),
                    std::slice::from_raw_parts(cd, DRBG_STRUCT_BYTES),
                    std::slice::from_raw_parts(rd, DRBG_STRUCT_BYTES),
                );
            }
        }
    }
}

/// `AES_XOF_struct` = `buffer[16] ‖ unsigned long buffer_pos ‖
/// unsigned long length_remaining ‖ key[32] ‖ ctr[16]`.
const XOF_STRUCT_BYTES: usize = 16 + 8 + 8 + 32 + 16;

#[test]
fn cfg55_seedexpander_stream() {
    let (l, _p) = env();
    let mut rng = Rng::new(0x5555);
    unsafe {
        let cinit: FnSeedexpanderInit = *l.c("seedexpander_init");
        let rinit: FnSeedexpanderInit = *l.r("seedexpander_init");
        let cse: FnSeedexpander = *l.c("seedexpander");
        let rse: FnSeedexpander = *l.r("seedexpander");

        for maxlen in [1u64, 16, 17, 256, 4096, 0xFFFF_FFFF] {
            let mut seed = rng.bytes(32);
            let mut div = rng.bytes(8);
            let mut seed2 = seed.clone();
            let mut div2 = div.clone();
            let mut cctx = vec![0xA5u8; XOF_STRUCT_BYTES];
            let mut rctx = cctx.clone();
            let c = cinit(
                cctx.as_mut_ptr(),
                seed.as_mut_ptr(),
                div.as_mut_ptr(),
                maxlen as c_ulong,
            );
            let r = rinit(
                rctx.as_mut_ptr(),
                seed2.as_mut_ptr(),
                div2.as_mut_ptr(),
                maxlen as c_ulong,
            );
            same_i(&format!("seedexpander_init(maxlen={maxlen}) ret"), c, r);
            assert_eq!(c, 0);
            same(
                &format!("seedexpander_init(maxlen={maxlen}) ctx"),
                &cctx,
                &rctx,
            );

            // draws that cross the 16-byte internal buffer boundary many times,
            // exercising ctr[12..16] carry propagation
            for (step, &xlen) in [1usize, 15, 16, 17, 1, 100, 3, 16, 31, 2]
                .iter()
                .enumerate()
            {
                let mut co = vec![0xA5u8; xlen + 32];
                let mut ro = co.clone();
                let c = cse(cctx.as_mut_ptr(), co.as_mut_ptr(), xlen as c_ulong);
                let r = rse(rctx.as_mut_ptr(), ro.as_mut_ptr(), xlen as c_ulong);
                same_i(
                    &format!("seedexpander(maxlen={maxlen}, step={step}, xlen={xlen}) ret"),
                    c,
                    r,
                );
                same(
                    &format!("seedexpander(maxlen={maxlen}, step={step}, xlen={xlen}) out"),
                    &co,
                    &ro,
                );
                same(
                    &format!("seedexpander(maxlen={maxlen}, step={step}) ctx"),
                    &cctx,
                    &rctx,
                );
            }
        }
    }
}
