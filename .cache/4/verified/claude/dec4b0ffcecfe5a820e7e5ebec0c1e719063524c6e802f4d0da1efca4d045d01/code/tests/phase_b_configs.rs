//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Every test drives BOTH the C `.so` and the Rust `.so` through `libloading`
//! and compares the returned `lm_vec2` bit-for-bit.

mod harness;

use harness::*;

// ---------------------------------------------------------------------------
// C33 — ABI shape
// ---------------------------------------------------------------------------

#[test]
fn abi_struct_layout() {
    // `typedef struct lm_vec2 { float x, y; } lm_vec2;`
    assert_eq!(std::mem::size_of::<Vec2>(), 8, "lm_vec2 must be 8 bytes");
    assert_eq!(std::mem::align_of::<Vec2>(), 4, "lm_vec2 must be 4-aligned");
    let v = Vec2::new(1.0, 2.0);
    let base = &v as *const Vec2 as usize;
    assert_eq!(&v.x as *const f32 as usize - base, 0, "x at offset 0");
    assert_eq!(&v.y as *const f32 as usize - base, 4, "y at offset 4");

    // A swapped x/y in the returned register pair would show up here: the unit
    // triangle (0,0),(1,0),(0,1) with p=(0.25,0.75) has u (along p3-p1) = 0.75
    // and v (along p2-p1) = 0.25, which are distinguishable.
    let r = c_call(
        Vec2::new(0.0, 0.0),
        Vec2::new(1.0, 0.0),
        Vec2::new(0.0, 1.0),
        Vec2::new(0.25, 0.75),
    );
    assert_eq!(r.x.to_bits(), 0.75f32.to_bits(), "C: u must be 0.75, got {r:?}");
    assert_eq!(r.y.to_bits(), 0.25f32.to_bits(), "C: v must be 0.25, got {r:?}");
    let rr = rust_call(
        Vec2::new(0.0, 0.0),
        Vec2::new(1.0, 0.0),
        Vec2::new(0.0, 1.0),
        Vec2::new(0.25, 0.75),
    );
    assert_eq!(rr.bits(), r.bits(), "rust {rr:?} != C {r:?}");
}

#[test]
fn harness_loads_both_libraries() {
    let a = api();
    assert!(a.c_path.exists(), "C .so missing: {}", a.c_path.display());
    assert!(
        a.rust_path.exists(),
        "rust .so missing: {}",
        a.rust_path.display()
    );
    assert_ne!(
        a.c_path, a.rust_path,
        "must load two distinct libraries, not the same file twice"
    );
    eprintln!("C   .so: {}", a.c_path.display());
    eprintln!("rust.so: {}", a.rust_path.display());
}

/// Guard: assert the loaded C `.so` is the **reference** build, i.e. the one
/// produced by the prescribed command
///
/// ```sh
/// cd c_src && mkdir -p build && cd build
/// cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
/// ```
///
/// which sets no `CMAKE_BUILD_TYPE` and therefore compiles at `-O0`.
///
/// This matters because GCC's choice of x86 SSE *destination* operand — which
/// decides **which NaN payload survives** when two NaNs meet in one
/// instruction — differs between `-O0` and `-O3`. The two C builds disagree
/// with *each other*: for the input below, the `-O0` build returns
/// `(0x7fc01234, 0x7fdeadbe)` while an `-O3` build returns
/// `(0x7fc01234, 0x7fc01234)`. No single Rust binary can be bit-identical to
/// both, so `src/lib.rs` reproduces the reference (`-O0`) build.
///
/// If this test fails, the C `.so` was built with optimization; rebuild it with
/// the command above rather than "fixing" the Rust.
#[test]
fn reference_c_build_is_unoptimized() {
    let (p1, p2, p3, p) = from_slots([
        NAN_PAYLOAD_A, // p1.x = 0x7fc01234
        0.0,
        1.0,
        0.0,
        NAN_PAYLOAD_B, // p3.x = 0x7fdeadbe
        1.0,
        0.5,
        0.5,
    ]);
    let c = c_call(p1, p2, p3, p);
    assert_eq!(
        c.bits(),
        (0x7FC0_1234, 0x7FDE_ADBE),
        "the loaded C .so is NOT the reference (-O0) build — got {c:?}. \
         An -O3 build returns (0x7fc01234, 0x7fc01234) here. Rebuild with:\n  \
         cd c_src/build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .\n  \
         (loaded from {})",
        api().c_path.display()
    );
    // And the Rust must agree with that reference.
    diff("reference build guard", p1, p2, p3, p);
}

