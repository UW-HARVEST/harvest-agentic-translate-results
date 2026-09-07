use libloading::Library;
use std::ffi::{c_float, c_int, c_void};
use std::path::{Path, PathBuf};
use std::process::Command;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct V {
    x: c_float,
    y: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Raycast {
    t: c_float,
    n: V,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Circle {
    p: V,
    r: c_float,
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
    r: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Ray {
    p: V,
    d: V,
    t: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct M {
    x: V,
    y: V,
}

type FnVff = unsafe extern "C" fn(c_float, c_float) -> V;
type FnFv = unsafe extern "C" fn(V) -> c_float;
type FnFvv = unsafe extern "C" fn(V, V) -> c_float;
type FnVv = unsafe extern "C" fn(V) -> V;
type FnVvv = unsafe extern "C" fn(V, V) -> V;
type FnVvf = unsafe extern "C" fn(V, c_float) -> V;
type FnVmv = unsafe extern "C" fn(M, V) -> V;
type FnIaa = unsafe extern "C" fn(Aabb, Aabb) -> c_int;
type FnIav = unsafe extern "C" fn(Aabb, V) -> c_int;
type FnIcv = unsafe extern "C" fn(Circle, V) -> c_int;
type FnIrc = unsafe extern "C" fn(Ray, Circle, *mut Raycast) -> c_int;
type FnIra = unsafe extern "C" fn(Ray, Aabb, *mut Raycast) -> c_int;
type FnIrk = unsafe extern "C" fn(Ray, Capsule, *mut Raycast) -> c_int;
type FnCast = unsafe extern "C" fn(Ray, *const c_void, c_int, *mut Raycast) -> c_int;
type FnSpec = unsafe extern "C" fn(
    *mut Raycast,
    c_float,
    c_float,
    c_float,
    c_float,
    c_float,
    c_float,
    c_float,
) -> c_int;

#[cfg(target_arch = "x86_64")]
core::arch::global_asm!(
    ".text",
    ".globl differential_call_cast_with_eax",
    ".type differential_call_cast_with_eax,@function",
    "differential_call_cast_with_eax:",
    "mov r10, rdi",
    "sub rsp, 24",
    "mov rax, [rsi]",
    "mov [rsp], rax",
    "mov rax, [rsi + 8]",
    "mov [rsp + 8], rax",
    "mov eax, [rsi + 16]",
    "mov [rsp + 16], eax",
    "mov rdi, rdx",
    "mov esi, ecx",
    "mov rdx, r8",
    "mov eax, r9d",
    "call r10",
    "add rsp, 24",
    "ret",
    ".size differential_call_cast_with_eax, .-differential_call_cast_with_eax",
);

#[cfg(target_arch = "x86_64")]
unsafe extern "C" {
    fn differential_call_cast_with_eax(
        function: *const c_void,
        ray: *const Ray,
        shape: *const c_void,
        shape_type: c_int,
        out: *mut Raycast,
        eax_seed: u32,
    ) -> c_int;
}

#[cfg(target_arch = "x86_64")]
fn call_cast_with_eax(
    function: FnCast,
    ray: &Ray,
    shape: *const c_void,
    shape_type: c_int,
    out: *mut Raycast,
    eax_seed: u32,
) -> c_int {
    unsafe {
        differential_call_cast_with_eax(
            function as *const () as *const c_void,
            ray,
            shape,
            shape_type,
            out,
            eax_seed,
        )
    }
}

struct Api {
    _library: Library,
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
    c2_ray_to_circle: FnIrc,
    c2_aabb_to_aabb: FnIaa,
    c2_ray_to_aabb: FnIra,
    c2_ccw90: FnVv,
    c2_mulmv_t: FnVmv,
    c2_aabb_to_point: FnIav,
    c2_circle_to_point: FnIcv,
    c2_ray_to_capsule: FnIrk,
    c2_cast_ray: FnCast,
    spec_ray: FnSpec,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        unsafe fn get<T: Copy>(library: &Library, name: &[u8]) -> T {
            unsafe { *library.get::<T>(name).unwrap() }
        }
        Self {
            c2_v: unsafe { get(&library, b"c2V\0") },
            c2_dot: unsafe { get(&library, b"c2Dot\0") },
            c2_len: unsafe { get(&library, b"c2Len\0") },
            c2_add: unsafe { get(&library, b"c2Add\0") },
            c2_sub: unsafe { get(&library, b"c2Sub\0") },
            c2_mulvs: unsafe { get(&library, b"c2Mulvs\0") },
            c2_div: unsafe { get(&library, b"c2Div\0") },
            c2_norm: unsafe { get(&library, b"c2Norm\0") },
            c2_minv: unsafe { get(&library, b"c2Minv\0") },
            c2_maxv: unsafe { get(&library, b"c2Maxv\0") },
            c2_skew: unsafe { get(&library, b"c2Skew\0") },
            c2_absv: unsafe { get(&library, b"c2Absv\0") },
            c2_ray_to_circle: unsafe { get(&library, b"c2RaytoCircle\0") },
            c2_aabb_to_aabb: unsafe { get(&library, b"c2AABBtoAABB\0") },
            c2_ray_to_aabb: unsafe { get(&library, b"c2RaytoAABB\0") },
            c2_ccw90: unsafe { get(&library, b"c2CCW90\0") },
            c2_mulmv_t: unsafe { get(&library, b"c2MulmvT\0") },
            c2_aabb_to_point: unsafe { get(&library, b"c2AABBtoPoint\0") },
            c2_circle_to_point: unsafe { get(&library, b"c2CircleToPoint\0") },
            c2_ray_to_capsule: unsafe { get(&library, b"c2RaytoCapsule\0") },
            c2_cast_ray: unsafe { get(&library, b"c2CastRay\0") },
            spec_ray: unsafe { get(&library, b"spec_ray\0") },
            _library: library,
        }
    }
}

fn library_paths() -> (PathBuf, PathBuf) {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_build = manifest.parent().unwrap().join("c_src/build");
    let mut c_libraries: Vec<_> = std::fs::read_dir(&c_build)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", c_build.display()))
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.extension().is_some_and(|extension| extension == "so")
                && path
                    .file_name()
                    .is_some_and(|name| name.to_string_lossy().starts_with("libharvest-work-"))
        })
        .collect();
    c_libraries.sort();
    assert_eq!(
        c_libraries.len(),
        1,
        "expected exactly one C shared library in {}",
        c_build.display()
    );
    let rust = manifest.join("target/release/libspec_ray_lib.so");
    assert!(
        rust.is_file(),
        "Rust cdylib missing at {}; run cargo build --release",
        rust.display()
    );
    (c_libraries.remove(0), rust)
}

fn apis() -> (Api, Api) {
    let (c_path, rust_path) = library_paths();
    unsafe { (Api::load(&c_path), Api::load(&rust_path)) }
}

fn bytes<T>(value: &T) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts((value as *const T).cast::<u8>(), std::mem::size_of::<T>())
    }
}

