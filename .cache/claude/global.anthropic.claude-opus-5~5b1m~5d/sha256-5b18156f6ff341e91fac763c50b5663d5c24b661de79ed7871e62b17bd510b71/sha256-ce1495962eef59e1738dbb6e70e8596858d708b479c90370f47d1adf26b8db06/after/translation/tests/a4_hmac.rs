//! Area 4a — `crypto_auth` (generic) and the three HMAC primitives:
//! `crypto_auth_hmacsha256`, `crypto_auth_hmacsha512`,
//! `crypto_auth_hmacsha512256`.
//!
//! Covers `configs_4.md` rows 4.1–4.141 and `errors_4.md` rows 4.1–4.10,
//! 4.13–4.15, 4.18–4.21, 4.23, 4.24.
mod common;
use common::*;
use std::ffi::{c_char, c_int};

type Auth = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8) -> c_int;
type Verify = unsafe extern "C" fn(*const u8, *const u8, u64, *const u8) -> c_int;
type Init = unsafe extern "C" fn(*mut u8, *const u8, usize) -> c_int;
type Update = unsafe extern "C" fn(*mut u8, *const u8, u64) -> c_int;
type Fin = unsafe extern "C" fn(*mut u8, *mut u8) -> c_int;
type Keygen = unsafe extern "C" fn(*mut u8);
type SizeFn = unsafe extern "C" fn() -> usize;
type StrFn = unsafe extern "C" fn() -> *const c_char;

/// Message lengths from the config table (SHA-256 and SHA-512 pad/block
/// boundaries plus their neighbours).
const LENS: [usize; 12] = [0, 1, 55, 56, 63, 64, 65, 111, 112, 127, 128, 129];

/// The two deterministic RNG streams installed by `common` are process-global,
/// so the `*_keygen` tests (the only ones here that consume randomness) must
/// not reseed and drain them concurrently — `cargo test` runs them on separate
/// threads.  Every RNG-consuming test takes this lock for its whole body.
static RNG_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

// ------------------------------------------------------------------ primitives

#[derive(Copy, Clone)]
struct Hmac {
    prefix: &'static str,
    tag: usize,
    block: usize,
    keybytes: usize,
}

const SHA256: Hmac = Hmac { prefix: "crypto_auth_hmacsha256", tag: 32, block: 64, keybytes: 32 };
const SHA512: Hmac = Hmac { prefix: "crypto_auth_hmacsha512", tag: 64, block: 128, keybytes: 32 };
const SHA512256: Hmac =
    Hmac { prefix: "crypto_auth_hmacsha512256", tag: 32, block: 128, keybytes: 32 };

fn sym(prefix: &str, suffix: &str) -> String {
    format!("{prefix}{suffix}")
}

fn size_of_both(name: &str) -> usize {
    let (c, r) = both::<SizeFn>(name);
    let (cv, rv) = unsafe { (c(), r()) };
    assert_eq!(cv, rv, "{name}: size mismatch (C {cv}, Rust {rv})");
    cv
}

fn statebytes(h: &Hmac) -> usize {
    size_of_both(&sym(h.prefix, "_statebytes"))
}

/// One-shot `*_auth(out, in, inlen, k)` on both libraries; asserts equal return
/// code, equal tag and no out-of-bounds write.  Returns the tag.
#[track_caller]
fn oneshot(h: &Hmac, msg: &[u8], key: &[u8], label: &str) -> Vec<u8> {
    assert_eq!(key.len(), h.keybytes, "one-shot requires a KEYBYTES key");
    let (c, r) = both::<Auth>(h.prefix);
    let mut co = padded(h.tag);
    let mut ro = padded(h.tag);
    let (rc, rr) = unsafe {
        (
            c(co.as_mut_ptr(), msg.as_ptr(), msg.len() as u64, key.as_ptr()),
            r(ro.as_mut_ptr(), msg.as_ptr(), msg.len() as u64, key.as_ptr()),
        )
    };
    eqi(&format!("{label}: {} rc", h.prefix), rc, rr);
    assert_eq!(rc, 0, "{label}: {} must return 0 (errors_4 4.23)", h.prefix);
    eqb(&format!("{label}: {} tag", h.prefix), &co[..h.tag], &ro[..h.tag]);
    check_pad(&format!("{label}: {} out", h.prefix), &co, h.tag);
    check_pad(&format!("{label}: {} out", h.prefix), &ro, h.tag);
    co.truncate(h.tag);
    co
}

/// `*_verify` on both libraries; asserts equal return code and returns it.
#[track_caller]
fn verify(h: &Hmac, tag: &[u8], msg: &[u8], key: &[u8], label: &str) -> c_int {
    let (c, r) = both::<Verify>(&sym(h.prefix, "_verify"));
    let (rc, rr) = unsafe {
        (
            c(tag.as_ptr(), msg.as_ptr(), msg.len() as u64, key.as_ptr()),
            r(tag.as_ptr(), msg.as_ptr(), msg.len() as u64, key.as_ptr()),
        )
    };
    eqi(&format!("{label}: {}_verify rc", h.prefix), rc, rr);
    rc
}

