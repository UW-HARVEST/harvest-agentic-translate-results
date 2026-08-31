//! Area 8 (part 1c) — `crypto_pwhash/argon2/argon2-encoding.c`:
//! `argon2_encode_string` / `argon2_decode_string` and the `decode_decimal`
//! helper they share.
//!
//! Covers `configs_8.md` rows 8.76 – 8.86 and `errors_8.md` rows 8.112 – 8.147.
//! (Row 8.148 — the `SB` / `sodium_bin2base64` buffer-overflow path — lives in
//! `a8_argon2_core.rs`, because in libsodium 1.0.23 `sodium_bin2base64` calls
//! `sodium_misuse()` instead of returning NULL and the test has to fork.)
mod common;
use common::*;
use std::ffi::{c_char, c_int};
use std::ptr::null_mut;

// ------------------------------------------------------------------- types

/// `argon2_context` from `argon2.h`, field for field.
#[repr(C)]
#[derive(Clone, Copy)]
struct Ctx {
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

type EncodeString = unsafe extern "C" fn(*mut c_char, usize, *mut Ctx, c_int) -> c_int;
type DecodeString = unsafe extern "C" fn(*mut Ctx, *const c_char, c_int) -> c_int;

const ARGON2_OK: c_int = 0;
const OUTPUT_TOO_SHORT: c_int = -2;
const SALT_TOO_SHORT: c_int = -6;
const PWD_PTR_MISMATCH: c_int = -18;
const TIME_TOO_SMALL: c_int = -12;
const MEMORY_TOO_LITTLE: c_int = -14;
const LANES_TOO_FEW: c_int = -16;
const INCORRECT_TYPE: c_int = -26;
const ENCODING_FAIL: c_int = -31;
const DECODING_FAIL: c_int = -32;

const T_I: c_int = 1;
const T_ID: c_int = 2;

// ----------------------------------------------------------------- helpers

fn b64(v: &[u8]) -> String {
    const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut s = String::new();
    let mut i = 0;
    while i + 3 <= v.len() {
        let n = ((v[i] as u32) << 16) | ((v[i + 1] as u32) << 8) | v[i + 2] as u32;
        for sh in [18, 12, 6, 0] {
            s.push(A[(n >> sh) as usize & 63] as char);
        }
        i += 3;
    }
    match v.len() - i {
        1 => {
            let n = (v[i] as u32) << 16;
            s.push(A[(n >> 18) as usize & 63] as char);
            s.push(A[(n >> 12) as usize & 63] as char);
        }
        2 => {
            let n = ((v[i] as u32) << 16) | ((v[i + 1] as u32) << 8);
            for sh in [18, 12, 6] {
                s.push(A[(n >> sh) as usize & 63] as char);
            }
        }
        _ => {}
    }
    s
}

fn cstr(s: &str) -> Vec<u8> {
    let mut v = s.as_bytes().to_vec();
    v.push(0);
    v
}

fn as_str(b: &[u8]) -> String {
    let end = b.iter().position(|&x| x == 0).unwrap_or(b.len());
    String::from_utf8_lossy(&b[..end]).into_owned()
}

fn kind(ty: c_int) -> &'static str {
    if ty == T_I {
        "argon2i"
    } else {
        "argon2id"
    }
}

/// The canonical encoding of a parameter set, as `argon2_encode_string`
/// produces it.
fn canonical(ty: c_int, m: u32, t: u32, p: u32, salt: &[u8], out: &[u8]) -> String {
    format!(
        "${}$v=19$m={m},t={t},p={p}${}${}",
        kind(ty),
        b64(salt),
        b64(out)
    )
}

/// Everything `argon2_encode_string` produces, on both libraries.
#[track_caller]
#[allow(clippy::too_many_arguments)]
fn encode(
    c: &EncodeString,
    r: &EncodeString,
    label: &str,
    dst_len: usize,
    m: u32,
    t: u32,
    lanes: u32,
    threads: u32,
    salt: &[u8],
    out: &[u8],
    ty: c_int,
) -> (c_int, Vec<u8>) {
    let run = |f: &EncodeString| -> (c_int, Vec<u8>) {
        let mut salt_v = salt.to_vec();
        let mut out_v = out.to_vec();
        let mut dst = padded(dst_len.max(1));
        for i in 0..dst_len.max(1) {
            dst[i] = 0xC3;
        }
        let mut ctx = Ctx {
            out: out_v.as_mut_ptr(),
            outlen: out_v.len() as u32,
            pwd: null_mut(),
            pwdlen: 0,
            salt: salt_v.as_mut_ptr(),
            saltlen: salt_v.len() as u32,
            secret: null_mut(),
            secretlen: 0,
            ad: null_mut(),
            adlen: 0,
            t_cost: t,
            m_cost: m,
            lanes,
            threads,
            flags: 0,
        };
        let ret = unsafe { f(dst.as_mut_ptr() as *mut c_char, dst_len, &mut ctx, ty) };
        check_pad(label, &dst, dst_len.max(1));
        (ret, dst[..dst_len.max(1)].to_vec())
    };
    let (a, ab) = run(c);
    let (b, bb) = run(r);
    eqi(&format!("{label} ret"), a, b);
    eqb(&format!("{label} dst"), &ab, &bb);
    (a, ab)
}

/// Everything `argon2_decode_string` produces, on both libraries.
struct Decoded {
    ret: c_int,
    m_cost: u32,
    t_cost: u32,
    lanes: u32,
    threads: u32,
    salt: Vec<u8>,
    saltlen: u32,
    out: Vec<u8>,
    outlen: u32,
}

