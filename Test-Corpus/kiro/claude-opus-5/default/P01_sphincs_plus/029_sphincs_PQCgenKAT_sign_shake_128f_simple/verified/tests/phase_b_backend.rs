//! Phase B — valid-path differential tests for the backend-specific primitives
//! (rows C45–C60 of `CONFIGS.md`).
//!
//! Each test is a no-op under the backends it does not apply to, so the same
//! file runs unchanged under every feature combination.

mod common;

use common::*;
use std::ffi::{c_int, c_ulong};

// ===========================================================================
// blake — C45–C49
// ===========================================================================

#[test]
fn cfg_c45_blake256_incremental() {
    if !IS_BLAKE {
        return;
    }
    type Init = unsafe extern "C" fn(*mut BlakeState256);
    type Upd = unsafe extern "C" fn(*mut BlakeState256, *const u8, u64);
    type Fin = unsafe extern "C" fn(*mut BlakeState256, *mut u8);
    let (cinit, rinit) = libs().pair::<Init>("blake256_init");
    let (cupd, rupd) = libs().pair::<Upd>("blake256_update");
    let (cfin, rfin) = libs().pair::<Fin>("blake256_final");
    let mut rng = Rng::new(SEED ^ 45);

    // Chunk sequences (in *bytes*; the C API takes bits) that walk `buflen`
    // through 0, <55, 55, 56, 63, 64, >64 and multi-block updates.
    let seqs: Vec<Vec<usize>> = vec![
        vec![],
        vec![0],
        vec![1],
        vec![54],
        vec![55],
        vec![56],
        vec![63],
        vec![64],
        vec![65],
        vec![1, 1, 1],
        vec![55, 1],
        vec![54, 1, 1],
        vec![32, 32],
        vec![32, 33],
        vec![64, 64],
        vec![63, 65, 1],
        vec![100, 200, 7],
        vec![1, 63, 64, 65, 127],
    ];

    for seq in seqs {
        let mut cs = BlakeState256::zeroed();
        let mut rs = BlakeState256::zeroed();
        unsafe {
            cinit(&mut cs);
            rinit(&mut rs);
        }
        same_val(&format!("blake256_init state ({seq:?})"), cs, rs);
        for &n in &seq {
            let data = rng.bytes(n.max(1));
            unsafe {
                cupd(&mut cs, data.as_ptr(), (n * 8) as u64);
                rupd(&mut rs, data.as_ptr(), (n * 8) as u64);
            }
            same_val(&format!("blake256_update({n}) state, seq {seq:?}"), cs, rs);
        }
        let mut cd = [0xAAu8; 40];
        let mut rd = [0xAAu8; 40];
        unsafe {
            cfin(&mut cs, cd.as_mut_ptr());
            rfin(&mut rs, rd.as_mut_ptr());
        }
        same(&format!("blake256_final digest, seq {seq:?}"), &cd, &rd);
        same_val(&format!("blake256_final state, seq {seq:?}"), cs, rs);
    }
}

#[test]
fn cfg_c46_blake_one_shot() {
    if !IS_BLAKE {
        return;
    }
    type F = unsafe extern "C" fn(*mut u8, *const u8, u64) -> c_int;
    let mut rng = Rng::new(SEED ^ 46);
    for (name, outlen) in [("blake256", 32usize), ("blake512", 64)] {
        let (c, r) = libs().pair::<F>(name);
        for inlen in [
            0usize, 1, 2, 54, 55, 56, 63, 64, 65, 110, 111, 112, 119, 120, 127, 128, 129, 250, 1000,
        ] {
            let data = rng.bytes(inlen.max(1));
            let mut cd = vec![0xAAu8; outlen + 8];
            let mut rd = vec![0xAAu8; outlen + 8];
            let cv = unsafe { c(cd.as_mut_ptr(), data.as_ptr(), inlen as u64) };
            let rv = unsafe { r(rd.as_mut_ptr(), data.as_ptr(), inlen as u64) };
            same_val(&format!("{name}(inlen={inlen}) return"), cv, rv);
            same(&format!("{name}(inlen={inlen})"), &cd, &rd);
        }
    }
}

