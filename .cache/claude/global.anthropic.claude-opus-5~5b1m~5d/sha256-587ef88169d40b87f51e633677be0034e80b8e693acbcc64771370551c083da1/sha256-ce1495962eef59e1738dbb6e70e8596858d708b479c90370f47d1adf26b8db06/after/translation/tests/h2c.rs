//! Differential tests for the hash-to-curve / `*_from_string` surface.
//!
//! C ground truth:
//!   * `crypto_core/ed25519/core_h2c.c`            (`_sodium_core_h2c_string_to_hash`)
//!   * `crypto_core/ed25519/core_ed25519.c`        (`_string_to_points`,
//!     `crypto_core_ed25519_from_string{,_nu}`, `crypto_core_ed25519_scalar_from_string`)
//!   * `crypto_core/ed25519/core_ristretto255.c`   (`_string_to_element`,
//!     `crypto_core_ristretto255_from_string`, `crypto_core_ristretto255_scalar_from_string`,
//!     `crypto_core_ristretto255_from_hash`)
//!   * `crypto_core/ed25519/ref10/ed25519_ref10.c` (`ge25519_from_uniform`,
//!     `ge25519_from_hash`, `ristretto255_from_hash` — exported with the
//!     `_sodium_` prefix by `private/quirks.h`)
//!
//! Everything is called through `dlopen`/`dlsym` on both shared objects.

#[macro_use]
mod common;

use core::ffi::c_int;

/// Canary fill for every output buffer: any over-write past the documented
/// output length shows up as a byte mismatch (or as a diff against the
/// untouched reference copy).
const CANARY: u8 = 0xA5;

const CORE_H2C_SHA256: c_int = 1;
const CORE_H2C_SHA512: c_int = 2;

/// `int core_h2c_string_to_hash(unsigned char *h, const size_t h_len,
///                              const unsigned char *ctx, size_t ctx_len,
///                              const unsigned char *msg, size_t msg_len,
///                              int hash_alg)`
type H2cFn =
    unsafe extern "C" fn(*mut u8, usize, *const u8, usize, *const u8, usize, c_int) -> c_int;

/// `int crypto_core_*_{,scalar_}from_string(unsigned char *out,
///        const unsigned char *ctx, size_t ctx_len,
///        const unsigned char *msg, size_t msg_len, int hash_alg)`
type StrFn = unsafe extern "C" fn(*mut u8, *const u8, usize, *const u8, usize, c_int) -> c_int;

/// `int crypto_core_ristretto255_from_hash(unsigned char *p, const unsigned char *r)`
type HashIFn = unsafe extern "C" fn(*mut u8, *const u8) -> c_int;

/// `void ge25519_from_uniform / ge25519_from_hash / ristretto255_from_hash`
type HashVFn = unsafe extern "C" fn(*mut u8, *const u8);

// ---------------------------------------------------------------- errno ------

fn errno() -> i32 {
    std::io::Error::last_os_error().raw_os_error().unwrap_or(0)
}

/// Force errno to something that is definitely not EINVAL(22) so that a later
/// `errno == 22` observation actually proves the callee set it.
fn poison_errno() {
    let _ = std::fs::metadata("/nonexistent/h2c/errno/probe");
    // ENOENT == 2 on Linux.
    assert_ne!(errno(), 22, "errno poisoning failed");
}

// --------------------------------------------------------- h2c comparison ----

/// Output-buffer size used for all `core_h2c_string_to_hash` calls.
/// `h_len` is limited to 0xff by the C `assert()`, so 320 always leaves a
/// canary tail.
const H2C_BUF: usize = 320;

#[allow(clippy::too_many_arguments)]
fn run_h2c(
    tag: &str,
    cf: H2cFn,
    rf: H2cFn,
    h_len: usize,
    h_null: bool,
    ctx: &[u8],
    ctx_null: bool,
    msg: &[u8],
    msg_null: bool,
    alg: c_int,
) -> c_int {
    assert!(h_len <= H2C_BUF);
    let mut cb = vec![CANARY; H2C_BUF];
    let mut rb = vec![CANARY; H2C_BUF];
    let cptr: *const u8 = if ctx_null {
        core::ptr::null()
    } else {
        ctx.as_ptr()
    };
    let mptr: *const u8 = if msg_null {
        core::ptr::null()
    } else {
        msg.as_ptr()
    };
    let rc = unsafe {
        cf(
            if h_null {
                core::ptr::null_mut()
            } else {
                cb.as_mut_ptr()
            },
            h_len,
            cptr,
            ctx.len(),
            mptr,
            msg.len(),
            alg,
        )
    };
    let rr = unsafe {
        rf(
            if h_null {
                core::ptr::null_mut()
            } else {
                rb.as_mut_ptr()
            },
            h_len,
            cptr,
            ctx.len(),
            mptr,
            msg.len(),
            alg,
        )
    };
    common::eqi(tag, rc, rr);
    common::eqb(tag, &cb, &rb);
    // On failure nothing may be written at all.
    if rc != 0 {
        common::eqb(&format!("{tag} (untouched on error)"), &vec![CANARY; H2C_BUF], &cb);
    } else if !h_null {
        // Nothing past h_len may be touched.
        common::eqb(
            &format!("{tag} (canary tail)"),
            &vec![CANARY; H2C_BUF - h_len],
            &cb[h_len..],
        );
        // Sanity: the first h_len bytes really were produced (guards against a
        // vacuously-passing all-canary comparison).
        // (only for h_len >= 4; a 1-byte output can legitimately be 0xA5)
        if h_len >= 4 {
            assert!(
                cb[..h_len].iter().any(|&b| b != CANARY),
                "{tag}: output still all-canary"
            );
        }
    }
    rc
}

