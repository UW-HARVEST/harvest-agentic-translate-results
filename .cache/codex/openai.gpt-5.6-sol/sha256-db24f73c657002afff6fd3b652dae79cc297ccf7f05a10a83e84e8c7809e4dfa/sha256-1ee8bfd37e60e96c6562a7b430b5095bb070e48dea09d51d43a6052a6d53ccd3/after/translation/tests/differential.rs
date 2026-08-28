use libloading::Library;
use std::ffi::{c_int, c_void};
use std::fs;
use std::path::{Path, PathBuf};

#[cfg(target_arch = "x86_64")]
core::arch::global_asm!(
    ".global call_cast_with_eax",
    ".type call_cast_with_eax,@function",
    "call_cast_with_eax:",
    "mov r11, rdi",
    "mov r10d, esi",
    "sub rsp, 24",
    "mov rax, [rdx]",
    "mov [rsp], rax",
    "mov rax, [rdx + 8]",
    "mov [rsp + 8], rax",
    "mov eax, [rdx + 16]",
    "mov [rsp + 16], eax",
    "mov rdi, rcx",
    "mov esi, r8d",
    "mov rdx, r9",
    "mov eax, r10d",
    "call r11",
    "add rsp, 24",
    "ret",
    ".size call_cast_with_eax, .-call_cast_with_eax",
);

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

type VFn = unsafe extern "C" fn(f32, f32) -> V;
type VVFloatFn = unsafe extern "C" fn(V, V) -> f32;
type VFloatFn = unsafe extern "C" fn(V) -> f32;
type VVFn = unsafe extern "C" fn(V, V) -> V;
type VSFn = unsafe extern "C" fn(V, f32) -> V;
type VFn1 = unsafe extern "C" fn(V) -> V;
type RayCircleFn = unsafe extern "C" fn(Ray, Circle, *mut Raycast) -> c_int;
type AabbAabbFn = unsafe extern "C" fn(Aabb, Aabb) -> c_int;
type RayAabbFn = unsafe extern "C" fn(Ray, Aabb, *mut Raycast) -> c_int;
type MVFn = unsafe extern "C" fn(M, V) -> V;
type AabbPointFn = unsafe extern "C" fn(Aabb, V) -> c_int;
type CirclePointFn = unsafe extern "C" fn(Circle, V) -> c_int;
type RayCapsuleFn = unsafe extern "C" fn(Ray, Capsule, *mut Raycast) -> c_int;
type CastRayFn = unsafe extern "C" fn(Ray, *const c_void, c_int, *mut Raycast) -> c_int;
type GenRayFn = unsafe extern "C" fn(
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
unsafe extern "C" {
    fn call_cast_with_eax(
        function: CastRayFn,
        eax: c_int,
        ray: *const Ray,
        shape: *const c_void,
        kind: c_int,
        out: *mut Raycast,
    ) -> c_int;
}

#[derive(Clone, Copy)]
struct Api {
    v: VFn,
    dot: VVFloatFn,
    len: VFloatFn,
    add: VVFn,
    sub: VVFn,
    mulvs: VSFn,
    div: VSFn,
    norm: VFn1,
    minv: VVFn,
    maxv: VVFn,
    skew: VFn1,
    absv: VFn1,
    ray_circle: RayCircleFn,
    aabb_aabb: AabbAabbFn,
    ray_aabb: RayAabbFn,
    ccw90: VFn1,
    mulmvt: MVFn,
    aabb_point: AabbPointFn,
    circle_point: CirclePointFn,
    ray_capsule: RayCapsuleFn,
    cast_ray: CastRayFn,
    gen_ray: GenRayFn,
}

impl Api {
    unsafe fn load(lib: &Library) -> Self {
        unsafe fn symbol<T: Copy>(lib: &Library, name: &[u8]) -> T {
            unsafe { *lib.get::<T>(name).unwrap() }
        }
        Self {
            v: unsafe { symbol(lib, b"c2V\0") },
            dot: unsafe { symbol(lib, b"c2Dot\0") },
            len: unsafe { symbol(lib, b"c2Len\0") },
            add: unsafe { symbol(lib, b"c2Add\0") },
            sub: unsafe { symbol(lib, b"c2Sub\0") },
            mulvs: unsafe { symbol(lib, b"c2Mulvs\0") },
            div: unsafe { symbol(lib, b"c2Div\0") },
            norm: unsafe { symbol(lib, b"c2Norm\0") },
            minv: unsafe { symbol(lib, b"c2Minv\0") },
            maxv: unsafe { symbol(lib, b"c2Maxv\0") },
            skew: unsafe { symbol(lib, b"c2Skew\0") },
            absv: unsafe { symbol(lib, b"c2Absv\0") },
            ray_circle: unsafe { symbol(lib, b"c2RaytoCircle\0") },
            aabb_aabb: unsafe { symbol(lib, b"c2AABBtoAABB\0") },
            ray_aabb: unsafe { symbol(lib, b"c2RaytoAABB\0") },
            ccw90: unsafe { symbol(lib, b"c2CCW90\0") },
            mulmvt: unsafe { symbol(lib, b"c2MulmvT\0") },
            aabb_point: unsafe { symbol(lib, b"c2AABBtoPoint\0") },
            circle_point: unsafe { symbol(lib, b"c2CircleToPoint\0") },
            ray_capsule: unsafe { symbol(lib, b"c2RaytoCapsule\0") },
            cast_ray: unsafe { symbol(lib, b"c2CastRay\0") },
            gen_ray: unsafe { symbol(lib, b"gen_ray\0") },
        }
    }
}

struct Pair {
    _c_lib: Library,
    _rust_lib: Library,
    c: Api,
    rust: Api,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    let build = manifest_dir().join("../c_src/build");
    let mut paths: Vec<_> = fs::read_dir(&build)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "so"))
        .collect();
    paths.sort();
    assert_eq!(paths.len(), 1, "expected exactly one C shared library");
    paths.pop().unwrap()
}

