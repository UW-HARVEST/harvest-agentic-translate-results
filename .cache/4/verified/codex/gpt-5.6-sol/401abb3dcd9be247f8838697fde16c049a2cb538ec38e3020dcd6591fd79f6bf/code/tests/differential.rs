use libloading::{Library, Symbol};
use std::ffi::{c_float, c_int, c_void};
use std::path::{Path, PathBuf};

const ITERATIONS: usize = 512;
const SPECIAL_FLOATS: [f32; 10] = [
    f32::from_bits(0x0000_0000),
    f32::from_bits(0x8000_0000),
    f32::from_bits(0x7f80_0000),
    f32::from_bits(0xff80_0000),
    f32::from_bits(0x7fc0_0001),
    f32::from_bits(0xffc0_1234),
    f32::from_bits(0x0000_0001),
    f32::from_bits(0x8000_0001),
    f32::from_bits(0x7f7f_ffff),
    f32::from_bits(0xff7f_ffff),
];

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct C2v {
    x: c_float,
    y: c_float,
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

struct Libraries {
    c: Library,
    rust: Library,
}

impl Libraries {
    fn load() -> Self {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let c_path = root.join("c_src/build/libtranslated_rust.so");
        let rust_path = find_rust_library(root);
        assert!(
            c_path.is_file(),
            "missing C shared library: {}",
            c_path.display()
        );
        assert!(
            rust_path.is_file(),
            "missing Rust shared library: {}",
            rust_path.display()
        );

        // SAFETY: Both paths name shared libraries built from this repository.
        unsafe {
            Self {
                c: Library::new(c_path).expect("load C shared library"),
                rust: Library::new(rust_path).expect("load Rust shared library"),
            }
        }
    }

    unsafe fn symbols<T>(&self, name: &[u8]) -> (Symbol<'_, T>, Symbol<'_, T>) {
        // SAFETY: Every caller supplies the C ABI signature from lib.c.
        unsafe {
            (
                self.c.get(name).expect("resolve C symbol"),
                self.rust.get(name).expect("resolve Rust symbol"),
            )
        }
    }
}

fn find_rust_library(root: &Path) -> PathBuf {
    for direct in [
        root.join("target/debug/libcircle_collide_lib.so"),
        root.join("target/release/libcircle_collide_lib.so"),
    ] {
        if direct.is_file() {
            return direct;
        }
    }

    for profile in ["debug", "release"] {
        let deps = root.join("target").join(profile).join("deps");
        if let Ok(entries) = std::fs::read_dir(&deps)
            && let Some(path) = entries
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .find(|path| {
                    path.file_name()
                        .and_then(|name| name.to_str())
                        .is_some_and(|name| {
                            name.starts_with("libcircle_collide_lib") && name.ends_with(".so")
                        })
                })
        {
            return path;
        }
    }

    panic!(
        "Rust cdylib not found under {}",
        root.join("target").display()
    )
}

#[derive(Clone, Copy)]
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn bits(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x as u32
    }

    fn raw_f32(&mut self) -> f32 {
        f32::from_bits(self.bits())
    }

    fn finite(&mut self, low: f32, high: f32) -> f32 {
        let unit = (self.bits() as f64 / u32::MAX as f64) as f32;
        low + (high - low) * unit
    }
}