// ---------------------------------------------------------------------------
// Geometry helpers
// ---------------------------------------------------------------------------

/// Build `p = p1 + u*(p3-p1) + v*(p2-p1)` — the inverse of `to_barycentric`.
fn point_from_bary(p1: Vec2, p2: Vec2, p3: Vec2, u: f32, v: f32) -> Vec2 {
    Vec2::new(
        p1.x + u * (p3.x - p1.x) + v * (p2.x - p1.x),
        p1.y + u * (p3.y - p1.y) + v * (p2.y - p1.y),
    )
}

/// A random triangle whose vertices are far enough apart to be well conditioned.
fn nondegenerate_triangle(rng: &mut Rng, lo: f32, hi: f32) -> (Vec2, Vec2, Vec2) {
    loop {
        let p1 = rng.vec2(lo, hi);
        let p2 = rng.vec2(lo, hi);
        let p3 = rng.vec2(lo, hi);
        // Twice the signed area; reject near-degenerate configurations so the
        // row really tests the well-conditioned path (degenerate ones have
        // their own rows: C19, C28, C29).
        let area = (p2.x - p1.x) * (p3.y - p1.y) - (p3.x - p1.x) * (p2.y - p1.y);
        let scale = (hi - lo).abs().max(1.0);
        if area.abs() > 0.01 * scale * scale {
            return (p1, p2, p3);
        }
    }
}

// ---------------------------------------------------------------------------
// C1 / C2 — interior and exterior query points
// ---------------------------------------------------------------------------

#[test]
fn cfg_c1_interior_unit() {
    let mut rng = Rng::seeded();
    let mut saw_finite = false;
    for _ in 0..iters(20_000) {
        let (p1, p2, p3) = nondegenerate_triangle(&mut rng, -1.0, 1.0);
        // Uniformly random barycentric point strictly inside.
        let mut u = rng.uniform(0.0, 1.0);
        let mut v = rng.uniform(0.0, 1.0);
        if u + v > 1.0 {
            u = 1.0 - u;
            v = 1.0 - v;
        }
        let p = point_from_bary(p1, p2, p3, u, v);
        diff("C1 interior", p1, p2, p3, p);
        let r = c_call(p1, p2, p3, p);
        if r.x.is_finite() && r.y.is_finite() {
            saw_finite = true;
        }
    }
    assert!(saw_finite, "C1 must exercise finite results, not only NaNs");
}

#[test]
fn cfg_c2_exterior() {
    let mut rng = Rng::seeded();
    for _ in 0..iters(20_000) {
        let (p1, p2, p3) = nondegenerate_triangle(&mut rng, -1.0, 1.0);
        // Deliberately outside: u,v in [-4,4] with u+v>1 or either negative.
        let u = rng.uniform(-4.0, 4.0);
        let v = rng.uniform(-4.0, 4.0);
        let p = point_from_bary(p1, p2, p3, u, v);
        diff("C2 exterior", p1, p2, p3, p);
    }
}

// ---------------------------------------------------------------------------
// C3 / C4 / C5 — query point exactly on a vertex
// ---------------------------------------------------------------------------

#[test]
fn cfg_c3_p_on_p1() {
    let mut rng = Rng::seeded();
    for _ in 0..iters(10_000) {
        let (p1, p2, p3) = nondegenerate_triangle(&mut rng, -1e3, 1e3);
        diff("C3 p==p1", p1, p2, p3, p1);
        // Also with the ±0 variants of the coincident point.
        for (sx, sy) in [(1.0f32, 1.0f32), (-1.0, 1.0), (1.0, -1.0), (-1.0, -1.0)] {
            let q = Vec2::new(p1.x * sx * sx, p1.y * sy * sy); // value-preserving
            diff("C3 p==p1 (copy)", p1, p2, p3, q);
        }
    }
}

#[test]
fn cfg_c4_p_on_p2() {
    let mut rng = Rng::seeded();
    for _ in 0..iters(10_000) {
        let (p1, p2, p3) = nondegenerate_triangle(&mut rng, -1e3, 1e3);
        diff("C4 p==p2", p1, p2, p3, p2);
    }
}