fn rust_library_path() -> PathBuf {
    manifest_dir().join("target/release/libgen_ray_lib.so")
}

fn load_one(path: &Path) -> (Library, Api) {
    let lib = unsafe { Library::new(path).unwrap() };
    let api = unsafe { Api::load(&lib) };
    (lib, api)
}

fn pair() -> Pair {
    let (c_lib, c) = load_one(&c_library_path());
    let (rust_lib, rust) = load_one(&rust_library_path());
    Pair {
        _c_lib: c_lib,
        _rust_lib: rust_lib,
        c,
        rust,
    }
}

fn bytes<T>(value: &T) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts((value as *const T).cast::<u8>(), std::mem::size_of::<T>())
    }
}

fn assert_bytes<T>(label: &str, c: T, rust: T) {
    assert_eq!(
        bytes(&c),
        bytes(&rust),
        "{label}: C bytes differ from Rust bytes"
    );
}

fn sentinel() -> Raycast {
    Raycast {
        t: f32::from_bits(0x7fc1_2345),
        n: V {
            x: f32::from_bits(0x8000_0000),
            y: f32::from_bits(0x7f81_4321),
        },
    }
}

fn compare_out<F>(label: &str, call: F) -> (c_int, Raycast)
where
    F: Fn(bool, *mut Raycast) -> c_int,
{
    let mut c_out = sentinel();
    let mut rust_out = sentinel();
    let c_result = call(false, &mut c_out);
    let rust_result = call(true, &mut rust_out);
    assert_eq!(c_result, rust_result, "{label}: return value");
    assert_bytes(label, c_out, rust_out);
    (c_result, c_out)
}

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
        let fixed = (self.u32() % 20_001) as i32 - 10_000;
        fixed as f32 / 128.0
    }

    fn positive(&mut self) -> f32 {
        0.25 + (self.u32() % 4096) as f32 / 256.0
    }

    fn v(&mut self) -> V {
        V {
            x: self.finite(),
            y: self.finite(),
        }
    }
}

