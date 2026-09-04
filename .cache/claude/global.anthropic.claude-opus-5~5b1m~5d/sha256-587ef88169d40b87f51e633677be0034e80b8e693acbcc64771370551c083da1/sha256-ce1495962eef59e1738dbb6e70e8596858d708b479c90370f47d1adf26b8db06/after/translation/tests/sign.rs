//! Differential tests for AREA `sign`:
//!
//!   * `crypto_scalarmult/crypto_scalarmult.c`
//!   * `crypto_scalarmult/curve25519/scalarmult_curve25519.c`
//!   * `crypto_scalarmult/curve25519/ref10/x25519_ref10.c`
//!   * `crypto_scalarmult/ed25519/ref10/scalarmult_ed25519_ref10.c`
//!   * `crypto_scalarmult/ristretto255/ref10/scalarmult_ristretto255_ref10.c`
//!   * `crypto_sign/crypto_sign.c`
//!   * `crypto_sign/ed25519/sign_ed25519.c`
//!   * `crypto_sign/ed25519/ref10/{keypair,sign,open}.c`
//!
//! Every call goes through `dlopen`/`dlsym` on the C and the Rust `.so`.
//!
//! NOTE: `cargo test --test sign --target-dir DIR` does *not* build the cdylib
//! into `DIR/release/`, so `tests/common/mod.rs` silently falls back to
//! `translation/target/release/liblibsodium.so`.  Run
//! `cargo build --release --offline --target-dir DIR` first if you want the
//! tests to exercise a freshly built Rust library from `DIR`.

#![allow(non_camel_case_types)]

#[macro_use]
mod common;

use core::ffi::{c_char, c_int, c_void};

// ---------------------------------------------------------------------------
// function types
// ---------------------------------------------------------------------------

type FnSize = unsafe extern "C" fn() -> usize;
type FnStr = unsafe extern "C" fn() -> *const c_char;
type Fn2 = unsafe extern "C" fn(*mut u8, *const u8) -> c_int;
type Fn3 = unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> c_int;
type FnSign = unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> c_int;
type FnSignPh = unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8, c_int) -> c_int;
type FnOpen = unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> c_int;
type FnVerify = unsafe extern "C" fn(*const u8, *const u8, u64, *const u8) -> c_int;
type FnVerifyPh = unsafe extern "C" fn(*const u8, *const u8, u64, *const u8, c_int) -> c_int;
type FnStInit = unsafe extern "C" fn(*mut u8) -> c_int;
type FnStUpdate = unsafe extern "C" fn(*mut u8, *const u8, u64) -> c_int;
type FnStCreate = unsafe extern "C" fn(*mut u8, *mut u8, *mut u64, *const u8) -> c_int;
type FnStVerify = unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> c_int;
type FnHinit = unsafe extern "C" fn(*mut u8, c_int);

/// `struct crypto_scalarmult_curve25519_implementation` (scalarmult_curve25519.h)
#[repr(C)]
struct Curve25519Impl {
    mult: unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> c_int,
    mult_base: unsafe extern "C" fn(*mut u8, *const u8) -> c_int,
}

// ---------------------------------------------------------------------------
// constants
// ---------------------------------------------------------------------------

/// L = 2^252 + 27742317777372353535851937790883648493 (little endian)
const L: [u8; 32] = [
    0xed, 0xd3, 0xf5, 0x5c, 0x1a, 0x63, 0x12, 0x58, 0xd6, 0x9c, 0xf7, 0xa2, 0xde, 0xf9, 0xde, 0x14,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10,
];

/// The 7 X25519 small-order / low-order encodings from `x25519_ref10.c`.
const X25519_BLOCKLIST: [[u8; 32]; 7] = [
    [0x00; 32],
    [
        0x01, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0,
    ],
    [
        0xe0, 0xeb, 0x7a, 0x7c, 0x3b, 0x41, 0xb8, 0xae, 0x16, 0x56, 0xe3, 0xfa, 0xf1, 0x9f, 0xc4,
        0x6a, 0xda, 0x09, 0x8d, 0xeb, 0x9c, 0x32, 0xb1, 0xfd, 0x86, 0x62, 0x05, 0x16, 0x5f, 0x49,
        0xb8, 0x00,
    ],
    [
        0x5f, 0x9c, 0x95, 0xbc, 0xa3, 0x50, 0x8c, 0x24, 0xb1, 0xd0, 0xb1, 0x55, 0x9c, 0x83, 0xef,
        0x5b, 0x04, 0x44, 0x5c, 0xc4, 0x58, 0x1c, 0x8e, 0x86, 0xd8, 0x22, 0x4e, 0xdd, 0xd0, 0x9f,
        0x11, 0x57,
    ],
    [
        0xec, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0x7f,
    ],
    [
        0xed, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0x7f,
    ],
    [
        0xee, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0x7f,
    ],
];

/// The 8 canonical + non-canonical small-order Ed25519 point encodings.
const ED_SMALL_ORDER_HEX: [&str; 12] = [
    // identity (order 1)
    "0100000000000000000000000000000000000000000000000000000000000000",
    "0100000000000000000000000000000000000000000000000000000000000080",
    // order 2
    "ecffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f",
    "ecffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
    // order 4
    "0000000000000000000000000000000000000000000000000000000000000000",
    "0000000000000000000000000000000000000000000000000000000000000080",
    // order 8
    "26e8958fc2b227b045c3f489f2ef98f0d5dfac05d3c63339b13802886d53fc05",
    "26e8958fc2b227b045c3f489f2ef98f0d5dfac05d3c63339b13802886d53fc85",
    "c7176a703d4dd84fba3c0b760d10670f2a2053fa2c39ccc64ec7fd7792ac037a",
    "c7176a703d4dd84fba3c0b760d10670f2a2053fa2c39ccc64ec7fd7792ac03fa",
    // non-canonical encodings of the above (y = p, y = p+1)
    "edffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f",
    "eeffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f",
];

fn unhex(s: &str) -> [u8; 32] {
    let mut out = [0u8; 32];
    let b = s.as_bytes();
    assert_eq!(b.len(), 64, "hex len");
    for i in 0..32 {
        out[i] = u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap();
    }
    out
}

/// little-endian 32-byte * small scalar, no reduction (must not overflow)
fn mul_small(a: &[u8; 32], k: u32) -> [u8; 32] {
    let mut out = [0u8; 32];
    let mut carry: u64 = 0;
    for i in 0..32 {
        let v = a[i] as u64 * k as u64 + carry;
        out[i] = (v & 0xff) as u8;
        carry = v >> 8;
    }
    assert_eq!(carry, 0, "mul_small overflow");
    out
}

// ---------------------------------------------------------------------------
// classification helpers (taken from the C library == ground truth) so that we
// can PROVE each rejection branch was actually exercised.
// ---------------------------------------------------------------------------

struct Internals {
    ge_is_canonical: unsafe extern "C" fn(*const u8) -> c_int,
    ge_frombytes: unsafe extern "C" fn(*mut u8, *const u8) -> c_int,
    ge_frombytes_neg: unsafe extern "C" fn(*mut u8, *const u8) -> c_int,
    ge_has_small_order: unsafe extern "C" fn(*const u8) -> c_int,
    ge_on_main_subgroup: unsafe extern "C" fn(*const u8) -> c_int,
    sc_is_canonical: unsafe extern "C" fn(*const u8) -> c_int,
    r255_frombytes: unsafe extern "C" fn(*mut u8, *const u8) -> c_int,
}

fn internals() -> Internals {
    let l = common::libs();
    Internals {
        ge_is_canonical: getsym!(
            l.c,
            "_sodium_ge25519_is_canonical",
            unsafe extern "C" fn(*const u8) -> c_int
        ),
        ge_frombytes: getsym!(
            l.c,
            "_sodium_ge25519_frombytes",
            unsafe extern "C" fn(*mut u8, *const u8) -> c_int
        ),
        ge_frombytes_neg: getsym!(
            l.c,
            "_sodium_ge25519_frombytes_negate_vartime",
            unsafe extern "C" fn(*mut u8, *const u8) -> c_int
        ),
        ge_has_small_order: getsym!(
            l.c,
            "_sodium_ge25519_has_small_order",
            unsafe extern "C" fn(*const u8) -> c_int
        ),
        ge_on_main_subgroup: getsym!(
            l.c,
            "_sodium_ge25519_is_on_main_subgroup",
            unsafe extern "C" fn(*const u8) -> c_int
        ),
        sc_is_canonical: getsym!(
            l.c,
            "_sodium_sc25519_is_canonical",
            unsafe extern "C" fn(*const u8) -> c_int
        ),
        r255_frombytes: getsym!(
            l.c,
            "_sodium_ristretto255_frombytes",
            unsafe extern "C" fn(*mut u8, *const u8) -> c_int
        ),
    }
}

/// Which rejection reason would `_crypto_scalarmult_ed25519()` hit for point `p`?
/// 0 = none, 1 = non-canonical, 2 = not on curve, 3 = small order, 4 = not on
/// the main subgroup.
fn ed_point_class(it: &Internals, p: &[u8; 32]) -> u32 {
    let mut ge = [0u8; 160]; // sizeof(ge25519_p3) = 4 * int32_t[10]
    unsafe {
        if (it.ge_is_canonical)(p.as_ptr()) == 0 {
            return 1;
        }
        if (it.ge_frombytes)(ge.as_mut_ptr(), p.as_ptr()) != 0 {
            return 2;
        }
        if (it.ge_has_small_order)(ge.as_ptr()) != 0 {
            return 3;
        }
        if (it.ge_on_main_subgroup)(ge.as_ptr()) == 0 {
            return 4;
        }
    }
    0
}

/// Same, for `crypto_sign_ed25519_pk_to_curve25519()` (no canonicity check, uses
/// `ge25519_frombytes_negate_vartime`).
/// 0 = accepted, 2 = not on curve, 3 = small order, 4 = not on main subgroup.
fn pk_class(it: &Internals, p: &[u8; 32]) -> u32 {
    let mut ge = [0u8; 160];
    unsafe {
        if (it.ge_frombytes_neg)(ge.as_mut_ptr(), p.as_ptr()) != 0 {
            return 2;
        }
        if (it.ge_has_small_order)(ge.as_ptr()) != 0 {
            return 3;
        }
        if (it.ge_on_main_subgroup)(ge.as_ptr()) == 0 {
            return 4;
        }
    }
    0
}

/// Like `both!` but with a run-time (non-literal) symbol name.
fn both_dyn<T: Copy>(name: &str) -> (T, T) {
    let l = common::libs();
    let mut nz = name.as_bytes().to_vec();
    nz.push(0);
    unsafe {
        let cs: libloading::Symbol<T> = l
            .c
            .get(&nz)
            .unwrap_or_else(|e| panic!("C missing {name}: {e}"));
        let rs: libloading::Symbol<T> = l
            .r
            .get(&nz)
            .unwrap_or_else(|e| panic!("Rust missing {name}: {e}"));
        (*cs, *rs)
    }
}

fn cstr(p: *const c_char) -> String {
    assert!(!p.is_null());
    let mut v = Vec::new();
    unsafe {
        let mut i = 0isize;
        while *p.offset(i) != 0 {
            v.push(*p.offset(i) as u8);
            i += 1;
        }
    }
    String::from_utf8(v).unwrap()
}

/// Run `f` on both libraries with a canary-padded 32-byte output buffer at
/// offset 16 and compare the return code and the FULL buffer.
fn cmp2(ctx: &str, cf: Fn2, rf: Fn2, n: &[u8; 32], outlen: usize) -> (c_int, Vec<u8>) {
    let mut cb = vec![0xA5u8; outlen + 32];
    let mut rb = vec![0xA5u8; outlen + 32];
    let rc = unsafe { cf(cb.as_mut_ptr().add(16), n.as_ptr()) };
    let rr = unsafe { rf(rb.as_mut_ptr().add(16), n.as_ptr()) };
    common::eqi(ctx, rc, rr);
    common::eqb(ctx, &cb, &rb);
    (rc, cb[16..16 + outlen].to_vec())
}

fn cmp3(
    ctx: &str,
    cf: Fn3,
    rf: Fn3,
    n: &[u8; 32],
    p: &[u8; 32],
    outlen: usize,
) -> (c_int, Vec<u8>) {
    let mut cb = vec![0xA5u8; outlen + 32];
    let mut rb = vec![0xA5u8; outlen + 32];
    let rc = unsafe { cf(cb.as_mut_ptr().add(16), n.as_ptr(), p.as_ptr()) };
    let rr = unsafe { rf(rb.as_mut_ptr().add(16), n.as_ptr(), p.as_ptr()) };
    common::eqi(ctx, rc, rr);
    common::eqb(ctx, &cb, &rb);
    (rc, cb[16..16 + outlen].to_vec())
}