/// Streaming `init` / `update`* / `final`, comparing the **full** state buffer
/// after `init` and after every `update` (and after `final`).
///
/// `key` is `None` for the `key == NULL` configurations.
#[track_caller]
fn stream(h: &Hmac, key: Option<&[u8]>, keylen: usize, chunks: &[&[u8]], label: &str) -> Vec<u8> {
    let sb = statebytes(h);
    let (ci, ri) = both::<Init>(&sym(h.prefix, "_init"));
    let (cu, ru) = both::<Update>(&sym(h.prefix, "_update"));
    let (cf, rf) = both::<Fin>(&sym(h.prefix, "_final"));

    // `padded()` zeroes the state region, so the *uninitialised* tail of the
    // internal SHA buffers is identical on both sides and the full-state
    // comparison below is meaningful.
    let mut cs = padded(sb);
    let mut rs = padded(sb);
    let kp = match key {
        Some(k) => k.as_ptr(),
        None => std::ptr::null(),
    };
    let (rc, rr) = unsafe { (ci(cs.as_mut_ptr(), kp, keylen), ri(rs.as_mut_ptr(), kp, keylen)) };
    eqi(&format!("{label}: init rc"), rc, rr);
    assert_eq!(rc, 0, "{label}: init must return 0");
    eqb(&format!("{label}: STATE after init"), &cs[..sb], &rs[..sb]);
    check_pad(&format!("{label}: C state"), &cs, sb);
    check_pad(&format!("{label}: R state"), &rs, sb);

    for (i, ch) in chunks.iter().enumerate() {
        let (rc, rr) = unsafe {
            (
                cu(cs.as_mut_ptr(), ch.as_ptr(), ch.len() as u64),
                ru(rs.as_mut_ptr(), ch.as_ptr(), ch.len() as u64),
            )
        };
        eqi(&format!("{label}: update[{i}] rc"), rc, rr);
        assert_eq!(rc, 0, "{label}: update must return 0");
        eqb(
            &format!("{label}: STATE after update[{i}] (len {})", ch.len()),
            &cs[..sb],
            &rs[..sb],
        );
        check_pad(&format!("{label}: C state"), &cs, sb);
        check_pad(&format!("{label}: R state"), &rs, sb);
    }

    let mut co = padded(h.tag);
    let mut ro = padded(h.tag);
    let (rc, rr) = unsafe { (cf(cs.as_mut_ptr(), co.as_mut_ptr()), rf(rs.as_mut_ptr(), ro.as_mut_ptr())) };
    eqi(&format!("{label}: final rc"), rc, rr);
    assert_eq!(rc, 0, "{label}: final must return 0");
    eqb(&format!("{label}: tag"), &co[..h.tag], &ro[..h.tag]);
    eqb(&format!("{label}: STATE after final"), &cs[..sb], &rs[..sb]);
    check_pad(&format!("{label}: C out"), &co, h.tag);
    check_pad(&format!("{label}: R out"), &ro, h.tag);
    co.truncate(h.tag);
    co
}

fn chunks_of<'a>(msg: &'a [u8], cuts: &[usize]) -> Vec<&'a [u8]> {
    let mut v = Vec::new();
    let mut prev = 0usize;
    for &c in cuts {
        assert!(c >= prev && c <= msg.len());
        v.push(&msg[prev..c]);
        prev = c;
    }
    v.push(&msg[prev..]);
    v
}

fn random_cuts(rng: &mut Rng, len: usize, n: usize) -> Vec<usize> {
    let mut c: Vec<usize> = (0..n).map(|_| rng.below(len + 1)).collect();
    c.sort_unstable();
    c
}

fn exports(which: &str, name: &str) -> bool {
    let l = if which == "c" { c_lib() } else { rust_lib() };
    let mut b: Vec<u8> = name.as_bytes().to_vec();
    b.push(0);
    unsafe { l.get::<*const std::ffi::c_void>(&b).is_ok() }
}

// ======================================================================
// 4.4, 4.5, 4.6, 4.53, 4.100, 4.140 — accessors and API shape
// ======================================================================

#[test]
fn accessors_and_api_shape() {
    // 4.5 / 4.4 — generic accessors.
    assert_eq!(size_of_both("crypto_auth_bytes"), 32);
    assert_eq!(size_of_both("crypto_auth_keybytes"), 32);
    let (cp, rp) = both::<StrFn>("crypto_auth_primitive");
    unsafe {
        let cs = std::ffi::CStr::from_ptr(cp());
        let rs = std::ffi::CStr::from_ptr(rp());
        assert_eq!(cs.to_bytes(), b"hmacsha512256");
        assert_eq!(cs, rs, "crypto_auth_primitive mismatch");
    }

    // 4.53 — hmacsha256: 32 / 32 / two crypto_hash_sha256_state (104 each).
    assert_eq!(size_of_both("crypto_auth_hmacsha256_bytes"), 32);
    assert_eq!(size_of_both("crypto_auth_hmacsha256_keybytes"), 32);
    assert_eq!(size_of_both("crypto_auth_hmacsha256_statebytes"), 208);

    // 4.100 — hmacsha512: 64 / 32 / two crypto_hash_sha512_state (208 each).
    assert_eq!(size_of_both("crypto_auth_hmacsha512_bytes"), 64);
    assert_eq!(size_of_both("crypto_auth_hmacsha512_keybytes"), 32);
    assert_eq!(size_of_both("crypto_auth_hmacsha512_statebytes"), 416);

    // 4.140 — hmacsha512256 statebytes == hmacsha512 statebytes (typedef).
    assert_eq!(size_of_both("crypto_auth_hmacsha512256_bytes"), 32);
    assert_eq!(size_of_both("crypto_auth_hmacsha512256_keybytes"), 32);
    assert_eq!(size_of_both("crypto_auth_hmacsha512256_statebytes"), 416);
    assert_eq!(
        size_of_both("crypto_auth_hmacsha512256_statebytes"),
        size_of_both("crypto_auth_hmacsha512_statebytes")
    );

    // 4.6 — the generic crypto_auth API exposes no streaming surface at all;
    // the Rust port must not invent one.
    for name in [
        "crypto_auth_statebytes",
        "crypto_auth_init",
        "crypto_auth_update",
        "crypto_auth_final",
        "crypto_auth_state",
    ] {
        assert!(!exports("c", name), "C unexpectedly exports {name}");
        assert!(
            !exports("r", name),
            "Rust invented a generic streaming symbol `{name}` that the C API does not have"
        );
    }
}

// ======================================================================
// hmacsha256
// ======================================================================

// 4.7–4.18 (one-shot, every length) + 4.48 (good tag verifies)
#[test]
fn sha256_oneshot_lengths() {
    let mut rng = Rng::new(0x4_0256);
    for &len in LENS.iter() {
        for rep in 0..6 {
            let key = rng.bytes(32);
            let msg = rng.bytes(len);
            let tag = oneshot(&SHA256, &msg, &key, &format!("sha256 len={len} rep={rep}"));
            // 4.48 — the freshly produced tag must verify.
            assert_eq!(
                verify(&SHA256, &tag, &msg, &key, &format!("sha256 good len={len}")),
                0
            );
        }
    }
}

