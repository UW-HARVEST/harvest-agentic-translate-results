use libloading::Library;
use std::ffi::{c_int, c_void};
use std::path::{Path, PathBuf};
use std::process::Command;

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
struct Ray {
    p: V,
    d: V,
    t: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct M {
    x: V,
    y: V,
}

type V2 = unsafe extern "C" fn(f32, f32) -> V;
type VVf = unsafe extern "C" fn(V, V) -> f32;
type Vf = unsafe extern "C" fn(V) -> f32;
type VVr = unsafe extern "C" fn(V, V) -> V;
type VFr = unsafe extern "C" fn(V, f32) -> V;
type Vr = unsafe extern "C" fn(V) -> V;
type AabbAabb = unsafe extern "C" fn(Aabb, Aabb) -> c_int;
type AabbPoint = unsafe extern "C" fn(Aabb, V) -> c_int;
type CirclePoint = unsafe extern "C" fn(Circle, V) -> c_int;
type MulmvT = unsafe extern "C" fn(M, V) -> V;
type RayCircle = unsafe extern "C" fn(Ray, Circle, *mut Raycast) -> c_int;
type RayAabb = unsafe extern "C" fn(Ray, Aabb, *mut Raycast) -> c_int;
type RayCapsule = unsafe extern "C" fn(Ray, Capsule, *mut Raycast) -> c_int;
type CastRay = unsafe extern "C" fn(Ray, *const c_void, c_int, *mut Raycast) -> c_int;
type GenRay = unsafe extern "C" fn(
    *mut Raycast,
    *mut Raycast,
    *mut Raycast,
    f32,
    f32,
    f32,
    f32,
    f32,
    f32,
    f32,
    f32,
    f32,
    f32,
    f32,
    f32,
    f32,
    f32,
    f32,
    f32,
) -> c_int;

#[cfg(target_arch = "x86_64")]
core::arch::global_asm!(
    ".global call_cast_seeded",
    ".type call_cast_seeded,@function",
    "call_cast_seeded:",
    "mov r11, rdi",
    "mov rdi, rsi",
    "mov esi, edx",
    "mov rdx, rcx",
    "mov eax, r8d",
    "jmp r11",
    ".size call_cast_seeded, .-call_cast_seeded",
);

#[cfg(target_arch = "x86_64")]
unsafe extern "C" {
    fn call_cast_seeded(
        function: *const c_void,
        ray: Ray,
        shape: *const c_void,
        kind: c_int,
        out: *mut Raycast,
        seed: c_int,
    ) -> c_int;
}

struct Pair {
    c: Library,
    rust: Library,
}

impl Pair {
    unsafe fn load() -> Self {
        Self {
            c: unsafe { Library::new(c_library()) }.unwrap(),
            rust: unsafe { Library::new(rust_library()) }.unwrap(),
        }
    }

    unsafe fn symbols<T: Copy>(&self, name: &[u8]) -> (T, T) {
        (
            *unsafe { self.c.get::<T>(name) }.unwrap(),
            *unsafe { self.rust.get::<T>(name) }.unwrap(),
        )
    }
}

fn c_library() -> PathBuf {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/build");
    std::fs::read_dir(dir)
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| {
            path.extension().is_some_and(|ext| ext == "so")
                && path
                    .file_name()
                    .is_some_and(|name| name != "libgen_ray_lib.so")
        })
        .expect("C shared library")
}

fn rust_library() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("target/release/libgen_ray_lib.so")
}

fn sentinel() -> Raycast {
    Raycast {
        t: f32::from_bits(0x7fc1_2345),
        n: V {
            x: f32::from_bits(0x8000_0000),
            y: f32::from_bits(0x7f80_0000),
        },
    }
}

fn same_f32(label: &str, c: f32, rust: f32) {
    assert_eq!(
        c.to_bits(),
        rust.to_bits(),
        "{label}: C={c:?}/0x{:08x}, Rust={rust:?}/0x{:08x}",
        c.to_bits(),
        rust.to_bits()
    );
}

fn same_v(label: &str, c: V, rust: V) {
    same_f32(&format!("{label}.x"), c.x, rust.x);
    same_f32(&format!("{label}.y"), c.y, rust.y);
}

fn same_cast(label: &str, c_ret: c_int, c: Raycast, r_ret: c_int, rust: Raycast) {
    assert_eq!(c_ret, r_ret, "{label}: return value");
    same_f32(&format!("{label}.t"), c.t, rust.t);
    same_v(&format!("{label}.n"), c.n, rust.n);
}

struct Rng(u64);

impl Rng {
    fn new() -> Self {
        Self(0x6a09_e667_f3bc_c909)
    }

    fn u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x as u32
    }

    fn range(&mut self, low: f32, high: f32) -> f32 {
        let unit = (self.u32() >> 8) as f32 / ((1u32 << 24) - 1) as f32;
        low + (high - low) * unit
    }

    fn v(&mut self) -> V {
        V {
            x: self.range(-50.0, 50.0),
            y: self.range(-50.0, 50.0),
        }
    }
}

