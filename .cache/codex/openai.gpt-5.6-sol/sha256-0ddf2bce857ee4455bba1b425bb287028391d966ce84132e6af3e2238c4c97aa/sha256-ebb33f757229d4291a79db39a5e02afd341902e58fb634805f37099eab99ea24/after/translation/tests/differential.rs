use libloading::Library;
use std::ffi::{c_int, c_void};
use std::path::PathBuf;

const CASES: usize = 96;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct V {
    x: f32,
    y: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Raycast {
    t: f32,
    n: V,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Rot {
    c: f32,
    s: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Xform {
    p: V,
    r: Rot,
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
struct Poly {
    count: c_int,
    verts: [V; 8],
    norms: [V; 8],
}

#[repr(C)]
struct PaddedPoly {
    poly: Poly,
    ninth_norm: V,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Ray {
    p: V,
    d: V,
    t: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Mat {
    x: V,
    y: V,
}

type VFn = unsafe extern "C" fn(f32, f32) -> V;
type VVFloatFn = unsafe extern "C" fn(V, V) -> f32;
type VFloatFn = unsafe extern "C" fn(V) -> f32;
type VVFn = unsafe extern "C" fn(V, V) -> V;
type VSFn = unsafe extern "C" fn(V, f32) -> V;
type VToVFn = unsafe extern "C" fn(V) -> V;
type AabbAabbFn = unsafe extern "C" fn(Aabb, Aabb) -> c_int;
type AabbPointFn = unsafe extern "C" fn(Aabb, V) -> c_int;
type CirclePointFn = unsafe extern "C" fn(Circle, V) -> c_int;
type MatVFn = unsafe extern "C" fn(Mat, V) -> V;
type RotFn = unsafe extern "C" fn() -> Rot;
type XformFn = unsafe extern "C" fn() -> Xform;
type RotVFn = unsafe extern "C" fn(Rot, V) -> V;
type XformVFn = unsafe extern "C" fn(Xform, V) -> V;
type RayCircleFn = unsafe extern "C" fn(Ray, Circle, *mut Raycast) -> c_int;
type RayAabbFn = unsafe extern "C" fn(Ray, Aabb, *mut Raycast) -> c_int;
type RayCapsuleFn = unsafe extern "C" fn(Ray, Capsule, *mut Raycast) -> c_int;
type RayPolyFn = unsafe extern "C" fn(Ray, *const Poly, *const Xform, *mut Raycast) -> c_int;
type CastRayFn =
    unsafe extern "C" fn(Ray, *const c_void, *const Xform, c_int, *mut Raycast) -> c_int;
type PolyRayFn = unsafe extern "C" fn(*mut Raycast, *mut Raycast) -> c_int;

fn libraries() -> (Library, Library) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_path = root.join("../c_src/build/libharvest-work-U9Mauy.so");
    let rust_path = root.join("target/release/libpoly_ray_lib.so");
    assert!(
        c_path.is_file(),
        "missing C shared object: {}",
        c_path.display()
    );
    assert!(
        rust_path.is_file(),
        "missing Rust shared object: {}",
        rust_path.display()
    );
    unsafe {
        (
            Library::new(c_path).expect("load C shared object"),
            Library::new(rust_path).expect("load Rust shared object"),
        )
    }
}

unsafe fn symbol<T: Copy>(lib: &Library, name: &[u8]) -> T {
    unsafe { *lib.get::<T>(name).unwrap() }
}

fn raw<T>(value: &T) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts((value as *const T).cast::<u8>(), std::mem::size_of::<T>())
    }
}

fn same<T>(label: &str, c: T, rust: T) {
    assert_eq!(
        raw(&c),
        raw(&rust),
        "{label}: C={:?} Rust={:?}",
        raw(&c),
        raw(&rust)
    );
}

fn sentinel() -> Raycast {
    Raycast {
        t: f32::from_bits(0x7fc0_1234),
        n: V {
            x: f32::from_bits(0x7fc0_5678),
            y: -12345.25,
        },
    }
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn u32(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 32) as u32
    }

    fn unit(&mut self) -> f32 {
        (self.u32() as f64 / u32::MAX as f64) as f32
    }

    fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.unit()
    }

    fn signed(&mut self) -> f32 {
        self.range(-20.0, 20.0)
    }
}

fn v(x: f32, y: f32) -> V {
    V { x, y }
}

fn box_at(cx: f32, cy: f32, hx: f32, hy: f32) -> Aabb {
    Aabb {
        min: v(cx - hx, cy - hy),
        max: v(cx + hx, cy + hy),
    }
}

fn rect_poly(hx: f32, hy: f32) -> Poly {
    let mut p = Poly {
        count: 4,
        verts: [v(0.0, 0.0); 8],
        norms: [v(0.0, 0.0); 8],
    };
    p.verts[0] = v(hx, -hy);
    p.verts[1] = v(hx, hy);
    p.verts[2] = v(-hx, hy);
    p.verts[3] = v(-hx, -hy);
    p.norms[0] = v(1.0, 0.0);
    p.norms[1] = v(0.0, 1.0);
    p.norms[2] = v(-1.0, 0.0);
    p.norms[3] = v(0.0, -1.0);
    p
}

fn octagon_poly(r: f32) -> Poly {
    let q = std::f32::consts::FRAC_1_SQRT_2;
    let normals = [
        v(1.0, 0.0),
        v(q, q),
        v(0.0, 1.0),
        v(-q, q),
        v(-1.0, 0.0),
        v(-q, -q),
        v(0.0, -1.0),
        v(q, -q),
    ];
    let mut p = Poly {
        count: 8,
        verts: [v(0.0, 0.0); 8],
        norms: normals,
    };
    for (i, n) in normals.into_iter().enumerate() {
        p.verts[i] = v(n.x * r, n.y * r);
    }
    p
}

#[test]
fn configs_001_through_024_vector_surface() {
    let (c_lib, r_lib) = libraries();
    unsafe {
        let cv: VFn = symbol(&c_lib, b"c2V\0");
        let rv: VFn = symbol(&r_lib, b"c2V\0");
        let cdot: VVFloatFn = symbol(&c_lib, b"c2Dot\0");
        let rdot: VVFloatFn = symbol(&r_lib, b"c2Dot\0");
        let clen: VFloatFn = symbol(&c_lib, b"c2Len\0");
        let rlen: VFloatFn = symbol(&r_lib, b"c2Len\0");
        let cadd: VVFn = symbol(&c_lib, b"c2Add\0");
        let radd: VVFn = symbol(&r_lib, b"c2Add\0");
        let csub: VVFn = symbol(&c_lib, b"c2Sub\0");
        let rsub: VVFn = symbol(&r_lib, b"c2Sub\0");
        let cmul: VSFn = symbol(&c_lib, b"c2Mulvs\0");
        let rmul: VSFn = symbol(&r_lib, b"c2Mulvs\0");
        let cdiv: VSFn = symbol(&c_lib, b"c2Div\0");
        let rdiv: VSFn = symbol(&r_lib, b"c2Div\0");
        let cnorm: VToVFn = symbol(&c_lib, b"c2Norm\0");
        let rnorm: VToVFn = symbol(&r_lib, b"c2Norm\0");
        let cmin: VVFn = symbol(&c_lib, b"c2Minv\0");
        let rmin: VVFn = symbol(&r_lib, b"c2Minv\0");
        let cmax: VVFn = symbol(&c_lib, b"c2Maxv\0");
        let rmax: VVFn = symbol(&r_lib, b"c2Maxv\0");
        let cskew: VToVFn = symbol(&c_lib, b"c2Skew\0");
        let rskew: VToVFn = symbol(&r_lib, b"c2Skew\0");
        let cabs: VToVFn = symbol(&c_lib, b"c2Absv\0");
        let rabs: VToVFn = symbol(&r_lib, b"c2Absv\0");

        let mut rng = Rng::new(0x0010_0240_cafe_f00d);
        for i in 0..CASES {
            let a = v(rng.signed(), rng.signed());
            let mut b = v(rng.signed(), rng.signed());
            if b.x == a.x {
                b.x += 1.0;
            }
            if b.y == a.y {
                b.y += 1.0;
            }
            same("001 c2V", cv(a.x, a.y), rv(a.x, a.y));
            same("002 c2Dot", cdot(a, b), rdot(a, b));
            let nz = if a.x == 0.0 && a.y == 0.0 {
                v(1.0, 0.0)
            } else {
                a
            };
            same("003 c2Len nonzero", clen(nz), rlen(nz));
            same("004 c2Len zero", clen(v(0.0, 0.0)), rlen(v(0.0, 0.0)));
            same("005 c2Add", cadd(a, b), radd(a, b));
            same("006 c2Sub", csub(a, b), rsub(a, b));
            let scalar = match i % 3 {
                0 => rng.range(0.01, 10.0),
                1 => 0.0,
                _ => rng.range(-10.0, -0.01),
            };
            same("007 c2Mulvs", cmul(a, scalar), rmul(a, scalar));
            let divisor = if i % 2 == 0 {
                rng.range(0.01, 10.0)
            } else {
                rng.range(-10.0, -0.01)
            };
            same("008 c2Div nonzero", cdiv(a, divisor), rdiv(a, divisor));
            same("009 c2Div zero", cdiv(a, 0.0), rdiv(a, 0.0));
            same("010 c2Norm nonzero", cnorm(nz), rnorm(nz));
            same("011 c2Norm zero", cnorm(v(0.0, 0.0)), rnorm(v(0.0, 0.0)));

            let lo_x = rng.range(-20.0, -1.0);
            let hi_x = rng.range(1.0, 20.0);
            let lo_y = rng.range(-20.0, -1.0);
            let hi_y = rng.range(1.0, 20.0);
            let min_cases = [
                (v(lo_x, lo_y), v(hi_x, hi_y), "012 c2Minv AA"),
                (v(lo_x, hi_y), v(hi_x, lo_y), "013 c2Minv AB"),
                (v(hi_x, lo_y), v(lo_x, hi_y), "014 c2Minv BA"),
                (v(hi_x, hi_y), v(lo_x, lo_y), "015 c2Minv BB"),
            ];
            for (ma, mb, label) in min_cases {
                same(label, cmin(ma, mb), rmin(ma, mb));
            }
            same("015 c2Minv equal", cmin(a, a), rmin(a, a));

            let max_cases = [
                (v(hi_x, hi_y), v(lo_x, lo_y), "016 c2Maxv AA"),
                (v(hi_x, lo_y), v(lo_x, hi_y), "017 c2Maxv AB"),
                (v(lo_x, hi_y), v(hi_x, lo_y), "018 c2Maxv BA"),
                (v(lo_x, lo_y), v(hi_x, hi_y), "019 c2Maxv BB"),
            ];
            for (ma, mb, label) in max_cases {
                same(label, cmax(ma, mb), rmax(ma, mb));
            }
            same("019 c2Maxv equal", cmax(a, a), rmax(a, a));
            same("020 c2Skew", cskew(a), rskew(a));

            let px = rng.range(0.0, 20.0);
            let py = rng.range(0.0, 20.0);
            for (input, label) in [
                (v(px, py), "021 c2Absv ++"),
                (v(-px, py), "022 c2Absv -+"),
                (v(px, -py), "023 c2Absv +-"),
                (v(-px, -py), "024 c2Absv --"),
                (v(-0.0, 0.0), "024 c2Absv signed zero"),
            ] {
                same(label, cabs(input), rabs(input));
            }
        }
    }
}

#[test]
fn configs_036_through_040_and_048_through_052_linear_surface() {
    let (c_lib, r_lib) = libraries();
    unsafe {
        let ccw: VToVFn = symbol(&c_lib, b"c2CCW90\0");
        let rcw: VToVFn = symbol(&r_lib, b"c2CCW90\0");
        let cmat: MatVFn = symbol(&c_lib, b"c2MulmvT\0");
        let rmat: MatVFn = symbol(&r_lib, b"c2MulmvT\0");
        let caabb_point: AabbPointFn = symbol(&c_lib, b"c2AABBtoPoint\0");
        let raabb_point: AabbPointFn = symbol(&r_lib, b"c2AABBtoPoint\0");
        let ccircle_point: CirclePointFn = symbol(&c_lib, b"c2CircleToPoint\0");
        let rcircle_point: CirclePointFn = symbol(&r_lib, b"c2CircleToPoint\0");
        let crot_id: RotFn = symbol(&c_lib, b"c2RotIdentity\0");
        let rrot_id: RotFn = symbol(&r_lib, b"c2RotIdentity\0");
        let cx_id: XformFn = symbol(&c_lib, b"c2xIdentity\0");
        let rx_id: XformFn = symbol(&r_lib, b"c2xIdentity\0");
        let cmulrv: RotVFn = symbol(&c_lib, b"c2Mulrv\0");
        let rmulrv: RotVFn = symbol(&r_lib, b"c2Mulrv\0");
        let cmulrvt: RotVFn = symbol(&c_lib, b"c2MulrvT\0");
        let rmulrvt: RotVFn = symbol(&r_lib, b"c2MulrvT\0");
        let cmulxvt: XformVFn = symbol(&c_lib, b"c2MulxvT\0");
        let rmulxvt: XformVFn = symbol(&r_lib, b"c2MulxvT\0");

        same("048 identity rotation", crot_id(), rrot_id());
        same("049 identity transform", cx_id(), rx_id());

        let mut rng = Rng::new(0x0360_0520_1234_5678);
        for _ in 0..CASES {
            let a = v(rng.signed(), rng.signed());
            same("036 c2CCW90", ccw(a), rcw(a));
            let m = Mat {
                x: v(rng.signed(), rng.signed()),
                y: v(rng.signed(), rng.signed()),
            };
            same("037 c2MulmvT", cmat(m, a), rmat(m, a));

            let cx = rng.range(-10.0, 10.0);
            let cy = rng.range(-10.0, 10.0);
            let hx = rng.range(0.1, 8.0);
            let hy = rng.range(0.1, 8.0);
            let b = box_at(cx, cy, hx, hy);
            let inside = v(
                rng.range(cx - hx * 0.9, cx + hx * 0.9),
                rng.range(cy - hy * 0.9, cy + hy * 0.9),
            );
            same(
                "038 AABB point inside",
                caabb_point(b, inside),
                raabb_point(b, inside),
            );
            for edge in [
                v(cx - hx, cy),
                v(cx + hx, cy),
                v(cx, cy - hy),
                v(cx, cy + hy),
                v(cx + hx, cy + hy),
            ] {
                same(
                    "039 AABB point boundary",
                    caabb_point(b, edge),
                    raabb_point(b, edge),
                );
            }

            let radius = rng.range(0.1, 8.0);
            let circle = Circle {
                p: v(cx, cy),
                r: radius,
            };
            let point = v(cx + rng.range(-0.7, 0.7) * radius, cy);
            same(
                "040 circle point inside",
                ccircle_point(circle, point),
                rcircle_point(circle, point),
            );

            let angle = rng.range(-3.0, 3.0);
            let rot = Rot {
                c: angle.cos(),
                s: angle.sin(),
            };
            same("050 c2Mulrv", cmulrv(rot, a), rmulrv(rot, a));
            same("051 c2MulrvT", cmulrvt(rot, a), rmulrvt(rot, a));
            let xf = Xform {
                p: v(rng.signed(), rng.signed()),
                r: rot,
            };
            same("052 c2MulxvT", cmulxvt(xf, a), rmulxvt(xf, a));
        }
    }
}

#[test]
fn configs_025_through_035_circle_and_aabb_rays() {
    let (c_lib, r_lib) = libraries();
    unsafe {
        let ccircle: RayCircleFn = symbol(&c_lib, b"c2RaytoCircle\0");
        let rcircle: RayCircleFn = symbol(&r_lib, b"c2RaytoCircle\0");
        let caabb_aabb: AabbAabbFn = symbol(&c_lib, b"c2AABBtoAABB\0");
        let raabb_aabb: AabbAabbFn = symbol(&r_lib, b"c2AABBtoAABB\0");
        let caabb: RayAabbFn = symbol(&c_lib, b"c2RaytoAABB\0");
        let raabb: RayAabbFn = symbol(&r_lib, b"c2RaytoAABB\0");

        let mut rng = Rng::new(0x0250_0350_dead_beef);
        for _ in 0..CASES {
            let cx = rng.range(-8.0, 8.0);
            let cy = rng.range(-8.0, 8.0);
            let radius = rng.range(0.25, 4.0);
            let distance = rng.range(0.25, 8.0);
            let circle = Circle {
                p: v(cx, cy),
                r: radius,
            };
            let exact_cx = (rng.u32() % 9) as f32 - 4.0;
            let exact_cy = (rng.u32() % 9) as f32 - 4.0;
            let exact_radius = (rng.u32() % 4 + 1) as f32;
            let exact_distance = (rng.u32() % 5 + 1) as f32;
            let exact_circle = Circle {
                p: v(exact_cx, exact_cy),
                r: exact_radius,
            };
            let circle_cases = [
                (
                    Ray {
                        p: v(cx - radius - distance, cy),
                        d: v(1.0, 0.0),
                        t: distance + radius * 3.0,
                    },
                    circle,
                    "025 circle secant",
                ),
                (
                    Ray {
                        p: v(exact_cx - exact_distance, exact_cy + exact_radius),
                        d: v(1.0, 0.0),
                        t: exact_distance + 1.0,
                    },
                    exact_circle,
                    "026 circle tangent",
                ),
                (
                    Ray {
                        p: v(exact_cx - exact_radius, exact_cy),
                        d: v(1.0, 0.0),
                        t: exact_radius * 2.0,
                    },
                    exact_circle,
                    "027 circle t=0",
                ),
                (
                    Ray {
                        p: v(exact_cx - exact_radius - exact_distance, exact_cy),
                        d: v(1.0, 0.0),
                        t: exact_distance,
                    },
                    exact_circle,
                    "028 circle t=A.t",
                ),
            ];
            for (ray, shape, label) in circle_cases {
                let mut co = sentinel();
                let mut ro = sentinel();
                let cr = ccircle(ray, shape, &mut co);
                let rr = rcircle(ray, shape, &mut ro);
                same(label, cr, rr);
                same(label, co, ro);
                assert_eq!(cr, 1, "{label} should hit");
            }

            let hx = rng.range(0.2, 5.0);
            let hy = rng.range(0.2, 5.0);
            let a = box_at(cx, cy, hx, hy);
            let overlapping = box_at(
                cx + rng.range(-hx * 0.8, hx * 0.8),
                cy + rng.range(-hy * 0.8, hy * 0.8),
                hx,
                hy,
            );
            same(
                "029 AABB overlap",
                caabb_aabb(a, overlapping),
                raabb_aabb(a, overlapping),
            );
            for touching in [
                box_at(cx + hx * 2.0, cy, hx, hy),
                box_at(cx, cy + hy * 2.0, hx, hy),
                box_at(cx + hx * 2.0, cy + hy * 2.0, hx, hy),
            ] {
                same(
                    "030 AABB contact",
                    caabb_aabb(a, touching),
                    raabb_aabb(a, touching),
                );
            }

            let dx = rng.range(0.2, 5.0);
            let dy = rng.range(0.2, 5.0);
            let ray_cases = [
                (
                    Ray {
                        p: v(cx - hx - dx, cy),
                        d: v(1.0, 0.0),
                        t: dx + hx * 3.0,
                    },
                    "031 AABB min-x",
                ),
                (
                    Ray {
                        p: v(cx + hx + dx, cy),
                        d: v(-1.0, 0.0),
                        t: dx + hx * 3.0,
                    },
                    "032 AABB max-x",
                ),
                (
                    Ray {
                        p: v(cx, cy - hy - dy),
                        d: v(0.0, 1.0),
                        t: dy + hy * 3.0,
                    },
                    "033 AABB min-y",
                ),
                (
                    Ray {
                        p: v(cx, cy + hy + dy),
                        d: v(0.0, -1.0),
                        t: dy + hy * 3.0,
                    },
                    "034 AABB max-y",
                ),
                (
                    Ray {
                        p: v(cx - hx - dx, cy - hy - dx),
                        d: v(1.0, 1.0),
                        t: dx + (hx.max(hy) * 3.0),
                    },
                    "035 AABB corner tie",
                ),
            ];
            for (ray, label) in ray_cases {
                let mut co = sentinel();
                let mut ro = sentinel();
                let cr = caabb(ray, a, &mut co);
                let rr = raabb(ray, a, &mut ro);
                same(label, cr, rr);
                same(label, co, ro);
                assert_eq!(cr, 1, "{label} should hit");
            }
        }
    }
}

#[test]
fn configs_041_through_047_capsule_paths() {
    let (c_lib, r_lib) = libraries();
    unsafe {
        let ccap: RayCapsuleFn = symbol(&c_lib, b"c2RaytoCapsule\0");
        let rcap: RayCapsuleFn = symbol(&r_lib, b"c2RaytoCapsule\0");
        let mut rng = Rng::new(0x0410_0470_feed_face);

        for _ in 0..CASES {
            let ox = rng.range(-8.0, 8.0);
            let oy = rng.range(-8.0, 8.0);
            let half = rng.range(0.5, 6.0);
            let radius = rng.range(0.2, 2.0);
            let distance = rng.range(0.2, 5.0);
            let cap = Capsule {
                a: v(ox, oy - half),
                b: v(ox, oy + half),
                r: radius,
            };
            let cases = [
                (
                    Ray {
                        p: v(ox, oy),
                        d: v(1.0, 0.0),
                        t: distance,
                    },
                    "041 capsule starts in body",
                ),
                (
                    Ray {
                        p: v(ox, oy - half - radius * 0.5),
                        d: v(0.0, -1.0),
                        t: distance,
                    },
                    "042 capsule starts in cap A",
                ),
                (
                    Ray {
                        p: v(ox, oy + half + radius * 0.5),
                        d: v(0.0, 1.0),
                        t: distance,
                    },
                    "043 capsule starts in cap B",
                ),
                (
                    Ray {
                        p: v(ox, oy - half - radius - distance),
                        d: v(0.0, 1.0),
                        t: distance + radius * 3.0,
                    },
                    "044 capsule hits cap A",
                ),
                (
                    Ray {
                        p: v(ox, oy + half + radius + distance),
                        d: v(0.0, -1.0),
                        t: distance + radius * 3.0,
                    },
                    "045 capsule hits cap B",
                ),
                (
                    Ray {
                        p: v(ox + radius + distance, oy),
                        d: v(-1.0, 0.0),
                        t: distance + radius * 3.0,
                    },
                    "046 capsule positive side",
                ),
                (
                    Ray {
                        p: v(ox - radius - distance, oy),
                        d: v(1.0, 0.0),
                        t: distance + radius * 3.0,
                    },
                    "047 capsule negative side",
                ),
            ];
            for (ray, label) in cases {
                let mut co = sentinel();
                let mut ro = sentinel();
                let cr = ccap(ray, cap, &mut co);
                let rr = rcap(ray, cap, &mut ro);
                same(label, cr, rr);
                same(label, co, ro);
                assert_eq!(cr, 1, "{label} should hit");
            }
        }
    }
}

fn world_point(xf: Xform, local: V) -> V {
    v(
        xf.p.x + xf.r.c * local.x - xf.r.s * local.y,
        xf.p.y + xf.r.s * local.x + xf.r.c * local.y,
    )
}

fn world_direction(xf: Xform, local: V) -> V {
    v(
        xf.r.c * local.x - xf.r.s * local.y,
        xf.r.s * local.x + xf.r.c * local.y,
    )
}

#[test]
fn configs_053_through_060_polygon_paths() {
    let (c_lib, r_lib) = libraries();
    unsafe {
        let cpoly: RayPolyFn = symbol(&c_lib, b"c2RaytoPoly\0");
        let rpoly: RayPolyFn = symbol(&r_lib, b"c2RaytoPoly\0");
        let mut rng = Rng::new(0x0530_0600_5eed_1234);

        for _ in 0..CASES {
            let hx = rng.range(0.25, 5.0);
            let hy = rng.range(0.25, 5.0);
            let distance = rng.range(0.2, 7.0);
            let p = rect_poly(hx, hy);
            let local_rays = [
                (
                    Ray {
                        p: v(hx + distance, 0.0),
                        d: v(-1.0, 0.0),
                        t: distance + hx * 3.0,
                    },
                    "053 polygon edge 0",
                ),
                (
                    Ray {
                        p: v(0.0, hy + distance),
                        d: v(0.0, -1.0),
                        t: distance + hy * 3.0,
                    },
                    "054 polygon edge 1",
                ),
                (
                    Ray {
                        p: v(-hx - distance, 0.0),
                        d: v(1.0, 0.0),
                        t: distance + hx * 3.0,
                    },
                    "055 polygon edge 2",
                ),
                (
                    Ray {
                        p: v(0.0, -hy - distance),
                        d: v(0.0, 1.0),
                        t: distance + hy * 3.0,
                    },
                    "056 polygon edge 3",
                ),
            ];
            for (ray, label) in local_rays {
                let mut co = sentinel();
                let mut ro = sentinel();
                let cr = cpoly(ray, &p, std::ptr::null(), &mut co);
                let rr = rpoly(ray, &p, std::ptr::null(), &mut ro);
                same(label, cr, rr);
                same(label, co, ro);
                assert_eq!(cr, 1, "{label} should hit");
            }

            let identity = Xform {
                p: v(0.0, 0.0),
                r: Rot { c: 1.0, s: 0.0 },
            };
            let identity_ray = Ray {
                p: v(hx + distance, 0.0),
                d: v(-1.0, 0.0),
                t: distance + hx * 3.0,
            };
            let mut co = sentinel();
            let mut ro = sentinel();
            let cr = cpoly(identity_ray, &p, &identity, &mut co);
            let rr = rpoly(identity_ray, &p, &identity, &mut ro);
            same("057 polygon explicit identity result", cr, rr);
            same("057 polygon explicit identity output", co, ro);

            let translated = Xform {
                p: v(rng.range(-10.0, 10.0), rng.range(-10.0, 10.0)),
                r: Rot { c: 1.0, s: 0.0 },
            };
            let local = Ray {
                p: v(-hx - distance, 0.0),
                d: v(1.0, 0.0),
                t: distance + hx * 3.0,
            };
            let translated_ray = Ray {
                p: world_point(translated, local.p),
                d: world_direction(translated, local.d),
                t: local.t,
            };
            co = sentinel();
            ro = sentinel();
            let cr = cpoly(translated_ray, &p, &translated, &mut co);
            let rr = rpoly(translated_ray, &p, &translated, &mut ro);
            same("058 polygon translated result", cr, rr);
            same("058 polygon translated output", co, ro);

            let angle = rng.range(-2.8, 2.8);
            let rotated = Xform {
                p: v(rng.range(-5.0, 5.0), rng.range(-5.0, 5.0)),
                r: Rot {
                    c: angle.cos(),
                    s: angle.sin(),
                },
            };
            let local = Ray {
                p: v(0.0, hy + distance),
                d: v(0.0, -1.0),
                t: distance + hy * 3.0,
            };
            let rotated_ray = Ray {
                p: world_point(rotated, local.p),
                d: world_direction(rotated, local.d),
                t: local.t,
            };
            co = sentinel();
            ro = sentinel();
            let cr = cpoly(rotated_ray, &p, &rotated, &mut co);
            let rr = rpoly(rotated_ray, &p, &rotated, &mut ro);
            same("059 polygon rotated result", cr, rr);
            same("059 polygon rotated output", co, ro);

            let oct = octagon_poly(rng.range(0.5, 5.0));
            let ray = Ray {
                p: v(oct.verts[0].x + distance, 0.0),
                d: v(-1.0, 0.0),
                t: distance + oct.verts[0].x * 3.0,
            };
            co = sentinel();
            ro = sentinel();
            let cr = cpoly(ray, &oct, std::ptr::null(), &mut co);
            let rr = rpoly(ray, &oct, std::ptr::null(), &mut ro);
            same("060 polygon count 8 result", cr, rr);
            same("060 polygon count 8 output", co, ro);
            assert_eq!(cr, 1);

            let padded = PaddedPoly {
                poly: Poly {
                    count: 9,
                    verts: [v(rng.signed(), rng.signed()); 8],
                    norms: [v(0.0, 0.0); 8],
                },
                ninth_norm: v(0.0, 0.0),
            };
            co = sentinel();
            ro = sentinel();
            let padded_ray = Ray {
                p: v(rng.signed(), rng.signed()),
                d: v(rng.signed(), rng.signed()),
                t: rng.range(0.0, 10.0),
            };
            let cr = cpoly(padded_ray, &padded.poly, std::ptr::null(), &mut co);
            let rr = rpoly(padded_ray, &padded.poly, std::ptr::null(), &mut ro);
            // Zero normals make the ray fields irrelevant; both loops still read index 8.
            same("067 polygon padded count 9 result", cr, rr);
            same("067 polygon padded count 9 output", co, ro);
        }
    }
}

#[test]
fn configs_061_through_066_dispatch_and_composed_api() {
    let (c_lib, r_lib) = libraries();
    unsafe {
        let ccast: CastRayFn = symbol(&c_lib, b"c2CastRay\0");
        let rcast: CastRayFn = symbol(&r_lib, b"c2CastRay\0");
        let cpoly_ray: PolyRayFn = symbol(&c_lib, b"poly_ray\0");
        let rpoly_ray: PolyRayFn = symbol(&r_lib, b"poly_ray\0");
        let mut rng = Rng::new(0x0610_0660_abcd_9876);

        for _ in 0..CASES {
            let radius = rng.range(0.25, 3.0);
            let distance = rng.range(0.25, 5.0);
            let circle = Circle {
                p: v(rng.range(-4.0, 4.0), rng.range(-4.0, 4.0)),
                r: radius,
            };
            let circle_ray = Ray {
                p: v(circle.p.x - radius - distance, circle.p.y),
                d: v(1.0, 0.0),
                t: distance + radius * 3.0,
            };
            let mut co = sentinel();
            let mut ro = sentinel();
            let cr = ccast(
                circle_ray,
                (&circle as *const Circle).cast(),
                std::ptr::null(),
                0,
                &mut co,
            );
            let rr = rcast(
                circle_ray,
                (&circle as *const Circle).cast(),
                std::ptr::null(),
                0,
                &mut ro,
            );
            same("061 circle dispatch result", cr, rr);
            same("061 circle dispatch output", co, ro);

            let bx = rng.range(-4.0, 4.0);
            let by = rng.range(-4.0, 4.0);
            let aabb = box_at(bx, by, radius, radius * 1.5);
            let aabb_ray = Ray {
                p: v(aabb.min.x - distance, by),
                d: v(1.0, 0.0),
                t: distance + radius * 4.0,
            };
            co = sentinel();
            ro = sentinel();
            let cr = ccast(
                aabb_ray,
                (&aabb as *const Aabb).cast(),
                std::ptr::null(),
                1,
                &mut co,
            );
            let rr = rcast(
                aabb_ray,
                (&aabb as *const Aabb).cast(),
                std::ptr::null(),
                1,
                &mut ro,
            );
            same("062 AABB dispatch result", cr, rr);
            same("062 AABB dispatch output", co, ro);

            let cap = Capsule {
                a: v(bx, by - 2.0 * radius),
                b: v(bx, by + 2.0 * radius),
                r: radius,
            };
            let cap_ray = Ray {
                p: v(bx + radius + distance, by),
                d: v(-1.0, 0.0),
                t: distance + radius * 3.0,
            };
            co = sentinel();
            ro = sentinel();
            let cr = ccast(
                cap_ray,
                (&cap as *const Capsule).cast(),
                std::ptr::null(),
                2,
                &mut co,
            );
            let rr = rcast(
                cap_ray,
                (&cap as *const Capsule).cast(),
                std::ptr::null(),
                2,
                &mut ro,
            );
            same("063 capsule dispatch result", cr, rr);
            same("063 capsule dispatch output", co, ro);

            let p = rect_poly(radius, radius * 1.5);
            let poly_ray = Ray {
                p: v(-radius - distance, 0.0),
                d: v(1.0, 0.0),
                t: distance + radius * 3.0,
            };
            co = sentinel();
            ro = sentinel();
            let cr = ccast(
                poly_ray,
                (&p as *const Poly).cast(),
                std::ptr::null(),
                3,
                &mut co,
            );
            let rr = rcast(
                poly_ray,
                (&p as *const Poly).cast(),
                std::ptr::null(),
                3,
                &mut ro,
            );
            same("064 polygon null-transform dispatch result", cr, rr);
            same("064 polygon null-transform dispatch output", co, ro);

            let angle = rng.range(-2.0, 2.0);
            let xf = Xform {
                p: v(rng.range(-5.0, 5.0), rng.range(-5.0, 5.0)),
                r: Rot {
                    c: angle.cos(),
                    s: angle.sin(),
                },
            };
            let world_ray = Ray {
                p: world_point(xf, poly_ray.p),
                d: world_direction(xf, poly_ray.d),
                t: poly_ray.t,
            };
            co = sentinel();
            ro = sentinel();
            let cr = ccast(world_ray, (&p as *const Poly).cast(), &xf, 3, &mut co);
            let rr = rcast(world_ray, (&p as *const Poly).cast(), &xf, 3, &mut ro);
            same("065 polygon transformed dispatch result", cr, rr);
            same("065 polygon transformed dispatch output", co, ro);
        }

        for _ in 0..CASES {
            let mut c1 = sentinel();
            let mut c2 = sentinel();
            let mut r1 = sentinel();
            let mut r2 = sentinel();
            let cr = cpoly_ray(&mut c1, &mut c2);
            let rr = rpoly_ray(&mut r1, &mut r2);
            same("066 poly_ray result", cr, rr);
            same("066 poly_ray cast1", c1, r1);
            same("066 poly_ray cast2", c2, r2);
        }
    }
}

#[test]
fn errors_001_through_016_circle_and_aabb_rejections() {
    let (c_lib, r_lib) = libraries();
    unsafe {
        let ccircle: RayCircleFn = symbol(&c_lib, b"c2RaytoCircle\0");
        let rcircle: RayCircleFn = symbol(&r_lib, b"c2RaytoCircle\0");
        let caabb_aabb: AabbAabbFn = symbol(&c_lib, b"c2AABBtoAABB\0");
        let raabb_aabb: AabbAabbFn = symbol(&r_lib, b"c2AABBtoAABB\0");
        let caabb_ray: RayAabbFn = symbol(&c_lib, b"c2RaytoAABB\0");
        let raabb_ray: RayAabbFn = symbol(&r_lib, b"c2RaytoAABB\0");
        let caabb_point: AabbPointFn = symbol(&c_lib, b"c2AABBtoPoint\0");
        let raabb_point: AabbPointFn = symbol(&r_lib, b"c2AABBtoPoint\0");
        let ccircle_point: CirclePointFn = symbol(&c_lib, b"c2CircleToPoint\0");
        let rcircle_point: CirclePointFn = symbol(&r_lib, b"c2CircleToPoint\0");
        let mut rng = Rng::new(0xe001_0160_1111_2222);

        for _ in 0..CASES {
            let radius = rng.range(0.25, 4.0);
            let gap = rng.range(0.25, 4.0);
            let circle = Circle {
                p: v(0.0, 0.0),
                r: radius,
            };
            let cases = [
                (
                    Ray {
                        p: v(-5.0, radius + gap),
                        d: v(1.0, 0.0),
                        t: 20.0,
                    },
                    "error 001 negative discriminant",
                ),
                (
                    Ray {
                        p: v(-radius - gap, 0.0),
                        d: v(-1.0, 0.0),
                        t: 20.0,
                    },
                    "error 002 t below zero",
                ),
                (
                    Ray {
                        p: v(-radius - gap, 0.0),
                        d: v(1.0, 0.0),
                        t: gap * 0.5,
                    },
                    "error 003 t above A.t",
                ),
            ];
            for (ray, label) in cases {
                let mut co = sentinel();
                let mut ro = sentinel();
                let cr = ccircle(ray, circle, &mut co);
                let rr = rcircle(ray, circle, &mut ro);
                same(label, cr, rr);
                same(label, co, ro);
                assert_eq!(cr, 0, "{label}");
            }

            let a = box_at(0.0, 0.0, 1.0, 1.0);
            let separated = [
                (box_at(-3.0, 0.0, 1.0, 1.0), "error 004 B.max.x < A.min.x"),
                (box_at(3.0, 0.0, 1.0, 1.0), "error 005 A.max.x < B.min.x"),
                (box_at(0.0, -3.0, 1.0, 1.0), "error 006 B.max.y < A.min.y"),
                (box_at(0.0, 3.0, 1.0, 1.0), "error 007 A.max.y < B.min.y"),
            ];
            for (b, label) in separated {
                let cr = caabb_aabb(a, b);
                let rr = raabb_aabb(a, b);
                same(label, cr, rr);
                assert_eq!(cr, 0);
            }

            let broad_miss = Ray {
                p: v(-5.0, 4.0),
                d: v(1.0, 0.0),
                t: 2.0,
            };
            let mut co = sentinel();
            let mut ro = sentinel();
            let cr = caabb_ray(broad_miss, a, &mut co);
            let rr = raabb_ray(broad_miss, a, &mut ro);
            same("error 008 AABB broad miss result", cr, rr);
            same("error 008 AABB broad miss output", co, ro);
            assert_eq!(cr, 0);

            let sat_miss = Ray {
                p: v(-2.0, 0.9),
                d: v(2.9, 1.1),
                t: 1.0,
            };
            co = sentinel();
            ro = sentinel();
            let cr = caabb_ray(sat_miss, a, &mut co);
            let rr = raabb_ray(sat_miss, a, &mut ro);
            same("error 009 AABB SAT miss result", cr, rr);
            same("error 009 AABB SAT miss output", co, ro);
            assert_eq!(cr, 0);

            let nan_ray = Ray {
                p: v(f32::NAN, f32::NAN),
                d: v(0.0, 0.0),
                t: 1.0,
            };
            co = sentinel();
            ro = sentinel();
            let cr = caabb_ray(nan_ray, a, &mut co);
            let rr = raabb_ray(nan_ray, a, &mut ro);
            same("error 010 AABB unordered planes result", cr, rr);
            same("error 010 AABB unordered planes output", co, ro);
            assert_eq!(cr, 0);

            let point_cases = [
                (v(-1.0 - gap, 0.0), "error 011 point below min x"),
                (v(0.0, -1.0 - gap), "error 012 point below min y"),
                (v(1.0 + gap, 0.0), "error 013 point above max x"),
                (v(0.0, 1.0 + gap), "error 014 point above max y"),
            ];
            for (point, label) in point_cases {
                let cr = caabb_point(a, point);
                let rr = raabb_point(a, point);
                same(label, cr, rr);
                assert_eq!(cr, 0);
            }

            let boundary = v(radius, 0.0);
            let outside = v(radius + gap, 0.0);
            let cr = ccircle_point(circle, boundary);
            let rr = rcircle_point(circle, boundary);
            same("error 015 circle boundary", cr, rr);
            assert_eq!(cr, 0);
            let cr = ccircle_point(circle, outside);
            let rr = rcircle_point(circle, outside);
            same("error 016 circle outside", cr, rr);
            assert_eq!(cr, 0);
        }
    }
}

#[test]
fn errors_017_through_022_capsule_and_polygon_rejections() {
    let (c_lib, r_lib) = libraries();
    unsafe {
        let ccap: RayCapsuleFn = symbol(&c_lib, b"c2RaytoCapsule\0");
        let rcap: RayCapsuleFn = symbol(&r_lib, b"c2RaytoCapsule\0");
        let cpoly: RayPolyFn = symbol(&c_lib, b"c2RaytoPoly\0");
        let rpoly: RayPolyFn = symbol(&r_lib, b"c2RaytoPoly\0");
        let mut rng = Rng::new(0xe017_0220_3333_4444);

        for _ in 0..CASES {
            let half = rng.range(1.0, 5.0);
            let radius = rng.range(0.2, 2.0);
            let gap = rng.range(0.2, 4.0);
            let cap = Capsule {
                a: v(0.0, -half),
                b: v(0.0, half),
                r: radius,
            };
            let cap_cases = [
                (
                    Ray {
                        p: v(radius + gap, 0.0),
                        d: v(0.0, 1.0),
                        t: 0.5,
                    },
                    "error 017 capsule misses body and caps",
                ),
                (
                    Ray {
                        p: v(0.0, -half - radius - gap),
                        d: v(0.0, -1.0),
                        t: gap + radius,
                    },
                    "error 018 delegated cap A miss",
                ),
                (
                    Ray {
                        p: v(0.0, half + radius + gap),
                        d: v(0.0, 1.0),
                        t: gap + radius,
                    },
                    "error 019 delegated cap B miss",
                ),
            ];
            for (ray, label) in cap_cases {
                let mut co = sentinel();
                let mut ro = sentinel();
                let cr = ccap(ray, cap, &mut co);
                let rr = rcap(ray, cap, &mut ro);
                same(label, cr, rr);
                same(label, co, ro);
                assert_eq!(cr, 0, "{label}");
            }

            let p = rect_poly(1.0, 1.0);
            let poly_cases = [
                (
                    Ray {
                        p: v(2.0 + gap, 0.0),
                        d: v(0.0, 1.0),
                        t: 10.0,
                    },
                    "error 020 polygon parallel outside",
                ),
                (
                    Ray {
                        p: v(-2.0, 2.0),
                        d: v(1.0, -0.2),
                        t: 10.0,
                    },
                    "error 021 polygon hi below lo",
                ),
            ];
            for (ray, label) in poly_cases {
                let mut co = sentinel();
                let mut ro = sentinel();
                let cr = cpoly(ray, &p, std::ptr::null(), &mut co);
                let rr = rpoly(ray, &p, std::ptr::null(), &mut ro);
                same(label, cr, rr);
                same(label, co, ro);
                assert_eq!(cr, 0, "{label}");
            }

            for (shape, ray, label) in [
                (
                    Poly { count: 0, ..p },
                    Ray {
                        p: v(-2.0, 0.0),
                        d: v(1.0, 0.0),
                        t: 10.0,
                    },
                    "error 022 polygon count zero",
                ),
                (
                    Poly { count: -1, ..p },
                    Ray {
                        p: v(-2.0, 0.0),
                        d: v(1.0, 0.0),
                        t: 10.0,
                    },
                    "error 022 polygon negative count",
                ),
                (
                    p,
                    Ray {
                        p: v(0.0, 0.0),
                        d: v(1.0, 0.0),
                        t: 10.0,
                    },
                    "error 022 polygon starts inside no entry",
                ),
            ] {
                let mut co = sentinel();
                let mut ro = sentinel();
                let cr = cpoly(ray, &shape, std::ptr::null(), &mut co);
                let rr = rpoly(ray, &shape, std::ptr::null(), &mut ro);
                same(label, cr, rr);
                same(label, co, ro);
                assert_eq!(cr, 0, "{label}");
            }
        }
    }
}

#[test]
fn errors_023_through_032_dispatch_boundaries_and_safe_nulls() {
    let (c_lib, r_lib) = libraries();
    unsafe {
        let ccircle: RayCircleFn = symbol(&c_lib, b"c2RaytoCircle\0");
        let rcircle: RayCircleFn = symbol(&r_lib, b"c2RaytoCircle\0");
        let caabb: RayAabbFn = symbol(&c_lib, b"c2RaytoAABB\0");
        let raabb: RayAabbFn = symbol(&r_lib, b"c2RaytoAABB\0");
        let ccap: RayCapsuleFn = symbol(&c_lib, b"c2RaytoCapsule\0");
        let rcap: RayCapsuleFn = symbol(&r_lib, b"c2RaytoCapsule\0");
        let cpoly: RayPolyFn = symbol(&c_lib, b"c2RaytoPoly\0");
        let rpoly: RayPolyFn = symbol(&r_lib, b"c2RaytoPoly\0");
        let ccast: CastRayFn = symbol(&c_lib, b"c2CastRay\0");
        let rcast: CastRayFn = symbol(&r_lib, b"c2CastRay\0");

        let miss_ray = Ray {
            p: v(-5.0, 5.0),
            d: v(1.0, 0.0),
            t: 1.0,
        };
        for invalid in [-1, 4, 255, c_int::MAX, c_int::MIN] {
            let cr = ccast(
                miss_ray,
                std::ptr::null(),
                std::ptr::null(),
                invalid,
                std::ptr::null_mut(),
            );
            let rr = rcast(
                miss_ray,
                std::ptr::null(),
                std::ptr::null(),
                invalid,
                std::ptr::null_mut(),
            );
            same("error 023/032 invalid enum null pointers", cr, rr);
            assert_eq!(cr, 0);
        }

        let circle = Circle {
            p: v(0.0, 0.0),
            r: 1.0,
        };
        let aabb = box_at(0.0, 0.0, 1.0, 1.0);
        let cap = Capsule {
            a: v(0.0, -1.0),
            b: v(0.0, 1.0),
            r: 0.5,
        };
        let poly = rect_poly(1.0, 1.0);
        for (shape, type_b, label) in [
            (
                (&circle as *const Circle).cast::<c_void>(),
                0,
                "error 024 circle dispatch",
            ),
            (
                (&aabb as *const Aabb).cast::<c_void>(),
                1,
                "error 025 AABB dispatch",
            ),
            (
                (&cap as *const Capsule).cast::<c_void>(),
                2,
                "error 026 capsule dispatch",
            ),
            (
                (&poly as *const Poly).cast::<c_void>(),
                3,
                "error 027 polygon dispatch",
            ),
        ] {
            let mut co = sentinel();
            let mut ro = sentinel();
            let cr = ccast(miss_ray, shape, std::ptr::null(), type_b, &mut co);
            let rr = rcast(miss_ray, shape, std::ptr::null(), type_b, &mut ro);
            same(label, cr, rr);
            same(label, co, ro);
            assert_eq!(cr, 0, "{label}");
        }

        let padded = PaddedPoly {
            poly: Poly {
                count: 9,
                verts: [v(0.0, 0.0); 8],
                norms: [v(0.0, 0.0); 8],
            },
            ninth_norm: v(0.0, 0.0),
        };
        let mut co = sentinel();
        let mut ro = sentinel();
        let cr = cpoly(miss_ray, &padded.poly, std::ptr::null(), &mut co);
        let rr = rpoly(miss_ray, &padded.poly, std::ptr::null(), &mut ro);
        same("error 028 padded count 9 result", cr, rr);
        same("error 028 padded count 9 output", co, ro);
        assert_eq!(cr, 0);

        let circle_miss = Ray {
            p: v(-2.0, 2.0),
            d: v(1.0, 0.0),
            t: 4.0,
        };
        same(
            "error 029 circle null out",
            ccircle(circle_miss, circle, std::ptr::null_mut()),
            rcircle(circle_miss, circle, std::ptr::null_mut()),
        );
        same(
            "error 030 AABB null out",
            caabb(miss_ray, aabb, std::ptr::null_mut()),
            raabb(miss_ray, aabb, std::ptr::null_mut()),
        );
        let parallel_outside = Ray {
            p: v(2.0, 0.0),
            d: v(0.0, 1.0),
            t: 4.0,
        };
        same(
            "error 031 polygon null out",
            cpoly(
                parallel_outside,
                &poly,
                std::ptr::null(),
                std::ptr::null_mut(),
            ),
            rpoly(
                parallel_outside,
                &poly,
                std::ptr::null(),
                std::ptr::null_mut(),
            ),
        );

        // Keep the direct capsule symbol in the boundary suite and verify its miss output.
        co = sentinel();
        ro = sentinel();
        let cr = ccap(miss_ray, cap, &mut co);
        let rr = rcap(miss_ray, cap, &mut ro);
        same("capsule direct boundary miss result", cr, rr);
        same("capsule direct boundary miss output", co, ro);
    }
}