// ===========================================================================
// 1. constant getters / primitive names
// ===========================================================================

#[test]
fn getters() {
    let sizes: [(&str, usize); 21] = [
        ("crypto_scalarmult_bytes", 32),
        ("crypto_scalarmult_scalarbytes", 32),
        ("crypto_scalarmult_curve25519_bytes", 32),
        ("crypto_scalarmult_curve25519_scalarbytes", 32),
        ("crypto_scalarmult_ed25519_bytes", 32),
        ("crypto_scalarmult_ed25519_scalarbytes", 32),
        ("crypto_scalarmult_ristretto255_bytes", 32),
        ("crypto_scalarmult_ristretto255_scalarbytes", 32),
        ("crypto_sign_bytes", 64),
        ("crypto_sign_seedbytes", 32),
        ("crypto_sign_publickeybytes", 32),
        ("crypto_sign_secretkeybytes", 64),
        ("crypto_sign_messagebytes_max", usize::MAX - 64),
        ("crypto_sign_statebytes", 208),
        ("crypto_sign_ed25519_bytes", 64),
        ("crypto_sign_ed25519_seedbytes", 32),
        ("crypto_sign_ed25519_publickeybytes", 32),
        ("crypto_sign_ed25519_secretkeybytes", 64),
        ("crypto_sign_ed25519_messagebytes_max", usize::MAX - 64),
        ("crypto_sign_ed25519ph_statebytes", 208),
        ("crypto_sign_ed25519ph_statebytes", 208),
    ];
    for (name, want) in sizes {
        let (c, r) = both_dyn::<FnSize>(name);
        let (cv, rv) = unsafe { (c(), r()) };
        assert_eq!(cv, rv, "{name}: C={cv} Rust={rv}");
        assert_eq!(cv, want, "{name}: C returned {cv}, expected {want}");
    }

    for (name, want) in [
        ("crypto_scalarmult_primitive", "curve25519"),
        ("crypto_sign_primitive", "ed25519"),
    ] {
        let (c, r) = both_dyn::<FnStr>(name);
        let (cv, rv) = unsafe { (cstr(c()), cstr(r())) };
        assert_eq!(cv, rv, "{name}");
        assert_eq!(cv, want, "{name}");
    }

    // statebytes must agree with the value we use for the opaque state buffers
    let (c, _) = both!("crypto_sign_statebytes", FnSize);
    let (c2, _) = both!("crypto_sign_ed25519ph_statebytes", FnSize);
    assert_eq!(unsafe { c() }, unsafe { c2() });
}

// ===========================================================================
// 2. x25519_ref10 implementation struct (exported data object)
// ===========================================================================

#[test]
fn x25519_ref10_implementation_struct() {
    let (ci, ri) = both_data!(
        "crypto_scalarmult_curve25519_ref10_implementation",
        Curve25519Impl
    );
    assert!(!ci.is_null() && !ri.is_null());
    let (cm, rm) = unsafe { ((*ci).mult, (*ri).mult) };
    let (cmb, rmb) = unsafe { ((*ci).mult_base, (*ri).mult_base) };

    let mut rng = common::Rng::new(0x5CA1AB1E);

    // mult_base through the function pointer
    for i in 0..24 {
        let mut n = [0u8; 32];
        rng.fill(&mut n);
        let (_, cq) = cmp2(&format!("impl.mult_base #{i}"), cmb, rmb, &n, 32);
        // must agree with the public wrapper too
        let (pc, pr) = both!("crypto_scalarmult_curve25519_base", Fn2);
        let (_, pq) = cmp2(&format!("wrapper base #{i}"), pc, pr, &n, 32);
        common::eqb("impl vs wrapper base", &cq, &pq);
    }

    // mult through the function pointer, valid + blocklisted points
    for i in 0..24 {
        let mut n = [0u8; 32];
        let mut p = [0u8; 32];
        rng.fill(&mut n);
        rng.fill(&mut p);
        let (rc, _) = cmp3(&format!("impl.mult #{i}"), cm, rm, &n, &p, 32);
        assert_eq!(rc, 0, "random p should be accepted by ref10 mult");
    }
    for (i, p) in X25519_BLOCKLIST.iter().enumerate() {
        let mut n = [0u8; 32];
        rng.fill(&mut n);
        let (rc, _) = cmp3(&format!("impl.mult blocklist #{i}"), cm, rm, &n, p, 32);
        assert_eq!(rc, -1, "blocklist[{i}] must be rejected");
        // high bit set: has_small_order() masks s[31] & 0x7f, so still rejected
        let mut p2 = *p;
        p2[31] |= 0x80;
        let (rc2, _) = cmp3(&format!("impl.mult blocklist-hi #{i}"), cm, rm, &n, &p2, 32);
        assert_eq!(rc2, -1, "blocklist[{i}]|2^255 must be rejected");
    }
}

// ===========================================================================
// 3. crypto_scalarmult_curve25519_base / crypto_scalarmult_base
// ===========================================================================

#[test]
fn curve25519_base() {
    let (cb, rb) = both!("crypto_scalarmult_curve25519_base", Fn2);
    let (cg, rg) = both!("crypto_scalarmult_base", Fn2);
    let mut rng = common::Rng::new(0xBA5E_0001);

    // fixed edge scalars + random ones
    let mut scalars: Vec<[u8; 32]> = vec![
        [0u8; 32],
        [0xffu8; 32],
        [0x01u8; 32],
        L,
        mul_small(&L, 2),
        mul_small(&L, 7),
    ];
    {
        // scalars that exercise clamping: bit 0..2, bit 255, bit 254
        let mut s = [0u8; 32];
        s[0] = 0x07;
        scalars.push(s);
        let mut s = [0u8; 32];
        s[31] = 0x80;
        scalars.push(s);
        let mut s = [0u8; 32];
        s[31] = 0x40;
        s[0] = 0x08;
        scalars.push(s);
        let mut s = [0xffu8; 32];
        s[0] = 0xf8;
        s[31] = 0x7f;
        scalars.push(s);
    }
    for _ in 0..30 {
        let mut s = [0u8; 32];
        rng.fill(&mut s);
        scalars.push(s);
    }

    for (i, n) in scalars.iter().enumerate() {
        let (rc, q1) = cmp2(&format!("curve25519_base #{i}"), cb, rb, n, 32);
        assert_eq!(rc, 0, "curve25519_base always returns 0");
        let (rc2, q2) = cmp2(&format!("scalarmult_base #{i}"), cg, rg, n, 32);
        assert_eq!(rc2, 0);
        common::eqb("dispatcher == curve25519_base", &q1, &q2);

        // clamping: n and clamp(n) must give the same result
        let mut cl = *n;
        cl[0] &= 248;
        cl[31] &= 127;
        cl[31] |= 64;
        let (_, q3) = cmp2(&format!("curve25519_base clamped #{i}"), cb, rb, &cl, 32);
        common::eqb("clamp invariance", &q1, &q3);
    }

    // aliasing: q == n (the C uses `unsigned char *t = q` as scratch)
    for i in 0..8 {
        let mut n = [0u8; 32];
        rng.fill(&mut n);
        let mut cbuf = [0xA5u8; 64];
        let mut rbuf = [0xA5u8; 64];
        cbuf[16..48].copy_from_slice(&n);
        rbuf[16..48].copy_from_slice(&n);
        let rc = unsafe { cb(cbuf.as_mut_ptr().add(16), cbuf.as_ptr().add(16)) };
        let rr = unsafe { rb(rbuf.as_mut_ptr().add(16), rbuf.as_ptr().add(16)) };
        common::eqi(&format!("base alias #{i}"), rc, rr);
        common::eqb(&format!("base alias #{i}"), &cbuf, &rbuf);
    }
}

// ===========================================================================
// 4. crypto_scalarmult_curve25519 / crypto_scalarmult
// ===========================================================================

#[test]
fn curve25519_mult() {
    let (cm, rm) = both!("crypto_scalarmult_curve25519", Fn3);
    let (cg, rg) = both!("crypto_scalarmult", Fn3);
    let (cb, rb) = both!("crypto_scalarmult_curve25519_base", Fn2);
    let mut rng = common::Rng::new(0xBA5E_0002);

    let mut n_ok = 0usize;
    let mut n_rej = 0usize;

    // --- random scalars * random points ------------------------------------
    for i in 0..30 {
        let mut n = [0u8; 32];
        let mut p = [0u8; 32];
        rng.fill(&mut n);
        rng.fill(&mut p);
        let (rc, q1) = cmp3(&format!("curve25519 #{i}"), cm, rm, &n, &p, 32);
        let (rc2, q2) = cmp3(&format!("scalarmult #{i}"), cg, rg, &n, &p, 32);
        assert_eq!(rc, rc2);
        common::eqb("dispatcher == curve25519", &q1, &q2);
        if rc == 0 {
            n_ok += 1;
        } else {
            n_rej += 1;
        }
    }
    assert!(n_ok >= 25, "expected mostly accepted random points");

    // --- random scalars * real public keys (X25519 ECDH agreement) ---------
    for i in 0..16 {
        let mut a = [0u8; 32];
        let mut b = [0u8; 32];
        rng.fill(&mut a);
        rng.fill(&mut b);
        let (_, pa) = cmp2("ecdh pa", cb, rb, &a, 32);
        let (_, pb) = cmp2("ecdh pb", cb, rb, &b, 32);
        let (mut pa32, mut pb32) = ([0u8; 32], [0u8; 32]);
        pa32.copy_from_slice(&pa);
        pb32.copy_from_slice(&pb);
        let (r1, k1) = cmp3(&format!("ecdh a*pb #{i}"), cm, rm, &a, &pb32, 32);
        let (r2, k2) = cmp3(&format!("ecdh b*pa #{i}"), cm, rm, &b, &pa32, 32);
        assert_eq!(r1, 0);
        assert_eq!(r2, 0);
        common::eqb("ECDH agreement", &k1, &k2);
    }

    // --- the 7 blocklisted points and their high-bit variants -> -1 --------
    for (i, p) in X25519_BLOCKLIST.iter().enumerate() {
        for hi in [false, true] {
            let mut pp = *p;
            if hi {
                pp[31] |= 0x80;
            }
            for k in 0..3 {
                let mut n = [0u8; 32];
                rng.fill(&mut n);
                if k == 1 {
                    n = [0u8; 32];
                }
                if k == 2 {
                    n = [0xffu8; 32];
                }
                let (rc, _) = cmp3(
                    &format!("curve25519 blocklist#{i} hi={hi} k={k}"),
                    cm,
                    rm,
                    &n,
                    &pp,
                    32,
                );
                assert_eq!(rc, -1, "blocklist#{i} hi={hi} must return -1");
                n_rej += 1;
            }
        }
    }
    assert!(n_rej >= 42);

    // --- non-canonical point encodings that are NOT blocklisted ------------
    // p+2 .. p+18 reduce to 2..18 (mod p) and are accepted; 2^255-1 reduces to
    // p-1+2^255 -> masked to p-1 == blocklist entry 4 (rejected).
    for k in 2..=18u8 {
        let mut pp = [0xffu8; 32];
        pp[0] = 0xed_u8.wrapping_add(k); // = p + k, valid only for k <= 18
        pp[31] = 0x7f;
        let mut n = [0u8; 32];
        rng.fill(&mut n);
        let (rc, qa) = cmp3(&format!("noncanon p+{k}"), cm, rm, &n, &pp, 32);
        assert_eq!(rc, 0, "p+{k} must be accepted");
        // must equal the canonical encoding of k
        let mut can = [0u8; 32];
        can[0] = k;
        let (rc2, qb) = cmp3(&format!("canon {k}"), cm, rm, &n, &can, 32);
        assert_eq!(rc2, 0);
        common::eqb("non-canonical == canonical", &qa, &qb);
    }
    // high bit set on an arbitrary point is ignored by fe25519_frombytes
    for i in 0..8 {
        let mut p = [0u8; 32];
        let mut n = [0u8; 32];
        rng.fill(&mut p);
        rng.fill(&mut n);
        p[31] &= 0x7f;
        let mut phi = p;
        phi[31] |= 0x80;
        let (r1, q1) = cmp3(&format!("hi-clear #{i}"), cm, rm, &n, &p, 32);
        let (r2, q2) = cmp3(&format!("hi-set #{i}"), cm, rm, &n, &phi, 32);
        assert_eq!(r1, r2);
        common::eqb("bit255 ignored", &q1, &q2);
    }

    // --- clamping of the scalar --------------------------------------------
    for i in 0..12 {
        let mut n = [0u8; 32];
        let mut p = [0u8; 32];
        rng.fill(&mut n);
        rng.fill(&mut p);
        let mut cl = n;
        cl[0] &= 248;
        cl[31] &= 127;
        cl[31] |= 64;
        let (_, q1) = cmp3(&format!("clamp raw #{i}"), cm, rm, &n, &p, 32);
        let (_, q2) = cmp3(&format!("clamp done #{i}"), cm, rm, &cl, &p, 32);
        common::eqb("scalar clamping", &q1, &q2);
    }

    // --- aliasing q == n and q == p ---------------------------------------
    for i in 0..8 {
        let mut n = [0u8; 32];
        let mut p = [0u8; 32];
        rng.fill(&mut n);
        rng.fill(&mut p);
        // q aliases n
        let mut cbuf = [0xA5u8; 64];
        let mut rbuf = [0xA5u8; 64];
        cbuf[16..48].copy_from_slice(&n);
        rbuf[16..48].copy_from_slice(&n);
        let rc = unsafe { cm(cbuf.as_mut_ptr().add(16), cbuf.as_ptr().add(16), p.as_ptr()) };
        let rr = unsafe { rm(rbuf.as_mut_ptr().add(16), rbuf.as_ptr().add(16), p.as_ptr()) };
        common::eqi(&format!("mult alias q=n #{i}"), rc, rr);
        common::eqb(&format!("mult alias q=n #{i}"), &cbuf, &rbuf);
        // q aliases p
        let mut cbuf = [0xA5u8; 64];
        let mut rbuf = [0xA5u8; 64];
        cbuf[16..48].copy_from_slice(&p);
        rbuf[16..48].copy_from_slice(&p);
        let rc = unsafe { cm(cbuf.as_mut_ptr().add(16), n.as_ptr(), cbuf.as_ptr().add(16)) };
        let rr = unsafe { rm(rbuf.as_mut_ptr().add(16), n.as_ptr(), rbuf.as_ptr().add(16)) };
        common::eqi(&format!("mult alias q=p #{i}"), rc, rr);
        common::eqb(&format!("mult alias q=p #{i}"), &cbuf, &rbuf);
    }
}

