//! Phase C — ERROR-PATH differential tests, ERRORS.md rows 131–206.
//!
//! For every invalid-input condition we construct that exact condition, drive
//! BOTH the C `.so` and the Rust `.so`, and assert they agree on the *observable
//! error surface*: identical integer return / sentinel / errno / zeroed output,
//! or — for the `sodium_misuse()`→`abort()` paths — the identical process fate
//! in a forked child (`Fate::Signaled(6)` for `SIGABRT`, or both `Exited(0)`).
//!
//! Build configuration note (drives reachability):
//!   * The C `.so` is compiled with NO `HAVE_*` feature macros, so AES-NI is not
//!     compiled. `crypto_aead_aes256gcm_is_available()` returns 0 and every
//!     aes256gcm entry point takes the `errno = ENOSYS; return -1;` fallback
//!     (rows 131–140).
//!   * On a 64-bit host `MESSAGEBYTES_MAX` for the non-ietf AEADs / secretbox /
//!     box `_easy` is `2^64 − ABYTES`, which cannot be reached because the OS
//!     cannot allocate a >16-EiB buffer and `unsigned long long mlen` cannot
//!     exceed `SIZE_MAX`; those `misuse` rows are documented unreachable.
//!     The chacha20poly1305_ietf `MESSAGEBYTES_MAX` is `2^38 − 64`, which *is*
//!     reachable by declaring a huge `mlen` against a small real buffer — the C
//!     checks the length before writing — so we exercise it under `same_fate`.

mod common;
use common::*;

// ---- errno numbers on Linux (per task spec) ----
const ENOSYS: i32 = 38;

// ===========================================================================
// Exact C signatures (from include/sodium/*.h)
// ===========================================================================

// All AEAD encrypt: (c, clen_p, m, mlen, ad, adlen, nsec, npub, k) -> i32
type AeadEnc = unsafe extern "C" fn(
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
// All AEAD decrypt: (m, mlen_p, nsec, c, clen, ad, adlen, npub, k) -> i32
type AeadDec = unsafe extern "C" fn(
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
// encrypt_detached: (c, mac, maclen_p, m, mlen, ad, adlen, nsec, npub, k) -> i32
type AeadEncDet = unsafe extern "C" fn(
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
// decrypt_detached: (m, nsec, c, clen, mac, ad, adlen, npub, k) -> i32
type AeadDecDet = unsafe extern "C" fn(
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

// aes256gcm beforenm: (ctx, k) -> i32 ; afternm variants take ctx in place of k.
type AesBeforenm = unsafe extern "C" fn(*mut u8, *const u8) -> i32;
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
type AesEncDetAfternm = unsafe extern "C" fn(
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
type AesDecDetAfternm = unsafe extern "C" fn(
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
type IntFn = unsafe extern "C" fn() -> i32;

// secretbox_xsalsa20poly1305: (c, m, mlen, n, k) -> i32 (NaCl padded interface)
type SbNacl = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, *const u8) -> i32;
// secretbox_easy / open_easy: (c/m, m/c, len, n, k) -> i32
type SbEasy = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, *const u8) -> i32;
// open_detached: (m, c, mac, clen, n, k) -> i32
type SbOpenDet =
    unsafe extern "C" fn(*mut u8, *const u8, *const u8, u64, *const u8, *const u8) -> i32;
// detached (seal): (c, mac, m, mlen, n, k) -> i32
type SbDet = unsafe extern "C" fn(*mut u8, *mut u8, *const u8, u64, *const u8, *const u8) -> i32;

// secretstream state ops
type SizeFn = unsafe extern "C" fn() -> usize;
type SsInitPush = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> i32;
type SsInitPull = unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> i32;
type SsPush =
    unsafe extern "C" fn(*mut u8, *mut u8, *mut u64, *const u8, u64, *const u8, u64, u8) -> i32;
type SsPull =
    unsafe extern "C" fn(*mut u8, *mut u8, *mut u64, *mut u8, *const u8, u64, *const u8, u64) -> i32;

// box beforenm: (k, pk, sk) -> i32
type BoxBeforenm = unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> i32;
// box keypair: (pk, sk) -> i32
type BoxKeypair = unsafe extern "C" fn(*mut u8, *mut u8) -> i32;
// NaCl box / open: (c/m, m/c, len, n, pk, sk) -> i32
type BoxNacl =
    unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, *const u8, *const u8) -> i32;
// detached: (c, mac, m, mlen, n, pk, sk) -> i32
type BoxDet =
    unsafe extern "C" fn(*mut u8, *mut u8, *const u8, u64, *const u8, *const u8, *const u8) -> i32;
// open_detached: (m, c, mac, clen, n, pk, sk) -> i32
type BoxOpenDet =
    unsafe extern "C" fn(*mut u8, *const u8, *const u8, u64, *const u8, *const u8, *const u8) -> i32;
// open_easy: (m, c, clen, n, pk, sk) -> i32
type BoxOpenEasy =
    unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, *const u8, *const u8) -> i32;
// open_easy_afternm: (m, c, clen, n, k) -> i32
type BoxOpenEasyAfternm = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, *const u8) -> i32;
// seal: (c, m, mlen, pk) -> i32
type BoxSeal = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8) -> i32;
// seal_open: (m, c, clen, pk, sk) -> i32
type BoxSealOpen = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, *const u8) -> i32;

// ===========================================================================
// Small-order curve25519 public keys — read verbatim from
// c_src/libsodium/crypto_scalarmult/curve25519/ref10/x25519_ref10.c
// `has_small_order()` blocklist (7 entries; the C source is ground truth —
// the count is 7, not 12).  A pk equal to any of these makes
// crypto_scalarmult_curve25519 (and therefore crypto_box_*_beforenm) return -1.
// ===========================================================================
const SMALL_ORDER: [[u8; 32]; 7] = [
    // 0 (order 4)
    [0; 32],
    // 1 (order 1)
    [
        0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00,
    ],
    // 325606...  (order 8)
    [
        0xe0, 0xeb, 0x7a, 0x7c, 0x3b, 0x41, 0xb8, 0xae, 0x16, 0x56, 0xe3, 0xfa, 0xf1, 0x9f, 0xc4,
        0x6a, 0xda, 0x09, 0x8d, 0xeb, 0x9c, 0x32, 0xb1, 0xfd, 0x86, 0x62, 0x05, 0x16, 0x5f, 0x49,
        0xb8, 0x00,
    ],
    // 393823...  (order 8)
    [
        0x5f, 0x9c, 0x95, 0xbc, 0xa3, 0x50, 0x8c, 0x24, 0xb1, 0xd0, 0xb1, 0x55, 0x9c, 0x83, 0xef,
        0x5b, 0x04, 0x44, 0x5c, 0xc4, 0x58, 0x1c, 0x8e, 0x86, 0xd8, 0x22, 0x4e, 0xdd, 0xd0, 0x9f,
        0x11, 0x57,
    ],
    // p-1 (order 2)
    [
        0xec, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0x7f,
    ],
    // p (=0, order 4)
    [
        0xed, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0x7f,
    ],
    // p+1 (=1, order 1)
    [
        0xee, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0x7f,
    ],
];

// ===========================================================================
// 7.a  aes256gcm — rows 131–140
// ===========================================================================

/// ERRORS.md row 131: `crypto_aead_aes256gcm_is_available` must return the SAME
/// value from both libs. In this portable (no AES-NI) build that value is 0.
#[test]
fn aes256gcm_is_available_matches() {
    let d = duo();
    let (cf, rf) = d.pair::<IntFn>("crypto_aead_aes256gcm_is_available");
    let c = unsafe { (*cf)() };
    let r = unsafe { (*rf)() };
    eq_i32("aes256gcm_is_available", c, r);
    assert_eq!(c, 0, "portable build must report aes256gcm unavailable");
}

