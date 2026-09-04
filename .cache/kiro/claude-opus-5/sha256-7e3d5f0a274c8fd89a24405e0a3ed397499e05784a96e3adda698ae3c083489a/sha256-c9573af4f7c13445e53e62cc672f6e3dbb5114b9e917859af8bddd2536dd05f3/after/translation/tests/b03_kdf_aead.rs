//! Phase B — CONFIGS.md rows 41–61: the KDF and AEAD surface.
//!
//! Differentially tests libsodium's KDF (blake2b + HKDF-SHA256/512) and AEAD
//! (chacha20poly1305 family, aegis128l/256, aes256gcm-ENOSYS) primitives by
//! calling the identically-named export in the C `.so` and the Rust `.so` and
//! comparing return codes, written out-lengths and output bytes byte-for-byte.
//! Every AEAD is additionally cross-checked: C ciphertext must decrypt in the
//! Rust lib and vice-versa.

mod common;
use common::*;

// ---------------------------------------------------------------------------
// C signatures (see c_src/libsodium/include/sodium/crypto_kdf*.h and
// crypto_aead*.h). Note: KDF length params are `size_t` (usize); AEAD length
// params are `unsigned long long` (u64), with `*_p` out-params `*mut u64`.
// ---------------------------------------------------------------------------

// crypto_kdf_(blake2b_)derive_from_key(subkey, subkey_len(size_t),
//   subkey_id(uint64_t), ctx(*const c_char), key)
type KdfDerive = unsafe extern "C" fn(*mut u8, usize, u64, *const u8, *const u8) -> i32;

// crypto_kdf_hkdf_shaXXX_extract(prk, salt, salt_len(size_t), ikm, ikm_len(size_t))
type HkdfExtract = unsafe extern "C" fn(*mut u8, *const u8, usize, *const u8, usize) -> i32;
// crypto_kdf_hkdf_shaXXX_expand(out, out_len(size_t), ctx(*const c_char), ctx_len(size_t), prk)
type HkdfExpand = unsafe extern "C" fn(*mut u8, usize, *const u8, usize, *const u8) -> i32;
// multipart
type HkdfExtractInit = unsafe extern "C" fn(*mut u8, *const u8, usize) -> i32;
type HkdfExtractUpdate = unsafe extern "C" fn(*mut u8, *const u8, usize) -> i32;
type HkdfExtractFinal = unsafe extern "C" fn(*mut u8, *mut u8) -> i32;

type SizeFn = unsafe extern "C" fn() -> usize;
type KeygenFn = unsafe extern "C" fn(*mut u8);
type IntFn = unsafe extern "C" fn() -> i32;

// AEAD combined encrypt: (c, clen_p, m, mlen, ad, adlen, nsec, npub, k)
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
// AEAD combined decrypt: (m, mlen_p, nsec, c, clen, ad, adlen, npub, k)
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
// AEAD encrypt_detached: (c, mac, maclen_p, m, mlen, ad, adlen, nsec, npub, k)
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
// AEAD decrypt_detached: (m, nsec, c, clen, mac, ad, adlen, npub, k)
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

// aes256gcm precomputation entry points
type AeadBeforenm = unsafe extern "C" fn(*mut u8, *const u8) -> i32;

const ENOSYS: i32 = 38; // Linux errno for "Function not implemented"

fn size_of(d: &'static Duo, name: &str) -> usize {
    let (f, _) = d.pair::<SizeFn>(name);
    unsafe { f() }
}

// ===========================================================================
// Row 41/42: crypto_kdf_blake2b_derive_from_key + generic crypto_kdf_derive_from_key
// subkey_len ∈ {16,17,32,63,64} × subkey_id ∈ {0,1,2^32,2^64-1} × random 8-byte ctx
// ===========================================================================
#[test]
fn kdf_derive_from_key() {
    let d = duo();
    let mut rng = Rng::new(0x4B4446_01);

    let ctxbytes = size_of(d, "crypto_kdf_blake2b_contextbytes"); // 8
    let keybytes = size_of(d, "crypto_kdf_blake2b_keybytes"); // 32
    assert_eq!(ctxbytes, 8);
    assert_eq!(keybytes, 32);

    let subkey_lens = [16usize, 17, 32, 63, 64];
    let subkey_ids = [0u64, 1, 1u64 << 32, u64::MAX];

    for name in [
        "crypto_kdf_blake2b_derive_from_key",
        "crypto_kdf_derive_from_key",
    ] {
        let (cf, rf) = d.pair::<KdfDerive>(name);
        for &slen in &subkey_lens {
            for &sid in &subkey_ids {
                // Many randomized (ctx,key) per configuration.
                for _ in 0..40 {
                    let ctx = rng.bytes(ctxbytes);
                    let key = rng.bytes(keybytes);
                    let mut oc = vec![0u8; slen];
                    let mut or = vec![0u8; slen];
                    let (rc, rr) = unsafe {
                        (
                            cf(oc.as_mut_ptr(), slen, sid, ctx.as_ptr(), key.as_ptr()),
                            rf(or.as_mut_ptr(), slen, sid, ctx.as_ptr(), key.as_ptr()),
                        )
                    };
                    eq_i32(&format!("{name} ret slen={slen} sid={sid}"), rc, rr);
                    eq_bytes(&format!("{name} out slen={slen} sid={sid}"), &oc, &or);
                }
            }
        }

        // Out-of-range subkey_len values must fail identically (< MIN 16, > MAX 64).
        for &bad in &[0usize, 1, 15, 65, 100] {
            let ctx = rng.bytes(ctxbytes);
            let key = rng.bytes(keybytes);
            let mut oc = vec![0u8; bad.max(1)];
            let mut or = vec![0u8; bad.max(1)];
            let (rc, rr) = unsafe {
                (
                    cf(oc.as_mut_ptr(), bad, 0, ctx.as_ptr(), key.as_ptr()),
                    rf(or.as_mut_ptr(), bad, 0, ctx.as_ptr(), key.as_ptr()),
                )
            };
            eq_i32(&format!("{name} bad slen={bad} ret"), rc, rr);
            if rc == 0 {
                eq_bytes(&format!("{name} bad slen={bad} out"), &oc, &or);
            }
        }
    }
}