/// The `-(1 & ((d - 1) >> 8))` all-zero-result check in
/// `crypto_scalarmult_curve25519()`: search for an input where the ref10 `mult`
/// succeeds but the result is all-zero (which would make the wrapper return -1
/// while `mult` returned 0). Verify C and Rust behave identically either way.
#[test]
fn curve25519_zero_result_check() {
    let (ci, ri) = both_data!(
        "crypto_scalarmult_curve25519_ref10_implementation",
        Curve25519Impl
    );
    let (cmi, rmi) = unsafe { ((*ci).mult, (*ri).mult) };
    let (cm, rm) = both!("crypto_scalarmult_curve25519", Fn3);
    let mut rng = common::Rng::new(0xBA5E_0003);

    let mut zero_hits = 0usize;
    // candidate points: random, all blocklist entries +/- small deltas, and the
    // non-canonical range p..p+18
    let mut cands: Vec<[u8; 32]> = Vec::new();
    for p in X25519_BLOCKLIST.iter() {
        cands.push(*p);
        for d in 1..=3u8 {
            let mut q = *p;
            q[0] = q[0].wrapping_add(d);
            cands.push(q);
            let mut q = *p;
            q[31] ^= d;
            cands.push(q);
        }
    }
    for k in 0..=20u8 {
        let mut pp = [0xffu8; 32];
        pp[0] = 0xed_u8.wrapping_add(k);
        pp[31] = 0x7f;
        cands.push(pp);
    }
    for _ in 0..100 {
        let mut p = [0u8; 32];
        rng.fill(&mut p);
        cands.push(p);
    }

    for (i, p) in cands.iter().enumerate() {
        let mut n = [0u8; 32];
        rng.fill(&mut n);
        let mut cq = [0xA5u8; 32];
        let mut rq = [0xA5u8; 32];
        let rc = unsafe { cmi(cq.as_mut_ptr(), n.as_ptr(), p.as_ptr()) };
        let rr = unsafe { rmi(rq.as_mut_ptr(), n.as_ptr(), p.as_ptr()) };
        common::eqi(&format!("mult raw #{i}"), rc, rr);
        common::eqb(&format!("mult raw #{i}"), &cq, &rq);

        let (wc, _) = cmp3(&format!("wrapper #{i}"), cm, rm, &n, p, 32);
        if rc == 0 && cq.iter().all(|&b| b == 0) {
            zero_hits += 1;
            assert_eq!(wc, -1, "all-zero result must give -1");
        } else if rc == 0 {
            assert_eq!(wc, 0);
        } else {
            assert_eq!(wc, -1);
        }
    }
    // Documented finding: with the clamped scalar (multiple of 8, bit 254 set,
    // bit 255 clear) no point that survives has_small_order() can produce an
    // all-zero result, so the `d == 0` branch is unreachable.  We assert the
    // search found none, so the claim stays honest if libsodium ever changes.
    assert_eq!(
        zero_hits, 0,
        "unexpectedly found an all-zero result: update the notes"
    );
}

#[test]
fn curve25519_pick_best_implementation() {
    let (c, r) = both!(
        "_crypto_scalarmult_curve25519_pick_best_implementation",
        unsafe extern "C" fn() -> c_int
    );
    for _ in 0..3 {
        let (cv, rv) = unsafe { (c(), r()) };
        common::eqi("pick_best_implementation", cv, rv);
        assert_eq!(cv, 0);
    }
    // still functional afterwards (the static `implementation` pointer)
    let (cm, rm) = both!("crypto_scalarmult_curve25519", Fn3);
    let mut rng = common::Rng::new(0xBA5E_0004);
    for i in 0..8 {
        let mut n = [0u8; 32];
        let mut p = [0u8; 32];
        rng.fill(&mut n);
        rng.fill(&mut p);
        let (rc, _) = cmp3(&format!("post-pick #{i}"), cm, rm, &n, &p, 32);
        assert_eq!(rc, 0);
    }
    // ... and so is the blocklist rejection through the (re)selected impl
    let mut n = [0u8; 32];
    rng.fill(&mut n);
    let (rc, _) = cmp3("post-pick blocklist", cm, rm, &n, &X25519_BLOCKLIST[0], 32);
    assert_eq!(rc, -1);
}

// ===========================================================================
// 5. crypto_scalarmult_ed25519{,_noclamp,_base,_base_noclamp}
// ===========================================================================

#[test]
fn ed25519_scalarmult_base_variants() {
    let (cb, rb) = both!("crypto_scalarmult_ed25519_base", Fn2);
    let (cbn, rbn) = both!("crypto_scalarmult_ed25519_base_noclamp", Fn2);
    let mut rng = common::Rng::new(0xED25_0001);

    let mut hit_zero = 0usize;
    let mut hit_inf = 0usize;
    let mut hit_ok = 0usize;

    let mut scalars: Vec<[u8; 32]> = vec![[0u8; 32], [0xffu8; 32], [1u8; 32]];
    for k in 1..=7u32 {
        scalars.push(mul_small(&L, k));
    }
    {
        let mut s = [0u8; 32];
        s[0] = 1;
        scalars.push(s);
        let mut s = [0u8; 32];
        s[31] = 0x80;
        scalars.push(s); // -> t[31] &= 127 makes it zero for noclamp
        let mut s = L;
        s[31] |= 0x80;
        scalars.push(s); // L with bit 255 set: masked back to L
    }
    for _ in 0..25 {
        let mut s = [0u8; 32];
        rng.fill(&mut s);
        scalars.push(s);
    }

    for (i, n) in scalars.iter().enumerate() {
        let (rc, _) = cmp2(&format!("ed base #{i}"), cb, rb, n, 32);
        let (rcn, qn) = cmp2(&format!("ed base_noclamp #{i}"), cbn, rbn, n, 32);
        let is_zero = n.iter().all(|&b| b == 0);
        if is_zero {
            assert_eq!(rc, -1, "zero scalar must be rejected (clamped)");
            assert_eq!(rcn, -1, "zero scalar must be rejected (noclamp)");
            hit_zero += 1;
        }
        // noclamp: multiples of L (mod 2^255) give the identity -> is_inf -> -1
        let mut masked = *n;
        masked[31] &= 127;
        let mut is_mult_of_l = false;
        for k in 1..=7u32 {
            if masked == mul_small(&L, k) {
                is_mult_of_l = true;
            }
        }
        if masked.iter().all(|&b| b == 0) && !is_zero {
            // t becomes 0 -> identity -> is_inf, but sodium_is_zero(n) is false
            assert_eq!(rcn, -1);
            hit_inf += 1;
        }
        if is_mult_of_l {
            assert_eq!(rcn, -1, "n = k*L must be rejected by noclamp");
            assert_eq!(&qn[..], &unhex(ED_SMALL_ORDER_HEX[0])[..], "identity bytes");
            hit_inf += 1;
        }
        if rc == 0 {
            hit_ok += 1;
        }
    }
    assert!(hit_zero >= 1 && hit_inf >= 8 && hit_ok >= 20, "coverage");

    // clamp equivalence: base(n) == base_noclamp(clamp(n))
    for i in 0..12 {
        let mut n = [0u8; 32];
        rng.fill(&mut n);
        let (_, q1) = cmp2(&format!("clampeq a #{i}"), cb, rb, &n, 32);
        let mut cl = n;
        cl[0] &= 248;
        cl[31] |= 64;
        cl[31] &= 127;
        let (_, q2) = cmp2(&format!("clampeq b #{i}"), cbn, rbn, &cl, 32);
        common::eqb("clamp equivalence", &q1, &q2);
    }

    // aliasing q == n
    for i in 0..6 {
        let mut n = [0u8; 32];
        rng.fill(&mut n);
        for (f, g, tag) in [(cb, rb, "base"), (cbn, rbn, "base_noclamp")] {
            let mut cbuf = [0xA5u8; 64];
            let mut rbuf = [0xA5u8; 64];
            cbuf[16..48].copy_from_slice(&n);
            rbuf[16..48].copy_from_slice(&n);
            let rc = unsafe { f(cbuf.as_mut_ptr().add(16), cbuf.as_ptr().add(16)) };
            let rr = unsafe { g(rbuf.as_mut_ptr().add(16), rbuf.as_ptr().add(16)) };
            common::eqi(&format!("ed {tag} alias #{i}"), rc, rr);
            common::eqb(&format!("ed {tag} alias #{i}"), &cbuf, &rbuf);
        }
    }
}

