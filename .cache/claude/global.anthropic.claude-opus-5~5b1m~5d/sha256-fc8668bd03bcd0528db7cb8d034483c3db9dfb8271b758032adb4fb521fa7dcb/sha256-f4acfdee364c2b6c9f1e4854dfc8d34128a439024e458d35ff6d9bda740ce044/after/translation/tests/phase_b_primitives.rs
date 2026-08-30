//! Phase B rows 47-50: the hash primitives of the active backend, driven
//! directly (lowest-level public entry points).
mod common;
use common::*;

fn len_sweep() -> Vec<usize> {
    let mut v: Vec<usize> = (0..=8).collect();
    v.extend_from_slice(&[
        15, 16, 17, 31, 32, 33, 54, 55, 56, 57, 63, 64, 65, 71, 72, 73, 95, 96, 97, 110, 111, 112,
        113, 119, 120, 121, 127, 128, 129, 135, 136, 137, 167, 168, 169, 191, 192, 193, 255, 256,
        257, 271, 272, 273, 383, 384, 385, 1000,
    ]);
    v
}

// =====================================================================
// row 47 — blake
// =====================================================================
#[cfg(spx_backend = "blake")]
mod blake {
    use super::*;

    /// `blakestate256` = { u32 h[8], s[4], t[2]; int buflen, nullt; u8 buf[64]; }
    const ST256: usize = 8 * 4 + 4 * 4 + 2 * 4 + 4 + 4 + 64;
    /// `blakestate512` = { u64 h[8], s[4], t[2]; int buflen, nullt; u8 buf[128]; }
    const ST512: usize = 8 * 8 + 4 * 8 + 2 * 8 + 4 + 4 + 128;

    type FOneShot = unsafe extern "C" fn(*mut u8, *const u8, u64) -> i32;
    type FInit = unsafe extern "C" fn(*mut u8);
    type FUpdate = unsafe extern "C" fn(*mut u8, *const u8, u64);
    type FFinal = unsafe extern "C" fn(*mut u8, *mut u8);
    type FCompress = unsafe extern "C" fn(*mut u8, *const u8);
    type FMgf1 = unsafe extern "C" fn(*mut u8, u64, *const u8, u64);

    #[test]
    fn b47a_blake_oneshot() {
        let p = pair();
        let mut rng = Rng::new(SEED ^ 470);
        unsafe {
            let c2 = sym!(p.c, b"blake256\0", FOneShot);
            let r2 = sym!(p.r, b"blake256\0", FOneShot);
            let c5 = sym!(p.c, b"blake512\0", FOneShot);
            let r5 = sym!(p.r, b"blake512\0", FOneShot);
            for inlen in len_sweep() {
                let m = rng.bytes(inlen.max(1));
                let mut co = obuf(32);
                let mut ro = obuf(32);
                let a = c2(co.as_mut_ptr(), m.as_ptr(), inlen as u64);
                let b = r2(ro.as_mut_ptr(), m.as_ptr(), inlen as u64);
                eqv(&format!("blake256 ret inlen={inlen}"), a, b);
                eqb(&format!("blake256 inlen={inlen}"), &co, &ro);

                let mut co = obuf(64);
                let mut ro = obuf(64);
                let a = c5(co.as_mut_ptr(), m.as_ptr(), inlen as u64);
                let b = r5(ro.as_mut_ptr(), m.as_ptr(), inlen as u64);
                eqv(&format!("blake512 ret inlen={inlen}"), a, b);
                eqb(&format!("blake512 inlen={inlen}"), &co, &ro);
            }
        }
    }

