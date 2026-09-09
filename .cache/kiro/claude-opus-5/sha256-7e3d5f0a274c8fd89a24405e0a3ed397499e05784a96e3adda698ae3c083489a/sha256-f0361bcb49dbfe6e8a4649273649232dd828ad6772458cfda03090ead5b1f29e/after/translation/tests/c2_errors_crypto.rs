//! Phase C — error-path differential tests, ERRORS.md groups 2 & 3
//! (AEAD, secretbox, secretstream, box, stream; scalarmult, core, sign, kx, kem)

mod common;
use common::*;
use std::os::raw::{c_char, c_int, c_ulonglong};

type Sz = usize;

macro_rules! cmp2 {
    ($ctx:expr, $cexpr:expr, $rexpr:expr) => {{
        let ctx: &str = &$ctx;
        set_errno(0);
        let a: i32 = unsafe { $cexpr };
        let ea = errno();
        set_errno(0);
        let b: i32 = unsafe { $rexpr };
        let eb = errno();
        assert_eq!(a, b, "{}: return differs (C={} Rust={})", ctx, a, b);
        assert_eq!(ea, eb, "{}: errno differs (C={} Rust={})", ctx, ea, eb);
        a
    }};
}

/// Compare how both libraries terminate when the same call is made, by running
/// each in a forked child. Used for `sodium_misuse()` rows, which `abort()`.
#[track_caller]
fn cmp_abort(ctx: &str, cf: impl FnOnce(), rf: impl FnOnce()) {
    let oc = run_forked(cf);
    let or = run_forked(rf);
    assert_eq!(oc, or, "{ctx}: termination differs (C={oc:?} Rust={or:?})");
    assert_eq!(oc, Outcome::Signaled(libc::SIGABRT), "{ctx}: expected SIGABRT");
}

// ============================================================ AEAD

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

fn g(name: &str) -> usize {
    let (c, r) = pair::<unsafe extern "C" fn() -> Sz>(name);
    unsafe {
        assert_eq!(c(), r(), "{name}");
        c()
    }
}

/// rows 52/55/59/64/69 (clen < ABYTES), 53/54/56/57/60/61/66/71 (MAC mismatch)
fn aead_error_family(p: &str) {
    let key = g(&format!("{p}_keybytes"));
    let npub = g(&format!("{p}_npubbytes"));
    let ab = g(&format!("{p}_abytes"));
    let maxm = g(&format!("{p}_messagebytes_max"));

    let (ec, _) = pair::<EncF>(&format!("{p}_encrypt"));
    let (dc, dr) = pair::<DecF>(&format!("{p}_decrypt"));
    let (edc, _) = pair::<EncDetF>(&format!("{p}_encrypt_detached"));
    let (ddc, ddr) = pair::<DecDetF>(&format!("{p}_decrypt_detached"));

    let mut rng = Rng::new(SEED ^ p.len() as u64);
    let k = rng.bytes(key);
    let np = rng.bytes(npub);
    let ad = rng.bytes(16);

    // clen strictly below ABYTES -> -1 without touching the detached path
    let junk = rng.bytes(ab + 64);
    for clen in 0..ab {
        for adl in [0usize, 16] {
            let mut ma = buf(64);
            let mut mb = buf(64);
            let mut la: c_ulonglong = 0xDEAD;
            let mut lb: c_ulonglong = 0xDEAD;
            let ret = cmp2!(
                &format!("{p}_decrypt clen={clen} adl={adl}"),
                dc(ma.as_mut_ptr(), &mut la, std::ptr::null_mut(), junk.as_ptr(), clen as c_ulonglong, ad.as_ptr(), adl as c_ulonglong, np.as_ptr(), k.as_ptr()),
                dr(mb.as_mut_ptr(), &mut lb, std::ptr::null_mut(), junk.as_ptr(), clen as c_ulonglong, ad.as_ptr(), adl as c_ulonglong, np.as_ptr(), k.as_ptr())
            );
            assert_eq!(ret, -1, "{p}_decrypt clen={clen} must be rejected");
            same_bytes(&format!("{p}_decrypt short clen={clen}"), &ma, &mb);
            assert_eq!(la, lb, "{p}_decrypt mlen on short clen");
        }
    }

    // MAC mismatch, with m != NULL and m == NULL
    for ml in [0usize, 1, 16, 17, 63, 64, 65, 200] {
        let m = rng.bytes(ml);
        for adl in [0usize, 1, 16, 17] {
            let mut ct = buf(ml + ab);
            let mut clen: c_ulonglong = 0;
            unsafe {
                ec(ct.as_mut_ptr(), &mut clen, m.as_ptr(), ml as c_ulonglong, ad.as_ptr(), adl as c_ulonglong, std::ptr::null(), np.as_ptr(), k.as_ptr());
            }
            // flip each byte of the tag, plus a ciphertext byte, plus wrong ad/nonce/key
            let mut variants: Vec<(String, Vec<u8>, usize, Vec<u8>, Vec<u8>)> = Vec::new();
            for i in 0..ab {
                let mut bad = ct.clone();
                bad[ml + i] ^= 0x01;
                variants.push((format!("tag@{i}"), bad, adl, np.clone(), k.clone()));
            }
            if ml > 0 {
                let mut bad = ct.clone();
                bad[0] ^= 0x80;
                variants.push(("ct@0".into(), bad, adl, np.clone(), k.clone()));
                let mut bad = ct.clone();
                bad[ml - 1] ^= 0x01;
                variants.push(("ct@last".into(), bad, adl, np.clone(), k.clone()));
            }
            variants.push(("wrong-ad-len".into(), ct.clone(), adl + 1, np.clone(), k.clone()));
            variants.push(("wrong-nonce".into(), ct.clone(), adl, rng.bytes(npub), k.clone()));
            variants.push(("wrong-key".into(), ct.clone(), adl, np.clone(), rng.bytes(key)));

            for (label, bad, badadl, bnp, bk) in variants {
                let badad = rng.bytes(badadl.max(1));
                for use_m in [true, false] {
                    // combined
                    let mut ma = buf(ml + 8);
                    let mut mb = buf(ml + 8);
                    let mut la: c_ulonglong = 0xDEAD;
                    let mut lb: c_ulonglong = 0xDEAD;
                    let ret = cmp2!(
                        &format!("{p}_decrypt {label} ml={ml}"),
                        dc(if use_m { ma.as_mut_ptr() } else { std::ptr::null_mut() }, &mut la, std::ptr::null_mut(), bad.as_ptr(), clen, badad.as_ptr(), badadl as c_ulonglong, bnp.as_ptr(), bk.as_ptr()),
                        dr(if use_m { mb.as_mut_ptr() } else { std::ptr::null_mut() }, &mut lb, std::ptr::null_mut(), bad.as_ptr(), clen, badad.as_ptr(), badadl as c_ulonglong, bnp.as_ptr(), bk.as_ptr())
                    );
                    assert_eq!(ret, -1, "{p}_decrypt {label} must be rejected");
                    same_bytes(&format!("{p}_decrypt {label} out"), &ma, &mb);
                    assert_eq!(la, lb, "{p}_decrypt {label} mlen");

                    // detached
                    let mut ma = buf(ml + 8);
                    let mut mb = buf(ml + 8);
                    let ret = cmp2!(
                        &format!("{p}_decrypt_detached {label} ml={ml}"),
                        ddc(if use_m { ma.as_mut_ptr() } else { std::ptr::null_mut() }, std::ptr::null_mut(), bad.as_ptr(), ml as c_ulonglong, bad[ml..].as_ptr(), badad.as_ptr(), badadl as c_ulonglong, bnp.as_ptr(), bk.as_ptr()),
                        ddr(if use_m { mb.as_mut_ptr() } else { std::ptr::null_mut() }, std::ptr::null_mut(), bad.as_ptr(), ml as c_ulonglong, bad[ml..].as_ptr(), badad.as_ptr(), badadl as c_ulonglong, bnp.as_ptr(), bk.as_ptr())
                    );
                    assert_eq!(ret, -1, "{p}_decrypt_detached {label} must be rejected");
                    same_bytes(&format!("{p}_decrypt_detached {label} out"), &ma, &mb);
                }
            }
        }
    }

    // rows 50/51/58/62/63/67/68: mlen > MESSAGEBYTES_MAX -> sodium_misuse().
    // The check is the FIRST statement, so nothing is dereferenced. Use only
    // increments that do not wrap past 2^64 (MESSAGEBYTES_MAX is SIZE_MAX-16
    // for some primitives, so MAX+1000 would wrap back into the valid range).
    let ec2 = ec.clone();
    let (er2, _unused) = (pair::<EncF>(&format!("{p}_encrypt")).1, ());
    for over in [1usize, 2, 16] {
        let Some(ml) = maxm.checked_add(over) else { continue };
        let ec3 = ec2.clone();
        let er3 = er2.clone();
        cmp_abort(
            &format!("{p}_encrypt mlen=MAX+{over}"),
            || {
                let k = vec![0u8; 64];
                let np = vec![0u8; 64];
                unsafe {
                    ec3(std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null(), ml as c_ulonglong, std::ptr::null(), 0, std::ptr::null(), np.as_ptr(), k.as_ptr())
                };
            },
            || {
                let k = vec![0u8; 64];
                let np = vec![0u8; 64];
                unsafe {
                    er3(std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null(), ml as c_ulonglong, std::ptr::null(), 0, std::ptr::null(), np.as_ptr(), k.as_ptr())
                };
            },
        );
    }
    // NOTE: the C checks MESSAGEBYTES_MAX only in `_encrypt`, NOT in
    // `_encrypt_detached`, for the (x)chacha20poly1305 primitives. Only the
    // aegis ones check both mlen and adlen in `_encrypt_detached`, so those
    // rows live in the aegis-specific test.
    let _ = &edc;
}

#[test]
fn aead_chacha20poly1305_errors() {
    aead_error_family("crypto_aead_chacha20poly1305");
}

#[test]
fn aead_chacha20poly1305_ietf_errors() {
    aead_error_family("crypto_aead_chacha20poly1305_ietf");
}

#[test]
fn aead_xchacha20poly1305_ietf_errors() {
    aead_error_family("crypto_aead_xchacha20poly1305_ietf");
}

