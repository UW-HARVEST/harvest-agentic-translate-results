//! Phase B rows 11-25: the raw hash primitives of the ACTIVE backend.
//!
//! Each backend section is `#[cfg]`-gated exactly like the C `lib/CMakeLists.txt`
//! `add_subdirectory(${HASH_BACKEND})`.
mod common;
use common::*;

/// Message / output lengths that straddle every block and padding boundary the
/// C implementations special-case.
const LENS: &[usize] = &[
    0, 1, 2, 15, 16, 17, 31, 32, 33, 54, 55, 56, 57, 63, 64, 65, 71, 72, 73, 110, 111, 112, 113,
    127, 128, 129, 135, 136, 137, 168, 200, 255, 256, 257, 500,
];
const OUTLENS: &[usize] = &[0, 1, 2, 31, 32, 33, 63, 64, 65, 100, 135, 136, 137, 272, 300];
const ITERS: usize = 8;

// ===========================================================================
// BLAKE (rows 11-14)
// ===========================================================================
#[cfg(spx_backend = "blake")]
mod blake {
    use super::*;

    type FnHash =
        unsafe extern "C" fn(*mut u8, *const u8, core::ffi::c_ulonglong) -> core::ffi::c_int;
    type FnInit256 = unsafe extern "C" fn(*mut BlakeState256Ffi);
    type FnUpd256 =
        unsafe extern "C" fn(*mut BlakeState256Ffi, *const u8, core::ffi::c_ulonglong);
    type FnFin256 = unsafe extern "C" fn(*mut BlakeState256Ffi, *mut u8);
    type FnCmp256 = unsafe extern "C" fn(*mut BlakeState256Ffi, *const u8);
    type FnInit512 = unsafe extern "C" fn(*mut BlakeState512Ffi);
    type FnUpd512 =
        unsafe extern "C" fn(*mut BlakeState512Ffi, *const u8, core::ffi::c_ulonglong);
    type FnFin512 = unsafe extern "C" fn(*mut BlakeState512Ffi, *mut u8);
    type FnCmp512 = unsafe extern "C" fn(*mut BlakeState512Ffi, *const u8);
    type FnMgf1 = unsafe extern "C" fn(
        *mut u8,
        core::ffi::c_ulong,
        *const u8,
        core::ffi::c_ulong,
    );

    #[test]
    fn row11_blake_oneshot_all_lengths() {
        let l = libs();
        let mut rng = Rng::new(0x1111);
        for (name, outb) in [("blake256", 32usize), ("blake512", 64usize)] {
            let (c, r) = l.pair::<FnHash>(name);
            for &n in LENS {
                for _ in 0..ITERS {
                    let inp = rng.bytes(n);
                    let mut co = vec![0xA5u8; outb + 8];
                    let mut ro = vec![0xA5u8; outb + 8];
                    let cv =
                        unsafe { c(co.as_mut_ptr(), inp.as_ptr(), n as core::ffi::c_ulonglong) };
                    let rv =
                        unsafe { r(ro.as_mut_ptr(), inp.as_ptr(), n as core::ffi::c_ulonglong) };
                    assert_eq_dbg(&format!("{name}(inlen={n}) retval"), cv, rv);
                    assert_bytes_eq(&format!("{name}(inlen={n})"), &co, &ro);
                }
            }
        }
    }

