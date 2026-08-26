#![allow(unsafe_op_in_unsafe_fn)]

use libloading::Library;
use std::ffi::{c_float, c_int, c_void};
use std::mem::size_of;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

const CIRCLE: c_int = 0;
const AABB: c_int = 1;
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
struct Box2 {
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
    i_a: [c_int; 3],
    i_b: [c_int; 3],
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
    s_a: V,
    s_b: V,
    p: V,
    u: c_float,
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
    div: c_float,
    count: c_int,
}

struct Libraries {
    c: Library,
    rust: Library,
}

static BUILD: OnceLock<()> = OnceLock::new();

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn run(command: &mut Command, description: &str) {
    let status = command
        .status()
        .unwrap_or_else(|error| panic!("{description}: {error}"));
    assert!(status.success(), "{description} failed with {status}");
}

fn ensure_shared_libraries() {
    BUILD.get_or_init(|| {
        let root = manifest_dir();
        let c_build = root.join("c_src/build");
        if !c_build.join("libtranslated_rust.so").is_file() {
            run(
                Command::new("cmake")
                    .arg("-S")
                    .arg(root.join("c_src"))
                    .arg("-B")
                    .arg(&c_build)
                    .arg("-DCMAKE_POSITION_INDEPENDENT_CODE=ON"),
                "configure C shared library",
            );
            run(
                Command::new("cmake").arg("--build").arg(&c_build),
                "build C shared library",
            );
        }

        let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
        run(
            Command::new(cargo)
                .current_dir(&root)
                .arg("rustc")
                .arg("--no-default-features")
                .arg("--lib")
                .arg("--")
                .arg("--crate-type=cdylib"),
            "build Rust cdylib",
        );
    });
}

fn rust_library_path(root: &Path) -> PathBuf {
    let target = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("target"));
    target.join("debug/libaabb_lib.so")
}

fn libraries() -> Libraries {
    ensure_shared_libraries();
    let root = manifest_dir();
    let c_path = root.join("c_src/build/libtranslated_rust.so");
    let rust_path = rust_library_path(&root);
    assert!(c_path.is_file(), "missing {}", c_path.display());
    assert!(rust_path.is_file(), "missing {}", rust_path.display());
    unsafe {
        Libraries {
            c: Library::new(c_path).expect("load C library"),
            rust: Library::new(rust_path).expect("load Rust library"),
        }
    }
}

unsafe fn symbols<T: Copy>(libraries: &Libraries, name: &[u8]) -> (T, T) {
    let c = *libraries.c.get::<T>(name).expect("C symbol");
    let rust = *libraries.rust.get::<T>(name).expect("Rust symbol");
    (c, rust)
}

fn bytes<T>(value: &T) -> &[u8] {
    unsafe { std::slice::from_raw_parts((value as *const T).cast(), size_of::<T>()) }
}

fn assert_bytes<T>(left: &T, right: &T, context: &str) {
    assert_eq!(bytes(left), bytes(right), "{context}");
}

fn assert_float(left: c_float, right: c_float, context: &str) {
    assert_eq!(
        left.to_bits(),
        right.to_bits(),
        "{context}: {left:?} != {right:?}"
    );
}

#[derive(Clone)]
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x as u32
    }

    fn f(&mut self) -> c_float {
        let signed = (self.next_u32() % 20_001) as i32 - 10_000;
        signed as c_float / 37.0
    }

    fn positive(&mut self) -> c_float {
        (self.next_u32() % 5_000 + 1) as c_float / 101.0
    }

    fn v(&mut self) -> V {
        V {
            x: self.f(),
            y: self.f(),
        }
    }
}

fn sentinel_v() -> V {
    V {
        x: f32::from_bits(0x7fc0_0123),
        y: f32::from_bits(0x7fc0_0456),
    }
}

fn sv(point: V) -> Sv {
    Sv {
        s_a: V {
            x: point.x + 3.0,
            y: point.y - 2.0,
        },
        s_b: V {
            x: point.x - 5.0,
            y: point.y + 7.0,
        },
        p: point,
        u: 11.0,
        i_a: 1,
        i_b: 2,
    }
}

fn simplex(points: &[V], count: c_int) -> Simplex {
    let mut result = Simplex {
        div: 13.0,
        count,
        ..Simplex::default()
    };
    if let Some(point) = points.first() {
        result.a = sv(*point);
    }
    if let Some(point) = points.get(1) {
        result.b = sv(*point);
    }
    if let Some(point) = points.get(2) {
        result.c = sv(*point);
    }
    result.d = sv(V { x: 17.0, y: 19.0 });
    result
}

#[derive(Clone, Copy)]
enum Shape {
    Circle(Circle),
    Aabb(Box2),
    Capsule(Capsule),
}

impl Shape {
    fn kind(&self) -> c_int {
        match self {
            Self::Circle(_) => CIRCLE,
            Self::Aabb(_) => AABB,
            Self::Capsule(_) => CAPSULE,
        }
    }