#[test]
fn cfg_c5_p_on_p3() {
    let mut rng = Rng::seeded();
    for _ in 0..iters(10_000) {
        let (p1, p2, p3) = nondegenerate_triangle(&mut rng, -1e3, 1e3);
        diff("C5 p==p3", p1, p2, p3, p3);
    }
}

// ---------------------------------------------------------------------------
// C6 — query point on the edges
// ---------------------------------------------------------------------------

#[test]
fn cfg_c6_p_on_edges() {
    let mut rng = Rng::seeded();
    for _ in 0..iters(10_000) {
        let (p1, p2, p3) = nondegenerate_triangle(&mut rng, -1.0, 1.0);
        let mid = |a: Vec2, b: Vec2| Vec2::new((a.x + b.x) * 0.5, (a.y + b.y) * 0.5);
        diff("C6 mid(p1,p2)", p1, p2, p3, mid(p1, p2));
        diff("C6 mid(p1,p3)", p1, p2, p3, mid(p1, p3));
        diff("C6 mid(p2,p3)", p1, p2, p3, mid(p2, p3));
        // A random point along each edge.
        let t = rng.uniform(0.0, 1.0);
        let lerp = |a: Vec2, b: Vec2| {
            Vec2::new(a.x + t * (b.x - a.x), a.y + t * (b.y - a.y))
        };
        diff("C6 lerp(p1,p2)", p1, p2, p3, lerp(p1, p2));
        diff("C6 lerp(p1,p3)", p1, p2, p3, lerp(p1, p3));
        diff("C6 lerp(p2,p3)", p1, p2, p3, lerp(p2, p3));
    }
}

// ---------------------------------------------------------------------------
// C7 / C8 — orthogonal edges (dot01 == 0)
// ---------------------------------------------------------------------------

#[test]
fn cfg_c7_unit_triangle_orthogonal() {
    let p1 = Vec2::new(0.0, 0.0);
    let p2 = Vec2::new(1.0, 0.0);
    let p3 = Vec2::new(0.0, 1.0);
    let mut rng = Rng::seeded();
    // Exact, hand-picked points first.
    for &(x, y) in &[
        (0.0f32, 0.0f32),
        (1.0, 0.0),
        (0.0, 1.0),
        (0.5, 0.5),
        (0.25, 0.25),
        (1.0, 1.0),
        (-1.0, -1.0),
        (2.0, -3.0),
        (1e-30, 1e30),
        (f32::MAX, f32::MIN),
    ] {
        diff("C7 unit fixed", p1, p2, p3, Vec2::new(x, y));
    }
    for _ in 0..iters(20_000) {
        let p = Vec2::new(rng.uniform(-2.0, 2.0), rng.uniform(-2.0, 2.0));
        diff("C7 unit random", p1, p2, p3, p);
        // Sanity: for the unit triangle u==p.y and v==p.x exactly.
        let r = c_call(p1, p2, p3, p);
        assert_eq!(
            (r.x.to_bits(), r.y.to_bits()),
            (p.y.to_bits(), p.x.to_bits()),
            "unit-triangle identity broken for {p:?} -> {r:?}"
        );
    }
}

#[test]
fn cfg_c8_right_triangle() {
    let mut rng = Rng::seeded();
    for _ in 0..iters(20_000) {
        let ox = rng.uniform(-100.0, 100.0);
        let oy = rng.uniform(-100.0, 100.0);
        let a = rng.uniform(-50.0, 50.0);
        let b = rng.uniform(-50.0, 50.0);
        if a == 0.0 || b == 0.0 {
            continue;
        }
        let p1 = Vec2::new(ox, oy);
        let p2 = Vec2::new(ox + a, oy); // v1 along +x
        let p3 = Vec2::new(ox, oy + b); // v0 along +y  => dot01 == 0
        let p = Vec2::new(rng.uniform(-200.0, 200.0), rng.uniform(-200.0, 200.0));
        diff("C8 right triangle", p1, p2, p3, p);
        // Rotate the roles so the zero dot lands in the other slots too.
        diff("C8 right triangle swapped", p1, p3, p2, p);
    }
}

// ---------------------------------------------------------------------------
// C9 / C10 — oblique triangles and both windings
// ---------------------------------------------------------------------------

