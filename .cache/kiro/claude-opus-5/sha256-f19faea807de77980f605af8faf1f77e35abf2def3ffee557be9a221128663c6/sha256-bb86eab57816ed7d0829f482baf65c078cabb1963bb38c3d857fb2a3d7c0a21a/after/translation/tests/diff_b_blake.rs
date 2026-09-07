//! CONFIGS.md group B, blake backend (rows B1–B9).

mod common;

#[cfg(backend_blake)]
mod blake {
    use crate::common::*;
    use crate::{pair_backend};
    use std::os::raw::{c_int, c_ulong};

    /// `blakestate256` from `lib/blake/include/blake.h`.
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct State256 {
        h: [u32; 8],
        s: [u32; 4],
        t: [u32; 2],
        buflen: i32,
        nullt: i32,
        buf: [u8; 64],
    }
    impl State256 {
        fn zeroed() -> Self {
            unsafe { std::mem::zeroed() }
        }
        fn bytes(&self) -> &[u8] {
            unsafe {
                std::slice::from_raw_parts(self as *const _ as *const u8, std::mem::size_of::<Self>())
            }
        }
    }

    /// `blakestate512` from `lib/blake/include/blake.h`.
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct State512 {
        h: [u64; 8],
        s: [u64; 4],
        t: [u64; 2],
        buflen: i32,
        nullt: i32,
        buf: [u8; 128],
    }
    impl State512 {
        fn zeroed() -> Self {
            unsafe { std::mem::zeroed() }
        }
        fn bytes(&self) -> &[u8] {
            unsafe {
                std::slice::from_raw_parts(self as *const _ as *const u8, std::mem::size_of::<Self>())
            }
        }
    }

    type OneShot = unsafe extern "C" fn(*mut u8, *const u8, u64) -> c_int;
    type Init256 = unsafe extern "C" fn(*mut State256);
    type Upd256 = unsafe extern "C" fn(*mut State256, *const u8, u64);
    type Fin256 = unsafe extern "C" fn(*mut State256, *mut u8);
    type Cmp256 = unsafe extern "C" fn(*mut State256, *const u8);
    type Init512 = unsafe extern "C" fn(*mut State512);
    type Upd512 = unsafe extern "C" fn(*mut State512, *const u8, u64);
    type Fin512 = unsafe extern "C" fn(*mut State512, *mut u8);
    type Cmp512 = unsafe extern "C" fn(*mut State512, *const u8);
    type Mgf1 = unsafe extern "C" fn(*mut u8, c_ulong, *const u8, c_ulong);

    const LENS_256: &[usize] = &[
        0, 1, 2, 31, 32, 33, 54, 55, 56, 57, 63, 64, 65, 111, 127, 128, 129, 255, 256, 1000,
    ];
    const LENS_512: &[usize] = &[
        0, 1, 2, 63, 64, 65, 110, 111, 112, 113, 127, 128, 129, 239, 255, 256, 257, 1000,
    ];

    #[test]
    fn b1_blake256_oneshot() {
        let libs = Libs::load();
        let (c, r) = pair_backend!(libs, "blake256", OneShot);
        let mut rng = Rng::new(101);
        for &n in LENS_256 {
            for i in 0..40 {
                let inp: Vec<u8> = match i {
                    0 => vec![0u8; n],
                    1 => vec![0xFFu8; n],
                    _ => rng.bytes(n),
                };
                let mut co = [0u8; 32];
                let mut ro = [0u8; 32];
                let (cr, rr) = unsafe {
                    (
                        c(co.as_mut_ptr(), inp.as_ptr(), n as u64),
                        r(ro.as_mut_ptr(), inp.as_ptr(), n as u64),
                    )
                };
                eq(&format!("blake256 ret (inlen={n})"), cr, rr);
                eq_bytes(&format!("blake256(inlen={n})"), &co, &ro);
            }
        }
    }

