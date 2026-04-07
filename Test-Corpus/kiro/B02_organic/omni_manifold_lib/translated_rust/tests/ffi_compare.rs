use libloading::{Library, Symbol};
use std::os::raw::c_int;

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
struct c2v { x: f32, y: f32 }

#[repr(C)]
#[derive(Clone, Debug)]
struct c2Manifold {
    count: c_int,
    depths: [f32; 2],
    contact_points: [c2v; 2],
    n: c2v,
}
impl Default for c2Manifold { fn default() -> Self { unsafe { std::mem::zeroed() } } }

const C2_TYPE_CAPSULE: c_int = 0;
const C2_TYPE_CIRCLE: c_int = 1;
const C2_TYPE_AABB: c_int = 2;

type OmniManifoldFn = unsafe extern "C" fn(
    *mut c2Manifold, c_int, f32, f32, f32, f32, f32, c_int, f32, f32, f32, f32, f32,
);

fn manifold_str(m: &c2Manifold) -> String {
    let mut s = format!("count={}", m.count);
    for i in 0..m.count as usize {
        s += &format!(" d[{}]={:e} cp[{}]=({:e},{:e})",
            i, m.depths[i], i, m.contact_points[i].x, m.contact_points[i].y);
    }
    if m.count > 0 {
        s += &format!(" n=({:e},{:e})", m.n.x, m.n.y);
    }
    s
}

fn call_lib(lib: &Library, ta: c_int, a1: f32, a2: f32, a3: f32, a4: f32, a5: f32,
            tb: c_int, b1: f32, b2: f32, b3: f32, b4: f32, b5: f32) -> c2Manifold {
    unsafe {
        let f: Symbol<OmniManifoldFn> = lib.get(b"omni_manifold").unwrap();
        let mut m = c2Manifold::default();
        f(&mut m, ta, a1, a2, a3, a4, a5, tb, b1, b2, b3, b4, b5);
        m
    }
}

fn assert_manifold_eq(c: &c2Manifold, r: &c2Manifold, label: &str) {
    assert_eq!(c.count, r.count, "{label}: count C={} Rust={}", c.count, r.count);
    for i in 0..c.count as usize {
        assert_eq!(c.depths[i].to_bits(), r.depths[i].to_bits(),
            "{label}: depths[{i}] C={} Rust={}", c.depths[i], r.depths[i]);
        assert_eq!(c.contact_points[i].x.to_bits(), r.contact_points[i].x.to_bits(),
            "{label}: cp[{i}].x C={} Rust={}", c.contact_points[i].x, r.contact_points[i].x);
        assert_eq!(c.contact_points[i].y.to_bits(), r.contact_points[i].y.to_bits(),
            "{label}: cp[{i}].y C={} Rust={}", c.contact_points[i].y, r.contact_points[i].y);
    }
    if c.count > 0 {
        assert_eq!(c.n.x.to_bits(), r.n.x.to_bits(),
            "{label}: n.x C={} Rust={}", c.n.x, r.n.x);
        assert_eq!(c.n.y.to_bits(), r.n.y.to_bits(),
            "{label}: n.y C={} Rust={}", c.n.y, r.n.y);
    }
}

struct TC {
    label: &'static str,
    ta: c_int, a1: f32, a2: f32, a3: f32, a4: f32, a5: f32,
    tb: c_int, b1: f32, b2: f32, b3: f32, b4: f32, b5: f32,
}

