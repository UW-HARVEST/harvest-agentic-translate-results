//! Phase B, level 0 — hash-backend primitives (`CONFIGS.md` rows C10–C25).
//!
//! Each block is `cfg`-gated to the backend that actually exports the symbols.

mod common;
use common::*;

/* ================================================================== */
/* blake                                                              */
/* ================================================================== */

#[cfg(all(feature = "blake", not(feature = "sha2"), not(feature = "shake")))]
mod blake {
    use super::*;

    type Blake = unsafe extern "C" fn(*mut u8, *const u8, u64) -> i32;
    type Init256 = unsafe extern "C" fn(*mut BlakeState256);
    type Update256 = unsafe extern "C" fn(*mut BlakeState256, *const u8, u64);
    type Final256 = unsafe extern "C" fn(*mut BlakeState256, *mut u8);
    type Compress256 = unsafe extern "C" fn(*mut BlakeState256, *const u8);
    type Init512 = unsafe extern "C" fn(*mut BlakeState512);
    type Update512 = unsafe extern "C" fn(*mut BlakeState512, *const u8, u64);
    type Final512 = unsafe extern "C" fn(*mut BlakeState512, *mut u8);
    type Compress512 = unsafe extern "C" fn(*mut BlakeState512, *const u8);
    type Mgf1 = unsafe extern "C" fn(*mut u8, u64, *const u8, u64);

    /// Input lengths that hit both padding branches of both block sizes.
    fn inlens() -> Vec<usize> {
        vec![0, 1, 2, 54, 55, 56, 57, 63, 64, 65, 110, 111, 112, 113, 127, 128, 129, 200, 255]
    }

    /* ---- C10 ------------------------------------------------------ */
    #[test]
    fn c10_blake_oneshot() {
        let l = libs();
        let mut rng = Rng::new(SEED + 10);
        for (name, outlen) in [("blake256", 32usize), ("blake512", 64)] {
            let (c, r) = l.pair::<Blake>(name);
            for len in inlens() {
                for _ in 0..4 {
                    let inp = rng.bytes(len);
                    let mut co = vec![0u8; outlen];
                    let mut ro = vec![0u8; outlen];
                    let (cv, rv) = unsafe {
                        (
                            c(co.as_mut_ptr(), inp.as_ptr(), len as u64),
                            r(ro.as_mut_ptr(), inp.as_ptr(), len as u64),
                        )
                    };
                    assert_eq!(cv, rv, "{name} return value (len={len})");
                    eq_bytes(&format!("{name}(len={len})"), &co, &ro);
                }
            }
        }
    }

