//! Differential tests for the AEAD + SECRETBOX + SECRETSTREAM family.
//!
//! Every call goes through symbols loaded from BOTH the C and the Rust
//! `.so` via `sympair!`; we compare return codes and output buffers
//! byte-for-byte. The C library is ground truth.

mod common;
use common::{libs, Rng};

use std::os::raw::c_char;

// ---- FFI type aliases for the AEAD one-shot / detached families ----------

// encrypt (combined): c, clen_p, m, mlen, ad, adlen, nsec, npub, k
type AeadEncrypt = unsafe extern "C" fn(
    *mut u8,
    *mut u64,
    *const u8,
    u64,
    *const u8,
    u64,
    *const u8,
    *const u8,
    *const u8,
) -> i32;

// decrypt (combined): m, mlen_p, nsec, c, clen, ad, adlen, npub, k
type AeadDecrypt = unsafe extern "C" fn(
    *mut u8,
    *mut u64,
    *mut u8,
    *const u8,
    u64,
    *const u8,
    u64,
    *const u8,
    *const u8,
) -> i32;

// encrypt_detached: c, mac, maclen_p, m, mlen, ad, adlen, nsec, npub, k
type AeadEncryptDetached = unsafe extern "C" fn(
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
) -> i32;

// decrypt_detached: m, nsec, c, clen, mac, ad, adlen, npub, k
type AeadDecryptDetached = unsafe extern "C" fn(
    *mut u8,
    *mut u8,
    *const u8,
    u64,
    *const u8,
    *const u8,
    u64,
    *const u8,
    *const u8,
) -> i32;

/// Message lengths exercised across every family: edge cases plus block
/// boundaries and a large buffer.
const MSG_LENS: &[usize] = &[
    0, 1, 15, 16, 17, 31, 32, 33, 63, 64, 65, 100, 127, 128, 129, 255, 256, 1000, 4096, 8192,
];

/// AAD shapes: empty and several nonzero sizes.
const AAD_LENS: &[usize] = &[0, 1, 7, 16, 33, 64, 200];

/// Helper: a null pointer of u8 (for empty/absent AAD).
fn null_u8() -> *const u8 {
    std::ptr::null()
}

/// Generic Phase-B driver for an AEAD family that has a fixed
/// (keybytes, npubbytes, abytes). Runs combined + detached roundtrips and
/// asserts C/Rust byte equality of ciphertext, mac, and decrypted plaintext.
#[allow(clippy::too_many_arguments)]
fn aead_phase_b(
    name: &str,
    keybytes: usize,
    npubbytes: usize,
    abytes: usize,
    c_enc: &AeadEncrypt,
    r_enc: &AeadEncrypt,
    c_dec: &AeadDecrypt,
    r_dec: &AeadDecrypt,
    c_enc_d: &AeadEncryptDetached,
    r_enc_d: &AeadEncryptDetached,
    c_dec_d: &AeadDecryptDetached,
    r_dec_d: &AeadDecryptDetached,
) {
    let mut rng = Rng::new(0xA0A0_0000 ^ name.len() as u64);

    for &mlen in MSG_LENS {
        for &adlen in AAD_LENS {
            let key = rng.vec(keybytes);
            let npub = rng.vec(npubbytes);
            let msg = rng.vec(mlen);
            // Also test a null AAD pointer when adlen == 0.
            let ad = rng.vec(adlen);
            let ad_ptr = if adlen == 0 { null_u8() } else { ad.as_ptr() };

            // ---------- combined encrypt ----------
            let mut c_ct = vec![0u8; mlen + abytes];
            let mut r_ct = vec![0u8; mlen + abytes];
            let mut c_clen: u64 = 0;
            let mut r_clen: u64 = 0;
            let rc_c = unsafe {
                c_enc(
                    c_ct.as_mut_ptr(),
                    &mut c_clen,
                    msg.as_ptr(),
                    mlen as u64,
                    ad_ptr,
                    adlen as u64,
                    std::ptr::null(),
                    npub.as_ptr(),
                    key.as_ptr(),
                )
            };
            let rc_r = unsafe {
                r_enc(
                    r_ct.as_mut_ptr(),
                    &mut r_clen,
                    msg.as_ptr(),
                    mlen as u64,
                    ad_ptr,
                    adlen as u64,
                    std::ptr::null(),
                    npub.as_ptr(),
                    key.as_ptr(),
                )
            };
            assert_eq!(rc_c, rc_r, "{name}: combined encrypt rc mlen={mlen} adlen={adlen}");
            assert_eq!(c_clen, r_clen, "{name}: clen mlen={mlen} adlen={adlen}");
            assert_eq!(c_ct, r_ct, "{name}: ciphertext mlen={mlen} adlen={adlen}");

            // ---------- combined decrypt (roundtrip on BOTH) ----------
            let mut c_pt = vec![0u8; mlen];
            let mut r_pt = vec![0u8; mlen];
            let mut c_mlen: u64 = 0;
            let mut r_mlen: u64 = 0;
            // C decrypts C ciphertext; Rust decrypts Rust ciphertext.
            let dc_c = unsafe {
                c_dec(
                    c_pt.as_mut_ptr(),
                    &mut c_mlen,
                    std::ptr::null_mut(),
                    c_ct.as_ptr(),
                    c_clen,
                    ad_ptr,
                    adlen as u64,
                    npub.as_ptr(),
                    key.as_ptr(),
                )
            };
            let dc_r = unsafe {
                r_dec(
                    r_pt.as_mut_ptr(),
                    &mut r_mlen,
                    std::ptr::null_mut(),
                    r_ct.as_ptr(),
                    r_clen,
                    ad_ptr,
                    adlen as u64,
                    npub.as_ptr(),
                    key.as_ptr(),
                )
            };
            assert_eq!(dc_c, 0, "{name}: C combined decrypt failed mlen={mlen}");
            assert_eq!(dc_r, 0, "{name}: Rust combined decrypt failed mlen={mlen}");
            assert_eq!(c_mlen, r_mlen, "{name}: decrypt mlen out");
            assert_eq!(c_pt, msg, "{name}: C roundtrip plaintext");
            assert_eq!(r_pt, msg, "{name}: Rust roundtrip plaintext");

            // Cross-check: Rust must also decrypt the C ciphertext identically.
            let mut x_pt = vec![0u8; mlen];
            let mut x_mlen: u64 = 0;
            let dx = unsafe {
                r_dec(
                    x_pt.as_mut_ptr(),
                    &mut x_mlen,
                    std::ptr::null_mut(),
                    c_ct.as_ptr(),
                    c_clen,
                    ad_ptr,
                    adlen as u64,
                    npub.as_ptr(),
                    key.as_ptr(),
                )
            };
            assert_eq!(dx, 0, "{name}: Rust decrypt of C ciphertext");
            assert_eq!(x_pt, msg, "{name}: cross plaintext");

            // ---------- detached encrypt ----------
            let mut c_ctd = vec![0u8; mlen];
            let mut r_ctd = vec![0u8; mlen];
            let mut c_mac = vec![0u8; abytes];
            let mut r_mac = vec![0u8; abytes];
            let mut c_maclen: u64 = 0;
            let mut r_maclen: u64 = 0;
            let ec_c = unsafe {
                c_enc_d(
                    c_ctd.as_mut_ptr(),
                    c_mac.as_mut_ptr(),
                    &mut c_maclen,
                    msg.as_ptr(),
                    mlen as u64,
                    ad_ptr,
                    adlen as u64,
                    std::ptr::null(),
                    npub.as_ptr(),
                    key.as_ptr(),
                )
            };
            let ec_r = unsafe {
                r_enc_d(
                    r_ctd.as_mut_ptr(),
                    r_mac.as_mut_ptr(),
                    &mut r_maclen,
                    msg.as_ptr(),
                    mlen as u64,
                    ad_ptr,
                    adlen as u64,
                    std::ptr::null(),
                    npub.as_ptr(),
                    key.as_ptr(),
                )
            };
            assert_eq!(ec_c, ec_r, "{name}: detached encrypt rc");
            assert_eq!(c_maclen, r_maclen, "{name}: maclen");
            assert_eq!(c_ctd, r_ctd, "{name}: detached ciphertext mlen={mlen} adlen={adlen}");
            assert_eq!(c_mac, r_mac, "{name}: detached mac mlen={mlen} adlen={adlen}");
            // Detached ciphertext + mac must equal combined ciphertext.
            assert_eq!(&c_ct[..mlen], &c_ctd[..], "{name}: detached vs combined ct");
            assert_eq!(&c_ct[mlen..], &c_mac[..], "{name}: detached vs combined mac");

            // ---------- detached decrypt (roundtrip on BOTH) ----------
            let mut c_ptd = vec![0u8; mlen];
            let mut r_ptd = vec![0u8; mlen];
            let dd_c = unsafe {
                c_dec_d(
                    c_ptd.as_mut_ptr(),
                    std::ptr::null_mut(),
                    c_ctd.as_ptr(),
                    mlen as u64,
                    c_mac.as_ptr(),
                    ad_ptr,
                    adlen as u64,
                    npub.as_ptr(),
                    key.as_ptr(),
                )
            };
            let dd_r = unsafe {
                r_dec_d(
                    r_ptd.as_mut_ptr(),
                    std::ptr::null_mut(),
                    r_ctd.as_ptr(),
                    mlen as u64,
                    r_mac.as_ptr(),
                    ad_ptr,
                    adlen as u64,
                    npub.as_ptr(),
                    key.as_ptr(),
                )
            };
            assert_eq!(dd_c, 0, "{name}: C detached decrypt");
            assert_eq!(dd_r, 0, "{name}: Rust detached decrypt");
            assert_eq!(c_ptd, msg, "{name}: C detached plaintext");
            assert_eq!(r_ptd, msg, "{name}: Rust detached plaintext");
        }
    }
}

