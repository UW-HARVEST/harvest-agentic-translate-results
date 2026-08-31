//! AEAD constructions: chacha20poly1305 (original + IETF), xchacha20poly1305,
//! aes256gcm (incl. the precomputation `*_afternm` API), aegis128l, aegis256.
mod common;

use common::*;
use std::os::raw::{c_int, c_uchar, c_ulonglong, c_void};

type FnEncrypt = unsafe extern "C" fn(
    *mut c_uchar,          // c
    *mut c_ulonglong,      // clen_p
    *const c_uchar,        // m
    c_ulonglong,           // mlen
    *const c_uchar,        // ad
    c_ulonglong,           // adlen
    *const c_uchar,        // nsec
    *const c_uchar,        // npub
    *const c_uchar,        // k
) -> c_int;

type FnDecrypt = unsafe extern "C" fn(
    *mut c_uchar,          // m
    *mut c_ulonglong,      // mlen_p
    *mut c_uchar,          // nsec
    *const c_uchar,        // c
    c_ulonglong,           // clen
    *const c_uchar,        // ad
    c_ulonglong,           // adlen
    *const c_uchar,        // npub
    *const c_uchar,        // k
) -> c_int;

type FnEncryptDetached = unsafe extern "C" fn(
    *mut c_uchar,          // c
    *mut c_uchar,          // mac
    *mut c_ulonglong,      // maclen_p
    *const c_uchar,        // m
    c_ulonglong,           // mlen
    *const c_uchar,        // ad
    c_ulonglong,           // adlen
    *const c_uchar,        // nsec
    *const c_uchar,        // npub
    *const c_uchar,        // k
) -> c_int;

type FnDecryptDetached = unsafe extern "C" fn(
    *mut c_uchar,          // m
    *mut c_uchar,          // nsec
    *const c_uchar,        // c
    c_ulonglong,           // clen
    *const c_uchar,        // mac
    *const c_uchar,        // ad
    c_ulonglong,           // adlen
    *const c_uchar,        // npub
    *const c_uchar,        // k
) -> c_int;

type FnKeygen = unsafe extern "C" fn(*mut c_uchar);

