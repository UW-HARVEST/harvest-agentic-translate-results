//! CONFIGS.md group B, sha2 backend (rows B10–B16).

mod common;

#[cfg(backend_sha2)]
mod sha2 {
    use crate::common::*;
    use crate::{pair_backend};
    use std::os::raw::c_ulong;

    type OneShot = unsafe extern "C" fn(*mut u8, *const u8, usize);
    type IncInit = unsafe extern "C" fn(*mut u8);
    type IncBlocks = unsafe extern "C" fn(*mut u8, *const u8, usize);
    type IncFinal = unsafe extern "C" fn(*mut u8, *mut u8, *const u8, usize);
    type Mgf1 = unsafe extern "C" fn(*mut u8, c_ulong, *const u8, c_ulong);
    type SeedState = unsafe extern "C" fn(*mut u8);

    const LENS_256: &[usize] = &[
        0, 1, 2, 31, 32, 54, 55, 56, 57, 63, 64, 65, 119, 120, 127, 128, 129, 1000,
    ];
    const LENS_512: &[usize] = &[
        0, 1, 2, 63, 64, 110, 111, 112, 113, 127, 128, 129, 239, 240, 255, 256, 257, 1000,
    ];

    fn oneshot_case(libs: &Libs, name: &'static str, outlen: usize, seed: u64, lens: &[usize]) {
        let c: libloading::os::unix::Symbol<OneShot> = libs.c_backend(name);
        let r: libloading::os::unix::Symbol<OneShot> = libs.r(name);
        let mut rng = Rng::new(seed);
        for &n in lens {
            for i in 0..40 {
                let inp: Vec<u8> = match i {
                    0 => vec![0u8; n],
                    1 => vec![0xFFu8; n],
                    _ => rng.bytes(n),
                };
                let mut co = vec![0xAAu8; outlen + 8];
                let mut ro = vec![0xAAu8; outlen + 8];
                unsafe {
                    c(co.as_mut_ptr(), inp.as_ptr(), n);
                    r(ro.as_mut_ptr(), inp.as_ptr(), n);
                }
                eq_bytes(&format!("{name}(inlen={n})"), &co, &ro);
            }
        }
    }

    #[test]
    fn b10_sha256() {
        let libs = Libs::load();
        oneshot_case(&libs, "sha256", 32, 201, LENS_256);
    }

    #[test]
    fn b12_sha512() {
        let libs = Libs::load();
        oneshot_case(&libs, "sha512", 64, 202, LENS_512);
    }

    /// Drives init / inc_blocks / inc_finalize and compares the digest **and**
    /// the full state after every call.
    fn incremental_case(
        libs: &Libs,
        init: &'static str,
        blocks: &'static str,
        finalize: &'static str,
        state_len: usize,
        block_bytes: usize,
        out_len: usize,
        seed: u64,
        tails: &[usize],
    ) {
        let ci: libloading::os::unix::Symbol<IncInit> = libs.c_backend(init);
        let ri: libloading::os::unix::Symbol<IncInit> = libs.r(init);
        let cb: libloading::os::unix::Symbol<IncBlocks> = libs.c_backend(blocks);
        let rb: libloading::os::unix::Symbol<IncBlocks> = libs.r(blocks);
        let cf: libloading::os::unix::Symbol<IncFinal> = libs.c_backend(finalize);
        let rf: libloading::os::unix::Symbol<IncFinal> = libs.r(finalize);
        let mut rng = Rng::new(seed);

        for &nblocks in &[0usize, 1, 2, 3, 5] {
            for &tail in tails {
                for _ in 0..12 {
                    let mut cs = vec![0u8; state_len];
                    let mut rs = vec![0u8; state_len];
                    unsafe {
                        ci(cs.as_mut_ptr());
                        ri(rs.as_mut_ptr());
                    }
                    eq_bytes(&format!("{init} state"), &cs, &rs);

                    let blk = rng.bytes(nblocks * block_bytes + 1);
                    if nblocks > 0 {
                        unsafe {
                            cb(cs.as_mut_ptr(), blk.as_ptr(), nblocks);
                            rb(rs.as_mut_ptr(), blk.as_ptr(), nblocks);
                        }
                        eq_bytes(&format!("{blocks}(nblocks={nblocks}) state"), &cs, &rs);
                    }

                    // A second, separate inc_blocks call to exercise chaining.
                    let blk2 = rng.bytes(block_bytes + 1);
                    unsafe {
                        cb(cs.as_mut_ptr(), blk2.as_ptr(), 1);
                        rb(rs.as_mut_ptr(), blk2.as_ptr(), 1);
                    }
                    eq_bytes(&format!("{blocks} chained state"), &cs, &rs);

                    let t = rng.bytes(tail + 1);
                    let mut co = vec![0xAAu8; out_len + 8];
                    let mut ro = vec![0xAAu8; out_len + 8];
                    unsafe {
                        cf(co.as_mut_ptr(), cs.as_mut_ptr(), t.as_ptr(), tail);
                        rf(ro.as_mut_ptr(), rs.as_mut_ptr(), t.as_ptr(), tail);
                    }
                    eq_bytes(&format!("{finalize}(tail={tail}) digest"), &co, &ro);
                    eq_bytes(&format!("{finalize}(tail={tail}) state"), &cs, &rs);
                }
            }
        }
    }