    #[test]
    fn b47b_blake_incremental() {
        let p = pair();
        let mut rng = Rng::new(SEED ^ 471);
        unsafe {
            for (init, upd, fin, stlen, outlen) in [
                (&b"blake256_init\0"[..], &b"blake256_update\0"[..], &b"blake256_final\0"[..], ST256, 32usize),
                (&b"blake512_init\0"[..], &b"blake512_update\0"[..], &b"blake512_final\0"[..], ST512, 64usize),
            ] {
                let ci = sym!(p.c, init, FInit);
                let ri = sym!(p.r, init, FInit);
                let cu = sym!(p.c, upd, FUpdate);
                let ru = sym!(p.r, upd, FUpdate);
                let cf = sym!(p.c, fin, FFinal);
                let rf = sym!(p.r, fin, FFinal);

                // (a) `blakeX(...)`-style usage: one update with a BIT length.
                for inlen in len_sweep() {
                    let m = rng.bytes(inlen.max(1));
                    let mut cs = vec![0x77u8; stlen];
                    let mut rs = vec![0x77u8; stlen];
                    ci(cs.as_mut_ptr());
                    ri(rs.as_mut_ptr());
                    eqb("blake state after init", &cs, &rs);
                    cu(cs.as_mut_ptr(), m.as_ptr(), (inlen * 8) as u64);
                    ru(rs.as_mut_ptr(), m.as_ptr(), (inlen * 8) as u64);
                    eqb(&format!("blake state after update bits inlen={inlen}"), &cs, &rs);
                    let mut co = obuf(outlen);
                    let mut ro = obuf(outlen);
                    cf(cs.as_mut_ptr(), co.as_mut_ptr());
                    rf(rs.as_mut_ptr(), ro.as_mut_ptr());
                    eqb(&format!("blake digest bits inlen={inlen}"), &co, &ro);
                    eqb(&format!("blake state after final inlen={inlen}"), &cs, &rs);
                }

                // (b) the `hash_blake.c` usage: multiple updates with BYTE
                //     counts passed as the "bit" length. This is what the
                //     SPHINCS+ layer actually does, so it must match exactly.
                for parts in [
                    vec![N, N, 0usize],
                    vec![N, N, 1],
                    vec![N, N, 40],
                    vec![N, PK_BYTES, 0],
                    vec![N, PK_BYTES, 137],
                    vec![8, 8, 8, 8],
                    vec![64, 64],
                    vec![128, 128],
                ] {
                    let mut cs = vec![0u8; stlen];
                    let mut rs = vec![0u8; stlen];
                    ci(cs.as_mut_ptr());
                    ri(rs.as_mut_ptr());
                    for len in &parts {
                        let m = rng.bytes((*len).max(1));
                        cu(cs.as_mut_ptr(), m.as_ptr(), *len as u64);
                        ru(rs.as_mut_ptr(), m.as_ptr(), *len as u64);
                        eqb(&format!("blake state after byte-count update {len}"), &cs, &rs);
                    }
                    let mut co = obuf(outlen);
                    let mut ro = obuf(outlen);
                    cf(cs.as_mut_ptr(), co.as_mut_ptr());
                    rf(rs.as_mut_ptr(), ro.as_mut_ptr());
                    eqb(&format!("blake digest byte-count parts={parts:?}"), &co, &ro);
                }
            }
        }
    }

    #[test]
    fn b47c_blake_compress() {
        let p = pair();
        let mut rng = Rng::new(SEED ^ 472);
        unsafe {
            let ci = sym!(p.c, b"blake256_init\0", FInit);
            let ri = sym!(p.r, b"blake256_init\0", FInit);
            let cc = sym!(p.c, b"blake256_compress\0", FCompress);
            let rc = sym!(p.r, b"blake256_compress\0", FCompress);
            for _ in 0..64 {
                let mut cs = vec![0u8; ST256];
                let mut rs = vec![0u8; ST256];
                ci(cs.as_mut_ptr());
                ri(rs.as_mut_ptr());
                let blk = rng.bytes(64);
                cc(cs.as_mut_ptr(), blk.as_ptr());
                rc(rs.as_mut_ptr(), blk.as_ptr());
                eqb("blake256_compress state", &cs, &rs);
            }
            let ci = sym!(p.c, b"blake512_init\0", FInit);
            let ri = sym!(p.r, b"blake512_init\0", FInit);
            let cc = sym!(p.c, b"blake512_compress\0", FCompress);
            let rc = sym!(p.r, b"blake512_compress\0", FCompress);
            for _ in 0..64 {
                let mut cs = vec![0u8; ST512];
                let mut rs = vec![0u8; ST512];
                ci(cs.as_mut_ptr());
                ri(rs.as_mut_ptr());
                let blk = rng.bytes(128);
                cc(cs.as_mut_ptr(), blk.as_ptr());
                rc(rs.as_mut_ptr(), blk.as_ptr());
                eqb("blake512_compress state", &cs, &rs);
            }
        }
    }

