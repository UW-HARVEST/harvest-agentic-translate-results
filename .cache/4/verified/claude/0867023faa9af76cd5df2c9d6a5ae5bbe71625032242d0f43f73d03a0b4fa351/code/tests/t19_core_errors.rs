//! Phase C — error / rejection paths for the **G6** module group
//! (`ERRORS.md` section `## G6`, rows G6-001 … G6-149).
//!
//! Three kinds of row:
//!
//! * **`return -1` / `return 0` rows** — called directly on both `.so`s. The
//!   return value *and* the output buffer (canary-filled, so "untouched" and
//!   "clobbered with a valid point" are both observable) must match exactly.
//! * **`errno = EINVAL` rows** — `core_h2c_string_to_hash`'s `default:` case.
//!   `errno` is cleared before each call and compared afterwards.
//! * **`sodium_misuse()` rows** — the two reachable ChaCha20-IETF guards. These
//!   `abort()`, so they run in a **child process** (once per library) with the
//!   observing handler installed, and the exit status plus the bytes written
//!   before the abort are compared.
//!
//! Rows whose branch is genuinely unreachable in this build are gathered in
//! `documented_unreachable_error_rows` together with the reason, so no row is
//! silently dropped.

mod common;
use common::*;

// ---------------------------------------------------------------------------
// signatures
// ---------------------------------------------------------------------------

type Sz = unsafe extern "C" fn() -> usize;
type V1 = unsafe extern "C" fn(*mut u8);
type I1c = unsafe extern "C" fn(*const u8) -> i32;
type V2 = unsafe extern "C" fn(*mut u8, *const u8);
type I2 = unsafe extern "C" fn(*mut u8, *const u8) -> i32;
type V3 = unsafe extern "C" fn(*mut u8, *const u8, *const u8);
type I3 = unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> i32;
type V4 = unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8);
type Core = unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8) -> i32;
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
// helpers
// ---------------------------------------------------------------------------

/// `reset_rngs()` touches process-global state shared by both libraries, so
/// tests that use it must not run concurrently.
static RNG_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn rng_guard() -> std::sync::MutexGuard<'static, ()> {
    RNG_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

const EINVAL: i32 = 22;

fn clear_errno() {
    // a syscall that is known to succeed leaves errno alone, so set it to 0
    // explicitly through a successful call and then verify it reads back as 0.
    let _ = std::fs::metadata("/");
    unsafe { *libc_errno() = 0 };
}

fn errno() -> i32 {
    unsafe { *libc_errno() }
}

