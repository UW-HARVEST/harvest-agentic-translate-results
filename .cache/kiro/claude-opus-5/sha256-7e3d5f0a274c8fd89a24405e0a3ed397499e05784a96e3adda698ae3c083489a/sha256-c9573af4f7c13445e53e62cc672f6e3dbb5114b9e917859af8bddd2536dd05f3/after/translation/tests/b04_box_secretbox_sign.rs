//! Phase B — CONFIGS.md rows 67–93: secretbox, box (full / precomputed /
//! sealed / both primitives) and sign (detached / combined / multipart ed25519ph).
//!
//! Every entry point is called through BOTH shared libraries (C ground truth vs
//! the Rust translation) via `libloading`, and the outputs are compared
//! byte-for-byte and return-code-for-return-code. For every `*_verify` / `*_open`
//! we confirm the correct input is accepted and that each single-byte flip of the
//! signature/MAC is rejected with the SAME return code from both libraries.

mod common;
use common::*;

// ---------------------------------------------------------------------------
// C function-pointer types (exact signatures from
// c_src/libsodium/include/sodium/*.h). `unsigned long long` -> u64, size_t
// accessors -> usize.
// ---------------------------------------------------------------------------

// secretbox low-level: (c, m, mlen, n, k)
type SbLow = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, *const u8) -> i32;
// secretbox easy: (c, m, mlen, n, k)  /  open_easy: (m, c, clen, n, k)
type SbEasy = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, *const u8) -> i32;
// secretbox detached: (c, mac, m, mlen, n, k)
type SbDet = unsafe extern "C" fn(*mut u8, *mut u8, *const u8, u64, *const u8, *const u8) -> i32;
// secretbox open_detached: (m, c, mac, clen, n, k)
type SbOpenDet =
    unsafe extern "C" fn(*mut u8, *const u8, *const u8, u64, *const u8, *const u8) -> i32;
// xchacha secretbox easy/open_easy is same shape as SbEasy;
// xchacha detached/open_detached same as SbDet/SbOpenDet.

// keypair: (pk, sk) -> i32
type Keypair = unsafe extern "C" fn(*mut u8, *mut u8) -> i32;
// seed_keypair: (pk, sk, seed) -> i32
type SeedKeypair = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> i32;

// box easy: (c, m, mlen, n, pk, sk)  /  open_easy: (m, c, clen, n, pk, sk)
type BoxEasy =
    unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, *const u8, *const u8) -> i32;
// box detached: (c, mac, m, mlen, n, pk, sk)
type BoxDet =
    unsafe extern "C" fn(*mut u8, *mut u8, *const u8, u64, *const u8, *const u8, *const u8) -> i32;
// box open_detached: (m, c, mac, clen, n, pk, sk)
type BoxOpenDet = unsafe extern "C" fn(
    *mut u8,
    *const u8,
    *const u8,
    u64,
    *const u8,
    *const u8,
    *const u8,
) -> i32;
// beforenm: (k, pk, sk) -> i32
type Beforenm = unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> i32;
// afternm easy: (c, m, mlen, n, k)  == SbEasy shape
type AfternmEasy = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, *const u8) -> i32;
// afternm detached: (c, mac, m, mlen, n, k) == SbDet shape
type AfternmDet =
    unsafe extern "C" fn(*mut u8, *mut u8, *const u8, u64, *const u8, *const u8) -> i32;
// afternm open_detached: (m, c, mac, clen, n, k) == SbOpenDet shape
type AfternmOpenDet =
    unsafe extern "C" fn(*mut u8, *const u8, *const u8, u64, *const u8, *const u8) -> i32;
// seal: (c, m, mlen, pk) -> i32
type Seal = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8) -> i32;
// seal_open: (m, c, clen, pk, sk) -> i32
type SealOpen = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, *const u8) -> i32;

// sign_detached: (sig, siglen_p, m, mlen, sk)
type SignDetached = unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> i32;
// sign_verify_detached: (sig, m, mlen, pk)
type SignVerifyDetached = unsafe extern "C" fn(*const u8, *const u8, u64, *const u8) -> i32;
// sign (combined): (sm, smlen_p, m, mlen, sk)
type SignCombined = unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> i32;
// sign_open (combined): (m, mlen_p, sm, smlen, pk)
type SignOpen = unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> i32;
// sk_to_seed / sk_to_pk / pk_to_curve / sk_to_curve: (out, in) -> i32
type Convert2 = unsafe extern "C" fn(*mut u8, *const u8) -> i32;
// ph_init: (state) -> i32
type PhInit = unsafe extern "C" fn(*mut u8) -> i32;
// ph_update: (state, m, mlen) -> i32
type PhUpdate = unsafe extern "C" fn(*mut u8, *const u8, u64) -> i32;
// ph_final_create: (state, sig, siglen_p, sk) -> i32
type PhFinalCreate = unsafe extern "C" fn(*mut u8, *mut u8, *mut u64, *const u8) -> i32;
// ph_final_verify: (state, sig, pk) -> i32
type PhFinalVerify = unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> i32;

type Acc = unsafe extern "C" fn() -> usize;

fn acc(d: &'static Duo, name: &str) -> usize {
    let (f, _) = d.pair::<Acc>(name);
    unsafe { f() }
}

// secretbox constants
const SB_KEY: usize = 32;
const SB_NONCE: usize = 24;
const SB_MAC: usize = 16;
const SB_ZEROBYTES: usize = 32; // crypto_secretbox_xsalsa20poly1305_ZEROBYTES
const SB_BOXZEROBYTES: usize = 16;

// box constants (curve25519 for both primitives)
const BOX_PK: usize = 32;
const BOX_SK: usize = 32;
const BOX_NONCE: usize = 24;
const BOX_MAC: usize = 16;
const BOX_SEED: usize = 32;
const BOX_BEFORENM: usize = 32;
const BOX_SEAL_OVERHEAD: usize = BOX_PK + BOX_MAC; // 48
const BOX_ZEROBYTES: usize = 32; // curve25519xsalsa20poly1305_ZEROBYTES
const BOX_BOXZEROBYTES: usize = 16;

// sign constants
const SIGN_SEED: usize = 32;
const SIGN_PK: usize = 32;
const SIGN_SK: usize = 64;
const SIGN_BYTES: usize = 64;

// ===========================================================================
// F. secretbox
// ===========================================================================

/// Row 67: crypto_secretbox_xsalsa20poly1305 / _open — the zero-padded
/// low-level NaCl API. Input must have the first ZEROBYTES (32) bytes zero;
/// output ciphertext has the first BOXZEROBYTES (16) bytes zero.
#[test]
fn r67_secretbox_xsalsa20poly1305_lowlevel() {
    let d = duo();
    let (enc_c, enc_r) = d.pair::<SbLow>("crypto_secretbox_xsalsa20poly1305");
    let (dec_c, dec_r) = d.pair::<SbLow>("crypto_secretbox_xsalsa20poly1305_open");
    let mut rng = Rng::new(0x6700_0001);

    // mlen here is the total padded length (>= ZEROBYTES). CONFIGS row 67 sweep.
    for &mlen in &[32usize, 33, 47, 48, 49, 64, 96, 1000] {
        for _ in 0..40 {
            let key = rng.bytes(SB_KEY);
            let nonce = rng.bytes(SB_NONCE);
            // padded plaintext: first ZEROBYTES are zero, remainder random.
            let mut m = vec![0u8; mlen];
            if mlen > SB_ZEROBYTES {
                let tail = rng.bytes(mlen - SB_ZEROBYTES);
                m[SB_ZEROBYTES..].copy_from_slice(&tail);
            }
            let mut cc = vec![0u8; mlen];
            let mut cr = vec![0u8; mlen];
            let (rc, rr) = unsafe {
                (
                    enc_c(
                        cc.as_mut_ptr(),
                        m.as_ptr(),
                        mlen as u64,
                        nonce.as_ptr(),
                        key.as_ptr(),
                    ),
                    enc_r(
                        cr.as_mut_ptr(),
                        m.as_ptr(),
                        mlen as u64,
                        nonce.as_ptr(),
                        key.as_ptr(),
                    ),
                )
            };
            eq_i32(&format!("sb_low enc rc mlen={mlen}"), rc, rr);
            eq_bytes(&format!("sb_low ct mlen={mlen}"), &cc, &cr);

            // Decrypt (clen == mlen padded). Output first ZEROBYTES are zero.
            let mut oc = vec![0u8; mlen];
            let mut or = vec![0u8; mlen];
            let (dc, dr) = unsafe {
                (
                    dec_c(
                        oc.as_mut_ptr(),
                        cc.as_ptr(),
                        mlen as u64,
                        nonce.as_ptr(),
                        key.as_ptr(),
                    ),
                    dec_r(
                        or.as_mut_ptr(),
                        cr.as_ptr(),
                        mlen as u64,
                        nonce.as_ptr(),
                        key.as_ptr(),
                    ),
                )
            };
            eq_i32(&format!("sb_low dec rc mlen={mlen}"), dc, dr);
            assert_eq!(dc, 0, "sb_low dec should succeed mlen={mlen}");
            eq_bytes(&format!("sb_low pt mlen={mlen}"), &oc, &or);
            assert_eq!(
                &oc[SB_ZEROBYTES..],
                &m[SB_ZEROBYTES..],
                "sb_low roundtrip mlen={mlen}"
            );

            // Tamper: flip each byte of the MAC region (bytes BOXZEROBYTES..ZEROBYTES
            // of the ciphertext hold the poly1305 tag). Reject with identical rc.
            for i in SB_BOXZEROBYTES..SB_ZEROBYTES {
                let mut ct = cc.clone();
                ct[i] ^= 0x01;
                let mut t_oc = vec![0u8; mlen];
                let mut t_or = vec![0u8; mlen];
                let (tc, tr) = unsafe {
                    (
                        dec_c(
                            t_oc.as_mut_ptr(),
                            ct.as_ptr(),
                            mlen as u64,
                            nonce.as_ptr(),
                            key.as_ptr(),
                        ),
                        dec_r(
                            t_or.as_mut_ptr(),
                            ct.as_ptr(),
                            mlen as u64,
                            nonce.as_ptr(),
                            key.as_ptr(),
                        ),
                    )
                };
                eq_i32(&format!("sb_low tamper byte {i} mlen={mlen}"), tc, tr);
                assert_ne!(tc, 0, "sb_low tamper must be rejected byte {i} mlen={mlen}");
            }
        }
    }
}

/// Shared secretbox sweep length set for rows 68-72.
const SB_LENS: &[usize] = &[0, 1, 15, 16, 17, 31, 32, 33, 64, 1000];