    /* ---- C11 ------------------------------------------------------ */
    #[test]
    fn c11_blake_streaming() {
        let l = libs();
        let mut rng = Rng::new(SEED + 11);

        // NOTE: blake*_update takes a length in BITS (see blake256() calling
        // blake256_update(&S, in, inlen*8)).
        let (ci, ri) = l.pair::<Init256>("blake256_init");
        let (cu, ru) = l.pair::<Update256>("blake256_update");
        let (cf, rf) = l.pair::<Final256>("blake256_final");
        for total in [0usize, 1, 55, 56, 64, 65, 128, 200] {
            for nchunks in 1usize..=5 {
                let inp = rng.bytes(total);
                let mut cs = BlakeState256::default();
                let mut rs = BlakeState256::default();
                unsafe {
                    ci(&mut cs);
                    ri(&mut rs);
                }
                assert_eq!(cs, rs, "blake256_init state");
                // split into nchunks pieces (byte-at-a-time when nchunks > total)
                let mut cuts: Vec<usize> = (0..nchunks).map(|_| rng.below(total as u64 + 1) as usize).collect();
                cuts.push(0);
                cuts.push(total);
                cuts.sort_unstable();
                for w in cuts.windows(2) {
                    let (a, b) = (w[0], w[1]);
                    if b == a {
                        continue;
                    }
                    unsafe {
                        cu(&mut cs, inp.as_ptr().add(a), ((b - a) * 8) as u64);
                        ru(&mut rs, inp.as_ptr().add(a), ((b - a) * 8) as u64);
                    }
                    assert_eq!(cs, rs, "blake256_update state (total={total}, chunk {a}..{b})");
                }
                let mut co = [0u8; 32];
                let mut ro = [0u8; 32];
                unsafe {
                    cf(&mut cs, co.as_mut_ptr());
                    rf(&mut rs, ro.as_mut_ptr());
                }
                eq_bytes(&format!("blake256 streaming digest (total={total})"), &co, &ro);
                assert_eq!(cs, rs, "blake256_final state");
            }
        }

        let (ci, ri) = l.pair::<Init512>("blake512_init");
        let (cu, ru) = l.pair::<Update512>("blake512_update");
        let (cf, rf) = l.pair::<Final512>("blake512_final");
        for total in [0usize, 1, 111, 112, 128, 129, 256, 300] {
            for nchunks in 1usize..=5 {
                let inp = rng.bytes(total);
                let mut cs = BlakeState512::default();
                let mut rs = BlakeState512::default();
                unsafe {
                    ci(&mut cs);
                    ri(&mut rs);
                }
                assert_eq!(cs, rs, "blake512_init state");
                let mut cuts: Vec<usize> = (0..nchunks).map(|_| rng.below(total as u64 + 1) as usize).collect();
                cuts.push(0);
                cuts.push(total);
                cuts.sort_unstable();
                for w in cuts.windows(2) {
                    let (a, b) = (w[0], w[1]);
                    if b == a {
                        continue;
                    }
                    unsafe {
                        cu(&mut cs, inp.as_ptr().add(a), ((b - a) * 8) as u64);
                        ru(&mut rs, inp.as_ptr().add(a), ((b - a) * 8) as u64);
                    }
                    assert_eq!(cs, rs, "blake512_update state (total={total}, chunk {a}..{b})");
                }
                let mut co = [0u8; 64];
                let mut ro = [0u8; 64];
                unsafe {
                    cf(&mut cs, co.as_mut_ptr());
                    rf(&mut rs, ro.as_mut_ptr());
                }
                eq_bytes(&format!("blake512 streaming digest (total={total})"), &co, &ro);
                assert_eq!(cs, rs, "blake512_final state");
            }
        }
    }

    /* ---- C12 ------------------------------------------------------ */
    #[test]
    fn c12_blake_compress() {
        let l = libs();
        let mut rng = Rng::new(SEED + 12);
        let (c, r) = l.pair::<Compress256>("blake256_compress");
        for _ in 0..NUM_ITERS {
            let mut st = BlakeState256::default();
            for x in st.h.iter_mut() {
                *x = rng.next_u32();
            }
            for x in st.s.iter_mut() {
                *x = rng.next_u32();
            }
            for x in st.t.iter_mut() {
                *x = rng.next_u32();
            }
            st.nullt = (rng.next_u32() & 1) as i32;
            let block = rng.bytes(64);
            let mut cs = st;
            let mut rs = st;
            unsafe {
                c(&mut cs, block.as_ptr());
                r(&mut rs, block.as_ptr());
            }
            assert_eq!(cs, rs, "blake256_compress state");
        }
        let (c, r) = l.pair::<Compress512>("blake512_compress");
        for _ in 0..NUM_ITERS {
            let mut st = BlakeState512::default();
            for x in st.h.iter_mut() {
                *x = rng.next_u64();
            }
            for x in st.s.iter_mut() {
                *x = rng.next_u64();
            }
            for x in st.t.iter_mut() {
                *x = rng.next_u64();
            }
            st.nullt = (rng.next_u32() & 1) as i32;
            let block = rng.bytes(128);
            let mut cs = st;
            let mut rs = st;
            unsafe {
                c(&mut cs, block.as_ptr());
                r(&mut rs, block.as_ptr());
            }
            assert_eq!(cs, rs, "blake512_compress state");
        }
    }

