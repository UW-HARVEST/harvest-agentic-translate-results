//! Phase B (gap filling) — the `CONFIGS.md ## G4` rows that
//! `tests/t03_hashes.rs` does **not** already exercise.
//!
//! `t03_hashes.rs` drives every G4 primitive through its one-shot and
//! `_init`/`_update`/`_final` (`_squeeze`) forms with *randomised* chunking, so
//! the rows it covers are not repeated here. What it does **not** cover, and
//! what this file adds, is:
//!
//! * the **named, deterministic split patterns** the table calls out (they are
//!   the exact buffer states the C distinguishes, and a random split only hits
//!   them by luck),
//! * `_update` / `_absorb` calls with length 0 interleaved between real ones,
//!   and genuinely NULL `in` pointers with `inlen == 0`,
//! * a handful of input lengths missing from `t03`'s length lists
//!   (239/240/241 for SHA-512, 271/272/273 for SHA3-256, 300, …),
//! * the whole of `crypto_shorthash` (siphash24 / siphashx24 / the generic
//!   dispatcher / `_keygen`), which `t03` does not touch at all,
//! * the exhaustive `crypto_core_keccak1600` offset × length matrices and the
//!   permutation identities (this group only *observes* keccak1600 — it is
//!   owned elsewhere — but the rows still have to be checked),
//! * the cross-primitive identities the table asks for: NULL vs all-zero
//!   salt/personal, `shake256` with `domain = 0x06` == `sha3-256`,
//!   `kdf_blake2b(ctx = 0…0)` == `generichash_blake2b_init_salt_personal`
//!   with `personal = NULL`, `2 × permute_12 != permute_24`,
//! * XOF state duplication by plain struct copy (there is no `_clone`),
//! * the hkdf salt/ikm/out_len/prk values missing from `t03`'s lists and the
//!   RFC-5869-shaped end-to-end `extract` + `expand` matrices.
//!
//! Rows already fully covered elsewhere and therefore **not** re-tested here:
//!
//! * `t03_hashes.rs`: G4-001 … G4-008, G4-011 (except `inlen = 257`),
//!   G4-017, G4-019, G4-021, G4-022, G4-024, G4-025, G4-026, G4-028, G4-029,
//!   G4-032, G4-036, G4-037, G4-038, G4-040 (random splits), G4-041 (except
//!   239/240/241), G4-042, G4-043 (except 300), G4-045 (random splits),
//!   G4-046, G4-054, G4-057, G4-062, G4-065, G4-068, G4-090, G4-093,
//!   G4-095 (except 100), G4-097, G4-098, G4-099, G4-100, G4-102, G4-120,
//!   G4-121, G4-124, G4-125, G4-128, G4-130, G4-134, G4-137, G4-140,
//!   G4-143, G4-146.
//! * `t14_hash_errors.rs` (Phase C, but it drives these valid-input rows in
//!   full): G4-031, G4-033, G4-034, G4-035 (accessors + `pick_best`),
//!   G4-047, G4-048, G4-056 (sha2 / sha3 accessors and the sha3 enumeration),
//!   G4-060 (`_squeeze(0)` on a fresh state), G4-072 … G4-075 (XOF
//!   accessors), G4-076 (`keccak1600_init` + `extract_bytes(0, 200)`),
//!   G4-088, G4-089 (no `_pad`; `statebytes == 224`), G4-091 (poly1305
//!   `in = NULL, inlen = 0`), G4-101 (poly1305 accessors), G4-107
//!   (`shorthash(in = NULL, 0)`), G4-112 (shorthash accessors),
//!   G4-114 … G4-119 (all of `crypto_verify_16/32/64`, including the aliased
//!   and `_bytes` rows), G4-126, G4-127 (kdf accessors), G4-138, G4-147
//!   (hkdf accessors).
//! * G4-077 is explicitly "N/A": `crypto_core_keccak1600_init` takes no
//!   rate/capacity, so the rates live in the callers and are exercised through
//!   the sha3 / shake entry points (rates 72, 136, 168 all appear below).

mod common;
use common::*;

use std::ffi::c_char;

type HashOneShot = unsafe extern "C" fn(*mut u8, *const u8, u64) -> i32;
type StInit = unsafe extern "C" fn(*mut u8) -> i32;
type StUpdate = unsafe extern "C" fn(*mut u8, *const u8, u64) -> i32;
type StFinal = unsafe extern "C" fn(*mut u8, *mut u8) -> i32;

type GhOneShot = unsafe extern "C" fn(*mut u8, usize, *const u8, u64, *const u8, usize) -> i32;
type GhSaltPers = unsafe extern "C" fn(
    *mut u8,
    usize,
    *const u8,
    u64,
    *const u8,
    usize,
    *const u8,
    *const u8,
) -> i32;
type GhInit = unsafe extern "C" fn(*mut u8, *const u8, usize, usize) -> i32;
type GhInitSaltPers =
    unsafe extern "C" fn(*mut u8, *const u8, usize, usize, *const u8, *const u8) -> i32;
type GhFinal = unsafe extern "C" fn(*mut u8, *mut u8, usize) -> i32;

type Short = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8) -> i32;
type Keygen = unsafe extern "C" fn(*mut u8);

type OtaOneShot = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8) -> i32;
type OtaInit = unsafe extern "C" fn(*mut u8, *const u8) -> i32;

type XofOneShot = unsafe extern "C" fn(*mut u8, usize, *const u8, u64) -> i32;
type XofInitDomain = unsafe extern "C" fn(*mut u8, u8) -> i32;
type XofSqueeze = unsafe extern "C" fn(*mut u8, *mut u8, usize) -> i32;

type KcInit = unsafe extern "C" fn(*mut u8);
type KcXor = unsafe extern "C" fn(*mut u8, *const u8, usize, usize);
type KcExtract = unsafe extern "C" fn(*const u8, *mut u8, usize, usize);
type KcPermute = unsafe extern "C" fn(*mut u8);

type KdfDerive = unsafe extern "C" fn(*mut u8, usize, u64, *const c_char, *const u8) -> i32;
type HkdfExtract = unsafe extern "C" fn(*mut u8, *const u8, usize, *const u8, usize) -> i32;
type HkdfExpand = unsafe extern "C" fn(*mut u8, usize, *const c_char, usize, *const u8) -> i32;
type HkdfExInit = unsafe extern "C" fn(*mut u8, *const u8, usize) -> i32;
type HkdfExUpdate = unsafe extern "C" fn(*mut u8, *const u8, usize) -> i32;
type HkdfExFinal = unsafe extern "C" fn(*mut u8, *mut u8) -> i32;

type SizeFn = unsafe extern "C" fn() -> usize;

fn usz(name: &str) -> usize {
    unsafe { sym::<SizeFn>(c_lib(), name)() }
}

/// Drive a `_init`/`_update`/`_final` SHA-family stream with an explicit list
/// of chunk sizes (some of which may be 0) on both libraries, compare the two
/// digests with each other and with the one-shot.
fn sha_split(prefix: &str, dl: usize, data: &[u8], parts: &[usize], what: &str) {
    let (ci, ri) = pair::<StInit>(&format!("{prefix}_init"));
    let (cu, ru) = pair::<StUpdate>(&format!("{prefix}_update"));
    let (cf, rf) = pair::<StFinal>(&format!("{prefix}_final"));
    let sb = format!("{prefix}_statebytes");
    let total: usize = parts.iter().sum();
    assert_eq!(total, data.len(), "{what}: split does not cover the data");

    let mut out = [canary(dl), canary(dl)];
    for (which, (init, upd, fin)) in [(ci, cu, cf), (ri, ru, rf)].into_iter().enumerate() {
        let mut st = State::for_sym(&sb);
        unsafe {
            assert_eq!(init(st.as_mut_ptr()), 0, "{prefix}_init");
            let mut off = 0usize;
            for &n in parts {
                let p = if n == 0 && off == data.len() {
                    std::ptr::null()
                } else {
                    data[off..].as_ptr()
                };
                assert_eq!(upd(st.as_mut_ptr(), p, n as u64), 0, "{prefix}_update");
                off += n;
            }
            assert_eq!(fin(st.as_mut_ptr(), out[which].as_mut_ptr()), 0, "{prefix}_final");
        }
    }
    let (a, b) = (out[0].clone(), out[1].clone());
    eq_bytes(&format!("{prefix} {what}"), &a, &b);
    let (c1, _) = pair::<HashOneShot>(prefix);
    let mut os = canary(dl);
    unsafe { c1(os.as_mut_ptr(), data.as_ptr(), data.len() as u64) };
    eq_bytes(&format!("{prefix} {what} == one-shot"), &os, &a);
}

// ===========================================================================
// SHA-2 — the named split patterns, the missing lengths, update(0)
// ===========================================================================