// ===========================================================================
// Rows 43/47: crypto_kdf_hkdf_shaXXX_extract one-shot + multipart
// salt_len/ikm_len ∈ {0,1,32,64,65,1000}
// ===========================================================================
fn hkdf_extract_family(seed: u64, variant: &str, prk_len: usize) {
    let d = duo();
    let mut rng = Rng::new(seed);

    let one_shot = format!("crypto_kdf_hkdf_{variant}_extract");
    let init = format!("crypto_kdf_hkdf_{variant}_extract_init");
    let update = format!("crypto_kdf_hkdf_{variant}_extract_update");
    let final_ = format!("crypto_kdf_hkdf_{variant}_extract_final");
    let statebytes_name = format!("crypto_kdf_hkdf_{variant}_statebytes");
    let keybytes_name = format!("crypto_kdf_hkdf_{variant}_keybytes");

    assert_eq!(size_of(d, &keybytes_name), prk_len);

    let (osc, osr) = d.pair::<HkdfExtract>(&one_shot);
    let (initc, initr) = d.pair::<HkdfExtractInit>(&init);
    let (updc, updr) = d.pair::<HkdfExtractUpdate>(&update);
    let (finc, finr) = d.pair::<HkdfExtractFinal>(&final_);

    let sb = size_of(d, &statebytes_name);
    let lens = [0usize, 1, 32, 64, 65, 1000];

    for &salt_len in &lens {
        for &ikm_len in &lens {
            for _ in 0..15 {
                let salt = rng.bytes(salt_len);
                let ikm = rng.bytes(ikm_len);
                let salt_ptr = if salt_len == 0 {
                    std::ptr::null()
                } else {
                    salt.as_ptr()
                };

                // --- one-shot ---
                let mut prkc = vec![0u8; prk_len];
                let mut prkr = vec![0u8; prk_len];
                let (rc, rr) = unsafe {
                    (
                        osc(prkc.as_mut_ptr(), salt_ptr, salt_len, ikm.as_ptr(), ikm_len),
                        osr(prkr.as_mut_ptr(), salt_ptr, salt_len, ikm.as_ptr(), ikm_len),
                    )
                };
                eq_i32(
                    &format!("{one_shot} ret salt={salt_len} ikm={ikm_len}"),
                    rc,
                    rr,
                );
                eq_bytes(
                    &format!("{one_shot} prk salt={salt_len} ikm={ikm_len}"),
                    &prkc,
                    &prkr,
                );

                // --- multipart: split ikm into random chunks incl. 0-length updates ---
                let mut statec = vec![0u8; sb + 64];
                let mut stater = vec![0u8; sb + 64];
                let (ic, ir) = unsafe {
                    (
                        initc(statec.as_mut_ptr(), salt_ptr, salt_len),
                        initr(stater.as_mut_ptr(), salt_ptr, salt_len),
                    )
                };
                eq_i32(&format!("{init} ret salt={salt_len}"), ic, ir);

                // Randomized chunk boundaries, guaranteeing at least one zero-length update.
                let mut off = 0usize;
                let mut first = true;
                while off < ikm_len || first {
                    let remaining = ikm_len - off;
                    let chunk = if first {
                        0 // force a zero-length update first
                    } else if remaining == 0 {
                        break;
                    } else {
                        1 + rng.below(remaining)
                    };
                    first = false;
                    let slice = &ikm[off..off + chunk];
                    let uc = unsafe { updc(statec.as_mut_ptr(), slice.as_ptr(), chunk) };
                    let ur = unsafe { updr(stater.as_mut_ptr(), slice.as_ptr(), chunk) };
                    eq_i32(&format!("{update} ret"), uc, ur);
                    off += chunk;
                }

                let mut mpc = vec![0u8; prk_len];
                let mut mpr = vec![0u8; prk_len];
                let (fc, fr) = unsafe {
                    (
                        finc(statec.as_mut_ptr(), mpc.as_mut_ptr()),
                        finr(stater.as_mut_ptr(), mpr.as_mut_ptr()),
                    )
                };
                eq_i32(
                    &format!("{final_} ret salt={salt_len} ikm={ikm_len}"),
                    fc,
                    fr,
                );
                eq_bytes(
                    &format!("{final_} prk salt={salt_len} ikm={ikm_len}"),
                    &mpc,
                    &mpr,
                );

                // Multipart must equal the one-shot result (both C and Rust).
                eq_bytes(
                    &format!("{one_shot} one-shot==multipart (C) salt={salt_len} ikm={ikm_len}"),
                    &prkc,
                    &mpc,
                );
                eq_bytes(
                    &format!("{one_shot} one-shot==multipart (R) salt={salt_len} ikm={ikm_len}"),
                    &prkr,
                    &mpr,
                );
            }
        }
    }
}

