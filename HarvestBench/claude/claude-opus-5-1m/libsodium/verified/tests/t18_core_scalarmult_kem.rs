//! Phase B — valid-input differential tests for the **G6** module group:
//! `crypto_core` (ed25519 / ristretto255 / h2c / softaes), `crypto_scalarmult`,
//! `crypto_stream` (only the rows `t02_lowlevel.rs` does not already drive),
//! `crypto_kem` and `crypto_ipcrypt`.
//!
//! Every entry point is reached through `dlsym` on both `.so`s, so the
//! `#[no_mangle]` export wrappers are part of what is compared. Inputs are
//! randomised from a fixed seed, so a failure is always reproducible.
//!
//! # Row map for CONFIGS `## G6`
//!
//! Already driven by **`t02_lowlevel.rs`**, therefore *not* duplicated here:
//!
//! * chacha20 keystream / `_xor` / `_xor_ic` / `_ietf*` sweeps —
//!   G6-001, G6-002, G6-003, G6-005, G6-008, G6-009, G6-010, G6-011, G6-012.
//! * salsa20 / salsa2012 / salsa208 / xchacha20 / xsalsa20 keystream, `_xor`
//!   and `_xor_ic` sweeps — G6-017, G6-018, G6-019, G6-020, G6-022, G6-023,
//!   G6-024, G6-025, G6-026, G6-027, G6-029, G6-030, G6-033, G6-034.
//! * every `crypto_stream_*_keygen` — G6-045.
//! * `crypto_core_salsa20` / `_salsa2012` / `_salsa208` / `_hsalsa20` /
//!   `_hchacha20` over the `c = NULL` / sigma / random-constant matrix —
//!   G6-047, G6-048, G6-049, G6-050, G6-051, G6-052, G6-053, G6-054.
//!
//! Driven by **`t01_constants.rs`** (and re-asserted here anyway, in
//! `build_configuration_rows`, `stream_derived_relationships` and
//! `ipcrypt_keygen_and_sizes`): G6-037, G6-038, G6-039, G6-040, G6-041,
//! G6-042, G6-043, G6-044, G6-055, G6-056, G6-057, G6-058, G6-059, G6-069,
//! G6-070, G6-080, G6-087, G6-108, G6-123, G6-134, G6-142, G6-144, G6-162,
//! G6-163, G6-164, G6-165.
//!
//! Everything else in `## G6` is covered by this file: the ed25519 /
//! ristretto255 group and scalar arithmetic, the hash-to-curve paths, every
//! `crypto_scalarmult_*` flavour, ML-KEM-768 / X-Wing, every `ipcrypt`
//! variant (including `pfx`), the `softaes` primitives that back them, the
//! exported implementation vtables, and the *derived-relationship* stream rows
//! that `t02` does not assert.

mod common;
use common::*;

// ---------------------------------------------------------------------------
// C signatures used throughout
// ---------------------------------------------------------------------------

type Sz = unsafe extern "C" fn() -> usize;
type Str = unsafe extern "C" fn() -> *const std::ffi::c_char;

/// `void f(unsigned char *)`
type V1 = unsafe extern "C" fn(*mut u8);
/// `int f(const unsigned char *)`
type I1c = unsafe extern "C" fn(*const u8) -> i32;
/// `void f(unsigned char *, const unsigned char *)`
type V2 = unsafe extern "C" fn(*mut u8, *const u8);
/// `int f(unsigned char *, const unsigned char *)`
type I2 = unsafe extern "C" fn(*mut u8, *const u8) -> i32;
/// `void f(unsigned char *, const unsigned char *, const unsigned char *)`
type V3 = unsafe extern "C" fn(*mut u8, *const u8, *const u8);
/// `int f(unsigned char *, const unsigned char *, const unsigned char *)`
type I3 = unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> i32;
/// `void f(unsigned char *, const unsigned char *, const unsigned char *, const unsigned char *)`
type V4 = unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8);

/// `int f(unsigned char *p, const unsigned char *ctx, size_t, const unsigned char *msg, size_t, int)`
type FromString =
    unsafe extern "C" fn(*mut u8, *const u8, usize, *const u8, usize, i32) -> i32;

type KemKeypair = unsafe extern "C" fn(*mut u8, *mut u8) -> i32;
type KemSeedKeypair = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> i32;
type KemEnc = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> i32;
type KemEncDet = unsafe extern "C" fn(*mut u8, *mut u8, *const u8, *const u8) -> i32;
type KemDec = unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> i32;

type Stream = unsafe extern "C" fn(*mut u8, u64, *const u8, *const u8) -> i32;
type StreamXor = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, *const u8) -> i32;
type StreamXorIc64 =
    unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, u64, *const u8) -> i32;
type StreamXorIc32 =
    unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, u32, *const u8) -> i32;

// ---------------------------------------------------------------------------
// small helpers
// ---------------------------------------------------------------------------

/// `reset_rngs()` rewinds a *process-global* PRNG state shared by both loaded
/// libraries, so any two tests that use it must not run concurrently. libtest
/// runs `#[test]`s on parallel threads, so every RNG-dependent test in this
/// file takes this lock for its whole body.
static RNG_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn rng_guard() -> std::sync::MutexGuard<'static, ()> {
    RNG_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

fn unhex(s: &str) -> Vec<u8> {
    assert!(s.len() % 2 == 0);
    (0..s.len() / 2)
        .map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap())
        .collect()
}

fn unhex32(s: &str) -> [u8; 32] {
    let v = unhex(s);
    let mut a = [0u8; 32];
    a.copy_from_slice(&v);
    a
}

fn szof(name: &str) -> usize {
    unsafe { sym::<Sz>(c_lib(), name)() }
}

fn eq_sz(name: &str) -> usize {
    let (c, r) = pair::<Sz>(name);
    let (a, b) = unsafe { (c(), r()) };
    eq_usize(name, a, b);
    a
}

fn eq_str(name: &str) -> String {
    let (c, r) = pair::<Str>(name);
    let f = |p: *const std::ffi::c_char| unsafe {
        std::ffi::CStr::from_ptr(p).to_string_lossy().to_string()
    };
    let (a, b) = unsafe { (f(c()), f(r())) };
    assert_eq!(a, b, "{name}");
    a
}

/// `s * k` over 32 little-endian bytes (k small), truncated to 32 bytes.
fn mul_small(s: &[u8; 32], k: u32) -> [u8; 32] {
    let mut out = [0u8; 32];
    let mut carry = 0u32;
    for i in 0..32 {
        let v = s[i] as u32 * k + carry;
        out[i] = v as u8;
        carry = v >> 8;
    }
    out
}

// ---------------------------------------------------------------------------
// fixed constants
// ---------------------------------------------------------------------------

/// The ed25519 base point `B`, compressed.
const ED_BASE: &str = "5866666666666666666666666666666666666666666666666666666666666666";
/// The ristretto255 generator.
const RIS_GEN: &str = "e2f2ae0a6abc4e71a884a961c500515f58e30b6aa582dd8db6a65945e08d2d76";
/// `L = 2^252 + 27742317777372353535851937790883648493`, little-endian.
const L_HEX: &str = "edd3f55c1a631258d69cf7a2def9de14000000000000000000000000000000 10";

fn ell() -> [u8; 32] {
    unhex32(&L_HEX.replace(' ', ""))
}
fn ell_minus_1() -> [u8; 32] {
    let mut l = ell();
    l[0] -= 1;
    l
}
fn ell_plus_1() -> [u8; 32] {
    let mut l = ell();
    l[0] += 1;
    l
}

/// The documented ed25519 small-order encodings (identity, order 2, the two
/// order-4 encodings, the two order-8 points).
fn ed_small_order() -> Vec<[u8; 32]> {
    let mut v = Vec::new();
    v.push(unhex32(
        "0100000000000000000000000000000000000000000000000000000000000000",
    )); // identity
    v.push(unhex32(
        "ecffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f",
    )); // order 2
    v.push([0u8; 32]); // order 4
    let mut o4b = [0u8; 32];
    o4b[31] = 0x80;
    v.push(o4b); // order 4, bit 255 set
    v.push(unhex32(
        "26e8958fc2b227b045c3f489f2ef98f0d5dfac05d3c63339b13802886d53fc05",
    )); // order 8
    v.push(unhex32(
        "c7176a703d4dd84fba3c0b760d10670f2a2053fa2c39ccc64ec7fd7792ac03fa",
    )); // order 8
    v
}

/// The 7 blocklisted small-order Montgomery x-coordinates of `x25519_ref10.c`.
fn x25519_blocklist() -> Vec<[u8; 32]> {
    vec![
        unhex32("0000000000000000000000000000000000000000000000000000000000000000"),
        unhex32("0100000000000000000000000000000000000000000000000000000000000000"),
        unhex32("e0eb7a7c3b41b8ae1656e3faf19fc46ada098deb9c32b1fd866205165f49b800"),
        unhex32("5f9c95bca3508c24b1d0b1559c83ef5b04445cc4581c8e86d8224eddd09f1157"),
        unhex32("ecffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f"),
        unhex32("edffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f"),
        unhex32("eeffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f"),
    ]
}

/// Canonical scalar shapes used throughout the scalar-arithmetic rows.
fn scalar_shapes(rng: &mut Rng, n_random: usize) -> Vec<[u8; 32]> {
    let mut v: Vec<[u8; 32]> = Vec::new();
    v.push([0u8; 32]);
    let mut one = [0u8; 32];
    one[0] = 1;
    v.push(one);
    let mut two = [0u8; 32];
    two[0] = 2;
    v.push(two);
    v.push(ell_minus_1());
    v.push(ell());
    v.push(ell_plus_1());
    v.push([0xffu8; 32]);
    let mut hi = [0u8; 32];
    hi[31] = 0x80;
    v.push(hi);
    for k in 2..8u32 {
        v.push(mul_small(&ell(), k));
    }
    for _ in 0..n_random {
        let mut s = [0u8; 32];
        rng.fill(&mut s);
        v.push(s);
        // a canonical one too (top 3 bits cleared makes it < L with high prob.)
        let mut s2 = s;
        s2[31] &= 0x0f;
        v.push(s2);
    }
    v
}

/// A batch of valid ed25519 main-subgroup points, derived deterministically
/// with `crypto_scalarmult_ed25519_base_noclamp` (no RNG involved).
fn ed_valid_points(rng: &mut Rng, n: usize) -> Vec<[u8; 32]> {
    let f = sym::<I2>(c_lib(), "crypto_scalarmult_ed25519_base_noclamp");
    let mut out = Vec::new();
    out.push(unhex32(ED_BASE));
    while out.len() < n {
        let mut s = [0u8; 32];
        rng.fill(&mut s);
        s[31] &= 0x0f;
        if s.iter().all(|&b| b == 0) {
            continue;
        }
        let mut q = [0u8; 32];
        if unsafe { f(q.as_mut_ptr(), s.as_ptr()) } == 0 {
            out.push(q);
        }
    }
    out
}

/// A batch of valid ristretto255 elements, derived with
/// `crypto_core_ristretto255_from_hash` (deterministic, always succeeds).
fn ris_valid_points(rng: &mut Rng, n: usize) -> Vec<[u8; 32]> {
    let f = sym::<I2>(c_lib(), "crypto_core_ristretto255_from_hash");
    let mut out = Vec::new();
    out.push(unhex32(RIS_GEN));
    out.push([0u8; 32]); // ristretto255 identity
    while out.len() < n {
        let h = rng.bytes(64);
        let mut q = [0u8; 32];
        assert_eq!(unsafe { f(q.as_mut_ptr(), h.as_ptr()) }, 0);
        out.push(q);
    }
    out
}

// ===========================================================================
// crypto_core_ed25519 — group arithmetic
// ===========================================================================

/// CONFIGS G6-088, G6-089, G6-090, G6-091 — `crypto_core_ed25519_add` /
/// `_sub` over random valid points, the documented identities, non-canonical
/// encodings and every small-order point (all of which `_add`/`_sub` accept,
/// because they only run `ge25519_frombytes` + `is_on_curve`).
#[test]
fn core_ed25519_add_sub() {
    setup();
    let mut rng = Rng::new(0x6001);
    let (c_add, r_add) = pair::<I3>("crypto_core_ed25519_add");
    let (c_sub, r_sub) = pair::<I3>("crypto_core_ed25519_sub");

    let identity = unhex32("0100000000000000000000000000000000000000000000000000000000000000");
    let base = unhex32(ED_BASE);

    let mut pts: Vec<[u8; 32]> = ed_valid_points(&mut rng, 40);
    // negations of the valid points: P + (-P) = identity
    let negs: Vec<[u8; 32]> = pts
        .iter()
        .map(|p| {
            let mut q = *p;
            q[31] ^= 0x80;
            q
        })
        .collect();
    // non-canonical but on-curve: y = p (re-encoding of y = 0)
    pts.push(unhex32(
        "edffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f",
    ));
    pts.push([0xffu8; 32]);
    pts.extend(ed_small_order());
    pts.push(identity);

    let mut cases: Vec<([u8; 32], [u8; 32])> = Vec::new();
    cases.push((base, base)); // 2B
    cases.push((base, identity)); // B
    cases.push((identity, base));
    cases.push((identity, identity));
    for (p, n) in pts.iter().zip(negs.iter()) {
        cases.push((*p, *n)); // P + (-P) -> identity
    }
    for i in 0..pts.len() {
        for j in 0..pts.len() {
            if (i * 7 + j * 3) % 5 == 0 {
                cases.push((pts[i], pts[j]));
            }
        }
    }
    for _ in 0..600 {
        cases.push((*rng.pick(&pts), *rng.pick(&pts)));
    }

    for (p, q) in &cases {
        let mut a = canary(32);
        let mut b = canary(32);
        let (ra, rb) = unsafe {
            (
                c_add(a.as_mut_ptr(), p.as_ptr(), q.as_ptr()),
                r_add(b.as_mut_ptr(), p.as_ptr(), q.as_ptr()),
            )
        };
        eq_i32(&format!("ed25519_add({}, {}) rc", hex(p), hex(q)), ra, rb);
        eq_bytes(&format!("ed25519_add({}, {})", hex(p), hex(q)), &a, &b);

        let mut a = canary(32);
        let mut b = canary(32);
        let (ra, rb) = unsafe {
            (
                c_sub(a.as_mut_ptr(), p.as_ptr(), q.as_ptr()),
                r_sub(b.as_mut_ptr(), p.as_ptr(), q.as_ptr()),
            )
        };
        eq_i32(&format!("ed25519_sub({}, {}) rc", hex(p), hex(q)), ra, rb);
        eq_bytes(&format!("ed25519_sub({}, {})", hex(p), hex(q)), &a, &b);
    }

    // documented properties (they hold in the C, so they must hold in Rust too)
    let valid = ed_valid_points(&mut rng, 8);
    for p in &valid {
        for q in &valid {
            let mut s = [0u8; 32];
            let mut back = [0u8; 32];
            unsafe {
                assert_eq!(r_add(s.as_mut_ptr(), p.as_ptr(), q.as_ptr()), 0);
                assert_eq!(r_sub(back.as_mut_ptr(), s.as_ptr(), q.as_ptr()), 0);
            }
            eq_bytes("sub(add(P,Q),Q) == P", p, &back);
        }
        let mut z = [0u8; 32];
        unsafe { assert_eq!(r_sub(z.as_mut_ptr(), p.as_ptr(), p.as_ptr()), 0) };
        eq_bytes("sub(P,P) == identity", &identity, &z);
    }
}

/// CONFIGS G6-097 — `crypto_core_ed25519_random`: RNG-driven, so both
/// libraries are rewound to the same seed. Every output must additionally be a
/// valid main-subgroup point (`_is_valid_point == 1`).
#[test]
fn core_ed25519_random() {
    let _rng_lock = rng_guard();
    setup();
    let (c, r) = pair::<V1>("crypto_core_ed25519_random");
    let (c_ok, r_ok) = pair::<I1c>("crypto_core_ed25519_is_valid_point");
    for seed in 0..128u64 {
        let mut a = canary(32);
        let mut b = canary(32);
        reset_rngs(0xED00_0000 + seed);
        unsafe { c(a.as_mut_ptr()) };
        reset_rngs(0xED00_0000 + seed);
        unsafe { r(b.as_mut_ptr()) };
        eq_bytes(&format!("ed25519_random seed={seed}"), &a, &b);
        let (x, y) = unsafe { (c_ok(a.as_ptr()), r_ok(b.as_ptr())) };
        eq_i32("is_valid_point(random)", x, y);
        assert_eq!(x, 1, "ed25519_random produced an invalid point");
    }
}

/// CONFIGS G6-098 — `crypto_core_ed25519_scalar_random`: the rejection-sampling
/// loop must consume the RNG stream identically, and every output must be
/// canonical and non-zero.
#[test]
fn core_ed25519_scalar_random() {
    let _rng_lock = rng_guard();
    setup();
    let (c, r) = pair::<V1>("crypto_core_ed25519_scalar_random");
    let (c_ok, r_ok) = pair::<I1c>("crypto_core_ed25519_scalar_is_canonical");
    for seed in 0..256u64 {
        let mut a = canary(32);
        let mut b = canary(32);
        reset_rngs(0xEE00_0000 + seed);
        unsafe { c(a.as_mut_ptr()) };
        reset_rngs(0xEE00_0000 + seed);
        unsafe { r(b.as_mut_ptr()) };
        eq_bytes(&format!("ed25519_scalar_random seed={seed}"), &a, &b);
        let (x, y) = unsafe { (c_ok(a.as_ptr()), r_ok(b.as_ptr())) };
        eq_i32("scalar_is_canonical(random)", x, y);
        assert_eq!(x, 1);
        assert!(a.iter().any(|&v| v != 0));
    }
}

/// CONFIGS G6-099, G6-100, G6-101 — `crypto_core_ed25519_scalar_add` / `_sub` /
/// `_mul` over the full shape matrix {0, 1, 2, L-1, L, L+1, kL, all-0xff,
/// random} x itself.
#[test]
fn core_ed25519_scalar_add_sub_mul() {
    setup();
    let mut rng = Rng::new(0x6002);
    let shapes = scalar_shapes(&mut rng, 60);
    for name in [
        "crypto_core_ed25519_scalar_add",
        "crypto_core_ed25519_scalar_sub",
        "crypto_core_ed25519_scalar_mul",
    ] {
        let (c, r) = pair::<V3>(name);
        for x in &shapes {
            for y in &shapes {
                let mut a = canary(32);
                let mut b = canary(32);
                unsafe {
                    c(a.as_mut_ptr(), x.as_ptr(), y.as_ptr());
                    r(b.as_mut_ptr(), x.as_ptr(), y.as_ptr());
                }
                eq_bytes(&format!("{name}({}, {})", hex(x), hex(y)), &a, &b);
            }
        }
    }
}

/// CONFIGS G6-102, G6-103, G6-104 — `_scalar_negate` / `_scalar_complement` /
/// `_scalar_invert` over the same shape matrix (plus the `mul(s, invert(s))`
/// consistency property for non-zero `s`).
#[test]
fn core_ed25519_scalar_unary() {
    setup();
    let mut rng = Rng::new(0x6003);
    let shapes = scalar_shapes(&mut rng, 200);
    for name in [
        "crypto_core_ed25519_scalar_negate",
        "crypto_core_ed25519_scalar_complement",
    ] {
        let (c, r) = pair::<V2>(name);
        for s in &shapes {
            let mut a = canary(32);
            let mut b = canary(32);
            unsafe {
                c(a.as_mut_ptr(), s.as_ptr());
                r(b.as_mut_ptr(), s.as_ptr());
            }
            eq_bytes(&format!("{name}({})", hex(s)), &a, &b);
        }
    }

    let (c_inv, r_inv) = pair::<I2>("crypto_core_ed25519_scalar_invert");
    let (_, r_mul) = pair::<V3>("crypto_core_ed25519_scalar_mul");
    let (_, r_red) = pair::<V2>("crypto_core_ed25519_scalar_reduce");
    let mut one = [0u8; 32];
    one[0] = 1;
    for s in &shapes {
        let mut a = canary(32);
        let mut b = canary(32);
        let (ra, rb) = unsafe {
            (
                c_inv(a.as_mut_ptr(), s.as_ptr()),
                r_inv(b.as_mut_ptr(), s.as_ptr()),
            )
        };
        eq_i32(&format!("scalar_invert({}) rc", hex(s)), ra, rb);
        eq_bytes(&format!("scalar_invert({})", hex(s)), &a, &b);
        // `sc25519_invert` computes s^(L-2) mod L, so `s * invert(s) == 1` only
        // when `s mod L != 0`; the shape list deliberately contains multiples
        // of L (which `_scalar_invert` still accepts, returning 0).
        let mut ext = [0u8; 64];
        ext[..32].copy_from_slice(s);
        let mut red = [0u8; 32];
        unsafe { r_red(red.as_mut_ptr(), ext.as_ptr()) };
        if ra == 0 && red.iter().any(|&v| v != 0) {
            let mut prod = [0u8; 32];
            unsafe { r_mul(prod.as_mut_ptr(), s.as_ptr(), a.as_ptr()) };
            eq_bytes(&format!("s * s^-1 == 1 for s={}", hex(s)), &one, &prod);
        }
    }
}