/// G4-039 (SHA-256 split patterns), G4-040 (`_update(0)` interleaved),
/// G4-044 (SHA-512 split patterns), G4-045 (`_update(0)` interleaved),
/// G4-041 / G4-042 / G4-043 (the 239 / 240 / 241 and 300-byte gaps).
#[test]
fn sha2_named_splits_and_missing_lengths() {
    setup();
    let mut rng = Rng::new(0xF100);

    // --- SHA-256: the exact patterns from the table, each leaving a different
    // `r = (count >> 3) & 0x3f` at the head of the 2nd update.
    let sha256_splits: &[&[usize]] = &[
        &[55, 1],
        &[56, 1],
        &[63, 1],
        &[64, 1],
        &[1, 63],
        &[32, 32],
        &[65, 64],
        &[54, 1, 1],
        &[63, 64, 1],
        &[1, 1, 62],
    ];
    for parts in sha256_splits {
        let total: usize = parts.iter().sum();
        for kind in 0..3 {
            let data = match kind {
                0 => rng.bytes(total),
                1 => vec![0u8; total],
                _ => vec![0xffu8; total],
            };
            sha_split("crypto_hash_sha256", 32, &data, parts, &format!("split {parts:?}/{kind}"));
        }
    }
    // --- SHA-512 patterns
    let sha512_splits: &[&[usize]] = &[
        &[111, 1],
        &[112, 1],
        &[127, 1],
        &[128, 1],
        &[1, 127],
        &[64, 64],
        &[129, 128],
        &[110, 1, 1],
        &[127, 128, 1],
    ];
    for parts in sha512_splits {
        let total: usize = parts.iter().sum();
        for kind in 0..3 {
            let data = match kind {
                0 => rng.bytes(total),
                1 => vec![0u8; total],
                _ => vec![0xffu8; total],
            };
            sha_split("crypto_hash_sha512", 64, &data, parts, &format!("split {parts:?}/{kind}"));
        }
    }

    // --- `_update(inlen = 0)` interleaved between real updates must be an
    // exact no-op (the C returns before touching `count`).
    for &(prefix, dl) in &[("crypto_hash_sha256", 32usize), ("crypto_hash_sha512", 64)] {
        for &total in &[0usize, 1, 55, 56, 64, 111, 112, 128, 300, 1000] {
            let data = rng.bytes(total);
            let half = total / 2;
            let parts: Vec<usize> = vec![0, half, 0, 0, total - half, 0];
            sha_split(prefix, dl, &data, &parts, &format!("update(0) interleaved total={total}"));
        }
    }

    // --- lengths missing from `t03`'s list
    for &(prefix, dl, lens) in &[
        ("crypto_hash_sha256", 32usize, &[300usize, 320, 448, 449][..]),
        ("crypto_hash_sha512", 64, &[239usize, 240, 241, 300][..]),
    ] {
        let (c1, r1) = pair::<HashOneShot>(prefix);
        for &len in lens {
            for kind in 0..3 {
                let data = match kind {
                    0 => rng.bytes(len),
                    1 => vec![0u8; len],
                    _ => vec![0xffu8; len],
                };
                let mut a = canary(dl);
                let mut b = canary(dl);
                let (ra, rb) = unsafe {
                    (
                        c1(a.as_mut_ptr(), data.as_ptr(), len as u64),
                        r1(b.as_mut_ptr(), data.as_ptr(), len as u64),
                    )
                };
                eq_i32(&format!("{prefix}({len}) rc"), ra, rb);
                eq_bytes(&format!("{prefix}(len={len},kind={kind})"), &a, &b);
                // single update, one-byte-at-a-time, and a 2-way split
                sha_split(prefix, dl, &data, &[len], &format!("one update len={len}"));
                sha_split(prefix, dl, &data, &vec![1usize; len], &format!("1-byte len={len}"));
                sha_split(prefix, dl, &data, &[len - 1, 1], &format!("({}, 1)", len - 1));
            }
        }
    }
    // in == NULL with inlen == 0 through the streaming API
    for &(prefix, dl) in &[("crypto_hash_sha256", 32usize), ("crypto_hash_sha512", 64)] {
        sha_split(prefix, dl, &[], &[0, 0, 0], "NULL/0 updates only");
    }
}

// ===========================================================================
// SHA-3 — named splits, missing lengths, update(0)
// ===========================================================================

/// G4-049 / G4-050 (the 271 / 272 / 273 gap), G4-051 (SHA3-256 split
/// patterns), G4-052 (300 single-byte absorbs), G4-053 (`_update(0)`),
/// G4-055 (SHA3-512 split patterns).
#[test]
fn sha3_named_splits_and_missing_lengths() {
    setup();
    let mut rng = Rng::new(0xF110);

    for &(prefix, dl, rate) in &[
        ("crypto_hash_sha3256", 32usize, 136usize),
        ("crypto_hash_sha3512", 64, 72),
    ] {
        let (c1, r1) = pair::<HashOneShot>(prefix);
        // The lengths the table names, including the 2*rate-1 / 2*rate /
        // 2*rate+1 triple that `t03`'s list misses for rate 136.
        for &len in &[
            0usize,
            1,
            rate - 1,
            rate,
            rate + 1,
            2 * rate - 1,
            2 * rate,
            2 * rate + 1,
            3 * rate,
            300,
            1000,
        ] {
            for kind in 0..3 {
                let data = match kind {
                    0 => rng.bytes(len),
                    1 => vec![0u8; len],
                    _ => vec![0xffu8; len],
                };
                let mut a = canary(dl);
                let mut b = canary(dl);
                let (ra, rb) = unsafe {
                    (
                        c1(a.as_mut_ptr(), data.as_ptr(), len as u64),
                        r1(b.as_mut_ptr(), data.as_ptr(), len as u64),
                    )
                };
                eq_i32(&format!("{prefix}({len}) rc"), ra, rb);
                eq_bytes(&format!("{prefix}(len={len},kind={kind})"), &a, &b);
                sha_split(prefix, dl, &data, &[len], &format!("one update len={len}"));
                if len <= 300 {
                    sha_split(prefix, dl, &data, &vec![1usize; len], &format!("1-byte len={len}"));
                }
            }
        }

        // the named split patterns — `(rate, 1)` is the key one: after update 1
        // `offset == rate`, so update 2 must permute first.
        let splits: Vec<Vec<usize>> = vec![
            vec![rate - 1, 1],
            vec![rate, 1],
            vec![1, rate - 1],
            vec![rate + 1, rate - 1],
            vec![rate / 2, rate / 2],
            vec![rate, rate],
            vec![rate, rate, 1],
            vec![2 * rate, 1],
            vec![1, rate, rate],
        ];
        for parts in &splits {
            let total: usize = parts.iter().sum();
            let data = rng.bytes(total);
            sha_split(prefix, dl, &data, parts, &format!("split {parts:?}"));
        }

        // `_update(0)` interleaved, including right after a rate-boundary
        // update (where the C leaves `offset == rate` and defers the permute).
        for &total in &[0usize, 1, rate, 2 * rate, 300] {
            let data = rng.bytes(total);
            let parts: Vec<usize> = if total >= rate {
                vec![0, rate, 0, 0, total - rate, 0]
            } else {
                vec![0, total, 0]
            };
            sha_split(prefix, dl, &data, &parts, &format!("update(0) total={total}"));
        }
        sha_split(prefix, dl, &[], &[0, 0, 0], "NULL/0 updates only");
    }
}

// ===========================================================================
// BLAKE2b / generichash gaps
// ===========================================================================

fn gh_split(
    prefix: &str,
    outlen: usize,
    key: &[u8],
    data: &[u8],
    parts: &[usize],
    what: &str,
) -> Vec<u8> {
    let (ci, ri) = pair::<GhInit>(&format!("{prefix}_init"));
    let (cu, ru) = pair::<StUpdate>(&format!("{prefix}_update"));
    let (cf, rf) = pair::<GhFinal>(&format!("{prefix}_final"));
    let sb = format!("{prefix}_statebytes");
    let kp = if key.is_empty() {
        std::ptr::null()
    } else {
        key.as_ptr()
    };
    let mut out = [canary(outlen), canary(outlen)];
    for (which, (init, upd, fin)) in [(ci, cu, cf), (ri, ru, rf)].into_iter().enumerate() {
        let mut st = State::for_sym(&sb);
        unsafe {
            assert_eq!(init(st.as_mut_ptr(), kp, key.len(), outlen), 0);
            let mut off = 0usize;
            for &n in parts {
                let p = if n == 0 && off == data.len() {
                    std::ptr::null()
                } else {
                    data[off..].as_ptr()
                };
                assert_eq!(upd(st.as_mut_ptr(), p, n as u64), 0);
                off += n;
            }
            assert_eq!(fin(st.as_mut_ptr(), out[which].as_mut_ptr(), outlen), 0);
        }
    }
    let (a, b) = (out[0].clone(), out[1].clone());
    eq_bytes(&format!("{prefix} {what}"), &a, &b);
    let (c1, _) = pair::<GhOneShot>(prefix);
    let mut os = canary(outlen);
    unsafe {
        c1(os.as_mut_ptr(), outlen, data.as_ptr(), data.len() as u64, kp, key.len());
    }
    eq_bytes(&format!("{prefix} {what} == one-shot"), &os, &a);
    a
}

