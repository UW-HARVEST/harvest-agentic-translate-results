//! Area 6 — `crypto_aead/aes256gcm`: the ENOSYS stub family.
//!
//! In this build no `HAVE_*` macro is defined, so `aead_aes256gcm.c` compiles
//! the stub block: `_is_available()` returns `0` and every operation sets
//! `errno = ENOSYS` and returns `-1` **without touching** `*clen_p` /
//! `*mlen_p` / `*maclen_p` (which is what makes it different from every other
//! AEAD in this area, all of which store a `0`).
//!
//! Covers `configs_6.md` rows 6.27–6.39 and `errors_6.md` rows 6.30–6.39.
#![allow(clippy::too_many_arguments)]

mod common;
use common::*;
use std::ffi::{c_int, c_void};
use std::ptr::{null, null_mut};

const ENOSYS: c_int = 38; // Linux

type Getter = unsafe extern "C" fn() -> usize;
type IsAvail = unsafe extern "C" fn() -> c_int;
type Keygen = unsafe extern "C" fn(*mut u8);
type Enc = unsafe extern "C" fn(
    *mut u8,
    *mut u64,
    *const u8,
    u64,
    *const u8,
    u64,
    *const u8,
    *const u8,
    *const c_void,
) -> c_int;
type EncD = unsafe extern "C" fn(
    *mut u8,
    *mut u8,
    *mut u64,
    *const u8,
    u64,
    *const u8,
    u64,
    *const u8,
    *const u8,
    *const c_void,
) -> c_int;
type Dec = unsafe extern "C" fn(
    *mut u8,
    *mut u64,
    *mut u8,
    *const u8,
    u64,
    *const u8,
    u64,
    *const u8,
    *const c_void,
) -> c_int;
type DecD = unsafe extern "C" fn(
    *mut u8,
    *mut u8,
    *const u8,
    u64,
    *const u8,
    *const u8,
    u64,
    *const u8,
    *const c_void,
) -> c_int;
type Beforenm = unsafe extern "C" fn(*mut c_void, *const u8) -> c_int;
type SodiumMalloc = unsafe extern "C" fn(usize) -> *mut c_void;
type SodiumFree = unsafe extern "C" fn(*mut c_void);

const POISON: u64 = 0xDEAD_BEEF_CAFE_1234;

const MLEN: [usize; 14] = [0, 1, 15, 16, 17, 31, 32, 33, 63, 64, 65, 127, 128, 129];
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
        Some(n) => (buf.as_ptr(), n as u64),
    }
}

/// 16-byte-aligned stack storage mirroring `CRYPTO_ALIGN(16)
/// crypto_aead_aes256gcm_state`.
#[repr(C, align(16))]
struct AlignedState([u8; 512]);

// ------------------------------------------------------------------ 6.27 / 6.30

#[test]
fn is_available_is_zero() {
    let (c, r) = both::<IsAvail>("crypto_aead_aes256gcm_is_available");
    unsafe {
        let cv = c();
        let rv = r();
        assert_eq!(cv, 0, "C crypto_aead_aes256gcm_is_available() must be 0");
        eqi("is_available", cv, rv);
    }
    // repeated calls stay 0 and never set errno
    for _ in 0..8 {
        set_errno(0);
        unsafe {
            assert_eq!(c(), 0);
            assert_eq!(r(), 0);
        }
        assert_eq!(errno(), 0, "is_available must not touch errno");
    }
}

// ------------------------------------------------------------------ 6.28 / 6.83

#[test]
fn constant_getters_still_work() {
    let want_mbm: usize = {
        let a = usize::MAX - 16;
        let b = (16u64 * ((1u64 << 32) - 2)) as usize;
        if a < b {
            a
        } else {
            b
        }
    };
    for (suffix, want) in [
        ("_keybytes", 32usize),
        ("_nsecbytes", 0),
        ("_npubbytes", 12),
        ("_abytes", 16),
        ("_messagebytes_max", want_mbm),
    ] {
        let (c, r) = both::<Getter>(&format!("crypto_aead_aes256gcm{suffix}"));
        unsafe {
            let cv = c();
            let rv = r();
            assert_eq!(cv, want, "C crypto_aead_aes256gcm{suffix}");
            assert_eq!(rv, cv, "Rust crypto_aead_aes256gcm{suffix}");
        }
    }
    let (c, r) = both::<Getter>("crypto_aead_aes256gcm_statebytes");
    unsafe {
        let cv = c();
        let rv = r();
        assert_eq!(cv, rv, "statebytes mismatch (C {cv}, Rust {rv})");
        assert_ne!(cv, 0, "statebytes must be non-zero");
        assert_eq!(cv % 16, 0, "statebytes must be a multiple of 16");
        // (sizeof(state) + 15) & ~15 with sizeof(state) == 512
        assert_eq!(cv, 512);
    }
}