#[track_caller]
fn decode(
    c: &DecodeString,
    r: &DecodeString,
    label: &str,
    s: &str,
    maxsalt: usize,
    maxout: usize,
    ty: c_int,
) -> Decoded {
    let z = cstr(s);
    let run = |f: &DecodeString| -> Decoded {
        let mut salt = padded(maxsalt.max(1));
        let mut out = padded(maxout.max(1));
        let mut ctx = Ctx {
            out: out.as_mut_ptr(),
            outlen: maxout as u32,
            pwd: null_mut(),
            pwdlen: 0,
            salt: salt.as_mut_ptr(),
            saltlen: maxsalt as u32,
            secret: null_mut(),
            secretlen: 0,
            ad: null_mut(),
            adlen: 0,
            t_cost: 0xDEAD_BEEF,
            m_cost: 0xDEAD_BEEF,
            lanes: 0xDEAD_BEEF,
            threads: 0xDEAD_BEEF,
            flags: 0,
        };
        let ret = unsafe { f(&mut ctx, z.as_ptr() as *const c_char, ty) };
        check_pad(&format!("{label} salt"), &salt, maxsalt.max(1));
        check_pad(&format!("{label} out"), &out, maxout.max(1));
        Decoded {
            ret,
            m_cost: ctx.m_cost,
            t_cost: ctx.t_cost,
            lanes: ctx.lanes,
            threads: ctx.threads,
            salt: salt[..maxsalt.max(1)].to_vec(),
            saltlen: ctx.saltlen,
            out: out[..maxout.max(1)].to_vec(),
            outlen: ctx.outlen,
        }
    };
    let a = run(c);
    let b = run(r);
    eqi(&format!("{label} ret"), a.ret, b.ret);
    assert_eq!(a.m_cost, b.m_cost, "{label}: ctx.m_cost mismatch");
    assert_eq!(a.t_cost, b.t_cost, "{label}: ctx.t_cost mismatch");
    assert_eq!(a.lanes, b.lanes, "{label}: ctx.lanes mismatch");
    assert_eq!(a.threads, b.threads, "{label}: ctx.threads mismatch");
    assert_eq!(a.saltlen, b.saltlen, "{label}: ctx.saltlen mismatch");
    assert_eq!(a.outlen, b.outlen, "{label}: ctx.outlen mismatch");
    eqb(&format!("{label} salt buffer"), &a.salt, &b.salt);
    eqb(&format!("{label} out buffer"), &a.out, &b.out);
    a
}

// =================== 8.76 – 8.81 argon2_encode_string ====================

#[test]
fn r8_76_to_79_encode_string() {
    let (c, r) = both::<EncodeString>("_sodium_argon2_encode_string");
    let (cd, rd) = both::<DecodeString>("_sodium_argon2_decode_string");

    // 8.76 Argon2_i, m_cost 8, t_cost 1, lanes 1, saltlen 8, outlen 16.
    let salt8: Vec<u8> = (0x41u8..0x49).collect();
    let out16: Vec<u8> = (0x61u8..0x71).collect();
    let want = canonical(T_I, 8, 1, 1, &salt8, &out16);
    let (ret, dst) = encode(&c, &r, "8.76", 128, 8, 1, 1, 1, &salt8, &out16, T_I);
    assert_eq!(ret, ARGON2_OK);
    assert_eq!(as_str(&dst), want, "8.76: unexpected encoding");
    assert_eq!(want, "$argon2i$v=19$m=8,t=1,p=1$QUJDREVGR0g$YWJjZGVmZ2hpamtsbW5vcA");
    // ... and argon2_decode_string recovers all four parameters unchanged.
    let d = decode(&cd, &rd, "8.76 decode", &want, 64, 64, T_I);
    assert_eq!(d.ret, ARGON2_OK);
    assert_eq!((d.m_cost, d.t_cost, d.lanes), (8, 1, 1));
    assert_eq!(d.threads, d.lanes, "8.82: threads must be copied from lanes");
    assert_eq!(d.saltlen, 8);
    assert_eq!(d.outlen, 16);
    eqb("8.76 salt round trip", &salt8, &d.salt[..8]);
    eqb("8.76 out round trip", &out16, &d.out[..16]);

    // 8.77 Argon2_id, m_cost 65536, t_cost 3, lanes 1, saltlen 16, outlen 32.
    let salt16: Vec<u8> = (0x30u8..0x40).collect();
    let out32: Vec<u8> = (0x80u8..0xA0).collect();
    let want = canonical(T_ID, 65536, 3, 1, &salt16, &out32);
    let (ret, dst) = encode(&c, &r, "8.77", 128, 65536, 3, 1, 1, &salt16, &out32, T_ID);
    assert_eq!(ret, ARGON2_OK);
    assert_eq!(as_str(&dst), want);
    assert!(want.starts_with("$argon2id$v=19$m=65536,t=3,p=1$"));
    let d = decode(&cd, &rd, "8.77 decode", &want, 64, 64, T_ID);
    assert_eq!(d.ret, ARGON2_OK);
    assert_eq!((d.m_cost, d.t_cost, d.lanes, d.threads), (65536, 3, 1, 1));

    // 8.78 maximum-width decimal fields: m_cost / t_cost = 4294967295 (10
    // digits) and lanes = 16777215 (8 digits).  The result must still fit into
    // 128 bytes and round trip.
    let want = canonical(T_ID, 4294967295, 4294967295, 16777215, &salt16, &out32);
    assert!(want.len() < 128, "8.78: {} chars", want.len());
    let (ret, dst) = encode(
        &c, &r, "8.78", 128, 4294967295, 4294967295, 16777215, 1, &salt16, &out32, T_ID,
    );
    assert_eq!(ret, ARGON2_OK, "8.78: max-width fields must encode");
    assert_eq!(as_str(&dst), want);
    let d = decode(&cd, &rd, "8.78 decode", &want, 64, 64, T_ID);
    assert_eq!(d.ret, ARGON2_OK);
    assert_eq!(
        (d.m_cost, d.t_cost, d.lanes, d.threads),
        (4294967295, 4294967295, 16777215, 16777215)
    );

    // 8.79 dst_len exactly strlen(expected) + 1 is the tightest accepting size,
    // for both types.  One byte less aborts inside sodium_bin2base64 (see
    // a8_argon2_core::e8_148_encoding_buffer_aborts), so only the accepting
    // size is exercised here.
    for ty in [T_I, T_ID] {
        let want = canonical(ty, 8, 1, 1, &salt8, &out16);
        let (ret, dst) = encode(
            &c, &r, &format!("8.79 {} tight", kind(ty)), want.len() + 1, 8, 1, 1, 1, &salt8,
            &out16, ty,
        );
        assert_eq!(ret, ARGON2_OK, "8.79 {}: dst_len = strlen+1 must be accepted", kind(ty));
        assert_eq!(as_str(&dst), want);
    }
}