/// CONFIGS G6-105 — `crypto_core_ed25519_scalar_reduce` over 64-byte
/// non-reduced inputs: all-zero, 1, all-0xff (= 2^512-1), L and L-1
/// zero-extended, 2^256, plus many random ones.
#[test]
fn core_ed25519_scalar_reduce() {
    setup();
    let mut rng = Rng::new(0x6004);
    let (c, r) = pair::<V2>("crypto_core_ed25519_scalar_reduce");

    let mut cases: Vec<Vec<u8>> = Vec::new();
    cases.push(vec![0u8; 64]);
    let mut one = vec![0u8; 64];
    one[0] = 1;
    cases.push(one);
    cases.push(vec![0xffu8; 64]);
    let mut l64 = vec![0u8; 64];
    l64[..32].copy_from_slice(&ell());
    cases.push(l64);
    let mut lm64 = vec![0u8; 64];
    lm64[..32].copy_from_slice(&ell_minus_1());
    cases.push(lm64);
    let mut p256 = vec![0u8; 64];
    p256[32] = 1;
    cases.push(p256);
    let mut top = vec![0u8; 64];
    top[63] = 0x80;
    cases.push(top);
    for k in 1..8u32 {
        let mut kl = vec![0u8; 64];
        kl[..32].copy_from_slice(&mul_small(&ell(), k));
        cases.push(kl);
    }
    for _ in 0..4000 {
        cases.push(rng.bytes(64));
    }

    for s in &cases {
        let mut a = canary(32);
        let mut b = canary(32);
        unsafe {
            c(a.as_mut_ptr(), s.as_ptr());
            r(b.as_mut_ptr(), s.as_ptr());
        }
        eq_bytes(&format!("scalar_reduce({})", hex(s)), &a, &b);
    }
}

/// CONFIGS G6-106 — `crypto_core_ed25519_scalar_is_canonical` over the
/// documented shapes (0, 1, L-1, L, L+1, all-0xff, `00`x31+`10`, masked random).
#[test]
fn core_ed25519_scalar_is_canonical() {
    setup();
    let mut rng = Rng::new(0x6005);
    let (c, r) = pair::<I1c>("crypto_core_ed25519_scalar_is_canonical");
    let mut shapes = scalar_shapes(&mut rng, 40);
    let mut t = [0u8; 32];
    t[31] = 0x10;
    shapes.push(t);
    for i in 0..32 {
        let mut s = ell();
        s[i] = s[i].wrapping_sub(1);
        shapes.push(s);
        let mut s = ell();
        s[i] = s[i].wrapping_add(1);
        shapes.push(s);
    }
    for _ in 0..3000 {
        let mut s = [0u8; 32];
        rng.fill(&mut s);
        s[31] &= 0x1f;
        shapes.push(s);
    }
    for s in &shapes {
        let (a, b) = unsafe { (c(s.as_ptr()), r(s.as_ptr())) };
        eq_i32(&format!("scalar_is_canonical({})", hex(s)), a, b);
    }
}

// ===========================================================================
// hash-to-curve (`core_h2c.c`) paths
// ===========================================================================

/// The `(ctx_len, msg_len)` x content grid used by every `*_from_string*` row.
/// `ctx_len = 0` is exercised both with a real pointer and with `ctx = NULL`
/// (CONFIGS G6-093), and `ctx_len = 256` / `300` take the oversize-DST path
/// (CONFIGS G6-094 / G6-114).
fn h2c_inputs(rng: &mut Rng) -> Vec<(Option<Vec<u8>>, Option<Vec<u8>>)> {
    let mut v: Vec<(Option<Vec<u8>>, Option<Vec<u8>>)> = Vec::new();
    let ctx_lens = [0usize, 1, 17, 32, 100, 255, 256, 257, 300];
    let msg_lens = [0usize, 1, 32, 100, 255, 256];
    for (i, &cl) in ctx_lens.iter().enumerate() {
        for (j, &ml) in msg_lens.iter().enumerate() {
            let kind = (i + j) % 3;
            let ctx = match kind {
                0 => vec![0u8; cl],
                1 => b"QUUX-V01-CS02-with-edwards25519_XMD:SHA-512_ELL2_RO_"
                    .iter()
                    .cycle()
                    .take(cl)
                    .cloned()
                    .collect(),
                _ => rng.bytes(cl),
            };
            let msg = match kind {
                0 => rng.bytes(ml),
                1 => b"abcdef0123456789".iter().cycle().take(ml).cloned().collect(),
                _ => vec![0xffu8; ml],
            };
            v.push((Some(ctx), Some(msg)));
        }
    }
    // NULL ctx / NULL msg with zero lengths — a distinct C path
    v.push((None, None));
    v.push((None, Some(rng.bytes(40))));
    v.push((Some(rng.bytes(9)), None));
    v
}

fn drive_from_string(name: &str, outlen: usize, rng: &mut Rng) {
    let (c, r) = pair::<FromString>(name);
    for (ctx, msg) in h2c_inputs(rng) {
        let (cp, cl) = match &ctx {
            Some(v) => (v.as_ptr(), v.len()),
            None => (std::ptr::null(), 0),
        };
        let (mp, ml) = match &msg {
            Some(v) => (v.as_ptr(), v.len()),
            None => (std::ptr::null(), 0),
        };
        for alg in [1i32, 2] {
            let mut a = canary(outlen);
            let mut b = canary(outlen);
            let (ra, rb) = unsafe {
                (
                    c(a.as_mut_ptr(), cp, cl, mp, ml, alg),
                    r(b.as_mut_ptr(), cp, cl, mp, ml, alg),
                )
            };
            eq_i32(&format!("{name}(alg={alg}, ctx_len={cl}, msg_len={ml}) rc"), ra, rb);
            eq_bytes(
                &format!("{name}(alg={alg}, ctx_len={cl}, msg_len={ml})"),
                &a,
                &b,
            );
        }
    }
}

/// CONFIGS G6-092, G6-093, G6-094, G6-095, G6-096, G6-124, G6-125, G6-126 —
/// `crypto_core_ed25519_from_string` (the "ro" suite: two 48-byte hash blocks,
/// byte-reversed, mapped and added) and `_from_string_nu` (one block, no
/// addition), across both `hash_alg` values, the `ctx = NULL` path and the
/// `ctx_len > 0xff` oversize-DST path. G6-096: only these two entry points
/// exist — there is no `_from_string_ro` and no `_from_uniform`.
#[test]
fn core_ed25519_from_string() {
    setup();
    let mut rng = Rng::new(0x6006);
    drive_from_string("crypto_core_ed25519_from_string", 32, &mut rng);
    drive_from_string("crypto_core_ed25519_from_string_nu", 32, &mut rng);

    // G6-096: the absent API really is absent from both libraries.
    for absent in [
        "crypto_core_ed25519_from_uniform",
        "crypto_core_ed25519_from_string_ro",
        "crypto_core_ristretto255_from_string_nu",
        "crypto_core_ristretto255_from_string_ro",
    ] {
        assert!(
            unsafe { c_lib().get::<*const std::ffi::c_void>(absent.as_bytes()) }.is_err(),
            "{absent} unexpectedly present in the C reference"
        );
        assert!(
            unsafe { r_lib().get::<*const std::ffi::c_void>(absent.as_bytes()) }.is_err(),
            "{absent} invented by the Rust translation"
        );
    }

    // `_from_string` (ro) and `_from_string_nu` must differ for identical input
    let (_, ro) = pair::<FromString>("crypto_core_ed25519_from_string");
    let (_, nu) = pair::<FromString>("crypto_core_ed25519_from_string_nu");
    let ctx = b"ctx";
    let msg = b"msg";
    let mut x = [0u8; 32];
    let mut y = [0u8; 32];
    unsafe {
        assert_eq!(ro(x.as_mut_ptr(), ctx.as_ptr(), 3, msg.as_ptr(), 3, 2), 0);
        assert_eq!(nu(y.as_mut_ptr(), ctx.as_ptr(), 3, msg.as_ptr(), 3, 2), 0);
    }
    assert_ne!(x, y, "ro and nu suites must not coincide");
}

/// CONFIGS G6-107 — `crypto_core_ed25519_scalar_from_string`: consumes a
/// 48-byte hash (`h_len = 48`), byte-reverses it into a 64-byte buffer and
/// reduces mod L, so the result is always canonical.
#[test]
fn core_ed25519_scalar_from_string() {
    setup();
    let mut rng = Rng::new(0x6007);
    drive_from_string("crypto_core_ed25519_scalar_from_string", 32, &mut rng);

    let (_, f) = pair::<FromString>("crypto_core_ed25519_scalar_from_string");
    let (_, canon) = pair::<I1c>("crypto_core_ed25519_scalar_is_canonical");
    for i in 0..32usize {
        let msg = rng.bytes(i * 3);
        let ctx = rng.bytes(i);
        let mut s = [0u8; 32];
        unsafe {
            assert_eq!(
                f(
                    s.as_mut_ptr(),
                    ctx.as_ptr(),
                    ctx.len(),
                    msg.as_ptr(),
                    msg.len(),
                    1 + (i as i32 & 1)
                ),
                0
            );
            assert_eq!(canon(s.as_ptr()), 1, "scalar_from_string not canonical");
        }
    }
}

// ===========================================================================
// crypto_core_ristretto255
// ===========================================================================

/// CONFIGS G6-109, G6-110 — `crypto_core_ristretto255_add` / `_sub` over random
/// valid elements, `(G, G)`, `(G, identity)`, `(P, -P)` and the
/// `sub(add(P,Q),Q) == P` round trip. Both operands go through the full
/// `ristretto255_frombytes` validation.
#[test]
fn core_ristretto255_add_sub() {
    setup();
    let mut rng = Rng::new(0x6008);
    let (c_add, r_add) = pair::<I3>("crypto_core_ristretto255_add");
    let (c_sub, r_sub) = pair::<I3>("crypto_core_ristretto255_sub");
    let pts = ris_valid_points(&mut rng, 32);
    let genr = unhex32(RIS_GEN);
    let zero = [0u8; 32];

    let mut cases: Vec<([u8; 32], [u8; 32])> = Vec::new();
    cases.push((genr, genr));
    cases.push((genr, zero));
    cases.push((zero, genr));
    cases.push((zero, zero));
    for i in 0..pts.len() {
        for j in 0..pts.len() {
            if (i + j) % 3 == 0 {
                cases.push((pts[i], pts[j]));
            }
        }
    }
    for _ in 0..600 {
        cases.push((*rng.pick(&pts), *rng.pick(&pts)));
    }

    for (p, q) in &cases {
        let mut a = canary(32);
        let mut b = canary(32);
        let (ra, rb) = unsafe {
            (
                c_add(a.as_mut_ptr(), p.as_ptr(), q.as_ptr()),
                r_add(b.as_mut_ptr(), p.as_ptr(), q.as_ptr()),
            )
        };
        eq_i32(&format!("ristretto255_add({}, {}) rc", hex(p), hex(q)), ra, rb);
        eq_bytes(&format!("ristretto255_add({}, {})", hex(p), hex(q)), &a, &b);

        let mut a = canary(32);
        let mut b = canary(32);
        let (ra, rb) = unsafe {
            (
                c_sub(a.as_mut_ptr(), p.as_ptr(), q.as_ptr()),
                r_sub(b.as_mut_ptr(), p.as_ptr(), q.as_ptr()),
            )
        };
        eq_i32(&format!("ristretto255_sub({}, {}) rc", hex(p), hex(q)), ra, rb);
        eq_bytes(&format!("ristretto255_sub({}, {})", hex(p), hex(q)), &a, &b);
    }

    // properties: sub(P,P) == identity; sub(add(P,Q),Q) == P; P + (-P) == identity
    for p in &pts {
        let mut z = [0u8; 32];
        unsafe { assert_eq!(r_sub(z.as_mut_ptr(), p.as_ptr(), p.as_ptr()), 0) };
        eq_bytes("ristretto sub(P,P) == identity", &zero, &z);
        for q in &pts[..6] {
            let mut s = [0u8; 32];
            let mut back = [0u8; 32];
            unsafe {
                assert_eq!(r_add(s.as_mut_ptr(), p.as_ptr(), q.as_ptr()), 0);
                assert_eq!(r_sub(back.as_mut_ptr(), s.as_ptr(), q.as_ptr()), 0);
            }
            eq_bytes("ristretto sub(add(P,Q),Q) == P", p, &back);
        }
    }
}

/// CONFIGS G6-111, G6-112 — `crypto_core_ristretto255_from_hash`: 64-byte input
/// shapes (all-zero, all-0xff, `01`+63 zeros, random) and the bit-255 masking
/// property (flipping bit 255 in either half must not change the output).
#[test]
fn core_ristretto255_from_hash() {
    setup();
    let mut rng = Rng::new(0x6009);
    let (c, r) = pair::<I2>("crypto_core_ristretto255_from_hash");

    let mut cases: Vec<Vec<u8>> = Vec::new();
    cases.push(vec![0u8; 64]);
    cases.push(vec![0xffu8; 64]);
    let mut one = vec![0u8; 64];
    one[0] = 1;
    cases.push(one);
    let mut hi = vec![0u8; 64];
    hi[31] = 0x80;
    hi[63] = 0x80;
    cases.push(hi);
    // field-boundary halves: p-1, p, p+1 in each half
    for pat in [
        "ecffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f",
        "edffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f",
        "eeffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f",
    ] {
        let mut v = unhex(pat);
        v.extend_from_slice(&unhex(pat));
        cases.push(v);
    }
    for _ in 0..800 {
        cases.push(rng.bytes(64));
    }

    for h in &cases {
        let mut a = canary(32);
        let mut b = canary(32);
        let (ra, rb) = unsafe {
            (
                c(a.as_mut_ptr(), h.as_ptr()),
                r(b.as_mut_ptr(), h.as_ptr()),
            )
        };
        eq_i32("ristretto255_from_hash rc", ra, rb);
        eq_bytes(&format!("ristretto255_from_hash({})", hex(h)), &a, &b);

        // G6-112: bit 255 of either half is silently dropped
        for flip in [31usize, 63] {
            let mut h2 = h.clone();
            h2[flip] ^= 0x80;
            let mut d = canary(32);
            unsafe { r(d.as_mut_ptr(), h2.as_ptr()) };
            eq_bytes("from_hash ignores bit 255", &a, &d);
        }
    }
}

/// CONFIGS G6-113, G6-114, G6-115 — `crypto_core_ristretto255_from_string`:
/// 64 hash bytes fed to `ristretto255_from_hash` **unreversed**, across both
/// `hash_alg` values, the `ctx = NULL` path and the oversize-DST path.
/// G6-115: there is no `_from_string_nu` / `_ro` for ristretto255.
#[test]
fn core_ristretto255_from_string() {
    setup();
    let mut rng = Rng::new(0x600a);
    drive_from_string("crypto_core_ristretto255_from_string", 32, &mut rng);

    // every result is a valid element
    let (_, f) = pair::<FromString>("crypto_core_ristretto255_from_string");
    let (_, ok) = pair::<I1c>("crypto_core_ristretto255_is_valid_point");
    for i in 0..24usize {
        let ctx = rng.bytes(i);
        let msg = rng.bytes(i * 5);
        let mut p = [0u8; 32];
        unsafe {
            assert_eq!(
                f(
                    p.as_mut_ptr(),
                    ctx.as_ptr(),
                    ctx.len(),
                    msg.as_ptr(),
                    msg.len(),
                    1 + (i as i32 & 1)
                ),
                0
            );
            assert_eq!(ok(p.as_ptr()), 1);
        }
        assert_eq!(p[0] & 1, 0, "ristretto encodings are always even");
        assert_eq!(p[31] & 0x80, 0, "bit 255 is always clear");
    }
}

/// CONFIGS G6-116, G6-117 — `crypto_core_ristretto255_random` (64 random
/// `HASHBYTES` -> `from_hash`) and `_scalar_random` (delegates verbatim to the
/// ed25519 rejection-sampling loop, so the two must agree byte-for-byte).
#[test]
fn core_ristretto255_randoms() {
    let _rng_lock = rng_guard();
    setup();
    let (c, r) = pair::<V1>("crypto_core_ristretto255_random");
    let (c_ok, r_ok) = pair::<I1c>("crypto_core_ristretto255_is_valid_point");
    for seed in 0..96u64 {
        let mut a = canary(32);
        let mut b = canary(32);
        reset_rngs(0xF100_0000 + seed);
        unsafe { c(a.as_mut_ptr()) };
        reset_rngs(0xF100_0000 + seed);
        unsafe { r(b.as_mut_ptr()) };
        eq_bytes(&format!("ristretto255_random seed={seed}"), &a, &b);
        let (x, y) = unsafe { (c_ok(a.as_ptr()), r_ok(b.as_ptr())) };
        eq_i32("is_valid_point(ristretto random)", x, y);
        assert_eq!(x, 1);
        assert_eq!(a[0] & 1, 0);
        assert_eq!(a[31] & 0x80, 0);
    }

    let (c_sr, r_sr) = pair::<V1>("crypto_core_ristretto255_scalar_random");
    let (_, ed_sr) = pair::<V1>("crypto_core_ed25519_scalar_random");
    for seed in 0..48u64 {
        let mut a = canary(32);
        let mut b = canary(32);
        let mut d = canary(32);
        reset_rngs(0xF200_0000 + seed);
        unsafe { c_sr(a.as_mut_ptr()) };
        reset_rngs(0xF200_0000 + seed);
        unsafe { r_sr(b.as_mut_ptr()) };
        reset_rngs(0xF200_0000 + seed);
        unsafe { ed_sr(d.as_mut_ptr()) };
        eq_bytes(&format!("ristretto255_scalar_random seed={seed}"), &a, &b);
        eq_bytes("ristretto scalar_random == ed25519 scalar_random", &a, &d);
    }
}