// ------------------------------------------------------------------ 6.29 / 6.82

#[test]
fn keygen_still_works() {
    let (c, r) = both::<Keygen>("crypto_aead_aes256gcm_keygen");
    for seed in [7u64, 99, 0xABCD] {
        rng_reseed(seed);
        let mut a = padded(32);
        let mut b = padded(32);
        set_errno(0);
        unsafe {
            c(a.as_mut_ptr());
            r(b.as_mut_ptr());
        }
        eqb("aes256gcm_keygen", &a, &b);
        check_pad("aes256gcm_keygen (C)", &a, 32);
        check_pad("aes256gcm_keygen (Rust)", &b, 32);
        assert_eq!(errno(), 0, "keygen must not set errno");
        assert!(a[..32].iter().any(|x| *x != 0));
        let mut a2 = padded(32);
        let mut b2 = padded(32);
        unsafe {
            c(a2.as_mut_ptr());
            r(b2.as_mut_ptr());
        }
        eqb("aes256gcm_keygen 2nd", &a2, &b2);
        assert_ne!(&a[..32], &a2[..32], "two keygen calls identical");
    }
}

// ------------------------------------------------------------------ 6.30 / 6.35
// errors 6.31 / 6.36

fn check_encrypt(name: &str, key_or_state: *const c_void) {
    let (c, r) = both::<Enc>(name);
    let mut rng = Rng::new(0x6130);
    for &mlen in MLEN.iter() {
        for &adc in ADLEN.iter() {
            let adlen = adc.unwrap_or(0);
            let m = rng.bytes(mlen + 1);
            let ad = rng.bytes(adlen + 1);
            let npub = rng.bytes(12);
            let (adp, adl) = ptr_len(&ad, adc);
            for with_ptr in [true, false] {
                let mut cc = poisoned(mlen + 16);
                let mut cr = poisoned(mlen + 16);
                let mut lc = POISON;
                let mut lr = POISON;
                let (pc, pr) = if with_ptr {
                    (&mut lc as *mut u64, &mut lr as *mut u64)
                } else {
                    (null_mut(), null_mut())
                };
                set_errno(0);
                let rc = unsafe {
                    c(
                        cc.as_mut_ptr(),
                        pc,
                        m.as_ptr(),
                        mlen as u64,
                        adp,
                        adl,
                        null(),
                        npub.as_ptr(),
                        key_or_state,
                    )
                };
                let ec = errno();
                set_errno(0);
                let rr = unsafe {
                    r(
                        cr.as_mut_ptr(),
                        pr,
                        m.as_ptr(),
                        mlen as u64,
                        adp,
                        adl,
                        null(),
                        npub.as_ptr(),
                        key_or_state,
                    )
                };
                let er = errno();
                eqi(&format!("{name} mlen={mlen} adc={adc:?}"), rc, rr);
                assert_eq!(rc, -1, "{name}: must return -1");
                assert_eq!(ec, ENOSYS, "{name}: C errno");
                assert_eq!(er, ec, "{name}: errno mismatch (C {ec}, Rust {er})");
                // *clen_p left UNWRITTEN, `c` untouched
                assert_eq!(lc, POISON, "{name}: C wrote *clen_p");
                assert_eq!(lr, POISON, "{name}: Rust wrote *clen_p");
                eqb(&format!("{name}: c buffer"), &cc, &cr);
                assert!(
                    cc[..mlen + 16].iter().all(|b| *b == 0xDD),
                    "{name}: C touched the output buffer"
                );
                check_pad(&format!("{name}: c (C)"), &cc, mlen + 16);
                check_pad(&format!("{name}: c (Rust)"), &cr, mlen + 16);
            }
        }
    }
}