    /* ---- C13 ------------------------------------------------------ */
    #[test]
    fn c13_blake_mgf1() {
        let l = libs();
        let mut rng = Rng::new(SEED + 13);
        let mut outlens = vec![1usize, 31, 32, 33, 63, 64, 65, 100, SPX_N, SPX_N * 2];
        outlens.push(SPX_WOTS_LEN * SPX_N);
        outlens.push(SPX_FORS_TREES as usize * SPX_N);
        outlens.sort_unstable();
        outlens.dedup();
        for name in ["SPX_blake256_mgf1", "SPX_blake512_mgf1"] {
            let (c, r) = l.pair::<Mgf1>(name);
            for &outlen in &outlens {
                for inlen in [1usize, SPX_N + SPX_ADDR_BYTES, 48, 64, 100] {
                    let inp = rng.bytes(inlen);
                    let mut co = vec![0u8; outlen];
                    let mut ro = vec![0u8; outlen];
                    unsafe {
                        c(co.as_mut_ptr(), outlen as u64, inp.as_ptr(), inlen as u64);
                        r(ro.as_mut_ptr(), outlen as u64, inp.as_ptr(), inlen as u64);
                    }
                    eq_bytes(&format!("{name}(outlen={outlen},inlen={inlen})"), &co, &ro);
                }
            }
        }
    }

    /* ---- C14 ------------------------------------------------------ */
    #[test]
    fn c14_blake_cst() {
        let l = libs();
        // `const u64 cst[16]` in blake512.c is a non-static global, so it is an
        // exported read-only data symbol; the Rust side must export it too.
        let c = l.c::<*const u8>("cst");
        let r = l.rs::<*const u8>("cst");
        let cb = unsafe { std::slice::from_raw_parts(*c, 128) };
        let rb = unsafe { std::slice::from_raw_parts(*r, 128) };
        eq_bytes("cst table", cb, rb);
    }
}

/* ================================================================== */
/* sha2                                                               */
/* ================================================================== */

#[cfg(feature = "sha2")]
mod sha2 {
    use super::*;

    type Sha = unsafe extern "C" fn(*mut u8, *const u8, usize);
    type IncInit = unsafe extern "C" fn(*mut u8);
    type IncBlocks = unsafe extern "C" fn(*mut u8, *const u8, usize);
    type IncFinalize = unsafe extern "C" fn(*mut u8, *mut u8, *const u8, usize);
    type Mgf1 = unsafe extern "C" fn(*mut u8, u64, *const u8, u64);
    type SeedState = unsafe extern "C" fn(*mut u8);

    fn inlens() -> Vec<usize> {
        vec![0, 1, 2, 54, 55, 56, 57, 63, 64, 65, 110, 111, 112, 113, 127, 128, 129, 200, 255]
    }

    /* ---- C15 ------------------------------------------------------ */
    #[test]
    fn c15_sha_oneshot() {
        let l = libs();
        let mut rng = Rng::new(SEED + 15);
        for (name, outlen) in [("sha256", 32usize), ("sha512", 64)] {
            let (c, r) = l.pair::<Sha>(name);
            for len in inlens() {
                for _ in 0..4 {
                    let inp = rng.bytes(len);
                    let mut co = vec![0u8; outlen];
                    let mut ro = vec![0u8; outlen];
                    unsafe {
                        c(co.as_mut_ptr(), inp.as_ptr(), len);
                        r(ro.as_mut_ptr(), inp.as_ptr(), len);
                    }
                    eq_bytes(&format!("{name}(len={len})"), &co, &ro);
                }
            }
        }
    }

