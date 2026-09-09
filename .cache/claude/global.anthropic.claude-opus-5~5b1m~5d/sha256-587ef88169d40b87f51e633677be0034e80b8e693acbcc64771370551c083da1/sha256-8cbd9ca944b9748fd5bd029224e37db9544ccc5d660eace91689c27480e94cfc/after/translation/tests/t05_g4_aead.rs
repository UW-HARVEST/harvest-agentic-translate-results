//! Phase B — valid-path differential tests for group G4 (CONFIGS.md rows 510-716).
//!
//! Every test drives BOTH the C reference library and the Rust translation
//! through `libloading` and compares return values, output buffers and
//! `*_p` out-parameters byte-for-byte.
//!
//! Modules covered: `crypto_aead/` (aegis128l, aegis256, aes256gcm,
//! chacha20poly1305 original + ietf, xchacha20poly1305), `crypto_secretbox/`
//! (xsalsa20poly1305 incl. the NaCl zero-padded form, xchacha20poly1305),
//! `crypto_secretstream/`, `crypto_stream/` (chacha20 + ietf + ietf_ext,
//! salsa20, salsa2012, salsa208, xsalsa20, xchacha20, default), and
//! `crypto_core_{salsa20,salsa2012,salsa208,hsalsa20,hchacha20}`.
#![allow(clippy::too_many_arguments)]

mod common;
use common::*;
use std::os::raw::{c_char, c_int};
use std::ptr;

// ===========================================================================
// C signatures
// ===========================================================================
type SzFn = unsafe extern "C" fn() -> usize;
type U8Fn = unsafe extern "C" fn() -> u8;
type StrFn = unsafe extern "C" fn() -> *const c_char;
type IntFn = unsafe extern "C" fn() -> c_int;
type KeygenFn = unsafe extern "C" fn(*mut u8);

/// `(c, clen_p, m, mlen, ad, adlen, nsec, npub, k)` — also matches
/// `crypto_aead_aes256gcm_encrypt_afternm` (last arg = state).
type EncFn = unsafe extern "C" fn(
    *mut u8,
    *mut u64,
    *const u8,
    u64,
    *const u8,
    u64,
    *const u8,
    *const u8,
    *const u8,
) -> c_int;
/// `(m, mlen_p, nsec, c, clen, ad, adlen, npub, k)`
type DecFn = unsafe extern "C" fn(
    *mut u8,
    *mut u64,
    *mut u8,
    *const u8,
    u64,
    *const u8,
    u64,
    *const u8,
    *const u8,
) -> c_int;
/// `(c, mac, maclen_p, m, mlen, ad, adlen, nsec, npub, k)`
type EncDetFn = unsafe extern "C" fn(
    *mut u8,
    *mut u8,
    *mut u64,
    *const u8,
    u64,
    *const u8,
    u64,
    *const u8,
    *const u8,
    *const u8,
) -> c_int;
/// `(m, nsec, c, clen, mac, ad, adlen, npub, k)`
type DecDetFn = unsafe extern "C" fn(
    *mut u8,
    *mut u8,
    *const u8,
    u64,
    *const u8,
    *const u8,
    u64,
    *const u8,
    *const u8,
) -> c_int;
type BeforenmFn = unsafe extern "C" fn(*mut u8, *const u8) -> c_int;

// secretbox
type SbEasy = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, *const u8) -> c_int;
type SbDet = unsafe extern "C" fn(*mut u8, *mut u8, *const u8, u64, *const u8, *const u8) -> c_int;
type SbOpenDet =
    unsafe extern "C" fn(*mut u8, *const u8, *const u8, u64, *const u8, *const u8) -> c_int;

// secretstream
type SsInitPush = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> c_int;
type SsInitPull = unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> c_int;
type SsPush =
    unsafe extern "C" fn(*mut u8, *mut u8, *mut u64, *const u8, u64, *const u8, u64, u8) -> c_int;
type SsPull = unsafe extern "C" fn(
    *mut u8,
    *mut u8,
    *mut u64,
    *mut u8,
    *const u8,
    u64,
    *const u8,
    u64,
) -> c_int;
type SsRekey = unsafe extern "C" fn(*mut u8);

// crypto_stream
type StreamFn = unsafe extern "C" fn(*mut u8, u64, *const u8, *const u8) -> c_int;
type XorFn = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, *const u8) -> c_int;
type XorIc64Fn = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, u64, *const u8) -> c_int;
type XorIc32Fn = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, u32, *const u8) -> c_int;

// crypto_core
type CoreFn = unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8) -> c_int;

// ===========================================================================
// small helpers
// ===========================================================================
unsafe fn fpair<F: Copy + 'static>(name: &str) -> (F, F) {
    let (a, b) = pair::<F>(name);
    (*a, *b)
}

/// Call a `size_t (*)(void)` getter in both libraries, assert they agree,
/// return the shared value.
#[track_caller]
fn sz(name: &str) -> usize {
    unsafe {
        let (c, r) = fpair::<SzFn>(name);
        let (a, b) = (c(), r());
        assert_eq!(a, b, "{name}: C returned {a}, Rust returned {b}");
        a
    }
}

#[track_caller]
fn sz_is(name: &str, want: usize) -> usize {
    let v = sz(name);
    assert_eq!(v, want, "{name}: expected {want}, both libraries returned {v}");
    v
}

#[track_caller]
fn u8_is(name: &str, want: u8) {
    unsafe {
        let (c, r) = fpair::<U8Fn>(name);
        let (a, b) = (c(), r());
        assert_eq!(a, b, "{name}: C={a} Rust={b}");
        assert_eq!(a, want, "{name}: expected {want}, got {a}");
    }
}

#[track_caller]
fn str_is(name: &str, want: &str) {
    unsafe {
        let (c, r) = fpair::<StrFn>(name);
        let a = std::ffi::CStr::from_ptr(c()).to_str().unwrap().to_owned();
        let b = std::ffi::CStr::from_ptr(r()).to_str().unwrap().to_owned();
        assert_eq!(a, b, "{name}: C={a:?} Rust={b:?}");
        assert_eq!(a, want, "{name}: expected {want:?}");
    }
}

/// `*_keygen`: deterministic randombytes makes the two libraries comparable.
#[track_caller]
fn keygen_check(name: &str, n: usize) {
    install_det_random();
    let (c, r) = unsafe { fpair::<KeygenFn>(name) };
    const G: usize = 8;
    let mut a = vec![0u8; n + G];
    let mut b = vec![0u8; n + G];
    det_reseed(0x5EED_0001);
    unsafe { c(a.as_mut_ptr()) };
    det_reseed(0x5EED_0001);
    unsafe { r(b.as_mut_ptr()) };
    eq_bytes(&format!("{name}"), &a, &b);
    assert!(
        a[n..].iter().all(|&x| x == 0),
        "{name}: wrote past {n} bytes"
    );
    assert!(a[..n].iter().any(|&x| x != 0), "{name}: produced a zero key");
    // Two successive draws from the same stream must differ, and the two
    // libraries must produce the same *sequence*.
    let mut a2 = vec![0u8; n + G];
    let mut b2 = vec![0u8; n + G];
    det_reseed(0x5EED_0002);
    unsafe {
        c(a.as_mut_ptr());
        c(a2.as_mut_ptr());
    }
    det_reseed(0x5EED_0002);
    unsafe {
        r(b.as_mut_ptr());
        r(b2.as_mut_ptr());
    }
    eq_bytes(&format!("{name} seq[0]"), &a, &b);
    eq_bytes(&format!("{name} seq[1]"), &a2, &b2);
    assert_ne!(a[..n], a2[..n], "{name}: two keygen calls returned same key");
}

const MLENS: &[usize] = &[
    0, 1, 15, 16, 17, 31, 32, 33, 63, 64, 65, 127, 128, 129, 1000,
];
const ADLENS: &[usize] = &[0, 1, 15, 16, 17, 31, 32, 33, 63, 64, 65, 1000];
const G: usize = 5; // guard bytes appended to every output buffer

// ===========================================================================
// generic AEAD driver
// ===========================================================================
struct Aead {
    pfx: String,
    kb: usize,
    npb: usize,
    ab: usize,
    enc_c: EncFn,
    enc_r: EncFn,
    dec_c: DecFn,
    dec_r: DecFn,
    ed_c: EncDetFn,
    ed_r: EncDetFn,
    dd_c: DecDetFn,
    dd_r: DecDetFn,
}

impl Aead {
    fn new(pfx: &str) -> Aead {
        unsafe {
            let (enc_c, enc_r) = fpair::<EncFn>(&format!("{pfx}_encrypt"));
            let (dec_c, dec_r) = fpair::<DecFn>(&format!("{pfx}_decrypt"));
            let (ed_c, ed_r) = fpair::<EncDetFn>(&format!("{pfx}_encrypt_detached"));
            let (dd_c, dd_r) = fpair::<DecDetFn>(&format!("{pfx}_decrypt_detached"));
            Aead {
                pfx: pfx.to_string(),
                kb: sz(&format!("{pfx}_keybytes")),
                npb: sz(&format!("{pfx}_npubbytes")),
                ab: sz(&format!("{pfx}_abytes")),
                enc_c,
                enc_r,
                dec_c,
                dec_r,
                ed_c,
                ed_r,
                dd_c,
                dd_r,
            }
        }
    }

    /// Combined encrypt in both libraries; returns the shared ciphertext.
    fn enc_cmp(&self, label: &str, m: &[u8], ad: &[u8], npub: &[u8], k: &[u8]) -> Vec<u8> {
        let mlen = m.len();
        let clen = mlen + self.ab;
        let adp = if ad.is_empty() {
            ptr::null()
        } else {
            ad.as_ptr()
        };
        let mut a = vec![0xA5u8; clen + G];
        let mut b = vec![0xA5u8; clen + G];
        let mut la = u64::MAX;
        let mut lb = u64::MAX;
        let (ra, rb) = unsafe {
            (
                (self.enc_c)(
                    a.as_mut_ptr(),
                    &mut la,
                    m.as_ptr(),
                    mlen as u64,
                    adp,
                    ad.len() as u64,
                    ptr::null(),
                    npub.as_ptr(),
                    k.as_ptr(),
                ),
                (self.enc_r)(
                    b.as_mut_ptr(),
                    &mut lb,
                    m.as_ptr(),
                    mlen as u64,
                    adp,
                    ad.len() as u64,
                    ptr::null(),
                    npub.as_ptr(),
                    k.as_ptr(),
                ),
            )
        };
        eq_i32(&format!("{label}: encrypt rc"), ra, rb);
        eq_bytes(&format!("{label}: encrypt out"), &a, &b);
        assert_eq!(la, lb, "{label}: encrypt clen_p (C={la} Rust={lb})");
        assert_eq!(ra, 0, "{label}: encrypt must succeed");
        assert_eq!(la, clen as u64, "{label}: clen_p value");
        assert!(
            a[clen..].iter().all(|&x| x == 0xA5),
            "{label}: encrypt wrote past clen"
        );
        a.truncate(clen);
        a
    }

    /// Combined decrypt in both libraries; compares rc, `*mlen_p` and `m`.
    fn dec_cmp(&self, label: &str, ct: &[u8], ad: &[u8], npub: &[u8], k: &[u8]) -> c_int {
        let clen = ct.len();
        let mlen = clen.saturating_sub(self.ab);
        let adp = if ad.is_empty() {
            ptr::null()
        } else {
            ad.as_ptr()
        };
        let mut a = vec![0x33u8; mlen + G];
        let mut b = vec![0x33u8; mlen + G];
        let mut la = u64::MAX;
        let mut lb = u64::MAX;
        let (ra, rb) = unsafe {
            (
                (self.dec_c)(
                    a.as_mut_ptr(),
                    &mut la,
                    ptr::null_mut(),
                    ct.as_ptr(),
                    clen as u64,
                    adp,
                    ad.len() as u64,
                    npub.as_ptr(),
                    k.as_ptr(),
                ),
                (self.dec_r)(
                    b.as_mut_ptr(),
                    &mut lb,
                    ptr::null_mut(),
                    ct.as_ptr(),
                    clen as u64,
                    adp,
                    ad.len() as u64,
                    npub.as_ptr(),
                    k.as_ptr(),
                ),
            )
        };
        eq_i32(&format!("{label}: decrypt rc"), ra, rb);
        eq_bytes(&format!("{label}: decrypt m"), &a, &b);
        assert_eq!(la, lb, "{label}: decrypt mlen_p (C={la} Rust={lb})");
        if ra == 0 {
            assert_eq!(la, mlen as u64, "{label}: mlen_p on success");
        } else {
            assert_eq!(la, 0, "{label}: mlen_p must be 0 on failure");
        }
        assert!(
            a[mlen..].iter().all(|&x| x == 0x33),
            "{label}: decrypt wrote past mlen"
        );
        ra
    }

    /// Detached decrypt in both libraries, optionally in verify-only
    /// (`m == NULL`) mode.
    fn dec_det_cmp(
        &self,
        label: &str,
        c: &[u8],
        mac: &[u8],
        ad: &[u8],
        npub: &[u8],
        k: &[u8],
        m_null: bool,
    ) -> c_int {
        let clen = c.len();
        let adp = if ad.is_empty() {
            ptr::null()
        } else {
            ad.as_ptr()
        };
        let mut a = vec![0x77u8; clen + G];
        let mut b = vec![0x77u8; clen + G];
        let (pa, pb) = if m_null {
            (ptr::null_mut(), ptr::null_mut())
        } else {
            (a.as_mut_ptr(), b.as_mut_ptr())
        };
        let (ra, rb) = unsafe {
            (
                (self.dd_c)(
                    pa,
                    ptr::null_mut(),
                    c.as_ptr(),
                    clen as u64,
                    mac.as_ptr(),
                    adp,
                    ad.len() as u64,
                    npub.as_ptr(),
                    k.as_ptr(),
                ),
                (self.dd_r)(
                    pb,
                    ptr::null_mut(),
                    c.as_ptr(),
                    clen as u64,
                    mac.as_ptr(),
                    adp,
                    ad.len() as u64,
                    npub.as_ptr(),
                    k.as_ptr(),
                ),
            )
        };
        eq_i32(&format!("{label}: decrypt_detached rc"), ra, rb);
        eq_bytes(&format!("{label}: decrypt_detached m"), &a, &b);
        if m_null {
            assert!(
                a.iter().all(|&x| x == 0x77),
                "{label}: verify-only mode wrote to a buffer"
            );
        }
        ra
    }