/// G4-009 (`key != NULL` with `keylen == 0`), G4-010 (`in == NULL`,
/// `inlen == 0`), G4-011 (`inlen = 257`), G4-012 (300 single-byte updates),
/// G4-013 / G4-014 / G4-015 / G4-016 (the named `(127,1) (128,1) (255,1)
/// (256,1)` splits — the exact lazy-buffer boundaries), G4-018 (`_update(0)`
/// interleaved), G4-020 (init/final `outlen` mismatch, re-checked here as a
/// *valid* configuration).
#[test]
fn generichash_named_splits_and_null_pointers() {
    setup();
    let mut rng = Rng::new(0xF120);

    for prefix in ["crypto_generichash", "crypto_generichash_blake2b"] {
        let (c1, r1) = pair::<GhOneShot>(prefix);

        // G4-009: a real, non-empty key buffer with keylen == 0 must be
        // byte-identical to key == NULL (the pointer is never read).
        let key = rng.bytes(64);
        let data = rng.bytes(300);
        for &outlen in &[16usize, 32, 64] {
            let mut a = canary(outlen);
            let mut b = canary(outlen);
            let mut n = canary(outlen);
            let (ra, rb) = unsafe {
                (
                    c1(a.as_mut_ptr(), outlen, data.as_ptr(), 300, key.as_ptr(), 0),
                    r1(b.as_mut_ptr(), outlen, data.as_ptr(), 300, key.as_ptr(), 0),
                )
            };
            eq_i32(&format!("{prefix} key!=NULL keylen=0 rc"), ra, rb);
            assert_eq!(ra, 0);
            eq_bytes(&format!("{prefix} key!=NULL keylen=0"), &a, &b);
            unsafe { c1(n.as_mut_ptr(), outlen, data.as_ptr(), 300, std::ptr::null(), 0) };
            eq_bytes(&format!("{prefix} key!=NULL keylen=0 == unkeyed"), &n, &a);

            // G4-010: in == NULL, inlen == 0.
            let mut a = canary(outlen);
            let mut b = canary(outlen);
            let (ra, rb) = unsafe {
                (
                    c1(a.as_mut_ptr(), outlen, std::ptr::null(), 0, std::ptr::null(), 0),
                    r1(b.as_mut_ptr(), outlen, std::ptr::null(), 0, std::ptr::null(), 0),
                )
            };
            eq_i32(&format!("{prefix} in=NULL inlen=0 rc"), ra, rb);
            assert_eq!(ra, 0);
            eq_bytes(&format!("{prefix} in=NULL inlen=0"), &a, &b);
            // and the same through the streaming trio
            gh_split(prefix, outlen, &[], &[], &[0, 0], "NULL/0 updates only");
        }

        // G4-013 … G4-016: the lazy-2-block-buffer boundaries.
        let named: &[&[usize]] = &[
            &[127, 1],  // buflen lands exactly on one block, no compress
            &[128, 1],  // fill == 128, inlen == 1 <= fill, still no compress
            &[255, 1],  // buflen reaches 256 with ZERO compresses
            &[256, 1],  // the FIRST compress fires (shift-buffer-left path)
            &[257, 1],
            &[128, 128],
            &[255, 2],
            &[1, 255],
            &[1, 256],
            &[128, 127, 1],
            &[256, 128, 1],
        ];
        for parts in named {
            let total: usize = parts.iter().sum();
            for &keylen in &[0usize, 16, 32, 64] {
                let key = rng.bytes(keylen);
                let data = rng.bytes(total);
                gh_split(prefix, 32, &key, &data, parts, &format!("split {parts:?} key={keylen}"));
            }
        }

        // G4-011: inlen == 257 through one single update.
        // G4-012: 300 one-byte updates (buflen sweeps 0..256, one compress at
        // the 257th byte).
        for &total in &[257usize, 300] {
            for &keylen in &[0usize, 32] {
                let key = rng.bytes(keylen);
                let data = rng.bytes(total);
                gh_split(prefix, 32, &key, &data, &[total], &format!("one update {total}"));
                gh_split(prefix, 32, &key, &data, &vec![1usize; total], &format!("1-byte {total}"));
            }
        }

        // G4-018: update(0), update(64), update(0), update(64), update(0)
        for &keylen in &[0usize, 32] {
            let key = rng.bytes(keylen);
            let data = rng.bytes(128);
            gh_split(prefix, 32, &key, &data, &[0, 64, 0, 64, 0], "0/64/0/64/0");
            let data = rng.bytes(300);
            gh_split(prefix, 32, &key, &data, &[0, 0, 300, 0], "0/0/300/0");
        }

        // G4-020 (valid-input view): `_final`'s outlen is independent of
        // `_init`'s; the first `min(a,b)` bytes always agree.
        let (ci, ri) = pair::<GhInit>(&format!("{prefix}_init"));
        let (cu, ru) = pair::<StUpdate>(&format!("{prefix}_update"));
        let (cf, rf) = pair::<GhFinal>(&format!("{prefix}_final"));
        let sb = format!("{prefix}_statebytes");
        let data = rng.bytes(400);
        for &init_out in &[16usize, 32, 64] {
            let mut dumps: Vec<(usize, Vec<u8>)> = Vec::new();
            for &fin_out in &[1usize, 16, 17, 31, 32, 63, 64] {
                let mut out = [canary(fin_out), canary(fin_out)];
                for (which, (init, upd, fin)) in
                    [(ci, cu, cf), (ri, ru, rf)].into_iter().enumerate()
                {
                    let mut st = State::for_sym(&sb);
                    unsafe {
                        assert_eq!(init(st.as_mut_ptr(), std::ptr::null(), 0, init_out), 0);
                        assert_eq!(upd(st.as_mut_ptr(), data.as_ptr(), 400), 0);
                        assert_eq!(
                            fin(st.as_mut_ptr(), out[which].as_mut_ptr(), fin_out),
                            0,
                            "{prefix}_final(init={init_out}, fin={fin_out})"
                        );
                    }
                }
                let (a, b) = (out[0].clone(), out[1].clone());
                eq_bytes(&format!("{prefix} init={init_out} fin={fin_out}"), &a, &b);
                dumps.push((fin_out, a));
            }
            // all of them are prefixes of the widest dump
            let widest = dumps.iter().max_by_key(|(n, _)| *n).unwrap().1.clone();
            for (n, d) in &dumps {
                assert_eq!(&widest[..*n], &d[..], "{prefix} init={init_out} fin={n} not a prefix");
            }
        }
    }
}

/// G4-023 (`salt = NULL, personal = NULL` MUST equal plain
/// `crypto_generichash_blake2b`), G4-027 (16 zero bytes must be bit-identical
/// to `NULL`), G4-030 (`_init_salt_personal` keyed with 1-byte-at-a-time
/// updates over 300 bytes — the shape `crypto_kdf_blake2b` builds).
#[test]
fn generichash_salt_personal_identities() {
    setup();
    let mut rng = Rng::new(0xF121);
    let (csp, rsp) = pair::<GhSaltPers>("crypto_generichash_blake2b_salt_personal");
    let (cp, _) = pair::<GhOneShot>("crypto_generichash_blake2b");
    let zero16 = [0u8; 16];

    for &outlen in &[1usize, 16, 32, 64] {
        for &keylen in &[0usize, 1, 16, 32, 64] {
            for &inlen in &[0usize, 1, 127, 128, 129, 255, 256, 257, 1000] {
                let key = rng.bytes(keylen);
                let data = rng.bytes(inlen);
                let kp = if keylen == 0 {
                    std::ptr::null()
                } else {
                    key.as_ptr()
                };
                let mut variants: Vec<(String, Vec<u8>, Vec<u8>)> = Vec::new();
                for &(sp, pp, what) in &[
                    (std::ptr::null(), std::ptr::null(), "NULL/NULL"),
                    (zero16.as_ptr(), std::ptr::null(), "ZERO/NULL"),
                    (std::ptr::null(), zero16.as_ptr(), "NULL/ZERO"),
                    (zero16.as_ptr(), zero16.as_ptr(), "ZERO/ZERO"),
                ] {
                    let mut a = canary(outlen);
                    let mut b = canary(outlen);
                    let (ra, rb) = unsafe {
                        (
                            csp(a.as_mut_ptr(), outlen, data.as_ptr(), inlen as u64, kp, keylen,
                                sp, pp),
                            rsp(b.as_mut_ptr(), outlen, data.as_ptr(), inlen as u64, kp, keylen,
                                sp, pp),
                        )
                    };
                    eq_i32(&format!("salt_personal {what} rc"), ra, rb);
                    assert_eq!(ra, 0);
                    eq_bytes(
                        &format!("salt_personal {what}(out={outlen},key={keylen},in={inlen})"),
                        &a, &b,
                    );
                    variants.push((what.to_string(), a, b));
                }
                // all four must be bit-identical …
                for i in 1..variants.len() {
                    assert_eq!(
                        variants[0].1, variants[i].1,
                        "salt_personal {} != {} (out={outlen},key={keylen},in={inlen})",
                        variants[0].0, variants[i].0
                    );
                }
                // … and identical to plain crypto_generichash_blake2b
                let mut plain = canary(outlen);
                unsafe {
                    cp(plain.as_mut_ptr(), outlen, data.as_ptr(), inlen as u64, kp, keylen);
                }
                eq_bytes(
                    &format!("salt_personal NULL/NULL == plain (out={outlen},key={keylen},in={inlen})"),
                    &plain, &variants[0].1,
                );
            }
        }
    }

    // G4-030: keyed keylen = 64, random salt/personal, outlen = 64, updates fed
    // one byte at a time over 300 bytes.
    let (ci, ri) = pair::<GhInitSaltPers>("crypto_generichash_blake2b_init_salt_personal");
    let (cu, ru) = pair::<StUpdate>("crypto_generichash_blake2b_update");
    let (cf, rf) = pair::<GhFinal>("crypto_generichash_blake2b_final");
    for trial in 0..6 {
        let key = rng.bytes(64);
        let salt = rng.bytes(16);
        let pers = rng.bytes(16);
        let data = rng.bytes(300);
        for style in 0..3u32 {
            let parts: Vec<usize> = match style {
                0 => vec![300],
                1 => vec![1; 300],
                _ => chunks(&mut rng, 300, 3),
            };
            let mut out = [canary(64), canary(64)];
            for (which, (init, upd, fin)) in [(ci, cu, cf), (ri, ru, rf)].into_iter().enumerate() {
                let mut st = State::for_sym("crypto_generichash_blake2b_statebytes");
                unsafe {
                    assert_eq!(
                        init(st.as_mut_ptr(), key.as_ptr(), 64, 64, salt.as_ptr(), pers.as_ptr()),
                        0
                    );
                    let mut off = 0usize;
                    for &n in &parts {
                        assert_eq!(upd(st.as_mut_ptr(), data[off..].as_ptr(), n as u64), 0);
                        off += n;
                    }
                    assert_eq!(fin(st.as_mut_ptr(), out[which].as_mut_ptr(), 64), 0);
                }
            }
            let (a, b) = (out[0].clone(), out[1].clone());
            eq_bytes(&format!("init_salt_personal keyed 1-byte trial={trial} style={style}"), &a, &b);
            // must equal the `_salt_personal` one-shot
            let mut os = canary(64);
            unsafe {
                csp(os.as_mut_ptr(), 64, data.as_ptr(), 300, key.as_ptr(), 64,
                    salt.as_ptr(), pers.as_ptr());
            }
            eq_bytes("init_salt_personal streaming == one-shot", &os, &a);
        }
    }
}

// ===========================================================================
// crypto_shorthash — not touched by t03 at all
// ===========================================================================