    /* ---- C16 ------------------------------------------------------ */
    #[test]
    fn c16_sha_incremental() {
        let l = libs();
        let mut rng = Rng::new(SEED + 16);
        // (init, blocks, finalize, state bytes, block bytes, digest bytes)
        let variants: [(&str, &str, &str, usize, usize, usize); 2] = [
            ("sha256_inc_init", "sha256_inc_blocks", "sha256_inc_finalize", 40, 64, 32),
            ("sha512_inc_init", "sha512_inc_blocks", "sha512_inc_finalize", 72, 128, 64),
        ];
        for (i_name, b_name, f_name, slen, blen, dlen) in variants {
            let (ci, ri) = l.pair::<IncInit>(i_name);
            let (cb, rb) = l.pair::<IncBlocks>(b_name);
            let (cf, rf) = l.pair::<IncFinalize>(f_name);
            for nblocks in 0usize..=3 {
                for tail in [0usize, 1, blen - 9, blen - 8, blen - 1, blen, blen + 1, 2 * blen - 9] {
                    let blocks = rng.bytes(nblocks * blen);
                    let tailbuf = rng.bytes(tail);
                    let mut cs = vec![0u8; slen];
                    let mut rs = vec![0u8; slen];
                    unsafe {
                        ci(cs.as_mut_ptr());
                        ri(rs.as_mut_ptr());
                    }
                    eq_bytes(&format!("{i_name} state"), &cs, &rs);
                    if nblocks > 0 {
                        unsafe {
                            cb(cs.as_mut_ptr(), blocks.as_ptr(), nblocks);
                            rb(rs.as_mut_ptr(), blocks.as_ptr(), nblocks);
                        }
                        eq_bytes(&format!("{b_name}({nblocks}) state"), &cs, &rs);
                    }
                    let mut co = vec![0u8; dlen];
                    let mut ro = vec![0u8; dlen];
                    unsafe {
                        cf(co.as_mut_ptr(), cs.as_mut_ptr(), tailbuf.as_ptr(), tail);
                        rf(ro.as_mut_ptr(), rs.as_mut_ptr(), tailbuf.as_ptr(), tail);
                    }
                    eq_bytes(
                        &format!("{f_name}(nblocks={nblocks},tail={tail}) digest"),
                        &co,
                        &ro,
                    );
                }
            }
        }
    }

    /* ---- C17 ------------------------------------------------------ */
    #[test]
    fn c17_sha_mgf1() {
        let l = libs();
        let mut rng = Rng::new(SEED + 17);
        let mut outlens = vec![1usize, 31, 32, 33, 63, 64, 65, 100, SPX_N, SPX_N * 2];
        outlens.push(SPX_WOTS_LEN * SPX_N);
        outlens.push(SPX_FORS_TREES as usize * SPX_N);
        outlens.sort_unstable();
        outlens.dedup();
        for name in ["SPX_mgf1_256", "SPX_mgf1_512"] {
            let (c, r) = l.pair::<Mgf1>(name);
            for &outlen in &outlens {
                for inlen in [1usize, SPX_N + 22, 48, 64, 100] {
                    let inp = rng.bytes(inlen);
                    let mut co = vec![0u8; outlen];
                    let mut ro = vec![0u8; outlen];
                    unsafe {
                        c(co.as_mut_ptr(), outlen as u64, inp.as_ptr(), inlen as u64);
                        r(ro.as_mut_ptr(), outlen as u64, inp.as_ptr(), inlen as u64);
                    }
                    eq_bytes(&format!("{name}(outlen={outlen},inlen={inlen})"), &co, &ro);
                }
            }
        }
    }

    /* ---- C18 ------------------------------------------------------ */
    #[test]
    fn c18_sha_seed_state() {
        let l = libs();
        let (c, r) = l.pair::<SeedState>("SPX_seed_state");
        let mut rng = Rng::new(SEED + 18);
        for i in 0..NUM_ITERS {
            let pub_seed = match i {
                0 => vec![0u8; SPX_N],
                1 => vec![0xffu8; SPX_N],
                _ => rng.bytes(SPX_N),
            };
            let sk_seed = rng.bytes(SPX_N);
            let mut cc = Ctx::new();
            let mut rc = Ctx::new();
            cc.set_seeds(&pub_seed, &sk_seed);
            rc.set_seeds(&pub_seed, &sk_seed);
            unsafe {
                c(cc.as_mut_ptr());
                r(rc.as_mut_ptr());
            }
            eq_bytes("SPX_seed_state spx_ctx image", cc.bytes(), rc.bytes());
        }
    }
}

/* ================================================================== */
/* shake                                                              */
/* ================================================================== */