/// Generic Phase-C driver: error paths shared by all AEAD families.
#[allow(clippy::too_many_arguments)]
fn aead_phase_c(
    name: &str,
    keybytes: usize,
    npubbytes: usize,
    abytes: usize,
    c_enc: &AeadEncrypt,
    r_enc: &AeadEncrypt,
    c_dec: &AeadDecrypt,
    r_dec: &AeadDecrypt,
    c_dec_d: &AeadDecryptDetached,
    r_dec_d: &AeadDecryptDetached,
) {
    let mut rng = Rng::new(0xC0C0_0000 ^ name.len() as u64);
    let mlen = 64usize;
    let adlen = 16usize;

    for iter in 0..8 {
        let key = rng.vec(keybytes);
        let npub = rng.vec(npubbytes);
        let msg = rng.vec(mlen);
        let ad = rng.vec(adlen);

        // Baseline encrypt (via C, then confirm Rust produces the same bytes).
        let mut ct = vec![0u8; mlen + abytes];
        let mut clen: u64 = 0;
        unsafe {
            c_enc(
                ct.as_mut_ptr(),
                &mut clen,
                msg.as_ptr(),
                mlen as u64,
                ad.as_ptr(),
                adlen as u64,
                std::ptr::null(),
                npub.as_ptr(),
                key.as_ptr(),
            );
        }
        let mut r_ct = vec![0u8; mlen + abytes];
        let mut r_clen: u64 = 0;
        unsafe {
            r_enc(
                r_ct.as_mut_ptr(),
                &mut r_clen,
                msg.as_ptr(),
                mlen as u64,
                ad.as_ptr(),
                adlen as u64,
                std::ptr::null(),
                npub.as_ptr(),
                key.as_ptr(),
            );
        }
        assert_eq!(clen, r_clen, "{name}: phase-c baseline clen");
        assert_eq!(ct, r_ct, "{name}: phase-c baseline ciphertext");

        // --- (1) tampered ciphertext byte ---
        {
            let mut bad = ct.clone();
            let idx = rng.range(mlen.max(1));
            bad[idx] ^= 0x40;
            let mut cp = vec![0u8; mlen];
            let mut rp = vec![0u8; mlen];
            let mut cl: u64 = 0;
            let mut rl: u64 = 0;
            let a = unsafe { call_dec(c_dec, &mut cp, &mut cl, &bad, clen, &ad, &npub, &key) };
            let b = unsafe { call_dec(r_dec, &mut rp, &mut rl, &bad, clen, &ad, &npub, &key) };
            assert_eq!(a, -1, "{name}: C tampered ct iter={iter}");
            assert_eq!(b, -1, "{name}: Rust tampered ct iter={iter}");
        }

        // --- (2) tampered tag byte (last abytes) ---
        {
            let mut bad = ct.clone();
            let tlen = bad.len();
            bad[tlen_last(tlen, rng.range(abytes))] ^= 0x11;
            let mut cp = vec![0u8; mlen];
            let mut rp = vec![0u8; mlen];
            let mut cl: u64 = 0;
            let mut rl: u64 = 0;
            let a = unsafe { call_dec(c_dec, &mut cp, &mut cl, &bad, clen, &ad, &npub, &key) };
            let b = unsafe { call_dec(r_dec, &mut rp, &mut rl, &bad, clen, &ad, &npub, &key) };
            assert_eq!(a, -1, "{name}: C tampered tag iter={iter}");
            assert_eq!(b, -1, "{name}: Rust tampered tag iter={iter}");
        }

        // --- (3) tampered AAD ---
        {
            let mut bad_ad = ad.clone();
            bad_ad[rng.range(adlen)] ^= 0x80;
            let mut cp = vec![0u8; mlen];
            let mut rp = vec![0u8; mlen];
            let mut cl: u64 = 0;
            let mut rl: u64 = 0;
            let a = unsafe { call_dec(c_dec, &mut cp, &mut cl, &ct, clen, &bad_ad, &npub, &key) };
            let b = unsafe { call_dec(r_dec, &mut rp, &mut rl, &ct, clen, &bad_ad, &npub, &key) };
            assert_eq!(a, -1, "{name}: C tampered ad iter={iter}");
            assert_eq!(b, -1, "{name}: Rust tampered ad iter={iter}");
        }

        // --- (4) wrong nonce ---
        {
            let mut bad_npub = npub.clone();
            bad_npub[0] ^= 0x01;
            let mut cp = vec![0u8; mlen];
            let mut rp = vec![0u8; mlen];
            let mut cl: u64 = 0;
            let mut rl: u64 = 0;
            let a = unsafe { call_dec(c_dec, &mut cp, &mut cl, &ct, clen, &ad, &bad_npub, &key) };
            let b = unsafe { call_dec(r_dec, &mut rp, &mut rl, &ct, clen, &ad, &bad_npub, &key) };
            assert_eq!(a, -1, "{name}: C wrong nonce iter={iter}");
            assert_eq!(b, -1, "{name}: Rust wrong nonce iter={iter}");
        }

        // --- (5) wrong key ---
        {
            let mut bad_key = key.clone();
            bad_key[0] ^= 0x01;
            let mut cp = vec![0u8; mlen];
            let mut rp = vec![0u8; mlen];
            let mut cl: u64 = 0;
            let mut rl: u64 = 0;
            let a = unsafe { call_dec(c_dec, &mut cp, &mut cl, &ct, clen, &ad, &npub, &bad_key) };
            let b = unsafe { call_dec(r_dec, &mut rp, &mut rl, &ct, clen, &ad, &npub, &bad_key) };
            assert_eq!(a, -1, "{name}: C wrong key iter={iter}");
            assert_eq!(b, -1, "{name}: Rust wrong key iter={iter}");
        }

        // --- (6) truncated ciphertext (< abytes) ---
        for &trunc in &[0usize, 1, abytes - 1] {
            let mut cp = vec![0u8; mlen];
            let mut rp = vec![0u8; mlen];
            let mut cl: u64 = 0;
            let mut rl: u64 = 0;
            let a =
                unsafe { call_dec(c_dec, &mut cp, &mut cl, &ct, trunc as u64, &ad, &npub, &key) };
            let b =
                unsafe { call_dec(r_dec, &mut rp, &mut rl, &ct, trunc as u64, &ad, &npub, &key) };
            assert_eq!(a, -1, "{name}: C truncated clen={trunc}");
            assert_eq!(b, -1, "{name}: Rust truncated clen={trunc}");
        }

        // --- (7) detached: tampered mac ---
        {
            let mut mac = ct[mlen..].to_vec();
            mac[0] ^= 0x55;
            let ctbody = &ct[..mlen];
            let mut cp = vec![0u8; mlen];
            let mut rp = vec![0u8; mlen];
            let a = unsafe {
                c_dec_d(
                    cp.as_mut_ptr(),
                    std::ptr::null_mut(),
                    ctbody.as_ptr(),
                    mlen as u64,
                    mac.as_ptr(),
                    ad.as_ptr(),
                    adlen as u64,
                    npub.as_ptr(),
                    key.as_ptr(),
                )
            };
            let b = unsafe {
                r_dec_d(
                    rp.as_mut_ptr(),
                    std::ptr::null_mut(),
                    ctbody.as_ptr(),
                    mlen as u64,
                    mac.as_ptr(),
                    ad.as_ptr(),
                    adlen as u64,
                    npub.as_ptr(),
                    key.as_ptr(),
                )
            };
            assert_eq!(a, -1, "{name}: C detached tampered mac");
            assert_eq!(b, -1, "{name}: Rust detached tampered mac");
        }
    }
}

