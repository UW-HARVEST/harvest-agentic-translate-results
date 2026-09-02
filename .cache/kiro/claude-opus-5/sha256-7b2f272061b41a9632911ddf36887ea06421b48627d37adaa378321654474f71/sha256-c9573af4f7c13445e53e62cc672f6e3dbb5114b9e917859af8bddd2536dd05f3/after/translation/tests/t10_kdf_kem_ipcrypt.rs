//! Differential tests for `crypto_kdf/`, `crypto_kem/` and `crypto_ipcrypt/`.
//!
//! Every assertion loads BOTH shared objects through the harness and drives
//! the `#[no_mangle]` `extern "C"` exports exactly as an external C consumer
//! would. The C at `c_src/libsodium/` is the ground truth: where the two
//! disagree the C wins and the failure is reported verbatim.
//!
//! Notes distilled from the C source that shape these tests:
//!   * `crypto_kdf_blake2b_derive_from_key` (and the generic `crypto_kdf_*`
//!     facade that forwards to it) range-checks `subkey_len` against
//!     [`BYTES_MIN`, `BYTES_MAX`] and on violation sets `errno = EINVAL` and
//!     returns -1 (kdf_blake2b.c).
//!   * `crypto_kdf_hkdf_*_expand` range-checks `out_len <= BYTES_MAX`, else
//!     `errno = EINVAL`, -1 (kdf_hkdf_sha256.c). The streaming extract API is
//!     a thin wrapper over hmac init/update/final, so any chunking of the ikm
//!     must reproduce the one-shot extract byte-for-byte.
//!   * `crypto_kem_mlkem768_dec` uses *implicit rejection*: it always returns
//!     0 and produces a pseudo-random shared secret derived from `z || ct`
//!     when decapsulation fails, so that secret must still match bit-for-bit.
//!     `..._enc_deterministic` rejects a non-canonical public key (any packed
//!     coefficient >= q = 3329) with -1 (kem_mlkem768_ref.c).
//!   * ipcrypt `_encrypt`/`_decrypt` and friends return void and take
//!     fixed-size buffers (crypto_ipcrypt.h). The `pfx`/`ndx` variants have a
//!     `d == 0` fallback that only fires when the two 16-byte key halves
//!     expand to the same round-key middle (ipcrypt_soft.c); equal halves is
//!     the practical trigger.

mod harness;
use harness::*;

use std::ffi::{c_char, c_int};
use std::ptr;

const SEED: u64 = 0x5EED_0010;

// ---------------------------------------------------------------------------
// Small helpers.
// ---------------------------------------------------------------------------

const EINVAL: i32 = 22; // Linux errno for EINVAL.

fn errno() -> i32 {
    unsafe { *libc::__errno_location() }
}
fn clear_errno() {
    unsafe {
        *libc::__errno_location() = 0;
    }
}

/// A `usize` constant read identically from both libraries.
fn size(name: &str) -> usize {
    let (c, r) = sym::<unsafe extern "C" fn() -> usize>(name);
    let (cv, rv) = unsafe { (c(), r()) };
    assert_eq!(cv, rv, "{name} disagrees C={cv} Rust={rv}");
    cv
}

// ===========================================================================
// (a) crypto_kdf_blake2b_derive_from_key + generic crypto_kdf_derive_from_key
// ===========================================================================

type KdfDerive =
    unsafe extern "C" fn(*mut u8, usize, u64, *const c_char, *const u8) -> c_int;

#[test]
fn kdf_derive_from_key_full_matrix() {
    let bytes_min = size("crypto_kdf_blake2b_bytes_min");
    let bytes_max = size("crypto_kdf_blake2b_bytes_max");
    let ctxbytes = size("crypto_kdf_blake2b_contextbytes");
    let keybytes = size("crypto_kdf_blake2b_keybytes");
    assert_eq!(bytes_min, 16);
    assert_eq!(bytes_max, 64);
    assert_eq!(ctxbytes, 8);
    assert_eq!(keybytes, 32);
    // The generic facade must advertise identical constants.
    assert_eq!(size("crypto_kdf_bytes_min"), bytes_min);
    assert_eq!(size("crypto_kdf_bytes_max"), bytes_max);
    assert_eq!(size("crypto_kdf_contextbytes"), ctxbytes);
    assert_eq!(size("crypto_kdf_keybytes"), keybytes);

    let mut rng = Rng::new(SEED);

    // subkey_len values: the in-range ones plus the two just outside the
    // range, which the C rejects with errno=EINVAL and -1.
    let lens: &[(usize, bool)] = &[
        (bytes_min - 1, false), // 15 -> EINVAL
        (bytes_min, true),      // 16
        (bytes_min + 1, true),  // 17
        (32, true),
        (bytes_max, true),      // 64
        (bytes_max + 1, false), // 65 -> EINVAL
    ];
    let ids: &[u64] = &[0, 1, 2, u64::MAX];

    // Several distinct 8-byte contexts, incl. all-zero and all-0xff.
    let contexts: Vec<[u8; 8]> = vec![
        [0u8; 8],
        [0xffu8; 8],
        *b"Examples",
        *b"__test__",
        [1, 2, 3, 4, 5, 6, 7, 8],
    ];

    for name in ["crypto_kdf_blake2b_derive_from_key", "crypto_kdf_derive_from_key"] {
        let (c, r) = sym::<KdfDerive>(name);
        for &(len, in_range) in lens {
            for &id in ids {
                for ctx in &contexts {
                    let key = rng.bytes(keybytes);
                    let mut oc = out_buf(len.max(1));
                    let mut or = out_buf(len.max(1));

                    clear_errno();
                    let (rc, ce) = unsafe {
                        let rc = c(
                            oc.as_mut_ptr(),
                            len,
                            id,
                            ctx.as_ptr() as *const c_char,
                            key.as_ptr(),
                        );
                        (rc, errno())
                    };
                    clear_errno();
                    let (rr, re) = unsafe {
                        let rr = r(
                            or.as_mut_ptr(),
                            len,
                            id,
                            ctx.as_ptr() as *const c_char,
                            key.as_ptr(),
                        );
                        (rr, errno())
                    };

                    assert_eq!(rc, rr, "{name} rc len={len} id={id} ctx={}", hex(ctx));
                    if !in_range {
                        assert_eq!(rc, -1, "{name} out-of-range should be -1 len={len}");
                        assert_eq!(ce, EINVAL, "{name} C errno len={len}");
                        assert_eq!(re, EINVAL, "{name} Rust errno len={len}");
                    } else {
                        assert_eq!(rc, 0, "{name} in-range should be 0 len={len}");
                    }
                    // Full output + canary must match regardless of outcome.
                    eqb(
                        &format!("{name} out len={len} id={id} ctx={}", hex(ctx)),
                        &oc,
                        &or,
                    );
                }
            }
        }
        // The two facades must produce identical subkeys for the same inputs.
        if name == "crypto_kdf_derive_from_key" {
            let (cb, _) = sym::<KdfDerive>("crypto_kdf_blake2b_derive_from_key");
            for &id in ids {
                let key = rng.bytes(keybytes);
                let ctx = *b"identity";
                let mut a = out_buf(32);
                let mut b = out_buf(32);
                unsafe {
                    c(a.as_mut_ptr(), 32, id, ctx.as_ptr() as *const c_char, key.as_ptr());
                    cb(b.as_mut_ptr(), 32, id, ctx.as_ptr() as *const c_char, key.as_ptr());
                }
                eqb(&format!("generic==blake2b id={id}"), &a, &b);
            }
        }
    }
}

