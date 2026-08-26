//! Phase B/C/D — the remaining `_`-prefixed **internal-but-exported** symbols
//! that `t06_internal_exports.rs` does not cover:
//!
//! * `_sodium_blake2b*` (11) — the raw BLAKE2b streaming/simple API, the ref
//!   compress function and `blake2b_long`.
//! * `_sodium_argon2*` (16) — `argon2_ctx`, `argon2_hash`, the
//!   `argon2i/argon2id` `_hash_raw` / `_hash_encoded` / `_verify` forms,
//!   `argon2_validate_inputs`, `argon2_encode_string`, `argon2_decode_string`,
//!   and the internal `argon2_initialize` / `_fill_memory_blocks` /
//!   `_fill_segment_ref` / `_finalize` (driven through `argon2_ctx`).
//! * `_sodium_escrypt*` (10) — `escrypt_init_local` / `_free_local` /
//!   `_alloc_region` / `_free_region` / `_kdf_nosse` / `_r` / `_gensalt_r` /
//!   `_parse_setting` / `_PBKDF2_SHA256`.
//! * `_sodium_fe25519_*` (3), `_sodium_ge25519_*` (19), `_sodium_sc25519_*` (5),
//!   `_sodium_ristretto255_*` (3), `_sodium_core_h2c_string_to_hash` (1) — the
//!   ed25519 field / group / scalar internals, compared both semantically (via
//!   the `*_tobytes` encodings) and at the ABI level (raw struct bytes, since
//!   these functions take `ge25519_p3 *` across the exported boundary).
//!
//! Struct layouts (from the C headers, with `HAVE_TI_MODE` undefined so
//! `fe25519` is `int32_t[10]`):
//!
//! | type | size |
//! |---|---|
//! | `fe25519` | 40 |
//! | `ge25519_p2` (X,Y,Z) | 120 |
//! | `ge25519_p3` (X,Y,Z,T) | 160 |
//! | `ge25519_p1p1` (X,Y,Z,T) | 160 |
//! | `blake2b_state` (packed) | `crypto_generichash_blake2b_statebytes()` |
//! | `blake2b_param` (packed) | 64 |
//! | `argon2_context` | 11 × u32 + 5 pointers, `#[repr(C)]` |
//! | `escrypt_region_t` = `escrypt_local_t` | 24 (`void*`, `void*`, `size_t`) |

mod common;
use common::*;
use std::ffi::c_char;
use std::ffi::c_void;

const FE: usize = 40;
const GE_P2: usize = 3 * FE;
const GE_P3: usize = 4 * FE;

// ===========================================================================
// _sodium_blake2b*
// ===========================================================================

type B2Init = unsafe extern "C" fn(*mut u8, u8) -> i32;
type B2InitSaltPers = unsafe extern "C" fn(*mut u8, u8, *const c_void, *const c_void) -> i32;
type B2InitKey = unsafe extern "C" fn(*mut u8, u8, *const c_void, u8) -> i32;
type B2InitKeySaltPers =
    unsafe extern "C" fn(*mut u8, u8, *const c_void, u8, *const c_void, *const c_void) -> i32;
type B2InitParam = unsafe extern "C" fn(*mut u8, *const u8) -> i32;
type B2Update = unsafe extern "C" fn(*mut u8, *const u8, u64) -> i32;
type B2Final = unsafe extern "C" fn(*mut u8, *mut u8, u8) -> i32;
type B2Simple = unsafe extern "C" fn(*mut u8, *const c_void, *const c_void, u8, u64, u8) -> i32;
type B2SaltPers = unsafe extern "C" fn(
    *mut u8,
    *const c_void,
    *const c_void,
    u8,
    u64,
    u8,
    *const c_void,
    *const c_void,
) -> i32;
type B2Compress = unsafe extern "C" fn(*mut u8, *const u8) -> i32;
type B2Long = unsafe extern "C" fn(*mut c_void, usize, *const c_void, usize) -> i32;

fn b2_statebytes() -> usize {
    unsafe {
        sym::<unsafe extern "C" fn() -> usize>(c_lib(), "crypto_generichash_blake2b_statebytes")()
    }
}

/// `_sodium_blake2b` / `_salt_personal` — the simple one-shot forms, over the
/// full `outlen` × `keylen` × `inlen` grid the C accepts.
#[test]
fn blake2b_internal_simple_api() {
    setup();
    let mut rng = Rng::new(0xB200);
    let (c1, r1) = pair::<B2Simple>("_sodium_blake2b");
    let (c2, r2) = pair::<B2SaltPers>("_sodium_blake2b_salt_personal");
    for outlen in [1u8, 2, 15, 16, 17, 31, 32, 33, 63, 64] {
        for keylen in [0u8, 1, 15, 16, 32, 63, 64] {
            for inlen in [0usize, 1, 2, 127, 128, 129, 255, 256, 257, 1000] {
                let key = rng.bytes(keylen as usize);
                let inp = rng.bytes(inlen);
                let kp = if keylen == 0 {
                    std::ptr::null()
                } else {
                    key.as_ptr() as *const c_void
                };
                let ip = if inlen == 0 {
                    std::ptr::null()
                } else {
                    inp.as_ptr() as *const c_void
                };
                let mut a = canary(outlen as usize);
                let mut b = canary(outlen as usize);
                let (ra, rb) = unsafe {
                    (
                        c1(a.as_mut_ptr(), ip, kp, outlen, inlen as u64, keylen),
                        r1(b.as_mut_ptr(), ip, kp, outlen, inlen as u64, keylen),
                    )
                };
                eq_i32(&format!("_sodium_blake2b rc(out={outlen},key={keylen})"), ra, rb);
                eq_bytes(
                    &format!("_sodium_blake2b(out={outlen},key={keylen},in={inlen})"),
                    &a,
                    &b,
                );

                // salt/personal: NULL/NULL, salt only, personal only, both
                let salt = rng.bytes(16);
                let pers = rng.bytes(16);
                for combo in 0..4u32 {
                    let sp = if combo & 1 == 0 {
                        std::ptr::null()
                    } else {
                        salt.as_ptr() as *const c_void
                    };
                    let pp = if combo & 2 == 0 {
                        std::ptr::null()
                    } else {
                        pers.as_ptr() as *const c_void
                    };
                    let mut a = canary(outlen as usize);
                    let mut b = canary(outlen as usize);
                    let (ra, rb) = unsafe {
                        (
                            c2(a.as_mut_ptr(), ip, kp, outlen, inlen as u64, keylen, sp, pp),
                            r2(b.as_mut_ptr(), ip, kp, outlen, inlen as u64, keylen, sp, pp),
                        )
                    };
                    eq_i32("_sodium_blake2b_salt_personal rc", ra, rb);
                    eq_bytes(
                        &format!(
                            "_sodium_blake2b_salt_personal(out={outlen},key={keylen},in={inlen},combo={combo})"
                        ),
                        &a,
                        &b,
                    );
                }
            }
        }
    }
}

/// `_sodium_blake2b_init` / `_init_key` / `_init_salt_personal` /
/// `_init_key_salt_personal` / `_init_param` / `_update` / `_final`, with
/// randomised update chunking, and the raw `blake2b_state` bytes compared after
/// every step (these are exported functions taking `blake2b_state *`, so the
/// state layout is part of the ABI).
#[test]
fn blake2b_internal_streaming_api() {
    setup();
    let mut rng = Rng::new(0xB201);
    let sb = b2_statebytes();
    let (ci, ri) = pair::<B2Init>("_sodium_blake2b_init");
    let (cik, rik) = pair::<B2InitKey>("_sodium_blake2b_init_key");
    let (cisp, risp) = pair::<B2InitSaltPers>("_sodium_blake2b_init_salt_personal");
    let (ciksp, riksp) = pair::<B2InitKeySaltPers>("_sodium_blake2b_init_key_salt_personal");
    let (cu, ru) = pair::<B2Update>("_sodium_blake2b_update");
    let (cf, rf) = pair::<B2Final>("_sodium_blake2b_final");

    for outlen in [1u8, 16, 32, 64] {
        for inlen in [0usize, 1, 127, 128, 129, 300, 1000] {
            for style in 0..4u32 {
                if style == 1 && inlen > 300 {
                    continue;
                }
                let inp = rng.bytes(inlen);
                let parts = chunks(&mut rng, inlen, style);
                let key = rng.bytes(32);
                let salt = rng.bytes(16);
                let pers = rng.bytes(16);

                // four init variants
                for variant in 0..4u32 {
                    let mut out = [canary(outlen as usize), canary(outlen as usize)];
                    let mut states: Vec<Vec<u8>> = Vec::new();
                    let mut rcs = [0i32; 2];
                    for w in 0..2usize {
                        let mut st = State::new(sb);
                        unsafe {
                            rcs[w] = match variant {
                                0 => {
                                    let f = if w == 0 { ci } else { ri };
                                    f(st.as_mut_ptr(), outlen)
                                }
                                1 => {
                                    let f = if w == 0 { cik } else { rik };
                                    f(st.as_mut_ptr(), outlen, key.as_ptr() as *const c_void, 32)
                                }
                                2 => {
                                    let f = if w == 0 { cisp } else { risp };
                                    f(
                                        st.as_mut_ptr(),
                                        outlen,
                                        salt.as_ptr() as *const c_void,
                                        pers.as_ptr() as *const c_void,
                                    )
                                }
                                _ => {
                                    let f = if w == 0 { ciksp } else { riksp };
                                    f(
                                        st.as_mut_ptr(),
                                        outlen,
                                        key.as_ptr() as *const c_void,
                                        32,
                                        salt.as_ptr() as *const c_void,
                                        pers.as_ptr() as *const c_void,
                                    )
                                }
                            };
                            if rcs[w] == 0 {
                                let upd = if w == 0 { cu } else { ru };
                                let fin = if w == 0 { cf } else { rf };
                                let mut off = 0;
                                for &n in &parts {
                                    upd(st.as_mut_ptr(), inp[off..].as_ptr(), n as u64);
                                    off += n;
                                }
                                states.push(st.bytes().to_vec());
                                fin(st.as_mut_ptr(), out[w].as_mut_ptr(), outlen);
                            } else {
                                states.push(st.bytes().to_vec());
                            }
                        }
                    }
                    eq_i32(
                        &format!("_sodium_blake2b_init variant={variant} rc(out={outlen})"),
                        rcs[0],
                        rcs[1],
                    );
                    eq_bytes(
                        &format!("blake2b_state bytes after update (variant={variant},in={inlen})"),
                        &states[0],
                        &states[1],
                    );
                    let (a, b) = (out[0].clone(), out[1].clone());
                    eq_bytes(
                        &format!(
                            "_sodium_blake2b streaming(variant={variant},out={outlen},in={inlen},style={style})"
                        ),
                        &a,
                        &b,
                    );
                }
            }
        }
    }
}