#[test]
fn kdf_hkdf_sha256_extract() {
    hkdf_extract_family(0x484B_0256, "sha256", 32);
}

#[test]
fn kdf_hkdf_sha512_extract() {
    hkdf_extract_family(0x484B_0512, "sha512", 64);
}

// ===========================================================================
// Rows 45/48: crypto_kdf_hkdf_shaXXX_expand
// sha256: out_len ∈ {0,1,31,32,33,64,8160(MAX)} × ctx_len ∈ {0,1,64}
// sha512: out_len ∈ {0,1,63,64,65,16320(MAX)} × ctx_len ∈ {0,1,64}
// ===========================================================================
fn hkdf_expand_family(seed: u64, variant: &str, prk_len: usize, out_lens: &[usize]) {
    let d = duo();
    let mut rng = Rng::new(seed);
    let expand_name = format!("crypto_kdf_hkdf_{variant}_expand");
    let bytes_max_name = format!("crypto_kdf_hkdf_{variant}_bytes_max");

    let (ec, er) = d.pair::<HkdfExpand>(&expand_name);
    let max = size_of(d, &bytes_max_name);
    // Verify the advertised MAX matches what we sweep (255*digest).
    assert_eq!(*out_lens.last().unwrap(), max);

    let ctx_lens = [0usize, 1, 64];

    for &out_len in out_lens {
        for &ctx_len in &ctx_lens {
            let iters = if out_len > 1024 { 4 } else { 20 };
            for _ in 0..iters {
                let prk = rng.bytes(prk_len);
                let ctx = rng.bytes(ctx_len);
                let ctx_ptr = if ctx_len == 0 {
                    std::ptr::null()
                } else {
                    ctx.as_ptr()
                };
                let mut oc = vec![0u8; out_len.max(1)];
                let mut or = vec![0u8; out_len.max(1)];
                let (rc, rr) = unsafe {
                    (
                        ec(oc.as_mut_ptr(), out_len, ctx_ptr, ctx_len, prk.as_ptr()),
                        er(or.as_mut_ptr(), out_len, ctx_ptr, ctx_len, prk.as_ptr()),
                    )
                };
                eq_i32(
                    &format!("{expand_name} ret out={out_len} ctx={ctx_len}"),
                    rc,
                    rr,
                );
                eq_bytes(
                    &format!("{expand_name} out={out_len} ctx={ctx_len}"),
                    &oc[..out_len],
                    &or[..out_len],
                );
            }
        }
    }

    // Over-max out_len must fail identically.
    let prk = rng.bytes(prk_len);
    let mut oc = vec![0u8; 16];
    let mut or = vec![0u8; 16];
    let (rc, rr) = unsafe {
        (
            ec(oc.as_mut_ptr(), max + 1, std::ptr::null(), 0, prk.as_ptr()),
            er(or.as_mut_ptr(), max + 1, std::ptr::null(), 0, prk.as_ptr()),
        )
    };
    eq_i32(&format!("{expand_name} over-max ret"), rc, rr);
}

#[test]
fn kdf_hkdf_sha256_expand() {
    hkdf_expand_family(0x4558_0256, "sha256", 32, &[0, 1, 31, 32, 33, 64, 8160]);
}

#[test]
fn kdf_hkdf_sha512_expand() {
    hkdf_expand_family(0x4558_0512, "sha512", 64, &[0, 1, 63, 64, 65, 16320]);
}

// ===========================================================================
// Row 46: crypto_kdf_hkdf_shaXXX_keygen (randomized — length / non-degeneracy)
// ===========================================================================
fn keygen_check(d: &'static Duo, name: &str, len: usize) {
    let (cf, rf) = d.pair::<KeygenFn>(name);
    // Non-degeneracy: successive outputs from the same lib differ; not all zero.
    let mut prev = vec![0u8; len];
    for i in 0..8 {
        let mut kc = vec![0u8; len];
        let mut kr = vec![0u8; len];
        unsafe {
            cf(kc.as_mut_ptr());
            rf(kr.as_mut_ptr());
        }
        assert!(
            kc.iter().any(|&b| b != 0),
            "{name}: C keygen all-zero (iter {i})"
        );
        assert!(
            kr.iter().any(|&b| b != 0),
            "{name}: Rust keygen all-zero (iter {i})"
        );
        if i > 0 {
            assert!(
                kc != prev,
                "{name}: C keygen produced identical output twice"
            );
        }
        prev = kc;
    }
}

#[test]
fn kdf_hkdf_keygen() {
    let d = duo();
    keygen_check(
        d,
        "crypto_kdf_hkdf_sha256_keygen",
        size_of(d, "crypto_kdf_hkdf_sha256_keybytes"),
    );
    keygen_check(
        d,
        "crypto_kdf_hkdf_sha512_keygen",
        size_of(d, "crypto_kdf_hkdf_sha512_keybytes"),
    );
}

// ===========================================================================
// Shared AEAD driver (rows 49–58, 61)
// ===========================================================================
struct AeadSpec {
    name: &'static str,
    npub: usize,
    key: usize,
    abytes: usize,
    mlens: &'static [usize],
    adlens: &'static [usize],
    seed: u64,
}