#[cfg(all(any(feature = "shake", feature = "shake256"), not(feature = "sha2")))]
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

    /* ---- C19 ------------------------------------------------------ */
    #[test]
    fn c19_shake256_oneshot() {
        let l = libs();
        let (c, r) = l.pair::<Shake>("shake256");
        let mut rng = Rng::new(SEED + 19);
        let outlens = [1usize, 31, 32, RATE - 1, RATE, RATE + 1, 2 * RATE, 2 * RATE + 1, 271, 272];
        let inlens = [0usize, 1, 2, RATE - 1, RATE, RATE + 1, 2 * RATE, 200, 500];
        for &outlen in &outlens {
            for &inlen in &inlens {
                let inp = rng.bytes(inlen);
                let mut co = vec![0u8; outlen];
                let mut ro = vec![0u8; outlen];
                unsafe {
                    c(co.as_mut_ptr(), outlen, inp.as_ptr(), inlen);
                    r(ro.as_mut_ptr(), outlen, inp.as_ptr(), inlen);
                }
                eq_bytes(&format!("shake256(outlen={outlen},inlen={inlen})"), &co, &ro);
            }
        }
    }

    /* ---- C20 ------------------------------------------------------ */
    #[test]
    fn c20_shake_absorb_squeeze() {
        let l = libs();
        let (ca, ra) = l.pair::<Absorb>("shake256_absorb");
        let (cs, rs) = l.pair::<SqueezeBlocks>("shake256_squeezeblocks");
        let mut rng = Rng::new(SEED + 20);
        for inlen in [0usize, 1, RATE - 1, RATE, RATE + 1, 2 * RATE, 300] {
            for nblocks in 1usize..=3 {
                let inp = rng.bytes(inlen);
                let mut cst = [0u64; 25];
                let mut rst = [0u64; 25];
                unsafe {
                    ca(cst.as_mut_ptr(), inp.as_ptr(), inlen);
                    ra(rst.as_mut_ptr(), inp.as_ptr(), inlen);
                }
                assert_eq!(cst, rst, "shake256_absorb state (inlen={inlen})");
                let mut co = vec![0u8; nblocks * RATE];
                let mut ro = vec![0u8; nblocks * RATE];
                unsafe {
                    cs(co.as_mut_ptr(), nblocks, cst.as_mut_ptr());
                    rs(ro.as_mut_ptr(), nblocks, rst.as_mut_ptr());
                }
                eq_bytes(
                    &format!("shake256_squeezeblocks(inlen={inlen},nblocks={nblocks})"),
                    &co,
                    &ro,
                );
                assert_eq!(cst, rst, "state after squeezeblocks");
            }
        }
    }

    /* ---- C21 ------------------------------------------------------ */
    #[test]
    fn c21_shake_incremental() {
        let l = libs();
        let (ci, ri) = l.pair::<IncInit>("shake256_inc_init");
        let (ca, ra) = l.pair::<IncAbsorb>("shake256_inc_absorb");
        let (cf, rf) = l.pair::<IncFinalize>("shake256_inc_finalize");
        let (cq, rq) = l.pair::<IncSqueeze>("shake256_inc_squeeze");
        let mut rng = Rng::new(SEED + 21);
        for total in [0usize, 1, RATE - 1, RATE, RATE + 1, 2 * RATE, 300] {
            for nchunks in 1usize..=5 {
                let inp = rng.bytes(total);
                let mut cst = [0u64; 26];
                let mut rst = [0u64; 26];
                unsafe {
                    ci(cst.as_mut_ptr());
                    ri(rst.as_mut_ptr());
                }
                assert_eq!(cst, rst, "shake256_inc_init state");
                let mut cuts: Vec<usize> =
                    (0..nchunks).map(|_| rng.below(total as u64 + 1) as usize).collect();
                cuts.push(0);
                cuts.push(total);
                cuts.sort_unstable();
                for w in cuts.windows(2) {
                    let (a, b) = (w[0], w[1]);
                    unsafe {
                        ca(cst.as_mut_ptr(), inp.as_ptr().add(a), b - a);
                        ra(rst.as_mut_ptr(), inp.as_ptr().add(a), b - a);
                    }
                    assert_eq!(cst, rst, "inc_absorb state (total={total}, {a}..{b})");
                }
                unsafe {
                    cf(cst.as_mut_ptr());
                    rf(rst.as_mut_ptr());
                }
                assert_eq!(cst, rst, "inc_finalize state");
                for sq in [1usize, 32, RATE - 1, RATE, RATE + 1] {
                    let mut co = vec![0u8; sq];
                    let mut ro = vec![0u8; sq];
                    unsafe {
                        cq(co.as_mut_ptr(), sq, cst.as_mut_ptr());
                        rq(ro.as_mut_ptr(), sq, rst.as_mut_ptr());
                    }
                    eq_bytes(&format!("inc_squeeze({sq}) total={total}"), &co, &ro);
                    assert_eq!(cst, rst, "state after inc_squeeze({sq})");
                }
            }
        }
    }
}