fn check_encrypt_detached(name: &str, key_or_state: *const c_void) {
    let (c, r) = both::<EncD>(name);
    let mut rng = Rng::new(0x6131);
    for &mlen in MLEN.iter() {
        for &adc in ADLEN.iter() {
            let adlen = adc.unwrap_or(0);
            let m = rng.bytes(mlen + 1);
            let ad = rng.bytes(adlen + 1);
            let npub = rng.bytes(12);
            let (adp, adl) = ptr_len(&ad, adc);
            for with_ptr in [true, false] {
                let mut cc = poisoned(mlen);
                let mut cr = poisoned(mlen);
                let mut mc = poisoned(16);
                let mut mr = poisoned(16);
                let mut lc = POISON;
                let mut lr = POISON;
                let (pc, pr) = if with_ptr {
                    (&mut lc as *mut u64, &mut lr as *mut u64)
                } else {
                    (null_mut(), null_mut())
                };
                set_errno(0);
                let rc = unsafe {
                    c(
                        cc.as_mut_ptr(),
                        mc.as_mut_ptr(),
                        pc,
                        m.as_ptr(),
                        mlen as u64,
                        adp,
                        adl,
                        null(),
                        npub.as_ptr(),
                        key_or_state,
                    )
                };
                let ec = errno();
                set_errno(0);
                let rr = unsafe {
                    r(
                        cr.as_mut_ptr(),
                        mr.as_mut_ptr(),
                        pr,
                        m.as_ptr(),
                        mlen as u64,
                        adp,
                        adl,
                        null(),
                        npub.as_ptr(),
                        key_or_state,
                    )
                };
                let er = errno();
                eqi(&format!("{name} mlen={mlen} adc={adc:?}"), rc, rr);
                assert_eq!(rc, -1);
                assert_eq!(ec, ENOSYS, "{name}: C errno");
                assert_eq!(er, ec, "{name}: errno mismatch");
                assert_eq!(lc, POISON, "{name}: C wrote *maclen_p");
                assert_eq!(lr, POISON, "{name}: Rust wrote *maclen_p");
                eqb(&format!("{name}: c"), &cc, &cr);
                eqb(&format!("{name}: mac"), &mc, &mr);
                assert!(cc[..mlen].iter().all(|b| *b == 0xDD));
                assert!(mc[..16].iter().all(|b| *b == 0xDD));
                check_pad(&format!("{name}: c pad"), &cc, mlen);
                check_pad(&format!("{name}: mac pad"), &mc, 16);
            }
        }
    }
}

fn check_decrypt(name: &str, key_or_state: *const c_void) {
    let (c, r) = both::<Dec>(name);
    let mut rng = Rng::new(0x6132);
    for &clen in [0usize, 1, 15, 16, 17, 48].iter() {
        for &adc in ADLEN.iter() {
            let adlen = adc.unwrap_or(0);
            let cbuf = rng.bytes(clen + 1);
            let ad = rng.bytes(adlen + 1);
            let npub = rng.bytes(12);
            let (adp, adl) = ptr_len(&ad, adc);
            for with_ptr in [true, false] {
                let mcap = clen.max(16);
                let mut mc = poisoned(mcap);
                let mut mr = poisoned(mcap);
                let mut lc = POISON;
                let mut lr = POISON;
                let (pc, pr) = if with_ptr {
                    (&mut lc as *mut u64, &mut lr as *mut u64)
                } else {
                    (null_mut(), null_mut())
                };
                set_errno(0);
                let rc = unsafe {
                    c(
                        mc.as_mut_ptr(),
                        pc,
                        null_mut(),
                        cbuf.as_ptr(),
                        clen as u64,
                        adp,
                        adl,
                        npub.as_ptr(),
                        key_or_state,
                    )
                };
                let ec = errno();
                set_errno(0);
                let rr = unsafe {
                    r(
                        mr.as_mut_ptr(),
                        pr,
                        null_mut(),
                        cbuf.as_ptr(),
                        clen as u64,
                        adp,
                        adl,
                        npub.as_ptr(),
                        key_or_state,
                    )
                };
                let er = errno();
                eqi(&format!("{name} clen={clen} adc={adc:?}"), rc, rr);
                assert_eq!(rc, -1);
                assert_eq!(ec, ENOSYS, "{name}: C errno");
                assert_eq!(er, ec, "{name}: errno mismatch");
                // *mlen_p left UNWRITTEN — this is the aes256gcm-specific
                // difference from every other AEAD (which stores 0).
                assert_eq!(lc, POISON, "{name}: C wrote *mlen_p");
                assert_eq!(lr, POISON, "{name}: Rust wrote *mlen_p");
                eqb(&format!("{name}: m"), &mc, &mr);
                assert!(
                    mc[..mcap].iter().all(|b| *b == 0xDD),
                    "{name}: m was touched (not even zeroed is expected)"
                );
                check_pad(&format!("{name}: m pad"), &mc, mcap);
            }
        }
    }
}