/// `__errno_location()` from the process' libc (glibc). Both `.so`s and the
/// test binary share it.
fn libc_errno() -> *mut i32 {
    unsafe extern "C" {
        fn __errno_location() -> *mut i32;
    }
    unsafe { __errno_location() }
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

fn eq_sz(name: &str) -> usize {
    let (c, r) = pair::<Sz>(name);
    let (a, b) = unsafe { (c(), r()) };
    eq_usize(name, a, b);
    a
}

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

const ED_BASE: &str = "5866666666666666666666666666666666666666666666666666666666666666";
const RIS_GEN: &str = "e2f2ae0a6abc4e71a884a961c500515f58e30b6aa582dd8db6a65945e08d2d76";
/// The ristretto255 identity.
const IDENTITY_ED: &str = "0100000000000000000000000000000000000000000000000000000000000000";

fn ell() -> [u8; 32] {
    unhex32("edd3f55c1a631258d69cf7a2def9de1400000000000000000000000000000010")
}
fn ell_minus_1() -> [u8; 32] {
    let mut l = ell();
    l[0] -= 1;
    l
}

/// The 7 blocklisted Montgomery x-coordinates of `x25519_ref10.c`, in order.
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

/// The documented ed25519 small-order encodings.
fn ed_small_order() -> Vec<(&'static str, [u8; 32])> {
    vec![
        ("identity", unhex32(IDENTITY_ED)),
        (
            "order-2",
            unhex32("ecffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f"),
        ),
        ("order-4 (00..00)", [0u8; 32]),
        ("order-4 (00..80)", {
            let mut v = [0u8; 32];
            v[31] = 0x80;
            v
        }),
        (
            "order-8 a",
            unhex32("26e8958fc2b227b045c3f489f2ef98f0d5dfac05d3c63339b13802886d53fc05"),
        ),
        (
            "order-8 b",
            unhex32("c7176a703d4dd84fba3c0b760d10670f2a2053fa2c39ccc64ec7fd7792ac03fa"),
        ),
    ]
}

/// A canonical, on-curve ed25519 encoding whose `y` is not on the curve at all:
/// `02 00 .. 00`. `ge25519_frombytes` returns -1 for it.
fn ed_not_on_curve() -> [u8; 32] {
    unhex32("0200000000000000000000000000000000000000000000000000000000000000")
}

/// Non-canonical ed25519 encodings (`ge25519_is_canonical == 0`) whose `y` is
/// **off** the curve, so `ge25519_frombytes` also rejects them.
fn ed_non_canonical() -> Vec<[u8; 32]> {
    vec![
        [0xffu8; 32], // y = 18
        unhex32("efffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f"), // y = 2
    ]
}

/// Non-canonical ed25519 encodings whose `y` *is* on the curve:
/// `ed ff..ff 7f` re-encodes y = 0 and `ee ff..ff 7f` re-encodes y = 1.
/// `crypto_core_ed25519_add`/`_sub` accept these (no canonicality check),
/// while `_is_valid_point` and `crypto_scalarmult_ed25519*` reject them.
fn ed_non_canonical_on_curve() -> Vec<[u8; 32]> {
    vec![
        unhex32("edffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f"),
        unhex32("eeffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f"),
    ]
}

/// Valid ed25519 main-subgroup points, derived without touching the RNG.
fn ed_valid_points(rng: &mut Rng, n: usize) -> Vec<[u8; 32]> {
    let f = sym::<I2>(c_lib(), "crypto_scalarmult_ed25519_base_noclamp");
    let mut out = vec![unhex32(ED_BASE)];
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

/// An on-curve, canonical, non-small-order ed25519 point that is **not** in the
/// prime-order subgroup: `B + T8` for each order-8 generator (`ERRORS` G6-023,
/// G6-052). `crypto_core_ed25519_add` performs no subgroup check, so it is the
/// natural constructor.
fn ed_torsion_points() -> Vec<[u8; 32]> {
    let add = sym::<I3>(c_lib(), "crypto_core_ed25519_add");
    let base = unhex32(ED_BASE);
    let mut out = Vec::new();
    for (_, t) in ed_small_order() {
        let mut r = [0u8; 32];
        if unsafe { add(r.as_mut_ptr(), base.as_ptr(), t.as_ptr()) } == 0 {
            // skip the identity (B + identity == B, which *is* in the subgroup)
            if r != base {
                out.push(r);
            }
        }
    }
    assert!(!out.is_empty(), "could not build an off-subgroup point");
    out
}

/// Valid ristretto255 elements, derived without touching the RNG.
fn ris_valid_points(rng: &mut Rng, n: usize) -> Vec<[u8; 32]> {
    let f = sym::<I2>(c_lib(), "crypto_core_ristretto255_from_hash");
    let mut out = vec![unhex32(RIS_GEN)];
    while out.len() < n {
        let h = rng.bytes(64);
        let mut q = [0u8; 32];
        assert_eq!(unsafe { f(q.as_mut_ptr(), h.as_ptr()) }, 0);
        if q.iter().any(|&b| b != 0) {
            out.push(q);
        }
    }
    out
}

/// Invalid ristretto255 encodings, one per `ristretto255_frombytes` sub-check.
fn ris_invalid_points() -> Vec<(&'static str, [u8; 32])> {
    vec![
        // s[0] & 1 -> non-canonical (odd)
        ("odd", unhex32(IDENTITY_ED)),
        ("odd (random)", unhex32(
            "03ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff00",
        )),
        // bit 255 set -> non-canonical
        ("bit255", unhex32(
            "0000000000000000000000000000000000000000000000000000000000000080",
        )),
        // >= p and even -> non-canonical
        ("s >= p", unhex32(
            "eeffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f",
        )),
        ("s == p-1", unhex32(
            "ecffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f",
        )),
        // canonical + even but not a valid ristretto255 encoding
        // (RFC 9496 A.2 "non-square x^2" vector)
        ("non-square", unhex32(
            "26948d35ca62e643e26a83177332e6b6afeb9d08e4268b650f1f5bbd8d81d371",
        )),
    ]
}

// ===========================================================================
// crypto_scalarmult — curve25519
// ===========================================================================

/// ERRORS G6-001, G6-004, G6-005, G6-006, G6-007, G6-008, G6-009, G6-010,
/// G6-011, G6-012 — `crypto_scalarmult_curve25519` (and the generic
/// `crypto_scalarmult`) reject each of the 7 blocklisted small-order
/// x-coordinates with `-1` and leave `q` untouched; bit 255 of the point is
/// masked before the comparison, so the same 7 values with bit 255 set are
/// rejected too; and all-0xff (which masks to `ff..7f`, matching nothing) is
/// **accepted**.
#[test]
fn scalarmult_curve25519_blocklist() {
    setup();
    let mut rng = Rng::new(0x7101);
    let (c, r) = pair::<I3>("crypto_scalarmult_curve25519");
    let (c_g, r_g) = pair::<I3>("crypto_scalarmult");

    let scalars: Vec<Vec<u8>> = {
        let mut v = vec![vec![0u8; 32], vec![0xffu8; 32], vec![1u8; 32]];
        for _ in 0..12 {
            v.push(rng.bytes(32));
        }
        v
    };

    for (i, p) in x25519_blocklist().iter().enumerate() {
        for n in &scalars {
            for hi in [false, true] {
                let mut pp = *p;
                if hi {
                    pp[31] |= 0x80;
                }
                for (which, cf, rf) in [
                    ("crypto_scalarmult_curve25519", c, r),
                    ("crypto_scalarmult", c_g, r_g),
                ] {
                    let mut a = canary(32);
                    let mut b = canary(32);
                    let (ra, rb) = unsafe {
                        (
                            cf(a.as_mut_ptr(), n.as_ptr(), pp.as_ptr()),
                            rf(b.as_mut_ptr(), n.as_ptr(), pp.as_ptr()),
                        )
                    };
                    eq_i32(
                        &format!("{which} blocklist[{i}] (bit255={hi}) rc"),
                        ra,
                        rb,
                    );
                    assert_eq!(ra, -1, "{which} must reject blocklist[{i}]");
                    eq_bytes(
                        &format!("{which} blocklist[{i}] q untouched"),
                        &a,
                        &b,
                    );
                    assert_eq!(a, canary(32), "{which} must leave q untouched");
                }
            }
        }
    }

    // G6-011 (negative control): all-0xff is NOT blocklisted.
    let allff = [0xffu8; 32];
    for n in &scalars {
        let mut a = canary(32);
        let mut b = canary(32);
        let (ra, rb) = unsafe {
            (
                c(a.as_mut_ptr(), n.as_ptr(), allff.as_ptr()),
                r(b.as_mut_ptr(), n.as_ptr(), allff.as_ptr()),
            )
        };
        eq_i32("x25519(p=all-0xff) rc", ra, rb);
        assert_eq!(ra, 0, "all-0xff must be accepted");
        eq_bytes("x25519(p=all-0xff)", &a, &b);
    }
}

/// ERRORS G6-003, G6-014 — neither `crypto_scalarmult_base` nor
/// `crypto_scalarmult_curve25519_base` has a rejection branch: every scalar,
/// including all-zero and all-0xff, returns 0 with a non-zero `q`.
#[test]
fn scalarmult_curve25519_base_never_fails() {
    setup();
    let mut rng = Rng::new(0x7102);
    for name in [
        "crypto_scalarmult_curve25519_base",
        "crypto_scalarmult_base",
    ] {
        let (c, r) = pair::<I2>(name);
        let mut scalars: Vec<Vec<u8>> = vec![vec![0u8; 32], vec![0xffu8; 32]];
        scalars.push(ell().to_vec());
        scalars.push(ell_minus_1().to_vec());
        let mut hi = vec![0u8; 32];
        hi[31] = 0x80;
        scalars.push(hi);
        for _ in 0..60 {
            scalars.push(rng.bytes(32));
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
            eq_i32(&format!("{name}({}) rc", hex(n)), ra, rb);
            assert_eq!(ra, 0, "{name} must always return 0");
            eq_bytes(&format!("{name}({})", hex(n)), &a, &b);
            assert!(a.iter().any(|&x| x != 0), "{name} produced an all-zero q");
        }
    }
}

// ===========================================================================
// crypto_scalarmult — ed25519
// ===========================================================================

/// ERRORS G6-015, G6-016, G6-017, G6-018, G6-019, G6-020, G6-021, G6-022,
/// G6-023 (clamped) and G6-026, G6-027, G6-028, G6-029 (`_noclamp`) — every
/// point-validation rejection of `_crypto_scalarmult_ed25519`: non-canonical
/// encodings, a canonical `y` that is not on the curve, all six small-order
/// encodings, and an on-curve point of order 8L.
#[test]
fn scalarmult_ed25519_point_rejections() {
    setup();
    let mut rng = Rng::new(0x7103);
    let (c, r) = pair::<I3>("crypto_scalarmult_ed25519");
    let (c_nc, r_nc) = pair::<I3>("crypto_scalarmult_ed25519_noclamp");

    let mut bad: Vec<(String, [u8; 32])> = Vec::new();
    for p in ed_non_canonical().into_iter().chain(ed_non_canonical_on_curve()) {
        bad.push((format!("non-canonical {}", hex(&p)), p));
    }
    bad.push(("y not on curve (02 00..00)".into(), ed_not_on_curve()));
    for (tag, p) in ed_small_order() {
        bad.push((format!("small order: {tag}"), p));
    }
    for (i, p) in ed_torsion_points().iter().enumerate() {
        bad.push((format!("order 8L #{i}"), *p));
    }

    let scalars: Vec<Vec<u8>> = {
        let mut v = vec![vec![1u8; 32], vec![0xffu8; 32]];
        let mut one = vec![0u8; 32];
        one[0] = 1;
        v.push(one);
        for _ in 0..6 {
            v.push(rng.bytes(32));
        }
        v
    };

    for (tag, p) in &bad {
        for n in &scalars {
            for (which, cf, rf) in [
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
                eq_i32(&format!("scalarmult_{which} [{tag}] rc"), ra, rb);
                assert_eq!(ra, -1, "scalarmult_{which} must reject [{tag}]");
                eq_bytes(&format!("scalarmult_{which} [{tag}] q"), &a, &b);
                assert_eq!(
                    a,
                    canary(32),
                    "scalarmult_{which} must leave q untouched for [{tag}]"
                );
            }
        }
    }
}

/// ERRORS G6-025, G6-030, G6-031, G6-032, G6-033, G6-035, G6-036, G6-037,
/// G6-038 — the *scalar* rejections. The clamped forms can only fail via
/// `sodium_is_zero(n)` (after `q` has already been clobbered with the
/// `2^254 * P` result); `_noclamp` additionally reaches the `_is_inf(q)` half
/// with `n = L, 2L, … 7L` and with `n = 00..00 80`.
#[test]
fn scalarmult_ed25519_scalar_rejections() {
    setup();
    let mut rng = Rng::new(0x7104);
    let (c, r) = pair::<I3>("crypto_scalarmult_ed25519");
    let (c_nc, r_nc) = pair::<I3>("crypto_scalarmult_ed25519_noclamp");
    let (c_b, r_b) = pair::<I2>("crypto_scalarmult_ed25519_base");
    let (c_bn, r_bn) = pair::<I2>("crypto_scalarmult_ed25519_base_noclamp");
    let points = ed_valid_points(&mut rng, 6);
    let identity = unhex32(IDENTITY_ED);

    let zero = [0u8; 32];
    let mut top = [0u8; 32];
    top[31] = 0x80;

    // --- n = 32 zero bytes: all four entry points return -1 (G6-025, G6-033,
    //     G6-035, G6-038). The clamped forms clobber `q` with a valid point
    //     first; `_noclamp` yields the identity encoding.
    for p in &points {
        for (which, cf, rf) in [("ed25519", c, r), ("ed25519_noclamp", c_nc, r_nc)] {
            let mut a = canary(32);
            let mut b = canary(32);
            let (ra, rb) = unsafe {
                (
                    cf(a.as_mut_ptr(), zero.as_ptr(), p.as_ptr()),
                    rf(b.as_mut_ptr(), zero.as_ptr(), p.as_ptr()),
                )
            };
            eq_i32(&format!("scalarmult_{which}(n=0) rc"), ra, rb);
            assert_eq!(ra, -1);
            eq_bytes(&format!("scalarmult_{which}(n=0) q clobbered"), &a, &b);
            assert_ne!(a, canary(32), "q must have been overwritten");
            if which == "ed25519_noclamp" {
                eq_bytes("noclamp(0, P) yields the identity", &identity, &a);
            }
        }
    }
    for (which, cf, rf) in [("base", c_b, r_b), ("base_noclamp", c_bn, r_bn)] {
        let mut a = canary(32);
        let mut b = canary(32);
        let (ra, rb) = unsafe {
            (
                cf(a.as_mut_ptr(), zero.as_ptr()),
                rf(b.as_mut_ptr(), zero.as_ptr()),
            )
        };
        eq_i32(&format!("ed25519_{which}(n=0) rc"), ra, rb);
        assert_eq!(ra, -1);
        eq_bytes(&format!("ed25519_{which}(n=0) q"), &a, &b);
        if which == "base_noclamp" {
            eq_bytes("base_noclamp(0) yields the identity", &identity, &a);
        }
    }

    // --- G6-030 / G6-032 / G6-036: n = kL for k = 1..7 (all < 2^255, so
    //     `t[31] &= 127` leaves them alone) -> Q = identity -> -1 in the
    //     `_noclamp` forms. `sodium_is_zero(n)` is false here, so the
    //     `_is_inf(q)` half fires on its own.
    for k in 1..8u32 {
        let n = mul_small(&ell(), k);
        for p in &points {
            let mut a = canary(32);
            let mut b = canary(32);
            let (ra, rb) = unsafe {
                (
                    c_nc(a.as_mut_ptr(), n.as_ptr(), p.as_ptr()),
                    r_nc(b.as_mut_ptr(), n.as_ptr(), p.as_ptr()),
                )
            };
            eq_i32(&format!("noclamp(n={k}L) rc"), ra, rb);
            assert_eq!(ra, -1, "noclamp({k}L) must return -1");
            eq_bytes(&format!("noclamp(n={k}L) q"), &a, &b);
            eq_bytes(&format!("noclamp(n={k}L) == identity"), &identity, &a);
        }
        let mut a = canary(32);
        let mut b = canary(32);
        let (ra, rb) = unsafe {
            (
                c_bn(a.as_mut_ptr(), n.as_ptr()),
                r_bn(b.as_mut_ptr(), n.as_ptr()),
            )
        };
        eq_i32(&format!("base_noclamp(n={k}L) rc"), ra, rb);
        assert_eq!(ra, -1);
        eq_bytes(&format!("base_noclamp(n={k}L) q"), &a, &b);
        eq_bytes("base_noclamp(kL) == identity", &identity, &a);
    }

    // --- G6-031 / G6-037: n = 00..00 80 -> `t[31] &= 127` zeroes it
    for p in &points {
        let mut a = canary(32);
        let mut b = canary(32);
        let (ra, rb) = unsafe {
            (
                c_nc(a.as_mut_ptr(), top.as_ptr(), p.as_ptr()),
                r_nc(b.as_mut_ptr(), top.as_ptr(), p.as_ptr()),
            )
        };
        eq_i32("noclamp(n=00..80) rc", ra, rb);
        assert_eq!(ra, -1);
        eq_bytes("noclamp(n=00..80) q", &a, &b);
        eq_bytes("noclamp(n=00..80) == identity", &identity, &a);
    }
    let mut a = canary(32);
    let mut b = canary(32);
    let (ra, rb) = unsafe {
        (
            c_bn(a.as_mut_ptr(), top.as_ptr()),
            r_bn(b.as_mut_ptr(), top.as_ptr()),
        )
    };
    eq_i32("base_noclamp(n=00..80) rc", ra, rb);
    assert_eq!(ra, -1);
    eq_bytes("base_noclamp(n=00..80) q", &a, &b);

    // the clamped forms accept both of those (clamping forces bit 254)
    for p in &points {
        for n in [&top] {
            let mut a = canary(32);
            let mut b = canary(32);
            let (ra, rb) = unsafe {
                (
                    c(a.as_mut_ptr(), n.as_ptr(), p.as_ptr()),
                    r(b.as_mut_ptr(), n.as_ptr(), p.as_ptr()),
                )
            };
            eq_i32("clamped(n=00..80) rc", ra, rb);
            assert_eq!(ra, 0, "the clamped form must accept n = 00..80");
            eq_bytes("clamped(n=00..80)", &a, &b);
        }
    }
    for k in 1..8u32 {
        let n = mul_small(&ell(), k);
        let mut a = canary(32);
        let mut b = canary(32);
        let (ra, rb) = unsafe {
            (
                c_b(a.as_mut_ptr(), n.as_ptr()),
                r_b(b.as_mut_ptr(), n.as_ptr()),
            )
        };
        eq_i32(&format!("base(n={k}L) rc"), ra, rb);
        assert_eq!(ra, 0, "the clamped base form must accept {k}L");
        eq_bytes(&format!("base(n={k}L)"), &a, &b);
    }
}

// ===========================================================================
// crypto_scalarmult — ristretto255
// ===========================================================================

/// ERRORS G6-039, G6-040, G6-041, G6-042 — every `ristretto255_frombytes`
/// rejection reached from `crypto_scalarmult_ristretto255`: odd `s`, bit 255
/// set, `s >= p`, and a canonical even `s` that is not a valid encoding.
#[test]
fn scalarmult_ristretto255_point_rejections() {
    setup();
    let mut rng = Rng::new(0x7105);
    let (c, r) = pair::<I3>("crypto_scalarmult_ristretto255");
    let scalars: Vec<Vec<u8>> = {
        let mut v = vec![vec![1u8; 32]];
        let mut one = vec![0u8; 32];
        one[0] = 1;
        v.push(one);
        for _ in 0..8 {
            v.push(rng.bytes(32));
        }
        v
    };
    for (tag, p) in ris_invalid_points() {
        for n in &scalars {
            let mut a = canary(32);
            let mut b = canary(32);
            let (ra, rb) = unsafe {
                (
                    c(a.as_mut_ptr(), n.as_ptr(), p.as_ptr()),
                    r(b.as_mut_ptr(), n.as_ptr(), p.as_ptr()),
                )
            };
            eq_i32(&format!("ristretto255_mult [{tag}] rc"), ra, rb);
            assert_eq!(ra, -1, "ristretto255_mult must reject [{tag}]");
            eq_bytes(&format!("ristretto255_mult [{tag}] q"), &a, &b);
            assert_eq!(a, canary(32), "q must be untouched for [{tag}]");
        }
    }
}

/// ERRORS G6-043, G6-044, G6-045, G6-046, G6-047 — the `sodium_is_zero(q)`
/// rejection: `n = 0`, `n = kL` and `n = 00..00 80` all send a valid element
/// to the identity, whose ristretto255 encoding is 32 zero bytes. Note there
/// is **no** `sodium_is_zero(n)` check here (unlike ed25519), so the only
/// signal is the all-zero output.
#[test]
fn scalarmult_ristretto255_identity_rejections() {
    setup();
    let mut rng = Rng::new(0x7106);
    let (c, r) = pair::<I3>("crypto_scalarmult_ristretto255");
    let (c_b, r_b) = pair::<I2>("crypto_scalarmult_ristretto255_base");
    let points = ris_valid_points(&mut rng, 6);

    let mut ns: Vec<(String, [u8; 32])> = vec![("0".into(), [0u8; 32])];
    let mut top = [0u8; 32];
    top[31] = 0x80;
    ns.push(("00..80".into(), top));
    for k in 1..8u32 {
        ns.push((format!("{k}L"), mul_small(&ell(), k)));
    }

    for (tag, n) in &ns {
        for p in &points {
            let mut a = canary(32);
            let mut b = canary(32);
            let (ra, rb) = unsafe {
                (
                    c(a.as_mut_ptr(), n.as_ptr(), p.as_ptr()),
                    r(b.as_mut_ptr(), n.as_ptr(), p.as_ptr()),
                )
            };
            eq_i32(&format!("ristretto255_mult(n={tag}) rc"), ra, rb);
            assert_eq!(ra, -1, "ristretto255_mult(n={tag}) must return -1");
            eq_bytes(&format!("ristretto255_mult(n={tag}) q"), &a, &b);
            assert_eq!(&a[..], &[0u8; 32][..], "q must be 32 zero bytes");
        }
        let mut a = canary(32);
        let mut b = canary(32);
        let (ra, rb) = unsafe {
            (
                c_b(a.as_mut_ptr(), n.as_ptr()),
                r_b(b.as_mut_ptr(), n.as_ptr()),
            )
        };
        eq_i32(&format!("ristretto255_base(n={tag}) rc"), ra, rb);
        assert_eq!(ra, -1);
        eq_bytes(&format!("ristretto255_base(n={tag}) q"), &a, &b);
        assert_eq!(&a[..], &[0u8; 32][..]);
    }

    // the identity element itself: mult(n, identity) == identity -> -1
    let ident = [0u8; 32];
    for _ in 0..8 {
        let n = rng.bytes(32);
        let mut a = canary(32);
        let mut b = canary(32);
        let (ra, rb) = unsafe {
            (
                c(a.as_mut_ptr(), n.as_ptr(), ident.as_ptr()),
                r(b.as_mut_ptr(), n.as_ptr(), ident.as_ptr()),
            )
        };
        eq_i32("ristretto255_mult(p=identity) rc", ra, rb);
        assert_eq!(ra, -1);
        eq_bytes("ristretto255_mult(p=identity) q", &a, &b);
    }
}

// ===========================================================================
// crypto_core_ed25519
// ===========================================================================

/// ERRORS G6-048, G6-049, G6-051, G6-052, G6-053 —
/// `crypto_core_ed25519_is_valid_point` returns **0** (not -1) for
/// non-canonical encodings, a `y` that is not on the curve, every small-order
/// point and every point with a torsion component; and 1 for a genuine
/// main-subgroup point.
#[test]
fn core_ed25519_is_valid_point_rejections() {
    setup();
    let mut rng = Rng::new(0x7107);
    let (c, r) = pair::<I1c>("crypto_core_ed25519_is_valid_point");

    let mut bad: Vec<(String, [u8; 32])> = Vec::new();
    for p in ed_non_canonical().into_iter().chain(ed_non_canonical_on_curve()) {
        bad.push((format!("non-canonical {}", hex(&p)), p));
    }
    bad.push(("y not on curve".into(), ed_not_on_curve()));
    for (tag, p) in ed_small_order() {
        bad.push((format!("small order: {tag}"), p));
    }
    for (i, p) in ed_torsion_points().iter().enumerate() {
        bad.push((format!("torsion #{i}"), *p));
    }
    for (tag, p) in &bad {
        let (a, b) = unsafe { (c(p.as_ptr()), r(p.as_ptr())) };
        eq_i32(&format!("is_valid_point [{tag}]"), a, b);
        assert_eq!(a, 0, "is_valid_point must return 0 for [{tag}]");
    }

    // G6-053 (control)
    for p in ed_valid_points(&mut rng, 24) {
        let (a, b) = unsafe { (c(p.as_ptr()), r(p.as_ptr())) };
        eq_i32("is_valid_point (valid)", a, b);
        assert_eq!(a, 1);
    }

    // and every random 32-byte string: the two must simply agree
    for _ in 0..4000 {
        let p = rng.bytes(32);
        let (a, b) = unsafe { (c(p.as_ptr()), r(p.as_ptr())) };
        eq_i32(&format!("is_valid_point({})", hex(&p)), a, b);
    }
}

/// ERRORS G6-054, G6-056, G6-058, G6-059, G6-061, G6-063 —
/// `crypto_core_ed25519_add` / `_sub` only validate `ge25519_frombytes` +
/// `is_on_curve`. So `02 00..00` in either operand gives `-1` with `r`
/// untouched, while non-canonical encodings and small-order points are
/// **accepted** (return 0), and `sub(p, p)` gives the identity.
#[test]
fn core_ed25519_add_sub_rejections() {
    setup();
    let mut rng = Rng::new(0x7108);
    let bad = ed_not_on_curve();
    let good = ed_valid_points(&mut rng, 6);
    let identity = unhex32(IDENTITY_ED);

    for name in ["crypto_core_ed25519_add", "crypto_core_ed25519_sub"] {
        let (c, r) = pair::<I3>(name);
        // bad `p` (G6-054 / G6-059) and bad `q` (G6-056 / G6-061)
        for g in &good {
            for (tag, p, q) in [
                ("bad p", &bad, g),
                ("bad q", g, &bad),
                ("both bad", &bad, &bad),
            ] {
                let mut a = canary(32);
                let mut b = canary(32);
                let (ra, rb) = unsafe {
                    (
                        c(a.as_mut_ptr(), p.as_ptr(), q.as_ptr()),
                        r(b.as_mut_ptr(), p.as_ptr(), q.as_ptr()),
                    )
                };
                eq_i32(&format!("{name} [{tag}] rc"), ra, rb);
                assert_eq!(ra, -1, "{name} must reject [{tag}]");
                eq_bytes(&format!("{name} [{tag}] r"), &a, &b);
                assert_eq!(a, canary(32), "{name} must leave r untouched");
            }
        }

        // G6-058 / G6-063: non-canonical and small-order operands ARE accepted
        // — `_add`/`_sub` never call `ge25519_is_canonical` /
        // `has_small_order` / `is_on_main_subgroup`. Only the two non-canonical
        // encodings whose `y` is genuinely on the curve (`ed ff..7f` = y 0 and
        // `ee ff..7f` = y 1) must return 0; `ef ff..7f` (y 2) and `ff`x32
        // (y 18) are off the curve, so `ge25519_frombytes` rejects them and
        // only the C/Rust agreement can be asserted.
        let mut accepted: Vec<(String, [u8; 32])> = Vec::new();
        for p in ed_non_canonical_on_curve() {
            accepted.push((format!("non-canonical on-curve {}", hex(&p)), p));
        }
        for (tag, p) in ed_small_order() {
            accepted.push((format!("small order: {tag}"), p));
        }
        for (tag, p) in &accepted {
            for g in &good {
                let mut a = canary(32);
                let mut b = canary(32);
                let (ra, rb) = unsafe {
                    (
                        c(a.as_mut_ptr(), p.as_ptr(), g.as_ptr()),
                        r(b.as_mut_ptr(), p.as_ptr(), g.as_ptr()),
                    )
                };
                eq_i32(&format!("{name} [{tag}] accepted rc"), ra, rb);
                eq_bytes(&format!("{name} [{tag}] accepted"), &a, &b);
                assert_eq!(ra, 0, "{name} must accept [{tag}]");
                assert_ne!(a, canary(32), "{name} must have written r");
            }
        }
        // the remaining non-canonical encodings are off the curve: whatever the
        // C decides, the Rust must agree exactly.
        for p in ed_non_canonical() {
            for g in &good {
                let mut a = canary(32);
                let mut b = canary(32);
                let (ra, rb) = unsafe {
                    (
                        c(a.as_mut_ptr(), p.as_ptr(), g.as_ptr()),
                        r(b.as_mut_ptr(), p.as_ptr(), g.as_ptr()),
                    )
                };
                eq_i32(&format!("{name} [non-canonical {}] rc", hex(&p)), ra, rb);
                eq_bytes(&format!("{name} [non-canonical {}]", hex(&p)), &a, &b);
            }
        }
    }

    // G6-063: sub(p, p) == identity, return 0
    let (_, sub) = pair::<I3>("crypto_core_ed25519_sub");
    for p in &good {
        let mut a = canary(32);
        let ra = unsafe { sub(a.as_mut_ptr(), p.as_ptr(), p.as_ptr()) };
        assert_eq!(ra, 0);
        eq_bytes("sub(p, p) == identity", &identity, &a);
    }
}

/// ERRORS G6-064, G6-065, G6-075, G6-087, G6-092, G6-094, G6-095 — every
/// `*_from_string*` entry point forwards an out-of-range `hash_alg` to
/// `core_h2c_string_to_hash`'s `default:` case, which sets `errno = EINVAL`
/// and returns -1 without touching the output.
#[test]
fn from_string_bad_hash_alg_sets_einval() {
    setup();
    let mut rng = Rng::new(0x7109);
    let names = [
        "crypto_core_ed25519_from_string",
        "crypto_core_ed25519_from_string_nu",
        "crypto_core_ed25519_scalar_from_string",
        "crypto_core_ristretto255_from_string",
        "crypto_core_ristretto255_scalar_from_string",
    ];
    let algs: &[i32] = &[0, 3, 4, -1, -2, 100, i32::MAX, i32::MIN];
    for name in names {
        let (c, r) = pair::<FromString>(name);
        for &alg in algs {
            for (ctx_len, msg_len) in [(0usize, 0usize), (3, 5), (300, 40)] {
                let ctx = rng.bytes(ctx_len);
                let msg = rng.bytes(msg_len);
                let mut a = canary(32);
                let mut b = canary(32);

                clear_errno();
                let ra = unsafe {
                    c(
                        a.as_mut_ptr(),
                        ctx.as_ptr(),
                        ctx_len,
                        msg.as_ptr(),
                        msg_len,
                        alg,
                    )
                };
                let ea = errno();

                clear_errno();
                let rb = unsafe {
                    r(
                        b.as_mut_ptr(),
                        ctx.as_ptr(),
                        ctx_len,
                        msg.as_ptr(),
                        msg_len,
                        alg,
                    )
                };
                let eb = errno();

                eq_i32(&format!("{name}(hash_alg={alg}) rc"), ra, rb);
                assert_eq!(ra, -1, "{name}(hash_alg={alg}) must fail");
                eq_i32(&format!("{name}(hash_alg={alg}) errno"), ea, eb);
                assert_eq!(ea, EINVAL, "{name}(hash_alg={alg}) must set EINVAL");
                eq_bytes(&format!("{name}(hash_alg={alg}) output"), &a, &b);
                assert_eq!(a, canary(32), "{name} must leave the output untouched");
            }
        }
        // control: the two valid algorithms succeed and do not set EINVAL
        for alg in [1i32, 2] {
            let ctx = rng.bytes(7);
            let msg = rng.bytes(11);
            let mut a = canary(32);
            let mut b = canary(32);
            clear_errno();
            let ra = unsafe { c(a.as_mut_ptr(), ctx.as_ptr(), 7, msg.as_ptr(), 11, alg) };
            clear_errno();
            let rb = unsafe { r(b.as_mut_ptr(), ctx.as_ptr(), 7, msg.as_ptr(), 11, alg) };
            eq_i32(&format!("{name}(hash_alg={alg}) rc"), ra, rb);
            assert_eq!(ra, 0);
            eq_bytes(&format!("{name}(hash_alg={alg})"), &a, &b);
        }
    }
}

/// ERRORS G6-098 — `ctx_len > 0xff` is **not** an error: the DST is replaced by
/// `SHA-x("H2C-OVERSIZE-DST-" || ctx)` and `ctx_len` becomes 32 or 64. So the
/// call succeeds and produces a *different* result from the same ctx truncated
/// to 255 bytes, and `msg_len` is unbounded.
#[test]
fn core_h2c_oversize_ctx_is_not_an_error() {
    setup();
    let mut rng = Rng::new(0x710a);
    for name in [
        "crypto_core_ed25519_from_string",
        "crypto_core_ed25519_from_string_nu",
        "crypto_core_ed25519_scalar_from_string",
        "crypto_core_ristretto255_from_string",
        "crypto_core_ristretto255_scalar_from_string",
    ] {
        let (c, r) = pair::<FromString>(name);
        for alg in [1i32, 2] {
            let big = rng.bytes(1000);
            let msg = rng.bytes(5000);
            for &cl in &[255usize, 256, 257, 512, 1000] {
                let mut a = canary(32);
                let mut b = canary(32);
                clear_errno();
                let ra = unsafe {
                    c(a.as_mut_ptr(), big.as_ptr(), cl, msg.as_ptr(), msg.len(), alg)
                };
                let ea = errno();
                clear_errno();
                let rb = unsafe {
                    r(b.as_mut_ptr(), big.as_ptr(), cl, msg.as_ptr(), msg.len(), alg)
                };
                let eb = errno();
                eq_i32(&format!("{name}(ctx_len={cl}) rc"), ra, rb);
                assert_eq!(ra, 0, "{name}(ctx_len={cl}) must succeed");
                eq_i32(&format!("{name}(ctx_len={cl}) errno"), ea, eb);
                eq_bytes(&format!("{name}(ctx_len={cl})"), &a, &b);
            }
            // 255 (ordinary DST) and 256 (oversize DST) must differ
            let mut a = [0u8; 32];
            let mut b = [0u8; 32];
            unsafe {
                assert_eq!(
                    r(a.as_mut_ptr(), big.as_ptr(), 255, msg.as_ptr(), 8, alg),
                    0
                );
                assert_eq!(
                    r(b.as_mut_ptr(), big.as_ptr(), 256, msg.as_ptr(), 8, alg),
                    0
                );
            }
            assert_ne!(a, b, "{name}: the oversize-DST path must differ");
        }
    }
}

/// ERRORS G6-069, G6-070, G6-076 — `crypto_core_ed25519_random`,
/// `_scalar_random` and all six `void` scalar helpers have no rejection branch:
/// they always write, and non-canonical / `>= L` inputs are silently reduced.
#[test]
fn core_ed25519_void_entry_points_never_fail() {
    let _rng_lock = rng_guard();
    setup();
    let mut rng = Rng::new(0x710b);

    // `_random` / `_scalar_random`: always write, always valid
    let (c_p, r_p) = pair::<V1>("crypto_core_ed25519_random");
    let (c_s, r_s) = pair::<V1>("crypto_core_ed25519_scalar_random");
    let (_, valid) = pair::<I1c>("crypto_core_ed25519_is_valid_point");
    let (_, canon) = pair::<I1c>("crypto_core_ed25519_scalar_is_canonical");
    for seed in 0..24u64 {
        let mut a = canary(32);
        let mut b = canary(32);
        reset_rngs(0x9100_0000 + seed);
        unsafe { c_p(a.as_mut_ptr()) };
        reset_rngs(0x9100_0000 + seed);
        unsafe { r_p(b.as_mut_ptr()) };
        eq_bytes("ed25519_random", &a, &b);
        assert_eq!(unsafe { valid(b.as_ptr()) }, 1);

        let mut a = canary(32);
        let mut b = canary(32);
        reset_rngs(0x9200_0000 + seed);
        unsafe { c_s(a.as_mut_ptr()) };
        reset_rngs(0x9200_0000 + seed);
        unsafe { r_s(b.as_mut_ptr()) };
        eq_bytes("ed25519_scalar_random", &a, &b);
        assert_eq!(unsafe { canon(b.as_ptr()) }, 1);
        assert!(b.iter().any(|&x| x != 0));
    }

    // the `void` scalar helpers accept anything, including >= L
    let mut inputs: Vec<[u8; 32]> = vec![[0u8; 32], [0xffu8; 32], ell(), ell_minus_1()];
    for k in 2..8u32 {
        inputs.push(mul_small(&ell(), k));
    }
    for _ in 0..24 {
        let mut s = [0u8; 32];
        rng.fill(&mut s);
        inputs.push(s);
    }
    for name in [
        "crypto_core_ed25519_scalar_negate",
        "crypto_core_ed25519_scalar_complement",
    ] {
        let (c, r) = pair::<V2>(name);
        for s in &inputs {
            let mut a = canary(32);
            let mut b = canary(32);
            unsafe {
                c(a.as_mut_ptr(), s.as_ptr());
                r(b.as_mut_ptr(), s.as_ptr());
            }
            eq_bytes(&format!("{name}({})", hex(s)), &a, &b);
        }
    }
    for name in [
        "crypto_core_ed25519_scalar_add",
        "crypto_core_ed25519_scalar_sub",
        "crypto_core_ed25519_scalar_mul",
    ] {
        let (c, r) = pair::<V3>(name);
        for x in &inputs {
            for y in &inputs {
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
    let (c_red, r_red) = pair::<V2>("crypto_core_ed25519_scalar_reduce");
    for _ in 0..1500 {
        let s = rng.bytes(64);
        let mut a = canary(32);
        let mut b = canary(32);
        unsafe {
            c_red(a.as_mut_ptr(), s.as_ptr());
            r_red(b.as_mut_ptr(), s.as_ptr());
        }
        eq_bytes("scalar_reduce", &a, &b);
    }
}

/// ERRORS G6-071, G6-072 — `crypto_core_ed25519_scalar_invert` rejects **only**
/// an all-zero `s` (`- sodium_is_zero(s, 32)`), and `recip` is still written by
/// `sc25519_invert` before the return, so the "garbage" output must match too.
/// Non-canonical scalars (`s >= L`, all-0xff) are *not* rejected.
#[test]
fn core_ed25519_scalar_invert_zero() {
    setup();
    let mut rng = Rng::new(0x710c);
    for name in [
        "crypto_core_ed25519_scalar_invert",
        "crypto_core_ristretto255_scalar_invert",
    ] {
        let (c, r) = pair::<I2>(name);
        // G6-071 / G6-090: s = 0
        let zero = [0u8; 32];
        let mut a = canary(32);
        let mut b = canary(32);
        let (ra, rb) = unsafe {
            (
                c(a.as_mut_ptr(), zero.as_ptr()),
                r(b.as_mut_ptr(), zero.as_ptr()),
            )
        };
        eq_i32(&format!("{name}(0) rc"), ra, rb);
        assert_eq!(ra, -1, "{name}(0) must return -1");
        eq_bytes(&format!("{name}(0) recip"), &a, &b);
        assert_ne!(a, canary(32), "recip is still written before the return");

        // G6-072: s >= L is accepted
        let mut accepted: Vec<[u8; 32]> = vec![ell(), [0xffu8; 32], ell_minus_1()];
        for k in 2..8u32 {
            accepted.push(mul_small(&ell(), k));
        }
        for _ in 0..24 {
            let mut s = [0u8; 32];
            rng.fill(&mut s);
            accepted.push(s);
        }
        for s in &accepted {
            let mut a = canary(32);
            let mut b = canary(32);
            let (ra, rb) = unsafe {
                (
                    c(a.as_mut_ptr(), s.as_ptr()),
                    r(b.as_mut_ptr(), s.as_ptr()),
                )
            };
            eq_i32(&format!("{name}({}) rc", hex(s)), ra, rb);
            assert_eq!(ra, 0, "{name} must accept non-canonical s");
            eq_bytes(&format!("{name}({})", hex(s)), &a, &b);
        }
    }
}

/// ERRORS G6-073, G6-074, G6-091 — `crypto_core_ed25519_scalar_is_canonical`
/// (and the ristretto255 alias) return **0** for `s >= L` and 1 for 0, 1 and
/// L-1. The comparison is byte-wise from the high end down.
#[test]
fn scalar_is_canonical_rejections() {
    setup();
    let mut rng = Rng::new(0x710d);
    for name in [
        "crypto_core_ed25519_scalar_is_canonical",
        "crypto_core_ristretto255_scalar_is_canonical",
    ] {
        let (c, r) = pair::<I1c>(name);
        // rejected: L, L+1, all-0xff, kL, 00..00 10
        let mut bad: Vec<[u8; 32]> = vec![ell(), [0xffu8; 32]];
        let mut lp1 = ell();
        lp1[0] += 1;
        bad.push(lp1);
        for k in 2..8u32 {
            bad.push(mul_small(&ell(), k));
        }
        for s in &bad {
            let (a, b) = unsafe { (c(s.as_ptr()), r(s.as_ptr())) };
            eq_i32(&format!("{name}({})", hex(s)), a, b);
            assert_eq!(a, 0, "{name}({}) must be 0", hex(s));
        }
        // accepted: 0, 1, L-1, and 2^252 = `00`x31 + `10` (which is < L,
        // since L = 2^252 + 27742317777372353535851937790883648493)
        let mut good: Vec<[u8; 32]> = vec![[0u8; 32], ell_minus_1()];
        let mut one = [0u8; 32];
        one[0] = 1;
        good.push(one);
        let mut p252 = [0u8; 32];
        p252[31] = 0x10;
        good.push(p252);
        for s in &good {
            let (a, b) = unsafe { (c(s.as_ptr()), r(s.as_ptr())) };
            eq_i32(&format!("{name}({})", hex(s)), a, b);
            assert_eq!(a, 1, "{name}({}) must be 1", hex(s));
        }
        // and 4000 random shapes: the two must simply agree
        for _ in 0..4000 {
            let s = rng.bytes(32);
            let (a, b) = unsafe { (c(s.as_ptr()), r(s.as_ptr())) };
            eq_i32(&format!("{name}({})", hex(&s)), a, b);
        }
    }
}

// ===========================================================================
// crypto_core_ristretto255
// ===========================================================================

/// ERRORS G6-077, G6-078, G6-079, G6-080, G6-081 —
/// `crypto_core_ristretto255_is_valid_point` returns 0 for odd `s`, bit 255
/// set, `s >= p` and canonical-even-but-invalid encodings; and 1 for the
/// identity (32 zero bytes), which *is* a valid encoding.
#[test]
fn core_ristretto255_is_valid_point_rejections() {
    setup();
    let mut rng = Rng::new(0x710e);
    let (c, r) = pair::<I1c>("crypto_core_ristretto255_is_valid_point");
    for (tag, p) in ris_invalid_points() {
        let (a, b) = unsafe { (c(p.as_ptr()), r(p.as_ptr())) };
        eq_i32(&format!("ristretto255_is_valid_point [{tag}]"), a, b);
        assert_eq!(a, 0, "[{tag}] must be rejected");
    }
    // G6-081 (control): the identity is valid
    let ident = [0u8; 32];
    let (a, b) = unsafe { (c(ident.as_ptr()), r(ident.as_ptr())) };
    eq_i32("ristretto255_is_valid_point(identity)", a, b);
    assert_eq!(a, 1);
    for p in ris_valid_points(&mut rng, 24) {
        let (a, b) = unsafe { (c(p.as_ptr()), r(p.as_ptr())) };
        eq_i32("ristretto255_is_valid_point (valid)", a, b);
        assert_eq!(a, 1);
    }
    for _ in 0..4000 {
        let p = rng.bytes(32);
        let (a, b) = unsafe { (c(p.as_ptr()), r(p.as_ptr())) };
        eq_i32(&format!("ristretto255_is_valid_point({})", hex(&p)), a, b);
    }
}

/// ERRORS G6-082, G6-083, G6-084, G6-085 —
/// `crypto_core_ristretto255_add` / `_sub` run the full
/// `ristretto255_frombytes` validation on both operands, so an invalid `p` or
/// `q` gives -1 with `r` untouched.
#[test]
fn core_ristretto255_add_sub_rejections() {
    setup();
    let mut rng = Rng::new(0x710f);
    let good = ris_valid_points(&mut rng, 5);
    for name in [
        "crypto_core_ristretto255_add",
        "crypto_core_ristretto255_sub",
    ] {
        let (c, r) = pair::<I3>(name);
        for (tag, bad) in ris_invalid_points() {
            for g in &good {
                for (side, p, q) in [
                    ("bad p", &bad, g),
                    ("bad q", g, &bad),
                    ("both bad", &bad, &bad),
                ] {
                    let mut a = canary(32);
                    let mut b = canary(32);
                    let (ra, rb) = unsafe {
                        (
                            c(a.as_mut_ptr(), p.as_ptr(), q.as_ptr()),
                            r(b.as_mut_ptr(), p.as_ptr(), q.as_ptr()),
                        )
                    };
                    eq_i32(&format!("{name} [{tag}/{side}] rc"), ra, rb);
                    assert_eq!(ra, -1, "{name} must reject [{tag}/{side}]");
                    eq_bytes(&format!("{name} [{tag}/{side}] r"), &a, &b);
                    assert_eq!(a, canary(32), "r must be untouched");
                }
            }
        }
    }
}

/// ERRORS G6-086, G6-088, G6-089, G6-093 —
/// `crypto_core_ristretto255_from_hash` accepts any 64-byte input and always
/// returns 0; `_random`, `_scalar_random` and the six `void` scalar helpers
/// have no rejection branch at all.
#[test]
fn core_ristretto255_void_entry_points_never_fail() {
    let _rng_lock = rng_guard();
    setup();
    let mut rng = Rng::new(0x7110);

    let (c_fh, r_fh) = pair::<I2>("crypto_core_ristretto255_from_hash");
    let mut cases: Vec<Vec<u8>> = vec![vec![0u8; 64], vec![0xffu8; 64]];
    for _ in 0..600 {
        cases.push(rng.bytes(64));
    }
    for h in &cases {
        let mut a = canary(32);
        let mut b = canary(32);
        let (ra, rb) = unsafe {
            (
                c_fh(a.as_mut_ptr(), h.as_ptr()),
                r_fh(b.as_mut_ptr(), h.as_ptr()),
            )
        };
        eq_i32("ristretto255_from_hash rc", ra, rb);
        assert_eq!(ra, 0, "from_hash can never fail");
        eq_bytes("ristretto255_from_hash", &a, &b);
    }

    let (c_p, r_p) = pair::<V1>("crypto_core_ristretto255_random");
    let (c_s, r_s) = pair::<V1>("crypto_core_ristretto255_scalar_random");
    for seed in 0..24u64 {
        let mut a = canary(32);
        let mut b = canary(32);
        reset_rngs(0x9300_0000 + seed);
        unsafe { c_p(a.as_mut_ptr()) };
        reset_rngs(0x9300_0000 + seed);
        unsafe { r_p(b.as_mut_ptr()) };
        eq_bytes("ristretto255_random", &a, &b);
        let mut a = canary(32);
        let mut b = canary(32);
        reset_rngs(0x9400_0000 + seed);
        unsafe { c_s(a.as_mut_ptr()) };
        reset_rngs(0x9400_0000 + seed);
        unsafe { r_s(b.as_mut_ptr()) };
        eq_bytes("ristretto255_scalar_random", &a, &b);
    }

    let mut inputs: Vec<[u8; 32]> = vec![[0u8; 32], [0xffu8; 32], ell(), ell_minus_1()];
    for _ in 0..16 {
        let mut s = [0u8; 32];
        rng.fill(&mut s);
        inputs.push(s);
    }
    for name in [
        "crypto_core_ristretto255_scalar_negate",
        "crypto_core_ristretto255_scalar_complement",
    ] {
        let (c, r) = pair::<V2>(name);
        for s in &inputs {
            let mut a = canary(32);
            let mut b = canary(32);
            unsafe {
                c(a.as_mut_ptr(), s.as_ptr());
                r(b.as_mut_ptr(), s.as_ptr());
            }
            eq_bytes(name, &a, &b);
        }
    }
    for name in [
        "crypto_core_ristretto255_scalar_add",
        "crypto_core_ristretto255_scalar_sub",
        "crypto_core_ristretto255_scalar_mul",
    ] {
        let (c, r) = pair::<V3>(name);
        for x in &inputs {
            for y in &inputs {
                let mut a = canary(32);
                let mut b = canary(32);
                unsafe {
                    c(a.as_mut_ptr(), x.as_ptr(), y.as_ptr());
                    r(b.as_mut_ptr(), x.as_ptr(), y.as_ptr());
                }
                eq_bytes(name, &a, &b);
            }
        }
    }
    let (c_red, r_red) = pair::<V2>("crypto_core_ristretto255_scalar_reduce");
    for _ in 0..1000 {
        let s = rng.bytes(64);
        let mut a = canary(32);
        let mut b = canary(32);
        unsafe {
            c_red(a.as_mut_ptr(), s.as_ptr());
            r_red(b.as_mut_ptr(), s.as_ptr());
        }
        eq_bytes("ristretto255_scalar_reduce", &a, &b);
    }
}

// ===========================================================================
// crypto_core_salsa* / hsalsa20 / hchacha20
// ===========================================================================

/// ERRORS G6-099, G6-100, G6-101 — `crypto_core_salsa20` / `_salsa2012` /
/// `_salsa208` / `_hsalsa20` / `_hchacha20` have no rejection branch and
/// unconditionally return 0; `c == NULL` is a *legal* input (it selects the
/// sigma constants), not an error.
#[test]
fn core_salsa_family_never_fails() {
    setup();
    let mut rng = Rng::new(0x7111);
    for (name, outlen) in [
        ("crypto_core_salsa20", 64usize),
        ("crypto_core_salsa2012", 64),
        ("crypto_core_salsa208", 64),
        ("crypto_core_hsalsa20", 32),
        ("crypto_core_hchacha20", 32),
    ] {
        let (c, r) = pair::<Core>(name);
        for iter in 0..48 {
            let inp = match iter % 3 {
                0 => rng.bytes(16),
                1 => vec![0u8; 16],
                _ => vec![0xffu8; 16],
            };
            let k = match iter % 3 {
                0 => vec![0u8; 32],
                1 => vec![0xffu8; 32],
                _ => rng.bytes(32),
            };
            let cst: Option<Vec<u8>> = match iter % 3 {
                0 => None,
                1 => Some(b"expand 32-byte k".to_vec()),
                _ => Some(rng.bytes(16)),
            };
            let cp = cst.as_ref().map_or(std::ptr::null(), |v| v.as_ptr());
            let mut a = canary(outlen);
            let mut b = canary(outlen);
            let (ra, rb) = unsafe {
                (
                    c(a.as_mut_ptr(), inp.as_ptr(), k.as_ptr(), cp),
                    r(b.as_mut_ptr(), inp.as_ptr(), k.as_ptr(), cp),
                )
            };
            eq_i32(&format!("{name} rc"), ra, rb);
            assert_eq!(ra, 0, "{name} must always return 0");
            eq_bytes(name, &a, &b);
        }
    }
}

// ===========================================================================
// crypto_stream
// ===========================================================================

/// ERRORS G6-102, G6-103, G6-116, G6-117, G6-118, G6-121, G6-122 — the stream
/// primitives with no guard at all: `crypto_stream` / `_xor`, every
/// `salsa20` / `salsa2012` / `salsa208` / `xsalsa20` entry point (including
/// `ic = 2^64-1`) and every `_keygen`. All return 0 for every length.
#[test]
fn stream_no_rejection_paths() {
    let _rng_lock = rng_guard();
    setup();
    let mut rng = Rng::new(0x7112);
    let lens: &[usize] = &[0, 1, 63, 64, 65, 127, 128, 129, 512, 1000];

    for (prefix, nb) in [
        ("crypto_stream", 24usize),
        ("crypto_stream_salsa20", 8),
        ("crypto_stream_salsa2012", 8),
        ("crypto_stream_salsa208", 8),
        ("crypto_stream_xsalsa20", 24),
    ] {
        let (c_ks, r_ks) = pair::<Stream>(prefix);
        let (c_x, r_x) = pair::<StreamXor>(&format!("{prefix}_xor"));
        for &len in lens {
            let k = rng.bytes(32);
            let n = rng.bytes(nb);
            let m = rng.bytes(len);
            let mut a = canary(len);
            let mut b = canary(len);
            let (ra, rb) = unsafe {
                (
                    c_ks(a.as_mut_ptr(), len as u64, n.as_ptr(), k.as_ptr()),
                    r_ks(b.as_mut_ptr(), len as u64, n.as_ptr(), k.as_ptr()),
                )
            };
            eq_i32(&format!("{prefix}({len}) rc"), ra, rb);
            assert_eq!(ra, 0);
            eq_bytes(prefix, &a, &b);

            let mut a = canary(len);
            let mut b = canary(len);
            let (ra, rb) = unsafe {
                (
                    c_x(a.as_mut_ptr(), m.as_ptr(), len as u64, n.as_ptr(), k.as_ptr()),
                    r_x(b.as_mut_ptr(), m.as_ptr(), len as u64, n.as_ptr(), k.as_ptr()),
                )
            };
            eq_i32(&format!("{prefix}_xor({len}) rc"), ra, rb);
            assert_eq!(ra, 0);
            eq_bytes(&format!("{prefix}_xor"), &a, &b);
        }
    }

    // `_xor_ic` with the extreme 64-bit counters: accepted, counter wraps
    for (name, nb) in [
        ("crypto_stream_salsa20_xor_ic", 8usize),
        ("crypto_stream_xsalsa20_xor_ic", 24),
        ("crypto_stream_chacha20_xor_ic", 8),
        ("crypto_stream_xchacha20_xor_ic", 24),
    ] {
        let (c, r) = pair::<StreamXorIc64>(name);
        for &len in &[0usize, 1, 64, 65, 128, 200] {
            for &ic in &[0u64, 0xffff_ffff, 0x1_0000_0000, u64::MAX - 1, u64::MAX] {
                let k = rng.bytes(32);
                let n = rng.bytes(nb);
                let m = rng.bytes(len);
                let mut a = canary(len);
                let mut b = canary(len);
                let (ra, rb) = unsafe {
                    (
                        c(a.as_mut_ptr(), m.as_ptr(), len as u64, n.as_ptr(), ic, k.as_ptr()),
                        r(b.as_mut_ptr(), m.as_ptr(), len as u64, n.as_ptr(), ic, k.as_ptr()),
                    )
                };
                eq_i32(&format!("{name}(len={len}, ic={ic:#x}) rc"), ra, rb);
                assert_eq!(ra, 0, "{name} has no guard");
                eq_bytes(&format!("{name}(len={len}, ic={ic:#x})"), &a, &b);
            }
        }
    }

    // G6-122: every keygen is `void randombytes_buf(k, 32)`
    for name in [
        "crypto_stream_keygen",
        "crypto_stream_chacha20_keygen",
        "crypto_stream_chacha20_ietf_keygen",
        "crypto_stream_salsa20_keygen",
        "crypto_stream_salsa2012_keygen",
        "crypto_stream_salsa208_keygen",
        "crypto_stream_xchacha20_keygen",
        "crypto_stream_xsalsa20_keygen",
    ] {
        let (c, r) = pair::<V1>(name);
        for seed in 0..6u64 {
            let mut a = canary(32);
            let mut b = canary(32);
            reset_rngs(0x9500_0000 + seed);
            unsafe { c(a.as_mut_ptr()) };
            reset_rngs(0x9500_0000 + seed);
            unsafe { r(b.as_mut_ptr()) };
            eq_bytes(name, &a, &b);
            assert_ne!(a, canary(32), "{name} wrote nothing");
        }
    }
}

/// ERRORS G6-113, G6-114 — the two `crypto_stream_chacha20_ietf_xor_ic`
/// boundary cases that do **not** misuse: `mlen = 64, ic = 0xffffffff`
/// (`ceil(64/64) = 1`, bound `2^32 - 1`) and `mlen = 0, ic = 0xffffffff`
/// (bound `2^32`, and `mlen == 0` short-circuits inside the ref backend).
/// The neighbouring accepted cases are swept too.
#[test]
fn stream_chacha20_ietf_xor_ic_boundaries() {
    setup();
    let mut rng = Rng::new(0x7113);
    let (c, r) = pair::<StreamXorIc32>("crypto_stream_chacha20_ietf_xor_ic");
    let mut cases: Vec<(usize, u32)> = vec![(0, 0xffff_ffff), (64, 0xffff_ffff)];
    for mlen in [0usize, 1, 32, 63, 64] {
        cases.push((mlen, 0xffff_ffff));
    }
    for mlen in [65usize, 128] {
        cases.push((mlen, 0xffff_fffe));
    }
    for mlen in [129usize, 192] {
        cases.push((mlen, 0xffff_fffd));
    }
    cases.push((1000, 0xffff_ffff - 16));
    for (mlen, ic) in cases {
        let blocks = ((mlen + 63) / 64) as u64;
        assert!(
            ic as u64 <= 0x1_0000_0000 - blocks,
            "case ({mlen}, {ic:#x}) would misuse"
        );
        let k = rng.bytes(32);
        let n = rng.bytes(12);
        let m = rng.bytes(mlen);
        let mut a = canary(mlen);
        let mut b = canary(mlen);
        let (ra, rb) = unsafe {
            (
                c(a.as_mut_ptr(), m.as_ptr(), mlen as u64, n.as_ptr(), ic, k.as_ptr()),
                r(b.as_mut_ptr(), m.as_ptr(), mlen as u64, n.as_ptr(), ic, k.as_ptr()),
            )
        };
        eq_i32(&format!("ietf_xor_ic(mlen={mlen}, ic={ic:#x}) rc"), ra, rb);
        assert_eq!(ra, 0, "({mlen}, {ic:#x}) must be accepted");
        eq_bytes(&format!("ietf_xor_ic(mlen={mlen}, ic={ic:#x})"), &a, &b);
    }
}

// ===========================================================================
// crypto_kem
// ===========================================================================

/// ERRORS G6-123, G6-124, G6-125, G6-126, G6-127, G6-128 — ML-KEM-768:
/// `_seed_keypair` / `_keypair` never fail; `_enc_deterministic` / `_enc`
/// reject a non-canonical public key with -1 and leave `ct`/`ss` untouched;
/// `_dec` **never** returns -1 (implicit rejection) even for a corrupted
/// ciphertext or an all-zero secret key.
#[test]
fn kem_mlkem768_rejections() {
    let _rng_lock = rng_guard();
    setup();
    let mut rng = Rng::new(0x7114);
    let (pkb, skb, ctb, ssb) = (1184usize, 2400usize, 1088usize, 32usize);
    let (c_sk, r_sk) = pair::<KemSeedKeypair>("crypto_kem_mlkem768_seed_keypair");
    let (c_kp, r_kp) = pair::<KemKeypair>("crypto_kem_mlkem768_keypair");
    let (c_ed, r_ed) = pair::<KemEncDet>("crypto_kem_mlkem768_enc_deterministic");
    let (c_e, r_e) = pair::<KemEnc>("crypto_kem_mlkem768_enc");
    let (c_d, r_d) = pair::<KemDec>("crypto_kem_mlkem768_dec");

    // G6-123: any 64-byte seed is accepted
    for seed in [vec![0u8; 64], vec![0xffu8; 64], rng.bytes(64), rng.bytes(64)] {
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
        assert_eq!(ra, 0);
        eq_bytes("mlkem768_seed_keypair pk", &apk, &bpk);
        eq_bytes("mlkem768_seed_keypair sk", &ask, &bsk);
    }
    // G6-124
    for seed in 0..2u64 {
        let mut apk = canary(pkb);
        let mut ask = canary(skb);
        let mut bpk = canary(pkb);
        let mut bsk = canary(skb);
        reset_rngs(0x9600_0000 + seed);
        let ra = unsafe { c_kp(apk.as_mut_ptr(), ask.as_mut_ptr()) };
        reset_rngs(0x9600_0000 + seed);
        let rb = unsafe { r_kp(bpk.as_mut_ptr(), bsk.as_mut_ptr()) };
        eq_i32("mlkem768_keypair rc", ra, rb);
        assert_eq!(ra, 0);
        eq_bytes("mlkem768_keypair pk", &apk, &bpk);
        eq_bytes("mlkem768_keypair sk", &ask, &bsk);
    }

    // G6-125: non-canonical pk (any coefficient >= 3329) -> -1
    let mut bad_pks: Vec<(String, Vec<u8>)> = vec![("all-0xff".into(), vec![0xffu8; pkb])];
    {
        // a valid pk with the very first 12-bit coefficient pair forced to
        // 0xfff (= 4095 >= 3329)
        let seed = rng.bytes(64);
        let mut pk = vec![0u8; pkb];
        let mut sk = vec![0u8; skb];
        unsafe { assert_eq!(r_sk(pk.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr()), 0) };
        let mut p2 = pk.clone();
        p2[0] = 0xff;
        p2[1] = 0xff;
        p2[2] = 0xff;
        bad_pks.push(("valid pk with coeff 0 forced out of range".into(), p2));
        let mut p3 = pk.clone();
        // last coefficient of the last polynomial
        p3[1149] = 0xff;
        p3[1150] = 0xff;
        p3[1151] = 0xff;
        bad_pks.push(("valid pk with the last coeff out of range".into(), p3));
    }
    for (tag, pk) in &bad_pks {
        for es in [vec![0u8; 32], vec![0xffu8; 32], rng.bytes(32)] {
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
            eq_i32(&format!("mlkem768_enc_deterministic [{tag}] rc"), ra, rb);
            assert_eq!(ra, -1, "[{tag}] must be rejected");
            eq_bytes("mlkem768_enc_deterministic ct untouched", &act, &bct);
            eq_bytes("mlkem768_enc_deterministic ss untouched", &ass, &bss);
            assert_eq!(act, canary(ctb));
            assert_eq!(ass, canary(ssb));
        }
        // G6-126: `_enc` propagates the same failure
        let mut act = canary(ctb);
        let mut ass = canary(ssb);
        let mut bct = canary(ctb);
        let mut bss = canary(ssb);
        reset_rngs(0x9700_0000);
        let ra = unsafe { c_e(act.as_mut_ptr(), ass.as_mut_ptr(), pk.as_ptr()) };
        reset_rngs(0x9700_0000);
        let rb = unsafe { r_e(bct.as_mut_ptr(), bss.as_mut_ptr(), pk.as_ptr()) };
        eq_i32(&format!("mlkem768_enc [{tag}] rc"), ra, rb);
        assert_eq!(ra, -1);
        eq_bytes("mlkem768_enc ct untouched", &act, &bct);
        eq_bytes("mlkem768_enc ss untouched", &ass, &bss);
    }

    // G6-127 / G6-128: `_dec` never returns -1
    let seed = rng.bytes(64);
    let mut pk = vec![0u8; pkb];
    let mut sk = vec![0u8; skb];
    unsafe { assert_eq!(r_sk(pk.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr()), 0) };
    let es = rng.bytes(32);
    let mut ct = vec![0u8; ctb];
    let mut ss = vec![0u8; ssb];
    unsafe {
        assert_eq!(
            r_ed(ct.as_mut_ptr(), ss.as_mut_ptr(), pk.as_ptr(), es.as_ptr()),
            0
        );
    }
    let mut cts: Vec<(String, Vec<u8>)> = Vec::new();
    for &(byte, bit) in &[(0usize, 0u8), (1, 7), (500, 2), (1087, 7)] {
        let mut v = ct.clone();
        v[byte] ^= 1 << bit;
        cts.push((format!("bit {bit} of byte {byte} flipped"), v));
    }
    cts.push(("all-zero ct".into(), vec![0u8; ctb]));
    cts.push(("all-0xff ct".into(), vec![0xffu8; ctb]));
    for _ in 0..4 {
        cts.push(("random ct".into(), rng.bytes(ctb)));
    }
    let sks: Vec<(String, Vec<u8>)> = vec![
        ("valid sk".into(), sk.clone()),
        ("all-zero sk".into(), vec![0u8; skb]),
        ("all-0xff sk".into(), vec![0xffu8; skb]),
        ("random sk".into(), rng.bytes(skb)),
    ];
    for (sktag, s) in &sks {
        for (cttag, v) in &cts {
            let mut a = canary(ssb);
            let mut b = canary(ssb);
            let (ra, rb) = unsafe {
                (
                    c_d(a.as_mut_ptr(), v.as_ptr(), s.as_ptr()),
                    r_d(b.as_mut_ptr(), v.as_ptr(), s.as_ptr()),
                )
            };
            eq_i32(&format!("mlkem768_dec [{sktag}/{cttag}] rc"), ra, rb);
            assert_eq!(ra, 0, "mlkem768_dec must always return 0");
            eq_bytes(&format!("mlkem768_dec [{sktag}/{cttag}] ss"), &a, &b);
        }
        // and the *valid* ciphertext with a broken key: still 0
        let mut a = canary(ssb);
        let mut b = canary(ssb);
        let (ra, rb) = unsafe {
            (
                c_d(a.as_mut_ptr(), ct.as_ptr(), s.as_ptr()),
                r_d(b.as_mut_ptr(), ct.as_ptr(), s.as_ptr()),
            )
        };
        eq_i32(&format!("mlkem768_dec [{sktag}/valid ct] rc"), ra, rb);
        assert_eq!(ra, 0);
        eq_bytes(&format!("mlkem768_dec [{sktag}/valid ct] ss"), &a, &b);
        if sktag == "valid sk" {
            eq_bytes("mlkem768_dec recovers the real ss", &ss, &a);
        }
    }
}

/// ERRORS G6-129, G6-130, G6-131, G6-132, G6-133, G6-135, G6-136 — X-Wing:
/// key generation never fails; encapsulation fails when either the ML-KEM half
/// of `pk` is non-canonical **or** the X25519 half is a blocklisted small-order
/// point; decapsulation fails only via the X25519 half of the ciphertext (with
/// `ss` untouched), while corruption of the ML-KEM half is implicitly rejected
/// and yields `0` with a different `ss`.
#[test]
fn kem_xwing_rejections() {
    let _rng_lock = rng_guard();
    setup();
    let mut rng = Rng::new(0x7115);
    let (pkb, skb, ctb, ssb) = (1216usize, 32usize, 1120usize, 32usize);
    let (c_sk, r_sk) = pair::<KemSeedKeypair>("crypto_kem_xwing_seed_keypair");
    let (c_kp, r_kp) = pair::<KemKeypair>("crypto_kem_xwing_keypair");
    let (c_ed, r_ed) = pair::<KemEncDet>("crypto_kem_xwing_enc_deterministic");
    let (c_e, r_e) = pair::<KemEnc>("crypto_kem_xwing_enc");
    let (c_d, r_d) = pair::<KemDec>("crypto_kem_xwing_dec");

    // G6-129 / G6-130
    for seed in [vec![0u8; 32], vec![0xffu8; 32], rng.bytes(32)] {
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
        assert_eq!(ra, 0);
        eq_bytes("xwing_seed_keypair pk", &apk, &bpk);
        eq_bytes("xwing_seed_keypair sk", &ask, &bsk);
    }
    for seed in 0..2u64 {
        let mut apk = canary(pkb);
        let mut ask = canary(skb);
        let mut bpk = canary(pkb);
        let mut bsk = canary(skb);
        reset_rngs(0x9800_0000 + seed);
        let ra = unsafe { c_kp(apk.as_mut_ptr(), ask.as_mut_ptr()) };
        reset_rngs(0x9800_0000 + seed);
        let rb = unsafe { r_kp(bpk.as_mut_ptr(), bsk.as_mut_ptr()) };
        eq_i32("xwing_keypair rc", ra, rb);
        assert_eq!(ra, 0);
        eq_bytes("xwing_keypair pk", &apk, &bpk);
        eq_bytes("xwing_keypair sk", &ask, &bsk);
    }

    // a real key pair to build the bad public keys from
    let seed = rng.bytes(32);
    let mut pk = vec![0u8; pkb];
    let mut sk = vec![0u8; skb];
    unsafe { assert_eq!(r_sk(pk.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr()), 0) };

    let mut bad_pks: Vec<(String, Vec<u8>)> = Vec::new();
    // G6-131: the ML-KEM half is non-canonical
    bad_pks.push(("all-0xff pk".into(), vec![0xffu8; pkb]));
    let mut p = pk.clone();
    p[0] = 0xff;
    p[1] = 0xff;
    p[2] = 0xff;
    bad_pks.push(("ML-KEM half non-canonical".into(), p));
    // G6-132: the X25519 half is a blocklisted small-order point
    for (i, b) in x25519_blocklist().iter().enumerate() {
        let mut p = pk.clone();
        p[1184..].copy_from_slice(b);
        bad_pks.push((format!("X25519 half = blocklist[{i}]"), p));
        let mut p = pk.clone();
        let mut hi = *b;
        hi[31] |= 0x80;
        p[1184..].copy_from_slice(&hi);
        bad_pks.push((format!("X25519 half = blocklist[{i}] | bit255"), p));
    }

    for (tag, bp) in &bad_pks {
        for es in [vec![0u8; 64], vec![0xffu8; 64], rng.bytes(64)] {
            let mut act = canary(ctb);
            let mut ass = canary(ssb);
            let mut bct = canary(ctb);
            let mut bss = canary(ssb);
            let (ra, rb) = unsafe {
                (
                    c_ed(act.as_mut_ptr(), ass.as_mut_ptr(), bp.as_ptr(), es.as_ptr()),
                    r_ed(bct.as_mut_ptr(), bss.as_mut_ptr(), bp.as_ptr(), es.as_ptr()),
                )
            };
            eq_i32(&format!("xwing_enc_deterministic [{tag}] rc"), ra, rb);
            assert_eq!(ra, -1, "[{tag}] must be rejected");
            eq_bytes("xwing_enc_deterministic ct", &act, &bct);
            eq_bytes("xwing_enc_deterministic ss", &ass, &bss);
            assert_eq!(act, canary(ctb), "ct must be untouched");
            assert_eq!(ass, canary(ssb), "ss must be untouched");
        }
        // G6-133
        let mut act = canary(ctb);
        let mut ass = canary(ssb);
        let mut bct = canary(ctb);
        let mut bss = canary(ssb);
        reset_rngs(0x9900_0000);
        let ra = unsafe { c_e(act.as_mut_ptr(), ass.as_mut_ptr(), bp.as_ptr()) };
        reset_rngs(0x9900_0000);
        let rb = unsafe { r_e(bct.as_mut_ptr(), bss.as_mut_ptr(), bp.as_ptr()) };
        eq_i32(&format!("xwing_enc [{tag}] rc"), ra, rb);
        assert_eq!(ra, -1);
        eq_bytes("xwing_enc ct", &act, &bct);
        eq_bytes("xwing_enc ss", &ass, &bss);
    }

    // a real ciphertext to corrupt
    let es = rng.bytes(64);
    let mut ct = vec![0u8; ctb];
    let mut ss = vec![0u8; ssb];
    unsafe {
        assert_eq!(
            r_ed(ct.as_mut_ptr(), ss.as_mut_ptr(), pk.as_ptr(), es.as_ptr()),
            0
        );
    }

    // G6-135: the X25519 half of the ciphertext is small-order -> -1, ss untouched
    for (i, b) in x25519_blocklist().iter().enumerate() {
        for hi in [false, true] {
            let mut v = ct.clone();
            let mut p = *b;
            if hi {
                p[31] |= 0x80;
            }
            v[1088..].copy_from_slice(&p);
            let mut a = canary(ssb);
            let mut bb = canary(ssb);
            let (ra, rb) = unsafe {
                (
                    c_d(a.as_mut_ptr(), v.as_ptr(), sk.as_ptr()),
                    r_d(bb.as_mut_ptr(), v.as_ptr(), sk.as_ptr()),
                )
            };
            eq_i32(
                &format!("xwing_dec [ct_x25519 = blocklist[{i}], bit255={hi}] rc"),
                ra,
                rb,
            );
            assert_eq!(ra, -1, "small-order ct_x25519 must be rejected");
            eq_bytes("xwing_dec ss untouched", &a, &bb);
            assert_eq!(a, canary(ssb), "ss must be untouched");
        }
    }

    // G6-136: corruption of the ML-KEM half is NOT rejected
    for &(byte, bit) in &[(0usize, 0u8), (600, 3), (1087, 7)] {
        let mut v = ct.clone();
        v[byte] ^= 1 << bit;
        let mut a = canary(ssb);
        let mut b = canary(ssb);
        let (ra, rb) = unsafe {
            (
                c_d(a.as_mut_ptr(), v.as_ptr(), sk.as_ptr()),
                r_d(b.as_mut_ptr(), v.as_ptr(), sk.as_ptr()),
            )
        };
        eq_i32("xwing_dec (ML-KEM half corrupt) rc", ra, rb);
        assert_eq!(ra, 0, "ML-KEM corruption is implicitly rejected, not failed");
        eq_bytes("xwing_dec (ML-KEM half corrupt) ss", &a, &b);
        assert_ne!(&a[..], &ss[..], "the ss must differ");
    }
    // ... and a non-small-order corruption of the X25519 half also succeeds
    for &(byte, bit) in &[(1088usize, 1u8), (1100, 4), (1119, 6)] {
        let mut v = ct.clone();
        v[byte] ^= 1 << bit;
        let mut a = canary(ssb);
        let mut b = canary(ssb);
        let (ra, rb) = unsafe {
            (
                c_d(a.as_mut_ptr(), v.as_ptr(), sk.as_ptr()),
                r_d(b.as_mut_ptr(), v.as_ptr(), sk.as_ptr()),
            )
        };
        eq_i32("xwing_dec (X25519 half perturbed) rc", ra, rb);
        eq_bytes("xwing_dec (X25519 half perturbed) ss", &a, &b);
    }
}

/// ERRORS G6-137, G6-138, G6-139, G6-140 — the generic `crypto_kem_*` dispatch
/// inherits X-Wing's behaviour exactly: `_seed_keypair` / `_keypair` always 0,
/// `_enc` -1 for an invalid `pk`, `_dec` -1 only for a small-order X25519 half
/// of the ciphertext and 0 (with a different `ss`) for any other corruption.
#[test]
fn kem_generic_rejections() {
    let _rng_lock = rng_guard();
    setup();
    let mut rng = Rng::new(0x7116);
    let (pkb, skb, ctb, ssb) = (1216usize, 32usize, 1120usize, 32usize);
    let (c_sk, r_sk) = pair::<KemSeedKeypair>("crypto_kem_seed_keypair");
    let (c_kp, r_kp) = pair::<KemKeypair>("crypto_kem_keypair");
    let (c_e, r_e) = pair::<KemEnc>("crypto_kem_enc");
    let (c_d, r_d) = pair::<KemDec>("crypto_kem_dec");

    // G6-137 / G6-138
    for seed in [vec![0u8; 32], vec![0xffu8; 32], rng.bytes(32)] {
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
        eq_i32("crypto_kem_seed_keypair rc", ra, rb);
        assert_eq!(ra, 0);
        eq_bytes("crypto_kem_seed_keypair pk", &apk, &bpk);
        eq_bytes("crypto_kem_seed_keypair sk", &ask, &bsk);
    }
    let mut apk = canary(pkb);
    let mut ask = canary(skb);
    let mut bpk = canary(pkb);
    let mut bsk = canary(skb);
    reset_rngs(0x9a00_0000);
    let ra = unsafe { c_kp(apk.as_mut_ptr(), ask.as_mut_ptr()) };
    reset_rngs(0x9a00_0000);
    let rb = unsafe { r_kp(bpk.as_mut_ptr(), bsk.as_mut_ptr()) };
    eq_i32("crypto_kem_keypair rc", ra, rb);
    assert_eq!(ra, 0);
    eq_bytes("crypto_kem_keypair pk", &apk, &bpk);
    eq_bytes("crypto_kem_keypair sk", &ask, &bsk);

    // G6-139: invalid pk
    let mut bad_pks: Vec<(String, Vec<u8>)> = vec![("all-0xff".into(), vec![0xffu8; pkb])];
    let mut p = apk.clone();
    p[1184..].copy_from_slice(&[0u8; 32]); // small-order X25519 half
    bad_pks.push(("X25519 half all-zero".into(), p));
    let mut p = apk.clone();
    p[0] = 0xff;
    p[1] = 0xff;
    p[2] = 0xff;
    bad_pks.push(("ML-KEM half non-canonical".into(), p));
    for (tag, bp) in &bad_pks {
        let mut act = canary(ctb);
        let mut ass = canary(ssb);
        let mut bct = canary(ctb);
        let mut bss = canary(ssb);
        reset_rngs(0x9b00_0000);
        let ra = unsafe { c_e(act.as_mut_ptr(), ass.as_mut_ptr(), bp.as_ptr()) };
        reset_rngs(0x9b00_0000);
        let rb = unsafe { r_e(bct.as_mut_ptr(), bss.as_mut_ptr(), bp.as_ptr()) };
        eq_i32(&format!("crypto_kem_enc [{tag}] rc"), ra, rb);
        assert_eq!(ra, -1, "crypto_kem_enc must reject [{tag}]");
        eq_bytes("crypto_kem_enc ct", &act, &bct);
        eq_bytes("crypto_kem_enc ss", &ass, &bss);
        assert_eq!(act, canary(ctb));
        assert_eq!(ass, canary(ssb));
    }

    // G6-140: a real ciphertext, then the two corruption kinds
    let mut ct = canary(ctb);
    let mut ss = canary(ssb);
    reset_rngs(0x9c00_0000);
    let mut ct2 = canary(ctb);
    let mut ss2 = canary(ssb);
    unsafe {
        assert_eq!(c_e(ct.as_mut_ptr(), ss.as_mut_ptr(), apk.as_ptr()), 0);
    }
    reset_rngs(0x9c00_0000);
    unsafe {
        assert_eq!(r_e(ct2.as_mut_ptr(), ss2.as_mut_ptr(), bpk.as_ptr()), 0);
    }
    eq_bytes("crypto_kem_enc ct", &ct, &ct2);
    eq_bytes("crypto_kem_enc ss", &ss, &ss2);

    for (i, b) in x25519_blocklist().iter().enumerate() {
        let mut v = ct.clone();
        v[1088..].copy_from_slice(b);
        let mut a = canary(ssb);
        let mut bb = canary(ssb);
        let (ra, rb) = unsafe {
            (
                c_d(a.as_mut_ptr(), v.as_ptr(), ask.as_ptr()),
                r_d(bb.as_mut_ptr(), v.as_ptr(), bsk.as_ptr()),
            )
        };
        eq_i32(&format!("crypto_kem_dec [blocklist[{i}]] rc"), ra, rb);
        assert_eq!(ra, -1);
        eq_bytes("crypto_kem_dec ss untouched", &a, &bb);
        assert_eq!(a, canary(ssb));
    }
    for &(byte, bit) in &[(0usize, 0u8), (700, 5), (1087, 3), (1090, 2)] {
        let mut v = ct.clone();
        v[byte] ^= 1 << bit;
        let mut a = canary(ssb);
        let mut bb = canary(ssb);
        let (ra, rb) = unsafe {
            (
                c_d(a.as_mut_ptr(), v.as_ptr(), ask.as_ptr()),
                r_d(bb.as_mut_ptr(), v.as_ptr(), bsk.as_ptr()),
            )
        };
        eq_i32(&format!("crypto_kem_dec [byte {byte} bit {bit}] rc"), ra, rb);
        assert_eq!(ra, 0, "any other corruption is implicitly rejected");
        eq_bytes("crypto_kem_dec ss", &a, &bb);
        assert_ne!(&a[..], &ss[..]);
    }
}

// ===========================================================================
// crypto_ipcrypt / softaes / implementation pickers
// ===========================================================================

/// ERRORS G6-141, G6-142, G6-143, G6-144, G6-145, G6-147 — every
/// `crypto_ipcrypt_*` entry point is `void` with no validation and no
/// rejection branch: all-zero / all-0xff / random inputs, tweaks and keys are
/// accepted, the keygens are plain `randombytes_buf`, and the constant
/// accessors cannot fail.
#[test]
fn ipcrypt_no_rejection_paths() {
    let _rng_lock = rng_guard();
    setup();
    let mut rng = Rng::new(0x7117);

    let ips: Vec<Vec<u8>> = {
        let mut v = vec![vec![0u8; 16], vec![0xffu8; 16]];
        let mut m = vec![0u8; 16];
        m[10] = 0xff;
        m[11] = 0xff;
        v.push(m);
        for _ in 0..8 {
            v.push(rng.bytes(16));
        }
        v
    };
    let k16: Vec<Vec<u8>> = vec![vec![0u8; 16], vec![0xffu8; 16], rng.bytes(16)];
    let k32: Vec<Vec<u8>> = vec![
        vec![0u8; 32],
        vec![0xffu8; 32],
        rng.bytes(32),
        {
            let mut k = rng.bytes(16);
            let c = k.clone();
            k.extend_from_slice(&c);
            k
        },
    ];
    let t8: Vec<Vec<u8>> = vec![vec![0u8; 8], vec![0xffu8; 8], rng.bytes(8)];
    let t16: Vec<Vec<u8>> = vec![vec![0u8; 16], vec![0xffu8; 16], rng.bytes(16)];

    for name in ["crypto_ipcrypt_encrypt", "crypto_ipcrypt_decrypt"] {
        let (c, r) = pair::<V3>(name);
        for k in &k16 {
            for ip in &ips {
                let mut a = canary(16);
                let mut b = canary(16);
                unsafe {
                    c(a.as_mut_ptr(), ip.as_ptr(), k.as_ptr());
                    r(b.as_mut_ptr(), ip.as_ptr(), k.as_ptr());
                }
                eq_bytes(name, &a, &b);
                assert_ne!(a, canary(16), "{name} wrote nothing");
            }
        }
    }
    let (c, r) = pair::<V4>("crypto_ipcrypt_nd_encrypt");
    for k in &k16 {
        for ip in &ips {
            for t in &t8 {
                let mut a = canary(24);
                let mut b = canary(24);
                unsafe {
                    c(a.as_mut_ptr(), ip.as_ptr(), t.as_ptr(), k.as_ptr());
                    r(b.as_mut_ptr(), ip.as_ptr(), t.as_ptr(), k.as_ptr());
                }
                eq_bytes("crypto_ipcrypt_nd_encrypt", &a, &b);
            }
        }
    }
    let (c, r) = pair::<V3>("crypto_ipcrypt_nd_decrypt");
    for k in &k16 {
        for _ in 0..8 {
            let blob = rng.bytes(24);
            let mut a = canary(16);
            let mut b = canary(16);
            unsafe {
                c(a.as_mut_ptr(), blob.as_ptr(), k.as_ptr());
                r(b.as_mut_ptr(), blob.as_ptr(), k.as_ptr());
            }
            eq_bytes("crypto_ipcrypt_nd_decrypt", &a, &b);
        }
    }
    let (c, r) = pair::<V4>("crypto_ipcrypt_ndx_encrypt");
    for k in &k32 {
        for ip in &ips {
            for t in &t16 {
                let mut a = canary(32);
                let mut b = canary(32);
                unsafe {
                    c(a.as_mut_ptr(), ip.as_ptr(), t.as_ptr(), k.as_ptr());
                    r(b.as_mut_ptr(), ip.as_ptr(), t.as_ptr(), k.as_ptr());
                }
                eq_bytes("crypto_ipcrypt_ndx_encrypt", &a, &b);
            }
        }
    }
    let (c, r) = pair::<V3>("crypto_ipcrypt_ndx_decrypt");
    for k in &k32 {
        for _ in 0..8 {
            let blob = rng.bytes(32);
            let mut a = canary(16);
            let mut b = canary(16);
            unsafe {
                c(a.as_mut_ptr(), blob.as_ptr(), k.as_ptr());
                r(b.as_mut_ptr(), blob.as_ptr(), k.as_ptr());
            }
            eq_bytes("crypto_ipcrypt_ndx_decrypt", &a, &b);
        }
    }
    for name in ["crypto_ipcrypt_pfx_encrypt", "crypto_ipcrypt_pfx_decrypt"] {
        let (c, r) = pair::<V3>(name);
        for k in &k32 {
            for ip in &ips {
                let mut a = canary(16);
                let mut b = canary(16);
                unsafe {
                    c(a.as_mut_ptr(), ip.as_ptr(), k.as_ptr());
                    r(b.as_mut_ptr(), ip.as_ptr(), k.as_ptr());
                }
                eq_bytes(name, &a, &b);
            }
        }
    }

    // G6-145: the keygens
    for (name, n) in [
        ("crypto_ipcrypt_keygen", 16usize),
        ("crypto_ipcrypt_nd_keygen", 16),
        ("crypto_ipcrypt_ndx_keygen", 32),
        ("crypto_ipcrypt_pfx_keygen", 32),
    ] {
        let (c, r) = pair::<V1>(name);
        for seed in 0..6u64 {
            let mut a = canary(n);
            let mut b = canary(n);
            reset_rngs(0x9d00_0000 + seed);
            unsafe { c(a.as_mut_ptr()) };
            reset_rngs(0x9d00_0000 + seed);
            unsafe { r(b.as_mut_ptr()) };
            eq_bytes(name, &a, &b);
        }
    }

    // G6-147: the constant accessors
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

/// ERRORS G6-148, G6-149 — `_crypto_*_pick_best_implementation` always returns
/// 0 (every `#if` body is compiled out), and no `softaes` primitive has a
/// rejection branch, an `assert` or an `abort`: every input, including the
/// all-zero and all-0xff extremes, produces a value and returns normally.
#[test]
fn pickers_and_softaes_have_no_rejection_paths() {
    setup();
    let mut rng = Rng::new(0x7118);
    for name in [
        "_crypto_scalarmult_curve25519_pick_best_implementation",
        "_crypto_stream_chacha20_pick_best_implementation",
        "_crypto_stream_salsa20_pick_best_implementation",
        "_crypto_ipcrypt_pick_best_implementation",
    ] {
        let (c, r) = pair::<unsafe extern "C" fn() -> i32>(name);
        for _ in 0..4 {
            let (a, b) = unsafe { (c(), r()) };
            eq_i32(name, a, b);
            assert_eq!(a, 0, "{name} must return 0");
        }
    }

    let (c_ek1, r_ek1) = pair::<ExpandKey>("_sodium_softaes_expand_key128");
    let (c_ek2, r_ek2) = pair::<ExpandKey>("_sodium_softaes_expand_key256");
    let (c_ik1, r_ik1) = pair::<InvertKs>("_sodium_softaes_invert_key_schedule128");
    let (c_ik2, r_ik2) = pair::<InvertKs>("_sodium_softaes_invert_key_schedule256");
    let (c_imc, r_imc) = pair::<InvMix>("_sodium_softaes_inv_mix_columns");

    for k in [vec![0u8; 16], vec![0xffu8; 16], rng.bytes(16), rng.bytes(16)] {
        let mut a = [AesBlock::default(); 11];
        let mut b = [AesBlock::default(); 11];
        unsafe {
            c_ek1(a.as_mut_ptr(), k.as_ptr());
            r_ek1(b.as_mut_ptr(), k.as_ptr());
            c_ik1(a.as_mut_ptr());
            r_ik1(b.as_mut_ptr());
        }
        assert_eq!(a, b, "softaes AES-128 schedule for key {}", hex(&k));
    }
    for k in [vec![0u8; 32], vec![0xffu8; 32], rng.bytes(32)] {
        let mut a = [AesBlock::default(); 15];
        let mut b = [AesBlock::default(); 15];
        unsafe {
            c_ek2(a.as_mut_ptr(), k.as_ptr());
            r_ek2(b.as_mut_ptr(), k.as_ptr());
            c_ik2(a.as_mut_ptr());
            r_ik2(b.as_mut_ptr());
        }
        assert_eq!(a, b, "softaes AES-256 schedule for key {}", hex(&k));
    }
    let blocks = [
        AesBlock::default(),
        AesBlock { w0: !0, w1: !0, w2: !0, w3: !0 },
        AesBlock { w0: 1, w1: 0, w2: 0, w3: 0 },
    ];
    for b in blocks {
        let (x, y) = unsafe { (c_imc(b), r_imc(b)) };
        assert_eq!(x, y, "softaes_inv_mix_columns");
        for name in [
            "_sodium_softaes_block_encrypt",
            "_sodium_softaes_block_decrypt",
            "_sodium_softaes_block_encryptlast",
            "_sodium_softaes_block_decryptlast",
        ] {
            let (cf, rf) = pair::<Round>(name);
            for rk in blocks {
                let (x, y) = unsafe { (cf(b, rk), rf(b, rk)) };
                assert_eq!(x, y, "{name}");
            }
        }
    }
}

// ===========================================================================
// sodium_misuse() rows — out of process
// ===========================================================================

/// The two reachable `sodium_misuse()` guards of `stream_chacha20.c`
/// (`ERRORS.md` G6-110, G6-111, G6-112, G6-115). The other 6 guards compare
/// against `SODIUM_SIZE_MAX == 2^64-1`, which an `unsigned long long` argument
/// can never exceed on x86-64 — see `documented_unreachable_error_rows`.
const MISUSE_CASES: &[&str] = &[
    // G6-110: crypto_stream_chacha20_ietf, clen > 2^38 (the length alone
    // aborts, before the output buffer is touched)
    "ietf/clen=max+1",
    "ietf/clen=u64max",
    // G6-115: crypto_stream_chacha20_ietf_xor, mlen > 2^38
    "ietf_xor/mlen=max+1",
    "ietf_xor/mlen=u64max",
    // G6-111 / G6-112: the 32-bit counter overflow guard of _ietf_xor_ic
    "ietf_xor_ic/mlen=65,ic=0xffffffff",
    "ietf_xor_ic/mlen=129,ic=0xfffffffe",
    "ietf_xor_ic/mlen=193,ic=0xfffffffd",
    "ietf_xor_ic/mlen=64,ic=0xffffffff+1block",
];

#[test]
fn misuse_child() {
    let Some((tag, lib)) = child_case() else {
        return;
    };
    // `crypto_stream_chacha20_ietf_MESSAGEBYTES_MAX`
    let max = unsafe { sym::<Sz>(lib, "crypto_stream_chacha20_ietf_messagebytes_max")() } as u64;
    let k = [7u8; 32];
    let n = [9u8; 12];
    let m = [0u8; 256];
    let mut out = canary(256);
    set_observation(out.as_ptr(), out.len());

    match tag.as_str() {
        "ietf/clen=max+1" | "ietf/clen=u64max" => {
            let clen = if tag.ends_with("u64max") { u64::MAX } else { max + 1 };
            let f = sym::<Stream>(lib, "crypto_stream_chacha20_ietf");
            let rc = unsafe { f(out.as_mut_ptr(), clen, n.as_ptr(), k.as_ptr()) };
            println!("OBS rc={rc} out={}", hex(&out));
        }
        "ietf_xor/mlen=max+1" | "ietf_xor/mlen=u64max" => {
            let mlen = if tag.ends_with("u64max") { u64::MAX } else { max + 1 };
            let f = sym::<StreamXor>(lib, "crypto_stream_chacha20_ietf_xor");
            let rc = unsafe {
                f(out.as_mut_ptr(), m.as_ptr(), mlen, n.as_ptr(), k.as_ptr())
            };
            println!("OBS rc={rc} out={}", hex(&out));
        }
        "ietf_xor_ic/mlen=65,ic=0xffffffff"
        | "ietf_xor_ic/mlen=129,ic=0xfffffffe"
        | "ietf_xor_ic/mlen=193,ic=0xfffffffd"
        | "ietf_xor_ic/mlen=64,ic=0xffffffff+1block" => {
            let (mlen, ic): (u64, u32) = match tag.as_str() {
                "ietf_xor_ic/mlen=65,ic=0xffffffff" => (65, 0xffff_ffff),
                "ietf_xor_ic/mlen=129,ic=0xfffffffe" => (129, 0xffff_fffe),
                "ietf_xor_ic/mlen=193,ic=0xfffffffd" => (193, 0xffff_fffd),
                // `ceil(64/64) = 1` is fine at ic = 2^32-1, but one more
                // block is not: mlen = 128 needs ic <= 2^32-2.
                _ => (128, 0xffff_ffff),
            };
            let f = sym::<StreamXorIc32>(lib, "crypto_stream_chacha20_ietf_xor_ic");
            let rc = unsafe {
                f(out.as_mut_ptr(), m.as_ptr(), mlen, n.as_ptr(), ic, k.as_ptr())
            };
            println!("OBS rc={rc} out={}", hex(&out));
        }
        other => panic!("unknown tag {other}"),
    }
    use std::io::Write;
    let _ = std::io::stdout().flush();
    std::process::exit(0);
}

#[test]
fn misuse_paths_match() {
    if child_tag().is_some() {
        return;
    }
    setup();
    for &tag in MISUSE_CASES {
        let c = run_child("misuse_child", "c", tag);
        let r = run_child("misuse_child", "r", tag);
        eq_child(tag, &c, &r);
        assert_eq!(
            c.status.code(),
            Some(MISUSE_EXIT),
            "{tag}: C did not reach sodium_misuse (stdout: {}, stderr: {})",
            String::from_utf8_lossy(&c.stdout),
            String::from_utf8_lossy(&c.stderr)
        );
    }
}

// ===========================================================================
// rows whose branch is unreachable in this build
// ===========================================================================

/// ERRORS rows recorded as **not constructible in this build** — documented
/// here so that no row is silently dropped, together with the reason and the
/// surrogate evidence gathered by the other tests in this file.
///
/// * **G6-002, G6-013** — `crypto_scalarmult_curve25519`'s second
///   `return -1` (all-zero `q` after a successful `mult`). Every 255-bit-masked
///   small-order `x` is already in the 7-entry blocklist, so `implementation->
///   mult` never returns 0 with an all-zero `q`. Asserted below by sweeping
///   many random `(n, p)` pairs and checking `q` is never all-zero on success.
/// * **G6-024, G6-034** — `_crypto_scalarmult_ed25519{,_base}`'s `_is_inf(q)`
///   in the **clamped** form: clamping forces `t` into `[2^254, 2^255)` with
///   `t % 8 == 0`, so `t ≡ 0 (mod L)` would need `t` to be a multiple of
///   `8L > 2^255`. Asserted below: every clamped call over the `kL` /
///   `00..00 80` scalar family returns 0.
/// * **G6-050, G6-055, G6-057, G6-060, G6-062** — the `ge25519_is_on_curve`
///   belt-and-braces checks. `ge25519_frombytes` already returns -1 for every
///   off-curve `y`, so the extra test can never fire on its own.
/// * **G6-066** — `crypto_core_ed25519_from_string`'s inner
///   `crypto_core_ed25519_add` failure: both halves come from
///   `ge25519_from_hash`, hence always on-curve.
/// * **G6-067** — `_string_to_points(n > 2)` `abort()`: `n` is only ever the
///   literal 1 or 2 at the two call sites, both `static`.
/// * **G6-068** — `ge25519_elligator2`'s `abort()` when `fe25519_sqrt` fails:
///   `x` is constructed so that `g(x)` is a square.
/// * **G6-096, G6-097** — `assert(h_len <= 0xff)` in `core_h2c.c`. `h_len` is
///   only ever 48, 64 or 96 from the public API.
/// * **G6-104, G6-105, G6-106, G6-107, G6-108, G6-109, G6-119, G6-120** —
///   `mlen/clen > SODIUM_SIZE_MAX` (= `2^64-1` on x86-64). The parameter is
///   `unsigned long long`, so the comparison is never true. Asserted below.
/// * **G6-134** — `crypto_kem_xwing_dec`'s `crypto_kem_mlkem768_dec != 0`:
///   ML-KEM decapsulation uses implicit rejection and has no `return -1` at
///   all (asserted by `kem_mlkem768_rejections`).
/// * **G6-146** — a NULL pointer to any `crypto_ipcrypt_*` entry point is
///   declared `__attribute__((nonnull))`, i.e. undefined behaviour / SIGSEGV,
///   not a checked rejection. Deliberately not exercised.
#[test]
fn documented_unreachable_error_rows() {
    setup();
    let mut rng = Rng::new(0x7119);

    // G6-002 / G6-013: a successful x25519 never yields an all-zero q.
    let (_, x) = pair::<I3>("crypto_scalarmult_curve25519");
    let (_, xb) = pair::<I2>("crypto_scalarmult_curve25519_base");
    let mut points: Vec<Vec<u8>> = vec![vec![0xffu8; 32]];
    let mut basep = vec![0u8; 32];
    basep[0] = 9;
    points.push(basep);
    for _ in 0..16 {
        let s = rng.bytes(32);
        let mut q = vec![0u8; 32];
        unsafe { assert_eq!(xb(q.as_mut_ptr(), s.as_ptr()), 0) };
        points.push(q);
    }
    for p in &points {
        for _ in 0..8 {
            let n = rng.bytes(32);
            let mut q = vec![0u8; 32];
            let rc = unsafe { x(q.as_mut_ptr(), n.as_ptr(), p.as_ptr()) };
            if rc == 0 {
                assert!(
                    q.iter().any(|&b| b != 0),
                    "the second `return -1` of crypto_scalarmult_curve25519 became reachable"
                );
            }
        }
    }

    // G6-024 / G6-034: the clamped ed25519 forms never reach `_is_inf(q)`.
    let (_, ed) = pair::<I3>("crypto_scalarmult_ed25519");
    let (_, edb) = pair::<I2>("crypto_scalarmult_ed25519_base");
    let base = unhex32(ED_BASE);
    let mut scalars: Vec<[u8; 32]> = Vec::new();
    let mut top = [0u8; 32];
    top[31] = 0x80;
    scalars.push(top);
    for k in 1..8u32 {
        scalars.push(mul_small(&ell(), k));
    }
    for _ in 0..200 {
        let mut s = [0u8; 32];
        rng.fill(&mut s);
        scalars.push(s);
    }
    for n in &scalars {
        if n.iter().all(|&b| b == 0) {
            continue; // that is the `sodium_is_zero(n)` row, not `_is_inf`
        }
        let mut q = [0u8; 32];
        let rc = unsafe { ed(q.as_mut_ptr(), n.as_ptr(), base.as_ptr()) };
        assert_eq!(
            rc, 0,
            "clamped crypto_scalarmult_ed25519 reached _is_inf for n = {}",
            hex(n)
        );
        let mut q = [0u8; 32];
        let rc = unsafe { edb(q.as_mut_ptr(), n.as_ptr()) };
        assert_eq!(
            rc, 0,
            "clamped crypto_scalarmult_ed25519_base reached _is_inf for n = {}",
            hex(n)
        );
    }

    // G6-104 .. G6-109, G6-119, G6-120: the non-ietf size limit is 2^64-1, so
    // `mlen > SODIUM_SIZE_MAX` is not expressible in an `unsigned long long`.
    for p in [
        "crypto_stream_chacha20",
        "crypto_stream_xchacha20",
        "crypto_stream_salsa20",
        "crypto_stream_xsalsa20",
        "crypto_stream",
    ] {
        let (c, r) = pair::<Sz>(&format!("{p}_messagebytes_max"));
        let (a, b) = unsafe { (c(), r()) };
        eq_usize(&format!("{p}_messagebytes_max"), a, b);
        assert_eq!(
            a,
            usize::MAX,
            "{p}_messagebytes_max is not 2^64-1, so its guard may be reachable"
        );
    }

    // G6-050 / G6-055 / G6-057 / G6-060 / G6-062: `ge25519_frombytes` already
    // rejects every off-curve `y`, so `ge25519_is_on_curve` never fires alone.
    // Surrogate: `_is_valid_point` and `_add` agree on the rejection for every
    // canonical `y` that is not on the curve.
    let (_, valid) = pair::<I1c>("crypto_core_ed25519_is_valid_point");
    let (_, add) = pair::<I3>("crypto_core_ed25519_add");
    let good = ed_valid_points(&mut rng, 3);
    let mut off_curve = 0usize;
    for _ in 0..3000 {
        let mut p = rng.bytes(32);
        p[31] &= 0x7f;
        p[0] = p[0].wrapping_sub(p[0] % 4); // stay canonical-ish
        let mut r0 = [0u8; 32];
        let rc_add = unsafe { add(r0.as_mut_ptr(), p.as_ptr(), good[0].as_ptr()) };
        let rc_valid = unsafe { valid(p.as_ptr()) };
        if rc_add == -1 {
            off_curve += 1;
            assert_eq!(
                rc_valid, 0,
                "frombytes rejected {} but is_valid_point accepted it",
                hex(&p)
            );
        }
    }
    assert!(off_curve > 0, "no off-curve y was generated");

    // G6-134: ML-KEM decapsulation has no `return -1`; see
    // `kem_mlkem768_rejections` for the exhaustive sweep.
    // G6-146: NULL-pointer UB, deliberately not exercised.
}