// =============================================================== core_h2c ====

/// h2c-1 / h2c-2: full `h_len` boundary grid for both hash ids.
///
/// SHA-256 block loop step is 32 bytes, SHA-512's is 64, so the interesting
/// values are (k*32, k*32±1) and (k*64, k*64±1) plus the documented maximum
/// 0xff (`assert(h_len <= 0xff)`).
#[test]
fn h2c_h_len_grid() {
    let (c, r) = both!("_sodium_core_h2c_string_to_hash", H2cFn);
    let mut rng = common::Rng::new(0x4831_3243);
    let ctx = b"h2c-h-len-grid".to_vec();
    for alg in [CORE_H2C_SHA256, CORE_H2C_SHA512] {
        for h_len in [
            0usize, 1, 2, 15, 16, 31, 32, 33, 47, 48, 49, 63, 64, 65, 95, 96, 97, 127, 128, 129,
            159, 160, 191, 192, 193, 223, 224, 254, 255,
        ] {
            for msg_len in [0usize, 1, 37] {
                let msg = rng.bytes(msg_len);
                let tag = format!("h2c h_len={h_len} alg={alg} msg_len={msg_len}");
                let rc = run_h2c(&tag, c, r, h_len, false, &ctx, false, &msg, false, alg);
                common::eqi(&format!("{tag} rc"), 0, rc);
            }
        }
    }
}

/// h2c-3 / h2c-4: `ctx_len` grid, including the `ctx_len > 0xff`
/// "H2C-OVERSIZE-DST-" pre-hash branch (and the aliasing quirk that the
/// pre-hashed DST lives in `u0`, which is then overwritten by the main hash,
/// so the per-block hashes consume the *new* `u0` as the DST).
#[test]
fn h2c_ctx_len_grid() {
    let (c, r) = both!("_sodium_core_h2c_string_to_hash", H2cFn);
    let mut rng = common::Rng::new(0xC0DE_1111);
    for alg in [CORE_H2C_SHA256, CORE_H2C_SHA512] {
        for ctx_len in [
            0usize, 1, 2, 16, 31, 32, 33, 63, 64, 65, 127, 128, 129, 253, 254, 255, 256, 257, 258,
            300, 511, 512, 513, 1000, 4096,
        ] {
            let ctx = rng.bytes(ctx_len);
            for h_len in [0usize, 1, 32, 48, 64, 96, 255] {
                let msg = rng.bytes(11);
                let tag = format!("h2c ctx_len={ctx_len} alg={alg} h_len={h_len}");
                let rc = run_h2c(&tag, c, r, h_len, false, &ctx, false, &msg, false, alg);
                common::eqi(&format!("{tag} rc"), 0, rc);
            }
        }
    }
}

/// h2c-5 / h2c-6: `msg_len` grid straddling both SHA block sizes (the message
/// is absorbed after a 64- resp. 128-byte zero block, so the interesting sizes
/// are around 64 and 128).
#[test]
fn h2c_msg_len_grid() {
    let (c, r) = both!("_sodium_core_h2c_string_to_hash", H2cFn);
    let mut rng = common::Rng::new(0x5EED_2222);
    let ctx = b"QUUX-V01-CS02-with-expander-SHA256-128".to_vec();
    for alg in [CORE_H2C_SHA256, CORE_H2C_SHA512] {
        for msg_len in [
            0usize, 1, 2, 55, 56, 63, 64, 65, 111, 112, 127, 128, 129, 191, 192, 255, 256, 1000,
            4096, 10000,
        ] {
            let msg = rng.bytes(msg_len);
            for h_len in [0usize, 32, 48, 64, 96] {
                let tag = format!("h2c msg_len={msg_len} alg={alg} h_len={h_len}");
                let rc = run_h2c(&tag, c, r, h_len, false, &ctx, false, &msg, false, alg);
                common::eqi(&format!("{tag} rc"), 0, rc);
            }
        }
    }
}