// ===========================================================================
// (b) crypto_kdf_hkdf_sha256_* and crypto_kdf_hkdf_sha512_*
// ===========================================================================

type HkdfExtract =
    unsafe extern "C" fn(*mut u8, *const u8, usize, *const u8, usize) -> c_int;
type HkdfExpand =
    unsafe extern "C" fn(*mut u8, usize, *const c_char, usize, *const u8) -> c_int;
type HkdfExtractInit = unsafe extern "C" fn(*mut u8, *const u8, usize) -> c_int;
type HkdfExtractUpdate = unsafe extern "C" fn(*mut u8, *const u8, usize) -> c_int;
type HkdfExtractFinal = unsafe extern "C" fn(*mut u8, *mut u8) -> c_int;

struct Hkdf {
    pfx: &'static str,
    keybytes: usize,
}

const HKDFS: &[Hkdf] = &[
    Hkdf { pfx: "crypto_kdf_hkdf_sha256", keybytes: 32 },
    Hkdf { pfx: "crypto_kdf_hkdf_sha512", keybytes: 64 },
];

const HKDF_STATE_MAX: usize = 512;

#[test]
fn hkdf_extract_one_shot() {
    let mut rng = Rng::new(SEED ^ 1);
    let lens = [0usize, 1, 32, 200];
    for h in HKDFS {
        let kb = size(&format!("{}_keybytes", h.pfx));
        assert_eq!(kb, h.keybytes, "{} keybytes", h.pfx);
        let (c, r) = sym::<HkdfExtract>(&format!("{}_extract", h.pfx));
        for &salt_len in &lens {
            for &ikm_len in &lens {
                let salt = rng.bytes(salt_len.max(1));
                let ikm = rng.bytes(ikm_len.max(1));
                let sp = if salt_len == 0 { ptr::null() } else { salt.as_ptr() };
                let ip = if ikm_len == 0 { ptr::null() } else { ikm.as_ptr() };
                let mut oc = out_buf(h.keybytes);
                let mut or = out_buf(h.keybytes);
                unsafe {
                    let rc = c(oc.as_mut_ptr(), sp, salt_len, ip, ikm_len);
                    let rr = r(or.as_mut_ptr(), sp, salt_len, ip, ikm_len);
                    assert_eq!(rc, rr, "{}_extract rc salt={salt_len} ikm={ikm_len}", h.pfx);
                }
                eqb(
                    &format!("{}_extract prk salt={salt_len} ikm={ikm_len}", h.pfx),
                    &oc,
                    &or,
                );
            }
        }
    }
}