    /// Full valid-path battery for one (mlen, adlen) shape.
    fn one(&self, rng: &mut Rng, mlen: usize, adlen: usize) {
        let ab = self.ab;
        let label = format!("{} m={} ad={}", self.pfx, mlen, adlen);
        let k = rng.bytes(self.kb);
        let npub = rng.bytes(self.npb);
        let m = rng.bytes(mlen);
        let ad = rng.bytes(adlen);
        let adp = if adlen == 0 {
            ptr::null()
        } else {
            ad.as_ptr()
        };
        let clen = mlen + ab;

        // --- combined encrypt ------------------------------------------------
        let ct = self.enc_cmp(&label, &m, &ad, &npub, &k);

        // clen_p == NULL
        {
            let mut a = vec![0xA5u8; clen + G];
            let mut b = vec![0xA5u8; clen + G];
            let (ra, rb) = unsafe {
                (
                    (self.enc_c)(
                        a.as_mut_ptr(),
                        ptr::null_mut(),
                        m.as_ptr(),
                        mlen as u64,
                        adp,
                        adlen as u64,
                        ptr::null(),
                        npub.as_ptr(),
                        k.as_ptr(),
                    ),
                    (self.enc_r)(
                        b.as_mut_ptr(),
                        ptr::null_mut(),
                        m.as_ptr(),
                        mlen as u64,
                        adp,
                        adlen as u64,
                        ptr::null(),
                        npub.as_ptr(),
                        k.as_ptr(),
                    ),
                )
            };
            eq_i32(&format!("{label}: encrypt clen_p=NULL rc"), ra, rb);
            eq_bytes(&format!("{label}: encrypt clen_p=NULL"), &a, &b);
            eq_bytes(&format!("{label}: encrypt clen_p=NULL == ct"), &a[..clen], &ct);
        }

        // in-place c == m (with ABYTES of tail room)
        {
            let mut a = vec![0xA5u8; clen + G];
            a[..mlen].copy_from_slice(&m);
            let mut b = a.clone();
            unsafe {
                let pa = a.as_mut_ptr();
                let pb = b.as_mut_ptr();
                let ra = (self.enc_c)(
                    pa,
                    ptr::null_mut(),
                    pa,
                    mlen as u64,
                    adp,
                    adlen as u64,
                    ptr::null(),
                    npub.as_ptr(),
                    k.as_ptr(),
                );
                let rb = (self.enc_r)(
                    pb,
                    ptr::null_mut(),
                    pb,
                    mlen as u64,
                    adp,
                    adlen as u64,
                    ptr::null(),
                    npub.as_ptr(),
                    k.as_ptr(),
                );
                eq_i32(&format!("{label}: in-place encrypt rc"), ra, rb);
            }
            eq_bytes(&format!("{label}: in-place encrypt"), &a, &b);
            eq_bytes(&format!("{label}: in-place encrypt == ct"), &a[..clen], &ct);
        }

        // --- detached encrypt ------------------------------------------------
        let mut mac_c = vec![0x5Au8; ab + G];
        {
            let mut ca = vec![0xA5u8; mlen + G];
            let mut cb = vec![0xA5u8; mlen + G];
            let mut mb = vec![0x5Au8; ab + G];
            let mut la = u64::MAX;
            let mut lb = u64::MAX;
            let (ra, rb) = unsafe {
                (
                    (self.ed_c)(
                        ca.as_mut_ptr(),
                        mac_c.as_mut_ptr(),
                        &mut la,
                        m.as_ptr(),
                        mlen as u64,
                        adp,
                        adlen as u64,
                        ptr::null(),
                        npub.as_ptr(),
                        k.as_ptr(),
                    ),
                    (self.ed_r)(
                        cb.as_mut_ptr(),
                        mb.as_mut_ptr(),
                        &mut lb,
                        m.as_ptr(),
                        mlen as u64,
                        adp,
                        adlen as u64,
                        ptr::null(),
                        npub.as_ptr(),
                        k.as_ptr(),
                    ),
                )
            };
            eq_i32(&format!("{label}: encrypt_detached rc"), ra, rb);
            eq_bytes(&format!("{label}: encrypt_detached c"), &ca, &cb);
            eq_bytes(&format!("{label}: encrypt_detached mac"), &mac_c, &mb);
            assert_eq!(la, lb, "{label}: maclen_p");
            assert_eq!(la, ab as u64, "{label}: maclen_p value");
            // the detached ciphertext/mac must reconstruct the combined output
            eq_bytes(&format!("{label}: detached c == ct[..mlen]"), &ca[..mlen], &ct[..mlen]);
            eq_bytes(
                &format!("{label}: detached mac == ct tail"),
                &mac_c[..ab],
                &ct[mlen..],
            );
            // maclen_p == NULL
            let mut ca2 = vec![0xA5u8; mlen + G];
            let mut mac2 = vec![0x5Au8; ab + G];
            let mut cb2 = vec![0xA5u8; mlen + G];
            let mut mb2 = vec![0x5Au8; ab + G];
            unsafe {
                let ra = (self.ed_c)(
                    ca2.as_mut_ptr(),
                    mac2.as_mut_ptr(),
                    ptr::null_mut(),
                    m.as_ptr(),
                    mlen as u64,
                    adp,
                    adlen as u64,
                    ptr::null(),
                    npub.as_ptr(),
                    k.as_ptr(),
                );
                let rb = (self.ed_r)(
                    cb2.as_mut_ptr(),
                    mb2.as_mut_ptr(),
                    ptr::null_mut(),
                    m.as_ptr(),
                    mlen as u64,
                    adp,
                    adlen as u64,
                    ptr::null(),
                    npub.as_ptr(),
                    k.as_ptr(),
                );
                eq_i32(&format!("{label}: encrypt_detached maclen_p=NULL rc"), ra, rb);
            }
            eq_bytes(&format!("{label}: ed maclen_p=NULL c"), &ca2, &cb2);
            eq_bytes(&format!("{label}: ed maclen_p=NULL mac"), &mac2, &mb2);
        }
        mac_c.truncate(ab);

        // --- combined decrypt, valid ----------------------------------------
        assert_eq!(
            self.dec_cmp(&format!("{label} valid"), &ct, &ad, &npub, &k),
            0,
            "{label}: round trip must succeed"
        );
        // recovered plaintext must equal m (checked once, via the C library)
        {
            let mut a = vec![0x33u8; mlen + G];
            let mut la = u64::MAX;
            let ra = unsafe {
                (self.dec_c)(
                    a.as_mut_ptr(),
                    &mut la,
                    ptr::null_mut(),
                    ct.as_ptr(),
                    ct.len() as u64,
                    adp,
                    adlen as u64,
                    npub.as_ptr(),
                    k.as_ptr(),
                )
            };
            assert_eq!(ra, 0);
            eq_bytes(&format!("{label}: recovered plaintext"), &m, &a[..mlen]);
        }
        // mlen_p == NULL
        {
            let mut a = vec![0x33u8; mlen + G];
            let mut b = vec![0x33u8; mlen + G];
            let (ra, rb) = unsafe {
                (
                    (self.dec_c)(
                        a.as_mut_ptr(),
                        ptr::null_mut(),
                        ptr::null_mut(),
                        ct.as_ptr(),
                        ct.len() as u64,
                        adp,
                        adlen as u64,
                        npub.as_ptr(),
                        k.as_ptr(),
                    ),
                    (self.dec_r)(
                        b.as_mut_ptr(),
                        ptr::null_mut(),
                        ptr::null_mut(),
                        ct.as_ptr(),
                        ct.len() as u64,
                        adp,
                        adlen as u64,
                        npub.as_ptr(),
                        k.as_ptr(),
                    ),
                )
            };
            eq_i32(&format!("{label}: decrypt mlen_p=NULL rc"), ra, rb);
            eq_bytes(&format!("{label}: decrypt mlen_p=NULL m"), &a, &b);
        }
        // in-place m == c
        {
            let mut a = vec![0u8; ct.len() + G];
            a[..ct.len()].copy_from_slice(&ct);
            for x in a[ct.len()..].iter_mut() {
                *x = 0x33;
            }
            let mut b = a.clone();
            let mut la = u64::MAX;
            let mut lb = u64::MAX;
            unsafe {
                let pa = a.as_mut_ptr();
                let pb = b.as_mut_ptr();
                let ra = (self.dec_c)(
                    pa,
                    &mut la,
                    ptr::null_mut(),
                    pa,
                    ct.len() as u64,
                    adp,
                    adlen as u64,
                    npub.as_ptr(),
                    k.as_ptr(),
                );
                let rb = (self.dec_r)(
                    pb,
                    &mut lb,
                    ptr::null_mut(),
                    pb,
                    ct.len() as u64,
                    adp,
                    adlen as u64,
                    npub.as_ptr(),
                    k.as_ptr(),
                );
                eq_i32(&format!("{label}: in-place decrypt rc"), ra, rb);
            }
            eq_bytes(&format!("{label}: in-place decrypt"), &a, &b);
            assert_eq!(la, lb);
            eq_bytes(&format!("{label}: in-place plaintext"), &m, &a[..mlen]);
        }

        // --- detached decrypt -----------------------------------------------
        assert_eq!(
            self.dec_det_cmp(
                &format!("{label} det valid"),
                &ct[..mlen],
                &mac_c,
                &ad,
                &npub,
                &k,
                false
            ),
            0
        );
        assert_eq!(
            self.dec_det_cmp(
                &format!("{label} det valid m=NULL"),
                &ct[..mlen],
                &mac_c,
                &ad,
                &npub,
                &k,
                true
            ),
            0
        );

        // --- tampering: every -1 path must match ----------------------------
        // flip one bit in the ciphertext body
        if mlen > 0 {
            let mut bad = ct.clone();
            let i = rng.below(mlen);
            bad[i] ^= 1 << (rng.below(8));
            assert_eq!(self.dec_cmp(&format!("{label} tamper-ct"), &bad, &ad, &npub, &k), -1);
            assert_eq!(
                self.dec_det_cmp(
                    &format!("{label} tamper-ct det"),
                    &bad[..mlen],
                    &mac_c,
                    &ad,
                    &npub,
                    &k,
                    false
                ),
                -1
            );
            assert_eq!(
                self.dec_det_cmp(
                    &format!("{label} tamper-ct det m=NULL"),
                    &bad[..mlen],
                    &mac_c,
                    &ad,
                    &npub,
                    &k,
                    true
                ),
                -1
            );
        }
        // flip one bit in the mac
        {
            let mut bad = ct.clone();
            let i = mlen + rng.below(ab);
            bad[i] ^= 0x80;
            assert_eq!(self.dec_cmp(&format!("{label} tamper-mac"), &bad, &ad, &npub, &k), -1);
            let mut badmac = mac_c.clone();
            badmac[rng.below(ab)] ^= 0x01;
            assert_eq!(
                self.dec_det_cmp(
                    &format!("{label} tamper-mac det"),
                    &ct[..mlen],
                    &badmac,
                    &ad,
                    &npub,
                    &k,
                    false
                ),
                -1
            );
            assert_eq!(
                self.dec_det_cmp(
                    &format!("{label} tamper-mac det m=NULL"),
                    &ct[..mlen],
                    &badmac,
                    &ad,
                    &npub,
                    &k,
                    true
                ),
                -1
            );
        }
        // flip one bit in the nonce
        {
            let mut bad = npub.clone();
            bad[rng.below(self.npb)] ^= 0x10;
            assert_eq!(self.dec_cmp(&format!("{label} tamper-npub"), &ct, &ad, &bad, &k), -1);
            assert_eq!(
                self.dec_det_cmp(
                    &format!("{label} tamper-npub det"),
                    &ct[..mlen],
                    &mac_c,
                    &ad,
                    &bad,
                    &k,
                    false
                ),
                -1
            );
        }
        // flip one bit in the ad
        if adlen > 0 {
            let mut bad = ad.clone();
            bad[rng.below(adlen)] ^= 0x20;
            assert_eq!(self.dec_cmp(&format!("{label} tamper-ad"), &ct, &bad, &npub, &k), -1);
            assert_eq!(
                self.dec_det_cmp(
                    &format!("{label} tamper-ad det"),
                    &ct[..mlen],
                    &mac_c,
                    &bad,
                    &npub,
                    &k,
                    false
                ),
                -1
            );
        }
        // wrong key
        {
            let mut bad = k.clone();
            bad[rng.below(self.kb)] ^= 0x40;
            assert_eq!(self.dec_cmp(&format!("{label} tamper-k"), &ct, &ad, &npub, &bad), -1);
        }
    }

    /// `ad != NULL` with `adlen == 0` must be byte-identical to `ad == NULL`.
    fn ad_null_equiv(&self, rng: &mut Rng, mlen: usize) {
        let label = format!("{} ad-null-equiv m={}", self.pfx, mlen);
        let k = rng.bytes(self.kb);
        let npub = rng.bytes(self.npb);
        let m = rng.bytes(mlen);
        let adbuf = rng.bytes(32);
        let clen = mlen + self.ab;
        let ptrs: [*const u8; 2] = [ptr::null(), adbuf.as_ptr()];
        let mut shared: Vec<Vec<u8>> = Vec::new();
        for (i, adp) in ptrs.iter().enumerate() {
            let mut a = vec![0xA5u8; clen + G];
            let mut b = vec![0xA5u8; clen + G];
            unsafe {
                let ra = (self.enc_c)(
                    a.as_mut_ptr(),
                    ptr::null_mut(),
                    m.as_ptr(),
                    mlen as u64,
                    *adp,
                    0,
                    ptr::null(),
                    npub.as_ptr(),
                    k.as_ptr(),
                );
                let rb = (self.enc_r)(
                    b.as_mut_ptr(),
                    ptr::null_mut(),
                    m.as_ptr(),
                    mlen as u64,
                    *adp,
                    0,
                    ptr::null(),
                    npub.as_ptr(),
                    k.as_ptr(),
                );
                eq_i32(&format!("{label} variant {i} rc"), ra, rb);
            }
            eq_bytes(&format!("{label} variant {i}"), &a, &b);
            shared.push(a);
        }
        eq_bytes(
            &format!("{label}: ad=NULL vs ad!=NULL,adlen=0"),
            &shared[0],
            &shared[1],
        );
    }
}

// ===========================================================================
// aegis128l — CONFIGS rows 510-533
// ===========================================================================
#[test]
fn c510_aegis128l_keygen() {
    // row 510
    keygen_check("crypto_aead_aegis128l_keygen", 16);
}

#[test]
fn c511_aegis128l_constants() {
    // row 511
    sz_is("crypto_aead_aegis128l_keybytes", 16);
    sz_is("crypto_aead_aegis128l_nsecbytes", 0);
    sz_is("crypto_aead_aegis128l_npubbytes", 16);
    sz_is("crypto_aead_aegis128l_abytes", 32);
    sz_is(
        "crypto_aead_aegis128l_messagebytes_max",
        core::cmp::min(usize::MAX - 32, (1usize << 61) - 1),
    );
}

#[test]
fn c512_520_522_527_530_aegis128l_encrypt_sweep() {
    // rows 512, 514-520, 522, 527, 529, 530, 531 (full mlen x adlen sweep,
    // combined + detached + round trip + in-place + tamper)
    let a = Aead::new("crypto_aead_aegis128l");
    let mut rng = Rng::new(0x0A_E6_15_12);
    for &ml in MLENS {
        for &al in ADLENS {
            a.one(&mut rng, ml, al);
        }
    }
}

#[test]
fn c513_aegis128l_ad_nonnull_adlen0() {
    // row 513
    let a = Aead::new("crypto_aead_aegis128l");
    let mut rng = Rng::new(0x0A_E6_15_13);
    for &ml in &[0usize, 1, 16, 32, 33, 1000] {
        a.ad_null_equiv(&mut rng, ml);
    }
}

#[test]
fn c521_524_525_526_528_532_533_aegis128l_pointer_modes() {
    // rows 521 (clen_p NULL), 524 (clen == ABYTES), 525 (mlen_p NULL),
    // 526 (in-place), 528 (maclen_p NULL), 532 (m == NULL verify-only),
    // 533 (clen == 0 with non-empty ad).
    // All of these are exercised inside Aead::one; here we pin down the two
    // extreme shapes explicitly.
    let a = Aead::new("crypto_aead_aegis128l");
    let mut rng = Rng::new(0x0A_E6_15_21);
    a.one(&mut rng, 0, 0); // clen == ABYTES exactly
    a.one(&mut rng, 0, 1000); // clen == 0 detached, mac over ad only
    for &al in &[0usize, 1, 16, 17, 1000] {
        let k = rng.bytes(a.kb);
        let npub = rng.bytes(a.npb);
        let ad = rng.bytes(al);
        let ct = a.enc_cmp("aegis128l clen0", &[], &ad, &npub, &k);
        // detached decrypt with clen == 0
        assert_eq!(
            a.dec_det_cmp("aegis128l clen0 det", &[], &ct, &ad, &npub, &k, false),
            0
        );
        assert_eq!(
            a.dec_det_cmp("aegis128l clen0 det mNULL", &[], &ct, &ad, &npub, &k, true),
            0
        );
    }
}

#[test]
fn c523_aegis128l_decrypt_roundtrip_large() {
    // row 523 — round trip for large / rate-straddling shapes
    let a = Aead::new("crypto_aead_aegis128l");
    let mut rng = Rng::new(0x0A_E6_15_23);
    for &ml in &[1000usize, 4096, 4097, 8191] {
        for &al in &[0usize, 17, 1000] {
            a.one(&mut rng, ml, al);
        }
    }
}

// ===========================================================================
// aegis256 — CONFIGS rows 534-554
// ===========================================================================
#[test]
fn c534_aegis256_keygen() {
    // row 534
    keygen_check("crypto_aead_aegis256_keygen", 32);
}