#[test]
fn r8_80_81_base64_alphabet_and_lengths() {
    let (c, r) = both::<EncodeString>("_sodium_argon2_encode_string");
    let (cd, rd) = both::<DecodeString>("_sodium_argon2_decode_string");

    // 8.80 byte patterns that exercise the non-URL-safe alphabet: '+' (62) and
    // '/' (63) must both appear, and no '=' padding may be emitted.
    let mut rng = Rng::new(0x8_0080);
    let mut found = 0;
    for _ in 0..200 {
        let salt = rng.bytes(16);
        let out = rng.bytes(32);
        let s_b64 = b64(&salt);
        let o_b64 = b64(&out);
        if !(s_b64.contains('+') || o_b64.contains('+')) {
            continue;
        }
        if !(s_b64.contains('/') || o_b64.contains('/')) {
            continue;
        }
        let want = canonical(T_ID, 8, 1, 1, &salt, &out);
        let (ret, dst) = encode(&c, &r, "8.80 +/ alphabet", 128, 8, 1, 1, 1, &salt, &out, T_ID);
        assert_eq!(ret, ARGON2_OK);
        assert_eq!(as_str(&dst), want);
        assert!(
            !s_b64.contains('=') && !o_b64.contains('='),
            "8.80: ORIGINAL_NO_PADDING must not emit '='"
        );
        let d = decode(&cd, &rd, "8.80 decode", &want, 64, 64, T_ID);
        assert_eq!(d.ret, ARGON2_OK);
        eqb("8.80 salt round trip", &salt, &d.salt[..16]);
        eqb("8.80 out round trip", &out, &d.out[..32]);
        found += 1;
        if found == 4 {
            break;
        }
    }
    assert!(found > 0, "8.80: no salt/out pair containing both '+' and '/'");

    // Deterministic '+' / '/' witnesses: 0xFB 0xEF 0xBE -> "++++"-ish groups.
    for salt in [
        vec![0xFBu8, 0xEF, 0xBE, 0xFB, 0xEF, 0xBE, 0xFB, 0xEF],
        vec![0xFFu8; 9],
    ] {
        let out: Vec<u8> = vec![0xFF; 16];
        let want = canonical(T_ID, 8, 1, 1, &salt, &out);
        let (ret, dst) = encode(&c, &r, "8.80 fixed", 128, 8, 1, 1, 1, &salt, &out, T_ID);
        assert_eq!(ret, ARGON2_OK);
        assert_eq!(as_str(&dst), want);
    }

    // 8.81 saltlen % 3 in {0, 1, 2}: 9 bytes -> 12 chars (no leftover bits),
    // 16 -> 22 chars (2 leftover bits), 8 -> 11 chars (4 leftover bits).
    // All must round trip.
    for saltlen in [8usize, 9, 10, 16, 17, 18, 32, 33] {
        let salt = rng.bytes(saltlen);
        let out = rng.bytes(32);
        let want = canonical(T_ID, 8, 1, 1, &salt, &out);
        let expect_chars = (saltlen * 8 + 5) / 6;
        assert_eq!(b64(&salt).len(), expect_chars, "8.81 saltlen={saltlen}");
        let (ret, dst) = encode(
            &c, &r, &format!("8.81 saltlen={saltlen}"), 128, 8, 1, 1, 1, &salt, &out, T_ID,
        );
        assert_eq!(ret, ARGON2_OK);
        assert_eq!(as_str(&dst), want);
        let d = decode(
            &cd, &rd, &format!("8.81 decode saltlen={saltlen}"), &want, 64, 64, T_ID,
        );
        assert_eq!(d.ret, ARGON2_OK, "8.81 saltlen={saltlen}");
        assert_eq!(d.saltlen, saltlen as u32);
        eqb("8.81 salt round trip", &salt, &d.salt[..saltlen]);
    }
}

// =================== 8.82 – 8.86 argon2_decode_string ====================

