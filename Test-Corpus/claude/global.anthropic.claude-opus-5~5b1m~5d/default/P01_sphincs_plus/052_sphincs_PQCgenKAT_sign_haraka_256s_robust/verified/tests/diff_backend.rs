//! Phase B — differential tests for the hash-backend primitives
//! (`lib/<backend>/src/*.c`).  Only the module for the active backend is
//! compiled; every one drives the *lowest-level* exported entry points, not
//! just the one-shot convenience wrappers.

mod common;
use common::*;

/// Message/​input lengths that straddle every block/rate boundary used by any
/// of the four backends (SHA-256 block 64, SHA-512 / BLAKE-512 block 128,
/// SHAKE-256 rate 136, HARAKA-S rate 32, BLAKE padding boundaries 55/111).
pub fn interesting_lens() -> Vec<usize> {
    let mut v = vec![
        0, 1, 2, 3, 7, 8, 15, 16, 17, 23, 24, 31, 32, 33, 47, 48, 54, 55, 56, 63, 64, 65, 71, 72,
        79, 80, 95, 96, 110, 111, 112, 119, 120, 127, 128, 129, 135, 136, 137, 143, 144, 159, 160,
        191, 192, 199, 200, 255, 256, 257, 271, 272, 383, 384, 511, 512, 1000, 1023, 1024, 1025,
    ];
    v.dedup();
    v
}

// ==================================================================
// BLAKE
// ==================================================================
#[cfg(spx_backend = "blake")]
mod blake {
    use super::*;

    type Blake = unsafe extern "C" fn(*mut u8, *const u8, u64) -> i32;
    type Init = unsafe extern "C" fn(*mut u8);
    type Update = unsafe extern "C" fn(*mut u8, *const u8, u64);
    type Compress = unsafe extern "C" fn(*mut u8, *const u8);
    type Final = unsafe extern "C" fn(*mut u8, *mut u8);
    type Mgf1 = unsafe extern "C" fn(*mut u8, u64, *const u8, u64);

    // sizeof(blakestate256) = 8*4 + 4*4 + 2*4 + 4 + 4 + 64 = 128
    const ST256: usize = 128;
    // sizeof(blakestate512) = 8*8 + 4*8 + 2*8 + 4 + 4 + 128 = 248
    const ST512: usize = 248;

    #[test]
    fn cst_data_symbol_matches() {
        let libs = Libs::load();
        let c = libs.c_data("cst");
        let r = libs.r_data("cst");
        unsafe {
            let cs = std::slice::from_raw_parts(c as *const u64, 16);
            let rs = std::slice::from_raw_parts(r as *const u64, 16);
            assert_eq!(cs, rs, "exported `cst` table differs");
        }
    }

    #[test]
    fn blake256_oneshot() {
        let libs = Libs::load();
        let (c, r) = libs.pair::<Blake>("blake256");
        let mut rng = Rng::new(0xB256);
        for len in interesting_lens() {
            for _ in 0..3 {
                let m = rng.bytes(len);
                let mut co = [0u8; 32];
                let mut ro = [0u8; 32];
                unsafe {
                    c(co.as_mut_ptr(), m.as_ptr(), len as u64);
                    r(ro.as_mut_ptr(), m.as_ptr(), len as u64);
                }
                assert_bytes_eq(&format!("blake256(len={})", len), &co, &ro);
            }
        }
    }

    #[test]
    fn blake512_oneshot() {
        let libs = Libs::load();
        let (c, r) = libs.pair::<Blake>("blake512");
        let mut rng = Rng::new(0xB512);
        for len in interesting_lens() {
            for _ in 0..3 {
                let m = rng.bytes(len);
                let mut co = [0u8; 64];
                let mut ro = [0u8; 64];
                unsafe {
                    c(co.as_mut_ptr(), m.as_ptr(), len as u64);
                    r(ro.as_mut_ptr(), m.as_ptr(), len as u64);
                }
                assert_bytes_eq(&format!("blake512(len={})", len), &co, &ro);
            }
        }
    }