/// ERRORS.md rows 132–140: ALL NINE aes256gcm entry points return -1 with
/// errno == ENOSYS(38) identically from both libs, in this no-AES-NI build.
#[test]
fn aes256gcm_all_entry_points_enosys() {
    let d = duo();

    let key = [0u8; 32];
    let npub = [0u8; 12];
    let ctx = [0u8; 512]; // >= statebytes; contents irrelevant (checked before use)
    let msg = [0u8; 16];
    let mut out = [0u8; 64];
    let mut mac = [0u8; 16];
    let mut clen: u64 = 0;

    // row 132 encrypt
    {
        let (cf, rf) = d.pair::<AeadEnc>("crypto_aead_aes256gcm_encrypt");
        let (rc, ce) = with_errno(|| unsafe {
            (*cf)(out.as_mut_ptr(), &mut clen, msg.as_ptr(), msg.len() as u64,
                  std::ptr::null(), 0, std::ptr::null(), npub.as_ptr(), key.as_ptr())
        });
        let (rr, re) = with_errno(|| unsafe {
            (*rf)(out.as_mut_ptr(), &mut clen, msg.as_ptr(), msg.len() as u64,
                  std::ptr::null(), 0, std::ptr::null(), npub.as_ptr(), key.as_ptr())
        });
        eq_i32("aes256gcm_encrypt ret", rc, rr);
        eq_i32("aes256gcm_encrypt errno", ce, re);
        assert_eq!((rc, ce), (-1, ENOSYS));
    }

    // row 133 encrypt_detached
    {
        let (cf, rf) = d.pair::<AeadEncDet>("crypto_aead_aes256gcm_encrypt_detached");
        let (rc, ce) = with_errno(|| unsafe {
            (*cf)(out.as_mut_ptr(), mac.as_mut_ptr(), std::ptr::null_mut(),
                  msg.as_ptr(), msg.len() as u64, std::ptr::null(), 0,
                  std::ptr::null(), npub.as_ptr(), key.as_ptr())
        });
        let (rr, re) = with_errno(|| unsafe {
            (*rf)(out.as_mut_ptr(), mac.as_mut_ptr(), std::ptr::null_mut(),
                  msg.as_ptr(), msg.len() as u64, std::ptr::null(), 0,
                  std::ptr::null(), npub.as_ptr(), key.as_ptr())
        });
        eq_i32("aes256gcm_encrypt_detached ret", rc, rr);
        eq_i32("aes256gcm_encrypt_detached errno", ce, re);
        assert_eq!((rc, ce), (-1, ENOSYS));
    }

    // row 134 decrypt
    {
        let (cf, rf) = d.pair::<AeadDec>("crypto_aead_aes256gcm_decrypt");
        let ct = [0u8; 32];
        let mut mlen: u64 = 0;
        let (rc, ce) = with_errno(|| unsafe {
            (*cf)(out.as_mut_ptr(), &mut mlen, std::ptr::null_mut(), ct.as_ptr(),
                  ct.len() as u64, std::ptr::null(), 0, npub.as_ptr(), key.as_ptr())
        });
        let (rr, re) = with_errno(|| unsafe {
            (*rf)(out.as_mut_ptr(), &mut mlen, std::ptr::null_mut(), ct.as_ptr(),
                  ct.len() as u64, std::ptr::null(), 0, npub.as_ptr(), key.as_ptr())
        });
        eq_i32("aes256gcm_decrypt ret", rc, rr);
        eq_i32("aes256gcm_decrypt errno", ce, re);
        assert_eq!((rc, ce), (-1, ENOSYS));
    }

    // row 135 decrypt_detached
    {
        let (cf, rf) = d.pair::<AeadDecDet>("crypto_aead_aes256gcm_decrypt_detached");
        let ct = [0u8; 16];
        let (rc, ce) = with_errno(|| unsafe {
            (*cf)(out.as_mut_ptr(), std::ptr::null_mut(), ct.as_ptr(), ct.len() as u64,
                  mac.as_ptr(), std::ptr::null(), 0, npub.as_ptr(), key.as_ptr())
        });
        let (rr, re) = with_errno(|| unsafe {
            (*rf)(out.as_mut_ptr(), std::ptr::null_mut(), ct.as_ptr(), ct.len() as u64,
                  mac.as_ptr(), std::ptr::null(), 0, npub.as_ptr(), key.as_ptr())
        });
        eq_i32("aes256gcm_decrypt_detached ret", rc, rr);
        eq_i32("aes256gcm_decrypt_detached errno", ce, re);
        assert_eq!((rc, ce), (-1, ENOSYS));
    }

    // row 136 beforenm
    {
        let (cf, rf) = d.pair::<AesBeforenm>("crypto_aead_aes256gcm_beforenm");
        let mut cctx = ctx;
        let mut rctx = ctx;
        let (rc, ce) = with_errno(|| unsafe { (*cf)(cctx.as_mut_ptr(), key.as_ptr()) });
        let (rr, re) = with_errno(|| unsafe { (*rf)(rctx.as_mut_ptr(), key.as_ptr()) });
        eq_i32("aes256gcm_beforenm ret", rc, rr);
        eq_i32("aes256gcm_beforenm errno", ce, re);
        assert_eq!((rc, ce), (-1, ENOSYS));
    }

    // row 137 encrypt_afternm
    {
        let (cf, rf) = d.pair::<AesEncAfternm>("crypto_aead_aes256gcm_encrypt_afternm");
        let (rc, ce) = with_errno(|| unsafe {
            (*cf)(out.as_mut_ptr(), &mut clen, msg.as_ptr(), msg.len() as u64,
                  std::ptr::null(), 0, std::ptr::null(), npub.as_ptr(), ctx.as_ptr())
        });
        let (rr, re) = with_errno(|| unsafe {
            (*rf)(out.as_mut_ptr(), &mut clen, msg.as_ptr(), msg.len() as u64,
                  std::ptr::null(), 0, std::ptr::null(), npub.as_ptr(), ctx.as_ptr())
        });
        eq_i32("aes256gcm_encrypt_afternm ret", rc, rr);
        eq_i32("aes256gcm_encrypt_afternm errno", ce, re);
        assert_eq!((rc, ce), (-1, ENOSYS));
    }

    // row 138 encrypt_detached_afternm
    {
        let (cf, rf) = d.pair::<AesEncDetAfternm>("crypto_aead_aes256gcm_encrypt_detached_afternm");
        let (rc, ce) = with_errno(|| unsafe {
            (*cf)(out.as_mut_ptr(), mac.as_mut_ptr(), std::ptr::null_mut(),
                  msg.as_ptr(), msg.len() as u64, std::ptr::null(), 0,
                  std::ptr::null(), npub.as_ptr(), ctx.as_ptr())
        });
        let (rr, re) = with_errno(|| unsafe {
            (*rf)(out.as_mut_ptr(), mac.as_mut_ptr(), std::ptr::null_mut(),
                  msg.as_ptr(), msg.len() as u64, std::ptr::null(), 0,
                  std::ptr::null(), npub.as_ptr(), ctx.as_ptr())
        });
        eq_i32("aes256gcm_encrypt_detached_afternm ret", rc, rr);
        eq_i32("aes256gcm_encrypt_detached_afternm errno", ce, re);
        assert_eq!((rc, ce), (-1, ENOSYS));
    }

    // row 139 decrypt_afternm
    {
        let (cf, rf) = d.pair::<AesDecAfternm>("crypto_aead_aes256gcm_decrypt_afternm");
        let ct = [0u8; 32];
        let mut mlen: u64 = 0;
        let (rc, ce) = with_errno(|| unsafe {
            (*cf)(out.as_mut_ptr(), &mut mlen, std::ptr::null_mut(), ct.as_ptr(),
                  ct.len() as u64, std::ptr::null(), 0, npub.as_ptr(), ctx.as_ptr())
        });
        let (rr, re) = with_errno(|| unsafe {
            (*rf)(out.as_mut_ptr(), &mut mlen, std::ptr::null_mut(), ct.as_ptr(),
                  ct.len() as u64, std::ptr::null(), 0, npub.as_ptr(), ctx.as_ptr())
        });
        eq_i32("aes256gcm_decrypt_afternm ret", rc, rr);
        eq_i32("aes256gcm_decrypt_afternm errno", ce, re);
        assert_eq!((rc, ce), (-1, ENOSYS));
    }

    // row 140 decrypt_detached_afternm
    {
        let (cf, rf) = d.pair::<AesDecDetAfternm>("crypto_aead_aes256gcm_decrypt_detached_afternm");
        let ct = [0u8; 16];
        let (rc, ce) = with_errno(|| unsafe {
            (*cf)(out.as_mut_ptr(), std::ptr::null_mut(), ct.as_ptr(), ct.len() as u64,
                  mac.as_ptr(), std::ptr::null(), 0, npub.as_ptr(), ctx.as_ptr())
        });
        let (rr, re) = with_errno(|| unsafe {
            (*rf)(out.as_mut_ptr(), std::ptr::null_mut(), ct.as_ptr(), ct.len() as u64,
                  mac.as_ptr(), std::ptr::null(), 0, npub.as_ptr(), ctx.as_ptr())
        });
        eq_i32("aes256gcm_decrypt_detached_afternm ret", rc, rr);
        eq_i32("aes256gcm_decrypt_detached_afternm errno", ce, re);
        assert_eq!((rc, ce), (-1, ENOSYS));
    }
}

// ===========================================================================
// Generic poly1305-family AEAD error harness (chacha / ietf / xchacha_ietf)
// ===========================================================================

/// Encrypt a valid ciphertext with `enc`, using the given key/nonce sizes.
fn aead_seal(
    enc: &libloading::Symbol<'static, AeadEnc>,
    m: &[u8],
    ad: &[u8],
    npub: &[u8],
    key: &[u8],
    abytes: usize,
) -> Vec<u8> {
    let mut c = vec![0u8; m.len() + abytes];
    let mut clen: u64 = 0;
    let r = unsafe {
        (**enc)(
            c.as_mut_ptr(),
            &mut clen,
            m.as_ptr(),
            m.len() as u64,
            if ad.is_empty() { std::ptr::null() } else { ad.as_ptr() },
            ad.len() as u64,
            std::ptr::null(),
            npub.as_ptr(),
            key.as_ptr(),
        )
    };
    assert_eq!(r, 0, "aead_seal failed");
    c.truncate(clen as usize);
    c
}