#[test]
fn hkdf_extract_streaming_equals_one_shot() {
    let mut rng = Rng::new(SEED ^ 2);
    for h in HKDFS {
        let sb = size(&format!("{}_statebytes", h.pfx));
        assert!(sb <= HKDF_STATE_MAX, "{} statebytes={sb}", h.pfx);
        let (cone, _) = sym::<HkdfExtract>(&format!("{}_extract", h.pfx));
        let (cinit, rinit) = sym::<HkdfExtractInit>(&format!("{}_extract_init", h.pfx));
        let (cupd, rupd) = sym::<HkdfExtractUpdate>(&format!("{}_extract_update", h.pfx));
        let (cfin, rfin) = sym::<HkdfExtractFinal>(&format!("{}_extract_final", h.pfx));

        for &salt_len in &[0usize, 1, 32, 200] {
            for &ikm_len in &[0usize, 1, 32, 200] {
                let salt = rng.bytes(salt_len.max(1));
                let ikm = rng.bytes(ikm_len.max(1));
                let sp = if salt_len == 0 { ptr::null() } else { salt.as_ptr() };

                // Reference one-shot extract from C.
                let ip = if ikm_len == 0 { ptr::null() } else { ikm.as_ptr() };
                let mut want = out_buf(h.keybytes);
                unsafe { cone(want.as_mut_ptr(), sp, salt_len, ip, ikm_len) };

                // Several different chunkings of the ikm.
                let chunkings: Vec<Vec<usize>> = {
                    let mut v: Vec<Vec<usize>> = vec![vec![ikm_len]];
                    if ikm_len > 0 {
                        v.push(std::iter::repeat(1).take(ikm_len).collect());
                        if ikm_len >= 3 {
                            v.push(vec![1, ikm_len - 1]);
                            v.push(vec![ikm_len / 2, ikm_len - ikm_len / 2]);
                            v.push(vec![ikm_len - 1, 1]);
                        }
                    } else {
                        v.push(vec![]);
                        v.push(vec![0, 0]);
                    }
                    v
                };

                for chunks in chunkings {
                    if chunks.iter().sum::<usize>() != ikm_len {
                        continue;
                    }
                    let mut stc = vec![0xa5u8; HKDF_STATE_MAX];
                    let mut str_ = vec![0xa5u8; HKDF_STATE_MAX];
                    unsafe {
                        let ic = cinit(stc.as_mut_ptr(), sp, salt_len);
                        let ir = rinit(str_.as_mut_ptr(), sp, salt_len);
                        assert_eq!(ic, ir, "{}_extract_init rc", h.pfx);
                    }
                    eqb(&format!("{}_extract_init state salt={salt_len}", h.pfx), &stc[..sb], &str_[..sb]);

                    let mut off = 0usize;
                    for &n in &chunks {
                        let cp = if n == 0 { ptr::null() } else { ikm[off..].as_ptr() };
                        unsafe {
                            let uc = cupd(stc.as_mut_ptr(), cp, n);
                            let ur = rupd(str_.as_mut_ptr(), cp, n);
                            assert_eq!(uc, ur, "{}_extract_update rc", h.pfx);
                        }
                        eqb(
                            &format!("{}_extract_update state ikm={ikm_len} chunks={chunks:?}", h.pfx),
                            &stc[..sb],
                            &str_[..sb],
                        );
                        off += n;
                    }
                    let mut oc = out_buf(h.keybytes);
                    let mut or = out_buf(h.keybytes);
                    unsafe {
                        let fc = cfin(stc.as_mut_ptr(), oc.as_mut_ptr());
                        let fr = rfin(str_.as_mut_ptr(), or.as_mut_ptr());
                        assert_eq!(fc, fr, "{}_extract_final rc", h.pfx);
                    }
                    eqb(
                        &format!("{}_extract_final prk salt={salt_len} ikm={ikm_len} chunks={chunks:?}", h.pfx),
                        &oc,
                        &or,
                    );
                    // Streaming == one-shot (both C and via Rust, since oc==or).
                    eqb(
                        &format!("{}_extract streaming==one-shot salt={salt_len} ikm={ikm_len} chunks={chunks:?}", h.pfx),
                        &want[..h.keybytes],
                        &oc[..h.keybytes],
                    );
                }
            }
        }
    }
}

#[test]
fn hkdf_expand_all_lengths_and_range_check() {
    let mut rng = Rng::new(SEED ^ 3);
    for h in HKDFS {
        let bytes_max = size(&format!("{}_bytes_max", h.pfx));
        assert_eq!(size(&format!("{}_bytes_min", h.pfx)), 0, "{} bytes_min", h.pfx);
        assert_eq!(bytes_max, 0xff * h.keybytes, "{} bytes_max", h.pfx);
        let (c, r) = sym::<HkdfExpand>(&format!("{}_expand", h.pfx));

        let out_lens: &[(usize, bool)] = &[
            (1, true),
            (32, true),
            (bytes_max, true),
            (bytes_max + 1, false), // errno=EINVAL, -1
        ];
        for &(out_len, ok) in out_lens {
            for &ctx_len in &[0usize, 1, 32, 200] {
                let prk = rng.bytes(h.keybytes);
                let ctx = rng.bytes(ctx_len.max(1));
                let cp = if ctx_len == 0 { ptr::null() } else { ctx.as_ptr() as *const c_char };
                let mut oc = out_buf(out_len);
                let mut or = out_buf(out_len);

                clear_errno();
                let (rc, ce) = unsafe {
                    let rc = c(oc.as_mut_ptr(), out_len, cp, ctx_len, prk.as_ptr());
                    (rc, errno())
                };
                clear_errno();
                let (rr, re) = unsafe {
                    let rr = r(or.as_mut_ptr(), out_len, cp, ctx_len, prk.as_ptr());
                    (rr, errno())
                };
                assert_eq!(rc, rr, "{}_expand rc out={out_len} ctx={ctx_len}", h.pfx);
                if ok {
                    assert_eq!(rc, 0, "{}_expand in-range rc out={out_len}", h.pfx);
                } else {
                    assert_eq!(rc, -1, "{}_expand oversize rc out={out_len}", h.pfx);
                    assert_eq!(ce, EINVAL, "{}_expand C errno out={out_len}", h.pfx);
                    assert_eq!(re, EINVAL, "{}_expand Rust errno out={out_len}", h.pfx);
                }
                eqb(&format!("{}_expand out={out_len} ctx={ctx_len}", h.pfx), &oc, &or);
            }
        }
    }
}