    #[test]
    fn row12_blake256_incremental() {
        let l = libs();
        let (ci, ri) = l.pair::<FnInit256>("blake256_init");
        let (cu, ru) = l.pair::<FnUpd256>("blake256_update");
        let (cf, rf) = l.pair::<FnFin256>("blake256_final");
        let (co, ro) = l.pair::<FnHash>("blake256");
        let mut rng = Rng::new(0x1212);
        for &n in LENS {
            for _ in 0..ITERS {
                let inp = rng.bytes(n);
                // random split into 1..5 chunks
                let nchunks = 1 + rng.below(5);
                let mut cuts: Vec<usize> = (0..nchunks - 1).map(|_| rng.below(n + 1)).collect();
                cuts.push(n);
                cuts.push(0);
                cuts.sort_unstable();

                let mut cs: BlakeState256Ffi = unsafe { core::mem::zeroed() };
                let mut rs: BlakeState256Ffi = unsafe { core::mem::zeroed() };
                unsafe {
                    ci(&mut cs);
                    ri(&mut rs);
                }
                // The state must match after init, and after EVERY update.
                assert_state256("after init", &cs, &rs);
                for w in cuts.windows(2) {
                    let (a, b) = (w[0], w[1]);
                    unsafe {
                        cu(&mut cs, inp[a..].as_ptr(), ((b - a) * 8) as core::ffi::c_ulonglong); // NOTE: blake*_update takes BITS
                        ru(&mut rs, inp[a..].as_ptr(), ((b - a) * 8) as core::ffi::c_ulonglong); // NOTE: blake*_update takes BITS
                    }
                    assert_state256(&format!("after update({}..{})", a, b), &cs, &rs);
                }
                let mut cd = [0xA5u8; 40];
                let mut rd = [0xA5u8; 40];
                unsafe {
                    cf(&mut cs, cd.as_mut_ptr());
                    rf(&mut rs, rd.as_mut_ptr());
                }
                assert_bytes_eq(&format!("blake256 incremental digest (inlen={n})"), &cd, &rd);

                // Separately: a SINGLE non-empty update must reproduce the
                // one-shot. (A *zero-length* update is NOT a no-op in the C --
                // `blake256_update` line 327 does `else S->buflen = 0`, wiping
                // any buffered bytes -- so the multi-chunk digest above may
                // legitimately differ from the one-shot. That quirk is covered
                // by the per-update state comparison.)
                let mut cs2: BlakeState256Ffi = unsafe { core::mem::zeroed() };
                let mut rs2: BlakeState256Ffi = unsafe { core::mem::zeroed() };
                let mut cd2 = [0xA5u8; 40];
                let mut rd2 = [0xA5u8; 40];
                let mut c1 = [0xA5u8; 40];
                let mut r1 = [0xA5u8; 40];
                unsafe {
                    ci(&mut cs2);
                    ri(&mut rs2);
                    cu(&mut cs2, inp.as_ptr(), (n * 8) as core::ffi::c_ulonglong);
                    ru(&mut rs2, inp.as_ptr(), (n * 8) as core::ffi::c_ulonglong);
                    cf(&mut cs2, cd2.as_mut_ptr());
                    rf(&mut rs2, rd2.as_mut_ptr());
                    co(c1.as_mut_ptr(), inp.as_ptr(), n as core::ffi::c_ulonglong);
                    ro(r1.as_mut_ptr(), inp.as_ptr(), n as core::ffi::c_ulonglong);
                }
                assert_bytes_eq(&format!("blake256 1-update digest (inlen={n})"), &cd2, &rd2);
                assert_bytes_eq(&format!("blake256 1-update == one-shot, C (inlen={n})"), &c1, &cd2);
                assert_bytes_eq(&format!("blake256 1-update == one-shot, Rust (inlen={n})"), &r1, &rd2);
            }
        }
    }