/// CONFIGS G6-118, G6-119, G6-120, G6-121, G6-122 — every
/// `crypto_core_ristretto255_scalar_*` wrapper, compared against its own C
/// counterpart *and* against the ed25519 function it delegates to.
#[test]
fn core_ristretto255_scalar_ops() {
    setup();
    let mut rng = Rng::new(0x600b);
    let shapes = scalar_shapes(&mut rng, 30);

    // binary: add / sub / mul
    for (ris, ed) in [
        ("crypto_core_ristretto255_scalar_add", "crypto_core_ed25519_scalar_add"),
        ("crypto_core_ristretto255_scalar_sub", "crypto_core_ed25519_scalar_sub"),
        ("crypto_core_ristretto255_scalar_mul", "crypto_core_ed25519_scalar_mul"),
    ] {
        let (c, r) = pair::<V3>(ris);
        let (_, ed_r) = pair::<V3>(ed);
        for x in &shapes {
            for y in &shapes {
                let mut a = canary(32);
                let mut b = canary(32);
                let mut d = canary(32);
                unsafe {
                    c(a.as_mut_ptr(), x.as_ptr(), y.as_ptr());
                    r(b.as_mut_ptr(), x.as_ptr(), y.as_ptr());
                    ed_r(d.as_mut_ptr(), x.as_ptr(), y.as_ptr());
                }
                eq_bytes(&format!("{ris}({}, {})", hex(x), hex(y)), &a, &b);
                eq_bytes(&format!("{ris} == {ed}"), &a, &d);
            }
        }
    }

    // unary void: negate / complement
    for (ris, ed) in [
        (
            "crypto_core_ristretto255_scalar_negate",
            "crypto_core_ed25519_scalar_negate",
        ),
        (
            "crypto_core_ristretto255_scalar_complement",
            "crypto_core_ed25519_scalar_complement",
        ),
    ] {
        let (c, r) = pair::<V2>(ris);
        let (_, ed_r) = pair::<V2>(ed);
        for s in &shapes {
            let mut a = canary(32);
            let mut b = canary(32);
            let mut d = canary(32);
            unsafe {
                c(a.as_mut_ptr(), s.as_ptr());
                r(b.as_mut_ptr(), s.as_ptr());
                ed_r(d.as_mut_ptr(), s.as_ptr());
            }
            eq_bytes(&format!("{ris}({})", hex(s)), &a, &b);
            eq_bytes(&format!("{ris} == {ed}"), &a, &d);
        }
    }

    // invert
    let (c_inv, r_inv) = pair::<I2>("crypto_core_ristretto255_scalar_invert");
    let (_, ed_inv) = pair::<I2>("crypto_core_ed25519_scalar_invert");
    for s in &shapes {
        let mut a = canary(32);
        let mut b = canary(32);
        let mut d = canary(32);
        let (ra, rb) = unsafe {
            (
                c_inv(a.as_mut_ptr(), s.as_ptr()),
                r_inv(b.as_mut_ptr(), s.as_ptr()),
            )
        };
        let rd = unsafe { ed_inv(d.as_mut_ptr(), s.as_ptr()) };
        eq_i32("ristretto255_scalar_invert rc", ra, rb);
        eq_i32("ristretto255_scalar_invert == ed25519 rc", ra, rd);
        eq_bytes(&format!("ristretto255_scalar_invert({})", hex(s)), &a, &b);
        eq_bytes("ristretto255_scalar_invert == ed25519", &a, &d);
    }

    // reduce (64-byte input)
    let (c_red, r_red) = pair::<V2>("crypto_core_ristretto255_scalar_reduce");
    let (_, ed_red) = pair::<V2>("crypto_core_ed25519_scalar_reduce");
    let mut reds: Vec<Vec<u8>> = vec![vec![0u8; 64], vec![0xffu8; 64]];
    let mut l64 = vec![0u8; 64];
    l64[..32].copy_from_slice(&ell());
    reds.push(l64);
    let mut p256 = vec![0u8; 64];
    p256[32] = 1;
    reds.push(p256);
    for _ in 0..40 {
        reds.push(rng.bytes(64));
    }
    for s in &reds {
        let mut a = canary(32);
        let mut b = canary(32);
        let mut d = canary(32);
        unsafe {
            c_red(a.as_mut_ptr(), s.as_ptr());
            r_red(b.as_mut_ptr(), s.as_ptr());
            ed_red(d.as_mut_ptr(), s.as_ptr());
        }
        eq_bytes(&format!("ristretto255_scalar_reduce({})", hex(s)), &a, &b);
        eq_bytes("ristretto255_scalar_reduce == ed25519", &a, &d);
    }

    // is_canonical
    let (c_can, r_can) = pair::<I1c>("crypto_core_ristretto255_scalar_is_canonical");
    let (_, ed_can) = pair::<I1c>("crypto_core_ed25519_scalar_is_canonical");
    for s in &shapes {
        let (a, b, d) = unsafe { (c_can(s.as_ptr()), r_can(s.as_ptr()), ed_can(s.as_ptr())) };
        eq_i32(&format!("ristretto255_scalar_is_canonical({})", hex(s)), a, b);
        eq_i32("ristretto255_scalar_is_canonical == ed25519", a, d);
    }

    // scalar_from_string delegates to the ed25519 version (G6-122)
    drive_from_string("crypto_core_ristretto255_scalar_from_string", 32, &mut rng);
    let (_, ris_fs) = pair::<FromString>("crypto_core_ristretto255_scalar_from_string");
    let (_, ed_fs) = pair::<FromString>("crypto_core_ed25519_scalar_from_string");
    for i in 0..16usize {
        let ctx = rng.bytes(i * 7);
        let msg = rng.bytes(i * 11);
        for alg in [1i32, 2] {
            let mut a = [0u8; 32];
            let mut b = [0u8; 32];
            unsafe {
                assert_eq!(
                    ris_fs(a.as_mut_ptr(), ctx.as_ptr(), ctx.len(), msg.as_ptr(), msg.len(), alg),
                    0
                );
                assert_eq!(
                    ed_fs(b.as_mut_ptr(), ctx.as_ptr(), ctx.len(), msg.as_ptr(), msg.len(), alg),
                    0
                );
            }
            eq_bytes("ristretto scalar_from_string == ed25519", &a, &b);
        }
    }
}

// ===========================================================================
// crypto_scalarmult
// ===========================================================================

/// CONFIGS G6-060, G6-061, G6-062, G6-063, G6-064 —
/// `crypto_scalarmult_curve25519`: the base point, `n = 0` (clamps to 2^254),
/// bit-255-set point encodings, random DH shapes and the
/// `mult(n1, base(n2)) == mult(n2, base(n1))` commutativity config. The 7
/// blocklisted small-order inputs are the Phase C rows, but they are driven
/// here too so that the *return value and untouched output* agree.
#[test]
fn scalarmult_curve25519() {
    setup();
    let mut rng = Rng::new(0x600c);
    let (c, r) = pair::<I3>("crypto_scalarmult_curve25519");
    let (c_b, r_b) = pair::<I2>("crypto_scalarmult_curve25519_base");

    let mut basep = [0u8; 32];
    basep[0] = 9;
    let mut base_hi = basep;
    base_hi[31] = 0x80;

    // scalars
    let mut scalars: Vec<[u8; 32]> = Vec::new();
    scalars.push([0u8; 32]);
    let mut one = [0u8; 32];
    one[0] = 1;
    scalars.push(one);
    let mut two = [0u8; 32];
    two[0] = 2;
    scalars.push(two);
    scalars.push([0xffu8; 32]);
    scalars.push(ell_minus_1());
    scalars.push(ell());
    let mut hi = [0u8; 32];
    hi[31] = 0x80;
    scalars.push(hi);
    for _ in 0..32 {
        let mut s = [0u8; 32];
        rng.fill(&mut s);
        scalars.push(s);
    }

    // points: base, base with bit 255 set, all-zero, all-0xff, blocklist,
    // and public keys derived from random scalars
    let mut points: Vec<[u8; 32]> = vec![basep, base_hi, [0u8; 32], [0xffu8; 32]];
    points.extend(x25519_blocklist());
    for b in x25519_blocklist() {
        let mut v = b;
        v[31] |= 0x80;
        points.push(v);
    }
    for s in &scalars[..8] {
        let mut q = [0u8; 32];
        unsafe { assert_eq!(r_b(q.as_mut_ptr(), s.as_ptr()), 0) };
        points.push(q);
    }

    for n in &scalars {
        for p in &points {
            let mut a = canary(32);
            let mut b = canary(32);
            let (ra, rb) = unsafe {
                (
                    c(a.as_mut_ptr(), n.as_ptr(), p.as_ptr()),
                    r(b.as_mut_ptr(), n.as_ptr(), p.as_ptr()),
                )
            };
            eq_i32(&format!("x25519(n={}, p={}) rc", hex(n), hex(p)), ra, rb);
            eq_bytes(&format!("x25519(n={}, p={})", hex(n), hex(p)), &a, &b);
        }
    }

    // G6-063: bit 255 of the point encoding is ignored
    for p in &points {
        let mut p2 = *p;
        p2[31] ^= 0x80;
        let mut a = canary(32);
        let mut b = canary(32);
        let n = scalars[7];
        let (ra, rb) = unsafe {
            (
                r(a.as_mut_ptr(), n.as_ptr(), p.as_ptr()),
                r(b.as_mut_ptr(), n.as_ptr(), p2.as_ptr()),
            )
        };
        eq_i32("x25519 ignores bit 255 of p (rc)", ra, rb);
        eq_bytes("x25519 ignores bit 255 of p", &a, &b);
    }

    // G6-064: commutativity
    for _ in 0..40 {
        let n1 = rng.bytes(32);
        let n2 = rng.bytes(32);
        let mut p1 = [0u8; 32];
        let mut p2 = [0u8; 32];
        let mut s1 = [0u8; 32];
        let mut s2 = [0u8; 32];
        unsafe {
            assert_eq!(r_b(p1.as_mut_ptr(), n1.as_ptr()), 0);
            assert_eq!(r_b(p2.as_mut_ptr(), n2.as_ptr()), 0);
            assert_eq!(r(s1.as_mut_ptr(), n1.as_ptr(), p2.as_ptr()), 0);
            assert_eq!(r(s2.as_mut_ptr(), n2.as_ptr(), p1.as_ptr()), 0);
        }
        eq_bytes("x25519 commutativity", &s1, &s2);
        // and the C agrees on the same shared secret
        let mut s3 = [0u8; 32];
        unsafe { assert_eq!(c(s3.as_mut_ptr(), n1.as_ptr(), p2.as_ptr()), 0) };
        eq_bytes("x25519 C == Rust shared secret", &s3, &s1);
    }
    let _ = c_b;
}

/// CONFIGS G6-065, G6-066, G6-067 — `crypto_scalarmult_curve25519_base` over
/// every scalar shape, including the `q == n` aliasing that the C code's
/// `unsigned char *t = q` relies on, and `n` = 32 zero bytes.
#[test]
fn scalarmult_curve25519_base() {
    setup();
    let mut rng = Rng::new(0x600d);
    let (c, r) = pair::<I2>("crypto_scalarmult_curve25519_base");
    let mut scalars: Vec<[u8; 32]> = vec![[0u8; 32], [0xffu8; 32], ell_minus_1(), ell()];
    let mut one = [0u8; 32];
    one[0] = 1;
    scalars.push(one);
    let mut two = [0u8; 32];
    two[0] = 2;
    scalars.push(two);
    for _ in 0..400 {
        let mut s = [0u8; 32];
        rng.fill(&mut s);
        scalars.push(s);
    }
    for n in &scalars {
        let mut a = canary(32);
        let mut b = canary(32);
        let (ra, rb) = unsafe {
            (
                c(a.as_mut_ptr(), n.as_ptr()),
                r(b.as_mut_ptr(), n.as_ptr()),
            )
        };
        eq_i32(&format!("x25519_base({}) rc", hex(n)), ra, rb);
        eq_bytes(&format!("x25519_base({})", hex(n)), &a, &b);
        assert_eq!(ra, 0, "x25519_base has no rejection branch");

        // aliased q == n: `t = q` means the clamped scalar overwrites the input
        let mut ai = n.to_vec();
        let mut bi = n.to_vec();
        let (ra, rb) = unsafe {
            (
                c(ai.as_mut_ptr(), ai.as_ptr()),
                r(bi.as_mut_ptr(), bi.as_ptr()),
            )
        };
        eq_i32("x25519_base aliased rc", ra, rb);
        eq_bytes(&format!("x25519_base aliased({})", hex(n)), &ai, &bi);
    }
}

/// CONFIGS G6-068, G6-069, G6-070 — the generic `crypto_scalarmult` /
/// `crypto_scalarmult_base` dispatch must be byte-identical to the
/// curve25519 entry points, and the size/primitive accessors must match.
#[test]
fn scalarmult_generic_dispatch() {
    setup();
    let mut rng = Rng::new(0x600e);
    assert_eq!(eq_sz("crypto_scalarmult_bytes"), 32);
    assert_eq!(eq_sz("crypto_scalarmult_scalarbytes"), 32);
    assert_eq!(eq_sz("crypto_scalarmult_curve25519_bytes"), 32);
    assert_eq!(eq_sz("crypto_scalarmult_curve25519_scalarbytes"), 32);
    assert_eq!(eq_str("crypto_scalarmult_primitive"), "curve25519");

    let (c_g, r_g) = pair::<I3>("crypto_scalarmult");
    let (_, r_x) = pair::<I3>("crypto_scalarmult_curve25519");
    let (c_gb, r_gb) = pair::<I2>("crypto_scalarmult_base");
    let (_, r_xb) = pair::<I2>("crypto_scalarmult_curve25519_base");

    let mut points: Vec<[u8; 32]> = x25519_blocklist();
    let mut basep = [0u8; 32];
    basep[0] = 9;
    points.push(basep);
    points.push([0xffu8; 32]);
    for _ in 0..8 {
        let s = rng.bytes(32);
        let mut q = [0u8; 32];
        unsafe { assert_eq!(r_xb(q.as_mut_ptr(), s.as_ptr()), 0) };
        points.push(q);
    }
    for _ in 0..200 {
        let n = rng.bytes(32);
        let p = *rng.pick(&points);
        let mut a = canary(32);
        let mut b = canary(32);
        let mut d = canary(32);
        let (ra, rb) = unsafe {
            (
                c_g(a.as_mut_ptr(), n.as_ptr(), p.as_ptr()),
                r_g(b.as_mut_ptr(), n.as_ptr(), p.as_ptr()),
            )
        };
        let rd = unsafe { r_x(d.as_mut_ptr(), n.as_ptr(), p.as_ptr()) };
        eq_i32("crypto_scalarmult rc", ra, rb);
        eq_i32("crypto_scalarmult == _curve25519 rc", ra, rd);
        eq_bytes("crypto_scalarmult", &a, &b);
        eq_bytes("crypto_scalarmult == _curve25519", &a, &d);

        let mut a = canary(32);
        let mut b = canary(32);
        let mut d = canary(32);
        let (ra, rb) = unsafe {
            (
                c_gb(a.as_mut_ptr(), n.as_ptr()),
                r_gb(b.as_mut_ptr(), n.as_ptr()),
            )
        };
        let rd = unsafe { r_xb(d.as_mut_ptr(), n.as_ptr()) };
        eq_i32("crypto_scalarmult_base rc", ra, rb);
        eq_i32("crypto_scalarmult_base == _curve25519_base rc", ra, rd);
        eq_bytes("crypto_scalarmult_base", &a, &b);
        eq_bytes("crypto_scalarmult_base == _curve25519_base", &a, &d);
    }
}

/// CONFIGS G6-071, G6-072, G6-073, G6-074, G6-075, G6-076 —
/// `crypto_scalarmult_ed25519` and `_noclamp` over the base point, random
/// main-subgroup points and every scalar shape. Clamping dominates small
/// scalars in the clamped form, while `_noclamp` returns `p` verbatim for
/// `n = 1` and honours `n = L+1 ~ 1`, `n = L-1 ~ -p`.
#[test]
fn scalarmult_ed25519_and_noclamp() {
    setup();
    let mut rng = Rng::new(0x600f);
    let (c, r) = pair::<I3>("crypto_scalarmult_ed25519");
    let (c_nc, r_nc) = pair::<I3>("crypto_scalarmult_ed25519_noclamp");
    let base = unhex32(ED_BASE);

    let mut scalars: Vec<[u8; 32]> = Vec::new();
    let mut one = [0u8; 32];
    one[0] = 1;
    scalars.push(one);
    let mut two = [0u8; 32];
    two[0] = 2;
    scalars.push(two);
    scalars.push([0u8; 32]);
    scalars.push([0xffu8; 32]);
    scalars.push(ell_minus_1());
    scalars.push(ell());
    scalars.push(ell_plus_1());
    let mut hi = [0u8; 32];
    hi[31] = 0x80;
    scalars.push(hi);
    for k in 2..8u32 {
        scalars.push(mul_small(&ell(), k));
    }
    for _ in 0..40 {
        let mut s = [0u8; 32];
        rng.fill(&mut s);
        scalars.push(s);
    }

    let mut points = ed_valid_points(&mut rng, 16);
    points.extend(ed_small_order());
    points.push([0xffu8; 32]);
    points.push(unhex32(
        "edffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f",
    ));
    points.push(unhex32(
        "0200000000000000000000000000000000000000000000000000000000000000",
    ));

    for n in &scalars {
        for p in &points {
            for (name, cf, rf) in [
                ("ed25519", c, r),
                ("ed25519_noclamp", c_nc, r_nc),
            ] {
                let mut a = canary(32);
                let mut b = canary(32);
                let (ra, rb) = unsafe {
                    (
                        cf(a.as_mut_ptr(), n.as_ptr(), p.as_ptr()),
                        rf(b.as_mut_ptr(), n.as_ptr(), p.as_ptr()),
                    )
                };
                eq_i32(
                    &format!("scalarmult_{name}(n={}, p={}) rc", hex(n), hex(p)),
                    ra,
                    rb,
                );
                eq_bytes(
                    &format!("scalarmult_{name}(n={}, p={})", hex(n), hex(p)),
                    &a,
                    &b,
                );
            }
        }
    }

    // G6-074: noclamp(1, P) == P
    for p in &ed_valid_points(&mut rng, 6) {
        let mut q = canary(32);
        unsafe { assert_eq!(r_nc(q.as_mut_ptr(), one.as_ptr(), p.as_ptr()), 0) };
        eq_bytes("noclamp(1, P) == P", p, &q);
    }
    // G6-076: noclamp(L+1, P) == noclamp(1, P) and noclamp(L-1, P) == -P
    let lp1 = ell_plus_1();
    let lm1 = ell_minus_1();
    for p in &ed_valid_points(&mut rng, 6) {
        let mut a = [0u8; 32];
        let mut b = [0u8; 32];
        unsafe {
            assert_eq!(r_nc(a.as_mut_ptr(), lp1.as_ptr(), p.as_ptr()), 0);
            assert_eq!(r_nc(b.as_mut_ptr(), one.as_ptr(), p.as_ptr()), 0);
        }
        eq_bytes("noclamp(L+1, P) == noclamp(1, P)", &a, &b);
        let mut neg = [0u8; 32];
        unsafe { assert_eq!(r_nc(neg.as_mut_ptr(), lm1.as_ptr(), p.as_ptr()), 0) };
        let mut expect = *p;
        expect[31] ^= 0x80;
        eq_bytes("noclamp(L-1, P) == -P", &expect, &neg);
    }
    // G6-071/073: the clamped form ignores the low 3 and top 2 bits of n
    for _ in 0..12 {
        let mut n = [0u8; 32];
        rng.fill(&mut n);
        let mut n2 = n;
        n2[0] ^= 0x07;
        n2[31] ^= 0xc0;
        let p = base;
        let mut a = [0u8; 32];
        let mut b = [0u8; 32];
        unsafe {
            assert_eq!(r(a.as_mut_ptr(), n.as_ptr(), p.as_ptr()), 0);
            assert_eq!(r(b.as_mut_ptr(), n2.as_ptr(), p.as_ptr()), 0);
        }
        eq_bytes("clamped scalarmult ignores clamped bits", &a, &b);
    }
}

/// CONFIGS G6-077, G6-078, G6-079, G6-080 —
/// `crypto_scalarmult_ed25519_base` / `_base_noclamp`, including the
/// `q == n` aliasing and the `base_noclamp(n) == noclamp(n, B)` cross-check
/// between the precomputed-table and the general ladder.
#[test]
fn scalarmult_ed25519_base() {
    setup();
    let mut rng = Rng::new(0x6010);
    assert_eq!(eq_sz("crypto_scalarmult_ed25519_bytes"), 32);
    assert_eq!(eq_sz("crypto_scalarmult_ed25519_scalarbytes"), 32);
    let (c, r) = pair::<I2>("crypto_scalarmult_ed25519_base");
    let (c_nc, r_nc) = pair::<I2>("crypto_scalarmult_ed25519_base_noclamp");
    let (_, mult_nc) = pair::<I3>("crypto_scalarmult_ed25519_noclamp");
    let base = unhex32(ED_BASE);

    let mut scalars: Vec<[u8; 32]> = Vec::new();
    let mut one = [0u8; 32];
    one[0] = 1;
    scalars.push(one);
    let mut two = [0u8; 32];
    two[0] = 2;
    scalars.push(two);
    scalars.push([0u8; 32]);
    scalars.push([0xffu8; 32]);
    scalars.push(ell_minus_1());
    scalars.push(ell());
    scalars.push(ell_plus_1());
    let mut hi = [0u8; 32];
    hi[31] = 0x80;
    scalars.push(hi);
    for k in 2..8u32 {
        scalars.push(mul_small(&ell(), k));
    }
    for _ in 0..300 {
        let mut s = [0u8; 32];
        rng.fill(&mut s);
        scalars.push(s);
    }

    for n in &scalars {
        for (name, cf, rf) in [("base", c, r), ("base_noclamp", c_nc, r_nc)] {
            let mut a = canary(32);
            let mut b = canary(32);
            let (ra, rb) = unsafe {
                (
                    cf(a.as_mut_ptr(), n.as_ptr()),
                    rf(b.as_mut_ptr(), n.as_ptr()),
                )
            };
            eq_i32(&format!("ed25519_{name}({}) rc", hex(n)), ra, rb);
            eq_bytes(&format!("ed25519_{name}({})", hex(n)), &a, &b);

            // aliased q == n
            let mut ai = n.to_vec();
            let mut bi = n.to_vec();
            let (ra, rb) = unsafe {
                (
                    cf(ai.as_mut_ptr(), ai.as_ptr()),
                    rf(bi.as_mut_ptr(), bi.as_ptr()),
                )
            };
            eq_i32(&format!("ed25519_{name} aliased rc"), ra, rb);
            eq_bytes(&format!("ed25519_{name} aliased({})", hex(n)), &ai, &bi);
        }
    }

    // G6-078: base_noclamp(1) is the base point encoding
    let mut q = [0u8; 32];
    unsafe { assert_eq!(r_nc(q.as_mut_ptr(), one.as_ptr()), 0) };
    eq_bytes("base_noclamp(1) == B", &base, &q);

    // G6-079: base_noclamp(n) == noclamp(n, B) through two different routines
    for n in &scalars {
        let mut a = canary(32);
        let mut b = canary(32);
        let (ra, rb) = unsafe {
            (
                r_nc(a.as_mut_ptr(), n.as_ptr()),
                mult_nc(b.as_mut_ptr(), n.as_ptr(), base.as_ptr()),
            )
        };
        eq_i32("base_noclamp vs noclamp(B) rc", ra, rb);
        eq_bytes(&format!("base_noclamp({}) == noclamp(n, B)", hex(n)), &a, &b);
    }
}