/// Run BOTH `decrypt` impls on identical inputs and assert identical ret + m.
fn aead_dec_pair(
    dec_c: &libloading::Symbol<'static, AeadDec>,
    dec_r: &libloading::Symbol<'static, AeadDec>,
    c: &[u8],
    ad: &[u8],
    npub: &[u8],
    key: &[u8],
    mlen_out: usize,
    what: &str,
) {
    let mut mc = vec![0xAAu8; mlen_out.max(1)];
    let mut mr = vec![0xAAu8; mlen_out.max(1)];
    let mut mlc: u64 = 0;
    let mut mlr: u64 = 0;
    let adp = if ad.is_empty() { std::ptr::null() } else { ad.as_ptr() };
    let rc = unsafe {
        (**dec_c)(mc.as_mut_ptr(), &mut mlc, std::ptr::null_mut(), c.as_ptr(),
                  c.len() as u64, adp, ad.len() as u64, npub.as_ptr(), key.as_ptr())
    };
    let rr = unsafe {
        (**dec_r)(mr.as_mut_ptr(), &mut mlr, std::ptr::null_mut(), c.as_ptr(),
                  c.len() as u64, adp, ad.len() as u64, npub.as_ptr(), key.as_ptr())
    };
    eq_i32(&format!("{what} ret"), rc, rr);
    // On failure the C decrypt zeroes m; compare the whole output buffer so we
    // catch any zeroing divergence.
    eq_bytes(&format!("{what} m"), &mc, &mr);
}

/// ERRORS.md rows 141, 142, 144 (chacha20poly1305 combined decrypt):
///  * 141: `clen < ABYTES (16)` → -1.
///  * 142: tampered ciphertext / tampered MAC → -1 and `m` zeroed identically.
///  * 144: wrong AD → -1. Also wrong nonce / wrong key → -1, `m` zeroed.
/// Every MAC byte and a sample of ciphertext bytes are flipped.
#[test]
fn chacha20poly1305_decrypt_errors() {
    let d = duo();
    let (enc_c, _enc_r) = d.pair::<AeadEnc>("crypto_aead_chacha20poly1305_encrypt");
    let (dec_c, dec_r) = d.pair::<AeadDec>("crypto_aead_chacha20poly1305_decrypt");
    aead_poly_decrypt_suite(&enc_c, &dec_c, &dec_r, 16, 8, 32, "chacha20poly1305");
}

/// ERRORS.md rows 146, 147 (chacha20poly1305_ietf combined decrypt): short clen,
/// tampered ct/MAC/AD/nonce → -1 with identical `m`.
#[test]
fn chacha20poly1305_ietf_decrypt_errors() {
    let d = duo();
    let (enc_c, _) = d.pair::<AeadEnc>("crypto_aead_chacha20poly1305_ietf_encrypt");
    let (dec_c, dec_r) = d.pair::<AeadDec>("crypto_aead_chacha20poly1305_ietf_decrypt");
    aead_poly_decrypt_suite(&enc_c, &dec_c, &dec_r, 16, 12, 32, "chacha20poly1305_ietf");
}

/// ERRORS.md rows 150, 151 (xchacha20poly1305_ietf combined decrypt): short clen,
/// tampered ct/MAC/AD/nonce → -1 with identical `m`.
#[test]
fn xchacha20poly1305_ietf_decrypt_errors() {
    let d = duo();
    let (enc_c, _) = d.pair::<AeadEnc>("crypto_aead_xchacha20poly1305_ietf_encrypt");
    let (dec_c, dec_r) = d.pair::<AeadDec>("crypto_aead_xchacha20poly1305_ietf_decrypt");
    aead_poly_decrypt_suite(&enc_c, &dec_c, &dec_r, 16, 24, 32, "xchacha20poly1305_ietf");
}

fn aead_poly_decrypt_suite(
    enc_c: &libloading::Symbol<'static, AeadEnc>,
    dec_c: &libloading::Symbol<'static, AeadDec>,
    dec_r: &libloading::Symbol<'static, AeadDec>,
    abytes: usize,
    nlen: usize,
    klen: usize,
    tag: &str,
) {
    let mut rng = Rng::new(0xA5A5_0000 ^ (tag.len() as u64));
    let key = rng.bytes(klen);
    let npub = rng.bytes(nlen);
    let ad = rng.bytes(11);
    let msg = rng.bytes(40);

    // Short clen: 0..ABYTES-1 → both must fail identically (m untouched/zeroed).
    for clen in 0..abytes {
        let ct = rng.bytes(clen);
        aead_dec_pair(dec_c, dec_r, &ct, &ad, &npub, &key, 0, &format!("{tag} short clen={clen}"));
    }

    // Valid ciphertext produced by C (both libs share the algorithm).
    let ct = aead_seal(enc_c, &msg, &ad, &npub, &key, abytes);

    // Tampered MAC: flip every bit of every MAC byte (last ABYTES bytes).
    for i in (ct.len() - abytes)..ct.len() {
        for bit in 0..8 {
            let mut bad = ct.clone();
            bad[i] ^= 1 << bit;
            aead_dec_pair(dec_c, dec_r, &bad, &ad, &npub, &key, msg.len(),
                          &format!("{tag} MAC-flip byte{i} bit{bit}"));
        }
    }
    // Tampered ciphertext body: sample several positions.
    let body = ct.len() - abytes;
    for &i in &[0usize, 1, body / 2, body.saturating_sub(1)] {
        if i >= body { continue; }
        let mut bad = ct.clone();
        bad[i] ^= 0x80;
        aead_dec_pair(dec_c, dec_r, &bad, &ad, &npub, &key, msg.len(),
                      &format!("{tag} CT-flip byte{i}"));
    }
    // Wrong AD.
    let mut wrong_ad = ad.clone();
    wrong_ad[0] ^= 0xff;
    aead_dec_pair(dec_c, dec_r, &ct, &wrong_ad, &npub, &key, msg.len(), &format!("{tag} wrong AD"));
    // Wrong nonce.
    let mut wrong_n = npub.clone();
    wrong_n[0] ^= 0xff;
    aead_dec_pair(dec_c, dec_r, &ct, &ad, &wrong_n, &key, msg.len(), &format!("{tag} wrong nonce"));
    // Wrong key.
    let mut wrong_k = key.clone();
    wrong_k[0] ^= 0xff;
    aead_dec_pair(dec_c, dec_r, &ct, &ad, &npub, &wrong_k, msg.len(), &format!("{tag} wrong key"));
}

/// Run BOTH decrypt_detached impls and assert identical ret + m.
fn aead_dec_det_pair(
    dc: &libloading::Symbol<'static, AeadDecDet>,
    dr: &libloading::Symbol<'static, AeadDecDet>,
    ct: &[u8],
    mac: &[u8],
    ad: &[u8],
    npub: &[u8],
    key: &[u8],
    what: &str,
) {
    let mut mc = vec![0xAAu8; ct.len().max(1)];
    let mut mr = vec![0xAAu8; ct.len().max(1)];
    let adp = if ad.is_empty() { std::ptr::null() } else { ad.as_ptr() };
    let rc = unsafe {
        (**dc)(mc.as_mut_ptr(), std::ptr::null_mut(), ct.as_ptr(), ct.len() as u64,
               mac.as_ptr(), adp, ad.len() as u64, npub.as_ptr(), key.as_ptr())
    };
    let rr = unsafe {
        (**dr)(mr.as_mut_ptr(), std::ptr::null_mut(), ct.as_ptr(), ct.len() as u64,
               mac.as_ptr(), adp, ad.len() as u64, npub.as_ptr(), key.as_ptr())
    };
    eq_i32(&format!("{what} ret"), rc, rr);
    eq_bytes(&format!("{what} m"), &mc, &mr);
}

/// ERRORS.md rows 143, 148, 152 (poly1305-family decrypt_detached, wrong MAC →
/// -1 and `m` zeroed identically). Every MAC byte is bit-flipped.
#[test]
fn poly1305_decrypt_detached_wrong_mac() {
    let d = duo();
    for (enc, dec, nlen, klen, tag) in [
        ("crypto_aead_chacha20poly1305_encrypt_detached",
         "crypto_aead_chacha20poly1305_decrypt_detached", 8usize, 32usize, "chacha20poly1305"),
        ("crypto_aead_chacha20poly1305_ietf_encrypt_detached",
         "crypto_aead_chacha20poly1305_ietf_decrypt_detached", 12, 32, "chacha20poly1305_ietf"),
        ("crypto_aead_xchacha20poly1305_ietf_encrypt_detached",
         "crypto_aead_xchacha20poly1305_ietf_decrypt_detached", 24, 32, "xchacha20poly1305_ietf"),
    ] {
        let (enc_c, _) = d.pair::<AeadEncDet>(enc);
        let (dc, dr) = d.pair::<AeadDecDet>(dec);
        let mut rng = Rng::new(0xD00D ^ tag.len() as u64);
        let key = rng.bytes(klen);
        let npub = rng.bytes(nlen);
        let ad = rng.bytes(7);
        let msg = rng.bytes(24);

        let mut ct = vec![0u8; msg.len()];
        let mut mac = vec![0u8; 16];
        let r = unsafe {
            (*enc_c)(ct.as_mut_ptr(), mac.as_mut_ptr(), std::ptr::null_mut(),
                     msg.as_ptr(), msg.len() as u64, ad.as_ptr(), ad.len() as u64,
                     std::ptr::null(), npub.as_ptr(), key.as_ptr())
        };
        assert_eq!(r, 0, "{tag} encrypt_detached failed");

        // Sanity: correct MAC decrypts identically to 0 in both.
        aead_dec_det_pair(&dc, &dr, &ct, &mac, &ad, &npub, &key, &format!("{tag} good MAC"));

        // Flip every bit of every MAC byte → both -1 and m zeroed.
        for i in 0..mac.len() {
            for bit in 0..8 {
                let mut bad = mac.clone();
                bad[i] ^= 1 << bit;
                aead_dec_det_pair(&dc, &dr, &ct, &bad, &ad, &npub, &key,
                                  &format!("{tag} det MAC-flip byte{i} bit{bit}"));
            }
        }
    }
}