#[test]
fn c535_aegis256_constants() {
    // row 535
    sz_is("crypto_aead_aegis256_keybytes", 32);
    sz_is("crypto_aead_aegis256_nsecbytes", 0);
    sz_is("crypto_aead_aegis256_npubbytes", 32);
    sz_is("crypto_aead_aegis256_abytes", 32);
    sz_is(
        "crypto_aead_aegis256_messagebytes_max",
        core::cmp::min(usize::MAX - 32, (1usize << 61) - 1),
    );
}

#[test]
fn c536_554_aegis256_sweep() {
    // rows 536, 538-544, 546, 547, 550-554
    let a = Aead::new("crypto_aead_aegis256");
    let mut rng = Rng::new(0x0A_E6_25_36);
    for &ml in MLENS {
        for &al in ADLENS {
            a.one(&mut rng, ml, al);
        }
    }
}

#[test]
fn c537_aegis256_ad_nonnull_adlen0() {
    // row 537
    let a = Aead::new("crypto_aead_aegis256");
    let mut rng = Rng::new(0x0A_E6_25_37);
    for &ml in &[0usize, 1, 16, 17, 1000] {
        a.ad_null_equiv(&mut rng, ml);
    }
}

#[test]
fn c545_548_549_aegis256_pointer_modes() {
    // rows 545 (clen_p NULL), 548 (clen == 32), 549 (mlen_p NULL),
    // 551 (maclen_p NULL/non-NULL)
    let a = Aead::new("crypto_aead_aegis256");
    let mut rng = Rng::new(0x0A_E6_25_45);
    a.one(&mut rng, 0, 0);
    for &ml in &[1000usize, 2048, 2049] {
        a.one(&mut rng, ml, 1000);
    }
}

// ===========================================================================
// aes256gcm — CONFIGS rows 555-567 (unavailable in this build)
// ===========================================================================
#[test]
fn c555_aes256gcm_is_available() {
    // row 555 — portable build: 0
    unsafe {
        let (c, r) = fpair::<IntFn>("crypto_aead_aes256gcm_is_available");
        let (a, b) = (c(), r());
        eq_i32("aes256gcm_is_available", a, b);
        assert_eq!(a, 0, "expected 0 in a build without HW AES");
    }
}

#[test]
fn c556_557_aes256gcm_constants_and_keygen() {
    // rows 556, 557
    sz_is("crypto_aead_aes256gcm_keybytes", 32);
    sz_is("crypto_aead_aes256gcm_nsecbytes", 0);
    sz_is("crypto_aead_aes256gcm_npubbytes", 12);
    sz_is("crypto_aead_aes256gcm_abytes", 16);
    sz_is("crypto_aead_aes256gcm_statebytes", 512);
    sz_is(
        "crypto_aead_aes256gcm_messagebytes_max",
        core::cmp::min(usize::MAX - 16, 16 * ((1usize << 32) - 2)),
    );
    keygen_check("crypto_aead_aes256gcm_keygen", 32);
}

/// 16-byte aligned scratch for `crypto_aead_aes256gcm_state`.
struct GcmState(Vec<u128>, usize);
impl GcmState {
    fn new(n: usize) -> GcmState {
        GcmState(vec![0u128; (n + 15) / 16], n)
    }
    fn ptr(&mut self) -> *mut u8 {
        self.0.as_mut_ptr() as *mut u8
    }
    fn bytes(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.0.as_ptr() as *const u8, self.1) }
    }
}

#[test]
fn c558_567_aes256gcm_all_entry_points_enosys() {
    // rows 558-567 — every aes256gcm entry point is a hard -1/ENOSYS rejection
    // in this build (this also pins ERRORS rows 426-434); the constant getters
    // and `keygen` still work (rows 556/557).
    let stb = sz("crypto_aead_aes256gcm_statebytes");
    let mut rng = Rng::new(0xAE5_256);
    let k = rng.bytes(32);
    let npub = rng.bytes(12);
    unsafe {
        let (enc_c, enc_r) = fpair::<EncFn>("crypto_aead_aes256gcm_encrypt");
        let (dec_c, dec_r) = fpair::<DecFn>("crypto_aead_aes256gcm_decrypt");
        let (ed_c, ed_r) = fpair::<EncDetFn>("crypto_aead_aes256gcm_encrypt_detached");
        let (dd_c, dd_r) = fpair::<DecDetFn>("crypto_aead_aes256gcm_decrypt_detached");
        let (bn_c, bn_r) = fpair::<BeforenmFn>("crypto_aead_aes256gcm_beforenm");
        let (ean_c, ean_r) = fpair::<EncFn>("crypto_aead_aes256gcm_encrypt_afternm");
        let (edan_c, edan_r) =
            fpair::<EncDetFn>("crypto_aead_aes256gcm_encrypt_detached_afternm");
        let (dan_c, dan_r) = fpair::<DecFn>("crypto_aead_aes256gcm_decrypt_afternm");
        let (ddan_c, ddan_r) =
            fpair::<DecDetFn>("crypto_aead_aes256gcm_decrypt_detached_afternm");

        // --- row 562: beforenm fails and leaves the state untouched ----------
        let mut st_c = GcmState::new(stb);
        let mut st_r = GcmState::new(stb);
        let stp_c: *const u8 = st_c.ptr() as *const u8;
        let stp_r: *const u8 = st_r.ptr() as *const u8;
        set_errno(0);
        let ra = bn_c(st_c.ptr(), k.as_ptr());
        let ea = errno();
        set_errno(0);
        let rb = bn_r(st_r.ptr(), k.as_ptr());
        let eb = errno();
        eq_i32("beforenm rc", ra, rb);
        assert_eq!(ra, -1, "row 562: beforenm must fail in this build");
        assert_eq!((ea, eb), (ENOSYS, ENOSYS), "row 562: beforenm errno");
        eq_bytes("row562 beforenm state", st_c.bytes(), st_r.bytes());
        assert!(
            st_c.bytes().iter().all(|&x| x == 0),
            "row 562: state must not be initialized"
        );

        // --- rows 558/559/563/564/567: encrypt legs -------------------------
        for &ml in &[0usize, 1, 15, 16, 17, 63, 64, 65, 128, 1000] {
            for &al in &[0usize, 1, 16, 17, 1000] {
                let m = rng.bytes(ml);
                let ad = rng.bytes(al);
                let adp = if al == 0 { ptr::null() } else { ad.as_ptr() };
                let mut ca = vec![0x11u8; ml + 16 + G];
                let mut cb = ca.clone();

                // combined encrypt (row 558) and encrypt_afternm (rows 563/567)
                for (i, (f, g)) in [(enc_c, enc_r), (ean_c, ean_r)].into_iter().enumerate() {
                    let last: *const u8 = if i == 0 { k.as_ptr() } else { stp_c };
                    let last_r: *const u8 = if i == 0 { k.as_ptr() } else { stp_r };
                    let mut la = 0xdead_beefu64;
                    let mut lb = 0xdead_beefu64;
                    set_errno(0);
                    let ra = f(
                        ca.as_mut_ptr(),
                        &mut la,
                        m.as_ptr(),
                        ml as u64,
                        adp,
                        al as u64,
                        ptr::null(),
                        npub.as_ptr(),
                        last,
                    );
                    let ea = errno();
                    set_errno(0);
                    let rb = g(
                        cb.as_mut_ptr(),
                        &mut lb,
                        m.as_ptr(),
                        ml as u64,
                        adp,
                        al as u64,
                        ptr::null(),
                        npub.as_ptr(),
                        last_r,
                    );
                    let eb = errno();
                    eq_i32(&format!("gcm enc[{i}] m={ml} ad={al} rc"), ra, rb);
                    assert_eq!(ra, -1);
                    assert_eq!((ea, eb), (ENOSYS, ENOSYS), "gcm enc[{i}] errno");
                    assert_eq!(la, lb, "gcm enc[{i}] clen_p");
                    assert_eq!(la, 0xdead_beef, "gcm enc[{i}]: clen_p must not be written");
                    eq_bytes(&format!("gcm enc[{i}] out"), &ca, &cb);
                }

                // detached encrypt (row 559) and _detached_afternm (row 564),
                // with maclen_p both NULL and non-NULL
                for (i, (f, g)) in [(ed_c, ed_r), (edan_c, edan_r)].into_iter().enumerate() {
                    for maclen_p_null in [false, true] {
                        let last: *const u8 = if i == 0 { k.as_ptr() } else { stp_c };
                        let last_r: *const u8 = if i == 0 { k.as_ptr() } else { stp_r };
                        let mut mca = vec![0x22u8; 16 + G];
                        let mut mcb = mca.clone();
                        let mut la = 0xdead_beefu64;
                        let mut lb = 0xdead_beefu64;
                        let (pa, pb) = if maclen_p_null {
                            (ptr::null_mut(), ptr::null_mut())
                        } else {
                            (&mut la as *mut u64, &mut lb as *mut u64)
                        };
                        set_errno(0);
                        let ra = f(
                            ca.as_mut_ptr(),
                            mca.as_mut_ptr(),
                            pa,
                            m.as_ptr(),
                            ml as u64,
                            adp,
                            al as u64,
                            ptr::null(),
                            npub.as_ptr(),
                            last,
                        );
                        let ea = errno();
                        set_errno(0);
                        let rb = g(
                            cb.as_mut_ptr(),
                            mcb.as_mut_ptr(),
                            pb,
                            m.as_ptr(),
                            ml as u64,
                            adp,
                            al as u64,
                            ptr::null(),
                            npub.as_ptr(),
                            last_r,
                        );
                        let eb = errno();
                        eq_i32(&format!("gcm encdet[{i}] rc"), ra, rb);
                        assert_eq!(ra, -1);
                        assert_eq!((ea, eb), (ENOSYS, ENOSYS));
                        assert_eq!(la, lb, "gcm encdet[{i}] maclen_p");
                        assert_eq!(la, 0xdead_beef, "maclen_p must not be written");
                        eq_bytes(&format!("gcm encdet[{i}] mac"), &mca, &mcb);
                        eq_bytes(&format!("gcm encdet[{i}] c"), &ca, &cb);
                    }
                }
            }
        }

        // --- rows 560/561/565/566: decrypt legs -----------------------------
        for &cl in &[0usize, 15, 16, 17, 32, 1016] {
            let ct = rng.bytes(cl);
            let ad = rng.bytes(17);
            let mac = rng.bytes(16);
            let mut ma = vec![0x44u8; cl + G];
            let mut mb = ma.clone();
            for (i, (f, g)) in [(dec_c, dec_r), (dan_c, dan_r)].into_iter().enumerate() {
                for mlen_p_null in [false, true] {
                    let last: *const u8 = if i == 0 { k.as_ptr() } else { stp_c };
                    let last_r: *const u8 = if i == 0 { k.as_ptr() } else { stp_r };
                    let mut la = 0xdead_beefu64;
                    let mut lb = 0xdead_beefu64;
                    let (pa, pb) = if mlen_p_null {
                        (ptr::null_mut(), ptr::null_mut())
                    } else {
                        (&mut la as *mut u64, &mut lb as *mut u64)
                    };
                    set_errno(0);
                    let ra = f(
                        ma.as_mut_ptr(),
                        pa,
                        ptr::null_mut(),
                        ct.as_ptr(),
                        cl as u64,
                        ad.as_ptr(),
                        17,
                        npub.as_ptr(),
                        last,
                    );
                    let ea = errno();
                    set_errno(0);
                    let rb = g(
                        mb.as_mut_ptr(),
                        pb,
                        ptr::null_mut(),
                        ct.as_ptr(),
                        cl as u64,
                        ad.as_ptr(),
                        17,
                        npub.as_ptr(),
                        last_r,
                    );
                    let eb = errno();
                    eq_i32(&format!("gcm dec[{i}] clen={cl} rc"), ra, rb);
                    assert_eq!(ra, -1);
                    assert_eq!((ea, eb), (ENOSYS, ENOSYS));
                    assert_eq!(la, lb, "gcm dec[{i}] mlen_p");
                    assert_eq!(la, 0xdead_beef, "mlen_p must not be written");
                    eq_bytes(&format!("gcm dec[{i}] m"), &ma, &mb);
                }
            }
            for (i, (f, g)) in [(dd_c, dd_r), (ddan_c, ddan_r)].into_iter().enumerate() {
                for m_null in [false, true] {
                    let last: *const u8 = if i == 0 { k.as_ptr() } else { stp_c };
                    let last_r: *const u8 = if i == 0 { k.as_ptr() } else { stp_r };
                    let (pa, pb) = if m_null {
                        (ptr::null_mut(), ptr::null_mut())
                    } else {
                        (ma.as_mut_ptr(), mb.as_mut_ptr())
                    };
                    set_errno(0);
                    let ra = f(
                        pa,
                        ptr::null_mut(),
                        ct.as_ptr(),
                        cl as u64,
                        mac.as_ptr(),
                        ad.as_ptr(),
                        17,
                        npub.as_ptr(),
                        last,
                    );
                    let ea = errno();
                    set_errno(0);
                    let rb = g(
                        pb,
                        ptr::null_mut(),
                        ct.as_ptr(),
                        cl as u64,
                        mac.as_ptr(),
                        ad.as_ptr(),
                        17,
                        npub.as_ptr(),
                        last_r,
                    );
                    let eb = errno();
                    eq_i32(&format!("gcm decdet[{i}] rc"), ra, rb);
                    assert_eq!(ra, -1);
                    assert_eq!((ea, eb), (ENOSYS, ENOSYS));
                    eq_bytes(&format!("gcm decdet[{i}] m"), &ma, &mb);
                }
            }
        }
        // row 567: the shared state buffer stayed untouched through every
        // afternm call, in both libraries.
        eq_bytes("row567 gcm state after afternm reuse", st_c.bytes(), st_r.bytes());
        assert!(st_c.bytes().iter().all(|&x| x == 0));
    }
}

// ===========================================================================
// chacha20poly1305 (original) — CONFIGS rows 568-582
// ===========================================================================
#[test]
fn c568_chacha20poly1305_keygen() {
    // row 568
    keygen_check("crypto_aead_chacha20poly1305_keygen", 32);
}

#[test]
fn c569_chacha20poly1305_constants() {
    // row 569
    sz_is("crypto_aead_chacha20poly1305_keybytes", 32);
    sz_is("crypto_aead_chacha20poly1305_nsecbytes", 0);
    sz_is("crypto_aead_chacha20poly1305_npubbytes", 8);
    sz_is("crypto_aead_chacha20poly1305_abytes", 16);
    sz_is(
        "crypto_aead_chacha20poly1305_messagebytes_max",
        usize::MAX - 16,
    );
}

#[test]
fn c570_582_chacha20poly1305_sweep() {
    // rows 570, 572-575, 577-582
    let a = Aead::new("crypto_aead_chacha20poly1305");
    let mut rng = Rng::new(0xC20_A13_05);
    for &ml in MLENS {
        for &al in ADLENS {
            a.one(&mut rng, ml, al);
        }
    }
}

#[test]
fn c571_chacha20poly1305_ad_nonnull_adlen0() {
    // row 571
    let a = Aead::new("crypto_aead_chacha20poly1305");
    let mut rng = Rng::new(0xC20_A13_71);
    for &ml in &[0usize, 1, 16, 64, 1000] {
        a.ad_null_equiv(&mut rng, ml);
    }
}

#[test]
fn c576_chacha20poly1305_chunk_boundary() {
    // row 576 — mlen crossing STREAM_POLY1305_CHUNK (131072)
    let a = Aead::new("crypto_aead_chacha20poly1305");
    let mut rng = Rng::new(0xC20_A13_76);
    for &ml in &[131071usize, 131072, 131073, 262144, 262145] {
        for &al in &[0usize, 17] {
            a.one(&mut rng, ml, al);
        }
    }
}

#[test]
fn c578_chacha20poly1305_clen_eq_abytes() {
    // row 578 — clen == 16 exactly (empty message)
    let a = Aead::new("crypto_aead_chacha20poly1305");
    let mut rng = Rng::new(0xC20_A13_78);
    for &al in &[0usize, 1, 16, 17, 1000] {
        a.one(&mut rng, 0, al);
    }
}

// ===========================================================================
// chacha20poly1305_ietf — CONFIGS rows 583-595
// ===========================================================================
#[test]
fn c583_ietf_keygen() {
    // row 583
    keygen_check("crypto_aead_chacha20poly1305_ietf_keygen", 32);
}

