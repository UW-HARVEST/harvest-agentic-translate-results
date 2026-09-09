//! Phase C — error-path differential tests for group G5 (ERRORS.md rows 509-638),
//! plus generic FFI boundary probes (NULL where the C never dereferences,
//! zero / oversized lengths, one step past every documented range, and
//! out-of-range `hash_alg` enum ints).
//!
//! Rows whose rejection is a *return value* are compared directly.
//! Rows whose rejection is a `sodium_misuse()` abort are compared by
//! re-executing this binary in two child processes (one per library) and
//! comparing how the two children terminated.
#![allow(clippy::too_many_arguments)]

mod common;
use common::*;
use libloading::Symbol;
use std::os::raw::{c_char, c_int};
use std::ptr;

const SEED: u64 = 0x0E11_0E11_5A5A_0008;

// ===========================================================================
// helpers (kept local: `tests/common/mod.rs` is shared with other agents)
// ===========================================================================
fn hx(s: &str) -> Vec<u8> {
    assert!(s.len() % 2 == 0, "odd hex length");
    (0..s.len() / 2)
        .map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap())
        .collect()
}

const L_HEX: &str = "edd3f55c1a631258d69cf7a2def9de1400000000000000000000000000000010";
const LM1_HEX: &str = "ecd3f55c1a631258d69cf7a2def9de1400000000000000000000000000000010";
const ONE_HEX: &str = "0100000000000000000000000000000000000000000000000000000000000000";
const ZERO_HEX: &str = "0000000000000000000000000000000000000000000000000000000000000000";
const R_BASE_HEX: &str = "e2f2ae0a6abc4e71a884a961c500515f58e30b6aa582dd8db6a65945e08d2d76";
const ED_BASE_HEX: &str = "5866666666666666666666666666666666666666666666666666666666666666";

/// The curve25519 blocklist (`has_small_order`, x25519_ref10.c). ERRORS 509-515.
fn x25519_blocklist() -> Vec<Vec<u8>> {
    vec![
        hx("0000000000000000000000000000000000000000000000000000000000000000"),
        hx("0100000000000000000000000000000000000000000000000000000000000000"),
        hx("e0eb7a7c3b41b8ae1656e3faf19fc46ada098deb9c32b1fd866205165f49b800"),
        hx("5f9c95bca3508c24b1d0b1559c83ef5b04445cc4581c8e86d8224eddd09f1157"),
        hx("ecffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f"),
        hx("edffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f"),
        hx("eeffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f"),
    ]
}

/// The 8 ed25519 torsion (small-order) encodings + their high-bit variants.
/// ERRORS 522 / 582 / 593 / 603.
fn ed25519_small_order() -> Vec<Vec<u8>> {
    vec![
        hx("0100000000000000000000000000000000000000000000000000000000000000"),
        hx("0000000000000000000000000000000000000000000000000000000000000000"),
        hx("0000000000000000000000000000000000000000000000000000000000000080"),
        hx("ecffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f"),
        hx("ecffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"),
        hx("26e8958fc2b227b045c3f489f2ef98f0d5dfac05d3c63339b13802886d53fc05"),
        hx("26e8958fc2b227b045c3f489f2ef98f0d5dfac05d3c63339b13802886d53fc85"),
        hx("c7176a703d4dd84fba3c0b760d10670f2a2053fa2c39ccc64ec7fd7792ac037a"),
        hx("c7176a703d4dd84fba3c0b760d10670f2a2053fa2c39ccc64ec7fd7792ac03fa"),
    ]
}

/// Non-canonical ed25519 y encodings (`ge25519_is_canonical == 0`).
/// ERRORS 520 / 580 / 600.
fn ed25519_noncanonical() -> Vec<Vec<u8>> {
    vec![
        hx("edffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f"),
        hx("eeffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f"),
        hx("efffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f"),
        hx("ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"),
        hx("fdffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"),
    ]
}

/// Bad ristretto255 encodings, grouped by the rejection they trigger.
/// ERRORS 530-535 / 621-626.
fn ristretto_bad() -> Vec<(&'static str, Vec<u8>)> {
    vec![
        // bit 255 set (row 530 / 621)
        ("hi-bit", hx("0000000000000000000000000000000000000000000000000000000000000080")),
        ("hi-bit", hx("ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff")),
        // negative field element, p[0] odd (row 531 / 622)
        ("odd", hx("0100000000000000000000000000000000000000000000000000000000000000")),
        ("odd", hx("01ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff")),
        ("odd", hx("ed57ffd8c914fb201471d1c3d245ce3c746fcbe63a3679d51b6a516ebebe0e20")),
        // non-canonical field encoding (row 532 / 623)
        ("noncanon", hx("edffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f")),
        ("noncanon", hx("f3ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff")),
        ("noncanon", hx("00ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff")),
        // v*u2^2 non-square (row 533 / 624)
        ("nonsquare", hx("26948d35ca62e643e26a83177332e6b6afeb9d08e4268b650f1f5bbd8d81d371")),
        ("nonsquare", hx("4eac077a713c57b4f4397629a4145982c661f48044dd3f96427d40b147d9742f")),
        ("nonsquare", hx("de6a7b00deadc788eb6b6c8d20c0ae96c2f2019078fa604fee5b87d6e989ad7b")),
        // T = X*Y negative (row 534 / 625)
        ("negT", hx("3eb858e78f5a7254d8c9731174a94f76755fd3941c0ac93735c07ba14579630e")),
        ("negT", hx("a45fdc55c76448c049a1ab33f17023edfb2be3581e9c7aade8a6125215e04220")),
    ]
}

// ===========================================================================
// FFI signatures
// ===========================================================================
type V1 = unsafe extern "C" fn(*mut u8);
type V2 = unsafe extern "C" fn(*mut u8, *const u8);
type V3 = unsafe extern "C" fn(*mut u8, *const u8, *const u8);
type I2 = unsafe extern "C" fn(*mut u8, *const u8) -> c_int;
type I3 = unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> c_int;
type Chk = unsafe extern "C" fn(*const u8) -> c_int;
type Getter = unsafe extern "C" fn() -> usize;
type SeedKeypair = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> c_int;
type BoxEasy =
    unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, *const u8, *const u8) -> c_int;
type BoxDet = unsafe extern "C" fn(
    *mut u8,
    *mut u8,
    *const u8,
    u64,
    *const u8,
    *const u8,
    *const u8,
) -> c_int;
type BoxOpenDet = unsafe extern "C" fn(
    *mut u8,
    *const u8,
    *const u8,
    u64,
    *const u8,
    *const u8,
    *const u8,
) -> c_int;
type Afternm = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, *const u8) -> c_int;
type DetAfternm =
    unsafe extern "C" fn(*mut u8, *mut u8, *const u8, u64, *const u8, *const u8) -> c_int;
type OpenDetAfternm =
    unsafe extern "C" fn(*mut u8, *const u8, *const u8, u64, *const u8, *const u8) -> c_int;
type Seal = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8) -> c_int;
type SealOpen = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, *const u8) -> c_int;
type KxSess = unsafe extern "C" fn(*mut u8, *mut u8, *const u8, *const u8, *const u8) -> c_int;
type SignFn = unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> c_int;
type VerifyFn = unsafe extern "C" fn(*const u8, *const u8, u64, *const u8) -> c_int;
type PhInit = unsafe extern "C" fn(*mut u8) -> c_int;
type PhUpdate = unsafe extern "C" fn(*mut u8, *const u8, u64) -> c_int;
type PhCreate = unsafe extern "C" fn(*mut u8, *mut u8, *mut u64, *const u8) -> c_int;
type PhVerify = unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> c_int;
type FromStr = unsafe extern "C" fn(*mut u8, *const u8, usize, *const u8, usize, c_int) -> c_int;

type P<F> = (Symbol<'static, F>, Symbol<'static, F>);

// ===========================================================================
// generic differential runners (identical semantics to t07)
// ===========================================================================
#[track_caller]
unsafe fn run_i2(f: &P<I2>, olen: usize, inp: &[u8], ctx: &str) -> (i32, Vec<u8>) {
    let mut co = vec![0xAAu8; olen];
    let mut ro = vec![0xAAu8; olen];
    let a = (f.0)(co.as_mut_ptr(), inp.as_ptr());
    let b = (f.1)(ro.as_mut_ptr(), inp.as_ptr());
    eq_i32(ctx, a, b);
    eq_bytes(ctx, &co, &ro);
    (a, co)
}

#[track_caller]
unsafe fn run_i3(f: &P<I3>, olen: usize, x: &[u8], y: &[u8], ctx: &str) -> (i32, Vec<u8>) {
    let mut co = vec![0xAAu8; olen];
    let mut ro = vec![0xAAu8; olen];
    let a = (f.0)(co.as_mut_ptr(), x.as_ptr(), y.as_ptr());
    let b = (f.1)(ro.as_mut_ptr(), x.as_ptr(), y.as_ptr());
    eq_i32(ctx, a, b);
    eq_bytes(ctx, &co, &ro);
    (a, co)
}

#[track_caller]
unsafe fn run_v2(f: &P<V2>, olen: usize, inp: &[u8], ctx: &str) -> Vec<u8> {
    let mut co = vec![0xAAu8; olen];
    let mut ro = vec![0xAAu8; olen];
    (f.0)(co.as_mut_ptr(), inp.as_ptr());
    (f.1)(ro.as_mut_ptr(), inp.as_ptr());
    eq_bytes(ctx, &co, &ro);
    co
}

#[track_caller]
unsafe fn run_v3(f: &P<V3>, olen: usize, x: &[u8], y: &[u8], ctx: &str) -> Vec<u8> {
    let mut co = vec![0xAAu8; olen];
    let mut ro = vec![0xAAu8; olen];
    (f.0)(co.as_mut_ptr(), x.as_ptr(), y.as_ptr());
    (f.1)(ro.as_mut_ptr(), x.as_ptr(), y.as_ptr());
    eq_bytes(ctx, &co, &ro);
    co
}

#[track_caller]
unsafe fn run_chk(f: &P<Chk>, inp: &[u8], ctx: &str) -> i32 {
    let a = (f.0)(inp.as_ptr());
    let b = (f.1)(inp.as_ptr());
    eq_i32(ctx, a, b);
    a
}

#[track_caller]
unsafe fn getter(name: &str) -> usize {
    let (c, r) = pair::<Getter>(name);
    let a = c();
    let b = r();
    assert_eq!(a, b, "{name}: C={a} Rust={b}");
    a
}

/// `crypto_box_seed_keypair` on a fixed seed (also compares C/Rust).
unsafe fn box_keypair(seed_byte: u8) -> (Vec<u8>, Vec<u8>) {
    let (c, r) = pair::<SeedKeypair>("crypto_box_seed_keypair");
    let seed = vec![seed_byte; 32];
    let mut cpk = [0xAAu8; 32];
    let mut csk = [0xAAu8; 32];
    let mut rpk = [0xAAu8; 32];
    let mut rsk = [0xAAu8; 32];
    let a = c(cpk.as_mut_ptr(), csk.as_mut_ptr(), seed.as_ptr());
    let b = r(rpk.as_mut_ptr(), rsk.as_mut_ptr(), seed.as_ptr());
    eq_i32("box_seed_keypair", a, b);
    eq_bytes("box_seed_keypair pk", &cpk, &rpk);
    eq_bytes("box_seed_keypair sk", &csk, &rsk);
    (cpk.to_vec(), csk.to_vec())
}

unsafe fn ed_keypair(byte: u8) -> (Vec<u8>, Vec<u8>) {
    let (c, r) = pair::<SeedKeypair>("crypto_sign_ed25519_seed_keypair");
    let seed = vec![byte; 32];
    let mut cpk = [0u8; 32];
    let mut csk = [0u8; 64];
    let mut rpk = [0u8; 32];
    let mut rsk = [0u8; 64];
    let a = c(cpk.as_mut_ptr(), csk.as_mut_ptr(), seed.as_ptr());
    let b = r(rpk.as_mut_ptr(), rsk.as_mut_ptr(), seed.as_ptr());
    eq_i32("ed_keypair", a, b);
    eq_bytes("ed_keypair pk", &cpk, &rpk);
    eq_bytes("ed_keypair sk", &csk, &rsk);
    (cpk.to_vec(), csk.to_vec())
}

/// Every `hash_alg` value that must be rejected across the FFI boundary.
const BAD_ALGS: [c_int; 10] = [0, -1, 3, 4, 5, 255, 256, 65536, c_int::MIN, c_int::MAX];

// ===========================================================================
// ERRORS rows 509-519 — crypto_scalarmult_curve25519
// ===========================================================================
#[test]
fn g5e_scalarmult_curve25519_small_order() {
    unsafe {
        let mut rng = Rng::new(SEED);
        for name in ["crypto_scalarmult", "crypto_scalarmult_curve25519"] {
            let f = pair::<I3>(name);
            // rows 509-515: every blocklist entry, with several scalars
            for (i, p) in x25519_blocklist().iter().enumerate() {
                for (j, n) in [
                    hx(ONE_HEX),
                    vec![0x08u8; 32],
                    vec![0u8; 32],
                    vec![0xffu8; 32],
                    rng.bytes(32),
                ]
                .iter()
                .enumerate()
                {
                    let (ret, q) = run_i3(&f, 32, n, p, &format!("row509 {name} block[{i}] n[{j}]"));
                    eq_i32(
                        &format!("row509 {name} block[{i}] n[{j}] must be -1"),
                        ret,
                        -1,
                    );
                    // "q untouched" -- our prefill must survive
                    assert_eq!(
                        q,
                        vec![0xAAu8; 32],
                        "row509 {name} block[{i}]: q must be untouched"
                    );
                }
                // row 516: the same entries with bit 255 set
                let mut p2 = p.clone();
                p2[31] |= 0x80;
                let (ret, q) = run_i3(&f, 32, &hx(ONE_HEX), &p2, &format!("row516 {name}[{i}]"));
                eq_i32(&format!("row516 {name}[{i}] hi-bit still blocked"), ret, -1);
                assert_eq!(q, vec![0xAAu8; 32]);
            }
            // one step past the blocklist: a *valid* p must succeed
            let fb = pair::<I2>("crypto_scalarmult_base");
            let (_, valid) = run_i2(&fb, 32, &vec![0x37u8; 32], "valid pk");
            let (ret, _) = run_i3(&f, 32, &hx(ONE_HEX), &valid, &format!("{name} valid p"));
            eq_i32(&format!("{name} valid p succeeds"), ret, 0);
            // near-miss encodings of the blocklist entries
            for (i, p) in x25519_blocklist().iter().enumerate() {
                let mut p2 = p.clone();
                p2[0] = p2[0].wrapping_add(1);
                run_i3(&f, 32, &hx(ONE_HEX), &p2, &format!("{name} block[{i}]+1"));
                let mut p3 = p.clone();
                p3[15] ^= 0x01;
                run_i3(&f, 32, &hx(ONE_HEX), &p3, &format!("{name} block[{i}] mid-flip"));
            }
            // rows 517/518 note: the "ladder output all-zero" and
            // "implementation->mult() != 0" branches are LCOV_EXCL in the C and
            // cannot be reached from the public API (every small-order p is
            // already caught by has_small_order, and the ref10 mult always
            // returns 0). A broad random sweep asserts C/Rust agreement on the
            // return value regardless of which branch fires.
            for i in 0..600 {
                let n = rng.bytes(32);
                let p = rng.bytes(32);
                run_i3(&f, 32, &n, &p, &format!("row517 {name} random p[{i}]"));
            }
        }

        // row 519: _base has no rejection path at all
        for base in ["crypto_scalarmult_base", "crypto_scalarmult_curve25519_base"] {
            let f = pair::<I2>(base);
            for (i, n) in [
                vec![0u8; 32],
                vec![0xffu8; 32],
                hx(ONE_HEX),
                hx(L_HEX),
                vec![0x08u8; 32],
            ]
            .iter()
            .enumerate()
            {
                let (ret, _) = run_i2(&f, 32, n, &format!("row519 {base}[{i}]"));
                eq_i32(&format!("row519 {base}[{i}] always 0"), ret, 0);
            }
            for i in 0..400 {
                let n = rng.bytes(32);
                let (ret, _) = run_i2(&f, 32, &n, &format!("row519 {base} rand[{i}]"));
                eq_i32(&format!("row519 {base} rand[{i}] always 0"), ret, 0);
            }
        }
    }
}

