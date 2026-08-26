//! Characterises the *one* place where the C library is not a function of its
//! inputs, so the rest of the suite knows exactly which inputs it may compare and
//! why `common::zero_stack` exists.
//!
//! `c2MakeProxy` has no `C2_TYPE_POLY` case and no `default:`, so it writes
//! **nothing** for a polygon. `c2GJK` declares `c2Proxy pA, pB;` without an
//! initialiser, therefore every call with `typeA`/`typeB == C2_TYPE_POLY` (or any
//! out-of-range `C2_TYPE`) reads an **uninitialised stack local**:
//!
//! ```c
//! c2Proxy pA; c2Proxy pB;            /* uninitialised                     */
//! c2MakeProxy(A, typeA, &pA);        /* no-op for POLY                    */
//! c2MakeProxy(B, typeB, &pB);        /* no-op for POLY                    */
//! ...
//! s.a.sB = c2Mulxv(bx, pB.verts[0]);            /* reads garbage          */
//! int iB = c2Support(pB.verts, pB.count, ...);  /* garbage count -> OOB   */
//! ```
//!
//! Measured behaviour of the compiled C `.so` (gcc 11.5.0, `cmake` default flags,
//! i.e. `-O0`):
//!
//! * From a debug test binary, `pB.verts[0]` reads back the two halves of a **stack
//!   address** — e.g. `(0.001952216, 4.5751e-41)` == `0x00007f89_3affe180` — so
//!   `c2GJK` returns a nonsense distance. The value moves with ASLR from run to run.
//! * From a minimal fresh-process C driver (`dlopen` + one
//!   `omni_manifold(AABB, CAPSULE)` call), the garbage `pB.count` makes `c2Support`
//!   walk off the end of the array and the process dies with **SIGSEGV, exit 139,
//!   reproducibly**. The same happens from a release-profile test binary, which is
//!   why the demonstration below runs in a **child process**.
//!
//! So for these inputs the C library has no defined behaviour and no translation can
//! be byte-identical to it. `src/gjk.rs` zero-initialises both proxies, reproducing
//! the "virgin, zero-filled stack page" case: a POLY operand behaves as a single
//! point at the origin with radius 0. That is deterministic, never crashes, and is
//! asserted here.
//!
//! `common::zero_stack()` forces the C side into that same state, which is what lets
//! `tests/phase_b_manifolds.rs` and `tests/phase_b_api.rs` compare the polygon paths
//! byte-for-byte after all (and with them the five `static` helpers `c2Clip`,
//! `c2SidePlanes`, `c2SidePlanesFromPoly`, `c2KeepDeep`, `c2Incident`, which are
//! unreachable through any other entry point).
//!
//! ## Which inputs are affected
//!
//! | entry point | affected when |
//! |---|---|
//! | `c2GJK` | `typeA` or `typeB` is `C2_TYPE_POLY` or out of range |
//! | `c2CapsuletoPolyManifold` | **always** (it calls `c2GJK` with `C2_TYPE_POLY`) |
//! | `c2AABBtoCapsuleManifold` | **always** (it promotes the AABB to a `c2Poly`) |
//! | `c2Collide` / `omni_manifold` | pairs `(AABB, CAPSULE)` and `(CAPSULE, AABB)` |
//!
//! Every other entry point and every other type pair *is* a pure function of its
//! inputs and is compared byte-for-byte by the `phase_b_*` / `phase_c_*` suites.
#![allow(non_snake_case)]
#![allow(clippy::unnecessary_cast, clippy::needless_range_loop, clippy::let_and_return)]
#![allow(clippy::field_reassign_with_default)]

mod common;
use common::*;
use std::ffi::c_void;

/// Deliberately dirty the stack below the current frame with a recognisable pattern.
#[inline(never)]
fn dirty_stack(depth: u32) -> u64 {
    let mut buf = [0u64; 96];
    for (i, b) in buf.iter_mut().enumerate() {
        *b = 0xDEAD_BEEF_CAFE_0000u64 | (i as u64) | ((depth as u64) << 32);
    }
    let mut acc = std::hint::black_box(&buf).iter().fold(0u64, |a, b| a ^ b);
    if depth > 0 {
        acc ^= dirty_stack(depth - 1);
    }
    std::hint::black_box(acc)
}

/// `zeroed`: run [`zero_stack`] as the *last* statement before the FFI call.
///
/// It has to be last. Anything called in between -- even `poison_v` -- gets a frame
/// in the very region that was just zeroed and dirties it again, which is why every
/// wrapper in this suite follows the shape "prepare locals, `zero_stack()`, call".
fn gjk_poly_impl(
    f: &libloading::Symbol<'_, FnGJK>,
    cap: &c2Capsule,
    poly: &c2Poly,
    zeroed: bool,
) -> (f32, c2v, c2v, i32) {
    let (mut a, mut b, mut it) = (poison_v(1), poison_v(2), -12345i32);
    if zeroed {
        zero_stack();
    }
    let d = unsafe {
        f(
            cap as *const c2Capsule as *const c_void,
            C2_TYPE_CAPSULE,
            std::ptr::null(),
            poly as *const c2Poly as *const c_void,
            C2_TYPE_POLY,
            std::ptr::null(),
            &mut a,
            &mut b,
            0,
            &mut it,
            std::ptr::null_mut(),
        )
    };
    (d, a, b, it)
}