fn aead_suite(prefix: &str) {
    for s in [
        "keybytes",
        "nsecbytes",
        "npubbytes",
        "abytes",
        "messagebytes_max",
    ] {
        cmp_size(&format!("{prefix}_{s}"));
    }
    unsafe {
        let (ckb, _): (FnSize, FnSize) = pair(&format!("{prefix}_keybytes"));
        let (cnb, _): (FnSize, FnSize) = pair(&format!("{prefix}_npubbytes"));
        let (cab, _): (FnSize, FnSize) = pair(&format!("{prefix}_abytes"));
        let kb = ckb();
        let nb = cnb();
        let ab = cab();

        let (ce, re): (FnEncrypt, FnEncrypt) = pair(&format!("{prefix}_encrypt"));
        let (cd, rd): (FnDecrypt, FnDecrypt) = pair(&format!("{prefix}_decrypt"));
        let (ced, red): (FnEncryptDetached, FnEncryptDetached) =
            pair(&format!("{prefix}_encrypt_detached"));
        let (cdd, rdd): (FnDecryptDetached, FnDecryptDetached) =
            pair(&format!("{prefix}_decrypt_detached"));

        let mut rng = Rng::new(0x4000 + prefix.len() as u64);
        let msg = rng.vec(3001);
        let ad = rng.vec(1001);

        let mut keys: Vec<Vec<u8>> = vec![vec![0u8; kb], vec![0xffu8; kb]];
        keys.push(rng.vec(kb));
        let mut npubs: Vec<Vec<u8>> = vec![vec![0u8; nb], vec![0xffu8; nb]];
        npubs.push(rng.vec(nb));

        let mlens: Vec<usize> = vec![
            0, 1, 2, 15, 16, 17, 31, 32, 33, 63, 64, 65, 100, 127, 128, 129, 191, 192, 193, 255,
            256, 257, 511, 512, 513, 1000, 2048, 3000,
        ];
        let adlens: Vec<usize> = vec![0, 1, 15, 16, 17, 31, 32, 33, 63, 64, 65, 128, 500, 1000];

        for key in &keys {
            for npub in &npubs {
                for &mlen in &mlens {
                    for &adlen in &adlens {
                        // --- combined encrypt ---
                        let mut cc = vec![0xAAu8; mlen + ab + 8];
                        let mut rc = vec![0xAAu8; mlen + ab + 8];
                        let mut ccl: c_ulonglong = 0xdead;
                        let mut rcl: c_ulonglong = 0xdead;
                        let adptr = if adlen == 0 {
                            std::ptr::null()
                        } else {
                            ad.as_ptr()
                        };
                        let a = ce(
                            cc.as_mut_ptr(),
                            &mut ccl,
                            msg.as_ptr(),
                            mlen as c_ulonglong,
                            adptr,
                            adlen as c_ulonglong,
                            std::ptr::null(),
                            npub.as_ptr(),
                            key.as_ptr(),
                        );
                        let b = re(
                            rc.as_mut_ptr(),
                            &mut rcl,
                            msg.as_ptr(),
                            mlen as c_ulonglong,
                            adptr,
                            adlen as c_ulonglong,
                            std::ptr::null(),
                            npub.as_ptr(),
                            key.as_ptr(),
                        );
                        let tag = format!("{prefix}_encrypt(mlen={mlen},adlen={adlen})");
                        assert_eq!(a, b, "{tag} return");
                        assert_eq!(ccl, rcl, "{tag} clen");
                        assert_bytes_eq(&tag, &cc, &rc);

                        // NULL clen_p
                        let mut cc2 = vec![0xAAu8; mlen + ab + 8];
                        let mut rc2 = vec![0xAAu8; mlen + ab + 8];
                        ce(
                            cc2.as_mut_ptr(),
                            std::ptr::null_mut(),
                            msg.as_ptr(),
                            mlen as c_ulonglong,
                            adptr,
                            adlen as c_ulonglong,
                            std::ptr::null(),
                            npub.as_ptr(),
                            key.as_ptr(),
                        );
                        re(
                            rc2.as_mut_ptr(),
                            std::ptr::null_mut(),
                            msg.as_ptr(),
                            mlen as c_ulonglong,
                            adptr,
                            adlen as c_ulonglong,
                            std::ptr::null(),
                            npub.as_ptr(),
                            key.as_ptr(),
                        );
                        assert_bytes_eq(&format!("{tag} NULL clen_p"), &cc2, &rc2);

                        // --- combined decrypt (valid) ---
                        let clen = ccl as usize;
                        let mut cm = vec![0xAAu8; mlen + 8];
                        let mut rm = vec![0xAAu8; mlen + 8];
                        let mut cml: c_ulonglong = 0xdead;
                        let mut rml: c_ulonglong = 0xdead;
                        let a = cd(
                            cm.as_mut_ptr(),
                            &mut cml,
                            std::ptr::null_mut(),
                            cc.as_ptr(),
                            clen as c_ulonglong,
                            adptr,
                            adlen as c_ulonglong,
                            npub.as_ptr(),
                            key.as_ptr(),
                        );
                        let b = rd(
                            rm.as_mut_ptr(),
                            &mut rml,
                            std::ptr::null_mut(),
                            cc.as_ptr(),
                            clen as c_ulonglong,
                            adptr,
                            adlen as c_ulonglong,
                            npub.as_ptr(),
                            key.as_ptr(),
                        );
                        let dtag = format!("{prefix}_decrypt(mlen={mlen},adlen={adlen})");
                        assert_eq!(a, b, "{dtag} return");
                        assert_eq!(cml, rml, "{dtag} mlen");
                        assert_bytes_eq(&dtag, &cm, &rm);
                        assert_eq!(a, 0, "{dtag} should succeed");
                        assert_eq!(&cm[..mlen], &msg[..mlen], "{dtag} plaintext roundtrip");

                        // --- combined decrypt (tampered) ---
                        for bad in tamperings(&cc[..clen], ab, mlen) {
                            let mut cm = vec![0xAAu8; mlen + 8];
                            let mut rm = vec![0xAAu8; mlen + 8];
                            let mut cml: c_ulonglong = 0xdead;
                            let mut rml: c_ulonglong = 0xdead;
                            let a = cd(
                                cm.as_mut_ptr(),
                                &mut cml,
                                std::ptr::null_mut(),
                                bad.as_ptr(),
                                bad.len() as c_ulonglong,
                                adptr,
                                adlen as c_ulonglong,
                                npub.as_ptr(),
                                key.as_ptr(),
                            );
                            let b = rd(
                                rm.as_mut_ptr(),
                                &mut rml,
                                std::ptr::null_mut(),
                                bad.as_ptr(),
                                bad.len() as c_ulonglong,
                                adptr,
                                adlen as c_ulonglong,
                                npub.as_ptr(),
                                key.as_ptr(),
                            );
                            let t = format!("{prefix}_decrypt tampered(len={})", bad.len());
                            assert_eq!(a, b, "{t} return");
                            assert_eq!(cml, rml, "{t} mlen");
                            assert_bytes_eq(&t, &cm, &rm);
                        }
                        // wrong AD
                        if adlen > 0 {
                            let mut bad_ad = ad[..adlen].to_vec();
                            bad_ad[0] ^= 1;
                            let mut cm = vec![0xAAu8; mlen + 8];
                            let mut rm = vec![0xAAu8; mlen + 8];
                            let a = cd(
                                cm.as_mut_ptr(),
                                std::ptr::null_mut(),
                                std::ptr::null_mut(),
                                cc.as_ptr(),
                                clen as c_ulonglong,
                                bad_ad.as_ptr(),
                                adlen as c_ulonglong,
                                npub.as_ptr(),
                                key.as_ptr(),
                            );
                            let b = rd(
                                rm.as_mut_ptr(),
                                std::ptr::null_mut(),
                                std::ptr::null_mut(),
                                cc.as_ptr(),
                                clen as c_ulonglong,
                                bad_ad.as_ptr(),
                                adlen as c_ulonglong,
                                npub.as_ptr(),
                                key.as_ptr(),
                            );
                            assert_eq!(a, b, "{prefix}_decrypt wrong-AD return");
                            assert_bytes_eq(&format!("{prefix}_decrypt wrong AD"), &cm, &rm);
                        }

                        // --- detached ---
                        let mut cc = vec![0xAAu8; mlen + 8];
                        let mut rc = vec![0xAAu8; mlen + 8];
                        let mut cmac = vec![0xAAu8; ab + 8];
                        let mut rmac = vec![0xAAu8; ab + 8];
                        let mut cmacl: c_ulonglong = 0xdead;
                        let mut rmacl: c_ulonglong = 0xdead;
                        let a = ced(
                            cc.as_mut_ptr(),
                            cmac.as_mut_ptr(),
                            &mut cmacl,
                            msg.as_ptr(),
                            mlen as c_ulonglong,
                            adptr,
                            adlen as c_ulonglong,
                            std::ptr::null(),
                            npub.as_ptr(),
                            key.as_ptr(),
                        );
                        let b = red(
                            rc.as_mut_ptr(),
                            rmac.as_mut_ptr(),
                            &mut rmacl,
                            msg.as_ptr(),
                            mlen as c_ulonglong,
                            adptr,
                            adlen as c_ulonglong,
                            std::ptr::null(),
                            npub.as_ptr(),
                            key.as_ptr(),
                        );
                        let etag = format!("{prefix}_encrypt_detached(mlen={mlen},adlen={adlen})");
                        assert_eq!(a, b, "{etag} return");
                        assert_eq!(cmacl, rmacl, "{etag} maclen");
                        assert_bytes_eq(&format!("{etag} c"), &cc, &rc);
                        assert_bytes_eq(&format!("{etag} mac"), &cmac, &rmac);

                        // NULL maclen_p
                        let mut cmac2 = vec![0xAAu8; ab + 8];
                        let mut rmac2 = vec![0xAAu8; ab + 8];
                        ced(
                            cc.as_mut_ptr(),
                            cmac2.as_mut_ptr(),
                            std::ptr::null_mut(),
                            msg.as_ptr(),
                            mlen as c_ulonglong,
                            adptr,
                            adlen as c_ulonglong,
                            std::ptr::null(),
                            npub.as_ptr(),
                            key.as_ptr(),
                        );
                        red(
                            rc.as_mut_ptr(),
                            rmac2.as_mut_ptr(),
                            std::ptr::null_mut(),
                            msg.as_ptr(),
                            mlen as c_ulonglong,
                            adptr,
                            adlen as c_ulonglong,
                            std::ptr::null(),
                            npub.as_ptr(),
                            key.as_ptr(),
                        );
                        assert_bytes_eq(&format!("{etag} NULL maclen_p"), &cmac2, &rmac2);

                        let mut cm = vec![0xAAu8; mlen + 8];
                        let mut rm = vec![0xAAu8; mlen + 8];
                        let a = cdd(
                            cm.as_mut_ptr(),
                            std::ptr::null_mut(),
                            cc.as_ptr(),
                            mlen as c_ulonglong,
                            cmac.as_ptr(),
                            adptr,
                            adlen as c_ulonglong,
                            npub.as_ptr(),
                            key.as_ptr(),
                        );
                        let b = rdd(
                            rm.as_mut_ptr(),
                            std::ptr::null_mut(),
                            cc.as_ptr(),
                            mlen as c_ulonglong,
                            cmac.as_ptr(),
                            adptr,
                            adlen as c_ulonglong,
                            npub.as_ptr(),
                            key.as_ptr(),
                        );
                        let dtag2 =
                            format!("{prefix}_decrypt_detached(mlen={mlen},adlen={adlen})");
                        assert_eq!(a, b, "{dtag2} return");
                        assert_bytes_eq(&dtag2, &cm, &rm);
                        assert_eq!(a, 0, "{dtag2} should succeed");

                        // NULL m (verify only)
                        let a = cdd(
                            std::ptr::null_mut(),
                            std::ptr::null_mut(),
                            cc.as_ptr(),
                            mlen as c_ulonglong,
                            cmac.as_ptr(),
                            adptr,
                            adlen as c_ulonglong,
                            npub.as_ptr(),
                            key.as_ptr(),
                        );
                        let b = rdd(
                            std::ptr::null_mut(),
                            std::ptr::null_mut(),
                            cc.as_ptr(),
                            mlen as c_ulonglong,
                            cmac.as_ptr(),
                            adptr,
                            adlen as c_ulonglong,
                            npub.as_ptr(),
                            key.as_ptr(),
                        );
                        assert_eq!(a, b, "{dtag2} NULL m return");

                        // bad mac
                        for bit in [0usize, 1, ab * 8 - 1] {
                            let mut badmac = cmac[..ab].to_vec();
                            badmac[bit / 8] ^= 1 << (bit % 8);
                            let mut cm = vec![0xAAu8; mlen + 8];
                            let mut rm = vec![0xAAu8; mlen + 8];
                            let a = cdd(
                                cm.as_mut_ptr(),
                                std::ptr::null_mut(),
                                cc.as_ptr(),
                                mlen as c_ulonglong,
                                badmac.as_ptr(),
                                adptr,
                                adlen as c_ulonglong,
                                npub.as_ptr(),
                                key.as_ptr(),
                            );
                            let b = rdd(
                                rm.as_mut_ptr(),
                                std::ptr::null_mut(),
                                cc.as_ptr(),
                                mlen as c_ulonglong,
                                badmac.as_ptr(),
                                adptr,
                                adlen as c_ulonglong,
                                npub.as_ptr(),
                                key.as_ptr(),
                            );
                            assert_eq!(a, b, "{dtag2} bad mac bit {bit} return");
                            assert_bytes_eq(&format!("{dtag2} bad mac bit {bit}"), &cm, &rm);
                        }
                    }
                }
            }
        }

        // keygen
        let (ck, rk): (FnKeygen, FnKeygen) = pair(&format!("{prefix}_keygen"));
        for _ in 0..4 {
            let mut a = vec![0xAAu8; kb + 8];
            let mut b = vec![0xAAu8; kb + 8];
            det_reset();
            ck(a.as_mut_ptr());
            det_reset();
            rk(b.as_mut_ptr());
            assert_bytes_eq(&format!("{prefix}_keygen"), &a, &b);
        }
    }
}