    #[test]
    fn b47d_blake_mgf1_and_cst() {
        let p = pair();
        let mut rng = Rng::new(SEED ^ 473);
        unsafe {
            for name in [&b"SPX_blake256_mgf1\0"[..], &b"SPX_blake512_mgf1\0"[..]] {
                let cf = sym!(p.c, name, FMgf1);
                let rf = sym!(p.r, name, FMgf1);
                for inlen in [1usize, 4, 16, 32, 48, 64, 80, 96, 128, 200] {
                    for outlen in [1usize, 2, 31, 32, 33, 63, 64, 65, 127, 128, 129, 200] {
                        let m = rng.bytes(inlen);
                        let mut co = obuf(outlen);
                        let mut ro = obuf(outlen);
                        cf(co.as_mut_ptr(), outlen as u64, m.as_ptr(), inlen as u64);
                        rf(ro.as_mut_ptr(), outlen as u64, m.as_ptr(), inlen as u64);
                        eqb(
                            &format!("{} in={inlen} out={outlen}", String::from_utf8_lossy(name)),
                            &co,
                            &ro,
                        );
                    }
                }
            }
            // the exported `cst` data symbol (const u64 cst[16])
            let cp = data_ptr(&p.c, b"cst\0");
            let rp = data_ptr(&p.r, b"cst\0");
            eqb(
                "cst",
                core::slice::from_raw_parts(cp, 128),
                core::slice::from_raw_parts(rp, 128),
            );
        }
    }
}

// =====================================================================
// row 48 — sha2
// =====================================================================
#[cfg(spx_backend = "sha2")]
mod sha2 {
    use super::*;

    type FSha = unsafe extern "C" fn(*mut u8, *const u8, usize);
    type FIncInit = unsafe extern "C" fn(*mut u8);
    type FIncBlocks = unsafe extern "C" fn(*mut u8, *const u8, usize);
    type FIncFinal = unsafe extern "C" fn(*mut u8, *mut u8, *const u8, usize);
    type FMgf1 = unsafe extern "C" fn(*mut u8, u64, *const u8, u64);
    type FSeedState = unsafe extern "C" fn(*mut u8);

    #[test]
    fn b48a_sha_oneshot() {
        let p = pair();
        let mut rng = Rng::new(SEED ^ 480);
        unsafe {
            for (name, outlen) in [(&b"sha256\0"[..], 32usize), (&b"sha512\0"[..], 64)] {
                let cf = sym!(p.c, name, FSha);
                let rf = sym!(p.r, name, FSha);
                for inlen in len_sweep() {
                    let m = rng.bytes(inlen.max(1));
                    let mut co = obuf(outlen);
                    let mut ro = obuf(outlen);
                    cf(co.as_mut_ptr(), m.as_ptr(), inlen);
                    rf(ro.as_mut_ptr(), m.as_ptr(), inlen);
                    eqb(
                        &format!("{} inlen={inlen}", String::from_utf8_lossy(name)),
                        &co,
                        &ro,
                    );
                }
            }
        }
    }