    /// init / update(with *bit* lengths, as the C callers use) / final, plus
    /// the raw compress function, driven directly.
    #[test]
    fn blake256_incremental_and_compress() {
        let libs = Libs::load();
        let (ci, ri) = libs.pair::<Init>("blake256_init");
        let (cu, ru) = libs.pair::<Update>("blake256_update");
        let (cf, rf) = libs.pair::<Final>("blake256_final");
        let (cc, rc) = libs.pair::<Compress>("blake256_compress");
        let mut rng = Rng::new(0xB2561);

        // raw compress on a random state + random block
        for _ in 0..200 {
            let mut cs = rng.bytes(ST256);
            let mut rs = cs.clone();
            let blk = rng.bytes(64);
            unsafe {
                cc(cs.as_mut_ptr(), blk.as_ptr());
                rc(rs.as_mut_ptr(), blk.as_ptr());
            }
            assert_bytes_eq("blake256_compress", &cs, &rs);
        }

        // multi-chunk incremental hashing; chunk lengths are given in BITS,
        // matching how hash_blake.c calls the routine.
        for _ in 0..200 {
            let nchunks = 1 + rng.below(5) as usize;
            let chunks: Vec<Vec<u8>> = (0..nchunks)
                .map(|_| rng.bytes_upto(200))
                .collect();
            let mut cs = vec![0u8; ST256];
            let mut rs = vec![0u8; ST256];
            let mut co = [0u8; 32];
            let mut ro = [0u8; 32];
            unsafe {
                ci(cs.as_mut_ptr());
                ri(rs.as_mut_ptr());
                for ch in &chunks {
                    cu(cs.as_mut_ptr(), ch.as_ptr(), (ch.len() * 8) as u64);
                    ru(rs.as_mut_ptr(), ch.as_ptr(), (ch.len() * 8) as u64);
                }
                assert_bytes_eq("blake256 state after updates", &cs, &rs);
                cf(cs.as_mut_ptr(), co.as_mut_ptr());
                rf(rs.as_mut_ptr(), ro.as_mut_ptr());
            }
            assert_bytes_eq("blake256_final", &co, &ro);
            assert_bytes_eq("blake256 state after final", &cs, &rs);
        }

        // The quirky call pattern used by hash_blake.c: byte counts passed
        // where bit counts are expected.
        for len in [0usize, 1, 8, 16, 24, 32, 64, 128, 136] {
            let m = rng.bytes(len.max(1) * 8);
            let mut cs = vec![0u8; ST256];
            let mut rs = vec![0u8; ST256];
            let mut co = [0u8; 32];
            let mut ro = [0u8; 32];
            unsafe {
                ci(cs.as_mut_ptr());
                ri(rs.as_mut_ptr());
                cu(cs.as_mut_ptr(), m.as_ptr(), len as u64);
                ru(rs.as_mut_ptr(), m.as_ptr(), len as u64);
                cf(cs.as_mut_ptr(), co.as_mut_ptr());
                rf(rs.as_mut_ptr(), ro.as_mut_ptr());
            }
            assert_bytes_eq(&format!("blake256 bitlen={}", len), &co, &ro);
        }
    }