/// h2c-E1: `hash_alg` outside `{CORE_H2C_SHA256, CORE_H2C_SHA512}` hits the
/// `switch` default: `errno = EINVAL; return -1;` and nothing is written.
#[test]
fn h2c_bad_hash_alg() {
    let (c, r) = both!("_sodium_core_h2c_string_to_hash", H2cFn);
    let ctx = b"ctx".to_vec();
    let msg = b"message".to_vec();
    for alg in [
        0,
        3,
        4,
        -1,
        -2,
        99,
        256,
        0x1_0000,
        c_int::MIN,
        c_int::MAX,
        i32::from(u8::MAX),
    ] {
        for h_len in [0usize, 1, 32, 64, 96, 255] {
            let tag = format!("h2c bad alg={alg} h_len={h_len}");

            poison_errno();
            let mut cb = vec![CANARY; H2C_BUF];
            let rc = unsafe {
                c(
                    cb.as_mut_ptr(),
                    h_len,
                    ctx.as_ptr(),
                    ctx.len(),
                    msg.as_ptr(),
                    msg.len(),
                    alg,
                )
            };
            let c_errno = errno();

            poison_errno();
            let mut rb = vec![CANARY; H2C_BUF];
            let rr = unsafe {
                r(
                    rb.as_mut_ptr(),
                    h_len,
                    ctx.as_ptr(),
                    ctx.len(),
                    msg.as_ptr(),
                    msg.len(),
                    alg,
                )
            };
            let r_errno = errno();

            common::eqi(&tag, rc, rr);
            assert_eq!(rc, -1, "{tag}: expected -1");
            common::eqb(&tag, &cb, &rb);
            common::eqb(&format!("{tag} untouched"), &vec![CANARY; H2C_BUF], &cb);
            assert_eq!(c_errno, 22, "{tag}: C errno should be EINVAL");
            common::eqi(&format!("{tag} errno"), c_errno, r_errno);
        }
    }
}

/// h2c-E2: `h == NULL` is only reachable with `h_len == 0` (the `memcpy` loop
/// never runs); `ctx == NULL` / `msg == NULL` are tolerated with length 0
/// because `crypto_hash_sha*_update` returns early on `inlen == 0`.
#[test]
fn h2c_null_pointers() {
    let (c, r) = both!("_sodium_core_h2c_string_to_hash", H2cFn);
    let empty: [u8; 0] = [];
    for alg in [CORE_H2C_SHA256, CORE_H2C_SHA512] {
        // h = NULL, h_len = 0
        let rc = run_h2c(
            &format!("h2c h=NULL alg={alg}"),
            c,
            r,
            0,
            true,
            b"ctx",
            false,
            b"msg",
            false,
            alg,
        );
        common::eqi("h2c h=NULL rc", 0, rc);

        // ctx = NULL, ctx_len = 0
        for h_len in [0usize, 1, 32, 64] {
            let rc = run_h2c(
                &format!("h2c ctx=NULL alg={alg} h_len={h_len}"),
                c,
                r,
                h_len,
                false,
                &empty,
                true,
                b"msg",
                false,
                alg,
            );
            common::eqi("h2c ctx=NULL rc", 0, rc);
        }

        // msg = NULL, msg_len = 0
        for h_len in [0usize, 1, 32, 64] {
            let rc = run_h2c(
                &format!("h2c msg=NULL alg={alg} h_len={h_len}"),
                c,
                r,
                h_len,
                false,
                b"ctx",
                false,
                &empty,
                true,
                alg,
            );
            common::eqi("h2c msg=NULL rc", 0, rc);
        }

        // everything NULL / zero
        let rc = run_h2c(
            &format!("h2c all-NULL alg={alg}"),
            c,
            r,
            0,
            true,
            &empty,
            true,
            &empty,
            true,
            alg,
        );
        common::eqi("h2c all-NULL rc", 0, rc);

        // empty (non-NULL) ctx and msg with a real output
        let rc = run_h2c(
            &format!("h2c empty ctx+msg alg={alg}"),
            c,
            r,
            64,
            false,
            &empty,
            false,
            &empty,
            false,
            alg,
        );
        common::eqi("h2c empty rc", 0, rc);
    }
}