    #[test]
    fn b48b_sha_incremental() {
        let p = pair();
        let mut rng = Rng::new(SEED ^ 481);
        unsafe {
            for (init, blocks, fin, stlen, blk, outlen) in [
                (&b"sha256_inc_init\0"[..], &b"sha256_inc_blocks\0"[..], &b"sha256_inc_finalize\0"[..], 40usize, 64usize, 32usize),
                (&b"sha512_inc_init\0"[..], &b"sha512_inc_blocks\0"[..], &b"sha512_inc_finalize\0"[..], 72, 128, 64),
            ] {
                let ci = sym!(p.c, init, FIncInit);
                let ri = sym!(p.r, init, FIncInit);
                let cb = sym!(p.c, blocks, FIncBlocks);
                let rb = sym!(p.r, blocks, FIncBlocks);
                let cf = sym!(p.c, fin, FIncFinal);
                let rf = sym!(p.r, fin, FIncFinal);

                for nblocks in [0usize, 1, 2, 3] {
                    for tail in [0usize, 1, blk - 10, blk - 9, blk - 8, blk - 1, blk, blk + 1, 2 * blk + 3] {
                        let mut cs = vec![0x33u8; stlen + SLACK];
                        let mut rs = vec![0x33u8; stlen + SLACK];
                        ci(cs.as_mut_ptr());
                        ri(rs.as_mut_ptr());
                        eqb("sha state after init", &cs, &rs);
                        if nblocks > 0 {
                            let m = rng.bytes(nblocks * blk);
                            cb(cs.as_mut_ptr(), m.as_ptr(), nblocks);
                            rb(rs.as_mut_ptr(), m.as_ptr(), nblocks);
                            eqb(&format!("sha state after {nblocks} blocks"), &cs, &rs);
                        }
                        let t = rng.bytes(tail.max(1));
                        let mut co = obuf(outlen);
                        let mut ro = obuf(outlen);
                        cf(co.as_mut_ptr(), cs.as_mut_ptr(), t.as_ptr(), tail);
                        rf(ro.as_mut_ptr(), rs.as_mut_ptr(), t.as_ptr(), tail);
                        eqb(&format!("sha digest nblocks={nblocks} tail={tail}"), &co, &ro);
                        eqb(&format!("sha state after finalize nblocks={nblocks} tail={tail}"), &cs, &rs);
                    }
                }
            }
        }
    }

    #[test]
    fn b48c_mgf1_and_seed_state() {
        let p = pair();
        let mut rng = Rng::new(SEED ^ 482);
        unsafe {
            for name in [&b"SPX_mgf1_256\0"[..], &b"SPX_mgf1_512\0"[..]] {
                let cf = sym!(p.c, name, FMgf1);
                let rf = sym!(p.r, name, FMgf1);
                for inlen in [1usize, 4, 16, 32, 48, 64, 96, 128, 200] {
                    for outlen in [1usize, 2, 31, 32, 33, 63, 64, 65, 127, 128, 129, 200] {
                        let m = rng.bytes(inlen);
                        let mut co = obuf(outlen);
                        let mut ro = obuf(outlen);
                        cf(co.as_mut_ptr(), outlen as u64, m.as_ptr(), inlen as u64);
                        rf(ro.as_mut_ptr(), outlen as u64, m.as_ptr(), inlen as u64);
                        eqb(
                            &format!("{} in={inlen} out={outlen}", String::from_utf8_lossy(name)),
                            &co,
                            &ro,
                        );
                    }
                }
            }
            // seed_state fills ctx->state_seeded(+_512) from pub_seed
            let cf = sym!(p.c, b"SPX_seed_state\0", FSeedState);
            let rf = sym!(p.r, b"SPX_seed_state\0", FSeedState);
            for _ in 0..32 {
                let ps = rng.bytes(N);
                let ss = rng.bytes(N);
                let mut cc = vec![0u8; CTX_BYTES];
                let mut rc = vec![0u8; CTX_BYTES];
                cc[..N].copy_from_slice(&ps);
                cc[N..2 * N].copy_from_slice(&ss);
                rc.copy_from_slice(&cc);
                cf(cc.as_mut_ptr());
                rf(rc.as_mut_ptr());
                eqb("seed_state ctx", &cc, &rc);
            }
        }
    }
}

// =====================================================================
// row 49 — shake
// =====================================================================
#[cfg(spx_backend = "shake")]
mod shake {
    use super::*;

    type FShake = unsafe extern "C" fn(*mut u8, usize, *const u8, usize);
    type FAbsorb = unsafe extern "C" fn(*mut u64, *const u8, usize);
    type FSqueezeBlocks = unsafe extern "C" fn(*mut u8, usize, *mut u64);
    type FIncInit = unsafe extern "C" fn(*mut u64);
    type FIncAbsorb = unsafe extern "C" fn(*mut u64, *const u8, usize);
    type FIncFinalize = unsafe extern "C" fn(*mut u64);
    type FIncSqueeze = unsafe extern "C" fn(*mut u8, usize, *mut u64);

    const RATE: usize = 136;