/// rows 65/70: aegis `decrypt_detached` rejects oversized clen/adlen with -1
/// (NOT an abort), unlike the encrypt side.
#[test]
fn aead_aegis_errors() {
    for p in ["crypto_aead_aegis128l", "crypto_aead_aegis256"] {
        aead_error_family(p);
        let maxm = g(&format!("{p}_messagebytes_max"));
        let (ddc, ddr) = pair::<DecDetF>(&format!("{p}_decrypt_detached"));
        let k = vec![0u8; 32];
        let np = vec![0u8; 32];
        let mac = vec![0u8; 32];
        // rows 63/68: aegis encrypt_detached DOES check adlen -> abort
        let (edc, edr) = pair::<EncDetF>(&format!("{p}_encrypt_detached"));
        for over in [1usize, 2] {
            let Some(al) = maxm.checked_add(over) else { continue };
            let edc2 = edc.clone();
            let edr2 = edr.clone();
            cmp_abort(
                &format!("{p}_encrypt_detached adlen=MAX+{over}"),
                || {
                    let k = vec![0u8; 64];
                    let np = vec![0u8; 64];
                    let mut mac = vec![0u8; 64];
                    unsafe {
                        edc2(std::ptr::null_mut(), mac.as_mut_ptr(), std::ptr::null_mut(), std::ptr::null(), 0, std::ptr::null(), al as c_ulonglong, std::ptr::null(), np.as_ptr(), k.as_ptr())
                    };
                },
                || {
                    let k = vec![0u8; 64];
                    let np = vec![0u8; 64];
                    let mut mac = vec![0u8; 64];
                    unsafe {
                        edr2(std::ptr::null_mut(), mac.as_mut_ptr(), std::ptr::null_mut(), std::ptr::null(), 0, std::ptr::null(), al as c_ulonglong, std::ptr::null(), np.as_ptr(), k.as_ptr())
                    };
                },
            );
        }
        for (cl, al) in [
            (maxm + 1, 0usize),
            (0usize, maxm + 1),
            (maxm + 1, maxm + 1),
        ] {
            let ret = cmp2!(
                &format!("{p}_decrypt_detached cl={cl} al={al}"),
                ddc(std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null(), cl as c_ulonglong, mac.as_ptr(), std::ptr::null(), al as c_ulonglong, np.as_ptr(), k.as_ptr()),
                ddr(std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null(), cl as c_ulonglong, mac.as_ptr(), std::ptr::null(), al as c_ulonglong, np.as_ptr(), k.as_ptr())
            );
            assert_eq!(ret, -1, "{p}_decrypt_detached oversized must return -1");
        }
    }
}

// ======================================================== secretbox

type SbEasyF =
    unsafe extern "C" fn(*mut u8, *const u8, c_ulonglong, *const u8, *const u8) -> c_int;
type SbDetF =
    unsafe extern "C" fn(*mut u8, *mut u8, *const u8, c_ulonglong, *const u8, *const u8) -> c_int;
type SbOpenDetF =
    unsafe extern "C" fn(*mut u8, *const u8, *const u8, c_ulonglong, *const u8, *const u8) -> c_int;

/// rows 82-84, 88-90
fn secretbox_error_family(p: &str) {
    let mac = g(&format!("{p}_macbytes"));
    let nonce = g(&format!("{p}_noncebytes"));
    let keyb = g(&format!("{p}_keybytes"));
    let maxm = g(&format!("{p}_messagebytes_max"));

    let (ec, _) = pair::<SbEasyF>(&format!("{p}_easy"));
    let (oc, or) = pair::<SbEasyF>(&format!("{p}_open_easy"));
    let (dc, _) = pair::<SbDetF>(&format!("{p}_detached"));
    let (odc, odr) = pair::<SbOpenDetF>(&format!("{p}_open_detached"));

    let mut rng = Rng::new(SEED ^ 3);
    let k = rng.bytes(keyb);
    let n = rng.bytes(nonce);

    // row 83/89: clen < MACBYTES
    let junk = rng.bytes(mac + 64);
    for clen in 0..mac {
        let mut ma = buf(64);
        let mut mb = buf(64);
        let ret = cmp2!(
            &format!("{p}_open_easy clen={clen}"),
            oc(ma.as_mut_ptr(), junk.as_ptr(), clen as c_ulonglong, n.as_ptr(), k.as_ptr()),
            or(mb.as_mut_ptr(), junk.as_ptr(), clen as c_ulonglong, n.as_ptr(), k.as_ptr())
        );
        assert_eq!(ret, -1, "{p}_open_easy clen={clen} must be rejected");
        same_bytes(&format!("{p}_open_easy clen={clen}"), &ma, &mb);
    }

    // rows 84/90: verify failure
    for ml in [0usize, 1, 16, 32, 33, 64, 200] {
        let m = rng.bytes(ml);
        let mut ct = buf(ml + mac);
        unsafe {
            ec(ct.as_mut_ptr(), m.as_ptr(), ml as c_ulonglong, n.as_ptr(), k.as_ptr());
        }
        let mut det = buf(ml);
        let mut dmac = buf(mac);
        unsafe {
            dc(det.as_mut_ptr(), dmac.as_mut_ptr(), m.as_ptr(), ml as c_ulonglong, n.as_ptr(), k.as_ptr());
        }
        for i in 0..(ml + mac) {
            let mut bad = ct.clone();
            bad[i] ^= 0x01;
            let mut ma = buf(ml + 8);
            let mut mb = buf(ml + 8);
            let ret = cmp2!(
                &format!("{p}_open_easy corrupt@{i}"),
                oc(ma.as_mut_ptr(), bad.as_ptr(), (ml + mac) as c_ulonglong, n.as_ptr(), k.as_ptr()),
                or(mb.as_mut_ptr(), bad.as_ptr(), (ml + mac) as c_ulonglong, n.as_ptr(), k.as_ptr())
            );
            assert_eq!(ret, -1, "{p}_open_easy corrupt@{i}");
            same_bytes(&format!("{p}_open_easy corrupt@{i}"), &ma, &mb);
        }
        for i in 0..mac {
            let mut bm = dmac.clone();
            bm[i] ^= 0x80;
            for use_m in [true, false] {
                let mut ma = buf(ml + 8);
                let mut mb = buf(ml + 8);
                let ret = cmp2!(
                    &format!("{p}_open_detached bad mac@{i}"),
                    odc(if use_m { ma.as_mut_ptr() } else { std::ptr::null_mut() }, det.as_ptr(), bm.as_ptr(), ml as c_ulonglong, n.as_ptr(), k.as_ptr()),
                    odr(if use_m { mb.as_mut_ptr() } else { std::ptr::null_mut() }, det.as_ptr(), bm.as_ptr(), ml as c_ulonglong, n.as_ptr(), k.as_ptr())
                );
                assert_eq!(ret, -1, "{p}_open_detached bad mac@{i}");
                same_bytes(&format!("{p}_open_detached bad mac@{i}"), &ma, &mb);
            }
        }
        // wrong key / wrong nonce
        let k2 = rng.bytes(keyb);
        let n2 = rng.bytes(nonce);
        for (label, kk, nn) in [("wrong-key", &k2, &n), ("wrong-nonce", &k, &n2)] {
            let mut ma = buf(ml + 8);
            let mut mb = buf(ml + 8);
            let ret = cmp2!(
                &format!("{p}_open_easy {label}"),
                oc(ma.as_mut_ptr(), ct.as_ptr(), (ml + mac) as c_ulonglong, nn.as_ptr(), kk.as_ptr()),
                or(mb.as_mut_ptr(), ct.as_ptr(), (ml + mac) as c_ulonglong, nn.as_ptr(), kk.as_ptr())
            );
            assert_eq!(ret, -1, "{p}_open_easy {label}");
            same_bytes(&format!("{p}_open_easy {label}"), &ma, &mb);
        }
    }

    // rows 82/88: mlen > MESSAGEBYTES_MAX -> abort
    let (er, _) = (pair::<SbEasyF>(&format!("{p}_easy")).1, ());
    for over in [1usize, 2, 16] {
        let Some(ml) = maxm.checked_add(over) else { continue };
        let ec2 = ec.clone();
        let er2 = er.clone();
        cmp_abort(
            &format!("{p}_easy mlen=MAX+{over}"),
            || {
                let k = vec![0u8; 64];
                let n = vec![0u8; 64];
                unsafe { ec2(std::ptr::null_mut(), std::ptr::null(), ml as c_ulonglong, n.as_ptr(), k.as_ptr()) };
            },
            || {
                let k = vec![0u8; 64];
                let n = vec![0u8; 64];
                unsafe { er2(std::ptr::null_mut(), std::ptr::null(), ml as c_ulonglong, n.as_ptr(), k.as_ptr()) };
            },
        );
    }
}

#[test]
fn secretbox_errors() {
    secretbox_error_family("crypto_secretbox");
    secretbox_error_family("crypto_secretbox_xchacha20poly1305");
}

/// rows 85-87: padded low-level form rejects mlen/clen < 32
#[test]
fn secretbox_padded_lowlevel_errors() {
    // Only crypto_secretbox_xsalsa20poly1305 and the generic crypto_secretbox
    // expose the padded (NaCl) form; there is no
    // crypto_secretbox_xchacha20poly1305 / _open in this build.
    for p in ["crypto_secretbox_xsalsa20poly1305", "crypto_secretbox"] {
        assert!(libs().has(p) && libs().has(&format!("{p}_open")), "{p} padded form missing");
        let (ec, er) = pair::<SbEasyF>(p);
        let (oc, or) = pair::<SbEasyF>(&format!("{p}_open"));
        let mut rng = Rng::seeded();
        let k = rng.bytes(32);
        let n = rng.bytes(24);
        let m = vec![0u8; 128];
        for l in 0..32u64 {
            let mut a = buf(128);
            let mut b = buf(128);
            let ret = cmp2!(
                &format!("{p} mlen={l}"),
                ec(a.as_mut_ptr(), m.as_ptr(), l, n.as_ptr(), k.as_ptr()),
                er(b.as_mut_ptr(), m.as_ptr(), l, n.as_ptr(), k.as_ptr())
            );
            assert_eq!(ret, -1, "{p} mlen={l} must be rejected");
            same_bytes(&format!("{p} mlen={l}"), &a, &b);

            let mut a = buf(128);
            let mut b = buf(128);
            let ret = cmp2!(
                &format!("{p}_open clen={l}"),
                oc(a.as_mut_ptr(), m.as_ptr(), l, n.as_ptr(), k.as_ptr()),
                or(b.as_mut_ptr(), m.as_ptr(), l, n.as_ptr(), k.as_ptr())
            );
            assert_eq!(ret, -1, "{p}_open clen={l} must be rejected");
            same_bytes(&format!("{p}_open clen={l}"), &a, &b);
        }
        // row 87: verify failure on the padded form
        for body in [0usize, 1, 32, 100] {
            let pml = 32 + body;
            let mut pm = vec![0u8; 32];
            pm.extend(rng.bytes(body));
            let mut ct = buf(pml);
            unsafe {
                ec(ct.as_mut_ptr(), pm.as_ptr(), pml as c_ulonglong, n.as_ptr(), k.as_ptr());
            }
            for i in 16..pml {
                let mut bad = ct.clone();
                bad[i] ^= 0x01;
                let mut a = buf(pml + 8);
                let mut b = buf(pml + 8);
                let ret = cmp2!(
                    &format!("{p}_open corrupt@{i}"),
                    oc(a.as_mut_ptr(), bad.as_ptr(), pml as c_ulonglong, n.as_ptr(), k.as_ptr()),
                    or(b.as_mut_ptr(), bad.as_ptr(), pml as c_ulonglong, n.as_ptr(), k.as_ptr())
                );
                assert_eq!(ret, -1, "{p}_open corrupt@{i}");
                same_bytes(&format!("{p}_open corrupt@{i}"), &a, &b);
            }
        }
    }
}