/// `_sodium_blake2b_init_param` — a fully explicit 64-byte `blake2b_param`.
#[test]
fn blake2b_internal_init_param() {
    setup();
    let mut rng = Rng::new(0xB202);
    let sb = b2_statebytes();
    let (cip, rip) = pair::<B2InitParam>("_sodium_blake2b_init_param");
    let (cu, ru) = pair::<B2Update>("_sodium_blake2b_update");
    let (cf, rf) = pair::<B2Final>("_sodium_blake2b_final");

    for _ in 0..48 {
        // packed blake2b_param: digest_length, key_length, fanout, depth,
        // leaf_length[4], node_offset[8], node_depth, inner_length,
        // reserved[14], salt[16], personal[16]
        let mut p = vec![0u8; 64];
        let outlen = (rng.range(1, 64)) as u8;
        p[0] = outlen;
        p[1] = 0; // key_length
        p[2] = 1; // fanout
        p[3] = 1; // depth
        let leaf = rng.bytes(4);
        p[4..8].copy_from_slice(&leaf);
        let node = rng.bytes(8);
        p[8..16].copy_from_slice(&node);
        p[16] = rng.byte(); // node_depth
        p[17] = rng.byte(); // inner_length
        let salt = rng.bytes(16);
        p[32..48].copy_from_slice(&salt);
        let pers = rng.bytes(16);
        p[48..64].copy_from_slice(&pers);

        let n = rng.below(600);
        let inp = rng.bytes(n);
        let mut out = [canary(outlen as usize), canary(outlen as usize)];
        let mut rcs = [0i32; 2];
        let mut states: Vec<Vec<u8>> = Vec::new();
        for w in 0..2usize {
            let mut st = State::new(sb);
            unsafe {
                let f = if w == 0 { cip } else { rip };
                rcs[w] = f(st.as_mut_ptr(), p.as_ptr());
                states.push(st.bytes().to_vec());
                if rcs[w] == 0 {
                    let upd = if w == 0 { cu } else { ru };
                    let fin = if w == 0 { cf } else { rf };
                    upd(st.as_mut_ptr(), inp.as_ptr(), n as u64);
                    fin(st.as_mut_ptr(), out[w].as_mut_ptr(), outlen);
                }
            }
        }
        eq_i32("_sodium_blake2b_init_param rc", rcs[0], rcs[1]);
        eq_bytes("_sodium_blake2b_init_param state", &states[0], &states[1]);
        let (a, b) = (out[0].clone(), out[1].clone());
        eq_bytes(&format!("_sodium_blake2b_init_param digest(out={outlen})"), &a, &b);
    }
}

/// `_sodium_blake2b_compress_ref` — one raw 128-byte compression at a time,
/// driven from an initialised state whose counters we also vary.
#[test]
fn blake2b_internal_compress_ref() {
    setup();
    let mut rng = Rng::new(0xB203);
    let sb = b2_statebytes();
    let (ci, ri) = pair::<B2Init>("_sodium_blake2b_init");
    let (cc, rc_) = pair::<B2Compress>("_sodium_blake2b_compress_ref");
    for _ in 0..64 {
        let block = rng.bytes(128);
        let mut states: Vec<Vec<u8>> = Vec::new();
        let mut rcs = [0i32; 2];
        for w in 0..2usize {
            let mut st = State::new(sb);
            unsafe {
                let f = if w == 0 { ci } else { ri };
                f(st.as_mut_ptr(), 64);
                // bump the byte counter t[0] (offset 64 in blake2b_state:
                // h[8] = 64 bytes, then t[2]) so the compression sees a
                // non-zero length, exactly as `blake2b_update` would
                let t = 128u64.to_le_bytes();
                std::ptr::copy_nonoverlapping(t.as_ptr(), st.as_mut_ptr().add(64), 8);
                let comp = if w == 0 { cc } else { rc_ };
                rcs[w] = comp(st.as_mut_ptr(), block.as_ptr());
                states.push(st.bytes().to_vec());
            }
        }
        eq_i32("_sodium_blake2b_compress_ref rc", rcs[0], rcs[1]);
        eq_bytes(
            &format!("_sodium_blake2b_compress_ref state(block={})", hex(&block[..16])),
            &states[0],
            &states[1],
        );
    }
}

/// `_sodium_blake2b_long` — the argon2 variable-length BLAKE2b. Boundary
/// `outlen` values 64/65/96/97 change the code path.
#[test]
fn blake2b_internal_long() {
    setup();
    let mut rng = Rng::new(0xB204);
    let (c, r) = pair::<B2Long>("_sodium_blake2b_long");
    for outlen in [0usize, 1, 32, 63, 64, 65, 96, 97, 128, 129, 1000, 1024] {
        for inlen in [0usize, 1, 64, 127, 128, 129, 500] {
            let inp = rng.bytes(inlen);
            let mut a = canary(outlen.max(1));
            let mut b = canary(outlen.max(1));
            let (ra, rb) = unsafe {
                (
                    c(
                        a.as_mut_ptr() as *mut c_void,
                        outlen,
                        inp.as_ptr() as *const c_void,
                        inlen,
                    ),
                    r(
                        b.as_mut_ptr() as *mut c_void,
                        outlen,
                        inp.as_ptr() as *const c_void,
                        inlen,
                    ),
                )
            };
            eq_i32(&format!("_sodium_blake2b_long rc(out={outlen},in={inlen})"), ra, rb);
            eq_bytes(
                &format!("_sodium_blake2b_long(out={outlen},in={inlen})"),
                &a,
                &b,
            );
        }
    }
}

// ===========================================================================
// _sodium_argon2*
// ===========================================================================

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

type Argon2Ctx = unsafe extern "C" fn(*mut Argon2Context, i32) -> i32;
type Argon2Validate = unsafe extern "C" fn(*const Argon2Context) -> i32;
type Argon2Hash = unsafe extern "C" fn(
    u32, u32, u32, *const c_void, usize, *const c_void, usize, *mut c_void, usize, *mut c_char,
    usize, i32, u32,
) -> i32;
type Argon2HashRaw = unsafe extern "C" fn(
    u32, u32, u32, *const c_void, usize, *const c_void, usize, *mut c_void, usize,
) -> i32;
type Argon2HashEncoded = unsafe extern "C" fn(
    u32, u32, u32, *const c_void, usize, *const c_void, usize, usize, *mut c_char, usize,
) -> i32;
type Argon2Verify = unsafe extern "C" fn(*const c_char, *const c_void, usize, i32) -> i32;
type Argon2VerifyT = unsafe extern "C" fn(*const c_char, *const c_void, usize) -> i32;
type Argon2Encode = unsafe extern "C" fn(*mut c_char, usize, *mut Argon2Context, i32) -> i32;
type Argon2Decode = unsafe extern "C" fn(*mut Argon2Context, *const c_char, i32) -> i32;

const ARGON2_I: i32 = 1;
const ARGON2_ID: i32 = 2;

/// `_sodium_argon2_ctx` — the lowest-level Argon2 entry point, which drives
/// `argon2_initialize`, `argon2_fill_memory_blocks`, `argon2_fill_segment_ref`
/// and `argon2_finalize` internally. Cheap parameters only (`t_cost` 1..3,
/// `m_cost` 8..64 KiB) so the test stays fast.
#[test]
fn argon2_internal_ctx() {
    setup();
    let mut rng = Rng::new(0xA200);
    let (c, r) = pair::<Argon2Ctx>("_sodium_argon2_ctx");
    for &ty in &[ARGON2_I, ARGON2_ID] {
        for &t_cost in &[1u32, 2, 3] {
            for &lanes in &[1u32, 2, 4] {
                // m_cost must be >= 8 * lanes; also exercise the
                // 2*SYNC_POINTS*lanes segment-length shapes
                for &m_cost in &[8 * lanes, 8 * lanes + 1, 32 * lanes, 64, 256, 1024] {
                    if m_cost < 8 * lanes {
                        continue;
                    }
                    if ty == ARGON2_I && t_cost < 1 {
                        continue;
                    }
                    for &outlen in &[16usize, 32, 64] {
                        for &pwdlen in &[0usize, 1, 8, 100] {
                            let mut pwd = rng.bytes(pwdlen);
                            let mut salt = rng.bytes(16);
                            let mut secret = rng.bytes(8);
                            let mut ad = rng.bytes(12);
                            let mut out = [canary(outlen), canary(outlen)];
                            let mut rcs = [0i32; 2];
                            for (w, f) in [c, r].into_iter().enumerate() {
                                let mut ctx = Argon2Context {
                                    out: out[w].as_mut_ptr(),
                                    outlen: outlen as u32,
                                    pwd: pwd.as_mut_ptr(),
                                    pwdlen: pwdlen as u32,
                                    salt: salt.as_mut_ptr(),
                                    saltlen: 16,
                                    secret: secret.as_mut_ptr(),
                                    secretlen: 8,
                                    ad: ad.as_mut_ptr(),
                                    adlen: 12,
                                    t_cost,
                                    m_cost,
                                    lanes,
                                    threads: lanes,
                                    flags: 0,
                                };
                                rcs[w] = unsafe { f(&mut ctx, ty) };
                            }
                            eq_i32(
                                &format!(
                                    "_sodium_argon2_ctx rc(ty={ty},t={t_cost},m={m_cost},lanes={lanes},out={outlen},pwd={pwdlen})"
                                ),
                                rcs[0],
                                rcs[1],
                            );
                            let (a, b) = (out[0].clone(), out[1].clone());
                            eq_bytes(
                                &format!(
                                    "_sodium_argon2_ctx(ty={ty},t={t_cost},m={m_cost},lanes={lanes},out={outlen},pwd={pwdlen})"
                                ),
                                &a,
                                &b,
                            );
                        }
                    }
                }
            }
        }
    }
}

/// `_sodium_argon2_validate_inputs` — every distinct rejection code, plus valid
/// inputs. This is the Phase C surface of the internal validator.
#[test]
fn argon2_internal_validate_inputs() {
    setup();
    let (c, r) = pair::<Argon2Validate>("_sodium_argon2_validate_inputs");
    let mut out = vec![0u8; 64];
    let mut pwd = vec![0u8; 16];
    let mut salt = vec![0u8; 16];
    let base = |o: &mut Vec<u8>, p: &mut Vec<u8>, s: &mut Vec<u8>| Argon2Context {
        out: o.as_mut_ptr(),
        outlen: 32,
        pwd: p.as_mut_ptr(),
        pwdlen: 16,
        salt: s.as_mut_ptr(),
        saltlen: 16,
        secret: std::ptr::null_mut(),
        secretlen: 0,
        ad: std::ptr::null_mut(),
        adlen: 0,
        t_cost: 3,
        m_cost: 4096,
        lanes: 1,
        threads: 1,
        flags: 0,
    };
    // (label, mutator)
    type Mut = fn(&mut Argon2Context);
    let cases: Vec<(&str, Mut)> = vec![
        ("valid", |_c| {}),
        ("ctx.out = NULL", |c| c.out = std::ptr::null_mut()),
        ("outlen = 0", |c| c.outlen = 0),
        ("outlen = 3", |c| c.outlen = 3),
        ("outlen = u32::MAX", |c| c.outlen = u32::MAX),
        ("pwd = NULL, pwdlen != 0", |c| c.pwd = std::ptr::null_mut()),
        ("pwdlen = 0", |c| c.pwdlen = 0),
        ("salt = NULL", |c| c.salt = std::ptr::null_mut()),
        ("saltlen = 0", |c| c.saltlen = 0),
        ("saltlen = 7", |c| c.saltlen = 7),
        ("secret = NULL, secretlen != 0", |c| c.secretlen = 16),
        ("ad = NULL, adlen != 0", |c| c.adlen = 16),
        ("t_cost = 0", |c| c.t_cost = 0),
        ("t_cost = u32::MAX", |c| c.t_cost = u32::MAX),
        ("m_cost = 0", |c| c.m_cost = 0),
        ("m_cost = 1", |c| c.m_cost = 1),
        ("m_cost = 7", |c| c.m_cost = 7),
        ("m_cost = u32::MAX", |c| c.m_cost = u32::MAX),
        ("lanes = 0", |c| c.lanes = 0),
        ("lanes = u32::MAX", |c| c.lanes = u32::MAX),
        ("threads = 0", |c| c.threads = 0),
        ("threads = u32::MAX", |c| c.threads = u32::MAX),
        ("m_cost < 8*lanes", |c| {
            c.lanes = 4;
            c.threads = 4;
            c.m_cost = 8;
        }),
        ("lanes = 4, m_cost = 32", |c| {
            c.lanes = 4;
            c.threads = 4;
            c.m_cost = 32;
        }),
        ("flags = u32::MAX", |c| c.flags = u32::MAX),
    ];
    for (label, m) in cases {
        let mut ctx = base(&mut out, &mut pwd, &mut salt);
        m(&mut ctx);
        let (a, b) = unsafe { (c(&ctx), r(&ctx)) };
        eq_i32(&format!("_sodium_argon2_validate_inputs({label})"), a, b);
    }
}