#[test]
fn hkdf_keygen_writes_keybytes() {
    for h in HKDFS {
        let (c, r) = sym::<unsafe extern "C" fn(*mut u8)>(&format!("{}_keygen", h.pfx));
        let mut bc = out_buf(h.keybytes);
        let mut br = out_buf(h.keybytes);
        unsafe {
            c(bc.as_mut_ptr());
            r(br.as_mut_ptr());
        }
        // Non-deterministic: only the canary must be intact and neither may be
        // all zeros.
        eqb(&format!("{}_keygen canary", h.pfx), &bc[h.keybytes..], &br[h.keybytes..]);
        assert_ne!(&bc[..h.keybytes], &vec![0u8; h.keybytes][..], "{}_keygen C zeros", h.pfx);
        assert_ne!(&br[..h.keybytes], &vec![0u8; h.keybytes][..], "{}_keygen Rust zeros", h.pfx);
    }
}

// ===========================================================================
// (c) crypto_kem_mlkem768_*, crypto_kem_xwing_*, generic crypto_kem_*
// ===========================================================================

type KemSeedKp = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> c_int;
type KemKp = unsafe extern "C" fn(*mut u8, *mut u8) -> c_int;
type KemEnc = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> c_int;
type KemEncDet = unsafe extern "C" fn(*mut u8, *mut u8, *const u8, *const u8) -> c_int;
type KemDec = unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> c_int;

struct Kem {
    pfx: &'static str,
    pk: usize,
    sk: usize,
    ct: usize,
    ss: usize,
    seed: usize,
    /// length of the enc_deterministic seed, if that entry point is exported.
    enc_det_seed: Option<usize>,
    /// mlkem's `dec` ALWAYS returns 0 (pure implicit rejection). xwing's `dec`
    /// can also return -1 when the X25519 half of the ciphertext is a
    /// low-order / all-zero point (crypto_scalarmult_curve25519 fails), so its
    /// return code is only pinned by the differential check, not a fixed 0.
    dec_always_zero: bool,
}

fn mlkem() -> Kem {
    Kem {
        pfx: "crypto_kem_mlkem768",
        pk: 1184,
        sk: 2400,
        ct: 1088,
        ss: 32,
        seed: 64,
        enc_det_seed: Some(32),
        dec_always_zero: true,
    }
}
fn xwing() -> Kem {
    Kem {
        pfx: "crypto_kem_xwing",
        pk: 1216,
        sk: 32,
        ct: 1120,
        ss: 32,
        seed: 32,
        enc_det_seed: Some(64),
        dec_always_zero: false, // x25519 half can reject with -1
    }
}
fn generic() -> Kem {
    // The generic facade is xwing.
    Kem {
        pfx: "crypto_kem",
        pk: 1216,
        sk: 32,
        ct: 1120,
        ss: 32,
        seed: 32,
        enc_det_seed: None,
        dec_always_zero: false,
    }
}

impl Kem {
    fn check_sizes(&self) {
        assert_eq!(size(&format!("{}_publickeybytes", self.pfx)), self.pk, "{} pk", self.pfx);
        assert_eq!(size(&format!("{}_secretkeybytes", self.pfx)), self.sk, "{} sk", self.pfx);
        assert_eq!(size(&format!("{}_ciphertextbytes", self.pfx)), self.ct, "{} ct", self.pfx);
        assert_eq!(size(&format!("{}_sharedsecretbytes", self.pfx)), self.ss, "{} ss", self.pfx);
        assert_eq!(size(&format!("{}_seedbytes", self.pfx)), self.seed, "{} seed", self.pfx);
    }
}

#[test]
fn kem_seed_keypair_deterministic() {
    let mut rng = Rng::new(SEED ^ 4);
    for kem in [mlkem(), xwing(), generic()] {
        kem.check_sizes();
        let (c, r) = sym::<KemSeedKp>(&format!("{}_seed_keypair", kem.pfx));
        for _ in 0..8 {
            let seed = rng.bytes(kem.seed);
            let mut pkc = out_buf(kem.pk);
            let mut pkr = out_buf(kem.pk);
            let mut skc = out_buf(kem.sk);
            let mut skr = out_buf(kem.sk);
            unsafe {
                let rc = c(pkc.as_mut_ptr(), skc.as_mut_ptr(), seed.as_ptr());
                let rr = r(pkr.as_mut_ptr(), skr.as_mut_ptr(), seed.as_ptr());
                assert_eq!(rc, rr, "{}_seed_keypair rc", kem.pfx);
                assert_eq!(rc, 0, "{}_seed_keypair rc==0", kem.pfx);
            }
            eqb(&format!("{}_seed_keypair pk", kem.pfx), &pkc, &pkr);
            eqb(&format!("{}_seed_keypair sk", kem.pfx), &skc, &skr);
        }
    }
}

#[test]
fn kem_enc_deterministic_bytes_exact() {
    let mut rng = Rng::new(SEED ^ 5);
    for kem in [mlkem(), xwing()] {
        let enc_seed = match kem.enc_det_seed {
            Some(s) => s,
            None => continue,
        };
        let name = format!("{}_enc_deterministic", kem.pfx);
        if !has(&name) {
            continue;
        }
        let (cskp, _) = sym::<KemSeedKp>(&format!("{}_seed_keypair", kem.pfx));
        let (ce, re) = sym::<KemEncDet>(&name);
        for _ in 0..6 {
            // Deterministic key from a seed so both libs share the same pk.
            let kpseed = rng.bytes(kem.seed);
            let mut pk = vec![0u8; kem.pk];
            let mut sk = vec![0u8; kem.sk];
            unsafe { cskp(pk.as_mut_ptr(), sk.as_mut_ptr(), kpseed.as_ptr()) };

            let eseed = rng.bytes(enc_seed);
            let mut ctc = out_buf(kem.ct);
            let mut ctr = out_buf(kem.ct);
            let mut ssc = out_buf(kem.ss);
            let mut ssr = out_buf(kem.ss);
            unsafe {
                let rc = ce(ctc.as_mut_ptr(), ssc.as_mut_ptr(), pk.as_ptr(), eseed.as_ptr());
                let rr = re(ctr.as_mut_ptr(), ssr.as_mut_ptr(), pk.as_ptr(), eseed.as_ptr());
                assert_eq!(rc, rr, "{name} rc");
                assert_eq!(rc, 0, "{name} rc==0");
            }
            eqb(&format!("{name} ct"), &ctc, &ctr);
            eqb(&format!("{name} ss"), &ssc, &ssr);
        }
    }
}