/// G4-104, G4-105, G4-106 (siphash24 — every `left = inlen & 7` tail case with
/// 0, 1 and many full rounds), G4-108 (extreme keys), G4-109, G4-110
/// (siphashx24 — a completely different seed / finalisation), G4-111 (the
/// generic dispatcher) and G4-113 (`crypto_shorthash_keygen`).
#[test]
fn shorthash_full_matrix() {
    setup();
    let mut rng = Rng::new(0xF130);
    let mut lens: Vec<usize> = (0usize..=33).collect();
    lens.extend_from_slice(&[47, 48, 63, 64, 100, 127, 128, 129, 1000, 2000]);

    for &(prefix, dl) in &[
        ("crypto_shorthash_siphash24", 8usize),
        ("crypto_shorthash_siphashx24", 16),
        ("crypto_shorthash", 8),
    ] {
        let (c, r) = pair::<Short>(prefix);
        for &len in &lens {
            for kind in 0..5 {
                let k = match kind {
                    0 => rng.bytes(16),
                    1 => vec![0u8; 16],
                    2 => vec![0xffu8; 16],
                    3 => {
                        let mut v = vec![0u8; 16];
                        rng.fill(&mut v[8..]);
                        v
                    }
                    _ => {
                        let mut v = vec![0xffu8; 16];
                        rng.fill(&mut v[..8]);
                        v
                    }
                };
                let data = match kind {
                    1 => vec![0u8; len],
                    2 => vec![0xffu8; len],
                    _ => rng.bytes(len),
                };
                let mut a = canary(dl);
                let mut b = canary(dl);
                let (ra, rb) = unsafe {
                    (
                        c(a.as_mut_ptr(), data.as_ptr(), len as u64, k.as_ptr()),
                        r(b.as_mut_ptr(), data.as_ptr(), len as u64, k.as_ptr()),
                    )
                };
                eq_i32(&format!("{prefix}(len={len},kind={kind}) rc"), ra, rb);
                assert_eq!(ra, 0);
                eq_bytes(&format!("{prefix}(len={len},kind={kind})"), &a, &b);
            }
        }
    }

    // G4-111: the generic dispatcher is a pure forward to siphash24.
    let (cg, _) = pair::<Short>("crypto_shorthash");
    let (cs, _) = pair::<Short>("crypto_shorthash_siphash24");
    let (cx, _) = pair::<Short>("crypto_shorthash_siphashx24");
    for &len in &[0usize, 1, 7, 8, 9, 15, 16, 17, 100] {
        let k = rng.bytes(16);
        let data = rng.bytes(len);
        let mut g = canary(8);
        let mut s = canary(8);
        let mut x = canary(16);
        unsafe {
            cg(g.as_mut_ptr(), data.as_ptr(), len as u64, k.as_ptr());
            cs(s.as_mut_ptr(), data.as_ptr(), len as u64, k.as_ptr());
            cx(x.as_mut_ptr(), data.as_ptr(), len as u64, k.as_ptr());
        }
        eq_bytes(&format!("crypto_shorthash == siphash24(len={len})"), &s, &g);
        // G4-110: the 128-bit variant must agree with siphash24 on NOTHING.
        assert_ne!(
            &x[..8], &s[..],
            "siphashx24 must not alias siphash24 (len={len})"
        );
    }

    // G4-113: keygen fills exactly 16 bytes.
    let (ck, rk) = pair::<Keygen>("crypto_shorthash_keygen");
    for seed in 0..8u64 {
        let mut a = canary(16);
        let mut b = canary(16);
        reset_rngs(0xF130_0000 + seed);
        unsafe { ck(a.as_mut_ptr()) };
        reset_rngs(0xF130_0000 + seed);
        unsafe { rk(b.as_mut_ptr()) };
        eq_bytes("crypto_shorthash_keygen", &a, &b);
        assert_ne!(a, canary(16));
    }
    assert_eq!(usz("crypto_shorthash_keybytes"), 16);
}

// ===========================================================================
// Poly1305 gaps
// ===========================================================================

/// G4-092 (the `r`-clamp boundary keys, including `key[0..16] = 0` with random
/// `key[16..32]`), G4-094 (the named split patterns), G4-095 (`total = 100`
/// single-byte updates), G4-096 (`_update(0)` interleaved), G4-103
/// (`pick_best_implementation` leaves the results invariant).
#[test]
fn poly1305_named_splits_and_clamp_keys() {
    setup();
    let mut rng = Rng::new(0xF140);

    for prefix in ["crypto_onetimeauth", "crypto_onetimeauth_poly1305"] {
        let (c1, r1) = pair::<OtaOneShot>(prefix);
        let (ci, ri) = pair::<OtaInit>(&format!("{prefix}_init"));
        let (cu, ru) = pair::<StUpdate>(&format!("{prefix}_update"));
        let (cf, rf) = pair::<StFinal>(&format!("{prefix}_final"));
        let sb = format!("{prefix}_statebytes");

        // G4-092: keys at the clamp boundaries.
        let mut keys: Vec<(String, Vec<u8>)> = vec![
            ("all-ff".into(), vec![0xffu8; 32]),
            ("all-zero".into(), vec![0u8; 32]),
        ];
        {
            // r == 0 (the MAC is then just `pad`)
            let mut k = vec![0u8; 32];
            rng.fill(&mut k[16..]);
            keys.push(("r=0".into(), k));
            // r all-ff before clamping
            let mut k = vec![0xffu8; 32];
            rng.fill(&mut k[16..]);
            keys.push(("r=ff".into(), k));
            // only the bytes the clamp mask keeps
            let mut k = vec![0u8; 32];
            for i in [3usize, 7, 11, 15] {
                k[i] = 0xff;
            }
            rng.fill(&mut k[16..]);
            keys.push(("r=top-bytes".into(), k));
        }
        for (what, k) in &keys {
            for &len in &[0usize, 1, 15, 16, 17, 31, 32, 33, 63, 64, 65, 100, 1000] {
                let data = rng.bytes(len);
                let mut a = canary(16);
                let mut b = canary(16);
                let (ra, rb) = unsafe {
                    (
                        c1(a.as_mut_ptr(), data.as_ptr(), len as u64, k.as_ptr()),
                        r1(b.as_mut_ptr(), data.as_ptr(), len as u64, k.as_ptr()),
                    )
                };
                eq_i32(&format!("{prefix} key={what} len={len} rc"), ra, rb);
                assert_eq!(ra, 0);
                eq_bytes(&format!("{prefix} key={what} len={len}"), &a, &b);
            }
        }

        // G4-094 / G4-095 / G4-096: named splits, 100 one-byte updates,
        // interleaved zero-length updates.
        let mut patterns: Vec<Vec<usize>> = vec![
            vec![15, 1],
            vec![16, 1],
            vec![1, 15],
            vec![8, 8],
            vec![17, 15],
            vec![1, 31],
            vec![33, 31],
            vec![16, 16],
            vec![15, 15, 2],
            vec![1; 16],
            vec![1; 17],
            vec![1; 32],
            vec![1; 33],
            vec![1; 100],
            vec![0, 16, 0, 0, 17, 0],
            vec![0, 0, 0],
            vec![0, 1000, 0],
        ];
        patterns.push(chunks(&mut rng, 1000, 3));
        for parts in &patterns {
            let total: usize = parts.iter().sum();
            let k = rng.bytes(32);
            let data = rng.bytes(total);
            let mut out = [canary(16), canary(16)];
            for (which, (init, upd, fin)) in [(ci, cu, cf), (ri, ru, rf)].into_iter().enumerate() {
                let mut st = State::for_sym(&sb);
                unsafe {
                    assert_eq!(init(st.as_mut_ptr(), k.as_ptr()), 0);
                    let mut off = 0usize;
                    for &n in parts {
                        let p = if n == 0 && off == total {
                            std::ptr::null()
                        } else {
                            data[off..].as_ptr()
                        };
                        assert_eq!(upd(st.as_mut_ptr(), p, n as u64), 0);
                        off += n;
                    }
                    assert_eq!(fin(st.as_mut_ptr(), out[which].as_mut_ptr()), 0);
                }
            }
            let (a, b) = (out[0].clone(), out[1].clone());
            let label = if parts.len() > 8 {
                format!("split {}x (total={total})", parts.len())
            } else {
                format!("split {parts:?}")
            };
            eq_bytes(&format!("{prefix} {label}"), &a, &b);
            let mut os = canary(16);
            unsafe { c1(os.as_mut_ptr(), data.as_ptr(), total as u64, k.as_ptr()) };
            eq_bytes(&format!("{prefix} {label} == one-shot"), &os, &a);
        }
    }

    // G4-103: `pick_best_implementation` always re-selects donna, so results
    // are invariant across calls.
    type PickBest = unsafe extern "C" fn() -> i32;
    let (cpb, rpb) = pair::<PickBest>("_crypto_onetimeauth_poly1305_pick_best_implementation");
    let (c1, r1) = pair::<OtaOneShot>("crypto_onetimeauth_poly1305");
    let k = rng.bytes(32);
    let data = rng.bytes(777);
    let mut base = canary(16);
    unsafe { c1(base.as_mut_ptr(), data.as_ptr(), 777, k.as_ptr()) };
    for round in 0..4 {
        let (ra, rb) = unsafe { (cpb(), rpb()) };
        eq_i32("poly1305 pick_best rc", ra, rb);
        assert_eq!(ra, 0);
        let mut a = canary(16);
        let mut b = canary(16);
        unsafe {
            c1(a.as_mut_ptr(), data.as_ptr(), 777, k.as_ptr());
            r1(b.as_mut_ptr(), data.as_ptr(), 777, k.as_ptr());
        }
        eq_bytes(&format!("poly1305 tag after pick_best #{round}"), &a, &b);
        eq_bytes(&format!("poly1305 tag invariant #{round}"), &base, &a);
    }
}

// ===========================================================================
// XOFs — named absorb / squeeze patterns, cross-checks, state copy
// ===========================================================================

/// (prefix, rate)
const XOFS: &[(&str, usize)] = &[
    ("crypto_xof_shake128", 168),
    ("crypto_xof_shake256", 136),
    ("crypto_xof_turboshake128", 168),
    ("crypto_xof_turboshake256", 136),
];