/// `_sodium_argon2_hash`, `_sodium_argon2i_hash_raw`, `_sodium_argon2id_hash_raw`,
/// `_sodium_argon2i_hash_encoded`, `_sodium_argon2id_hash_encoded`.
#[test]
fn argon2_internal_hash_forms() {
    setup();
    let mut rng = Rng::new(0xA201);
    let (chr_i, rhr_i) = pair::<Argon2HashRaw>("_sodium_argon2i_hash_raw");
    let (chr_d, rhr_d) = pair::<Argon2HashRaw>("_sodium_argon2id_hash_raw");
    let (che_i, rhe_i) = pair::<Argon2HashEncoded>("_sodium_argon2i_hash_encoded");
    let (che_d, rhe_d) = pair::<Argon2HashEncoded>("_sodium_argon2id_hash_encoded");
    let (ch, rh) = pair::<Argon2Hash>("_sodium_argon2_hash");

    for &(t_cost, m_cost) in &[(3u32, 32u32), (1, 8), (2, 64), (3, 1024)] {
        for &outlen in &[16usize, 32] {
            for &pwdlen in &[0usize, 1, 8, 64] {
                let pwd = rng.bytes(pwdlen);
                let salt = rng.bytes(16);
                // raw forms (argon2i requires t_cost >= 1 here; the public
                // wrapper's >= 3 rule lives one layer up)
                for (label, cf, rf) in [
                    ("argon2i_hash_raw", chr_i, rhr_i),
                    ("argon2id_hash_raw", chr_d, rhr_d),
                ] {
                    let mut a = canary(outlen);
                    let mut b = canary(outlen);
                    let (ra, rb) = unsafe {
                        (
                            cf(t_cost, m_cost, 1, pwd.as_ptr() as *const c_void, pwdlen,
                               salt.as_ptr() as *const c_void, 16,
                               a.as_mut_ptr() as *mut c_void, outlen),
                            rf(t_cost, m_cost, 1, pwd.as_ptr() as *const c_void, pwdlen,
                               salt.as_ptr() as *const c_void, 16,
                               b.as_mut_ptr() as *mut c_void, outlen),
                        )
                    };
                    eq_i32(&format!("_sodium_{label} rc(t={t_cost},m={m_cost})"), ra, rb);
                    eq_bytes(
                        &format!("_sodium_{label}(t={t_cost},m={m_cost},out={outlen},pwd={pwdlen})"),
                        &a,
                        &b,
                    );
                }
                // encoded forms
                for (label, cf, rf) in [
                    ("argon2i_hash_encoded", che_i, rhe_i),
                    ("argon2id_hash_encoded", che_d, rhe_d),
                ] {
                    let mut a = vec![0xA5u8; 256];
                    let mut b = vec![0xA5u8; 256];
                    let (ra, rb) = unsafe {
                        (
                            cf(t_cost, m_cost, 1, pwd.as_ptr() as *const c_void, pwdlen,
                               salt.as_ptr() as *const c_void, 16, outlen,
                               a.as_mut_ptr() as *mut c_char, 256),
                            rf(t_cost, m_cost, 1, pwd.as_ptr() as *const c_void, pwdlen,
                               salt.as_ptr() as *const c_void, 16, outlen,
                               b.as_mut_ptr() as *mut c_char, 256),
                        )
                    };
                    eq_i32(&format!("_sodium_{label} rc"), ra, rb);
                    eq_bytes(&format!("_sodium_{label}(t={t_cost},m={m_cost})"), &a, &b);
                }
                // the generic argon2_hash, both types, raw + encoded
                for &ty in &[ARGON2_I, ARGON2_ID] {
                    let mut ra_out = canary(outlen);
                    let mut rb_out = canary(outlen);
                    let mut ra_enc = vec![0xA5u8; 256];
                    let mut rb_enc = vec![0xA5u8; 256];
                    let (ra, rb) = unsafe {
                        (
                            ch(t_cost, m_cost, 1, pwd.as_ptr() as *const c_void, pwdlen,
                               salt.as_ptr() as *const c_void, 16,
                               ra_out.as_mut_ptr() as *mut c_void, outlen,
                               ra_enc.as_mut_ptr() as *mut c_char, 256, ty, 0x13),
                            rh(t_cost, m_cost, 1, pwd.as_ptr() as *const c_void, pwdlen,
                               salt.as_ptr() as *const c_void, 16,
                               rb_out.as_mut_ptr() as *mut c_void, outlen,
                               rb_enc.as_mut_ptr() as *mut c_char, 256, ty, 0x13),
                        )
                    };
                    eq_i32(&format!("_sodium_argon2_hash rc(ty={ty})"), ra, rb);
                    eq_bytes(&format!("_sodium_argon2_hash raw(ty={ty})"), &ra_out, &rb_out);
                    eq_bytes(&format!("_sodium_argon2_hash enc(ty={ty})"), &ra_enc, &rb_enc);
                }
                // out-of-range `type` / `version` values crossing the FFI
                for &(ty, ver) in &[(0i32, 0x13u32), (3, 0x13), (-1, 0x13), (i32::MAX, 0x13),
                                    (ARGON2_I, 0), (ARGON2_I, 0x10), (ARGON2_I, u32::MAX)] {
                    let mut ra_out = canary(outlen);
                    let mut rb_out = canary(outlen);
                    let (ra, rb) = unsafe {
                        (
                            ch(t_cost, m_cost, 1, pwd.as_ptr() as *const c_void, pwdlen,
                               salt.as_ptr() as *const c_void, 16,
                               ra_out.as_mut_ptr() as *mut c_void, outlen,
                               std::ptr::null_mut(), 0, ty, ver),
                            rh(t_cost, m_cost, 1, pwd.as_ptr() as *const c_void, pwdlen,
                               salt.as_ptr() as *const c_void, 16,
                               rb_out.as_mut_ptr() as *mut c_void, outlen,
                               std::ptr::null_mut(), 0, ty, ver),
                        )
                    };
                    eq_i32(&format!("_sodium_argon2_hash rc(ty={ty},ver={ver:#x})"), ra, rb);
                    eq_bytes(&format!("_sodium_argon2_hash out(ty={ty},ver={ver:#x})"), &ra_out, &rb_out);
                }
            }
        }
    }
}