/// h2c-7: 400 fully random (seeded) configurations across both hash ids.
#[test]
fn h2c_random() {
    let (c, r) = both!("_sodium_core_h2c_string_to_hash", H2cFn);
    let mut rng = common::Rng::new(0xF00D_BABE);
    for i in 0..400usize {
        let alg = if rng.next_u64() & 1 == 0 {
            CORE_H2C_SHA256
        } else {
            CORE_H2C_SHA512
        };
        let h_len = rng.below(256); // 0 ..= 255
        let ctx_len = rng.below(600);
        let msg_len = rng.below(400);
        let ctx = rng.bytes(ctx_len);
        let msg = rng.bytes(msg_len);
        let tag = format!("h2c rnd #{i} alg={alg} h_len={h_len} ctx_len={ctx_len} msg_len={msg_len}");
        let rc = run_h2c(&tag, c, r, h_len, false, &ctx, false, &msg, false, alg);
        common::eqi(&format!("{tag} rc"), 0, rc);
    }
}

// ------------------------------------------------- *_from_string comparison --

/// Output-buffer size used for every `*_from_string`: the real output is
/// 32 bytes, the tail is canary.
const STR_BUF: usize = 96;

#[allow(clippy::too_many_arguments)]
fn run_str(
    tag: &str,
    cf: StrFn,
    rf: StrFn,
    ctx: &[u8],
    ctx_null: bool,
    msg: &[u8],
    msg_null: bool,
    alg: c_int,
    expect_rc: Option<c_int>,
) {
    let mut cb = vec![CANARY; STR_BUF];
    let mut rb = vec![CANARY; STR_BUF];
    let cptr: *const u8 = if ctx_null {
        core::ptr::null()
    } else {
        ctx.as_ptr()
    };
    let mptr: *const u8 = if msg_null {
        core::ptr::null()
    } else {
        msg.as_ptr()
    };
    let rc = unsafe { cf(cb.as_mut_ptr(), cptr, ctx.len(), mptr, msg.len(), alg) };
    let rr = unsafe { rf(rb.as_mut_ptr(), cptr, ctx.len(), mptr, msg.len(), alg) };
    common::eqi(tag, rc, rr);
    common::eqb(tag, &cb, &rb);
    if let Some(e) = expect_rc {
        assert_eq!(rc, e, "{tag}: unexpected rc");
    }
    if rc == 0 {
        common::eqb(
            &format!("{tag} canary tail"),
            &vec![CANARY; STR_BUF - 32],
            &cb[32..],
        );
        assert!(
            cb[..32].iter().any(|&b| b != CANARY),
            "{tag}: output still all-canary"
        );
    } else {
        common::eqb(&format!("{tag} untouched"), &vec![CANARY; STR_BUF], &cb);
    }
}

/// Shared configuration grid for all four `*_from_string` entry points.
fn str_grid(name: &str, cf: StrFn, rf: StrFn, seed: u64) {
    let mut rng = common::Rng::new(seed);

    // ctx_len grid, incl. the oversize-DST branch (>0xff).
    for alg in [CORE_H2C_SHA256, CORE_H2C_SHA512] {
        for ctx_len in [0usize, 1, 16, 63, 64, 65, 254, 255, 256, 257, 512, 1000] {
            let ctx = rng.bytes(ctx_len);
            for msg_len in [0usize, 1, 63, 64, 65, 127, 128, 129, 1000] {
                let msg = rng.bytes(msg_len);
                run_str(
                    &format!("{name} alg={alg} ctx_len={ctx_len} msg_len={msg_len}"),
                    cf,
                    rf,
                    &ctx,
                    false,
                    &msg,
                    false,
                    alg,
                    Some(0),
                );
            }
        }
        // NULL ctx / NULL msg with zero length.
        let empty: [u8; 0] = [];
        run_str(
            &format!("{name} alg={alg} ctx=NULL"),
            cf,
            rf,
            &empty,
            true,
            b"msg",
            false,
            alg,
            Some(0),
        );
        run_str(
            &format!("{name} alg={alg} msg=NULL"),
            cf,
            rf,
            b"ctx",
            false,
            &empty,
            true,
            alg,
            Some(0),
        );
        run_str(
            &format!("{name} alg={alg} both NULL"),
            cf,
            rf,
            &empty,
            true,
            &empty,
            true,
            alg,
            Some(0),
        );
    }

    // 60 random cases.
    for i in 0..60usize {
        let alg = if rng.next_u64() & 1 == 0 {
            CORE_H2C_SHA256
        } else {
            CORE_H2C_SHA512
        };
        let cl = rng.below(400);
        let ml = rng.below(300);
        let ctx = rng.bytes(cl);
        let msg = rng.bytes(ml);
        run_str(
            &format!("{name} rnd #{i} alg={alg} ctx_len={} msg_len={}", ctx.len(), msg.len()),
            cf,
            rf,
            &ctx,
            false,
            &msg,
            false,
            alg,
            Some(0),
        );
    }

    // Out-of-range hash ids: core_h2c returns -1 and the wrapper propagates it.
    for alg in [0, 3, -1, 99, c_int::MIN, c_int::MAX] {
        poison_errno();
        run_str(
            &format!("{name} bad alg={alg}"),
            cf,
            rf,
            b"ctx",
            false,
            b"msg",
            false,
            alg,
            Some(-1),
        );
    }
}