#[test]
fn vector_and_matrix_surface_c01_c23_c65() {
    let libs = pair();
    let specials = [
        0.0,
        -0.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::from_bits(0x7fc1_2345),
    ];
    for &x in &specials {
        for &y in &specials {
            assert_bytes("C01 c2V specials", unsafe { (libs.c.v)(x, y) }, unsafe {
                (libs.rust.v)(x, y)
            });
        }
    }

    let mut rng = Rng::new(0xd1ff_e4e7_1234_5678);
    for iteration in 0..256 {
        let a = rng.v();
        let b = rng.v();
        let scalar = rng.positive();
        let nonzero = if iteration & 1 == 0 { scalar } else { -scalar };
        let matrix = M {
            x: rng.v(),
            y: rng.v(),
        };
        macro_rules! compare {
            ($label:literal, $field:ident, ($($arg:expr),*)) => {
                assert_bytes(
                    $label,
                    unsafe { (libs.c.$field)($($arg),*) },
                    unsafe { (libs.rust.$field)($($arg),*) },
                )
            };
        }
        compare!("C02 c2Dot", dot, (a, b));
        compare!("C03 c2Len", len, (a));
        compare!("C04 c2Add", add, (a, b));
        compare!("C05 c2Sub", sub, (a, b));
        compare!("C06 c2Mulvs", mulvs, (a, scalar));
        compare!("C07 c2Div", div, (a, nonzero));
        let norm_input = if a.x == 0.0 && a.y == 0.0 {
            V { x: 1.0, y: 0.0 }
        } else {
            a
        };
        compare!("C08 c2Norm", norm, (norm_input));

        let dx = rng.positive();
        let dy = rng.positive();
        let min_cases = [
            (
                V { x: a.x, y: a.y },
                V {
                    x: a.x + dx,
                    y: a.y + dy,
                },
            ),
            (
                V {
                    x: a.x,
                    y: a.y + dy,
                },
                V {
                    x: a.x + dx,
                    y: a.y,
                },
            ),
            (
                V {
                    x: a.x + dx,
                    y: a.y,
                },
                V {
                    x: a.x,
                    y: a.y + dy,
                },
            ),
            (
                V {
                    x: a.x + dx,
                    y: a.y + dy,
                },
                V { x: a.x, y: a.y },
            ),
        ];
        for (left, right) in min_cases {
            compare!("C09-C12 c2Minv branch quadrant", minv, (left, right));
        }
        let max_cases = [
            (
                V {
                    x: a.x + dx,
                    y: a.y + dy,
                },
                V { x: a.x, y: a.y },
            ),
            (
                V {
                    x: a.x + dx,
                    y: a.y,
                },
                V {
                    x: a.x,
                    y: a.y + dy,
                },
            ),
            (
                V {
                    x: a.x,
                    y: a.y + dy,
                },
                V {
                    x: a.x + dx,
                    y: a.y,
                },
            ),
            (
                V { x: a.x, y: a.y },
                V {
                    x: a.x + dx,
                    y: a.y + dy,
                },
            ),
        ];
        for (left, right) in max_cases {
            compare!("C13-C16 c2Maxv branch quadrant", maxv, (left, right));
        }
        compare!("C17 c2Skew", skew, (a));
        for signs in 0..4 {
            let signed = V {
                x: if signs & 1 == 0 {
                    a.x.abs()
                } else {
                    -a.x.abs() - 0.25
                },
                y: if signs & 2 == 0 {
                    a.y.abs()
                } else {
                    -a.y.abs() - 0.25
                },
            };
            compare!("C18-C21 c2Absv sign quadrant", absv, (signed));
        }
        compare!("C22 c2CCW90", ccw90, (a));
        compare!("C23 c2MulmvT", mulmvt, (matrix, b));
    }

    let zero = V { x: 0.0, y: -0.0 };
    assert_bytes("C03 zero length", unsafe { (libs.c.len)(zero) }, unsafe {
        (libs.rust.len)(zero)
    });
    assert_bytes("C65 c2Norm zero", unsafe { (libs.c.norm)(zero) }, unsafe {
        (libs.rust.norm)(zero)
    });
    assert_bytes(
        "C65 c2Div by zero",
        unsafe { (libs.c.div)(V { x: 1.0, y: -1.0 }, 0.0) },
        unsafe { (libs.rust.div)(V { x: 1.0, y: -1.0 }, 0.0) },
    );
    let nan = f32::from_bits(0x7fc1_2345);
    let nan_v = V { x: nan, y: 2.0 };
    assert_bytes(
        "C65 c2Minv NaN fallthrough",
        unsafe { (libs.c.minv)(nan_v, V { x: 3.0, y: 1.0 }) },
        unsafe { (libs.rust.minv)(nan_v, V { x: 3.0, y: 1.0 }) },
    );
}