    #[test]
    fn blake512_incremental_and_compress() {
        let libs = Libs::load();
        let (ci, ri) = libs.pair::<Init>("blake512_init");
        let (cu, ru) = libs.pair::<Update>("blake512_update");
        let (cf, rf) = libs.pair::<Final>("blake512_final");
        let (cc, rc) = libs.pair::<Compress>("blake512_compress");
        let mut rng = Rng::new(0xB5121);

        for _ in 0..200 {
            let mut cs = rng.bytes(ST512);
            let mut rs = cs.clone();
            let blk = rng.bytes(128);
            unsafe {
                cc(cs.as_mut_ptr(), blk.as_ptr());
                rc(rs.as_mut_ptr(), blk.as_ptr());
            }
            assert_bytes_eq("blake512_compress", &cs, &rs);
        }

        for _ in 0..200 {
            let nchunks = 1 + rng.below(5) as usize;
            let chunks: Vec<Vec<u8>> = (0..nchunks)
                .map(|_| rng.bytes_upto(400))
                .collect();
            let mut cs = vec![0u8; ST512];
            let mut rs = vec![0u8; ST512];
            let mut co = [0u8; 64];
            let mut ro = [0u8; 64];
            unsafe {
                ci(cs.as_mut_ptr());
                ri(rs.as_mut_ptr());
                for ch in &chunks {
                    cu(cs.as_mut_ptr(), ch.as_ptr(), (ch.len() * 8) as u64);
                    ru(rs.as_mut_ptr(), ch.as_ptr(), (ch.len() * 8) as u64);
                }
                assert_bytes_eq("blake512 state after updates", &cs, &rs);
                cf(cs.as_mut_ptr(), co.as_mut_ptr());
                rf(rs.as_mut_ptr(), ro.as_mut_ptr());
            }
            assert_bytes_eq("blake512_final", &co, &ro);
            assert_bytes_eq("blake512 state after final", &cs, &rs);
        }

        for len in [0usize, 1, 8, 16, 24, 32, 64, 128, 136] {
            let m = rng.bytes(len.max(1) * 8);
            let mut cs = vec![0u8; ST512];
            let mut rs = vec![0u8; ST512];
            let mut co = [0u8; 64];
            let mut ro = [0u8; 64];
            unsafe {
                ci(cs.as_mut_ptr());
                ri(rs.as_mut_ptr());
                cu(cs.as_mut_ptr(), m.as_ptr(), len as u64);
                ru(rs.as_mut_ptr(), m.as_ptr(), len as u64);
                cf(cs.as_mut_ptr(), co.as_mut_ptr());
                rf(rs.as_mut_ptr(), ro.as_mut_ptr());
            }
            assert_bytes_eq(&format!("blake512 bitlen={}", len), &co, &ro);
        }
    }

    #[test]
    fn blake_mgf1_all_lengths() {
        let libs = Libs::load();
        let mut rng = Rng::new(0x0F1);
        for name in ["SPX_blake256_mgf1", "SPX_blake512_mgf1"] {
            let (c, r) = libs.pair::<Mgf1>(name);
            let block = if name.contains("256") { 32 } else { 64 };
            let mut outlens: Vec<usize> = vec![
                1,
                block - 1,
                block,
                block + 1,
                2 * block - 1,
                2 * block,
                2 * block + 1,
                5 * block + 3,
            ];
            outlens.push(DGST_BYTES);
            for inlen in [1usize, 4, 16, 32, 48, 64, 96, 2 * N + 32, 2 * N + 64] {
                for &outlen in &outlens {
                    let inp = rng.bytes(inlen);
                    let mut co = vec![0xEEu8; outlen];
                    let mut ro = vec![0xEEu8; outlen];
                    unsafe {
                        c(co.as_mut_ptr(), outlen as u64, inp.as_ptr(), inlen as u64);
                        r(ro.as_mut_ptr(), outlen as u64, inp.as_ptr(), inlen as u64);
                    }
                    assert_bytes_eq(
                        &format!("{}(outlen={}, inlen={})", name, outlen, inlen),
                        &co,
                        &ro,
                    );
                }
            }
        }
    }
}

// ==================================================================
// SHA-2
// ==================================================================
#[cfg(spx_backend = "sha2")]
mod sha2 {
    use super::*;
    use std::ffi::c_void;

    type Sha = unsafe extern "C" fn(*mut u8, *const u8, usize);
    type IncInit = unsafe extern "C" fn(*mut u8);
    type IncBlocks = unsafe extern "C" fn(*mut u8, *const u8, usize);
    type IncFinalize = unsafe extern "C" fn(*mut u8, *mut u8, *const u8, usize);
    type Mgf1 = unsafe extern "C" fn(*mut u8, u64, *const u8, u64);
    type SeedState = unsafe extern "C" fn(*mut c_void);