/// Rows 68 & 70: crypto_secretbox_easy / _open_easy and the
/// xchacha20poly1305 equivalent.
#[test]
fn r68_r70_secretbox_easy() {
    let d = duo();
    let mut rng = Rng::new(0x6800_0001);
    for prefix in ["crypto_secretbox", "crypto_secretbox_xchacha20poly1305"] {
        let (enc_c, enc_r) = d.pair::<SbEasy>(&format!("{prefix}_easy"));
        let (dec_c, dec_r) = d.pair::<SbEasy>(&format!("{prefix}_open_easy"));
        for &mlen in SB_LENS {
            for _ in 0..30 {
                let key = rng.bytes(SB_KEY);
                let nonce = rng.bytes(SB_NONCE);
                let m = rng.bytes(mlen);
                let clen = mlen + SB_MAC;
                let mut cc = vec![0u8; clen];
                let mut cr = vec![0u8; clen];
                let (rc, rr) = unsafe {
                    (
                        enc_c(
                            cc.as_mut_ptr(),
                            m.as_ptr(),
                            mlen as u64,
                            nonce.as_ptr(),
                            key.as_ptr(),
                        ),
                        enc_r(
                            cr.as_mut_ptr(),
                            m.as_ptr(),
                            mlen as u64,
                            nonce.as_ptr(),
                            key.as_ptr(),
                        ),
                    )
                };
                eq_i32(&format!("{prefix} easy enc mlen={mlen}"), rc, rr);
                eq_bytes(&format!("{prefix} easy ct mlen={mlen}"), &cc, &cr);

                let mut oc = vec![0u8; mlen];
                let mut or = vec![0u8; mlen];
                let (dc, dr) = unsafe {
                    (
                        dec_c(
                            oc.as_mut_ptr(),
                            cc.as_ptr(),
                            clen as u64,
                            nonce.as_ptr(),
                            key.as_ptr(),
                        ),
                        dec_r(
                            or.as_mut_ptr(),
                            cr.as_ptr(),
                            clen as u64,
                            nonce.as_ptr(),
                            key.as_ptr(),
                        ),
                    )
                };
                eq_i32(&format!("{prefix} open_easy mlen={mlen}"), dc, dr);
                assert_eq!(dc, 0, "{prefix} open_easy should accept mlen={mlen}");
                eq_bytes(&format!("{prefix} open_easy pt mlen={mlen}"), &oc, &or);
                assert_eq!(oc, m, "{prefix} easy roundtrip mlen={mlen}");

                // Flip every byte of ciphertext (MAC is first 16 bytes).
                for i in 0..clen {
                    let mut ct = cc.clone();
                    ct[i] ^= 0x01;
                    let mut a = vec![0u8; mlen];
                    let mut b = vec![0u8; mlen];
                    let (tc, tr) = unsafe {
                        (
                            dec_c(
                                a.as_mut_ptr(),
                                ct.as_ptr(),
                                clen as u64,
                                nonce.as_ptr(),
                                key.as_ptr(),
                            ),
                            dec_r(
                                b.as_mut_ptr(),
                                ct.as_ptr(),
                                clen as u64,
                                nonce.as_ptr(),
                                key.as_ptr(),
                            ),
                        )
                    };
                    eq_i32(&format!("{prefix} easy tamper {i} mlen={mlen}"), tc, tr);
                    assert_ne!(tc, 0, "{prefix} easy tamper must reject {i} mlen={mlen}");
                }
            }
        }
    }
}

/// Rows 69 & 71: crypto_secretbox_detached / _open_detached and the
/// xchacha20poly1305 equivalent.
#[test]
fn r69_r71_secretbox_detached() {
    let d = duo();
    let mut rng = Rng::new(0x6900_0001);
    for prefix in ["crypto_secretbox", "crypto_secretbox_xchacha20poly1305"] {
        let (enc_c, enc_r) = d.pair::<SbDet>(&format!("{prefix}_detached"));
        let (dec_c, dec_r) = d.pair::<SbOpenDet>(&format!("{prefix}_open_detached"));
        for &mlen in SB_LENS {
            for _ in 0..30 {
                let key = rng.bytes(SB_KEY);
                let nonce = rng.bytes(SB_NONCE);
                let m = rng.bytes(mlen);
                let mut cc = vec![0u8; mlen];
                let mut cr = vec![0u8; mlen];
                let mut mac_c = vec![0u8; SB_MAC];
                let mut mac_r = vec![0u8; SB_MAC];
                let (rc, rr) = unsafe {
                    (
                        enc_c(
                            cc.as_mut_ptr(),
                            mac_c.as_mut_ptr(),
                            m.as_ptr(),
                            mlen as u64,
                            nonce.as_ptr(),
                            key.as_ptr(),
                        ),
                        enc_r(
                            cr.as_mut_ptr(),
                            mac_r.as_mut_ptr(),
                            m.as_ptr(),
                            mlen as u64,
                            nonce.as_ptr(),
                            key.as_ptr(),
                        ),
                    )
                };
                eq_i32(&format!("{prefix} det enc mlen={mlen}"), rc, rr);
                eq_bytes(&format!("{prefix} det ct mlen={mlen}"), &cc, &cr);
                eq_bytes(&format!("{prefix} det mac mlen={mlen}"), &mac_c, &mac_r);

                let mut oc = vec![0u8; mlen];
                let mut or = vec![0u8; mlen];
                let (dc, dr) = unsafe {
                    (
                        dec_c(
                            oc.as_mut_ptr(),
                            cc.as_ptr(),
                            mac_c.as_ptr(),
                            mlen as u64,
                            nonce.as_ptr(),
                            key.as_ptr(),
                        ),
                        dec_r(
                            or.as_mut_ptr(),
                            cr.as_ptr(),
                            mac_r.as_ptr(),
                            mlen as u64,
                            nonce.as_ptr(),
                            key.as_ptr(),
                        ),
                    )
                };
                eq_i32(&format!("{prefix} open_det mlen={mlen}"), dc, dr);
                assert_eq!(dc, 0, "{prefix} open_det accept mlen={mlen}");
                eq_bytes(&format!("{prefix} open_det pt mlen={mlen}"), &oc, &or);
                assert_eq!(oc, m, "{prefix} det roundtrip mlen={mlen}");

                // Flip each byte of the MAC.
                for i in 0..SB_MAC {
                    let mut mac = mac_c.clone();
                    mac[i] ^= 0x01;
                    let mut a = vec![0u8; mlen];
                    let mut b = vec![0u8; mlen];
                    let (tc, tr) = unsafe {
                        (
                            dec_c(
                                a.as_mut_ptr(),
                                cc.as_ptr(),
                                mac.as_ptr(),
                                mlen as u64,
                                nonce.as_ptr(),
                                key.as_ptr(),
                            ),
                            dec_r(
                                b.as_mut_ptr(),
                                cr.as_ptr(),
                                mac.as_ptr(),
                                mlen as u64,
                                nonce.as_ptr(),
                                key.as_ptr(),
                            ),
                        )
                    };
                    eq_i32(&format!("{prefix} det mac tamper {i} mlen={mlen}"), tc, tr);
                    assert_ne!(tc, 0, "{prefix} det mac tamper reject {i} mlen={mlen}");
                }
                // Flip each byte of ciphertext (when non-empty).
                for i in 0..mlen.min(24) {
                    let mut ct = cc.clone();
                    ct[i] ^= 0x01;
                    let mut a = vec![0u8; mlen];
                    let mut b = vec![0u8; mlen];
                    let (tc, tr) = unsafe {
                        (
                            dec_c(
                                a.as_mut_ptr(),
                                ct.as_ptr(),
                                mac_c.as_ptr(),
                                mlen as u64,
                                nonce.as_ptr(),
                                key.as_ptr(),
                            ),
                            dec_r(
                                b.as_mut_ptr(),
                                ct.as_ptr(),
                                mac_r.as_ptr(),
                                mlen as u64,
                                nonce.as_ptr(),
                                key.as_ptr(),
                            ),
                        )
                    };
                    eq_i32(&format!("{prefix} det ct tamper {i} mlen={mlen}"), tc, tr);
                    assert_ne!(tc, 0, "{prefix} det ct tamper reject {i} mlen={mlen}");
                }
            }
        }
    }
}

/// Row 72: secretbox in-place — c == m aliasing for easy and detached.
#[test]
fn r72_secretbox_inplace() {
    let d = duo();
    let mut rng = Rng::new(0x7200_0001);
    for prefix in ["crypto_secretbox", "crypto_secretbox_xchacha20poly1305"] {
        let (easy_c, easy_r) = d.pair::<SbEasy>(&format!("{prefix}_easy"));
        let (oeasy_c, oeasy_r) = d.pair::<SbEasy>(&format!("{prefix}_open_easy"));
        let (det_c, det_r) = d.pair::<SbDet>(&format!("{prefix}_detached"));
        let (odet_c, odet_r) = d.pair::<SbOpenDet>(&format!("{prefix}_open_detached"));
        for &mlen in SB_LENS {
            for _ in 0..25 {
                let key = rng.bytes(SB_KEY);
                let nonce = rng.bytes(SB_NONCE);
                let m = rng.bytes(mlen);

                // --- easy in-place: buffer holds m at offset MAC, c == buffer start.
                // libsodium supports c == m for easy (message at start, encrypted
                // in place with the tag prepended). Buffer of clen with m in the
                // first mlen bytes; call with c=buf, m=buf.
                let clen = mlen + SB_MAC;
                let mut bc = vec![0u8; clen];
                let mut br = vec![0u8; clen];
                bc[..mlen].copy_from_slice(&m);
                br[..mlen].copy_from_slice(&m);
                let (rc, rr) = unsafe {
                    (
                        easy_c(
                            bc.as_mut_ptr(),
                            bc.as_ptr(),
                            mlen as u64,
                            nonce.as_ptr(),
                            key.as_ptr(),
                        ),
                        easy_r(
                            br.as_mut_ptr(),
                            br.as_ptr(),
                            mlen as u64,
                            nonce.as_ptr(),
                            key.as_ptr(),
                        ),
                    )
                };
                eq_i32(&format!("{prefix} easy inplace enc mlen={mlen}"), rc, rr);
                eq_bytes(&format!("{prefix} easy inplace ct mlen={mlen}"), &bc, &br);
                // open in-place: m == c.
                let (dc, dr) = unsafe {
                    (
                        oeasy_c(
                            bc.as_mut_ptr(),
                            bc.as_ptr(),
                            clen as u64,
                            nonce.as_ptr(),
                            key.as_ptr(),
                        ),
                        oeasy_r(
                            br.as_mut_ptr(),
                            br.as_ptr(),
                            clen as u64,
                            nonce.as_ptr(),
                            key.as_ptr(),
                        ),
                    )
                };
                eq_i32(&format!("{prefix} easy inplace open mlen={mlen}"), dc, dr);
                assert_eq!(dc, 0, "{prefix} easy inplace open accept mlen={mlen}");
                eq_bytes(
                    &format!("{prefix} easy inplace pt mlen={mlen}"),
                    &bc[..mlen],
                    &br[..mlen],
                );
                assert_eq!(
                    &bc[..mlen],
                    &m[..],
                    "{prefix} easy inplace roundtrip mlen={mlen}"
                );

                // --- detached in-place: c == m.
                let mut dbc = m.clone();
                let mut dbr = m.clone();
                let mut mac_c = vec![0u8; SB_MAC];
                let mut mac_r = vec![0u8; SB_MAC];
                let (rc2, rr2) = unsafe {
                    (
                        det_c(
                            dbc.as_mut_ptr(),
                            mac_c.as_mut_ptr(),
                            dbc.as_ptr(),
                            mlen as u64,
                            nonce.as_ptr(),
                            key.as_ptr(),
                        ),
                        det_r(
                            dbr.as_mut_ptr(),
                            mac_r.as_mut_ptr(),
                            dbr.as_ptr(),
                            mlen as u64,
                            nonce.as_ptr(),
                            key.as_ptr(),
                        ),
                    )
                };
                eq_i32(&format!("{prefix} det inplace enc mlen={mlen}"), rc2, rr2);
                eq_bytes(&format!("{prefix} det inplace ct mlen={mlen}"), &dbc, &dbr);
                eq_bytes(
                    &format!("{prefix} det inplace mac mlen={mlen}"),
                    &mac_c,
                    &mac_r,
                );
                let (dc2, dr2) = unsafe {
                    (
                        odet_c(
                            dbc.as_mut_ptr(),
                            dbc.as_ptr(),
                            mac_c.as_ptr(),
                            mlen as u64,
                            nonce.as_ptr(),
                            key.as_ptr(),
                        ),
                        odet_r(
                            dbr.as_mut_ptr(),
                            dbr.as_ptr(),
                            mac_r.as_ptr(),
                            mlen as u64,
                            nonce.as_ptr(),
                            key.as_ptr(),
                        ),
                    )
                };
                eq_i32(&format!("{prefix} det inplace open mlen={mlen}"), dc2, dr2);
                assert_eq!(dc2, 0, "{prefix} det inplace open accept mlen={mlen}");
                eq_bytes(&format!("{prefix} det inplace pt mlen={mlen}"), &dbc, &dbr);
                assert_eq!(dbc, m, "{prefix} det inplace roundtrip mlen={mlen}");
            }
        }
    }
}