unsafe fn compare_ray_circle(
    label: &str,
    c: RayCircle,
    rust: RayCircle,
    ray: Ray,
    circle: Circle,
) -> c_int {
    let mut co = sentinel();
    let mut ro = sentinel();
    let cr = unsafe { c(ray, circle, &mut co) };
    let rr = unsafe { rust(ray, circle, &mut ro) };
    same_cast(label, cr, co, rr, ro);
    cr
}

unsafe fn compare_ray_aabb(label: &str, c: RayAabb, rust: RayAabb, ray: Ray, aabb: Aabb) -> c_int {
    let mut co = sentinel();
    let mut ro = sentinel();
    let cr = unsafe { c(ray, aabb, &mut co) };
    let rr = unsafe { rust(ray, aabb, &mut ro) };
    same_cast(label, cr, co, rr, ro);
    cr
}

unsafe fn compare_ray_capsule(
    label: &str,
    c: RayCapsule,
    rust: RayCapsule,
    ray: Ray,
    capsule: Capsule,
) -> c_int {
    let mut co = sentinel();
    let mut ro = sentinel();
    let cr = unsafe { c(ray, capsule, &mut co) };
    let rr = unsafe { rust(ray, capsule, &mut ro) };
    same_cast(label, cr, co, rr, ro);
    cr
}

#[test]
fn phase_b_low_level_vector_and_predicate_configs() {
    unsafe {
        let pair = Pair::load();
        let (cv, rv) = pair.symbols::<V2>(b"c2V\0");
        let (cdot, rdot) = pair.symbols::<VVf>(b"c2Dot\0");
        let (clen, rlen) = pair.symbols::<Vf>(b"c2Len\0");
        let (cadd, radd) = pair.symbols::<VVr>(b"c2Add\0");
        let (csub, rsub) = pair.symbols::<VVr>(b"c2Sub\0");
        let (cmul, rmul) = pair.symbols::<VFr>(b"c2Mulvs\0");
        let (cdiv, rdiv) = pair.symbols::<VFr>(b"c2Div\0");
        let (cnorm, rnorm) = pair.symbols::<Vr>(b"c2Norm\0");
        let (cmin, rmin) = pair.symbols::<VVr>(b"c2Minv\0");
        let (cmax, rmax) = pair.symbols::<VVr>(b"c2Maxv\0");
        let (cskew, rskew) = pair.symbols::<Vr>(b"c2Skew\0");
        let (cabs, rabs) = pair.symbols::<Vr>(b"c2Absv\0");
        let (cccw, rccw) = pair.symbols::<Vr>(b"c2CCW90\0");
        let (cmm, rmm) = pair.symbols::<MulmvT>(b"c2MulmvT\0");
        let (caa, raa) = pair.symbols::<AabbAabb>(b"c2AABBtoAABB\0");
        let (cap, rap) = pair.symbols::<AabbPoint>(b"c2AABBtoPoint\0");
        let (ccp, rcp) = pair.symbols::<CirclePoint>(b"c2CircleToPoint\0");

        let mut rng = Rng::new();
        for i in 0..512 {
            let a = rng.v();
            let b = rng.v();
            let scalar = match i % 5 {
                0 => -3.0,
                1 => -0.0,
                2 => 0.0,
                3 => 0.25,
                _ => 7.0,
            };
            same_v("row1 c2V", cv(a.x, a.y), rv(a.x, a.y));
            same_f32("row2 c2Dot", cdot(a, b), rdot(a, b));
            same_f32("row3 c2Len", clen(a), rlen(a));
            same_v("row4 c2Add", cadd(a, b), radd(a, b));
            same_v("row4 c2Sub", csub(a, b), rsub(a, b));
            same_v("row5 c2Mulvs", cmul(a, scalar), rmul(a, scalar));
            let divisor = if i % 7 == 0 { 0.0 } else { scalar };
            same_v("row6 c2Div", cdiv(a, divisor), rdiv(a, divisor));
            same_v(
                "row7 c2Norm",
                cnorm(if i % 11 == 0 { V { x: 0.0, y: 0.0 } } else { a }),
                rnorm(if i % 11 == 0 { V { x: 0.0, y: 0.0 } } else { a }),
            );
            same_v("row8 c2Minv", cmin(a, b), rmin(a, b));
            same_v("row9 c2Maxv", cmax(a, b), rmax(a, b));
            same_v("row10 c2Skew", cskew(a), rskew(a));
            same_v("row10 c2CCW90", cccw(a), rccw(a));
            same_v("row11 c2Absv", cabs(a), rabs(a));
            let m = M { x: a, y: b };
            let v = rng.v();
            same_v("row12 c2MulmvT", cmm(m, v), rmm(m, v));
        }

        for i in 0..128 {
            let x = rng.range(-20.0, 20.0);
            let y = rng.range(-20.0, 20.0);
            let w = rng.range(0.5, 8.0);
            let h = rng.range(0.5, 8.0);
            let a = Aabb {
                min: V { x, y },
                max: V { x: x + w, y: y + h },
            };
            let overlap = Aabb {
                min: V {
                    x: x + w * 0.25,
                    y: y + h * 0.25,
                },
                max: V {
                    x: x + w * 1.25,
                    y: y + h * 1.25,
                },
            };
            let touching = if i % 2 == 0 {
                Aabb {
                    min: V {
                        x: a.max.x,
                        y: y + h * 0.2,
                    },
                    max: V {
                        x: a.max.x + w,
                        y: y + h * 0.8,
                    },
                }
            } else {
                Aabb {
                    min: a.max,
                    max: V {
                        x: a.max.x + w,
                        y: a.max.y + h,
                    },
                }
            };
            for (row, b) in [(16, overlap), (17, touching)] {
                assert_eq!(caa(a, b), 1, "CONFIGS row {row}");
                assert_eq!(caa(a, b), raa(a, b), "CONFIGS row {row}");
            }

            let p = match i % 5 {
                0 => V {
                    x: rng.range(a.min.x, a.max.x),
                    y: rng.range(a.min.y, a.max.y),
                },
                1 => V {
                    x: a.min.x,
                    y: rng.range(a.min.y, a.max.y),
                },
                2 => V {
                    x: a.max.x,
                    y: rng.range(a.min.y, a.max.y),
                },
                3 => V {
                    x: rng.range(a.min.x, a.max.x),
                    y: a.min.y,
                },
                _ => a.max,
            };
            assert_eq!(cap(a, p), 1, "CONFIGS row 23");
            assert_eq!(cap(a, p), rap(a, p), "CONFIGS row 23");
        }
        let circle = Circle {
            p: V { x: 1.0, y: 2.0 },
            r: 3.0,
        };
        for _ in 0..128 {
            let angle_proxy = rng.range(-1.0, 1.0);
            let p = V {
                x: 1.0 + angle_proxy,
                y: 2.0 + rng.range(-1.0, 1.0),
            };
            assert_eq!(ccp(circle, p), 1);
            assert_eq!(ccp(circle, p), rcp(circle, p));
        }
    }
}