fn same<T>(label: &str, c_value: T, rust_value: T) {
    assert_eq!(
        bytes(&c_value),
        bytes(&rust_value),
        "{label}: byte mismatch"
    );
}

fn sentinel() -> Raycast {
    Raycast {
        t: f32::from_bits(0x7fc1_2345),
        n: V {
            x: f32::from_bits(0x8000_0000),
            y: f32::from_bits(0x7f81_2345),
        },
    }
}

fn compare_ray_circle(c: &Api, rust: &Api, ray: Ray, circle: Circle, label: &str) -> c_int {
    let mut c_out = sentinel();
    let mut rust_out = sentinel();
    let c_result = unsafe { (c.c2_ray_to_circle)(ray, circle, &mut c_out) };
    let rust_result = unsafe { (rust.c2_ray_to_circle)(ray, circle, &mut rust_out) };
    same(&format!("{label} return"), c_result, rust_result);
    same(&format!("{label} output"), c_out, rust_out);
    c_result
}

fn compare_ray_aabb(c: &Api, rust: &Api, ray: Ray, aabb: Aabb, label: &str) -> c_int {
    let mut c_out = sentinel();
    let mut rust_out = sentinel();
    let c_result = unsafe { (c.c2_ray_to_aabb)(ray, aabb, &mut c_out) };
    let rust_result = unsafe { (rust.c2_ray_to_aabb)(ray, aabb, &mut rust_out) };
    same(&format!("{label} return"), c_result, rust_result);
    same(&format!("{label} output"), c_out, rust_out);
    c_result
}

fn compare_ray_capsule(c: &Api, rust: &Api, ray: Ray, capsule: Capsule, label: &str) -> c_int {
    let mut c_out = sentinel();
    let mut rust_out = sentinel();
    let c_result = unsafe { (c.c2_ray_to_capsule)(ray, capsule, &mut c_out) };
    let rust_result = unsafe { (rust.c2_ray_to_capsule)(ray, capsule, &mut rust_out) };
    same(&format!("{label} return"), c_result, rust_result);
    same(&format!("{label} output"), c_out, rust_out);
    c_result
}

fn compare_cast(
    c: &Api,
    rust: &Api,
    ray: Ray,
    shape: *const c_void,
    shape_type: c_int,
    label: &str,
) -> c_int {
    let mut c_out = sentinel();
    let mut rust_out = sentinel();
    let c_result = unsafe { (c.c2_cast_ray)(ray, shape, shape_type, &mut c_out) };
    let rust_result = unsafe { (rust.c2_cast_ray)(ray, shape, shape_type, &mut rust_out) };
    same(&format!("{label} return"), c_result, rust_result);
    same(&format!("{label} output"), c_out, rust_out);
    c_result
}

#[derive(Clone, Copy)]
struct Rng(u32);

impl Rng {
    fn new(seed: u32) -> Self {
        Self(seed)
    }

    fn u32(&mut self) -> u32 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 17;
        value ^= value << 5;
        self.0 = value;
        value
    }

    fn finite(&mut self) -> f32 {
        ((self.u32() % 200_001) as i32 - 100_000) as f32 / 1024.0
    }

    fn v(&mut self) -> V {
        V {
            x: self.finite(),
            y: self.finite(),
        }
    }
}

fn v(x: f32, y: f32) -> V {
    V { x, y }
}

fn translated(base: V, x: f32, y: f32) -> V {
    v(base.x + x, base.y + y)
}

