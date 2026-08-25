#![allow(non_snake_case)]

use libloading::Library;
use std::ffi::{c_float, c_int, c_void};
use std::mem::{MaybeUninit, size_of};
use std::path::PathBuf;
use std::process::Command;
use std::sync::Once;

const CIRCLE: c_int = 0;
const AABB_TYPE: c_int = 1;
const CAPSULE: c_int = 2;

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

struct Libs {
    c: Library,
    rust: Library,
}

impl Libs {
    fn load() -> Self {
        static LOAD_LIBM: Once = Once::new();
        static BUILD_RUST_SO: Once = Once::new();
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_path = root.join("c_src/build/libtranslated_rust.so");
        let rust_path = root.join("target/debug/libreverse_collide_lib.so");
        LOAD_LIBM.call_once(|| unsafe {
            use libloading::os::unix::{Library as UnixLibrary, RTLD_GLOBAL, RTLD_NOW};
            let libm = UnixLibrary::open(Some("libm.so.6"), RTLD_NOW | RTLD_GLOBAL)
                .expect("failed to load libm for the C shared object");
            std::mem::forget(libm);
        });
        BUILD_RUST_SO.call_once(|| {
            let status = Command::new(env!("CARGO"))
                .args(["build", "--no-default-features"])
                .current_dir(&root)
                .status()
                .expect("failed to invoke cargo build for the Rust cdylib");
            assert!(status.success(), "cargo build for the Rust cdylib failed");
        });
        assert!(c_path.is_file(), "missing C library: {}", c_path.display());
        assert!(
            rust_path.is_file(),
            "missing Rust library: {}",
            rust_path.display()
        );
        unsafe {
            Self {
                c: Library::new(c_path).unwrap(),
                rust: Library::new(rust_path).unwrap(),
            }
        }
    }

    fn pair<T: Copy>(&self, name: &[u8]) -> (T, T) {
        unsafe {
            (
                *self.c.get::<T>(name).unwrap(),
                *self.rust.get::<T>(name).unwrap(),
            )
        }
    }
}

#[derive(Clone, Copy)]
struct Rng(u64);

impl Rng {
    fn new() -> Self {
        Self(0x4d59_5df4_d0f3_3173)
    }

    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0 = self.0.wrapping_mul(0x2545_f491_4f6c_dd1d);
        self.0
    }

    fn f(&mut self) -> f32 {
        ((self.next() % 20_001) as i32 - 10_000) as f32 / 16.0
    }

    fn positive(&mut self) -> f32 {
        ((self.next() % 1000) + 1) as f32 / 16.0
    }

    fn v(&mut self) -> V {
        V {
            x: self.f(),
            y: self.f(),
        }
    }
}

fn assert_f32(c: f32, rust: f32) {
    assert_eq!(
        c.to_bits(),
        rust.to_bits(),
        "C={c:?} ({:08x}), Rust={rust:?} ({:08x})",
        c.to_bits(),
        rust.to_bits()
    );
}

fn assert_v(c: V, rust: V) {
    assert_f32(c.x, rust.x);
    assert_f32(c.y, rust.y);
}

fn bytes<T>(value: &T) -> &[u8] {
    unsafe { std::slice::from_raw_parts((value as *const T).cast(), size_of::<T>()) }
}

fn assert_bytes<T>(c: &T, rust: &T) {
    assert_eq!(bytes(c), bytes(rust));
}

fn poisoned<T>() -> T {
    let mut value = MaybeUninit::<T>::uninit();
    unsafe {
        std::ptr::write_bytes(value.as_mut_ptr().cast::<u8>(), 0xa5, size_of::<T>());
        value.assume_init()
    }
}