/// `keypair`/`enc` are non-deterministic; verify the canary and return code,
/// then cross-check that a key pair from one library round-trips enc/dec
/// through the OTHER library (interop), and that dec is deterministic.
#[test]
fn kem_keypair_enc_dec_cross_roundtrip() {
    for kem in [mlkem(), xwing(), generic()] {
        let (ckp, rkp) = sym::<KemKp>(&format!("{}_keypair", kem.pfx));
        let (cenc, renc) = sym::<KemEnc>(&format!("{}_enc", kem.pfx));
        let (cdec, rdec) = sym::<KemDec>(&format!("{}_dec", kem.pfx));

        for _ in 0..4 {
            // keypair: non-deterministic. Only canary + rc are meaningful.
            let mut pkc = out_buf(kem.pk);
            let mut skc = out_buf(kem.sk);
            let mut pkr = out_buf(kem.pk);
            let mut skr = out_buf(kem.sk);
            unsafe {
                let rc = ckp(pkc.as_mut_ptr(), skc.as_mut_ptr());
                let rr = rkp(pkr.as_mut_ptr(), skr.as_mut_ptr());
                assert_eq!(rc, rr, "{}_keypair rc", kem.pfx);
                assert_eq!(rc, 0);
            }
            eqb(&format!("{}_keypair pk canary", kem.pfx), &pkc[kem.pk..], &pkr[kem.pk..]);
            eqb(&format!("{}_keypair sk canary", kem.pfx), &skc[kem.sk..], &skr[kem.sk..]);

            // Use the C-produced key pair for every direction so both
            // libraries operate on identical key material.
            let pk = pkc[..kem.pk].to_vec();
            let sk = skc[..kem.sk].to_vec();

            // enc: non-deterministic. Encapsulate with C, then decapsulate the
            // SAME ct with both libs -> the shared secret is deterministic and
            // must match, and must equal the ss produced by enc.
            let mut ct = out_buf(kem.ct);
            let mut ss_enc = out_buf(kem.ss);
            unsafe {
                let rc = cenc(ct.as_mut_ptr(), ss_enc.as_mut_ptr(), pk.as_ptr());
                assert_eq!(rc, 0, "{}_enc rc", kem.pfx);
            }
            let mut ssc = out_buf(kem.ss);
            let mut ssr = out_buf(kem.ss);
            unsafe {
                let dc = cdec(ssc.as_mut_ptr(), ct[..kem.ct].as_ptr(), sk.as_ptr());
                let dr = rdec(ssr.as_mut_ptr(), ct[..kem.ct].as_ptr(), sk.as_ptr());
                assert_eq!(dc, dr, "{}_dec rc", kem.pfx);
                assert_eq!(dc, 0, "{}_dec rc==0", kem.pfx);
            }
            eqb(&format!("{}_dec ss C vs Rust", kem.pfx), &ssc, &ssr);
            eqb(&format!("{}_enc/dec agree", kem.pfx), &ss_enc[..kem.ss], &ssc[..kem.ss]);

            // Interop: encapsulate with the RUST lib against the same pk, then
            // decapsulate with the C lib. The recovered secret must equal the
            // one the Rust enc reported.
            let mut ct2 = out_buf(kem.ct);
            let mut ss_enc2 = out_buf(kem.ss);
            unsafe {
                let rc = renc(ct2.as_mut_ptr(), ss_enc2.as_mut_ptr(), pk.as_ptr());
                assert_eq!(rc, 0, "{}_enc(Rust) rc", kem.pfx);
            }
            let mut ss_dec2 = out_buf(kem.ss);
            unsafe {
                let dc = cdec(ss_dec2.as_mut_ptr(), ct2[..kem.ct].as_ptr(), sk.as_ptr());
                assert_eq!(dc, 0, "{}_dec(C of Rust ct) rc", kem.pfx);
            }
            eqb(
                &format!("{}_enc(Rust)->dec(C) roundtrip", kem.pfx),
                &ss_enc2[..kem.ss],
                &ss_dec2[..kem.ss],
            );
        }
    }
}

