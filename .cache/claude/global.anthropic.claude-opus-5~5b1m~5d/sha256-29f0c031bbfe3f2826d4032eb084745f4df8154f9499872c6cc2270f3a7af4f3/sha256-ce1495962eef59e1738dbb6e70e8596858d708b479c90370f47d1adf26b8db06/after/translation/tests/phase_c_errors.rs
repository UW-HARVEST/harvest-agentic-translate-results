//! Phase C — error-path differential tests, one test per row of `ERRORS.md`.
//!
//! Each test constructs the exact invalid input/condition, calls BOTH the C
//! `.so` and the Rust `.so`, and asserts they return the SAME sentinel (not
//! merely "both failed somehow"): `collided`'s only rejection value is `0`, and
//! row 12 pins down that `0` really came from the `default:` arm.

#![allow(non_snake_case)]

mod common;

use common::*;
use std::ffi::{c_int, c_void};

fn cp(x: &C2Circle) -> *const c_void {
    x as *const C2Circle as *const c_void
}
fn bp(x: &C2AABB) -> *const c_void {
    x as *const C2AABB as *const c_void
}

/// Tag values that match no enumerator (the enum only has 0 and 1).
const INVALID_TAGS: [C2_TYPE; 10] = [
    2,           // one past the last enumerator
    3,
    4,
    255,
    256,
    0x0001_0000,
    0x7fff_ffff, // INT_MAX
    0x8000_0000, // INT_MIN bit pattern (negative if the enum is signed)
    0xdead_beef,
    0xffff_ffff, // -1
];
const VALID_TAGS: [C2_TYPE; 2] = [C2_TYPE_CIRCLE, C2_TYPE_AABB];

/// Operands that collide under *every* valid tag pair, so a `0` result can only
/// mean "the `default:` arm was taken".
fn always_hit() -> (C2Circle, C2AABB) {
    (circle(0.0, 0.0, 10.0), aabb(-10.0, -10.0, 10.0, 10.0))
}

// ===========================================================================
// Row 1 — typeA matches no enumerator (outer default, lib.c:95)
// ===========================================================================

#[test]
fn row01_typeA_out_of_range() {
    let (c, r) = both();
    let (circ, bx) = always_hit();
    let mut rng = Rng::new(0x0101);
    for &bad in &INVALID_TAGS {
        for &tb in VALID_TAGS.iter().chain(INVALID_TAGS.iter()) {
            for ptr_pair in [(cp(&circ), cp(&circ)), (bp(&bx), bp(&bx)), (cp(&circ), bp(&bx))] {
                let ctx = format!("typeA=0x{bad:08x} typeB=0x{tb:08x}");
                let cv = unsafe { (c.collided)(ptr_pair.0, bad, ptr_pair.1, tb) };
                let rv = unsafe { (r.collided)(ptr_pair.0, bad, ptr_pair.1, tb) };
                eq_int("row01", &ctx, cv, rv);
                assert_eq!(cv, 0, "[row01] C should reject with 0: {ctx}");
            }
        }
    }
    // Randomized invalid tags, with randomized (wild) operands.
    for i in 0..ITERS {
        let bad = loop {
            let t = rng.next_u32();
            if t > 1 {
                break t;
            }
        };
        let ca = rng.c_wild();
        let ba = rng.b_wild();
        let tb = rng.next_u32();
        let ctx = format!("rand {i}: typeA=0x{bad:08x} typeB=0x{tb:08x}");
        let cv = unsafe { (c.collided)(cp(&ca), bad, bp(&ba), tb) };
        let rv = unsafe { (r.collided)(cp(&ca), bad, bp(&ba), tb) };
        eq_int("row01", &ctx, cv, rv);
        assert_eq!(cv, 0, "[row01] {ctx}");
    }
}

// ===========================================================================
// Row 2 — typeA == CIRCLE, typeB invalid (first inner default, lib.c:81)
// ===========================================================================