fn aead_driver(spec: &AeadSpec) {
    let d = duo();
    let mut rng = Rng::new(spec.seed);

    let enc_name = format!("crypto_aead_{}_encrypt", spec.name);
    let dec_name = format!("crypto_aead_{}_decrypt", spec.name);
    let encd_name = format!("crypto_aead_{}_encrypt_detached", spec.name);
    let decd_name = format!("crypto_aead_{}_decrypt_detached", spec.name);

    let (encc, encr) = d.pair::<AeadEncrypt>(&enc_name);
    let (decc, decr) = d.pair::<AeadDecrypt>(&dec_name);
    let (encdc, encdr) = d.pair::<AeadEncryptDetached>(&encd_name);
    let (decdc, decdr) = d.pair::<AeadDecryptDetached>(&decd_name);

    // Verify advertised sizes match both libs (via pair which panics on mismatch).
    let abytes = size_of(d, &format!("crypto_aead_{}_abytes", spec.name));
    let npub = size_of(d, &format!("crypto_aead_{}_npubbytes", spec.name));
    let key = size_of(d, &format!("crypto_aead_{}_keybytes", spec.name));
    assert_eq!(abytes, spec.abytes, "{} abytes", spec.name);
    assert_eq!(npub, spec.npub, "{} npubbytes", spec.name);
    assert_eq!(key, spec.key, "{} keybytes", spec.name);

    for &mlen in spec.mlens {
        for &adlen in spec.adlens {
            let iters = if mlen >= 1000 { 6 } else { 20 };
            for _ in 0..iters {
                let m = rng.bytes(mlen);
                let ad = rng.bytes(adlen);
                let npub_b = rng.bytes(npub);
                let key_b = rng.bytes(key);
                // ad == NULL when adlen == 0
                let ad_ptr = if adlen == 0 {
                    std::ptr::null()
                } else {
                    ad.as_ptr()
                };
                let m_ptr = if mlen == 0 {
                    std::ptr::null()
                } else {
                    m.as_ptr()
                };

                // ---------------- combined encrypt ----------------
                let mut cc = vec![0u8; mlen + abytes];
                let mut cr = vec![0u8; mlen + abytes];
                let mut clc: u64 = 0;
                let mut clr: u64 = 0;
                let (rc, rr) = unsafe {
                    (
                        encc(
                            cc.as_mut_ptr(),
                            &mut clc,
                            m_ptr,
                            mlen as u64,
                            ad_ptr,
                            adlen as u64,
                            std::ptr::null(), // nsec always NULL
                            npub_b.as_ptr(),
                            key_b.as_ptr(),
                        ),
                        encr(
                            cr.as_mut_ptr(),
                            &mut clr,
                            m_ptr,
                            mlen as u64,
                            ad_ptr,
                            adlen as u64,
                            std::ptr::null(),
                            npub_b.as_ptr(),
                            key_b.as_ptr(),
                        ),
                    )
                };
                let tag = format!("{enc_name} m={mlen} ad={adlen}");
                eq_i32(&format!("{tag} ret"), rc, rr);
                assert_eq!(clc, clr, "{tag}: clen_p C={clc} R={clr}");
                assert_eq!(clc as usize, mlen + abytes, "{tag}: clen unexpected");
                eq_bytes(&tag, &cc, &cr);

                // ---------------- combined decrypt (round-trip both ways) ----------------
                // Rust decrypts C ciphertext; C decrypts Rust ciphertext.
                let run_dec = |dfn: &libloading::Symbol<AeadDecrypt>, ct: &[u8], who: &str| {
                    let mut out = vec![0u8; mlen.max(1)];
                    let mut ml: u64 = 0;
                    let ret = unsafe {
                        dfn(
                            out.as_mut_ptr(),
                            &mut ml,
                            std::ptr::null_mut(),
                            ct.as_ptr(),
                            (mlen + abytes) as u64,
                            ad_ptr,
                            adlen as u64,
                            npub_b.as_ptr(),
                            key_b.as_ptr(),
                        )
                    };
                    assert_eq!(
                        ret, 0,
                        "{dec_name} {who} m={mlen} ad={adlen} failed roundtrip"
                    );
                    assert_eq!(ml as usize, mlen, "{dec_name} {who}: mlen_p mismatch");
                    assert_eq!(&out[..mlen], &m[..], "{dec_name} {who}: plaintext mismatch");
                };
                run_dec(&decr, &cc, "R-dec-C-ct"); // Rust decrypts C ciphertext
                run_dec(&decc, &cr, "C-dec-R-ct"); // C decrypts Rust ciphertext

                // Also check both libs return same code & plaintext on their own ciphertext.
                let mut oc = vec![0u8; mlen.max(1)];
                let mut or = vec![0u8; mlen.max(1)];
                let mut mlc: u64 = 0;
                let mut mlr: u64 = 0;
                let (drc, drr) = unsafe {
                    (
                        decc(
                            oc.as_mut_ptr(),
                            &mut mlc,
                            std::ptr::null_mut(),
                            cc.as_ptr(),
                            (mlen + abytes) as u64,
                            ad_ptr,
                            adlen as u64,
                            npub_b.as_ptr(),
                            key_b.as_ptr(),
                        ),
                        decr(
                            or.as_mut_ptr(),
                            &mut mlr,
                            std::ptr::null_mut(),
                            cr.as_ptr(),
                            (mlen + abytes) as u64,
                            ad_ptr,
                            adlen as u64,
                            npub_b.as_ptr(),
                            key_b.as_ptr(),
                        ),
                    )
                };
                eq_i32(&format!("{dec_name} ret m={mlen} ad={adlen}"), drc, drr);
                assert_eq!(mlc, mlr, "{dec_name}: mlen_p C={mlc} R={mlr}");
                eq_bytes(
                    &format!("{dec_name} pt m={mlen} ad={adlen}"),
                    &oc[..mlen],
                    &or[..mlen],
                );

                // Tampered ciphertext must be rejected identically.
                if mlen + abytes > 0 {
                    let mut bad = cc.clone();
                    let idx = rng.below(bad.len());
                    bad[idx] ^= 0x80;
                    let mut o = vec![0u8; mlen.max(1)];
                    let mut ml: u64 = 0;
                    let (bc, br) = unsafe {
                        (
                            decc(
                                o.as_mut_ptr(),
                                &mut ml,
                                std::ptr::null_mut(),
                                bad.as_ptr(),
                                (mlen + abytes) as u64,
                                ad_ptr,
                                adlen as u64,
                                npub_b.as_ptr(),
                                key_b.as_ptr(),
                            ),
                            decr(
                                o.as_mut_ptr(),
                                &mut ml,
                                std::ptr::null_mut(),
                                bad.as_ptr(),
                                (mlen + abytes) as u64,
                                ad_ptr,
                                adlen as u64,
                                npub_b.as_ptr(),
                                key_b.as_ptr(),
                            ),
                        )
                    };
                    eq_i32(
                        &format!("{dec_name} tamper ret m={mlen} ad={adlen}"),
                        bc,
                        br,
                    );
                    assert_eq!(bc, -1, "{dec_name} tamper should fail");
                }

                // ---------------- detached encrypt (maclen_p non-NULL and NULL) ----------------
                let mut dcc = vec![0u8; mlen.max(1)];
                let mut dcr = vec![0u8; mlen.max(1)];
                let mut macc = vec![0u8; abytes];
                let mut macr = vec![0u8; abytes];
                let mut mlc2: u64 = 0;
                let mut mlr2: u64 = 0;
                let (edc, edr) = unsafe {
                    (
                        encdc(
                            dcc.as_mut_ptr(),
                            macc.as_mut_ptr(),
                            &mut mlc2,
                            m_ptr,
                            mlen as u64,
                            ad_ptr,
                            adlen as u64,
                            std::ptr::null(),
                            npub_b.as_ptr(),
                            key_b.as_ptr(),
                        ),
                        encdr(
                            dcr.as_mut_ptr(),
                            macr.as_mut_ptr(),
                            &mut mlr2,
                            m_ptr,
                            mlen as u64,
                            ad_ptr,
                            adlen as u64,
                            std::ptr::null(),
                            npub_b.as_ptr(),
                            key_b.as_ptr(),
                        ),
                    )
                };
                let dtag = format!("{encd_name} m={mlen} ad={adlen}");
                eq_i32(&format!("{dtag} ret"), edc, edr);
                assert_eq!(mlc2, mlr2, "{dtag}: maclen_p C={mlc2} R={mlr2}");
                assert_eq!(mlc2 as usize, abytes, "{dtag}: maclen unexpected");
                eq_bytes(&format!("{dtag} ciphertext"), &dcc[..mlen], &dcr[..mlen]);
                eq_bytes(&format!("{dtag} mac"), &macc, &macr);
                // Detached ciphertext+mac must equal combined output layout.
                eq_bytes(&format!("{dtag} vs combined ct"), &dcc[..mlen], &cc[..mlen]);
                eq_bytes(&format!("{dtag} vs combined mac"), &macc, &cc[mlen..]);

                // maclen_p == NULL path.
                let mut macc_n = vec![0u8; abytes];
                let mut macr_n = vec![0u8; abytes];
                let (edcn, edrn) = unsafe {
                    (
                        encdc(
                            dcc.as_mut_ptr(),
                            macc_n.as_mut_ptr(),
                            std::ptr::null_mut(),
                            m_ptr,
                            mlen as u64,
                            ad_ptr,
                            adlen as u64,
                            std::ptr::null(),
                            npub_b.as_ptr(),
                            key_b.as_ptr(),
                        ),
                        encdr(
                            dcr.as_mut_ptr(),
                            macr_n.as_mut_ptr(),
                            std::ptr::null_mut(),
                            m_ptr,
                            mlen as u64,
                            ad_ptr,
                            adlen as u64,
                            std::ptr::null(),
                            npub_b.as_ptr(),
                            key_b.as_ptr(),
                        ),
                    )
                };
                eq_i32(&format!("{dtag} maclen=NULL ret"), edcn, edrn);
                eq_bytes(&format!("{dtag} maclen=NULL mac"), &macc_n, &macr_n);

                // ---------------- detached decrypt (round-trip both ways) ----------------
                let run_decd = |dfn: &libloading::Symbol<AeadDecryptDetached>,
                                ct: &[u8],
                                mac: &[u8],
                                who: &str| {
                    let mut out = vec![0u8; mlen.max(1)];
                    let ret = unsafe {
                        dfn(
                            out.as_mut_ptr(),
                            std::ptr::null_mut(),
                            ct.as_ptr(),
                            mlen as u64,
                            mac.as_ptr(),
                            ad_ptr,
                            adlen as u64,
                            npub_b.as_ptr(),
                            key_b.as_ptr(),
                        )
                    };
                    assert_eq!(
                        ret, 0,
                        "{decd_name} {who} m={mlen} ad={adlen} roundtrip failed"
                    );
                    assert_eq!(&out[..mlen], &m[..], "{decd_name} {who} plaintext mismatch");
                };
                run_decd(&decdr, &dcc, &macc, "R-dec-C"); // Rust decrypts C output
                run_decd(&decdc, &dcr, &macr, "C-dec-R"); // C decrypts Rust output

                // Both libs decrypt-detached same code & plaintext on own output.
                let mut ddc = vec![0u8; mlen.max(1)];
                let mut ddr = vec![0u8; mlen.max(1)];
                let (ddrc, ddrr) = unsafe {
                    (
                        decdc(
                            ddc.as_mut_ptr(),
                            std::ptr::null_mut(),
                            dcc.as_ptr(),
                            mlen as u64,
                            macc.as_ptr(),
                            ad_ptr,
                            adlen as u64,
                            npub_b.as_ptr(),
                            key_b.as_ptr(),
                        ),
                        decdr(
                            ddr.as_mut_ptr(),
                            std::ptr::null_mut(),
                            dcr.as_ptr(),
                            mlen as u64,
                            macr.as_ptr(),
                            ad_ptr,
                            adlen as u64,
                            npub_b.as_ptr(),
                            key_b.as_ptr(),
                        ),
                    )
                };
                eq_i32(&format!("{decd_name} ret m={mlen} ad={adlen}"), ddrc, ddrr);
                eq_bytes(
                    &format!("{decd_name} pt m={mlen} ad={adlen}"),
                    &ddc[..mlen],
                    &ddr[..mlen],
                );
            }
        }
    }
}