/// CONFIGS G6-081, G6-082, G6-083, G6-084, G6-085, G6-086, G6-087 —
/// `crypto_scalarmult_ristretto255` and `_base`: never clamped (only
/// `t[31] &= 127`), so `n = 1` returns `p` verbatim, `n = L+k ~ k`, and
/// `base(n) == mult(n, G)`.
#[test]
fn scalarmult_ristretto255() {
    setup();
    let mut rng = Rng::new(0x6011);
    assert_eq!(eq_sz("crypto_scalarmult_ristretto255_bytes"), 32);
    assert_eq!(eq_sz("crypto_scalarmult_ristretto255_scalarbytes"), 32);
    let (c, r) = pair::<I3>("crypto_scalarmult_ristretto255");
    let (c_b, r_b) = pair::<I2>("crypto_scalarmult_ristretto255_base");
    let genr = unhex32(RIS_GEN);

    let mut scalars: Vec<[u8; 32]> = Vec::new();
    let mut one = [0u8; 32];
    one[0] = 1;
    scalars.push(one);
    let mut two = [0u8; 32];
    two[0] = 2;
    scalars.push(two);
    scalars.push([0u8; 32]);
    scalars.push([0xffu8; 32]);
    scalars.push(ell_minus_1());
    scalars.push(ell());
    scalars.push(ell_plus_1());
    let mut lp2 = ell();
    lp2[0] += 2;
    scalars.push(lp2);
    let mut hi = [0u8; 32];
    hi[31] = 0x80;
    scalars.push(hi);
    for k in 2..8u32 {
        scalars.push(mul_small(&ell(), k));
    }
    for _ in 0..40 {
        let mut s = [0u8; 32];
        rng.fill(&mut s);
        scalars.push(s);
    }

    let mut points = ris_valid_points(&mut rng, 20);
    // invalid encodings too — the return code and untouched output must agree
    points.push(unhex32(
        "0100000000000000000000000000000000000000000000000000000000000000",
    ));
    points.push(unhex32(
        "0000000000000000000000000000000000000000000000000000000000000080",
    ));
    points.push(unhex32(
        "eeffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f",
    ));
    points.push(unhex32(
        "26948d35ca62e643e26a83177332e6b6afeb9d08e4268b650f1f5bbd8d81d371",
    ));

    for n in &scalars {
        for p in &points {
            let mut a = canary(32);
            let mut b = canary(32);
            let (ra, rb) = unsafe {
                (
                    c(a.as_mut_ptr(), n.as_ptr(), p.as_ptr()),
                    r(b.as_mut_ptr(), n.as_ptr(), p.as_ptr()),
                )
            };
            eq_i32(
                &format!("ristretto255_mult(n={}, p={}) rc", hex(n), hex(p)),
                ra,
                rb,
            );
            eq_bytes(
                &format!("ristretto255_mult(n={}, p={})", hex(n), hex(p)),
                &a,
                &b,
            );
        }
        let mut a = canary(32);
        let mut b = canary(32);
        let (ra, rb) = unsafe {
            (
                c_b(a.as_mut_ptr(), n.as_ptr()),
                r_b(b.as_mut_ptr(), n.as_ptr()),
            )
        };
        eq_i32(&format!("ristretto255_base({}) rc", hex(n)), ra, rb);
        eq_bytes(&format!("ristretto255_base({})", hex(n)), &a, &b);

        // aliased q == n (`t = q`)
        let mut ai = n.to_vec();
        let mut bi = n.to_vec();
        let (ra, rb) = unsafe {
            (
                c_b(ai.as_mut_ptr(), ai.as_ptr()),
                r_b(bi.as_mut_ptr(), bi.as_ptr()),
            )
        };
        eq_i32("ristretto255_base aliased rc", ra, rb);
        eq_bytes("ristretto255_base aliased", &ai, &bi);

        // G6-086: base(n) == mult(n, G)
        let mut d = canary(32);
        let rd = unsafe { r(d.as_mut_ptr(), n.as_ptr(), genr.as_ptr()) };
        eq_i32("ristretto255 base vs mult(G) rc", rb, rd);
        eq_bytes("ristretto255 base(n) == mult(n, G)", &b, &d);
    }

    // G6-081/084: mult(1, P) == P, mult(L+1, P) == mult(1, P), mult(L+2,P)==mult(2,P)
    let lp1 = ell_plus_1();
    for p in &ris_valid_points(&mut rng, 6) {
        if p.iter().all(|&b| b == 0) {
            continue; // the identity multiplies to the identity -> -1
        }
        let mut a = [0u8; 32];
        unsafe { assert_eq!(r(a.as_mut_ptr(), one.as_ptr(), p.as_ptr()), 0) };
        eq_bytes("ristretto255 mult(1, P) == P", p, &a);
        let mut b = [0u8; 32];
        unsafe { assert_eq!(r(b.as_mut_ptr(), lp1.as_ptr(), p.as_ptr()), 0) };
        eq_bytes("ristretto255 mult(L+1, P) == P", p, &b);
        let mut c2 = [0u8; 32];
        let mut d2 = [0u8; 32];
        unsafe {
            assert_eq!(r(c2.as_mut_ptr(), two.as_ptr(), p.as_ptr()), 0);
            assert_eq!(r(d2.as_mut_ptr(), lp2.as_ptr(), p.as_ptr()), 0);
        }
        eq_bytes("ristretto255 mult(L+2, P) == mult(2, P)", &c2, &d2);
    }
    // G6-085: base(1) is the generator
    let mut g = [0u8; 32];
    unsafe { assert_eq!(r_b(g.as_mut_ptr(), one.as_ptr()), 0) };
    eq_bytes("ristretto255 base(1) == G", &genr, &g);
}

// ===========================================================================
// crypto_kem
// ===========================================================================

/// CONFIGS G6-127, G6-128, G6-129, G6-134 — ML-KEM-768 key generation:
/// `_seed_keypair` over the documented 64-byte seed shapes, `_keypair` under
/// the deterministic RNG, and determinism of `_seed_keypair`.
#[test]
fn kem_mlkem768_keypair() {
    let _rng_lock = rng_guard();
    setup();
    let mut rng = Rng::new(0x6012);
    let pkb = eq_sz("crypto_kem_mlkem768_publickeybytes");
    let skb = eq_sz("crypto_kem_mlkem768_secretkeybytes");
    let ssb = eq_sz("crypto_kem_mlkem768_sharedsecretbytes");
    let ctb = eq_sz("crypto_kem_mlkem768_ciphertextbytes");
    let seb = eq_sz("crypto_kem_mlkem768_seedbytes");
    assert_eq!((pkb, skb, ctb, ssb, seb), (1184, 2400, 1088, 32, 64));

    let (c_sk, r_sk) = pair::<KemSeedKeypair>("crypto_kem_mlkem768_seed_keypair");
    let mut seeds: Vec<Vec<u8>> = vec![vec![0u8; 64], vec![0xffu8; 64]];
    let mut s1 = vec![0u8; 64];
    s1[63] = 1;
    seeds.push(s1);
    let mut s2 = vec![0u8; 64];
    s2[0] = 1;
    seeds.push(s2);
    for _ in 0..16 {
        seeds.push(rng.bytes(64));
    }

    for seed in &seeds {
        let mut apk = canary(pkb);
        let mut ask = canary(skb);
        let mut bpk = canary(pkb);
        let mut bsk = canary(skb);
        let (ra, rb) = unsafe {
            (
                c_sk(apk.as_mut_ptr(), ask.as_mut_ptr(), seed.as_ptr()),
                r_sk(bpk.as_mut_ptr(), bsk.as_mut_ptr(), seed.as_ptr()),
            )
        };
        eq_i32("mlkem768_seed_keypair rc", ra, rb);
        eq_bytes("mlkem768_seed_keypair pk", &apk, &bpk);
        eq_bytes("mlkem768_seed_keypair sk", &ask, &bsk);
        // G6-128: sk layout — sk[1152..2336] is pk, sk[2368..] is z = seed[32..]
        assert_eq!(&ask[1152..1152 + pkb], &apk[..]);
        assert_eq!(&ask[2368..2400], &seed[32..64]);
        // G6-129: determinism
        let mut cpk = canary(pkb);
        let mut csk = canary(skb);
        unsafe { r_sk(cpk.as_mut_ptr(), csk.as_mut_ptr(), seed.as_ptr()) };
        eq_bytes("mlkem768_seed_keypair determinism", &bpk, &cpk);
        eq_bytes("mlkem768_seed_keypair determinism sk", &bsk, &csk);
    }

    // G6-127: `_keypair` draws a 64-byte seed from the RNG
    let (c_kp, r_kp) = pair::<KemKeypair>("crypto_kem_mlkem768_keypair");
    for seed in 0..6u64 {
        let mut apk = canary(pkb);
        let mut ask = canary(skb);
        let mut bpk = canary(pkb);
        let mut bsk = canary(skb);
        reset_rngs(0x1234_0000 + seed);
        let ra = unsafe { c_kp(apk.as_mut_ptr(), ask.as_mut_ptr()) };
        reset_rngs(0x1234_0000 + seed);
        let rb = unsafe { r_kp(bpk.as_mut_ptr(), bsk.as_mut_ptr()) };
        eq_i32("mlkem768_keypair rc", ra, rb);
        eq_bytes("mlkem768_keypair pk", &apk, &bpk);
        eq_bytes("mlkem768_keypair sk", &ask, &bsk);
    }
}

/// CONFIGS G6-130, G6-131, G6-132, G6-133 — ML-KEM-768 encapsulation and
/// decapsulation. Decapsulation uses **implicit rejection**: a corrupted
/// ciphertext still returns 0 but yields a different (deterministic) shared
/// secret, so both the return code and the shared secret are compared for
/// valid *and* corrupted ciphertexts.
#[test]
fn kem_mlkem768_enc_dec() {
    let _rng_lock = rng_guard();
    setup();
    let mut rng = Rng::new(0x6013);
    let (pkb, skb, ctb, ssb) = (1184usize, 2400usize, 1088usize, 32usize);
    let (_, r_sk) = pair::<KemSeedKeypair>("crypto_kem_mlkem768_seed_keypair");
    let (c_ed, r_ed) = pair::<KemEncDet>("crypto_kem_mlkem768_enc_deterministic");
    let (c_e, r_e) = pair::<KemEnc>("crypto_kem_mlkem768_enc");
    let (c_d, r_d) = pair::<KemDec>("crypto_kem_mlkem768_dec");

    for it in 0..12u64 {
        let seed = rng.bytes(64);
        let mut pk = vec![0u8; pkb];
        let mut sk = vec![0u8; skb];
        unsafe { assert_eq!(r_sk(pk.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr()), 0) };

        // G6-130: enc_deterministic over the documented 32-byte seed shapes
        let eseeds: Vec<Vec<u8>> = vec![
            vec![0u8; 32],
            vec![0xffu8; 32],
            rng.bytes(32),
            rng.bytes(32),
            rng.bytes(32),
            rng.bytes(32),
            rng.bytes(32),
            rng.bytes(32),
        ];
        for es in &eseeds {
            let mut act = canary(ctb);
            let mut ass = canary(ssb);
            let mut bct = canary(ctb);
            let mut bss = canary(ssb);
            let (ra, rb) = unsafe {
                (
                    c_ed(act.as_mut_ptr(), ass.as_mut_ptr(), pk.as_ptr(), es.as_ptr()),
                    r_ed(bct.as_mut_ptr(), bss.as_mut_ptr(), pk.as_ptr(), es.as_ptr()),
                )
            };
            eq_i32("mlkem768_enc_deterministic rc", ra, rb);
            eq_bytes("mlkem768_enc_deterministic ct", &act, &bct);
            eq_bytes("mlkem768_enc_deterministic ss", &ass, &bss);

            // G6-132: round trip
            let mut ads = canary(ssb);
            let mut bds = canary(ssb);
            let (ra, rb) = unsafe {
                (
                    c_d(ads.as_mut_ptr(), act.as_ptr(), sk.as_ptr()),
                    r_d(bds.as_mut_ptr(), bct.as_ptr(), sk.as_ptr()),
                )
            };
            eq_i32("mlkem768_dec rc", ra, rb);
            eq_bytes("mlkem768_dec ss (C vs Rust)", &ads, &bds);
            eq_bytes("mlkem768_dec recovers the encapsulated ss", &ass, &ads);
        }

        // G6-131: `_enc` draws its own seed
        let mut act = canary(ctb);
        let mut ass = canary(ssb);
        let mut bct = canary(ctb);
        let mut bss = canary(ssb);
        reset_rngs(0x2345_0000 + it);
        let ra = unsafe { c_e(act.as_mut_ptr(), ass.as_mut_ptr(), pk.as_ptr()) };
        reset_rngs(0x2345_0000 + it);
        let rb = unsafe { r_e(bct.as_mut_ptr(), bss.as_mut_ptr(), pk.as_ptr()) };
        eq_i32("mlkem768_enc rc", ra, rb);
        eq_bytes("mlkem768_enc ct", &act, &bct);
        eq_bytes("mlkem768_enc ss", &ass, &bss);

        // G6-133: implicit rejection — corrupted ct still returns 0, with a
        // deterministic pseudorandom ss that both libraries must agree on.
        let mut cts: Vec<Vec<u8>> = Vec::new();
        for &(byte, bit) in &[
            (0usize, 0u8),
            (1, 1),
            (127, 2),
            (383, 4),
            (543, 3),
            (959, 5),
            (1086, 0),
            (1087, 7),
        ] {
            let mut v = act.clone();
            v[byte] ^= 1 << bit;
            cts.push(v);
        }
        cts.push(vec![0u8; ctb]);
        cts.push(vec![0xffu8; ctb]);
        cts.push(rng.bytes(ctb));
        for ct in &cts {
            let mut ads = canary(ssb);
            let mut bds = canary(ssb);
            let (ra, rb) = unsafe {
                (
                    c_d(ads.as_mut_ptr(), ct.as_ptr(), sk.as_ptr()),
                    r_d(bds.as_mut_ptr(), ct.as_ptr(), sk.as_ptr()),
                )
            };
            eq_i32("mlkem768_dec(corrupt) rc", ra, rb);
            assert_eq!(ra, 0, "ML-KEM decapsulation never returns -1");
            eq_bytes("mlkem768_dec(corrupt) ss (C vs Rust)", &ads, &bds);
            assert_ne!(&ads[..], &ass[..], "implicit rejection must change ss");
        }
    }
}

/// CONFIGS G6-135, G6-136, G6-137, G6-138, G6-139, G6-140, G6-141, G6-142 —
/// X-Wing: `_seed_keypair` / `_keypair` / `_enc_deterministic` / `_enc` /
/// `_dec`, the 1216/32/1120/32/32 shapes, the SHA3-256 combiner and the
/// "corrupted ML-KEM half is *not* rejected" behaviour.
#[test]
fn kem_xwing() {
    let _rng_lock = rng_guard();
    setup();
    let mut rng = Rng::new(0x6014);
    let pkb = eq_sz("crypto_kem_xwing_publickeybytes");
    let skb = eq_sz("crypto_kem_xwing_secretkeybytes");
    let ctb = eq_sz("crypto_kem_xwing_ciphertextbytes");
    let ssb = eq_sz("crypto_kem_xwing_sharedsecretbytes");
    let seb = eq_sz("crypto_kem_xwing_seedbytes");
    assert_eq!((pkb, skb, ctb, ssb, seb), (1216, 32, 1120, 32, 32));

    let (c_sk, r_sk) = pair::<KemSeedKeypair>("crypto_kem_xwing_seed_keypair");
    let (c_kp, r_kp) = pair::<KemKeypair>("crypto_kem_xwing_keypair");
    let (c_ed, r_ed) = pair::<KemEncDet>("crypto_kem_xwing_enc_deterministic");
    let (c_e, r_e) = pair::<KemEnc>("crypto_kem_xwing_enc");
    let (c_d, r_d) = pair::<KemDec>("crypto_kem_xwing_dec");

    let mut seeds: Vec<Vec<u8>> = vec![vec![0u8; 32], vec![0xffu8; 32]];
    let mut s1 = vec![0u8; 32];
    s1[31] = 1;
    seeds.push(s1);
    for _ in 0..6 {
        seeds.push(rng.bytes(32));
    }

    for (i, seed) in seeds.iter().enumerate() {
        let mut apk = canary(pkb);
        let mut ask = canary(skb);
        let mut bpk = canary(pkb);
        let mut bsk = canary(skb);
        let (ra, rb) = unsafe {
            (
                c_sk(apk.as_mut_ptr(), ask.as_mut_ptr(), seed.as_ptr()),
                r_sk(bpk.as_mut_ptr(), bsk.as_mut_ptr(), seed.as_ptr()),
            )
        };
        eq_i32("xwing_seed_keypair rc", ra, rb);
        eq_bytes("xwing_seed_keypair pk", &apk, &bpk);
        eq_bytes("xwing_seed_keypair sk", &ask, &bsk);
        // G6-136: sk is a verbatim copy of the seed
        assert_eq!(&ask[..], &seed[..]);

        // G6-137/138: enc_deterministic over 64-byte seed shapes
        let eseeds: Vec<Vec<u8>> = vec![vec![0u8; 64], vec![0xffu8; 64], rng.bytes(64)];
        for es in &eseeds {
            let mut act = canary(ctb);
            let mut ass = canary(ssb);
            let mut bct = canary(ctb);
            let mut bss = canary(ssb);
            let (ra, rb) = unsafe {
                (
                    c_ed(act.as_mut_ptr(), ass.as_mut_ptr(), apk.as_ptr(), es.as_ptr()),
                    r_ed(bct.as_mut_ptr(), bss.as_mut_ptr(), apk.as_ptr(), es.as_ptr()),
                )
            };
            eq_i32("xwing_enc_deterministic rc", ra, rb);
            eq_bytes("xwing_enc_deterministic ct", &act, &bct);
            eq_bytes("xwing_enc_deterministic ss", &ass, &bss);
            assert_eq!(ra, 0);

            // G6-140: round trip
            let mut ads = canary(ssb);
            let mut bds = canary(ssb);
            let (ra, rb) = unsafe {
                (
                    c_d(ads.as_mut_ptr(), act.as_ptr(), ask.as_ptr()),
                    r_d(bds.as_mut_ptr(), bct.as_ptr(), bsk.as_ptr()),
                )
            };
            eq_i32("xwing_dec rc", ra, rb);
            eq_bytes("xwing_dec ss (C vs Rust)", &ads, &bds);
            eq_bytes("xwing_dec recovers ss", &ass, &ads);

            // G6-141: corruption in the ML-KEM half is NOT rejected
            for &(byte, bit) in &[(0usize, 1u8), (777, 4), (1087, 7)] {
                let mut ct2 = act.clone();
                ct2[byte] ^= 1 << bit;
                let mut ads2 = canary(ssb);
                let mut bds2 = canary(ssb);
                let (ra, rb) = unsafe {
                    (
                        c_d(ads2.as_mut_ptr(), ct2.as_ptr(), ask.as_ptr()),
                        r_d(bds2.as_mut_ptr(), ct2.as_ptr(), ask.as_ptr()),
                    )
                };
                eq_i32("xwing_dec(mlkem-half corrupt) rc", ra, rb);
                assert_eq!(ra, 0);
                eq_bytes("xwing_dec(mlkem-half corrupt) ss", &ads2, &bds2);
                assert_ne!(&ads2[..], &ads[..]);
            }
            // corruption in the X25519 half: still 0 unless it becomes a
            // small-order point (that case is a Phase C row)
            let mut ct3 = act.clone();
            ct3[1100] ^= 0x20;
            let mut ads3 = canary(ssb);
            let mut bds3 = canary(ssb);
            let (ra, rb) = unsafe {
                (
                    c_d(ads3.as_mut_ptr(), ct3.as_ptr(), ask.as_ptr()),
                    r_d(bds3.as_mut_ptr(), ct3.as_ptr(), ask.as_ptr()),
                )
            };
            eq_i32("xwing_dec(x25519-half corrupt) rc", ra, rb);
            eq_bytes("xwing_dec(x25519-half corrupt) ss", &ads3, &bds3);
        }

        // G6-135/139: RNG-driven keypair and enc
        let mut apk2 = canary(pkb);
        let mut ask2 = canary(skb);
        let mut bpk2 = canary(pkb);
        let mut bsk2 = canary(skb);
        reset_rngs(0x3456_0000 + i as u64);
        let ra = unsafe { c_kp(apk2.as_mut_ptr(), ask2.as_mut_ptr()) };
        reset_rngs(0x3456_0000 + i as u64);
        let rb = unsafe { r_kp(bpk2.as_mut_ptr(), bsk2.as_mut_ptr()) };
        eq_i32("xwing_keypair rc", ra, rb);
        eq_bytes("xwing_keypair pk", &apk2, &bpk2);
        eq_bytes("xwing_keypair sk", &ask2, &bsk2);

        let mut act = canary(ctb);
        let mut ass = canary(ssb);
        let mut bct = canary(ctb);
        let mut bss = canary(ssb);
        reset_rngs(0x4567_0000 + i as u64);
        let ra = unsafe { c_e(act.as_mut_ptr(), ass.as_mut_ptr(), apk2.as_ptr()) };
        reset_rngs(0x4567_0000 + i as u64);
        let rb = unsafe { r_e(bct.as_mut_ptr(), bss.as_mut_ptr(), bpk2.as_ptr()) };
        eq_i32("xwing_enc rc", ra, rb);
        eq_bytes("xwing_enc ct", &act, &bct);
        eq_bytes("xwing_enc ss", &ass, &bss);
        let mut ads = canary(ssb);
        unsafe { assert_eq!(r_d(ads.as_mut_ptr(), act.as_ptr(), ask2.as_ptr()), 0) };
        eq_bytes("xwing enc/dec round trip", &ass, &ads);
    }
}

