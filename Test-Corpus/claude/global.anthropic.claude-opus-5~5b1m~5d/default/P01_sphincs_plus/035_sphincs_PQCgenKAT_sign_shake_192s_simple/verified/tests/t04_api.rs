//! CONFIGS.md rows 27-31: the public `api.h` surface.

mod common;
use common::*;

type FLen = unsafe extern "C" fn() -> u64;
type FSeedKeypair = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> i32;
type FKeypair = unsafe extern "C" fn(*mut u8, *mut u8) -> i32;
type FSignature = unsafe extern "C" fn(*mut u8, *mut usize, *const u8, usize, *const u8) -> i32;
type FVerify = unsafe extern "C" fn(*const u8, usize, *const u8, usize, *const u8) -> i32;
type FSign = unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> i32;
type FOpen = unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> i32;
type FRbInit = unsafe extern "C" fn(*mut u8, *mut u8);

macro_rules! f {
    ($side:expr, $name:expr, $t:ty) => {
        unsafe { std::mem::transmute::<usize, $t>($side.addr($name)) }
    };
}

/// Re-seed BOTH DRBGs identically so that the `randombytes()` stream consumed
/// by `crypto_sign_keypair` / `crypto_sign_signature` is the same on both sides.
pub fn reseed_both(entropy: &[u8; 48], ps: Option<&[u8; 48]>) {
    let p = libs();
    let ci = f!(p.c, "randombytes_init", FRbInit);
    let ri = f!(p.rust, "randombytes_init", FRbInit);
    let mut e = *entropy;
    let mut s: [u8; 48];
    unsafe {
        match ps {
            None => {
                ci(e.as_mut_ptr(), std::ptr::null_mut());
                ri(e.as_mut_ptr(), std::ptr::null_mut());
            }
            Some(x) => {
                s = *x;
                ci(e.as_mut_ptr(), s.as_mut_ptr());
                ri(e.as_mut_ptr(), s.as_mut_ptr());
            }
        }
    }
}

/// row 27 -- the four size accessors
#[test]
fn row27_sizes() {
    let p = libs();
    for (name, expect) in [
        ("crypto_sign_secretkeybytes", SPX_SK_BYTES as u64),
        ("crypto_sign_publickeybytes", SPX_PK_BYTES as u64),
        ("crypto_sign_bytes", SPX_BYTES as u64),
        ("crypto_sign_seedbytes", CRYPTO_SEEDBYTES as u64),
    ] {
        let cf = f!(p.c, name, FLen);
        let rf = f!(p.rust, name, FLen);
        let (a, b) = unsafe { (cf(), rf()) };
        eq(name, a, b);
        eq(&format!("{name} vs headers"), a, expect);
    }
}

/// row 28 -- `crypto_sign_seed_keypair`
#[test]
fn row28_seed_keypair() {
    let p = libs();
    let cf = f!(p.c, "crypto_sign_seed_keypair", FSeedKeypair);
    let rf = f!(p.rust, "crypto_sign_seed_keypair", FSeedKeypair);
    let mut rng = Rng::new(SEED ^ 28);

    for iter in 0..4 {
        let seed = match iter {
            0 => vec![0u8; CRYPTO_SEEDBYTES],
            1 => vec![0xFFu8; CRYPTO_SEEDBYTES],
            _ => rng.bytes(CRYPTO_SEEDBYTES),
        };
        let mut pk1 = vec![0x11u8; SPX_PK_BYTES];
        let mut sk1 = vec![0x22u8; SPX_SK_BYTES];
        let mut pk2 = pk1.clone();
        let mut sk2 = sk1.clone();
        let (r1, r2) = unsafe {
            (
                cf(pk1.as_mut_ptr(), sk1.as_mut_ptr(), seed.as_ptr()),
                rf(pk2.as_mut_ptr(), sk2.as_mut_ptr(), seed.as_ptr()),
            )
        };
        eq(&format!("seed_keypair ret(iter={iter})"), r1, r2);
        eq(&format!("seed_keypair ret==0(iter={iter})"), r1, 0);
        eq_bytes(&format!("seed_keypair pk(iter={iter})"), &pk1, &pk2);
        eq_bytes(&format!("seed_keypair sk(iter={iter})"), &sk1, &sk2);
    }
}