fn gjk_poly(
    f: &libloading::Symbol<'_, FnGJK>,
    cap: &c2Capsule,
    poly: &c2Poly,
) -> (f32, c2v, c2v, i32) {
    gjk_poly_impl(f, cap, poly, false)
}

fn gjk_poly_zeroed(
    f: &libloading::Symbol<'_, FnGJK>,
    cap: &c2Capsule,
    poly: &c2Poly,
) -> (f32, c2v, c2v, i32) {
    gjk_poly_impl(f, cap, poly, true)
}

// ---------------------------------------------------------------------------
// What the Rust side guarantees
// ---------------------------------------------------------------------------

/// The Rust side implements the zero-proxy model: a POLY operand behaves as the
/// single point (0,0) with radius 0, so `c2GJK(capsule, POLY)` returns the distance
/// from the capsule *segment* to the origin with `outB == (0,0)`.
#[test]
fn rust_poly_operand_is_the_zero_proxy() {
    let l = libs();
    let (_cf, rf) = l.get::<FnGJK>("c2GJK");

    // Closest point of the segment [(3,4),(5,6)] to (0,0) is (3,4), distance 5.
    let cap = c2Capsule { a: v(3.0, 4.0), b: v(5.0, 6.0), r: 1.0 };
    let mut poly = c2Poly::default();
    poly.count = 5;
    for i in 0..5 {
        poly.verts[i] = v(10.0 + i as f32, 20.0 - i as f32);
    }

    let (rd, ra, rb, rit) = gjk_poly(&rf, &cap, &poly);
    assert_eq!(rd, 5.0, "Rust must report the segment-to-origin distance");
    assert_eq!(ra, v(3.0, 4.0), "witness on A");
    assert_eq!(rb, v(0.0, 0.0), "witness on B is the origin (zero proxy)");
    assert_eq!(rit, 0, "terminates immediately");
}

/// ...and it is *deterministic*: identical bytes no matter how dirty the stack below
/// the caller's frame is. This is exactly the property the C library lacks.
#[test]
fn rust_poly_operand_is_deterministic_under_dirty_stack() {
    let l = libs();
    let (_cf, rf) = l.get::<FnGJK>("c2GJK");
    let cap = c2Capsule { a: v(3.0, 4.0), b: v(5.0, 6.0), r: 1.0 };
    let poly = c2Poly::default();

    let reference = gjk_poly(&rf, &cap, &poly);
    for depth in [0u32, 1, 4, 16, 40] {
        let sink = dirty_stack(depth);
        let got = gjk_poly(&rf, &cap, &poly);
        assert_eq!(
            (got.0.to_bits(), got.1, got.2, got.3),
            (reference.0.to_bits(), reference.1, reference.2, reference.3),
            "Rust c2GJK(POLY) changed after dirtying the stack (depth {depth}, sink {sink:x})"
        );
    }
}

/// Rust must never crash on the POLY path, however the polygon is shaped —
/// including `count` values far outside `0..=8`.
#[test]
fn rust_poly_path_never_crashes() {
    let l = libs();
    let (_cf, rf) = l.get::<FnGJK>("c2GJK");
    let mut rng = Rng::new(0x5EED_1234);
    for trial in 0..200 {
        let cap = c2Capsule { a: rng.vec_mixed(20.0), b: rng.vec_mixed(20.0), r: rng.f_mixed(3.0) };
        let mut poly = c2Poly::default();
        poly.count = match trial % 5 {
            0 => 0,
            1 => -7,
            2 => 8,
            3 => 4,
            _ => rng.below(9) as i32,
        };
        for i in 0..8 {
            poly.verts[i] = rng.vec_mixed(10.0);
            poly.norms[i] = rng.vec_mixed(1.0);
        }
        let (d, a, b, it) = gjk_poly(&rf, &cap, &poly);
        assert!((0..=20).contains(&it), "trial {trial}: iterations out of range: {it}");
        let _ = (d, a, b);
    }
}

// ---------------------------------------------------------------------------
// What the C side actually does — demonstrated in a CHILD PROCESS, because the
// uninitialised `pB.count` can make `c2Support` walk into unmapped memory.
// ---------------------------------------------------------------------------

const CHILD_ENV: &str = "OMNI_UB_CHILD";

