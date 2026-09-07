use libloading::Library;
use std::ffi::{c_float, c_int, c_void};
use std::fmt::Debug;
use std::mem::size_of;
use std::path::PathBuf;
use std::process::Command;
use std::ptr::{null, null_mut};

const CIRCLE: c_int = 0;
const AABB: c_int = 1;
const CAPSULE: c_int = 2;

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
struct V {
    x: c_float,
    y: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
struct R {
    c: c_float,
    s: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
struct X {
    p: V,
    r: R,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
struct Circle {
    p: V,
    r: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
struct Aabb {
    min: V,
    max: V,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
struct Capsule {
    a: V,
    b: V,
    r: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
struct Cache {
    metric: c_float,
    count: c_int,
    i_a: [c_int; 3],
    i_b: [c_int; 3],
    div: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
struct Proxy {
    radius: c_float,
    count: c_int,
    verts: [V; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
struct Sv {
    s_a: V,
    s_b: V,
    p: V,
    u: c_float,
    i_a: c_int,
    i_b: c_int,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
struct Simplex {
    a: Sv,
    b: Sv,
    c: Sv,
    d: Sv,
    div: c_float,
    count: c_int,
}

struct LibPair {
    _libm: Library,
    c: Library,
    rust: Library,
}

impl LibPair {
    fn load() -> Self {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_path = manifest.join("../c_src/build/libharvest-work-nUa2tT.so");
        let rust_path = manifest.join("target/release/libreverse_collide_lib.so");
        assert!(
            c_path.exists(),
            "missing C shared library: {}",
            c_path.display()
        );
        assert!(
            rust_path.exists(),
            "missing Rust shared library: {}",
            rust_path.display()
        );
        unsafe {
            let libm: Library = libloading::os::unix::Library::open(
                Some("libm.so.6"),
                libloading::os::unix::RTLD_NOW | libloading::os::unix::RTLD_GLOBAL,
            )
            .unwrap()
            .into();
            Self {
                _libm: libm,
                c: Library::new(c_path).unwrap(),
                rust: Library::new(rust_path).unwrap(),
            }
        }
    }

    unsafe fn symbols<T: Copy>(&self, name: &[u8]) -> (T, T) {
        unsafe {
            (
                *self.c.get::<T>(name).unwrap(),
                *self.rust.get::<T>(name).unwrap(),
            )
        }
    }
}

#[derive(Clone)]
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
        x as u32
    }

    fn raw_f32(&mut self) -> f32 {
        f32::from_bits(self.u32())
    }

    fn finite(&mut self) -> f32 {
        (self.u32() as i32 as f32) / 65536.0
    }

    fn positive(&mut self) -> f32 {
        (self.u32() % 20_000 + 1) as f32 / 100.0
    }

    fn v(&mut self) -> V {
        V {
            x: self.finite(),
            y: self.finite(),
        }
    }

    fn raw_v(&mut self) -> V {
        V {
            x: self.raw_f32(),
            y: self.raw_f32(),
        }
    }
}

fn bytes<T>(value: &T) -> &[u8] {
    unsafe { std::slice::from_raw_parts((value as *const T).cast(), size_of::<T>()) }
}

fn assert_bits_eq<T: Debug>(left: &T, right: &T, context: impl Debug) {
    assert_eq!(
        bytes(left),
        bytes(right),
        "byte mismatch for {context:?}\nC: {left:?}\nRust: {right:?}"
    );
}

fn assert_f32_eq(left: f32, right: f32, context: impl Debug) {
    assert_eq!(
        left.to_bits(),
        right.to_bits(),
        "float mismatch for {context:?}: C={left:?} ({:#010x}), Rust={right:?} ({:#010x})",
        left.to_bits(),
        right.to_bits()
    );
}

fn sentinel_proxy() -> Proxy {
    Proxy {
        radius: f32::from_bits(0x7fc0_1234),
        count: 0x1234_5678,
        verts: [V {
            x: f32::from_bits(0x7fc0_5678),
            y: -12345.25,
        }; 8],
    }
}

#[test]
fn symbol_parity() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let paths = [
        manifest.join("../c_src/build/libharvest-work-nUa2tT.so"),
        manifest.join("target/release/libreverse_collide_lib.so"),
    ];
    let mut symbols = Vec::new();
    for path in paths {
        let output = Command::new("nm")
            .args(["-D", "--defined-only", "--format=posix"])
            .arg(path)
            .output()
            .unwrap();
        assert!(output.status.success());
        let mut names: Vec<_> = String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .filter_map(|line| line.split_whitespace().next())
            .map(str::to_owned)
            .collect();
        names.sort();
        symbols.push(names);
    }
    assert_eq!(symbols[0].len(), 38);
    assert_eq!(symbols[0], symbols[1]);
}

#[test]
fn vector_scalar_and_transform_surface() {
    let libs = LibPair::load();
    let mut rng = Rng::new(0x4d59_5df4_d0f3_3173);
    unsafe {
        let (c_v, r_v): (
            unsafe extern "C" fn(f32, f32) -> V,
            unsafe extern "C" fn(f32, f32) -> V,
        ) = libs.symbols(b"c2V\0");
        let (c_mulvs, r_mulvs): (
            unsafe extern "C" fn(V, f32) -> V,
            unsafe extern "C" fn(V, f32) -> V,
        ) = libs.symbols(b"c2Mulvs\0");
        let (c_max, r_max): (
            unsafe extern "C" fn(V, V) -> V,
            unsafe extern "C" fn(V, V) -> V,
        ) = libs.symbols(b"c2Maxv\0");
        let (c_min, r_min): (
            unsafe extern "C" fn(V, V) -> V,
            unsafe extern "C" fn(V, V) -> V,
        ) = libs.symbols(b"c2Minv\0");
        let (c_clamp, r_clamp): (
            unsafe extern "C" fn(V, V, V) -> V,
            unsafe extern "C" fn(V, V, V) -> V,
        ) = libs.symbols(b"c2Clampv\0");
        let (c_sub, r_sub): (
            unsafe extern "C" fn(V, V) -> V,
            unsafe extern "C" fn(V, V) -> V,
        ) = libs.symbols(b"c2Sub\0");
        let (c_add, r_add): (
            unsafe extern "C" fn(V, V) -> V,
            unsafe extern "C" fn(V, V) -> V,
        ) = libs.symbols(b"c2Add\0");
        let (c_dot, r_dot): (
            unsafe extern "C" fn(V, V) -> f32,
            unsafe extern "C" fn(V, V) -> f32,
        ) = libs.symbols(b"c2Dot\0");
        let (c_det, r_det): (
            unsafe extern "C" fn(V, V) -> f32,
            unsafe extern "C" fn(V, V) -> f32,
        ) = libs.symbols(b"c2Det2\0");
        let (c_neg, r_neg): (unsafe extern "C" fn(V) -> V, unsafe extern "C" fn(V) -> V) =
            libs.symbols(b"c2Neg\0");
        let (c_skew, r_skew): (unsafe extern "C" fn(V) -> V, unsafe extern "C" fn(V) -> V) =
            libs.symbols(b"c2Skew\0");
        let (c_ccw, r_ccw): (unsafe extern "C" fn(V) -> V, unsafe extern "C" fn(V) -> V) =
            libs.symbols(b"c2CCW90\0");
        let (c_mulrv, r_mulrv): (
            unsafe extern "C" fn(R, V) -> V,
            unsafe extern "C" fn(R, V) -> V,
        ) = libs.symbols(b"c2Mulrv\0");
        let (c_mulrvt, r_mulrvt): (
            unsafe extern "C" fn(R, V) -> V,
            unsafe extern "C" fn(R, V) -> V,
        ) = libs.symbols(b"c2MulrvT\0");
        let (c_mulxv, r_mulxv): (
            unsafe extern "C" fn(X, V) -> V,
            unsafe extern "C" fn(X, V) -> V,
        ) = libs.symbols(b"c2Mulxv\0");

        let edges = [
            0.0,
            -0.0,
            1.0,
            -1.0,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::from_bits(0x7fc0_1234),
            f32::from_bits(0xffc0_5678),
            f32::MIN_POSITIVE,
            f32::from_bits(1),
            f32::MAX,
            f32::MIN,
        ];
        for &x in &edges {
            for &y in &edges {
                assert_bits_eq(&c_v(x, y), &r_v(x, y), ("c2V", x, y));
            }
        }

        for i in 0..2_000 {
            let a = rng.raw_v();
            let b = rng.raw_v();
            let scalar = rng.raw_f32();
            assert_bits_eq(
                &c_mulvs(a, scalar),
                &r_mulvs(a, scalar),
                ("c2Mulvs", i, a, scalar),
            );
            assert_bits_eq(&c_max(a, b), &r_max(a, b), ("c2Maxv", i, a, b));
            assert_bits_eq(&c_min(a, b), &r_min(a, b), ("c2Minv", i, a, b));
            assert_bits_eq(&c_sub(a, b), &r_sub(a, b), ("c2Sub", i, a, b));
            assert_bits_eq(&c_add(a, b), &r_add(a, b), ("c2Add", i, a, b));
            assert_f32_eq(c_dot(a, b), r_dot(a, b), ("c2Dot", i, a, b));
            assert_f32_eq(c_det(a, b), r_det(a, b), ("c2Det2", i, a, b));
            assert_bits_eq(&c_neg(a), &r_neg(a), ("c2Neg", i, a));
            assert_bits_eq(&c_skew(a), &r_skew(a), ("c2Skew", i, a));
            assert_bits_eq(&c_ccw(a), &r_ccw(a), ("c2CCW90", i, a));

            let lo = V {
                x: -rng.positive(),
                y: -rng.positive(),
            };
            let hi = V {
                x: rng.positive(),
                y: rng.positive(),
            };
            assert_bits_eq(
                &c_clamp(a, lo, hi),
                &r_clamp(a, lo, hi),
                ("c2Clampv", i, a, lo, hi),
            );

            let rot = R {
                c: rng.raw_f32(),
                s: rng.raw_f32(),
            };
            let transform = X { p: b, r: rot };
            assert_bits_eq(&c_mulrv(rot, a), &r_mulrv(rot, a), ("c2Mulrv", i, rot, a));
            assert_bits_eq(
                &c_mulrvt(rot, a),
                &r_mulrvt(rot, a),
                ("c2MulrvT", i, rot, a),
            );
            assert_bits_eq(
                &c_mulxv(transform, a),
                &r_mulxv(transform, a),
                ("c2Mulxv", i, transform, a),
            );
        }
    }
}

#[test]
fn constructors_bounds_proxies_and_norms() {
    let libs = LibPair::load();
    let mut rng = Rng::new(0x9e37_79b9_7f4a_7c15);
    unsafe {
        let (c_ri, r_ri): (unsafe extern "C" fn() -> R, unsafe extern "C" fn() -> R) =
            libs.symbols(b"c2RotIdentity\0");
        let (c_xi, r_xi): (unsafe extern "C" fn() -> X, unsafe extern "C" fn() -> X) =
            libs.symbols(b"c2xIdentity\0");
        let (c_bb, r_bb): (
            unsafe extern "C" fn(*mut V, *mut Aabb),
            unsafe extern "C" fn(*mut V, *mut Aabb),
        ) = libs.symbols(b"c2BBVerts\0");
        let (c_proxy, r_proxy): (
            unsafe extern "C" fn(*const c_void, c_int, *mut Proxy),
            unsafe extern "C" fn(*const c_void, c_int, *mut Proxy),
        ) = libs.symbols(b"c2MakeProxy\0");
        let (c_len, r_len): (
            unsafe extern "C" fn(V) -> f32,
            unsafe extern "C" fn(V) -> f32,
        ) = libs.symbols(b"c2Len\0");
        let (c_div, r_div): (
            unsafe extern "C" fn(V, f32) -> V,
            unsafe extern "C" fn(V, f32) -> V,
        ) = libs.symbols(b"c2Div\0");
        let (c_norm, r_norm): (unsafe extern "C" fn(V) -> V, unsafe extern "C" fn(V) -> V) =
            libs.symbols(b"c2Norm\0");

        assert_bits_eq(&c_ri(), &r_ri(), "c2RotIdentity");
        assert_bits_eq(&c_xi(), &r_xi(), "c2xIdentity");

        for i in 0..1_000 {
            let mut bb_c = Aabb {
                min: rng.raw_v(),
                max: rng.raw_v(),
            };
            let mut bb_r = bb_c;
            let mut out_c = [V::default(); 4];
            let mut out_r = [V::default(); 4];
            c_bb(out_c.as_mut_ptr(), &mut bb_c);
            r_bb(out_r.as_mut_ptr(), &mut bb_r);
            assert_bits_eq(&out_c, &out_r, ("c2BBVerts", i, bb_c));

            let circle = Circle {
                p: rng.raw_v(),
                r: rng.raw_f32(),
            };
            let aabb = Aabb {
                min: rng.raw_v(),
                max: rng.raw_v(),
            };
            let capsule = Capsule {
                a: rng.raw_v(),
                b: rng.raw_v(),
                r: rng.raw_f32(),
            };
            for (kind, ptr) in [
                (CIRCLE, (&circle as *const Circle).cast::<c_void>()),
                (AABB, (&aabb as *const Aabb).cast::<c_void>()),
                (CAPSULE, (&capsule as *const Capsule).cast::<c_void>()),
            ] {
                let mut pc = sentinel_proxy();
                let mut pr = pc;
                c_proxy(ptr, kind, &mut pc);
                r_proxy(ptr, kind, &mut pr);
                assert_bits_eq(&pc, &pr, ("c2MakeProxy", i, kind));
            }

            let v = rng.raw_v();
            let divisor = rng.raw_f32();
            assert_f32_eq(c_len(v), r_len(v), ("c2Len", i, v));
            assert_bits_eq(
                &c_div(v, divisor),
                &r_div(v, divisor),
                ("c2Div", i, v, divisor),
            );
            assert_bits_eq(&c_norm(v), &r_norm(v), ("c2Norm", i, v));
        }
    }
}

fn simplex_with_points(a: V, b: V, c: V) -> Simplex {
    Simplex {
        a: Sv {
            s_a: V { x: 1.0, y: 2.0 },
            s_b: V { x: 3.0, y: 4.0 },
            p: a,
            u: -11.0,
            i_a: 10,
            i_b: 11,
        },
        b: Sv {
            s_a: V { x: 5.0, y: 6.0 },
            s_b: V { x: 7.0, y: 8.0 },
            p: b,
            u: -22.0,
            i_a: 20,
            i_b: 21,
        },
        c: Sv {
            s_a: V { x: 9.0, y: 10.0 },
            s_b: V { x: 11.0, y: 12.0 },
            p: c,
            u: -33.0,
            i_a: 30,
            i_b: 31,
        },
        d: Sv {
            s_a: V { x: 13.0, y: 14.0 },
            s_b: V { x: 15.0, y: 16.0 },
            p: V { x: 17.0, y: 18.0 },
            u: -44.0,
            i_a: 40,
            i_b: 41,
        },
        div: -55.0,
        count: 3,
    }
}

#[test]
fn simplex_metric_reduction_direction_support_and_witness() {
    let libs = LibPair::load();
    let mut rng = Rng::new(0xd1b5_4a32_d192_ed03);
    unsafe {
        let (c_metric, r_metric): (
            unsafe extern "C" fn(*mut Simplex) -> f32,
            unsafe extern "C" fn(*mut Simplex) -> f32,
        ) = libs.symbols(b"c2GJKSimplexMetric\0");
        let (c_22, r_22): (
            unsafe extern "C" fn(*mut Simplex),
            unsafe extern "C" fn(*mut Simplex),
        ) = libs.symbols(b"c22\0");
        let (c_23, r_23): (
            unsafe extern "C" fn(*mut Simplex),
            unsafe extern "C" fn(*mut Simplex),
        ) = libs.symbols(b"c23\0");
        let (c_d, r_d): (
            unsafe extern "C" fn(*mut Simplex) -> V,
            unsafe extern "C" fn(*mut Simplex) -> V,
        ) = libs.symbols(b"c2D\0");
        let (c_support, r_support): (
            unsafe extern "C" fn(*const V, c_int, V) -> c_int,
            unsafe extern "C" fn(*const V, c_int, V) -> c_int,
        ) = libs.symbols(b"c2Support\0");
        let (c_witness, r_witness): (
            unsafe extern "C" fn(*mut Simplex, *mut V, *mut V),
            unsafe extern "C" fn(*mut Simplex, *mut V, *mut V),
        ) = libs.symbols(b"c2Witness\0");
        let (c_l, r_l): (
            unsafe extern "C" fn(*mut Simplex) -> V,
            unsafe extern "C" fn(*mut Simplex) -> V,
        ) = libs.symbols(b"c2L\0");

        for count in 1..=3 {
            for i in 0..500 {
                let mut sc = simplex_with_points(rng.v(), rng.v(), rng.v());
                sc.count = count;
                let mut sr = sc;
                assert_f32_eq(
                    c_metric(&mut sc),
                    r_metric(&mut sr),
                    ("metric", count, i, sc),
                );
            }
        }

        let c22_cases = [
            (V { x: 1.0, y: 0.0 }, V { x: 2.0, y: 0.0 }, 10),
            (V { x: -2.0, y: 0.0 }, V { x: -1.0, y: 0.0 }, 20),
            (V { x: -1.0, y: 0.0 }, V { x: 1.0, y: 0.0 }, 2),
        ];
        for (a, b, expected) in c22_cases {
            for i in 0..300 {
                let jitter = V {
                    x: rng.finite() * 1.0e-6,
                    y: rng.finite() * 1.0e-6,
                };
                let mut sc = simplex_with_points(
                    V {
                        x: a.x + jitter.x,
                        y: a.y + jitter.y,
                    },
                    V {
                        x: b.x + jitter.x,
                        y: b.y + jitter.y,
                    },
                    V::default(),
                );
                sc.count = 2;
                let mut sr = sc;
                c_22(&mut sc);
                r_22(&mut sr);
                assert_bits_eq(&sc, &sr, ("c22", expected, i, a, b));
            }
        }

        let mut seen = [false; 7];
        for i in 0..200_000 {
            let mut sc = simplex_with_points(rng.v(), rng.v(), rng.v());
            let mut sr = sc;
            c_23(&mut sc);
            r_23(&mut sr);
            assert_bits_eq(&sc, &sr, ("c23", i));
            let branch = match (sc.count, sc.a.i_a, sc.b.i_a) {
                (1, 10, _) => 0,
                (1, 20, _) => 1,
                (1, 30, _) => 2,
                (2, 10, 20) => 3,
                (2, 20, 30) => 4,
                (2, 30, 10) => 5,
                (3, _, _) => 6,
                other => panic!("unexpected c23 result {other:?}: {sc:?}"),
            };
            seen[branch] = true;
            if seen.iter().all(|x| *x) && i > 5_000 {
                break;
            }
        }
        assert!(
            seen.iter().all(|x| *x),
            "not all c23 branches covered: {seen:?}"
        );

        for count in [1, 2, 3] {
            for i in 0..500 {
                let mut sc = simplex_with_points(rng.v(), rng.v(), rng.v());
                sc.count = count;
                sc.div = rng.positive();
                sc.a.u = rng.positive();
                sc.b.u = rng.positive();
                sc.c.u = rng.positive();
                let mut sr = sc;
                assert_bits_eq(&c_d(&mut sc), &r_d(&mut sr), ("c2D", count, i));
                assert_bits_eq(&c_l(&mut sc), &r_l(&mut sr), ("c2L", count, i));
                let mut ca = V { x: -9.0, y: -8.0 };
                let mut cb = V { x: -7.0, y: -6.0 };
                let mut ra = ca;
                let mut rb = cb;
                c_witness(&mut sc, &mut ca, &mut cb);
                r_witness(&mut sr, &mut ra, &mut rb);
                assert_bits_eq(&(ca, cb), &(ra, rb), ("c2Witness", count, i));
            }
        }

        for count in [1, 2, 3, 8] {
            for i in 0..500 {
                let mut verts = [V::default(); 8];
                for v in &mut verts[..count] {
                    *v = rng.v();
                }
                let direction = rng.v();
                assert_eq!(
                    c_support(verts.as_ptr(), count as c_int, direction),
                    r_support(verts.as_ptr(), count as c_int, direction),
                    "c2Support mismatch count={count} iteration={i}"
                );
            }
        }
        let tied = [
            V { x: 2.0, y: 1.0 },
            V { x: 2.0, y: -9.0 },
            V { x: 1.0, y: 5.0 },
        ];
        assert_eq!(c_support(tied.as_ptr(), 3, V { x: 1.0, y: 0.0 }), 0);
        assert_eq!(r_support(tied.as_ptr(), 3, V { x: 1.0, y: 0.0 }), 0);
    }
}

#[derive(Clone, Copy, Debug)]
enum Shape {
    Circle(Circle),
    Aabb(Aabb),
    Capsule(Capsule),
}

impl Shape {
    fn kind(self) -> c_int {
        match self {
            Self::Circle(_) => CIRCLE,
            Self::Aabb(_) => AABB,
            Self::Capsule(_) => CAPSULE,
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

fn random_shape(rng: &mut Rng, kind: c_int) -> Shape {
    match kind {
        CIRCLE => Shape::Circle(Circle {
            p: rng.v(),
            r: rng.positive(),
        }),
        AABB => {
            let center = rng.v();
            let ex = rng.positive();
            let ey = rng.positive();
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
        CAPSULE => {
            let a = rng.v();
            let mut b = rng.v();
            if a.x == b.x && a.y == b.y {
                b.x += 1.0;
            }
            Shape::Capsule(Capsule {
                a,
                b,
                r: rng.positive(),
            })
        }
        _ => unreachable!(),
    }
}

type GjkFn = unsafe extern "C" fn(
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

unsafe fn compare_gjk(
    c_gjk: GjkFn,
    r_gjk: GjkFn,
    a: &Shape,
    b: &Shape,
    ax: Option<&X>,
    bx: Option<&X>,
    use_radius: c_int,
    output_mask: u8,
    initial_cache: Option<Cache>,
    context: impl Debug,
) -> Option<Cache> {
    let mut ca = V {
        x: -101.25,
        y: 202.5,
    };
    let mut cb = V {
        x: -303.75,
        y: 404.0,
    };
    let mut ra = ca;
    let mut rb = cb;
    let mut ci = -12345;
    let mut ri = ci;
    let mut cc = initial_cache.unwrap_or_default();
    let mut rc = cc;
    let ap = ax.map_or(null(), |value| value);
    let bp = bx.map_or(null(), |value| value);
    let ca_ptr = if output_mask & 1 != 0 {
        &mut ca
    } else {
        null_mut()
    };
    let cb_ptr = if output_mask & 2 != 0 {
        &mut cb
    } else {
        null_mut()
    };
    let ra_ptr = if output_mask & 1 != 0 {
        &mut ra
    } else {
        null_mut()
    };
    let rb_ptr = if output_mask & 2 != 0 {
        &mut rb
    } else {
        null_mut()
    };
    let ci_ptr = if output_mask & 4 != 0 {
        &mut ci
    } else {
        null_mut()
    };
    let ri_ptr = if output_mask & 4 != 0 {
        &mut ri
    } else {
        null_mut()
    };
    let cc_ptr = if initial_cache.is_some() {
        &mut cc
    } else {
        null_mut()
    };
    let rc_ptr = if initial_cache.is_some() {
        &mut rc
    } else {
        null_mut()
    };

    let cd = unsafe {
        c_gjk(
            a.ptr(),
            a.kind(),
            ap,
            b.ptr(),
            b.kind(),
            bp,
            ca_ptr,
            cb_ptr,
            use_radius,
            ci_ptr,
            cc_ptr,
        )
    };
    let rd = unsafe {
        r_gjk(
            a.ptr(),
            a.kind(),
            ap,
            b.ptr(),
            b.kind(),
            bp,
            ra_ptr,
            rb_ptr,
            use_radius,
            ri_ptr,
            rc_ptr,
        )
    };
    assert_f32_eq(cd, rd, (&context, "distance"));
    assert_bits_eq(&ca, &ra, (&context, "outA"));
    assert_bits_eq(&cb, &rb, (&context, "outB"));
    assert_eq!(ci, ri, "iteration mismatch for {context:?}");
    if initial_cache.is_some() {
        assert_bits_eq(&cc, &rc, (&context, "cache"));
        Some(cc)
    } else {
        None
    }
}

#[test]
fn gjk_all_shape_options_transforms_outputs_and_caches() {
    let libs = LibPair::load();
    let mut rng = Rng::new(0xa076_1d64_78bd_642f);
    unsafe {
        let (c_gjk, r_gjk): (GjkFn, GjkFn) = libs.symbols(b"c2GJK\0");
        for kind_a in [CIRCLE, AABB, CAPSULE] {
            for kind_b in [CIRCLE, AABB, CAPSULE] {
                for i in 0..300 {
                    let a = random_shape(&mut rng, kind_a);
                    let b = random_shape(&mut rng, kind_b);
                    for use_radius in [0, 1, -7] {
                        compare_gjk(
                            c_gjk,
                            r_gjk,
                            &a,
                            &b,
                            None,
                            None,
                            use_radius,
                            0b111,
                            Some(Cache::default()),
                            ("shape-pair", kind_a, kind_b, i, use_radius),
                        );
                    }
                }
            }
        }

        let a = Shape::Capsule(Capsule {
            a: V { x: -4.0, y: 1.0 },
            b: V { x: 7.0, y: 3.0 },
            r: 2.0,
        });
        let b = Shape::Aabb(Aabb {
            min: V { x: -2.0, y: -5.0 },
            max: V { x: 6.0, y: 8.0 },
        });
        let ax = X {
            p: V { x: 11.0, y: -13.0 },
            r: R { c: 0.6, s: 0.8 },
        };
        let bx = X {
            p: V { x: -17.0, y: 19.0 },
            r: R { c: -0.8, s: 0.6 },
        };
        for transform_mask in 0..4 {
            for output_mask in 0..8 {
                compare_gjk(
                    c_gjk,
                    r_gjk,
                    &a,
                    &b,
                    (transform_mask & 1 != 0).then_some(&ax),
                    (transform_mask & 2 != 0).then_some(&bx),
                    1,
                    output_mask,
                    None,
                    ("pointers", transform_mask, output_mask),
                );
            }
        }

        let mut cache = compare_gjk(
            c_gjk,
            r_gjk,
            &a,
            &b,
            Some(&ax),
            Some(&bx),
            1,
            0b111,
            Some(Cache::default()),
            "cache-fill",
        )
        .unwrap();
        for i in 0..100 {
            cache = compare_gjk(
                c_gjk,
                r_gjk,
                &a,
                &b,
                Some(&ax),
                Some(&bx),
                i & 1,
                0b111,
                Some(cache),
                ("cache-reuse", i),
            )
            .unwrap();
        }

        let huge_a = Shape::Aabb(Aabb {
            min: V {
                x: -10_000.0,
                y: -10_000.0,
            },
            max: V {
                x: 10_000.0,
                y: 10_000.0,
            },
        });
        let huge_b = Shape::Aabb(Aabb {
            min: V {
                x: 30_000.0,
                y: 40_000.0,
            },
            max: V {
                x: 50_000.0,
                y: 60_000.0,
            },
        });
        let rejected = Cache {
            metric: 1.0,
            count: 3,
            i_a: [0, 2, 1],
            i_b: [0, 0, 0],
            div: 1.0,
        };
        compare_gjk(
            c_gjk,
            r_gjk,
            &huge_a,
            &huge_b,
            None,
            None,
            1,
            0b111,
            Some(rejected),
            "cache-metric-rejection",
        );

        let explicit = [
            (
                Shape::Circle(Circle {
                    p: V { x: 0.0, y: 0.0 },
                    r: 2.0,
                }),
                Shape::Circle(Circle {
                    p: V { x: 10.0, y: 0.0 },
                    r: 3.0,
                }),
            ),
            (
                Shape::Circle(Circle {
                    p: V { x: 0.0, y: 0.0 },
                    r: 2.0,
                }),
                Shape::Circle(Circle {
                    p: V { x: 5.0, y: 0.0 },
                    r: 3.0,
                }),
            ),
            (
                Shape::Circle(Circle {
                    p: V { x: 0.0, y: 0.0 },
                    r: 10.0,
                }),
                Shape::Circle(Circle {
                    p: V { x: 1.0, y: 0.0 },
                    r: 3.0,
                }),
            ),
        ];
        for (i, (a, b)) in explicit.iter().enumerate() {
            for use_radius in [0, 1] {
                compare_gjk(
                    c_gjk,
                    r_gjk,
                    a,
                    b,
                    None,
                    None,
                    use_radius,
                    0b111,
                    Some(Cache::default()),
                    ("separate-touch-overlap", i, use_radius),
                );
            }
        }
    }
}

#[test]
fn collision_wrappers_dispatch_and_reverse_driver() {
    let libs = LibPair::load();
    let mut rng = Rng::new(0xe703_7ed1_a0b4_28db);
    unsafe {
        let (c_aa, r_aa): (
            unsafe extern "C" fn(Aabb, Aabb) -> c_int,
            unsafe extern "C" fn(Aabb, Aabb) -> c_int,
        ) = libs.symbols(b"c2AABBtoAABB\0");
        let (c_ac, r_ac): (
            unsafe extern "C" fn(Aabb, Capsule) -> c_int,
            unsafe extern "C" fn(Aabb, Capsule) -> c_int,
        ) = libs.symbols(b"c2AABBtoCapsule\0");
        let (c_kk, r_kk): (
            unsafe extern "C" fn(Capsule, Capsule) -> c_int,
            unsafe extern "C" fn(Capsule, Capsule) -> c_int,
        ) = libs.symbols(b"c2CapsuletoCapsule\0");
        let (c_cc, r_cc): (
            unsafe extern "C" fn(Circle, Circle) -> c_int,
            unsafe extern "C" fn(Circle, Circle) -> c_int,
        ) = libs.symbols(b"c2CircletoCircle\0");
        let (c_ca, r_ca): (
            unsafe extern "C" fn(Circle, Aabb) -> c_int,
            unsafe extern "C" fn(Circle, Aabb) -> c_int,
        ) = libs.symbols(b"c2CircletoAABB\0");
        let (c_ck, r_ck): (
            unsafe extern "C" fn(Circle, Capsule) -> c_int,
            unsafe extern "C" fn(Circle, Capsule) -> c_int,
        ) = libs.symbols(b"c2CircletoCapsule\0");
        let (c_collided, r_collided): (
            unsafe extern "C" fn(*const c_void, c_int, *const c_void, c_int) -> c_int,
            unsafe extern "C" fn(*const c_void, c_int, *const c_void, c_int) -> c_int,
        ) = libs.symbols(b"c2Collided\0");
        let (c_reverse, r_reverse): (
            unsafe extern "C" fn(f32, f32, f32) -> c_int,
            unsafe extern "C" fn(f32, f32, f32) -> c_int,
        ) = libs.symbols(b"reverse_collide\0");

        for i in 0..5_000 {
            let circle_a = match random_shape(&mut rng, CIRCLE) {
                Shape::Circle(v) => v,
                _ => unreachable!(),
            };
            let circle_b = match random_shape(&mut rng, CIRCLE) {
                Shape::Circle(v) => v,
                _ => unreachable!(),
            };
            let aabb_a = match random_shape(&mut rng, AABB) {
                Shape::Aabb(v) => v,
                _ => unreachable!(),
            };
            let aabb_b = match random_shape(&mut rng, AABB) {
                Shape::Aabb(v) => v,
                _ => unreachable!(),
            };
            let cap_a = match random_shape(&mut rng, CAPSULE) {
                Shape::Capsule(v) => v,
                _ => unreachable!(),
            };
            let cap_b = match random_shape(&mut rng, CAPSULE) {
                Shape::Capsule(v) => v,
                _ => unreachable!(),
            };
            assert_eq!(c_aa(aabb_a, aabb_b), r_aa(aabb_a, aabb_b), "AABB/AABB {i}");
            assert_eq!(c_ac(aabb_a, cap_b), r_ac(aabb_a, cap_b), "AABB/capsule {i}");
            assert_eq!(
                c_kk(cap_a, cap_b),
                r_kk(cap_a, cap_b),
                "capsule/capsule {i}"
            );
            assert_eq!(
                c_cc(circle_a, circle_b),
                r_cc(circle_a, circle_b),
                "circle/circle {i}"
            );
            assert_eq!(
                c_ca(circle_a, aabb_b),
                r_ca(circle_a, aabb_b),
                "circle/AABB {i}"
            );
            assert_eq!(
                c_ck(circle_a, cap_b),
                r_ck(circle_a, cap_b),
                "circle/capsule {i}"
            );

            let shapes = [
                Shape::Circle(circle_a),
                Shape::Aabb(aabb_a),
                Shape::Capsule(cap_a),
            ];
            let other = [
                Shape::Circle(circle_b),
                Shape::Aabb(aabb_b),
                Shape::Capsule(cap_b),
            ];
            for a in &shapes {
                for b in &other {
                    assert_eq!(
                        c_collided(a.ptr(), a.kind(), b.ptr(), b.kind()),
                        r_collided(a.ptr(), a.kind(), b.ptr(), b.kind()),
                        "c2Collided pair ({},{}) iteration {i}",
                        a.kind(),
                        b.kind()
                    );
                }
            }

            let x = rng.finite();
            let y = rng.finite();
            let radius = rng.positive();
            assert_eq!(
                c_reverse(x, y, radius),
                r_reverse(x, y, radius),
                "reverse {i}"
            );
        }

        let aabb = Aabb {
            min: V { x: 0.0, y: 0.0 },
            max: V { x: 2.0, y: 2.0 },
        };
        let aabb_cases = [
            Aabb {
                min: V { x: 2.0, y: 0.0 },
                max: V { x: 4.0, y: 2.0 },
            },
            Aabb {
                min: V { x: -4.0, y: 0.0 },
                max: V { x: -1.0, y: 2.0 },
            },
            Aabb {
                min: V { x: 3.0, y: 0.0 },
                max: V { x: 4.0, y: 2.0 },
            },
            Aabb {
                min: V { x: 0.0, y: -4.0 },
                max: V { x: 2.0, y: -1.0 },
            },
            Aabb {
                min: V { x: 0.0, y: 3.0 },
                max: V { x: 2.0, y: 4.0 },
            },
        ];
        for b in aabb_cases {
            assert_eq!(c_aa(aabb, b), r_aa(aabb, b));
        }

        let segment = Capsule {
            a: V { x: 0.0, y: 0.0 },
            b: V { x: 10.0, y: 0.0 },
            r: 1.0,
        };
        for circle in [
            Circle {
                p: V { x: -2.0, y: 0.0 },
                r: 0.5,
            },
            Circle {
                p: V { x: 5.0, y: 0.5 },
                r: 0.5,
            },
            Circle {
                p: V { x: 12.0, y: 0.0 },
                r: 0.5,
            },
            Circle {
                p: V { x: 5.0, y: 2.0 },
                r: 1.0,
            },
        ] {
            assert_eq!(c_ck(circle, segment), r_ck(circle, segment));
        }

        for (x, y, radius) in [
            (-70.0, 0.0, 0.0),
            (-50.0, 0.0, 0.0),
            (-40.0, -40.0, 0.0),
            (-27.5, -27.5, 20.0),
            (-40.0, 40.0, 0.0),
            (-20.0, 100.0, 10.0),
            (0.0, 0.0, 1_000.0),
        ] {
            assert_eq!(c_reverse(x, y, radius), r_reverse(x, y, radius));
        }
    }
}

#[test]
fn defined_error_and_boundary_surface() {
    let libs = LibPair::load();
    unsafe {
        let (c_proxy, r_proxy): (
            unsafe extern "C" fn(*const c_void, c_int, *mut Proxy),
            unsafe extern "C" fn(*const c_void, c_int, *mut Proxy),
        ) = libs.symbols(b"c2MakeProxy\0");
        let (c_metric, r_metric): (
            unsafe extern "C" fn(*mut Simplex) -> f32,
            unsafe extern "C" fn(*mut Simplex) -> f32,
        ) = libs.symbols(b"c2GJKSimplexMetric\0");
        let (c_d, r_d): (
            unsafe extern "C" fn(*mut Simplex) -> V,
            unsafe extern "C" fn(*mut Simplex) -> V,
        ) = libs.symbols(b"c2D\0");
        let (c_witness, r_witness): (
            unsafe extern "C" fn(*mut Simplex, *mut V, *mut V),
            unsafe extern "C" fn(*mut Simplex, *mut V, *mut V),
        ) = libs.symbols(b"c2Witness\0");
        let (c_l, r_l): (
            unsafe extern "C" fn(*mut Simplex) -> V,
            unsafe extern "C" fn(*mut Simplex) -> V,
        ) = libs.symbols(b"c2L\0");
        let (c_support, r_support): (
            unsafe extern "C" fn(*const V, c_int, V) -> c_int,
            unsafe extern "C" fn(*const V, c_int, V) -> c_int,
        ) = libs.symbols(b"c2Support\0");
        let (c_collided, r_collided): (
            unsafe extern "C" fn(*const c_void, c_int, *const c_void, c_int) -> c_int,
            unsafe extern "C" fn(*const c_void, c_int, *const c_void, c_int) -> c_int,
        ) = libs.symbols(b"c2Collided\0");

        for invalid in [c_int::MIN, -99, -1, 3, 99, c_int::MAX] {
            let mut pc = sentinel_proxy();
            let mut pr = pc;
            c_proxy(null(), invalid, &mut pc);
            r_proxy(null(), invalid, &mut pr);
            assert_bits_eq(&pc, &pr, ("invalid proxy type", invalid));
            assert_bits_eq(&pc, &sentinel_proxy(), ("proxy unchanged", invalid));
            c_proxy(null(), invalid, null_mut());
            r_proxy(null(), invalid, null_mut());
        }

        for count in [c_int::MIN, -99, -1, 0, 4, 99, c_int::MAX] {
            let mut sc = simplex_with_points(
                V { x: 1.0, y: 2.0 },
                V { x: 3.0, y: 4.0 },
                V { x: 5.0, y: 6.0 },
            );
            sc.count = count;
            sc.div = 0.0;
            let mut sr = sc;
            assert_f32_eq(
                c_metric(&mut sc),
                r_metric(&mut sr),
                ("invalid metric count", count),
            );
            assert_f32_eq(c_metric(&mut sc), 0.0, ("metric sentinel", count));
            assert_bits_eq(&c_d(&mut sc), &r_d(&mut sr), ("invalid D count", count));
            assert_bits_eq(&c_l(&mut sc), &r_l(&mut sr), ("invalid L count", count));
            let mut ca = V { x: 9.0, y: 8.0 };
            let mut cb = V { x: 7.0, y: 6.0 };
            let mut ra = ca;
            let mut rb = cb;
            c_witness(&mut sc, &mut ca, &mut cb);
            r_witness(&mut sr, &mut ra, &mut rb);
            assert_bits_eq(&(ca, cb), &(ra, rb), ("invalid witness count", count));
        }

        let readable = V { x: 4.0, y: -7.0 };
        for count in [c_int::MIN, -10, -1, 0] {
            assert_eq!(c_support(&readable, count, V { x: 1.0, y: 2.0 }), 0);
            assert_eq!(r_support(&readable, count, V { x: 1.0, y: 2.0 }), 0);
        }

        for type_a in [c_int::MIN, -1, 3, c_int::MAX] {
            assert_eq!(c_collided(null(), type_a, null(), c_int::MIN), 0);
            assert_eq!(r_collided(null(), type_a, null(), c_int::MIN), 0);
        }
        let circle = Circle {
            p: V { x: 0.0, y: 0.0 },
            r: 1.0,
        };
        let aabb = Aabb {
            min: V { x: -1.0, y: -1.0 },
            max: V { x: 1.0, y: 1.0 },
        };
        let capsule = Capsule {
            a: V { x: -1.0, y: 0.0 },
            b: V { x: 1.0, y: 0.0 },
            r: 1.0,
        };
        for (type_a, ptr_a) in [
            (CIRCLE, (&circle as *const Circle).cast::<c_void>()),
            (AABB, (&aabb as *const Aabb).cast::<c_void>()),
            (CAPSULE, (&capsule as *const Capsule).cast::<c_void>()),
        ] {
            for type_b in [c_int::MIN, -1, 3, c_int::MAX] {
                assert_eq!(c_collided(ptr_a, type_a, null(), type_b), 0);
                assert_eq!(r_collided(ptr_a, type_a, null(), type_b), 0);
            }
        }
    }
}