#[test]
fn ed25519_scalarmult_point_variants() {
    let it = internals();
    let (cm, rm) = both!("crypto_scalarmult_ed25519", Fn3);
    let (cmn, rmn) = both!("crypto_scalarmult_ed25519_noclamp", Fn3);
    let (cb, _rb) = both!("crypto_scalarmult_ed25519_base", Fn2);
    let mut rng = common::Rng::new(0xED25_0002);

    // valid main-subgroup points, produced by the C library
    let mut valid: Vec<[u8; 32]> = Vec::new();
    while valid.len() < 12 {
        let mut s = [0u8; 32];
        rng.fill(&mut s);
        let mut p = [0u8; 32];
        if unsafe { cb(p.as_mut_ptr(), s.as_ptr()) } == 0 {
            valid.push(p);
        }
    }

    let mut cls = [0usize; 5];
    let mut ok_clamped = 0usize;
    let mut inf_noclamp = 0usize;
    let mut zero_scalar = 0usize;

    // ---- points ----------------------------------------------------------
    let mut points: Vec<[u8; 32]> = valid.clone();
    for h in ED_SMALL_ORDER_HEX {
        points.push(unhex(h));
    }
    // non-canonical y >= p
    for k in 0..=6u8 {
        let mut pp = [0xffu8; 32];
        pp[0] = 0xed_u8.wrapping_add(k);
        pp[31] = 0x7f;
        points.push(pp);
    }
    // random encodings: ~1/2 not on the curve, of the rest ~7/8 off the main
    // subgroup
    for _ in 0..40 {
        let mut p = [0u8; 32];
        rng.fill(&mut p);
        points.push(p);
    }

    for (i, p) in points.iter().enumerate() {
        let k = ed_point_class(&it, p);
        cls[k as usize] += 1;
        for (j, n) in [
            [0u8; 32],
            [0xffu8; 32],
            L,
            mul_small(&L, 3),
            {
                let mut s = [0u8; 32];
                rng.fill(&mut s);
                s
            },
            {
                let mut s = [0u8; 32];
                s[0] = 1;
                s
            },
        ]
        .iter()
        .enumerate()
        {
            let (rc, q) = cmp3(&format!("ed mult p#{i} n#{j}"), cm, rm, n, p, 32);
            let (rcn, qn) = cmp3(&format!("ed mult_noclamp p#{i} n#{j}"), cmn, rmn, n, p, 32);
            if k != 0 {
                assert_eq!(rc, -1, "point class {k} must be rejected (clamp)");
                assert_eq!(rcn, -1, "point class {k} must be rejected (noclamp)");
                // q untouched (canary must still be 0xA5 -- checked in cmp3)
                assert!(q.iter().all(|&b| b == 0xA5), "q must not be written");
                assert!(qn.iter().all(|&b| b == 0xA5), "q must not be written");
                continue;
            }
            if n.iter().all(|&b| b == 0) {
                assert_eq!(rc, -1, "zero scalar rejected");
                assert_eq!(rcn, -1, "zero scalar rejected");
                zero_scalar += 1;
                continue;
            }
            let mut masked = *n;
            masked[31] &= 127;
            let mut mult_of_l = masked.iter().all(|&b| b == 0);
            for kk in 1..=7u32 {
                if masked == mul_small(&L, kk) {
                    mult_of_l = true;
                }
            }
            if mult_of_l {
                assert_eq!(rcn, -1, "n = k*L on a main-subgroup point -> identity");
                assert_eq!(&qn[..], &unhex(ED_SMALL_ORDER_HEX[0])[..]);
                inf_noclamp += 1;
            } else {
                assert_eq!(rcn, 0);
            }
            assert_eq!(rc, 0, "clamped scalar on a valid point succeeds");
            ok_clamped += 1;
        }
    }
    assert!(cls[0] >= 12, "accepted points: {cls:?}");
    assert!(cls[1] >= 1, "non-canonical points: {cls:?}");
    assert!(cls[2] >= 5, "off-curve points: {cls:?}");
    assert!(cls[3] >= 4, "small-order points: {cls:?}");
    assert!(cls[4] >= 5, "off-main-subgroup points: {cls:?}");
    assert!(ok_clamped >= 20 && inf_noclamp >= 12 && zero_scalar >= 12);

    // ---- clamp equivalence + aliasing on valid points ---------------------
    for (i, p) in valid.iter().take(6).enumerate() {
        let mut n = [0u8; 32];
        rng.fill(&mut n);
        let (_, q1) = cmp3(&format!("ed clampeq a #{i}"), cm, rm, &n, p, 32);
        let mut cl = n;
        cl[0] &= 248;
        cl[31] |= 64;
        cl[31] &= 127;
        let (_, q2) = cmp3(&format!("ed clampeq b #{i}"), cmn, rmn, &cl, p, 32);
        common::eqb("ed clamp equivalence", &q1, &q2);

        // q == n aliasing
        let mut cbuf = [0xA5u8; 64];
        let mut rbuf = [0xA5u8; 64];
        cbuf[16..48].copy_from_slice(&n);
        rbuf[16..48].copy_from_slice(&n);
        let rc = unsafe { cm(cbuf.as_mut_ptr().add(16), cbuf.as_ptr().add(16), p.as_ptr()) };
        let rr = unsafe { rm(rbuf.as_mut_ptr().add(16), rbuf.as_ptr().add(16), p.as_ptr()) };
        common::eqi(&format!("ed alias q=n #{i}"), rc, rr);
        common::eqb(&format!("ed alias q=n #{i}"), &cbuf, &rbuf);

        // q == p aliasing
        let mut cbuf = [0xA5u8; 64];
        let mut rbuf = [0xA5u8; 64];
        cbuf[16..48].copy_from_slice(p);
        rbuf[16..48].copy_from_slice(p);
        let rc = unsafe { cm(cbuf.as_mut_ptr().add(16), n.as_ptr(), cbuf.as_ptr().add(16)) };
        let rr = unsafe { rm(rbuf.as_mut_ptr().add(16), n.as_ptr(), rbuf.as_ptr().add(16)) };
        common::eqi(&format!("ed alias q=p #{i}"), rc, rr);
        common::eqb(&format!("ed alias q=p #{i}"), &cbuf, &rbuf);
    }
}

// ===========================================================================
// 6. ristretto255
// ===========================================================================

#[test]
fn ristretto255_scalarmult() {
    let it = internals();
    let (cm, rm) = both!("crypto_scalarmult_ristretto255", Fn3);
    let (cb, rb) = both!("crypto_scalarmult_ristretto255_base", Fn2);
    let mut rng = common::Rng::new(0x8157_0001);

    // ---- base ------------------------------------------------------------
    let mut valid: Vec<[u8; 32]> = Vec::new();
    let mut base_zero = 0usize;
    let mut base_ok = 0usize;
    let mut scalars: Vec<[u8; 32]> = vec![[0u8; 32], [0xffu8; 32]];
    for k in 1..=7u32 {
        scalars.push(mul_small(&L, k));
    }
    {
        let mut s = [0u8; 32];
        s[31] = 0x80;
        scalars.push(s); // masked to 0
        let mut s = [0u8; 32];
        s[0] = 1;
        scalars.push(s);
    }
    for _ in 0..25 {
        let mut s = [0u8; 32];
        rng.fill(&mut s);
        scalars.push(s);
    }
    for (i, n) in scalars.iter().enumerate() {
        let (rc, q) = cmp2(&format!("r255 base #{i}"), cb, rb, n, 32);
        if rc == 0 {
            base_ok += 1;
            let mut p = [0u8; 32];
            p.copy_from_slice(&q);
            if valid.len() < 12 {
                valid.push(p);
            }
        } else {
            assert_eq!(rc, -1);
            assert!(q.iter().all(|&b| b == 0), "zero result on rejection");
            base_zero += 1;
        }
    }
    assert!(base_ok >= 20, "r255 base ok={base_ok}");
    assert!(base_zero >= 9, "r255 base zero={base_zero}");

    // ---- mult ------------------------------------------------------------
    let mut points: Vec<[u8; 32]> = valid.clone();
    points.push([0u8; 32]); // identity encoding: valid, gives identity -> -1
    points.push([0xffu8; 32]);
    for h in ED_SMALL_ORDER_HEX {
        points.push(unhex(h));
    }
    for _ in 0..40 {
        let mut p = [0u8; 32];
        rng.fill(&mut p);
        points.push(p);
    }

    let mut bad_point = 0usize;
    let mut zero_out = 0usize;
    let mut good = 0usize;
    for (i, p) in points.iter().enumerate() {
        let mut ge = [0u8; 160];
        let decodable = unsafe { (it.r255_frombytes)(ge.as_mut_ptr(), p.as_ptr()) } == 0;
        for (j, n) in [
            [0u8; 32],
            [0xffu8; 32],
            L,
            mul_small(&L, 2),
            {
                let mut s = [0u8; 32];
                rng.fill(&mut s);
                s
            },
        ]
        .iter()
        .enumerate()
        {
            let (rc, q) = cmp3(&format!("r255 mult p#{i} n#{j}"), cm, rm, n, p, 32);
            if !decodable {
                assert_eq!(rc, -1, "undecodable ristretto point");
                assert!(q.iter().all(|&b| b == 0xA5), "q must not be written");
                bad_point += 1;
            } else if rc == -1 {
                assert!(q.iter().all(|&b| b == 0), "identity encodes as all-zero");
                zero_out += 1;
            } else {
                good += 1;
            }
        }
    }
    assert!(bad_point >= 20, "r255 undecodable={bad_point}");
    assert!(zero_out >= 12, "r255 zero result={zero_out}");
    assert!(good >= 20, "r255 good={good}");

    // aliasing q == n / q == p
    for (i, p) in valid.iter().take(5).enumerate() {
        let mut n = [0u8; 32];
        rng.fill(&mut n);
        let mut cbuf = [0xA5u8; 64];
        let mut rbuf = [0xA5u8; 64];
        cbuf[16..48].copy_from_slice(&n);
        rbuf[16..48].copy_from_slice(&n);
        let rc = unsafe { cm(cbuf.as_mut_ptr().add(16), cbuf.as_ptr().add(16), p.as_ptr()) };
        let rr = unsafe { rm(rbuf.as_mut_ptr().add(16), rbuf.as_ptr().add(16), p.as_ptr()) };
        common::eqi(&format!("r255 alias q=n #{i}"), rc, rr);
        common::eqb(&format!("r255 alias q=n #{i}"), &cbuf, &rbuf);

        let mut cbuf = [0xA5u8; 64];
        let mut rbuf = [0xA5u8; 64];
        cbuf[16..48].copy_from_slice(p);
        rbuf[16..48].copy_from_slice(p);
        let rc = unsafe { cm(cbuf.as_mut_ptr().add(16), n.as_ptr(), cbuf.as_ptr().add(16)) };
        let rr = unsafe { rm(rbuf.as_mut_ptr().add(16), n.as_ptr(), rbuf.as_ptr().add(16)) };
        common::eqi(&format!("r255 alias q=p #{i}"), rc, rr);
        common::eqb(&format!("r255 alias q=p #{i}"), &cbuf, &rbuf);
    }

    // base aliasing q == n
    for i in 0..5 {
        let mut n = [0u8; 32];
        rng.fill(&mut n);
        let mut cbuf = [0xA5u8; 64];
        let mut rbuf = [0xA5u8; 64];
        cbuf[16..48].copy_from_slice(&n);
        rbuf[16..48].copy_from_slice(&n);
        let rc = unsafe { cb(cbuf.as_mut_ptr().add(16), cbuf.as_ptr().add(16)) };
        let rr = unsafe { rb(rbuf.as_mut_ptr().add(16), rbuf.as_ptr().add(16)) };
        common::eqi(&format!("r255 base alias #{i}"), rc, rr);
        common::eqb(&format!("r255 base alias #{i}"), &cbuf, &rbuf);
    }
}

// ===========================================================================
// 7. keypairs
// ===========================================================================

fn seed_keypair(f: unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> c_int, seed: &[u8; 32]) -> ([u8; 32], [u8; 64], c_int) {
    let mut pk = [0u8; 32];
    let mut sk = [0u8; 64];
    let rc = unsafe { f(pk.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr()) };
    (pk, sk, rc)
}

#[test]
fn sign_keypairs() {
    let (cs, rs) = both!(
        "crypto_sign_ed25519_seed_keypair",
        unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> c_int
    );
    let (cs2, rs2) = both!(
        "crypto_sign_seed_keypair",
        unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> c_int
    );
    let (ck, rk) = both!("crypto_sign_ed25519_keypair", Fn2Keypair);
    let (ck2, rk2) = both!("crypto_sign_keypair", Fn2Keypair);
    let (csp, rsp) = both!("crypto_sign_ed25519_sk_to_pk", Fn2);
    let (css, rss) = both!("crypto_sign_ed25519_sk_to_seed", Fn2);

    let mut rng = common::Rng::new(0x5EED_0001);
    let mut seeds: Vec<[u8; 32]> = vec![[0u8; 32], [0xffu8; 32], L];
    for _ in 0..25 {
        let mut s = [0u8; 32];
        rng.fill(&mut s);
        seeds.push(s);
    }

    for (i, seed) in seeds.iter().enumerate() {
        // canary-padded buffers so we can catch over-writes
        let mut cpk = [0xA5u8; 64];
        let mut rpk = [0xA5u8; 64];
        let mut csk = [0xA5u8; 96];
        let mut rsk = [0xA5u8; 96];
        let rc = unsafe {
            cs(
                cpk.as_mut_ptr().add(16),
                csk.as_mut_ptr().add(16),
                seed.as_ptr(),
            )
        };
        let rr = unsafe {
            rs(
                rpk.as_mut_ptr().add(16),
                rsk.as_mut_ptr().add(16),
                seed.as_ptr(),
            )
        };
        common::eqi(&format!("seed_keypair #{i}"), rc, rr);
        assert_eq!(rc, 0);
        common::eqb(&format!("seed_keypair pk #{i}"), &cpk, &rpk);
        common::eqb(&format!("seed_keypair sk #{i}"), &csk, &rsk);
        assert_eq!(&csk[16..48], &seed[..], "sk[0..32] == seed");
        assert_eq!(&csk[48..80], &cpk[16..48], "sk[32..64] == pk");

        // dispatcher
        let (pk2, sk2, rc2) = seed_keypair(cs2, seed);
        let (pk3, sk3, rc3) = seed_keypair(rs2, seed);
        common::eqi("crypto_sign_seed_keypair", rc2, rc3);
        common::eqb("dispatcher pk", &pk2, &pk3);
        common::eqb("dispatcher sk", &sk2, &sk3);
        assert_eq!(&pk2[..], &cpk[16..48]);

        // sk_to_pk / sk_to_seed (need the full 64-byte sk, so call directly)
        let mut sk64 = [0u8; 64];
        sk64.copy_from_slice(&csk[16..80]);
        let mut a = [0xA5u8; 64];
        let mut b = [0xA5u8; 64];
        let rc = unsafe { csp(a.as_mut_ptr().add(16), sk64.as_ptr()) };
        let rr = unsafe { rsp(b.as_mut_ptr().add(16), sk64.as_ptr()) };
        common::eqi("sk_to_pk", rc, rr);
        assert_eq!(rc, 0);
        common::eqb("sk_to_pk", &a, &b);
        assert_eq!(&a[16..48], &pk2[..]);

        let mut a = [0xA5u8; 64];
        let mut b = [0xA5u8; 64];
        let rc = unsafe { css(a.as_mut_ptr().add(16), sk64.as_ptr()) };
        let rr = unsafe { rss(b.as_mut_ptr().add(16), sk64.as_ptr()) };
        common::eqi("sk_to_seed", rc, rr);
        assert_eq!(rc, 0);
        common::eqb("sk_to_seed", &a, &b);
        assert_eq!(&a[16..48], &seed[..]);
    }

    // keypair(): RNG-driven, compare the return code only, then check the
    // internal consistency of each library's own output.
    for (kf, tag) in [(ck, "c ed25519_keypair"), (rk, "r ed25519_keypair")] {
        let mut pk = [0u8; 32];
        let mut sk = [0u8; 64];
        let rc = unsafe { kf(pk.as_mut_ptr(), sk.as_mut_ptr()) };
        assert_eq!(rc, 0, "{tag}");
        assert_eq!(&sk[32..], &pk[..], "{tag}: sk[32..] == pk");
    }
    let (a, b) = unsafe {
        let mut pk = [0u8; 32];
        let mut sk = [0u8; 64];
        let ra = ck2(pk.as_mut_ptr(), sk.as_mut_ptr());
        let mut pk2 = [0u8; 32];
        let mut sk2 = [0u8; 64];
        let rb = rk2(pk2.as_mut_ptr(), sk2.as_mut_ptr());
        assert_eq!(&sk[32..], &pk[..]);
        assert_eq!(&sk2[32..], &pk2[..]);
        (ra, rb)
    };
    common::eqi("crypto_sign_keypair rc", a, b);
}