/// Ciphertext mutations that must be rejected identically by both libraries,
/// including truncations below ABYTES.
fn tamperings(ct: &[u8], ab: usize, mlen: usize) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    if !ct.is_empty() {
        let mut v = ct.to_vec();
        v[0] ^= 1;
        out.push(v);
        let mut v = ct.to_vec();
        let n = v.len();
        v[n - 1] ^= 0x80;
        out.push(v);
    }
    // truncations: one byte short, exactly ABYTES-1, zero
    if ct.len() > 0 {
        out.push(ct[..ct.len() - 1].to_vec());
    }
    if ab > 0 {
        out.push(ct[..(ab - 1).min(ct.len())].to_vec());
    }
    out.push(Vec::new());
    if mlen > 0 && ct.len() > ab {
        // flip a bit in the message part only
        let mut v = ct.to_vec();
        v[mlen / 2] ^= 0x10;
        out.push(v);
    }
    out
}

#[test]
fn crypto_aead_chacha20poly1305_matches() {
    aead_suite("crypto_aead_chacha20poly1305");
}

#[test]
fn crypto_aead_chacha20poly1305_ietf_matches() {
    aead_suite("crypto_aead_chacha20poly1305_ietf");
}

#[test]
fn crypto_aead_xchacha20poly1305_ietf_matches() {
    aead_suite("crypto_aead_xchacha20poly1305_ietf");
}