fn tlen_last(total: usize, off: usize) -> usize {
    total - 1 - off
}

#[allow(clippy::too_many_arguments)]
unsafe fn call_dec(
    f: &AeadDecrypt,
    m: &mut [u8],
    mlen_p: &mut u64,
    c: &[u8],
    clen: u64,
    ad: &[u8],
    npub: &[u8],
    key: &[u8],
) -> i32 {
    f(
        m.as_mut_ptr(),
        mlen_p,
        std::ptr::null_mut(),
        c.as_ptr(),
        clen,
        ad.as_ptr(),
        ad.len() as u64,
        npub.as_ptr(),
        key.as_ptr(),
    )
}

// =====================================================================
// AEAD family test entry points
// =====================================================================

macro_rules! aead_family {
    ($testname:ident, $prefix:literal, $key:expr, $npub:expr, $abytes:expr) => {
        #[test]
        fn $testname() {
            let l = libs();
            let (c_enc, r_enc) =
                sympair!(l, concat!($prefix, "_encrypt").as_bytes(), AeadEncrypt);
            let (c_dec, r_dec) =
                sympair!(l, concat!($prefix, "_decrypt").as_bytes(), AeadDecrypt);
            let (c_enc_d, r_enc_d) = sympair!(
                l,
                concat!($prefix, "_encrypt_detached").as_bytes(),
                AeadEncryptDetached
            );
            let (c_dec_d, r_dec_d) = sympair!(
                l,
                concat!($prefix, "_decrypt_detached").as_bytes(),
                AeadDecryptDetached
            );
            aead_phase_b(
                $prefix, $key, $npub, $abytes, &c_enc, &r_enc, &c_dec, &r_dec, &c_enc_d,
                &r_enc_d, &c_dec_d, &r_dec_d,
            );
            aead_phase_c(
                $prefix, $key, $npub, $abytes, &c_enc, &r_enc, &c_dec, &r_dec, &c_dec_d,
                &r_dec_d,
            );
        }
    };
}

aead_family!(
    chacha20poly1305_orig,
    "crypto_aead_chacha20poly1305",
    32,
    8,
    16
);
aead_family!(
    chacha20poly1305_ietf,
    "crypto_aead_chacha20poly1305_ietf",
    32,
    12,
    16
);
aead_family!(
    xchacha20poly1305_ietf,
    "crypto_aead_xchacha20poly1305_ietf",
    32,
    24,
    16
);
aead_family!(aegis128l, "crypto_aead_aegis128l", 16, 16, 32);
aead_family!(aegis256, "crypto_aead_aegis256", 32, 32, 32);

// =====================================================================
// AES256-GCM: guarded by is_available (), plus precomputed beforenm/afternm.
// =====================================================================

type AesBeforenm = unsafe extern "C" fn(*mut u8, *const u8) -> i32;
// afternm encrypt: c, clen_p, m, mlen, ad, adlen, nsec, npub, ctx
type AesEncAfternm = unsafe extern "C" fn(
    *mut u8,
    *mut u64,
    *const u8,
    u64,
    *const u8,
    u64,
    *const u8,
    *const u8,
    *const u8,
) -> i32;
// afternm decrypt: m, mlen_p, nsec, c, clen, ad, adlen, npub, ctx
type AesDecAfternm = unsafe extern "C" fn(
    *mut u8,
    *mut u64,
    *mut u8,
    *const u8,
    u64,
    *const u8,
    u64,
    *const u8,
    *const u8,
) -> i32;

#[test]
fn aes256gcm_is_available_matches() {
    let l = libs();
    let (c_av, r_av) =
        sympair!(l, b"crypto_aead_aes256gcm_is_available", unsafe extern "C" fn() -> i32);
    let cv = unsafe { c_av() };
    let rv = unsafe { r_av() };
    assert_eq!(cv, rv, "aes256gcm is_available divergence: C={cv} Rust={rv}");
}