#[test]
fn row02_circle_typeB_out_of_range() {
    let (c, r) = both();
    let (circ, bx) = always_hit();
    for &bad in &INVALID_TAGS {
        for (k, p) in [cp(&circ), bp(&bx)].iter().enumerate() {
            let ctx = format!("typeA=CIRCLE typeB=0x{bad:08x} bptr={k}");
            let cv = unsafe { (c.collided)(cp(&circ), C2_TYPE_CIRCLE, *p, bad) };
            let rv = unsafe { (r.collided)(cp(&circ), C2_TYPE_CIRCLE, *p, bad) };
            eq_int("row02", &ctx, cv, rv);
            assert_eq!(cv, 0, "[row02] C should reject with 0: {ctx}");
        }
    }
    let mut rng = Rng::new(0x0202);
    for i in 0..ITERS {
        let bad = loop {
            let t = rng.next_u32();
            if t > 1 {
                break t;
            }
        };
        let ca = rng.c_wild();
        let cb = rng.c_wild();
        let ctx = format!("rand {i}: typeA=CIRCLE typeB=0x{bad:08x}");
        let cv = unsafe { (c.collided)(cp(&ca), C2_TYPE_CIRCLE, cp(&cb), bad) };
        let rv = unsafe { (r.collided)(cp(&ca), C2_TYPE_CIRCLE, cp(&cb), bad) };
        eq_int("row02", &ctx, cv, rv);
        assert_eq!(cv, 0, "[row02] {ctx}");
    }
}

// ===========================================================================
// Row 3 — typeA == AABB, typeB invalid (second inner default, lib.c:91)
// ===========================================================================

#[test]
fn row03_aabb_typeB_out_of_range() {
    let (c, r) = both();
    let (circ, bx) = always_hit();
    for &bad in &INVALID_TAGS {
        for (k, p) in [cp(&circ), bp(&bx)].iter().enumerate() {
            let ctx = format!("typeA=AABB typeB=0x{bad:08x} bptr={k}");
            let cv = unsafe { (c.collided)(bp(&bx), C2_TYPE_AABB, *p, bad) };
            let rv = unsafe { (r.collided)(bp(&bx), C2_TYPE_AABB, *p, bad) };
            eq_int("row03", &ctx, cv, rv);
            assert_eq!(cv, 0, "[row03] C should reject with 0: {ctx}");
        }
    }
    let mut rng = Rng::new(0x0303);
    for i in 0..ITERS {
        let bad = loop {
            let t = rng.next_u32();
            if t > 1 {
                break t;
            }
        };
        let ba = rng.b_wild();
        let bb = rng.b_wild();
        let ctx = format!("rand {i}: typeA=AABB typeB=0x{bad:08x}");
        let cv = unsafe { (c.collided)(bp(&ba), C2_TYPE_AABB, bp(&bb), bad) };
        let rv = unsafe { (r.collided)(bp(&ba), C2_TYPE_AABB, bp(&bb), bad) };
        eq_int("row03", &ctx, cv, rv);
        assert_eq!(cv, 0, "[row03] {ctx}");
    }
}

// ===========================================================================
// Row 4 — the full enum-value matrix across the FFI boundary
// ===========================================================================

#[test]
fn row04_enum_value_matrix() {
    let (c, r) = both();
    let (circ, bx) = always_hit();
    // A 16-byte backing store lets either tag read its full object safely.
    let all: Vec<C2_TYPE> = VALID_TAGS.iter().chain(INVALID_TAGS.iter()).copied().collect();
    for &ta in &all {
        for &tb in &all {
            // Feed pointers that are valid for whichever interpretation the tag
            // picks: the box is 16 bytes, big enough for a 12-byte circle read.
            for (k, (pa, pb)) in [
                (cp(&circ), cp(&circ)),
                (bp(&bx), bp(&bx)),
                (cp(&circ), bp(&bx)),
                (bp(&bx), cp(&circ)),
            ]
            .iter()
            .enumerate()
            {
                let ctx = format!("typeA=0x{ta:08x} typeB=0x{tb:08x} ptrs={k}");
                let cv = unsafe { (c.collided)(*pa, ta, *pb, tb) };
                let rv = unsafe { (r.collided)(*pa, ta, *pb, tb) };
                eq_int("row04", &ctx, cv, rv);
                let both_valid = ta <= 1 && tb <= 1;
                if !both_valid {
                    assert_eq!(cv, 0, "[row04] invalid tag must yield 0: {ctx}");
                }
            }
        }
    }
}