/// Corrupted ciphertexts: single-bit flips, all-zero, all-0xff. Both libs must
/// agree on the return code AND the resulting shared secret. mlkem uses
/// implicit rejection (dec returns 0 with a pseudorandom secret that must
/// still match bit-for-bit); xwing may additionally return -1 when its X25519
/// half is degenerate, but the C and Rust must still return the same code and
/// the same secret bytes.
#[test]
fn kem_dec_corrupted_ciphertext() {
    let mut rng = Rng::new(SEED ^ 7);
    for kem in [mlkem(), xwing()] {
        let (cskp, _) = sym::<KemSeedKp>(&format!("{}_seed_keypair", kem.pfx));
        let (cenc, _) = sym::<KemEnc>(&format!("{}_enc", kem.pfx));
        let (cdec, rdec) = sym::<KemDec>(&format!("{}_dec", kem.pfx));

        // Deterministic key pair so both libs share sk.
        let kpseed = rng.bytes(kem.seed);
        let mut pk = vec![0u8; kem.pk];
        let mut sk = vec![0u8; kem.sk];
        unsafe { cskp(pk.as_mut_ptr(), sk.as_mut_ptr(), kpseed.as_ptr()) };

        let mut ct = vec![0u8; kem.ct];
        let mut ss0 = vec![0u8; kem.ss];
        unsafe { cenc(ct.as_mut_ptr(), ss0.as_mut_ptr(), pk.as_ptr()) };

        // Build the corrupted variants.
        let mut variants: Vec<(String, Vec<u8>)> = Vec::new();
        variants.push(("all_zero".into(), vec![0u8; kem.ct]));
        variants.push(("all_ff".into(), vec![0xffu8; kem.ct]));
        // Single-bit flips at a spread of positions.
        for &pos in &[0usize, 1, 7, kem.ct / 2, kem.ct - 1] {
            for bit in [0u8, 3, 7] {
                let mut c = ct.clone();
                c[pos] ^= 1 << bit;
                variants.push((format!("flip@{pos}.{bit}"), c));
            }
        }

        for (tag, bad) in variants {
            let mut ssc = out_buf(kem.ss);
            let mut ssr = out_buf(kem.ss);
            unsafe {
                let dc = cdec(ssc.as_mut_ptr(), bad.as_ptr(), sk.as_ptr());
                let dr = rdec(ssr.as_mut_ptr(), bad.as_ptr(), sk.as_ptr());
                assert_eq!(dc, dr, "{}_dec corrupted rc [{tag}]", kem.pfx);
                if kem.dec_always_zero {
                    // mlkem's implicit rejection: dec returns 0 with a
                    // pseudorandom secret even for a garbage ciphertext.
                    assert_eq!(dc, 0, "{}_dec implicit-rejection rc==0 [{tag}]", kem.pfx);
                }
            }
            // The (pseudorandom) shared secret must match between libs.
            eqb(&format!("{}_dec corrupted ss [{tag}]", kem.pfx), &ssc, &ssr);
        }
    }
}

/// Non-canonical public keys (packed coefficient >= q = 3329) must be rejected
/// identically. Setting whole PK bytes to 0xff yields 12-bit coefficients of
/// 0xfff = 4095 >= 3329, so `polyvec_is_canonical` fails and enc returns -1.
#[test]
fn kem_enc_non_canonical_public_key() {
    let mut rng = Rng::new(SEED ^ 8);
    for kem in [mlkem(), xwing()] {
        let enc_seed = kem.enc_det_seed.unwrap();
        let det = format!("{}_enc_deterministic", kem.pfx);
        let (cd, rd) = sym::<KemEncDet>(&det);
        let (ce, re) = sym::<KemEnc>(&format!("{}_enc", kem.pfx));

        // A wholly non-canonical pk (all 0xff) and a valid pk with only its
        // mlkem polynomial region set to 0xff.
        let mut variants: Vec<(String, Vec<u8>)> = Vec::new();
        variants.push(("all_ff".into(), vec![0xffu8; kem.pk]));
        {
            // Start from a real pk, corrupt the leading polynomial bytes.
            let (cskp, _) = sym::<KemSeedKp>(&format!("{}_seed_keypair", kem.pfx));
            let kpseed = rng.bytes(kem.seed);
            let mut pk = vec![0u8; kem.pk];
            let mut sk = vec![0u8; kem.sk];
            unsafe { cskp(pk.as_mut_ptr(), sk.as_mut_ptr(), kpseed.as_ptr()) };
            for b in pk.iter_mut().take(1152) {
                *b = 0xff;
            }
            variants.push(("poly_ff".into(), pk));
        }

        for (tag, pk) in variants {
            let eseed = rng.bytes(enc_seed);
            let mut ctc = out_buf(kem.ct);
            let mut ctr = out_buf(kem.ct);
            let mut ssc = out_buf(kem.ss);
            let mut ssr = out_buf(kem.ss);
            unsafe {
                let rc = cd(ctc.as_mut_ptr(), ssc.as_mut_ptr(), pk.as_ptr(), eseed.as_ptr());
                let rr = rd(ctr.as_mut_ptr(), ssr.as_mut_ptr(), pk.as_ptr(), eseed.as_ptr());
                assert_eq!(rc, rr, "{det} non-canonical rc [{tag}]");
                assert_eq!(rc, -1, "{det} non-canonical should reject [{tag}]");
            }
            // The non-deterministic enc must reject the same pk identically too.
            let mut ec = out_buf(kem.ct);
            let mut er = out_buf(kem.ct);
            let mut esc = out_buf(kem.ss);
            let mut esr = out_buf(kem.ss);
            unsafe {
                let rc = ce(ec.as_mut_ptr(), esc.as_mut_ptr(), pk.as_ptr());
                let rr = re(er.as_mut_ptr(), esr.as_mut_ptr(), pk.as_ptr());
                assert_eq!(rc, rr, "{}_enc non-canonical rc [{tag}]", kem.pfx);
                assert_eq!(rc, -1, "{}_enc non-canonical should reject [{tag}]", kem.pfx);
            }
        }
    }
}

// ===========================================================================
// (d) crypto_ipcrypt_*
// ===========================================================================

type Ip2 = unsafe extern "C" fn(*mut u8, *const u8, *const u8); // out, in, k
type Ip3 = unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8); // out, in, t, k