/// ERRORS.md row 149: `crypto_aead_chacha20poly1305_ietf_encrypt` with
/// `mlen > ietf_MESSAGEBYTES_MAX (2^38 − 64)` calls `sodium_misuse()` → abort.
/// This is the one poly1305-family MESSAGEBYTES_MAX row that is REACHABLE on
/// 64-bit: we declare a huge `mlen` against a tiny real buffer; the C guard
/// checks the length before touching memory, so no OOB access occurs before the
/// abort. Verified identical process fate (both SIGABRT) via `same_fate`.
///
/// Rows 145, 153 (non-ietf chacha / xchacha_ietf `MESSAGEBYTES_MAX`) are
/// documented UNREACHABLE: their max is `2^64 − 16`, which exceeds `SIZE_MAX`
/// and cannot be expressed by any real `mlen`, so the misuse branch is dead on
/// this platform. We therefore do not fabricate them.
#[test]
fn chacha20poly1305_ietf_encrypt_messagebytes_max_aborts() {
    let d = duo();
    let (cf, rf) = d.pair::<AeadEnc>("crypto_aead_chacha20poly1305_ietf_encrypt");
    let cf = *cf;
    let rf = *rf;
    // 2^38 - 64 is the ietf MESSAGEBYTES_MAX; go one past it.
    let huge: u64 = (1u64 << 38) - 64 + 1;
    same_fate(
        "chacha20poly1305_ietf_encrypt mlen>MAX aborts",
        move || {
            let key = [0u8; 32];
            let npub = [0u8; 12];
            let mut c = [0u8; 32];
            let m = [0u8; 1];
            unsafe {
                cf(c.as_mut_ptr(), std::ptr::null_mut(), m.as_ptr(), huge,
                   std::ptr::null(), 0, std::ptr::null(), npub.as_ptr(), key.as_ptr());
            }
        },
        move || {
            let key = [0u8; 32];
            let npub = [0u8; 12];
            let mut c = [0u8; 32];
            let m = [0u8; 1];
            unsafe {
                rf(c.as_mut_ptr(), std::ptr::null_mut(), m.as_ptr(), huge,
                   std::ptr::null(), 0, std::ptr::null(), npub.as_ptr(), key.as_ptr());
            }
        },
    );
}

// ===========================================================================
// 7.b  aegis128l / aegis256 — rows 154–167
// ===========================================================================

/// ERRORS.md rows 154, 155, 161, 162 (aegis combined decrypt):
///  * short clen `< ABYTES (32)` → -1;
///  * tampered ct / MAC / AD / nonce / key → -1 and `m` zeroed identically.
#[test]
fn aegis_decrypt_errors() {
    let d = duo();
    for (enc, dec, nlen, tag) in [
        ("crypto_aead_aegis128l_encrypt", "crypto_aead_aegis128l_decrypt", 16usize, "aegis128l"),
        ("crypto_aead_aegis256_encrypt", "crypto_aead_aegis256_decrypt", 32usize, "aegis256"),
    ] {
        let (enc_c, _) = d.pair::<AeadEnc>(enc);
        let (dec_c, dec_r) = d.pair::<AeadDec>(dec);
        let abytes = 32usize;
        let mut rng = Rng::new(0xAE_6152 ^ tag.len() as u64);
        let key = rng.bytes(32);
        let npub = rng.bytes(nlen);
        let ad = rng.bytes(9);
        let msg = rng.bytes(48);

        for clen in 0..abytes {
            let ct = rng.bytes(clen);
            aead_dec_pair(&dec_c, &dec_r, &ct, &ad, &npub, &key, 0,
                          &format!("{tag} short clen={clen}"));
        }

        let ct = aead_seal(&enc_c, &msg, &ad, &npub, &key, abytes);
        // Flip every bit of every MAC byte.
        for i in (ct.len() - abytes)..ct.len() {
            for bit in 0..8 {
                let mut bad = ct.clone();
                bad[i] ^= 1 << bit;
                aead_dec_pair(&dec_c, &dec_r, &bad, &ad, &npub, &key, msg.len(),
                              &format!("{tag} MAC-flip byte{i} bit{bit}"));
            }
        }
        let body = ct.len() - abytes;
        for &i in &[0usize, 1, body / 2, body.saturating_sub(1)] {
            if i >= body { continue; }
            let mut bad = ct.clone();
            bad[i] ^= 0x80;
            aead_dec_pair(&dec_c, &dec_r, &bad, &ad, &npub, &key, msg.len(),
                          &format!("{tag} CT-flip byte{i}"));
        }
        let mut wad = ad.clone(); wad[0] ^= 0xff;
        aead_dec_pair(&dec_c, &dec_r, &ct, &wad, &npub, &key, msg.len(), &format!("{tag} wrong AD"));
        let mut wn = npub.clone(); wn[0] ^= 0xff;
        aead_dec_pair(&dec_c, &dec_r, &ct, &ad, &wn, &key, msg.len(), &format!("{tag} wrong nonce"));
        let mut wk = key.clone(); wk[0] ^= 0xff;
        aead_dec_pair(&dec_c, &dec_r, &ct, &ad, &npub, &wk, msg.len(), &format!("{tag} wrong key"));
    }
}

/// ERRORS.md rows 156, 163 (aegis decrypt_detached, wrong MAC → -1, m zeroed).
#[test]
fn aegis_decrypt_detached_wrong_mac() {
    let d = duo();
    for (enc, dec, nlen, tag) in [
        ("crypto_aead_aegis128l_encrypt_detached",
         "crypto_aead_aegis128l_decrypt_detached", 16usize, "aegis128l"),
        ("crypto_aead_aegis256_encrypt_detached",
         "crypto_aead_aegis256_decrypt_detached", 32usize, "aegis256"),
    ] {
        let (enc_c, _) = d.pair::<AeadEncDet>(enc);
        let (dc, dr) = d.pair::<AeadDecDet>(dec);
        let mut rng = Rng::new(0xBEE5 ^ tag.len() as u64);
        let key = rng.bytes(32);
        let npub = rng.bytes(nlen);
        let ad = rng.bytes(5);
        let msg = rng.bytes(20);

        let mut ct = vec![0u8; msg.len()];
        let mut mac = vec![0u8; 32];
        let r = unsafe {
            (*enc_c)(ct.as_mut_ptr(), mac.as_mut_ptr(), std::ptr::null_mut(),
                     msg.as_ptr(), msg.len() as u64, ad.as_ptr(), ad.len() as u64,
                     std::ptr::null(), npub.as_ptr(), key.as_ptr())
        };
        assert_eq!(r, 0, "{tag} encrypt_detached failed");
        aead_dec_det_pair(&dc, &dr, &ct, &mac, &ad, &npub, &key, &format!("{tag} good MAC"));
        for i in 0..mac.len() {
            // sample two bits per byte to keep it quick but cover every byte.
            for bit in [0usize, 7] {
                let mut bad = mac.clone();
                bad[i] ^= 1 << bit;
                aead_dec_det_pair(&dc, &dr, &ct, &bad, &ad, &npub, &key,
                                  &format!("{tag} det MAC-flip byte{i} bit{bit}"));
            }
        }
    }
}

// Rows 157, 158, 164, 165 (aegis encrypt / encrypt_detached MESSAGEBYTES_MAX):
//   UNREACHABLE. aegis MESSAGEBYTES_MAX == SIZE_MAX (2^64−1); an mlen/adlen that
//   large is impossible to allocate and cannot be exceeded by a real u64, so the
//   `sodium_misuse()` branch is dead on 64-bit.
// Rows 159, 166 (aegis decrypt_detached clen/adlen > MESSAGEBYTES_MAX): the
//   `-1` guard is likewise unreachable (same SIZE_MAX argument).
// Rows 160, 167 (`aegis*_mac` maclen neither 16 nor 32): UNREACHABLE via public
//   API — the public AEAD entry points always pass maclen == ABYTES (32); the
//   internal `_mac` helper with a variable maclen is never exposed, so the
//   `return -1` for an out-of-range maclen cannot be triggered through any
//   exported symbol. Documented, not faked.

