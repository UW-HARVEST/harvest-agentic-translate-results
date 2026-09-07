//! Phase B — `rng.c`, the NIST AES-256-CTR-DRBG (`CONFIGS.md` rows C54–C60).

mod common;
use common::*;

type Aes256Ecb = unsafe extern "C" fn(*mut u8, *mut u8, *mut u8);
type DrbgUpdate = unsafe extern "C" fn(*mut u8, *mut u8, *mut u8);
type RandombytesInit = unsafe extern "C" fn(*mut u8, *mut u8);
type Randombytes = unsafe extern "C" fn(*mut u8, u64) -> i32;
type SeedexpanderInit = unsafe extern "C" fn(*mut AesXofStruct, *mut u8, *mut u8, u64) -> i32;
type Seedexpander = unsafe extern "C" fn(*mut AesXofStruct, *mut u8, u64) -> i32;

/* ---- C54 --------------------------------------------------------- */

#[test]
fn c54_aes256_ecb() {
    let l = libs();
    let (c, r) = l.pair::<Aes256Ecb>("AES256_ECB");
    let mut rng = Rng::new(SEED + 54);
    for i in 0..NUM_ITERS {
        let (key, ctr) = match i {
            0 => (vec![0u8; 32], vec![0u8; 16]),
            1 => (vec![0xffu8; 32], vec![0xffu8; 16]),
            2 => (vec![0u8; 32], vec![0xffu8; 16]),
            _ => (rng.bytes(32), rng.bytes(16)),
        };
        let mut ck = key.clone();
        let mut rk = key.clone();
        let mut cc = ctr.clone();
        let mut rc = ctr.clone();
        // 16 guard bytes: EVP_EncryptUpdate must emit exactly one block.
        let mut cb = vec![0xAAu8; 32];
        let mut rb = vec![0xAAu8; 32];
        unsafe {
            c(ck.as_mut_ptr(), cc.as_mut_ptr(), cb.as_mut_ptr());
            r(rk.as_mut_ptr(), rc.as_mut_ptr(), rb.as_mut_ptr());
        }
        eq_bytes("AES256_ECB output", &cb, &rb);
        assert_eq!(&cb[16..], &[0xAAu8; 16], "AES256_ECB wrote more than 16 bytes");
        eq_bytes("AES256_ECB must not modify key", &ck, &rk);
        eq_bytes("AES256_ECB must not modify ctr", &cc, &rc);
    }
}

/* ---- C55 --------------------------------------------------------- */

#[test]
fn c55_drbg_update() {
    let l = libs();
    let (c, r) = l.pair::<DrbgUpdate>("AES256_CTR_DRBG_Update");
    let mut rng = Rng::new(SEED + 55);
    for i in 0..NUM_ITERS {
        let key = rng.bytes(32);
        let v = match i {
            0 => vec![0u8; 16],
            1 => vec![0xffu8; 16],                     // carry through all 16 bytes
            2 => {
                let mut x = vec![0xffu8; 16];
                x[0] = 0;
                x
            }
            _ => rng.bytes(16),
        };
        let provided = if i % 2 == 0 { Some(rng.bytes(48)) } else { None };
        let mut ck = key.clone();
        let mut rk = key.clone();
        let mut cv = v.clone();
        let mut rv = v.clone();
        let mut cp = provided.clone();
        let mut rp = provided.clone();
        unsafe {
            c(
                cp.as_mut().map_or(std::ptr::null_mut(), |x| x.as_mut_ptr()),
                ck.as_mut_ptr(),
                cv.as_mut_ptr(),
            );
            r(
                rp.as_mut().map_or(std::ptr::null_mut(), |x| x.as_mut_ptr()),
                rk.as_mut_ptr(),
                rv.as_mut_ptr(),
            );
        }
        eq_bytes(
            &format!("DRBG_Update Key (provided_data={})", provided.is_some()),
            &ck,
            &rk,
        );
        eq_bytes("DRBG_Update V", &cv, &rv);
        assert_eq!(cp, rp, "provided_data must not be modified");
    }
}

/* ---- C56 --------------------------------------------------------- */

