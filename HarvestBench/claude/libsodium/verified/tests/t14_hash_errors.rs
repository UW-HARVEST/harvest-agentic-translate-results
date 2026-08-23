//! Phase C — error paths of the hash / MAC / XOF / KDF group
//! (`ERRORS.md` section `## G4`, rows `G4-001` … `G4-135`).
//!
//! Three kinds of row:
//!
//! * **`return -1` / `errno` rows** — driven directly against both `.so`s; the
//!   return value, `errno` and the output buffer (canary-filled, so any write
//!   is visible) must match byte-for-byte.
//! * **`sodium_misuse()` rows** — the handler runs and *then* `abort()`s, so
//!   each row runs in a child process with the observing handler installed
//!   (exit code `MISUSE_EXIT == 77`).
//! * **raw `assert()` rows** — the reference `.so` is built *without* `NDEBUG`;
//!   a failing `assert()` bypasses the misuse handler entirely and dies with
//!   SIGABRT. Those rows are also run out of process, but the expectation is
//!   "signal 6, no `MISUSE` line".
//!
//! Rows whose trigger is genuinely unreachable or undefined behaviour are
//! recorded in `documented_unreachable_error_rows` so nothing is silently
//! dropped.

mod common;
use common::*;

use std::ffi::{c_char, c_void};

// ---------------------------------------------------------------------------
// fn types
// ---------------------------------------------------------------------------

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
type GhUpdate = unsafe extern "C" fn(*mut u8, *const u8, u64) -> i32;
type GhFinal = unsafe extern "C" fn(*mut u8, *mut u8, usize) -> i32;

type B2Init = unsafe extern "C" fn(*mut u8, u8) -> i32;
type B2InitSp = unsafe extern "C" fn(*mut u8, u8, *const u8, *const u8) -> i32;
type B2InitKey = unsafe extern "C" fn(*mut u8, u8, *const u8, u8) -> i32;
type B2InitKeySp = unsafe extern "C" fn(*mut u8, u8, *const u8, u8, *const u8, *const u8) -> i32;

type HashOneShot = unsafe extern "C" fn(*mut u8, *const u8, u64) -> i32;
type StInit = unsafe extern "C" fn(*mut u8) -> i32;
type StUpdate = unsafe extern "C" fn(*mut u8, *const u8, u64) -> i32;
type StFinal = unsafe extern "C" fn(*mut u8, *mut u8) -> i32;

type Short = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8) -> i32;

type OtaOneShot = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8) -> i32;
type OtaVerify = unsafe extern "C" fn(*const u8, *const u8, u64, *const u8) -> i32;
type OtaInit = unsafe extern "C" fn(*mut u8, *const u8) -> i32;

type Verify = unsafe extern "C" fn(*const u8, *const u8) -> i32;

type XofOneShot = unsafe extern "C" fn(*mut u8, usize, *const u8, u64) -> i32;
type XofInitDomain = unsafe extern "C" fn(*mut u8, u8) -> i32;
type XofSqueeze = unsafe extern "C" fn(*mut u8, *mut u8, usize) -> i32;

type KdfDerive = unsafe extern "C" fn(*mut u8, usize, u64, *const c_char, *const u8) -> i32;

type HkdfExtract = unsafe extern "C" fn(*mut u8, *const u8, usize, *const u8, usize) -> i32;
type HkdfExpand = unsafe extern "C" fn(*mut u8, usize, *const c_char, usize, *const u8) -> i32;
type HkdfExInit = unsafe extern "C" fn(*mut u8, *const u8, usize) -> i32;
type HkdfExUpdate = unsafe extern "C" fn(*mut u8, *const u8, usize) -> i32;
type HkdfExFinal = unsafe extern "C" fn(*mut u8, *mut u8) -> i32;

type SizeFn = unsafe extern "C" fn() -> usize;
type U8Fn = unsafe extern "C" fn() -> u8;
type StrFn = unsafe extern "C" fn() -> *const c_char;

const EINVAL: i32 = 22;

fn usz(lib: &'static libloading::Library, name: &str) -> usize {
    unsafe { sym::<SizeFn>(lib, name)() }
}

/// `true` when the symbol is absent from **both** libraries.
fn absent_in_both(name: &str) -> bool {
    unsafe { c_lib().get::<*const c_void>(name.as_bytes()) }.is_err()
        && unsafe { r_lib().get::<*const c_void>(name.as_bytes()) }.is_err()
}

fn clear_errno() {
    let _ = std::fs::metadata("/");
}
fn errno() -> i32 {
    std::io::Error::last_os_error().raw_os_error().unwrap_or(0)
}

fn cstr(lib: &'static libloading::Library, name: &str) -> String {
    unsafe {
        let p = sym::<StrFn>(lib, name)();
        std::ffi::CStr::from_ptr(p).to_string_lossy().into_owned()
    }
}

// ===========================================================================
// crypto_generichash — `outlen` / `keylen` range rejections
// ===========================================================================

/// G4-001, G4-002, G4-003 (`crypto_generichash_blake2b` one-shot),
/// G4-013, G4-014, G4-015 (the `crypto_generichash` dispatcher),
/// G4-021, G4-022, G4-023 (`..._salt_personal`).
///
/// Every one of those entry points range-checks **before** the `assert()`, so
/// even `outlen == SIZE_MAX` is a plain `-1` and never aborts.
#[test]
fn generichash_one_shot_outlen_keylen_rejected() {
    setup();
    let mut rng = Rng::new(0xE400);
    let bad_out: &[usize] = &[0, 65, 66, 100, 127, 128, 255, 256, 257, 1000, usize::MAX];
    let bad_key: &[usize] = &[65, 66, 100, 255, 256, 1000, usize::MAX];

    let (csp, rsp) = pair::<GhSaltPers>("crypto_generichash_blake2b_salt_personal");
    let salt = rng.bytes(16);
    let pers = rng.bytes(16);

    for prefix in ["crypto_generichash_blake2b", "crypto_generichash"] {
        let (c, r) = pair::<GhOneShot>(prefix);
        for &outlen in bad_out {
            for &keylen in &[0usize, 32] {
                let key = rng.bytes(keylen.max(1));
                let inp = rng.bytes(129);
                let kp = if keylen == 0 {
                    std::ptr::null()
                } else {
                    key.as_ptr()
                };
                let mut a = canary(80);
                let mut b = canary(80);
                let (ra, rb) = unsafe {
                    (
                        c(a.as_mut_ptr(), outlen, inp.as_ptr(), 129, kp, keylen),
                        r(b.as_mut_ptr(), outlen, inp.as_ptr(), 129, kp, keylen),
                    )
                };
                eq_i32(&format!("{prefix}(outlen={outlen},keylen={keylen}) rc"), ra, rb);
                assert_eq!(ra, -1, "{prefix} must reject outlen={outlen}");
                eq_bytes(&format!("{prefix}(outlen={outlen}) out"), &a, &b);
                assert_eq!(a, canary(80), "{prefix} wrote to out on rejection");
            }
        }
        for &keylen in bad_key {
            // `key != NULL` so the NULL-key misuse path is not the one hit.
            let key = rng.bytes(64);
            let inp = rng.bytes(65);
            let mut a = canary(80);
            let mut b = canary(80);
            let (ra, rb) = unsafe {
                (
                    c(a.as_mut_ptr(), 32, inp.as_ptr(), 65, key.as_ptr(), keylen),
                    r(b.as_mut_ptr(), 32, inp.as_ptr(), 65, key.as_ptr(), keylen),
                )
            };
            eq_i32(&format!("{prefix}(keylen={keylen}) rc"), ra, rb);
            assert_eq!(ra, -1, "{prefix} must reject keylen={keylen}");
            eq_bytes(&format!("{prefix}(keylen={keylen}) out"), &a, &b);
            assert_eq!(a, canary(80));
        }
    }

    // `_salt_personal` has the identical guard (G4-021 / G4-022 / G4-023).
    for &outlen in bad_out {
        let inp = rng.bytes(200);
        let mut a = canary(80);
        let mut b = canary(80);
        let (ra, rb) = unsafe {
            (
                csp(a.as_mut_ptr(), outlen, inp.as_ptr(), 200, std::ptr::null(), 0,
                    salt.as_ptr(), pers.as_ptr()),
                rsp(b.as_mut_ptr(), outlen, inp.as_ptr(), 200, std::ptr::null(), 0,
                    salt.as_ptr(), pers.as_ptr()),
            )
        };
        eq_i32(&format!("salt_personal(outlen={outlen}) rc"), ra, rb);
        assert_eq!(ra, -1);
        eq_bytes(&format!("salt_personal(outlen={outlen}) out"), &a, &b);
        assert_eq!(a, canary(80));
    }
    for &keylen in bad_key {
        let key = rng.bytes(64);
        let inp = rng.bytes(1);
        let mut a = canary(80);
        let mut b = canary(80);
        let (ra, rb) = unsafe {
            (
                csp(a.as_mut_ptr(), 32, inp.as_ptr(), 1, key.as_ptr(), keylen,
                    salt.as_ptr(), pers.as_ptr()),
                rsp(b.as_mut_ptr(), 32, inp.as_ptr(), 1, key.as_ptr(), keylen,
                    salt.as_ptr(), pers.as_ptr()),
            )
        };
        eq_i32(&format!("salt_personal(keylen={keylen}) rc"), ra, rb);
        assert_eq!(ra, -1);
        eq_bytes(&format!("salt_personal(keylen={keylen}) out"), &a, &b);
        assert_eq!(a, canary(80));
    }
}

/// G4-030, G4-031, G4-032 (`crypto_generichash_blake2b_init`),
/// G4-046, G4-047, G4-048 (`..._init_salt_personal`),
/// G4-053, G4-054, G4-055 (the generic `crypto_generichash_init`).
#[test]
fn generichash_init_outlen_keylen_rejected() {
    setup();
    let mut rng = Rng::new(0xE401);
    let bad_out: &[usize] = &[0, 65, 66, 100, 255, 256, 257, 1000, usize::MAX];
    let bad_key: &[usize] = &[65, 66, 100, 255, 256, usize::MAX];
    let salt = rng.bytes(16);
    let pers = rng.bytes(16);
    let key = rng.bytes(64);

    for prefix in ["crypto_generichash_blake2b", "crypto_generichash"] {
        let (ci, ri) = pair::<GhInit>(&format!("{prefix}_init"));
        let sb = format!("{prefix}_statebytes");
        for &outlen in bad_out {
            for &keylen in &[0usize, 32] {
                let kp = if keylen == 0 {
                    std::ptr::null()
                } else {
                    key.as_ptr()
                };
                let mut sa = State::for_sym(&sb);
                let mut sr = State::for_sym(&sb);
                let (ra, rb) =
                    unsafe { (ci(sa.as_mut_ptr(), kp, keylen, outlen), ri(sr.as_mut_ptr(), kp, keylen, outlen)) };
                eq_i32(&format!("{prefix}_init(outlen={outlen}) rc"), ra, rb);
                assert_eq!(ra, -1, "{prefix}_init must reject outlen={outlen}");
            }
        }
        for &keylen in bad_key {
            let mut sa = State::for_sym(&sb);
            let mut sr = State::for_sym(&sb);
            let (ra, rb) = unsafe {
                (
                    ci(sa.as_mut_ptr(), key.as_ptr(), keylen, 32),
                    ri(sr.as_mut_ptr(), key.as_ptr(), keylen, 32),
                )
            };
            eq_i32(&format!("{prefix}_init(keylen={keylen}) rc"), ra, rb);
            assert_eq!(ra, -1, "{prefix}_init must reject keylen={keylen}");
        }
    }

    let (ci, ri) = pair::<GhInitSaltPers>("crypto_generichash_blake2b_init_salt_personal");
    for &outlen in bad_out {
        let mut sa = State::for_sym("crypto_generichash_blake2b_statebytes");
        let mut sr = State::for_sym("crypto_generichash_blake2b_statebytes");
        let (ra, rb) = unsafe {
            (
                ci(sa.as_mut_ptr(), std::ptr::null(), 0, outlen, salt.as_ptr(), pers.as_ptr()),
                ri(sr.as_mut_ptr(), std::ptr::null(), 0, outlen, salt.as_ptr(), pers.as_ptr()),
            )
        };
        eq_i32(&format!("init_salt_personal(outlen={outlen}) rc"), ra, rb);
        assert_eq!(ra, -1);
    }
    for &keylen in bad_key {
        let mut sa = State::for_sym("crypto_generichash_blake2b_statebytes");
        let mut sr = State::for_sym("crypto_generichash_blake2b_statebytes");
        let (ra, rb) = unsafe {
            (
                ci(sa.as_mut_ptr(), key.as_ptr(), keylen, 32, salt.as_ptr(), pers.as_ptr()),
                ri(sr.as_mut_ptr(), key.as_ptr(), keylen, 32, salt.as_ptr(), pers.as_ptr()),
            )
        };
        eq_i32(&format!("init_salt_personal(keylen={keylen}) rc"), ra, rb);
        assert_eq!(ra, -1);
    }
}