/// `_sodium_argon2_verify` / `_argon2i_verify` / `_argon2id_verify`, plus
/// `_argon2_encode_string` / `_argon2_decode_string` round trips and every
/// distinct decoding rejection.
#[test]
fn argon2_internal_verify_and_encoding() {
    setup();
    let mut rng = Rng::new(0xA202);
    let (che, _) = pair::<Argon2HashEncoded>("_sodium_argon2i_hash_encoded");
    let (chd, _) = pair::<Argon2HashEncoded>("_sodium_argon2id_hash_encoded");
    let (cv, rv) = pair::<Argon2Verify>("_sodium_argon2_verify");
    let (cvi, rvi) = pair::<Argon2VerifyT>("_sodium_argon2i_verify");
    let (cvd, rvd) = pair::<Argon2VerifyT>("_sodium_argon2id_verify");
    let (cenc, renc) = pair::<Argon2Encode>("_sodium_argon2_encode_string");
    let (cdec, rdec) = pair::<Argon2Decode>("_sodium_argon2_decode_string");

    for (ty, hash_encoded, vt_c, vt_r) in [
        (ARGON2_I, che, cvi, rvi),
        (ARGON2_ID, chd, cvd, rvd),
    ] {
        let pwd = rng.bytes(16);
        let salt = rng.bytes(16);
        let mut enc = vec![0u8; 256];
        let rc = unsafe {
            hash_encoded(3, 32, 1, pwd.as_ptr() as *const c_void, 16,
                         salt.as_ptr() as *const c_void, 16, 32,
                         enc.as_mut_ptr() as *mut c_char, 256)
        };
        assert_eq!(rc, 0, "hash_encoded must succeed");
        let s = std::ffi::CStr::from_bytes_until_nul(&enc).unwrap().to_owned();

        // correct password
        for (label, a, b) in [
            ("argon2_verify", unsafe { cv(s.as_ptr(), pwd.as_ptr() as *const c_void, 16, ty) },
                              unsafe { rv(s.as_ptr(), pwd.as_ptr() as *const c_void, 16, ty) }),
            ("typed_verify", unsafe { vt_c(s.as_ptr(), pwd.as_ptr() as *const c_void, 16) },
                             unsafe { vt_r(s.as_ptr(), pwd.as_ptr() as *const c_void, 16) }),
        ] {
            eq_i32(&format!("_sodium_{label} ok (ty={ty})"), a, b);
            assert_eq!(a, 0, "{label} must accept the right password");
        }
        // wrong password / wrong length / empty
        for wrong in [rng.bytes(16), rng.bytes(15), rng.bytes(0), vec![0u8; 16]] {
            let (a, b) = unsafe {
                (
                    cv(s.as_ptr(), wrong.as_ptr() as *const c_void, wrong.len(), ty),
                    rv(s.as_ptr(), wrong.as_ptr() as *const c_void, wrong.len(), ty),
                )
            };
            eq_i32(&format!("_sodium_argon2_verify wrong pwd (ty={ty})"), a, b);
            let (a, b) = unsafe {
                (
                    vt_c(s.as_ptr(), wrong.as_ptr() as *const c_void, wrong.len()),
                    vt_r(s.as_ptr(), wrong.as_ptr() as *const c_void, wrong.len()),
                )
            };
            eq_i32(&format!("_sodium_argon2*_verify wrong pwd (ty={ty})"), a, b);
        }
        // out-of-range `type` across the FFI
        for &bad_ty in &[0i32, 3, -1, i32::MAX, i32::MIN] {
            let (a, b) = unsafe {
                (
                    cv(s.as_ptr(), pwd.as_ptr() as *const c_void, 16, bad_ty),
                    rv(s.as_ptr(), pwd.as_ptr() as *const c_void, 16, bad_ty),
                )
            };
            eq_i32(&format!("_sodium_argon2_verify bad type={bad_ty}"), a, b);
        }
        // decode_string: valid, then every mutation family
        let sbytes = s.to_bytes();
        let mut malformed: Vec<Vec<u8>> = Vec::new();
        malformed.push(sbytes.to_vec()); // valid
        malformed.push(b"".to_vec());
        malformed.push(b"$".to_vec());
        malformed.push(b"$argon2x$v=19$m=32,t=3,p=1$AAAA$AAAA".to_vec());
        malformed.push(b"$argon2i$v=18$m=32,t=3,p=1$AAAA$AAAA".to_vec());
        malformed.push(b"$argon2i$v=19$m=0,t=3,p=1$AAAA$AAAA".to_vec());
        malformed.push(b"$argon2i$v=19$m=032,t=3,p=1$AAAA$AAAA".to_vec()); // non-minimal
        malformed.push(b"$argon2i$v=19$m=4294967296,t=3,p=1$AAAA$AAAA".to_vec()); // > u32
        malformed.push(b"$argon2i$v=19$t=3,p=1$AAAA$AAAA".to_vec()); // missing m=
        malformed.push(b"$argon2i$v=19$m=32,p=1$AAAA$AAAA".to_vec()); // missing t=
        malformed.push(b"$argon2i$v=19$m=32,t=3$AAAA$AAAA".to_vec()); // missing p=
        malformed.push(b"$argon2i$m=32,t=3,p=1$AAAA$AAAA".to_vec()); // missing v=
        malformed.push(b"$argon2i$v=19$m=32,t=3,p=1$!!!!$AAAA".to_vec()); // bad b64
        malformed.push(b"$argon2i$v=19$m=32,t=3,p=1$AAAA$!!!!".to_vec());
        for i in 0..sbytes.len() {
            // truncations
            if i % 7 == 0 {
                malformed.push(sbytes[..i].to_vec());
            }
        }
        for i in (0..sbytes.len()).step_by(5) {
            let mut v = sbytes.to_vec();
            v[i] = b'Z';
            malformed.push(v);
        }
        {
            let mut v = sbytes.to_vec();
            v.push(b'X'); // trailing character
            malformed.push(v);
        }
        for m in &malformed {
            let mut cstr = m.clone();
            cstr.push(0);
            for &dty in &[ty, 0, 3, -1] {
                let mut out = [vec![0u8; 64], vec![0u8; 64]];
                let mut pw = [vec![0u8; 64], vec![0u8; 64]];
                let mut sl = [vec![0u8; 64], vec![0u8; 64]];
                let mut rcs = [0i32; 2];
                let mut ctxs: Vec<Vec<u8>> = Vec::new();
                for (w, f) in [cdec, rdec].into_iter().enumerate() {
                    let mut ctx = Argon2Context {
                        out: out[w].as_mut_ptr(),
                        outlen: 64,
                        pwd: pw[w].as_mut_ptr(),
                        pwdlen: 64,
                        salt: sl[w].as_mut_ptr(),
                        saltlen: 64,
                        secret: std::ptr::null_mut(),
                        secretlen: 0,
                        ad: std::ptr::null_mut(),
                        adlen: 0,
                        t_cost: 0,
                        m_cost: 0,
                        lanes: 0,
                        threads: 0,
                        flags: 0,
                    };
                    rcs[w] = unsafe { f(&mut ctx, cstr.as_ptr() as *const c_char, dty) };
                    ctxs.push(vec![
                        ctx.outlen as u8, (ctx.outlen >> 8) as u8,
                        ctx.saltlen as u8, (ctx.saltlen >> 8) as u8,
                        ctx.t_cost as u8, (ctx.t_cost >> 8) as u8,
                        ctx.m_cost as u8, (ctx.m_cost >> 8) as u8,
                        ctx.lanes as u8, ctx.threads as u8,
                    ]);
                }
                eq_i32(
                    &format!(
                        "_sodium_argon2_decode_string rc(ty={dty}, str={:?})",
                        String::from_utf8_lossy(m)
                    ),
                    rcs[0],
                    rcs[1],
                );
                eq_bytes("_sodium_argon2_decode_string ctx fields", &ctxs[0], &ctxs[1]);
                let (a, b) = (out[0].clone(), out[1].clone());
                eq_bytes("_sodium_argon2_decode_string out", &a, &b);
                let (a, b) = (sl[0].clone(), sl[1].clone());
                eq_bytes("_sodium_argon2_decode_string salt", &a, &b);
            }
        }

        // encode_string round trip and buffer-too-small rejections.
        //
        // NOTE (derived from `argon2-encoding.c`): the `SS(...)` macro returns
        // `ARGON2_ENCODING_FAIL` when the *literal* does not fit, but the
        // `SB(...)` macro delegates to `sodium_bin2base64`, which **aborts via
        // `sodium_misuse()`** when `b64_maxlen` is too small rather than
        // returning NULL — so `SB`'s `return ARGON2_ENCODING_FAIL` is dead code.
        // The prefix consumes 27 bytes for these parameters, and the full string
        // needs 94, so:
        //   dst_len <= 27          -> clean ARGON2_ENCODING_FAIL   (tested here)
        //   28 <= dst_len <= 94    -> SIGABRT inside sodium_bin2base64
        //                             (tested out of process, below)
        //   dst_len >= 95          -> success                      (tested here)
        // (the exact totals are 94 for `$argon2i$v=` and 95 for `$argon2id$v=`,
        //  so 27 / 95 are the boundaries that hold for BOTH variants)
        for dstlen in [0usize, 1, 8, 16, 26, 27, 95, 96, 100, 128, 256] {
            let mut pw2 = rng.bytes(16);
            let mut sl2 = rng.bytes(16);
            let mut o2 = rng.bytes(32);
            let mut dst = [vec![0xA5u8; 300], vec![0xA5u8; 300]];
            let mut rcs = [0i32; 2];
            for (w, f) in [cenc, renc].into_iter().enumerate() {
                let mut ctx = Argon2Context {
                    out: o2.as_mut_ptr(),
                    outlen: 32,
                    pwd: pw2.as_mut_ptr(),
                    pwdlen: 16,
                    salt: sl2.as_mut_ptr(),
                    saltlen: 16,
                    secret: std::ptr::null_mut(),
                    secretlen: 0,
                    ad: std::ptr::null_mut(),
                    adlen: 0,
                    t_cost: 3,
                    m_cost: 32,
                    lanes: 1,
                    threads: 1,
                    flags: 0,
                };
                rcs[w] = unsafe {
                    f(dst[w].as_mut_ptr() as *mut c_char, dstlen, &mut ctx, ty)
                };
            }
            eq_i32(
                &format!("_sodium_argon2_encode_string rc(dstlen={dstlen},ty={ty})"),
                rcs[0],
                rcs[1],
            );
            let (a, b) = (dst[0].clone(), dst[1].clone());
            eq_bytes(
                &format!("_sodium_argon2_encode_string(dstlen={dstlen},ty={ty})"),
                &a,
                &b,
            );
        }
    }
}

// ===========================================================================
// _sodium_escrypt*
// ===========================================================================

#[repr(C)]
struct EscryptRegion {
    base: *mut c_void,
    aligned: *mut c_void,
    size: usize,
}

type EsInitLocal = unsafe extern "C" fn(*mut EscryptRegion) -> i32;
type EsFreeLocal = unsafe extern "C" fn(*mut EscryptRegion) -> i32;
type EsAllocRegion = unsafe extern "C" fn(*mut EscryptRegion, usize) -> *mut c_void;
type EsFreeRegion = unsafe extern "C" fn(*mut EscryptRegion) -> i32;
type EsKdf = unsafe extern "C" fn(
    *mut EscryptRegion, *const u8, usize, *const u8, usize, u64, u32, u32, *mut u8, usize,
) -> i32;
type EsR = unsafe extern "C" fn(
    *mut EscryptRegion, *const u8, usize, *const u8, *mut u8, usize,
) -> *mut u8;
type EsGensalt =
    unsafe extern "C" fn(u32, u32, u32, *const u8, usize, *mut u8, usize) -> *mut u8;
type EsParseSetting = unsafe extern "C" fn(*const u8, *mut u32, *mut u32, *mut u32) -> *const u8;
type EsPbkdf2 = unsafe extern "C" fn(*const u8, usize, *const u8, usize, u64, *mut u8, usize);

/// `_sodium_escrypt_init_local` / `_free_local` / `_alloc_region` /
/// `_free_region` — the allocator lifecycle.
#[test]
fn escrypt_internal_region_lifecycle() {
    setup();
    let (cil, ril) = pair::<EsInitLocal>("_sodium_escrypt_init_local");
    let (cfl, rfl) = pair::<EsFreeLocal>("_sodium_escrypt_free_local");
    let (car, rar) = pair::<EsAllocRegion>("_sodium_escrypt_alloc_region");
    let (cfr, rfr) = pair::<EsFreeRegion>("_sodium_escrypt_free_region");

    for (il, fl, ar, fr) in [(cil, cfl, car, cfr), (ril, rfl, rar, rfr)] {
        let mut reg = EscryptRegion {
            base: std::ptr::null_mut(),
            aligned: std::ptr::null_mut(),
            size: 0,
        };
        assert_eq!(unsafe { il(&mut reg) }, 0, "escrypt_init_local");
        assert!(reg.base.is_null() && reg.aligned.is_null() && reg.size == 0);
        assert_eq!(unsafe { fl(&mut reg) }, 0, "escrypt_free_local");

        for size in [1usize, 64, 4096, 1 << 20] {
            let mut reg = EscryptRegion {
                base: std::ptr::null_mut(),
                aligned: std::ptr::null_mut(),
                size: 0,
            };
            let p = unsafe { ar(&mut reg, size) };
            assert!(!p.is_null(), "escrypt_alloc_region({size})");
            assert_eq!(reg.size, size);
            assert_eq!(unsafe { fr(&mut reg) }, 0, "escrypt_free_region");
            assert!(reg.base.is_null() && reg.aligned.is_null() && reg.size == 0);
        }
        // freeing a zeroed region must be a no-op success in both
        let mut reg = EscryptRegion {
            base: std::ptr::null_mut(),
            aligned: std::ptr::null_mut(),
            size: 0,
        };
        assert_eq!(unsafe { fr(&mut reg) }, 0);
    }
    // an absurd size must fail identically
    for size in [usize::MAX, usize::MAX / 2] {
        let mut a = EscryptRegion { base: std::ptr::null_mut(), aligned: std::ptr::null_mut(), size: 0 };
        let mut b = EscryptRegion { base: std::ptr::null_mut(), aligned: std::ptr::null_mut(), size: 0 };
        let pa = unsafe { car(&mut a, size) };
        let pb = unsafe { rar(&mut b, size) };
        assert_eq!(pa.is_null(), pb.is_null(), "alloc_region({size}) nullness");
        assert_eq!(a.size, b.size);
        if !pa.is_null() {
            unsafe { cfr(&mut a) };
        }
        if !pb.is_null() {
            unsafe { rfr(&mut b) };
        }
    }
}

/// `_sodium_escrypt_kdf_nosse` — the live scrypt KDF in this build. Explicit
/// `N`/`r`/`p` covering the smix/blockmix loop shapes, plus every rejection.
#[test]
fn escrypt_internal_kdf_nosse() {
    setup();
    let mut rng = Rng::new(0xE200);
    let (ck, rk) = pair::<EsKdf>("_sodium_escrypt_kdf_nosse");
    let (cil, ril) = pair::<EsInitLocal>("_sodium_escrypt_init_local");
    let (cfl, rfl) = pair::<EsFreeLocal>("_sodium_escrypt_free_local");

    // (N, r, p) — valid shapes plus every documented rejection
    let params: &[(u64, u32, u32)] = &[
        (2, 1, 1), (2, 8, 1), (2, 1, 4), (4, 4, 2), (16, 8, 1), (1024, 8, 1),
        (2, 8, 512), (16, 1, 1), (256, 2, 3),
        // rejections
        (0, 8, 1), (1, 8, 1), (3, 8, 1), (6, 8, 1), (u64::MAX, 8, 1),
        (1u64 << 33, 8, 1),
        (16, 0, 1), (16, 8, 0),
        (16, (1 << 30) / 8, 8), // r*p >= 2^30
        (16, 1 << 30, 1),
    ];
    for &(n, r, p) in params {
        for &buflen in &[16usize, 32, 64] {
            for &pwlen in &[0usize, 1, 16, 100] {
                let pw = rng.bytes(pwlen);
                let salt = rng.bytes(32);
                let mut out = [canary(buflen), canary(buflen)];
                let mut rcs = [0i32; 2];
                for (w, (il, fl, kdf)) in
                    [(cil, cfl, ck), (ril, rfl, rk)].into_iter().enumerate()
                {
                    let mut reg = EscryptRegion {
                        base: std::ptr::null_mut(),
                        aligned: std::ptr::null_mut(),
                        size: 0,
                    };
                    unsafe {
                        il(&mut reg);
                        rcs[w] = kdf(
                            &mut reg,
                            pw.as_ptr(), pwlen,
                            salt.as_ptr(), 32,
                            n, r, p,
                            out[w].as_mut_ptr(), buflen,
                        );
                        fl(&mut reg);
                    }
                }
                eq_i32(
                    &format!("_sodium_escrypt_kdf_nosse rc(N={n},r={r},p={p},buf={buflen},pw={pwlen})"),
                    rcs[0],
                    rcs[1],
                );
                let (a, b) = (out[0].clone(), out[1].clone());
                eq_bytes(
                    &format!("_sodium_escrypt_kdf_nosse(N={n},r={r},p={p},buf={buflen},pw={pwlen})"),
                    &a,
                    &b,
                );
            }
        }
    }
}