#[test]
fn cfg_c9_oblique() {
    let mut rng = Rng::seeded();
    for _ in 0..iters(20_000) {
        let (p1, p2, p3) = nondegenerate_triangle(&mut rng, -30.0, 30.0);
        let v0 = (p3.x - p1.x, p3.y - p1.y);
        let v1 = (p2.x - p1.x, p2.y - p1.y);
        if v0.0 * v1.0 + v0.1 * v1.1 == 0.0 {
            continue; // orthogonal case is C7/C8
        }
        let p = rng.vec2(-60.0, 60.0);
        diff("C9 oblique", p1, p2, p3, p);
    }
}

#[test]
fn cfg_c10_both_windings() {
    let mut rng = Rng::seeded();
    for _ in 0..iters(20_000) {
        let (p1, p2, p3) = nondegenerate_triangle(&mut rng, -10.0, 10.0);
        let p = rng.vec2(-20.0, 20.0);
        diff("C10 ccw", p1, p2, p3, p);
        diff("C10 cw", p1, p3, p2, p); // reversed winding
        // All six vertex orderings.
        diff("C10 perm 213", p2, p1, p3, p);
        diff("C10 perm 231", p2, p3, p1, p);
        diff("C10 perm 312", p3, p1, p2, p);
        diff("C10 perm 321", p3, p2, p1, p);
    }
}

// ---------------------------------------------------------------------------
// C11 / C12 — sign class and exact integral values
// ---------------------------------------------------------------------------

#[test]
fn cfg_c11_all_negative() {
    let mut rng = Rng::seeded();
    for _ in 0..iters(20_000) {
        let mut s = [0.0f32; 8];
        for v in s.iter_mut() {
            *v = rng.uniform(-1e3, -1.0);
        }
        diff_slots("C11 all negative", s);
    }
    // All-positive counterpart.
    for _ in 0..iters(20_000) {
        let mut s = [0.0f32; 8];
        for v in s.iter_mut() {
            *v = rng.uniform(1.0, 1e3);
        }
        diff_slots("C11 all positive", s);
    }
}

#[test]
fn cfg_c12_integral() {
    let mut rng = Rng::seeded();
    for _ in 0..iters(30_000) {
        let mut s = [0.0f32; 8];
        for v in s.iter_mut() {
            *v = (rng.below(129) as i32 - 64) as f32;
        }
        diff_slots("C12 integral", s);
    }
}

// ---------------------------------------------------------------------------
// C13 / C14 / C15 / C16 / C17 / C18 — magnitude classes
// ---------------------------------------------------------------------------

#[test]
fn cfg_c13_large_magnitude() {
    let mut rng = Rng::seeded();
    for _ in 0..iters(30_000) {
        let mut s = [0.0f32; 8];
        for v in s.iter_mut() {
            *v = rng.uniform(-1e18, 1e18);
        }
        diff_slots("C13 large", s);
    }
}

#[test]
fn cfg_c14_overflow_magnitude() {
    let mut rng = Rng::seeded();
    let mut saw_nan = false;
    for _ in 0..iters(30_000) {
        let mut s = [0.0f32; 8];
        for v in s.iter_mut() {
            *v = rng.uniform(-1e20, 1e20);
        }
        diff_slots("C14 overflow", s);
        let (p1, p2, p3, p) = from_slots(s);
        let r = c_call(p1, p2, p3, p);
        if r.x.is_nan() || r.y.is_nan() {
            saw_nan = true;
        }
    }
    assert!(
        saw_nan,
        "C14 should reach the inf-inf NaN path at least once"
    );
}

#[test]
fn cfg_c15_tiny_magnitude() {
    let mut rng = Rng::seeded();
    for _ in 0..iters(30_000) {
        let mut s = [0.0f32; 8];
        for v in s.iter_mut() {
            *v = rng.uniform(-1e-20, 1e-20);
        }
        diff_slots("C15 tiny", s);
    }
}