/// G4-019, G4-020 — `BYTES_MIN` / `KEYBYTES_MIN` are documentation-only:
/// `outlen == 1 … 15` and `keylen == 1 … 15` are **accepted**.
#[test]
fn generichash_below_advertised_minimums_accepted() {
    setup();
    let mut rng = Rng::new(0xE402);
    assert_eq!(usz(c_lib(), "crypto_generichash_bytes_min"), 16);
    assert_eq!(usz(c_lib(), "crypto_generichash_keybytes_min"), 16);
    for prefix in ["crypto_generichash_blake2b", "crypto_generichash"] {
        let (c, r) = pair::<GhOneShot>(prefix);
        for outlen in 1usize..=15 {
            for keylen in 0usize..=15 {
                let key = rng.bytes(keylen.max(1));
                let n = rng.range(0, 200);
                let inp = rng.bytes(n);
                let kp = if keylen == 0 {
                    std::ptr::null()
                } else {
                    key.as_ptr()
                };
                let mut a = canary(outlen);
                let mut b = canary(outlen);
                let (ra, rb) = unsafe {
                    (
                        c(a.as_mut_ptr(), outlen, inp.as_ptr(), inp.len() as u64, kp, keylen),
                        r(b.as_mut_ptr(), outlen, inp.as_ptr(), inp.len() as u64, kp, keylen),
                    )
                };
                eq_i32(&format!("{prefix}(out={outlen},key={keylen}) rc"), ra, rb);
                assert_eq!(ra, 0, "{prefix} must accept outlen={outlen} keylen={keylen}");
                eq_bytes(&format!("{prefix}(out={outlen},key={keylen})"), &a, &b);
            }
        }
    }
}

/// G4-033, G4-034 (`_init`), G4-049, G4-050 (`_init_salt_personal`).
///
/// The NULL-key asymmetry: the one-shot `sodium_misuse()`s on
/// `key == NULL && keylen > 0` (see `misuse_child`), but `_init` silently does
/// an **unkeyed** init and returns `0`. `key != NULL && keylen == 0` is also
/// unkeyed. `salt`/`personal == NULL` is 16 zero bytes.
#[test]
fn generichash_init_null_key_and_null_salt_accepted() {
    setup();
    let mut rng = Rng::new(0xE403);
    let key = rng.bytes(64);
    let zeros16 = [0u8; 16];
    let salt = rng.bytes(16);

    for prefix in ["crypto_generichash_blake2b", "crypto_generichash"] {
        let (ci, ri) = pair::<GhInit>(&format!("{prefix}_init"));
        let (cu, ru) = pair::<GhUpdate>(&format!("{prefix}_update"));
        let (cf, rf) = pair::<GhFinal>(&format!("{prefix}_final"));
        let sb = format!("{prefix}_statebytes");
        let (c1, _) = pair::<GhOneShot>(prefix);
        let inp = rng.bytes(300);

        // Reference: a genuinely unkeyed one-shot digest.
        let mut unkeyed = canary(32);
        unsafe { c1(unkeyed.as_mut_ptr(), 32, inp.as_ptr(), 300, std::ptr::null(), 0) };

        for &(kp, keylen, what) in &[
            (std::ptr::null(), 5usize, "key=NULL keylen=5"),
            (std::ptr::null(), 64usize, "key=NULL keylen=64"),
            (key.as_ptr(), 0usize, "key!=NULL keylen=0"),
        ] {
            let mut out = [canary(32), canary(32)];
            let mut rcs = [1i32; 2];
            for (which, (init, upd, fin)) in [(ci, cu, cf), (ri, ru, rf)].into_iter().enumerate() {
                let mut st = State::for_sym(&sb);
                unsafe {
                    rcs[which] = init(st.as_mut_ptr(), kp, keylen, 32);
                    upd(st.as_mut_ptr(), inp.as_ptr(), 300);
                    fin(st.as_mut_ptr(), out[which].as_mut_ptr(), 32);
                }
            }
            eq_i32(&format!("{prefix}_init({what}) rc"), rcs[0], rcs[1]);
            assert_eq!(rcs[0], 0, "{prefix}_init({what}) must be accepted");
            let (a, b) = (out[0].clone(), out[1].clone());
            eq_bytes(&format!("{prefix}_init({what}) digest"), &a, &b);
            eq_bytes(&format!("{prefix}_init({what}) == unkeyed"), &unkeyed, &a);
        }
    }

    // `_init_salt_personal`: NULL salt/personal == 16 zero bytes, and a NULL
    // key with keylen > 0 is silently unkeyed.
    let (ci, ri) = pair::<GhInitSaltPers>("crypto_generichash_blake2b_init_salt_personal");
    let (cu, ru) = pair::<GhUpdate>("crypto_generichash_blake2b_update");
    let (cf, rf) = pair::<GhFinal>("crypto_generichash_blake2b_final");
    let inp = rng.bytes(200);
    let mut digests: Vec<(String, Vec<u8>)> = Vec::new();
    for &(sp, pp, kp, keylen, what) in &[
        (std::ptr::null(), std::ptr::null(), std::ptr::null(), 0usize, "null/null"),
        (zeros16.as_ptr(), std::ptr::null(), std::ptr::null(), 0usize, "zero/null"),
        (std::ptr::null(), zeros16.as_ptr(), std::ptr::null(), 0usize, "null/zero"),
        (zeros16.as_ptr(), zeros16.as_ptr(), std::ptr::null(), 0usize, "zero/zero"),
        (salt.as_ptr(), std::ptr::null(), std::ptr::null(), 7usize, "salt/null key=NULL len=7"),
        (std::ptr::null(), std::ptr::null(), std::ptr::null(), 33usize, "null/null key=NULL len=33"),
        (std::ptr::null(), std::ptr::null(), key.as_ptr(), 0usize, "null/null key!=NULL len=0"),
    ] {
        let mut out = [canary(32), canary(32)];
        let mut rcs = [1i32; 2];
        for (which, (init, upd, fin)) in [(ci, cu, cf), (ri, ru, rf)].into_iter().enumerate() {
            let mut st = State::for_sym("crypto_generichash_blake2b_statebytes");
            unsafe {
                rcs[which] = init(st.as_mut_ptr(), kp, keylen, 32, sp, pp);
                upd(st.as_mut_ptr(), inp.as_ptr(), 200);
                fin(st.as_mut_ptr(), out[which].as_mut_ptr(), 32);
            }
        }
        eq_i32(&format!("init_salt_personal({what}) rc"), rcs[0], rcs[1]);
        assert_eq!(rcs[0], 0, "init_salt_personal({what}) must be accepted");
        let (a, b) = (out[0].clone(), out[1].clone());
        eq_bytes(&format!("init_salt_personal({what}) digest"), &a, &b);
        digests.push((what.to_string(), a));
    }
    // NULL salt/personal must be *identical* to all-zero salt/personal, and to
    // an unkeyed init with a NULL key of nonzero length.
    for i in 1..4 {
        assert_eq!(digests[0].1, digests[i].1, "{} != {}", digests[0].0, digests[i].0);
    }
    assert_eq!(digests[0].1, digests[5].1, "NULL key with keylen>0 must be unkeyed");
    assert_eq!(digests[0].1, digests[6].1, "keylen==0 must ignore the key pointer");
}

/// G4-056, G4-061, G4-062, G4-063 — the generichash `_update`/`_final`
/// state machine: `_update` never rejects anything (not even after `_final`),
/// a second `_final` returns `-1` and writes nothing, and `_final`'s `outlen`
/// is *not* validated against the `outlen` given to `_init`.
#[test]
fn generichash_final_state_machine() {
    setup();
    let mut rng = Rng::new(0xE404);
    for prefix in ["crypto_generichash_blake2b", "crypto_generichash"] {
        let (ci, ri) = pair::<GhInit>(&format!("{prefix}_init"));
        let (cu, ru) = pair::<GhUpdate>(&format!("{prefix}_update"));
        let (cf, rf) = pair::<GhFinal>(&format!("{prefix}_final"));
        let sb = format!("{prefix}_statebytes");

        for &init_out in &[1usize, 16, 32, 64] {
            for &fin_out in &[1usize, 16, 17, 31, 32, 63, 64] {
                let n = rng.range(0, 400);
                let inp = rng.bytes(n);
                // `_update` with in = NULL, inlen = 0 (G4-056) plus a real
                // update, then `_final` twice with an intervening `_update`
                // (G4-062).
                let mut d1 = [canary(64), canary(64)];
                let mut d2 = [canary(64), canary(64)];
                let mut rcs = [[0i32; 4]; 2];
                for (which, (init, upd, fin)) in
                    [(ci, cu, cf), (ri, ru, rf)].into_iter().enumerate()
                {
                    let mut st = State::for_sym(&sb);
                    unsafe {
                        assert_eq!(init(st.as_mut_ptr(), std::ptr::null(), 0, init_out), 0);
                        rcs[which][0] = upd(st.as_mut_ptr(), std::ptr::null(), 0);
                        upd(st.as_mut_ptr(), inp.as_ptr(), inp.len() as u64);
                        rcs[which][1] = fin(st.as_mut_ptr(), d1[which].as_mut_ptr(), fin_out);
                        // an update after `_final` still returns 0 …
                        rcs[which][2] = upd(st.as_mut_ptr(), inp.as_ptr(), inp.len() as u64);
                        // … but the second `_final` is latched off
                        rcs[which][3] = fin(st.as_mut_ptr(), d2[which].as_mut_ptr(), fin_out);
                    }
                }
                for k in 0..4 {
                    eq_i32(
                        &format!("{prefix} init={init_out} fin={fin_out} rc[{k}]"),
                        rcs[0][k], rcs[1][k],
                    );
                }
                assert_eq!(rcs[0][0], 0, "{prefix}_update(NULL,0) must return 0");
                assert_eq!(rcs[0][1], 0, "{prefix}_final #1 must return 0");
                assert_eq!(rcs[0][2], 0, "{prefix}_update after _final must return 0");
                assert_eq!(rcs[0][3], -1, "{prefix}_final #2 must return -1");
                let (a, b) = (d1[0].clone(), d1[1].clone());
                eq_bytes(&format!("{prefix} digest init={init_out} fin={fin_out}"), &a, &b);
                let (a2, b2) = (d2[0].clone(), d2[1].clone());
                eq_bytes(&format!("{prefix} 2nd final buffer"), &a2, &b2);
                assert_eq!(a2, canary(64), "{prefix} rejected 2nd _final wrote to out");
            }
        }

        // G4-063 explicitly: `_init(..., 32)` then `_final(..., 64)` returns 0
        // and the first 32 bytes equal the `_final(..., 32)` result, because
        // both copy from the same 64-byte `h[]` dump.
        let inp = rng.bytes(257);
        let mut wide = [canary(64), canary(64)];
        let mut narrow = canary(32);
        for (which, (init, upd, fin)) in [(ci, cu, cf), (ri, ru, rf)].into_iter().enumerate() {
            let mut st = State::for_sym(&sb);
            unsafe {
                assert_eq!(init(st.as_mut_ptr(), std::ptr::null(), 0, 32), 0);
                upd(st.as_mut_ptr(), inp.as_ptr(), 257);
                assert_eq!(fin(st.as_mut_ptr(), wide[which].as_mut_ptr(), 64), 0);
            }
        }
        {
            let mut st = State::for_sym(&sb);
            unsafe {
                ci(st.as_mut_ptr(), std::ptr::null(), 0, 32);
                cu(st.as_mut_ptr(), inp.as_ptr(), 257);
                cf(st.as_mut_ptr(), narrow.as_mut_ptr(), 32);
            }
        }
        let (a, b) = (wide[0].clone(), wide[1].clone());
        eq_bytes(&format!("{prefix} init(32)+final(64)"), &a, &b);
        eq_bytes(&format!("{prefix} init(32)+final(64) prefix"), &narrow, &a[..32]);
    }
}

// ===========================================================================
// SHA-2 / SHA-3
// ===========================================================================