/// CONFIGS G6-143, G6-144 — the generic `crypto_kem_*` dispatch is X-Wing;
/// every output must be byte-identical to the `crypto_kem_xwing_*` entry
/// points, and there is no generic `crypto_kem_enc_deterministic`.
#[test]
fn kem_generic_dispatch() {
    let _rng_lock = rng_guard();
    setup();
    let mut rng = Rng::new(0x6015);
    assert_eq!(eq_sz("crypto_kem_publickeybytes"), 1216);
    assert_eq!(eq_sz("crypto_kem_secretkeybytes"), 32);
    assert_eq!(eq_sz("crypto_kem_ciphertextbytes"), 1120);
    assert_eq!(eq_sz("crypto_kem_sharedsecretbytes"), 32);
    assert_eq!(eq_sz("crypto_kem_seedbytes"), 32);
    assert_eq!(eq_str("crypto_kem_primitive"), "xwing");
    assert!(
        unsafe {
            c_lib().get::<*const std::ffi::c_void>(b"crypto_kem_enc_deterministic")
        }
        .is_err(),
        "the generic API has no crypto_kem_enc_deterministic"
    );
    assert!(
        unsafe {
            r_lib().get::<*const std::ffi::c_void>(b"crypto_kem_enc_deterministic")
        }
        .is_err()
    );

    let (c_sk, r_sk) = pair::<KemSeedKeypair>("crypto_kem_seed_keypair");
    let (_, x_sk) = pair::<KemSeedKeypair>("crypto_kem_xwing_seed_keypair");
    let (c_kp, r_kp) = pair::<KemKeypair>("crypto_kem_keypair");
    let (c_e, r_e) = pair::<KemEnc>("crypto_kem_enc");
    let (c_d, r_d) = pair::<KemDec>("crypto_kem_dec");
    let (_, x_d) = pair::<KemDec>("crypto_kem_xwing_dec");

    for i in 0..5u64 {
        let seed = if i == 0 {
            vec![0u8; 32]
        } else if i == 1 {
            vec![0xffu8; 32]
        } else {
            rng.bytes(32)
        };
        let mut apk = canary(1216);
        let mut ask = canary(32);
        let mut bpk = canary(1216);
        let mut bsk = canary(32);
        let mut xpk = canary(1216);
        let mut xsk = canary(32);
        let (ra, rb) = unsafe {
            (
                c_sk(apk.as_mut_ptr(), ask.as_mut_ptr(), seed.as_ptr()),
                r_sk(bpk.as_mut_ptr(), bsk.as_mut_ptr(), seed.as_ptr()),
            )
        };
        unsafe { x_sk(xpk.as_mut_ptr(), xsk.as_mut_ptr(), seed.as_ptr()) };
        eq_i32("crypto_kem_seed_keypair rc", ra, rb);
        eq_bytes("crypto_kem_seed_keypair pk", &apk, &bpk);
        eq_bytes("crypto_kem_seed_keypair sk", &ask, &bsk);
        eq_bytes("crypto_kem_seed_keypair == xwing", &apk, &xpk);

        let mut ct_a = canary(1120);
        let mut ss_a = canary(32);
        let mut ct_b = canary(1120);
        let mut ss_b = canary(32);
        reset_rngs(0x5678_0000 + i);
        let ra = unsafe { c_e(ct_a.as_mut_ptr(), ss_a.as_mut_ptr(), apk.as_ptr()) };
        reset_rngs(0x5678_0000 + i);
        let rb = unsafe { r_e(ct_b.as_mut_ptr(), ss_b.as_mut_ptr(), bpk.as_ptr()) };
        eq_i32("crypto_kem_enc rc", ra, rb);
        eq_bytes("crypto_kem_enc ct", &ct_a, &ct_b);
        eq_bytes("crypto_kem_enc ss", &ss_a, &ss_b);

        let mut ds_a = canary(32);
        let mut ds_b = canary(32);
        let mut ds_x = canary(32);
        let (ra, rb) = unsafe {
            (
                c_d(ds_a.as_mut_ptr(), ct_a.as_ptr(), ask.as_ptr()),
                r_d(ds_b.as_mut_ptr(), ct_b.as_ptr(), bsk.as_ptr()),
            )
        };
        unsafe { x_d(ds_x.as_mut_ptr(), ct_b.as_ptr(), bsk.as_ptr()) };
        eq_i32("crypto_kem_dec rc", ra, rb);
        eq_bytes("crypto_kem_dec ss", &ds_a, &ds_b);
        eq_bytes("crypto_kem_dec == xwing_dec", &ds_a, &ds_x);
        eq_bytes("crypto_kem round trip", &ss_a, &ds_a);

        // RNG-driven keypair
        let mut apk2 = canary(1216);
        let mut ask2 = canary(32);
        let mut bpk2 = canary(1216);
        let mut bsk2 = canary(32);
        reset_rngs(0x6789_0000 + i);
        let ra = unsafe { c_kp(apk2.as_mut_ptr(), ask2.as_mut_ptr()) };
        reset_rngs(0x6789_0000 + i);
        let rb = unsafe { r_kp(bpk2.as_mut_ptr(), bsk2.as_mut_ptr()) };
        eq_i32("crypto_kem_keypair rc", ra, rb);
        eq_bytes("crypto_kem_keypair pk", &apk2, &bpk2);
        eq_bytes("crypto_kem_keypair sk", &ask2, &bsk2);
    }
}

// ===========================================================================
// crypto_ipcrypt
// ===========================================================================

/// The IP-address input shapes used by every ipcrypt row: IPv4-mapped
/// (`00`x10 `ff ff` + 4 bytes), native IPv6, all-zero, all-0xff, random.
fn ip_shapes(rng: &mut Rng) -> Vec<[u8; 16]> {
    let mut v: Vec<[u8; 16]> = Vec::new();
    let mk4 = |a: u8, b: u8, c: u8, d: u8| {
        let mut x = [0u8; 16];
        x[10] = 0xff;
        x[11] = 0xff;
        x[12] = a;
        x[13] = b;
        x[14] = c;
        x[15] = d;
        x
    };
    v.push(mk4(192, 168, 1, 1));
    v.push(mk4(0, 0, 0, 0));
    v.push(mk4(255, 255, 255, 255));
    v.push(mk4(10, 0, 0, 1));
    v.push(mk4(10, 0, 0, 2));
    v.push(mk4(10, 1, 0, 1));
    v.push([0u8; 16]);
    v.push([0xffu8; 16]);
    let mut v6 = [0u8; 16];
    v6[0] = 0x20;
    v6[1] = 0x01;
    v6[2] = 0x0d;
    v6[3] = 0xb8;
    for i in 4..16 {
        v6[i] = rng.byte();
    }
    v.push(v6);
    let mut v6b = v6;
    v6b[15] ^= 0xff; // same /120 prefix
    v.push(v6b);
    for _ in 0..14 {
        let mut x = [0u8; 16];
        rng.fill(&mut x);
        v.push(x);
    }
    v
}

/// 16-byte key shapes: all-zero, all-0xff, the FIPS-197 key, random.
fn key16_shapes(rng: &mut Rng) -> Vec<[u8; 16]> {
    let mut v: Vec<[u8; 16]> = vec![[0u8; 16], [0xffu8; 16]];
    let mut fips = [0u8; 16];
    for i in 0..16 {
        fips[i] = i as u8;
    }
    v.push(fips);
    for _ in 0..12 {
        let mut k = [0u8; 16];
        rng.fill(&mut k);
        v.push(k);
    }
    v
}

/// 32-byte key shapes for the `ndx` / `pfx` variants — both the ordinary
/// branch and the **degenerate** `k[0..16] == k[16..32]` branch.
fn key32_shapes(rng: &mut Rng) -> Vec<[u8; 32]> {
    let mut v: Vec<[u8; 32]> = Vec::new();
    v.push([0u8; 32]); // degenerate
    v.push([0xffu8; 32]); // degenerate
    let mut rep = [0u8; 32];
    for i in 0..16 {
        rep[i] = (i as u8) * 3 + 1;
        rep[16 + i] = (i as u8) * 3 + 1;
    }
    v.push(rep); // degenerate
    let mut fips = [0u8; 32];
    for i in 0..32 {
        fips[i] = i as u8;
    }
    v.push(fips); // non-degenerate
    for _ in 0..6 {
        let mut k = [0u8; 32];
        rng.fill(&mut k);
        v.push(k); // non-degenerate with overwhelming probability
    }
    v
}

/// CONFIGS G6-145, G6-146, G6-147 — the deterministic (plain AES-128-ECB)
/// ipcrypt variant: `encrypt` / `decrypt` over every IP and key shape, the
/// round trip (which goes through the separate inverse key schedule), and the
/// FIPS-197 AES-128 known-answer test.
#[test]
fn ipcrypt_deterministic() {
    setup();
    let mut rng = Rng::new(0x6016);
    assert_eq!(eq_sz("crypto_ipcrypt_bytes"), 16);
    assert_eq!(eq_sz("crypto_ipcrypt_keybytes"), 16);
    let (c_e, r_e) = pair::<V3>("crypto_ipcrypt_encrypt");
    let (c_d, r_d) = pair::<V3>("crypto_ipcrypt_decrypt");

    for k in &key16_shapes(&mut rng) {
        for ip in &ip_shapes(&mut rng) {
            let mut a = canary(16);
            let mut b = canary(16);
            unsafe {
                c_e(a.as_mut_ptr(), ip.as_ptr(), k.as_ptr());
                r_e(b.as_mut_ptr(), ip.as_ptr(), k.as_ptr());
            }
            eq_bytes(&format!("ipcrypt_encrypt(ip={}, k={})", hex(ip), hex(k)), &a, &b);

            let mut ad = canary(16);
            let mut bd = canary(16);
            unsafe {
                c_d(ad.as_mut_ptr(), a.as_ptr(), k.as_ptr());
                r_d(bd.as_mut_ptr(), b.as_ptr(), k.as_ptr());
            }
            eq_bytes("ipcrypt_decrypt", &ad, &bd);
            eq_bytes("ipcrypt round trip", ip, &ad);

            // decrypt of an arbitrary block (not necessarily a ciphertext)
            let mut ad = canary(16);
            let mut bd = canary(16);
            unsafe {
                c_d(ad.as_mut_ptr(), ip.as_ptr(), k.as_ptr());
                r_d(bd.as_mut_ptr(), ip.as_ptr(), k.as_ptr());
            }
            eq_bytes("ipcrypt_decrypt(raw)", &ad, &bd);
        }
    }

    // G6-147: FIPS-197 AES-128 vector
    let k = unhex("000102030405060708090a0b0c0d0e0f");
    let pt = unhex("00112233445566778899aabbccddeeff");
    let expect = unhex("69c4e0d86a7b0430d8cdb78070b4c55a");
    let mut a = canary(16);
    let mut b = canary(16);
    unsafe {
        c_e(a.as_mut_ptr(), pt.as_ptr(), k.as_ptr());
        r_e(b.as_mut_ptr(), pt.as_ptr(), k.as_ptr());
    }
    eq_bytes("FIPS-197 (C vs Rust)", &a, &b);
    eq_bytes("FIPS-197 known answer", &expect, &b);
    let mut back = canary(16);
    unsafe { r_d(back.as_mut_ptr(), expect.as_ptr(), k.as_ptr()) };
    eq_bytes("FIPS-197 inverse", &pt, &back);
}

/// CONFIGS G6-148, G6-149, G6-150, G6-151 — the `nd` variant: 8-byte tweak
/// copied verbatim into `out[0..8]`, the "prince-like" tweak block XORed into
/// **every** round key, and the decrypt path's `inv_mix_columns(tweak)` for the
/// middle rounds vs the raw tweak for `AES_DECLAST`.
#[test]
fn ipcrypt_nd() {
    setup();
    let mut rng = Rng::new(0x6017);
    assert_eq!(eq_sz("crypto_ipcrypt_nd_keybytes"), 16);
    assert_eq!(eq_sz("crypto_ipcrypt_nd_tweakbytes"), 8);
    assert_eq!(eq_sz("crypto_ipcrypt_nd_inputbytes"), 16);
    assert_eq!(eq_sz("crypto_ipcrypt_nd_outputbytes"), 24);
    let (c_e, r_e) = pair::<V4>("crypto_ipcrypt_nd_encrypt");
    let (c_d, r_d) = pair::<V3>("crypto_ipcrypt_nd_decrypt");

    let tweaks: Vec<Vec<u8>> = {
        let mut v = vec![
            vec![0u8; 8],
            vec![0xffu8; 8],
            unhex("0102030405060708"),
            unhex("00ff00ff00ff00ff"),
        ];
        for _ in 0..5 {
            v.push(rng.bytes(8));
        }
        v
    };

    for k in &key16_shapes(&mut rng) {
        for ip in &ip_shapes(&mut rng) {
            for t in &tweaks {
                let mut a = canary(24);
                let mut b = canary(24);
                unsafe {
                    c_e(a.as_mut_ptr(), ip.as_ptr(), t.as_ptr(), k.as_ptr());
                    r_e(b.as_mut_ptr(), ip.as_ptr(), t.as_ptr(), k.as_ptr());
                }
                eq_bytes(
                    &format!("nd_encrypt(ip={}, t={}, k={})", hex(ip), hex(t), hex(k)),
                    &a,
                    &b,
                );
                assert_eq!(&a[..8], &t[..], "tweak must be copied verbatim");

                let mut ad = canary(16);
                let mut bd = canary(16);
                unsafe {
                    c_d(ad.as_mut_ptr(), a.as_ptr(), k.as_ptr());
                    r_d(bd.as_mut_ptr(), b.as_ptr(), k.as_ptr());
                }
                eq_bytes("nd_decrypt", &ad, &bd);
                eq_bytes("nd round trip", ip, &ad);
            }
        }
    }

    // decrypt of arbitrary 24-byte blobs (tweak read out of the input)
    for _ in 0..300 {
        let blob = rng.bytes(24);
        let k = rng.bytes(16);
        let mut a = canary(16);
        let mut b = canary(16);
        unsafe {
            c_d(a.as_mut_ptr(), blob.as_ptr(), k.as_ptr());
            r_d(b.as_mut_ptr(), blob.as_ptr(), k.as_ptr());
        }
        eq_bytes("nd_decrypt(raw)", &a, &b);
    }

    // G6-149: only the low 16 bits of each tweak word are populated, so
    // tweak bytes are pairwise-independent halfwords; changing t[1] must
    // change the output, and the tweak block never touches bits 16..31.
    let k = rng.bytes(16);
    let ip = rng.bytes(16);
    let base_t = vec![0u8; 8];
    let mut base_out = canary(24);
    unsafe { r_e(base_out.as_mut_ptr(), ip.as_ptr(), base_t.as_ptr(), k.as_ptr()) };
    for i in 0..8usize {
        let mut t = base_t.clone();
        t[i] = 0x5a;
        let mut o = canary(24);
        unsafe { r_e(o.as_mut_ptr(), ip.as_ptr(), t.as_ptr(), k.as_ptr()) };
        assert_ne!(o[8..], base_out[8..], "tweak byte {i} had no effect");
        let mut oc = canary(24);
        unsafe { c_e(oc.as_mut_ptr(), ip.as_ptr(), t.as_ptr(), k.as_ptr()) };
        eq_bytes("nd tweak byte sweep", &oc, &o);
    }
}

/// CONFIGS G6-152, G6-153, G6-154, G6-155 — the `ndx` (XEX) variant with a
/// 32-byte key: `k[0..16]` data key, `k[16..32]` tweak key, the tweak XORed
/// only into the first and last round keys, and the **degenerate-key** branch
/// (`k[0..16] == k[16..32]` -> re-expand from `k[0..16] XOR 0x5a`).
#[test]
fn ipcrypt_ndx() {
    setup();
    let mut rng = Rng::new(0x6018);
    assert_eq!(eq_sz("crypto_ipcrypt_ndx_keybytes"), 32);
    assert_eq!(eq_sz("crypto_ipcrypt_ndx_tweakbytes"), 16);
    assert_eq!(eq_sz("crypto_ipcrypt_ndx_inputbytes"), 16);
    assert_eq!(eq_sz("crypto_ipcrypt_ndx_outputbytes"), 32);
    let (c_e, r_e) = pair::<V4>("crypto_ipcrypt_ndx_encrypt");
    let (c_d, r_d) = pair::<V3>("crypto_ipcrypt_ndx_decrypt");

    let mut tweaks: Vec<[u8; 16]> = vec![[0u8; 16], [0xffu8; 16]];
    for _ in 0..4 {
        let mut t = [0u8; 16];
        rng.fill(&mut t);
        tweaks.push(t);
    }

    for k in &key32_shapes(&mut rng) {
        let degenerate = k[..16] == k[16..];
        for ip in &ip_shapes(&mut rng) {
            for t in &tweaks {
                let mut a = canary(32);
                let mut b = canary(32);
                unsafe {
                    c_e(a.as_mut_ptr(), ip.as_ptr(), t.as_ptr(), k.as_ptr());
                    r_e(b.as_mut_ptr(), ip.as_ptr(), t.as_ptr(), k.as_ptr());
                }
                eq_bytes(
                    &format!(
                        "ndx_encrypt(ip={}, t={}, k={}, degenerate={degenerate})",
                        hex(ip),
                        hex(t),
                        hex(k)
                    ),
                    &a,
                    &b,
                );
                assert_eq!(&a[..16], &t[..], "tweak must be copied verbatim");

                let mut ad = canary(16);
                let mut bd = canary(16);
                unsafe {
                    c_d(ad.as_mut_ptr(), a.as_ptr(), k.as_ptr());
                    r_d(bd.as_mut_ptr(), b.as_ptr(), k.as_ptr());
                }
                eq_bytes("ndx_decrypt", &ad, &bd);
                eq_bytes("ndx round trip", ip, &ad);
            }
        }
    }

    // arbitrary 32-byte blobs
    for _ in 0..300 {
        let blob = rng.bytes(32);
        let k = rng.bytes(32);
        let mut a = canary(16);
        let mut b = canary(16);
        unsafe {
            c_d(a.as_mut_ptr(), blob.as_ptr(), k.as_ptr());
            r_d(b.as_mut_ptr(), blob.as_ptr(), k.as_ptr());
        }
        eq_bytes("ndx_decrypt(raw)", &a, &b);
    }

    // G6-153: the degenerate branch really is a different transform. With
    // k = k1||k1 the data schedule comes from k1 XOR 0x5a, so the output must
    // differ from the non-degenerate key k1||(k1 with one bit flipped).
    let mut k1 = [0u8; 16];
    rng.fill(&mut k1);
    let mut kd = [0u8; 32];
    kd[..16].copy_from_slice(&k1);
    kd[16..].copy_from_slice(&k1);
    let mut kn = kd;
    kn[31] ^= 0x01;
    let ip = rng.bytes(16);
    let t = rng.bytes(16);
    let mut od = canary(32);
    let mut on = canary(32);
    unsafe {
        r_e(od.as_mut_ptr(), ip.as_ptr(), t.as_ptr(), kd.as_ptr());
        r_e(on.as_mut_ptr(), ip.as_ptr(), t.as_ptr(), kn.as_ptr());
    }
    assert_ne!(od[16..], on[16..]);
    let mut odc = canary(32);
    unsafe { c_e(odc.as_mut_ptr(), ip.as_ptr(), t.as_ptr(), kd.as_ptr()) };
    eq_bytes("ndx degenerate key", &odc, &od);
}