    #[test]
    fn sha256_oneshot() {
        let libs = Libs::load();
        let (c, r) = libs.pair::<Sha>("sha256");
        let mut rng = Rng::new(0x256);
        for len in interesting_lens() {
            for _ in 0..3 {
                let m = rng.bytes(len);
                let mut co = [0u8; 32];
                let mut ro = [0u8; 32];
                unsafe {
                    c(co.as_mut_ptr(), m.as_ptr(), len);
                    r(ro.as_mut_ptr(), m.as_ptr(), len);
                }
                assert_bytes_eq(&format!("sha256(len={})", len), &co, &ro);
            }
        }
    }

    #[test]
    fn sha512_oneshot() {
        let libs = Libs::load();
        let (c, r) = libs.pair::<Sha>("sha512");
        let mut rng = Rng::new(0x512);
        for len in interesting_lens() {
            for _ in 0..3 {
                let m = rng.bytes(len);
                let mut co = [0u8; 64];
                let mut ro = [0u8; 64];
                unsafe {
                    c(co.as_mut_ptr(), m.as_ptr(), len);
                    r(ro.as_mut_ptr(), m.as_ptr(), len);
                }
                assert_bytes_eq(&format!("sha512(len={})", len), &co, &ro);
            }
        }
    }

    #[test]
    fn sha_incremental() {
        let libs = Libs::load();
        let mut rng = Rng::new(0x1C00);
        for (name, statelen, outlen, block) in
            [("sha256", 40usize, 32usize, 64usize), ("sha512", 72, 64, 128)]
        {
            let (ci, ri) = libs.pair::<IncInit>(&format!("{}_inc_init", name));
            let (cb, rb) = libs.pair::<IncBlocks>(&format!("{}_inc_blocks", name));
            let (cf, rf) = libs.pair::<IncFinalize>(&format!("{}_inc_finalize", name));

            for nblocks in [0usize, 1, 2, 3, 5] {
                for taillen in [0usize, 1, block - 9, block - 8, block - 1, block, block + 1, 2 * block + 5] {
                    let data = rng.bytes(nblocks * block);
                    let tail = rng.bytes(taillen);
                    let mut cs = vec![0u8; statelen];
                    let mut rs = vec![0u8; statelen];
                    let mut co = vec![0u8; outlen];
                    let mut ro = vec![0u8; outlen];
                    unsafe {
                        ci(cs.as_mut_ptr());
                        ri(rs.as_mut_ptr());
                        assert_bytes_eq(&format!("{}_inc_init state", name), &cs, &rs);
                        if nblocks > 0 {
                            cb(cs.as_mut_ptr(), data.as_ptr(), nblocks);
                            rb(rs.as_mut_ptr(), data.as_ptr(), nblocks);
                        }
                        assert_bytes_eq(&format!("{}_inc_blocks state", name), &cs, &rs);
                        cf(co.as_mut_ptr(), cs.as_mut_ptr(), tail.as_ptr(), taillen);
                        rf(ro.as_mut_ptr(), rs.as_mut_ptr(), tail.as_ptr(), taillen);
                    }
                    assert_bytes_eq(
                        &format!("{}_inc_finalize(nblocks={}, tail={})", name, nblocks, taillen),
                        &co,
                        &ro,
                    );
                    assert_bytes_eq(
                        &format!("{}_inc_finalize state(nblocks={}, tail={})", name, nblocks, taillen),
                        &cs,
                        &rs,
                    );
                }
            }
        }
    }

