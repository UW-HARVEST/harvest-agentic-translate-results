//! Phase B rows 43-46: the deterministic AES-256-CTR-DRBG of `app/src/rng.c`
//! (`randombytes_init`, `randombytes`, `AES256_ECB`, `AES256_CTR_DRBG_Update`,
//! `seedexpander_init`, `seedexpander`) plus the exported `DRBG_ctx` global.
mod common;
use common::*;

type FRbInit = unsafe extern "C" fn(*mut u8, *mut u8);
type FRb = unsafe extern "C" fn(*mut u8, u64) -> i32;
type FEcb = unsafe extern "C" fn(*mut u8, *mut u8, *mut u8);
type FUpd = unsafe extern "C" fn(*mut u8, *mut u8, *mut u8);
type FSeInit = unsafe extern "C" fn(*mut u8, *mut u8, *mut u8, u64) -> i32;
type FSe = unsafe extern "C" fn(*mut u8, *mut u8, u64) -> i32;

/// `AES_XOF_struct`: `{ u8 buffer[16]; unsigned long buffer_pos;
/// unsigned long length_remaining; u8 key[32]; u8 ctr[16]; }`
pub const XOF_BYTES: usize = 80;

#[test]
fn b43_drbg_sequence() {
    let _guard = drbg_lock();
    let p = pair();
    let mut rng = Rng::new(SEED ^ 43);
    unsafe {
        let ci = sym!(p.c, b"randombytes_init\0", FRbInit);
        let ri = sym!(p.r, b"randombytes_init\0", FRbInit);
        let cr = sym!(p.c, b"randombytes\0", FRb);
        let rr = sym!(p.r, b"randombytes\0", FRb);

        for trial in 0..8 {
            let mut e1 = rng.bytes(48);
            let mut e2 = e1.clone();
            if trial % 3 == 0 {
                ci(e1.as_mut_ptr(), core::ptr::null_mut());
                ri(e2.as_mut_ptr(), core::ptr::null_mut());
            } else {
                let mut p1 = rng.bytes(48);
                let mut p2 = p1.clone();
                ci(e1.as_mut_ptr(), p1.as_mut_ptr());
                ri(e2.as_mut_ptr(), p2.as_mut_ptr());
            }
            eqb("DRBG_ctx after init", &drbg_image(&p.c), &drbg_image(&p.r));

            for xlen in [0usize, 1, 2, 15, 16, 17, 31, 32, 33, 47, 48, 49, 100, 257] {
                let mut cb = obuf(xlen);
                let mut rb = obuf(xlen);
                let a = cr(cb.as_mut_ptr(), xlen as u64);
                let b = rr(rb.as_mut_ptr(), xlen as u64);
                eqv(&format!("randombytes ret xlen={xlen}"), a, b);
                eqv(&format!("randombytes ret is RNG_SUCCESS xlen={xlen}"), a, 0);
                eqb(&format!("randombytes out xlen={xlen}"), &cb, &rb);
                eqb(
                    &format!("DRBG_ctx after randombytes xlen={xlen}"),
                    &drbg_image(&p.c),
                    &drbg_image(&p.r),
                );
            }
        }
    }
}

#[test]
fn b44_aes256_ecb() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 44);
    unsafe {
        let cf = sym!(p.c, b"AES256_ECB\0", FEcb);
        let rf = sym!(p.r, b"AES256_ECB\0", FEcb);

        // FIPS-197 C.3 known-answer vector plus randomised inputs.
        let mut fixed_key: Vec<u8> = (0u8..32).collect();
        let mut fixed_ctr: Vec<u8> = vec![
            0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd,
            0xee, 0xff,
        ];
        let mut co = obuf(16);
        let mut ro = obuf(16);
        cf(fixed_key.as_mut_ptr(), fixed_ctr.as_mut_ptr(), co.as_mut_ptr());
        rf(fixed_key.as_mut_ptr(), fixed_ctr.as_mut_ptr(), ro.as_mut_ptr());
        eqb("AES256_ECB FIPS-197 vector", &co, &ro);
        eqv(
            "AES256_ECB FIPS-197 expected ciphertext",
            &co[..16],
            &[
                0x8eu8, 0xa2, 0xb7, 0xca, 0x51, 0x67, 0x45, 0xbf, 0xea, 0xfc, 0x49, 0x90, 0x4b,
                0x49, 0x60, 0x89,
            ][..],
        );

        for _ in 0..256 {
            let mut k1 = rng.bytes(32);
            let mut k2 = k1.clone();
            let mut c1 = rng.bytes(16);
            let mut c2 = c1.clone();
            let mut co = obuf(16);
            let mut ro = obuf(16);
            cf(k1.as_mut_ptr(), c1.as_mut_ptr(), co.as_mut_ptr());
            rf(k2.as_mut_ptr(), c2.as_mut_ptr(), ro.as_mut_ptr());
            eqb("AES256_ECB out", &co, &ro);
            eqb("AES256_ECB key untouched", &k1, &k2);
            eqb("AES256_ECB ctr untouched", &c1, &c2);
        }
        // extremes
        for (k, c) in [(0x00u8, 0x00u8), (0xff, 0xff), (0x00, 0xff), (0xff, 0x00)] {
            let mut k1 = vec![k; 32];
            let mut k2 = k1.clone();
            let mut c1 = vec![c; 16];
            let mut c2 = c1.clone();
            let mut co = obuf(16);
            let mut ro = obuf(16);
            cf(k1.as_mut_ptr(), c1.as_mut_ptr(), co.as_mut_ptr());
            rf(k2.as_mut_ptr(), c2.as_mut_ptr(), ro.as_mut_ptr());
            eqb("AES256_ECB extremes", &co, &ro);
        }
    }
}

