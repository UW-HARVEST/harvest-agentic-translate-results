//! CONFIGS.md rows 41-45: the raw SHA-256 / SHA-512 primitives, MGF1 and
//! `seed_state`.  Only meaningful when the sha2 backend is selected.

mod common;
use common::*;

type FIncInit = unsafe extern "C" fn(*mut u8);
type FIncBlocks = unsafe extern "C" fn(*mut u8, *const u8, usize);
type FIncFinal = unsafe extern "C" fn(*mut u8, *mut u8, *const u8, usize);
type FOneShot = unsafe extern "C" fn(*mut u8, *const u8, usize);
type FMgf1 = unsafe extern "C" fn(*mut u8, u64, *const u8, u64);
type FSeedState = unsafe extern "C" fn(*mut u8);

macro_rules! f {
    ($side:expr, $name:expr, $t:ty) => {
        unsafe { std::mem::transmute::<usize, $t>($side.addr($name)) }
    };
}

fn skip() -> bool {
    BACKEND != "sha2"
}

fn inc_variant(
    prefix: &str,
    statelen: usize,
    outlen: usize,
    blocklen: usize,
    nblocks: &[usize],
    inlens: &[usize],
    salt: u64,
) {
    let p = libs();
    let ci = f!(p.c, &format!("{prefix}_inc_init"), FIncInit);
    let ri = f!(p.rust, &format!("{prefix}_inc_init"), FIncInit);
    let cb = f!(p.c, &format!("{prefix}_inc_blocks"), FIncBlocks);
    let rb = f!(p.rust, &format!("{prefix}_inc_blocks"), FIncBlocks);
    let cfin = f!(p.c, &format!("{prefix}_inc_finalize"), FIncFinal);
    let rfin = f!(p.rust, &format!("{prefix}_inc_finalize"), FIncFinal);
    let mut rng = Rng::new(SEED ^ salt);

    for &nb in nblocks {
        for &inlen in inlens {
            let blocks = rng.bytes((nb * blocklen).max(1));
            let tail = rng.bytes(inlen.max(1));
            let mut s1 = vec![0xA7u8; statelen];
            let mut s2 = s1.clone();
            unsafe {
                ci(s1.as_mut_ptr());
                ri(s2.as_mut_ptr());
            }
            eq_bytes(&format!("{prefix}_inc_init"), &s1, &s2);
            unsafe {
                cb(s1.as_mut_ptr(), blocks.as_ptr(), nb);
                rb(s2.as_mut_ptr(), blocks.as_ptr(), nb);
            }
            eq_bytes(&format!("{prefix}_inc_blocks(nb={nb})"), &s1, &s2);
            let mut o1 = vec![0xB8u8; outlen + 8];
            let mut o2 = o1.clone();
            unsafe {
                cfin(o1.as_mut_ptr(), s1.as_mut_ptr(), tail.as_ptr(), inlen);
                rfin(o2.as_mut_ptr(), s2.as_mut_ptr(), tail.as_ptr(), inlen);
            }
            eq_bytes(
                &format!("{prefix}_inc_finalize out(nb={nb},inlen={inlen})"),
                &o1,
                &o2,
            );
            eq_bytes(
                &format!("{prefix}_inc_finalize state(nb={nb},inlen={inlen})"),
                &s1,
                &s2,
            );
        }
    }
}

/// row 41 -- `sha256_inc_init` / `sha256_inc_blocks` / `sha256_inc_finalize`
#[test]
fn row41_sha256_incremental() {
    if skip() {
        return;
    }
    inc_variant(
        "sha256",
        40,
        32,
        64,
        &[0, 1, 2, 5],
        &[0, 1, 54, 55, 56, 57, 63, 64, 65, 200],
        41,
    );
}