/// G4-068, G4-069, G4-070, G4-071, G4-072, G4-073 — the SHA-2 family has no
/// error paths at all: `in == NULL && inlen == 0` is fine, `_update(0)` is a
/// no-op, re-`init` mid-stream restarts, `_update` after `_final` operates on
/// the zeroed state and a second `_final` returns 0.
#[test]
fn sha2_has_no_error_paths() {
    setup();
    let mut rng = Rng::new(0xE410);
    for &(prefix, dl) in &[
        ("crypto_hash_sha256", 32usize),
        ("crypto_hash_sha512", 64),
        ("crypto_hash", 64),
    ] {
        let (c, r) = pair::<HashOneShot>(prefix);
        // in == NULL && inlen == 0
        let mut a = canary(dl);
        let mut b = canary(dl);
        let (ra, rb) = unsafe {
            (
                c(a.as_mut_ptr(), std::ptr::null(), 0),
                r(b.as_mut_ptr(), std::ptr::null(), 0),
            )
        };
        eq_i32(&format!("{prefix}(NULL,0) rc"), ra, rb);
        assert_eq!(ra, 0);
        eq_bytes(&format!("{prefix}(NULL,0)"), &a, &b);
    }

    for &(prefix, dl) in &[("crypto_hash_sha256", 32usize), ("crypto_hash_sha512", 64)] {
        let (ci, ri) = pair::<StInit>(&format!("{prefix}_init"));
        let (cu, ru) = pair::<StUpdate>(&format!("{prefix}_update"));
        let (cf, rf) = pair::<StFinal>(&format!("{prefix}_final"));
        let (c1, _) = pair::<HashOneShot>(prefix);
        let sb = format!("{prefix}_statebytes");

        for trial in 0..12 {
            let n = rng.range(0, 400);
            let inp = rng.bytes(n);
            let n2 = rng.range(1, 200);
            let extra = rng.bytes(n2);
            let mut d1 = [canary(dl), canary(dl)];
            let mut d2 = [canary(dl), canary(dl)];
            let mut d3 = [canary(dl), canary(dl)];
            let mut rcs = [[0i32; 5]; 2];
            for (which, (init, upd, fin)) in
                [(ci, cu, cf), (ri, ru, rf)].into_iter().enumerate()
            {
                let mut st = State::for_sym(&sb);
                unsafe {
                    assert_eq!(init(st.as_mut_ptr()), 0);
                    // G4-070: 0-length update, both with a real and a NULL ptr
                    rcs[which][0] = upd(st.as_mut_ptr(), std::ptr::null(), 0);
                    rcs[which][1] = upd(st.as_mut_ptr(), inp.as_ptr(), 0);
                    upd(st.as_mut_ptr(), inp.as_ptr(), inp.len() as u64);
                    rcs[which][2] = fin(st.as_mut_ptr(), d1[which].as_mut_ptr());
                    // G4-071: update after final — no rejection
                    rcs[which][3] = upd(st.as_mut_ptr(), extra.as_ptr(), extra.len() as u64);
                    rcs[which][4] = fin(st.as_mut_ptr(), d2[which].as_mut_ptr());
                    // G4-069: re-init mid-stream silently restarts
                    let mut st2 = State::for_sym(&sb);
                    init(st2.as_mut_ptr());
                    upd(st2.as_mut_ptr(), extra.as_ptr(), extra.len() as u64);
                    init(st2.as_mut_ptr());
                    upd(st2.as_mut_ptr(), inp.as_ptr(), inp.len() as u64);
                    fin(st2.as_mut_ptr(), d3[which].as_mut_ptr());
                }
            }
            for k in 0..5 {
                eq_i32(&format!("{prefix} trial={trial} rc[{k}]"), rcs[0][k], rcs[1][k]);
                assert_eq!(rcs[0][k], 0, "{prefix} rc[{k}] must be 0");
            }
            let (a, b) = (d1[0].clone(), d1[1].clone());
            eq_bytes(&format!("{prefix} digest"), &a, &b);
            let (a2, b2) = (d2[0].clone(), d2[1].clone());
            eq_bytes(&format!("{prefix} post-final digest"), &a2, &b2);
            let (a3, b3) = (d3[0].clone(), d3[1].clone());
            eq_bytes(&format!("{prefix} re-init digest"), &a3, &b3);
            // re-init must equal the plain one-shot
            let mut os = canary(dl);
            unsafe { c1(os.as_mut_ptr(), inp.as_ptr(), inp.len() as u64) };
            eq_bytes(&format!("{prefix} re-init == one-shot"), &os, &a3);
            eq_bytes(&format!("{prefix} 0-length update is a no-op"), &os, &a);
        }
    }

    // G4-072: a second `_final` with no intervening update also returns 0 and
    // emits the digest of the empty message (the state was zeroed).
    for &(prefix, dl) in &[("crypto_hash_sha256", 32usize), ("crypto_hash_sha512", 64)] {
        let (ci, ri) = pair::<StInit>(&format!("{prefix}_init"));
        let (cu, ru) = pair::<StUpdate>(&format!("{prefix}_update"));
        let (cf, rf) = pair::<StFinal>(&format!("{prefix}_final"));
        let sb = format!("{prefix}_statebytes");
        let inp = rng.bytes(199);
        let mut d2 = [canary(dl), canary(dl)];
        let mut rc2 = [0i32; 2];
        for (which, (init, upd, fin)) in [(ci, cu, cf), (ri, ru, rf)].into_iter().enumerate() {
            let mut st = State::for_sym(&sb);
            let mut tmp = canary(dl);
            unsafe {
                init(st.as_mut_ptr());
                upd(st.as_mut_ptr(), inp.as_ptr(), 199);
                fin(st.as_mut_ptr(), tmp.as_mut_ptr());
                rc2[which] = fin(st.as_mut_ptr(), d2[which].as_mut_ptr());
            }
        }
        eq_i32(&format!("{prefix} second _final rc"), rc2[0], rc2[1]);
        assert_eq!(rc2[0], 0, "{prefix} second _final must return 0");
        let (a, b) = (d2[0].clone(), d2[1].clone());
        eq_bytes(&format!("{prefix} second _final digest"), &a, &b);
    }

    // G4-073 constants
    assert_eq!(usz(c_lib(), "crypto_hash_bytes"), 64);
    for lib in [c_lib(), r_lib()] {
        assert_eq!(usz(lib, "crypto_hash_bytes"), 64);
        assert_eq!(usz(lib, "crypto_hash_sha256_bytes"), 32);
        assert_eq!(usz(lib, "crypto_hash_sha512_bytes"), 64);
        assert_eq!(usz(lib, "crypto_hash_sha256_statebytes"), 104);
        assert_eq!(usz(lib, "crypto_hash_sha512_statebytes"), 208);
        assert_eq!(cstr(lib, "crypto_hash_primitive"), "sha512");
    }
}

/// G4-074, G4-075 (`_update` after `_final` returns `-1` but still permutes,
/// resets `offset` and absorbs), G4-076, G4-077 (a second `_final` returns
/// `-1` but STILL writes a full digest), G4-078, G4-079.
#[test]
fn sha3_post_final_state_machine() {
    setup();
    let mut rng = Rng::new(0xE411);
    for &(prefix, dl, rate) in &[
        ("crypto_hash_sha3256", 32usize, 136usize),
        ("crypto_hash_sha3512", 64, 72),
    ] {
        let (ci, ri) = pair::<StInit>(&format!("{prefix}_init"));
        let (cu, ru) = pair::<StUpdate>(&format!("{prefix}_update"));
        let (cf, rf) = pair::<StFinal>(&format!("{prefix}_final"));
        let (c1, r1) = pair::<HashOneShot>(prefix);
        let sb = format!("{prefix}_statebytes");

        // one-shot never fails, incl. in == NULL && inlen == 0 (G4-078)
        let mut a = canary(dl);
        let mut b = canary(dl);
        let (ra, rb) = unsafe {
            (
                c1(a.as_mut_ptr(), std::ptr::null(), 0),
                r1(b.as_mut_ptr(), std::ptr::null(), 0),
            )
        };
        eq_i32(&format!("{prefix}(NULL,0) rc"), ra, rb);
        assert_eq!(ra, 0);
        eq_bytes(&format!("{prefix}(NULL,0)"), &a, &b);

        for &inlen in &[0usize, 1, rate - 2, rate - 1, rate, rate + 1, 2 * rate, 300] {
            for &extra in &[0usize, 1, rate - 1, rate, rate + 3] {
                let inp = rng.bytes(inlen);
                let more = rng.bytes(extra.max(1));
                let mut d1 = [canary(dl), canary(dl)];
                let mut d2 = [canary(dl), canary(dl)];
                let mut d3 = [canary(dl), canary(dl)];
                let mut rcs = [[9i32; 4]; 2];
                for (which, (init, upd, fin)) in
                    [(ci, cu, cf), (ri, ru, rf)].into_iter().enumerate()
                {
                    let mut st = State::for_sym(&sb);
                    unsafe {
                        assert_eq!(init(st.as_mut_ptr()), 0, "{prefix}_init");
                        upd(st.as_mut_ptr(), inp.as_ptr(), inlen as u64);
                        rcs[which][0] = fin(st.as_mut_ptr(), d1[which].as_mut_ptr());
                        // G4-074/075: update after final -> -1, but the state
                        // recovers (permute + offset = 0) and absorbs anyway.
                        rcs[which][1] = upd(st.as_mut_ptr(), more.as_ptr(), extra as u64);
                        rcs[which][2] = fin(st.as_mut_ptr(), d2[which].as_mut_ptr());
                        // G4-076/077: a second final on a FINALIZED state -> -1
                        // yet a full `dl`-byte write.
                        rcs[which][3] = fin(st.as_mut_ptr(), d3[which].as_mut_ptr());
                    }
                }
                let what = format!("{prefix}(in={inlen},extra={extra})");
                for k in 0..4 {
                    eq_i32(&format!("{what} rc[{k}]"), rcs[0][k], rcs[1][k]);
                }
                assert_eq!(rcs[0][0], 0, "{what}: first _final must return 0");
                assert_eq!(rcs[0][1], -1, "{what}: _update after _final must return -1");
                assert_eq!(rcs[0][2], 0, "{what}: _final after the recovering _update -> 0");
                assert_eq!(rcs[0][3], -1, "{what}: second _final must return -1");
                let (x, y) = (d1[0].clone(), d1[1].clone());
                eq_bytes(&format!("{what} digest1"), &x, &y);
                let (x, y) = (d2[0].clone(), d2[1].clone());
                eq_bytes(&format!("{what} digest2 (after recovering update)"), &x, &y);
                let (x, y) = (d3[0].clone(), d3[1].clone());
                eq_bytes(&format!("{what} digest3 (rejected 2nd final)"), &x, &y);
                assert_ne!(x, canary(dl), "{what}: rejected 2nd _final still writes");
            }
        }
    }

    // G4-079 constants
    for lib in [c_lib(), r_lib()] {
        assert_eq!(usz(lib, "crypto_hash_sha3256_bytes"), 32);
        assert_eq!(usz(lib, "crypto_hash_sha3512_bytes"), 64);
        assert_eq!(usz(lib, "crypto_hash_sha3256_statebytes"), 256);
        assert_eq!(usz(lib, "crypto_hash_sha3512_statebytes"), 256);
    }
    // G4-048/G4-078: only sha3256 / sha3512 exist.
    for missing in [
        "crypto_hash_sha3224",
        "crypto_hash_sha3384",
        "crypto_hash_sha3224_init",
        "crypto_hash_sha3384_init",
    ] {
        assert!(absent_in_both(missing), "{missing} must not exist");
    }
}

// ===========================================================================
// shorthash / onetimeauth / verify
// ===========================================================================

