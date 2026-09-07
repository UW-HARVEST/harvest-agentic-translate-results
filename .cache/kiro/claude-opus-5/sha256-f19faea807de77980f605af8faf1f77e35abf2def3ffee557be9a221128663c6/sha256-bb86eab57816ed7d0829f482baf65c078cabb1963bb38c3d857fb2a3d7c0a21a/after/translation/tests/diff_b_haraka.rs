//! CONFIGS.md group B, haraka backend (rows B20–B25).

mod common;

#[cfg(backend_haraka)]
mod haraka {
    use crate::common::*;
    use crate::{pair_backend};

    const RATE: usize = 32;

    type Tweak = unsafe extern "C" fn(*mut u8);
    type Perm = unsafe extern "C" fn(*mut u8, *const u8, *const u8);
    type SInit = unsafe extern "C" fn(*mut u8);
    type SAbsorb = unsafe extern "C" fn(*mut u8, *const u8, usize, *const u8);
    type SFinalize = unsafe extern "C" fn(*mut u8);
    type SSqueeze = unsafe extern "C" fn(*mut u8, usize, *mut u8, *const u8);
    type SOneShot = unsafe extern "C" fn(*mut u8, u64, *const u8, u64, *const u8);

    #[test]
    fn b20_tweak_constants() {
        let libs = Libs::load();
        let (c, r) = pair_backend!(libs, "SPX_tweak_constants", Tweak);
        let mut rng = Rng::new(401);
        for i in 0..200 {
            let ps: Vec<u8> = match i {
                0 => vec![0u8; SPX_N],
                1 => vec![0xFFu8; SPX_N],
                _ => rng.bytes(SPX_N),
            };
            let sk = rng.bytes(SPX_N);
            let mut cc = Ctx::new();
            let mut rc = Ctx::new();
            for ctx in [&mut cc, &mut rc] {
                ctx.set_pub_seed(&ps);
                ctx.set_sk_seed(&sk);
            }
            unsafe {
                c(cc.as_mut_ptr());
                r(rc.as_mut_ptr());
            }
            eq_bytes("tweak_constants ctx", cc.bytes(), rc.bytes());
            assert_eq!(CTX_BYTES, 2 * SPX_N + 960);
        }
    }

    fn perm_case(libs: &Libs, name: &'static str, inlen: usize, outlen: usize, seed: u64) {
        let c: libloading::os::unix::Symbol<Perm> = libs.c_backend(name);
        let r: libloading::os::unix::Symbol<Perm> = libs.r(name);
        let mut rng = Rng::new(seed);
        for i in 0..200 {
            let ps = rng.bytes(SPX_N);
            let sk = rng.bytes(SPX_N);
            let (cc, rc) = init_ctx_pair(libs, &ps, &sk);
            let inp: Vec<u8> = match i {
                0 => vec![0u8; inlen],
                1 => vec![0xFFu8; inlen],
                _ => rng.bytes(inlen),
            };
            let mut co = vec![0xAAu8; outlen + 8];
            let mut ro = vec![0xAAu8; outlen + 8];
            unsafe {
                c(co.as_mut_ptr(), inp.as_ptr(), cc.as_ptr());
                r(ro.as_mut_ptr(), inp.as_ptr(), rc.as_ptr());
            }
            eq_bytes(name, &co, &ro);
        }
    }

    #[test]
    fn b21_haraka256() {
        let libs = Libs::load();
        perm_case(&libs, "SPX_haraka256", 32, 32, 402);
    }

    #[test]
    fn b22_haraka512() {
        let libs = Libs::load();
        perm_case(&libs, "SPX_haraka512", 64, 32, 403);
    }

    #[test]
    fn b23_haraka512_perm() {
        let libs = Libs::load();
        perm_case(&libs, "SPX_haraka512_perm", 64, 64, 404);
    }