#[test]
fn cfg_c47_blake_compress() {
    if !IS_BLAKE {
        return;
    }
    let mut rng = Rng::new(SEED ^ 47);
    {
        type F = unsafe extern "C" fn(*mut BlakeState256, *const u8);
        let (c, r) = libs().pair::<F>("blake256_compress");
        for _ in 0..iters_cheap() {
            let mut cs = BlakeState256::zeroed();
            rng.fill(unsafe {
                std::slice::from_raw_parts_mut(
                    (&mut cs) as *mut BlakeState256 as *mut u8,
                    std::mem::size_of::<BlakeState256>(),
                )
            });
            // buflen/nullt must stay plausible ints
            cs.buflen = 0;
            cs.nullt = 0;
            let mut rs = cs;
            let block = rng.bytes(64);
            unsafe {
                c(&mut cs, block.as_ptr());
                r(&mut rs, block.as_ptr());
            }
            same_val("blake256_compress", cs, rs);
        }
    }
    {
        type F = unsafe extern "C" fn(*mut BlakeState512, *const u8);
        let (c, r) = libs().pair::<F>("blake512_compress");
        for _ in 0..iters_cheap() {
            let mut cs = BlakeState512::zeroed();
            rng.fill(unsafe {
                std::slice::from_raw_parts_mut(
                    (&mut cs) as *mut BlakeState512 as *mut u8,
                    std::mem::size_of::<BlakeState512>(),
                )
            });
            cs.buflen = 0;
            cs.nullt = 0;
            let mut rs = cs;
            let block = rng.bytes(128);
            unsafe {
                c(&mut cs, block.as_ptr());
                r(&mut rs, block.as_ptr());
            }
            same_val("blake512_compress", cs, rs);
        }
    }
}

#[test]
fn cfg_c48_blake_mgf1() {
    if !IS_BLAKE {
        return;
    }
    type F = unsafe extern "C" fn(*mut u8, c_ulong, *const u8, c_ulong);
    let mut rng = Rng::new(SEED ^ 48);
    for (name, dgst) in [("SPX_blake256_mgf1", 32usize), ("SPX_blake512_mgf1", 64)] {
        let (c, r) = libs().pair::<F>(name);
        for outlen in [
            0usize,
            1,
            dgst - 1,
            dgst,
            dgst + 1,
            2 * dgst - 1,
            2 * dgst,
            2 * dgst + 1,
            3 * dgst,
            200,
        ] {
            for inlen in [0usize, 1, 32, 48, 64] {
                let inp = rng.bytes(inlen.max(1));
                let mut cd = vec![0xAAu8; outlen + 8];
                let mut rd = vec![0xAAu8; outlen + 8];
                unsafe {
                    c(cd.as_mut_ptr(), outlen as c_ulong, inp.as_ptr(), inlen as c_ulong);
                    r(rd.as_mut_ptr(), outlen as c_ulong, inp.as_ptr(), inlen as c_ulong);
                }
                same(&format!("{name}(outlen={outlen}, inlen={inlen})"), &cd, &rd);
            }
        }
    }
}