    #[test]
    fn row12_blake512_incremental() {
        let l = libs();
        let (ci, ri) = l.pair::<FnInit512>("blake512_init");
        let (cu, ru) = l.pair::<FnUpd512>("blake512_update");
        let (cf, rf) = l.pair::<FnFin512>("blake512_final");
        let mut rng = Rng::new(0x1213);
        for &n in LENS {
            for _ in 0..ITERS {
                let inp = rng.bytes(n);
                let nchunks = 1 + rng.below(5);
                let mut cuts: Vec<usize> = (0..nchunks - 1).map(|_| rng.below(n + 1)).collect();
                cuts.push(n);
                cuts.push(0);
                cuts.sort_unstable();

                let mut cs: BlakeState512Ffi = unsafe { core::mem::zeroed() };
                let mut rs: BlakeState512Ffi = unsafe { core::mem::zeroed() };
                unsafe {
                    ci(&mut cs);
                    ri(&mut rs);
                }
                assert_state512("after init", &cs, &rs);
                for w in cuts.windows(2) {
                    let (a, b) = (w[0], w[1]);
                    unsafe {
                        cu(&mut cs, inp[a..].as_ptr(), ((b - a) * 8) as core::ffi::c_ulonglong); // NOTE: blake*_update takes BITS
                        ru(&mut rs, inp[a..].as_ptr(), ((b - a) * 8) as core::ffi::c_ulonglong); // NOTE: blake*_update takes BITS
                    }
                    assert_state512(&format!("after update({}..{})", a, b), &cs, &rs);
                }
                let mut cd = [0xA5u8; 72];
                let mut rd = [0xA5u8; 72];
                unsafe {
                    cf(&mut cs, cd.as_mut_ptr());
                    rf(&mut rs, rd.as_mut_ptr());
                }
                assert_bytes_eq(&format!("blake512 incremental digest (inlen={n})"), &cd, &rd);
            }
        }
    }

    #[test]
    fn row13_blake_compress() {
        let l = libs();
        let (c2, r2) = l.pair::<FnCmp256>("blake256_compress");
        let (c5, r5) = l.pair::<FnCmp512>("blake512_compress");
        let mut rng = Rng::new(0x1313);
        for i in 0..256 {
            // 256-bit
            let mut cs: BlakeState256Ffi = unsafe { core::mem::zeroed() };
            for w in cs.h.iter_mut() {
                *w = rng.next_u32();
            }
            for w in cs.s.iter_mut() {
                *w = rng.next_u32();
            }
            for w in cs.t.iter_mut() {
                *w = rng.next_u32();
            }
            cs.nullt = (i % 2) as core::ffi::c_int;
            let block = rng.bytes(64);
            let mut rs = cs.clone();
            unsafe {
                c2(&mut cs, block.as_ptr());
                r2(&mut rs, block.as_ptr());
            }
            assert_state256(&format!("blake256_compress #{i}"), &cs, &rs);

            // 512-bit
            let mut cs5: BlakeState512Ffi = unsafe { core::mem::zeroed() };
            for w in cs5.h.iter_mut() {
                *w = rng.next_u64();
            }
            for w in cs5.s.iter_mut() {
                *w = rng.next_u64();
            }
            for w in cs5.t.iter_mut() {
                *w = rng.next_u64();
            }
            cs5.nullt = (i % 2) as core::ffi::c_int;
            let block5 = rng.bytes(128);
            let mut rs5 = cs5.clone();
            unsafe {
                c5(&mut cs5, block5.as_ptr());
                r5(&mut rs5, block5.as_ptr());
            }
            assert_state512(&format!("blake512_compress #{i}"), &cs5, &rs5);
        }
    }

    #[test]
    fn row14_blake_mgf1() {
        let l = libs();
        let mut rng = Rng::new(0x1414);
        for name in ["SPX_blake256_mgf1", "SPX_blake512_mgf1"] {
            let (c, r) = l.pair::<FnMgf1>(name);
            for &outlen in OUTLENS {
                for &inlen in &[0usize, 1, 16, 32, 48, 64, 96, 128] {
                    for _ in 0..ITERS {
                        let inp = rng.bytes(inlen.max(1));
                        let mut co = vec![0xA5u8; outlen + 8];
                        let mut ro = vec![0xA5u8; outlen + 8];
                        unsafe {
                            c(
                                co.as_mut_ptr(),
                                outlen as core::ffi::c_ulong,
                                inp.as_ptr(),
                                inlen as core::ffi::c_ulong,
                            );
                            r(
                                ro.as_mut_ptr(),
                                outlen as core::ffi::c_ulong,
                                inp.as_ptr(),
                                inlen as core::ffi::c_ulong,
                            );
                        }
                        assert_bytes_eq(
                            &format!("{name}(outlen={outlen}, inlen={inlen})"),
                            &co,
                            &ro,
                        );
                    }
                }
            }
        }
    }