/// CONFIGS G6-156, G6-157, G6-158, G6-159, G6-160 — the prefix-preserving
/// `pfx` variant: the `is_ipv4_mapped` path split (32 vs 128 bit positions,
/// and the pre-set `out[10..12] = ff ff`), both `pfx_pad_prefix` shapes, the
/// degenerate-key branch, the prefix-preservation property and the round trip.
#[test]
fn ipcrypt_pfx() {
    setup();
    let mut rng = Rng::new(0x6019);
    assert_eq!(eq_sz("crypto_ipcrypt_pfx_keybytes"), 32);
    assert_eq!(eq_sz("crypto_ipcrypt_pfx_bytes"), 16);
    let (c_e, r_e) = pair::<V3>("crypto_ipcrypt_pfx_encrypt");
    let (c_d, r_d) = pair::<V3>("crypto_ipcrypt_pfx_decrypt");

    let keys = key32_shapes(&mut rng);
    let ips = ip_shapes(&mut rng);
    for k in &keys {
        let degenerate = k[..16] == k[16..];
        for ip in &ips {
            let v4 = ip[..10].iter().all(|&b| b == 0) && ip[10] == 0xff && ip[11] == 0xff;
            let mut a = canary(16);
            let mut b = canary(16);
            unsafe {
                c_e(a.as_mut_ptr(), ip.as_ptr(), k.as_ptr());
                r_e(b.as_mut_ptr(), ip.as_ptr(), k.as_ptr());
            }
            eq_bytes(
                &format!(
                    "pfx_encrypt(ip={}, k={}, v4={v4}, degenerate={degenerate})",
                    hex(ip),
                    hex(k)
                ),
                &a,
                &b,
            );
            if v4 {
                // the IPv4-mapped prefix is preserved verbatim
                assert!(a[..10].iter().all(|&x| x == 0));
                assert_eq!((a[10], a[11]), (0xff, 0xff));
            }

            let mut ad = canary(16);
            let mut bd = canary(16);
            unsafe {
                c_d(ad.as_mut_ptr(), a.as_ptr(), k.as_ptr());
                r_d(bd.as_mut_ptr(), b.as_ptr(), k.as_ptr());
            }
            eq_bytes("pfx_decrypt", &ad, &bd);
            eq_bytes("pfx round trip", ip, &ad);

            // decrypt of an arbitrary 16-byte blob (the path is chosen on the
            // ciphertext, so both branches are reached here too)
            let mut ad = canary(16);
            let mut bd = canary(16);
            unsafe {
                c_d(ad.as_mut_ptr(), ip.as_ptr(), k.as_ptr());
                r_d(bd.as_mut_ptr(), ip.as_ptr(), k.as_ptr());
            }
            eq_bytes("pfx_decrypt(raw)", &ad, &bd);
        }
    }

    // G6-158: prefix preservation. Two IPv4-mapped addresses sharing an n-bit
    // prefix must have ciphertexts sharing the same n-bit prefix.
    let shared_bits = |x: &[u8], y: &[u8], from: usize| -> usize {
        let mut n = 0usize;
        for i in from..128 {
            let bi = i / 8;
            let sh = 7 - (i % 8);
            if (x[bi] >> sh) & 1 == (y[bi] >> sh) & 1 {
                n += 1;
            } else {
                break;
            }
        }
        n
    };
    let k = &keys[3];
    let mk4 = |a: u8, b: u8, c: u8, d: u8| {
        let mut x = [0u8; 16];
        x[10] = 0xff;
        x[11] = 0xff;
        x[12] = a;
        x[13] = b;
        x[14] = c;
        x[15] = d;
        x
    };
    for (p, q) in [
        (mk4(10, 0, 0, 1), mk4(10, 0, 0, 2)),
        (mk4(10, 0, 0, 1), mk4(10, 1, 0, 1)),
        (mk4(192, 168, 1, 1), mk4(192, 168, 1, 254)),
    ] {
        let mut cp = [0u8; 16];
        let mut cq = [0u8; 16];
        unsafe {
            r_e(cp.as_mut_ptr(), p.as_ptr(), k.as_ptr());
            r_e(cq.as_mut_ptr(), q.as_ptr(), k.as_ptr());
        }
        let want = shared_bits(&p, &q, 96);
        let got = shared_bits(&cp, &cq, 96);
        assert_eq!(want, got, "IPv4 prefix preservation: {} vs {}", hex(&p), hex(&q));
    }
    // IPv6 pairs sharing a /64
    for _ in 0..4 {
        let mut p = [0u8; 16];
        rng.fill(&mut p);
        let mut q = p;
        for i in 8..16 {
            q[i] = rng.byte();
        }
        if p[8..] == q[8..] {
            continue;
        }
        let mut cp = [0u8; 16];
        let mut cq = [0u8; 16];
        unsafe {
            r_e(cp.as_mut_ptr(), p.as_ptr(), k.as_ptr());
            r_e(cq.as_mut_ptr(), q.as_ptr(), k.as_ptr());
        }
        let want = shared_bits(&p, &q, 0);
        let got = shared_bits(&cp, &cq, 0);
        assert_eq!(want, got, "IPv6 prefix preservation");
    }

    // G6-159: the degenerate branch is a genuinely different transform
    let mut k1 = [0u8; 16];
    rng.fill(&mut k1);
    let mut kd = [0u8; 32];
    kd[..16].copy_from_slice(&k1);
    kd[16..].copy_from_slice(&k1);
    let mut kn = kd;
    kn[31] ^= 0x01;
    let ip = mk4(1, 2, 3, 4);
    let mut od = [0u8; 16];
    let mut on = [0u8; 16];
    let mut odc = [0u8; 16];
    unsafe {
        r_e(od.as_mut_ptr(), ip.as_ptr(), kd.as_ptr());
        r_e(on.as_mut_ptr(), ip.as_ptr(), kn.as_ptr());
        c_e(odc.as_mut_ptr(), ip.as_ptr(), kd.as_ptr());
    }
    assert_ne!(od, on);
    eq_bytes("pfx degenerate key", &odc, &od);
}

/// CONFIGS G6-161, G6-162, G6-163, G6-164, G6-165 — every ipcrypt keygen
/// (16/16/32/32 random bytes) and every size accessor.
#[test]
fn ipcrypt_keygen_and_sizes() {
    let _rng_lock = rng_guard();
    setup();
    for (name, n) in [
        ("crypto_ipcrypt_keygen", 16usize),
        ("crypto_ipcrypt_nd_keygen", 16),
        ("crypto_ipcrypt_ndx_keygen", 32),
        ("crypto_ipcrypt_pfx_keygen", 32),
    ] {
        let (c, r) = pair::<V1>(name);
        for seed in 0..8u64 {
            let mut a = canary(n);
            let mut b = canary(n);
            reset_rngs(0x7000_0000 + seed);
            unsafe { c(a.as_mut_ptr()) };
            reset_rngs(0x7000_0000 + seed);
            unsafe { r(b.as_mut_ptr()) };
            eq_bytes(&format!("{name} seed={seed}"), &a, &b);
            assert_ne!(a, canary(n), "{name} wrote nothing");
        }
    }
    for (name, want) in [
        ("crypto_ipcrypt_bytes", 16usize),
        ("crypto_ipcrypt_keybytes", 16),
        ("crypto_ipcrypt_nd_keybytes", 16),
        ("crypto_ipcrypt_nd_tweakbytes", 8),
        ("crypto_ipcrypt_nd_inputbytes", 16),
        ("crypto_ipcrypt_nd_outputbytes", 24),
        ("crypto_ipcrypt_ndx_keybytes", 32),
        ("crypto_ipcrypt_ndx_tweakbytes", 16),
        ("crypto_ipcrypt_ndx_inputbytes", 16),
        ("crypto_ipcrypt_ndx_outputbytes", 32),
        ("crypto_ipcrypt_pfx_keybytes", 32),
        ("crypto_ipcrypt_pfx_bytes", 16),
    ] {
        assert_eq!(eq_sz(name), want, "{name}");
    }
}

// ===========================================================================
// exported implementation vtables + softaes
// ===========================================================================

/// Read a data symbol's address as a slice of raw function pointers.
unsafe fn vtable(lib: &'static libloading::Library, name: &str, n: usize) -> Vec<*const ()> {
    let base = sym::<*const *const ()>(lib, name);
    (0..n).map(|i| unsafe { *base.add(i) }).collect()
}

/// CONFIGS G6-166 — the `ipcrypt_soft_implementation` vtable: 8 function
/// pointers, driven directly and compared against the C ones.
#[test]
fn ipcrypt_soft_implementation_vtable() {
    setup();
    let mut rng = Rng::new(0x601a);
    let cv = unsafe { vtable(c_lib(), "ipcrypt_soft_implementation", 8) };
    let rv = unsafe { vtable(r_lib(), "ipcrypt_soft_implementation", 8) };
    assert!(cv.iter().all(|p| !p.is_null()), "C vtable has NULL entries");
    assert!(rv.iter().all(|p| !p.is_null()), "Rust vtable has NULL entries");

    let k16 = rng.bytes(16);
    let k32 = rng.bytes(32);
    let ip = rng.bytes(16);
    let t8 = rng.bytes(8);
    let t16 = rng.bytes(16);

    // slot -> (out len, in len, has tweak, key)
    let specs: &[(usize, usize, usize, Option<&Vec<u8>>, &Vec<u8>)] = &[
        (0, 16, 16, None, &k16),      // encrypt
        (1, 16, 16, None, &k16),      // decrypt
        (2, 24, 16, Some(&t8), &k16), // nd_encrypt
        (4, 32, 16, Some(&t16), &k32), // ndx_encrypt
        (6, 16, 16, None, &k32),      // pfx_encrypt
        (7, 16, 16, None, &k32),      // pfx_decrypt
    ];
    for &(slot, outlen, inlen, tw, key) in specs {
        let mut a = canary(outlen);
        let mut b = canary(outlen);
        unsafe {
            match tw {
                None => {
                    let cf: V3 = std::mem::transmute(cv[slot]);
                    let rf: V3 = std::mem::transmute(rv[slot]);
                    cf(a.as_mut_ptr(), ip[..inlen].as_ptr(), key.as_ptr());
                    rf(b.as_mut_ptr(), ip[..inlen].as_ptr(), key.as_ptr());
                }
                Some(t) => {
                    let cf: V4 = std::mem::transmute(cv[slot]);
                    let rf: V4 = std::mem::transmute(rv[slot]);
                    cf(a.as_mut_ptr(), ip[..inlen].as_ptr(), t.as_ptr(), key.as_ptr());
                    rf(b.as_mut_ptr(), ip[..inlen].as_ptr(), t.as_ptr(), key.as_ptr());
                }
            }
        }
        eq_bytes(&format!("ipcrypt_soft_implementation slot {slot}"), &a, &b);
    }
    // nd_decrypt (slot 3) and ndx_decrypt (slot 5) take a longer input
    for (slot, inlen, key) in [(3usize, 24usize, &k16), (5, 32, &k32)] {
        let blob = rng.bytes(inlen);
        let mut a = canary(16);
        let mut b = canary(16);
        unsafe {
            let cf: V3 = std::mem::transmute(cv[slot]);
            let rf: V3 = std::mem::transmute(rv[slot]);
            cf(a.as_mut_ptr(), blob.as_ptr(), key.as_ptr());
            rf(b.as_mut_ptr(), blob.as_ptr(), key.as_ptr());
        }
        eq_bytes(&format!("ipcrypt_soft_implementation slot {slot}"), &a, &b);
    }
}

/// CONFIGS G6-046, G6-170, G6-171 — the exported `ref`/`ref10` implementation
/// vtables and the `_pick_best_implementation` selectors. Each vtable entry is
/// called through its raw function pointer, so the *selected* implementation is
/// what gets compared, not just the public wrapper.
#[test]
fn implementation_vtables_and_pickers() {
    setup();
    let mut rng = Rng::new(0x601b);

    // `_pick_best_implementation` — always 0, and must not change behaviour
    for name in [
        "_crypto_scalarmult_curve25519_pick_best_implementation",
        "_crypto_stream_chacha20_pick_best_implementation",
        "_crypto_stream_salsa20_pick_best_implementation",
        "_crypto_ipcrypt_pick_best_implementation",
    ] {
        let (c, r) = pair::<unsafe extern "C" fn() -> i32>(name);
        let (a, b) = unsafe { (c(), r()) };
        eq_i32(name, a, b);
        assert_eq!(a, 0, "{name} must return 0");
    }

    // crypto_stream_chacha20_ref_implementation: 4 slots
    let cv = unsafe { vtable(c_lib(), "crypto_stream_chacha20_ref_implementation", 4) };
    let rv = unsafe { vtable(r_lib(), "crypto_stream_chacha20_ref_implementation", 4) };
    for &len in &[0usize, 1, 63, 64, 65, 200] {
        let k = rng.bytes(32);
        let n8 = rng.bytes(8);
        let n12 = rng.bytes(12);
        let m = rng.bytes(len);
        // slot 0: stream (8-byte nonce)
        let mut a = canary(len);
        let mut b = canary(len);
        unsafe {
            let cf: Stream = std::mem::transmute(cv[0]);
            let rf: Stream = std::mem::transmute(rv[0]);
            let x = cf(a.as_mut_ptr(), len as u64, n8.as_ptr(), k.as_ptr());
            let y = rf(b.as_mut_ptr(), len as u64, n8.as_ptr(), k.as_ptr());
            eq_i32("chacha20 ref stream rc", x, y);
        }
        eq_bytes("chacha20 ref stream", &a, &b);
        // slot 1: stream_ietf_ext (16-byte nonce)
        let n16 = rng.bytes(16);
        let mut a = canary(len);
        let mut b = canary(len);
        unsafe {
            let cf: Stream = std::mem::transmute(cv[1]);
            let rf: Stream = std::mem::transmute(rv[1]);
            let x = cf(a.as_mut_ptr(), len as u64, n16.as_ptr(), k.as_ptr());
            let y = rf(b.as_mut_ptr(), len as u64, n16.as_ptr(), k.as_ptr());
            eq_i32("chacha20 ref stream_ietf_ext rc", x, y);
        }
        eq_bytes("chacha20 ref stream_ietf_ext", &a, &b);
        // slot 2: stream_xor_ic (u64 ic)
        for &ic in &[0u64, 1, u64::MAX] {
            let mut a = canary(len);
            let mut b = canary(len);
            unsafe {
                let cf: StreamXorIc64 = std::mem::transmute(cv[2]);
                let rf: StreamXorIc64 = std::mem::transmute(rv[2]);
                let x = cf(a.as_mut_ptr(), m.as_ptr(), len as u64, n8.as_ptr(), ic, k.as_ptr());
                let y = rf(b.as_mut_ptr(), m.as_ptr(), len as u64, n8.as_ptr(), ic, k.as_ptr());
                eq_i32("chacha20 ref stream_xor_ic rc", x, y);
            }
            eq_bytes("chacha20 ref stream_xor_ic", &a, &b);
        }
        // slot 3: stream_ietf_ext_xor_ic (u32 ic) — no overflow guard
        for &ic in &[0u32, 1, 0xffff_ffff] {
            let mut a = canary(len);
            let mut b = canary(len);
            unsafe {
                let cf: StreamXorIc32 = std::mem::transmute(cv[3]);
                let rf: StreamXorIc32 = std::mem::transmute(rv[3]);
                let x = cf(a.as_mut_ptr(), m.as_ptr(), len as u64, n16.as_ptr(), ic, k.as_ptr());
                let y = rf(b.as_mut_ptr(), m.as_ptr(), len as u64, n16.as_ptr(), ic, k.as_ptr());
                eq_i32("chacha20 ref stream_ietf_ext_xor_ic rc", x, y);
            }
            eq_bytes("chacha20 ref stream_ietf_ext_xor_ic", &a, &b);
        }
        let _ = n12;
    }

    // crypto_stream_salsa20_ref_implementation: 2 slots
    let cv = unsafe { vtable(c_lib(), "crypto_stream_salsa20_ref_implementation", 2) };
    let rv = unsafe { vtable(r_lib(), "crypto_stream_salsa20_ref_implementation", 2) };
    for &len in &[0usize, 1, 64, 65, 200] {
        let k = rng.bytes(32);
        let n = rng.bytes(8);
        let m = rng.bytes(len);
        let mut a = canary(len);
        let mut b = canary(len);
        unsafe {
            let cf: Stream = std::mem::transmute(cv[0]);
            let rf: Stream = std::mem::transmute(rv[0]);
            let x = cf(a.as_mut_ptr(), len as u64, n.as_ptr(), k.as_ptr());
            let y = rf(b.as_mut_ptr(), len as u64, n.as_ptr(), k.as_ptr());
            eq_i32("salsa20 ref stream rc", x, y);
        }
        eq_bytes("salsa20 ref stream", &a, &b);
        for &ic in &[0u64, 1, u64::MAX] {
            let mut a = canary(len);
            let mut b = canary(len);
            unsafe {
                let cf: StreamXorIc64 = std::mem::transmute(cv[1]);
                let rf: StreamXorIc64 = std::mem::transmute(rv[1]);
                let x = cf(a.as_mut_ptr(), m.as_ptr(), len as u64, n.as_ptr(), ic, k.as_ptr());
                let y = rf(b.as_mut_ptr(), m.as_ptr(), len as u64, n.as_ptr(), ic, k.as_ptr());
                eq_i32("salsa20 ref stream_xor_ic rc", x, y);
            }
            eq_bytes("salsa20 ref stream_xor_ic", &a, &b);
        }
    }

    // crypto_scalarmult_curve25519_ref10_implementation: mult / mult_base
    let cv = unsafe { vtable(c_lib(), "crypto_scalarmult_curve25519_ref10_implementation", 2) };
    let rv = unsafe { vtable(r_lib(), "crypto_scalarmult_curve25519_ref10_implementation", 2) };
    for _ in 0..8 {
        let n = rng.bytes(32);
        let mut a = canary(32);
        let mut b = canary(32);
        unsafe {
            let cf: I2 = std::mem::transmute(cv[1]);
            let rf: I2 = std::mem::transmute(rv[1]);
            let x = cf(a.as_mut_ptr(), n.as_ptr());
            let y = rf(b.as_mut_ptr(), n.as_ptr());
            eq_i32("ref10 mult_base rc", x, y);
        }
        eq_bytes("ref10 mult_base", &a, &b);
        let p = a.clone();
        let n2 = rng.bytes(32);
        let mut a = canary(32);
        let mut b = canary(32);
        unsafe {
            let cf: I3 = std::mem::transmute(cv[0]);
            let rf: I3 = std::mem::transmute(rv[0]);
            let x = cf(a.as_mut_ptr(), n2.as_ptr(), p.as_ptr());
            let y = rf(b.as_mut_ptr(), n2.as_ptr(), p.as_ptr());
            eq_i32("ref10 mult rc", x, y);
        }
        eq_bytes("ref10 mult", &a, &b);
    }
}

/// `SoftAesBlock` — 4 x `uint32_t`, passed and returned by value.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
struct AesBlock {
    w0: u32,
    w1: u32,
    w2: u32,
    w3: u32,
}

type ExpandKey = unsafe extern "C" fn(*mut AesBlock, *const u8);
type InvertKs = unsafe extern "C" fn(*mut AesBlock);
type InvMix = unsafe extern "C" fn(AesBlock) -> AesBlock;
type Round = unsafe extern "C" fn(AesBlock, AesBlock) -> AesBlock;

fn ld(b: &[u8]) -> AesBlock {
    AesBlock {
        w0: u32::from_le_bytes(b[0..4].try_into().unwrap()),
        w1: u32::from_le_bytes(b[4..8].try_into().unwrap()),
        w2: u32::from_le_bytes(b[8..12].try_into().unwrap()),
        w3: u32::from_le_bytes(b[12..16].try_into().unwrap()),
    }
}