#[test]
fn c584_ietf_constants() {
    // row 584
    sz_is("crypto_aead_chacha20poly1305_ietf_keybytes", 32);
    sz_is("crypto_aead_chacha20poly1305_ietf_nsecbytes", 0);
    sz_is("crypto_aead_chacha20poly1305_ietf_npubbytes", 12);
    sz_is("crypto_aead_chacha20poly1305_ietf_abytes", 16);
    sz_is(
        "crypto_aead_chacha20poly1305_ietf_messagebytes_max",
        core::cmp::min(usize::MAX - 16, 64 * ((1usize << 32) - 1)),
    );
}

#[test]
fn c585_595_ietf_sweep() {
    // rows 585, 587, 588, 589, 590, 592, 593, 594, 595
    let a = Aead::new("crypto_aead_chacha20poly1305_ietf");
    let mut rng = Rng::new(0x1E7F_0585);
    for &ml in MLENS {
        for &al in ADLENS {
            a.one(&mut rng, ml, al);
        }
    }
}

#[test]
fn c586_ietf_ad_nonnull_adlen0() {
    // row 586 — (0x10-0)&0xf == 0, no pad emitted
    let a = Aead::new("crypto_aead_chacha20poly1305_ietf");
    let mut rng = Rng::new(0x1E7F_0586);
    for &ml in &[0usize, 1, 15, 16, 17, 1000] {
        a.ad_null_equiv(&mut rng, ml);
    }
}

#[test]
fn c591_ietf_chunk_boundary() {
    // row 591 — 32-bit ic chunk advance across STREAM_POLY1305_CHUNK
    let a = Aead::new("crypto_aead_chacha20poly1305_ietf");
    let mut rng = Rng::new(0x1E7F_0591);
    for &ml in &[131071usize, 131072, 131073, 262144] {
        for &al in &[0usize, 17] {
            a.one(&mut rng, ml, al);
        }
    }
}

// ===========================================================================
// xchacha20poly1305_ietf — CONFIGS rows 596-608
// ===========================================================================
#[test]
fn c596_xchacha_keygen() {
    // row 596
    keygen_check("crypto_aead_xchacha20poly1305_ietf_keygen", 32);
}

#[test]
fn c597_xchacha_constants() {
    // row 597
    sz_is("crypto_aead_xchacha20poly1305_ietf_keybytes", 32);
    sz_is("crypto_aead_xchacha20poly1305_ietf_nsecbytes", 0);
    sz_is("crypto_aead_xchacha20poly1305_ietf_npubbytes", 24);
    sz_is("crypto_aead_xchacha20poly1305_ietf_abytes", 16);
    sz_is(
        "crypto_aead_xchacha20poly1305_ietf_messagebytes_max",
        usize::MAX - 16,
    );
}

#[test]
fn c598_607_xchacha_sweep() {
    // rows 598, 600, 601, 602, 604, 605, 606, 607
    let a = Aead::new("crypto_aead_xchacha20poly1305_ietf");
    let mut rng = Rng::new(0x8C_AA_CCA_20);
    for &ml in MLENS {
        for &al in ADLENS {
            a.one(&mut rng, ml, al);
        }
    }
}

#[test]
fn c599_xchacha_ad_nonnull_adlen0() {
    // row 599
    let a = Aead::new("crypto_aead_xchacha20poly1305_ietf");
    let mut rng = Rng::new(0x8C_AA_05_99);
    for &ml in &[0usize, 1, 16, 17, 1000] {
        a.ad_null_equiv(&mut rng, ml);
    }
}

#[test]
fn c603_xchacha_chunk_boundary() {
    // row 603 — ietf_ext stream, chunk loop crossing 131072
    let a = Aead::new("crypto_aead_xchacha20poly1305_ietf");
    let mut rng = Rng::new(0x8C_AA_06_03);
    for &ml in &[131071usize, 131072, 131073, 262144] {
        for &al in &[0usize, 17] {
            a.one(&mut rng, ml, al);
        }
    }
}

#[test]
fn c608_xchacha_vs_ietf_equivalence() {
    // row 608 — xchacha20poly1305_ietf == chacha20poly1305_ietf keyed with
    // hchacha20(npub[0..16), k) and nonce (0,0,0,0 || npub[16..24)).
    let x = Aead::new("crypto_aead_xchacha20poly1305_ietf");
    let i = Aead::new("crypto_aead_chacha20poly1305_ietf");
    let (hc_c, hc_r) = unsafe { fpair::<CoreFn>("crypto_core_hchacha20") };
    let mut rng = Rng::new(0x0608_E9);
    for &ml in &[0usize, 1, 16, 63, 64, 65, 1000] {
        for &al in &[0usize, 17] {
            let k = rng.bytes(32);
            let npub = rng.bytes(24);
            let m = rng.bytes(ml);
            let ad = rng.bytes(al);
            let mut k2c = [0u8; 32];
            let mut k2r = [0u8; 32];
            unsafe {
                hc_c(k2c.as_mut_ptr(), npub.as_ptr(), k.as_ptr(), ptr::null());
                hc_r(k2r.as_mut_ptr(), npub.as_ptr(), k.as_ptr(), ptr::null());
            }
            eq_bytes("row608 hchacha20 subkey", &k2c, &k2r);
            let mut npub2 = [0u8; 12];
            npub2[4..].copy_from_slice(&npub[16..24]);
            let xa = x.enc_cmp("row608 xchacha", &m, &ad, &npub, &k);
            let ia = i.enc_cmp("row608 ietf", &m, &ad, &npub2, &k2c);
            eq_bytes("row608 xchacha == ietf(hchacha20 key)", &xa, &ia);
        }
    }
}

// ===========================================================================
// crypto_secretbox — CONFIGS rows 609-623
// ===========================================================================
struct Sb {
    pfx: String,
    kb: usize,
    nb: usize,
    mb: usize,
    easy_c: SbEasy,
    easy_r: SbEasy,
    open_c: SbEasy,
    open_r: SbEasy,
    det_c: SbDet,
    det_r: SbDet,
    od_c: SbOpenDet,
    od_r: SbOpenDet,
}

impl Sb {
    fn new(pfx: &str, easy: &str, open_easy: &str, det: &str, open_det: &str) -> Sb {
        unsafe {
            let (easy_c, easy_r) = fpair::<SbEasy>(easy);
            let (open_c, open_r) = fpair::<SbEasy>(open_easy);
            let (det_c, det_r) = fpair::<SbDet>(det);
            let (od_c, od_r) = fpair::<SbOpenDet>(open_det);
            Sb {
                pfx: pfx.to_string(),
                kb: sz(&format!("{pfx}_keybytes")),
                nb: sz(&format!("{pfx}_noncebytes")),
                mb: sz(&format!("{pfx}_macbytes")),
                easy_c,
                easy_r,
                open_c,
                open_r,
                det_c,
                det_r,
                od_c,
                od_r,
            }
        }
    }

    fn one(&self, rng: &mut Rng, mlen: usize) {
        let label = format!("{} m={}", self.pfx, mlen);
        let mb = self.mb;
        let k = rng.bytes(self.kb);
        let n = rng.bytes(self.nb);
        let m = rng.bytes(mlen);
        let clen = mlen + mb;

        // --- easy ------------------------------------------------------------
        let mut a = vec![0xA5u8; clen + G];
        let mut b = vec![0xA5u8; clen + G];
        let (ra, rb) = unsafe {
            (
                (self.easy_c)(a.as_mut_ptr(), m.as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr()),
                (self.easy_r)(b.as_mut_ptr(), m.as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr()),
            )
        };
        eq_i32(&format!("{label}: easy rc"), ra, rb);
        eq_bytes(&format!("{label}: easy out"), &a, &b);
        assert_eq!(ra, 0);
        assert!(a[clen..].iter().all(|&x| x == 0xA5), "{label}: overrun");
        let ct = a[..clen].to_vec();

        // in-place easy: c == m - MACBYTES
        {
            let mut p = vec![0xA5u8; clen + G];
            p[mb..mb + mlen].copy_from_slice(&m);
            let mut q = p.clone();
            unsafe {
                let pp = p.as_mut_ptr();
                let qq = q.as_mut_ptr();
                let ra = (self.easy_c)(pp, pp.add(mb), mlen as u64, n.as_ptr(), k.as_ptr());
                let rb = (self.easy_r)(qq, qq.add(mb), mlen as u64, n.as_ptr(), k.as_ptr());
                eq_i32(&format!("{label}: in-place easy rc"), ra, rb);
            }
            eq_bytes(&format!("{label}: in-place easy"), &p, &q);
            eq_bytes(&format!("{label}: in-place easy == ct"), &p[..clen], &ct);
        }

        // --- detached --------------------------------------------------------
        {
            let mut ca = vec![0xA5u8; mlen + G];
            let mut cb = vec![0xA5u8; mlen + G];
            let mut ma = vec![0x5Au8; mb + G];
            let mut mbf = vec![0x5Au8; mb + G];
            let (ra, rb) = unsafe {
                (
                    (self.det_c)(
                        ca.as_mut_ptr(),
                        ma.as_mut_ptr(),
                        m.as_ptr(),
                        mlen as u64,
                        n.as_ptr(),
                        k.as_ptr(),
                    ),
                    (self.det_r)(
                        cb.as_mut_ptr(),
                        mbf.as_mut_ptr(),
                        m.as_ptr(),
                        mlen as u64,
                        n.as_ptr(),
                        k.as_ptr(),
                    ),
                )
            };
            eq_i32(&format!("{label}: detached rc"), ra, rb);
            eq_bytes(&format!("{label}: detached c"), &ca, &cb);
            eq_bytes(&format!("{label}: detached mac"), &ma, &mbf);
            eq_bytes(&format!("{label}: mac == easy prefix"), &ma[..mb], &ct[..mb]);
            eq_bytes(&format!("{label}: c == easy suffix"), &ca[..mlen], &ct[mb..]);
        }

        // partially overlapping detached (c = m + 1) -> memmove fixup path
        if mlen > 1 {
            let mut p = vec![0u8; mlen + 1 + G];
            p[1..1 + mlen].copy_from_slice(&m);
            let mut q = p.clone();
            let mut ma = vec![0x5Au8; mb + G];
            let mut mbf = vec![0x5Au8; mb + G];
            unsafe {
                let pp = p.as_mut_ptr();
                let qq = q.as_mut_ptr();
                let ra = (self.det_c)(
                    pp,
                    ma.as_mut_ptr(),
                    pp.add(1),
                    mlen as u64,
                    n.as_ptr(),
                    k.as_ptr(),
                );
                let rb = (self.det_r)(
                    qq,
                    mbf.as_mut_ptr(),
                    qq.add(1),
                    mlen as u64,
                    n.as_ptr(),
                    k.as_ptr(),
                );
                eq_i32(&format!("{label}: overlap detached rc"), ra, rb);
            }
            eq_bytes(&format!("{label}: overlap detached out"), &p, &q);
            eq_bytes(&format!("{label}: overlap detached mac"), &ma, &mbf);
        }

        // --- open_easy -------------------------------------------------------
        let ok = self.open_easy_cmp(&format!("{label} open"), &ct, &n, &k);
        assert_eq!(ok, 0, "{label}: round trip");
        {
            let mut a = vec![0x33u8; mlen + G];
            let ra = unsafe {
                (self.open_c)(a.as_mut_ptr(), ct.as_ptr(), clen as u64, n.as_ptr(), k.as_ptr())
            };
            assert_eq!(ra, 0);
            eq_bytes(&format!("{label}: recovered"), &m, &a[..mlen]);
        }
        // in-place open: m == c
        {
            let mut p = vec![0x33u8; clen + G];
            p[..clen].copy_from_slice(&ct);
            let mut q = p.clone();
            unsafe {
                let pp = p.as_mut_ptr();
                let qq = q.as_mut_ptr();
                let ra = (self.open_c)(pp, pp, clen as u64, n.as_ptr(), k.as_ptr());
                let rb = (self.open_r)(qq, qq, clen as u64, n.as_ptr(), k.as_ptr());
                eq_i32(&format!("{label}: in-place open rc"), ra, rb);
                assert_eq!(ra, 0);
            }
            eq_bytes(&format!("{label}: in-place open"), &p, &q);
            eq_bytes(&format!("{label}: in-place plaintext"), &m, &p[..mlen]);
        }
        // tampered
        {
            let mut bad = ct.clone();
            bad[rng.below(mb)] ^= 0x01;
            assert_eq!(self.open_easy_cmp(&format!("{label} bad-mac"), &bad, &n, &k), -1);
            if mlen > 0 {
                let mut bad = ct.clone();
                bad[mb + rng.below(mlen)] ^= 0x08;
                assert_eq!(self.open_easy_cmp(&format!("{label} bad-ct"), &bad, &n, &k), -1);
            }
            let mut badn = n.clone();
            badn[rng.below(self.nb)] ^= 0x04;
            assert_eq!(self.open_easy_cmp(&format!("{label} bad-n"), &ct, &badn, &k), -1);
            let mut badk = k.clone();
            badk[rng.below(self.kb)] ^= 0x02;
            assert_eq!(self.open_easy_cmp(&format!("{label} bad-k"), &ct, &n, &badk), -1);
        }

        // --- open_detached ---------------------------------------------------
        assert_eq!(
            self.open_det_cmp(&format!("{label} od"), &ct[mb..], &ct[..mb], &n, &k, false),
            0
        );
        assert_eq!(
            self.open_det_cmp(&format!("{label} od m=NULL"), &ct[mb..], &ct[..mb], &n, &k, true),
            0
        );
        {
            let mut badmac = ct[..mb].to_vec();
            badmac[rng.below(mb)] ^= 0x80;
            assert_eq!(
                self.open_det_cmp(&format!("{label} od bad"), &ct[mb..], &badmac, &n, &k, false),
                -1
            );
            assert_eq!(
                self.open_det_cmp(
                    &format!("{label} od bad m=NULL"),
                    &ct[mb..],
                    &badmac,
                    &n,
                    &k,
                    true
                ),
                -1
            );
        }
    }

    fn open_easy_cmp(&self, label: &str, ct: &[u8], n: &[u8], k: &[u8]) -> c_int {
        let clen = ct.len();
        let mlen = clen.saturating_sub(self.mb);
        let mut a = vec![0x33u8; mlen + G];
        let mut b = vec![0x33u8; mlen + G];
        let (ra, rb) = unsafe {
            (
                (self.open_c)(a.as_mut_ptr(), ct.as_ptr(), clen as u64, n.as_ptr(), k.as_ptr()),
                (self.open_r)(b.as_mut_ptr(), ct.as_ptr(), clen as u64, n.as_ptr(), k.as_ptr()),
            )
        };
        eq_i32(&format!("{label}: open_easy rc"), ra, rb);
        eq_bytes(&format!("{label}: open_easy m"), &a, &b);
        ra
    }

    fn open_det_cmp(
        &self,
        label: &str,
        c: &[u8],
        mac: &[u8],
        n: &[u8],
        k: &[u8],
        m_null: bool,
    ) -> c_int {
        let clen = c.len();
        let mut a = vec![0x77u8; clen + G];
        let mut b = vec![0x77u8; clen + G];
        let (pa, pb) = if m_null {
            (ptr::null_mut(), ptr::null_mut())
        } else {
            (a.as_mut_ptr(), b.as_mut_ptr())
        };
        let (ra, rb) = unsafe {
            (
                (self.od_c)(pa, c.as_ptr(), mac.as_ptr(), clen as u64, n.as_ptr(), k.as_ptr()),
                (self.od_r)(pb, c.as_ptr(), mac.as_ptr(), clen as u64, n.as_ptr(), k.as_ptr()),
            )
        };
        eq_i32(&format!("{label}: open_detached rc"), ra, rb);
        eq_bytes(&format!("{label}: open_detached m"), &a, &b);
        if m_null {
            assert!(a.iter().all(|&x| x == 0x77), "{label}: verify-only wrote m");
        }
        ra
    }
}

fn secretbox_default() -> Sb {
    Sb::new(
        "crypto_secretbox",
        "crypto_secretbox_easy",
        "crypto_secretbox_open_easy",
        "crypto_secretbox_detached",
        "crypto_secretbox_open_detached",
    )
}