// ===========================================================================
// Row 5 — exactly one past the last enumerator
// ===========================================================================

#[test]
fn row05_one_past_last_enumerator() {
    let (c, r) = both();
    let (circ, bx) = always_hit();
    const PAST: C2_TYPE = C2_TYPE_AABB + 1; // == 2
    let cases: [(*const c_void, C2_TYPE, *const c_void, C2_TYPE); 4] = [
        (cp(&circ), PAST, cp(&circ), C2_TYPE_CIRCLE),
        (cp(&circ), C2_TYPE_CIRCLE, cp(&circ), PAST),
        (bp(&bx), C2_TYPE_AABB, bp(&bx), PAST),
        (bp(&bx), PAST, bp(&bx), PAST),
    ];
    for (i, (pa, ta, pb, tb)) in cases.iter().enumerate() {
        let ctx = format!("case {i}: typeA=0x{ta:08x} typeB=0x{tb:08x}");
        let cv = unsafe { (c.collided)(*pa, *ta, *pb, *tb) };
        let rv = unsafe { (r.collided)(*pa, *ta, *pb, *tb) };
        eq_int("row05", &ctx, cv, rv);
        assert_eq!(cv, 0, "[row05] {ctx}");
    }
}

// ===========================================================================
// Row 6 — null pointers together with an invalid tag (safe: no dereference)
// ===========================================================================

#[test]
fn row06_null_pointers_with_invalid_tag() {
    let (c, r) = both();
    let (circ, bx) = always_hit();
    let null = std::ptr::null::<c_void>();
    for &bad in &INVALID_TAGS {
        // typeA invalid ⇒ outer default ⇒ B is never touched either.
        let combos: [(*const c_void, C2_TYPE, *const c_void, C2_TYPE); 6] = [
            (null, bad, null, C2_TYPE_CIRCLE),
            (null, bad, null, C2_TYPE_AABB),
            (null, bad, null, bad),
            (cp(&circ), C2_TYPE_CIRCLE, null, bad),
            (bp(&bx), C2_TYPE_AABB, null, bad),
            (null, bad, cp(&circ), C2_TYPE_CIRCLE),
        ];
        for (i, (pa, ta, pb, tb)) in combos.iter().enumerate() {
            let ctx = format!("combo {i}: typeA=0x{ta:08x} typeB=0x{tb:08x} (null operands)");
            let cv = unsafe { (c.collided)(*pa, *ta, *pb, *tb) };
            let rv = unsafe { (r.collided)(*pa, *ta, *pb, *tb) };
            eq_int("row06", &ctx, cv, rv);
            assert_eq!(cv, 0, "[row06] {ctx}");
        }
    }
}

// ===========================================================================
// Row 7 — null pointer with a VALID tag: the C dereferences unconditionally.
// Both libraries must fault the same way, so each is run in a child process and
// the termination signal is compared.
// ===========================================================================

const CHILD_LIB: &str = "PHASE_C_NULL_DEREF_LIB";
const CHILD_CASE: &str = "PHASE_C_NULL_DEREF_CASE";

/// Helper that actually performs the faulting call; it is a no-op unless the
/// parent asks for it through the environment.
#[test]
fn null_deref_child() {
    let Ok(which) = std::env::var(CHILD_LIB) else {
        return; // normal test run: nothing to do
    };
    let case: usize = std::env::var(CHILD_CASE).unwrap().parse().unwrap();
    let api = if which == "c" { c() } else { r() };
    let null = std::ptr::null::<c_void>();
    let circ = circle(0.0, 0.0, 1.0);
    let bx = aabb(-1.0, -1.0, 1.0, 1.0);
    let v: c_int = unsafe {
        match case {
            0 => (api.collided)(null, C2_TYPE_CIRCLE, null, C2_TYPE_CIRCLE),
            1 => (api.collided)(null, C2_TYPE_CIRCLE, null, C2_TYPE_AABB),
            2 => (api.collided)(null, C2_TYPE_AABB, null, C2_TYPE_CIRCLE),
            3 => (api.collided)(null, C2_TYPE_AABB, null, C2_TYPE_AABB),
            4 => (api.collided)(cp(&circ), C2_TYPE_CIRCLE, null, C2_TYPE_CIRCLE),
            5 => (api.collided)(null, C2_TYPE_CIRCLE, cp(&circ), C2_TYPE_CIRCLE),
            6 => (api.collided)(bp(&bx), C2_TYPE_AABB, null, C2_TYPE_AABB),
            _ => (api.collided)(null, C2_TYPE_AABB, bp(&bx), C2_TYPE_AABB),
        }
    };
    // Reached only if the null read did not fault; report it so the parent can
    // compare "survived with value v" between the two libraries.
    println!("SURVIVED {v}");
    std::process::exit(0);
}