type Fn2Keypair = unsafe extern "C" fn(*mut u8, *mut u8) -> c_int;

// ===========================================================================
// 8. sign / open, detached / verify_detached
// ===========================================================================

const MLENS: [usize; 9] = [0, 1, 31, 32, 33, 64, 127, 128, 1000];

#[test]
fn sign_and_open() {
    let (cs, rs) = both!("crypto_sign_ed25519", FnSign);
    let (cs2, rs2) = both!("crypto_sign", FnSign);
    let (co, ro) = both!("crypto_sign_ed25519_open", FnOpen);
    let (co2, ro2) = both!("crypto_sign_open", FnOpen);
    let (ckp, _) = both!(
        "crypto_sign_ed25519_seed_keypair",
        unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> c_int
    );
    let mut rng = common::Rng::new(0x516E_0001);

    for &mlen in MLENS.iter() {
        for trial in 0..2 {
            let mut seed = [0u8; 32];
            rng.fill(&mut seed);
            let (pk, sk, _) = seed_keypair(ckp, &seed);
            let m = rng.bytes(mlen);

            // ---- crypto_sign with siglen_p non-NULL -----------------------
            let tot = mlen + 64;
            let mut cb = vec![0xA5u8; tot + 32];
            let mut rb = vec![0xA5u8; tot + 32];
            let mut csl: u64 = 0xDEAD_BEEF;
            let mut rsl: u64 = 0xDEAD_BEEF;
            let rc = unsafe {
                cs(
                    cb.as_mut_ptr().add(16),
                    &mut csl,
                    m.as_ptr(),
                    mlen as u64,
                    sk.as_ptr(),
                )
            };
            let rr = unsafe {
                rs(
                    rb.as_mut_ptr().add(16),
                    &mut rsl,
                    m.as_ptr(),
                    mlen as u64,
                    sk.as_ptr(),
                )
            };
            common::eqi(&format!("sign mlen={mlen} #{trial}"), rc, rr);
            assert_eq!(rc, 0);
            assert_eq!(csl, rsl);
            assert_eq!(csl, tot as u64);
            common::eqb(&format!("sign mlen={mlen} #{trial}"), &cb, &rb);

            // ---- crypto_sign with siglen_p == NULL ------------------------
            let mut cb2 = vec![0xA5u8; tot + 32];
            let mut rb2 = vec![0xA5u8; tot + 32];
            let rc = unsafe {
                cs(
                    cb2.as_mut_ptr().add(16),
                    core::ptr::null_mut(),
                    m.as_ptr(),
                    mlen as u64,
                    sk.as_ptr(),
                )
            };
            let rr = unsafe {
                rs(
                    rb2.as_mut_ptr().add(16),
                    core::ptr::null_mut(),
                    m.as_ptr(),
                    mlen as u64,
                    sk.as_ptr(),
                )
            };
            common::eqi("sign NULL smlen_p", rc, rr);
            common::eqb("sign NULL smlen_p", &cb2, &rb2);
            common::eqb("sign NULL == non-NULL", &cb, &cb2);

            // ---- crypto_sign dispatcher -----------------------------------
            let mut cb3 = vec![0xA5u8; tot + 32];
            let mut rb3 = vec![0xA5u8; tot + 32];
            let mut a: u64 = 0;
            let mut b: u64 = 0;
            let rc = unsafe {
                cs2(
                    cb3.as_mut_ptr().add(16),
                    &mut a,
                    m.as_ptr(),
                    mlen as u64,
                    sk.as_ptr(),
                )
            };
            let rr = unsafe {
                rs2(
                    rb3.as_mut_ptr().add(16),
                    &mut b,
                    m.as_ptr(),
                    mlen as u64,
                    sk.as_ptr(),
                )
            };
            common::eqi("crypto_sign", rc, rr);
            assert_eq!(a, b);
            common::eqb("crypto_sign", &cb3, &rb3);
            common::eqb("crypto_sign == ed25519", &cb, &cb3);

            // ---- in-place signing: m aliases sm+64 ------------------------
            let mut cb4 = vec![0xA5u8; tot + 32];
            let mut rb4 = vec![0xA5u8; tot + 32];
            cb4[16 + 64..16 + 64 + mlen].copy_from_slice(&m);
            rb4[16 + 64..16 + 64 + mlen].copy_from_slice(&m);
            let mut a: u64 = 0;
            let mut b: u64 = 0;
            let rc = unsafe {
                let p = cb4.as_mut_ptr().add(16);
                cs(p, &mut a, p.add(64), mlen as u64, sk.as_ptr())
            };
            let rr = unsafe {
                let p = rb4.as_mut_ptr().add(16);
                rs(p, &mut b, p.add(64), mlen as u64, sk.as_ptr())
            };
            common::eqi("in-place sign", rc, rr);
            assert_eq!(a, b);
            common::eqb("in-place sign", &cb4, &rb4);
            common::eqb("in-place sign == copy", &cb, &cb4);

            let sm = cb[16..16 + tot].to_vec();

            // ---- open: m non-NULL, mlen_p non-NULL ------------------------
            let mut cm = vec![0xA5u8; mlen + 32];
            let mut rmv = vec![0xA5u8; mlen + 32];
            let mut cl: u64 = 0xDEAD_BEEF;
            let mut rl: u64 = 0xDEAD_BEEF;
            let rc = unsafe {
                co(
                    cm.as_mut_ptr().add(16),
                    &mut cl,
                    sm.as_ptr(),
                    tot as u64,
                    pk.as_ptr(),
                )
            };
            let rr = unsafe {
                ro(
                    rmv.as_mut_ptr().add(16),
                    &mut rl,
                    sm.as_ptr(),
                    tot as u64,
                    pk.as_ptr(),
                )
            };
            common::eqi(&format!("open mlen={mlen}"), rc, rr);
            assert_eq!(rc, 0);
            assert_eq!(cl, rl);
            assert_eq!(cl, mlen as u64);
            common::eqb(&format!("open mlen={mlen}"), &cm, &rmv);
            assert_eq!(&cm[16..16 + mlen], &m[..]);

            // ---- open: m == NULL ------------------------------------------
            let mut cl: u64 = 0;
            let mut rl: u64 = 0;
            let rc = unsafe {
                co(
                    core::ptr::null_mut(),
                    &mut cl,
                    sm.as_ptr(),
                    tot as u64,
                    pk.as_ptr(),
                )
            };
            let rr = unsafe {
                ro(
                    core::ptr::null_mut(),
                    &mut rl,
                    sm.as_ptr(),
                    tot as u64,
                    pk.as_ptr(),
                )
            };
            common::eqi("open m=NULL", rc, rr);
            assert_eq!(cl, rl);
            assert_eq!(cl, mlen as u64);

            // ---- open: mlen_p == NULL -------------------------------------
            let mut cm = vec![0xA5u8; mlen + 32];
            let mut rmv = vec![0xA5u8; mlen + 32];
            let rc = unsafe {
                co(
                    cm.as_mut_ptr().add(16),
                    core::ptr::null_mut(),
                    sm.as_ptr(),
                    tot as u64,
                    pk.as_ptr(),
                )
            };
            let rr = unsafe {
                ro(
                    rmv.as_mut_ptr().add(16),
                    core::ptr::null_mut(),
                    sm.as_ptr(),
                    tot as u64,
                    pk.as_ptr(),
                )
            };
            common::eqi("open mlen_p=NULL", rc, rr);
            common::eqb("open mlen_p=NULL", &cm, &rmv);

            // ---- open: both NULL ------------------------------------------
            let rc = unsafe {
                co(
                    core::ptr::null_mut(),
                    core::ptr::null_mut(),
                    sm.as_ptr(),
                    tot as u64,
                    pk.as_ptr(),
                )
            };
            let rr = unsafe {
                ro(
                    core::ptr::null_mut(),
                    core::ptr::null_mut(),
                    sm.as_ptr(),
                    tot as u64,
                    pk.as_ptr(),
                )
            };
            common::eqi("open both NULL", rc, rr);

            // ---- open in-place (m == sm) ----------------------------------
            let mut cb5 = vec![0xA5u8; tot + 32];
            let mut rb5 = vec![0xA5u8; tot + 32];
            cb5[16..16 + tot].copy_from_slice(&sm);
            rb5[16..16 + tot].copy_from_slice(&sm);
            let mut a: u64 = 0;
            let mut b: u64 = 0;
            let rc = unsafe {
                let p = cb5.as_mut_ptr().add(16);
                co(p, &mut a, p, tot as u64, pk.as_ptr())
            };
            let rr = unsafe {
                let p = rb5.as_mut_ptr().add(16);
                ro(p, &mut b, p, tot as u64, pk.as_ptr())
            };
            common::eqi("open in-place", rc, rr);
            assert_eq!(a, b);
            common::eqb("open in-place", &cb5, &rb5);

            // ---- dispatcher -----------------------------------------------
            let mut cm = vec![0xA5u8; mlen + 32];
            let mut rmv = vec![0xA5u8; mlen + 32];
            let mut a: u64 = 0;
            let mut b: u64 = 0;
            let rc = unsafe {
                co2(
                    cm.as_mut_ptr().add(16),
                    &mut a,
                    sm.as_ptr(),
                    tot as u64,
                    pk.as_ptr(),
                )
            };
            let rr = unsafe {
                ro2(
                    rmv.as_mut_ptr().add(16),
                    &mut b,
                    sm.as_ptr(),
                    tot as u64,
                    pk.as_ptr(),
                )
            };
            common::eqi("crypto_sign_open", rc, rr);
            assert_eq!(a, b);
            common::eqb("crypto_sign_open", &cm, &rmv);

            // ---- tampered signed message: every byte of the 64-byte sig ---
            for bit in 0..64usize {
                let mut bad = sm.clone();
                bad[bit] ^= 0x01;
                let mut cm = vec![0xA5u8; mlen + 32];
                let mut rmv = vec![0xA5u8; mlen + 32];
                let mut a: u64 = 0xDEAD;
                let mut b: u64 = 0xDEAD;
                let rc = unsafe {
                    co(
                        cm.as_mut_ptr().add(16),
                        &mut a,
                        bad.as_ptr(),
                        tot as u64,
                        pk.as_ptr(),
                    )
                };
                let rr = unsafe {
                    ro(
                        rmv.as_mut_ptr().add(16),
                        &mut b,
                        bad.as_ptr(),
                        tot as u64,
                        pk.as_ptr(),
                    )
                };
                common::eqi(&format!("open tampered sig byte {bit}"), rc, rr);
                assert_eq!(rc, -1, "tampered sig byte {bit} must fail");
                assert_eq!(a, 0);
                assert_eq!(b, 0);
                // the C zeroes `m` (mlen bytes) on failure
                common::eqb(&format!("open tampered m byte {bit}"), &cm, &rmv);
                assert!(cm[16..16 + mlen].iter().all(|&x| x == 0));
                assert!(cm[..16].iter().all(|&x| x == 0xA5));
                assert!(cm[16 + mlen..].iter().all(|&x| x == 0xA5));
            }
            // ---- tampered message body ------------------------------------
            if mlen > 0 {
                for off in [0usize, mlen / 2, mlen - 1] {
                    let mut bad = sm.clone();
                    bad[64 + off] ^= 0x80;
                    let mut cm = vec![0xA5u8; mlen + 32];
                    let mut rmv = vec![0xA5u8; mlen + 32];
                    let mut a: u64 = 1;
                    let mut b: u64 = 1;
                    let rc = unsafe {
                        co(
                            cm.as_mut_ptr().add(16),
                            &mut a,
                            bad.as_ptr(),
                            tot as u64,
                            pk.as_ptr(),
                        )
                    };
                    let rr = unsafe {
                        ro(
                            rmv.as_mut_ptr().add(16),
                            &mut b,
                            bad.as_ptr(),
                            tot as u64,
                            pk.as_ptr(),
                        )
                    };
                    common::eqi("open tampered body", rc, rr);
                    assert_eq!(rc, -1);
                    assert_eq!(a, b);
                    common::eqb("open tampered body", &cm, &rmv);
                }
            }

            // ---- wrong pk --------------------------------------------------
            let mut seed2 = [0u8; 32];
            rng.fill(&mut seed2);
            let (pk2, _, _) = seed_keypair(ckp, &seed2);
            let mut cm = vec![0xA5u8; mlen + 32];
            let mut rmv = vec![0xA5u8; mlen + 32];
            let mut a: u64 = 7;
            let mut b: u64 = 7;
            let rc = unsafe {
                co(
                    cm.as_mut_ptr().add(16),
                    &mut a,
                    sm.as_ptr(),
                    tot as u64,
                    pk2.as_ptr(),
                )
            };
            let rr = unsafe {
                ro(
                    rmv.as_mut_ptr().add(16),
                    &mut b,
                    sm.as_ptr(),
                    tot as u64,
                    pk2.as_ptr(),
                )
            };
            common::eqi("open wrong pk", rc, rr);
            assert_eq!(rc, -1);
            assert_eq!(a, 0);
            assert_eq!(b, 0);
            common::eqb("open wrong pk", &cm, &rmv);

            // ---- badsig path with m == NULL / mlen_p == NULL ---------------
            let mut bad = sm.clone();
            bad[7] ^= 0x55;
            let mut a: u64 = 9;
            let mut b: u64 = 9;
            let rc = unsafe {
                co(
                    core::ptr::null_mut(),
                    &mut a,
                    bad.as_ptr(),
                    tot as u64,
                    pk.as_ptr(),
                )
            };
            let rr = unsafe {
                ro(
                    core::ptr::null_mut(),
                    &mut b,
                    bad.as_ptr(),
                    tot as u64,
                    pk.as_ptr(),
                )
            };
            common::eqi("open badsig m=NULL", rc, rr);
            assert_eq!(rc, -1);
            assert_eq!(a, 0);
            assert_eq!(b, 0);
            let mut cm2 = vec![0xA5u8; mlen + 32];
            let mut rm2 = vec![0xA5u8; mlen + 32];
            let rc = unsafe {
                co(
                    cm2.as_mut_ptr().add(16),
                    core::ptr::null_mut(),
                    bad.as_ptr(),
                    tot as u64,
                    pk.as_ptr(),
                )
            };
            let rr = unsafe {
                ro(
                    rm2.as_mut_ptr().add(16),
                    core::ptr::null_mut(),
                    bad.as_ptr(),
                    tot as u64,
                    pk.as_ptr(),
                )
            };
            common::eqi("open badsig mlen_p=NULL", rc, rr);
            assert_eq!(rc, -1);
            common::eqb("open badsig mlen_p=NULL", &cm2, &rm2);
            assert!(cm2[16..16 + mlen].iter().all(|&x| x == 0));
            let rc = unsafe {
                co(
                    core::ptr::null_mut(),
                    core::ptr::null_mut(),
                    bad.as_ptr(),
                    tot as u64,
                    pk.as_ptr(),
                )
            };
            let rr = unsafe {
                ro(
                    core::ptr::null_mut(),
                    core::ptr::null_mut(),
                    bad.as_ptr(),
                    tot as u64,
                    pk.as_ptr(),
                )
            };
            common::eqi("open badsig both NULL", rc, rr);
            assert_eq!(rc, -1);
        }
    }
}