/// The canonical 16-byte inputs the task calls out, plus randoms.
fn ip_special_inputs() -> Vec<(String, [u8; 16])> {
    let mut v = Vec::new();
    v.push(("all_zero".into(), [0u8; 16]));
    v.push(("all_ff".into(), [0xffu8; 16]));
    // IPv4-mapped: bytes 10,11 = 0xff, rest of first 10 zero, last 4 an addr.
    let mut mapped = [0u8; 16];
    mapped[10] = 0xff;
    mapped[11] = 0xff;
    mapped[12] = 192;
    mapped[13] = 0;
    mapped[14] = 2;
    mapped[15] = 1;
    v.push(("ipv4_mapped".into(), mapped));
    // Native IPv6 (2001:db8::1).
    let native: [u8; 16] = [0x20, 0x01, 0x0d, 0xb8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1];
    v.push(("ipv6".into(), native));
    v
}

#[test]
fn ipcrypt_encrypt_decrypt_roundtrip() {
    assert_eq!(size("crypto_ipcrypt_bytes"), 16);
    assert_eq!(size("crypto_ipcrypt_keybytes"), 16);
    let (ce, re) = sym::<Ip2>("crypto_ipcrypt_encrypt");
    let (cd, rd) = sym::<Ip2>("crypto_ipcrypt_decrypt");
    let mut rng = Rng::new(SEED ^ 9);

    let mut inputs = ip_special_inputs();
    for i in 0..24 {
        let b = rng.bytes(16);
        let mut a = [0u8; 16];
        a.copy_from_slice(&b);
        inputs.push((format!("rand{i}"), a));
    }

    for (tag, input) in &inputs {
        for kv in 0..8 {
            let key = if kv == 0 {
                vec![0u8; 16]
            } else if kv == 1 {
                vec![0xffu8; 16]
            } else {
                rng.bytes(16)
            };
            let mut ec = out_buf(16);
            let mut er = out_buf(16);
            unsafe {
                ce(ec.as_mut_ptr(), input.as_ptr(), key.as_ptr());
                re(er.as_mut_ptr(), input.as_ptr(), key.as_ptr());
            }
            eqb(&format!("ipcrypt_encrypt [{tag}] key{kv}"), &ec, &er);

            // decrypt(encrypt(x)) == x
            let mut dc = out_buf(16);
            let mut dr = out_buf(16);
            unsafe {
                cd(dc.as_mut_ptr(), ec[..16].as_ptr(), key.as_ptr());
                rd(dr.as_mut_ptr(), er[..16].as_ptr(), key.as_ptr());
            }
            eqb(&format!("ipcrypt_decrypt [{tag}] key{kv}"), &dc, &dr);
            eqb(&format!("ipcrypt roundtrip [{tag}] key{kv}"), &dc[..16], &input[..]);
        }
    }
}

#[test]
fn ipcrypt_nd_encrypt_decrypt_roundtrip() {
    assert_eq!(size("crypto_ipcrypt_nd_inputbytes"), 16);
    assert_eq!(size("crypto_ipcrypt_nd_outputbytes"), 24);
    assert_eq!(size("crypto_ipcrypt_nd_tweakbytes"), 8);
    assert_eq!(size("crypto_ipcrypt_nd_keybytes"), 16);
    let (ce, re) = sym::<Ip3>("crypto_ipcrypt_nd_encrypt");
    let (cd, rd) = sym::<Ip2>("crypto_ipcrypt_nd_decrypt");
    let mut rng = Rng::new(SEED ^ 10);

    let mut inputs = ip_special_inputs();
    for i in 0..16 {
        let b = rng.bytes(16);
        let mut a = [0u8; 16];
        a.copy_from_slice(&b);
        inputs.push((format!("rand{i}"), a));
    }

    for (tag, input) in &inputs {
        for kv in 0..6 {
            let key = match kv {
                0 => vec![0u8; 16],
                1 => vec![0xffu8; 16],
                _ => rng.bytes(16),
            };
            for tv in 0..3 {
                let tweak = match tv {
                    0 => vec![0u8; 8],
                    1 => vec![0xffu8; 8],
                    _ => rng.bytes(8),
                };
                // out is 24 bytes.
                let mut ec = out_buf(24);
                let mut er = out_buf(24);
                unsafe {
                    ce(ec.as_mut_ptr(), input.as_ptr(), tweak.as_ptr(), key.as_ptr());
                    re(er.as_mut_ptr(), input.as_ptr(), tweak.as_ptr(), key.as_ptr());
                }
                eqb(&format!("nd_encrypt [{tag}] key{kv} tweak{tv}"), &ec, &er);

                // decrypt: in = 24, out = 16.
                let mut dc = out_buf(16);
                let mut dr = out_buf(16);
                unsafe {
                    cd(dc.as_mut_ptr(), ec[..24].as_ptr(), key.as_ptr());
                    rd(dr.as_mut_ptr(), er[..24].as_ptr(), key.as_ptr());
                }
                eqb(&format!("nd_decrypt [{tag}] key{kv} tweak{tv}"), &dc, &dr);
                eqb(&format!("nd roundtrip [{tag}] key{kv} tweak{tv}"), &dc[..16], &input[..]);
            }
        }
    }
}

