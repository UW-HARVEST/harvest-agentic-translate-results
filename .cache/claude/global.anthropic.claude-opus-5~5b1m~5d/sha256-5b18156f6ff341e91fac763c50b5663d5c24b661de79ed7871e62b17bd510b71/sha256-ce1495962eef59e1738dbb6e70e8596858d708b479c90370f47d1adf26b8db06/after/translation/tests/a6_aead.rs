//! Area 6 — `crypto_aead/`: aegis128l, aegis256, chacha20poly1305,
//! chacha20poly1305_ietf, xchacha20poly1305_ietf.
//!
//! Covers `phaseA/configs_6.md` rows 6.1–6.26, 6.40–6.77 and
//! `phaseA/errors_6.md` rows 6.1–6.29a, 6.40–6.49, 6.82, 6.83
//! (the aes256gcm stub family lives in `a6_aes256gcm.rs`).
#![allow(clippy::too_many_arguments)]

mod common;
use common::*;
use libloading::Symbol;
use std::ffi::c_int;
use std::ptr::{null, null_mut};

// ---------------------------------------------------------------- signatures

type Getter = unsafe extern "C" fn() -> usize;
type Keygen = unsafe extern "C" fn(*mut u8);
type Enc = unsafe extern "C" fn(
    *mut u8,   // c
    *mut u64,  // clen_p
    *const u8, // m
    u64,       // mlen
    *const u8, // ad
    u64,       // adlen
    *const u8, // nsec
    *const u8, // npub
    *const u8, // k
) -> c_int;
type Dec = unsafe extern "C" fn(
    *mut u8,   // m
    *mut u64,  // mlen_p
    *mut u8,   // nsec
    *const u8, // c
    u64,       // clen
    *const u8, // ad
    u64,       // adlen
    *const u8, // npub
    *const u8, // k
) -> c_int;
type EncD = unsafe extern "C" fn(
    *mut u8,   // c
    *mut u8,   // mac
    *mut u64,  // maclen_p
    *const u8, // m
    u64,       // mlen
    *const u8, // ad
    u64,       // adlen
    *const u8, // nsec
    *const u8, // npub
    *const u8, // k
) -> c_int;
type DecD = unsafe extern "C" fn(
    *mut u8,   // m
    *mut u8,   // nsec
    *const u8, // c
    u64,       // clen
    *const u8, // mac
    *const u8, // ad
    u64,       // adlen
    *const u8, // npub
    *const u8, // k
) -> c_int;

// ---------------------------------------------------------------- descriptors

struct Fam {
    name: &'static str,
    kb: usize,
    npb: usize,
    ab: usize,
    mbm: usize,
    /// `true` when `_decrypt_detached` has the `clen/adlen > MESSAGEBYTES_MAX`
    /// early `return -1` guard (aegis only).
    guarded_decd: bool,
    enc: (Symbol<'static, Enc>, Symbol<'static, Enc>),
    dec: (Symbol<'static, Dec>, Symbol<'static, Dec>),
    encd: (Symbol<'static, EncD>, Symbol<'static, EncD>),
    decd: (Symbol<'static, DecD>, Symbol<'static, DecD>),
    keygen: (Symbol<'static, Keygen>, Symbol<'static, Keygen>),
}

fn fam(name: &'static str, kb: usize, npb: usize, ab: usize, mbm: usize, guarded: bool) -> Fam {
    Fam {
        name,
        kb,
        npb,
        ab,
        mbm,
        guarded_decd: guarded,
        enc: both::<Enc>(&format!("{name}_encrypt")),
        dec: both::<Dec>(&format!("{name}_decrypt")),
        encd: both::<EncD>(&format!("{name}_encrypt_detached")),
        decd: both::<DecD>(&format!("{name}_decrypt_detached")),
        keygen: both::<Keygen>(&format!("{name}_keygen")),
    }
}

const AEGIS_MBM: usize = (1usize << 61) - 1;
const CHACHA_IETF_MBM: usize = 64 * ((1usize << 32) - 1);

fn families() -> Vec<Fam> {
    vec![
        fam("crypto_aead_aegis128l", 16, 16, 32, AEGIS_MBM, true),
        fam("crypto_aead_aegis256", 32, 32, 32, AEGIS_MBM, true),
        fam("crypto_aead_chacha20poly1305", 32, 8, 16, usize::MAX - 16, false),
        fam(
            "crypto_aead_chacha20poly1305_ietf",
            32,
            12,
            16,
            CHACHA_IETF_MBM,
            false,
        ),
        fam(
            "crypto_aead_xchacha20poly1305_ietf",
            32,
            24,
            16,
            usize::MAX - 16,
            false,
        ),
    ]
}

// ---------------------------------------------------------------- sweeps

/// mandatory short-message shape sweep
const MLEN: [usize; 14] = [0, 1, 15, 16, 17, 31, 32, 33, 63, 64, 65, 127, 128, 129];
/// `None` = `ad == NULL && adlen == 0`; `Some(n)` = non-NULL `ad` with `adlen == n`
const ADLEN: [Option<usize>; 9] = [
    None,
    Some(0),
    Some(1),
    Some(15),
    Some(16),
    Some(17),
    Some(31),
    Some(32),
    Some(33),
];

const BIG_AEGIS128L: [usize; 9] = [224, 255, 256, 257, 511, 512, 513, 1024, 4096];
const BIG_AEGIS256: [usize; 9] = [112, 127, 128, 129, 255, 256, 257, 1024, 4096];
const BIG_CHACHA: [usize; 7] = [4096, 65536, 131071, 131072, 131073, 262144, 262145];

fn big_for(f: &Fam) -> Vec<usize> {
    match f.name {
        "crypto_aead_aegis128l" => BIG_AEGIS128L.to_vec(),
        "crypto_aead_aegis256" => BIG_AEGIS256.to_vec(),
        _ => BIG_CHACHA.to_vec(),
    }
}

fn big_adlens(f: &Fam) -> Vec<usize> {
    match f.name {
        "crypto_aead_aegis128l" => vec![0, 64, 65, 128],
        "crypto_aead_aegis256" => vec![0, 32, 33, 64],
        "crypto_aead_chacha20poly1305_ietf" => vec![0, 15, 16, 17],
        _ => vec![0, 16, 17],
    }
}

// ---------------------------------------------------------------- helpers

const POISON: u64 = 0xDEAD_BEEF_CAFE_1234;

/// `padded()` with the payload region filled with a recognisable byte so that
/// "left untouched" and "zeroed" are distinguishable.
fn poisoned(len: usize) -> Vec<u8> {
    let mut v = padded(len);
    for b in v[..len].iter_mut() {
        *b = 0xDD;
    }
    v
}