#[test]
fn b45_drbg_update() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 45);
    unsafe {
        let cf = sym!(p.c, b"AES256_CTR_DRBG_Update\0", FUpd);
        let rf = sym!(p.r, b"AES256_CTR_DRBG_Update\0", FUpd);
        for trial in 0..128 {
            let k = rng.bytes(32);
            let v = rng.bytes(16);
            let mut ck = k.clone();
            let mut rk = k.clone();
            let mut cv = v.clone();
            let mut rv = v.clone();
            if trial % 2 == 0 {
                cf(core::ptr::null_mut(), ck.as_mut_ptr(), cv.as_mut_ptr());
                rf(core::ptr::null_mut(), rk.as_mut_ptr(), rv.as_mut_ptr());
            } else {
                let mut pd1 = rng.bytes(48);
                let mut pd2 = pd1.clone();
                cf(pd1.as_mut_ptr(), ck.as_mut_ptr(), cv.as_mut_ptr());
                rf(pd2.as_mut_ptr(), rk.as_mut_ptr(), rv.as_mut_ptr());
                eqb("DRBG_Update provided_data untouched", &pd1, &pd2);
            }
            eqb("DRBG_Update Key", &ck, &rk);
            eqb("DRBG_Update V", &cv, &rv);
        }
        // V all-0xff exercises the carry chain of the 128-bit increment
        for vpat in [0x00u8, 0xff] {
            let mut ck = vec![0x5au8; 32];
            let mut rk = ck.clone();
            let mut cv = vec![vpat; 16];
            let mut rv = cv.clone();
            cf(core::ptr::null_mut(), ck.as_mut_ptr(), cv.as_mut_ptr());
            rf(core::ptr::null_mut(), rk.as_mut_ptr(), rv.as_mut_ptr());
            eqb("DRBG_Update carry Key", &ck, &rk);
            eqb("DRBG_Update carry V", &cv, &rv);
        }
    }
}

#[test]
fn b46_seedexpander_sequence() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 46);
    unsafe {
        let cinit = sym!(p.c, b"seedexpander_init\0", FSeInit);
        let rinit = sym!(p.r, b"seedexpander_init\0", FSeInit);
        let cse = sym!(p.c, b"seedexpander\0", FSe);
        let rse = sym!(p.r, b"seedexpander\0", FSe);

        for maxlen in [1u64, 16, 17, 256, 4096, 65536, 0xFFFF_FFFF] {
            let mut seed = rng.bytes(32);
            let mut div = rng.bytes(8);
            let mut cctx = vec![0xC3u8; XOF_BYTES];
            let mut rctx = vec![0xC3u8; XOF_BYTES];
            let a = cinit(cctx.as_mut_ptr(), seed.as_mut_ptr(), div.as_mut_ptr(), maxlen);
            let b = rinit(rctx.as_mut_ptr(), seed.as_mut_ptr(), div.as_mut_ptr(), maxlen);
            eqv(&format!("seedexpander_init ret maxlen={maxlen}"), a, b);
            eqb(&format!("AES_XOF_struct after init maxlen={maxlen}"), &cctx, &rctx);

            for xlen in [1u64, 5, 15, 16, 17, 33, 100, 137] {
                if xlen >= maxlen {
                    continue;
                }
                let mut cb = obuf(xlen as usize);
                let mut rb = obuf(xlen as usize);
                let a = cse(cctx.as_mut_ptr(), cb.as_mut_ptr(), xlen);
                let b = rse(rctx.as_mut_ptr(), rb.as_mut_ptr(), xlen);
                eqv(&format!("seedexpander ret maxlen={maxlen} xlen={xlen}"), a, b);
                eqb(&format!("seedexpander out maxlen={maxlen} xlen={xlen}"), &cb, &rb);
                eqb(
                    &format!("AES_XOF_struct maxlen={maxlen} xlen={xlen}"),
                    &cctx,
                    &rctx,
                );
            }
        }

        // long randomised call sequences with a big budget
        for _ in 0..16 {
            let mut seed = rng.bytes(32);
            let mut div = rng.bytes(8);
            let mut cctx = vec![0u8; XOF_BYTES];
            let mut rctx = vec![0u8; XOF_BYTES];
            cinit(cctx.as_mut_ptr(), seed.as_mut_ptr(), div.as_mut_ptr(), 100_000);
            rinit(rctx.as_mut_ptr(), seed.as_mut_ptr(), div.as_mut_ptr(), 100_000);
            for _ in 0..40 {
                let xlen = 1 + rng.below(70) as u64;
                let mut cb = obuf(xlen as usize);
                let mut rb = obuf(xlen as usize);
                let a = cse(cctx.as_mut_ptr(), cb.as_mut_ptr(), xlen);
                let b = rse(rctx.as_mut_ptr(), rb.as_mut_ptr(), xlen);
                eqv("seedexpander seq ret", a, b);
                eqb("seedexpander seq out", &cb, &rb);
                eqb("seedexpander seq ctx", &cctx, &rctx);
            }
        }
    }
}