    #[test]
    fn b11_sha256_incremental() {
        let libs = Libs::load();
        incremental_case(
            &libs,
            "sha256_inc_init",
            "sha256_inc_blocks",
            "sha256_inc_finalize",
            40,
            64,
            32,
            203,
            &[0, 1, 54, 55, 56, 63, 64, 65, 119, 120, 200],
        );
    }

    #[test]
    fn b13_sha512_incremental() {
        let libs = Libs::load();
        incremental_case(
            &libs,
            "sha512_inc_init",
            "sha512_inc_blocks",
            "sha512_inc_finalize",
            72,
            128,
            64,
            204,
            &[0, 1, 110, 111, 112, 127, 128, 129, 239, 240, 400],
        );
    }

    fn mgf1_case(libs: &Libs, name: &'static str, seed: u64, outlens: &[usize], inlens: &[usize]) {
        let c: libloading::os::unix::Symbol<Mgf1> = libs.c_backend(name);
        let r: libloading::os::unix::Symbol<Mgf1> = libs.r(name);
        let mut rng = Rng::new(seed);
        for &ol in outlens {
            for &il in inlens {
                for i in 0..20 {
                    let inp: Vec<u8> = match i {
                        0 => vec![0u8; il],
                        1 => vec![0xFFu8; il],
                        _ => rng.bytes(il),
                    };
                    let mut co = vec![0xAAu8; ol + 8];
                    let mut ro = vec![0xAAu8; ol + 8];
                    unsafe {
                        c(co.as_mut_ptr(), ol as c_ulong, inp.as_ptr(), il as c_ulong);
                        r(ro.as_mut_ptr(), ol as c_ulong, inp.as_ptr(), il as c_ulong);
                    }
                    eq_bytes(&format!("{name}(outlen={ol}, inlen={il})"), &co, &ro);
                }
            }
        }
    }

    #[test]
    fn b14_mgf1_256() {
        let libs = Libs::load();
        mgf1_case(
            &libs,
            "SPX_mgf1_256",
            205,
            &[1, 2, 31, 32, 33, 63, 64, 65, 100, 200],
            &[0, 1, 4, 32, 54, 64, 100],
        );
    }

    #[test]
    fn b15_mgf1_512() {
        let libs = Libs::load();
        mgf1_case(
            &libs,
            "SPX_mgf1_512",
            206,
            &[1, 2, 63, 64, 65, 127, 128, 129, 200],
            &[0, 1, 4, 64, 111, 128, 200],
        );
    }

    #[test]
    fn b16_seed_state() {
        let libs = Libs::load();
        let (c, r) = pair_backend!(libs, "SPX_seed_state", SeedState);
        let mut rng = Rng::new(207);
        for i in 0..200 {
            let seed: Vec<u8> = match i {
                0 => vec![0u8; SPX_N],
                1 => vec![0xFFu8; SPX_N],
                _ => rng.bytes(SPX_N),
            };
            let sk = rng.bytes(SPX_N);
            let mut cc = Ctx::new();
            let mut rc = Ctx::new();
            for ctx in [&mut cc, &mut rc] {
                ctx.set_pub_seed(&seed);
                ctx.set_sk_seed(&sk);
            }
            unsafe {
                c(cc.as_mut_ptr());
                r(rc.as_mut_ptr());
            }
            eq_bytes("seed_state ctx", cc.bytes(), rc.bytes());
            // state_seeded is 40 bytes at offset 2*SPX_N; for N >= 24 the
            // 72-byte sha512 state follows.
            assert_eq!(CTX_BYTES, 2 * SPX_N + 40 + if N_GE_24 { 72 } else { 0 });
        }
    }
}
