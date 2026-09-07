use libloading::{Library, Symbol};
use std::ffi::{c_float, c_int, c_uint, c_void};
use std::mem::{MaybeUninit, size_of};
use std::path::PathBuf;

const CIRCLE: c_uint = 0;
const AABB: c_uint = 1;
const CAPSULE: c_uint = 2;

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
struct Bb {
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

struct Libs {
    c: Library,
    r: Library,
}

impl Libs {
    unsafe fn load() -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c = root.join("../c_src/build/libharvest-work-QmjZHd.so");
        let r = root.join("target/release/libaabb_lib.so");
        assert!(c.is_file(), "missing C shared object: {}", c.display());
        assert!(r.is_file(), "missing Rust shared object: {}", r.display());
        Self {
            c: unsafe { Library::new(c).unwrap() },
            r: unsafe { Library::new(r).unwrap() },
        }
    }

    unsafe fn pair<T>(&self, name: &[u8]) -> (Symbol<'_, T>, Symbol<'_, T>) {
        (unsafe { self.c.get(name).unwrap() }, unsafe {
            self.r.get(name).unwrap()
        })
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

    fn f(&mut self) -> f32 {
        let n = (self.u32() % 2_000_001) as i32 - 1_000_000;
        n as f32 / 4096.0
    }

    fn positive(&mut self) -> f32 {
        (self.u32() % 100_000) as f32 / 4096.0 + 0.01
    }

    fn v(&mut self) -> V {
        V {
            x: self.f(),
            y: self.f(),
        }
    }
}

fn bytes<T>(value: &T) -> &[u8] {
    unsafe { std::slice::from_raw_parts((value as *const T).cast(), size_of::<T>()) }
}

fn assert_bytes<T>(label: &str, c: &T, r: &T) {
    assert_eq!(bytes(c), bytes(r), "{label}");
}

fn assert_f32(label: &str, c: f32, r: f32) {
    assert_eq!(c.to_bits(), r.to_bits(), "{label}: C={c:?} Rust={r:?}");
}

fn sv(p: V) -> Sv {
    Sv {
        s_a: V {
            x: p.x + 0.25,
            y: p.y - 0.5,
        },
        s_b: V {
            x: p.x - 0.75,
            y: p.y + 1.0,
        },
        p,
        u: 0.0,
        i_a: 0,
        i_b: 0,
    }
}

fn simplex(a: V, b: V, c: V, count: i32) -> Simplex {
    Simplex {
        a: sv(a),
        b: sv(b),
        c: sv(c),
        d: sv(V::default()),
        div: 1.0,
        count,
    }
}

#[derive(Clone, Copy)]
enum Shape {
    Circle(Circle),
    Bb(Bb),
    Capsule(Capsule),
}

impl Shape {
    fn ty(&self) -> c_uint {
        match self {
            Shape::Circle(_) => CIRCLE,
            Shape::Bb(_) => AABB,
            Shape::Capsule(_) => CAPSULE,
        }
    }