/// G4-080, G4-082, G4-083 — siphash has no validation whatsoever;
/// `in == NULL && inlen == 0` is safe and returns `0`.
#[test]
fn shorthash_has_no_error_paths() {
    setup();
    let mut rng = Rng::new(0xE420);
    for &(prefix, dl) in &[
        ("crypto_shorthash", 8usize),
        ("crypto_shorthash_siphash24", 8),
        ("crypto_shorthash_siphashx24", 16),
    ] {
        let (c, r) = pair::<Short>(prefix);
        let k = rng.bytes(16);
        let mut a = canary(dl);
        let mut b = canary(dl);
        let (ra, rb) = unsafe {
            (
                c(a.as_mut_ptr(), std::ptr::null(), 0, k.as_ptr()),
                r(b.as_mut_ptr(), std::ptr::null(), 0, k.as_ptr()),
            )
        };
        eq_i32(&format!("{prefix}(NULL,0) rc"), ra, rb);
        assert_eq!(ra, 0);
        eq_bytes(&format!("{prefix}(NULL,0)"), &a, &b);
        assert_ne!(a, canary(dl), "{prefix} must write {dl} bytes");

        // ... and never a nonzero return for any input shape
        for &len in &[0usize, 1, 7, 8, 9, 15, 16, 17, 100, 1000] {
            let inp = rng.bytes(len);
            let mut a = canary(dl);
            let mut b = canary(dl);
            let (ra, rb) = unsafe {
                (
                    c(a.as_mut_ptr(), inp.as_ptr(), len as u64, k.as_ptr()),
                    r(b.as_mut_ptr(), inp.as_ptr(), len as u64, k.as_ptr()),
                )
            };
            eq_i32(&format!("{prefix}(len={len}) rc"), ra, rb);
            assert_eq!(ra, 0);
            eq_bytes(&format!("{prefix}(len={len})"), &a, &b);
        }
    }
    for lib in [c_lib(), r_lib()] {
        assert_eq!(usz(lib, "crypto_shorthash_bytes"), 8);
        assert_eq!(usz(lib, "crypto_shorthash_keybytes"), 16);
        assert_eq!(usz(lib, "crypto_shorthash_siphash24_bytes"), 8);
        assert_eq!(usz(lib, "crypto_shorthash_siphash24_keybytes"), 16);
        assert_eq!(usz(lib, "crypto_shorthash_siphashx24_bytes"), 16);
        assert_eq!(usz(lib, "crypto_shorthash_siphashx24_keybytes"), 16);
        assert_eq!(cstr(lib, "crypto_shorthash_primitive"), "siphash24");
    }
}

/// G4-084, G4-085 (`_verify`), G4-086 … G4-090 (no rejection path anywhere in
/// poly1305, including `_update` after `_final` and a second `_final`).
#[test]
fn onetimeauth_error_paths() {
    setup();
    let mut rng = Rng::new(0xE421);
    for prefix in ["crypto_onetimeauth", "crypto_onetimeauth_poly1305"] {
        let (c1, r1) = pair::<OtaOneShot>(prefix);
        let (cv, rv) = pair::<OtaVerify>(&format!("{prefix}_verify"));
        let (ci, ri) = pair::<OtaInit>(&format!("{prefix}_init"));
        let (cu, ru) = pair::<StUpdate>(&format!("{prefix}_update"));
        let (cf, rf) = pair::<StFinal>(&format!("{prefix}_final"));
        let sb = format!("{prefix}_statebytes");

        // G4-086: in == NULL && inlen == 0
        let k = rng.bytes(32);
        let mut a = canary(16);
        let mut b = canary(16);
        let (ra, rb) = unsafe {
            (
                c1(a.as_mut_ptr(), std::ptr::null(), 0, k.as_ptr()),
                r1(b.as_mut_ptr(), std::ptr::null(), 0, k.as_ptr()),
            )
        };
        eq_i32(&format!("{prefix}(NULL,0) rc"), ra, rb);
        assert_eq!(ra, 0);
        eq_bytes(&format!("{prefix}(NULL,0)"), &a, &b);

        for &len in &[0usize, 1, 15, 16, 17, 31, 32, 33, 1000] {
            let k = rng.bytes(32);
            let inp = rng.bytes(len);
            let mut tag = canary(16);
            unsafe { c1(tag.as_mut_ptr(), inp.as_ptr(), len as u64, k.as_ptr()) };

            // G4-085: correct tag
            let (va, vb) = unsafe {
                (
                    cv(tag.as_ptr(), inp.as_ptr(), len as u64, k.as_ptr()),
                    rv(tag.as_ptr(), inp.as_ptr(), len as u64, k.as_ptr()),
                )
            };
            eq_i32(&format!("{prefix}_verify(ok,len={len})"), va, vb);
            assert_eq!(va, 0);

            // G4-084: every single-byte corruption, plus multi-byte ones
            for i in 0..16usize {
                for &mask in &[1u8, 0x80, 0xff] {
                    let mut bad = tag.clone();
                    bad[i] ^= mask;
                    let (va, vb) = unsafe {
                        (
                            cv(bad.as_ptr(), inp.as_ptr(), len as u64, k.as_ptr()),
                            rv(bad.as_ptr(), inp.as_ptr(), len as u64, k.as_ptr()),
                        )
                    };
                    eq_i32(&format!("{prefix}_verify(bad[{i}]^{mask:#x},len={len})"), va, vb);
                    assert_eq!(va, -1, "{prefix}_verify must return exactly -1");
                }
            }
            for bad in [vec![0u8; 16], vec![0xffu8; 16], rng.bytes(16)] {
                if bad == tag {
                    continue;
                }
                let (va, vb) = unsafe {
                    (
                        cv(bad.as_ptr(), inp.as_ptr(), len as u64, k.as_ptr()),
                        rv(bad.as_ptr(), inp.as_ptr(), len as u64, k.as_ptr()),
                    )
                };
                eq_i32(&format!("{prefix}_verify(wholly wrong,len={len})"), va, vb);
                assert_eq!(va, -1);
            }

            // G4-087 / G4-088 / G4-089: streaming has no rejection path.
            let extra = rng.bytes(37);
            let mut t1 = [canary(16), canary(16)];
            let mut t2 = [canary(16), canary(16)];
            let mut t3 = [canary(16), canary(16)];
            let mut rcs = [[9i32; 6]; 2];
            for (which, (init, upd, fin)) in
                [(ci, cu, cf), (ri, ru, rf)].into_iter().enumerate()
            {
                let mut st = State::for_sym(&sb);
                unsafe {
                    rcs[which][0] = init(st.as_mut_ptr(), k.as_ptr());
                    rcs[which][1] = upd(st.as_mut_ptr(), std::ptr::null(), 0);
                    rcs[which][2] = upd(st.as_mut_ptr(), inp.as_ptr(), len as u64);
                    rcs[which][3] = fin(st.as_mut_ptr(), t1[which].as_mut_ptr());
                    rcs[which][4] = upd(st.as_mut_ptr(), extra.as_ptr(), 37);
                    rcs[which][5] = fin(st.as_mut_ptr(), t2[which].as_mut_ptr());
                    // and a plain second `_final`
                    let mut st2 = State::for_sym(&sb);
                    init(st2.as_mut_ptr(), k.as_ptr());
                    upd(st2.as_mut_ptr(), inp.as_ptr(), len as u64);
                    let mut tmp = canary(16);
                    fin(st2.as_mut_ptr(), tmp.as_mut_ptr());
                    fin(st2.as_mut_ptr(), t3[which].as_mut_ptr());
                }
            }
            for k2 in 0..6 {
                eq_i32(&format!("{prefix} stream rc[{k2}] len={len}"), rcs[0][k2], rcs[1][k2]);
                assert_eq!(rcs[0][k2], 0, "{prefix} stream rc[{k2}] must be 0");
            }
            let (x, y) = (t1[0].clone(), t1[1].clone());
            eq_bytes(&format!("{prefix} stream tag len={len}"), &x, &y);
            eq_bytes(&format!("{prefix} stream tag == one-shot"), &tag, &x);
            let (x, y) = (t2[0].clone(), t2[1].clone());
            eq_bytes(&format!("{prefix} post-final tag"), &x, &y);
            let (x, y) = (t3[0].clone(), t3[1].clone());
            eq_bytes(&format!("{prefix} second-final tag"), &x, &y);
        }
    }
    // G4-090 constants
    for lib in [c_lib(), r_lib()] {
        assert_eq!(usz(lib, "crypto_onetimeauth_bytes"), 16);
        assert_eq!(usz(lib, "crypto_onetimeauth_keybytes"), 32);
        assert_eq!(usz(lib, "crypto_onetimeauth_statebytes"), 256);
        assert_eq!(usz(lib, "crypto_onetimeauth_poly1305_bytes"), 16);
        assert_eq!(usz(lib, "crypto_onetimeauth_poly1305_keybytes"), 32);
        assert_eq!(usz(lib, "crypto_onetimeauth_poly1305_statebytes"), 256);
        assert_eq!(cstr(lib, "crypto_onetimeauth_primitive"), "poly1305");
    }
}

/// G4-091, G4-092, G4-093, G4-094, G4-095 — `crypto_verify_16/32/64`:
/// exactly `0` on equality and exactly `-1` for every possible difference.
#[test]
fn crypto_verify_rows() {
    setup();
    let mut rng = Rng::new(0xE422);
    for &n in &[16usize, 32, 64] {
        let (c, r) = pair::<Verify>(&format!("crypto_verify_{n}"));
        assert_eq!(usz(c_lib(), &format!("crypto_verify_{n}_bytes")), n);
        assert_eq!(usz(r_lib(), &format!("crypto_verify_{n}_bytes")), n);

        for kind in 0..3 {
            let x: Vec<u8> = match kind {
                0 => rng.bytes(n),
                1 => vec![0u8; n],
                _ => vec![0xffu8; n],
            };
            // equal (G4-092) and aliased (CONFIGS G4-118)
            let (ra, rb) = unsafe { (c(x.as_ptr(), x.as_ptr()), r(x.as_ptr(), x.as_ptr())) };
            eq_i32(&format!("crypto_verify_{n} aliased"), ra, rb);
            assert_eq!(ra, 0);
            let y = x.clone();
            let (ra, rb) = unsafe { (c(x.as_ptr(), y.as_ptr()), r(x.as_ptr(), y.as_ptr())) };
            eq_i32(&format!("crypto_verify_{n} equal"), ra, rb);
            assert_eq!(ra, 0);

            // every byte position x every single-bit difference
            for i in 0..n {
                for bit in 0..8u32 {
                    let mut z = x.clone();
                    z[i] ^= 1u8 << bit;
                    let (ra, rb) = unsafe { (c(x.as_ptr(), z.as_ptr()), r(x.as_ptr(), z.as_ptr())) };
                    eq_i32(&format!("crypto_verify_{n} diff byte {i} bit {bit}"), ra, rb);
                    assert_eq!(ra, -1, "crypto_verify_{n} must return exactly -1");
                    let (ra, rb) = unsafe { (c(z.as_ptr(), x.as_ptr()), r(z.as_ptr(), x.as_ptr())) };
                    eq_i32(&format!("crypto_verify_{n} diff (swapped) {i}/{bit}"), ra, rb);
                    assert_eq!(ra, -1);
                }
            }
            // all bytes different
            let z: Vec<u8> = x.iter().map(|b| !b).collect();
            let (ra, rb) = unsafe { (c(x.as_ptr(), z.as_ptr()), r(x.as_ptr(), z.as_ptr())) };
            eq_i32(&format!("crypto_verify_{n} all differ"), ra, rb);
            assert_eq!(ra, -1);
        }
    }
}

// ===========================================================================
// keccak1600 (NOT owned by this group — read-only assertions only)
// ===========================================================================

/// G4-096, G4-099, G4-100, G4-101 — the keccak1600 surface has no failure mode
/// at all: `_init`/`_permute_*` are `void`, `_statebytes` is 224 and there is
/// no `_absorb` / `_pad` / rate-taking `_init`.
#[test]
fn keccak1600_has_no_error_surface() {
    setup();
    assert_eq!(usz(c_lib(), "crypto_core_keccak1600_statebytes"), 224);
    assert_eq!(usz(r_lib(), "crypto_core_keccak1600_statebytes"), 224);
    for missing in [
        "crypto_core_keccak1600_absorb",
        "crypto_core_keccak1600_pad",
        "crypto_core_keccak1600_squeeze",
        "crypto_core_keccak1600_init_with_rate",
        "crypto_core_keccak1600_rate",
    ] {
        assert!(absent_in_both(missing), "{missing} must not exist");
    }
    // `_init` cannot fail: it zeroes the first 200 bytes and leaves the 24
    // trailing padding bytes of the 224-byte struct alone.
    type KcInit = unsafe extern "C" fn(*mut u8);
    type KcExtract = unsafe extern "C" fn(*const u8, *mut u8, usize, usize);
    let (ci, ri) = pair::<KcInit>("crypto_core_keccak1600_init");
    let (ce, re) = pair::<KcExtract>("crypto_core_keccak1600_extract_bytes");
    let mut out = [canary(200), canary(200)];
    let mut tails = [[0u8; 24], [0u8; 24]];
    for (which, (init, ext)) in [(ci, ce), (ri, re)].into_iter().enumerate() {
        let mut st = State::new(224);
        unsafe {
            std::ptr::write_bytes(st.as_mut_ptr(), 0x5A, 224);
            init(st.as_mut_ptr());
            ext(st.as_ptr(), out[which].as_mut_ptr(), 0, 200);
            std::ptr::copy_nonoverlapping(st.as_ptr().add(200), tails[which].as_mut_ptr(), 24);
        }
    }
    let (a, b) = (out[0].clone(), out[1].clone());
    eq_bytes("keccak1600_init zeroes 200 bytes", &a, &b);
    assert_eq!(a, vec![0u8; 200]);
    eq_bytes("keccak1600_init leaves the tail alone", &tails[0], &tails[1]);
    assert_eq!(tails[0], [0x5Au8; 24]);
}