    #[test]
    fn b49a_shake256_oneshot() {
        let p = pair();
        let mut rng = Rng::new(SEED ^ 490);
        unsafe {
            let cf = sym!(p.c, b"shake256\0", FShake);
            let rf = sym!(p.r, b"shake256\0", FShake);
            for inlen in len_sweep() {
                for outlen in [0usize, 1, 32, 135, 136, 137, 271, 272, 273] {
                    let m = rng.bytes(inlen.max(1));
                    let mut co = obuf(outlen);
                    let mut ro = obuf(outlen);
                    cf(co.as_mut_ptr(), outlen, m.as_ptr(), inlen);
                    rf(ro.as_mut_ptr(), outlen, m.as_ptr(), inlen);
                    eqb(&format!("shake256 in={inlen} out={outlen}"), &co, &ro);
                }
            }
        }
    }

    #[test]
    fn b49b_shake256_absorb_squeeze() {
        let p = pair();
        let mut rng = Rng::new(SEED ^ 491);
        unsafe {
            let ca = sym!(p.c, b"shake256_absorb\0", FAbsorb);
            let ra = sym!(p.r, b"shake256_absorb\0", FAbsorb);
            let cs = sym!(p.c, b"shake256_squeezeblocks\0", FSqueezeBlocks);
            let rs = sym!(p.r, b"shake256_squeezeblocks\0", FSqueezeBlocks);
            for inlen in [0usize, 1, 135, 136, 137, 271, 272, 273, 500] {
                for nblocks in [0usize, 1, 2, 3] {
                    let m = rng.bytes(inlen.max(1));
                    let mut cst = vec![0u64; 25 + 8];
                    let mut rst = vec![0u64; 25 + 8];
                    ca(cst.as_mut_ptr(), m.as_ptr(), inlen);
                    ra(rst.as_mut_ptr(), m.as_ptr(), inlen);
                    eqv(&format!("shake256_absorb state in={inlen}"), &cst, &rst);
                    let mut co = obuf(nblocks * RATE);
                    let mut ro = obuf(nblocks * RATE);
                    cs(co.as_mut_ptr(), nblocks, cst.as_mut_ptr());
                    rs(ro.as_mut_ptr(), nblocks, rst.as_mut_ptr());
                    eqb(&format!("squeezeblocks out in={inlen} n={nblocks}"), &co, &ro);
                    eqv(&format!("squeezeblocks state in={inlen} n={nblocks}"), &cst, &rst);
                }
            }
        }
    }

    #[test]
    fn b49c_shake256_incremental() {
        let p = pair();
        let mut rng = Rng::new(SEED ^ 492);
        unsafe {
            let ci = sym!(p.c, b"shake256_inc_init\0", FIncInit);
            let ri = sym!(p.r, b"shake256_inc_init\0", FIncInit);
            let ca = sym!(p.c, b"shake256_inc_absorb\0", FIncAbsorb);
            let ra = sym!(p.r, b"shake256_inc_absorb\0", FIncAbsorb);
            let cfz = sym!(p.c, b"shake256_inc_finalize\0", FIncFinalize);
            let rfz = sym!(p.r, b"shake256_inc_finalize\0", FIncFinalize);
            let cq = sym!(p.c, b"shake256_inc_squeeze\0", FIncSqueeze);
            let rq = sym!(p.r, b"shake256_inc_squeeze\0", FIncSqueeze);

            for parts in [
                vec![0usize],
                vec![1],
                vec![N, N, 0],
                vec![N, N, 1],
                vec![N, PK_BYTES, 137],
                vec![135, 1],
                vec![136, 1],
                vec![1, 135],
                vec![100, 100, 100],
                vec![272, 3],
            ] {
                let mut cst = vec![0u64; 26 + 8];
                let mut rst = vec![0u64; 26 + 8];
                ci(cst.as_mut_ptr());
                ri(rst.as_mut_ptr());
                eqv("inc state after init", &cst, &rst);
                for len in &parts {
                    let m = rng.bytes((*len).max(1));
                    ca(cst.as_mut_ptr(), m.as_ptr(), *len);
                    ra(rst.as_mut_ptr(), m.as_ptr(), *len);
                    eqv(&format!("inc state after absorb {len}"), &cst, &rst);
                }
                cfz(cst.as_mut_ptr());
                rfz(rst.as_mut_ptr());
                eqv("inc state after finalize", &cst, &rst);
                // split squeezes across the rate boundary
                for outlen in [1usize, 31, 32, 104, 136, 137, 200] {
                    let mut co = obuf(outlen);
                    let mut ro = obuf(outlen);
                    cq(co.as_mut_ptr(), outlen, cst.as_mut_ptr());
                    rq(ro.as_mut_ptr(), outlen, rst.as_mut_ptr());
                    eqb(&format!("inc squeeze out={outlen} parts={parts:?}"), &co, &ro);
                    eqv(&format!("inc state after squeeze {outlen}"), &cst, &rst);
                }
            }
        }
    }
}

