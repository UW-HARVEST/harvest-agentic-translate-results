//! Phase B + C for the last uncovered exports.
//!
//! * every `*_keygen()` helper (non-deterministic: length + canary + the key is
//!   usable in the OTHER library),
//! * `crypto_pwhash_argon2i_str*` / `crypto_pwhash_argon2id_str*`,
//! * `sodium_init` / `sodium_crit_enter` / `sodium_crit_leave` /
//!   `sodium_misuse` / `sodium_set_misuse_handler`,
//! * the legacy `randombytes(buf, len)` entry point,
//! * the exported Argon2 internals (`_sodium_argon2*`), `_sodium_escrypt_r`
//!   and `_sodium_core_h2c_string_to_hash`.

mod harness;
use harness::*;

use std::ffi::{c_char, c_int, c_void, CStr};
use std::ptr;

const SEED: u64 = 0x5EED_000E;

unsafe fn errno() -> c_int {
    *libc::__errno_location()
}
unsafe fn set_errno(v: c_int) {
    *libc::__errno_location() = v;
}

// ---------------------------------------------------------------------------
// Every *_keygen helper.
// ---------------------------------------------------------------------------

/// (`_keygen` symbol, the `*_keybytes()` getter that gives its output length)
/// (Note: `crypto_kdf_blake2b_keygen` and `crypto_shorthash_siphash24_keygen`
/// do NOT exist in libsodium 1.0.23 — only the generic `crypto_kdf_keygen` /
/// `crypto_shorthash_keygen` do — and neither `.so` exports them, which
/// `SYMBOLS.md`'s 0/0 diff already covers.)
const KEYGENS: &[(&str, &str)] = &[
    ("crypto_aead_aegis128l_keygen", "crypto_aead_aegis128l_keybytes"),
    ("crypto_aead_aegis256_keygen", "crypto_aead_aegis256_keybytes"),
    ("crypto_aead_aes256gcm_keygen", "crypto_aead_aes256gcm_keybytes"),
    ("crypto_aead_chacha20poly1305_keygen", "crypto_aead_chacha20poly1305_keybytes"),
    ("crypto_aead_chacha20poly1305_ietf_keygen", "crypto_aead_chacha20poly1305_ietf_keybytes"),
    ("crypto_aead_xchacha20poly1305_ietf_keygen", "crypto_aead_xchacha20poly1305_ietf_keybytes"),
    ("crypto_auth_keygen", "crypto_auth_keybytes"),
    ("crypto_auth_hmacsha256_keygen", "crypto_auth_hmacsha256_keybytes"),
    ("crypto_auth_hmacsha512_keygen", "crypto_auth_hmacsha512_keybytes"),
    ("crypto_auth_hmacsha512256_keygen", "crypto_auth_hmacsha512256_keybytes"),
    ("crypto_generichash_keygen", "crypto_generichash_keybytes"),
    ("crypto_generichash_blake2b_keygen", "crypto_generichash_blake2b_keybytes"),
    ("crypto_kdf_keygen", "crypto_kdf_keybytes"),
    ("crypto_kdf_hkdf_sha256_keygen", "crypto_kdf_hkdf_sha256_keybytes"),
    ("crypto_kdf_hkdf_sha512_keygen", "crypto_kdf_hkdf_sha512_keybytes"),
    ("crypto_onetimeauth_keygen", "crypto_onetimeauth_keybytes"),
    ("crypto_onetimeauth_poly1305_keygen", "crypto_onetimeauth_poly1305_keybytes"),
    ("crypto_secretbox_keygen", "crypto_secretbox_keybytes"),
    ("crypto_secretbox_xsalsa20poly1305_keygen", "crypto_secretbox_xsalsa20poly1305_keybytes"),
    ("crypto_secretstream_xchacha20poly1305_keygen", "crypto_secretstream_xchacha20poly1305_keybytes"),
    ("crypto_shorthash_keygen", "crypto_shorthash_keybytes"),
    ("crypto_stream_keygen", "crypto_stream_keybytes"),
    ("crypto_stream_chacha20_keygen", "crypto_stream_chacha20_keybytes"),
    ("crypto_stream_chacha20_ietf_keygen", "crypto_stream_chacha20_ietf_keybytes"),
    ("crypto_stream_salsa20_keygen", "crypto_stream_salsa20_keybytes"),
    ("crypto_stream_salsa2012_keygen", "crypto_stream_salsa2012_keybytes"),
    ("crypto_stream_salsa208_keygen", "crypto_stream_salsa208_keybytes"),
    ("crypto_stream_xchacha20_keygen", "crypto_stream_xchacha20_keybytes"),
    ("crypto_stream_xsalsa20_keygen", "crypto_stream_xsalsa20_keybytes"),
    ("crypto_ipcrypt_keygen", "crypto_ipcrypt_keybytes"),
    ("crypto_ipcrypt_nd_keygen", "crypto_ipcrypt_nd_keybytes"),
    ("crypto_ipcrypt_ndx_keygen", "crypto_ipcrypt_ndx_keybytes"),
    ("crypto_ipcrypt_pfx_keygen", "crypto_ipcrypt_pfx_keybytes"),
];

/// `*_keygen` draws from the CSPRNG, so its bytes are not comparable. What IS
/// comparable: that both write EXACTLY `keybytes()` bytes (canary intact), that
/// neither produces a constant, and that repeated calls differ.
#[test]
fn every_keygen_writes_exactly_keybytes() {
    let mut checked = 0;
    for (kg, kb) in KEYGENS {
        assert!(has(kg), "{kg} not exported by both .so");
        let (cl, rl) = sym::<unsafe extern "C" fn() -> usize>(kb);
        let (lc, lr) = unsafe { (cl(), rl()) };
        assert_eq!(lc, lr, "{kb}");
        let n = lc;

        let (c, r) = sym::<unsafe extern "C" fn(*mut u8)>(kg);
        let mut prev_c: Vec<Vec<u8>> = Vec::new();
        let mut prev_r: Vec<Vec<u8>> = Vec::new();
        for _ in 0..8 {
            let mut bc = out_buf(n);
            let mut br = out_buf(n);
            unsafe {
                c(bc.as_mut_ptr());
                r(br.as_mut_ptr());
            }
            // canary must be untouched in BOTH: neither may write past keybytes
            eqb(&format!("{kg}: canary"), &bc[n..], &br[n..]);
            eqb(&format!("{kg}: canary vs pristine"), &bc[n..], &out_buf(n)[n..]);
            assert_ne!(&bc[..n], &vec![0u8; n][..], "{kg}: C produced all zeros");
            assert_ne!(&br[..n], &vec![0u8; n][..], "{kg}: Rust produced all zeros");
            prev_c.push(bc[..n].to_vec());
            prev_r.push(br[..n].to_vec());
        }
        // no repeats within either library (would indicate a fixed key)
        for i in 0..prev_c.len() {
            for j in i + 1..prev_c.len() {
                assert_ne!(prev_c[i], prev_c[j], "{kg}: C repeated a key");
                assert_ne!(prev_r[i], prev_r[j], "{kg}: Rust repeated a key");
            }
        }
        checked += 1;
    }
    assert!(checked >= 30, "only {checked} keygens checked");
}