#[test]
fn crypto_aead_aegis128l_matches() {
    aead_suite("crypto_aead_aegis128l");
}

#[test]
fn crypto_aead_aegis256_matches() {
    aead_suite("crypto_aead_aegis256");
}

#[test]
fn crypto_aead_aes256gcm_matches() {
    cmp_int("crypto_aead_aes256gcm_is_available");
    cmp_size("crypto_aead_aes256gcm_statebytes");
    for s in [
        "keybytes",
        "nsecbytes",
        "npubbytes",
        "abytes",
        "messagebytes_max",
    ] {
        cmp_size(&format!("crypto_aead_aes256gcm_{s}"));
    }
    unsafe {
        let (cav, _): (FnInt, FnInt) = pair("crypto_aead_aes256gcm_is_available");
        if cav() == 0 {
            // The reference build has no AES-NI/ARM-Crypto, so every data-path
            // entry point is the ENOSYS stub: it must return -1 and leave the
            // output buffers untouched in both libraries.
            aes256gcm_unavailable_suite();
            let (ckb, _): (FnSize, FnSize) = pair("crypto_aead_aes256gcm_keybytes");
            let kb = ckb();
            let (ck, rk): (FnKeygen, FnKeygen) = pair("crypto_aead_aes256gcm_keygen");
            for _ in 0..4 {
                let mut a = vec![0xAAu8; kb + 8];
                let mut b = vec![0xAAu8; kb + 8];
                det_reset();
                ck(a.as_mut_ptr());
                det_reset();
                rk(b.as_mut_ptr());
                assert_bytes_eq("crypto_aead_aes256gcm_keygen", &a, &b);
            }
            return;
        }
    }
    aead_suite("crypto_aead_aes256gcm");
    aes256gcm_afternm_suite();
}