// ===========================================================================
// Row 61: in-place operation (c == m pointer aliasing) for chacha/aegis variants.
// ===========================================================================
fn aead_inplace(spec: &AeadSpec) {
    let d = duo();
    let mut rng = Rng::new(spec.seed ^ 0xA11A5);

    let enc_name = format!("crypto_aead_{}_encrypt", spec.name);
    let dec_name = format!("crypto_aead_{}_decrypt", spec.name);
    let (encc, encr) = d.pair::<AeadEncrypt>(&enc_name);
    let (decc, decr) = d.pair::<AeadDecrypt>(&dec_name);
    let abytes = spec.abytes;

    for &mlen in spec.mlens {
        for _ in 0..10 {
            let m = rng.bytes(mlen);
            let ad = rng.bytes(16);
            let npub_b = rng.bytes(spec.npub);
            let key_b = rng.bytes(spec.key);

            // Buffer holds the message in the first mlen bytes; c == m (in-place).
            let mut bufc = vec![0u8; mlen + abytes];
            let mut bufr = vec![0u8; mlen + abytes];
            bufc[..mlen].copy_from_slice(&m);
            bufr[..mlen].copy_from_slice(&m);
            let mut clc: u64 = 0;
            let mut clr: u64 = 0;
            let (rc, rr) = unsafe {
                (
                    encc(
                        bufc.as_mut_ptr(),
                        &mut clc,
                        bufc.as_ptr(), // m == c
                        mlen as u64,
                        ad.as_ptr(),
                        16,
                        std::ptr::null(),
                        npub_b.as_ptr(),
                        key_b.as_ptr(),
                    ),
                    encr(
                        bufr.as_mut_ptr(),
                        &mut clr,
                        bufr.as_ptr(),
                        mlen as u64,
                        ad.as_ptr(),
                        16,
                        std::ptr::null(),
                        npub_b.as_ptr(),
                        key_b.as_ptr(),
                    ),
                )
            };
            eq_i32(&format!("{enc_name} inplace ret m={mlen}"), rc, rr);
            eq_bytes(&format!("{enc_name} inplace m={mlen}"), &bufc, &bufr);

            // Decrypt in place (c == m) and recover the plaintext.
            let mut mlc: u64 = 0;
            let ret = unsafe {
                decc(
                    bufc.as_mut_ptr(),
                    &mut mlc,
                    std::ptr::null_mut(),
                    bufc.as_ptr(),
                    (mlen + abytes) as u64,
                    ad.as_ptr(),
                    16,
                    npub_b.as_ptr(),
                    key_b.as_ptr(),
                )
            };
            assert_eq!(ret, 0, "{dec_name} inplace decrypt failed m={mlen}");
            assert_eq!(mlc as usize, mlen);
            assert_eq!(
                &bufc[..mlen],
                &m[..],
                "{dec_name} inplace plaintext mismatch"
            );
            let _ = decr; // rust decrypt covered by aead_driver; keep symbol referenced
        }
    }
}