/// A key generated by ONE library must be fully usable by the OTHER — this is
/// what "the keygen agrees" actually means for a non-deterministic function.
#[test]
fn keygen_output_is_cross_usable() {
    type Mac4 = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8) -> c_int;
    let mut rng = Rng::new(SEED);
    let msg = rng.bytes(100);

    for (kg, mac, outlen, keylen) in [
        ("crypto_auth_keygen", "crypto_auth", 32usize, 32usize),
        ("crypto_auth_hmacsha256_keygen", "crypto_auth_hmacsha256", 32, 32),
        ("crypto_auth_hmacsha512_keygen", "crypto_auth_hmacsha512", 64, 32),
        ("crypto_auth_hmacsha512256_keygen", "crypto_auth_hmacsha512256", 32, 32),
        ("crypto_onetimeauth_keygen", "crypto_onetimeauth", 16, 32),
        ("crypto_shorthash_keygen", "crypto_shorthash", 8, 16),
    ] {
        let (ckg, rkg) = sym::<unsafe extern "C" fn(*mut u8)>(kg);
        let (cm, rm) = sym::<Mac4>(mac);
        for (tag, gen) in [("C-key", ckg), ("Rust-key", rkg)] {
            let mut k = vec![0u8; keylen];
            unsafe { gen(k.as_mut_ptr()) };
            let mut oc = out_buf(outlen);
            let mut or = out_buf(outlen);
            unsafe {
                let rc = cm(oc.as_mut_ptr(), msg.as_ptr(), 100, k.as_ptr());
                let rr = rm(or.as_mut_ptr(), msg.as_ptr(), 100, k.as_ptr());
                assert_eq!(rc, rr, "{mac} with {tag}");
            }
            eqb(&format!("{mac} with {tag}"), &oc, &or);
        }
    }
}

// ---------------------------------------------------------------------------
// crypto_pwhash_argon2i_str* / crypto_pwhash_argon2id_str*
// ---------------------------------------------------------------------------

type PwStr = unsafe extern "C" fn(*mut c_char, *const c_char, u64, u64, usize) -> c_int;
type PwStrVerify = unsafe extern "C" fn(*const c_char, *const c_char, u64) -> c_int;
type PwStrRehash = unsafe extern "C" fn(*const c_char, u64, usize) -> c_int;

const STRBYTES: usize = 128;

/// Known-good encoded hashes, produced with (opslimit, memlimit) = (3, 8192)
/// for argon2i and (1, 8192) for argon2id — the cheapest legal parameters.
/// Verification is fully deterministic, so these are byte-exact vectors.
fn fixed_vectors() -> Vec<(&'static str, &'static str, &'static str)> {
    // (label, encoded string, the password that matches it)
    vec![
        ("argon2i", "$argon2i$v=19$m=8,t=3,p=1$c29tZXNhbHRzb21lc2FsdA$YOMDMy8QoEHhRQ0V+2xVdCbW7cPRe6BqhWzOOw", "password"),
        ("argon2id", "$argon2id$v=19$m=8,t=1,p=1$c29tZXNhbHRzb21lc2FsdA$Zf/oGXKKZ0dnCmYWLBGKFbBQaBBSXhTVoQ", "password"),
    ]
}

const MALFORMED: &[&str] = &[
    "",
    "$",
    "$$",
    "$argon2i",
    "$argon2i$",
    "$argon2x$v=19$m=8,t=3,p=1$c29tZXNhbHQ$aaaa",
    "$argon2i$v=18$m=8,t=3,p=1$c29tZXNhbHQ$aaaa",
    "$argon2i$v=19$m=8,t=3$c29tZXNhbHQ$aaaa",
    "$argon2i$v=19$t=3,p=1$c29tZXNhbHQ$aaaa",
    "$argon2i$v=19$m=08,t=3,p=1$c29tZXNhbHQ$aaaa",
    "$argon2i$v=19$m=x,t=3,p=1$c29tZXNhbHQ$aaaa",
    "$argon2i$v=19$m=8,t=3,p=1$!!!!$aaaa",
    "$argon2i$v=19$m=8,t=3,p=1$c29tZXNhbHQ$",
    "$argon2i$v=19$m=8,t=3,p=1$c29tZXNhbHQ",
    "$argon2i$v=19$m=8,t=3,p=1$c29tZXNhbHRzb21lc2FsdA$YOMDMy8QoEHhRQ0V+2xVdCbW7cPRe6BqhWzOOwX",
    "$argon2i$v=19$m=8,t=3,p=1$c29tZXNhbHRzb21lc2FsdA$YOMDMy8QoEHhRQ0V+2xVdCbW7cPRe6BqhWzOO",
    "$argon2id$v=19$m=8,t=1,p=1$c29tZXNhbHRzb21lc2FsdA$Zf/oGXKKZ0dnCmYWLBGKFbBQaBBSXhTVoQtrailing",
    "$7$C6..../....c29tZXNhbHQ$",
    "not a hash at all",
];

fn cstr(s: &str) -> Vec<u8> {
    let mut v = s.as_bytes().to_vec();
    v.push(0);
    v
}