    #[test]
    fn mgf1_all_lengths() {
        let libs = Libs::load();
        let mut rng = Rng::new(0x9F1);
        for (name, block) in [("SPX_mgf1_256", 32usize), ("SPX_mgf1_512", 64)] {
            let (c, r) = libs.pair::<Mgf1>(name);
            let mut outlens = vec![
                1,
                block - 1,
                block,
                block + 1,
                2 * block,
                2 * block + 1,
                5 * block + 3,
            ];
            outlens.push(DGST_BYTES);
            for inlen in [1usize, 4, 16, 32, 48, 64, 96, 2 * N + 32, 2 * N + 64] {
                for &outlen in &outlens {
                    let inp = rng.bytes(inlen);
                    let mut co = vec![0xEEu8; outlen];
                    let mut ro = vec![0xEEu8; outlen];
                    unsafe {
                        c(co.as_mut_ptr(), outlen as u64, inp.as_ptr(), inlen as u64);
                        r(ro.as_mut_ptr(), outlen as u64, inp.as_ptr(), inlen as u64);
                    }
                    assert_bytes_eq(
                        &format!("{}(outlen={}, inlen={})", name, outlen, inlen),
                        &co,
                        &ro,
                    );
                }
            }
        }
    }

    #[test]
    fn seed_state_matches() {
        let libs = Libs::load();
        let (c, r) = libs.pair::<SeedState>("SPX_seed_state");
        let mut rng = Rng::new(0x5EED);
        for _ in 0..100 {
            let ps = rng.bytes(N);
            let ss = rng.bytes(N);
            let mut cc = Ctx::new(&ps, &ss);
            let mut rc = Ctx::new(&ps, &ss);
            unsafe {
                c(cc.as_mut_ptr());
                r(rc.as_mut_ptr());
            }
            assert_bytes_eq("SPX_seed_state ctx", cc.live(), rc.live());
        }
    }
}

// ==================================================================
// SHAKE / FIPS-202
// ==================================================================
#[cfg(spx_backend = "shake")]
mod shake {
    use super::*;

    type Shake = unsafe extern "C" fn(*mut u8, usize, *const u8, usize);
    type Absorb = unsafe extern "C" fn(*mut u64, *const u8, usize);
    type SqueezeBlocks = unsafe extern "C" fn(*mut u8, usize, *mut u64);
    type IncInit = unsafe extern "C" fn(*mut u64);
    type IncAbsorb = unsafe extern "C" fn(*mut u64, *const u8, usize);
    type IncFinalize = unsafe extern "C" fn(*mut u64);
    type IncSqueeze = unsafe extern "C" fn(*mut u8, usize, *mut u64);

    const RATE: usize = 136;

    #[test]
    fn shake256_oneshot() {
        let libs = Libs::load();
        let (c, r) = libs.pair::<Shake>("shake256");
        let mut rng = Rng::new(0x5A0);
        for inlen in interesting_lens() {
            for outlen in [1usize, RATE - 1, RATE, RATE + 1, 2 * RATE, N, DGST_BYTES] {
                let m = rng.bytes(inlen);
                let mut co = vec![0u8; outlen];
                let mut ro = vec![0u8; outlen];
                unsafe {
                    c(co.as_mut_ptr(), outlen, m.as_ptr(), inlen);
                    r(ro.as_mut_ptr(), outlen, m.as_ptr(), inlen);
                }
                assert_bytes_eq(
                    &format!("shake256(inlen={}, outlen={})", inlen, outlen),
                    &co,
                    &ro,
                );
            }
        }
    }

