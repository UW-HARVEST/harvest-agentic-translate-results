#![allow(non_snake_case)]

use libloading::Library;
use std::ffi::{c_float, c_int, c_void};
use std::mem::size_of;
use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::process::Command;

unsafe extern "C" {
    fn free(pointer: *mut c_void);
}

const CAPSULE: c_int = 0;
const CIRCLE: c_int = 1;
const AABB: c_int = 2;
const INVALID: c_int = 99;
const CASES: usize = 128;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct V {
    x: c_float,
    y: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct R {
    c: c_float,
    s: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct X {
    p: V,
    r: R,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct Circle {
    p: V,
    r: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct Aabb {
    min: V,
    max: V,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct Capsule {
    a: V,
    b: V,
    r: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct Cache {
    metric: c_float,
    count: c_int,
    iA: [c_int; 3],
    iB: [c_int; 3],
    div: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct Proxy {
    radius: c_float,
    count: c_int,
    verts: [V; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct Sv {
    sA: V,
    sB: V,
    p: V,
    u: c_float,
    iA: c_int,
    iB: c_int,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct Simplex {
    a: Sv,
    b: Sv,
    c: Sv,
    d: Sv,
    div: c_float,
    count: c_int,
}

type VFn = unsafe extern "C" fn(c_float, c_float) -> V;
type VScalarFn = unsafe extern "C" fn(V, c_float) -> V;
type VVFn = unsafe extern "C" fn(V, V) -> V;
type VVVFn = unsafe extern "C" fn(V, V, V) -> V;
type VFloatFn = unsafe extern "C" fn(V) -> c_float;
type VVFloatFn = unsafe extern "C" fn(V, V) -> c_float;
type VUnaryFn = unsafe extern "C" fn(V) -> V;
type RFn = unsafe extern "C" fn() -> R;
type XFn = unsafe extern "C" fn() -> X;
type RVFn = unsafe extern "C" fn(R, V) -> V;
type XVFn = unsafe extern "C" fn(X, V) -> V;
type BBVertsFn = unsafe extern "C" fn(*mut V, *mut Aabb);
type MakeProxyFn = unsafe extern "C" fn(*const c_void, c_int, *mut Proxy);
type SimplexFloatFn = unsafe extern "C" fn(*mut Simplex) -> c_float;
type SimplexVoidFn = unsafe extern "C" fn(*mut Simplex);
type SimplexVFn = unsafe extern "C" fn(*mut Simplex) -> V;
type SupportFn = unsafe extern "C" fn(*const V, c_int, V) -> c_int;
type WitnessFn = unsafe extern "C" fn(*mut Simplex, *mut V, *mut V);
type GjKFn = unsafe extern "C" fn(
    *const c_void,
    c_int,
    *const X,
    *const c_void,
    c_int,
    *const X,
    *mut V,
    *mut V,
    c_int,
    *mut c_int,
    *mut Cache,
) -> c_float;
type AabbAabbFn = unsafe extern "C" fn(Aabb, Aabb) -> c_int;
type AabbCapsuleFn = unsafe extern "C" fn(Aabb, Capsule) -> c_int;
type CapsuleCapsuleFn = unsafe extern "C" fn(Capsule, Capsule) -> c_int;
type CircleCircleFn = unsafe extern "C" fn(Circle, Circle) -> c_int;
type CircleAabbFn = unsafe extern "C" fn(Circle, Aabb) -> c_int;
type CircleCapsuleFn = unsafe extern "C" fn(Circle, Capsule) -> c_int;
type CollidedFn = unsafe extern "C" fn(*const c_void, c_int, *const c_void, c_int) -> c_int;
type PartsFn =
    unsafe extern "C" fn(c_int, c_float, c_float, c_float, c_float, c_float) -> *mut c_void;
type OmniFn = unsafe extern "C" fn(
    c_int,
    c_float,
    c_float,
    c_float,
    c_float,
    c_float,
    c_int,
    c_float,
    c_float,
    c_float,
    c_float,
    c_float,
) -> c_int;

struct Libs {
    c: Library,
    rust: Library,
}

impl Libs {
    unsafe fn load() -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_path = root.join("c_src/build/libtranslated_rust.so");
        assert!(
            c_path.exists(),
            "missing C shared object: {}",
            c_path.display()
        );

        let test_exe = std::env::current_exe().expect("test executable path");
        let deps = test_exe.parent().expect("test deps directory");
        let candidates = [
            deps.join("libomni_collide_lib.so"),
            root.join("target/debug/libomni_collide_lib.so"),
            root.join("target/release/libomni_collide_lib.so"),
        ];
        let rust_path = candidates
            .into_iter()
            .find(|path| path.exists())
            .expect("missing Rust shared object");

        Self {
            c: unsafe { Library::new(c_path).expect("load C shared object") },
            rust: unsafe { Library::new(rust_path).expect("load Rust shared object") },
        }
    }

    unsafe fn pair<T: Copy>(&self, name: &[u8]) -> (T, T) {
        let c = unsafe { *self.c.get::<T>(name).unwrap() };
        let rust = unsafe { *self.rust.get::<T>(name).unwrap() };
        (c, rust)
    }
}

#[derive(Clone, Copy)]
struct Rng(u64);

impl Rng {
    fn new() -> Self {
        Self(0x6a09_e667_f3bc_c909)
    }

    fn u32(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0 as u32
    }

    fn finite(&mut self) -> f32 {
        let sign = self.u32() & 0x8000_0000;
        let exponent = ((self.u32() % 253) + 1) << 23;
        let fraction = self.u32() & 0x007f_ffff;
        f32::from_bits(sign | exponent | fraction)
    }

    fn moderate(&mut self) -> f32 {
        (self.u32() as i32 % 200_001) as f32 / 100.0
    }

    fn v(&mut self) -> V {
        V {
            x: self.moderate(),
            y: self.moderate(),
        }
    }
}

fn bytes<T>(value: &T) -> &[u8] {
    unsafe { std::slice::from_raw_parts((value as *const T).cast(), size_of::<T>()) }
}

fn eq_bytes<T>(context: &str, c: &T, rust: &T) {
    assert_eq!(bytes(c), bytes(rust), "{context}");
}

fn eq_f32(context: &str, c: f32, rust: f32) {
    assert_eq!(
        c.to_bits(),
        rust.to_bits(),
        "{context}: C={c:?}, Rust={rust:?}"
    );
}

fn special_floats() -> [f32; 10] {
    [
        0.0,
        -0.0,
        1.0,
        -1.0,
        f32::MIN_POSITIVE,
        f32::MAX,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::from_bits(0x7fc0_1234),
        f32::from_bits(0xffc0_5678),
    ]
}

fn shape_bytes(typ: c_int, values: [f32; 5]) -> Vec<u8> {
    match typ {
        CIRCLE => bytes(&Circle {
            p: V {
                x: values[0],
                y: values[1],
            },
            r: values[2],
        })
        .to_vec(),
        AABB => bytes(&Aabb {
            min: V {
                x: values[0],
                y: values[1],
            },
            max: V {
                x: values[2],
                y: values[3],
            },
        })
        .to_vec(),
        CAPSULE => bytes(&Capsule {
            a: V {
                x: values[0],
                y: values[1],
            },
            b: V {
                x: values[2],
                y: values[3],
            },
            r: values[4],
        })
        .to_vec(),
        _ => unreachable!(),
    }
}

fn as_ptr(bytes: &[u8]) -> *const c_void {
    bytes.as_ptr().cast()
}

fn random_shape_values(rng: &mut Rng, typ: c_int) -> [f32; 5] {
    let x = rng.moderate() / 10.0;
    let y = rng.moderate() / 10.0;
    match typ {
        CIRCLE => [x, y, (rng.moderate().abs() % 20.0) + 0.01, 0.0, 0.0],
        AABB => {
            let w = (rng.moderate().abs() % 30.0) + 0.01;
            let h = (rng.moderate().abs() % 30.0) + 0.01;
            [x, y, x + w, y + h, 0.0]
        }
        CAPSULE => [
            x,
            y,
            x + (rng.moderate() % 30.0),
            y + (rng.moderate() % 30.0),
            (rng.moderate().abs() % 20.0) + 0.01,
        ],
        _ => unreachable!(),
    }
}

unsafe fn compare_gjk_call(
    c_gjk: GjKFn,
    r_gjk: GjKFn,
    typ_a: c_int,
    values_a: [f32; 5],
    ax: Option<X>,
    typ_b: c_int,
    values_b: [f32; 5],
    bx: Option<X>,
    use_radius: c_int,
    output_mask: u8,
    mut c_cache: Option<&mut Cache>,
    mut r_cache: Option<&mut Cache>,
    context: &str,
) {
    let shape_a = shape_bytes(typ_a, values_a);
    let shape_b = shape_bytes(typ_b, values_b);
    let mut c_a = V {
        x: f32::from_bits(0x7fc0_1001),
        y: f32::from_bits(0xffc0_1002),
    };
    let mut c_b = V {
        x: f32::from_bits(0x7fc0_1003),
        y: f32::from_bits(0xffc0_1004),
    };
    let mut r_a = c_a;
    let mut r_b = c_b;
    let mut c_iterations = -777;
    let mut r_iterations = -777;
    let c_distance = unsafe {
        c_gjk(
            as_ptr(&shape_a),
            typ_a,
            ax.as_ref().map_or(std::ptr::null(), |value| value),
            as_ptr(&shape_b),
            typ_b,
            bx.as_ref().map_or(std::ptr::null(), |value| value),
            if output_mask & 1 != 0 {
                &mut c_a
            } else {
                std::ptr::null_mut()
            },
            if output_mask & 2 != 0 {
                &mut c_b
            } else {
                std::ptr::null_mut()
            },
            use_radius,
            if output_mask & 4 != 0 {
                &mut c_iterations
            } else {
                std::ptr::null_mut()
            },
            c_cache
                .as_deref_mut()
                .map_or(std::ptr::null_mut(), |value| value),
        )
    };
    let r_distance = unsafe {
        r_gjk(
            as_ptr(&shape_a),
            typ_a,
            ax.as_ref().map_or(std::ptr::null(), |value| value),
            as_ptr(&shape_b),
            typ_b,
            bx.as_ref().map_or(std::ptr::null(), |value| value),
            if output_mask & 1 != 0 {
                &mut r_a
            } else {
                std::ptr::null_mut()
            },
            if output_mask & 2 != 0 {
                &mut r_b
            } else {
                std::ptr::null_mut()
            },
            use_radius,
            if output_mask & 4 != 0 {
                &mut r_iterations
            } else {
                std::ptr::null_mut()
            },
            r_cache
                .as_deref_mut()
                .map_or(std::ptr::null_mut(), |value| value),
        )
    };
    eq_f32(&format!("{context} distance"), c_distance, r_distance);
    if output_mask & 1 != 0 {
        eq_bytes(&format!("{context} outA"), &c_a, &r_a);
    }
    if output_mask & 2 != 0 {
        eq_bytes(&format!("{context} outB"), &c_b, &r_b);
    }
    if output_mask & 4 != 0 {
        assert_eq!(c_iterations, r_iterations, "{context} iterations");
        assert!(
            (0..=20).contains(&c_iterations),
            "{context} iteration bound"
        );
    }
    if let (Some(c_cache), Some(r_cache)) = (c_cache, r_cache) {
        eq_bytes(&format!("{context} cache"), c_cache, r_cache);
    }
}

#[test]
fn rows_01_21_vector_proxy_and_transform_primitives() {
    unsafe {
        let libs = Libs::load();
        let (c_v, r_v) = libs.pair::<VFn>(b"c2V");
        let (c_mulvs, r_mulvs) = libs.pair::<VScalarFn>(b"c2Mulvs");
        let (c_max, r_max) = libs.pair::<VVFn>(b"c2Maxv");
        let (c_min, r_min) = libs.pair::<VVFn>(b"c2Minv");
        let (c_clamp, r_clamp) = libs.pair::<VVVFn>(b"c2Clampv");
        let (c_sub, r_sub) = libs.pair::<VVFn>(b"c2Sub");
        let (c_dot, r_dot) = libs.pair::<VVFloatFn>(b"c2Dot");
        let (c_rot_identity, r_rot_identity) = libs.pair::<RFn>(b"c2RotIdentity");
        let (c_x_identity, r_x_identity) = libs.pair::<XFn>(b"c2xIdentity");
        let (c_bb_verts, r_bb_verts) = libs.pair::<BBVertsFn>(b"c2BBVerts");
        let (c_proxy, r_proxy) = libs.pair::<MakeProxyFn>(b"c2MakeProxy");
        let (c_len, r_len) = libs.pair::<VFloatFn>(b"c2Len");
        let (c_det, r_det) = libs.pair::<VVFloatFn>(b"c2Det2");
        let (c_metric, r_metric) = libs.pair::<SimplexFloatFn>(b"c2GJKSimplexMetric");
        let (c_mulrv, r_mulrv) = libs.pair::<RVFn>(b"c2Mulrv");
        let (c_add, r_add) = libs.pair::<VVFn>(b"c2Add");
        let (c_mulxv, r_mulxv) = libs.pair::<XVFn>(b"c2Mulxv");

        let special = special_floats();
        for (i, &x) in special.iter().enumerate() {
            for (j, &y) in special.iter().enumerate() {
                let a = V { x, y };
                let b = V {
                    x: special[(i + j + 3) % special.len()],
                    y: special[(i * 3 + j + 1) % special.len()],
                };
                eq_bytes("row 1 c2V", &c_v(x, y), &r_v(x, y));
                eq_bytes("row 2 c2Mulvs", &c_mulvs(a, b.x), &r_mulvs(a, b.x));
                eq_bytes("row 3 c2Maxv", &c_max(a, b), &r_max(a, b));
                eq_bytes("row 4 c2Minv", &c_min(a, b), &r_min(a, b));
                eq_bytes(
                    "row 5 c2Clampv",
                    &c_clamp(a, V { x: -1.0, y: -2.0 }, V { x: 1.0, y: 2.0 }),
                    &r_clamp(a, V { x: -1.0, y: -2.0 }, V { x: 1.0, y: 2.0 }),
                );
                eq_bytes("row 6 c2Sub", &c_sub(a, b), &r_sub(a, b));
                eq_f32("row 7 c2Dot", c_dot(a, b), r_dot(a, b));
                eq_f32("row 14 c2Len", c_len(a), r_len(a));
                eq_f32("row 15 c2Det2", c_det(a, b), r_det(a, b));
                let rot = R { c: x, s: y };
                eq_bytes(
                    &format!(
                        "row 19 c2Mulrv i={i} j={j} rot=({:#010x},{:#010x}) b=({:#010x},{:#010x})",
                        rot.c.to_bits(),
                        rot.s.to_bits(),
                        b.x.to_bits(),
                        b.y.to_bits()
                    ),
                    &c_mulrv(rot, b),
                    &r_mulrv(rot, b),
                );
                eq_bytes("row 20 c2Add", &c_add(a, b), &r_add(a, b));
                let transform = X { p: a, r: rot };
                eq_bytes(
                    "row 21 c2Mulxv",
                    &c_mulxv(transform, b),
                    &r_mulxv(transform, b),
                );
            }
        }

        eq_bytes("row 8 c2RotIdentity", &c_rot_identity(), &r_rot_identity());
        eq_bytes("row 9 c2xIdentity", &c_x_identity(), &r_x_identity());

        let mut rng = Rng::new();
        for _ in 0..CASES {
            let a = rng.v();
            let b = rng.v();
            let scalar = rng.finite();
            eq_bytes("row 1 random c2V", &c_v(a.x, a.y), &r_v(a.x, a.y));
            eq_bytes(
                "row 2 random c2Mulvs",
                &c_mulvs(a, scalar),
                &r_mulvs(a, scalar),
            );
            eq_bytes("row 3 random c2Maxv", &c_max(a, b), &r_max(a, b));
            eq_bytes("row 4 random c2Minv", &c_min(a, b), &r_min(a, b));
            eq_bytes("row 6 random c2Sub", &c_sub(a, b), &r_sub(a, b));
            eq_f32("row 7 random c2Dot", c_dot(a, b), r_dot(a, b));

            let mut bb_c = Aabb { min: a, max: b };
            let mut bb_r = bb_c;
            let mut out_c = [V::default(); 4];
            let mut out_r = [V::default(); 4];
            c_bb_verts(out_c.as_mut_ptr(), &mut bb_c);
            r_bb_verts(out_r.as_mut_ptr(), &mut bb_r);
            eq_bytes("row 10 c2BBVerts", &out_c, &out_r);

            let circle = Circle {
                p: a,
                r: rng.moderate(),
            };
            let aabb = Aabb { min: a, max: b };
            let capsule = Capsule {
                a,
                b,
                r: rng.moderate(),
            };
            for (row, typ, shape) in [
                (11, CIRCLE, bytes(&circle)),
                (12, AABB, bytes(&aabb)),
                (13, CAPSULE, bytes(&capsule)),
            ] {
                let mut p_c = Proxy {
                    radius: f32::from_bits(0x7fc0_1111),
                    count: -7,
                    verts: [V { x: 9.0, y: -9.0 }; 8],
                };
                let mut p_r = p_c;
                c_proxy(shape.as_ptr().cast(), typ, &mut p_c);
                r_proxy(shape.as_ptr().cast(), typ, &mut p_r);
                eq_bytes(&format!("row {row} c2MakeProxy"), &p_c, &p_r);
            }

            for count in 1..=3 {
                let mut s_c = Simplex {
                    a: Sv {
                        p: a,
                        ..Sv::default()
                    },
                    b: Sv {
                        p: b,
                        ..Sv::default()
                    },
                    c: Sv {
                        p: rng.v(),
                        ..Sv::default()
                    },
                    count,
                    ..Simplex::default()
                };
                let mut s_r = s_c;
                eq_f32(
                    &format!("rows 16-18 metric count {count}"),
                    c_metric(&mut s_c),
                    r_metric(&mut s_r),
                );
            }
        }
    }
}

fn seeded_simplex(points: [V; 3]) -> Simplex {
    Simplex {
        a: Sv {
            p: points[0],
            iA: 10,
            iB: 20,
            ..Sv::default()
        },
        b: Sv {
            p: points[1],
            iA: 11,
            iB: 21,
            ..Sv::default()
        },
        c: Sv {
            p: points[2],
            iA: 12,
            iB: 22,
            ..Sv::default()
        },
        div: 7.0,
        count: 3,
        ..Simplex::default()
    }
}

#[test]
fn rows_22_31_simplex_reduction_branches() {
    unsafe {
        let libs = Libs::load();
        let (c_22, r_22) = libs.pair::<SimplexVoidFn>(b"c22");
        let (c_23, r_23) = libs.pair::<SimplexVoidFn>(b"c23");

        let edge_cases = [
            (22, [V { x: 0.0, y: 0.0 }, V { x: 2.0, y: 0.0 }], 1),
            (23, [V { x: 2.0, y: 0.0 }, V { x: 0.0, y: 0.0 }], 1),
            (24, [V { x: -2.0, y: 1.0 }, V { x: 2.0, y: 1.0 }], 2),
        ];
        for (row, points, expected_count) in edge_cases {
            for scale in 1..=CASES {
                let k = scale as f32 / 17.0;
                let mut s_c = seeded_simplex([
                    V {
                        x: points[0].x * k,
                        y: points[0].y * k,
                    },
                    V {
                        x: points[1].x * k,
                        y: points[1].y * k,
                    },
                    V::default(),
                ]);
                s_c.count = 2;
                let mut s_r = s_c;
                c_22(&mut s_c);
                r_22(&mut s_r);
                assert_eq!(s_c.count, expected_count, "row {row} fixture");
                eq_bytes(&format!("row {row} c22"), &s_c, &s_r);
            }
        }

        let triangle_cases = [
            (
                25,
                [
                    V { x: 0.0, y: 0.0 },
                    V { x: 2.0, y: 0.0 },
                    V { x: 0.0, y: 2.0 },
                ],
                1,
            ),
            (
                26,
                [
                    V { x: 2.0, y: 0.0 },
                    V { x: 0.0, y: 0.0 },
                    V { x: 0.0, y: 2.0 },
                ],
                1,
            ),
            (
                27,
                [
                    V { x: 2.0, y: 0.0 },
                    V { x: 0.0, y: 2.0 },
                    V { x: 0.0, y: 0.0 },
                ],
                1,
            ),
            (
                28,
                [
                    V { x: -2.0, y: 1.0 },
                    V { x: 2.0, y: 1.0 },
                    V { x: 0.0, y: 3.0 },
                ],
                2,
            ),
            (
                29,
                [
                    V { x: 0.0, y: 3.0 },
                    V { x: -2.0, y: 1.0 },
                    V { x: 2.0, y: 1.0 },
                ],
                2,
            ),
            (
                30,
                [
                    V { x: 2.0, y: 1.0 },
                    V { x: 0.0, y: 3.0 },
                    V { x: -2.0, y: 1.0 },
                ],
                2,
            ),
            (
                31,
                [
                    V { x: -2.0, y: -1.0 },
                    V { x: 2.0, y: -1.0 },
                    V { x: 0.0, y: 2.0 },
                ],
                3,
            ),
        ];
        for (row, points, expected_count) in triangle_cases {
            for scale in 1..=CASES {
                let k = scale as f32 / 19.0;
                let mut scaled = points;
                for point in &mut scaled {
                    point.x *= k;
                    point.y *= k;
                }
                let mut s_c = seeded_simplex(scaled);
                let mut s_r = s_c;
                c_23(&mut s_c);
                r_23(&mut s_r);
                assert_eq!(s_c.count, expected_count, "row {row} fixture");
                eq_bytes(&format!("row {row} c23"), &s_c, &s_r);
            }
        }
    }
}

#[test]
fn rows_32_46_remaining_low_level_operations() {
    unsafe {
        let libs = Libs::load();
        let (c_neg, r_neg) = libs.pair::<VUnaryFn>(b"c2Neg");
        let (c_skew, r_skew) = libs.pair::<VUnaryFn>(b"c2Skew");
        let (c_ccw, r_ccw) = libs.pair::<VUnaryFn>(b"c2CCW90");
        let (c_d, r_d) = libs.pair::<SimplexVFn>(b"c2D");
        let (c_support, r_support) = libs.pair::<SupportFn>(b"c2Support");
        let (c_witness, r_witness) = libs.pair::<WitnessFn>(b"c2Witness");
        let (c_div, r_div) = libs.pair::<VScalarFn>(b"c2Div");
        let (c_norm, r_norm) = libs.pair::<VUnaryFn>(b"c2Norm");
        let (c_l, r_l) = libs.pair::<SimplexVFn>(b"c2L");
        let (c_mulrvt, r_mulrvt) = libs.pair::<RVFn>(b"c2MulrvT");

        let special = special_floats();
        for (i, &x) in special.iter().enumerate() {
            for (j, &y) in special.iter().enumerate() {
                let v = V { x, y };
                eq_bytes("row 32 c2Neg", &c_neg(v), &r_neg(v));
                eq_bytes("row 32 c2Skew", &c_skew(v), &r_skew(v));
                eq_bytes("row 32 c2CCW90", &c_ccw(v), &r_ccw(v));
                eq_bytes("row 42 c2Div", &c_div(v, special[j]), &r_div(v, special[j]));
                eq_bytes("row 43 c2Norm", &c_norm(v), &r_norm(v));
                let rot = R {
                    c: special[(i + j + 1) % special.len()],
                    s: special[(i * 7 + j + 2) % special.len()],
                };
                eq_bytes(
                    &format!("row 46 c2MulrvT i={i} j={j}"),
                    &c_mulrvt(rot, v),
                    &r_mulrvt(rot, v),
                );
            }
        }

        let d_cases = [
            (33, 1, V { x: 2.0, y: -3.0 }, V::default()),
            (34, 2, V { x: 1.0, y: 0.0 }, V { x: 1.0, y: 2.0 }),
            (35, 2, V { x: 1.0, y: 0.0 }, V { x: 1.0, y: -2.0 }),
        ];
        for (row, count, a, b) in d_cases {
            for scale in 1..=CASES {
                let k = scale as f32 / 23.0;
                let mut s_c = seeded_simplex([
                    V {
                        x: a.x * k,
                        y: a.y * k,
                    },
                    V {
                        x: b.x * k,
                        y: b.y * k,
                    },
                    V::default(),
                ]);
                s_c.count = count;
                let mut s_r = s_c;
                eq_bytes(&format!("row {row} c2D"), &c_d(&mut s_c), &r_d(&mut s_r));
            }
        }

        let mut rng = Rng::new();
        for count in [1, 2, 4, 8] {
            for _ in 0..CASES {
                let mut verts = [V::default(); 8];
                for value in &mut verts[..count] {
                    *value = rng.v();
                }
                let direction = rng.v();
                assert_eq!(
                    c_support(verts.as_ptr(), count as c_int, direction),
                    r_support(verts.as_ptr(), count as c_int, direction),
                    "rows 36-37 c2Support count {count}"
                );

                for value in &mut verts[..count] {
                    *value = V { x: 1.0, y: 1.0 };
                }
                assert_eq!(
                    c_support(verts.as_ptr(), count as c_int, direction),
                    r_support(verts.as_ptr(), count as c_int, direction),
                    "row 38 tied c2Support count {count}"
                );
            }
        }

        for count in 1..=3 {
            for _ in 0..CASES {
                let mut s_c = Simplex {
                    a: Sv {
                        sA: rng.v(),
                        sB: rng.v(),
                        p: rng.v(),
                        u: rng.moderate(),
                        ..Sv::default()
                    },
                    b: Sv {
                        sA: rng.v(),
                        sB: rng.v(),
                        p: rng.v(),
                        u: rng.moderate(),
                        ..Sv::default()
                    },
                    c: Sv {
                        sA: rng.v(),
                        sB: rng.v(),
                        p: rng.v(),
                        u: rng.moderate(),
                        ..Sv::default()
                    },
                    div: {
                        let value = rng.moderate();
                        if value == 0.0 { 1.0 } else { value }
                    },
                    count,
                    ..Simplex::default()
                };
                let mut s_r = s_c;
                let mut ca = V::default();
                let mut cb = V::default();
                let mut ra = V::default();
                let mut rb = V::default();
                c_witness(&mut s_c, &mut ca, &mut cb);
                r_witness(&mut s_r, &mut ra, &mut rb);
                eq_bytes(&format!("rows 39-41 c2Witness A count {count}"), &ca, &ra);
                eq_bytes(&format!("rows 39-41 c2Witness B count {count}"), &cb, &rb);

                if count <= 2 {
                    eq_bytes(
                        &format!("rows 44-45 c2L count {count}"),
                        &c_l(&mut s_c),
                        &r_l(&mut s_r),
                    );
                }
            }
        }
    }
}

#[test]
fn error_rows_01_06_defined_default_and_range_behavior() {
    unsafe {
        let libs = Libs::load();
        let (c_proxy, r_proxy) = libs.pair::<MakeProxyFn>(b"c2MakeProxy");
        let (c_metric, r_metric) = libs.pair::<SimplexFloatFn>(b"c2GJKSimplexMetric");
        let (c_d, r_d) = libs.pair::<SimplexVFn>(b"c2D");
        let (c_support, r_support) = libs.pair::<SupportFn>(b"c2Support");
        let (c_witness, r_witness) = libs.pair::<WitnessFn>(b"c2Witness");
        let (c_l, r_l) = libs.pair::<SimplexVFn>(b"c2L");

        let shape = Circle {
            p: V { x: 1.0, y: 2.0 },
            r: 3.0,
        };
        for invalid in [-1, 3, INVALID, c_int::MAX] {
            let mut p_c = Proxy {
                radius: f32::from_bits(0x7fc0_4321),
                count: -123,
                verts: [V { x: -7.0, y: 9.0 }; 8],
            };
            let mut p_r = p_c;
            let before = p_c;
            c_proxy((&shape as *const Circle).cast(), invalid, &mut p_c);
            r_proxy((&shape as *const Circle).cast(), invalid, &mut p_r);
            eq_bytes("error row 1 C proxy unchanged", &before, &p_c);
            eq_bytes("error row 1 Rust proxy", &p_c, &p_r);
        }

        for invalid_count in [-100, -1, 0, 4, c_int::MAX] {
            let mut s_c = seeded_simplex([
                V { x: 1.0, y: 2.0 },
                V { x: 3.0, y: 4.0 },
                V { x: 5.0, y: 6.0 },
            ]);
            s_c.count = invalid_count;
            let mut s_r = s_c;
            eq_f32(
                "error row 2 metric default",
                c_metric(&mut s_c),
                r_metric(&mut s_r),
            );
            eq_bytes("error row 3 c2D default", &c_d(&mut s_c), &r_d(&mut s_r));

            let mut ca = V { x: 9.0, y: 8.0 };
            let mut cb = V { x: 7.0, y: 6.0 };
            let mut ra = ca;
            let mut rb = cb;
            c_witness(&mut s_c, &mut ca, &mut cb);
            r_witness(&mut s_r, &mut ra, &mut rb);
            eq_bytes("error row 5 witness A", &ca, &ra);
            eq_bytes("error row 5 witness B", &cb, &rb);
            eq_bytes("error row 6 c2L default", &c_l(&mut s_c), &r_l(&mut s_r));
        }

        let vertex = V { x: 2.0, y: 3.0 };
        for count in [c_int::MIN, -1, 0] {
            assert_eq!(
                c_support(&vertex, count, V { x: 1.0, y: -1.0 }),
                r_support(&vertex, count, V { x: 1.0, y: -1.0 }),
                "error row 4 c2Support count {count}"
            );
        }
    }
}

#[test]
fn rows_47_52_gjk_option_shape_and_cache_cross_product() {
    unsafe {
        let libs = Libs::load();
        let (c_gjk, r_gjk) = libs.pair::<GjKFn>(b"c2GJK");
        let types = [CAPSULE, CIRCLE, AABB];
        let identity = X {
            p: V { x: 0.0, y: 0.0 },
            r: R { c: 1.0, s: 0.0 },
        };
        let shifted = X {
            p: V { x: 3.25, y: -1.5 },
            r: R { c: 0.8, s: 0.6 },
        };
        let mut rng = Rng::new();

        for &typ_a in &types {
            for &typ_b in &types {
                for case in 0..CASES {
                    let values_a = random_shape_values(&mut rng, typ_a);
                    let values_b = random_shape_values(&mut rng, typ_b);
                    for use_radius in [0, 1] {
                        compare_gjk_call(
                            c_gjk,
                            r_gjk,
                            typ_a,
                            values_a,
                            None,
                            typ_b,
                            values_b,
                            None,
                            use_radius,
                            7,
                            None,
                            None,
                            &format!(
                                "rows 47/49 pair {typ_a},{typ_b} case {case} radius {use_radius}"
                            ),
                        );
                    }

                    let (ax, bx) = match case % 4 {
                        0 => (None, None),
                        1 => (Some(identity), None),
                        2 => (None, Some(shifted)),
                        _ => (Some(shifted), Some(identity)),
                    };
                    compare_gjk_call(
                        c_gjk,
                        r_gjk,
                        typ_a,
                        values_a,
                        ax,
                        typ_b,
                        values_b,
                        bx,
                        1,
                        7,
                        None,
                        None,
                        &format!("row 48 transform pair {typ_a},{typ_b} case {case}"),
                    );
                }
            }
        }

        let circle_a = [0.0, 0.0, 2.0, 0.0, 0.0];
        for (relation, circle_b) in [
            ("separated", [5.0, 0.0, 2.0, 0.0, 0.0]),
            ("touching", [4.0, 0.0, 2.0, 0.0, 0.0]),
            ("overlapping", [3.0, 0.0, 2.0, 0.0, 0.0]),
        ] {
            for _ in 0..CASES {
                compare_gjk_call(
                    c_gjk,
                    r_gjk,
                    CIRCLE,
                    circle_a,
                    None,
                    CIRCLE,
                    circle_b,
                    None,
                    1,
                    7,
                    None,
                    None,
                    &format!("row 50 {relation}"),
                );
            }
        }

        let values_a = [0.0, 0.0, 2.0, 0.0, 0.5];
        let values_b = [1.0, 1.0, 3.0, 1.0, 0.75];
        for output_mask in 0..8 {
            for _ in 0..CASES {
                compare_gjk_call(
                    c_gjk,
                    r_gjk,
                    CAPSULE,
                    values_a,
                    None,
                    CAPSULE,
                    values_b,
                    None,
                    1,
                    output_mask,
                    None,
                    None,
                    &format!("row 51 output mask {output_mask}"),
                );
            }
        }

        for &typ_a in &types {
            for &typ_b in &types {
                for case in 0..32 {
                    let values_a = random_shape_values(&mut rng, typ_a);
                    let values_b = random_shape_values(&mut rng, typ_b);
                    let mut c_cache = Cache::default();
                    let mut r_cache = Cache::default();
                    compare_gjk_call(
                        c_gjk,
                        r_gjk,
                        typ_a,
                        values_a,
                        None,
                        typ_b,
                        values_b,
                        None,
                        1,
                        7,
                        Some(&mut c_cache),
                        Some(&mut r_cache),
                        &format!("row 52 cold cache pair {typ_a},{typ_b} case {case}"),
                    );
                    compare_gjk_call(
                        c_gjk,
                        r_gjk,
                        typ_a,
                        values_a,
                        None,
                        typ_b,
                        values_b,
                        None,
                        1,
                        7,
                        Some(&mut c_cache),
                        Some(&mut r_cache),
                        &format!("row 52 warm cache pair {typ_a},{typ_b} case {case}"),
                    );
                }
            }
        }

        let mut rejected_c = Cache {
            metric: 0.0,
            count: 3,
            iA: [0, 0, 0],
            iB: [0, 2, 1],
            div: 1.0,
        };
        let mut rejected_r = rejected_c;
        compare_gjk_call(
            c_gjk,
            r_gjk,
            AABB,
            [0.0, 0.0, 0.0, 0.0, 0.0],
            None,
            AABB,
            [0.0, 0.0, 100_000.0, 100_000.0, 0.0],
            None,
            0,
            7,
            Some(&mut rejected_c),
            Some(&mut rejected_r),
            "row 52 rejected cache",
        );
    }
}

#[test]
fn rows_53_63_collision_dispatch_allocation_and_one_shot() {
    unsafe {
        let libs = Libs::load();
        let (c_aa, r_aa) = libs.pair::<AabbAabbFn>(b"c2AABBtoAABB");
        let (c_ac, r_ac) = libs.pair::<AabbCapsuleFn>(b"c2AABBtoCapsule");
        let (c_ccap, r_ccap) = libs.pair::<CapsuleCapsuleFn>(b"c2CapsuletoCapsule");
        let (c_cc, r_cc) = libs.pair::<CircleCircleFn>(b"c2CircletoCircle");
        let (c_ca, r_ca) = libs.pair::<CircleAabbFn>(b"c2CircletoAABB");
        let (c_ccirclecap, r_ccirclecap) = libs.pair::<CircleCapsuleFn>(b"c2CircletoCapsule");
        let (c_collided, r_collided) = libs.pair::<CollidedFn>(b"c2Collided");
        let (c_parts, r_parts) = libs.pair::<PartsFn>(b"ptr_from_parts");
        let (c_omni, r_omni) = libs.pair::<OmniFn>(b"omni_collide");

        let aabb = Aabb {
            min: V { x: 0.0, y: 0.0 },
            max: V { x: 2.0, y: 2.0 },
        };
        for (relation, other) in [
            (
                "separated",
                Aabb {
                    min: V { x: 3.0, y: 0.0 },
                    max: V { x: 5.0, y: 2.0 },
                },
            ),
            (
                "touching",
                Aabb {
                    min: V { x: 2.0, y: 0.0 },
                    max: V { x: 4.0, y: 2.0 },
                },
            ),
            (
                "overlapping",
                Aabb {
                    min: V { x: 1.0, y: 1.0 },
                    max: V { x: 3.0, y: 3.0 },
                },
            ),
        ] {
            for _ in 0..CASES {
                assert_eq!(c_aa(aabb, other), r_aa(aabb, other), "row 53 {relation}");
            }
        }

        let base_capsule = Capsule {
            a: V { x: 0.0, y: 0.0 },
            b: V { x: 2.0, y: 0.0 },
            r: 1.0,
        };
        let circle = Circle {
            p: V { x: 1.0, y: 1.0 },
            r: 1.0,
        };
        for (relation, offset) in [("separated", 5.0), ("touching", 4.0), ("overlapping", 3.0)] {
            let cap = Capsule {
                a: V { x: 0.0, y: offset },
                b: V { x: 2.0, y: offset },
                r: 1.0,
            };
            let other_circle = Circle {
                p: V { x: offset, y: 1.0 },
                r: 1.0,
            };
            for _ in 0..CASES {
                assert_eq!(c_ac(aabb, cap), r_ac(aabb, cap), "row 54 {relation}");
                assert_eq!(
                    c_ccap(base_capsule, cap),
                    r_ccap(base_capsule, cap),
                    "row 55 {relation}"
                );
                assert_eq!(
                    c_cc(circle, other_circle),
                    r_cc(circle, other_circle),
                    "row 56 {relation}"
                );
                assert_eq!(
                    c_ca(other_circle, aabb),
                    r_ca(other_circle, aabb),
                    "row 57 {relation}"
                );
                assert_eq!(
                    c_ccirclecap(other_circle, base_capsule),
                    r_ccirclecap(other_circle, base_capsule),
                    "row 58 {relation}"
                );
            }
        }

        let mut rng = Rng::new();
        for case in 0..CASES {
            let av = random_shape_values(&mut rng, AABB);
            let bv = random_shape_values(&mut rng, AABB);
            let aa = Aabb {
                min: V { x: av[0], y: av[1] },
                max: V { x: av[2], y: av[3] },
            };
            let ab = Aabb {
                min: V { x: bv[0], y: bv[1] },
                max: V { x: bv[2], y: bv[3] },
            };
            assert_eq!(c_aa(aa, ab), r_aa(aa, ab), "row 53 random {case}");

            let cv = random_shape_values(&mut rng, CIRCLE);
            let circle = Circle {
                p: V { x: cv[0], y: cv[1] },
                r: cv[2],
            };
            let cv2 = random_shape_values(&mut rng, CIRCLE);
            let circle2 = Circle {
                p: V {
                    x: cv2[0],
                    y: cv2[1],
                },
                r: cv2[2],
            };
            assert_eq!(
                c_cc(circle, circle2),
                r_cc(circle, circle2),
                "row 56 random"
            );
            assert_eq!(c_ca(circle, aa), r_ca(circle, aa), "row 57 random");

            let capv = random_shape_values(&mut rng, CAPSULE);
            let capsule = Capsule {
                a: V {
                    x: capv[0],
                    y: capv[1],
                },
                b: V {
                    x: capv[2],
                    y: capv[3],
                },
                r: capv[4],
            };
            let capv2 = random_shape_values(&mut rng, CAPSULE);
            let capsule2 = Capsule {
                a: V {
                    x: capv2[0],
                    y: capv2[1],
                },
                b: V {
                    x: capv2[2],
                    y: capv2[3],
                },
                r: capv2[4],
            };
            assert_eq!(c_ac(aa, capsule), r_ac(aa, capsule), "row 54 random");
            assert_eq!(
                c_ccap(capsule, capsule2),
                r_ccap(capsule, capsule2),
                "row 55 random"
            );
            assert_eq!(
                c_ccirclecap(circle, capsule),
                r_ccirclecap(circle, capsule),
                "row 58 random"
            );
        }

        for &typ_a in &[CAPSULE, CIRCLE, AABB] {
            for &typ_b in &[CAPSULE, CIRCLE, AABB] {
                for case in 0..CASES {
                    let values_a = random_shape_values(&mut rng, typ_a);
                    let values_b = random_shape_values(&mut rng, typ_b);
                    let shape_a = shape_bytes(typ_a, values_a);
                    let shape_b = shape_bytes(typ_b, values_b);
                    assert_eq!(
                        c_collided(as_ptr(&shape_a), typ_a, as_ptr(&shape_b), typ_b),
                        r_collided(as_ptr(&shape_a), typ_a, as_ptr(&shape_b), typ_b),
                        "row 59 pair {typ_a},{typ_b} case {case}"
                    );
                    assert_eq!(
                        c_omni(
                            typ_a,
                            values_a[0],
                            values_a[1],
                            values_a[2],
                            values_a[3],
                            values_a[4],
                            typ_b,
                            values_b[0],
                            values_b[1],
                            values_b[2],
                            values_b[3],
                            values_b[4],
                        ),
                        r_omni(
                            typ_a,
                            values_a[0],
                            values_a[1],
                            values_a[2],
                            values_a[3],
                            values_a[4],
                            typ_b,
                            values_b[0],
                            values_b[1],
                            values_b[2],
                            values_b[3],
                            values_b[4],
                        ),
                        "row 63 pair {typ_a},{typ_b} case {case}"
                    );
                }
            }
        }

        for (row, typ, size) in [
            (60, CIRCLE, size_of::<Circle>()),
            (61, AABB, size_of::<Aabb>()),
            (62, CAPSULE, size_of::<Capsule>()),
        ] {
            for case in 0..CASES {
                let values = random_shape_values(&mut rng, typ);
                let c_pointer = c_parts(typ, values[0], values[1], values[2], values[3], values[4]);
                let r_pointer = r_parts(typ, values[0], values[1], values[2], values[3], values[4]);
                assert!(!c_pointer.is_null() && !r_pointer.is_null());
                let c_value = std::slice::from_raw_parts(c_pointer.cast::<u8>(), size);
                let r_value = std::slice::from_raw_parts(r_pointer.cast::<u8>(), size);
                assert_eq!(c_value, r_value, "row {row} case {case}");
                free(c_pointer);
                free(r_pointer);
            }
        }
    }
}

#[test]
fn error_rows_07_10_invalid_collision_enums() {
    unsafe {
        let libs = Libs::load();
        let (c_collided, r_collided) = libs.pair::<CollidedFn>(b"c2Collided");
        let circle = Circle {
            p: V { x: 1.0, y: 2.0 },
            r: 3.0,
        };
        for invalid in [-1, 3, INVALID, c_int::MAX] {
            assert_eq!(
                c_collided(std::ptr::null(), invalid, std::ptr::null(), invalid),
                r_collided(std::ptr::null(), invalid, std::ptr::null(), invalid),
                "error row 7 invalid typeA {invalid}"
            );
            for (row, type_a) in [(8, CIRCLE), (9, AABB), (10, CAPSULE)] {
                assert_eq!(
                    c_collided(
                        (&circle as *const Circle).cast(),
                        type_a,
                        std::ptr::null(),
                        invalid
                    ),
                    r_collided(
                        (&circle as *const Circle).cast(),
                        type_a,
                        std::ptr::null(),
                        invalid
                    ),
                    "error row {row} invalid typeB {invalid}"
                );
            }
        }
    }
}

#[test]
fn error_rows_11_19_gjk_boundaries() {
    unsafe {
        let libs = Libs::load();
        let (c_gjk, r_gjk) = libs.pair::<GjKFn>(b"c2GJK");
        let circle_a = shape_bytes(CIRCLE, [0.0, 0.0, 1.0, 0.0, 0.0]);
        let circle_b = shape_bytes(CIRCLE, [4.0, 0.0, 1.0, 0.0, 0.0]);
        let identity = X {
            p: V { x: 0.0, y: 0.0 },
            r: R { c: 1.0, s: 0.0 },
        };

        for (ax, bx, row) in [
            (None, Some(identity), 11),
            (Some(identity), None, 12),
            (None, None, 13),
        ] {
            for _ in 0..CASES {
                compare_gjk_call(
                    c_gjk,
                    r_gjk,
                    CIRCLE,
                    [0.0, 0.0, 1.0, 0.0, 0.0],
                    ax,
                    CIRCLE,
                    [4.0, 0.0, 1.0, 0.0, 0.0],
                    bx,
                    1,
                    7,
                    None,
                    None,
                    &format!("error row {row}"),
                );
            }
        }

        let mut zero_c = Cache::default();
        let mut zero_r = Cache::default();
        compare_gjk_call(
            c_gjk,
            r_gjk,
            CIRCLE,
            [0.0, 0.0, 1.0, 0.0, 0.0],
            None,
            CIRCLE,
            [4.0, 0.0, 1.0, 0.0, 0.0],
            None,
            1,
            7,
            Some(&mut zero_c),
            Some(&mut zero_r),
            "error row 14 zero cache",
        );
        assert!((1..=3).contains(&zero_c.count));

        let mut rejected_c = Cache {
            metric: 0.0,
            count: 3,
            iA: [0, 0, 0],
            iB: [0, 2, 1],
            div: 1.0,
        };
        let mut rejected_r = rejected_c;
        compare_gjk_call(
            c_gjk,
            r_gjk,
            AABB,
            [0.0, 0.0, 0.0, 0.0, 0.0],
            None,
            AABB,
            [0.0, 0.0, 100_000.0, 100_000.0, 0.0],
            None,
            0,
            7,
            Some(&mut rejected_c),
            Some(&mut rejected_r),
            "error row 15 rejected cache",
        );
        assert_eq!(
            rejected_c.count, 1,
            "fixture must reject the cached triangle"
        );

        let mut c_out_a = V::default();
        let mut c_out_b = V::default();
        let mut r_out_a = V::default();
        let mut r_out_b = V::default();
        let mut c_iterations = -1;
        let mut r_iterations = -1;
        let c_distance = c_gjk(
            as_ptr(&circle_a),
            CIRCLE,
            std::ptr::null(),
            as_ptr(&circle_a),
            CIRCLE,
            std::ptr::null(),
            &mut c_out_a,
            &mut c_out_b,
            1,
            &mut c_iterations,
            std::ptr::null_mut(),
        );
        let r_distance = r_gjk(
            as_ptr(&circle_a),
            CIRCLE,
            std::ptr::null(),
            as_ptr(&circle_a),
            CIRCLE,
            std::ptr::null(),
            &mut r_out_a,
            &mut r_out_b,
            1,
            &mut r_iterations,
            std::ptr::null_mut(),
        );
        eq_f32("error rows 17-18 epsilon/collapse", c_distance, r_distance);
        assert_eq!(c_distance.to_bits(), 0.0f32.to_bits());
        assert_eq!(c_iterations, r_iterations);
        assert_eq!(c_iterations, 0, "fixture must terminate at epsilon check");
        eq_bytes("error row 18 midpoint A", &c_out_a, &r_out_a);
        eq_bytes("error row 18 midpoint B", &c_out_b, &r_out_b);

        let mut rng = Rng::new();
        let types = [CAPSULE, CIRCLE, AABB];
        for case in 0..10_000 {
            let typ_a = types[(rng.u32() % 3) as usize];
            let typ_b = types[(rng.u32() % 3) as usize];
            let values_a = random_shape_values(&mut rng, typ_a);
            let values_b = random_shape_values(&mut rng, typ_b);
            let shape_a = shape_bytes(typ_a, values_a);
            let shape_b = shape_bytes(typ_b, values_b);
            let mut c_iter = -1;
            let mut r_iter = -1;
            c_gjk(
                as_ptr(&shape_a),
                typ_a,
                std::ptr::null(),
                as_ptr(&shape_b),
                typ_b,
                std::ptr::null(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                1,
                &mut c_iter,
                std::ptr::null_mut(),
            );
            r_gjk(
                as_ptr(&shape_a),
                typ_a,
                std::ptr::null(),
                as_ptr(&shape_b),
                typ_b,
                std::ptr::null(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                1,
                &mut r_iter,
                std::ptr::null_mut(),
            );
            assert_eq!(c_iter, r_iter, "error row 16 case {case}");
            assert!((0..=20).contains(&c_iter), "error row 16 case {case}");
        }

        for output_mask in 0..8 {
            compare_gjk_call(
                c_gjk,
                r_gjk,
                CIRCLE,
                [0.0, 0.0, 1.0, 0.0, 0.0],
                None,
                CIRCLE,
                [4.0, 0.0, 1.0, 0.0, 0.0],
                None,
                1,
                output_mask,
                None,
                None,
                &format!("error row 19 mask {output_mask}"),
            );
        }

        let _ = circle_b;
    }
}

#[test]
#[ignore]
fn ffi_undefined_null_probe_child() {
    let Ok(scenario) = std::env::var("FFI_NULL_SCENARIO") else {
        return;
    };
    let library_kind = std::env::var("FFI_LIBRARY_KIND").unwrap();
    unsafe {
        let libs = Libs::load();
        let library = if library_kind == "c" {
            &libs.c
        } else {
            &libs.rust
        };
        let mut vertex = V { x: 1.0, y: 2.0 };
        let mut vertices = [V::default(); 4];
        let mut aabb = Aabb {
            min: V { x: 0.0, y: 0.0 },
            max: V { x: 1.0, y: 1.0 },
        };
        let circle = Circle {
            p: V { x: 0.0, y: 0.0 },
            r: 1.0,
        };
        let mut proxy = Proxy::default();
        let mut simplex = Simplex {
            count: 1,
            div: 1.0,
            ..Simplex::default()
        };

        match scenario.as_str() {
            "bb_out" => {
                let function = *library.get::<BBVertsFn>(b"c2BBVerts").unwrap();
                function(std::ptr::null_mut(), &mut aabb);
            }
            "bb_shape" => {
                let function = *library.get::<BBVertsFn>(b"c2BBVerts").unwrap();
                function(vertices.as_mut_ptr(), std::ptr::null_mut());
            }
            "proxy_shape" => {
                let function = *library.get::<MakeProxyFn>(b"c2MakeProxy").unwrap();
                function(std::ptr::null(), CIRCLE, &mut proxy);
            }
            "proxy_out" => {
                let function = *library.get::<MakeProxyFn>(b"c2MakeProxy").unwrap();
                function(
                    (&circle as *const Circle).cast(),
                    CIRCLE,
                    std::ptr::null_mut(),
                );
            }
            "metric" => {
                let function = *library
                    .get::<SimplexFloatFn>(b"c2GJKSimplexMetric")
                    .unwrap();
                function(std::ptr::null_mut());
            }
            "c22" => {
                let function = *library.get::<SimplexVoidFn>(b"c22").unwrap();
                function(std::ptr::null_mut());
            }
            "c23" => {
                let function = *library.get::<SimplexVoidFn>(b"c23").unwrap();
                function(std::ptr::null_mut());
            }
            "direction" => {
                let function = *library.get::<SimplexVFn>(b"c2D").unwrap();
                function(std::ptr::null_mut());
            }
            "support" => {
                let function = *library.get::<SupportFn>(b"c2Support").unwrap();
                function(std::ptr::null(), 0, V::default());
            }
            "witness_simplex" => {
                let function = *library.get::<WitnessFn>(b"c2Witness").unwrap();
                function(std::ptr::null_mut(), &mut vertex, &mut vertices[0]);
            }
            "witness_a" => {
                let function = *library.get::<WitnessFn>(b"c2Witness").unwrap();
                function(&mut simplex, std::ptr::null_mut(), &mut vertex);
            }
            "witness_b" => {
                let function = *library.get::<WitnessFn>(b"c2Witness").unwrap();
                function(&mut simplex, &mut vertex, std::ptr::null_mut());
            }
            "location" => {
                let function = *library.get::<SimplexVFn>(b"c2L").unwrap();
                function(std::ptr::null_mut());
            }
            "gjk_shape_a" => {
                let function = *library.get::<GjKFn>(b"c2GJK").unwrap();
                function(
                    std::ptr::null(),
                    CIRCLE,
                    std::ptr::null(),
                    (&circle as *const Circle).cast(),
                    CIRCLE,
                    std::ptr::null(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    1,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                );
            }
            "gjk_shape_b" => {
                let function = *library.get::<GjKFn>(b"c2GJK").unwrap();
                function(
                    (&circle as *const Circle).cast(),
                    CIRCLE,
                    std::ptr::null(),
                    std::ptr::null(),
                    CIRCLE,
                    std::ptr::null(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    1,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                );
            }
            "collided_a" => {
                let function = *library.get::<CollidedFn>(b"c2Collided").unwrap();
                function(
                    std::ptr::null(),
                    CIRCLE,
                    (&circle as *const Circle).cast(),
                    CIRCLE,
                );
            }
            "collided_b" => {
                let function = *library.get::<CollidedFn>(b"c2Collided").unwrap();
                function(
                    (&circle as *const Circle).cast(),
                    CIRCLE,
                    std::ptr::null(),
                    CIRCLE,
                );
            }
            _ => panic!("unknown null scenario"),
        }
    }
}

#[test]
fn generic_null_zero_and_oversized_ffi_boundaries() {
    let executable = std::env::current_exe().unwrap();
    for scenario in [
        "bb_out",
        "bb_shape",
        "proxy_shape",
        "proxy_out",
        "metric",
        "c22",
        "c23",
        "direction",
        "support",
        "witness_simplex",
        "witness_a",
        "witness_b",
        "location",
        "gjk_shape_a",
        "gjk_shape_b",
        "collided_a",
        "collided_b",
    ] {
        let run = |kind: &str| {
            Command::new(&executable)
                .arg("ffi_undefined_null_probe_child")
                .arg("--exact")
                .arg("--ignored")
                .env("FFI_NULL_SCENARIO", scenario)
                .env("FFI_LIBRARY_KIND", kind)
                .status()
                .unwrap()
        };
        let c_status = run("c");
        let rust_status = run("rust");
        assert!(!c_status.success(), "{scenario}: C unexpectedly returned");
        assert!(
            !rust_status.success(),
            "{scenario}: Rust unexpectedly returned"
        );
        assert_eq!(
            (c_status.code(), c_status.signal()),
            (rust_status.code(), rust_status.signal()),
            "{scenario}: process termination differs"
        );
    }

    unsafe {
        let libs = Libs::load();
        let (c_support, r_support) = libs.pair::<SupportFn>(b"c2Support");
        let mut rng = Rng::new();
        let mut vertices = vec![V::default(); 257];
        for vertex in &mut vertices {
            *vertex = rng.v();
        }
        for count in [0, 1, 8, 9, 256, 257] {
            assert_eq!(
                c_support(vertices.as_ptr(), count, V { x: 3.0, y: -2.0 }),
                r_support(vertices.as_ptr(), count, V { x: 3.0, y: -2.0 }),
                "zero/oversized count {count}"
            );
        }
    }
}