#[test]
fn primitive_and_ieee_configuration_rows_1_through_29() {
    let (c, rust) = apis();
    let mut rng = Rng::new(0x91e1_0da5);
    for case in 0..4096 {
        let a = rng.v();
        let b = rng.v();
        let mut scalar = rng.finite();
        if scalar == 0.0 {
            scalar = 1.0;
        }
        let matrix = M {
            x: rng.v(),
            y: rng.v(),
        };
        unsafe {
            same(
                &format!("c2V finite {case}"),
                (c.c2_v)(a.x, a.y),
                (rust.c2_v)(a.x, a.y),
            );
            same(
                &format!("c2Dot finite {case}"),
                (c.c2_dot)(a, b),
                (rust.c2_dot)(a, b),
            );
            same(
                &format!("c2Len finite {case}"),
                (c.c2_len)(a),
                (rust.c2_len)(a),
            );
            same(
                &format!("c2Add finite {case}"),
                (c.c2_add)(a, b),
                (rust.c2_add)(a, b),
            );
            same(
                &format!("c2Sub finite {case}"),
                (c.c2_sub)(a, b),
                (rust.c2_sub)(a, b),
            );
            same(
                &format!("c2Mulvs finite {case}"),
                (c.c2_mulvs)(a, scalar),
                (rust.c2_mulvs)(a, scalar),
            );
            same(
                &format!("c2Div finite {case}"),
                (c.c2_div)(a, scalar),
                (rust.c2_div)(a, scalar),
            );
            same(
                &format!("c2Norm finite {case}"),
                (c.c2_norm)(a),
                (rust.c2_norm)(a),
            );
            same(
                &format!("c2Minv finite {case}"),
                (c.c2_minv)(a, b),
                (rust.c2_minv)(a, b),
            );
            same(
                &format!("c2Maxv finite {case}"),
                (c.c2_maxv)(a, b),
                (rust.c2_maxv)(a, b),
            );
            same(
                &format!("c2Skew finite {case}"),
                (c.c2_skew)(a),
                (rust.c2_skew)(a),
            );
            same(
                &format!("c2Absv finite {case}"),
                (c.c2_absv)(a),
                (rust.c2_absv)(a),
            );
            same(
                &format!("c2CCW90 finite {case}"),
                (c.c2_ccw90)(a),
                (rust.c2_ccw90)(a),
            );
            same(
                &format!("c2MulmvT finite {case}"),
                (c.c2_mulmv_t)(matrix, b),
                (rust.c2_mulmv_t)(matrix, b),
            );
        }
    }

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
    for (i, &a_bits) in edge_bits.iter().enumerate() {
        for (j, &b_bits) in edge_bits.iter().enumerate() {
            let a = v(f32::from_bits(a_bits), f32::from_bits(b_bits));
            let b = v(f32::from_bits(b_bits), f32::from_bits(a_bits));
            let scalar = f32::from_bits(b_bits);
            let matrix = M { x: a, y: b };
            let label = format!("IEEE edge {i}/{j}");
            unsafe {
                same(
                    &format!("{label} c2V"),
                    (c.c2_v)(a.x, a.y),
                    (rust.c2_v)(a.x, a.y),
                );
                same(
                    &format!("{label} c2Dot"),
                    (c.c2_dot)(a, b),
                    (rust.c2_dot)(a, b),
                );
                same(&format!("{label} c2Len"), (c.c2_len)(a), (rust.c2_len)(a));
                same(
                    &format!("{label} c2Add"),
                    (c.c2_add)(a, b),
                    (rust.c2_add)(a, b),
                );
                same(
                    &format!("{label} c2Sub"),
                    (c.c2_sub)(a, b),
                    (rust.c2_sub)(a, b),
                );
                same(
                    &format!("{label} c2Mulvs"),
                    (c.c2_mulvs)(a, scalar),
                    (rust.c2_mulvs)(a, scalar),
                );
                same(
                    &format!("{label} c2Div"),
                    (c.c2_div)(a, scalar),
                    (rust.c2_div)(a, scalar),
                );
                same(
                    &format!("{label} c2Norm"),
                    (c.c2_norm)(a),
                    (rust.c2_norm)(a),
                );
                same(
                    &format!("{label} c2Minv"),
                    (c.c2_minv)(a, b),
                    (rust.c2_minv)(a, b),
                );
                same(
                    &format!("{label} c2Maxv"),
                    (c.c2_maxv)(a, b),
                    (rust.c2_maxv)(a, b),
                );
                same(
                    &format!("{label} c2Skew"),
                    (c.c2_skew)(a),
                    (rust.c2_skew)(a),
                );
                same(
                    &format!("{label} c2Absv"),
                    (c.c2_absv)(a),
                    (rust.c2_absv)(a),
                );
                same(
                    &format!("{label} c2CCW90"),
                    (c.c2_ccw90)(a),
                    (rust.c2_ccw90)(a),
                );
                same(
                    &format!("{label} c2MulmvT"),
                    (c.c2_mulmv_t)(matrix, b),
                    (rust.c2_mulmv_t)(matrix, b),
                );
            }
        }
    }
}

#[test]
fn predicate_configuration_rows_30_through_38_and_error_rows_1_through_9() {
    let (c, rust) = apis();
    let mut rng = Rng::new(0x42ac_e519);
    for case in 0..1024 {
        let origin = v(
            (rng.u32() % 201) as f32 - 100.0,
            (rng.u32() % 201) as f32 - 100.0,
        );
        let width = (rng.u32() % 20 + 1) as f32;
        let height = (rng.u32() % 20 + 1) as f32;
        let a = Aabb {
            min: origin,
            max: translated(origin, width, height),
        };
        let overlap = Aabb {
            min: translated(origin, width * 0.25, height * 0.25),
            max: translated(origin, width * 1.25, height * 1.25),
        };
        let touching = [
            Aabb {
                min: translated(origin, -width, 0.0),
                max: translated(origin, 0.0, height),
            },
            Aabb {
                min: translated(origin, width, 0.0),
                max: translated(origin, width * 2.0, height),
            },
            Aabb {
                min: translated(origin, 0.0, -height),
                max: translated(origin, width, 0.0),
            },
            Aabb {
                min: translated(origin, 0.0, height),
                max: translated(origin, width, height * 2.0),
            },
        ];
        let separated = [
            Aabb {
                min: translated(origin, -width - 2.0, 0.0),
                max: translated(origin, -1.0, height),
            },
            Aabb {
                min: translated(origin, width + 1.0, 0.0),
                max: translated(origin, width * 2.0 + 1.0, height),
            },
            Aabb {
                min: translated(origin, 0.0, -height - 2.0),
                max: translated(origin, width, -1.0),
            },
            Aabb {
                min: translated(origin, 0.0, height + 1.0),
                max: translated(origin, width, height * 2.0 + 1.0),
            },
        ];
        unsafe {
            let c_result = (c.c2_aabb_to_aabb)(a, overlap);
            let rust_result = (rust.c2_aabb_to_aabb)(a, overlap);
            same(&format!("AABB overlap {case}"), c_result, rust_result);
            assert_eq!(c_result, 1);
            for (side, shape) in touching.into_iter().enumerate() {
                let c_result = (c.c2_aabb_to_aabb)(a, shape);
                let rust_result = (rust.c2_aabb_to_aabb)(a, shape);
                same(
                    &format!("AABB touching {case}/{side}"),
                    c_result,
                    rust_result,
                );
                assert_eq!(c_result, 1);
            }
            for (side, shape) in separated.into_iter().enumerate() {
                let c_result = (c.c2_aabb_to_aabb)(a, shape);
                let rust_result = (rust.c2_aabb_to_aabb)(a, shape);
                same(
                    &format!("AABB separated {case}/{side}"),
                    c_result,
                    rust_result,
                );
                assert_eq!(c_result, 0);
            }

            let inside = translated(origin, width * 0.5, height * 0.5);
            let boundaries = [
                origin,
                translated(origin, width, 0.0),
                translated(origin, 0.0, height),
                translated(origin, width, height),
            ];
            let outside = [
                translated(origin, -1.0, height * 0.5),
                translated(origin, width * 0.5, -1.0),
                translated(origin, width + 1.0, height * 0.5),
                translated(origin, width * 0.5, height + 1.0),
            ];
            let c_result = (c.c2_aabb_to_point)(a, inside);
            let rust_result = (rust.c2_aabb_to_point)(a, inside);
            same(&format!("point inside AABB {case}"), c_result, rust_result);
            assert_eq!(c_result, 1);
            for (side, point) in boundaries.into_iter().enumerate() {
                let c_result = (c.c2_aabb_to_point)(a, point);
                let rust_result = (rust.c2_aabb_to_point)(a, point);
                same(
                    &format!("point boundary AABB {case}/{side}"),
                    c_result,
                    rust_result,
                );
                assert_eq!(c_result, 1);
            }
            for (side, point) in outside.into_iter().enumerate() {
                let c_result = (c.c2_aabb_to_point)(a, point);
                let rust_result = (rust.c2_aabb_to_point)(a, point);
                same(
                    &format!("point outside AABB {case}/{side}"),
                    c_result,
                    rust_result,
                );
                assert_eq!(c_result, 0);
            }

            let radius = (rng.u32() % 20 + 1) as f32;
            for signed_radius in [radius, -radius] {
                let circle = Circle {
                    p: origin,
                    r: signed_radius,
                };
                let c_result =
                    (c.c2_circle_to_point)(circle, translated(origin, radius * 0.5, 0.0));
                let rust_result =
                    (rust.c2_circle_to_point)(circle, translated(origin, radius * 0.5, 0.0));
                same(
                    &format!("point inside circle {case}/{signed_radius}"),
                    c_result,
                    rust_result,
                );
                assert_eq!(c_result, 1);
                let c_result = (c.c2_circle_to_point)(circle, translated(origin, radius, 0.0));
                let rust_result =
                    (rust.c2_circle_to_point)(circle, translated(origin, radius, 0.0));
                same(
                    &format!("point on circle {case}/{signed_radius}"),
                    c_result,
                    rust_result,
                );
                assert_eq!(c_result, 0);
                let c_result =
                    (c.c2_circle_to_point)(circle, translated(origin, radius + 1.0, 0.0));
                let rust_result =
                    (rust.c2_circle_to_point)(circle, translated(origin, radius + 1.0, 0.0));
                same(
                    &format!("point outside circle {case}/{signed_radius}"),
                    c_result,
                    rust_result,
                );
                assert_eq!(c_result, 0);
            }
            let zero = Circle { p: origin, r: 0.0 };
            let c_result = (c.c2_circle_to_point)(zero, origin);
            let rust_result = (rust.c2_circle_to_point)(zero, origin);
            same(&format!("zero-radius circle {case}"), c_result, rust_result);
            assert_eq!(c_result, 0);
        }
    }
}