// ==================================================== secretstream

type SsPushF = unsafe extern "C" fn(
    *mut u8,
    *mut u8,
    *mut c_ulonglong,
    *const u8,
    c_ulonglong,
    *const u8,
    c_ulonglong,
    u8,
) -> c_int;
type SsPullF = unsafe extern "C" fn(
    *mut u8,
    *mut u8,
    *mut c_ulonglong,
    *mut u8,
    *const u8,
    c_ulonglong,
    *const u8,
    c_ulonglong,
) -> c_int;

/// rows 91-94
#[test]
fn secretstream_errors() {
    let p = "crypto_secretstream_xchacha20poly1305";
    let statebytes = g(&format!("{p}_statebytes"));
    let abytes = g(&format!("{p}_abytes"));
    let headerbytes = g(&format!("{p}_headerbytes"));
    let keybytes = g(&format!("{p}_keybytes"));
    let maxm = g(&format!("{p}_messagebytes_max"));

    let (ipc, ipr) = pair::<unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> c_int>(&format!(
        "{p}_init_push"
    ));
    let (iplc, iplr) = pair::<unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> c_int>(
        &format!("{p}_init_pull"),
    );
    let (pushc, _) = pair::<SsPushF>(&format!("{p}_push"));
    let (pullc, pullr) = pair::<SsPullF>(&format!("{p}_pull"));

    let mut rng = Rng::seeded();
    let k = rng.bytes(keybytes);
    let mut header = buf(headerbytes);
    let mut sp = buf(statebytes);
    unsafe {
        ipc(sp.as_mut_ptr(), header.as_mut_ptr(), k.as_ptr());
    }

    // row 92: inlen < ABYTES
    let junk = rng.bytes(abytes + 64);
    for inlen in 0..abytes {
        let mut sa = buf(statebytes);
        let mut sb = buf(statebytes);
        unsafe {
            iplc(sa.as_mut_ptr(), header.as_ptr(), k.as_ptr());
            iplr(sb.as_mut_ptr(), header.as_ptr(), k.as_ptr());
        }
        let mut ma = buf(64);
        let mut mb = buf(64);
        let mut la: c_ulonglong = 0xDEAD;
        let mut lb: c_ulonglong = 0xDEAD;
        let mut ta: u8 = 0xEE;
        let mut tb: u8 = 0xEE;
        let ret = cmp2!(
            &format!("{p}_pull inlen={inlen}"),
            pullc(sa.as_mut_ptr(), ma.as_mut_ptr(), &mut la, &mut ta, junk.as_ptr(), inlen as c_ulonglong, std::ptr::null(), 0),
            pullr(sb.as_mut_ptr(), mb.as_mut_ptr(), &mut lb, &mut tb, junk.as_ptr(), inlen as c_ulonglong, std::ptr::null(), 0)
        );
        assert_eq!(ret, -1, "{p}_pull inlen={inlen} must be rejected");
        same_bytes(&format!("{p}_pull inlen={inlen}"), &ma, &mb);
        assert_eq!(la, lb, "pull mlen on short input");
        assert_eq!(ta, tb, "pull tag on short input");
        same_bytes("pull state on short input", &sa, &sb);
    }

    // row 94: MAC mismatch, and wrong ad
    for ml in [0usize, 1, 16, 64, 200] {
        for tag in [0u8, 1, 2, 3] {
            let m = rng.bytes(ml);
            let ad = rng.bytes(16);
            let mut spx = sp.clone();
            let mut ct = buf(ml + abytes);
            unsafe {
                pushc(spx.as_mut_ptr(), ct.as_mut_ptr(), std::ptr::null_mut(), m.as_ptr(), ml as c_ulonglong, ad.as_ptr(), 16, tag);
            }
            for i in 0..(ml + abytes) {
                let mut bad = ct.clone();
                bad[i] ^= 0x01;
                let mut sa = buf(statebytes);
                let mut sb = buf(statebytes);
                unsafe {
                    iplc(sa.as_mut_ptr(), header.as_ptr(), k.as_ptr());
                    iplr(sb.as_mut_ptr(), header.as_ptr(), k.as_ptr());
                }
                let mut ma = buf(ml + 8);
                let mut mb = buf(ml + 8);
                let mut la: c_ulonglong = 0xDEAD;
                let mut lb: c_ulonglong = 0xDEAD;
                let mut ta: u8 = 0xEE;
                let mut tb: u8 = 0xEE;
                let ret = cmp2!(
                    &format!("{p}_pull corrupt@{i} ml={ml}"),
                    pullc(sa.as_mut_ptr(), ma.as_mut_ptr(), &mut la, &mut ta, bad.as_ptr(), (ml + abytes) as c_ulonglong, ad.as_ptr(), 16),
                    pullr(sb.as_mut_ptr(), mb.as_mut_ptr(), &mut lb, &mut tb, bad.as_ptr(), (ml + abytes) as c_ulonglong, ad.as_ptr(), 16)
                );
                assert_eq!(ret, -1, "{p}_pull corrupt@{i}");
                same_bytes(&format!("{p}_pull corrupt@{i} out"), &ma, &mb);
                assert_eq!(la, lb);
                assert_eq!(ta, tb);
                same_bytes("pull state after failure", &sa, &sb);
            }
            // wrong ad
            let bad_ad = rng.bytes(16);
            let mut sa = buf(statebytes);
            let mut sb = buf(statebytes);
            unsafe {
                iplc(sa.as_mut_ptr(), header.as_ptr(), k.as_ptr());
                iplr(sb.as_mut_ptr(), header.as_ptr(), k.as_ptr());
            }
            let mut ma = buf(ml + 8);
            let mut mb = buf(ml + 8);
            let ret = cmp2!(
                &format!("{p}_pull wrong ad ml={ml}"),
                pullc(sa.as_mut_ptr(), ma.as_mut_ptr(), std::ptr::null_mut(), std::ptr::null_mut(), ct.as_ptr(), (ml + abytes) as c_ulonglong, bad_ad.as_ptr(), 16),
                pullr(sb.as_mut_ptr(), mb.as_mut_ptr(), std::ptr::null_mut(), std::ptr::null_mut(), ct.as_ptr(), (ml + abytes) as c_ulonglong, bad_ad.as_ptr(), 16)
            );
            assert_eq!(ret, -1, "{p}_pull wrong ad");
            same_bytes(&format!("{p}_pull wrong ad out"), &ma, &mb);
            // wrong key (different init_pull key)
            let k2 = rng.bytes(keybytes);
            let mut sa = buf(statebytes);
            let mut sb = buf(statebytes);
            unsafe {
                iplc(sa.as_mut_ptr(), header.as_ptr(), k2.as_ptr());
                iplr(sb.as_mut_ptr(), header.as_ptr(), k2.as_ptr());
            }
            let mut ma = buf(ml + 8);
            let mut mb = buf(ml + 8);
            let ret = cmp2!(
                &format!("{p}_pull wrong key ml={ml}"),
                pullc(sa.as_mut_ptr(), ma.as_mut_ptr(), std::ptr::null_mut(), std::ptr::null_mut(), ct.as_ptr(), (ml + abytes) as c_ulonglong, ad.as_ptr(), 16),
                pullr(sb.as_mut_ptr(), mb.as_mut_ptr(), std::ptr::null_mut(), std::ptr::null_mut(), ct.as_ptr(), (ml + abytes) as c_ulonglong, ad.as_ptr(), 16)
            );
            assert_eq!(ret, -1, "{p}_pull wrong key");
        }
    }
    // row 91: push mlen > MESSAGEBYTES_MAX -> abort
    let (pushr, _) = (pair::<SsPushF>(&format!("{p}_push")).1, ());
    for over in [1usize, 2, 16] {
        let Some(ml) = maxm.checked_add(over) else { continue };
        let pc = pushc.clone();
        let pr = pushr.clone();
        let kk = k.clone();
        let kk2 = k.clone();
        cmp_abort(
            &format!("{p}_push mlen=MAX+{over}"),
            || {
                let mut st = vec![0u8; 128];
                let mut h = vec![0u8; 64];
                unsafe {
                    ipc(st.as_mut_ptr(), h.as_mut_ptr(), kk.as_ptr());
                    pc(st.as_mut_ptr(), std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null(), ml as c_ulonglong, std::ptr::null(), 0, 0)
                };
            },
            || {
                let mut st = vec![0u8; 128];
                let mut h = vec![0u8; 64];
                unsafe {
                    ipr(st.as_mut_ptr(), h.as_mut_ptr(), kk2.as_ptr());
                    pr(st.as_mut_ptr(), std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null(), ml as c_ulonglong, std::ptr::null(), 0, 0)
                };
            },
        );
    }
    // row 93: pull inlen - ABYTES > MESSAGEBYTES_MAX -> abort
    for over in [1usize, 5] {
        let Some(inlen) = maxm.checked_add(abytes).and_then(|x| x.checked_add(over)) else { continue };
        let pc = pullc.clone();
        let pr = pullr.clone();
        let kk = k.clone();
        let kk2 = k.clone();
        let hh = header.clone();
        let hh2 = header.clone();
        cmp_abort(
            &format!("{p}_pull inlen=MAX+ABYTES+{over}"),
            || {
                let mut st = vec![0u8; 128];
                unsafe {
                    iplc(st.as_mut_ptr(), hh.as_ptr(), kk.as_ptr());
                    pc(st.as_mut_ptr(), std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null(), inlen as c_ulonglong, std::ptr::null(), 0)
                };
            },
            || {
                let mut st = vec![0u8; 128];
                unsafe {
                    iplr(st.as_mut_ptr(), hh2.as_ptr(), kk2.as_ptr());
                    pr(st.as_mut_ptr(), std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null(), inlen as c_ulonglong, std::ptr::null(), 0)
                };
            },
        );
    }
}

// ============================================================ box

type BoxEasyF =
    unsafe extern "C" fn(*mut u8, *const u8, c_ulonglong, *const u8, *const u8, *const u8) -> c_int;