#[test]
fn argon2_primitive_str_verify_and_needs_rehash() {
    for alg in ["argon2i", "argon2id"] {
        let vname = format!("crypto_pwhash_{alg}_str_verify");
        let rname = format!("crypto_pwhash_{alg}_str_needs_rehash");
        let (cv, rv) = sym::<PwStrVerify>(&vname);
        let (cr, rr) = sym::<PwStrRehash>(&rname);

        // valid vectors (both algorithms' strings are fed to BOTH verifiers,
        // so the prefix-mismatch rejection is exercised too)
        for (label, enc, pw) in fixed_vectors() {
            let e = cstr(enc);
            for pwd in [pw, "wrong", ""] {
                let p = cstr(pwd);
                unsafe {
                    set_errno(0);
                    let a = cv(e.as_ptr() as _, p.as_ptr() as _, pwd.len() as u64);
                    let ea = errno();
                    set_errno(0);
                    let b = rv(e.as_ptr() as _, p.as_ptr() as _, pwd.len() as u64);
                    let eb = errno();
                    assert_eq!(a, b, "{vname}({label}, {pwd:?}) rc");
                    assert_eq!(ea, eb, "{vname}({label}, {pwd:?}) errno");
                }
            }
            // needs_rehash across a range of parameters
            for (ops, mem) in [(1u64, 8192usize), (2, 8192), (3, 8192), (3, 16384), (4, 8192), (1, 1 << 20)] {
                unsafe {
                    set_errno(0);
                    let a = cr(e.as_ptr() as _, ops, mem);
                    let ea = errno();
                    set_errno(0);
                    let b = rr(e.as_ptr() as _, ops, mem);
                    let eb = errno();
                    assert_eq!(a, b, "{rname}({label}, ops={ops}, mem={mem}) rc");
                    assert_eq!(ea, eb, "{rname}({label}, ops={ops}, mem={mem}) errno");
                }
            }
        }
        // malformed strings
        for bad in MALFORMED {
            let e = cstr(bad);
            let p = cstr("password");
            unsafe {
                set_errno(0);
                let a = cv(e.as_ptr() as _, p.as_ptr() as _, 8);
                let ea = errno();
                set_errno(0);
                let b = rv(e.as_ptr() as _, p.as_ptr() as _, 8);
                let eb = errno();
                assert_eq!(a, b, "{vname}({bad:?}) rc");
                assert_eq!(ea, eb, "{vname}({bad:?}) errno");

                set_errno(0);
                let a = cr(e.as_ptr() as _, 3, 8192);
                let ea = errno();
                set_errno(0);
                let b = rr(e.as_ptr() as _, 3, 8192);
                let eb = errno();
                assert_eq!(a, b, "{rname}({bad:?}) rc");
                assert_eq!(ea, eb, "{rname}({bad:?}) errno");
            }
        }
    }
}