#[test]
fn ray_circle_configuration_rows_39_through_44_and_error_rows_10_through_12_22() {
    let (c, rust) = apis();
    let mut rng = Rng::new(0x633d_7a11);
    for case in 0..1024 {
        let center = v(
            (rng.u32() % 201) as f32 - 100.0,
            (rng.u32() % 201) as f32 - 100.0,
        );
        let radius = (rng.u32() % 20 + 1) as f32;
        let gap = (rng.u32() % 20 + 1) as f32;
        let circle = Circle {
            p: center,
            r: radius,
        };

        let hit = Ray {
            p: translated(center, -radius - gap, 0.0),
            d: v(1.0, 0.0),
            t: gap + radius * 2.0 + 1.0,
        };
        assert_eq!(
            compare_ray_circle(&c, &rust, hit, circle, &format!("circle hit {case}")),
            1
        );

        let tangent = Ray {
            p: translated(center, -radius - gap, radius),
            d: v(1.0, 0.0),
            t: gap + radius + 1.0,
        };
        assert_eq!(
            compare_ray_circle(
                &c,
                &rust,
                tangent,
                circle,
                &format!("circle tangent {case}")
            ),
            1
        );

        let discriminant_miss = Ray {
            p: translated(center, -radius - gap, radius + 1.0),
            d: v(1.0, 0.0),
            t: gap + radius * 3.0,
        };
        assert_eq!(
            compare_ray_circle(
                &c,
                &rust,
                discriminant_miss,
                circle,
                &format!("circle discriminant miss {case}"),
            ),
            0
        );

        let behind = Ray {
            p: translated(center, radius + gap, 0.0),
            d: v(1.0, 0.0),
            t: gap + radius * 3.0,
        };
        assert_eq!(
            compare_ray_circle(&c, &rust, behind, circle, &format!("circle behind {case}")),
            0
        );

        let beyond = Ray {
            p: translated(center, -radius - gap, 0.0),
            d: v(1.0, 0.0),
            t: gap * 0.5,
        };
        assert_eq!(
            compare_ray_circle(
                &c,
                &rust,
                beyond,
                circle,
                &format!("circle beyond extent {case}")
            ),
            0
        );

        let inside = Ray {
            p: translated(center, radius * 0.5, 0.0),
            d: v(1.0, 0.0),
            t: radius * 3.0,
        };
        assert_eq!(
            compare_ray_circle(
                &c,
                &rust,
                inside,
                circle,
                &format!("circle starts inside {case}")
            ),
            0
        );

        let zero_direction = Ray {
            p: translated(center, radius + gap, 0.0),
            d: v(0.0, 0.0),
            t: 0.0,
        };
        assert_eq!(
            compare_ray_circle(
                &c,
                &rust,
                zero_direction,
                circle,
                &format!("circle zero direction {case}"),
            ),
            0
        );

        let negative_extent = Ray {
            p: translated(center, -radius - gap, 0.0),
            d: v(1.0, 0.0),
            t: -gap,
        };
        assert_eq!(
            compare_ray_circle(
                &c,
                &rust,
                negative_extent,
                circle,
                &format!("circle negative extent {case}"),
            ),
            0
        );

        let negative_radius = Circle {
            p: center,
            r: -radius,
        };
        assert_eq!(
            compare_ray_circle(
                &c,
                &rust,
                hit,
                negative_radius,
                &format!("circle negative radius {case}"),
            ),
            1
        );
    }

    let special = [
        0.0,
        -0.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::from_bits(0x7fc1_2345),
    ];
    for (case, value) in special.into_iter().enumerate() {
        let ray = Ray {
            p: v(value, 0.0),
            d: v(1.0, value),
            t: value,
        };
        let circle = Circle {
            p: v(0.0, value),
            r: value,
        };
        compare_ray_circle(&c, &rust, ray, circle, &format!("circle IEEE {case}"));
    }

    let miss = Ray {
        p: v(-5.0, 5.0),
        d: v(1.0, 0.0),
        t: 2.0,
    };
    let circle = Circle {
        p: v(0.0, 0.0),
        r: 1.0,
    };
    let c_result = unsafe { (c.c2_ray_to_circle)(miss, circle, std::ptr::null_mut()) };
    let rust_result = unsafe { (rust.c2_ray_to_circle)(miss, circle, std::ptr::null_mut()) };
    same("c2RaytoCircle null output on miss", c_result, rust_result);
    assert_eq!(c_result, 0);
}