// ===========================================================================
// ERRORS rows 520-529 — crypto_scalarmult_ed25519{,_noclamp,_base,_base_noclamp}
// ===========================================================================
#[test]
fn g5e_scalarmult_ed25519_rejections() {
    install_det_random();
    unsafe {
        let m = pair::<I3>("crypto_scalarmult_ed25519");
        let mn = pair::<I3>("crypto_scalarmult_ed25519_noclamp");
        let b = pair::<I2>("crypto_scalarmult_ed25519_base");
        let bn = pair::<I2>("crypto_scalarmult_ed25519_base_noclamp");
        let add = pair::<I3>("crypto_core_ed25519_add");
        let mut rng = Rng::new(SEED ^ 0x11);
        let good_n = hx("0700000000000000000000000000000000000000000000000000000000000000");
        let (pk, _) = ed_keypair(0x71);

        // row 520: non-canonical p
        for (i, p) in ed25519_noncanonical().iter().enumerate() {
            for (nm, f) in [("ed25519", &m), ("ed25519_noclamp", &mn)] {
                let (ret, q) = run_i3(f, 32, &good_n, p, &format!("row520 {nm} noncanon[{i}]"));
                eq_i32(&format!("row520 {nm} noncanon[{i}]"), ret, -1);
                assert_eq!(q, vec![0xAAu8; 32], "row520 {nm}: q must be untouched");
            }
        }

        // row 521: p does not decode to a point (~50% of random strings)
        let mut undecodable = 0usize;
        for i in 0..600 {
            let p = rng.bytes(32);
            let (ra, _) = run_i3(&m, 32, &good_n, &p, &format!("row521 ed25519 rand[{i}]"));
            let (rb, _) = run_i3(&mn, 32, &good_n, &p, &format!("row521 noclamp rand[{i}]"));
            eq_i32(&format!("row521 clamp/noclamp same verdict[{i}]"), ra, rb);
            if ra != 0 {
                undecodable += 1;
            }
        }
        assert!(undecodable > 100, "row521: expected many undecodable p");
        // the concrete example from the table
        let (ret, _) = run_i3(&m, 32, &good_n, &vec![0x02u8; 32], "row521 0202..02");
        eq_i32("row521 0202..02", ret, -1);

        // row 522: small-order p
        for (i, p) in ed25519_small_order().iter().enumerate() {
            for (nm, f) in [("ed25519", &m), ("ed25519_noclamp", &mn)] {
                let (ret, q) = run_i3(f, 32, &good_n, p, &format!("row522 {nm} torsion[{i}]"));
                eq_i32(&format!("row522 {nm} torsion[{i}]"), ret, -1);
                assert_eq!(q, vec![0xAAu8; 32], "row522 {nm}: q must be untouched");
            }
        }

        // row 523: p on the curve but of composite order
        // (valid pk + an order-8 point, built with crypto_core_ed25519_add,
        //  which -- ERRORS row 609 -- performs no subgroup check)
        let ivp = pair::<Chk>("crypto_core_ed25519_is_valid_point");
        let mut composite = 0usize;
        for (i, t) in ed25519_small_order().iter().enumerate() {
            let (ra, r) = run_i3(&add, 32, &pk, t, &format!("row609 add(pk,torsion[{i}])"));
            if ra != 0 {
                continue; // some torsion encodings are not decodable by add()
            }
            let valid = run_chk(&ivp, &r, &format!("row604 is_valid_point(pk+t[{i}])"));
            let (rb, _) = run_i3(&m, 32, &good_n, &r, &format!("row523 ed25519 pk+t[{i}]"));
            let (rc, _) = run_i3(&mn, 32, &good_n, &r, &format!("row523 noclamp pk+t[{i}]"));
            eq_i32(&format!("row523 clamp/noclamp agree[{i}]"), rb, rc);
            if valid == 0 && r != pk {
                composite += 1;
                eq_i32(&format!("row523 composite-order p rejected[{i}]"), rb, -1);
                eq_i32(&format!("row604 is_valid_point == 0 [{i}]"), valid, 0);
            }
        }
        assert!(composite >= 2, "row523: expected composite-order points");

        // row 524: n = 0 for the clamped variant -- returns -1 but q IS clobbered
        let (ret, q) = run_i3(&m, 32, &vec![0u8; 32], &pk, "row524 n=0");
        eq_i32("row524 n=0", ret, -1);
        assert_ne!(q, vec![0xAAu8; 32], "row524: q IS written before the -1");
        // row 527: same for _base
        let (ret, q) = run_i2(&b, 32, &vec![0u8; 32], "row527 base n=0");
        eq_i32("row527 base n=0", ret, -1);
        assert_ne!(q, vec![0xAAu8; 32], "row527: q IS clobbered");

        // row 525 / 528: n = 0 for the noclamp variants
        let (ret, q) = run_i3(&mn, 32, &vec![0u8; 32], &pk, "row525 noclamp n=0");
        eq_i32("row525 noclamp n=0", ret, -1);
        eq_bytes("row525 noclamp q == identity", &hx(ONE_HEX), &q);
        let (ret, q) = run_i2(&bn, 32, &vec![0u8; 32], "row528 base_noclamp n=0");
        eq_i32("row528 base_noclamp n=0", ret, -1);
        eq_bytes("row528 base_noclamp q == identity", &hx(ONE_HEX), &q);

        // row 526 / 529: n = L and other nonzero n = 0 mod L (after n[31] &= 127)
        let l = hx(L_HEX);
        let mut ln = Vec::new();
        ln.push(l.clone());
        // L with bit 255 set: masked back to L
        let mut l2 = l.clone();
        l2[31] |= 0x80;
        ln.push(l2);
        for (i, n) in ln.iter().enumerate() {
            let (ret, q) = run_i3(&mn, 32, n, &pk, &format!("row526 noclamp n=L[{i}]"));
            eq_i32(&format!("row526 noclamp n=L[{i}]"), ret, -1);
            eq_bytes(&format!("row526 q == identity[{i}]"), &hx(ONE_HEX), &q);
            let (ret, q) = run_i2(&bn, 32, n, &format!("row529 base_noclamp n=L[{i}]"));
            eq_i32(&format!("row529 base_noclamp n=L[{i}]"), ret, -1);
            eq_bytes(&format!("row529 q == identity[{i}]"), &hx(ONE_HEX), &q);
        }
        // one step past: L+1 and L-1 must succeed
        for (nm, nh) in [("L-1", LM1_HEX), ("L+1", "eed3f55c1a631258d69cf7a2def9de1400000000000000000000000000000010")] {
            let n = hx(nh);
            let (ret, _) = run_i2(&bn, 32, &n, &format!("base_noclamp n={nm}"));
            eq_i32(&format!("base_noclamp n={nm} succeeds"), ret, 0);
            let (ret, _) = run_i3(&mn, 32, &n, &pk, &format!("noclamp n={nm}"));
            eq_i32(&format!("noclamp n={nm} succeeds"), ret, 0);
        }
        // 2L (mod 2^255) also reduces to 0 mod L
        let sadd = pair::<V3>("crypto_core_ed25519_scalar_add");
        let _ = &sadd;
        // n whose low 255 bits are 2L: 2L = 0x1ba7...20, still < 2^255
        let two_l = {
            let mut acc = [0u8; 32];
            let mut carry = 0u16;
            for i in 0..32 {
                let v = (l[i] as u16) * 2 + carry;
                acc[i] = (v & 0xff) as u8;
                carry = v >> 8;
            }
            acc.to_vec()
        };
        let (ret, _) = run_i2(&bn, 32, &two_l, "row529 base_noclamp n=2L");
        eq_i32("row529 base_noclamp n=2L", ret, -1);

        // ed25519 base has no p argument, so the only rejection is n == 0
        for i in 0..300 {
            let n = rng.bytes(32);
            let (ra, _) = run_i2(&b, 32, &n, &format!("base rand[{i}]"));
            eq_i32(&format!("base rand[{i}] succeeds"), ra, 0);
            run_i2(&bn, 32, &n, &format!("base_noclamp rand[{i}]"));
        }
    }
}

// ===========================================================================
// ERRORS rows 530-538 — crypto_scalarmult_ristretto255
// ===========================================================================
#[test]
fn g5e_scalarmult_ristretto255_rejections() {
    unsafe {
        let m = pair::<I3>("crypto_scalarmult_ristretto255");
        let b = pair::<I2>("crypto_scalarmult_ristretto255_base");
        let good_n = hx("0700000000000000000000000000000000000000000000000000000000000000");

        // rows 530-535
        for (i, (kind, p)) in ristretto_bad().iter().enumerate() {
            let (ret, q) = run_i3(&m, 32, &good_n, p, &format!("row530 ristretto {kind}[{i}]"));
            eq_i32(&format!("row530 ristretto {kind}[{i}] must be -1"), ret, -1);
            assert_eq!(q, vec![0xAAu8; 32], "row530 {kind}[{i}]: q untouched");
        }
        // row 535: fe25519_iszero(h->Y) -- searched for by brute force below;
        // any random encoding that decodes must also be accepted, so the
        // random sweep asserts C/Rust agreement over all classes at once.
        let mut rng = Rng::new(SEED ^ 0x22);
        let mut rejected = 0usize;
        for i in 0..800 {
            let p = rng.bytes(32);
            let (ret, _) = run_i3(&m, 32, &good_n, &p, &format!("row530 ristretto rand[{i}]"));
            if ret != 0 {
                rejected += 1;
            }
        }
        assert!(rejected > 200, "row530: expected many rejected encodings");

        // row 536: valid p but n = 0 / n = L
        let base = hx(R_BASE_HEX);
        for (nm, n) in [
            ("0", vec![0u8; 32]),
            ("L", hx(L_HEX)),
            ("L|hi", {
                let mut v = hx(L_HEX);
                v[31] |= 0x80;
                v
            }),
        ] {
            let (ret, q) = run_i3(&m, 32, &n, &base, &format!("row536 ristretto n={nm}"));
            eq_i32(&format!("row536 ristretto n={nm}"), ret, -1);
            eq_bytes(&format!("row536 q == identity (n={nm})"), &hx(ZERO_HEX), &q);
            // rows 537, 538
            let (ret, q) = run_i2(&b, 32, &n, &format!("row537 ristretto_base n={nm}"));
            eq_i32(&format!("row537 ristretto_base n={nm}"), ret, -1);
            eq_bytes(&format!("row537 q == identity (n={nm})"), &hx(ZERO_HEX), &q);
        }
        // one step past: L-1 and L+1 succeed
        for (nm, nh) in [
            ("L-1", LM1_HEX),
            ("L+1", "eed3f55c1a631258d69cf7a2def9de1400000000000000000000000000000010"),
        ] {
            let n = hx(nh);
            let (ret, _) = run_i2(&b, 32, &n, &format!("ristretto_base n={nm}"));
            eq_i32(&format!("ristretto_base n={nm} succeeds"), ret, 0);
            let (ret, _) = run_i3(&m, 32, &n, &base, &format!("ristretto n={nm}"));
            eq_i32(&format!("ristretto n={nm} succeeds"), ret, 0);
        }
        // the identity as p: decodes fine, but n*identity == identity -> -1
        let (ret, q) = run_i3(&m, 32, &good_n, &hx(ZERO_HEX), "ristretto p=identity");
        eq_i32("ristretto p=identity", ret, -1);
        eq_bytes("ristretto p=identity q", &hx(ZERO_HEX), &q);
    }
}

// ===========================================================================
// ERRORS rows 539-543, 551-552, 556, 562 — crypto_box small-order pk
// ===========================================================================
#[test]
fn g5e_box_small_order_pk() {
    unsafe {
        let (apk, ask) = box_keypair(0xA1);
        let (bpk, bsk) = box_keypair(0xB2);
        let n = vec![0x5Au8; 24];
        let m = vec![0x11u8; 64];

        for variant in ["", "curve25519xchacha20poly1305_"] {
            let bef = pair::<I3>(&format!("crypto_box_{variant}beforenm"));
            let det = pair::<BoxDet>(&format!("crypto_box_{variant}detached"));
            let odet = pair::<BoxOpenDet>(&format!("crypto_box_{variant}open_detached"));
            let easy = pair::<BoxEasy>(&format!("crypto_box_{variant}easy"));
            let oeasy = pair::<BoxEasy>(&format!("crypto_box_{variant}open_easy"));

            for (i, bad) in x25519_blocklist().iter().enumerate() {
                // rows 539/540: beforenm -> -1, k untouched
                let (ret, k) = run_i3(&bef, 32, bad, &ask, &format!("row539 {variant}beforenm[{i}]"));
                eq_i32(&format!("row539 {variant}beforenm[{i}]"), ret, -1);
                assert_eq!(k, vec![0xAAu8; 32], "row539: k must be untouched");

                // row 541: detached -> -1
                let mut cc = vec![0xAAu8; 64];
                let mut rc = vec![0xAAu8; 64];
                let mut cmac = [0xAAu8; 16];
                let mut rmac = [0xAAu8; 16];
                let a = (det.0)(
                    cc.as_mut_ptr(),
                    cmac.as_mut_ptr(),
                    m.as_ptr(),
                    64,
                    n.as_ptr(),
                    bad.as_ptr(),
                    ask.as_ptr(),
                );
                let bb = (det.1)(
                    rc.as_mut_ptr(),
                    rmac.as_mut_ptr(),
                    m.as_ptr(),
                    64,
                    n.as_ptr(),
                    bad.as_ptr(),
                    ask.as_ptr(),
                );
                eq_i32(&format!("row541 {variant}detached[{i}]"), a, bb);
                eq_bytes(&format!("row541 {variant}detached c[{i}]"), &cc, &rc);
                eq_bytes(&format!("row541 {variant}detached mac[{i}]"), &cmac, &rmac);
                eq_i32(&format!("row541 {variant}detached[{i}] == -1"), a, -1);

                // row 543: easy -> -1
                let mut cc = vec![0xAAu8; 80];
                let mut rc = vec![0xAAu8; 80];
                let a = (easy.0)(
                    cc.as_mut_ptr(),
                    m.as_ptr(),
                    64,
                    n.as_ptr(),
                    bad.as_ptr(),
                    ask.as_ptr(),
                );
                let bb = (easy.1)(
                    rc.as_mut_ptr(),
                    m.as_ptr(),
                    64,
                    n.as_ptr(),
                    bad.as_ptr(),
                    ask.as_ptr(),
                );
                eq_i32(&format!("row543 {variant}easy[{i}]"), a, bb);
                eq_bytes(&format!("row543 {variant}easy c[{i}]"), &cc, &rc);
                eq_i32(&format!("row543 {variant}easy[{i}] == -1"), a, -1);

                // rows 551/552: open_easy / open_detached with a small-order pk
                let mut cm = vec![0xAAu8; 64];
                let mut rm = vec![0xAAu8; 64];
                let a = (oeasy.0)(
                    cm.as_mut_ptr(),
                    cc.as_ptr(),
                    80,
                    n.as_ptr(),
                    bad.as_ptr(),
                    bsk.as_ptr(),
                );
                let bb = (oeasy.1)(
                    rm.as_mut_ptr(),
                    rc.as_ptr(),
                    80,
                    n.as_ptr(),
                    bad.as_ptr(),
                    bsk.as_ptr(),
                );
                eq_i32(&format!("row551 {variant}open_easy[{i}]"), a, bb);
                eq_bytes(&format!("row551 {variant}open_easy m[{i}]"), &cm, &rm);
                eq_i32(&format!("row551 {variant}open_easy[{i}] == -1"), a, -1);
                let mac = [0u8; 16];
                let a = (odet.0)(
                    cm.as_mut_ptr(),
                    cc.as_ptr(),
                    mac.as_ptr(),
                    64,
                    n.as_ptr(),
                    bad.as_ptr(),
                    bsk.as_ptr(),
                );
                let bb = (odet.1)(
                    rm.as_mut_ptr(),
                    rc.as_ptr(),
                    mac.as_ptr(),
                    64,
                    n.as_ptr(),
                    bad.as_ptr(),
                    bsk.as_ptr(),
                );
                eq_i32(&format!("row551 {variant}open_detached[{i}]"), a, bb);
                eq_i32(&format!("row551 {variant}open_detached[{i}] == -1"), a, -1);
            }

            // row 556: correct ciphertext, wrong sk / wrong pk / wrong nonce
            let mut ct = vec![0u8; 80];
            assert_eq!(
                (easy.0)(
                    ct.as_mut_ptr(),
                    m.as_ptr(),
                    64,
                    n.as_ptr(),
                    bpk.as_ptr(),
                    ask.as_ptr()
                ),
                0
            );
            let (wpk, wsk) = box_keypair(0xC3);
            let mut wrong_n = n.clone();
            wrong_n[0] ^= 1;
            for (nm, pk_u, sk_u, nn) in [
                ("wrong sk", &apk, &wsk, &n),
                ("wrong pk", &wpk, &bsk, &n),
                ("wrong nonce", &apk, &bsk, &wrong_n),
            ] {
                let mut cm = vec![0xAAu8; 64];
                let mut rm = vec![0xAAu8; 64];
                let a = (oeasy.0)(
                    cm.as_mut_ptr(),
                    ct.as_ptr(),
                    80,
                    nn.as_ptr(),
                    pk_u.as_ptr(),
                    sk_u.as_ptr(),
                );
                let bb = (oeasy.1)(
                    rm.as_mut_ptr(),
                    ct.as_ptr(),
                    80,
                    nn.as_ptr(),
                    pk_u.as_ptr(),
                    sk_u.as_ptr(),
                );
                eq_i32(&format!("row556 {variant}open_easy {nm}"), a, bb);
                eq_bytes(&format!("row556 {variant}open_easy {nm} m"), &cm, &rm);
                eq_i32(&format!("row556 {variant}open_easy {nm} == -1"), a, -1);
            }
        }

        // row 562: the NaCl-style API also rejects a small-order pk
        let boxf = pair::<BoxEasy>("crypto_box");
        let openf = pair::<BoxEasy>("crypto_box_open");
        let mut padded = vec![0u8; 96];
        for (i, bad) in x25519_blocklist().iter().enumerate() {
            let mut cc = vec![0xAAu8; 96];
            let mut rc = vec![0xAAu8; 96];
            let a = (boxf.0)(
                cc.as_mut_ptr(),
                padded.as_ptr(),
                96,
                n.as_ptr(),
                bad.as_ptr(),
                ask.as_ptr(),
            );
            let bb = (boxf.1)(
                rc.as_mut_ptr(),
                padded.as_ptr(),
                96,
                n.as_ptr(),
                bad.as_ptr(),
                ask.as_ptr(),
            );
            eq_i32(&format!("row562 crypto_box small-order[{i}]"), a, bb);
            eq_bytes(&format!("row562 crypto_box c[{i}]"), &cc, &rc);
            eq_i32(&format!("row562 crypto_box[{i}] == -1"), a, -1);
            let a = (openf.0)(
                cc.as_mut_ptr(),
                padded.as_ptr(),
                96,
                n.as_ptr(),
                bad.as_ptr(),
                bsk.as_ptr(),
            );
            let bb = (openf.1)(
                rc.as_mut_ptr(),
                padded.as_ptr(),
                96,
                n.as_ptr(),
                bad.as_ptr(),
                bsk.as_ptr(),
            );
            eq_i32(&format!("row562 crypto_box_open small-order[{i}]"), a, bb);
            eq_i32(&format!("row562 crypto_box_open[{i}] == -1"), a, -1);
        }
        padded[0] = 0; // keep `padded` used
    }
}