    #[test]
    fn b2_blake256_incremental() {
        let libs = Libs::load();
        let (ci, ri) = pair_backend!(libs, "blake256_init", Init256);
        let (cu, ru) = pair_backend!(libs, "blake256_update", Upd256);
        let (cf, rf) = pair_backend!(libs, "blake256_final", Fin256);
        let mut rng = Rng::new(102);

        for trial in 0..300 {
            // 1..=5 byte-aligned chunks, deliberately straddling the 64-byte block
            let nchunks = 1 + (trial % 5);
            let chunks: Vec<Vec<u8>> = (0..nchunks)
                .map(|k| {
                    let len = match (trial + k) % 7 {
                        0 => 0,
                        1 => 1,
                        2 => 55,
                        3 => 63,
                        4 => 64,
                        5 => 65,
                        _ => rng.below(200) as usize,
                    };
                    let mut v = rng.bytes(len);
                    v.push(0); // the Rust wrapper reads one guard byte past inlen>>3
                    v
                })
                .collect();

            let mut cs = State256::zeroed();
            let mut rs = State256::zeroed();
            unsafe {
                ci(&mut cs);
                ri(&mut rs);
            }
            eq_bytes("blake256_init state", cs.bytes(), rs.bytes());
            for ch in &chunks {
                let bits = ((ch.len() - 1) * 8) as u64;
                unsafe {
                    cu(&mut cs, ch.as_ptr(), bits);
                    ru(&mut rs, ch.as_ptr(), bits);
                }
                eq_bytes(&format!("blake256_update state (bits={bits})"), cs.bytes(), rs.bytes());
            }
            let mut co = [0u8; 32];
            let mut ro = [0u8; 32];
            unsafe {
                cf(&mut cs, co.as_mut_ptr());
                rf(&mut rs, ro.as_mut_ptr());
            }
            eq_bytes("blake256_final digest", &co, &ro);
            eq_bytes("blake256_final state", cs.bytes(), rs.bytes());
        }
    }

    #[test]
    fn b3_blake256_compress() {
        let libs = Libs::load();
        let (ci, ri) = pair_backend!(libs, "blake256_init", Init256);
        let (cc, rc) = pair_backend!(libs, "blake256_compress", Cmp256);
        let mut rng = Rng::new(103);
        for _ in 0..300 {
            let mut cs = State256::zeroed();
            let mut rs = State256::zeroed();
            unsafe {
                ci(&mut cs);
                ri(&mut rs);
            }
            // Perturb the salt/counter so the test is not limited to the IV.
            for k in 0..4 {
                cs.s[k] = rng.next_u32();
                rs.s[k] = cs.s[k];
            }
            cs.t[0] = rng.next_u32();
            cs.t[1] = rng.next_u32();
            rs.t = cs.t;
            cs.nullt = (rng.next_u32() & 1) as i32;
            rs.nullt = cs.nullt;

            let block = rng.bytes(64);
            unsafe {
                cc(&mut cs, block.as_ptr());
                rc(&mut rs, block.as_ptr());
            }
            eq_bytes("blake256_compress state", cs.bytes(), rs.bytes());
        }
    }

    #[test]
    fn b4_blake512_oneshot() {
        let libs = Libs::load();
        let (c, r) = pair_backend!(libs, "blake512", OneShot);
        let mut rng = Rng::new(104);
        for &n in LENS_512 {
            for i in 0..40 {
                let inp: Vec<u8> = match i {
                    0 => vec![0u8; n],
                    1 => vec![0xFFu8; n],
                    _ => rng.bytes(n),
                };
                let mut co = [0u8; 64];
                let mut ro = [0u8; 64];
                let (cr, rr) = unsafe {
                    (
                        c(co.as_mut_ptr(), inp.as_ptr(), n as u64),
                        r(ro.as_mut_ptr(), inp.as_ptr(), n as u64),
                    )
                };
                eq(&format!("blake512 ret (inlen={n})"), cr, rr);
                eq_bytes(&format!("blake512(inlen={n})"), &co, &ro);
            }
        }
    }