// 4.19–4.30 — streaming with a single update must be bit-identical to one-shot.
#[test]
fn sha256_streaming_single_update() {
    let mut rng = Rng::new(0x4_0257);
    for &len in LENS.iter() {
        for rep in 0..4 {
            let key = rng.bytes(32);
            let msg = rng.bytes(len);
            let a = oneshot(&SHA256, &msg, &key, &format!("sha256 os len={len}"));
            let b = stream(
                &SHA256,
                Some(&key),
                32,
                &[&msg[..]],
                &format!("sha256 stream1 len={len} rep={rep}"),
            );
            eqb(&format!("sha256 stream==oneshot len={len}"), &a, &b);
        }
    }
}

// 4.31–4.37 — the documented multi-update splits, plus many randomized ones.
#[test]
fn sha256_multi_update_splits() {
    let mut rng = Rng::new(0x4_0258);
    let key = rng.bytes(32);

    // fixed splits from the table
    let fixed: [(usize, &[usize]); 8] = [
        (64, &[0]),        // 4.31 (0, 64)
        (64, &[1]),        // 4.32 (1, 63)
        (64, &[63]),       // 4.33 (63, 1)
        (65, &[64]),       // 4.34 (64, 1)
        (64, &[32]),       // 4.35 (32, 32)
        (112, &[40, 80]),  // 4.37 (40, 40, 32)
        (112, &[56]),      // 4.37 (56, 56)
        (129, &[0, 0, 0]), // repeated zero-length no-ops
    ];
    for (len, cuts) in fixed.iter() {
        let msg = rng.bytes(*len);
        let a = oneshot(&SHA256, &msg, &key, "sha256 split ref");
        let b = stream(
            &SHA256,
            Some(&key),
            32,
            &chunks_of(&msg, cuts),
            &format!("sha256 split len={len} cuts={cuts:?}"),
        );
        eqb(&format!("sha256 split len={len} cuts={cuts:?}"), &a, &b);
    }

    // 4.36 — 129 one-byte updates
    {
        let msg = rng.bytes(129);
        let single: Vec<&[u8]> = msg.iter().map(std::slice::from_ref).collect();
        let a = oneshot(&SHA256, &msg, &key, "sha256 129x1 ref");
        let b = stream(&SHA256, Some(&key), 32, &single, "sha256 129x1");
        eqb("sha256 129 one-byte updates", &a, &b);
    }

    // randomized splits straddling every boundary
    for len in 0..=200usize {
        for n in [1usize, 2, 3, 5] {
            let msg = rng.bytes(len);
            let cuts = random_cuts(&mut rng, len, n);
            let a = oneshot(&SHA256, &msg, &key, "sha256 rnd ref");
            let b = stream(
                &SHA256,
                Some(&key),
                32,
                &chunks_of(&msg, &cuts),
                &format!("sha256 rnd len={len} cuts={cuts:?}"),
            );
            eqb(&format!("sha256 rnd len={len} cuts={cuts:?}"), &a, &b);
        }
    }
}

// 4.38, 4.39 (== errors_4 4.18), 4.40–4.47 (== errors_4 4.20)
#[test]
fn sha256_key_lengths() {
    let mut rng = Rng::new(0x4_0259);
    let msg = rng.bytes(77);

    // 4.38 / 4.39 — keylen == 0 with non-NULL and with NULL key agree, and both
    // return 0 (errors_4 4.18: the `key == NULL` arm does *not* misuse when
    // keylen == 0).
    let dummy = rng.bytes(16);
    let a = stream(&SHA256, Some(&dummy), 0, &[&msg[..]], "sha256 keylen=0 non-null");
    let b = stream(&SHA256, None, 0, &[&msg[..]], "sha256 keylen=0 NULL");
    eqb("sha256 keylen=0: NULL key == non-NULL key", &a, &b);

    for &kl in &[0usize, 1, 2, 15, 31, 32, 33, 63, 64, 65, 96, 127, 128, 129, 200, 256, 1000] {
        for rep in 0..3 {
            let key = rng.bytes(kl);
            let t = stream(
                &SHA256,
                Some(&key),
                kl,
                &[&msg[..]],
                &format!("sha256 keylen={kl} rep={rep}"),
            );
            // multi-chunk must agree too
            let cuts = random_cuts(&mut rng, msg.len(), 3);
            let t2 = stream(
                &SHA256,
                Some(&key),
                kl,
                &chunks_of(&msg, &cuts),
                &format!("sha256 keylen={kl} split"),
            );
            eqb("sha256 keylen split", &t, &t2);

            // 4.42 — keylen == KEYBYTES is exactly what the one-shot uses.
            if kl == 32 {
                let os = oneshot(&SHA256, &msg, &key, "sha256 keylen32 oneshot");
                eqb("sha256 keylen=32 == one-shot", &t, &os);
            }
            // 4.45–4.47 / errors_4 4.20 — keylen > 64 hashes the key to
            // SHA-256(key) and forces keylen = 32.  keylen == 64 must NOT hash.
            if kl > 64 {
                let (ch, rh) = both::<
                    unsafe extern "C" fn(*mut u8, *const u8, u64) -> c_int,
                >("crypto_hash_sha256");
                let mut khc = [0u8; 32];
                let mut khr = [0u8; 32];
                unsafe {
                    assert_eq!(ch(khc.as_mut_ptr(), key.as_ptr(), kl as u64), 0);
                    assert_eq!(rh(khr.as_mut_ptr(), key.as_ptr(), kl as u64), 0);
                }
                eqb("crypto_hash_sha256 of long key", &khc, &khr);
                let t3 = stream(
                    &SHA256,
                    Some(&khc),
                    32,
                    &[&msg[..]],
                    &format!("sha256 hashed-key keylen={kl}"),
                );
                eqb(&format!("sha256 keylen={kl} == SHA256(key)"), &t, &t3);
            }
        }
    }

    // 4.44 boundary proof: keylen == 64 does not hash, i.e. it differs from the
    // tag obtained with SHA-256(key) as a 32-byte key (with overwhelming
    // probability) — the important part is that C and Rust agree, which
    // `stream()` already asserted; here we additionally pin the boundary.
    let k64 = rng.bytes(64);
    let t64 = stream(&SHA256, Some(&k64), 64, &[&msg[..]], "sha256 keylen=64");
    let mut kh = [0u8; 32];
    let (ch, _rh) =
        both::<unsafe extern "C" fn(*mut u8, *const u8, u64) -> c_int>("crypto_hash_sha256");
    unsafe {
        assert_eq!(ch(kh.as_mut_ptr(), k64.as_ptr(), 64), 0);
    }
    let th = stream(&SHA256, Some(&kh), 32, &[&msg[..]], "sha256 keylen=64 hashed");
    assert_ne!(t64, th, "keylen == 64 must NOT be hashed (boundary is `>` not `>=`)");
}

