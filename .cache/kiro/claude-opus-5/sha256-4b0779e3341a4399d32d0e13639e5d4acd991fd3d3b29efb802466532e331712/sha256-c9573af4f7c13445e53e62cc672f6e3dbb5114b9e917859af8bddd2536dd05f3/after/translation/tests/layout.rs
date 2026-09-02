//! Layout parity: the struct mirrors used by the differential harness must have
//! exactly the size, alignment and field offsets that the C compiler gives the
//! originals in `c_src/src/lib.c`.
//!
//! These numbers were read off a `gcc` build of the same declarations:
//!
//! ```text
//! c2v 8/4   c2r 8/4   c2x 16/4   c2Circle 12/4   c2AABB 16/4   c2Capsule 20/4
//! c2GJKCache 36/4 (div@32)      c2Proxy 72/4 (verts@8)
//! c2sv 36/4 (iB@32)
//! c2Simplex 152/4  a@0 b@36 c@72 d@108 div@144 count@148
//! ```
//!
//! This matters because it is what makes `[c2sv; 4]` a valid stand-in for the
//! four named members `a, b, c, d` (the C walks them with `c2sv *verts = &s.a;`),
//! and it is what guarantees the field-wise comparison in the harness covers
//! every byte the C writes — there is no padding anywhere to hide a difference.

mod common;
use common::*;
use std::mem::{align_of, size_of};

macro_rules! layout {
    ($t:ty, $size:expr, $align:expr) => {
        assert_eq!(size_of::<$t>(), $size, "size_of::<{}>()", stringify!($t));
        assert_eq!(align_of::<$t>(), $align, "align_of::<{}>()", stringify!($t));
    };
}

fn offset_of<T, F>(base: &T, field: &F) -> usize {
    field as *const F as usize - base as *const T as usize
}

#[test]
fn struct_layouts_match_the_c_compiler() {
    layout!(C2v, 8, 4);
    layout!(C2r, 8, 4);
    layout!(C2x, 16, 4);
    layout!(C2Circle, 12, 4);
    layout!(C2AABB, 16, 4);
    layout!(C2Capsule, 20, 4);
    layout!(C2GJKCache, 36, 4);
    layout!(C2Proxy, 72, 4);
    layout!(C2sv, 36, 4);
    layout!(C2Simplex, 152, 4);

    let c = C2GJKCache::default();
    assert_eq!(offset_of(&c, &c.metric), 0);
    assert_eq!(offset_of(&c, &c.count), 4);
    assert_eq!(offset_of(&c, &c.iA), 8);
    assert_eq!(offset_of(&c, &c.iB), 20);
    assert_eq!(offset_of(&c, &c.div), 32);

    let p = C2Proxy::default();
    assert_eq!(offset_of(&p, &p.radius), 0);
    assert_eq!(offset_of(&p, &p.count), 4);
    assert_eq!(offset_of(&p, &p.verts), 8);

    let v = C2sv::default();
    assert_eq!(offset_of(&v, &v.sA), 0);
    assert_eq!(offset_of(&v, &v.sB), 8);
    assert_eq!(offset_of(&v, &v.p), 16);
    assert_eq!(offset_of(&v, &v.u), 24);
    assert_eq!(offset_of(&v, &v.iA), 28);
    assert_eq!(offset_of(&v, &v.iB), 32);

    // The four named c2sv members become [c2sv; 4] at 0 / 36 / 72 / 108.
    let s = C2Simplex::default();
    assert_eq!(offset_of(&s, &s.v[0]), 0);
    assert_eq!(offset_of(&s, &s.v[1]), 36);
    assert_eq!(offset_of(&s, &s.v[2]), 72);
    assert_eq!(offset_of(&s, &s.v[3]), 108);
    assert_eq!(offset_of(&s, &s.div), 144);
    assert_eq!(offset_of(&s, &s.count), 148);
}

/// The field-wise comparator must decompose each struct into exactly
/// `size_of / 4` lanes — i.e. it must cover every byte, with no padding skipped.
#[test]
fn comparator_covers_every_byte() {
    fn check<T: Lanes + Default>(name: &str) {
        let v = T::default();
        assert_eq!(
            v.lanes().len(),
            size_of::<T>() / 4,
            "{name}: comparator covers {} of {} 4-byte words",
            v.lanes().len(),
            size_of::<T>() / 4
        );
    }
    check::<C2v>("C2v");
    check::<C2r>("C2r");
    check::<C2x>("C2x");
    check::<C2Circle>("C2Circle");
    check::<C2AABB>("C2AABB");
    check::<C2Capsule>("C2Capsule");
    check::<C2GJKCache>("C2GJKCache");
    check::<C2Proxy>("C2Proxy");
    check::<C2sv>("C2sv");
    check::<C2Simplex>("C2Simplex");
}

/// Sanity check on the comparator itself: it must reject a one-bit difference in
/// any non-NaN lane (including a `+0` vs `-0` sign flip), and must accept two
/// NaNs with different payloads (see the note in `Lane::agrees`).
#[test]
fn comparator_rejects_real_differences() {
    let a = C2v { x: 1.0, y: 2.0 };
    let mut b = a;
    b.y = f32::from_bits(a.y.to_bits() + 1);
    assert!(lanes_agree(&a.lanes(), &b.lanes()).is_some(), "1-ulp difference not caught");

    let z = C2v { x: 0.0, y: 0.0 };
    let nz = C2v { x: -0.0, y: 0.0 };
    assert!(lanes_agree(&z.lanes(), &nz.lanes()).is_some(), "+0 vs -0 not caught");

    let n1 = C2v { x: f32::from_bits(0x7fc0_0001), y: 1.0 };
    let n2 = C2v { x: f32::from_bits(0xffc0_9999), y: 1.0 };
    assert!(lanes_agree(&n1.lanes(), &n2.lanes()).is_none(), "NaN payloads must be tolerated");

    let n3 = C2v { x: f32::from_bits(0x7fc0_0001), y: 0.0 };
    assert!(lanes_agree(&n1.lanes(), &n3.lanes()).is_some(), "NaN tolerance leaked into other lanes");

    // An int lane must never be NaN-tolerated.
    let c1 = C2GJKCache { count: 1, ..Default::default() };
    let c2 = C2GJKCache { count: 2, ..Default::default() };
    assert!(lanes_agree(&c1.lanes(), &c2.lanes()).is_some(), "int lane difference not caught");
}

/// Both `.so`s must actually have been loaded from disk, and from different
/// files — a harness that accidentally loaded the same library twice would pass
/// everything vacuously.
#[test]
fn both_libraries_are_distinct_files() {
    let l = libs();
    assert!(l.c_path.is_file(), "C .so not found: {}", l.c_path.display());
    assert!(l.r_path.is_file(), "Rust .so not found: {}", l.r_path.display());
    let c = std::fs::canonicalize(&l.c_path).unwrap();
    let r = std::fs::canonicalize(&l.r_path).unwrap();
    assert_ne!(c, r, "the harness loaded the same file twice");
    eprintln!("C    .so: {}", c.display());
    eprintln!("Rust .so: {}", r.display());

    // And the symbols really do resolve to different addresses.
    type Fn3 = unsafe extern "C" fn(f32, f32, f32) -> i32;
    let (cf, rf) = l.pair::<Fn3>("reverse_collide");
    let (ca, ra) = (*cf as *const (), *rf as *const ());
    assert_ne!(ca, ra, "reverse_collide resolved to the same address in both libraries");
}