    /// `shake256_absorb` requires inlen to be a multiple of the rate.
    #[test]
    fn shake256_absorb_squeezeblocks() {
        let libs = Libs::load();
        let (ca, ra) = libs.pair::<Absorb>("shake256_absorb");
        let (cs, rs) = libs.pair::<SqueezeBlocks>("shake256_squeezeblocks");
        let mut rng = Rng::new(0xA850);
        for nin in [1usize, 2, 3, 5] {
            for nout in [1usize, 2, 3, 8] {
                let m = rng.bytes(nin * RATE);
                let mut cst = vec![0u64; 25];
                let mut rst = vec![0u64; 25];
                let mut co = vec![0u8; nout * RATE];
                let mut ro = vec![0u8; nout * RATE];
                unsafe {
                    ca(cst.as_mut_ptr(), m.as_ptr(), nin * RATE);
                    ra(rst.as_mut_ptr(), m.as_ptr(), nin * RATE);
                    assert_eq!(cst, rst, "shake256_absorb state (nin={})", nin);
                    cs(co.as_mut_ptr(), nout, cst.as_mut_ptr());
                    rs(ro.as_mut_ptr(), nout, rst.as_mut_ptr());
                }
                assert_bytes_eq(
                    &format!("shake256_squeezeblocks(nin={}, nout={})", nin, nout),
                    &co,
                    &ro,
                );
                assert_eq!(cst, rst, "state after squeezeblocks");
            }
        }
    }

    #[test]
    fn shake256_incremental() {
        let libs = Libs::load();
        let (ci, ri) = libs.pair::<IncInit>("shake256_inc_init");
        let (ca, ra) = libs.pair::<IncAbsorb>("shake256_inc_absorb");
        let (cf, rf) = libs.pair::<IncFinalize>("shake256_inc_finalize");
        let (cq, rq) = libs.pair::<IncSqueeze>("shake256_inc_squeeze");
        let mut rng = Rng::new(0x1C50);

        for _ in 0..200 {
            let nchunks = 1 + rng.below(5) as usize;
            let chunks: Vec<Vec<u8>> = (0..nchunks)
                .map(|_| rng.bytes_upto(300))
                .collect();
            let nsq = 1 + rng.below(4) as usize;
            let sqlens: Vec<usize> = rng.lens(nsq, 200);

            let mut cst = vec![0u64; 26];
            let mut rst = vec![0u64; 26];
            unsafe {
                ci(cst.as_mut_ptr());
                ri(rst.as_mut_ptr());
                assert_eq!(cst, rst, "inc_init state");
                for ch in &chunks {
                    ca(cst.as_mut_ptr(), ch.as_ptr(), ch.len());
                    ra(rst.as_mut_ptr(), ch.as_ptr(), ch.len());
                }
                assert_eq!(cst, rst, "inc_absorb state");
                cf(cst.as_mut_ptr());
                rf(rst.as_mut_ptr());
                assert_eq!(cst, rst, "inc_finalize state");
                for &l in &sqlens {
                    let mut co = vec![0u8; l];
                    let mut ro = vec![0u8; l];
                    cq(co.as_mut_ptr(), l, cst.as_mut_ptr());
                    rq(ro.as_mut_ptr(), l, rst.as_mut_ptr());
                    assert_bytes_eq(&format!("inc_squeeze({})", l), &co, &ro);
                    assert_eq!(cst, rst, "state after inc_squeeze");
                }
            }
        }

        // boundary absorb/squeeze lengths around the rate
        for l in [0usize, 1, RATE - 1, RATE, RATE + 1, 2 * RATE, 2 * RATE + 1] {
            let m = rng.bytes(l);
            let mut cst = vec![0u64; 26];
            let mut rst = vec![0u64; 26];
            let mut co = vec![0u8; 2 * RATE + 7];
            let mut ro = vec![0u8; 2 * RATE + 7];
            unsafe {
                ci(cst.as_mut_ptr());
                ri(rst.as_mut_ptr());
                ca(cst.as_mut_ptr(), m.as_ptr(), l);
                ra(rst.as_mut_ptr(), m.as_ptr(), l);
                cf(cst.as_mut_ptr());
                rf(rst.as_mut_ptr());
                cq(co.as_mut_ptr(), co.len(), cst.as_mut_ptr());
                rq(ro.as_mut_ptr(), ro.len(), rst.as_mut_ptr());
            }
            assert_bytes_eq(&format!("inc absorb/squeeze boundary l={}", l), &co, &ro);
        }
    }
}

// ==================================================================
// HARAKA
// ==================================================================
#[cfg(spx_backend = "haraka")]
mod haraka {
    use super::*;
    use std::ffi::c_void;