#[test]
fn r8_82_to_86_decode_string() {
    let (ce, re) = both::<EncodeString>("_sodium_argon2_encode_string");
    let (c, r) = both::<DecodeString>("_sodium_argon2_decode_string");
    let salt8: Vec<u8> = (0x41u8..0x49).collect();
    let out16: Vec<u8> = (0x61u8..0x71).collect();
    let salt16: Vec<u8> = (0x30u8..0x40).collect();
    let out32: Vec<u8> = (0x80u8..0xA0).collect();

    // 8.82 buffer capacities generously larger than the encoded sizes; after a
    // successful decode ctx.threads == ctx.lanes.
    let s = canonical(T_I, 8, 1, 1, &salt8, &out16);
    for (maxsalt, maxout) in [(64usize, 64usize), (128, 128), (8, 16)] {
        let d = decode(
            &c, &r, &format!("8.82/8.83 maxsalt={maxsalt} maxout={maxout}"), &s, maxsalt, maxout,
            T_I,
        );
        assert_eq!(d.ret, ARGON2_OK, "8.82/8.83 maxsalt={maxsalt} maxout={maxout}");
        assert_eq!(d.threads, d.lanes);
        assert_eq!((d.saltlen, d.outlen), (8, 16));
        eqb("8.82 salt", &salt8, &d.salt[..8]);
        eqb("8.82 out", &out16, &d.out[..16]);
    }
    // 8.83 the capacities set *exactly* to the decoded sizes (8 and 16) is the
    // tightest accepting shape; one byte less and the base64 decoder overruns.
    for (maxsalt, maxout) in [(7usize, 16usize), (8, 15)] {
        let d = decode(
            &c, &r, &format!("8.127/8.132 maxsalt={maxsalt} maxout={maxout}"), &s, maxsalt,
            maxout, T_I,
        );
        assert_eq!(
            d.ret, DECODING_FAIL,
            "8.127/8.132 maxsalt={maxsalt} maxout={maxout}: expected -32"
        );
    }

    // 8.84 multi-digit m and p > 1; also every field at its minimal decimal.
    let s = canonical(T_ID, 65536, 2, 4, &salt16, &out32);
    let d = decode(&c, &r, "8.84 m=65536,t=2,p=4", &s, 64, 64, T_ID);
    assert_eq!(d.ret, ARGON2_OK);
    assert_eq!((d.m_cost, d.t_cost, d.lanes, d.threads), (65536, 2, 4, 4));
    let s = canonical(T_ID, 8, 1, 1, &salt16, &out32);
    let d = decode(&c, &r, "8.84 m=8,t=1,p=1", &s, 64, 64, T_ID);
    assert_eq!(d.ret, ARGON2_OK);
    assert_eq!((d.m_cost, d.t_cost, d.lanes), (8, 1, 1));

    // 8.85 the version field is mandatory and must be exactly v=19.
    for ty in [T_I, T_ID] {
        let s = canonical(ty, 8, 1, 1, &salt16, &out32);
        assert!(s.contains("$v=19$"));
        let d = decode(&c, &r, &format!("8.85 {} v=19", kind(ty)), &s, 64, 64, ty);
        assert_eq!(d.ret, ARGON2_OK);
        // Dropping the version field is a decoding failure (errors row 8.115).
        let without = s.replacen("v=19$", "", 1);
        let d = decode(&c, &r, &format!("8.115 {} no version", kind(ty)), &without, 64, 64, ty);
        assert_eq!(d.ret, DECODING_FAIL);
    }

    // 8.86 decode then re-encode must reproduce the canonical string byte for
    // byte, for both types.
    for ty in [T_I, T_ID] {
        for (m, t, p, salt, out) in [
            (8u32, 1u32, 1u32, salt8.clone(), out16.clone()),
            (16, 2, 2, salt16.clone(), out32.clone()),
            (65536, 3, 1, salt16.clone(), out32.clone()),
            (4294967295, 4294967295, 16777215, salt16.clone(), out32.clone()),
        ] {
            let s = canonical(ty, m, t, p, &salt, &out);
            let d = decode(
                &c, &r, &format!("8.86 decode {} m={m} t={t} p={p}", kind(ty)), &s, 64, 64, ty,
            );
            assert_eq!(d.ret, ARGON2_OK, "8.86 {} m={m} t={t} p={p}", kind(ty));
            let (ret, dst) = encode(
                &ce, &re, &format!("8.86 re-encode {}", kind(ty)), 256, d.m_cost, d.t_cost,
                d.lanes, d.threads, &d.salt[..d.saltlen as usize], &d.out[..d.outlen as usize],
                ty,
            );
            assert_eq!(ret, ARGON2_OK);
            assert_eq!(as_str(&dst), s, "8.86: canonical round trip broke");
        }
    }
}

// ============================ error surface =============================