#[test]
fn aes256gcm_full() {
    let l = libs();
    let (c_av, _r_av) =
        sympair!(l, b"crypto_aead_aes256gcm_is_available", unsafe extern "C" fn() -> i32);
    if unsafe { c_av() } == 0 {
        // Hardware AES not present on this host: the one-shot APIs call
        // sodium_misuse()/abort. Availability parity is checked separately.
        eprintln!("aes256gcm not available on this host; skipping crypto body");
        return;
    }

    const KEY: usize = 32;
    const NPUB: usize = 12;
    const ABYTES: usize = 16;

    let (c_enc, r_enc) = sympair!(l, b"crypto_aead_aes256gcm_encrypt", AeadEncrypt);
    let (c_dec, r_dec) = sympair!(l, b"crypto_aead_aes256gcm_decrypt", AeadDecrypt);
    let (c_enc_d, r_enc_d) =
        sympair!(l, b"crypto_aead_aes256gcm_encrypt_detached", AeadEncryptDetached);
    let (c_dec_d, r_dec_d) =
        sympair!(l, b"crypto_aead_aes256gcm_decrypt_detached", AeadDecryptDetached);
    aead_phase_b(
        "crypto_aead_aes256gcm",
        KEY,
        NPUB,
        ABYTES,
        &c_enc,
        &r_enc,
        &c_dec,
        &r_dec,
        &c_enc_d,
        &r_enc_d,
        &c_dec_d,
        &r_dec_d,
    );
    aead_phase_c(
        "crypto_aead_aes256gcm",
        KEY,
        NPUB,
        ABYTES,
        &c_enc,
        &r_enc,
        &c_dec,
        &r_dec,
        &c_dec_d,
        &r_dec_d,
    );

    // Precomputed (beforenm/afternm) path.
    let (c_before, r_before) = sympair!(l, b"crypto_aead_aes256gcm_beforenm", AesBeforenm);
    let (c_enc_a, r_enc_a) =
        sympair!(l, b"crypto_aead_aes256gcm_encrypt_afternm", AesEncAfternm);
    let (c_dec_a, r_dec_a) =
        sympair!(l, b"crypto_aead_aes256gcm_decrypt_afternm", AesDecAfternm);
    let (c_sb, r_sb) =
        sympair!(l, b"crypto_aead_aes256gcm_statebytes", unsafe extern "C" fn() -> usize);
    let sb = unsafe { c_sb() };
    assert_eq!(sb, unsafe { r_sb() }, "statebytes divergence");

    let mut rng = Rng::new(0xAE5A);
    for &mlen in &[0usize, 1, 16, 17, 64, 1000] {
        for &adlen in &[0usize, 16] {
            let key = rng.vec(KEY);
            let npub = rng.vec(NPUB);
            let msg = rng.vec(mlen);
            let ad = rng.vec(adlen);

            let mut c_ctx = vec![0u8; sb];
            let mut r_ctx = vec![0u8; sb];
            unsafe {
                c_before(c_ctx.as_mut_ptr(), key.as_ptr());
                r_before(r_ctx.as_mut_ptr(), key.as_ptr());
            }

            let mut c_ct = vec![0u8; mlen + ABYTES];
            let mut r_ct = vec![0u8; mlen + ABYTES];
            let mut cl = 0u64;
            let mut rl = 0u64;
            let rc = unsafe {
                c_enc_a(
                    c_ct.as_mut_ptr(),
                    &mut cl,
                    msg.as_ptr(),
                    mlen as u64,
                    ad.as_ptr(),
                    adlen as u64,
                    std::ptr::null(),
                    npub.as_ptr(),
                    c_ctx.as_ptr(),
                )
            };
            let rr = unsafe {
                r_enc_a(
                    r_ct.as_mut_ptr(),
                    &mut rl,
                    msg.as_ptr(),
                    mlen as u64,
                    ad.as_ptr(),
                    adlen as u64,
                    std::ptr::null(),
                    npub.as_ptr(),
                    r_ctx.as_ptr(),
                )
            };
            assert_eq!(rc, rr, "afternm encrypt rc");
            assert_eq!(c_ct, r_ct, "afternm ciphertext mlen={mlen} adlen={adlen}");

            let mut c_pt = vec![0u8; mlen];
            let mut r_pt = vec![0u8; mlen];
            let mut cml = 0u64;
            let mut rml = 0u64;
            let dc = unsafe {
                c_dec_a(
                    c_pt.as_mut_ptr(),
                    &mut cml,
                    std::ptr::null_mut(),
                    c_ct.as_ptr(),
                    cl,
                    ad.as_ptr(),
                    adlen as u64,
                    npub.as_ptr(),
                    c_ctx.as_ptr(),
                )
            };
            let dr = unsafe {
                r_dec_a(
                    r_pt.as_mut_ptr(),
                    &mut rml,
                    std::ptr::null_mut(),
                    r_ct.as_ptr(),
                    rl,
                    ad.as_ptr(),
                    adlen as u64,
                    npub.as_ptr(),
                    r_ctx.as_ptr(),
                )
            };
            assert_eq!(dc, 0, "C afternm decrypt");
            assert_eq!(dr, 0, "Rust afternm decrypt");
            assert_eq!(c_pt, msg, "C afternm roundtrip");
            assert_eq!(r_pt, msg, "Rust afternm roundtrip");
        }
    }
}

// =====================================================================
// SECRETBOX
// =====================================================================

// easy: c, m, mlen, n, k
type SbEasy = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, *const u8) -> i32;
// open_easy: m, c, clen, n, k
type SbOpenEasy = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, *const u8) -> i32;
// detached: c, mac, m, mlen, n, k
type SbDetached =
    unsafe extern "C" fn(*mut u8, *mut u8, *const u8, u64, *const u8, *const u8) -> i32;
// open_detached: m, c, mac, clen, n, k
type SbOpenDetached =
    unsafe extern "C" fn(*mut u8, *const u8, *const u8, u64, *const u8, *const u8) -> i32;

/// Phase B/C for a `*_easy` / `*_detached` secretbox family.
fn secretbox_easy_family(
    prefix: &str,
    macbytes: usize,
    noncebytes: usize,
    keybytes: usize,
) {
    let l = libs();
    let (c_easy, r_easy) = sympair!(l, format!("{prefix}_easy").as_bytes(), SbEasy);
    let (c_open, r_open) = sympair!(l, format!("{prefix}_open_easy").as_bytes(), SbOpenEasy);
    let (c_det, r_det) = sympair!(l, format!("{prefix}_detached").as_bytes(), SbDetached);
    let (c_odet, r_odet) =
        sympair!(l, format!("{prefix}_open_detached").as_bytes(), SbOpenDetached);

    let mut rng = Rng::new(0x5B_0000 ^ prefix.len() as u64);
    for &mlen in MSG_LENS {
        let key = rng.vec(keybytes);
        let nonce = rng.vec(noncebytes);
        let msg = rng.vec(mlen);

        // easy encrypt
        let mut c_ct = vec![0u8; mlen + macbytes];
        let mut r_ct = vec![0u8; mlen + macbytes];
        let a = unsafe {
            c_easy(c_ct.as_mut_ptr(), msg.as_ptr(), mlen as u64, nonce.as_ptr(), key.as_ptr())
        };
        let b = unsafe {
            r_easy(r_ct.as_mut_ptr(), msg.as_ptr(), mlen as u64, nonce.as_ptr(), key.as_ptr())
        };
        assert_eq!(a, b, "{prefix}: easy rc mlen={mlen}");
        assert_eq!(c_ct, r_ct, "{prefix}: easy ct mlen={mlen}");

        // easy open (roundtrip on BOTH)
        let mut c_pt = vec![0u8; mlen];
        let mut r_pt = vec![0u8; mlen];
        let oa = unsafe {
            c_open(c_pt.as_mut_ptr(), c_ct.as_ptr(), c_ct.len() as u64, nonce.as_ptr(), key.as_ptr())
        };
        let ob = unsafe {
            r_open(r_pt.as_mut_ptr(), r_ct.as_ptr(), r_ct.len() as u64, nonce.as_ptr(), key.as_ptr())
        };
        assert_eq!(oa, 0, "{prefix}: C open_easy mlen={mlen}");
        assert_eq!(ob, 0, "{prefix}: Rust open_easy mlen={mlen}");
        assert_eq!(c_pt, msg, "{prefix}: C open plaintext");
        assert_eq!(r_pt, msg, "{prefix}: Rust open plaintext");

        // detached encrypt
        let mut c_ctd = vec![0u8; mlen];
        let mut r_ctd = vec![0u8; mlen];
        let mut c_mac = vec![0u8; macbytes];
        let mut r_mac = vec![0u8; macbytes];
        let da = unsafe {
            c_det(
                c_ctd.as_mut_ptr(),
                c_mac.as_mut_ptr(),
                msg.as_ptr(),
                mlen as u64,
                nonce.as_ptr(),
                key.as_ptr(),
            )
        };
        let db = unsafe {
            r_det(
                r_ctd.as_mut_ptr(),
                r_mac.as_mut_ptr(),
                msg.as_ptr(),
                mlen as u64,
                nonce.as_ptr(),
                key.as_ptr(),
            )
        };
        assert_eq!(da, db, "{prefix}: detached rc");
        assert_eq!(c_ctd, r_ctd, "{prefix}: detached ct");
        assert_eq!(c_mac, r_mac, "{prefix}: detached mac");
        // easy layout is mac||ct
        assert_eq!(&c_ct[..macbytes], &c_mac[..], "{prefix}: easy mac prefix");
        assert_eq!(&c_ct[macbytes..], &c_ctd[..], "{prefix}: easy ct body");

        // detached open (roundtrip on BOTH)
        let mut c_ptd = vec![0u8; mlen];
        let mut r_ptd = vec![0u8; mlen];
        let ooa = unsafe {
            c_odet(
                c_ptd.as_mut_ptr(),
                c_ctd.as_ptr(),
                c_mac.as_ptr(),
                mlen as u64,
                nonce.as_ptr(),
                key.as_ptr(),
            )
        };
        let oob = unsafe {
            r_odet(
                r_ptd.as_mut_ptr(),
                r_ctd.as_ptr(),
                r_mac.as_ptr(),
                mlen as u64,
                nonce.as_ptr(),
                key.as_ptr(),
            )
        };
        assert_eq!(ooa, 0, "{prefix}: C open_detached");
        assert_eq!(oob, 0, "{prefix}: Rust open_detached");
        assert_eq!(c_ptd, msg, "{prefix}: C open_detached pt");
        assert_eq!(r_ptd, msg, "{prefix}: Rust open_detached pt");
    }

    // ---- Phase C: error paths ----
    let mlen = 64usize;
    for iter in 0..6 {
        let key = rng.vec(keybytes);
        let nonce = rng.vec(noncebytes);
        let msg = rng.vec(mlen);
        let mut ct = vec![0u8; mlen + macbytes];
        unsafe {
            c_easy(ct.as_mut_ptr(), msg.as_ptr(), mlen as u64, nonce.as_ptr(), key.as_ptr());
        }

        // tampered ciphertext / tag
        {
            let mut bad = ct.clone();
            let idx = rng.range(bad.len());
            bad[idx] ^= 0x22;
            let mut cp = vec![0u8; mlen];
            let mut rp = vec![0u8; mlen];
            let a = unsafe {
                c_open(cp.as_mut_ptr(), bad.as_ptr(), bad.len() as u64, nonce.as_ptr(), key.as_ptr())
            };
            let b = unsafe {
                r_open(rp.as_mut_ptr(), bad.as_ptr(), bad.len() as u64, nonce.as_ptr(), key.as_ptr())
            };
            assert_eq!(a, -1, "{prefix}: C tampered open iter={iter}");
            assert_eq!(b, -1, "{prefix}: Rust tampered open iter={iter}");
        }
        // wrong nonce
        {
            let mut bn = nonce.clone();
            bn[0] ^= 1;
            let mut cp = vec![0u8; mlen];
            let mut rp = vec![0u8; mlen];
            let a = unsafe {
                c_open(cp.as_mut_ptr(), ct.as_ptr(), ct.len() as u64, bn.as_ptr(), key.as_ptr())
            };
            let b = unsafe {
                r_open(rp.as_mut_ptr(), ct.as_ptr(), ct.len() as u64, bn.as_ptr(), key.as_ptr())
            };
            assert_eq!(a, -1, "{prefix}: C wrong nonce");
            assert_eq!(b, -1, "{prefix}: Rust wrong nonce");
        }
        // truncated (< macbytes)
        for &trunc in &[0usize, 1, macbytes - 1] {
            let mut cp = vec![0u8; mlen];
            let mut rp = vec![0u8; mlen];
            let a = unsafe {
                c_open(cp.as_mut_ptr(), ct.as_ptr(), trunc as u64, nonce.as_ptr(), key.as_ptr())
            };
            let b = unsafe {
                r_open(rp.as_mut_ptr(), ct.as_ptr(), trunc as u64, nonce.as_ptr(), key.as_ptr())
            };
            assert_eq!(a, -1, "{prefix}: C truncated {trunc}");
            assert_eq!(b, -1, "{prefix}: Rust truncated {trunc}");
        }
    }
}