    type Tweak = unsafe extern "C" fn(*mut c_void);
    type Perm = unsafe extern "C" fn(*mut u8, *const u8, *const c_void);
    type HarakaS = unsafe extern "C" fn(*mut u8, u64, *const u8, u64, *const c_void);
    type IncInit = unsafe extern "C" fn(*mut u8);
    type IncAbsorb = unsafe extern "C" fn(*mut u8, *const u8, usize, *const c_void);
    type IncFinalize = unsafe extern "C" fn(*mut u8);
    type IncSqueeze = unsafe extern "C" fn(*mut u8, usize, *mut u8, *const c_void);

    const RATE: usize = 32;

    #[test]
    fn tweak_constants_matches() {
        let libs = Libs::load();
        let (c, r) = libs.pair::<Tweak>("SPX_tweak_constants");
        let mut rng = Rng::new(0x7EA4);
        for _ in 0..100 {
            let ps = rng.bytes(N);
            let ss = rng.bytes(N);
            let mut cc = Ctx::new(&ps, &ss);
            let mut rc = Ctx::new(&ps, &ss);
            unsafe {
                c(cc.as_mut_ptr());
                r(rc.as_mut_ptr());
            }
            assert_bytes_eq("SPX_tweak_constants ctx", cc.live(), rc.live());
        }
    }

    #[test]
    fn haraka512_perm_and_512_and_256() {
        let libs = Libs::load();
        let mut rng = Rng::new(0x4A4A);
        let ps = rng.bytes(N);
        let ss = rng.bytes(N);
        let (cc, rc) = init_ctx_pair(&libs, &ps, &ss);

        let (cp, rp) = libs.pair::<Perm>("SPX_haraka512_perm");
        let (c5, r5) = libs.pair::<Perm>("SPX_haraka512");
        let (c2, r2) = libs.pair::<Perm>("SPX_haraka256");

        for _ in 0..500 {
            let inp64 = rng.bytes(64);
            let inp32 = rng.bytes(32);
            let mut co = [0u8; 64];
            let mut ro = [0u8; 64];
            unsafe {
                cp(co.as_mut_ptr(), inp64.as_ptr(), cc.as_ptr());
                rp(ro.as_mut_ptr(), inp64.as_ptr(), rc.as_ptr());
            }
            assert_bytes_eq("SPX_haraka512_perm", &co, &ro);

            let mut co = [0u8; 32];
            let mut ro = [0u8; 32];
            unsafe {
                c5(co.as_mut_ptr(), inp64.as_ptr(), cc.as_ptr());
                r5(ro.as_mut_ptr(), inp64.as_ptr(), rc.as_ptr());
            }
            assert_bytes_eq("SPX_haraka512", &co, &ro);

            let mut co = [0u8; 32];
            let mut ro = [0u8; 32];
            unsafe {
                c2(co.as_mut_ptr(), inp32.as_ptr(), cc.as_ptr());
                r2(ro.as_mut_ptr(), inp32.as_ptr(), rc.as_ptr());
            }
            assert_bytes_eq("SPX_haraka256", &co, &ro);
        }
    }

    #[test]
    fn haraka_sponge_oneshot() {
        let libs = Libs::load();
        let mut rng = Rng::new(0x5F00);
        let ps = rng.bytes(N);
        let ss = rng.bytes(N);
        let (cc, rc) = init_ctx_pair(&libs, &ps, &ss);
        let (c, r) = libs.pair::<HarakaS>("SPX_haraka_S");

        for inlen in interesting_lens() {
            for outlen in [1usize, RATE - 1, RATE, RATE + 1, 2 * RATE, N, DGST_BYTES] {
                let m = rng.bytes(inlen);
                let mut co = vec![0u8; outlen];
                let mut ro = vec![0u8; outlen];
                unsafe {
                    c(
                        co.as_mut_ptr(),
                        outlen as u64,
                        m.as_ptr(),
                        inlen as u64,
                        cc.as_ptr(),
                    );
                    r(
                        ro.as_mut_ptr(),
                        outlen as u64,
                        m.as_ptr(),
                        inlen as u64,
                        rc.as_ptr(),
                    );
                }
                assert_bytes_eq(
                    &format!("SPX_haraka_S(inlen={}, outlen={})", inlen, outlen),
                    &co,
                    &ro,
                );
            }
        }
    }