#[test]
fn phase_b_raycast_configs() {
    unsafe {
        let pair = Pair::load();
        let (crc, rrc) = pair.symbols::<RayCircle>(b"c2RaytoCircle\0");
        let (cra, rra) = pair.symbols::<RayAabb>(b"c2RaytoAABB\0");
        let (crcap, rrcap) = pair.symbols::<RayCapsule>(b"c2RaytoCapsule\0");
        let mut rng = Rng::new();

        for i in 0..128 {
            let cx = (rng.u32() % 21) as f32 - 10.0;
            let cy = (rng.u32() % 21) as f32 - 10.0;
            let radius = [0.5, 1.0, 2.0, 4.0][(rng.u32() % 4) as usize];
            let gap = [1.0, 2.0, 4.0, 8.0][(rng.u32() % 4) as usize];
            let circle = Circle {
                p: V { x: cx, y: cy },
                r: radius,
            };
            let (ray, row) = match i % 3 {
                0 => (
                    Ray {
                        p: V {
                            x: cx - radius - gap,
                            y: cy,
                        },
                        d: V { x: 1.0, y: 0.0 },
                        t: gap + radius * 2.0,
                    },
                    13,
                ),
                1 => (
                    Ray {
                        p: V {
                            x: cx - gap,
                            y: cy + radius,
                        },
                        d: V { x: 1.0, y: 0.0 },
                        t: gap * 2.0,
                    },
                    14,
                ),
                _ => (
                    Ray {
                        p: V {
                            x: cx - radius,
                            y: cy,
                        },
                        d: V { x: 1.0, y: 0.0 },
                        t: radius * 2.0,
                    },
                    15,
                ),
            };
            assert_eq!(
                compare_ray_circle(&format!("row{row}"), crc, rrc, ray, circle),
                1,
                "CONFIGS row {row}"
            );
        }

        for i in 0..256 {
            let x0 = rng.range(-10.0, 10.0);
            let y0 = rng.range(-10.0, 10.0);
            let w = rng.range(0.25, 5.0);
            let h = rng.range(0.25, 5.0);
            let gap = rng.range(0.25, 5.0);
            let b = Aabb {
                min: V { x: x0, y: y0 },
                max: V {
                    x: x0 + w,
                    y: y0 + h,
                },
            };
            let (ray, row) = match i % 5 {
                0 => (
                    Ray {
                        p: V {
                            x: x0 - gap,
                            y: y0 + h * 0.4,
                        },
                        d: V { x: 1.0, y: 0.0 },
                        t: gap + w + 1.0,
                    },
                    18,
                ),
                1 => (
                    Ray {
                        p: V {
                            x: x0 + w + gap,
                            y: y0 + h * 0.6,
                        },
                        d: V { x: -1.0, y: 0.0 },
                        t: gap + w + 1.0,
                    },
                    19,
                ),
                2 => (
                    Ray {
                        p: V {
                            x: x0 + w * 0.4,
                            y: y0 - gap,
                        },
                        d: V { x: 0.0, y: 1.0 },
                        t: gap + h + 1.0,
                    },
                    20,
                ),
                3 => (
                    Ray {
                        p: V {
                            x: x0 + w * 0.6,
                            y: y0 + h + gap,
                        },
                        d: V { x: 0.0, y: -1.0 },
                        t: gap + h + 1.0,
                    },
                    21,
                ),
                _ => (
                    Ray {
                        p: V {
                            x: x0 - gap,
                            y: y0 - gap,
                        },
                        d: V { x: 1.0, y: 1.0 },
                        t: gap + w.min(h) + 1.0,
                    },
                    22,
                ),
            };
            assert_eq!(compare_ray_aabb(&format!("row{row}"), cra, rra, ray, b), 1);
        }

        for _ in 0..64 {
            let x = rng.range(-20.0, 20.0);
            let y = rng.range(-20.0, 20.0);
            let len = rng.range(5.0, 15.0);
            let r = rng.range(0.25, 2.0);
            let capsule = Capsule {
                a: V { x, y },
                b: V { x, y: y + len },
                r,
            };
            let cases = [
                (
                    25,
                    Ray {
                        p: V {
                            x,
                            y: y + len * 0.5,
                        },
                        d: V { x: 1.0, y: 0.0 },
                        t: r * 2.0,
                    },
                ),
                (
                    26,
                    Ray {
                        p: V { x, y: y - r * 0.5 },
                        d: V { x: 0.0, y: -1.0 },
                        t: r * 2.0,
                    },
                ),
                (
                    27,
                    Ray {
                        p: V {
                            x,
                            y: y + len + r * 0.5,
                        },
                        d: V { x: 0.0, y: 1.0 },
                        t: r * 2.0,
                    },
                ),
                (
                    28,
                    Ray {
                        p: V {
                            x: x + r * 0.5,
                            y: y - r * 2.0,
                        },
                        d: V { x: 0.0, y: 1.0 },
                        t: r * 5.0,
                    },
                ),
                (
                    29,
                    Ray {
                        p: V {
                            x: x + r * 0.5,
                            y: y + len + r * 2.0,
                        },
                        d: V { x: 0.0, y: -1.0 },
                        t: r * 5.0,
                    },
                ),
                (
                    30,
                    Ray {
                        p: V {
                            x: x - r * 3.0,
                            y: y - r * 0.5,
                        },
                        d: V { x: 1.0, y: 0.0 },
                        t: r * 6.0,
                    },
                ),
                (
                    31,
                    Ray {
                        p: V {
                            x: x - r * 3.0,
                            y: y + len + r * 0.5,
                        },
                        d: V { x: 1.0, y: 0.0 },
                        t: r * 6.0,
                    },
                ),
                (
                    32,
                    Ray {
                        p: V {
                            x: x + r * 3.0,
                            y: y + len * 0.5,
                        },
                        d: V { x: -1.0, y: 0.0 },
                        t: r * 6.0,
                    },
                ),
                (
                    33,
                    Ray {
                        p: V {
                            x: x - r * 3.0,
                            y: y + len * 0.5,
                        },
                        d: V { x: 1.0, y: 0.0 },
                        t: r * 6.0,
                    },
                ),
            ];
            for (row, ray) in cases {
                assert_eq!(
                    compare_ray_capsule(&format!("row{row}"), crcap, rrcap, ray, capsule),
                    1
                );
            }
        }
    }
}