// ===========================================================================
// G. box
// ===========================================================================

/// Row 73: crypto_box_keypair (random — length + non-degenerate only) and
/// crypto_box_seed_keypair with FIXED seeds (byte-identical between libs).
#[test]
fn r73_box_keypair_seed() {
    let d = duo();
    let (sk_c, sk_r) = d.pair::<SeedKeypair>("crypto_box_seed_keypair");
    let mut rng = Rng::new(0x7300_0001);

    // Fixed seeds incl. all-zero and all-0xff and random.
    let mut seeds: Vec<Vec<u8>> = vec![vec![0u8; BOX_SEED], vec![0xffu8; BOX_SEED]];
    for _ in 0..60 {
        seeds.push(rng.bytes(BOX_SEED));
    }
    for (idx, seed) in seeds.iter().enumerate() {
        let mut pkc = vec![0u8; BOX_PK];
        let mut skc = vec![0u8; BOX_SK];
        let mut pkr = vec![0u8; BOX_PK];
        let mut skr = vec![0u8; BOX_SK];
        let (rc, rr) = unsafe {
            (
                sk_c(pkc.as_mut_ptr(), skc.as_mut_ptr(), seed.as_ptr()),
                sk_r(pkr.as_mut_ptr(), skr.as_mut_ptr(), seed.as_ptr()),
            )
        };
        eq_i32(&format!("box_seed_keypair rc seed#{idx}"), rc, rr);
        eq_bytes(&format!("box_seed_keypair pk seed#{idx}"), &pkc, &pkr);
        eq_bytes(&format!("box_seed_keypair sk seed#{idx}"), &skc, &skr);
    }

    // Random keypair: cannot compare bytes (randomized), only length + rc + non-degenerate.
    let (kp_c, kp_r) = d.pair::<Keypair>("crypto_box_keypair");
    for _ in 0..10 {
        let mut pkc = vec![0u8; BOX_PK];
        let mut skc = vec![0u8; BOX_SK];
        let mut pkr = vec![0u8; BOX_PK];
        let mut skr = vec![0u8; BOX_SK];
        let (rc, rr) = unsafe {
            (
                kp_c(pkc.as_mut_ptr(), skc.as_mut_ptr()),
                kp_r(pkr.as_mut_ptr(), skr.as_mut_ptr()),
            )
        };
        eq_i32("box_keypair rc", rc, rr);
        assert!(pkc.iter().any(|&b| b != 0), "box_keypair C pk degenerate");
        assert!(
            pkr.iter().any(|&b| b != 0),
            "box_keypair Rust pk degenerate"
        );
    }
}

/// Generate a deterministic pair of keypairs from an rng.
fn gen_box_pair(d: &'static Duo, rng: &mut Rng) -> ([u8; 32], [u8; 32], [u8; 32], [u8; 32]) {
    let (sk_c, _) = d.pair::<SeedKeypair>("crypto_box_seed_keypair");
    let seed_a = rng.bytes(BOX_SEED);
    let seed_b = rng.bytes(BOX_SEED);
    let (mut pka, mut ska, mut pkb, mut skb) = ([0u8; 32], [0u8; 32], [0u8; 32], [0u8; 32]);
    unsafe {
        sk_c(pka.as_mut_ptr(), ska.as_mut_ptr(), seed_a.as_ptr());
        sk_c(pkb.as_mut_ptr(), skb.as_mut_ptr(), seed_b.as_ptr());
    }
    (pka, ska, pkb, skb)
}

/// Row 74: crypto_box_beforenm + _afternm / _open_afternm (zero-padded
/// low-level API). afternm/open_afternm are the deprecated NaCl padded forms.
#[test]
fn r74_box_beforenm_afternm_lowlevel() {
    let d = duo();
    let (bnm_c, bnm_r) = d.pair::<Beforenm>("crypto_box_beforenm");
    let (enc_c, enc_r) = d.pair::<AfternmEasy>("crypto_box_afternm");
    let (dec_c, dec_r) = d.pair::<AfternmEasy>("crypto_box_open_afternm");
    let mut rng = Rng::new(0x7400_0001);

    for &mlen in &[32usize, 33, 64, 1000] {
        for _ in 0..30 {
            let (pka, _ska, pkb, skb) = gen_box_pair(d, &mut rng);
            // beforenm(k, pk_a, sk_b)
            let mut kc = vec![0u8; BOX_BEFORENM];
            let mut kr = vec![0u8; BOX_BEFORENM];
            let (bc, brr) = unsafe {
                (
                    bnm_c(kc.as_mut_ptr(), pka.as_ptr(), skb.as_ptr()),
                    bnm_r(kr.as_mut_ptr(), pka.as_ptr(), skb.as_ptr()),
                )
            };
            let _ = pkb;
            eq_i32("box_beforenm rc", bc, brr);
            eq_bytes("box_beforenm k", &kc, &kr);

            let nonce = rng.bytes(BOX_NONCE);
            // zero-padded plaintext (first ZEROBYTES zero).
            let mut m = vec![0u8; mlen];
            if mlen > BOX_ZEROBYTES {
                let tail = rng.bytes(mlen - BOX_ZEROBYTES);
                m[BOX_ZEROBYTES..].copy_from_slice(&tail);
            }
            let mut cc = vec![0u8; mlen];
            let mut cr = vec![0u8; mlen];
            let (rc, rr) = unsafe {
                (
                    enc_c(
                        cc.as_mut_ptr(),
                        m.as_ptr(),
                        mlen as u64,
                        nonce.as_ptr(),
                        kc.as_ptr(),
                    ),
                    enc_r(
                        cr.as_mut_ptr(),
                        m.as_ptr(),
                        mlen as u64,
                        nonce.as_ptr(),
                        kr.as_ptr(),
                    ),
                )
            };
            eq_i32(&format!("box_afternm enc mlen={mlen}"), rc, rr);
            eq_bytes(&format!("box_afternm ct mlen={mlen}"), &cc, &cr);

            let mut oc = vec![0u8; mlen];
            let mut or = vec![0u8; mlen];
            let (dc, dr) = unsafe {
                (
                    dec_c(
                        oc.as_mut_ptr(),
                        cc.as_ptr(),
                        mlen as u64,
                        nonce.as_ptr(),
                        kc.as_ptr(),
                    ),
                    dec_r(
                        or.as_mut_ptr(),
                        cr.as_ptr(),
                        mlen as u64,
                        nonce.as_ptr(),
                        kr.as_ptr(),
                    ),
                )
            };
            eq_i32(&format!("box_open_afternm mlen={mlen}"), dc, dr);
            assert_eq!(dc, 0, "box_open_afternm accept mlen={mlen}");
            eq_bytes(&format!("box_open_afternm pt mlen={mlen}"), &oc, &or);
            assert_eq!(
                &oc[BOX_ZEROBYTES..],
                &m[BOX_ZEROBYTES..],
                "box afternm roundtrip mlen={mlen}"
            );

            // Tamper each MAC byte (BOXZEROBYTES..ZEROBYTES).
            for i in BOX_BOXZEROBYTES..BOX_ZEROBYTES {
                let mut ct = cc.clone();
                ct[i] ^= 0x01;
                let mut a = vec![0u8; mlen];
                let mut b = vec![0u8; mlen];
                let (tc, tr) = unsafe {
                    (
                        dec_c(
                            a.as_mut_ptr(),
                            ct.as_ptr(),
                            mlen as u64,
                            nonce.as_ptr(),
                            kc.as_ptr(),
                        ),
                        dec_r(
                            b.as_mut_ptr(),
                            ct.as_ptr(),
                            mlen as u64,
                            nonce.as_ptr(),
                            kr.as_ptr(),
                        ),
                    )
                };
                eq_i32(&format!("box_afternm tamper {i} mlen={mlen}"), tc, tr);
                assert_ne!(tc, 0, "box_afternm tamper reject {i} mlen={mlen}");
            }
        }
    }
}

const BOX_LENS: &[usize] = &[0, 1, 16, 17, 64, 1000];