    fn ptr(&self) -> *const c_void {
        match self {
            Shape::Circle(v) => (v as *const Circle).cast(),
            Shape::Bb(v) => (v as *const Bb).cast(),
            Shape::Capsule(v) => (v as *const Capsule).cast(),
        }
    }
}

fn random_shape(rng: &mut Rng, ty: c_uint, offset: f32) -> Shape {
    match ty {
        CIRCLE => Shape::Circle(Circle {
            p: V {
                x: rng.f() + offset,
                y: rng.f() - offset * 0.25,
            },
            r: rng.positive(),
        }),
        AABB => {
            let p = V {
                x: rng.f() + offset,
                y: rng.f() - offset * 0.25,
            };
            let w = rng.positive();
            let h = rng.positive();
            Shape::Bb(Bb {
                min: p,
                max: V {
                    x: p.x + w,
                    y: p.y + h,
                },
            })
        }
        CAPSULE => {
            let a = V {
                x: rng.f() + offset,
                y: rng.f() - offset * 0.25,
            };
            let mut b = V {
                x: a.x + rng.f() * 0.25,
                y: a.y + rng.f() * 0.25,
            };
            if b.x == a.x && b.y == a.y {
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

#[test]
fn every_c_symbol_loads_from_both_shared_objects() {
    unsafe {
        let libs = Libs::load();
        for name in [
            b"aabb\0".as_slice(),
            b"c22\0",
            b"c23\0",
            b"c2AABBtoAABB\0",
            b"c2AABBtoCapsule\0",
            b"c2Add\0",
            b"c2BBVerts\0",
            b"c2CCW90\0",
            b"c2CapsuletoCapsule\0",
            b"c2CircletoAABB\0",
            b"c2CircletoCapsule\0",
            b"c2CircletoCircle\0",
            b"c2Clampv\0",
            b"c2Collided\0",
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
        ] {
            let _: Symbol<'_, unsafe extern "C" fn()> = libs.c.get(name).unwrap();
            let _: Symbol<'_, unsafe extern "C" fn()> = libs.r.get(name).unwrap();
        }
    }
}

#[test]
fn vector_transform_proxy_and_scalar_surface() {
    unsafe {
        let libs = Libs::load();
        type VV = unsafe extern "C" fn(V, V) -> V;
        type VF = unsafe extern "C" fn(V, f32) -> V;
        type VVF = unsafe extern "C" fn(V, V) -> f32;
        type VVV = unsafe extern "C" fn(V, V, V) -> V;
        type RV = unsafe extern "C" fn(R, V) -> V;
        type XV = unsafe extern "C" fn(X, V) -> V;
        type OneV = unsafe extern "C" fn(V) -> V;
        type OneF = unsafe extern "C" fn(V) -> f32;

        let (c_v, r_v) = libs.pair::<unsafe extern "C" fn(f32, f32) -> V>(b"c2V\0");
        let (c_mulvs, r_mulvs) = libs.pair::<VF>(b"c2Mulvs\0");
        let (c_max, r_max) = libs.pair::<VV>(b"c2Maxv\0");
        let (c_min, r_min) = libs.pair::<VV>(b"c2Minv\0");
        let (c_clamp, r_clamp) = libs.pair::<VVV>(b"c2Clampv\0");
        let (c_sub, r_sub) = libs.pair::<VV>(b"c2Sub\0");
        let (c_dot, r_dot) = libs.pair::<VVF>(b"c2Dot\0");
        let (c_len, r_len) = libs.pair::<OneF>(b"c2Len\0");
        let (c_det, r_det) = libs.pair::<VVF>(b"c2Det2\0");
        let (c_mulrv, r_mulrv) = libs.pair::<RV>(b"c2Mulrv\0");
        let (c_add, r_add) = libs.pair::<VV>(b"c2Add\0");
        let (c_mulxv, r_mulxv) = libs.pair::<XV>(b"c2Mulxv\0");
        let (c_neg, r_neg) = libs.pair::<OneV>(b"c2Neg\0");
        let (c_skew, r_skew) = libs.pair::<OneV>(b"c2Skew\0");
        let (c_ccw, r_ccw) = libs.pair::<OneV>(b"c2CCW90\0");
        let (c_div, r_div) = libs.pair::<VF>(b"c2Div\0");
        let (c_norm, r_norm) = libs.pair::<OneV>(b"c2Norm\0");
        let (c_mulrvt, r_mulrvt) = libs.pair::<RV>(b"c2MulrvT\0");

        let mut rng = Rng::new(0x71a8_5eed_1234_5678);
        for i in 0..4000 {
            let a = rng.v();
            let b = rng.v();
            let scalar = if i % 97 == 0 { 0.0 } else { rng.f() };
            let lo = V {
                x: a.x.min(b.x),
                y: a.y.min(b.y),
            };
            let hi = V {
                x: a.x.max(b.x),
                y: a.y.max(b.y),
            };
            let q = match i % 9 {
                0 => V {
                    x: lo.x - 1.0,
                    y: lo.y - 1.0,
                },
                1 => V {
                    x: lo.x - 1.0,
                    y: (lo.y + hi.y) * 0.5,
                },
                2 => V {
                    x: lo.x - 1.0,
                    y: hi.y + 1.0,
                },
                3 => V {
                    x: (lo.x + hi.x) * 0.5,
                    y: lo.y - 1.0,
                },
                4 => V {
                    x: (lo.x + hi.x) * 0.5,
                    y: (lo.y + hi.y) * 0.5,
                },
                5 => V {
                    x: (lo.x + hi.x) * 0.5,
                    y: hi.y + 1.0,
                },
                6 => V {
                    x: hi.x + 1.0,
                    y: lo.y - 1.0,
                },
                7 => V {
                    x: hi.x + 1.0,
                    y: (lo.y + hi.y) * 0.5,
                },
                _ => V {
                    x: hi.x + 1.0,
                    y: hi.y + 1.0,
                },
            };
            let rot = R {
                c: rng.f(),
                s: rng.f(),
            };
            let xf = X { p: rng.v(), r: rot };

            assert_bytes("c2V", &c_v(a.x, a.y), &r_v(a.x, a.y));
            assert_bytes("c2Mulvs", &c_mulvs(a, scalar), &r_mulvs(a, scalar));
            assert_bytes("c2Maxv", &c_max(a, b), &r_max(a, b));
            assert_bytes("c2Minv", &c_min(a, b), &r_min(a, b));
            assert_bytes("c2Clampv", &c_clamp(q, lo, hi), &r_clamp(q, lo, hi));
            assert_bytes("c2Sub", &c_sub(a, b), &r_sub(a, b));
            assert_f32("c2Dot", c_dot(a, b), r_dot(a, b));
            assert_f32("c2Len", c_len(a), r_len(a));
            assert_f32("c2Det2", c_det(a, b), r_det(a, b));
            assert_bytes("c2Mulrv", &c_mulrv(rot, a), &r_mulrv(rot, a));
            assert_bytes("c2Add", &c_add(a, b), &r_add(a, b));
            assert_bytes("c2Mulxv", &c_mulxv(xf, a), &r_mulxv(xf, a));
            assert_bytes("c2Neg", &c_neg(a), &r_neg(a));
            assert_bytes("c2Skew", &c_skew(a), &r_skew(a));
            assert_bytes("c2CCW90", &c_ccw(a), &r_ccw(a));
            assert_bytes("c2Div", &c_div(a, scalar), &r_div(a, scalar));
            assert_bytes("c2Norm", &c_norm(a), &r_norm(a));
            assert_bytes("c2MulrvT", &c_mulrvt(rot, a), &r_mulrvt(rot, a));
        }

        for a in [
            V::default(),
            V { x: 1.0, y: -1.0 },
            V {
                x: f32::INFINITY,
                y: 2.0,
            },
        ] {
            assert_f32("c2Len special", c_len(a), r_len(a));
            assert_bytes("c2Norm special", &c_norm(a), &r_norm(a));
            assert_bytes("c2Div zero", &c_div(a, 0.0), &r_div(a, 0.0));
        }

        for (a, b) in [
            (V { x: 4.0, y: 4.0 }, V { x: 1.0, y: 1.0 }),
            (V { x: 4.0, y: 1.0 }, V { x: 1.0, y: 4.0 }),
            (V { x: 1.0, y: 4.0 }, V { x: 4.0, y: 1.0 }),
            (V { x: 1.0, y: 1.0 }, V { x: 4.0, y: 4.0 }),
            (V { x: 4.0, y: 4.0 }, V { x: 4.0, y: 4.0 }),
        ] {
            assert_bytes("c2Maxv selection", &c_max(a, b), &r_max(a, b));
            assert_bytes("c2Minv selection", &c_min(a, b), &r_min(a, b));
        }

        let (c_ri, r_ri) = libs.pair::<unsafe extern "C" fn() -> R>(b"c2RotIdentity\0");
        let (c_xi, r_xi) = libs.pair::<unsafe extern "C" fn() -> X>(b"c2xIdentity\0");
        assert_bytes("c2RotIdentity", &c_ri(), &r_ri());
        assert_bytes("c2xIdentity", &c_xi(), &r_xi());

        let (c_bbverts, r_bbverts) =
            libs.pair::<unsafe extern "C" fn(*mut V, *mut Bb)>(b"c2BBVerts\0");
        let (c_proxy, r_proxy) =
            libs.pair::<unsafe extern "C" fn(*const c_void, c_uint, *mut Proxy)>(b"c2MakeProxy\0");
        for _ in 0..1000 {
            let mut bb_c = Bb {
                min: rng.v(),
                max: rng.v(),
            };
            let mut bb_r = bb_c;
            let mut out_c = [V::default(); 4];
            let mut out_r = [V::default(); 4];
            c_bbverts(out_c.as_mut_ptr(), &mut bb_c);
            r_bbverts(out_r.as_mut_ptr(), &mut bb_r);
            assert_bytes("c2BBVerts", &out_c, &out_r);

            let shapes = [
                Shape::Circle(Circle {
                    p: rng.v(),
                    r: rng.f(),
                }),
                Shape::Bb(bb_c),
                Shape::Capsule(Capsule {
                    a: rng.v(),
                    b: rng.v(),
                    r: rng.f(),
                }),
            ];
            for shape in shapes {
                let mut pc: Proxy = std::mem::transmute([0xa5u8; size_of::<Proxy>()]);
                let mut pr = pc;
                c_proxy(shape.ptr(), shape.ty(), &mut pc);
                r_proxy(shape.ptr(), shape.ty(), &mut pr);
                assert_bytes("c2MakeProxy", &pc, &pr);
            }
        }

        let mut equal_c = Bb {
            min: V { x: 2.0, y: -3.0 },
            max: V { x: 2.0, y: -3.0 },
        };
        let mut equal_r = equal_c;
        let mut out_c = [V::default(); 4];
        let mut out_r = [V::default(); 4];
        c_bbverts(out_c.as_mut_ptr(), &mut equal_c);
        r_bbverts(out_r.as_mut_ptr(), &mut equal_r);
        assert_bytes("c2BBVerts equal bounds", &out_c, &out_r);
    }
}

#[test]
fn simplex_surface_and_all_branch_regions() {
    unsafe {
        let libs = Libs::load();
        let (c_metric, r_metric) =
            libs.pair::<unsafe extern "C" fn(*mut Simplex) -> f32>(b"c2GJKSimplexMetric\0");
        let (c_22, r_22) = libs.pair::<unsafe extern "C" fn(*mut Simplex)>(b"c22\0");
        let (c_23, r_23) = libs.pair::<unsafe extern "C" fn(*mut Simplex)>(b"c23\0");
        let (c_d, r_d) = libs.pair::<unsafe extern "C" fn(*mut Simplex) -> V>(b"c2D\0");
        let (c_witness, r_witness) =
            libs.pair::<unsafe extern "C" fn(*mut Simplex, *mut V, *mut V)>(b"c2Witness\0");
        let (c_l, r_l) = libs.pair::<unsafe extern "C" fn(*mut Simplex) -> V>(b"c2L\0");

        let mut rng = Rng::new(0xc223_c223_55aa_9911);
        let mut c22_regions = [false; 3];
        let mut c23_regions = [false; 7];
        let mut d_regions = [false; 3];

        for _ in 0..100_000 {
            let original = simplex(rng.v(), rng.v(), rng.v(), 3);

            for count in 1..=3 {
                let mut sc = original;
                sc.count = count;
                let mut sr = sc;
                assert_f32("c2GJKSimplexMetric", c_metric(&mut sc), r_metric(&mut sr));
            }

            let mut sc = original;
            sc.count = 2;
            let before = sc;
            let mut sr = sc;
            c_22(&mut sc);
            r_22(&mut sr);
            assert_bytes("c22", &sc, &sr);
            let region = if sc.count == 2 {
                2
            } else if bytes(&sc.a.p) == bytes(&before.a.p) {
                0
            } else {
                1
            };
            c22_regions[region] = true;

            let mut sc = original;
            let before = sc;
            let mut sr = sc;
            c_23(&mut sc);
            r_23(&mut sr);
            assert_bytes("c23", &sc, &sr);
            let region = match sc.count {
                1 if bytes(&sc.a.p) == bytes(&before.a.p) => 0,
                1 if bytes(&sc.a.p) == bytes(&before.b.p) => 1,
                1 => 2,
                2 if bytes(&sc.a.p) == bytes(&before.a.p)
                    && bytes(&sc.b.p) == bytes(&before.b.p) =>
                {
                    3
                }
                2 if bytes(&sc.a.p) == bytes(&before.b.p)
                    && bytes(&sc.b.p) == bytes(&before.c.p) =>
                {
                    4
                }
                2 => 5,
                3 => 6,
                _ => unreachable!(),
            };
            c23_regions[region] = true;

            for count in 1..=3 {
                let mut sc = original;
                sc.count = count;
                let mut sr = sc;
                assert_bytes("c2D", &c_d(&mut sc), &r_d(&mut sr));
                if count == 1 {
                    d_regions[0] = true;
                } else if count == 2 {
                    let ab = V {
                        x: sc.b.p.x - sc.a.p.x,
                        y: sc.b.p.y - sc.a.p.y,
                    };
                    let neg_a = V {
                        x: -sc.a.p.x,
                        y: -sc.a.p.y,
                    };
                    let det = ab.x * neg_a.y - ab.y * neg_a.x;
                    d_regions[if det > 0.0 { 1 } else { 2 }] = true;
                }
            }

            for count in 1..=3 {
                let mut sc = original;
                sc.count = count;
                sc.div = rng.positive();
                sc.a.u = rng.f();
                sc.b.u = rng.f();
                sc.c.u = rng.f();
                let mut sr = sc;
                let (mut ac, mut bc, mut ar, mut br) =
                    (V::default(), V::default(), V::default(), V::default());
                c_witness(&mut sc, &mut ac, &mut bc);
                r_witness(&mut sr, &mut ar, &mut br);
                assert_bytes("c2Witness A", &ac, &ar);
                assert_bytes("c2Witness B", &bc, &br);
                assert_bytes("c2L", &c_l(&mut sc), &r_l(&mut sr));
            }

            if c22_regions.iter().all(|x| *x)
                && c23_regions.iter().all(|x| *x)
                && d_regions.iter().all(|x| *x)
            {
                break;
            }
        }
        assert!(
            c22_regions.iter().all(|x| *x),
            "missing c22 region: {c22_regions:?}"
        );
        assert!(
            c23_regions.iter().all(|x| *x),
            "missing c23 region: {c23_regions:?}"
        );
        assert!(
            d_regions.iter().all(|x| *x),
            "missing c2D region: {d_regions:?}"
        );
    }
}

#[test]
fn support_count_and_tie_surface() {
    unsafe {
        let libs = Libs::load();
        let (c, r) = libs.pair::<unsafe extern "C" fn(*const V, c_int, V) -> c_int>(b"c2Support\0");
        let cases: Vec<(Vec<V>, V)> = vec![
            (vec![V { x: 3.0, y: 4.0 }], V { x: 1.0, y: 1.0 }),
            (
                vec![
                    V { x: 9.0, y: 0.0 },
                    V { x: 1.0, y: 0.0 },
                    V { x: 2.0, y: 0.0 },
                ],
                V { x: 1.0, y: 0.0 },
            ),
            (
                vec![
                    V { x: 1.0, y: 0.0 },
                    V { x: 8.0, y: 0.0 },
                    V { x: 2.0, y: 0.0 },
                ],
                V { x: 1.0, y: 0.0 },
            ),
            (
                vec![
                    V { x: 8.0, y: 0.0 },
                    V { x: 8.0, y: 0.0 },
                    V { x: 2.0, y: 0.0 },
                ],
                V { x: 1.0, y: 0.0 },
            ),
            (
                (0..32)
                    .map(|i| V {
                        x: i as f32,
                        y: -(i as f32),
                    })
                    .collect(),
                V { x: 1.0, y: 0.0 },
            ),
        ];
        for (verts, d) in cases {
            assert_eq!(
                c(verts.as_ptr(), verts.len() as i32, d),
                r(verts.as_ptr(), verts.len() as i32, d)
            );
        }

        let mut rng = Rng::new(0x5e77_5e77);
        for _ in 0..5000 {
            let count = (rng.u32() % 31 + 1) as usize;
            let verts: Vec<V> = (0..count).map(|_| rng.v()).collect();
            let d = rng.v();
            assert_eq!(
                c(verts.as_ptr(), count as i32, d),
                r(verts.as_ptr(), count as i32, d)
            );
        }
    }
}

type Gjk = unsafe extern "C" fn(
    *const c_void,
    c_uint,
    *const X,
    *const c_void,
    c_uint,
    *const X,
    *mut V,
    *mut V,
    c_int,
    *mut c_int,
    *mut Cache,
) -> f32;

unsafe fn compare_gjk(
    c: &Symbol<'_, Gjk>,
    r: &Symbol<'_, Gjk>,
    a: &Shape,
    b: &Shape,
    ax: Option<&X>,
    bx: Option<&X>,
    use_radius: i32,
    output_mask: u8,
    cache_c: Option<&mut Cache>,
    cache_r: Option<&mut Cache>,
) {
    let mut out_ac = V { x: 91.0, y: 92.0 };
    let mut out_bc = V { x: 93.0, y: 94.0 };
    let mut out_ar = out_ac;
    let mut out_br = out_bc;
    let mut iter_c = -99;
    let mut iter_r = -99;
    let out_ac_ptr = if output_mask & 1 != 0 {
        &mut out_ac
    } else {
        std::ptr::null_mut()
    };
    let out_ar_ptr = if output_mask & 1 != 0 {
        &mut out_ar
    } else {
        std::ptr::null_mut()
    };
    let out_bc_ptr = if output_mask & 2 != 0 {
        &mut out_bc
    } else {
        std::ptr::null_mut()
    };
    let out_br_ptr = if output_mask & 2 != 0 {
        &mut out_br
    } else {
        std::ptr::null_mut()
    };
    let iter_c_ptr = if output_mask & 4 != 0 {
        &mut iter_c
    } else {
        std::ptr::null_mut()
    };
    let iter_r_ptr = if output_mask & 4 != 0 {
        &mut iter_r
    } else {
        std::ptr::null_mut()
    };
    let cache_c_ptr = cache_c.map_or(std::ptr::null_mut(), |v| v as *mut Cache);
    let cache_r_ptr = cache_r.map_or(std::ptr::null_mut(), |v| v as *mut Cache);
    let ax_ptr = ax.map_or(std::ptr::null(), |v| v as *const X);
    let bx_ptr = bx.map_or(std::ptr::null(), |v| v as *const X);

    let dc = unsafe {
        c(
            a.ptr(),
            a.ty(),
            ax_ptr,
            b.ptr(),
            b.ty(),
            bx_ptr,
            out_ac_ptr,
            out_bc_ptr,
            use_radius,
            iter_c_ptr,
            cache_c_ptr,
        )
    };
    let dr = unsafe {
        r(
            a.ptr(),
            a.ty(),
            ax_ptr,
            b.ptr(),
            b.ty(),
            bx_ptr,
            out_ar_ptr,
            out_br_ptr,
            use_radius,
            iter_r_ptr,
            cache_r_ptr,
        )
    };
    assert_f32("c2GJK distance", dc, dr);
    assert_bytes("c2GJK outA", &out_ac, &out_ar);
    assert_bytes("c2GJK outB", &out_bc, &out_br);
    assert_eq!(iter_c, iter_r, "c2GJK iterations");
}

#[test]
fn gjk_all_shape_option_output_and_cache_configurations() {
    unsafe {
        let libs = Libs::load();
        let (c, r) = libs.pair::<Gjk>(b"c2GJK\0");
        let mut rng = Rng::new(0x6a6b_1234_f00d_beef);
        let mut saw_three_point_hit = false;

        for ta in 0..=2 {
            for tb in 0..=2 {
                for sample in 0..300 {
                    let offset = if sample % 3 == 0 { 1000.0 } else { 0.0 };
                    let a = random_shape(&mut rng, ta, 0.0);
                    let b = random_shape(&mut rng, tb, offset);
                    let ax = X {
                        p: rng.v(),
                        r: R {
                            c: rng.f(),
                            s: rng.f(),
                        },
                    };
                    let bx = X {
                        p: rng.v(),
                        r: R {
                            c: rng.f(),
                            s: rng.f(),
                        },
                    };
                    let transform_mode = sample % 4;
                    let ax_opt = if transform_mode & 1 != 0 {
                        Some(&ax)
                    } else {
                        None
                    };
                    let bx_opt = if transform_mode & 2 != 0 {
                        Some(&bx)
                    } else {
                        None
                    };
                    compare_gjk(
                        &c,
                        &r,
                        &a,
                        &b,
                        ax_opt,
                        bx_opt,
                        (sample & 1) as i32,
                        (sample % 8) as u8,
                        None,
                        None,
                    );

                    let mut cc = Cache::default();
                    let mut cr = Cache::default();
                    compare_gjk(
                        &c,
                        &r,
                        &a,
                        &b,
                        ax_opt,
                        bx_opt,
                        (sample & 1) as i32,
                        7,
                        Some(&mut cc),
                        Some(&mut cr),
                    );
                    assert_bytes("cold cache", &cc, &cr);
                    saw_three_point_hit |= cc.count == 3;
                    compare_gjk(
                        &c,
                        &r,
                        &a,
                        &b,
                        ax_opt,
                        bx_opt,
                        (sample & 1) as i32,
                        7,
                        Some(&mut cc),
                        Some(&mut cr),
                    );
                    assert_bytes("warm cache", &cc, &cr);
                }
            }
        }

        let a = Shape::Bb(Bb {
            min: V { x: -2.0, y: -3.0 },
            max: V { x: 4.0, y: 5.0 },
        });
        let b = Shape::Bb(Bb {
            min: V { x: -1.0, y: -1.0 },
            max: V { x: 6.0, y: 7.0 },
        });
        for count in 1..=3 {
            for _ in 0..100 {
                let mut cc = Cache {
                    metric: rng.f(),
                    count,
                    i_a: [0, 1, 2],
                    i_b: [2, 1, 0],
                    div: rng.positive(),
                };
                let mut cr = cc;
                compare_gjk(
                    &c,
                    &r,
                    &a,
                    &b,
                    None,
                    None,
                    1,
                    7,
                    Some(&mut cc),
                    Some(&mut cr),
                );
                assert_bytes("manual cache count", &cc, &cr);
            }
        }

        let separated_a = Shape::Circle(Circle {
            p: V { x: 0.0, y: 0.0 },
            r: 1.0,
        });
        let separated_b = Shape::Circle(Circle {
            p: V { x: 10.0, y: 0.0 },
            r: 2.0,
        });
        let overlap_b = Shape::Circle(Circle {
            p: V { x: 1.0, y: 0.0 },
            r: 2.0,
        });
        for use_radius in [0, 1] {
            compare_gjk(
                &c,
                &r,
                &separated_a,
                &separated_b,
                None,
                None,
                use_radius,
                7,
                None,
                None,
            );
            compare_gjk(
                &c,
                &r,
                &separated_a,
                &overlap_b,
                None,
                None,
                use_radius,
                7,
                None,
                None,
            );
        }

        let hit_a = Shape::Bb(Bb {
            min: V { x: -2.0, y: -2.0 },
            max: V { x: 2.0, y: 2.0 },
        });
        let hit_b = Shape::Bb(Bb {
            min: V { x: -1.0, y: -1.0 },
            max: V { x: 3.0, y: 3.0 },
        });
        let mut hit_cc = Cache::default();
        let mut hit_cr = Cache::default();
        compare_gjk(
            &c,
            &r,
            &hit_a,
            &hit_b,
            None,
            None,
            0,
            7,
            Some(&mut hit_cc),
            Some(&mut hit_cr),
        );
        assert_eq!(hit_cc.count, hit_cr.count);

        let point_a = Shape::Circle(Circle {
            p: V { x: 4.0, y: -7.0 },
            r: 0.0,
        });
        let point_b = point_a;
        let mut tiny_cc = Cache::default();
        let mut tiny_cr = Cache::default();
        compare_gjk(
            &c,
            &r,
            &point_a,
            &point_b,
            None,
            None,
            0,
            7,
            Some(&mut tiny_cc),
            Some(&mut tiny_cr),
        );
        assert_eq!(tiny_cc.count, 1, "expected C tiny-direction simplex");
        assert_eq!(tiny_cr.count, 1, "expected Rust tiny-direction simplex");
        assert!(
            saw_three_point_hit,
            "randomized GJK corpus missed count-3 hit branch"
        );
    }
}

#[test]
fn collision_wrappers_dispatch_and_driver_surface() {
    unsafe {
        let libs = Libs::load();
        let (c_aa, r_aa) = libs.pair::<unsafe extern "C" fn(Bb, Bb) -> i32>(b"c2AABBtoAABB\0");
        let (c_ac, r_ac) =
            libs.pair::<unsafe extern "C" fn(Bb, Capsule) -> i32>(b"c2AABBtoCapsule\0");
        let (c_ccap, r_ccap) =
            libs.pair::<unsafe extern "C" fn(Capsule, Capsule) -> i32>(b"c2CapsuletoCapsule\0");
        let (c_cc, r_cc) =
            libs.pair::<unsafe extern "C" fn(Circle, Circle) -> i32>(b"c2CircletoCircle\0");
        let (c_ca, r_ca) =
            libs.pair::<unsafe extern "C" fn(Circle, Bb) -> i32>(b"c2CircletoAABB\0");
        let (c_ccaps, r_ccaps) =
            libs.pair::<unsafe extern "C" fn(Circle, Capsule) -> i32>(b"c2CircletoCapsule\0");
        let (c_collided, r_collided) =
            libs.pair::<unsafe extern "C" fn(*const c_void, c_uint, *const c_void, c_uint) -> i32>(
                b"c2Collided\0",
            );
        let (c_aabb, r_aabb) =
            libs.pair::<unsafe extern "C" fn(f32, f32, f32, f32) -> i32>(b"aabb\0");

        let mut rng = Rng::new(0xc011_1de5_abcdef01);
        for sample in 0..10_000 {
            let aa = match random_shape(&mut rng, AABB, 0.0) {
                Shape::Bb(x) => x,
                _ => unreachable!(),
            };
            let ab = match random_shape(&mut rng, AABB, if sample % 2 == 0 { 500.0 } else { 0.0 }) {
                Shape::Bb(x) => x,
                _ => unreachable!(),
            };
            let ca = match random_shape(&mut rng, CIRCLE, 0.0) {
                Shape::Circle(x) => x,
                _ => unreachable!(),
            };
            let cb = match random_shape(&mut rng, CIRCLE, if sample % 2 == 0 { 500.0 } else { 0.0 })
            {
                Shape::Circle(x) => x,
                _ => unreachable!(),
            };
            let pa = match random_shape(&mut rng, CAPSULE, 0.0) {
                Shape::Capsule(x) => x,
                _ => unreachable!(),
            };
            let pb =
                match random_shape(&mut rng, CAPSULE, if sample % 2 == 0 { 500.0 } else { 0.0 }) {
                    Shape::Capsule(x) => x,
                    _ => unreachable!(),
                };

            assert_eq!(c_aa(aa, ab), r_aa(aa, ab), "c2AABBtoAABB");
            assert_eq!(c_ac(aa, pb), r_ac(aa, pb), "c2AABBtoCapsule");
            assert_eq!(c_ccap(pa, pb), r_ccap(pa, pb), "c2CapsuletoCapsule");
            assert_eq!(c_cc(ca, cb), r_cc(ca, cb), "c2CircletoCircle");
            assert_eq!(c_ca(ca, ab), r_ca(ca, ab), "c2CircletoAABB");
            assert_eq!(c_ccaps(ca, pb), r_ccaps(ca, pb), "c2CircletoCapsule");

            let shapes = [Shape::Circle(ca), Shape::Bb(aa), Shape::Capsule(pa)];
            let shapes_b = [Shape::Circle(cb), Shape::Bb(ab), Shape::Capsule(pb)];
            for a in &shapes {
                for b in &shapes_b {
                    assert_eq!(
                        c_collided(a.ptr(), a.ty(), b.ptr(), b.ty()),
                        r_collided(a.ptr(), a.ty(), b.ptr(), b.ty()),
                        "c2Collided pair {} {}",
                        a.ty(),
                        b.ty()
                    );
                }
            }
            assert_eq!(
                c_aabb(aa.min.x, aa.min.y, aa.max.x, aa.max.y),
                r_aabb(aa.min.x, aa.min.y, aa.max.x, aa.max.y),
                "aabb"
            );
            assert_eq!(
                c_aabb(aa.max.x, aa.max.y, aa.min.x, aa.min.y),
                r_aabb(aa.max.x, aa.max.y, aa.min.x, aa.min.y),
                "aabb reversed"
            );
        }

        let base_circle = Circle {
            p: V { x: 0.0, y: 0.0 },
            r: 1.0,
        };
        for other in [
            Circle {
                p: V { x: 3.0, y: 0.0 },
                r: 1.0,
            },
            Circle {
                p: V { x: 2.0, y: 0.0 },
                r: 1.0,
            },
            Circle {
                p: V { x: 1.0, y: 0.0 },
                r: 1.0,
            },
        ] {
            assert_eq!(c_cc(base_circle, other), r_cc(base_circle, other));
        }
        let bb = Bb {
            min: V { x: -1.0, y: -1.0 },
            max: V { x: 1.0, y: 1.0 },
        };
        for other in [
            Bb {
                min: V { x: -4.0, y: -1.0 },
                max: V { x: -3.0, y: 1.0 },
            },
            Bb {
                min: V { x: 3.0, y: -1.0 },
                max: V { x: 4.0, y: 1.0 },
            },
            Bb {
                min: V { x: -1.0, y: -4.0 },
                max: V { x: 1.0, y: -3.0 },
            },
            Bb {
                min: V { x: -1.0, y: 3.0 },
                max: V { x: 1.0, y: 4.0 },
            },
            Bb {
                min: V { x: 1.0, y: -1.0 },
                max: V { x: 2.0, y: 1.0 },
            },
            Bb {
                min: V { x: -0.5, y: -0.5 },
                max: V { x: 0.5, y: 0.5 },
            },
        ] {
            assert_eq!(c_aa(bb, other), r_aa(bb, other));
        }
        for circle in [
            Circle {
                p: V { x: 3.0, y: 0.0 },
                r: 1.0,
            },
            Circle {
                p: V { x: 2.0, y: 0.0 },
                r: 1.0,
            },
            Circle {
                p: V { x: 1.5, y: 0.0 },
                r: 1.0,
            },
            Circle {
                p: V { x: 0.0, y: 0.0 },
                r: 1.0,
            },
        ] {
            assert_eq!(c_ca(circle, bb), r_ca(circle, bb));
        }
        let cap = Capsule {
            a: V { x: 0.0, y: 0.0 },
            b: V { x: 10.0, y: 0.0 },
            r: 1.0,
        };
        for circle in [
            Circle {
                p: V { x: -3.0, y: 0.0 },
                r: 1.0,
            },
            Circle {
                p: V { x: 5.0, y: 3.0 },
                r: 1.0,
            },
            Circle {
                p: V { x: 13.0, y: 0.0 },
                r: 1.0,
            },
        ] {
            assert_eq!(c_ccaps(circle, cap), r_ccaps(circle, cap));
        }

        for other in [
            Capsule {
                a: V { x: 20.0, y: 20.0 },
                b: V { x: 30.0, y: 20.0 },
                r: 1.0,
            },
            Capsule {
                a: V { x: -2.0, y: 0.0 },
                b: V { x: 2.0, y: 0.0 },
                r: 1.0,
            },
        ] {
            assert_eq!(c_ac(bb, other), r_ac(bb, other));
        }
        for other in [
            Capsule {
                a: V { x: 20.0, y: 20.0 },
                b: V { x: 30.0, y: 20.0 },
                r: 1.0,
            },
            Capsule {
                a: V { x: 5.0, y: -2.0 },
                b: V { x: 5.0, y: 2.0 },
                r: 1.0,
            },
        ] {
            assert_eq!(c_ccap(cap, other), r_ccap(cap, other));
        }
    }
}

#[test]
fn rejection_default_and_boundary_surface() {
    unsafe {
        let libs = Libs::load();
        let (c_collided, r_collided) =
            libs.pair::<unsafe extern "C" fn(*const c_void, c_uint, *const c_void, c_uint) -> i32>(
                b"c2Collided\0",
            );
        let invalids = [3u32, u32::MAX, 0x7fff_ffff];
        let circle = Circle::default();
        let bb = Bb::default();
        let cap = Capsule::default();
        for invalid in invalids {
            assert_eq!(
                c_collided(std::ptr::null(), invalid, std::ptr::null(), invalid),
                r_collided(std::ptr::null(), invalid, std::ptr::null(), invalid)
            );
            for (ptr, ty) in [
                ((&circle as *const Circle).cast::<c_void>(), CIRCLE),
                ((&bb as *const Bb).cast::<c_void>(), AABB),
                ((&cap as *const Capsule).cast::<c_void>(), CAPSULE),
            ] {
                assert_eq!(
                    c_collided(ptr, ty, std::ptr::null(), invalid),
                    r_collided(ptr, ty, std::ptr::null(), invalid)
                );
            }
        }

        let (c_proxy, r_proxy) =
            libs.pair::<unsafe extern "C" fn(*const c_void, c_uint, *mut Proxy)>(b"c2MakeProxy\0");
        for invalid in invalids {
            let mut pc: Proxy = std::mem::transmute([0x5au8; size_of::<Proxy>()]);
            let mut pr = pc;
            c_proxy(std::ptr::null(), invalid, &mut pc);
            r_proxy(std::ptr::null(), invalid, &mut pr);
            assert_bytes("invalid c2MakeProxy", &pc, &pr);
        }

        let (c_metric, r_metric) =
            libs.pair::<unsafe extern "C" fn(*mut Simplex) -> f32>(b"c2GJKSimplexMetric\0");
        let (c_d, r_d) = libs.pair::<unsafe extern "C" fn(*mut Simplex) -> V>(b"c2D\0");
        let (c_witness, r_witness) =
            libs.pair::<unsafe extern "C" fn(*mut Simplex, *mut V, *mut V)>(b"c2Witness\0");
        let (c_l, r_l) = libs.pair::<unsafe extern "C" fn(*mut Simplex) -> V>(b"c2L\0");
        for count in [i32::MIN, -1, 0, 4, i32::MAX] {
            let mut sc = simplex(
                V { x: 1.0, y: 2.0 },
                V { x: 3.0, y: 4.0 },
                V { x: 5.0, y: 6.0 },
                count,
            );
            let mut sr = sc;
            assert_f32("invalid metric count", c_metric(&mut sc), r_metric(&mut sr));
            assert_bytes("invalid D count", &c_d(&mut sc), &r_d(&mut sr));
            assert_bytes("invalid L count", &c_l(&mut sc), &r_l(&mut sr));
            let (mut ac, mut bc, mut ar, mut br) = (
                V { x: 9.0, y: 8.0 },
                V { x: 7.0, y: 6.0 },
                V { x: 9.0, y: 8.0 },
                V { x: 7.0, y: 6.0 },
            );
            c_witness(&mut sc, &mut ac, &mut bc);
            r_witness(&mut sr, &mut ar, &mut br);
            assert_bytes("invalid witness A", &ac, &ar);
            assert_bytes("invalid witness B", &bc, &br);
        }

        let (c_support, r_support) =
            libs.pair::<unsafe extern "C" fn(*const V, c_int, V) -> c_int>(b"c2Support\0");
        let readable = V { x: 1.0, y: 2.0 };
        assert_eq!(
            c_support(&readable, 0, V { x: 3.0, y: 4.0 }),
            r_support(&readable, 0, V { x: 3.0, y: 4.0 })
        );
    }
}

#[test]
fn ffi_layouts_are_exactly_the_c_layout_sizes() {
    assert_eq!(size_of::<V>(), 8);
    assert_eq!(size_of::<R>(), 8);
    assert_eq!(size_of::<X>(), 16);
    assert_eq!(size_of::<Circle>(), 12);
    assert_eq!(size_of::<Bb>(), 16);
    assert_eq!(size_of::<Capsule>(), 20);
    assert_eq!(size_of::<Cache>(), 36);
    assert_eq!(size_of::<Proxy>(), 72);
    assert_eq!(size_of::<Sv>(), 36);
    assert_eq!(size_of::<Simplex>(), 152);
    let _ = MaybeUninit::<Simplex>::uninit();
}