#[test]
fn cfg_c16_subnormal() {
    let mut rng = Rng::seeded();
    for _ in 0..iters(30_000) {
        let mut s = [0.0f32; 8];
        for v in s.iter_mut() {
            *v = rng.subnormal();
        }
        diff_slots("C16 subnormal", s);
    }
    // Extremes.
    diff_slots("C16 min subnormal", [SUBNORMAL_MIN; 8]);
    diff_slots("C16 max subnormal", [SUBNORMAL_MAX; 8]);
    diff_slots(
        "C16 mixed subnormal",
        [
            SUBNORMAL_MIN,
            -SUBNORMAL_MIN,
            SUBNORMAL_MAX,
            -SUBNORMAL_MAX,
            f32::MIN_POSITIVE,
            -f32::MIN_POSITIVE,
            SUBNORMAL_MIN,
            SUBNORMAL_MAX,
        ],
    );
}

#[test]
fn cfg_c17_mixed_binades() {
    let mut rng = Rng::seeded();
    for _ in 0..iters(50_000) {
        let mut s = [0.0f32; 8];
        for v in s.iter_mut() {
            *v = rng.binade(-45, 45);
        }
        diff_slots("C17 mixed binades", s);
    }
}

#[test]
fn cfg_c18_log_uniform_finite() {
    let mut rng = Rng::seeded();
    for _ in 0..iters(50_000) {
        let mut s = [0.0f32; 8];
        for v in s.iter_mut() {
            *v = rng.finite();
        }
        diff_slots("C18 log-uniform finite", s);
    }
}

// ---------------------------------------------------------------------------
// C19 — near-collinear (catastrophically cancelled denominator)
// ---------------------------------------------------------------------------

#[test]
fn cfg_c19_near_collinear() {
    let mut rng = Rng::seeded();
    for _ in 0..iters(30_000) {
        let p1 = rng.vec2(-10.0, 10.0);
        let p2 = rng.vec2(-10.0, 10.0);
        let t = rng.uniform(-3.0, 3.0);
        // eps spans "exactly collinear" through "barely off the line".
        let eps = rng.binade(-40, -1);
        let p3 = Vec2::new(
            p1.x + t * (p2.x - p1.x) + eps,
            p1.y + t * (p2.y - p1.y) - eps,
        );
        let p = rng.vec2(-10.0, 10.0);
        diff("C19 near collinear", p1, p2, p3, p);
    }
}

// ---------------------------------------------------------------------------
// C20 — exhaustive signed zeros (all 2^8 sign combinations)
// ---------------------------------------------------------------------------

#[test]
fn cfg_c20_signed_zero_exhaustive() {
    for mask in 0u32..256 {
        let mut s = [0.0f32; 8];
        for (i, v) in s.iter_mut().enumerate() {
            *v = if mask & (1 << i) != 0 { -0.0 } else { 0.0 };
        }
        diff_slots("C20 signed zeros", s);
    }
    // Signed zeros mixed with ordinary values in every slot.
    let mut rng = Rng::seeded();
    for _ in 0..iters(20_000) {
        let mut s = random_slots(&mut rng, -4.0, 4.0);
        let n = 1 + rng.below(4) as usize;
        for _ in 0..n {
            let slot = rng.below(8) as usize;
            s[slot] = if rng.bool() { -0.0 } else { 0.0 };
        }
        diff_slots("C20 zeros mixed", s);
    }
}

// ---------------------------------------------------------------------------
// C21 / C22 / C23 / C24 — one special value per slot
// ---------------------------------------------------------------------------

fn special_in_each_slot(case: &str, specials: &[f32], n_iters: u32) {
    let mut rng = Rng::seeded();
    for slot in 0..8usize {
        for &sp in specials {
            // Exact, fixed surroundings first (reproducible minimal case).
            let mut base = [1.0f32, 2.0, 3.0, 5.0, 7.0, 11.0, 13.0, 17.0];
            base[slot] = sp;
            diff_slots(&format!("{case} fixed slot={}", SLOT_NAMES[slot]), base);
            // Then randomized surroundings.
            for _ in 0..n_iters {
                let mut s = random_slots(&mut rng, -100.0, 100.0);
                s[slot] = sp;
                diff_slots(&format!("{case} slot={}", SLOT_NAMES[slot]), s);
            }
        }
    }
}

#[test]
fn cfg_c21_inf_single_slot() {
    special_in_each_slot(
        "C21 inf",
        &[f32::INFINITY, f32::NEG_INFINITY],
        iters(2_000),
    );
}

#[test]
fn cfg_c22_qnan_single_slot() {
    special_in_each_slot(
        "C22 qnan",
        &[QNAN, NAN_PAYLOAD_A, NAN_PAYLOAD_B],
        iters(2_000),
    );
}