// 4.49, 4.50, 4.51 / errors_4 4.1, 4.2
#[test]
fn sha256_verify_corruption() {
    let mut rng = Rng::new(0x4_025a);
    for &len in LENS.iter() {
        let key = rng.bytes(32);
        let msg = rng.bytes(len);
        let tag = oneshot(&SHA256, &msg, &key, "sha256 verify base");
        assert_eq!(verify(&SHA256, &tag, &msg, &key, "sha256 good"), 0);

        // every byte position, every bit
        for i in 0..32usize {
            for bit in 0..8u32 {
                let mut bad = tag.clone();
                bad[i] ^= 1u8 << bit;
                assert_eq!(
                    verify(&SHA256, &bad, &msg, &key, &format!("sha256 bad[{i}:{bit}]")),
                    -1,
                    "sha256 flipped bit {bit} of byte {i} must reject"
                );
            }
        }
        // 4.51 — all-zero, random, wrong message, wrong key
        assert_eq!(verify(&SHA256, &[0u8; 32], &msg, &key, "sha256 zero tag"), -1);
        for _ in 0..8 {
            let rnd = rng.bytes(32);
            if rnd == tag {
                continue;
            }
            assert_eq!(verify(&SHA256, &rnd, &msg, &key, "sha256 random tag"), -1);
        }
        let mut other = msg.clone();
        other.push(0);
        assert_eq!(verify(&SHA256, &tag, &other, &key, "sha256 wrong msg"), -1);
        let mut k2 = key.clone();
        k2[0] ^= 1;
        assert_eq!(verify(&SHA256, &tag, &msg, &k2, "sha256 wrong key"), -1);
    }
}

// 4.52 — keygen
#[test]
fn sha256_keygen() {
    let _rng_guard = RNG_LOCK.lock().unwrap();
    let (c, r) = both::<Keygen>("crypto_auth_hmacsha256_keygen");
    let mut prev: Option<Vec<u8>> = None;
    for i in 0..8 {
        rng_reseed(0x9000 + i as u64);
        let mut ck = padded(32);
        let mut rk = padded(32);
        unsafe {
            c(ck.as_mut_ptr());
            r(rk.as_mut_ptr());
        }
        eqb("hmacsha256_keygen", &ck[..32], &rk[..32]);
        check_pad("hmacsha256_keygen C", &ck, 32);
        check_pad("hmacsha256_keygen R", &rk, 32);
        if let Some(p) = &prev {
            assert_ne!(p, &ck[..32].to_vec(), "successive keygen outputs must differ");
        }
        prev = Some(ck[..32].to_vec());
        // usable as a key (4.42)
        let msg = vec![7u8; 40];
        oneshot(&SHA256, &msg, &ck[..32], "sha256 keygen key");
    }
}

// ======================================================================
// hmacsha512
// ======================================================================

// 4.54–4.65 + 4.95
#[test]
fn sha512_oneshot_lengths() {
    let mut rng = Rng::new(0x4_0512);
    for &len in LENS.iter() {
        for rep in 0..6 {
            let key = rng.bytes(32);
            let msg = rng.bytes(len);
            let tag = oneshot(&SHA512, &msg, &key, &format!("sha512 len={len} rep={rep}"));
            assert_eq!(verify(&SHA512, &tag, &msg, &key, "sha512 good"), 0);
        }
    }
}

// 4.66–4.77
#[test]
fn sha512_streaming_single_update() {
    let mut rng = Rng::new(0x4_0513);
    for &len in LENS.iter() {
        for rep in 0..4 {
            let key = rng.bytes(32);
            let msg = rng.bytes(len);
            let a = oneshot(&SHA512, &msg, &key, "sha512 os");
            let b = stream(
                &SHA512,
                Some(&key),
                32,
                &[&msg[..]],
                &format!("sha512 stream1 len={len} rep={rep}"),
            );
            eqb(&format!("sha512 stream==oneshot len={len}"), &a, &b);
        }
    }
}

// 4.78–4.84
#[test]
fn sha512_multi_update_splits() {
    let mut rng = Rng::new(0x4_0514);
    let key = rng.bytes(32);
    let fixed: [(usize, &[usize]); 7] = [
        (128, &[0]),      // 4.78
        (128, &[1]),      // 4.79
        (128, &[127]),    // 4.80
        (129, &[128]),    // 4.81
        (128, &[64]),     // 4.82
        (112, &[40, 80]), // 4.84
        (129, &[0, 0]),   // zero-length no-ops
    ];
    for (len, cuts) in fixed.iter() {
        let msg = rng.bytes(*len);
        let a = oneshot(&SHA512, &msg, &key, "sha512 split ref");
        let b = stream(
            &SHA512,
            Some(&key),
            32,
            &chunks_of(&msg, cuts),
            &format!("sha512 split len={len} cuts={cuts:?}"),
        );
        eqb(&format!("sha512 split len={len} cuts={cuts:?}"), &a, &b);
    }
    // 4.83 — 129 one-byte updates
    let msg = rng.bytes(129);
    let single: Vec<&[u8]> = msg.iter().map(std::slice::from_ref).collect();
    let a = oneshot(&SHA512, &msg, &key, "sha512 129x1 ref");
    let b = stream(&SHA512, Some(&key), 32, &single, "sha512 129x1");
    eqb("sha512 129 one-byte updates", &a, &b);

    for len in 0..=200usize {
        for n in [1usize, 2, 4] {
            let msg = rng.bytes(len);
            let cuts = random_cuts(&mut rng, len, n);
            let a = oneshot(&SHA512, &msg, &key, "sha512 rnd ref");
            let b = stream(
                &SHA512,
                Some(&key),
                32,
                &chunks_of(&msg, &cuts),
                &format!("sha512 rnd len={len} cuts={cuts:?}"),
            );
            eqb(&format!("sha512 rnd len={len} cuts={cuts:?}"), &a, &b);
        }
    }
}

