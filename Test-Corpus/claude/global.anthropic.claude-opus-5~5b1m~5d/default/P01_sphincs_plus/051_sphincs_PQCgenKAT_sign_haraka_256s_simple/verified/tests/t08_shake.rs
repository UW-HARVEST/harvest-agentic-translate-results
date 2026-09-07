//! CONFIGS.md rows 46-51: the raw FIPS-202 primitives.
//! Only meaningful when the shake backend is selected.

mod common;
use common::*;

type FAbsorb = unsafe extern "C" fn(*mut u64, *const u8, usize);
type FSqueezeBlocks = unsafe extern "C" fn(*mut u8, usize, *mut u64);
type FIncInit = unsafe extern "C" fn(*mut u64);
type FIncAbsorb = unsafe extern "C" fn(*mut u64, *const u8, usize);
type FIncFinalize = unsafe extern "C" fn(*mut u64);
type FIncSqueeze = unsafe extern "C" fn(*mut u8, usize, *mut u64);
type FShake = unsafe extern "C" fn(*mut u8, usize, *const u8, usize);
type FSha3IncFinal = unsafe extern "C" fn(*mut u8, *mut u64);
type FSha3 = unsafe extern "C" fn(*mut u8, *const u8, usize);

macro_rules! f {
    ($side:expr, $name:expr, $t:ty) => {
        unsafe { std::mem::transmute::<usize, $t>($side.addr($name)) }
    };
}

fn skip() -> bool {
    BACKEND != "shake"
}

/// `lib/shake/include/fips202.h` *declares* the SHAKE-128 and SHA3-256/512
/// entry points, but `lib/shake/src/fips202.c` only *defines* the SHAKE-256
/// family -- `nm -D libshake.so` lists nothing else.  The Rust port supplies
/// them anyway (extra exports are harmless), but there is no C counterpart to
/// compare against, so those rows are skipped rather than faked.
fn c_has(sym: &str) -> bool {
    libs().c.has(sym)
}

fn as_bytes(s: &[u64]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(s.as_ptr() as *const u8, s.len() * 8) }
}

/// rows 46 / 48 -- the non-incremental `*_absorb` + `*_squeezeblocks` pair
fn absorb_squeeze(prefix: &str, rate: usize, salt: u64) {
    let p = libs();
    let ca = f!(p.c, &format!("{prefix}_absorb"), FAbsorb);
    let ra = f!(p.rust, &format!("{prefix}_absorb"), FAbsorb);
    let cs = f!(p.c, &format!("{prefix}_squeezeblocks"), FSqueezeBlocks);
    let rs = f!(p.rust, &format!("{prefix}_squeezeblocks"), FSqueezeBlocks);
    let mut rng = Rng::new(SEED ^ salt);

    for &inlen in &[0usize, 1, rate - 1, rate, rate + 1, 2 * rate, 2 * rate + 1, 400] {
        for &nblocks in &[1usize, 2, 3] {
            let data = rng.bytes(inlen.max(1));
            let mut s1 = vec![0u64; 25];
            let mut s2 = vec![0u64; 25];
            unsafe {
                ca(s1.as_mut_ptr(), data.as_ptr(), inlen);
                ra(s2.as_mut_ptr(), data.as_ptr(), inlen);
            }
            eq_bytes(
                &format!("{prefix}_absorb state(inlen={inlen})"),
                as_bytes(&s1),
                as_bytes(&s2),
            );
            let mut o1 = vec![0x1Au8; nblocks * rate + 8];
            let mut o2 = o1.clone();
            unsafe {
                cs(o1.as_mut_ptr(), nblocks, s1.as_mut_ptr());
                rs(o2.as_mut_ptr(), nblocks, s2.as_mut_ptr());
            }
            eq_bytes(
                &format!("{prefix}_squeezeblocks(inlen={inlen},nblocks={nblocks})"),
                &o1,
                &o2,
            );
            eq_bytes(
                &format!("{prefix}_squeezeblocks state(inlen={inlen},nblocks={nblocks})"),
                as_bytes(&s1),
                as_bytes(&s2),
            );
        }
    }
}