#[cfg(unix)]
fn run_null_deref_child(which: &str, case: usize) -> String {
    use std::os::unix::process::ExitStatusExt;
    let out = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "null_deref_child", "--nocapture", "--test-threads=1"])
        .env(CHILD_LIB, which)
        .env(CHILD_CASE, case.to_string())
        .output()
        .expect("spawning the child test binary");
    let stdout = String::from_utf8_lossy(&out.stdout);
    if let Some(sig) = out.status.signal() {
        format!("signal {sig}")
    } else if let Some(line) = stdout.lines().find(|l| l.starts_with("SURVIVED")) {
        line.to_string()
    } else {
        format!("exit {:?}", out.status.code())
    }
}

#[cfg(unix)]
#[test]
fn row07_null_pointer_with_valid_tag_faults_identically() {
    for case in 0..8 {
        let c_outcome = run_null_deref_child("c", case);
        let r_outcome = run_null_deref_child("rust", case);
        assert_eq!(
            c_outcome, r_outcome,
            "[row07] case {case}: C outcome {c_outcome:?} != Rust outcome {r_outcome:?}"
        );
        // The C has no null check, so the only correct outcome is a fault.
        assert!(
            c_outcome.starts_with("signal"),
            "[row07] case {case}: expected a fault, got {c_outcome:?}"
        );
    }
}

// ===========================================================================
// Row 8 — object smaller than the tag claims (the C reads past its end)
// ===========================================================================

#[test]
fn row08_undersized_object_reads_same_bytes() {
    let (c, r) = both();
    let mut rng = Rng::new(0x0808);
    for i in 0..ITERS {
        // 32-byte backing store, so a 16-byte AABB read of an 8-byte c2v or a
        // 12-byte circle stays inside allocated memory (as it must for the
        // comparison to be meaningful) while still reading "past the object".
        let mut store = [0u8; 32];
        for b in store.iter_mut() {
            *b = rng.next_u32() as u8;
        }
        // An 8-byte c2v at offset 0, tagged as a 16-byte AABB.
        let small = rng.v_small();
        store[0..8].copy_from_slice(&unsafe { std::mem::transmute::<C2v, [u8; 8]>(small) });
        // A 12-byte circle at offset 16, tagged as a 16-byte AABB.
        let circ = rng.c_small();
        store[16..28].copy_from_slice(&unsafe { std::mem::transmute::<C2Circle, [u8; 12]>(circ) });
        let p0 = store.as_ptr() as *const c_void;
        let p1 = unsafe { store.as_ptr().add(16) } as *const c_void;
        let ctx = format!("iter {i}: store={:02x?}", &store[..]);
        unsafe {
            eq_int(
                "row08/c2v-as-aabb",
                &ctx,
                (c.collided)(p0, C2_TYPE_AABB, p0, C2_TYPE_AABB),
                (r.collided)(p0, C2_TYPE_AABB, p0, C2_TYPE_AABB),
            );
            eq_int(
                "row08/circle-as-aabb",
                &ctx,
                (c.collided)(p1, C2_TYPE_AABB, p0, C2_TYPE_AABB),
                (r.collided)(p1, C2_TYPE_AABB, p0, C2_TYPE_AABB),
            );
            eq_int(
                "row08/c2v-as-circle",
                &ctx,
                (c.collided)(p0, C2_TYPE_CIRCLE, p1, C2_TYPE_CIRCLE),
                (r.collided)(p0, C2_TYPE_CIRCLE, p1, C2_TYPE_CIRCLE),
            );
        }
    }
}