#[test]
fn secretbox_easy_default() {
    // default == xsalsa20poly1305
    secretbox_easy_family("crypto_secretbox", 16, 24, 32);
}

#[test]
fn secretbox_easy_xchacha20poly1305() {
    secretbox_easy_family("crypto_secretbox_xchacha20poly1305", 16, 24, 32);
}

// ---- NaCl-style padded API: crypto_secretbox / _open and xsalsa variant ----

// c, m, mlen, n, k  (m must have ZEROBYTES=32 leading zeros)
type SbNacl = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, *const u8) -> i32;

fn secretbox_nacl_family(prefix: &str) {
    let l = libs();
    let (c_box, r_box) = sympair!(l, prefix.as_bytes(), SbNacl);
    let (c_open, r_open) = sympair!(l, format!("{prefix}_open").as_bytes(), SbNacl);

    const ZEROBYTES: usize = 32;
    const BOXZEROBYTES: usize = 16;
    let mut rng = Rng::new(0x5B_AC ^ prefix.len() as u64);

    // Valid: message buffer has 32 leading zero bytes.
    for &body in &[1usize, 16, 17, 64, 100, 1000] {
        let mlen = ZEROBYTES + body;
        let key = rng.vec(32);
        let nonce = rng.vec(24);
        let mut msg = vec![0u8; mlen];
        rng.fill(&mut msg[ZEROBYTES..]);

        let mut c_ct = vec![0u8; mlen];
        let mut r_ct = vec![0u8; mlen];
        let a = unsafe {
            c_box(c_ct.as_mut_ptr(), msg.as_ptr(), mlen as u64, nonce.as_ptr(), key.as_ptr())
        };
        let b = unsafe {
            r_box(r_ct.as_mut_ptr(), msg.as_ptr(), mlen as u64, nonce.as_ptr(), key.as_ptr())
        };
        assert_eq!(a, b, "{prefix}: nacl box rc body={body}");
        assert_eq!(a, 0, "{prefix}: nacl box ok");
        assert_eq!(c_ct, r_ct, "{prefix}: nacl ct body={body}");
        // first BOXZEROBYTES of ciphertext are zero
        assert!(c_ct[..BOXZEROBYTES].iter().all(|&x| x == 0));

        let mut c_pt = vec![0u8; mlen];
        let mut r_pt = vec![0u8; mlen];
        let oa = unsafe {
            c_open(c_pt.as_mut_ptr(), c_ct.as_ptr(), mlen as u64, nonce.as_ptr(), key.as_ptr())
        };
        let ob = unsafe {
            r_open(r_pt.as_mut_ptr(), r_ct.as_ptr(), mlen as u64, nonce.as_ptr(), key.as_ptr())
        };
        assert_eq!(oa, 0, "{prefix}: C nacl open");
        assert_eq!(ob, 0, "{prefix}: Rust nacl open");
        assert_eq!(c_pt, msg, "{prefix}: C nacl roundtrip");
        assert_eq!(r_pt, msg, "{prefix}: Rust nacl roundtrip");
    }

    // Error: mlen < 32 rejected by both.
    for &short in &[0usize, 1, 16, 31] {
        let key = rng.vec(32);
        let nonce = rng.vec(24);
        let msg = vec![0u8; short.max(32)];
        let mut c_ct = vec![0u8; 64];
        let mut r_ct = vec![0u8; 64];
        let a = unsafe {
            c_box(c_ct.as_mut_ptr(), msg.as_ptr(), short as u64, nonce.as_ptr(), key.as_ptr())
        };
        let b = unsafe {
            r_box(r_ct.as_mut_ptr(), msg.as_ptr(), short as u64, nonce.as_ptr(), key.as_ptr())
        };
        assert_eq!(a, -1, "{prefix}: C nacl short mlen={short}");
        assert_eq!(b, -1, "{prefix}: Rust nacl short mlen={short}");

        let mut c_pt = vec![0u8; 64];
        let mut r_pt = vec![0u8; 64];
        let oa = unsafe {
            c_open(c_pt.as_mut_ptr(), c_ct.as_ptr(), short as u64, nonce.as_ptr(), key.as_ptr())
        };
        let ob = unsafe {
            r_open(r_pt.as_mut_ptr(), r_ct.as_ptr(), short as u64, nonce.as_ptr(), key.as_ptr())
        };
        assert_eq!(oa, -1, "{prefix}: C nacl open short");
        assert_eq!(ob, -1, "{prefix}: Rust nacl open short");
    }

    // Error: tampered / wrong nonce on a valid box.
    {
        let key = rng.vec(32);
        let nonce = rng.vec(24);
        let mlen = ZEROBYTES + 48;
        let mut msg = vec![0u8; mlen];
        rng.fill(&mut msg[ZEROBYTES..]);
        let mut ct = vec![0u8; mlen];
        unsafe {
            c_box(ct.as_mut_ptr(), msg.as_ptr(), mlen as u64, nonce.as_ptr(), key.as_ptr());
        }
        let mut bad = ct.clone();
        bad[ZEROBYTES + 4] ^= 0x40;
        let mut c_pt = vec![0u8; mlen];
        let mut r_pt = vec![0u8; mlen];
        let a = unsafe {
            c_open(c_pt.as_mut_ptr(), bad.as_ptr(), mlen as u64, nonce.as_ptr(), key.as_ptr())
        };
        let b = unsafe {
            r_open(r_pt.as_mut_ptr(), bad.as_ptr(), mlen as u64, nonce.as_ptr(), key.as_ptr())
        };
        assert_eq!(a, -1, "{prefix}: C nacl tampered");
        assert_eq!(b, -1, "{prefix}: Rust nacl tampered");
    }
}