/// When AES hardware support is absent the C implementation compiles a set of
/// stubs that set `errno = ENOSYS` and return -1 without writing anything.
/// Verify the Rust translation behaves identically for every entry point.
fn aes256gcm_unavailable_suite() {
    unsafe {
        let (ckb, _): (FnSize, FnSize) = pair("crypto_aead_aes256gcm_keybytes");
        let (cnb, _): (FnSize, FnSize) = pair("crypto_aead_aes256gcm_npubbytes");
        let (cab, _): (FnSize, FnSize) = pair("crypto_aead_aes256gcm_abytes");
        let (csb, _): (FnSize, FnSize) = pair("crypto_aead_aes256gcm_statebytes");
        let kb = ckb();
        let nb = cnb();
        let ab = cab();
        let sb = csb();

        let mut rng = Rng::new(0x4200);
        let msg = rng.vec(600);
        let ad = rng.vec(200);
        let key = rng.vec(kb);
        let npub = rng.vec(nb);

        let (ce, re): (FnEncrypt, FnEncrypt) = pair("crypto_aead_aes256gcm_encrypt");
        let (cd, rd): (FnDecrypt, FnDecrypt) = pair("crypto_aead_aes256gcm_decrypt");
        let (ced, red): (FnEncryptDetached, FnEncryptDetached) =
            pair("crypto_aead_aes256gcm_encrypt_detached");
        let (cdd, rdd): (FnDecryptDetached, FnDecryptDetached) =
            pair("crypto_aead_aes256gcm_decrypt_detached");
        let (cbn, rbn): (FnBeforenm, FnBeforenm) = pair("crypto_aead_aes256gcm_beforenm");
        let (cea, rea): (FnEncryptAfternm, FnEncryptAfternm) =
            pair("crypto_aead_aes256gcm_encrypt_afternm");
        let (cda, rda): (FnDecryptAfternm, FnDecryptAfternm) =
            pair("crypto_aead_aes256gcm_decrypt_afternm");
        let (ceda, reda): (FnEncryptDetachedAfternm, FnEncryptDetachedAfternm) =
            pair("crypto_aead_aes256gcm_encrypt_detached_afternm");
        let (cdda, rdda): (FnDecryptDetachedAfternm, FnDecryptDetachedAfternm) =
            pair("crypto_aead_aes256gcm_decrypt_detached_afternm");

        for &mlen in &[0usize, 1, 16, 17, 64, 500] {
            for &adlen in &[0usize, 1, 16, 200] {
                let mut cbuf = vec![0xAAu8; mlen + ab + 16];
                let mut rbuf = vec![0xAAu8; mlen + ab + 16];
                let mut cl: c_ulonglong = 0xdead;
                let mut rl: c_ulonglong = 0xdead;
                let a = ce(
                    cbuf.as_mut_ptr(),
                    &mut cl,
                    msg.as_ptr(),
                    mlen as c_ulonglong,
                    ad.as_ptr(),
                    adlen as c_ulonglong,
                    std::ptr::null(),
                    npub.as_ptr(),
                    key.as_ptr(),
                );
                let b = re(
                    rbuf.as_mut_ptr(),
                    &mut rl,
                    msg.as_ptr(),
                    mlen as c_ulonglong,
                    ad.as_ptr(),
                    adlen as c_ulonglong,
                    std::ptr::null(),
                    npub.as_ptr(),
                    key.as_ptr(),
                );
                assert_eq!(a, b, "aes256gcm_encrypt stub return");
                assert_eq!(cl, rl, "aes256gcm_encrypt stub clen");
                assert_bytes_eq("aes256gcm_encrypt stub buffer", &cbuf, &rbuf);

                let mut cbuf = vec![0xAAu8; mlen + 16];
                let mut rbuf = vec![0xAAu8; mlen + 16];
                let mut cl: c_ulonglong = 0xdead;
                let mut rl: c_ulonglong = 0xdead;
                let a = cd(
                    cbuf.as_mut_ptr(),
                    &mut cl,
                    std::ptr::null_mut(),
                    msg.as_ptr(),
                    (mlen + ab) as c_ulonglong,
                    ad.as_ptr(),
                    adlen as c_ulonglong,
                    npub.as_ptr(),
                    key.as_ptr(),
                );
                let b = rd(
                    rbuf.as_mut_ptr(),
                    &mut rl,
                    std::ptr::null_mut(),
                    msg.as_ptr(),
                    (mlen + ab) as c_ulonglong,
                    ad.as_ptr(),
                    adlen as c_ulonglong,
                    npub.as_ptr(),
                    key.as_ptr(),
                );
                assert_eq!(a, b, "aes256gcm_decrypt stub return");
                assert_eq!(cl, rl, "aes256gcm_decrypt stub mlen");
                assert_bytes_eq("aes256gcm_decrypt stub buffer", &cbuf, &rbuf);

                let mut cbuf = vec![0xAAu8; mlen + 16];
                let mut rbuf = vec![0xAAu8; mlen + 16];
                let mut cmac = vec![0xAAu8; ab + 8];
                let mut rmac = vec![0xAAu8; ab + 8];
                let mut cl: c_ulonglong = 0xdead;
                let mut rl: c_ulonglong = 0xdead;
                let a = ced(
                    cbuf.as_mut_ptr(),
                    cmac.as_mut_ptr(),
                    &mut cl,
                    msg.as_ptr(),
                    mlen as c_ulonglong,
                    ad.as_ptr(),
                    adlen as c_ulonglong,
                    std::ptr::null(),
                    npub.as_ptr(),
                    key.as_ptr(),
                );
                let b = red(
                    rbuf.as_mut_ptr(),
                    rmac.as_mut_ptr(),
                    &mut rl,
                    msg.as_ptr(),
                    mlen as c_ulonglong,
                    ad.as_ptr(),
                    adlen as c_ulonglong,
                    std::ptr::null(),
                    npub.as_ptr(),
                    key.as_ptr(),
                );
                assert_eq!(a, b, "aes256gcm_encrypt_detached stub return");
                assert_eq!(cl, rl, "aes256gcm_encrypt_detached stub maclen");
                assert_bytes_eq("aes256gcm_encrypt_detached stub c", &cbuf, &rbuf);
                assert_bytes_eq("aes256gcm_encrypt_detached stub mac", &cmac, &rmac);

                let mac = rng.vec(ab);
                let mut cbuf = vec![0xAAu8; mlen + 16];
                let mut rbuf = vec![0xAAu8; mlen + 16];
                let a = cdd(
                    cbuf.as_mut_ptr(),
                    std::ptr::null_mut(),
                    msg.as_ptr(),
                    mlen as c_ulonglong,
                    mac.as_ptr(),
                    ad.as_ptr(),
                    adlen as c_ulonglong,
                    npub.as_ptr(),
                    key.as_ptr(),
                );
                let b = rdd(
                    rbuf.as_mut_ptr(),
                    std::ptr::null_mut(),
                    msg.as_ptr(),
                    mlen as c_ulonglong,
                    mac.as_ptr(),
                    ad.as_ptr(),
                    adlen as c_ulonglong,
                    npub.as_ptr(),
                    key.as_ptr(),
                );
                assert_eq!(a, b, "aes256gcm_decrypt_detached stub return");
                assert_bytes_eq("aes256gcm_decrypt_detached stub buffer", &cbuf, &rbuf);
            }
        }

        // precomputation entry points
        let mut cst = AlignedBuf::new(sb, 0xA5);
        let mut rst = AlignedBuf::new(sb, 0xA5);
        let a = cbn(cst.as_mut_ptr() as *mut c_void, key.as_ptr());
        let b = rbn(rst.as_mut_ptr() as *mut c_void, key.as_ptr());
        assert_eq!(a, b, "aes256gcm_beforenm stub return");
        assert_bytes_eq("aes256gcm_beforenm stub state", cst.as_slice(), rst.as_slice());

        let mut cbuf = vec![0xAAu8; 600];
        let mut rbuf = vec![0xAAu8; 600];
        let mut cmac = vec![0xAAu8; ab + 8];
        let mut rmac = vec![0xAAu8; ab + 8];
        let mut cl: c_ulonglong = 0xdead;
        let mut rl: c_ulonglong = 0xdead;
        let a = cea(
            cbuf.as_mut_ptr(),
            &mut cl,
            msg.as_ptr(),
            100,
            ad.as_ptr(),
            10,
            std::ptr::null(),
            npub.as_ptr(),
            cst.as_ptr() as *const c_void,
        );
        let b = rea(
            rbuf.as_mut_ptr(),
            &mut rl,
            msg.as_ptr(),
            100,
            ad.as_ptr(),
            10,
            std::ptr::null(),
            npub.as_ptr(),
            rst.as_ptr() as *const c_void,
        );
        assert_eq!((a, cl), (b, rl), "aes256gcm_encrypt_afternm stub");
        assert_bytes_eq("aes256gcm_encrypt_afternm stub buffer", &cbuf, &rbuf);

        let mut cbuf = vec![0xAAu8; 600];
        let mut rbuf = vec![0xAAu8; 600];
        let a = cda(
            cbuf.as_mut_ptr(),
            &mut cl,
            std::ptr::null_mut(),
            msg.as_ptr(),
            100,
            ad.as_ptr(),
            10,
            npub.as_ptr(),
            cst.as_ptr() as *const c_void,
        );
        let b = rda(
            rbuf.as_mut_ptr(),
            &mut rl,
            std::ptr::null_mut(),
            msg.as_ptr(),
            100,
            ad.as_ptr(),
            10,
            npub.as_ptr(),
            rst.as_ptr() as *const c_void,
        );
        assert_eq!((a, cl), (b, rl), "aes256gcm_decrypt_afternm stub");
        assert_bytes_eq("aes256gcm_decrypt_afternm stub buffer", &cbuf, &rbuf);

        let mut cbuf = vec![0xAAu8; 600];
        let mut rbuf = vec![0xAAu8; 600];
        let a = ceda(
            cbuf.as_mut_ptr(),
            cmac.as_mut_ptr(),
            &mut cl,
            msg.as_ptr(),
            100,
            ad.as_ptr(),
            10,
            std::ptr::null(),
            npub.as_ptr(),
            cst.as_ptr() as *const c_void,
        );
        let b = reda(
            rbuf.as_mut_ptr(),
            rmac.as_mut_ptr(),
            &mut rl,
            msg.as_ptr(),
            100,
            ad.as_ptr(),
            10,
            std::ptr::null(),
            npub.as_ptr(),
            rst.as_ptr() as *const c_void,
        );
        assert_eq!((a, cl), (b, rl), "aes256gcm_encrypt_detached_afternm stub");
        assert_bytes_eq("aes256gcm_encrypt_detached_afternm stub c", &cbuf, &rbuf);
        assert_bytes_eq("aes256gcm_encrypt_detached_afternm stub mac", &cmac, &rmac);

        let mac = rng.vec(ab);
        let mut cbuf = vec![0xAAu8; 600];
        let mut rbuf = vec![0xAAu8; 600];
        let a = cdda(
            cbuf.as_mut_ptr(),
            std::ptr::null_mut(),
            msg.as_ptr(),
            100,
            mac.as_ptr(),
            ad.as_ptr(),
            10,
            npub.as_ptr(),
            cst.as_ptr() as *const c_void,
        );
        let b = rdda(
            rbuf.as_mut_ptr(),
            std::ptr::null_mut(),
            msg.as_ptr(),
            100,
            mac.as_ptr(),
            ad.as_ptr(),
            10,
            npub.as_ptr(),
            rst.as_ptr() as *const c_void,
        );
        assert_eq!(a, b, "aes256gcm_decrypt_detached_afternm stub");
        assert_bytes_eq("aes256gcm_decrypt_detached_afternm stub buffer", &cbuf, &rbuf);
    }
}