/// Rows 75-78: box easy / detached and their _afternm precomputed forms
/// (default crypto_box == curve25519xsalsa20poly1305).
#[test]
fn r75_78_box_easy_detached_afternm() {
    let d = duo();
    let mut rng = Rng::new(0x7500_0001);

    let (easy_c, easy_r) = d.pair::<BoxEasy>("crypto_box_easy");
    let (oeasy_c, oeasy_r) = d.pair::<BoxEasy>("crypto_box_open_easy");
    let (det_c, det_r) = d.pair::<BoxDet>("crypto_box_detached");
    let (odet_c, odet_r) = d.pair::<BoxOpenDet>("crypto_box_open_detached");
    let (bnm_c, bnm_r) = d.pair::<Beforenm>("crypto_box_beforenm");
    let (ea_c, ea_r) = d.pair::<AfternmEasy>("crypto_box_easy_afternm");
    let (oea_c, oea_r) = d.pair::<AfternmEasy>("crypto_box_open_easy_afternm");
    let (da_c, da_r) = d.pair::<AfternmDet>("crypto_box_detached_afternm");
    let (oda_c, oda_r) = d.pair::<AfternmOpenDet>("crypto_box_open_detached_afternm");

    for &mlen in BOX_LENS {
        for _ in 0..25 {
            let (pka, ska, pkb, skb) = gen_box_pair(d, &mut rng);
            let nonce = rng.bytes(BOX_NONCE);
            let m = rng.bytes(mlen);
            let clen = mlen + BOX_MAC;

            // Row 75: easy — encrypt with (pk_b, sk_a), decrypt with (pk_a, sk_b).
            let mut cc = vec![0u8; clen];
            let mut cr = vec![0u8; clen];
            let (rc, rr) = unsafe {
                (
                    easy_c(
                        cc.as_mut_ptr(),
                        m.as_ptr(),
                        mlen as u64,
                        nonce.as_ptr(),
                        pkb.as_ptr(),
                        ska.as_ptr(),
                    ),
                    easy_r(
                        cr.as_mut_ptr(),
                        m.as_ptr(),
                        mlen as u64,
                        nonce.as_ptr(),
                        pkb.as_ptr(),
                        ska.as_ptr(),
                    ),
                )
            };
            eq_i32(&format!("box_easy enc mlen={mlen}"), rc, rr);
            eq_bytes(&format!("box_easy ct mlen={mlen}"), &cc, &cr);
            let mut oc = vec![0u8; mlen];
            let mut or = vec![0u8; mlen];
            let (dc, dr) = unsafe {
                (
                    oeasy_c(
                        oc.as_mut_ptr(),
                        cc.as_ptr(),
                        clen as u64,
                        nonce.as_ptr(),
                        pka.as_ptr(),
                        skb.as_ptr(),
                    ),
                    oeasy_r(
                        or.as_mut_ptr(),
                        cr.as_ptr(),
                        clen as u64,
                        nonce.as_ptr(),
                        pka.as_ptr(),
                        skb.as_ptr(),
                    ),
                )
            };
            eq_i32(&format!("box_open_easy mlen={mlen}"), dc, dr);
            assert_eq!(dc, 0, "box_open_easy accept mlen={mlen}");
            eq_bytes(&format!("box_open_easy pt mlen={mlen}"), &oc, &or);
            assert_eq!(oc, m, "box_easy roundtrip mlen={mlen}");
            for i in 0..clen {
                let mut ct = cc.clone();
                ct[i] ^= 0x01;
                let mut a = vec![0u8; mlen];
                let mut b = vec![0u8; mlen];
                let (tc, tr) = unsafe {
                    (
                        oeasy_c(
                            a.as_mut_ptr(),
                            ct.as_ptr(),
                            clen as u64,
                            nonce.as_ptr(),
                            pka.as_ptr(),
                            skb.as_ptr(),
                        ),
                        oeasy_r(
                            b.as_mut_ptr(),
                            ct.as_ptr(),
                            clen as u64,
                            nonce.as_ptr(),
                            pka.as_ptr(),
                            skb.as_ptr(),
                        ),
                    )
                };
                eq_i32(&format!("box_easy tamper {i} mlen={mlen}"), tc, tr);
                assert_ne!(tc, 0, "box_easy tamper reject {i} mlen={mlen}");
            }

            // Row 76: detached.
            let mut dc_ct = vec![0u8; mlen];
            let mut dr_ct = vec![0u8; mlen];
            let mut mac_c = vec![0u8; BOX_MAC];
            let mut mac_r = vec![0u8; BOX_MAC];
            let (rc2, rr2) = unsafe {
                (
                    det_c(
                        dc_ct.as_mut_ptr(),
                        mac_c.as_mut_ptr(),
                        m.as_ptr(),
                        mlen as u64,
                        nonce.as_ptr(),
                        pkb.as_ptr(),
                        ska.as_ptr(),
                    ),
                    det_r(
                        dr_ct.as_mut_ptr(),
                        mac_r.as_mut_ptr(),
                        m.as_ptr(),
                        mlen as u64,
                        nonce.as_ptr(),
                        pkb.as_ptr(),
                        ska.as_ptr(),
                    ),
                )
            };
            eq_i32(&format!("box_detached enc mlen={mlen}"), rc2, rr2);
            eq_bytes(&format!("box_detached ct mlen={mlen}"), &dc_ct, &dr_ct);
            eq_bytes(&format!("box_detached mac mlen={mlen}"), &mac_c, &mac_r);
            let mut oc2 = vec![0u8; mlen];
            let mut or2 = vec![0u8; mlen];
            let (dc2, dr2) = unsafe {
                (
                    odet_c(
                        oc2.as_mut_ptr(),
                        dc_ct.as_ptr(),
                        mac_c.as_ptr(),
                        mlen as u64,
                        nonce.as_ptr(),
                        pka.as_ptr(),
                        skb.as_ptr(),
                    ),
                    odet_r(
                        or2.as_mut_ptr(),
                        dr_ct.as_ptr(),
                        mac_r.as_ptr(),
                        mlen as u64,
                        nonce.as_ptr(),
                        pka.as_ptr(),
                        skb.as_ptr(),
                    ),
                )
            };
            eq_i32(&format!("box_open_detached mlen={mlen}"), dc2, dr2);
            assert_eq!(dc2, 0, "box_open_detached accept mlen={mlen}");
            eq_bytes(&format!("box_open_detached pt mlen={mlen}"), &oc2, &or2);
            assert_eq!(oc2, m, "box_detached roundtrip mlen={mlen}");
            for i in 0..BOX_MAC {
                let mut mac = mac_c.clone();
                mac[i] ^= 0x01;
                let mut a = vec![0u8; mlen];
                let mut b = vec![0u8; mlen];
                let (tc, tr) = unsafe {
                    (
                        odet_c(
                            a.as_mut_ptr(),
                            dc_ct.as_ptr(),
                            mac.as_ptr(),
                            mlen as u64,
                            nonce.as_ptr(),
                            pka.as_ptr(),
                            skb.as_ptr(),
                        ),
                        odet_r(
                            b.as_mut_ptr(),
                            dr_ct.as_ptr(),
                            mac.as_ptr(),
                            mlen as u64,
                            nonce.as_ptr(),
                            pka.as_ptr(),
                            skb.as_ptr(),
                        ),
                    )
                };
                eq_i32(&format!("box_detached mac tamper {i} mlen={mlen}"), tc, tr);
                assert_ne!(tc, 0, "box_detached mac tamper reject {i} mlen={mlen}");
            }

            // Precompute shared key (row 77/78): beforenm(k, pk_b, sk_a) for
            // sender, and beforenm(k, pk_a, sk_b) for receiver — both must equal.
            let mut ka_c = vec![0u8; BOX_BEFORENM];
            let mut ka_r = vec![0u8; BOX_BEFORENM];
            let mut kb_c = vec![0u8; BOX_BEFORENM];
            let mut kb_r = vec![0u8; BOX_BEFORENM];
            unsafe {
                bnm_c(ka_c.as_mut_ptr(), pkb.as_ptr(), ska.as_ptr());
                bnm_r(ka_r.as_mut_ptr(), pkb.as_ptr(), ska.as_ptr());
                bnm_c(kb_c.as_mut_ptr(), pka.as_ptr(), skb.as_ptr());
                bnm_r(kb_r.as_mut_ptr(), pka.as_ptr(), skb.as_ptr());
            }
            eq_bytes("box_beforenm ka", &ka_c, &ka_r);
            eq_bytes("box_beforenm kb", &kb_c, &kb_r);

            // Row 77: easy_afternm.
            let mut ec = vec![0u8; clen];
            let mut er = vec![0u8; clen];
            let (r3c, r3r) = unsafe {
                (
                    ea_c(
                        ec.as_mut_ptr(),
                        m.as_ptr(),
                        mlen as u64,
                        nonce.as_ptr(),
                        ka_c.as_ptr(),
                    ),
                    ea_r(
                        er.as_mut_ptr(),
                        m.as_ptr(),
                        mlen as u64,
                        nonce.as_ptr(),
                        ka_r.as_ptr(),
                    ),
                )
            };
            eq_i32(&format!("box_easy_afternm enc mlen={mlen}"), r3c, r3r);
            eq_bytes(&format!("box_easy_afternm ct mlen={mlen}"), &ec, &er);
            let mut poc = vec![0u8; mlen];
            let mut por = vec![0u8; mlen];
            let (d3c, d3r) = unsafe {
                (
                    oea_c(
                        poc.as_mut_ptr(),
                        ec.as_ptr(),
                        clen as u64,
                        nonce.as_ptr(),
                        kb_c.as_ptr(),
                    ),
                    oea_r(
                        por.as_mut_ptr(),
                        er.as_ptr(),
                        clen as u64,
                        nonce.as_ptr(),
                        kb_r.as_ptr(),
                    ),
                )
            };
            eq_i32(&format!("box_open_easy_afternm mlen={mlen}"), d3c, d3r);
            assert_eq!(d3c, 0, "box_open_easy_afternm accept mlen={mlen}");
            eq_bytes(&format!("box_open_easy_afternm pt mlen={mlen}"), &poc, &por);
            assert_eq!(poc, m, "box_easy_afternm roundtrip mlen={mlen}");
            for i in 0..clen {
                let mut ct = ec.clone();
                ct[i] ^= 0x01;
                let mut a = vec![0u8; mlen];
                let mut b = vec![0u8; mlen];
                let (tc, tr) = unsafe {
                    (
                        oea_c(
                            a.as_mut_ptr(),
                            ct.as_ptr(),
                            clen as u64,
                            nonce.as_ptr(),
                            kb_c.as_ptr(),
                        ),
                        oea_r(
                            b.as_mut_ptr(),
                            ct.as_ptr(),
                            clen as u64,
                            nonce.as_ptr(),
                            kb_r.as_ptr(),
                        ),
                    )
                };
                eq_i32(&format!("box_easy_afternm tamper {i} mlen={mlen}"), tc, tr);
                assert_ne!(tc, 0, "box_easy_afternm tamper reject {i} mlen={mlen}");
            }

            // Row 78: detached_afternm.
            let mut dac = vec![0u8; mlen];
            let mut dar = vec![0u8; mlen];
            let mut dmac_c = vec![0u8; BOX_MAC];
            let mut dmac_r = vec![0u8; BOX_MAC];
            let (r4c, r4r) = unsafe {
                (
                    da_c(
                        dac.as_mut_ptr(),
                        dmac_c.as_mut_ptr(),
                        m.as_ptr(),
                        mlen as u64,
                        nonce.as_ptr(),
                        ka_c.as_ptr(),
                    ),
                    da_r(
                        dar.as_mut_ptr(),
                        dmac_r.as_mut_ptr(),
                        m.as_ptr(),
                        mlen as u64,
                        nonce.as_ptr(),
                        ka_r.as_ptr(),
                    ),
                )
            };
            eq_i32(&format!("box_detached_afternm enc mlen={mlen}"), r4c, r4r);
            eq_bytes(&format!("box_detached_afternm ct mlen={mlen}"), &dac, &dar);
            eq_bytes(
                &format!("box_detached_afternm mac mlen={mlen}"),
                &dmac_c,
                &dmac_r,
            );
            let mut o4c = vec![0u8; mlen];
            let mut o4r = vec![0u8; mlen];
            let (d4c, d4r) = unsafe {
                (
                    oda_c(
                        o4c.as_mut_ptr(),
                        dac.as_ptr(),
                        dmac_c.as_ptr(),
                        mlen as u64,
                        nonce.as_ptr(),
                        kb_c.as_ptr(),
                    ),
                    oda_r(
                        o4r.as_mut_ptr(),
                        dar.as_ptr(),
                        dmac_r.as_ptr(),
                        mlen as u64,
                        nonce.as_ptr(),
                        kb_r.as_ptr(),
                    ),
                )
            };
            eq_i32(&format!("box_open_detached_afternm mlen={mlen}"), d4c, d4r);
            assert_eq!(d4c, 0, "box_open_detached_afternm accept mlen={mlen}");
            eq_bytes(
                &format!("box_open_detached_afternm pt mlen={mlen}"),
                &o4c,
                &o4r,
            );
            assert_eq!(o4c, m, "box_detached_afternm roundtrip mlen={mlen}");
            for i in 0..BOX_MAC {
                let mut mac = dmac_c.clone();
                mac[i] ^= 0x01;
                let mut a = vec![0u8; mlen];
                let mut b = vec![0u8; mlen];
                let (tc, tr) = unsafe {
                    (
                        oda_c(
                            a.as_mut_ptr(),
                            dac.as_ptr(),
                            mac.as_ptr(),
                            mlen as u64,
                            nonce.as_ptr(),
                            kb_c.as_ptr(),
                        ),
                        oda_r(
                            b.as_mut_ptr(),
                            dar.as_ptr(),
                            mac.as_ptr(),
                            mlen as u64,
                            nonce.as_ptr(),
                            kb_r.as_ptr(),
                        ),
                    )
                };
                eq_i32(
                    &format!("box_detached_afternm mac tamper {i} mlen={mlen}"),
                    tc,
                    tr,
                );
                assert_ne!(
                    tc, 0,
                    "box_detached_afternm mac tamper reject {i} mlen={mlen}"
                );
            }
        }
    }
}