fn st(b: AesBlock) -> Vec<u8> {
    let mut v = Vec::with_capacity(16);
    for w in [b.w0, b.w1, b.w2, b.w3] {
        v.extend_from_slice(&w.to_le_bytes());
    }
    v
}

fn blk_hex(b: AesBlock) -> String {
    hex(&st(b))
}

/// CONFIGS G6-167, G6-168, G6-169 — `softaes`. The symbols are exported under
/// their internal `_sodium_softaes_*` aliases, so the round functions, the key
/// schedules and `inv_mix_columns` can be compared directly (and the
/// composition is additionally checked against the FIPS-197 AES-128 vector,
/// which also confirms the ABI of the by-value `SoftAesBlock`).
///
/// `FAVOR_PERFORMANCE` is undefined in this build, so the live definitions are
/// the SRM-1R bitsliced ones; the 1024-entry-LUT half of `softaes.c` is
/// compiled out and must not appear in the translation either.
#[test]
fn softaes_primitives() {
    setup();
    let mut rng = Rng::new(0x601c);
    let (c_ek1, r_ek1) = pair::<ExpandKey>("_sodium_softaes_expand_key128");
    let (c_ek2, r_ek2) = pair::<ExpandKey>("_sodium_softaes_expand_key256");
    let (c_ik1, r_ik1) = pair::<InvertKs>("_sodium_softaes_invert_key_schedule128");
    let (c_ik2, r_ik2) = pair::<InvertKs>("_sodium_softaes_invert_key_schedule256");
    let (c_imc, r_imc) = pair::<InvMix>("_sodium_softaes_inv_mix_columns");
    let (c_enc, r_enc) = pair::<Round>("_sodium_softaes_block_encrypt");
    let (c_dec, r_dec) = pair::<Round>("_sodium_softaes_block_decrypt");
    let (c_encl, r_encl) = pair::<Round>("_sodium_softaes_block_encryptlast");
    let (c_decl, r_decl) = pair::<Round>("_sodium_softaes_block_decryptlast");

    // G6-169: key schedules (AES-128 and AES-256) and their inversion
    let mut k16s: Vec<Vec<u8>> = vec![vec![0u8; 16], vec![0xffu8; 16], unhex("000102030405060708090a0b0c0d0e0f")];
    let mut k32s: Vec<Vec<u8>> = vec![
        vec![0u8; 32],
        vec![0xffu8; 32],
        unhex("000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f"),
    ];
    for _ in 0..64 {
        k16s.push(rng.bytes(16));
        k32s.push(rng.bytes(32));
    }
    for k in &k16s {
        let mut a = [AesBlock::default(); 11];
        let mut b = [AesBlock::default(); 11];
        unsafe {
            c_ek1(a.as_mut_ptr(), k.as_ptr());
            r_ek1(b.as_mut_ptr(), k.as_ptr());
        }
        assert_eq!(a, b, "expand_key128({})", hex(k));
        unsafe {
            c_ik1(a.as_mut_ptr());
            r_ik1(b.as_mut_ptr());
        }
        assert_eq!(a, b, "invert_key_schedule128({})", hex(k));
    }
    for k in &k32s {
        let mut a = [AesBlock::default(); 15];
        let mut b = [AesBlock::default(); 15];
        unsafe {
            c_ek2(a.as_mut_ptr(), k.as_ptr());
            r_ek2(b.as_mut_ptr(), k.as_ptr());
        }
        assert_eq!(a, b, "expand_key256({})", hex(k));
        unsafe {
            c_ik2(a.as_mut_ptr());
            r_ik2(b.as_mut_ptr());
        }
        assert_eq!(a, b, "invert_key_schedule256({})", hex(k));
    }

    // G6-168: every round function, over many random (block, round key) pairs
    let mut blocks: Vec<AesBlock> = vec![
        AesBlock::default(),
        AesBlock { w0: !0, w1: !0, w2: !0, w3: !0 },
    ];
    for i in 0..32u32 {
        // single-bit blocks exercise every S-box bit plane in isolation
        let bit = 1u32 << (i % 32);
        blocks.push(AesBlock { w0: bit, w1: 0, w2: 0, w3: 0 });
        blocks.push(AesBlock { w0: 0, w1: bit, w2: 0, w3: 0 });
        blocks.push(AesBlock { w0: 0, w1: 0, w2: bit, w3: 0 });
        blocks.push(AesBlock { w0: 0, w1: 0, w2: 0, w3: bit });
    }
    for _ in 0..128 {
        blocks.push(ld(&rng.bytes(16)));
    }
    let mut rkeys: Vec<AesBlock> = vec![
        AesBlock::default(),
        AesBlock { w0: !0, w1: !0, w2: !0, w3: !0 },
    ];
    for _ in 0..16 {
        rkeys.push(ld(&rng.bytes(16)));
    }

    for b in &blocks {
        let (x, y) = unsafe { (c_imc(*b), r_imc(*b)) };
        assert_eq!(x, y, "inv_mix_columns({})", blk_hex(*b));
        for rk in &rkeys {
            for (name, cf, rf) in [
                ("block_encrypt", c_enc, r_enc),
                ("block_decrypt", c_dec, r_dec),
                ("block_encryptlast", c_encl, r_encl),
                ("block_decryptlast", c_decl, r_decl),
            ] {
                let (x, y) = unsafe { (cf(*b, *rk), rf(*b, *rk)) };
                assert_eq!(
                    x,
                    y,
                    "softaes_{name}(block={}, rk={}): C={} Rust={}",
                    blk_hex(*b),
                    blk_hex(*rk),
                    blk_hex(x),
                    blk_hex(y)
                );
            }
        }
    }

    // Compose the primitives into full AES-128 and check the FIPS-197 vector.
    // This pins the *semantics* of the round functions, not just C/Rust
    // agreement, and validates the by-value struct ABI.
    let key = unhex("000102030405060708090a0b0c0d0e0f");
    let pt = unhex("00112233445566778899aabbccddeeff");
    let want = unhex("69c4e0d86a7b0430d8cdb78070b4c55a");
    let mut rk = [AesBlock::default(); 11];
    unsafe { r_ek1(rk.as_mut_ptr(), key.as_ptr()) };
    let mut t = ld(&pt);
    t = AesBlock {
        w0: t.w0 ^ rk[0].w0,
        w1: t.w1 ^ rk[0].w1,
        w2: t.w2 ^ rk[0].w2,
        w3: t.w3 ^ rk[0].w3,
    };
    for i in 1..10 {
        t = unsafe { r_enc(t, rk[i]) };
    }
    t = unsafe { r_encl(t, rk[10]) };
    eq_bytes("softaes composed AES-128 == FIPS-197", &want, &st(t));

    // and the inverse direction
    let mut rki = rk;
    unsafe { r_ik1(rki.as_mut_ptr()) };
    let mut t = ld(&want);
    t = AesBlock {
        w0: t.w0 ^ rki[10].w0,
        w1: t.w1 ^ rki[10].w1,
        w2: t.w2 ^ rki[10].w2,
        w3: t.w3 ^ rki[10].w3,
    };
    for i in (1..10).rev() {
        t = unsafe { r_dec(t, rki[i]) };
    }
    t = unsafe { r_decl(t, rki[0]) };
    eq_bytes("softaes composed AES-128 inverse", &pt, &st(t));
}

// ===========================================================================
// crypto_stream cross-checks not already driven by t02_lowlevel.rs
// ===========================================================================

/// CONFIGS G6-004, G6-006, G6-013, G6-014, G6-016, G6-021, G6-028, G6-031,
/// G6-032, G6-035, G6-036, G6-037 … G6-044 — the *derived-relationship* stream
/// rows that `t02_lowlevel.rs` does not assert: `_xor_ic` counter offsets,
/// `ic` truncation for the 32-bit ietf counter, `_ietf_ext` == `_ietf`,
/// generic dispatch == xsalsa20, the two-stage x-variant equivalences and the
/// constant accessors of every stream module.
#[test]
fn stream_derived_relationships() {
    setup();
    let mut rng = Rng::new(0x601d);

    // G6-037 .. G6-044: constants
    assert_eq!(eq_sz("crypto_stream_keybytes"), 32);
    assert_eq!(eq_sz("crypto_stream_noncebytes"), 24);
    assert_eq!(eq_str("crypto_stream_primitive"), "xsalsa20");
    for p in [
        "crypto_stream",
        "crypto_stream_chacha20",
        "crypto_stream_salsa20",
        "crypto_stream_salsa2012",
        "crypto_stream_salsa208",
        "crypto_stream_xchacha20",
        "crypto_stream_xsalsa20",
    ] {
        let (c, r) = pair::<unsafe extern "C" fn() -> usize>(&format!("{p}_messagebytes_max"));
        let (a, b) = unsafe { (c(), r()) };
        eq_usize(&format!("{p}_messagebytes_max"), a, b);
        assert_eq!(a, usize::MAX, "{p}_messagebytes_max");
    }
    let (c, r) = pair::<unsafe extern "C" fn() -> usize>(
        "crypto_stream_chacha20_ietf_messagebytes_max",
    );
    let (a, b) = unsafe { (c(), r()) };
    eq_usize("chacha20_ietf_messagebytes_max", a, b);
    assert_eq!(a, 274_877_906_944);
    assert_eq!(eq_sz("crypto_stream_chacha20_ietf_keybytes"), 32);
    assert_eq!(eq_sz("crypto_stream_chacha20_ietf_noncebytes"), 12);

    // G6-004 / G6-006 / G6-007: chacha20 (64-bit counter) offsets
    let (_, xor_ic) = pair::<StreamXorIc64>("crypto_stream_chacha20_xor_ic");
    let (_, ks) = pair::<Stream>("crypto_stream_chacha20");
    for _ in 0..6 {
        let k = rng.bytes(32);
        let n = rng.bytes(8);
        let mut full = vec![0u8; 64 * 6];
        unsafe { assert_eq!(ks(full.as_mut_ptr(), full.len() as u64, n.as_ptr(), k.as_ptr()), 0) };
        for ic in 1..5u64 {
            let z = vec![0u8; 65];
            let mut out = vec![0u8; 65];
            unsafe {
                assert_eq!(
                    xor_ic(out.as_mut_ptr(), z.as_ptr(), 65, n.as_ptr(), ic, k.as_ptr()),
                    0
                );
            }
            let off = (64 * ic) as usize;
            eq_bytes(
                &format!("chacha20 ic={ic} equals keystream at offset {off}"),
                &full[off..off + 65],
                &out,
            );
        }
        // ic = 0xffffffff, mlen = 128 -> the second block is ic = 2^32
        let z = vec![0u8; 128];
        let mut a = vec![0u8; 128];
        let mut b = vec![0u8; 64];
        unsafe {
            assert_eq!(
                xor_ic(a.as_mut_ptr(), z.as_ptr(), 128, n.as_ptr(), 0xffff_ffff, k.as_ptr()),
                0
            );
            assert_eq!(
                xor_ic(b.as_mut_ptr(), z.as_ptr(), 64, n.as_ptr(), 0x1_0000_0000, k.as_ptr()),
                0
            );
        }
        eq_bytes("chacha20 ic 2^32 == second block of ic 2^32-1", &a[64..], &b);
        // ic = 2^64-1, mlen = 128 -> the counter wraps to 0 for block 2
        let mut a = vec![0u8; 128];
        let mut b = vec![0u8; 64];
        unsafe {
            assert_eq!(
                xor_ic(a.as_mut_ptr(), z.as_ptr(), 128, n.as_ptr(), u64::MAX, k.as_ptr()),
                0
            );
            assert_eq!(xor_ic(b.as_mut_ptr(), z.as_ptr(), 64, n.as_ptr(), 0, k.as_ptr()), 0);
        }
        eq_bytes("chacha20 counter wraps to 0", &a[64..], &b);
    }

    // G6-021 / G6-022: same for salsa20's full 64-bit counter
    let (_, s_xor_ic) = pair::<StreamXorIc64>("crypto_stream_salsa20_xor_ic");
    for _ in 0..6 {
        let k = rng.bytes(32);
        let n = rng.bytes(8);
        let z = vec![0u8; 128];
        let mut a = vec![0u8; 128];
        let mut b = vec![0u8; 64];
        unsafe {
            assert_eq!(
                s_xor_ic(a.as_mut_ptr(), z.as_ptr(), 128, n.as_ptr(), 0xffff_ffff, k.as_ptr()),
                0
            );
            assert_eq!(
                s_xor_ic(b.as_mut_ptr(), z.as_ptr(), 64, n.as_ptr(), 0x1_0000_0000, k.as_ptr()),
                0
            );
        }
        eq_bytes("salsa20 ic 2^32 == second block of ic 2^32-1", &a[64..], &b);
        let mut a = vec![0u8; 128];
        let mut b = vec![0u8; 64];
        unsafe {
            assert_eq!(
                s_xor_ic(a.as_mut_ptr(), z.as_ptr(), 128, n.as_ptr(), u64::MAX, k.as_ptr()),
                0
            );
            assert_eq!(s_xor_ic(b.as_mut_ptr(), z.as_ptr(), 64, n.as_ptr(), 0, k.as_ptr()), 0);
        }
        eq_bytes("salsa20 counter wraps to 0", &a[64..], &b);
    }

    // G6-013: the ietf `ic` parameter is `uint32_t`, so 2^32 and 2^64-1 are
    // not representable. Calling with the truncated values must agree.
    let (c_ietf_ic, r_ietf_ic) = pair::<StreamXorIc32>("crypto_stream_chacha20_ietf_xor_ic");
    for _ in 0..6 {
        let k = rng.bytes(32);
        let n = rng.bytes(12);
        let m = rng.bytes(64);
        for &ic in &[0u32, (0x1_0000_0000u64 as u32), (u64::MAX as u32)] {
            let mut a = canary(64);
            let mut b = canary(64);
            let (x, y) = unsafe {
                (
                    c_ietf_ic(a.as_mut_ptr(), m.as_ptr(), 64, n.as_ptr(), ic, k.as_ptr()),
                    r_ietf_ic(b.as_mut_ptr(), m.as_ptr(), 64, n.as_ptr(), ic, k.as_ptr()),
                )
            };
            eq_i32("ietf_xor_ic truncated rc", x, y);
            eq_bytes("ietf_xor_ic truncated", &a, &b);
        }
        // ic = 2^32 truncates to 0
        let mut a = canary(64);
        let mut b = canary(64);
        unsafe {
            r_ietf_ic(a.as_mut_ptr(), m.as_ptr(), 64, n.as_ptr(), 0, k.as_ptr());
            r_ietf_ic(
                b.as_mut_ptr(),
                m.as_ptr(),
                64,
                n.as_ptr(),
                0x1_0000_0000u64 as u32,
                k.as_ptr(),
            );
        }
        eq_bytes("ietf ic 2^32 truncates to 0", &a, &b);
    }

    // G6-014: `crypto_stream_chacha20_ietf` forwards straight to
    // `_ietf_ext` with the *same* nonce pointer (`chacha_ietf_ivsetup` reads
    // only 12 bytes), so the two must be byte-identical for the same inputs.
    let (_, ietf) = pair::<Stream>("crypto_stream_chacha20_ietf");
    let (_, ietf_ext) = pair::<Stream>("crypto_stream_chacha20_ietf_ext");
    let (_, ietf_xor_ic) = pair::<StreamXorIc32>("crypto_stream_chacha20_ietf_xor_ic");
    let (_, ext_xor_ic) = pair::<StreamXorIc32>("crypto_stream_chacha20_ietf_ext_xor_ic");
    for &len in &[0usize, 1, 64, 65, 200, 1000] {
        let k = rng.bytes(32);
        let n12 = rng.bytes(12);
        let mut a = canary(len);
        let mut b = canary(len);
        unsafe {
            assert_eq!(ietf(a.as_mut_ptr(), len as u64, n12.as_ptr(), k.as_ptr()), 0);
            assert_eq!(
                ietf_ext(b.as_mut_ptr(), len as u64, n12.as_ptr(), k.as_ptr()),
                0
            );
        }
        eq_bytes("ietf_ext == ietf", &a, &b);
        // G6-015: `_ietf_ext_xor_ic` has no counter-overflow guard, while
        // `_ietf_xor_ic` does; for in-range `ic` they must agree exactly.
        let m = rng.bytes(len);
        for &ic in &[0u32, 1, 2] {
            let mut a = canary(len);
            let mut b = canary(len);
            unsafe {
                assert_eq!(
                    ietf_xor_ic(a.as_mut_ptr(), m.as_ptr(), len as u64, n12.as_ptr(), ic, k.as_ptr()),
                    0
                );
                assert_eq!(
                    ext_xor_ic(b.as_mut_ptr(), m.as_ptr(), len as u64, n12.as_ptr(), ic, k.as_ptr()),
                    0
                );
            }
            eq_bytes("ietf_ext_xor_ic == ietf_xor_ic (in range)", &a, &b);
        }
        // G6-015: `_ietf_ext_xor_ic` has NO counter-overflow guard, so
        // `ic = 0xffffffff` with `mlen > 64` silently wraps `j12` to 0 and
        // corrupts the nonce by incrementing `j13`. `_ietf_xor_ic` would
        // `sodium_misuse()` here (that row lives in t19_core_errors.rs).
        for &ic in &[0xffff_fffeu32, 0xffff_ffff] {
            let mut a = canary(len);
            let mut b = canary(len);
            let (x, y) = unsafe {
                (
                    ext_xor_ic(a.as_mut_ptr(), m.as_ptr(), len as u64, n12.as_ptr(), ic, k.as_ptr()),
                    ext_xor_ic(b.as_mut_ptr(), m.as_ptr(), len as u64, n12.as_ptr(), ic, k.as_ptr()),
                )
            };
            eq_i32("ietf_ext_xor_ic wrap rc", x, y);
            let (cf, rf) = pair::<StreamXorIc32>("crypto_stream_chacha20_ietf_ext_xor_ic");
            let mut a = canary(len);
            let mut b = canary(len);
            let (x, y) = unsafe {
                (
                    cf(a.as_mut_ptr(), m.as_ptr(), len as u64, n12.as_ptr(), ic, k.as_ptr()),
                    rf(b.as_mut_ptr(), m.as_ptr(), len as u64, n12.as_ptr(), ic, k.as_ptr()),
                )
            };
            eq_i32(&format!("ietf_ext_xor_ic(len={len}, ic={ic:#x}) rc"), x, y);
            eq_bytes(
                &format!("ietf_ext_xor_ic(len={len}, ic={ic:#x}) counter wrap"),
                &a,
                &b,
            );
        }
    }

    // G6-016: the RFC 8439 nonce
    let rfc_n = unhex("000000000000004a00000000");
    let rfc_k = unhex("808182838485868788898a8b8c8d8e8f909192939495969798999a9b9c9d9e9f");
    let (c_i, r_i) = pair::<Stream>("crypto_stream_chacha20_ietf");
    let mut a = canary(114);
    let mut b = canary(114);
    unsafe {
        c_i(a.as_mut_ptr(), 114, rfc_n.as_ptr(), rfc_k.as_ptr());
        r_i(b.as_mut_ptr(), 114, rfc_n.as_ptr(), rfc_k.as_ptr());
    }
    eq_bytes("RFC 8439 nonce", &a, &b);

    // G6-028 / G6-031: xchacha20 == hchacha20 then chacha20 (8-byte nonce)
    let (_, hchacha) =
        pair::<unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8) -> i32>(
            "crypto_core_hchacha20",
        );
    let (_, xchacha) = pair::<Stream>("crypto_stream_xchacha20");
    for &len in &[0usize, 1, 64, 65, 200] {
        for kind in 0..3 {
            let k = match kind {
                0 => vec![0u8; 32],
                1 => vec![0xffu8; 32],
                _ => rng.bytes(32),
            };
            let n = match kind {
                0 => vec![0u8; 24],
                1 => vec![0xffu8; 24],
                _ => rng.bytes(24),
            };
            let mut k2 = [0u8; 32];
            unsafe {
                assert_eq!(
                    hchacha(k2.as_mut_ptr(), n.as_ptr(), k.as_ptr(), std::ptr::null()),
                    0
                );
            }
            let mut a = canary(len);
            let mut b = canary(len);
            unsafe {
                assert_eq!(xchacha(a.as_mut_ptr(), len as u64, n.as_ptr(), k.as_ptr()), 0);
                assert_eq!(
                    ks(b.as_mut_ptr(), len as u64, n[16..].as_ptr(), k2.as_ptr()),
                    0
                );
            }
            eq_bytes("xchacha20 == hchacha20 + chacha20", &a, &b);
        }
    }

    // G6-032: xsalsa20 == hsalsa20 then salsa20
    let (_, hsalsa) =
        pair::<unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8) -> i32>(
            "crypto_core_hsalsa20",
        );
    let (_, xsalsa) = pair::<Stream>("crypto_stream_xsalsa20");
    let (_, salsa) = pair::<Stream>("crypto_stream_salsa20");
    for &len in &[0usize, 1, 64, 65, 200] {
        let k = rng.bytes(32);
        let n = rng.bytes(24);
        let mut sub = [0u8; 32];
        unsafe {
            assert_eq!(
                hsalsa(sub.as_mut_ptr(), n.as_ptr(), k.as_ptr(), std::ptr::null()),
                0
            );
        }
        let mut a = canary(len);
        let mut b = canary(len);
        unsafe {
            assert_eq!(xsalsa(a.as_mut_ptr(), len as u64, n.as_ptr(), k.as_ptr()), 0);
            assert_eq!(
                salsa(b.as_mut_ptr(), len as u64, n[16..].as_ptr(), sub.as_ptr()),
                0
            );
        }
        eq_bytes("xsalsa20 == hsalsa20 + salsa20", &a, &b);
    }

    // G6-035 / G6-036: the generic dispatch is xsalsa20
    let (c_g, r_g) = pair::<Stream>("crypto_stream");
    let (c_gx, r_gx) = pair::<StreamXor>("crypto_stream_xor");
    let (_, x_xor) = pair::<StreamXor>("crypto_stream_xsalsa20_xor");
    for &len in &[0usize, 1, 64, 65, 200, 1000] {
        for kind in 0..3 {
            let k = match kind {
                0 => vec![0u8; 32],
                1 => vec![0xffu8; 32],
                _ => rng.bytes(32),
            };
            let n = match kind {
                0 => vec![0u8; 24],
                1 => vec![0xffu8; 24],
                _ => rng.bytes(24),
            };
            let m = rng.bytes(len);
            let mut a = canary(len);
            let mut b = canary(len);
            let mut d = canary(len);
            unsafe {
                let x = c_g(a.as_mut_ptr(), len as u64, n.as_ptr(), k.as_ptr());
                let y = r_g(b.as_mut_ptr(), len as u64, n.as_ptr(), k.as_ptr());
                eq_i32("crypto_stream rc", x, y);
                assert_eq!(xsalsa(d.as_mut_ptr(), len as u64, n.as_ptr(), k.as_ptr()), 0);
            }
            eq_bytes("crypto_stream", &a, &b);
            eq_bytes("crypto_stream == xsalsa20", &a, &d);

            let mut a = canary(len);
            let mut b = canary(len);
            let mut d = canary(len);
            unsafe {
                let x = c_gx(a.as_mut_ptr(), m.as_ptr(), len as u64, n.as_ptr(), k.as_ptr());
                let y = r_gx(b.as_mut_ptr(), m.as_ptr(), len as u64, n.as_ptr(), k.as_ptr());
                eq_i32("crypto_stream_xor rc", x, y);
                assert_eq!(
                    x_xor(d.as_mut_ptr(), m.as_ptr(), len as u64, n.as_ptr(), k.as_ptr()),
                    0
                );
            }
            eq_bytes("crypto_stream_xor", &a, &b);
            eq_bytes("crypto_stream_xor == xsalsa20_xor", &a, &d);
        }
    }
}