// ===========================================================================
// 8.  crypto_secretbox — rows 168–176
// ===========================================================================

/// ERRORS.md rows 168, 169, 170 (xsalsa20poly1305 NaCl padded interface):
///  * 168: encrypt with `mlen < 32` (ZEROBYTES) → -1;
///  * 169: open with `clen < 32` → -1;
///  * 170: open with a failing poly1305 verify → -1.
#[test]
fn secretbox_xsalsa20poly1305_nacl_errors() {
    let d = duo();
    let (sc, sr) = d.pair::<SbNacl>("crypto_secretbox_xsalsa20poly1305");
    let (oc, or) = d.pair::<SbNacl>("crypto_secretbox_xsalsa20poly1305_open");
    let mut rng = Rng::new(0x5B01);
    let key = rng.bytes(32);
    let nonce = rng.bytes(24);

    // Row 168: mlen < 32 → -1 (both). C requires the first ZEROBYTES(32) of m
    // to be zero AND mlen >= 32.
    for mlen in 0..32u64 {
        let m = vec![0u8; mlen as usize];
        let mut cc = vec![0u8; mlen as usize + 32];
        let mut rc_ = vec![0u8; mlen as usize + 32];
        let rc = unsafe { (*sc)(cc.as_mut_ptr(), m.as_ptr(), mlen, nonce.as_ptr(), key.as_ptr()) };
        let rr = unsafe { (*sr)(rc_.as_mut_ptr(), m.as_ptr(), mlen, nonce.as_ptr(), key.as_ptr()) };
        eq_i32(&format!("secretbox_xsalsa seal mlen={mlen}"), rc, rr);
        assert_eq!(rc, -1, "mlen<32 must fail");
    }

    // Row 169: open clen < 32 → -1 (both).
    for clen in 0..32u64 {
        let c = vec![0u8; clen as usize];
        let mut mc = vec![0u8; clen as usize + 32];
        let mut mr = vec![0u8; clen as usize + 32];
        let rc = unsafe { (*oc)(mc.as_mut_ptr(), c.as_ptr(), clen, nonce.as_ptr(), key.as_ptr()) };
        let rr = unsafe { (*or)(mr.as_mut_ptr(), c.as_ptr(), clen, nonce.as_ptr(), key.as_ptr()) };
        eq_i32(&format!("secretbox_xsalsa open clen={clen}"), rc, rr);
        assert_eq!(rc, -1, "clen<32 must fail");
    }

    // Row 170: build a valid padded ciphertext (m has 32 leading zero bytes),
    // then corrupt the poly1305 region (BOXZEROBYTES..) so verify fails → -1.
    let plainlen = 40usize;
    let mut m = vec![0u8; 32 + plainlen];
    rng.fill(&mut m[32..]);
    let mut c = vec![0u8; m.len()];
    let r = unsafe { (*sc)(c.as_mut_ptr(), m.as_ptr(), m.len() as u64, nonce.as_ptr(), key.as_ptr()) };
    assert_eq!(r, 0, "secretbox_xsalsa seal failed");
    // Corrupt a byte in the MAC region (bytes 16..32 of the ciphertext).
    for i in 16..32 {
        let mut bad = c.clone();
        bad[i] ^= 0xff;
        let mut mc = vec![0xAAu8; m.len()];
        let mut mr = vec![0xAAu8; m.len()];
        let rc = unsafe {
            (*oc)(mc.as_mut_ptr(), bad.as_ptr(), bad.len() as u64, nonce.as_ptr(), key.as_ptr())
        };
        let rr = unsafe {
            (*or)(mr.as_mut_ptr(), bad.as_ptr(), bad.len() as u64, nonce.as_ptr(), key.as_ptr())
        };
        eq_i32(&format!("secretbox_xsalsa open bad-mac byte{i}"), rc, rr);
        assert_eq!(rc, -1, "poly1305 verify must fail");
    }
}

/// ERRORS.md rows 172, 173, 175, 176 (secretbox `_easy`/`_detached` variants):
///  * 172: `crypto_secretbox_open_easy` with `clen < MACBYTES (16)` → -1 (0..15);
///  * 173: `crypto_secretbox_open_detached` with a bad MAC → -1;
///  * 175: `crypto_secretbox_xchacha20poly1305_open_easy` with `clen < 16` → -1;
///  * 176: `crypto_secretbox_xchacha20poly1305_open_detached` with a bad MAC → -1.
///
/// Rows 171, 174 (`_easy` `mlen > MESSAGEBYTES_MAX`) are UNREACHABLE on 64-bit:
/// MESSAGEBYTES_MAX == SIZE_MAX − MACBYTES, unreachable by any real `mlen`.
#[test]
fn secretbox_easy_detached_errors() {
    let d = duo();
    let mut rng = Rng::new(0x5B0E);
    let key = rng.bytes(32);
    let nonce_x = rng.bytes(24);

    // ---- open_easy short clen (rows 172 / 175) ----
    for (name, nlen) in [
        ("crypto_secretbox_open_easy", 24usize),
        ("crypto_secretbox_xchacha20poly1305_open_easy", 24usize),
    ] {
        let (oc, or) = d.pair::<SbEasy>(name);
        let nonce = rng.bytes(nlen);
        for clen in 0..16u64 {
            let c = vec![0u8; clen as usize];
            let mut mc = vec![0xAAu8; 16];
            let mut mr = vec![0xAAu8; 16];
            let rc = unsafe { (*oc)(mc.as_mut_ptr(), c.as_ptr(), clen, nonce.as_ptr(), key.as_ptr()) };
            let rr = unsafe { (*or)(mr.as_mut_ptr(), c.as_ptr(), clen, nonce.as_ptr(), key.as_ptr()) };
            eq_i32(&format!("{name} clen={clen}"), rc, rr);
            assert_eq!(rc, -1, "{name} clen<16 must fail");
        }
    }

    // ---- open_detached bad MAC (rows 173 / 176) ----
    for (sealname, detname, nlen, tag) in [
        ("crypto_secretbox_detached", "crypto_secretbox_open_detached", 24usize, "secretbox"),
        ("crypto_secretbox_xchacha20poly1305_detached",
         "crypto_secretbox_xchacha20poly1305_open_detached", 24usize, "xsecretbox"),
    ] {
        let (seal_c, _) = d.pair::<SbDet>(sealname);
        let (oc, or) = d.pair::<SbOpenDet>(detname);
        let nonce = if tag == "secretbox" { nonce_x.clone() } else { rng.bytes(nlen) };
        let msg = rng.bytes(24);
        let mut ct = vec![0u8; msg.len()];
        let mut mac = vec![0u8; 16];
        let r = unsafe {
            (*seal_c)(ct.as_mut_ptr(), mac.as_mut_ptr(), msg.as_ptr(), msg.len() as u64,
                      nonce.as_ptr(), key.as_ptr())
        };
        assert_eq!(r, 0, "{tag} detached seal failed");
        // good MAC decrypts identically.
        {
            let mut mc = vec![0xAAu8; msg.len()];
            let mut mr = vec![0xAAu8; msg.len()];
            let rc = unsafe { (*oc)(mc.as_mut_ptr(), ct.as_ptr(), mac.as_ptr(), ct.len() as u64, nonce.as_ptr(), key.as_ptr()) };
            let rr = unsafe { (*or)(mr.as_mut_ptr(), ct.as_ptr(), mac.as_ptr(), ct.len() as u64, nonce.as_ptr(), key.as_ptr()) };
            eq_i32(&format!("{tag} good mac ret"), rc, rr);
            eq_bytes(&format!("{tag} good mac m"), &mc, &mr);
        }
        for i in 0..mac.len() {
            for bit in [0usize, 7] {
                let mut bad = mac.clone();
                bad[i] ^= 1 << bit;
                let mut mc = vec![0xAAu8; msg.len()];
                let mut mr = vec![0xAAu8; msg.len()];
                let rc = unsafe { (*oc)(mc.as_mut_ptr(), ct.as_ptr(), bad.as_ptr(), ct.len() as u64, nonce.as_ptr(), key.as_ptr()) };
                let rr = unsafe { (*or)(mr.as_mut_ptr(), ct.as_ptr(), bad.as_ptr(), ct.len() as u64, nonce.as_ptr(), key.as_ptr()) };
                eq_i32(&format!("{tag} bad mac byte{i} bit{bit} ret"), rc, rr);
                assert_eq!(rc, -1, "{tag} bad MAC must fail");
            }
        }
    }
}

// ===========================================================================
// 9.  crypto_secretstream_xchacha20poly1305 — rows 178, 180, 181, 182
// ===========================================================================