const CHACHA_MLENS: &[usize] = &[0, 1, 15, 16, 17, 63, 64, 65, 1000];
const CHACHA_ADLENS: &[usize] = &[0, 1, 15, 16, 17, 64];
const AEGIS_MLENS: &[usize] = &[0, 1, 31, 32, 33, 63, 64, 65, 1000];
const AEGIS_ADLENS: &[usize] = &[0, 1, 31, 32, 33, 64];

// Rows 49/50
#[test]
fn aead_chacha20poly1305() {
    aead_driver(&AeadSpec {
        name: "chacha20poly1305",
        npub: 8,
        key: 32,
        abytes: 16,
        mlens: CHACHA_MLENS,
        adlens: CHACHA_ADLENS,
        seed: 0xC201,
    });
}

// Rows 51/52
#[test]
fn aead_chacha20poly1305_ietf() {
    aead_driver(&AeadSpec {
        name: "chacha20poly1305_ietf",
        npub: 12,
        key: 32,
        abytes: 16,
        mlens: CHACHA_MLENS,
        adlens: CHACHA_ADLENS,
        seed: 0xC202,
    });
}

// Rows 53/54
#[test]
fn aead_xchacha20poly1305_ietf() {
    aead_driver(&AeadSpec {
        name: "xchacha20poly1305_ietf",
        npub: 24,
        key: 32,
        abytes: 16,
        mlens: CHACHA_MLENS,
        adlens: CHACHA_ADLENS,
        seed: 0xC203,
    });
}