// ---------------------------------------------------------------------------
// aes256gcm precomputation API
// ---------------------------------------------------------------------------

type FnBeforenm = unsafe extern "C" fn(*mut c_void, *const c_uchar) -> c_int;
type FnEncryptAfternm = unsafe extern "C" fn(
    *mut c_uchar,
    *mut c_ulonglong,
    *const c_uchar,
    c_ulonglong,
    *const c_uchar,
    c_ulonglong,
    *const c_uchar,
    *const c_uchar,
    *const c_void,
) -> c_int;
type FnDecryptAfternm = unsafe extern "C" fn(
    *mut c_uchar,
    *mut c_ulonglong,
    *mut c_uchar,
    *const c_uchar,
    c_ulonglong,
    *const c_uchar,
    c_ulonglong,
    *const c_uchar,
    *const c_void,
) -> c_int;
type FnEncryptDetachedAfternm = unsafe extern "C" fn(
    *mut c_uchar,
    *mut c_uchar,
    *mut c_ulonglong,
    *const c_uchar,
    c_ulonglong,
    *const c_uchar,
    c_ulonglong,
    *const c_uchar,
    *const c_uchar,
    *const c_void,
) -> c_int;
type FnDecryptDetachedAfternm = unsafe extern "C" fn(
    *mut c_uchar,
    *mut c_uchar,
    *const c_uchar,
    c_ulonglong,
    *const c_uchar,
    *const c_uchar,
    c_ulonglong,
    *const c_uchar,
    *const c_void,
) -> c_int;