/// Runs in a child process (see [`c_poly_operand_is_undefined_behaviour`]). Calls the
/// C library on the POLY path with the stack left as-is and prints what came back.
#[test]
#[ignore = "spawned as a child process by c_poly_operand_is_undefined_behaviour"]
fn ub_child_calls_c_poly_path() {
    if std::env::var(CHILD_ENV).is_err() {
        // Never run this directly: it may SIGSEGV by design.
        println!("SKIPPED (set {CHILD_ENV}=1 to run)");
        return;
    }
    let l = libs();
    let (cf, _rf) = l.get::<FnGJK>("c2GJK");
    let cap = c2Capsule { a: v(3.0, 4.0), b: v(5.0, 6.0), r: 1.0 };
    let poly = c2Poly::default();
    // No zero_stack() here -- that is the whole point.
    let (cd, ca, cb, cit) = gjk_poly(&cf, &cap, &poly);
    println!("RESULT d={} outA=({},{}) outB_bits=(0x{:08x},0x{:08x}) iters={cit}",
        cd, ca.x, ca.y, cb.x.to_bits(), cb.y.to_bits());
    // pA *is* initialised for a capsule, so outA stays meaningful -- which pinpoints
    // pB as the uninitialised operand. Reported rather than asserted: this child is
    // deliberately executing undefined behaviour, so it must not fail the build.
    println!("OUTA_OK {}", ca == v(3.0, 4.0));
    if cd == 5.0 && cb.x == 0.0 && cb.y == 0.0 {
        println!("VERDICT zero-proxy (this run's stack happened to be zero-filled)");
    } else {
        println!("VERDICT garbage (uninitialised stack observed)");
    }
}

/// Demonstrates that the C library's POLY path is **not** a function of its inputs:
/// the child either reports a value derived from an uninitialised stack slot, or dies
/// with a signal. Both outcomes are recorded; neither is something a translation can
/// reproduce, which is why `common::zero_stack()` is used everywhere else.
#[test]
fn c_poly_operand_is_undefined_behaviour() {
    use std::process::Command;
    let exe = std::env::current_exe().expect("current_exe");
    let out = Command::new(&exe)
        .args(["ub_child_calls_c_poly_path", "--ignored", "--exact", "--nocapture", "--test-threads=1"])
        .env(CHILD_ENV, "1")
        .output()
        .expect("failed to spawn the child test process");

    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    println!("child status: {:?}", out.status);
    for line in stdout.lines().filter(|l| l.starts_with("RESULT") || l.starts_with("VERDICT")) {
        println!("child: {line}");
    }

    // The child is deliberately executing undefined behaviour, so it may die from a
    // signal, or survive and report either garbage or (if its stack happened to be
    // zero-filled) the zero-proxy answer. All three are informative; anything else
    // means the probe itself is broken.
    let crashed = out.status.code().is_none();
    let garbage = stdout.contains("VERDICT garbage");
    let zero_proxy = stdout.contains("VERDICT zero-proxy");

    assert!(
        crashed || garbage || zero_proxy,
        "the child neither crashed nor produced a verdict (exit {:?}).\n\
         --- child stdout ---\n{stdout}\n--- child stderr ---\n{stderr}",
        out.status.code()
    );

    if crashed {
        println!(
            "=> C SIGSEGV'd on the POLY path: the uninitialised `pB.count` made \
             c2Support read unmapped memory. Confirms ERRORS.md rows 19/20."
        );
    } else if garbage {
        println!(
            "=> C returned a value derived from an uninitialised stack slot. \
             Confirms ERRORS.md rows 19/20."
        );
    } else {
        println!(
            "=> C happened to see a zero-filled stack this run, i.e. it agreed with \
             the model src/gjk.rs implements. Still UB, just benign here."
        );
    }
}

/// With `zero_stack()` applied to both sides the C library agrees with Rust exactly.
/// This is the property the rest of the suite relies on.
#[test]
fn zero_stack_makes_c_agree_with_rust() {
    let l = libs();
    let (cf, rf) = l.get::<FnGJK>("c2GJK");
    let cap = c2Capsule { a: v(3.0, 4.0), b: v(5.0, 6.0), r: 1.0 };
    let poly = c2Poly::default();

    // Warm up so lazy PLT resolution inside the .so does not dirty the stack during
    // the measured calls.
    warmup(|| {
        let _ = gjk_poly_zeroed(&cf, &cap, &poly);
        let _ = gjk_poly_zeroed(&rf, &cap, &poly);
    });

    for depth in [0u32, 1, 4, 16, 40] {
        let _ = dirty_stack(depth);
        let (cd, ca, cb, cit) = gjk_poly_zeroed(&cf, &cap, &poly);
        let (rd, ra, rb, rit) = gjk_poly_zeroed(&rf, &cap, &poly);
        let ctx = format!("after dirty_stack({depth}) + zero_stack()");
        eq_f32("c2GJK(POLY) dist", &ctx, cd, rd);
        eq("c2GJK(POLY) outA", &ctx, &ca, &ra);
        eq("c2GJK(POLY) outB", &ctx, &cb, &rb);
        eq_i32("c2GJK(POLY) iters", &ctx, cit, rit);
        assert_eq!(cd, 5.0, "with a zeroed stack the C side sees the zero proxy");
    }
}
