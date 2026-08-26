use libloading::Library;
use std::ffi::{c_float, c_int, c_void};
use std::path::{Path, PathBuf};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct C2v {
    x: c_float,
    y: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct C2r {
    c: c_float,
    s: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct C2x {
    p: C2v,
    r: C2r,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct C2Circle {
    p: C2v,
    r: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct C2Aabb {
    min: C2v,
    max: C2v,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct C2Capsule {
    a: C2v,
    b: C2v,
    r: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct C2GjkCache {
    metric: c_float,
    count: c_int,
    i_a: [c_int; 3],
    i_b: [c_int; 3],
    div: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct C2Proxy {
    radius: c_float,
    count: c_int,
    verts: [C2v; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct C2sv {
    s_a: C2v,
    s_b: C2v,
    p: C2v,
    u: c_float,
    i_a: c_int,
    i_b: c_int,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct C2Simplex {
    a: C2sv,
    b: C2sv,
    c: C2sv,
    d: C2sv,
    div: c_float,
    count: c_int,
}

struct Libraries {
    c: Library,
    rust: Library,
}

impl Libraries {
    unsafe fn open() -> Self {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let c_path = root.join("c_src/build/libtranslated_rust.so");
        assert!(c_path.is_file(), "missing C library: {}", c_path.display());

        let exe = std::env::current_exe().expect("current test executable");
        let deps = exe.parent().expect("test deps directory");
        let candidates = [
            deps.join("libcapsule_lib.so"),
            deps.parent()
                .expect("target profile directory")
                .join("libcapsule_lib.so"),
            root.join("target/debug/libcapsule_lib.so"),
            root.join("target/release/libcapsule_lib.so"),
        ];
        let rust_path = candidates
            .into_iter()
            .find(|path| path.is_file())
            .unwrap_or_else(|| {
                panic!(
                    "missing Rust cdylib; searched from {}",
                    display_paths(deps, root)
                )
            });

        Self {
            c: unsafe { Library::new(c_path).expect("load C shared library") },
            rust: unsafe { Library::new(rust_path).expect("load Rust shared library") },
        }
    }

    unsafe fn pair<T: Copy>(&self, name: &[u8]) -> (T, T) {
        (
            *unsafe { self.c.get::<T>(name).expect("C symbol") },
            *unsafe { self.rust.get::<T>(name).expect("Rust symbol") },
        )
    }
}

fn display_paths(deps: &Path, root: &Path) -> String {
    let paths: [PathBuf; 4] = [
        deps.join("libcapsule_lib.so"),
        deps.parent().unwrap().join("libcapsule_lib.so"),
        root.join("target/debug/libcapsule_lib.so"),
        root.join("target/release/libcapsule_lib.so"),
    ];
    paths
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

fn bits(value: f32) -> u32 {
    value.to_bits()
}

fn assert_float_eq(c: f32, rust: f32, context: &str) {
    assert_eq!(
        bits(c),
        bits(rust),
        "{context}: C={c:?} ({:#010x}), Rust={rust:?} ({:#010x})",
        bits(c),
        bits(rust)
    );
}

fn assert_v_eq(c: C2v, rust: C2v, context: &str) {
    assert_float_eq(c.x, rust.x, &format!("{context}.x"));
    assert_float_eq(c.y, rust.y, &format!("{context}.y"));
}

fn assert_r_eq(c: C2r, rust: C2r, context: &str) {
    assert_float_eq(c.c, rust.c, &format!("{context}.c"));
    assert_float_eq(c.s, rust.s, &format!("{context}.s"));
}

fn assert_x_eq(c: C2x, rust: C2x, context: &str) {
    assert_v_eq(c.p, rust.p, &format!("{context}.p"));
    assert_r_eq(c.r, rust.r, &format!("{context}.r"));
}

fn assert_sv_eq(c: C2sv, rust: C2sv, context: &str) {
    assert_v_eq(c.s_a, rust.s_a, &format!("{context}.sA"));
    assert_v_eq(c.s_b, rust.s_b, &format!("{context}.sB"));
    assert_v_eq(c.p, rust.p, &format!("{context}.p"));
    assert_float_eq(c.u, rust.u, &format!("{context}.u"));
    assert_eq!(c.i_a, rust.i_a, "{context}.iA");
    assert_eq!(c.i_b, rust.i_b, "{context}.iB");
}

fn assert_simplex_eq(c: C2Simplex, rust: C2Simplex, context: &str) {
    assert_sv_eq(c.a, rust.a, &format!("{context}.a"));
    assert_sv_eq(c.b, rust.b, &format!("{context}.b"));
    assert_sv_eq(c.c, rust.c, &format!("{context}.c"));
    assert_sv_eq(c.d, rust.d, &format!("{context}.d"));
    assert_float_eq(c.div, rust.div, &format!("{context}.div"));
    assert_eq!(c.count, rust.count, "{context}.count");
}

fn assert_cache_eq(c: C2GjkCache, rust: C2GjkCache, context: &str) {
    assert_float_eq(c.metric, rust.metric, &format!("{context}.metric"));
    assert_eq!(c.count, rust.count, "{context}.count");
    assert_eq!(c.i_a, rust.i_a, "{context}.iA");
    assert_eq!(c.i_b, rust.i_b, "{context}.iB");
    assert_float_eq(c.div, rust.div, &format!("{context}.div"));
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u32(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 32) as u32
    }

    fn f32(&mut self) -> f32 {
        let unit = (self.next_u32() >> 8) as f32 / ((1u32 << 24) as f32);
        (unit * 200.0) - 100.0
    }

    fn nonzero(&mut self) -> f32 {
        let value = self.f32();
        if value == 0.0 { 1.0 } else { value }
    }

    fn v(&mut self) -> C2v {
        C2v {
            x: self.f32(),
            y: self.f32(),
        }
    }
}

type V2Fn = unsafe extern "C" fn(f32, f32) -> C2v;
type VScalarFn = unsafe extern "C" fn(C2v, f32) -> C2v;
type VVFn = unsafe extern "C" fn(C2v, C2v) -> C2v;
type VVVFn = unsafe extern "C" fn(C2v, C2v, C2v) -> C2v;
type VVFloatFn = unsafe extern "C" fn(C2v, C2v) -> f32;
type VFloatFn = unsafe extern "C" fn(C2v) -> f32;
type VFn = unsafe extern "C" fn(C2v) -> C2v;

#[test]
fn vector_transform_and_proxy_surface_rows_1_to_20_31_42_to_45_49() {
    unsafe {
        let libs = Libraries::open();
        let (c_v, r_v) = libs.pair::<V2Fn>(b"c2V\0");
        let (c_mulvs, r_mulvs) = libs.pair::<VScalarFn>(b"c2Mulvs\0");
        let (c_max, r_max) = libs.pair::<VVFn>(b"c2Maxv\0");
        let (c_min, r_min) = libs.pair::<VVFn>(b"c2Minv\0");
        let (c_clamp, r_clamp) = libs.pair::<VVVFn>(b"c2Clampv\0");
        let (c_sub, r_sub) = libs.pair::<VVFn>(b"c2Sub\0");
        let (c_dot, r_dot) = libs.pair::<VVFloatFn>(b"c2Dot\0");
        let (c_len, r_len) = libs.pair::<VFloatFn>(b"c2Len\0");
        let (c_det, r_det) = libs.pair::<VVFloatFn>(b"c2Det2\0");
        let (c_add, r_add) = libs.pair::<VVFn>(b"c2Add\0");
        let (c_neg, r_neg) = libs.pair::<VFn>(b"c2Neg\0");
        let (c_skew, r_skew) = libs.pair::<VFn>(b"c2Skew\0");
        let (c_ccw, r_ccw) = libs.pair::<VFn>(b"c2CCW90\0");
        let (c_div, r_div) = libs.pair::<VScalarFn>(b"c2Div\0");
        let (c_norm, r_norm) = libs.pair::<VFn>(b"c2Norm\0");

        let mut rng = Rng::new(0x5eed_0001);
        for i in 0..512 {
            let a = rng.v();
            let b = rng.v();
            let scalar = if i % 17 == 0 { 0.0 } else { rng.f32() };
            assert_v_eq(c_v(a.x, a.y), r_v(a.x, a.y), "row 1 c2V");
            assert_v_eq(c_mulvs(a, scalar), r_mulvs(a, scalar), "row 2 c2Mulvs");
            assert_v_eq(c_max(a, b), r_max(a, b), "row 3 c2Maxv");
            assert_v_eq(c_min(a, b), r_min(a, b), "row 4 c2Minv");
            assert_v_eq(c_sub(a, b), r_sub(a, b), "row 6 c2Sub");
            assert_float_eq(c_dot(a, b), r_dot(a, b), "row 7 c2Dot");
            assert_float_eq(c_len(a), r_len(a), "row 13 c2Len");
            assert_float_eq(c_det(a, b), r_det(a, b), "row 14 c2Det2");
            assert_v_eq(c_add(a, b), r_add(a, b), "row 19 c2Add");
            assert_v_eq(c_neg(a), r_neg(a), "row 31 c2Neg");
            assert_v_eq(c_skew(a), r_skew(a), "row 31 c2Skew");
            assert_v_eq(c_ccw(a), r_ccw(a), "row 31 c2CCW90");

            let divisor = rng.nonzero();
            assert_v_eq(c_div(a, divisor), r_div(a, divisor), "row 42 c2Div");
            assert_v_eq(c_norm(a), r_norm(a), "row 44 c2Norm");
        }

        let winners = [
            (C2v { x: 2.0, y: 2.0 }, C2v { x: 1.0, y: 1.0 }),
            (C2v { x: 2.0, y: 1.0 }, C2v { x: 1.0, y: 2.0 }),
            (C2v { x: 1.0, y: 2.0 }, C2v { x: 2.0, y: 1.0 }),
            (C2v { x: 1.0, y: 1.0 }, C2v { x: 2.0, y: 2.0 }),
            (C2v { x: 1.0, y: 1.0 }, C2v { x: 1.0, y: 1.0 }),
        ];
        for (a, b) in winners {
            assert_v_eq(c_max(a, b), r_max(a, b), "row 3 winner matrix");
            assert_v_eq(c_min(a, b), r_min(a, b), "row 4 winner matrix");
        }

        for x_state in [-2.0, 0.5, 3.0] {
            for y_state in [-2.0, 0.5, 3.0] {
                let value = C2v {
                    x: x_state,
                    y: y_state,
                };
                let lo = C2v { x: 0.0, y: 0.0 };
                let hi = C2v { x: 1.0, y: 1.0 };
                assert_v_eq(c_clamp(value, lo, hi), r_clamp(value, lo, hi), "row 5");
            }
        }

        for zero in [0.0f32, -0.0] {
            let input = C2v { x: 1.0, y: -1.0 };
            assert_v_eq(
                c_div(input, zero),
                r_div(input, zero),
                "row 43 zero divisor",
            );
        }
        assert_v_eq(
            c_norm(C2v::default()),
            r_norm(C2v::default()),
            "row 45 zero norm",
        );

        type RotIdentityFn = unsafe extern "C" fn() -> C2r;
        type XIdentityFn = unsafe extern "C" fn() -> C2x;
        type MulRvFn = unsafe extern "C" fn(C2r, C2v) -> C2v;
        type MulXvFn = unsafe extern "C" fn(C2x, C2v) -> C2v;
        let (c_rot_identity, r_rot_identity) = libs.pair::<RotIdentityFn>(b"c2RotIdentity\0");
        let (c_x_identity, r_x_identity) = libs.pair::<XIdentityFn>(b"c2xIdentity\0");
        let (c_mulrv, r_mulrv) = libs.pair::<MulRvFn>(b"c2Mulrv\0");
        let (c_mulxv, r_mulxv) = libs.pair::<MulXvFn>(b"c2Mulxv\0");
        let (c_mulrvt, r_mulrvt) = libs.pair::<MulRvFn>(b"c2MulrvT\0");
        assert_r_eq(
            c_rot_identity(),
            r_rot_identity(),
            "row 8 rotation identity",
        );
        assert_x_eq(c_x_identity(), r_x_identity(), "row 8 transform identity");
        for _ in 0..512 {
            let rot = C2r {
                c: rng.f32(),
                s: rng.f32(),
            };
            let transform = C2x { p: rng.v(), r: rot };
            let value = rng.v();
            assert_v_eq(c_mulrv(rot, value), r_mulrv(rot, value), "row 18");
            assert_v_eq(
                c_mulxv(transform, value),
                r_mulxv(transform, value),
                "row 20",
            );
            assert_v_eq(c_mulrvt(rot, value), r_mulrvt(rot, value), "row 49");
        }

        type BbVertsFn = unsafe extern "C" fn(*mut C2v, *mut C2Aabb);
        type MakeProxyFn = unsafe extern "C" fn(*const c_void, c_int, *mut C2Proxy);
        let (c_bb, r_bb) = libs.pair::<BbVertsFn>(b"c2BBVerts\0");
        let (c_proxy, r_proxy) = libs.pair::<MakeProxyFn>(b"c2MakeProxy\0");
        for _ in 0..256 {
            let aabb = C2Aabb {
                min: rng.v(),
                max: rng.v(),
            };
            let mut c_out = [C2v::default(); 4];
            let mut r_out = [C2v::default(); 4];
            c_bb(c_out.as_mut_ptr(), &aabb as *const _ as *mut _);
            r_bb(r_out.as_mut_ptr(), &aabb as *const _ as *mut _);
            for j in 0..4 {
                assert_v_eq(c_out[j], r_out[j], "row 9 c2BBVerts");
            }

            let circle = C2Circle {
                p: rng.v(),
                r: rng.f32(),
            };
            let capsule = C2Capsule {
                a: rng.v(),
                b: rng.v(),
                r: rng.f32(),
            };
            for (shape, kind, row) in [
                (&circle as *const C2Circle as *const c_void, 0, 10),
                (&aabb as *const C2Aabb as *const c_void, 1, 11),
                (&capsule as *const C2Capsule as *const c_void, 2, 12),
            ] {
                let sentinel = C2v {
                    x: 1234.0,
                    y: -4321.0,
                };
                let mut c_result = C2Proxy {
                    radius: 99.0,
                    count: -99,
                    verts: [sentinel; 8],
                };
                let mut r_result = c_result;
                c_proxy(shape, kind, &mut c_result);
                r_proxy(shape, kind, &mut r_result);
                assert_float_eq(
                    c_result.radius,
                    r_result.radius,
                    &format!("row {row} radius"),
                );
                assert_eq!(c_result.count, r_result.count, "row {row} count");
                for j in 0..8 {
                    assert_v_eq(
                        c_result.verts[j],
                        r_result.verts[j],
                        &format!("row {row} v{j}"),
                    );
                }
            }
        }
    }
}

fn random_sv(rng: &mut Rng) -> C2sv {
    C2sv {
        s_a: rng.v(),
        s_b: rng.v(),
        p: rng.v(),
        u: rng.f32(),
        i_a: (rng.next_u32() % 4) as i32,
        i_b: (rng.next_u32() % 4) as i32,
    }
}

fn random_simplex(rng: &mut Rng) -> C2Simplex {
    C2Simplex {
        a: random_sv(rng),
        b: random_sv(rng),
        c: random_sv(rng),
        d: random_sv(rng),
        div: rng.nonzero(),
        count: 0,
    }
}

fn sub(a: C2v, b: C2v) -> C2v {
    C2v {
        x: a.x - b.x,
        y: a.y - b.y,
    }
}

fn dot(a: C2v, b: C2v) -> f32 {
    a.x * b.x + a.y * b.y
}

fn det(a: C2v, b: C2v) -> f32 {
    a.x * b.y - a.y * b.x
}

fn c22_branch(simplex: &C2Simplex) -> usize {
    let a = simplex.a.p;
    let b = simplex.b.p;
    let u = dot(b, sub(b, a));
    let v = dot(a, sub(a, b));
    if v <= 0.0 {
        0
    } else if u <= 0.0 {
        1
    } else {
        2
    }
}

fn c23_branch(simplex: &C2Simplex) -> usize {
    let a = simplex.a.p;
    let b = simplex.b.p;
    let c = simplex.c.p;
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
fn simplex_and_support_surface_rows_15_to_17_21_to_30_32_to_41_46_to_48() {
    unsafe {
        let libs = Libraries::open();
        type SimplexFloatFn = unsafe extern "C" fn(*mut C2Simplex) -> f32;
        type SimplexVoidFn = unsafe extern "C" fn(*mut C2Simplex);
        type SimplexVFn = unsafe extern "C" fn(*mut C2Simplex) -> C2v;
        type SupportFn = unsafe extern "C" fn(*const C2v, c_int, C2v) -> c_int;
        type WitnessFn = unsafe extern "C" fn(*mut C2Simplex, *mut C2v, *mut C2v);
        let (c_metric, r_metric) = libs.pair::<SimplexFloatFn>(b"c2GJKSimplexMetric\0");
        let (c_22, r_22) = libs.pair::<SimplexVoidFn>(b"c22\0");
        let (c_23, r_23) = libs.pair::<SimplexVoidFn>(b"c23\0");
        let (c_d, r_d) = libs.pair::<SimplexVFn>(b"c2D\0");
        let (c_support, r_support) = libs.pair::<SupportFn>(b"c2Support\0");
        let (c_witness, r_witness) = libs.pair::<WitnessFn>(b"c2Witness\0");
        let (c_l, r_l) = libs.pair::<SimplexVFn>(b"c2L\0");

        let mut rng = Rng::new(0x5eed_0002);
        for count in [-7, 0, 1, 2, 3, 4, 99] {
            for _ in 0..256 {
                let mut c_simplex = random_simplex(&mut rng);
                c_simplex.count = count;
                let mut r_simplex = c_simplex;
                assert_float_eq(
                    c_metric(&mut c_simplex),
                    r_metric(&mut r_simplex),
                    &format!("rows 15-17 metric count {count}"),
                );
            }
        }

        let mut seen_22 = [false; 3];
        for _ in 0..4096 {
            let mut c_simplex = random_simplex(&mut rng);
            c_simplex.count = 2;
            let branch = c22_branch(&c_simplex);
            seen_22[branch] = true;
            let mut r_simplex = c_simplex;
            c_22(&mut c_simplex);
            r_22(&mut r_simplex);
            assert_simplex_eq(c_simplex, r_simplex, &format!("rows 21-23 branch {branch}"));
        }
        assert_eq!(seen_22, [true; 3], "all c22 branches must be sampled");

        let mut seen_23 = [false; 7];
        for _ in 0..20_000 {
            let mut c_simplex = random_simplex(&mut rng);
            c_simplex.count = 3;
            let branch = c23_branch(&c_simplex);
            seen_23[branch] = true;
            let mut r_simplex = c_simplex;
            c_23(&mut c_simplex);
            r_23(&mut r_simplex);
            assert_simplex_eq(c_simplex, r_simplex, &format!("rows 24-30 branch {branch}"));
        }
        assert_eq!(seen_23, [true; 7], "all c23 branches must be sampled");

        for count in [-1, 0, 1, 2, 3, 4] {
            for i in 0..512 {
                let mut c_simplex = random_simplex(&mut rng);
                c_simplex.count = count;
                if count == 2 && i % 2 == 0 {
                    c_simplex.a.p = C2v { x: 1.0, y: 0.0 };
                    c_simplex.b.p = C2v { x: 1.0, y: 1.0 };
                } else if count == 2 {
                    c_simplex.a.p = C2v { x: -1.0, y: 0.0 };
                    c_simplex.b.p = C2v { x: -1.0, y: 1.0 };
                }
                let mut r_simplex = c_simplex;
                assert_v_eq(
                    c_d(&mut c_simplex),
                    r_d(&mut r_simplex),
                    &format!("rows 32-35 c2D count {count}"),
                );

                let mut c_a = C2v { x: 9.0, y: 9.0 };
                let mut c_b = C2v { x: 8.0, y: 8.0 };
                let mut r_a = c_a;
                let mut r_b = c_b;
                c_witness(&mut c_simplex, &mut c_a, &mut c_b);
                r_witness(&mut r_simplex, &mut r_a, &mut r_b);
                assert_v_eq(c_a, r_a, &format!("rows 38-41 witness A count {count}"));
                assert_v_eq(c_b, r_b, &format!("rows 38-41 witness B count {count}"));

                assert_v_eq(
                    c_l(&mut c_simplex),
                    r_l(&mut r_simplex),
                    &format!("rows 46-48 c2L count {count}"),
                );
            }
        }

        let one = [C2v { x: 4.0, y: -2.0 }];
        for count in [0, 1] {
            for _ in 0..64 {
                let direction = rng.v();
                assert_eq!(
                    c_support(one.as_ptr(), count, direction),
                    r_support(one.as_ptr(), count, direction),
                    "row 36 support count {count}"
                );
            }
        }

        for iteration in 0..1024 {
            let mut vertices = [C2v::default(); 8];
            for vertex in &mut vertices {
                *vertex = rng.v();
            }
            let direction = if iteration % 17 == 0 {
                C2v::default()
            } else {
                rng.v()
            };
            assert_eq!(
                c_support(vertices.as_ptr(), vertices.len() as c_int, direction),
                r_support(vertices.as_ptr(), vertices.len() as c_int, direction),
                "row 37 support many"
            );
        }

        let tie_vertices = [
            C2v { x: 1.0, y: 0.0 },
            C2v { x: 1.0, y: 5.0 },
            C2v { x: 0.0, y: 100.0 },
        ];
        let direction = C2v { x: 1.0, y: 0.0 };
        assert_eq!(
            c_support(tie_vertices.as_ptr(), 3, direction),
            r_support(tie_vertices.as_ptr(), 3, direction),
            "row 37 first maximum wins ties"
        );

        let mut oversized = vec![C2v::default(); 257];
        for vertex in &mut oversized {
            *vertex = rng.v();
        }
        for _ in 0..128 {
            let direction = rng.v();
            assert_eq!(
                c_support(oversized.as_ptr(), oversized.len() as c_int, direction),
                r_support(oversized.as_ptr(), oversized.len() as c_int, direction),
                "row 37 caller-backed oversized count"
            );
        }
    }
}

#[derive(Clone, Copy)]
enum Shape {
    Circle(C2Circle),
    Aabb(C2Aabb),
    Capsule(C2Capsule),
}

impl Shape {
    fn kind(self) -> c_int {
        match self {
            Self::Circle(_) => 0,
            Self::Aabb(_) => 1,
            Self::Capsule(_) => 2,
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

fn random_shapes(rng: &mut Rng) -> [Shape; 3] {
    let p = rng.v();
    let extent = C2v {
        x: rng.f32().abs() + 0.01,
        y: rng.f32().abs() + 0.01,
    };
    [
        Shape::Circle(C2Circle {
            p: rng.v(),
            r: rng.f32().abs(),
        }),
        Shape::Aabb(C2Aabb {
            min: p,
            max: C2v {
                x: p.x + extent.x,
                y: p.y + extent.y,
            },
        }),
        Shape::Capsule(C2Capsule {
            a: rng.v(),
            b: rng.v(),
            r: rng.f32().abs(),
        }),
    ]
}

type GjkFn = unsafe extern "C" fn(
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

unsafe fn compare_gjk(
    c_gjk: GjkFn,
    r_gjk: GjkFn,
    a: &Shape,
    b: &Shape,
    ax: Option<&C2x>,
    bx: Option<&C2x>,
    use_radius: c_int,
    outputs: u8,
    c_cache: Option<&mut C2GjkCache>,
    r_cache: Option<&mut C2GjkCache>,
    context: &str,
) {
    let mut c_out_a = C2v {
        x: 12345.0,
        y: -12345.0,
    };
    let mut r_out_a = c_out_a;
    let mut c_out_b = C2v {
        x: 23456.0,
        y: -23456.0,
    };
    let mut r_out_b = c_out_b;
    let mut c_iterations = -777;
    let mut r_iterations = -777;
    let ax_ptr = ax.map_or(std::ptr::null(), |value| value);
    let bx_ptr = bx.map_or(std::ptr::null(), |value| value);
    let c_cache_ptr = c_cache.map_or(std::ptr::null_mut(), |value| value);
    let r_cache_ptr = r_cache.map_or(std::ptr::null_mut(), |value| value);
    let c_distance = unsafe {
        c_gjk(
            a.as_ptr(),
            a.kind(),
            ax_ptr,
            b.as_ptr(),
            b.kind(),
            bx_ptr,
            if outputs & 1 != 0 {
                &mut c_out_a
            } else {
                std::ptr::null_mut()
            },
            if outputs & 2 != 0 {
                &mut c_out_b
            } else {
                std::ptr::null_mut()
            },
            use_radius,
            if outputs & 4 != 0 {
                &mut c_iterations
            } else {
                std::ptr::null_mut()
            },
            c_cache_ptr,
        )
    };
    let r_distance = unsafe {
        r_gjk(
            a.as_ptr(),
            a.kind(),
            ax_ptr,
            b.as_ptr(),
            b.kind(),
            bx_ptr,
            if outputs & 1 != 0 {
                &mut r_out_a
            } else {
                std::ptr::null_mut()
            },
            if outputs & 2 != 0 {
                &mut r_out_b
            } else {
                std::ptr::null_mut()
            },
            use_radius,
            if outputs & 4 != 0 {
                &mut r_iterations
            } else {
                std::ptr::null_mut()
            },
            r_cache_ptr,
        )
    };
    assert_float_eq(c_distance, r_distance, &format!("{context}.distance"));
    assert_v_eq(c_out_a, r_out_a, &format!("{context}.outA"));
    assert_v_eq(c_out_b, r_out_b, &format!("{context}.outB"));
    assert_eq!(c_iterations, r_iterations, "{context}.iterations");
}

#[test]
fn gjk_surface_rows_50_to_55() {
    unsafe {
        let libs = Libraries::open();
        let (c_gjk, r_gjk) = libs.pair::<GjkFn>(b"c2GJK\0");
        let mut rng = Rng::new(0x5eed_0003);

        for iteration in 0..128 {
            let a_shapes = random_shapes(&mut rng);
            let b_shapes = random_shapes(&mut rng);
            for a in &a_shapes {
                for b in &b_shapes {
                    for use_radius in [0, 1, -7] {
                        compare_gjk(
                            c_gjk,
                            r_gjk,
                            a,
                            b,
                            None,
                            None,
                            use_radius,
                            7,
                            None,
                            None,
                            &format!(
                                "rows 50-51 iteration {iteration} types {}-{} radius {use_radius}",
                                a.kind(),
                                b.kind()
                            ),
                        );
                    }
                }
            }
        }

        for iteration in 0..32 {
            let a_shapes = random_shapes(&mut rng);
            let b_shapes = random_shapes(&mut rng);
            let ax = C2x {
                p: rng.v(),
                r: C2r {
                    c: rng.f32(),
                    s: rng.f32(),
                },
            };
            let bx = C2x {
                p: rng.v(),
                r: C2r {
                    c: rng.f32(),
                    s: rng.f32(),
                },
            };
            for a in &a_shapes {
                for b in &b_shapes {
                    for (a_transform, b_transform) in
                        [(Some(&ax), None), (None, Some(&bx)), (Some(&ax), Some(&bx))]
                    {
                        compare_gjk(
                            c_gjk,
                            r_gjk,
                            a,
                            b,
                            a_transform,
                            b_transform,
                            (iteration & 1) as i32,
                            7,
                            None,
                            None,
                            "row 52 explicit transforms",
                        );
                    }
                }
            }
        }

        let shapes_a = random_shapes(&mut rng);
        let shapes_b = random_shapes(&mut rng);
        for outputs in 0..8 {
            compare_gjk(
                c_gjk,
                r_gjk,
                &shapes_a[2],
                &shapes_b[1],
                None,
                None,
                1,
                outputs,
                None,
                None,
                &format!("row 53 optional output mask {outputs}"),
            );
        }

        for a in &shapes_a {
            for b in &shapes_b {
                let mut c_cache = C2GjkCache::default();
                let mut r_cache = C2GjkCache::default();
                compare_gjk(
                    c_gjk,
                    r_gjk,
                    a,
                    b,
                    None,
                    None,
                    1,
                    7,
                    Some(&mut c_cache),
                    Some(&mut r_cache),
                    "row 54 empty cache",
                );
                assert_cache_eq(c_cache, r_cache, "row 54 populated cache");
                compare_gjk(
                    c_gjk,
                    r_gjk,
                    a,
                    b,
                    None,
                    None,
                    1,
                    7,
                    Some(&mut c_cache),
                    Some(&mut r_cache),
                    "row 54 reused cache",
                );
                assert_cache_eq(c_cache, r_cache, "row 54 reused cache result");
            }
        }

        let large_a = Shape::Aabb(C2Aabb {
            min: C2v { x: 0.0, y: 0.0 },
            max: C2v {
                x: 20_000.0,
                y: 20_000.0,
            },
        });
        let large_b = Shape::Aabb(C2Aabb {
            min: C2v {
                x: -5_000.0,
                y: -10_000.0,
            },
            max: C2v {
                x: 15_000.0,
                y: 10_000.0,
            },
        });
        let seed_cache = C2GjkCache {
            metric: -300_000_000.0,
            count: 3,
            i_a: [0, 1, 2],
            i_b: [2, 0, 1],
            div: 1.0,
        };
        let mut c_cache = seed_cache;
        let mut r_cache = seed_cache;
        compare_gjk(
            c_gjk,
            r_gjk,
            &large_a,
            &large_b,
            None,
            None,
            0,
            7,
            Some(&mut c_cache),
            Some(&mut r_cache),
            "rows 54-55 metric-stressed cache and loop exits",
        );
        assert_cache_eq(c_cache, r_cache, "rows 54-55 stressed cache result");
    }
}

#[test]
fn collision_and_capsule_surface_rows_56_to_66() {
    unsafe {
        let libs = Libraries::open();
        type AabbAabbFn = unsafe extern "C" fn(C2Aabb, C2Aabb) -> c_int;
        type AabbCapsuleFn = unsafe extern "C" fn(C2Aabb, C2Capsule) -> c_int;
        type CapsuleCapsuleFn = unsafe extern "C" fn(C2Capsule, C2Capsule) -> c_int;
        type CircleCircleFn = unsafe extern "C" fn(C2Circle, C2Circle) -> c_int;
        type CircleAabbFn = unsafe extern "C" fn(C2Circle, C2Aabb) -> c_int;
        type CircleCapsuleFn = unsafe extern "C" fn(C2Circle, C2Capsule) -> c_int;
        type CollidedFn = unsafe extern "C" fn(*const c_void, c_int, *const c_void, c_int) -> c_int;
        type CapsuleFn = unsafe extern "C" fn(f32, f32, f32, f32, f32) -> c_int;
        let (c_aa, r_aa) = libs.pair::<AabbAabbFn>(b"c2AABBtoAABB\0");
        let (c_ac, r_ac) = libs.pair::<AabbCapsuleFn>(b"c2AABBtoCapsule\0");
        let (c_ccap, r_ccap) = libs.pair::<CapsuleCapsuleFn>(b"c2CapsuletoCapsule\0");
        let (c_cc, r_cc) = libs.pair::<CircleCircleFn>(b"c2CircletoCircle\0");
        let (c_ca, r_ca) = libs.pair::<CircleAabbFn>(b"c2CircletoAABB\0");
        let (c_c_cap, r_c_cap) = libs.pair::<CircleCapsuleFn>(b"c2CircletoCapsule\0");
        let (c_collided, r_collided) = libs.pair::<CollidedFn>(b"c2Collided\0");
        let (c_capsule, r_capsule) = libs.pair::<CapsuleFn>(b"capsule\0");

        let unit_box = C2Aabb {
            min: C2v { x: 0.0, y: 0.0 },
            max: C2v { x: 1.0, y: 1.0 },
        };
        let aabb_cases = [
            unit_box,
            C2Aabb {
                min: C2v { x: 0.5, y: 0.5 },
                max: C2v { x: 2.0, y: 2.0 },
            },
            C2Aabb {
                min: C2v { x: 1.0, y: 0.0 },
                max: C2v { x: 2.0, y: 1.0 },
            },
            C2Aabb {
                min: C2v { x: -2.0, y: 0.0 },
                max: C2v { x: -1.0, y: 1.0 },
            },
            C2Aabb {
                min: C2v { x: 2.0, y: 0.0 },
                max: C2v { x: 3.0, y: 1.0 },
            },
            C2Aabb {
                min: C2v { x: 0.0, y: -2.0 },
                max: C2v { x: 1.0, y: -1.0 },
            },
            C2Aabb {
                min: C2v { x: 0.0, y: 2.0 },
                max: C2v { x: 1.0, y: 3.0 },
            },
        ];
        for case in aabb_cases {
            assert_eq!(c_aa(unit_box, case), r_aa(unit_box, case), "rows 56-57");
        }

        let capsule_cases = [
            C2Capsule {
                a: C2v { x: 3.0, y: 0.5 },
                b: C2v { x: 4.0, y: 0.5 },
                r: 0.25,
            },
            C2Capsule {
                a: C2v { x: 0.25, y: 0.5 },
                b: C2v { x: 0.75, y: 0.5 },
                r: 0.25,
            },
            C2Capsule {
                a: C2v { x: 2.0, y: 0.5 },
                b: C2v { x: 3.0, y: 0.5 },
                r: 1.0,
            },
        ];
        for capsule in capsule_cases {
            assert_eq!(c_ac(unit_box, capsule), r_ac(unit_box, capsule), "row 58");
        }
        for a in capsule_cases {
            for b in capsule_cases {
                assert_eq!(c_ccap(a, b), r_ccap(a, b), "row 59");
            }
        }

        let base_circle = C2Circle {
            p: C2v { x: 0.0, y: 0.0 },
            r: 1.0,
        };
        for other in [
            C2Circle {
                p: C2v { x: 3.0, y: 0.0 },
                r: 1.0,
            },
            C2Circle {
                p: C2v { x: 1.0, y: 0.0 },
                r: 1.0,
            },
            C2Circle {
                p: C2v { x: 2.0, y: 0.0 },
                r: 1.0,
            },
        ] {
            assert_eq!(c_cc(base_circle, other), r_cc(base_circle, other), "row 60");
        }

        for circle in [
            C2Circle {
                p: C2v { x: 0.5, y: 0.5 },
                r: 0.1,
            },
            C2Circle {
                p: C2v { x: 2.0, y: 0.5 },
                r: 0.5,
            },
            C2Circle {
                p: C2v { x: 1.5, y: 0.5 },
                r: 0.5,
            },
            C2Circle {
                p: C2v { x: 2.0, y: 2.0 },
                r: 2.0f32.sqrt(),
            },
        ] {
            assert_eq!(c_ca(circle, unit_box), r_ca(circle, unit_box), "row 61");
        }

        let segment = C2Capsule {
            a: C2v { x: 0.0, y: 0.0 },
            b: C2v { x: 10.0, y: 0.0 },
            r: 1.0,
        };
        for x in [-2.0, 5.0, 12.0] {
            for offset in [0.0, 1.5, 2.0, 3.0] {
                let circle = C2Circle {
                    p: C2v { x, y: offset },
                    r: 1.0,
                };
                assert_eq!(
                    c_c_cap(circle, segment),
                    r_c_cap(circle, segment),
                    "rows 62-64 x={x} offset={offset}"
                );
            }
        }

        let mut rng = Rng::new(0x5eed_0004);
        for iteration in 0..1024 {
            let a_shapes = random_shapes(&mut rng);
            let b_shapes = random_shapes(&mut rng);

            let circle_a = match a_shapes[0] {
                Shape::Circle(value) => value,
                _ => unreachable!(),
            };
            let circle_b = match b_shapes[0] {
                Shape::Circle(value) => value,
                _ => unreachable!(),
            };
            let aabb_a = match a_shapes[1] {
                Shape::Aabb(value) => value,
                _ => unreachable!(),
            };
            let aabb_b = match b_shapes[1] {
                Shape::Aabb(value) => value,
                _ => unreachable!(),
            };
            let capsule_a = match a_shapes[2] {
                Shape::Capsule(value) => value,
                _ => unreachable!(),
            };
            let capsule_b = match b_shapes[2] {
                Shape::Capsule(value) => value,
                _ => unreachable!(),
            };
            assert_eq!(
                c_aa(aabb_a, aabb_b),
                r_aa(aabb_a, aabb_b),
                "rows 56-57 random"
            );
            assert_eq!(
                c_ac(aabb_a, capsule_b),
                r_ac(aabb_a, capsule_b),
                "row 58 random"
            );
            assert_eq!(
                c_ccap(capsule_a, capsule_b),
                r_ccap(capsule_a, capsule_b),
                "row 59 random"
            );
            assert_eq!(
                c_cc(circle_a, circle_b),
                r_cc(circle_a, circle_b),
                "row 60 random"
            );
            assert_eq!(
                c_ca(circle_a, aabb_b),
                r_ca(circle_a, aabb_b),
                "row 61 random"
            );
            assert_eq!(
                c_c_cap(circle_a, capsule_b),
                r_c_cap(circle_a, capsule_b),
                "rows 62-64 random"
            );

            for a in &a_shapes {
                for b in &b_shapes {
                    assert_eq!(
                        c_collided(a.as_ptr(), a.kind(), b.as_ptr(), b.kind()),
                        r_collided(a.as_ptr(), a.kind(), b.as_ptr(), b.kind()),
                        "row 65 iteration {iteration} types {}-{}",
                        a.kind(),
                        b.kind()
                    );
                }
            }
        }

        let targeted_capsules = [
            (-70.0, 0.0, -70.0, 0.0, 1.0),
            (-30.0, -30.0, -30.0, -30.0, 1.0),
            (-30.0, 70.0, -30.0, 70.0, 1.0),
            (1000.0, 1000.0, 1001.0, 1001.0, 1.0),
            (-70.0, 0.0, -30.0, -30.0, 2.0),
            (-70.0, 0.0, -30.0, 70.0, 2.0),
            (-30.0, -30.0, -30.0, 70.0, 2.0),
            (-30.0, 20.0, -30.0, 20.0, 100.0),
        ];
        let mut seen_masks = [false; 8];
        for (min_x, min_y, max_x, max_y, radius) in targeted_capsules {
            let c_result = c_capsule(min_x, min_y, max_x, max_y, radius);
            let r_result = r_capsule(min_x, min_y, max_x, max_y, radius);
            assert_eq!(c_result, r_result, "row 66 targeted capsule");
            seen_masks[c_result as usize] = true;
        }
        for _ in 0..20_000 {
            let args = (rng.f32(), rng.f32(), rng.f32(), rng.f32(), rng.f32().abs());
            let c_result = c_capsule(args.0, args.1, args.2, args.3, args.4);
            let r_result = r_capsule(args.0, args.1, args.2, args.3, args.4);
            assert_eq!(c_result, r_result, "row 66 randomized capsule");
            seen_masks[c_result as usize] = true;
        }
        assert!(
            seen_masks.iter().filter(|seen| **seen).count() >= 4,
            "row 66 should cover multiple bitmask interactions: {seen_masks:?}"
        );
    }
}

#[test]
fn error_surface_rows_1_to_5_invalid_enums() {
    unsafe {
        let libs = Libraries::open();
        type MakeProxyFn = unsafe extern "C" fn(*const c_void, c_int, *mut C2Proxy);
        type CollidedFn = unsafe extern "C" fn(*const c_void, c_int, *const c_void, c_int) -> c_int;
        let (c_proxy, r_proxy) = libs.pair::<MakeProxyFn>(b"c2MakeProxy\0");
        let (c_collided, r_collided) = libs.pair::<CollidedFn>(b"c2Collided\0");

        for invalid in [-2, -1, 3, 4, c_int::MAX] {
            let sentinel = C2v {
                x: 1234.0,
                y: -4321.0,
            };
            let initial = C2Proxy {
                radius: 99.0,
                count: -99,
                verts: [sentinel; 8],
            };
            let mut c_out = initial;
            let mut r_out = initial;
            c_proxy(std::ptr::null(), invalid, &mut c_out);
            r_proxy(std::ptr::null(), invalid, &mut r_out);
            assert_float_eq(c_out.radius, r_out.radius, "error row 1 radius");
            assert_eq!(c_out.count, r_out.count, "error row 1 count");
            for index in 0..8 {
                assert_v_eq(c_out.verts[index], r_out.verts[index], "error row 1 vertex");
            }

            c_proxy(std::ptr::null(), invalid, std::ptr::null_mut());
            r_proxy(std::ptr::null(), invalid, std::ptr::null_mut());

            assert_eq!(
                c_collided(std::ptr::null(), invalid, std::ptr::null(), invalid),
                r_collided(std::ptr::null(), invalid, std::ptr::null(), invalid),
                "error row 2"
            );
            for valid_a in 0..=2 {
                assert_eq!(
                    c_collided(std::ptr::null(), valid_a, std::ptr::null(), invalid),
                    r_collided(std::ptr::null(), valid_a, std::ptr::null(), invalid),
                    "error rows 3-5 typeA={valid_a}"
                );
            }
        }
    }
}
