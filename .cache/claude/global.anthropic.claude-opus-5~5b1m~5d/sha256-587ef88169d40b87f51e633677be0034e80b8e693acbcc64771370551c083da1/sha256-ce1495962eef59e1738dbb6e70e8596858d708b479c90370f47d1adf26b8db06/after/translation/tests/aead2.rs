//! Differential tests for AREA `aead2`:
//!
//!   * `crypto_aead/aes256gcm/aead_aes256gcm.c`
//!   * `crypto_aead/aegis128l/{aead_aegis128l.c,aegis128l_soft.c}`
//!   * `crypto_aead/aegis256/{aead_aegis256.c,aegis256_soft.c}`
//!   * `crypto_core/softaes/softaes.c`
//!
//! Build configuration under test: **no `HAVE_*` macros**.  Consequences that
//! the tests below encode (verified with `tools/cpp.sh`):
//!
//!   * `aead_aes256gcm.c` compiles the `#if !((HAVE_ARMCRYPTO && ...) || ...)`
//!     stub branch: `crypto_aead_aes256gcm_is_available()` returns `0` and
//!     *every* other entry point sets `errno = ENOSYS` (38 on Linux) and
//!     returns `-1` without touching any buffer.  There is no working AES-GCM.
//!   * `aegis128l`/`aegis256` only have the `*_soft` implementation, and
//!     `_crypto_aead_*_pick_best_implementation()` unconditionally selects it
//!     and returns 0.
//!   * `softaes.c` compiles the `#else` (non-`FAVOR_PERFORMANCE`) SRM-1R
//!     bitsliced branch of `softaes_block_encrypt`.
//!
//! Everything is called through `dlopen`/`dlsym` on both shared objects.

#![allow(non_camel_case_types)]

#[macro_use]
mod common;

use core::ffi::{c_char, c_int, c_void};
use libloading::Library;
use std::sync::atomic::{AtomicU64, Ordering};

// ---------------------------------------------------------------- symbols ----

/// Look up a symbol whose name is only known at run time (the `getsym!` macro
/// in `common` needs a literal).  Semantics are identical to `getsym!`.
fn sym<T: Copy>(lib: &Library, name: &str) -> T {
    let mut b = name.as_bytes().to_vec();
    b.push(0);
    unsafe {
        let s: libloading::Symbol<T> = lib
            .get(&b)
            .unwrap_or_else(|e| panic!("missing symbol {}: {}", name, e));
        *s
    }
}

extern "C" {
    fn __errno_location() -> *mut c_int;
}
fn errno_get() -> c_int {
    unsafe { *__errno_location() }
}
fn errno_set(v: c_int) {
    unsafe { *__errno_location() = v }
}

const ENOSYS: c_int = 38; // Linux <errno.h>

// ------------------------------------------------------------ signatures -----

type Getter = unsafe extern "C" fn() -> usize;
type Keygen = unsafe extern "C" fn(*mut u8);
type Pick = unsafe extern "C" fn() -> c_int;

type PEnc = unsafe extern "C" fn(
    *mut u8,    // c
    *mut u64,   // clen_p
    *const u8,  // m
    u64,        // mlen
    *const u8,  // ad
    u64,        // adlen
    *const u8,  // nsec
    *const u8,  // npub
    *const u8,  // k
) -> c_int;

type PDec = unsafe extern "C" fn(
    *mut u8,    // m
    *mut u64,   // mlen_p
    *mut u8,    // nsec
    *const u8,  // c
    u64,        // clen
    *const u8,  // ad
    u64,        // adlen
    *const u8,  // npub
    *const u8,  // k
) -> c_int;