/// Row 79: crypto_box_seal / _seal_open. seal is randomized (ephemeral key) so
/// compare length + round-trip within each lib, AND cross-library: seal with C
/// then seal_open with Rust (and vice versa) must recover identical plaintext.
#[test]
fn r79_box_seal() {
    let d = duo();
    let (seal_c, seal_r) = d.pair::<Seal>("crypto_box_seal");
    let (open_c, open_r) = d.pair::<SealOpen>("crypto_box_seal_open");
    let mut rng = Rng::new(0x7900_0001);

    for &mlen in &[0usize, 1, 16, 17, 64, 1000] {
        for _ in 0..25 {
            let (pk, sk, _pkb, _skb) = gen_box_pair(d, &mut rng);
            let m = rng.bytes(mlen);
            let clen = mlen + BOX_SEAL_OVERHEAD;

            let mut cc = vec![0u8; clen];
            let mut cr = vec![0u8; clen];
            let (rc, rr) = unsafe {
                (
                    seal_c(cc.as_mut_ptr(), m.as_ptr(), mlen as u64, pk.as_ptr()),
                    seal_r(cr.as_mut_ptr(), m.as_ptr(), mlen as u64, pk.as_ptr()),
                )
            };
            eq_i32(&format!("box_seal rc mlen={mlen}"), rc, rr);
            assert_eq!(rc, 0, "box_seal ok mlen={mlen}");

            // Cross-library open: C ciphertext -> Rust open, and vice versa.
            let mut x1 = vec![0u8; mlen];
            let r1 = unsafe {
                open_r(
                    x1.as_mut_ptr(),
                    cc.as_ptr(),
                    clen as u64,
                    pk.as_ptr(),
                    sk.as_ptr(),
                )
            };
            assert_eq!(r1, 0, "box_seal_open C-seal/Rust-open accept mlen={mlen}");
            assert_eq!(
                x1, m,
                "box_seal_open C-seal/Rust-open plaintext mlen={mlen}"
            );
            let mut x2 = vec![0u8; mlen];
            let r2 = unsafe {
                open_c(
                    x2.as_mut_ptr(),
                    cr.as_ptr(),
                    clen as u64,
                    pk.as_ptr(),
                    sk.as_ptr(),
                )
            };
            assert_eq!(r2, 0, "box_seal_open Rust-seal/C-open accept mlen={mlen}");
            assert_eq!(
                x2, m,
                "box_seal_open Rust-seal/C-open plaintext mlen={mlen}"
            );
            // Also same-lib round-trips with matching return codes.
            let mut o_cc = vec![0u8; mlen];
            let mut o_rr = vec![0u8; mlen];
            let (dc, dr) = unsafe {
                (
                    open_c(
                        o_cc.as_mut_ptr(),
                        cc.as_ptr(),
                        clen as u64,
                        pk.as_ptr(),
                        sk.as_ptr(),
                    ),
                    open_r(
                        o_rr.as_mut_ptr(),
                        cr.as_ptr(),
                        clen as u64,
                        pk.as_ptr(),
                        sk.as_ptr(),
                    ),
                )
            };
            eq_i32(&format!("box_seal_open samelib rc mlen={mlen}"), dc, dr);
            assert_eq!(o_cc, m, "C seal roundtrip mlen={mlen}");
            assert_eq!(o_rr, m, "Rust seal roundtrip mlen={mlen}");

            // Tamper the C ciphertext: flip each byte in a bounded window; both
            // libs must reject with the same code. (Flipping the ephemeral pk
            // yields a different-but-valid pk, still failing the MAC.)
            for i in 0..clen.min(64) {
                let mut t = cc.clone();
                t[i] ^= 0x01;
                let mut a = vec![0u8; mlen];
                let mut b = vec![0u8; mlen];
                let (tc, tr) = unsafe {
                    (
                        open_c(
                            a.as_mut_ptr(),
                            t.as_ptr(),
                            clen as u64,
                            pk.as_ptr(),
                            sk.as_ptr(),
                        ),
                        open_r(
                            b.as_mut_ptr(),
                            t.as_ptr(),
                            clen as u64,
                            pk.as_ptr(),
                            sk.as_ptr(),
                        ),
                    )
                };
                eq_i32(&format!("box_seal_open tamper {i} mlen={mlen}"), tc, tr);
                assert_ne!(tc, 0, "box_seal_open tamper reject {i} mlen={mlen}");
            }
        }
    }
}

/// Generate a deterministic curve25519xsalsa20poly1305 keypair pair.
fn gen_pair_named(
    d: &'static Duo,
    rng: &mut Rng,
    seed_kp: &str,
) -> ([u8; 32], [u8; 32], [u8; 32], [u8; 32]) {
    let (sk_c, _) = d.pair::<SeedKeypair>(seed_kp);
    let seed_a = rng.bytes(BOX_SEED);
    let seed_b = rng.bytes(BOX_SEED);
    let (mut pka, mut ska, mut pkb, mut skb) = ([0u8; 32], [0u8; 32], [0u8; 32], [0u8; 32]);
    unsafe {
        sk_c(pka.as_mut_ptr(), ska.as_mut_ptr(), seed_a.as_ptr());
        sk_c(pkb.as_mut_ptr(), skb.as_mut_ptr(), seed_b.as_ptr());
    }
    (pka, ska, pkb, skb)
}

/// Rows 80-81: crypto_box_curve25519xsalsa20poly1305 / _open (raw) and its
/// _beforenm / _afternm / _open_afternm (all zero-padded low-level).
#[test]
fn r80_81_box_curve25519xsalsa20poly1305_raw() {
    let d = duo();
    let base = "crypto_box_curve25519xsalsa20poly1305";
    let (enc_c, enc_r) = d.pair::<BoxEasy>(base);
    let (dec_c, dec_r) = d.pair::<BoxEasy>(&format!("{base}_open"));
    let (bnm_c, bnm_r) = d.pair::<Beforenm>(&format!("{base}_beforenm"));
    let (anm_c, anm_r) = d.pair::<AfternmEasy>(&format!("{base}_afternm"));
    let (oanm_c, oanm_r) = d.pair::<AfternmEasy>(&format!("{base}_open_afternm"));
    let mut rng = Rng::new(0x8000_0001);

    for &mlen in &[32usize, 33, 64, 1000] {
        for _ in 0..30 {
            let (pka, ska, pkb, skb) = gen_pair_named(d, &mut rng, &format!("{base}_seed_keypair"));
            let nonce = rng.bytes(BOX_NONCE);
            let mut m = vec![0u8; mlen];
            if mlen > BOX_ZEROBYTES {
                let tail = rng.bytes(mlen - BOX_ZEROBYTES);
                m[BOX_ZEROBYTES..].copy_from_slice(&tail);
            }
            // Raw box: (c, m, mlen, n, pk_b, sk_a).
            let mut cc = vec![0u8; mlen];
            let mut cr = vec![0u8; mlen];
            let (rc, rr) = unsafe {
                (
                    enc_c(
                        cc.as_mut_ptr(),
                        m.as_ptr(),
                        mlen as u64,
                        nonce.as_ptr(),
                        pkb.as_ptr(),
                        ska.as_ptr(),
                    ),
                    enc_r(
                        cr.as_mut_ptr(),
                        m.as_ptr(),
                        mlen as u64,
                        nonce.as_ptr(),
                        pkb.as_ptr(),
                        ska.as_ptr(),
                    ),
                )
            };
            eq_i32(&format!("{base} enc mlen={mlen}"), rc, rr);
            eq_bytes(&format!("{base} ct mlen={mlen}"), &cc, &cr);
            let mut oc = vec![0u8; mlen];
            let mut or = vec![0u8; mlen];
            let (dc, dr) = unsafe {
                (
                    dec_c(
                        oc.as_mut_ptr(),
                        cc.as_ptr(),
                        mlen as u64,
                        nonce.as_ptr(),
                        pka.as_ptr(),
                        skb.as_ptr(),
                    ),
                    dec_r(
                        or.as_mut_ptr(),
                        cr.as_ptr(),
                        mlen as u64,
                        nonce.as_ptr(),
                        pka.as_ptr(),
                        skb.as_ptr(),
                    ),
                )
            };
            eq_i32(&format!("{base}_open mlen={mlen}"), dc, dr);
            assert_eq!(dc, 0, "{base}_open accept mlen={mlen}");
            eq_bytes(&format!("{base}_open pt mlen={mlen}"), &oc, &or);
            assert_eq!(
                &oc[BOX_ZEROBYTES..],
                &m[BOX_ZEROBYTES..],
                "{base} roundtrip mlen={mlen}"
            );

            // beforenm/afternm.
            let mut ka_c = vec![0u8; BOX_BEFORENM];
            let mut ka_r = vec![0u8; BOX_BEFORENM];
            let mut kb_c = vec![0u8; BOX_BEFORENM];
            let mut kb_r = vec![0u8; BOX_BEFORENM];
            unsafe {
                bnm_c(ka_c.as_mut_ptr(), pkb.as_ptr(), ska.as_ptr());
                bnm_r(ka_r.as_mut_ptr(), pkb.as_ptr(), ska.as_ptr());
                bnm_c(kb_c.as_mut_ptr(), pka.as_ptr(), skb.as_ptr());
                bnm_r(kb_r.as_mut_ptr(), pka.as_ptr(), skb.as_ptr());
            }
            eq_bytes(&format!("{base}_beforenm ka"), &ka_c, &ka_r);
            eq_bytes(&format!("{base}_beforenm kb"), &kb_c, &kb_r);

            let mut ec = vec![0u8; mlen];
            let mut er = vec![0u8; mlen];
            let (r2c, r2r) = unsafe {
                (
                    anm_c(
                        ec.as_mut_ptr(),
                        m.as_ptr(),
                        mlen as u64,
                        nonce.as_ptr(),
                        ka_c.as_ptr(),
                    ),
                    anm_r(
                        er.as_mut_ptr(),
                        m.as_ptr(),
                        mlen as u64,
                        nonce.as_ptr(),
                        ka_r.as_ptr(),
                    ),
                )
            };
            eq_i32(&format!("{base}_afternm mlen={mlen}"), r2c, r2r);
            eq_bytes(&format!("{base}_afternm ct mlen={mlen}"), &ec, &er);
            let mut poc = vec![0u8; mlen];
            let mut por = vec![0u8; mlen];
            let (d2c, d2r) = unsafe {
                (
                    oanm_c(
                        poc.as_mut_ptr(),
                        ec.as_ptr(),
                        mlen as u64,
                        nonce.as_ptr(),
                        kb_c.as_ptr(),
                    ),
                    oanm_r(
                        por.as_mut_ptr(),
                        er.as_ptr(),
                        mlen as u64,
                        nonce.as_ptr(),
                        kb_r.as_ptr(),
                    ),
                )
            };
            eq_i32(&format!("{base}_open_afternm mlen={mlen}"), d2c, d2r);
            assert_eq!(d2c, 0, "{base}_open_afternm accept mlen={mlen}");
            eq_bytes(&format!("{base}_open_afternm pt mlen={mlen}"), &poc, &por);
            assert_eq!(
                &poc[BOX_ZEROBYTES..],
                &m[BOX_ZEROBYTES..],
                "{base} afternm roundtrip mlen={mlen}"
            );

            for i in BOX_BOXZEROBYTES..BOX_ZEROBYTES {
                let mut ct = cc.clone();
                ct[i] ^= 0x01;
                let mut a = vec![0u8; mlen];
                let mut b = vec![0u8; mlen];
                let (tc, tr) = unsafe {
                    (
                        dec_c(
                            a.as_mut_ptr(),
                            ct.as_ptr(),
                            mlen as u64,
                            nonce.as_ptr(),
                            pka.as_ptr(),
                            skb.as_ptr(),
                        ),
                        dec_r(
                            b.as_mut_ptr(),
                            ct.as_ptr(),
                            mlen as u64,
                            nonce.as_ptr(),
                            pka.as_ptr(),
                            skb.as_ptr(),
                        ),
                    )
                };
                eq_i32(&format!("{base} tamper {i} mlen={mlen}"), tc, tr);
                assert_ne!(tc, 0, "{base} tamper reject {i} mlen={mlen}");
            }
        }
    }
}