// ---------------------------------------------------------- ed25519 points ---

/// h2c-8..: `crypto_core_ed25519_from_string` (two points, h_len = 96).
#[test]
fn ed25519_from_string() {
    let (c, r) = both!("crypto_core_ed25519_from_string", StrFn);
    str_grid("ed25519_from_string", c, r, 0x1111_2222);
}

/// h2c-9..: `crypto_core_ed25519_from_string_nu` (one point, h_len = 48).
#[test]
fn ed25519_from_string_nu() {
    let (c, r) = both!("crypto_core_ed25519_from_string_nu", StrFn);
    str_grid("ed25519_from_string_nu", c, r, 0x3333_4444);
}

/// h2c-10..: `crypto_core_ed25519_scalar_from_string` (h_len = 48, reduced).
#[test]
fn ed25519_scalar_from_string() {
    let (c, r) = both!("crypto_core_ed25519_scalar_from_string", StrFn);
    str_grid("ed25519_scalar_from_string", c, r, 0x5555_6666);
}

/// h2c-11..: `crypto_core_ristretto255_from_string` (h_len = 64).
#[test]
fn ristretto255_from_string() {
    let (c, r) = both!("crypto_core_ristretto255_from_string", StrFn);
    str_grid("ristretto255_from_string", c, r, 0x7777_8888);
}

/// h2c-12..: `crypto_core_ristretto255_scalar_from_string` (delegates to the
/// ed25519 scalar variant).
#[test]
fn ristretto255_scalar_from_string() {
    let (c, r) = both!("crypto_core_ristretto255_scalar_from_string", StrFn);
    str_grid("ristretto255_scalar_from_string", c, r, 0x9999_AAAA);
}

/// `crypto_core_ristretto255_scalar_from_string` must be bit-identical to
/// `crypto_core_ed25519_scalar_from_string` in BOTH libraries (the C is a
/// straight delegation).
#[test]
fn ristretto255_scalar_from_string_is_ed25519() {
    let (ce, re) = both!("crypto_core_ed25519_scalar_from_string", StrFn);
    let (cr, rr) = both!("crypto_core_ristretto255_scalar_from_string", StrFn);
    let mut rng = common::Rng::new(0xBBBB_CCCC);
    for i in 0..20usize {
        let cl = rng.below(40);
        let ctx = rng.bytes(cl);
        let msg = rng.bytes(17);
        let alg = if i % 2 == 0 {
            CORE_H2C_SHA256
        } else {
            CORE_H2C_SHA512
        };
        let mut a = [CANARY; 32];
        let mut b = [CANARY; 32];
        let mut x = [CANARY; 32];
        let mut y = [CANARY; 32];
        unsafe {
            ce(a.as_mut_ptr(), ctx.as_ptr(), ctx.len(), msg.as_ptr(), msg.len(), alg);
            cr(b.as_mut_ptr(), ctx.as_ptr(), ctx.len(), msg.as_ptr(), msg.len(), alg);
            re(x.as_mut_ptr(), ctx.as_ptr(), ctx.len(), msg.as_ptr(), msg.len(), alg);
            rr(y.as_mut_ptr(), ctx.as_ptr(), ctx.len(), msg.as_ptr(), msg.len(), alg);
        }
        common::eqb(&format!("C ed25519 vs ristretto scalar_from_string #{i}"), &a, &b);
        common::eqb(&format!("R ed25519 vs ristretto scalar_from_string #{i}"), &x, &y);
        common::eqb(&format!("C vs R ed25519 scalar_from_string #{i}"), &a, &x);
    }
}

// ------------------------------------------------ raw from_hash/from_uniform -

/// Interesting 32-byte field-element patterns (0, 1, p-1, p, p+1, 2^255-1,
/// high-bit and 3-bit-top variants that exercise the `optblocker`/`>>5>>2`
/// sign extraction).
fn edge_32() -> Vec<[u8; 32]> {
    let mut v: Vec<[u8; 32]> = Vec::new();
    let mut z = [0u8; 32];
    v.push(z); // 0
    z[0] = 1;
    v.push(z); // 1
    v.push([0xff; 32]); // 2^256-1
    let mut p = [0xffu8; 32];
    p[0] = 0xec;
    p[31] = 0x7f;
    v.push(p); // p-1
    p[0] = 0xed;
    v.push(p); // p
    p[0] = 0xee;
    v.push(p); // p+1
    let mut m = [0u8; 32];
    m[31] = 0x80;
    v.push(m); // only the ignored high bit set
    let mut m2 = [0u8; 32];
    m2[31] = 0x20;
    v.push(m2); // bit 5 -> x_sign source
    let mut m3 = [0u8; 32];
    m3[31] = 0xe0;
    v.push(m3); // top three bits
    let mut m4 = [0x55u8; 32];
    m4[31] = 0x7f;
    v.push(m4);
    v
}