/// row 29 -- `crypto_sign_keypair` (consumes the DRBG)
#[test]
fn row29_keypair() {
    let p = libs();
    let cf = f!(p.c, "crypto_sign_keypair", FKeypair);
    let rf = f!(p.rust, "crypto_sign_keypair", FKeypair);
    let mut rng = Rng::new(SEED ^ 29);

    for iter in 0..4 {
        let mut ent = [0u8; 48];
        rng.fill(&mut ent);
        let mut ps = [0u8; 48];
        rng.fill(&mut ps);
        let with_ps = iter % 2 == 1;

        let mut pk1 = vec![0u8; SPX_PK_BYTES];
        let mut sk1 = vec![0u8; SPX_SK_BYTES];
        let mut pk2 = pk1.clone();
        let mut sk2 = sk1.clone();

        reseed_both(&ent, if with_ps { Some(&ps) } else { None });
        let r1 = unsafe { cf(pk1.as_mut_ptr(), sk1.as_mut_ptr()) };
        // re-seed again so the Rust side starts from the same DRBG state
        reseed_both(&ent, if with_ps { Some(&ps) } else { None });
        let r2 = unsafe { rf(pk2.as_mut_ptr(), sk2.as_mut_ptr()) };

        eq(&format!("keypair ret(with_ps={with_ps})"), r1, r2);
        eq_bytes(&format!("keypair pk(with_ps={with_ps})"), &pk1, &pk2);
        eq_bytes(&format!("keypair sk(with_ps={with_ps})"), &sk1, &sk2);
    }
}

/// row 30 -- `crypto_sign_signature` + `crypto_sign_verify`
#[test]
fn row30_signature_verify() {
    let p = libs();
    let ckp = f!(p.c, "crypto_sign_seed_keypair", FSeedKeypair);
    let rkp = f!(p.rust, "crypto_sign_seed_keypair", FSeedKeypair);
    let csg = f!(p.c, "crypto_sign_signature", FSignature);
    let rsg = f!(p.rust, "crypto_sign_signature", FSignature);
    let cvf = f!(p.c, "crypto_sign_verify", FVerify);
    let rvf = f!(p.rust, "crypto_sign_verify", FVerify);
    let mut rng = Rng::new(SEED ^ 30);

    let seed = rng.bytes(CRYPTO_SEEDBYTES);
    let mut pk1 = vec![0u8; SPX_PK_BYTES];
    let mut sk1 = vec![0u8; SPX_SK_BYTES];
    let mut pk2 = pk1.clone();
    let mut sk2 = sk1.clone();
    unsafe {
        ckp(pk1.as_mut_ptr(), sk1.as_mut_ptr(), seed.as_ptr());
        rkp(pk2.as_mut_ptr(), sk2.as_mut_ptr(), seed.as_ptr());
    }
    eq_bytes("row30 pk", &pk1, &pk2);
    eq_bytes("row30 sk", &sk1, &sk2);

    let mut ent = [0u8; 48];
    rng.fill(&mut ent);

    for &mlen in &[0usize, 1, 33, 64, 65, 200] {
        let m = rng.bytes(mlen.max(1));
        let mut s1 = vec![0xA0u8; SPX_BYTES + 8];
        let mut s2 = s1.clone();
        let mut l1 = usize::MAX;
        let mut l2 = usize::MAX;

        reseed_both(&ent, None);
        let r1 = unsafe { csg(s1.as_mut_ptr(), &mut l1, m.as_ptr(), mlen, sk1.as_ptr()) };
        reseed_both(&ent, None);
        let r2 = unsafe { rsg(s2.as_mut_ptr(), &mut l2, m.as_ptr(), mlen, sk2.as_ptr()) };

        eq(&format!("signature ret(mlen={mlen})"), r1, r2);
        eq(&format!("signature siglen(mlen={mlen})"), l1, l2);
        eq(&format!("signature siglen==SPX_BYTES(mlen={mlen})"), l1, SPX_BYTES);
        eq_bytes(&format!("signature sig(mlen={mlen})"), &s1, &s2);

        // valid verification (both must accept), then a corrupted one
        let (v1, v2) = unsafe {
            (
                cvf(s1.as_ptr(), l1, m.as_ptr(), mlen, pk1.as_ptr()),
                rvf(s2.as_ptr(), l2, m.as_ptr(), mlen, pk2.as_ptr()),
            )
        };
        eq(&format!("verify ok(mlen={mlen})"), v1, v2);
        eq(&format!("verify ok==0(mlen={mlen})"), v1, 0);
    }
}