// ===========================================================================
// Row 9 — type confusion: the tag disagrees with the real object
// ===========================================================================

#[test]
fn row09_type_confusion_agrees() {
    let (c, r) = both();
    let mut rng = Rng::new(0x0909);
    for i in 0..ITERS {
        let bx = rng.b_small(); // 16 bytes, safe to read as a 12-byte circle
        // A circle padded to 16 bytes, safe to read as an AABB.
        let circ = rng.c_small();
        let mut padded = [0u8; 16];
        padded[..12].copy_from_slice(&unsafe { std::mem::transmute::<C2Circle, [u8; 12]>(circ) });
        padded[12..].copy_from_slice(&rng.next_u32().to_le_bytes());
        let pbox = bp(&bx);
        let pcirc = padded.as_ptr() as *const c_void;
        let ctx = format!("iter {i}: box={} circle={}", show_b(bx), show_c(circ));
        unsafe {
            // box object claimed to be a circle
            eq_int(
                "row09/box-as-circle",
                &ctx,
                (c.collided)(pbox, C2_TYPE_CIRCLE, pbox, C2_TYPE_CIRCLE),
                (r.collided)(pbox, C2_TYPE_CIRCLE, pbox, C2_TYPE_CIRCLE),
            );
            // circle object claimed to be a box
            eq_int(
                "row09/circle-as-box",
                &ctx,
                (c.collided)(pcirc, C2_TYPE_AABB, pcirc, C2_TYPE_AABB),
                (r.collided)(pcirc, C2_TYPE_AABB, pcirc, C2_TYPE_AABB),
            );
            // mixed confusion
            eq_int(
                "row09/mixed",
                &ctx,
                (c.collided)(pbox, C2_TYPE_CIRCLE, pcirc, C2_TYPE_AABB),
                (r.collided)(pbox, C2_TYPE_CIRCLE, pcirc, C2_TYPE_AABB),
            );
            eq_int(
                "row09/mixed-swapped",
                &ctx,
                (c.collided)(pcirc, C2_TYPE_AABB, pbox, C2_TYPE_CIRCLE),
                (r.collided)(pcirc, C2_TYPE_AABB, pbox, C2_TYPE_CIRCLE),
            );
        }
    }
}

// ===========================================================================
// Row 10 — misaligned pointers
// ===========================================================================

#[test]
fn row10_misaligned_pointer() {
    let (c, r) = both();
    let mut rng = Rng::new(0x0A0A);
    for i in 0..ITERS {
        let mut buf = [0u8; 80];
        let circ = rng.c_wild();
        let bx = rng.b_wild();
        // Every offset 1..=15 (so 1-, 2- and 4-byte misalignments all occur).
        let off_c = 1 + (rng.below(15) as usize);
        let off_b = 32 + 1 + (rng.below(15) as usize);
        buf[off_c..off_c + 12]
            .copy_from_slice(&unsafe { std::mem::transmute::<C2Circle, [u8; 12]>(circ) });
        buf[off_b..off_b + 16]
            .copy_from_slice(&unsafe { std::mem::transmute::<C2AABB, [u8; 16]>(bx) });
        let pc = unsafe { buf.as_ptr().add(off_c) } as *const c_void;
        let pb = unsafe { buf.as_ptr().add(off_b) } as *const c_void;
        let ctx = format!("iter {i}: off_c={off_c} off_b={off_b} circ={} box={}", show_c(circ), show_b(bx));
        unsafe {
            eq_int("row10/CC", &ctx, (c.collided)(pc, C2_TYPE_CIRCLE, pc, C2_TYPE_CIRCLE), (r.collided)(pc, C2_TYPE_CIRCLE, pc, C2_TYPE_CIRCLE));
            eq_int("row10/CA", &ctx, (c.collided)(pc, C2_TYPE_CIRCLE, pb, C2_TYPE_AABB), (r.collided)(pc, C2_TYPE_CIRCLE, pb, C2_TYPE_AABB));
            eq_int("row10/AC", &ctx, (c.collided)(pb, C2_TYPE_AABB, pc, C2_TYPE_CIRCLE), (r.collided)(pb, C2_TYPE_AABB, pc, C2_TYPE_CIRCLE));
            eq_int("row10/AA", &ctx, (c.collided)(pb, C2_TYPE_AABB, pb, C2_TYPE_AABB), (r.collided)(pb, C2_TYPE_AABB, pb, C2_TYPE_AABB));
        }
    }
}