#[test]
fn open_short_smlen() {
    let (co, ro) = both!("crypto_sign_ed25519_open", FnOpen);
    let (co2, ro2) = both!("crypto_sign_open", FnOpen);
    let (ckp, _) = both!(
        "crypto_sign_ed25519_seed_keypair",
        unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> c_int
    );
    let mut rng = common::Rng::new(0x516E_0002);
    let mut seed = [0u8; 32];
    rng.fill(&mut seed);
    let (pk, _sk, _) = seed_keypair(ckp, &seed);
    let sm = rng.bytes(64);

    // smlen < 64 -> -1, *mlen_p = 0, m untouched
    for smlen in 0..64u64 {
        for (f, g, tag) in [
            (co, ro, "ed25519_open"),
            (co2, ro2, "crypto_sign_open"),
        ] {
            let mut cm = [0xA5u8; 64];
            let mut rmv = [0xA5u8; 64];
            let mut a: u64 = 0x1234;
            let mut b: u64 = 0x1234;
            let rc = unsafe { f(cm.as_mut_ptr(), &mut a, sm.as_ptr(), smlen, pk.as_ptr()) };
            let rr = unsafe { g(rmv.as_mut_ptr(), &mut b, sm.as_ptr(), smlen, pk.as_ptr()) };
            common::eqi(&format!("{tag} smlen={smlen}"), rc, rr);
            assert_eq!(rc, -1);
            assert_eq!(a, 0);
            assert_eq!(b, 0);
            common::eqb(&format!("{tag} smlen={smlen}"), &cm, &rmv);
            assert!(cm.iter().all(|&x| x == 0xA5), "m must be untouched");
            // mlen_p == NULL as well
            let rc = unsafe {
                f(
                    core::ptr::null_mut(),
                    core::ptr::null_mut(),
                    sm.as_ptr(),
                    smlen,
                    pk.as_ptr(),
                )
            };
            let rr = unsafe {
                g(
                    core::ptr::null_mut(),
                    core::ptr::null_mut(),
                    sm.as_ptr(),
                    smlen,
                    pk.as_ptr(),
                )
            };
            common::eqi(&format!("{tag} smlen={smlen} NULLs"), rc, rr);
        }
    }
}

#[test]
fn sign_detached_and_verify() {
    let (cd, rd) = both!("crypto_sign_ed25519_detached", FnSign);
    let (cd2, rd2) = both!("crypto_sign_detached", FnSign);
    let (cdp, rdp) = both!("_crypto_sign_ed25519_detached", FnSignPh);
    let (cv, rv) = both!("crypto_sign_ed25519_verify_detached", FnVerify);
    let (cv2, rv2) = both!("crypto_sign_verify_detached", FnVerify);
    let (cvp, rvp) = both!("_crypto_sign_ed25519_verify_detached", FnVerifyPh);
    let (ckp, _) = both!(
        "crypto_sign_ed25519_seed_keypair",
        unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> c_int
    );
    let mut rng = common::Rng::new(0x516E_0003);

    for &mlen in MLENS.iter() {
        let mut seed = [0u8; 32];
        rng.fill(&mut seed);
        let (pk, sk, _) = seed_keypair(ckp, &seed);
        let m = rng.bytes(mlen);

        // detached, siglen_p non-NULL / NULL
        let mut ca = [0xA5u8; 96];
        let mut ra = [0xA5u8; 96];
        let mut cl: u64 = 0;
        let mut rl: u64 = 0;
        let rc = unsafe {
            cd(
                ca.as_mut_ptr().add(16),
                &mut cl,
                m.as_ptr(),
                mlen as u64,
                sk.as_ptr(),
            )
        };
        let rr = unsafe {
            rd(
                ra.as_mut_ptr().add(16),
                &mut rl,
                m.as_ptr(),
                mlen as u64,
                sk.as_ptr(),
            )
        };
        common::eqi(&format!("detached mlen={mlen}"), rc, rr);
        assert_eq!(rc, 0);
        assert_eq!(cl, 64);
        assert_eq!(rl, 64);
        common::eqb(&format!("detached mlen={mlen}"), &ca, &ra);

        let mut cb = [0xA5u8; 96];
        let mut rb = [0xA5u8; 96];
        let rc = unsafe {
            cd(
                cb.as_mut_ptr().add(16),
                core::ptr::null_mut(),
                m.as_ptr(),
                mlen as u64,
                sk.as_ptr(),
            )
        };
        let rr = unsafe {
            rd(
                rb.as_mut_ptr().add(16),
                core::ptr::null_mut(),
                m.as_ptr(),
                mlen as u64,
                sk.as_ptr(),
            )
        };
        common::eqi("detached NULL siglen_p", rc, rr);
        common::eqb("detached NULL siglen_p", &cb, &rb);
        common::eqb("detached NULL == non-NULL", &ca, &cb);

        // dispatcher
        let mut cb = [0xA5u8; 96];
        let mut rb = [0xA5u8; 96];
        let mut a: u64 = 0;
        let mut b: u64 = 0;
        let rc = unsafe {
            cd2(
                cb.as_mut_ptr().add(16),
                &mut a,
                m.as_ptr(),
                mlen as u64,
                sk.as_ptr(),
            )
        };
        let rr = unsafe {
            rd2(
                rb.as_mut_ptr().add(16),
                &mut b,
                m.as_ptr(),
                mlen as u64,
                sk.as_ptr(),
            )
        };
        common::eqi("crypto_sign_detached", rc, rr);
        assert_eq!(a, b);
        common::eqb("crypto_sign_detached", &cb, &rb);
        common::eqb("crypto_sign_detached == ed25519", &ca, &cb);

        // _crypto_sign_ed25519_detached with prehashed = 0, 1, 2, -1
        for ph in [0i32, 1, 2, -1] {
            let mut cb = [0xA5u8; 96];
            let mut rb = [0xA5u8; 96];
            let mut a: u64 = 0;
            let mut b: u64 = 0;
            let rc = unsafe {
                cdp(
                    cb.as_mut_ptr().add(16),
                    &mut a,
                    m.as_ptr(),
                    mlen as u64,
                    sk.as_ptr(),
                    ph,
                )
            };
            let rr = unsafe {
                rdp(
                    rb.as_mut_ptr().add(16),
                    &mut b,
                    m.as_ptr(),
                    mlen as u64,
                    sk.as_ptr(),
                    ph,
                )
            };
            common::eqi(&format!("_detached ph={ph}"), rc, rr);
            assert_eq!(a, b);
            common::eqb(&format!("_detached ph={ph}"), &cb, &rb);
            if ph == 0 {
                common::eqb("_detached ph=0 == public", &ca, &cb);
            }

            // matching verify
            let mut sig = [0u8; 64];
            sig.copy_from_slice(&cb[16..80]);
            let rc = unsafe { cvp(sig.as_ptr(), m.as_ptr(), mlen as u64, pk.as_ptr(), ph) };
            let rr = unsafe { rvp(sig.as_ptr(), m.as_ptr(), mlen as u64, pk.as_ptr(), ph) };
            common::eqi(&format!("_verify ph={ph}"), rc, rr);
            assert_eq!(rc, 0, "prehashed={ph} round trip");
        }

        let mut sig = [0u8; 64];
        sig.copy_from_slice(&ca[16..80]);

        // verify_detached OK
        for (f, g, tag) in [
            (cv, rv, "ed25519_verify_detached"),
            (cv2, rv2, "crypto_sign_verify_detached"),
        ] {
            let rc = unsafe { f(sig.as_ptr(), m.as_ptr(), mlen as u64, pk.as_ptr()) };
            let rr = unsafe { g(sig.as_ptr(), m.as_ptr(), mlen as u64, pk.as_ptr()) };
            common::eqi(&format!("{tag} mlen={mlen}"), rc, rr);
            assert_eq!(rc, 0);
        }

        // flip every byte of the signature (all 8 bits of each byte via xor 0xff
        // would be slower; use a rotating mask that hits every bit position)
        for i in 0..64usize {
            for mask in [0x01u8, 0x40, 0x80] {
                let mut bad = sig;
                bad[i] ^= mask;
                let rc = unsafe { cv(bad.as_ptr(), m.as_ptr(), mlen as u64, pk.as_ptr()) };
                let rr = unsafe { rv(bad.as_ptr(), m.as_ptr(), mlen as u64, pk.as_ptr()) };
                common::eqi(&format!("verify tampered [{i}]^{mask:#x}"), rc, rr);
                assert_eq!(rc, -1, "tampered sig byte {i} mask {mask:#x} must fail");
            }
        }
        // tamper the message
        if mlen > 0 {
            for off in [0usize, mlen / 2, mlen - 1] {
                let mut bad = m.clone();
                bad[off] ^= 0x20;
                let rc = unsafe { cv(sig.as_ptr(), bad.as_ptr(), mlen as u64, pk.as_ptr()) };
                let rr = unsafe { rv(sig.as_ptr(), bad.as_ptr(), mlen as u64, pk.as_ptr()) };
                common::eqi("verify tampered msg", rc, rr);
                assert_eq!(rc, -1);
            }
        }
        // truncated / extended message length
        if mlen > 0 {
            let rc = unsafe { cv(sig.as_ptr(), m.as_ptr(), (mlen - 1) as u64, pk.as_ptr()) };
            let rr = unsafe { rv(sig.as_ptr(), m.as_ptr(), (mlen - 1) as u64, pk.as_ptr()) };
            common::eqi("verify short mlen", rc, rr);
            assert_eq!(rc, -1);
        }
    }
}