type BoxAfternmF =
    unsafe extern "C" fn(*mut u8, *const u8, c_ulonglong, *const u8, *const u8) -> c_int;

/// The 7 small-order / non-canonical curve25519 points from the C blocklist.
fn small_order_points() -> Vec<[u8; 32]> {
    vec![
        [0u8; 32],
        {
            let mut v = [0u8; 32];
            v[0] = 1;
            v
        },
        [
            0xe0, 0xeb, 0x7a, 0x7c, 0x3b, 0x41, 0xb8, 0xae, 0x16, 0x56, 0xe3, 0xfa, 0xf1, 0x9f,
            0xc4, 0x6a, 0xda, 0x09, 0x8d, 0xeb, 0x9c, 0x32, 0xb1, 0xfd, 0x86, 0x62, 0x05, 0x16,
            0x5f, 0x49, 0xb8, 0x00,
        ],
        [
            0x5f, 0x9c, 0x95, 0xbc, 0xa3, 0x50, 0x8c, 0x24, 0xb1, 0xd0, 0xb1, 0x55, 0x9c, 0x83,
            0xef, 0x5b, 0x04, 0x44, 0x5c, 0xc4, 0x58, 0x1c, 0x8e, 0x86, 0xd8, 0x22, 0x4e, 0xdd,
            0xd0, 0x9f, 0x11, 0x57,
        ],
        [
            0xec, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0x7f,
        ],
        [
            0xed, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0x7f,
        ],
        [
            0xee, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0x7f,
        ],
    ]
}

/// rows 95-116
fn box_error_family(p: &str, has_easy: bool) {
    let mac = g(&format!("{p}_macbytes"));
    let nonce = g(&format!("{p}_noncebytes"));
    let beforenm = g(&format!("{p}_beforenmbytes"));

    let (skc, _) = pair::<unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> c_int>(&format!(
        "{p}_seed_keypair"
    ));
    let mut rng = Rng::new(SEED ^ 11);
    let seed = rng.bytes(32);
    let mut pk = vec![0u8; 32];
    let mut sk = vec![0u8; 32];
    unsafe {
        skc(pk.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr());
    }
    let n = rng.bytes(nonce);

    // rows 99/100/103/108/111/112: beforenm rejects small-order peer keys, and
    // that failure must propagate identically through every wrapper.
    let (bc, br) = pair::<unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> c_int>(&format!(
        "{p}_beforenm"
    ));
    for (i, bad_pk) in small_order_points().iter().enumerate() {
        let mut ka = buf(beforenm);
        let mut kb = buf(beforenm);
        let ret = cmp2!(
            &format!("{p}_beforenm small-order #{i}"),
            bc(ka.as_mut_ptr(), bad_pk.as_ptr(), sk.as_ptr()),
            br(kb.as_mut_ptr(), bad_pk.as_ptr(), sk.as_ptr())
        );
        assert_eq!(ret, -1, "{p}_beforenm small-order #{i} must be rejected");
        same_bytes(&format!("{p}_beforenm small-order #{i}"), &ka, &kb);

        if has_easy {
            for (label, name) in [
                ("easy", format!("{p}_easy")),
                ("open_easy", format!("{p}_open_easy")),
            ] {
                let (c, r) = pair::<BoxEasyF>(&name);
                for ml in [0usize, 1, 64] {
                    let m = rng.bytes(ml + mac);
                    let mut a = buf(ml + mac + 8);
                    let mut b = buf(ml + mac + 8);
                    let len = if label == "open_easy" { ml + mac } else { ml };
                    let ret = cmp2!(
                        &format!("{name} small-order #{i} ml={ml}"),
                        c(a.as_mut_ptr(), m.as_ptr(), len as c_ulonglong, n.as_ptr(), bad_pk.as_ptr(), sk.as_ptr()),
                        r(b.as_mut_ptr(), m.as_ptr(), len as c_ulonglong, n.as_ptr(), bad_pk.as_ptr(), sk.as_ptr())
                    );
                    assert_eq!(ret, -1, "{name} small-order #{i}");
                    same_bytes(&format!("{name} small-order #{i}"), &a, &b);
                }
            }
            let (dc2, dr2) = pair::<
                unsafe extern "C" fn(*mut u8, *mut u8, *const u8, c_ulonglong, *const u8, *const u8, *const u8) -> c_int,
            >(&format!("{p}_detached"));
            let m = rng.bytes(64);
            let mut a = buf(64);
            let mut b = buf(64);
            let mut mca = buf(mac);
            let mut mcb = buf(mac);
            let ret = cmp2!(
                &format!("{p}_detached small-order #{i}"),
                dc2(a.as_mut_ptr(), mca.as_mut_ptr(), m.as_ptr(), 64, n.as_ptr(), bad_pk.as_ptr(), sk.as_ptr()),
                dr2(b.as_mut_ptr(), mcb.as_mut_ptr(), m.as_ptr(), 64, n.as_ptr(), bad_pk.as_ptr(), sk.as_ptr())
            );
            assert_eq!(ret, -1);
            let (odc, odr) = pair::<
                unsafe extern "C" fn(*mut u8, *const u8, *const u8, c_ulonglong, *const u8, *const u8, *const u8) -> c_int,
            >(&format!("{p}_open_detached"));
            let ret = cmp2!(
                &format!("{p}_open_detached small-order #{i}"),
                odc(a.as_mut_ptr(), m.as_ptr(), mca.as_ptr(), 64, n.as_ptr(), bad_pk.as_ptr(), sk.as_ptr()),
                odr(b.as_mut_ptr(), m.as_ptr(), mcb.as_ptr(), 64, n.as_ptr(), bad_pk.as_ptr(), sk.as_ptr())
            );
            assert_eq!(ret, -1);
        }
        // padded low-level form
        if libs().has(p) {
            let (lc, lr) = pair::<BoxEasyF>(p);
            let (loc, lor) = pair::<BoxEasyF>(&format!("{p}_open"));
            let m = vec![0u8; 128];
            let mut a = buf(128);
            let mut b = buf(128);
            let ret = cmp2!(
                &format!("{p} padded small-order #{i}"),
                lc(a.as_mut_ptr(), m.as_ptr(), 64, n.as_ptr(), bad_pk.as_ptr(), sk.as_ptr()),
                lr(b.as_mut_ptr(), m.as_ptr(), 64, n.as_ptr(), bad_pk.as_ptr(), sk.as_ptr())
            );
            assert_eq!(ret, -1);
            let ret = cmp2!(
                &format!("{p}_open padded small-order #{i}"),
                loc(a.as_mut_ptr(), m.as_ptr(), 64, n.as_ptr(), bad_pk.as_ptr(), sk.as_ptr()),
                lor(b.as_mut_ptr(), m.as_ptr(), 64, n.as_ptr(), bad_pk.as_ptr(), sk.as_ptr())
            );
            assert_eq!(ret, -1);
        }
    }

    // rows 101/102/113/114: clen < MACBYTES on the *_open_easy* wrappers
    if has_easy {
        let (oc, or) = pair::<BoxEasyF>(&format!("{p}_open_easy"));
        let (ac, ar) = pair::<BoxAfternmF>(&format!("{p}_open_easy_afternm"));
        let mut ka = buf(beforenm);
        unsafe {
            bc(ka.as_mut_ptr(), pk.as_ptr(), sk.as_ptr());
        }
        let junk = vec![0u8; mac + 64];
        for clen in 0..mac {
            let mut a = buf(64);
            let mut b = buf(64);
            let ret = cmp2!(
                &format!("{p}_open_easy clen={clen}"),
                oc(a.as_mut_ptr(), junk.as_ptr(), clen as c_ulonglong, n.as_ptr(), pk.as_ptr(), sk.as_ptr()),
                or(b.as_mut_ptr(), junk.as_ptr(), clen as c_ulonglong, n.as_ptr(), pk.as_ptr(), sk.as_ptr())
            );
            assert_eq!(ret, -1, "{p}_open_easy clen={clen}");
            same_bytes(&format!("{p}_open_easy clen={clen}"), &a, &b);
            let mut a = buf(64);
            let mut b = buf(64);
            let ret = cmp2!(
                &format!("{p}_open_easy_afternm clen={clen}"),
                ac(a.as_mut_ptr(), junk.as_ptr(), clen as c_ulonglong, n.as_ptr(), ka.as_ptr()),
                ar(b.as_mut_ptr(), junk.as_ptr(), clen as c_ulonglong, n.as_ptr(), ka.as_ptr())
            );
            assert_eq!(ret, -1, "{p}_open_easy_afternm clen={clen}");
            same_bytes(&format!("{p}_open_easy_afternm clen={clen}"), &a, &b);
        }
    }

    // rows 106/107: padded afternm rejects mlen/clen < 32
    if libs().has(&format!("{p}_afternm")) {
        let (ac, ar) = pair::<BoxAfternmF>(&format!("{p}_afternm"));
        let (oc, or) = pair::<BoxAfternmF>(&format!("{p}_open_afternm"));
        let mut ka = buf(beforenm);
        unsafe {
            bc(ka.as_mut_ptr(), pk.as_ptr(), sk.as_ptr());
        }
        let m = vec![0u8; 128];
        for l in 0..32u64 {
            let mut a = buf(128);
            let mut b = buf(128);
            let ret = cmp2!(
                &format!("{p}_afternm mlen={l}"),
                ac(a.as_mut_ptr(), m.as_ptr(), l, n.as_ptr(), ka.as_ptr()),
                ar(b.as_mut_ptr(), m.as_ptr(), l, n.as_ptr(), ka.as_ptr())
            );
            assert_eq!(ret, -1, "{p}_afternm mlen={l}");
            let mut a = buf(128);
            let mut b = buf(128);
            let ret = cmp2!(
                &format!("{p}_open_afternm clen={l}"),
                oc(a.as_mut_ptr(), m.as_ptr(), l, n.as_ptr(), ka.as_ptr()),
                or(b.as_mut_ptr(), m.as_ptr(), l, n.as_ptr(), ka.as_ptr())
            );
            assert_eq!(ret, -1, "{p}_open_afternm clen={l}");
        }
    }

    // rows 96/116: seal_open clen < SEALBYTES
    if libs().has(&format!("{p}_seal_open")) {
        let sealbytes = g(&format!("{p}_sealbytes"));
        let (soc, sor) = pair::<
            unsafe extern "C" fn(*mut u8, *const u8, c_ulonglong, *const u8, *const u8) -> c_int,
        >(&format!("{p}_seal_open"));
        let junk = vec![0u8; sealbytes + 64];
        for clen in 0..sealbytes {
            let mut a = buf(64);
            let mut b = buf(64);
            let ret = cmp2!(
                &format!("{p}_seal_open clen={clen}"),
                soc(a.as_mut_ptr(), junk.as_ptr(), clen as c_ulonglong, pk.as_ptr(), sk.as_ptr()),
                sor(b.as_mut_ptr(), junk.as_ptr(), clen as c_ulonglong, pk.as_ptr(), sk.as_ptr())
            );
            assert_eq!(ret, -1, "{p}_seal_open clen={clen}");
            same_bytes(&format!("{p}_seal_open clen={clen}"), &a, &b);
        }
        // corrupted sealed blob of a valid length
        let (sc, _) = pair::<unsafe extern "C" fn(*mut u8, *const u8, c_ulonglong, *const u8) -> c_int>(
            &format!("{p}_seal"),
        );
        for ml in [0usize, 1, 64] {
            let m = rng.bytes(ml);
            let mut ct = buf(ml + sealbytes);
            unsafe {
                sc(ct.as_mut_ptr(), m.as_ptr(), ml as c_ulonglong, pk.as_ptr());
            }
            for i in 0..(ml + sealbytes) {
                let mut bad = ct.clone();
                bad[i] ^= 0x01;
                let mut a = buf(ml + 8);
                let mut b = buf(ml + 8);
                let ret = cmp2!(
                    &format!("{p}_seal_open corrupt@{i}"),
                    soc(a.as_mut_ptr(), bad.as_ptr(), (ml + sealbytes) as c_ulonglong, pk.as_ptr(), sk.as_ptr()),
                    sor(b.as_mut_ptr(), bad.as_ptr(), (ml + sealbytes) as c_ulonglong, pk.as_ptr(), sk.as_ptr())
                );
                assert_eq!(ret, -1, "{p}_seal_open corrupt@{i}");
                same_bytes(&format!("{p}_seal_open corrupt@{i}"), &a, &b);
            }
        }
    }

    // rows 97/98/109/110/115: mlen > MESSAGEBYTES_MAX -> abort
    let maxm = g(&format!("{p}_messagebytes_max"));
    let mut abort_targets: Vec<String> = Vec::new();
    if has_easy {
        abort_targets.push(format!("{p}_easy"));
    }
    for name in abort_targets {
        let (c, r) = pair::<BoxEasyF>(&name);
        for over in [1usize, 2] {
            let Some(ml) = maxm.checked_add(over) else { continue };
            let c2 = c.clone();
            let r2 = r.clone();
            let pk2 = pk.clone();
            let sk2 = sk.clone();
            let pk3 = pk.clone();
            let sk3 = sk.clone();
            cmp_abort(
                &format!("{name} mlen=MAX+{over}"),
                || {
                    let n = vec![0u8; 64];
                    unsafe {
                        c2(std::ptr::null_mut(), std::ptr::null(), ml as c_ulonglong, n.as_ptr(), pk2.as_ptr(), sk2.as_ptr())
                    };
                },
                || {
                    let n = vec![0u8; 64];
                    unsafe {
                        r2(std::ptr::null_mut(), std::ptr::null(), ml as c_ulonglong, n.as_ptr(), pk3.as_ptr(), sk3.as_ptr())
                    };
                },
            );
        }
    }
    if has_easy {
        let name = format!("{p}_easy_afternm");
        let (c, r) = pair::<BoxAfternmF>(&name);
        let mut ka = vec![0u8; beforenm];
        unsafe {
            bc(ka.as_mut_ptr(), pk.as_ptr(), sk.as_ptr());
        }
        for over in [1usize, 2] {
            let Some(ml) = maxm.checked_add(over) else { continue };
            let c2 = c.clone();
            let r2 = r.clone();
            let k1 = ka.clone();
            let k2 = ka.clone();
            cmp_abort(
                &format!("{name} mlen=MAX+{over}"),
                || {
                    let n = vec![0u8; 64];
                    unsafe { c2(std::ptr::null_mut(), std::ptr::null(), ml as c_ulonglong, n.as_ptr(), k1.as_ptr()) };
                },
                || {
                    let n = vec![0u8; 64];
                    unsafe { r2(std::ptr::null_mut(), std::ptr::null(), ml as c_ulonglong, n.as_ptr(), k2.as_ptr()) };
                },
            );
        }
    }
    if libs().has(&format!("{p}_seal")) {
        let name = format!("{p}_seal");
        let (c, r) = pair::<unsafe extern "C" fn(*mut u8, *const u8, c_ulonglong, *const u8) -> c_int>(
            &name,
        );
        for over in [1usize] {
            let Some(ml) = maxm.checked_add(over) else { continue };
            let c2 = c.clone();
            let r2 = r.clone();
            let pk2 = pk.clone();
            let pk3 = pk.clone();
            cmp_abort(
                &format!("{name} mlen=MAX+{over}"),
                || unsafe {
                    c2(std::ptr::null_mut(), std::ptr::null(), ml as c_ulonglong, pk2.as_ptr());
                },
                || unsafe {
                    r2(std::ptr::null_mut(), std::ptr::null(), ml as c_ulonglong, pk3.as_ptr());
                },
            );
        }
    }
}