/// h2c-13: `_sodium_ge25519_from_uniform` (32-byte input, 32-byte output,
/// input is first `memcpy`'d into the output buffer, so aliasing is legal).
#[test]
fn ge25519_from_uniform() {
    let (c, r) = both!("_sodium_ge25519_from_uniform", HashVFn);
    let mut rng = common::Rng::new(0xDDDD_EEEE);

    let mut inputs = edge_32();
    for _ in 0..60 {
        let mut b = [0u8; 32];
        rng.fill(&mut b);
        inputs.push(b);
    }
    for (i, inp) in inputs.iter().enumerate() {
        let mut cb = [CANARY; 64];
        let mut rb = [CANARY; 64];
        unsafe {
            c(cb.as_mut_ptr(), inp.as_ptr());
            r(rb.as_mut_ptr(), inp.as_ptr());
        }
        let tag = format!("ge25519_from_uniform #{i} r={}", common::hex(inp));
        common::eqb(&tag, &cb, &rb);
        common::eqb(&format!("{tag} canary tail"), &[CANARY; 32], &cb[32..]);

        // s == r (in-place): the C memcpy(s, r, 32) makes this well-defined.
        let mut ca = *inp;
        let mut ra = *inp;
        unsafe {
            c(ca.as_mut_ptr(), ca.as_ptr());
            r(ra.as_mut_ptr(), ra.as_ptr());
        }
        common::eqb(&format!("{tag} aliased"), &ca, &ra);
        common::eqb(&format!("{tag} aliased == non-aliased"), &ca, &cb[..32]);
    }
}

/// Interesting 64-byte inputs for the `*_from_hash` functions.
fn edge_64() -> Vec<[u8; 64]> {
    let mut v: Vec<[u8; 64]> = Vec::new();
    v.push([0u8; 64]);
    v.push([0xffu8; 64]);
    let mut a = [0u8; 64];
    a[0] = 1;
    v.push(a);
    let mut b = [0u8; 64];
    b[32] = 1;
    v.push(b);
    // p, p-1, p+1 in both halves
    for lo in [0xecu8, 0xed, 0xee] {
        let mut h = [0xffu8; 64];
        h[0] = lo;
        h[31] = 0x7f;
        h[32] = lo;
        h[63] = 0x7f;
        v.push(h);
    }
    // exercise the >>5>>2 bit of h[31] / h[63]
    for top in [0x20u8, 0x40, 0x80, 0xe0, 0xff] {
        let mut h = [0u8; 64];
        h[31] = top;
        h[63] = top;
        v.push(h);
    }
    v
}

/// h2c-14: `_sodium_ge25519_from_hash` (64-byte input reduced by
/// `fe25519_reduce64`, 32-byte output).
#[test]
fn ge25519_from_hash() {
    let (c, r) = both!("_sodium_ge25519_from_hash", HashVFn);
    let mut rng = common::Rng::new(0x1234_5678);
    let mut inputs = edge_64();
    for _ in 0..60 {
        let mut b = [0u8; 64];
        rng.fill(&mut b);
        inputs.push(b);
    }
    for (i, inp) in inputs.iter().enumerate() {
        let mut cb = [CANARY; 64];
        let mut rb = [CANARY; 64];
        unsafe {
            c(cb.as_mut_ptr(), inp.as_ptr());
            r(rb.as_mut_ptr(), inp.as_ptr());
        }
        let tag = format!("ge25519_from_hash #{i}");
        common::eqb(&tag, &cb, &rb);
        common::eqb(&format!("{tag} canary tail"), &[CANARY; 32], &cb[32..]);
    }
}

/// h2c-15: `_sodium_ristretto255_from_hash` (64-byte input, 32-byte output).
#[test]
fn ristretto255_from_hash_raw() {
    let (c, r) = both!("_sodium_ristretto255_from_hash", HashVFn);
    let mut rng = common::Rng::new(0x2468_ACE0);
    let mut inputs = edge_64();
    for _ in 0..60 {
        let mut b = [0u8; 64];
        rng.fill(&mut b);
        inputs.push(b);
    }
    for (i, inp) in inputs.iter().enumerate() {
        let mut cb = [CANARY; 64];
        let mut rb = [CANARY; 64];
        unsafe {
            c(cb.as_mut_ptr(), inp.as_ptr());
            r(rb.as_mut_ptr(), inp.as_ptr());
        }
        let tag = format!("ristretto255_from_hash #{i}");
        common::eqb(&tag, &cb, &rb);
        common::eqb(&format!("{tag} canary tail"), &[CANARY; 32], &cb[32..]);
    }
}

