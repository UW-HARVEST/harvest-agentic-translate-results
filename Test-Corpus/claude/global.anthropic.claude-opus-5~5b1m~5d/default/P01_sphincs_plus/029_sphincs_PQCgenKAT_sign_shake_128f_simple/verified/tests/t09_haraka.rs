//! CONFIGS.md rows 52-55: the raw Haraka primitives.
//! Only meaningful when the haraka backend is selected.

mod common;
use common::*;

type FTweak = unsafe extern "C" fn(*mut u8);
type FIncInit = unsafe extern "C" fn(*mut u8);
type FIncAbsorb = unsafe extern "C" fn(*mut u8, *const u8, usize, *const u8);
type FIncFinalize = unsafe extern "C" fn(*mut u8);
type FIncSqueeze = unsafe extern "C" fn(*mut u8, usize, *mut u8, *const u8);
type FHarakaS = unsafe extern "C" fn(*mut u8, u64, *const u8, u64, *const u8);
type FPerm = unsafe extern "C" fn(*mut u8, *const u8, *const u8);

/// HARAKAS_RATE from `lib/haraka/src/haraka.c`
const RATE: usize = 32;

macro_rules! f {
    ($side:expr, $name:expr, $t:ty) => {
        unsafe { std::mem::transmute::<usize, $t>($side.addr($name)) }
    };
}

fn skip() -> bool {
    BACKEND != "haraka"
}

/// row 52 -- `SPX_tweak_constants` (both constant tables compared)
#[test]
fn row52_tweak_constants() {
    if skip() {
        return;
    }
    let p = libs();
    let cf = f!(p.c, "SPX_tweak_constants", FTweak);
    let rf = f!(p.rust, "SPX_tweak_constants", FTweak);
    let mut rng = Rng::new(SEED ^ 52);
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
            &format!("tweak_constants(iter={iter})"),
            &a[..CTX_LIVE],
            &b[..CTX_LIVE],
        );
    }
}

fn tweaked_pair(rng: &mut Rng) -> (Vec<u8>, Vec<u8>) {
    let p = libs();
    let cf = f!(p.c, "SPX_tweak_constants", FTweak);
    let rf = f!(p.rust, "SPX_tweak_constants", FTweak);
    let mut a = new_ctx();
    seed_ctx(&mut a, rng);
    let mut b = a.clone();
    unsafe {
        cf(a.as_mut_ptr());
        rf(b.as_mut_ptr());
    }
    (a, b)
}

/// row 53 -- `SPX_haraka512_perm`, `SPX_haraka512`, `SPX_haraka256`
#[test]
fn row53_haraka_perms() {
    if skip() {
        return;
    }
    let p = libs();
    let cperm = f!(p.c, "SPX_haraka512_perm", FPerm);
    let rperm = f!(p.rust, "SPX_haraka512_perm", FPerm);
    let c512 = f!(p.c, "SPX_haraka512", FPerm);
    let r512 = f!(p.rust, "SPX_haraka512", FPerm);
    let c256 = f!(p.c, "SPX_haraka256", FPerm);
    let r256 = f!(p.rust, "SPX_haraka256", FPerm);
    let mut rng = Rng::new(SEED ^ 53);

    for iter in 0..32 {
        let (ca, ra) = tweaked_pair(&mut rng);
        let in64 = match iter {
            0 => vec![0u8; 64],
            1 => vec![0xFFu8; 64],
            _ => rng.bytes(64),
        };
        let mut o1 = vec![0x11u8; 72];
        let mut o2 = o1.clone();
        unsafe {
            cperm(o1.as_mut_ptr(), in64.as_ptr(), ca.as_ptr());
            rperm(o2.as_mut_ptr(), in64.as_ptr(), ra.as_ptr());
        }
        eq_bytes("haraka512_perm", &o1, &o2);

        let mut q1 = vec![0x22u8; 40];
        let mut q2 = q1.clone();
        unsafe {
            c512(q1.as_mut_ptr(), in64.as_ptr(), ca.as_ptr());
            r512(q2.as_mut_ptr(), in64.as_ptr(), ra.as_ptr());
        }
        eq_bytes("haraka512", &q1, &q2);

        let in32 = &in64[..32];
        let mut r1 = vec![0x33u8; 40];
        let mut r2 = r1.clone();
        unsafe {
            c256(r1.as_mut_ptr(), in32.as_ptr(), ca.as_ptr());
            r256(r2.as_mut_ptr(), in32.as_ptr(), ra.as_ptr());
        }
        eq_bytes("haraka256", &r1, &r2);
    }
}

