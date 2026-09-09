//! Phase B — valid-path differential tests, CONFIGS.md group 2, part 2:
//! AEAD (rows 60-72), secretbox (73-80), secretstream (81-90), box (91-103)

mod common;
use common::*;
use std::os::raw::{c_char, c_int, c_ulonglong};

type Sz = usize;

const MLENS: &[usize] = &[
    0, 1, 15, 16, 17, 31, 32, 33, 63, 64, 65, 127, 128, 129, 191, 192, 255, 256, 1000, 4096,
];
const ADLENS: &[usize] = &[0, 1, 15, 16, 17, 31, 32, 64, 100];

// ============================================================ AEAD

struct Aead {
    prefix: String,
    key: usize,
    npub: usize,
    abytes: usize,
}

fn aead_of(prefix: &str) -> Aead {
    let g = |s: &str| -> usize {
        let (c, r) = pair::<unsafe extern "C" fn() -> Sz>(&format!("{prefix}_{s}"));
        unsafe {
            assert_eq!(c(), r(), "{prefix}_{s}");
            c()
        }
    };
    Aead {
        prefix: prefix.to_string(),
        key: g("keybytes"),
        npub: g("npubbytes"),
        abytes: g("abytes"),
    }
}

type EncF = unsafe extern "C" fn(
    *mut u8,
    *mut c_ulonglong,
    *const u8,
    c_ulonglong,
    *const u8,
    c_ulonglong,
    *const u8,
    *const u8,
    *const u8,
) -> c_int;
type EncDetF = unsafe extern "C" fn(
    *mut u8,
    *mut u8,
    *mut c_ulonglong,
    *const u8,
    c_ulonglong,
    *const u8,
    c_ulonglong,
    *const u8,
    *const u8,
    *const u8,
) -> c_int;
type DecF = unsafe extern "C" fn(
    *mut u8,
    *mut c_ulonglong,
    *mut u8,
    *const u8,
    c_ulonglong,
    *const u8,
    c_ulonglong,
    *const u8,
    *const u8,
) -> c_int;
type DecDetF = unsafe extern "C" fn(
    *mut u8,
    *mut u8,
    *const u8,
    c_ulonglong,
    *const u8,
    *const u8,
    c_ulonglong,
    *const u8,
    *const u8,
) -> c_int;

fn aead_family(a: &Aead) {
    let p = &a.prefix;
    let (ec, er) = pair::<EncF>(&format!("{p}_encrypt"));
    let (edc, edr) = pair::<EncDetF>(&format!("{p}_encrypt_detached"));
    let (dc, dr) = pair::<DecF>(&format!("{p}_decrypt"));
    let (ddc, ddr) = pair::<DecDetF>(&format!("{p}_decrypt_detached"));

    let mut rng = Rng::new(SEED ^ p.len() as u64 ^ a.npub as u64);
    for &ml in MLENS {
        for &al in ADLENS {
            for rep in 0..3 {
                let k = rng.bytes(a.key);
                let np = rng.bytes(a.npub);
                let m = rng.bytes(ml);
                let ad = rng.bytes(al.max(1));
                // ad: NULL when al==0 on odd reps, non-NULL otherwise
                let adp: *const u8 = if al == 0 && rep % 2 == 0 {
                    std::ptr::null()
                } else {
                    ad.as_ptr()
                };

                // --- combined encrypt, clen_p set and NULL
                for use_clen in [true, false] {
                    let mut ca = buf(ml + a.abytes + 8);
                    let mut cb = buf(ml + a.abytes + 8);
                    let mut lc: c_ulonglong = 0xDEAD;
                    let mut lr: c_ulonglong = 0xDEAD;
                    unsafe {
                        same_ret(
                            &format!("{p}_encrypt"),
                            ec(
                                ca.as_mut_ptr(),
                                if use_clen { &mut lc } else { std::ptr::null_mut() },
                                m.as_ptr(),
                                ml as c_ulonglong,
                                adp,
                                al as c_ulonglong,
                                std::ptr::null(),
                                np.as_ptr(),
                                k.as_ptr(),
                            ),
                            er(
                                cb.as_mut_ptr(),
                                if use_clen { &mut lr } else { std::ptr::null_mut() },
                                m.as_ptr(),
                                ml as c_ulonglong,
                                adp,
                                al as c_ulonglong,
                                std::ptr::null(),
                                np.as_ptr(),
                                k.as_ptr(),
                            ),
                        );
                    }
                    same_bytes(&format!("{p}_encrypt ml={ml} al={al}"), &ca, &cb);
                    assert_eq!(lc, lr, "{p}_encrypt clen");
                }

                // recompute a canonical ciphertext to decrypt
                let mut ct = buf(ml + a.abytes);
                let mut clen: c_ulonglong = 0;
                unsafe {
                    ec(
                        ct.as_mut_ptr(),
                        &mut clen,
                        m.as_ptr(),
                        ml as c_ulonglong,
                        adp,
                        al as c_ulonglong,
                        std::ptr::null(),
                        np.as_ptr(),
                        k.as_ptr(),
                    );
                }

                // --- combined decrypt, m set / NULL, mlen_p set / NULL
                for use_m in [true, false] {
                    for use_mlen in [true, false] {
                        let mut ma = buf(ml + 8);
                        let mut mb = buf(ml + 8);
                        let mut lc: c_ulonglong = 0xDEAD;
                        let mut lr: c_ulonglong = 0xDEAD;
                        unsafe {
                            same_ret(
                                &format!("{p}_decrypt"),
                                dc(
                                    if use_m { ma.as_mut_ptr() } else { std::ptr::null_mut() },
                                    if use_mlen { &mut lc } else { std::ptr::null_mut() },
                                    std::ptr::null_mut(),
                                    ct.as_ptr(),
                                    clen,
                                    adp,
                                    al as c_ulonglong,
                                    np.as_ptr(),
                                    k.as_ptr(),
                                ),
                                dr(
                                    if use_m { mb.as_mut_ptr() } else { std::ptr::null_mut() },
                                    if use_mlen { &mut lr } else { std::ptr::null_mut() },
                                    std::ptr::null_mut(),
                                    ct.as_ptr(),
                                    clen,
                                    adp,
                                    al as c_ulonglong,
                                    np.as_ptr(),
                                    k.as_ptr(),
                                ),
                            );
                        }
                        same_bytes(&format!("{p}_decrypt ml={ml} al={al}"), &ma, &mb);
                        assert_eq!(lc, lr, "{p}_decrypt mlen");
                        if use_m {
                            same_bytes(&format!("{p} round-trip ml={ml}"), &m, &ma[..ml]);
                        }
                    }
                }

                // --- detached encrypt, maclen_p set / NULL
                for use_maclen in [true, false] {
                    let mut ca = buf(ml + 8);
                    let mut cb = buf(ml + 8);
                    let mut mca = buf(a.abytes);
                    let mut mcb = buf(a.abytes);
                    let mut lc: c_ulonglong = 0xDEAD;
                    let mut lr: c_ulonglong = 0xDEAD;
                    unsafe {
                        same_ret(
                            &format!("{p}_encrypt_detached"),
                            edc(
                                ca.as_mut_ptr(),
                                mca.as_mut_ptr(),
                                if use_maclen { &mut lc } else { std::ptr::null_mut() },
                                m.as_ptr(),
                                ml as c_ulonglong,
                                adp,
                                al as c_ulonglong,
                                std::ptr::null(),
                                np.as_ptr(),
                                k.as_ptr(),
                            ),
                            edr(
                                cb.as_mut_ptr(),
                                mcb.as_mut_ptr(),
                                if use_maclen { &mut lr } else { std::ptr::null_mut() },
                                m.as_ptr(),
                                ml as c_ulonglong,
                                adp,
                                al as c_ulonglong,
                                std::ptr::null(),
                                np.as_ptr(),
                                k.as_ptr(),
                            ),
                        );
                    }
                    same_bytes(&format!("{p}_encrypt_detached c ml={ml} al={al}"), &ca, &cb);
                    same_bytes(&format!("{p}_encrypt_detached mac ml={ml} al={al}"), &mca, &mcb);
                    assert_eq!(lc, lr, "{p}_encrypt_detached maclen");

                    // --- detached decrypt, m set / NULL (verify-only)
                    for use_m in [true, false] {
                        let mut ma = buf(ml + 8);
                        let mut mb = buf(ml + 8);
                        unsafe {
                            same_ret(
                                &format!("{p}_decrypt_detached"),
                                ddc(
                                    if use_m { ma.as_mut_ptr() } else { std::ptr::null_mut() },
                                    std::ptr::null_mut(),
                                    ca.as_ptr(),
                                    ml as c_ulonglong,
                                    mca.as_ptr(),
                                    adp,
                                    al as c_ulonglong,
                                    np.as_ptr(),
                                    k.as_ptr(),
                                ),
                                ddr(
                                    if use_m { mb.as_mut_ptr() } else { std::ptr::null_mut() },
                                    std::ptr::null_mut(),
                                    cb.as_ptr(),
                                    ml as c_ulonglong,
                                    mcb.as_ptr(),
                                    adp,
                                    al as c_ulonglong,
                                    np.as_ptr(),
                                    k.as_ptr(),
                                ),
                            );
                        }
                        same_bytes(&format!("{p}_decrypt_detached ml={ml} al={al}"), &ma, &mb);
                    }
                }
            }
        }
    }
    // keygen: length only
    let g = format!("{p}_keygen");
    if libs().has(&g) {
        let (c, r) = pair::<unsafe extern "C" fn(*mut u8)>(&g);
        let mut x = buf(a.key);
        unsafe {
            c(x.as_mut_ptr());
            r(x.as_mut_ptr());
        }
    }
    for s in ["nsecbytes", "messagebytes_max"] {
        let g = format!("{p}_{s}");
        if libs().has(&g) {
            let (c, r) = pair::<unsafe extern "C" fn() -> Sz>(&g);
            unsafe { assert_eq!(c(), r(), "{g}") };
        }
    }
}