// 4.85–4.94 / errors_4 4.19, 4.21
#[test]
fn sha512_key_lengths() {
    let mut rng = Rng::new(0x4_0515);
    let msg = rng.bytes(101);

    let dummy = rng.bytes(16);
    let a = stream(&SHA512, Some(&dummy), 0, &[&msg[..]], "sha512 keylen=0 non-null");
    let b = stream(&SHA512, None, 0, &[&msg[..]], "sha512 keylen=0 NULL");
    eqb("sha512 keylen=0: NULL key == non-NULL key", &a, &b);

    for &kl in &[0usize, 1, 2, 31, 32, 33, 63, 64, 65, 127, 128, 129, 200, 256, 257, 1000] {
        for rep in 0..3 {
            let key = rng.bytes(kl);
            let t = stream(
                &SHA512,
                Some(&key),
                kl,
                &[&msg[..]],
                &format!("sha512 keylen={kl} rep={rep}"),
            );
            let cuts = random_cuts(&mut rng, msg.len(), 3);
            let t2 = stream(
                &SHA512,
                Some(&key),
                kl,
                &chunks_of(&msg, &cuts),
                &format!("sha512 keylen={kl} split"),
            );
            eqb("sha512 keylen split", &t, &t2);
            if kl == 32 {
                let os = oneshot(&SHA512, &msg, &key, "sha512 keylen32 oneshot");
                eqb("sha512 keylen=32 == one-shot", &t, &os);
            }
            if kl > 128 {
                let (ch, rh) = both::<
                    unsafe extern "C" fn(*mut u8, *const u8, u64) -> c_int,
                >("crypto_hash_sha512");
                let mut khc = [0u8; 64];
                let mut khr = [0u8; 64];
                unsafe {
                    assert_eq!(ch(khc.as_mut_ptr(), key.as_ptr(), kl as u64), 0);
                    assert_eq!(rh(khr.as_mut_ptr(), key.as_ptr(), kl as u64), 0);
                }
                eqb("crypto_hash_sha512 of long key", &khc, &khr);
                let t3 = stream(
                    &SHA512,
                    Some(&khc),
                    64,
                    &[&msg[..]],
                    &format!("sha512 hashed-key keylen={kl}"),
                );
                eqb(&format!("sha512 keylen={kl} == SHA512(key)"), &t, &t3);
            }
        }
    }

    // 4.91 boundary: keylen == 128 must not hash.
    let k128 = rng.bytes(128);
    let t128 = stream(&SHA512, Some(&k128), 128, &[&msg[..]], "sha512 keylen=128");
    let mut kh = [0u8; 64];
    let (ch, _r) =
        both::<unsafe extern "C" fn(*mut u8, *const u8, u64) -> c_int>("crypto_hash_sha512");
    unsafe {
        assert_eq!(ch(kh.as_mut_ptr(), k128.as_ptr(), 128), 0);
    }
    let th = stream(&SHA512, Some(&kh), 64, &[&msg[..]], "sha512 keylen=128 hashed");
    assert_ne!(t128, th, "keylen == 128 must NOT be hashed");
}

// 4.96, 4.97, 4.98 / errors_4 4.4, 4.5
#[test]
fn sha512_verify_corruption() {
    let mut rng = Rng::new(0x4_0516);
    for &len in LENS.iter() {
        let key = rng.bytes(32);
        let msg = rng.bytes(len);
        let tag = oneshot(&SHA512, &msg, &key, "sha512 verify base");
        assert_eq!(verify(&SHA512, &tag, &msg, &key, "sha512 good"), 0);
        for i in 0..64usize {
            for bit in 0..8u32 {
                let mut bad = tag.clone();
                bad[i] ^= 1u8 << bit;
                assert_eq!(
                    verify(&SHA512, &bad, &msg, &key, &format!("sha512 bad[{i}:{bit}]")),
                    -1
                );
            }
        }
        assert_eq!(verify(&SHA512, &[0u8; 64], &msg, &key, "sha512 zero tag"), -1);
        for _ in 0..6 {
            let rnd = rng.bytes(64);
            assert_eq!(verify(&SHA512, &rnd, &msg, &key, "sha512 random tag"), -1);
        }
        let mut other = msg.clone();
        other.push(0xff);
        assert_eq!(verify(&SHA512, &tag, &other, &key, "sha512 wrong msg"), -1);
        let mut k2 = key.clone();
        k2[31] ^= 0x80;
        assert_eq!(verify(&SHA512, &tag, &msg, &k2, "sha512 wrong key"), -1);
    }
}

// 4.99
#[test]
fn sha512_keygen() {
    let _rng_guard = RNG_LOCK.lock().unwrap();
    let (c, r) = both::<Keygen>("crypto_auth_hmacsha512_keygen");
    let mut prev: Option<Vec<u8>> = None;
    for i in 0..8 {
        rng_reseed(0xA000 + i as u64);
        let mut ck = padded(32);
        let mut rk = padded(32);
        unsafe {
            c(ck.as_mut_ptr());
            r(rk.as_mut_ptr());
        }
        eqb("hmacsha512_keygen", &ck[..32], &rk[..32]);
        check_pad("hmacsha512_keygen C", &ck, 32);
        check_pad("hmacsha512_keygen R", &rk, 32);
        if let Some(p) = &prev {
            assert_ne!(p, &ck[..32].to_vec());
        }
        prev = Some(ck[..32].to_vec());
    }
}

// ======================================================================
// hmacsha512256
// ======================================================================

// 4.101–4.112 + 4.135
#[test]
fn sha512256_oneshot_lengths() {
    let mut rng = Rng::new(0x4_5225);
    for &len in LENS.iter() {
        for rep in 0..6 {
            let key = rng.bytes(32);
            let msg = rng.bytes(len);
            let tag = oneshot(&SHA512256, &msg, &key, &format!("512256 len={len} rep={rep}"));
            assert_eq!(verify(&SHA512256, &tag, &msg, &key, "512256 good"), 0);
        }
    }
}