    /// The `cst` data symbol is exported by both `.so`s; verify the 128 bytes
    /// are identical through `dlsym`.
    #[test]
    fn blake512_cst_data_symbol() {
        let l = libs();
        let c = l.sym::<*const u64>(Which::C, "cst");
        let r = l.sym::<*const u64>(Which::R, "cst");
        let cb = unsafe { core::slice::from_raw_parts(*c as *const u8, 128) };
        let rb = unsafe { core::slice::from_raw_parts(*r as *const u8, 128) };
        assert_bytes_eq("cst[16] (blake512 round constants)", cb, rb);
    }

    #[track_caller]
    fn assert_state256(what: &str, c: &BlakeState256Ffi, r: &BlakeState256Ffi) {
        let cb = unsafe {
            core::slice::from_raw_parts(c as *const _ as *const u8, core::mem::size_of::<BlakeState256Ffi>())
        };
        let rb = unsafe {
            core::slice::from_raw_parts(r as *const _ as *const u8, core::mem::size_of::<BlakeState256Ffi>())
        };
        assert_bytes_eq(&format!("blakestate256 {what}"), cb, rb);
    }

    #[track_caller]
    fn assert_state512(what: &str, c: &BlakeState512Ffi, r: &BlakeState512Ffi) {
        let cb = unsafe {
            core::slice::from_raw_parts(c as *const _ as *const u8, core::mem::size_of::<BlakeState512Ffi>())
        };
        let rb = unsafe {
            core::slice::from_raw_parts(r as *const _ as *const u8, core::mem::size_of::<BlakeState512Ffi>())
        };
        assert_bytes_eq(&format!("blakestate512 {what}"), cb, rb);
    }
}

// ===========================================================================
// SHA-2 (rows 15-18)
// ===========================================================================
#[cfg(spx_backend = "sha2")]
mod sha2 {
    use super::*;

    type FnHash = unsafe extern "C" fn(*mut u8, *const u8, usize);
    type FnIncInit = unsafe extern "C" fn(*mut u8);
    type FnIncBlocks = unsafe extern "C" fn(*mut u8, *const u8, usize);
    type FnIncFinal = unsafe extern "C" fn(*mut u8, *mut u8, *const u8, usize);
    type FnMgf1 =
        unsafe extern "C" fn(*mut u8, core::ffi::c_ulong, *const u8, core::ffi::c_ulong);
    type FnSeedState = unsafe extern "C" fn(*mut SpxCtxFfi);

    #[test]
    fn row15_sha2_oneshot_all_lengths() {
        let l = libs();
        let mut rng = Rng::new(0x1515);
        for (name, outb) in [("sha256", 32usize), ("sha512", 64usize)] {
            let (c, r) = l.pair::<FnHash>(name);
            for &n in LENS {
                for _ in 0..ITERS {
                    let inp = rng.bytes(n);
                    let mut co = vec![0xA5u8; outb + 8];
                    let mut ro = vec![0xA5u8; outb + 8];
                    unsafe {
                        c(co.as_mut_ptr(), inp.as_ptr(), n);
                        r(ro.as_mut_ptr(), inp.as_ptr(), n);
                    }
                    assert_bytes_eq(&format!("{name}(inlen={n})"), &co, &ro);
                }
            }
        }
    }