#[test]
fn rows60_63_chacha20poly1305() {
    aead_family(&aead_of("crypto_aead_chacha20poly1305"));
}

#[test]
fn rows64_65_chacha20poly1305_ietf() {
    aead_family(&aead_of("crypto_aead_chacha20poly1305_ietf"));
}

#[test]
fn rows66_67_xchacha20poly1305_ietf() {
    aead_family(&aead_of("crypto_aead_xchacha20poly1305_ietf"));
}

#[test]
fn rows68_69_aegis128l() {
    aead_family(&aead_of("crypto_aead_aegis128l"));
}

#[test]
fn rows70_71_aegis256() {
    aead_family(&aead_of("crypto_aead_aegis256"));
}

#[test]
fn row72_aes256gcm_enosys_stubs() {
    // With no HAVE_* macros the C compiles to ENOSYS stubs. Rust must match.
    let (avc, avr) = pair::<unsafe extern "C" fn() -> c_int>("crypto_aead_aes256gcm_is_available");
    unsafe { same_ret("aes256gcm_is_available", avc(), avr()) };

    let mut rng = Rng::seeded();
    let k = rng.bytes(32);
    let np = rng.bytes(12);
    let m = rng.bytes(64);
    let ad = rng.bytes(16);
    let mut o = buf(256);

    let (ec, er) = pair::<EncF>("crypto_aead_aes256gcm_encrypt");
    unsafe {
        let (a, ea) = with_errno(|| {
            ec(o.as_mut_ptr(), std::ptr::null_mut(), m.as_ptr(), 64, ad.as_ptr(), 16, std::ptr::null(), np.as_ptr(), k.as_ptr())
        });
        let (b, eb) = with_errno(|| {
            er(o.as_mut_ptr(), std::ptr::null_mut(), m.as_ptr(), 64, ad.as_ptr(), 16, std::ptr::null(), np.as_ptr(), k.as_ptr())
        });
        same_ret("aes256gcm_encrypt", a, b);
        assert_eq!(ea, eb, "aes256gcm_encrypt errno (C={ea} Rust={eb})");
    }
    let (dc, dr) = pair::<DecF>("crypto_aead_aes256gcm_decrypt");
    unsafe {
        let (a, ea) = with_errno(|| {
            dc(o.as_mut_ptr(), std::ptr::null_mut(), std::ptr::null_mut(), m.as_ptr(), 64, ad.as_ptr(), 16, np.as_ptr(), k.as_ptr())
        });
        let (b, eb) = with_errno(|| {
            dr(o.as_mut_ptr(), std::ptr::null_mut(), std::ptr::null_mut(), m.as_ptr(), 64, ad.as_ptr(), 16, np.as_ptr(), k.as_ptr())
        });
        same_ret("aes256gcm_decrypt", a, b);
        assert_eq!(ea, eb, "aes256gcm_decrypt errno");
    }
    let (edc, edr) = pair::<EncDetF>("crypto_aead_aes256gcm_encrypt_detached");
    let mut mac = buf(16);
    unsafe {
        let (a, ea) = with_errno(|| {
            edc(o.as_mut_ptr(), mac.as_mut_ptr(), std::ptr::null_mut(), m.as_ptr(), 64, ad.as_ptr(), 16, std::ptr::null(), np.as_ptr(), k.as_ptr())
        });
        let (b, eb) = with_errno(|| {
            edr(o.as_mut_ptr(), mac.as_mut_ptr(), std::ptr::null_mut(), m.as_ptr(), 64, ad.as_ptr(), 16, std::ptr::null(), np.as_ptr(), k.as_ptr())
        });
        same_ret("aes256gcm_encrypt_detached", a, b);
        assert_eq!(ea, eb, "errno");
    }
    let (ddc, ddr) = pair::<DecDetF>("crypto_aead_aes256gcm_decrypt_detached");
    unsafe {
        let (a, ea) = with_errno(|| {
            ddc(o.as_mut_ptr(), std::ptr::null_mut(), m.as_ptr(), 64, mac.as_ptr(), ad.as_ptr(), 16, np.as_ptr(), k.as_ptr())
        });
        let (b, eb) = with_errno(|| {
            ddr(o.as_mut_ptr(), std::ptr::null_mut(), m.as_ptr(), 64, mac.as_ptr(), ad.as_ptr(), 16, np.as_ptr(), k.as_ptr())
        });
        same_ret("aes256gcm_decrypt_detached", a, b);
        assert_eq!(ea, eb, "errno");
    }
    // beforenm + all four afternm variants
    let sb = {
        let (c, r) = pair::<unsafe extern "C" fn() -> Sz>("crypto_aead_aes256gcm_statebytes");
        unsafe {
            assert_eq!(c(), r());
            c()
        }
    };
    let mut st = buf(sb.max(512));
    let (bc, br) =
        pair::<unsafe extern "C" fn(*mut u8, *const u8) -> c_int>("crypto_aead_aes256gcm_beforenm");
    unsafe {
        let (a, ea) = with_errno(|| bc(st.as_mut_ptr(), k.as_ptr()));
        let (b, eb) = with_errno(|| br(st.as_mut_ptr(), k.as_ptr()));
        same_ret("aes256gcm_beforenm", a, b);
        assert_eq!(ea, eb, "errno");
    }
    let (ac, ar) = pair::<EncF>("crypto_aead_aes256gcm_encrypt_afternm");
    unsafe {
        let (a, ea) = with_errno(|| {
            ac(o.as_mut_ptr(), std::ptr::null_mut(), m.as_ptr(), 64, ad.as_ptr(), 16, std::ptr::null(), np.as_ptr(), st.as_ptr())
        });
        let (b, eb) = with_errno(|| {
            ar(o.as_mut_ptr(), std::ptr::null_mut(), m.as_ptr(), 64, ad.as_ptr(), 16, std::ptr::null(), np.as_ptr(), st.as_ptr())
        });
        same_ret("encrypt_afternm", a, b);
        assert_eq!(ea, eb, "errno");
    }
    let (ac, ar) = pair::<EncDetF>("crypto_aead_aes256gcm_encrypt_detached_afternm");
    unsafe {
        let (a, ea) = with_errno(|| {
            ac(o.as_mut_ptr(), mac.as_mut_ptr(), std::ptr::null_mut(), m.as_ptr(), 64, ad.as_ptr(), 16, std::ptr::null(), np.as_ptr(), st.as_ptr())
        });
        let (b, eb) = with_errno(|| {
            ar(o.as_mut_ptr(), mac.as_mut_ptr(), std::ptr::null_mut(), m.as_ptr(), 64, ad.as_ptr(), 16, std::ptr::null(), np.as_ptr(), st.as_ptr())
        });
        same_ret("encrypt_detached_afternm", a, b);
        assert_eq!(ea, eb, "errno");
    }
    let (ac, ar) = pair::<DecF>("crypto_aead_aes256gcm_decrypt_afternm");
    unsafe {
        let (a, ea) = with_errno(|| {
            ac(o.as_mut_ptr(), std::ptr::null_mut(), std::ptr::null_mut(), m.as_ptr(), 64, ad.as_ptr(), 16, np.as_ptr(), st.as_ptr())
        });
        let (b, eb) = with_errno(|| {
            ar(o.as_mut_ptr(), std::ptr::null_mut(), std::ptr::null_mut(), m.as_ptr(), 64, ad.as_ptr(), 16, np.as_ptr(), st.as_ptr())
        });
        same_ret("decrypt_afternm", a, b);
        assert_eq!(ea, eb, "errno");
    }
    let (ac, ar) = pair::<DecDetF>("crypto_aead_aes256gcm_decrypt_detached_afternm");
    unsafe {
        let (a, ea) = with_errno(|| {
            ac(o.as_mut_ptr(), std::ptr::null_mut(), m.as_ptr(), 64, mac.as_ptr(), ad.as_ptr(), 16, np.as_ptr(), st.as_ptr())
        });
        let (b, eb) = with_errno(|| {
            ar(o.as_mut_ptr(), std::ptr::null_mut(), m.as_ptr(), 64, mac.as_ptr(), ad.as_ptr(), 16, np.as_ptr(), st.as_ptr())
        });
        same_ret("decrypt_detached_afternm", a, b);
        assert_eq!(ea, eb, "errno");
    }
    for g in [
        "crypto_aead_aes256gcm_keybytes",
        "crypto_aead_aes256gcm_nsecbytes",
        "crypto_aead_aes256gcm_npubbytes",
        "crypto_aead_aes256gcm_abytes",
        "crypto_aead_aes256gcm_statebytes",
        "crypto_aead_aes256gcm_messagebytes_max",
    ] {
        let (c, r) = pair::<unsafe extern "C" fn() -> Sz>(g);
        unsafe { assert_eq!(c(), r(), "{g}") };
    }
}