/// `_sodium_escrypt_gensalt_r`, `_sodium_escrypt_parse_setting`,
/// `_sodium_escrypt_r` — the crypt(3)-style string API.
#[test]
fn escrypt_internal_string_api() {
    setup();
    let mut rng = Rng::new(0xE201);
    let (cg, rg) = pair::<EsGensalt>("_sodium_escrypt_gensalt_r");
    let (cp, rp) = pair::<EsParseSetting>("_sodium_escrypt_parse_setting");
    let (cr, rr) = pair::<EsR>("_sodium_escrypt_r");
    let (cil, ril) = pair::<EsInitLocal>("_sodium_escrypt_init_local");
    let (cfl, rfl) = pair::<EsFreeLocal>("_sodium_escrypt_free_local");

    for &(n_log2, r, p) in &[
        (1u32, 8u32, 1u32), (4, 8, 1), (10, 8, 1), (14, 8, 1),
        // invalid
        (0, 8, 1), (64, 8, 1), (255, 8, 1), (10, 0, 1), (10, 8, 0),
        (10, 1 << 30, 1),
    ] {
        for &srclen in &[0usize, 1, 16, 32, 33] {
            for &buflen in &[0usize, 1, 32, 57, 58, 64, 128] {
                let src = rng.bytes(srclen);
                let mut buf = [vec![0xA5u8; 200], vec![0xA5u8; 200]];
                let mut nulls = [false, false];
                for (w, f) in [cg, rg].into_iter().enumerate() {
                    let q = unsafe {
                        f(n_log2, r, p, src.as_ptr(), srclen, buf[w].as_mut_ptr(), buflen)
                    };
                    nulls[w] = q.is_null();
                }
                assert_eq!(
                    nulls[0], nulls[1],
                    "_sodium_escrypt_gensalt_r nullness(N_log2={n_log2},r={r},p={p},src={srclen},buf={buflen})"
                );
                let (a, b) = (buf[0].clone(), buf[1].clone());
                eq_bytes(
                    &format!("_sodium_escrypt_gensalt_r(N_log2={n_log2},r={r},p={p},src={srclen},buf={buflen})"),
                    &a,
                    &b,
                );

                // parse the produced setting back
                if !nulls[0] {
                    let setting = buf[0].clone();
                    let mut vals = [[0u32; 3], [0u32; 3]];
                    let mut ends = [0isize, 0];
                    for (w, f) in [cp, rp].into_iter().enumerate() {
                        let q = unsafe {
                            f(setting.as_ptr(), &mut vals[w][0], &mut vals[w][1], &mut vals[w][2])
                        };
                        ends[w] = if q.is_null() {
                            -1
                        } else {
                            unsafe { q.offset_from(setting.as_ptr()) }
                        };
                    }
                    assert_eq!(ends[0], ends[1], "_sodium_escrypt_parse_setting end offset");
                    assert_eq!(vals[0], vals[1], "_sodium_escrypt_parse_setting N/r/p");

                    // and hash a password with it
                    let pw = rng.bytes(8);
                    let mut hbuf = [vec![0xA5u8; 200], vec![0xA5u8; 200]];
                    let mut hnull = [false, false];
                    for (w, (il, fl, f)) in
                        [(cil, cfl, cr), (ril, rfl, rr)].into_iter().enumerate()
                    {
                        // keep the work small: only hash when N_log2 is tiny
                        if n_log2 > 10 {
                            continue;
                        }
                        let mut reg = EscryptRegion {
                            base: std::ptr::null_mut(),
                            aligned: std::ptr::null_mut(),
                            size: 0,
                        };
                        unsafe {
                            il(&mut reg);
                            let q = f(
                                &mut reg, pw.as_ptr(), 8, setting.as_ptr(),
                                hbuf[w].as_mut_ptr(), 200,
                            );
                            hnull[w] = q.is_null();
                            fl(&mut reg);
                        }
                    }
                    if n_log2 <= 10 {
                        assert_eq!(hnull[0], hnull[1], "_sodium_escrypt_r nullness");
                        let (a, b) = (hbuf[0].clone(), hbuf[1].clone());
                        eq_bytes("_sodium_escrypt_r output", &a, &b);
                    }
                }
            }
        }
    }

    // malformed settings for parse_setting
    let bad: &[&[u8]] = &[
        b"\0",
        b"$7$\0",
        b"$7$A\0",
        b"$8$C6..../....\0",
        b"$7$C6..../....",
        b"xyz\0",
        b"$7$!!......\0",
    ];
    for s in bad {
        let mut vals = [[0xDEADBEEFu32; 3], [0xDEADBEEFu32; 3]];
        let mut ends = [0isize, 0];
        for (w, f) in [cp, rp].into_iter().enumerate() {
            let q = unsafe { f(s.as_ptr(), &mut vals[w][0], &mut vals[w][1], &mut vals[w][2]) };
            ends[w] = if q.is_null() { -1 } else { unsafe { q.offset_from(s.as_ptr()) } };
        }
        assert_eq!(
            ends[0], ends[1],
            "_sodium_escrypt_parse_setting({:?}) end", String::from_utf8_lossy(s)
        );
        assert_eq!(vals[0], vals[1], "_sodium_escrypt_parse_setting({:?}) N/r/p",
                   String::from_utf8_lossy(s));
    }
}

/// `_sodium_escrypt_PBKDF2_SHA256`.
#[test]
fn escrypt_internal_pbkdf2() {
    setup();
    let mut rng = Rng::new(0xE202);
    let (c, r) = pair::<EsPbkdf2>("_sodium_escrypt_PBKDF2_SHA256");
    for &c_iter in &[1u64, 2, 3, 16, 1000] {
        for &dklen in &[0usize, 1, 31, 32, 33, 63, 64, 65, 100] {
            for &(pwlen, saltlen) in &[(0usize, 0usize), (1, 1), (8, 16), (64, 32), (100, 100)] {
                let pw = rng.bytes(pwlen);
                let salt = rng.bytes(saltlen);
                let mut a = canary(dklen.max(1));
                let mut b = canary(dklen.max(1));
                unsafe {
                    c(pw.as_ptr(), pwlen, salt.as_ptr(), saltlen, c_iter, a.as_mut_ptr(), dklen);
                    r(pw.as_ptr(), pwlen, salt.as_ptr(), saltlen, c_iter, b.as_mut_ptr(), dklen);
                }
                eq_bytes(
                    &format!("_sodium_escrypt_PBKDF2_SHA256(c={c_iter},dk={dklen},pw={pwlen},salt={saltlen})"),
                    &a,
                    &b,
                );
            }
        }
    }
}

// ===========================================================================
// _sodium_fe25519_* / _sodium_ge25519_* / _sodium_sc25519_* /
// _sodium_ristretto255_* / _sodium_core_h2c_string_to_hash
// ===========================================================================

type FeFrombytes = unsafe extern "C" fn(*mut u8, *const u8);
type FeTobytes = unsafe extern "C" fn(*mut u8, *const u8);
type FeInvert = unsafe extern "C" fn(*mut u8, *const u8);
type GeTobytes = unsafe extern "C" fn(*mut u8, *const u8);
type GeFrombytes = unsafe extern "C" fn(*mut u8, *const u8) -> i32;
type GeConv = unsafe extern "C" fn(*mut u8, *const u8);
type GeAdd = unsafe extern "C" fn(*mut u8, *const u8, *const u8);
type GeScalarmultBase = unsafe extern "C" fn(*mut u8, *const u8);
type GeScalarmult = unsafe extern "C" fn(*mut u8, *const u8, *const u8);
type GeDoubleScalarmult = unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8);
type GePred = unsafe extern "C" fn(*const u8) -> i32;
type GeClear = unsafe extern "C" fn(*mut u8);
type GeFromBytes32 = unsafe extern "C" fn(*mut u8, *const u8);
type ScInvert = unsafe extern "C" fn(*mut u8, *const u8);
type ScReduce = unsafe extern "C" fn(*mut u8);
type ScMul = unsafe extern "C" fn(*mut u8, *const u8, *const u8);
type ScMulAdd = unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8);
type H2c = unsafe extern "C" fn(*mut u8, usize, *const u8, usize, *const u8, usize, i32) -> i32;

/// Interesting 32-byte encodings: canonical, non-canonical, small-order,
/// off-subgroup, and random valid points.
fn point_corpus(rng: &mut Rng) -> Vec<Vec<u8>> {
    let mut v: Vec<Vec<u8>> = vec![
        vec![0u8; 32],
        vec![0xffu8; 32],
        {
            let mut x = vec![0u8; 32];
            x[0] = 1;
            x
        },
        {
            let mut x = vec![0xffu8; 32];
            x[0] = 0xec;
            x[31] = 0x7f;
            x
        }, // ec ff..ff 7f
        {
            let mut x = vec![0xffu8; 32];
            x[0] = 0xed;
            x[31] = 0x7f;
            x
        },
        {
            let mut x = vec![0u8; 32];
            x[31] = 0x80;
            x
        },
    ];
    for b in [2u8, 3, 4, 5, 6, 7, 8, 9, 0x0a] {
        let mut x = vec![0u8; 32];
        x[0] = b;
        v.push(x);
    }
    // random valid points via crypto_core_ed25519_random
    let f = sym::<unsafe extern "C" fn(*mut u8)>(c_lib(), "crypto_core_ed25519_random");
    for s in 0..12u64 {
        let mut p = vec![0u8; 32];
        reset_rngs(0x1_0000 + s);
        unsafe { f(p.as_mut_ptr()) };
        v.push(p);
    }
    for _ in 0..12 {
        v.push(rng.bytes(32));
    }
    v
}