/// Rows 82-85: all crypto_box_curve25519xchacha20poly1305_* forms:
/// easy/open_easy, detached/open_detached, beforenm + all four _afternm forms,
/// seal/seal_open.
#[test]
fn r82_85_box_curve25519xchacha20poly1305() {
    let d = duo();
    let base = "crypto_box_curve25519xchacha20poly1305";
    let (easy_c, easy_r) = d.pair::<BoxEasy>(&format!("{base}_easy"));
    let (oeasy_c, oeasy_r) = d.pair::<BoxEasy>(&format!("{base}_open_easy"));
    let (det_c, det_r) = d.pair::<BoxDet>(&format!("{base}_detached"));
    let (odet_c, odet_r) = d.pair::<BoxOpenDet>(&format!("{base}_open_detached"));
    let (bnm_c, bnm_r) = d.pair::<Beforenm>(&format!("{base}_beforenm"));
    let (ea_c, ea_r) = d.pair::<AfternmEasy>(&format!("{base}_easy_afternm"));
    let (oea_c, oea_r) = d.pair::<AfternmEasy>(&format!("{base}_open_easy_afternm"));
    let (da_c, da_r) = d.pair::<AfternmDet>(&format!("{base}_detached_afternm"));
    let (oda_c, oda_r) = d.pair::<AfternmOpenDet>(&format!("{base}_open_detached_afternm"));
    let (seal_c, seal_r) = d.pair::<Seal>(&format!("{base}_seal"));
    let (sopen_c, sopen_r) = d.pair::<SealOpen>(&format!("{base}_seal_open"));
    let mut rng = Rng::new(0x8200_0001);

    for &mlen in BOX_LENS {
        for _ in 0..20 {
            let (pka, ska, pkb, skb) = gen_pair_named(d, &mut rng, &format!("{base}_seed_keypair"));
            let nonce = rng.bytes(BOX_NONCE);
            let m = rng.bytes(mlen);
            let clen = mlen + BOX_MAC;

            // easy
            let mut cc = vec![0u8; clen];
            let mut cr = vec![0u8; clen];
            let (rc, rr) = unsafe {
                (
                    easy_c(
                        cc.as_mut_ptr(),
                        m.as_ptr(),
                        mlen as u64,
                        nonce.as_ptr(),
                        pkb.as_ptr(),
                        ska.as_ptr(),
                    ),
                    easy_r(
                        cr.as_mut_ptr(),
                        m.as_ptr(),
                        mlen as u64,
                        nonce.as_ptr(),
                        pkb.as_ptr(),
                        ska.as_ptr(),
                    ),
                )
            };
            eq_i32(&format!("{base}_easy mlen={mlen}"), rc, rr);
            eq_bytes(&format!("{base}_easy ct mlen={mlen}"), &cc, &cr);
            let mut oc = vec![0u8; mlen];
            let mut or = vec![0u8; mlen];
            let (dc, dr) = unsafe {
                (
                    oeasy_c(
                        oc.as_mut_ptr(),
                        cc.as_ptr(),
                        clen as u64,
                        nonce.as_ptr(),
                        pka.as_ptr(),
                        skb.as_ptr(),
                    ),
                    oeasy_r(
                        or.as_mut_ptr(),
                        cr.as_ptr(),
                        clen as u64,
                        nonce.as_ptr(),
                        pka.as_ptr(),
                        skb.as_ptr(),
                    ),
                )
            };
            eq_i32(&format!("{base}_open_easy mlen={mlen}"), dc, dr);
            assert_eq!(dc, 0, "{base}_open_easy accept mlen={mlen}");
            eq_bytes(&format!("{base}_open_easy pt mlen={mlen}"), &oc, &or);
            assert_eq!(oc, m, "{base}_easy roundtrip mlen={mlen}");
            for i in 0..clen {
                let mut ct = cc.clone();
                ct[i] ^= 0x01;
                let mut a = vec![0u8; mlen];
                let mut b = vec![0u8; mlen];
                let (tc, tr) = unsafe {
                    (
                        oeasy_c(
                            a.as_mut_ptr(),
                            ct.as_ptr(),
                            clen as u64,
                            nonce.as_ptr(),
                            pka.as_ptr(),
                            skb.as_ptr(),
                        ),
                        oeasy_r(
                            b.as_mut_ptr(),
                            ct.as_ptr(),
                            clen as u64,
                            nonce.as_ptr(),
                            pka.as_ptr(),
                            skb.as_ptr(),
                        ),
                    )
                };
                eq_i32(&format!("{base}_easy tamper {i} mlen={mlen}"), tc, tr);
                assert_ne!(tc, 0, "{base}_easy tamper reject {i} mlen={mlen}");
            }

            // detached
            let mut dcx = vec![0u8; mlen];
            let mut drx = vec![0u8; mlen];
            let mut mac_c = vec![0u8; BOX_MAC];
            let mut mac_r = vec![0u8; BOX_MAC];
            let (r2c, r2r) = unsafe {
                (
                    det_c(
                        dcx.as_mut_ptr(),
                        mac_c.as_mut_ptr(),
                        m.as_ptr(),
                        mlen as u64,
                        nonce.as_ptr(),
                        pkb.as_ptr(),
                        ska.as_ptr(),
                    ),
                    det_r(
                        drx.as_mut_ptr(),
                        mac_r.as_mut_ptr(),
                        m.as_ptr(),
                        mlen as u64,
                        nonce.as_ptr(),
                        pkb.as_ptr(),
                        ska.as_ptr(),
                    ),
                )
            };
            eq_i32(&format!("{base}_detached mlen={mlen}"), r2c, r2r);
            eq_bytes(&format!("{base}_detached ct mlen={mlen}"), &dcx, &drx);
            eq_bytes(&format!("{base}_detached mac mlen={mlen}"), &mac_c, &mac_r);
            let mut o2c = vec![0u8; mlen];
            let mut o2r = vec![0u8; mlen];
            let (d2c, d2r) = unsafe {
                (
                    odet_c(
                        o2c.as_mut_ptr(),
                        dcx.as_ptr(),
                        mac_c.as_ptr(),
                        mlen as u64,
                        nonce.as_ptr(),
                        pka.as_ptr(),
                        skb.as_ptr(),
                    ),
                    odet_r(
                        o2r.as_mut_ptr(),
                        drx.as_ptr(),
                        mac_r.as_ptr(),
                        mlen as u64,
                        nonce.as_ptr(),
                        pka.as_ptr(),
                        skb.as_ptr(),
                    ),
                )
            };
            eq_i32(&format!("{base}_open_detached mlen={mlen}"), d2c, d2r);
            assert_eq!(d2c, 0, "{base}_open_detached accept mlen={mlen}");
            eq_bytes(&format!("{base}_open_detached pt mlen={mlen}"), &o2c, &o2r);
            assert_eq!(o2c, m, "{base}_detached roundtrip mlen={mlen}");
            for i in 0..BOX_MAC {
                let mut mac = mac_c.clone();
                mac[i] ^= 0x01;
                let mut a = vec![0u8; mlen];
                let mut b = vec![0u8; mlen];
                let (tc, tr) = unsafe {
                    (
                        odet_c(
                            a.as_mut_ptr(),
                            dcx.as_ptr(),
                            mac.as_ptr(),
                            mlen as u64,
                            nonce.as_ptr(),
                            pka.as_ptr(),
                            skb.as_ptr(),
                        ),
                        odet_r(
                            b.as_mut_ptr(),
                            drx.as_ptr(),
                            mac.as_ptr(),
                            mlen as u64,
                            nonce.as_ptr(),
                            pka.as_ptr(),
                            skb.as_ptr(),
                        ),
                    )
                };
                eq_i32(
                    &format!("{base}_detached mac tamper {i} mlen={mlen}"),
                    tc,
                    tr,
                );
                assert_ne!(tc, 0, "{base}_detached mac tamper reject {i} mlen={mlen}");
            }

            // beforenm + all four afternm forms
            let mut ka_c = vec![0u8; BOX_BEFORENM];
            let mut ka_r = vec![0u8; BOX_BEFORENM];
            let mut kb_c = vec![0u8; BOX_BEFORENM];
            let mut kb_r = vec![0u8; BOX_BEFORENM];
            unsafe {
                bnm_c(ka_c.as_mut_ptr(), pkb.as_ptr(), ska.as_ptr());
                bnm_r(ka_r.as_mut_ptr(), pkb.as_ptr(), ska.as_ptr());
                bnm_c(kb_c.as_mut_ptr(), pka.as_ptr(), skb.as_ptr());
                bnm_r(kb_r.as_mut_ptr(), pka.as_ptr(), skb.as_ptr());
            }
            eq_bytes(&format!("{base}_beforenm ka"), &ka_c, &ka_r);
            eq_bytes(&format!("{base}_beforenm kb"), &kb_c, &kb_r);

            // easy_afternm + open_easy_afternm
            let mut ec = vec![0u8; clen];
            let mut er = vec![0u8; clen];
            let (r3c, r3r) = unsafe {
                (
                    ea_c(
                        ec.as_mut_ptr(),
                        m.as_ptr(),
                        mlen as u64,
                        nonce.as_ptr(),
                        ka_c.as_ptr(),
                    ),
                    ea_r(
                        er.as_mut_ptr(),
                        m.as_ptr(),
                        mlen as u64,
                        nonce.as_ptr(),
                        ka_r.as_ptr(),
                    ),
                )
            };
            eq_i32(&format!("{base}_easy_afternm mlen={mlen}"), r3c, r3r);
            eq_bytes(&format!("{base}_easy_afternm ct mlen={mlen}"), &ec, &er);
            let mut p3c = vec![0u8; mlen];
            let mut p3r = vec![0u8; mlen];
            let (d3c, d3r) = unsafe {
                (
                    oea_c(
                        p3c.as_mut_ptr(),
                        ec.as_ptr(),
                        clen as u64,
                        nonce.as_ptr(),
                        kb_c.as_ptr(),
                    ),
                    oea_r(
                        p3r.as_mut_ptr(),
                        er.as_ptr(),
                        clen as u64,
                        nonce.as_ptr(),
                        kb_r.as_ptr(),
                    ),
                )
            };
            eq_i32(&format!("{base}_open_easy_afternm mlen={mlen}"), d3c, d3r);
            assert_eq!(d3c, 0, "{base}_open_easy_afternm accept mlen={mlen}");
            eq_bytes(
                &format!("{base}_open_easy_afternm pt mlen={mlen}"),
                &p3c,
                &p3r,
            );
            assert_eq!(p3c, m, "{base}_easy_afternm roundtrip mlen={mlen}");

            // detached_afternm + open_detached_afternm
            let mut dac = vec![0u8; mlen];
            let mut dar = vec![0u8; mlen];
            let mut dmac_c = vec![0u8; BOX_MAC];
            let mut dmac_r = vec![0u8; BOX_MAC];
            let (r4c, r4r) = unsafe {
                (
                    da_c(
                        dac.as_mut_ptr(),
                        dmac_c.as_mut_ptr(),
                        m.as_ptr(),
                        mlen as u64,
                        nonce.as_ptr(),
                        ka_c.as_ptr(),
                    ),
                    da_r(
                        dar.as_mut_ptr(),
                        dmac_r.as_mut_ptr(),
                        m.as_ptr(),
                        mlen as u64,
                        nonce.as_ptr(),
                        ka_r.as_ptr(),
                    ),
                )
            };
            eq_i32(&format!("{base}_detached_afternm mlen={mlen}"), r4c, r4r);
            eq_bytes(
                &format!("{base}_detached_afternm ct mlen={mlen}"),
                &dac,
                &dar,
            );
            eq_bytes(
                &format!("{base}_detached_afternm mac mlen={mlen}"),
                &dmac_c,
                &dmac_r,
            );
            let mut p4c = vec![0u8; mlen];
            let mut p4r = vec![0u8; mlen];
            let (d4c, d4r) = unsafe {
                (
                    oda_c(
                        p4c.as_mut_ptr(),
                        dac.as_ptr(),
                        dmac_c.as_ptr(),
                        mlen as u64,
                        nonce.as_ptr(),
                        kb_c.as_ptr(),
                    ),
                    oda_r(
                        p4r.as_mut_ptr(),
                        dar.as_ptr(),
                        dmac_r.as_ptr(),
                        mlen as u64,
                        nonce.as_ptr(),
                        kb_r.as_ptr(),
                    ),
                )
            };
            eq_i32(
                &format!("{base}_open_detached_afternm mlen={mlen}"),
                d4c,
                d4r,
            );
            assert_eq!(d4c, 0, "{base}_open_detached_afternm accept mlen={mlen}");
            eq_bytes(
                &format!("{base}_open_detached_afternm pt mlen={mlen}"),
                &p4c,
                &p4r,
            );
            assert_eq!(p4c, m, "{base}_detached_afternm roundtrip mlen={mlen}");

            // seal / seal_open (row 85) — randomized, so round-trip + cross-lib.
            let slen = mlen + BOX_SEAL_OVERHEAD;
            let mut sc = vec![0u8; slen];
            let mut sr = vec![0u8; slen];
            let (rs_c, rs_r) = unsafe {
                (
                    seal_c(sc.as_mut_ptr(), m.as_ptr(), mlen as u64, pka.as_ptr()),
                    seal_r(sr.as_mut_ptr(), m.as_ptr(), mlen as u64, pka.as_ptr()),
                )
            };
            eq_i32(&format!("{base}_seal rc mlen={mlen}"), rs_c, rs_r);
            assert_eq!(rs_c, 0, "{base}_seal ok mlen={mlen}");
            // cross-library open
            let mut xo = vec![0u8; mlen];
            let ret = unsafe {
                sopen_r(
                    xo.as_mut_ptr(),
                    sc.as_ptr(),
                    slen as u64,
                    pka.as_ptr(),
                    ska.as_ptr(),
                )
            };
            assert_eq!(ret, 0, "{base} C-seal/Rust-open accept mlen={mlen}");
            assert_eq!(xo, m, "{base} C-seal/Rust-open pt mlen={mlen}");
            let mut yo = vec![0u8; mlen];
            let ret2 = unsafe {
                sopen_c(
                    yo.as_mut_ptr(),
                    sr.as_ptr(),
                    slen as u64,
                    pka.as_ptr(),
                    ska.as_ptr(),
                )
            };
            assert_eq!(ret2, 0, "{base} Rust-seal/C-open accept mlen={mlen}");
            assert_eq!(yo, m, "{base} Rust-seal/C-open pt mlen={mlen}");
        }
    }
}