// ===========================================================================
// XOFs
// ===========================================================================

/// (prefix, rate)
const XOFS: &[(&str, usize)] = &[
    ("crypto_xof_shake128", 168),
    ("crypto_xof_shake256", 136),
    ("crypto_xof_turboshake128", 168),
    ("crypto_xof_turboshake256", 136),
];

/// G4-102, G4-103, G4-104, G4-105 — `_update` after `_squeeze` returns `-1`
/// and yet *recovers*: one permutation, `phase = ABSORBING`, `offset = 0`, and
/// the new input IS absorbed. Every downstream byte must match.
#[test]
fn xof_update_after_squeeze() {
    setup();
    let mut rng = Rng::new(0xE430);
    for &(prefix, rate) in XOFS {
        let (ci, ri) = pair::<StInit>(&format!("{prefix}_init"));
        let (cu, ru) = pair::<StUpdate>(&format!("{prefix}_update"));
        let (cs, rs) = pair::<XofSqueeze>(&format!("{prefix}_squeeze"));
        let sb = format!("{prefix}_statebytes");
        for &inlen in &[0usize, 1, rate - 1, rate, rate + 1, 2 * rate] {
            for &sq1 in &[0usize, 1, 32, rate - 1, rate, rate + 1] {
                for &extra in &[0usize, 1, rate, rate + 5] {
                    let inp = rng.bytes(inlen);
                    let more = rng.bytes(extra.max(1));
                    let out2len = rate + 41;
                    let mut o1 = [canary(sq1.max(1)), canary(sq1.max(1))];
                    let mut o2 = [canary(out2len), canary(out2len)];
                    let mut rcs = [[9i32; 4]; 2];
                    for (which, (init, upd, sqz)) in
                        [(ci, cu, cs), (ri, ru, rs)].into_iter().enumerate()
                    {
                        let mut st = State::for_sym(&sb);
                        unsafe {
                            assert_eq!(init(st.as_mut_ptr()), 0);
                            rcs[which][0] = upd(st.as_mut_ptr(), inp.as_ptr(), inlen as u64);
                            rcs[which][1] = sqz(st.as_mut_ptr(), o1[which].as_mut_ptr(), sq1);
                            rcs[which][2] = upd(st.as_mut_ptr(), more.as_ptr(), extra as u64);
                            rcs[which][3] = sqz(st.as_mut_ptr(), o2[which].as_mut_ptr(), out2len);
                        }
                    }
                    let what = format!("{prefix}(in={inlen},sq1={sq1},extra={extra})");
                    for k in 0..4 {
                        eq_i32(&format!("{what} rc[{k}]"), rcs[0][k], rcs[1][k]);
                    }
                    assert_eq!(rcs[0][0], 0, "{what}: first _update -> 0");
                    assert_eq!(rcs[0][1], 0, "{what}: _squeeze always -> 0");
                    assert_eq!(rcs[0][2], -1, "{what}: _update after _squeeze -> -1");
                    assert_eq!(rcs[0][3], 0, "{what}: _squeeze after recovery -> 0");
                    let (x, y) = (o1[0].clone(), o1[1].clone());
                    eq_bytes(&format!("{what} squeeze1"), &x, &y);
                    let (x, y) = (o2[0].clone(), o2[1].clone());
                    eq_bytes(&format!("{what} squeeze2 after recovering update"), &x, &y);
                }
            }
        }
    }
}

/// G4-106 — `_squeeze` never rejects: `outlen == 0`, repeated calls and a
/// squeeze on a freshly `_init`ed state (no `_update` at all) all return `0`.
#[test]
fn xof_squeeze_never_rejects() {
    setup();
    let mut rng = Rng::new(0xE431);
    for &(prefix, rate) in XOFS {
        let (ci, ri) = pair::<StInit>(&format!("{prefix}_init"));
        let (cs, rs) = pair::<XofSqueeze>(&format!("{prefix}_squeeze"));
        let (cu, ru) = pair::<StUpdate>(&format!("{prefix}_update"));
        let sb = format!("{prefix}_statebytes");

        // squeeze(0) on a fresh state still finalises; a later squeeze
        // continues from offset 0.
        for &prime in &[0usize, 1, rate] {
            let inp = rng.bytes(prime.max(1));
            let total = 3 * rate + 7;
            let mut out = [canary(total), canary(total)];
            let mut rcs = [[9i32; 8]; 2];
            for (which, (init, upd, sqz)) in
                [(ci, cu, cs), (ri, ru, rs)].into_iter().enumerate()
            {
                let mut st = State::for_sym(&sb);
                unsafe {
                    init(st.as_mut_ptr());
                    if prime > 0 {
                        upd(st.as_mut_ptr(), inp.as_ptr(), prime as u64);
                    }
                    let mut z = canary(1);
                    // three zero-length squeezes in a row
                    rcs[which][0] = sqz(st.as_mut_ptr(), z.as_mut_ptr(), 0);
                    rcs[which][1] = sqz(st.as_mut_ptr(), z.as_mut_ptr(), 0);
                    rcs[which][2] = sqz(st.as_mut_ptr(), std::ptr::null_mut(), 0);
                    assert_eq!(z, canary(1), "{prefix}: squeeze(0) wrote a byte");
                    // then a chain of real squeezes
                    let mut off = 0usize;
                    let mut i = 3;
                    for &n in &[1usize, 31, rate, rate + 5] {
                        rcs[which][i] = sqz(st.as_mut_ptr(), out[which][off..].as_mut_ptr(), n);
                        off += n;
                        i += 1;
                    }
                    let rest = total - off;
                    rcs[which][7] = sqz(st.as_mut_ptr(), out[which][off..].as_mut_ptr(), rest);
                }
            }
            for k in 0..8 {
                eq_i32(&format!("{prefix} squeeze rc[{k}] prime={prime}"), rcs[0][k], rcs[1][k]);
                assert_eq!(rcs[0][k], 0, "{prefix} _squeeze must always return 0");
            }
            let (x, y) = (out[0].clone(), out[1].clone());
            eq_bytes(&format!("{prefix} chained squeezes prime={prime}"), &x, &y);
        }
    }
}

/// G4-107, G4-108, G4-109, G4-112 — `_init_with_domain` has **no** validation:
/// all 256 byte values return `0`; `_init` == `_init_with_domain(0x1F)`;
/// `blockbytes` / `statebytes` / `domain_standard` never fail.
#[test]
fn xof_init_with_domain_accepts_every_byte() {
    setup();
    let mut rng = Rng::new(0xE432);
    for &(prefix, rate) in XOFS {
        for lib in [c_lib(), r_lib()] {
            assert_eq!(usz(lib, &format!("{prefix}_blockbytes")), rate);
            assert_eq!(usz(lib, &format!("{prefix}_statebytes")), 256);
            assert_eq!(unsafe { sym::<U8Fn>(lib, &format!("{prefix}_domain_standard"))() }, 0x1f);
        }
        let (cid, rid) = pair::<XofInitDomain>(&format!("{prefix}_init_with_domain"));
        let (ci, ri) = pair::<StInit>(&format!("{prefix}_init"));
        let (cu, ru) = pair::<StUpdate>(&format!("{prefix}_update"));
        let (cs, rs) = pair::<XofSqueeze>(&format!("{prefix}_squeeze"));
        let sb = format!("{prefix}_statebytes");

        let inp = rng.bytes(2 * rate + 5);
        let outlen = rate + 9;
        let mut standard = canary(outlen);
        let mut standard_inlen = 0usize;
        for dom in 0u16..=255 {
            let dom = dom as u8;
            let inlen = *rng.pick(&[0usize, 1, rate - 1, rate, rate + 1, 2 * rate + 5]);
            let mut out = [canary(outlen), canary(outlen)];
            let mut rcs = [9i32; 2];
            for (which, (init, upd, sqz)) in
                [(cid, cu, cs), (rid, ru, rs)].into_iter().enumerate()
            {
                let mut st = State::for_sym(&sb);
                unsafe {
                    rcs[which] = init(st.as_mut_ptr(), dom);
                    upd(st.as_mut_ptr(), inp.as_ptr(), inlen as u64);
                    sqz(st.as_mut_ptr(), out[which].as_mut_ptr(), outlen);
                }
            }
            eq_i32(&format!("{prefix}_init_with_domain({dom:#04x}) rc"), rcs[0], rcs[1]);
            assert_eq!(
                rcs[0], 0,
                "{prefix}_init_with_domain({dom:#04x}) must be accepted (no range check exists)"
            );
            let (x, y) = (out[0].clone(), out[1].clone());
            eq_bytes(&format!("{prefix} domain={dom:#04x}"), &x, &y);
            if dom == 0x1f {
                standard = x;
                standard_inlen = inlen;
            }
        }
        // `_init` must be exactly `_init_with_domain(DOMAIN_STANDARD)`.
        let inlen = standard_inlen;
        let mut out = [canary(outlen), canary(outlen)];
        let mut rcs = [9i32; 2];
        for (which, (init, upd, sqz)) in [(ci, cu, cs), (ri, ru, rs)].into_iter().enumerate() {
            let mut st = State::for_sym(&sb);
            unsafe {
                rcs[which] = init(st.as_mut_ptr());
                upd(st.as_mut_ptr(), inp.as_ptr(), inlen as u64);
                sqz(st.as_mut_ptr(), out[which].as_mut_ptr(), outlen);
            }
        }
        eq_i32(&format!("{prefix}_init rc"), rcs[0], rcs[1]);
        assert_eq!(rcs[0], 0);
        let (x, y) = (out[0].clone(), out[1].clone());
        eq_bytes(&format!("{prefix}_init"), &x, &y);
        eq_bytes(&format!("{prefix}_init == _init_with_domain(0x1f)"), &standard, &x);
    }
}

/// G4-110, G4-111 — the one-shot XOF never rejects (`outlen == 0`,
/// `in == NULL && inlen == 0`, very large `outlen`) and `_final` / `_absorb` /
/// `_clone` are not part of the exported surface.
#[test]
fn xof_one_shot_never_rejects_and_missing_entry_points() {
    setup();
    let mut rng = Rng::new(0xE433);
    for &(prefix, rate) in XOFS {
        let (c, r) = pair::<XofOneShot>(prefix);
        // outlen == 0 must not write anything
        let inp = rng.bytes(rate + 3);
        let mut a = canary(8);
        let mut b = canary(8);
        let (ra, rb) = unsafe {
            (
                c(a.as_mut_ptr(), 0, inp.as_ptr(), (rate + 3) as u64),
                r(b.as_mut_ptr(), 0, inp.as_ptr(), (rate + 3) as u64),
            )
        };
        eq_i32(&format!("{prefix}(outlen=0) rc"), ra, rb);
        assert_eq!(ra, 0);
        eq_bytes(&format!("{prefix}(outlen=0)"), &a, &b);
        assert_eq!(a, canary(8));

        // in == NULL && inlen == 0
        let mut a = canary(64);
        let mut b = canary(64);
        let (ra, rb) = unsafe {
            (
                c(a.as_mut_ptr(), 64, std::ptr::null(), 0),
                r(b.as_mut_ptr(), 64, std::ptr::null(), 0),
            )
        };
        eq_i32(&format!("{prefix}(NULL,0) rc"), ra, rb);
        assert_eq!(ra, 0);
        eq_bytes(&format!("{prefix}(NULL,0)"), &a, &b);

        // no output-length limit whatsoever
        let big = 200_000usize;
        let mut a = canary(big);
        let mut b = canary(big);
        let (ra, rb) = unsafe {
            (
                c(a.as_mut_ptr(), big, inp.as_ptr(), (rate + 3) as u64),
                r(b.as_mut_ptr(), big, inp.as_ptr(), (rate + 3) as u64),
            )
        };
        eq_i32(&format!("{prefix}(outlen={big}) rc"), ra, rb);
        assert_eq!(ra, 0, "{prefix} has no BYTES_MAX");
        eq_bytes(&format!("{prefix}(outlen={big})"), &a, &b);

        for suffix in ["_final", "_absorb", "_clone", "_bytes_max", "_keygen"] {
            let name = format!("{prefix}{suffix}");
            assert!(absent_in_both(&name), "{name} must not exist");
        }
    }
}