#[test]
fn ray_aabb_configuration_rows_45_through_52_and_error_rows_13_through_15_24() {
    let (c, rust) = apis();
    let mut rng = Rng::new(0x1bf4_a205);
    for case in 0..1024 {
        let base = v(
            (rng.u32() % 201) as f32 - 100.0,
            (rng.u32() % 201) as f32 - 100.0,
        );
        let width = (rng.u32() % 20 + 2) as f32;
        let height = (rng.u32() % 20 + 2) as f32;
        let gap = (rng.u32() % 10 + 1) as f32;
        let aabb = Aabb {
            min: base,
            max: translated(base, width, height),
        };
        let rays = [
            (
                "left",
                Ray {
                    p: translated(base, -gap, height * 0.5),
                    d: v(1.0, 0.0),
                    t: gap + width + 1.0,
                },
            ),
            (
                "right",
                Ray {
                    p: translated(base, width + gap, height * 0.5),
                    d: v(-1.0, 0.0),
                    t: gap + width + 1.0,
                },
            ),
            (
                "bottom",
                Ray {
                    p: translated(base, width * 0.5, -gap),
                    d: v(0.0, 1.0),
                    t: gap + height + 1.0,
                },
            ),
            (
                "top",
                Ray {
                    p: translated(base, width * 0.5, height + gap),
                    d: v(0.0, -1.0),
                    t: gap + height + 1.0,
                },
            ),
        ];
        for (face, ray) in rays {
            assert_eq!(
                compare_ray_aabb(&c, &rust, ray, aabb, &format!("AABB {face} hit {case}")),
                1
            );
        }

        let starts_inside = Ray {
            p: translated(base, width * 0.5, height * 0.5),
            d: v(1.0, 0.25),
            t: width + height,
        };
        assert_eq!(
            compare_ray_aabb(
                &c,
                &rust,
                starts_inside,
                aabb,
                &format!("AABB starts inside {case}"),
            ),
            1
        );

        let corner = Ray {
            p: translated(base, -gap, -gap),
            d: v(1.0, 1.0),
            t: gap + width.max(height) + 1.0,
        };
        compare_ray_aabb(&c, &rust, corner, aabb, &format!("AABB corner {case}"));

        let zero_length = Ray {
            p: translated(base, width * 0.5, height * 0.5),
            d: v(0.0, 0.0),
            t: 0.0,
        };
        assert_eq!(
            compare_ray_aabb(
                &c,
                &rust,
                zero_length,
                aabb,
                &format!("AABB zero length {case}"),
            ),
            1
        );

        let starts_on_boundary = Ray {
            p: translated(base, 0.0, height * 0.5),
            d: v(1.0, 0.0),
            t: width,
        };
        assert_eq!(
            compare_ray_aabb(
                &c,
                &rust,
                starts_on_boundary,
                aabb,
                &format!("AABB starts on boundary {case}"),
            ),
            1
        );

        let degenerate_box = Aabb {
            min: translated(base, width * 0.5, height * 0.5),
            max: translated(base, width * 0.5, height * 0.5),
        };
        let through_point = Ray {
            p: translated(base, -gap, height * 0.5),
            d: v(1.0, 0.0),
            t: gap + width,
        };
        compare_ray_aabb(
            &c,
            &rust,
            through_point,
            degenerate_box,
            &format!("AABB degenerate box {case}"),
        );

        let broad_miss = Ray {
            p: translated(base, -gap - 10.0, height + gap + 10.0),
            d: v(-1.0, 0.0),
            t: 5.0,
        };
        assert_eq!(
            compare_ray_aabb(
                &c,
                &rust,
                broad_miss,
                aabb,
                &format!("AABB broad-phase miss {case}"),
            ),
            0
        );

        let sat_miss = Ray {
            p: translated(base, -5.0, height - 1.0),
            d: v(width + 4.0, 6.0),
            t: 1.0,
        };
        assert_eq!(
            compare_ray_aabb(&c, &rust, sat_miss, aabb, &format!("AABB SAT miss {case}")),
            0
        );

        let reversed = Aabb {
            min: aabb.max,
            max: aabb.min,
        };
        compare_ray_aabb(
            &c,
            &rust,
            starts_inside,
            reversed,
            &format!("AABB reversed box {case}"),
        );
    }

    let nan = f32::from_bits(0x7fc1_2345);
    let nan_ray = Ray {
        p: v(nan, nan),
        d: v(0.0, 0.0),
        t: 1.0,
    };
    let box_ = Aabb {
        min: v(0.0, 0.0),
        max: v(10.0, 10.0),
    };
    assert_eq!(
        compare_ray_aabb(&c, &rust, nan_ray, box_, "AABB all plane tests unordered"),
        0
    );

    let special = [f32::INFINITY, f32::NEG_INFINITY, nan, -0.0];
    for (case, value) in special.into_iter().enumerate() {
        let ray = Ray {
            p: v(value, 1.0),
            d: v(1.0, value),
            t: value,
        };
        let shape = Aabb {
            min: v(-value, 0.0),
            max: v(value, 2.0),
        };
        compare_ray_aabb(&c, &rust, ray, shape, &format!("AABB IEEE {case}"));
    }

    let miss = Ray {
        p: v(-5.0, 20.0),
        d: v(-1.0, 0.0),
        t: 2.0,
    };
    let c_result = unsafe { (c.c2_ray_to_aabb)(miss, box_, std::ptr::null_mut()) };
    let rust_result = unsafe { (rust.c2_ray_to_aabb)(miss, box_, std::ptr::null_mut()) };
    same("c2RaytoAABB null output on miss", c_result, rust_result);
    assert_eq!(c_result, 0);
}