/// G4-057 / G4-062 / G4-065 / G4-068 (`outlen = 2 * rate` exactly, the one
/// value missing from `t03`'s cross product), G4-058 / G4-063 / G4-066 /
/// G4-069 (the named absorb splits `(rate-1,1) (rate,1) (1,rate-1)
/// (rate/2,rate/2)` plus one byte at a time over 400) and the named squeeze
/// chains `(32,32,32,32) (1x400) (rate-1,1) (rate,1) (rate+1,rate-1)
/// (rate,rate)`, G4-059 (successive squeezes concatenate exactly).
#[test]
fn xof_named_absorb_and_squeeze_patterns() {
    setup();
    let mut rng = Rng::new(0xF150);
    for &(prefix, rate) in XOFS {
        let (ci, ri) = pair::<StInit>(&format!("{prefix}_init"));
        let (cu, ru) = pair::<StUpdate>(&format!("{prefix}_update"));
        let (cs, rs) = pair::<XofSqueeze>(&format!("{prefix}_squeeze"));
        let (c1, r1) = pair::<XofOneShot>(prefix);
        let sb = format!("{prefix}_statebytes");

        // --- one-shot: outlen == 2 * rate exactly (and a couple of neighbours)
        for &inlen in &[0usize, 1, rate - 1, rate, rate + 1, 2 * rate, 1000] {
            for &outlen in &[2 * rate, 2 * rate - 1, 3 * rate, 400] {
                let data = rng.bytes(inlen);
                let mut a = canary(outlen);
                let mut b = canary(outlen);
                let (ra, rb) = unsafe {
                    (
                        c1(a.as_mut_ptr(), outlen, data.as_ptr(), inlen as u64),
                        r1(b.as_mut_ptr(), outlen, data.as_ptr(), inlen as u64),
                    )
                };
                eq_i32(&format!("{prefix}(in={inlen},out={outlen}) rc"), ra, rb);
                eq_bytes(&format!("{prefix}(in={inlen},out={outlen})"), &a, &b);
            }
        }

        // --- named absorb splits: every one must be invisible.
        let absorb: Vec<Vec<usize>> = vec![
            vec![rate - 1, 1],
            vec![rate, 1],
            vec![1, rate - 1],
            vec![rate / 2, rate / 2],
            vec![rate, rate],
            vec![2 * rate, 1],
            vec![1, rate, rate],
            vec![0, rate, 0, 1, 0],
            vec![1; 400],
        ];
        let total_out = 3 * rate + 29;
        for parts in &absorb {
            let total: usize = parts.iter().sum();
            let data = rng.bytes(total);
            let mut out = [canary(total_out), canary(total_out)];
            for (which, (init, upd, sqz)) in
                [(ci, cu, cs), (ri, ru, rs)].into_iter().enumerate()
            {
                let mut st = State::for_sym(&sb);
                unsafe {
                    assert_eq!(init(st.as_mut_ptr()), 0);
                    let mut off = 0usize;
                    for &n in parts {
                        let p = if n == 0 && off == total {
                            std::ptr::null()
                        } else {
                            data[off..].as_ptr()
                        };
                        assert_eq!(upd(st.as_mut_ptr(), p, n as u64), 0);
                        off += n;
                    }
                    assert_eq!(sqz(st.as_mut_ptr(), out[which].as_mut_ptr(), total_out), 0);
                }
            }
            let (a, b) = (out[0].clone(), out[1].clone());
            let label = if parts.len() > 8 {
                format!("absorb {}x1 (total={total})", parts.len())
            } else {
                format!("absorb {parts:?}")
            };
            eq_bytes(&format!("{prefix} {label}"), &a, &b);
            // must equal the one-shot
            let mut os = canary(total_out);
            unsafe { c1(os.as_mut_ptr(), total_out, data.as_ptr(), total as u64) };
            eq_bytes(&format!("{prefix} {label} == one-shot"), &os, &a);
        }

        // --- named squeeze chains after ONE absorb of 100 bytes: successive
        // squeezes must concatenate into exactly the one-big-squeeze stream.
        let data = rng.bytes(100);
        let chains: Vec<Vec<usize>> = vec![
            vec![32, 32, 32, 32],
            vec![1; 400],
            vec![rate - 1, 1],
            vec![rate, 1],
            vec![rate + 1, rate - 1],
            vec![rate, rate],
            vec![0, 1, 0, rate, 0, rate + 5, 0],
            vec![169, 167],
        ];
        for chain in &chains {
            let total: usize = chain.iter().sum();
            let mut out = [canary(total), canary(total)];
            for (which, (init, upd, sqz)) in
                [(ci, cu, cs), (ri, ru, rs)].into_iter().enumerate()
            {
                let mut st = State::for_sym(&sb);
                unsafe {
                    init(st.as_mut_ptr());
                    upd(st.as_mut_ptr(), data.as_ptr(), 100);
                    let mut off = 0usize;
                    for &n in chain {
                        let p = if n == 0 && off == total {
                            std::ptr::null_mut()
                        } else {
                            out[which][off..].as_mut_ptr()
                        };
                        assert_eq!(sqz(st.as_mut_ptr(), p, n), 0);
                        off += n;
                    }
                }
            }
            let (a, b) = (out[0].clone(), out[1].clone());
            let label = if chain.len() > 8 {
                format!("squeeze {}x1 (total={total})", chain.len())
            } else {
                format!("squeeze {chain:?}")
            };
            eq_bytes(&format!("{prefix} {label}"), &a, &b);
            let mut os = canary(total);
            unsafe { c1(os.as_mut_ptr(), total, data.as_ptr(), 100) };
            eq_bytes(&format!("{prefix} {label} concatenates == one big squeeze"), &os, &a);
        }
    }
}

/// G4-061 / G4-064 / G4-067 / G4-070 — the *named* domain bytes with the named
/// `inlen` set, plus the two cross-primitive identities the table asks for:
/// `shake*_init_with_domain(0x1F)` == `shake*_init`, and
/// `shake256` at rate 136 with `domain = 0x06` and `outlen = 32` reproduces
/// `crypto_hash_sha3256` exactly (and `outlen = 64` at rate 72 would be
/// sha3-512, which is what `crypto_hash_sha3512` is checked against here).
#[test]
fn xof_named_domains_and_sha3_cross_check() {
    setup();
    let mut rng = Rng::new(0xF151);
    const DOMAINS: &[u8] = &[0x01, 0x06, 0x1f, 0x7f, 0x00, 0x80, 0xff];

    for &(prefix, rate) in XOFS {
        let (cid, rid) = pair::<XofInitDomain>(&format!("{prefix}_init_with_domain"));
        let (ci, ri) = pair::<StInit>(&format!("{prefix}_init"));
        let (cu, ru) = pair::<StUpdate>(&format!("{prefix}_update"));
        let (cs, rs) = pair::<XofSqueeze>(&format!("{prefix}_squeeze"));
        let sb = format!("{prefix}_statebytes");

        for &dom in DOMAINS {
            for &inlen in &[0usize, rate - 1, rate, rate + 1] {
                let data = rng.bytes(inlen.max(1));
                let mut out = [canary(64), canary(64)];
                let mut rcs = [9i32; 2];
                for (which, (init, upd, sqz)) in
                    [(cid, cu, cs), (rid, ru, rs)].into_iter().enumerate()
                {
                    let mut st = State::for_sym(&sb);
                    unsafe {
                        rcs[which] = init(st.as_mut_ptr(), dom);
                        upd(st.as_mut_ptr(), data.as_ptr(), inlen as u64);
                        sqz(st.as_mut_ptr(), out[which].as_mut_ptr(), 64);
                    }
                }
                eq_i32(&format!("{prefix}_init_with_domain({dom:#04x}) rc"), rcs[0], rcs[1]);
                assert_eq!(rcs[0], 0);
                let (a, b) = (out[0].clone(), out[1].clone());
                eq_bytes(&format!("{prefix} domain={dom:#04x} in={inlen}"), &a, &b);

                // domain 0x1F must equal `_init`
                if dom == 0x1f {
                    let mut plain = canary(64);
                    let mut st = State::for_sym(&sb);
                    unsafe {
                        ci(st.as_mut_ptr());
                        cu(st.as_mut_ptr(), data.as_ptr(), inlen as u64);
                        cs(st.as_mut_ptr(), plain.as_mut_ptr(), 64);
                    }
                    eq_bytes(&format!("{prefix} domain 0x1f == _init"), &plain, &a);
                    // and the Rust `_init` too
                    let mut plain_r = canary(64);
                    let mut st = State::for_sym(&sb);
                    unsafe {
                        ri(st.as_mut_ptr());
                        ru(st.as_mut_ptr(), data.as_ptr(), inlen as u64);
                        rs(st.as_mut_ptr(), plain_r.as_mut_ptr(), 64);
                    }
                    eq_bytes(&format!("{prefix} Rust domain 0x1f == _init"), &plain, &plain_r);
                }
            }
        }
    }

    // G4-064: shake256 (rate 136) + domain 0x06 + outlen 32 == sha3-256.
    for &(xof, hash, dl) in &[
        ("crypto_xof_shake256", "crypto_hash_sha3256", 32usize),
    ] {
        let (cid, rid) = pair::<XofInitDomain>(&format!("{xof}_init_with_domain"));
        let (cu, ru) = pair::<StUpdate>(&format!("{xof}_update"));
        let (cs, rs) = pair::<XofSqueeze>(&format!("{xof}_squeeze"));
        let (ch, rh) = pair::<HashOneShot>(hash);
        let sb = format!("{xof}_statebytes");
        for &inlen in &[0usize, 1, 135, 136, 137, 271, 272, 273, 1000] {
            let data = rng.bytes(inlen);
            let mut xo = [canary(dl), canary(dl)];
            for (which, (init, upd, sqz)) in
                [(cid, cu, cs), (rid, ru, rs)].into_iter().enumerate()
            {
                let mut st = State::for_sym(&sb);
                unsafe {
                    assert_eq!(init(st.as_mut_ptr(), 0x06), 0);
                    upd(st.as_mut_ptr(), data.as_ptr(), inlen as u64);
                    sqz(st.as_mut_ptr(), xo[which].as_mut_ptr(), dl);
                }
            }
            let mut hc = canary(dl);
            let mut hr = canary(dl);
            unsafe {
                ch(hc.as_mut_ptr(), data.as_ptr(), inlen as u64);
                rh(hr.as_mut_ptr(), data.as_ptr(), inlen as u64);
            }
            let (a, b) = (xo[0].clone(), xo[1].clone());
            eq_bytes(&format!("{xof} domain 0x06 (in={inlen})"), &a, &b);
            eq_bytes(&format!("{hash} (in={inlen})"), &hc, &hr);
            eq_bytes(
                &format!("{xof}(domain=0x06,out={dl}) == {hash} (in={inlen})"),
                &hc, &a,
            );
        }
    }
}

