use libloading::Library;
use std::ffi::{c_char, c_int, c_void};
use std::mem::size_of;
use std::path::PathBuf;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct V {
    x: f32,
    y: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct R {
    c: f32,
    s: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct X {
    p: V,
    r: R,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Circle {
    p: V,
    r: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Aabb {
    min: V,
    max: V,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Capsule {
    a: V,
    b: V,
    r: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
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

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Sv {
    s_a: V,
    s_b: V,
    p: V,
    u: f32,
    i_a: c_int,
    i_b: c_int,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Simplex {
    a: Sv,
    b: Sv,
    c: Sv,
    d: Sv,
    div: f32,
    count: c_int,
}

const ZERO_V: V = V { x: 0.0, y: 0.0 };
const ZERO_SV: Sv = Sv {
    s_a: ZERO_V,
    s_b: ZERO_V,
    p: ZERO_V,
    u: 0.0,
    i_a: 0,
    i_b: 0,
};

struct Libraries {
    c: Library,
    rust: Library,
}

impl Libraries {
    unsafe fn load() -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_path = root.join("c_src/build/libtranslated_rust.so");
        let rust_path = root.join("target/release/libgjk_lib.so");
        assert!(c_path.is_file(), "missing C library: {}", c_path.display());
        assert!(
            rust_path.is_file(),
            "missing Rust library: {}",
            rust_path.display()
        );
        Self {
            c: unsafe { Library::new(c_path).unwrap() },
            rust: unsafe { Library::new(rust_path).unwrap() },
        }
    }

    unsafe fn pair<F: Copy>(&self, name: &[u8]) -> (F, F) {
        let c = unsafe { *self.c.get::<F>(name).unwrap() };
        let rust = unsafe { *self.rust.get::<F>(name).unwrap() };
        (c, rust)
    }
}

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

    fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        lo + (self.next_u32() % (hi - lo) as u32) as i32
    }

    fn finite(&mut self) -> f32 {
        self.range_i32(-4096, 4097) as f32 / 32.0
    }

    fn positive(&mut self) -> f32 {
        self.range_i32(1, 1025) as f32 / 32.0
    }

    fn vector(&mut self) -> V {
        V {
            x: self.finite(),
            y: self.finite(),
        }
    }
}

fn simplex() -> Simplex {
    Simplex {
        a: ZERO_SV,
        b: ZERO_SV,
        c: ZERO_SV,
        d: ZERO_SV,
        div: 0.0,
        count: 0,
    }
}

fn cache() -> Cache {
    Cache {
        metric: 0.0,
        count: 0,
        i_a: [0; 3],
        i_b: [0; 3],
        div: 0.0,
    }
}

fn proxy_sentinel() -> Proxy {
    Proxy {
        radius: f32::from_bits(0x4123_4567),
        count: 0x1357_2468,
        verts: [V {
            x: f32::from_bits(0x4211_2233),
            y: f32::from_bits(0xc244_5566),
        }; 8],
    }
}

fn assert_f32_eq(c: f32, rust: f32, context: &str) {
    assert_eq!(
        c.to_bits(),
        rust.to_bits(),
        "{context}: C={c:?} ({:#010x}), Rust={rust:?} ({:#010x})",
        c.to_bits(),
        rust.to_bits()
    );
}

fn assert_bytes_eq<T>(c: &T, rust: &T, context: &str) {
    let c = unsafe { std::slice::from_raw_parts((c as *const T).cast::<u8>(), size_of::<T>()) };
    let rust =
        unsafe { std::slice::from_raw_parts((rust as *const T).cast::<u8>(), size_of::<T>()) };
    assert_eq!(c, rust, "{context}");
}

type V2V = unsafe extern "C" fn(V) -> V;
type VV2V = unsafe extern "C" fn(V, V) -> V;
type VV2F = unsafe extern "C" fn(V, V) -> f32;
type VF2V = unsafe extern "C" fn(V, f32) -> V;

#[test]
fn differential_foundations_and_proxy_rows_1_through_21() {
    unsafe {
        let libs = Libraries::load();
        let (c_v, r_v) = libs.pair::<unsafe extern "C" fn(f32, f32) -> V>(b"c2V\0");
        let (c_mulvs, r_mulvs) = libs.pair::<VF2V>(b"c2Mulvs\0");
        let (c_max, r_max) = libs.pair::<VV2V>(b"c2Maxv\0");
        let (c_min, r_min) = libs.pair::<VV2V>(b"c2Minv\0");
        let (c_clamp, r_clamp) =
            libs.pair::<unsafe extern "C" fn(V, V, V) -> V>(b"c2Clampv\0");
        let (c_sub, r_sub) = libs.pair::<VV2V>(b"c2Sub\0");
        let (c_dot, r_dot) = libs.pair::<VV2F>(b"c2Dot\0");
        let (c_rot_id, r_rot_id) =
            libs.pair::<unsafe extern "C" fn() -> R>(b"c2RotIdentity\0");
        let (c_x_id, r_x_id) = libs.pair::<unsafe extern "C" fn() -> X>(b"c2xIdentity\0");
        let (c_bb, r_bb) =
            libs.pair::<unsafe extern "C" fn(*mut V, *mut Aabb)>(b"c2BBVerts\0");
        let (c_proxy, r_proxy) = libs
            .pair::<unsafe extern "C" fn(*const c_void, c_int, *mut Proxy)>(b"c2MakeProxy\0");
        let (c_len, r_len) = libs.pair::<unsafe extern "C" fn(V) -> f32>(b"c2Len\0");
        let (c_det, r_det) = libs.pair::<VV2F>(b"c2Det2\0");
        let (c_metric, r_metric) = libs
            .pair::<unsafe extern "C" fn(*mut Simplex) -> f32>(b"c2GJKSimplexMetric\0");
        let (c_mulrv, r_mulrv) =
            libs.pair::<unsafe extern "C" fn(R, V) -> V>(b"c2Mulrv\0");
        let (c_add, r_add) = libs.pair::<VV2V>(b"c2Add\0");
        let (c_mulxv, r_mulxv) =
            libs.pair::<unsafe extern "C" fn(X, V) -> V>(b"c2Mulxv\0");

        let mut rng = Rng::new(0x8a5c_19d3_c764_291e);
        for case in 0..512 {
            let a = rng.vector();
            let b = rng.vector();
            let scale = if case % 11 == 0 { 0.0 } else { rng.finite() };
            assert_bytes_eq(&c_v(a.x, a.y), &r_v(a.x, a.y), "c2V");
            assert_bytes_eq(&c_mulvs(a, scale), &r_mulvs(a, scale), "c2Mulvs");
            assert_bytes_eq(&c_max(a, b), &r_max(a, b), "c2Maxv");
            assert_bytes_eq(&c_min(a, b), &r_min(a, b), "c2Minv");
            let lo = V {
                x: a.x.min(b.x),
                y: a.y.min(b.y),
            };
            let hi = V {
                x: a.x.max(b.x),
                y: a.y.max(b.y),
            };
            let clamp_input = rng.vector();
            assert_bytes_eq(
                &c_clamp(clamp_input, lo, hi),
                &r_clamp(clamp_input, lo, hi),
                "c2Clampv",
            );
            assert_bytes_eq(&c_sub(a, b), &r_sub(a, b), "c2Sub");
            assert_f32_eq(c_dot(a, b), r_dot(a, b), "c2Dot");
            assert_f32_eq(c_len(a), r_len(a), "c2Len");
            assert_f32_eq(c_det(a, b), r_det(a, b), "c2Det2");
            assert_bytes_eq(&c_add(a, b), &r_add(a, b), "c2Add");

            let rot = R {
                c: rng.finite(),
                s: rng.finite(),
            };
            assert_bytes_eq(&c_mulrv(rot, a), &r_mulrv(rot, a), "c2Mulrv");
            let transform = X {
                p: rng.vector(),
                r: rot,
            };
            assert_bytes_eq(&c_mulxv(transform, a), &r_mulxv(transform, a), "c2Mulxv");

            let mut c_box = Aabb { min: lo, max: hi };
            let mut r_box = c_box;
            let mut c_out = [ZERO_V; 4];
            let mut r_out = [ZERO_V; 4];
            c_bb(c_out.as_mut_ptr(), &mut c_box);
            r_bb(r_out.as_mut_ptr(), &mut r_box);
            assert_bytes_eq(&c_out, &r_out, "c2BBVerts");

            let circle = Circle {
                p: a,
                r: rng.positive(),
            };
            let capsule = Capsule {
                a,
                b,
                r: rng.positive(),
            };
            for (shape, shape_type) in [
                ((&circle as *const Circle).cast::<c_void>(), 0),
                ((&c_box as *const Aabb).cast::<c_void>(), 1),
                ((&capsule as *const Capsule).cast::<c_void>(), 2),
            ] {
                let mut cp = proxy_sentinel();
                let mut rp = cp;
                c_proxy(shape, shape_type, &mut cp);
                r_proxy(shape, shape_type, &mut rp);
                assert_bytes_eq(&cp, &rp, "c2MakeProxy valid type");
            }

            let mut s = simplex();
            s.a.p = a;
            s.b.p = b;
            s.c.p = rng.vector();
            for count in 1..=3 {
                let mut cs = s;
                let mut rs = s;
                cs.count = count;
                rs.count = count;
                assert_f32_eq(c_metric(&mut cs), r_metric(&mut rs), "c2GJKSimplexMetric");
            }
        }

        assert_bytes_eq(&c_rot_id(), &r_rot_id(), "c2RotIdentity");
        assert_bytes_eq(&c_x_id(), &r_x_id(), "c2xIdentity");

        for invalid_type in [-2, -1, 3, 4, i32::MAX] {
            let mut cp = proxy_sentinel();
            let mut rp = cp;
            c_proxy(std::ptr::null(), invalid_type, &mut cp);
            r_proxy(std::ptr::null(), invalid_type, &mut rp);
            assert_bytes_eq(&cp, &rp, "c2MakeProxy invalid enum boundary");
            assert_bytes_eq(&cp, &proxy_sentinel(), "C invalid enum leaves proxy unchanged");
        }
    }
}

fn c22_region(a: V, b: V) -> usize {
    let u = b.x * (b.x - a.x) + b.y * (b.y - a.y);
    let v = a.x * (a.x - b.x) + a.y * (a.y - b.y);
    if v <= 0.0 {
        0
    } else if u <= 0.0 {
        1
    } else {
        2
    }
}

fn det(a: V, b: V) -> f32 {
    a.x * b.y - a.y * b.x
}

fn sub(a: V, b: V) -> V {
    V {
        x: a.x - b.x,
        y: a.y - b.y,
    }
}

fn c23_region(a: V, b: V, c: V) -> usize {
    let u_ab = b.x * (b.x - a.x) + b.y * (b.y - a.y);
    let v_ab = a.x * (a.x - b.x) + a.y * (a.y - b.y);
    let u_bc = c.x * (c.x - b.x) + c.y * (c.y - b.y);
    let v_bc = b.x * (b.x - c.x) + b.y * (b.y - c.y);
    let u_ca = a.x * (a.x - c.x) + a.y * (a.y - c.y);
    let v_ca = c.x * (c.x - a.x) + c.y * (c.y - a.y);
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
fn differential_simplex_direction_support_and_boundary_rows_22_through_48() {
    unsafe {
        let libs = Libraries::load();
        let (c_22, r_22) = libs.pair::<unsafe extern "C" fn(*mut Simplex)>(b"c22\0");
        let (c_23, r_23) = libs.pair::<unsafe extern "C" fn(*mut Simplex)>(b"c23\0");
        let (c_neg, r_neg) = libs.pair::<V2V>(b"c2Neg\0");
        let (c_skew, r_skew) = libs.pair::<V2V>(b"c2Skew\0");
        let (c_ccw, r_ccw) = libs.pair::<V2V>(b"c2CCW90\0");
        let (c_d, r_d) = libs.pair::<unsafe extern "C" fn(*mut Simplex) -> V>(b"c2D\0");
        let (c_support, r_support) =
            libs.pair::<unsafe extern "C" fn(*const V, c_int, V) -> c_int>(b"c2Support\0");
        let (c_witness, r_witness) = libs
            .pair::<unsafe extern "C" fn(*mut Simplex, *mut V, *mut V)>(b"c2Witness\0");
        let (c_div, r_div) = libs.pair::<VF2V>(b"c2Div\0");
        let (c_norm, r_norm) = libs.pair::<V2V>(b"c2Norm\0");
        let (c_l, r_l) = libs.pair::<unsafe extern "C" fn(*mut Simplex) -> V>(b"c2L\0");
        let (c_mulrvt, r_mulrvt) =
            libs.pair::<unsafe extern "C" fn(R, V) -> V>(b"c2MulrvT\0");
        let (c_metric, r_metric) = libs
            .pair::<unsafe extern "C" fn(*mut Simplex) -> f32>(b"c2GJKSimplexMetric\0");

        let mut rng = Rng::new(0x49ef_2187_a661_0d5b);
        let mut c22_seen = [0usize; 3];
        let mut c23_seen = [0usize; 7];
        for _ in 0..200_000 {
            let a = rng.vector();
            let b = rng.vector();
            let c = rng.vector();

            let r22_index = c22_region(a, b);
            if c22_seen[r22_index] < 128 {
                let mut cs = simplex();
                cs.a.p = a;
                cs.b.p = b;
                cs.count = 2;
                let mut rs = cs;
                c_22(&mut cs);
                r_22(&mut rs);
                assert_bytes_eq(&cs, &rs, "c22 branch");
                c22_seen[r22_index] += 1;
            }

            let r23_index = c23_region(a, b, c);
            if c23_seen[r23_index] < 128 {
                let mut cs = simplex();
                cs.a.p = a;
                cs.b.p = b;
                cs.c.p = c;
                cs.count = 3;
                let mut rs = cs;
                c_23(&mut cs);
                r_23(&mut rs);
                assert_bytes_eq(&cs, &rs, "c23 branch");
                c23_seen[r23_index] += 1;
            }
            if c22_seen.iter().all(|&n| n == 128) && c23_seen.iter().all(|&n| n == 128) {
                break;
            }
        }
        assert_eq!(c22_seen, [128; 3], "failed to generate every c22 region");
        assert_eq!(c23_seen, [128; 7], "failed to generate every c23 region");

        for case in 0..512 {
            let a = rng.vector();
            assert_bytes_eq(&c_neg(a), &r_neg(a), "c2Neg");
            assert_bytes_eq(&c_skew(a), &r_skew(a), "c2Skew");
            assert_bytes_eq(&c_ccw(a), &r_ccw(a), "c2CCW90");
            let rot = R {
                c: rng.finite(),
                s: rng.finite(),
            };
            assert_bytes_eq(&c_mulrvt(rot, a), &r_mulrvt(rot, a), "c2MulrvT");

            let mut base = simplex();
            base.a.p = a;
            base.b.p = rng.vector();
            for count in [1, 2, 3] {
                let mut cs = base;
                let mut rs = base;
                cs.count = count;
                rs.count = count;
                assert_bytes_eq(&c_d(&mut cs), &r_d(&mut rs), "c2D");
            }

            let mut verts = [ZERO_V; 8];
            for v in &mut verts {
                *v = rng.vector();
            }
            let direction = rng.vector();
            for count in [1, 2, 8] {
                assert_eq!(
                    c_support(verts.as_ptr(), count, direction),
                    r_support(verts.as_ptr(), count, direction),
                    "c2Support"
                );
            }
            let tied = [V { x: 1.0, y: 0.0 }; 4];
            assert_eq!(c_support(tied.as_ptr(), 4, tied[0]), 0);
            assert_eq!(r_support(tied.as_ptr(), 4, tied[0]), 0);

            let mut weighted = simplex();
            weighted.a = Sv {
                s_a: rng.vector(),
                s_b: rng.vector(),
                p: rng.vector(),
                u: rng.positive(),
                i_a: 0,
                i_b: 1,
            };
            weighted.b = Sv {
                s_a: rng.vector(),
                s_b: rng.vector(),
                p: rng.vector(),
                u: rng.positive(),
                i_a: 1,
                i_b: 2,
            };
            weighted.c = Sv {
                s_a: rng.vector(),
                s_b: rng.vector(),
                p: rng.vector(),
                u: rng.positive(),
                i_a: 2,
                i_b: 0,
            };
            weighted.div = rng.positive();
            for count in [1, 2, 3] {
                let mut cs = weighted;
                let mut rs = weighted;
                cs.count = count;
                rs.count = count;
                let mut ca = ZERO_V;
                let mut cb = ZERO_V;
                let mut ra = ZERO_V;
                let mut rb = ZERO_V;
                c_witness(&mut cs, &mut ca, &mut cb);
                r_witness(&mut rs, &mut ra, &mut rb);
                assert_bytes_eq(&ca, &ra, "c2Witness A");
                assert_bytes_eq(&cb, &rb, "c2Witness B");
                assert_bytes_eq(&c_l(&mut cs), &r_l(&mut rs), "c2L");
            }

            let divisor = if case % 2 == 0 {
                rng.positive()
            } else {
                -rng.positive()
            };
            assert_bytes_eq(&c_div(a, divisor), &r_div(a, divisor), "c2Div");
            if a.x != 0.0 || a.y != 0.0 {
                assert_bytes_eq(&c_norm(a), &r_norm(a), "c2Norm");
            }
        }

        for invalid_count in [0, -1, 4, i32::MAX] {
            let mut cs = simplex();
            let mut rs = cs;
            cs.count = invalid_count;
            rs.count = invalid_count;
            assert_f32_eq(c_metric(&mut cs), r_metric(&mut rs), "metric invalid count");
            assert_bytes_eq(&c_d(&mut cs), &r_d(&mut rs), "direction invalid count");
            assert_bytes_eq(&c_l(&mut cs), &r_l(&mut rs), "location invalid count");
            let mut ca = V { x: 1.0, y: 2.0 };
            let mut cb = V { x: 3.0, y: 4.0 };
            let mut ra = ca;
            let mut rb = cb;
            c_witness(&mut cs, &mut ca, &mut cb);
            r_witness(&mut rs, &mut ra, &mut rb);
            assert_bytes_eq(&ca, &ra, "witness invalid count A");
            assert_bytes_eq(&cb, &rb, "witness invalid count B");
        }

        let one = [V { x: 7.0, y: -9.0 }];
        for count in [0, -1, i32::MIN] {
            assert_eq!(c_support(one.as_ptr(), count, one[0]), 0);
            assert_eq!(r_support(one.as_ptr(), count, one[0]), 0);
        }
        for zero in [0.0, -0.0] {
            for v in [V { x: 1.0, y: -1.0 }, ZERO_V] {
                assert_bytes_eq(&c_div(v, zero), &r_div(v, zero), "division by zero");
            }
        }
        assert_bytes_eq(&c_norm(ZERO_V), &r_norm(ZERO_V), "normalize zero");
    }
}

#[derive(Clone, Copy)]
enum Shape {
    Circle(Circle),
    Aabb(Aabb),
    Capsule(Capsule),
}

impl Shape {
    fn ptr(&self) -> *const c_void {
        match self {
            Self::Circle(v) => (v as *const Circle).cast(),
            Self::Aabb(v) => (v as *const Aabb).cast(),
            Self::Capsule(v) => (v as *const Capsule).cast(),
        }
    }
}

fn make_shape(kind: c_int, center_x: f32, scale: f32) -> (Shape, f32) {
    match kind {
        0 => {
            let radius = scale;
            (
                Shape::Circle(Circle {
                    p: V {
                        x: center_x,
                        y: 0.0,
                    },
                    r: radius,
                }),
                radius,
            )
        }
        1 => {
            let half = scale;
            (
                Shape::Aabb(Aabb {
                    min: V {
                        x: center_x - half,
                        y: -half,
                    },
                    max: V {
                        x: center_x + half,
                        y: half,
                    },
                }),
                half,
            )
        }
        2 => {
            let half_segment = scale;
            let radius = scale * 0.5;
            (
                Shape::Capsule(Capsule {
                    a: V {
                        x: center_x - half_segment,
                        y: 0.0,
                    },
                    b: V {
                        x: center_x + half_segment,
                        y: 0.0,
                    },
                    r: radius,
                }),
                half_segment + radius,
            )
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
) -> f32;

#[allow(clippy::too_many_arguments)]
unsafe fn compare_gjk_call(
    c_gjk: Gjk,
    r_gjk: Gjk,
    shape_a: &Shape,
    type_a: c_int,
    ax: *const X,
    shape_b: &Shape,
    type_b: c_int,
    bx: *const X,
    use_radius: c_int,
    output_mask: u8,
    c_cache: *mut Cache,
    r_cache: *mut Cache,
    context: &str,
) -> (f32, Cache, Cache) {
    let sentinel = V {
        x: f32::from_bits(0x4212_3456),
        y: f32::from_bits(0xc265_4321),
    };
    let mut ca = sentinel;
    let mut cb = sentinel;
    let mut ra = sentinel;
    let mut rb = sentinel;
    let mut ci = 0x1234_5678;
    let mut ri = 0x1234_5678;
    let ca_ptr = if output_mask & 1 != 0 {
        &mut ca
    } else {
        std::ptr::null_mut()
    };
    let ra_ptr = if output_mask & 1 != 0 {
        &mut ra
    } else {
        std::ptr::null_mut()
    };
    let cb_ptr = if output_mask & 2 != 0 {
        &mut cb
    } else {
        std::ptr::null_mut()
    };
    let rb_ptr = if output_mask & 2 != 0 {
        &mut rb
    } else {
        std::ptr::null_mut()
    };
    let ci_ptr = if output_mask & 4 != 0 {
        &mut ci
    } else {
        std::ptr::null_mut()
    };
    let ri_ptr = if output_mask & 4 != 0 {
        &mut ri
    } else {
        std::ptr::null_mut()
    };

    let cd = unsafe {
        c_gjk(
            shape_a.ptr(),
            type_a,
            ax,
            shape_b.ptr(),
            type_b,
            bx,
            ca_ptr,
            cb_ptr,
            use_radius,
            ci_ptr,
            c_cache,
        )
    };
    let rd = unsafe {
        r_gjk(
            shape_a.ptr(),
            type_a,
            ax,
            shape_b.ptr(),
            type_b,
            bx,
            ra_ptr,
            rb_ptr,
            use_radius,
            ri_ptr,
            r_cache,
        )
    };
    assert_f32_eq(cd, rd, context);
    assert_bytes_eq(&ca, &ra, context);
    assert_bytes_eq(&cb, &rb, context);
    assert_eq!(ci, ri, "{context}: iteration output");
    let cc = if c_cache.is_null() {
        cache()
    } else {
        unsafe { *c_cache }
    };
    let rc = if r_cache.is_null() {
        cache()
    } else {
        unsafe { *r_cache }
    };
    if !c_cache.is_null() {
        assert_bytes_eq(&cc, &rc, context);
    }
    (cd, cc, rc)
}

#[test]
fn differential_gjk_full_cross_product_rows_49_through_60() {
    unsafe {
        let libs = Libraries::load();
        let (c_gjk, r_gjk) = libs.pair::<Gjk>(b"c2GJK\0");
        let mut rng = Rng::new(0xf012_52a7_9b83_c46d);
        let identity = X {
            p: ZERO_V,
            r: R { c: 1.0, s: 0.0 },
        };
        let mut seen_counts = [false; 4];

        for type_a in 0..=2 {
            for type_b in 0..=2 {
                for geometry in 0..3 {
                    for sample in 0..4 {
                        let scale_a = rng.range_i32(8, 65) as f32 / 8.0;
                        let scale_b = rng.range_i32(8, 65) as f32 / 8.0;
                        let (shape_a, reach_a) = make_shape(type_a, 0.0, scale_a);
                        let (_, reach_b) = make_shape(type_b, 0.0, scale_b);
                        let center_b = match geometry {
                            0 => 0.0,
                            1 => reach_a + reach_b,
                            _ => reach_a + reach_b + 1.0 + sample as f32 / 8.0,
                        };
                        let (shape_b, _) = make_shape(type_b, center_b, scale_b);

                        for transform_mask in 0..4 {
                            let ax = if transform_mask & 1 == 0 {
                                std::ptr::null()
                            } else {
                                &identity
                            };
                            let bx = if transform_mask & 2 == 0 {
                                std::ptr::null()
                            } else {
                                &identity
                            };
                            for use_radius in [0, 1, -7] {
                                for output_mask in 0..8 {
                                    for cache_mode in 0..3 {
                                        let mut cc = cache();
                                        let mut rc = cache();
                                        if cache_mode == 2 {
                                            compare_gjk_call(
                                                c_gjk,
                                                r_gjk,
                                                &shape_a,
                                                type_a,
                                                ax,
                                                &shape_b,
                                                type_b,
                                                bx,
                                                use_radius,
                                                7,
                                                &mut cc,
                                                &mut rc,
                                                "c2GJK warmup",
                                            );
                                            if (0..=3).contains(&cc.count) {
                                                seen_counts[cc.count as usize] = true;
                                            }
                                        }
                                        let (cc_ptr, rc_ptr) = if cache_mode == 0 {
                                            (std::ptr::null_mut(), std::ptr::null_mut())
                                        } else {
                                            (
                                                &mut cc as *mut Cache,
                                                &mut rc as *mut Cache,
                                            )
                                        };
                                        compare_gjk_call(
                                            c_gjk,
                                            r_gjk,
                                            &shape_a,
                                            type_a,
                                            ax,
                                            &shape_b,
                                            type_b,
                                            bx,
                                            use_radius,
                                            output_mask,
                                            cc_ptr,
                                            rc_ptr,
                                            "c2GJK cross product",
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        assert!(seen_counts[1], "no one-vertex cache was generated");
        assert!(seen_counts[2], "no two-vertex cache was generated");
        assert!(seen_counts[3], "no three-vertex cache was generated");

        let large_a = Shape::Aabb(Aabb {
            min: V {
                x: -10_000.0,
                y: -10_000.0,
            },
            max: V {
                x: 10_000.0,
                y: 10_000.0,
            },
        });
        let large_b = large_a;
        let mut rejected_c = Cache {
            metric: 1.0,
            count: 3,
            i_a: [0, 2, 1],
            i_b: [2, 0, 3],
            div: 1.0,
        };
        let mut rejected_r = rejected_c;
        compare_gjk_call(
            c_gjk,
            r_gjk,
            &large_a,
            1,
            std::ptr::null(),
            &large_b,
            1,
            std::ptr::null(),
            1,
            7,
            &mut rejected_c,
            &mut rejected_r,
            "c2GJK rejected negative-metric cache",
        );

        let transformed_a = Shape::Capsule(Capsule {
            a: V { x: -2.0, y: 0.0 },
            b: V { x: 2.0, y: 0.0 },
            r: 0.75,
        });
        let transformed_b = Shape::Circle(Circle {
            p: V { x: 1.0, y: 1.0 },
            r: 0.5,
        });
        for _ in 0..256 {
            let ax = X {
                p: rng.vector(),
                r: R {
                    c: rng.finite(),
                    s: rng.finite(),
                },
            };
            let bx = X {
                p: rng.vector(),
                r: R {
                    c: rng.finite(),
                    s: rng.finite(),
                },
            };
            compare_gjk_call(
                c_gjk,
                r_gjk,
                &transformed_a,
                2,
                &ax,
                &transformed_b,
                0,
                &bx,
                1,
                7,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                "c2GJK nonidentity transforms",
            );
        }
    }
}

type GjkWrapper = unsafe extern "C" fn(
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

#[test]
fn differential_public_wrapper_rows_61_and_62() {
    unsafe {
        let libs = Libraries::load();
        let (c_gjk, r_gjk) = libs.pair::<GjkWrapper>(b"gjk\0");
        let mut rng = Rng::new(0x711d_b8a2_0cc9_356f);
        for reverse in [0, 1, -1, 2, 127] {
            for _ in 0..1024 {
                let min = rng.vector();
                let extent = V {
                    x: rng.positive(),
                    y: rng.positive(),
                };
                let max = V {
                    x: min.x + extent.x,
                    y: min.y + extent.y,
                };
                let cap_a = rng.vector();
                let cap_b = rng.vector();
                let radius = rng.positive();
                let mut ca = ZERO_V;
                let mut cb = ZERO_V;
                let mut ra = ZERO_V;
                let mut rb = ZERO_V;
                c_gjk(
                    reverse,
                    &mut ca,
                    &mut cb,
                    min.x,
                    min.y,
                    max.x,
                    max.y,
                    cap_a.x,
                    cap_a.y,
                    cap_b.x,
                    cap_b.y,
                    radius,
                );
                r_gjk(
                    reverse,
                    &mut ra,
                    &mut rb,
                    min.x,
                    min.y,
                    max.x,
                    max.y,
                    cap_a.x,
                    cap_a.y,
                    cap_b.x,
                    cap_b.y,
                    radius,
                );
                assert_bytes_eq(&ca, &ra, "gjk output A");
                assert_bytes_eq(&cb, &rb, "gjk output B");
            }
        }
    }
}