/// ERRORS.md rows 178, 180, 181, 182 (secretstream pull error paths). Because C
/// and Rust keep INDEPENDENT opaque state structs, each library is driven with
/// its own `init_push`/`init_pull`-produced state; we then assert the two
/// libraries agree on the observable pull result (ret + tag).
///  * 178: `_pull` with `inlen < ABYTES (17)` → -1 AND `*tag_p` set to 0xff by both.
///  * 180: `_pull` with MAC mismatch (tampered ct / wrong AD / wrong key) → -1.
///  * 181: `_pull` with messages consumed OUT OF ORDER (state desync) → -1.
///  * 182: `_init_pull` with a WRONG header, then `_pull` → -1.
///
/// Rows 177, 179 (`_push`/`_pull` `mlen > MESSAGEBYTES_MAX`) are UNREACHABLE on
/// 64-bit: MESSAGEBYTES_MAX == SIZE_MAX − ABYTES, unreachable by any real length.
#[test]
fn secretstream_pull_errors() {
    let d = duo();
    let (sb_c, sb_r) = d.pair::<SizeFn>("crypto_secretstream_xchacha20poly1305_statebytes");
    let statebytes = unsafe { (*sb_c)() };
    assert_eq!(statebytes, unsafe { (*sb_r)() }, "statebytes mismatch");

    let (ipush_c, _ipush_r) = d.pair::<SsInitPush>("crypto_secretstream_xchacha20poly1305_init_push");
    let (ipull_c, ipull_r) = d.pair::<SsInitPull>("crypto_secretstream_xchacha20poly1305_init_pull");
    let (push_c, _push_r) = d.pair::<SsPush>("crypto_secretstream_xchacha20poly1305_push");
    let (pull_c, pull_r) = d.pair::<SsPull>("crypto_secretstream_xchacha20poly1305_pull");
    const HEADERBYTES: usize = 24;
    const ABYTES: usize = 17;

    let mut rng = Rng::new(0x557E_A11);
    let key = rng.bytes(32);

    // Produce a sequence of ciphertext frames using C (both libs share the
    // algorithm), returning (header, frames).
    let make_frames = |msgs: &[Vec<u8>], ads: &[Vec<u8>]| -> (Vec<u8>, Vec<Vec<u8>>) {
        let mut st = vec![0u8; statebytes];
        let mut header = vec![0u8; HEADERBYTES];
        let r = unsafe { (*ipush_c)(st.as_mut_ptr(), header.as_mut_ptr(), key.as_ptr()) };
        assert_eq!(r, 0);
        let mut frames = Vec::new();
        for (m, ad) in msgs.iter().zip(ads) {
            let mut c = vec![0u8; m.len() + ABYTES];
            let mut clen: u64 = 0;
            let adp = if ad.is_empty() { std::ptr::null() } else { ad.as_ptr() };
            let r = unsafe {
                (*push_c)(st.as_mut_ptr(), c.as_mut_ptr(), &mut clen, m.as_ptr(),
                          m.len() as u64, adp, ad.len() as u64, 0)
            };
            assert_eq!(r, 0);
            c.truncate(clen as usize);
            frames.push(c);
        }
        (header, frames)
    };

    // ---- Row 178: inlen < ABYTES → -1, *tag_p = 0xff on BOTH ----
    {
        let (header, _) = make_frames(&[vec![1u8; 4]], &[vec![]]);
        for inlen in 0..ABYTES {
            let mut stc = vec![0u8; statebytes];
            let mut str_ = vec![0u8; statebytes];
            assert_eq!(unsafe { (*ipull_c)(stc.as_mut_ptr(), header.as_ptr(), key.as_ptr()) }, 0);
            assert_eq!(unsafe { (*ipull_r)(str_.as_mut_ptr(), header.as_ptr(), key.as_ptr()) }, 0);
            let inbuf = vec![0u8; inlen];
            let mut mc = vec![0u8; 8];
            let mut mr = vec![0u8; 8];
            let mut tc: u8 = 0x11;
            let mut tr: u8 = 0x11;
            let mut mlc: u64 = 0;
            let mut mlr: u64 = 0;
            let rc = unsafe {
                (*pull_c)(stc.as_mut_ptr(), mc.as_mut_ptr(), &mut mlc, &mut tc,
                          inbuf.as_ptr(), inlen as u64, std::ptr::null(), 0)
            };
            let rr = unsafe {
                (*pull_r)(str_.as_mut_ptr(), mr.as_mut_ptr(), &mut mlr, &mut tr,
                          inbuf.as_ptr(), inlen as u64, std::ptr::null(), 0)
            };
            eq_i32(&format!("ss pull short inlen={inlen}"), rc, rr);
            assert_eq!(rc, -1, "short inlen must fail");
            assert_eq!(tc, 0xff, "C must set tag=0xff on short inlen");
            assert_eq!(tr, 0xff, "Rust must set tag=0xff on short inlen");
        }
    }

    // ---- Row 180: MAC mismatch (tampered ct / wrong AD / wrong key) → -1 ----
    {
        let ad = vec![7u8, 8, 9];
        let (header, frames) = make_frames(&[vec![9u8; 20]], &[ad.clone()]);
        let frame = &frames[0];

        let run_pull = |stc: &mut [u8], str_: &mut [u8], input: &[u8], adp: &[u8], what: &str| {
            let mut mc = vec![0xAAu8; input.len()];
            let mut mr = vec![0xAAu8; input.len()];
            let mut tc: u8 = 0;
            let mut tr: u8 = 0;
            let mut mlc: u64 = 0;
            let mut mlr: u64 = 0;
            let ap = if adp.is_empty() { std::ptr::null() } else { adp.as_ptr() };
            let rc = unsafe {
                (*pull_c)(stc.as_mut_ptr(), mc.as_mut_ptr(), &mut mlc, &mut tc,
                          input.as_ptr(), input.len() as u64, ap, adp.len() as u64)
            };
            let rr = unsafe {
                (*pull_r)(str_.as_mut_ptr(), mr.as_mut_ptr(), &mut mlr, &mut tr,
                          input.as_ptr(), input.len() as u64, ap, adp.len() as u64)
            };
            eq_i32(&format!("{what} ret"), rc, rr);
            assert_eq!(rc, -1, "{what} must fail");
        };

        // tampered ciphertext: flip a body byte, midpoint, and a MAC byte.
        for &i in &[0usize, frame.len() / 2, frame.len() - 1] {
            let mut bad = frame.clone();
            bad[i] ^= 0xff;
            let mut stc = vec![0u8; statebytes];
            let mut str_ = vec![0u8; statebytes];
            unsafe { (*ipull_c)(stc.as_mut_ptr(), header.as_ptr(), key.as_ptr()); }
            unsafe { (*ipull_r)(str_.as_mut_ptr(), header.as_ptr(), key.as_ptr()); }
            run_pull(&mut stc, &mut str_, &bad, &ad, &format!("ss pull tampered ct byte{i}"));
        }
        // wrong AD.
        {
            let mut stc = vec![0u8; statebytes];
            let mut str_ = vec![0u8; statebytes];
            unsafe { (*ipull_c)(stc.as_mut_ptr(), header.as_ptr(), key.as_ptr()); }
            unsafe { (*ipull_r)(str_.as_mut_ptr(), header.as_ptr(), key.as_ptr()); }
            let mut wad = ad.clone(); wad[0] ^= 0xff;
            run_pull(&mut stc, &mut str_, frame, &wad, "ss pull wrong AD");
        }
        // wrong key: init_pull with a different key, then pull the good frame.
        {
            let mut wkey = key.clone(); wkey[0] ^= 0xff;
            let mut stc = vec![0u8; statebytes];
            let mut str_ = vec![0u8; statebytes];
            unsafe { (*ipull_c)(stc.as_mut_ptr(), header.as_ptr(), wkey.as_ptr()); }
            unsafe { (*ipull_r)(str_.as_mut_ptr(), header.as_ptr(), wkey.as_ptr()); }
            run_pull(&mut stc, &mut str_, frame, &ad, "ss pull wrong key");
        }
    }

    // ---- Row 181: messages consumed OUT OF ORDER (state desync) → -1 ----
    {
        let msgs = vec![vec![1u8; 10], vec![2u8; 10], vec![3u8; 10]];
        let ads = vec![vec![], vec![], vec![]];
        let (header, frames) = make_frames(&msgs, &ads);
        // init_pull both, then pull frame[1] FIRST (skipping frame[0]) → the
        // stream nonce/counter is desynced so the poly1305 tag fails → -1.
        let mut stc = vec![0u8; statebytes];
        let mut str_ = vec![0u8; statebytes];
        unsafe { (*ipull_c)(stc.as_mut_ptr(), header.as_ptr(), key.as_ptr()); }
        unsafe { (*ipull_r)(str_.as_mut_ptr(), header.as_ptr(), key.as_ptr()); }
        let f = &frames[1];
        let mut mc = vec![0xAAu8; f.len()];
        let mut mr = vec![0xAAu8; f.len()];
        let mut tc: u8 = 0; let mut tr: u8 = 0;
        let mut mlc: u64 = 0; let mut mlr: u64 = 0;
        let rc = unsafe {
            (*pull_c)(stc.as_mut_ptr(), mc.as_mut_ptr(), &mut mlc, &mut tc, f.as_ptr(), f.len() as u64, std::ptr::null(), 0)
        };
        let rr = unsafe {
            (*pull_r)(str_.as_mut_ptr(), mr.as_mut_ptr(), &mut mlr, &mut tr, f.as_ptr(), f.len() as u64, std::ptr::null(), 0)
        };
        eq_i32("ss pull out-of-order ret", rc, rr);
        assert_eq!(rc, -1, "out-of-order pull must fail");
    }

    // ---- Row 182: wrong header → later pull returns -1 ----
    {
        let (good_header, frames) = make_frames(&[vec![5u8; 16]], &[vec![]]);
        let mut bad_header = good_header.clone();
        bad_header[0] ^= 0xff;
        let mut stc = vec![0u8; statebytes];
        let mut str_ = vec![0u8; statebytes];
        assert_eq!(unsafe { (*ipull_c)(stc.as_mut_ptr(), bad_header.as_ptr(), key.as_ptr()) }, 0);
        assert_eq!(unsafe { (*ipull_r)(str_.as_mut_ptr(), bad_header.as_ptr(), key.as_ptr()) }, 0);
        let f = &frames[0];
        let mut mc = vec![0xAAu8; f.len()];
        let mut mr = vec![0xAAu8; f.len()];
        let mut tc: u8 = 0; let mut tr: u8 = 0;
        let mut mlc: u64 = 0; let mut mlr: u64 = 0;
        let rc = unsafe {
            (*pull_c)(stc.as_mut_ptr(), mc.as_mut_ptr(), &mut mlc, &mut tc, f.as_ptr(), f.len() as u64, std::ptr::null(), 0)
        };
        let rr = unsafe {
            (*pull_r)(str_.as_mut_ptr(), mr.as_mut_ptr(), &mut mlr, &mut tr, f.as_ptr(), f.len() as u64, std::ptr::null(), 0)
        };
        eq_i32("ss pull wrong-header ret", rc, rr);
        assert_eq!(rc, -1, "wrong header must make pull fail");
    }
}