#[test]
fn secretbox_nacl_default() {
    secretbox_nacl_family("crypto_secretbox");
}

#[test]
fn secretbox_nacl_xsalsa20poly1305() {
    secretbox_nacl_family("crypto_secretbox_xsalsa20poly1305");
}

// =====================================================================
// SECRETSTREAM (xchacha20poly1305)
// =====================================================================

type SsInitPush = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> i32;
type SsInitPull = unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> i32;
// push: state, c, clen_p, m, mlen, ad, adlen, tag
type SsPush = unsafe extern "C" fn(
    *mut u8,
    *mut u8,
    *mut u64,
    *const u8,
    u64,
    *const u8,
    u64,
    u8,
) -> i32;
// pull: state, m, mlen_p, tag_p, c, clen, ad, adlen
type SsPull = unsafe extern "C" fn(
    *mut u8,
    *mut u8,
    *mut u64,
    *mut u8,
    *const u8,
    u64,
    *const u8,
    u64,
) -> i32;
type SsRekey = unsafe extern "C" fn(*mut u8);

#[test]
fn secretstream_roundtrip_and_errors() {
    let l = libs();
    let (c_sb, r_sb) = sympair!(
        l,
        b"crypto_secretstream_xchacha20poly1305_statebytes",
        unsafe extern "C" fn() -> usize
    );
    let (c_hb, _r_hb) = sympair!(
        l,
        b"crypto_secretstream_xchacha20poly1305_headerbytes",
        unsafe extern "C" fn() -> usize
    );
    let (c_ab, r_ab) = sympair!(
        l,
        b"crypto_secretstream_xchacha20poly1305_abytes",
        unsafe extern "C" fn() -> usize
    );
    let (c_kb, _r_kb) = sympair!(
        l,
        b"crypto_secretstream_xchacha20poly1305_keybytes",
        unsafe extern "C" fn() -> usize
    );

    let statebytes = unsafe { c_sb() };
    assert_eq!(statebytes, unsafe { r_sb() }, "ss statebytes");
    let headerbytes = unsafe { c_hb() };
    let abytes = unsafe { c_ab() };
    assert_eq!(abytes, unsafe { r_ab() }, "ss abytes");
    let keybytes = unsafe { c_kb() };

    // tag constants must match
    for tag_sym in [
        &b"crypto_secretstream_xchacha20poly1305_tag_message"[..],
        &b"crypto_secretstream_xchacha20poly1305_tag_push"[..],
        &b"crypto_secretstream_xchacha20poly1305_tag_rekey"[..],
        &b"crypto_secretstream_xchacha20poly1305_tag_final"[..],
    ] {
        let (c_t, r_t) = sympair!(l, tag_sym, unsafe extern "C" fn() -> u8);
        assert_eq!(unsafe { c_t() }, unsafe { r_t() }, "ss tag const parity");
    }

    let (c_ipush, r_ipush) =
        sympair!(l, b"crypto_secretstream_xchacha20poly1305_init_push", SsInitPush);
    let (c_ipull, r_ipull) =
        sympair!(l, b"crypto_secretstream_xchacha20poly1305_init_pull", SsInitPull);
    let (c_push, r_push) = sympair!(l, b"crypto_secretstream_xchacha20poly1305_push", SsPush);
    let (c_pull, r_pull) = sympair!(l, b"crypto_secretstream_xchacha20poly1305_pull", SsPull);
    let (c_rekey, r_rekey) =
        sympair!(l, b"crypto_secretstream_xchacha20poly1305_rekey", SsRekey);

    const TAG_MESSAGE: u8 = 0x00;
    const TAG_PUSH: u8 = 0x01;
    const TAG_REKEY: u8 = 0x02;
    const TAG_FINAL: u8 = 0x03;

    let mut rng = Rng::new(0x557_5757);

    // We drive C and Rust with the SAME header + key so that outputs must
    // match byte-for-byte. init_push normally randomizes the header, so we
    // seed C's header, then feed the identical header into Rust via init_pull
    // for both. To force identical streams we instead: run C init_push to get
    // a header, then init_pull BOTH libs from that header, and drive push via
    // the pull-initialized state? push requires a push-state. Simplest robust
    // approach: init_push on C, capture header; init_push on Rust would give a
    // different (random) header. So we compare the DECRYPT side across libs and
    // the ENCRYPT side within each lib, plus cross-decrypt C<->Rust.
    for &nmsg in &[1usize, 2, 5, 10] {
        // ----- C encrypts a whole stream -----
        let key = rng.vec(keybytes);
        let mut c_state = vec![0u8; statebytes];
        let mut header = vec![0u8; headerbytes];
        let ip = unsafe { c_ipush(c_state.as_mut_ptr(), header.as_mut_ptr(), key.as_ptr()) };
        assert_eq!(ip, 0, "C init_push");

        let mut messages: Vec<Vec<u8>> = Vec::new();
        let mut ads: Vec<Vec<u8>> = Vec::new();
        let mut tags: Vec<u8> = Vec::new();
        let mut ciphers: Vec<Vec<u8>> = Vec::new();

        for i in 0..nmsg {
            let mlen = *MSG_LENS.get(i % MSG_LENS.len()).unwrap();
            let adlen = *AAD_LENS.get(i % AAD_LENS.len()).unwrap();
            let m = rng.vec(mlen);
            let ad = rng.vec(adlen);
            let tag = if i + 1 == nmsg {
                TAG_FINAL
            } else if i % 3 == 2 {
                TAG_PUSH
            } else {
                TAG_MESSAGE
            };
            let ad_ptr = if adlen == 0 { std::ptr::null() } else { ad.as_ptr() };

            let mut ct = vec![0u8; mlen + abytes];
            let mut clen = 0u64;
            let rc = unsafe {
                c_push(
                    c_state.as_mut_ptr(),
                    ct.as_mut_ptr(),
                    &mut clen,
                    m.as_ptr(),
                    mlen as u64,
                    ad_ptr,
                    adlen as u64,
                    tag,
                )
            };
            assert_eq!(rc, 0, "C push i={i}");
            assert_eq!(clen as usize, mlen + abytes, "C push clen");
            messages.push(m);
            ads.push(ad);
            tags.push(tag);
            ciphers.push(ct);
        }

        // ----- Rust pulls (decrypts) that C stream, from the same header -----
        let mut r_pull_state = vec![0u8; statebytes];
        let ipr = unsafe { r_ipull(r_pull_state.as_mut_ptr(), header.as_ptr(), key.as_ptr()) };
        assert_eq!(ipr, 0, "Rust init_pull");
        // ----- C pulls its own stream too (sanity) -----
        let mut c_pull_state = vec![0u8; statebytes];
        let ipc = unsafe { c_ipull(c_pull_state.as_mut_ptr(), header.as_ptr(), key.as_ptr()) };
        assert_eq!(ipc, 0, "C init_pull");

        for i in 0..nmsg {
            let mlen = messages[i].len();
            let adlen = ads[i].len();
            let ad_ptr = if adlen == 0 { std::ptr::null() } else { ads[i].as_ptr() };
            let ct = &ciphers[i];

            let mut r_m = vec![0u8; mlen];
            let mut r_tag = 0xFFu8;
            let mut r_mlen = 0u64;
            let rr = unsafe {
                r_pull(
                    r_pull_state.as_mut_ptr(),
                    r_m.as_mut_ptr(),
                    &mut r_mlen,
                    &mut r_tag,
                    ct.as_ptr(),
                    ct.len() as u64,
                    ad_ptr,
                    adlen as u64,
                )
            };
            let mut c_m = vec![0u8; mlen];
            let mut c_tag = 0xFFu8;
            let mut c_mlen = 0u64;
            let rc = unsafe {
                c_pull(
                    c_pull_state.as_mut_ptr(),
                    c_m.as_mut_ptr(),
                    &mut c_mlen,
                    &mut c_tag,
                    ct.as_ptr(),
                    ct.len() as u64,
                    ad_ptr,
                    adlen as u64,
                )
            };
            assert_eq!(rr, 0, "Rust pull i={i}");
            assert_eq!(rc, 0, "C pull i={i}");
            assert_eq!(r_tag, tags[i], "Rust pull tag i={i}");
            assert_eq!(c_tag, tags[i], "C pull tag i={i}");
            assert_eq!(r_m, messages[i], "Rust pull plaintext i={i}");
            assert_eq!(c_m, messages[i], "C pull plaintext i={i}");
            assert_eq!(r_mlen, c_mlen, "pull mlen i={i}");
        }
    }

    // ----- Rust encrypts, C decrypts (exercise the Rust push path) -----
    for &nmsg in &[1usize, 3, 6] {
        let key = rng.vec(keybytes);
        let mut r_state = vec![0u8; statebytes];
        let mut header = vec![0u8; headerbytes];
        let ip = unsafe { r_ipush(r_state.as_mut_ptr(), header.as_mut_ptr(), key.as_ptr()) };
        assert_eq!(ip, 0, "Rust init_push");

        let mut messages: Vec<Vec<u8>> = Vec::new();
        let mut ads: Vec<Vec<u8>> = Vec::new();
        let mut tags: Vec<u8> = Vec::new();
        let mut ciphers: Vec<Vec<u8>> = Vec::new();
        for i in 0..nmsg {
            let mlen = *MSG_LENS.get((i * 3) % MSG_LENS.len()).unwrap();
            let adlen = *AAD_LENS.get(i % AAD_LENS.len()).unwrap();
            let m = rng.vec(mlen);
            let ad = rng.vec(adlen);
            let tag = if i + 1 == nmsg { TAG_FINAL } else { TAG_MESSAGE };
            let ad_ptr = if adlen == 0 { std::ptr::null() } else { ad.as_ptr() };
            let mut ct = vec![0u8; mlen + abytes];
            let mut clen = 0u64;
            let rc = unsafe {
                r_push(
                    r_state.as_mut_ptr(),
                    ct.as_mut_ptr(),
                    &mut clen,
                    m.as_ptr(),
                    mlen as u64,
                    ad_ptr,
                    adlen as u64,
                    tag,
                )
            };
            assert_eq!(rc, 0, "Rust push i={i}");
            messages.push(m);
            ads.push(ad);
            tags.push(tag);
            ciphers.push(ct);
        }
        // C decrypts the Rust-produced stream.
        let mut c_pull_state = vec![0u8; statebytes];
        assert_eq!(
            unsafe { c_ipull(c_pull_state.as_mut_ptr(), header.as_ptr(), key.as_ptr()) },
            0,
            "C init_pull of Rust header"
        );
        for i in 0..nmsg {
            let mlen = messages[i].len();
            let adlen = ads[i].len();
            let ad_ptr = if adlen == 0 { std::ptr::null() } else { ads[i].as_ptr() };
            let ct = &ciphers[i];
            let mut m = vec![0u8; mlen];
            let mut tag = 0xFFu8;
            let mut ml = 0u64;
            let rc = unsafe {
                c_pull(
                    c_pull_state.as_mut_ptr(),
                    m.as_mut_ptr(),
                    &mut ml,
                    &mut tag,
                    ct.as_ptr(),
                    ct.len() as u64,
                    ad_ptr,
                    adlen as u64,
                )
            };
            assert_eq!(rc, 0, "C pull of Rust stream i={i}");
            assert_eq!(m, messages[i], "C decrypt of Rust push i={i}");
            assert_eq!(tag, tags[i], "C tag of Rust push i={i}");
        }
    }

    // ----- Explicit TAG_REKEY / rekey() parity -----
    // Build a stream on C with an explicit rekey tag, plus a manual rekey(),
    // and confirm Rust reproduces the same ciphertext when driven identically.
    {
        let key = rng.vec(keybytes);
        let mut header = vec![0u8; headerbytes];
        let mut c_state = vec![0u8; statebytes];
        let mut r_state = vec![0u8; statebytes];
        unsafe {
            c_ipush(c_state.as_mut_ptr(), header.as_mut_ptr(), key.as_ptr());
        }
        // Rust push-state seeded from the SAME header via init_pull won't work
        // for pushing; instead init_push on Rust reusing C's header by copying
        // the C state is not portable. So we validate rekey by comparing the
        // C encrypt stream against a C decrypt stream that also rekeys, and a
        // Rust decrypt of the same, ensuring rekey semantics match.
        let msgs: [(&[u8], u8); 4] = [
            (b"first".as_slice(), TAG_MESSAGE),
            (b"second-with-explicit-rekey".as_slice(), TAG_REKEY),
            (b"third".as_slice(), TAG_MESSAGE),
            (b"final-message".as_slice(), TAG_FINAL),
        ];
        let mut cts: Vec<Vec<u8>> = Vec::new();
        for (idx, (m, tag)) in msgs.iter().enumerate() {
            let mut ct = vec![0u8; m.len() + abytes];
            let mut clen = 0u64;
            unsafe {
                c_push(
                    c_state.as_mut_ptr(),
                    ct.as_mut_ptr(),
                    &mut clen,
                    m.as_ptr(),
                    m.len() as u64,
                    std::ptr::null(),
                    0,
                    *tag,
                );
            }
            // Manually rekey after the 3rd message on the encrypt side.
            if idx == 2 {
                unsafe { c_rekey(c_state.as_mut_ptr()) };
            }
            cts.push(ct);
        }

        // Decrypt with C and Rust; both must rekey after msg idx 2.
        unsafe {
            c_ipull(c_state.as_mut_ptr(), header.as_ptr(), key.as_ptr());
            r_ipull(r_state.as_mut_ptr(), header.as_ptr(), key.as_ptr());
        }
        for (idx, (m, tag)) in msgs.iter().enumerate() {
            let ct = &cts[idx];
            let mut cm = vec![0u8; m.len()];
            let mut rm = vec![0u8; m.len()];
            let mut ct_tag = 0u8;
            let mut rt_tag = 0u8;
            let mut cl = 0u64;
            let mut rl = 0u64;
            let rc = unsafe {
                c_pull(
                    c_state.as_mut_ptr(),
                    cm.as_mut_ptr(),
                    &mut cl,
                    &mut ct_tag,
                    ct.as_ptr(),
                    ct.len() as u64,
                    std::ptr::null(),
                    0,
                )
            };
            let rr = unsafe {
                r_pull(
                    r_state.as_mut_ptr(),
                    rm.as_mut_ptr(),
                    &mut rl,
                    &mut rt_tag,
                    ct.as_ptr(),
                    ct.len() as u64,
                    std::ptr::null(),
                    0,
                )
            };
            assert_eq!(rc, 0, "C rekey-stream pull idx={idx}");
            assert_eq!(rr, 0, "Rust rekey-stream pull idx={idx}");
            assert_eq!(cm, *m, "C rekey pt idx={idx}");
            assert_eq!(rm, *m, "Rust rekey pt idx={idx}");
            assert_eq!(ct_tag, *tag, "C rekey tag idx={idx}");
            assert_eq!(rt_tag, *tag, "Rust rekey tag idx={idx}");
            if idx == 2 {
                unsafe {
                    c_rekey(c_state.as_mut_ptr());
                    r_rekey(r_state.as_mut_ptr());
                }
            }
        }
    }

    // ----- Phase C: secretstream error paths -----
    {
        let key = rng.vec(keybytes);
        let mut header = vec![0u8; headerbytes];
        let mut c_state = vec![0u8; statebytes];
        unsafe {
            c_ipush(c_state.as_mut_ptr(), header.as_mut_ptr(), key.as_ptr());
        }
        let msg = rng.vec(64);
        let mut ct = vec![0u8; 64 + abytes];
        let mut clen = 0u64;
        unsafe {
            c_push(
                c_state.as_mut_ptr(),
                ct.as_mut_ptr(),
                &mut clen,
                msg.as_ptr(),
                64,
                std::ptr::null(),
                0,
                TAG_MESSAGE,
            );
        }

        // (a) tampered ciphertext -> -1 on both
        let mut c_state2 = vec![0u8; statebytes];
        let mut r_state2 = vec![0u8; statebytes];
        let run_pull = |ipull: &SsInitPull,
                        pull: &SsPull,
                        state: &mut [u8],
                        header: &[u8],
                        key: &[u8],
                        ct: &[u8]|
         -> i32 {
            unsafe {
                ipull(state.as_mut_ptr(), header.as_ptr(), key.as_ptr());
            }
            let mut m = vec![0u8; ct.len()];
            let mut tag = 0u8;
            let mut ml = 0u64;
            unsafe {
                pull(
                    state.as_mut_ptr(),
                    m.as_mut_ptr(),
                    &mut ml,
                    &mut tag,
                    ct.as_ptr(),
                    ct.len() as u64,
                    std::ptr::null(),
                    0,
                )
            }
        };

        let mut bad = ct.clone();
        bad[10] ^= 0x33;
        let a = run_pull(&c_ipull, &c_pull, &mut c_state2, &header, &key, &bad);
        let b = run_pull(&r_ipull, &r_pull, &mut r_state2, &header, &key, &bad);
        assert_eq!(a, -1, "C ss tampered ct");
        assert_eq!(b, -1, "Rust ss tampered ct");

        // (b) truncated (< abytes) -> -1 on both
        for &trunc in &[0usize, 1, abytes - 1] {
            let a = run_pull(&c_ipull, &c_pull, &mut c_state2, &header, &key, &ct[..trunc]);
            let b = run_pull(&r_ipull, &r_pull, &mut r_state2, &header, &key, &ct[..trunc]);
            assert_eq!(a, -1, "C ss truncated {trunc}");
            assert_eq!(b, -1, "Rust ss truncated {trunc}");
        }

        // (c) wrong key in init_pull -> auth failure -> -1 on both
        let mut bad_key = key.clone();
        bad_key[0] ^= 1;
        let a = run_pull(&c_ipull, &c_pull, &mut c_state2, &header, &bad_key, &ct);
        let b = run_pull(&r_ipull, &r_pull, &mut r_state2, &header, &bad_key, &ct);
        assert_eq!(a, -1, "C ss wrong key");
        assert_eq!(b, -1, "Rust ss wrong key");

        // (d) tampered header -> auth failure -> -1 on both
        let mut bad_hdr = header.clone();
        bad_hdr[0] ^= 1;
        let a = run_pull(&c_ipull, &c_pull, &mut c_state2, &bad_hdr, &key, &ct);
        let b = run_pull(&r_ipull, &r_pull, &mut r_state2, &bad_hdr, &key, &ct);
        assert_eq!(a, -1, "C ss tampered header");
        assert_eq!(b, -1, "Rust ss tampered header");
    }
}