#[test]
fn box_errors() {
    box_error_family("crypto_box", true);
}

#[test]
fn box_curve25519xsalsa20poly1305_errors() {
    box_error_family("crypto_box_curve25519xsalsa20poly1305", false);
}

#[test]
fn box_curve25519xchacha20poly1305_errors() {
    box_error_family("crypto_box_curve25519xchacha20poly1305", true);
}

// ============================================== rows 117-119: stream

#[test]
fn rows118_119_stream_errors() {
    // row 118: crypto_stream_chacha20_ietf(_xor) with len > 2^38
    let maxm = g("crypto_stream_chacha20_ietf_messagebytes_max");
    let (kc, kr) = pair::<
        unsafe extern "C" fn(*mut u8, c_ulonglong, *const u8, *const u8) -> c_int,
    >("crypto_stream_chacha20_ietf");
    for over in [1u64, 2, 1000] {
        let l = maxm as u64 + over;
        let kc2 = kc.clone();
        let kr2 = kr.clone();
        cmp_abort(
            &format!("chacha20_ietf len=MAX+{over}"),
            || {
                let k = vec![0u8; 32];
                let n = vec![0u8; 12];
                unsafe { kc2(std::ptr::null_mut(), l, n.as_ptr(), k.as_ptr()) };
            },
            || {
                let k = vec![0u8; 32];
                let n = vec![0u8; 12];
                unsafe { kr2(std::ptr::null_mut(), l, n.as_ptr(), k.as_ptr()) };
            },
        );
    }
    let (xc, xr) = pair::<
        unsafe extern "C" fn(*mut u8, *const u8, c_ulonglong, *const u8, *const u8) -> c_int,
    >("crypto_stream_chacha20_ietf_xor");
    for over in [1u64, 7] {
        let l = maxm as u64 + over;
        let xc2 = xc.clone();
        let xr2 = xr.clone();
        cmp_abort(
            &format!("chacha20_ietf_xor len=MAX+{over}"),
            || {
                let k = vec![0u8; 32];
                let n = vec![0u8; 12];
                unsafe { xc2(std::ptr::null_mut(), std::ptr::null(), l, n.as_ptr(), k.as_ptr()) };
            },
            || {
                let k = vec![0u8; 32];
                let n = vec![0u8; 12];
                unsafe { xr2(std::ptr::null_mut(), std::ptr::null(), l, n.as_ptr(), k.as_ptr()) };
            },
        );
    }
    // row 119: ic beyond the 32-bit counter capacity
    let (ic, ir) = pair::<
        unsafe extern "C" fn(*mut u8, *const u8, c_ulonglong, *const u8, u32, *const u8) -> c_int,
    >("crypto_stream_chacha20_ietf_xor_ic");
    for mlen in [0u64, 1, 64, 65, 128, 4096] {
        let blocks = (mlen + 63) / 64;
        let max_ic = (1u64 << 32) - blocks;
        for over in [1u64, 2] {
            let bad = max_ic + over;
            if bad > u32::MAX as u64 {
                continue;
            }
            let ic2 = ic.clone();
            let ir2 = ir.clone();
            let bic = bad as u32;
            cmp_abort(
                &format!("chacha20_ietf_xor_ic mlen={mlen} ic={bic}"),
                || {
                    let k = vec![0u8; 32];
                    let n = vec![0u8; 12];
                    let m = vec![0u8; 4096];
                    let mut o = vec![0u8; 4096];
                    unsafe { ic2(o.as_mut_ptr(), m.as_ptr(), mlen, n.as_ptr(), bic, k.as_ptr()) };
                },
                || {
                    let k = vec![0u8; 32];
                    let n = vec![0u8; 12];
                    let m = vec![0u8; 4096];
                    let mut o = vec![0u8; 4096];
                    unsafe { ir2(o.as_mut_ptr(), m.as_ptr(), mlen, n.as_ptr(), bic, k.as_ptr()) };
                },
            );
        }
        // exactly at the limit must NOT abort
        if max_ic <= u32::MAX as u64 {
            let ok = max_ic as u32;
            let mut a = buf(4096 + 8);
            let mut b = buf(4096 + 8);
            let m = vec![0x33u8; 4096];
            let k = vec![0u8; 32];
            let n = vec![0u8; 12];
            let ret = cmp2!(
                &format!("chacha20_ietf_xor_ic mlen={mlen} ic=max"),
                ic(a.as_mut_ptr(), m.as_ptr(), mlen, n.as_ptr(), ok, k.as_ptr()),
                ir(b.as_mut_ptr(), m.as_ptr(), mlen, n.as_ptr(), ok, k.as_ptr())
            );
            assert_eq!(ret, 0, "ic at the limit must succeed");
            same_bytes("chacha20_ietf_xor_ic at limit", &a, &b);
        }
    }
}

// ============================ rows 120-166: scalarmult / core / sign / kx / kem