#[test]
fn phase_b_dispatch_and_gen_ray_configs() {
    unsafe {
        let pair = Pair::load();
        let (ccast, rcast) = pair.symbols::<CastRay>(b"c2CastRay\0");
        let (cgen, rgen) = pair.symbols::<GenRay>(b"gen_ray\0");
        let ray = Ray {
            p: V { x: 0.0, y: 0.0 },
            d: V { x: 1.0, y: 0.0 },
            t: 10.0,
        };
        let circle = Circle {
            p: V { x: 3.0, y: 0.0 },
            r: 0.5,
        };
        let capsule = Capsule {
            a: V { x: 5.0, y: -1.0 },
            b: V { x: 5.0, y: 1.0 },
            r: 0.5,
        };
        let aabb = Aabb {
            min: V { x: 7.0, y: -0.5 },
            max: V { x: 8.0, y: 0.5 },
        };
        for _ in 0..128 {
            for (row, kind, ptr) in [
                (34, 0, (&circle as *const Circle).cast::<c_void>()),
                (35, 1, (&aabb as *const Aabb).cast::<c_void>()),
                (36, 2, (&capsule as *const Capsule).cast::<c_void>()),
            ] {
                let mut co = sentinel();
                let mut ro = sentinel();
                let cr = ccast(ray, ptr, kind, &mut co);
                let rr = rcast(ray, ptr, kind, &mut ro);
                same_cast(&format!("row{row}"), cr, co, rr, ro);
                assert_eq!(cr, 1);
            }
        }

        for mask in 0..8 {
            for jitter in 0..64 {
                let j = jitter as f32 / 1000.0;
                let cy = if mask & 1 != 0 { 0.0 } else { 100.0 };
                let cap_y = if mask & 2 != 0 { 0.0 } else { 100.0 };
                let box_y = if mask & 4 != 0 { 0.0 } else { 100.0 };
                let mut co1 = sentinel();
                let mut co2 = sentinel();
                let mut co3 = sentinel();
                let mut ro1 = sentinel();
                let mut ro2 = sentinel();
                let mut ro3 = sentinel();
                let cr = cgen(
                    &mut co1,
                    &mut co2,
                    &mut co3,
                    10.0,
                    0.0,
                    0.0,
                    0.0,
                    3.0 + j,
                    cy,
                    0.5,
                    5.0 + j,
                    cap_y - 1.0,
                    5.0 + j,
                    cap_y + 1.0,
                    0.5,
                    7.0 + j,
                    box_y - 0.5,
                    8.0 + j,
                    box_y + 0.5,
                );
                let rr = rgen(
                    &mut ro1,
                    &mut ro2,
                    &mut ro3,
                    10.0,
                    0.0,
                    0.0,
                    0.0,
                    3.0 + j,
                    cy,
                    0.5,
                    5.0 + j,
                    cap_y - 1.0,
                    5.0 + j,
                    cap_y + 1.0,
                    0.5,
                    7.0 + j,
                    box_y - 0.5,
                    8.0 + j,
                    box_y + 0.5,
                );
                assert_eq!(cr, mask, "row{} C mask", 37 + mask);
                assert_eq!(rr, mask, "row{} Rust mask", 37 + mask);
                same_cast(
                    &format!("row{} circle", 37 + mask),
                    cr & 1,
                    co1,
                    rr & 1,
                    ro1,
                );
                same_cast(
                    &format!("row{} capsule", 37 + mask),
                    (cr >> 1) & 1,
                    co2,
                    (rr >> 1) & 1,
                    ro2,
                );
                same_cast(
                    &format!("row{} aabb", 37 + mask),
                    (cr >> 2) & 1,
                    co3,
                    (rr >> 2) & 1,
                    ro3,
                );
            }
        }
    }
}