/// Constant/introspection parity across the whole family (sizes & primitive).
#[test]
fn size_and_primitive_parity() {
    let l = libs();
    let size_syms: &[&[u8]] = &[
        b"crypto_aead_chacha20poly1305_keybytes",
        b"crypto_aead_chacha20poly1305_npubbytes",
        b"crypto_aead_chacha20poly1305_nsecbytes",
        b"crypto_aead_chacha20poly1305_abytes",
        b"crypto_aead_chacha20poly1305_messagebytes_max",
        b"crypto_aead_chacha20poly1305_ietf_keybytes",
        b"crypto_aead_chacha20poly1305_ietf_npubbytes",
        b"crypto_aead_chacha20poly1305_ietf_abytes",
        b"crypto_aead_xchacha20poly1305_ietf_keybytes",
        b"crypto_aead_xchacha20poly1305_ietf_npubbytes",
        b"crypto_aead_xchacha20poly1305_ietf_abytes",
        b"crypto_aead_aegis128l_keybytes",
        b"crypto_aead_aegis128l_npubbytes",
        b"crypto_aead_aegis128l_abytes",
        b"crypto_aead_aegis256_keybytes",
        b"crypto_aead_aegis256_npubbytes",
        b"crypto_aead_aegis256_abytes",
        b"crypto_aead_aes256gcm_keybytes",
        b"crypto_aead_aes256gcm_npubbytes",
        b"crypto_aead_aes256gcm_abytes",
        b"crypto_aead_aes256gcm_statebytes",
        b"crypto_secretbox_keybytes",
        b"crypto_secretbox_noncebytes",
        b"crypto_secretbox_macbytes",
        b"crypto_secretbox_zerobytes",
        b"crypto_secretbox_boxzerobytes",
        b"crypto_secretbox_xchacha20poly1305_keybytes",
        b"crypto_secretbox_xchacha20poly1305_noncebytes",
        b"crypto_secretbox_xchacha20poly1305_macbytes",
        b"crypto_secretbox_xsalsa20poly1305_keybytes",
        b"crypto_secretbox_xsalsa20poly1305_noncebytes",
        b"crypto_secretbox_xsalsa20poly1305_macbytes",
        b"crypto_secretbox_xsalsa20poly1305_zerobytes",
        b"crypto_secretbox_xsalsa20poly1305_boxzerobytes",
        b"crypto_secretstream_xchacha20poly1305_abytes",
        b"crypto_secretstream_xchacha20poly1305_headerbytes",
        b"crypto_secretstream_xchacha20poly1305_keybytes",
        b"crypto_secretstream_xchacha20poly1305_statebytes",
    ];
    for sym in size_syms {
        let (c_f, r_f) = sympair!(l, sym, unsafe extern "C" fn() -> usize);
        assert_eq!(
            unsafe { c_f() },
            unsafe { r_f() },
            "size divergence: {}",
            std::str::from_utf8(sym).unwrap()
        );
    }

    let (c_p, r_p) =
        sympair!(l, b"crypto_secretbox_primitive", unsafe extern "C" fn() -> *const c_char);
    let cs = unsafe { std::ffi::CStr::from_ptr(c_p()) };
    let rs = unsafe { std::ffi::CStr::from_ptr(r_p()) };
    assert_eq!(cs, rs, "secretbox primitive string");
}