// ======================================================= secretbox

type SbEasyF =
    unsafe extern "C" fn(*mut u8, *const u8, c_ulonglong, *const u8, *const u8) -> c_int;
type SbDetF =
    unsafe extern "C" fn(*mut u8, *mut u8, *const u8, c_ulonglong, *const u8, *const u8) -> c_int;
type SbOpenDetF =
    unsafe extern "C" fn(*mut u8, *const u8, *const u8, c_ulonglong, *const u8, *const u8) -> c_int;

fn secretbox_family(p: &str) {
    let mac = {
        let (c, r) = pair::<unsafe extern "C" fn() -> Sz>(&format!("{p}_macbytes"));
        unsafe {
            assert_eq!(c(), r());
            c()
        }
    };
    let nonce = {
        let (c, r) = pair::<unsafe extern "C" fn() -> Sz>(&format!("{p}_noncebytes"));
        unsafe {
            assert_eq!(c(), r());
            c()
        }
    };
    let keyb = {
        let (c, r) = pair::<unsafe extern "C" fn() -> Sz>(&format!("{p}_keybytes"));
        unsafe {
            assert_eq!(c(), r());
            c()
        }
    };

    let (ec, er) = pair::<SbEasyF>(&format!("{p}_easy"));
    let (oc, or) = pair::<SbEasyF>(&format!("{p}_open_easy"));
    let (dc, dr) = pair::<SbDetF>(&format!("{p}_detached"));
    let (odc, odr) = pair::<SbOpenDetF>(&format!("{p}_open_detached"));

    let mut rng = Rng::new(SEED ^ p.len() as u64);
    for &ml in MLENS {
        for _ in 0..4 {
            let k = rng.bytes(keyb);
            let n = rng.bytes(nonce);
            let m = rng.bytes(ml);

            let mut ca = buf(ml + mac + 8);
            let mut cb = buf(ml + mac + 8);
            unsafe {
                same_ret(
                    &format!("{p}_easy"),
                    ec(ca.as_mut_ptr(), m.as_ptr(), ml as c_ulonglong, n.as_ptr(), k.as_ptr()),
                    er(cb.as_mut_ptr(), m.as_ptr(), ml as c_ulonglong, n.as_ptr(), k.as_ptr()),
                );
            }
            same_bytes(&format!("{p}_easy ml={ml}"), &ca, &cb);

            let mut ma = buf(ml + 8);
            let mut mb = buf(ml + 8);
            unsafe {
                same_ret(
                    &format!("{p}_open_easy"),
                    oc(ma.as_mut_ptr(), ca.as_ptr(), (ml + mac) as c_ulonglong, n.as_ptr(), k.as_ptr()),
                    or(mb.as_mut_ptr(), cb.as_ptr(), (ml + mac) as c_ulonglong, n.as_ptr(), k.as_ptr()),
                );
            }
            same_bytes(&format!("{p}_open_easy ml={ml}"), &ma, &mb);
            same_bytes(&format!("{p} round-trip ml={ml}"), &m, &ma[..ml]);

            // in-place easy (c and m overlap) — the C explicitly memmoves
            let mut ia = m.clone();
            ia.resize(ml + mac + 8, CANARY);
            let mut ib = ia.clone();
            unsafe {
                ec(ia.as_mut_ptr(), ia.as_ptr(), ml as c_ulonglong, n.as_ptr(), k.as_ptr());
                er(ib.as_mut_ptr(), ib.as_ptr(), ml as c_ulonglong, n.as_ptr(), k.as_ptr());
            }
            same_bytes(&format!("{p}_easy in-place ml={ml}"), &ia, &ib);

            // detached
            let mut ca = buf(ml + 8);
            let mut cb = buf(ml + 8);
            let mut mca = buf(mac);
            let mut mcb = buf(mac);
            unsafe {
                same_ret(
                    &format!("{p}_detached"),
                    dc(ca.as_mut_ptr(), mca.as_mut_ptr(), m.as_ptr(), ml as c_ulonglong, n.as_ptr(), k.as_ptr()),
                    dr(cb.as_mut_ptr(), mcb.as_mut_ptr(), m.as_ptr(), ml as c_ulonglong, n.as_ptr(), k.as_ptr()),
                );
            }
            same_bytes(&format!("{p}_detached c ml={ml}"), &ca, &cb);
            same_bytes(&format!("{p}_detached mac ml={ml}"), &mca, &mcb);

            for use_m in [true, false] {
                let mut ma = buf(ml + 8);
                let mut mb = buf(ml + 8);
                unsafe {
                    same_ret(
                        &format!("{p}_open_detached"),
                        odc(
                            if use_m { ma.as_mut_ptr() } else { std::ptr::null_mut() },
                            ca.as_ptr(),
                            mca.as_ptr(),
                            ml as c_ulonglong,
                            n.as_ptr(),
                            k.as_ptr(),
                        ),
                        odr(
                            if use_m { mb.as_mut_ptr() } else { std::ptr::null_mut() },
                            cb.as_ptr(),
                            mcb.as_ptr(),
                            ml as c_ulonglong,
                            n.as_ptr(),
                            k.as_ptr(),
                        ),
                    );
                }
                same_bytes(&format!("{p}_open_detached ml={ml}"), &ma, &mb);
            }
        }
    }
    let g = format!("{p}_keygen");
    if libs().has(&g) {
        let (c, r) = pair::<unsafe extern "C" fn(*mut u8)>(&g);
        let mut x = buf(keyb);
        unsafe {
            c(x.as_mut_ptr());
            r(x.as_mut_ptr());
        }
    }
    for s in ["messagebytes_max", "zerobytes", "boxzerobytes"] {
        let g = format!("{p}_{s}");
        if libs().has(&g) {
            let (c, r) = pair::<unsafe extern "C" fn() -> Sz>(&g);
            unsafe { assert_eq!(c(), r(), "{g}") };
        }
    }
}