// ===========================================================================
// ERRORS rows 547-550, 553-555 — short ciphertexts and MAC failures
// ===========================================================================
#[test]
fn g5e_box_open_short_and_mac_failure() {
    unsafe {
        let (apk, ask) = box_keypair(0xD4);
        let (bpk, bsk) = box_keypair(0xE5);
        let n = vec![0x77u8; 24];
        let m = vec![0x22u8; 64];
        let mut rng = Rng::new(SEED ^ 0x33);

        for variant in ["", "curve25519xchacha20poly1305_"] {
            let easy = pair::<BoxEasy>(&format!("crypto_box_{variant}easy"));
            let oeasy = pair::<BoxEasy>(&format!("crypto_box_{variant}open_easy"));
            let oeasy_a = pair::<Afternm>(&format!("crypto_box_{variant}open_easy_afternm"));
            let odet_a =
                pair::<OpenDetAfternm>(&format!("crypto_box_{variant}open_detached_afternm"));
            let det_a = pair::<DetAfternm>(&format!("crypto_box_{variant}detached_afternm"));
            let bef = pair::<I3>(&format!("crypto_box_{variant}beforenm"));
            let (rk, k) = run_i3(&bef, 32, &bpk, &ask, "beforenm");
            assert_eq!(rk, 0);

            // rows 547-550: clen in 0..15 -> -1; clen == 16 is the first valid one
            for clen in 0u64..=16 {
                let cbuf = vec![0u8; 32];
                let mut cm = vec![0xAAu8; 32];
                let mut rm = vec![0xAAu8; 32];
                let a = (oeasy.0)(
                    cm.as_mut_ptr(),
                    cbuf.as_ptr(),
                    clen,
                    n.as_ptr(),
                    apk.as_ptr(),
                    bsk.as_ptr(),
                );
                let b = (oeasy.1)(
                    rm.as_mut_ptr(),
                    cbuf.as_ptr(),
                    clen,
                    n.as_ptr(),
                    apk.as_ptr(),
                    bsk.as_ptr(),
                );
                let ctx = format!("row547 {variant}open_easy clen={clen}");
                eq_i32(&ctx, a, b);
                eq_bytes(&ctx, &cm, &rm);
                eq_i32(&format!("{ctx} == -1"), a, -1);
                if clen < 16 {
                    assert_eq!(cm, vec![0xAAu8; 32], "{ctx}: m must be untouched");
                }
                // row 548/550: the afternm form
                let a = (oeasy_a.0)(cm.as_mut_ptr(), cbuf.as_ptr(), clen, n.as_ptr(), k.as_ptr());
                let b = (oeasy_a.1)(rm.as_mut_ptr(), cbuf.as_ptr(), clen, n.as_ptr(), k.as_ptr());
                let ctx = format!("row548 {variant}open_easy_afternm clen={clen}");
                eq_i32(&ctx, a, b);
                eq_bytes(&ctx, &cm, &rm);
                eq_i32(&format!("{ctx} == -1"), a, -1);
            }

            // a genuine ciphertext to corrupt
            let mut ct = vec![0u8; 80];
            assert_eq!(
                (easy.0)(
                    ct.as_mut_ptr(),
                    m.as_ptr(),
                    64,
                    n.as_ptr(),
                    bpk.as_ptr(),
                    ask.as_ptr()
                ),
                0
            );

            // row 554: flip a bit in the MAC (c[0..16]) or in the body (c[16..])
            for pos in 0..80usize {
                let mut bad = ct.clone();
                bad[pos] ^= 0x01;
                let mut cm = vec![0xAAu8; 64];
                let mut rm = vec![0xAAu8; 64];
                let a = (oeasy.0)(
                    cm.as_mut_ptr(),
                    bad.as_ptr(),
                    80,
                    n.as_ptr(),
                    apk.as_ptr(),
                    bsk.as_ptr(),
                );
                let b = (oeasy.1)(
                    rm.as_mut_ptr(),
                    bad.as_ptr(),
                    80,
                    n.as_ptr(),
                    apk.as_ptr(),
                    bsk.as_ptr(),
                );
                let ctx = format!("row554 {variant}open_easy flip@{pos}");
                eq_i32(&ctx, a, b);
                eq_bytes(&ctx, &cm, &rm);
                eq_i32(&format!("{ctx} == -1"), a, -1);
                assert_eq!(cm, vec![0xAAu8; 64], "{ctx}: m must NOT be written");
                let a = (oeasy_a.0)(cm.as_mut_ptr(), bad.as_ptr(), 80, n.as_ptr(), k.as_ptr());
                let b = (oeasy_a.1)(rm.as_mut_ptr(), bad.as_ptr(), 80, n.as_ptr(), k.as_ptr());
                let ctx = format!("row554 {variant}open_easy_afternm flip@{pos}");
                eq_i32(&ctx, a, b);
                eq_bytes(&ctx, &cm, &rm);
                eq_i32(&format!("{ctx} == -1"), a, -1);
            }

            // rows 553/555: open_detached_afternm tag mismatch in mac / c / n / k
            let mut body = vec![0u8; 64];
            let mut mac = [0u8; 16];
            assert_eq!(
                (det_a.0)(
                    body.as_mut_ptr(),
                    mac.as_mut_ptr(),
                    m.as_ptr(),
                    64,
                    n.as_ptr(),
                    k.as_ptr()
                ),
                0
            );
            // sanity: the untouched triple verifies
            let mut cm = vec![0xAAu8; 64];
            let mut rm = vec![0xAAu8; 64];
            let a = (odet_a.0)(
                cm.as_mut_ptr(),
                body.as_ptr(),
                mac.as_ptr(),
                64,
                n.as_ptr(),
                k.as_ptr(),
            );
            let b = (odet_a.1)(
                rm.as_mut_ptr(),
                body.as_ptr(),
                mac.as_ptr(),
                64,
                n.as_ptr(),
                k.as_ptr(),
            );
            eq_i32("row553 baseline", a, b);
            eq_bytes("row553 baseline", &cm, &rm);
            eq_i32("row553 baseline == 0", a, 0);
            // mac = 16 x 0x00
            let zmac = [0u8; 16];
            let mut cm = vec![0xAAu8; 64];
            let mut rm = vec![0xAAu8; 64];
            let a = (odet_a.0)(
                cm.as_mut_ptr(),
                body.as_ptr(),
                zmac.as_ptr(),
                64,
                n.as_ptr(),
                k.as_ptr(),
            );
            let b = (odet_a.1)(
                rm.as_mut_ptr(),
                body.as_ptr(),
                zmac.as_ptr(),
                64,
                n.as_ptr(),
                k.as_ptr(),
            );
            eq_i32(&format!("row553 {variant}zero mac"), a, b);
            eq_bytes(&format!("row553 {variant}zero mac"), &cm, &rm);
            eq_i32(&format!("row553 {variant}zero mac == -1"), a, -1);
            assert_eq!(cm, vec![0xAAu8; 64], "row553: m must not be written");
            // single-bit flips in each argument
            for (what, bits) in [("mac", 16 * 8), ("c", 64 * 8), ("n", 24 * 8), ("k", 32 * 8)] {
                for _ in 0..24 {
                    let bit = rng.below(bits);
                    let (byte, mask) = (bit / 8, 1u8 << (bit % 8));
                    let mut mac2 = mac;
                    let mut body2 = body.clone();
                    let mut n2 = n.clone();
                    let mut k2 = k.clone();
                    match what {
                        "mac" => mac2[byte] ^= mask,
                        "c" => body2[byte] ^= mask,
                        "n" => n2[byte] ^= mask,
                        _ => k2[byte] ^= mask,
                    }
                    let mut cm = vec![0xAAu8; 64];
                    let mut rm = vec![0xAAu8; 64];
                    let a = (odet_a.0)(
                        cm.as_mut_ptr(),
                        body2.as_ptr(),
                        mac2.as_ptr(),
                        64,
                        n2.as_ptr(),
                        k2.as_ptr(),
                    );
                    let b = (odet_a.1)(
                        rm.as_mut_ptr(),
                        body2.as_ptr(),
                        mac2.as_ptr(),
                        64,
                        n2.as_ptr(),
                        k2.as_ptr(),
                    );
                    let ctx = format!("row553 {variant}flip {what} bit {bit}");
                    eq_i32(&ctx, a, b);
                    eq_bytes(&ctx, &cm, &rm);
                    eq_i32(&format!("{ctx} == -1"), a, -1);
                }
            }
            // clen = 0 with a bogus mac (boundary: the shortest detached input)
            let mut cm = vec![0xAAu8; 1];
            let mut rm = vec![0xAAu8; 1];
            let a = (odet_a.0)(
                cm.as_mut_ptr(),
                body.as_ptr(),
                zmac.as_ptr(),
                0,
                n.as_ptr(),
                k.as_ptr(),
            );
            let b = (odet_a.1)(
                rm.as_mut_ptr(),
                body.as_ptr(),
                zmac.as_ptr(),
                0,
                n.as_ptr(),
                k.as_ptr(),
            );
            eq_i32(&format!("{variant}open_detached_afternm clen=0"), a, b);
            eq_bytes(&format!("{variant}open_detached_afternm clen=0"), &cm, &rm);
        }
    }
}

// ===========================================================================
// ERRORS rows 557-561, 563 — NaCl zero-padded API length checks
// ===========================================================================
#[test]
fn g5e_box_nacl_length_checks() {
    unsafe {
        let (apk, ask) = box_keypair(0xF6);
        let (bpk, bsk) = box_keypair(0x07);
        let n = vec![0x3Cu8; 24];
        let bef = pair::<I3>("crypto_box_beforenm");
        let (rk, k) = run_i3(&bef, 32, &bpk, &ask, "beforenm");
        assert_eq!(rk, 0);

        let boxes: Vec<(&str, P<BoxEasy>)> = vec![
            ("crypto_box", pair::<BoxEasy>("crypto_box")),
            (
                "crypto_box_curve25519xsalsa20poly1305",
                pair::<BoxEasy>("crypto_box_curve25519xsalsa20poly1305"),
            ),
        ];
        let opens: Vec<(&str, P<BoxEasy>)> = vec![
            ("crypto_box_open", pair::<BoxEasy>("crypto_box_open")),
            (
                "crypto_box_curve25519xsalsa20poly1305_open",
                pair::<BoxEasy>("crypto_box_curve25519xsalsa20poly1305_open"),
            ),
        ];
        let afternms: Vec<(&str, P<Afternm>)> = vec![
            ("crypto_box_afternm", pair::<Afternm>("crypto_box_afternm")),
            (
                "crypto_box_curve25519xsalsa20poly1305_afternm",
                pair::<Afternm>("crypto_box_curve25519xsalsa20poly1305_afternm"),
            ),
        ];
        let oafternms: Vec<(&str, P<Afternm>)> = vec![
            (
                "crypto_box_open_afternm",
                pair::<Afternm>("crypto_box_open_afternm"),
            ),
            (
                "crypto_box_curve25519xsalsa20poly1305_open_afternm",
                pair::<Afternm>("crypto_box_curve25519xsalsa20poly1305_open_afternm"),
            ),
        ];

        // rows 557-560: mlen / clen in 0..31 -> -1; 32 is the first valid length
        let buf = vec![0u8; 64];
        for len in 0u64..=32 {
            for (nm, f) in boxes.iter() {
                let mut cc = vec![0xAAu8; 64];
                let mut rc = vec![0xAAu8; 64];
                let a = (f.0)(
                    cc.as_mut_ptr(),
                    buf.as_ptr(),
                    len,
                    n.as_ptr(),
                    bpk.as_ptr(),
                    ask.as_ptr(),
                );
                let b = (f.1)(
                    rc.as_mut_ptr(),
                    buf.as_ptr(),
                    len,
                    n.as_ptr(),
                    bpk.as_ptr(),
                    ask.as_ptr(),
                );
                let ctx = format!("row557 {nm} mlen={len}");
                eq_i32(&ctx, a, b);
                eq_bytes(&ctx, &cc, &rc);
                eq_i32(&format!("{ctx} expected"), a, if len < 32 { -1 } else { 0 });
            }
            for (nm, f) in afternms.iter() {
                let mut cc = vec![0xAAu8; 64];
                let mut rc = vec![0xAAu8; 64];
                let a = (f.0)(cc.as_mut_ptr(), buf.as_ptr(), len, n.as_ptr(), k.as_ptr());
                let b = (f.1)(rc.as_mut_ptr(), buf.as_ptr(), len, n.as_ptr(), k.as_ptr());
                let ctx = format!("row558 {nm} mlen={len}");
                eq_i32(&ctx, a, b);
                eq_bytes(&ctx, &cc, &rc);
                eq_i32(&format!("{ctx} expected"), a, if len < 32 { -1 } else { 0 });
            }
            for (nm, f) in opens.iter() {
                let mut cm = vec![0xAAu8; 64];
                let mut rm = vec![0xAAu8; 64];
                let a = (f.0)(
                    cm.as_mut_ptr(),
                    buf.as_ptr(),
                    len,
                    n.as_ptr(),
                    apk.as_ptr(),
                    bsk.as_ptr(),
                );
                let b = (f.1)(
                    rm.as_mut_ptr(),
                    buf.as_ptr(),
                    len,
                    n.as_ptr(),
                    apk.as_ptr(),
                    bsk.as_ptr(),
                );
                let ctx = format!("row559 {nm} clen={len}");
                eq_i32(&ctx, a, b);
                eq_bytes(&ctx, &cm, &rm);
                eq_i32(&format!("{ctx} == -1"), a, -1);
            }
            for (nm, f) in oafternms.iter() {
                let mut cm = vec![0xAAu8; 64];
                let mut rm = vec![0xAAu8; 64];
                let a = (f.0)(cm.as_mut_ptr(), buf.as_ptr(), len, n.as_ptr(), k.as_ptr());
                let b = (f.1)(rm.as_mut_ptr(), buf.as_ptr(), len, n.as_ptr(), k.as_ptr());
                let ctx = format!("row560 {nm} clen={len}");
                eq_i32(&ctx, a, b);
                eq_bytes(&ctx, &cm, &rm);
                eq_i32(&format!("{ctx} == -1"), a, -1);
            }
        }

        // row 561: Poly1305 verify failure on c+16 (flip any bit in c[16..])
        let mlen = 96usize;
        let mut m = vec![0u8; mlen];
        for (i, b) in m[32..].iter_mut().enumerate() {
            *b = i as u8;
        }
        let mut ct = vec![0u8; mlen];
        assert_eq!(
            (boxes[0].1 .0)(
                ct.as_mut_ptr(),
                m.as_ptr(),
                mlen as u64,
                n.as_ptr(),
                bpk.as_ptr(),
                ask.as_ptr()
            ),
            0
        );
        for pos in 16..mlen {
            let mut bad = ct.clone();
            bad[pos] ^= 0x80;
            for (nm, f) in opens.iter() {
                let mut cm = vec![0xAAu8; mlen];
                let mut rm = vec![0xAAu8; mlen];
                let a = (f.0)(
                    cm.as_mut_ptr(),
                    bad.as_ptr(),
                    mlen as u64,
                    n.as_ptr(),
                    apk.as_ptr(),
                    bsk.as_ptr(),
                );
                let b = (f.1)(
                    rm.as_mut_ptr(),
                    bad.as_ptr(),
                    mlen as u64,
                    n.as_ptr(),
                    apk.as_ptr(),
                    bsk.as_ptr(),
                );
                let ctx = format!("row561 {nm} flip@{pos}");
                eq_i32(&ctx, a, b);
                eq_bytes(&ctx, &cm, &rm);
                eq_i32(&format!("{ctx} == -1"), a, -1);
            }
            for (nm, f) in oafternms.iter() {
                let mut cm = vec![0xAAu8; mlen];
                let mut rm = vec![0xAAu8; mlen];
                let a = (f.0)(
                    cm.as_mut_ptr(),
                    bad.as_ptr(),
                    mlen as u64,
                    n.as_ptr(),
                    k.as_ptr(),
                );
                let b = (f.1)(
                    rm.as_mut_ptr(),
                    bad.as_ptr(),
                    mlen as u64,
                    n.as_ptr(),
                    k.as_ptr(),
                );
                let ctx = format!("row561 {nm} flip@{pos}");
                eq_i32(&ctx, a, b);
                eq_bytes(&ctx, &cm, &rm);
                eq_i32(&format!("{ctx} == -1"), a, -1);
            }
        }

        // row 563: a non-zero-padded m is NOT rejected -- documented divergence
        // risk. Only C/Rust agreement is asserted (C returns 0).
        let mut rng = Rng::new(SEED ^ 0x44);
        for i in 0..40 {
            let m = rng.bytes(64);
            let mut cc = vec![0xAAu8; 64];
            let mut rc = vec![0xAAu8; 64];
            let a = (boxes[0].1 .0)(
                cc.as_mut_ptr(),
                m.as_ptr(),
                64,
                n.as_ptr(),
                bpk.as_ptr(),
                ask.as_ptr(),
            );
            let b = (boxes[0].1 .1)(
                rc.as_mut_ptr(),
                m.as_ptr(),
                64,
                n.as_ptr(),
                bpk.as_ptr(),
                ask.as_ptr(),
            );
            eq_i32(&format!("row563 crypto_box unpadded[{i}]"), a, b);
            eq_bytes(&format!("row563 crypto_box unpadded[{i}]"), &cc, &rc);
            eq_i32(&format!("row563[{i}] accepted"), a, 0);
        }
    }
}