// ===========================================================================
// Row 11 — "out of range" floats are never rejected
// ===========================================================================

#[test]
fn row11_no_float_validation() {
    let (c, r) = both();
    let classes = [
        f32::NAN,
        f32::from_bits(0x7f80_0001), // sNaN
        f32::from_bits(0xffff_ffff), // -NaN, all payload bits set
        f32::INFINITY,
        f32::NEG_INFINITY,
        0.0,
        -0.0,
        f32::MIN_POSITIVE,
        f32::from_bits(0x0000_0001), // smallest subnormal
        f32::MAX,
        f32::MIN,
    ];
    for &val in &classes {
        // Poison every field of both operands with the same odd value.
        let circ = circle(val, val, val);
        let bx = C2AABB { min: v(val, val), max: v(val, val) };
        let ctx = format!("val 0x{:08x}", fb(val));
        unsafe {
            let cv = (c.collided)(cp(&circ), C2_TYPE_CIRCLE, cp(&circ), C2_TYPE_CIRCLE);
            eq_int("row11/CC", &ctx, cv, (r.collided)(cp(&circ), C2_TYPE_CIRCLE, cp(&circ), C2_TYPE_CIRCLE));
            assert_bool_like("row11/CC", &ctx, cv);
            let cv = (c.collided)(cp(&circ), C2_TYPE_CIRCLE, bp(&bx), C2_TYPE_AABB);
            eq_int("row11/CA", &ctx, cv, (r.collided)(cp(&circ), C2_TYPE_CIRCLE, bp(&bx), C2_TYPE_AABB));
            assert_bool_like("row11/CA", &ctx, cv);
            let cv = (c.collided)(bp(&bx), C2_TYPE_AABB, cp(&circ), C2_TYPE_CIRCLE);
            eq_int("row11/AC", &ctx, cv, (r.collided)(bp(&bx), C2_TYPE_AABB, cp(&circ), C2_TYPE_CIRCLE));
            assert_bool_like("row11/AC", &ctx, cv);
            let cv = (c.collided)(bp(&bx), C2_TYPE_AABB, bp(&bx), C2_TYPE_AABB);
            eq_int("row11/AA", &ctx, cv, (r.collided)(bp(&bx), C2_TYPE_AABB, bp(&bx), C2_TYPE_AABB));
            assert_bool_like("row11/AA", &ctx, cv);
        }
    }
    // Negative radii and inverted boxes are likewise accepted.
    let mut rng = Rng::new(0x0B0B);
    for i in 0..ITERS {
        let circ = C2Circle { p: rng.v_small(), r: -rng.small().abs() };
        let b = rng.b_small();
        let inv = aabb(b.max.x, b.max.y, b.min.x, b.min.y);
        let ctx = format!("rand {i}: circ={} inverted={}", show_c(circ), show_b(inv));
        unsafe {
            eq_int("row11/neg-r", &ctx,
                (c.collided)(cp(&circ), C2_TYPE_CIRCLE, bp(&inv), C2_TYPE_AABB),
                (r.collided)(cp(&circ), C2_TYPE_CIRCLE, bp(&inv), C2_TYPE_AABB));
        }
    }
}

// ===========================================================================
// Row 12 — the nine shape/vector functions have no error path at all
// ===========================================================================