#[test]
fn rows120_121_scalarmult_curve25519_errors() {
    for name in ["crypto_scalarmult_curve25519", "crypto_scalarmult"] {
        let (c, r) = pair::<unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> c_int>(name);
        let mut rng = Rng::seeded();
        for (i, p) in small_order_points().iter().enumerate() {
            for _ in 0..8 {
                let n = rng.bytes(32);
                let mut a = buf(32);
                let mut b = buf(32);
                let ret = cmp2!(
                    &format!("{name} small-order #{i}"),
                    c(a.as_mut_ptr(), n.as_ptr(), p.as_ptr()),
                    r(b.as_mut_ptr(), n.as_ptr(), p.as_ptr())
                );
                assert_eq!(ret, -1, "{name} small-order #{i} must be rejected");
                same_bytes(&format!("{name} small-order #{i}"), &a, &b);
            }
        }
        // high bit set variants of the blocklist (masked by the C)
        for (i, p) in small_order_points().iter().enumerate() {
            let mut q = *p;
            q[31] |= 0x80;
            let n = rng.bytes(32);
            let mut a = buf(32);
            let mut b = buf(32);
            let ret = cmp2!(
                &format!("{name} small-order+highbit #{i}"),
                c(a.as_mut_ptr(), n.as_ptr(), q.as_ptr()),
                r(b.as_mut_ptr(), n.as_ptr(), q.as_ptr())
            );
            assert_eq!(ret, -1, "{name} small-order+highbit #{i}");
            same_bytes(&format!("{name} small-order+highbit #{i}"), &a, &b);
        }
        // all-zero scalar against a valid point (row 121: all-zero output)
        let (bcp, _) = pair::<unsafe extern "C" fn(*mut u8, *const u8) -> c_int>(
            "crypto_scalarmult_curve25519_base",
        );
        let sk = rng.bytes(32);
        let mut vp = vec![0u8; 32];
        unsafe {
            bcp(vp.as_mut_ptr(), sk.as_ptr());
        }
        for z in [vec![0u8; 32], {
            let mut v = vec![0u8; 32];
            v[31] = 0x80;
            v
        }] {
            let mut a = buf(32);
            let mut b = buf(32);
            cmp2!(
                &format!("{name} zero scalar"),
                c(a.as_mut_ptr(), z.as_ptr(), vp.as_ptr()),
                r(b.as_mut_ptr(), z.as_ptr(), vp.as_ptr())
            );
            same_bytes(&format!("{name} zero scalar"), &a, &b);
        }
    }
}

#[test]
fn rows122_130_scalarmult_ed25519_ristretto_errors() {
    let mut rng = Rng::seeded();
    // A collection of invalid / non-canonical / small-order ed25519 encodings.
    let mut bad_points: Vec<Vec<u8>> = vec![
        vec![0u8; 32],
        vec![0xff; 32],
        {
            let mut v = vec![0u8; 32];
            v[0] = 1;
            v
        },
        // canonical field element boundaries: p, p+1, ... (non-canonical)
        {
            let mut v = vec![0xffu8; 32];
            v[0] = 0xed;
            v[31] = 0x7f;
            v
        },
        {
            let mut v = vec![0xffu8; 32];
            v[0] = 0xee;
            v[31] = 0x7f;
            v
        },
    ];
    for _ in 0..300 {
        bad_points.push(rng.bytes(32));
    }
    for name in ["crypto_scalarmult_ed25519", "crypto_scalarmult_ed25519_noclamp"] {
        let (c, r) = pair::<unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> c_int>(name);
        for p in &bad_points {
            let n = {
                let mut s = rng.bytes(32);
                s[31] &= 0x0f;
                s[0] |= 1;
                s
            };
            let mut a = buf(32);
            let mut b = buf(32);
            cmp2!(
                &format!("{name} bad point"),
                c(a.as_mut_ptr(), n.as_ptr(), p.as_ptr()),
                r(b.as_mut_ptr(), n.as_ptr(), p.as_ptr())
            );
            same_bytes(&format!("{name} bad point {}", hex(p)), &a, &b);
        }
        // row 126/128: zero scalar and scalars ≡ 0 mod L against a valid point
        let (bp, _) = pair::<unsafe extern "C" fn(*mut u8, *const u8) -> c_int>(
            "crypto_scalarmult_ed25519_base",
        );
        let s = {
            let mut v = rng.bytes(32);
            v[31] &= 0x0f;
            v[0] |= 1;
            v
        };
        let mut vp = vec![0u8; 32];
        unsafe {
            assert_eq!(bp(vp.as_mut_ptr(), s.as_ptr()), 0);
        }
        for z in [vec![0u8; 32], {
            let mut v = vec![0u8; 32];
            v[31] = 0x80;
            v
        }] {
            let all_zero = z.iter().all(|&x| x == 0);
            let mut a = buf(32);
            let mut b = buf(32);
            let ret = cmp2!(
                &format!("{name} zero-ish scalar"),
                c(a.as_mut_ptr(), z.as_ptr(), vp.as_ptr()),
                r(b.as_mut_ptr(), z.as_ptr(), vp.as_ptr())
            );
            if all_zero {
                assert_eq!(ret, -1, "{name} all-zero scalar must be rejected");
            }
            same_bytes(&format!("{name} zero-ish scalar"), &a, &b);
        }
    }
    // rows 129/130: base variants with a zero scalar
    for name in [
        "crypto_scalarmult_ed25519_base",
        "crypto_scalarmult_ed25519_base_noclamp",
        "crypto_scalarmult_ristretto255_base",
    ] {
        let (c, r) = pair::<unsafe extern "C" fn(*mut u8, *const u8) -> c_int>(name);
        for z in [vec![0u8; 32], {
            let mut v = vec![0u8; 32];
            v[31] = 0x80;
            v
        }] {
            let all_zero = z.iter().all(|&x| x == 0);
            let mut a = buf(32);
            let mut b = buf(32);
            let ret = cmp2!(
                &format!("{name} zero-ish scalar"),
                c(a.as_mut_ptr(), z.as_ptr()),
                r(b.as_mut_ptr(), z.as_ptr())
            );
            if all_zero {
                assert_eq!(ret, -1, "{name} all-zero scalar must be rejected");
            }
            same_bytes(&format!("{name} zero-ish scalar"), &a, &b);
        }
        // L itself and multiples of L reduce to 0 for the ristretto/noclamp cases
        let l: Vec<u8> = vec![
            0xed, 0xd3, 0xf5, 0x5c, 0x1a, 0x63, 0x12, 0x58, 0xd6, 0x9c, 0xf7, 0xa2, 0xde, 0xf9,
            0xde, 0x14, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x10,
        ];
        let mut a = buf(32);
        let mut b = buf(32);
        cmp2!(
            &format!("{name} scalar=L"),
            c(a.as_mut_ptr(), l.as_ptr()),
            r(b.as_mut_ptr(), l.as_ptr())
        );
        same_bytes(&format!("{name} scalar=L"), &a, &b);
    }
    // rows 131/132: ristretto255 with invalid encodings
    let (c, r) = pair::<unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> c_int>(
        "crypto_scalarmult_ristretto255",
    );
    for p in &bad_points {
        let n = {
            let mut v = rng.bytes(32);
            v[31] &= 0x0f;
            v[0] |= 1;
            v
        };
        let mut a = buf(32);
        let mut b = buf(32);
        cmp2!(
            "ristretto255 bad point",
            c(a.as_mut_ptr(), n.as_ptr(), p.as_ptr()),
            r(b.as_mut_ptr(), n.as_ptr(), p.as_ptr())
        );
        same_bytes(&format!("ristretto255 bad point {}", hex(p)), &a, &b);
    }
}

#[test]
fn rows134_145_core_errors() {
    let mut rng = Rng::seeded();
    let mut blobs: Vec<Vec<u8>> = vec![vec![0u8; 32], vec![0xff; 32], vec![1u8; 32]];
    for _ in 0..400 {
        blobs.push(rng.bytes(32));
    }
    // rows 134/141: is_valid_point returns 0 (not -1) for invalid input
    for name in [
        "crypto_core_ed25519_is_valid_point",
        "crypto_core_ristretto255_is_valid_point",
    ] {
        let (c, r) = pair::<unsafe extern "C" fn(*const u8) -> c_int>(name);
        for p in &blobs {
            let ret = cmp2!(&format!("{name}"), c(p.as_ptr()), r(p.as_ptr()));
            assert!(ret == 0 || ret == 1, "{name} must return 0 or 1, got {ret}");
        }
    }
    // rows 135/136/142/143: add / sub reject off-curve or undecodable points
    for name in [
        "crypto_core_ed25519_add",
        "crypto_core_ed25519_sub",
        "crypto_core_ristretto255_add",
        "crypto_core_ristretto255_sub",
    ] {
        let (c, r) = pair::<unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> c_int>(name);
        for i in 0..blobs.len() {
            let p = &blobs[i];
            let q = &blobs[(i * 3 + 1) % blobs.len()];
            let mut a = buf(32);
            let mut b = buf(32);
            cmp2!(
                &format!("{name}"),
                c(a.as_mut_ptr(), p.as_ptr(), q.as_ptr()),
                r(b.as_mut_ptr(), p.as_ptr(), q.as_ptr())
            );
            same_bytes(&format!("{name} {} {}", hex(p), hex(q)), &a, &b);
        }
    }
    // rows 137/144: scalar_invert of zero (and of L, which reduces to zero)
    for name in [
        "crypto_core_ed25519_scalar_invert",
        "crypto_core_ristretto255_scalar_invert",
    ] {
        let (c, r) = pair::<unsafe extern "C" fn(*mut u8, *const u8) -> c_int>(name);
        let mut zeros: Vec<Vec<u8>> = vec![vec![0u8; 32]];
        zeros.push(vec![
            0xed, 0xd3, 0xf5, 0x5c, 0x1a, 0x63, 0x12, 0x58, 0xd6, 0x9c, 0xf7, 0xa2, 0xde, 0xf9,
            0xde, 0x14, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x10,
        ]);
        for s in &zeros {
            let mut a = buf(32);
            let mut b = buf(32);
            let ret = cmp2!(
                &format!("{name} zero"),
                c(a.as_mut_ptr(), s.as_ptr()),
                r(b.as_mut_ptr(), s.as_ptr())
            );
            if s.iter().all(|&x| x == 0) {
                assert_eq!(ret, -1, "{name} of zero must return -1");
            }
            same_bytes(&format!("{name} zero"), &a, &b);
        }
    }
    // rows 138/139/140/145: out-of-range hash_alg values across the FFI
    for name in [
        "crypto_core_ed25519_from_string",
        "crypto_core_ed25519_from_string_nu",
        "crypto_core_ed25519_scalar_from_string",
        "crypto_core_ristretto255_from_string",
        "crypto_core_ristretto255_scalar_from_string",
    ] {
        let (c, r) = pair::<
            unsafe extern "C" fn(*mut u8, *const u8, Sz, *const u8, Sz, c_int) -> c_int,
        >(name);
        let ctx = b"ctx";
        let msg = b"message";
        for alg in [0i32, 3, 4, -1, -2, 99, i32::MIN, i32::MAX] {
            let mut a = buf(32);
            let mut b = buf(32);
            let ret = cmp2!(
                &format!("{name} hash_alg={alg}"),
                c(a.as_mut_ptr(), ctx.as_ptr(), 3, msg.as_ptr(), 7, alg),
                r(b.as_mut_ptr(), ctx.as_ptr(), 3, msg.as_ptr(), 7, alg)
            );
            assert_eq!(ret, -1, "{name} hash_alg={alg} must be rejected");
            same_bytes(&format!("{name} hash_alg={alg}"), &a, &b);
        }
    }
}