#[test]
fn c609_secretbox_keygen() {
    // row 609
    keygen_check("crypto_secretbox_keygen", 32);
}

#[test]
fn c610_secretbox_constants() {
    // row 610
    sz_is("crypto_secretbox_keybytes", 32);
    sz_is("crypto_secretbox_noncebytes", 24);
    sz_is("crypto_secretbox_macbytes", 16);
    sz_is("crypto_secretbox_zerobytes", 32);
    sz_is("crypto_secretbox_boxzerobytes", 16);
    sz_is("crypto_secretbox_messagebytes_max", usize::MAX - 16);
    str_is("crypto_secretbox_primitive", "xsalsa20poly1305");
}

#[test]
fn c611_619_secretbox_easy_detached_sweep() {
    // rows 611, 612, 614-619 (incl. the mlen0 = 64-ZEROBYTES = 32 split point,
    // row 617)
    let sb = secretbox_default();
    let mut rng = Rng::new(0x5B_06_11);
    for &ml in &[
        0usize, 1, 15, 16, 17, 30, 31, 32, 33, 34, 63, 64, 65, 127, 128, 1000,
    ] {
        sb.one(&mut rng, ml);
    }
}

#[test]
fn c613_secretbox_chunk_boundary() {
    // row 613 — mlen crossing STREAM_POLY1305_CHUNK (131072)
    let sb = secretbox_default();
    let mut rng = Rng::new(0x5B_06_13);
    for &ml in &[131071usize, 131072, 131073, 262144] {
        sb.one(&mut rng, ml);
    }
}

#[test]
fn c620_622_secretbox_nacl_zero_padded_form() {
    // rows 620, 621, 622 — the low-level NaCl API with ZEROBYTES=32 leading
    // zero bytes on m and BOXZEROBYTES=16 leading zero bytes on c, and the
    // cross-check against crypto_secretbox_easy.
    let zb = sz_is("crypto_secretbox_zerobytes", 32);
    let bzb = sz_is("crypto_secretbox_boxzerobytes", 16);
    let mb = 16usize;
    let sb = secretbox_default();
    let mut rng = Rng::new(0x5B_06_20);
    unsafe {
        for name in [
            ("crypto_secretbox", "crypto_secretbox_open"),
            (
                "crypto_secretbox_xsalsa20poly1305",
                "crypto_secretbox_xsalsa20poly1305_open",
            ),
        ] {
            let (sb_c, sb_r) = fpair::<SbEasy>(name.0);
            let (op_c, op_r) = fpair::<SbEasy>(name.1);
            for &payload in &[0usize, 1, 15, 16, 17, 31, 32, 33, 64, 65, 1000] {
                let mlen = zb + payload;
                let k = rng.bytes(32);
                let n = rng.bytes(24);
                let mut m = vec![0u8; mlen];
                let body = rng.bytes(payload);
                m[zb..].copy_from_slice(&body);
                let label = format!("{} mlen={}", name.0, mlen);

                let mut a = vec![0xA5u8; mlen + G];
                let mut b = vec![0xA5u8; mlen + G];
                let ra = sb_c(a.as_mut_ptr(), m.as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr());
                let rb = sb_r(b.as_mut_ptr(), m.as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr());
                eq_i32(&format!("{label}: rc"), ra, rb);
                eq_bytes(&format!("{label}: out"), &a, &b);
                assert_eq!(ra, 0);
                assert!(a[mlen..].iter().all(|&x| x == 0xA5), "{label}: overrun");
                assert!(
                    a[..bzb].iter().all(|&x| x == 0),
                    "{label}: first BOXZEROBYTES must be zeroed"
                );

                // row 622: cross-check against crypto_secretbox_easy
                let mut ez = vec![0u8; payload + mb + G];
                let re = (sb.easy_c)(
                    ez.as_mut_ptr(),
                    body.as_ptr(),
                    payload as u64,
                    n.as_ptr(),
                    k.as_ptr(),
                );
                assert_eq!(re, 0);
                eq_bytes(&format!("{label}: mac == easy mac"), &a[bzb..zb], &ez[..mb]);
                eq_bytes(
                    &format!("{label}: ct == easy ct"),
                    &a[zb..mlen],
                    &ez[mb..mb + payload],
                );

                // row 621: open
                let ct = a[..mlen].to_vec();
                let mut oa = vec![0x33u8; mlen + G];
                let mut ob = vec![0x33u8; mlen + G];
                let ra = op_c(oa.as_mut_ptr(), ct.as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr());
                let rb = op_r(ob.as_mut_ptr(), ct.as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr());
                eq_i32(&format!("{label}: open rc"), ra, rb);
                eq_bytes(&format!("{label}: open out"), &oa, &ob);
                assert_eq!(ra, 0);
                assert!(
                    oa[..zb].iter().all(|&x| x == 0),
                    "{label}: open must zero the first ZEROBYTES"
                );
                eq_bytes(&format!("{label}: open payload"), &oa[zb..mlen], &body);

                // tampered mac at c[16..32)
                let mut bad = ct.clone();
                bad[bzb + rng.below(mb)] ^= 0x40;
                let mut oa = vec![0x33u8; mlen + G];
                let mut ob = vec![0x33u8; mlen + G];
                let ra = op_c(oa.as_mut_ptr(), bad.as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr());
                let rb = op_r(ob.as_mut_ptr(), bad.as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr());
                eq_i32(&format!("{label}: open bad rc"), ra, rb);
                eq_bytes(&format!("{label}: open bad out"), &oa, &ob);
                assert_eq!(ra, -1);
                assert!(oa.iter().all(|&x| x == 0x33), "{label}: m must be untouched");
            }
        }
    }
}

#[test]
fn c623_secretbox_xsalsa20poly1305_getters_and_keygen() {
    // row 623
    sz_is("crypto_secretbox_xsalsa20poly1305_keybytes", 32);
    sz_is("crypto_secretbox_xsalsa20poly1305_noncebytes", 24);
    sz_is("crypto_secretbox_xsalsa20poly1305_zerobytes", 32);
    sz_is("crypto_secretbox_xsalsa20poly1305_boxzerobytes", 16);
    sz_is("crypto_secretbox_xsalsa20poly1305_macbytes", 16);
    sz_is(
        "crypto_secretbox_xsalsa20poly1305_messagebytes_max",
        usize::MAX - 16,
    );
    keygen_check("crypto_secretbox_xsalsa20poly1305_keygen", 32);
}

// ===========================================================================
// crypto_secretbox_xchacha20poly1305 — CONFIGS rows 624-631
// ===========================================================================
#[test]
fn c624_secretbox_xchacha_constants() {
    // row 624
    sz_is("crypto_secretbox_xchacha20poly1305_keybytes", 32);
    sz_is("crypto_secretbox_xchacha20poly1305_noncebytes", 24);
    sz_is("crypto_secretbox_xchacha20poly1305_macbytes", 16);
    sz_is(
        "crypto_secretbox_xchacha20poly1305_messagebytes_max",
        usize::MAX - 16,
    );
    assert!(
        !has_sym("crypto_secretbox_xchacha20poly1305_keygen"),
        "row 624: this variant has no keygen"
    );
}

#[test]
fn c625_631_secretbox_xchacha_sweep() {
    // rows 625, 626, 627, 628, 629, 630, 631 (the mlen 30..34 window straddles
    // the `mlen0 = min(mlen, 64-32)` block0 split, and the C only runs the
    // block0 keystream over `mlen0 + ZEROBYTES` bytes on the seal side)
    let sb = Sb::new(
        "crypto_secretbox_xchacha20poly1305",
        "crypto_secretbox_xchacha20poly1305_easy",
        "crypto_secretbox_xchacha20poly1305_open_easy",
        "crypto_secretbox_xchacha20poly1305_detached",
        "crypto_secretbox_xchacha20poly1305_open_detached",
    );
    let mut rng = Rng::new(0x5B_CC_25);
    for &ml in &[
        0usize, 1, 15, 16, 17, 30, 31, 32, 33, 34, 63, 64, 65, 127, 128, 1000, 131073,
    ] {
        sb.one(&mut rng, ml);
    }
}

// ===========================================================================
// crypto_secretstream — CONFIGS rows 632-655
// ===========================================================================
struct SsState {
    buf: Vec<u64>,
    n: usize,
}
impl SsState {
    fn new(n: usize) -> SsState {
        SsState {
            buf: vec![0u64; (n + 7) / 8 + 2],
            n,
        }
    }
    fn ptr(&mut self) -> *mut u8 {
        self.buf.as_mut_ptr() as *mut u8
    }
    fn bytes(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.buf.as_ptr() as *const u8, self.n) }
    }
    /// The bytes past the state proper must stay zero (overrun canary).
    fn canary_ok(&self) -> bool {
        let all = unsafe {
            std::slice::from_raw_parts(self.buf.as_ptr() as *const u8, self.buf.len() * 8)
        };
        all[self.n..].iter().all(|&x| x == 0)
    }
}

struct Ss {
    stb: usize,
    hb: usize,
    ab: usize,
    kb: usize,
    ipush_c: SsInitPush,
    ipush_r: SsInitPush,
    ipull_c: SsInitPull,
    ipull_r: SsInitPull,
    push_c: SsPush,
    push_r: SsPush,
    pull_c: SsPull,
    pull_r: SsPull,
    rekey_c: SsRekey,
    rekey_r: SsRekey,
}

impl Ss {
    fn new() -> Ss {
        unsafe {
            let p = "crypto_secretstream_xchacha20poly1305";
            let (ipush_c, ipush_r) = fpair::<SsInitPush>(&format!("{p}_init_push"));
            let (ipull_c, ipull_r) = fpair::<SsInitPull>(&format!("{p}_init_pull"));
            let (push_c, push_r) = fpair::<SsPush>(&format!("{p}_push"));
            let (pull_c, pull_r) = fpair::<SsPull>(&format!("{p}_pull"));
            let (rekey_c, rekey_r) = fpair::<SsRekey>(&format!("{p}_rekey"));
            Ss {
                stb: sz(&format!("{p}_statebytes")),
                hb: sz(&format!("{p}_headerbytes")),
                ab: sz(&format!("{p}_abytes")),
                kb: sz(&format!("{p}_keybytes")),
                ipush_c,
                ipush_r,
                ipull_c,
                ipull_r,
                push_c,
                push_r,
                pull_c,
                pull_r,
                rekey_c,
                rekey_r,
            }
        }
    }

    /// init_push in both libraries with the same deterministic RNG stream;
    /// returns (state_c, state_r, header).
    fn init_push(&self, k: &[u8], seed: u64) -> (SsState, SsState, Vec<u8>) {
        install_det_random();
        let mut sc = SsState::new(self.stb);
        let mut sr = SsState::new(self.stb);
        let mut ha = vec![0xA5u8; self.hb + G];
        let mut hb = vec![0xA5u8; self.hb + G];
        det_reseed(seed);
        let ra = unsafe { (self.ipush_c)(sc.ptr(), ha.as_mut_ptr(), k.as_ptr()) };
        det_reseed(seed);
        let rb = unsafe { (self.ipush_r)(sr.ptr(), hb.as_mut_ptr(), k.as_ptr()) };
        eq_i32("init_push rc", ra, rb);
        assert_eq!(ra, 0);
        eq_bytes("init_push header", &ha, &hb);
        eq_bytes("init_push state", sc.bytes(), sr.bytes());
        assert!(ha[self.hb..].iter().all(|&x| x == 0xA5), "header overrun");
        assert!(sc.canary_ok() && sr.canary_ok(), "state overrun");
        ha.truncate(self.hb);
        (sc, sr, ha)
    }

    fn init_pull(&self, header: &[u8], k: &[u8]) -> (SsState, SsState) {
        let mut sc = SsState::new(self.stb);
        let mut sr = SsState::new(self.stb);
        let ra = unsafe { (self.ipull_c)(sc.ptr(), header.as_ptr(), k.as_ptr()) };
        let rb = unsafe { (self.ipull_r)(sr.ptr(), header.as_ptr(), k.as_ptr()) };
        eq_i32("init_pull rc", ra, rb);
        assert_eq!(ra, 0);
        eq_bytes("init_pull state", sc.bytes(), sr.bytes());
        assert!(sc.canary_ok() && sr.canary_ok(), "state overrun");
        (sc, sr)
    }

    /// push in both libraries; compares out, `*outlen_p` and the new state.
    fn push(
        &self,
        label: &str,
        sc: &mut SsState,
        sr: &mut SsState,
        m: &[u8],
        ad: &[u8],
        tag: u8,
        outlen_p_null: bool,
    ) -> Vec<u8> {
        let mlen = m.len();
        let olen = mlen + self.ab;
        let adp = if ad.is_empty() {
            ptr::null()
        } else {
            ad.as_ptr()
        };
        let mut a = vec![0xA5u8; olen + G];
        let mut b = vec![0xA5u8; olen + G];
        let mut la = 0xdead_beefu64;
        let mut lb = 0xdead_beefu64;
        let (pa, pb) = if outlen_p_null {
            (ptr::null_mut(), ptr::null_mut())
        } else {
            (&mut la as *mut u64, &mut lb as *mut u64)
        };
        let (ra, rb) = unsafe {
            (
                (self.push_c)(
                    sc.ptr(),
                    a.as_mut_ptr(),
                    pa,
                    m.as_ptr(),
                    mlen as u64,
                    adp,
                    ad.len() as u64,
                    tag,
                ),
                (self.push_r)(
                    sr.ptr(),
                    b.as_mut_ptr(),
                    pb,
                    m.as_ptr(),
                    mlen as u64,
                    adp,
                    ad.len() as u64,
                    tag,
                ),
            )
        };
        eq_i32(&format!("{label}: push rc"), ra, rb);
        assert_eq!(ra, 0, "{label}: push must succeed");
        eq_bytes(&format!("{label}: push out"), &a, &b);
        eq_bytes(&format!("{label}: push state"), sc.bytes(), sr.bytes());
        assert_eq!(la, lb, "{label}: outlen_p");
        if !outlen_p_null {
            assert_eq!(la, olen as u64, "{label}: outlen_p value");
        }
        assert!(a[olen..].iter().all(|&x| x == 0xA5), "{label}: push overrun");
        assert!(sc.canary_ok() && sr.canary_ok(), "{label}: state overrun");
        a.truncate(olen);
        a
    }

    /// pull in both libraries; compares m, `*mlen_p`, `*tag_p` and the state.
    fn pull(
        &self,
        label: &str,
        sc: &mut SsState,
        sr: &mut SsState,
        inbuf: &[u8],
        ad: &[u8],
        nulls: (bool, bool),
    ) -> (c_int, Vec<u8>, u8) {
        let inlen = inbuf.len();
        let mlen = inlen.saturating_sub(self.ab);
        let adp = if ad.is_empty() {
            ptr::null()
        } else {
            ad.as_ptr()
        };
        let mut a = vec![0x33u8; mlen + G];
        let mut b = vec![0x33u8; mlen + G];
        let mut la = 0xdead_beefu64;
        let mut lb = 0xdead_beefu64;
        let mut ta = 0x5Au8;
        let mut tb = 0x5Au8;
        let (pla, plb) = if nulls.0 {
            (ptr::null_mut(), ptr::null_mut())
        } else {
            (&mut la as *mut u64, &mut lb as *mut u64)
        };
        let (pta, ptb) = if nulls.1 {
            (ptr::null_mut(), ptr::null_mut())
        } else {
            (&mut ta as *mut u8, &mut tb as *mut u8)
        };
        let (ra, rb) = unsafe {
            (
                (self.pull_c)(
                    sc.ptr(),
                    a.as_mut_ptr(),
                    pla,
                    pta,
                    inbuf.as_ptr(),
                    inlen as u64,
                    adp,
                    ad.len() as u64,
                ),
                (self.pull_r)(
                    sr.ptr(),
                    b.as_mut_ptr(),
                    plb,
                    ptb,
                    inbuf.as_ptr(),
                    inlen as u64,
                    adp,
                    ad.len() as u64,
                ),
            )
        };
        eq_i32(&format!("{label}: pull rc"), ra, rb);
        eq_bytes(&format!("{label}: pull m"), &a, &b);
        eq_bytes(&format!("{label}: pull state"), sc.bytes(), sr.bytes());
        assert_eq!(la, lb, "{label}: mlen_p");
        assert_eq!(ta, tb, "{label}: tag_p");
        assert!(a[mlen..].iter().all(|&x| x == 0x33), "{label}: pull overrun");
        assert!(sc.canary_ok() && sr.canary_ok(), "{label}: state overrun");
        a.truncate(mlen);
        (ra, a, ta)
    }
}