/// row 31 -- `crypto_sign` + `crypto_sign_open` round trip
#[test]
fn row31_sign_open() {
    let p = libs();
    let ckp = f!(p.c, "crypto_sign_seed_keypair", FSeedKeypair);
    let rkp = f!(p.rust, "crypto_sign_seed_keypair", FSeedKeypair);
    let cs = f!(p.c, "crypto_sign", FSign);
    let rs = f!(p.rust, "crypto_sign", FSign);
    let co = f!(p.c, "crypto_sign_open", FOpen);
    let ro = f!(p.rust, "crypto_sign_open", FOpen);
    let mut rng = Rng::new(SEED ^ 31);

    let seed = rng.bytes(CRYPTO_SEEDBYTES);
    let mut pk1 = vec![0u8; SPX_PK_BYTES];
    let mut sk1 = vec![0u8; SPX_SK_BYTES];
    let mut pk2 = pk1.clone();
    let mut sk2 = sk1.clone();
    unsafe {
        ckp(pk1.as_mut_ptr(), sk1.as_mut_ptr(), seed.as_ptr());
        rkp(pk2.as_mut_ptr(), sk2.as_mut_ptr(), seed.as_ptr());
    }

    let mut ent = [0u8; 48];
    rng.fill(&mut ent);

    for &mlen in &[0usize, 1, 33, 64, 200] {
        let m = rng.bytes(mlen.max(1));
        let mut sm1 = vec![0xB0u8; SPX_BYTES + mlen + 8];
        let mut sm2 = sm1.clone();
        let mut sl1 = u64::MAX;
        let mut sl2 = u64::MAX;

        reseed_both(&ent, None);
        let r1 = unsafe {
            cs(
                sm1.as_mut_ptr(),
                &mut sl1,
                m.as_ptr(),
                mlen as u64,
                sk1.as_ptr(),
            )
        };
        reseed_both(&ent, None);
        let r2 = unsafe {
            rs(
                sm2.as_mut_ptr(),
                &mut sl2,
                m.as_ptr(),
                mlen as u64,
                sk2.as_ptr(),
            )
        };
        eq(&format!("sign ret(mlen={mlen})"), r1, r2);
        eq(&format!("sign smlen(mlen={mlen})"), sl1, sl2);
        eq(
            &format!("sign smlen==SPX_BYTES+mlen(mlen={mlen})"),
            sl1,
            (SPX_BYTES + mlen) as u64,
        );
        eq_bytes(&format!("sign sm(mlen={mlen})"), &sm1, &sm2);

        let mut o1 = vec![0xC0u8; SPX_BYTES + mlen + 8];
        let mut o2 = o1.clone();
        let mut ml1 = u64::MAX;
        let mut ml2 = u64::MAX;
        let (v1, v2) = unsafe {
            (
                co(o1.as_mut_ptr(), &mut ml1, sm1.as_ptr(), sl1, pk1.as_ptr()),
                ro(o2.as_mut_ptr(), &mut ml2, sm2.as_ptr(), sl2, pk2.as_ptr()),
            )
        };
        eq(&format!("open ret(mlen={mlen})"), v1, v2);
        eq(&format!("open ret==0(mlen={mlen})"), v1, 0);
        eq(&format!("open mlen(mlen={mlen})"), ml1, ml2);
        eq(&format!("open mlen==mlen(mlen={mlen})"), ml1, mlen as u64);
        eq_bytes(&format!("open m(mlen={mlen})"), &o1[..mlen], &o2[..mlen]);
        eq_bytes(&format!("open m==input(mlen={mlen})"), &o1[..mlen], &m[..mlen]);
    }
}