fn ptr_len(buf: &[u8], case: Option<usize>) -> (*const u8, u64) {
    match case {
        None => (null(), 0),
        Some(n) => {
            assert!(buf.len() >= n);
            (buf.as_ptr(), n as u64)
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Nsec {
    Null,
    Poisoned,
}

struct Ctx {
    k: Vec<u8>,
    npub: Vec<u8>,
    m: Vec<u8>,
    ad: Vec<u8>,
}

impl Ctx {
    fn new(rng: &mut Rng, f: &Fam, mlen: usize, adlen: usize) -> Ctx {
        Ctx {
            k: rng.bytes(f.kb),
            npub: rng.bytes(f.npb),
            m: rng.bytes(mlen + 1),
            ad: rng.bytes(adlen + 1),
        }
    }
}

// ------------------------------------------------------------ combined API

/// Runs `*_encrypt` on both libraries, asserts full agreement and returns the
/// (identical) ciphertext.
fn enc_combined(
    f: &Fam,
    cx: &Ctx,
    mlen: usize,
    adc: Option<usize>,
    with_clen: bool,
    nsec: bool,
    label: &str,
) -> Vec<u8> {
    let ab = f.ab;
    let (adp, adl) = ptr_len(&cx.ad, adc);
    let nsec_buf = [0x5Au8];
    let nsec_p = if nsec {
        nsec_buf.as_ptr()
    } else {
        null::<u8>()
    };
    let mut cc = padded(mlen + ab);
    let mut cr = padded(mlen + ab);
    let mut lc = POISON;
    let mut lr = POISON;
    let (pc, pr) = if with_clen {
        (&mut lc as *mut u64, &mut lr as *mut u64)
    } else {
        (null_mut(), null_mut())
    };
    let rc = unsafe {
        (f.enc.0)(
            cc.as_mut_ptr(),
            pc,
            cx.m.as_ptr(),
            mlen as u64,
            adp,
            adl,
            nsec_p,
            cx.npub.as_ptr(),
            cx.k.as_ptr(),
        )
    };
    let rr = unsafe {
        (f.enc.1)(
            cr.as_mut_ptr(),
            pr,
            cx.m.as_ptr(),
            mlen as u64,
            adp,
            adl,
            nsec_p,
            cx.npub.as_ptr(),
            cx.k.as_ptr(),
        )
    };
    eqi(&format!("{label}: encrypt ret"), rc, rr);
    assert_eq!(rc, 0, "{label}: C encrypt should succeed");
    eqb(&format!("{label}: encrypt c"), &cc, &cr);
    check_pad(&format!("{label}: encrypt c (C)"), &cc, mlen + ab);
    check_pad(&format!("{label}: encrypt c (Rust)"), &cr, mlen + ab);
    if with_clen {
        assert_eq!(lc, (mlen + ab) as u64, "{label}: C *clen_p");
        assert_eq!(lr, lc, "{label}: *clen_p mismatch (C {lc}, Rust {lr})");
    } else {
        assert_eq!(lc, POISON);
        assert_eq!(lr, POISON);
    }
    cc.truncate(mlen + ab);
    cc
}

/// Runs `*_decrypt` on both libraries against `c`, asserting the expected
/// return code, the `*mlen_p` value, and the documented buffer state.
fn dec_combined(
    f: &Fam,
    cx: &Ctx,
    c: &[u8],
    adc: Option<usize>,
    with_mlen: bool,
    nsec: Nsec,
    expect: c_int,
    label: &str,
) -> Vec<u8> {
    let ab = f.ab;
    let clen = c.len();
    let (adp, adl) = ptr_len(&cx.ad, adc);
    let mcap = if clen >= ab { clen - ab } else { 64 };
    let mut mc = poisoned(mcap);
    let mut mr = poisoned(mcap);
    let mut nc = poisoned(8);
    let mut nr = poisoned(8);
    let (npc, npr) = match nsec {
        Nsec::Null => (null_mut(), null_mut()),
        Nsec::Poisoned => (nc.as_mut_ptr(), nr.as_mut_ptr()),
    };
    let mut lc = POISON;
    let mut lr = POISON;
    let (pc, pr) = if with_mlen {
        (&mut lc as *mut u64, &mut lr as *mut u64)
    } else {
        (null_mut(), null_mut())
    };
    let rc = unsafe {
        (f.dec.0)(
            mc.as_mut_ptr(),
            pc,
            npc,
            c.as_ptr(),
            clen as u64,
            adp,
            adl,
            cx.npub.as_ptr(),
            cx.k.as_ptr(),
        )
    };
    let rr = unsafe {
        (f.dec.1)(
            mr.as_mut_ptr(),
            pr,
            npr,
            c.as_ptr(),
            clen as u64,
            adp,
            adl,
            cx.npub.as_ptr(),
            cx.k.as_ptr(),
        )
    };
    eqi(&format!("{label}: decrypt ret"), rc, rr);
    assert_eq!(rc, expect, "{label}: C decrypt return");
    eqb(&format!("{label}: decrypt m"), &mc, &mr);
    check_pad(&format!("{label}: decrypt m (C)"), &mc, mcap);
    check_pad(&format!("{label}: decrypt m (Rust)"), &mr, mcap);
    if with_mlen {
        let want = if rc == 0 { (clen - ab) as u64 } else { 0 };
        assert_eq!(lc, want, "{label}: C *mlen_p");
        assert_eq!(lr, lc, "{label}: *mlen_p mismatch (C {lc}, Rust {lr})");
    } else {
        assert_eq!(lc, POISON);
        assert_eq!(lr, POISON);
    }
    if nsec == Nsec::Poisoned {
        // NSECBYTES == 0: the buffer is never written, even on success.
        eqb(&format!("{label}: nsec buffer"), &nc, &nr);
        assert!(
            nc[..8].iter().all(|b| *b == 0xDD),
            "{label}: nsec out-param was written by C"
        );
    }
    if rc != 0 && clen >= ab {
        // AEAD `_decrypt_detached` memsets `m` on MAC failure.
        assert!(
            mc[..mcap].iter().all(|b| *b == 0),
            "{label}: m not zeroed on MAC failure (C)"
        );
    }
    if rc != 0 && clen < ab {
        assert!(
            mc[..mcap].iter().all(|b| *b == 0xDD),
            "{label}: m touched although clen < ABYTES (C)"
        );
    }
    mc.truncate(mcap);
    mc
}

// ------------------------------------------------------------ detached API

fn enc_detached(
    f: &Fam,
    cx: &Ctx,
    mlen: usize,
    adc: Option<usize>,
    with_maclen: bool,
    nsec: bool,
    label: &str,
) -> (Vec<u8>, Vec<u8>) {
    let ab = f.ab;
    let (adp, adl) = ptr_len(&cx.ad, adc);
    let nsec_buf = [0x5Au8];
    let nsec_p = if nsec {
        nsec_buf.as_ptr()
    } else {
        null::<u8>()
    };
    let mut cc = padded(mlen);
    let mut cr = padded(mlen);
    let mut mac_c = padded(ab);
    let mut mac_r = padded(ab);
    let mut lc = POISON;
    let mut lr = POISON;
    let (pc, pr) = if with_maclen {
        (&mut lc as *mut u64, &mut lr as *mut u64)
    } else {
        (null_mut(), null_mut())
    };
    let rc = unsafe {
        (f.encd.0)(
            cc.as_mut_ptr(),
            mac_c.as_mut_ptr(),
            pc,
            cx.m.as_ptr(),
            mlen as u64,
            adp,
            adl,
            nsec_p,
            cx.npub.as_ptr(),
            cx.k.as_ptr(),
        )
    };
    let rr = unsafe {
        (f.encd.1)(
            cr.as_mut_ptr(),
            mac_r.as_mut_ptr(),
            pr,
            cx.m.as_ptr(),
            mlen as u64,
            adp,
            adl,
            nsec_p,
            cx.npub.as_ptr(),
            cx.k.as_ptr(),
        )
    };
    eqi(&format!("{label}: encrypt_detached ret"), rc, rr);
    // Row 6.45: none of the six families rejects anything here.
    assert_eq!(rc, 0, "{label}: C encrypt_detached should return 0");
    eqb(&format!("{label}: encrypt_detached c"), &cc, &cr);
    eqb(&format!("{label}: encrypt_detached mac"), &mac_c, &mac_r);
    check_pad(&format!("{label}: ed c (C)"), &cc, mlen);
    check_pad(&format!("{label}: ed c (Rust)"), &cr, mlen);
    check_pad(&format!("{label}: ed mac (C)"), &mac_c, ab);
    check_pad(&format!("{label}: ed mac (Rust)"), &mac_r, ab);
    if with_maclen {
        assert_eq!(lc, ab as u64, "{label}: C *maclen_p");
        assert_eq!(lr, lc, "{label}: *maclen_p mismatch");
    } else {
        assert_eq!(lc, POISON);
        assert_eq!(lr, POISON);
    }
    cc.truncate(mlen);
    mac_c.truncate(ab);
    (cc, mac_c)
}

/// `m == NULL` when `verify_only`.
fn dec_detached(
    f: &Fam,
    cx: &Ctx,
    c: &[u8],
    mac: &[u8],
    adc: Option<usize>,
    verify_only: bool,
    nsec: Nsec,
    expect: c_int,
    label: &str,
) -> Vec<u8> {
    let (adp, adl) = ptr_len(&cx.ad, adc);
    let clen = c.len();
    let mut mc = poisoned(clen);
    let mut mr = poisoned(clen);
    let (mpc, mpr) = if verify_only {
        (null_mut(), null_mut())
    } else {
        (mc.as_mut_ptr(), mr.as_mut_ptr())
    };
    let mut nc = poisoned(8);
    let mut nr = poisoned(8);
    let (npc, npr) = match nsec {
        Nsec::Null => (null_mut(), null_mut()),
        Nsec::Poisoned => (nc.as_mut_ptr(), nr.as_mut_ptr()),
    };
    let rc = unsafe {
        (f.decd.0)(
            mpc,
            npc,
            c.as_ptr(),
            clen as u64,
            mac.as_ptr(),
            adp,
            adl,
            cx.npub.as_ptr(),
            cx.k.as_ptr(),
        )
    };
    let rr = unsafe {
        (f.decd.1)(
            mpr,
            npr,
            c.as_ptr(),
            clen as u64,
            mac.as_ptr(),
            adp,
            adl,
            cx.npub.as_ptr(),
            cx.k.as_ptr(),
        )
    };
    eqi(&format!("{label}: decrypt_detached ret"), rc, rr);
    assert_eq!(rc, expect, "{label}: C decrypt_detached return");
    eqb(&format!("{label}: decrypt_detached m"), &mc, &mr);
    check_pad(&format!("{label}: dd m (C)"), &mc, clen);
    check_pad(&format!("{label}: dd m (Rust)"), &mr, clen);
    if verify_only {
        // "nothing written anywhere" (rows 6.9/6.19/6.25/6.29a/6.44)
        assert!(
            mc[..clen].iter().all(|b| *b == 0xDD),
            "{label}: m written although m == NULL"
        );
    } else if rc != 0 {
        assert!(
            mc[..clen].iter().all(|b| *b == 0),
            "{label}: m not zeroed on MAC failure"
        );
    }
    if nsec == Nsec::Poisoned {
        eqb(&format!("{label}: nsec buffer"), &nc, &nr);
        assert!(nc[..8].iter().all(|b| *b == 0xDD));
    }
    mc.truncate(clen);
    mc
}

// ================================================================= 6.1 / 6.14
// ================================================== 6.40 / 6.52 / 6.65 / 6.83

#[test]
fn constant_getters() {
    for f in families() {
        let g = |suffix: &str| -> (usize, usize) {
            let (c, r) = both::<Getter>(&format!("{}{}", f.name, suffix));
            unsafe { (c(), r()) }
        };
        for (suffix, want) in [
            ("_keybytes", f.kb),
            ("_nsecbytes", 0usize),
            ("_npubbytes", f.npb),
            ("_abytes", f.ab),
            ("_messagebytes_max", f.mbm),
        ] {
            let (cv, rv) = g(suffix);
            assert_eq!(cv, want, "{}{}: C value", f.name, suffix);
            assert_eq!(rv, cv, "{}{}: Rust value differs", f.name, suffix);
        }
    }
}

// ================================================ 6.2 / 6.15 / 6.41 / 6.53 / 6.66

#[test]
fn keygen_fills_and_varies() {
    for f in families() {
        for seed in [1u64, 2, 0x1234_5678] {
            rng_reseed(seed);
            let mut a = padded(f.kb);
            let mut b = padded(f.kb);
            unsafe {
                (f.keygen.0)(a.as_mut_ptr());
                (f.keygen.1)(b.as_mut_ptr());
            }
            eqb(&format!("{}_keygen", f.name), &a, &b);
            check_pad(&format!("{}_keygen (C)", f.name), &a, f.kb);
            check_pad(&format!("{}_keygen (Rust)", f.name), &b, f.kb);
            // fully written: with the difftest RNG an all-zero key is
            // vanishingly unlikely
            assert!(a[..f.kb].iter().any(|x| *x != 0), "{}: keygen all zero", f.name);
            // two successive calls differ
            let mut c2 = padded(f.kb);
            let mut r2 = padded(f.kb);
            unsafe {
                (f.keygen.0)(c2.as_mut_ptr());
                (f.keygen.1)(r2.as_mut_ptr());
            }
            eqb(&format!("{}_keygen 2nd", f.name), &c2, &r2);
            assert_ne!(
                &a[..f.kb],
                &c2[..f.kb],
                "{}: two keygen calls identical",
                f.name
            );
        }
    }
}

// =============================== 6.3 / 6.4 / 6.16 / 6.17 / 6.42 / 6.43
// =============================== 6.54 / 6.55 / 6.67 / 6.68

#[test]
fn combined_mlen_sweep() {
    let mut rng = Rng::new(0x6003);
    for f in families() {
        for with_ptrs in [true, false] {
            for &mlen in MLEN.iter() {
                for rep in 0..4 {
                    let cx = Ctx::new(&mut rng, &f, mlen, 0);
                    let label = format!(
                        "{} combined mlen={mlen} ptrs={with_ptrs} rep={rep}",
                        f.name
                    );
                    let c = enc_combined(&f, &cx, mlen, None, with_ptrs, false, &label);
                    assert_eq!(c.len(), mlen + f.ab);
                    let m = dec_combined(&f, &cx, &c, None, with_ptrs, Nsec::Null, 0, &label);
                    eqb(&format!("{label}: recovered plaintext"), &cx.m[..mlen], &m);
                }
            }
        }
    }
}

// =============================== 6.5 / 6.18 / 6.44 / 6.56 / 6.69

#[test]
fn combined_ad_sweep() {
    let mut rng = Rng::new(0x6005);
    for f in families() {
        let mlens: Vec<usize> = match f.name {
            "crypto_aead_aegis128l" => vec![0, 1, 32, 33, 64],
            "crypto_aead_aegis256" => vec![0, 1, 16, 17, 32],
            "crypto_aead_chacha20poly1305" => vec![0, 1, 16, 63, 64, 65],
            _ => MLEN.to_vec(),
        };
        for &mlen in mlens.iter() {
            for &adc in ADLEN.iter() {
                let adlen = adc.unwrap_or(0);
                let cx = Ctx::new(&mut rng, &f, mlen, adlen);
                let label = format!("{} ad mlen={mlen} adc={adc:?}", f.name);
                let c = enc_combined(&f, &cx, mlen, adc, true, false, &label);
                let m = dec_combined(&f, &cx, &c, adc, true, Nsec::Null, 0, &label);
                eqb(&format!("{label}: plaintext"), &cx.m[..mlen], &m);
                // `ad` must be treated identically by both sides: a differing
                // `ad` on the open side must fail on both.
                if adlen > 0 {
                    let mut cx2 = Ctx {
                        k: cx.k.clone(),
                        npub: cx.npub.clone(),
                        m: cx.m.clone(),
                        ad: cx.ad.clone(),
                    };
                    cx2.ad[0] ^= 0x80;
                    dec_combined(&f, &cx2, &c, adc, true, Nsec::Null, -1, &label);
                }
            }
        }
    }
}

// =============================== 6.6 / 6.19 / 6.45 / 6.57 / 6.70

#[test]
fn combined_big_messages() {
    let mut rng = Rng::new(0x6006);
    for f in families() {
        for &mlen in big_for(&f).iter() {
            for &adlen in big_adlens(&f).iter() {
                let cx = Ctx::new(&mut rng, &f, mlen, adlen);
                let label = format!("{} big mlen={mlen} adlen={adlen}", f.name);
                let c = enc_combined(&f, &cx, mlen, Some(adlen), true, false, &label);
                let m = dec_combined(&f, &cx, &c, Some(adlen), true, Nsec::Null, 0, &label);
                eqb(&format!("{label}: plaintext"), &cx.m[..mlen], &m);
                // detached must agree with the combined framing
                let (cd, mac) = enc_detached(&f, &cx, mlen, Some(adlen), true, false, &label);
                eqb(&format!("{label}: detached c"), &c[..mlen], &cd);
                eqb(&format!("{label}: detached mac"), &c[mlen..], &mac);
            }
        }
    }
}

/// "message lengths sweeping 0..300 plus a few multi-KiB"
#[test]
fn combined_dense_length_sweep() {
    let mut rng = Rng::new(0x600D);
    let extra = [1024usize, 2047, 2048, 4096, 8191];
    for f in families() {
        let lens: Vec<usize> = (0..=300).chain(extra.iter().copied()).collect();
        for mlen in lens {
            let adlen = rng.below(40);
            let adc = if rng.byte() & 1 == 0 { None } else { Some(adlen) };
            let cx = Ctx::new(&mut rng, &f, mlen, adlen);
            let label = format!("{} dense mlen={mlen}", f.name);
            let c = enc_combined(&f, &cx, mlen, adc, true, false, &label);
            let m = dec_combined(&f, &cx, &c, adc, true, Nsec::Null, 0, &label);
            eqb(&format!("{label}: plaintext"), &cx.m[..mlen], &m);
        }
    }
}

// =============================== 6.7 / 6.8 / 6.20 / 6.21 / 6.46 / 6.47
// =============================== 6.58 / 6.59 / 6.71 / 6.72 / 6.46(errors)

#[test]
fn detached_sweep() {
    let mut rng = Rng::new(0x6007);
    for f in families() {
        for &mlen in MLEN.iter() {
            for &adc in ADLEN.iter() {
                let adlen = adc.unwrap_or(0);
                let cx = Ctx::new(&mut rng, &f, mlen, adlen);
                let label = format!("{} detached mlen={mlen} adc={adc:?}", f.name);
                let (c1, mac1) = enc_detached(&f, &cx, mlen, adc, true, false, &label);
                // maclen_p == NULL must not change the output (row 6.46 errors)
                let (c2, mac2) = enc_detached(&f, &cx, mlen, adc, false, false, &label);
                eqb(&format!("{label}: maclen NULL c"), &c1, &c2);
                eqb(&format!("{label}: maclen NULL mac"), &mac1, &mac2);
                // nsec != NULL must not change the output (row 6.48 errors)
                let (c3, mac3) = enc_detached(&f, &cx, mlen, adc, true, true, &label);
                eqb(&format!("{label}: nsec!=NULL c"), &c1, &c3);
                eqb(&format!("{label}: nsec!=NULL mac"), &mac1, &mac3);
                // combined API must equal detached output split at mlen
                let comb = enc_combined(&f, &cx, mlen, adc, true, false, &label);
                eqb(&format!("{label}: combined vs detached c"), &comb[..mlen], &c1);
                eqb(
                    &format!("{label}: combined vs detached mac"),
                    &comb[mlen..],
                    &mac1,
                );
                // round trip through the detached open
                let m = dec_detached(&f, &cx, &c1, &mac1, adc, false, Nsec::Null, 0, &label);
                eqb(&format!("{label}: plaintext"), &cx.m[..mlen], &m);
            }
        }
    }
}

// =============================== 6.9 / 6.22 / 6.48 / 6.60 / 6.73

#[test]
fn decrypt_detached_verify_only() {
    let mut rng = Rng::new(0x6009);
    for f in families() {
        for &mlen in MLEN.iter() {
            for &adc in [None, Some(0usize), Some(17usize)].iter() {
                let adlen = adc.unwrap_or(0);
                let cx = Ctx::new(&mut rng, &f, mlen, adlen);
                let label = format!("{} verify-only mlen={mlen} adc={adc:?}", f.name);
                let (c, mac) = enc_detached(&f, &cx, mlen, adc, true, false, &label);
                dec_detached(&f, &cx, &c, &mac, adc, true, Nsec::Null, 0, &label);
                // tampered mac -> -1 and *nothing* written
                let mut bad = mac.clone();
                bad[0] ^= 0x01;
                dec_detached(&f, &cx, &c, &bad, adc, true, Nsec::Null, -1, &label);
                // tampered ciphertext -> -1
                if mlen > 0 {
                    let mut badc = c.clone();
                    badc[mlen - 1] ^= 0x40;
                    dec_detached(&f, &cx, &badc, &mac, adc, true, Nsec::Null, -1, &label);
                }
                // wrong key / wrong nonce
                let mut cx2 = Ctx {
                    k: cx.k.clone(),
                    npub: cx.npub.clone(),
                    m: cx.m.clone(),
                    ad: cx.ad.clone(),
                };
                cx2.k[0] ^= 0xff;
                dec_detached(&f, &cx2, &c, &mac, adc, true, Nsec::Null, -1, &label);
                let mut cx3 = Ctx {
                    k: cx.k.clone(),
                    npub: cx.npub.clone(),
                    m: cx.m.clone(),
                    ad: cx.ad.clone(),
                };
                cx3.npub[0] ^= 0xff;
                dec_detached(&f, &cx3, &c, &mac, adc, true, Nsec::Null, -1, &label);
            }
        }
    }
}

// =============================== 6.10 / 6.23 / 6.49 / 6.61 / 6.74 (+ errors 6.48/6.49)

#[test]
fn nsec_is_ignored() {
    let mut rng = Rng::new(0x6010);
    for f in families() {
        for &mlen in [0usize, 1, 17, 64, 65].iter() {
            let cx = Ctx::new(&mut rng, &f, mlen, 8);
            let label = format!("{} nsec mlen={mlen}", f.name);
            let base = enc_combined(&f, &cx, mlen, Some(8), true, false, &label);
            let with = enc_combined(&f, &cx, mlen, Some(8), true, true, &label);
            eqb(&format!("{label}: nsec!=NULL encrypt"), &base, &with);
            for nsec in [Nsec::Null, Nsec::Poisoned] {
                let m = dec_combined(&f, &cx, &base, Some(8), true, nsec, 0, &label);
                eqb(&format!("{label}: plaintext"), &cx.m[..mlen], &m);
            }
            let (c, mac) = enc_detached(&f, &cx, mlen, Some(8), true, false, &label);
            for nsec in [Nsec::Null, Nsec::Poisoned] {
                let m = dec_detached(&f, &cx, &c, &mac, Some(8), false, nsec, 0, &label);
                eqb(&format!("{label}: detached plaintext"), &cx.m[..mlen], &m);
            }
        }
    }
}

// =============================== 6.11 / 6.24 / 6.50 / 6.62 / 6.75

#[test]
fn in_place_aliasing() {
    let mut rng = Rng::new(0x6011);
    for f in families() {
        let mlens: Vec<usize> = match f.name {
            "crypto_aead_aegis128l" => vec![0, 1, 32, 33, 64, 1024],
            "crypto_aead_aegis256" => vec![0, 1, 16, 17, 32, 1024],
            _ => vec![0, 1, 64, 65, 131073],
        };
        for &mlen in mlens.iter() {
            let cx = Ctx::new(&mut rng, &f, mlen, 13);
            let label = format!("{} in-place mlen={mlen}", f.name);
            let reference = enc_combined(&f, &cx, mlen, Some(13), true, false, &label);
            let (adp, adl) = ptr_len(&cx.ad, Some(13));

            // encrypt with c == m
            let mut bc = padded(mlen + f.ab);
            let mut br = padded(mlen + f.ab);
            bc[..mlen].copy_from_slice(&cx.m[..mlen]);
            br[..mlen].copy_from_slice(&cx.m[..mlen]);
            let mut lc = POISON;
            let mut lr = POISON;
            let rc = unsafe {
                (f.enc.0)(
                    bc.as_mut_ptr(),
                    &mut lc,
                    bc.as_ptr(),
                    mlen as u64,
                    adp,
                    adl,
                    null(),
                    cx.npub.as_ptr(),
                    cx.k.as_ptr(),
                )
            };
            let rr = unsafe {
                (f.enc.1)(
                    br.as_mut_ptr(),
                    &mut lr,
                    br.as_ptr(),
                    mlen as u64,
                    adp,
                    adl,
                    null(),
                    cx.npub.as_ptr(),
                    cx.k.as_ptr(),
                )
            };
            eqi(&format!("{label}: in-place encrypt ret"), rc, rr);
            eqb(&format!("{label}: in-place encrypt"), &bc, &br);
            assert_eq!(lc, lr);
            eqb(
                &format!("{label}: in-place == out-of-place"),
                &reference,
                &bc[..mlen + f.ab],
            );
            check_pad(&format!("{label}: in-place enc (C)"), &bc, mlen + f.ab);
            check_pad(&format!("{label}: in-place enc (Rust)"), &br, mlen + f.ab);

            // decrypt with m == c
            let clen = mlen + f.ab;
            let mut dc = padded(clen);
            let mut dr = padded(clen);
            dc[..clen].copy_from_slice(&reference);
            dr[..clen].copy_from_slice(&reference);
            let rc = unsafe {
                (f.dec.0)(
                    dc.as_mut_ptr(),
                    &mut lc,
                    null_mut(),
                    dc.as_ptr(),
                    clen as u64,
                    adp,
                    adl,
                    cx.npub.as_ptr(),
                    cx.k.as_ptr(),
                )
            };
            let rr = unsafe {
                (f.dec.1)(
                    dr.as_mut_ptr(),
                    &mut lr,
                    null_mut(),
                    dr.as_ptr(),
                    clen as u64,
                    adp,
                    adl,
                    cx.npub.as_ptr(),
                    cx.k.as_ptr(),
                )
            };
            eqi(&format!("{label}: in-place decrypt ret"), rc, rr);
            assert_eq!(rc, 0);
            eqb(&format!("{label}: in-place decrypt"), &dc, &dr);
            eqb(
                &format!("{label}: in-place decrypt plaintext"),
                &cx.m[..mlen],
                &dc[..mlen],
            );
            check_pad(&format!("{label}: in-place dec (C)"), &dc, clen);
            check_pad(&format!("{label}: in-place dec (Rust)"), &dr, clen);
        }
    }
}

// =============================== 6.12 / 6.25 (+ nonce/key corner cases)

#[test]
fn corner_keys_and_nonces() {
    let mut rng = Rng::new(0x6012);
    for f in families() {
        let mlens: Vec<usize> = match f.name {
            "crypto_aead_aegis128l" => vec![0, 32, 33],
            "crypto_aead_aegis256" => vec![0, 16, 17],
            _ => vec![0, 16, 17, 64],
        };
        let keys: Vec<Vec<u8>> = vec![vec![0u8; f.kb], vec![0xffu8; f.kb], rng.bytes(f.kb)];
        let nonces: Vec<Vec<u8>> = {
            let mut v = vec![vec![0u8; f.npb], vec![0xffu8; f.npb], rng.bytes(f.npb)];
            // a nonce whose high half only is non-zero (relevant for the
            // xchacha/hchacha split)
            let mut hi = vec![0u8; f.npb];
            for b in hi[f.npb / 2..].iter_mut() {
                *b = 0xA7;
            }
            v.push(hi);
            v
        };
        for k in keys.iter() {
            for npub in nonces.iter() {
                for &mlen in mlens.iter() {
                    for &adc in [None, Some(0usize), Some(16usize)].iter() {
                        let adlen = adc.unwrap_or(0);
                        let cx = Ctx {
                            k: k.clone(),
                            npub: npub.clone(),
                            m: rng.bytes(mlen + 1),
                            ad: rng.bytes(adlen + 1),
                        };
                        let label = format!(
                            "{} corner k[0]={:#02x} n[0]={:#02x} mlen={mlen}",
                            f.name, k[0], npub[0]
                        );
                        let c = enc_combined(&f, &cx, mlen, adc, true, false, &label);
                        let m = dec_combined(&f, &cx, &c, adc, true, Nsec::Null, 0, &label);
                        eqb(&format!("{label}: plaintext"), &cx.m[..mlen], &m);
                    }
                }
            }
        }
    }
}

// =============================== 6.13 / 6.26 / 6.51 / 6.63 / 6.76 (pinned inputs)

#[test]
fn pinned_deterministic_vectors() {
    for f in families() {
        // deterministic, fully-specified inputs; both libraries must agree
        // byte-for-byte on ciphertext and tag.
        for variant in 0..4u8 {
            let k: Vec<u8> = (0..f.kb).map(|i| (i as u8).wrapping_mul(7) ^ variant).collect();
            let npub: Vec<u8> = match variant {
                1 => vec![0u8; f.npb],
                2 => vec![0xffu8; f.npb],
                _ => (0..f.npb).map(|i| 0x40u8 + i as u8).collect(),
            };
            for &mlen in [0usize, 1, 16, 31, 32, 63, 64, 114, 129, 256].iter() {
                for &adlen in [0usize, 12, 16, 17].iter() {
                    let cx = Ctx {
                        k: k.clone(),
                        npub: npub.clone(),
                        m: (0..mlen + 1).map(|i| (i * 31 + 7) as u8).collect(),
                        ad: (0..adlen + 1).map(|i| (i * 17 + 3) as u8).collect(),
                    };
                    let label = format!("{} pinned v={variant} mlen={mlen} adlen={adlen}", f.name);
                    let c = enc_combined(&f, &cx, mlen, Some(adlen), true, false, &label);
                    let m = dec_combined(&f, &cx, &c, Some(adlen), true, Nsec::Null, 0, &label);
                    eqb(&format!("{label}: plaintext"), &cx.m[..mlen], &m);
                }
            }
        }
    }
}

/// RFC 8439 §2.8.2 AEAD_CHACHA20_POLY1305 test vector — pins the absolute
/// output of both libraries, not just their agreement.
#[test]
fn kat_rfc8439_chacha20poly1305_ietf() {
    let f = fam(
        "crypto_aead_chacha20poly1305_ietf",
        32,
        12,
        16,
        CHACHA_IETF_MBM,
        false,
    );
    let m = b"Ladies and Gentlemen of the class of '99: If I could offer you \
only one tip for the future, sunscreen would be it.";
    let ad: [u8; 12] = [
        0x50, 0x51, 0x52, 0x53, 0xc0, 0xc1, 0xc2, 0xc3, 0xc4, 0xc5, 0xc6, 0xc7,
    ];
    let k: Vec<u8> = (0x80u8..0xa0).collect();
    let npub: [u8; 12] = [
        0x07, 0x00, 0x00, 0x00, 0x40, 0x41, 0x42, 0x43, 0x44, 0x45, 0x46, 0x47,
    ];
    let want_tag: [u8; 16] = [
        0x1a, 0xe1, 0x0b, 0x59, 0x4f, 0x09, 0xe2, 0x6a, 0x7e, 0x90, 0x2e, 0xcb, 0xd0, 0x60, 0x06,
        0x91,
    ];
    let mut cc = padded(m.len() + 16);
    let mut cr = padded(m.len() + 16);
    let mut lc = 0u64;
    let mut lr = 0u64;
    unsafe {
        assert_eq!(
            (f.enc.0)(
                cc.as_mut_ptr(),
                &mut lc,
                m.as_ptr(),
                m.len() as u64,
                ad.as_ptr(),
                ad.len() as u64,
                null(),
                npub.as_ptr(),
                k.as_ptr()
            ),
            0
        );
        assert_eq!(
            (f.enc.1)(
                cr.as_mut_ptr(),
                &mut lr,
                m.as_ptr(),
                m.len() as u64,
                ad.as_ptr(),
                ad.len() as u64,
                null(),
                npub.as_ptr(),
                k.as_ptr()
            ),
            0
        );
    }
    assert_eq!(lc, (m.len() + 16) as u64);
    assert_eq!(lr, lc);
    eqb("rfc8439 ciphertext", &cc, &cr);
    eqb("rfc8439 tag (C)", &want_tag, &cc[m.len()..m.len() + 16]);
    eqb("rfc8439 tag (Rust)", &want_tag, &cr[m.len()..m.len() + 16]);
}

// =============================== 6.64

#[test]
fn ietf_and_original_chacha_differ() {
    let orig = fam(
        "crypto_aead_chacha20poly1305",
        32,
        8,
        16,
        usize::MAX - 16,
        false,
    );
    let ietf = fam(
        "crypto_aead_chacha20poly1305_ietf",
        32,
        12,
        16,
        CHACHA_IETF_MBM,
        false,
    );
    let mut rng = Rng::new(0x6064);
    for &mlen in [0usize, 1, 15, 16, 17, 64, 65].iter() {
        for &adlen in [0usize, 1, 16, 17].iter() {
            let k = rng.bytes(32);
            let n12 = rng.bytes(12);
            let m = rng.bytes(mlen + 1);
            let ad = rng.bytes(adlen + 1);
            let cx_o = Ctx {
                k: k.clone(),
                npub: n12[..8].to_vec(),
                m: m.clone(),
                ad: ad.clone(),
            };
            let cx_i = Ctx {
                k: k.clone(),
                npub: n12.clone(),
                m: m.clone(),
                ad: ad.clone(),
            };
            let label = format!("orig-vs-ietf mlen={mlen} adlen={adlen}");
            let co = enc_combined(&orig, &cx_o, mlen, Some(adlen), true, false, &label);
            let ci = enc_combined(&ietf, &cx_i, mlen, Some(adlen), true, false, &label);
            assert_ne!(co, ci, "{label}: the two families collapsed to one");
        }
    }
}

// =============================== 6.77 (xchacha == ietf under hchacha20)

#[test]
fn xchacha_equals_ietf_under_hchacha20() {
    type HChacha = unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8) -> c_int;
    let (hc, hr) = both::<HChacha>("crypto_core_hchacha20");
    let x = fam(
        "crypto_aead_xchacha20poly1305_ietf",
        32,
        24,
        16,
        usize::MAX - 16,
        false,
    );
    let ietf = fam(
        "crypto_aead_chacha20poly1305_ietf",
        32,
        12,
        16,
        CHACHA_IETF_MBM,
        false,
    );
    let mut rng = Rng::new(0x6077);
    for &mlen in [0usize, 1, 16, 17, 64, 65, 300].iter() {
        for &adlen in [0usize, 1, 16, 17].iter() {
            let k = rng.bytes(32);
            let npub = rng.bytes(24);
            let m = rng.bytes(mlen + 1);
            let ad = rng.bytes(adlen + 1);

            let mut k2c = [0u8; 32];
            let mut k2r = [0u8; 32];
            unsafe {
                hc(k2c.as_mut_ptr(), npub.as_ptr(), k.as_ptr(), null());
                hr(k2r.as_mut_ptr(), npub.as_ptr(), k.as_ptr(), null());
            }
            eqb("hchacha20 subkey", &k2c, &k2r);
            let mut npub2 = [0u8; 12];
            npub2[4..].copy_from_slice(&npub[16..24]);

            let label = format!("xchacha-wiring mlen={mlen} adlen={adlen}");
            let cx_x = Ctx {
                k: k.clone(),
                npub: npub.clone(),
                m: m.clone(),
                ad: ad.clone(),
            };
            let cx_i = Ctx {
                k: k2c.to_vec(),
                npub: npub2.to_vec(),
                m: m.clone(),
                ad: ad.clone(),
            };
            let cxv = enc_combined(&x, &cx_x, mlen, Some(adlen), true, false, &label);
            let civ = enc_combined(&ietf, &cx_i, mlen, Some(adlen), true, false, &label);
            eqb(&format!("{label}: xchacha vs ietf"), &cxv, &civ);
        }
    }
}

// =============================== cross-library ciphertext interop

#[test]
fn cross_library_ciphertext_interop() {
    let mut rng = Rng::new(0x60C0);
    for f in families() {
        for &mlen in [0usize, 1, 17, 32, 64, 129, 1024].iter() {
            let adlen = 17usize;
            let cx = Ctx::new(&mut rng, &f, mlen, adlen);
            let (adp, adl) = ptr_len(&cx.ad, Some(adlen));
            let ab = f.ab;
            let mut cbuf = vec![vec![0u8; mlen + ab], vec![0u8; mlen + ab]];
            unsafe {
                assert_eq!(
                    (f.enc.0)(
                        cbuf[0].as_mut_ptr(),
                        null_mut(),
                        cx.m.as_ptr(),
                        mlen as u64,
                        adp,
                        adl,
                        null(),
                        cx.npub.as_ptr(),
                        cx.k.as_ptr()
                    ),
                    0
                );
                assert_eq!(
                    (f.enc.1)(
                        cbuf[1].as_mut_ptr(),
                        null_mut(),
                        cx.m.as_ptr(),
                        mlen as u64,
                        adp,
                        adl,
                        null(),
                        cx.npub.as_ptr(),
                        cx.k.as_ptr()
                    ),
                    0
                );
            }
            eqb(&format!("{} interop ct", f.name), &cbuf[0], &cbuf[1]);
            // C ciphertext opened by Rust, Rust ciphertext opened by C
            for (prod, opener) in [(0usize, 1usize), (1, 0)] {
                let mut out = poisoned(mlen);
                let mut ml = POISON;
                let dec = if opener == 0 { &f.dec.0 } else { &f.dec.1 };
                let rc = unsafe {
                    dec(
                        out.as_mut_ptr(),
                        &mut ml,
                        null_mut(),
                        cbuf[prod].as_ptr(),
                        (mlen + ab) as u64,
                        adp,
                        adl,
                        cx.npub.as_ptr(),
                        cx.k.as_ptr(),
                    )
                };
                assert_eq!(
                    rc, 0,
                    "{}: ciphertext from lib {prod} rejected by lib {opener}",
                    f.name
                );
                assert_eq!(ml, mlen as u64);
                eqb(&format!("{} interop pt", f.name), &cx.m[..mlen], &out[..mlen]);
                check_pad("interop out", &out, mlen);
            }
        }
    }
}

// ============================================================ ERROR SURFACE
// errors_6.md 6.4/6.5, 6.14/6.15, 6.22/6.23, 6.27/6.28, 6.41/6.42

#[test]
fn decrypt_short_ciphertext() {
    let mut rng = Rng::new(0x6E04);
    for f in families() {
        let ab = f.ab;
        for clen in 0..=ab + 1 {
            for &with_ptr in [true, false].iter() {
                for &adc in [None, Some(0usize), Some(9usize)].iter() {
                    let adlen = adc.unwrap_or(0);
                    let cx = Ctx::new(&mut rng, &f, 0, adlen);
                    let c = rng.bytes(clen.max(1));
                    let label = format!("{} short clen={clen} ptr={with_ptr}", f.name);
                    // random data of length >= ABYTES will not verify
                    let expect = -1;
                    dec_combined(
                        &f,
                        &cx,
                        &c[..clen],
                        adc,
                        with_ptr,
                        Nsec::Null,
                        expect,
                        &label,
                    );
                }
            }
        }
        // clen == ABYTES with a *valid* empty-message box must succeed
        let cx = Ctx::new(&mut rng, &f, 0, 0);
        let label = format!("{} clen==ABYTES valid", f.name);
        let c = enc_combined(&f, &cx, 0, None, true, false, &label);
        assert_eq!(c.len(), ab);
        dec_combined(&f, &cx, &c, None, true, Nsec::Null, 0, &label);
        // ...and truncating it by one byte is rejected before any crypto
        dec_combined(&f, &cx, &c[..ab - 1], None, true, Nsec::Null, -1, &label);
    }
}

#[test]
fn tag_corrupted_in_every_byte_position() {
    let mut rng = Rng::new(0x6E05);
    for f in families() {
        for &mlen in [0usize, 1, 16, 32, 33].iter() {
            let cx = Ctx::new(&mut rng, &f, mlen, 11);
            let label = format!("{} tamper mlen={mlen}", f.name);
            let c = enc_combined(&f, &cx, mlen, Some(11), true, false, &label);
            for pos in 0..c.len() {
                for bit in [0x01u8, 0x80] {
                    let mut bad = c.clone();
                    bad[pos] ^= bit;
                    dec_combined(
                        &f,
                        &cx,
                        &bad,
                        Some(11),
                        true,
                        Nsec::Null,
                        -1,
                        &format!("{label} pos={pos} bit={bit:#02x}"),
                    );
                }
            }
            // detached: corrupt each mac byte
            let (cd, mac) = enc_detached(&f, &cx, mlen, Some(11), true, false, &label);
            for pos in 0..mac.len() {
                let mut bad = mac.clone();
                bad[pos] ^= 0xff;
                dec_detached(
                    &f,
                    &cx,
                    &cd,
                    &bad,
                    Some(11),
                    false,
                    Nsec::Null,
                    -1,
                    &format!("{label} mac pos={pos}"),
                );
            }
        }
    }
}

/// errors_6.md 6.6/6.7/6.16/6.17 — the aegis `_decrypt_detached` length guards
/// return `-1` instead of aborting.
#[test]
fn aegis_decrypt_detached_length_guards() {
    for f in families() {
        if !f.guarded_decd {
            continue;
        }
        let k = vec![0x11u8; f.kb];
        let npub = vec![0x22u8; f.npb];
        let mac = vec![0x33u8; f.ab];
        let cbuf = vec![0x44u8; 64];
        let ad = vec![0x55u8; 64];
        let huge = (f.mbm as u64) + 1;
        // clen > MESSAGEBYTES_MAX
        let rc = unsafe {
            (f.decd.0)(
                null_mut(),
                null_mut(),
                cbuf.as_ptr(),
                huge,
                mac.as_ptr(),
                ad.as_ptr(),
                0,
                npub.as_ptr(),
                k.as_ptr(),
            )
        };
        let rr = unsafe {
            (f.decd.1)(
                null_mut(),
                null_mut(),
                cbuf.as_ptr(),
                huge,
                mac.as_ptr(),
                ad.as_ptr(),
                0,
                npub.as_ptr(),
                k.as_ptr(),
            )
        };
        eqi(&format!("{}: clen > MESSAGEBYTES_MAX", f.name), rc, rr);
        assert_eq!(rc, -1);
        // adlen > MESSAGEBYTES_MAX
        let mut mc = poisoned(16);
        let mut mr = poisoned(16);
        let rc = unsafe {
            (f.decd.0)(
                mc.as_mut_ptr(),
                null_mut(),
                cbuf.as_ptr(),
                16,
                mac.as_ptr(),
                ad.as_ptr(),
                huge,
                npub.as_ptr(),
                k.as_ptr(),
            )
        };
        let rr = unsafe {
            (f.decd.1)(
                mr.as_mut_ptr(),
                null_mut(),
                cbuf.as_ptr(),
                16,
                mac.as_ptr(),
                ad.as_ptr(),
                huge,
                npub.as_ptr(),
                k.as_ptr(),
            )
        };
        eqi(&format!("{}: adlen > MESSAGEBYTES_MAX", f.name), rc, rr);
        assert_eq!(rc, -1);
        eqb(&format!("{}: m after guard", f.name), &mc, &mr);
        assert!(
            mc[..16].iter().all(|b| *b == 0xDD),
            "{}: m touched by the guarded path",
            f.name
        );
    }
}

/// errors_6.md 6.1/6.11/6.21/6.26/6.40 — `*_encrypt` with
/// `mlen > MESSAGEBYTES_MAX` reaches `sodium_misuse()` and aborts.
#[test]
fn encrypt_messagebytes_max_aborts() {
    for f in families() {
        let mbm = f.mbm as u64;
        let (ec, er) = (f.enc.0.clone(), f.enc.1.clone());
        let kb = f.kb;
        let npb = f.npb;
        eq_abort(
            &format!("{}_encrypt mlen > MESSAGEBYTES_MAX", f.name),
            move || unsafe {
                let k = vec![0u8; kb];
                let n = vec![0u8; npb];
                let mut out = [0u8; 64];
                ec(
                    out.as_mut_ptr(),
                    null_mut(),
                    out.as_ptr(),
                    mbm + 1,
                    null(),
                    0,
                    null(),
                    n.as_ptr(),
                    k.as_ptr(),
                );
            },
            move || unsafe {
                let k = vec![0u8; kb];
                let n = vec![0u8; npb];
                let mut out = [0u8; 64];
                er(
                    out.as_mut_ptr(),
                    null_mut(),
                    out.as_ptr(),
                    mbm + 1,
                    null(),
                    0,
                    null(),
                    n.as_ptr(),
                    k.as_ptr(),
                );
            },
        );
    }
}

/// errors_6.md 6.2/6.3/6.12/6.13 — only aegis `*_encrypt_detached` guards
/// `mlen`/`adlen`, and it aborts.
#[test]
fn aegis_encrypt_detached_aborts() {
    for f in families() {
        if !f.guarded_decd {
            continue; // rows 6.45: the chacha families have no guard at all
        }
        let mbm = f.mbm as u64;
        let kb = f.kb;
        let npb = f.npb;
        for (what, mlen, adlen) in [("mlen", mbm + 1, 0u64), ("adlen", 0u64, mbm + 1)] {
            let (ec, er) = (f.encd.0.clone(), f.encd.1.clone());
            eq_abort(
                &format!("{}_encrypt_detached {what} > MESSAGEBYTES_MAX", f.name),
                move || unsafe {
                    let k = vec![0u8; kb];
                    let n = vec![0u8; npb];
                    let mut out = [0u8; 64];
                    let mut mac = [0u8; 32];
                    let mut maclen = 0u64;
                    ec(
                        out.as_mut_ptr(),
                        mac.as_mut_ptr(),
                        &mut maclen,
                        out.as_ptr(),
                        mlen,
                        out.as_ptr(),
                        adlen,
                        null(),
                        n.as_ptr(),
                        k.as_ptr(),
                    );
                },
                move || unsafe {
                    let k = vec![0u8; kb];
                    let n = vec![0u8; npb];
                    let mut out = [0u8; 64];
                    let mut mac = [0u8; 32];
                    let mut maclen = 0u64;
                    er(
                        out.as_mut_ptr(),
                        mac.as_mut_ptr(),
                        &mut maclen,
                        out.as_ptr(),
                        mlen,
                        out.as_ptr(),
                        adlen,
                        null(),
                        n.as_ptr(),
                        k.as_ptr(),
                    );
                },
            );
        }
    }
}