#[test]
fn phase_b_ieee_special_values() {
    unsafe {
        let pair = Pair::load();
        let (cv, rv) = pair.symbols::<V2>(b"c2V\0");
        let (cdot, rdot) = pair.symbols::<VVf>(b"c2Dot\0");
        let (clen, rlen) = pair.symbols::<Vf>(b"c2Len\0");
        let (cadd, radd) = pair.symbols::<VVr>(b"c2Add\0");
        let (csub, rsub) = pair.symbols::<VVr>(b"c2Sub\0");
        let (cmul, rmul) = pair.symbols::<VFr>(b"c2Mulvs\0");
        let (cdiv, rdiv) = pair.symbols::<VFr>(b"c2Div\0");
        let (cnorm, rnorm) = pair.symbols::<Vr>(b"c2Norm\0");
        let (cmin, rmin) = pair.symbols::<VVr>(b"c2Minv\0");
        let (cmax, rmax) = pair.symbols::<VVr>(b"c2Maxv\0");
        let (cskew, rskew) = pair.symbols::<Vr>(b"c2Skew\0");
        let (cccw, rccw) = pair.symbols::<Vr>(b"c2CCW90\0");
        let (cabs, rabs) = pair.symbols::<Vr>(b"c2Absv\0");
        let (cmm, rmm) = pair.symbols::<MulmvT>(b"c2MulmvT\0");
        let (caa, raa) = pair.symbols::<AabbAabb>(b"c2AABBtoAABB\0");
        let (cap, rap) = pair.symbols::<AabbPoint>(b"c2AABBtoPoint\0");
        let (ccp, rcp) = pair.symbols::<CirclePoint>(b"c2CircleToPoint\0");
        let (crc, rrc) = pair.symbols::<RayCircle>(b"c2RaytoCircle\0");
        let (cra, rra) = pair.symbols::<RayAabb>(b"c2RaytoAABB\0");
        let (crcap, rrcap) = pair.symbols::<RayCapsule>(b"c2RaytoCapsule\0");
        let (ccast, rcast) = pair.symbols::<CastRay>(b"c2CastRay\0");
        let (cgen, rgen) = pair.symbols::<GenRay>(b"gen_ray\0");
        let values = [
            0.0,
            -0.0,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::from_bits(0x7fc0_0001),
            f32::from_bits(0xffc0_1234),
            f32::from_bits(0x7f80_0001),
            f32::MIN_POSITIVE,
            f32::from_bits(1),
            f32::MAX,
            -1.0,
            1.0,
        ];
        for &x in &values {
            for &y in &values {
                let a = V { x, y };
                let b = V { x: y, y: x };
                same_v("row45 c2V", cv(x, y), rv(x, y));
                same_f32("row45 c2Dot", cdot(a, b), rdot(a, b));
                same_f32("row45 c2Len", clen(a), rlen(a));
                same_v("row45 c2Add", cadd(a, b), radd(a, b));
                same_v("row45 c2Sub", csub(a, b), rsub(a, b));
                same_v("row45 c2Mulvs", cmul(a, y), rmul(a, y));
                same_v("row45 c2Div", cdiv(a, y), rdiv(a, y));
                same_v("row45 c2Norm", cnorm(a), rnorm(a));
                same_v("row45 c2Minv", cmin(a, b), rmin(a, b));
                same_v("row45 c2Maxv", cmax(a, b), rmax(a, b));
                same_v("row45 c2Skew", cskew(a), rskew(a));
                same_v("row45 c2CCW90", cccw(a), rccw(a));
                same_v("row45 c2Absv", cabs(a), rabs(a));
                same_v(
                    "row45 c2MulmvT",
                    cmm(M { x: a, y: b }, b),
                    rmm(M { x: a, y: b }, b),
                );

                let aabb = Aabb { min: a, max: b };
                let circle = Circle { p: a, r: y };
                let capsule = Capsule { a, b, r: y };
                let ray = Ray { p: a, d: b, t: x };
                assert_eq!(
                    caa(aabb, Aabb { min: b, max: a }),
                    raa(aabb, Aabb { min: b, max: a }),
                    "row45 c2AABBtoAABB"
                );
                assert_eq!(cap(aabb, b), rap(aabb, b), "row45 c2AABBtoPoint");
                assert_eq!(ccp(circle, b), rcp(circle, b), "row45 c2CircleToPoint");
                compare_ray_circle("row45 c2RaytoCircle", crc, rrc, ray, circle);
                compare_ray_aabb("row45 c2RaytoAABB", cra, rra, ray, aabb);
                compare_ray_capsule("row45 c2RaytoCapsule", crcap, rrcap, ray, capsule);

                for (kind, shape) in [
                    (0, (&circle as *const Circle).cast::<c_void>()),
                    (1, (&aabb as *const Aabb).cast::<c_void>()),
                    (2, (&capsule as *const Capsule).cast::<c_void>()),
                ] {
                    let mut co = sentinel();
                    let mut ro = sentinel();
                    let cr = ccast(ray, shape, kind, &mut co);
                    let rr = rcast(ray, shape, kind, &mut ro);
                    same_cast("row45 c2CastRay", cr, co, rr, ro);
                }

                let mut co1 = sentinel();
                let mut co2 = sentinel();
                let mut co3 = sentinel();
                let mut ro1 = sentinel();
                let mut ro2 = sentinel();
                let mut ro3 = sentinel();
                let cr = cgen(
                    &mut co1, &mut co2, &mut co3, x, y, y, x, x, y, y, x, y, y, x, y, x, y, y, x,
                );
                let rr = rgen(
                    &mut ro1, &mut ro2, &mut ro3, x, y, y, x, x, y, y, x, y, y, x, y, x, y, y, x,
                );
                assert_eq!(cr, rr, "row45 gen_ray return");
                same_cast("row45 gen_ray circle", cr & 1, co1, rr & 1, ro1);
                same_cast(
                    "row45 gen_ray capsule",
                    (cr >> 1) & 1,
                    co2,
                    (rr >> 1) & 1,
                    ro2,
                );
                same_cast("row45 gen_ray aabb", (cr >> 2) & 1, co3, (rr >> 2) & 1, ro3);
            }
        }
    }
}