#[test]
fn point_and_overlap_surface_c24_c28_c63_c64_e04_e07_e11_e15() {
    let libs = pair();
    let mut rng = Rng::new(0xaabb_c1ac_1e55_0001);
    for _ in 0..128 {
        let x = rng.finite();
        let y = rng.finite();
        let w = rng.positive();
        let h = rng.positive();
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
                x: x + w * 0.75,
                y: y + h * 0.75,
            },
        };
        let touching = Aabb {
            min: V { x: x + w, y: y + h },
            max: V {
                x: x + w * 2.0,
                y: y + h * 2.0,
            },
        };
        for (label, b, expected) in [
            ("C24 overlap", overlap, 1),
            ("C25 touching", touching, 1),
            (
                "E04 left",
                Aabb {
                    min: V { x: x - 2.0 * w, y },
                    max: V { x: x - w, y: y + h },
                },
                0,
            ),
            (
                "E05 right",
                Aabb {
                    min: V { x: x + 2.0 * w, y },
                    max: V {
                        x: x + 3.0 * w,
                        y: y + h,
                    },
                },
                0,
            ),
            (
                "E06 below",
                Aabb {
                    min: V { x, y: y - 2.0 * h },
                    max: V { x: x + w, y: y - h },
                },
                0,
            ),
            (
                "E07 above",
                Aabb {
                    min: V { x, y: y + 2.0 * h },
                    max: V {
                        x: x + w,
                        y: y + 3.0 * h,
                    },
                },
                0,
            ),
        ] {
            let c_result = unsafe { (libs.c.aabb_aabb)(a, b) };
            let rust_result = unsafe { (libs.rust.aabb_aabb)(a, b) };
            assert_eq!(c_result, expected, "{label}: expected C branch");
            assert_eq!(c_result, rust_result, "{label}: differential");
        }
        let point_cases = [
            (
                "C26 inside",
                V {
                    x: x + w * 0.5,
                    y: y + h * 0.5,
                },
                1,
            ),
            ("C26 boundary", V { x, y }, 1),
            (
                "E11 left",
                V {
                    x: x - w,
                    y: y + h * 0.5,
                },
                0,
            ),
            (
                "E12 below",
                V {
                    x: x + w * 0.5,
                    y: y - h,
                },
                0,
            ),
            (
                "E13 right",
                V {
                    x: x + 2.0 * w,
                    y: y + h * 0.5,
                },
                0,
            ),
            (
                "E14 above",
                V {
                    x: x + w * 0.5,
                    y: y + 2.0 * h,
                },
                0,
            ),
        ];
        for (label, point, expected) in point_cases {
            let c_result = unsafe { (libs.c.aabb_point)(a, point) };
            let rust_result = unsafe { (libs.rust.aabb_point)(a, point) };
            assert_eq!(c_result, expected, "{label}: expected C branch");
            assert_eq!(c_result, rust_result, "{label}: differential");
        }

        let r = rng.positive();
        let circle = Circle { p: V { x, y }, r };
        for (label, point, expected) in [
            ("C27 inside", V { x: x + r * 0.5, y }, 1),
            ("C28/E15 boundary", V { x: x + r, y }, 0),
            ("C28/E15 outside", V { x: x + r * 2.0, y }, 0),
        ] {
            let c_result = unsafe { (libs.c.circle_point)(circle, point) };
            let rust_result = unsafe { (libs.rust.circle_point)(circle, point) };
            assert_eq!(c_result, expected, "{label}: expected C branch");
            assert_eq!(c_result, rust_result, "{label}: differential");
        }
    }

    let reversed = Aabb {
        min: V { x: 4.0, y: 3.0 },
        max: V { x: -2.0, y: -1.0 },
    };
    assert_eq!(
        unsafe { (libs.c.aabb_point)(reversed, V { x: 0.0, y: 0.0 }) },
        unsafe { (libs.rust.aabb_point)(reversed, V { x: 0.0, y: 0.0 }) },
        "C64 reversed AABB point"
    );
    assert_eq!(
        unsafe { (libs.c.aabb_aabb)(reversed, reversed) },
        unsafe { (libs.rust.aabb_aabb)(reversed, reversed) },
        "C64 reversed AABB overlap"
    );
    for radius in [0.0, -0.0, -1.0] {
        let circle = Circle {
            p: V { x: 0.0, y: 0.0 },
            r: radius,
        };
        assert_eq!(
            unsafe { (libs.c.circle_point)(circle, V { x: 0.0, y: 0.0 }) },
            unsafe { (libs.rust.circle_point)(circle, V { x: 0.0, y: 0.0 }) },
            "C63 nonpositive circle radius"
        );
    }
}