#[test]
fn c632_secretstream_keygen() {
    // row 632
    keygen_check("crypto_secretstream_xchacha20poly1305_keygen", 32);
}

#[test]
fn c633_634_secretstream_constants() {
    // rows 633, 634
    let stb = sz("crypto_secretstream_xchacha20poly1305_statebytes");
    assert_eq!(stb, 52, "statebytes = sizeof(state) = 32+12+8");
    sz_is("crypto_secretstream_xchacha20poly1305_abytes", 17);
    sz_is("crypto_secretstream_xchacha20poly1305_headerbytes", 24);
    sz_is("crypto_secretstream_xchacha20poly1305_keybytes", 32);
    sz_is(
        "crypto_secretstream_xchacha20poly1305_messagebytes_max",
        core::cmp::min(usize::MAX - 17, 64 * ((1usize << 32) - 2)),
    );
    u8_is("crypto_secretstream_xchacha20poly1305_tag_message", 0x00);
    u8_is("crypto_secretstream_xchacha20poly1305_tag_push", 0x01);
    u8_is("crypto_secretstream_xchacha20poly1305_tag_rekey", 0x02);
    u8_is("crypto_secretstream_xchacha20poly1305_tag_final", 0x03);
}

#[test]
fn c635_637_secretstream_init() {
    // rows 635, 636, 637 — init_push state layout, init_pull reproducing it,
    // and init_pull accepting arbitrary headers.
    let ss = Ss::new();
    let (hc_c, hc_r) = unsafe { fpair::<CoreFn>("crypto_core_hchacha20") };
    let mut rng = Rng::new(0x55_06_35);
    for i in 0..8u64 {
        let k = rng.bytes(ss.kb);
        let (sc, _sr, header) = ss.init_push(&k, 0x1000 + i);
        // row 635: k = hchacha20(header[0..16), key), counter = 1,
        // inonce = header[16..24), _pad zeroed
        let mut sub_c = [0u8; 32];
        let mut sub_r = [0u8; 32];
        unsafe {
            hc_c(sub_c.as_mut_ptr(), header.as_ptr(), k.as_ptr(), ptr::null());
            hc_r(sub_r.as_mut_ptr(), header.as_ptr(), k.as_ptr(), ptr::null());
        }
        eq_bytes("row635 hchacha20", &sub_c, &sub_r);
        let st = sc.bytes();
        eq_bytes("row635 state->k", &st[..32], &sub_c);
        assert_eq!(&st[32..36], &[1u8, 0, 0, 0], "row635 counter");
        eq_bytes("row635 inonce", &st[36..44], &header[16..24]);
        assert!(st[44..52].iter().all(|&x| x == 0), "row635 _pad");

        // row 636: init_pull(header) reproduces the identical state
        let (pc, _pr) = ss.init_pull(&header, &k);
        eq_bytes("row636 init_pull == init_push state", sc.bytes(), pc.bytes());
    }
    // row 637: arbitrary headers accepted unconditionally
    let k = rng.bytes(ss.kb);
    for h in [
        vec![0u8; ss.hb],
        vec![0xffu8; ss.hb],
        rng.bytes(ss.hb),
        (0..ss.hb as u8).collect(),
    ] {
        let _ = ss.init_pull(&h, &k);
    }
}

#[test]
fn c638_651_secretstream_push_pull_sweep() {
    // rows 638, 639, 640, 642, 644, 645, 646, 647, 648, 649, 650, 651
    let ss = Ss::new();
    let mut rng = Rng::new(0x55_06_38);
    let tags: [u8; 3] = [0x00, 0x01, 0x03]; // MESSAGE, PUSH, FINAL (no rekey bit)
    for &ml in MLENS {
        for &al in &[0usize, 1, 15, 16, 17, 64, 1000] {
            for &tag in tags.iter() {
                let k = rng.bytes(ss.kb);
                let m = rng.bytes(ml);
                let ad = rng.bytes(al);
                let label = format!("ss m={ml} ad={al} tag={tag:#04x}");
                let (mut sc, mut sr, header) = ss.init_push(&k, rng.next_u64());
                let out = ss.push(&label, &mut sc, &mut sr, &m, &ad, tag, false);
                assert_eq!(out.len(), ml + ss.ab);

                // pull it back
                let (mut pc, mut pr) = ss.init_pull(&header, &k);
                let (rc, got, gtag) = ss.pull(&label, &mut pc, &mut pr, &out, &ad, (false, false));
                assert_eq!(rc, 0, "{label}: pull must succeed");
                eq_bytes(&format!("{label}: pull plaintext"), &m, &got);
                assert_eq!(gtag, tag, "{label}: recovered tag");
                // pull state must equal push state
                eq_bytes(&format!("{label}: push/pull state sync"), sc.bytes(), pc.bytes());

                // row 649: mlen_p / tag_p NULL
                for nulls in [(true, false), (false, true), (true, true)] {
                    let (mut pc2, mut pr2) = ss.init_pull(&header, &k);
                    let (rc, got2, _) =
                        ss.pull(&label, &mut pc2, &mut pr2, &out, &ad, nulls);
                    assert_eq!(rc, 0);
                    eq_bytes(&format!("{label}: pull nulls plaintext"), &m, &got2);
                }

                // row 644: outlen_p == NULL
                {
                    let (mut sc2, mut sr2) = ss.init_pull(&header, &k);
                    let out2 = ss.push(&label, &mut sc2, &mut sr2, &m, &ad, tag, true);
                    eq_bytes(&format!("{label}: outlen_p NULL out"), &out, &out2);
                }

                // row 645: in-place push, out == m - 1
                {
                    let (mut sc2, mut sr2) = ss.init_pull(&header, &k);
                    let olen = ml + ss.ab;
                    let mut a = vec![0xA5u8; olen + G];
                    a[1..1 + ml].copy_from_slice(&m);
                    let mut b = a.clone();
                    let adp = if al == 0 { ptr::null() } else { ad.as_ptr() };
                    unsafe {
                        let pa = a.as_mut_ptr();
                        let pb = b.as_mut_ptr();
                        let ra = (ss.push_c)(
                            sc2.ptr(),
                            pa,
                            ptr::null_mut(),
                            pa.add(1),
                            ml as u64,
                            adp,
                            al as u64,
                            tag,
                        );
                        let rb = (ss.push_r)(
                            sr2.ptr(),
                            pb,
                            ptr::null_mut(),
                            pb.add(1),
                            ml as u64,
                            adp,
                            al as u64,
                            tag,
                        );
                        eq_i32(&format!("{label}: in-place push rc"), ra, rb);
                    }
                    eq_bytes(&format!("{label}: in-place push"), &a, &b);
                    eq_bytes(&format!("{label}: in-place push == out"), &a[..olen], &out);
                    eq_bytes(&format!("{label}: in-place push state"), sc2.bytes(), sr2.bytes());
                }

                // row 651: in-place pull, m == in + 1
                {
                    let (mut pc2, mut pr2) = ss.init_pull(&header, &k);
                    let olen = ml + ss.ab;
                    let mut a = vec![0x33u8; olen + G];
                    a[..olen].copy_from_slice(&out);
                    let mut b = a.clone();
                    let adp = if al == 0 { ptr::null() } else { ad.as_ptr() };
                    unsafe {
                        let pa = a.as_mut_ptr();
                        let pb = b.as_mut_ptr();
                        let ra = (ss.pull_c)(
                            pc2.ptr(),
                            pa.add(1),
                            ptr::null_mut(),
                            ptr::null_mut(),
                            pa,
                            olen as u64,
                            adp,
                            al as u64,
                        );
                        let rb = (ss.pull_r)(
                            pr2.ptr(),
                            pb.add(1),
                            ptr::null_mut(),
                            ptr::null_mut(),
                            pb,
                            olen as u64,
                            adp,
                            al as u64,
                        );
                        eq_i32(&format!("{label}: in-place pull rc"), ra, rb);
                        assert_eq!(ra, 0);
                    }
                    eq_bytes(&format!("{label}: in-place pull"), &a, &b);
                    eq_bytes(&format!("{label}: in-place plaintext"), &a[1..1 + ml], &m);
                }
            }
        }
    }
}

#[test]
fn c641_643_secretstream_rekey_tag_and_arbitrary_tags() {
    // rows 641 (TAG_REKEY implicit rekey), 643 (arbitrary tag bytes accepted;
    // only bit 0x02 triggers the rekey).
    let ss = Ss::new();
    let mut rng = Rng::new(0x55_06_41);
    for &tag in &[
        0x00u8, 0x01, 0x02, 0x03, 0x04, 0x7f, 0x80, 0x82, 0xfe, 0xff,
    ] {
        let k = rng.bytes(ss.kb);
        let m1 = rng.bytes(37);
        let m2 = rng.bytes(41);
        let ad = rng.bytes(17);
        let label = format!("ss tag={tag:#04x}");
        let (mut sc, mut sr, header) = ss.init_push(&k, rng.next_u64());
        let o1 = ss.push(&label, &mut sc, &mut sr, &m1, &ad, tag, false);
        let o2 = ss.push(&label, &mut sc, &mut sr, &m2, &ad, 0, false);

        // the pull side must follow, tag byte round-trips verbatim
        let (mut pc, mut pr) = ss.init_pull(&header, &k);
        let (rc, g1, t1) = ss.pull(&label, &mut pc, &mut pr, &o1, &ad, (false, false));
        assert_eq!(rc, 0);
        eq_bytes(&format!("{label}: m1"), &m1, &g1);
        assert_eq!(t1, tag, "{label}: arbitrary tag must round-trip verbatim");
        let (rc, g2, t2) = ss.pull(&label, &mut pc, &mut pr, &o2, &ad, (false, false));
        assert_eq!(rc, 0);
        eq_bytes(&format!("{label}: m2"), &m2, &g2);
        assert_eq!(t2, 0);
        eq_bytes(&format!("{label}: state sync"), sc.bytes(), pc.bytes());

        // row 641: with the REKEY bit set, the SECOND message differs from the
        // stream produced when the bit is clear.
        let (mut ac, mut ar, _h2) = ss.init_push(&k, 0x777);
        let (mut bc, mut br, _h3) = ss.init_push(&k, 0x777);
        let _ = ss.push(&label, &mut ac, &mut ar, &m1, &ad, tag, false);
        let _ = ss.push(&label, &mut bc, &mut br, &m1, &ad, tag & !0x02, false);
        let x = ss.push(&label, &mut ac, &mut ar, &m2, &ad, 0, false);
        let y = ss.push(&label, &mut bc, &mut br, &m2, &ad, 0, false);
        if tag & 0x02 != 0 {
            assert_ne!(x, y, "{label}: REKEY bit must change the following stream");
        } else {
            assert_eq!(x, y, "{label}: no REKEY bit -> identical stream");
        }
    }
}

#[test]
fn c652_secretstream_multi_message_sequence() {
    // row 652 — multi-message sequences with mixed tags / lengths / ad
    let ss = Ss::new();
    let mut rng = Rng::new(0x55_06_52);
    let script: [u8; 7] = [0x00, 0x00, 0x01, 0x00, 0x02, 0x00, 0x03];
    for round in 0..6u64 {
        let k = rng.bytes(ss.kb);
        let (mut sc, mut sr, header) = ss.init_push(&k, 0x2000 + round);
        let (mut pc, mut pr) = ss.init_pull(&header, &k);
        let n = 5 + (round as usize % 6);
        for i in 0..n {
            let tag = script[i % script.len()];
            let ml = [0usize, 1, 17, 63, 64, 65, 128, 1000][i % 8];
            let al = [0usize, 1, 16, 17, 1000][i % 5];
            let m = rng.bytes(ml);
            let ad = rng.bytes(al);
            let label = format!("ss seq r={round} i={i} tag={tag:#04x}");
            let out = ss.push(&label, &mut sc, &mut sr, &m, &ad, tag, false);
            let (rc, got, gtag) = ss.pull(&label, &mut pc, &mut pr, &out, &ad, (false, false));
            assert_eq!(rc, 0, "{label}");
            eq_bytes(&format!("{label}: plaintext"), &m, &got);
            assert_eq!(gtag, tag);
            eq_bytes(&format!("{label}: state sync"), sc.bytes(), pc.bytes());
        }
    }
}

#[test]
fn c653_654_secretstream_explicit_rekey() {
    // rows 653, 654 — explicit rekey() on both sides, including immediately
    // after init_push and immediately after an implicit REKEY-tag rekey.
    let ss = Ss::new();
    let mut rng = Rng::new(0x55_06_53);
    for round in 0..5u64 {
        let k = rng.bytes(ss.kb);
        let (mut sc, mut sr, header) = ss.init_push(&k, 0x3000 + round);
        let (mut pc, mut pr) = ss.init_pull(&header, &k);

        // row 654: rekey immediately after init, before any message
        unsafe {
            (ss.rekey_c)(sc.ptr());
            (ss.rekey_r)(sr.ptr());
            (ss.rekey_c)(pc.ptr());
            (ss.rekey_r)(pr.ptr());
        }
        eq_bytes("rekey-after-init push state", sc.bytes(), sr.bytes());
        eq_bytes("rekey-after-init pull state", pc.bytes(), pr.bytes());
        eq_bytes("rekey-after-init sync", sc.bytes(), pc.bytes());

        for i in 0..4usize {
            let m = rng.bytes(17 + i * 13);
            let ad = rng.bytes(i);
            let label = format!("ss rekey r={round} i={i}");
            // implicit rekey via TAG_REKEY, then an immediate explicit rekey
            let tag = if i == 1 { 0x02u8 } else { 0x00u8 };
            let out = ss.push(&label, &mut sc, &mut sr, &m, &ad, tag, false);
            let (rc, got, _) = ss.pull(&label, &mut pc, &mut pr, &out, &ad, (false, false));
            assert_eq!(rc, 0);
            eq_bytes(&format!("{label}: plaintext"), &m, &got);
            eq_bytes(&format!("{label}: state sync"), sc.bytes(), pc.bytes());
            // row 653/654: explicit rekey at the same point on both sides
            unsafe {
                (ss.rekey_c)(sc.ptr());
                (ss.rekey_r)(sr.ptr());
                (ss.rekey_c)(pc.ptr());
                (ss.rekey_r)(pr.ptr());
            }
            eq_bytes(&format!("{label}: rekey push state"), sc.bytes(), sr.bytes());
            eq_bytes(&format!("{label}: rekey pull state"), pc.bytes(), pr.bytes());
            eq_bytes(&format!("{label}: rekey sync"), sc.bytes(), pc.bytes());
        }
    }
}

#[test]
fn c655_secretstream_long_run_counter_carry() {
    // row 655 — 400 messages so `sodium_increment(counter, 4)` carries out of
    // the low byte several times.
    let ss = Ss::new();
    let mut rng = Rng::new(0x55_06_55);
    let k = rng.bytes(ss.kb);
    let (mut sc, mut sr, header) = ss.init_push(&k, 0x4000);
    let (mut pc, mut pr) = ss.init_pull(&header, &k);
    for i in 0..400usize {
        let m = rng.bytes(i % 71);
        let ad = if i % 3 == 0 { rng.bytes(i % 19) } else { vec![] };
        let label = format!("ss long i={i}");
        let out = ss.push(&label, &mut sc, &mut sr, &m, &ad, 0, false);
        let (rc, got, _) = ss.pull(&label, &mut pc, &mut pr, &out, &ad, (false, false));
        assert_eq!(rc, 0, "{label}");
        eq_bytes(&format!("{label}: plaintext"), &m, &got);
        eq_bytes(&format!("{label}: state sync"), sc.bytes(), pc.bytes());
    }
    // after 400 pushes the 4-byte little-endian counter must read 401
    assert_eq!(&sc.bytes()[32..36], &401u32.to_le_bytes());
}

// ===========================================================================
// crypto_stream_* — CONFIGS rows 656-701
// ===========================================================================
const CLENS: &[usize] = &[0, 1, 63, 64, 65, 127, 128, 129, 1000];