// ===========================================================================
// KDFs
// ===========================================================================

/// G4-113, G4-114, G4-118, G4-116, G4-117, G4-119 —
/// `crypto_kdf[_blake2b]_derive_from_key` rejects `subkey_len` outside
/// `[BYTES_MIN, BYTES_MAX] == [16, 64]` with `errno = EINVAL` and `-1`,
/// and accepts every value inside it.
#[test]
fn kdf_derive_from_key_subkey_len_rejected() {
    setup();
    let mut rng = Rng::new(0xE440);
    for prefix in ["crypto_kdf_blake2b", "crypto_kdf"] {
        for lib in [c_lib(), r_lib()] {
            assert_eq!(usz(lib, &format!("{prefix}_bytes_min")), 16);
            assert_eq!(usz(lib, &format!("{prefix}_bytes_max")), 64);
            assert_eq!(usz(lib, &format!("{prefix}_contextbytes")), 8);
            assert_eq!(usz(lib, &format!("{prefix}_keybytes")), 32);
        }
        let (c, r) = pair::<KdfDerive>(&format!("{prefix}_derive_from_key"));
        let key = rng.bytes(32);
        let ctx = rng.bytes(8);

        for &sklen in &[0usize, 1, 2, 8, 15, 65, 66, 100, 255, 256, 1000, usize::MAX] {
            for &id in &[0u64, 1, 0xffff_ffff, u64::MAX] {
                let mut a = canary(80);
                let mut b = canary(80);
                clear_errno();
                let ra = unsafe {
                    c(a.as_mut_ptr(), sklen, id, ctx.as_ptr() as *const c_char, key.as_ptr())
                };
                let ea = errno();
                clear_errno();
                let rb = unsafe {
                    r(b.as_mut_ptr(), sklen, id, ctx.as_ptr() as *const c_char, key.as_ptr())
                };
                let eb = errno();
                eq_i32(&format!("{prefix}_derive_from_key(len={sklen}) rc"), ra, rb);
                assert_eq!(ra, -1, "{prefix} must reject subkey_len={sklen}");
                eq_i32(&format!("{prefix}_derive_from_key(len={sklen}) errno"), ea, eb);
                assert_eq!(ea, EINVAL, "{prefix} must set errno = EINVAL");
                eq_bytes(&format!("{prefix}_derive_from_key(len={sklen}) out"), &a, &b);
                assert_eq!(a, canary(80), "{prefix} wrote to subkey on rejection");
            }
        }
        // every accepted length succeeds and matches
        for sklen in 16usize..=64 {
            let mut a = canary(sklen);
            let mut b = canary(sklen);
            let (ra, rb) = unsafe {
                (
                    c(a.as_mut_ptr(), sklen, 7, ctx.as_ptr() as *const c_char, key.as_ptr()),
                    r(b.as_mut_ptr(), sklen, 7, ctx.as_ptr() as *const c_char, key.as_ptr()),
                )
            };
            eq_i32(&format!("{prefix}_derive_from_key(len={sklen}) rc"), ra, rb);
            assert_eq!(ra, 0);
            eq_bytes(&format!("{prefix}_derive_from_key(len={sklen})"), &a, &b);
        }
    }
    for lib in [c_lib(), r_lib()] {
        assert_eq!(cstr(lib, "crypto_kdf_primitive"), "blake2b");
    }
    // G4-116: there is no `ctx_len` parameter anywhere, so no context-length
    // rejection branch can exist.
    assert!(absent_in_both("crypto_kdf_derive_from_key_with_ctx_len"));
}

/// G4-120, G4-121, G4-122, G4-123 — `_expand` rejects `out_len > BYTES_MAX`
/// with `errno = EINVAL`; `out_len == 0` (== `BYTES_MIN`) and
/// `ctx == NULL, ctx_len == 0` are accepted.
#[test]
fn hkdf_expand_out_len_rejected() {
    setup();
    let mut rng = Rng::new(0xE441);
    for &(prefix, kb, bmax) in &[
        ("crypto_kdf_hkdf_sha256", 32usize, 8160usize),
        ("crypto_kdf_hkdf_sha512", 64, 16320),
    ] {
        for lib in [c_lib(), r_lib()] {
            assert_eq!(usz(lib, &format!("{prefix}_keybytes")), kb);
            assert_eq!(usz(lib, &format!("{prefix}_bytes_min")), 0);
            assert_eq!(usz(lib, &format!("{prefix}_bytes_max")), bmax);
            assert_eq!(usz(lib, &format!("{prefix}_statebytes")), if kb == 32 { 208 } else { 416 });
        }
        let (c, r) = pair::<HkdfExpand>(&format!("{prefix}_expand"));
        let prk = rng.bytes(kb);
        let ctx = rng.bytes(8);

        for &outlen in &[bmax + 1, bmax + 2, bmax * 2, usize::MAX, usize::MAX - 1] {
            let mut a = canary(128);
            let mut b = canary(128);
            clear_errno();
            let ra = unsafe {
                c(a.as_mut_ptr(), outlen, ctx.as_ptr() as *const c_char, 8, prk.as_ptr())
            };
            let ea = errno();
            clear_errno();
            let rb = unsafe {
                r(b.as_mut_ptr(), outlen, ctx.as_ptr() as *const c_char, 8, prk.as_ptr())
            };
            let eb = errno();
            eq_i32(&format!("{prefix}_expand(out_len={outlen}) rc"), ra, rb);
            assert_eq!(ra, -1, "{prefix}_expand must reject out_len={outlen}");
            eq_i32(&format!("{prefix}_expand(out_len={outlen}) errno"), ea, eb);
            assert_eq!(ea, EINVAL);
            eq_bytes(&format!("{prefix}_expand(out_len={outlen}) out"), &a, &b);
            assert_eq!(a, canary(128), "{prefix}_expand wrote on rejection");
        }

        // G4-122: out_len == 0 -> 0, nothing written. G4-123: ctx = NULL/0.
        for &(cp, ctxlen, what) in &[
            (ctx.as_ptr() as *const c_char, 8usize, "ctx=8"),
            (std::ptr::null(), 0usize, "ctx=NULL/0"),
        ] {
            let mut a = canary(64);
            let mut b = canary(64);
            let (ra, rb) = unsafe {
                (
                    c(a.as_mut_ptr(), 0, cp, ctxlen, prk.as_ptr()),
                    r(b.as_mut_ptr(), 0, cp, ctxlen, prk.as_ptr()),
                )
            };
            eq_i32(&format!("{prefix}_expand(out_len=0,{what}) rc"), ra, rb);
            assert_eq!(ra, 0);
            eq_bytes(&format!("{prefix}_expand(out_len=0,{what})"), &a, &b);
            assert_eq!(a, canary(64), "out_len == 0 must write nothing");

            // and a real output with ctx = NULL / ctx_len = 0
            let mut a = canary(bmax.min(200));
            let mut b = canary(bmax.min(200));
            let n = bmax.min(200);
            let (ra, rb) = unsafe {
                (
                    c(a.as_mut_ptr(), n, cp, ctxlen, prk.as_ptr()),
                    r(b.as_mut_ptr(), n, cp, ctxlen, prk.as_ptr()),
                )
            };
            eq_i32(&format!("{prefix}_expand({what}) rc"), ra, rb);
            assert_eq!(ra, 0);
            eq_bytes(&format!("{prefix}_expand({what})"), &a, &b);
        }
        // exactly BYTES_MAX is still accepted
        let mut a = canary(bmax);
        let mut b = canary(bmax);
        let (ra, rb) = unsafe {
            (
                c(a.as_mut_ptr(), bmax, ctx.as_ptr() as *const c_char, 8, prk.as_ptr()),
                r(b.as_mut_ptr(), bmax, ctx.as_ptr() as *const c_char, 8, prk.as_ptr()),
            )
        };
        eq_i32(&format!("{prefix}_expand(out_len=BYTES_MAX) rc"), ra, rb);
        assert_eq!(ra, 0);
        eq_bytes(&format!("{prefix}_expand(out_len=BYTES_MAX)"), &a, &b);
    }
}

/// G4-128, G4-129, G4-130, G4-131, G4-132, G4-133 — the non-abort hkdf rows:
/// `salt == NULL && salt_len == 0` is the RFC-5869 default salt, an over-long
/// salt is silently pre-hashed, and `_extract_update` / `_extract_final` have
/// no rejection path at all (not even after `_extract_final`).
#[test]
fn hkdf_extract_no_rejection_rows() {
    setup();
    let mut rng = Rng::new(0xE442);
    for &(prefix, kb, block) in &[
        ("crypto_kdf_hkdf_sha256", 32usize, 64usize),
        ("crypto_kdf_hkdf_sha512", 64, 128),
    ] {
        let (ce, re) = pair::<HkdfExtract>(&format!("{prefix}_extract"));
        let (ci, ri) = pair::<HkdfExInit>(&format!("{prefix}_extract_init"));
        let (cu, ru) = pair::<HkdfExUpdate>(&format!("{prefix}_extract_update"));
        let (cf, rf) = pair::<HkdfExFinal>(&format!("{prefix}_extract_final"));
        let sb = format!("{prefix}_statebytes");

        // G4-128: salt == NULL && salt_len == 0
        for &ikmlen in &[0usize, 1, 32, 100] {
            let ikm = rng.bytes(ikmlen.max(1));
            let ip = if ikmlen == 0 {
                std::ptr::null()
            } else {
                ikm.as_ptr()
            };
            let mut a = canary(kb);
            let mut b = canary(kb);
            let (ra, rb) = unsafe {
                (
                    ce(a.as_mut_ptr(), std::ptr::null(), 0, ip, ikmlen),
                    re(b.as_mut_ptr(), std::ptr::null(), 0, ip, ikmlen),
                )
            };
            eq_i32(&format!("{prefix}_extract(salt=NULL/0,ikm={ikmlen}) rc"), ra, rb);
            assert_eq!(ra, 0);
            eq_bytes(&format!("{prefix}_extract(salt=NULL/0,ikm={ikmlen})"), &a, &b);

            // same through the streaming trio
            let mut out = [canary(kb), canary(kb)];
            let mut rcs = [[9i32; 3]; 2];
            for (which, (init, upd, fin)) in [(ci, cu, cf), (ri, ru, rf)].into_iter().enumerate() {
                let mut st = State::for_sym(&sb);
                unsafe {
                    rcs[which][0] = init(st.as_mut_ptr(), std::ptr::null(), 0);
                    rcs[which][1] = upd(st.as_mut_ptr(), ip, ikmlen);
                    rcs[which][2] = fin(st.as_mut_ptr(), out[which].as_mut_ptr());
                }
            }
            for k in 0..3 {
                eq_i32(&format!("{prefix} stream rc[{k}]"), rcs[0][k], rcs[1][k]);
                assert_eq!(rcs[0][k], 0);
            }
            let (x, y) = (out[0].clone(), out[1].clone());
            eq_bytes(&format!("{prefix}_extract streaming(salt=NULL/0)"), &x, &y);
            eq_bytes(&format!("{prefix}_extract streaming == one-shot"), &a, &x);
        }

        // G4-129 / G4-130: salt_len above the HMAC block size is pre-hashed.
        for &saltlen in &[block - 1, block, block + 1, block + 2, 200, 1000] {
            let salt = rng.bytes(saltlen);
            let ikm = rng.bytes(64);
            let mut a = canary(kb);
            let mut b = canary(kb);
            let (ra, rb) = unsafe {
                (
                    ce(a.as_mut_ptr(), salt.as_ptr(), saltlen, ikm.as_ptr(), 64),
                    re(b.as_mut_ptr(), salt.as_ptr(), saltlen, ikm.as_ptr(), 64),
                )
            };
            eq_i32(&format!("{prefix}_extract(salt={saltlen}) rc"), ra, rb);
            assert_eq!(ra, 0, "{prefix}_extract must accept salt_len={saltlen}");
            eq_bytes(&format!("{prefix}_extract(salt={saltlen})"), &a, &b);
            // pre-hashing is observable: salt_len > block == HASH(salt)
            if saltlen > block {
                let hprefix = if kb == 32 { "crypto_hash_sha256" } else { "crypto_hash_sha512" };
                let (hc, _) = pair::<HashOneShot>(hprefix);
                let mut hs = canary(kb);
                unsafe { hc(hs.as_mut_ptr(), salt.as_ptr(), saltlen as u64) };
                let mut expect = canary(kb);
                unsafe { ce(expect.as_mut_ptr(), hs.as_ptr(), kb, ikm.as_ptr(), 64) };
                eq_bytes(&format!("{prefix}: long salt == HASH(salt)"), &expect, &a);
            }
        }

        // G4-131 / G4-132: post-`_extract_final` update / a second final.
        let salt = rng.bytes(32);
        let ikm = rng.bytes(100);
        let mut p1 = [canary(kb), canary(kb)];
        let mut p2 = [canary(kb), canary(kb)];
        let mut rcs = [[9i32; 5]; 2];
        for (which, (init, upd, fin)) in [(ci, cu, cf), (ri, ru, rf)].into_iter().enumerate() {
            let mut st = State::for_sym(&sb);
            unsafe {
                init(st.as_mut_ptr(), salt.as_ptr(), 32);
                rcs[which][0] = upd(st.as_mut_ptr(), std::ptr::null(), 0);
                rcs[which][1] = upd(st.as_mut_ptr(), ikm.as_ptr(), 100);
                rcs[which][2] = fin(st.as_mut_ptr(), p1[which].as_mut_ptr());
                rcs[which][3] = upd(st.as_mut_ptr(), ikm.as_ptr(), 100);
                rcs[which][4] = fin(st.as_mut_ptr(), p2[which].as_mut_ptr());
            }
        }
        for k in 0..5 {
            eq_i32(&format!("{prefix} post-final rc[{k}]"), rcs[0][k], rcs[1][k]);
            assert_eq!(rcs[0][k], 0, "{prefix} hkdf has no rejection path here");
        }
        let (x, y) = (p1[0].clone(), p1[1].clone());
        eq_bytes(&format!("{prefix} prk"), &x, &y);
        let (x, y) = (p2[0].clone(), p2[1].clone());
        eq_bytes(&format!("{prefix} prk after post-final update"), &x, &y);

        // G4-133: keygen is `void` and cannot fail.
        type Keygen = unsafe extern "C" fn(*mut u8);
        let (ck, rk) = pair::<Keygen>(&format!("{prefix}_keygen"));
        for seed in 0..4u64 {
            let mut a = canary(kb);
            let mut b = canary(kb);
            reset_rngs(0xE44_2000 + seed);
            unsafe { ck(a.as_mut_ptr()) };
            reset_rngs(0xE44_2000 + seed);
            unsafe { rk(b.as_mut_ptr()) };
            eq_bytes(&format!("{prefix}_keygen"), &a, &b);
        }
    }
}

