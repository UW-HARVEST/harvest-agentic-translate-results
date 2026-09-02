//! Phase C (documented indeterminacy) — the `C2_TYPE_POLY` proxy path.
//!
//! `c2MakeProxy` (lib.c:126) has no `C2_TYPE_POLY` case and `c2GJK` (lib.c:437)
//! declares `c2Proxy pA, pB;` uninitialized, so on the poly path — which every
//! AABB<->CAPSULE collision goes through via `c2AABBtoCapsuleManifold` — the C
//! reads an indeterminate local. Its contents come from the *caller's* stack.
//!
//! `stack_dependence_of_the_c_library` demonstrates that: with stale bytes
//! below the call the C changes its own answer for identical inputs, so no
//! translation can match it unconditionally. `scrubbing_pins_both_libraries`
//! shows that zero-filling the stack below the call pins the C to all-zeros,
//! which is exactly the state `src/lib.rs` initializes its proxies to — and
//! then the two libraries agree bit-for-bit. Every differential test in this
//! suite calls `scrub_stack()` before crossing the FFI boundary for that
//! reason.

#![allow(non_snake_case)]

mod common;
use common::*;

/// A capsule-vs-AABB pair that lands close enough to the `d < A.r` threshold
/// for the indeterminate proxy to flip the branch.
const CAP: [f32; 5] = [-1.0, 2.5, 3.0, -1.0, 1.5];
const BOX: [f32; 5] = [0.5, -3.0, 1.5, 2.0, 2.0];

fn call(f: &FnOmni) -> c2Manifold {
    let mut m = poison_manifold(88);
    unsafe {
        f(
            &mut m,
            C2_TYPE_CAPSULE,
            CAP[0], CAP[1], CAP[2], CAP[3], CAP[4],
            C2_TYPE_AABB,
            BOX[0], BOX[1], BOX[2], BOX[3], BOX[4],
        )
    };
    m
}

/// Recurse `depth` times, leaving a recognisable pattern on the stack at each
/// level, then make the call. Without scrubbing the callee sees that pattern.
#[inline(never)]
fn call_after_dirtying(f: &FnOmni, depth: usize, scrub: bool) -> c2Manifold {
    if depth == 0 {
        if scrub {
            scrub_stack();
        }
        return call(f);
    }
    // Fill with 1 rather than something like 0xDEADBEEF: the C reads these
    // bytes as a `c2Proxy`, and a huge garbage `count` would send `c2Support`
    // off the end of the 8-element array and segfault. `1` is observable but
    // harmless.
    let mut pad = [1u32; 64];
    std::hint::black_box(&mut pad);
    let m = call_after_dirtying(f, depth - 1, scrub);
    std::hint::black_box(&pad);
    m
}

/// Evidence that the C's poly-path answer is a function of the caller's stack.
///
/// `#[ignore]`d on purpose: it deliberately calls the C with stale bytes below
/// the frame, which is real undefined behaviour — depending on what those bytes
/// decode to as a `c2Proxy.count`, `c2Support` can walk off the end of the
/// 8-element vertex array and segfault. Run it explicitly with
/// `cargo test --release -- --ignored stack_dependence` to reproduce the
/// finding; it is not part of the default suite because a crash here is the C
/// misbehaving, not a translation defect.
#[test]
#[ignore]
fn stack_dependence_of_the_c_library() {
    let p = pair();
    let (cO, _) = p.get::<FnOmni>(b"omni_manifold");
    let mut results = Vec::new();
    for d in 0..6 {
        results.push(raw(&call_after_dirtying(&cO, d, false)));
    }
    let all_same = results.windows(2).all(|w| w[0] == w[1]);
    assert!(
        !all_same,
        "expected the C library's poly-path answer to depend on caller stack \
         contents; if this now holds it means the uninitialized c2Proxy stopped \
         being observable and the note in ERRORS.md should be revisited"
    );
}

#[test]
fn scrubbing_pins_both_libraries() {
    let p = pair();
    let (cO, rO) = p.get::<FnOmni>(b"omni_manifold");
    let mut c_first: Option<Vec<u8>> = None;
    for d in 0..6 {
        let c = raw(&call_after_dirtying(&cO, d, true));
        let r = raw(&call_after_dirtying(&rO, d, true));
        assert_eq!(c, r, "depth {d}: C and Rust disagree after scrubbing");
        match &c_first {
            None => c_first = Some(c),
            Some(f) => assert_eq!(
                *f, c,
                "depth {d}: C still varies with caller stack despite scrubbing"
            ),
        }
    }
}

/// The same conclusion, at scale: a broad randomized sweep of the two
/// AABB<->CAPSULE type orders must be byte-identical once the stack is scrubbed.
#[test]
fn poly_path_sweep_matches_under_scrubbing() {
    let p = pair();
    let (cf, rf) = p.get::<FnOmni>(b"omni_manifold");
    let mut rng = Rng::new(0xF0F0);
    for k in 0..120_000 {
        let (ta, tb) = if k % 2 == 0 {
            (C2_TYPE_CAPSULE, C2_TYPE_AABB)
        } else {
            (C2_TYPE_AABB, C2_TYPE_CAPSULE)
        };
        let v: [f32; 10] = std::array::from_fn(|_| match k % 4 {
            0 => rng.grid(0.5, 8),
            1 => rng.sym(5.0),
            2 => rng.sym(1e12),
            _ => rng.spicy(),
        });
        let mut cm = poison_manifold(k as u8);
        let mut rm = cm;
        scrub_stack();
        unsafe {
            cf(&mut cm, ta, v[0], v[1], v[2], v[3], v[4], tb, v[5], v[6], v[7], v[8], v[9])
        };
        scrub_stack();
        unsafe {
            rf(&mut rm, ta, v[0], v[1], v[2], v[3], v[4], tb, v[5], v[6], v[7], v[8], v[9])
        };
        same(&format!("poly path k={k} ta={ta} tb={tb}"), &cm, &rm);
    }
}