#[test]
fn cfg_c23_snan_single_slot() {
    special_in_each_slot("C23 snan", &[SNAN, SNAN_NEG], iters(2_000));
}

#[test]
fn cfg_c24_negative_nan_single_slot() {
    special_in_each_slot("C24 -nan", &[QNAN_NEG, NAN_ALL_ONES], iters(2_000));
}

// ---------------------------------------------------------------------------
// C25 — two NaNs meeting inside one SSE instruction
// ---------------------------------------------------------------------------

#[test]
fn cfg_c25_two_nan_operands() {
    let nans = [QNAN, QNAN_NEG, SNAN, SNAN_NEG, NAN_ALL_ONES, NAN_PAYLOAD_A, NAN_PAYLOAD_B];
    // Slot pairs that are the two operands of a single subss in lm_sub2:
    //   v0.x = p3.x - p1.x  -> slots 4,0
    //   v0.y = p3.y - p1.y  -> slots 5,1
    //   v1.x = p2.x - p1.x  -> slots 2,0
    //   v1.y = p2.y - p1.y  -> slots 3,1
    //   v2.x = p.x  - p1.x  -> slots 6,0
    //   v2.y = p.y  - p1.y  -> slots 7,1
    let pairs = [(4, 0), (5, 1), (2, 0), (3, 1), (6, 0), (7, 1)];
    let base = [1.0f32, 2.0, 3.0, 5.0, 7.0, 11.0, 13.0, 17.0];
    for (a, b) in pairs {
        for &na in &nans {
            for &nb in &nans {
                let mut s = base;
                s[a] = na;
                s[b] = nb;
                diff_slots(
                    &format!("C25 two-NaN {}={:#010x} {}={:#010x}",
                        SLOT_NAMES[a], na.to_bits(), SLOT_NAMES[b], nb.to_bits()),
                    s,
                );
            }
        }
    }
    // Whole-vector NaN collisions (all 8 slots from the NaN pool), randomized.
    let mut rng = Rng::seeded();
    for _ in 0..iters(50_000) {
        let mut s = [0.0f32; 8];
        for v in s.iter_mut() {
            *v = nans[rng.below(nans.len() as u32) as usize];
        }
        diff_slots("C25 all-NaN vector", s);
    }
}

// ---------------------------------------------------------------------------
// C26 / C27 — mixtures and fully random bit patterns
// ---------------------------------------------------------------------------

#[test]
fn cfg_c26_nan_inf_mixture() {
    let specials = [
        QNAN,
        QNAN_NEG,
        SNAN,
        SNAN_NEG,
        NAN_ALL_ONES,
        NAN_PAYLOAD_A,
        f32::INFINITY,
        f32::NEG_INFINITY,
    ];
    let mut rng = Rng::seeded();
    for _ in 0..iters(100_000) {
        let mut s = random_slots(&mut rng, -1e3, 1e3);
        let n = 1 + rng.below(5) as usize;
        for _ in 0..n {
            let slot = rng.below(8) as usize;
            s[slot] = specials[rng.below(specials.len() as u32) as usize];
        }
        diff_slots("C26 nan/inf mixture", s);
    }
}

#[test]
fn cfg_c27_random_bit_patterns() {
    let mut rng = Rng::seeded();
    for _ in 0..iters(300_000) {
        let mut s = [0.0f32; 8];
        for v in s.iter_mut() {
            *v = rng.any_bits();
        }
        diff_slots("C27 random bits", s);
    }
}

/// Exhaustive sweep over a 6-value pool in all 8 slots: 6^8 = 1_679_616 cases,
/// covering every pairing of `±0`, finite, `inf`, QNaN and SNaN operands.
#[test]
fn cfg_c27b_pool_exhaustive_positive() {
    let pool = [0.0f32, 1.0, f32::INFINITY, QNAN, SNAN, 3.0];
    exhaustive_pool("C27b pool+", &pool);
}

/// Second exhaustive sweep with the negative / subnormal counterparts.
#[test]
fn cfg_c27c_pool_exhaustive_negative() {
    let pool = [
        -0.0f32,
        -1.0,
        f32::NEG_INFINITY,
        QNAN_NEG,
        SUBNORMAL_MIN,
        f32::MAX,
    ];
    exhaustive_pool("C27c pool-", &pool);
}