/* ================================================================== */
/* haraka                                                             */
/* ================================================================== */

#[cfg(all(
    not(feature = "sha2"),
    not(feature = "shake"),
    not(feature = "shake256"),
    not(feature = "blake")
))]
mod haraka {
    use super::*;

    type Tweak = unsafe extern "C" fn(*mut u8);
    type Perm = unsafe extern "C" fn(*mut u8, *const u8, *const u8);
    type HarakaS = unsafe extern "C" fn(*mut u8, u64, *const u8, u64, *const u8);
    type IncInit = unsafe extern "C" fn(*mut u8);
    type IncAbsorb = unsafe extern "C" fn(*mut u8, *const u8, usize, *const u8);
    type IncFinalize = unsafe extern "C" fn(*mut u8);
    type IncSqueeze = unsafe extern "C" fn(*mut u8, usize, *mut u8, *const u8);

    /* ---- C22 ------------------------------------------------------ */
    #[test]
    fn c22_haraka_tweak_constants() {
        let l = libs();
        let (c, r) = l.pair::<Tweak>("SPX_tweak_constants");
        let mut rng = Rng::new(SEED + 22);
        for i in 0..NUM_ITERS {
            let (pub_seed, sk_seed) = match i {
                0 => (vec![0u8; SPX_N], vec![0u8; SPX_N]),
                1 => (vec![0xffu8; SPX_N], vec![0xffu8; SPX_N]),
                _ => (rng.bytes(SPX_N), rng.bytes(SPX_N)),
            };
            let mut cc = Ctx::new();
            let mut rc = Ctx::new();
            cc.set_seeds(&pub_seed, &sk_seed);
            rc.set_seeds(&pub_seed, &sk_seed);
            unsafe {
                c(cc.as_mut_ptr());
                r(rc.as_mut_ptr());
            }
            eq_bytes("SPX_tweak_constants spx_ctx image", cc.bytes(), rc.bytes());
        }
    }

    /* ---- C23 ------------------------------------------------------ */
    #[test]
    fn c23_haraka_perms() {
        let l = libs();
        let mut rng = Rng::new(SEED + 23);
        let (cc, rc) = init_ctx_pair(&rng.bytes(SPX_N), &rng.bytes(SPX_N));
        for (name, inlen, outlen) in [
            ("SPX_haraka256", 32usize, 32usize),
            ("SPX_haraka512", 64, 32),
            ("SPX_haraka512_perm", 64, 64),
        ] {
            let (c, r) = l.pair::<Perm>(name);
            for i in 0..NUM_ITERS {
                let inp = match i {
                    0 => vec![0u8; inlen],
                    1 => vec![0xffu8; inlen],
                    _ => rng.bytes(inlen),
                };
                let mut co = vec![0u8; outlen];
                let mut ro = vec![0u8; outlen];
                unsafe {
                    c(co.as_mut_ptr(), inp.as_ptr(), cc.as_ptr());
                    r(ro.as_mut_ptr(), inp.as_ptr(), rc.as_ptr());
                }
                eq_bytes(name, &co, &ro);
            }
        }
    }