#[test]
fn circle_cast_surface_c29_c34_c63_e01_e03_e22() {
    let libs = pair();
    let mut rng = Rng::new(0xc1ac_1e00_5eed_0022);
    for _ in 0..128 {
        let x = rng.finite();
        let y = rng.finite();
        let r = rng.positive();
        let circle = Circle { p: V { x, y }, r };
        let cases = [
            (
                "C29 in-range hit",
                Ray {
                    p: V { x: x - 5.0 * r, y },
                    d: V { x: 1.0, y: 0.0 },
                    t: 10.0 * r,
                },
                1,
            ),
            (
                "C32/E01 negative discriminant",
                Ray {
                    p: V {
                        x: x - 5.0 * r,
                        y: y + 2.0 * r,
                    },
                    d: V { x: 1.0, y: 0.0 },
                    t: 10.0 * r,
                },
                0,
            ),
            (
                "C33/E02 behind ray",
                Ray {
                    p: V { x: x + 3.0 * r, y },
                    d: V { x: 1.0, y: 0.0 },
                    t: 10.0 * r,
                },
                0,
            ),
            (
                "C34/E03 beyond segment",
                Ray {
                    p: V { x: x - 5.0 * r, y },
                    d: V { x: 1.0, y: 0.0 },
                    t: 2.0 * r,
                },
                0,
            ),
        ];
        for (label, ray, expected) in cases {
            let (result, out) = compare_out(label, |rust, out| unsafe {
                if rust {
                    (libs.rust.ray_circle)(ray, circle, out)
                } else {
                    (libs.c.ray_circle)(ray, circle, out)
                }
            });
            assert_eq!(result, expected, "{label}: expected C branch");
            if expected == 0 {
                assert_bytes(&format!("{label}: rejected output"), out, sentinel());
            }
        }

        let exact_r = f32::from_bits(((121 + (rng.u32() % 13)) << 23) as u32);
        let exact_circle = Circle {
            p: V { x: 0.0, y: 0.0 },
            r: exact_r,
        };
        for (label, ray) in [
            (
                "C30 exact tangent",
                Ray {
                    p: V {
                        x: -5.0 * exact_r,
                        y: exact_r,
                    },
                    d: V { x: 1.0, y: 0.0 },
                    t: 10.0 * exact_r,
                },
            ),
            (
                "C31 exact boundary t zero",
                Ray {
                    p: V {
                        x: -exact_r,
                        y: 0.0,
                    },
                    d: V { x: 1.0, y: 0.0 },
                    t: 4.0 * exact_r,
                },
            ),
        ] {
            let (result, _) = compare_out(label, |rust, out| unsafe {
                if rust {
                    (libs.rust.ray_circle)(ray, exact_circle, out)
                } else {
                    (libs.c.ray_circle)(ray, exact_circle, out)
                }
            });
            assert_eq!(result, 1, "{label}: expected C branch");
        }

        let miss = Ray {
            p: V {
                x: x - 5.0 * r,
                y: y + 2.0 * r,
            },
            d: V { x: 1.0, y: 0.0 },
            t: 10.0 * r,
        };
        let c_result = unsafe { (libs.c.ray_circle)(miss, circle, std::ptr::null_mut()) };
        let rust_result = unsafe { (libs.rust.ray_circle)(miss, circle, std::ptr::null_mut()) };
        assert_eq!(c_result, 0, "E22 expected C rejection");
        assert_eq!(c_result, rust_result, "E22 differential");
    }

    let ray = Ray {
        p: V { x: -2.0, y: 0.0 },
        d: V { x: 1.0, y: 0.0 },
        t: 4.0,
    };
    for radius in [0.0, -0.0, -1.0] {
        let circle = Circle {
            p: V { x: 0.0, y: 0.0 },
            r: radius,
        };
        compare_out("C63 nonpositive-radius circle cast", |rust, out| unsafe {
            if rust {
                (libs.rust.ray_circle)(ray, circle, out)
            } else {
                (libs.c.ray_circle)(ray, circle, out)
            }
        });
    }
}