/// `_sodium_fe25519_frombytes` / `_tobytes` / `_invert` — the raw field ops.
/// The `fe25519` representation (`int32_t[10]`) is compared byte-for-byte
/// because these exported functions take `fe25519` across the ABI boundary.
#[test]
fn fe25519_internal_exports() {
    setup();
    let mut rng = Rng::new(0xFE00);
    let (cfb, rfb) = pair::<FeFrombytes>("_sodium_fe25519_frombytes");
    let (ctb, rtb) = pair::<FeTobytes>("_sodium_fe25519_tobytes");
    let (cin, rin) = pair::<FeInvert>("_sodium_fe25519_invert");

    let mut inputs: Vec<Vec<u8>> = vec![vec![0u8; 32], vec![0xffu8; 32]];
    for i in 0..32 {
        let mut x = vec![0u8; 32];
        x[i] = 0xff;
        inputs.push(x);
    }
    for _ in 0..64 {
        inputs.push(rng.bytes(32));
    }
    for s in &inputs {
        let mut fe = [vec![0u8; FE], vec![0u8; FE]];
        for (w, f) in [cfb, rfb].into_iter().enumerate() {
            unsafe { f(fe[w].as_mut_ptr(), s.as_ptr()) };
        }
        eq_bytes(&format!("_sodium_fe25519_frombytes({})", hex(s)), &fe[0], &fe[1]);

        let mut back = [canary(32), canary(32)];
        for (w, f) in [ctb, rtb].into_iter().enumerate() {
            unsafe { f(back[w].as_mut_ptr(), fe[0].as_ptr()) };
        }
        let (a, b) = (back[0].clone(), back[1].clone());
        eq_bytes(&format!("_sodium_fe25519_tobytes({})", hex(s)), &a, &b);

        let mut inv = [vec![0u8; FE], vec![0u8; FE]];
        for (w, f) in [cin, rin].into_iter().enumerate() {
            unsafe { f(inv[w].as_mut_ptr(), fe[0].as_ptr()) };
        }
        eq_bytes(&format!("_sodium_fe25519_invert({})", hex(s)), &inv[0], &inv[1]);
        // and the canonical encodings of the inverse must agree
        let mut e = [canary(32), canary(32)];
        for (w, f) in [ctb, rtb].into_iter().enumerate() {
            unsafe { f(e[w].as_mut_ptr(), inv[0].as_ptr()) };
        }
        let (a, b) = (e[0].clone(), e[1].clone());
        eq_bytes("fe25519_invert encoded", &a, &b);
    }
}

/// `_sodium_sc25519_*` — scalar arithmetic.
#[test]
fn sc25519_internal_exports() {
    setup();
    let mut rng = Rng::new(0x5C00);
    let (cinv, rinv) = pair::<ScInvert>("_sodium_sc25519_invert");
    let (cred, rred) = pair::<ScReduce>("_sodium_sc25519_reduce");
    let (cmul, rmul) = pair::<ScMul>("_sodium_sc25519_mul");
    let (cma, rma) = pair::<ScMulAdd>("_sodium_sc25519_muladd");
    let (cis, ris) = pair::<GePred>("_sodium_sc25519_is_canonical");

    // L = 2^252 + 27742317777372353535851937790883648493
    let l: [u8; 32] = [
        0xed, 0xd3, 0xf5, 0x5c, 0x1a, 0x63, 0x12, 0x58, 0xd6, 0x9c, 0xf7, 0xa2, 0xde, 0xf9, 0xde,
        0x14, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x10,
    ];
    let mut lm1 = l;
    lm1[0] -= 1;
    let mut lp1 = l;
    lp1[0] += 1;

    let mut scalars: Vec<Vec<u8>> = vec![
        vec![0u8; 32],
        {
            let mut x = vec![0u8; 32];
            x[0] = 1;
            x
        },
        {
            let mut x = vec![0u8; 32];
            x[0] = 2;
            x
        },
        l.to_vec(),
        lm1.to_vec(),
        lp1.to_vec(),
        vec![0xffu8; 32],
    ];
    for _ in 0..40 {
        scalars.push(rng.bytes(32));
    }

    for s in &scalars {
        // is_canonical
        let (a, b) = unsafe { (cis(s.as_ptr()), ris(s.as_ptr())) };
        eq_i32(&format!("_sodium_sc25519_is_canonical({})", hex(s)), a, b);
        // invert
        let mut inv = [canary(32), canary(32)];
        for (w, f) in [cinv, rinv].into_iter().enumerate() {
            unsafe { f(inv[w].as_mut_ptr(), s.as_ptr()) };
        }
        let (x, y) = (inv[0].clone(), inv[1].clone());
        eq_bytes(&format!("_sodium_sc25519_invert({})", hex(s)), &x, &y);
        // mul / muladd with random partners
        for _ in 0..3 {
            let t = rng.bytes(32);
            let u = rng.bytes(32);
            let mut o = [canary(32), canary(32)];
            for (w, f) in [cmul, rmul].into_iter().enumerate() {
                unsafe { f(o[w].as_mut_ptr(), s.as_ptr(), t.as_ptr()) };
            }
            let (x, y) = (o[0].clone(), o[1].clone());
            eq_bytes("_sodium_sc25519_mul", &x, &y);
            let mut o = [canary(32), canary(32)];
            for (w, f) in [cma, rma].into_iter().enumerate() {
                unsafe { f(o[w].as_mut_ptr(), s.as_ptr(), t.as_ptr(), u.as_ptr()) };
            }
            let (x, y) = (o[0].clone(), o[1].clone());
            eq_bytes("_sodium_sc25519_muladd", &x, &y);
        }
    }

    // reduce operates in place on 64 bytes
    let mut wide: Vec<Vec<u8>> = vec![vec![0u8; 64], vec![0xffu8; 64]];
    for i in 0..64 {
        let mut x = vec![0u8; 64];
        x[i] = 0xff;
        wide.push(x);
    }
    for _ in 0..40 {
        wide.push(rng.bytes(64));
    }
    for s in &wide {
        let mut o = [s.clone(), s.clone()];
        for (w, f) in [cred, rred].into_iter().enumerate() {
            unsafe { f(o[w].as_mut_ptr()) };
        }
        let (x, y) = (o[0].clone(), o[1].clone());
        eq_bytes(&format!("_sodium_sc25519_reduce({})", hex(&s[..16])), &x, &y);
    }
}

/// `_sodium_ge25519_*` — group element decode / encode / add / sub /
/// scalarmult / predicates / cofactor clearing / hash-to-curve.
#[test]
fn ge25519_internal_exports() {
    setup();
    let mut rng = Rng::new(0x6E00);
    let (cfb, rfb) = pair::<GeFrombytes>("_sodium_ge25519_frombytes");
    let (cfbn, rfbn) = pair::<GeFrombytes>("_sodium_ge25519_frombytes_negate_vartime");
    let (cp3tb, rp3tb) = pair::<GeTobytes>("_sodium_ge25519_p3_tobytes");
    let (ctb, rtb) = pair::<GeTobytes>("_sodium_ge25519_tobytes");
    let (cadd, radd) = pair::<GeAdd>("_sodium_ge25519_p3_add");
    let (csub, rsub) = pair::<GeAdd>("_sodium_ge25519_p3_sub");
    let (csb, rsb) = pair::<GeScalarmultBase>("_sodium_ge25519_scalarmult_base");
    let (csm, rsm) = pair::<GeScalarmult>("_sodium_ge25519_scalarmult");
    let (cds, rds) = pair::<GeDoubleScalarmult>("_sodium_ge25519_double_scalarmult_vartime");
    let (cisc, risc) = pair::<GePred>("_sodium_ge25519_is_canonical");
    let (cioc, rioc) = pair::<unsafe extern "C" fn(*const u8) -> i32>("_sodium_ge25519_is_on_curve");
    let (cims, rims) =
        pair::<unsafe extern "C" fn(*const u8) -> i32>("_sodium_ge25519_is_on_main_subgroup");
    let (chso, rhso) =
        pair::<unsafe extern "C" fn(*const u8) -> i32>("_sodium_ge25519_has_small_order");
    let (ccc, rcc) = pair::<GeClear>("_sodium_ge25519_clear_cofactor");
    let (cfu, rfu) = pair::<GeFromBytes32>("_sodium_ge25519_from_uniform");
    let (cfh, rfh) = pair::<GeFromBytes32>("_sodium_ge25519_from_hash");
    let (cp1p2, rp1p2) = pair::<GeConv>("_sodium_ge25519_p1p1_to_p2");
    let (cp1p3, rp1p3) = pair::<GeConv>("_sodium_ge25519_p1p1_to_p3");
    let (cp2p3, rp2p3) = pair::<GeConv>("_sodium_ge25519_p2_to_p3");

    let corpus = point_corpus(&mut rng);

    // is_canonical works on the encoded form
    for s in &corpus {
        let (a, b) = unsafe { (cisc(s.as_ptr()), risc(s.as_ptr())) };
        eq_i32(&format!("_sodium_ge25519_is_canonical({})", hex(s)), a, b);
    }

    // frombytes / frombytes_negate_vartime + the p3 predicates + tobytes
    let mut valid_p3: Vec<Vec<u8>> = Vec::new();
    for s in &corpus {
        for (label, cf, rf) in [
            ("frombytes", cfb, rfb),
            ("frombytes_negate_vartime", cfbn, rfbn),
        ] {
            let mut p3 = [vec![0u8; GE_P3], vec![0u8; GE_P3]];
            let mut rcs = [0i32; 2];
            for (w, f) in [cf, rf].into_iter().enumerate() {
                rcs[w] = unsafe { f(p3[w].as_mut_ptr(), s.as_ptr()) };
            }
            eq_i32(&format!("_sodium_ge25519_{label}({}) rc", hex(s)), rcs[0], rcs[1]);
            eq_bytes(
                &format!("_sodium_ge25519_{label}({}) p3", hex(s)),
                &p3[0],
                &p3[1],
            );
            if rcs[0] == 0 {
                if label == "frombytes" {
                    valid_p3.push(p3[0].clone());
                }
                // predicates on the decoded point
                for (pl, cp, rp) in [
                    ("is_on_curve", cioc, rioc),
                    ("is_on_main_subgroup", cims, rims),
                    ("has_small_order", chso, rhso),
                ] {
                    let (a, b) = unsafe { (cp(p3[0].as_ptr()), rp(p3[0].as_ptr())) };
                    eq_i32(&format!("_sodium_ge25519_{pl}({})", hex(s)), a, b);
                }
                // p3_tobytes must round-trip
                let mut enc = [canary(32), canary(32)];
                for (w, f) in [cp3tb, rp3tb].into_iter().enumerate() {
                    unsafe { f(enc[w].as_mut_ptr(), p3[0].as_ptr()) };
                }
                let (x, y) = (enc[0].clone(), enc[1].clone());
                eq_bytes(&format!("_sodium_ge25519_p3_tobytes({})", hex(s)), &x, &y);
                // ge25519_tobytes reads a p2 (X,Y,Z) — the first 120 bytes of
                // the p3 have exactly that layout
                let mut enc = [canary(32), canary(32)];
                for (w, f) in [ctb, rtb].into_iter().enumerate() {
                    unsafe { f(enc[w].as_mut_ptr(), p3[0].as_ptr()) };
                }
                let (x, y) = (enc[0].clone(), enc[1].clone());
                eq_bytes(&format!("_sodium_ge25519_tobytes({})", hex(s)), &x, &y);
                // clear_cofactor mutates a p3 in place
                let mut cf3 = [p3[0].clone(), p3[0].clone()];
                for (w, f) in [ccc, rcc].into_iter().enumerate() {
                    unsafe { f(cf3[w].as_mut_ptr()) };
                }
                eq_bytes(
                    &format!("_sodium_ge25519_clear_cofactor({})", hex(s)),
                    &cf3[0],
                    &cf3[1],
                );
            }
        }
    }
    assert!(valid_p3.len() >= 8, "need some decodable points");

    // p3_add / p3_sub over pairs of valid points
    for i in 0..valid_p3.len().min(14) {
        for j in 0..valid_p3.len().min(14) {
            for (label, cf, rf) in [("p3_add", cadd, radd), ("p3_sub", csub, rsub)] {
                let mut o = [vec![0u8; GE_P3], vec![0u8; GE_P3]];
                for (w, f) in [cf, rf].into_iter().enumerate() {
                    unsafe { f(o[w].as_mut_ptr(), valid_p3[i].as_ptr(), valid_p3[j].as_ptr()) };
                }
                eq_bytes(&format!("_sodium_ge25519_{label}({i},{j})"), &o[0], &o[1]);
                let mut enc = [canary(32), canary(32)];
                for (w, f) in [cp3tb, rp3tb].into_iter().enumerate() {
                    unsafe { f(enc[w].as_mut_ptr(), o[0].as_ptr()) };
                }
                let (x, y) = (enc[0].clone(), enc[1].clone());
                eq_bytes(&format!("_sodium_ge25519_{label} encoded"), &x, &y);
            }
        }
    }

    // scalarmult_base / scalarmult / double_scalarmult_vartime
    let mut scalars: Vec<Vec<u8>> = vec![vec![0u8; 32], vec![0xffu8; 32], {
        let mut x = vec![0u8; 32];
        x[0] = 1;
        x
    }];
    for _ in 0..12 {
        scalars.push(rng.bytes(32));
    }
    for a in &scalars {
        let mut o = [vec![0u8; GE_P3], vec![0u8; GE_P3]];
        for (w, f) in [csb, rsb].into_iter().enumerate() {
            unsafe { f(o[w].as_mut_ptr(), a.as_ptr()) };
        }
        eq_bytes(&format!("_sodium_ge25519_scalarmult_base({})", hex(a)), &o[0], &o[1]);

        for p in valid_p3.iter().take(6) {
            let mut q = [vec![0u8; GE_P3], vec![0u8; GE_P3]];
            for (w, f) in [csm, rsm].into_iter().enumerate() {
                unsafe { f(q[w].as_mut_ptr(), a.as_ptr(), p.as_ptr()) };
            }
            eq_bytes("_sodium_ge25519_scalarmult", &q[0], &q[1]);

            let b = rng.bytes(32);
            let mut r2 = [vec![0u8; GE_P2], vec![0u8; GE_P2]];
            for (w, f) in [cds, rds].into_iter().enumerate() {
                unsafe { f(r2[w].as_mut_ptr(), a.as_ptr(), p.as_ptr(), b.as_ptr()) };
            }
            eq_bytes("_sodium_ge25519_double_scalarmult_vartime", &r2[0], &r2[1]);
            // encode the p2 result
            let mut enc = [canary(32), canary(32)];
            for (w, f) in [ctb, rtb].into_iter().enumerate() {
                unsafe { f(enc[w].as_mut_ptr(), r2[0].as_ptr()) };
            }
            let (x, y) = (enc[0].clone(), enc[1].clone());
            eq_bytes("double_scalarmult encoded", &x, &y);

            // p2 -> p3 conversion
            let mut p3 = [vec![0u8; GE_P3], vec![0u8; GE_P3]];
            for (w, f) in [cp2p3, rp2p3].into_iter().enumerate() {
                unsafe { f(p3[w].as_mut_ptr(), r2[0].as_ptr()) };
            }
            eq_bytes("_sodium_ge25519_p2_to_p3", &p3[0], &p3[1]);
        }
    }

    // p1p1 -> p2 / p3. A ge25519_p1p1 is 4 field elements; feed it the bytes of
    // a decoded p3 (same 160-byte shape) so the conversion is well-defined.
    for p in valid_p3.iter().take(8) {
        let mut o2 = [vec![0u8; GE_P2], vec![0u8; GE_P2]];
        for (w, f) in [cp1p2, rp1p2].into_iter().enumerate() {
            unsafe { f(o2[w].as_mut_ptr(), p.as_ptr()) };
        }
        eq_bytes("_sodium_ge25519_p1p1_to_p2", &o2[0], &o2[1]);
        let mut o3 = [vec![0u8; GE_P3], vec![0u8; GE_P3]];
        for (w, f) in [cp1p3, rp1p3].into_iter().enumerate() {
            unsafe { f(o3[w].as_mut_ptr(), p.as_ptr()) };
        }
        eq_bytes("_sodium_ge25519_p1p1_to_p3", &o3[0], &o3[1]);
    }

    // from_uniform (32-byte input) / from_hash (64-byte input)
    let mut uni: Vec<Vec<u8>> = vec![vec![0u8; 32], vec![0xffu8; 32]];
    for _ in 0..40 {
        uni.push(rng.bytes(32));
    }
    for r in &uni {
        let mut o = [canary(32), canary(32)];
        for (w, f) in [cfu, rfu].into_iter().enumerate() {
            unsafe { f(o[w].as_mut_ptr(), r.as_ptr()) };
        }
        let (x, y) = (o[0].clone(), o[1].clone());
        eq_bytes(&format!("_sodium_ge25519_from_uniform({})", hex(r)), &x, &y);
    }
    let mut hs: Vec<Vec<u8>> = vec![vec![0u8; 64], vec![0xffu8; 64]];
    for _ in 0..40 {
        hs.push(rng.bytes(64));
    }
    for h in &hs {
        let mut o = [canary(32), canary(32)];
        for (w, f) in [cfh, rfh].into_iter().enumerate() {
            unsafe { f(o[w].as_mut_ptr(), h.as_ptr()) };
        }
        let (x, y) = (o[0].clone(), o[1].clone());
        eq_bytes(&format!("_sodium_ge25519_from_hash({})", hex(&h[..16])), &x, &y);
    }
}