#[test]
fn vector_transform_and_proxy_surface_rows_1_through_36() {
    type V2 = unsafe extern "C" fn(f32, f32) -> V;
    type Vs = unsafe extern "C" fn(V, f32) -> V;
    type Vv = unsafe extern "C" fn(V, V) -> V;
    type Vvv = unsafe extern "C" fn(V, V, V) -> V;
    type Fvv = unsafe extern "C" fn(V, V) -> f32;
    type Fv = unsafe extern "C" fn(V) -> f32;
    type R0 = unsafe extern "C" fn() -> R;
    type X0 = unsafe extern "C" fn() -> X;
    type BbVerts = unsafe extern "C" fn(*mut V, *mut Aabb);
    type MakeProxy = unsafe extern "C" fn(*const c_void, c_int, *mut Proxy);
    type Metric = unsafe extern "C" fn(*mut Simplex) -> f32;
    type Rv = unsafe extern "C" fn(R, V) -> V;
    type Xv = unsafe extern "C" fn(X, V) -> V;

    let libs = Libs::load();
    let (c_v, r_v) = libs.pair::<V2>(b"c2V\0");
    let (c_mulvs, r_mulvs) = libs.pair::<Vs>(b"c2Mulvs\0");
    let (c_max, r_max) = libs.pair::<Vv>(b"c2Maxv\0");
    let (c_min, r_min) = libs.pair::<Vv>(b"c2Minv\0");
    let (c_clamp, r_clamp) = libs.pair::<Vvv>(b"c2Clampv\0");
    let (c_sub, r_sub) = libs.pair::<Vv>(b"c2Sub\0");
    let (c_dot, r_dot) = libs.pair::<Fvv>(b"c2Dot\0");
    let (c_rot_id, r_rot_id) = libs.pair::<R0>(b"c2RotIdentity\0");
    let (c_x_id, r_x_id) = libs.pair::<X0>(b"c2xIdentity\0");
    let (c_bb, r_bb) = libs.pair::<BbVerts>(b"c2BBVerts\0");
    let (c_proxy, r_proxy) = libs.pair::<MakeProxy>(b"c2MakeProxy\0");
    let (c_len, r_len) = libs.pair::<Fv>(b"c2Len\0");
    let (c_det, r_det) = libs.pair::<Fvv>(b"c2Det2\0");
    let (c_metric, r_metric) = libs.pair::<Metric>(b"c2GJKSimplexMetric\0");
    let (c_mulrv, r_mulrv) = libs.pair::<Rv>(b"c2Mulrv\0");
    let (c_add, r_add) = libs.pair::<Vv>(b"c2Add\0");
    let (c_mulxv, r_mulxv) = libs.pair::<Xv>(b"c2Mulxv\0");

    let mut rng = Rng::new();
    unsafe {
        for _ in 0..256 {
            let a = rng.v();
            let b = rng.v();
            let scalar = rng.f();
            assert_v(c_v(a.x, a.y), r_v(a.x, a.y));
            assert_v(c_mulvs(a, scalar), r_mulvs(a, scalar));
            assert_v(c_sub(a, b), r_sub(a, b));
            assert_f32(c_dot(a, b), r_dot(a, b));
            assert_f32(c_len(a), r_len(a));
            assert_f32(c_det(a, b), r_det(a, b));
            assert_v(c_add(a, b), r_add(a, b));

            let rotation = R {
                c: rng.f(),
                s: rng.f(),
            };
            let transform = X {
                p: rng.v(),
                r: rotation,
            };
            assert_v(c_mulrv(rotation, a), r_mulrv(rotation, a));
            assert_v(c_mulxv(transform, a), r_mulxv(transform, a));
        }

        for mask in 0..4 {
            for _ in 0..64 {
                let b = rng.v();
                let a = V {
                    x: if mask & 1 != 0 {
                        b.x + rng.positive()
                    } else if mask == 0 {
                        b.x
                    } else {
                        b.x - rng.positive()
                    },
                    y: if mask & 2 != 0 {
                        b.y + rng.positive()
                    } else if mask == 0 {
                        b.y
                    } else {
                        b.y - rng.positive()
                    },
                };
                assert_v(c_max(a, b), r_max(a, b));
                assert_v(c_min(a, b), r_min(a, b));
            }
        }

        let lo = V { x: -10.0, y: -20.0 };
        let hi = V { x: 10.0, y: 20.0 };
        let states_x = [-11.0, 0.0, 11.0];
        let states_y = [-21.0, 0.0, 21.0];
        for x in states_x {
            for y in states_y {
                let value = V { x, y };
                assert_v(c_clamp(value, lo, hi), r_clamp(value, lo, hi));
            }
        }

        assert_bytes(&c_rot_id(), &r_rot_id());
        assert_bytes(&c_x_id(), &r_x_id());

        for _ in 0..128 {
            let center = rng.v();
            let half = V {
                x: rng.positive(),
                y: rng.positive(),
            };
            let mut c_box = Aabb {
                min: V {
                    x: center.x - half.x,
                    y: center.y - half.y,
                },
                max: V {
                    x: center.x + half.x,
                    y: center.y + half.y,
                },
            };
            let mut r_box = c_box;
            let mut c_out = [V::default(); 4];
            let mut r_out = [V::default(); 4];
            c_bb(c_out.as_mut_ptr(), &mut c_box);
            r_bb(r_out.as_mut_ptr(), &mut r_box);
            assert_bytes(&c_out, &r_out);

            let circle = Circle {
                p: rng.v(),
                r: rng.positive(),
            };
            let capsule = Capsule {
                a: rng.v(),
                b: rng.v(),
                r: rng.positive(),
            };
            for (shape, kind) in [
                ((&circle as *const Circle).cast::<c_void>(), CIRCLE),
                ((&c_box as *const Aabb).cast::<c_void>(), AABB_TYPE),
                ((&capsule as *const Capsule).cast::<c_void>(), CAPSULE),
            ] {
                let mut cp: Proxy = poisoned();
                let mut rp = cp;
                c_proxy(shape, kind, &mut cp);
                r_proxy(shape, kind, &mut rp);
                assert_bytes(&cp, &rp);
            }
        }

        for count in [1, 2, 3, 0, 4, -1] {
            for _ in 0..64 {
                let mut cs = Simplex::default();
                cs.count = count;
                cs.a.p = rng.v();
                cs.b.p = rng.v();
                cs.c.p = rng.v();
                let mut rs = cs;
                assert_f32(c_metric(&mut cs), r_metric(&mut rs));
                assert_bytes(&cs, &rs);
            }
        }
    }
}

fn quarter_turn(v: V, turns: u64, scale: f32) -> V {
    let v = match turns & 3 {
        0 => v,
        1 => V { x: -v.y, y: v.x },
        2 => V { x: -v.x, y: -v.y },
        _ => V { x: v.y, y: -v.x },
    };
    V {
        x: v.x * scale,
        y: v.y * scale,
    }
}