/// `*_str` embeds a fresh random salt, so the encoded bytes differ. What must
/// match: the return code, the `$`-delimited parameter prefix, and the fact
/// that each library's own string verifies under BOTH verifiers.
#[test]
fn argon2_primitive_str_cross_verify() {
    for (alg, minops) in [("argon2i", 3u64), ("argon2id", 1u64)] {
        let sname = format!("crypto_pwhash_{alg}_str");
        let vname = format!("crypto_pwhash_{alg}_str_verify");
        let (cs, rs) = sym::<PwStr>(&sname);
        let (cv, rv) = sym::<PwStrVerify>(&vname);
        let mut rng = Rng::new(SEED ^ 1);

        for (ops, mem) in [(minops, 8192usize), (minops + 1, 8192), (minops, 16384)] {
            for pwlen in [0usize, 1, 8, 64] {
                let pw: Vec<u8> = rng.bytes(pwlen).iter().map(|b| 0x20 + (b % 0x5f)).collect();
                let mut pwz = pw.clone();
                pwz.push(0);

                let mut oc = vec![0u8; STRBYTES + 16];
                let mut or = vec![0u8; STRBYTES + 16];
                let (a, b) = unsafe {
                    (
                        cs(oc.as_mut_ptr() as _, pwz.as_ptr() as _, pwlen as u64, ops, mem),
                        rs(or.as_mut_ptr() as _, pwz.as_ptr() as _, pwlen as u64, ops, mem),
                    )
                };
                assert_eq!(a, b, "{sname}(ops={ops}, mem={mem}, pwlen={pwlen}) rc");
                if a != 0 {
                    continue;
                }
                let sc = CStr::from_bytes_until_nul(&oc).unwrap().to_bytes();
                let sr = CStr::from_bytes_until_nul(&or).unwrap().to_bytes();
                assert!(sc.len() < STRBYTES, "{sname}: C string too long");
                assert!(sr.len() < STRBYTES, "{sname}: Rust string too long");

                // parameter prefix: everything up to and including the '$'
                // before the salt (4 '$' separators in "$argon2X$v=..$m=..$")
                let prefix = |s: &[u8]| -> Vec<u8> {
                    let mut n = 0;
                    for (i, &c) in s.iter().enumerate() {
                        if c == b'$' {
                            n += 1;
                            if n == 4 {
                                return s[..=i].to_vec();
                            }
                        }
                    }
                    s.to_vec()
                };
                eqb(
                    &format!("{sname} parameter prefix (ops={ops}, mem={mem})"),
                    &prefix(sc),
                    &prefix(sr),
                );

                // each library's string must verify under BOTH verifiers
                for (tag, s) in [("C", sc), ("Rust", sr)] {
                    let z = {
                        let mut v = s.to_vec();
                        v.push(0);
                        v
                    };
                    unsafe {
                        let x = cv(z.as_ptr() as _, pwz.as_ptr() as _, pwlen as u64);
                        let y = rv(z.as_ptr() as _, pwz.as_ptr() as _, pwlen as u64);
                        assert_eq!(x, y, "{vname} on {tag} string: rc");
                        assert_eq!(x, 0, "{vname} on {tag} string must verify");
                    }
                    // wrong password must be rejected by both
                    let bad = cstr("definitely-not-the-password");
                    unsafe {
                        let x = cv(z.as_ptr() as _, bad.as_ptr() as _, 27);
                        let y = rv(z.as_ptr() as _, bad.as_ptr() as _, 27);
                        assert_eq!(x, y, "{vname} on {tag} string, wrong pw: rc");
                        assert_eq!(x, -1);
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// sodium/core.c
// ---------------------------------------------------------------------------

#[test]
fn sodium_init_and_crit_sections() {
    let (ci, ri) = sym::<unsafe extern "C" fn() -> c_int>("sodium_init");
    let (ce, re) = sym::<unsafe extern "C" fn() -> c_int>("sodium_crit_enter");
    let (cl, rl) = sym::<unsafe extern "C" fn() -> c_int>("sodium_crit_leave");
    unsafe {
        // already initialised by the harness -> both must report 1
        for _ in 0..5 {
            let (a, b) = (ci(), ri());
            assert_eq!(a, b, "sodium_init (repeat)");
            assert_eq!(a, 1, "sodium_init on an already-initialised library");
        }
        // balanced enter/leave, several times
        for _ in 0..10 {
            let (a, b) = (ce(), re());
            assert_eq!(a, b, "sodium_crit_enter");
            let (a2, b2) = (cl(), rl());
            assert_eq!(a2, b2, "sodium_crit_leave");
        }
        // unbalanced leave (no matching enter)
        set_errno(0);
        let a = cl();
        let ea = errno();
        set_errno(0);
        let b = rl();
        let eb = errno();
        assert_eq!(a, b, "sodium_crit_leave without enter: rc");
        assert_eq!(ea, eb, "sodium_crit_leave without enter: errno");
    }
}

/// `sodium_misuse()` must terminate the process the same way in both.
#[test]
fn sodium_misuse_terminates_identically() {
    same_outcome(
        "sodium_misuse()",
        || {
            let (c, _) = sym::<unsafe extern "C" fn()>("sodium_misuse");
            unsafe { c() };
            0
        },
        || {
            let (_, r) = sym::<unsafe extern "C" fn()>("sodium_misuse");
            unsafe { r() };
            0
        },
    );
}

extern "C" fn misuse_handler() {
    unsafe { libc::_exit(77) };
}

/// With a handler installed, `sodium_misuse()` must run the handler in both
/// (here the handler exits 77, so a matching exit status proves it ran).
#[test]
fn sodium_set_misuse_handler_is_honoured() {
    same_outcome(
        "sodium_set_misuse_handler + sodium_misuse",
        || {
            let (cset, _) =
                sym::<unsafe extern "C" fn(Option<extern "C" fn()>) -> c_int>("sodium_set_misuse_handler");
            let (cm, _) = sym::<unsafe extern "C" fn()>("sodium_misuse");
            unsafe {
                if cset(Some(misuse_handler)) != 0 {
                    return 1;
                }
                cm();
            }
            2
        },
        || {
            let (_, rset) =
                sym::<unsafe extern "C" fn(Option<extern "C" fn()>) -> c_int>("sodium_set_misuse_handler");
            let (_, rm) = sym::<unsafe extern "C" fn()>("sodium_misuse");
            unsafe {
                if rset(Some(misuse_handler)) != 0 {
                    return 1;
                }
                rm();
            }
            2
        },
    );
    // Installing and clearing the handler must return the same code, and must
    // not disturb the library.
    let (cset, rset) =
        sym::<unsafe extern "C" fn(Option<extern "C" fn()>) -> c_int>("sodium_set_misuse_handler");
    unsafe {
        assert_eq!(cset(Some(misuse_handler)), rset(Some(misuse_handler)));
        assert_eq!(cset(None), rset(None), "clearing the misuse handler");
    }
    // and a misuse after clearing is a plain abort again
    same_outcome(
        "sodium_misuse after clearing the handler",
        || {
            let (cset, _) =
                sym::<unsafe extern "C" fn(Option<extern "C" fn()>) -> c_int>("sodium_set_misuse_handler");
            let (cm, _) = sym::<unsafe extern "C" fn()>("sodium_misuse");
            unsafe {
                cset(None);
                cm();
            }
            0
        },
        || {
            let (_, rset) =
                sym::<unsafe extern "C" fn(Option<extern "C" fn()>) -> c_int>("sodium_set_misuse_handler");
            let (_, rm) = sym::<unsafe extern "C" fn()>("sodium_misuse");
            unsafe {
                rset(None);
                rm();
            }
            0
        },
    );
}

// ---------------------------------------------------------------------------
// the legacy `randombytes(buf, buf_len)` entry point
// ---------------------------------------------------------------------------

#[test]
fn legacy_randombytes_entry_point() {
    let (c, r) = sym::<unsafe extern "C" fn(*mut u8, u64)>("randombytes");
    for n in [0usize, 1, 16, 32, 64, 1000, 4096] {
        let mut bc = out_buf(n);
        let mut br = out_buf(n);
        unsafe {
            c(bc.as_mut_ptr(), n as u64);
            r(br.as_mut_ptr(), n as u64);
        }
        eqb(&format!("randombytes({n}) canary"), &bc[n..], &br[n..]);
        eqb(&format!("randombytes({n}) canary pristine"), &bc[n..], &out_buf(n)[n..]);
        if n >= 16 {
            assert_ne!(&bc[..n], &vec![0u8; n][..], "randombytes: C all zeros");
            assert_ne!(&br[..n], &vec![0u8; n][..], "randombytes: Rust all zeros");
        }
    }
}

// ---------------------------------------------------------------------------
// `_sodium_core_h2c_string_to_hash`
// ---------------------------------------------------------------------------

#[test]
fn core_h2c_string_to_hash_internal() {
    type F = unsafe extern "C" fn(*mut u8, usize, *const u8, usize, *const u8, usize, c_int) -> c_int;
    let (c, r) = sym::<F>("_sodium_core_h2c_string_to_hash");
    let mut rng = Rng::new(SEED ^ 2);
    // hash_alg 1 = CORE_H2C_SHA256, 2 = CORE_H2C_SHA512; every other int is a
    // real input across the FFI boundary and must be handled identically.
    for alg in [0i32, 1, 2, 3, -1, 255, 999, i32::MIN, i32::MAX] {
        for h_len in [0usize, 1, 32, 48, 64, 96, 128, 200] {
            for ctx_len in [0usize, 1, 32, 254, 255, 256, 300] {
                for msg_len in [0usize, 1, 32, 200] {
                    let ctx = rng.bytes(ctx_len);
                    let msg = rng.bytes(msg_len);
                    let cp = if ctx_len == 0 { ptr::null() } else { ctx.as_ptr() };
                    let mp = if msg_len == 0 { ptr::null() } else { msg.as_ptr() };
                    let mut oc = out_buf(h_len.max(1));
                    let mut or = out_buf(h_len.max(1));
                    unsafe {
                        let a = c(oc.as_mut_ptr(), h_len, cp, ctx_len, mp, msg_len, alg);
                        let b = r(or.as_mut_ptr(), h_len, cp, ctx_len, mp, msg_len, alg);
                        assert_eq!(
                            a, b,
                            "_sodium_core_h2c_string_to_hash(h_len={h_len}, ctx={ctx_len}, msg={msg_len}, alg={alg}) rc"
                        );
                    }
                    eqb(
                        &format!("core_h2c_string_to_hash h_len={h_len} ctx={ctx_len} msg={msg_len} alg={alg}"),
                        &oc,
                        &or,
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// the exported Argon2 internals
// ---------------------------------------------------------------------------

#[repr(C)]
struct Argon2Context {
    out: *mut u8,
    outlen: u32,
    pwd: *mut u8,
    pwdlen: u32,
    salt: *mut u8,
    saltlen: u32,
    secret: *mut u8,
    secretlen: u32,
    ad: *mut u8,
    adlen: u32,
    t_cost: u32,
    m_cost: u32,
    lanes: u32,
    threads: u32,
    flags: u32,
}

const ARGON2_I: c_int = 1;
const ARGON2_ID: c_int = 2;
const ARGON2_MAX_LANES: u32 = 0x00FF_FFFF;

type A2HashRaw = unsafe extern "C" fn(u32, u32, u32, *const c_void, usize, *const c_void, usize, *mut c_void, usize) -> c_int;
type A2HashEnc = unsafe extern "C" fn(u32, u32, u32, *const c_void, usize, *const c_void, usize, usize, *mut c_char, usize) -> c_int;
type A2Verify = unsafe extern "C" fn(*const c_char, *const c_void, usize) -> c_int;
type A2Hash = unsafe extern "C" fn(u32, u32, u32, *const c_void, usize, *const c_void, usize, *mut c_void, usize, *mut c_char, usize, c_int) -> c_int;
type A2VerifyT = unsafe extern "C" fn(*const c_char, *const c_void, usize, c_int) -> c_int;
type A2Ctx = unsafe extern "C" fn(*mut Argon2Context, c_int) -> c_int;

/// The cheapest legal Argon2 work factors: t_cost 1..3, m_cost 8..16 KiB,
/// lanes/threads 1 (libsodium always uses 1).
const CHEAP: &[(u32, u32)] = &[(1, 8), (2, 8), (3, 8), (1, 16), (3, 16)];

#[test]
fn argon2_internal_hash_raw_and_encoded() {
    let (craw_i, rraw_i) = sym::<A2HashRaw>("_sodium_argon2i_hash_raw");
    let (craw_d, rraw_d) = sym::<A2HashRaw>("_sodium_argon2id_hash_raw");
    let (cenc_i, renc_i) = sym::<A2HashEnc>("_sodium_argon2i_hash_encoded");
    let (cenc_d, renc_d) = sym::<A2HashEnc>("_sodium_argon2id_hash_encoded");
    let (chash, rhash) = sym::<A2Hash>("_sodium_argon2_hash");
    let mut rng = Rng::new(SEED ^ 3);

    for &(t, m) in CHEAP {
        for hashlen in [16usize, 17, 32, 64] {
            for pwdlen in [0usize, 1, 8, 64] {
                let pwd = rng.bytes(pwdlen);
                let salt = rng.bytes(16);
                let pp = if pwdlen == 0 { ptr::null() } else { pwd.as_ptr() as *const c_void };

                for (label, cf, rf) in [
                    ("argon2i_hash_raw", craw_i, rraw_i),
                    ("argon2id_hash_raw", craw_d, rraw_d),
                ] {
                    let mut oc = out_buf(hashlen);
                    let mut or = out_buf(hashlen);
                    let (a, b) = unsafe {
                        (
                            cf(t, m, 1, pp, pwdlen, salt.as_ptr() as _, 16, oc.as_mut_ptr() as _, hashlen),
                            rf(t, m, 1, pp, pwdlen, salt.as_ptr() as _, 16, or.as_mut_ptr() as _, hashlen),
                        )
                    };
                    assert_eq!(a, b, "_sodium_{label}(t={t},m={m},len={hashlen},pw={pwdlen}) rc");
                    // argon2_hash() starts with randombytes_buf(hash, hashlen),
                    // so the payload is only meaningful on success; the canary
                    // must be intact either way.
                    eqb(&format!("_sodium_{label} canary t={t} m={m} len={hashlen}"), &oc[hashlen..], &or[hashlen..]);
                    if a == 0 {
                        eqb(&format!("_sodium_{label} t={t} m={m} len={hashlen} pw={pwdlen}"), &oc, &or);
                    }
                }
                // encoded form: deterministic here because the salt is supplied
                for (label, cf, rf) in [
                    ("argon2i_hash_encoded", cenc_i, renc_i),
                    ("argon2id_hash_encoded", cenc_d, renc_d),
                ] {
                    let mut oc = vec![0u8; 256];
                    let mut or = vec![0u8; 256];
                    unsafe {
                        let a = cf(t, m, 1, pp, pwdlen, salt.as_ptr() as _, 16, hashlen, oc.as_mut_ptr() as _, 256);
                        let b = rf(t, m, 1, pp, pwdlen, salt.as_ptr() as _, 16, hashlen, or.as_mut_ptr() as _, 256);
                        assert_eq!(a, b, "_sodium_{label}(t={t},m={m}) rc");
                    }
                    eqb(&format!("_sodium_{label} t={t} m={m} len={hashlen} pw={pwdlen}"), &oc, &or);
                    // A too-small `encodedlen` is NOT a graceful failure: see
                    // `argon2_hash_encoded_small_buffer_aborts_identically`.
                }
                // the generic _sodium_argon2_hash, both types, raw and encoded
                for ty in [ARGON2_I, ARGON2_ID, 0, 3, -1, 999] {
                    let mut hc = out_buf(hashlen);
                    let mut hr = out_buf(hashlen);
                    let mut ec = vec![0u8; 256];
                    let mut er = vec![0u8; 256];
                    let (a, b) = unsafe {
                        (
                            chash(t, m, 1, pp, pwdlen, salt.as_ptr() as _, 16, hc.as_mut_ptr() as _, hashlen, ec.as_mut_ptr() as _, 256, ty),
                            rhash(t, m, 1, pp, pwdlen, salt.as_ptr() as _, 16, hr.as_mut_ptr() as _, hashlen, er.as_mut_ptr() as _, 256, ty),
                        )
                    };
                    assert_eq!(a, b, "_sodium_argon2_hash(type={ty}) rc");
                    // randombytes_buf(hash, hashlen) on entry: payload only
                    // comparable on success, canary always.
                    eqb(&format!("_sodium_argon2_hash type={ty} canary"), &hc[hashlen..], &hr[hashlen..]);
                    if a == 0 {
                        eqb(&format!("_sodium_argon2_hash type={ty} hash"), &hc, &hr);
                        eqb(&format!("_sodium_argon2_hash type={ty} encoded"), &ec, &er);
                    }
                }
            }
        }
    }
    // Out-of-range work factors: every gate in argon2_validate_inputs that is
    // actually *reachable*.
    //
    // Deliberately NOT included: t_cost = u32::MAX and m_cost = u32::MAX.
    // ARGON2_MAX_TIME and ARGON2_MAX_MEMORY are both 0xFFFFFFFF, so those are
    // *valid* inputs at this layer — `argon2_validate_inputs` accepts them and
    // the C then genuinely attempts 4 billion passes / a 4 TiB allocation. They
    // are not rejections and cannot be run. The reachable over-max rejection is
    // the one the public wrappers impose (`crypto_pwhash_argon2i`'s
    // OPSLIMIT_MAX / MEMLIMIT_MAX gates, covered in t11_pwhash).
    for (t, m, lanes, hashlen) in [
        (0u32, 8u32, 1u32, 32usize),
        (1, 0, 1, 32),
        (1, 7, 1, 32),
        (1, 8, 0, 32),
        (1, 8, 1, 0),
        (1, 8, 1, 15),
        (1, 8, 2, 32),
        (1, 8, ARGON2_MAX_LANES + 1, 32),
        (1, 8, u32::MAX, 32),
    ] {
        let pwd = [1u8, 2, 3, 4];
        let salt = [9u8; 16];
        let mut oc = out_buf(hashlen.max(1));
        let mut or = out_buf(hashlen.max(1));
        let (a, b) = unsafe {
            (
                craw_i(t, m, lanes, pwd.as_ptr() as _, 4, salt.as_ptr() as _, 16, oc.as_mut_ptr() as _, hashlen),
                rraw_i(t, m, lanes, pwd.as_ptr() as _, 4, salt.as_ptr() as _, 16, or.as_mut_ptr() as _, hashlen),
            )
        };
        assert_eq!(a, b, "argon2i_hash_raw invalid (t={t},m={m},lanes={lanes},len={hashlen}) rc");
        eqb(&format!("argon2i_hash_raw invalid canary t={t} m={m} lanes={lanes}"), &oc[hashlen.max(1)..], &or[hashlen.max(1)..]);
        if a == 0 {
            eqb(&format!("argon2i_hash_raw t={t} m={m} lanes={lanes} len={hashlen}"), &oc, &or);
        }
    }
    // short salts (SALT_TOO_SHORT gate)
    for saltlen in [0usize, 1, 7, 8, 15, 16] {
        let pwd = [1u8; 8];
        let salt = [7u8; 16];
        let mut oc = out_buf(32);
        let mut or = out_buf(32);
        let (a, b) = unsafe {
            (
                craw_d(1, 8, 1, pwd.as_ptr() as _, 8, salt.as_ptr() as _, saltlen, oc.as_mut_ptr() as _, 32),
                rraw_d(1, 8, 1, pwd.as_ptr() as _, 8, salt.as_ptr() as _, saltlen, or.as_mut_ptr() as _, 32),
            )
        };
        assert_eq!(a, b, "argon2id_hash_raw saltlen={saltlen} rc");
        eqb(&format!("argon2id_hash_raw saltlen={saltlen} canary"), &oc[32..], &or[32..]);
        if a == 0 {
            eqb(&format!("argon2id_hash_raw saltlen={saltlen}"), &oc, &or);
        }
    }
}

#[test]
fn argon2_internal_verify() {
    let (ci, ri) = sym::<A2Verify>("_sodium_argon2i_verify");
    let (cd, rd) = sym::<A2Verify>("_sodium_argon2id_verify");
    let (cg, rg) = sym::<A2VerifyT>("_sodium_argon2_verify");
    for (label, enc, pw) in fixed_vectors() {
        let e = cstr(enc);
        for pwd in [pw, "wrong", ""] {
            let p = cstr(pwd);
            unsafe {
                let a = ci(e.as_ptr() as _, p.as_ptr() as _, pwd.len());
                let b = ri(e.as_ptr() as _, p.as_ptr() as _, pwd.len());
                assert_eq!(a, b, "_sodium_argon2i_verify({label},{pwd:?})");
                let a = cd(e.as_ptr() as _, p.as_ptr() as _, pwd.len());
                let b = rd(e.as_ptr() as _, p.as_ptr() as _, pwd.len());
                assert_eq!(a, b, "_sodium_argon2id_verify({label},{pwd:?})");
                for ty in [ARGON2_I, ARGON2_ID, 0, 3, -1] {
                    let a = cg(e.as_ptr() as _, p.as_ptr() as _, pwd.len(), ty);
                    let b = rg(e.as_ptr() as _, p.as_ptr() as _, pwd.len(), ty);
                    assert_eq!(a, b, "_sodium_argon2_verify({label},{pwd:?},type={ty})");
                }
            }
        }
    }
    for bad in MALFORMED {
        let e = cstr(bad);
        let p = cstr("password");
        unsafe {
            assert_eq!(
                ci(e.as_ptr() as _, p.as_ptr() as _, 8),
                ri(e.as_ptr() as _, p.as_ptr() as _, 8),
                "_sodium_argon2i_verify({bad:?})"
            );
            assert_eq!(
                cd(e.as_ptr() as _, p.as_ptr() as _, 8),
                rd(e.as_ptr() as _, p.as_ptr() as _, 8),
                "_sodium_argon2id_verify({bad:?})"
            );
            for ty in [ARGON2_I, ARGON2_ID, 7] {
                assert_eq!(
                    cg(e.as_ptr() as _, p.as_ptr() as _, 8, ty),
                    rg(e.as_ptr() as _, p.as_ptr() as _, 8, ty),
                    "_sodium_argon2_verify({bad:?},type={ty})"
                );
            }
        }
    }
}

/// `_sodium_argon2_ctx` is the lowest-level Argon2 entry point: it takes the
/// whole `argon2_context` struct, which is where the secret/ad/lanes/threads
/// axes live. Everything above it (`argon2_hash`, the libsodium wrappers)
/// hard-codes secret = ad = NULL and lanes = threads = 1, so this is the only
/// way to exercise those.
#[test]
fn argon2_internal_ctx() {
    let (c, r) = sym::<A2Ctx>("_sodium_argon2_ctx");
    let mut rng = Rng::new(SEED ^ 4);

    for &(t, m) in CHEAP {
        for outlen in [16u32, 32, 64] {
            for (secretlen, adlen) in [(0u32, 0u32), (8, 0), (0, 8), (16, 32), (32, 200)] {
                for ty in [ARGON2_I, ARGON2_ID, 0, 3, -1] {
                    let mut pwd = rng.bytes(16);
                    let mut salt = rng.bytes(16);
                    let mut secret = rng.bytes(secretlen.max(1) as usize);
                    let mut ad = rng.bytes(adlen.max(1) as usize);
                    let mut oc = out_buf(outlen as usize);
                    let mut or = out_buf(outlen as usize);

                    let mut mk = |out: *mut u8| Argon2Context {
                        out,
                        outlen,
                        pwd: pwd.as_mut_ptr(),
                        pwdlen: 16,
                        salt: salt.as_mut_ptr(),
                        saltlen: 16,
                        secret: if secretlen == 0 { ptr::null_mut() } else { secret.as_mut_ptr() },
                        secretlen,
                        ad: if adlen == 0 { ptr::null_mut() } else { ad.as_mut_ptr() },
                        adlen,
                        t_cost: t,
                        m_cost: m,
                        lanes: 1,
                        threads: 1,
                        flags: 0,
                    };
                    let mut ctxc = mk(oc.as_mut_ptr());
                    let mut ctxr = mk(or.as_mut_ptr());
                    unsafe {
                        let a = c(&mut ctxc, ty);
                        let b = r(&mut ctxr, ty);
                        assert_eq!(
                            a, b,
                            "_sodium_argon2_ctx(t={t},m={m},out={outlen},secret={secretlen},ad={adlen},type={ty}) rc"
                        );
                    }
                    eqb(
                        &format!("_sodium_argon2_ctx t={t} m={m} out={outlen} secret={secretlen} ad={adlen} type={ty}"),
                        &oc,
                        &or,
                    );
                }
            }
        }
    }
    // multi-lane configurations, which no public wrapper can reach
    for lanes in [1u32, 2, 3, 4, 8] {
        for &(t, m) in &[(1u32, 8u32), (2, 32), (3, 64)] {
            let mut pwd = rng.bytes(16);
            let mut salt = rng.bytes(16);
            let mut oc = out_buf(32);
            let mut or = out_buf(32);
            let mut mk = |out: *mut u8| Argon2Context {
                out,
                outlen: 32,
                pwd: pwd.as_mut_ptr(),
                pwdlen: 16,
                salt: salt.as_mut_ptr(),
                saltlen: 16,
                secret: ptr::null_mut(),
                secretlen: 0,
                ad: ptr::null_mut(),
                adlen: 0,
                t_cost: t,
                m_cost: m,
                lanes,
                threads: lanes,
                flags: 0,
            };
            let mut ctxc = mk(oc.as_mut_ptr());
            let mut ctxr = mk(or.as_mut_ptr());
            unsafe {
                let a = c(&mut ctxc, ARGON2_ID);
                let b = r(&mut ctxr, ARGON2_ID);
                assert_eq!(a, b, "_sodium_argon2_ctx lanes={lanes} t={t} m={m} rc");
            }
            eqb(&format!("_sodium_argon2_ctx lanes={lanes} t={t} m={m}"), &oc, &or);
        }
    }
    // NULL out / NULL pwd / NULL salt rejection
    let mut oc = out_buf(32);
    let mut base = Argon2Context {
        out: oc.as_mut_ptr(),
        outlen: 32,
        pwd: ptr::null_mut(),
        pwdlen: 0,
        salt: ptr::null_mut(),
        saltlen: 0,
        secret: ptr::null_mut(),
        secretlen: 0,
        ad: ptr::null_mut(),
        adlen: 0,
        t_cost: 1,
        m_cost: 8,
        lanes: 1,
        threads: 1,
        flags: 0,
    };
    unsafe {
        let a = c(&mut base, ARGON2_ID);
        let b = r(&mut base, ARGON2_ID);
        assert_eq!(a, b, "_sodium_argon2_ctx all-NULL rc");
    }
    // NULL context pointer
    unsafe {
        let a = c(ptr::null_mut(), ARGON2_ID);
        let b = r(ptr::null_mut(), ARGON2_ID);
        assert_eq!(a, b, "_sodium_argon2_ctx(NULL) rc");
    }
}

/// `_sodium_argon2_initialize` / `_fill_memory_blocks` / `_fill_segment_ref` /
/// `_finalize` are the internal pipeline stages. They take an
/// `argon2_instance_t` whose layout is private, so rather than reconstructing
/// it, drive them the way the C does — through `_sodium_argon2_ctx` — and pin
/// down that each is exported by both and that the composed pipeline agrees.
/// (`argon2_internal_ctx` above is that end-to-end check.) Here we only assert
/// export parity and that the symbols are callable addresses in both.
#[test]
fn argon2_internal_pipeline_stages_are_exported() {
    for name in [
        "_sodium_argon2_initialize",
        "_sodium_argon2_fill_memory_blocks",
        "_sodium_argon2_fill_segment_ref",
        "_sodium_argon2_finalize",
        "_sodium_argon2_validate_inputs",
        "_sodium_argon2_encode_string",
        "_sodium_argon2_decode_string",
    ] {
        assert!(has(name), "{name} must be exported by BOTH .so");
        let (c, r) = sym::<*const ()>(name);
        assert!(!c.is_null(), "{name}: null in C .so");
        assert!(!r.is_null(), "{name}: null in Rust .so");
    }
}

// ---------------------------------------------------------------------------
// `_sodium_escrypt_r`
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy)]
struct EscryptRegion {
    base: *mut c_void,
    aligned: *mut c_void,
    size: usize,
}

#[test]
fn escrypt_r_internal() {
    type InitLocal = unsafe extern "C" fn(*mut EscryptRegion) -> c_int;
    type FreeLocal = unsafe extern "C" fn(*mut EscryptRegion) -> c_int;
    type EscryptR = unsafe extern "C" fn(*mut EscryptRegion, *const u8, usize, *const u8, *mut u8, usize) -> *mut u8;
    type GenSalt = unsafe extern "C" fn(u32, u32, u32, *const u8, usize, *mut u8, usize) -> *mut u8;

    let (cinit, rinit) = sym::<InitLocal>("_sodium_escrypt_init_local");
    let (cfree, rfree) = sym::<FreeLocal>("_sodium_escrypt_free_local");
    let (cr, rr) = sym::<EscryptR>("_sodium_escrypt_r");
    let (cgs, rgs) = sym::<GenSalt>("_sodium_escrypt_gensalt_r");
    let mut rng = Rng::new(SEED ^ 5);

    let mut lc = EscryptRegion { base: ptr::null_mut(), aligned: ptr::null_mut(), size: 0 };
    let mut lr = EscryptRegion { base: ptr::null_mut(), aligned: ptr::null_mut(), size: 0 };
    unsafe {
        assert_eq!(cinit(&mut lc), rinit(&mut lr), "escrypt_init_local");
    }

    // Small N_log2 keeps this fast: N = 2^N_log2, and r*p stays tiny.
    for n_log2 in [1u32, 2, 4, 8] {
        for r_ in [1u32, 2, 8] {
            for p in [1u32, 2] {
                let src = rng.bytes(32);
                let mut sc = vec![0u8; 128];
                let mut sr = vec![0u8; 128];
                let (pc, pr) = unsafe {
                    (
                        cgs(n_log2, r_, p, src.as_ptr(), 32, sc.as_mut_ptr(), 128),
                        rgs(n_log2, r_, p, src.as_ptr(), 32, sr.as_mut_ptr(), 128),
                    )
                };
                assert_eq!(pc.is_null(), pr.is_null(), "gensalt_r({n_log2},{r_},{p}) nullness");
                eqb(&format!("gensalt_r({n_log2},{r_},{p})"), &sc, &sr);
                if pc.is_null() {
                    continue;
                }
                for pwlen in [0usize, 1, 8, 64] {
                    let pw = rng.bytes(pwlen);
                    let pp = if pwlen == 0 { ptr::null() } else { pw.as_ptr() };
                    for buflen in [0usize, 1, 32, 64, 128, 256] {
                        let mut bc = out_buf(buflen.max(1));
                        let mut br = out_buf(buflen.max(1));
                        let (oc, or) = unsafe {
                            (
                                cr(&mut lc, pp, pwlen, sc.as_ptr(), bc.as_mut_ptr(), buflen),
                                rr(&mut lr, pp, pwlen, sr.as_ptr(), br.as_mut_ptr(), buflen),
                            )
                        };
                        assert_eq!(
                            oc.is_null(),
                            or.is_null(),
                            "escrypt_r(N=2^{n_log2},r={r_},p={p},pw={pwlen},buf={buflen}) nullness"
                        );
                        // escrypt_r() opens with `randombytes_buf(buf, buflen)`,
                        // so the buffer is deliberately randomised and only the
                        // NUL-terminated result it writes on SUCCESS is
                        // comparable. The canary must survive either way.
                        eqb(
                            &format!("escrypt_r canary N=2^{n_log2} r={r_} p={p} buf={buflen}"),
                            &bc[buflen.max(1)..],
                            &br[buflen.max(1)..],
                        );
                        if !oc.is_null() {
                            let a = CStr::from_bytes_until_nul(&bc).unwrap().to_bytes();
                            let b = CStr::from_bytes_until_nul(&br).unwrap().to_bytes();
                            eqb(
                                &format!("escrypt_r N=2^{n_log2} r={r_} p={p} pw={pwlen} buf={buflen}"),
                                a,
                                b,
                            );
                        }
                    }
                }
            }
        }
    }
    // malformed settings
    for bad in [
        &b"\0"[..],
        &b"$"[..],
        &b"$6$"[..],
        &b"$7$"[..],
        &b"$7$A"[..],
        &b"$7$AAAAAAAAAAA"[..],
        &b"$8$C6..../....salt$"[..],
        &b"garbage"[..],
    ] {
        let mut s = bad.to_vec();
        s.push(0);
        let pw = b"password";
        let mut bc = out_buf(128);
        let mut br = out_buf(128);
        let (oc, or) = unsafe {
            (
                cr(&mut lc, pw.as_ptr(), 8, s.as_ptr(), bc.as_mut_ptr(), 128),
                rr(&mut lr, pw.as_ptr(), 8, s.as_ptr(), br.as_mut_ptr(), 128),
            )
        };
        assert_eq!(oc.is_null(), or.is_null(), "escrypt_r bad setting {bad:?} nullness");
        eqb(&format!("escrypt_r bad setting {bad:?} canary"), &bc[128..], &br[128..]);
        if !oc.is_null() {
            let a = CStr::from_bytes_until_nul(&bc).unwrap().to_bytes();
            let b = CStr::from_bytes_until_nul(&br).unwrap().to_bytes();
            eqb(&format!("escrypt_r bad setting {bad:?}"), a, b);
        }
    }
    unsafe {
        assert_eq!(cfree(&mut lc), rfree(&mut lr), "escrypt_free_local");
    }
}

/// `argon2*_hash_encoded` with a too-small `encodedlen` does NOT fail
/// gracefully. `argon2_encode_string` writes the base64 fields with
/// `sodium_bin2base64(dst, dst_len, ...)`, and `sodium_bin2base64` calls
/// `sodium_misuse()` — it never returns NULL — when the destination is too
/// small. So an `encodedlen` large enough for the `$argon2i$v=19$m=..,t=..,p=..$`
/// prefix but too small for the base64 salt terminates the process.
///
/// This is a real, reachable rejection that the happy-path view of the API hides
/// (the `SS(...)` macro's own `pp_len >= dst_len` check makes the SMALLEST
/// capacities return `ARGON2_ENCODING_FAIL` cleanly, so only the middle band
/// aborts — the boundary is exactly what this test pins down).
#[test]
fn argon2_hash_encoded_small_buffer_aborts_identically() {
    for name in ["_sodium_argon2i_hash_encoded", "_sodium_argon2id_hash_encoded"] {
        for cap in [
            0usize, 1, 2, 8, 12, 16, 20, 24, 26, 27, 28, 30, 32, 40, 48, 56, 60, 64, 70, 72, 76, 80,
            96, 128, 256,
        ] {
            let n1 = name.to_string();
            let n2 = name.to_string();
            same_outcome(
                &format!("{name} encodedlen={cap}"),
                move || {
                    let (c, _) = sym::<A2HashEnc>(&n1);
                    let pwd = [1u8; 8];
                    let salt = [7u8; 16];
                    let mut o = vec![0u8; cap + 64];
                    unsafe {
                        c(1, 8, 1, pwd.as_ptr() as _, 8, salt.as_ptr() as _, 16, 32, o.as_mut_ptr() as _, cap)
                    }
                },
                move || {
                    let (_, r) = sym::<A2HashEnc>(&n2);
                    let pwd = [1u8; 8];
                    let salt = [7u8; 16];
                    let mut o = vec![0u8; cap + 64];
                    unsafe {
                        r(1, 8, 1, pwd.as_ptr() as _, 8, salt.as_ptr() as _, 16, 32, o.as_mut_ptr() as _, cap)
                    }
                },
            );
        }
    }
    // Same via the generic entry point.
    for cap in [0usize, 8, 26, 32, 48, 64, 80, 256] {
        for ty in [ARGON2_I, ARGON2_ID] {
            same_outcome(
                &format!("_sodium_argon2_hash encodedlen={cap} type={ty}"),
                move || {
                    let (c, _) = sym::<A2Hash>("_sodium_argon2_hash");
                    let pwd = [1u8; 8];
                    let salt = [7u8; 16];
                    let mut h = vec![0u8; 32];
                    let mut e = vec![0u8; cap + 64];
                    unsafe {
                        c(1, 8, 1, pwd.as_ptr() as _, 8, salt.as_ptr() as _, 16,
                          h.as_mut_ptr() as _, 32, e.as_mut_ptr() as _, cap, ty)
                    }
                },
                move || {
                    let (_, r) = sym::<A2Hash>("_sodium_argon2_hash");
                    let pwd = [1u8; 8];
                    let salt = [7u8; 16];
                    let mut h = vec![0u8; 32];
                    let mut e = vec![0u8; cap + 64];
                    unsafe {
                        r(1, 8, 1, pwd.as_ptr() as _, 8, salt.as_ptr() as _, 16,
                          h.as_mut_ptr() as _, 32, e.as_mut_ptr() as _, cap, ty)
                    }
                },
            );
        }
    }
}