/// `_sodium_ristretto255_frombytes` / `_p3_tobytes` / `_from_hash`.
#[test]
fn ristretto255_internal_exports() {
    setup();
    let mut rng = Rng::new(0x7500);
    let (cfb, rfb) = pair::<GeFrombytes>("_sodium_ristretto255_frombytes");
    let (ctb, rtb) = pair::<GeTobytes>("_sodium_ristretto255_p3_tobytes");
    let (cfh, rfh) = pair::<GeFromBytes32>("_sodium_ristretto255_from_hash");

    let mut corpus: Vec<Vec<u8>> = vec![vec![0u8; 32], vec![0xffu8; 32]];
    for i in 0..32 {
        let mut x = vec![0u8; 32];
        x[i] = 1;
        corpus.push(x);
    }
    // valid ristretto points
    let f = sym::<unsafe extern "C" fn(*mut u8)>(c_lib(), "crypto_core_ristretto255_random");
    for s in 0..12u64 {
        let mut p = vec![0u8; 32];
        reset_rngs(0x2_0000 + s);
        unsafe { f(p.as_mut_ptr()) };
        corpus.push(p);
    }
    for _ in 0..40 {
        corpus.push(rng.bytes(32));
    }

    for s in &corpus {
        let mut p3 = [vec![0u8; GE_P3], vec![0u8; GE_P3]];
        let mut rcs = [0i32; 2];
        for (w, fx) in [cfb, rfb].into_iter().enumerate() {
            rcs[w] = unsafe { fx(p3[w].as_mut_ptr(), s.as_ptr()) };
        }
        eq_i32(&format!("_sodium_ristretto255_frombytes({}) rc", hex(s)), rcs[0], rcs[1]);
        eq_bytes(
            &format!("_sodium_ristretto255_frombytes({}) p3", hex(s)),
            &p3[0],
            &p3[1],
        );
        if rcs[0] == 0 {
            let mut enc = [canary(32), canary(32)];
            for (w, fx) in [ctb, rtb].into_iter().enumerate() {
                unsafe { fx(enc[w].as_mut_ptr(), p3[0].as_ptr()) };
            }
            let (x, y) = (enc[0].clone(), enc[1].clone());
            eq_bytes(&format!("_sodium_ristretto255_p3_tobytes({})", hex(s)), &x, &y);
        }
    }

    let mut hs: Vec<Vec<u8>> = vec![vec![0u8; 64], vec![0xffu8; 64]];
    for _ in 0..40 {
        hs.push(rng.bytes(64));
    }
    for h in &hs {
        let mut o = [canary(32), canary(32)];
        for (w, fx) in [cfh, rfh].into_iter().enumerate() {
            unsafe { fx(o[w].as_mut_ptr(), h.as_ptr()) };
        }
        let (x, y) = (o[0].clone(), o[1].clone());
        eq_bytes(
            &format!("_sodium_ristretto255_from_hash({})", hex(&h[..16])),
            &x,
            &y,
        );
    }
}

/// `_sodium_core_h2c_string_to_hash` — both hash algorithms, `ctx_len` on both
/// sides of the 0xff DST boundary, and out-of-range `hash_alg` values.
#[test]
fn core_h2c_internal_export() {
    setup();
    let mut rng = Rng::new(0x8200);
    let (c, r) = pair::<H2c>("_sodium_core_h2c_string_to_hash");
    // CORE_H2C_SHA256 = 1, CORE_H2C_SHA512 = 2
    for &alg in &[1i32, 2, 0, 3, -1, i32::MAX, i32::MIN] {
        for &h_len in &[0usize, 1, 32, 48, 64, 96, 128] {
            for &ctx_len in &[0usize, 1, 32, 254, 255, 256, 300] {
                for &msg_len in &[0usize, 1, 32, 100] {
                    let ctx = rng.bytes(ctx_len);
                    let msg = rng.bytes(msg_len);
                    let mut a = canary(h_len.max(1));
                    let mut b = canary(h_len.max(1));
                    let (ra, rb) = unsafe {
                        (
                            c(a.as_mut_ptr(), h_len, ctx.as_ptr(), ctx_len,
                              msg.as_ptr(), msg_len, alg),
                            r(b.as_mut_ptr(), h_len, ctx.as_ptr(), ctx_len,
                              msg.as_ptr(), msg_len, alg),
                        )
                    };
                    eq_i32(
                        &format!("_sodium_core_h2c_string_to_hash rc(alg={alg},h={h_len},ctx={ctx_len},msg={msg_len})"),
                        ra,
                        rb,
                    );
                    eq_bytes(
                        &format!("_sodium_core_h2c_string_to_hash(alg={alg},h={h_len},ctx={ctx_len},msg={msg_len})"),
                        &a,
                        &b,
                    );
                }
            }
        }
    }
}

// ===========================================================================
// Out-of-process rows: `_sodium_argon2_encode_string` with a `dst_len` that is
// large enough to reach the base64 stage but too small for it. The C reaches
// `sodium_bin2base64`, whose size guard is `sodium_misuse()` -> `abort()`, so
// `SB`'s `return ARGON2_ENCODING_FAIL` is unreachable. Both libraries must abort
// identically.
// ===========================================================================

#[test]
fn encode_string_misuse_child() {
    let Some((tag, lib)) = child_case() else {
        return;
    };
    let dstlen: usize = tag
        .rsplit('/')
        .next()
        .unwrap()
        .parse()
        .expect("dstlen in tag");
    let ty: i32 = if tag.contains("argon2id") { ARGON2_ID } else { ARGON2_I };
    let mut pwd = vec![7u8; 16];
    let mut salt = vec![9u8; 16];
    let mut out = vec![3u8; 32];
    let mut dst = vec![0xA5u8; 400];
    set_observation(dst.as_ptr(), 128);
    let f = sym::<Argon2Encode>(lib, "_sodium_argon2_encode_string");
    let mut ctx = Argon2Context {
        out: out.as_mut_ptr(),
        outlen: 32,
        pwd: pwd.as_mut_ptr(),
        pwdlen: 16,
        salt: salt.as_mut_ptr(),
        saltlen: 16,
        secret: std::ptr::null_mut(),
        secretlen: 0,
        ad: std::ptr::null_mut(),
        adlen: 0,
        t_cost: 3,
        m_cost: 32,
        lanes: 1,
        threads: 1,
        flags: 0,
    };
    let rc = unsafe { f(dst.as_mut_ptr() as *mut c_char, dstlen, &mut ctx, ty) };
    println!("OBS rc={rc} dst={}", hex(&dst[..128]));
    use std::io::Write;
    let _ = std::io::stdout().flush();
    std::process::exit(0);
}