// =====================================================================
// row 50 — haraka
// =====================================================================
#[cfg(spx_backend = "haraka")]
mod haraka {
    use super::*;

    type FTweak = unsafe extern "C" fn(*mut u8);
    type FPerm = unsafe extern "C" fn(*mut u8, *const u8, *const u8);
    type FS = unsafe extern "C" fn(*mut u8, u64, *const u8, u64, *const u8);
    type FIncInit = unsafe extern "C" fn(*mut u8);
    type FIncAbsorb = unsafe extern "C" fn(*mut u8, *const u8, usize, *const u8);
    type FIncFinalize = unsafe extern "C" fn(*mut u8);
    type FIncSqueeze = unsafe extern "C" fn(*mut u8, usize, *mut u8, *const u8);

    #[test]
    fn b50a_tweak_constants() {
        let p = pair();
        let mut rng = Rng::new(SEED ^ 500);
        unsafe {
            let cf = sym!(p.c, b"SPX_tweak_constants\0", FTweak);
            let rf = sym!(p.r, b"SPX_tweak_constants\0", FTweak);
            for _ in 0..32 {
                let ps = rng.bytes(N);
                let ss = rng.bytes(N);
                let mut cc = vec![0u8; CTX_BYTES];
                let mut rc = vec![0u8; CTX_BYTES];
                cc[..N].copy_from_slice(&ps);
                cc[N..2 * N].copy_from_slice(&ss);
                rc.copy_from_slice(&cc);
                cf(cc.as_mut_ptr());
                rf(rc.as_mut_ptr());
                eqb("tweak_constants ctx", &cc, &rc);
            }
        }
    }

    #[test]
    fn b50b_haraka_permutations() {
        let p = pair();
        let mut rng = Rng::new(SEED ^ 501);
        unsafe {
            let cp = sym!(p.c, b"SPX_haraka512_perm\0", FPerm);
            let rp = sym!(p.r, b"SPX_haraka512_perm\0", FPerm);
            let c5 = sym!(p.c, b"SPX_haraka512\0", FPerm);
            let r5 = sym!(p.r, b"SPX_haraka512\0", FPerm);
            let c2 = sym!(p.c, b"SPX_haraka256\0", FPerm);
            let r2 = sym!(p.r, b"SPX_haraka256\0", FPerm);
            for _ in 0..48 {
                let ps = rng.bytes(N);
                let ss = rng.bytes(N);
                let cc = make_ctx(&p.c, &ps, &ss);
                let rc = make_ctx(&p.r, &ps, &ss);
                let in64 = rng.bytes(64);
                let in32 = rng.bytes(32);

                let mut co = obuf(64);
                let mut ro = obuf(64);
                cp(co.as_mut_ptr(), in64.as_ptr(), cc.as_ptr());
                rp(ro.as_mut_ptr(), in64.as_ptr(), rc.as_ptr());
                eqb("haraka512_perm", &co, &ro);

                let mut co = obuf(32);
                let mut ro = obuf(32);
                c5(co.as_mut_ptr(), in64.as_ptr(), cc.as_ptr());
                r5(ro.as_mut_ptr(), in64.as_ptr(), rc.as_ptr());
                eqb("haraka512", &co, &ro);

                let mut co = obuf(32);
                let mut ro = obuf(32);
                c2(co.as_mut_ptr(), in32.as_ptr(), cc.as_ptr());
                r2(ro.as_mut_ptr(), in32.as_ptr(), rc.as_ptr());
                eqb("haraka256", &co, &ro);
            }
        }
    }