#[test]
fn row12_shape_functions_have_no_error_path() {
    let (c, r) = both();
    let mut rng = Rng::new(0x0C0C);
    for i in 0..ITERS {
        let (a, b) = (rng.v_wild(), rng.v_wild());
        let (ca, cb) = (rng.c_wild(), rng.c_wild());
        let (ba, bb) = (rng.b_wild(), rng.b_wild());
        let ctx = format!("iter {i}: a={} b={} ca={} ba={}", show_v(a), show_v(b), show_c(ca), show_b(ba));
        unsafe {
            eq_v("row12/c2V", &ctx, (c.c2V)(a.x, a.y), (r.c2V)(a.x, a.y));
            eq_v("row12/c2Maxv", &ctx, (c.c2Maxv)(a, b), (r.c2Maxv)(a, b));
            eq_v("row12/c2Minv", &ctx, (c.c2Minv)(a, b), (r.c2Minv)(a, b));
            eq_v("row12/c2Clampv", &ctx, (c.c2Clampv)(a, b, a), (r.c2Clampv)(a, b, a));
            eq_v("row12/c2Sub", &ctx, (c.c2Sub)(a, b), (r.c2Sub)(a, b));
            eq_f32("row12/c2Dot", &ctx, (c.c2Dot)(a, b), (r.c2Dot)(a, b));

            let cv = (c.c2CircletoCircle)(ca, cb);
            eq_int("row12/CC", &ctx, cv, (r.c2CircletoCircle)(ca, cb));
            assert_bool_like("row12/CC", &ctx, cv);
            let cv = (c.c2CircletoAABB)(ca, ba);
            eq_int("row12/CA", &ctx, cv, (r.c2CircletoAABB)(ca, ba));
            assert_bool_like("row12/CA", &ctx, cv);
            let cv = (c.c2AABBtoAABB)(ba, bb);
            eq_int("row12/AA", &ctx, cv, (r.c2AABBtoAABB)(ba, bb));
            assert_bool_like("row12/AA", &ctx, cv);
        }
    }
}

// ===========================================================================
// Proof that the `0` of rows 1-6 is the `default:` arm, not a plain miss
// ===========================================================================

#[test]
fn sentinel_zero_is_the_default_arm_not_a_miss() {
    let (c, r) = both();
    let (circ, bx) = always_hit();
    // 1) With valid tags these operands report a collision in BOTH libraries.
    unsafe {
        for (label, pa, ta, pb, tb) in [
            ("CC", cp(&circ), C2_TYPE_CIRCLE, cp(&circ), C2_TYPE_CIRCLE),
            ("CA", cp(&circ), C2_TYPE_CIRCLE, bp(&bx), C2_TYPE_AABB),
            ("AC", bp(&bx), C2_TYPE_AABB, cp(&circ), C2_TYPE_CIRCLE),
            ("AA", bp(&bx), C2_TYPE_AABB, bp(&bx), C2_TYPE_AABB),
        ] {
            let cv = (c.collided)(pa, ta, pb, tb);
            let rv = (r.collided)(pa, ta, pb, tb);
            eq_int("sentinel", label, cv, rv);
            assert_eq!(cv, 1, "[sentinel] {label} was expected to collide");
        }
        // 2) The very same operands with an invalid tag return 0 — so the 0 can
        //    only come from the `default:` arm.
        for &bad in &INVALID_TAGS {
            let ctx = format!("bad tag 0x{bad:08x}");
            let cv = (c.collided)(cp(&circ), bad, bp(&bx), C2_TYPE_AABB);
            eq_int("sentinel/A", &ctx, cv, (r.collided)(cp(&circ), bad, bp(&bx), C2_TYPE_AABB));
            assert_eq!(cv, 0, "[sentinel] {ctx}");
            let cv = (c.collided)(cp(&circ), C2_TYPE_CIRCLE, bp(&bx), bad);
            eq_int("sentinel/B", &ctx, cv, (r.collided)(cp(&circ), C2_TYPE_CIRCLE, bp(&bx), bad));
            assert_eq!(cv, 0, "[sentinel] {ctx}");
            let cv = (c.collided)(bp(&bx), C2_TYPE_AABB, cp(&circ), bad);
            eq_int("sentinel/C", &ctx, cv, (r.collided)(bp(&bx), C2_TYPE_AABB, cp(&circ), bad));
            assert_eq!(cv, 0, "[sentinel] {ctx}");
        }
    }
}