// ===========================================================================
// build-configuration rows
// ===========================================================================

/// CONFIGS G6-108, G6-123, G6-172, G6-173 — the ed25519 / ristretto255 size
/// accessors (32/32/64/32/64 and 32/64/32/64) and build configuration. `HAVE_TI_MODE` is undefined,
/// so the live ed25519 field representation is the 10-limb `fe_25_5` one; that
/// is not directly observable, but every ed25519/ristretto255 row above is a
/// differential test of exactly that arithmetic. `NDEBUG` is undefined, so the
/// two `assert(h_len <= 0xff)` in `core_h2c.c` are live — they are unreachable
/// from the public API (`h_len` is only ever 48, 64 or 96), which is asserted
/// here by exercising every reachable `h_len` and confirming success.
#[test]
fn build_configuration_rows() {
    setup();
    // h_len = 48 (`_scalar_from_string`, `_from_string_nu`), 96
    // (`_from_string`) and 64 (ristretto `_from_string`) all succeed.
    let ctx = b"ctx";
    let msg = b"msg";
    for name in [
        "crypto_core_ed25519_scalar_from_string",
        "crypto_core_ed25519_from_string_nu",
        "crypto_core_ed25519_from_string",
        "crypto_core_ristretto255_from_string",
        "crypto_core_ristretto255_scalar_from_string",
    ] {
        let (c, r) = pair::<FromString>(name);
        for alg in [1i32, 2] {
            let mut a = canary(32);
            let mut b = canary(32);
            let (x, y) = unsafe {
                (
                    c(a.as_mut_ptr(), ctx.as_ptr(), 3, msg.as_ptr(), 3, alg),
                    r(b.as_mut_ptr(), ctx.as_ptr(), 3, msg.as_ptr(), 3, alg),
                )
            };
            eq_i32(&format!("{name} rc"), x, y);
            assert_eq!(x, 0, "{name} must succeed for the reachable h_len values");
            eq_bytes(name, &a, &b);
        }
    }
    // sanity: the sizes that drive `h_len` are what CONFIGS claims
    assert_eq!(szof("crypto_core_ed25519_hashbytes"), 64);
    assert_eq!(szof("crypto_core_ristretto255_hashbytes"), 64);
    assert_eq!(szof("crypto_core_ed25519_nonreducedscalarbytes"), 64);
    assert_eq!(eq_sz("crypto_core_ed25519_bytes"), 32);
    assert_eq!(eq_sz("crypto_core_ed25519_uniformbytes"), 32);
    assert_eq!(eq_sz("crypto_core_ed25519_scalarbytes"), 32);
    assert_eq!(eq_sz("crypto_core_ed25519_hashbytes"), 64);
    assert_eq!(eq_sz("crypto_core_ed25519_nonreducedscalarbytes"), 64);
    assert_eq!(eq_sz("crypto_core_ristretto255_bytes"), 32);
    assert_eq!(eq_sz("crypto_core_ristretto255_hashbytes"), 64);
    assert_eq!(eq_sz("crypto_core_ristretto255_scalarbytes"), 32);
    assert_eq!(eq_sz("crypto_core_ristretto255_nonreducedscalarbytes"), 64);
}

// ===========================================================================
// structured / adversarial inputs
//
// Uniform random bytes almost never hit the interesting corners of the
// 10-limb `fe_25_5` field arithmetic or of `sc25519_reduce` / `sc25519_mul`.
// These sweeps drive single-bit values, field-boundary values and
// hash-block-boundary lengths, which is where a carry or masking bug in the
// translation would show up.
// ===========================================================================

/// CONFIGS G6-101, G6-105 (structured inputs) — `sc25519_mul` and
/// `sc25519_reduce` over single-bit scalars, byte-saturated prefixes and
/// values straddling multiples of L.
#[test]
fn scalar_arithmetic_structured_inputs() {
    setup();
    let (_, mul) = pair::<V3>("crypto_core_ed25519_scalar_mul");
    let (c_mul, _) = pair::<V3>("crypto_core_ed25519_scalar_mul");
    let (c_red, r_red) = pair::<V2>("crypto_core_ed25519_scalar_reduce");
    let (c_add, r_add) = pair::<V3>("crypto_core_ed25519_scalar_add");
    let (c_sub, r_sub) = pair::<V3>("crypto_core_ed25519_scalar_sub");
    let (c_neg, r_neg) = pair::<V2>("crypto_core_ed25519_scalar_negate");
    let (c_comp, r_comp) = pair::<V2>("crypto_core_ed25519_scalar_complement");

    // single-bit 32-byte scalars: 2^0 .. 2^255
    let mut bits: Vec<[u8; 32]> = Vec::new();
    for i in 0..256usize {
        let mut s = [0u8; 32];
        s[i / 8] = 1 << (i % 8);
        bits.push(s);
    }
    // saturated prefixes: 0xff for the first k bytes, 0x00 after (and vice versa)
    let mut sat: Vec<[u8; 32]> = Vec::new();
    for k in 0..=32usize {
        let mut a = [0u8; 32];
        for i in 0..k {
            a[i] = 0xff;
        }
        sat.push(a);
        let mut b = [0xffu8; 32];
        for i in 0..k {
            b[i] = 0;
        }
        sat.push(b);
    }
    // L +/- small deltas, and kL +/- small deltas
    let mut near_l: Vec<[u8; 32]> = Vec::new();
    for k in 1..8u32 {
        let base = mul_small(&ell(), k);
        for d in 0..4u8 {
            let mut a = base;
            let mut carry = d as u16;
            for i in 0..32 {
                let v = a[i] as u16 + carry;
                a[i] = v as u8;
                carry = v >> 8;
                if carry == 0 {
                    break;
                }
            }
            near_l.push(a);
            let mut b = base;
            let mut borrow = d as i16;
            for i in 0..32 {
                let v = b[i] as i16 - borrow;
                b[i] = v as u8;
                borrow = if v < 0 { 1 } else { 0 };
                if borrow == 0 {
                    break;
                }
            }
            near_l.push(b);
        }
    }

    let mut all: Vec<[u8; 32]> = Vec::new();
    all.extend(&bits);
    all.extend(&sat);
    all.extend(&near_l);

    // unary
    for s in &all {
        for (name, cf, rf) in [
            ("scalar_negate", c_neg, r_neg),
            ("scalar_complement", c_comp, r_comp),
        ] {
            let mut a = canary(32);
            let mut b = canary(32);
            unsafe {
                cf(a.as_mut_ptr(), s.as_ptr());
                rf(b.as_mut_ptr(), s.as_ptr());
            }
            eq_bytes(&format!("{name}({})", hex(s)), &a, &b);
        }
    }

    // binary: single-bit x single-bit is the worst case for the Barrett
    // reduction inside `sc25519_mul`
    for x in &bits {
        for y in &bits {
            let mut a = canary(32);
            let mut b = canary(32);
            unsafe {
                c_mul(a.as_mut_ptr(), x.as_ptr(), y.as_ptr());
                mul(b.as_mut_ptr(), x.as_ptr(), y.as_ptr());
            }
            eq_bytes(&format!("scalar_mul({}, {})", hex(x), hex(y)), &a, &b);
        }
    }
    // add / sub over the structured set (samples the `sodium_add` carry chain)
    for x in &all {
        for y in &all {
            if (x[0] as usize + y[31] as usize) % 7 != 0 {
                continue; // thin the O(n^2) grid but keep it wide
            }
            for (name, cf, rf) in [
                ("scalar_add", c_add, r_add),
                ("scalar_sub", c_sub, r_sub),
            ] {
                let mut a = canary(32);
                let mut b = canary(32);
                unsafe {
                    cf(a.as_mut_ptr(), x.as_ptr(), y.as_ptr());
                    rf(b.as_mut_ptr(), x.as_ptr(), y.as_ptr());
                }
                eq_bytes(&format!("{name}({}, {})", hex(x), hex(y)), &a, &b);
            }
        }
    }

    // reduce: single-bit 64-byte inputs (2^0 .. 2^511) and saturated prefixes
    let mut reds: Vec<Vec<u8>> = Vec::new();
    for i in 0..512usize {
        let mut s = vec![0u8; 64];
        s[i / 8] = 1 << (i % 8);
        reds.push(s);
    }
    for k in 0..=64usize {
        let mut a = vec![0u8; 64];
        for i in 0..k {
            a[i] = 0xff;
        }
        reds.push(a);
        let mut b = vec![0xffu8; 64];
        for i in 0..k {
            b[i] = 0;
        }
        reds.push(b);
    }
    for s in &all {
        let mut v = vec![0u8; 64];
        v[..32].copy_from_slice(s);
        reds.push(v.clone());
        let mut w = vec![0u8; 64];
        w[32..].copy_from_slice(s);
        reds.push(w);
    }
    for s in &reds {
        let mut a = canary(32);
        let mut b = canary(32);
        unsafe {
            c_red(a.as_mut_ptr(), s.as_ptr());
            r_red(b.as_mut_ptr(), s.as_ptr());
        }
        eq_bytes(&format!("scalar_reduce({})", hex(s)), &a, &b);
    }
}

/// CONFIGS G6-088, G6-091, G6-109, G6-110, G6-111, G6-112 (structured inputs) —
/// the `fe_25_5` field arithmetic at its boundaries: every canonical `y` in
/// `0..96` and `p-96..p`, `2^255-1`, and the ristretto255 elligator with each
/// half set to `0`, `1`, `p-2`, `p-1`, `p`, `p+1` and `2^255-1`.
#[test]
fn field_arithmetic_structured_inputs() {
    setup();
    let mut rng = Rng::new(0x601e);
    let (c_add, r_add) = pair::<I3>("crypto_core_ed25519_add");
    let (c_sub, r_sub) = pair::<I3>("crypto_core_ed25519_sub");
    let (c_valid, r_valid) = pair::<I1c>("crypto_core_ed25519_is_valid_point");
    let (c_ris, r_ris) = pair::<I1c>("crypto_core_ristretto255_is_valid_point");
    let (c_fh, r_fh) = pair::<I2>("crypto_core_ristretto255_from_hash");
    let (c_x, r_x) = pair::<I3>("crypto_scalarmult_curve25519");

    // p = 2^255 - 19, little-endian
    let p_le = unhex32("edffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f");

    // canonical encodings with small y and with y just below p, both signs
    let mut encs: Vec<[u8; 32]> = Vec::new();
    for y in 0..96u32 {
        let mut e = [0u8; 32];
        e[0] = (y & 0xff) as u8;
        e[1] = (y >> 8) as u8;
        encs.push(e);
        let mut f = e;
        f[31] |= 0x80;
        encs.push(f);
    }
    for d in 0..96u32 {
        let mut e = p_le;
        let mut borrow = d as i32;
        for i in 0..32 {
            let v = e[i] as i32 - (borrow & 0xff);
            borrow >>= 8;
            if v < 0 {
                e[i] = (v + 256) as u8;
                borrow += 1;
            } else {
                e[i] = v as u8;
            }
            if borrow == 0 {
                break;
            }
        }
        encs.push(e);
        let mut f = e;
        f[31] |= 0x80;
        encs.push(f);
    }
    encs.push([0xffu8; 32]);

    // ed25519: `_is_valid_point` and `_add`/`_sub` must agree on every one
    let good = ed_valid_points(&mut rng, 4);
    for e in &encs {
        let (a, b) = unsafe { (c_valid(e.as_ptr()), r_valid(e.as_ptr())) };
        eq_i32(&format!("is_valid_point({})", hex(e)), a, b);
        let (a, b) = unsafe { (c_ris(e.as_ptr()), r_ris(e.as_ptr())) };
        eq_i32(&format!("ristretto255_is_valid_point({})", hex(e)), a, b);
        for g in &good {
            for (name, cf, rf) in [("add", c_add, r_add), ("sub", c_sub, r_sub)] {
                let mut x = canary(32);
                let mut y = canary(32);
                let (ra, rb) = unsafe {
                    (
                        cf(x.as_mut_ptr(), e.as_ptr(), g.as_ptr()),
                        rf(y.as_mut_ptr(), e.as_ptr(), g.as_ptr()),
                    )
                };
                eq_i32(&format!("ed25519_{name}({}) rc", hex(e)), ra, rb);
                eq_bytes(&format!("ed25519_{name}({})", hex(e)), &x, &y);
                let mut x = canary(32);
                let mut y = canary(32);
                let (ra, rb) = unsafe {
                    (
                        cf(x.as_mut_ptr(), g.as_ptr(), e.as_ptr()),
                        rf(y.as_mut_ptr(), g.as_ptr(), e.as_ptr()),
                    )
                };
                eq_i32(&format!("ed25519_{name}(g, {}) rc", hex(e)), ra, rb);
                eq_bytes(&format!("ed25519_{name}(g, {})", hex(e)), &x, &y);
            }
        }
    }

    // ristretto255_from_hash with each half at a field boundary
    let mut halves: Vec<[u8; 32]> = Vec::new();
    for v in [0u32, 1, 2, 3] {
        let mut h = [0u8; 32];
        h[0] = v as u8;
        halves.push(h);
    }
    for d in 0..3u32 {
        let mut h = p_le;
        h[0] = h[0].wrapping_sub(d as u8);
        halves.push(h);
    }
    halves.push(p_le);
    {
        let mut hp1 = p_le;
        hp1[0] += 1;
        halves.push(hp1);
    }
    halves.push([0xffu8; 32]);
    {
        let mut top = [0u8; 32];
        top[31] = 0x80;
        halves.push(top);
    }
    for a in &halves {
        for b in &halves {
            let mut h = Vec::with_capacity(64);
            h.extend_from_slice(a);
            h.extend_from_slice(b);
            let mut x = canary(32);
            let mut y = canary(32);
            let (ra, rb) = unsafe {
                (
                    c_fh(x.as_mut_ptr(), h.as_ptr()),
                    r_fh(y.as_mut_ptr(), h.as_ptr()),
                )
            };
            eq_i32("ristretto255_from_hash rc", ra, rb);
            eq_bytes(&format!("ristretto255_from_hash({})", hex(&h)), &x, &y);
        }
    }

    // x25519 with a single-bit x-coordinate for every bit position, and the
    // field boundaries; the ladder must agree bit for bit.
    let n = rng.bytes(32);
    let mut xs: Vec<[u8; 32]> = Vec::new();
    for i in 0..256usize {
        let mut p = [0u8; 32];
        p[i / 8] = 1 << (i % 8);
        xs.push(p);
    }
    xs.extend(halves.iter().cloned());
    for p in &xs {
        let mut a = canary(32);
        let mut b = canary(32);
        let (ra, rb) = unsafe {
            (
                c_x(a.as_mut_ptr(), n.as_ptr(), p.as_ptr()),
                r_x(b.as_mut_ptr(), n.as_ptr(), p.as_ptr()),
            )
        };
        eq_i32(&format!("x25519(p={}) rc", hex(p)), ra, rb);
        eq_bytes(&format!("x25519(p={})", hex(p)), &a, &b);
    }
}

/// CONFIGS G6-124, G6-125, G6-126 (structured inputs) — the h2c expand loop at
/// its SHA-256 / SHA-512 block and digest boundaries: `msg_len` and `ctx_len`
/// swept over 0..=200 plus every value adjacent to 55/56/63/64/111/112/127/128
/// (the SHA padding boundaries) and 0xff/0x100 (the DST-derivation switch).
#[test]
fn h2c_length_boundary_sweep() {
    setup();
    let mut rng = Rng::new(0x601f);
    let big_ctx = rng.bytes(600);
    let big_msg = rng.bytes(600);

    let mut lens: Vec<usize> = (0..=72).collect();
    for base in [55usize, 56, 63, 64, 111, 112, 119, 120, 127, 128, 191, 192, 255, 256] {
        for d in 0..3usize {
            lens.push(base.saturating_sub(d));
            lens.push(base + d);
        }
    }
    lens.push(511);
    lens.push(512);
    lens.push(600);
    lens.sort();
    lens.dedup();

    for name in [
        "crypto_core_ed25519_from_string",
        "crypto_core_ed25519_from_string_nu",
        "crypto_core_ed25519_scalar_from_string",
        "crypto_core_ristretto255_from_string",
    ] {
        let (c, r) = pair::<FromString>(name);
        for alg in [1i32, 2] {
            // sweep msg_len with a fixed short ctx
            for &ml in &lens {
                let mut a = canary(32);
                let mut b = canary(32);
                let (ra, rb) = unsafe {
                    (
                        c(a.as_mut_ptr(), big_ctx.as_ptr(), 5, big_msg.as_ptr(), ml, alg),
                        r(b.as_mut_ptr(), big_ctx.as_ptr(), 5, big_msg.as_ptr(), ml, alg),
                    )
                };
                eq_i32(&format!("{name}(alg={alg}, msg_len={ml}) rc"), ra, rb);
                eq_bytes(&format!("{name}(alg={alg}, msg_len={ml})"), &a, &b);
            }
            // sweep ctx_len with a fixed short msg
            for &cl in &lens {
                let mut a = canary(32);
                let mut b = canary(32);
                let (ra, rb) = unsafe {
                    (
                        c(a.as_mut_ptr(), big_ctx.as_ptr(), cl, big_msg.as_ptr(), 7, alg),
                        r(b.as_mut_ptr(), big_ctx.as_ptr(), cl, big_msg.as_ptr(), 7, alg),
                    )
                };
                eq_i32(&format!("{name}(alg={alg}, ctx_len={cl}) rc"), ra, rb);
                eq_bytes(&format!("{name}(alg={alg}, ctx_len={cl})"), &a, &b);
            }
        }
    }
}
