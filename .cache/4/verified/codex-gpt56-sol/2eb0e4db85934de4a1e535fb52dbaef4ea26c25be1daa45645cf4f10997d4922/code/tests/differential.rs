#![allow(non_snake_case)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_float, c_int, c_void};
use std::path::PathBuf;

const CIRCLE: c_int = 0;
const AABB: c_int = 1;
const CAPSULE: c_int = 2;
const CASES: usize = 128;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct c2v {
    x: c_float,
    y: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct c2r {
    c: c_float,
    s: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct c2x {
    p: c2v,
    r: c2r,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct c2Circle {
    p: c2v,
    r: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct c2AABB {
    min: c2v,
    max: c2v,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct c2Capsule {
    a: c2v,
    b: c2v,
    r: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct c2GJKCache {
    metric: c_float,
    count: c_int,
    iA: [c_int; 3],
    iB: [c_int; 3],
    div: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct c2Proxy {
    radius: c_float,
    count: c_int,
    verts: [c2v; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct c2sv {
    sA: c2v,
    sB: c2v,
    p: c2v,
    u: c_float,
    iA: c_int,
    iB: c_int,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct c2Simplex {
    verts: [c2sv; 4],
    div: c_float,
    count: c_int,
}

struct Libraries {
    c: Library,
    rust: Library,
}

impl Libraries {
    fn load() -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_path = root.join("c_src/build/libtranslated_rust.so");
        let rust_path = std::env::var_os("RUST_DYLIB_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|| root.join("target/release/libgjk_cache_lib.so"));
        assert!(c_path.is_file(), "missing C library: {}", c_path.display());
        assert!(
            rust_path.is_file(),
            "missing Rust library: {}",
            rust_path.display()
        );
        unsafe {
            Self {
                c: Library::new(c_path).expect("load C library"),
                rust: Library::new(rust_path).expect("load Rust library"),
            }
        }
    }

    unsafe fn symbols<T>(&self, name: &[u8]) -> (Symbol<'_, T>, Symbol<'_, T>) {
        unsafe {
            (
                self.c.get(name).expect("C symbol"),
                self.rust.get(name).expect("Rust symbol"),
            )
        }
    }
}

#[derive(Clone, Copy)]
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

    fn finite(&mut self) -> f32 {
        let sign = if self.u32() & 1 == 0 { 1.0 } else { -1.0 };
        sign * (self.u32() % 200_001) as f32 / 1000.0
    }

    fn positive(&mut self) -> f32 {
        0.25 + (self.u32() % 10_000) as f32 / 1000.0
    }

    fn vec(&mut self) -> c2v {
        c2v {
            x: self.finite(),
            y: self.finite(),
        }
    }
}

fn bytes<T>(value: &T) -> &[u8] {
    unsafe { std::slice::from_raw_parts(value as *const T as *const u8, std::mem::size_of::<T>()) }
}

fn assert_bytes<T>(c: &T, rust: &T, context: &str) {
    assert_eq!(
        bytes(c),
        bytes(rust),
        "{context}: C={:?}, Rust={:?}",
        bytes(c),
        bytes(rust)
    );
}

fn assert_float(c: f32, rust: f32, context: &str) {
    assert_eq!(
        c.to_bits(),
        rust.to_bits(),
        "{context}: C={c:?} ({:#010x}), Rust={rust:?} ({:#010x})",
        c.to_bits(),
        rust.to_bits()
    );
}

fn seeded_sv(rng: &mut Rng) -> c2sv {
    c2sv {
        sA: rng.vec(),
        sB: rng.vec(),
        p: rng.vec(),
        u: rng.finite(),
        iA: (rng.u32() % 8) as i32,
        iB: (rng.u32() % 8) as i32,
    }
}

fn simplex_with_points(rng: &mut Rng, points: [c2v; 3], count: i32) -> c2Simplex {
    let mut simplex = c2Simplex {
        verts: [
            seeded_sv(rng),
            seeded_sv(rng),
            seeded_sv(rng),
            seeded_sv(rng),
        ],
        div: rng.positive(),
        count,
    };
    for (vertex, point) in simplex.verts.iter_mut().zip(points) {
        vertex.p = point;
    }
    simplex
}

#[test]
fn vector_scalar_and_transform_exports_match() {
    let libs = Libraries::load();
    unsafe {
        type V = unsafe extern "C" fn(f32, f32) -> c2v;
        type VS = unsafe extern "C" fn(c2v, f32) -> c2v;
        type VV = unsafe extern "C" fn(c2v, c2v) -> c2v;
        type VVV = unsafe extern "C" fn(c2v, c2v, c2v) -> c2v;
        type VF = unsafe extern "C" fn(c2v) -> f32;
        type VVF = unsafe extern "C" fn(c2v, c2v) -> f32;
        type RV = unsafe extern "C" fn(c2r, c2v) -> c2v;
        type XV = unsafe extern "C" fn(c2x, c2v) -> c2v;
        type R0 = unsafe extern "C" fn() -> c2r;
        type X0 = unsafe extern "C" fn() -> c2x;

        let (c_v, r_v) = libs.symbols::<V>(b"c2V\0");
        let (c_mulvs, r_mulvs) = libs.symbols::<VS>(b"c2Mulvs\0");
        let (c_max, r_max) = libs.symbols::<VV>(b"c2Maxv\0");
        let (c_min, r_min) = libs.symbols::<VV>(b"c2Minv\0");
        let (c_clamp, r_clamp) = libs.symbols::<VVV>(b"c2Clampv\0");
        let (c_sub, r_sub) = libs.symbols::<VV>(b"c2Sub\0");
        let (c_dot, r_dot) = libs.symbols::<VVF>(b"c2Dot\0");
        let (c_len, r_len) = libs.symbols::<VF>(b"c2Len\0");
        let (c_det, r_det) = libs.symbols::<VVF>(b"c2Det2\0");
        let (c_mulrv, r_mulrv) = libs.symbols::<RV>(b"c2Mulrv\0");
        let (c_add, r_add) = libs.symbols::<VV>(b"c2Add\0");
        let (c_mulxv, r_mulxv) = libs.symbols::<XV>(b"c2Mulxv\0");
        let (c_neg, r_neg) = libs.symbols::<unsafe extern "C" fn(c2v) -> c2v>(b"c2Neg\0");
        let (c_skew, r_skew) = libs.symbols::<unsafe extern "C" fn(c2v) -> c2v>(b"c2Skew\0");
        let (c_ccw, r_ccw) = libs.symbols::<unsafe extern "C" fn(c2v) -> c2v>(b"c2CCW90\0");
        let (c_div, r_div) = libs.symbols::<VS>(b"c2Div\0");
        let (c_norm, r_norm) = libs.symbols::<unsafe extern "C" fn(c2v) -> c2v>(b"c2Norm\0");
        let (c_mulrvt, r_mulrvt) = libs.symbols::<RV>(b"c2MulrvT\0");
        let (c_rot_id, r_rot_id) = libs.symbols::<R0>(b"c2RotIdentity\0");
        let (c_x_id, r_x_id) = libs.symbols::<X0>(b"c2xIdentity\0");

        assert_bytes(&c_rot_id(), &r_rot_id(), "c2RotIdentity");
        assert_bytes(&c_x_id(), &r_x_id(), "c2xIdentity");

        let specials = [
            0.0,
            -0.0,
            1.0,
            -1.0,
            f32::from_bits(1),
            f32::from_bits(0x8000_0001),
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::from_bits(0x7fc0_1234),
            f32::from_bits(0xffc0_5678),
        ];
        for &x in &specials {
            for &y in &specials {
                let a = c2v { x, y };
                let b = c2v { x: y, y: x };
                let lo = c2v { x: -1.0, y: 1.0 };
                let hi = c2v { x: 1.0, y: -1.0 };
                assert_bytes(&c_v(x, y), &r_v(x, y), "c2V special");
                assert_bytes(&c_mulvs(a, y), &r_mulvs(a, y), "c2Mulvs special");
                assert_bytes(&c_max(a, b), &r_max(a, b), "c2Maxv special");
                assert_bytes(&c_min(a, b), &r_min(a, b), "c2Minv special");
                assert_bytes(&c_clamp(a, lo, hi), &r_clamp(a, lo, hi), "c2Clampv special");
                assert_bytes(&c_sub(a, b), &r_sub(a, b), "c2Sub special");
                assert_float(c_dot(a, b), r_dot(a, b), "c2Dot special");
                assert_float(c_len(a), r_len(a), "c2Len special");
                assert_float(c_det(a, b), r_det(a, b), "c2Det2 special");
                assert_bytes(&c_add(a, b), &r_add(a, b), "c2Add special");
                assert_bytes(&c_neg(a), &r_neg(a), "c2Neg special");
                assert_bytes(&c_skew(a), &r_skew(a), "c2Skew special");
                assert_bytes(&c_ccw(a), &r_ccw(a), "c2CCW90 special");
                assert_bytes(&c_div(a, y), &r_div(a, y), "c2Div special");
                assert_bytes(&c_norm(a), &r_norm(a), "c2Norm special");
            }
        }

        let mut rng = Rng::new(0x7a45_10c2_668d_90ef);
        for _ in 0..CASES * 4 {
            let a = rng.vec();
            let b = rng.vec();
            let lo = rng.vec();
            let hi = rng.vec();
            let scalar = rng.finite();
            let rot = c2r {
                c: rng.finite(),
                s: rng.finite(),
            };
            let transform = c2x {
                p: rng.vec(),
                r: rot,
            };
            assert_bytes(&c_v(a.x, a.y), &r_v(a.x, a.y), "c2V random");
            assert_bytes(&c_mulvs(a, scalar), &r_mulvs(a, scalar), "c2Mulvs random");
            assert_bytes(&c_max(a, b), &r_max(a, b), "c2Maxv random");
            assert_bytes(&c_min(a, b), &r_min(a, b), "c2Minv random");
            assert_bytes(&c_clamp(a, lo, hi), &r_clamp(a, lo, hi), "c2Clampv random");
            assert_bytes(&c_sub(a, b), &r_sub(a, b), "c2Sub random");
            assert_float(c_dot(a, b), r_dot(a, b), "c2Dot random");
            assert_float(c_len(a), r_len(a), "c2Len random");
            assert_float(c_det(a, b), r_det(a, b), "c2Det2 random");
            assert_bytes(&c_mulrv(rot, a), &r_mulrv(rot, a), "c2Mulrv random");
            assert_bytes(&c_mulrvt(rot, a), &r_mulrvt(rot, a), "c2MulrvT random");
            assert_bytes(&c_add(a, b), &r_add(a, b), "c2Add random");
            assert_bytes(
                &c_mulxv(transform, a),
                &r_mulxv(transform, a),
                "c2Mulxv random",
            );
            assert_bytes(&c_neg(a), &r_neg(a), "c2Neg random");
            assert_bytes(&c_skew(a), &r_skew(a), "c2Skew random");
            assert_bytes(&c_ccw(a), &r_ccw(a), "c2CCW90 random");
            assert_bytes(&c_div(a, scalar), &r_div(a, scalar), "c2Div random");
            assert_bytes(&c_norm(a), &r_norm(a), "c2Norm random");
        }
    }
}

#[test]
fn bounding_box_and_proxy_exports_match() {
    let libs = Libraries::load();
    unsafe {
        type BB = unsafe extern "C" fn(*mut c2v, *mut c2AABB);
        type Proxy = unsafe extern "C" fn(*const c_void, c_int, *mut c2Proxy);
        let (c_bb, r_bb) = libs.symbols::<BB>(b"c2BBVerts\0");
        let (c_proxy, r_proxy) = libs.symbols::<Proxy>(b"c2MakeProxy\0");
        let mut rng = Rng::new(0x8d1e_f977_5aa3_61c4);

        for _ in 0..CASES {
            let bb = c2AABB {
                min: rng.vec(),
                max: rng.vec(),
            };
            let mut c_out = [rng.vec(), rng.vec(), rng.vec(), rng.vec()];
            let mut r_out = c_out;
            c_bb(c_out.as_mut_ptr(), &mut bb.clone());
            r_bb(r_out.as_mut_ptr(), &mut bb.clone());
            assert_bytes(&c_out, &r_out, "c2BBVerts");

            let circle = c2Circle {
                p: rng.vec(),
                r: rng.finite(),
            };
            let capsule = c2Capsule {
                a: rng.vec(),
                b: rng.vec(),
                r: rng.finite(),
            };
            for (kind, shape) in [
                (CIRCLE, &circle as *const c2Circle as *const c_void),
                (AABB, &bb as *const c2AABB as *const c_void),
                (CAPSULE, &capsule as *const c2Capsule as *const c_void),
            ] {
                let seed = c2Proxy {
                    radius: rng.finite(),
                    count: rng.u32() as i32,
                    verts: [
                        rng.vec(),
                        rng.vec(),
                        rng.vec(),
                        rng.vec(),
                        rng.vec(),
                        rng.vec(),
                        rng.vec(),
                        rng.vec(),
                    ],
                };
                let mut c_out = seed;
                let mut r_out = seed;
                c_proxy(shape, kind, &mut c_out);
                r_proxy(shape, kind, &mut r_out);
                assert_bytes(&c_out, &r_out, "c2MakeProxy valid type");
            }
        }
    }
}

#[test]
fn invalid_proxy_enum_matches_c_noop() {
    let libs = Libraries::load();
    unsafe {
        type Proxy = unsafe extern "C" fn(*const c_void, c_int, *mut c2Proxy);
        let (c_proxy, r_proxy) = libs.symbols::<Proxy>(b"c2MakeProxy\0");
        let mut rng = Rng::new(0xc15e_39aa_2074_1df0);
        for &kind in &[-2_147_483_648, -1, 3, 4, 99, 2_147_483_647] {
            for _ in 0..CASES {
                let seed = c2Proxy {
                    radius: rng.finite(),
                    count: rng.u32() as i32,
                    verts: [
                        rng.vec(),
                        rng.vec(),
                        rng.vec(),
                        rng.vec(),
                        rng.vec(),
                        rng.vec(),
                        rng.vec(),
                        rng.vec(),
                    ],
                };
                let mut c_out = seed;
                let mut r_out = seed;
                c_proxy(std::ptr::null(), kind, &mut c_out);
                r_proxy(std::ptr::null(), kind, &mut r_out);
                assert_bytes(&seed, &c_out, "C invalid c2MakeProxy must be no-op");
                assert_bytes(&c_out, &r_out, "invalid c2MakeProxy enum");
            }
        }
    }
}

fn sub(a: c2v, b: c2v) -> c2v {
    c2v {
        x: a.x - b.x,
        y: a.y - b.y,
    }
}

fn dot(a: c2v, b: c2v) -> f32 {
    a.x * b.x + a.y * b.y
}

fn det(a: c2v, b: c2v) -> f32 {
    a.x * b.y - a.y * b.x
}

fn classify_c22(a: c2v, b: c2v) -> usize {
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

fn classify_c23(a: c2v, b: c2v, c: c2v) -> usize {
    let uAB = dot(b, sub(b, a));
    let vAB = dot(a, sub(a, b));
    let uBC = dot(c, sub(c, b));
    let vBC = dot(b, sub(b, c));
    let uCA = dot(a, sub(a, c));
    let vCA = dot(c, sub(c, a));
    let area = det(sub(b, a), sub(c, a));
    let uABC = det(b, c) * area;
    let vABC = det(c, a) * area;
    let wABC = det(a, b) * area;
    if vAB <= 0.0 && uCA <= 0.0 {
        0
    } else if uAB <= 0.0 && vBC <= 0.0 {
        1
    } else if uBC <= 0.0 && vCA <= 0.0 {
        2
    } else if uAB > 0.0 && vAB > 0.0 && wABC <= 0.0 {
        3
    } else if uBC > 0.0 && vBC > 0.0 && uABC <= 0.0 {
        4
    } else if uCA > 0.0 && vCA > 0.0 && vABC <= 0.0 {
        5
    } else {
        6
    }
}

#[test]
fn simplex_metric_direction_and_interpolation_exports_match() {
    let libs = Libraries::load();
    unsafe {
        type SF = unsafe extern "C" fn(*mut c2Simplex) -> f32;
        type SV = unsafe extern "C" fn(*mut c2Simplex) -> c2v;
        let (c_metric, r_metric) = libs.symbols::<SF>(b"c2GJKSimplexMetric\0");
        let (c_d, r_d) = libs.symbols::<SV>(b"c2D\0");
        let (c_l, r_l) = libs.symbols::<SV>(b"c2L\0");
        let mut rng = Rng::new(0x4b3f_20a1_c79d_558e);

        for &count in &[-7, 0, 1, 2, 3, 4, 99] {
            for _ in 0..CASES {
                let points = [rng.vec(), rng.vec(), rng.vec()];
                let seed = simplex_with_points(&mut rng, points, count);

                let mut c_input = seed;
                let mut r_input = seed;
                assert_float(
                    c_metric(&mut c_input),
                    r_metric(&mut r_input),
                    "c2GJKSimplexMetric",
                );
                assert_bytes(&c_input, &r_input, "metric input preservation");

                let mut c_input = seed;
                let mut r_input = seed;
                assert_bytes(&c_d(&mut c_input), &r_d(&mut r_input), "c2D count mode");
                assert_bytes(&c_input, &r_input, "c2D input preservation");

                let mut c_input = seed;
                let mut r_input = seed;
                assert_bytes(&c_l(&mut c_input), &r_l(&mut r_input), "c2L count mode");
                assert_bytes(&c_input, &r_input, "c2L input preservation");
            }
        }

        for sign in [-1.0f32, 0.0, 1.0] {
            for _ in 0..CASES {
                let a = c2v {
                    x: rng.positive(),
                    y: 0.0,
                };
                let b = c2v {
                    x: a.x,
                    y: sign * rng.positive(),
                };
                let c = rng.vec();
                let seed = simplex_with_points(&mut rng, [a, b, c], 2);
                let mut c_input = seed;
                let mut r_input = seed;
                assert_bytes(
                    &c_d(&mut c_input),
                    &r_d(&mut r_input),
                    "c2D determinant sign",
                );
            }
        }
    }
}

#[test]
fn c22_all_branches_match() {
    let libs = Libraries::load();
    unsafe {
        type S = unsafe extern "C" fn(*mut c2Simplex);
        let (c_fn, r_fn) = libs.symbols::<S>(b"c22\0");
        let mut rng = Rng::new(0xa711_0c92_e43b_8d5f);
        let mut counts = [0usize; 3];
        for _ in 0..1_000_000 {
            if counts.iter().all(|&count| count >= CASES) {
                break;
            }
            let points = [rng.vec(), rng.vec(), rng.vec()];
            let branch = classify_c22(points[0], points[1]);
            if counts[branch] >= CASES {
                continue;
            }
            let seed = simplex_with_points(&mut rng, points, 2);
            let mut c_simplex = seed;
            let mut r_simplex = seed;
            c_fn(&mut c_simplex);
            r_fn(&mut r_simplex);
            assert_bytes(&c_simplex, &r_simplex, "c22 branch");
            counts[branch] += 1;
        }
        assert_eq!(counts, [CASES; 3], "insufficient c22 branch samples");
    }
}

#[test]
fn c23_all_branches_match() {
    let libs = Libraries::load();
    unsafe {
        type S = unsafe extern "C" fn(*mut c2Simplex);
        let (c_fn, r_fn) = libs.symbols::<S>(b"c23\0");
        let mut rng = Rng::new(0x3ca4_896e_b5d2_701f);
        let mut counts = [0usize; 7];
        for _ in 0..2_000_000 {
            if counts.iter().all(|&count| count >= CASES) {
                break;
            }
            let points = [rng.vec(), rng.vec(), rng.vec()];
            let branch = classify_c23(points[0], points[1], points[2]);
            if counts[branch] >= CASES {
                continue;
            }
            let seed = simplex_with_points(&mut rng, points, 3);
            let mut c_simplex = seed;
            let mut r_simplex = seed;
            c_fn(&mut c_simplex);
            r_fn(&mut r_simplex);
            assert_bytes(&c_simplex, &r_simplex, "c23 branch");
            counts[branch] += 1;
        }
        assert_eq!(counts, [CASES; 7], "insufficient c23 branch samples");
    }
}

#[test]
fn support_and_witness_exports_match() {
    let libs = Libraries::load();
    unsafe {
        type Support = unsafe extern "C" fn(*const c2v, c_int, c2v) -> c_int;
        type Witness = unsafe extern "C" fn(*mut c2Simplex, *mut c2v, *mut c2v);
        let (c_support, r_support) = libs.symbols::<Support>(b"c2Support\0");
        let (c_witness, r_witness) = libs.symbols::<Witness>(b"c2Witness\0");
        let mut rng = Rng::new(0xf618_2dac_9357_40be);

        for _ in 0..CASES {
            let singleton = [rng.vec()];
            let direction = rng.vec();
            assert_eq!(
                c_support(singleton.as_ptr(), 1, direction),
                r_support(singleton.as_ptr(), 1, direction),
                "c2Support singleton"
            );

            let first_x = rng.positive();
            let tied = [
                c2v {
                    x: first_x,
                    y: rng.finite(),
                },
                c2v {
                    x: first_x,
                    y: rng.finite(),
                },
                c2v {
                    x: first_x - rng.positive(),
                    y: rng.finite(),
                },
            ];
            let d = c2v { x: 1.0, y: 0.0 };
            assert_eq!(c_support(tied.as_ptr(), 3, d), 0, "C support tie");
            assert_eq!(
                c_support(tied.as_ptr(), 3, d),
                r_support(tied.as_ptr(), 3, d),
                "c2Support first/tie"
            );

            let later = [
                c2v {
                    x: first_x,
                    y: rng.finite(),
                },
                c2v {
                    x: first_x + rng.positive(),
                    y: rng.finite(),
                },
                c2v {
                    x: first_x - rng.positive(),
                    y: rng.finite(),
                },
            ];
            assert_eq!(
                c_support(later.as_ptr(), 3, d),
                r_support(later.as_ptr(), 3, d),
                "c2Support later maximum"
            );
        }

        for &count in &[-17, -1, 0] {
            for _ in 0..CASES {
                let readable_first = [rng.vec()];
                let direction = rng.vec();
                assert_eq!(
                    c_support(readable_first.as_ptr(), count, direction),
                    r_support(readable_first.as_ptr(), count, direction),
                    "c2Support nonpositive count"
                );
            }
        }

        for &count in &[9usize, 1024] {
            for _ in 0..CASES {
                let verts: Vec<c2v> = (0..count).map(|_| rng.vec()).collect();
                let direction = rng.vec();
                assert_eq!(
                    c_support(verts.as_ptr(), count as i32, direction),
                    r_support(verts.as_ptr(), count as i32, direction),
                    "c2Support oversized readable input"
                );
            }
        }

        for &count in &[-3, 0, 1, 2, 3, 4, 50] {
            for _ in 0..CASES {
                let points = [rng.vec(), rng.vec(), rng.vec()];
                let mut seed = simplex_with_points(&mut rng, points, count);
                seed.div = rng.positive();
                for vertex in &mut seed.verts {
                    vertex.u = rng.positive();
                }
                let mut c_simplex = seed;
                let mut r_simplex = seed;
                let mut c_a = rng.vec();
                let mut c_b = rng.vec();
                let mut r_a = c_a;
                let mut r_b = c_b;
                c_witness(&mut c_simplex, &mut c_a, &mut c_b);
                r_witness(&mut r_simplex, &mut r_a, &mut r_b);
                assert_bytes(&c_a, &r_a, "c2Witness A");
                assert_bytes(&c_b, &r_b, "c2Witness B");
                assert_bytes(&c_simplex, &r_simplex, "c2Witness input preservation");
            }
        }
    }
}

#[derive(Clone, Copy)]
enum Shape {
    Circle(c2Circle),
    Aabb(c2AABB),
    Capsule(c2Capsule),
}

impl Shape {
    fn kind(self) -> c_int {
        match self {
            Shape::Circle(_) => CIRCLE,
            Shape::Aabb(_) => AABB,
            Shape::Capsule(_) => CAPSULE,
        }
    }

    fn ptr(&self) -> *const c_void {
        match self {
            Shape::Circle(value) => value as *const c2Circle as *const c_void,
            Shape::Aabb(value) => value as *const c2AABB as *const c_void,
            Shape::Capsule(value) => value as *const c2Capsule as *const c_void,
        }
    }

    fn x_extent(self) -> f32 {
        match self {
            Shape::Circle(value) => value.r.abs(),
            Shape::Aabb(value) => (value.max.x - value.min.x).abs() * 0.5,
            Shape::Capsule(value) => (value.a.x - value.b.x).abs() * 0.5 + value.r.abs(),
        }
    }

    fn shift_x(&mut self, delta: f32) {
        match self {
            Shape::Circle(value) => value.p.x += delta,
            Shape::Aabb(value) => {
                value.min.x += delta;
                value.max.x += delta;
            }
            Shape::Capsule(value) => {
                value.a.x += delta;
                value.b.x += delta;
            }
        }
    }
}

fn random_shape(kind: c_int, rng: &mut Rng) -> Shape {
    let y = rng.finite() * 0.02;
    match kind {
        CIRCLE => Shape::Circle(c2Circle {
            p: c2v { x: 0.0, y },
            r: rng.positive(),
        }),
        AABB => {
            let hx = rng.positive();
            let hy = rng.positive();
            Shape::Aabb(c2AABB {
                min: c2v { x: -hx, y: y - hy },
                max: c2v { x: hx, y: y + hy },
            })
        }
        CAPSULE => {
            let half_x = (rng.u32() % 2) as f32 * rng.positive() * 0.2;
            let half_y = rng.positive();
            Shape::Capsule(c2Capsule {
                a: c2v {
                    x: -half_x,
                    y: y - half_y,
                },
                b: c2v {
                    x: half_x,
                    y: y + half_y,
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
    *const c2x,
    *const c_void,
    c_int,
    *const c2x,
    *mut c2v,
    *mut c2v,
    c_int,
    *mut c_int,
    *mut c2GJKCache,
) -> f32;

#[derive(Clone, Copy)]
struct GjkResult {
    distance: f32,
    out_a: c2v,
    out_b: c2v,
    iterations: c_int,
    cache: Option<c2GJKCache>,
}

unsafe fn invoke_gjk(
    function: &Symbol<'_, Gjk>,
    a: &Shape,
    ax: Option<&c2x>,
    b: &Shape,
    bx: Option<&c2x>,
    use_radius: c_int,
    mut cache: Option<c2GJKCache>,
) -> GjkResult {
    let mut out_a = c2v {
        x: f32::from_bits(0x7fc0_1234),
        y: f32::from_bits(0xffc0_5678),
    };
    let mut out_b = c2v {
        x: f32::from_bits(0x7fc0_9abc),
        y: f32::from_bits(0xffc0_def0),
    };
    let mut iterations = -77;
    let distance = unsafe {
        function(
            a.ptr(),
            a.kind(),
            ax.map_or(std::ptr::null(), |value| value),
            b.ptr(),
            b.kind(),
            bx.map_or(std::ptr::null(), |value| value),
            &mut out_a,
            &mut out_b,
            use_radius,
            &mut iterations,
            cache.as_mut().map_or(std::ptr::null_mut(), |value| value),
        )
    };
    GjkResult {
        distance,
        out_a,
        out_b,
        iterations,
        cache,
    }
}

fn assert_gjk(c: &GjkResult, rust: &GjkResult, context: &str) {
    assert_float(c.distance, rust.distance, context);
    assert_bytes(&c.out_a, &rust.out_a, context);
    assert_bytes(&c.out_b, &rust.out_b, context);
    assert_eq!(c.iterations, rust.iterations, "{context}: iterations");
    match (&c.cache, &rust.cache) {
        (Some(c_cache), Some(r_cache)) => assert_bytes(c_cache, r_cache, context),
        (None, None) => {}
        _ => panic!("{context}: cache presence differs"),
    }
}

fn empty_cache(rng: &mut Rng) -> c2GJKCache {
    c2GJKCache {
        metric: rng.finite(),
        count: 0,
        iA: [rng.u32() as i32, rng.u32() as i32, rng.u32() as i32],
        iB: [rng.u32() as i32, rng.u32() as i32, rng.u32() as i32],
        div: rng.finite(),
    }
}

#[test]
fn gjk_ordered_shape_pair_cross_product_matches() {
    let libs = Libraries::load();
    unsafe {
        let (c_gjk, r_gjk) = libs.symbols::<Gjk>(b"c2GJK\0");
        let mut rng = Rng::new(0x5e0a_a32d_9418_b76c);
        let kinds = [CIRCLE, AABB, CAPSULE];
        let mut saw_zero_iterations = false;
        let mut saw_multiple_iterations = false;
        let mut saw_zero_distance = false;
        let mut saw_positive_distance = false;

        for &kind_a in &kinds {
            for &kind_b in &kinds {
                for _ in 0..16 {
                    let a = random_shape(kind_a, &mut rng);
                    let base_b = random_shape(kind_b, &mut rng);
                    for geometry in 0..3 {
                        let mut b = base_b;
                        let separation = match geometry {
                            0 => a.x_extent() + b.x_extent() + rng.positive(),
                            1 => a.x_extent() + b.x_extent(),
                            _ => 0.0,
                        };
                        b.shift_x(separation);
                        for &use_radius in &[0, -3] {
                            for ax_mode in 0..2 {
                                for bx_mode in 0..2 {
                                    let ax_value = c2x {
                                        p: c2v {
                                            x: rng.finite() * 0.01,
                                            y: rng.finite() * 0.01,
                                        },
                                        r: c2r {
                                            c: 0.9238795,
                                            s: 0.38268343,
                                        },
                                    };
                                    let bx_value = c2x {
                                        p: c2v {
                                            x: rng.finite() * 0.01,
                                            y: rng.finite() * 0.01,
                                        },
                                        r: c2r {
                                            c: 0.9659258,
                                            s: -0.25881904,
                                        },
                                    };
                                    let ax = (ax_mode != 0).then_some(&ax_value);
                                    let bx = (bx_mode != 0).then_some(&bx_value);

                                    let c_result =
                                        invoke_gjk(&c_gjk, &a, ax, &b, bx, use_radius, None);
                                    let r_result =
                                        invoke_gjk(&r_gjk, &a, ax, &b, bx, use_radius, None);
                                    assert_gjk(&c_result, &r_result, "c2GJK null cache");

                                    let seed = empty_cache(&mut rng);
                                    let c_empty =
                                        invoke_gjk(&c_gjk, &a, ax, &b, bx, use_radius, Some(seed));
                                    let r_empty =
                                        invoke_gjk(&r_gjk, &a, ax, &b, bx, use_radius, Some(seed));
                                    assert_gjk(&c_empty, &r_empty, "c2GJK empty cache");

                                    let c_warm = invoke_gjk(
                                        &c_gjk,
                                        &a,
                                        ax,
                                        &b,
                                        bx,
                                        use_radius,
                                        c_empty.cache,
                                    );
                                    let r_warm = invoke_gjk(
                                        &r_gjk,
                                        &a,
                                        ax,
                                        &b,
                                        bx,
                                        use_radius,
                                        r_empty.cache,
                                    );
                                    assert_gjk(&c_warm, &r_warm, "c2GJK warm cache");

                                    saw_zero_iterations |= c_result.iterations == 0;
                                    saw_multiple_iterations |= c_result.iterations >= 2;
                                    saw_zero_distance |= c_result.distance == 0.0;
                                    saw_positive_distance |= c_result.distance > 0.0;
                                }
                            }
                        }
                    }
                }
            }
        }
        assert!(saw_zero_iterations, "GJK matrix missed zero-iteration exit");
        assert!(
            saw_multiple_iterations,
            "GJK matrix missed simplex iteration"
        );
        assert!(saw_zero_distance, "GJK matrix missed overlap/hit");
        assert!(saw_positive_distance, "GJK matrix missed separated shapes");
    }
}

#[test]
fn gjk_optional_outputs_and_cache_rejection_match() {
    let libs = Libraries::load();
    unsafe {
        let (c_gjk, r_gjk) = libs.symbols::<Gjk>(b"c2GJK\0");
        let mut rng = Rng::new(0x2d8b_705e_f431_9ac6);

        for mask in 0u8..8 {
            for _ in 0..CASES {
                let a = random_shape(AABB, &mut rng);
                let mut b = random_shape(CAPSULE, &mut rng);
                b.shift_x(rng.positive());
                let mut c_a = rng.vec();
                let mut c_b = rng.vec();
                let mut r_a = c_a;
                let mut r_b = c_b;
                let mut c_iterations = rng.u32() as i32;
                let mut r_iterations = c_iterations;
                let c_distance = c_gjk(
                    a.ptr(),
                    a.kind(),
                    std::ptr::null(),
                    b.ptr(),
                    b.kind(),
                    std::ptr::null(),
                    if mask & 1 != 0 {
                        &mut c_a
                    } else {
                        std::ptr::null_mut()
                    },
                    if mask & 2 != 0 {
                        &mut c_b
                    } else {
                        std::ptr::null_mut()
                    },
                    -1,
                    if mask & 4 != 0 {
                        &mut c_iterations
                    } else {
                        std::ptr::null_mut()
                    },
                    std::ptr::null_mut(),
                );
                let r_distance = r_gjk(
                    a.ptr(),
                    a.kind(),
                    std::ptr::null(),
                    b.ptr(),
                    b.kind(),
                    std::ptr::null(),
                    if mask & 1 != 0 {
                        &mut r_a
                    } else {
                        std::ptr::null_mut()
                    },
                    if mask & 2 != 0 {
                        &mut r_b
                    } else {
                        std::ptr::null_mut()
                    },
                    -1,
                    if mask & 4 != 0 {
                        &mut r_iterations
                    } else {
                        std::ptr::null_mut()
                    },
                    std::ptr::null_mut(),
                );
                assert_float(c_distance, r_distance, "c2GJK optional outputs");
                assert_bytes(&c_a, &r_a, "c2GJK optional outA");
                assert_bytes(&c_b, &r_b, "c2GJK optional outB");
                assert_eq!(c_iterations, r_iterations, "c2GJK optional iterations");
            }
        }

        for _ in 0..CASES {
            let a = Shape::Aabb(c2AABB {
                min: c2v { x: 0.0, y: 0.0 },
                max: c2v {
                    x: 20_000.0,
                    y: 20_000.0,
                },
            });
            let b = a;
            let rejected = c2GJKCache {
                metric: 1.0,
                count: 3,
                iA: [0, 0, 0],
                iB: [0, 2, 1],
                div: rng.positive(),
            };
            let empty = c2GJKCache {
                count: 0,
                ..rejected
            };
            let c_rejected = invoke_gjk(&c_gjk, &a, None, &b, None, 0, Some(rejected));
            let r_rejected = invoke_gjk(&r_gjk, &a, None, &b, None, 0, Some(rejected));
            assert_gjk(&c_rejected, &r_rejected, "c2GJK rejected cache");

            let c_empty = invoke_gjk(&c_gjk, &a, None, &b, None, 0, Some(empty));
            assert_float(
                c_rejected.distance,
                c_empty.distance,
                "C rejected cache distance",
            );
            assert_bytes(&c_rejected.out_a, &c_empty.out_a, "C rejected cache outA");
            assert_bytes(&c_rejected.out_b, &c_empty.out_b, "C rejected cache outB");
            assert_eq!(
                c_rejected.iterations, c_empty.iterations,
                "C cache rejection iterations"
            );
        }
    }
}

#[test]
fn gjk_cache_wrapper_reverse_modes_match() {
    let libs = Libraries::load();
    unsafe {
        type Wrapper = unsafe extern "C" fn(
            c_char,
            *mut c2v,
            *mut c2v,
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
        let (c_wrapper, r_wrapper) = libs.symbols::<Wrapper>(b"gjk_cache\0");
        let mut rng = Rng::new(0x9713_6ea8_b04f_2dc5);
        for &reverse in &[0i8, 1, -1, 127] {
            for _ in 0..CASES {
                let args = [
                    rng.finite(),
                    rng.finite(),
                    rng.finite(),
                    rng.finite(),
                    rng.finite(),
                    rng.finite(),
                    rng.finite(),
                    rng.finite(),
                    rng.finite(),
                ];
                let mut c_a = rng.vec();
                let mut c_b = rng.vec();
                let mut r_a = c_a;
                let mut r_b = c_b;
                c_wrapper(
                    reverse, &mut c_a, &mut c_b, args[0], args[1], args[2], args[3], args[4],
                    args[5], args[6], args[7], args[8],
                );
                r_wrapper(
                    reverse, &mut r_a, &mut r_b, args[0], args[1], args[2], args[3], args[4],
                    args[5], args[6], args[7], args[8],
                );
                assert_bytes(&c_a, &r_a, "gjk_cache untouched a9");
                assert_bytes(&c_b, &r_b, "gjk_cache untouched b9");

                c_wrapper(
                    reverse,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    args[0],
                    args[1],
                    args[2],
                    args[3],
                    args[4],
                    args[5],
                    args[6],
                    args[7],
                    args[8],
                );
                r_wrapper(
                    reverse,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    args[0],
                    args[1],
                    args[2],
                    args[3],
                    args[4],
                    args[5],
                    args[6],
                    args[7],
                    args[8],
                );
            }
        }
    }
}

fn selected_library(which: &str) -> Library {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let path = match which {
        "c" => root.join("c_src/build/libtranslated_rust.so"),
        "rust" => root.join("target/release/libgjk_cache_lib.so"),
        _ => panic!("unknown probe library"),
    };
    unsafe { Library::new(path).expect("load probe library") }
}

#[test]
fn required_null_pointer_probe_child() {
    let Some(case) = std::env::var_os("FFI_NULL_PROBE_CASE") else {
        return;
    };
    let which = std::env::var("FFI_NULL_PROBE_LIB").expect("probe library selector");
    let library = selected_library(&which);
    let case = case.to_string_lossy();
    unsafe {
        match case.as_ref() {
            "bb_out" => {
                let function: Symbol<unsafe extern "C" fn(*mut c2v, *mut c2AABB)> =
                    library.get(b"c2BBVerts\0").unwrap();
                let mut bb = c2AABB {
                    min: ZEROV,
                    max: ZEROV,
                };
                function(std::ptr::null_mut(), &mut bb);
            }
            "bb_input" => {
                let function: Symbol<unsafe extern "C" fn(*mut c2v, *mut c2AABB)> =
                    library.get(b"c2BBVerts\0").unwrap();
                let mut out = [ZEROV; 4];
                function(out.as_mut_ptr(), std::ptr::null_mut());
            }
            "proxy_shape" => {
                let function: Symbol<unsafe extern "C" fn(*const c_void, c_int, *mut c2Proxy)> =
                    library.get(b"c2MakeProxy\0").unwrap();
                let mut proxy = zero_proxy();
                function(std::ptr::null(), CIRCLE, &mut proxy);
            }
            "proxy_out" => {
                let function: Symbol<unsafe extern "C" fn(*const c_void, c_int, *mut c2Proxy)> =
                    library.get(b"c2MakeProxy\0").unwrap();
                let circle = c2Circle { p: ZEROV, r: 1.0 };
                function(
                    &circle as *const c2Circle as *const c_void,
                    CIRCLE,
                    std::ptr::null_mut(),
                );
            }
            "metric" => {
                let function: Symbol<unsafe extern "C" fn(*mut c2Simplex) -> f32> =
                    library.get(b"c2GJKSimplexMetric\0").unwrap();
                std::hint::black_box(function(std::ptr::null_mut()));
            }
            "c22" => {
                let function: Symbol<unsafe extern "C" fn(*mut c2Simplex)> =
                    library.get(b"c22\0").unwrap();
                function(std::ptr::null_mut());
            }
            "c23" => {
                let function: Symbol<unsafe extern "C" fn(*mut c2Simplex)> =
                    library.get(b"c23\0").unwrap();
                function(std::ptr::null_mut());
            }
            "direction" => {
                let function: Symbol<unsafe extern "C" fn(*mut c2Simplex) -> c2v> =
                    library.get(b"c2D\0").unwrap();
                std::hint::black_box(function(std::ptr::null_mut()));
            }
            "support" => {
                let function: Symbol<unsafe extern "C" fn(*const c2v, c_int, c2v) -> c_int> =
                    library.get(b"c2Support\0").unwrap();
                std::hint::black_box(function(std::ptr::null(), 1, ZEROV));
            }
            "witness_simplex" => {
                let function: Symbol<unsafe extern "C" fn(*mut c2Simplex, *mut c2v, *mut c2v)> =
                    library.get(b"c2Witness\0").unwrap();
                let mut a = ZEROV;
                let mut b = ZEROV;
                function(std::ptr::null_mut(), &mut a, &mut b);
            }
            "witness_a" => {
                let function: Symbol<unsafe extern "C" fn(*mut c2Simplex, *mut c2v, *mut c2v)> =
                    library.get(b"c2Witness\0").unwrap();
                let mut simplex = one_point_simplex();
                let mut b = ZEROV;
                function(&mut simplex, std::ptr::null_mut(), &mut b);
            }
            "witness_b" => {
                let function: Symbol<unsafe extern "C" fn(*mut c2Simplex, *mut c2v, *mut c2v)> =
                    library.get(b"c2Witness\0").unwrap();
                let mut simplex = one_point_simplex();
                let mut a = ZEROV;
                function(&mut simplex, &mut a, std::ptr::null_mut());
            }
            "interpolation" => {
                let function: Symbol<unsafe extern "C" fn(*mut c2Simplex) -> c2v> =
                    library.get(b"c2L\0").unwrap();
                std::hint::black_box(function(std::ptr::null_mut()));
            }
            "gjk_a" | "gjk_b" => {
                let function: Symbol<Gjk> = library.get(b"c2GJK\0").unwrap();
                let circle = c2Circle { p: ZEROV, r: 1.0 };
                let valid = &circle as *const c2Circle as *const c_void;
                let null = std::ptr::null();
                std::hint::black_box(function(
                    if case == "gjk_a" { null } else { valid },
                    CIRCLE,
                    std::ptr::null(),
                    if case == "gjk_b" { null } else { valid },
                    CIRCLE,
                    std::ptr::null(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    0,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                ));
            }
            _ => panic!("unknown null probe"),
        }
    }
    panic!("required-null probe unexpectedly returned");
}

const ZEROV: c2v = c2v { x: 0.0, y: 0.0 };
const ZEROSV: c2sv = c2sv {
    sA: ZEROV,
    sB: ZEROV,
    p: ZEROV,
    u: 0.0,
    iA: 0,
    iB: 0,
};

fn zero_proxy() -> c2Proxy {
    c2Proxy {
        radius: 0.0,
        count: 0,
        verts: [ZEROV; 8],
    }
}

fn one_point_simplex() -> c2Simplex {
    c2Simplex {
        verts: [ZEROSV; 4],
        div: 1.0,
        count: 1,
    }
}

#[cfg(unix)]
#[test]
fn required_null_pointer_crash_behavior_matches() {
    use std::os::unix::process::ExitStatusExt;
    use std::process::Command;

    let cases = [
        "bb_out",
        "bb_input",
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
        "interpolation",
        "gjk_a",
        "gjk_b",
    ];
    let executable = std::env::current_exe().expect("test executable");
    for case in cases {
        let run = |which: &str| {
            Command::new(&executable)
                .args([
                    "--exact",
                    "required_null_pointer_probe_child",
                    "--test-threads=1",
                ])
                .env("FFI_NULL_PROBE_CASE", case)
                .env("FFI_NULL_PROBE_LIB", which)
                .output()
                .expect("run null-pointer probe")
        };
        let c = run("c");
        let rust = run("rust");
        assert!(
            !c.status.success() && !rust.status.success(),
            "{case}: a required-null call unexpectedly succeeded"
        );
        assert!(
            c.status.signal().is_some() && rust.status.signal().is_some(),
            "{case}: probe did not terminate by signal; C={:?}, Rust={:?}",
            c.status,
            rust.status
        );
        assert_eq!(
            c.status.signal(),
            rust.status.signal(),
            "{case}: C and Rust terminated differently"
        );
    }
}
