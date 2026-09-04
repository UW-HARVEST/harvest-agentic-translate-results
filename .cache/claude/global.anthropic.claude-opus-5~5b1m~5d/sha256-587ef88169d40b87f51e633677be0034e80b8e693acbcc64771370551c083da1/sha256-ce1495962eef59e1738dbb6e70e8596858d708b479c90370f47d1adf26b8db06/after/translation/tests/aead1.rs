//! Differential tests for AREA `aead1`:
//!
//!   * `crypto_aead/chacha20poly1305/aead_chacha20poly1305.c`
//!   * `crypto_aead/xchacha20poly1305/aead_xchacha20poly1305.c`
//!   * `crypto_secretbox/crypto_secretbox.c`
//!   * `crypto_secretbox/crypto_secretbox_easy.c`
//!   * `crypto_secretbox/xsalsa20poly1305/secretbox_xsalsa20poly1305.c`
//!   * `crypto_secretbox/xchacha20poly1305/secretbox_xchacha20poly1305.c`
//!   * `crypto_secretstream/xchacha20poly1305/secretstream_xchacha20poly1305.c`
//!
//! Everything is called through `dlopen`/`dlsym` on both the C and the Rust
//! shared object, never directly.

#[macro_use]
mod common;

use core::ffi::{c_char, c_int, c_void};
use core::ptr::{null, null_mut};
use std::sync::atomic::{AtomicU64, Ordering};

// ---------------------------------------------------------------- helpers ----

const CAN: u8 = 0x5A;
const TAIL: usize = 16;

/// Output buffer of `n` usable bytes plus `TAIL` canary bytes.
fn cbuf(n: usize) -> Vec<u8> {
    vec![CAN; n + TAIL]
}

/// Compare the FULL buffers (usable region + canary tail) and check neither
/// implementation wrote past `n`.
fn chk(ctx: &str, c: &[u8], r: &[u8], n: usize) {
    common::eqb(ctx, c, r);
    assert!(
        c[n..].iter().all(|&b| b == CAN),
        "{}: C wrote past the end of the output buffer",
        ctx
    );
    assert!(
        r[n..].iter().all(|&b| b == CAN),
        "{}: Rust wrote past the end of the output buffer",
        ctx
    );
}

fn u64p(v: &mut u64, some: bool) -> *mut u64 {
    if some {
        v as *mut u64
    } else {
        null_mut()
    }
}

// ------------------------------------------------- deterministic randombytes --
//
// `crypto_*_keygen()` and `crypto_secretstream_..._init_push()` call
// `randombytes_buf()`. To make them byte-comparable we install our own
// deterministic implementation into BOTH libraries via
// `randombytes_set_implementation()` (which, in this build, unconditionally
// replaces the implementation pointer).

#[repr(C)]
struct RbImpl {
    implementation_name: Option<unsafe extern "C" fn() -> *const c_char>,
    random: Option<unsafe extern "C" fn() -> u32>,
    stir: Option<unsafe extern "C" fn()>,
    uniform: Option<unsafe extern "C" fn(u32) -> u32>,
    buf: Option<unsafe extern "C" fn(*mut c_void, usize)>,
    close: Option<unsafe extern "C" fn() -> c_int>,
}
unsafe impl Sync for RbImpl {}

static SEQ: AtomicU64 = AtomicU64::new(0);

fn seq_reset(v: u64) {
    SEQ.store(v, Ordering::SeqCst);
}