fn aes256gcm_afternm_suite() {
    unsafe {
        let (csb, _): (FnSize, FnSize) = pair("crypto_aead_aes256gcm_statebytes");
        let (ckb, _): (FnSize, FnSize) = pair("crypto_aead_aes256gcm_keybytes");
        let (cnb, _): (FnSize, FnSize) = pair("crypto_aead_aes256gcm_npubbytes");
        let (cab, _): (FnSize, FnSize) = pair("crypto_aead_aes256gcm_abytes");
        let sb = csb();
        let kb = ckb();
        let nb = cnb();
        let ab = cab();

        let (cbn, rbn): (FnBeforenm, FnBeforenm) = pair("crypto_aead_aes256gcm_beforenm");
        let (cea, rea): (FnEncryptAfternm, FnEncryptAfternm) =
            pair("crypto_aead_aes256gcm_encrypt_afternm");
        let (cda, rda): (FnDecryptAfternm, FnDecryptAfternm) =
            pair("crypto_aead_aes256gcm_decrypt_afternm");
        let (ceda, reda): (FnEncryptDetachedAfternm, FnEncryptDetachedAfternm) =
            pair("crypto_aead_aes256gcm_encrypt_detached_afternm");
        let (cdda, rdda): (FnDecryptDetachedAfternm, FnDecryptDetachedAfternm) =
            pair("crypto_aead_aes256gcm_decrypt_detached_afternm");

        let mut rng = Rng::new(0x4100);
        let msg = rng.vec(3001);
        let ad = rng.vec(1001);
        let mut keys: Vec<Vec<u8>> = vec![vec![0u8; kb], vec![0xffu8; kb]];
        keys.push(rng.vec(kb));
        let mut npubs: Vec<Vec<u8>> = vec![vec![0u8; nb], vec![0xffu8; nb]];
        npubs.push(rng.vec(nb));

        for key in &keys {
            // The precomputed state contains expanded round keys; compare it
            // byte-for-byte between the two implementations.
            let mut cst = AlignedBuf::new(sb, 0xA5);
            let mut rst = AlignedBuf::new(sb, 0xA5);
            let a = cbn(cst.as_mut_ptr() as *mut c_void, key.as_ptr());
            let b = rbn(rst.as_mut_ptr() as *mut c_void, key.as_ptr());
            assert_eq!(a, b, "aes256gcm_beforenm return");
            assert_bytes_eq("aes256gcm_beforenm state", cst.as_slice(), rst.as_slice());

            for npub in &npubs {
                for &mlen in &[0usize, 1, 15, 16, 17, 31, 32, 33, 63, 64, 65, 128, 1000, 3000] {
                    for &adlen in &[0usize, 1, 15, 16, 17, 32, 64, 500] {
                        let adptr = if adlen == 0 {
                            std::ptr::null()
                        } else {
                            ad.as_ptr()
                        };
                        let mut cc = vec![0xAAu8; mlen + ab + 8];
                        let mut rc = vec![0xAAu8; mlen + ab + 8];
                        let mut ccl: c_ulonglong = 0xdead;
                        let mut rcl: c_ulonglong = 0xdead;
                        let a = cea(
                            cc.as_mut_ptr(),
                            &mut ccl,
                            msg.as_ptr(),
                            mlen as c_ulonglong,
                            adptr,
                            adlen as c_ulonglong,
                            std::ptr::null(),
                            npub.as_ptr(),
                            cst.as_ptr() as *const c_void,
                        );
                        let b = rea(
                            rc.as_mut_ptr(),
                            &mut rcl,
                            msg.as_ptr(),
                            mlen as c_ulonglong,
                            adptr,
                            adlen as c_ulonglong,
                            std::ptr::null(),
                            npub.as_ptr(),
                            rst.as_ptr() as *const c_void,
                        );
                        let tag = format!("aes256gcm_encrypt_afternm(mlen={mlen},adlen={adlen})");
                        assert_eq!(a, b, "{tag} return");
                        assert_eq!(ccl, rcl, "{tag} clen");
                        assert_bytes_eq(&tag, &cc, &rc);

                        let mut cm = vec![0xAAu8; mlen + 8];
                        let mut rm = vec![0xAAu8; mlen + 8];
                        let mut cml: c_ulonglong = 0xdead;
                        let mut rml: c_ulonglong = 0xdead;
                        let a = cda(
                            cm.as_mut_ptr(),
                            &mut cml,
                            std::ptr::null_mut(),
                            cc.as_ptr(),
                            ccl,
                            adptr,
                            adlen as c_ulonglong,
                            npub.as_ptr(),
                            cst.as_ptr() as *const c_void,
                        );
                        let b = rda(
                            rm.as_mut_ptr(),
                            &mut rml,
                            std::ptr::null_mut(),
                            cc.as_ptr(),
                            ccl,
                            adptr,
                            adlen as c_ulonglong,
                            npub.as_ptr(),
                            rst.as_ptr() as *const c_void,
                        );
                        let dtag = format!("aes256gcm_decrypt_afternm(mlen={mlen},adlen={adlen})");
                        assert_eq!(a, b, "{dtag} return");
                        assert_eq!(cml, rml, "{dtag} mlen");
                        assert_bytes_eq(&dtag, &cm, &rm);
                        assert_eq!(&cm[..mlen], &msg[..mlen], "{dtag} roundtrip");

                        for bad in tamperings(&cc[..ccl as usize], ab, mlen) {
                            let mut cm = vec![0xAAu8; mlen + 8];
                            let mut rm = vec![0xAAu8; mlen + 8];
                            let mut cml: c_ulonglong = 0xdead;
                            let mut rml: c_ulonglong = 0xdead;
                            let a = cda(
                                cm.as_mut_ptr(),
                                &mut cml,
                                std::ptr::null_mut(),
                                bad.as_ptr(),
                                bad.len() as c_ulonglong,
                                adptr,
                                adlen as c_ulonglong,
                                npub.as_ptr(),
                                cst.as_ptr() as *const c_void,
                            );
                            let b = rda(
                                rm.as_mut_ptr(),
                                &mut rml,
                                std::ptr::null_mut(),
                                bad.as_ptr(),
                                bad.len() as c_ulonglong,
                                adptr,
                                adlen as c_ulonglong,
                                npub.as_ptr(),
                                rst.as_ptr() as *const c_void,
                            );
                            assert_eq!(a, b, "{dtag} tampered return");
                            assert_eq!(cml, rml, "{dtag} tampered mlen");
                            assert_bytes_eq(&format!("{dtag} tampered"), &cm, &rm);
                        }

                        // detached afternm
                        let mut cc = vec![0xAAu8; mlen + 8];
                        let mut rc = vec![0xAAu8; mlen + 8];
                        let mut cmac = vec![0xAAu8; ab + 8];
                        let mut rmac = vec![0xAAu8; ab + 8];
                        let mut cmacl: c_ulonglong = 0xdead;
                        let mut rmacl: c_ulonglong = 0xdead;
                        let a = ceda(
                            cc.as_mut_ptr(),
                            cmac.as_mut_ptr(),
                            &mut cmacl,
                            msg.as_ptr(),
                            mlen as c_ulonglong,
                            adptr,
                            adlen as c_ulonglong,
                            std::ptr::null(),
                            npub.as_ptr(),
                            cst.as_ptr() as *const c_void,
                        );
                        let b = reda(
                            rc.as_mut_ptr(),
                            rmac.as_mut_ptr(),
                            &mut rmacl,
                            msg.as_ptr(),
                            mlen as c_ulonglong,
                            adptr,
                            adlen as c_ulonglong,
                            std::ptr::null(),
                            npub.as_ptr(),
                            rst.as_ptr() as *const c_void,
                        );
                        let etag =
                            format!("aes256gcm_encrypt_detached_afternm(mlen={mlen},adlen={adlen})");
                        assert_eq!(a, b, "{etag} return");
                        assert_eq!(cmacl, rmacl, "{etag} maclen");
                        assert_bytes_eq(&format!("{etag} c"), &cc, &rc);
                        assert_bytes_eq(&format!("{etag} mac"), &cmac, &rmac);

                        let mut cm = vec![0xAAu8; mlen + 8];
                        let mut rm = vec![0xAAu8; mlen + 8];
                        let a = cdda(
                            cm.as_mut_ptr(),
                            std::ptr::null_mut(),
                            cc.as_ptr(),
                            mlen as c_ulonglong,
                            cmac.as_ptr(),
                            adptr,
                            adlen as c_ulonglong,
                            npub.as_ptr(),
                            cst.as_ptr() as *const c_void,
                        );
                        let b = rdda(
                            rm.as_mut_ptr(),
                            std::ptr::null_mut(),
                            cc.as_ptr(),
                            mlen as c_ulonglong,
                            cmac.as_ptr(),
                            adptr,
                            adlen as c_ulonglong,
                            npub.as_ptr(),
                            rst.as_ptr() as *const c_void,
                        );
                        let dtag2 =
                            format!("aes256gcm_decrypt_detached_afternm(mlen={mlen},adlen={adlen})");
                        assert_eq!(a, b, "{dtag2} return");
                        assert_bytes_eq(&dtag2, &cm, &rm);

                        let mut badmac = cmac[..ab].to_vec();
                        badmac[0] ^= 1;
                        let mut cm = vec![0xAAu8; mlen + 8];
                        let mut rm = vec![0xAAu8; mlen + 8];
                        let a = cdda(
                            cm.as_mut_ptr(),
                            std::ptr::null_mut(),
                            cc.as_ptr(),
                            mlen as c_ulonglong,
                            badmac.as_ptr(),
                            adptr,
                            adlen as c_ulonglong,
                            npub.as_ptr(),
                            cst.as_ptr() as *const c_void,
                        );
                        let b = rdda(
                            rm.as_mut_ptr(),
                            std::ptr::null_mut(),
                            cc.as_ptr(),
                            mlen as c_ulonglong,
                            badmac.as_ptr(),
                            adptr,
                            adlen as c_ulonglong,
                            npub.as_ptr(),
                            rst.as_ptr() as *const c_void,
                        );
                        assert_eq!(a, b, "{dtag2} bad mac return");
                        assert_bytes_eq(&format!("{dtag2} bad mac"), &cm, &rm);
                    }
                }
            }
        }
    }
}