    #[test]
    fn b24_haraka_s() {
        let libs = Libs::load();
        let (c, r) = pair_backend!(libs, "SPX_haraka_S", SOneShot);
        let mut rng = Rng::new(405);
        let ps = rng.bytes(SPX_N);
        let sk = rng.bytes(SPX_N);
        let (cc, rc) = init_ctx_pair(&libs, &ps, &sk);

        for &ol in &[1usize, 2, 16, 31, 32, 33, 63, 64, 65, 100, 200] {
            for &il in &[0usize, 1, 31, 32, 33, 63, 64, 65, 200, 1000] {
                for i in 0..8 {
                    let inp: Vec<u8> = match i {
                        0 => vec![0u8; il.max(1)],
                        1 => vec![0xFFu8; il.max(1)],
                        _ => rng.bytes(il.max(1)),
                    };
                    let mut co = vec![0xAAu8; ol + 8];
                    let mut ro = vec![0xAAu8; ol + 8];
                    unsafe {
                        c(co.as_mut_ptr(), ol as u64, inp.as_ptr(), il as u64, cc.as_ptr());
                        r(ro.as_mut_ptr(), ol as u64, inp.as_ptr(), il as u64, rc.as_ptr());
                    }
                    eq_bytes(&format!("haraka_S(outlen={ol}, inlen={il})"), &co, &ro);
                }
            }
        }
    }

    #[test]
    fn b25_haraka_s_incremental() {
        let libs = Libs::load();
        let (ci, ri) = pair_backend!(libs, "SPX_haraka_S_inc_init", SInit);
        let (ca, ra) = pair_backend!(libs, "SPX_haraka_S_inc_absorb", SAbsorb);
        let (cf, rf) = pair_backend!(libs, "SPX_haraka_S_inc_finalize", SFinalize);
        let (cq, rq) = pair_backend!(libs, "SPX_haraka_S_inc_squeeze", SSqueeze);
        let mut rng = Rng::new(406);
        let ps = rng.bytes(SPX_N);
        let sk = rng.bytes(SPX_N);
        let (cc, rc) = init_ctx_pair(&libs, &ps, &sk);

        for trial in 0..250 {
            let mut c_s = [0u8; 65];
            let mut r_s = [0u8; 65];
            unsafe {
                ci(c_s.as_mut_ptr());
                ri(r_s.as_mut_ptr());
            }
            eq_bytes("haraka_S_inc_init state", &c_s, &r_s);

            for k in 0..(1 + trial % 5) {
                let len = match (trial + k) % 8 {
                    0 => 0,
                    1 => 1,
                    2 => RATE - 1,
                    3 => RATE,
                    4 => RATE + 1,
                    5 => 2 * RATE,
                    6 => 200,
                    _ => rng.below(100) as usize,
                };
                let chunk = rng.bytes(len.max(1));
                unsafe {
                    ca(c_s.as_mut_ptr(), chunk.as_ptr(), len, cc.as_ptr());
                    ra(r_s.as_mut_ptr(), chunk.as_ptr(), len, rc.as_ptr());
                }
                eq_bytes(&format!("haraka_S_inc_absorb({len}) state"), &c_s, &r_s);
            }
            unsafe {
                cf(c_s.as_mut_ptr());
                rf(r_s.as_mut_ptr());
            }
            eq_bytes("haraka_S_inc_finalize state", &c_s, &r_s);

            for k in 0..4 {
                let ol = match (trial + k) % 6 {
                    0 => 1,
                    1 => 16,
                    2 => RATE - 1,
                    3 => RATE,
                    4 => RATE + 1,
                    _ => 100,
                };
                let mut co = vec![0xAAu8; ol + 8];
                let mut ro = vec![0xAAu8; ol + 8];
                unsafe {
                    cq(co.as_mut_ptr(), ol, c_s.as_mut_ptr(), cc.as_ptr());
                    rq(ro.as_mut_ptr(), ol, r_s.as_mut_ptr(), rc.as_ptr());
                }
                eq_bytes(&format!("haraka_S_inc_squeeze({ol})"), &co, &ro);
                eq_bytes(&format!("haraka_S_inc_squeeze({ol}) state"), &c_s, &r_s);
            }
        }
    }
}