/// Sweep **every** `dst_len` from 0 to 100 (plus 128 and 256) for both
/// variants, out of process, and require the C and Rust libraries to produce
/// the identical outcome for each — whether that outcome is a clean
/// `ARGON2_ENCODING_FAIL`, a `sodium_misuse()` abort, or success. Sweeping the
/// whole range means the exact abort/clean-fail boundaries are discovered from
/// the C rather than assumed.
#[test]
fn encode_string_dst_len_sweep_matches_out_of_process() {
    if child_tag().is_some() {
        return;
    }
    setup();
    let mut aborted = 0usize;
    let mut clean = 0usize;
    let mut ok = 0usize;
    for variant in ["argon2i", "argon2id"] {
        let mut lens: Vec<usize> = (0..=100).collect();
        lens.extend([128usize, 256]);
        for dstlen in lens {
            let tag = format!("encode/{variant}/{dstlen}");
            let c = run_child("encode_string_misuse_child", "c", &tag);
            let r = run_child("encode_string_misuse_child", "r", &tag);
            eq_child(&tag, &c, &r);
            match c.status.code() {
                Some(MISUSE_EXIT) => aborted += 1,
                Some(0) => {
                    let out = String::from_utf8_lossy(&c.stdout);
                    if out.contains("rc=0 ") {
                        ok += 1;
                    } else {
                        clean += 1;
                    }
                }
                other => panic!("{tag}: unexpected C exit {other:?}"),
            }
        }
    }
    // the three outcome classes must all actually occur, otherwise the sweep
    // is not exercising the branch this row is about
    assert!(aborted > 0, "no dst_len reached sodium_misuse");
    assert!(clean > 0, "no dst_len produced a clean ARGON2_ENCODING_FAIL");
    assert!(ok > 0, "no dst_len succeeded");
    eprintln!("dst_len sweep: {aborted} abort, {clean} clean-fail, {ok} success");
}

// ===========================================================================
// `_sodium_argon2_initialize` / `_fill_memory_blocks` / `_fill_segment_ref` /
// `_finalize` — driven **directly**, replicating exactly the sequence
// `argon2_ctx` performs (argon2.c), so these four exported internals are
// exercised on their own rather than only through the wrapper.
//
// `argon2_instance_t` layout (argon2-core.h):
//   block_region *region;  uint64_t *pseudo_rands;
//   u32 passes, current_pass, memory_blocks, segment_length, lane_length,
//       lanes, threads;  argon2_type type;  int print_internals;
// `ARGON2_SYNC_POINTS` is 4.
// ===========================================================================

const ARGON2_SYNC_POINTS: u32 = 4;

#[repr(C)]
#[derive(Clone, Copy)]
struct Argon2Instance {
    region: *mut c_void,
    pseudo_rands: *mut u64,
    passes: u32,
    current_pass: u32,
    memory_blocks: u32,
    segment_length: u32,
    lane_length: u32,
    lanes: u32,
    threads: u32,
    ty: i32,
    print_internals: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Argon2Position {
    pass: u32,
    lane: u32,
    slice: u8,
    index: u32,
}

type A2Initialize = unsafe extern "C" fn(*mut Argon2Instance, *mut Argon2Context) -> i32;
type A2FillMemory = unsafe extern "C" fn(*mut Argon2Instance, u32);
type A2Finalize = unsafe extern "C" fn(*const Argon2Context, *mut Argon2Instance);
type A2FillSegment = unsafe extern "C" fn(*const Argon2Instance, Argon2Position);

#[test]
fn argon2_internal_initialize_fill_finalize() {
    setup();
    let mut rng = Rng::new(0xA203);
    let (cinit, rinit) = pair::<A2Initialize>("_sodium_argon2_initialize");
    let (cfill, rfill) = pair::<A2FillMemory>("_sodium_argon2_fill_memory_blocks");
    let (cfin, rfin) = pair::<A2Finalize>("_sodium_argon2_finalize");
    let (cseg, rseg) = pair::<A2FillSegment>("_sodium_argon2_fill_segment_ref");
    let (cctx, rctx) = pair::<Argon2Ctx>("_sodium_argon2_ctx");

    for &ty in &[ARGON2_I, ARGON2_ID] {
        for &t_cost in &[1u32, 2, 3] {
            for &lanes in &[1u32, 2, 4] {
                for &m_cost in &[8 * lanes, 32 * lanes, 64, 256] {
                    if m_cost < 8 * lanes {
                        continue;
                    }
                    for &outlen in &[16usize, 32, 64] {
                        let pwd_src = rng.bytes(16);
                        let salt_src = rng.bytes(16);

                        // --- (a) the manual sequence, per library -----------
                        let mut manual = [canary(outlen), canary(outlen)];
                        let mut rcs = [0i32; 2];
                        for (w, (init, fill, fin)) in
                            [(cinit, cfill, cfin), (rinit, rfill, rfin)]
                                .into_iter()
                                .enumerate()
                        {
                            let mut pwd = pwd_src.clone();
                            let mut salt = salt_src.clone();
                            let mut ctx = Argon2Context {
                                out: manual[w].as_mut_ptr(),
                                outlen: outlen as u32,
                                pwd: pwd.as_mut_ptr(),
                                pwdlen: 16,
                                salt: salt.as_mut_ptr(),
                                saltlen: 16,
                                secret: std::ptr::null_mut(),
                                secretlen: 0,
                                ad: std::ptr::null_mut(),
                                adlen: 0,
                                t_cost,
                                m_cost,
                                lanes,
                                threads: lanes,
                                flags: 0,
                            };
                            // exactly argon2_ctx's memory alignment arithmetic
                            let mut memory_blocks = m_cost;
                            if memory_blocks < 2 * ARGON2_SYNC_POINTS * lanes {
                                memory_blocks = 2 * ARGON2_SYNC_POINTS * lanes;
                            }
                            let segment_length = memory_blocks / (lanes * ARGON2_SYNC_POINTS);
                            memory_blocks = segment_length * (lanes * ARGON2_SYNC_POINTS);
                            let mut inst = Argon2Instance {
                                region: std::ptr::null_mut(),
                                pseudo_rands: std::ptr::null_mut(),
                                passes: t_cost,
                                current_pass: u32::MAX,
                                memory_blocks,
                                segment_length,
                                lane_length: segment_length * ARGON2_SYNC_POINTS,
                                lanes,
                                threads: lanes,
                                ty,
                                print_internals: 0,
                            };
                            unsafe {
                                rcs[w] = init(&mut inst, &mut ctx);
                                assert_eq!(rcs[w], 0, "_sodium_argon2_initialize");
                                for pass in 0..inst.passes {
                                    fill(&mut inst, pass);
                                }
                                fin(&ctx, &mut inst);
                            }
                        }
                        eq_i32("_sodium_argon2_initialize rc", rcs[0], rcs[1]);
                        let (a, b) = (manual[0].clone(), manual[1].clone());
                        eq_bytes(
                            &format!(
                                "manual init/fill/finalize(ty={ty},t={t_cost},m={m_cost},lanes={lanes},out={outlen})"
                            ),
                            &a,
                            &b,
                        );

                        // --- (b) it must equal the wrapper's own output -----
                        let mut viactx = [canary(outlen), canary(outlen)];
                        for (w, f) in [cctx, rctx].into_iter().enumerate() {
                            let mut pwd = pwd_src.clone();
                            let mut salt = salt_src.clone();
                            let mut ctx = Argon2Context {
                                out: viactx[w].as_mut_ptr(),
                                outlen: outlen as u32,
                                pwd: pwd.as_mut_ptr(),
                                pwdlen: 16,
                                salt: salt.as_mut_ptr(),
                                saltlen: 16,
                                secret: std::ptr::null_mut(),
                                secretlen: 0,
                                ad: std::ptr::null_mut(),
                                adlen: 0,
                                t_cost,
                                m_cost,
                                lanes,
                                threads: lanes,
                                flags: 0,
                            };
                            let rc = unsafe { f(&mut ctx, ty) };
                            assert_eq!(rc, 0);
                        }
                        let (x, y) = (viactx[0].clone(), viactx[1].clone());
                        eq_bytes("argon2_ctx C vs Rust", &x, &y);
                        eq_bytes(
                            "manual sequence == argon2_ctx",
                            &manual[0],
                            &viactx[0],
                        );
                    }
                }
            }
        }
    }
    let _ = (cseg, rseg);
}

/// `_sodium_argon2_fill_segment_ref` called directly on a freshly initialised
/// instance, for every `(pass, lane, slice)` position, comparing the resulting
/// digest. This is the single hottest inner routine of Argon2 and the one most
/// likely to diverge.
#[test]
fn argon2_internal_fill_segment_ref_direct() {
    setup();
    let mut rng = Rng::new(0xA204);
    let (cinit, rinit) = pair::<A2Initialize>("_sodium_argon2_initialize");
    let (cseg, rseg) = pair::<A2FillSegment>("_sodium_argon2_fill_segment_ref");
    let (cfin, rfin) = pair::<A2Finalize>("_sodium_argon2_finalize");

    for &ty in &[ARGON2_I, ARGON2_ID] {
        for &lanes in &[1u32, 2] {
            for &m_cost in &[8 * lanes, 32 * lanes, 128] {
                for &t_cost in &[1u32, 2] {
                    let pwd_src = rng.bytes(16);
                    let salt_src = rng.bytes(16);
                    let outlen = 32usize;
                    let mut out = [canary(outlen), canary(outlen)];
                    for (w, (init, seg, fin)) in
                        [(cinit, cseg, cfin), (rinit, rseg, rfin)].into_iter().enumerate()
                    {
                        let mut pwd = pwd_src.clone();
                        let mut salt = salt_src.clone();
                        let mut ctx = Argon2Context {
                            out: out[w].as_mut_ptr(),
                            outlen: outlen as u32,
                            pwd: pwd.as_mut_ptr(),
                            pwdlen: 16,
                            salt: salt.as_mut_ptr(),
                            saltlen: 16,
                            secret: std::ptr::null_mut(),
                            secretlen: 0,
                            ad: std::ptr::null_mut(),
                            adlen: 0,
                            t_cost,
                            m_cost,
                            lanes,
                            threads: lanes,
                            flags: 0,
                        };
                        let mut memory_blocks = m_cost;
                        if memory_blocks < 2 * ARGON2_SYNC_POINTS * lanes {
                            memory_blocks = 2 * ARGON2_SYNC_POINTS * lanes;
                        }
                        let segment_length = memory_blocks / (lanes * ARGON2_SYNC_POINTS);
                        memory_blocks = segment_length * (lanes * ARGON2_SYNC_POINTS);
                        let mut inst = Argon2Instance {
                            region: std::ptr::null_mut(),
                            pseudo_rands: std::ptr::null_mut(),
                            passes: t_cost,
                            current_pass: u32::MAX,
                            memory_blocks,
                            segment_length,
                            lane_length: segment_length * ARGON2_SYNC_POINTS,
                            lanes,
                            threads: lanes,
                            ty,
                            print_internals: 0,
                        };
                        unsafe {
                            assert_eq!(init(&mut inst, &mut ctx), 0);
                            // drive the segments in the same (pass, slice, lane)
                            // order `argon2_fill_memory_blocks` uses
                            for pass in 0..inst.passes {
                                inst.current_pass = pass;
                                for slice in 0..ARGON2_SYNC_POINTS {
                                    for lane in 0..inst.lanes {
                                        seg(
                                            &inst,
                                            Argon2Position {
                                                pass,
                                                lane,
                                                slice: slice as u8,
                                                index: 0,
                                            },
                                        );
                                    }
                                }
                            }
                            fin(&ctx, &mut inst);
                        }
                    }
                    let (a, b) = (out[0].clone(), out[1].clone());
                    eq_bytes(
                        &format!(
                            "_sodium_argon2_fill_segment_ref direct(ty={ty},t={t_cost},m={m_cost},lanes={lanes})"
                        ),
                        &a,
                        &b,
                    );
                }
            }
        }
    }
}
