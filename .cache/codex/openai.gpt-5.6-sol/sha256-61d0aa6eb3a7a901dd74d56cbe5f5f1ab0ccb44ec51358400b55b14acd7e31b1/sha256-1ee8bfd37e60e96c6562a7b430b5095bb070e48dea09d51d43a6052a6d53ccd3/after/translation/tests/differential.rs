use libloading::Library;
use std::ffi::{c_char, c_int, c_void};
use std::path::{Path, PathBuf};
use std::ptr::{null, null_mut};

const CIRCLE: c_int = 0;
const AABB: c_int = 1;
const CAPSULE: c_int = 2;
const RANDOM_CASES: usize = 32;

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct C2v {
    x: f32,
    y: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct C2r {
    c: f32,
    s: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct C2x {
    p: C2v,
    r: C2r,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct C2Circle {
    p: C2v,
    r: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct C2Aabb {
    min: C2v,
    max: C2v,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct C2Capsule {
    a: C2v,
    b: C2v,
    r: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct C2GjkCache {
    metric: f32,
    count: c_int,
    i_a: [c_int; 3],
    i_b: [c_int; 3],
    div: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct C2Proxy {
    radius: f32,
    count: c_int,
    verts: [C2v; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct C2sv {
    s_a: C2v,
    s_b: C2v,
    p: C2v,
    u: f32,
    i_a: c_int,
    i_b: c_int,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct C2Simplex {
    a: C2sv,
    b: C2sv,
    c: C2sv,
    d: C2sv,
    div: f32,
    count: c_int,
}

type C2VFn = unsafe extern "C" fn(f32, f32) -> C2v;
type C2VecScalarFn = unsafe extern "C" fn(C2v, f32) -> C2v;
type C2VecVecFn = unsafe extern "C" fn(C2v, C2v) -> C2v;
type C2DotFn = unsafe extern "C" fn(C2v, C2v) -> f32;
type C2RotIdentityFn = unsafe extern "C" fn() -> C2r;
type C2xIdentityFn = unsafe extern "C" fn() -> C2x;
type C2BBVertsFn = unsafe extern "C" fn(*mut C2v, *mut C2Aabb);
type C2MakeProxyFn = unsafe extern "C" fn(*const c_void, c_int, *mut C2Proxy);
type C2VecFloatFn = unsafe extern "C" fn(C2v) -> f32;
type C2MetricFn = unsafe extern "C" fn(*mut C2Simplex) -> f32;
type C2RotVecFn = unsafe extern "C" fn(C2r, C2v) -> C2v;
type C2TransformVecFn = unsafe extern "C" fn(C2x, C2v) -> C2v;
type C2SimplexMutFn = unsafe extern "C" fn(*mut C2Simplex);
type C2SimplexVecFn = unsafe extern "C" fn(*mut C2Simplex) -> C2v;
type C2SupportFn = unsafe extern "C" fn(*const C2v, c_int, C2v) -> c_int;
type C2WitnessFn = unsafe extern "C" fn(*mut C2Simplex, *mut C2v, *mut C2v);
type C2GjkFn = unsafe extern "C" fn(
    *const c_void,
    c_int,
    *const C2x,
    *const c_void,
    c_int,
    *const C2x,
    *mut C2v,
    *mut C2v,
    c_int,
    *mut c_int,
    *mut C2GjkCache,
) -> f32;
type GjkFn =
    unsafe extern "C" fn(c_char, *mut C2v, *mut C2v, f32, f32, f32, f32, f32, f32, f32, f32, f32);

struct Api {
    _library: Library,
    c2_v: C2VFn,
    c2_mulvs: C2VecScalarFn,
    c2_maxv: C2VecVecFn,
    c2_minv: C2VecVecFn,
    c2_clampv: unsafe extern "C" fn(C2v, C2v, C2v) -> C2v,
    c2_sub: C2VecVecFn,
    c2_dot: C2DotFn,
    c2_rot_identity: C2RotIdentityFn,
    c2x_identity: C2xIdentityFn,
    c2_bb_verts: C2BBVertsFn,
    c2_make_proxy: C2MakeProxyFn,
    c2_len: C2VecFloatFn,
    c2_det2: C2DotFn,
    c2_metric: C2MetricFn,
    c2_mulrv: C2RotVecFn,
    c2_add: C2VecVecFn,
    c2_mulxv: C2TransformVecFn,
    c22: C2SimplexMutFn,
    c23: C2SimplexMutFn,
    c2_neg: unsafe extern "C" fn(C2v) -> C2v,
    c2_skew: unsafe extern "C" fn(C2v) -> C2v,
    c2_ccw90: unsafe extern "C" fn(C2v) -> C2v,
    c2_d: C2SimplexVecFn,
    c2_support: C2SupportFn,
    c2_witness: C2WitnessFn,
    c2_div: C2VecScalarFn,
    c2_norm: unsafe extern "C" fn(C2v) -> C2v,
    c2_l: C2SimplexVecFn,
    c2_mulrv_t: C2RotVecFn,
    c2_gjk: C2GjkFn,
    gjk: GjkFn,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        unsafe fn get<T: Copy>(library: &Library, name: &[u8]) -> T {
            *unsafe { library.get::<T>(name) }
                .unwrap_or_else(|error| panic!("failed to load {:?}: {error}", name))
        }

        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        unsafe {
            Self {
                c2_v: get(&library, b"c2V\0"),
                c2_mulvs: get(&library, b"c2Mulvs\0"),
                c2_maxv: get(&library, b"c2Maxv\0"),
                c2_minv: get(&library, b"c2Minv\0"),
                c2_clampv: get(&library, b"c2Clampv\0"),
                c2_sub: get(&library, b"c2Sub\0"),
                c2_dot: get(&library, b"c2Dot\0"),
                c2_rot_identity: get(&library, b"c2RotIdentity\0"),
                c2x_identity: get(&library, b"c2xIdentity\0"),
                c2_bb_verts: get(&library, b"c2BBVerts\0"),
                c2_make_proxy: get(&library, b"c2MakeProxy\0"),
                c2_len: get(&library, b"c2Len\0"),
                c2_det2: get(&library, b"c2Det2\0"),
                c2_metric: get(&library, b"c2GJKSimplexMetric\0"),
                c2_mulrv: get(&library, b"c2Mulrv\0"),
                c2_add: get(&library, b"c2Add\0"),
                c2_mulxv: get(&library, b"c2Mulxv\0"),
                c22: get(&library, b"c22\0"),
                c23: get(&library, b"c23\0"),
                c2_neg: get(&library, b"c2Neg\0"),
                c2_skew: get(&library, b"c2Skew\0"),
                c2_ccw90: get(&library, b"c2CCW90\0"),
                c2_d: get(&library, b"c2D\0"),
                c2_support: get(&library, b"c2Support\0"),
                c2_witness: get(&library, b"c2Witness\0"),
                c2_div: get(&library, b"c2Div\0"),
                c2_norm: get(&library, b"c2Norm\0"),
                c2_l: get(&library, b"c2L\0"),
                c2_mulrv_t: get(&library, b"c2MulrvT\0"),
                c2_gjk: get(&library, b"c2GJK\0"),
                gjk: get(&library, b"gjk\0"),
                _library: library,
            }
        }
    }
}

struct Libraries {
    c: Api,
    rust: Api,
}

impl Libraries {
    fn load() -> Self {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_path = manifest
            .parent()
            .unwrap()
            .join("c_src/build/libharvest-work-m0JAPI.so");
        let debug_path = manifest.join("target/debug/libgjk_lib.so");
        let release_path = manifest.join("target/release/libgjk_lib.so");
        let rust_path = if let Some(path) = std::env::var_os("GJK_RUST_SO") {
            PathBuf::from(path)
        } else if debug_path.exists() {
            debug_path
        } else {
            release_path
        };
        assert!(c_path.exists(), "missing C library: {}", c_path.display());
        assert!(
            rust_path.exists(),
            "missing Rust library: {}",
            rust_path.display()
        );
        unsafe {
            Self {
                c: Api::load(&c_path),
                rust: Api::load(&rust_path),
            }
        }
    }
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        (x >> 16) as u32
    }

    fn f32(&mut self, low: f32, high: f32) -> f32 {
        let unit = (self.u32() as f64 / u32::MAX as f64) as f32;
        low + (high - low) * unit
    }

    fn vec(&mut self) -> C2v {
        C2v {
            x: self.f32(-20.0, 20.0),
            y: self.f32(-20.0, 20.0),
        }
    }

    fn nonzero_vec(&mut self) -> C2v {
        loop {
            let value = self.vec();
            if value.x != 0.0 || value.y != 0.0 {
                return value;
            }
        }
    }

    fn transform(&mut self) -> C2x {
        let quarter_turn = self.u32() % 4;
        let r = match quarter_turn {
            0 => C2r { c: 1.0, s: 0.0 },
            1 => C2r { c: 0.0, s: 1.0 },
            2 => C2r { c: -1.0, s: 0.0 },
            _ => C2r { c: 0.0, s: -1.0 },
        };
        C2x { p: self.vec(), r }
    }
}

fn assert_same<T>(c: &T, rust: &T, context: impl std::fmt::Display) {
    let size = std::mem::size_of::<T>();
    let c_bytes = unsafe { std::slice::from_raw_parts((c as *const T).cast::<u8>(), size) };
    let rust_bytes = unsafe { std::slice::from_raw_parts((rust as *const T).cast::<u8>(), size) };
    assert_eq!(c_bytes, rust_bytes, "{context}");
}

fn sentinel_vec() -> C2v {
    C2v {
        x: f32::from_bits(0x4229_0000),
        y: f32::from_bits(0xc2f6_0000),
    }
}

fn sentinel_proxy() -> C2Proxy {
    C2Proxy {
        radius: 91.25,
        count: -77,
        verts: [sentinel_vec(); 8],
    }
}

fn random_simplex(rng: &mut Rng, count: c_int) -> C2Simplex {
    fn vertex(rng: &mut Rng) -> C2sv {
        C2sv {
            s_a: rng.vec(),
            s_b: rng.vec(),
            p: rng.vec(),
            u: rng.f32(0.1, 10.0),
            i_a: (rng.u32() % 8) as c_int,
            i_b: (rng.u32() % 8) as c_int,
        }
    }
    C2Simplex {
        a: vertex(rng),
        b: vertex(rng),
        c: vertex(rng),
        d: vertex(rng),
        div: rng.f32(0.5, 20.0),
        count,
    }
}

#[test]
fn vector_proxy_and_transform_configurations_match() {
    let libs = Libraries::load();
    let mut rng = Rng::new(0x5eed_c001_0000_0001);

    unsafe {
        for case in 0..RANDOM_CASES {
            let a = rng.vec();
            let b = rng.vec();
            let scalar = rng.f32(-8.0, 8.0);
            macro_rules! compare_call {
                ($field:ident($($argument:expr),*), $name:literal) => {{
                    let c_value = (libs.c.$field)($($argument),*);
                    let rust_value = (libs.rust.$field)($($argument),*);
                    assert_same(&c_value, &rust_value, format!("{} case {}", $name, case));
                }};
            }
            compare_call!(c2_v(a.x, a.y), "c2V");
            compare_call!(c2_mulvs(a, scalar), "c2Mulvs");
            compare_call!(c2_sub(a, b), "c2Sub");
            compare_call!(c2_dot(a, b), "c2Dot");
            compare_call!(c2_add(a, b), "c2Add");
            compare_call!(c2_neg(a), "c2Neg");
            compare_call!(c2_skew(a), "c2Skew");
            compare_call!(c2_ccw90(a), "c2CCW90");
            compare_call!(c2_len(a), "c2Len");
            compare_call!(c2_det2(a, b), "c2Det2");

            let divisor = loop {
                let value = rng.f32(-8.0, 8.0);
                if value.abs() >= 0.25 {
                    break value;
                }
            };
            compare_call!(c2_div(a, divisor), "c2Div");
            let normal_input = rng.nonzero_vec();
            compare_call!(c2_norm(normal_input), "c2Norm");

            let rotation = rng.transform().r;
            let transform = rng.transform();
            compare_call!(c2_mulrv(rotation, a), "c2Mulrv");
            compare_call!(c2_mulrv_t(rotation, a), "c2MulrvT");
            compare_call!(c2_mulxv(transform, a), "c2Mulxv");
        }

        for x_gt in [false, true] {
            for y_gt in [false, true] {
                for case in 0..RANDOM_CASES {
                    let b = rng.vec();
                    let dx = rng.f32(0.1, 10.0);
                    let dy = rng.f32(0.1, 10.0);
                    let a = C2v {
                        x: if x_gt { b.x + dx } else { b.x - dx },
                        y: if y_gt { b.y + dy } else { b.y - dy },
                    };
                    let c_value = (libs.c.c2_maxv)(a, b);
                    let rust_value = (libs.rust.c2_maxv)(a, b);
                    assert_same(
                        &c_value,
                        &rust_value,
                        format!("c2Maxv x_gt={x_gt} y_gt={y_gt} case={case}"),
                    );
                }
            }
        }

        for x_lt in [false, true] {
            for y_lt in [false, true] {
                for case in 0..RANDOM_CASES {
                    let b = rng.vec();
                    let dx = rng.f32(0.1, 10.0);
                    let dy = rng.f32(0.1, 10.0);
                    let a = C2v {
                        x: if x_lt { b.x - dx } else { b.x + dx },
                        y: if y_lt { b.y - dy } else { b.y + dy },
                    };
                    let c_value = (libs.c.c2_minv)(a, b);
                    let rust_value = (libs.rust.c2_minv)(a, b);
                    assert_same(
                        &c_value,
                        &rust_value,
                        format!("c2Minv x_lt={x_lt} y_lt={y_lt} case={case}"),
                    );
                }
            }
        }

        let lo = C2v { x: -5.0, y: -7.0 };
        let hi = C2v { x: 6.0, y: 9.0 };
        for x_region in 0..3 {
            for y_region in 0..3 {
                for case in 0..RANDOM_CASES {
                    let x = match x_region {
                        0 => rng.f32(-20.0, lo.x - 0.1),
                        1 => rng.f32(lo.x, hi.x),
                        _ => rng.f32(hi.x + 0.1, 20.0),
                    };
                    let y = match y_region {
                        0 => rng.f32(-20.0, lo.y - 0.1),
                        1 => rng.f32(lo.y, hi.y),
                        _ => rng.f32(hi.y + 0.1, 20.0),
                    };
                    let c_value = (libs.c.c2_clampv)(C2v { x, y }, lo, hi);
                    let rust_value = (libs.rust.c2_clampv)(C2v { x, y }, lo, hi);
                    assert_same(
                        &c_value,
                        &rust_value,
                        format!("c2Clampv x_region={x_region} y_region={y_region} case={case}"),
                    );
                }
            }
        }

        let c_rotation = (libs.c.c2_rot_identity)();
        let rust_rotation = (libs.rust.c2_rot_identity)();
        assert_same(&c_rotation, &rust_rotation, "c2RotIdentity");
        let c_transform = (libs.c.c2x_identity)();
        let rust_transform = (libs.rust.c2x_identity)();
        assert_same(&c_transform, &rust_transform, "c2xIdentity");

        for case in 0..RANDOM_CASES {
            let center = rng.vec();
            let half_x = rng.f32(0.1, 8.0);
            let half_y = rng.f32(0.1, 8.0);
            let bb = C2Aabb {
                min: C2v {
                    x: center.x - half_x,
                    y: center.y - half_y,
                },
                max: C2v {
                    x: center.x + half_x,
                    y: center.y + half_y,
                },
            };
            let mut c_bb = bb;
            let mut rust_bb = bb;
            let mut c_verts = [sentinel_vec(); 4];
            let mut rust_verts = [sentinel_vec(); 4];
            (libs.c.c2_bb_verts)(c_verts.as_mut_ptr(), &mut c_bb);
            (libs.rust.c2_bb_verts)(rust_verts.as_mut_ptr(), &mut rust_bb);
            assert_same(&c_verts, &rust_verts, format!("c2BBVerts case={case}"));

            let circle = C2Circle {
                p: rng.vec(),
                r: rng.f32(0.0, 5.0),
            };
            let capsule = C2Capsule {
                a: rng.vec(),
                b: rng.vec(),
                r: rng.f32(0.0, 5.0),
            };
            let shapes = [
                ((&circle as *const C2Circle).cast::<c_void>(), CIRCLE),
                ((&bb as *const C2Aabb).cast::<c_void>(), AABB),
                ((&capsule as *const C2Capsule).cast::<c_void>(), CAPSULE),
            ];
            for (shape_ptr, shape_type) in shapes {
                let mut c_proxy = sentinel_proxy();
                let mut rust_proxy = sentinel_proxy();
                (libs.c.c2_make_proxy)(shape_ptr, shape_type, &mut c_proxy);
                (libs.rust.c2_make_proxy)(shape_ptr, shape_type, &mut rust_proxy);
                assert_same(
                    &c_proxy,
                    &rust_proxy,
                    format!("c2MakeProxy type={shape_type} case={case}"),
                );
            }
        }

        for count in [1, 2, 3, 0, 4, -1] {
            for case in 0..RANDOM_CASES {
                let mut c_simplex = random_simplex(&mut rng, count);
                let mut rust_simplex = c_simplex;
                let c_metric = (libs.c.c2_metric)(&mut c_simplex);
                let rust_metric = (libs.rust.c2_metric)(&mut rust_simplex);
                assert_same(
                    &c_metric,
                    &rust_metric,
                    format!("c2GJKSimplexMetric count={count} case={case}"),
                );
            }
        }
    }
}

fn dot(a: C2v, b: C2v) -> f32 {
    a.x * b.x + a.y * b.y
}

fn sub(a: C2v, b: C2v) -> C2v {
    C2v {
        x: a.x - b.x,
        y: a.y - b.y,
    }
}

fn det(a: C2v, b: C2v) -> f32 {
    a.x * b.y - a.y * b.x
}

fn c23_branch(a: C2v, b: C2v, c: C2v) -> usize {
    let u_ab = dot(b, sub(b, a));
    let v_ab = dot(a, sub(a, b));
    let u_bc = dot(c, sub(c, b));
    let v_bc = dot(b, sub(b, c));
    let u_ca = dot(a, sub(a, c));
    let v_ca = dot(c, sub(c, a));
    let area = det(sub(b, a), sub(c, a));
    let u_abc = det(b, c) * area;
    let v_abc = det(c, a) * area;
    let w_abc = det(a, b) * area;
    if v_ab <= 0.0 && u_ca <= 0.0 {
        0
    } else if u_ab <= 0.0 && v_bc <= 0.0 {
        1
    } else if u_bc <= 0.0 && v_ca <= 0.0 {
        2
    } else if u_ab > 0.0 && v_ab > 0.0 && w_abc <= 0.0 {
        3
    } else if u_bc > 0.0 && v_bc > 0.0 && u_abc <= 0.0 {
        4
    } else if u_ca > 0.0 && v_ca > 0.0 && v_abc <= 0.0 {
        5
    } else {
        6
    }
}

#[test]
fn simplex_and_support_configurations_match() {
    let libs = Libraries::load();
    let mut rng = Rng::new(0x5eed_c030_0000_0002);

    unsafe {
        let c22_points = [
            (C2v { x: 1.0, y: 0.0 }, C2v { x: 2.0, y: 0.0 }),
            (C2v { x: 2.0, y: 0.0 }, C2v { x: 1.0, y: 0.0 }),
            (C2v { x: -1.0, y: 0.0 }, C2v { x: 1.0, y: 0.0 }),
        ];
        for (branch, (a, b)) in c22_points.into_iter().enumerate() {
            for case in 0..RANDOM_CASES {
                let scale = rng.f32(0.1, 20.0);
                let mut c_simplex = random_simplex(&mut rng, 2);
                c_simplex.a.p = C2v {
                    x: a.x * scale,
                    y: a.y * scale,
                };
                c_simplex.b.p = C2v {
                    x: b.x * scale,
                    y: b.y * scale,
                };
                let mut rust_simplex = c_simplex;
                (libs.c.c22)(&mut c_simplex);
                (libs.rust.c22)(&mut rust_simplex);
                assert_same(
                    &c_simplex,
                    &rust_simplex,
                    format!("c22 branch={branch} case={case}"),
                );
            }
        }

        let mut branch_counts = [0usize; 7];
        let mut attempts = 0usize;
        while branch_counts.iter().any(|count| *count < RANDOM_CASES) {
            attempts += 1;
            assert!(attempts < 1_000_000, "could not generate every c23 branch");
            let a = rng.vec();
            let b = rng.vec();
            let c = rng.vec();
            let branch = c23_branch(a, b, c);
            if branch_counts[branch] >= RANDOM_CASES {
                continue;
            }
            let case = branch_counts[branch];
            branch_counts[branch] += 1;
            let mut c_simplex = random_simplex(&mut rng, 3);
            c_simplex.a.p = a;
            c_simplex.b.p = b;
            c_simplex.c.p = c;
            let mut rust_simplex = c_simplex;
            (libs.c.c23)(&mut c_simplex);
            (libs.rust.c23)(&mut rust_simplex);
            assert_same(
                &c_simplex,
                &rust_simplex,
                format!("c23 branch={branch} case={case}"),
            );
        }

        for case in 0..RANDOM_CASES {
            let mut count_one_c = random_simplex(&mut rng, 1);
            let mut count_one_rust = count_one_c;
            let c_value = (libs.c.c2_d)(&mut count_one_c);
            let rust_value = (libs.rust.c2_d)(&mut count_one_rust);
            assert_same(&c_value, &rust_value, format!("c2D count=1 case={case}"));

            for positive in [false, true] {
                let mut c_simplex = random_simplex(&mut rng, 2);
                c_simplex.a.p = C2v { x: 1.0, y: 0.0 };
                c_simplex.b.p = if positive {
                    C2v { x: 1.0, y: 1.0 }
                } else {
                    C2v { x: 1.0, y: -1.0 }
                };
                let mut rust_simplex = c_simplex;
                let c_value = (libs.c.c2_d)(&mut c_simplex);
                let rust_value = (libs.rust.c2_d)(&mut rust_simplex);
                assert_same(
                    &c_value,
                    &rust_value,
                    format!("c2D count=2 positive={positive} case={case}"),
                );
            }
            for count in [3, 0, 4, -1] {
                let mut c_simplex = random_simplex(&mut rng, count);
                let mut rust_simplex = c_simplex;
                let c_value = (libs.c.c2_d)(&mut c_simplex);
                let rust_value = (libs.rust.c2_d)(&mut rust_simplex);
                assert_same(
                    &c_value,
                    &rust_value,
                    format!("c2D default count={count} case={case}"),
                );
            }
        }

        for case in 0..RANDOM_CASES {
            let one = [rng.vec()];
            let direction = rng.nonzero_vec();
            let c_index = (libs.c.c2_support)(one.as_ptr(), 1, direction);
            let rust_index = (libs.rust.c2_support)(one.as_ptr(), 1, direction);
            assert_same(
                &c_index,
                &rust_index,
                format!("c2Support count=1 case={case}"),
            );

            let winner = (rng.u32() % 8) as usize;
            let mut unique = [C2v::default(); 8];
            for (index, vertex) in unique.iter_mut().enumerate() {
                vertex.x = index as f32;
                vertex.y = rng.f32(-10.0, 10.0);
            }
            unique.swap(winner, 7);
            let x_direction = C2v { x: 1.0, y: 0.0 };
            let c_index = (libs.c.c2_support)(unique.as_ptr(), 8, x_direction);
            let rust_index = (libs.rust.c2_support)(unique.as_ptr(), 8, x_direction);
            assert_same(
                &c_index,
                &rust_index,
                format!("c2Support unique maximum case={case}"),
            );

            let tied = [
                C2v { x: 9.0, y: 1.0 },
                C2v { x: 2.0, y: 4.0 },
                C2v { x: 9.0, y: -3.0 },
                C2v { x: -1.0, y: 7.0 },
            ];
            let c_index = (libs.c.c2_support)(tied.as_ptr(), 4, x_direction);
            let rust_index = (libs.rust.c2_support)(tied.as_ptr(), 4, x_direction);
            assert_same(
                &c_index,
                &rust_index,
                format!("c2Support tied maximum case={case}"),
            );
        }

        for count in [1, 2, 3, 0, 4, -1] {
            for case in 0..RANDOM_CASES {
                let mut c_simplex = random_simplex(&mut rng, count);
                let mut rust_simplex = c_simplex;
                let mut c_a = sentinel_vec();
                let mut c_b = sentinel_vec();
                let mut rust_a = sentinel_vec();
                let mut rust_b = sentinel_vec();
                (libs.c.c2_witness)(&mut c_simplex, &mut c_a, &mut c_b);
                (libs.rust.c2_witness)(&mut rust_simplex, &mut rust_a, &mut rust_b);
                assert_same(
                    &c_a,
                    &rust_a,
                    format!("c2Witness a count={count} case={case}"),
                );
                assert_same(
                    &c_b,
                    &rust_b,
                    format!("c2Witness b count={count} case={case}"),
                );
            }
        }

        for count in [1, 2, 3, 0, 4, -1] {
            for case in 0..RANDOM_CASES {
                let mut c_simplex = random_simplex(&mut rng, count);
                let mut rust_simplex = c_simplex;
                let c_value = (libs.c.c2_l)(&mut c_simplex);
                let rust_value = (libs.rust.c2_l)(&mut rust_simplex);
                assert_same(
                    &c_value,
                    &rust_value,
                    format!("c2L count={count} case={case}"),
                );
            }
        }
    }
}

enum Shape {
    Circle(C2Circle),
    Aabb(C2Aabb),
    Capsule(C2Capsule),
}

impl Shape {
    fn random(shape_type: c_int, rng: &mut Rng) -> Self {
        match shape_type {
            CIRCLE => Self::Circle(C2Circle {
                p: rng.vec(),
                r: rng.f32(0.0, 4.0),
            }),
            AABB => {
                let center = rng.vec();
                let half_x = rng.f32(0.05, 6.0);
                let half_y = rng.f32(0.05, 6.0);
                Self::Aabb(C2Aabb {
                    min: C2v {
                        x: center.x - half_x,
                        y: center.y - half_y,
                    },
                    max: C2v {
                        x: center.x + half_x,
                        y: center.y + half_y,
                    },
                })
            }
            CAPSULE => Self::Capsule(C2Capsule {
                a: rng.vec(),
                b: rng.vec(),
                r: rng.f32(0.0, 4.0),
            }),
            _ => unreachable!(),
        }
    }

    fn as_ptr(&self) -> *const c_void {
        match self {
            Self::Circle(value) => (value as *const C2Circle).cast(),
            Self::Aabb(value) => (value as *const C2Aabb).cast(),
            Self::Capsule(value) => (value as *const C2Capsule).cast(),
        }
    }
}

unsafe fn warm_cache(
    libs: &Libraries,
    shape_a: &Shape,
    type_a: c_int,
    ax: *const C2x,
    shape_b: &Shape,
    type_b: c_int,
    bx: *const C2x,
    use_radius: c_int,
    c_cache: &mut C2GjkCache,
    rust_cache: &mut C2GjkCache,
    context: &str,
) {
    let mut c_a = sentinel_vec();
    let mut c_b = sentinel_vec();
    let mut rust_a = sentinel_vec();
    let mut rust_b = sentinel_vec();
    let mut c_iterations = -99;
    let mut rust_iterations = -99;
    let c_distance = unsafe {
        (libs.c.c2_gjk)(
            shape_a.as_ptr(),
            type_a,
            ax,
            shape_b.as_ptr(),
            type_b,
            bx,
            &mut c_a,
            &mut c_b,
            use_radius,
            &mut c_iterations,
            c_cache,
        )
    };
    let rust_distance = unsafe {
        (libs.rust.c2_gjk)(
            shape_a.as_ptr(),
            type_a,
            ax,
            shape_b.as_ptr(),
            type_b,
            bx,
            &mut rust_a,
            &mut rust_b,
            use_radius,
            &mut rust_iterations,
            rust_cache,
        )
    };
    assert_same(
        &c_distance,
        &rust_distance,
        format!("{context} warm distance"),
    );
    assert_same(&c_a, &rust_a, format!("{context} warm outA"));
    assert_same(&c_b, &rust_b, format!("{context} warm outB"));
    assert_same(
        &c_iterations,
        &rust_iterations,
        format!("{context} warm iterations"),
    );
    assert_same(c_cache, rust_cache, format!("{context} warm cache"));
}

#[test]
fn gjk_configuration_cross_product_matches() {
    let libs = Libraries::load();
    let mut rng = Rng::new(0x5eed_c056_0000_0003);

    unsafe {
        for type_a in [CIRCLE, AABB, CAPSULE] {
            for type_b in [CIRCLE, AABB, CAPSULE] {
                for transform_mode in 0..4 {
                    for use_radius in [0, 1] {
                        for cache_mode in 0..3 {
                            for case in 0..RANDOM_CASES {
                                let shape_a = Shape::random(type_a, &mut rng);
                                let shape_b = Shape::random(type_b, &mut rng);
                                let ax_value = rng.transform();
                                let bx_value = rng.transform();
                                let ax = if transform_mode & 1 != 0 {
                                    &ax_value
                                } else {
                                    null()
                                };
                                let bx = if transform_mode & 2 != 0 {
                                    &bx_value
                                } else {
                                    null()
                                };
                                let context = format!(
                                    "c2GJK A={type_a} B={type_b} transforms={transform_mode} \
                                     radius={use_radius} cache={cache_mode} case={case}"
                                );

                                let mut c_cache = C2GjkCache::default();
                                let mut rust_cache = C2GjkCache::default();
                                if cache_mode == 2 {
                                    warm_cache(
                                        &libs,
                                        &shape_a,
                                        type_a,
                                        ax,
                                        &shape_b,
                                        type_b,
                                        bx,
                                        use_radius,
                                        &mut c_cache,
                                        &mut rust_cache,
                                        &context,
                                    );
                                }
                                let c_cache_ptr = if cache_mode == 0 {
                                    null_mut()
                                } else {
                                    &mut c_cache
                                };
                                let rust_cache_ptr = if cache_mode == 0 {
                                    null_mut()
                                } else {
                                    &mut rust_cache
                                };
                                let mut c_a = sentinel_vec();
                                let mut c_b = sentinel_vec();
                                let mut rust_a = sentinel_vec();
                                let mut rust_b = sentinel_vec();
                                let mut c_iterations = -99;
                                let mut rust_iterations = -99;
                                let c_distance = (libs.c.c2_gjk)(
                                    shape_a.as_ptr(),
                                    type_a,
                                    ax,
                                    shape_b.as_ptr(),
                                    type_b,
                                    bx,
                                    &mut c_a,
                                    &mut c_b,
                                    use_radius,
                                    &mut c_iterations,
                                    c_cache_ptr,
                                );
                                let rust_distance = (libs.rust.c2_gjk)(
                                    shape_a.as_ptr(),
                                    type_a,
                                    ax,
                                    shape_b.as_ptr(),
                                    type_b,
                                    bx,
                                    &mut rust_a,
                                    &mut rust_b,
                                    use_radius,
                                    &mut rust_iterations,
                                    rust_cache_ptr,
                                );
                                assert_same(
                                    &c_distance,
                                    &rust_distance,
                                    format!("{context} distance"),
                                );
                                assert_same(&c_a, &rust_a, format!("{context} outA"));
                                assert_same(&c_b, &rust_b, format!("{context} outB"));
                                assert_same(
                                    &c_iterations,
                                    &rust_iterations,
                                    format!("{context} iterations"),
                                );
                                if cache_mode != 0 {
                                    assert_same(&c_cache, &rust_cache, format!("{context} cache"));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn gjk_optional_outputs_and_wrapper_match() {
    let libs = Libraries::load();
    let mut rng = Rng::new(0x5eed_c272_0000_0004);

    unsafe {
        for mask in 0u32..16 {
            for case in 0..RANDOM_CASES {
                let shape_a = Shape::random(AABB, &mut rng);
                let shape_b = Shape::random(CAPSULE, &mut rng);
                let mut c_a = sentinel_vec();
                let mut c_b = sentinel_vec();
                let mut rust_a = sentinel_vec();
                let mut rust_b = sentinel_vec();
                let mut c_iterations = -99;
                let mut rust_iterations = -99;
                let mut c_cache = C2GjkCache::default();
                let mut rust_cache = C2GjkCache::default();
                let c_a_ptr = if mask & 1 != 0 { &mut c_a } else { null_mut() };
                let rust_a_ptr = if mask & 1 != 0 {
                    &mut rust_a
                } else {
                    null_mut()
                };
                let c_b_ptr = if mask & 2 != 0 { &mut c_b } else { null_mut() };
                let rust_b_ptr = if mask & 2 != 0 {
                    &mut rust_b
                } else {
                    null_mut()
                };
                let c_iterations_ptr = if mask & 4 != 0 {
                    &mut c_iterations
                } else {
                    null_mut()
                };
                let rust_iterations_ptr = if mask & 4 != 0 {
                    &mut rust_iterations
                } else {
                    null_mut()
                };
                let c_cache_ptr = if mask & 8 != 0 {
                    &mut c_cache
                } else {
                    null_mut()
                };
                let rust_cache_ptr = if mask & 8 != 0 {
                    &mut rust_cache
                } else {
                    null_mut()
                };
                let c_distance = (libs.c.c2_gjk)(
                    shape_a.as_ptr(),
                    AABB,
                    null(),
                    shape_b.as_ptr(),
                    CAPSULE,
                    null(),
                    c_a_ptr,
                    c_b_ptr,
                    1,
                    c_iterations_ptr,
                    c_cache_ptr,
                );
                let rust_distance = (libs.rust.c2_gjk)(
                    shape_a.as_ptr(),
                    AABB,
                    null(),
                    shape_b.as_ptr(),
                    CAPSULE,
                    null(),
                    rust_a_ptr,
                    rust_b_ptr,
                    1,
                    rust_iterations_ptr,
                    rust_cache_ptr,
                );
                let context = format!("c2GJK optional mask={mask:#x} case={case}");
                assert_same(&c_distance, &rust_distance, format!("{context} distance"));
                assert_same(&c_a, &rust_a, format!("{context} outA storage"));
                assert_same(&c_b, &rust_b, format!("{context} outB storage"));
                assert_same(
                    &c_iterations,
                    &rust_iterations,
                    format!("{context} iteration storage"),
                );
                assert_same(&c_cache, &rust_cache, format!("{context} cache storage"));
            }
        }

        for reverse in [0 as c_char, 1 as c_char, -1 as c_char] {
            for case in 0..RANDOM_CASES {
                let a1 = rng.f32(-20.0, 0.0);
                let a2 = rng.f32(-20.0, 0.0);
                let a3 = rng.f32(0.1, 20.0);
                let a4 = rng.f32(0.1, 20.0);
                let b1 = rng.f32(-20.0, 20.0);
                let b2 = rng.f32(-20.0, 20.0);
                let b3 = rng.f32(-20.0, 20.0);
                let b4 = rng.f32(-20.0, 20.0);
                let b5 = rng.f32(0.0, 5.0);
                let mut c_a = sentinel_vec();
                let mut c_b = sentinel_vec();
                let mut rust_a = sentinel_vec();
                let mut rust_b = sentinel_vec();
                (libs.c.gjk)(
                    reverse, &mut c_a, &mut c_b, a1, a2, a3, a4, b1, b2, b3, b4, b5,
                );
                (libs.rust.gjk)(
                    reverse,
                    &mut rust_a,
                    &mut rust_b,
                    a1,
                    a2,
                    a3,
                    a4,
                    b1,
                    b2,
                    b3,
                    b4,
                    b5,
                );
                assert_same(
                    &c_a,
                    &rust_a,
                    format!("gjk reverse={reverse} outA case={case}"),
                );
                assert_same(
                    &c_b,
                    &rust_b,
                    format!("gjk reverse={reverse} outB case={case}"),
                );
            }
        }
    }
}

#[test]
fn defined_boundary_behavior_matches() {
    let libs = Libraries::load();
    let mut rng = Rng::new(0x5eed_e001_0000_0005);

    unsafe {
        let circle = C2Circle {
            p: C2v { x: 2.0, y: -3.0 },
            r: 4.0,
        };
        for invalid_type in [-1, 3, c_int::MAX] {
            let mut c_proxy = sentinel_proxy();
            let mut rust_proxy = sentinel_proxy();
            let original = c_proxy;
            (libs.c.c2_make_proxy)(
                (&circle as *const C2Circle).cast(),
                invalid_type,
                &mut c_proxy,
            );
            (libs.rust.c2_make_proxy)(
                (&circle as *const C2Circle).cast(),
                invalid_type,
                &mut rust_proxy,
            );
            assert_same(
                &original,
                &c_proxy,
                format!("C c2MakeProxy invalid type={invalid_type} unchanged"),
            );
            assert_same(
                &c_proxy,
                &rust_proxy,
                format!("c2MakeProxy invalid type={invalid_type}"),
            );
        }

        let first = [C2v { x: 3.0, y: 4.0 }];
        for count in [0, -1] {
            let c_index = (libs.c.c2_support)(first.as_ptr(), count, C2v { x: 1.0, y: 1.0 });
            let rust_index = (libs.rust.c2_support)(first.as_ptr(), count, C2v { x: 1.0, y: 1.0 });
            assert_same(
                &c_index,
                &rust_index,
                format!("c2Support boundary count={count}"),
            );
            assert_eq!(c_index, 0);
        }
        let oversized = [
            C2v { x: 0.0, y: 0.0 },
            C2v { x: 1.0, y: 0.0 },
            C2v { x: 2.0, y: 0.0 },
            C2v { x: 3.0, y: 0.0 },
            C2v { x: 4.0, y: 0.0 },
            C2v { x: 5.0, y: 0.0 },
            C2v { x: 6.0, y: 0.0 },
            C2v { x: 7.0, y: 0.0 },
            C2v { x: 8.0, y: 0.0 },
        ];
        let c_index = (libs.c.c2_support)(oversized.as_ptr(), 9, C2v { x: 1.0, y: 0.0 });
        let rust_index = (libs.rust.c2_support)(oversized.as_ptr(), 9, C2v { x: 1.0, y: 0.0 });
        assert_same(&c_index, &rust_index, "c2Support oversized count=9");
        assert_eq!(c_index, 8);

        for count in [0, -1, 4, c_int::MAX] {
            let mut c_simplex = random_simplex(&mut rng, count);
            let mut rust_simplex = c_simplex;
            let c_metric = (libs.c.c2_metric)(&mut c_simplex);
            let rust_metric = (libs.rust.c2_metric)(&mut rust_simplex);
            assert_same(
                &c_metric,
                &rust_metric,
                format!("metric invalid count={count}"),
            );

            let c_direction = (libs.c.c2_d)(&mut c_simplex);
            let rust_direction = (libs.rust.c2_d)(&mut rust_simplex);
            assert_same(
                &c_direction,
                &rust_direction,
                format!("c2D invalid count={count}"),
            );

            let mut c_a = sentinel_vec();
            let mut c_b = sentinel_vec();
            let mut rust_a = sentinel_vec();
            let mut rust_b = sentinel_vec();
            (libs.c.c2_witness)(&mut c_simplex, &mut c_a, &mut c_b);
            (libs.rust.c2_witness)(&mut rust_simplex, &mut rust_a, &mut rust_b);
            assert_same(
                &c_a,
                &rust_a,
                format!("c2Witness invalid count={count} outA"),
            );
            assert_same(
                &c_b,
                &rust_b,
                format!("c2Witness invalid count={count} outB"),
            );

            let c_point = (libs.c.c2_l)(&mut c_simplex);
            let rust_point = (libs.rust.c2_l)(&mut rust_simplex);
            assert_same(&c_point, &rust_point, format!("c2L invalid count={count}"));
        }

        for divisor in [0.0f32, -0.0f32] {
            for vector in [C2v { x: 1.0, y: -1.0 }, C2v { x: 0.0, y: -0.0 }] {
                let c_value = (libs.c.c2_div)(vector, divisor);
                let rust_value = (libs.rust.c2_div)(vector, divisor);
                assert_same(
                    &c_value,
                    &rust_value,
                    format!("c2Div zero divisor bits={:#x}", divisor.to_bits()),
                );
            }
        }
        let zero = C2v { x: 0.0, y: 0.0 };
        let c_normal = (libs.c.c2_norm)(zero);
        let rust_normal = (libs.rust.c2_norm)(zero);
        assert_same(&c_normal, &rust_normal, "c2Norm zero vector");

        let shape_a = Shape::random(AABB, &mut rng);
        let shape_b = Shape::random(CAPSULE, &mut rng);
        let identity = C2x {
            p: C2v { x: 0.0, y: 0.0 },
            r: C2r { c: 1.0, s: 0.0 },
        };
        for (ax, bx, label) in [
            (null(), &identity as *const C2x, "null ax"),
            (&identity as *const C2x, null(), "null bx"),
            (null(), null(), "null ax and bx"),
        ] {
            let mut c_a = sentinel_vec();
            let mut c_b = sentinel_vec();
            let mut rust_a = sentinel_vec();
            let mut rust_b = sentinel_vec();
            let mut c_iterations = -99;
            let mut rust_iterations = -99;
            let mut c_cache = C2GjkCache::default();
            let mut rust_cache = C2GjkCache::default();
            let c_distance = (libs.c.c2_gjk)(
                shape_a.as_ptr(),
                AABB,
                ax,
                shape_b.as_ptr(),
                CAPSULE,
                bx,
                &mut c_a,
                &mut c_b,
                1,
                &mut c_iterations,
                &mut c_cache,
            );
            let rust_distance = (libs.rust.c2_gjk)(
                shape_a.as_ptr(),
                AABB,
                ax,
                shape_b.as_ptr(),
                CAPSULE,
                bx,
                &mut rust_a,
                &mut rust_b,
                1,
                &mut rust_iterations,
                &mut rust_cache,
            );
            assert_same(&c_distance, &rust_distance, format!("{label} distance"));
            assert_same(&c_a, &rust_a, format!("{label} outA"));
            assert_same(&c_b, &rust_b, format!("{label} outB"));
            assert_same(
                &c_iterations,
                &rust_iterations,
                format!("{label} iterations"),
            );
            assert_same(&c_cache, &rust_cache, format!("{label} cache"));
        }

        let args = (-4.0, -3.0, 7.0, 9.0, 2.0, -8.0, 5.0, 6.0, 1.5);
        for null_output in 0..2 {
            let mut c_a = sentinel_vec();
            let mut c_b = sentinel_vec();
            let mut rust_a = sentinel_vec();
            let mut rust_b = sentinel_vec();
            let c_a_ptr = if null_output == 0 {
                null_mut()
            } else {
                &mut c_a
            };
            let rust_a_ptr = if null_output == 0 {
                null_mut()
            } else {
                &mut rust_a
            };
            let c_b_ptr = if null_output == 1 {
                null_mut()
            } else {
                &mut c_b
            };
            let rust_b_ptr = if null_output == 1 {
                null_mut()
            } else {
                &mut rust_b
            };
            (libs.c.gjk)(
                0, c_a_ptr, c_b_ptr, args.0, args.1, args.2, args.3, args.4, args.5, args.6,
                args.7, args.8,
            );
            (libs.rust.gjk)(
                0, rust_a_ptr, rust_b_ptr, args.0, args.1, args.2, args.3, args.4, args.5, args.6,
                args.7, args.8,
            );
            assert_same(
                &c_a,
                &rust_a,
                format!("gjk null output={null_output} outA storage"),
            );
            assert_same(
                &c_b,
                &rust_b,
                format!("gjk null output={null_output} outB storage"),
            );
        }
    }
}