#[test]
fn e8_112_to_118_decode_prefix_and_version() {
    let (c, r) = both::<DecodeString>("_sodium_argon2_decode_string");
    let salt16: Vec<u8> = (0x30u8..0x40).collect();
    let out32: Vec<u8> = (0x80u8..0xA0).collect();
    let good_i = canonical(T_I, 8, 1, 1, &salt16, &out32);
    let good_id = canonical(T_ID, 8, 1, 1, &salt16, &out32);
    let sb = b64(&salt16);
    let ob = b64(&out32);

    // 8.112 a `type` that is neither Argon2_i nor Argon2_id.
    for ty in [0, 3, 4, -1, c_int::MAX, c_int::MIN] {
        let d = decode(&c, &r, &format!("8.112 type={ty}"), &good_id, 64, 64, ty);
        assert_eq!(d.ret, INCORRECT_TYPE, "8.112 type={ty}: expected -26");
    }

    // 8.113 the wrong type prefix.
    let d = decode(&c, &r, "8.113 argon2id decoded as Argon2_i", &good_id, 64, 64, T_I);
    assert_eq!(d.ret, DECODING_FAIL);
    let d = decode(&c, &r, "8.113 argon2i decoded as Argon2_id", &good_i, 64, 64, T_ID);
    assert_eq!(d.ret, DECODING_FAIL);

    // 8.114 prefix garbage.
    for s in [
        "",
        "$",
        "$argon",
        "$argon2",
        "$argon2i",
        &format!("argon2i$v=19$m=8,t=1,p=1${sb}${ob}"),
        &format!("$argon2d$v=19$m=8,t=1,p=1${sb}${ob}"),
        &format!("$ARGON2I$v=19$m=8,t=1,p=1${sb}${ob}"),
        &format!(" $argon2i$v=19$m=8,t=1,p=1${sb}${ob}"),
        &format!("$$argon2i$v=19$m=8,t=1,p=1${sb}${ob}"),
    ] {
        let d = decode(&c, &r, &format!("8.114 {s:?}"), s, 64, 64, T_I);
        assert_eq!(d.ret, DECODING_FAIL, "8.114 {s:?}: expected -32");
    }

    // 8.115 the `$v=` field is missing.
    for s in [
        format!("$argon2id$m=8,t=1,p=1${sb}${ob}"),
        format!("$argon2id$19$m=8,t=1,p=1${sb}${ob}"),
        format!("$argon2id$v19$m=8,t=1,p=1${sb}${ob}"),
        format!("$argon2id$V=19$m=8,t=1,p=1${sb}${ob}"),
    ] {
        let d = decode(&c, &r, &format!("8.115 {s:?}"), &s, 64, 64, T_ID);
        assert_eq!(d.ret, DECODING_FAIL, "8.115 {s:?}: expected -32");
    }

    // 8.116 / 8.117 the version is not a minimal decimal, or is out of range.
    for s in [
        format!("$argon2id$v=$m=8,t=1,p=1${sb}${ob}"),
        format!("$argon2id$v=019$m=8,t=1,p=1${sb}${ob}"),
        format!("$argon2id$v=+19$m=8,t=1,p=1${sb}${ob}"),
        format!("$argon2id$v=-19$m=8,t=1,p=1${sb}${ob}"),
        format!("$argon2id$v= 19$m=8,t=1,p=1${sb}${ob}"),
        // 8.117 > UINT32_MAX, and long enough to overflow `unsigned long`.
        format!("$argon2id$v=4294967296$m=8,t=1,p=1${sb}${ob}"),
        format!("$argon2id$v=99999999999999999999999999$m=8,t=1,p=1${sb}${ob}"),
    ] {
        let d = decode(&c, &r, &format!("8.116/8.117 {s:?}"), &s, 64, 64, T_ID);
        assert_eq!(d.ret, DECODING_FAIL, "8.116/8.117 {s:?}: expected -32");
    }

    // 8.118 a well-formed but wrong version -> ARGON2_INCORRECT_TYPE, not
    // DECODING_FAIL.
    for v in [0u32, 1, 16, 18, 20, 4294967295] {
        let s = format!("$argon2id$v={v}$m=8,t=1,p=1${sb}${ob}");
        let d = decode(&c, &r, &format!("8.118 v={v}"), &s, 64, 64, T_ID);
        assert_eq!(d.ret, INCORRECT_TYPE, "8.118 v={v}: expected -26");
    }
    // "v=1a9" is *not* a DECODING_FAIL: decode_decimal stops at 'a' with the
    // value 1, and the version check fires before the following CC("$m=").
    // (errors row 8.116 predicts -32 for this shape; the C code returns -26.)
    let s = format!("$argon2id$v=1a9$m=8,t=1,p=1${sb}${ob}");
    let d = decode(&c, &r, "8.116 v=1a9", &s, 64, 64, T_ID);
    assert_eq!(d.ret, INCORRECT_TYPE, "8.116: v=1a9 parses as version 1");
    // A version that *does* parse as 19 but has trailing junk fails the CC.
    let s = format!("$argon2id$v=19x$m=8,t=1,p=1${sb}${ob}");
    let d = decode(&c, &r, "8.116 v=19x", &s, 64, 64, T_ID);
    assert_eq!(d.ret, DECODING_FAIL, "8.116: trailing junk after v=19");

    // v=19 succeeds.
    let d = decode(&c, &r, "8.118 v=19", &good_id, 64, 64, T_ID);
    assert_eq!(d.ret, ARGON2_OK);
}