/// rows 47 / 48 -- the incremental SHAKE API
fn inc_shake(prefix: &str, rate: usize, salt: u64) {
    let p = libs();
    let ci = f!(p.c, &format!("{prefix}_inc_init"), FIncInit);
    let ri = f!(p.rust, &format!("{prefix}_inc_init"), FIncInit);
    let ca = f!(p.c, &format!("{prefix}_inc_absorb"), FIncAbsorb);
    let ra = f!(p.rust, &format!("{prefix}_inc_absorb"), FIncAbsorb);
    let cfin = f!(p.c, &format!("{prefix}_inc_finalize"), FIncFinalize);
    let rfin = f!(p.rust, &format!("{prefix}_inc_finalize"), FIncFinalize);
    let cq = f!(p.c, &format!("{prefix}_inc_squeeze"), FIncSqueeze);
    let rq = f!(p.rust, &format!("{prefix}_inc_squeeze"), FIncSqueeze);
    let mut rng = Rng::new(SEED ^ salt);

    let lens: &[usize] = &[0, 1, rate - 1, rate, rate + 1, 2 * rate, 300];
    for &l1 in lens {
        for nabs in 1..=3usize {
            let mut s1 = vec![0u64; 26];
            let mut s2 = vec![0u64; 26];
            unsafe {
                ci(s1.as_mut_ptr());
                ri(s2.as_mut_ptr());
            }
            eq_bytes(
                &format!("{prefix}_inc_init"),
                as_bytes(&s1),
                as_bytes(&s2),
            );
            for k in 0..nabs {
                let l = if k == 0 { l1 } else { (l1 + 7 * k) % (2 * rate + 5) };
                let d = rng.bytes(l.max(1));
                unsafe {
                    ca(s1.as_mut_ptr(), d.as_ptr(), l);
                    ra(s2.as_mut_ptr(), d.as_ptr(), l);
                }
                eq_bytes(
                    &format!("{prefix}_inc_absorb state(l1={l1},nabs={nabs},k={k})"),
                    as_bytes(&s1),
                    as_bytes(&s2),
                );
            }
            unsafe {
                cfin(s1.as_mut_ptr());
                rfin(s2.as_mut_ptr());
            }
            eq_bytes(
                &format!("{prefix}_inc_finalize state(l1={l1},nabs={nabs})"),
                as_bytes(&s1),
                as_bytes(&s2),
            );
            for &outlen in &[1usize, 32, rate, rate + 1, 400] {
                let mut o1 = vec![0x2Bu8; outlen + 8];
                let mut o2 = o1.clone();
                unsafe {
                    cq(o1.as_mut_ptr(), outlen, s1.as_mut_ptr());
                    rq(o2.as_mut_ptr(), outlen, s2.as_mut_ptr());
                }
                eq_bytes(
                    &format!("{prefix}_inc_squeeze(l1={l1},nabs={nabs},outlen={outlen})"),
                    &o1,
                    &o2,
                );
                eq_bytes(
                    &format!("{prefix}_inc_squeeze state(l1={l1},nabs={nabs},outlen={outlen})"),
                    as_bytes(&s1),
                    as_bytes(&s2),
                );
            }
        }
    }
}

/// row 46 -- SHAKE-128 absorb/squeezeblocks (168-byte rate)
#[test]
fn row46_shake128_absorb_squeeze() {
    if skip() || !c_has("shake128_absorb") {
        return;
    }
    absorb_squeeze("shake128", 168, 46);
}

/// row 47 -- SHAKE-128 incremental
#[test]
fn row47_shake128_incremental() {
    if skip() || !c_has("shake128_inc_init") {
        return;
    }
    inc_shake("shake128", 168, 47);
}

/// row 48 -- SHAKE-256 absorb/squeezeblocks + incremental (136-byte rate)
#[test]
fn row48_shake256() {
    if skip() {
        return;
    }
    absorb_squeeze("shake256", 136, 48);
    inc_shake("shake256", 136, 481);
}