/// All the rejection sites of `_crypto_sign_ed25519_verify_detached()`.
#[test]
fn verify_detached_rejections() {
    let it = internals();
    let (cv, rv) = both!("crypto_sign_ed25519_verify_detached", FnVerify);
    let (cd, _) = both!("crypto_sign_ed25519_detached", FnSign);
    let (ckp, _) = both!(
        "crypto_sign_ed25519_seed_keypair",
        unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> c_int
    );
    let mut rng = common::Rng::new(0x516E_0004);

    let mut seed = [0u8; 32];
    rng.fill(&mut seed);
    let (pk, sk, _) = seed_keypair(ckp, &seed);
    let m = rng.bytes(37);
    let mut good = [0u8; 64];
    let mut sl: u64 = 0;
    let rc = unsafe {
        cd(
            good.as_mut_ptr(),
            &mut sl,
            m.as_ptr(),
            m.len() as u64,
            sk.as_ptr(),
        )
    };
    assert_eq!(rc, 0);

    let check = |tag: &str, sig: &[u8; 64], pk: &[u8; 32], want: c_int| {
        let rc = unsafe { cv(sig.as_ptr(), m.as_ptr(), m.len() as u64, pk.as_ptr()) };
        let rr = unsafe { rv(sig.as_ptr(), m.as_ptr(), m.len() as u64, pk.as_ptr()) };
        common::eqi(tag, rc, rr);
        assert_eq!(rc, want, "{tag}: C returned {rc}, expected {want}");
    };

    check("good sig", &good, &pk, 0);

    // ---- E: (sig[63] & 240) != 0 && !sc25519_is_canonical(S) --------------
    // S = L, S = L+1, S = 2L, S = 2^256-1
    let mut noncanon: Vec<[u8; 32]> = vec![L, mul_small(&L, 2), mul_small(&L, 7), [0xffu8; 32]];
    {
        let mut s = L;
        s[0] = s[0].wrapping_add(1);
        noncanon.push(s);
        let mut s = L;
        s[31] = 0x20;
        noncanon.push(s);
    }
    let mut hit = 0;
    for (i, s) in noncanon.iter().enumerate() {
        assert_eq!(
            unsafe { (it.sc_is_canonical)(s.as_ptr()) },
            0,
            "S#{i} should be non-canonical"
        );
        assert_ne!(s[31] & 240, 0, "S#{i} must have a nonzero high nibble");
        let mut sig = good;
        sig[32..].copy_from_slice(s);
        check(&format!("non-canonical S #{i}"), &sig, &pk, -1);
        hit += 1;
    }
    assert!(hit >= 6);

    // A canonical S with a nonzero high nibble is NOT rejected by that check:
    // S = L-1 has sig[63] == 0x10.
    {
        let mut s = L;
        s[0] -= 1;
        assert_eq!(unsafe { (it.sc_is_canonical)(s.as_ptr()) }, 1);
        assert_ne!(s[31] & 240, 0);
        let mut sig = good;
        sig[32..].copy_from_slice(&s);
        // it survives the canonicity gate and then fails the equation
        check("canonical S = L-1", &sig, &pk, -1);
    }

    // ---- E: ge25519_is_canonical(pk) == 0 --------------------------------
    let mut ncpk = 0;
    for k in 0..=6u8 {
        let mut p = [0xffu8; 32];
        p[0] = 0xed_u8.wrapping_add(k);
        p[31] = 0x7f;
        assert_eq!(unsafe { (it.ge_is_canonical)(p.as_ptr()) }, 0);
        check(&format!("non-canonical pk p+{k}"), &good, &p, -1);
        ncpk += 1;
        // and with the high bit set (ignored by is_canonical)
        let mut p2 = p;
        p2[31] |= 0x80;
        check(&format!("non-canonical pk p+{k} hi"), &good, &p2, -1);
        ncpk += 1;
    }
    assert!(ncpk >= 14);

    // ---- E: pk off-curve / small order / (main-subgroup NOT checked here) --
    let mut off_curve = 0;
    let mut small_pk = 0;
    for h in ED_SMALL_ORDER_HEX {
        let p = unhex(h);
        let k = pk_class(&it, &p);
        if k == 2 {
            off_curve += 1;
        }
        if k == 3 {
            small_pk += 1;
        }
        check(&format!("small-order pk {h}"), &good, &p, -1);
    }
    assert!(small_pk >= 6, "small-order pk hits: {small_pk}");
    // all-zero pk
    check("all-zero pk", &good, &[0u8; 32], -1);
    // random pk: mostly off-curve, all must be rejected
    for i in 0..40 {
        let mut p = [0u8; 32];
        rng.fill(&mut p);
        let k = pk_class(&it, &p);
        if unsafe { (it.ge_is_canonical)(p.as_ptr()) } == 0 {
            check(&format!("random pk #{i} noncanon"), &good, &p, -1);
            continue;
        }
        if k == 2 {
            off_curve += 1;
        }
        check(&format!("random pk #{i}"), &good, &p, -1);
    }
    assert!(off_curve >= 5, "off-curve pk hits: {off_curve}");

    // ---- E: R off-curve / small order ------------------------------------
    let mut r_small = 0;
    let mut r_bad = 0;
    for h in ED_SMALL_ORDER_HEX {
        let mut sig = good;
        sig[..32].copy_from_slice(&unhex(h));
        let mut ge = [0u8; 160];
        let dec = unsafe { (it.ge_frombytes)(ge.as_mut_ptr(), sig.as_ptr()) };
        if dec != 0 {
            r_bad += 1;
        } else if unsafe { (it.ge_has_small_order)(ge.as_ptr()) } != 0 {
            r_small += 1;
        }
        check(&format!("small-order R {h}"), &sig, &pk, -1);
    }
    assert!(r_small >= 6, "small-order R hits: {r_small}");
    // all-zero R
    {
        let mut sig = good;
        sig[..32].copy_from_slice(&[0u8; 32]);
        check("all-zero R", &sig, &pk, -1);
    }
    for i in 0..40 {
        let mut sig = good;
        let mut r = [0u8; 32];
        rng.fill(&mut r);
        sig[..32].copy_from_slice(&r);
        let mut ge = [0u8; 160];
        if unsafe { (it.ge_frombytes)(ge.as_mut_ptr(), sig.as_ptr()) } != 0 {
            r_bad += 1;
        }
        check(&format!("random R #{i}"), &sig, &pk, -1);
    }
    assert!(r_bad >= 5, "off-curve R hits: {r_bad}");

    // ---- all-zero signature ----------------------------------------------
    check("all-zero sig", &[0u8; 64], &pk, -1);
    check("all-zero sig + all-zero pk", &[0u8; 64], &[0u8; 32], -1);
    check("all-ff sig", &[0xffu8; 64], &pk, -1);

    // ---- mlen = 0 ---------------------------------------------------------
    {
        let mut sig0 = [0u8; 64];
        let mut sl: u64 = 0;
        let rc = unsafe {
            cd(
                sig0.as_mut_ptr(),
                &mut sl,
                core::ptr::null(),
                0,
                sk.as_ptr(),
            )
        };
        assert_eq!(rc, 0);
        let a = unsafe { cv(sig0.as_ptr(), core::ptr::null(), 0, pk.as_ptr()) };
        let b = unsafe { rv(sig0.as_ptr(), core::ptr::null(), 0, pk.as_ptr()) };
        common::eqi("verify mlen=0 NULL m", a, b);
        assert_eq!(a, 0);
    }
}

// ===========================================================================
// 9. ed25519ph streaming
// ===========================================================================