fn seq_next() -> u64 {
    let s = SEQ
        .fetch_add(0x9E37_79B9_7F4A_7C15, Ordering::SeqCst)
        .wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = s;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

unsafe extern "C" fn det_name() -> *const c_char {
    b"aead1det\0".as_ptr() as *const c_char
}
unsafe extern "C" fn det_random() -> u32 {
    (seq_next() >> 32) as u32
}
unsafe extern "C" fn det_stir() {}
unsafe extern "C" fn det_buf(buf: *mut c_void, size: usize) {
    let p = buf as *mut u8;
    for i in 0..size {
        *p.add(i) = (seq_next() >> 33) as u8;
    }
}
unsafe extern "C" fn det_close() -> c_int {
    0
}

static DET_IMPL: RbImpl = RbImpl {
    implementation_name: Some(det_name),
    random: Some(det_random),
    stir: Some(det_stir),
    uniform: None,
    buf: Some(det_buf),
    close: Some(det_close),
};

fn install_det_randombytes() {
    let (c, r) = both!(
        "randombytes_set_implementation",
        unsafe extern "C" fn(*const RbImpl) -> c_int
    );
    unsafe {
        assert_eq!(c(&DET_IMPL as *const RbImpl), 0);
        assert_eq!(r(&DET_IMPL as *const RbImpl), 0);
    }
}

// ================================================================ constants ==

macro_rules! eq_size {
    ($name:literal) => {{
        let (c, r) = both!($name, unsafe extern "C" fn() -> usize);
        let (cv, rv) = unsafe { (c(), r()) };
        assert_eq!(cv, rv, "{}: value mismatch", $name);
        cv
    }};
}

macro_rules! eq_u8 {
    ($name:literal) => {{
        let (c, r) = both!($name, unsafe extern "C" fn() -> u8);
        let (cv, rv) = unsafe { (c(), r()) };
        assert_eq!(cv, rv, "{}: value mismatch", $name);
        cv
    }};
}

#[test]
fn constants() {
    // --- aead chacha20poly1305 (original, 8-byte nonce)
    assert_eq!(eq_size!("crypto_aead_chacha20poly1305_keybytes"), 32);
    assert_eq!(eq_size!("crypto_aead_chacha20poly1305_npubbytes"), 8);
    assert_eq!(eq_size!("crypto_aead_chacha20poly1305_nsecbytes"), 0);
    assert_eq!(eq_size!("crypto_aead_chacha20poly1305_abytes"), 16);
    assert_eq!(
        eq_size!("crypto_aead_chacha20poly1305_messagebytes_max"),
        usize::MAX - 16
    );

    // --- aead chacha20poly1305_ietf (12-byte nonce)
    assert_eq!(eq_size!("crypto_aead_chacha20poly1305_ietf_keybytes"), 32);
    assert_eq!(eq_size!("crypto_aead_chacha20poly1305_ietf_npubbytes"), 12);
    assert_eq!(eq_size!("crypto_aead_chacha20poly1305_ietf_nsecbytes"), 0);
    assert_eq!(eq_size!("crypto_aead_chacha20poly1305_ietf_abytes"), 16);
    assert_eq!(
        eq_size!("crypto_aead_chacha20poly1305_ietf_messagebytes_max"),
        64usize * ((1usize << 32) - 1)
    );

    // --- aead xchacha20poly1305_ietf
    assert_eq!(eq_size!("crypto_aead_xchacha20poly1305_ietf_keybytes"), 32);
    assert_eq!(eq_size!("crypto_aead_xchacha20poly1305_ietf_npubbytes"), 24);
    assert_eq!(eq_size!("crypto_aead_xchacha20poly1305_ietf_nsecbytes"), 0);
    assert_eq!(eq_size!("crypto_aead_xchacha20poly1305_ietf_abytes"), 16);
    assert_eq!(
        eq_size!("crypto_aead_xchacha20poly1305_ietf_messagebytes_max"),
        usize::MAX - 16
    );

    // --- secretbox (generic + xsalsa20poly1305 + xchacha20poly1305)
    assert_eq!(eq_size!("crypto_secretbox_keybytes"), 32);
    assert_eq!(eq_size!("crypto_secretbox_noncebytes"), 24);
    assert_eq!(eq_size!("crypto_secretbox_zerobytes"), 32);
    assert_eq!(eq_size!("crypto_secretbox_boxzerobytes"), 16);
    assert_eq!(eq_size!("crypto_secretbox_macbytes"), 16);
    assert_eq!(eq_size!("crypto_secretbox_messagebytes_max"), usize::MAX - 16);

    assert_eq!(eq_size!("crypto_secretbox_xsalsa20poly1305_keybytes"), 32);
    assert_eq!(eq_size!("crypto_secretbox_xsalsa20poly1305_noncebytes"), 24);
    assert_eq!(eq_size!("crypto_secretbox_xsalsa20poly1305_zerobytes"), 32);
    assert_eq!(eq_size!("crypto_secretbox_xsalsa20poly1305_boxzerobytes"), 16);
    assert_eq!(eq_size!("crypto_secretbox_xsalsa20poly1305_macbytes"), 16);
    assert_eq!(
        eq_size!("crypto_secretbox_xsalsa20poly1305_messagebytes_max"),
        usize::MAX - 16
    );

    assert_eq!(eq_size!("crypto_secretbox_xchacha20poly1305_keybytes"), 32);
    assert_eq!(eq_size!("crypto_secretbox_xchacha20poly1305_noncebytes"), 24);
    assert_eq!(eq_size!("crypto_secretbox_xchacha20poly1305_macbytes"), 16);
    assert_eq!(
        eq_size!("crypto_secretbox_xchacha20poly1305_messagebytes_max"),
        usize::MAX - 16
    );

    // --- secretstream
    assert_eq!(eq_size!("crypto_secretstream_xchacha20poly1305_statebytes"), 52);
    assert_eq!(eq_size!("crypto_secretstream_xchacha20poly1305_abytes"), 17);
    assert_eq!(eq_size!("crypto_secretstream_xchacha20poly1305_headerbytes"), 24);
    assert_eq!(eq_size!("crypto_secretstream_xchacha20poly1305_keybytes"), 32);
    assert_eq!(
        eq_size!("crypto_secretstream_xchacha20poly1305_messagebytes_max"),
        64usize * ((1usize << 32) - 2)
    );
    assert_eq!(eq_u8!("crypto_secretstream_xchacha20poly1305_tag_message"), 0x00);
    assert_eq!(eq_u8!("crypto_secretstream_xchacha20poly1305_tag_push"), 0x01);
    assert_eq!(eq_u8!("crypto_secretstream_xchacha20poly1305_tag_rekey"), 0x02);
    assert_eq!(eq_u8!("crypto_secretstream_xchacha20poly1305_tag_final"), 0x03);

    // --- crypto_secretbox_primitive() (const char *)
    let (c, r) = both!(
        "crypto_secretbox_primitive",
        unsafe extern "C" fn() -> *const c_char
    );
    let (cs, rs) = unsafe {
        (
            std::ffi::CStr::from_ptr(c()).to_owned(),
            std::ffi::CStr::from_ptr(r()).to_owned(),
        )
    };
    assert_eq!(cs, rs, "crypto_secretbox_primitive mismatch");
    assert_eq!(cs.to_str().unwrap(), "xsalsa20poly1305");
}

// ===================================================================== AEAD ==

type EncDet = unsafe extern "C" fn(
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

type Enc = unsafe extern "C" fn(
    *mut u8, *mut u64, *const u8, u64, *const u8, u64, *const u8, *const u8, *const u8,
) -> c_int;

type DecDet = unsafe extern "C" fn(
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

type Dec = unsafe extern "C" fn(
    *mut u8, *mut u64, *mut u8, *const u8, u64, *const u8, u64, *const u8, *const u8,
) -> c_int;

struct Aead {
    name: &'static str,
    npub: usize,
    ced: EncDet,
    red: EncDet,
    ce: Enc,
    re: Enc,
    cdd: DecDet,
    rdd: DecDet,
    cd: Dec,
    rd: Dec,
}

macro_rules! mk_aead {
    ($name:literal, $npub:expr, $ed:literal, $e:literal, $dd:literal, $d:literal) => {{
        let (ced, red) = both!($ed, EncDet);
        let (ce, re) = both!($e, Enc);
        let (cdd, rdd) = both!($dd, DecDet);
        let (cd, rd) = both!($d, Dec);
        Aead {
            name: $name,
            npub: $npub,
            ced,
            red,
            ce,
            re,
            cdd,
            rdd,
            cd,
            rd,
        }
    }};
}

fn aead_c20p1305() -> Aead {
    mk_aead!(
        "chacha20poly1305",
        8,
        "crypto_aead_chacha20poly1305_encrypt_detached",
        "crypto_aead_chacha20poly1305_encrypt",
        "crypto_aead_chacha20poly1305_decrypt_detached",
        "crypto_aead_chacha20poly1305_decrypt"
    )
}

fn aead_c20p1305_ietf() -> Aead {
    mk_aead!(
        "chacha20poly1305_ietf",
        12,
        "crypto_aead_chacha20poly1305_ietf_encrypt_detached",
        "crypto_aead_chacha20poly1305_ietf_encrypt",
        "crypto_aead_chacha20poly1305_ietf_decrypt_detached",
        "crypto_aead_chacha20poly1305_ietf_decrypt"
    )
}

fn aead_xc20p1305_ietf() -> Aead {
    mk_aead!(
        "xchacha20poly1305_ietf",
        24,
        "crypto_aead_xchacha20poly1305_ietf_encrypt_detached",
        "crypto_aead_xchacha20poly1305_ietf_encrypt",
        "crypto_aead_xchacha20poly1305_ietf_decrypt_detached",
        "crypto_aead_xchacha20poly1305_ietf_decrypt"
    )
}

/// The full encrypt/decrypt matrix: message sizes x ad shapes x optional
/// output pointers x nsec NULL / non-NULL.
fn aead_matrix(a: &Aead, seed: u64) {
    let mut rng = common::Rng::new(seed);
    for &mlen in [0usize, 1, 15, 16, 17, 63, 64, 65, 1000].iter() {
        for adcase in 0..5usize {
            for trial in 0..3usize {
                let k = rng.bytes(32);
                let npub = rng.bytes(a.npub);
                let m = rng.bytes(mlen);
                let (adlen, adv, ad_null) = match adcase {
                    0 => (0usize, Vec::new(), true), // ad == NULL, adlen == 0
                    1 => (0usize, rng.bytes(4), false), // ad != NULL, adlen == 0
                    2 => (1usize, rng.bytes(1), false),
                    3 => (16usize, rng.bytes(16), false),
                    _ => {
                        let n = 1 + rng.below(100);
                        (n, rng.bytes(n), false)
                    }
                };
                let adp: *const u8 = if ad_null { null() } else { adv.as_ptr() };
                let base = format!(
                    "{} mlen={} adcase={} adlen={} trial={}",
                    a.name, mlen, adcase, adlen, trial
                );

                // ---------------- encrypt_detached ----------------
                let mut ref_ct = vec![0u8; mlen];
                let mut ref_mac = vec![0u8; 16];
                for &maclen_some in [false, true].iter() {
                    for &nsec_some in [false, true].iter() {
                        let ctx = format!("{} encdet maclen={} nsec={}", base, maclen_some, nsec_some);
                        let (mut cc, mut rc) = (cbuf(mlen), cbuf(mlen));
                        let (mut cm, mut rm) = (cbuf(16), cbuf(16));
                        let (mut cml, mut rml) = (0xDEAD_BEEFu64, 0xDEAD_BEEFu64);
                        let cnsec = vec![0x11u8; 32];
                        let rnsec = vec![0x11u8; 32];
                        let rcc = unsafe {
                            (a.ced)(
                                cc.as_mut_ptr(),
                                cm.as_mut_ptr(),
                                u64p(&mut cml, maclen_some),
                                m.as_ptr(),
                                mlen as u64,
                                adp,
                                adlen as u64,
                                if nsec_some { cnsec.as_ptr() } else { null() },
                                npub.as_ptr(),
                                k.as_ptr(),
                            )
                        };
                        let rrr = unsafe {
                            (a.red)(
                                rc.as_mut_ptr(),
                                rm.as_mut_ptr(),
                                u64p(&mut rml, maclen_some),
                                m.as_ptr(),
                                mlen as u64,
                                adp,
                                adlen as u64,
                                if nsec_some { rnsec.as_ptr() } else { null() },
                                npub.as_ptr(),
                                k.as_ptr(),
                            )
                        };
                        common::eqi(&ctx, rcc, rrr);
                        assert_eq!(rcc, 0, "{}: expected 0", ctx);
                        chk(&format!("{} c", ctx), &cc, &rc, mlen);
                        chk(&format!("{} mac", ctx), &cm, &rm, 16);
                        assert_eq!(cml, rml, "{}: maclen_p mismatch", ctx);
                        assert_eq!(
                            cml,
                            if maclen_some { 16 } else { 0xDEAD_BEEF },
                            "{}: maclen_p value",
                            ctx
                        );
                        // nsec must be ignored, i.e. left untouched
                        assert!(cnsec.iter().all(|&b| b == 0x11), "{}: C touched nsec", ctx);
                        assert!(rnsec.iter().all(|&b| b == 0x11), "{}: Rust touched nsec", ctx);
                        ref_ct.copy_from_slice(&cc[..mlen]);
                        ref_mac.copy_from_slice(&cm[..16]);
                    }
                }

                // ---------------- encrypt (combined) ----------------
                for &clen_some in [false, true].iter() {
                    for &nsec_some in [false, true].iter() {
                        let ctx = format!("{} enc clen={} nsec={}", base, clen_some, nsec_some);
                        let (mut cc, mut rc) = (cbuf(mlen + 16), cbuf(mlen + 16));
                        let (mut ccl, mut rcl) = (0xDEAD_BEEFu64, 0xDEAD_BEEFu64);
                        let cnsec = vec![0x22u8; 32];
                        let rnsec = vec![0x22u8; 32];
                        let rcc = unsafe {
                            (a.ce)(
                                cc.as_mut_ptr(),
                                u64p(&mut ccl, clen_some),
                                m.as_ptr(),
                                mlen as u64,
                                adp,
                                adlen as u64,
                                if nsec_some { cnsec.as_ptr() } else { null() },
                                npub.as_ptr(),
                                k.as_ptr(),
                            )
                        };
                        let rrr = unsafe {
                            (a.re)(
                                rc.as_mut_ptr(),
                                u64p(&mut rcl, clen_some),
                                m.as_ptr(),
                                mlen as u64,
                                adp,
                                adlen as u64,
                                if nsec_some { rnsec.as_ptr() } else { null() },
                                npub.as_ptr(),
                                k.as_ptr(),
                            )
                        };
                        common::eqi(&ctx, rcc, rrr);
                        assert_eq!(rcc, 0, "{}: expected 0", ctx);
                        chk(&format!("{} c", ctx), &cc, &rc, mlen + 16);
                        assert_eq!(ccl, rcl, "{}: clen_p mismatch", ctx);
                        assert_eq!(
                            ccl,
                            if clen_some {
                                (mlen + 16) as u64
                            } else {
                                0xDEAD_BEEF
                            },
                            "{}: clen_p value",
                            ctx
                        );
                        assert!(cnsec.iter().all(|&b| b == 0x22), "{}: C touched nsec", ctx);
                        assert!(rnsec.iter().all(|&b| b == 0x22), "{}: Rust touched nsec", ctx);
                        // must equal detached ct || mac
                        common::eqb(&format!("{} c==det", ctx), &cc[..mlen], &ref_ct);
                        common::eqb(&format!("{} mac==det", ctx), &cc[mlen..mlen + 16], &ref_mac);
                    }
                }

                // ---------------- decrypt_detached (happy path) ----------------
                for &m_some in [true, false].iter() {
                    for &nsec_some in [false, true].iter() {
                        let ctx = format!("{} decdet m={} nsec={}", base, m_some, nsec_some);
                        let (mut cm, mut rm) = (cbuf(mlen), cbuf(mlen));
                        let mut cnsec = vec![0x33u8; 32];
                        let mut rnsec = vec![0x33u8; 32];
                        let rcc = unsafe {
                            (a.cdd)(
                                if m_some { cm.as_mut_ptr() } else { null_mut() },
                                if nsec_some { cnsec.as_mut_ptr() } else { null_mut() },
                                ref_ct.as_ptr(),
                                mlen as u64,
                                ref_mac.as_ptr(),
                                adp,
                                adlen as u64,
                                npub.as_ptr(),
                                k.as_ptr(),
                            )
                        };
                        let rrr = unsafe {
                            (a.rdd)(
                                if m_some { rm.as_mut_ptr() } else { null_mut() },
                                if nsec_some { rnsec.as_mut_ptr() } else { null_mut() },
                                ref_ct.as_ptr(),
                                mlen as u64,
                                ref_mac.as_ptr(),
                                adp,
                                adlen as u64,
                                npub.as_ptr(),
                                k.as_ptr(),
                            )
                        };
                        common::eqi(&ctx, rcc, rrr);
                        assert_eq!(rcc, 0, "{}: expected 0", ctx);
                        chk(&format!("{} m", ctx), &cm, &rm, mlen);
                        if m_some {
                            common::eqb(&format!("{} roundtrip", ctx), &cm[..mlen], &m);
                        }
                        assert!(cnsec.iter().all(|&b| b == 0x33), "{}: C touched nsec", ctx);
                        assert!(rnsec.iter().all(|&b| b == 0x33), "{}: Rust touched nsec", ctx);
                    }
                }

                // ---------------- decrypt (combined, happy path) ----------------
                let mut ct_full = ref_ct.clone();
                ct_full.extend_from_slice(&ref_mac);
                for &mlen_some in [false, true].iter() {
                    for &nsec_some in [false, true].iter() {
                        let ctx = format!("{} dec mlen_p={} nsec={}", base, mlen_some, nsec_some);
                        let (mut cm, mut rm) = (cbuf(mlen), cbuf(mlen));
                        let (mut cml, mut rml) = (0xDEAD_BEEFu64, 0xDEAD_BEEFu64);
                        let mut cnsec = vec![0x44u8; 32];
                        let mut rnsec = vec![0x44u8; 32];
                        let rcc = unsafe {
                            (a.cd)(
                                cm.as_mut_ptr(),
                                u64p(&mut cml, mlen_some),
                                if nsec_some { cnsec.as_mut_ptr() } else { null_mut() },
                                ct_full.as_ptr(),
                                ct_full.len() as u64,
                                adp,
                                adlen as u64,
                                npub.as_ptr(),
                                k.as_ptr(),
                            )
                        };
                        let rrr = unsafe {
                            (a.rd)(
                                rm.as_mut_ptr(),
                                u64p(&mut rml, mlen_some),
                                if nsec_some { rnsec.as_mut_ptr() } else { null_mut() },
                                ct_full.as_ptr(),
                                ct_full.len() as u64,
                                adp,
                                adlen as u64,
                                npub.as_ptr(),
                                k.as_ptr(),
                            )
                        };
                        common::eqi(&ctx, rcc, rrr);
                        assert_eq!(rcc, 0, "{}: expected 0", ctx);
                        chk(&format!("{} m", ctx), &cm, &rm, mlen);
                        common::eqb(&format!("{} roundtrip", ctx), &cm[..mlen], &m);
                        assert_eq!(cml, rml, "{}: mlen_p mismatch", ctx);
                        assert_eq!(
                            cml,
                            if mlen_some { mlen as u64 } else { 0xDEAD_BEEF },
                            "{}: mlen_p value",
                            ctx
                        );
                        assert!(cnsec.iter().all(|&b| b == 0x44), "{}: C touched nsec", ctx);
                        assert!(rnsec.iter().all(|&b| b == 0x44), "{}: Rust touched nsec", ctx);
                    }
                }

                // ---------------- in-place encryption (c == m) ----------------
                {
                    let ctx = format!("{} inplace", base);
                    let mut cbuf_ = cbuf(mlen + 16);
                    let mut rbuf_ = cbuf(mlen + 16);
                    cbuf_[..mlen].copy_from_slice(&m);
                    rbuf_[..mlen].copy_from_slice(&m);
                    let rcc = unsafe {
                        (a.ce)(
                            cbuf_.as_mut_ptr(),
                            null_mut(),
                            cbuf_.as_ptr(),
                            mlen as u64,
                            adp,
                            adlen as u64,
                            null(),
                            npub.as_ptr(),
                            k.as_ptr(),
                        )
                    };
                    let rrr = unsafe {
                        (a.re)(
                            rbuf_.as_mut_ptr(),
                            null_mut(),
                            rbuf_.as_ptr(),
                            mlen as u64,
                            adp,
                            adlen as u64,
                            null(),
                            npub.as_ptr(),
                            k.as_ptr(),
                        )
                    };
                    common::eqi(&ctx, rcc, rrr);
                    chk(&ctx, &cbuf_, &rbuf_, mlen + 16);
                    common::eqb(&format!("{} ct", ctx), &cbuf_[..mlen], &ref_ct);
                }
            }
        }
    }
}

#[test]
fn aead_chacha20poly1305_matrix() {
    aead_matrix(&aead_c20p1305(), 0x1111_2222_3333_4444);
}

#[test]
fn aead_chacha20poly1305_ietf_matrix() {
    aead_matrix(&aead_c20p1305_ietf(), 0x5555_6666_7777_8888);
}

#[test]
fn aead_xchacha20poly1305_ietf_matrix() {
    aead_matrix(&aead_xc20p1305_ietf(), 0x9999_aaaa_bbbb_cccc);
}

/// Tampering: every byte position of ciphertext / mac / ad flipped in turn,
/// plus wrong key and wrong nonce. Both `decrypt_detached` (m NULL and
/// non-NULL) and the combined `decrypt`.
fn aead_tamper(a: &Aead, seed: u64) {
    let mut rng = common::Rng::new(seed);
    for &mlen in [0usize, 1, 16, 17, 64, 65].iter() {
        for adlen in [0usize, 1, 16, 33].iter().copied() {
            let k = rng.bytes(32);
            let npub = rng.bytes(a.npub);
            let m = rng.bytes(mlen);
            let ad = rng.bytes(adlen);
            let adp: *const u8 = if adlen == 0 { null() } else { ad.as_ptr() };

            let mut ct = cbuf(mlen + 16);
            let rc0 = unsafe {
                (a.ce)(
                    ct.as_mut_ptr(),
                    null_mut(),
                    m.as_ptr(),
                    mlen as u64,
                    adp,
                    adlen as u64,
                    null(),
                    npub.as_ptr(),
                    k.as_ptr(),
                )
            };
            assert_eq!(rc0, 0);
            let ct = ct[..mlen + 16].to_vec();

            // helper closure running both libs on a mutated input
            let run = |ctx: &str,
                       ct: &[u8],
                       adp: *const u8,
                       adlen: usize,
                       npub: &[u8],
                       k: &[u8],
                       expect: c_int| {
                // combined decrypt, mlen_p non-NULL
                let (mut cm, mut rm) = (cbuf(mlen), cbuf(mlen));
                let (mut cml, mut rml) = (0xDEAD_BEEFu64, 0xDEAD_BEEFu64);
                let rcc = unsafe {
                    (a.cd)(
                        cm.as_mut_ptr(),
                        &mut cml,
                        null_mut(),
                        ct.as_ptr(),
                        ct.len() as u64,
                        adp,
                        adlen as u64,
                        npub.as_ptr(),
                        k.as_ptr(),
                    )
                };
                let rrr = unsafe {
                    (a.rd)(
                        rm.as_mut_ptr(),
                        &mut rml,
                        null_mut(),
                        ct.as_ptr(),
                        ct.len() as u64,
                        adp,
                        adlen as u64,
                        npub.as_ptr(),
                        k.as_ptr(),
                    )
                };
                common::eqi(&format!("{} dec", ctx), rcc, rrr);
                assert_eq!(rcc, expect, "{} dec: unexpected rc", ctx);
                assert_eq!(cml, rml, "{} dec: mlen_p", ctx);
                chk(&format!("{} dec m", ctx), &cm, &rm, mlen);

                // detached, m != NULL
                let (mut cm, mut rm) = (cbuf(mlen), cbuf(mlen));
                let rcc = unsafe {
                    (a.cdd)(
                        cm.as_mut_ptr(),
                        null_mut(),
                        ct.as_ptr(),
                        mlen as u64,
                        ct[mlen..].as_ptr(),
                        adp,
                        adlen as u64,
                        npub.as_ptr(),
                        k.as_ptr(),
                    )
                };
                let rrr = unsafe {
                    (a.rdd)(
                        rm.as_mut_ptr(),
                        null_mut(),
                        ct.as_ptr(),
                        mlen as u64,
                        ct[mlen..].as_ptr(),
                        adp,
                        adlen as u64,
                        npub.as_ptr(),
                        k.as_ptr(),
                    )
                };
                common::eqi(&format!("{} decdet", ctx), rcc, rrr);
                assert_eq!(rcc, expect, "{} decdet: unexpected rc", ctx);
                chk(&format!("{} decdet m", ctx), &cm, &rm, mlen);

                // detached, m == NULL (returns crypto_verify_16's result directly)
                let rcc = unsafe {
                    (a.cdd)(
                        null_mut(),
                        null_mut(),
                        ct.as_ptr(),
                        mlen as u64,
                        ct[mlen..].as_ptr(),
                        adp,
                        adlen as u64,
                        npub.as_ptr(),
                        k.as_ptr(),
                    )
                };
                let rrr = unsafe {
                    (a.rdd)(
                        null_mut(),
                        null_mut(),
                        ct.as_ptr(),
                        mlen as u64,
                        ct[mlen..].as_ptr(),
                        adp,
                        adlen as u64,
                        npub.as_ptr(),
                        k.as_ptr(),
                    )
                };
                common::eqi(&format!("{} decdet-null", ctx), rcc, rrr);
                assert_eq!(rcc, expect, "{} decdet-null: unexpected rc", ctx);
            };

            // sanity: untampered decrypts
            run("clean", &ct, adp, adlen, &npub, &k, 0);

            // flip every bit position of every ciphertext + mac byte
            for i in 0..ct.len() {
                for bit in [0x01u8, 0x80u8].iter().copied() {
                    let mut t = ct.clone();
                    t[i] ^= bit;
                    run(
                        &format!("{} mlen={} adlen={} ctflip[{}]^{:02x}", a.name, mlen, adlen, i, bit),
                        &t,
                        adp,
                        adlen,
                        &npub,
                        &k,
                        -1,
                    );
                }
            }
            // flip every ad byte
            for i in 0..adlen {
                let mut t = ad.clone();
                t[i] ^= 0x01;
                run(
                    &format!("{} mlen={} adlen={} adflip[{}]", a.name, mlen, adlen, i),
                    &ct,
                    t.as_ptr(),
                    adlen,
                    &npub,
                    &k,
                    -1,
                );
            }
            // ad length one too long / one too short (when possible)
            if adlen > 0 {
                run(
                    &format!("{} mlen={} adlen={} short-ad", a.name, mlen, adlen),
                    &ct,
                    ad.as_ptr(),
                    adlen - 1,
                    &npub,
                    &k,
                    -1,
                );
            }
            // wrong key
            for i in [0usize, 31].iter().copied() {
                let mut wk = k.clone();
                wk[i] ^= 0x40;
                run(
                    &format!("{} mlen={} adlen={} wrongkey[{}]", a.name, mlen, adlen, i),
                    &ct,
                    adp,
                    adlen,
                    &npub,
                    &wk,
                    -1,
                );
            }
            // wrong nonce
            for i in 0..a.npub {
                let mut wn = npub.clone();
                wn[i] ^= 0x08;
                run(
                    &format!("{} mlen={} adlen={} wrongnonce[{}]", a.name, mlen, adlen, i),
                    &ct,
                    adp,
                    adlen,
                    &wn,
                    &k,
                    -1,
                );
            }
        }
    }
}

#[test]
fn aead_chacha20poly1305_tamper() {
    aead_tamper(&aead_c20p1305(), 0x0102_0304_0506_0708);
}

#[test]
fn aead_chacha20poly1305_ietf_tamper() {
    aead_tamper(&aead_c20p1305_ietf(), 0x1112_1314_1516_1718);
}

#[test]
fn aead_xchacha20poly1305_ietf_tamper() {
    aead_tamper(&aead_xc20p1305_ietf(), 0x2122_2324_2526_2728);
}

/// `clen < ABYTES` short-ciphertext rejection for every clen in 0..16, with
/// `mlen_p` NULL and non-NULL.
fn aead_short_clen(a: &Aead, seed: u64) {
    let mut rng = common::Rng::new(seed);
    let k = rng.bytes(32);
    let npub = rng.bytes(a.npub);
    let ad = rng.bytes(7);
    let src = rng.bytes(16);
    for clen in 0..16usize {
        for &mlen_some in [false, true].iter() {
            for &ad_null in [false, true].iter() {
                let ctx = format!("{} shortclen={} mlen_p={} adnull={}", a.name, clen, mlen_some, ad_null);
                let (mut cm, mut rm) = (cbuf(16), cbuf(16));
                let (mut cml, mut rml) = (0xDEAD_BEEFu64, 0xDEAD_BEEFu64);
                let (adp, adlen): (*const u8, u64) = if ad_null {
                    (null(), 0)
                } else {
                    (ad.as_ptr(), 7)
                };
                let rcc = unsafe {
                    (a.cd)(
                        cm.as_mut_ptr(),
                        u64p(&mut cml, mlen_some),
                        null_mut(),
                        src.as_ptr(),
                        clen as u64,
                        adp,
                        adlen,
                        npub.as_ptr(),
                        k.as_ptr(),
                    )
                };
                let rrr = unsafe {
                    (a.rd)(
                        rm.as_mut_ptr(),
                        u64p(&mut rml, mlen_some),
                        null_mut(),
                        src.as_ptr(),
                        clen as u64,
                        adp,
                        adlen,
                        npub.as_ptr(),
                        k.as_ptr(),
                    )
                };
                common::eqi(&ctx, rcc, rrr);
                assert_eq!(rcc, -1, "{}: expected -1", ctx);
                assert_eq!(cml, rml, "{}: mlen_p", ctx);
                assert_eq!(
                    cml,
                    if mlen_some { 0 } else { 0xDEAD_BEEF },
                    "{}: mlen_p value",
                    ctx
                );
                chk(&ctx, &cm, &rm, 0); // m must not be touched at all
            }
        }
    }
}

#[test]
fn aead_short_ciphertext_rejection() {
    aead_short_clen(&aead_c20p1305(), 0x3132_3334);
    aead_short_clen(&aead_c20p1305_ietf(), 0x4142_4344);
    aead_short_clen(&aead_xc20p1305_ietf(), 0x5152_5354);
}

// =============================================== secretbox: low-level NaCl ===

type SbLow = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, *const u8) -> c_int;

fn secretbox_lowlevel(name: &str, seal: (SbLow, SbLow), open: (SbLow, SbLow), seed: u64) {
    let (cseal, rseal) = seal;
    let (copen, ropen) = open;
    let mut rng = common::Rng::new(seed);

    // --- mlen < ZEROBYTES(32) => -1, output untouched
    for mlen in 0..32usize {
        let k = rng.bytes(32);
        let n = rng.bytes(24);
        let m = vec![0u8; 32.max(mlen)];
        let ctx = format!("{} seal short mlen={}", name, mlen);
        let (mut cc, mut rc) = (cbuf(64), cbuf(64));
        let rcc = unsafe { cseal(cc.as_mut_ptr(), m.as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr()) };
        let rrr = unsafe { rseal(rc.as_mut_ptr(), m.as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr()) };
        common::eqi(&ctx, rcc, rrr);
        assert_eq!(rcc, -1, "{}: expected -1", ctx);
        chk(&ctx, &cc, &rc, 0);

        // --- clen < ZEROBYTES(32) => -1 for open as well
        let ctx = format!("{} open short clen={}", name, mlen);
        let src = rng.bytes(64);
        let (mut cm, mut rm) = (cbuf(64), cbuf(64));
        let rcc = unsafe { copen(cm.as_mut_ptr(), src.as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr()) };
        let rrr = unsafe { ropen(rm.as_mut_ptr(), src.as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr()) };
        common::eqi(&ctx, rcc, rrr);
        assert_eq!(rcc, -1, "{}: expected -1", ctx);
        chk(&ctx, &cm, &rm, 0);
    }

    // --- valid, zero-padded API
    for &plen in [0usize, 1, 15, 16, 17, 31, 32, 33, 63, 64, 65, 1000].iter() {
        let mlen = plen + 32; // ZEROBYTES padding
        for trial in 0..3usize {
            let k = rng.bytes(32);
            let n = rng.bytes(24);
            // m[0..32] must be zero (NaCl API); also exercise a non-zero prefix,
            // which the C accepts without any check.
            for &zero_pad in [true, false].iter() {
                let base = format!("{} plen={} trial={} zeropad={}", name, plen, trial, zero_pad);
                let mut m = rng.bytes(mlen);
                if zero_pad {
                    for b in m[..32].iter_mut() {
                        *b = 0;
                    }
                }
                let (mut cc, mut rc) = (cbuf(mlen), cbuf(mlen));
                let rcc =
                    unsafe { cseal(cc.as_mut_ptr(), m.as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr()) };
                let rrr =
                    unsafe { rseal(rc.as_mut_ptr(), m.as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr()) };
                common::eqi(&base, rcc, rrr);
                assert_eq!(rcc, 0, "{}: expected 0", base);
                chk(&format!("{} seal", base), &cc, &rc, mlen);
                assert!(cc[..16].iter().all(|&b| b == 0), "{}: BOXZEROBYTES not zeroed", base);
                let ct = cc[..mlen].to_vec();

                // open. NOTE: the NaCl low-level API derives the poly1305 key
                // from c[0..32] = stream[0..32] ^ m[0..32] when sealing, but
                // _open recomputes it as stream[0..32]. So a message whose
                // first ZEROBYTES bytes are NOT zero seals fine but never
                // opens (both implementations must agree on that -1).
                let open_exp: c_int = if zero_pad { 0 } else { -1 };
                let (mut cm, mut rm) = (cbuf(mlen), cbuf(mlen));
                let rcc =
                    unsafe { copen(cm.as_mut_ptr(), ct.as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr()) };
                let rrr =
                    unsafe { ropen(rm.as_mut_ptr(), ct.as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr()) };
                common::eqi(&format!("{} open", base), rcc, rrr);
                assert_eq!(rcc, open_exp, "{} open: unexpected rc", base);
                chk(&format!("{} open", base), &cm, &rm, mlen);
                if zero_pad {
                    assert!(cm[..32].iter().all(|&b| b == 0), "{}: ZEROBYTES not zeroed", base);
                    common::eqb(&format!("{} roundtrip", base), &cm[..mlen], &m);
                } else {
                    // rejected before m is written at all
                    assert!(cm.iter().all(|&b| b == CAN), "{}: m touched on failure", base);
                }

                // in-place seal (c == m)
                let ctx = format!("{} inplace-seal", base);
                let (mut cb, mut rb) = (cbuf(mlen), cbuf(mlen));
                cb[..mlen].copy_from_slice(&m);
                rb[..mlen].copy_from_slice(&m);
                let rcc =
                    unsafe { cseal(cb.as_mut_ptr(), cb.as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr()) };
                let rrr =
                    unsafe { rseal(rb.as_mut_ptr(), rb.as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr()) };
                common::eqi(&ctx, rcc, rrr);
                chk(&ctx, &cb, &rb, mlen);
                common::eqb(&format!("{} ==oop", ctx), &cb[..mlen], &ct);

                // in-place open (m == c)
                let ctx = format!("{} inplace-open", base);
                let (mut cb, mut rb) = (cbuf(mlen), cbuf(mlen));
                cb[..mlen].copy_from_slice(&ct);
                rb[..mlen].copy_from_slice(&ct);
                let rcc =
                    unsafe { copen(cb.as_mut_ptr(), cb.as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr()) };
                let rrr =
                    unsafe { ropen(rb.as_mut_ptr(), rb.as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr()) };
                common::eqi(&ctx, rcc, rrr);
                assert_eq!(rcc, open_exp, "{}: unexpected rc", ctx);
                chk(&ctx, &cb, &rb, mlen);

                // tampering: flip every byte of the ciphertext (mac lives at
                // c[16..32]); c[0..16] is ignored by the verifier.
                if plen <= 33 {
                    for i in 0..mlen {
                        let mut t = ct.clone();
                        t[i] ^= 0x01;
                        let ctx = format!("{} tamper[{}]", base, i);
                        let (mut cm, mut rm) = (cbuf(mlen), cbuf(mlen));
                        let rcc = unsafe {
                            copen(cm.as_mut_ptr(), t.as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr())
                        };
                        let rrr = unsafe {
                            ropen(rm.as_mut_ptr(), t.as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr())
                        };
                        common::eqi(&ctx, rcc, rrr);
                        chk(&ctx, &cm, &rm, mlen);
                        if i < 16 {
                            assert_eq!(rcc, open_exp, "{}: c[0..16] must be ignored", ctx);
                        } else {
                            assert_eq!(rcc, -1, "{}: expected -1", ctx);
                        }
                    }
                }
                // wrong key / nonce
                for (tag, wk, wn) in [
                    ("wrongkey", {
                        let mut x = k.clone();
                        x[5] ^= 0x10;
                        x
                    }, n.clone()),
                    ("wrongnonce", k.clone(), {
                        let mut x = n.clone();
                        x[5] ^= 0x10;
                        x
                    }),
                ]
                .iter()
                {
                    let ctx = format!("{} {}", base, tag);
                    let (mut cm, mut rm) = (cbuf(mlen), cbuf(mlen));
                    let rcc = unsafe {
                        copen(cm.as_mut_ptr(), ct.as_ptr(), mlen as u64, wn.as_ptr(), wk.as_ptr())
                    };
                    let rrr = unsafe {
                        ropen(rm.as_mut_ptr(), ct.as_ptr(), mlen as u64, wn.as_ptr(), wk.as_ptr())
                    };
                    common::eqi(&ctx, rcc, rrr);
                    assert_eq!(rcc, -1, "{}: expected -1", ctx);
                    chk(&ctx, &cm, &rm, mlen);
                }
            }
        }
    }
}

#[test]
fn secretbox_xsalsa20poly1305_lowlevel() {
    let seal = both!("crypto_secretbox_xsalsa20poly1305", SbLow);
    let open = both!("crypto_secretbox_xsalsa20poly1305_open", SbLow);
    secretbox_lowlevel("xsalsa20poly1305", seal, open, 0x6162_6364_6566_6768);
}

#[test]
fn secretbox_generic_lowlevel() {
    // crypto_secretbox()/crypto_secretbox_open() are thin wrappers around the
    // xsalsa20poly1305 ones.
    let seal = both!("crypto_secretbox", SbLow);
    let open = both!("crypto_secretbox_open", SbLow);
    secretbox_lowlevel("secretbox", seal, open, 0x7172_7374_7576_7778);
}

// ============================================ secretbox: easy / detached =====

type SbDet = unsafe extern "C" fn(*mut u8, *mut u8, *const u8, u64, *const u8, *const u8) -> c_int;
type SbOpenDet =
    unsafe extern "C" fn(*mut u8, *const u8, *const u8, u64, *const u8, *const u8) -> c_int;

struct Sb {
    name: &'static str,
    cdet: SbDet,
    rdet: SbDet,
    ceasy: SbLow,
    reasy: SbLow,
    codet: SbOpenDet,
    rodet: SbOpenDet,
    coeasy: SbLow,
    roeasy: SbLow,
}

macro_rules! mk_sb {
    ($name:literal, $det:literal, $easy:literal, $odet:literal, $oeasy:literal) => {{
        let (cdet, rdet) = both!($det, SbDet);
        let (ceasy, reasy) = both!($easy, SbLow);
        let (codet, rodet) = both!($odet, SbOpenDet);
        let (coeasy, roeasy) = both!($oeasy, SbLow);
        Sb {
            name: $name,
            cdet,
            rdet,
            ceasy,
            reasy,
            codet,
            rodet,
            coeasy,
            roeasy,
        }
    }};
}

fn sb_xsalsa() -> Sb {
    mk_sb!(
        "secretbox",
        "crypto_secretbox_detached",
        "crypto_secretbox_easy",
        "crypto_secretbox_open_detached",
        "crypto_secretbox_open_easy"
    )
}

fn sb_xchacha() -> Sb {
    mk_sb!(
        "secretbox_xchacha20poly1305",
        "crypto_secretbox_xchacha20poly1305_detached",
        "crypto_secretbox_xchacha20poly1305_easy",
        "crypto_secretbox_xchacha20poly1305_open_detached",
        "crypto_secretbox_xchacha20poly1305_open_easy"
    )
}

fn secretbox_easy_matrix(s: &Sb, seed: u64) {
    let mut rng = common::Rng::new(seed);
    for &mlen in [0usize, 1, 15, 16, 17, 31, 32, 33, 63, 64, 65, 1000].iter() {
        for trial in 0..3usize {
            let k = rng.bytes(32);
            let n = rng.bytes(24);
            let m = rng.bytes(mlen);
            let base = format!("{} mlen={} trial={}", s.name, mlen, trial);

            // ---------- detached ----------
            let (mut cc, mut rc) = (cbuf(mlen), cbuf(mlen));
            let (mut cmac, mut rmac) = (cbuf(16), cbuf(16));
            let rcc = unsafe {
                (s.cdet)(
                    cc.as_mut_ptr(),
                    cmac.as_mut_ptr(),
                    m.as_ptr(),
                    mlen as u64,
                    n.as_ptr(),
                    k.as_ptr(),
                )
            };
            let rrr = unsafe {
                (s.rdet)(
                    rc.as_mut_ptr(),
                    rmac.as_mut_ptr(),
                    m.as_ptr(),
                    mlen as u64,
                    n.as_ptr(),
                    k.as_ptr(),
                )
            };
            common::eqi(&format!("{} det", base), rcc, rrr);
            assert_eq!(rcc, 0);
            chk(&format!("{} det c", base), &cc, &rc, mlen);
            chk(&format!("{} det mac", base), &cmac, &rmac, 16);
            let ct = cc[..mlen].to_vec();
            let mac = cmac[..16].to_vec();

            // ---------- easy ----------
            let (mut cce, mut rce) = (cbuf(mlen + 16), cbuf(mlen + 16));
            let rcc = unsafe {
                (s.ceasy)(cce.as_mut_ptr(), m.as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr())
            };
            let rrr = unsafe {
                (s.reasy)(rce.as_mut_ptr(), m.as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr())
            };
            common::eqi(&format!("{} easy", base), rcc, rrr);
            assert_eq!(rcc, 0);
            chk(&format!("{} easy", base), &cce, &rce, mlen + 16);
            common::eqb(&format!("{} easy mac==det", base), &cce[..16], &mac);
            common::eqb(&format!("{} easy ct==det", base), &cce[16..16 + mlen], &ct);
            let ct_easy = cce[..mlen + 16].to_vec();

            // ---------- open_detached, m non-NULL and NULL ----------
            for &m_some in [true, false].iter() {
                let ctx = format!("{} opendet m={}", base, m_some);
                let (mut cm, mut rm) = (cbuf(mlen), cbuf(mlen));
                let rcc = unsafe {
                    (s.codet)(
                        if m_some { cm.as_mut_ptr() } else { null_mut() },
                        ct.as_ptr(),
                        mac.as_ptr(),
                        mlen as u64,
                        n.as_ptr(),
                        k.as_ptr(),
                    )
                };
                let rrr = unsafe {
                    (s.rodet)(
                        if m_some { rm.as_mut_ptr() } else { null_mut() },
                        ct.as_ptr(),
                        mac.as_ptr(),
                        mlen as u64,
                        n.as_ptr(),
                        k.as_ptr(),
                    )
                };
                common::eqi(&ctx, rcc, rrr);
                assert_eq!(rcc, 0, "{}: expected 0", ctx);
                chk(&ctx, &cm, &rm, mlen);
                if m_some {
                    common::eqb(&format!("{} roundtrip", ctx), &cm[..mlen], &m);
                }
            }

            // ---------- open_easy ----------
            let ctx = format!("{} openeasy", base);
            let (mut cm, mut rm) = (cbuf(mlen), cbuf(mlen));
            let rcc = unsafe {
                (s.coeasy)(
                    cm.as_mut_ptr(),
                    ct_easy.as_ptr(),
                    ct_easy.len() as u64,
                    n.as_ptr(),
                    k.as_ptr(),
                )
            };
            let rrr = unsafe {
                (s.roeasy)(
                    rm.as_mut_ptr(),
                    ct_easy.as_ptr(),
                    ct_easy.len() as u64,
                    n.as_ptr(),
                    k.as_ptr(),
                )
            };
            common::eqi(&ctx, rcc, rrr);
            assert_eq!(rcc, 0);
            chk(&ctx, &cm, &rm, mlen);
            common::eqb(&format!("{} roundtrip", ctx), &cm[..mlen], &m);

            // ---------- in-place easy: easy(buf, buf+16, mlen) ----------
            {
                let ctx = format!("{} inplace-easy", base);
                let (mut cb, mut rb) = (cbuf(mlen + 16), cbuf(mlen + 16));
                cb[16..16 + mlen].copy_from_slice(&m);
                rb[16..16 + mlen].copy_from_slice(&m);
                let rcc = unsafe {
                    (s.ceasy)(
                        cb.as_mut_ptr(),
                        cb.as_ptr().add(16),
                        mlen as u64,
                        n.as_ptr(),
                        k.as_ptr(),
                    )
                };
                let rrr = unsafe {
                    (s.reasy)(
                        rb.as_mut_ptr(),
                        rb.as_ptr().add(16),
                        mlen as u64,
                        n.as_ptr(),
                        k.as_ptr(),
                    )
                };
                common::eqi(&ctx, rcc, rrr);
                chk(&ctx, &cb, &rb, mlen + 16);
                common::eqb(&format!("{} ==oop", ctx), &cb[..mlen + 16], &ct_easy);

                // in-place open_easy: open_easy(buf, buf, clen)
                let ctx = format!("{} inplace-openeasy", base);
                let (mut cb, mut rb) = (cbuf(mlen + 16), cbuf(mlen + 16));
                cb[..mlen + 16].copy_from_slice(&ct_easy);
                rb[..mlen + 16].copy_from_slice(&ct_easy);
                let rcc = unsafe {
                    (s.coeasy)(
                        cb.as_mut_ptr(),
                        cb.as_ptr(),
                        (mlen + 16) as u64,
                        n.as_ptr(),
                        k.as_ptr(),
                    )
                };
                let rrr = unsafe {
                    (s.roeasy)(
                        rb.as_mut_ptr(),
                        rb.as_ptr(),
                        (mlen + 16) as u64,
                        n.as_ptr(),
                        k.as_ptr(),
                    )
                };
                common::eqi(&ctx, rcc, rrr);
                assert_eq!(rcc, 0);
                chk(&ctx, &cb, &rb, mlen + 16);
                common::eqb(&format!("{} plain", ctx), &cb[..mlen], &m);
            }

            // ---------- overlapping (memmove) branches of *_detached ----------
            // c = base+0, m = base+off with 0 < off < mlen  -> memmove(c, m, mlen)
            // c = base+off, m = base+0 with 0 < off < mlen  -> memmove(c, m, mlen)
            for off in [1usize, 3, 8, 16].iter().copied() {
                if off >= mlen {
                    continue;
                }
                for &c_first in [true, false].iter() {
                    let ctx = format!("{} overlap off={} c_first={}", base, off, c_first);
                    let total = mlen + off + 32;
                    let (mut cb, mut rb) = (cbuf(total), cbuf(total));
                    let (mut cmac2, mut rmac2) = (cbuf(16), cbuf(16));
                    let (cd, md) = if c_first { (0usize, off) } else { (off, 0usize) };
                    cb[md..md + mlen].copy_from_slice(&m);
                    rb[md..md + mlen].copy_from_slice(&m);
                    let rcc = unsafe {
                        (s.cdet)(
                            cb.as_mut_ptr().add(cd),
                            cmac2.as_mut_ptr(),
                            cb.as_ptr().add(md),
                            mlen as u64,
                            n.as_ptr(),
                            k.as_ptr(),
                        )
                    };
                    let rrr = unsafe {
                        (s.rdet)(
                            rb.as_mut_ptr().add(cd),
                            rmac2.as_mut_ptr(),
                            rb.as_ptr().add(md),
                            mlen as u64,
                            n.as_ptr(),
                            k.as_ptr(),
                        )
                    };
                    common::eqi(&ctx, rcc, rrr);
                    chk(&format!("{} buf", ctx), &cb, &rb, total);
                    chk(&format!("{} mac", ctx), &cmac2, &rmac2, 16);
                    common::eqb(&format!("{} ct", ctx), &cb[cd..cd + mlen], &ct);
                    common::eqb(&format!("{} mac==oop", ctx), &cmac2[..16], &mac);

                    // ... and the matching open_detached overlap branch
                    let ctx = format!("{} overlap-open off={} c_first={}", base, off, c_first);
                    let (mut cb, mut rb) = (cbuf(total), cbuf(total));
                    // the ciphertext lives at offset `cd`, plaintext goes to `md`
                    cb[cd..cd + mlen].copy_from_slice(&ct);
                    rb[cd..cd + mlen].copy_from_slice(&ct);
                    let rcc = unsafe {
                        (s.codet)(
                            cb.as_mut_ptr().add(md),
                            cb.as_ptr().add(cd),
                            mac.as_ptr(),
                            mlen as u64,
                            n.as_ptr(),
                            k.as_ptr(),
                        )
                    };
                    let rrr = unsafe {
                        (s.rodet)(
                            rb.as_mut_ptr().add(md),
                            rb.as_ptr().add(cd),
                            mac.as_ptr(),
                            mlen as u64,
                            n.as_ptr(),
                            k.as_ptr(),
                        )
                    };
                    common::eqi(&ctx, rcc, rrr);
                    assert_eq!(rcc, 0, "{}: expected 0", ctx);
                    chk(&format!("{} buf", ctx), &cb, &rb, total);
                    common::eqb(&format!("{} plain", ctx), &cb[md..md + mlen], &m);
                }
            }

            // ---------- tampering ----------
            if mlen <= 65 {
                for i in 0..mlen + 16 {
                    let mut t = ct_easy.clone();
                    t[i] ^= 0x01;
                    let ctx = format!("{} tamper[{}]", base, i);
                    let (mut cm, mut rm) = (cbuf(mlen), cbuf(mlen));
                    let rcc = unsafe {
                        (s.coeasy)(
                            cm.as_mut_ptr(),
                            t.as_ptr(),
                            t.len() as u64,
                            n.as_ptr(),
                            k.as_ptr(),
                        )
                    };
                    let rrr = unsafe {
                        (s.roeasy)(
                            rm.as_mut_ptr(),
                            t.as_ptr(),
                            t.len() as u64,
                            n.as_ptr(),
                            k.as_ptr(),
                        )
                    };
                    common::eqi(&ctx, rcc, rrr);
                    assert_eq!(rcc, -1, "{}: expected -1", ctx);
                    // C returns before writing anything to m
                    chk(&ctx, &cm, &rm, 0);
                }
                // tamper the detached mac, every byte
                for i in 0..16usize {
                    let mut t = mac.clone();
                    t[i] ^= 0x80;
                    let ctx = format!("{} mactamper[{}]", base, i);
                    let (mut cm, mut rm) = (cbuf(mlen), cbuf(mlen));
                    let rcc = unsafe {
                        (s.codet)(
                            cm.as_mut_ptr(),
                            ct.as_ptr(),
                            t.as_ptr(),
                            mlen as u64,
                            n.as_ptr(),
                            k.as_ptr(),
                        )
                    };
                    let rrr = unsafe {
                        (s.rodet)(
                            rm.as_mut_ptr(),
                            ct.as_ptr(),
                            t.as_ptr(),
                            mlen as u64,
                            n.as_ptr(),
                            k.as_ptr(),
                        )
                    };
                    common::eqi(&ctx, rcc, rrr);
                    assert_eq!(rcc, -1, "{}: expected -1", ctx);
                    chk(&ctx, &cm, &rm, 0);
                }
            }

            // ---------- wrong key / nonce ----------
            for (tag, wk, wn) in [
                ("wrongkey", {
                    let mut x = k.clone();
                    x[9] ^= 0x20;
                    x
                }, n.clone()),
                ("wrongnonce", k.clone(), {
                    let mut x = n.clone();
                    x[23] ^= 0x20;
                    x
                }),
            ]
            .iter()
            {
                let ctx = format!("{} {}", base, tag);
                let (mut cm, mut rm) = (cbuf(mlen), cbuf(mlen));
                let rcc = unsafe {
                    (s.coeasy)(
                        cm.as_mut_ptr(),
                        ct_easy.as_ptr(),
                        ct_easy.len() as u64,
                        wn.as_ptr(),
                        wk.as_ptr(),
                    )
                };
                let rrr = unsafe {
                    (s.roeasy)(
                        rm.as_mut_ptr(),
                        ct_easy.as_ptr(),
                        ct_easy.len() as u64,
                        wn.as_ptr(),
                        wk.as_ptr(),
                    )
                };
                common::eqi(&ctx, rcc, rrr);
                assert_eq!(rcc, -1, "{}: expected -1", ctx);
                chk(&ctx, &cm, &rm, 0);
            }
        }
    }

    // ---------- open_easy with clen < MACBYTES ----------
    let k = rng.bytes(32);
    let n = rng.bytes(24);
    let src = rng.bytes(16);
    for clen in 0..16usize {
        let ctx = format!("{} openeasy short clen={}", s.name, clen);
        let (mut cm, mut rm) = (cbuf(16), cbuf(16));
        let rcc = unsafe {
            (s.coeasy)(cm.as_mut_ptr(), src.as_ptr(), clen as u64, n.as_ptr(), k.as_ptr())
        };
        let rrr = unsafe {
            (s.roeasy)(rm.as_mut_ptr(), src.as_ptr(), clen as u64, n.as_ptr(), k.as_ptr())
        };
        common::eqi(&ctx, rcc, rrr);
        assert_eq!(rcc, -1, "{}: expected -1", ctx);
        chk(&ctx, &cm, &rm, 0);
    }
}

#[test]
fn secretbox_easy_xsalsa20poly1305() {
    secretbox_easy_matrix(&sb_xsalsa(), 0x8182_8384_8586_8788);
}

#[test]
fn secretbox_easy_xchacha20poly1305() {
    secretbox_easy_matrix(&sb_xchacha(), 0x9192_9394_9596_9798);
}

// ============================================================ secretstream ===

type SsInitPull = unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> c_int;
type SsInitPush = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> c_int;
type SsRekey = unsafe extern "C" fn(*mut u8);
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

const SS_STATEBYTES: usize = 52;
const SS_ABYTES: usize = 17;
const SS_HEADERBYTES: usize = 24;

struct Ss {
    cip: SsInitPull,
    rip: SsInitPull,
    cpush: SsPush,
    rpush: SsPush,
    cpull: SsPull,
    rpull: SsPull,
    crekey: SsRekey,
    rrekey: SsRekey,
}

fn ss() -> Ss {
    let (cip, rip) = both!("crypto_secretstream_xchacha20poly1305_init_pull", SsInitPull);
    let (cpush, rpush) = both!("crypto_secretstream_xchacha20poly1305_push", SsPush);
    let (cpull, rpull) = both!("crypto_secretstream_xchacha20poly1305_pull", SsPull);
    let (crekey, rrekey) = both!("crypto_secretstream_xchacha20poly1305_rekey", SsRekey);
    Ss {
        cip,
        rip,
        cpush,
        rpush,
        cpull,
        rpull,
        crekey,
        rrekey,
    }
}

/// Initialise a pair of state buffers from the same header+key and check they
/// are byte-identical (the state is 3 `unsigned char` arrays, so there is no
/// uninitialised padding and a full-buffer comparison is valid).
fn ss_init_pull_pair(s: &Ss, header: &[u8], k: &[u8], ctx: &str) -> (Vec<u8>, Vec<u8>) {
    let mut cs = vec![CAN; SS_STATEBYTES + TAIL];
    let mut rs = vec![CAN; SS_STATEBYTES + TAIL];
    let rcc = unsafe { (s.cip)(cs.as_mut_ptr(), header.as_ptr(), k.as_ptr()) };
    let rrr = unsafe { (s.rip)(rs.as_mut_ptr(), header.as_ptr(), k.as_ptr()) };
    common::eqi(&format!("{} init_pull", ctx), rcc, rrr);
    assert_eq!(rcc, 0);
    chk(&format!("{} init_pull state", ctx), &cs, &rs, SS_STATEBYTES);
    (cs, rs)
}

#[test]
fn secretstream_statebytes_and_init_pull() {
    let s = ss();
    let (c, r) = both!(
        "crypto_secretstream_xchacha20poly1305_statebytes",
        unsafe extern "C" fn() -> usize
    );
    unsafe {
        assert_eq!(c(), r());
        assert_eq!(c(), SS_STATEBYTES);
    }
    let mut rng = common::Rng::new(0xA1A2_A3A4_A5A6_A7A8);
    for _ in 0..40 {
        let header = rng.bytes(SS_HEADERBYTES);
        let k = rng.bytes(32);
        ss_init_pull_pair(&s, &header, &k, "rand");
    }
    // all-zero and all-ff edge inputs
    ss_init_pull_pair(&s, &[0u8; SS_HEADERBYTES], &[0u8; 32], "zeros");
    ss_init_pull_pair(&s, &[0xffu8; SS_HEADERBYTES], &[0xffu8; 32], "ffs");
}

#[test]
fn secretstream_explicit_rekey() {
    let s = ss();
    let mut rng = common::Rng::new(0xB1B2_B3B4_B5B6_B7B8);
    for _ in 0..20 {
        let header = rng.bytes(SS_HEADERBYTES);
        let k = rng.bytes(32);
        let (mut cs, mut rs) = ss_init_pull_pair(&s, &header, &k, "rekey");
        for round in 0..4 {
            unsafe {
                (s.crekey)(cs.as_mut_ptr());
                (s.rrekey)(rs.as_mut_ptr());
            }
            chk(
                &format!("rekey round={} state", round),
                &cs,
                &rs,
                SS_STATEBYTES,
            );
        }
    }
    // rekey on an arbitrary (crafted) state, including all-zero and all-ff
    for fill in [0x00u8, 0xff, 0x5a].iter().copied() {
        let mut cs = vec![CAN; SS_STATEBYTES + TAIL];
        let mut rs = vec![CAN; SS_STATEBYTES + TAIL];
        for i in 0..SS_STATEBYTES {
            cs[i] = fill;
            rs[i] = fill;
        }
        unsafe {
            (s.crekey)(cs.as_mut_ptr());
            (s.rrekey)(rs.as_mut_ptr());
        }
        chk(&format!("rekey crafted fill={:02x}", fill), &cs, &rs, SS_STATEBYTES);
    }
}

/// A full multi-message session: every ciphertext must match byte-for-byte and
/// the complete 52-byte state must match after every single operation.
#[test]
fn secretstream_session() {
    let s = ss();
    let mut rng = common::Rng::new(0xC1C2_C3C4_C5C6_C7C8);

    // tags: the four documented ones plus out-of-range byte values (C takes an
    // `unsigned char`, so any value is accepted).
    let tags: [u8; 8] = [0x00, 0x01, 0x02, 0x03, 0x04, 0x7f, 0x80, 0xff];

    for &mlen in [0usize, 1, 15, 16, 17, 47, 63, 64, 65, 1000].iter() {
        for &tag in tags.iter() {
            for adcase in 0..4usize {
                let header = rng.bytes(SS_HEADERBYTES);
                let k = rng.bytes(32);
                let (mut cs, mut rs) = ss_init_pull_pair(&s, &header, &k, "session");
                let (mut cs2, mut rs2) = ss_init_pull_pair(&s, &header, &k, "session-pull");

                for msg in 0..4usize {
                    let m = rng.bytes(mlen);
                    let (adlen, adv, ad_null) = match adcase {
                        0 => (0usize, Vec::new(), true),
                        1 => (0usize, rng.bytes(4), false),
                        2 => (1usize, rng.bytes(1), false),
                        _ => {
                            let n = 1 + rng.below(40);
                            (n, rng.bytes(n), false)
                        }
                    };
                    let adp: *const u8 = if ad_null { null() } else { adv.as_ptr() };
                    let base = format!(
                        "ss mlen={} tag={:02x} adcase={} adlen={} msg={}",
                        mlen, tag, adcase, adlen, msg
                    );

                    // ----- push (outlen_p non-NULL on even messages) -----
                    let outlen_some = msg % 2 == 0;
                    let (mut co, mut ro) = (cbuf(mlen + SS_ABYTES), cbuf(mlen + SS_ABYTES));
                    let (mut col, mut rol) = (0xDEAD_BEEFu64, 0xDEAD_BEEFu64);
                    let rcc = unsafe {
                        (s.cpush)(
                            cs.as_mut_ptr(),
                            co.as_mut_ptr(),
                            u64p(&mut col, outlen_some),
                            m.as_ptr(),
                            mlen as u64,
                            adp,
                            adlen as u64,
                            tag,
                        )
                    };
                    let rrr = unsafe {
                        (s.rpush)(
                            rs.as_mut_ptr(),
                            ro.as_mut_ptr(),
                            u64p(&mut rol, outlen_some),
                            m.as_ptr(),
                            mlen as u64,
                            adp,
                            adlen as u64,
                            tag,
                        )
                    };
                    common::eqi(&format!("{} push", base), rcc, rrr);
                    assert_eq!(rcc, 0);
                    chk(&format!("{} push out", base), &co, &ro, mlen + SS_ABYTES);
                    chk(&format!("{} push state", base), &cs, &rs, SS_STATEBYTES);
                    assert_eq!(col, rol, "{}: outlen_p", base);
                    assert_eq!(
                        col,
                        if outlen_some {
                            (mlen + SS_ABYTES) as u64
                        } else {
                            0xDEAD_BEEF
                        },
                        "{}: outlen_p value",
                        base
                    );
                    let ct = co[..mlen + SS_ABYTES].to_vec();

                    // ----- pull -----
                    let ptrs_some = msg % 2 == 1;
                    let (mut cm, mut rm) = (cbuf(mlen), cbuf(mlen));
                    let (mut cml, mut rml) = (0xDEAD_BEEFu64, 0xDEAD_BEEFu64);
                    let (mut ctag, mut rtag) = (0x77u8, 0x77u8);
                    let rcc = unsafe {
                        (s.cpull)(
                            cs2.as_mut_ptr(),
                            cm.as_mut_ptr(),
                            u64p(&mut cml, ptrs_some),
                            if ptrs_some { &mut ctag } else { null_mut() },
                            ct.as_ptr(),
                            ct.len() as u64,
                            adp,
                            adlen as u64,
                        )
                    };
                    let rrr = unsafe {
                        (s.rpull)(
                            rs2.as_mut_ptr(),
                            rm.as_mut_ptr(),
                            u64p(&mut rml, ptrs_some),
                            if ptrs_some { &mut rtag } else { null_mut() },
                            ct.as_ptr(),
                            ct.len() as u64,
                            adp,
                            adlen as u64,
                        )
                    };
                    common::eqi(&format!("{} pull", base), rcc, rrr);
                    assert_eq!(rcc, 0, "{} pull: expected 0", base);
                    chk(&format!("{} pull m", base), &cm, &rm, mlen);
                    chk(&format!("{} pull state", base), &cs2, &rs2, SS_STATEBYTES);
                    assert_eq!(cml, rml, "{}: mlen_p", base);
                    assert_eq!(ctag, rtag, "{}: tag_p", base);
                    if ptrs_some {
                        assert_eq!(cml, mlen as u64, "{}: mlen_p value", base);
                        assert_eq!(ctag, tag, "{}: tag_p value", base);
                    } else {
                        assert_eq!(cml, 0xDEAD_BEEF);
                        assert_eq!(ctag, 0x77);
                    }
                    common::eqb(&format!("{} roundtrip", base), &cm[..mlen], &m);

                    // both states must still agree with each other after the
                    // push/pull pair (push and pull advance the state the same way)
                    common::eqb(
                        &format!("{} push-vs-pull state", base),
                        &cs[..SS_STATEBYTES],
                        &cs2[..SS_STATEBYTES],
                    );
                }
            }
        }
    }
}

#[test]
fn secretstream_pull_errors() {
    let s = ss();
    let mut rng = common::Rng::new(0xD1D2_D3D4_D5D6_D7D8);

    // ---- inlen < ABYTES for every inlen 0..17
    let header = rng.bytes(SS_HEADERBYTES);
    let k = rng.bytes(32);
    let src = rng.bytes(32);
    for inlen in 0..SS_ABYTES {
        for &ptrs_some in [false, true].iter() {
            let (mut cs, mut rs) = ss_init_pull_pair(&s, &header, &k, "shortin");
            let state0 = cs.clone();
            let ctx = format!("pull inlen={} ptrs={}", inlen, ptrs_some);
            let (mut cm, mut rm) = (cbuf(32), cbuf(32));
            let (mut cml, mut rml) = (0xDEAD_BEEFu64, 0xDEAD_BEEFu64);
            let (mut ctag, mut rtag) = (0x77u8, 0x77u8);
            let rcc = unsafe {
                (s.cpull)(
                    cs.as_mut_ptr(),
                    cm.as_mut_ptr(),
                    u64p(&mut cml, ptrs_some),
                    if ptrs_some { &mut ctag } else { null_mut() },
                    src.as_ptr(),
                    inlen as u64,
                    null(),
                    0,
                )
            };
            let rrr = unsafe {
                (s.rpull)(
                    rs.as_mut_ptr(),
                    rm.as_mut_ptr(),
                    u64p(&mut rml, ptrs_some),
                    if ptrs_some { &mut rtag } else { null_mut() },
                    src.as_ptr(),
                    inlen as u64,
                    null(),
                    0,
                )
            };
            common::eqi(&ctx, rcc, rrr);
            assert_eq!(rcc, -1, "{}: expected -1", ctx);
            chk(&format!("{} m", ctx), &cm, &rm, 0);
            chk(&format!("{} state", ctx), &cs, &rs, SS_STATEBYTES);
            common::eqb(&format!("{} state unchanged", ctx), &state0, &cs);
            assert_eq!(cml, rml, "{}: mlen_p", ctx);
            assert_eq!(ctag, rtag, "{}: tag_p", ctx);
            assert_eq!(cml, if ptrs_some { 0 } else { 0xDEAD_BEEF });
            assert_eq!(ctag, if ptrs_some { 0xff } else { 0x77 });
        }
    }

    // ---- corrupted ciphertext: flip every byte, incl. the tag byte and mac
    for &mlen in [0usize, 1, 16, 17, 33].iter() {
        for adlen in [0usize, 5].iter().copied() {
            let header = rng.bytes(SS_HEADERBYTES);
            let k = rng.bytes(32);
            let m = rng.bytes(mlen);
            let ad = rng.bytes(adlen);
            let adp: *const u8 = if adlen == 0 { null() } else { ad.as_ptr() };

            let (mut cs, _rs) = ss_init_pull_pair(&s, &header, &k, "corrupt-push");
            let mut ct = cbuf(mlen + SS_ABYTES);
            let rc0 = unsafe {
                (s.cpush)(
                    cs.as_mut_ptr(),
                    ct.as_mut_ptr(),
                    null_mut(),
                    m.as_ptr(),
                    mlen as u64,
                    adp,
                    adlen as u64,
                    0x00,
                )
            };
            assert_eq!(rc0, 0);
            let ct = ct[..mlen + SS_ABYTES].to_vec();

            for i in 0..ct.len() {
                let mut t = ct.clone();
                t[i] ^= 0x01;
                let ctx = format!("ss corrupt mlen={} adlen={} byte={}", mlen, adlen, i);
                let (mut cs, mut rs) = ss_init_pull_pair(&s, &header, &k, &ctx);
                let state0 = cs.clone();
                let (mut cm, mut rm) = (cbuf(mlen), cbuf(mlen));
                let (mut cml, mut rml) = (0xDEAD_BEEFu64, 0xDEAD_BEEFu64);
                let (mut ctag, mut rtag) = (0x77u8, 0x77u8);
                let rcc = unsafe {
                    (s.cpull)(
                        cs.as_mut_ptr(),
                        cm.as_mut_ptr(),
                        &mut cml,
                        &mut ctag,
                        t.as_ptr(),
                        t.len() as u64,
                        adp,
                        adlen as u64,
                    )
                };
                let rrr = unsafe {
                    (s.rpull)(
                        rs.as_mut_ptr(),
                        rm.as_mut_ptr(),
                        &mut rml,
                        &mut rtag,
                        t.as_ptr(),
                        t.len() as u64,
                        adp,
                        adlen as u64,
                    )
                };
                common::eqi(&ctx, rcc, rrr);
                assert_eq!(rcc, -1, "{}: expected -1", ctx);
                chk(&format!("{} m", ctx), &cm, &rm, 0);
                chk(&format!("{} state", ctx), &cs, &rs, SS_STATEBYTES);
                common::eqb(&format!("{} state unchanged", ctx), &state0, &cs);
                assert_eq!(cml, rml);
                assert_eq!(ctag, rtag);
                assert_eq!(cml, 0);
                assert_eq!(ctag, 0xff);
            }

            // corrupted / mismatched ad
            for i in 0..adlen {
                let mut t = ad.clone();
                t[i] ^= 0x01;
                let ctx = format!("ss corrupt-ad mlen={} byte={}", mlen, i);
                let (mut cs, mut rs) = ss_init_pull_pair(&s, &header, &k, &ctx);
                let (mut cm, mut rm) = (cbuf(mlen), cbuf(mlen));
                let rcc = unsafe {
                    (s.cpull)(
                        cs.as_mut_ptr(),
                        cm.as_mut_ptr(),
                        null_mut(),
                        null_mut(),
                        ct.as_ptr(),
                        ct.len() as u64,
                        t.as_ptr(),
                        adlen as u64,
                    )
                };
                let rrr = unsafe {
                    (s.rpull)(
                        rs.as_mut_ptr(),
                        rm.as_mut_ptr(),
                        null_mut(),
                        null_mut(),
                        ct.as_ptr(),
                        ct.len() as u64,
                        t.as_ptr(),
                        adlen as u64,
                    )
                };
                common::eqi(&ctx, rcc, rrr);
                assert_eq!(rcc, -1, "{}: expected -1", ctx);
                chk(&format!("{} m", ctx), &cm, &rm, 0);
                chk(&format!("{} state", ctx), &cs, &rs, SS_STATEBYTES);
            }

            // wrong key => wrong state => mac mismatch
            let ctx = format!("ss wrongkey mlen={}", mlen);
            let mut wk = k.clone();
            wk[3] ^= 0x10;
            let (mut cs, mut rs) = ss_init_pull_pair(&s, &header, &wk, &ctx);
            let (mut cm, mut rm) = (cbuf(mlen), cbuf(mlen));
            let rcc = unsafe {
                (s.cpull)(
                    cs.as_mut_ptr(),
                    cm.as_mut_ptr(),
                    null_mut(),
                    null_mut(),
                    ct.as_ptr(),
                    ct.len() as u64,
                    adp,
                    adlen as u64,
                )
            };
            let rrr = unsafe {
                (s.rpull)(
                    rs.as_mut_ptr(),
                    rm.as_mut_ptr(),
                    null_mut(),
                    null_mut(),
                    ct.as_ptr(),
                    ct.len() as u64,
                    adp,
                    adlen as u64,
                )
            };
            common::eqi(&ctx, rcc, rrr);
            assert_eq!(rcc, -1, "{}: expected -1", ctx);
            chk(&format!("{} m", ctx), &cm, &rm, 0);
            chk(&format!("{} state", ctx), &cs, &rs, SS_STATEBYTES);
        }
    }
}

/// The `sodium_is_zero(counter)` re-key branch of push/pull: we craft the state
/// so the 32-bit counter is 0xffffffff and one `sodium_increment` wraps it to 0.
#[test]
fn secretstream_counter_wrap_rekey() {
    let s = ss();
    let mut rng = common::Rng::new(0xE1E2_E3E4_E5E6_E7E8);
    for &counter in [
        [0xffu8, 0xff, 0xff, 0xff],
        [0xfeu8, 0xff, 0xff, 0xff],
        [0x00u8, 0x00, 0x00, 0x00],
        [0x01u8, 0x00, 0x00, 0x00],
    ]
    .iter()
    {
        for &tag in [0x00u8, 0x01, 0x02, 0x03].iter() {
            for &mlen in [0usize, 1, 17, 64].iter() {
                let header = rng.bytes(SS_HEADERBYTES);
                let k = rng.bytes(32);
                let m = rng.bytes(mlen);
                let ctx = format!(
                    "wrap counter={:02x?} tag={:02x} mlen={}",
                    counter, tag, mlen
                );
                let (mut cs, mut rs) = ss_init_pull_pair(&s, &header, &k, &ctx);
                // overwrite the 4-byte counter (state->nonce[0..4]) in both states
                for i in 0..4 {
                    cs[32 + i] = counter[i];
                    rs[32 + i] = counter[i];
                }
                let (mut co, mut ro) = (cbuf(mlen + SS_ABYTES), cbuf(mlen + SS_ABYTES));
                let (mut col, mut rol) = (0u64, 0u64);
                let rcc = unsafe {
                    (s.cpush)(
                        cs.as_mut_ptr(),
                        co.as_mut_ptr(),
                        &mut col,
                        m.as_ptr(),
                        mlen as u64,
                        null(),
                        0,
                        tag,
                    )
                };
                let rrr = unsafe {
                    (s.rpush)(
                        rs.as_mut_ptr(),
                        ro.as_mut_ptr(),
                        &mut rol,
                        m.as_ptr(),
                        mlen as u64,
                        null(),
                        0,
                        tag,
                    )
                };
                common::eqi(&ctx, rcc, rrr);
                assert_eq!(col, rol);
                chk(&format!("{} out", ctx), &co, &ro, mlen + SS_ABYTES);
                chk(&format!("{} state", ctx), &cs, &rs, SS_STATEBYTES);

                // and the corresponding pull, from the same crafted state
                let (mut cs2, mut rs2) = ss_init_pull_pair(&s, &header, &k, &ctx);
                for i in 0..4 {
                    cs2[32 + i] = counter[i];
                    rs2[32 + i] = counter[i];
                }
                let (mut cm, mut rm) = (cbuf(mlen), cbuf(mlen));
                let (mut cml, mut rml) = (0u64, 0u64);
                let (mut ctag, mut rtag) = (0u8, 0u8);
                let rcc = unsafe {
                    (s.cpull)(
                        cs2.as_mut_ptr(),
                        cm.as_mut_ptr(),
                        &mut cml,
                        &mut ctag,
                        co.as_ptr(),
                        (mlen + SS_ABYTES) as u64,
                        null(),
                        0,
                    )
                };
                let rrr = unsafe {
                    (s.rpull)(
                        rs2.as_mut_ptr(),
                        rm.as_mut_ptr(),
                        &mut rml,
                        &mut rtag,
                        ro.as_ptr(),
                        (mlen + SS_ABYTES) as u64,
                        null(),
                        0,
                    )
                };
                common::eqi(&format!("{} pull", ctx), rcc, rrr);
                assert_eq!(rcc, 0, "{} pull: expected 0", ctx);
                assert_eq!((cml, ctag), (rml, rtag));
                assert_eq!(ctag, tag);
                chk(&format!("{} pull m", ctx), &cm, &rm, mlen);
                chk(&format!("{} pull state", ctx), &cs2, &rs2, SS_STATEBYTES);
                common::eqb(&format!("{} pull roundtrip", ctx), &cm[..mlen], &m);
            }
        }
    }
}

// ================================================= keygen / init_push (RNG) ==

/// `*_keygen()` and `secretstream_init_push()` are the only functions in this
/// area that consume `randombytes_buf()`. We install a deterministic
/// `randombytes` implementation into both libraries so their outputs are
/// directly comparable.
#[test]
fn keygen_and_init_push() {
    install_det_randombytes();

    macro_rules! keygen {
        ($name:literal, $len:expr) => {{
            let (c, r) = both!($name, unsafe extern "C" fn(*mut u8));
            for round in 0..8u64 {
                let (mut co, mut ro) = (cbuf($len), cbuf($len));
                seq_reset(0x1234_5678 ^ round);
                unsafe { c(co.as_mut_ptr()) };
                seq_reset(0x1234_5678 ^ round);
                unsafe { r(ro.as_mut_ptr()) };
                chk(&format!("{} round={}", $name, round), &co, &ro, $len);
                assert!(
                    co[..$len].iter().any(|&b| b != 0),
                    "{}: key is all zero",
                    $name
                );
            }
        }};
    }

    keygen!("crypto_aead_chacha20poly1305_keygen", 32);
    keygen!("crypto_aead_chacha20poly1305_ietf_keygen", 32);
    keygen!("crypto_aead_xchacha20poly1305_ietf_keygen", 32);
    keygen!("crypto_secretbox_keygen", 32);
    keygen!("crypto_secretbox_xsalsa20poly1305_keygen", 32);
    keygen!("crypto_secretstream_xchacha20poly1305_keygen", 32);

    // ---- init_push: header comes from randombytes_buf, so it is comparable too
    let s = ss();
    let (cipush, ripush) = both!(
        "crypto_secretstream_xchacha20poly1305_init_push",
        SsInitPush
    );
    let mut rng = common::Rng::new(0xF1F2_F3F4_F5F6_F7F8);
    for round in 0..20u64 {
        let k = rng.bytes(32);
        let (mut ch, mut rh) = (cbuf(SS_HEADERBYTES), cbuf(SS_HEADERBYTES));
        let mut cs = vec![CAN; SS_STATEBYTES + TAIL];
        let mut rs = vec![CAN; SS_STATEBYTES + TAIL];
        seq_reset(0xABCD_0000 ^ round);
        let rcc = unsafe { (cipush)(cs.as_mut_ptr(), ch.as_mut_ptr(), k.as_ptr()) };
        seq_reset(0xABCD_0000 ^ round);
        let rrr = unsafe { (ripush)(rs.as_mut_ptr(), rh.as_mut_ptr(), k.as_ptr()) };
        let ctx = format!("init_push round={}", round);
        common::eqi(&ctx, rcc, rrr);
        assert_eq!(rcc, 0);
        chk(&format!("{} header", ctx), &ch, &rh, SS_HEADERBYTES);
        chk(&format!("{} state", ctx), &cs, &rs, SS_STATEBYTES);

        // the state produced by init_push must equal the one init_pull derives
        // from the same header (cross-check, independent of the RNG)
        let (cs2, _rs2) = ss_init_pull_pair(&s, &ch[..SS_HEADERBYTES], &k, &ctx);
        common::eqb(
            &format!("{} init_push==init_pull", ctx),
            &cs[..SS_STATEBYTES],
            &cs2[..SS_STATEBYTES],
        );

        // and a push/pull round-trip on those states
        let mlen_r = 1 + rng.below(80);
        let m = rng.bytes(mlen_r);
        let (mut co, mut ro) = (cbuf(m.len() + SS_ABYTES), cbuf(m.len() + SS_ABYTES));
        let rcc = unsafe {
            (s.cpush)(
                cs.as_mut_ptr(),
                co.as_mut_ptr(),
                null_mut(),
                m.as_ptr(),
                m.len() as u64,
                null(),
                0,
                0x00,
            )
        };
        let rrr = unsafe {
            (s.rpush)(
                rs.as_mut_ptr(),
                ro.as_mut_ptr(),
                null_mut(),
                m.as_ptr(),
                m.len() as u64,
                null(),
                0,
                0x00,
            )
        };
        common::eqi(&format!("{} push", ctx), rcc, rrr);
        chk(&format!("{} push out", ctx), &co, &ro, m.len() + SS_ABYTES);
        chk(&format!("{} push state", ctx), &cs, &rs, SS_STATEBYTES);
    }
}