struct StreamNoIc {
    name: String,
    kb: usize,
    nb: usize,
    st_c: StreamFn,
    st_r: StreamFn,
    xor_c: XorFn,
    xor_r: XorFn,
}

impl StreamNoIc {
    fn new(name: &str) -> StreamNoIc {
        unsafe {
            let (st_c, st_r) = fpair::<StreamFn>(name);
            let (xor_c, xor_r) = fpair::<XorFn>(&format!("{name}_xor"));
            StreamNoIc {
                name: name.to_string(),
                kb: sz(&format!("{name}_keybytes")),
                nb: sz(&format!("{name}_noncebytes")),
                st_c,
                st_r,
                xor_c,
                xor_r,
            }
        }
    }

    /// keystream form + `_xor` + in-place `_xor`, cross-checked against each
    /// other.
    fn sweep(&self, rng: &mut Rng) {
        for &cl in CLENS {
            let k = rng.bytes(self.kb);
            let n = rng.bytes(self.nb);
            let m = rng.bytes(cl);
            let label = format!("{} clen={}", self.name, cl);

            // keystream form: clen == 0 must leave the buffer untouched
            let mut a = vec![0xA5u8; cl + G];
            let mut b = vec![0xA5u8; cl + G];
            let (ra, rb) = unsafe {
                (
                    (self.st_c)(a.as_mut_ptr(), cl as u64, n.as_ptr(), k.as_ptr()),
                    (self.st_r)(b.as_mut_ptr(), cl as u64, n.as_ptr(), k.as_ptr()),
                )
            };
            eq_i32(&format!("{label}: stream rc"), ra, rb);
            assert_eq!(ra, 0);
            eq_bytes(&format!("{label}: keystream"), &a, &b);
            assert!(a[cl..].iter().all(|&x| x == 0xA5), "{label}: overrun");
            let ks = a[..cl].to_vec();

            // _xor must equal keystream XOR m
            let mut a = vec![0xA5u8; cl + G];
            let mut b = vec![0xA5u8; cl + G];
            let (ra, rb) = unsafe {
                (
                    (self.xor_c)(
                        a.as_mut_ptr(),
                        m.as_ptr(),
                        cl as u64,
                        n.as_ptr(),
                        k.as_ptr(),
                    ),
                    (self.xor_r)(
                        b.as_mut_ptr(),
                        m.as_ptr(),
                        cl as u64,
                        n.as_ptr(),
                        k.as_ptr(),
                    ),
                )
            };
            eq_i32(&format!("{label}: xor rc"), ra, rb);
            assert_eq!(ra, 0);
            eq_bytes(&format!("{label}: xor"), &a, &b);
            let want: Vec<u8> = ks.iter().zip(&m).map(|(x, y)| x ^ y).collect();
            eq_bytes(&format!("{label}: xor == keystream^m"), &a[..cl], &want);

            // in-place _xor
            let mut p = vec![0xA5u8; cl + G];
            p[..cl].copy_from_slice(&m);
            let mut q = p.clone();
            unsafe {
                let pp = p.as_mut_ptr();
                let qq = q.as_mut_ptr();
                let ra = (self.xor_c)(pp, pp, cl as u64, n.as_ptr(), k.as_ptr());
                let rb = (self.xor_r)(qq, qq, cl as u64, n.as_ptr(), k.as_ptr());
                eq_i32(&format!("{label}: in-place xor rc"), ra, rb);
            }
            eq_bytes(&format!("{label}: in-place xor"), &p, &q);
            eq_bytes(&format!("{label}: in-place == xor"), &p[..cl], &want);
        }
    }
}

/// `_xor_ic` sweep for a 64-bit-`ic` primitive.
fn xor_ic64_sweep(name: &str, kb: usize, nb: usize, ics: &[u64], mlens: &[usize], seed: u64) {
    let (f_c, f_r) = unsafe { fpair::<XorIc64Fn>(&format!("{name}_xor_ic")) };
    let (x_c, x_r) = unsafe { fpair::<XorFn>(&format!("{name}_xor")) };
    let mut rng = Rng::new(seed);
    for &ic in ics {
        for &ml in mlens {
            let k = rng.bytes(kb);
            let n = rng.bytes(nb);
            let m = rng.bytes(ml);
            let label = format!("{name}_xor_ic ic={ic:#x} mlen={ml}");
            let mut a = vec![0xA5u8; ml + G];
            let mut b = vec![0xA5u8; ml + G];
            let (ra, rb) = unsafe {
                (
                    f_c(a.as_mut_ptr(), m.as_ptr(), ml as u64, n.as_ptr(), ic, k.as_ptr()),
                    f_r(b.as_mut_ptr(), m.as_ptr(), ml as u64, n.as_ptr(), ic, k.as_ptr()),
                )
            };
            eq_i32(&format!("{label}: rc"), ra, rb);
            assert_eq!(ra, 0);
            eq_bytes(&format!("{label}: out"), &a, &b);
            assert!(a[ml..].iter().all(|&x| x == 0xA5), "{label}: overrun");

            if ic == 0 {
                // must be byte-identical to `_xor`
                let mut c = vec![0xA5u8; ml + G];
                let mut d = vec![0xA5u8; ml + G];
                unsafe {
                    x_c(c.as_mut_ptr(), m.as_ptr(), ml as u64, n.as_ptr(), k.as_ptr());
                    x_r(d.as_mut_ptr(), m.as_ptr(), ml as u64, n.as_ptr(), k.as_ptr());
                }
                eq_bytes(&format!("{label}: ic=0 vs _xor (C)"), &a, &c);
                eq_bytes(&format!("{label}: ic=0 vs _xor (Rust)"), &b, &d);
            }
            if ic == 1 && ml > 0 {
                // must equal bytes 64.. of the ic=0 keystream
                let z = vec![0u8; ml + 64];
                let mut ks = vec![0u8; ml + 64 + G];
                unsafe {
                    x_c(
                        ks.as_mut_ptr(),
                        z.as_ptr(),
                        (ml + 64) as u64,
                        n.as_ptr(),
                        k.as_ptr(),
                    );
                }
                let want: Vec<u8> = ks[64..64 + ml]
                    .iter()
                    .zip(&m)
                    .map(|(x, y)| x ^ y)
                    .collect();
                eq_bytes(&format!("{label}: ic=1 == 64-byte shift"), &a[..ml], &want);
            }
        }
    }
}

#[test]
fn c656_chacha20_constants_and_keygen() {
    // row 656
    sz_is("crypto_stream_chacha20_keybytes", 32);
    sz_is("crypto_stream_chacha20_noncebytes", 8);
    sz_is("crypto_stream_chacha20_messagebytes_max", usize::MAX);
    keygen_check("crypto_stream_chacha20_keygen", 32);
}

#[test]
fn c657_659_chacha20_stream_and_xor() {
    // rows 657, 658, 659
    let s = StreamNoIc::new("crypto_stream_chacha20");
    let mut rng = Rng::new(0xC20_06_57);
    s.sweep(&mut rng);
}

#[test]
fn c660_664_chacha20_xor_ic() {
    // rows 660 (ic=0), 661 (ic=1), 662 (large), 663 (2^32-1), 664 (2^32,
    // 2^32+1, 2^64-1 — the 64-bit counter, no ic guard in the non-ietf form)
    xor_ic64_sweep(
        "crypto_stream_chacha20",
        32,
        8,
        &[
            0,
            1,
            2,
            0xffff,
            0xdead_beef,
            1u64 << 31,
            0xffff_fffe,
            0xffff_ffff,
            1u64 << 32,
            (1u64 << 32) + 1,
            1u64 << 63,
            u64::MAX,
        ],
        &[0, 1, 63, 64, 65, 127, 128, 129, 1000],
        0xC20_06_60,
    );
}

#[test]
fn c665_chacha20_ietf_constants_and_keygen() {
    // row 665
    sz_is("crypto_stream_chacha20_ietf_keybytes", 32);
    sz_is("crypto_stream_chacha20_ietf_noncebytes", 12);
    sz_is(
        "crypto_stream_chacha20_ietf_messagebytes_max",
        core::cmp::min(usize::MAX, 64 * (1usize << 32)),
    );
    keygen_check("crypto_stream_chacha20_ietf_keygen", 32);
}

#[test]
fn c666_667_chacha20_ietf_stream_and_xor() {
    // rows 666, 667
    let s = StreamNoIc::new("crypto_stream_chacha20_ietf");
    let mut rng = Rng::new(0x1E7F_06_66);
    s.sweep(&mut rng);
}

/// `_xor_ic` sweep for a 32-bit-`ic` primitive.
fn xor_ic32_sweep(name: &str, kb: usize, nb: usize, cases: &[(u32, usize)], seed: u64) {
    let (f_c, f_r) = unsafe { fpair::<XorIc32Fn>(&format!("{name}_xor_ic")) };
    let mut rng = Rng::new(seed);
    for &(ic, ml) in cases {
        let k = rng.bytes(kb);
        let n = rng.bytes(nb);
        let m = rng.bytes(ml);
        let label = format!("{name}_xor_ic ic={ic:#x} mlen={ml}");
        let mut a = vec![0xA5u8; ml + G];
        let mut b = vec![0xA5u8; ml + G];
        let (ra, rb) = unsafe {
            (
                f_c(a.as_mut_ptr(), m.as_ptr(), ml as u64, n.as_ptr(), ic, k.as_ptr()),
                f_r(b.as_mut_ptr(), m.as_ptr(), ml as u64, n.as_ptr(), ic, k.as_ptr()),
            )
        };
        eq_i32(&format!("{label}: rc"), ra, rb);
        assert_eq!(ra, 0);
        eq_bytes(&format!("{label}: out"), &a, &b);
        assert!(a[ml..].iter().all(|&x| x == 0xA5), "{label}: overrun");
    }
}

#[test]
fn c668_671_chacha20_ietf_xor_ic() {
    // rows 668 (ic=0 == _ietf_xor), 669 (ic=1 == 64-byte shift), 670 (large but
    // legal), 671 (ic = 2^32-1 with mlen 0 and 1..64 — the largest ic that
    // still satisfies ic + ceil(mlen/64) <= 2^32).
    let (f_c, f_r) = unsafe {
        fpair::<XorIc32Fn>("crypto_stream_chacha20_ietf_xor_ic")
    };
    let (x_c, x_r) = unsafe { fpair::<XorFn>("crypto_stream_chacha20_ietf_xor") };
    let mut rng = Rng::new(0x1E7F_06_68);
    // row 668/669 against the plain _xor form
    for &ml in CLENS {
        let k = rng.bytes(32);
        let n = rng.bytes(12);
        let m = rng.bytes(ml);
        let mut a = vec![0xA5u8; ml + G];
        let mut b = vec![0xA5u8; ml + G];
        let mut c = vec![0xA5u8; ml + G];
        let mut d = vec![0xA5u8; ml + G];
        unsafe {
            let ra = f_c(a.as_mut_ptr(), m.as_ptr(), ml as u64, n.as_ptr(), 0, k.as_ptr());
            let rb = f_r(b.as_mut_ptr(), m.as_ptr(), ml as u64, n.as_ptr(), 0, k.as_ptr());
            eq_i32("ietf ic=0 rc", ra, rb);
            x_c(c.as_mut_ptr(), m.as_ptr(), ml as u64, n.as_ptr(), k.as_ptr());
            x_r(d.as_mut_ptr(), m.as_ptr(), ml as u64, n.as_ptr(), k.as_ptr());
        }
        eq_bytes("row668 ietf ic=0 C/Rust", &a, &b);
        eq_bytes("row668 ietf _xor C/Rust", &c, &d);
        eq_bytes("row668 ic=0 == _xor", &a, &c);
        // row 669: ic=1 is the 64-byte-shifted keystream
        if ml > 0 {
            let z = vec![0u8; ml + 64];
            let mut ks = vec![0u8; ml + 64 + G];
            let mut e = vec![0xA5u8; ml + G];
            let mut f = vec![0xA5u8; ml + G];
            unsafe {
                x_c(
                    ks.as_mut_ptr(),
                    z.as_ptr(),
                    (ml + 64) as u64,
                    n.as_ptr(),
                    k.as_ptr(),
                );
                let ra = f_c(e.as_mut_ptr(), m.as_ptr(), ml as u64, n.as_ptr(), 1, k.as_ptr());
                let rb = f_r(f.as_mut_ptr(), m.as_ptr(), ml as u64, n.as_ptr(), 1, k.as_ptr());
                eq_i32("ietf ic=1 rc", ra, rb);
            }
            eq_bytes("row669 ietf ic=1 C/Rust", &e, &f);
            let want: Vec<u8> = ks[64..64 + ml].iter().zip(&m).map(|(x, y)| x ^ y).collect();
            eq_bytes("row669 ic=1 == 64-byte shift", &e[..ml], &want);
        }
    }
    // rows 670/671: legal large ic values (ic + ceil(mlen/64) <= 2^32)
    let mut cases: Vec<(u32, usize)> = vec![
        (2, 128),
        (0xffff, 1000),
        (1u32 << 31, 1000),
        (0xffff_fffe, 64),
        (0xffff_ffff, 0),
        (0xffff_ffff, 1),
        (0xffff_ffff, 63),
        (0xffff_ffff, 64),
    ];
    cases.push((0xffff_fffd, 128));
    xor_ic32_sweep("crypto_stream_chacha20_ietf", 32, 12, &cases, 0x1E7F_06_71);
}

#[test]
fn c672_673_chacha20_ietf_ext() {
    // rows 672, 673 — the internal ietf_ext entry points: same keystream as
    // `_ietf`, but capped at SODIUM_SIZE_MAX and with NO ic overflow guard, so
    // the 32-bit counter is allowed to carry into the IV.
    let (e_c, e_r) = unsafe { fpair::<StreamFn>("crypto_stream_chacha20_ietf_ext") };
    let (i_c, i_r) = unsafe { fpair::<StreamFn>("crypto_stream_chacha20_ietf") };
    let (x_c, x_r) =
        unsafe { fpair::<XorIc32Fn>("crypto_stream_chacha20_ietf_ext_xor_ic") };
    let mut rng = Rng::new(0x1E7F_06_72);
    for &cl in &[0usize, 1, 64, 65, 1000] {
        let k = rng.bytes(32);
        let n = rng.bytes(12);
        let mut a = vec![0xA5u8; cl + G];
        let mut b = vec![0xA5u8; cl + G];
        let mut c = vec![0xA5u8; cl + G];
        let mut d = vec![0xA5u8; cl + G];
        unsafe {
            let ra = e_c(a.as_mut_ptr(), cl as u64, n.as_ptr(), k.as_ptr());
            let rb = e_r(b.as_mut_ptr(), cl as u64, n.as_ptr(), k.as_ptr());
            eq_i32("ietf_ext rc", ra, rb);
            i_c(c.as_mut_ptr(), cl as u64, n.as_ptr(), k.as_ptr());
            i_r(d.as_mut_ptr(), cl as u64, n.as_ptr(), k.as_ptr());
        }
        eq_bytes("row672 ietf_ext", &a, &b);
        eq_bytes("row672 ietf", &c, &d);
        eq_bytes("row672 ietf_ext == ietf", &a, &c);
    }
    // row 673: ic values that the guarded `_ietf_xor_ic` would reject
    for &(ic, ml) in &[
        (0u32, 64usize),
        (1, 65),
        (1u32 << 31, 128),
        (0xffff_ffff, 64),
        (0xffff_ffff, 65),
        (0xffff_ffff, 128),
        (0xffff_fffe, 128),
    ] {
        let k = rng.bytes(32);
        let n = rng.bytes(12);
        let m = rng.bytes(ml);
        let mut a = vec![0xA5u8; ml + G];
        let mut b = vec![0xA5u8; ml + G];
        let (ra, rb) = unsafe {
            (
                x_c(a.as_mut_ptr(), m.as_ptr(), ml as u64, n.as_ptr(), ic, k.as_ptr()),
                x_r(b.as_mut_ptr(), m.as_ptr(), ml as u64, n.as_ptr(), ic, k.as_ptr()),
            )
        };
        eq_i32(&format!("row673 ext_xor_ic ic={ic:#x} mlen={ml} rc"), ra, rb);
        assert_eq!(ra, 0);
        eq_bytes(&format!("row673 ext_xor_ic ic={ic:#x} mlen={ml}"), &a, &b);
    }
}