#[test]
fn rows73_76_secretbox() {
    secretbox_family("crypto_secretbox");
    let (c, r) = pair::<unsafe extern "C" fn() -> *const c_char>("crypto_secretbox_primitive");
    unsafe {
        let a = std::ffi::CStr::from_ptr(c()).to_bytes().to_vec();
        let b = std::ffi::CStr::from_ptr(r()).to_bytes().to_vec();
        same_bytes("crypto_secretbox_primitive", &a, &b);
    }
}

#[test]
fn rows79_80_secretbox_xchacha20poly1305() {
    secretbox_family("crypto_secretbox_xchacha20poly1305");
}

/// Rows 77-78: the padded low-level xsalsa20poly1305 / xchacha20poly1305 form.
#[test]
fn rows77_78_secretbox_padded_lowlevel() {
    for p in [
        "crypto_secretbox_xsalsa20poly1305",
        "crypto_secretbox_xchacha20poly1305",
    ] {
        if !libs().has(p) {
            continue;
        }
        let (ec, er) = pair::<SbEasyF>(p);
        let (oc, or) = pair::<SbEasyF>(&format!("{p}_open"));
        let mut rng = Rng::new(SEED ^ 7);
        // The padded API requires mlen >= 32 with the first 32 bytes zeroed.
        for &body in &[0usize, 1, 15, 16, 31, 32, 33, 64, 65, 128, 1000] {
            let ml = 32 + body;
            for _ in 0..4 {
                let k = rng.bytes(32);
                let n = rng.bytes(24);
                let mut m = vec![0u8; 32];
                m.extend(rng.bytes(body));
                let mut ca = buf(ml + 8);
                let mut cb = buf(ml + 8);
                unsafe {
                    same_ret(
                        p,
                        ec(ca.as_mut_ptr(), m.as_ptr(), ml as c_ulonglong, n.as_ptr(), k.as_ptr()),
                        er(cb.as_mut_ptr(), m.as_ptr(), ml as c_ulonglong, n.as_ptr(), k.as_ptr()),
                    );
                }
                same_bytes(&format!("{p} padded ml={ml}"), &ca, &cb);
                // first 16 bytes must be zero per the NaCl contract
                assert!(ca[..16].iter().all(|&x| x == 0), "{p}: c[0..16] not zeroed");

                let mut ma = buf(ml + 8);
                let mut mb = buf(ml + 8);
                unsafe {
                    same_ret(
                        &format!("{p}_open"),
                        oc(ma.as_mut_ptr(), ca.as_ptr(), ml as c_ulonglong, n.as_ptr(), k.as_ptr()),
                        or(mb.as_mut_ptr(), cb.as_ptr(), ml as c_ulonglong, n.as_ptr(), k.as_ptr()),
                    );
                }
                same_bytes(&format!("{p}_open padded ml={ml}"), &ma, &mb);
                same_bytes(&format!("{p} padded round-trip"), &m[32..], &ma[32..ml]);
            }
        }
    }
}