#[test]
fn rows146_155_sign_errors() {
    let mut rng = Rng::seeded();
    let (skc, _) = pair::<unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> c_int>(
        "crypto_sign_ed25519_seed_keypair",
    );
    let seed = rng.bytes(32);
    let mut pk = vec![0u8; 32];
    let mut sk = vec![0u8; 64];
    unsafe {
        skc(pk.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr());
    }

    for prefix in ["crypto_sign", "crypto_sign_ed25519"] {
        let (dc, _) = pair::<
            unsafe extern "C" fn(*mut u8, *mut c_ulonglong, *const u8, c_ulonglong, *const u8) -> c_int,
        >(&format!("{prefix}_detached"));
        let (vc, vr) = pair::<
            unsafe extern "C" fn(*const u8, *const u8, c_ulonglong, *const u8) -> c_int,
        >(&format!("{prefix}_verify_detached"));
        let (oc, or) = pair::<
            unsafe extern "C" fn(*mut u8, *mut c_ulonglong, *const u8, c_ulonglong, *const u8) -> c_int,
        >(&format!("{prefix}_open"));

        // row 151: smlen < 64
        let junk = rng.bytes(128);
        for smlen in 0..64u64 {
            let mut a = buf(128);
            let mut b = buf(128);
            let mut la: c_ulonglong = 0xDEAD;
            let mut lb: c_ulonglong = 0xDEAD;
            let ret = cmp2!(
                &format!("{prefix}_open smlen={smlen}"),
                oc(a.as_mut_ptr(), &mut la, junk.as_ptr(), smlen, pk.as_ptr()),
                or(b.as_mut_ptr(), &mut lb, junk.as_ptr(), smlen, pk.as_ptr())
            );
            assert_eq!(ret, -1, "{prefix}_open smlen={smlen} must be rejected");
            same_bytes(&format!("{prefix}_open smlen={smlen}"), &a, &b);
            assert_eq!(la, lb, "{prefix}_open mlen on short input");
        }

        for ml in [0usize, 1, 32, 64, 200] {
            let m = rng.bytes(ml);
            let mut sig = vec![0u8; 64];
            unsafe {
                dc(sig.as_mut_ptr(), std::ptr::null_mut(), m.as_ptr(), ml as c_ulonglong, sk.as_ptr());
            }
            // row 150: corrupt each signature byte
            for i in 0..64 {
                let mut bad = sig.clone();
                bad[i] ^= 0x01;
                let ret = cmp2!(
                    &format!("{prefix}_verify_detached sig@{i}"),
                    vc(bad.as_ptr(), m.as_ptr(), ml as c_ulonglong, pk.as_ptr()),
                    vr(bad.as_ptr(), m.as_ptr(), ml as c_ulonglong, pk.as_ptr())
                );
                assert_eq!(ret, -1, "{prefix}_verify_detached sig@{i}");
            }
            // rows 146/149: non-canonical S (high bits of sig[63]) and bad R
            for v in [0x10u8, 0x20, 0x40, 0x80, 0xf0, 0xff] {
                let mut bad = sig.clone();
                bad[63] |= v;
                cmp2!(
                    &format!("{prefix}_verify_detached S|{v:#02x}"),
                    vc(bad.as_ptr(), m.as_ptr(), ml as c_ulonglong, pk.as_ptr()),
                    vr(bad.as_ptr(), m.as_ptr(), ml as c_ulonglong, pk.as_ptr())
                );
            }
            // rows 147/148: non-canonical / small-order public keys
            let mut bad_pks: Vec<Vec<u8>> = vec![
                vec![0u8; 32],
                vec![0xff; 32],
                {
                    let mut v = vec![0u8; 32];
                    v[0] = 1;
                    v
                },
                {
                    let mut v = vec![0xffu8; 32];
                    v[0] = 0xed;
                    v[31] = 0x7f;
                    v
                },
            ];
            for _ in 0..80 {
                bad_pks.push(rng.bytes(32));
            }
            for bpk in &bad_pks {
                let ret = cmp2!(
                    &format!("{prefix}_verify_detached bad pk"),
                    vc(sig.as_ptr(), m.as_ptr(), ml as c_ulonglong, bpk.as_ptr()),
                    vr(sig.as_ptr(), m.as_ptr(), ml as c_ulonglong, bpk.as_ptr())
                );
                assert_eq!(ret, -1, "{prefix}_verify_detached bad pk {}", hex(bpk));
            }
            // row 153: open with a corrupted signed message
            let mut sm = vec![0u8; 64 + ml];
            sm[..64].copy_from_slice(&sig);
            sm[64..].copy_from_slice(&m);
            for i in 0..sm.len() {
                let mut bad = sm.clone();
                bad[i] ^= 0x01;
                let mut a = buf(ml + 8);
                let mut b = buf(ml + 8);
                let mut la: c_ulonglong = 0xDEAD;
                let mut lb: c_ulonglong = 0xDEAD;
                let ret = cmp2!(
                    &format!("{prefix}_open corrupt@{i}"),
                    oc(a.as_mut_ptr(), &mut la, bad.as_ptr(), (64 + ml) as c_ulonglong, pk.as_ptr()),
                    or(b.as_mut_ptr(), &mut lb, bad.as_ptr(), (64 + ml) as c_ulonglong, pk.as_ptr())
                );
                assert_eq!(ret, -1, "{prefix}_open corrupt@{i}");
                same_bytes(&format!("{prefix}_open corrupt@{i}"), &a, &b);
                assert_eq!(la, lb);
            }
            // wrong message length
            if ml > 0 {
                cmp2!(
                    &format!("{prefix}_verify_detached wrong mlen"),
                    vc(sig.as_ptr(), m.as_ptr(), (ml - 1) as c_ulonglong, pk.as_ptr()),
                    vr(sig.as_ptr(), m.as_ptr(), (ml - 1) as c_ulonglong, pk.as_ptr())
                );
            }
        }
    }

    // row 154: pk_to_curve25519 rejects non-main-subgroup keys
    let (pc, pr) = pair::<unsafe extern "C" fn(*mut u8, *const u8) -> c_int>(
        "crypto_sign_ed25519_pk_to_curve25519",
    );
    let mut bad_pks: Vec<Vec<u8>> = vec![vec![0u8; 32], vec![0xff; 32], {
        let mut v = vec![0u8; 32];
        v[0] = 1;
        v
    }];
    for _ in 0..300 {
        bad_pks.push(rng.bytes(32));
    }
    for bpk in &bad_pks {
        let mut a = buf(32);
        let mut b = buf(32);
        cmp2!(
            "pk_to_curve25519 bad pk",
            pc(a.as_mut_ptr(), bpk.as_ptr()),
            pr(b.as_mut_ptr(), bpk.as_ptr())
        );
        same_bytes(&format!("pk_to_curve25519 {}", hex(bpk)), &a, &b);
    }

    // row 155: ed25519ph final_verify with corrupted signatures
    for (init, upd, fcre, fver) in [
        (
            "crypto_sign_init",
            "crypto_sign_update",
            "crypto_sign_final_create",
            "crypto_sign_final_verify",
        ),
        (
            "crypto_sign_ed25519ph_init",
            "crypto_sign_ed25519ph_update",
            "crypto_sign_ed25519ph_final_create",
            "crypto_sign_ed25519ph_final_verify",
        ),
    ] {
        let sb = g("crypto_sign_statebytes");
        let (ic, ir) = pair::<unsafe extern "C" fn(*mut u8) -> c_int>(init);
        let (uc, ur) =
            pair::<unsafe extern "C" fn(*mut u8, *const u8, c_ulonglong) -> c_int>(upd);
        let (fc, _) = pair::<
            unsafe extern "C" fn(*mut u8, *mut u8, *mut c_ulonglong, *const u8) -> c_int,
        >(fcre);
        let (vfc, vfr) =
            pair::<unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> c_int>(fver);
        let m = rng.bytes(200);
        let mut st = buf(sb);
        let mut sig = vec![0u8; 64];
        unsafe {
            ic(st.as_mut_ptr());
            uc(st.as_mut_ptr(), m.as_ptr(), 200);
            fc(st.as_mut_ptr(), sig.as_mut_ptr(), std::ptr::null_mut(), sk.as_ptr());
        }
        for i in 0..64 {
            let mut bad = sig.clone();
            bad[i] ^= 0x01;
            let mut sa = buf(sb);
            let mut sbb = buf(sb);
            unsafe {
                ic(sa.as_mut_ptr());
                ir(sbb.as_mut_ptr());
                uc(sa.as_mut_ptr(), m.as_ptr(), 200);
                ur(sbb.as_mut_ptr(), m.as_ptr(), 200);
            }
            let ret = cmp2!(
                &format!("{fver} sig@{i}"),
                vfc(sa.as_mut_ptr(), bad.as_ptr(), pk.as_ptr()),
                vfr(sbb.as_mut_ptr(), bad.as_ptr(), pk.as_ptr())
            );
            assert_eq!(ret, -1, "{fver} sig@{i} must be rejected");
        }
        // bad public keys
        for bpk in bad_pks.iter().take(40) {
            let mut sa = buf(sb);
            let mut sbb = buf(sb);
            unsafe {
                ic(sa.as_mut_ptr());
                ir(sbb.as_mut_ptr());
                uc(sa.as_mut_ptr(), m.as_ptr(), 200);
                ur(sbb.as_mut_ptr(), m.as_ptr(), 200);
            }
            cmp2!(
                &format!("{fver} bad pk"),
                vfc(sa.as_mut_ptr(), sig.as_ptr(), bpk.as_ptr()),
                vfr(sbb.as_mut_ptr(), sig.as_ptr(), bpk.as_ptr())
            );
        }
    }
}