/// Tests for type combos that don't trigger the C UB path (no AABB+Capsule).
/// Both libraries loaded simultaneously is safe here.
#[test]
fn test_safe_cases() {
    let c_lib = unsafe { Library::new("c_src/build/libtranslated_rust.so").unwrap() };
    let r_lib = unsafe { Library::new("target/debug/libomni_manifold_lib.so").unwrap() };

    let mut cases: Vec<TC> = vec![
        // Circle-Circle
        TC { label: "cc_overlap", ta: C2_TYPE_CIRCLE, a1: 0.0, a2: 0.0, a3: 5.0, a4: 0.0, a5: 0.0,
            tb: C2_TYPE_CIRCLE, b1: 3.0, b2: 0.0, b3: 5.0, b4: 0.0, b5: 0.0 },
        TC { label: "cc_no", ta: C2_TYPE_CIRCLE, a1: 0.0, a2: 0.0, a3: 1.0, a4: 0.0, a5: 0.0,
            tb: C2_TYPE_CIRCLE, b1: 10.0, b2: 0.0, b3: 1.0, b4: 0.0, b5: 0.0 },
        TC { label: "cc_concentric", ta: C2_TYPE_CIRCLE, a1: 0.0, a2: 0.0, a3: 5.0, a4: 0.0, a5: 0.0,
            tb: C2_TYPE_CIRCLE, b1: 0.0, b2: 0.0, b3: 3.0, b4: 0.0, b5: 0.0 },
        // Circle-AABB
        TC { label: "ca_overlap", ta: C2_TYPE_CIRCLE, a1: 3.0, a2: 3.0, a3: 2.0, a4: 0.0, a5: 0.0,
            tb: C2_TYPE_AABB, b1: 0.0, b2: 0.0, b3: 4.0, b4: 4.0, b5: 0.0 },
        TC { label: "ca_inside", ta: C2_TYPE_CIRCLE, a1: 2.0, a2: 2.0, a3: 0.5, a4: 0.0, a5: 0.0,
            tb: C2_TYPE_AABB, b1: 0.0, b2: 0.0, b3: 4.0, b4: 4.0, b5: 0.0 },
        TC { label: "ca_no", ta: C2_TYPE_CIRCLE, a1: 10.0, a2: 10.0, a3: 1.0, a4: 0.0, a5: 0.0,
            tb: C2_TYPE_AABB, b1: 0.0, b2: 0.0, b3: 4.0, b4: 4.0, b5: 0.0 },
        // AABB-Circle
        TC { label: "ac_overlap", ta: C2_TYPE_AABB, a1: 0.0, a2: 0.0, a3: 4.0, a4: 4.0, a5: 0.0,
            tb: C2_TYPE_CIRCLE, b1: 3.0, b2: 3.0, b3: 2.0, b4: 0.0, b5: 0.0 },
        // Circle-Capsule
        TC { label: "ccaps_overlap", ta: C2_TYPE_CIRCLE, a1: 0.0, a2: 0.0, a3: 2.0, a4: 0.0, a5: 0.0,
            tb: C2_TYPE_CAPSULE, b1: -5.0, b2: 0.0, b3: 5.0, b4: 0.0, b5: 1.0 },
        TC { label: "ccaps_no", ta: C2_TYPE_CIRCLE, a1: 0.0, a2: 10.0, a3: 1.0, a4: 0.0, a5: 0.0,
            tb: C2_TYPE_CAPSULE, b1: -5.0, b2: 0.0, b3: 5.0, b4: 0.0, b5: 1.0 },
        // Capsule-Circle
        TC { label: "capsc_overlap", ta: C2_TYPE_CAPSULE, a1: -5.0, a2: 0.0, a3: 5.0, a4: 0.0, a5: 1.0,
            tb: C2_TYPE_CIRCLE, b1: 0.0, b2: 0.0, b3: 2.0, b4: 0.0, b5: 0.0 },
        // AABB-AABB
        TC { label: "aa_overlap", ta: C2_TYPE_AABB, a1: 0.0, a2: 0.0, a3: 4.0, a4: 4.0, a5: 0.0,
            tb: C2_TYPE_AABB, b1: 2.0, b2: 2.0, b3: 6.0, b4: 6.0, b5: 0.0 },
        TC { label: "aa_no", ta: C2_TYPE_AABB, a1: 0.0, a2: 0.0, a3: 1.0, a4: 1.0, a5: 0.0,
            tb: C2_TYPE_AABB, b1: 5.0, b2: 5.0, b3: 6.0, b4: 6.0, b5: 0.0 },
        TC { label: "aa_edge", ta: C2_TYPE_AABB, a1: 0.0, a2: 0.0, a3: 2.0, a4: 2.0, a5: 0.0,
            tb: C2_TYPE_AABB, b1: 1.5, b2: 0.0, b3: 3.5, b4: 2.0, b5: 0.0 },
        // Capsule-Capsule
        TC { label: "capscaps_overlap", ta: C2_TYPE_CAPSULE, a1: 0.0, a2: 0.0, a3: 4.0, a4: 0.0, a5: 1.0,
            tb: C2_TYPE_CAPSULE, b1: 2.0, b2: 0.0, b3: 6.0, b4: 0.0, b5: 1.0 },
        TC { label: "capscaps_perp", ta: C2_TYPE_CAPSULE, a1: -5.0, a2: 0.0, a3: 5.0, a4: 0.0, a5: 1.0,
            tb: C2_TYPE_CAPSULE, b1: 0.0, b2: -5.0, b3: 0.0, b4: 5.0, b5: 1.0 },
        TC { label: "capscaps_no", ta: C2_TYPE_CAPSULE, a1: 0.0, a2: 0.0, a3: 4.0, a4: 0.0, a5: 0.5,
            tb: C2_TYPE_CAPSULE, b1: 0.0, b2: 10.0, b3: 4.0, b4: 10.0, b5: 0.5 },
        TC { label: "capscaps_diag", ta: C2_TYPE_CAPSULE, a1: 0.0, a2: 0.0, a3: 3.0, a4: 3.0, a5: 1.0,
            tb: C2_TYPE_CAPSULE, b1: 1.0, b2: 0.0, b3: 4.0, b4: 3.0, b5: 1.0 },
    ];
    // Sweeps
    for i in 0..20 {
        let o = i as f32 * 0.5;
        cases.push(TC { label: "sweep_cc", ta: C2_TYPE_CIRCLE, a1: 0.0, a2: 0.0, a3: 3.0, a4: 0.0, a5: 0.0,
            tb: C2_TYPE_CIRCLE, b1: o, b2: 0.0, b3: 3.0, b4: 0.0, b5: 0.0 });
    }
    for i in 0..20 {
        let o = i as f32 * 0.3;
        cases.push(TC { label: "sweep_ca", ta: C2_TYPE_CIRCLE, a1: o, a2: o, a3: 2.0, a4: 0.0, a5: 0.0,
            tb: C2_TYPE_AABB, b1: 0.0, b2: 0.0, b3: 4.0, b4: 4.0, b5: 0.0 });
    }
    for i in 0..20 {
        let o = i as f32 * 0.25;
        cases.push(TC { label: "sweep_aa", ta: C2_TYPE_AABB, a1: 0.0, a2: 0.0, a3: 2.0, a4: 2.0, a5: 0.0,
            tb: C2_TYPE_AABB, b1: o, b2: o, b3: o + 2.0, b4: o + 2.0, b5: 0.0 });
    }
    for i in 0..20 {
        let o = i as f32 * 0.3;
        cases.push(TC { label: "sweep_capscaps", ta: C2_TYPE_CAPSULE, a1: 0.0, a2: 0.0, a3: 4.0, a4: 0.0, a5: 1.0,
            tb: C2_TYPE_CAPSULE, b1: 0.0, b2: o, b3: 4.0, b4: o, b5: 1.0 });
    }
    for i in 0..20 {
        let o = i as f32 * 0.4;
        cases.push(TC { label: "sweep_ccaps", ta: C2_TYPE_CIRCLE, a1: 0.0, a2: o, a3: 2.0, a4: 0.0, a5: 0.0,
            tb: C2_TYPE_CAPSULE, b1: -3.0, b2: 0.0, b3: 3.0, b4: 0.0, b5: 1.0 });
    }

    for (idx, tc) in cases.iter().enumerate() {
        let cm = call_lib(&c_lib, tc.ta, tc.a1, tc.a2, tc.a3, tc.a4, tc.a5,
                          tc.tb, tc.b1, tc.b2, tc.b3, tc.b4, tc.b5);
        let rm = call_lib(&r_lib, tc.ta, tc.a1, tc.a2, tc.a3, tc.a4, tc.a5,
                          tc.tb, tc.b1, tc.b2, tc.b3, tc.b4, tc.b5);
        assert_manifold_eq(&cm, &rm, &format!("{}[{}]", tc.label, idx));
    }
}