// ===========================================================================
// ERRORS rows 565, 566, 568-573 — seal / seal_open
// ===========================================================================
#[test]
fn g5e_box_seal_errors() {
    install_det_random();
    unsafe {
        let (pk, sk) = box_keypair(0x18);
        let m = vec![0x5Bu8; 64];
        let mut rng = Rng::new(SEED ^ 0x55);

        for variant in ["", "curve25519xchacha20poly1305_"] {
            let seal = pair::<Seal>(&format!("crypto_box_{variant}seal"));
            let open = pair::<SealOpen>(&format!("crypto_box_{variant}seal_open"));

            // row 565: crypto_box_keypair inside seal cannot fail -> seal succeeds
            let mut cc = vec![0xAAu8; 112];
            let mut rc = vec![0xAAu8; 112];
            det_reseed(0x515A_0001);
            let a = (seal.0)(cc.as_mut_ptr(), m.as_ptr(), 64, pk.as_ptr());
            det_reseed(0x515A_0001);
            let b = (seal.1)(rc.as_mut_ptr(), m.as_ptr(), 64, pk.as_ptr());
            eq_i32(&format!("row565 {variant}seal"), a, b);
            eq_bytes(&format!("row565 {variant}seal"), &cc, &rc);
            eq_i32(&format!("row565 {variant}seal == 0"), a, 0);
            let good = cc.clone();

            // rows 566/568: recipient pk small-order -> -1, but c[0..32] written
            for (i, bad) in x25519_blocklist().iter().enumerate() {
                let mut cc = vec![0xAAu8; 112];
                let mut rc = vec![0xAAu8; 112];
                det_reseed(0x515A_0002);
                let a = (seal.0)(cc.as_mut_ptr(), m.as_ptr(), 64, bad.as_ptr());
                det_reseed(0x515A_0002);
                let b = (seal.1)(rc.as_mut_ptr(), m.as_ptr(), 64, bad.as_ptr());
                let ctx = format!("row566 {variant}seal small-order[{i}]");
                eq_i32(&ctx, a, b);
                eq_bytes(&ctx, &cc, &rc);
                eq_i32(&format!("{ctx} == -1"), a, -1);
                assert_ne!(
                    &cc[..32],
                    &[0xAAu8; 32][..],
                    "{ctx}: the ephemeral pk IS written even on failure"
                );
                assert_eq!(
                    &cc[32..],
                    &vec![0xAAu8; 80][..],
                    "{ctx}: only c[0..32] is written"
                );
            }

            // rows 569/570: clen in 0..47 -> -1; 48 is the first valid length
            for clen in 0u64..=48 {
                let mut cm = vec![0xAAu8; 64];
                let mut rm = vec![0xAAu8; 64];
                let a = (open.0)(
                    cm.as_mut_ptr(),
                    good.as_ptr(),
                    clen,
                    pk.as_ptr(),
                    sk.as_ptr(),
                );
                let b = (open.1)(
                    rm.as_mut_ptr(),
                    good.as_ptr(),
                    clen,
                    pk.as_ptr(),
                    sk.as_ptr(),
                );
                let ctx = format!("row569 {variant}seal_open clen={clen}");
                eq_i32(&ctx, a, b);
                eq_bytes(&ctx, &cm, &rm);
                // clen == 48 corresponds to a 0-byte message and the stored MAC
                // covers 64 bytes, so it still fails -- but it must fail the
                // *MAC* check, not the length check, and identically in both.
                eq_i32(&format!("{ctx} == -1"), a, -1);
            }
            // the full length succeeds
            let mut cm = vec![0xAAu8; 64];
            let mut rm = vec![0xAAu8; 64];
            let a = (open.0)(
                cm.as_mut_ptr(),
                good.as_ptr(),
                112,
                pk.as_ptr(),
                sk.as_ptr(),
            );
            let b = (open.1)(
                rm.as_mut_ptr(),
                good.as_ptr(),
                112,
                pk.as_ptr(),
                sk.as_ptr(),
            );
            eq_i32(&format!("{variant}seal_open full"), a, b);
            eq_bytes(&format!("{variant}seal_open full"), &cm, &rm);
            eq_i32(&format!("{variant}seal_open full == 0"), a, 0);

            // row 571: the embedded ephemeral pk is small-order
            for (i, bad) in x25519_blocklist().iter().enumerate() {
                let mut bad_c = good.clone();
                bad_c[..32].copy_from_slice(bad);
                let mut cm = vec![0xAAu8; 64];
                let mut rm = vec![0xAAu8; 64];
                let a = (open.0)(
                    cm.as_mut_ptr(),
                    bad_c.as_ptr(),
                    112,
                    pk.as_ptr(),
                    sk.as_ptr(),
                );
                let b = (open.1)(
                    rm.as_mut_ptr(),
                    bad_c.as_ptr(),
                    112,
                    pk.as_ptr(),
                    sk.as_ptr(),
                );
                let ctx = format!("row571 {variant}seal_open bad epk[{i}]");
                eq_i32(&ctx, a, b);
                eq_bytes(&ctx, &cm, &rm);
                eq_i32(&format!("{ctx} == -1"), a, -1);
            }

            // rows 572/573: MAC failure -- flip any bit; also wrong recipient key
            for pos in 0..112usize {
                let mut bad_c = good.clone();
                bad_c[pos] ^= 0x01;
                let mut cm = vec![0xAAu8; 64];
                let mut rm = vec![0xAAu8; 64];
                let a = (open.0)(
                    cm.as_mut_ptr(),
                    bad_c.as_ptr(),
                    112,
                    pk.as_ptr(),
                    sk.as_ptr(),
                );
                let b = (open.1)(
                    rm.as_mut_ptr(),
                    bad_c.as_ptr(),
                    112,
                    pk.as_ptr(),
                    sk.as_ptr(),
                );
                let ctx = format!("row572 {variant}seal_open flip@{pos}");
                eq_i32(&ctx, a, b);
                eq_bytes(&ctx, &cm, &rm);
                eq_i32(&format!("{ctx} == -1"), a, -1);
            }
            let (wpk, wsk) = box_keypair(0x29);
            for (nm, p_u, s_u) in [("wrong pk", &wpk, &sk), ("wrong sk", &pk, &wsk)] {
                let mut cm = vec![0xAAu8; 64];
                let mut rm = vec![0xAAu8; 64];
                let a = (open.0)(
                    cm.as_mut_ptr(),
                    good.as_ptr(),
                    112,
                    p_u.as_ptr(),
                    s_u.as_ptr(),
                );
                let b = (open.1)(
                    rm.as_mut_ptr(),
                    good.as_ptr(),
                    112,
                    p_u.as_ptr(),
                    s_u.as_ptr(),
                );
                let ctx = format!("row572 {variant}seal_open {nm}");
                eq_i32(&ctx, a, b);
                eq_bytes(&ctx, &cm, &rm);
                eq_i32(&format!("{ctx} == -1"), a, -1);
            }
            // random garbage of a valid length
            for i in 0..60 {
                let c = rng.bytes(112);
                let mut cm = vec![0xAAu8; 64];
                let mut rm = vec![0xAAu8; 64];
                let a = (open.0)(cm.as_mut_ptr(), c.as_ptr(), 112, pk.as_ptr(), sk.as_ptr());
                let b = (open.1)(rm.as_mut_ptr(), c.as_ptr(), 112, pk.as_ptr(), sk.as_ptr());
                let ctx = format!("row572 {variant}seal_open garbage[{i}]");
                eq_i32(&ctx, a, b);
                eq_bytes(&ctx, &cm, &rm);
            }
        }
    }
}

// ===========================================================================
// ERRORS rows 576-578 — crypto_kx
// ===========================================================================
#[test]
fn g5e_kx_errors() {
    unsafe {
        let (cpk, csk) = {
            let (c, r) = pair::<SeedKeypair>("crypto_kx_seed_keypair");
            let seed = vec![0x4Bu8; 32];
            let mut cpk = [0u8; 32];
            let mut csk = [0u8; 32];
            let mut rpk = [0u8; 32];
            let mut rsk = [0u8; 32];
            let a = c(cpk.as_mut_ptr(), csk.as_mut_ptr(), seed.as_ptr());
            let b = r(rpk.as_mut_ptr(), rsk.as_mut_ptr(), seed.as_ptr());
            eq_i32("kx_seed_keypair", a, b);
            eq_bytes("kx_seed_keypair pk", &cpk, &rpk);
            eq_bytes("kx_seed_keypair sk", &csk, &rsk);
            (cpk.to_vec(), csk.to_vec())
        };
        let cli = pair::<KxSess>("crypto_kx_client_session_keys");
        let srv = pair::<KxSess>("crypto_kx_server_session_keys");

        // rows 576/577: the peer public key is small-order -> -1, rx/tx untouched
        for (role, f) in [("client", &cli), ("server", &srv)] {
            for (i, bad) in x25519_blocklist().iter().enumerate() {
                let mut c_rx = [0xAAu8; 32];
                let mut c_tx = [0xAAu8; 32];
                let mut r_rx = [0xAAu8; 32];
                let mut r_tx = [0xAAu8; 32];
                let a = (f.0)(
                    c_rx.as_mut_ptr(),
                    c_tx.as_mut_ptr(),
                    cpk.as_ptr(),
                    csk.as_ptr(),
                    bad.as_ptr(),
                );
                let b = (f.1)(
                    r_rx.as_mut_ptr(),
                    r_tx.as_mut_ptr(),
                    cpk.as_ptr(),
                    csk.as_ptr(),
                    bad.as_ptr(),
                );
                let ctx = format!("row576 kx_{role}_session_keys small-order[{i}]");
                eq_i32(&ctx, a, b);
                eq_bytes(&format!("{ctx} rx"), &c_rx, &r_rx);
                eq_bytes(&format!("{ctx} tx"), &c_tx, &r_tx);
                eq_i32(&format!("{ctx} == -1"), a, -1);
                assert_eq!(c_rx, [0xAAu8; 32], "{ctx}: rx must be untouched");
                assert_eq!(c_tx, [0xAAu8; 32], "{ctx}: tx must be untouched");
                // with a single NULL output the -1 must still be returned
                let mut buf = [0xAAu8; 32];
                let mut buf2 = [0xAAu8; 32];
                let a = (f.0)(
                    buf.as_mut_ptr(),
                    ptr::null_mut(),
                    cpk.as_ptr(),
                    csk.as_ptr(),
                    bad.as_ptr(),
                );
                let b = (f.1)(
                    buf2.as_mut_ptr(),
                    ptr::null_mut(),
                    cpk.as_ptr(),
                    csk.as_ptr(),
                    bad.as_ptr(),
                );
                eq_i32(&format!("{ctx} tx=NULL"), a, b);
                eq_bytes(&format!("{ctx} tx=NULL buf"), &buf, &buf2);
                eq_i32(&format!("{ctx} tx=NULL == -1"), a, -1);
            }
        }

        // row 578: keypair / seed_keypair have no failure path at all
        let (c, r) = pair::<SeedKeypair>("crypto_kx_seed_keypair");
        let mut rng = Rng::new(SEED ^ 0x66);
        let mut seeds: Vec<Vec<u8>> = vec![vec![0u8; 32], vec![0xffu8; 32]];
        for _ in 0..200 {
            seeds.push(rng.bytes(32));
        }
        for (i, seed) in seeds.iter().enumerate() {
            let mut cpk = [0xAAu8; 32];
            let mut csk = [0xAAu8; 32];
            let mut rpk = [0xAAu8; 32];
            let mut rsk = [0xAAu8; 32];
            let a = c(cpk.as_mut_ptr(), csk.as_mut_ptr(), seed.as_ptr());
            let b = r(rpk.as_mut_ptr(), rsk.as_mut_ptr(), seed.as_ptr());
            eq_i32(&format!("row578 kx_seed_keypair[{i}]"), a, b);
            eq_bytes(&format!("row578 pk[{i}]"), &cpk, &rpk);
            eq_bytes(&format!("row578 sk[{i}]"), &csk, &rsk);
            eq_i32(&format!("row578[{i}] always 0"), a, 0);
        }
        install_det_random();
        let (kc, kr) = pair::<unsafe extern "C" fn(*mut u8, *mut u8) -> c_int>("crypto_kx_keypair");
        for i in 0..40u64 {
            let mut cpk = [0xAAu8; 32];
            let mut csk = [0xAAu8; 32];
            let mut rpk = [0xAAu8; 32];
            let mut rsk = [0xAAu8; 32];
            det_reseed(0x4B00 + i);
            let a = kc(cpk.as_mut_ptr(), csk.as_mut_ptr());
            det_reseed(0x4B00 + i);
            let b = kr(rpk.as_mut_ptr(), rsk.as_mut_ptr());
            eq_i32(&format!("row578 kx_keypair[{i}]"), a, b);
            eq_bytes(&format!("row578 kx_keypair pk[{i}]"), &cpk, &rpk);
            eq_bytes(&format!("row578 kx_keypair sk[{i}]"), &csk, &rsk);
            eq_i32(&format!("row578 kx_keypair[{i}] always 0"), a, 0);
        }
    }
}