// ===========================================================================
// 10.  crypto_box — rows 183–206
// ===========================================================================

/// Fetch a fresh (pk, sk) keypair from the C library.
fn box_keypair(d: &'static Duo) -> ([u8; 32], [u8; 32]) {
    let (kp, _) = d.pair::<BoxKeypair>("crypto_box_keypair");
    let mut pk = [0u8; 32];
    let mut sk = [0u8; 32];
    let r = unsafe { (*kp)(pk.as_mut_ptr(), sk.as_mut_ptr()) };
    assert_eq!(r, 0);
    (pk, sk)
}

/// ERRORS.md rows 183, 198 (`beforenm` with a small-order pk → -1). Every one of
/// the 7 blocklisted small-order curve25519 points is tried against BOTH the
/// xsalsa and xchacha `beforenm` (and the top-level `crypto_box_beforenm`).
#[test]
fn box_beforenm_small_order() {
    let d = duo();
    let (_pk, sk) = box_keypair(d);
    for name in [
        "crypto_box_curve25519xsalsa20poly1305_beforenm",
        "crypto_box_curve25519xchacha20poly1305_beforenm",
        "crypto_box_beforenm",
    ] {
        let (cf, rf) = d.pair::<BoxBeforenm>(name);
        for (idx, pk) in SMALL_ORDER.iter().enumerate() {
            let mut kc = [0u8; 32];
            let mut kr = [0u8; 32];
            let rc = unsafe { (*cf)(kc.as_mut_ptr(), pk.as_ptr(), sk.as_ptr()) };
            let rr = unsafe { (*rf)(kr.as_mut_ptr(), pk.as_ptr(), sk.as_ptr()) };
            eq_i32(&format!("{name} small-order[{idx}]"), rc, rr);
            assert_eq!(rc, -1, "{name} small-order pk must fail");
        }
    }
}

/// ERRORS.md rows 184, 185 (`crypto_box_curve25519xsalsa20poly1305` /
/// `_open` with a small-order pk → -1 because `beforenm` fails).
#[test]
fn box_xsalsa_nacl_small_order() {
    let d = duo();
    let (_pk, sk) = box_keypair(d);
    let (sc, sr) = d.pair::<BoxNacl>("crypto_box_curve25519xsalsa20poly1305");
    let (oc, or) = d.pair::<BoxNacl>("crypto_box_curve25519xsalsa20poly1305_open");
    let nonce = [0u8; 24];
    let m = [0u8; 64]; // ZEROBYTES-padded
    for (idx, pk) in SMALL_ORDER.iter().enumerate() {
        let mut cc = [0u8; 64];
        let mut cr = [0u8; 64];
        let rc = unsafe { (*sc)(cc.as_mut_ptr(), m.as_ptr(), m.len() as u64, nonce.as_ptr(), pk.as_ptr(), sk.as_ptr()) };
        let rr = unsafe { (*sr)(cr.as_mut_ptr(), m.as_ptr(), m.len() as u64, nonce.as_ptr(), pk.as_ptr(), sk.as_ptr()) };
        eq_i32(&format!("box_xsalsa seal small-order[{idx}]"), rc, rr);
        assert_eq!(rc, -1);
        let mut mc = [0u8; 64];
        let mut mr = [0u8; 64];
        let rc = unsafe { (*oc)(mc.as_mut_ptr(), m.as_ptr(), m.len() as u64, nonce.as_ptr(), pk.as_ptr(), sk.as_ptr()) };
        let rr = unsafe { (*or)(mr.as_mut_ptr(), m.as_ptr(), m.len() as u64, nonce.as_ptr(), pk.as_ptr(), sk.as_ptr()) };
        eq_i32(&format!("box_xsalsa open small-order[{idx}]"), rc, rr);
        assert_eq!(rc, -1);
    }
}

/// ERRORS.md row 186 (`crypto_box_curve25519xsalsa20poly1305_open` MAC verify
/// fails → -1). A valid padded box is built with the C seal, then corrupted.
#[test]
fn box_xsalsa_nacl_bad_mac() {
    let d = duo();
    let (pk_a, sk_a) = box_keypair(d);
    let (pk_b, sk_b) = box_keypair(d);
    let (sc, _) = d.pair::<BoxNacl>("crypto_box_curve25519xsalsa20poly1305");
    let (oc, or) = d.pair::<BoxNacl>("crypto_box_curve25519xsalsa20poly1305_open");
    let nonce = [0x42u8; 24];
    let mut rng = Rng::new(0xB0C5);
    // NaCl interface: first ZEROBYTES(32) of m must be zero.
    let mut m = vec![0u8; 32 + 30];
    rng.fill(&mut m[32..]);
    let mut c = vec![0u8; m.len()];
    let r = unsafe { (*sc)(c.as_mut_ptr(), m.as_ptr(), m.len() as u64, nonce.as_ptr(), pk_b.as_ptr(), sk_a.as_ptr()) };
    assert_eq!(r, 0, "box seal failed");
    // Corrupt bytes in the MAC region (BOXZEROBYTES(16)..32 of the ciphertext).
    for i in 16..32 {
        let mut bad = c.clone();
        bad[i] ^= 0xff;
        let mut mc = vec![0xAAu8; m.len()];
        let mut mr = vec![0xAAu8; m.len()];
        let rc = unsafe { (*oc)(mc.as_mut_ptr(), bad.as_ptr(), bad.len() as u64, nonce.as_ptr(), pk_a.as_ptr(), sk_b.as_ptr()) };
        let rr = unsafe { (*or)(mr.as_mut_ptr(), bad.as_ptr(), bad.len() as u64, nonce.as_ptr(), pk_a.as_ptr(), sk_b.as_ptr()) };
        eq_i32(&format!("box_xsalsa open bad-mac byte{i}"), rc, rr);
        assert_eq!(rc, -1, "bad MAC must fail");
    }
}