#[test]
fn capsule_and_dispatch_configuration_rows_53_through_65_and_error_rows_16_through_21_29_30() {
    let (c, rust) = apis();
    let mut rng = Rng::new(0xa114_7e2b);
    for case in 0..1024 {
        let base = v(
            (rng.u32() % 201) as f32 - 100.0,
            (rng.u32() % 201) as f32 - 100.0,
        );
        let length = (rng.u32() % 20 + 4) as f32;
        let radius = (rng.u32() % 4 + 1) as f32;
        let gap = (rng.u32() % 5 + 2) as f32;
        let capsule = Capsule {
            a: base,
            b: translated(base, 0.0, length),
            r: radius,
        };

        let starts_body = Ray {
            p: translated(base, radius * 0.5, length * 0.5),
            d: v(1.0, 0.0),
            t: 1.0,
        };
        assert_eq!(
            compare_ray_capsule(
                &c,
                &rust,
                starts_body,
                capsule,
                &format!("capsule starts in body {case}"),
            ),
            1
        );

        let starts_a = Ray {
            p: translated(base, 0.0, -radius * 0.5),
            d: v(0.0, -1.0),
            t: 1.0,
        };
        assert_eq!(
            compare_ray_capsule(
                &c,
                &rust,
                starts_a,
                capsule,
                &format!("capsule starts in cap A {case}"),
            ),
            1
        );

        let starts_b = Ray {
            p: translated(base, 0.0, length + radius * 0.5),
            d: v(0.0, 1.0),
            t: 1.0,
        };
        assert_eq!(
            compare_ray_capsule(
                &c,
                &rust,
                starts_b,
                capsule,
                &format!("capsule starts in cap B {case}"),
            ),
            1
        );

        let positive_side = Ray {
            p: translated(base, radius + gap, length * 0.5),
            d: v(-1.0, 0.0),
            t: gap + radius * 2.0 + 1.0,
        };
        assert_eq!(
            compare_ray_capsule(
                &c,
                &rust,
                positive_side,
                capsule,
                &format!("capsule positive side {case}"),
            ),
            1
        );

        let negative_side = Ray {
            p: translated(base, -radius - gap, length * 0.5),
            d: v(1.0, 0.0),
            t: gap + radius * 2.0 + 1.0,
        };
        assert_eq!(
            compare_ray_capsule(
                &c,
                &rust,
                negative_side,
                capsule,
                &format!("capsule negative side {case}"),
            ),
            1
        );

        let cap_a_hit = Ray {
            p: translated(base, 0.0, -radius - gap),
            d: v(0.0, 1.0),
            t: gap + radius * 2.0 + 1.0,
        };
        assert_eq!(
            compare_ray_capsule(
                &c,
                &rust,
                cap_a_hit,
                capsule,
                &format!("capsule cap A hit {case}"),
            ),
            1
        );

        let cap_b_hit = Ray {
            p: translated(base, 0.0, length + radius + gap),
            d: v(0.0, -1.0),
            t: gap + radius * 2.0 + 1.0,
        };
        assert_eq!(
            compare_ray_capsule(
                &c,
                &rust,
                cap_b_hit,
                capsule,
                &format!("capsule cap B hit {case}"),
            ),
            1
        );

        let final_miss = Ray {
            p: translated(base, radius + gap, length * 0.5),
            d: v(1.0, 0.0),
            t: gap,
        };
        assert_eq!(
            compare_ray_capsule(
                &c,
                &rust,
                final_miss,
                capsule,
                &format!("capsule final miss {case}"),
            ),
            0
        );

        let delegated_a_miss = Ray {
            p: translated(base, radius * 0.5, -radius - gap),
            d: v(0.0, -1.0),
            t: gap,
        };
        assert_eq!(
            compare_ray_capsule(
                &c,
                &rust,
                delegated_a_miss,
                capsule,
                &format!("capsule delegated A miss {case}"),
            ),
            0
        );

        let delegated_b_miss = Ray {
            p: translated(base, radius * 0.5, length + radius + gap),
            d: v(0.0, 1.0),
            t: gap,
        };
        assert_eq!(
            compare_ray_capsule(
                &c,
                &rust,
                delegated_b_miss,
                capsule,
                &format!("capsule delegated B miss {case}"),
            ),
            0
        );

        let crossing_below_miss = Ray {
            p: translated(base, radius + gap, -radius - gap),
            d: v(-1.0, 0.0),
            t: (radius + gap) * 2.0,
        };
        assert_eq!(
            compare_ray_capsule(
                &c,
                &rust,
                crossing_below_miss,
                capsule,
                &format!("capsule crossing below miss {case}"),
            ),
            0
        );

        let crossing_above_miss = Ray {
            p: translated(base, radius + gap, length + radius + gap),
            d: v(-1.0, 0.0),
            t: (radius + gap) * 2.0,
        };
        assert_eq!(
            compare_ray_capsule(
                &c,
                &rust,
                crossing_above_miss,
                capsule,
                &format!("capsule crossing above miss {case}"),
            ),
            0
        );

        let circle = Circle {
            p: translated(base, length * 0.25, length * 0.25),
            r: radius,
        };
        let circle_ray = Ray {
            p: translated(circle.p, -radius - gap, 0.0),
            d: v(1.0, 0.0),
            t: gap + radius * 2.0,
        };
        assert_eq!(
            compare_cast(
                &c,
                &rust,
                circle_ray,
                (&circle as *const Circle).cast(),
                0,
                &format!("dispatch circle {case}"),
            ),
            1
        );
        let circle_miss_ray = Ray {
            p: translated(circle.p, -radius - gap, radius + 1.0),
            d: v(1.0, 0.0),
            t: gap + radius * 2.0,
        };
        assert_eq!(
            compare_cast(
                &c,
                &rust,
                circle_miss_ray,
                (&circle as *const Circle).cast(),
                0,
                &format!("dispatch circle miss {case}"),
            ),
            0
        );

        let aabb = Aabb {
            min: base,
            max: translated(base, length, length),
        };
        let aabb_ray = Ray {
            p: translated(base, -gap, length * 0.5),
            d: v(1.0, 0.0),
            t: gap + length + 1.0,
        };
        assert_eq!(
            compare_cast(
                &c,
                &rust,
                aabb_ray,
                (&aabb as *const Aabb).cast(),
                1,
                &format!("dispatch AABB {case}"),
            ),
            1
        );
        let aabb_miss_ray = Ray {
            p: translated(base, -gap, length + gap),
            d: v(-1.0, 0.0),
            t: gap,
        };
        assert_eq!(
            compare_cast(
                &c,
                &rust,
                aabb_miss_ray,
                (&aabb as *const Aabb).cast(),
                1,
                &format!("dispatch AABB miss {case}"),
            ),
            0
        );

        assert_eq!(
            compare_cast(
                &c,
                &rust,
                positive_side,
                (&capsule as *const Capsule).cast(),
                2,
                &format!("dispatch capsule {case}"),
            ),
            1
        );
        assert_eq!(
            compare_cast(
                &c,
                &rust,
                final_miss,
                (&capsule as *const Capsule).cast(),
                2,
                &format!("dispatch capsule miss {case}"),
            ),
            0
        );

        let zero_extent = Ray {
            p: translated(base, radius + gap, length * 0.5),
            d: v(-1.0, 0.0),
            t: 0.0,
        };
        compare_ray_capsule(
            &c,
            &rust,
            zero_extent,
            capsule,
            &format!("capsule zero extent {case}"),
        );
    }

    let base_capsule = Capsule {
        a: v(0.0, 0.0),
        b: v(0.0, 10.0),
        r: 1.0,
    };
    let ordinary_ray = Ray {
        p: v(3.0, 5.0),
        d: v(-1.0, 0.0),
        t: 6.0,
    };
    let degenerate_shapes = [
        Capsule {
            a: v(0.0, 0.0),
            b: v(0.0, 0.0),
            r: 1.0,
        },
        Capsule {
            a: base_capsule.b,
            b: base_capsule.a,
            r: 1.0,
        },
        Capsule {
            r: 0.0,
            ..base_capsule
        },
        Capsule {
            r: -1.0,
            ..base_capsule
        },
    ];
    for (case, capsule) in degenerate_shapes.into_iter().enumerate() {
        compare_ray_capsule(
            &c,
            &rust,
            ordinary_ray,
            capsule,
            &format!("capsule degenerate {case}"),
        );
    }
    let nan = f32::from_bits(0x7fc1_2345);
    compare_ray_capsule(
        &c,
        &rust,
        Ray {
            p: v(nan, 0.0),
            d: v(0.0, nan),
            t: nan,
        },
        Capsule {
            a: v(0.0, nan),
            b: v(nan, 1.0),
            r: nan,
        },
        "capsule IEEE",
    );

    #[cfg(target_arch = "x86_64")]
    {
        let mut invalid_rng = Rng::new(0xc241_59ad);
        for case in 0..1024 {
            let seed = invalid_rng.u32();
            let ray = Ray {
                p: invalid_rng.v(),
                d: invalid_rng.v(),
                t: invalid_rng.finite(),
            };
            for shape_type in [-1, 3] {
                let mut c_out = sentinel();
                let mut rust_out = sentinel();
                let shape = (&base_capsule as *const Capsule).cast();
                let c_result =
                    call_cast_with_eax(c.c2_cast_ray, &ray, shape, shape_type, &mut c_out, seed);
                let rust_result = call_cast_with_eax(
                    rust.c2_cast_ray,
                    &ray,
                    shape,
                    shape_type,
                    &mut rust_out,
                    seed,
                );
                same(
                    &format!("invalid shape discriminator {shape_type}/{case}"),
                    c_result,
                    rust_result,
                );
                same(
                    &format!("invalid shape output {shape_type}/{case}"),
                    c_out,
                    rust_out,
                );
                assert_eq!(c_result as u32, seed);
            }
        }
        let seed = 0x91e1_0da5;
        let c_invalid = call_cast_with_eax(
            c.c2_cast_ray,
            &ordinary_ray,
            std::ptr::null(),
            -1,
            std::ptr::null_mut(),
            seed,
        );
        let rust_invalid = call_cast_with_eax(
            rust.c2_cast_ray,
            &ordinary_ray,
            std::ptr::null(),
            -1,
            std::ptr::null_mut(),
            seed,
        );
        same("invalid shape with null pointers", c_invalid, rust_invalid);
        assert_eq!(c_invalid as u32, seed);
    }

    let miss_circle = Circle {
        p: v(0.0, 0.0),
        r: 1.0,
    };
    let miss_ray = Ray {
        p: v(-5.0, 5.0),
        d: v(1.0, 0.0),
        t: 2.0,
    };
    let c_miss = unsafe {
        (c.c2_cast_ray)(
            miss_ray,
            (&miss_circle as *const Circle).cast(),
            0,
            std::ptr::null_mut(),
        )
    };
    let rust_miss = unsafe {
        (rust.c2_cast_ray)(
            miss_ray,
            (&miss_circle as *const Circle).cast(),
            0,
            std::ptr::null_mut(),
        )
    };
    same("dispatch null output on miss", c_miss, rust_miss);
    assert_eq!(c_miss, 0);
}