#[test]
fn phase_b_randomized_full_geometric_surface() {
    unsafe {
        let pair = Pair::load();
        let (caa, raa) = pair.symbols::<AabbAabb>(b"c2AABBtoAABB\0");
        let (cap, rap) = pair.symbols::<AabbPoint>(b"c2AABBtoPoint\0");
        let (ccp, rcp) = pair.symbols::<CirclePoint>(b"c2CircleToPoint\0");
        let (crc, rrc) = pair.symbols::<RayCircle>(b"c2RaytoCircle\0");
        let (cra, rra) = pair.symbols::<RayAabb>(b"c2RaytoAABB\0");
        let (crcap, rrcap) = pair.symbols::<RayCapsule>(b"c2RaytoCapsule\0");
        let (ccast, rcast) = pair.symbols::<CastRay>(b"c2CastRay\0");
        let mut rng = Rng::new();

        for i in 0..4096 {
            let p = rng.v();
            let d = rng.v();
            let ray = Ray {
                p,
                d,
                t: rng.range(-5.0, 30.0),
            };
            let circle = Circle {
                p: rng.v(),
                r: rng.range(-3.0, 8.0),
            };
            let aabb = Aabb {
                min: rng.v(),
                max: rng.v(),
            };
            let mut end = rng.v();
            if end.x == p.x && end.y == p.y {
                end.x += 1.0;
            }
            let capsule = Capsule {
                a: rng.v(),
                b: end,
                r: rng.range(-3.0, 8.0),
            };

            assert_eq!(
                caa(aabb, Aabb { min: p, max: d }),
                raa(aabb, Aabb { min: p, max: d }),
                "random AABB {i}"
            );
            assert_eq!(cap(aabb, p), rap(aabb, p), "random point {i}");
            assert_eq!(ccp(circle, p), rcp(circle, p), "random circle point {i}");
            compare_ray_circle(&format!("random circle {i}"), crc, rrc, ray, circle);
            compare_ray_aabb(&format!("random aabb {i}"), cra, rra, ray, aabb);
            compare_ray_capsule(&format!("random capsule {i}"), crcap, rrcap, ray, capsule);

            for (kind, shape) in [
                (0, (&circle as *const Circle).cast::<c_void>()),
                (1, (&aabb as *const Aabb).cast::<c_void>()),
                (2, (&capsule as *const Capsule).cast::<c_void>()),
            ] {
                let mut co = sentinel();
                let mut ro = sentinel();
                let cr = ccast(ray, shape, kind, &mut co);
                let rr = rcast(ray, shape, kind, &mut ro);
                same_cast(&format!("random dispatch {i}/{kind}"), cr, co, rr, ro);
            }
        }
    }
}