#[test]
fn e8_119_to_126_decode_parameter_fields() {
    let (c, r) = both::<DecodeString>("_sodium_argon2_decode_string");
    let salt16: Vec<u8> = (0x30u8..0x40).collect();
    let out32: Vec<u8> = (0x80u8..0xA0).collect();
    let sb = b64(&salt16);
    let ob = b64(&out32);

    let mut bad: Vec<String> = Vec::new();
    // 8.119 `$m=` missing after the version.
    bad.push(format!("$argon2id$v=19$t=1,p=1${sb}${ob}"));
    bad.push("$argon2id$v=19".to_string());
    bad.push("$argon2id$v=19$".to_string());
    bad.push(format!("$argon2id$v=19$M=8,t=1,p=1${sb}${ob}"));
    bad.push(format!("$argon2id$v=19$m8,t=1,p=1${sb}${ob}"));
    // 8.120 a bad `m=` value.
    for m in ["", "08", "4294967296", "-8", "+8", " 8", "8x", "0x8"] {
        bad.push(format!("$argon2id$v=19$m={m},t=1,p=1${sb}${ob}"));
    }
    // 8.121 `,t=` missing.
    bad.push(format!("$argon2id$v=19$m=8${sb}${ob}"));
    bad.push(format!("$argon2id$v=19$m=8;t=1,p=1${sb}${ob}"));
    bad.push(format!("$argon2id$v=19$m=8,T=1,p=1${sb}${ob}"));
    bad.push("$argon2id$v=19$m=8".to_string());
    // 8.122 a bad `t=` value.
    for t in ["", "01", "4294967296", "-1", "+1"] {
        bad.push(format!("$argon2id$v=19$m=8,t={t},p=1${sb}${ob}"));
    }
    // 8.123 `,p=` missing.
    bad.push(format!("$argon2id$v=19$m=8,t=1${sb}${ob}"));
    bad.push(format!("$argon2id$v=19$m=8,t=1;p=1${sb}${ob}"));
    bad.push("$argon2id$v=19$m=8,t=1".to_string());
    // 8.124 a bad `p=` value.
    for p in ["", "01", "4294967296", "-1", "+1"] {
        bad.push(format!("$argon2id$v=19$m=8,t=1,p={p}${sb}${ob}"));
    }
    // 8.126 the `$` before the salt is missing.
    bad.push(format!("$argon2id$v=19$m=8,t=1,p=1{sb}${ob}"));
    bad.push(format!("$argon2id$v=19$m=8,t=1,p=1,{sb}${ob}"));
    // 8.131 the `$` between salt and hash is missing: the salt base64 consumes
    // both fields and the following CC("$") sees the NUL.
    bad.push(format!("$argon2id$v=19$m=8,t=1,p=1${sb}{ob}"));
    // 8.139 trailing garbage after the hash.
    bad.push(format!("$argon2id$v=19$m=8,t=1,p=1${sb}${ob}$"));
    bad.push(format!("$argon2id$v=19$m=8,t=1,p=1${sb}${ob}!"));
    bad.push(format!("$argon2id$v=19$m=8,t=1,p=1${sb}${ob}-"));
    bad.push(format!("$argon2id$v=19$m=8,t=1,p=1${sb}${ob}\n"));
    bad.push(format!("$argon2id$v=19$m=8,t=1,p=1${sb}${ob} "));

    for s in &bad {
        let d = decode(&c, &r, &format!("8.119-8.139 {s:?}"), s, 64, 64, T_ID);
        assert_eq!(d.ret, DECODING_FAIL, "8.119-8.139 {s:?}: expected -32");
    }

    // 8.125 the three `> UINT32_MAX` guards after each DECIMAL_U32 are dead
    // code (the values are already uint32_t), so the maximum values decode.
    let s = format!("$argon2id$v=19$m=4294967295,t=4294967295,p=16777215${sb}${ob}");
    let d = decode(&c, &r, "8.125 max fields", &s, 64, 64, T_ID);
    assert_eq!(d.ret, ARGON2_OK, "8.125: maximum uint32 fields must decode");
    assert_eq!(
        (d.m_cost, d.t_cost, d.lanes, d.threads),
        (4294967295, 4294967295, 16777215, 16777215)
    );
}

