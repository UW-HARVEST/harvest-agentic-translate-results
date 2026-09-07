//! CONFIGS.md rows 36-40: the raw BLAKE-256 / BLAKE-512 primitives and MGF1.
//! Only meaningful when the blake backend is selected.

mod common;
use common::*;

type FInitS = unsafe extern "C" fn(*mut u8);
type FUpdate = unsafe extern "C" fn(*mut u8, *const u8, u64);
type FFinal = unsafe extern "C" fn(*mut u8, *mut u8);
type FOneShot = unsafe extern "C" fn(*mut u8, *const u8, u64) -> i32;
type FCompress = unsafe extern "C" fn(*mut u8, *const u8);
type FMgf1 = unsafe extern "C" fn(*mut u8, u64, *const u8, u64);

/// `blakestate256`: `u32 h[8], s[4], t[2]; int buflen, nullt; u8 buf[64]`
const ST256: usize = 8 * 4 + 4 * 4 + 2 * 4 + 4 + 4 + 64;
/// `blakestate512`: `u64 h[8], s[4], t[2]; int buflen, nullt; u8 buf[128]`
const ST512: usize = 8 * 8 + 4 * 8 + 2 * 8 + 4 + 4 + 128;

macro_rules! f {
    ($side:expr, $name:expr, $t:ty) => {
        unsafe { std::mem::transmute::<usize, $t>($side.addr($name)) }
    };
}

fn skip() -> bool {
    BACKEND != "blake"
}

/// Incremental init/update(*)/final for one of the two BLAKE variants.
fn incremental(
    variant: &str,
    stsize: usize,
    outbytes: usize,
    inlens: &[usize],
    splits: &[usize],
) {
    let p = libs();
    let ci = f!(p.c, &format!("{variant}_init"), FInitS);
    let ri = f!(p.rust, &format!("{variant}_init"), FInitS);
    let cu = f!(p.c, &format!("{variant}_update"), FUpdate);
    let ru = f!(p.rust, &format!("{variant}_update"), FUpdate);
    let cfin = f!(p.c, &format!("{variant}_final"), FFinal);
    let rfin = f!(p.rust, &format!("{variant}_final"), FFinal);
    let mut rng = Rng::new(SEED ^ 36);

    for &inlen in inlens {
        for &nsplit in splits {
            let data = rng.bytes(inlen.max(1));
            let mut s1 = vec![0u8; stsize];
            let mut s2 = vec![0u8; stsize];
            unsafe {
                ci(s1.as_mut_ptr());
                ri(s2.as_mut_ptr());
            }
            eq_bytes(&format!("{variant}_init state"), &s1, &s2);

            // Split the data into `nsplit` pieces (each fed as bit length).
            let mut off = 0usize;
            for k in 0..nsplit {
                let rem = inlen - off;
                let take = if k + 1 == nsplit {
                    rem
                } else {
                    rem / (nsplit - k)
                };
                unsafe {
                    cu(s1.as_mut_ptr(), data[off..].as_ptr(), (take * 8) as u64);
                    ru(s2.as_mut_ptr(), data[off..].as_ptr(), (take * 8) as u64);
                }
                eq_bytes(
                    &format!("{variant}_update state(inlen={inlen},split={nsplit},k={k})"),
                    &s1,
                    &s2,
                );
                off += take;
            }

            let mut o1 = vec![0x3Cu8; outbytes + 8];
            let mut o2 = o1.clone();
            unsafe {
                cfin(s1.as_mut_ptr(), o1.as_mut_ptr());
                rfin(s2.as_mut_ptr(), o2.as_mut_ptr());
            }
            eq_bytes(
                &format!("{variant}_final digest(inlen={inlen},split={nsplit})"),
                &o1,
                &o2,
            );
            eq_bytes(
                &format!("{variant}_final state(inlen={inlen},split={nsplit})"),
                &s1,
                &s2,
            );
        }
    }
}

/// row 36 -- `blake256_init/update/final`
#[test]
fn row36_blake256_incremental() {
    if skip() {
        return;
    }
    incremental(
        "blake256",
        ST256,
        32,
        &[0, 1, 2, 54, 55, 56, 57, 63, 64, 65, 111, 127, 128, 129, 255, 256, 1000],
        &[1, 2, 3],
    );
}