#[test]
fn cfg_c49_blake512_incremental() {
    if !IS_BLAKE {
        return;
    }
    type Init = unsafe extern "C" fn(*mut BlakeState512);
    type Upd = unsafe extern "C" fn(*mut BlakeState512, *const u8, u64);
    type Fin = unsafe extern "C" fn(*mut BlakeState512, *mut u8);
    let (cinit, rinit) = libs().pair::<Init>("blake512_init");
    let (cupd, rupd) = libs().pair::<Upd>("blake512_update");
    let (cfin, rfin) = libs().pair::<Fin>("blake512_final");
    let mut rng = Rng::new(SEED ^ 49);

    let seqs: Vec<Vec<usize>> = vec![
        vec![],
        vec![0],
        vec![1],
        vec![110],
        vec![111],
        vec![112],
        vec![127],
        vec![128],
        vec![129],
        vec![1, 1, 1],
        vec![111, 1],
        vec![110, 1, 1],
        vec![64, 64],
        vec![64, 65],
        vec![128, 128],
        vec![127, 129, 1],
        vec![200, 300, 7],
    ];
    for seq in seqs {
        let mut cs = BlakeState512::zeroed();
        let mut rs = BlakeState512::zeroed();
        unsafe {
            cinit(&mut cs);
            rinit(&mut rs);
        }
        same_val(&format!("blake512_init state ({seq:?})"), cs, rs);
        for &n in &seq {
            let data = rng.bytes(n.max(1));
            unsafe {
                cupd(&mut cs, data.as_ptr(), (n * 8) as u64);
                rupd(&mut rs, data.as_ptr(), (n * 8) as u64);
            }
            same_val(&format!("blake512_update({n}) state, seq {seq:?}"), cs, rs);
        }
        let mut cd = [0xAAu8; 72];
        let mut rd = [0xAAu8; 72];
        unsafe {
            cfin(&mut cs, cd.as_mut_ptr());
            rfin(&mut rs, rd.as_mut_ptr());
        }
        same(&format!("blake512_final digest, seq {seq:?}"), &cd, &rd);
        same_val(&format!("blake512_final state, seq {seq:?}"), cs, rs);
    }
}

// ===========================================================================
// sha2 — C50–C53
// ===========================================================================

fn sha_inc_row(prefix: &str, state_bytes: usize, block: usize, out: usize, inlens: &[usize], seed: u64) {
    type Init = unsafe extern "C" fn(*mut u8);
    type Blocks = unsafe extern "C" fn(*mut u8, *const u8, usize);
    type Final = unsafe extern "C" fn(*mut u8, *mut u8, *const u8, usize);
    type OneShot = unsafe extern "C" fn(*mut u8, *const u8, usize);
    let (cinit, rinit) = libs().pair::<Init>(&format!("{prefix}_inc_init"));
    let (cblk, rblk) = libs().pair::<Blocks>(&format!("{prefix}_inc_blocks"));
    let (cfin, rfin) = libs().pair::<Final>(&format!("{prefix}_inc_finalize"));
    let (cone, rone) = libs().pair::<OneShot>(prefix);
    let mut rng = Rng::new(seed);

    for nblocks in [0usize, 1, 2, 3] {
        for &inlen in inlens {
            let mut cs = vec![0xAAu8; state_bytes];
            let mut rs = vec![0xAAu8; state_bytes];
            unsafe {
                cinit(cs.as_mut_ptr());
                rinit(rs.as_mut_ptr());
            }
            same(&format!("{prefix}_inc_init state"), &cs, &rs);

            let blk = rng.bytes((nblocks * block).max(1));
            unsafe {
                cblk(cs.as_mut_ptr(), blk.as_ptr(), nblocks);
                rblk(rs.as_mut_ptr(), blk.as_ptr(), nblocks);
            }
            same(&format!("{prefix}_inc_blocks({nblocks}) state"), &cs, &rs);

            let tail = rng.bytes(inlen.max(1));
            let mut cd = vec![0xAAu8; out + 8];
            let mut rd = vec![0xAAu8; out + 8];
            unsafe {
                cfin(cd.as_mut_ptr(), cs.as_mut_ptr(), tail.as_ptr(), inlen);
                rfin(rd.as_mut_ptr(), rs.as_mut_ptr(), tail.as_ptr(), inlen);
            }
            same(
                &format!("{prefix}_inc_finalize(nblocks={nblocks}, inlen={inlen}) digest"),
                &cd,
                &rd,
            );
            same(
                &format!("{prefix}_inc_finalize(nblocks={nblocks}, inlen={inlen}) state"),
                &cs,
                &rs,
            );
        }
    }

    for &inlen in inlens {
        let data = rng.bytes(inlen.max(1));
        let mut cd = vec![0xAAu8; out + 8];
        let mut rd = vec![0xAAu8; out + 8];
        unsafe {
            cone(cd.as_mut_ptr(), data.as_ptr(), inlen);
            rone(rd.as_mut_ptr(), data.as_ptr(), inlen);
        }
        same(&format!("{prefix}(inlen={inlen})"), &cd, &rd);
    }
}