fn exhaustive_pool(case: &str, pool: &[f32]) {
    let n = pool.len();
    let total = n.pow(8);
    let mut idx = [0usize; 8];
    for _ in 0..total {
        let mut s = [0.0f32; 8];
        for (k, v) in s.iter_mut().enumerate() {
            *v = pool[idx[k]];
        }
        diff_slots(case, s);
        // odometer increment
        for k in 0..8 {
            idx[k] += 1;
            if idx[k] < n {
                break;
            }
            idx[k] = 0;
        }
    }
}

/// Randomized draws from the full interesting-value pool (30 values, all
/// encoding classes) in every slot.
#[test]
fn cfg_c27d_pool_random() {
    let mut rng = Rng::seeded();
    for _ in 0..iters(300_000) {
        let mut s = [0.0f32; 8];
        for v in s.iter_mut() {
            *v = POOL[rng.below(POOL.len() as u32) as usize];
        }
        diff_slots("C27d pool random", s);
    }
}

// ---------------------------------------------------------------------------
// C28 / C29 — duplicate vertices and exact collinearity
// ---------------------------------------------------------------------------

#[test]
fn cfg_c28_duplicate_vertices() {
    let mut rng = Rng::seeded();
    for _ in 0..iters(20_000) {
        let a = rng.vec2(-100.0, 100.0);
        let b = rng.vec2(-100.0, 100.0);
        let p = rng.vec2(-100.0, 100.0);
        diff("C28 p1==p2", a, a, b, p);
        diff("C28 p1==p3", a, b, a, p);
        diff("C28 p2==p3", a, b, b, p);
        diff("C28 all equal", a, a, a, p);
        diff("C28 all equal, p too", a, a, a, a);
    }
    // Extremes of the duplicate-vertex family.
    for &v in &[0.0f32, -0.0, 1.0, -1.0, f32::MAX, f32::MIN_POSITIVE, SUBNORMAL_MIN] {
        let a = Vec2::new(v, v);
        diff("C28 fixed all equal", a, a, a, Vec2::new(1.0, 2.0));
        diff("C28 fixed p1==p2", a, a, Vec2::new(3.0, 4.0), Vec2::new(1.0, 2.0));
    }
}

#[test]
fn cfg_c29_exact_collinear() {
    let mut rng = Rng::seeded();
    let mut saw_nan = false;
    for _ in 0..iters(20_000) {
        let p1 = Vec2::new(
            (rng.below(65) as i32 - 32) as f32,
            (rng.below(65) as i32 - 32) as f32,
        );
        let d = Vec2::new(
            (rng.below(17) as i32 - 8) as f32,
            (rng.below(17) as i32 - 8) as f32,
        );
        // Integral multiples keep the collinearity exact in binary32.
        let k2 = (rng.below(9) as i32 - 4) as f32;
        let k3 = (rng.below(9) as i32 - 4) as f32;
        let p2 = Vec2::new(p1.x + k2 * d.x, p1.y + k2 * d.y);
        let p3 = Vec2::new(p1.x + k3 * d.x, p1.y + k3 * d.y);
        let p = rng.vec2(-64.0, 64.0);
        diff("C29 exact collinear", p1, p2, p3, p);
        let r = c_call(p1, p2, p3, p);
        if r.x.is_nan() || r.y.is_nan() {
            saw_nan = true;
        }
    }
    // Exact collinearity forces BOTH the denominator and both numerators to
    // zero, so the C returns the indefinite QNaN rather than ±inf (see
    // ERRORS.md E5). True ±inf needs mixed binades — covered by E19.
    assert!(
        saw_nan,
        "C29 should reach the 0/0 -> indefinite-QNaN path at least once"
    );
    // Textbook collinear cases.
    diff(
        "C29 (0,0),(1,1),(2,2)",
        Vec2::new(0.0, 0.0),
        Vec2::new(1.0, 1.0),
        Vec2::new(2.0, 2.0),
        Vec2::new(3.0, 7.0),
    );
    diff(
        "C29 x axis",
        Vec2::new(0.0, 0.0),
        Vec2::new(1.0, 0.0),
        Vec2::new(2.0, 0.0),
        Vec2::new(0.5, 0.25),
    );
    diff(
        "C29 y axis",
        Vec2::new(0.0, 0.0),
        Vec2::new(0.0, 1.0),
        Vec2::new(0.0, 2.0),
        Vec2::new(0.5, 0.25),
    );
}