#[test]
fn spec_ray_configuration_rows_66_through_68_and_error_row_32() {
    let (c, rust) = apis();
    let mut rng = Rng::new(0xd3c4_9907);
    for case in 0..2048 {
        let center = v(
            (rng.u32() % 201) as f32 - 100.0,
            (rng.u32() % 201) as f32 - 100.0,
        );
        let radius = (rng.u32() % 20 + 1) as f32;
        let gap = (rng.u32() % 20 + 1) as f32;
        let ray_origin = translated(center, -radius - gap, 0.0);
        let mouse_hit = translated(center, radius + gap, 0.0);
        let mut c_out = sentinel();
        let mut rust_out = sentinel();
        let c_result = unsafe {
            (c.spec_ray)(
                &mut c_out,
                mouse_hit.x,
                mouse_hit.y,
                center.x,
                center.y,
                radius,
                ray_origin.x,
                ray_origin.y,
            )
        };
        let rust_result = unsafe {
            (rust.spec_ray)(
                &mut rust_out,
                mouse_hit.x,
                mouse_hit.y,
                center.x,
                center.y,
                radius,
                ray_origin.x,
                ray_origin.y,
            )
        };
        same(&format!("spec hit return {case}"), c_result, rust_result);
        same(&format!("spec hit output {case}"), c_out, rust_out);
        assert_eq!(c_result, 1);

        c_out = sentinel();
        rust_out = sentinel();
        let c_result = unsafe {
            (c.spec_ray)(
                &mut c_out,
                mouse_hit.x,
                mouse_hit.y,
                center.x,
                center.y,
                -radius,
                ray_origin.x,
                ray_origin.y,
            )
        };
        let rust_result = unsafe {
            (rust.spec_ray)(
                &mut rust_out,
                mouse_hit.x,
                mouse_hit.y,
                center.x,
                center.y,
                -radius,
                ray_origin.x,
                ray_origin.y,
            )
        };
        same(
            &format!("spec negative radius return {case}"),
            c_result,
            rust_result,
        );
        same(
            &format!("spec negative radius output {case}"),
            c_out,
            rust_out,
        );
        assert_eq!(c_result, 1);

        let miss_origin = translated(center, -radius - gap, radius + 1.0);
        let miss_mouse = translated(center, radius + gap, radius + 1.0);
        c_out = sentinel();
        rust_out = sentinel();
        let c_result = unsafe {
            (c.spec_ray)(
                &mut c_out,
                miss_mouse.x,
                miss_mouse.y,
                center.x,
                center.y,
                radius,
                miss_origin.x,
                miss_origin.y,
            )
        };
        let rust_result = unsafe {
            (rust.spec_ray)(
                &mut rust_out,
                miss_mouse.x,
                miss_mouse.y,
                center.x,
                center.y,
                radius,
                miss_origin.x,
                miss_origin.y,
            )
        };
        same(&format!("spec miss return {case}"), c_result, rust_result);
        same(&format!("spec miss output {case}"), c_out, rust_out);
        assert_eq!(c_result, 0);

        let tangent_origin = translated(center, -radius - gap, radius);
        let tangent_mouse = translated(center, radius + gap, radius);
        c_out = sentinel();
        rust_out = sentinel();
        let c_result = unsafe {
            (c.spec_ray)(
                &mut c_out,
                tangent_mouse.x,
                tangent_mouse.y,
                center.x,
                center.y,
                radius,
                tangent_origin.x,
                tangent_origin.y,
            )
        };
        let rust_result = unsafe {
            (rust.spec_ray)(
                &mut rust_out,
                tangent_mouse.x,
                tangent_mouse.y,
                center.x,
                center.y,
                radius,
                tangent_origin.x,
                tangent_origin.y,
            )
        };
        same(
            &format!("spec tangent return {case}"),
            c_result,
            rust_result,
        );
        same(&format!("spec tangent output {case}"), c_out, rust_out);
    }

    let special = [
        0.0,
        -0.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::from_bits(0x7fc1_2345),
    ];
    for (case, value) in special.into_iter().enumerate() {
        let mut c_out = sentinel();
        let mut rust_out = sentinel();
        let c_result =
            unsafe { (c.spec_ray)(&mut c_out, value, value, 0.0, value, value, value, value) };
        let rust_result = unsafe {
            (rust.spec_ray)(&mut rust_out, value, value, 0.0, value, value, value, value)
        };
        same(&format!("spec IEEE return {case}"), c_result, rust_result);
        same(&format!("spec IEEE output {case}"), c_out, rust_out);
    }

    let c_miss = unsafe { (c.spec_ray)(std::ptr::null_mut(), 1.0, 5.0, 0.0, 0.0, 1.0, -5.0, 5.0) };
    let rust_miss =
        unsafe { (rust.spec_ray)(std::ptr::null_mut(), 1.0, 5.0, 0.0, 0.0, 1.0, -5.0, 5.0) };
    same("spec null output on miss", c_miss, rust_miss);
    assert_eq!(c_miss, 0);
}