#[test]
fn c674_681_salsa20() {
    // rows 674 (constants + keygen), 675 (keystream), 676 (_xor + in-place),
    // 677-681 (_xor_ic: 0, 1, large, 2^32-1, 2^32, 2^40, 2^63, 2^64-1)
    sz_is("crypto_stream_salsa20_keybytes", 32);
    sz_is("crypto_stream_salsa20_noncebytes", 8);
    sz_is("crypto_stream_salsa20_messagebytes_max", usize::MAX);
    keygen_check("crypto_stream_salsa20_keygen", 32);
    let s = StreamNoIc::new("crypto_stream_salsa20");
    let mut rng = Rng::new(0x5A_06_75);
    s.sweep(&mut rng);
    xor_ic64_sweep(
        "crypto_stream_salsa20",
        32,
        8,
        &[
            0,
            1,
            2,
            0xffff,
            0xdead_beef,
            1u64 << 31,
            0xffff_fffe,
            0xffff_ffff,
            1u64 << 32,
            1u64 << 40,
            1u64 << 63,
            u64::MAX,
        ],
        &[0, 1, 63, 64, 65, 127, 128, 129, 1000],
        0x5A_06_77,
    );
}

#[test]
fn c682_684_salsa2012() {
    // rows 682, 683, 684 — no `_xor_ic` entry point for salsa2012
    sz_is("crypto_stream_salsa2012_keybytes", 32);
    sz_is("crypto_stream_salsa2012_noncebytes", 8);
    sz_is("crypto_stream_salsa2012_messagebytes_max", usize::MAX);
    keygen_check("crypto_stream_salsa2012_keygen", 32);
    assert!(
        !has_sym("crypto_stream_salsa2012_xor_ic"),
        "row 683: salsa2012 has no _xor_ic"
    );
    let s = StreamNoIc::new("crypto_stream_salsa2012");
    let mut rng = Rng::new(0x20_12_06_83);
    s.sweep(&mut rng);
}

#[test]
fn c685_687_salsa208() {
    // rows 685, 686, 687
    sz_is("crypto_stream_salsa208_keybytes", 32);
    sz_is("crypto_stream_salsa208_noncebytes", 8);
    sz_is("crypto_stream_salsa208_messagebytes_max", usize::MAX);
    keygen_check("crypto_stream_salsa208_keygen", 32);
    assert!(
        !has_sym("crypto_stream_salsa208_xor_ic"),
        "row 686: salsa208 has no _xor_ic"
    );
    let s = StreamNoIc::new("crypto_stream_salsa208");
    let mut rng = Rng::new(0x20_08_06_86);
    s.sweep(&mut rng);
}

#[test]
fn c688_692_xsalsa20() {
    // rows 688, 689, 690, 691, 692
    sz_is("crypto_stream_xsalsa20_keybytes", 32);
    sz_is("crypto_stream_xsalsa20_noncebytes", 24);
    sz_is("crypto_stream_xsalsa20_messagebytes_max", usize::MAX);
    keygen_check("crypto_stream_xsalsa20_keygen", 32);
    let s = StreamNoIc::new("crypto_stream_xsalsa20");
    let mut rng = Rng::new(0x5A_06_89);
    s.sweep(&mut rng);
    xor_ic64_sweep(
        "crypto_stream_xsalsa20",
        32,
        24,
        &[
            0,
            1,
            2,
            0xffff,
            0xdead_beef,
            1u64 << 31,
            0xffff_fffe,
            0xffff_ffff,
            1u64 << 32,
            u64::MAX,
        ],
        &[0, 1, 63, 64, 65, 128, 1000],
        0x5A_06_91,
    );
    // row 692: xsalsa20 == salsa20 keyed with hsalsa20(n[0..16), k, NULL) and
    // nonce n[16..24)
    let (hs_c, hs_r) = unsafe { fpair::<CoreFn>("crypto_core_hsalsa20") };
    let (xs_c, xs_r) = unsafe { fpair::<StreamFn>("crypto_stream_xsalsa20") };
    let (s2_c, s2_r) = unsafe { fpair::<StreamFn>("crypto_stream_salsa20") };
    for &cl in &[0usize, 1, 63, 64, 65, 1000] {
        let k = rng.bytes(32);
        let n = rng.bytes(24);
        let mut sub_c = [0u8; 32];
        let mut sub_r = [0u8; 32];
        let mut a = vec![0u8; cl + G];
        let mut b = vec![0u8; cl + G];
        let mut c = vec![0u8; cl + G];
        let mut d = vec![0u8; cl + G];
        unsafe {
            hs_c(sub_c.as_mut_ptr(), n.as_ptr(), k.as_ptr(), ptr::null());
            hs_r(sub_r.as_mut_ptr(), n.as_ptr(), k.as_ptr(), ptr::null());
            xs_c(a.as_mut_ptr(), cl as u64, n.as_ptr(), k.as_ptr());
            xs_r(b.as_mut_ptr(), cl as u64, n.as_ptr(), k.as_ptr());
            s2_c(c.as_mut_ptr(), cl as u64, n[16..].as_ptr(), sub_c.as_ptr());
            s2_r(d.as_mut_ptr(), cl as u64, n[16..].as_ptr(), sub_c.as_ptr());
        }
        eq_bytes("row692 hsalsa20 subkey", &sub_c, &sub_r);
        eq_bytes("row692 xsalsa20", &a, &b);
        eq_bytes("row692 salsa20", &c, &d);
        eq_bytes("row692 xsalsa20 == salsa20(hsalsa20 key)", &a, &c);
    }
}

#[test]
fn c693_697_xchacha20() {
    // rows 693, 694, 695, 696, 697
    sz_is("crypto_stream_xchacha20_keybytes", 32);
    sz_is("crypto_stream_xchacha20_noncebytes", 24);
    sz_is("crypto_stream_xchacha20_messagebytes_max", usize::MAX);
    keygen_check("crypto_stream_xchacha20_keygen", 32);
    let s = StreamNoIc::new("crypto_stream_xchacha20");
    let mut rng = Rng::new(0x8C_06_94);
    s.sweep(&mut rng);
    xor_ic64_sweep(
        "crypto_stream_xchacha20",
        32,
        24,
        &[
            0,
            1,
            2,
            0xffff,
            0xdead_beef,
            1u64 << 31,
            0xffff_fffe,
            0xffff_ffff,
            1u64 << 32,
            u64::MAX,
        ],
        &[0, 1, 63, 64, 65, 128, 1000],
        0x8C_06_96,
    );
    // row 697
    let (hc_c, hc_r) = unsafe { fpair::<CoreFn>("crypto_core_hchacha20") };
    let (xc_c, xc_r) = unsafe { fpair::<StreamFn>("crypto_stream_xchacha20") };
    let (cc_c, cc_r) = unsafe { fpair::<StreamFn>("crypto_stream_chacha20") };
    for &cl in &[0usize, 1, 63, 64, 65, 1000] {
        let k = rng.bytes(32);
        let n = rng.bytes(24);
        let mut sub_c = [0u8; 32];
        let mut sub_r = [0u8; 32];
        let mut a = vec![0u8; cl + G];
        let mut b = vec![0u8; cl + G];
        let mut c = vec![0u8; cl + G];
        let mut d = vec![0u8; cl + G];
        unsafe {
            hc_c(sub_c.as_mut_ptr(), n.as_ptr(), k.as_ptr(), ptr::null());
            hc_r(sub_r.as_mut_ptr(), n.as_ptr(), k.as_ptr(), ptr::null());
            xc_c(a.as_mut_ptr(), cl as u64, n.as_ptr(), k.as_ptr());
            xc_r(b.as_mut_ptr(), cl as u64, n.as_ptr(), k.as_ptr());
            cc_c(c.as_mut_ptr(), cl as u64, n[16..].as_ptr(), sub_c.as_ptr());
            cc_r(d.as_mut_ptr(), cl as u64, n[16..].as_ptr(), sub_c.as_ptr());
        }
        eq_bytes("row697 hchacha20 subkey", &sub_c, &sub_r);
        eq_bytes("row697 xchacha20", &a, &b);
        eq_bytes("row697 chacha20", &c, &d);
        eq_bytes("row697 xchacha20 == chacha20(hchacha20 key)", &a, &c);
    }
}

#[test]
fn c698_701_crypto_stream_default() {
    // rows 698, 699, 700, 701 — the default primitive is xsalsa20
    sz_is("crypto_stream_keybytes", 32);
    sz_is("crypto_stream_noncebytes", 24);
    sz_is("crypto_stream_messagebytes_max", usize::MAX);
    str_is("crypto_stream_primitive", "xsalsa20");
    keygen_check("crypto_stream_keygen", 32);
    let s = StreamNoIc::new("crypto_stream");
    let mut rng = Rng::new(0x57_07_00);
    s.sweep(&mut rng);
    let (d_c, d_r) = unsafe { fpair::<StreamFn>("crypto_stream") };
    let (x_c, x_r) = unsafe { fpair::<StreamFn>("crypto_stream_xsalsa20") };
    let (dx_c, dx_r) = unsafe { fpair::<XorFn>("crypto_stream_xor") };
    let (xx_c, xx_r) = unsafe { fpair::<XorFn>("crypto_stream_xsalsa20_xor") };
    for &cl in &[0usize, 1, 63, 64, 65, 128, 1000] {
        let k = rng.bytes(32);
        let n = rng.bytes(24);
        let m = rng.bytes(cl);
        let mut a = vec![0u8; cl + G];
        let mut b = vec![0u8; cl + G];
        let mut c = vec![0u8; cl + G];
        let mut d = vec![0u8; cl + G];
        unsafe {
            d_c(a.as_mut_ptr(), cl as u64, n.as_ptr(), k.as_ptr());
            d_r(b.as_mut_ptr(), cl as u64, n.as_ptr(), k.as_ptr());
            x_c(c.as_mut_ptr(), cl as u64, n.as_ptr(), k.as_ptr());
            x_r(d.as_mut_ptr(), cl as u64, n.as_ptr(), k.as_ptr());
        }
        eq_bytes("row700 crypto_stream C/Rust", &a, &b);
        eq_bytes("row700 xsalsa20 C/Rust", &c, &d);
        eq_bytes("row700 crypto_stream == xsalsa20", &a, &c);
        let mut a = vec![0u8; cl + G];
        let mut b = vec![0u8; cl + G];
        let mut c = vec![0u8; cl + G];
        let mut d = vec![0u8; cl + G];
        unsafe {
            dx_c(a.as_mut_ptr(), m.as_ptr(), cl as u64, n.as_ptr(), k.as_ptr());
            dx_r(b.as_mut_ptr(), m.as_ptr(), cl as u64, n.as_ptr(), k.as_ptr());
            xx_c(c.as_mut_ptr(), m.as_ptr(), cl as u64, n.as_ptr(), k.as_ptr());
            xx_r(d.as_mut_ptr(), m.as_ptr(), cl as u64, n.as_ptr(), k.as_ptr());
        }
        eq_bytes("row701 crypto_stream_xor C/Rust", &a, &b);
        eq_bytes("row701 xsalsa20_xor C/Rust", &c, &d);
        eq_bytes("row701 crypto_stream_xor == xsalsa20_xor", &a, &c);
    }
}

// ===========================================================================
// crypto_core_* — CONFIGS rows 702-716
// ===========================================================================
const SIGMA: [u8; 16] = *b"expand 32-byte k";

fn core_sweep(name: &str, outlen: usize, seed: u64) {
    let (f_c, f_r) = unsafe { fpair::<CoreFn>(name) };
    let mut rng = Rng::new(seed);
    let inputs: Vec<(Vec<u8>, Vec<u8>)> = vec![
        (vec![0u8; 16], vec![0u8; 32]),
        (vec![0xffu8; 16], vec![0xffu8; 32]),
        (vec![0u8; 16], vec![0xffu8; 32]),
        (vec![0xffu8; 16], vec![0u8; 32]),
        (rng.bytes(16), rng.bytes(32)),
        (rng.bytes(16), rng.bytes(32)),
        (rng.bytes(16), rng.bytes(32)),
        (rng.bytes(16), rng.bytes(32)),
    ];
    for (inp, k) in &inputs {
        // c == NULL (sigma constants)
        let mut a = vec![0xA5u8; outlen + G];
        let mut b = vec![0xA5u8; outlen + G];
        let (ra, rb) = unsafe {
            (
                f_c(a.as_mut_ptr(), inp.as_ptr(), k.as_ptr(), ptr::null()),
                f_r(b.as_mut_ptr(), inp.as_ptr(), k.as_ptr(), ptr::null()),
            )
        };
        eq_i32(&format!("{name} c=NULL rc"), ra, rb);
        assert_eq!(ra, 0);
        eq_bytes(&format!("{name} c=NULL out"), &a, &b);
        assert!(a[outlen..].iter().all(|&x| x == 0xA5), "{name}: overrun");
        let sigma_out = a[..outlen].to_vec();

        // c != NULL
        for (tag, cst) in [
            ("sigma", SIGMA.to_vec()),
            ("zero", vec![0u8; 16]),
            ("ff", vec![0xffu8; 16]),
            ("random", rng.bytes(16)),
        ] {
            let mut a = vec![0xA5u8; outlen + G];
            let mut b = vec![0xA5u8; outlen + G];
            let (ra, rb) = unsafe {
                (
                    f_c(a.as_mut_ptr(), inp.as_ptr(), k.as_ptr(), cst.as_ptr()),
                    f_r(b.as_mut_ptr(), inp.as_ptr(), k.as_ptr(), cst.as_ptr()),
                )
            };
            eq_i32(&format!("{name} c={tag} rc"), ra, rb);
            assert_eq!(ra, 0);
            eq_bytes(&format!("{name} c={tag} out"), &a, &b);
            if tag == "sigma" {
                eq_bytes(
                    &format!("{name} c=sigma == c=NULL"),
                    &a[..outlen],
                    &sigma_out,
                );
            }
        }
    }
}

#[test]
fn c702_704_core_salsa20() {
    // rows 702, 703, 704
    core_sweep("crypto_core_salsa20", 64, 0x50_07_02);
    sz_is("crypto_core_salsa20_outputbytes", 64);
    sz_is("crypto_core_salsa20_inputbytes", 16);
    sz_is("crypto_core_salsa20_keybytes", 32);
    sz_is("crypto_core_salsa20_constbytes", 16);
}

#[test]
fn c705_707_core_salsa2012() {
    // rows 705, 706, 707
    core_sweep("crypto_core_salsa2012", 64, 0x50_07_05);
    sz_is("crypto_core_salsa2012_outputbytes", 64);
    sz_is("crypto_core_salsa2012_inputbytes", 16);
    sz_is("crypto_core_salsa2012_keybytes", 32);
    sz_is("crypto_core_salsa2012_constbytes", 16);
}

#[test]
fn c708_710_core_salsa208() {
    // rows 708, 709, 710
    core_sweep("crypto_core_salsa208", 64, 0x50_07_08);
    sz_is("crypto_core_salsa208_outputbytes", 64);
    sz_is("crypto_core_salsa208_inputbytes", 16);
    sz_is("crypto_core_salsa208_keybytes", 32);
    sz_is("crypto_core_salsa208_constbytes", 16);
}

#[test]
fn c711_713_core_hsalsa20() {
    // rows 711, 712, 713
    core_sweep("crypto_core_hsalsa20", 32, 0x50_07_11);
    sz_is("crypto_core_hsalsa20_outputbytes", 32);
    sz_is("crypto_core_hsalsa20_inputbytes", 16);
    sz_is("crypto_core_hsalsa20_keybytes", 32);
    sz_is("crypto_core_hsalsa20_constbytes", 16);
}

#[test]
fn c714_716_core_hchacha20() {
    // rows 714, 715, 716
    core_sweep("crypto_core_hchacha20", 32, 0x50_07_14);
    sz_is("crypto_core_hchacha20_outputbytes", 32);
    sz_is("crypto_core_hchacha20_inputbytes", 16);
    sz_is("crypto_core_hchacha20_keybytes", 32);
    sz_is("crypto_core_hchacha20_constbytes", 16);
}