/// row 49 -- `shake128` / `shake256` one-shot
#[test]
fn row49_shake_oneshot() {
    if skip() {
        return;
    }
    let p = libs();
    let mut rng = Rng::new(SEED ^ 49);
    let names: Vec<&str> = ["shake128", "shake256"]
        .into_iter()
        .filter(|n| c_has(n))
        .collect();
    assert!(names.contains(&"shake256"), "C must define shake256");
    for name in names {
        let cf = f!(p.c, name, FShake);
        let rf = f!(p.rust, name, FShake);
        for &outlen in &[1usize, 32, 135, 136, 137, 168, 169, 500] {
            for &inlen in &[0usize, 1, 135, 136, 168, 200] {
                let data = rng.bytes(inlen.max(1));
                let mut o1 = vec![0x3Cu8; outlen + 8];
                let mut o2 = o1.clone();
                unsafe {
                    cf(o1.as_mut_ptr(), outlen, data.as_ptr(), inlen);
                    rf(o2.as_mut_ptr(), outlen, data.as_ptr(), inlen);
                }
                eq_bytes(&format!("{name}(outlen={outlen},inlen={inlen})"), &o1, &o2);
            }
        }
    }

    // Anchor: FIPS-202 SHAKE256("abc"), first 32 bytes.  Proves the test is
    // not vacuous.
    let cf = f!(p.c, "shake256", FShake);
    let rf = f!(p.rust, "shake256", FShake);
    let abc = b"abc";
    let mut o1 = [0u8; 32];
    let mut o2 = [0u8; 32];
    unsafe {
        cf(o1.as_mut_ptr(), 32, abc.as_ptr(), 3);
        rf(o2.as_mut_ptr(), 32, abc.as_ptr(), 3);
    }
    let want = [
        0x48, 0x33, 0x66, 0x60, 0x13, 0x60, 0xa8, 0x77, 0x1c, 0x68, 0x63, 0x08, 0x0c, 0xc4, 0x11,
        0x4d, 0x8d, 0xb4, 0x45, 0x30, 0xf8, 0xf1, 0xe1, 0xee, 0x4f, 0x94, 0xea, 0x37, 0xe7, 0x8b,
        0x57, 0x39,
    ];
    eq_bytes("shake256(\"abc\") KAT (C)", &o1, &want);
    eq_bytes("shake256(\"abc\") KAT (Rust)", &o2, &want);
}

/// rows 50/51 -- SHA3-256 and SHA3-512, one-shot and incremental
fn sha3(prefix: &str, outbytes: usize, rate: usize, salt: u64) {
    let p = libs();
    let cf = f!(p.c, prefix, FSha3);
    let rf = f!(p.rust, prefix, FSha3);
    let ci = f!(p.c, &format!("{prefix}_inc_init"), FIncInit);
    let ri = f!(p.rust, &format!("{prefix}_inc_init"), FIncInit);
    let ca = f!(p.c, &format!("{prefix}_inc_absorb"), FIncAbsorb);
    let ra = f!(p.rust, &format!("{prefix}_inc_absorb"), FIncAbsorb);
    let cfin = f!(p.c, &format!("{prefix}_inc_finalize"), FSha3IncFinal);
    let rfin = f!(p.rust, &format!("{prefix}_inc_finalize"), FSha3IncFinal);
    let mut rng = Rng::new(SEED ^ salt);

    let lens: &[usize] = &[0, 1, rate - 1, rate, rate + 1, 2 * rate, 300];
    for &inlen in lens {
        let data = rng.bytes(inlen.max(1));
        let mut o1 = vec![0x4Du8; outbytes + 8];
        let mut o2 = o1.clone();
        unsafe {
            cf(o1.as_mut_ptr(), data.as_ptr(), inlen);
            rf(o2.as_mut_ptr(), data.as_ptr(), inlen);
        }
        eq_bytes(&format!("{prefix}(inlen={inlen})"), &o1, &o2);

        for nabs in 1..=3usize {
            let mut s1 = vec![0u64; 26];
            let mut s2 = vec![0u64; 26];
            unsafe {
                ci(s1.as_mut_ptr());
                ri(s2.as_mut_ptr());
            }
            for k in 0..nabs {
                let l = if k == 0 {
                    inlen
                } else {
                    (inlen + 11 * k) % (2 * rate + 3)
                };
                let d = rng.bytes(l.max(1));
                unsafe {
                    ca(s1.as_mut_ptr(), d.as_ptr(), l);
                    ra(s2.as_mut_ptr(), d.as_ptr(), l);
                }
                eq_bytes(
                    &format!("{prefix}_inc_absorb(inlen={inlen},nabs={nabs},k={k})"),
                    as_bytes(&s1),
                    as_bytes(&s2),
                );
            }
            let mut q1 = vec![0x5Eu8; outbytes + 8];
            let mut q2 = q1.clone();
            unsafe {
                cfin(q1.as_mut_ptr(), s1.as_mut_ptr());
                rfin(q2.as_mut_ptr(), s2.as_mut_ptr());
            }
            eq_bytes(
                &format!("{prefix}_inc_finalize(inlen={inlen},nabs={nabs})"),
                &q1,
                &q2,
            );
            eq_bytes(
                &format!("{prefix}_inc_finalize state(inlen={inlen},nabs={nabs})"),
                as_bytes(&s1),
                as_bytes(&s2),
            );
        }
    }
}

/// row 50 -- SHA3-256
#[test]
fn row50_sha3_256() {
    if skip() || !c_has("sha3_256") {
        return;
    }
    sha3("sha3_256", 32, 136, 50);
}

/// row 51 -- SHA3-512
#[test]
fn row51_sha3_512() {
    if skip() || !c_has("sha3_512") {
        return;
    }
    sha3("sha3_512", 64, 72, 51);
}