    /* ---- C24 ------------------------------------------------------ */
    #[test]
    fn c24_haraka_S() {
        let l = libs();
        let (c, r) = l.pair::<HarakaS>("SPX_haraka_S");
        let mut rng = Rng::new(SEED + 24);
        let (cc, rc) = init_ctx_pair(&rng.bytes(SPX_N), &rng.bytes(SPX_N));
        let mut outlens = vec![1usize, 31, 32, 33, 63, 64, 65, 100, SPX_N];
        outlens.push(SPX_WOTS_LEN * SPX_N);
        outlens.push(SPX_FORS_TREES as usize * SPX_N);
        outlens.sort_unstable();
        outlens.dedup();
        for &outlen in &outlens {
            for inlen in [0usize, 1, 31, 32, 33, 63, 64, 65, 200] {
                let inp = rng.bytes(inlen);
                let mut co = vec![0u8; outlen];
                let mut ro = vec![0u8; outlen];
                unsafe {
                    c(co.as_mut_ptr(), outlen as u64, inp.as_ptr(), inlen as u64, cc.as_ptr());
                    r(ro.as_mut_ptr(), outlen as u64, inp.as_ptr(), inlen as u64, rc.as_ptr());
                }
                eq_bytes(&format!("SPX_haraka_S(outlen={outlen},inlen={inlen})"), &co, &ro);
            }
        }
    }

    /* ---- C25 ------------------------------------------------------ */
    #[test]
    fn c25_haraka_S_incremental() {
        let l = libs();
        let (ci, ri) = l.pair::<IncInit>("SPX_haraka_S_inc_init");
        let (ca, ra) = l.pair::<IncAbsorb>("SPX_haraka_S_inc_absorb");
        let (cf, rf) = l.pair::<IncFinalize>("SPX_haraka_S_inc_finalize");
        let (cq, rq) = l.pair::<IncSqueeze>("SPX_haraka_S_inc_squeeze");
        let mut rng = Rng::new(SEED + 25);
        let (cc, rc) = init_ctx_pair(&rng.bytes(SPX_N), &rng.bytes(SPX_N));
        for total in [0usize, 1, 31, 32, 33, 64, 65, 200] {
            for nchunks in 1usize..=5 {
                let inp = rng.bytes(total);
                let mut cst = [0u8; 65];
                let mut rst = [0u8; 65];
                unsafe {
                    ci(cst.as_mut_ptr());
                    ri(rst.as_mut_ptr());
                }
                eq_bytes("haraka_S_inc_init state", &cst, &rst);
                let mut cuts: Vec<usize> =
                    (0..nchunks).map(|_| rng.below(total as u64 + 1) as usize).collect();
                cuts.push(0);
                cuts.push(total);
                cuts.sort_unstable();
                for w in cuts.windows(2) {
                    let (a, b) = (w[0], w[1]);
                    unsafe {
                        ca(cst.as_mut_ptr(), inp.as_ptr().add(a), b - a, cc.as_ptr());
                        ra(rst.as_mut_ptr(), inp.as_ptr().add(a), b - a, rc.as_ptr());
                    }
                    eq_bytes(&format!("inc_absorb state total={total} {a}..{b}"), &cst, &rst);
                }
                unsafe {
                    cf(cst.as_mut_ptr());
                    rf(rst.as_mut_ptr());
                }
                eq_bytes("haraka_S_inc_finalize state", &cst, &rst);
                for sq in [1usize, 16, 31, 32, 33, 64] {
                    let mut co = vec![0u8; sq];
                    let mut ro = vec![0u8; sq];
                    unsafe {
                        cq(co.as_mut_ptr(), sq, cst.as_mut_ptr(), cc.as_ptr());
                        rq(ro.as_mut_ptr(), sq, rst.as_mut_ptr(), rc.as_ptr());
                    }
                    eq_bytes(&format!("inc_squeeze({sq}) total={total}"), &co, &ro);
                    eq_bytes("state after inc_squeeze", &cst, &rst);
                }
            }
        }
    }
}