/// G4-071 — there is no `_clone`; a plain struct copy of the opaque
/// `*_state` must duplicate the sponge exactly, so both copies produce the
/// same continuation of the stream (and that continuation must be the same for
/// C and Rust).
#[test]
fn xof_state_struct_copy_duplicates_the_sponge() {
    setup();
    let mut rng = Rng::new(0xF152);
    for &(prefix, rate) in XOFS {
        let (ci, ri) = pair::<StInit>(&format!("{prefix}_init"));
        let (cu, ru) = pair::<StUpdate>(&format!("{prefix}_update"));
        let (cs, rs) = pair::<XofSqueeze>(&format!("{prefix}_squeeze"));
        let sbn = usz(&format!("{prefix}_statebytes"));
        assert_eq!(sbn, 256);

        for &inlen in &[0usize, 1, rate - 1, rate, rate + 1, 2 * rate + 7] {
            for &pre in &[0usize, 1, 32, rate] {
                let data = rng.bytes(inlen.max(1));
                let tail = 2 * rate + 11;
                let mut outs = [
                    [canary(tail), canary(tail)],
                    [canary(tail), canary(tail)],
                ];
                for (which, (init, upd, sqz)) in
                    [(ci, cu, cs), (ri, ru, rs)].into_iter().enumerate()
                {
                    let mut st = State::new(sbn);
                    let mut cp = State::new(sbn);
                    unsafe {
                        init(st.as_mut_ptr());
                        upd(st.as_mut_ptr(), data.as_ptr(), inlen as u64);
                        if pre > 0 {
                            let mut skip = canary(pre);
                            sqz(st.as_mut_ptr(), skip.as_mut_ptr(), pre);
                        }
                        // duplicate the opaque state by a plain byte copy
                        std::ptr::copy_nonoverlapping(st.as_ptr(), cp.as_mut_ptr(), sbn);
                        sqz(st.as_mut_ptr(), outs[0][which].as_mut_ptr(), tail);
                        sqz(cp.as_mut_ptr(), outs[1][which].as_mut_ptr(), tail);
                    }
                }
                let (a, b) = (outs[0][0].clone(), outs[0][1].clone());
                eq_bytes(&format!("{prefix} original after copy (in={inlen},pre={pre})"), &a, &b);
                let (ca, cb) = (outs[1][0].clone(), outs[1][1].clone());
                eq_bytes(&format!("{prefix} copy (in={inlen},pre={pre})"), &ca, &cb);
                assert_eq!(a, ca, "{prefix}: a struct copy must duplicate the sponge");
            }
        }
    }
}

// ===========================================================================
// crypto_core_keccak1600 — exhaustive offset/length matrices
//
// NOTE: `src/crypto_core/` is owned by a different module group; these tests
// only *observe* it. A failure here is reported, not fixed locally.
// ===========================================================================

/// G4-078 (`_xor_bytes` `offset` 0..9 x `length` {0,1,7,8,9,15,16,17} — the
/// three-phase head/word/tail loop), G4-079 (`offset = 0` x the four live
/// rates plus the full state), G4-080 (byte-at-a-time absorption == one bulk
/// absorb), G4-081 (`length = 0` is a complete no-op), G4-082 (XOR
/// involution).
#[test]
fn keccak1600_xor_bytes_matrix() {
    setup();
    let mut rng = Rng::new(0xF160);
    let (ci, ri) = pair::<KcInit>("crypto_core_keccak1600_init");
    let (cx, rx) = pair::<KcXor>("crypto_core_keccak1600_xor_bytes");
    let (ce, re) = pair::<KcExtract>("crypto_core_keccak1600_extract_bytes");
    let sbn = usz("crypto_core_keccak1600_statebytes");

    // G4-078: every `offset % 8` head length x every tail length.
    for offset in 0usize..10 {
        for &length in &[0usize, 1, 7, 8, 9, 15, 16, 17] {
            assert!(offset + length <= 200);
            let data = rng.bytes(length.max(1));
            let mut fin = [canary(200), canary(200)];
            for (which, (init, xorb, ext)) in [(ci, cx, ce), (ri, rx, re)].into_iter().enumerate() {
                let mut st = State::new(sbn);
                unsafe {
                    init(st.as_mut_ptr());
                    xorb(st.as_mut_ptr(), data.as_ptr(), offset, length);
                    ext(st.as_ptr(), fin[which].as_mut_ptr(), 0, 200);
                }
            }
            let (a, b) = (fin[0].clone(), fin[1].clone());
            eq_bytes(&format!("keccak xor(off={offset},len={length})"), &a, &b);
            // the XOR landed exactly where it should and nowhere else
            let mut expect = vec![0u8; 200];
            expect[offset..offset + length].copy_from_slice(&data[..length]);
            assert_eq!(a, expect, "keccak xor(off={offset},len={length}) placement");
        }
    }

    // G4-079: offset 0, length in {72, 136, 168, 200} — the pure word loop.
    for &length in &[72usize, 136, 168, 200] {
        let data = rng.bytes(length);
        let mut fin = [canary(200), canary(200)];
        for (which, (init, xorb, ext)) in [(ci, cx, ce), (ri, rx, re)].into_iter().enumerate() {
            let mut st = State::new(sbn);
            unsafe {
                init(st.as_mut_ptr());
                xorb(st.as_mut_ptr(), data.as_ptr(), 0, length);
                ext(st.as_ptr(), fin[which].as_mut_ptr(), 0, 200);
            }
        }
        let (a, b) = (fin[0].clone(), fin[1].clone());
        eq_bytes(&format!("keccak xor(0,{length})"), &a, &b);
    }

    // G4-080: byte-at-a-time absorption == one bulk absorb (this is exactly
    // the invariant the shake `_update` chunking relies on).
    for &length in &[72usize, 136, 168, 200] {
        let data = rng.bytes(length);
        let mut fin = [canary(200), canary(200), canary(200), canary(200)];
        for (which, (init, xorb, ext)) in [(ci, cx, ce), (ri, rx, re)].into_iter().enumerate() {
            let mut st = State::new(sbn);
            unsafe {
                init(st.as_mut_ptr());
                for i in 0..length {
                    xorb(st.as_mut_ptr(), data[i..].as_ptr(), i, 1);
                }
                ext(st.as_ptr(), fin[which].as_mut_ptr(), 0, 200);
                let mut st2 = State::new(sbn);
                init(st2.as_mut_ptr());
                xorb(st2.as_mut_ptr(), data.as_ptr(), 0, length);
                ext(st2.as_ptr(), fin[2 + which].as_mut_ptr(), 0, 200);
            }
        }
        let (a, b) = (fin[0].clone(), fin[1].clone());
        eq_bytes(&format!("keccak incremental xor len={length}"), &a, &b);
        let (c, d) = (fin[2].clone(), fin[3].clone());
        eq_bytes(&format!("keccak bulk xor len={length}"), &c, &d);
        assert_eq!(a, c, "keccak: incremental XOR must equal one bulk XOR");
    }

    // G4-081: length == 0 at any offset is a no-op. G4-082: XOR involution.
    for offset in [0usize, 1, 7, 8, 71, 135, 167, 199, 200] {
        let base = rng.bytes(200);
        let data = rng.bytes(64);
        let mut fin = [canary(200), canary(200), canary(200), canary(200)];
        for (which, (init, xorb, ext)) in [(ci, cx, ce), (ri, rx, re)].into_iter().enumerate() {
            let mut st = State::new(sbn);
            unsafe {
                init(st.as_mut_ptr());
                xorb(st.as_mut_ptr(), base.as_ptr(), 0, 200);
                let before = {
                    let mut v = canary(200);
                    ext(st.as_ptr(), v.as_mut_ptr(), 0, 200);
                    v
                };
                xorb(st.as_mut_ptr(), data.as_ptr(), offset, 0);
                ext(st.as_ptr(), fin[which].as_mut_ptr(), 0, 200);
                assert_eq!(fin[which], before, "keccak xor(len=0) is not a no-op");
                // involution: XOR the same bytes twice at the same offset
                let n = 64usize.min(200 - offset.min(200));
                if n > 0 {
                    xorb(st.as_mut_ptr(), data.as_ptr(), offset, n);
                    xorb(st.as_mut_ptr(), data.as_ptr(), offset, n);
                }
                ext(st.as_ptr(), fin[2 + which].as_mut_ptr(), 0, 200);
                assert_eq!(fin[2 + which], before, "keccak XOR is not involutive");
            }
        }
        let (a, b) = (fin[0].clone(), fin[1].clone());
        eq_bytes(&format!("keccak xor(len=0,off={offset})"), &a, &b);
        let (c, d) = (fin[2].clone(), fin[3].clone());
        eq_bytes(&format!("keccak xor involution off={offset}"), &c, &d);
    }
}