#[test]
fn ipcrypt_ndx_encrypt_decrypt_roundtrip() {
    assert_eq!(size("crypto_ipcrypt_ndx_inputbytes"), 16);
    assert_eq!(size("crypto_ipcrypt_ndx_outputbytes"), 32);
    assert_eq!(size("crypto_ipcrypt_ndx_tweakbytes"), 16);
    assert_eq!(size("crypto_ipcrypt_ndx_keybytes"), 32);
    let (ce, re) = sym::<Ip3>("crypto_ipcrypt_ndx_encrypt");
    let (cd, rd) = sym::<Ip2>("crypto_ipcrypt_ndx_decrypt");
    let mut rng = Rng::new(SEED ^ 11);

    let mut inputs = ip_special_inputs();
    for i in 0..16 {
        let b = rng.bytes(16);
        let mut a = [0u8; 16];
        a.copy_from_slice(&b);
        inputs.push((format!("rand{i}"), a));
    }

    for (tag, input) in &inputs {
        // Keys: random, and the d==0 fallback trigger (both halves equal).
        let mut keys: Vec<(String, Vec<u8>)> = Vec::new();
        keys.push(("zero".into(), vec![0u8; 32]));
        keys.push(("ff".into(), vec![0xffu8; 32]));
        {
            // Equal halves -> tkeys/rkeys expand identically -> d == 0 branch.
            let half = rng.bytes(16);
            let mut k = half.clone();
            k.extend_from_slice(&half);
            keys.push(("equal_halves".into(), k));
        }
        for i in 0..3 {
            keys.push((format!("rand{i}"), rng.bytes(32)));
        }

        for (ktag, key) in &keys {
            for tv in 0..3 {
                let tweak = match tv {
                    0 => vec![0u8; 16],
                    1 => vec![0xffu8; 16],
                    _ => rng.bytes(16),
                };
                let mut ec = out_buf(32);
                let mut er = out_buf(32);
                unsafe {
                    ce(ec.as_mut_ptr(), input.as_ptr(), tweak.as_ptr(), key.as_ptr());
                    re(er.as_mut_ptr(), input.as_ptr(), tweak.as_ptr(), key.as_ptr());
                }
                eqb(&format!("ndx_encrypt [{tag}] key={ktag} tweak{tv}"), &ec, &er);

                let mut dc = out_buf(16);
                let mut dr = out_buf(16);
                unsafe {
                    cd(dc.as_mut_ptr(), ec[..32].as_ptr(), key.as_ptr());
                    rd(dr.as_mut_ptr(), er[..32].as_ptr(), key.as_ptr());
                }
                eqb(&format!("ndx_decrypt [{tag}] key={ktag} tweak{tv}"), &dc, &dr);
                eqb(&format!("ndx roundtrip [{tag}] key={ktag} tweak{tv}"), &dc[..16], &input[..]);
            }
        }
    }
}

#[test]
fn ipcrypt_pfx_encrypt_decrypt_roundtrip() {
    assert_eq!(size("crypto_ipcrypt_pfx_bytes"), 16);
    assert_eq!(size("crypto_ipcrypt_pfx_keybytes"), 32);
    let (ce, re) = sym::<Ip2>("crypto_ipcrypt_pfx_encrypt");
    let (cd, rd) = sym::<Ip2>("crypto_ipcrypt_pfx_decrypt");
    let mut rng = Rng::new(SEED ^ 12);

    let mut inputs = ip_special_inputs();
    for i in 0..16 {
        let b = rng.bytes(16);
        let mut a = [0u8; 16];
        a.copy_from_slice(&b);
        inputs.push((format!("rand{i}"), a));
    }

    for (tag, input) in &inputs {
        // Include keys whose two halves are equal -> exercises the d==0 branch.
        let mut keys: Vec<(String, Vec<u8>)> = Vec::new();
        keys.push(("zero".into(), vec![0u8; 32]));
        keys.push(("ff".into(), vec![0xffu8; 32]));
        {
            let half = rng.bytes(16);
            let mut k = half.clone();
            k.extend_from_slice(&half);
            keys.push(("equal_halves".into(), k));
        }
        {
            let mut k = vec![0x42u8; 16];
            k.extend_from_slice(&vec![0x42u8; 16]);
            keys.push(("equal_halves2".into(), k));
        }
        for i in 0..3 {
            keys.push((format!("rand{i}"), rng.bytes(32)));
        }

        for (ktag, key) in &keys {
            let mut ec = out_buf(16);
            let mut er = out_buf(16);
            unsafe {
                ce(ec.as_mut_ptr(), input.as_ptr(), key.as_ptr());
                re(er.as_mut_ptr(), input.as_ptr(), key.as_ptr());
            }
            eqb(&format!("pfx_encrypt [{tag}] key={ktag}"), &ec, &er);

            let mut dc = out_buf(16);
            let mut dr = out_buf(16);
            unsafe {
                cd(dc.as_mut_ptr(), ec[..16].as_ptr(), key.as_ptr());
                rd(dr.as_mut_ptr(), er[..16].as_ptr(), key.as_ptr());
            }
            eqb(&format!("pfx_decrypt [{tag}] key={ktag}"), &dc, &dr);
            eqb(&format!("pfx roundtrip [{tag}] key={ktag}"), &dc[..16], &input[..]);
        }
    }
}

#[test]
fn ipcrypt_keygen_writes_keybytes() {
    for (name, kb) in [
        ("crypto_ipcrypt_keygen", 16usize),
        ("crypto_ipcrypt_nd_keygen", 16),
        ("crypto_ipcrypt_ndx_keygen", 32),
        ("crypto_ipcrypt_pfx_keygen", 32),
    ] {
        let (c, r) = sym::<unsafe extern "C" fn(*mut u8)>(name);
        let mut bc = out_buf(kb);
        let mut br = out_buf(kb);
        unsafe {
            c(bc.as_mut_ptr());
            r(br.as_mut_ptr());
        }
        eqb(&format!("{name} canary"), &bc[kb..], &br[kb..]);
        assert_ne!(&bc[..kb], &vec![0u8; kb][..], "{name} C zeros");
        assert_ne!(&br[..kb], &vec![0u8; kb][..], "{name} Rust zeros");
    }
}