#[test]
fn aabb_cast_surface_c35_c41_c64_c65_e08_e10() {
    let libs = pair();
    let mut rng = Rng::new(0xaabb_ca57_5eed_0041);
    for _ in 0..128 {
        let x = rng.finite();
        let y = rng.finite();
        let w = rng.positive();
        let h = rng.positive();
        let b = Aabb {
            min: V { x, y },
            max: V { x: x + w, y: y + h },
        };
        let cases = [
            (
                "C35 min-x face",
                Ray {
                    p: V {
                        x: x - 2.0 * w,
                        y: y + 0.5 * h,
                    },
                    d: V { x: 1.0, y: 0.0 },
                    t: 4.0 * w,
                },
                1,
            ),
            (
                "C36 max-x face",
                Ray {
                    p: V {
                        x: x + 3.0 * w,
                        y: y + 0.5 * h,
                    },
                    d: V { x: -1.0, y: 0.0 },
                    t: 4.0 * w,
                },
                1,
            ),
            (
                "C37 min-y face",
                Ray {
                    p: V {
                        x: x + 0.5 * w,
                        y: y - 2.0 * h,
                    },
                    d: V { x: 0.0, y: 1.0 },
                    t: 4.0 * h,
                },
                1,
            ),
            (
                "C38 max-y face",
                Ray {
                    p: V {
                        x: x + 0.5 * w,
                        y: y + 3.0 * h,
                    },
                    d: V { x: 0.0, y: -1.0 },
                    t: 4.0 * h,
                },
                1,
            ),
            (
                "C39 starts inside",
                Ray {
                    p: V {
                        x: x + 0.5 * w,
                        y: y + 0.5 * h,
                    },
                    d: V { x: 1.0, y: 0.0 },
                    t: 2.0 * w,
                },
                1,
            ),
            (
                "C40/E08 broadphase disjoint",
                Ray {
                    p: V {
                        x: x - 3.0 * w,
                        y: y + 3.0 * h,
                    },
                    d: V { x: -1.0, y: 0.0 },
                    t: w,
                },
                0,
            ),
            (
                "C41/E09 separating axis",
                Ray {
                    p: V {
                        x: x - w,
                        y: y + 0.75 * h,
                    },
                    d: V {
                        x: 1.25 * w,
                        y: 1.25 * h,
                    },
                    t: 1.0,
                },
                0,
            ),
        ];
        for (label, ray, expected) in cases {
            let (result, out) = compare_out(label, |rust, out| unsafe {
                if rust {
                    (libs.rust.ray_aabb)(ray, b, out)
                } else {
                    (libs.c.ray_aabb)(ray, b, out)
                }
            });
            assert_eq!(result, expected, "{label}: expected C branch");
            if expected == 0 {
                assert_bytes(&format!("{label}: rejected output"), out, sentinel());
            }
        }
    }

    let ray = Ray {
        p: V { x: -2.0, y: 0.0 },
        d: V { x: 1.0, y: 0.0 },
        t: 4.0,
    };
    let nan = f32::from_bits(0x7fc1_2345);
    let nan_box = Aabb {
        min: V { x: nan, y: nan },
        max: V { x: nan, y: nan },
    };
    let (result, out) = compare_out("C65/E10 NaN plane distances", |rust, out| unsafe {
        if rust {
            (libs.rust.ray_aabb)(ray, nan_box, out)
        } else {
            (libs.c.ray_aabb)(ray, nan_box, out)
        }
    });
    assert_eq!(result, 0, "E10 expected C rejection");
    assert_bytes("E10 rejected output", out, sentinel());

    let reversed = Aabb {
        min: V { x: 1.0, y: 1.0 },
        max: V { x: -1.0, y: -1.0 },
    };
    compare_out("C64 reversed AABB cast", |rust, out| unsafe {
        if rust {
            (libs.rust.ray_aabb)(ray, reversed, out)
        } else {
            (libs.c.ray_aabb)(ray, reversed, out)
        }
    });
}

#[test]
fn capsule_cast_surface_c42_c50_c63_e16_e20() {
    let libs = pair();
    let mut rng = Rng::new(0xca95_01e0_5eed_0050);
    for _ in 0..128 {
        let x = rng.finite();
        let y = rng.finite();
        let r = rng.positive();
        let capsule = Capsule {
            a: V { x, y },
            b: V { x, y: y + 4.0 * r },
            r,
        };
        let cases = [
            (
                "C42 starts in body",
                Ray {
                    p: V { x, y: y + 2.0 * r },
                    d: V { x: 1.0, y: 0.0 },
                    t: 4.0 * r,
                },
                1,
            ),
            (
                "C43 starts in cap A",
                Ray {
                    p: V { x, y: y - 0.5 * r },
                    d: V { x: 0.0, y: -1.0 },
                    t: 4.0 * r,
                },
                1,
            ),
            (
                "C44 starts in cap B",
                Ray {
                    p: V { x, y: y + 4.5 * r },
                    d: V { x: 0.0, y: 1.0 },
                    t: 4.0 * r,
                },
                1,
            ),
            (
                "C45 positive side",
                Ray {
                    p: V {
                        x: x + 2.0 * r,
                        y: y + 2.0 * r,
                    },
                    d: V { x: -1.0, y: 0.0 },
                    t: 4.0 * r,
                },
                1,
            ),
            (
                "C46 negative side",
                Ray {
                    p: V {
                        x: x - 2.0 * r,
                        y: y + 2.0 * r,
                    },
                    d: V { x: 1.0, y: 0.0 },
                    t: 4.0 * r,
                },
                1,
            ),
            (
                "C47 cap A crossing",
                Ray {
                    p: V {
                        x: x - 2.0 * r,
                        y: y - 0.5 * r,
                    },
                    d: V { x: 1.0, y: 0.0 },
                    t: 4.0 * r,
                },
                1,
            ),
            (
                "C48 cap B crossing",
                Ray {
                    p: V {
                        x: x - 2.0 * r,
                        y: y + 4.5 * r,
                    },
                    d: V { x: 1.0, y: 0.0 },
                    t: 4.0 * r,
                },
                1,
            ),
            (
                "C49/E16 no side crossing",
                Ray {
                    p: V {
                        x: x + 2.0 * r,
                        y: y + 2.0 * r,
                    },
                    d: V { x: 0.0, y: 1.0 },
                    t: 2.0 * r,
                },
                0,
            ),
            (
                "E17 delegated cap A rejects",
                Ray {
                    p: V {
                        x: x + 0.9 * r,
                        y: y - r,
                    },
                    d: V { x: 0.0, y: -1.0 },
                    t: 2.0 * r,
                },
                0,
            ),
            (
                "E18 delegated cap B rejects",
                Ray {
                    p: V {
                        x: x + 0.9 * r,
                        y: y + 5.0 * r,
                    },
                    d: V { x: 0.0, y: 1.0 },
                    t: 2.0 * r,
                },
                0,
            ),
            (
                "E19 side route cap A rejects",
                Ray {
                    p: V {
                        x: x - 2.0 * r,
                        y: y - 2.0 * r,
                    },
                    d: V { x: 1.0, y: 0.0 },
                    t: 4.0 * r,
                },
                0,
            ),
            (
                "E20 side route cap B rejects",
                Ray {
                    p: V {
                        x: x - 2.0 * r,
                        y: y + 6.0 * r,
                    },
                    d: V { x: 1.0, y: 0.0 },
                    t: 4.0 * r,
                },
                0,
            ),
        ];
        for (label, ray, expected) in cases {
            let (result, _) = compare_out(label, |rust, out| unsafe {
                if rust {
                    (libs.rust.ray_capsule)(ray, capsule, out)
                } else {
                    (libs.c.ray_capsule)(ray, capsule, out)
                }
            });
            assert_eq!(result, expected, "{label}: expected C branch");
        }
    }

    let degenerate = Capsule {
        a: V { x: 0.0, y: 0.0 },
        b: V { x: 0.0, y: 0.0 },
        r: 1.0,
    };
    let ray = Ray {
        p: V { x: 2.0, y: 0.0 },
        d: V { x: -1.0, y: 0.0 },
        t: 4.0,
    };
    compare_out("C50 degenerate capsule", |rust, out| unsafe {
        if rust {
            (libs.rust.ray_capsule)(ray, degenerate, out)
        } else {
            (libs.c.ray_capsule)(ray, degenerate, out)
        }
    });
    for radius in [0.0, -1.0] {
        let capsule = Capsule {
            a: V { x: 0.0, y: -1.0 },
            b: V { x: 0.0, y: 1.0 },
            r: radius,
        };
        compare_out("C63 nonpositive capsule radius", |rust, out| unsafe {
            if rust {
                (libs.rust.ray_capsule)(ray, capsule, out)
            } else {
                (libs.c.ray_capsule)(ray, capsule, out)
            }
        });
    }
}