#[test]
fn ed25519ph_streaming() {
    let statebytes = {
        let (c, r) = both!("crypto_sign_ed25519ph_statebytes", FnSize);
        let (cv, rv) = unsafe { (c(), r()) };
        assert_eq!(cv, rv);
        cv
    };

    let (ci, ri) = both!("crypto_sign_ed25519ph_init", FnStInit);
    let (cu, ru) = both!("crypto_sign_ed25519ph_update", FnStUpdate);
    let (cc, rc_) = both!("crypto_sign_ed25519ph_final_create", FnStCreate);
    let (cfv, rfv) = both!("crypto_sign_ed25519ph_final_verify", FnStVerify);
    let (ci2, ri2) = both!("crypto_sign_init", FnStInit);
    let (cu2, ru2) = both!("crypto_sign_update", FnStUpdate);
    let (cc2, rc2) = both!("crypto_sign_final_create", FnStCreate);
    let (cfv2, rfv2) = both!("crypto_sign_final_verify", FnStVerify);
    let (ckp, _) = both!(
        "crypto_sign_ed25519_seed_keypair",
        unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> c_int
    );
    let mut rng = common::Rng::new(0x9110_0001);

    for &mlen in MLENS.iter() {
        for pass in 0..2 {
            let mut seed = [0u8; 32];
            rng.fill(&mut seed);
            let (pk, sk, _) = seed_keypair(ckp, &seed);
            let m = rng.bytes(mlen);

            // random chunk split
            let mut chunks: Vec<usize> = Vec::new();
            let mut left = mlen;
            while left > 0 {
                let take = 1 + rng.below(left);
                chunks.push(take);
                left -= take;
            }
            if pass == 1 {
                // one extra zero-length update at the front and the back
                chunks.insert(0, 0);
                chunks.push(0);
            }

            // (init, update*) on both, comparing the FULL opaque state buffer.
            // crypto_sign_ed25519ph_state is a plain struct
            // { uint64 state[8]; uint64 count[2]; uint8 buf[128]; } == 208 bytes
            // with no padding, so a byte-exact comparison is valid.
            let mut cst = vec![0x5Au8; statebytes];
            let mut rst = vec![0x5Au8; statebytes];
            let a = unsafe { ci(cst.as_mut_ptr()) };
            let b = unsafe { ri(rst.as_mut_ptr()) };
            common::eqi("ph_init", a, b);
            assert_eq!(a, 0);
            common::eqb("ph_init state", &cst, &rst);

            let mut off = 0usize;
            for (k, &n) in chunks.iter().enumerate() {
                let a = unsafe { cu(cst.as_mut_ptr(), m.as_ptr().add(off), n as u64) };
                let b = unsafe { ru(rst.as_mut_ptr(), m.as_ptr().add(off), n as u64) };
                common::eqi(&format!("ph_update mlen={mlen} chunk {k}"), a, b);
                assert_eq!(a, 0);
                common::eqb(&format!("ph_update state mlen={mlen} chunk {k}"), &cst, &rst);
                off += n;
            }
            assert_eq!(off, mlen);

            // final_create (state is consumed) -- keep a copy for final_verify
            let cst_saved = cst.clone();
            let rst_saved = rst.clone();

            let mut csig = [0xA5u8; 96];
            let mut rsig = [0xA5u8; 96];
            let mut cl: u64 = 0;
            let mut rl: u64 = 0;
            let a = unsafe {
                cc(
                    cst.as_mut_ptr(),
                    csig.as_mut_ptr().add(16),
                    &mut cl,
                    sk.as_ptr(),
                )
            };
            let b = unsafe {
                rc_(
                    rst.as_mut_ptr(),
                    rsig.as_mut_ptr().add(16),
                    &mut rl,
                    sk.as_ptr(),
                )
            };
            common::eqi("ph_final_create", a, b);
            assert_eq!(a, 0);
            assert_eq!(cl, 64);
            assert_eq!(rl, 64);
            common::eqb("ph_final_create sig", &csig, &rsig);
            common::eqb("ph_final_create state", &cst, &rst);

            // siglen_p == NULL
            let mut cst3 = cst_saved.clone();
            let mut rst3 = rst_saved.clone();
            let mut c2 = [0xA5u8; 96];
            let mut r2 = [0xA5u8; 96];
            let a = unsafe {
                cc(
                    cst3.as_mut_ptr(),
                    c2.as_mut_ptr().add(16),
                    core::ptr::null_mut(),
                    sk.as_ptr(),
                )
            };
            let b = unsafe {
                rc_(
                    rst3.as_mut_ptr(),
                    r2.as_mut_ptr().add(16),
                    core::ptr::null_mut(),
                    sk.as_ptr(),
                )
            };
            common::eqi("ph_final_create NULL siglen_p", a, b);
            common::eqb("ph_final_create NULL siglen_p", &c2, &r2);
            common::eqb("ph_final_create NULL == non-NULL", &csig, &c2);

            let mut sig = [0u8; 64];
            sig.copy_from_slice(&csig[16..80]);

            // final_verify with the saved state
            let mut cst2 = cst_saved.clone();
            let mut rst2 = rst_saved.clone();
            let a = unsafe { cfv(cst2.as_mut_ptr(), sig.as_ptr(), pk.as_ptr()) };
            let b = unsafe { rfv(rst2.as_mut_ptr(), sig.as_ptr(), pk.as_ptr()) };
            common::eqi("ph_final_verify", a, b);
            assert_eq!(a, 0);
            common::eqb("ph_final_verify state", &cst2, &rst2);

            // final_verify with a tampered signature (all 64 bytes)
            for i in 0..64usize {
                let mut bad = sig;
                bad[i] ^= 0x01;
                let mut cst2 = cst_saved.clone();
                let mut rst2 = rst_saved.clone();
                let a = unsafe { cfv(cst2.as_mut_ptr(), bad.as_ptr(), pk.as_ptr()) };
                let b = unsafe { rfv(rst2.as_mut_ptr(), bad.as_ptr(), pk.as_ptr()) };
                common::eqi(&format!("ph_final_verify tampered {i}"), a, b);
                assert_eq!(a, -1);
                common::eqb("ph_final_verify tampered state", &cst2, &rst2);
            }
            // wrong pk
            let mut seed2 = [0u8; 32];
            rng.fill(&mut seed2);
            let (pk2, _, _) = seed_keypair(ckp, &seed2);
            let mut cst2 = cst_saved.clone();
            let mut rst2 = rst_saved.clone();
            let a = unsafe { cfv(cst2.as_mut_ptr(), sig.as_ptr(), pk2.as_ptr()) };
            let b = unsafe { rfv(rst2.as_mut_ptr(), sig.as_ptr(), pk2.as_ptr()) };
            common::eqi("ph_final_verify wrong pk", a, b);
            assert_eq!(a, -1);

            // ---- the generic crypto_sign_* dispatchers, same message ------
            let mut cst4 = vec![0x5Au8; statebytes];
            let mut rst4 = vec![0x5Au8; statebytes];
            let a = unsafe { ci2(cst4.as_mut_ptr()) };
            let b = unsafe { ri2(rst4.as_mut_ptr()) };
            common::eqi("crypto_sign_init", a, b);
            common::eqb("crypto_sign_init state", &cst4, &rst4);
            let a = unsafe { cu2(cst4.as_mut_ptr(), m.as_ptr(), mlen as u64) };
            let b = unsafe { ru2(rst4.as_mut_ptr(), m.as_ptr(), mlen as u64) };
            common::eqi("crypto_sign_update", a, b);
            common::eqb("crypto_sign_update state", &cst4, &rst4);
            let cst5 = cst4.clone();
            let rst5 = rst4.clone();
            let mut c3 = [0xA5u8; 96];
            let mut r3 = [0xA5u8; 96];
            let mut x: u64 = 0;
            let mut y: u64 = 0;
            let a = unsafe {
                cc2(
                    cst4.as_mut_ptr(),
                    c3.as_mut_ptr().add(16),
                    &mut x,
                    sk.as_ptr(),
                )
            };
            let b = unsafe {
                rc2(
                    rst4.as_mut_ptr(),
                    r3.as_mut_ptr().add(16),
                    &mut y,
                    sk.as_ptr(),
                )
            };
            common::eqi("crypto_sign_final_create", a, b);
            assert_eq!(x, y);
            common::eqb("crypto_sign_final_create", &c3, &r3);
            // one-shot updates must give the same signature as the chunked ones
            common::eqb("chunked == one-shot", &csig, &c3);
            let mut cst6 = cst5.clone();
            let mut rst6 = rst5.clone();
            let a = unsafe { cfv2(cst6.as_mut_ptr(), sig.as_ptr(), pk.as_ptr()) };
            let b = unsafe { rfv2(rst6.as_mut_ptr(), sig.as_ptr(), pk.as_ptr()) };
            common::eqi("crypto_sign_final_verify", a, b);
            assert_eq!(a, 0);
        }
    }
}

/// `_crypto_sign_ed25519_ref10_hinit()` on its own.
#[test]
fn hinit() {
    let statebytes = {
        let (c, _) = both!("crypto_sign_ed25519ph_statebytes", FnSize);
        unsafe { c() }
    };
    let (ch, rh) = both!("_crypto_sign_ed25519_ref10_hinit", FnHinit);
    let (cf, rf) = both!(
        "crypto_hash_sha512_final",
        unsafe extern "C" fn(*mut u8, *mut u8) -> c_int
    );

    for ph in [0i32, 1, 2, -1, i32::MIN, i32::MAX] {
        let mut cst = vec![0x5Au8; statebytes];
        let mut rst = vec![0x5Au8; statebytes];
        unsafe {
            ch(cst.as_mut_ptr(), ph);
            rh(rst.as_mut_ptr(), ph);
        }
        common::eqb(&format!("hinit state ph={ph}"), &cst, &rst);
        // and the digest of the (possibly DOM2-prefixed) empty message
        let mut co = [0u8; 64];
        let mut ro = [0u8; 64];
        let a = unsafe { cf(cst.as_mut_ptr(), co.as_mut_ptr()) };
        let b = unsafe { rf(rst.as_mut_ptr(), ro.as_mut_ptr()) };
        common::eqi(&format!("hinit final ph={ph}"), a, b);
        common::eqb(&format!("hinit digest ph={ph}"), &co, &ro);
        if ph == 0 {
            // SHA-512 of the empty string
            assert_eq!(
                common::hex(&co),
                "cf83e1357eefb8bdf1542850d66d8007d620e4050b5715dc83f4a921d36ce9ce\
                 47d0d13c5d85f2b0ff8318d2877eec2f63b931bd47417a81a538327af927da3e"
            );
        } else {
            assert_ne!(common::hex(&co), common::hex(&[0u8; 64]));
        }
    }
    // prehashed != 0 all produce the same state
    let mut a = vec![0x5Au8; statebytes];
    let mut b = vec![0x5Au8; statebytes];
    unsafe {
        ch(a.as_mut_ptr(), 1);
        ch(b.as_mut_ptr(), -7);
    }
    common::eqb("hinit prehashed truthiness", &a, &b);
}

// ===========================================================================
// 10. pk_to_curve25519 / sk_to_curve25519
// ===========================================================================

#[test]
fn key_conversion() {
    let it = internals();
    let (cp, rp) = both!("crypto_sign_ed25519_pk_to_curve25519", Fn2);
    let (csk, rsk) = both!("crypto_sign_ed25519_sk_to_curve25519", Fn2);
    let (ckp, _) = both!(
        "crypto_sign_ed25519_seed_keypair",
        unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> c_int
    );
    let (cx, rx) = both!("crypto_scalarmult_curve25519_base", Fn2);
    let mut rng = common::Rng::new(0xC0E0_0001);

    let mut cls = [0usize; 5];

    // valid ed25519 public keys
    for i in 0..20 {
        let mut seed = [0u8; 32];
        rng.fill(&mut seed);
        let (pk, sk, _) = seed_keypair(ckp, &seed);
        cls[pk_class(&it, &pk) as usize] += 1;

        let (rc, xpk) = cmp2(&format!("pk_to_curve25519 #{i}"), cp, rp, &pk, 32);
        assert_eq!(rc, 0);

        let mut sk32 = [0u8; 32];
        sk32.copy_from_slice(&sk[..32]);
        // sk_to_curve25519 only reads the first 32 bytes (the seed)
        let (rc2, xsk) = cmp2(&format!("sk_to_curve25519 #{i}"), csk, rsk, &sk32, 32);
        assert_eq!(rc2, 0);

        // the converted key pair must be a valid X25519 key pair
        let mut xsk32 = [0u8; 32];
        xsk32.copy_from_slice(&xsk);
        let (_, derived) = cmp2(&format!("x25519 base #{i}"), cx, rx, &xsk32, 32);
        common::eqb("pk_to_curve25519 == x25519_base(sk_to_curve25519)", &xpk, &derived);
    }

    // invalid pk: off-curve, small order, off-main-subgroup, non-canonical
    let mut bad: Vec<[u8; 32]> = Vec::new();
    for h in ED_SMALL_ORDER_HEX {
        bad.push(unhex(h));
    }
    for k in 0..=6u8 {
        let mut p = [0xffu8; 32];
        p[0] = 0xed_u8.wrapping_add(k);
        p[31] = 0x7f;
        bad.push(p);
    }
    for _ in 0..60 {
        let mut p = [0u8; 32];
        rng.fill(&mut p);
        bad.push(p);
    }
    let mut rejected = 0usize;
    for (i, p) in bad.iter().enumerate() {
        let k = pk_class(&it, p);
        cls[k as usize] += 1;
        let (rc, _) = cmp2(&format!("pk_to_curve25519 bad #{i}"), cp, rp, p, 32);
        if k == 0 {
            assert_eq!(rc, 0, "class 0 must be accepted");
        } else {
            assert_eq!(rc, -1, "class {k} must be rejected");
            rejected += 1;
        }
    }
    assert!(cls[2] >= 5, "off-curve: {cls:?}");
    assert!(cls[3] >= 6, "small order: {cls:?}");
    assert!(cls[4] >= 5, "off main subgroup: {cls:?}");
    assert!(rejected >= 40);

    // sk_to_curve25519 never fails, whatever the input
    for i in 0..20 {
        let mut sk = [0u8; 32];
        rng.fill(&mut sk);
        let (rc, _) = cmp2(&format!("sk_to_curve25519 random #{i}"), csk, rsk, &sk, 32);
        assert_eq!(rc, 0);
    }
    for s in [[0u8; 32], [0xffu8; 32]] {
        let (rc, _) = cmp2("sk_to_curve25519 edge", csk, rsk, &s, 32);
        assert_eq!(rc, 0);
    }
}

// ===========================================================================
// 11. cross-library interoperability (C signs, Rust verifies and vice versa)
// ===========================================================================

#[test]
fn cross_library_interop() {
    let l = common::libs();
    let cd = getsym!(l.c, "crypto_sign_ed25519_detached", FnSign);
    let rd = getsym!(l.r, "crypto_sign_ed25519_detached", FnSign);
    let cvf = getsym!(l.c, "crypto_sign_ed25519_verify_detached", FnVerify);
    let rvf = getsym!(l.r, "crypto_sign_ed25519_verify_detached", FnVerify);
    let ck = getsym!(l.c, "crypto_sign_ed25519_keypair", Fn2Keypair);
    let rk = getsym!(l.r, "crypto_sign_ed25519_keypair", Fn2Keypair);
    let mut rng = common::Rng::new(0xC205_0001);

    // keys generated with the (independent) RNG of each library
    for (kf, tag) in [(ck, "C keypair"), (rk, "Rust keypair")] {
        let mut pk = [0u8; 32];
        let mut sk = [0u8; 64];
        assert_eq!(unsafe { kf(pk.as_mut_ptr(), sk.as_mut_ptr()) }, 0, "{tag}");
        for &mlen in [0usize, 1, 33, 128].iter() {
            let m = rng.bytes(mlen);
            for (sf, sname) in [(cd, "C sign"), (rd, "Rust sign")] {
                let mut sig = [0u8; 64];
                let mut sl: u64 = 0;
                assert_eq!(
                    unsafe {
                        sf(
                            sig.as_mut_ptr(),
                            &mut sl,
                            m.as_ptr(),
                            mlen as u64,
                            sk.as_ptr(),
                        )
                    },
                    0
                );
                assert_eq!(sl, 64);
                let a = unsafe { cvf(sig.as_ptr(), m.as_ptr(), mlen as u64, pk.as_ptr()) };
                let b = unsafe { rvf(sig.as_ptr(), m.as_ptr(), mlen as u64, pk.as_ptr()) };
                assert_eq!(a, 0, "{tag}/{sname}: C verify failed");
                assert_eq!(b, 0, "{tag}/{sname}: Rust verify failed");
            }
        }
    }
}

// keep `c_void` referenced so the import is not flagged
const _: Option<*const c_void> = None;