#[test]
fn c56_randombytes() {
    let _drbg = drbg_lock();
    let l = libs();
    let (ci, ri) = l.pair::<RandombytesInit>("randombytes_init");
    let (c, r) = l.pair::<Randombytes>("randombytes");
    let cg = l.c::<*mut Drbg>("DRBG_ctx");
    let rg = l.rs::<*mut Drbg>("DRBG_ctx");
    let mut rng = Rng::new(SEED + 56);

    for use_pers in [false, true] {
        for xlen in [1usize, 15, 16, 17, 31, 32, 33, 48, 64, 1000] {
            let entropy = rng.bytes(48);
            let pers = rng.bytes(48);
            let mut ce = entropy.clone();
            let mut re = entropy.clone();
            let mut cp = pers.clone();
            let mut rp = pers.clone();
            unsafe {
                if use_pers {
                    ci(ce.as_mut_ptr(), cp.as_mut_ptr());
                    ri(re.as_mut_ptr(), rp.as_mut_ptr());
                } else {
                    ci(ce.as_mut_ptr(), std::ptr::null_mut());
                    ri(re.as_mut_ptr(), std::ptr::null_mut());
                }
            }
            assert_eq!(
                unsafe { **cg },
                unsafe { **rg },
                "DRBG_ctx after randombytes_init(pers={use_pers})"
            );
            let mut cb = vec![0xAAu8; xlen + 16];
            let mut rb = vec![0xAAu8; xlen + 16];
            let (cv, rv) = unsafe {
                (
                    c(cb.as_mut_ptr(), xlen as u64),
                    r(rb.as_mut_ptr(), xlen as u64),
                )
            };
            assert_eq!(cv, rv, "randombytes return (xlen={xlen})");
            assert_eq!(cv, 0, "RNG_SUCCESS");
            eq_bytes(&format!("randombytes output (xlen={xlen})"), &cb, &rb);
            assert_eq!(&cb[xlen..], &[0xAAu8; 16], "randombytes overran xlen");
            assert_eq!(unsafe { **cg }, unsafe { **rg }, "DRBG_ctx after randombytes");
        }
    }
}

/* ---- C57 --------------------------------------------------------- */

#[test]
fn c57_randombytes_chained() {
    let _drbg = drbg_lock();
    let l = libs();
    let (ci, ri) = l.pair::<RandombytesInit>("randombytes_init");
    let (c, r) = l.pair::<Randombytes>("randombytes");
    let cg = l.c::<*mut Drbg>("DRBG_ctx");
    let rg = l.rs::<*mut Drbg>("DRBG_ctx");
    let mut rng = Rng::new(SEED + 57);
    let entropy: Vec<u8> = (0..48u8).collect();
    let mut ce = entropy.clone();
    let mut re = entropy.clone();
    unsafe {
        ci(ce.as_mut_ptr(), std::ptr::null_mut());
        ri(re.as_mut_ptr(), std::ptr::null_mut());
    }
    for round in 0..64 {
        let xlen = 1 + rng.below(200) as usize;
        let mut cb = vec![0u8; xlen];
        let mut rb = vec![0u8; xlen];
        unsafe {
            c(cb.as_mut_ptr(), xlen as u64);
            r(rb.as_mut_ptr(), xlen as u64);
        }
        eq_bytes(&format!("chained randombytes round {round} (xlen={xlen})"), &cb, &rb);
        let (cd, rd) = unsafe { (**cg, **rg) };
        assert_eq!(cd, rd, "DRBG_ctx round {round}");
        assert_eq!(cd.reseed_counter, round + 2, "reseed_counter round {round}");
    }
}

/* ---- C58 --------------------------------------------------------- */

#[test]
fn c58_seedexpander() {
    let _drbg = drbg_lock();
    let l = libs();
    let (ci, ri) = l.pair::<SeedexpanderInit>("seedexpander_init");
    let (c, r) = l.pair::<Seedexpander>("seedexpander");
    let mut rng = Rng::new(SEED + 58);

    for maxlen in [1u64, 16, 17, 256, 0xFFFF, 0xFFFF_FFFF] {
        let seed = rng.bytes(32);
        let div = rng.bytes(8);
        let mut cs = seed.clone();
        let mut rs = seed.clone();
        let mut cd = div.clone();
        let mut rd = div.clone();
        let mut cctx = AesXofStruct::default();
        let mut rctx = AesXofStruct::default();
        let (cv, rv) = unsafe {
            (
                ci(&mut cctx, cs.as_mut_ptr(), cd.as_mut_ptr(), maxlen),
                ri(&mut rctx, rs.as_mut_ptr(), rd.as_mut_ptr(), maxlen),
            )
        };
        assert_eq!(cv, rv, "seedexpander_init return (maxlen={maxlen})");
        assert_eq!(cv, 0, "RNG_SUCCESS for maxlen < 2^32");
        assert_eq!(cctx, rctx, "AES_XOF_struct after init (maxlen={maxlen})");

        // (a) inside the 16-byte buffer, (b) straddling it, (c) several blocks
        let seqs: Vec<Vec<u64>> = vec![
            vec![1, 1, 1, 1],
            vec![8, 8, 8],
            vec![15, 1, 16],
            vec![16, 16, 16],
            vec![17, 33, 5],
            vec![48],
            vec![0, 1, 0, 2],
        ];
        for seq in seqs {
            if seq.iter().sum::<u64>() >= maxlen {
                continue;
            }
            let mut cctx2 = cctx;
            let mut rctx2 = rctx;
            for &xlen in &seq {
                let mut cb = vec![0xAAu8; xlen as usize + 16];
                let mut rb = vec![0xAAu8; xlen as usize + 16];
                let (cv, rv) = unsafe {
                    (
                        c(&mut cctx2, cb.as_mut_ptr(), xlen),
                        r(&mut rctx2, rb.as_mut_ptr(), xlen),
                    )
                };
                assert_eq!(cv, rv, "seedexpander return (maxlen={maxlen}, xlen={xlen})");
                eq_bytes(&format!("seedexpander out (maxlen={maxlen},xlen={xlen})"), &cb, &rb);
                assert_eq!(
                    &cb[xlen as usize..],
                    &[0xAAu8; 16],
                    "seedexpander overran xlen"
                );
                assert_eq!(cctx2, rctx2, "AES_XOF_struct (maxlen={maxlen},xlen={xlen})");
            }
        }
    }
}