type PEncDet = unsafe extern "C" fn(
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

type PDecDet = unsafe extern "C" fn(
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

// implementations.h: the two function pointers inside `*_soft_implementation`
type ImplEncDet = unsafe extern "C" fn(
    *mut u8,   // c
    *mut u8,   // mac
    usize,     // maclen
    *const u8, // m
    usize,     // mlen
    *const u8, // ad
    usize,     // adlen
    *const u8, // npub
    *const u8, // k
) -> c_int;

type ImplDecDet = unsafe extern "C" fn(
    *mut u8,   // m
    *const u8, // c
    usize,     // clen
    *const u8, // mac
    usize,     // maclen
    *const u8, // ad
    usize,     // adlen
    *const u8, // npub
    *const u8, // k
) -> c_int;

#[repr(C)]
#[derive(Copy, Clone)]
struct AegisImplementation {
    encrypt_detached: ImplEncDet,
    decrypt_detached: ImplDecDet,
}

#[allow(dead_code)]
struct AegisApi {
    enc: PEnc,
    dec: PDec,
    encd: PEncDet,
    decd: PDecDet,
    keygen: Keygen,
    keybytes: Getter,
    nsecbytes: Getter,
    npubbytes: Getter,
    abytes: Getter,
    messagebytes_max: Getter,
    pick: Pick,
}

fn load_aegis(lib: &Library, p: &str) -> AegisApi {
    AegisApi {
        enc: sym(lib, &format!("{p}_encrypt")),
        dec: sym(lib, &format!("{p}_decrypt")),
        encd: sym(lib, &format!("{p}_encrypt_detached")),
        decd: sym(lib, &format!("{p}_decrypt_detached")),
        keygen: sym(lib, &format!("{p}_keygen")),
        keybytes: sym(lib, &format!("{p}_keybytes")),
        nsecbytes: sym(lib, &format!("{p}_nsecbytes")),
        npubbytes: sym(lib, &format!("{p}_npubbytes")),
        abytes: sym(lib, &format!("{p}_abytes")),
        messagebytes_max: sym(lib, &format!("{p}_messagebytes_max")),
        pick: sym(lib, &format!("_{p}_pick_best_implementation")),
    }
}

const SIZES: [usize; 15] = [0, 1, 15, 16, 17, 31, 32, 33, 63, 64, 65, 127, 128, 129, 1000];

// ============================================================================
// AES256-GCM  (stub build: is_available() == 0, everything else -> ENOSYS/-1)
// ============================================================================

#[test]
fn aes256gcm_getters_and_state() {
    let (ckb, rkb) = both!("crypto_aead_aes256gcm_keybytes", Getter);
    let (cns, rns) = both!("crypto_aead_aes256gcm_nsecbytes", Getter);
    let (cnp, rnp) = both!("crypto_aead_aes256gcm_npubbytes", Getter);
    let (cab, rab) = both!("crypto_aead_aes256gcm_abytes", Getter);
    let (csb, rsb) = both!("crypto_aead_aes256gcm_statebytes", Getter);
    let (cmm, rmm) = both!("crypto_aead_aes256gcm_messagebytes_max", Getter);
    unsafe {
        assert_eq!(ckb(), rkb(), "aes256gcm keybytes");
        assert_eq!(ckb(), 32);
        assert_eq!(cns(), rns(), "aes256gcm nsecbytes");
        assert_eq!(cns(), 0);
        assert_eq!(cnp(), rnp(), "aes256gcm npubbytes");
        assert_eq!(cnp(), 12);
        assert_eq!(cab(), rab(), "aes256gcm abytes");
        assert_eq!(cab(), 16);
        assert_eq!(cmm(), rmm(), "aes256gcm messagebytes_max");
        // statebytes() == (sizeof(state) + 15) & ~15; the struct is
        // `CRYPTO_ALIGN(16) unsigned char opaque[512]` so it must be 512 and
        // the Rust `#[repr(C, align(16))] { opaque: [u8; 512] }` must agree.
        assert_eq!(csb(), rsb(), "aes256gcm statebytes");
        assert_eq!(csb(), 512, "aes256gcm statebytes value");
    }

    // beforenm() must not write into the state at all (it only sets errno).
    let (cbn, rbn) = both!(
        "crypto_aead_aes256gcm_beforenm",
        unsafe extern "C" fn(*mut u8, *const u8) -> c_int
    );
    let statebytes = unsafe { csb() };
    let key = [0x42u8; 32];
    for (tag, f) in [("C", cbn), ("Rust", rbn)] {
        // over-allocate + align to 16 so the call sees a properly aligned state
        let mut st = vec![0xA5u8; statebytes + 32];
        let off = (16 - (st.as_ptr() as usize % 16)) % 16;
        errno_set(0);
        let rc = unsafe { f(st.as_mut_ptr().add(off), key.as_ptr()) };
        assert_eq!(rc, -1, "{tag} beforenm return");
        assert_eq!(errno_get(), ENOSYS, "{tag} beforenm errno");
        assert!(
            st.iter().all(|&b| b == 0xA5),
            "{tag} beforenm must not touch the state buffer"
        );
    }
}

#[test]
fn aes256gcm_is_available() {
    let (c, r) = both!("crypto_aead_aes256gcm_is_available", unsafe extern "C" fn() -> c_int);
    let (cv, rv) = unsafe { (c(), r()) };
    common::eqi("aes256gcm is_available", cv, rv);
    // With no HAVE_TMMINTRIN_H/HAVE_WMMINTRIN_H/HAVE_ARMCRYPTO the C compiles
    // the stub branch, which returns 0 unconditionally.
    assert_eq!(cv, 0, "aes256gcm is_available must be 0 in this build config");
}

#[test]
fn aes256gcm_all_ops_enosys() {
    // Every one of the nine crypto entry points is the same stub:
    //     errno = ENOSYS; return -1;
    // Check the return value, errno, and that no output buffer is touched.
    let key = [0x11u8; 32];
    let npub = [0x22u8; 12];
    let msg = [0x33u8; 64];
    let ad = [0x44u8; 20];
    let statebytes = 512usize;

    macro_rules! chk {
        ($label:expr, $ty:ty, $call:expr) => {{
            let (cf, rf) = both!($label, $ty);
            for (tag, f) in [("C", cf), ("Rust", rf)] {
                let mut out = vec![0x5Au8; 256];
                let mut mac = vec![0x5Au8; 64];
                let mut lenp: u64 = 0xDEAD_BEEF_DEAD_BEEF;
                let mut st = vec![0x5Au8; statebytes + 32];
                let off = (16 - (st.as_ptr() as usize % 16)) % 16;
                errno_set(4242);
                #[allow(clippy::redundant_closure_call)]
                let rc = $call(f, &mut out, &mut mac, &mut lenp, &mut st, off);
                assert_eq!(rc, -1, "{} {} return", tag, $label);
                assert_eq!(errno_get(), ENOSYS, "{} {} errno", tag, $label);
                assert!(out.iter().all(|&b| b == 0x5A), "{} {} wrote c/m", tag, $label);
                assert!(mac.iter().all(|&b| b == 0x5A), "{} {} wrote mac", tag, $label);
                assert!(st.iter().all(|&b| b == 0x5A), "{} {} wrote state", tag, $label);
                assert_eq!(lenp, 0xDEAD_BEEF_DEAD_BEEF, "{} {} wrote len_p", tag, $label);
            }
        }};
    }

    chk!("crypto_aead_aes256gcm_encrypt", PEnc, |f: PEnc,
                                                 out: &mut Vec<u8>,
                                                 _mac: &mut Vec<u8>,
                                                 lenp: &mut u64,
                                                 _st: &mut Vec<u8>,
                                                 _off: usize| unsafe {
        f(
            out.as_mut_ptr(),
            lenp,
            msg.as_ptr(),
            msg.len() as u64,
            ad.as_ptr(),
            ad.len() as u64,
            core::ptr::null(),
            npub.as_ptr(),
            key.as_ptr(),
        )
    });

    chk!("crypto_aead_aes256gcm_decrypt", PDec, |f: PDec,
                                                out: &mut Vec<u8>,
                                                _mac: &mut Vec<u8>,
                                                lenp: &mut u64,
                                                _st: &mut Vec<u8>,
                                                _off: usize| unsafe {
        f(
            out.as_mut_ptr(),
            lenp,
            core::ptr::null_mut(),
            msg.as_ptr(),
            msg.len() as u64,
            ad.as_ptr(),
            ad.len() as u64,
            npub.as_ptr(),
            key.as_ptr(),
        )
    });

    chk!("crypto_aead_aes256gcm_encrypt_detached", PEncDet, |f: PEncDet,
                                                            out: &mut Vec<u8>,
                                                            mac: &mut Vec<u8>,
                                                            lenp: &mut u64,
                                                            _st: &mut Vec<u8>,
                                                            _off: usize| unsafe {
        f(
            out.as_mut_ptr(),
            mac.as_mut_ptr(),
            lenp,
            msg.as_ptr(),
            msg.len() as u64,
            ad.as_ptr(),
            ad.len() as u64,
            core::ptr::null(),
            npub.as_ptr(),
            key.as_ptr(),
        )
    });

    chk!("crypto_aead_aes256gcm_decrypt_detached", PDecDet, |f: PDecDet,
                                                            out: &mut Vec<u8>,
                                                            mac: &mut Vec<u8>,
                                                            _lenp: &mut u64,
                                                            _st: &mut Vec<u8>,
                                                            _off: usize| unsafe {
        f(
            out.as_mut_ptr(),
            core::ptr::null_mut(),
            msg.as_ptr(),
            msg.len() as u64,
            mac.as_ptr(),
            ad.as_ptr(),
            ad.len() as u64,
            npub.as_ptr(),
            key.as_ptr(),
        )
    });

    type ABeforenm = unsafe extern "C" fn(*mut u8, *const u8) -> c_int;
    chk!("crypto_aead_aes256gcm_beforenm", ABeforenm, |f: ABeforenm,
                                                      _out: &mut Vec<u8>,
                                                      _mac: &mut Vec<u8>,
                                                      _lenp: &mut u64,
                                                      st: &mut Vec<u8>,
                                                      off: usize| unsafe {
        f(st.as_mut_ptr().add(off), key.as_ptr())
    });

    type AEncAfter = unsafe extern "C" fn(
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
    chk!("crypto_aead_aes256gcm_encrypt_afternm", AEncAfter, |f: AEncAfter,
                                                             out: &mut Vec<u8>,
                                                             _mac: &mut Vec<u8>,
                                                             lenp: &mut u64,
                                                             st: &mut Vec<u8>,
                                                             off: usize| unsafe {
        f(
            out.as_mut_ptr(),
            lenp,
            msg.as_ptr(),
            msg.len() as u64,
            ad.as_ptr(),
            ad.len() as u64,
            core::ptr::null(),
            npub.as_ptr(),
            st.as_ptr().add(off),
        )
    });

    type ADecAfter = unsafe extern "C" fn(
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
    chk!("crypto_aead_aes256gcm_decrypt_afternm", ADecAfter, |f: ADecAfter,
                                                             out: &mut Vec<u8>,
                                                             _mac: &mut Vec<u8>,
                                                             lenp: &mut u64,
                                                             st: &mut Vec<u8>,
                                                             off: usize| unsafe {
        f(
            out.as_mut_ptr(),
            lenp,
            core::ptr::null_mut(),
            msg.as_ptr(),
            msg.len() as u64,
            ad.as_ptr(),
            ad.len() as u64,
            npub.as_ptr(),
            st.as_ptr().add(off),
        )
    });

    type AEncDetAfter = unsafe extern "C" fn(
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
    chk!(
        "crypto_aead_aes256gcm_encrypt_detached_afternm",
        AEncDetAfter,
        |f: AEncDetAfter,
         out: &mut Vec<u8>,
         mac: &mut Vec<u8>,
         lenp: &mut u64,
         st: &mut Vec<u8>,
         off: usize| unsafe {
            f(
                out.as_mut_ptr(),
                mac.as_mut_ptr(),
                lenp,
                msg.as_ptr(),
                msg.len() as u64,
                ad.as_ptr(),
                ad.len() as u64,
                core::ptr::null(),
                npub.as_ptr(),
                st.as_ptr().add(off),
            )
        }
    );

    type ADecDetAfter = unsafe extern "C" fn(
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
    chk!(
        "crypto_aead_aes256gcm_decrypt_detached_afternm",
        ADecDetAfter,
        |f: ADecDetAfter,
         out: &mut Vec<u8>,
         mac: &mut Vec<u8>,
         _lenp: &mut u64,
         st: &mut Vec<u8>,
         off: usize| unsafe {
            f(
                out.as_mut_ptr(),
                core::ptr::null_mut(),
                msg.as_ptr(),
                msg.len() as u64,
                mac.as_ptr(),
                ad.as_ptr(),
                ad.len() as u64,
                npub.as_ptr(),
                st.as_ptr().add(off),
            )
        }
    );
}

#[test]
fn aes256gcm_enosys_regardless_of_lengths() {
    // The stubs ignore every argument, including out-of-range lengths and NULL
    // optional pointers.  Sweep a handful of shapes on both libraries.
    let (ce, re) = both!("crypto_aead_aes256gcm_encrypt", PEnc);
    let (cd, rd) = both!("crypto_aead_aes256gcm_decrypt", PDec);
    let key = [7u8; 32];
    let npub = [8u8; 12];
    let mut buf = vec![0u8; 512];
    for &mlen in &[0u64, 1, 16, 17, u64::MAX, (1u64 << 61)] {
        for (tag, e, d) in [("C", ce, cd), ("Rust", re, rd)] {
            errno_set(0);
            let rc = unsafe {
                e(
                    buf.as_mut_ptr(),
                    core::ptr::null_mut(), // clen_p == NULL is tolerated
                    core::ptr::null(),     // m == NULL is tolerated
                    mlen,
                    core::ptr::null(), // ad == NULL
                    0,
                    core::ptr::null(),
                    npub.as_ptr(),
                    key.as_ptr(),
                )
            };
            assert_eq!(rc, -1, "{tag} encrypt mlen={mlen}");
            assert_eq!(errno_get(), ENOSYS, "{tag} encrypt errno mlen={mlen}");
            errno_set(0);
            let rc = unsafe {
                d(
                    core::ptr::null_mut(),
                    core::ptr::null_mut(),
                    core::ptr::null_mut(),
                    buf.as_ptr(),
                    mlen,
                    core::ptr::null(),
                    0,
                    npub.as_ptr(),
                    key.as_ptr(),
                )
            };
            assert_eq!(rc, -1, "{tag} decrypt clen={mlen}");
            assert_eq!(errno_get(), ENOSYS, "{tag} decrypt errno clen={mlen}");
        }
    }
}

// ============================================================================
// AEGIS-128L / AEGIS-256 public API
// ============================================================================

/// Run the whole differential suite for one AEGIS variant.
fn aegis_suite(name: &str, prefix: &str, keybytes: usize, npubbytes: usize) {
    let l = common::libs();
    let c = load_aegis(&l.c, prefix);
    let r = load_aegis(&l.r, prefix);
    let ab = 32usize; // ABYTES for both variants

    // ------------------------------------------------------------ getters ---
    unsafe {
        assert_eq!((c.keybytes)(), (r.keybytes)(), "{name} keybytes");
        assert_eq!((c.keybytes)(), keybytes, "{name} keybytes value");
        assert_eq!((c.nsecbytes)(), (r.nsecbytes)(), "{name} nsecbytes");
        assert_eq!((c.nsecbytes)(), 0, "{name} nsecbytes value");
        assert_eq!((c.npubbytes)(), (r.npubbytes)(), "{name} npubbytes");
        assert_eq!((c.npubbytes)(), npubbytes, "{name} npubbytes value");
        assert_eq!((c.abytes)(), (r.abytes)(), "{name} abytes");
        assert_eq!((c.abytes)(), ab, "{name} abytes value");
        assert_eq!(
            (c.messagebytes_max)(),
            (r.messagebytes_max)(),
            "{name} messagebytes_max"
        );
        assert_eq!(
            (c.messagebytes_max)(),
            (1usize << 61) - 1,
            "{name} messagebytes_max value"
        );
        // pick_best_implementation always selects the soft impl and returns 0.
        assert_eq!((c.pick)(), (r.pick)(), "{name} pick_best_implementation");
        assert_eq!((c.pick)(), 0, "{name} pick_best_implementation value");
    }

    let mut rng = common::Rng::new(0xAE61_5000 ^ (keybytes as u64) << 32 ^ npubbytes as u64);

    // ------------------------------------- one-shot / detached size matrix ---
    for &mlen in SIZES.iter() {
        for &adlen in SIZES.iter() {
            for trial in 0..3 {
                let k = rng.bytes(keybytes);
                let npub = rng.bytes(npubbytes);
                let m = rng.bytes(mlen);
                // one extra byte so the deliberate `adlen + 1` case below stays
                // inside the allocation
                let ad_full = rng.bytes(adlen + 1);
                let ad = &ad_full[..adlen];
                let ctx = format!("{name} mlen={mlen} adlen={adlen} t={trial}");

                // --- crypto_aead_*_encrypt ---------------------------------
                let mut cc = vec![0xCCu8; mlen + ab + 8]; // canary tail
                let mut rc_buf = vec![0xCCu8; mlen + ab + 8];
                let mut clen_c: u64 = 0xFFFF_FFFF_FFFF_FFFF;
                let mut clen_r: u64 = 0xFFFF_FFFF_FFFF_FFFF;
                let (adp, adp2) = if adlen == 0 {
                    (core::ptr::null(), core::ptr::null())
                } else {
                    (ad.as_ptr(), ad.as_ptr())
                };
                let mp = if mlen == 0 { core::ptr::null() } else { m.as_ptr() };
                let rcc = unsafe {
                    (c.enc)(
                        cc.as_mut_ptr(),
                        &mut clen_c,
                        mp,
                        mlen as u64,
                        adp,
                        adlen as u64,
                        core::ptr::null(),
                        npub.as_ptr(),
                        k.as_ptr(),
                    )
                };
                let rcr = unsafe {
                    (r.enc)(
                        rc_buf.as_mut_ptr(),
                        &mut clen_r,
                        mp,
                        mlen as u64,
                        adp2,
                        adlen as u64,
                        core::ptr::null(),
                        npub.as_ptr(),
                        k.as_ptr(),
                    )
                };
                common::eqi(&format!("{ctx} encrypt rc"), rcc, rcr);
                assert_eq!(rcc, 0, "{ctx} encrypt must succeed");
                assert_eq!(clen_c, clen_r, "{ctx} clen");
                assert_eq!(clen_c, (mlen + ab) as u64, "{ctx} clen value");
                common::eqb(&format!("{ctx} ciphertext"), &cc, &rc_buf);
                assert!(
                    cc[mlen + ab..].iter().all(|&b| b == 0xCC),
                    "{ctx} encrypt overran output"
                );

                // --- crypto_aead_*_encrypt with clen_p == NULL and a
                //     non-NULL (but always ignored) nsec -------------------
                let nsec_in = [0x5Eu8; 8];
                let mut cc2 = vec![0xCCu8; mlen + ab + 8];
                let mut rc2 = vec![0xCCu8; mlen + ab + 8];
                let a = unsafe {
                    (c.enc)(
                        cc2.as_mut_ptr(),
                        core::ptr::null_mut(),
                        mp,
                        mlen as u64,
                        adp,
                        adlen as u64,
                        nsec_in.as_ptr(),
                        npub.as_ptr(),
                        k.as_ptr(),
                    )
                };
                let b = unsafe {
                    (r.enc)(
                        rc2.as_mut_ptr(),
                        core::ptr::null_mut(),
                        mp,
                        mlen as u64,
                        adp,
                        adlen as u64,
                        nsec_in.as_ptr(),
                        npub.as_ptr(),
                        k.as_ptr(),
                    )
                };
                assert_eq!(nsec_in, [0x5Eu8; 8], "{ctx} nsec must be ignored by encrypt");
                common::eqi(&format!("{ctx} encrypt(clen_p=NULL) rc"), a, b);
                common::eqb(&format!("{ctx} encrypt(clen_p=NULL) out"), &cc2, &rc2);
                common::eqb(&format!("{ctx} encrypt(clen_p=NULL) == encrypt"), &cc, &cc2);

                // --- crypto_aead_*_encrypt_detached ------------------------
                let mut ec = vec![0xC3u8; mlen + 8];
                let mut er = vec![0xC3u8; mlen + 8];
                let mut mac_c = vec![0x3Cu8; ab + 8];
                let mut mac_r = vec![0x3Cu8; ab + 8];
                let mut maclen_c: u64 = 0;
                let mut maclen_r: u64 = 0;
                let a = unsafe {
                    (c.encd)(
                        ec.as_mut_ptr(),
                        mac_c.as_mut_ptr(),
                        &mut maclen_c,
                        mp,
                        mlen as u64,
                        adp,
                        adlen as u64,
                        core::ptr::null(),
                        npub.as_ptr(),
                        k.as_ptr(),
                    )
                };
                let b = unsafe {
                    (r.encd)(
                        er.as_mut_ptr(),
                        mac_r.as_mut_ptr(),
                        &mut maclen_r,
                        mp,
                        mlen as u64,
                        adp,
                        adlen as u64,
                        core::ptr::null(),
                        npub.as_ptr(),
                        k.as_ptr(),
                    )
                };
                common::eqi(&format!("{ctx} encrypt_detached rc"), a, b);
                assert_eq!(maclen_c, maclen_r, "{ctx} maclen_p");
                assert_eq!(maclen_c, ab as u64, "{ctx} maclen_p value");
                common::eqb(&format!("{ctx} detached c"), &ec, &er);
                common::eqb(&format!("{ctx} detached mac"), &mac_c, &mac_r);
                // detached output must agree with the combined form
                assert_eq!(&ec[..mlen], &cc[..mlen], "{ctx} detached vs combined c");
                assert_eq!(&mac_c[..ab], &cc[mlen..mlen + ab], "{ctx} detached vs combined mac");

                // --- encrypt_detached with maclen_p == NULL ----------------
                let mut e2 = vec![0u8; mlen];
                let mut m2 = vec![0u8; ab];
                let a = unsafe {
                    (c.encd)(
                        e2.as_mut_ptr(),
                        m2.as_mut_ptr(),
                        core::ptr::null_mut(),
                        mp,
                        mlen as u64,
                        adp,
                        adlen as u64,
                        core::ptr::null(),
                        npub.as_ptr(),
                        k.as_ptr(),
                    )
                };
                let mut e3 = vec![0u8; mlen];
                let mut m3 = vec![0u8; ab];
                let b = unsafe {
                    (r.encd)(
                        e3.as_mut_ptr(),
                        m3.as_mut_ptr(),
                        core::ptr::null_mut(),
                        mp,
                        mlen as u64,
                        adp,
                        adlen as u64,
                        core::ptr::null(),
                        npub.as_ptr(),
                        k.as_ptr(),
                    )
                };
                common::eqi(&format!("{ctx} encrypt_detached(maclen_p=NULL) rc"), a, b);
                common::eqb(&format!("{ctx} encrypt_detached(maclen_p=NULL) c"), &e2, &e3);
                common::eqb(&format!("{ctx} encrypt_detached(maclen_p=NULL) mac"), &m2, &m3);

                // --- crypto_aead_*_decrypt (valid) -------------------------
                for (dtag, dm_null) in [("m", false), ("m=NULL", true)] {
                    let mut dc = vec![0x77u8; mlen + 8];
                    let mut dr = vec![0x77u8; mlen + 8];
                    let mut mlen_c: u64 = 0xAAAA;
                    let mut mlen_r: u64 = 0xAAAA;
                    let (pc, pr) = if dm_null {
                        (core::ptr::null_mut(), core::ptr::null_mut())
                    } else {
                        (dc.as_mut_ptr(), dr.as_mut_ptr())
                    };
                    let a = unsafe {
                        (c.dec)(
                            pc,
                            &mut mlen_c,
                            core::ptr::null_mut(),
                            cc.as_ptr(),
                            (mlen + ab) as u64,
                            adp,
                            adlen as u64,
                            npub.as_ptr(),
                            k.as_ptr(),
                        )
                    };
                    let b = unsafe {
                        (r.dec)(
                            pr,
                            &mut mlen_r,
                            core::ptr::null_mut(),
                            cc.as_ptr(),
                            (mlen + ab) as u64,
                            adp,
                            adlen as u64,
                            npub.as_ptr(),
                            k.as_ptr(),
                        )
                    };
                    common::eqi(&format!("{ctx} decrypt[{dtag}] rc"), a, b);
                    assert_eq!(a, 0, "{ctx} decrypt[{dtag}] must succeed");
                    assert_eq!(mlen_c, mlen_r, "{ctx} decrypt[{dtag}] mlen_p");
                    assert_eq!(mlen_c, mlen as u64, "{ctx} decrypt[{dtag}] mlen_p value");
                    common::eqb(&format!("{ctx} decrypt[{dtag}] out"), &dc, &dr);
                    if !dm_null {
                        assert_eq!(&dc[..mlen], &m[..], "{ctx} decrypt[{dtag}] plaintext");
                        assert!(dc[mlen..].iter().all(|&b| b == 0x77), "{ctx} decrypt overran");
                    }
                }

                // --- decrypt with mlen_p == NULL ---------------------------
                let mut dc = vec![0u8; mlen];
                let mut dr = vec![0u8; mlen];
                let a = unsafe {
                    (c.dec)(
                        dc.as_mut_ptr(),
                        core::ptr::null_mut(),
                        core::ptr::null_mut(),
                        cc.as_ptr(),
                        (mlen + ab) as u64,
                        adp,
                        adlen as u64,
                        npub.as_ptr(),
                        k.as_ptr(),
                    )
                };
                let b = unsafe {
                    (r.dec)(
                        dr.as_mut_ptr(),
                        core::ptr::null_mut(),
                        core::ptr::null_mut(),
                        cc.as_ptr(),
                        (mlen + ab) as u64,
                        adp,
                        adlen as u64,
                        npub.as_ptr(),
                        k.as_ptr(),
                    )
                };
                common::eqi(&format!("{ctx} decrypt(mlen_p=NULL) rc"), a, b);
                common::eqb(&format!("{ctx} decrypt(mlen_p=NULL) out"), &dc, &dr);

                // --- decrypt_detached (valid, and with nsec non-NULL) ------
                let mut nsec = [0x99u8; 8];
                let mut dc = vec![0x11u8; mlen + 8];
                let mut dr = vec![0x11u8; mlen + 8];
                let a = unsafe {
                    (c.decd)(
                        dc.as_mut_ptr(),
                        nsec.as_mut_ptr(),
                        ec.as_ptr(),
                        mlen as u64,
                        mac_c.as_ptr(),
                        adp,
                        adlen as u64,
                        npub.as_ptr(),
                        k.as_ptr(),
                    )
                };
                let b = unsafe {
                    (r.decd)(
                        dr.as_mut_ptr(),
                        nsec.as_mut_ptr(),
                        ec.as_ptr(),
                        mlen as u64,
                        mac_c.as_ptr(),
                        adp,
                        adlen as u64,
                        npub.as_ptr(),
                        k.as_ptr(),
                    )
                };
                common::eqi(&format!("{ctx} decrypt_detached rc"), a, b);
                assert_eq!(a, 0, "{ctx} decrypt_detached must succeed");
                common::eqb(&format!("{ctx} decrypt_detached out"), &dc, &dr);
                assert_eq!(&dc[..mlen], &m[..], "{ctx} decrypt_detached plaintext");
                assert_eq!(nsec, [0x99u8; 8], "{ctx} nsec must be ignored");

                // --- decrypt_detached with m == NULL ----------------------
                let a = unsafe {
                    (c.decd)(
                        core::ptr::null_mut(),
                        core::ptr::null_mut(),
                        ec.as_ptr(),
                        mlen as u64,
                        mac_c.as_ptr(),
                        adp,
                        adlen as u64,
                        npub.as_ptr(),
                        k.as_ptr(),
                    )
                };
                let b = unsafe {
                    (r.decd)(
                        core::ptr::null_mut(),
                        core::ptr::null_mut(),
                        ec.as_ptr(),
                        mlen as u64,
                        mac_c.as_ptr(),
                        adp,
                        adlen as u64,
                        npub.as_ptr(),
                        k.as_ptr(),
                    )
                };
                common::eqi(&format!("{ctx} decrypt_detached(m=NULL) rc"), a, b);
                assert_eq!(a, 0, "{ctx} decrypt_detached(m=NULL) must succeed");

                // --- in-place encrypt then in-place decrypt ---------------
                let mut ip_c = vec![0xEEu8; mlen + ab + 8];
                ip_c[..mlen].copy_from_slice(&m);
                let mut ip_r = ip_c.clone();
                let a = unsafe {
                    (c.enc)(
                        ip_c.as_mut_ptr(),
                        core::ptr::null_mut(),
                        ip_c.as_ptr(),
                        mlen as u64,
                        adp,
                        adlen as u64,
                        core::ptr::null(),
                        npub.as_ptr(),
                        k.as_ptr(),
                    )
                };
                let b = unsafe {
                    (r.enc)(
                        ip_r.as_mut_ptr(),
                        core::ptr::null_mut(),
                        ip_r.as_ptr(),
                        mlen as u64,
                        adp,
                        adlen as u64,
                        core::ptr::null(),
                        npub.as_ptr(),
                        k.as_ptr(),
                    )
                };
                common::eqi(&format!("{ctx} in-place encrypt rc"), a, b);
                common::eqb(&format!("{ctx} in-place encrypt out"), &ip_c, &ip_r);
                assert_eq!(&ip_c[..mlen + ab], &cc[..mlen + ab], "{ctx} in-place == out-of-place");
                let a = unsafe {
                    (c.dec)(
                        ip_c.as_mut_ptr(),
                        core::ptr::null_mut(),
                        core::ptr::null_mut(),
                        ip_c.as_ptr(),
                        (mlen + ab) as u64,
                        adp,
                        adlen as u64,
                        npub.as_ptr(),
                        k.as_ptr(),
                    )
                };
                let b = unsafe {
                    (r.dec)(
                        ip_r.as_mut_ptr(),
                        core::ptr::null_mut(),
                        core::ptr::null_mut(),
                        ip_r.as_ptr(),
                        (mlen + ab) as u64,
                        adp,
                        adlen as u64,
                        npub.as_ptr(),
                        k.as_ptr(),
                    )
                };
                common::eqi(&format!("{ctx} in-place decrypt rc"), a, b);
                assert_eq!(a, 0, "{ctx} in-place decrypt must succeed");
                common::eqb(&format!("{ctx} in-place decrypt out"), &ip_c, &ip_r);
                assert_eq!(&ip_c[..mlen], &m[..], "{ctx} in-place decrypt plaintext");

                // --- wrong key / wrong nonce / wrong ad -------------------
                let mut k2 = k.clone();
                k2[0] ^= 1;
                let mut n2 = npub.clone();
                n2[0] ^= 1;
                for (wtag, kk, nn, aa, aal) in [
                    ("wrong-key", k2.as_ptr(), npub.as_ptr(), adp, adlen as u64),
                    ("wrong-nonce", k.as_ptr(), n2.as_ptr(), adp, adlen as u64),
                    ("wrong-adlen", k.as_ptr(), npub.as_ptr(), adp, adlen as u64 + 1),
                ] {
                    if wtag == "wrong-adlen" && adlen == 0 {
                        continue; // would read ad[0] from a NULL pointer
                    }
                    let mut dc = vec![0x22u8; mlen + 8];
                    let mut dr = vec![0x22u8; mlen + 8];
                    let a = unsafe {
                        (c.dec)(
                            dc.as_mut_ptr(),
                            core::ptr::null_mut(),
                            core::ptr::null_mut(),
                            cc.as_ptr(),
                            (mlen + ab) as u64,
                            aa,
                            aal,
                            nn,
                            kk,
                        )
                    };
                    let b = unsafe {
                        (r.dec)(
                            dr.as_mut_ptr(),
                            core::ptr::null_mut(),
                            core::ptr::null_mut(),
                            cc.as_ptr(),
                            (mlen + ab) as u64,
                            aa,
                            aal,
                            nn,
                            kk,
                        )
                    };
                    common::eqi(&format!("{ctx} {wtag} rc"), a, b);
                    assert_eq!(a, -1, "{ctx} {wtag} must fail");
                    common::eqb(&format!("{ctx} {wtag} out"), &dc, &dr);
                    assert!(dc[..mlen].iter().all(|&x| x == 0), "{ctx} {wtag} must zero m");
                    assert!(dc[mlen..].iter().all(|&x| x == 0x22), "{ctx} {wtag} overran");
                }
            }
        }
    }
}

/// Tag/ciphertext tampering, truncated ciphertexts and oversized lengths.
fn aegis_errors(name: &str, prefix: &str, keybytes: usize, npubbytes: usize) {
    let l = common::libs();
    let c = load_aegis(&l.c, prefix);
    let r = load_aegis(&l.r, prefix);
    let ab = 32usize;
    let mut rng = common::Rng::new(0x1234_5678 ^ keybytes as u64);

    for &mlen in &[0usize, 1, 15, 16, 17, 31, 32, 33, 64, 65, 1000] {
        let k = rng.bytes(keybytes);
        let npub = rng.bytes(npubbytes);
        let m = rng.bytes(mlen);
        let ad = rng.bytes(21);
        let mut ct = vec![0u8; mlen + ab];
        let rcc = unsafe {
            (c.enc)(
                ct.as_mut_ptr(),
                core::ptr::null_mut(),
                m.as_ptr(),
                mlen as u64,
                ad.as_ptr(),
                ad.len() as u64,
                core::ptr::null(),
                npub.as_ptr(),
                k.as_ptr(),
            )
        };
        assert_eq!(rcc, 0);

        // -- flip every bit position of every tag byte --------------------
        for byte in 0..ab {
            for bit in [0u8, 3, 7] {
                let mut bad = ct.clone();
                bad[mlen + byte] ^= 1 << bit;
                let mut dc = vec![0x5Bu8; mlen + 4];
                let mut dr = vec![0x5Bu8; mlen + 4];
                let mut lc: u64 = 0xBEEF;
                let mut lr: u64 = 0xBEEF;
                let a = unsafe {
                    (c.dec)(
                        dc.as_mut_ptr(),
                        &mut lc,
                        core::ptr::null_mut(),
                        bad.as_ptr(),
                        (mlen + ab) as u64,
                        ad.as_ptr(),
                        ad.len() as u64,
                        npub.as_ptr(),
                        k.as_ptr(),
                    )
                };
                let b = unsafe {
                    (r.dec)(
                        dr.as_mut_ptr(),
                        &mut lr,
                        core::ptr::null_mut(),
                        bad.as_ptr(),
                        (mlen + ab) as u64,
                        ad.as_ptr(),
                        ad.len() as u64,
                        npub.as_ptr(),
                        k.as_ptr(),
                    )
                };
                let ctx = format!("{name} tag-tamper mlen={mlen} byte={byte} bit={bit}");
                common::eqi(&format!("{ctx} rc"), a, b);
                assert_eq!(a, -1, "{ctx} must fail");
                assert_eq!(lc, lr, "{ctx} mlen_p");
                assert_eq!(lc, 0, "{ctx} mlen_p must be 0 on failure");
                common::eqb(&format!("{ctx} out"), &dc, &dr);
                assert!(dc[..mlen].iter().all(|&x| x == 0), "{ctx} must zero m");
                assert!(dc[mlen..].iter().all(|&x| x == 0x5B), "{ctx} overran");
            }
        }

        // -- flip every ciphertext byte -----------------------------------
        for byte in 0..mlen.min(80) {
            let mut bad = ct.clone();
            bad[byte] ^= 0x80;
            let mut dc = vec![0x5Bu8; mlen];
            let mut dr = vec![0x5Bu8; mlen];
            let a = unsafe {
                (c.dec)(
                    dc.as_mut_ptr(),
                    core::ptr::null_mut(),
                    core::ptr::null_mut(),
                    bad.as_ptr(),
                    (mlen + ab) as u64,
                    ad.as_ptr(),
                    ad.len() as u64,
                    npub.as_ptr(),
                    k.as_ptr(),
                )
            };
            let b = unsafe {
                (r.dec)(
                    dr.as_mut_ptr(),
                    core::ptr::null_mut(),
                    core::ptr::null_mut(),
                    bad.as_ptr(),
                    (mlen + ab) as u64,
                    ad.as_ptr(),
                    ad.len() as u64,
                    npub.as_ptr(),
                    k.as_ptr(),
                )
            };
            let ctx = format!("{name} ct-tamper mlen={mlen} byte={byte}");
            common::eqi(&format!("{ctx} rc"), a, b);
            assert_eq!(a, -1, "{ctx} must fail");
            common::eqb(&format!("{ctx} out"), &dc, &dr);
            assert!(dc.iter().all(|&x| x == 0), "{ctx} must zero m");
        }

        // -- tamper the detached mac, via decrypt_detached -----------------
        let mut mac = vec![0u8; ab];
        let mut ctd = vec![0u8; mlen];
        unsafe {
            (c.encd)(
                ctd.as_mut_ptr(),
                mac.as_mut_ptr(),
                core::ptr::null_mut(),
                m.as_ptr(),
                mlen as u64,
                ad.as_ptr(),
                ad.len() as u64,
                core::ptr::null(),
                npub.as_ptr(),
                k.as_ptr(),
            );
        }
        for byte in 0..ab {
            let mut bad = mac.clone();
            bad[byte] ^= 0x40;
            let mut dc = vec![0x6Cu8; mlen];
            let mut dr = vec![0x6Cu8; mlen];
            let a = unsafe {
                (c.decd)(
                    dc.as_mut_ptr(),
                    core::ptr::null_mut(),
                    ctd.as_ptr(),
                    mlen as u64,
                    bad.as_ptr(),
                    ad.as_ptr(),
                    ad.len() as u64,
                    npub.as_ptr(),
                    k.as_ptr(),
                )
            };
            let b = unsafe {
                (r.decd)(
                    dr.as_mut_ptr(),
                    core::ptr::null_mut(),
                    ctd.as_ptr(),
                    mlen as u64,
                    bad.as_ptr(),
                    ad.as_ptr(),
                    ad.len() as u64,
                    npub.as_ptr(),
                    k.as_ptr(),
                )
            };
            let ctx = format!("{name} detached-mac-tamper mlen={mlen} byte={byte}");
            common::eqi(&format!("{ctx} rc"), a, b);
            assert_eq!(a, -1, "{ctx} must fail");
            common::eqb(&format!("{ctx} out"), &dc, &dr);
            assert!(dc.iter().all(|&x| x == 0), "{ctx} must zero m");
        }
        // detached decrypt with m == NULL and a bad mac: nothing to zero
        let mut bad = mac.clone();
        bad[0] ^= 1;
        let a = unsafe {
            (c.decd)(
                core::ptr::null_mut(),
                core::ptr::null_mut(),
                ctd.as_ptr(),
                mlen as u64,
                bad.as_ptr(),
                ad.as_ptr(),
                ad.len() as u64,
                npub.as_ptr(),
                k.as_ptr(),
            )
        };
        let b = unsafe {
            (r.decd)(
                core::ptr::null_mut(),
                core::ptr::null_mut(),
                ctd.as_ptr(),
                mlen as u64,
                bad.as_ptr(),
                ad.as_ptr(),
                ad.len() as u64,
                npub.as_ptr(),
                k.as_ptr(),
            )
        };
        common::eqi(&format!("{name} detached bad-mac m=NULL mlen={mlen}"), a, b);
        assert_eq!(a, -1);
    }

    // -- clen < ABYTES for every clen: decrypt returns -1, *mlen_p = 0 ------
    let k = rng.bytes(keybytes);
    let npub = rng.bytes(npubbytes);
    let cbuf = [0x5Au8; 64];
    for clen in 0..ab as u64 {
        for withp in [false, true] {
            let mut lc: u64 = 0x1234;
            let mut lr: u64 = 0x1234;
            let mut dc = [0x9Fu8; 64];
            let mut dr = [0x9Fu8; 64];
            let (pc, pr): (*mut u64, *mut u64) = if withp {
                (&mut lc, &mut lr)
            } else {
                (core::ptr::null_mut(), core::ptr::null_mut())
            };
            let a = unsafe {
                (c.dec)(
                    dc.as_mut_ptr(),
                    pc,
                    core::ptr::null_mut(),
                    cbuf.as_ptr(),
                    clen,
                    core::ptr::null(),
                    0,
                    npub.as_ptr(),
                    k.as_ptr(),
                )
            };
            let b = unsafe {
                (r.dec)(
                    dr.as_mut_ptr(),
                    pr,
                    core::ptr::null_mut(),
                    cbuf.as_ptr(),
                    clen,
                    core::ptr::null(),
                    0,
                    npub.as_ptr(),
                    k.as_ptr(),
                )
            };
            let ctx = format!("{name} short clen={clen} mlen_p={withp}");
            common::eqi(&format!("{ctx} rc"), a, b);
            assert_eq!(a, -1, "{ctx} must fail");
            assert_eq!(lc, lr, "{ctx} mlen_p");
            if withp {
                assert_eq!(lc, 0, "{ctx} mlen_p must be 0");
            } else {
                assert_eq!(lc, 0x1234, "{ctx} mlen_p untouched");
            }
            // the implementation is never reached, so m stays untouched
            common::eqb(&format!("{ctx} out"), &dc, &dr);
            assert!(dc.iter().all(|&x| x == 0x9F), "{ctx} m must be untouched");
        }
    }

    // -- clen / adlen > MESSAGEBYTES_MAX in decrypt_detached -> -1 ----------
    let maxlen = (1u64 << 61) - 1;
    for (ctag, clen, adlen) in [
        ("clen=MAX+1", maxlen + 1, 0u64),
        ("clen=u64::MAX", u64::MAX, 0),
        ("adlen=MAX+1", 0, maxlen + 1),
        ("adlen=u64::MAX", 0, u64::MAX),
        ("both", maxlen + 1, maxlen + 1),
    ] {
        let mut dc = [0x3Eu8; 32];
        let mut dr = [0x3Eu8; 32];
        let a = unsafe {
            (c.decd)(
                dc.as_mut_ptr(),
                core::ptr::null_mut(),
                cbuf.as_ptr(),
                clen,
                cbuf.as_ptr(),
                cbuf.as_ptr(),
                adlen,
                npub.as_ptr(),
                k.as_ptr(),
            )
        };
        let b = unsafe {
            (r.decd)(
                dr.as_mut_ptr(),
                core::ptr::null_mut(),
                cbuf.as_ptr(),
                clen,
                cbuf.as_ptr(),
                cbuf.as_ptr(),
                adlen,
                npub.as_ptr(),
                k.as_ptr(),
            )
        };
        let ctx = format!("{name} decrypt_detached oversize {ctag}");
        common::eqi(&format!("{ctx} rc"), a, b);
        assert_eq!(a, -1, "{ctx} must return -1");
        common::eqb(&format!("{ctx} out"), &dc, &dr);
        assert!(dc.iter().all(|&x| x == 0x3E), "{ctx} must not touch m");
    }

    // -- decrypt() with clen so large that clen-ABYTES > MESSAGEBYTES_MAX --
    // (decrypt forwards to decrypt_detached, which rejects with -1 before
    //  dereferencing anything)
    for (ctag, clen) in [
        ("clen=u64::MAX", u64::MAX),
        ("clen=MAX+1+ABYTES", maxlen + 1 + ab as u64),
    ] {
        let mut lc: u64 = 0x77;
        let mut lr: u64 = 0x77;
        let a = unsafe {
            (c.dec)(
                core::ptr::null_mut(),
                &mut lc,
                core::ptr::null_mut(),
                cbuf.as_ptr(),
                clen,
                core::ptr::null(),
                0,
                npub.as_ptr(),
                k.as_ptr(),
            )
        };
        let b = unsafe {
            (r.dec)(
                core::ptr::null_mut(),
                &mut lr,
                core::ptr::null_mut(),
                cbuf.as_ptr(),
                clen,
                core::ptr::null(),
                0,
                npub.as_ptr(),
                k.as_ptr(),
            )
        };
        let ctx = format!("{name} decrypt oversize {ctag}");
        common::eqi(&format!("{ctx} rc"), a, b);
        assert_eq!(a, -1, "{ctx} must return -1");
        assert_eq!(lc, lr, "{ctx} mlen_p");
        assert_eq!(lc, 0, "{ctx} mlen_p must be 0");
    }
}

#[test]
fn aegis128l_public_api() {
    aegis_suite("aegis128l", "crypto_aead_aegis128l", 16, 16);
}

#[test]
fn aegis256_public_api() {
    aegis_suite("aegis256", "crypto_aead_aegis256", 32, 32);
}

#[test]
fn aegis128l_error_surface() {
    aegis_errors("aegis128l", "crypto_aead_aegis128l", 16, 16);
}

#[test]
fn aegis256_error_surface() {
    aegis_errors("aegis256", "crypto_aead_aegis256", 32, 32);
}

// ============================================================================
// *_soft_implementation data objects: call both function pointers directly so
// the maclen != {16,32} branches of aegis*_mac() are exercised too.
// ============================================================================

fn aegis_impl_suite(name: &str, cimpl: *const AegisImplementation, rimpl: *const AegisImplementation, keybytes: usize, npubbytes: usize) {
    let (ce, re) = unsafe { ((*cimpl).encrypt_detached, (*rimpl).encrypt_detached) };
    let (cd, rd) = unsafe { ((*cimpl).decrypt_detached, (*rimpl).decrypt_detached) };
    let mut rng = common::Rng::new(0xB0_0B_1E5 ^ keybytes as u64);

    for &maclen in &[0usize, 1, 8, 15, 16, 17, 31, 32, 33, 48] {
        for &mlen in &[0usize, 1, 15, 16, 17, 31, 32, 33, 63, 64, 65, 129] {
            for &adlen in &[0usize, 1, 16, 17, 32, 33, 64, 65] {
                let k = rng.bytes(keybytes);
                let npub = rng.bytes(npubbytes);
                let m = rng.bytes(mlen);
                let ad = rng.bytes(adlen);
                let ctx = format!("{name} impl maclen={maclen} mlen={mlen} adlen={adlen}");
                let adp = if adlen == 0 { core::ptr::null() } else { ad.as_ptr() };
                let mp = if mlen == 0 { core::ptr::null() } else { m.as_ptr() };

                let mut cc = vec![0x1Du8; mlen + 8];
                let mut rc_ = vec![0x1Du8; mlen + 8];
                let mut mc = vec![0x2Eu8; maclen + 8];
                let mut mr = vec![0x2Eu8; maclen + 8];
                let a = unsafe {
                    ce(
                        cc.as_mut_ptr(),
                        mc.as_mut_ptr(),
                        maclen,
                        mp,
                        mlen,
                        adp,
                        adlen,
                        npub.as_ptr(),
                        k.as_ptr(),
                    )
                };
                let b = unsafe {
                    re(
                        rc_.as_mut_ptr(),
                        mr.as_mut_ptr(),
                        maclen,
                        mp,
                        mlen,
                        adp,
                        adlen,
                        npub.as_ptr(),
                        k.as_ptr(),
                    )
                };
                common::eqi(&format!("{ctx} enc rc"), a, b);
                assert_eq!(
                    a,
                    if maclen == 16 || maclen == 32 { 0 } else { -1 },
                    "{ctx} enc rc value"
                );
                common::eqb(&format!("{ctx} enc c"), &cc, &rc_);
                common::eqb(&format!("{ctx} enc mac"), &mc, &mr);
                assert!(mc[maclen..].iter().all(|&x| x == 0x2E), "{ctx} mac overrun");

                let mut dc = vec![0x3Fu8; mlen + 8];
                let mut dr = vec![0x3Fu8; mlen + 8];
                let a = unsafe {
                    cd(
                        dc.as_mut_ptr(),
                        cc.as_ptr(),
                        mlen,
                        mc.as_ptr(),
                        maclen,
                        adp,
                        adlen,
                        npub.as_ptr(),
                        k.as_ptr(),
                    )
                };
                let b = unsafe {
                    rd(
                        dr.as_mut_ptr(),
                        rc_.as_ptr(),
                        mlen,
                        mr.as_ptr(),
                        maclen,
                        adp,
                        adlen,
                        npub.as_ptr(),
                        k.as_ptr(),
                    )
                };
                common::eqi(&format!("{ctx} dec rc"), a, b);
                assert_eq!(
                    a,
                    if maclen == 16 || maclen == 32 { 0 } else { -1 },
                    "{ctx} dec rc value"
                );
                common::eqb(&format!("{ctx} dec out"), &dc, &dr);
                if maclen == 16 || maclen == 32 {
                    assert_eq!(&dc[..mlen], &m[..], "{ctx} dec plaintext");
                } else {
                    assert!(dc[..mlen].iter().all(|&x| x == 0), "{ctx} dec must zero m");
                }
                assert!(dc[mlen..].iter().all(|&x| x == 0x3F), "{ctx} dec overrun");

                // m == NULL variant of the implementation-level decrypt
                let a = unsafe {
                    cd(
                        core::ptr::null_mut(),
                        cc.as_ptr(),
                        mlen,
                        mc.as_ptr(),
                        maclen,
                        adp,
                        adlen,
                        npub.as_ptr(),
                        k.as_ptr(),
                    )
                };
                let b = unsafe {
                    rd(
                        core::ptr::null_mut(),
                        rc_.as_ptr(),
                        mlen,
                        mr.as_ptr(),
                        maclen,
                        adp,
                        adlen,
                        npub.as_ptr(),
                        k.as_ptr(),
                    )
                };
                common::eqi(&format!("{ctx} dec(m=NULL) rc"), a, b);
            }
        }
    }
}

#[test]
fn aegis128l_soft_implementation_object() {
    let (ci, ri) = both_data!("aegis128l_soft_implementation", AegisImplementation);
    aegis_impl_suite("aegis128l", ci, ri, 16, 16);
}

#[test]
fn aegis256_soft_implementation_object() {
    let (ci, ri) = both_data!("aegis256_soft_implementation", AegisImplementation);
    aegis_impl_suite("aegis256", ci, ri, 32, 32);
}

#[test]
fn aegis_pick_best_implementation_is_idempotent() {
    let l = common::libs();
    for (name, prefix, kb, npb) in [
        ("aegis128l", "crypto_aead_aegis128l", 16usize, 16usize),
        ("aegis256", "crypto_aead_aegis256", 32, 32),
    ] {
        let c = load_aegis(&l.c, prefix);
        let r = load_aegis(&l.r, prefix);
        for round in 0..3 {
            let a = unsafe { (c.pick)() };
            let b = unsafe { (r.pick)() };
            common::eqi(&format!("{name} pick round {round}"), a, b);
            assert_eq!(a, 0);
            // encryption must still work (and match) after re-picking
            let mut rng = common::Rng::new(0xFEED ^ round as u64);
            let k = rng.bytes(kb);
            let npub = rng.bytes(npb);
            let m = rng.bytes(100);
            let mut cc = vec![0u8; 132];
            let mut rc_ = vec![0u8; 132];
            unsafe {
                (c.enc)(
                    cc.as_mut_ptr(),
                    core::ptr::null_mut(),
                    m.as_ptr(),
                    100,
                    core::ptr::null(),
                    0,
                    core::ptr::null(),
                    npub.as_ptr(),
                    k.as_ptr(),
                );
                (r.enc)(
                    rc_.as_mut_ptr(),
                    core::ptr::null_mut(),
                    m.as_ptr(),
                    100,
                    core::ptr::null(),
                    0,
                    core::ptr::null(),
                    npub.as_ptr(),
                    k.as_ptr(),
                );
            }
            common::eqb(&format!("{name} after pick round {round}"), &cc, &rc_);
        }
    }
}

// ============================================================================
// softaes
// ============================================================================

#[repr(C)]
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
struct SoftAesBlock {
    w0: u32,
    w1: u32,
    w2: u32,
    w3: u32,
}

impl SoftAesBlock {
    fn to_bytes(self) -> [u8; 16] {
        let mut o = [0u8; 16];
        o[0..4].copy_from_slice(&self.w0.to_le_bytes());
        o[4..8].copy_from_slice(&self.w1.to_le_bytes());
        o[8..12].copy_from_slice(&self.w2.to_le_bytes());
        o[12..16].copy_from_slice(&self.w3.to_le_bytes());
        o
    }
}

fn blocks_to_bytes(b: &[SoftAesBlock]) -> Vec<u8> {
    b.iter().flat_map(|x| x.to_bytes()).collect()
}

type BlockFn = unsafe extern "C" fn(SoftAesBlock, SoftAesBlock) -> SoftAesBlock;
type InvMix = unsafe extern "C" fn(SoftAesBlock) -> SoftAesBlock;
type Expand = unsafe extern "C" fn(*mut SoftAesBlock, *const u8);
type Invert = unsafe extern "C" fn(*mut SoftAesBlock);

#[test]
fn softaes_block_functions() {
    let (c_enc, r_enc) = both!("_sodium_softaes_block_encrypt", BlockFn);
    let (c_dec, r_dec) = both!("_sodium_softaes_block_decrypt", BlockFn);
    let (c_encl, r_encl) = both!("_sodium_softaes_block_encryptlast", BlockFn);
    let (c_decl, r_decl) = both!("_sodium_softaes_block_decryptlast", BlockFn);
    let (c_imc, r_imc) = both!("_sodium_softaes_inv_mix_columns", InvMix);

    let mut rng = common::Rng::new(0x50F7_AE50);

    // deterministic edge cases first, then many random blocks
    let mut cases: Vec<(SoftAesBlock, SoftAesBlock)> = vec![
        (SoftAesBlock::default(), SoftAesBlock::default()),
        (
            SoftAesBlock { w0: !0, w1: !0, w2: !0, w3: !0 },
            SoftAesBlock::default(),
        ),
        (
            SoftAesBlock::default(),
            SoftAesBlock { w0: !0, w1: !0, w2: !0, w3: !0 },
        ),
        (
            SoftAesBlock { w0: 1, w1: 0x8000_0000, w2: 0x00ff_00ff, w3: 0xff00_ff00 },
            SoftAesBlock { w0: 0x0102_0304, w1: 0x0506_0708, w2: 0x090a_0b0c, w3: 0x0d0e_0f10 },
        ),
    ];
    // every single-byte value in every byte position of w0 (exercises all SBOX
    // / INV_SBOX entries and every SRM-1R lane)
    for v in 0..256u32 {
        for sh in [0u32, 8, 16, 24] {
            cases.push((
                SoftAesBlock { w0: v << sh, w1: v << sh, w2: v << sh, w3: v << sh },
                SoftAesBlock::default(),
            ));
        }
    }
    for _ in 0..2000 {
        let b = SoftAesBlock {
            w0: rng.next_u64() as u32,
            w1: rng.next_u64() as u32,
            w2: rng.next_u64() as u32,
            w3: rng.next_u64() as u32,
        };
        let rk = SoftAesBlock {
            w0: rng.next_u64() as u32,
            w1: rng.next_u64() as u32,
            w2: rng.next_u64() as u32,
            w3: rng.next_u64() as u32,
        };
        cases.push((b, rk));
    }

    for (i, (b, rk)) in cases.iter().enumerate() {
        unsafe {
            common::eqb(
                &format!("softaes_block_encrypt #{i}"),
                &c_enc(*b, *rk).to_bytes(),
                &r_enc(*b, *rk).to_bytes(),
            );
            common::eqb(
                &format!("softaes_block_decrypt #{i}"),
                &c_dec(*b, *rk).to_bytes(),
                &r_dec(*b, *rk).to_bytes(),
            );
            common::eqb(
                &format!("softaes_block_encryptlast #{i}"),
                &c_encl(*b, *rk).to_bytes(),
                &r_encl(*b, *rk).to_bytes(),
            );
            common::eqb(
                &format!("softaes_block_decryptlast #{i}"),
                &c_decl(*b, *rk).to_bytes(),
                &r_decl(*b, *rk).to_bytes(),
            );
            common::eqb(
                &format!("softaes_inv_mix_columns #{i}"),
                &c_imc(*b).to_bytes(),
                &r_imc(*b).to_bytes(),
            );
        }
    }
}

#[test]
fn softaes_key_schedules() {
    let (c_e128, r_e128) = both!("_sodium_softaes_expand_key128", Expand);
    let (c_e256, r_e256) = both!("_sodium_softaes_expand_key256", Expand);
    let (c_i128, r_i128) = both!("_sodium_softaes_invert_key_schedule128", Invert);
    let (c_i256, r_i256) = both!("_sodium_softaes_invert_key_schedule256", Invert);

    let mut rng = common::Rng::new(0x5C4E_D01E);

    // fixed keys + many random keys
    let mut keys128: Vec<Vec<u8>> = vec![vec![0u8; 16], vec![0xFFu8; 16], (0..16u8).collect()];
    let mut keys256: Vec<Vec<u8>> = vec![vec![0u8; 32], vec![0xFFu8; 32], (0..32u8).collect()];
    for _ in 0..64 {
        keys128.push(rng.bytes(16));
        keys256.push(rng.bytes(32));
    }

    for (i, key) in keys128.iter().enumerate() {
        // 11 round keys + 2 canary blocks to catch over-writes
        let mut kc = [SoftAesBlock { w0: 0xDEAD_BEEF, w1: 0xDEAD_BEEF, w2: 0xDEAD_BEEF, w3: 0xDEAD_BEEF }; 13];
        let mut kr = kc;
        unsafe {
            c_e128(kc.as_mut_ptr(), key.as_ptr());
            r_e128(kr.as_mut_ptr(), key.as_ptr());
        }
        common::eqb(
            &format!("softaes_expand_key128 #{i}"),
            &blocks_to_bytes(&kc),
            &blocks_to_bytes(&kr),
        );
        assert_eq!(kc[11], kc[12], "expand_key128 #{i} canary");
        assert_eq!(kc[11].w0, 0xDEAD_BEEF, "expand_key128 #{i} overran");

        unsafe {
            c_i128(kc.as_mut_ptr());
            r_i128(kr.as_mut_ptr());
        }
        common::eqb(
            &format!("softaes_invert_key_schedule128 #{i}"),
            &blocks_to_bytes(&kc),
            &blocks_to_bytes(&kr),
        );
        // the C loop is `for (i = 1; i < 10; i++)`: index 0 and 10 stay put
        assert_eq!(kc[11].w0, 0xDEAD_BEEF, "invert128 #{i} overran");
    }

    for (i, key) in keys256.iter().enumerate() {
        let mut kc = [SoftAesBlock { w0: 0xC0DE_C0DE, w1: 0xC0DE_C0DE, w2: 0xC0DE_C0DE, w3: 0xC0DE_C0DE }; 17];
        let mut kr = kc;
        unsafe {
            c_e256(kc.as_mut_ptr(), key.as_ptr());
            r_e256(kr.as_mut_ptr(), key.as_ptr());
        }
        common::eqb(
            &format!("softaes_expand_key256 #{i}"),
            &blocks_to_bytes(&kc),
            &blocks_to_bytes(&kr),
        );
        assert_eq!(kc[15].w0, 0xC0DE_C0DE, "expand_key256 #{i} overran");

        unsafe {
            c_i256(kc.as_mut_ptr());
            r_i256(kr.as_mut_ptr());
        }
        common::eqb(
            &format!("softaes_invert_key_schedule256 #{i}"),
            &blocks_to_bytes(&kc),
            &blocks_to_bytes(&kr),
        );
        assert_eq!(kc[15].w0, 0xC0DE_C0DE, "invert256 #{i} overran");
    }
}

/// End-to-end AES-128/AES-256 block cipher built out of the exported softaes
/// primitives: catches any divergence that cancels out in a single round.
#[test]
fn softaes_full_cipher_roundtrip() {
    let (c_e128, r_e128) = both!("_sodium_softaes_expand_key128", Expand);
    let (c_e256, r_e256) = both!("_sodium_softaes_expand_key256", Expand);
    let (c_i128, r_i128) = both!("_sodium_softaes_invert_key_schedule128", Invert);
    let (c_i256, r_i256) = both!("_sodium_softaes_invert_key_schedule256", Invert);
    let (c_enc, r_enc) = both!("_sodium_softaes_block_encrypt", BlockFn);
    let (c_encl, r_encl) = both!("_sodium_softaes_block_encryptlast", BlockFn);
    let (c_dec, r_dec) = both!("_sodium_softaes_block_decrypt", BlockFn);
    let (c_decl, r_decl) = both!("_sodium_softaes_block_decryptlast", BlockFn);

    let xor = |a: SoftAesBlock, b: SoftAesBlock| SoftAesBlock {
        w0: a.w0 ^ b.w0,
        w1: a.w1 ^ b.w1,
        w2: a.w2 ^ b.w2,
        w3: a.w3 ^ b.w3,
    };

    let mut rng = common::Rng::new(0xA35_C1F4);
    for trial in 0..32 {
        let key128 = rng.bytes(16);
        let key256 = rng.bytes(32);
        let pt = SoftAesBlock {
            w0: rng.next_u64() as u32,
            w1: rng.next_u64() as u32,
            w2: rng.next_u64() as u32,
            w3: rng.next_u64() as u32,
        };

        // --- AES-128 ---
        let mut rkc = [SoftAesBlock::default(); 11];
        let mut rkr = [SoftAesBlock::default(); 11];
        unsafe {
            c_e128(rkc.as_mut_ptr(), key128.as_ptr());
            r_e128(rkr.as_mut_ptr(), key128.as_ptr());
        }
        let mut sc = xor(pt, rkc[0]);
        let mut sr = xor(pt, rkr[0]);
        for i in 1..10 {
            sc = unsafe { c_enc(sc, rkc[i]) };
            sr = unsafe { r_enc(sr, rkr[i]) };
        }
        let ctc = unsafe { c_encl(sc, rkc[10]) };
        let ctr = unsafe { r_encl(sr, rkr[10]) };
        common::eqb(
            &format!("aes128 encrypt trial {trial}"),
            &ctc.to_bytes(),
            &ctr.to_bytes(),
        );

        // decrypt with the inverted schedule
        let mut dkc = rkc;
        let mut dkr = rkr;
        unsafe {
            c_i128(dkc.as_mut_ptr());
            r_i128(dkr.as_mut_ptr());
        }
        let mut sc = xor(ctc, dkc[10]);
        let mut sr = xor(ctr, dkr[10]);
        for i in (1..10).rev() {
            sc = unsafe { c_dec(sc, dkc[i]) };
            sr = unsafe { r_dec(sr, dkr[i]) };
        }
        let ptc = unsafe { c_decl(sc, dkc[0]) };
        let ptr_ = unsafe { r_decl(sr, dkr[0]) };
        common::eqb(
            &format!("aes128 decrypt trial {trial}"),
            &ptc.to_bytes(),
            &ptr_.to_bytes(),
        );
        assert_eq!(ptc, pt, "aes128 roundtrip trial {trial}");

        // --- AES-256 ---
        let mut rkc = [SoftAesBlock::default(); 15];
        let mut rkr = [SoftAesBlock::default(); 15];
        unsafe {
            c_e256(rkc.as_mut_ptr(), key256.as_ptr());
            r_e256(rkr.as_mut_ptr(), key256.as_ptr());
        }
        let mut sc = xor(pt, rkc[0]);
        let mut sr = xor(pt, rkr[0]);
        for i in 1..14 {
            sc = unsafe { c_enc(sc, rkc[i]) };
            sr = unsafe { r_enc(sr, rkr[i]) };
        }
        let ctc = unsafe { c_encl(sc, rkc[14]) };
        let ctr = unsafe { r_encl(sr, rkr[14]) };
        common::eqb(
            &format!("aes256 encrypt trial {trial}"),
            &ctc.to_bytes(),
            &ctr.to_bytes(),
        );

        let mut dkc = rkc;
        let mut dkr = rkr;
        unsafe {
            c_i256(dkc.as_mut_ptr());
            r_i256(dkr.as_mut_ptr());
        }
        let mut sc = xor(ctc, dkc[14]);
        let mut sr = xor(ctr, dkr[14]);
        for i in (1..14).rev() {
            sc = unsafe { c_dec(sc, dkc[i]) };
            sr = unsafe { r_dec(sr, dkr[i]) };
        }
        let ptc = unsafe { c_decl(sc, dkc[0]) };
        let ptr_ = unsafe { r_decl(sr, dkr[0]) };
        common::eqb(
            &format!("aes256 decrypt trial {trial}"),
            &ptc.to_bytes(),
            &ptr_.to_bytes(),
        );
        assert_eq!(ptc, pt, "aes256 roundtrip trial {trial}");
    }
}

/// AES-128 known-answer test (FIPS-197 C.1) through the exported primitives:
/// pins both libraries to the real AES, not just to each other.
#[test]
fn softaes_fips197_kat() {
    let (c_e128, r_e128) = both!("_sodium_softaes_expand_key128", Expand);
    let (c_enc, r_enc) = both!("_sodium_softaes_block_encrypt", BlockFn);
    let (c_encl, r_encl) = both!("_sodium_softaes_block_encryptlast", BlockFn);

    // FIPS-197 C.1: key = 000102...0f
    let key: Vec<u8> = (0..16u8).collect();
    let pt: [u8; 16] = [
        0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee,
        0xff,
    ];
    let expect: [u8; 16] = [
        0x69, 0xc4, 0xe0, 0xd8, 0x6a, 0x7b, 0x04, 0x30, 0xd8, 0xcd, 0xb7, 0x80, 0x70, 0xb4, 0xc5,
        0x5a,
    ];

    let load = |b: &[u8]| SoftAesBlock {
        w0: u32::from_le_bytes([b[0], b[1], b[2], b[3]]),
        w1: u32::from_le_bytes([b[4], b[5], b[6], b[7]]),
        w2: u32::from_le_bytes([b[8], b[9], b[10], b[11]]),
        w3: u32::from_le_bytes([b[12], b[13], b[14], b[15]]),
    };
    let xor = |a: SoftAesBlock, b: SoftAesBlock| SoftAesBlock {
        w0: a.w0 ^ b.w0,
        w1: a.w1 ^ b.w1,
        w2: a.w2 ^ b.w2,
        w3: a.w3 ^ b.w3,
    };

    for (tag, ex, en, enl) in [
        ("C", c_e128, c_enc, c_encl),
        ("Rust", r_e128, r_enc, r_encl),
    ] {
        let mut rk = [SoftAesBlock::default(); 11];
        unsafe { ex(rk.as_mut_ptr(), key.as_ptr()) };
        let mut s = xor(load(&pt), rk[0]);
        for i in 1..10 {
            s = unsafe { en(s, rk[i]) };
        }
        let ct = unsafe { enl(s, rk[10]) };
        common::eqb(&format!("{tag} FIPS-197 AES-128 KAT"), &expect, &ct.to_bytes());
    }
}

// ============================================================================
// keygen (needs a deterministic randombytes implementation to be comparable)
// ============================================================================

#[repr(C)]
struct RandombytesImplementation {
    implementation_name: Option<unsafe extern "C" fn() -> *const c_char>,
    random: Option<unsafe extern "C" fn() -> u32>,
    stir: Option<unsafe extern "C" fn()>,
    uniform: Option<unsafe extern "C" fn(u32) -> u32>,
    buf: Option<unsafe extern "C" fn(*mut c_void, usize)>,
    close: Option<unsafe extern "C" fn() -> c_int>,
}
unsafe impl Sync for RandombytesImplementation {}

static CTR: AtomicU64 = AtomicU64::new(0);

unsafe extern "C" fn det_name() -> *const c_char {
    b"aead2-deterministic\0".as_ptr() as *const c_char
}
unsafe extern "C" fn det_random() -> u32 {
    CTR.fetch_add(1, Ordering::SeqCst) as u32
}
unsafe extern "C" fn det_stir() {}
unsafe extern "C" fn det_uniform(u: u32) -> u32 {
    if u == 0 {
        0
    } else {
        (CTR.fetch_add(1, Ordering::SeqCst) as u32) % u
    }
}
unsafe extern "C" fn det_buf(buf: *mut c_void, size: usize) {
    let p = buf as *mut u8;
    for i in 0..size {
        let n = CTR.fetch_add(1, Ordering::SeqCst);
        *p.add(i) = (n ^ (n >> 7) ^ 0xA5) as u8;
    }
}
unsafe extern "C" fn det_close() -> c_int {
    0
}

static DET_IMPL: RandombytesImplementation = RandombytesImplementation {
    implementation_name: Some(det_name),
    random: Some(det_random),
    stir: Some(det_stir),
    uniform: Some(det_uniform),
    buf: Some(det_buf),
    close: Some(det_close),
};

#[test]
fn keygen_writes_exactly_keybytes() {
    type SetImpl = unsafe extern "C" fn(*const RandombytesImplementation) -> c_int;
    let (c_set, r_set) = both!("randombytes_set_implementation", SetImpl);
    unsafe {
        assert_eq!(c_set(&DET_IMPL), 0, "C randombytes_set_implementation");
        assert_eq!(r_set(&DET_IMPL), 0, "Rust randombytes_set_implementation");
    }

    for (label, n) in [
        ("crypto_aead_aes256gcm_keygen", 32usize),
        ("crypto_aead_aegis128l_keygen", 16),
        ("crypto_aead_aegis256_keygen", 32),
    ] {
        let l = common::libs();
        let cf: Keygen = sym(&l.c, label);
        let rf: Keygen = sym(&l.r, label);
        for round in 0..8 {
            let mut kc = vec![0x5Du8; n + 16];
            let mut kr = vec![0x5Du8; n + 16];
            CTR.store(round * 1000, Ordering::SeqCst);
            unsafe { cf(kc.as_mut_ptr()) };
            CTR.store(round * 1000, Ordering::SeqCst);
            unsafe { rf(kr.as_mut_ptr()) };
            common::eqb(&format!("{label} round {round}"), &kc, &kr);
            assert!(
                kc[n..].iter().all(|&b| b == 0x5D),
                "{label} wrote past {n} bytes"
            );
            assert!(
                kc[..n].iter().any(|&b| b != 0x5D),
                "{label} wrote nothing"
            );
        }
    }
}