/// row 54 -- the incremental Haraka sponge
#[test]
fn row54_haraka_S_incremental() {
    if skip() {
        return;
    }
    let p = libs();
    let ci = f!(p.c, "SPX_haraka_S_inc_init", FIncInit);
    let ri = f!(p.rust, "SPX_haraka_S_inc_init", FIncInit);
    let ca = f!(p.c, "SPX_haraka_S_inc_absorb", FIncAbsorb);
    let ra = f!(p.rust, "SPX_haraka_S_inc_absorb", FIncAbsorb);
    let cfin = f!(p.c, "SPX_haraka_S_inc_finalize", FIncFinalize);
    let rfin = f!(p.rust, "SPX_haraka_S_inc_finalize", FIncFinalize);
    let cq = f!(p.c, "SPX_haraka_S_inc_squeeze", FIncSqueeze);
    let rq = f!(p.rust, "SPX_haraka_S_inc_squeeze", FIncSqueeze);
    let mut rng = Rng::new(SEED ^ 54);

    for &l1 in &[0usize, 1, RATE - 1, RATE, RATE + 1, 2 * RATE, 100] {
        for nabs in 1..=3usize {
            let (ca_ctx, ra_ctx) = tweaked_pair(&mut rng);
            let mut s1 = vec![0x44u8; 65];
            let mut s2 = s1.clone();
            unsafe {
                ci(s1.as_mut_ptr());
                ri(s2.as_mut_ptr());
            }
            eq_bytes("haraka_S_inc_init", &s1, &s2);
            for k in 0..nabs {
                let l = if k == 0 { l1 } else { (l1 + 5 * k) % (2 * RATE + 3) };
                let d = rng.bytes(l.max(1));
                unsafe {
                    ca(s1.as_mut_ptr(), d.as_ptr(), l, ca_ctx.as_ptr());
                    ra(s2.as_mut_ptr(), d.as_ptr(), l, ra_ctx.as_ptr());
                }
                eq_bytes(
                    &format!("haraka_S_inc_absorb(l1={l1},nabs={nabs},k={k})"),
                    &s1,
                    &s2,
                );
            }
            unsafe {
                cfin(s1.as_mut_ptr());
                rfin(s2.as_mut_ptr());
            }
            eq_bytes(
                &format!("haraka_S_inc_finalize(l1={l1},nabs={nabs})"),
                &s1,
                &s2,
            );
            for &outlen in &[1usize, 16, 32, 33, 64, 100] {
                let mut o1 = vec![0x55u8; outlen + 8];
                let mut o2 = o1.clone();
                unsafe {
                    cq(o1.as_mut_ptr(), outlen, s1.as_mut_ptr(), ca_ctx.as_ptr());
                    rq(o2.as_mut_ptr(), outlen, s2.as_mut_ptr(), ra_ctx.as_ptr());
                }
                eq_bytes(
                    &format!("haraka_S_inc_squeeze(l1={l1},nabs={nabs},outlen={outlen})"),
                    &o1,
                    &o2,
                );
                eq_bytes(
                    &format!("haraka_S_inc_squeeze state(l1={l1},nabs={nabs},outlen={outlen})"),
                    &s1,
                    &s2,
                );
            }
        }
    }
}

/// row 55 -- `SPX_haraka_S` one-shot
#[test]
fn row55_haraka_S_oneshot() {
    if skip() {
        return;
    }
    let p = libs();
    let cf = f!(p.c, "SPX_haraka_S", FHarakaS);
    let rf = f!(p.rust, "SPX_haraka_S", FHarakaS);
    let mut rng = Rng::new(SEED ^ 55);

    for &outlen in &[1usize, 16, 31, 32, 33, 64, 100] {
        for &inlen in &[0usize, 1, 31, 32, 33, 64, 100] {
            let (ca, ra) = tweaked_pair(&mut rng);
            let data = rng.bytes(inlen.max(1));
            let mut o1 = vec![0x66u8; outlen + 8];
            let mut o2 = o1.clone();
            unsafe {
                cf(
                    o1.as_mut_ptr(),
                    outlen as u64,
                    data.as_ptr(),
                    inlen as u64,
                    ca.as_ptr(),
                );
                rf(
                    o2.as_mut_ptr(),
                    outlen as u64,
                    data.as_ptr(),
                    inlen as u64,
                    ra.as_ptr(),
                );
            }
            eq_bytes(
                &format!("haraka_S(outlen={outlen},inlen={inlen})"),
                &o1,
                &o2,
            );
        }
    }
}