    #[test]
    fn haraka_sponge_incremental() {
        let libs = Libs::load();
        let mut rng = Rng::new(0x1C40);
        let ps = rng.bytes(N);
        let ss = rng.bytes(N);
        let (cc, rc) = init_ctx_pair(&libs, &ps, &ss);

        let (ci, ri) = libs.pair::<IncInit>("SPX_haraka_S_inc_init");
        let (ca, ra) = libs.pair::<IncAbsorb>("SPX_haraka_S_inc_absorb");
        let (cf, rf) = libs.pair::<IncFinalize>("SPX_haraka_S_inc_finalize");
        let (cq, rq) = libs.pair::<IncSqueeze>("SPX_haraka_S_inc_squeeze");

        for _ in 0..200 {
            let nchunks = 1 + rng.below(5) as usize;
            let chunks: Vec<Vec<u8>> = (0..nchunks)
                .map(|_| rng.bytes_upto(100))
                .collect();
            let nsq = 1 + rng.below(4) as usize;
            let sqlens: Vec<usize> = rng.lens(nsq, 100);

            let mut cst = vec![0u8; 65];
            let mut rst = vec![0u8; 65];
            unsafe {
                ci(cst.as_mut_ptr());
                ri(rst.as_mut_ptr());
                assert_bytes_eq("haraka inc_init state", &cst, &rst);
                for ch in &chunks {
                    ca(cst.as_mut_ptr(), ch.as_ptr(), ch.len(), cc.as_ptr());
                    ra(rst.as_mut_ptr(), ch.as_ptr(), ch.len(), rc.as_ptr());
                }
                assert_bytes_eq("haraka inc_absorb state", &cst, &rst);
                cf(cst.as_mut_ptr());
                rf(rst.as_mut_ptr());
                assert_bytes_eq("haraka inc_finalize state", &cst, &rst);
                for &l in &sqlens {
                    let mut co = vec![0u8; l];
                    let mut ro = vec![0u8; l];
                    cq(co.as_mut_ptr(), l, cst.as_mut_ptr(), cc.as_ptr());
                    rq(ro.as_mut_ptr(), l, rst.as_mut_ptr(), rc.as_ptr());
                    assert_bytes_eq(&format!("haraka inc_squeeze({})", l), &co, &ro);
                    assert_bytes_eq("haraka state after inc_squeeze", &cst, &rst);
                }
            }
        }

        for l in [0usize, 1, RATE - 1, RATE, RATE + 1, 2 * RATE, 2 * RATE + 1] {
            let m = rng.bytes(l);
            let mut cst = vec![0u8; 65];
            let mut rst = vec![0u8; 65];
            let mut co = vec![0u8; 2 * RATE + 7];
            let mut ro = vec![0u8; 2 * RATE + 7];
            unsafe {
                ci(cst.as_mut_ptr());
                ri(rst.as_mut_ptr());
                ca(cst.as_mut_ptr(), m.as_ptr(), l, cc.as_ptr());
                ra(rst.as_mut_ptr(), m.as_ptr(), l, rc.as_ptr());
                cf(cst.as_mut_ptr());
                rf(rst.as_mut_ptr());
                cq(co.as_mut_ptr(), co.len(), cst.as_mut_ptr(), cc.as_ptr());
                rq(ro.as_mut_ptr(), ro.len(), rst.as_mut_ptr(), rc.as_ptr());
            }
            assert_bytes_eq(&format!("haraka inc boundary l={}", l), &co, &ro);
        }
    }
}