    #[test]
    fn b5_blake512_incremental() {
        let libs = Libs::load();
        let (ci, ri) = pair_backend!(libs, "blake512_init", Init512);
        let (cu, ru) = pair_backend!(libs, "blake512_update", Upd512);
        let (cf, rf) = pair_backend!(libs, "blake512_final", Fin512);
        let mut rng = Rng::new(105);

        for trial in 0..300 {
            let nchunks = 1 + (trial % 5);
            let chunks: Vec<Vec<u8>> = (0..nchunks)
                .map(|k| {
                    let len = match (trial + k) % 7 {
                        0 => 0,
                        1 => 1,
                        2 => 111,
                        3 => 127,
                        4 => 128,
                        5 => 129,
                        _ => rng.below(400) as usize,
                    };
                    let mut v = rng.bytes(len);
                    v.push(0);
                    v
                })
                .collect();

            let mut cs = State512::zeroed();
            let mut rs = State512::zeroed();
            unsafe {
                ci(&mut cs);
                ri(&mut rs);
            }
            eq_bytes("blake512_init state", cs.bytes(), rs.bytes());
            for ch in &chunks {
                let bits = ((ch.len() - 1) * 8) as u64;
                unsafe {
                    cu(&mut cs, ch.as_ptr(), bits);
                    ru(&mut rs, ch.as_ptr(), bits);
                }
                eq_bytes(&format!("blake512_update state (bits={bits})"), cs.bytes(), rs.bytes());
            }
            let mut co = [0u8; 64];
            let mut ro = [0u8; 64];
            unsafe {
                cf(&mut cs, co.as_mut_ptr());
                rf(&mut rs, ro.as_mut_ptr());
            }
            eq_bytes("blake512_final digest", &co, &ro);
            eq_bytes("blake512_final state", cs.bytes(), rs.bytes());
        }
    }

    #[test]
    fn b6_blake512_compress() {
        let libs = Libs::load();
        let (ci, ri) = pair_backend!(libs, "blake512_init", Init512);
        let (cc, rc) = pair_backend!(libs, "blake512_compress", Cmp512);
        let mut rng = Rng::new(106);
        for _ in 0..300 {
            let mut cs = State512::zeroed();
            let mut rs = State512::zeroed();
            unsafe {
                ci(&mut cs);
                ri(&mut rs);
            }
            for k in 0..4 {
                cs.s[k] = rng.next_u64();
                rs.s[k] = cs.s[k];
            }
            cs.t[0] = rng.next_u64();
            cs.t[1] = rng.next_u64();
            rs.t = cs.t;
            cs.nullt = (rng.next_u32() & 1) as i32;
            rs.nullt = cs.nullt;

            let block = rng.bytes(128);
            unsafe {
                cc(&mut cs, block.as_ptr());
                rc(&mut rs, block.as_ptr());
            }
            eq_bytes("blake512_compress state", cs.bytes(), rs.bytes());
        }
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
    fn b7_blake256_mgf1() {
        let libs = Libs::load();
        mgf1_case(
            &libs,
            "SPX_blake256_mgf1",
            107,
            &[1, 2, 31, 32, 33, 63, 64, 65, 100, 200],
            &[0, 1, 4, 32, 48, 64, 100],
        );
    }

    #[test]
    fn b8_blake512_mgf1() {
        let libs = Libs::load();
        mgf1_case(
            &libs,
            "SPX_blake512_mgf1",
            108,
            &[1, 2, 63, 64, 65, 127, 128, 129, 200],
            &[0, 1, 4, 64, 96, 128, 200],
        );
    }

    #[test]
    fn b9_cst_data_symbol() {
        let libs = Libs::load();
        // `const u64 cst[16]` in blake512.c has external linkage.
        let c: libloading::os::unix::Symbol<*const [u64; 16]> = libs.c_backend("cst");
        let r: libloading::os::unix::Symbol<*const [u64; 16]> = libs.r("cst");
        let cv = unsafe { *(c.into_raw() as *const [u64; 16]) };
        let rv = unsafe { *(r.into_raw() as *const [u64; 16]) };
        eq("cst[16]", cv, rv);
    }
}