// ==================================================== secretstream

#[test]
fn rows81_90_secretstream() {
    let p = "crypto_secretstream_xchacha20poly1305";
    let g = |s: &str| -> usize {
        let (c, r) = pair::<unsafe extern "C" fn() -> Sz>(&format!("{p}_{s}"));
        unsafe {
            assert_eq!(c(), r(), "{p}_{s}");
            c()
        }
    };
    let statebytes = g("statebytes");
    let abytes = g("abytes");
    let headerbytes = g("headerbytes");
    let keybytes = g("keybytes");
    let _ = g("messagebytes_max");

    let tagf = |s: &str| -> u8 {
        let (c, r) = pair::<unsafe extern "C" fn() -> u8>(&format!("{p}_tag_{s}"));
        unsafe {
            assert_eq!(c(), r(), "{p}_tag_{s}");
            c()
        }
    };
    let t_msg = tagf("message");
    let t_push = tagf("push");
    let t_rekey = tagf("rekey");
    let t_final = tagf("final");

    let (ipc, ipr) = pair::<unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> c_int>(&format!(
        "{p}_init_push"
    ));
    let (iplc, iplr) = pair::<unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> c_int>(
        &format!("{p}_init_pull"),
    );
    let (pushc, pushr) = pair::<
        unsafe extern "C" fn(
            *mut u8,
            *mut u8,
            *mut c_ulonglong,
            *const u8,
            c_ulonglong,
            *const u8,
            c_ulonglong,
            u8,
        ) -> c_int,
    >(&format!("{p}_push"));
    let (pullc, pullr) = pair::<
        unsafe extern "C" fn(
            *mut u8,
            *mut u8,
            *mut c_ulonglong,
            *mut u8,
            *const u8,
            c_ulonglong,
            *const u8,
            c_ulonglong,
        ) -> c_int,
    >(&format!("{p}_pull"));
    let (rkc, rkr) = pair::<unsafe extern "C" fn(*mut u8)>(&format!("{p}_rekey"));

    let mut rng = Rng::seeded();

    // Rows 83-88, 90: multi-message streams with mixed tags, ad present/absent
    for nmsg in [1usize, 2, 3, 10, 50] {
        for trial in 0..4 {
            let k = rng.bytes(keybytes);
            let mut hc = buf(headerbytes);
            let mut hr = buf(headerbytes);
            let mut spc = buf(statebytes);
            let mut spr = buf(statebytes);
            // init_push writes a RANDOM header, so seed both states from the
            // C-generated header via init_pull to keep them in lockstep.
            unsafe {
                same_ret("init_push", ipc(spc.as_mut_ptr(), hc.as_mut_ptr(), k.as_ptr()), ipr(spr.as_mut_ptr(), hr.as_mut_ptr(), k.as_ptr()));
            }
            // Deterministic comparison: drive BOTH push states from the same
            // header using init_pull (the state layout is identical), then
            // compare every push output.
            let header = hc.clone();
            unsafe {
                same_ret(
                    "init_pull (as push seed)",
                    iplc(spc.as_mut_ptr(), header.as_ptr(), k.as_ptr()),
                    iplr(spr.as_mut_ptr(), header.as_ptr(), k.as_ptr()),
                );
            }
            same_bytes("secretstream state after init", &spc, &spr);

            // pull states for the round-trip
            let mut slc = buf(statebytes);
            let mut slr = buf(statebytes);
            unsafe {
                iplc(slc.as_mut_ptr(), header.as_ptr(), k.as_ptr());
                iplr(slr.as_mut_ptr(), header.as_ptr(), k.as_ptr());
            }

            for i in 0..nmsg {
                let ml = MLENS[rng.below(MLENS.len())];
                let al = ADLENS[rng.below(ADLENS.len())];
                let m = rng.bytes(ml);
                let ad = rng.bytes(al.max(1));
                let adp: *const u8 = if al == 0 && i % 2 == 0 {
                    std::ptr::null()
                } else {
                    ad.as_ptr()
                };
                // cycle deterministically through all four tags, plus an
                // explicit rekey in the middle of longer streams
                let tag = match (i + trial) % 4 {
                    0 => t_msg,
                    1 => t_push,
                    2 => t_rekey,
                    _ => t_final,
                };
                if nmsg >= 10 && i == nmsg / 2 {
                    unsafe {
                        rkc(spc.as_mut_ptr());
                        rkr(spr.as_mut_ptr());
                        rkc(slc.as_mut_ptr());
                        rkr(slr.as_mut_ptr());
                    }
                    same_bytes("secretstream state after rekey", &spc, &spr);
                }

                for use_clen in [true, false] {
                    // push must be applied only once per state, so snapshot
                    let mut spc2 = spc.clone();
                    let mut spr2 = spr.clone();
                    let mut ca = buf(ml + abytes + 8);
                    let mut cb = buf(ml + abytes + 8);
                    let mut lc: c_ulonglong = 0xDEAD;
                    let mut lr: c_ulonglong = 0xDEAD;
                    unsafe {
                        same_ret(
                            "push",
                            pushc(
                                spc2.as_mut_ptr(),
                                ca.as_mut_ptr(),
                                if use_clen { &mut lc } else { std::ptr::null_mut() },
                                m.as_ptr(),
                                ml as c_ulonglong,
                                adp,
                                al as c_ulonglong,
                                tag,
                            ),
                            pushr(
                                spr2.as_mut_ptr(),
                                cb.as_mut_ptr(),
                                if use_clen { &mut lr } else { std::ptr::null_mut() },
                                m.as_ptr(),
                                ml as c_ulonglong,
                                adp,
                                al as c_ulonglong,
                                tag,
                            ),
                        );
                    }
                    same_bytes(&format!("push ml={ml} al={al} tag={tag}"), &ca, &cb);
                    same_bytes("push state", &spc2, &spr2);
                    assert_eq!(lc, lr, "push clen");
                    if !use_clen {
                        // now commit this push to the real state and pull it
                        spc = spc2;
                        spr = spr2;
                        // Each pull variant must start from the SAME pre-pull
                        // state; only one of them advances the real state.
                        let slc_base = slc.clone();
                        let slr_base = slr.clone();
                        let mut advanced: Option<(Vec<u8>, Vec<u8>)> = None;
                        // NOTE: `m` MUST be non-NULL here: unlike the AEAD
                        // decrypt_detached functions, the C pull() passes `m`
                        // straight to crypto_stream_chacha20_ietf_xor_ic(), so
                        // m == NULL with mlen > 0 is UB in the C too.
                        for (use_m, use_mlen, use_tag) in
                            [(true, true, true), (true, false, false), (true, false, true)]
                        {
                            let mut slc2 = slc_base.clone();
                            let mut slr2 = slr_base.clone();
                            let mut ma = buf(ml + 8);
                            let mut mb = buf(ml + 8);
                            let mut xc: c_ulonglong = 0xDEAD;
                            let mut xr: c_ulonglong = 0xDEAD;
                            let mut tc: u8 = 0xEE;
                            let mut tr: u8 = 0xEE;
                            unsafe {
                                same_ret(
                                    "pull",
                                    pullc(
                                        slc2.as_mut_ptr(),
                                        if use_m { ma.as_mut_ptr() } else { std::ptr::null_mut() },
                                        if use_mlen { &mut xc } else { std::ptr::null_mut() },
                                        if use_tag { &mut tc } else { std::ptr::null_mut() },
                                        ca.as_ptr(),
                                        (ml + abytes) as c_ulonglong,
                                        adp,
                                        al as c_ulonglong,
                                    ),
                                    pullr(
                                        slr2.as_mut_ptr(),
                                        if use_m { mb.as_mut_ptr() } else { std::ptr::null_mut() },
                                        if use_mlen { &mut xr } else { std::ptr::null_mut() },
                                        if use_tag { &mut tr } else { std::ptr::null_mut() },
                                        cb.as_ptr(),
                                        (ml + abytes) as c_ulonglong,
                                        adp,
                                        al as c_ulonglong,
                                    ),
                                );
                            }
                            same_bytes(&format!("pull ml={ml} tag={tag}"), &ma, &mb);
                            same_bytes("pull state", &slc2, &slr2);
                            assert_eq!(xc, xr, "pull mlen");
                            assert_eq!(tc, tr, "pull tag");
                            if use_tag {
                                assert_eq!(tc, tag, "pull tag round-trip");
                            }
                            if use_m {
                                same_bytes("secretstream round-trip", &m, &ma[..ml]);
                            }
                            if use_m && use_mlen && use_tag {
                                advanced = Some((slc2, slr2));
                            }
                        }
                        let (a, b) = advanced.expect("one pull variant advances the state");
                        slc = a;
                        slr = b;
                    }
                }
            }
        }
    }
    // keygen length only
    let (kc, kr) = pair::<unsafe extern "C" fn(*mut u8)>(&format!("{p}_keygen"));
    let mut x = buf(keybytes);
    unsafe {
        kc(x.as_mut_ptr());
        kr(x.as_mut_ptr());
    }
}