#[test]
fn simplex_and_direction_surface_rows_37_through_69() {
    type MutSimplex = unsafe extern "C" fn(*mut Simplex);
    type SimplexV = unsafe extern "C" fn(*mut Simplex) -> V;
    type Support = unsafe extern "C" fn(*const V, c_int, V) -> c_int;
    type Witness = unsafe extern "C" fn(*mut Simplex, *mut V, *mut V);
    type Vv = unsafe extern "C" fn(V) -> V;
    type Vs = unsafe extern "C" fn(V, f32) -> V;
    type Rv = unsafe extern "C" fn(R, V) -> V;

    let libs = Libs::load();
    let (c_22, r_22) = libs.pair::<MutSimplex>(b"c22\0");
    let (c_23, r_23) = libs.pair::<MutSimplex>(b"c23\0");
    let (c_neg, r_neg) = libs.pair::<Vv>(b"c2Neg\0");
    let (c_skew, r_skew) = libs.pair::<Vv>(b"c2Skew\0");
    let (c_ccw, r_ccw) = libs.pair::<Vv>(b"c2CCW90\0");
    let (c_d, r_d) = libs.pair::<SimplexV>(b"c2D\0");
    let (c_support, r_support) = libs.pair::<Support>(b"c2Support\0");
    let (c_witness, r_witness) = libs.pair::<Witness>(b"c2Witness\0");
    let (c_div, r_div) = libs.pair::<Vs>(b"c2Div\0");
    let (c_norm, r_norm) = libs.pair::<Vv>(b"c2Norm\0");
    let (c_l, r_l) = libs.pair::<SimplexV>(b"c2L\0");
    let (c_mulrvt, r_mulrvt) = libs.pair::<Rv>(b"c2MulrvT\0");

    let mut rng = Rng::new();
    unsafe {
        let c22_cases = [
            (V { x: 1.0, y: 0.0 }, V { x: 2.0, y: 0.0 }, 1),
            (V { x: 2.0, y: 0.0 }, V { x: 1.0, y: 0.0 }, 1),
            (V { x: -1.0, y: 1.0 }, V { x: 1.0, y: 1.0 }, 2),
        ];
        for (a, b, expected_count) in c22_cases {
            for _ in 0..96 {
                let scale = rng.positive();
                let turns = rng.next();
                let mut cs = Simplex::default();
                cs.a.p = quarter_turn(a, turns, scale);
                cs.b.p = quarter_turn(b, turns, scale);
                cs.a.sA = rng.v();
                cs.a.sB = rng.v();
                cs.b.sA = rng.v();
                cs.b.sB = rng.v();
                cs.count = 2;
                let mut rs = cs;
                c_22(&mut cs);
                r_22(&mut rs);
                assert_eq!(cs.count, expected_count);
                assert_bytes(&cs, &rs);
            }
        }

        let c23_cases = [
            (
                V { x: 1.0, y: 0.0 },
                V { x: 2.0, y: 0.0 },
                V { x: 1.0, y: 1.0 },
                1,
            ),
            (
                V { x: 2.0, y: 0.0 },
                V { x: 1.0, y: 0.0 },
                V { x: 1.0, y: 1.0 },
                1,
            ),
            (
                V { x: 1.0, y: 1.0 },
                V { x: 2.0, y: 1.0 },
                V { x: 1.0, y: 0.0 },
                1,
            ),
            (
                V { x: -1.0, y: 1.0 },
                V { x: 1.0, y: 1.0 },
                V { x: 0.0, y: 3.0 },
                2,
            ),
            (
                V { x: 3.0, y: 0.0 },
                V { x: 1.0, y: -1.0 },
                V { x: 1.0, y: 1.0 },
                2,
            ),
            (
                V { x: 1.0, y: -1.0 },
                V { x: 3.0, y: 0.0 },
                V { x: 1.0, y: 1.0 },
                2,
            ),
            (
                V { x: -1.0, y: -1.0 },
                V { x: 1.0, y: -1.0 },
                V { x: 0.0, y: 1.0 },
                3,
            ),
        ];
        for (a, b, c, expected_count) in c23_cases {
            for _ in 0..96 {
                let scale = rng.positive();
                let turns = rng.next();
                let mut cs = Simplex::default();
                cs.a.p = quarter_turn(a, turns, scale);
                cs.b.p = quarter_turn(b, turns, scale);
                cs.c.p = quarter_turn(c, turns, scale);
                cs.a.sA = rng.v();
                cs.a.sB = rng.v();
                cs.b.sA = rng.v();
                cs.b.sB = rng.v();
                cs.c.sA = rng.v();
                cs.c.sB = rng.v();
                cs.count = 3;
                let mut rs = cs;
                c_23(&mut cs);
                r_23(&mut rs);
                assert_eq!(cs.count, expected_count);
                assert_bytes(&cs, &rs);
            }
        }

        for _ in 0..256 {
            let v = rng.v();
            assert_v(c_neg(v), r_neg(v));
            assert_v(c_skew(v), r_skew(v));
            assert_v(c_ccw(v), r_ccw(v));
            let divisor = if rng.next() & 1 == 0 {
                rng.positive()
            } else {
                -rng.positive()
            };
            assert_v(c_div(v, divisor), r_div(v, divisor));
            if v.x != 0.0 || v.y != 0.0 {
                assert_v(c_norm(v), r_norm(v));
            }
            let rotation = R {
                c: rng.f(),
                s: rng.f(),
            };
            assert_v(c_mulrvt(rotation, v), r_mulrvt(rotation, v));
        }
        assert_v(c_norm(V { x: 0.0, y: 0.0 }), r_norm(V { x: 0.0, y: 0.0 }));

        let d_cases = [
            (1, V { x: 2.0, y: 3.0 }, V::default()),
            (2, V { x: 1.0, y: 1.0 }, V { x: -1.0, y: 1.0 }),
            (2, V { x: 1.0, y: -1.0 }, V { x: -1.0, y: -1.0 }),
            (3, V { x: 2.0, y: 3.0 }, V { x: 4.0, y: 5.0 }),
            (0, V { x: 2.0, y: 3.0 }, V { x: 4.0, y: 5.0 }),
            (4, V { x: 2.0, y: 3.0 }, V { x: 4.0, y: 5.0 }),
        ];
        for (count, a, b) in d_cases {
            for _ in 0..64 {
                let turns = rng.next();
                let scale = rng.positive();
                let mut cs = Simplex::default();
                cs.count = count;
                cs.a.p = quarter_turn(a, turns, scale);
                cs.b.p = quarter_turn(b, turns, scale);
                let mut rs = cs;
                assert_v(c_d(&mut cs), r_d(&mut rs));
                assert_bytes(&cs, &rs);
            }
        }

        for _ in 0..128 {
            let one = [rng.v()];
            let d = rng.v();
            assert_eq!(c_support(one.as_ptr(), 1, d), r_support(one.as_ptr(), 1, d));

            let first_max = [
                V { x: 10.0, y: 0.0 },
                V { x: 5.0, y: 0.0 },
                V { x: -2.0, y: 0.0 },
            ];
            let later_max = [
                V { x: -2.0, y: 0.0 },
                V { x: 5.0, y: 0.0 },
                V { x: 10.0, y: 0.0 },
            ];
            let tied = [
                V { x: 10.0, y: 0.0 },
                V { x: 10.0, y: 0.0 },
                V { x: -2.0, y: 0.0 },
            ];
            let direction = V {
                x: rng.positive(),
                y: 0.0,
            };
            for verts in [first_max, later_max, tied] {
                assert_eq!(
                    c_support(verts.as_ptr(), 3, direction),
                    r_support(verts.as_ptr(), 3, direction)
                );
            }
        }

        for count in [1, 2, 3, 0, 4, -1] {
            for _ in 0..128 {
                let mut cs = Simplex::default();
                cs.count = count;
                cs.a.sA = rng.v();
                cs.a.sB = rng.v();
                cs.b.sA = rng.v();
                cs.b.sB = rng.v();
                cs.c.sA = rng.v();
                cs.c.sB = rng.v();
                cs.a.p = rng.v();
                cs.b.p = rng.v();
                cs.a.u = rng.positive();
                cs.b.u = rng.positive();
                cs.c.u = rng.positive();
                cs.div = cs.a.u + cs.b.u + cs.c.u;
                let mut rs = cs;
                let mut ca: V = poisoned();
                let mut cb: V = poisoned();
                let mut ra = ca;
                let mut rb = cb;
                c_witness(&mut cs, &mut ca, &mut cb);
                r_witness(&mut rs, &mut ra, &mut rb);
                assert_v(ca, ra);
                assert_v(cb, rb);
                assert_bytes(&cs, &rs);

                assert_v(c_l(&mut cs), r_l(&mut rs));
                assert_bytes(&cs, &rs);
            }
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

#[derive(Clone, Copy, Debug)]
enum Shape {
    Circle(Circle),
    Aabb(Aabb),
    Capsule(Capsule),
}

impl Shape {
    fn kind(&self) -> c_int {
        match self {
            Self::Circle(_) => CIRCLE,
            Self::Aabb(_) => AABB_TYPE,
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

fn random_shape(kind: c_int, rng: &mut Rng) -> Shape {
    match kind {
        CIRCLE => Shape::Circle(Circle {
            p: rng.v(),
            r: rng.positive(),
        }),
        AABB_TYPE => {
            let center = rng.v();
            let half = V {
                x: rng.positive(),
                y: rng.positive(),
            };
            Shape::Aabb(Aabb {
                min: V {
                    x: center.x - half.x,
                    y: center.y - half.y,
                },
                max: V {
                    x: center.x + half.x,
                    y: center.y + half.y,
                },
            })
        }
        CAPSULE => {
            let a = rng.v();
            let delta = V {
                x: rng.positive(),
                y: rng.positive(),
            };
            Shape::Capsule(Capsule {
                a,
                b: V {
                    x: a.x + delta.x,
                    y: a.y + delta.y,
                },
                r: rng.positive(),
            })
        }
        _ => unreachable!(),
    }
}

#[allow(clippy::too_many_arguments)]
unsafe fn compare_gjk(
    c_gjk: Gjk,
    r_gjk: Gjk,
    a: &Shape,
    ax: Option<&X>,
    b: &Shape,
    bx: Option<&X>,
    use_radius: c_int,
    write_a: bool,
    write_b: bool,
    write_iterations: bool,
    caches: Option<(&mut Cache, &mut Cache)>,
) {
    let mut ca: V = poisoned();
    let mut cb: V = poisoned();
    let mut ra = ca;
    let mut rb = cb;
    let mut ci = 0x5a5a_5a5a;
    let mut ri = ci;
    let ax_ptr = ax.map_or(std::ptr::null(), std::ptr::from_ref);
    let bx_ptr = bx.map_or(std::ptr::null(), std::ptr::from_ref);
    let ca_ptr = if write_a {
        &mut ca
    } else {
        std::ptr::null_mut()
    };
    let ra_ptr = if write_a {
        &mut ra
    } else {
        std::ptr::null_mut()
    };
    let cb_ptr = if write_b {
        &mut cb
    } else {
        std::ptr::null_mut()
    };
    let rb_ptr = if write_b {
        &mut rb
    } else {
        std::ptr::null_mut()
    };
    let ci_ptr = if write_iterations {
        &mut ci
    } else {
        std::ptr::null_mut()
    };
    let ri_ptr = if write_iterations {
        &mut ri
    } else {
        std::ptr::null_mut()
    };
    let (cc_ptr, rc_ptr) = match caches {
        Some((cc, rc)) => (cc as *mut Cache, rc as *mut Cache),
        None => (std::ptr::null_mut(), std::ptr::null_mut()),
    };

    let cd = unsafe {
        c_gjk(
            a.ptr(),
            a.kind(),
            ax_ptr,
            b.ptr(),
            b.kind(),
            bx_ptr,
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
            ax_ptr,
            b.ptr(),
            b.kind(),
            bx_ptr,
            ra_ptr,
            rb_ptr,
            use_radius,
            ri_ptr,
            rc_ptr,
        )
    };
    assert_f32(cd, rd);
    assert_v(ca, ra);
    assert_v(cb, rb);
    assert_eq!(ci, ri);
    if !cc_ptr.is_null() {
        assert_bytes(unsafe { &*cc_ptr }, unsafe { &*rc_ptr });
    }
}

#[test]
fn gjk_surface_rows_70_through_103() {
    let libs = Libs::load();
    let (c_gjk, r_gjk) = libs.pair::<Gjk>(b"c2GJK\0");
    let mut rng = Rng::new();

    unsafe {
        for use_radius in [0, 7] {
            for kind_a in [CIRCLE, AABB_TYPE, CAPSULE] {
                for kind_b in [CIRCLE, AABB_TYPE, CAPSULE] {
                    for _ in 0..64 {
                        let a = random_shape(kind_a, &mut rng);
                        let b = random_shape(kind_b, &mut rng);
                        compare_gjk(
                            c_gjk, r_gjk, &a, None, &b, None, use_radius, true, true, true, None,
                        );
                    }
                }
            }
        }

        let base_a = Shape::Aabb(Aabb {
            min: V { x: -2.0, y: -3.0 },
            max: V { x: 4.0, y: 5.0 },
        });
        let base_b = Shape::Capsule(Capsule {
            a: V { x: 10.0, y: -4.0 },
            b: V { x: 12.0, y: 8.0 },
            r: 2.0,
        });
        for _ in 0..96 {
            let ax = X {
                p: rng.v(),
                r: match rng.next() & 3 {
                    0 => R { c: 1.0, s: 0.0 },
                    1 => R { c: 0.0, s: 1.0 },
                    2 => R { c: -1.0, s: 0.0 },
                    _ => R { c: 0.0, s: -1.0 },
                },
            };
            let bx = X {
                p: rng.v(),
                r: match rng.next() & 3 {
                    0 => R { c: 1.0, s: 0.0 },
                    1 => R { c: 0.0, s: 1.0 },
                    2 => R { c: -1.0, s: 0.0 },
                    _ => R { c: 0.0, s: -1.0 },
                },
            };
            compare_gjk(
                c_gjk,
                r_gjk,
                &base_a,
                Some(&ax),
                &base_b,
                None,
                1,
                true,
                true,
                true,
                None,
            );
            compare_gjk(
                c_gjk,
                r_gjk,
                &base_a,
                None,
                &base_b,
                Some(&bx),
                1,
                true,
                true,
                true,
                None,
            );
            compare_gjk(
                c_gjk,
                r_gjk,
                &base_a,
                Some(&ax),
                &base_b,
                Some(&bx),
                1,
                true,
                true,
                true,
                None,
            );
        }

        for _ in 0..96 {
            let output_a = random_shape(AABB_TYPE, &mut rng);
            let output_b = random_shape(CAPSULE, &mut rng);
            compare_gjk(
                c_gjk, r_gjk, &output_a, None, &output_b, None, 1, false, true, true, None,
            );
            compare_gjk(
                c_gjk, r_gjk, &output_a, None, &output_b, None, 1, true, false, true, None,
            );
            compare_gjk(
                c_gjk, r_gjk, &output_a, None, &output_b, None, 1, true, true, false, None,
            );
            compare_gjk(
                c_gjk, r_gjk, &output_a, None, &output_b, None, 1, false, false, false, None,
            );
        }

        for count in [0, 1, 2, 3] {
            for _ in 0..96 {
                let mut cc = Cache {
                    metric: if count == 0 { rng.f() } else { 1.0 },
                    count,
                    iA: [0, 1, 2],
                    iB: [0, 1, 2],
                    div: 1.0,
                };
                let mut rc = cc;
                compare_gjk(
                    c_gjk,
                    r_gjk,
                    &base_a,
                    None,
                    &Shape::Aabb(Aabb {
                        min: V { x: 10.0, y: 10.0 },
                        max: V { x: 20.0, y: 20.0 },
                    }),
                    None,
                    1,
                    true,
                    true,
                    true,
                    Some((&mut cc, &mut rc)),
                );
                assert_bytes(&cc, &rc);
            }
        }

        for _ in 0..64 {
            let t = rng.v();
            let m = 6_000.0 + rng.positive();
            let cold_a = Shape::Aabb(Aabb { min: t, max: t });
            let cold_b = Shape::Aabb(Aabb {
                min: V {
                    x: t.x - m,
                    y: t.y - m,
                },
                max: V {
                    x: t.x + m,
                    y: t.y + m,
                },
            });
            let mut cc = Cache {
                metric: 1.0,
                count: 3,
                iA: [0, 0, 0],
                iB: [0, 3, 1],
                div: 1.0,
            };
            let mut rc = cc;
            compare_gjk(
                c_gjk,
                r_gjk,
                &cold_a,
                None,
                &cold_b,
                None,
                1,
                true,
                true,
                true,
                Some((&mut cc, &mut rc)),
            );
        }

        for _ in 0..96 {
            let t = rng.v();
            let scale = rng.positive();
            let geometry_cases = [
                (
                    Shape::Circle(Circle {
                        p: t,
                        r: 2.0 * scale,
                    }),
                    Shape::Circle(Circle {
                        p: V {
                            x: t.x + 10.0 * scale,
                            y: t.y,
                        },
                        r: 2.0 * scale,
                    }),
                ),
                (
                    Shape::Circle(Circle {
                        p: t,
                        r: 2.0 * scale,
                    }),
                    Shape::Circle(Circle {
                        p: V {
                            x: t.x + 4.0 * scale,
                            y: t.y,
                        },
                        r: 2.0 * scale,
                    }),
                ),
                (
                    Shape::Aabb(Aabb {
                        min: shifted(V { x: -2.0, y: -2.0 }, t),
                        max: shifted(V { x: 2.0, y: 2.0 }, t),
                    }),
                    Shape::Aabb(Aabb {
                        min: shifted(V { x: -1.0, y: -1.0 }, t),
                        max: shifted(V { x: 3.0, y: 3.0 }, t),
                    }),
                ),
                (
                    Shape::Capsule(Capsule { a: t, b: t, r: 0.0 }),
                    Shape::Aabb(Aabb { min: t, max: t }),
                ),
            ];
            for (a, b) in geometry_cases {
                for use_radius in [0, 1] {
                    compare_gjk(
                        c_gjk, r_gjk, &a, None, &b, None, use_radius, true, true, true, None,
                    );
                }
            }
        }
    }
}

fn shifted(v: V, by: V) -> V {
    V {
        x: v.x + by.x,
        y: v.y + by.y,
    }
}

#[test]
fn collision_surface_rows_104_through_138() {
    type Aa = unsafe extern "C" fn(Aabb, Aabb) -> c_int;
    type Ac = unsafe extern "C" fn(Aabb, Capsule) -> c_int;
    type Cc = unsafe extern "C" fn(Capsule, Capsule) -> c_int;
    type Oo = unsafe extern "C" fn(Circle, Circle) -> c_int;
    type Oa = unsafe extern "C" fn(Circle, Aabb) -> c_int;
    type Oc = unsafe extern "C" fn(Circle, Capsule) -> c_int;
    type Collided = unsafe extern "C" fn(*const c_void, c_int, *const c_void, c_int) -> c_int;
    type Reverse = unsafe extern "C" fn(f32, f32, f32) -> c_int;

    let libs = Libs::load();
    let (c_aa, r_aa) = libs.pair::<Aa>(b"c2AABBtoAABB\0");
    let (c_ac, r_ac) = libs.pair::<Ac>(b"c2AABBtoCapsule\0");
    let (c_cc, r_cc) = libs.pair::<Cc>(b"c2CapsuletoCapsule\0");
    let (c_oo, r_oo) = libs.pair::<Oo>(b"c2CircletoCircle\0");
    let (c_oa, r_oa) = libs.pair::<Oa>(b"c2CircletoAABB\0");
    let (c_oc, r_oc) = libs.pair::<Oc>(b"c2CircletoCapsule\0");
    let (c_collided, r_collided) = libs.pair::<Collided>(b"c2Collided\0");
    let (c_reverse, r_reverse) = libs.pair::<Reverse>(b"reverse_collide\0");
    let mut rng = Rng::new();

    unsafe {
        for _ in 0..128 {
            let t = rng.v();
            let boxes = [
                (
                    Aabb {
                        min: shifted(V { x: 0.0, y: 0.0 }, t),
                        max: shifted(V { x: 2.0, y: 2.0 }, t),
                    },
                    Aabb {
                        min: shifted(V { x: 3.0, y: 0.0 }, t),
                        max: shifted(V { x: 5.0, y: 2.0 }, t),
                    },
                    0,
                ),
                (
                    Aabb {
                        min: shifted(V { x: 0.0, y: 0.0 }, t),
                        max: shifted(V { x: 2.0, y: 2.0 }, t),
                    },
                    Aabb {
                        min: shifted(V { x: 0.0, y: 3.0 }, t),
                        max: shifted(V { x: 2.0, y: 5.0 }, t),
                    },
                    0,
                ),
                (
                    Aabb {
                        min: shifted(V { x: 0.0, y: 0.0 }, t),
                        max: shifted(V { x: 2.0, y: 2.0 }, t),
                    },
                    Aabb {
                        min: shifted(V { x: 2.0, y: 0.0 }, t),
                        max: shifted(V { x: 4.0, y: 2.0 }, t),
                    },
                    1,
                ),
                (
                    Aabb {
                        min: shifted(V { x: 0.0, y: 0.0 }, t),
                        max: shifted(V { x: 2.0, y: 2.0 }, t),
                    },
                    Aabb {
                        min: shifted(V { x: 1.0, y: 1.0 }, t),
                        max: shifted(V { x: 3.0, y: 3.0 }, t),
                    },
                    1,
                ),
            ];
            for (a, b, expected) in boxes {
                assert_eq!(c_aa(a, b), expected);
                assert_eq!(c_aa(a, b), r_aa(a, b));
            }

            let box_a = Aabb {
                min: shifted(V { x: 0.0, y: 0.0 }, t),
                max: shifted(V { x: 2.0, y: 2.0 }, t),
            };
            for (capsule, expected) in [
                (
                    Capsule {
                        a: shifted(V { x: 5.0, y: 0.0 }, t),
                        b: shifted(V { x: 5.0, y: 2.0 }, t),
                        r: 1.0,
                    },
                    0,
                ),
                (
                    Capsule {
                        a: shifted(V { x: 2.5, y: 0.0 }, t),
                        b: shifted(V { x: 2.5, y: 2.0 }, t),
                        r: 1.0,
                    },
                    1,
                ),
                (
                    Capsule {
                        a: shifted(V { x: 3.0, y: 0.0 }, t),
                        b: shifted(V { x: 3.0, y: 2.0 }, t),
                        r: 1.0,
                    },
                    1,
                ),
            ] {
                assert_eq!(c_ac(box_a, capsule), expected);
                assert_eq!(c_ac(box_a, capsule), r_ac(box_a, capsule));
            }

            let capsule_a = Capsule {
                a: shifted(V { x: 0.0, y: 0.0 }, t),
                b: shifted(V { x: 4.0, y: 0.0 }, t),
                r: 1.0,
            };
            for (capsule_b, expected) in [
                (
                    Capsule {
                        a: shifted(V { x: 0.0, y: 5.0 }, t),
                        b: shifted(V { x: 4.0, y: 5.0 }, t),
                        r: 1.0,
                    },
                    0,
                ),
                (
                    Capsule {
                        a: shifted(V { x: 0.0, y: 1.0 }, t),
                        b: shifted(V { x: 4.0, y: 1.0 }, t),
                        r: 1.0,
                    },
                    1,
                ),
                (
                    Capsule {
                        a: shifted(V { x: 0.0, y: 2.0 }, t),
                        b: shifted(V { x: 4.0, y: 2.0 }, t),
                        r: 1.0,
                    },
                    1,
                ),
            ] {
                assert_eq!(c_cc(capsule_a, capsule_b), expected);
                assert_eq!(c_cc(capsule_a, capsule_b), r_cc(capsule_a, capsule_b));
            }

            let circle_a = Circle {
                p: shifted(V { x: 0.0, y: 0.0 }, t),
                r: 2.0,
            };
            for (circle_b, expected) in [
                (
                    Circle {
                        p: shifted(V { x: 5.0, y: 0.0 }, t),
                        r: 2.0,
                    },
                    0,
                ),
                (
                    Circle {
                        p: shifted(V { x: 4.0, y: 0.0 }, t),
                        r: 2.0,
                    },
                    0,
                ),
                (
                    Circle {
                        p: shifted(V { x: 3.0, y: 0.0 }, t),
                        r: 2.0,
                    },
                    1,
                ),
            ] {
                assert_eq!(c_oo(circle_a, circle_b), expected);
                assert_eq!(c_oo(circle_a, circle_b), r_oo(circle_a, circle_b));
            }

            let aabb = Aabb {
                min: shifted(V { x: 0.0, y: 0.0 }, t),
                max: shifted(V { x: 4.0, y: 4.0 }, t),
            };
            let corner_tangent = std::f32::consts::SQRT_2;
            for (circle, expected) in [
                (
                    Circle {
                        p: shifted(V { x: 2.0, y: 2.0 }, t),
                        r: 0.25,
                    },
                    1,
                ),
                (
                    Circle {
                        p: shifted(V { x: 6.0, y: 2.0 }, t),
                        r: 1.0,
                    },
                    0,
                ),
                (
                    Circle {
                        p: shifted(V { x: 5.0, y: 2.0 }, t),
                        r: 1.0,
                    },
                    0,
                ),
                (
                    Circle {
                        p: shifted(V { x: 4.5, y: 2.0 }, t),
                        r: 1.0,
                    },
                    1,
                ),
                (
                    Circle {
                        p: shifted(V { x: 6.0, y: 6.0 }, t),
                        r: 1.0,
                    },
                    0,
                ),
                (
                    Circle {
                        p: shifted(V { x: 5.0, y: 5.0 }, t),
                        r: corner_tangent,
                    },
                    0,
                ),
                (
                    Circle {
                        p: shifted(V { x: 4.5, y: 4.5 }, t),
                        r: 1.0,
                    },
                    1,
                ),
            ] {
                assert_eq!(c_oa(circle, aabb), expected);
                assert_eq!(c_oa(circle, aabb), r_oa(circle, aabb));
            }

            let segment = Capsule {
                a: shifted(V { x: 0.0, y: 0.0 }, t),
                b: shifted(V { x: 10.0, y: 0.0 }, t),
                r: 1.0,
            };
            let circle_cases = [
                Circle {
                    p: shifted(V { x: -4.0, y: 0.0 }, t),
                    r: 1.0,
                },
                Circle {
                    p: shifted(V { x: -1.0, y: 0.0 }, t),
                    r: 1.0,
                },
                Circle {
                    p: shifted(V { x: 5.0, y: 4.0 }, t),
                    r: 1.0,
                },
                Circle {
                    p: shifted(V { x: 5.0, y: 1.0 }, t),
                    r: 1.0,
                },
                Circle {
                    p: shifted(V { x: 14.0, y: 0.0 }, t),
                    r: 1.0,
                },
                Circle {
                    p: shifted(V { x: 11.0, y: 0.0 }, t),
                    r: 1.0,
                },
                Circle {
                    p: shifted(V { x: 5.0, y: 2.0 }, t),
                    r: 1.0,
                },
            ];
            for circle in circle_cases {
                assert_eq!(c_oc(circle, segment), r_oc(circle, segment));
            }
            let point_capsule = Capsule {
                a: shifted(V { x: 0.0, y: 0.0 }, t),
                b: shifted(V { x: 0.0, y: 0.0 }, t),
                r: 1.0,
            };
            let point_circle = Circle {
                p: shifted(V { x: 0.5, y: 0.0 }, t),
                r: 1.0,
            };
            assert_eq!(
                c_oc(point_circle, point_capsule),
                r_oc(point_circle, point_capsule)
            );
        }

        for kind_a in [CIRCLE, AABB_TYPE, CAPSULE] {
            for kind_b in [CIRCLE, AABB_TYPE, CAPSULE] {
                for _ in 0..128 {
                    let a = random_shape(kind_a, &mut rng);
                    let b = random_shape(kind_b, &mut rng);
                    assert_eq!(
                        c_collided(a.ptr(), a.kind(), b.ptr(), b.kind()),
                        r_collided(a.ptr(), a.kind(), b.ptr(), b.kind())
                    );
                }
            }
        }

        for _ in 0..4096 {
            let x = rng.f();
            let y = rng.f();
            let r = rng.f();
            assert_eq!(c_reverse(x, y, r), r_reverse(x, y, r));
        }
        for (x, y, r) in [
            (-70.0, 0.0, 0.0),
            (-50.0, 0.0, 0.0),
            (-40.0, -40.0, 0.0),
            (-15.0, -15.0, 0.0),
            (-40.0, 40.0, 0.0),
            (-20.0, 100.0, 0.0),
            (0.0, 0.0, f32::MAX),
            (0.0, 0.0, -f32::MAX),
        ] {
            assert_eq!(c_reverse(x, y, r), r_reverse(x, y, r));
        }
    }
}

#[test]
fn error_surface_rows_1_through_5_and_generic_boundaries() {
    type MakeProxy = unsafe extern "C" fn(*const c_void, c_int, *mut Proxy);
    type Collided = unsafe extern "C" fn(*const c_void, c_int, *const c_void, c_int) -> c_int;
    type Support = unsafe extern "C" fn(*const V, c_int, V) -> c_int;

    let libs = Libs::load();
    let (c_proxy, r_proxy) = libs.pair::<MakeProxy>(b"c2MakeProxy\0");
    let (c_collided, r_collided) = libs.pair::<Collided>(b"c2Collided\0");
    let (c_support, r_support) = libs.pair::<Support>(b"c2Support\0");

    unsafe {
        for invalid in [-1, 3, 17, c_int::MIN, c_int::MAX] {
            let mut cp: Proxy = poisoned();
            let mut rp = cp;
            c_proxy(std::ptr::null(), invalid, &mut cp);
            r_proxy(std::ptr::null(), invalid, &mut rp);
            assert_bytes(&cp, &rp);

            c_proxy(std::ptr::null(), invalid, std::ptr::null_mut());
            r_proxy(std::ptr::null(), invalid, std::ptr::null_mut());

            assert_eq!(
                c_collided(std::ptr::null(), invalid, std::ptr::null(), invalid),
                0
            );
            assert_eq!(
                c_collided(std::ptr::null(), invalid, std::ptr::null(), invalid),
                r_collided(std::ptr::null(), invalid, std::ptr::null(), invalid)
            );
            for valid_a in [CIRCLE, AABB_TYPE, CAPSULE] {
                assert_eq!(
                    c_collided(std::ptr::null(), valid_a, std::ptr::null(), invalid),
                    0
                );
                assert_eq!(
                    c_collided(std::ptr::null(), valid_a, std::ptr::null(), invalid),
                    r_collided(std::ptr::null(), valid_a, std::ptr::null(), invalid)
                );
            }
        }

        let verts = [V { x: 1.0, y: 2.0 }; 32];
        for count in [0, -1, -100, 1, 8, 32] {
            assert_eq!(
                c_support(verts.as_ptr(), count, V { x: 3.0, y: 4.0 }),
                r_support(verts.as_ptr(), count, V { x: 3.0, y: 4.0 })
            );
        }
    }
}