/* ---- C59 --------------------------------------------------------- */

#[test]
fn c59_seedexpander_carry() {
    let _drbg = drbg_lock();
    let l = libs();
    let (ci, ri) = l.pair::<SeedexpanderInit>("seedexpander_init");
    let (c, r) = l.pair::<Seedexpander>("seedexpander");
    let mut rng = Rng::new(SEED + 59);
    // seedexpander_init zeroes ctr[12..16], so the counter starts at 0 and the
    // carry in ctr[15..12] is reached by squeezing enough blocks; do >600 blocks
    // to move ctr[14] and force the ctr[15] wrap many times.
    let seed = rng.bytes(32);
    let div = rng.bytes(8);
    let mut cs = seed.clone();
    let mut rs = seed.clone();
    let mut cd = div.clone();
    let mut rd = div.clone();
    let mut cctx = AesXofStruct::default();
    let mut rctx = AesXofStruct::default();
    unsafe {
        ci(&mut cctx, cs.as_mut_ptr(), cd.as_mut_ptr(), 0x10_0000);
        ri(&mut rctx, rs.as_mut_ptr(), rd.as_mut_ptr(), 0x10_0000);
    }
    assert_eq!(cctx, rctx);
    for round in 0..700 {
        let mut cb = vec![0u8; 16];
        let mut rb = vec![0u8; 16];
        let (cv, rv) = unsafe { (c(&mut cctx, cb.as_mut_ptr(), 16), r(&mut rctx, rb.as_mut_ptr(), 16)) };
        assert_eq!(cv, rv, "seedexpander return round {round}");
        eq_bytes(&format!("seedexpander carry round {round}"), &cb, &rb);
        assert_eq!(cctx, rctx, "AES_XOF_struct round {round}");
    }
    assert!(cctx.ctr[14] > 0, "the test did not actually reach a ctr[15] carry");
}

/* ---- C60 --------------------------------------------------------- */

#[test]
fn c60_drbg_ctx_global() {
    let _drbg = drbg_lock();
    let l = libs();
    let cg = l.c::<*mut Drbg>("DRBG_ctx");
    let rg = l.rs::<*mut Drbg>("DRBG_ctx");
    // Freshly loaded: both must be zero-initialised (C: a plain global).
    let (ci, ri) = l.pair::<RandombytesInit>("randombytes_init");
    let (c, r) = l.pair::<Randombytes>("randombytes");
    let mut rng = Rng::new(SEED + 60);
    for i in 0..8 {
        let entropy = rng.bytes(48);
        let pers = rng.bytes(48);
        let mut ce = entropy.clone();
        let mut re = entropy.clone();
        let mut cp = pers.clone();
        let mut rp = pers.clone();
        unsafe {
            if i % 2 == 0 {
                ci(ce.as_mut_ptr(), std::ptr::null_mut());
                ri(re.as_mut_ptr(), std::ptr::null_mut());
            } else {
                ci(ce.as_mut_ptr(), cp.as_mut_ptr());
                ri(re.as_mut_ptr(), rp.as_mut_ptr());
            }
            let mut cb = vec![0u8; 97];
            let mut rb = vec![0u8; 97];
            c(cb.as_mut_ptr(), 97);
            r(rb.as_mut_ptr(), 97);
            eq_bytes("randombytes output", &cb, &rb);
        }
        let (cd, rd) = unsafe { (**cg, **rg) };
        // Compare the raw 52-byte struct images too, not just field-by-field.
        let cbytes = unsafe { std::slice::from_raw_parts(*cg as *const u8, 52) };
        let rbytes = unsafe { std::slice::from_raw_parts(*rg as *const u8, 52) };
        eq_bytes("DRBG_ctx raw image", cbytes, rbytes);
        assert_eq!(cd, rd);
        assert_eq!(cd.reseed_counter, 2);
    }
}