    #[test]
    fn row16_sha2_incremental() {
        let l = libs();
        let mut rng = Rng::new(0x1616);
        for (init, blocks, fin, state_len, block_bytes, out_bytes) in [
            ("sha256_inc_init", "sha256_inc_blocks", "sha256_inc_finalize", 40usize, 64usize, 32usize),
            ("sha512_inc_init", "sha512_inc_blocks", "sha512_inc_finalize", 72, 128, 64),
        ] {
            let (ci, ri) = l.pair::<FnIncInit>(init);
            let (cb, rb) = l.pair::<FnIncBlocks>(blocks);
            let (cf, rf) = l.pair::<FnIncFinal>(fin);
            for &nblocks in &[0usize, 1, 2, 3] {
                for &tail in &[0usize, 1, 55, 56, 57, 63, 64, 65, 111, 112, 127, 128, 200] {
                    for _ in 0..ITERS {
                        let bulk = rng.bytes(nblocks * block_bytes + 1);
                        let tailbuf = rng.bytes(tail + 1);
                        let mut cs = vec![0xA5u8; state_len];
                        let mut rs = vec![0xA5u8; state_len];
                        unsafe {
                            ci(cs.as_mut_ptr());
                            ri(rs.as_mut_ptr());
                        }
                        assert_bytes_eq(&format!("{init} state"), &cs, &rs);
                        unsafe {
                            cb(cs.as_mut_ptr(), bulk.as_ptr(), nblocks);
                            rb(rs.as_mut_ptr(), bulk.as_ptr(), nblocks);
                        }
                        assert_bytes_eq(&format!("{blocks}({nblocks}) state"), &cs, &rs);
                        let mut cd = vec![0xA5u8; out_bytes + 8];
                        let mut rd = vec![0xA5u8; out_bytes + 8];
                        unsafe {
                            cf(cd.as_mut_ptr(), cs.as_mut_ptr(), tailbuf.as_ptr(), tail);
                            rf(rd.as_mut_ptr(), rs.as_mut_ptr(), tailbuf.as_ptr(), tail);
                        }
                        assert_bytes_eq(
                            &format!("{fin}(nblocks={nblocks}, tail={tail}) digest"),
                            &cd,
                            &rd,
                        );
                        assert_bytes_eq(
                            &format!("{fin}(nblocks={nblocks}, tail={tail}) state after"),
                            &cs,
                            &rs,
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn row17_sha2_mgf1() {
        let l = libs();
        let mut rng = Rng::new(0x1717);
        for name in ["SPX_mgf1_256", "SPX_mgf1_512"] {
            let (c, r) = l.pair::<FnMgf1>(name);
            for &outlen in OUTLENS {
                for &inlen in &[0usize, 1, 16, 32, 48, 64, 96, 128] {
                    for _ in 0..ITERS {
                        let inp = rng.bytes(inlen.max(1));
                        let mut co = vec![0xA5u8; outlen + 8];
                        let mut ro = vec![0xA5u8; outlen + 8];
                        unsafe {
                            c(
                                co.as_mut_ptr(),
                                outlen as core::ffi::c_ulong,
                                inp.as_ptr(),
                                inlen as core::ffi::c_ulong,
                            );
                            r(
                                ro.as_mut_ptr(),
                                outlen as core::ffi::c_ulong,
                                inp.as_ptr(),
                                inlen as core::ffi::c_ulong,
                            );
                        }
                        assert_bytes_eq(
                            &format!("{name}(outlen={outlen}, inlen={inlen})"),
                            &co,
                            &ro,
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn row18_sha2_seed_state() {
        let l = libs();
        let (c, r) = l.pair::<FnSeedState>("SPX_seed_state");
        let mut rng = Rng::new(0x1818);
        for _ in 0..128 {
            let mut cc = SpxCtxFfi::zeroed();
            rng.fill(&mut cc.pub_seed);
            rng.fill(&mut cc.sk_seed);
            let mut rc = cc.clone();
            unsafe {
                c(&mut cc);
                r(&mut rc);
            }
            assert_bytes_eq("SPX_seed_state -> spx_ctx", cc.as_bytes(), rc.as_bytes());
        }
    }
}

// ===========================================================================
// SHAKE (rows 19-21)
// ===========================================================================
#[cfg(spx_backend = "shake")]
mod shake {
    use super::*;

    type FnShake = unsafe extern "C" fn(*mut u8, usize, *const u8, usize);
    type FnAbsorb = unsafe extern "C" fn(*mut u64, *const u8, usize);
    type FnSqueezeBlocks = unsafe extern "C" fn(*mut u8, usize, *mut u64);
    type FnIncInit = unsafe extern "C" fn(*mut u64);
    type FnIncAbsorb = unsafe extern "C" fn(*mut u64, *const u8, usize);
    type FnIncFinal = unsafe extern "C" fn(*mut u64);
    type FnIncSqueeze = unsafe extern "C" fn(*mut u8, usize, *mut u64);

    #[test]
    fn row19_shake256_oneshot() {
        let l = libs();
        let (c, r) = l.pair::<FnShake>("shake256");
        let mut rng = Rng::new(0x1919);
        for &outlen in OUTLENS {
            for &inlen in LENS {
                let inp = rng.bytes(inlen.max(1));
                let mut co = vec![0xA5u8; outlen + 8];
                let mut ro = vec![0xA5u8; outlen + 8];
                unsafe {
                    c(co.as_mut_ptr(), outlen, inp.as_ptr(), inlen);
                    r(ro.as_mut_ptr(), outlen, inp.as_ptr(), inlen);
                }
                assert_bytes_eq(&format!("shake256(outlen={outlen}, inlen={inlen})"), &co, &ro);
            }
        }
    }

    #[test]
    fn row20_shake256_absorb_squeezeblocks() {
        let l = libs();
        let (ca, ra) = l.pair::<FnAbsorb>("shake256_absorb");
        let (cs, rs) = l.pair::<FnSqueezeBlocks>("shake256_squeezeblocks");
        let mut rng = Rng::new(0x2020);
        const RATE: usize = 136;
        for &nblocks in &[0usize, 1, 2, 3, 5] {
            for &inlen in LENS {
                let inp = rng.bytes(inlen.max(1));
                let mut cst = [0u64; 25];
                let mut rst = [0u64; 25];
                unsafe {
                    ca(cst.as_mut_ptr(), inp.as_ptr(), inlen);
                    ra(rst.as_mut_ptr(), inp.as_ptr(), inlen);
                }
                assert_bytes_eq(
                    &format!("shake256_absorb(inlen={inlen}) state"),
                    unsafe { core::slice::from_raw_parts(cst.as_ptr() as *const u8, 200) },
                    unsafe { core::slice::from_raw_parts(rst.as_ptr() as *const u8, 200) },
                );
                let mut co = vec![0xA5u8; nblocks * RATE + 8];
                let mut ro = vec![0xA5u8; nblocks * RATE + 8];
                unsafe {
                    cs(co.as_mut_ptr(), nblocks, cst.as_mut_ptr());
                    rs(ro.as_mut_ptr(), nblocks, rst.as_mut_ptr());
                }
                assert_bytes_eq(&format!("shake256_squeezeblocks({nblocks})"), &co, &ro);
                assert_bytes_eq(
                    "shake256_squeezeblocks state after",
                    unsafe { core::slice::from_raw_parts(cst.as_ptr() as *const u8, 200) },
                    unsafe { core::slice::from_raw_parts(rst.as_ptr() as *const u8, 200) },
                );
            }
        }
    }

    #[test]
    fn row21_shake256_incremental() {
        let l = libs();
        let (ci, ri) = l.pair::<FnIncInit>("shake256_inc_init");
        let (ca, ra) = l.pair::<FnIncAbsorb>("shake256_inc_absorb");
        let (cf, rf) = l.pair::<FnIncFinal>("shake256_inc_finalize");
        let (cq, rq) = l.pair::<FnIncSqueeze>("shake256_inc_squeeze");
        let mut rng = Rng::new(0x2121);
        let raw = |s: &[u64; 26]| unsafe {
            core::slice::from_raw_parts(s.as_ptr() as *const u8, 26 * 8)
        };
        for &n in LENS {
            for _ in 0..ITERS {
                let inp = rng.bytes(n);
                let nchunks = 1 + rng.below(5);
                let mut cuts: Vec<usize> = (0..nchunks - 1).map(|_| rng.below(n + 1)).collect();
                cuts.push(n);
                cuts.push(0);
                cuts.sort_unstable();

                let mut cst = [0xA5A5A5A5A5A5A5A5u64; 26];
                let mut rst = [0xA5A5A5A5A5A5A5A5u64; 26];
                unsafe {
                    ci(cst.as_mut_ptr());
                    ri(rst.as_mut_ptr());
                }
                assert_bytes_eq("shake256_inc_init state", raw(&cst), raw(&rst));
                for w in cuts.windows(2) {
                    unsafe {
                        ca(cst.as_mut_ptr(), inp[w[0]..].as_ptr(), w[1] - w[0]);
                        ra(rst.as_mut_ptr(), inp[w[0]..].as_ptr(), w[1] - w[0]);
                    }
                    assert_bytes_eq(
                        &format!("shake256_inc_absorb({}..{}) state", w[0], w[1]),
                        raw(&cst),
                        raw(&rst),
                    );
                }
                unsafe {
                    cf(cst.as_mut_ptr());
                    rf(rst.as_mut_ptr());
                }
                assert_bytes_eq("shake256_inc_finalize state", raw(&cst), raw(&rst));
                // Multiple squeezes of random sizes exercise the partial-block
                // carry held in s_inc[25].
                for _ in 0..4 {
                    let k = rng.below(200);
                    let mut co = vec![0xA5u8; k + 8];
                    let mut ro = vec![0xA5u8; k + 8];
                    unsafe {
                        cq(co.as_mut_ptr(), k, cst.as_mut_ptr());
                        rq(ro.as_mut_ptr(), k, rst.as_mut_ptr());
                    }
                    assert_bytes_eq(&format!("shake256_inc_squeeze({k})"), &co, &ro);
                    assert_bytes_eq("shake256_inc_squeeze state after", raw(&cst), raw(&rst));
                }
            }
        }
    }
}

// ===========================================================================
// HARAKA (rows 22-25)
// ===========================================================================
#[cfg(spx_backend = "haraka")]
mod haraka {
    use super::*;

    type FnTweak = unsafe extern "C" fn(*mut SpxCtxFfi);
    type FnPerm = unsafe extern "C" fn(*mut u8, *const u8, *const SpxCtxFfi);
    type FnS = unsafe extern "C" fn(
        *mut u8,
        core::ffi::c_ulonglong,
        *const u8,
        core::ffi::c_ulonglong,
        *const SpxCtxFfi,
    );
    type FnIncInit = unsafe extern "C" fn(*mut u8);
    type FnIncAbsorb = unsafe extern "C" fn(*mut u8, *const u8, usize, *const SpxCtxFfi);
    type FnIncFinal = unsafe extern "C" fn(*mut u8);
    type FnIncSqueeze = unsafe extern "C" fn(*mut u8, usize, *mut u8, *const SpxCtxFfi);

    fn tweaked_ctx(rng: &mut Rng) -> (SpxCtxFfi, SpxCtxFfi) {
        let l = libs();
        let (c, r) = l.pair::<FnTweak>("SPX_tweak_constants");
        let mut cc = SpxCtxFfi::zeroed();
        rng.fill(&mut cc.pub_seed);
        rng.fill(&mut cc.sk_seed);
        let mut rc = cc.clone();
        unsafe {
            c(&mut cc);
            r(&mut rc);
        }
        assert_bytes_eq("SPX_tweak_constants -> spx_ctx", cc.as_bytes(), rc.as_bytes());
        (cc, rc)
    }

    #[test]
    fn row22_tweak_constants() {
        let mut rng = Rng::new(0x2222);
        for _ in 0..128 {
            let _ = tweaked_ctx(&mut rng);
        }
    }

    #[test]
    fn row23_haraka_permutations() {
        let l = libs();
        let mut rng = Rng::new(0x2323);
        for (name, inb, outb) in [
            ("SPX_haraka256", 32usize, 32usize),
            ("SPX_haraka512", 64, 32),
            ("SPX_haraka512_perm", 64, 64),
        ] {
            let (c, r) = l.pair::<FnPerm>(name);
            for _ in 0..64 {
                let (cc, rc) = tweaked_ctx(&mut rng);
                let inp = rng.bytes(inb);
                let mut co = vec![0xA5u8; outb + 8];
                let mut ro = vec![0xA5u8; outb + 8];
                unsafe {
                    c(co.as_mut_ptr(), inp.as_ptr(), &cc);
                    r(ro.as_mut_ptr(), inp.as_ptr(), &rc);
                }
                assert_bytes_eq(name, &co, &ro);
            }
        }
    }

    #[test]
    fn row24_haraka_S() {
        let l = libs();
        let (c, r) = l.pair::<FnS>("SPX_haraka_S");
        let mut rng = Rng::new(0x2424);
        for &outlen in OUTLENS {
            for &inlen in LENS {
                let (cc, rc) = tweaked_ctx(&mut rng);
                let inp = rng.bytes(inlen.max(1));
                let mut co = vec![0xA5u8; outlen + 8];
                let mut ro = vec![0xA5u8; outlen + 8];
                unsafe {
                    c(
                        co.as_mut_ptr(),
                        outlen as core::ffi::c_ulonglong,
                        inp.as_ptr(),
                        inlen as core::ffi::c_ulonglong,
                        &cc,
                    );
                    r(
                        ro.as_mut_ptr(),
                        outlen as core::ffi::c_ulonglong,
                        inp.as_ptr(),
                        inlen as core::ffi::c_ulonglong,
                        &rc,
                    );
                }
                assert_bytes_eq(
                    &format!("SPX_haraka_S(outlen={outlen}, inlen={inlen})"),
                    &co,
                    &ro,
                );
            }
        }
    }

    #[test]
    fn row25_haraka_S_incremental() {
        let l = libs();
        let (ci, ri) = l.pair::<FnIncInit>("SPX_haraka_S_inc_init");
        let (ca, ra) = l.pair::<FnIncAbsorb>("SPX_haraka_S_inc_absorb");
        let (cf, rf) = l.pair::<FnIncFinal>("SPX_haraka_S_inc_finalize");
        let (cq, rq) = l.pair::<FnIncSqueeze>("SPX_haraka_S_inc_squeeze");
        let mut rng = Rng::new(0x2525);
        for &n in LENS {
            let (cc, rc) = tweaked_ctx(&mut rng);
            let inp = rng.bytes(n);
            let nchunks = 1 + rng.below(5);
            let mut cuts: Vec<usize> = (0..nchunks - 1).map(|_| rng.below(n + 1)).collect();
            cuts.push(n);
            cuts.push(0);
            cuts.sort_unstable();

            let mut cst = [0xA5u8; 65];
            let mut rst = [0xA5u8; 65];
            unsafe {
                ci(cst.as_mut_ptr());
                ri(rst.as_mut_ptr());
            }
            assert_bytes_eq("haraka_S_inc_init state", &cst, &rst);
            for w in cuts.windows(2) {
                unsafe {
                    ca(cst.as_mut_ptr(), inp[w[0]..].as_ptr(), w[1] - w[0], &cc);
                    ra(rst.as_mut_ptr(), inp[w[0]..].as_ptr(), w[1] - w[0], &rc);
                }
                assert_bytes_eq(
                    &format!("haraka_S_inc_absorb({}..{}) state", w[0], w[1]),
                    &cst,
                    &rst,
                );
            }
            unsafe {
                cf(cst.as_mut_ptr());
                rf(rst.as_mut_ptr());
            }
            assert_bytes_eq("haraka_S_inc_finalize state", &cst, &rst);
            for _ in 0..4 {
                let k = rng.below(200);
                let mut co = vec![0xA5u8; k + 8];
                let mut ro = vec![0xA5u8; k + 8];
                unsafe {
                    cq(co.as_mut_ptr(), k, cst.as_mut_ptr(), &cc);
                    rq(ro.as_mut_ptr(), k, rst.as_mut_ptr(), &rc);
                }
                assert_bytes_eq(&format!("haraka_S_inc_squeeze({k})"), &co, &ro);
                assert_bytes_eq("haraka_S_inc_squeeze state after", &cst, &rst);
            }
        }
    }
}