// 4.113–4.124
#[test]
fn sha512256_streaming_single_update() {
    let mut rng = Rng::new(0x4_5226);
    for &len in LENS.iter() {
        for rep in 0..4 {
            let key = rng.bytes(32);
            let msg = rng.bytes(len);
            let a = oneshot(&SHA512256, &msg, &key, "512256 os");
            let b = stream(
                &SHA512256,
                Some(&key),
                32,
                &[&msg[..]],
                &format!("512256 stream1 len={len} rep={rep}"),
            );
            eqb(&format!("512256 stream==oneshot len={len}"), &a, &b);
        }
    }
}

// 4.125–4.128
#[test]
fn sha512256_multi_update_splits() {
    let mut rng = Rng::new(0x4_5227);
    let key = rng.bytes(32);
    let fixed: [(usize, &[usize]); 6] = [
        (129, &[0]),   // 4.125 (0, n)
        (129, &[1]),   // 4.125 (1, n-1)
        (128, &[127]), // 4.126 (127, 1)
        (129, &[128]), // 4.126 (128, 1)
        (128, &[64]),  // 4.127 (64, 64)
        (129, &[0, 129, 129]),
    ];
    for (len, cuts) in fixed.iter() {
        let msg = rng.bytes(*len);
        let a = oneshot(&SHA512256, &msg, &key, "512256 split ref");
        let b = stream(
            &SHA512256,
            Some(&key),
            32,
            &chunks_of(&msg, cuts),
            &format!("512256 split len={len} cuts={cuts:?}"),
        );
        eqb(&format!("512256 split len={len} cuts={cuts:?}"), &a, &b);
    }
    // 4.128 — 129 one-byte updates
    let msg = rng.bytes(129);
    let single: Vec<&[u8]> = msg.iter().map(std::slice::from_ref).collect();
    let a = oneshot(&SHA512256, &msg, &key, "512256 129x1 ref");
    let b = stream(&SHA512256, Some(&key), 32, &single, "512256 129x1");
    eqb("512256 129 one-byte updates", &a, &b);

    for len in 0..=160usize {
        for n in [1usize, 2, 4] {
            let msg = rng.bytes(len);
            let cuts = random_cuts(&mut rng, len, n);
            let a = oneshot(&SHA512256, &msg, &key, "512256 rnd ref");
            let b = stream(
                &SHA512256,
                Some(&key),
                32,
                &chunks_of(&msg, &cuts),
                &format!("512256 rnd len={len} cuts={cuts:?}"),
            );
            eqb(&format!("512256 rnd len={len} cuts={cuts:?}"), &a, &b);
        }
    }
}

// 4.129–4.132
#[test]
fn sha512256_key_lengths() {
    let mut rng = Rng::new(0x4_5228);
    let msg = rng.bytes(93);

    // 4.132 — keylen 0 with key == NULL is permitted.
    let dummy = rng.bytes(8);
    let a = stream(&SHA512256, Some(&dummy), 0, &[&msg[..]], "512256 keylen=0 non-null");
    let b = stream(&SHA512256, None, 0, &[&msg[..]], "512256 keylen=0 NULL");
    eqb("512256 keylen=0: NULL == non-NULL", &a, &b);

    // 4.129 {0,1,32,64,127}, 4.130 {128}, 4.131 {129,256,1000}
    for &kl in &[0usize, 1, 32, 64, 127, 128, 129, 256, 1000] {
        let key = rng.bytes(kl);
        let t = stream(&SHA512256, Some(&key), kl, &[&msg[..]], &format!("512256 keylen={kl}"));
        // must be the 32-byte prefix of the hmacsha512 tag with the same key
        let t512 = stream(&SHA512, Some(&key), kl, &[&msg[..]], &format!("sha512 keylen={kl}"));
        eqb(&format!("512256 keylen={kl} truncation"), &t, &t512[..32]);
        if kl > 128 {
            let (ch, _r) =
                both::<unsafe extern "C" fn(*mut u8, *const u8, u64) -> c_int>("crypto_hash_sha512");
            let mut kh = [0u8; 64];
            unsafe {
                assert_eq!(ch(kh.as_mut_ptr(), key.as_ptr(), kl as u64), 0);
            }
            let t3 = stream(&SHA512256, Some(&kh), 64, &[&msg[..]], "512256 hashed key");
            eqb(&format!("512256 keylen={kl} == SHA512(key)"), &t, &t3);
        }
    }
}

// 4.133 — truncation semantics vs. hmacsha512
#[test]
fn sha512256_truncation_vs_sha512() {
    let mut rng = Rng::new(0x4_5229);
    for &len in LENS.iter() {
        for _ in 0..4 {
            let key = rng.bytes(32);
            let msg = rng.bytes(len);
            let t256 = oneshot(&SHA512256, &msg, &key, "512256 trunc");
            let t512 = oneshot(&SHA512, &msg, &key, "sha512 trunc");
            eqb(
                &format!("512256 tag == first 32 bytes of sha512 tag (len={len})"),
                &t256,
                &t512[..32],
            );
            // 4.138 — bytes 32..63 of the untruncated tag must be rejected.
            assert_eq!(
                verify(&SHA512256, &t512[32..64], &msg, &key, "512256 truncation confusion"),
                -1
            );
        }
    }
}