// ===========================================================================
// abort paths — run out of process
// ===========================================================================

/// Rows that reach `sodium_misuse()`: the observing handler prints
/// `MISUSE obs=<hex>` and exits with `MISUSE_EXIT`.
const MISUSE_CASES: &[&str] = &[
    // crypto_generichash_blake2b (one-shot)                      G4-007/008/009
    "gh_b2/in_null_5",
    "gh_b2/in_null_1",
    "gh_b2/out_null",
    "gh_b2/key_null_5",
    "gh_b2/key_null_64",
    // crypto_generichash (dispatcher)                            G4-016/017/018
    "gh_g/in_null_5",
    "gh_g/out_null",
    "gh_g/key_null_5",
    // crypto_generichash_blake2b_salt_personal                   G4-026/027/028
    "gh_sp/in_null_5",
    "gh_sp/out_null",
    "gh_sp/key_null_5",
    // _final with outlen == 0 and 65..=255                        G4-058/059
    "final_b2/outlen_0",
    "final_b2/outlen_65",
    "final_b2/outlen_100",
    "final_b2/outlen_255",
    "final_g/outlen_0",
    "final_g/outlen_65",
    "final_g/outlen_255",
    // the internal blake2b_init* family                    G4-037/038/039/040/042/043
    "b2_init/outlen_0",
    "b2_init/outlen_65",
    "b2_init/outlen_255",
    "b2_init_sp/outlen_0",
    "b2_init_sp/outlen_255",
    "b2_init_key/outlen_0",
    "b2_init_key/outlen_255",
    "b2_init_key/key_null",
    "b2_init_key/keylen_0",
    "b2_init_key/keylen_65",
    "b2_init_key/keylen_255",
    "b2_init_key_sp/outlen_0",
    "b2_init_key_sp/outlen_255",
    "b2_init_key_sp/key_null",
    "b2_init_key_sp/keylen_0",
    "b2_init_key_sp/keylen_65",
    // hkdf: the only abort path in the module                    G4-124..G4-127
    "hkdf256/extract_init/salt_null_1",
    "hkdf256/extract_init/salt_null_5",
    "hkdf256/extract_init/salt_null_64",
    "hkdf512/extract_init/salt_null_1",
    "hkdf512/extract_init/salt_null_5",
    "hkdf512/extract_init/salt_null_128",
    "hkdf256/extract/salt_null_5",
    "hkdf256/extract/salt_null_64",
    "hkdf512/extract/salt_null_5",
    "hkdf512/extract/salt_null_128",
];

/// Rows that die on a raw `assert()` — the misuse handler is NOT involved, so
/// the child must terminate with SIGABRT and print no `MISUSE` line.
const ASSERT_CASES: &[&str] = &[
    // `assert(outlen <= UINT8_MAX)` in crypto_generichash_blake2b_final G4-060
    "final_b2/outlen_256",
    "final_b2/outlen_257",
    "final_b2/outlen_1000",
    "final_b2/outlen_max",
    "final_g/outlen_256",
    "final_g/outlen_max",
];

/// Rows where the C dereferences a caller pointer *before* validating a length
/// (G4-115): `crypto_kdf[_blake2b]_derive_from_key` does
/// `memcpy(ctx_padded, ctx, 8)` before the `subkey_len` bounds check, so a NULL
/// `ctx` faults even for a `subkey_len` that would be rejected.
///
/// Also the hkdf `salt == NULL` rows where `salt_len` exceeds the HMAC block
/// size (64 for SHA-256, 128 for SHA-512): there the `keylen > BLOCK` pre-hash
/// branch is taken *before* the `key == NULL` check, so the NULL salt is
/// `memcpy`d instead of diagnosed — a SIGSEGV, not `sodium_misuse()`. That
/// refines rows G4-124 … G4-127, which only enumerate `salt_len == 5`.
const SEGV_CASES: &[&str] = &[
    "kdf_b2/ctx_null/len_15",
    "kdf_b2/ctx_null/len_65",
    "kdf_b2/ctx_null/len_32",
    "kdf_g/ctx_null/len_15",
    "kdf_g/ctx_null/len_32",
    "hkdf256/extract_init/salt_null_65",
    "hkdf256/extract_init/salt_null_200",
    "hkdf512/extract_init/salt_null_129",
    "hkdf512/extract_init/salt_null_200",
    "hkdf256/extract/salt_null_65",
    "hkdf512/extract/salt_null_129",
];

#[test]
fn misuse_child() {
    let Some((tag, lib)) = child_case() else {
        return;
    };
    let mut out = canary(320);
    let inp = [0x42u8; 200];
    let key = [0x37u8; 64];
    let salt = [0x11u8; 16];
    let pers = [0x22u8; 16];
    let mut st = State::new(384);

    let parts: Vec<&str> = tag.split('/').collect();
    match parts[0] {
        // ---- crypto_generichash[_blake2b] one-shot ----
        "gh_b2" | "gh_g" => {
            let name = if parts[0] == "gh_b2" {
                "crypto_generichash_blake2b"
            } else {
                "crypto_generichash"
            };
            let f = sym::<GhOneShot>(lib, name);
            set_observation(out.as_ptr(), 80);
            let rc = match parts[1] {
                "in_null_5" => unsafe {
                    f(out.as_mut_ptr(), 32, std::ptr::null(), 5, std::ptr::null(), 0)
                },
                "in_null_1" => unsafe {
                    f(out.as_mut_ptr(), 32, std::ptr::null(), 1, std::ptr::null(), 0)
                },
                "out_null" => unsafe {
                    f(std::ptr::null_mut(), 32, inp.as_ptr(), 200, std::ptr::null(), 0)
                },
                "key_null_5" => unsafe {
                    f(out.as_mut_ptr(), 32, inp.as_ptr(), 200, std::ptr::null(), 5)
                },
                "key_null_64" => unsafe {
                    f(out.as_mut_ptr(), 32, inp.as_ptr(), 200, std::ptr::null(), 64)
                },
                o => panic!("unknown case {o}"),
            };
            println!("OBS rc={rc} out={}", hex(&out[..80]));
        }
        // ---- crypto_generichash_blake2b_salt_personal ----
        "gh_sp" => {
            let f = sym::<GhSaltPers>(lib, "crypto_generichash_blake2b_salt_personal");
            set_observation(out.as_ptr(), 80);
            let rc = match parts[1] {
                "in_null_5" => unsafe {
                    f(out.as_mut_ptr(), 32, std::ptr::null(), 5, std::ptr::null(), 0,
                      salt.as_ptr(), pers.as_ptr())
                },
                "out_null" => unsafe {
                    f(std::ptr::null_mut(), 32, inp.as_ptr(), 200, std::ptr::null(), 0,
                      salt.as_ptr(), pers.as_ptr())
                },
                "key_null_5" => unsafe {
                    f(out.as_mut_ptr(), 32, inp.as_ptr(), 200, std::ptr::null(), 5,
                      salt.as_ptr(), pers.as_ptr())
                },
                o => panic!("unknown case {o}"),
            };
            println!("OBS rc={rc} out={}", hex(&out[..80]));
        }
        // ---- crypto_generichash[_blake2b]_final ----
        "final_b2" | "final_g" => {
            let base = if parts[0] == "final_b2" {
                "crypto_generichash_blake2b"
            } else {
                "crypto_generichash"
            };
            let init = sym::<GhInit>(lib, &format!("{base}_init"));
            let upd = sym::<GhUpdate>(lib, &format!("{base}_update"));
            let fin = sym::<GhFinal>(lib, &format!("{base}_final"));
            unsafe {
                init(st.as_mut_ptr(), std::ptr::null(), 0, 32);
                upd(st.as_mut_ptr(), inp.as_ptr(), 200);
            }
            let outlen: usize = match parts[1] {
                "outlen_0" => 0,
                "outlen_65" => 65,
                "outlen_100" => 100,
                "outlen_255" => 255,
                "outlen_256" => 256,
                "outlen_257" => 257,
                "outlen_1000" => 1000,
                "outlen_max" => usize::MAX,
                o => panic!("unknown case {o}"),
            };
            set_observation(out.as_ptr(), 320);
            let rc = unsafe { fin(st.as_mut_ptr(), out.as_mut_ptr(), outlen) };
            println!("OBS rc={rc} out={}", hex(&out[..320]));
        }
        // ---- internal blake2b_init family (exported as `_sodium_blake2b_*`) ----
        "b2_init" => {
            let f = sym::<B2Init>(lib, "_sodium_blake2b_init");
            let outlen: u8 = match parts[1] {
                "outlen_0" => 0,
                "outlen_65" => 65,
                "outlen_255" => 255,
                o => panic!("unknown case {o}"),
            };
            set_observation(st.as_ptr(), 64);
            let rc = unsafe { f(st.as_mut_ptr(), outlen) };
            println!("OBS rc={rc}");
        }
        "b2_init_sp" => {
            let f = sym::<B2InitSp>(lib, "_sodium_blake2b_init_salt_personal");
            let outlen: u8 = match parts[1] {
                "outlen_0" => 0,
                "outlen_255" => 255,
                o => panic!("unknown case {o}"),
            };
            set_observation(st.as_ptr(), 64);
            let rc = unsafe { f(st.as_mut_ptr(), outlen, salt.as_ptr(), pers.as_ptr()) };
            println!("OBS rc={rc}");
        }
        "b2_init_key" => {
            let f = sym::<B2InitKey>(lib, "_sodium_blake2b_init_key");
            set_observation(st.as_ptr(), 64);
            let (outlen, kp, keylen): (u8, *const u8, u8) = match parts[1] {
                "outlen_0" => (0, key.as_ptr(), 32),
                "outlen_255" => (255, key.as_ptr(), 32),
                "key_null" => (32, std::ptr::null(), 32),
                "keylen_0" => (32, key.as_ptr(), 0),
                "keylen_65" => (32, key.as_ptr(), 65),
                "keylen_255" => (32, key.as_ptr(), 255),
                o => panic!("unknown case {o}"),
            };
            let rc = unsafe { f(st.as_mut_ptr(), outlen, kp, keylen) };
            println!("OBS rc={rc}");
        }
        "b2_init_key_sp" => {
            let f = sym::<B2InitKeySp>(lib, "_sodium_blake2b_init_key_salt_personal");
            set_observation(st.as_ptr(), 64);
            let (outlen, kp, keylen): (u8, *const u8, u8) = match parts[1] {
                "outlen_0" => (0, key.as_ptr(), 32),
                "outlen_255" => (255, key.as_ptr(), 32),
                "key_null" => (32, std::ptr::null(), 32),
                "keylen_0" => (32, key.as_ptr(), 0),
                "keylen_65" => (32, key.as_ptr(), 65),
                o => panic!("unknown case {o}"),
            };
            let rc = unsafe {
                f(st.as_mut_ptr(), outlen, kp, keylen, salt.as_ptr(), pers.as_ptr())
            };
            println!("OBS rc={rc}");
        }
        // ---- hkdf `salt == NULL && salt_len > 0` ----
        "hkdf256" | "hkdf512" => {
            let base = if parts[0] == "hkdf256" {
                "crypto_kdf_hkdf_sha256"
            } else {
                "crypto_kdf_hkdf_sha512"
            };
            let salt_len: usize = parts[2].rsplit('_').next().unwrap().parse().unwrap();
            set_observation(out.as_ptr(), 64);
            let rc = if parts[1] == "extract_init" {
                let f = sym::<HkdfExInit>(lib, &format!("{base}_extract_init"));
                unsafe { f(st.as_mut_ptr(), std::ptr::null(), salt_len) }
            } else {
                let f = sym::<HkdfExtract>(lib, &format!("{base}_extract"));
                unsafe {
                    f(out.as_mut_ptr(), std::ptr::null(), salt_len, inp.as_ptr(), 200)
                }
            };
            println!("OBS rc={rc} out={}", hex(&out[..64]));
        }
        // ---- kdf: `ctx` is dereferenced before `subkey_len` is validated ----
        "kdf_b2" | "kdf_g" => {
            let name = if parts[0] == "kdf_b2" {
                "crypto_kdf_blake2b_derive_from_key"
            } else {
                "crypto_kdf_derive_from_key"
            };
            let f = sym::<KdfDerive>(lib, name);
            let sklen: usize = parts[2].rsplit('_').next().unwrap().parse().unwrap();
            let rc = unsafe {
                f(out.as_mut_ptr(), sklen, 1, std::ptr::null(), key.as_ptr())
            };
            println!("OBS rc={rc}");
        }
        o => panic!("unknown tag group {o}"),
    }

    use std::io::Write;
    let _ = std::io::stdout().flush();
    std::process::exit(0);
}