/// row 42 -- `sha256` one-shot
#[test]
fn row42_sha256_oneshot() {
    if skip() {
        return;
    }
    let p = libs();
    let cf = f!(p.c, "sha256", FOneShot);
    let rf = f!(p.rust, "sha256", FOneShot);
    let mut rng = Rng::new(SEED ^ 42);
    for &inlen in &[0usize, 1, 55, 56, 63, 64, 65, 119, 120, 128, 1000] {
        let data = rng.bytes(inlen.max(1));
        let mut o1 = vec![0xC9u8; 40];
        let mut o2 = o1.clone();
        unsafe {
            cf(o1.as_mut_ptr(), data.as_ptr(), inlen);
            rf(o2.as_mut_ptr(), data.as_ptr(), inlen);
        }
        eq_bytes(&format!("sha256(inlen={inlen})"), &o1, &o2);
    }

    // Anchor: FIPS-180 SHA-256("abc").  Proves this test is not vacuous.
    let abc = b"abc";
    let mut o1 = [0u8; 32];
    let mut o2 = [0u8; 32];
    unsafe {
        cf(o1.as_mut_ptr(), abc.as_ptr(), 3);
        rf(o2.as_mut_ptr(), abc.as_ptr(), 3);
    }
    let want = [
        0xba, 0x78, 0x16, 0xbf, 0x8f, 0x01, 0xcf, 0xea, 0x41, 0x41, 0x40, 0xde, 0x5d, 0xae, 0x22,
        0x23, 0xb0, 0x03, 0x61, 0xa3, 0x96, 0x17, 0x7a, 0x9c, 0xb4, 0x10, 0xff, 0x61, 0xf2, 0x00,
        0x15, 0xad,
    ];
    eq_bytes("sha256(\"abc\") KAT (C)", &o1, &want);
    eq_bytes("sha256(\"abc\") KAT (Rust)", &o2, &want);
}

/// row 43 -- the SHA-512 incremental API and one-shot
#[test]
fn row43_sha512() {
    if skip() {
        return;
    }
    inc_variant(
        "sha512",
        72,
        64,
        128,
        &[0, 1, 2, 3],
        &[0, 1, 110, 111, 112, 113, 127, 128, 129, 300],
        43,
    );

    let p = libs();
    let cf = f!(p.c, "sha512", FOneShot);
    let rf = f!(p.rust, "sha512", FOneShot);
    let mut rng = Rng::new(SEED ^ 431);
    for &inlen in &[0usize, 1, 111, 112, 127, 128, 129, 240, 256, 1000] {
        let data = rng.bytes(inlen.max(1));
        let mut o1 = vec![0xDAu8; 72];
        let mut o2 = o1.clone();
        unsafe {
            cf(o1.as_mut_ptr(), data.as_ptr(), inlen);
            rf(o2.as_mut_ptr(), data.as_ptr(), inlen);
        }
        eq_bytes(&format!("sha512(inlen={inlen})"), &o1, &o2);
    }
}

/// row 44 -- `SPX_mgf1_256` / `SPX_mgf1_512`
#[test]
fn row44_mgf1() {
    if skip() {
        return;
    }
    let p = libs();
    let mut rng = Rng::new(SEED ^ 44);
    for name in ["SPX_mgf1_256", "SPX_mgf1_512"] {
        let cf = f!(p.c, name, FMgf1);
        let rf = f!(p.rust, name, FMgf1);
        for &outlen in &[1usize, 31, 32, 33, 63, 64, 65, 100, 256] {
            for &inlen in &[1usize, 16, 48, 64, 100] {
                let input = rng.bytes(inlen);
                let mut o1 = vec![0xEBu8; outlen + 8];
                let mut o2 = o1.clone();
                unsafe {
                    cf(o1.as_mut_ptr(), outlen as u64, input.as_ptr(), inlen as u64);
                    rf(o2.as_mut_ptr(), outlen as u64, input.as_ptr(), inlen as u64);
                }
                eq_bytes(&format!("{name}(outlen={outlen},inlen={inlen})"), &o1, &o2);
            }
        }
    }
}

/// row 45 -- `SPX_seed_state`
#[test]
fn row45_seed_state() {
    if skip() {
        return;
    }
    let p = libs();
    let cf = f!(p.c, "SPX_seed_state", FSeedState);
    let rf = f!(p.rust, "SPX_seed_state", FSeedState);
    let mut rng = Rng::new(SEED ^ 45);
    for iter in 0..16 {
        let mut a = new_ctx();
        match iter {
            0 => {}
            1 => a[..2 * SPX_N].iter_mut().for_each(|x| *x = 0xFF),
            _ => seed_ctx(&mut a, &mut rng),
        }
        let mut b = a.clone();
        unsafe {
            cf(a.as_mut_ptr());
            rf(b.as_mut_ptr());
        }
        eq_bytes(
            &format!("seed_state(iter={iter})"),
            &a[..CTX_LIVE],
            &b[..CTX_LIVE],
        );
    }
}