// 4.134 — cross-API state interop (hmacsha512256_state is a typedef of
// hmacsha512_state), verified through the *_statebytes()-sized raw buffer.
#[test]
fn sha512256_state_interop_with_sha512() {
    let mut rng = Rng::new(0x4_522a);
    let sb = size_of_both("crypto_auth_hmacsha512256_statebytes");
    assert_eq!(sb, size_of_both("crypto_auth_hmacsha512_statebytes"));

    let (c256i, r256i) = both::<Init>("crypto_auth_hmacsha512256_init");
    let (c512u, r512u) = both::<Update>("crypto_auth_hmacsha512_update");
    let (c512f, r512f) = both::<Fin>("crypto_auth_hmacsha512_final");
    let (c256f, r256f) = both::<Fin>("crypto_auth_hmacsha512256_final");
    let (c512i, r512i) = both::<Init>("crypto_auth_hmacsha512_init");
    let (c256u, r256u) = both::<Update>("crypto_auth_hmacsha512256_update");

    for &len in LENS.iter() {
        let key = rng.bytes(32);
        let msg = rng.bytes(len);

        // (a) 512256_init + 512_update + 512_final -> 64-byte tag
        let mut cs = padded(sb);
        let mut rs = padded(sb);
        unsafe {
            eqi("interop a init", c256i(cs.as_mut_ptr(), key.as_ptr(), 32), r256i(rs.as_mut_ptr(), key.as_ptr(), 32));
        }
        eqb("interop a STATE after init", &cs[..sb], &rs[..sb]);
        unsafe {
            eqi(
                "interop a update",
                c512u(cs.as_mut_ptr(), msg.as_ptr(), len as u64),
                r512u(rs.as_mut_ptr(), msg.as_ptr(), len as u64),
            );
        }
        eqb("interop a STATE after update", &cs[..sb], &rs[..sb]);
        let mut co = padded(64);
        let mut ro = padded(64);
        unsafe {
            eqi("interop a final", c512f(cs.as_mut_ptr(), co.as_mut_ptr()), r512f(rs.as_mut_ptr(), ro.as_mut_ptr()));
        }
        eqb("interop a tag", &co[..64], &ro[..64]);
        check_pad("interop a C out", &co, 64);
        check_pad("interop a R out", &ro, 64);

        // (b) 512_init + 512256_update + 512256_final -> 32-byte tag
        let mut cs2 = padded(sb);
        let mut rs2 = padded(sb);
        unsafe {
            eqi("interop b init", c512i(cs2.as_mut_ptr(), key.as_ptr(), 32), r512i(rs2.as_mut_ptr(), key.as_ptr(), 32));
        }
        eqb("interop b STATE after init", &cs2[..sb], &rs2[..sb]);
        unsafe {
            eqi(
                "interop b update",
                c256u(cs2.as_mut_ptr(), msg.as_ptr(), len as u64),
                r256u(rs2.as_mut_ptr(), msg.as_ptr(), len as u64),
            );
        }
        eqb("interop b STATE after update", &cs2[..sb], &rs2[..sb]);
        let mut co2 = padded(32);
        let mut ro2 = padded(32);
        unsafe {
            eqi("interop b final", c256f(cs2.as_mut_ptr(), co2.as_mut_ptr()), r256f(rs2.as_mut_ptr(), ro2.as_mut_ptr()));
        }
        eqb("interop b tag", &co2[..32], &ro2[..32]);
        check_pad("interop b C out", &co2, 32);
        check_pad("interop b R out", &ro2, 32);

        // the 64-byte tag's prefix is the 32-byte tag
        eqb("interop: 64-byte prefix == 32-byte tag", &co[..32], &co2[..32]);
        let os = oneshot(&SHA512, &msg, &key, "interop reference");
        eqb("interop: matches one-shot hmacsha512", &co[..64], &os);
    }
}

// 4.136, 4.137, 4.138 / errors_4 4.7, 4.8
#[test]
fn sha512256_verify_corruption() {
    let mut rng = Rng::new(0x4_522b);
    for &len in LENS.iter() {
        let key = rng.bytes(32);
        let msg = rng.bytes(len);
        let tag = oneshot(&SHA512256, &msg, &key, "512256 verify base");
        assert_eq!(verify(&SHA512256, &tag, &msg, &key, "512256 good"), 0);
        for i in 0..32usize {
            for bit in 0..8u32 {
                let mut bad = tag.clone();
                bad[i] ^= 1u8 << bit;
                assert_eq!(
                    verify(&SHA512256, &bad, &msg, &key, &format!("512256 bad[{i}:{bit}]")),
                    -1
                );
            }
        }
        assert_eq!(verify(&SHA512256, &[0u8; 32], &msg, &key, "512256 zero tag"), -1);
        for _ in 0..6 {
            let rnd = rng.bytes(32);
            assert_eq!(verify(&SHA512256, &rnd, &msg, &key, "512256 random tag"), -1);
        }
        let mut other = msg.clone();
        other.push(1);
        assert_eq!(verify(&SHA512256, &tag, &other, &key, "512256 wrong msg"), -1);
        let mut k2 = key.clone();
        k2[7] ^= 0x10;
        assert_eq!(verify(&SHA512256, &tag, &msg, &k2, "512256 wrong key"), -1);
    }
}

// 4.139
#[test]
fn sha512256_keygen() {
    let _rng_guard = RNG_LOCK.lock().unwrap();
    let (c, r) = both::<Keygen>("crypto_auth_hmacsha512256_keygen");
    let mut prev: Option<Vec<u8>> = None;
    for i in 0..8 {
        rng_reseed(0xB000 + i as u64);
        let mut ck = padded(32);
        let mut rk = padded(32);
        unsafe {
            c(ck.as_mut_ptr());
            r(rk.as_mut_ptr());
        }
        eqb("hmacsha512256_keygen", &ck[..32], &rk[..32]);
        check_pad("hmacsha512256_keygen C", &ck, 32);
        check_pad("hmacsha512256_keygen R", &rk, 32);
        if let Some(p) = &prev {
            assert_ne!(p, &ck[..32].to_vec());
        }
        prev = Some(ck[..32].to_vec());
    }
}

// ======================================================================
// generic crypto_auth wrapper — 4.1, 4.2, 4.3, 4.141 / errors_4 4.10
// ======================================================================