// ===========================================================================
// ERRORS rows 579-587 — crypto_sign_ed25519_verify_detached
// ===========================================================================
#[test]
fn g5e_sign_verify_rejections() {
    unsafe {
        let (pk, sk) = ed_keypair(0x6D);
        let sg = pair::<SignFn>("crypto_sign_ed25519_detached");
        let mut rng = Rng::new(SEED ^ 0x77);
        let m = vec![0x91u8; 64];
        let mut sig = [0u8; 64];
        assert_eq!(
            (sg.0)(
                sig.as_mut_ptr(),
                ptr::null_mut(),
                m.as_ptr(),
                64,
                sk.as_ptr()
            ),
            0
        );

        for vname in [
            "crypto_sign_ed25519_verify_detached",
            "crypto_sign_verify_detached",
        ] {
            let vf = pair::<VerifyFn>(vname);
            let run = |s: &[u8], msg: &[u8], p: &[u8], ctx: &str| -> i32 {
                let a = (vf.0)(s.as_ptr(), msg.as_ptr(), msg.len() as u64, p.as_ptr());
                let b = (vf.1)(s.as_ptr(), msg.as_ptr(), msg.len() as u64, p.as_ptr());
                eq_i32(ctx, a, b);
                a
            };
            // baseline
            eq_i32(&format!("{vname} baseline"), run(&sig, &m, &pk, "baseline"), 0);

            // row 579: non-canonical S with sig[63] & 240 != 0
            for (i, sh) in [L_HEX, "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"]
                .iter()
                .enumerate()
            {
                let mut bad = sig;
                bad[32..].copy_from_slice(&hx(sh));
                let ctx = format!("row579 {vname} non-canonical S[{i}]");
                eq_i32(&format!("{ctx} == -1"), run(&bad, &m, &pk, &ctx), -1);
                assert_ne!(bad[63] & 240, 0, "row579: the sig[63]&240 gate must be open");
            }
            // row 586: ED25519_COMPAT is NOT defined in this build, so the
            // `sig[63] & 224` early return is dead code -- and it is also
            // behaviourally indistinguishable from the live check: 224 (0xe0) is
            // a subset of 240 (0xf0), and any S with a bit of 0xe0 set is
            // >= 2^253 > L, hence never canonical, so
            // `(sig[63] & 240) != 0 && !sc25519_is_canonical(S)` fires for
            // exactly the same inputs. Both bit patterns are exercised and must
            // be rejected identically by C and Rust.
            for bit in [0x20u8, 0x40, 0x80, 0x10] {
                let mut compat = sig;
                compat[63] = (compat[63] & 0x0f) | bit;
                let ctx = format!("row586 {vname} sig[63] |= {bit:#04x}");
                eq_i32(&format!("{ctx} == -1"), run(&compat, &m, &pk, &ctx), -1);
            }

            // row 580: non-canonical pk
            for (i, bad_pk) in ed25519_noncanonical().iter().enumerate() {
                let ctx = format!("row580 {vname} non-canonical pk[{i}]");
                eq_i32(&format!("{ctx} == -1"), run(&sig, &m, bad_pk, &ctx), -1);
            }
            // row 581: pk not a curve point (~50% of random strings)
            let mut nonpoints = 0usize;
            for i in 0..400 {
                let p = rng.bytes(32);
                let ctx = format!("row581 {vname} random pk[{i}]");
                if run(&sig, &m, &p, &ctx) != 0 {
                    nonpoints += 1;
                }
            }
            assert_eq!(nonpoints, 400, "row581: no random pk should verify");
            // row 582: pk of small order
            for (i, p) in ed25519_small_order().iter().enumerate() {
                let ctx = format!("row582 {vname} small-order pk[{i}]");
                eq_i32(&format!("{ctx} == -1"), run(&sig, &m, p, &ctx), -1);
            }
            // row 583: R = sig[0..32] not a curve point
            for i in 0..200 {
                let mut bad = sig;
                bad[..32].copy_from_slice(&rng.bytes(32));
                let ctx = format!("row583 {vname} random R[{i}]");
                eq_i32(&format!("{ctx} == -1"), run(&bad, &m, &pk, &ctx), -1);
            }
            // row 584: R of small order
            for (i, r) in ed25519_small_order().iter().enumerate() {
                let mut bad = sig;
                bad[..32].copy_from_slice(r);
                let ctx = format!("row584 {vname} small-order R[{i}]");
                eq_i32(&format!("{ctx} == -1"), run(&bad, &m, &pk, &ctx), -1);
            }
            // row 585: any single bit flipped in sig, in m, or a mismatched pk
            for pos in 0..64 * 8usize {
                let mut bad = sig;
                bad[pos / 8] ^= 1u8 << (pos % 8);
                let ctx = format!("row585 {vname} sig bit {pos}");
                eq_i32(&format!("{ctx} == -1"), run(&bad, &m, &pk, &ctx), -1);
            }
            for pos in 0..64usize {
                let mut bm = m.clone();
                bm[pos] ^= 0x01;
                let ctx = format!("row585 {vname} m byte {pos}");
                eq_i32(&format!("{ctx} == -1"), run(&sig, &bm, &pk, &ctx), -1);
            }
            let (pk2, _) = ed_keypair(0x7E);
            eq_i32(
                &format!("row585 {vname} mismatched pk == -1"),
                run(&sig, &m, &pk2, "row585 mismatched pk"),
                -1,
            );
            // truncated / extended messages
            for len in [0usize, 1, 32, 63, 65] {
                let mut bm = vec![0x91u8; len];
                if len > 64 {
                    bm[64] = 0;
                }
                let ctx = format!("row585 {vname} mlen={len}");
                eq_i32(&format!("{ctx} == -1"), run(&sig, &bm, &pk, &ctx), -1);
            }
            // fully random signatures
            for i in 0..300 {
                let s = rng.bytes(64);
                let ctx = format!("row585 {vname} random sig[{i}]");
                eq_i32(&format!("{ctx} == -1"), run(&s, &m, &pk, &ctx), -1);
            }
        }

        // row 587: ph <-> non-ph domain-prefix mismatch
        let init = pair::<PhInit>("crypto_sign_ed25519ph_init");
        let upd = pair::<PhUpdate>("crypto_sign_ed25519ph_update");
        let fin = pair::<PhCreate>("crypto_sign_ed25519ph_final_create");
        let ver = pair::<PhVerify>("crypto_sign_ed25519ph_final_verify");
        let vf = pair::<VerifyFn>("crypto_sign_ed25519_verify_detached");
        // ph signature over m
        let mut phsig = [[0u8; 64]; 2];
        let mut phver = [0i32; 2];
        let mut nonphver = [0i32; 2];
        for which in 0..2 {
            let mut st = vec![0xAAu64; 64];
            let sp = st.as_mut_ptr() as *mut u8;
            if which == 0 {
                (init.0)(sp);
                (upd.0)(sp, m.as_ptr(), 64);
                (fin.0)(sp, phsig[which].as_mut_ptr(), ptr::null_mut(), sk.as_ptr());
            } else {
                (init.1)(sp);
                (upd.1)(sp, m.as_ptr(), 64);
                (fin.1)(sp, phsig[which].as_mut_ptr(), ptr::null_mut(), sk.as_ptr());
            }
            // the ph signature must NOT verify with the non-ph verifier
            let s = phsig[which];
            nonphver[which] = if which == 0 {
                (vf.0)(s.as_ptr(), m.as_ptr(), 64, pk.as_ptr())
            } else {
                (vf.1)(s.as_ptr(), m.as_ptr(), 64, pk.as_ptr())
            };
            // and the non-ph signature must NOT verify with final_verify
            let mut st = vec![0xAAu64; 64];
            let sp = st.as_mut_ptr() as *mut u8;
            phver[which] = if which == 0 {
                (init.0)(sp);
                (upd.0)(sp, m.as_ptr(), 64);
                (ver.0)(sp, sig.as_ptr(), pk.as_ptr())
            } else {
                (init.1)(sp);
                (upd.1)(sp, m.as_ptr(), 64);
                (ver.1)(sp, sig.as_ptr(), pk.as_ptr())
            };
        }
        eq_bytes("row587 ph signature", &phsig[0], &phsig[1]);
        eq_i32("row587 ph sig via non-ph verify", nonphver[0], nonphver[1]);
        eq_i32("row587 ph sig via non-ph verify == -1", nonphver[0], -1);
        eq_i32("row587 non-ph sig via final_verify", phver[0], phver[1]);
        eq_i32("row587 non-ph sig via final_verify == -1", phver[0], -1);
    }
}

// ===========================================================================
// ERRORS rows 588-591, 596-599 — sign_open, and the no-rejection paths
// ===========================================================================
#[test]
fn g5e_sign_open_and_no_reject_paths() {
    unsafe {
        let (pk, sk) = ed_keypair(0x8F);
        let mut rng = Rng::new(SEED ^ 0x88);
        let m = vec![0x33u8; 100];
        let sgn = pair::<SignFn>("crypto_sign_ed25519");
        let mut sm = vec![0u8; 164];
        let mut smlen = 0u64;
        assert_eq!(
            (sgn.0)(sm.as_mut_ptr(), &mut smlen, m.as_ptr(), 100, sk.as_ptr()),
            0
        );
        assert_eq!(smlen, 164);

        for onm in ["crypto_sign_ed25519_open", "crypto_sign_open"] {
            let op = pair::<SignFn>(onm);
            // row 588: smlen in 0..63 -> -1, *mlen_p = 0, m untouched
            for smlen in 0u64..=64 {
                let mut cm = vec![0xAAu8; 128];
                let mut rm = vec![0xAAu8; 128];
                let mut cl = u64::MAX;
                let mut rl = u64::MAX;
                let a = (op.0)(cm.as_mut_ptr(), &mut cl, sm.as_ptr(), smlen, pk.as_ptr());
                let b = (op.1)(rm.as_mut_ptr(), &mut rl, sm.as_ptr(), smlen, pk.as_ptr());
                let ctx = format!("row588 {onm} smlen={smlen}");
                eq_i32(&ctx, a, b);
                eq_bytes(&ctx, &cm, &rm);
                assert_eq!(cl, rl, "{ctx}: *mlen_p differs");
                eq_i32(&format!("{ctx} == -1"), a, -1);
                assert_eq!(cl, 0, "{ctx}: *mlen_p must be 0");
                if smlen < 64 {
                    assert_eq!(cm, vec![0xAAu8; 128], "{ctx}: m must be untouched");
                }
            }
            // row 590: verify failure -> memset(m, 0, smlen-64), *mlen_p = 0
            for pos in [0usize, 31, 32, 63, 64, 100, 163] {
                let mut bad = sm.clone();
                bad[pos] ^= 0x01;
                let mut cm = vec![0xAAu8; 128];
                let mut rm = vec![0xAAu8; 128];
                let mut cl = u64::MAX;
                let mut rl = u64::MAX;
                let a = (op.0)(cm.as_mut_ptr(), &mut cl, bad.as_ptr(), 164, pk.as_ptr());
                let b = (op.1)(rm.as_mut_ptr(), &mut rl, bad.as_ptr(), 164, pk.as_ptr());
                let ctx = format!("row590 {onm} flip@{pos}");
                eq_i32(&ctx, a, b);
                eq_bytes(&ctx, &cm, &rm);
                assert_eq!(cl, rl);
                eq_i32(&format!("{ctx} == -1"), a, -1);
                assert_eq!(cl, 0, "{ctx}: *mlen_p must be 0");
                assert_eq!(&cm[..100], &vec![0u8; 100][..], "{ctx}: m must be zeroed");
                assert_eq!(&cm[100..], &vec![0xAAu8; 28][..], "{ctx}: only mlen zeroed");
            }
            // row 590 with m == NULL and mlen_p == NULL (both optional)
            let mut bad = sm.clone();
            bad[0] ^= 1;
            let a = (op.0)(
                ptr::null_mut(),
                ptr::null_mut(),
                bad.as_ptr(),
                164,
                pk.as_ptr(),
            );
            let b = (op.1)(
                ptr::null_mut(),
                ptr::null_mut(),
                bad.as_ptr(),
                164,
                pk.as_ptr(),
            );
            eq_i32(&format!("row590 {onm} m/mlen_p NULL"), a, b);
            eq_i32(&format!("row590 {onm} m/mlen_p NULL == -1"), a, -1);
            // random garbage
            for i in 0..200 {
                let g = rng.bytes(164);
                let mut cm = vec![0xAAu8; 128];
                let mut rm = vec![0xAAu8; 128];
                let mut cl = u64::MAX;
                let mut rl = u64::MAX;
                let a = (op.0)(cm.as_mut_ptr(), &mut cl, g.as_ptr(), 164, pk.as_ptr());
                let b = (op.1)(rm.as_mut_ptr(), &mut rl, g.as_ptr(), 164, pk.as_ptr());
                let ctx = format!("row590 {onm} garbage[{i}]");
                eq_i32(&ctx, a, b);
                eq_bytes(&ctx, &cm, &rm);
                assert_eq!(cl, rl);
            }
            // row 589 note: `smlen - 64 > SODIUM_SIZE_MAX - 64` requires
            // smlen > SIZE_MAX, which is unrepresentable in an
            // `unsigned long long` on this 64-bit target, so the branch is
            // unreachable. smlen = u64::MAX exercises the closest boundary
            // (and would read out of bounds, so only the length check must fire
            // -- which it does not; hence this is intentionally NOT called).
            let _ = u64::MAX;
        }

        // rows 597/598: sign / seed_keypair have no rejection path
        let sd = pair::<SignFn>("crypto_sign_ed25519_detached");
        let kp = pair::<SeedKeypair>("crypto_sign_ed25519_seed_keypair");
        let mut sks: Vec<Vec<u8>> = vec![vec![0u8; 64], vec![0xffu8; 64], sk.clone()];
        for _ in 0..60 {
            sks.push(rng.bytes(64));
        }
        for (i, s) in sks.iter().enumerate() {
            let mut csig = [0xAAu8; 64];
            let mut rsig = [0xAAu8; 64];
            let mut cl = u64::MAX;
            let mut rl = u64::MAX;
            let a = (sd.0)(csig.as_mut_ptr(), &mut cl, m.as_ptr(), 100, s.as_ptr());
            let b = (sd.1)(rsig.as_mut_ptr(), &mut rl, m.as_ptr(), 100, s.as_ptr());
            let ctx = format!("row597 detached arbitrary sk[{i}]");
            eq_i32(&ctx, a, b);
            eq_bytes(&ctx, &csig, &rsig);
            assert_eq!(cl, rl);
            eq_i32(&format!("{ctx} == 0"), a, 0);
            assert_eq!(cl, 64, "{ctx}: *siglen_p must be 64");
            // row 591: the combined sign therefore cannot fail either
            let mut csm = vec![0xAAu8; 164];
            let mut rsm = vec![0xAAu8; 164];
            let a = (sgn.0)(csm.as_mut_ptr(), ptr::null_mut(), m.as_ptr(), 100, s.as_ptr());
            let b = (sgn.1)(rsm.as_mut_ptr(), ptr::null_mut(), m.as_ptr(), 100, s.as_ptr());
            eq_i32(&format!("row591 crypto_sign_ed25519[{i}]"), a, b);
            eq_bytes(&format!("row591 crypto_sign_ed25519[{i}]"), &csm, &rsm);
            eq_i32(&format!("row591[{i}] == 0"), a, 0);
        }
        for (i, seed) in [vec![0u8; 32], vec![0xffu8; 32]].iter().enumerate() {
            let mut cpk = [0xAAu8; 32];
            let mut csk = [0xAAu8; 64];
            let mut rpk = [0xAAu8; 32];
            let mut rsk = [0xAAu8; 64];
            let a = (kp.0)(cpk.as_mut_ptr(), csk.as_mut_ptr(), seed.as_ptr());
            let b = (kp.1)(rpk.as_mut_ptr(), rsk.as_mut_ptr(), seed.as_ptr());
            eq_i32(&format!("row598 seed_keypair[{i}]"), a, b);
            eq_bytes(&format!("row598 pk[{i}]"), &cpk, &rpk);
            eq_bytes(&format!("row598 sk[{i}]"), &csk, &rsk);
            eq_i32(&format!("row598[{i}] == 0"), a, 0);
        }

        // row 599: ph_init / ph_update never fail (incl. mlen = 0, m = NULL)
        let init = pair::<PhInit>("crypto_sign_ed25519ph_init");
        let upd = pair::<PhUpdate>("crypto_sign_ed25519ph_update");
        let fin = pair::<PhCreate>("crypto_sign_ed25519ph_final_create");
        let mut rets: Vec<Vec<i32>> = Vec::new();
        let mut sigs: Vec<Vec<u8>> = Vec::new();
        for which in 0..2 {
            let mut st = vec![0xAAu64; 64];
            let sp = st.as_mut_ptr() as *mut u8;
            let mut rs = Vec::new();
            rs.push(if which == 0 { (init.0)(sp) } else { (init.1)(sp) });
            // m == NULL with mlen == 0 is accepted (the pointer is not read)
            rs.push(if which == 0 {
                (upd.0)(sp, ptr::null(), 0)
            } else {
                (upd.1)(sp, ptr::null(), 0)
            });
            rs.push(if which == 0 {
                (upd.0)(sp, m.as_ptr(), 0)
            } else {
                (upd.1)(sp, m.as_ptr(), 0)
            });
            rs.push(if which == 0 {
                (upd.0)(sp, m.as_ptr(), 100)
            } else {
                (upd.1)(sp, m.as_ptr(), 100)
            });
            let mut sig = [0xAAu8; 64];
            rs.push(if which == 0 {
                (fin.0)(sp, sig.as_mut_ptr(), ptr::null_mut(), sk.as_ptr())
            } else {
                (fin.1)(sp, sig.as_mut_ptr(), ptr::null_mut(), sk.as_ptr())
            });
            rets.push(rs);
            sigs.push(sig.to_vec());
        }
        assert_eq!(rets[0], rets[1], "row599: ph return codes differ");
        assert!(rets[0].iter().all(|&r| r == 0), "row599: all must be 0");
        eq_bytes("row599 ph signature", &sigs[0], &sigs[1]);
    }
}