#[test]
fn ffi_crash_child() {
    let Ok(library_path) = std::env::var("DIFF_CRASH_LIBRARY") else {
        return;
    };
    let crash_case = std::env::var("DIFF_CRASH_CASE").unwrap();
    let api = unsafe { Api::load(Path::new(&library_path)) };
    let hit_ray = Ray {
        p: v(-5.0, 0.0),
        d: v(1.0, 0.0),
        t: 10.0,
    };
    let circle = Circle {
        p: v(0.0, 0.0),
        r: 1.0,
    };
    let aabb = Aabb {
        min: v(0.0, -1.0),
        max: v(2.0, 1.0),
    };
    let capsule = Capsule {
        a: v(0.0, 0.0),
        b: v(0.0, 10.0),
        r: 1.0,
    };
    let mut output = sentinel();
    unsafe {
        match crash_case.as_str() {
            "circle_out_hit" => {
                (api.c2_ray_to_circle)(hit_ray, circle, std::ptr::null_mut());
            }
            "aabb_out_hit" => {
                (api.c2_ray_to_aabb)(hit_ray, aabb, std::ptr::null_mut());
            }
            "capsule_out" => {
                (api.c2_ray_to_capsule)(hit_ray, capsule, std::ptr::null_mut());
            }
            "cast_shape_null" => {
                (api.c2_cast_ray)(hit_ray, std::ptr::null(), 0, &mut output);
            }
            "cast_out_hit" => {
                (api.c2_cast_ray)(
                    hit_ray,
                    (&circle as *const Circle).cast(),
                    0,
                    std::ptr::null_mut(),
                );
            }
            "spec_out_hit" => {
                (api.spec_ray)(std::ptr::null_mut(), 5.0, 0.0, 0.0, 0.0, 1.0, -5.0, 0.0);
            }
            other => panic!("unknown crash case {other}"),
        }
    }
    panic!("{crash_case} unexpectedly survived");
}

fn crash_status(library: &Path, crash_case: &str) -> std::process::ExitStatus {
    Command::new(std::env::current_exe().unwrap())
        .arg("--exact")
        .arg("ffi_crash_child")
        .arg("--nocapture")
        .env("DIFF_CRASH_LIBRARY", library)
        .env("DIFF_CRASH_CASE", crash_case)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .unwrap()
}

#[test]
fn null_pointer_crash_error_rows_23_25_26_27_28_31() {
    use std::os::unix::process::ExitStatusExt;

    let (c_library, rust_library) = library_paths();
    for crash_case in [
        "circle_out_hit",
        "aabb_out_hit",
        "capsule_out",
        "cast_shape_null",
        "cast_out_hit",
        "spec_out_hit",
    ] {
        let c_status = crash_status(&c_library, crash_case);
        let rust_status = crash_status(&rust_library, crash_case);
        assert_eq!(
            c_status.signal(),
            rust_status.signal(),
            "{crash_case}: C status {c_status:?}, Rust status {rust_status:?}"
        );
        assert_eq!(
            c_status.signal(),
            Some(11),
            "{crash_case}: C did not receive SIGSEGV: {c_status:?}"
        );
    }
}