/// row 37 -- `blake256` one-shot and `blake256_compress`
#[test]
fn row37_blake256_oneshot_compress() {
    if skip() {
        return;
    }
    let p = libs();
    let c1 = f!(p.c, "blake256", FOneShot);
    let r1 = f!(p.rust, "blake256", FOneShot);
    let cc = f!(p.c, "blake256_compress", FCompress);
    let rc = f!(p.rust, "blake256_compress", FCompress);
    let mut rng = Rng::new(SEED ^ 37);

    for &inlen in &[0usize, 1, 32, 55, 56, 63, 64, 65, 128, 200, 1000] {
        let data = rng.bytes(inlen.max(1));
        let mut o1 = vec![0x4Du8; 40];
        let mut o2 = o1.clone();
        let (a, b) = unsafe {
            (
                c1(o1.as_mut_ptr(), data.as_ptr(), inlen as u64),
                r1(o2.as_mut_ptr(), data.as_ptr(), inlen as u64),
            )
        };
        eq(&format!("blake256 ret(inlen={inlen})"), a, b);
        eq_bytes(&format!("blake256(inlen={inlen})"), &o1, &o2);
    }

    for _ in 0..32 {
        let st = rng.bytes(ST256);
        let blk = rng.bytes(64);
        let mut s1 = st.clone();
        let mut s2 = st.clone();
        unsafe {
            cc(s1.as_mut_ptr(), blk.as_ptr());
            rc(s2.as_mut_ptr(), blk.as_ptr());
        }
        eq_bytes("blake256_compress", &s1, &s2);
    }
}

/// row 38 -- `blake512_init/update/final`, `blake512`, `blake512_compress`
#[test]
fn row38_blake512() {
    if skip() {
        return;
    }
    incremental(
        "blake512",
        ST512,
        64,
        &[0, 1, 111, 112, 113, 127, 128, 129, 239, 240, 255, 256, 1000],
        &[1, 2, 3],
    );

    let p = libs();
    let c1 = f!(p.c, "blake512", FOneShot);
    let r1 = f!(p.rust, "blake512", FOneShot);
    let cc = f!(p.c, "blake512_compress", FCompress);
    let rc = f!(p.rust, "blake512_compress", FCompress);
    let mut rng = Rng::new(SEED ^ 38);

    for &inlen in &[0usize, 1, 111, 112, 127, 128, 129, 256, 1000] {
        let data = rng.bytes(inlen.max(1));
        let mut o1 = vec![0x5Eu8; 72];
        let mut o2 = o1.clone();
        let (a, b) = unsafe {
            (
                c1(o1.as_mut_ptr(), data.as_ptr(), inlen as u64),
                r1(o2.as_mut_ptr(), data.as_ptr(), inlen as u64),
            )
        };
        eq(&format!("blake512 ret(inlen={inlen})"), a, b);
        eq_bytes(&format!("blake512(inlen={inlen})"), &o1, &o2);
    }

    for _ in 0..32 {
        let st = rng.bytes(ST512);
        let blk = rng.bytes(128);
        let mut s1 = st.clone();
        let mut s2 = st.clone();
        unsafe {
            cc(s1.as_mut_ptr(), blk.as_ptr());
            rc(s2.as_mut_ptr(), blk.as_ptr());
        }
        eq_bytes("blake512_compress", &s1, &s2);
    }
}

/// row 39 -- `SPX_blake256_mgf1` / `SPX_blake512_mgf1`
#[test]
fn row39_blake_mgf1() {
    if skip() {
        return;
    }
    let p = libs();
    let mut rng = Rng::new(SEED ^ 39);
    for name in ["SPX_blake256_mgf1", "SPX_blake512_mgf1"] {
        let cf = f!(p.c, name, FMgf1);
        let rf = f!(p.rust, name, FMgf1);
        for &outlen in &[1usize, 31, 32, 33, 63, 64, 65, 100, 256] {
            for &inlen in &[1usize, 16, 48, 64, 100] {
                let input = rng.bytes(inlen);
                let mut o1 = vec![0x6Fu8; outlen + 8];
                let mut o2 = o1.clone();
                unsafe {
                    cf(
                        o1.as_mut_ptr(),
                        outlen as u64,
                        input.as_ptr(),
                        inlen as u64,
                    );
                    rf(
                        o2.as_mut_ptr(),
                        outlen as u64,
                        input.as_ptr(),
                        inlen as u64,
                    );
                }
                eq_bytes(&format!("{name}(outlen={outlen},inlen={inlen})"), &o1, &o2);
            }
        }
    }
}

/// row 40 -- the exported `const u64 cst[16]`
#[test]
fn row40_cst() {
    if skip() {
        return;
    }
    let p = libs();
    let a = unsafe { std::slice::from_raw_parts(p.c.data("cst"), 16 * 8) };
    let b = unsafe { std::slice::from_raw_parts(p.rust.data("cst"), 16 * 8) };
    eq_bytes("cst", a, b);
}