#[test]
fn cfg_c50_sha256() {
    if !IS_SHA2 {
        return;
    }
    sha_inc_row(
        "sha256",
        40,
        64,
        32,
        &[0, 1, 54, 55, 56, 63, 64, 65, 119, 120, 128, 129, 1000],
        SEED ^ 50,
    );
}

#[test]
fn cfg_c51_sha512() {
    if !IS_SHA2 {
        return;
    }
    sha_inc_row(
        "sha512",
        72,
        128,
        64,
        &[0, 1, 110, 111, 112, 127, 128, 129, 239, 240, 256, 257, 1000],
        SEED ^ 51,
    );
}

#[test]
fn cfg_c52_sha2_mgf1() {
    if !IS_SHA2 {
        return;
    }
    type F = unsafe extern "C" fn(*mut u8, c_ulong, *const u8, c_ulong);
    let mut rng = Rng::new(SEED ^ 52);
    for (name, dgst) in [("SPX_mgf1_256", 32usize), ("SPX_mgf1_512", 64)] {
        let (c, r) = libs().pair::<F>(name);
        for outlen in [
            0usize,
            1,
            dgst - 1,
            dgst,
            dgst + 1,
            2 * dgst - 1,
            2 * dgst,
            2 * dgst + 1,
            3 * dgst,
            200,
        ] {
            for inlen in [0usize, 1, 32, 48, 64] {
                let inp = rng.bytes(inlen.max(1));
                let mut cd = vec![0xAAu8; outlen + 8];
                let mut rd = vec![0xAAu8; outlen + 8];
                unsafe {
                    c(cd.as_mut_ptr(), outlen as c_ulong, inp.as_ptr(), inlen as c_ulong);
                    r(rd.as_mut_ptr(), outlen as c_ulong, inp.as_ptr(), inlen as c_ulong);
                }
                same(&format!("{name}(outlen={outlen}, inlen={inlen})"), &cd, &rd);
            }
        }
    }
}

#[test]
fn cfg_c53_sha2_seed_state() {
    if !IS_SHA2 {
        return;
    }
    type F = unsafe extern "C" fn(*mut u8);
    let (c, r) = libs().pair::<F>("SPX_seed_state");
    let mut rng = Rng::new(SEED ^ 53);
    for _ in 0..iters_cheap() {
        let pub_seed = rng.bytes(SPX_N);
        let sk_seed = rng.bytes(SPX_N);
        let mut cc = Ctx::with_seeds(&pub_seed, &sk_seed);
        let mut rc = Ctx::with_seeds(&pub_seed, &sk_seed);
        unsafe {
            c(cc.as_mut_ptr());
            r(rc.as_mut_ptr());
        }
        same("seed_state -> spx_ctx", &cc.0, &rc.0);
    }
}

// ===========================================================================
// shake — C54–C56
// ===========================================================================