/// ERRORS.md rows 187, 190, 191, 199, 202 (`crypto_box_detached` /
/// `_open_detached` and the xchacha equivalents):
///  * 187/199: `detached` with a small-order pk → -1 (`beforenm` fails).
///  * 190/202: `open_detached` with a small-order pk → -1.
///  * 191/202: `open_detached` with a failing MAC → -1.
#[test]
fn box_detached_errors() {
    let d = duo();
    let (pk, sk) = box_keypair(d);
    let (pk2, _sk2) = box_keypair(d);
    let mut rng = Rng::new(0xDE7A);

    for (detname, opendetname, nlen, tag) in [
        ("crypto_box_detached", "crypto_box_open_detached", 24usize, "box"),
        ("crypto_box_curve25519xchacha20poly1305_detached",
         "crypto_box_curve25519xchacha20poly1305_open_detached", 24usize, "xbox"),
    ] {
        let (dc, dr) = d.pair::<BoxDet>(detname);
        let (oc, or) = d.pair::<BoxOpenDet>(opendetname);
        let nonce = rng.bytes(nlen);
        let msg = rng.bytes(20);

        // small-order pk on detached seal → -1 (both).
        for (idx, spk) in SMALL_ORDER.iter().enumerate() {
            let mut cc = vec![0u8; msg.len()];
            let mut cr = vec![0u8; msg.len()];
            let mut mac_c = [0u8; 16];
            let mut mac_r = [0u8; 16];
            let rc = unsafe { (*dc)(cc.as_mut_ptr(), mac_c.as_mut_ptr(), msg.as_ptr(), msg.len() as u64, nonce.as_ptr(), spk.as_ptr(), sk.as_ptr()) };
            let rr = unsafe { (*dr)(cr.as_mut_ptr(), mac_r.as_mut_ptr(), msg.as_ptr(), msg.len() as u64, nonce.as_ptr(), spk.as_ptr(), sk.as_ptr()) };
            eq_i32(&format!("{tag} detached small-order[{idx}]"), rc, rr);
            assert_eq!(rc, -1);
            // open_detached small-order pk → -1.
            let mut mc = vec![0u8; msg.len()];
            let mut mr = vec![0u8; msg.len()];
            let rc = unsafe { (*oc)(mc.as_mut_ptr(), cc.as_ptr(), mac_c.as_ptr(), msg.len() as u64, nonce.as_ptr(), spk.as_ptr(), sk.as_ptr()) };
            let rr = unsafe { (*or)(mr.as_mut_ptr(), cr.as_ptr(), mac_r.as_ptr(), msg.len() as u64, nonce.as_ptr(), spk.as_ptr(), sk.as_ptr()) };
            eq_i32(&format!("{tag} open_detached small-order[{idx}]"), rc, rr);
            assert_eq!(rc, -1);
        }

        // Build a valid detached box (recipient pk2 <- sender sk), then corrupt
        // the MAC → open_detached returns -1.
        let mut ct = vec![0u8; msg.len()];
        let mut mac = [0u8; 16];
        let r = unsafe { (*dc)(ct.as_mut_ptr(), mac.as_mut_ptr(), msg.as_ptr(), msg.len() as u64, nonce.as_ptr(), pk2.as_ptr(), sk.as_ptr()) };
        assert_eq!(r, 0, "{tag} detached seal failed");
        for i in 0..mac.len() {
            for bit in [0usize, 7] {
                let mut bad = mac;
                bad[i] ^= 1 << bit;
                let mut mc = vec![0xAAu8; msg.len()];
                let mut mr = vec![0xAAu8; msg.len()];
                // recipient opens with its sk against sender pk (use pk2 as
                // sender's public key mirror; MAC is broken regardless).
                let rc = unsafe { (*oc)(mc.as_mut_ptr(), ct.as_ptr(), bad.as_ptr(), msg.len() as u64, nonce.as_ptr(), pk.as_ptr(), sk.as_ptr()) };
                let rr = unsafe { (*or)(mr.as_mut_ptr(), ct.as_ptr(), bad.as_ptr(), msg.len() as u64, nonce.as_ptr(), pk.as_ptr(), sk.as_ptr()) };
                eq_i32(&format!("{tag} open_detached bad-mac byte{i} bit{bit}"), rc, rr);
                assert_eq!(rc, -1, "{tag} bad MAC must fail");
            }
        }
    }
}

/// ERRORS.md rows 192, 193, 203, 204 (`open_easy` / `open_easy_afternm` with
/// `clen < MACBYTES (16)` → -1) for both box and xchacha box variants.
#[test]
fn box_open_easy_short_clen() {
    let d = duo();
    let (pk, sk) = box_keypair(d);
    let mut rng = Rng::new(0xEA5E);

    for (name, nlen) in [
        ("crypto_box_open_easy", 24usize),
        ("crypto_box_curve25519xchacha20poly1305_open_easy", 24usize),
    ] {
        let (oc, or) = d.pair::<BoxOpenEasy>(name);
        let nonce = rng.bytes(nlen);
        for clen in 0..16u64 {
            let c = vec![0u8; clen as usize];
            let mut mc = vec![0xAAu8; 16];
            let mut mr = vec![0xAAu8; 16];
            let rc = unsafe { (*oc)(mc.as_mut_ptr(), c.as_ptr(), clen, nonce.as_ptr(), pk.as_ptr(), sk.as_ptr()) };
            let rr = unsafe { (*or)(mr.as_mut_ptr(), c.as_ptr(), clen, nonce.as_ptr(), pk.as_ptr(), sk.as_ptr()) };
            eq_i32(&format!("{name} clen={clen}"), rc, rr);
            assert_eq!(rc, -1, "{name} clen<16 must fail");
        }
    }

    // afternm variants take a precomputed shared key `k` instead of pk/sk.
    let (bn, _) = d.pair::<BoxBeforenm>("crypto_box_beforenm");
    let mut k = [0u8; 32];
    assert_eq!(unsafe { (*bn)(k.as_mut_ptr(), pk.as_ptr(), sk.as_ptr()) }, 0);
    for name in [
        "crypto_box_open_easy_afternm",
        "crypto_box_curve25519xchacha20poly1305_open_easy_afternm",
    ] {
        let (oc, or) = d.pair::<BoxOpenEasyAfternm>(name);
        let nonce = rng.bytes(24);
        for clen in 0..16u64 {
            let c = vec![0u8; clen as usize];
            let mut mc = vec![0xAAu8; 16];
            let mut mr = vec![0xAAu8; 16];
            let rc = unsafe { (*oc)(mc.as_mut_ptr(), c.as_ptr(), clen, nonce.as_ptr(), k.as_ptr()) };
            let rr = unsafe { (*or)(mr.as_mut_ptr(), c.as_ptr(), clen, nonce.as_ptr(), k.as_ptr()) };
            eq_i32(&format!("{name} clen={clen}"), rc, rr);
            assert_eq!(rc, -1, "{name} clen<16 must fail");
        }
    }
}

/// ERRORS.md rows 196, 197, 206 (`seal_open` short clen / tampered box → -1) for
/// both box and xchacha box variants.
///  * 196/206: `seal_open` with `clen < SEALBYTES (48)` → -1 (0..47).
///  * 197: `seal_open` with a tampered sealed box → -1.
#[test]
fn box_seal_open_errors() {
    let d = duo();
    let (pk, sk) = box_keypair(d);
    let mut rng = Rng::new(0x5EA1);

    for (sealname, openname, sealbytes, tag) in [
        ("crypto_box_seal", "crypto_box_seal_open", 48usize, "box_seal"),
        ("crypto_box_curve25519xchacha20poly1305_seal",
         "crypto_box_curve25519xchacha20poly1305_seal_open", 48usize, "xbox_seal"),
    ] {
        let (seal_c, _) = d.pair::<BoxSeal>(sealname);
        let (oc, or) = d.pair::<BoxSealOpen>(openname);

        // Short clen 0..SEALBYTES-1 → -1 (both).
        for clen in 0..sealbytes as u64 {
            let c = vec![0u8; clen as usize];
            let mut mc = vec![0xAAu8; 16];
            let mut mr = vec![0xAAu8; 16];
            let rc = unsafe { (*oc)(mc.as_mut_ptr(), c.as_ptr(), clen, pk.as_ptr(), sk.as_ptr()) };
            let rr = unsafe { (*or)(mr.as_mut_ptr(), c.as_ptr(), clen, pk.as_ptr(), sk.as_ptr()) };
            eq_i32(&format!("{tag} short clen={clen}"), rc, rr);
            assert_eq!(rc, -1, "{tag} clen<SEALBYTES must fail");
        }

        // Valid sealed box, then tamper.
        let msg = rng.bytes(24);
        let mut sealed = vec![0u8; msg.len() + sealbytes];
        let r = unsafe { (*seal_c)(sealed.as_mut_ptr(), msg.as_ptr(), msg.len() as u64, pk.as_ptr()) };
        assert_eq!(r, 0, "{tag} seal failed");
        // Tamper several positions (ephemeral pk region, ciphertext, MAC region).
        for &i in &[0usize, 16, sealbytes, sealed.len() - 1] {
            let mut bad = sealed.clone();
            bad[i] ^= 0xff;
            let mut mc = vec![0xAAu8; msg.len()];
            let mut mr = vec![0xAAu8; msg.len()];
            let rc = unsafe { (*oc)(mc.as_mut_ptr(), bad.as_ptr(), bad.len() as u64, pk.as_ptr(), sk.as_ptr()) };
            let rr = unsafe { (*or)(mr.as_mut_ptr(), bad.as_ptr(), bad.len() as u64, pk.as_ptr(), sk.as_ptr()) };
            eq_i32(&format!("{tag} tampered byte{i} ret"), rc, rr);
            assert_eq!(rc, -1, "{tag} tampered sealed box must fail");
        }
    }
}

// Rows 188, 189, 194, 195, 200, 201, 205 (box `_easy`/`_seal` MESSAGEBYTES_MAX
// misuse, and the `crypto_box_seal` ephemeral-keypair-fails path):
//   UNREACHABLE on 64-bit. `crypto_box_MESSAGEBYTES_MAX` == SIZE_MAX − MACBYTES,
//   so no real `mlen` can exceed it (rows 188/189/194/200/201/205 `misuse`).
//   The seal ephemeral `crypto_box_keypair` cannot fail (rows 195/205
//   `unreachable`): keypair generation has no failure path on this platform.
//   Documented here rather than fabricated.