// ============================================================ box

type BoxEasyF = unsafe extern "C" fn(
    *mut u8,
    *const u8,
    c_ulonglong,
    *const u8,
    *const u8,
    *const u8,
) -> c_int;
type BoxDetF = unsafe extern "C" fn(
    *mut u8,
    *mut u8,
    *const u8,
    c_ulonglong,
    *const u8,
    *const u8,
    *const u8,
) -> c_int;
type BoxOpenDetF = unsafe extern "C" fn(
    *mut u8,
    *const u8,
    *const u8,
    c_ulonglong,
    *const u8,
    *const u8,
    *const u8,
) -> c_int;
type BoxAfternmF =
    unsafe extern "C" fn(*mut u8, *const u8, c_ulonglong, *const u8, *const u8) -> c_int;
type BoxDetAfternmF =
    unsafe extern "C" fn(*mut u8, *mut u8, *const u8, c_ulonglong, *const u8, *const u8) -> c_int;
type BoxOpenDetAfternmF =
    unsafe extern "C" fn(*mut u8, *const u8, *const u8, c_ulonglong, *const u8, *const u8) -> c_int;

fn keypair_deterministic(
    p: &str,
    seed: &[u8],
) -> (Vec<u8>, Vec<u8>) {
    let (c, _r) = pair::<unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> c_int>(&format!(
        "{p}_seed_keypair"
    ));
    let mut pk = vec![0u8; 32];
    let mut sk = vec![0u8; 32];
    unsafe {
        assert_eq!(c(pk.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr()), 0);
    }
    (pk, sk)
}