// ===========================================================================
// ERRORS rows 592-596 — crypto_sign_ed25519_pk_to_curve25519 / sk_to_*
// ===========================================================================
#[test]
fn g5e_sign_key_conversion_errors() {
    unsafe {
        let p2c = pair::<I2>("crypto_sign_ed25519_pk_to_curve25519");
        let s2c = pair::<I2>("crypto_sign_ed25519_sk_to_curve25519");
        let s2s = pair::<I2>("crypto_sign_ed25519_sk_to_seed");
        let s2p = pair::<I2>("crypto_sign_ed25519_sk_to_pk");
        let add = pair::<I3>("crypto_core_ed25519_add");
        let (pk, _) = ed_keypair(0x9A);
        let mut rng = Rng::new(SEED ^ 0x99);

        // row 592: pk not a curve point
        let mut nonpoints = 0usize;
        for i in 0..500 {
            let p = rng.bytes(32);
            let (ret, q) = run_i2(&p2c, 32, &p, &format!("row592 pk_to_curve rand[{i}]"));
            if ret != 0 {
                nonpoints += 1;
                assert_eq!(q, vec![0xAAu8; 32], "row592: output untouched on failure");
            }
        }
        assert!(nonpoints > 100, "row592: expected many non-points");

        // row 593: pk in the torsion set
        for (i, p) in ed25519_small_order().iter().enumerate() {
            let (ret, q) = run_i2(&p2c, 32, p, &format!("row593 pk_to_curve torsion[{i}]"));
            eq_i32(&format!("row593 torsion[{i}] == -1"), ret, -1);
            assert_eq!(q, vec![0xAAu8; 32]);
        }

        // row 594: composite-order pk
        let mut composite = 0usize;
        for (i, t) in ed25519_small_order().iter().enumerate() {
            let (ra, r) = run_i3(&add, 32, &pk, t, &format!("row594 add(pk,t[{i}])"));
            if ra != 0 || r == pk {
                continue;
            }
            let (ret, _) = run_i2(&p2c, 32, &r, &format!("row594 pk_to_curve composite[{i}]"));
            if ret != 0 {
                composite += 1;
            }
        }
        assert!(composite >= 2, "row594: expected composite-order rejections");

        // row 595: NO ge25519_is_canonical check here, unlike verify_detached.
        // The `edff..7f`-style encodings are therefore ACCEPTED when they decode.
        // We only assert C/Rust agreement, and additionally show that
        // verify_detached DOES reject them (documented divergence).
        let vf = pair::<VerifyFn>("crypto_sign_ed25519_verify_detached");
        let dummy_sig = [0u8; 64];
        let msg = [0u8; 4];
        for (i, p) in ed25519_noncanonical().iter().enumerate() {
            let (ret, _) = run_i2(&p2c, 32, p, &format!("row595 pk_to_curve noncanon[{i}]"));
            let a = (vf.0)(dummy_sig.as_ptr(), msg.as_ptr(), 4, p.as_ptr());
            let b = (vf.1)(dummy_sig.as_ptr(), msg.as_ptr(), 4, p.as_ptr());
            eq_i32(&format!("row595 verify_detached noncanon[{i}]"), a, b);
            eq_i32(
                &format!("row595 verify_detached rejects noncanon[{i}]"),
                a,
                -1,
            );
            // ret is whatever the C decides -- recorded, not constrained
            let _ = ret;
        }

        // row 596: sk_to_seed / sk_to_pk / sk_to_curve25519 never fail
        let mut sks: Vec<Vec<u8>> = vec![vec![0u8; 64], vec![0xffu8; 64]];
        for _ in 0..300 {
            sks.push(rng.bytes(64));
        }
        for (i, s) in sks.iter().enumerate() {
            for (nm, f) in [
                ("sk_to_seed", &s2s),
                ("sk_to_pk", &s2p),
                ("sk_to_curve25519", &s2c),
            ] {
                let (ret, _) = run_i2(f, 32, s, &format!("row596 {nm}[{i}]"));
                eq_i32(&format!("row596 {nm}[{i}] always 0"), ret, 0);
            }
        }
    }
}

// ===========================================================================
// ERRORS rows 600-611 — crypto_core_ed25519 is_valid_point / add / sub
// ===========================================================================
#[test]
fn g5e_core_ed25519_point_rejections() {
    install_det_random();
    unsafe {
        let ivp = pair::<Chk>("crypto_core_ed25519_is_valid_point");
        let add = pair::<I3>("crypto_core_ed25519_add");
        let sub = pair::<I3>("crypto_core_ed25519_sub");
        let rnd = pair::<V1>("crypto_core_ed25519_random");
        let mut rng = Rng::new(SEED ^ 0xAA);

        // a genuine valid point
        let mut good = vec![0xAAu8; 32];
        let mut good_r = vec![0xAAu8; 32];
        det_reseed(0x1234_5678);
        (rnd.0)(good.as_mut_ptr());
        det_reseed(0x1234_5678);
        (rnd.1)(good_r.as_mut_ptr());
        eq_bytes("core_ed25519_random", &good, &good_r);
        assert_eq!(run_chk(&ivp, &good, "baseline is_valid_point"), 1);

        // row 600: non-canonical encodings
        for (i, p) in ed25519_noncanonical().iter().enumerate() {
            eq_i32(
                &format!("row600 is_valid_point noncanon[{i}] == 0"),
                run_chk(&ivp, p, &format!("row600 noncanon[{i}]")),
                0,
            );
        }
        // rows 601/602: random strings (about half are not valid y coordinates)
        let mut invalid = 0usize;
        for i in 0..800 {
            let p = rng.bytes(32);
            if run_chk(&ivp, &p, &format!("row601 is_valid_point rand[{i}]")) == 0 {
                invalid += 1;
            }
        }
        assert!(invalid > 400, "row601: expected many invalid encodings");
        // row 603: the torsion set (including identity and all-zero)
        for (i, p) in ed25519_small_order().iter().enumerate() {
            eq_i32(
                &format!("row603 is_valid_point torsion[{i}] == 0"),
                run_chk(&ivp, p, &format!("row603 torsion[{i}]")),
                0,
            );
        }
        // row 604: valid curve point of order 2L/4L/8L
        let mut composite = 0usize;
        for (i, t) in ed25519_small_order().iter().enumerate() {
            let (ra, r) = run_i3(&add, 32, &good, t, &format!("row609 add(p,t[{i}])"));
            if ra != 0 || r == good {
                continue;
            }
            if run_chk(&ivp, &r, &format!("row604 composite[{i}]")) == 0 {
                composite += 1;
            }
        }
        assert!(composite >= 2, "row604: expected composite-order points");

        // rows 605-608, 610-611: add/sub argument rejections. `p` is checked
        // first, so a bad `p` with a good `q` and vice versa both matter.
        let bad_points: Vec<Vec<u8>> = {
            let mut v: Vec<Vec<u8>> = Vec::new();
            // encodings that do not decode at all
            for _ in 0..40 {
                v.push(rng.bytes(32));
            }
            v.push(vec![0x02u8; 32]);
            v
        };
        let mut rejected = 0usize;
        for (i, bad) in bad_points.iter().enumerate() {
            for (nm, f) in [("add", &add), ("sub", &sub)] {
                let (r1, o1) = run_i3(f, 32, bad, &good, &format!("row605 {nm} bad p[{i}]"));
                let (r2, o2) = run_i3(f, 32, &good, bad, &format!("row607 {nm} bad q[{i}]"));
                if r1 != 0 {
                    rejected += 1;
                    assert_eq!(o1, vec![0xAAu8; 32], "row605 {nm}: r untouched");
                }
                if r2 != 0 {
                    assert_eq!(o2, vec![0xAAu8; 32], "row607 {nm}: r untouched");
                }
            }
        }
        assert!(rejected > 10, "row605: expected undecodable points");

        // row 609: non-canonical and small-order points ARE accepted by add/sub
        // (no is_canonical / small-order / subgroup check). Documented
        // divergence risk -- only C/Rust agreement is asserted.
        for (i, p) in ed25519_noncanonical()
            .iter()
            .chain(ed25519_small_order().iter())
            .enumerate()
        {
            run_i3(&add, 32, &good, p, &format!("row609 add(good, weird[{i}])"));
            run_i3(&add, 32, p, &good, &format!("row609 add(weird[{i}], good)"));
            run_i3(&sub, 32, &good, p, &format!("row610 sub(good, weird[{i}])"));
            run_i3(&sub, 32, p, &good, &format!("row611 sub(weird[{i}], good)"));
        }
        // the identity IS accepted (add) even though is_valid_point rejects it
        let (ret, r) = run_i3(&add, 32, &good, &hx(ONE_HEX), "row609 add(p, identity)");
        eq_i32("row609 add(p, identity) == 0", ret, 0);
        eq_bytes("row609 add(p, identity) == p", &good, &r);
        let (ret, _) = run_i3(&add, 32, &good, &hx(ZERO_HEX), "row609 add(p, all-zero)");
        eq_i32("row609 add(p, 32x00) accepted", ret, 0);
    }
}

// ===========================================================================
// ERRORS rows 612, 613, 619, 620 — ed25519 scalar helpers
// ===========================================================================
#[test]
fn g5e_core_ed25519_scalar_errors() {
    unsafe {
        let inv = pair::<I2>("crypto_core_ed25519_scalar_invert");
        let can = pair::<Chk>("crypto_core_ed25519_scalar_is_canonical");
        let neg = pair::<V2>("crypto_core_ed25519_scalar_negate");
        let cpl = pair::<V2>("crypto_core_ed25519_scalar_complement");
        let add = pair::<V3>("crypto_core_ed25519_scalar_add");
        let sub = pair::<V3>("crypto_core_ed25519_scalar_sub");
        let mul = pair::<V3>("crypto_core_ed25519_scalar_mul");
        let red = pair::<V2>("crypto_core_ed25519_scalar_reduce");
        let mut rng = Rng::new(SEED ^ 0xBB);

        // row 612: s = 32 x 0x00 -> -1, but recip IS still written
        let (ret, recip) = run_i2(&inv, 32, &vec![0u8; 32], "row612 invert(0)");
        eq_i32("row612 invert(0) == -1", ret, -1);
        assert_ne!(recip, vec![0xAAu8; 32], "row612: recip is still written");

        // row 613: s = L (and other nonzero s = 0 mod L) is NOT rejected --
        // only the literal byte-zero test is performed. Documented divergence.
        for (i, sh) in [
            L_HEX,
            "eed3f55c1a631258d69cf7a2def9de1400000000000000000000000000000010",
        ]
        .iter()
        .enumerate()
        {
            let s = hx(sh);
            let (ret, _) = run_i2(&inv, 32, &s, &format!("row613 invert(L+{i})"));
            if i == 0 {
                eq_i32("row613 invert(L) returns 0 (meaningless recip)", ret, 0);
            }
        }
        // 2L (still nonzero bytes) likewise returns 0
        let l = hx(L_HEX);
        let two_l = {
            let mut acc = [0u8; 32];
            let mut carry = 0u16;
            for i in 0..32 {
                let v = (l[i] as u16) * 2 + carry;
                acc[i] = (v & 0xff) as u8;
                carry = v >> 8;
            }
            acc.to_vec()
        };
        let (ret, _) = run_i2(&inv, 32, &two_l, "row613 invert(2L)");
        eq_i32("row613 invert(2L) returns 0", ret, 0);

        // row 620: scalar_is_canonical == 0 for s >= L
        for (i, sh) in [
            L_HEX,
            "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
            "0000000000000000000000000000000000000000000000000000000000000080",
            "eed3f55c1a631258d69cf7a2def9de1400000000000000000000000000000010",
        ]
        .iter()
        .enumerate()
        {
            let s = hx(sh);
            eq_i32(
                &format!("row620 is_canonical[{i}] == 0"),
                run_chk(&can, &s, &format!("row620[{i}]")),
                0,
            );
        }
        // one step below L is canonical
        eq_i32(
            "row620 L-1 is canonical",
            run_chk(&can, &hx(LM1_HEX), "row620 L-1"),
            1,
        );

        // row 619: the void-returning helpers have no rejection path at all --
        // every 32/64-byte input, including 64 x 0xff, is accepted.
        let inputs32: Vec<Vec<u8>> = vec![
            vec![0u8; 32],
            vec![0xffu8; 32],
            hx(L_HEX),
            hx(LM1_HEX),
            hx(ONE_HEX),
            hx("0000000000000000000000000000000000000000000000000000000000000080"),
        ];
        for (i, x) in inputs32.iter().enumerate() {
            run_v2(&neg, 32, x, &format!("row619 negate[{i}]"));
            run_v2(&cpl, 32, x, &format!("row619 complement[{i}]"));
            for (j, y) in inputs32.iter().enumerate() {
                run_v3(&add, 32, x, y, &format!("row619 add[{i}/{j}]"));
                run_v3(&sub, 32, x, y, &format!("row619 sub[{i}/{j}]"));
                run_v3(&mul, 32, x, y, &format!("row619 mul[{i}/{j}]"));
            }
        }
        for (i, s) in [vec![0u8; 64], vec![0xffu8; 64]].iter().enumerate() {
            run_v2(&red, 32, s, &format!("row619 reduce[{i}]"));
        }
        for i in 0..300 {
            let x = rng.bytes(32);
            let y = rng.bytes(32);
            run_v2(&neg, 32, &x, &format!("row619 negate rand[{i}]"));
            run_v2(&cpl, 32, &x, &format!("row619 complement rand[{i}]"));
            run_v3(&add, 32, &x, &y, &format!("row619 add rand[{i}]"));
            run_v3(&sub, 32, &x, &y, &format!("row619 sub rand[{i}]"));
            run_v3(&mul, 32, &x, &y, &format!("row619 mul rand[{i}]"));
            run_v2(&red, 32, &rng.bytes(64), &format!("row619 reduce rand[{i}]"));
            run_chk(&can, &x, &format!("row620 is_canonical rand[{i}]"));
            run_i2(&inv, 32, &x, &format!("row612 invert rand[{i}]"));
        }
    }
}

// ===========================================================================
// ERRORS rows 614, 615, 632, 633 — out-of-range `hash_alg`
// ===========================================================================
#[test]
fn g5e_from_string_bad_hash_alg() {
    unsafe {
        let names = [
            "crypto_core_ed25519_from_string",
            "crypto_core_ed25519_from_string_nu",
            "crypto_core_ed25519_scalar_from_string",
            "crypto_core_ristretto255_from_string",
            "crypto_core_ristretto255_scalar_from_string",
        ];
        let ctxs: Vec<Option<Vec<u8>>> = vec![
            Some(b"QUUX-V01-CS02".to_vec()),
            Some(vec![b'X'; 300]),
            None,
            Some(Vec::new()),
        ];
        let msgs: Vec<Vec<u8>> = vec![Vec::new(), b"abc".to_vec(), vec![0x5Au8; 500]];

        for name in names {
            let f = pair::<FromStr>(name);
            for &alg in BAD_ALGS.iter() {
                for (ci, cv) in ctxs.iter().enumerate() {
                    for (mi, mv) in msgs.iter().enumerate() {
                        let (cp, cl) = match cv {
                            Some(s) => (s.as_ptr(), s.len()),
                            None => (ptr::null(), 0usize),
                        };
                        let mut co = [0xAAu8; 32];
                        let mut ro = [0xAAu8; 32];
                        set_errno(0);
                        let a = (f.0)(co.as_mut_ptr(), cp, cl, mv.as_ptr(), mv.len(), alg);
                        let ae = errno();
                        set_errno(0);
                        let b = (f.1)(ro.as_mut_ptr(), cp, cl, mv.as_ptr(), mv.len(), alg);
                        let be = errno();
                        let ctx = format!("row614 {name} alg={alg} ctx#{ci} msg#{mi}");
                        eq_i32(&ctx, a, b);
                        eq_bytes(&ctx, &co, &ro);
                        eq_i32(&format!("{ctx} == -1"), a, -1);
                        assert_eq!(ae, be, "{ctx}: errno differs (C={ae}, Rust={be})");
                        assert_eq!(ae, EINVAL, "{ctx}: errno must be EINVAL");
                        assert_eq!(co, [0xAAu8; 32], "{ctx}: output must be untouched");
                    }
                }
            }
            // one step inside the range: 1 and 2 succeed
            for alg in [1i32, 2] {
                let c = b"ctx".to_vec();
                let m = b"msg".to_vec();
                let mut co = [0xAAu8; 32];
                let mut ro = [0xAAu8; 32];
                set_errno(0);
                let a = (f.0)(co.as_mut_ptr(), c.as_ptr(), 3, m.as_ptr(), 3, alg);
                let b = (f.1)(ro.as_mut_ptr(), c.as_ptr(), 3, m.as_ptr(), 3, alg);
                let ctx = format!("{name} alg={alg}");
                eq_i32(&ctx, a, b);
                eq_bytes(&ctx, &co, &ro);
                eq_i32(&format!("{ctx} == 0"), a, 0);
            }
        }
    }
}

// ===========================================================================
// ERRORS rows 621-630 — crypto_core_ristretto255 point rejections
// ===========================================================================
#[test]
fn g5e_core_ristretto255_point_rejections() {
    unsafe {
        let ivp = pair::<Chk>("crypto_core_ristretto255_is_valid_point");
        let add = pair::<I3>("crypto_core_ristretto255_add");
        let sub = pair::<I3>("crypto_core_ristretto255_sub");
        let good = hx(R_BASE_HEX);
        assert_eq!(run_chk(&ivp, &good, "baseline"), 1);
        // the identity IS a valid encoding
        assert_eq!(run_chk(&ivp, &hx(ZERO_HEX), "identity"), 1);

        // rows 621-625
        for (i, (kind, p)) in ristretto_bad().iter().enumerate() {
            eq_i32(
                &format!("row621 is_valid_point {kind}[{i}] == 0"),
                run_chk(&ivp, p, &format!("row621 {kind}[{i}]")),
                0,
            );
            // rows 627-630: add/sub reject the same encodings, r untouched,
            // and `p` is checked before `q`.
            let (r1, o1) = run_i3(&add, 32, p, &good, &format!("row627 add bad p {kind}[{i}]"));
            eq_i32(&format!("row627 add bad p {kind}[{i}] == -1"), r1, -1);
            assert_eq!(o1, vec![0xAAu8; 32], "row627: r untouched");
            let (r2, o2) = run_i3(&add, 32, &good, p, &format!("row628 add bad q {kind}[{i}]"));
            eq_i32(&format!("row628 add bad q {kind}[{i}] == -1"), r2, -1);
            assert_eq!(o2, vec![0xAAu8; 32], "row628: r untouched");
            let (r3, o3) = run_i3(&sub, 32, p, &good, &format!("row629 sub bad p {kind}[{i}]"));
            eq_i32(&format!("row629 sub bad p {kind}[{i}] == -1"), r3, -1);
            assert_eq!(o3, vec![0xAAu8; 32], "row629: r untouched");
            let (r4, o4) = run_i3(&sub, 32, &good, p, &format!("row630 sub bad q {kind}[{i}]"));
            eq_i32(&format!("row630 sub bad q {kind}[{i}] == -1"), r4, -1);
            assert_eq!(o4, vec![0xAAu8; 32], "row630: r untouched");
            // both bad
            run_i3(&add, 32, p, p, &format!("row627 add both bad {kind}[{i}]"));
            run_i3(&sub, 32, p, p, &format!("row629 sub both bad {kind}[{i}]"));
        }

        // row 626: fe25519_iszero(h->Y) -- brute-force search over random and
        // structured encodings; whichever class fires, C and Rust must agree.
        let mut rng = Rng::new(SEED ^ 0xCC);
        let mut invalid = 0usize;
        for i in 0..1200 {
            let mut p = rng.bytes(32);
            p[31] &= 0x7f; // keep the canonical-ish shape half the time
            if i % 2 == 0 {
                p[0] &= 0xfe;
            }
            if run_chk(&ivp, &p, &format!("row626 is_valid_point rand[{i}]")) == 0 {
                invalid += 1;
            }
            run_i3(&add, 32, &p, &good, &format!("row627 add rand[{i}]"));
            run_i3(&sub, 32, &good, &p, &format!("row630 sub rand[{i}]"));
        }
        assert!(invalid > 300, "row626: expected many invalid encodings");
        // structured y = 0 style candidates
        for (i, p) in [
            hx("0000000000000000000000000000000000000000000000000000000000000000"),
            hx("0200000000000000000000000000000000000000000000000000000000000000"),
            hx("ecffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f"),
        ]
        .iter()
        .enumerate()
        {
            run_chk(&ivp, p, &format!("row626 structured[{i}]"));
            run_i3(&add, 32, p, &good, &format!("row627 structured[{i}]"));
        }
    }
}