/// G4-083 (`_extract_bytes` `offset` x `length` matrix; extraction is
/// non-destructive so repeats must agree), G4-084 (chunked extraction ==
/// one big extract), G4-085 (`permute_24` applied 1/2/3 times to the zero
/// state), G4-086 (`permute_12`, and `2 x permute_12 != permute_24`),
/// G4-087 (full-state little-endian round trip through
/// `_xor_bytes(0,200)` + permute + `_extract_bytes(0,200)`).
#[test]
fn keccak1600_extract_and_permutations() {
    setup();
    let mut rng = Rng::new(0xF161);
    let (ci, ri) = pair::<KcInit>("crypto_core_keccak1600_init");
    let (cx, rx) = pair::<KcXor>("crypto_core_keccak1600_xor_bytes");
    let (ce, re) = pair::<KcExtract>("crypto_core_keccak1600_extract_bytes");
    let (c24, r24) = pair::<KcPermute>("crypto_core_keccak1600_permute_24");
    let (c12, r12) = pair::<KcPermute>("crypto_core_keccak1600_permute_12");
    let sbn = usz("crypto_core_keccak1600_statebytes");

    // G4-083: the offset x length matrix, twice each (non-destructive).
    let pattern = rng.bytes(200);
    for &offset in &[0usize, 1, 7, 8, 71, 135, 167, 199] {
        for &length in &[0usize, 1, 8, 33, 72, 136, 168] {
            if offset + length > 200 {
                continue;
            }
            let mut got = [canary(length.max(1)), canary(length.max(1))];
            let mut again = [canary(length.max(1)), canary(length.max(1))];
            for (which, (init, xorb, ext)) in [(ci, cx, ce), (ri, rx, re)].into_iter().enumerate() {
                let mut st = State::new(sbn);
                unsafe {
                    init(st.as_mut_ptr());
                    xorb(st.as_mut_ptr(), pattern.as_ptr(), 0, 200);
                    ext(st.as_ptr(), got[which].as_mut_ptr(), offset, length);
                    ext(st.as_ptr(), again[which].as_mut_ptr(), offset, length);
                }
            }
            let (a, b) = (got[0].clone(), got[1].clone());
            eq_bytes(&format!("keccak extract(off={offset},len={length})"), &a, &b);
            assert_eq!(a, again[0], "keccak extract must be non-destructive");
            if length > 0 {
                assert_eq!(&a[..length], &pattern[offset..offset + length]);
            }
        }
    }

    // G4-084: chunked extraction (0,32)(32,32)(64,32)(96,32) == one (0,128)
    for _ in 0..8 {
        let seed = rng.bytes(200);
        let mut chunked = [canary(128), canary(128)];
        let mut whole = [canary(128), canary(128)];
        for (which, (init, xorb, ext, p24)) in
            [(ci, cx, ce, c24), (ri, rx, re, r24)].into_iter().enumerate()
        {
            let mut st = State::new(sbn);
            unsafe {
                init(st.as_mut_ptr());
                xorb(st.as_mut_ptr(), seed.as_ptr(), 0, 200);
                p24(st.as_mut_ptr());
                for k in 0..4usize {
                    ext(st.as_ptr(), chunked[which][k * 32..].as_mut_ptr(), k * 32, 32);
                }
                ext(st.as_ptr(), whole[which].as_mut_ptr(), 0, 128);
            }
        }
        let (a, b) = (chunked[0].clone(), chunked[1].clone());
        eq_bytes("keccak chunked extract", &a, &b);
        let (c, d) = (whole[0].clone(), whole[1].clone());
        eq_bytes("keccak whole extract", &c, &d);
        assert_eq!(a, c, "keccak: chunked extraction must concatenate identically");
    }

    // G4-085 / G4-086 / G4-087: the permutations.
    for &n in &[1usize, 2, 3] {
        for kind in 0..3 {
            let seed: Vec<u8> = match kind {
                0 => vec![0u8; 200], // right after `_init`
                1 => rng.bytes(200),
                _ => (0..200u32).map(|i| i as u8).collect(),
            };
            let mut o24 = [canary(200), canary(200)];
            let mut o12 = [canary(200), canary(200)];
            for (which, (init, xorb, ext, p24, p12)) in
                [(ci, cx, ce, c24, c12), (ri, rx, re, r24, r12)].into_iter().enumerate()
            {
                let mut st = State::new(sbn);
                unsafe {
                    init(st.as_mut_ptr());
                    xorb(st.as_mut_ptr(), seed.as_ptr(), 0, 200);
                    for _ in 0..n {
                        p24(st.as_mut_ptr());
                    }
                    ext(st.as_ptr(), o24[which].as_mut_ptr(), 0, 200);
                    let mut st2 = State::new(sbn);
                    init(st2.as_mut_ptr());
                    xorb(st2.as_mut_ptr(), seed.as_ptr(), 0, 200);
                    for _ in 0..n {
                        p12(st2.as_mut_ptr());
                    }
                    ext(st2.as_ptr(), o12[which].as_mut_ptr(), 0, 200);
                }
            }
            let (a, b) = (o24[0].clone(), o24[1].clone());
            eq_bytes(&format!("keccak permute_24 x{n} kind={kind}"), &a, &b);
            let (c, d) = (o12[0].clone(), o12[1].clone());
            eq_bytes(&format!("keccak permute_12 x{n} kind={kind}"), &c, &d);
            assert_ne!(a, c, "permute_24 and permute_12 must differ");
        }
    }
    // 2 x permute_12 must NOT equal permute_24 (different round constants).
    let seed = vec![0u8; 200];
    let mut twice12 = canary(200);
    let mut once24 = canary(200);
    unsafe {
        let mut st = State::new(sbn);
        ci(st.as_mut_ptr());
        cx(st.as_mut_ptr(), seed.as_ptr(), 0, 200);
        c12(st.as_mut_ptr());
        c12(st.as_mut_ptr());
        ce(st.as_ptr(), twice12.as_mut_ptr(), 0, 200);
        let mut st2 = State::new(sbn);
        ci(st2.as_mut_ptr());
        cx(st2.as_mut_ptr(), seed.as_ptr(), 0, 200);
        c24(st2.as_mut_ptr());
        ce(st2.as_ptr(), once24.as_mut_ptr(), 0, 200);
    }
    assert_ne!(twice12, once24, "2 x permute_12 must NOT equal permute_24");
}

// ===========================================================================
// KDFs
// ===========================================================================

/// G4-122 (`ctx` = 8 zero bytes / random / `"context1"`, and the identity with
/// `crypto_generichash_blake2b_salt_personal`), G4-123 (the `subkey_len`
/// values 31 and 33 missing from `t03`'s list).
#[test]
fn kdf_blake2b_context_identities() {
    setup();
    let mut rng = Rng::new(0xF170);
    let (csp, _) = pair::<GhSaltPers>("crypto_generichash_blake2b_salt_personal");

    for prefix in ["crypto_kdf_blake2b", "crypto_kdf"] {
        let (c, r) = pair::<KdfDerive>(&format!("{prefix}_derive_from_key"));
        let ctxs: Vec<(String, Vec<u8>)> = vec![
            ("zero".into(), vec![0u8; 8]),
            ("context1".into(), b"context1".to_vec()),
            ("ff".into(), vec![0xffu8; 8]),
            ("random".into(), rng.bytes(8)),
        ];
        for (what, ctx) in &ctxs {
            for &sklen in &[16usize, 17, 31, 32, 33, 63, 64] {
                for &id in &[0u64, 1, 2, 0xffff_ffff, 0x1_0000_0000, u64::MAX] {
                    let key = rng.bytes(32);
                    let mut a = canary(sklen);
                    let mut b = canary(sklen);
                    let (ra, rb) = unsafe {
                        (
                            c(a.as_mut_ptr(), sklen, id, ctx.as_ptr() as *const c_char,
                              key.as_ptr()),
                            r(b.as_mut_ptr(), sklen, id, ctx.as_ptr() as *const c_char,
                              key.as_ptr()),
                        )
                    };
                    eq_i32(&format!("{prefix} ctx={what} len={sklen} rc"), ra, rb);
                    assert_eq!(ra, 0);
                    eq_bytes(&format!("{prefix} ctx={what} len={sklen} id={id}"), &a, &b);

                    // the equivalent `_salt_personal` call: salt = LE(id) || 0…,
                    // personal = ctx || 0…, key = the 32-byte kdf key.
                    let mut salt = [0u8; 16];
                    salt[..8].copy_from_slice(&id.to_le_bytes());
                    let mut pers = [0u8; 16];
                    pers[..8].copy_from_slice(ctx);
                    let mut expect = canary(sklen);
                    unsafe {
                        csp(expect.as_mut_ptr(), sklen, std::ptr::null(), 0, key.as_ptr(), 32,
                            salt.as_ptr(), pers.as_ptr());
                    }
                    eq_bytes(
                        &format!("{prefix} ctx={what} == salt_personal (len={sklen},id={id})"),
                        &expect, &a,
                    );

                    // G4-122: the all-zero context is exactly `personal = NULL`.
                    if what == "zero" {
                        let mut null_pers = canary(sklen);
                        unsafe {
                            csp(null_pers.as_mut_ptr(), sklen, std::ptr::null(), 0,
                                key.as_ptr(), 32, salt.as_ptr(), std::ptr::null());
                        }
                        eq_bytes(
                            &format!("{prefix} ctx=0…0 == personal=NULL (len={sklen},id={id})"),
                            &null_pers, &a,
                        );
                    }
                }
            }
        }
    }
}