fn v(x: f32, y: f32) -> C2v {
    C2v { x, y }
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

fn assert_v_eq(c: C2v, rust: C2v, context: &str) {
    assert_f32_eq(c.x, rust.x, &format!("{context}.x"));
    assert_f32_eq(c.y, rust.y, &format!("{context}.y"));
}

#[test]
fn rows_1_and_2_vector_construction_and_scale() {
    type V = unsafe extern "C" fn(f32, f32) -> C2v;
    type Mul = unsafe extern "C" fn(C2v, f32) -> C2v;
    let libs = Libraries::load();
    // SAFETY: Signatures match the C definitions.
    let ((c_v, r_v), (c_mul, r_mul)) = unsafe {
        (
            libs.symbols::<V>(b"c2V\0"),
            libs.symbols::<Mul>(b"c2Mulvs\0"),
        )
    };
    let mut rng = Rng::new(0x0102_0304_0506_0708);

    for i in 0..ITERATIONS {
        let x = rng.raw_f32();
        let y = rng.raw_f32();
        let scale = rng.raw_f32();
        // SAFETY: These functions take values and dereference no pointers.
        unsafe {
            assert_v_eq(c_v(x, y), r_v(x, y), &format!("row 1 iteration {i}"));
            assert_v_eq(
                c_mul(v(x, y), scale),
                r_mul(v(x, y), scale),
                &format!("row 2 iteration {i}"),
            );
        }
    }

    for (i, &x) in SPECIAL_FLOATS.iter().enumerate() {
        for (j, &y) in SPECIAL_FLOATS.iter().enumerate() {
            let scale = SPECIAL_FLOATS[(i + j) % SPECIAL_FLOATS.len()];
            // SAFETY: These functions take values and dereference no pointers.
            unsafe {
                assert_v_eq(c_v(x, y), r_v(x, y), "row 1 explicit IEEE corpus");
                assert_v_eq(
                    c_mul(v(x, y), scale),
                    r_mul(v(x, y), scale),
                    "row 2 explicit IEEE corpus",
                );
            }
        }
    }
}

#[test]
fn rows_3_to_6_vector_min_max_comparisons() {
    type Binary = unsafe extern "C" fn(C2v, C2v) -> C2v;
    let libs = Libraries::load();
    // SAFETY: Signatures match the C definitions.
    let ((c_max, r_max), (c_min, r_min)) = unsafe {
        (
            libs.symbols::<Binary>(b"c2Maxv\0"),
            libs.symbols::<Binary>(b"c2Minv\0"),
        )
    };
    let mut rng = Rng::new(0x1314_1516_1718_191a);

    for mask in 0..4 {
        for i in 0..ITERATIONS / 4 {
            let base_x = rng.finite(-1_000.0, 1_000.0);
            let base_y = rng.finite(-1_000.0, 1_000.0);
            let dx = rng.finite(0.25, 100.0);
            let dy = rng.finite(0.25, 100.0);
            let a = v(
                base_x + if mask & 1 != 0 { dx } else { -dx },
                base_y + if mask & 2 != 0 { dy } else { -dy },
            );
            let b = v(base_x, base_y);
            // SAFETY: These functions take values and dereference no pointers.
            unsafe {
                assert_v_eq(
                    c_max(a, b),
                    r_max(a, b),
                    &format!("row 3 mask {mask} iteration {i}"),
                );
                assert_v_eq(
                    c_min(a, b),
                    r_min(a, b),
                    &format!("row 5 mask {mask} iteration {i}"),
                );
            }
        }
    }

    for i in 0..ITERATIONS {
        let payload = (rng.bits() & 0x003f_ffff).max(1);
        let nan = f32::from_bits(0x7fc0_0000 | payload);
        let ordinary = rng.finite(-1_000.0, 1_000.0);
        let cases = [
            (v(ordinary, ordinary), v(ordinary, ordinary)),
            (v(nan, ordinary), v(ordinary, nan)),
            (v(ordinary, nan), v(nan, ordinary)),
        ];
        for (case, (a, b)) in cases.into_iter().enumerate() {
            // SAFETY: These functions take values and dereference no pointers.
            unsafe {
                assert_v_eq(
                    c_max(a, b),
                    r_max(a, b),
                    &format!("row 4 case {case} iteration {i}"),
                );
                assert_v_eq(
                    c_min(a, b),
                    r_min(a, b),
                    &format!("row 6 case {case} iteration {i}"),
                );
            }
        }
    }
}

#[test]
fn rows_7_and_8_vector_clamp() {
    type Clamp = unsafe extern "C" fn(C2v, C2v, C2v) -> C2v;
    let libs = Libraries::load();
    // SAFETY: Signature matches the C definition.
    let (c_clamp, r_clamp) = unsafe { libs.symbols::<Clamp>(b"c2Clampv\0") };
    let mut rng = Rng::new(0x2122_2324_2526_2728);

    for x_region in 0..3 {
        for y_region in 0..3 {
            for i in 0..ITERATIONS / 9 {
                let lo = v(rng.finite(-100.0, -10.0), rng.finite(-100.0, -10.0));
                let hi = v(rng.finite(10.0, 100.0), rng.finite(10.0, 100.0));
                let coordinate = |region, low: f32, high: f32| match region {
                    0 => low - 1.0,
                    1 => (low + high) * 0.5,
                    _ => high + 1.0,
                };
                let input = v(
                    coordinate(x_region, lo.x, hi.x),
                    coordinate(y_region, lo.y, hi.y),
                );
                // SAFETY: This function takes values and dereferences no pointers.
                unsafe {
                    assert_v_eq(
                        c_clamp(input, lo, hi),
                        r_clamp(input, lo, hi),
                        &format!("row 7 x-region {x_region} y-region {y_region} iteration {i}"),
                    );
                }
            }
        }
    }

    for i in 0..ITERATIONS {
        let nan = f32::from_bits(0x7fc0_0000 | (rng.bits() & 0x003f_ffff).max(1));
        let cases = [
            (v(0.0, 0.0), v(1.0, 2.0), v(-1.0, -2.0)),
            (v(nan, 0.0), v(-1.0, nan), v(1.0, 2.0)),
            (v(0.0, nan), v(nan, -1.0), v(2.0, 1.0)),
        ];
        for (case, (input, lo, hi)) in cases.into_iter().enumerate() {
            // SAFETY: This function takes values and dereferences no pointers.
            unsafe {
                assert_v_eq(
                    c_clamp(input, lo, hi),
                    r_clamp(input, lo, hi),
                    &format!("row 8 case {case} iteration {i}"),
                );
            }
        }
    }
}

#[test]
fn rows_9_and_10_vector_arithmetic() {
    type Sub = unsafe extern "C" fn(C2v, C2v) -> C2v;
    type Dot = unsafe extern "C" fn(C2v, C2v) -> f32;
    let libs = Libraries::load();
    // SAFETY: Signatures match the C definitions.
    let ((c_sub, r_sub), (c_dot, r_dot)) = unsafe {
        (
            libs.symbols::<Sub>(b"c2Sub\0"),
            libs.symbols::<Dot>(b"c2Dot\0"),
        )
    };
    let mut rng = Rng::new(0x3132_3334_3536_3738);

    for i in 0..ITERATIONS {
        let a = v(rng.raw_f32(), rng.raw_f32());
        let b = v(rng.raw_f32(), rng.raw_f32());
        // SAFETY: These functions take values and dereference no pointers.
        unsafe {
            assert_v_eq(c_sub(a, b), r_sub(a, b), &format!("row 9 iteration {i}"));
            assert_f32_eq(c_dot(a, b), r_dot(a, b), &format!("row 10 iteration {i}"));
        }
    }

    let boundary_pairs = [
        (v(f32::MAX, f32::MAX), v(f32::MAX, -f32::MAX)),
        (
            v(f32::from_bits(1), f32::from_bits(2)),
            v(f32::from_bits(1), f32::from_bits(2)),
        ),
        (v(f32::INFINITY, -0.0), v(0.0, f32::NEG_INFINITY)),
    ];
    for (a, b) in boundary_pairs {
        // SAFETY: These functions take values and dereference no pointers.
        unsafe {
            assert_v_eq(c_sub(a, b), r_sub(a, b), "row 9 explicit boundary");
            assert_f32_eq(c_dot(a, b), r_dot(a, b), "row 10 overflow/underflow");
        }
    }
}

#[test]
fn rows_11_to_14_circle_to_circle() {
    type Collide = unsafe extern "C" fn(C2Circle, C2Circle) -> c_int;
    let libs = Libraries::load();
    // SAFETY: Signature matches the C definition.
    let (c_fn, r_fn) = unsafe { libs.symbols::<Collide>(b"c2CircletoCircle\0") };
    let mut rng = Rng::new(0x4142_4344_4546_4748);

    for i in 0..ITERATIONS {
        let ax = rng.finite(-1_000.0, 1_000.0);
        let ay = rng.finite(-1_000.0, 1_000.0);
        let ar = rng.finite(1.0, 100.0);
        let br = rng.finite(1.0, 100.0);
        let sum = ar + br;
        let a = C2Circle {
            p: v(ax, ay),
            r: ar,
        };
        let overlap = C2Circle {
            p: v(ax + sum * 0.25, ay),
            r: br,
        };
        // Integers keep the squared equality exact for the strict tangent path.
        let tangent_radius = (rng.bits() % 100 + 1) as f32;
        let tangent = C2Circle {
            p: v(tangent_radius * 2.0, 0.0),
            r: tangent_radius,
        };
        let tangent_a = C2Circle {
            p: v(0.0, 0.0),
            r: tangent_radius,
        };
        let separated = C2Circle {
            p: v(ax + sum * 2.0, ay),
            r: -br,
        };
        // SAFETY: This function takes values and dereferences no pointers.
        unsafe {
            assert_eq!(c_fn(a, overlap), r_fn(a, overlap), "row 11 iteration {i}");
            assert_eq!(
                c_fn(tangent_a, tangent),
                r_fn(tangent_a, tangent),
                "row 12 iteration {i}"
            );
            assert_eq!(
                c_fn(a, separated),
                r_fn(a, separated),
                "row 13 iteration {i}"
            );
        }

        let special_a = C2Circle {
            p: v(rng.raw_f32(), rng.raw_f32()),
            r: rng.raw_f32(),
        };
        let special_b = C2Circle {
            p: v(rng.raw_f32(), rng.raw_f32()),
            r: rng.raw_f32(),
        };
        // SAFETY: This function takes values and dereferences no pointers.
        unsafe {
            assert_eq!(
                c_fn(special_a, special_b),
                r_fn(special_a, special_b),
                "row 14 iteration {i}"
            );
        }
    }

    for (i, &special) in SPECIAL_FLOATS.iter().enumerate() {
        let a = C2Circle {
            p: v(special, SPECIAL_FLOATS[(i + 1) % SPECIAL_FLOATS.len()]),
            r: SPECIAL_FLOATS[(i + 2) % SPECIAL_FLOATS.len()],
        };
        let b = C2Circle {
            p: v(
                SPECIAL_FLOATS[(i + 3) % SPECIAL_FLOATS.len()],
                SPECIAL_FLOATS[(i + 4) % SPECIAL_FLOATS.len()],
            ),
            r: SPECIAL_FLOATS[(i + 5) % SPECIAL_FLOATS.len()],
        };
        // SAFETY: This function takes values and dereferences no pointers.
        unsafe {
            assert_eq!(c_fn(a, b), r_fn(a, b), "row 14 explicit IEEE case {i}");
        }
    }
}

#[test]
fn rows_15_to_24_circle_to_aabb() {
    type Collide = unsafe extern "C" fn(C2Circle, C2Aabb) -> c_int;
    let libs = Libraries::load();
    // SAFETY: Signature matches the C definition.
    let (c_fn, r_fn) = unsafe { libs.symbols::<Collide>(b"c2CircletoAABB\0") };
    let mut rng = Rng::new(0x5152_5354_5556_5758);

    for x_region in 0..3 {
        for y_region in 0..3 {
            let row = 15 + x_region * 3 + y_region;
            for i in 0..ITERATIONS / 9 {
                let min = v(rng.finite(-100.0, -20.0), rng.finite(-100.0, -20.0));
                let max = v(rng.finite(20.0, 100.0), rng.finite(20.0, 100.0));
                let coordinate = |region, low: f32, high: f32| match region {
                    0 => low - 10.0,
                    1 => (low + high) * 0.5,
                    _ => high + 10.0,
                };
                let circle = C2Circle {
                    p: v(
                        coordinate(x_region, min.x, max.x),
                        coordinate(y_region, min.y, max.y),
                    ),
                    r: rng.finite(0.0, 30.0),
                };
                let aabb = C2Aabb { min, max };
                // SAFETY: This function takes values and dereferences no pointers.
                unsafe {
                    assert_eq!(
                        c_fn(circle, aabb),
                        r_fn(circle, aabb),
                        "row {row} iteration {i}"
                    );
                }
            }
        }
    }

    for i in 0..ITERATIONS {
        let cases = [
            (
                C2Circle {
                    p: v(rng.raw_f32(), rng.raw_f32()),
                    r: rng.raw_f32(),
                },
                C2Aabb {
                    min: v(rng.raw_f32(), rng.raw_f32()),
                    max: v(rng.raw_f32(), rng.raw_f32()),
                },
            ),
            (
                C2Circle {
                    p: v(0.0, 0.0),
                    r: -rng.finite(0.0, 100.0),
                },
                C2Aabb {
                    min: v(1.0, 1.0),
                    max: v(-1.0, -1.0),
                },
            ),
            (
                C2Circle {
                    p: v(4.0, 6.0),
                    r: 2.0,
                },
                C2Aabb {
                    min: v(5.0, 5.0),
                    max: v(5.0, 5.0),
                },
            ),
        ];
        for (case, (circle, aabb)) in cases.into_iter().enumerate() {
            // SAFETY: This function takes values and dereferences no pointers.
            unsafe {
                assert_eq!(
                    c_fn(circle, aabb),
                    r_fn(circle, aabb),
                    "row 24 case {case} iteration {i}"
                );
            }
        }
    }
}

#[test]
fn rows_25_to_28_circle_to_capsule() {
    type Collide = unsafe extern "C" fn(C2Circle, C2Capsule) -> c_int;
    let libs = Libraries::load();
    // SAFETY: Signature matches the C definition.
    let (c_fn, r_fn) = unsafe { libs.symbols::<Collide>(b"c2CircletoCapsule\0") };
    let mut rng = Rng::new(0x6162_6364_6566_6768);

    for region in 0..3 {
        let row = 25 + region;
        for i in 0..ITERATIONS {
            let a = v(rng.finite(-100.0, 100.0), rng.finite(-100.0, 100.0));
            let n = v(rng.finite(2.0, 20.0), rng.finite(2.0, 20.0));
            let b = v(a.x + n.x, a.y + n.y);
            let t = match region {
                0 => rng.finite(-2.0, -0.1),
                1 => rng.finite(0.1, 0.9),
                _ => rng.finite(1.1, 3.0),
            };
            let offset = rng.finite(-20.0, 20.0);
            let center = v(a.x + t * n.x - offset * n.y, a.y + t * n.y + offset * n.x);
            let circle = C2Circle {
                p: center,
                r: rng.finite(-10.0, 30.0),
            };
            let capsule = C2Capsule {
                a,
                b,
                r: rng.finite(-10.0, 30.0),
            };
            // SAFETY: This function takes values and dereferences no pointers.
            unsafe {
                assert_eq!(
                    c_fn(circle, capsule),
                    r_fn(circle, capsule),
                    "row {row} iteration {i}"
                );
            }
        }
    }

    for i in 0..ITERATIONS {
        let point = v(rng.raw_f32(), rng.raw_f32());
        let circle = C2Circle {
            p: v(rng.raw_f32(), rng.raw_f32()),
            r: rng.raw_f32(),
        };
        let degenerate = C2Capsule {
            a: point,
            b: point,
            r: rng.raw_f32(),
        };
        // SAFETY: This function takes values and dereferences no pointers.
        unsafe {
            assert_eq!(
                c_fn(circle, degenerate),
                r_fn(circle, degenerate),
                "row 28 iteration {i}"
            );
        }
    }

    let boundary_cases = [
        (
            C2Circle {
                p: v(5.0, 3.0),
                r: 1.0,
            },
            C2Capsule {
                a: v(0.0, 0.0),
                b: v(10.0, 0.0),
                r: 2.0,
            },
        ),
        (
            C2Circle {
                p: v(1.0, 1.0),
                r: -3.0,
            },
            C2Capsule {
                a: v(0.0, 0.0),
                b: v(0.0, 0.0),
                r: -4.0,
            },
        ),
    ];
    for (i, (circle, capsule)) in boundary_cases.into_iter().enumerate() {
        // SAFETY: This function takes values and dereferences no pointers.
        unsafe {
            assert_eq!(
                c_fn(circle, capsule),
                r_fn(circle, capsule),
                "row 28 explicit boundary case {i}"
            );
        }
    }
}

#[test]
fn rows_29_to_31_shape_dispatch() {
    type Dispatch = unsafe extern "C" fn(*const c_void, *const c_void, c_int) -> c_int;
    let libs = Libraries::load();
    // SAFETY: Signature uses c_int rather than a Rust enum, preserving all FFI inputs.
    let (c_fn, r_fn) = unsafe { libs.symbols::<Dispatch>(b"c2Collided\0") };
    let mut rng = Rng::new(0x7172_7374_7576_7778);

    for i in 0..ITERATIONS {
        let circle = C2Circle {
            p: v(rng.finite(-100.0, 100.0), rng.finite(-100.0, 100.0)),
            r: rng.finite(-20.0, 50.0),
        };
        let other_circle = C2Circle {
            p: v(rng.finite(-100.0, 100.0), rng.finite(-100.0, 100.0)),
            r: rng.finite(-20.0, 50.0),
        };
        let aabb = C2Aabb {
            min: v(rng.finite(-100.0, 0.0), rng.finite(-100.0, 0.0)),
            max: v(rng.finite(0.0, 100.0), rng.finite(0.0, 100.0)),
        };
        let capsule = C2Capsule {
            a: v(rng.finite(-100.0, 100.0), rng.finite(-100.0, 100.0)),
            b: v(rng.finite(-100.0, 100.0), rng.finite(-100.0, 100.0)),
            r: rng.finite(-20.0, 50.0),
        };
        let a_ptr = (&circle as *const C2Circle).cast();
        let inputs = [
            ((&other_circle as *const C2Circle).cast(), 0),
            ((&aabb as *const C2Aabb).cast(), 1),
            ((&capsule as *const C2Capsule).cast(), 2),
        ];
        for (shape, (b_ptr, kind)) in inputs.into_iter().enumerate() {
            // SAFETY: Pointers address values of the type selected by kind.
            unsafe {
                assert_eq!(
                    c_fn(a_ptr, b_ptr, kind),
                    r_fn(a_ptr, b_ptr, kind),
                    "row {} shape {shape} iteration {i}",
                    29 + shape
                );
            }
        }
    }
}

#[test]
fn row_32_complete_circle_collide_pipeline() {
    type CircleCollide = unsafe extern "C" fn(f32, f32, f32) -> c_int;
    let libs = Libraries::load();
    // SAFETY: Signature matches the public C declaration.
    let (c_fn, r_fn) = unsafe { libs.symbols::<CircleCollide>(b"circle_collide\0") };
    let mut rng = Rng::new(0x8182_8384_8586_8788);

    for i in 0..ITERATIONS * 4 {
        let (x, y, radius) = if i % 2 == 0 {
            (
                rng.finite(-200.0, 200.0),
                rng.finite(-200.0, 200.0),
                rng.finite(-50.0, 100.0),
            )
        } else {
            (rng.raw_f32(), rng.raw_f32(), rng.raw_f32())
        };
        // SAFETY: This function takes values and dereferences no caller pointers.
        unsafe {
            assert_eq!(
                c_fn(x, y, radius),
                r_fn(x, y, radius),
                "row 32 iteration {i}: ({x:?}, {y:?}, {radius:?})"
            );
        }
    }

    for (i, &x) in SPECIAL_FLOATS.iter().enumerate() {
        let y = SPECIAL_FLOATS[(i + 1) % SPECIAL_FLOATS.len()];
        let radius = SPECIAL_FLOATS[(i + 2) % SPECIAL_FLOATS.len()];
        // SAFETY: This function takes values and dereferences no caller pointers.
        unsafe {
            assert_eq!(
                c_fn(x, y, radius),
                r_fn(x, y, radius),
                "row 32 explicit IEEE case {i}"
            );
        }
    }
}

#[test]
fn error_row_1_invalid_shape_values_return_zero_without_dereference() {
    type Dispatch = unsafe extern "C" fn(*const c_void, *const c_void, c_int) -> c_int;
    let libs = Libraries::load();
    // SAFETY: Signature uses c_int rather than a Rust enum, preserving invalid inputs.
    let (c_fn, r_fn) = unsafe { libs.symbols::<Dispatch>(b"c2Collided\0") };
    let sentinel = C2Circle {
        p: v(1.0, 2.0),
        r: 3.0,
    };
    let pointer = (&sentinel as *const C2Circle).cast();
    let null = std::ptr::null();
    let invalid = [-1, 3, 4, 255, c_int::MIN, c_int::MAX];

    for &kind in &invalid {
        for &(a, b) in &[
            (null, null),
            (pointer, null),
            (null, pointer),
            (pointer, pointer),
        ] {
            // SAFETY: The C default arm and Rust wildcard arm do not dereference pointers.
            unsafe {
                let c_result = c_fn(a, b, kind);
                let rust_result = r_fn(a, b, kind);
                assert_eq!(c_result, 0, "C invalid kind {kind}");
                assert_eq!(rust_result, c_result, "Rust invalid kind {kind}");
            }
        }
    }
}