#[test]
fn generic_crypto_auth_equals_hmacsha512256() {
    let mut rng = Rng::new(0x4_0001);
    let (cg, rg) = both::<Auth>("crypto_auth");
    let (cgv, rgv) = both::<Verify>("crypto_auth_verify");

    for &len in LENS.iter() {
        for _ in 0..5 {
            let key = rng.bytes(32);
            let msg = rng.bytes(len);
            let mut co = padded(32);
            let mut ro = padded(32);
            let (rc, rr) = unsafe {
                (
                    cg(co.as_mut_ptr(), msg.as_ptr(), len as u64, key.as_ptr()),
                    rg(ro.as_mut_ptr(), msg.as_ptr(), len as u64, key.as_ptr()),
                )
            };
            eqi("crypto_auth rc", rc, rr);
            assert_eq!(rc, 0);
            eqb("crypto_auth tag", &co[..32], &ro[..32]);
            check_pad("crypto_auth C out", &co, 32);
            check_pad("crypto_auth R out", &ro, 32);

            // 4.1 / 4.141 — identical to the delegate
            let del = oneshot(&SHA512256, &msg, &key, "crypto_auth delegate");
            eqb("crypto_auth == hmacsha512256", &co[..32], &del);

            // 4.2 — good tag verifies, one flipped bit rejects, and the result
            // always equals crypto_auth_hmacsha512256_verify.
            let (gc, gr) = unsafe {
                (
                    cgv(co.as_ptr(), msg.as_ptr(), len as u64, key.as_ptr()),
                    rgv(co.as_ptr(), msg.as_ptr(), len as u64, key.as_ptr()),
                )
            };
            eqi("crypto_auth_verify good rc", gc, gr);
            assert_eq!(gc, 0);
            assert_eq!(gc, verify(&SHA512256, &del, &msg, &key, "delegate verify good"));

            for i in 0..32usize {
                for bit in [0u32, 3, 7] {
                    let mut bad = del.clone();
                    bad[i] ^= 1u8 << bit;
                    let (bc, br) = unsafe {
                        (
                            cgv(bad.as_ptr(), msg.as_ptr(), len as u64, key.as_ptr()),
                            rgv(bad.as_ptr(), msg.as_ptr(), len as u64, key.as_ptr()),
                        )
                    };
                    eqi("crypto_auth_verify bad rc", bc, br);
                    assert_eq!(bc, -1);
                    assert_eq!(
                        bc,
                        verify(&SHA512256, &bad, &msg, &key, "delegate verify bad"),
                        "crypto_auth_verify must agree with the delegate"
                    );
                }
            }
        }
    }
}

// 4.3
#[test]
fn generic_crypto_auth_keygen() {
    let _rng_guard = RNG_LOCK.lock().unwrap();
    let (c, r) = both::<Keygen>("crypto_auth_keygen");
    let mut prev: Option<Vec<u8>> = None;
    for i in 0..8 {
        rng_reseed(0xC000 + i as u64);
        let mut ck = padded(32);
        let mut rk = padded(32);
        unsafe {
            c(ck.as_mut_ptr());
            r(rk.as_mut_ptr());
        }
        eqb("crypto_auth_keygen", &ck[..32], &rk[..32]);
        check_pad("crypto_auth_keygen C", &ck, 32);
        check_pad("crypto_auth_keygen R", &rk, 32);
        if let Some(p) = &prev {
            assert_ne!(p, &ck[..32].to_vec(), "successive keygen outputs must differ");
        }
        prev = Some(ck[..32].to_vec());
    }
}

// ======================================================================
// errors_4 4.13, 4.14, 4.15 — sodium_misuse() on key == NULL with keylen > 0
// ======================================================================

#[test]
fn init_null_key_nonzero_keylen_misuses() {
    let sb256 = statebytes(&SHA256);
    let sb512 = statebytes(&SHA512);
    let (ci256, ri256) = both::<Init>("crypto_auth_hmacsha256_init");
    let (ci512, ri512) = both::<Init>("crypto_auth_hmacsha512_init");
    let (ci2565, ri2565) = both::<Init>("crypto_auth_hmacsha512256_init");
    let c256: Init = *ci256;
    let r256: Init = *ri256;
    let c512: Init = *ci512;
    let r512: Init = *ri512;
    let c5122: Init = *ci2565;
    let r5122: Init = *ri2565;

    // 4.13 — hmacsha256: 0 < keylen <= 64
    for kl in [1usize, 2, 32, 63, 64] {
        eq_abort(
            &format!("hmacsha256_init(NULL, {kl})"),
            || {
                let mut s = vec![0u8; sb256];
                unsafe { c256(s.as_mut_ptr(), std::ptr::null(), kl) };
            },
            || {
                let mut s = vec![0u8; sb256];
                unsafe { r256(s.as_mut_ptr(), std::ptr::null(), kl) };
            },
        );
    }
    // 4.14 — hmacsha512: 0 < keylen <= 128
    for kl in [1usize, 2, 64, 127, 128] {
        eq_abort(
            &format!("hmacsha512_init(NULL, {kl})"),
            || {
                let mut s = vec![0u8; sb512];
                unsafe { c512(s.as_mut_ptr(), std::ptr::null(), kl) };
            },
            || {
                let mut s = vec![0u8; sb512];
                unsafe { r512(s.as_mut_ptr(), std::ptr::null(), kl) };
            },
        );
    }
    // 4.15 — hmacsha512256: pure delegate, same condition
    for kl in [1usize, 128] {
        eq_abort(
            &format!("hmacsha512256_init(NULL, {kl})"),
            || {
                let mut s = vec![0u8; sb512];
                unsafe { c5122(s.as_mut_ptr(), std::ptr::null(), kl) };
            },
            || {
                let mut s = vec![0u8; sb512];
                unsafe { r5122(s.as_mut_ptr(), std::ptr::null(), kl) };
            },
        );
    }
}

// ======================================================================
// broad randomized fuzz across all three primitives
// ======================================================================

#[test]
fn hmac_randomized_fuzz() {
    let mut rng = Rng::new(0x4_beef);
    for prim in [&SHA256, &SHA512, &SHA512256] {
        for iter in 0..250 {
            let len = if iter % 7 == 0 { rng.range(0, 1200) } else { rng.range(0, 300) };
            let kl = match iter % 5 {
                0 => 32,
                1 => rng.range(0, prim.block),
                2 => prim.block,
                3 => rng.range(prim.block + 1, prim.block * 3),
                _ => rng.range(0, 40),
            };
            let key = rng.bytes(kl);
            let msg = rng.bytes(len);
            let n = rng.range(1, 6);
            let cuts = random_cuts(&mut rng, len, n);
            let label = format!("{} fuzz iter={iter} len={len} kl={kl}", prim.prefix);
            let a = stream(prim, Some(&key), kl, &chunks_of(&msg, &cuts), &label);
            let b = stream(prim, Some(&key), kl, &[&msg[..]], &format!("{label} single"));
            eqb(&format!("{label}: split == single"), &a, &b);
            if kl == 32 {
                let os = oneshot(prim, &msg, &key, &label);
                eqb(&format!("{label}: stream == one-shot"), &a, &os);
                assert_eq!(verify(prim, &os, &msg, &key, &label), 0);
                let mut bad = os.clone();
                let bi = rng.below(prim.tag);
                bad[bi] ^= 1u8 << rng.below(8);
                assert_eq!(verify(prim, &bad, &msg, &key, &label), -1);
            }
        }
    }
}