#[test]
fn phase_c_rejection_rows() {
    unsafe {
        let pair = Pair::load();
        let (crc, rrc) = pair.symbols::<RayCircle>(b"c2RaytoCircle\0");
        let (caa, raa) = pair.symbols::<AabbAabb>(b"c2AABBtoAABB\0");
        let (cra, rra) = pair.symbols::<RayAabb>(b"c2RaytoAABB\0");
        let (cap, rap) = pair.symbols::<AabbPoint>(b"c2AABBtoPoint\0");
        let (ccp, rcp) = pair.symbols::<CirclePoint>(b"c2CircleToPoint\0");
        let (crcap, rrcap) = pair.symbols::<RayCapsule>(b"c2RaytoCapsule\0");
        let circle = Circle {
            p: V { x: 0.0, y: 0.0 },
            r: 1.0,
        };
        let circle_rejections = [
            (
                1,
                Ray {
                    p: V { x: -3.0, y: 2.0 },
                    d: V { x: 1.0, y: 0.0 },
                    t: 10.0,
                },
            ),
            (
                2,
                Ray {
                    p: V { x: -3.0, y: 0.0 },
                    d: V { x: -1.0, y: 0.0 },
                    t: 10.0,
                },
            ),
            (
                3,
                Ray {
                    p: V { x: -3.0, y: 0.0 },
                    d: V { x: 1.0, y: 0.0 },
                    t: 1.0,
                },
            ),
        ];
        for _ in 0..128 {
            for (row, ray) in circle_rejections {
                assert_eq!(
                    compare_ray_circle(&format!("error row{row}"), crc, rrc, ray, circle),
                    0
                );
            }
        }

        let a = Aabb {
            min: V { x: 0.0, y: 0.0 },
            max: V { x: 1.0, y: 1.0 },
        };
        let separated = [
            (
                4,
                Aabb {
                    min: V { x: -3.0, y: 0.0 },
                    max: V { x: -1.0, y: 1.0 },
                },
            ),
            (
                5,
                Aabb {
                    min: V { x: 2.0, y: 0.0 },
                    max: V { x: 3.0, y: 1.0 },
                },
            ),
            (
                6,
                Aabb {
                    min: V { x: 0.0, y: -3.0 },
                    max: V { x: 1.0, y: -1.0 },
                },
            ),
            (
                7,
                Aabb {
                    min: V { x: 0.0, y: 2.0 },
                    max: V { x: 1.0, y: 3.0 },
                },
            ),
        ];
        for (row, b) in separated {
            assert_eq!(caa(a, b), 0, "error row{row}");
            assert_eq!(caa(a, b), raa(a, b), "error row{row}");
        }

        let ray_aabb_rejections = [
            (
                8,
                Ray {
                    p: V { x: -3.0, y: 3.0 },
                    d: V { x: -1.0, y: 0.0 },
                    t: 1.0,
                },
            ),
            (
                9,
                Ray {
                    p: V { x: -2.0, y: 0.5 },
                    d: V { x: 2.5, y: 1.5 },
                    t: 1.0,
                },
            ),
            (
                10,
                Ray {
                    p: V {
                        x: f32::NAN,
                        y: f32::NAN,
                    },
                    d: V {
                        x: f32::NAN,
                        y: f32::NAN,
                    },
                    t: 1.0,
                },
            ),
        ];
        for (row, ray) in ray_aabb_rejections {
            assert_eq!(
                compare_ray_aabb(&format!("error row{row}"), cra, rra, ray, a),
                0
            );
        }

        for (row, p) in [
            (11, V { x: -1.0, y: 0.5 }),
            (12, V { x: 0.5, y: -1.0 }),
            (13, V { x: 2.0, y: 0.5 }),
            (14, V { x: 0.5, y: 2.0 }),
        ] {
            assert_eq!(cap(a, p), 0, "error row{row}");
            assert_eq!(cap(a, p), rap(a, p), "error row{row}");
        }
        for p in [V { x: 1.0, y: 0.0 }, V { x: 2.0, y: 0.0 }] {
            assert_eq!(ccp(circle, p), 0, "error row15");
            assert_eq!(ccp(circle, p), rcp(circle, p), "error row15");
        }

        let capsule = Capsule {
            a: V { x: 0.0, y: 0.0 },
            b: V { x: 0.0, y: 10.0 },
            r: 1.0,
        };
        let capsule_rejections = [
            (
                16,
                Ray {
                    p: V { x: 3.0, y: 5.0 },
                    d: V { x: 1.0, y: 0.0 },
                    t: 2.0,
                },
            ),
            (
                17,
                Ray {
                    p: V { x: 0.5, y: -2.0 },
                    d: V { x: 0.0, y: -1.0 },
                    t: 5.0,
                },
            ),
            (
                18,
                Ray {
                    p: V { x: 0.5, y: 12.0 },
                    d: V { x: 0.0, y: 1.0 },
                    t: 5.0,
                },
            ),
            (
                19,
                Ray {
                    p: V { x: -3.0, y: -1.5 },
                    d: V { x: 1.0, y: 0.0 },
                    t: 6.0,
                },
            ),
            (
                20,
                Ray {
                    p: V { x: -3.0, y: 11.5 },
                    d: V { x: 1.0, y: 0.0 },
                    t: 6.0,
                },
            ),
        ];
        for _ in 0..64 {
            for (row, ray) in capsule_rejections {
                assert_eq!(
                    compare_ray_capsule(&format!("error row{row}"), crcap, rrcap, ray, capsule),
                    0
                );
            }
        }

        let (ccast, rcast) = pair.symbols::<CastRay>(b"c2CastRay\0");
        let shape = Circle {
            p: V { x: 2.0, y: 0.0 },
            r: 1.0,
        };
        #[cfg(target_arch = "x86_64")]
        for invalid in [3, -1, c_int::MAX] {
            for seed in [0, 1, 0x1234_5678, -1] {
                let mut co = sentinel();
                let mut ro = sentinel();
                let cr = call_cast_seeded(
                    ccast as *const () as *const c_void,
                    ray_aabb_rejections[0].1,
                    (&shape as *const Circle).cast(),
                    invalid,
                    &mut co,
                    seed,
                );
                let rr = call_cast_seeded(
                    rcast as *const () as *const c_void,
                    ray_aabb_rejections[0].1,
                    (&shape as *const Circle).cast(),
                    invalid,
                    &mut ro,
                    seed,
                );
                assert_eq!(cr, seed, "C error row21 invalid enum {invalid}");
                assert_eq!(rr, seed, "Rust error row21 invalid enum {invalid}");
                same_v("error row21 untouched normal", co.n, ro.n);
                same_f32("error row21 untouched t", co.t, ro.t);
            }
        }
    }
}