#[test]
fn cfg_c54_shake256_incremental() {
    if !IS_SHAKE {
        return;
    }
    type Init = unsafe extern "C" fn(*mut u64);
    type Absorb = unsafe extern "C" fn(*mut u64, *const u8, usize);
    type Fin = unsafe extern "C" fn(*mut u64);
    type Squeeze = unsafe extern "C" fn(*mut u8, usize, *mut u64);
    let (cinit, rinit) = libs().pair::<Init>("shake256_inc_init");
    let (cabs, rabs) = libs().pair::<Absorb>("shake256_inc_absorb");
    let (cfin, rfin) = libs().pair::<Fin>("shake256_inc_finalize");
    let (csq, rsq) = libs().pair::<Squeeze>("shake256_inc_squeeze");
    let mut rng = Rng::new(SEED ^ 54);

    let absorb_seqs: Vec<Vec<usize>> = vec![
        vec![],
        vec![0],
        vec![1],
        vec![135],
        vec![136],
        vec![137],
        vec![1, 135],
        vec![135, 1],
        vec![136, 136],
        vec![100, 100, 100],
        vec![271],
        vec![272],
        vec![1, 0, 1],
    ];
    let squeeze_seqs: Vec<Vec<usize>> = vec![
        vec![1],
        vec![32],
        vec![135],
        vec![136],
        vec![137],
        vec![1, 1, 1],
        vec![135, 1, 136],
        vec![272],
    ];

    for aseq in &absorb_seqs {
        for sseq in &squeeze_seqs {
            let mut cs = [0u64; 26];
            let mut rs = [0u64; 26];
            unsafe {
                cinit(cs.as_mut_ptr());
                rinit(rs.as_mut_ptr());
            }
            same_val("shake256_inc_init state", cs, rs);
            for &n in aseq {
                let data = rng.bytes(n.max(1));
                unsafe {
                    cabs(cs.as_mut_ptr(), data.as_ptr(), n);
                    rabs(rs.as_mut_ptr(), data.as_ptr(), n);
                }
                same_val(&format!("shake256_inc_absorb({n}) state"), cs, rs);
            }
            unsafe {
                cfin(cs.as_mut_ptr());
                rfin(rs.as_mut_ptr());
            }
            same_val("shake256_inc_finalize state", cs, rs);
            for &n in sseq {
                let mut cd = vec![0xAAu8; n + 8];
                let mut rd = vec![0xAAu8; n + 8];
                unsafe {
                    csq(cd.as_mut_ptr(), n, cs.as_mut_ptr());
                    rsq(rd.as_mut_ptr(), n, rs.as_mut_ptr());
                }
                same(
                    &format!("shake256_inc_squeeze({n}) out (absorb {aseq:?})"),
                    &cd,
                    &rd,
                );
                same_val("shake256_inc_squeeze state", cs, rs);
            }
        }
    }
}

#[test]
fn cfg_c55_shake256_absorb_squeezeblocks() {
    if !IS_SHAKE {
        return;
    }
    type Absorb = unsafe extern "C" fn(*mut u64, *const u8, usize);
    type Squeeze = unsafe extern "C" fn(*mut u8, usize, *mut u64);
    let (cabs, rabs) = libs().pair::<Absorb>("shake256_absorb");
    let (csq, rsq) = libs().pair::<Squeeze>("shake256_squeezeblocks");
    let mut rng = Rng::new(SEED ^ 55);

    for inlen in [0usize, 1, 135, 136, 137, 271, 272, 500] {
        for nblocks in [1usize, 2, 3] {
            let data = rng.bytes(inlen.max(1));
            let mut cs = [0u64; 25];
            let mut rs = [0u64; 25];
            unsafe {
                cabs(cs.as_mut_ptr(), data.as_ptr(), inlen);
                rabs(rs.as_mut_ptr(), data.as_ptr(), inlen);
            }
            same_val(&format!("shake256_absorb({inlen}) state"), cs, rs);
            let mut cd = vec![0xAAu8; nblocks * 136 + 8];
            let mut rd = vec![0xAAu8; nblocks * 136 + 8];
            unsafe {
                csq(cd.as_mut_ptr(), nblocks, cs.as_mut_ptr());
                rsq(rd.as_mut_ptr(), nblocks, rs.as_mut_ptr());
            }
            same(
                &format!("shake256_squeezeblocks(inlen={inlen}, nblocks={nblocks})"),
                &cd,
                &rd,
            );
            same_val("shake256_squeezeblocks state", cs, rs);
        }
    }
}