    #[test]
    fn b50c_haraka_sponge() {
        let p = pair();
        let mut rng = Rng::new(SEED ^ 502);
        unsafe {
            let cs = sym!(p.c, b"SPX_haraka_S\0", FS);
            let rs = sym!(p.r, b"SPX_haraka_S\0", FS);
            let ps = rng.bytes(N);
            let ss = rng.bytes(N);
            let cc = make_ctx(&p.c, &ps, &ss);
            let rc = make_ctx(&p.r, &ps, &ss);
            for inlen in len_sweep() {
                for outlen in [0usize, 1, 31, 32, 33, 64, 100] {
                    let m = rng.bytes(inlen.max(1));
                    let mut co = obuf(outlen);
                    let mut ro = obuf(outlen);
                    cs(co.as_mut_ptr(), outlen as u64, m.as_ptr(), inlen as u64, cc.as_ptr());
                    rs(ro.as_mut_ptr(), outlen as u64, m.as_ptr(), inlen as u64, rc.as_ptr());
                    eqb(&format!("haraka_S in={inlen} out={outlen}"), &co, &ro);
                }
            }
        }
    }

    #[test]
    fn b50d_haraka_incremental() {
        let p = pair();
        let mut rng = Rng::new(SEED ^ 503);
        unsafe {
            let ci = sym!(p.c, b"SPX_haraka_S_inc_init\0", FIncInit);
            let ri = sym!(p.r, b"SPX_haraka_S_inc_init\0", FIncInit);
            let ca = sym!(p.c, b"SPX_haraka_S_inc_absorb\0", FIncAbsorb);
            let ra = sym!(p.r, b"SPX_haraka_S_inc_absorb\0", FIncAbsorb);
            let cfz = sym!(p.c, b"SPX_haraka_S_inc_finalize\0", FIncFinalize);
            let rfz = sym!(p.r, b"SPX_haraka_S_inc_finalize\0", FIncFinalize);
            let cq = sym!(p.c, b"SPX_haraka_S_inc_squeeze\0", FIncSqueeze);
            let rq = sym!(p.r, b"SPX_haraka_S_inc_squeeze\0", FIncSqueeze);

            let ps = rng.bytes(N);
            let ss = rng.bytes(N);
            let cc = make_ctx(&p.c, &ps, &ss);
            let rc = make_ctx(&p.r, &ps, &ss);

            for parts in [
                vec![0usize],
                vec![1],
                vec![N, N, 0],
                vec![N, N, 1],
                vec![N, N, 137],
                vec![31, 1],
                vec![32, 1],
                vec![1, 31],
                vec![100, 100],
                vec![64, 3],
            ] {
                // s_inc is 65 bytes
                let mut cst = vec![0u8; 65 + SLACK];
                let mut rst = vec![0u8; 65 + SLACK];
                ci(cst.as_mut_ptr());
                ri(rst.as_mut_ptr());
                eqb("haraka inc state after init", &cst, &rst);
                for len in &parts {
                    let m = rng.bytes((*len).max(1));
                    ca(cst.as_mut_ptr(), m.as_ptr(), *len, cc.as_ptr());
                    ra(rst.as_mut_ptr(), m.as_ptr(), *len, rc.as_ptr());
                    eqb(&format!("haraka inc state after absorb {len}"), &cst, &rst);
                }
                cfz(cst.as_mut_ptr());
                rfz(rst.as_mut_ptr());
                eqb("haraka inc state after finalize", &cst, &rst);
                for outlen in [1usize, 16, 31, 32, 33, 64, 100] {
                    let mut co = obuf(outlen);
                    let mut ro = obuf(outlen);
                    cq(co.as_mut_ptr(), outlen, cst.as_mut_ptr(), cc.as_ptr());
                    rq(ro.as_mut_ptr(), outlen, rst.as_mut_ptr(), rc.as_ptr());
                    eqb(&format!("haraka inc squeeze out={outlen} parts={parts:?}"), &co, &ro);
                    eqb(&format!("haraka inc state after squeeze {outlen}"), &cst, &rst);
                }
            }
        }
    }
}