#[test]
fn rows156_159_kx_errors() {
    let (skc, _) = pair::<unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> c_int>(
        "crypto_kx_seed_keypair",
    );
    let mut rng = Rng::seeded();
    let seed = rng.bytes(32);
    let mut pk = vec![0u8; 32];
    let mut sk = vec![0u8; 32];
    unsafe {
        skc(pk.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr());
    }
    type KxF =
        unsafe extern "C" fn(*mut u8, *mut u8, *const u8, *const u8, *const u8) -> c_int;
    for name in [
        "crypto_kx_client_session_keys",
        "crypto_kx_server_session_keys",
    ] {
        let (c, r) = pair::<KxF>(name);
        // row 157/159: small-order peer public key -> -1
        for (i, bad) in small_order_points().iter().enumerate() {
            let mut rxa = buf(32);
            let mut txa = buf(32);
            let mut rxb = buf(32);
            let mut txb = buf(32);
            let ret = cmp2!(
                &format!("{name} small-order peer #{i}"),
                c(rxa.as_mut_ptr(), txa.as_mut_ptr(), pk.as_ptr(), sk.as_ptr(), bad.as_ptr()),
                r(rxb.as_mut_ptr(), txb.as_mut_ptr(), pk.as_ptr(), sk.as_ptr(), bad.as_ptr())
            );
            assert_eq!(ret, -1, "{name} small-order peer #{i} must be rejected");
            same_bytes(&format!("{name} rx on failure"), &rxa, &rxb);
            same_bytes(&format!("{name} tx on failure"), &txa, &txb);
        }
        // row 156/158: rx == NULL && tx == NULL -> abort
        let c2 = c.clone();
        let r2 = r.clone();
        let pk2 = pk.clone();
        let sk2 = sk.clone();
        let pk3 = pk.clone();
        let sk3 = sk.clone();
        cmp_abort(
            &format!("{name} rx=tx=NULL"),
            || unsafe {
                c2(std::ptr::null_mut(), std::ptr::null_mut(), pk2.as_ptr(), sk2.as_ptr(), pk2.as_ptr());
            },
            || unsafe {
                r2(std::ptr::null_mut(), std::ptr::null_mut(), pk3.as_ptr(), sk3.as_ptr(), pk3.as_ptr());
            },
        );
    }
}

#[test]
fn rows160_166_kem_errors() {
    let mut rng = Rng::seeded();
    // row 160/161: mlkem768 enc rejects non-canonical public keys
    let (ec, er) =
        pair::<unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> c_int>("crypto_kem_mlkem768_enc");
    let (edc, edr) = pair::<
        unsafe extern "C" fn(*mut u8, *mut u8, *const u8, *const u8) -> c_int,
    >("crypto_kem_mlkem768_enc_deterministic");
    let coins = rng.bytes(32);
    let mut bad_pks: Vec<Vec<u8>> = vec![vec![0xffu8; 1184], vec![0u8; 1184]];
    for _ in 0..40 {
        bad_pks.push(rng.bytes(1184));
    }
    // a valid pk with one coefficient pushed out of range
    let (skc, _) = pair::<unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> c_int>(
        "crypto_kem_mlkem768_seed_keypair",
    );
    let seed = rng.bytes(64);
    let mut vpk = vec![0u8; 1184];
    let mut vsk = vec![0u8; 2400];
    unsafe {
        skc(vpk.as_mut_ptr(), vsk.as_mut_ptr(), seed.as_ptr());
    }
    for i in [0usize, 1, 100, 500, 1151] {
        let mut p = vpk.clone();
        p[i] = 0xff;
        p[i + 1] = 0xff;
        bad_pks.push(p);
    }
    for (i, p) in bad_pks.iter().enumerate() {
        let mut cta = buf(1088);
        let mut ssa = buf(32);
        let mut ctb = buf(1088);
        let mut ssb = buf(32);
        cmp2!(
            &format!("mlkem768_enc bad pk #{i}"),
            ec(cta.as_mut_ptr(), ssa.as_mut_ptr(), p.as_ptr()),
            er(ctb.as_mut_ptr(), ssb.as_mut_ptr(), p.as_ptr())
        );
        let mut cta = buf(1088);
        let mut ssa = buf(32);
        let mut ctb = buf(1088);
        let mut ssb = buf(32);
        let ret = cmp2!(
            &format!("mlkem768_enc_deterministic bad pk #{i}"),
            edc(cta.as_mut_ptr(), ssa.as_mut_ptr(), p.as_ptr(), coins.as_ptr()),
            edr(ctb.as_mut_ptr(), ssb.as_mut_ptr(), p.as_ptr(), coins.as_ptr())
        );
        // whatever the verdict, the outputs must be identical
        same_bytes(&format!("mlkem768_enc_det bad pk #{i} ct"), &cta, &ctb);
        same_bytes(&format!("mlkem768_enc_det bad pk #{i} ss"), &ssa, &ssb);
        let _ = ret;
    }
    // row 162: mlkem768_dec never rejects — both must derive the SAME secret
    let (dc, dr) =
        pair::<unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> c_int>("crypto_kem_mlkem768_dec");
    let mut vct = vec![0u8; 1088];
    let mut vss = vec![0u8; 32];
    unsafe {
        edc(vct.as_mut_ptr(), vss.as_mut_ptr(), vpk.as_ptr(), coins.as_ptr());
    }
    for i in [0usize, 1, 17, 543, 1086, 1087] {
        let mut bad = vct.clone();
        bad[i] ^= 0xa5;
        let mut a = buf(32);
        let mut b = buf(32);
        let ret = cmp2!(
            &format!("mlkem768_dec corrupt@{i}"),
            dc(a.as_mut_ptr(), bad.as_ptr(), vsk.as_ptr()),
            dr(b.as_mut_ptr(), bad.as_ptr(), vsk.as_ptr())
        );
        assert_eq!(ret, 0, "mlkem768_dec must always return 0");
        same_bytes(&format!("mlkem768_dec corrupt@{i} ss"), &a, &b);
        assert_ne!(a, vss, "implicit rejection must not yield the real secret");
    }
    // fully random ciphertexts and secret keys
    for _ in 0..20 {
        let ct = rng.bytes(1088);
        let mut a = buf(32);
        let mut b = buf(32);
        cmp2!(
            "mlkem768_dec random ct",
            dc(a.as_mut_ptr(), ct.as_ptr(), vsk.as_ptr()),
            dr(b.as_mut_ptr(), ct.as_ptr(), vsk.as_ptr())
        );
        same_bytes("mlkem768_dec random ct ss", &a, &b);
        let sk = rng.bytes(2400);
        let mut a = buf(32);
        let mut b = buf(32);
        cmp2!(
            "mlkem768_dec random sk",
            dc(a.as_mut_ptr(), vct.as_ptr(), sk.as_ptr()),
            dr(b.as_mut_ptr(), vct.as_ptr(), sk.as_ptr())
        );
        same_bytes("mlkem768_dec random sk ss", &a, &b);
    }
    // rows 163/164: xwing enc rejects bad pk halves
    for prefix in ["crypto_kem_xwing", "crypto_kem"] {
        let (ec, er) =
            pair::<unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> c_int>(&format!("{prefix}_enc"));
        let (dc, dr) = pair::<unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> c_int>(
            &format!("{prefix}_dec"),
        );
        let mut bad: Vec<Vec<u8>> = vec![vec![0xffu8; 1216], vec![0u8; 1216]];
        for _ in 0..30 {
            bad.push(rng.bytes(1216));
        }
        // valid mlkem half + small-order x25519 half
        let (skc2, _) = pair::<unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> c_int>(
            &format!("{prefix}_seed_keypair"),
        );
        let s = rng.bytes(32);
        let mut xpk = vec![0u8; 1216];
        let mut xsk = vec![0u8; 32];
        unsafe {
            skc2(xpk.as_mut_ptr(), xsk.as_mut_ptr(), s.as_ptr());
        }
        for sp in small_order_points() {
            let mut p = xpk.clone();
            p[1184..].copy_from_slice(&sp);
            bad.push(p);
        }
        for (i, p) in bad.iter().enumerate() {
            let mut cta = buf(1120);
            let mut ssa = buf(32);
            let mut ctb = buf(1120);
            let mut ssb = buf(32);
            cmp2!(
                &format!("{prefix}_enc bad pk #{i}"),
                ec(cta.as_mut_ptr(), ssa.as_mut_ptr(), p.as_ptr()),
                er(ctb.as_mut_ptr(), ssb.as_mut_ptr(), p.as_ptr())
            );
        }
        if let Some(_) = libs().has(&format!("{prefix}_enc_deterministic")).then_some(()) {
            let (edc2, edr2) = pair::<
                unsafe extern "C" fn(*mut u8, *mut u8, *const u8, *const u8) -> c_int,
            >(&format!("{prefix}_enc_deterministic"));
            let eseed = rng.bytes(64);
            for (i, p) in bad.iter().enumerate() {
                let mut cta = buf(1120);
                let mut ssa = buf(32);
                let mut ctb = buf(1120);
                let mut ssb = buf(32);
                cmp2!(
                    &format!("{prefix}_enc_deterministic bad pk #{i}"),
                    edc2(cta.as_mut_ptr(), ssa.as_mut_ptr(), p.as_ptr(), eseed.as_ptr()),
                    edr2(ctb.as_mut_ptr(), ssb.as_mut_ptr(), p.as_ptr(), eseed.as_ptr())
                );
                same_bytes(&format!("{prefix}_enc_det bad pk #{i} ct"), &cta, &ctb);
                same_bytes(&format!("{prefix}_enc_det bad pk #{i} ss"), &ssa, &ssb);
            }
        }
        // rows 165/166: dec with corrupted / random ciphertexts and secret keys
        let mut vct = vec![0u8; 1120];
        let mut vss = vec![0u8; 32];
        unsafe {
            assert_eq!(ec(vct.as_mut_ptr(), vss.as_mut_ptr(), xpk.as_ptr()), 0);
        }
        for i in [0usize, 1, 500, 1118, 1119] {
            let mut b = vct.clone();
            b[i] ^= 0x5a;
            let mut a = buf(32);
            let mut bb = buf(32);
            cmp2!(
                &format!("{prefix}_dec corrupt@{i}"),
                dc(a.as_mut_ptr(), b.as_ptr(), xsk.as_ptr()),
                dr(bb.as_mut_ptr(), b.as_ptr(), xsk.as_ptr())
            );
            same_bytes(&format!("{prefix}_dec corrupt@{i} ss"), &a, &bb);
        }
        for _ in 0..20 {
            let ct = rng.bytes(1120);
            let mut a = buf(32);
            let mut bb = buf(32);
            cmp2!(
                &format!("{prefix}_dec random ct"),
                dc(a.as_mut_ptr(), ct.as_ptr(), xsk.as_ptr()),
                dr(bb.as_mut_ptr(), ct.as_ptr(), xsk.as_ptr())
            );
            same_bytes(&format!("{prefix}_dec random ct ss"), &a, &bb);
            let sk = rng.bytes(32);
            let mut a = buf(32);
            let mut bb = buf(32);
            cmp2!(
                &format!("{prefix}_dec random sk"),
                dc(a.as_mut_ptr(), vct.as_ptr(), sk.as_ptr()),
                dr(bb.as_mut_ptr(), vct.as_ptr(), sk.as_ptr())
            );
            same_bytes(&format!("{prefix}_dec random sk ss"), &a, &bb);
        }
    }
}

// ------------------------------------------------------------ silence unused
#[allow(dead_code)]
fn _unused(_: *const c_char) {}
