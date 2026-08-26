use libloading::Library;
use std::ffi::{c_float, c_int, c_void};
use std::path::PathBuf;
use std::process::Command;
use std::ptr;

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
#[derive(Clone, Copy)]
struct Circle {
    p: V,
    r: c_float,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Aabb {
    min: V,
    max: V,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Capsule {
    a: V,
    b: V,
    r: c_float,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Ray {
    p: V,
    d: V,
    t: c_float,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct M {
    x: V,
    y: V,
}

struct Rng(u64);

impl Rng {
    fn new() -> Self {
        Self(0x5eed_cafe_d15c_a11e)
    }

    fn next(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0 as u32
    }

    fn finite(&mut self) -> f32 {
        (self.next() as i32 % 32768) as f32 / 256.0
    }

    fn positive(&mut self) -> f32 {
        (self.next() % 4095 + 1) as f32 / 128.0
    }
}

fn libraries() -> (Library, Library) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c = root.join("c_src/build/libtranslated_rust.so");
    let rust = root.join("target/release/libspec_ray_lib.so");
    assert!(c.is_file(), "missing C library: {}", c.display());
    assert!(rust.is_file(), "missing Rust library: {}", rust.display());
    unsafe {
        (
            Library::new(c).expect("load C library"),
            Library::new(rust).expect("load Rust library"),
        )
    }
}

unsafe fn sym<T: Copy>(library: &Library, name: &[u8]) -> T {
    *unsafe { library.get::<T>(name) }.unwrap()
}

fn assert_float(c: f32, rust: f32, context: &str) {
    assert_eq!(
        c.to_bits(),
        rust.to_bits(),
        "{context}: C={c:?} ({:08x}), Rust={rust:?} ({:08x})",
        c.to_bits(),
        rust.to_bits()
    );
}

fn assert_v(c: V, rust: V, context: &str) {
    assert_float(c.x, rust.x, &format!("{context}.x"));
    assert_float(c.y, rust.y, &format!("{context}.y"));
}

fn assert_cast(c_ret: i32, c: Raycast, rust_ret: i32, rust: Raycast, context: &str) {
    assert_eq!(c_ret, rust_ret, "{context}: return");
    assert_float(c.t, rust.t, &format!("{context}.t"));
    assert_v(c.n, rust.n, &format!("{context}.n"));
}

fn sentinel() -> Raycast {
    Raycast {
        t: f32::from_bits(0x7fc1_2345),
        n: V {
            x: f32::from_bits(0x7fc2_3456),
            y: f32::from_bits(0xffc3_4567),
        },
    }
}

#[cfg(target_arch = "x86_64")]
#[inline(never)]
unsafe fn call_invalid_seeded(
    function: unsafe extern "C" fn(Ray, *const c_void, c_int, *mut Raycast) -> c_int,
    ray: Ray,
    payload: *const c_void,
    out: *mut Raycast,
    seed: c_int,
) -> c_int {
    let result: c_int;
    unsafe {
        core::arch::asm!(
            "sub rsp, 32",
            "mov rax, qword ptr [r10]",
            "mov qword ptr [rsp], rax",
            "mov rax, qword ptr [r10 + 8]",
            "mov qword ptr [rsp + 8], rax",
            "mov eax, dword ptr [r10 + 16]",
            "mov dword ptr [rsp + 16], eax",
            "mov eax, {seed:e}",
            "call r11",
            "add rsp, 32",
            in("r10") &ray,
            seed = in(reg) seed,
            in("r11") function,
            in("rdi") payload,
            in("esi") 3,
            in("rdx") out,
            lateout("eax") result,
            clobber_abi("C"),
        );
    }
    result
}

#[test]
fn vector_and_matrix_surface() {
    type V2 = unsafe extern "C" fn(V, V) -> V;
    type VS = unsafe extern "C" fn(V, f32) -> V;
    type V1 = unsafe extern "C" fn(V) -> V;
    type F2 = unsafe extern "C" fn(V, V) -> f32;
    type F1 = unsafe extern "C" fn(V) -> f32;
    type Make = unsafe extern "C" fn(f32, f32) -> V;
    type MV = unsafe extern "C" fn(M, V) -> V;
    let (c, r) = libraries();
    let pairs: &[(&[u8], bool)] = &[
        (b"c2Add\0", false),
        (b"c2Sub\0", false),
        (b"c2Minv\0", false),
        (b"c2Maxv\0", false),
    ];
    let unary = [
        b"c2Norm\0".as_slice(),
        b"c2Skew\0",
        b"c2CCW90\0",
        b"c2Absv\0",
    ];
    let mut rng = Rng::new();

    unsafe {
        let cv: Make = sym(&c, b"c2V\0");
        let rv: Make = sym(&r, b"c2V\0");
        let cdot: F2 = sym(&c, b"c2Dot\0");
        let rdot: F2 = sym(&r, b"c2Dot\0");
        let clen: F1 = sym(&c, b"c2Len\0");
        let rlen: F1 = sym(&r, b"c2Len\0");
        let cmul: VS = sym(&c, b"c2Mulvs\0");
        let rmul: VS = sym(&r, b"c2Mulvs\0");
        let cdiv: VS = sym(&c, b"c2Div\0");
        let rdiv: VS = sym(&r, b"c2Div\0");
        let cmv: MV = sym(&c, b"c2MulmvT\0");
        let rmv: MV = sym(&r, b"c2MulmvT\0");

        for i in 0..512 {
            let a = V {
                x: rng.finite(),
                y: rng.finite(),
            };
            let b = V {
                x: rng.finite(),
                y: rng.finite(),
            };
            let scalar = if i % 17 == 0 { -0.0 } else { rng.finite() };
            let divisor = if i % 31 == 0 { 0.0 } else { rng.positive() };
            assert_v(cv(a.x, a.y), rv(a.x, a.y), "c2V");
            assert_float(cdot(a, b), rdot(a, b), "c2Dot");
            assert_float(clen(a), rlen(a), "c2Len");
            assert_v(cmul(a, scalar), rmul(a, scalar), "c2Mulvs");
            assert_v(cdiv(a, divisor), rdiv(a, divisor), "c2Div");
            for (name, _) in pairs {
                let cf: V2 = sym(&c, name);
                let rf: V2 = sym(&r, name);
                assert_v(
                    cf(a, b),
                    rf(a, b),
                    std::str::from_utf8(&name[..name.len() - 1]).unwrap(),
                );
            }
            for name in unary {
                let cf: V1 = sym(&c, name);
                let rf: V1 = sym(&r, name);
                assert_v(
                    cf(a),
                    rf(a),
                    std::str::from_utf8(&name[..name.len() - 1]).unwrap(),
                );
            }
            let m = M { x: a, y: b };
            assert_v(cmv(m, b), rmv(m, b), "c2MulmvT");
        }

        let zero = V { x: 0.0, y: -0.0 };
        let cnorm: V1 = sym(&c, b"c2Norm\0");
        let rnorm: V1 = sym(&r, b"c2Norm\0");
        assert_v(cnorm(zero), rnorm(zero), "c2Norm zero");
    }
}

#[test]
fn point_and_box_predicates() {
    type BB = unsafe extern "C" fn(Aabb, Aabb) -> c_int;
    type BP = unsafe extern "C" fn(Aabb, V) -> c_int;
    type CP = unsafe extern "C" fn(Circle, V) -> c_int;
    let (c, r) = libraries();
    unsafe {
        let cbb: BB = sym(&c, b"c2AABBtoAABB\0");
        let rbb: BB = sym(&r, b"c2AABBtoAABB\0");
        let cbp: BP = sym(&c, b"c2AABBtoPoint\0");
        let rbp: BP = sym(&r, b"c2AABBtoPoint\0");
        let ccp: CP = sym(&c, b"c2CircleToPoint\0");
        let rcp: CP = sym(&r, b"c2CircleToPoint\0");
        let mut rng = Rng::new();
        for _ in 0..64 {
            let dx = rng.finite();
            let dy = rng.finite();
            let v = |x: f32, y: f32| V {
                x: x + dx,
                y: y + dy,
            };
            let a = Aabb {
                min: v(-2.0, -3.0),
                max: v(4.0, 5.0),
            };
            let boxes = [
                a,
                Aabb {
                    min: v(4.0, 5.0),
                    max: v(8.0, 9.0),
                },
                Aabb {
                    min: v(-8.0, -1.0),
                    max: v(-3.0, 1.0),
                },
                Aabb {
                    min: v(5.0, -1.0),
                    max: v(8.0, 1.0),
                },
                Aabb {
                    min: v(0.0, -8.0),
                    max: v(1.0, -4.0),
                },
                Aabb {
                    min: v(0.0, 6.0),
                    max: v(1.0, 8.0),
                },
            ];
            for b in boxes {
                assert_eq!(cbb(a, b), rbb(a, b));
            }
            for p in [
                v(0.0, 0.0),
                v(-2.0, 0.0),
                v(4.0, 5.0),
                v(-3.0, 0.0),
                v(5.0, 0.0),
                v(0.0, -4.0),
                v(0.0, 6.0),
            ] {
                assert_eq!(cbp(a, p), rbp(a, p));
            }
            let circle = Circle {
                p: v(1.0, 2.0),
                r: 3.0,
            };
            for p in [v(1.0, 2.0), v(4.0, 2.0), v(4.01, 2.0)] {
                assert_eq!(ccp(circle, p), rcp(circle, p));
            }
        }
    }
}

#[test]
fn ray_to_circle_surface_and_rejections() {
    type F = unsafe extern "C" fn(Ray, Circle, *mut Raycast) -> c_int;
    let (c, r) = libraries();
    unsafe {
        let cf: F = sym(&c, b"c2RaytoCircle\0");
        let rf: F = sym(&r, b"c2RaytoCircle\0");
        let mut rng = Rng::new();
        for i in 0..256 {
            let y = (rng.next() % 1900) as f32 / 1000.0 - 0.95;
            let t = 1.0 + rng.positive();
            let ray = Ray {
                p: V { x: -3.0, y },
                d: V { x: 1.0, y: 0.0 },
                t,
            };
            let circle = Circle {
                p: V { x: 0.0, y: 0.0 },
                r: 1.0,
            };
            let mut co = sentinel();
            let mut ro = sentinel();
            let cr = cf(ray, circle, &mut co);
            let rr = rf(ray, circle, &mut ro);
            assert_cast(cr, co, rr, ro, &format!("random circle {i}"));
        }
        let cases = [
            // Tangency and inclusive t endpoints.
            (
                Ray {
                    p: V { x: -3.0, y: 1.0 },
                    d: V { x: 1.0, y: 0.0 },
                    t: 3.0,
                },
                Circle {
                    p: V { x: 0.0, y: 0.0 },
                    r: 1.0,
                },
            ),
            (
                Ray {
                    p: V { x: -1.0, y: 0.0 },
                    d: V { x: 1.0, y: 0.0 },
                    t: 0.0,
                },
                Circle {
                    p: V { x: 0.0, y: 0.0 },
                    r: 1.0,
                },
            ),
            (
                Ray {
                    p: V { x: -3.0, y: 0.0 },
                    d: V { x: 1.0, y: 0.0 },
                    t: 2.0,
                },
                Circle {
                    p: V { x: 0.0, y: 0.0 },
                    r: 1.0,
                },
            ),
            // ERRORS rows 1-3.
            (
                Ray {
                    p: V { x: -3.0, y: 2.0 },
                    d: V { x: 1.0, y: 0.0 },
                    t: 10.0,
                },
                Circle {
                    p: V { x: 0.0, y: 0.0 },
                    r: 1.0,
                },
            ),
            (
                Ray {
                    p: V { x: 3.0, y: 0.0 },
                    d: V { x: 1.0, y: 0.0 },
                    t: 10.0,
                },
                Circle {
                    p: V { x: 0.0, y: 0.0 },
                    r: 1.0,
                },
            ),
            (
                Ray {
                    p: V { x: -3.0, y: 0.0 },
                    d: V { x: 1.0, y: 0.0 },
                    t: 1.0,
                },
                Circle {
                    p: V { x: 0.0, y: 0.0 },
                    r: 1.0,
                },
            ),
        ];
        for (ray, circle) in cases {
            for shift in -16..16 {
                let dx = shift as f32 * 0.25;
                let ray = Ray {
                    p: V {
                        x: ray.p.x + dx,
                        y: ray.p.y,
                    },
                    ..ray
                };
                let circle = Circle {
                    p: V {
                        x: circle.p.x + dx,
                        y: circle.p.y,
                    },
                    ..circle
                };
                let mut co = sentinel();
                let mut ro = sentinel();
                assert_cast(
                    cf(ray, circle, &mut co),
                    co,
                    rf(ray, circle, &mut ro),
                    ro,
                    "circle case",
                );
            }
        }
        let miss = cases[3];
        assert_eq!(
            cf(miss.0, miss.1, ptr::null_mut()),
            rf(miss.0, miss.1, ptr::null_mut())
        );
    }
}

#[test]
fn ray_to_aabb_surface_and_rejections() {
    type F = unsafe extern "C" fn(Ray, Aabb, *mut Raycast) -> c_int;
    let (c, r) = libraries();
    unsafe {
        let cf: F = sym(&c, b"c2RaytoAABB\0");
        let rf: F = sym(&r, b"c2RaytoAABB\0");
        let b = Aabb {
            min: V { x: -1.0, y: -1.0 },
            max: V { x: 1.0, y: 1.0 },
        };
        let rays = [
            Ray {
                p: V { x: -3.0, y: 0.0 },
                d: V { x: 1.0, y: 0.0 },
                t: 5.0,
            },
            Ray {
                p: V { x: 3.0, y: 0.0 },
                d: V { x: -1.0, y: 0.0 },
                t: 5.0,
            },
            Ray {
                p: V { x: 0.0, y: -3.0 },
                d: V { x: 0.0, y: 1.0 },
                t: 5.0,
            },
            Ray {
                p: V { x: 0.0, y: 3.0 },
                d: V { x: 0.0, y: -1.0 },
                t: 5.0,
            },
            Ray {
                p: V { x: 0.0, y: 0.0 },
                d: V { x: 1.0, y: 1.0 },
                t: 5.0,
            },
            Ray {
                p: V { x: 0.0, y: 0.0 },
                d: V { x: 0.0, y: 0.0 },
                t: 0.0,
            },
            // ERRORS row 8, broad-phase rejection.
            Ray {
                p: V { x: -3.0, y: 3.0 },
                d: V { x: -1.0, y: 0.0 },
                t: 2.0,
            },
            // ERRORS row 9, overlapping segment AABB but separating line.
            Ray {
                p: V { x: -2.0, y: 0.9 },
                d: V { x: 1.0, y: 0.6 },
                t: 3.0,
            },
        ];
        for ray in rays {
            for shift in -16..16 {
                let delta = shift as f32 * 0.25;
                let moved_ray = Ray {
                    p: V {
                        x: ray.p.x + delta,
                        y: ray.p.y - delta,
                    },
                    ..ray
                };
                let moved_box = Aabb {
                    min: V {
                        x: b.min.x + delta,
                        y: b.min.y - delta,
                    },
                    max: V {
                        x: b.max.x + delta,
                        y: b.max.y - delta,
                    },
                };
                let mut co = sentinel();
                let mut ro = sentinel();
                assert_cast(
                    cf(moved_ray, moved_box, &mut co),
                    co,
                    rf(moved_ray, moved_box, &mut ro),
                    ro,
                    "AABB ray",
                );
            }
        }
        // ERRORS row 10: NaNs make all four `t <= 1` checks false.
        let nan = f32::from_bits(0x7fc0_0001);
        let nan_box = Aabb {
            min: V { x: nan, y: nan },
            max: V { x: nan, y: nan },
        };
        let ray = rays[0];
        let mut co = sentinel();
        let mut ro = sentinel();
        assert_cast(
            cf(ray, nan_box, &mut co),
            co,
            rf(ray, nan_box, &mut ro),
            ro,
            "AABB all planes reject",
        );
        assert_eq!(
            cf(rays[6], b, ptr::null_mut()),
            rf(rays[6], b, ptr::null_mut())
        );
    }
}

#[test]
fn ray_to_capsule_surface_and_rejections() {
    type F = unsafe extern "C" fn(Ray, Capsule, *mut Raycast) -> c_int;
    let (c, r) = libraries();
    unsafe {
        let cf: F = sym(&c, b"c2RaytoCapsule\0");
        let rf: F = sym(&r, b"c2RaytoCapsule\0");
        let cap = Capsule {
            a: V { x: 0.0, y: 0.0 },
            b: V { x: 0.0, y: 10.0 },
            r: 1.0,
        };
        let rays = [
            Ray {
                p: V { x: 0.0, y: 5.0 },
                d: V { x: 1.0, y: 0.0 },
                t: 5.0,
            },
            Ray {
                p: V { x: 0.0, y: -0.5 },
                d: V { x: 1.0, y: 0.0 },
                t: 5.0,
            },
            Ray {
                p: V { x: 0.0, y: 10.5 },
                d: V { x: 1.0, y: 0.0 },
                t: 5.0,
            },
            Ray {
                p: V { x: 3.0, y: 5.0 },
                d: V { x: 1.0, y: 0.0 },
                t: 2.0,
            },
            Ray {
                p: V { x: 0.5, y: -2.0 },
                d: V { x: 0.0, y: 1.0 },
                t: 5.0,
            },
            Ray {
                p: V { x: 0.5, y: 12.0 },
                d: V { x: 0.0, y: -1.0 },
                t: 5.0,
            },
            Ray {
                p: V { x: 3.0, y: -2.0 },
                d: V { x: -1.0, y: 0.0 },
                t: 5.0,
            },
            Ray {
                p: V { x: 3.0, y: 12.0 },
                d: V { x: -1.0, y: 0.0 },
                t: 5.0,
            },
            Ray {
                p: V { x: 3.0, y: 5.0 },
                d: V { x: -1.0, y: 0.0 },
                t: 5.0,
            },
            Ray {
                p: V { x: -3.0, y: 5.0 },
                d: V { x: 1.0, y: 0.0 },
                t: 5.0,
            },
        ];
        for (i, ray) in rays.into_iter().enumerate() {
            for shift in -16..16 {
                let delta = shift as f32 * 0.25;
                let moved_ray = Ray {
                    p: V {
                        x: ray.p.x + delta,
                        y: ray.p.y - delta,
                    },
                    ..ray
                };
                let moved_cap = Capsule {
                    a: V {
                        x: cap.a.x + delta,
                        y: cap.a.y - delta,
                    },
                    b: V {
                        x: cap.b.x + delta,
                        y: cap.b.y - delta,
                    },
                    r: cap.r,
                };
                let mut co = sentinel();
                let mut ro = sentinel();
                assert_cast(
                    cf(moved_ray, moved_cap, &mut co),
                    co,
                    rf(moved_ray, moved_cap, &mut ro),
                    ro,
                    &format!("capsule branch {i}"),
                );
            }
        }
        let mut rng = Rng::new();
        for i in 0..256 {
            let ray = Ray {
                p: V {
                    x: rng.finite() / 16.0,
                    y: rng.finite() / 8.0 + 5.0,
                },
                d: V {
                    x: rng.finite() / 32.0,
                    y: rng.finite() / 32.0,
                },
                t: rng.positive(),
            };
            let mut co = sentinel();
            let mut ro = sentinel();
            assert_cast(
                cf(ray, cap, &mut co),
                co,
                rf(ray, cap, &mut ro),
                ro,
                &format!("random capsule {i}"),
            );
        }
    }
}

#[test]
fn dispatcher_and_spec_ray_surface() {
    type Cast = unsafe extern "C" fn(Ray, *const c_void, c_int, *mut Raycast) -> c_int;
    type Spec = unsafe extern "C" fn(*mut Raycast, f32, f32, f32, f32, f32, f32, f32) -> c_int;
    let (c, r) = libraries();
    unsafe {
        let cc: Cast = sym(&c, b"c2CastRay\0");
        let rc: Cast = sym(&r, b"c2CastRay\0");
        let ray = Ray {
            p: V { x: -3.0, y: 0.0 },
            d: V { x: 1.0, y: 0.0 },
            t: 6.0,
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
            r: 1.0,
        };
        let payloads = [
            (&circle as *const Circle as *const c_void, 0),
            (&aabb as *const Aabb as *const c_void, 1),
            (&capsule as *const Capsule as *const c_void, 2),
        ];
        for (payload, kind) in payloads {
            let mut co = sentinel();
            let mut ro = sentinel();
            assert_cast(
                cc(ray, payload, kind, &mut co),
                co,
                rc(ray, payload, kind, &mut ro),
                ro,
                "dispatch",
            );
        }
        // ERRORS row 19: C falls through with the incoming EAX register.
        #[cfg(target_arch = "x86_64")]
        for seed in [0, 1, -1, 0x1357_2468] {
            let mut co = sentinel();
            let mut ro = sentinel();
            let payload = &circle as *const Circle as *const c_void;
            let cr = call_invalid_seeded(cc, ray, payload, &mut co, seed);
            let rr = call_invalid_seeded(rc, ray, payload, &mut ro, seed);
            assert_eq!(cr, rr, "out-of-range C2_TYPE");
            assert_cast(cr, co, rr, ro, "invalid dispatch output");
        }

        let cs: Spec = sym(&c, b"spec_ray\0");
        let rs: Spec = sym(&r, b"spec_ray\0");
        let mut rng = Rng::new();
        for i in 0..256 {
            let mp_x = rng.finite();
            let mp_y = rng.finite();
            let cp_x = rng.finite();
            let cp_y = rng.finite();
            let radius = rng.positive();
            let rp_x = rng.finite();
            let rp_y = rng.finite();
            let mut co = sentinel();
            let mut ro = sentinel();
            assert_cast(
                cs(&mut co, mp_x, mp_y, cp_x, cp_y, radius, rp_x, rp_y),
                co,
                rs(&mut ro, mp_x, mp_y, cp_x, cp_y, radius, rp_x, rp_y),
                ro,
                &format!("spec_ray {i}"),
            );
        }
        let mut co = sentinel();
        let mut ro = sentinel();
        assert_cast(
            cs(&mut co, 1.0, 2.0, 5.0, 6.0, 1.0, 1.0, 2.0),
            co,
            rs(&mut ro, 1.0, 2.0, 5.0, 6.0, 1.0, 1.0, 2.0),
            ro,
            "spec_ray degenerate",
        );
        // Null output on a delegated miss is accepted because neither side writes it.
        assert_eq!(
            cs(ptr::null_mut(), 10.0, 0.0, 0.0, 10.0, 1.0, 0.0, 0.0),
            rs(ptr::null_mut(), 10.0, 0.0, 0.0, 10.0, 1.0, 0.0, 0.0)
        );
    }
}

#[test]
fn null_pointer_probe() {
    let Ok(path) = std::env::var("DIFF_PROBE_LIB") else {
        return;
    };
    let case = std::env::var("DIFF_PROBE_CASE").unwrap();
    let library = unsafe { Library::new(path).unwrap() };
    unsafe {
        let ray = Ray {
            p: V { x: -3.0, y: 0.0 },
            d: V { x: 1.0, y: 0.0 },
            t: 6.0,
        };
        match case.as_str() {
            "circle_out" => {
                let f: unsafe extern "C" fn(Ray, Circle, *mut Raycast) -> c_int =
                    sym(&library, b"c2RaytoCircle\0");
                f(
                    ray,
                    Circle {
                        p: V { x: 0.0, y: 0.0 },
                        r: 1.0,
                    },
                    ptr::null_mut(),
                );
            }
            "aabb_out" => {
                let f: unsafe extern "C" fn(Ray, Aabb, *mut Raycast) -> c_int =
                    sym(&library, b"c2RaytoAABB\0");
                f(
                    ray,
                    Aabb {
                        min: V { x: -1.0, y: -1.0 },
                        max: V { x: 1.0, y: 1.0 },
                    },
                    ptr::null_mut(),
                );
            }
            "capsule_out" => {
                let f: unsafe extern "C" fn(Ray, Capsule, *mut Raycast) -> c_int =
                    sym(&library, b"c2RaytoCapsule\0");
                f(
                    Ray {
                        p: V { x: 0.0, y: 5.0 },
                        ..ray
                    },
                    Capsule {
                        a: V { x: 0.0, y: 0.0 },
                        b: V { x: 0.0, y: 10.0 },
                        r: 1.0,
                    },
                    ptr::null_mut(),
                );
            }
            "dispatch_payload" => {
                let f: unsafe extern "C" fn(Ray, *const c_void, c_int, *mut Raycast) -> c_int =
                    sym(&library, b"c2CastRay\0");
                let mut out = sentinel();
                f(ray, ptr::null(), 0, &mut out);
            }
            "spec_out" => {
                let f: unsafe extern "C" fn(
                    *mut Raycast,
                    f32,
                    f32,
                    f32,
                    f32,
                    f32,
                    f32,
                    f32,
                ) -> c_int = sym(&library, b"spec_ray\0");
                f(ptr::null_mut(), 3.0, 0.0, 0.0, 0.0, 1.0, -3.0, 0.0);
            }
            _ => panic!("unknown probe case"),
        }
    }
}

#[cfg(unix)]
#[test]
fn null_pointer_dereferences_match() {
    use std::os::unix::process::ExitStatusExt;

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let executable = std::env::current_exe().unwrap();
    let libraries = [
        root.join("c_src/build/libtranslated_rust.so"),
        root.join("target/release/libspec_ray_lib.so"),
    ];
    for case in [
        "circle_out",
        "aabb_out",
        "capsule_out",
        "dispatch_payload",
        "spec_out",
    ] {
        let statuses: [std::process::ExitStatus; 2] = std::array::from_fn(|index| {
            Command::new(&executable)
                .args(["--exact", "null_pointer_probe", "--nocapture"])
                .env("DIFF_PROBE_LIB", &libraries[index])
                .env("DIFF_PROBE_CASE", case)
                .status()
                .unwrap()
        });
        assert!(
            !statuses[0].success(),
            "C unexpectedly accepted null in {case}"
        );
        assert!(
            !statuses[1].success(),
            "Rust unexpectedly accepted null in {case}"
        );
        assert_eq!(
            statuses[0].signal(),
            statuses[1].signal(),
            "different null-pointer termination for {case}"
        );
    }
}