fn check_decrypt_detached(name: &str, key_or_state: *const c_void) {
    let (c, r) = both::<DecD>(name);
    let mut rng = Rng::new(0x6133);
    for &clen in MLEN.iter() {
        for &adc in ADLEN.iter() {
            let adlen = adc.unwrap_or(0);
            let cbuf = rng.bytes(clen + 1);
            let ad = rng.bytes(adlen + 1);
            let npub = rng.bytes(12);
            let mac = rng.bytes(16);
            let (adp, adl) = ptr_len(&ad, adc);
            for m_null in [false, true] {
                let mut mc = poisoned(clen.max(16));
                let mut mr = poisoned(clen.max(16));
                let (mpc, mpr) = if m_null {
                    (null_mut(), null_mut())
                } else {
                    (mc.as_mut_ptr(), mr.as_mut_ptr())
                };
                set_errno(0);
                let rc = unsafe {
                    c(
                        mpc,
                        null_mut(),
                        cbuf.as_ptr(),
                        clen as u64,
                        mac.as_ptr(),
                        adp,
                        adl,
                        npub.as_ptr(),
                        key_or_state,
                    )
                };
                let ec = errno();
                set_errno(0);
                let rr = unsafe {
                    r(
                        mpr,
                        null_mut(),
                        cbuf.as_ptr(),
                        clen as u64,
                        mac.as_ptr(),
                        adp,
                        adl,
                        npub.as_ptr(),
                        key_or_state,
                    )
                };
                let er = errno();
                eqi(&format!("{name} clen={clen} m_null={m_null}"), rc, rr);
                assert_eq!(rc, -1);
                assert_eq!(ec, ENOSYS, "{name}: C errno");
                assert_eq!(er, ec, "{name}: errno mismatch");
                eqb(&format!("{name}: m"), &mc, &mr);
                assert!(
                    mc[..clen.max(16)].iter().all(|b| *b == 0xDD),
                    "{name}: m was written/zeroed"
                );
            }
        }
    }
}

#[test]
fn encrypt_is_enosys() {
    let k = vec![0x42u8; 32];
    check_encrypt("crypto_aead_aes256gcm_encrypt", k.as_ptr() as *const c_void);
}

#[test]
fn encrypt_detached_is_enosys() {
    let k = vec![0x42u8; 32];
    check_encrypt_detached(
        "crypto_aead_aes256gcm_encrypt_detached",
        k.as_ptr() as *const c_void,
    );
}

#[test]
fn decrypt_is_enosys() {
    let k = vec![0x42u8; 32];
    check_decrypt("crypto_aead_aes256gcm_decrypt", k.as_ptr() as *const c_void);
}

#[test]
fn decrypt_detached_is_enosys() {
    let k = vec![0x42u8; 32];
    check_decrypt_detached(
        "crypto_aead_aes256gcm_decrypt_detached",
        k.as_ptr() as *const c_void,
    );
}

// ------------------------------------------------------------------ 6.34 / errors 6.35

#[test]
fn beforenm_is_enosys_stack_and_heap() {
    let (bc, br) = both::<Beforenm>("crypto_aead_aes256gcm_beforenm");
    let (getter_c, _) = both::<Getter>("crypto_aead_aes256gcm_statebytes");
    let sb = unsafe { getter_c() };
    let k = vec![0x5Au8; 32];

    // --- stack, CRYPTO_ALIGN(16)
    let mut sc = AlignedState([0xDD; 512]);
    let mut sr = AlignedState([0xDD; 512]);
    assert_eq!(sc.0.as_ptr() as usize % 16, 0);
    set_errno(0);
    let rc = unsafe { bc(sc.0.as_mut_ptr() as *mut c_void, k.as_ptr()) };
    let ec = errno();
    set_errno(0);
    let rr = unsafe { br(sr.0.as_mut_ptr() as *mut c_void, k.as_ptr()) };
    let er = errno();
    eqi("beforenm (stack)", rc, rr);
    assert_eq!(rc, -1);
    assert_eq!(ec, ENOSYS);
    assert_eq!(er, ec);
    eqb("beforenm state (stack)", &sc.0, &sr.0);
    assert!(
        sc.0.iter().all(|b| *b == 0xDD),
        "beforenm must leave the state uninitialised"
    );

    // --- heap, via each library's own sodium_malloc (16-byte aligned)
    for (lib, bn) in [(c_lib(), &bc), (rust_lib(), &br)] {
        unsafe {
            let sm: libloading::Symbol<SodiumMalloc> = lib.get(b"sodium_malloc\0").unwrap();
            let sf: libloading::Symbol<SodiumFree> = lib.get(b"sodium_free\0").unwrap();
            let p = sm(sb);
            assert!(!p.is_null(), "sodium_malloc failed");
            assert_eq!(p as usize % 16, 0, "sodium_malloc is not 16-byte aligned");
            std::ptr::write_bytes(p as *mut u8, 0xDD, sb);
            set_errno(0);
            let rc = bn(p, k.as_ptr());
            let e = errno();
            assert_eq!(rc, -1, "heap beforenm return");
            assert_eq!(e, ENOSYS, "heap beforenm errno");
            let slice = std::slice::from_raw_parts(p as *const u8, sb);
            assert!(
                slice.iter().all(|b| *b == 0xDD),
                "heap beforenm wrote to the state"
            );
            sf(p);
        }
    }
}