/// h2c-16: `crypto_core_ristretto255_from_hash` — the public wrapper; always
/// returns 0, output must equal the raw `ristretto255_from_hash`.
#[test]
fn ristretto255_from_hash_public() {
    let (c, r) = both!("crypto_core_ristretto255_from_hash", HashIFn);
    let (craw, rraw) = both!("_sodium_ristretto255_from_hash", HashVFn);
    let mut rng = common::Rng::new(0x0BAD_F00D);
    let mut inputs = edge_64();
    for _ in 0..60 {
        let mut b = [0u8; 64];
        rng.fill(&mut b);
        inputs.push(b);
    }
    for (i, inp) in inputs.iter().enumerate() {
        let mut cb = [CANARY; 64];
        let mut rb = [CANARY; 64];
        let rc = unsafe { c(cb.as_mut_ptr(), inp.as_ptr()) };
        let rr = unsafe { r(rb.as_mut_ptr(), inp.as_ptr()) };
        let tag = format!("crypto_core_ristretto255_from_hash #{i}");
        common::eqi(&tag, rc, rr);
        assert_eq!(rc, 0, "{tag}: always returns 0");
        common::eqb(&tag, &cb, &rb);
        common::eqb(&format!("{tag} canary tail"), &[CANARY; 32], &cb[32..]);

        let mut cx = [CANARY; 64];
        let mut rx = [CANARY; 64];
        unsafe {
            craw(cx.as_mut_ptr(), inp.as_ptr());
            rraw(rx.as_mut_ptr(), inp.as_ptr());
        }
        common::eqb(&format!("{tag} == raw (C)"), &cb, &cx);
        common::eqb(&format!("{tag} == raw (Rust)"), &rb, &rx);
    }

    // p aliasing the first half of h: both read r0/r1 before writing s.
    for (i, inp) in inputs.iter().take(12).enumerate() {
        let mut cbuf = *inp;
        let mut rbuf = *inp;
        let rc = unsafe { c(cbuf.as_mut_ptr(), cbuf.as_ptr()) };
        let rr = unsafe { r(rbuf.as_mut_ptr(), rbuf.as_ptr()) };
        common::eqi(&format!("ristretto255_from_hash aliased #{i}"), rc, rr);
        common::eqb(&format!("ristretto255_from_hash aliased #{i}"), &cbuf, &rbuf);
    }
}

/// h2c-17: `crypto_core_ristretto255_from_string` must equal
/// `ristretto255_from_hash(core_h2c_string_to_hash(64))` — cross-checks that
/// the two translated layers are wired together the same way in both libs.
#[test]
fn ristretto255_from_string_matches_composition() {
    let (ch2c, rh2c) = both!("_sodium_core_h2c_string_to_hash", H2cFn);
    let (craw, rraw) = both!("_sodium_ristretto255_from_hash", HashVFn);
    let (cfs, rfs) = both!("crypto_core_ristretto255_from_string", StrFn);
    let mut rng = common::Rng::new(0x7E57_7E57);
    for i in 0..24usize {
        let alg = if i % 2 == 0 {
            CORE_H2C_SHA256
        } else {
            CORE_H2C_SHA512
        };
        let ctx = rng.bytes(if i % 3 == 0 { 300 } else { 12 });
        let msg = rng.bytes(7 * i);
        let tag = format!("ristretto255_from_string composition #{i}");

        let mut ch = [0u8; 64];
        let mut rh = [0u8; 64];
        unsafe {
            assert_eq!(
                ch2c(ch.as_mut_ptr(), 64, ctx.as_ptr(), ctx.len(), msg.as_ptr(), msg.len(), alg),
                0
            );
            assert_eq!(
                rh2c(rh.as_mut_ptr(), 64, ctx.as_ptr(), ctx.len(), msg.as_ptr(), msg.len(), alg),
                0
            );
        }
        common::eqb(&format!("{tag} h"), &ch, &rh);

        let mut cp = [CANARY; 32];
        let mut rp = [CANARY; 32];
        unsafe {
            craw(cp.as_mut_ptr(), ch.as_ptr());
            rraw(rp.as_mut_ptr(), rh.as_ptr());
        }

        let mut cs = [CANARY; 32];
        let mut rs = [CANARY; 32];
        unsafe {
            assert_eq!(
                cfs(cs.as_mut_ptr(), ctx.as_ptr(), ctx.len(), msg.as_ptr(), msg.len(), alg),
                0
            );
            assert_eq!(
                rfs(rs.as_mut_ptr(), ctx.as_ptr(), ctx.len(), msg.as_ptr(), msg.len(), alg),
                0
            );
        }
        common::eqb(&format!("{tag} C"), &cp, &cs);
        common::eqb(&format!("{tag} Rust"), &rp, &rs);
        common::eqb(&format!("{tag} C vs Rust"), &cs, &rs);
    }
}