/// Drives every abort row of `ERRORS.md ## G4` out of process and requires the
/// C and the Rust child to agree on exit code, signal *and* the side effects
/// observed before the abort.
#[test]
fn misuse_paths_match() {
    if child_tag().is_some() {
        return;
    }
    setup();
    use std::os::unix::process::ExitStatusExt;

    for &tag in MISUSE_CASES {
        let c = run_child("misuse_child", "c", tag);
        let r = run_child("misuse_child", "r", tag);
        eq_child(tag, &c, &r);
        assert_eq!(
            c.status.code(),
            Some(MISUSE_EXIT),
            "{tag}: C did not reach sodium_misuse (stdout: {}, stderr: {})",
            String::from_utf8_lossy(&c.stdout),
            String::from_utf8_lossy(&c.stderr),
        );
    }

    for &tag in ASSERT_CASES {
        let c = run_child("misuse_child", "c", tag);
        let r = run_child("misuse_child", "r", tag);
        eq_child(tag, &c, &r);
        assert_eq!(
            c.status.signal(),
            Some(6),
            "{tag}: C must die on a raw assert() (SIGABRT), got code={:?} signal={:?}, stderr: {}",
            c.status.code(),
            c.status.signal(),
            String::from_utf8_lossy(&c.stderr),
        );
        assert!(
            !String::from_utf8_lossy(&c.stdout).contains("MISUSE"),
            "{tag}: a raw assert() must NOT run the misuse handler"
        );
        assert!(
            String::from_utf8_lossy(&c.stderr).contains("outlen <= UINT8_MAX"),
            "{tag}: expected the glibc assertion message, got: {}",
            String::from_utf8_lossy(&c.stderr)
        );
    }

    for &tag in SEGV_CASES {
        let c = run_child("misuse_child", "c", tag);
        let r = run_child("misuse_child", "r", tag);
        eq_child(tag, &c, &r);
        assert_eq!(
            c.status.signal(),
            Some(11),
            "{tag}: C must fault on the pre-validation ctx dereference, \
             got code={:?} signal={:?}",
            c.status.code(),
            c.status.signal(),
        );
    }
}

// ===========================================================================
// rows that cannot be constructed
// ===========================================================================

/// Rows of `ERRORS.md ## G4` whose trigger is unreachable, compile-time only,
/// or undefined behaviour rather than a diagnosed rejection. Recorded here with
/// the reason so no row is silently dropped; where the claim rests on a
/// constant, the constant is asserted.
///
/// * **G4-004, G4-024** — `inlen > UINT64_MAX` with an `unsigned long long`
///   argument: dead on x86-64. Asserted below via `u64::MAX`.
/// * **G4-005, G4-006, G4-025, G4-051** — `assert(outlen <= UINT8_MAX)` /
///   `assert(keylen <= UINT8_MAX)` in the one-shot / `_init` /
///   `_init_salt_personal` wrappers: unreachable, the preceding `> 64` range
///   check already returned `-1`. Covered *positively* by
///   `generichash_one_shot_outlen_keylen_rejected`, which passes
///   `outlen = SIZE_MAX` and gets `-1` (never an abort).
/// * **G4-010, G4-011, G4-012, G4-029** — the internal `blake2b()` /
///   `blake2b_salt_personal()` `outlen`/`keylen` guards and the
///   `blake2b_init*() < 0` branches: unreachable *through the public wrapper*.
///   The guards themselves ARE exercised directly through the exported
///   `_sodium_blake2b_init*` symbols (`b2_init*` misuse tags).
/// * **G4-035, G4-036, G4-052** — `blake2b_init*() != 0 -> return -1` in the
///   `_init` wrappers: `blake2b_init*` only ever returns `0` or aborts.
/// * **G4-041, G4-044, G4-045** — `blake2b_init_param() < 0`: the function is a
///   straight-line `return 0`, and its only check is a `COMPILER_ASSERT`
///   (compile-time). Verified below by calling the exported
///   `_sodium_blake2b_init_param` and requiring `0`.
/// * **G4-064** — `assert(S->buflen <= BLAKE2B_BLOCKBYTES)` in `blake2b_final`:
///   `blake2b_update` caps `buflen` at `2*128`, so `buflen - 128 <= 128`
///   always. Driven with a 256-byte lazy buffer by
///   `generichash_final_state_machine`.
/// * **G4-065, G4-066, G4-067** — accessor / `_keygen` / `pick_best`
///   entry points with no failure mode; asserted below.
/// * **G4-081** — `crypto_shorthash_siphash24` with a key shorter than 16
///   bytes: an out-of-bounds READ, not a rejection. Constructing it is UB with
///   no defined observable, so it is not tested.
/// * **G4-097, G4-098** — `crypto_core_keccak1600_xor_bytes` /
///   `_extract_bytes` with `offset + length > 200`: silent out-of-bounds
///   access with a `void` return; no error is signalled and the effect is
///   memory corruption, so this cannot be turned into a differential
///   assertion. (`crypto_core/` is also outside this group's ownership.)
/// * **G4-115** — driven for real by the `SEGV_CASES` tags above.
/// * **G4-116, G4-117** — no `ctx_len` parameter exists, and after the
///   `16 <= subkey_len <= 64` check the callee cannot fail; asserted below.
/// * **G4-057, G4-068, G4-071 (partially), G4-123** — `in == NULL` with
///   `inlen > 0`: UB / SIGSEGV, explicitly *not* a diagnosed error. Only the
///   `inlen == 0` half is testable and is covered above.
#[test]
fn documented_unreachable_error_rows() {
    setup();
    let mut rng = Rng::new(0xE450);

    // G4-004 / G4-024: `inlen` is `unsigned long long`, so `inlen > UINT64_MAX`
    // is structurally impossible.
    assert_eq!(u64::MAX, 0xffff_ffff_ffff_ffff);

    // G4-041 / G4-044 / G4-045: `blake2b_init_param` is unconditional.
    type InitParam = unsafe extern "C" fn(*mut u8, *const u8) -> i32;
    let (cp, rp) = pair::<InitParam>("_sodium_blake2b_init_param");
    for _ in 0..32 {
        let param = rng.bytes(64);
        let mut sa = State::new(384);
        let mut sb2 = State::new(384);
        let (ra, rb) =
            unsafe { (cp(sa.as_mut_ptr(), param.as_ptr()), rp(sb2.as_mut_ptr(), param.as_ptr())) };
        eq_i32("_sodium_blake2b_init_param rc", ra, rb);
        assert_eq!(ra, 0, "blake2b_init_param can only return 0");
        eq_bytes("_sodium_blake2b_init_param state", &sa.bytes()[..64], &sb2.bytes()[..64]);
    }

    // G4-065: every accessor, plus `_keygen`.
    for lib in [c_lib(), r_lib()] {
        assert_eq!(usz(lib, "crypto_generichash_statebytes"), 384);
        assert_eq!(usz(lib, "crypto_generichash_blake2b_statebytes"), 384);
        assert_eq!(usz(lib, "crypto_generichash_bytes"), 32);
        assert_eq!(usz(lib, "crypto_generichash_bytes_min"), 16);
        assert_eq!(usz(lib, "crypto_generichash_bytes_max"), 64);
        assert_eq!(usz(lib, "crypto_generichash_keybytes"), 32);
        assert_eq!(usz(lib, "crypto_generichash_keybytes_min"), 16);
        assert_eq!(usz(lib, "crypto_generichash_keybytes_max"), 64);
        assert_eq!(usz(lib, "crypto_generichash_blake2b_saltbytes"), 16);
        assert_eq!(usz(lib, "crypto_generichash_blake2b_personalbytes"), 16);
        assert_eq!(cstr(lib, "crypto_generichash_primitive"), "blake2b");
    }

    // G4-066 / G4-067: `pick_best_implementation` always returns 0 and the
    // results are invariant across repeated calls.
    type PickBest = unsafe extern "C" fn() -> i32;
    let (cpb, rpb) = pair::<PickBest>("_crypto_generichash_blake2b_pick_best_implementation");
    let (c1, r1) = pair::<GhOneShot>("crypto_generichash_blake2b");
    let inp = rng.bytes(500);
    let mut base = canary(32);
    unsafe { c1(base.as_mut_ptr(), 32, inp.as_ptr(), 500, std::ptr::null(), 0) };
    for round in 0..4 {
        let mut a = canary(32);
        let mut b = canary(32);
        let (ra, rb) = unsafe { (cpb(), rpb()) };
        eq_i32("pick_best_implementation rc", ra, rb);
        assert_eq!(ra, 0);
        unsafe {
            c1(a.as_mut_ptr(), 32, inp.as_ptr(), 500, std::ptr::null(), 0);
            r1(b.as_mut_ptr(), 32, inp.as_ptr(), 500, std::ptr::null(), 0);
        }
        eq_bytes(&format!("digest invariant after pick_best #{round}"), &a, &b);
        eq_bytes(&format!("digest unchanged by pick_best #{round}"), &base, &a);
    }

    // G4-117: after the bounds check the callee cannot fail — every
    // `subkey_len` in `[16, 64]` succeeds (already asserted in
    // `kdf_derive_from_key_subkey_len_rejected`); re-assert the bound itself.
    assert_eq!(usz(c_lib(), "crypto_kdf_blake2b_bytes_min"), 16);
    assert_eq!(usz(c_lib(), "crypto_kdf_blake2b_bytes_max"), 64);
    assert_eq!(usz(c_lib(), "crypto_kdf_blake2b_keybytes"), 32);

    // G4-101 / G4-111: entry points that do not exist.
    for missing in [
        "crypto_core_keccak1600_absorb",
        "crypto_core_keccak1600_pad",
        "crypto_xof_shake128_final",
        "crypto_xof_shake256_absorb",
        "crypto_xof_turboshake128_clone",
        "crypto_xof_turboshake256_final",
    ] {
        assert!(absent_in_both(missing), "{missing} must not exist");
    }
}