#[test]
fn cast_dispatch_surface_c51_c53_e21() {
    let libs = pair();
    let ray = Ray {
        p: V { x: -4.0, y: 0.0 },
        d: V { x: 1.0, y: 0.0 },
        t: 8.0,
    };
    let circle = Circle {
        p: V { x: 0.0, y: 0.0 },
        r: 1.0,
    };
    let aabb = Aabb {
        min: V { x: -1.0, y: -1.0 },
        max: V { x: 1.0, y: 1.0 },
    };
    let capsule = Capsule {
        a: V { x: 0.0, y: -1.0 },
        b: V { x: 0.0, y: 1.0 },
        r: 0.5,
    };
    let shapes = [
        ("C51 circle dispatch", (&circle as *const Circle).cast(), 0),
        ("C52 AABB dispatch", (&aabb as *const Aabb).cast(), 1),
        (
            "C53 capsule dispatch",
            (&capsule as *const Capsule).cast(),
            2,
        ),
    ];
    for _ in 0..128 {
        for &(label, shape, kind) in &shapes {
            let (result, _) = compare_out(label, |rust, out| unsafe {
                if rust {
                    (libs.rust.cast_ray)(ray, shape, kind, out)
                } else {
                    (libs.c.cast_ray)(ray, shape, kind, out)
                }
            });
            assert_eq!(result, 1, "{label}: expected C dispatch hit");
        }
    }
    #[cfg(target_arch = "x86_64")]
    for kind in [i32::MIN, -2, -1, 3, 4, 99, i32::MAX] {
        for seed in [0, 1, -1, 0x1234_5678, i32::MIN, i32::MAX] {
            let mut c_out = sentinel();
            let mut rust_out = sentinel();
            let c_result = unsafe {
                call_cast_with_eax(
                    libs.c.cast_ray,
                    seed,
                    &ray,
                    (&circle as *const Circle).cast(),
                    kind,
                    &mut c_out,
                )
            };
            let rust_result = unsafe {
                call_cast_with_eax(
                    libs.rust.cast_ray,
                    seed,
                    &ray,
                    (&circle as *const Circle).cast(),
                    kind,
                    &mut rust_out,
                )
            };
            assert_eq!(c_result, seed, "E21 C must preserve incoming EAX");
            assert_eq!(c_result, rust_result, "E21 invalid enum");
            assert_bytes("E21 invalid enum output", c_out, rust_out);
            assert_bytes("E21 C leaves output unchanged", c_out, sentinel());
        }
    }
}

