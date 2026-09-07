//! CONFIGS.md group B, shake backend (rows B17–B19).

mod common;

#[cfg(backend_shake)]
mod shake {
    use crate::common::*;
    use crate::{pair_backend};

    const RATE: usize = 136;

    type Shake = unsafe extern "C" fn(*mut u8, usize, *const u8, usize);
    type IncInit = unsafe extern "C" fn(*mut u64);
    type IncAbsorb = unsafe extern "C" fn(*mut u64, *const u8, usize);
    type IncFinalize = unsafe extern "C" fn(*mut u64);
    type IncSqueeze = unsafe extern "C" fn(*mut u8, usize, *mut u64);
    type Absorb = unsafe extern "C" fn(*mut u64, *const u8, usize);
    type SqueezeBlocks = unsafe extern "C" fn(*mut u8, usize, *mut u64);

    #[test]
    fn b17_shake256_oneshot() {
        let libs = Libs::load();
        let (c, r) = pair_backend!(libs, "shake256", Shake);
        let mut rng = Rng::new(301);
        let outlens = [1usize, 2, 32, 135, 136, 137, 271, 272, 273, 500];
        let inlens = [0usize, 1, 2, 135, 136, 137, 271, 272, 1000];
        for &ol in &outlens {
            for &il in &inlens {
                for i in 0..12 {
                    let inp: Vec<u8> = match i {
                        0 => vec![0u8; il],
                        1 => vec![0xFFu8; il],
                        _ => rng.bytes(il),
                    };
                    let mut co = vec![0xAAu8; ol + 8];
                    let mut ro = vec![0xAAu8; ol + 8];
                    unsafe {
                        c(co.as_mut_ptr(), ol, inp.as_ptr(), il);
                        r(ro.as_mut_ptr(), ol, inp.as_ptr(), il);
                    }
                    eq_bytes(&format!("shake256(outlen={ol}, inlen={il})"), &co, &ro);
                }
            }
        }
    }

    #[test]
    fn b18_shake256_incremental() {
        let libs = Libs::load();
        let (ci, ri) = pair_backend!(libs, "shake256_inc_init", IncInit);
        let (ca, ra) = pair_backend!(libs, "shake256_inc_absorb", IncAbsorb);
        let (cf, rf) = pair_backend!(libs, "shake256_inc_finalize", IncFinalize);
        let (cs, rs) = pair_backend!(libs, "shake256_inc_squeeze", IncSqueeze);
        let mut rng = Rng::new(302);

        for trial in 0..250 {
            let mut c_state = [0u64; 26];
            let mut r_state = [0u64; 26];
            unsafe {
                ci(c_state.as_mut_ptr());
                ri(r_state.as_mut_ptr());
            }
            eq(&"shake256_inc_init state", c_state, r_state);

            let nchunks = 1 + (trial % 5);
            for k in 0..nchunks {
                let len = match (trial + k) % 8 {
                    0 => 0,
                    1 => 1,
                    2 => 135,
                    3 => 136,
                    4 => 137,
                    5 => 272,
                    6 => 1000,
                    _ => rng.below(300) as usize,
                };
                let chunk = rng.bytes(len.max(1));
                unsafe {
                    ca(c_state.as_mut_ptr(), chunk.as_ptr(), len);
                    ra(r_state.as_mut_ptr(), chunk.as_ptr(), len);
                }
                eq(&format!("shake256_inc_absorb({len}) state"), c_state, r_state);
            }
            unsafe {
                cf(c_state.as_mut_ptr());
                rf(r_state.as_mut_ptr());
            }
            eq(&"shake256_inc_finalize state", c_state, r_state);

            // squeeze in several chunks straddling the rate
            for k in 0..4 {
                let ol = match (trial + k) % 6 {
                    0 => 1,
                    1 => 32,
                    2 => 135,
                    3 => 136,
                    4 => 137,
                    _ => 300,
                };
                let mut co = vec![0xAAu8; ol + 8];
                let mut ro = vec![0xAAu8; ol + 8];
                unsafe {
                    cs(co.as_mut_ptr(), ol, c_state.as_mut_ptr());
                    rs(ro.as_mut_ptr(), ol, r_state.as_mut_ptr());
                }
                eq_bytes(&format!("shake256_inc_squeeze({ol})"), &co, &ro);
                eq(&format!("shake256_inc_squeeze({ol}) state"), c_state, r_state);
            }
        }
    }

    #[test]
    fn b19_shake256_absorb_squeezeblocks() {
        let libs = Libs::load();
        let (ca, ra) = pair_backend!(libs, "shake256_absorb", Absorb);
        let (cq, rq) = pair_backend!(libs, "shake256_squeezeblocks", SqueezeBlocks);
        let mut rng = Rng::new(303);

        for &il in &[0usize, 1, RATE - 1, RATE, RATE + 1, 2 * RATE, 3 * RATE + 7, 1000] {
            for &nblocks in &[1usize, 2, 4] {
                for _ in 0..12 {
                    let inp = rng.bytes(il.max(1));
                    // shake256_absorb zeroes the state itself, so an
                    // uninitialised (here: zeroed) 25-word buffer is correct.
                    let mut c_state = [0u64; 25];
                    let mut r_state = [0u64; 25];
                    unsafe {
                        ca(c_state.as_mut_ptr(), inp.as_ptr(), il);
                        ra(r_state.as_mut_ptr(), inp.as_ptr(), il);
                    }
                    eq(&format!("shake256_absorb({il}) state"), c_state, r_state);

                    let mut co = vec![0xAAu8; nblocks * RATE + 8];
                    let mut ro = vec![0xAAu8; nblocks * RATE + 8];
                    unsafe {
                        cq(co.as_mut_ptr(), nblocks, c_state.as_mut_ptr());
                        rq(ro.as_mut_ptr(), nblocks, r_state.as_mut_ptr());
                    }
                    eq_bytes(&format!("shake256_squeezeblocks({nblocks})"), &co, &ro);
                    eq(
                        &format!("shake256_squeezeblocks({nblocks}) state"),
                        c_state,
                        r_state,
                    );
                }
            }
        }
    }
}