    fn ptr(&self) -> *const c_void {
        match self {
            Self::Circle(value) => (value as *const Circle).cast(),
            Self::Aabb(value) => (value as *const Box2).cast(),
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
            let p = rng.v();
            let extent = V {
                x: rng.positive(),
                y: rng.positive(),
            };
            Shape::Aabb(Box2 {
                min: p,
                max: V {
                    x: p.x + extent.x,
                    y: p.y + extent.y,
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
) -> c_float;

#[derive(Clone, Copy)]
struct GjkOptions<'a> {
    ax: Option<&'a X>,
    bx: Option<&'a X>,
    use_radius: c_int,
    outputs: u8,
}

unsafe fn compare_gjk(
    libraries: &Libraries,
    shape_a: &Shape,
    shape_b: &Shape,
    options: GjkOptions<'_>,
    caches: Option<(&mut Cache, &mut Cache)>,
    context: &str,
) {
    let (c_fn, rust_fn) = symbols::<Gjk>(libraries, b"c2GJK\0");
    let ax = options.ax.map_or(std::ptr::null(), std::ptr::from_ref);
    let bx = options.bx.map_or(std::ptr::null(), std::ptr::from_ref);
    let mut c_a = sentinel_v();
    let mut r_a = sentinel_v();
    let mut c_b = sentinel_v();
    let mut r_b = sentinel_v();
    let mut c_iterations = 0x1234_5678;
    let mut r_iterations = 0x1234_5678;
    let c_a_ptr = if options.outputs & 1 != 0 {
        &mut c_a
    } else {
        std::ptr::null_mut()
    };
    let r_a_ptr = if options.outputs & 1 != 0 {
        &mut r_a
    } else {
        std::ptr::null_mut()
    };
    let c_b_ptr = if options.outputs & 2 != 0 {
        &mut c_b
    } else {
        std::ptr::null_mut()
    };
    let r_b_ptr = if options.outputs & 2 != 0 {
        &mut r_b
    } else {
        std::ptr::null_mut()
    };
    let c_i_ptr = if options.outputs & 4 != 0 {
        &mut c_iterations
    } else {
        std::ptr::null_mut()
    };
    let r_i_ptr = if options.outputs & 4 != 0 {
        &mut r_iterations
    } else {
        std::ptr::null_mut()
    };
    let (c_cache, r_cache) = match caches {
        Some((c_cache, r_cache)) => (c_cache as *mut Cache, r_cache as *mut Cache),
        None => (std::ptr::null_mut(), std::ptr::null_mut()),
    };

    let c_result = c_fn(
        shape_a.ptr(),
        shape_a.kind(),
        ax,
        shape_b.ptr(),
        shape_b.kind(),
        bx,
        c_a_ptr,
        c_b_ptr,
        options.use_radius,
        c_i_ptr,
        c_cache,
    );
    let r_result = rust_fn(
        shape_a.ptr(),
        shape_a.kind(),
        ax,
        shape_b.ptr(),
        shape_b.kind(),
        bx,
        r_a_ptr,
        r_b_ptr,
        options.use_radius,
        r_i_ptr,
        r_cache,
    );
    assert_float(c_result, r_result, context);
    if options.outputs & 1 != 0 {
        assert_bytes(&c_a, &r_a, context);
    }
    if options.outputs & 2 != 0 {
        assert_bytes(&c_b, &r_b, context);
    }
    if options.outputs & 4 != 0 {
        assert_eq!(c_iterations, r_iterations, "{context}");
    }
    if !c_cache.is_null() {
        assert_bytes(&*c_cache, &*r_cache, context);
    }
}

#[test]
fn vector_proxy_and_transform_surface() {
    unsafe {
        let libraries = libraries();
        let mut rng = Rng::new(0x1042_9385_abcd_7711);

        let (c_v, r_v) =
            symbols::<unsafe extern "C" fn(c_float, c_float) -> V>(&libraries, b"c2V\0");
        let (c_mulvs, r_mulvs) =
            symbols::<unsafe extern "C" fn(V, c_float) -> V>(&libraries, b"c2Mulvs\0");
        let (c_max, r_max) = symbols::<unsafe extern "C" fn(V, V) -> V>(&libraries, b"c2Maxv\0");
        let (c_min, r_min) = symbols::<unsafe extern "C" fn(V, V) -> V>(&libraries, b"c2Minv\0");
        let (c_clamp, r_clamp) =
            symbols::<unsafe extern "C" fn(V, V, V) -> V>(&libraries, b"c2Clampv\0");
        let (c_sub, r_sub) = symbols::<unsafe extern "C" fn(V, V) -> V>(&libraries, b"c2Sub\0");
        let (c_dot, r_dot) =
            symbols::<unsafe extern "C" fn(V, V) -> c_float>(&libraries, b"c2Dot\0");
        let (c_add, r_add) = symbols::<unsafe extern "C" fn(V, V) -> V>(&libraries, b"c2Add\0");
        let (c_neg, r_neg) = symbols::<unsafe extern "C" fn(V) -> V>(&libraries, b"c2Neg\0");
        let (c_skew, r_skew) = symbols::<unsafe extern "C" fn(V) -> V>(&libraries, b"c2Skew\0");
        let (c_ccw, r_ccw) = symbols::<unsafe extern "C" fn(V) -> V>(&libraries, b"c2CCW90\0");
        let (c_len, r_len) = symbols::<unsafe extern "C" fn(V) -> c_float>(&libraries, b"c2Len\0");
        let (c_det, r_det) =
            symbols::<unsafe extern "C" fn(V, V) -> c_float>(&libraries, b"c2Det2\0");
        let (c_div, r_div) =
            symbols::<unsafe extern "C" fn(V, c_float) -> V>(&libraries, b"c2Div\0");
        let (c_norm, r_norm) = symbols::<unsafe extern "C" fn(V) -> V>(&libraries, b"c2Norm\0");
        let (c_rot_id, r_rot_id) =
            symbols::<unsafe extern "C" fn() -> R>(&libraries, b"c2RotIdentity\0");
        let (c_x_id, r_x_id) = symbols::<unsafe extern "C" fn() -> X>(&libraries, b"c2xIdentity\0");
        let (c_mulrv, r_mulrv) =
            symbols::<unsafe extern "C" fn(R, V) -> V>(&libraries, b"c2Mulrv\0");
        let (c_mulrvt, r_mulrvt) =
            symbols::<unsafe extern "C" fn(R, V) -> V>(&libraries, b"c2MulrvT\0");
        let (c_mulxv, r_mulxv) =
            symbols::<unsafe extern "C" fn(X, V) -> V>(&libraries, b"c2Mulxv\0");

        assert_bytes(&c_rot_id(), &r_rot_id(), "c2RotIdentity");
        assert_bytes(&c_x_id(), &r_x_id(), "c2xIdentity");
        for iteration in 0..128 {
            let a = rng.v();
            let b = rng.v();
            let scalar = rng.f();
            assert_bytes(&c_v(a.x, a.y), &r_v(a.x, a.y), "c2V");
            assert_bytes(&c_mulvs(a, scalar), &r_mulvs(a, scalar), "c2Mulvs");
            assert_bytes(&c_sub(a, b), &r_sub(a, b), "c2Sub");
            assert_float(c_dot(a, b), r_dot(a, b), "c2Dot");
            assert_bytes(&c_add(a, b), &r_add(a, b), "c2Add");
            assert_bytes(&c_neg(a), &r_neg(a), "c2Neg");
            assert_bytes(&c_skew(a), &r_skew(a), "c2Skew");
            assert_bytes(&c_ccw(a), &r_ccw(a), "c2CCW90");
            assert_float(c_len(a), r_len(a), "c2Len");
            assert_float(c_det(a, b), r_det(a, b), "c2Det2");
            let divisor = rng.positive();
            assert_bytes(&c_div(a, divisor), &r_div(a, divisor), "c2Div nonzero");
            if a.x != 0.0 || a.y != 0.0 {
                assert_bytes(&c_norm(a), &r_norm(a), "c2Norm nonzero");
            }
            let rotation = R {
                c: rng.f(),
                s: rng.f(),
            };
            assert_bytes(&c_mulrv(rotation, a), &r_mulrv(rotation, a), "c2Mulrv");
            assert_bytes(&c_mulrvt(rotation, a), &r_mulrvt(rotation, a), "c2MulrvT");
            let transform = X { p: b, r: rotation };
            assert_bytes(&c_mulxv(transform, a), &r_mulxv(transform, a), "c2Mulxv");

            for state in 0..4 {
                let delta_x = rng.positive();
                let delta_y = rng.positive();
                let base = rng.v();
                let lhs = V {
                    x: if state & 1 != 0 {
                        base.x + delta_x
                    } else {
                        base.x
                    },
                    y: if state & 2 != 0 {
                        base.y + delta_y
                    } else {
                        base.y
                    },
                };
                let rhs = V {
                    x: if state & 1 != 0 {
                        base.x
                    } else if iteration % 8 == 0 {
                        base.x
                    } else {
                        base.x + delta_x
                    },
                    y: if state & 2 != 0 {
                        base.y
                    } else if iteration % 8 == 0 {
                        base.y
                    } else {
                        base.y + delta_y
                    },
                };
                assert_bytes(&c_max(lhs, rhs), &r_max(lhs, rhs), "c2Maxv branch");

                let min_lhs = V {
                    x: if state & 1 != 0 {
                        base.x - delta_x
                    } else {
                        base.x
                    },
                    y: if state & 2 != 0 {
                        base.y - delta_y
                    } else {
                        base.y
                    },
                };
                let min_rhs = V {
                    x: if state & 1 != 0 {
                        base.x
                    } else if iteration % 8 == 0 {
                        base.x
                    } else {
                        base.x - delta_x
                    },
                    y: if state & 2 != 0 {
                        base.y
                    } else if iteration % 8 == 0 {
                        base.y
                    } else {
                        base.y - delta_y
                    },
                };
                assert_bytes(
                    &c_min(min_lhs, min_rhs),
                    &r_min(min_lhs, min_rhs),
                    "c2Minv branch",
                );
            }

            let lo = rng.v();
            let hi = V {
                x: lo.x + rng.positive() + 1.0,
                y: lo.y + rng.positive() + 1.0,
            };
            for x_state in 0..3 {
                for y_state in 0..3 {
                    let component = |low: c_float, high: c_float, state: i32| match state {
                        0 => low - 1.0,
                        1 if iteration % 3 == 0 => low,
                        1 if iteration % 3 == 1 => high,
                        1 => (low + high) * 0.5,
                        _ => high + 1.0,
                    };
                    let value = V {
                        x: component(lo.x, hi.x, x_state),
                        y: component(lo.y, hi.y, y_state),
                    };
                    assert_bytes(
                        &c_clamp(value, lo, hi),
                        &r_clamp(value, lo, hi),
                        "c2Clampv branch",
                    );
                }
            }
        }

        let zero = V::default();
        for _ in 0..128 {
            let value = rng.v();
            assert_bytes(&c_div(value, 0.0), &r_div(value, 0.0), "c2Div zero");
        }
        assert_bytes(&c_norm(zero), &r_norm(zero), "c2Norm zero");

        let (c_bb, r_bb) =
            symbols::<unsafe extern "C" fn(*mut V, *mut Box2)>(&libraries, b"c2BBVerts\0");
        let (c_proxy, r_proxy) = symbols::<unsafe extern "C" fn(*const c_void, c_int, *mut Proxy)>(
            &libraries,
            b"c2MakeProxy\0",
        );
        for _ in 0..64 {
            let box_shape = match random_shape(&mut rng, AABB) {
                Shape::Aabb(value) => value,
                _ => unreachable!(),
            };
            let mut c_box = box_shape;
            let mut r_box = box_shape;
            let mut c_verts = [sentinel_v(); 4];
            let mut r_verts = [sentinel_v(); 4];
            c_bb(c_verts.as_mut_ptr(), &mut c_box);
            r_bb(r_verts.as_mut_ptr(), &mut r_box);
            assert_bytes(&c_verts, &r_verts, "c2BBVerts");

            for kind in [CIRCLE, AABB, CAPSULE] {
                let shape = random_shape(&mut rng, kind);
                let initial = Proxy {
                    radius: f32::from_bits(0x7fc0_0abc),
                    count: 0x1234_5678,
                    verts: [sentinel_v(); 8],
                };
                let mut c_out = initial;
                let mut r_out = initial;
                c_proxy(shape.ptr(), kind, &mut c_out);
                r_proxy(shape.ptr(), kind, &mut r_out);
                assert_bytes(&c_out, &r_out, "c2MakeProxy");
            }
        }
        for invalid in [-1, 3, 4, c_int::MAX, c_int::MIN] {
            c_proxy(std::ptr::null(), invalid, std::ptr::null_mut());
            r_proxy(std::ptr::null(), invalid, std::ptr::null_mut());
        }
    }
}

#[test]
fn simplex_support_and_witness_surface() {
    unsafe {
        let libraries = libraries();
        let mut rng = Rng::new(0xca55_71ce_8923_4421);
        let (c_metric, r_metric) = symbols::<unsafe extern "C" fn(*mut Simplex) -> c_float>(
            &libraries,
            b"c2GJKSimplexMetric\0",
        );
        let (c_22, r_22) = symbols::<unsafe extern "C" fn(*mut Simplex)>(&libraries, b"c22\0");
        let (c_23, r_23) = symbols::<unsafe extern "C" fn(*mut Simplex)>(&libraries, b"c23\0");
        let (c_d, r_d) = symbols::<unsafe extern "C" fn(*mut Simplex) -> V>(&libraries, b"c2D\0");
        let (c_support, r_support) = symbols::<unsafe extern "C" fn(*const V, c_int, V) -> c_int>(
            &libraries,
            b"c2Support\0",
        );
        let (c_witness, r_witness) = symbols::<unsafe extern "C" fn(*mut Simplex, *mut V, *mut V)>(
            &libraries,
            b"c2Witness\0",
        );
        let (c_l, r_l) = symbols::<unsafe extern "C" fn(*mut Simplex) -> V>(&libraries, b"c2L\0");

        for count in [0, 1, 2, 3, 7] {
            for _ in 0..64 {
                let points = [rng.v(), rng.v(), rng.v()];
                let mut c_s = simplex(&points, count);
                let mut r_s = c_s;
                assert_float(c_metric(&mut c_s), r_metric(&mut r_s), "c2GJKSimplexMetric");
            }
        }

        let line_cases = [
            ([V { x: 1.0, y: 0.0 }, V { x: 2.0, y: 0.0 }], 1),
            ([V { x: -2.0, y: 0.0 }, V { x: -1.0, y: 0.0 }], 1),
            ([V { x: -1.0, y: 0.0 }, V { x: 1.0, y: 0.0 }], 2),
        ];
        for (points, expected_count) in line_cases {
            for _ in 0..64 {
                let scale = rng.positive();
                let scaled = [
                    V {
                        x: points[0].x * scale,
                        y: points[0].y * scale,
                    },
                    V {
                        x: points[1].x * scale,
                        y: points[1].y * scale,
                    },
                ];
                let mut c_s = simplex(&scaled, 2);
                let mut r_s = c_s;
                c_22(&mut c_s);
                r_22(&mut r_s);
                assert_eq!(c_s.count, expected_count);
                assert_bytes(&c_s, &r_s, "c22 branch");
            }
        }

        let triangle_cases = [
            (
                [
                    V { x: 1.0, y: 0.0 },
                    V { x: 2.0, y: 1.0 },
                    V { x: 2.0, y: -1.0 },
                ],
                1,
            ),
            (
                [
                    V { x: 2.0, y: -1.0 },
                    V { x: 1.0, y: 0.0 },
                    V { x: 2.0, y: 1.0 },
                ],
                1,
            ),
            (
                [
                    V { x: 2.0, y: 1.0 },
                    V { x: 2.0, y: -1.0 },
                    V { x: 1.0, y: 0.0 },
                ],
                1,
            ),
            (
                [
                    V { x: -1.0, y: 1.0 },
                    V { x: 1.0, y: 1.0 },
                    V { x: 0.0, y: 2.0 },
                ],
                2,
            ),
            (
                [
                    V { x: 0.0, y: 2.0 },
                    V { x: -1.0, y: 1.0 },
                    V { x: 1.0, y: 1.0 },
                ],
                2,
            ),
            (
                [
                    V { x: 1.0, y: 1.0 },
                    V { x: 0.0, y: 2.0 },
                    V { x: -1.0, y: 1.0 },
                ],
                2,
            ),
            (
                [
                    V { x: -1.0, y: -1.0 },
                    V { x: 1.0, y: -1.0 },
                    V { x: 0.0, y: 1.0 },
                ],
                3,
            ),
        ];
        for (points, expected_count) in triangle_cases {
            for _ in 0..64 {
                let scale = rng.positive();
                let scaled = points.map(|point| V {
                    x: point.x * scale,
                    y: point.y * scale,
                });
                let mut c_s = simplex(&scaled, 3);
                let mut r_s = c_s;
                c_23(&mut c_s);
                r_23(&mut r_s);
                assert_eq!(c_s.count, expected_count);
                assert_bytes(&c_s, &r_s, "c23 branch");
            }
        }

        for count in [0, 1, 2, 3, 8] {
            for _ in 0..64 {
                let points = [rng.v(), rng.v(), rng.v()];
                let mut c_s = simplex(&points, count);
                let mut r_s = c_s;
                assert_bytes(&c_d(&mut c_s), &r_d(&mut r_s), "c2D count");
                assert_bytes(&c_l(&mut c_s), &r_l(&mut r_s), "c2L count");
            }
        }
        for determinant_positive in [false, true] {
            for _ in 0..64 {
                let scale = rng.positive();
                let points = if determinant_positive {
                    [
                        V {
                            x: 1.0 * scale,
                            y: 0.0,
                        },
                        V {
                            x: 1.0 * scale,
                            y: 1.0 * scale,
                        },
                    ]
                } else {
                    [
                        V {
                            x: 1.0 * scale,
                            y: 0.0,
                        },
                        V {
                            x: 1.0 * scale,
                            y: -1.0 * scale,
                        },
                    ]
                };
                let mut c_s = simplex(&points, 2);
                let mut r_s = c_s;
                assert_bytes(&c_d(&mut c_s), &r_d(&mut r_s), "c2D orientation");
            }
        }

        for _ in 0..128 {
            let direction = rng.v();
            let first = rng.v();
            let mut vertices = [first; 8];
            assert_eq!(
                c_support(vertices.as_ptr(), 0, direction),
                r_support(vertices.as_ptr(), 0, direction)
            );
            assert_eq!(
                c_support(vertices.as_ptr(), 1, direction),
                r_support(vertices.as_ptr(), 1, direction)
            );
            for vertex in &mut vertices[1..] {
                *vertex = first;
            }
            assert_eq!(
                c_support(vertices.as_ptr(), 8, direction),
                r_support(vertices.as_ptr(), 8, direction)
            );
            let axis = V { x: 1.0, y: 0.0 };
            for (index, vertex) in vertices.iter_mut().enumerate() {
                *vertex = V {
                    x: index as c_float,
                    y: rng.f(),
                };
            }
            assert_eq!(
                c_support(vertices.as_ptr(), 8, axis),
                r_support(vertices.as_ptr(), 8, axis)
            );
        }

        for count in [0, 1, 2, 3, 9] {
            for _ in 0..64 {
                let points = [rng.v(), rng.v(), rng.v()];
                let mut c_s = simplex(&points, count);
                c_s.div = rng.positive();
                c_s.a.u = rng.positive();
                c_s.b.u = rng.positive();
                c_s.c.u = rng.positive();
                let mut r_s = c_s;
                let mut c_a = sentinel_v();
                let mut c_b = sentinel_v();
                let mut r_a = sentinel_v();
                let mut r_b = sentinel_v();
                c_witness(&mut c_s, &mut c_a, &mut c_b);
                r_witness(&mut r_s, &mut r_a, &mut r_b);
                assert_bytes(&c_a, &r_a, "c2Witness A");
                assert_bytes(&c_b, &r_b, "c2Witness B");
            }
        }
    }
}

#[test]
fn gjk_surface() {
    unsafe {
        let libraries = libraries();
        let mut rng = Rng::new(0xd1ff_e2e7_5eed_0042);
        let kinds = [CIRCLE, AABB, CAPSULE];
        let identity_options = |use_radius| GjkOptions {
            ax: None,
            bx: None,
            use_radius,
            outputs: 7,
        };

        for kind_a in kinds {
            for kind_b in kinds {
                for use_radius in [0, 1] {
                    for iteration in 0..48 {
                        let shape_a = random_shape(&mut rng, kind_a);
                        let shape_b = random_shape(&mut rng, kind_b);
                        compare_gjk(
                            &libraries,
                            &shape_a,
                            &shape_b,
                            identity_options(use_radius),
                            None,
                            &format!(
                                "c2GJK pair {kind_a}/{kind_b} radius {use_radius} iteration {iteration}"
                            ),
                        );
                    }
                }
            }
        }

        let centered_shape = |kind, offset: c_float| match kind {
            CIRCLE => Shape::Circle(Circle {
                p: V {
                    x: offset,
                    y: offset,
                },
                r: 2.0,
            }),
            AABB => Shape::Aabb(Box2 {
                min: V {
                    x: offset - 2.0,
                    y: offset - 2.0,
                },
                max: V {
                    x: offset + 2.0,
                    y: offset + 2.0,
                },
            }),
            CAPSULE => Shape::Capsule(Capsule {
                a: V {
                    x: offset - 2.0,
                    y: offset,
                },
                b: V {
                    x: offset + 2.0,
                    y: offset,
                },
                r: 2.0,
            }),
            _ => unreachable!(),
        };
        for kind_a in kinds {
            for kind_b in kinds {
                for _ in 0..64 {
                    let jitter = (rng.next_u32() % 100) as c_float / 1_000.0;
                    let shape_a = centered_shape(kind_a, jitter);
                    let overlapping = centered_shape(kind_b, jitter + 0.25);
                    let separated = centered_shape(kind_b, jitter + 20.0);
                    compare_gjk(
                        &libraries,
                        &shape_a,
                        &overlapping,
                        identity_options(1),
                        None,
                        &format!("c2GJK guaranteed overlap {kind_a}/{kind_b}"),
                    );
                    compare_gjk(
                        &libraries,
                        &shape_a,
                        &separated,
                        identity_options(1),
                        None,
                        &format!("c2GJK guaranteed separation {kind_a}/{kind_b}"),
                    );
                }
            }
        }

        let rotations = [
            R { c: 1.0, s: 0.0 },
            R { c: 0.0, s: 1.0 },
            R { c: -1.0, s: 0.0 },
            R { c: 0.0, s: -1.0 },
        ];
        for kind_a in kinds {
            for kind_b in kinds {
                for iteration in 0..32 {
                    let shape_a = random_shape(&mut rng, kind_a);
                    let shape_b = random_shape(&mut rng, kind_b);
                    let ax = X {
                        p: rng.v(),
                        r: rotations[(rng.next_u32() as usize) % rotations.len()],
                    };
                    let bx = X {
                        p: rng.v(),
                        r: rotations[(rng.next_u32() as usize) % rotations.len()],
                    };
                    for mode in 0..3 {
                        let options = GjkOptions {
                            ax: if mode == 1 { None } else { Some(&ax) },
                            bx: if mode == 0 { None } else { Some(&bx) },
                            use_radius: (iteration & 1) as c_int,
                            outputs: 7,
                        };
                        compare_gjk(
                            &libraries,
                            &shape_a,
                            &shape_b,
                            options,
                            None,
                            &format!("c2GJK transforms mode {mode} pair {kind_a}/{kind_b}"),
                        );
                    }
                }
            }
        }

        for outputs in 0..8 {
            for kind_a in kinds {
                for kind_b in kinds {
                    for _ in 0..16 {
                        let shape_a = random_shape(&mut rng, kind_a);
                        let shape_b = random_shape(&mut rng, kind_b);
                        compare_gjk(
                            &libraries,
                            &shape_a,
                            &shape_b,
                            GjkOptions {
                                outputs,
                                ..identity_options(1)
                            },
                            None,
                            &format!("c2GJK output mask {outputs}"),
                        );
                    }
                }
            }
        }

        for kind_a in kinds {
            for kind_b in kinds {
                for iteration in 0..48 {
                    let shape_a = random_shape(&mut rng, kind_a);
                    let shape_b = random_shape(&mut rng, kind_b);
                    let mut c_cache = Cache::default();
                    let mut r_cache = Cache::default();
                    compare_gjk(
                        &libraries,
                        &shape_a,
                        &shape_b,
                        identity_options(iteration & 1),
                        Some((&mut c_cache, &mut r_cache)),
                        "c2GJK cold cache",
                    );
                    compare_gjk(
                        &libraries,
                        &shape_a,
                        &shape_b,
                        identity_options(iteration & 1),
                        Some((&mut c_cache, &mut r_cache)),
                        "c2GJK warm cache",
                    );
                }
            }
        }

        let large_box = Shape::Aabb(Box2 {
            min: V {
                x: -10_000.0,
                y: -10_000.0,
            },
            max: V {
                x: 10_000.0,
                y: 10_000.0,
            },
        });
        for count in [1, 2, 3] {
            for _ in 0..64 {
                let cache = Cache {
                    metric: rng.positive(),
                    count,
                    i_a: [0, 1, 2],
                    i_b: [1, 2, 3],
                    div: rng.positive(),
                };
                let mut c_cache = cache;
                let mut r_cache = cache;
                compare_gjk(
                    &libraries,
                    &large_box,
                    &large_box,
                    identity_options(0),
                    Some((&mut c_cache, &mut r_cache)),
                    &format!("c2GJK cached simplex count {count}"),
                );
            }
        }

        for _ in 0..64 {
            let cache = Cache {
                metric: 1.0,
                count: 3,
                i_a: [0, 0, 0],
                i_b: [0, 2, 1],
                div: 1.0,
            };
            let mut c_cache = cache;
            let mut r_cache = cache;
            compare_gjk(
                &libraries,
                &large_box,
                &large_box,
                identity_options(0),
                Some((&mut c_cache, &mut r_cache)),
                "c2GJK negative metric cache invalidation",
            );
        }
    }
}

#[test]
fn collision_and_wrapper_surface() {
    unsafe {
        let libraries = libraries();
        let mut rng = Rng::new(0xc011_1510_5eed_7001);
        type BoxBox = unsafe extern "C" fn(Box2, Box2) -> c_int;
        type BoxCapsule = unsafe extern "C" fn(Box2, Capsule) -> c_int;
        type CapsuleCapsule = unsafe extern "C" fn(Capsule, Capsule) -> c_int;
        type CircleCircle = unsafe extern "C" fn(Circle, Circle) -> c_int;
        type CircleBox = unsafe extern "C" fn(Circle, Box2) -> c_int;
        type CircleCapsule = unsafe extern "C" fn(Circle, Capsule) -> c_int;
        type Collided = unsafe extern "C" fn(*const c_void, c_int, *const c_void, c_int) -> c_int;
        type Aabb = unsafe extern "C" fn(c_float, c_float, c_float, c_float) -> c_int;

        let (c_box_box, r_box_box) = symbols::<BoxBox>(&libraries, b"c2AABBtoAABB\0");
        let (c_box_capsule, r_box_capsule) =
            symbols::<BoxCapsule>(&libraries, b"c2AABBtoCapsule\0");
        let (c_capsule_capsule, r_capsule_capsule) =
            symbols::<CapsuleCapsule>(&libraries, b"c2CapsuletoCapsule\0");
        let (c_circle_circle, r_circle_circle) =
            symbols::<CircleCircle>(&libraries, b"c2CircletoCircle\0");
        let (c_circle_box, r_circle_box) = symbols::<CircleBox>(&libraries, b"c2CircletoAABB\0");
        let (c_circle_capsule, r_circle_capsule) =
            symbols::<CircleCapsule>(&libraries, b"c2CircletoCapsule\0");
        let (c_collided, r_collided) = symbols::<Collided>(&libraries, b"c2Collided\0");
        let (c_aabb, r_aabb) = symbols::<Aabb>(&libraries, b"aabb\0");

        let base_box = Box2 {
            min: V { x: -1.0, y: -1.0 },
            max: V { x: 1.0, y: 1.0 },
        };
        for region in 0..5 {
            for _ in 0..128 {
                let gap = rng.positive();
                let extent = rng.positive();
                let other = match region {
                    0 => Box2 {
                        min: V {
                            x: -1.0 - gap - extent,
                            y: -0.5,
                        },
                        max: V {
                            x: -1.0 - gap,
                            y: 0.5,
                        },
                    },
                    1 => Box2 {
                        min: V {
                            x: 1.0 + gap,
                            y: -0.5,
                        },
                        max: V {
                            x: 1.0 + gap + extent,
                            y: 0.5,
                        },
                    },
                    2 => Box2 {
                        min: V {
                            x: -0.5,
                            y: -1.0 - gap - extent,
                        },
                        max: V {
                            x: 0.5,
                            y: -1.0 - gap,
                        },
                    },
                    3 => Box2 {
                        min: V {
                            x: -0.5,
                            y: 1.0 + gap,
                        },
                        max: V {
                            x: 0.5,
                            y: 1.0 + gap + extent,
                        },
                    },
                    _ => Box2 {
                        min: V {
                            x: -rng.positive().min(1.0),
                            y: -rng.positive().min(1.0),
                        },
                        max: V {
                            x: rng.positive().min(1.0),
                            y: rng.positive().min(1.0),
                        },
                    },
                };
                assert_eq!(
                    c_box_box(base_box, other),
                    r_box_box(base_box, other),
                    "c2AABBtoAABB region {region}"
                );
            }
        }
        let touching_box = Box2 {
            min: V { x: 1.0, y: -0.5 },
            max: V { x: 2.0, y: 0.5 },
        };
        assert_eq!(
            c_box_box(base_box, touching_box),
            r_box_box(base_box, touching_box)
        );

        for colliding in [false, true] {
            for _ in 0..128 {
                let offset = rng.positive();
                let capsule = if colliding {
                    Capsule {
                        a: V {
                            x: -2.0,
                            y: rng.f() % 0.5,
                        },
                        b: V {
                            x: 2.0,
                            y: rng.f() % 0.5,
                        },
                        r: rng.positive().min(1.0),
                    }
                } else {
                    Capsule {
                        a: V {
                            x: 5.0 + offset,
                            y: -1.0,
                        },
                        b: V {
                            x: 5.0 + offset,
                            y: 1.0,
                        },
                        r: 0.25,
                    }
                };
                assert_eq!(
                    c_box_capsule(base_box, capsule),
                    r_box_capsule(base_box, capsule),
                    "c2AABBtoCapsule"
                );
            }
        }

        for colliding in [false, true] {
            for _ in 0..128 {
                let first = Capsule {
                    a: V { x: -2.0, y: 0.0 },
                    b: V { x: 2.0, y: 0.0 },
                    r: 0.5,
                };
                let y = if colliding {
                    rng.positive().min(0.9)
                } else {
                    2.0 + rng.positive()
                };
                let second = Capsule {
                    a: V { x: -2.0, y },
                    b: V { x: 2.0, y },
                    r: 0.5,
                };
                assert_eq!(
                    c_capsule_capsule(first, second),
                    r_capsule_capsule(first, second),
                    "c2CapsuletoCapsule"
                );
            }
        }

        for relation in 0..3 {
            for _ in 0..128 {
                let r_a = rng.positive();
                let r_b = rng.positive();
                let sum = r_a + r_b;
                let distance = match relation {
                    0 => sum * 0.5,
                    1 => sum,
                    _ => sum + rng.positive(),
                };
                let a = Circle { p: rng.v(), r: r_a };
                let b = Circle {
                    p: V {
                        x: a.p.x + distance,
                        y: a.p.y,
                    },
                    r: r_b,
                };
                assert_eq!(
                    c_circle_circle(a, b),
                    r_circle_circle(a, b),
                    "c2CircletoCircle relation {relation}"
                );
            }
        }

        for region in 0..5 {
            for _ in 0..128 {
                let radius = if region == 3 {
                    [0.25, 0.5, 1.0, 2.0][(rng.next_u32() as usize) % 4]
                } else {
                    rng.positive().min(2.0)
                };
                let (center, expected_tangent) = match region {
                    0 => (V { x: 0.0, y: 0.0 }, false),
                    1 => (
                        V {
                            x: 1.0 + radius * 0.5,
                            y: 0.0,
                        },
                        false,
                    ),
                    2 => (
                        V {
                            x: 1.0 + radius * 0.25,
                            y: 1.0 + radius * 0.25,
                        },
                        false,
                    ),
                    3 => (
                        V {
                            x: 1.0 + radius,
                            y: 0.0,
                        },
                        true,
                    ),
                    _ => (
                        V {
                            x: 1.0 + radius + rng.positive(),
                            y: 0.0,
                        },
                        false,
                    ),
                };
                let circle = Circle {
                    p: center,
                    r: radius,
                };
                let c_result = c_circle_box(circle, base_box);
                let r_result = r_circle_box(circle, base_box);
                assert_eq!(c_result, r_result, "c2CircletoAABB region {region}");
                if expected_tangent {
                    assert_eq!(c_result, 0, "circle/AABB tangency is strict");
                }
            }
        }

        let horizontal_capsule = Capsule {
            a: V { x: 0.0, y: 0.0 },
            b: V { x: 10.0, y: 0.0 },
            r: 0.75,
        };
        for branch in 0..3 {
            for overlap in [false, true] {
                for _ in 0..128 {
                    let circle_radius = rng.positive().min(1.0);
                    let combined = circle_radius + horizontal_capsule.r;
                    let y = if overlap {
                        combined * 0.25
                    } else {
                        combined + rng.positive()
                    };
                    let x = match branch {
                        0 if overlap => -combined * 0.25,
                        0 => -0.25,
                        1 => 1.0 + (rng.next_u32() % 8) as c_float,
                        _ if overlap => 10.0 + combined * 0.25,
                        _ => 10.25,
                    };
                    let circle = Circle {
                        p: V { x, y },
                        r: circle_radius,
                    };
                    assert_eq!(
                        c_circle_capsule(circle, horizontal_capsule),
                        r_circle_capsule(circle, horizontal_capsule),
                        "c2CircletoCapsule branch {branch} overlap {overlap}"
                    );
                }
            }
        }

        for kind_a in [CIRCLE, AABB, CAPSULE] {
            for kind_b in [CIRCLE, AABB, CAPSULE] {
                for _ in 0..128 {
                    let shape_a = random_shape(&mut rng, kind_a);
                    let shape_b = random_shape(&mut rng, kind_b);
                    assert_eq!(
                        c_collided(shape_a.ptr(), kind_a, shape_b.ptr(), kind_b),
                        r_collided(shape_a.ptr(), kind_a, shape_b.ptr(), kind_b),
                        "c2Collided dispatch {kind_a}/{kind_b}"
                    );
                }
            }
        }

        for _ in 0..512 {
            let min = rng.v();
            let max = V {
                x: min.x + rng.positive(),
                y: min.y + rng.positive(),
            };
            assert_eq!(
                c_aabb(min.x, min.y, max.x, max.y),
                r_aabb(min.x, min.y, max.x, max.y),
                "aabb randomized"
            );
        }
        let wrapper_regions = [
            (-80.0, -10.0, -60.0, 10.0),
            (-35.0, -35.0, -20.0, -20.0),
            (-45.0, 50.0, -25.0, 70.0),
            (200.0, 200.0, 210.0, 210.0),
        ];
        for (min_x, min_y, max_x, max_y) in wrapper_regions {
            for _ in 0..64 {
                let jitter = (rng.next_u32() % 100) as c_float / 10_000.0;
                assert_eq!(
                    c_aabb(min_x + jitter, min_y, max_x + jitter, max_y),
                    r_aabb(min_x + jitter, min_y, max_x + jitter, max_y),
                    "aabb fixed collision region"
                );
            }
        }
    }
}

#[test]
fn error_surface_invalid_enums() {
    unsafe {
        let libraries = libraries();
        type Collided = unsafe extern "C" fn(*const c_void, c_int, *const c_void, c_int) -> c_int;
        let (c_collided, r_collided) = symbols::<Collided>(&libraries, b"c2Collided\0");
        for invalid in [-1, 3, 4, c_int::MAX, c_int::MIN] {
            let c_result = c_collided(std::ptr::null(), invalid, std::ptr::null(), invalid);
            let r_result = r_collided(std::ptr::null(), invalid, std::ptr::null(), invalid);
            assert_eq!(c_result, 0);
            assert_eq!(c_result, r_result, "invalid typeA {invalid}");
            for valid_a in [CIRCLE, AABB, CAPSULE] {
                let c_result = c_collided(std::ptr::null(), valid_a, std::ptr::null(), invalid);
                let r_result = r_collided(std::ptr::null(), valid_a, std::ptr::null(), invalid);
                assert_eq!(c_result, 0);
                assert_eq!(
                    c_result, r_result,
                    "valid typeA {valid_a}, invalid typeB {invalid}"
                );
            }
        }
    }
}

#[test]
fn ieee_and_degenerate_valid_inputs() {
    unsafe {
        let libraries = libraries();
        let values = [
            0.0,
            -0.0,
            1.0,
            -1.0,
            f32::from_bits(1),
            f32::from_bits(0x8000_0001),
            f32::MAX,
            -f32::MAX,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::from_bits(0x7fc0_0123),
            f32::from_bits(0xffc0_0456),
        ];
        let (c_v, r_v) =
            symbols::<unsafe extern "C" fn(c_float, c_float) -> V>(&libraries, b"c2V\0");
        let (c_mulvs, r_mulvs) =
            symbols::<unsafe extern "C" fn(V, c_float) -> V>(&libraries, b"c2Mulvs\0");
        let (c_max, r_max) = symbols::<unsafe extern "C" fn(V, V) -> V>(&libraries, b"c2Maxv\0");
        let (c_min, r_min) = symbols::<unsafe extern "C" fn(V, V) -> V>(&libraries, b"c2Minv\0");
        let (c_clamp, r_clamp) =
            symbols::<unsafe extern "C" fn(V, V, V) -> V>(&libraries, b"c2Clampv\0");
        let (c_sub, r_sub) = symbols::<unsafe extern "C" fn(V, V) -> V>(&libraries, b"c2Sub\0");
        let (c_dot, r_dot) =
            symbols::<unsafe extern "C" fn(V, V) -> c_float>(&libraries, b"c2Dot\0");
        let (c_add, r_add) = symbols::<unsafe extern "C" fn(V, V) -> V>(&libraries, b"c2Add\0");
        let (c_neg, r_neg) = symbols::<unsafe extern "C" fn(V) -> V>(&libraries, b"c2Neg\0");
        let (c_skew, r_skew) = symbols::<unsafe extern "C" fn(V) -> V>(&libraries, b"c2Skew\0");
        let (c_ccw, r_ccw) = symbols::<unsafe extern "C" fn(V) -> V>(&libraries, b"c2CCW90\0");
        let (c_len, r_len) = symbols::<unsafe extern "C" fn(V) -> c_float>(&libraries, b"c2Len\0");
        let (c_det, r_det) =
            symbols::<unsafe extern "C" fn(V, V) -> c_float>(&libraries, b"c2Det2\0");
        let (c_div, r_div) =
            symbols::<unsafe extern "C" fn(V, c_float) -> V>(&libraries, b"c2Div\0");
        let (c_norm, r_norm) = symbols::<unsafe extern "C" fn(V) -> V>(&libraries, b"c2Norm\0");
        let (c_mulrv, r_mulrv) =
            symbols::<unsafe extern "C" fn(R, V) -> V>(&libraries, b"c2Mulrv\0");
        let (c_mulrvt, r_mulrvt) =
            symbols::<unsafe extern "C" fn(R, V) -> V>(&libraries, b"c2MulrvT\0");
        let (c_mulxv, r_mulxv) =
            symbols::<unsafe extern "C" fn(X, V) -> V>(&libraries, b"c2Mulxv\0");

        for (i, &a) in values.iter().enumerate() {
            for (j, &b) in values.iter().enumerate() {
                let left = V { x: a, y: b };
                let right = V {
                    x: values[(i + j + 1) % values.len()],
                    y: values[(i * 3 + j + 2) % values.len()],
                };
                assert_bytes(&c_v(a, b), &r_v(a, b), "special c2V");
                assert_bytes(&c_mulvs(left, b), &r_mulvs(left, b), "special c2Mulvs");
                assert_bytes(&c_max(left, right), &r_max(left, right), "special c2Maxv");
                assert_bytes(&c_min(left, right), &r_min(left, right), "special c2Minv");
                assert_bytes(
                    &c_clamp(left, right, V { x: b, y: a }),
                    &r_clamp(left, right, V { x: b, y: a }),
                    "special c2Clampv",
                );
                assert_bytes(&c_sub(left, right), &r_sub(left, right), "special c2Sub");
                assert_float(c_dot(left, right), r_dot(left, right), "special c2Dot");
                assert_bytes(&c_add(left, right), &r_add(left, right), "special c2Add");
                assert_bytes(&c_neg(left), &r_neg(left), "special c2Neg");
                assert_bytes(&c_skew(left), &r_skew(left), "special c2Skew");
                assert_bytes(&c_ccw(left), &r_ccw(left), "special c2CCW90");
                assert_float(c_len(left), r_len(left), "special c2Len");
                assert_float(c_det(left, right), r_det(left, right), "special c2Det2");
                assert_bytes(&c_div(left, b), &r_div(left, b), "special c2Div");
                assert_bytes(&c_norm(left), &r_norm(left), "special c2Norm");
                let rotation = R { c: a, s: b };
                assert_bytes(
                    &c_mulrv(rotation, right),
                    &r_mulrv(rotation, right),
                    "special c2Mulrv",
                );
                assert_bytes(
                    &c_mulrvt(rotation, right),
                    &r_mulrvt(rotation, right),
                    "special c2MulrvT",
                );
                let transform = X {
                    p: left,
                    r: rotation,
                };
                assert_bytes(
                    &c_mulxv(transform, right),
                    &r_mulxv(transform, right),
                    "special c2Mulxv",
                );
            }
        }

        type BoxBox = unsafe extern "C" fn(Box2, Box2) -> c_int;
        type BoxCapsule = unsafe extern "C" fn(Box2, Capsule) -> c_int;
        type CapsuleCapsule = unsafe extern "C" fn(Capsule, Capsule) -> c_int;
        type CircleCircle = unsafe extern "C" fn(Circle, Circle) -> c_int;
        type CircleBox = unsafe extern "C" fn(Circle, Box2) -> c_int;
        type CircleCapsule = unsafe extern "C" fn(Circle, Capsule) -> c_int;
        type Aabb = unsafe extern "C" fn(c_float, c_float, c_float, c_float) -> c_int;
        let (c_box_box, r_box_box) = symbols::<BoxBox>(&libraries, b"c2AABBtoAABB\0");
        let (c_box_capsule, r_box_capsule) =
            symbols::<BoxCapsule>(&libraries, b"c2AABBtoCapsule\0");
        let (c_capsule_capsule, r_capsule_capsule) =
            symbols::<CapsuleCapsule>(&libraries, b"c2CapsuletoCapsule\0");
        let (c_circle_circle, r_circle_circle) =
            symbols::<CircleCircle>(&libraries, b"c2CircletoCircle\0");
        let (c_circle_box, r_circle_box) = symbols::<CircleBox>(&libraries, b"c2CircletoAABB\0");
        let (c_circle_capsule, r_circle_capsule) =
            symbols::<CircleCapsule>(&libraries, b"c2CircletoCapsule\0");
        let (c_aabb, r_aabb) = symbols::<Aabb>(&libraries, b"aabb\0");

        let boxes = [
            Box2 {
                min: V::default(),
                max: V::default(),
            },
            Box2 {
                min: V { x: 2.0, y: 3.0 },
                max: V { x: -2.0, y: -3.0 },
            },
            Box2 {
                min: V {
                    x: f32::NEG_INFINITY,
                    y: -1.0,
                },
                max: V {
                    x: f32::INFINITY,
                    y: 1.0,
                },
            },
        ];
        let circles = [
            Circle {
                p: V::default(),
                r: 0.0,
            },
            Circle {
                p: V { x: 1.0, y: -1.0 },
                r: -2.0,
            },
            Circle {
                p: V {
                    x: f32::from_bits(0x7fc0_0123),
                    y: f32::INFINITY,
                },
                r: f32::NEG_INFINITY,
            },
        ];
        let capsules = [
            Capsule {
                a: V::default(),
                b: V::default(),
                r: 0.0,
            },
            Capsule {
                a: V { x: 1.0, y: 1.0 },
                b: V { x: 1.0, y: 1.0 },
                r: -3.0,
            },
            Capsule {
                a: V {
                    x: f32::NEG_INFINITY,
                    y: 0.0,
                },
                b: V {
                    x: f32::INFINITY,
                    y: 0.0,
                },
                r: f32::from_bits(0x7fc0_0456),
            },
        ];
        for &box_a in &boxes {
            for &box_b in &boxes {
                assert_eq!(c_box_box(box_a, box_b), r_box_box(box_a, box_b));
            }
            for &circle in &circles {
                assert_eq!(c_circle_box(circle, box_a), r_circle_box(circle, box_a));
            }
            for &capsule in &capsules {
                assert_eq!(c_box_capsule(box_a, capsule), r_box_capsule(box_a, capsule));
            }
        }
        for &circle_a in &circles {
            for &circle_b in &circles {
                assert_eq!(
                    c_circle_circle(circle_a, circle_b),
                    r_circle_circle(circle_a, circle_b)
                );
            }
            for &capsule in &capsules {
                assert_eq!(
                    c_circle_capsule(circle_a, capsule),
                    r_circle_capsule(circle_a, capsule)
                );
            }
        }
        for &capsule_a in &capsules {
            for &capsule_b in &capsules {
                assert_eq!(
                    c_capsule_capsule(capsule_a, capsule_b),
                    r_capsule_capsule(capsule_a, capsule_b)
                );
            }
        }
        for &value in &values {
            assert_eq!(
                c_aabb(value, -value, value, value),
                r_aabb(value, -value, value, value)
            );
        }

        let degenerate_shapes = [
            Shape::Circle(circles[0]),
            Shape::Aabb(boxes[0]),
            Shape::Capsule(capsules[0]),
            Shape::Circle(circles[1]),
            Shape::Aabb(boxes[1]),
            Shape::Capsule(capsules[1]),
        ];
        for shape_a in &degenerate_shapes {
            for shape_b in &degenerate_shapes {
                for use_radius in [0, 1] {
                    compare_gjk(
                        &libraries,
                        shape_a,
                        shape_b,
                        GjkOptions {
                            ax: None,
                            bx: None,
                            use_radius,
                            outputs: 7,
                        },
                        None,
                        "c2GJK degenerate shape",
                    );
                }
            }
        }
    }
}
