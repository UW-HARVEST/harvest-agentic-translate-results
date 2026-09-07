use libloading::Library;
use std::ffi::{c_char, c_int, c_void};
use std::path::{Path, PathBuf};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct V {
    x: f32,
    y: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct R {
    c: f32,
    s: f32,
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
    r: f32,
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
    r: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct Cache {
    metric: f32,
    count: c_int,
    i_a: [c_int; 3],
    i_b: [c_int; 3],
    div: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Proxy {
    radius: f32,
    count: c_int,
    verts: [V; 8],
}

impl Default for Proxy {
    fn default() -> Self {
        Self {
            radius: 0.0,
            count: 0,
            verts: [V::default(); 8],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct Sv {
    s_a: V,
    s_b: V,
    p: V,
    u: f32,
    i_a: c_int,
    i_b: c_int,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct Simplex {
    a: Sv,
    b: Sv,
    c: Sv,
    d: Sv,
    div: f32,
    count: c_int,
}

const SYMBOLS: &[&[u8]] = &[
    b"c22\0",
    b"c23\0",
    b"c2Add\0",
    b"c2BBVerts\0",
    b"c2CCW90\0",
    b"c2Clampv\0",
    b"c2D\0",
    b"c2Det2\0",
    b"c2Div\0",
    b"c2Dot\0",
    b"c2GJK\0",
    b"c2GJKSimplexMetric\0",
    b"c2L\0",
    b"c2Len\0",
    b"c2MakeProxy\0",
    b"c2Maxv\0",
    b"c2Minv\0",
    b"c2Mulrv\0",
    b"c2MulrvT\0",
    b"c2Mulvs\0",
    b"c2Mulxv\0",
    b"c2Neg\0",
    b"c2Norm\0",
    b"c2RotIdentity\0",
    b"c2Skew\0",
    b"c2Sub\0",
    b"c2Support\0",
    b"c2V\0",
    b"c2Witness\0",
    b"c2xIdentity\0",
    b"gjk\0",
];

struct Pair {
    c: Library,
    rust: Library,
}

impl Pair {
    fn load() -> Self {
        unsafe {
            Self {
                c: Library::new(c_library()).expect("load C shared library"),
                rust: Library::new(rust_library()).expect("load Rust shared library"),
            }
        }
    }

    unsafe fn funcs<T: Copy>(&self, name: &[u8]) -> (T, T) {
        unsafe {
            (
                *self.c.get::<T>(name).expect("C symbol"),
                *self.rust.get::<T>(name).expect("Rust symbol"),
            )
        }
    }
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn c_library() -> PathBuf {
    root()
        .join("c_src")
        .join("build")
        .join("libharvest-work-pZoI7S.so")
}

fn rust_library() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("release")
        .join("libgjk_lib.so")
}

fn bytes<T>(value: &T) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts((value as *const T).cast::<u8>(), std::mem::size_of::<T>())
    }
}

fn same<T>(label: &str, c: &T, rust: &T) {
    assert_eq!(bytes(c), bytes(rust), "{label}");
}

fn same_f32(label: &str, c: f32, rust: f32) {
    assert_eq!(
        c.to_bits(),
        rust.to_bits(),
        "{label}: C={c:?}, Rust={rust:?}"
    );
}

#[derive(Clone)]
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x as u32
    }

    fn f(&mut self) -> f32 {
        let n = (self.next() % 20_001) as i32 - 10_000;
        n as f32 / 64.0
    }

    fn positive(&mut self) -> f32 {
        (self.next() % 1024 + 1) as f32 / 64.0
    }

    fn v(&mut self) -> V {
        V {
            x: self.f(),
            y: self.f(),
        }
    }
}

fn transformed(base: V, scale: f32, quarter_turn: bool) -> V {
    if quarter_turn {
        V {
            x: -base.y * scale,
            y: base.x * scale,
        }
    } else {
        V {
            x: base.x * scale,
            y: base.y * scale,
        }
    }
}

#[test]
fn all_dynamic_symbols_load_from_both_libraries() {
    let pair = Pair::load();
    for name in SYMBOLS {
        unsafe {
            pair.c.get::<*const c_void>(name).unwrap();
            pair.rust.get::<*const c_void>(name).unwrap();
        }
    }
}

#[test]
fn vector_scalar_and_transform_surface() {
    let pair = Pair::load();
    unsafe {
        let (c_v, r_v) = pair.funcs::<unsafe extern "C" fn(f32, f32) -> V>(b"c2V\0");
        let (c_mulvs, r_mulvs) = pair.funcs::<unsafe extern "C" fn(V, f32) -> V>(b"c2Mulvs\0");
        let (c_max, r_max) = pair.funcs::<unsafe extern "C" fn(V, V) -> V>(b"c2Maxv\0");
        let (c_min, r_min) = pair.funcs::<unsafe extern "C" fn(V, V) -> V>(b"c2Minv\0");
        let (c_clamp, r_clamp) = pair.funcs::<unsafe extern "C" fn(V, V, V) -> V>(b"c2Clampv\0");
        let (c_sub, r_sub) = pair.funcs::<unsafe extern "C" fn(V, V) -> V>(b"c2Sub\0");
        let (c_add, r_add) = pair.funcs::<unsafe extern "C" fn(V, V) -> V>(b"c2Add\0");
        let (c_dot, r_dot) = pair.funcs::<unsafe extern "C" fn(V, V) -> f32>(b"c2Dot\0");
        let (c_det, r_det) = pair.funcs::<unsafe extern "C" fn(V, V) -> f32>(b"c2Det2\0");
        let (c_len, r_len) = pair.funcs::<unsafe extern "C" fn(V) -> f32>(b"c2Len\0");
        let (c_neg, r_neg) = pair.funcs::<unsafe extern "C" fn(V) -> V>(b"c2Neg\0");
        let (c_skew, r_skew) = pair.funcs::<unsafe extern "C" fn(V) -> V>(b"c2Skew\0");
        let (c_ccw, r_ccw) = pair.funcs::<unsafe extern "C" fn(V) -> V>(b"c2CCW90\0");
        let (c_div, r_div) = pair.funcs::<unsafe extern "C" fn(V, f32) -> V>(b"c2Div\0");
        let (c_norm, r_norm) = pair.funcs::<unsafe extern "C" fn(V) -> V>(b"c2Norm\0");
        let (c_ri, r_ri) = pair.funcs::<unsafe extern "C" fn() -> R>(b"c2RotIdentity\0");
        let (c_xi, r_xi) = pair.funcs::<unsafe extern "C" fn() -> X>(b"c2xIdentity\0");
        let (c_mulrv, r_mulrv) = pair.funcs::<unsafe extern "C" fn(R, V) -> V>(b"c2Mulrv\0");
        let (c_mulrvt, r_mulrvt) = pair.funcs::<unsafe extern "C" fn(R, V) -> V>(b"c2MulrvT\0");
        let (c_mulxv, r_mulxv) = pair.funcs::<unsafe extern "C" fn(X, V) -> V>(b"c2Mulxv\0");

        same("rotation identity", &c_ri(), &r_ri());
        same("transform identity", &c_xi(), &r_xi());

        let mut rng = Rng::new(0x91e1_0da5_c79e_7b1d);
        for i in 0..1024 {
            let a = rng.v();
            let b = rng.v();
            let scalar = if i % 11 == 0 {
                0.0
            } else if i % 2 == 0 {
                rng.positive()
            } else {
                -rng.positive()
            };
            let made_c = c_v(a.x, a.y);
            let made_r = r_v(a.x, a.y);
            same("c2V", &made_c, &made_r);
            same("c2Mulvs", &c_mulvs(a, scalar), &r_mulvs(a, scalar));
            same("c2Maxv", &c_max(a, b), &r_max(a, b));
            same("c2Minv", &c_min(a, b), &r_min(a, b));
            same("c2Sub", &c_sub(a, b), &r_sub(a, b));
            same("c2Add", &c_add(a, b), &r_add(a, b));
            same_f32("c2Dot", c_dot(a, b), r_dot(a, b));
            same_f32("c2Det2", c_det(a, b), r_det(a, b));
            same_f32("c2Len", c_len(a), r_len(a));
            same("c2Neg", &c_neg(a), &r_neg(a));
            same("c2Skew", &c_skew(a), &r_skew(a));
            same("c2CCW90", &c_ccw(a), &r_ccw(a));

            let divisor = if i % 2 == 0 {
                rng.positive()
            } else {
                -rng.positive()
            };
            same("c2Div", &c_div(a, divisor), &r_div(a, divisor));
            let nonzero = if a.x == 0.0 && a.y == 0.0 {
                V { x: 1.0, y: 0.0 }
            } else {
                a
            };
            same("c2Norm", &c_norm(nonzero), &r_norm(nonzero));

            let lo = V {
                x: a.x.min(b.x),
                y: a.y.min(b.y),
            };
            let hi = V {
                x: a.x.max(b.x),
                y: a.y.max(b.y),
            };
            let value = match i % 3 {
                0 => V {
                    x: lo.x - rng.positive(),
                    y: hi.y + rng.positive(),
                },
                1 => lo,
                _ => V {
                    x: (lo.x + hi.x) * 0.5,
                    y: (lo.y + hi.y) * 0.5,
                },
            };
            same("c2Clampv", &c_clamp(value, lo, hi), &r_clamp(value, lo, hi));

            let rot = if i % 4 == 0 {
                R { c: 1.0, s: 0.0 }
            } else {
                R {
                    c: rng.f(),
                    s: rng.f(),
                }
            };
            let transform = X { p: b, r: rot };
            same("c2Mulrv", &c_mulrv(rot, a), &r_mulrv(rot, a));
            same("c2MulrvT", &c_mulrvt(rot, a), &r_mulrvt(rot, a));
            same("c2Mulxv", &c_mulxv(transform, a), &r_mulxv(transform, a));
        }
    }
}

#[test]
fn proxy_and_aabb_surface() {
    let pair = Pair::load();
    unsafe {
        let (c_bb, r_bb) = pair.funcs::<unsafe extern "C" fn(*mut V, *mut Aabb)>(b"c2BBVerts\0");
        let (c_proxy, r_proxy) =
            pair.funcs::<unsafe extern "C" fn(*const c_void, c_int, *mut Proxy)>(b"c2MakeProxy\0");
        let mut rng = Rng::new(0xda94_2042_e4dd_58b5);
        for i in 0..512 {
            let p = rng.v();
            let q = if i % 7 == 0 { p } else { rng.v() };
            let mut bb_c = Aabb { min: p, max: q };
            let mut bb_r = bb_c;
            let sentinel = V {
                x: -777.25,
                y: 888.5,
            };
            let mut out_c = [sentinel; 4];
            let mut out_r = out_c;
            c_bb(out_c.as_mut_ptr(), &mut bb_c);
            r_bb(out_r.as_mut_ptr(), &mut bb_r);
            same("c2BBVerts output", &out_c, &out_r);
            same("c2BBVerts input", &bb_c, &bb_r);

            let circle = Circle {
                p: rng.v(),
                r: rng.positive(),
            };
            let capsule = Capsule {
                a: rng.v(),
                b: rng.v(),
                r: rng.positive(),
            };
            for shape_type in 0..=2 {
                let initial = Proxy {
                    radius: -91.0,
                    count: -17,
                    verts: [sentinel; 8],
                };
                let mut c_out = initial;
                let mut r_out = initial;
                let shape = match shape_type {
                    0 => (&circle as *const Circle).cast(),
                    1 => (&bb_c as *const Aabb).cast(),
                    _ => (&capsule as *const Capsule).cast(),
                };
                c_proxy(shape, shape_type, &mut c_out);
                r_proxy(shape, shape_type, &mut r_out);
                same("c2MakeProxy", &c_out, &r_out);
            }
        }
    }
}

#[test]
fn simplex_branch_surface() {
    let pair = Pair::load();
    unsafe {
        let (c_metric, r_metric) =
            pair.funcs::<unsafe extern "C" fn(*mut Simplex) -> f32>(b"c2GJKSimplexMetric\0");
        let (c_22, r_22) = pair.funcs::<unsafe extern "C" fn(*mut Simplex)>(b"c22\0");
        let (c_23, r_23) = pair.funcs::<unsafe extern "C" fn(*mut Simplex)>(b"c23\0");
        let (c_d, r_d) = pair.funcs::<unsafe extern "C" fn(*mut Simplex) -> V>(b"c2D\0");
        let (c_witness, r_witness) =
            pair.funcs::<unsafe extern "C" fn(*mut Simplex, *mut V, *mut V)>(b"c2Witness\0");
        let (c_l, r_l) = pair.funcs::<unsafe extern "C" fn(*mut Simplex) -> V>(b"c2L\0");

        let edge_cases = [
            (
                V { x: 0.0, y: 0.0 },
                V { x: 2.0, y: 0.0 },
                1,
                V { x: 0.0, y: 0.0 },
            ),
            (
                V { x: 2.0, y: 0.0 },
                V { x: 0.0, y: 0.0 },
                1,
                V { x: 0.0, y: 0.0 },
            ),
            (
                V { x: -1.0, y: 0.0 },
                V { x: 1.0, y: 0.0 },
                2,
                V { x: -1.0, y: 0.0 },
            ),
        ];
        let triangle_cases = [
            (
                V { x: 0.0, y: 0.0 },
                V { x: 2.0, y: 0.0 },
                V { x: 0.0, y: 2.0 },
                1,
                V { x: 0.0, y: 0.0 },
                None,
            ),
            (
                V { x: 2.0, y: 0.0 },
                V { x: 0.0, y: 0.0 },
                V { x: 0.0, y: 2.0 },
                1,
                V { x: 0.0, y: 0.0 },
                None,
            ),
            (
                V { x: 2.0, y: 0.0 },
                V { x: 0.0, y: 2.0 },
                V { x: 0.0, y: 0.0 },
                1,
                V { x: 0.0, y: 0.0 },
                None,
            ),
            (
                V { x: -1.0, y: 1.0 },
                V { x: 1.0, y: 1.0 },
                V { x: 0.0, y: 3.0 },
                2,
                V { x: -1.0, y: 1.0 },
                Some(V { x: 1.0, y: 1.0 }),
            ),
            (
                V { x: -3.0, y: 0.0 },
                V { x: -1.0, y: 1.0 },
                V { x: -1.0, y: -1.0 },
                2,
                V { x: -1.0, y: 1.0 },
                Some(V { x: -1.0, y: -1.0 }),
            ),
            (
                V { x: 1.0, y: -1.0 },
                V { x: 0.0, y: -3.0 },
                V { x: -1.0, y: -1.0 },
                2,
                V { x: -1.0, y: -1.0 },
                Some(V { x: 1.0, y: -1.0 }),
            ),
            (
                V { x: -2.0, y: -1.0 },
                V { x: 2.0, y: -1.0 },
                V { x: 0.0, y: 2.0 },
                3,
                V { x: -2.0, y: -1.0 },
                Some(V { x: 2.0, y: -1.0 }),
            ),
        ];

        let mut rng = Rng::new(0xe8d8_17bd_aa71_6f49);
        for i in 0..256 {
            let scale = rng.positive();
            for &(a, b, expected_count, expected_a) in &edge_cases {
                let mut c_s = Simplex::default();
                c_s.a.p = transformed(a, scale, i % 2 == 0);
                c_s.b.p = transformed(b, scale, i % 2 == 0);
                c_s.a.s_a = rng.v();
                c_s.a.s_b = rng.v();
                c_s.b.s_a = rng.v();
                c_s.b.s_b = rng.v();
                let mut r_s = c_s;
                c_22(&mut c_s);
                r_22(&mut r_s);
                same("c22 simplex", &c_s, &r_s);
                assert_eq!(c_s.count, expected_count, "c22 branch fixture");
                same(
                    "c22 selected vertex",
                    &c_s.a.p,
                    &transformed(expected_a, scale, i % 2 == 0),
                );
            }

            for &(a, b, c, expected_count, expected_a, expected_b) in &triangle_cases {
                let mut c_s = Simplex::default();
                c_s.a.p = transformed(a, scale, i % 2 == 0);
                c_s.b.p = transformed(b, scale, i % 2 == 0);
                c_s.c.p = transformed(c, scale, i % 2 == 0);
                c_s.a.s_a = rng.v();
                c_s.a.s_b = rng.v();
                c_s.b.s_a = rng.v();
                c_s.b.s_b = rng.v();
                c_s.c.s_a = rng.v();
                c_s.c.s_b = rng.v();
                let mut r_s = c_s;
                c_23(&mut c_s);
                r_23(&mut r_s);
                same("c23 simplex", &c_s, &r_s);
                assert_eq!(c_s.count, expected_count, "c23 branch fixture");
                same(
                    "c23 selected vertex A",
                    &c_s.a.p,
                    &transformed(expected_a, scale, i % 2 == 0),
                );
                if let Some(expected_b) = expected_b {
                    same(
                        "c23 selected vertex B",
                        &c_s.b.p,
                        &transformed(expected_b, scale, i % 2 == 0),
                    );
                }
            }

            for count in [1, 2, 3, 9] {
                let mut c_s = Simplex::default();
                c_s.count = count;
                c_s.div = rng.positive();
                c_s.a = Sv {
                    s_a: rng.v(),
                    s_b: rng.v(),
                    p: rng.v(),
                    u: rng.positive(),
                    i_a: 0,
                    i_b: 1,
                };
                c_s.b = Sv {
                    s_a: rng.v(),
                    s_b: rng.v(),
                    p: rng.v(),
                    u: rng.positive(),
                    i_a: 1,
                    i_b: 2,
                };
                c_s.c = Sv {
                    s_a: rng.v(),
                    s_b: rng.v(),
                    p: rng.v(),
                    u: rng.positive(),
                    i_a: 2,
                    i_b: 0,
                };
                let mut r_s = c_s;
                same_f32("c2GJKSimplexMetric", c_metric(&mut c_s), r_metric(&mut r_s));
                same("c2D", &c_d(&mut c_s), &r_d(&mut r_s));
                same("c2L", &c_l(&mut c_s), &r_l(&mut r_s));
                let mut ca = V::default();
                let mut cb = V::default();
                let mut ra = V::default();
                let mut rb = V::default();
                c_witness(&mut c_s, &mut ca, &mut cb);
                r_witness(&mut r_s, &mut ra, &mut rb);
                same("c2Witness A", &ca, &ra);
                same("c2Witness B", &cb, &rb);
            }
        }

        for (a, b) in [
            (V { x: 1.0, y: 0.0 }, V { x: 1.0, y: 1.0 }),
            (V { x: 1.0, y: 0.0 }, V { x: 1.0, y: -1.0 }),
        ] {
            let mut c_s = Simplex::default();
            c_s.count = 2;
            c_s.a.p = a;
            c_s.b.p = b;
            let mut r_s = c_s;
            same("c2D orientation", &c_d(&mut c_s), &r_d(&mut r_s));
        }
    }
}

#[test]
fn support_surface() {
    let pair = Pair::load();
    unsafe {
        let (c_support, r_support) =
            pair.funcs::<unsafe extern "C" fn(*const V, c_int, V) -> c_int>(b"c2Support\0");
        let mut rng = Rng::new(0xac34_41a3_50e2_19df);
        for i in 0..1024 {
            let count = (i % 16 + 1) as usize;
            let mut verts = [V::default(); 16];
            for v in &mut verts[..count] {
                *v = rng.v();
            }
            let d = if i % 13 == 0 {
                V { x: 0.0, y: 0.0 }
            } else {
                rng.v()
            };
            assert_eq!(
                c_support(verts.as_ptr(), count as c_int, d),
                r_support(verts.as_ptr(), count as c_int, d),
                "c2Support"
            );
        }

        let first_max = [
            V { x: 3.0, y: 0.0 },
            V { x: 2.0, y: 0.0 },
            V { x: 1.0, y: 0.0 },
        ];
        let later_max = [
            V { x: 1.0, y: 0.0 },
            V { x: 4.0, y: 0.0 },
            V { x: 2.0, y: 0.0 },
        ];
        let tied = [
            V { x: 4.0, y: 0.0 },
            V { x: 4.0, y: 0.0 },
            V { x: 2.0, y: 0.0 },
        ];
        for (verts, expected) in [(first_max, 0), (later_max, 1), (tied, 0)] {
            let direction = V { x: 1.0, y: 0.0 };
            assert_eq!(c_support(verts.as_ptr(), 3, direction), expected);
            assert_eq!(r_support(verts.as_ptr(), 3, direction), expected);
        }
    }
}

enum Shape {
    Circle(Circle),
    Aabb(Aabb),
    Capsule(Capsule),
}

impl Shape {
    fn shape_type(&self) -> c_int {
        match self {
            Self::Circle(_) => 0,
            Self::Aabb(_) => 1,
            Self::Capsule(_) => 2,
        }
    }

    fn ptr(&self) -> *const c_void {
        match self {
            Self::Circle(value) => (value as *const Circle).cast(),
            Self::Aabb(value) => (value as *const Aabb).cast(),
            Self::Capsule(value) => (value as *const Capsule).cast(),
        }
    }
}

fn shape(kind: c_int, center: V, rng: &mut Rng) -> Shape {
    match kind {
        0 => Shape::Circle(Circle {
            p: center,
            r: rng.positive().min(2.0),
        }),
        1 => {
            let ex = rng.positive().min(2.0);
            let ey = rng.positive().min(2.0);
            Shape::Aabb(Aabb {
                min: V {
                    x: center.x - ex,
                    y: center.y - ey,
                },
                max: V {
                    x: center.x + ex,
                    y: center.y + ey,
                },
            })
        }
        _ => {
            let dx = rng.positive().min(2.0);
            let dy = rng.positive().min(2.0);
            Shape::Capsule(Capsule {
                a: V {
                    x: center.x - dx,
                    y: center.y - dy,
                },
                b: V {
                    x: center.x + dx,
                    y: center.y + dy,
                },
                r: rng.positive().min(2.0),
            })
        }
    }
}

type Gjk = unsafe extern "C" fn(
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
) -> f32;

#[allow(clippy::too_many_arguments)]
unsafe fn compare_gjk_call(
    c_gjk: Gjk,
    r_gjk: Gjk,
    a: &Shape,
    ax: Option<&X>,
    b: &Shape,
    bx: Option<&X>,
    use_radius: c_int,
    output: bool,
    initial_cache: Option<Cache>,
) -> Option<Cache> {
    let ax_ptr = ax.map_or(std::ptr::null(), std::ptr::from_ref);
    let bx_ptr = bx.map_or(std::ptr::null(), std::ptr::from_ref);
    let mut c_a = V {
        x: -1234.5,
        y: 9876.25,
    };
    let mut c_b = V {
        x: 333.75,
        y: -444.125,
    };
    let mut r_a = c_a;
    let mut r_b = c_b;
    let mut c_iterations = -99;
    let mut r_iterations = -99;
    let mut c_cache = initial_cache;
    let mut r_cache = initial_cache;
    let c_distance = unsafe {
        c_gjk(
            a.ptr(),
            a.shape_type(),
            ax_ptr,
            b.ptr(),
            b.shape_type(),
            bx_ptr,
            if output {
                &mut c_a
            } else {
                std::ptr::null_mut()
            },
            if output {
                &mut c_b
            } else {
                std::ptr::null_mut()
            },
            use_radius,
            if output {
                &mut c_iterations
            } else {
                std::ptr::null_mut()
            },
            c_cache
                .as_mut()
                .map_or(std::ptr::null_mut(), std::ptr::from_mut),
        )
    };
    let r_distance = unsafe {
        r_gjk(
            a.ptr(),
            a.shape_type(),
            ax_ptr,
            b.ptr(),
            b.shape_type(),
            bx_ptr,
            if output {
                &mut r_a
            } else {
                std::ptr::null_mut()
            },
            if output {
                &mut r_b
            } else {
                std::ptr::null_mut()
            },
            use_radius,
            if output {
                &mut r_iterations
            } else {
                std::ptr::null_mut()
            },
            r_cache
                .as_mut()
                .map_or(std::ptr::null_mut(), std::ptr::from_mut),
        )
    };
    same_f32("c2GJK distance", c_distance, r_distance);
    if output {
        same("c2GJK witness A", &c_a, &r_a);
        same("c2GJK witness B", &c_b, &r_b);
        assert_eq!(c_iterations, r_iterations, "c2GJK iterations");
    }
    match (&c_cache, &r_cache) {
        (Some(c), Some(rust)) => same("c2GJK cache", c, rust),
        (None, None) => {}
        _ => panic!("cache option mismatch"),
    }
    c_cache
}

#[test]
fn full_gjk_configuration_surface() {
    let pair = Pair::load();
    unsafe {
        let (c_gjk, r_gjk) = pair.funcs::<Gjk>(b"c2GJK\0");
        let mut rng = Rng::new(0x2c1b_3c6d_5a7e_9811);
        for type_a in 0..=2 {
            for type_b in 0..=2 {
                for i in 0..96 {
                    let overlap = i % 2 == 0;
                    let center_a = if overlap {
                        V { x: 0.0, y: 0.0 }
                    } else {
                        V { x: -12.0, y: -3.0 }
                    };
                    let center_b = if overlap {
                        V {
                            x: (i % 3) as f32 * 0.05,
                            y: -((i % 5) as f32) * 0.05,
                        }
                    } else {
                        V { x: 12.0, y: 4.0 }
                    };
                    let a = shape(type_a, center_a, &mut rng);
                    let b = shape(type_b, center_b, &mut rng);
                    let ax = X {
                        p: V {
                            x: (i % 3) as f32 * 0.125,
                            y: -((i % 7) as f32) * 0.0625,
                        },
                        r: if i % 2 == 0 {
                            R { c: 0.0, s: 1.0 }
                        } else {
                            R { c: 1.0, s: 0.0 }
                        },
                    };
                    let bx = X {
                        p: V {
                            x: -((i % 4) as f32) * 0.125,
                            y: (i % 6) as f32 * 0.0625,
                        },
                        r: if i % 3 == 0 {
                            R { c: 0.0, s: -1.0 }
                        } else {
                            R { c: 1.0, s: 0.0 }
                        },
                    };
                    let transforms = if i % 4 == 0 {
                        (Some(&ax), Some(&bx))
                    } else {
                        (None, None)
                    };
                    let use_radius = if i % 3 == 0 { 7 } else { 0 };
                    let output = i % 5 != 0;
                    match i % 3 {
                        0 => {
                            compare_gjk_call(
                                c_gjk,
                                r_gjk,
                                &a,
                                transforms.0,
                                &b,
                                transforms.1,
                                use_radius,
                                output,
                                None,
                            );
                        }
                        1 => {
                            compare_gjk_call(
                                c_gjk,
                                r_gjk,
                                &a,
                                transforms.0,
                                &b,
                                transforms.1,
                                use_radius,
                                output,
                                Some(Cache::default()),
                            );
                        }
                        _ => {
                            let primed = compare_gjk_call(
                                c_gjk,
                                r_gjk,
                                &a,
                                transforms.0,
                                &b,
                                transforms.1,
                                use_radius,
                                true,
                                Some(Cache::default()),
                            )
                            .unwrap();
                            compare_gjk_call(
                                c_gjk,
                                r_gjk,
                                &a,
                                transforms.0,
                                &b,
                                transforms.1,
                                use_radius,
                                output,
                                Some(primed),
                            );
                        }
                    }
                }
            }
        }

        let hit_a = Shape::Aabb(Aabb {
            min: V {
                x: -2.842_909_1,
                y: -2.738_44,
            },
            max: V {
                x: 1.221_725_3,
                y: 0.323_100_72,
            },
        });
        let hit_b = Shape::Capsule(Capsule {
            a: V {
                x: 1.604_795_5,
                y: -1.847_385_3,
            },
            b: V {
                x: 0.144_808_16,
                y: -0.671_209_2,
            },
            r: 0.852_086_6,
        });
        let hit_cache = compare_gjk_call(
            c_gjk,
            r_gjk,
            &hit_a,
            None,
            &hit_b,
            None,
            0,
            true,
            Some(Cache::default()),
        )
        .unwrap();
        assert_eq!(hit_cache.count, 3, "fixture must reach the hit branch");

        let separated_a = Shape::Aabb(Aabb {
            min: V { x: -1.0, y: -1.0 },
            max: V { x: 1.0, y: 1.0 },
        });
        let separated_b = Shape::Aabb(Aabb {
            min: V { x: 10.0, y: -1.0 },
            max: V { x: 12.0, y: 1.0 },
        });
        let duplicate_cache = compare_gjk_call(
            c_gjk,
            r_gjk,
            &separated_a,
            None,
            &separated_b,
            None,
            0,
            true,
            Some(Cache::default()),
        )
        .unwrap();
        assert_eq!(duplicate_cache.count, 1);
        assert_eq!(duplicate_cache.i_a[0], 1);
        assert_eq!(duplicate_cache.i_b[0], 0);
    }
}

#[test]
fn public_gjk_wrapper_surface() {
    type Wrapper =
        unsafe extern "C" fn(c_char, *mut V, *mut V, f32, f32, f32, f32, f32, f32, f32, f32, f32);
    let pair = Pair::load();
    unsafe {
        let (c_gjk, r_gjk) = pair.funcs::<Wrapper>(b"gjk\0");
        let mut rng = Rng::new(0x75a0_c3e1_b249_0f86);
        for i in 0..1024 {
            let overlap = i % 2 == 0;
            let cx = rng.f() * 0.05;
            let cy = rng.f() * 0.05;
            let ex = rng.positive().min(3.0);
            let ey = rng.positive().min(3.0);
            let (cap_x, cap_y) = if overlap {
                (cx, cy)
            } else {
                (cx + 20.0, cy - 15.0)
            };
            let args = (
                cx - ex,
                cy - ey,
                cx + ex,
                cy + ey,
                cap_x - 1.0,
                cap_y - 0.5,
                cap_x + 1.0,
                cap_y + 0.5,
                rng.positive().min(2.0),
            );
            for reverse in [0, 1, -1] {
                let mut ca = V::default();
                let mut cb = V::default();
                let mut ra = V::default();
                let mut rb = V::default();
                c_gjk(
                    reverse, &mut ca, &mut cb, args.0, args.1, args.2, args.3, args.4, args.5,
                    args.6, args.7, args.8,
                );
                r_gjk(
                    reverse, &mut ra, &mut rb, args.0, args.1, args.2, args.3, args.4, args.5,
                    args.6, args.7, args.8,
                );
                same("gjk output A", &ca, &ra);
                same("gjk output B", &cb, &rb);
            }
        }
    }
}

#[test]
fn boundary_behavior_surface() {
    let pair = Pair::load();
    unsafe {
        let (c_proxy, r_proxy) =
            pair.funcs::<unsafe extern "C" fn(*const c_void, c_int, *mut Proxy)>(b"c2MakeProxy\0");
        let circle = Circle {
            p: V { x: 1.0, y: 2.0 },
            r: 3.0,
        };
        for invalid in [-17, 3, c_int::MAX] {
            let initial = Proxy {
                radius: f32::from_bits(0x7fc0_1234),
                count: -123,
                verts: [V { x: 91.25, y: -17.5 }; 8],
            };
            let mut c_out = initial;
            let mut r_out = initial;
            c_proxy((&circle as *const Circle).cast(), invalid, &mut c_out);
            r_proxy((&circle as *const Circle).cast(), invalid, &mut r_out);
            same("invalid enum c2MakeProxy", &c_out, &r_out);
            same("C invalid enum is no-op", &c_out, &initial);
        }

        let (c_support, r_support) =
            pair.funcs::<unsafe extern "C" fn(*const V, c_int, V) -> c_int>(b"c2Support\0");
        let verts = [V { x: 1.0, y: 2.0 }; 16];
        let d = V { x: 3.0, y: 4.0 };
        assert_eq!(c_support(verts.as_ptr(), 0, d), 0);
        assert_eq!(r_support(verts.as_ptr(), 0, d), 0);
        assert_eq!(
            c_support(verts.as_ptr(), 16, d),
            r_support(verts.as_ptr(), 16, d)
        );

        let (c_div, r_div) = pair.funcs::<unsafe extern "C" fn(V, f32) -> V>(b"c2Div\0");
        let (c_norm, r_norm) = pair.funcs::<unsafe extern "C" fn(V) -> V>(b"c2Norm\0");
        for v in [V { x: 1.0, y: -1.0 }, V { x: 0.0, y: 0.0 }] {
            same("zero division", &c_div(v, 0.0), &r_div(v, 0.0));
        }
        same(
            "zero-vector normalization",
            &c_norm(V::default()),
            &r_norm(V::default()),
        );

        let (c_gjk, r_gjk) = pair.funcs::<Gjk>(b"c2GJK\0");
        let a = Shape::Aabb(Aabb {
            min: V { x: -1.0, y: -1.0 },
            max: V { x: 1.0, y: 1.0 },
        });
        let b = Shape::Circle(Circle {
            p: V { x: 4.0, y: 0.0 },
            r: 0.5,
        });
        compare_gjk_call(c_gjk, r_gjk, &a, None, &b, None, 1, false, None);

        type Wrapper = unsafe extern "C" fn(
            c_char,
            *mut V,
            *mut V,
            f32,
            f32,
            f32,
            f32,
            f32,
            f32,
            f32,
            f32,
            f32,
        );
        let (c_wrapper, r_wrapper) = pair.funcs::<Wrapper>(b"gjk\0");
        c_wrapper(
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            -1.0,
            -1.0,
            1.0,
            1.0,
            3.0,
            0.0,
            4.0,
            0.0,
            0.5,
        );
        r_wrapper(
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            -1.0,
            -1.0,
            1.0,
            1.0,
            3.0,
            0.0,
            4.0,
            0.0,
            0.5,
        );
    }
}

#[test]
fn required_null_pointer_has_same_process_rejection() {
    use std::os::unix::process::ExitStatusExt;
    use std::process::Command;

    let executable = std::env::current_exe().unwrap();
    let run = |library: &Path| {
        Command::new(&executable)
            .arg("--exact")
            .arg("required_null_probe")
            .arg("--ignored")
            .arg("--nocapture")
            .env("GJK_NULL_PROBE_LIBRARY", library)
            .status()
            .unwrap()
    };
    let c_status = run(&c_library());
    let rust_status = run(&rust_library());
    assert!(!c_status.success(), "C null probe unexpectedly succeeded");
    assert!(
        !rust_status.success(),
        "Rust null probe unexpectedly succeeded"
    );
    assert_eq!(
        c_status.signal(),
        rust_status.signal(),
        "required-null process signal"
    );
}

#[test]
#[ignore]
fn required_null_probe() {
    let Some(path) = std::env::var_os("GJK_NULL_PROBE_LIBRARY") else {
        return;
    };
    unsafe {
        let library = Library::new(path).unwrap();
        let function = library
            .get::<unsafe extern "C" fn(*mut V, *mut Aabb)>(b"c2BBVerts\0")
            .unwrap();
        let mut bb = Aabb::default();
        function(std::ptr::null_mut(), &mut bb);
    }
}