fn box_family(p: &str, has_easy: bool) {
    let mac = {
        let (c, r) = pair::<unsafe extern "C" fn() -> Sz>(&format!("{p}_macbytes"));
        unsafe {
            assert_eq!(c(), r());
            c()
        }
    };
    let nonce = {
        let (c, r) = pair::<unsafe extern "C" fn() -> Sz>(&format!("{p}_noncebytes"));
        unsafe {
            assert_eq!(c(), r());
            c()
        }
    };
    let beforenm = {
        let (c, r) = pair::<unsafe extern "C" fn() -> Sz>(&format!("{p}_beforenmbytes"));
        unsafe {
            assert_eq!(c(), r());
            c()
        }
    };

    // seed_keypair must be bit-identical in C and Rust
    let (skc, skr) = pair::<unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> c_int>(&format!(
        "{p}_seed_keypair"
    ));
    let mut rng = Rng::new(SEED ^ p.len() as u64);
    for _ in 0..20 {
        let seed = rng.bytes(32);
        let mut pa = buf(32);
        let mut sa = buf(32);
        let mut pb = buf(32);
        let mut sb = buf(32);
        unsafe {
            same_ret(
                &format!("{p}_seed_keypair"),
                skc(pa.as_mut_ptr(), sa.as_mut_ptr(), seed.as_ptr()),
                skr(pb.as_mut_ptr(), sb.as_mut_ptr(), seed.as_ptr()),
            );
        }
        same_bytes(&format!("{p}_seed_keypair pk"), &pa, &pb);
        same_bytes(&format!("{p}_seed_keypair sk"), &sa, &sb);
    }
    // keypair: nondeterministic, check it is self-consistent
    let (kpc, kpr) =
        pair::<unsafe extern "C" fn(*mut u8, *mut u8) -> c_int>(&format!("{p}_keypair"));
    let mut pa = buf(32);
    let mut sa = buf(32);
    unsafe {
        same_ret(&format!("{p}_keypair"), kpc(pa.as_mut_ptr(), sa.as_mut_ptr()), kpr(pa.as_mut_ptr(), sa.as_mut_ptr()));
    }

    // beforenm
    let (bc, br) =
        pair::<unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> c_int>(&format!("{p}_beforenm"));

    for trial in 0..8u64 {
        let seed_a = rng.bytes(32);
        let seed_b = rng.bytes(32);
        let (pk_a, sk_a) = keypair_deterministic(p, &seed_a);
        let (pk_b, sk_b) = keypair_deterministic(p, &seed_b);

        let mut ka = buf(beforenm);
        let mut kb = buf(beforenm);
        unsafe {
            same_ret(
                &format!("{p}_beforenm"),
                bc(ka.as_mut_ptr(), pk_b.as_ptr(), sk_a.as_ptr()),
                br(kb.as_mut_ptr(), pk_b.as_ptr(), sk_a.as_ptr()),
            );
        }
        same_bytes(&format!("{p}_beforenm"), &ka, &kb);

        for &ml in MLENS {
            let n = rng.bytes(nonce);
            let m = rng.bytes(ml);

            if has_easy {
                let (ec, er) = pair::<BoxEasyF>(&format!("{p}_easy"));
                let (oc, or) = pair::<BoxEasyF>(&format!("{p}_open_easy"));
                let mut ca = buf(ml + mac + 8);
                let mut cb = buf(ml + mac + 8);
                unsafe {
                    same_ret(
                        &format!("{p}_easy"),
                        ec(ca.as_mut_ptr(), m.as_ptr(), ml as c_ulonglong, n.as_ptr(), pk_b.as_ptr(), sk_a.as_ptr()),
                        er(cb.as_mut_ptr(), m.as_ptr(), ml as c_ulonglong, n.as_ptr(), pk_b.as_ptr(), sk_a.as_ptr()),
                    );
                }
                same_bytes(&format!("{p}_easy ml={ml}"), &ca, &cb);
                let mut ma = buf(ml + 8);
                let mut mb = buf(ml + 8);
                unsafe {
                    same_ret(
                        &format!("{p}_open_easy"),
                        oc(ma.as_mut_ptr(), ca.as_ptr(), (ml + mac) as c_ulonglong, n.as_ptr(), pk_a.as_ptr(), sk_b.as_ptr()),
                        or(mb.as_mut_ptr(), cb.as_ptr(), (ml + mac) as c_ulonglong, n.as_ptr(), pk_a.as_ptr(), sk_b.as_ptr()),
                    );
                }
                same_bytes(&format!("{p}_open_easy ml={ml}"), &ma, &mb);
                same_bytes(&format!("{p} round-trip ml={ml}"), &m, &ma[..ml]);

                // detached
                let (dc, dr) = pair::<BoxDetF>(&format!("{p}_detached"));
                let (odc, odr) = pair::<BoxOpenDetF>(&format!("{p}_open_detached"));
                let mut ca = buf(ml + 8);
                let mut cb = buf(ml + 8);
                let mut mca = buf(mac);
                let mut mcb = buf(mac);
                unsafe {
                    same_ret(
                        &format!("{p}_detached"),
                        dc(ca.as_mut_ptr(), mca.as_mut_ptr(), m.as_ptr(), ml as c_ulonglong, n.as_ptr(), pk_b.as_ptr(), sk_a.as_ptr()),
                        dr(cb.as_mut_ptr(), mcb.as_mut_ptr(), m.as_ptr(), ml as c_ulonglong, n.as_ptr(), pk_b.as_ptr(), sk_a.as_ptr()),
                    );
                }
                same_bytes(&format!("{p}_detached c ml={ml}"), &ca, &cb);
                same_bytes(&format!("{p}_detached mac ml={ml}"), &mca, &mcb);
                let mut ma = buf(ml + 8);
                let mut mb = buf(ml + 8);
                unsafe {
                    same_ret(
                        &format!("{p}_open_detached"),
                        odc(ma.as_mut_ptr(), ca.as_ptr(), mca.as_ptr(), ml as c_ulonglong, n.as_ptr(), pk_a.as_ptr(), sk_b.as_ptr()),
                        odr(mb.as_mut_ptr(), cb.as_ptr(), mcb.as_ptr(), ml as c_ulonglong, n.as_ptr(), pk_a.as_ptr(), sk_b.as_ptr()),
                    );
                }
                same_bytes(&format!("{p}_open_detached ml={ml}"), &ma, &mb);

                // afternm variants
                let (ac, ar) = pair::<BoxAfternmF>(&format!("{p}_easy_afternm"));
                let (aoc, aor) = pair::<BoxAfternmF>(&format!("{p}_open_easy_afternm"));
                let mut ca = buf(ml + mac + 8);
                let mut cb = buf(ml + mac + 8);
                unsafe {
                    same_ret(
                        &format!("{p}_easy_afternm"),
                        ac(ca.as_mut_ptr(), m.as_ptr(), ml as c_ulonglong, n.as_ptr(), ka.as_ptr()),
                        ar(cb.as_mut_ptr(), m.as_ptr(), ml as c_ulonglong, n.as_ptr(), kb.as_ptr()),
                    );
                }
                same_bytes(&format!("{p}_easy_afternm ml={ml}"), &ca, &cb);
                let mut ma = buf(ml + 8);
                let mut mb = buf(ml + 8);
                unsafe {
                    same_ret(
                        &format!("{p}_open_easy_afternm"),
                        aoc(ma.as_mut_ptr(), ca.as_ptr(), (ml + mac) as c_ulonglong, n.as_ptr(), ka.as_ptr()),
                        aor(mb.as_mut_ptr(), cb.as_ptr(), (ml + mac) as c_ulonglong, n.as_ptr(), kb.as_ptr()),
                    );
                }
                same_bytes(&format!("{p}_open_easy_afternm ml={ml}"), &ma, &mb);

                let (dac, dar) = pair::<BoxDetAfternmF>(&format!("{p}_detached_afternm"));
                let (odac, odar) =
                    pair::<BoxOpenDetAfternmF>(&format!("{p}_open_detached_afternm"));
                let mut ca = buf(ml + 8);
                let mut cb = buf(ml + 8);
                let mut mca = buf(mac);
                let mut mcb = buf(mac);
                unsafe {
                    same_ret(
                        &format!("{p}_detached_afternm"),
                        dac(ca.as_mut_ptr(), mca.as_mut_ptr(), m.as_ptr(), ml as c_ulonglong, n.as_ptr(), ka.as_ptr()),
                        dar(cb.as_mut_ptr(), mcb.as_mut_ptr(), m.as_ptr(), ml as c_ulonglong, n.as_ptr(), kb.as_ptr()),
                    );
                    same_bytes(&format!("{p}_detached_afternm ml={ml}"), &ca, &cb);
                    same_bytes(&format!("{p}_detached_afternm mac"), &mca, &mcb);
                    let mut ma = buf(ml + 8);
                    let mut mb = buf(ml + 8);
                    same_ret(
                        &format!("{p}_open_detached_afternm"),
                        odac(ma.as_mut_ptr(), ca.as_ptr(), mca.as_ptr(), ml as c_ulonglong, n.as_ptr(), ka.as_ptr()),
                        odar(mb.as_mut_ptr(), cb.as_ptr(), mcb.as_ptr(), ml as c_ulonglong, n.as_ptr(), kb.as_ptr()),
                    );
                    same_bytes(&format!("{p}_open_detached_afternm ml={ml}"), &ma, &mb);
                }
            }

            // Padded low-level form (rows 97-98 / 103): mlen >= 32 with the
            // leading 32 bytes zeroed.
            if libs().has(p) {
                let (lc, lr) = pair::<BoxEasyF>(p);
                let (loc, lor) = pair::<BoxEasyF>(&format!("{p}_open"));
                let body = ml;
                let pml = 32 + body;
                let mut pm = vec![0u8; 32];
                pm.extend(m.iter().copied());
                let mut ca = buf(pml + 8);
                let mut cb = buf(pml + 8);
                unsafe {
                    same_ret(
                        &format!("{p} padded"),
                        lc(ca.as_mut_ptr(), pm.as_ptr(), pml as c_ulonglong, n.as_ptr(), pk_b.as_ptr(), sk_a.as_ptr()),
                        lr(cb.as_mut_ptr(), pm.as_ptr(), pml as c_ulonglong, n.as_ptr(), pk_b.as_ptr(), sk_a.as_ptr()),
                    );
                }
                same_bytes(&format!("{p} padded ml={pml}"), &ca, &cb);
                let mut ma = buf(pml + 8);
                let mut mb = buf(pml + 8);
                unsafe {
                    same_ret(
                        &format!("{p}_open padded"),
                        loc(ma.as_mut_ptr(), ca.as_ptr(), pml as c_ulonglong, n.as_ptr(), pk_a.as_ptr(), sk_b.as_ptr()),
                        lor(mb.as_mut_ptr(), cb.as_ptr(), pml as c_ulonglong, n.as_ptr(), pk_a.as_ptr(), sk_b.as_ptr()),
                    );
                }
                same_bytes(&format!("{p}_open padded ml={pml}"), &ma, &mb);
                same_bytes(&format!("{p} padded round-trip"), &m, &ma[32..pml]);

                // padded afternm
                if libs().has(&format!("{p}_afternm")) {
                    let (ac, ar) = pair::<BoxAfternmF>(&format!("{p}_afternm"));
                    let (aoc, aor) = pair::<BoxAfternmF>(&format!("{p}_open_afternm"));
                    let mut ca = buf(pml + 8);
                    let mut cb = buf(pml + 8);
                    unsafe {
                        same_ret(
                            &format!("{p}_afternm"),
                            ac(ca.as_mut_ptr(), pm.as_ptr(), pml as c_ulonglong, n.as_ptr(), ka.as_ptr()),
                            ar(cb.as_mut_ptr(), pm.as_ptr(), pml as c_ulonglong, n.as_ptr(), kb.as_ptr()),
                        );
                    }
                    same_bytes(&format!("{p}_afternm ml={pml}"), &ca, &cb);
                    let mut ma = buf(pml + 8);
                    let mut mb = buf(pml + 8);
                    unsafe {
                        same_ret(
                            &format!("{p}_open_afternm"),
                            aoc(ma.as_mut_ptr(), ca.as_ptr(), pml as c_ulonglong, n.as_ptr(), ka.as_ptr()),
                            aor(mb.as_mut_ptr(), cb.as_ptr(), pml as c_ulonglong, n.as_ptr(), kb.as_ptr()),
                        );
                    }
                    same_bytes(&format!("{p}_open_afternm ml={pml}"), &ma, &mb);
                }
            }
        }
        let _ = trial;
    }

    // seal / seal_open (row 96 / 102): seal uses a random ephemeral key, so we
    // cross-open: C must open Rust's seal and vice-versa.
    let sealname = format!("{p}_seal");
    if libs().has(&sealname) {
        let sealbytes = {
            let (c, r) = pair::<unsafe extern "C" fn() -> Sz>(&format!("{p}_sealbytes"));
            unsafe {
                assert_eq!(c(), r());
                c()
            }
        };
        let (sc, sr) = pair::<unsafe extern "C" fn(*mut u8, *const u8, c_ulonglong, *const u8) -> c_int>(
            &sealname,
        );
        let (soc, sor) = pair::<
            unsafe extern "C" fn(*mut u8, *const u8, c_ulonglong, *const u8, *const u8) -> c_int,
        >(&format!("{p}_seal_open"));
        for &ml in MLENS {
            let seed = rng.bytes(32);
            let (pk, sk) = keypair_deterministic(p, &seed);
            let m = rng.bytes(ml);
            let mut ca = buf(ml + sealbytes);
            let mut cb = buf(ml + sealbytes);
            unsafe {
                same_ret(
                    &sealname,
                    sc(ca.as_mut_ptr(), m.as_ptr(), ml as c_ulonglong, pk.as_ptr()),
                    sr(cb.as_mut_ptr(), m.as_ptr(), ml as c_ulonglong, pk.as_ptr()),
                );
            }
            // cross-open both sealed blobs with both implementations
            for (label, blob) in [("C-seal", &ca), ("Rust-seal", &cb)] {
                let mut ma = buf(ml + 8);
                let mut mb = buf(ml + 8);
                unsafe {
                    same_ret(
                        &format!("{p}_seal_open {label}"),
                        soc(ma.as_mut_ptr(), blob.as_ptr(), (ml + sealbytes) as c_ulonglong, pk.as_ptr(), sk.as_ptr()),
                        sor(mb.as_mut_ptr(), blob.as_ptr(), (ml + sealbytes) as c_ulonglong, pk.as_ptr(), sk.as_ptr()),
                    );
                }
                same_bytes(&format!("{p}_seal_open {label} ml={ml}"), &ma, &mb);
                same_bytes(&format!("{p} seal round-trip {label}"), &m, &ma[..ml]);
            }
        }
    }

    for s in ["publickeybytes", "secretkeybytes", "seedbytes", "zerobytes", "boxzerobytes", "messagebytes_max"] {
        let g = format!("{p}_{s}");
        if libs().has(&g) {
            let (c, r) = pair::<unsafe extern "C" fn() -> Sz>(&g);
            unsafe { assert_eq!(c(), r(), "{g}") };
        }
    }
}

#[test]
fn rows91_99_crypto_box() {
    box_family("crypto_box", true);
    box_family("crypto_box_curve25519xsalsa20poly1305", false);
    let (c, r) = pair::<unsafe extern "C" fn() -> *const c_char>("crypto_box_primitive");
    unsafe {
        let a = std::ffi::CStr::from_ptr(c()).to_bytes().to_vec();
        let b = std::ffi::CStr::from_ptr(r()).to_bytes().to_vec();
        same_bytes("crypto_box_primitive", &a, &b);
    }
}

#[test]
fn rows100_103_box_curve25519xchacha20poly1305() {
    box_family("crypto_box_curve25519xchacha20poly1305", true);
}