// ===========================================================================
// H. sign
// ===========================================================================

/// Row 86: crypto_sign_seed_keypair and crypto_sign_ed25519_seed_keypair with
/// fixed seeds INCLUDING all-zero and all-0xff.
#[test]
fn r86_sign_seed_keypair() {
    let d = duo();
    let mut rng = Rng::new(0x8600_0001);
    let mut seeds: Vec<Vec<u8>> = vec![vec![0u8; SIGN_SEED], vec![0xffu8; SIGN_SEED]];
    for _ in 0..60 {
        seeds.push(rng.bytes(SIGN_SEED));
    }
    for name in [
        "crypto_sign_seed_keypair",
        "crypto_sign_ed25519_seed_keypair",
    ] {
        let (sk_c, sk_r) = d.pair::<SeedKeypair>(name);
        for (idx, seed) in seeds.iter().enumerate() {
            let mut pkc = vec![0u8; SIGN_PK];
            let mut skc = vec![0u8; SIGN_SK];
            let mut pkr = vec![0u8; SIGN_PK];
            let mut skr = vec![0u8; SIGN_SK];
            let (rc, rr) = unsafe {
                (
                    sk_c(pkc.as_mut_ptr(), skc.as_mut_ptr(), seed.as_ptr()),
                    sk_r(pkr.as_mut_ptr(), skr.as_mut_ptr(), seed.as_ptr()),
                )
            };
            eq_i32(&format!("{name} rc seed#{idx}"), rc, rr);
            eq_bytes(&format!("{name} pk seed#{idx}"), &pkc, &pkr);
            eq_bytes(&format!("{name} sk seed#{idx}"), &skc, &skr);
        }
    }
}

/// Row 87: crypto_sign_ed25519_sk_to_seed, _sk_to_pk (round-trip).
#[test]
fn r87_sign_sk_to_seed_pk() {
    let d = duo();
    let (kp_c, _) = d.pair::<SeedKeypair>("crypto_sign_ed25519_seed_keypair");
    let (s2seed_c, s2seed_r) = d.pair::<Convert2>("crypto_sign_ed25519_sk_to_seed");
    let (s2pk_c, s2pk_r) = d.pair::<Convert2>("crypto_sign_ed25519_sk_to_pk");
    let mut rng = Rng::new(0x8700_0001);

    let mut seeds: Vec<Vec<u8>> = vec![vec![0u8; SIGN_SEED], vec![0xffu8; SIGN_SEED]];
    for _ in 0..60 {
        seeds.push(rng.bytes(SIGN_SEED));
    }
    for (idx, seed) in seeds.iter().enumerate() {
        let mut pk = vec![0u8; SIGN_PK];
        let mut sk = vec![0u8; SIGN_SK];
        unsafe {
            kp_c(pk.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr());
        }

        let mut seed_c = vec![0u8; SIGN_SEED];
        let mut seed_r = vec![0u8; SIGN_SEED];
        let (rc, rr) = unsafe {
            (
                s2seed_c(seed_c.as_mut_ptr(), sk.as_ptr()),
                s2seed_r(seed_r.as_mut_ptr(), sk.as_ptr()),
            )
        };
        eq_i32(&format!("sk_to_seed rc #{idx}"), rc, rr);
        eq_bytes(&format!("sk_to_seed #{idx}"), &seed_c, &seed_r);
        assert_eq!(&seed_c[..], &seed[..], "sk_to_seed recovers seed #{idx}");

        let mut pk_c = vec![0u8; SIGN_PK];
        let mut pk_r = vec![0u8; SIGN_PK];
        let (rc2, rr2) = unsafe {
            (
                s2pk_c(pk_c.as_mut_ptr(), sk.as_ptr()),
                s2pk_r(pk_r.as_mut_ptr(), sk.as_ptr()),
            )
        };
        eq_i32(&format!("sk_to_pk rc #{idx}"), rc2, rr2);
        eq_bytes(&format!("sk_to_pk #{idx}"), &pk_c, &pk_r);
        assert_eq!(&pk_c[..], &pk[..], "sk_to_pk recovers pk #{idx}");
    }
}

/// Row 88: crypto_sign_ed25519_pk_to_curve25519, _sk_to_curve25519.
#[test]
fn r88_sign_to_curve25519() {
    let d = duo();
    let (kp_c, _) = d.pair::<SeedKeypair>("crypto_sign_ed25519_seed_keypair");
    let (pk2c_c, pk2c_r) = d.pair::<Convert2>("crypto_sign_ed25519_pk_to_curve25519");
    let (sk2c_c, sk2c_r) = d.pair::<Convert2>("crypto_sign_ed25519_sk_to_curve25519");
    let mut rng = Rng::new(0x8800_0001);

    for i in 0..80 {
        let seed = if i == 0 {
            vec![0u8; SIGN_SEED]
        } else if i == 1 {
            vec![0xffu8; SIGN_SEED]
        } else {
            rng.bytes(SIGN_SEED)
        };
        let mut pk = vec![0u8; SIGN_PK];
        let mut sk = vec![0u8; SIGN_SK];
        unsafe {
            kp_c(pk.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr());
        }

        let mut cpk_c = vec![0u8; 32];
        let mut cpk_r = vec![0u8; 32];
        let (rc, rr) = unsafe {
            (
                pk2c_c(cpk_c.as_mut_ptr(), pk.as_ptr()),
                pk2c_r(cpk_r.as_mut_ptr(), pk.as_ptr()),
            )
        };
        eq_i32(&format!("pk_to_curve25519 rc #{i}"), rc, rr);
        eq_bytes(&format!("pk_to_curve25519 #{i}"), &cpk_c, &cpk_r);

        let mut csk_c = vec![0u8; 32];
        let mut csk_r = vec![0u8; 32];
        let (rc2, rr2) = unsafe {
            (
                sk2c_c(csk_c.as_mut_ptr(), sk.as_ptr()),
                sk2c_r(csk_r.as_mut_ptr(), sk.as_ptr()),
            )
        };
        eq_i32(&format!("sk_to_curve25519 rc #{i}"), rc2, rr2);
        eq_bytes(&format!("sk_to_curve25519 #{i}"), &csk_c, &csk_r);
    }
}

const SIGN_LENS: &[usize] = &[0, 1, 31, 32, 33, 63, 64, 65, 127, 128, 1000];

/// Row 89: crypto_sign_ed25519_detached / _verify_detached.
#[test]
fn r89_sign_ed25519_detached() {
    let d = duo();
    let (kp_c, _) = d.pair::<SeedKeypair>("crypto_sign_ed25519_seed_keypair");
    let (sign_c, sign_r) = d.pair::<SignDetached>("crypto_sign_ed25519_detached");
    let (ver_c, ver_r) = d.pair::<SignVerifyDetached>("crypto_sign_ed25519_verify_detached");
    let mut rng = Rng::new(0x8900_0001);

    for &mlen in SIGN_LENS {
        for _ in 0..20 {
            let seed = rng.bytes(SIGN_SEED);
            let mut pk = vec![0u8; SIGN_PK];
            let mut sk = vec![0u8; SIGN_SK];
            unsafe {
                kp_c(pk.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr());
            }
            let m = rng.bytes(mlen);

            let mut sig_c = vec![0u8; SIGN_BYTES];
            let mut sig_r = vec![0u8; SIGN_BYTES];
            let mut sl_c: u64 = 0;
            let mut sl_r: u64 = 0;
            let (rc, rr) = unsafe {
                (
                    sign_c(
                        sig_c.as_mut_ptr(),
                        &mut sl_c,
                        m.as_ptr(),
                        mlen as u64,
                        sk.as_ptr(),
                    ),
                    sign_r(
                        sig_r.as_mut_ptr(),
                        &mut sl_r,
                        m.as_ptr(),
                        mlen as u64,
                        sk.as_ptr(),
                    ),
                )
            };
            eq_i32(&format!("sign_detached rc mlen={mlen}"), rc, rr);
            assert_eq!(sl_c, sl_r, "sign_detached siglen mlen={mlen}");
            assert_eq!(
                sl_c, SIGN_BYTES as u64,
                "sign_detached siglen==64 mlen={mlen}"
            );
            eq_bytes(&format!("sign_detached sig mlen={mlen}"), &sig_c, &sig_r);

            // Also test with siglen_p == NULL.
            let mut sig_c2 = vec![0u8; SIGN_BYTES];
            let mut sig_r2 = vec![0u8; SIGN_BYTES];
            let (rcn, rrn) = unsafe {
                (
                    sign_c(
                        sig_c2.as_mut_ptr(),
                        std::ptr::null_mut(),
                        m.as_ptr(),
                        mlen as u64,
                        sk.as_ptr(),
                    ),
                    sign_r(
                        sig_r2.as_mut_ptr(),
                        std::ptr::null_mut(),
                        m.as_ptr(),
                        mlen as u64,
                        sk.as_ptr(),
                    ),
                )
            };
            eq_i32(&format!("sign_detached null-len rc mlen={mlen}"), rcn, rrn);
            eq_bytes(
                &format!("sign_detached null-len sig mlen={mlen}"),
                &sig_c2,
                &sig_r2,
            );

            // verify correct.
            let (vc, vr) = unsafe {
                (
                    ver_c(sig_c.as_ptr(), m.as_ptr(), mlen as u64, pk.as_ptr()),
                    ver_r(sig_r.as_ptr(), m.as_ptr(), mlen as u64, pk.as_ptr()),
                )
            };
            eq_i32(&format!("verify_detached rc mlen={mlen}"), vc, vr);
            assert_eq!(vc, 0, "verify_detached accept mlen={mlen}");

            // Flip each byte of the signature.
            for i in 0..SIGN_BYTES {
                let mut s = sig_c.clone();
                s[i] ^= 0x01;
                let (tc, tr) = unsafe {
                    (
                        ver_c(s.as_ptr(), m.as_ptr(), mlen as u64, pk.as_ptr()),
                        ver_r(s.as_ptr(), m.as_ptr(), mlen as u64, pk.as_ptr()),
                    )
                };
                eq_i32(&format!("verify_detached tamper {i} mlen={mlen}"), tc, tr);
                assert_ne!(tc, 0, "verify_detached tamper reject {i} mlen={mlen}");
            }
        }
    }
}