#[test]
fn e8_127_to_143_decode_base64_and_validation() {
    let (c, r) = both::<DecodeString>("_sodium_argon2_decode_string");
    let salt16: Vec<u8> = (0x30u8..0x40).collect();
    let out32: Vec<u8> = (0x80u8..0xA0).collect();
    let sb = b64(&salt16);
    let ob = b64(&out32);

    // 8.127 the salt decodes to more than `ctx->saltlen` bytes on entry.
    for maxsalt in [0usize, 1, 8, 15] {
        let s = format!("$argon2id$v=19$m=8,t=1,p=1${sb}${ob}");
        let d = decode(&c, &r, &format!("8.127 maxsalt={maxsalt}"), &s, maxsalt, 64, T_ID);
        assert_eq!(d.ret, DECODING_FAIL, "8.127 maxsalt={maxsalt}: expected -32");
    }
    // ... and exactly 16 is accepted.
    let s = format!("$argon2id$v=19$m=8,t=1,p=1${sb}${ob}");
    let d = decode(&c, &r, "8.127 maxsalt=16", &s, 16, 64, T_ID);
    assert_eq!(d.ret, ARGON2_OK);

    // 8.128 invalid trailing bits in the salt base64, and '=' padding (the
    // ORIGINAL_NO_PADDING variant rejects it).
    for bad_salt in ["c29tZQ==", "c29tZQ=", "QQ==", "Q", "QUJDREVGR0g=", "QUJDREVGR0h!"] {
        let s = format!("$argon2id$v=19$m=8,t=1,p=1${bad_salt}${ob}");
        let d = decode(&c, &r, &format!("8.128 salt={bad_salt:?}"), &s, 64, 64, T_ID);
        assert_eq!(d.ret, DECODING_FAIL, "8.128 salt={bad_salt:?}: expected -32");
    }

    // 8.129 an empty salt field -> saltlen 0 with salt != NULL, caught by the
    // final argon2_validate_inputs.
    let s = format!("$argon2id$v=19$m=8,t=1,p=1$${ob}");
    let d = decode(&c, &r, "8.129 empty salt", &s, 64, 64, T_ID);
    assert_eq!(d.ret, SALT_TOO_SHORT, "8.129: expected -6");

    // 8.130 a salt shorter than 8 bytes after decoding.
    for n in [1usize, 2, 4, 7] {
        let short: Vec<u8> = (0..n as u8).collect();
        let s = format!("$argon2id$v=19$m=8,t=1,p=1${}${ob}", b64(&short));
        let d = decode(&c, &r, &format!("8.130 saltlen={n}"), &s, 64, 64, T_ID);
        assert_eq!(d.ret, SALT_TOO_SHORT, "8.130 saltlen={n}: expected -6");
    }

    // 8.132 the hash decodes to more than `ctx->outlen` bytes on entry.
    for maxout in [0usize, 1, 16, 31] {
        let s = format!("$argon2id$v=19$m=8,t=1,p=1${sb}${ob}");
        let d = decode(&c, &r, &format!("8.132 maxout={maxout}"), &s, 64, maxout, T_ID);
        assert_eq!(d.ret, DECODING_FAIL, "8.132 maxout={maxout}: expected -32");
    }
    let s = format!("$argon2id$v=19$m=8,t=1,p=1${sb}${ob}");
    let d = decode(&c, &r, "8.132 maxout=32", &s, 64, 32, T_ID);
    assert_eq!(d.ret, ARGON2_OK);

    // 8.133 a truncated / invalid final base64 group in the hash.
    // "Zg" decodes to 1 byte -> OUTPUT_TOO_SHORT; a group with non-zero
    // trailing bits fails the bit-padding check first (-32).
    for (bad_hash, want) in [
        ("Zg", OUTPUT_TOO_SHORT),
        ("YWJj", OUTPUT_TOO_SHORT),
        ("Zh", DECODING_FAIL),
        // `=` is not in the ORIGINAL_NO_PADDING alphabet, so the decoder stops
        // there with 4 bytes decoded and the *validation* fires before the
        // trailing-NUL check.
        ("YWJjZA==", OUTPUT_TOO_SHORT),
        ("Z", DECODING_FAIL),
    ] {
        let s = format!("$argon2id$v=19$m=8,t=1,p=1${sb}${bad_hash}");
        let d = decode(&c, &r, &format!("8.133 hash={bad_hash:?}"), &s, 64, 64, T_ID);
        assert_eq!(d.ret, want, "8.133 hash={bad_hash:?}: expected {want}");
    }

    // 8.134 an empty hash field -> outlen 0.
    let s = format!("$argon2id$v=19$m=8,t=1,p=1${sb}$");
    let d = decode(&c, &r, "8.134 empty hash", &s, 64, 64, T_ID);
    assert_eq!(d.ret, OUTPUT_TOO_SHORT, "8.134: expected -2");

    // 8.135 p=0 -> LANES_TOO_FEW (threads is copied from lanes, so -28 is not
    // reached first).
    let s = format!("$argon2id$v=19$m=8,t=1,p=0${sb}${ob}");
    let d = decode(&c, &r, "8.135 p=0", &s, 64, 64, T_ID);
    assert_eq!(d.ret, LANES_TOO_FEW, "8.135: expected -16");

    // 8.136 t=0.
    let s = format!("$argon2id$v=19$m=8,t=0,p=1${sb}${ob}");
    let d = decode(&c, &r, "8.136 t=0", &s, 64, 64, T_ID);
    assert_eq!(d.ret, TIME_TOO_SMALL, "8.136: expected -12");

    // 8.137 m=0..7.
    for m in 0u32..8 {
        let s = format!("$argon2id$v=19$m={m},t=1,p=1${sb}${ob}");
        let d = decode(&c, &r, &format!("8.137 m={m}"), &s, 64, 64, T_ID);
        assert_eq!(d.ret, MEMORY_TOO_LITTLE, "8.137 m={m}: expected -14");
    }

    // 8.138 m valid on its own but m < 8 * p.
    for (m, p) in [(8u32, 2u32), (15, 2), (8, 4), (31, 4), (8, 16777215)] {
        let s = format!("$argon2id$v=19$m={m},t=1,p={p}${sb}${ob}");
        let d = decode(&c, &r, &format!("8.138 m={m} p={p}"), &s, 64, 64, T_ID);
        assert_eq!(d.ret, MEMORY_TOO_LITTLE, "8.138 m={m} p={p}: expected -14");
    }
    // 16 = 8 * 2 is accepted.
    let s = format!("$argon2id$v=19$m=16,t=1,p=2${sb}${ob}");
    let d = decode(&c, &r, "8.138 m=16 p=2", &s, 64, 64, T_ID);
    assert_eq!(d.ret, ARGON2_OK);

    // 8.140 a caller-provided `pwd == NULL` with `pwdlen != 0` is caught by the
    // final validation.  argon2_decode_string never touches ctx->pwd, so this
    // needs a hand-built context.
    let z = cstr(&format!("$argon2id$v=19$m=8,t=1,p=1${sb}${ob}"));
    for f in [&c, &r] {
        let mut salt = padded(64);
        let mut out = padded(64);
        let mut ctx = Ctx {
            out: out.as_mut_ptr(),
            outlen: 64,
            pwd: null_mut(),
            pwdlen: 7, // NULL pointer with a non-zero length
            salt: salt.as_mut_ptr(),
            saltlen: 64,
            secret: null_mut(),
            secretlen: 0,
            ad: null_mut(),
            adlen: 0,
            t_cost: 0,
            m_cost: 0,
            lanes: 0,
            threads: 0,
            flags: 0,
        };
        let ret = unsafe { f(&mut ctx, z.as_ptr() as *const c_char, T_ID) };
        assert_eq!(ret, PWD_PTR_MISMATCH, "8.140: expected -18");
    }

    // 8.141 / 8.142 / 8.143 `decode_decimal` rejections, reached through every
    // field that uses it: no digit at all, a non-minimal encoding, and a value
    // that overflows `unsigned long`.
    let huge = "9".repeat(40);
    for field in ["v", "m", "t", "p"] {
        for value in ["", "00", "007", "019", "0000000001", huge.as_str()] {
            let s = match field {
                "v" => format!("$argon2id$v={value}$m=8,t=1,p=1${sb}${ob}"),
                "m" => format!("$argon2id$v=19$m={value},t=1,p=1${sb}${ob}"),
                "t" => format!("$argon2id$v=19$m=8,t={value},p=1${sb}${ob}"),
                _ => format!("$argon2id$v=19$m=8,t=1,p={value}${sb}${ob}"),
            };
            let d = decode(
                &c, &r, &format!("8.141-8.143 {field}={value:?}"), &s, 64, 64, T_ID,
            );
            assert_eq!(
                d.ret, DECODING_FAIL,
                "8.141-8.143 {field}={value:?}: expected -32"
            );
        }
    }
    // A bare "0" *is* accepted by decode_decimal (it then fails validation).
    let s = format!("$argon2id$v=0$m=8,t=1,p=1${sb}${ob}");
    let d = decode(&c, &r, "8.142 bare 0 accepted by decode_decimal", &s, 64, 64, T_ID);
    assert_eq!(d.ret, INCORRECT_TYPE, "8.142: v=0 parses, then fails the version check");
}