#[test]
fn cfg_c56_shake256_one_shot() {
    if !IS_SHAKE {
        return;
    }
    type F = unsafe extern "C" fn(*mut u8, usize, *const u8, usize);
    let (c, r) = libs().pair::<F>("shake256");
    let mut rng = Rng::new(SEED ^ 56);
    for outlen in [0usize, 1, 32, 135, 136, 137, 271, 272, 1000] {
        for inlen in [0usize, 1, 135, 136, 137, 272, 1000] {
            let data = rng.bytes(inlen.max(1));
            let mut cd = vec![0xAAu8; outlen + 8];
            let mut rd = vec![0xAAu8; outlen + 8];
            unsafe {
                c(cd.as_mut_ptr(), outlen, data.as_ptr(), inlen);
                r(rd.as_mut_ptr(), outlen, data.as_ptr(), inlen);
            }
            same(&format!("shake256(outlen={outlen}, inlen={inlen})"), &cd, &rd);
        }
    }
}

// ===========================================================================
// haraka — C57–C60
// ===========================================================================

fn tweaked_ctx_pair(rng: &mut Rng) -> (Ctx, Ctx) {
    type F = unsafe extern "C" fn(*mut u8);
    let (c, r) = libs().pair::<F>("SPX_tweak_constants");
    let pub_seed = rng.bytes(SPX_N);
    let sk_seed = rng.bytes(SPX_N);
    let mut cc = Ctx::with_seeds(&pub_seed, &sk_seed);
    let mut rc = Ctx::with_seeds(&pub_seed, &sk_seed);
    unsafe {
        c(cc.as_mut_ptr());
        r(rc.as_mut_ptr());
    }
    (cc, rc)
}

#[test]
fn cfg_c57_haraka_tweak_constants() {
    if !IS_HARAKA {
        return;
    }
    let mut rng = Rng::new(SEED ^ 57);
    for _ in 0..iters_cheap() {
        let (cc, rc) = tweaked_ctx_pair(&mut rng);
        same("tweak_constants -> spx_ctx", &cc.0, &rc.0);
    }
}

#[test]
fn cfg_c58_haraka_permutations() {
    if !IS_HARAKA {
        return;
    }
    type F = unsafe extern "C" fn(*mut u8, *const u8, *const u8);
    let mut rng = Rng::new(SEED ^ 58);
    for (name, inlen, outlen) in [
        ("SPX_haraka512_perm", 64usize, 64usize),
        ("SPX_haraka512", 64, 32),
        ("SPX_haraka256", 32, 32),
    ] {
        let (c, r) = libs().pair::<F>(name);
        for _ in 0..iters_cheap() {
            let (cc, rc) = tweaked_ctx_pair(&mut rng);
            let inp = rng.bytes(inlen);
            let mut cd = vec![0xAAu8; outlen + 8];
            let mut rd = vec![0xAAu8; outlen + 8];
            unsafe {
                c(cd.as_mut_ptr(), inp.as_ptr(), cc.as_ptr());
                r(rd.as_mut_ptr(), inp.as_ptr(), rc.as_ptr());
            }
            same(name, &cd, &rd);
        }
    }
}