/// Row 90: crypto_sign_ed25519 / _open (combined), with mlen_p/smlen_p both
/// NULL and non-NULL.
#[test]
fn r90_sign_ed25519_combined() {
    let d = duo();
    let (kp_c, _) = d.pair::<SeedKeypair>("crypto_sign_ed25519_seed_keypair");
    let (sign_c, sign_r) = d.pair::<SignCombined>("crypto_sign_ed25519");
    let (open_c, open_r) = d.pair::<SignOpen>("crypto_sign_ed25519_open");
    let mut rng = Rng::new(0x9000_0001);

    for &mlen in SIGN_LENS {
        for _ in 0..20 {
            let seed = rng.bytes(SIGN_SEED);
            let mut pk = vec![0u8; SIGN_PK];
            let mut sk = vec![0u8; SIGN_SK];
            unsafe {
                kp_c(pk.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr());
            }
            let m = rng.bytes(mlen);
            let smlen_max = mlen + SIGN_BYTES;

            // sign with non-NULL smlen_p.
            let mut sm_c = vec![0u8; smlen_max];
            let mut sm_r = vec![0u8; smlen_max];
            let mut sl_c: u64 = 0;
            let mut sl_r: u64 = 0;
            let (rc, rr) = unsafe {
                (
                    sign_c(
                        sm_c.as_mut_ptr(),
                        &mut sl_c,
                        m.as_ptr(),
                        mlen as u64,
                        sk.as_ptr(),
                    ),
                    sign_r(
                        sm_r.as_mut_ptr(),
                        &mut sl_r,
                        m.as_ptr(),
                        mlen as u64,
                        sk.as_ptr(),
                    ),
                )
            };
            eq_i32(&format!("sign rc mlen={mlen}"), rc, rr);
            assert_eq!(sl_c, sl_r, "sign smlen mlen={mlen}");
            assert_eq!(sl_c, smlen_max as u64, "sign smlen==mlen+64 mlen={mlen}");
            eq_bytes(&format!("sign sm mlen={mlen}"), &sm_c, &sm_r);

            // sign with smlen_p == NULL.
            let mut sm_c2 = vec![0u8; smlen_max];
            let mut sm_r2 = vec![0u8; smlen_max];
            let (rcn, rrn) = unsafe {
                (
                    sign_c(
                        sm_c2.as_mut_ptr(),
                        std::ptr::null_mut(),
                        m.as_ptr(),
                        mlen as u64,
                        sk.as_ptr(),
                    ),
                    sign_r(
                        sm_r2.as_mut_ptr(),
                        std::ptr::null_mut(),
                        m.as_ptr(),
                        mlen as u64,
                        sk.as_ptr(),
                    ),
                )
            };
            eq_i32(&format!("sign null-len rc mlen={mlen}"), rcn, rrn);
            eq_bytes(&format!("sign null-len sm mlen={mlen}"), &sm_c2, &sm_r2);

            // open with non-NULL mlen_p.
            let mut o_c = vec![0u8; smlen_max];
            let mut o_r = vec![0u8; smlen_max];
            let mut ml_c: u64 = 0;
            let mut ml_r: u64 = 0;
            let (oc, orr) = unsafe {
                (
                    open_c(
                        o_c.as_mut_ptr(),
                        &mut ml_c,
                        sm_c.as_ptr(),
                        sl_c,
                        pk.as_ptr(),
                    ),
                    open_r(
                        o_r.as_mut_ptr(),
                        &mut ml_r,
                        sm_r.as_ptr(),
                        sl_r,
                        pk.as_ptr(),
                    ),
                )
            };
            eq_i32(&format!("sign_open rc mlen={mlen}"), oc, orr);
            assert_eq!(oc, 0, "sign_open accept mlen={mlen}");
            assert_eq!(ml_c, ml_r, "sign_open mlen out mlen={mlen}");
            assert_eq!(ml_c, mlen as u64, "sign_open recovered mlen mlen={mlen}");
            eq_bytes(
                &format!("sign_open pt mlen={mlen}"),
                &o_c[..mlen],
                &o_r[..mlen],
            );
            assert_eq!(&o_c[..mlen], &m[..], "sign_open plaintext mlen={mlen}");

            // open with mlen_p == NULL.
            let mut o_c2 = vec![0u8; smlen_max];
            let mut o_r2 = vec![0u8; smlen_max];
            let (ocn, orrn) = unsafe {
                (
                    open_c(
                        o_c2.as_mut_ptr(),
                        std::ptr::null_mut(),
                        sm_c.as_ptr(),
                        sl_c,
                        pk.as_ptr(),
                    ),
                    open_r(
                        o_r2.as_mut_ptr(),
                        std::ptr::null_mut(),
                        sm_r.as_ptr(),
                        sl_r,
                        pk.as_ptr(),
                    ),
                )
            };
            eq_i32(&format!("sign_open null-len rc mlen={mlen}"), ocn, orrn);
            eq_bytes(
                &format!("sign_open null-len pt mlen={mlen}"),
                &o_c2[..mlen],
                &o_r2[..mlen],
            );

            // Flip each byte of the signature region (first 64 bytes of sm).
            for i in 0..SIGN_BYTES {
                let mut sm = sm_c.clone();
                sm[i] ^= 0x01;
                let mut a = vec![0u8; smlen_max];
                let mut b = vec![0u8; smlen_max];
                let (tc, tr) = unsafe {
                    (
                        open_c(
                            a.as_mut_ptr(),
                            std::ptr::null_mut(),
                            sm.as_ptr(),
                            sl_c,
                            pk.as_ptr(),
                        ),
                        open_r(
                            b.as_mut_ptr(),
                            std::ptr::null_mut(),
                            sm.as_ptr(),
                            sl_c,
                            pk.as_ptr(),
                        ),
                    )
                };
                eq_i32(&format!("sign_open tamper {i} mlen={mlen}"), tc, tr);
                assert_ne!(tc, 0, "sign_open tamper reject {i} mlen={mlen}");
            }
        }
    }
}

/// Rows 91-93: ed25519ph multipart (init/update/final_create/final_verify) with
/// random update splits (1-8) crossing 128-byte sha512 blocks, plus the generic
/// crypto_sign_* multipart form (identical to ed25519ph).
#[test]
fn r91_93_sign_ph_multipart() {
    let d = duo();
    let (kp_c, _) = d.pair::<SeedKeypair>("crypto_sign_ed25519_seed_keypair");

    // state buffer size from the library's own accessor.
    let sb_ed = acc(d, "crypto_sign_ed25519ph_statebytes");
    let sb_gen = acc(d, "crypto_sign_statebytes");
    assert!(sb_ed > 0 && sb_gen > 0);

    for (init_n, upd_n, fc_n, fv_n, sb) in [
        (
            "crypto_sign_ed25519ph_init",
            "crypto_sign_ed25519ph_update",
            "crypto_sign_ed25519ph_final_create",
            "crypto_sign_ed25519ph_final_verify",
            sb_ed,
        ),
        (
            "crypto_sign_init",
            "crypto_sign_update",
            "crypto_sign_final_create",
            "crypto_sign_final_verify",
            sb_gen,
        ),
    ] {
        let (init_c, init_r) = d.pair::<PhInit>(init_n);
        let (upd_c, upd_r) = d.pair::<PhUpdate>(upd_n);
        let (fc_c, fc_r) = d.pair::<PhFinalCreate>(fc_n);
        let (fv_c, fv_r) = d.pair::<PhFinalVerify>(fv_n);
        let mut rng = Rng::new(0x9100_0001 ^ (sb as u64));

        for &mlen in &[0usize, 1, 63, 64, 127, 128, 129, 200, 255, 256, 300, 1000] {
            for _ in 0..15 {
                let seed = rng.bytes(SIGN_SEED);
                let mut pk = vec![0u8; SIGN_PK];
                let mut sk = vec![0u8; SIGN_SK];
                unsafe {
                    kp_c(pk.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr());
                }
                let m = rng.bytes(mlen);

                // Random split into 1-8 chunks.
                let nchunks = 1 + rng.below(8);
                let mut bounds = vec![0usize];
                for _ in 0..nchunks.saturating_sub(1) {
                    bounds.push(rng.below(mlen + 1));
                }
                bounds.push(mlen);
                bounds.sort_unstable();

                let mut sc = vec![0u8; sb + 64];
                let mut sr = vec![0u8; sb + 64];
                unsafe {
                    let ic = init_c(sc.as_mut_ptr());
                    let ir = init_r(sr.as_mut_ptr());
                    eq_i32(&format!("{init_n} rc mlen={mlen}"), ic, ir);
                    for w in bounds.windows(2) {
                        let (a, b) = (w[0], w[1]);
                        let uc = upd_c(sc.as_mut_ptr(), m[a..b].as_ptr(), (b - a) as u64);
                        let ur = upd_r(sr.as_mut_ptr(), m[a..b].as_ptr(), (b - a) as u64);
                        eq_i32(&format!("{upd_n} rc mlen={mlen}"), uc, ur);
                    }
                }

                // final_create — note state is consumed; use separate states for
                // create and verify by re-driving.
                let mut sig_c = vec![0u8; SIGN_BYTES];
                let mut sig_r = vec![0u8; SIGN_BYTES];
                let mut sl_c: u64 = 0;
                let mut sl_r: u64 = 0;
                let (fcc, fcr) = unsafe {
                    (
                        fc_c(sc.as_mut_ptr(), sig_c.as_mut_ptr(), &mut sl_c, sk.as_ptr()),
                        fc_r(sr.as_mut_ptr(), sig_r.as_mut_ptr(), &mut sl_r, sk.as_ptr()),
                    )
                };
                eq_i32(&format!("{fc_n} rc mlen={mlen}"), fcc, fcr);
                assert_eq!(sl_c, sl_r, "{fc_n} siglen mlen={mlen}");
                assert_eq!(sl_c, SIGN_BYTES as u64, "{fc_n} siglen==64 mlen={mlen}");
                eq_bytes(&format!("{fc_n} sig mlen={mlen}"), &sig_c, &sig_r);

                // Re-drive fresh states for verification.
                let mut vc = vec![0u8; sb + 64];
                let mut vr = vec![0u8; sb + 64];
                unsafe {
                    init_c(vc.as_mut_ptr());
                    init_r(vr.as_mut_ptr());
                    for w in bounds.windows(2) {
                        let (a, b) = (w[0], w[1]);
                        upd_c(vc.as_mut_ptr(), m[a..b].as_ptr(), (b - a) as u64);
                        upd_r(vr.as_mut_ptr(), m[a..b].as_ptr(), (b - a) as u64);
                    }
                }
                let (vvc, vvr) = unsafe {
                    (
                        fv_c(vc.as_mut_ptr(), sig_c.as_ptr(), pk.as_ptr()),
                        fv_r(vr.as_mut_ptr(), sig_r.as_ptr(), pk.as_ptr()),
                    )
                };
                eq_i32(&format!("{fv_n} rc mlen={mlen}"), vvc, vvr);
                assert_eq!(vvc, 0, "{fv_n} accept mlen={mlen}");

                // Flip each byte of the signature; re-drive fresh states each time.
                for i in 0..SIGN_BYTES {
                    let mut s = sig_c.clone();
                    s[i] ^= 0x01;
                    let mut tc_state = vec![0u8; sb + 64];
                    let mut tr_state = vec![0u8; sb + 64];
                    let (tvc, tvr) = unsafe {
                        init_c(tc_state.as_mut_ptr());
                        init_r(tr_state.as_mut_ptr());
                        for w in bounds.windows(2) {
                            let (a, b) = (w[0], w[1]);
                            upd_c(tc_state.as_mut_ptr(), m[a..b].as_ptr(), (b - a) as u64);
                            upd_r(tr_state.as_mut_ptr(), m[a..b].as_ptr(), (b - a) as u64);
                        }
                        (
                            fv_c(tc_state.as_mut_ptr(), s.as_ptr(), pk.as_ptr()),
                            fv_r(tr_state.as_mut_ptr(), s.as_ptr(), pk.as_ptr()),
                        )
                    };
                    eq_i32(&format!("{fv_n} tamper {i} mlen={mlen}"), tvc, tvr);
                    assert_ne!(tvc, 0, "{fv_n} tamper reject {i} mlen={mlen}");
                }
            }
        }
    }
}