fn crash_status(lib: &Path, case: &str) -> std::process::ExitStatus {
    Command::new(std::env::current_exe().unwrap())
        .arg("--exact")
        .arg("ffi_child_crash")
        .arg("--nocapture")
        .env("DIFF_CRASH_LIB", lib)
        .env("DIFF_CRASH_CASE", case)
        .status()
        .unwrap()
}

#[test]
fn phase_c_null_pointer_boundaries() {
    use std::os::unix::process::ExitStatusExt;
    for case in [
        "circle_out",
        "aabb_out",
        "capsule_out",
        "cast_shape",
        "gen_out",
    ] {
        let c = crash_status(&c_library(), case);
        let rust = crash_status(&rust_library(), case);
        assert!(!c.success(), "C unexpectedly survived {case}");
        assert!(!rust.success(), "Rust unexpectedly survived {case}");
        assert_eq!(c.signal(), rust.signal(), "error rows22-24 case {case}");
    }
}

#[test]
fn ffi_child_crash() {
    let Ok(path) = std::env::var("DIFF_CRASH_LIB") else {
        return;
    };
    let case = std::env::var("DIFF_CRASH_CASE").unwrap();
    unsafe {
        let lib = Library::new(path).unwrap();
        let ray = Ray {
            p: V { x: 0.0, y: 0.0 },
            d: V { x: 1.0, y: 0.0 },
            t: 10.0,
        };
        match case.as_str() {
            "circle_out" => {
                let f = *lib.get::<RayCircle>(b"c2RaytoCircle\0").unwrap();
                f(
                    ray,
                    Circle {
                        p: V { x: 2.0, y: 0.0 },
                        r: 1.0,
                    },
                    std::ptr::null_mut(),
                );
            }
            "aabb_out" => {
                let f = *lib.get::<RayAabb>(b"c2RaytoAABB\0").unwrap();
                f(
                    ray,
                    Aabb {
                        min: V { x: 2.0, y: -1.0 },
                        max: V { x: 3.0, y: 1.0 },
                    },
                    std::ptr::null_mut(),
                );
            }
            "capsule_out" => {
                let f = *lib.get::<RayCapsule>(b"c2RaytoCapsule\0").unwrap();
                f(
                    ray,
                    Capsule {
                        a: V { x: 2.0, y: -1.0 },
                        b: V { x: 2.0, y: 1.0 },
                        r: 0.5,
                    },
                    std::ptr::null_mut(),
                );
            }
            "cast_shape" => {
                let f = *lib.get::<CastRay>(b"c2CastRay\0").unwrap();
                let mut out = sentinel();
                f(ray, std::ptr::null(), 0, &mut out);
            }
            "gen_out" => {
                let f = *lib.get::<GenRay>(b"gen_ray\0").unwrap();
                let mut one = sentinel();
                let mut three = sentinel();
                f(
                    &mut one,
                    std::ptr::null_mut(),
                    &mut three,
                    10.0,
                    0.0,
                    0.0,
                    0.0,
                    30.0,
                    100.0,
                    1.0,
                    5.0,
                    -1.0,
                    5.0,
                    1.0,
                    0.5,
                    70.0,
                    100.0,
                    80.0,
                    101.0,
                );
            }
            _ => panic!("unknown crash case"),
        }
    }
    panic!("FFI call unexpectedly survived");
}