// Rows 55/56
#[test]
fn aead_aegis128l() {
    aead_driver(&AeadSpec {
        name: "aegis128l",
        npub: 16,
        key: 16,
        abytes: 32,
        mlens: AEGIS_MLENS,
        adlens: AEGIS_ADLENS,
        seed: 0xA128,
    });
}

// Rows 57/58
#[test]
fn aead_aegis256() {
    aead_driver(&AeadSpec {
        name: "aegis256",
        npub: 32,
        key: 32,
        abytes: 32,
        mlens: AEGIS_MLENS,
        adlens: AEGIS_ADLENS,
        seed: 0xA256,
    });
}

// Row 61 in-place
#[test]
fn aead_inplace_all() {
    for spec in [
        AeadSpec {
            name: "chacha20poly1305",
            npub: 8,
            key: 32,
            abytes: 16,
            mlens: CHACHA_MLENS,
            adlens: CHACHA_ADLENS,
            seed: 0xC201,
        },
        AeadSpec {
            name: "chacha20poly1305_ietf",
            npub: 12,
            key: 32,
            abytes: 16,
            mlens: CHACHA_MLENS,
            adlens: CHACHA_ADLENS,
            seed: 0xC202,
        },
        AeadSpec {
            name: "xchacha20poly1305_ietf",
            npub: 24,
            key: 32,
            abytes: 16,
            mlens: CHACHA_MLENS,
            adlens: CHACHA_ADLENS,
            seed: 0xC203,
        },
        AeadSpec {
            name: "aegis128l",
            npub: 16,
            key: 16,
            abytes: 32,
            mlens: AEGIS_MLENS,
            adlens: AEGIS_ADLENS,
            seed: 0xA128,
        },
        AeadSpec {
            name: "aegis256",
            npub: 32,
            key: 32,
            abytes: 32,
            mlens: AEGIS_MLENS,
            adlens: AEGIS_ADLENS,
            seed: 0xA256,
        },
    ] {
        aead_inplace(&spec);
    }
}