fn compare_gen(libs: &Pair, label: &str, args: [f32; 16]) -> c_int {
    let mut c_out = [sentinel(), sentinel(), sentinel()];
    let mut rust_out = [sentinel(), sentinel(), sentinel()];
    let c_result = unsafe {
        (libs.c.gen_ray)(
            &mut c_out[0],
            &mut c_out[1],
            &mut c_out[2],
            args[0],
            args[1],
            args[2],
            args[3],
            args[4],
            args[5],
            args[6],
            args[7],
            args[8],
            args[9],
            args[10],
            args[11],
            args[12],
            args[13],
            args[14],
            args[15],
        )
    };
    let rust_result = unsafe {
        (libs.rust.gen_ray)(
            &mut rust_out[0],
            &mut rust_out[1],
            &mut rust_out[2],
            args[0],
            args[1],
            args[2],
            args[3],
            args[4],
            args[5],
            args[6],
            args[7],
            args[8],
            args[9],
            args[10],
            args[11],
            args[12],
            args[13],
            args[14],
            args[15],
        )
    };
    assert_eq!(c_result, rust_result, "{label}: return value");
    assert_bytes(label, c_out, rust_out);
    c_result
}

#[test]
fn composed_gen_ray_surface_c54_c62() {
    let libs = pair();
    let mut rng = Rng::new(0x6e6e_7261_795f_0062);
    for mask in 0..8 {
        for _ in 0..128 {
            let x = rng.finite();
            let y = rng.finite();
            let s = rng.positive();
            let far_y = y + 10.0 * s;
            let circle_y = if mask & 1 != 0 { y } else { far_y };
            let capsule_y = if mask & 2 != 0 { y } else { far_y };
            let aabb_y = if mask & 4 != 0 { y } else { far_y };
            let args = [
                x + 5.0 * s,
                y,
                x - 5.0 * s,
                y,
                x,
                circle_y,
                0.5 * s,
                x,
                capsule_y - s,
                x,
                capsule_y + s,
                0.5 * s,
                x - 0.5 * s,
                aabb_y - 0.5 * s,
                x + 0.5 * s,
                aabb_y + 0.5 * s,
            ];
            let result = compare_gen(&libs, "C54-C61 hit mask", args);
            assert_eq!(result, mask, "C54-C61 expected C hit mask");
        }
    }

    let zero_length = [
        1.0, 2.0, 1.0, 2.0, 0.0, 0.0, 1.0, 0.0, -1.0, 0.0, 1.0, 0.5, -1.0, -1.0, 1.0, 1.0,
    ];
    compare_gen(&libs, "C62 zero-length ray", zero_length);
}

#[test]
fn ffi_crash_probe() {
    let Some(which) = std::env::var_os("DIFF_CRASH_LIBRARY") else {
        return;
    };
    let case = std::env::var("DIFF_CRASH_CASE").unwrap();
    let path = if which == "c" {
        c_library_path()
    } else {
        rust_library_path()
    };
    let (_lib, api) = load_one(&path);
    let ray = Ray {
        p: V { x: -2.0, y: 0.0 },
        d: V { x: 1.0, y: 0.0 },
        t: 4.0,
    };
    let circle = Circle {
        p: V { x: 0.0, y: 0.0 },
        r: 1.0,
    };
    unsafe {
        match case.as_str() {
            "null_out" => {
                (api.ray_circle)(ray, circle, std::ptr::null_mut());
            }
            "null_shape" => {
                (api.cast_ray)(ray, std::ptr::null(), 0, &mut sentinel());
            }
            _ => panic!("unknown crash case"),
        }
    }
    panic!("FFI call unexpectedly returned");
}

#[cfg(unix)]
#[test]
fn null_pointer_crash_parity_e23_e24() {
    use std::os::unix::process::ExitStatusExt;
    use std::process::Command;

    let executable = std::env::current_exe().unwrap();
    for case in ["null_out", "null_shape"] {
        let run = |which: &str| {
            Command::new(&executable)
                .args(["--exact", "ffi_crash_probe"])
                .env("DIFF_CRASH_LIBRARY", which)
                .env("DIFF_CRASH_CASE", case)
                .output()
                .unwrap()
                .status
        };
        let c_status = run("c");
        let rust_status = run("rust");
        assert!(!c_status.success(), "{case}: C unexpectedly succeeded");
        assert_eq!(
            c_status.signal(),
            rust_status.signal(),
            "{case}: C and Rust terminated with different signals"
        );
        assert_eq!(c_status.signal(), Some(11), "{case}: expected SIGSEGV");
    }
}