/// AABB-Capsule and Capsule-AABB: load one library at a time to avoid
/// SIGSEGV from C's UB in c2MakeProxy (missing C2_TYPE_POLY handler).
#[test]
fn test_aabb_capsule_cases() {
    let cases: Vec<(c_int, f32, f32, f32, f32, f32, c_int, f32, f32, f32, f32, f32)> = {
        let mut v = vec![];
        v.push((C2_TYPE_AABB, 0.0, 0.0, 4.0, 4.0, 0.0,
                C2_TYPE_CAPSULE, 2.0, -1.0, 2.0, 5.0, 0.5));
        v.push((C2_TYPE_CAPSULE, 2.0, -1.0, 2.0, 5.0, 0.5,
                C2_TYPE_AABB, 0.0, 0.0, 4.0, 4.0, 0.0));
        for i in 0..20 {
            let o = i as f32 * 0.3;
            v.push((C2_TYPE_AABB, 0.0, 0.0, 3.0, 3.0, 0.0,
                    C2_TYPE_CAPSULE, 1.0, o, 1.0, o + 3.0, 0.5));
        }
        for i in 0..20 {
            let o = i as f32 * 0.3;
            v.push((C2_TYPE_CAPSULE, 1.0, o, 1.0, o + 3.0, 0.5,
                    C2_TYPE_AABB, 0.0, 0.0, 3.0, 3.0, 0.0));
        }
        v
    };

    for (idx, &(ta, a1, a2, a3, a4, a5, tb, b1, b2, b3, b4, b5)) in cases.iter().enumerate() {
        let c_str = {
            let lib = unsafe { Library::new("c_src/build/libtranslated_rust.so").unwrap() };
            manifold_str(&call_lib(&lib, ta, a1, a2, a3, a4, a5, tb, b1, b2, b3, b4, b5))
        };
        let r_str = {
            let lib = unsafe { Library::new("target/debug/libomni_manifold_lib.so").unwrap() };
            manifold_str(&call_lib(&lib, ta, a1, a2, a3, a4, a5, tb, b1, b2, b3, b4, b5))
        };
        assert_eq!(c_str, r_str, "aabb_capsule[{idx}]: C='{c_str}' Rust='{r_str}'");
    }
}