#[test]
fn e8_144_to_147_encode_rejections() {
    let (c, r) = both::<EncodeString>("_sodium_argon2_encode_string");
    let salt8: Vec<u8> = (0x41u8..0x49).collect();
    let out16: Vec<u8> = (0x61u8..0x71).collect();

    // 8.144 a `type` that is neither Argon2_i nor Argon2_id -> ENCODING_FAIL,
    // and nothing is written (the switch runs before the first SS).
    for ty in [0, 3, 4, -1, c_int::MAX, c_int::MIN] {
        let (ret, dst) = encode(
            &c, &r, &format!("8.144 type={ty}"), 128, 8, 1, 1, 1, &salt8, &out16, ty,
        );
        assert_eq!(ret, ENCODING_FAIL, "8.144 type={ty}: expected -31");
        assert!(
            dst.iter().all(|&b| b == 0xC3),
            "8.144 type={ty}: dst must be untouched"
        );
    }

    // 8.145 dst_len too small for the "$argon2id$v=" / "$argon2i$v=" prefix.
    // "$argon2i$v=" is 11 chars, "$argon2id$v=" is 12, and the check is
    // `pp_len >= dst_len`.
    for ty in [T_I, T_ID] {
        let prefix = if ty == T_I { 11 } else { 12 };
        for dst_len in [0usize, 1, 5, prefix - 1, prefix] {
            let (ret, _) = encode(
                &c, &r, &format!("8.145 {} dst_len={dst_len}", kind(ty)), dst_len, 8, 1, 1, 1,
                &salt8, &out16, ty,
            );
            assert_eq!(
                ret, ENCODING_FAIL,
                "8.145 {} dst_len={dst_len}: expected -31",
                kind(ty)
            );
        }
    }

    // 8.146 argon2_validate_inputs is checked *after* the prefix has already
    // been written, so `dst` holds a partial "$argon2id$v=" string.
    let cases: &[(&str, u32, u32, u32, u32, usize, usize, c_int)] = &[
        // label, m, t, lanes, threads, saltlen, outlen, expected
        ("8.146 salt too short", 8, 1, 1, 1, 4, 16, SALT_TOO_SHORT),
        ("8.146 out too short", 8, 1, 1, 1, 8, 8, OUTPUT_TOO_SHORT),
        ("8.146 t_cost=0", 8, 0, 1, 1, 8, 16, TIME_TOO_SMALL),
        ("8.146 m_cost=0", 0, 1, 1, 1, 8, 16, MEMORY_TOO_LITTLE),
        ("8.146 lanes=0", 8, 1, 0, 1, 8, 16, LANES_TOO_FEW),
    ];
    for &(label, m, t, lanes, threads, saltlen, outlen, want) in cases {
        let salt: Vec<u8> = (0..saltlen as u8).collect();
        let out: Vec<u8> = (0..outlen as u8).collect();
        let (ret, dst) = encode(&c, &r, label, 128, m, t, lanes, threads, &salt, &out, T_ID);
        assert_eq!(ret, want, "{label}: expected {want}, got {ret}");
        assert_eq!(
            as_str(&dst),
            "$argon2id$v=",
            "{label}: the prefix must already be in `dst`"
        );
    }

    // 8.147 dst_len runs out at a later SS/SX ("$m=", the m_cost digits,
    // ",t=", ",p=", "$").  Every one of these returns -31.  The first size at
    // which the failure moves into sodium_bin2base64 (and therefore aborts) is
    // dst_len = 27 for this parameter set, so the sweep stops at 26.
    for dst_len in 13..=26usize {
        let (ret, _) = encode(
            &c, &r, &format!("8.147 dst_len={dst_len}"), dst_len, 8, 1, 1, 1, &salt8, &out16,
            T_I,
        );
        assert_eq!(ret, ENCODING_FAIL, "8.147 dst_len={dst_len}: expected -31");
    }
    // The same sweep with wide decimal fields, so the SX steps are the ones
    // that run out.
    for dst_len in 13..=40usize {
        let (ret, _) = encode(
            &c, &r, &format!("8.147 wide dst_len={dst_len}"), dst_len, 4294967295, 4294967295,
            16777215, 1, &salt8, &out16, T_I,
        );
        assert_eq!(ret, ENCODING_FAIL, "8.147 wide dst_len={dst_len}: expected -31");
    }
}