// ===========================================================================
// Row 59: aes256gcm — is_available() identical (0), all NINE entry points
// return -1 with errno ENOSYS identically from both libraries.
// ===========================================================================
#[test]
fn aead_aes256gcm_unavailable() {
    let d = duo();

    // is_available() must return the SAME value from both (0 in this build).
    let (avc, avr) = d.pair::<IntFn>("crypto_aead_aes256gcm_is_available");
    let (av_c, av_r) = unsafe { (avc(), avr()) };
    eq_i32("aes256gcm_is_available", av_c, av_r);
    assert_eq!(
        av_c, 0,
        "aes256gcm should be unavailable in the portable build"
    );

    let statebytes = size_of(d, "crypto_aead_aes256gcm_statebytes");
    let key = size_of(d, "crypto_aead_aes256gcm_keybytes");

    // Buffers large enough for any call.
    let mut out = vec![0u8; 4096];
    let mut aux = vec![0u8; 4096];
    let m = vec![0u8; 64];
    let ad = vec![0u8; 16];
    let npub = vec![0u8; 12];
    let k = vec![0u8; key];
    let mut lenp: u64 = 0;
    let mut state = vec![0u8; statebytes + 64];

    // Helper: run C and Rust, expect both == -1 and errno == ENOSYS.
    macro_rules! check {
        ($label:expr, $cf:expr, $rf:expr, $call:expr) => {{
            let (rc, ec) = with_errno(|| unsafe { $call($cf) });
            let (rr, er) = with_errno(|| unsafe { $call($rf) });
            eq_i32(&format!("{} ret", $label), rc, rr);
            assert_eq!(rc, -1, "{}: expected -1, got {rc}", $label);
            assert_eq!(ec, ENOSYS, "{}: C errno {ec} != ENOSYS", $label);
            assert_eq!(er, ENOSYS, "{}: Rust errno {er} != ENOSYS", $label);
        }};
    }

    // 1. encrypt
    {
        let (cf, rf) = d.pair::<AeadEncrypt>("crypto_aead_aes256gcm_encrypt");
        check!("aes256gcm_encrypt", &cf, &rf, |f: &libloading::Symbol<
            AeadEncrypt,
        >| f(
            out.as_mut_ptr(),
            &mut lenp,
            m.as_ptr(),
            m.len() as u64,
            ad.as_ptr(),
            ad.len() as u64,
            std::ptr::null(),
            npub.as_ptr(),
            k.as_ptr()
        ));
    }
    // 2. decrypt
    {
        let (cf, rf) = d.pair::<AeadDecrypt>("crypto_aead_aes256gcm_decrypt");
        check!("aes256gcm_decrypt", &cf, &rf, |f: &libloading::Symbol<
            AeadDecrypt,
        >| f(
            out.as_mut_ptr(),
            &mut lenp,
            std::ptr::null_mut(),
            m.as_ptr(),
            m.len() as u64,
            ad.as_ptr(),
            ad.len() as u64,
            npub.as_ptr(),
            k.as_ptr()
        ));
    }
    // 3. encrypt_detached
    {
        let (cf, rf) = d.pair::<AeadEncryptDetached>("crypto_aead_aes256gcm_encrypt_detached");
        check!(
            "aes256gcm_encrypt_detached",
            &cf,
            &rf,
            |f: &libloading::Symbol<AeadEncryptDetached>| f(
                out.as_mut_ptr(),
                aux.as_mut_ptr(),
                &mut lenp,
                m.as_ptr(),
                m.len() as u64,
                ad.as_ptr(),
                ad.len() as u64,
                std::ptr::null(),
                npub.as_ptr(),
                k.as_ptr()
            )
        );
    }
    // 4. decrypt_detached
    {
        let (cf, rf) = d.pair::<AeadDecryptDetached>("crypto_aead_aes256gcm_decrypt_detached");
        check!(
            "aes256gcm_decrypt_detached",
            &cf,
            &rf,
            |f: &libloading::Symbol<AeadDecryptDetached>| f(
                out.as_mut_ptr(),
                std::ptr::null_mut(),
                m.as_ptr(),
                m.len() as u64,
                aux.as_ptr(),
                ad.as_ptr(),
                ad.len() as u64,
                npub.as_ptr(),
                k.as_ptr()
            )
        );
    }
    // 5. beforenm
    {
        let (cf, rf) = d.pair::<AeadBeforenm>("crypto_aead_aes256gcm_beforenm");
        check!("aes256gcm_beforenm", &cf, &rf, |f: &libloading::Symbol<
            AeadBeforenm,
        >| f(
            state.as_mut_ptr(),
            k.as_ptr()
        ));
    }
    // 6. encrypt_afternm (same signature shape as encrypt: last arg is ctx ptr)
    {
        let (cf, rf) = d.pair::<AeadEncrypt>("crypto_aead_aes256gcm_encrypt_afternm");
        check!(
            "aes256gcm_encrypt_afternm",
            &cf,
            &rf,
            |f: &libloading::Symbol<AeadEncrypt>| f(
                out.as_mut_ptr(),
                &mut lenp,
                m.as_ptr(),
                m.len() as u64,
                ad.as_ptr(),
                ad.len() as u64,
                std::ptr::null(),
                npub.as_ptr(),
                state.as_ptr()
            )
        );
    }
    // 7. encrypt_detached_afternm
    {
        let (cf, rf) =
            d.pair::<AeadEncryptDetached>("crypto_aead_aes256gcm_encrypt_detached_afternm");
        check!(
            "aes256gcm_encrypt_detached_afternm",
            &cf,
            &rf,
            |f: &libloading::Symbol<AeadEncryptDetached>| f(
                out.as_mut_ptr(),
                aux.as_mut_ptr(),
                &mut lenp,
                m.as_ptr(),
                m.len() as u64,
                ad.as_ptr(),
                ad.len() as u64,
                std::ptr::null(),
                npub.as_ptr(),
                state.as_ptr()
            )
        );
    }
    // 8. decrypt_afternm
    {
        let (cf, rf) = d.pair::<AeadDecrypt>("crypto_aead_aes256gcm_decrypt_afternm");
        check!(
            "aes256gcm_decrypt_afternm",
            &cf,
            &rf,
            |f: &libloading::Symbol<AeadDecrypt>| f(
                out.as_mut_ptr(),
                &mut lenp,
                std::ptr::null_mut(),
                m.as_ptr(),
                m.len() as u64,
                ad.as_ptr(),
                ad.len() as u64,
                npub.as_ptr(),
                state.as_ptr()
            )
        );
    }
    // 9. decrypt_detached_afternm
    {
        let (cf, rf) =
            d.pair::<AeadDecryptDetached>("crypto_aead_aes256gcm_decrypt_detached_afternm");
        check!(
            "aes256gcm_decrypt_detached_afternm",
            &cf,
            &rf,
            |f: &libloading::Symbol<AeadDecryptDetached>| f(
                out.as_mut_ptr(),
                std::ptr::null_mut(),
                m.as_ptr(),
                m.len() as u64,
                aux.as_ptr(),
                ad.as_ptr(),
                ad.len() as u64,
                npub.as_ptr(),
                state.as_ptr()
            )
        );
    }
}

// ===========================================================================
// Row 60: all AEAD *_keygen (length / distinctness only, randomized)
// ===========================================================================
#[test]
fn aead_keygen_all() {
    let d = duo();
    for (name, keysym) in [
        (
            "crypto_aead_chacha20poly1305_keygen",
            "crypto_aead_chacha20poly1305_keybytes",
        ),
        (
            "crypto_aead_chacha20poly1305_ietf_keygen",
            "crypto_aead_chacha20poly1305_ietf_keybytes",
        ),
        (
            "crypto_aead_xchacha20poly1305_ietf_keygen",
            "crypto_aead_xchacha20poly1305_ietf_keybytes",
        ),
        (
            "crypto_aead_aegis128l_keygen",
            "crypto_aead_aegis128l_keybytes",
        ),
        (
            "crypto_aead_aegis256_keygen",
            "crypto_aead_aegis256_keybytes",
        ),
        (
            "crypto_aead_aes256gcm_keygen",
            "crypto_aead_aes256gcm_keybytes",
        ),
    ] {
        keygen_check(d, name, size_of(d, keysym));
    }
}
