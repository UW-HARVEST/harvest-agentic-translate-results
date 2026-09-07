//! CONFIGS.md rows 32-35: `rng.c` (the NIST AES-256-CTR-DRBG).

mod common;
use common::*;

type FRbInit = unsafe extern "C" fn(*mut u8, *mut u8);
type FRb = unsafe extern "C" fn(*mut u8, u64) -> i32;
type FEcb = unsafe extern "C" fn(*mut u8, *mut u8, *mut u8);
type FUpdate = unsafe extern "C" fn(*mut u8, *mut u8, *mut u8);
type FSeInit = unsafe extern "C" fn(*mut CXof, *mut u8, *mut u8, u64) -> i32;
type FSe = unsafe extern "C" fn(*mut CXof, *mut u8, u64) -> i32;

/// `AES_XOF_struct` from `app/include/rng.h`
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CXof {
    pub buffer: [u8; 16],
    pub buffer_pos: u64,
    pub length_remaining: u64,
    pub key: [u8; 32],
    pub ctr: [u8; 16],
}

/// `AES256_CTR_DRBG_struct` -- `Key[32] || V[16] || int reseed_counter`
pub const DRBG_BYTES: usize = 32 + 16 + 4;

macro_rules! f {
    ($side:expr, $name:expr, $t:ty) => {
        unsafe { std::mem::transmute::<usize, $t>($side.addr($name)) }
    };
}

fn drbg_state(side: &Side) -> Vec<u8> {
    let p = side.data("DRBG_ctx");
    unsafe { std::slice::from_raw_parts(p, DRBG_BYTES).to_vec() }
}

/// row 32 -- `randombytes_init` (NULL and non-NULL personalization string)
/// followed by a sequence of `randombytes` calls of every interesting length.
#[test]
fn row32_randombytes() {
    let p = libs();
    let ci = f!(p.c, "randombytes_init", FRbInit);
    let ri = f!(p.rust, "randombytes_init", FRbInit);
    let cr = f!(p.c, "randombytes", FRb);
    let rr = f!(p.rust, "randombytes", FRb);
    let mut rng = Rng::new(SEED ^ 32);

    for with_ps in [false, true] {
        for round in 0..4 {
            let mut ent = [0u8; 48];
            match round {
                0 => (0..48).for_each(|i| ent[i] = i as u8),
                1 => ent = [0u8; 48],
                2 => ent = [0xFFu8; 48],
                _ => rng.fill(&mut ent),
            }
            let mut ps = [0u8; 48];
            rng.fill(&mut ps);

            let mut e1 = ent;
            let mut e2 = ent;
            let mut p1 = ps;
            let mut p2 = ps;
            unsafe {
                if with_ps {
                    ci(e1.as_mut_ptr(), p1.as_mut_ptr());
                    ri(e2.as_mut_ptr(), p2.as_mut_ptr());
                } else {
                    ci(e1.as_mut_ptr(), std::ptr::null_mut());
                    ri(e2.as_mut_ptr(), std::ptr::null_mut());
                }
            }
            eq_bytes(
                &format!("randombytes_init DRBG_ctx(with_ps={with_ps},round={round})"),
                &drbg_state(&p.c),
                &drbg_state(&p.rust),
            );

            for &xlen in &[0usize, 1, 15, 16, 17, 31, 32, 33, 48, 100] {
                let mut a = vec![0x5Au8; xlen + 8];
                let mut b = a.clone();
                let (r1, r2) = unsafe {
                    (
                        cr(a.as_mut_ptr(), xlen as u64),
                        rr(b.as_mut_ptr(), xlen as u64),
                    )
                };
                eq(&format!("randombytes ret(xlen={xlen})"), r1, r2);
                eq(&format!("randombytes ret==0(xlen={xlen})"), r1, 0);
                eq_bytes(&format!("randombytes out(xlen={xlen})"), &a, &b);
                eq_bytes(
                    &format!("randombytes DRBG_ctx(xlen={xlen})"),
                    &drbg_state(&p.c),
                    &drbg_state(&p.rust),
                );
            }
        }
    }
}

/// row 33 -- `AES256_ECB`
#[test]
fn row33_aes256_ecb() {
    let p = libs();
    let cf = f!(p.c, "AES256_ECB", FEcb);
    let rf = f!(p.rust, "AES256_ECB", FEcb);
    let mut rng = Rng::new(SEED ^ 33);

    for iter in 0..32 {
        let mut key = [0u8; 32];
        let mut ctr = [0u8; 16];
        match iter {
            0 => {}
            1 => {
                key = [0xFFu8; 32];
                ctr = [0xFFu8; 16];
            }
            2 => {
                // FIPS-197 C.3 AES-256 known answer
                (0..32).for_each(|i| key[i] = i as u8);
                ctr = [
                    0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc,
                    0xdd, 0xee, 0xff,
                ];
            }
            _ => {
                rng.fill(&mut key);
                rng.fill(&mut ctr);
            }
        }
        let mut k1 = key;
        let mut k2 = key;
        let mut c1 = ctr;
        let mut c2 = ctr;
        let mut o1 = [0xAAu8; 16];
        let mut o2 = [0xAAu8; 16];
        unsafe {
            cf(k1.as_mut_ptr(), c1.as_mut_ptr(), o1.as_mut_ptr());
            rf(k2.as_mut_ptr(), c2.as_mut_ptr(), o2.as_mut_ptr());
        }
        eq_bytes(&format!("AES256_ECB(iter={iter})"), &o1, &o2);
        eq_bytes(&format!("AES256_ECB key untouched({iter})"), &k1, &k2);
        eq_bytes(&format!("AES256_ECB ctr untouched({iter})"), &c1, &c2);
        if iter == 2 {
            let expect = [
                0x8e, 0xa2, 0xb7, 0xca, 0x51, 0x67, 0x45, 0xbf, 0xea, 0xfc, 0x49, 0x90, 0x4b, 0x49,
                0x60, 0x89,
            ];
            eq_bytes("AES256_ECB FIPS-197 KAT", &o1, &expect);
        }
    }
}