#[test]
fn cfg_c59_haraka_sponge_incremental() {
    if !IS_HARAKA {
        return;
    }
    type Init = unsafe extern "C" fn(*mut u8);
    type Absorb = unsafe extern "C" fn(*mut u8, *const u8, usize, *const u8);
    type Fin = unsafe extern "C" fn(*mut u8);
    type Squeeze = unsafe extern "C" fn(*mut u8, usize, *mut u8, *const u8);
    let (cinit, rinit) = libs().pair::<Init>("SPX_haraka_S_inc_init");
    let (cabs, rabs) = libs().pair::<Absorb>("SPX_haraka_S_inc_absorb");
    let (cfin, rfin) = libs().pair::<Fin>("SPX_haraka_S_inc_finalize");
    let (csq, rsq) = libs().pair::<Squeeze>("SPX_haraka_S_inc_squeeze");
    let mut rng = Rng::new(SEED ^ 59);

    let absorb_seqs: Vec<Vec<usize>> = vec![
        vec![],
        vec![0],
        vec![1],
        vec![31],
        vec![32],
        vec![33],
        vec![1, 31],
        vec![31, 1],
        vec![32, 32],
        vec![16, 16, 16],
        vec![64],
        vec![65],
    ];
    let squeeze_seqs: Vec<Vec<usize>> = vec![
        vec![1],
        vec![16],
        vec![31],
        vec![32],
        vec![33],
        vec![1, 1, 1],
        vec![31, 1, 32],
        vec![64],
    ];

    for aseq in &absorb_seqs {
        for sseq in &squeeze_seqs {
            let (cc, rc) = tweaked_ctx_pair(&mut rng);
            let mut cs = [0u8; 65];
            let mut rs = [0u8; 65];
            unsafe {
                cinit(cs.as_mut_ptr());
                rinit(rs.as_mut_ptr());
            }
            same("haraka_S_inc_init state", &cs, &rs);
            for &n in aseq {
                let data = rng.bytes(n.max(1));
                unsafe {
                    cabs(cs.as_mut_ptr(), data.as_ptr(), n, cc.as_ptr());
                    rabs(rs.as_mut_ptr(), data.as_ptr(), n, rc.as_ptr());
                }
                same(&format!("haraka_S_inc_absorb({n}) state"), &cs, &rs);
            }
            unsafe {
                cfin(cs.as_mut_ptr());
                rfin(rs.as_mut_ptr());
            }
            same("haraka_S_inc_finalize state", &cs, &rs);
            for &n in sseq {
                let mut cd = vec![0xAAu8; n + 8];
                let mut rd = vec![0xAAu8; n + 8];
                unsafe {
                    csq(cd.as_mut_ptr(), n, cs.as_mut_ptr(), cc.as_ptr());
                    rsq(rd.as_mut_ptr(), n, rs.as_mut_ptr(), rc.as_ptr());
                }
                same(
                    &format!("haraka_S_inc_squeeze({n}) out (absorb {aseq:?})"),
                    &cd,
                    &rd,
                );
                same("haraka_S_inc_squeeze state", &cs, &rs);
            }
        }
    }
}

#[test]
fn cfg_c60_haraka_s() {
    if !IS_HARAKA {
        return;
    }
    type F = unsafe extern "C" fn(*mut u8, u64, *const u8, u64, *const u8);
    let (c, r) = libs().pair::<F>("SPX_haraka_S");
    let mut rng = Rng::new(SEED ^ 60);
    for outlen in [0usize, 1, 16, 31, 32, 33, 64, 100] {
        for inlen in [0usize, 1, 31, 32, 33, 64, 100] {
            let (cc, rc) = tweaked_ctx_pair(&mut rng);
            let data = rng.bytes(inlen.max(1));
            let mut cd = vec![0xAAu8; outlen + 8];
            let mut rd = vec![0xAAu8; outlen + 8];
            unsafe {
                c(
                    cd.as_mut_ptr(),
                    outlen as u64,
                    data.as_ptr(),
                    inlen as u64,
                    cc.as_ptr(),
                );
                r(
                    rd.as_mut_ptr(),
                    outlen as u64,
                    data.as_ptr(),
                    inlen as u64,
                    rc.as_ptr(),
                );
            }
            same(&format!("haraka_S(outlen={outlen}, inlen={inlen})"), &cd, &rd);
        }
    }
}