// ===========================================================================
// ERRORS rows 631, 634-637 — ristretto255 from_hash / scalar helpers
// ===========================================================================
#[test]
fn g5e_core_ristretto255_scalar_errors() {
    unsafe {
        let fh = pair::<I2>("crypto_core_ristretto255_from_hash");
        let inv = pair::<I2>("crypto_core_ristretto255_scalar_invert");
        let can = pair::<Chk>("crypto_core_ristretto255_scalar_is_canonical");
        let ivp = pair::<Chk>("crypto_core_ristretto255_is_valid_point");
        let neg = pair::<V2>("crypto_core_ristretto255_scalar_negate");
        let cpl = pair::<V2>("crypto_core_ristretto255_scalar_complement");
        let add = pair::<V3>("crypto_core_ristretto255_scalar_add");
        let sub = pair::<V3>("crypto_core_ristretto255_scalar_sub");
        let mul = pair::<V3>("crypto_core_ristretto255_scalar_mul");
        let red = pair::<V2>("crypto_core_ristretto255_scalar_reduce");
        let rnd = pair::<V1>("crypto_core_ristretto255_random");
        let srnd = pair::<V1>("crypto_core_ristretto255_scalar_random");
        let mut rng = Rng::new(SEED ^ 0xDD);

        // row 631: from_hash has no rejection path
        for (i, r) in [vec![0u8; 64], vec![0xffu8; 64]].iter().enumerate() {
            let (ret, p) = run_i2(&fh, 32, r, &format!("row631 from_hash edge[{i}]"));
            eq_i32(&format!("row631 from_hash edge[{i}] == 0"), ret, 0);
            eq_i32(
                &format!("row631 result is valid[{i}]"),
                run_chk(&ivp, &p, "row631 valid"),
                1,
            );
        }
        for i in 0..600 {
            let r = rng.bytes(64);
            let (ret, _) = run_i2(&fh, 32, &r, &format!("row631 from_hash rand[{i}]"));
            eq_i32(&format!("row631 from_hash rand[{i}] == 0"), ret, 0);
        }

        // row 634: s = 32 x 0x00 -> -1 (delegates to the ed25519 form)
        let (ret, recip) = run_i2(&inv, 32, &vec![0u8; 32], "row634 invert(0)");
        eq_i32("row634 invert(0) == -1", ret, -1);
        assert_ne!(recip, vec![0xAAu8; 32], "row634: recip still written");
        let einv = pair::<I2>("crypto_core_ed25519_scalar_invert");
        let (ret2, recip2) = run_i2(&einv, 32, &vec![0u8; 32], "row634 ed invert(0)");
        eq_i32("row634 matches ed25519", ret, ret2);
        eq_bytes("row634 recip matches ed25519", &recip2, &recip);

        // row 635: s = L is NOT rejected (documented divergence)
        let (ret, _) = run_i2(&inv, 32, &hx(L_HEX), "row635 invert(L)");
        eq_i32("row635 invert(L) returns 0", ret, 0);

        // row 636: scalar_is_canonical == 0 for s >= L
        for (i, sh) in [
            L_HEX,
            "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
            "0000000000000000000000000000000000000000000000000000000000000080",
        ]
        .iter()
        .enumerate()
        {
            eq_i32(
                &format!("row636 is_canonical[{i}] == 0"),
                run_chk(&can, &hx(sh), &format!("row636[{i}]")),
                0,
            );
        }
        eq_i32(
            "row636 L-1 canonical",
            run_chk(&can, &hx(LM1_HEX), "row636 L-1"),
            1,
        );
        eq_i32(
            "row636 zero canonical",
            run_chk(&can, &hx(ZERO_HEX), "row636 zero"),
            1,
        );

        // row 637: the void-returning helpers accept everything
        let inputs: Vec<Vec<u8>> = vec![
            vec![0u8; 32],
            vec![0xffu8; 32],
            hx(L_HEX),
            hx(ONE_HEX),
            hx("0000000000000000000000000000000000000000000000000000000000000080"),
        ];
        for (i, x) in inputs.iter().enumerate() {
            run_v2(&neg, 32, x, &format!("row637 negate[{i}]"));
            run_v2(&cpl, 32, x, &format!("row637 complement[{i}]"));
            for (j, y) in inputs.iter().enumerate() {
                run_v3(&add, 32, x, y, &format!("row637 add[{i}/{j}]"));
                run_v3(&sub, 32, x, y, &format!("row637 sub[{i}/{j}]"));
                run_v3(&mul, 32, x, y, &format!("row637 mul[{i}/{j}]"));
            }
        }
        for (i, s) in [vec![0u8; 64], vec![0xffu8; 64]].iter().enumerate() {
            run_v2(&red, 32, s, &format!("row637 reduce[{i}]"));
        }
        for i in 0..300 {
            let x = rng.bytes(32);
            let y = rng.bytes(32);
            run_v2(&neg, 32, &x, &format!("row637 negate rand[{i}]"));
            run_v2(&cpl, 32, &x, &format!("row637 complement rand[{i}]"));
            run_v3(&add, 32, &x, &y, &format!("row637 add rand[{i}]"));
            run_v3(&sub, 32, &x, &y, &format!("row637 sub rand[{i}]"));
            run_v3(&mul, 32, &x, &y, &format!("row637 mul rand[{i}]"));
            run_v2(&red, 32, &rng.bytes(64), &format!("row637 reduce rand[{i}]"));
        }
        // row 637: the random generators are void as well
        install_det_random();
        for i in 0..16u64 {
            let mut co = [0xAAu8; 32];
            let mut ro = [0xAAu8; 32];
            det_reseed(0xDD00 + i);
            (rnd.0)(co.as_mut_ptr());
            det_reseed(0xDD00 + i);
            (rnd.1)(ro.as_mut_ptr());
            eq_bytes(&format!("row637 ristretto255_random[{i}]"), &co, &ro);
            let mut co = [0xAAu8; 32];
            let mut ro = [0xAAu8; 32];
            det_reseed(0xDE00 + i);
            (srnd.0)(co.as_mut_ptr());
            det_reseed(0xDE00 + i);
            (srnd.1)(ro.as_mut_ptr());
            eq_bytes(&format!("row637 scalar_random[{i}]"), &co, &ro);
        }
    }
}

// ===========================================================================
// ERRORS rows 616-618, 638 — absent / unreachable entry points
// ===========================================================================
#[test]
fn g5e_absent_and_unreachable() {
    let l = libs();
    let _ = l;
    // row 638: from_uniform / from_hash were removed from crypto_core_ed25519
    for n in [
        "crypto_core_ed25519_from_uniform",
        "crypto_core_ed25519_from_hash",
    ] {
        assert!(
            !has_sym(n),
            "row638: {n} must NOT exist in libsodium 1.0.23"
        );
    }
    // the replacement API and the ristretto255 map DO exist
    for n in [
        "crypto_core_ed25519_from_string",
        "crypto_core_ed25519_from_string_nu",
        "crypto_core_ristretto255_from_hash",
    ] {
        assert!(has_sym(n), "row638: {n} must exist");
    }
    // rows 616, 617, 618 are `abort()` / `assert()` sites that the C marks
    // LCOV_EXCL and that are mathematically unreachable from the public API:
    //   616 `_string_to_points(n > 2)`  -- only ever called with n = 1 or 2
    //   617 `assert(h_len <= 0xff)`     -- callers pass 48 or 96
    //   618 `ge25519_xmont_to_ymont`    -- never fails for a valid u
    // They have no reachable public entry point, so there is nothing to compare.
    // We at least pin the two output sizes that feed row 617.
    unsafe {
        assert_eq!(getter("crypto_core_ed25519_hashbytes"), 64);
        assert_eq!(getter("crypto_core_ed25519_nonreducedscalarbytes"), 64);
        assert_eq!(getter("crypto_core_ed25519_uniformbytes"), 32);
    }
}

