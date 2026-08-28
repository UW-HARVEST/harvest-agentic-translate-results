use libloading::Library;
use std::ffi::{c_float, c_int, c_void};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct C2v {
    x: c_float,
    y: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct C2Raycast {
    t: c_float,
    n: C2v,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct C2Circle {
    p: C2v,
    r: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct C2Aabb {
    min: C2v,
    max: C2v,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct C2Capsule {
    a: C2v,
    b: C2v,
    r: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct C2Ray {
    p: C2v,
    d: C2v,
    t: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct C2m {
    x: C2v,
    y: C2v,
}

type FnVff = unsafe extern "C" fn(c_float, c_float) -> C2v;
type FnFv = unsafe extern "C" fn(C2v) -> c_float;
type FnFvv = unsafe extern "C" fn(C2v, C2v) -> c_float;
type FnVv = unsafe extern "C" fn(C2v) -> C2v;
type FnVvv = unsafe extern "C" fn(C2v, C2v) -> C2v;
type FnVvf = unsafe extern "C" fn(C2v, c_float) -> C2v;
type FnVmv = unsafe extern "C" fn(C2m, C2v) -> C2v;
type FnIaa = unsafe extern "C" fn(C2Aabb, C2Aabb) -> c_int;
type FnIav = unsafe extern "C" fn(C2Aabb, C2v) -> c_int;
type FnIcv = unsafe extern "C" fn(C2Circle, C2v) -> c_int;
type FnIrc = unsafe extern "C" fn(C2Ray, C2Circle, *mut C2Raycast) -> c_int;
type FnIra = unsafe extern "C" fn(C2Ray, C2Aabb, *mut C2Raycast) -> c_int;
type FnIrk = unsafe extern "C" fn(C2Ray, C2Capsule, *mut C2Raycast) -> c_int;
type FnCast = unsafe extern "C" fn(C2Ray, *const c_void, c_int, *mut C2Raycast) -> c_int;
type FnSpec = unsafe extern "C" fn(
    *mut C2Raycast,
    c_float,
    c_float,
    c_float,
    c_float,
    c_float,
    c_float,
    c_float,
) -> c_int;

struct Api {
    _lib: Library,
    c2_v: FnVff,
    c2_dot: FnFvv,
    c2_len: FnFv,
    c2_add: FnVvv,
    c2_sub: FnVvv,
    c2_mulvs: FnVvf,
    c2_div: FnVvf,
    c2_norm: FnVv,
    c2_minv: FnVvv,
    c2_maxv: FnVvv,
    c2_skew: FnVv,
    c2_absv: FnVv,
    ray_circle: FnIrc,
    aabb_aabb: FnIaa,
    ray_aabb: FnIra,
    c2_ccw90: FnVv,
    c2_mulmvt: FnVmv,
    aabb_point: FnIav,
    circle_point: FnIcv,
    ray_capsule: FnIrk,
    cast_ray: FnCast,
    spec_ray: FnSpec,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        // SAFETY: Every copied symbol has the C ABI signature from src/lib.c.
        unsafe {
            let lib = Library::new(path)
                .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
            macro_rules! sym {
                ($name:literal, $ty:ty) => {
                    *lib.get::<$ty>(concat!($name, "\0").as_bytes())
                        .unwrap_or_else(|error| {
                            panic!("missing {} in {}: {error}", $name, path.display())
                        })
                };
            }
            Self {
                c2_v: sym!("c2V", FnVff),
                c2_dot: sym!("c2Dot", FnFvv),
                c2_len: sym!("c2Len", FnFv),
                c2_add: sym!("c2Add", FnVvv),
                c2_sub: sym!("c2Sub", FnVvv),
                c2_mulvs: sym!("c2Mulvs", FnVvf),
                c2_div: sym!("c2Div", FnVvf),
                c2_norm: sym!("c2Norm", FnVv),
                c2_minv: sym!("c2Minv", FnVvv),
                c2_maxv: sym!("c2Maxv", FnVvv),
                c2_skew: sym!("c2Skew", FnVv),
                c2_absv: sym!("c2Absv", FnVv),
                ray_circle: sym!("c2RaytoCircle", FnIrc),
                aabb_aabb: sym!("c2AABBtoAABB", FnIaa),
                ray_aabb: sym!("c2RaytoAABB", FnIra),
                c2_ccw90: sym!("c2CCW90", FnVv),
                c2_mulmvt: sym!("c2MulmvT", FnVmv),
                aabb_point: sym!("c2AABBtoPoint", FnIav),
                circle_point: sym!("c2CircleToPoint", FnIcv),
                ray_capsule: sym!("c2RaytoCapsule", FnIrk),
                cast_ray: sym!("c2CastRay", FnCast),
                spec_ray: sym!("spec_ray", FnSpec),
                _lib: lib,
            }
        }
    }
}

struct Pair {
    c: Api,
    rust: Api,
}

impl Pair {
    fn load() -> Self {
        let c = c_library_path();
        let rust = rust_library_path();
        assert!(c.is_file(), "C library is missing: {}", c.display());
        assert!(
            rust.is_file(),
            "release Rust library is missing: {}; run cargo build --release",
            rust.display()
        );
        // SAFETY: Api::load validates all expected symbols while the libraries
        // remain owned by the returned Api values.
        unsafe {
            Self {
                c: Api::load(&c),
                rust: Api::load(&rust),
            }
        }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    let directory = manifest_dir().join("../c_src/build");
    let mut libraries: Vec<_> = std::fs::read_dir(&directory)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", directory.display()))
        .map(|entry| entry.expect("invalid C build directory entry").path())
        .filter(|path| {
            path.extension().is_some_and(|extension| extension == "so")
                && path
                    .file_name()
                    .is_some_and(|name| name.to_string_lossy().starts_with("lib"))
        })
        .collect();
    libraries.sort();
    assert_eq!(libraries.len(), 1, "expected exactly one C shared object");
    libraries.remove(0)
}

fn rust_library_path() -> PathBuf {
    manifest_dir().join("target/release/libspec_ray_lib.so")
}

fn bytes<T>(value: &T) -> &[u8] {
    // SAFETY: The slice only borrows the initialized value for its exact size.
    unsafe {
        std::slice::from_raw_parts((value as *const T).cast::<u8>(), std::mem::size_of::<T>())
    }
}

fn same<T: Copy>(row: &str, case: usize, c: T, rust: T) {
    assert_eq!(
        bytes(&c),
        bytes(&rust),
        "{row} byte mismatch at case {case}: C={} Rust={}",
        hex(bytes(&c)),
        hex(bytes(&rust))
    );
}

fn hex(data: &[u8]) -> String {
    data.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn sentinel() -> C2Raycast {
    C2Raycast {
        t: f32::from_bits(0x7fc1_2345),
        n: C2v {
            x: f32::from_bits(0x8000_0000),
            y: f32::from_bits(0x7f81_2345),
        },
    }
}

unsafe fn same_ray_circle(pair: &Pair, row: &str, case: usize, ray: C2Ray, shape: C2Circle) {
    let mut c_out = sentinel();
    let mut rust_out = sentinel();
    // SAFETY: Both outputs point to writable C-layout storage.
    let (c_result, rust_result) = unsafe {
        (
            (pair.c.ray_circle)(ray, shape, &mut c_out),
            (pair.rust.ray_circle)(ray, shape, &mut rust_out),
        )
    };
    same(row, case, c_result, rust_result);
    same(row, case, c_out, rust_out);
}

unsafe fn same_ray_aabb(pair: &Pair, row: &str, case: usize, ray: C2Ray, shape: C2Aabb) {
    let mut c_out = sentinel();
    let mut rust_out = sentinel();
    // SAFETY: Both outputs point to writable C-layout storage.
    let (c_result, rust_result) = unsafe {
        (
            (pair.c.ray_aabb)(ray, shape, &mut c_out),
            (pair.rust.ray_aabb)(ray, shape, &mut rust_out),
        )
    };
    same(row, case, c_result, rust_result);
    same(row, case, c_out, rust_out);
}

unsafe fn same_ray_capsule(pair: &Pair, row: &str, case: usize, ray: C2Ray, shape: C2Capsule) {
    let mut c_out = sentinel();
    let mut rust_out = sentinel();
    // SAFETY: Both outputs point to writable C-layout storage.
    let (c_result, rust_result) = unsafe {
        (
            (pair.c.ray_capsule)(ray, shape, &mut c_out),
            (pair.rust.ray_capsule)(ray, shape, &mut rust_out),
        )
    };
    same(row, case, c_result, rust_result);
    same(row, case, c_out, rust_out);
}

#[cfg(target_arch = "x86_64")]
unsafe fn call_cast_with_eax(
    function: FnCast,
    ray: C2Ray,
    shape: *const c_void,
    tag: c_int,
    out: *mut C2Raycast,
    incoming_eax: u32,
) -> c_int {
    let mut eax = incoming_eax;
    // SAFETY: This constructs the SysV stack argument for C2Ray and calls the
    // dynamically loaded function with the remaining standard ABI registers.
    unsafe {
        core::arch::asm!(
            "sub rsp, 32",
            "mov r8, qword ptr [r10]",
            "mov qword ptr [rsp], r8",
            "mov r8, qword ptr [r10 + 8]",
            "mov qword ptr [rsp + 8], r8",
            "mov r8d, dword ptr [r10 + 16]",
            "mov dword ptr [rsp + 16], r8d",
            "call r11",
            "add rsp, 32",
            in("r11") function,
            in("r10") &ray,
            in("rdi") shape,
            in("esi") tag,
            in("rdx") out,
            inlateout("eax") eax,
            clobber_abi("C"),
        );
    }
    eax as c_int
}

#[derive(Clone, Copy)]
struct Rng(u32);

impl Rng {
    fn new(seed: u32) -> Self {
        Self(seed)
    }

    fn u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        x
    }

    fn finite(&mut self) -> f32 {
        ((self.u32() % 200_001) as i32 - 100_000) as f32 / 1024.0
    }

    fn positive(&mut self) -> f32 {
        (self.u32() % 4095 + 1) as f32 / 64.0
    }

    fn unit(&mut self) -> f32 {
        (self.u32() % 10_001) as f32 / 10_000.0
    }

    fn jitter(&mut self, scale: f32) -> f32 {
        (self.unit() * 2.0 - 1.0) * scale
    }

    fn vector(&mut self) -> C2v {
        C2v {
            x: self.finite(),
            y: self.finite(),
        }
    }
}

#[test]
fn dynamic_symbol_surface() {
    let pair = Pair::load();
    let _ = (&pair.c, &pair.rust);
}

#[test]
fn value_and_ieee_surface_c01_c14() {
    let pair = Pair::load();
    let edge_bits = [
        0x0000_0000,
        0x8000_0000,
        0x3f80_0000,
        0xbf80_0000,
        0x0000_0001,
        0x8000_0001,
        0x7f7f_ffff,
        0xff7f_ffff,
        0x7f80_0000,
        0xff80_0000,
        0x7fc1_2345,
    ];
    let mut case = 0;
    for &x in &edge_bits {
        for &y in &edge_bits {
            let a = C2v {
                x: f32::from_bits(x),
                y: f32::from_bits(y),
            };
            let b = C2v {
                x: f32::from_bits(y),
                y: f32::from_bits(x),
            };
            let scalar = f32::from_bits(y);
            let matrix = C2m { x: a, y: b };
            // SAFETY: Calls use the signatures loaded from each shared object.
            unsafe {
                same(
                    "C01",
                    case,
                    (pair.c.c2_v)(a.x, a.y),
                    (pair.rust.c2_v)(a.x, a.y),
                );
                same("C02", case, (pair.c.c2_dot)(a, b), (pair.rust.c2_dot)(a, b));
                same("C03", case, (pair.c.c2_len)(a), (pair.rust.c2_len)(a));
                same("C04", case, (pair.c.c2_add)(a, b), (pair.rust.c2_add)(a, b));
                same("C05", case, (pair.c.c2_sub)(a, b), (pair.rust.c2_sub)(a, b));
                same(
                    "C06",
                    case,
                    (pair.c.c2_mulvs)(a, scalar),
                    (pair.rust.c2_mulvs)(a, scalar),
                );
                same(
                    "C07",
                    case,
                    (pair.c.c2_div)(a, scalar),
                    (pair.rust.c2_div)(a, scalar),
                );
                same("C08", case, (pair.c.c2_norm)(a), (pair.rust.c2_norm)(a));
                same(
                    "C09",
                    case,
                    (pair.c.c2_minv)(a, b),
                    (pair.rust.c2_minv)(a, b),
                );
                same(
                    "C10",
                    case,
                    (pair.c.c2_maxv)(a, b),
                    (pair.rust.c2_maxv)(a, b),
                );
                same("C11", case, (pair.c.c2_skew)(a), (pair.rust.c2_skew)(a));
                same("C12", case, (pair.c.c2_absv)(a), (pair.rust.c2_absv)(a));
                same("C13", case, (pair.c.c2_ccw90)(a), (pair.rust.c2_ccw90)(a));
                same(
                    "C14",
                    case,
                    (pair.c.c2_mulmvt)(matrix, a),
                    (pair.rust.c2_mulmvt)(matrix, a),
                );
            }
            case += 1;
        }
    }

    let mut rng = Rng::new(0x91e1_0da5);
    for case in 0..10_000 {
        let a = rng.vector();
        let b = rng.vector();
        let matrix = C2m {
            x: rng.vector(),
            y: rng.vector(),
        };
        let mut scalar = rng.finite();
        if scalar == 0.0 {
            scalar = 1.0;
        }
        // SAFETY: Calls use the signatures loaded from each shared object.
        unsafe {
            same(
                "C01",
                case,
                (pair.c.c2_v)(a.x, a.y),
                (pair.rust.c2_v)(a.x, a.y),
            );
            same("C02", case, (pair.c.c2_dot)(a, b), (pair.rust.c2_dot)(a, b));
            same("C03", case, (pair.c.c2_len)(a), (pair.rust.c2_len)(a));
            same("C04", case, (pair.c.c2_add)(a, b), (pair.rust.c2_add)(a, b));
            same("C05", case, (pair.c.c2_sub)(a, b), (pair.rust.c2_sub)(a, b));
            same(
                "C06",
                case,
                (pair.c.c2_mulvs)(a, scalar),
                (pair.rust.c2_mulvs)(a, scalar),
            );
            same(
                "C07",
                case,
                (pair.c.c2_div)(a, scalar),
                (pair.rust.c2_div)(a, scalar),
            );
            same("C08", case, (pair.c.c2_norm)(a), (pair.rust.c2_norm)(a));
            same(
                "C09",
                case,
                (pair.c.c2_minv)(a, b),
                (pair.rust.c2_minv)(a, b),
            );
            same(
                "C10",
                case,
                (pair.c.c2_maxv)(a, b),
                (pair.rust.c2_maxv)(a, b),
            );
            same("C11", case, (pair.c.c2_skew)(a), (pair.rust.c2_skew)(a));
            same("C12", case, (pair.c.c2_absv)(a), (pair.rust.c2_absv)(a));
            same("C13", case, (pair.c.c2_ccw90)(a), (pair.rust.c2_ccw90)(a));
            same(
                "C14",
                case,
                (pair.c.c2_mulmvt)(matrix, b),
                (pair.rust.c2_mulmvt)(matrix, b),
            );
        }
    }
}

#[test]
fn relation_surface_c15_c27() {
    let pair = Pair::load();
    let mut rng = Rng::new(0xaabb_2026);
    for case in 0..512 {
        let x = rng.finite();
        let y = rng.finite();
        let w = rng.positive();
        let h = rng.positive();
        let gap = rng.positive();
        let a = C2Aabb {
            min: C2v { x, y },
            max: C2v { x: x + w, y: y + h },
        };
        let overlap = C2Aabb {
            min: C2v {
                x: x + w * rng.unit(),
                y: y + h * rng.unit(),
            },
            max: a.max,
        };
        let left = C2Aabb {
            min: C2v { x: x - gap - w, y },
            max: C2v {
                x: x - gap,
                y: y + h,
            },
        };
        let right = C2Aabb {
            min: C2v { x: x + w + gap, y },
            max: C2v {
                x: x + 2.0 * w + gap,
                y: y + h,
            },
        };
        let below = C2Aabb {
            min: C2v { x, y: y - gap - h },
            max: C2v {
                x: x + w,
                y: y - gap,
            },
        };
        let above = C2Aabb {
            min: C2v { x, y: y + h + gap },
            max: C2v {
                x: x + w,
                y: y + 2.0 * h + gap,
            },
        };
        let points = [
            (
                "C20",
                C2v {
                    x: x + w * rng.unit(),
                    y: y + h * rng.unit(),
                },
            ),
            ("C21", C2v { x: x - gap, y }),
            ("C22", C2v { x, y: y - gap }),
            ("C23", C2v { x: x + w + gap, y }),
            ("C24", C2v { x, y: y + h + gap }),
        ];
        // SAFETY: All by-value arguments have matching C layouts.
        unsafe {
            for (row, b) in [
                ("C15", overlap),
                ("C16", left),
                ("C17", right),
                ("C18", below),
                ("C19", above),
            ] {
                same(
                    row,
                    case,
                    (pair.c.aabb_aabb)(a, b),
                    (pair.rust.aabb_aabb)(a, b),
                );
            }
            for (row, point) in points {
                same(
                    row,
                    case,
                    (pair.c.aabb_point)(a, point),
                    (pair.rust.aabb_point)(a, point),
                );
            }

            let radius = rng.positive();
            let circle = C2Circle {
                p: C2v { x: 0.0, y: 0.0 },
                r: radius,
            };
            for (row, point) in [
                (
                    "C25",
                    C2v {
                        x: radius * 0.5,
                        y: 0.0,
                    },
                ),
                ("C26", C2v { x: radius, y: 0.0 }),
                (
                    "C27",
                    C2v {
                        x: radius + gap,
                        y: 0.0,
                    },
                ),
            ] {
                same(
                    row,
                    case,
                    (pair.c.circle_point)(circle, point),
                    (pair.rust.circle_point)(circle, point),
                );
            }
            let negative = C2Circle {
                p: circle.p,
                r: -radius,
            };
            same(
                "C27",
                case,
                (pair.c.circle_point)(negative, points[0].1),
                (pair.rust.circle_point)(negative, points[0].1),
            );
        }
    }
}

#[test]
fn ray_circle_surface_c28_c31() {
    let pair = Pair::load();
    let mut rng = Rng::new(0xc1cc_1e00);
    for case in 0..1024 {
        let radius = 0.25 + rng.unit() * 2.0;
        let distance = radius + 2.0 + rng.unit() * 5.0;
        let circle = C2Circle {
            p: C2v { x: 0.0, y: 0.0 },
            r: radius,
        };
        let miss = C2Ray {
            p: C2v {
                x: -distance,
                y: radius + 0.1 + rng.unit() * 3.0,
            },
            d: C2v { x: 1.0, y: 0.0 },
            t: distance * 3.0,
        };
        let behind = C2Ray {
            p: C2v {
                x: distance,
                y: 0.0,
            },
            d: C2v { x: 1.0, y: 0.0 },
            t: distance * 3.0,
        };
        let too_short = C2Ray {
            p: C2v {
                x: -distance,
                y: 0.0,
            },
            d: C2v { x: 1.0, y: 0.0 },
            t: (distance - radius) * rng.unit(),
        };
        let hit = C2Ray {
            p: C2v {
                x: -distance,
                y: 0.0,
            },
            d: C2v { x: 1.0, y: 0.0 },
            t: distance * 2.0,
        };
        let tangent = C2Ray {
            p: C2v {
                x: -distance,
                y: radius,
            },
            d: C2v { x: 1.0, y: 0.0 },
            t: distance * 2.0,
        };
        // SAFETY: Both output pointers are valid.
        unsafe {
            same_ray_circle(&pair, "C28/E01", case, miss, circle);
            same_ray_circle(&pair, "C29/E02", case, behind, circle);
            same_ray_circle(&pair, "C30/E03", case, too_short, circle);
            same_ray_circle(&pair, "C31", case, hit, circle);
            same_ray_circle(&pair, "C31", case + 1024, tangent, circle);
        }
    }
}

#[test]
fn ray_aabb_surface_c32_c38() {
    let pair = Pair::load();
    let mut rng = Rng::new(0xaabb_ca57);
    for case in 0..1024 {
        let extent = 1.0 + rng.unit() * 4.0;
        let distance = extent + 1.0 + rng.unit() * 5.0;
        let box_shape = C2Aabb {
            min: C2v {
                x: -extent,
                y: -extent,
            },
            max: C2v {
                x: extent,
                y: extent,
            },
        };
        let broad_miss = C2Ray {
            p: C2v {
                x: -distance,
                y: distance,
            },
            d: C2v { x: -1.0, y: 0.0 },
            t: distance,
        };
        let diagonal_miss = C2Ray {
            p: C2v {
                x: -distance,
                y: extent + 1.0,
            },
            d: C2v {
                x: distance * 2.0,
                y: -0.5,
            },
            t: 1.0,
        };
        let hits = [
            (
                "C34",
                C2Ray {
                    p: C2v {
                        x: -distance,
                        y: rng.jitter(extent * 0.5),
                    },
                    d: C2v { x: 1.0, y: 0.0 },
                    t: distance * 2.0,
                },
            ),
            (
                "C35",
                C2Ray {
                    p: C2v {
                        x: distance,
                        y: rng.jitter(extent * 0.5),
                    },
                    d: C2v { x: -1.0, y: 0.0 },
                    t: distance * 2.0,
                },
            ),
            (
                "C36",
                C2Ray {
                    p: C2v {
                        x: rng.jitter(extent * 0.5),
                        y: -distance,
                    },
                    d: C2v { x: 0.0, y: 1.0 },
                    t: distance * 2.0,
                },
            ),
            (
                "C37",
                C2Ray {
                    p: C2v {
                        x: rng.jitter(extent * 0.5),
                        y: distance,
                    },
                    d: C2v { x: 0.0, y: -1.0 },
                    t: distance * 2.0,
                },
            ),
        ];
        let nan = f32::from_bits(0x7fc0_0000 | (rng.u32() & 0x003f_ffff));
        let nan_ray = C2Ray {
            p: C2v { x: nan, y: nan },
            d: C2v { x: nan, y: nan },
            t: nan,
        };
        // SAFETY: Both output pointers are valid.
        unsafe {
            same_ray_aabb(&pair, "C32/E04", case, broad_miss, box_shape);
            same_ray_aabb(&pair, "C33/E05", case, diagonal_miss, box_shape);
            for (row, ray) in hits {
                same_ray_aabb(&pair, row, case, ray, box_shape);
            }
            same_ray_aabb(&pair, "C38/E06", case, nan_ray, box_shape);
        }
    }
}

#[test]
fn ray_capsule_surface_c39_c48() {
    let pair = Pair::load();
    let mut rng = Rng::new(0xca95_01e5);
    for case in 0..1024 {
        let length = 8.0 + rng.unit() * 8.0;
        let radius = 0.75 + rng.unit() * 0.5;
        let j = rng.jitter(0.05);
        let capsule = C2Capsule {
            a: C2v { x: 0.0, y: 0.0 },
            b: C2v { x: 0.0, y: length },
            r: radius,
        };
        let rows = [
            (
                "C39",
                C2Ray {
                    p: C2v {
                        x: j,
                        y: length * 0.5,
                    },
                    d: C2v { x: 1.0, y: 0.0 },
                    t: 2.0,
                },
            ),
            (
                "C40",
                C2Ray {
                    p: C2v {
                        x: j,
                        y: -radius * 0.5,
                    },
                    d: C2v { x: 1.0, y: 0.0 },
                    t: 2.0,
                },
            ),
            (
                "C41",
                C2Ray {
                    p: C2v {
                        x: j,
                        y: length + radius * 0.5,
                    },
                    d: C2v { x: 1.0, y: 0.0 },
                    t: 2.0,
                },
            ),
            (
                "C42/E07",
                C2Ray {
                    p: C2v {
                        x: radius + 4.0,
                        y: length * 0.5,
                    },
                    d: C2v { x: 1.0, y: 0.0 },
                    t: 2.0,
                },
            ),
            (
                "C43",
                C2Ray {
                    p: C2v {
                        x: radius * 0.5,
                        y: -radius * 2.0,
                    },
                    d: C2v { x: 0.0, y: 1.0 },
                    t: radius * 4.0,
                },
            ),
            (
                "C44",
                C2Ray {
                    p: C2v {
                        x: radius * 0.5,
                        y: length + radius * 2.0,
                    },
                    d: C2v { x: 0.0, y: -1.0 },
                    t: radius * 4.0,
                },
            ),
            (
                "C45",
                C2Ray {
                    p: C2v {
                        x: radius * 2.0,
                        y: -radius * 2.0,
                    },
                    d: C2v { x: -1.0, y: 0.0 },
                    t: radius * 4.0,
                },
            ),
            (
                "C46",
                C2Ray {
                    p: C2v {
                        x: radius * 2.0,
                        y: length + radius * 2.0,
                    },
                    d: C2v { x: -1.0, y: 0.0 },
                    t: radius * 4.0,
                },
            ),
            (
                "C47",
                C2Ray {
                    p: C2v {
                        x: radius * 2.0,
                        y: length * 0.5,
                    },
                    d: C2v { x: -1.0, y: 0.0 },
                    t: radius * 4.0,
                },
            ),
            (
                "C48",
                C2Ray {
                    p: C2v {
                        x: -radius * 2.0,
                        y: length * 0.5,
                    },
                    d: C2v { x: 1.0, y: 0.0 },
                    t: radius * 4.0,
                },
            ),
        ];
        // SAFETY: Both output pointers are valid.
        unsafe {
            for (row, ray) in rows {
                same_ray_capsule(&pair, row, case, ray, capsule);
            }
        }
    }
}

#[test]
fn dispatch_and_spec_surface_c49_c55() {
    let pair = Pair::load();
    let mut rng = Rng::new(0xd159_a7c4);
    for case in 0..4096 {
        let a = rng.vector();
        let b = rng.vector();
        let c = rng.vector();
        let d = rng.vector();
        let ray = C2Ray {
            p: a,
            d: b,
            t: rng.positive(),
        };
        let circle = C2Circle {
            p: c,
            r: rng.positive(),
        };
        let box_shape = C2Aabb {
            min: C2v {
                x: c.x.min(d.x),
                y: c.y.min(d.y),
            },
            max: C2v {
                x: c.x.max(d.x),
                y: c.y.max(d.y),
            },
        };
        let mut capsule = C2Capsule {
            a: c,
            b: d,
            r: rng.positive(),
        };
        if capsule.a.x == capsule.b.x && capsule.a.y == capsule.b.y {
            capsule.b.x += 1.0;
        }
        // SAFETY: Shape pointers match each type discriminator and outputs are writable.
        unsafe {
            for (row, shape, tag) in [
                ("C49", (&circle as *const C2Circle).cast::<c_void>(), 0),
                ("C50", (&box_shape as *const C2Aabb).cast::<c_void>(), 1),
                ("C51", (&capsule as *const C2Capsule).cast::<c_void>(), 2),
            ] {
                let mut c_out = sentinel();
                let mut rust_out = sentinel();
                let c_result = (pair.c.cast_ray)(ray, shape, tag, &mut c_out);
                let rust_result = (pair.rust.cast_ray)(ray, shape, tag, &mut rust_out);
                same(row, case, c_result, rust_result);
                same(row, case, c_out, rust_out);
            }

            let scenarios = [
                ("C52", -10.0, 0.0, 0.0, 0.0, 2.0, -20.0, 0.0),
                ("C53", -10.0, 10.0, 0.0, 0.0, 2.0, -20.0, 10.0),
                ("C54", a.x, a.y, c.x, c.y, circle.r, a.x, a.y),
                ("C55", a.x, a.y, c.x, c.y, 0.0, d.x, d.y),
                ("C55", a.x, a.y, c.x, c.y, -circle.r, d.x, d.y),
            ];
            for (row, mp_x, mp_y, cp_x, cp_y, r, rp_x, rp_y) in scenarios {
                let mut c_out = sentinel();
                let mut rust_out = sentinel();
                let c_result = (pair.c.spec_ray)(&mut c_out, mp_x, mp_y, cp_x, cp_y, r, rp_x, rp_y);
                let rust_result =
                    (pair.rust.spec_ray)(&mut rust_out, mp_x, mp_y, cp_x, cp_y, r, rp_x, rp_y);
                same(row, case, c_result, rust_result);
                same(row, case, c_out, rust_out);
            }
        }
    }
}

#[test]
fn rejection_surface_e01_e12() {
    let pair = Pair::load();
    let circle = C2Circle {
        p: C2v { x: 0.0, y: 0.0 },
        r: 1.0,
    };
    let box_shape = C2Aabb {
        min: C2v { x: -1.0, y: -1.0 },
        max: C2v { x: 1.0, y: 1.0 },
    };
    let circle_miss = C2Ray {
        p: C2v { x: -5.0, y: 5.0 },
        d: C2v { x: 1.0, y: 0.0 },
        t: 10.0,
    };
    let aabb_miss = C2Ray {
        p: C2v { x: -5.0, y: 5.0 },
        d: C2v { x: -1.0, y: 0.0 },
        t: 2.0,
    };
    // SAFETY: Miss paths do not dereference null outputs. Invalid tags do not
    // dereference either pointer in the built C object or Rust translation.
    unsafe {
        for case in 0..256 {
            let c_result = (pair.c.ray_circle)(circle_miss, circle, std::ptr::null_mut());
            let rust_result = (pair.rust.ray_circle)(circle_miss, circle, std::ptr::null_mut());
            same("E09", case, c_result, rust_result);

            let c_result = (pair.c.ray_aabb)(aabb_miss, box_shape, std::ptr::null_mut());
            let rust_result = (pair.rust.ray_aabb)(aabb_miss, box_shape, std::ptr::null_mut());
            same("E10", case, c_result, rust_result);

            let invalid_tags = [-1, 3, 4, c_int::MIN, c_int::MAX];
            for (tag_case, tag) in invalid_tags.into_iter().enumerate() {
                let mut c_out = sentinel();
                let mut rust_out = sentinel();
                let incoming_eax = 0x6a09_e667_u32.wrapping_add((case * 5 + tag_case) as u32);
                #[cfg(target_arch = "x86_64")]
                let (c_result, rust_result) = (
                    call_cast_with_eax(
                        pair.c.cast_ray,
                        circle_miss,
                        std::ptr::null(),
                        tag,
                        &mut c_out,
                        incoming_eax,
                    ),
                    call_cast_with_eax(
                        pair.rust.cast_ray,
                        circle_miss,
                        std::ptr::null(),
                        tag,
                        &mut rust_out,
                        incoming_eax,
                    ),
                );
                #[cfg(not(target_arch = "x86_64"))]
                let (c_result, rust_result) = (
                    (pair.c.cast_ray)(circle_miss, std::ptr::null(), tag, &mut c_out),
                    (pair.rust.cast_ray)(circle_miss, std::ptr::null(), tag, &mut rust_out),
                );
                same("E08/E11", case * 5 + tag_case, c_result, rust_result);
                #[cfg(target_arch = "x86_64")]
                same(
                    "E08/E11 incoming EAX",
                    case * 5 + tag_case,
                    c_result,
                    incoming_eax as c_int,
                );
                same("E08/E11", case * 5 + tag_case, c_out, rust_out);
            }

            let c_result =
                (pair.c.spec_ray)(std::ptr::null_mut(), -5.0, 5.0, 0.0, 0.0, 1.0, -6.0, 5.0);
            let rust_result =
                (pair.rust.spec_ray)(std::ptr::null_mut(), -5.0, 5.0, 0.0, 0.0, 1.0, -6.0, 5.0);
            same("E12", case, c_result, rust_result);
        }
    }
}

#[cfg(unix)]
fn signal(status: ExitStatus) -> Option<i32> {
    use std::os::unix::process::ExitStatusExt;
    status.signal()
}

#[cfg(unix)]
#[test]
fn null_dereference_signal_parity() {
    for scenario in [
        "circle_out",
        "aabb_out",
        "capsule_out",
        "cast_shape",
        "spec_out",
    ] {
        let c = crash_child("c", scenario);
        let rust = crash_child("rust", scenario);
        assert!(
            !c.success() && !rust.success(),
            "{scenario} unexpectedly returned: C={c:?}, Rust={rust:?}"
        );
        assert_eq!(
            signal(c),
            signal(rust),
            "{scenario} terminated with different signals"
        );
    }
}

#[cfg(unix)]
fn crash_child(library: &str, scenario: &str) -> ExitStatus {
    Command::new(std::env::current_exe().expect("test executable path"))
        .arg("--exact")
        .arg("null_dereference_child")
        .arg("--nocapture")
        .env("DIFF_CRASH_LIBRARY", library)
        .env("DIFF_CRASH_SCENARIO", scenario)
        .status()
        .expect("failed to execute crash child")
}

#[test]
fn null_dereference_child() {
    let Ok(which) = std::env::var("DIFF_CRASH_LIBRARY") else {
        return;
    };
    let scenario = std::env::var("DIFF_CRASH_SCENARIO").expect("crash scenario");
    let path = if which == "c" {
        c_library_path()
    } else {
        rust_library_path()
    };
    // SAFETY: This child intentionally exercises the C API's documented
    // unchecked-pointer behavior and is expected to terminate by signal.
    unsafe {
        let api = Api::load(&path);
        let circle = C2Circle {
            p: C2v { x: 0.0, y: 0.0 },
            r: 1.0,
        };
        let box_shape = C2Aabb {
            min: C2v { x: -1.0, y: -1.0 },
            max: C2v { x: 1.0, y: 1.0 },
        };
        let capsule = C2Capsule {
            a: C2v { x: 0.0, y: 0.0 },
            b: C2v { x: 0.0, y: 4.0 },
            r: 1.0,
        };
        let hit = C2Ray {
            p: C2v { x: -5.0, y: 0.0 },
            d: C2v { x: 1.0, y: 0.0 },
            t: 10.0,
        };
        match scenario.as_str() {
            "circle_out" => {
                (api.ray_circle)(hit, circle, std::ptr::null_mut());
            }
            "aabb_out" => {
                (api.ray_aabb)(hit, box_shape, std::ptr::null_mut());
            }
            "capsule_out" => {
                (api.ray_capsule)(hit, capsule, std::ptr::null_mut());
            }
            "cast_shape" => {
                let mut out = sentinel();
                (api.cast_ray)(hit, std::ptr::null(), 0, &mut out);
            }
            "spec_out" => {
                (api.spec_ray)(std::ptr::null_mut(), 5.0, 0.0, 0.0, 0.0, 1.0, -5.0, 0.0);
            }
            _ => panic!("unknown crash scenario"),
        }
    }
    panic!("unchecked null pointer did not terminate the child");
}