/// h2c-18: `crypto_core_ed25519_from_string` must equal
/// `add(from_hash(be(h[0..48])), from_hash(be(h[48..96])))` — cross-checks the
/// big-endian reversal in `_string_to_points` (48-byte slice reversed into a
/// 64-byte zero-padded buffer) plus `crypto_core_ed25519_add`.
#[test]
fn ed25519_from_string_matches_composition() {
    let (ch2c, rh2c) = both!("_sodium_core_h2c_string_to_hash", H2cFn);
    let (cfh, rfh) = both!("_sodium_ge25519_from_hash", HashVFn);
    let (cadd, _radd) = both!(
        "crypto_core_ed25519_add",
        unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> c_int
    );
    let (cfs, rfs) = both!("crypto_core_ed25519_from_string", StrFn);
    let (cnu, rnu) = both!("crypto_core_ed25519_from_string_nu", StrFn);
    let mut rng = common::Rng::new(0xABCD_1234);

    for i in 0..24usize {
        let alg = if i % 2 == 0 {
            CORE_H2C_SHA256
        } else {
            CORE_H2C_SHA512
        };
        let ctx = rng.bytes(if i % 3 == 0 { 400 } else { 9 });
        let msg = rng.bytes(5 * i);
        let tag = format!("ed25519_from_string composition #{i}");

        let mut h_be = [0u8; 96];
        unsafe {
            assert_eq!(
                ch2c(h_be.as_mut_ptr(), 96, ctx.as_ptr(), ctx.len(), msg.as_ptr(), msg.len(), alg),
                0
            );
        }
        let mut px = [0u8; 64];
        for k in 0..2usize {
            let mut h = [0u8; 64];
            for j in 0..48usize {
                h[j] = h_be[k * 48 + 48 - 1 - j];
            }
            unsafe { cfh(px.as_mut_ptr().add(k * 32), h.as_ptr()) };
        }
        let mut expect = [CANARY; 32];
        let add_rc =
            unsafe { cadd(expect.as_mut_ptr(), px.as_ptr(), px.as_ptr().add(32)) };
        assert_eq!(add_rc, 0, "{tag}: add of two h2c points should succeed");

        let mut cs = [CANARY; 32];
        let mut rs = [CANARY; 32];
        unsafe {
            assert_eq!(
                cfs(cs.as_mut_ptr(), ctx.as_ptr(), ctx.len(), msg.as_ptr(), msg.len(), alg),
                0
            );
            assert_eq!(
                rfs(rs.as_mut_ptr(), ctx.as_ptr(), ctx.len(), msg.as_ptr(), msg.len(), alg),
                0
            );
        }
        common::eqb(&format!("{tag} C == composed"), &cs, &expect);
        common::eqb(&format!("{tag} C vs Rust"), &cs, &rs);

        // _nu uses only the first 48-byte chunk of a 48-byte (n=1) hash.
        let mut h48 = [0u8; 48];
        unsafe {
            assert_eq!(
                rh2c(h48.as_mut_ptr(), 48, ctx.as_ptr(), ctx.len(), msg.as_ptr(), msg.len(), alg),
                0
            );
        }
        let mut h = [0u8; 64];
        for j in 0..48usize {
            h[j] = h48[48 - 1 - j];
        }
        let mut expect_nu_c = [CANARY; 32];
        let mut expect_nu_r = [CANARY; 32];
        unsafe {
            cfh(expect_nu_c.as_mut_ptr(), h.as_ptr());
            rfh(expect_nu_r.as_mut_ptr(), h.as_ptr());
        }
        let mut cnu_out = [CANARY; 32];
        let mut rnu_out = [CANARY; 32];
        unsafe {
            assert_eq!(
                cnu(cnu_out.as_mut_ptr(), ctx.as_ptr(), ctx.len(), msg.as_ptr(), msg.len(), alg),
                0
            );
            assert_eq!(
                rnu(rnu_out.as_mut_ptr(), ctx.as_ptr(), ctx.len(), msg.as_ptr(), msg.len(), alg),
                0
            );
        }
        common::eqb(&format!("{tag} nu C == composed"), &cnu_out, &expect_nu_c);
        common::eqb(&format!("{tag} nu Rust == composed"), &rnu_out, &expect_nu_r);
        common::eqb(&format!("{tag} nu C vs Rust"), &cnu_out, &rnu_out);
    }
}