// ===========================================================================
// Generic FFI boundary probes: nullable pointers, zero lengths, and one step
// past every documented range.
// ===========================================================================
#[test]
fn g5e_generic_boundaries() {
    install_det_random();
    unsafe {
        let (apk, ask) = box_keypair(0x3D);
        let (bpk, bsk) = box_keypair(0x4E);
        let n = vec![0x21u8; 24];

        // --- nullable `m` in every verify-only decryption path -------------
        for variant in ["", "curve25519xchacha20poly1305_"] {
            let easy = pair::<BoxEasy>(&format!("crypto_box_{variant}easy"));
            let oeasy = pair::<BoxEasy>(&format!("crypto_box_{variant}open_easy"));
            let bef = pair::<I3>(&format!("crypto_box_{variant}beforenm"));
            let oeasy_a = pair::<Afternm>(&format!("crypto_box_{variant}open_easy_afternm"));
            let odet_a =
                pair::<OpenDetAfternm>(&format!("crypto_box_{variant}open_detached_afternm"));
            let (_, k) = run_i3(&bef, 32, &bpk, &ask, "beforenm");
            for &mlen in [0usize, 1, 15, 16, 17, 31, 32, 33].iter() {
                let m = vec![0x66u8; mlen];
                let mut ct = vec![0u8; mlen + 16];
                assert_eq!(
                    (easy.0)(
                        ct.as_mut_ptr(),
                        m.as_ptr(),
                        mlen as u64,
                        n.as_ptr(),
                        bpk.as_ptr(),
                        ask.as_ptr()
                    ),
                    0
                );
                for (nm, ok) in [("good", true), ("corrupt", false)] {
                    let mut c = ct.clone();
                    if !ok {
                        c[0] ^= 0xff;
                    }
                    let a = (oeasy.0)(
                        ptr::null_mut(),
                        c.as_ptr(),
                        (mlen + 16) as u64,
                        n.as_ptr(),
                        apk.as_ptr(),
                        bsk.as_ptr(),
                    );
                    let b = (oeasy.1)(
                        ptr::null_mut(),
                        c.as_ptr(),
                        (mlen + 16) as u64,
                        n.as_ptr(),
                        apk.as_ptr(),
                        bsk.as_ptr(),
                    );
                    eq_i32(
                        &format!("{variant}open_easy m=NULL {nm} mlen={mlen}"),
                        a,
                        b,
                    );
                    eq_i32(
                        &format!("{variant}open_easy m=NULL {nm} expected"),
                        a,
                        if ok { 0 } else { -1 },
                    );
                    let a = (oeasy_a.0)(
                        ptr::null_mut(),
                        c.as_ptr(),
                        (mlen + 16) as u64,
                        n.as_ptr(),
                        k.as_ptr(),
                    );
                    let b = (oeasy_a.1)(
                        ptr::null_mut(),
                        c.as_ptr(),
                        (mlen + 16) as u64,
                        n.as_ptr(),
                        k.as_ptr(),
                    );
                    eq_i32(
                        &format!("{variant}open_easy_afternm m=NULL {nm} mlen={mlen}"),
                        a,
                        b,
                    );
                    let a = (odet_a.0)(
                        ptr::null_mut(),
                        c.as_ptr().add(16),
                        c.as_ptr(),
                        mlen as u64,
                        n.as_ptr(),
                        k.as_ptr(),
                    );
                    let b = (odet_a.1)(
                        ptr::null_mut(),
                        c.as_ptr().add(16),
                        c.as_ptr(),
                        mlen as u64,
                        n.as_ptr(),
                        k.as_ptr(),
                    );
                    eq_i32(
                        &format!("{variant}open_detached_afternm m=NULL {nm} mlen={mlen}"),
                        a,
                        b,
                    );
                }
            }
        }

        // --- nullable `siglen_p` / `mlen_p` -------------------------------
        let (pk, sk) = ed_keypair(0x5F);
        let sd = pair::<SignFn>("crypto_sign_ed25519_detached");
        let sgn = pair::<SignFn>("crypto_sign_ed25519");
        let op = pair::<SignFn>("crypto_sign_ed25519_open");
        for &mlen in [0usize, 1, 63, 64, 65].iter() {
            let m = vec![0x44u8; mlen];
            let mut csig = [0xAAu8; 64];
            let mut rsig = [0xAAu8; 64];
            let a = (sd.0)(
                csig.as_mut_ptr(),
                ptr::null_mut(),
                m.as_ptr(),
                mlen as u64,
                sk.as_ptr(),
            );
            let b = (sd.1)(
                rsig.as_mut_ptr(),
                ptr::null_mut(),
                m.as_ptr(),
                mlen as u64,
                sk.as_ptr(),
            );
            eq_i32(&format!("detached siglen_p=NULL mlen={mlen}"), a, b);
            eq_bytes(&format!("detached siglen_p=NULL mlen={mlen}"), &csig, &rsig);
            // m == NULL with mlen == 0 is fine (the pointer is not read)
            if mlen == 0 {
                let mut csig = [0xAAu8; 64];
                let mut rsig = [0xAAu8; 64];
                let a = (sd.0)(
                    csig.as_mut_ptr(),
                    ptr::null_mut(),
                    ptr::null(),
                    0,
                    sk.as_ptr(),
                );
                let b = (sd.1)(
                    rsig.as_mut_ptr(),
                    ptr::null_mut(),
                    ptr::null(),
                    0,
                    sk.as_ptr(),
                );
                eq_i32("detached m=NULL mlen=0", a, b);
                eq_bytes("detached m=NULL mlen=0", &csig, &rsig);
            }
            let mut csm = vec![0xAAu8; mlen + 64];
            let mut rsm = vec![0xAAu8; mlen + 64];
            let a = (sgn.0)(
                csm.as_mut_ptr(),
                ptr::null_mut(),
                m.as_ptr(),
                mlen as u64,
                sk.as_ptr(),
            );
            let b = (sgn.1)(
                rsm.as_mut_ptr(),
                ptr::null_mut(),
                m.as_ptr(),
                mlen as u64,
                sk.as_ptr(),
            );
            eq_i32(&format!("sign smlen_p=NULL mlen={mlen}"), a, b);
            eq_bytes(&format!("sign smlen_p=NULL mlen={mlen}"), &csm, &rsm);
            let a = (op.0)(
                ptr::null_mut(),
                ptr::null_mut(),
                csm.as_ptr(),
                (mlen + 64) as u64,
                pk.as_ptr(),
            );
            let b = (op.1)(
                ptr::null_mut(),
                ptr::null_mut(),
                csm.as_ptr(),
                (mlen + 64) as u64,
                pk.as_ptr(),
            );
            eq_i32(&format!("open m/mlen_p=NULL mlen={mlen}"), a, b);
            eq_i32(&format!("open m/mlen_p=NULL mlen={mlen} == 0"), a, 0);
        }

        // --- ph_final_create with siglen_p == NULL, ph_update with m == NULL --
        let init = pair::<PhInit>("crypto_sign_ed25519ph_init");
        let upd = pair::<PhUpdate>("crypto_sign_ed25519ph_update");
        let fin = pair::<PhCreate>("crypto_sign_ed25519ph_final_create");
        let mut sigs: Vec<Vec<u8>> = Vec::new();
        for which in 0..2 {
            let mut st = vec![0xAAu64; 64];
            let sp = st.as_mut_ptr() as *mut u8;
            let mut sig = [0xAAu8; 64];
            if which == 0 {
                (init.0)(sp);
                (upd.0)(sp, ptr::null(), 0);
                (fin.0)(sp, sig.as_mut_ptr(), ptr::null_mut(), sk.as_ptr());
            } else {
                (init.1)(sp);
                (upd.1)(sp, ptr::null(), 0);
                (fin.1)(sp, sig.as_mut_ptr(), ptr::null_mut(), sk.as_ptr());
            }
            sigs.push(sig.to_vec());
        }
        eq_bytes("ph siglen_p=NULL / m=NULL", &sigs[0], &sigs[1]);

        // --- nullable ctx in the from_string family (ctx_len must be 0) -----
        for name in [
            "crypto_core_ed25519_from_string",
            "crypto_core_ed25519_from_string_nu",
            "crypto_core_ed25519_scalar_from_string",
            "crypto_core_ristretto255_from_string",
            "crypto_core_ristretto255_scalar_from_string",
        ] {
            let f = pair::<FromStr>(name);
            for alg in [1i32, 2] {
                // ctx == NULL, ctx_len == 0 and msg == NULL, msg_len == 0
                let mut co = [0xAAu8; 32];
                let mut ro = [0xAAu8; 32];
                let a = (f.0)(co.as_mut_ptr(), ptr::null(), 0, ptr::null(), 0, alg);
                let b = (f.1)(ro.as_mut_ptr(), ptr::null(), 0, ptr::null(), 0, alg);
                let ctx = format!("{name} ctx/msg NULL alg={alg}");
                eq_i32(&ctx, a, b);
                eq_bytes(&ctx, &co, &ro);
                eq_i32(&format!("{ctx} == 0"), a, 0);
                // ctx_len exactly 255 and exactly 256 (the pre-hash boundary)
                for cl in [254usize, 255, 256, 257] {
                    let c = vec![b'Y'; cl];
                    let mut co = [0xAAu8; 32];
                    let mut ro = [0xAAu8; 32];
                    let a = (f.0)(co.as_mut_ptr(), c.as_ptr(), cl, ptr::null(), 0, alg);
                    let b = (f.1)(ro.as_mut_ptr(), c.as_ptr(), cl, ptr::null(), 0, alg);
                    let ctx = format!("{name} ctx_len={cl} alg={alg}");
                    eq_i32(&ctx, a, b);
                    eq_bytes(&ctx, &co, &ro);
                    eq_i32(&format!("{ctx} == 0"), a, 0);
                }
            }
        }

        // --- crypto_core_hsalsa20 with c == NULL (used by box_beforenm) -----
        type HSalsa = unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8) -> c_int;
        let (hc, hr) = pair::<HSalsa>("crypto_core_hsalsa20");
        let zero16 = [0u8; 16];
        let key = vec![0x9Bu8; 32];
        let mut co = [0xAAu8; 32];
        let mut ro = [0xAAu8; 32];
        let a = hc(co.as_mut_ptr(), zero16.as_ptr(), key.as_ptr(), ptr::null());
        let b = hr(ro.as_mut_ptr(), zero16.as_ptr(), key.as_ptr(), ptr::null());
        eq_i32("hsalsa20 c=NULL", a, b);
        eq_bytes("hsalsa20 c=NULL", &co, &ro);

        // --- one step past each documented length boundary ------------------
        // crypto_box_open_easy: 15 -> -1, 16 -> MAC check
        // crypto_box_open: 31 -> -1, 32 -> MAC check
        // crypto_box_seal_open: 47 -> -1, 48 -> MAC check
        let boxo = pair::<BoxEasy>("crypto_box_open");
        let sealo = pair::<SealOpen>("crypto_box_seal_open");
        let oeasy = pair::<BoxEasy>("crypto_box_open_easy");
        let buf = vec![0u8; 128];
        for (nm, len_bad, len_ok) in [("open_easy", 15u64, 16u64), ("open", 31, 32)] {
            for len in [len_bad, len_ok] {
                let f = if nm == "open_easy" { &oeasy } else { &boxo };
                let mut cm = vec![0xAAu8; 128];
                let mut rm = vec![0xAAu8; 128];
                let a = (f.0)(
                    cm.as_mut_ptr(),
                    buf.as_ptr(),
                    len,
                    n.as_ptr(),
                    apk.as_ptr(),
                    bsk.as_ptr(),
                );
                let b = (f.1)(
                    rm.as_mut_ptr(),
                    buf.as_ptr(),
                    len,
                    n.as_ptr(),
                    apk.as_ptr(),
                    bsk.as_ptr(),
                );
                let ctx = format!("boundary {nm} len={len}");
                eq_i32(&ctx, a, b);
                eq_bytes(&ctx, &cm, &rm);
                eq_i32(&format!("{ctx} == -1"), a, -1);
            }
        }
        for len in [47u64, 48] {
            let mut cm = vec![0xAAu8; 128];
            let mut rm = vec![0xAAu8; 128];
            let a = (sealo.0)(cm.as_mut_ptr(), buf.as_ptr(), len, apk.as_ptr(), bsk.as_ptr());
            let b = (sealo.1)(rm.as_mut_ptr(), buf.as_ptr(), len, apk.as_ptr(), bsk.as_ptr());
            let ctx = format!("boundary seal_open len={len}");
            eq_i32(&ctx, a, b);
            eq_bytes(&ctx, &cm, &rm);
            eq_i32(&format!("{ctx} == -1"), a, -1);
        }

        // zero-length outputs / inputs on the point APIs
        let ivp = pair::<Chk>("crypto_core_ed25519_is_valid_point");
        eq_i32(
            "ed25519 base point is valid",
            run_chk(&ivp, &hx(ED_BASE_HEX), "ed25519 base"),
            1,
        );
        let rivp = pair::<Chk>("crypto_core_ristretto255_is_valid_point");
        eq_i32(
            "ristretto base point is valid",
            run_chk(&rivp, &hx(R_BASE_HEX), "ristretto base"),
            1,
        );

        // --- one step past the `ge25519_is_canonical` boundary --------------
        // The C predicate is `p[1..31] == 0xff && (p[31] & 0x7f) == 0x7f &&
        // p[0] >= 0xed`, so byte 0 sweeping 0xe0..0xff walks straight across
        // the accept/reject edge. Every consumer of the predicate is probed.
        let smed = pair::<I3>("crypto_scalarmult_ed25519");
        let smedn = pair::<I3>("crypto_scalarmult_ed25519_noclamp");
        let eadd = pair::<I3>("crypto_core_ed25519_add");
        let esub = pair::<I3>("crypto_core_ed25519_sub");
        let p2c = pair::<I2>("crypto_sign_ed25519_pk_to_curve25519");
        let vf = pair::<VerifyFn>("crypto_sign_ed25519_verify_detached");
        let good_n = hx("0700000000000000000000000000000000000000000000000000000000000000");
        let sig64 = [0u8; 64];
        let msg4 = [0u8; 4];
        for hi in [0x7fu8, 0xff] {
            for b0 in 0xe0u8..=0xff {
                let mut p = vec![0xffu8; 32];
                p[0] = b0;
                p[31] = hi;
                let tag = format!("edcanon b0={b0:#04x} b31={hi:#04x}");
                run_chk(&ivp, &p, &format!("{tag} is_valid_point"));
                run_i3(&smed, 32, &good_n, &p, &format!("{tag} scalarmult_ed25519"));
                run_i3(&smedn, 32, &good_n, &p, &format!("{tag} scalarmult_noclamp"));
                run_i3(&eadd, 32, &p, &hx(ED_BASE_HEX), &format!("{tag} add"));
                run_i3(&esub, 32, &hx(ED_BASE_HEX), &p, &format!("{tag} sub"));
                run_i2(&p2c, 32, &p, &format!("{tag} pk_to_curve25519"));
                let a = (vf.0)(sig64.as_ptr(), msg4.as_ptr(), 4, p.as_ptr());
                let b = (vf.1)(sig64.as_ptr(), msg4.as_ptr(), 4, p.as_ptr());
                eq_i32(&format!("{tag} verify_detached"), a, b);
            }
            // and the same shape with one interior byte cleared (must become
            // canonical again, so the predicate must flip)
            for idx in [1usize, 15, 30] {
                let mut p = vec![0xffu8; 32];
                p[0] = 0xed;
                p[31] = hi;
                p[idx] = 0xfe;
                let tag = format!("edcanon interior[{idx}] b31={hi:#04x}");
                run_chk(&ivp, &p, &format!("{tag} is_valid_point"));
                run_i3(&smed, 32, &good_n, &p, &format!("{tag} scalarmult_ed25519"));
                run_i2(&p2c, 32, &p, &format!("{tag} pk_to_curve25519"));
            }
        }

        // --- one step past the `sc25519_is_canonical` boundary --------------
        // s is compared against L byte-by-byte from the top down; sweep byte 0
        // and byte 31 of L, and every single-byte increment/decrement of L.
        let ecan = pair::<Chk>("crypto_core_ed25519_scalar_is_canonical");
        let rcan = pair::<Chk>("crypto_core_ristretto255_scalar_is_canonical");
        let l = hx(L_HEX);
        for i in 0..32usize {
            for delta in [-1i16, 1] {
                let mut s = l.clone();
                let v = s[i] as i16 + delta;
                if !(0..=255).contains(&v) {
                    continue;
                }
                s[i] = v as u8;
                let tag = format!("sccanon L[{i}]{delta:+}");
                let a = run_chk(&ecan, &s, &format!("{tag} ed25519"));
                let b = run_chk(&rcan, &s, &format!("{tag} ristretto"));
                eq_i32(&format!("{tag} ed25519 == ristretto"), a, b);
                // and the corresponding S in a signature must be gated the same
                let mut sig = [0u8; 64];
                sig[32..].copy_from_slice(&s);
                let x = (vf.0)(sig.as_ptr(), msg4.as_ptr(), 4, pk.as_ptr());
                let y = (vf.1)(sig.as_ptr(), msg4.as_ptr(), 4, pk.as_ptr());
                eq_i32(&format!("{tag} verify_detached S"), x, y);
            }
        }
        for b0 in 0xe0u8..=0xff {
            let mut s = l.clone();
            s[0] = b0;
            run_chk(&ecan, &s, &format!("sccanon byte0={b0:#04x}"));
        }
        for b31 in 0x00u8..=0x20 {
            let mut s = l.clone();
            s[31] = b31;
            run_chk(&ecan, &s, &format!("sccanon byte31={b31:#04x}"));
        }

        // --- curve25519 field-element boundary for scalarmult --------------
        // p values around 2^255-19 (the ref10 `fe25519_frombytes` masks bit 255
        // and does NOT reduce, so p, p+1, ... must behave like 0, 1, ...).
        let smc = pair::<I3>("crypto_scalarmult_curve25519");
        for hi in [0x7fu8, 0xff] {
            for b0 in 0xe0u8..=0xff {
                let mut p = vec![0xffu8; 32];
                p[0] = b0;
                p[31] = hi;
                run_i3(
                    &smc,
                    32,
                    &good_n,
                    &p,
                    &format!("x25519 field edge b0={b0:#04x} b31={hi:#04x}"),
                );
            }
        }

        // --- ristretto255 from_hash with structured 64-byte halves ---------
        let fh = pair::<I2>("crypto_core_ristretto255_from_hash");
        let halves: Vec<Vec<u8>> = vec![
            vec![0u8; 32],
            vec![0xffu8; 32],
            hx(L_HEX),
            hx(ONE_HEX),
            hx(ZERO_HEX),
            hx("edffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f"),
            hx("ecffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f"),
        ];
        for (i, a) in halves.iter().enumerate() {
            for (j, b) in halves.iter().enumerate() {
                let mut r = Vec::with_capacity(64);
                r.extend_from_slice(a);
                r.extend_from_slice(b);
                let (ret, p) = run_i2(&fh, 32, &r, &format!("from_hash halves[{i}/{j}]"));
                eq_i32(&format!("from_hash halves[{i}/{j}] == 0"), ret, 0);
                eq_i32(
                    &format!("from_hash halves[{i}/{j}] valid"),
                    run_chk(&rivp, &p, "valid"),
                    1,
                );
            }
        }
    }
}

// ===========================================================================
// ERRORS rows 542, 544, 545, 546, 564, 567, 574, 575 — abort paths
// ===========================================================================
#[test]
fn g5e_abort_paths() {
    // rows 542, 544, 545, 546: mlen > *_MESSAGEBYTES_MAX -> sodium_misuse()
    for case in [
        "box_easy_mlen_max",
        "box_easy_afternm_mlen_max",
        "xchacha_easy_mlen_max",
        "xchacha_easy_afternm_mlen_max",
        // rows 564, 567: crypto_box_seal / xchacha seal
        "box_seal_mlen_max",
        "xchacha_seal_mlen_max",
        // rows 574, 575: crypto_kx_*_session_keys with rx == tx == NULL
        "kx_client_both_null",
        "kx_server_both_null",
    ] {
        let t = diff_abort_case(case);
        assert!(
            t.signal.is_some(),
            "abort case {case:?}: expected both children to die from a signal, got {t:?}"
        );
    }
    // control: the same calls with an in-range length must NOT abort
    for case in ["box_easy_ok", "kx_client_one_null"] {
        let t = diff_abort_case(case);
        assert_eq!(
            t.code,
            Some(0),
            "control case {case:?}: expected a clean exit in both, got {t:?}"
        );
    }
}

#[test]
fn zz_abort_child() {
    let Some((case, is_c)) = child_case() else {
        return; // normal test run: nothing to do
    };
    let l = libs();
    let h = if is_c { l.c } else { l.rs };
    macro_rules! sym {
        ($t:ty, $n:literal) => {{
            let s: libloading::Symbol<$t> = unsafe { h.get(concat!($n, "\0").as_bytes()) }.unwrap();
            s
        }};
    }
    unsafe {
        // a valid keypair, derived without touching randombytes
        let skp = sym!(SeedKeypair, "crypto_box_seed_keypair");
        let seed = [0x42u8; 32];
        let mut pk = [0u8; 32];
        let mut sk = [0u8; 32];
        assert_eq!(skp(pk.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr()), 0);
        let n = [0x11u8; 24];
        let mut cbuf = [0u8; 256];
        let m = [0u8; 64];
        let k = [0x22u8; 32];

        match case.as_str() {
            // --- row 542 -------------------------------------------------
            "box_easy_mlen_max" => {
                let f = sym!(BoxEasy, "crypto_box_easy");
                f(
                    cbuf.as_mut_ptr(),
                    m.as_ptr(),
                    u64::MAX,
                    n.as_ptr(),
                    pk.as_ptr(),
                    sk.as_ptr(),
                );
            }
            // --- row 544 -------------------------------------------------
            "box_easy_afternm_mlen_max" => {
                let f = sym!(Afternm, "crypto_box_easy_afternm");
                f(cbuf.as_mut_ptr(), m.as_ptr(), u64::MAX, n.as_ptr(), k.as_ptr());
            }
            // --- row 545 -------------------------------------------------
            "xchacha_easy_mlen_max" => {
                let f = sym!(BoxEasy, "crypto_box_curve25519xchacha20poly1305_easy");
                f(
                    cbuf.as_mut_ptr(),
                    m.as_ptr(),
                    u64::MAX,
                    n.as_ptr(),
                    pk.as_ptr(),
                    sk.as_ptr(),
                );
            }
            // --- row 546 -------------------------------------------------
            "xchacha_easy_afternm_mlen_max" => {
                let f = sym!(Afternm, "crypto_box_curve25519xchacha20poly1305_easy_afternm");
                f(cbuf.as_mut_ptr(), m.as_ptr(), u64::MAX, n.as_ptr(), k.as_ptr());
            }
            // --- row 564 -------------------------------------------------
            "box_seal_mlen_max" => {
                let f = sym!(Seal, "crypto_box_seal");
                f(cbuf.as_mut_ptr(), m.as_ptr(), u64::MAX, pk.as_ptr());
            }
            // --- row 567 -------------------------------------------------
            "xchacha_seal_mlen_max" => {
                let f = sym!(Seal, "crypto_box_curve25519xchacha20poly1305_seal");
                f(cbuf.as_mut_ptr(), m.as_ptr(), u64::MAX, pk.as_ptr());
            }
            // --- row 574 -------------------------------------------------
            "kx_client_both_null" => {
                let f = sym!(KxSess, "crypto_kx_client_session_keys");
                f(
                    ptr::null_mut(),
                    ptr::null_mut(),
                    pk.as_ptr(),
                    sk.as_ptr(),
                    pk.as_ptr(),
                );
            }
            // --- row 575 -------------------------------------------------
            "kx_server_both_null" => {
                let f = sym!(KxSess, "crypto_kx_server_session_keys");
                f(
                    ptr::null_mut(),
                    ptr::null_mut(),
                    pk.as_ptr(),
                    sk.as_ptr(),
                    pk.as_ptr(),
                );
            }
            // --- controls ------------------------------------------------
            "box_easy_ok" => {
                let f = sym!(BoxEasy, "crypto_box_easy");
                let r = f(
                    cbuf.as_mut_ptr(),
                    m.as_ptr(),
                    64,
                    n.as_ptr(),
                    pk.as_ptr(),
                    sk.as_ptr(),
                );
                assert_eq!(r, 0);
            }
            "kx_client_one_null" => {
                let f = sym!(KxSess, "crypto_kx_client_session_keys");
                let mut buf = [0u8; 32];
                let r = f(
                    buf.as_mut_ptr(),
                    ptr::null_mut(),
                    pk.as_ptr(),
                    sk.as_ptr(),
                    pk.as_ptr(),
                );
                assert_eq!(r, 0);
            }
            other => panic!("unknown abort case {other:?}"),
        }
        std::process::exit(0); // reached only if the call did NOT abort
    }
}

#[allow(dead_code)]
fn _keep_types_used(_: *const c_char, _: &[u8]) {
    let _ = ONE_HEX;
}