/// G4-129 / G4-139 (`extract`: the salt lengths straddling the HMAC block
/// size that `t03` misses — 63/64/65 for SHA-256 and 127/128/129 for
/// SHA-512), G4-131 / G4-141 (the same through `_extract_init`), G4-132 /
/// G4-141 (the named ikm split patterns and `_extract_update(NULL, 0)`
/// interleaved).
#[test]
fn hkdf_extract_block_boundaries_and_named_splits() {
    setup();
    let mut rng = Rng::new(0xF180);
    for &(prefix, kb, block) in &[
        ("crypto_kdf_hkdf_sha256", 32usize, 64usize),
        ("crypto_kdf_hkdf_sha512", 64, 128),
    ] {
        let (ce, re) = pair::<HkdfExtract>(&format!("{prefix}_extract"));
        let (ci, ri) = pair::<HkdfExInit>(&format!("{prefix}_extract_init"));
        let (cu, ru) = pair::<HkdfExUpdate>(&format!("{prefix}_extract_update"));
        let (cf, rf) = pair::<HkdfExFinal>(&format!("{prefix}_extract_final"));
        let sb = format!("{prefix}_statebytes");

        let salt_lens: Vec<usize> = vec![0, 1, 32, block - 1, block, block + 1, 100, 200];
        let ikm_lens: Vec<usize> = vec![0, 1, 32, 64, 100, 200];

        for &saltlen in &salt_lens {
            for &ikmlen in &ikm_lens {
                let salt = rng.bytes(saltlen);
                let ikm = rng.bytes(ikmlen);
                let sp = if saltlen == 0 {
                    std::ptr::null()
                } else {
                    salt.as_ptr()
                };
                let ip = if ikmlen == 0 {
                    std::ptr::null()
                } else {
                    ikm.as_ptr()
                };
                let mut a = canary(kb);
                let mut b = canary(kb);
                let (ra, rb) = unsafe {
                    (
                        ce(a.as_mut_ptr(), sp, saltlen, ip, ikmlen),
                        re(b.as_mut_ptr(), sp, saltlen, ip, ikmlen),
                    )
                };
                eq_i32(&format!("{prefix}_extract(salt={saltlen},ikm={ikmlen}) rc"), ra, rb);
                assert_eq!(ra, 0);
                eq_bytes(&format!("{prefix}_extract(salt={saltlen},ikm={ikmlen})"), &a, &b);

                // named ikm split patterns through the streaming trio
                let mut patterns: Vec<Vec<usize>> = Vec::new();
                if ikmlen >= 1 {
                    patterns.push(vec![ikmlen - 1, 1]);
                    patterns.push(vec![1, ikmlen - 1]);
                }
                patterns.push(vec![ikmlen]);
                patterns.push(vec![1; ikmlen]);
                patterns.push({
                    let mut v = vec![0usize];
                    v.push(ikmlen);
                    v.push(0);
                    v
                });
                if ikmlen >= 56 {
                    patterns.push(vec![55, 1, ikmlen - 56]);
                    patterns.push(vec![56, ikmlen - 56]);
                }
                if ikmlen >= 64 {
                    patterns.push(vec![63, 1, ikmlen - 64]);
                    patterns.push(vec![64, ikmlen - 64]);
                }
                if ikmlen >= 128 {
                    patterns.push(vec![111, 1, ikmlen - 112]);
                    patterns.push(vec![112, ikmlen - 112]);
                    patterns.push(vec![127, 1, ikmlen - 128]);
                    patterns.push(vec![128, ikmlen - 128]);
                }
                for parts in &patterns {
                    let mut out = [canary(kb), canary(kb)];
                    for (which, (init, upd, fin)) in
                        [(ci, cu, cf), (ri, ru, rf)].into_iter().enumerate()
                    {
                        let mut st = State::for_sym(&sb);
                        unsafe {
                            assert_eq!(init(st.as_mut_ptr(), sp, saltlen), 0);
                            let mut off = 0usize;
                            for &n in parts {
                                let p = if n == 0 || ikmlen == 0 {
                                    std::ptr::null()
                                } else {
                                    ikm[off..].as_ptr()
                                };
                                assert_eq!(upd(st.as_mut_ptr(), p, n), 0);
                                off += n;
                            }
                            assert_eq!(off, ikmlen);
                            assert_eq!(fin(st.as_mut_ptr(), out[which].as_mut_ptr()), 0);
                        }
                    }
                    let (x, y) = (out[0].clone(), out[1].clone());
                    let label = if parts.len() > 8 {
                        format!("{}x1", parts.len())
                    } else {
                        format!("{parts:?}")
                    };
                    eq_bytes(
                        &format!("{prefix}_extract stream(salt={saltlen},ikm={ikmlen},{label})"),
                        &x, &y,
                    );
                    eq_bytes(
                        &format!("{prefix}_extract stream == one-shot ({label})"),
                        &a, &x,
                    );
                }
            }
        }
    }
}

/// G4-133 / G4-142 (the `out_len` values missing from `t03`'s list: 31/65 for
/// SHA-256, 63/129 for SHA-512), G4-135 / G4-144 (multi-block chaining at
/// 65/96 and 129/192), G4-136 / G4-145 (`prk` = all-zero / all-0xFF /
/// `_extract` output / `_keygen` output).
#[test]
fn hkdf_expand_lengths_and_prk_variants() {
    setup();
    let mut rng = Rng::new(0xF181);
    for &(prefix, kb, bmax) in &[
        ("crypto_kdf_hkdf_sha256", 32usize, 8160usize),
        ("crypto_kdf_hkdf_sha512", 64, 16320),
    ] {
        let (cx, rx) = pair::<HkdfExpand>(&format!("{prefix}_expand"));
        let (ce, _) = pair::<HkdfExtract>(&format!("{prefix}_extract"));
        let (ck, _) = pair::<Keygen>(&format!("{prefix}_keygen"));

        // the four `prk` shapes the table asks for
        let mut prks: Vec<(String, Vec<u8>)> = vec![
            ("zero".into(), vec![0u8; kb]),
            ("ff".into(), vec![0xffu8; kb]),
        ];
        {
            let salt = rng.bytes(32);
            let ikm = rng.bytes(64);
            let mut p = canary(kb);
            unsafe { ce(p.as_mut_ptr(), salt.as_ptr(), 32, ikm.as_ptr(), 64) };
            prks.push(("extract".into(), p));
            let mut p = canary(kb);
            reset_rngs(0xF181_0000);
            unsafe { ck(p.as_mut_ptr()) };
            prks.push(("keygen".into(), p));
        }

        let out_lens: Vec<usize> = vec![
            0,
            1,
            kb - 1,
            kb,
            kb + 1,
            2 * kb,
            2 * kb + 1,
            3 * kb,
            kb + kb / 2,
            3 * kb + 7,
            bmax,
        ];
        for (what, prk) in &prks {
            for &outlen in &out_lens {
                for &ctxlen in &[0usize, 1, 8, 10, 32, 100] {
                    let ctx = rng.bytes(ctxlen);
                    let cp = if ctxlen == 0 {
                        std::ptr::null()
                    } else {
                        ctx.as_ptr() as *const c_char
                    };
                    let mut a = canary(outlen.max(1));
                    let mut b = canary(outlen.max(1));
                    let (ra, rb) = unsafe {
                        (
                            cx(a.as_mut_ptr(), outlen, cp, ctxlen, prk.as_ptr()),
                            rx(b.as_mut_ptr(), outlen, cp, ctxlen, prk.as_ptr()),
                        )
                    };
                    eq_i32(&format!("{prefix}_expand(prk={what},out={outlen}) rc"), ra, rb);
                    assert_eq!(ra, 0);
                    eq_bytes(
                        &format!("{prefix}_expand(prk={what},out={outlen},ctx={ctxlen})"),
                        &a, &b,
                    );
                    // multi-block chaining: a shorter output must be a prefix
                    // of a longer one for the same (prk, ctx).
                    if outlen > kb {
                        let mut short = canary(kb);
                        unsafe { cx(short.as_mut_ptr(), kb, cp, ctxlen, prk.as_ptr()) };
                        eq_bytes(
                            &format!("{prefix}_expand chaining prefix (out={outlen})"),
                            &short, &a[..kb],
                        );
                    }
                }
            }
        }
    }
}

/// G4-148 / G4-149 — the RFC-5869-shaped end-to-end `extract` + `expand`
/// matrices (`out_len` 42 and 82 are the values the RFC test vectors use).
#[test]
fn hkdf_end_to_end() {
    setup();
    let mut rng = Rng::new(0xF182);
    for &(prefix, kb, bmax, ikm_big) in &[
        ("crypto_kdf_hkdf_sha256", 32usize, 8160usize, 32usize),
        ("crypto_kdf_hkdf_sha512", 64, 16320, 64),
    ] {
        let (ce, re) = pair::<HkdfExtract>(&format!("{prefix}_extract"));
        let (cx, rx) = pair::<HkdfExpand>(&format!("{prefix}_expand"));
        for &saltlen in &[0usize, 32] {
            for &ikmlen in &[0usize, ikm_big] {
                let salt = rng.bytes(saltlen);
                let ikm = rng.bytes(ikmlen);
                let sp = if saltlen == 0 {
                    std::ptr::null()
                } else {
                    salt.as_ptr()
                };
                let ip = if ikmlen == 0 {
                    std::ptr::null()
                } else {
                    ikm.as_ptr()
                };
                let mut pa = canary(kb);
                let mut pb = canary(kb);
                let (ra, rb) = unsafe {
                    (
                        ce(pa.as_mut_ptr(), sp, saltlen, ip, ikmlen),
                        re(pb.as_mut_ptr(), sp, saltlen, ip, ikmlen),
                    )
                };
                eq_i32(&format!("{prefix} e2e extract rc"), ra, rb);
                assert_eq!(ra, 0);
                eq_bytes(&format!("{prefix} e2e prk(salt={saltlen},ikm={ikmlen})"), &pa, &pb);

                for &outlen in &[1usize, kb, 42, 82, bmax] {
                    for &ctxlen in &[0usize, 10] {
                        let ctx = rng.bytes(ctxlen);
                        let cp = if ctxlen == 0 {
                            std::ptr::null()
                        } else {
                            ctx.as_ptr() as *const c_char
                        };
                        let mut a = canary(outlen);
                        let mut b = canary(outlen);
                        // the Rust expand must also work off the Rust PRK
                        let (ra, rb) = unsafe {
                            (
                                cx(a.as_mut_ptr(), outlen, cp, ctxlen, pa.as_ptr()),
                                rx(b.as_mut_ptr(), outlen, cp, ctxlen, pb.as_ptr()),
                            )
                        };
                        eq_i32(&format!("{prefix} e2e expand(out={outlen}) rc"), ra, rb);
                        assert_eq!(ra, 0);
                        eq_bytes(
                            &format!("{prefix} e2e(salt={saltlen},ikm={ikmlen},out={outlen},ctx={ctxlen})"),
                            &a, &b,
                        );
                    }
                }
            }
        }
    }
}