/// row 34 -- `AES256_CTR_DRBG_Update`, `provided_data` NULL and non-NULL
#[test]
fn row34_drbg_update() {
    let p = libs();
    let cf = f!(p.c, "AES256_CTR_DRBG_Update", FUpdate);
    let rf = f!(p.rust, "AES256_CTR_DRBG_Update", FUpdate);
    let mut rng = Rng::new(SEED ^ 34);

    for with_pd in [false, true] {
        for iter in 0..16 {
            let mut key = [0u8; 32];
            let mut v = [0u8; 16];
            let mut pd = [0u8; 48];
            match iter {
                0 => {}
                1 => {
                    key = [0xFFu8; 32];
                    v = [0xFFu8; 16];
                    pd = [0xFFu8; 48];
                }
                _ => {
                    rng.fill(&mut key);
                    rng.fill(&mut v);
                    rng.fill(&mut pd);
                }
            }
            let mut k1 = key;
            let mut k2 = key;
            let mut v1 = v;
            let mut v2 = v;
            let mut p1 = pd;
            let mut p2 = pd;
            unsafe {
                if with_pd {
                    cf(p1.as_mut_ptr(), k1.as_mut_ptr(), v1.as_mut_ptr());
                    rf(p2.as_mut_ptr(), k2.as_mut_ptr(), v2.as_mut_ptr());
                } else {
                    cf(std::ptr::null_mut(), k1.as_mut_ptr(), v1.as_mut_ptr());
                    rf(std::ptr::null_mut(), k2.as_mut_ptr(), v2.as_mut_ptr());
                }
            }
            eq_bytes(&format!("DRBG_Update Key(with_pd={with_pd})"), &k1, &k2);
            eq_bytes(&format!("DRBG_Update V(with_pd={with_pd})"), &v1, &v2);
            eq_bytes(&format!("DRBG_Update pd(with_pd={with_pd})"), &p1, &p2);
        }
    }
}

/// row 35 -- `seedexpander_init` + `seedexpander`, walking the 16-byte buffer
#[test]
fn row35_seedexpander() {
    let p = libs();
    let cinit = f!(p.c, "seedexpander_init", FSeInit);
    let rinit = f!(p.rust, "seedexpander_init", FSeInit);
    let cse = f!(p.c, "seedexpander", FSe);
    let rse = f!(p.rust, "seedexpander", FSe);
    let mut rng = Rng::new(SEED ^ 35);

    let zero = CXof {
        buffer: [0; 16],
        buffer_pos: 0,
        length_remaining: 0,
        key: [0; 32],
        ctr: [0; 16],
    };

    for &maxlen in &[16u64, 17, 100, 4096, 0xFFFF_FFFF] {
        let mut seed = [0u8; 32];
        let mut div = [0u8; 8];
        rng.fill(&mut seed);
        rng.fill(&mut div);

        let mut x1 = zero;
        let mut x2 = zero;
        let mut s1 = seed;
        let mut s2 = seed;
        let mut d1 = div;
        let mut d2 = div;
        let (i1, i2) = unsafe {
            (
                cinit(&mut x1, s1.as_mut_ptr(), d1.as_mut_ptr(), maxlen),
                rinit(&mut x2, s2.as_mut_ptr(), d2.as_mut_ptr(), maxlen),
            )
        };
        eq(&format!("seedexpander_init ret(maxlen={maxlen})"), i1, i2);
        eq(&format!("seedexpander_init ret==0(maxlen={maxlen})"), i1, 0);
        eq(&format!("seedexpander_init ctx(maxlen={maxlen})"), x1, x2);

        // Sequences that stay inside the buffer, exactly exhaust it and cross
        // it repeatedly.
        for &xlen in &[0u64, 1, 5, 10, 16, 17, 31, 32, 33] {
            if xlen >= x1.length_remaining {
                continue;
            }
            let mut a = vec![0x7Eu8; xlen as usize + 8];
            let mut b = a.clone();
            let (r1, r2) = unsafe {
                (
                    cse(&mut x1, a.as_mut_ptr(), xlen),
                    rse(&mut x2, b.as_mut_ptr(), xlen),
                )
            };
            eq(&format!("seedexpander ret(maxlen={maxlen},xlen={xlen})"), r1, r2);
            eq_bytes(
                &format!("seedexpander out(maxlen={maxlen},xlen={xlen})"),
                &a,
                &b,
            );
            eq(
                &format!("seedexpander ctx(maxlen={maxlen},xlen={xlen})"),
                x1,
                x2,
            );
        }
    }
}