// ---------------------------------------------------------------------------
// C30 / C31 — ambient MXCSR (rounding mode, FTZ/DAZ)
// ---------------------------------------------------------------------------

#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
#[test]
fn cfg_c30_rounding_modes() {
    use harness::mxcsr;
    for (name, mode) in [
        ("nearest", mxcsr::ROUND_NEAREST),
        ("down", mxcsr::ROUND_DOWN),
        ("up", mxcsr::ROUND_UP),
        ("zero", mxcsr::ROUND_ZERO),
    ] {
        mxcsr::with(mxcsr::ROUND_MASK, mode, || {
            let mut rng = Rng::seeded();
            for _ in 0..iters(20_000) {
                let mut s = [0.0f32; 8];
                for v in s.iter_mut() {
                    *v = rng.binade(-20, 20);
                }
                diff_slots(&format!("C30 rounding={name}"), s);
            }
            // Also the pool, so specials meet the non-default mode.
            for _ in 0..iters(20_000) {
                let mut s = [0.0f32; 8];
                for v in s.iter_mut() {
                    *v = POOL[rng.below(POOL.len() as u32) as usize];
                }
                diff_slots(&format!("C30 pool rounding={name}"), s);
            }
        });
    }
}

#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
#[test]
fn cfg_c31_ftz_daz() {
    use harness::mxcsr;
    for (name, bits) in [
        ("ftz", mxcsr::FTZ),
        ("daz", mxcsr::DAZ),
        ("ftz+daz", mxcsr::FTZ | mxcsr::DAZ),
    ] {
        mxcsr::with(mxcsr::FTZ | mxcsr::DAZ, bits, || {
            let mut rng = Rng::seeded();
            for _ in 0..iters(20_000) {
                let mut s = [0.0f32; 8];
                for v in s.iter_mut() {
                    *v = if rng.bool() {
                        rng.subnormal()
                    } else {
                        rng.binade(-40, 10)
                    };
                }
                diff_slots(&format!("C31 {name}"), s);
            }
        });
    }
}

// ---------------------------------------------------------------------------
// C32 — statelessness: repeats, interleaving, threads
// ---------------------------------------------------------------------------

#[test]
fn cfg_c32_stateless_interleaved() {
    let mut rng = Rng::seeded();
    let cases: Vec<[f32; 8]> = (0..200)
        .map(|i| {
            let mut s = [0.0f32; 8];
            for v in s.iter_mut() {
                *v = if i % 3 == 0 {
                    POOL[rng.below(POOL.len() as u32) as usize]
                } else {
                    rng.binade(-20, 20)
                };
            }
            s
        })
        .collect();

    // Same inputs, three passes, forwards / backwards / strided: any hidden
    // state in either library would show up as an order-dependent result.
    let mut first: Vec<((u32, u32), (u32, u32))> = Vec::new();
    for s in &cases {
        let (p1, p2, p3, p) = from_slots(*s);
        first.push((c_call(p1, p2, p3, p).bits(), rust_call(p1, p2, p3, p).bits()));
    }
    for pass in 0..3 {
        let order: Vec<usize> = match pass {
            0 => (0..cases.len()).collect(),
            1 => (0..cases.len()).rev().collect(),
            _ => (0..cases.len()).step_by(7).chain((0..cases.len()).skip(1).step_by(5)).collect(),
        };
        for i in order {
            let (p1, p2, p3, p) = from_slots(cases[i]);
            diff("C32 interleaved", p1, p2, p3, p);
            assert_eq!(
                (c_call(p1, p2, p3, p).bits(), rust_call(p1, p2, p3, p).bits()),
                first[i],
                "order-dependent result at case {i} (pass {pass})"
            );
        }
    }

    // Two threads hammering both libraries concurrently.
    let cases2 = cases.clone();
    let h = std::thread::spawn(move || {
        for s in cases2.iter().rev() {
            let (p1, p2, p3, p) = from_slots(*s);
            diff("C32 thread B", p1, p2, p3, p);
        }
    });
    for s in cases.iter() {
        let (p1, p2, p3, p) = from_slots(*s);
        diff("C32 thread A", p1, p2, p3, p);
    }
    h.join().expect("worker thread panicked");
}