// ------------------------------------------------------------------ 6.35–6.38

#[test]
fn afternm_variants_are_enosys() {
    // The only obtainable state is one from a failed `_beforenm`; use it as-is.
    let mut st = AlignedState([0xDD; 512]);
    let k = vec![0x5Au8; 32];
    let (bc, br) = both::<Beforenm>("crypto_aead_aes256gcm_beforenm");
    unsafe {
        assert_eq!(bc(st.0.as_mut_ptr() as *mut c_void, k.as_ptr()), -1);
        assert_eq!(br(st.0.as_mut_ptr() as *mut c_void, k.as_ptr()), -1);
    }
    let sp = st.0.as_ptr() as *const c_void;
    check_encrypt("crypto_aead_aes256gcm_encrypt_afternm", sp);
    check_encrypt_detached("crypto_aead_aes256gcm_encrypt_detached_afternm", sp);
    check_decrypt("crypto_aead_aes256gcm_decrypt_afternm", sp);
    check_decrypt_detached("crypto_aead_aes256gcm_decrypt_detached_afternm", sp);
}

// ------------------------------------------------------------------ 6.39

#[test]
fn full_state_api_sequence_never_succeeds() {
    let (bc, br) = both::<Beforenm>("crypto_aead_aes256gcm_beforenm");
    let (ec_, er_) = both::<Enc>("crypto_aead_aes256gcm_encrypt_afternm");
    let (dc, dr) = both::<Dec>("crypto_aead_aes256gcm_decrypt_afternm");
    let k = vec![0x11u8; 32];
    let npub = vec![0x22u8; 12];
    let m = vec![0x33u8; 64];
    let ad = vec![0x44u8; 20];

    for (before, enc, dec) in [(&bc, &ec_, &dc), (&br, &er_, &dr)] {
        let mut st = AlignedState([0xDD; 512]);
        let mut cbuf = poisoned(64 + 16);
        let mut mbuf = poisoned(64);
        let mut clen = POISON;
        let mut mlen = POISON;

        set_errno(0);
        let r1 = unsafe { before(st.0.as_mut_ptr() as *mut c_void, k.as_ptr()) };
        assert_eq!(r1, -1);
        assert_eq!(errno(), ENOSYS);

        set_errno(0);
        let r2 = unsafe {
            enc(
                cbuf.as_mut_ptr(),
                &mut clen,
                m.as_ptr(),
                m.len() as u64,
                ad.as_ptr(),
                ad.len() as u64,
                null(),
                npub.as_ptr(),
                st.0.as_ptr() as *const c_void,
            )
        };
        assert_eq!(r2, -1);
        assert_eq!(errno(), ENOSYS);
        assert_eq!(clen, POISON);

        set_errno(0);
        let r3 = unsafe {
            dec(
                mbuf.as_mut_ptr(),
                &mut mlen,
                null_mut(),
                cbuf.as_ptr(),
                (64 + 16) as u64,
                ad.as_ptr(),
                ad.len() as u64,
                npub.as_ptr(),
                st.0.as_ptr() as *const c_void,
            )
        };
        assert_eq!(r3, -1);
        assert_eq!(errno(), ENOSYS);
        assert_eq!(mlen, POISON);

        // nothing was produced or consumed anywhere
        assert!(cbuf[..80].iter().all(|b| *b == 0xDD));
        assert!(mbuf[..64].iter().all(|b| *b == 0xDD));
        assert!(st.0.iter().all(|b| *b == 0xDD));
    }
}
