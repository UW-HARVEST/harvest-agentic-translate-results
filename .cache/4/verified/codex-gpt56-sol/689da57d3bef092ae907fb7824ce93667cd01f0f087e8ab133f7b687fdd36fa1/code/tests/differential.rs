use libloading::Library;
use std::path::PathBuf;
use std::ptr;
use std::sync::Mutex;

const CASES: usize = 64;
static TEST_LOCK: Mutex<()> = Mutex::new(());

unsafe extern "C" {
    fn fork() -> i32;
    fn waitpid(pid: i32, status: *mut i32, options: i32) -> i32;
    fn _exit(status: i32) -> !;
}

fn serial_guard() -> std::sync::MutexGuard<'static, ()> {
    TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

unsafe fn child_status<F: FnOnce()>(call: F) -> i32 {
    let pid = fork();
    assert!(pid >= 0, "fork failed");
    if pid == 0 {
        call();
        _exit(0);
    }
    let mut status = 0;
    assert_eq!(waitpid(pid, &mut status, 0), pid);
    status
}

struct Libraries {
    c: Library,
    rust: Library,
}

impl Libraries {
    unsafe fn open() -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c = Library::new(root.join("c_src/build/libqmath_c.so")).unwrap();
        let exe = std::env::current_exe().unwrap();
        let rust_path = exe
            .parent()
            .and_then(|p| p.parent())
            .unwrap()
            .join("libqmath.so");
        let rust = Library::new(rust_path).unwrap();
        Self { c, rust }
    }

    unsafe fn pair<T: Copy>(&self, name: &[u8]) -> (T, T) {
        (
            *self.c.get::<T>(name).unwrap(),
            *self.rust.get::<T>(name).unwrap(),
        )
    }

    unsafe fn data_pair<T>(&self, name: &[u8]) -> (*const T, *const T) {
        (
            *self.c.get::<*const T>(name).unwrap(),
            *self.rust.get::<*const T>(name).unwrap(),
        )
    }
}

#[derive(Clone)]
struct Rng(u32);

impl Rng {
    fn new(seed: u32) -> Self {
        Self(seed)
    }

    fn u32(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.0
    }

    fn i32(&mut self) -> i32 {
        self.u32() as i32
    }

    fn finite(&mut self) -> f32 {
        ((self.u32() % 20_001) as i32 - 10_000) as f32 / 37.0
    }

    fn positive(&mut self) -> f32 {
        (self.u32() % 10_000 + 1) as f32 / 31.0
    }

    fn channel(&mut self) -> f32 {
        (self.u32() % 256) as f32 / 255.0
    }
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

fn assert_floats(c: &[f32], rust: &[f32], context: &str) {
    assert_eq!(c.len(), rust.len());
    for (index, (&c, &rust)) in c.iter().zip(rust).enumerate() {
        assert_float(c, rust, &format!("{context}[{index}]"));
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct CPlane {
    normal: [f32; 3],
    dist: f32,
    plane_type: u8,
    signbits: u8,
    pad: [u8; 2],
}

#[test]
fn globals_match_byte_for_byte_config_row_1() {
    let _guard = serial_guard();
    unsafe {
        let libs = Libraries::open();
        macro_rules! compare_data {
            ($name:literal, $ty:ty) => {{
                let (c, rust) = libs.data_pair::<$ty>(concat!($name, "\0").as_bytes());
                let size = std::mem::size_of::<$ty>();
                let c = std::slice::from_raw_parts(c.cast::<u8>(), size);
                let rust = std::slice::from_raw_parts(rust.cast::<u8>(), size);
                assert_eq!(c, rust, $name);
            }};
        }

        compare_data!("vec3_origin", [f32; 3]);
        compare_data!("axisDefault", [[f32; 3]; 3]);
        compare_data!("colorBlack", [f32; 4]);
        compare_data!("colorRed", [f32; 4]);
        compare_data!("colorGreen", [f32; 4]);
        compare_data!("colorBlue", [f32; 4]);
        compare_data!("colorYellow", [f32; 4]);
        compare_data!("colorMagenta", [f32; 4]);
        compare_data!("colorCyan", [f32; 4]);
        compare_data!("colorWhite", [f32; 4]);
        compare_data!("colorLtGrey", [f32; 4]);
        compare_data!("colorMdGrey", [f32; 4]);
        compare_data!("colorDkGrey", [f32; 4]);
        compare_data!("g_color_table", [[f32; 4]; 8]);
        compare_data!("bytedirs", [[f32; 3]; 162]);
    }
}

#[test]
fn random_clamp_direction_color_and_normalize_config_rows_2_to_16() {
    let _guard = serial_guard();
    unsafe {
        let libs = Libraries::open();
        let mut rng = Rng::new(0x5eed_1234);

        type SeedFn = unsafe extern "C" fn(*mut i32) -> i32;
        type SeedFloatFn = unsafe extern "C" fn(*mut i32) -> f32;
        let (c_rand, r_rand) = libs.pair::<SeedFn>(b"Q_rand\0");
        let (c_random, r_random) = libs.pair::<SeedFloatFn>(b"Q_random\0");
        let (c_crandom, r_crandom) = libs.pair::<SeedFloatFn>(b"Q_crandom\0");
        for _ in 0..CASES {
            let seed = rng.i32();
            let (mut cs, mut rs) = (seed, seed);
            assert_eq!(c_rand(&mut cs), r_rand(&mut rs));
            assert_eq!(cs, rs);
            let (mut cs, mut rs) = (seed, seed);
            assert_float(c_random(&mut cs), r_random(&mut rs), "Q_random");
            assert_eq!(cs, rs);
            let (mut cs, mut rs) = (seed, seed);
            assert_float(c_crandom(&mut cs), r_crandom(&mut rs), "Q_crandom");
            assert_eq!(cs, rs);
        }

        type ClampCharFn = unsafe extern "C" fn(i32) -> i8;
        type ClampShortFn = unsafe extern "C" fn(i32) -> i16;
        let (c_char, r_char) = libs.pair::<ClampCharFn>(b"ClampChar\0");
        let (c_short, r_short) = libs.pair::<ClampShortFn>(b"ClampShort\0");
        for _ in 0..CASES {
            let char_value = (rng.u32() % 256) as i32 - 128;
            assert_eq!(c_char(char_value), r_char(char_value));
            let short_value = (rng.u32() % 65_536) as i32 - 32_768;
            assert_eq!(c_short(short_value), r_short(short_value));
        }

        type DirToByteFn = unsafe extern "C" fn(*const f32) -> i32;
        type ByteToDirFn = unsafe extern "C" fn(i32, *mut f32);
        let (c_dir_to, r_dir_to) = libs.pair::<DirToByteFn>(b"DirToByte\0");
        let (c_byte_to, r_byte_to) = libs.pair::<ByteToDirFn>(b"ByteToDir\0");
        for _ in 0..CASES {
            let negative = [-rng.positive(), -rng.positive(), -rng.positive()];
            assert_eq!(c_dir_to(negative.as_ptr()), r_dir_to(negative.as_ptr()));
        }
        for index in 0..162 {
            let mut c = [0.0; 3];
            let mut rust = [0.0; 3];
            c_byte_to(index, c.as_mut_ptr());
            r_byte_to(index, rust.as_mut_ptr());
            assert_floats(&c, &rust, "ByteToDir");
            assert_eq!(c_dir_to(c.as_ptr()), r_dir_to(rust.as_ptr()));
        }

        type Color3Fn = unsafe extern "C" fn(f32, f32, f32) -> u32;
        type Color4Fn = unsafe extern "C" fn(f32, f32, f32, f32) -> u32;
        let (c_color3, r_color3) = libs.pair::<Color3Fn>(b"ColorBytes3\0");
        let (c_color4, r_color4) = libs.pair::<Color4Fn>(b"ColorBytes4\0");
        for _ in 0..CASES {
            let values = [rng.channel(), rng.channel(), rng.channel(), rng.channel()];
            assert_eq!(
                c_color3(values[0], values[1], values[2]),
                r_color3(values[0], values[1], values[2]),
                "ColorBytes3"
            );
            assert_eq!(
                c_color4(values[0], values[1], values[2], values[3]),
                r_color4(values[0], values[1], values[2], values[3]),
                "ColorBytes4"
            );
        }

        type NormalizeColorFn = unsafe extern "C" fn(*const f32, *mut f32) -> f32;
        let (c_normalize, r_normalize) = libs.pair::<NormalizeColorFn>(b"NormalizeColor\0");
        let shapes = [
            [9.0, 3.0, 1.0],
            [1.0, 9.0, 3.0],
            [1.0, 3.0, 9.0],
            [0.0, 0.0, 0.0],
        ];
        for shape in shapes {
            for _ in 0..CASES {
                let scale = if shape == [0.0; 3] {
                    1.0
                } else {
                    rng.positive()
                };
                let input = [shape[0] * scale, shape[1] * scale, shape[2] * scale];
                let (mut c, mut rust) = ([f32::NAN; 3], [f32::NAN; 3]);
                assert_float(
                    c_normalize(input.as_ptr(), c.as_mut_ptr()),
                    r_normalize(input.as_ptr(), rust.as_mut_ptr()),
                    "NormalizeColor return",
                );
                assert_floats(&c, &rust, "NormalizeColor output");
            }
        }

        type PlaneFn = unsafe extern "C" fn(*mut f32, *const f32, *const f32, *const f32) -> i32;
        let (c_plane, r_plane) = libs.pair::<PlaneFn>(b"PlaneFromPoints\0");
        for _ in 0..CASES {
            let a = [rng.finite(), rng.finite(), rng.finite()];
            let b = [a[0] + 1.0, a[1], a[2]];
            let c = [a[0], a[1] + 1.0, a[2] + rng.finite() / 100.0];
            let (mut cp, mut rp) = ([0.0; 4], [0.0; 4]);
            assert_eq!(
                c_plane(cp.as_mut_ptr(), a.as_ptr(), b.as_ptr(), c.as_ptr()),
                r_plane(rp.as_mut_ptr(), a.as_ptr(), b.as_ptr(), c.as_ptr())
            );
            assert_floats(&cp, &rp, "PlaneFromPoints");
        }
    }
}

#[test]
fn rotations_and_basic_angles_config_rows_17_to_39() {
    let _guard = serial_guard();
    unsafe {
        let libs = Libraries::open();
        let mut rng = Rng::new(0xa11c_e517);

        type RotatePointFn = unsafe extern "C" fn(*mut f32, *const f32, *const f32, f32);
        type RotateDirectionFn = unsafe extern "C" fn(*mut [f32; 3], f32);
        let (c_rotate, r_rotate) = libs.pair::<RotatePointFn>(b"RotatePointAroundVector\0");
        let (c_around, r_around) = libs.pair::<RotateDirectionFn>(b"RotateAroundDirection\0");
        let directions = [
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [0.525731, 0.0, 0.850651],
        ];
        for &direction in &directions {
            for _ in 0..CASES {
                let point = [rng.finite(), rng.finite(), rng.finite()];
                let degrees = rng.finite();
                let (mut c, mut rust) = ([0.0; 3], [0.0; 3]);
                c_rotate(c.as_mut_ptr(), direction.as_ptr(), point.as_ptr(), degrees);
                r_rotate(
                    rust.as_mut_ptr(),
                    direction.as_ptr(),
                    point.as_ptr(),
                    degrees,
                );
                assert_floats(&c, &rust, "RotatePointAroundVector");
            }
        }
        for zero_yaw in [true, false] {
            for _ in 0..CASES {
                let direction = directions[(rng.u32() as usize) % directions.len()];
                let yaw = if zero_yaw {
                    0.0
                } else {
                    let value = rng.finite();
                    if value == 0.0 {
                        1.0
                    } else {
                        value
                    }
                };
                let mut c = [direction, [f32::NAN; 3], [f32::NAN; 3]];
                let mut rust = c;
                c_around(c.as_mut_ptr(), yaw);
                r_around(rust.as_mut_ptr(), yaw);
                assert_floats(
                    c.as_flattened(),
                    rust.as_flattened(),
                    "RotateAroundDirection",
                );
            }
        }

        type VecToAnglesFn = unsafe extern "C" fn(*const f32, *mut f32);
        let (c_vecto, r_vecto) = libs.pair::<VecToAnglesFn>(b"vectoangles\0");
        let branch_shapes = [
            [0.0, 0.0, 2.0],
            [0.0, 0.0, -2.0],
            [2.0, 1.0, 1.0],
            [2.0, -1.0, 1.0],
            [2.0, 1.0, -1.0],
            [0.0, 2.0, 1.0],
            [0.0, -2.0, -1.0],
        ];
        for shape in branch_shapes {
            for _ in 0..CASES {
                let scale = rng.positive();
                let input = [shape[0] * scale, shape[1] * scale, shape[2] * scale];
                let (mut c, mut rust) = ([0.0; 3], [0.0; 3]);
                c_vecto(input.as_ptr(), c.as_mut_ptr());
                r_vecto(input.as_ptr(), rust.as_mut_ptr());
                assert_floats(&c, &rust, "vectoangles");
            }
        }

        type AnglesToAxisFn = unsafe extern "C" fn(*const f32, *mut [f32; 3]);
        type AxisClearFn = unsafe extern "C" fn(*mut [f32; 3]);
        type AxisCopyFn = unsafe extern "C" fn(*const [f32; 3], *mut [f32; 3]);
        let (c_to_axis, r_to_axis) = libs.pair::<AnglesToAxisFn>(b"AnglesToAxis\0");
        let (c_clear, r_clear) = libs.pair::<AxisClearFn>(b"AxisClear\0");
        let (c_copy, r_copy) = libs.pair::<AxisCopyFn>(b"AxisCopy\0");
        for _ in 0..CASES {
            let angles = [rng.finite(), rng.finite(), rng.finite()];
            let (mut c, mut rust) = ([[0.0; 3]; 3], [[0.0; 3]; 3]);
            c_to_axis(angles.as_ptr(), c.as_mut_ptr());
            r_to_axis(angles.as_ptr(), rust.as_mut_ptr());
            assert_floats(c.as_flattened(), rust.as_flattened(), "AnglesToAxis");

            let mut cc = c;
            let mut rc = rust;
            c_clear(cc.as_mut_ptr());
            r_clear(rc.as_mut_ptr());
            assert_floats(cc.as_flattened(), rc.as_flattened(), "AxisClear");

            let (mut copied_c, mut copied_r) = ([[0.0; 3]; 3], [[0.0; 3]; 3]);
            c_copy(c.as_ptr(), copied_c.as_mut_ptr());
            r_copy(rust.as_ptr(), copied_r.as_mut_ptr());
            assert_floats(copied_c.as_flattened(), copied_r.as_flattened(), "AxisCopy");
        }

        type ProjectFn = unsafe extern "C" fn(*mut f32, *const f32, *const f32);
        type MakeNormalFn = unsafe extern "C" fn(*const f32, *mut f32, *mut f32);
        type VectorRotateFn = unsafe extern "C" fn(*const f32, *const [f32; 3], *mut f32);
        let (c_project, r_project) = libs.pair::<ProjectFn>(b"ProjectPointOnPlane\0");
        let (c_make, r_make) = libs.pair::<MakeNormalFn>(b"MakeNormalVectors\0");
        let (c_vector_rotate, r_vector_rotate) = libs.pair::<VectorRotateFn>(b"VectorRotate\0");
        for &normal in &directions {
            for _ in 0..CASES {
                let point = [rng.finite(), rng.finite(), rng.finite()];
                let (mut c, mut rust) = ([0.0; 3], [0.0; 3]);
                c_project(c.as_mut_ptr(), point.as_ptr(), normal.as_ptr());
                r_project(rust.as_mut_ptr(), point.as_ptr(), normal.as_ptr());
                assert_floats(&c, &rust, "ProjectPointOnPlane");

                let (mut cr, mut cu, mut rr, mut ru) = ([0.0; 3], [0.0; 3], [0.0; 3], [0.0; 3]);
                c_make(normal.as_ptr(), cr.as_mut_ptr(), cu.as_mut_ptr());
                r_make(normal.as_ptr(), rr.as_mut_ptr(), ru.as_mut_ptr());
                assert_floats(&cr, &rr, "MakeNormalVectors right");
                assert_floats(&cu, &ru, "MakeNormalVectors up");

                let matrix = [normal, cr, cu];
                c_vector_rotate(point.as_ptr(), matrix.as_ptr(), c.as_mut_ptr());
                let matrix = [normal, rr, ru];
                r_vector_rotate(point.as_ptr(), matrix.as_ptr(), rust.as_mut_ptr());
                assert_floats(&c, &rust, "VectorRotate");
            }
        }

        type UnaryFloatFn = unsafe extern "C" fn(f32) -> f32;
        let (c_rsqrt, r_rsqrt) = libs.pair::<UnaryFloatFn>(b"Q_rsqrt\0");
        let (c_fabs, r_fabs) = libs.pair::<UnaryFloatFn>(b"Q_fabs\0");
        for _ in 0..CASES {
            let value = rng.positive();
            assert_float(c_rsqrt(value), r_rsqrt(value), "Q_rsqrt positive");
            let signed = if rng.u32() & 1 == 0 { value } else { -value };
            assert_float(c_fabs(signed), r_fabs(signed), "Q_fabs");
        }
        for bits in [
            0_u32,
            0x8000_0000,
            (-1.0_f32).to_bits(),
            f32::INFINITY.to_bits(),
            f32::NEG_INFINITY.to_bits(),
            0x7fc0_1234,
            0xffc0_5678,
        ] {
            let value = f32::from_bits(bits);
            assert_float(c_rsqrt(value), r_rsqrt(value), "Q_rsqrt special");
            assert_float(c_fabs(value), r_fabs(value), "Q_fabs special");
        }
        for _ in 0..CASES {
            let negative = -rng.positive();
            assert_float(
                c_rsqrt(negative),
                r_rsqrt(negative),
                "Q_rsqrt randomized negative",
            );
            let nan = f32::from_bits(0x7f80_0001 | (rng.u32() & 0x007f_ffff));
            assert_float(c_rsqrt(nan), r_rsqrt(nan), "Q_rsqrt randomized NaN");
        }

        type LerpFn = unsafe extern "C" fn(f32, f32, f32) -> f32;
        let (c_lerp, r_lerp) = libs.pair::<LerpFn>(b"LerpAngle\0");
        for delta in [240.0_f32, -240.0, 120.0] {
            for _ in 0..CASES {
                let from = rng.finite();
                let to = from + delta;
                let frac = rng.channel();
                assert_float(c_lerp(from, to, frac), r_lerp(from, to, frac), "LerpAngle");
            }
        }
    }
}

#[test]
fn angle_normalization_and_plane_signs_config_rows_40_to_56() {
    let _guard = serial_guard();
    unsafe {
        let libs = Libraries::open();
        let mut rng = Rng::new(0x0ddc_0ffe);

        type BinaryFloatFn = unsafe extern "C" fn(f32, f32) -> f32;
        type UnaryFloatFn = unsafe extern "C" fn(f32) -> f32;
        type AnglesSubtractFn = unsafe extern "C" fn(*const f32, *const f32, *mut f32);
        let (c_subtract, r_subtract) = libs.pair::<BinaryFloatFn>(b"AngleSubtract\0");
        let (c_angles_subtract, r_angles_subtract) =
            libs.pair::<AnglesSubtractFn>(b"AnglesSubtract\0");
        let (c_mod, r_mod) = libs.pair::<UnaryFloatFn>(b"AngleMod\0");
        let (c_360, r_360) = libs.pair::<UnaryFloatFn>(b"AngleNormalize360\0");
        let (c_180, r_180) = libs.pair::<UnaryFloatFn>(b"AngleNormalize180\0");
        let (c_delta, r_delta) = libs.pair::<BinaryFloatFn>(b"AngleDelta\0");

        for delta in [1080.0_f32, -1080.0, 90.0] {
            for _ in 0..CASES {
                let a2 = rng.finite();
                let a1 = a2 + delta;
                assert_float(c_subtract(a1, a2), r_subtract(a1, a2), "AngleSubtract");
            }
        }
        for _ in 0..CASES {
            let a = [
                rng.finite() * 10.0,
                rng.finite() * 10.0,
                rng.finite() * 10.0,
            ];
            let b = [
                rng.finite() * 10.0,
                rng.finite() * 10.0,
                rng.finite() * 10.0,
            ];
            let (mut c, mut rust) = ([0.0; 3], [0.0; 3]);
            c_angles_subtract(a.as_ptr(), b.as_ptr(), c.as_mut_ptr());
            r_angles_subtract(a.as_ptr(), b.as_ptr(), rust.as_mut_ptr());
            assert_floats(&c, &rust, "AnglesSubtract");

            let angle = rng.finite() * 100.0;
            assert_float(c_mod(angle), r_mod(angle), "AngleMod");
            assert_float(c_360(angle), r_360(angle), "AngleNormalize360");
            assert_float(c_delta(a[0], b[0]), r_delta(a[0], b[0]), "AngleDelta");
        }
        for base in [90.0_f32, 270.0] {
            for _ in 0..CASES {
                let angle = base + (rng.channel() - 0.5) * 20.0;
                assert_float(c_180(angle), r_180(angle), "AngleNormalize180");
            }
        }

        type SetSignbitsFn = unsafe extern "C" fn(*mut CPlane);
        let (c_signs, r_signs) = libs.pair::<SetSignbitsFn>(b"SetPlaneSignbits\0");
        for signs in 0_u8..8 {
            for _ in 0..CASES {
                let mut normal = [rng.positive(), rng.positive(), rng.positive()];
                for (axis, value) in normal.iter_mut().enumerate() {
                    if signs & (1 << axis) != 0 {
                        *value = -*value;
                    }
                }
                let base = CPlane {
                    normal,
                    dist: rng.finite(),
                    plane_type: 3,
                    signbits: 0xff,
                    pad: [0x55, 0xaa],
                };
                let (mut c, mut rust) = (base, base);
                c_signs(&mut c);
                r_signs(&mut rust);
                assert_eq!(c.signbits, rust.signbits, "sign pattern {signs}");
                assert_eq!(c.pad, rust.pad);
            }
        }
    }
}

#[test]
fn plane_sides_bounds_and_vectors_config_rows_57_to_133() {
    let _guard = serial_guard();
    unsafe {
        let libs = Libraries::open();
        let mut rng = Rng::new(0xb0a0_d5aa);

        type BoxSideFn = unsafe extern "C" fn(*const f32, *const f32, *const CPlane) -> i32;
        let (c_box, r_box) = libs.pair::<BoxSideFn>(b"BoxOnPlaneSide\0");
        for plane_type in 0_u8..3 {
            for region in 0..3 {
                for _ in 0..CASES {
                    let extent = rng.positive() + 1.0;
                    let mins = [-extent, -extent * 1.5, -extent * 2.0];
                    let maxs = [extent, extent * 1.5, extent * 2.0];
                    let axis = plane_type as usize;
                    let dist = match region {
                        0 => mins[axis],
                        1 => maxs[axis],
                        _ => (mins[axis] + maxs[axis]) * 0.5,
                    };
                    let plane = CPlane {
                        normal: [0.0; 3],
                        dist,
                        plane_type,
                        signbits: 0,
                        pad: [0; 2],
                    };
                    assert_eq!(
                        c_box(mins.as_ptr(), maxs.as_ptr(), &plane),
                        r_box(mins.as_ptr(), maxs.as_ptr(), &plane),
                        "axial type={plane_type} region={region}"
                    );
                }
            }
        }

        for signbits in 0_u8..8 {
            for region in 0..3 {
                for _ in 0..CASES {
                    let extent = rng.positive() + 1.0;
                    let mins = [-extent, -extent * 1.25, -extent * 1.5];
                    let maxs = [extent, extent * 1.25, extent * 1.5];
                    let mut normal = [rng.positive(), rng.positive(), rng.positive()];
                    for (axis, value) in normal.iter_mut().enumerate() {
                        if signbits & (1 << axis) != 0 {
                            *value = -*value;
                        }
                    }
                    let span = normal[0].abs() * extent
                        + normal[1].abs() * extent * 1.25
                        + normal[2].abs() * extent * 1.5;
                    let dist = match region {
                        0 => -span - 1.0,
                        1 => span + 1.0,
                        _ => 0.0,
                    };
                    let plane = CPlane {
                        normal,
                        dist,
                        plane_type: 3,
                        signbits,
                        pad: [0; 2],
                    };
                    assert_eq!(
                        c_box(mins.as_ptr(), maxs.as_ptr(), &plane),
                        r_box(mins.as_ptr(), maxs.as_ptr(), &plane),
                        "nonaxial signbits={signbits} region={region}"
                    );
                }
            }
        }
        for _ in 0..CASES {
            let dist = match rng.u32() % 3 {
                0 => -rng.positive(),
                1 => 0.0,
                _ => rng.positive(),
            };
            let signbits = 8_u8.wrapping_add((rng.u32() % 248) as u8);
            let mins = [-2.0, -3.0, -4.0];
            let maxs = [2.0, 3.0, 4.0];
            let plane = CPlane {
                normal: [0.2, -0.3, 0.4],
                dist,
                plane_type: 3,
                signbits,
                pad: [0; 2],
            };
            assert_eq!(
                c_box(mins.as_ptr(), maxs.as_ptr(), &plane),
                r_box(mins.as_ptr(), maxs.as_ptr(), &plane),
                "default signbits"
            );
        }

        type RadiusFn = unsafe extern "C" fn(*const f32, *const f32) -> f32;
        type ClearFn = unsafe extern "C" fn(*mut f32, *mut f32);
        type AddFn = unsafe extern "C" fn(*const f32, *mut f32, *mut f32);
        let (c_radius, r_radius) = libs.pair::<RadiusFn>(b"RadiusFromBounds\0");
        let (c_clear, r_clear) = libs.pair::<ClearFn>(b"ClearBounds\0");
        let (c_add, r_add) = libs.pair::<AddFn>(b"AddPointToBounds\0");
        for _ in 0..CASES {
            let mut mins = [0.0; 3];
            let mut maxs = [0.0; 3];
            for axis in 0..3 {
                mins[axis] = -rng.positive();
                maxs[axis] = rng.positive();
            }
            assert_float(
                c_radius(mins.as_ptr(), maxs.as_ptr()),
                r_radius(mins.as_ptr(), maxs.as_ptr()),
                "RadiusFromBounds",
            );

            let (mut cmn, mut cmx, mut rmn, mut rmx) = (mins, maxs, mins, maxs);
            c_clear(cmn.as_mut_ptr(), cmx.as_mut_ptr());
            r_clear(rmn.as_mut_ptr(), rmx.as_mut_ptr());
            assert_floats(&cmn, &rmn, "ClearBounds mins");
            assert_floats(&cmx, &rmx, "ClearBounds maxs");
        }
        for relation in 0..27 {
            let relations = [relation / 9, (relation / 3) % 3, relation % 3];
            for _ in 0..CASES {
                let mins = [
                    -rng.positive() - 1.0,
                    -rng.positive() - 1.0,
                    -rng.positive() - 1.0,
                ];
                let maxs = [
                    rng.positive() + 1.0,
                    rng.positive() + 1.0,
                    rng.positive() + 1.0,
                ];
                let mut point = [0.0; 3];
                for axis in 0..3 {
                    point[axis] = match relations[axis] {
                        0 => mins[axis] - rng.positive(),
                        1 => (mins[axis] + maxs[axis]) * 0.5,
                        _ => maxs[axis] + rng.positive(),
                    };
                }
                let (mut cmn, mut cmx, mut rmn, mut rmx) = (mins, maxs, mins, maxs);
                c_add(point.as_ptr(), cmn.as_mut_ptr(), cmx.as_mut_ptr());
                r_add(point.as_ptr(), rmn.as_mut_ptr(), rmx.as_mut_ptr());
                assert_floats(&cmn, &rmn, "AddPointToBounds mins");
                assert_floats(&cmx, &rmx, "AddPointToBounds maxs");
            }
        }

        type NormalizeFn = unsafe extern "C" fn(*mut f32) -> f32;
        type Normalize2Fn = unsafe extern "C" fn(*const f32, *mut f32) -> f32;
        let (c_norm, r_norm) = libs.pair::<NormalizeFn>(b"VectorNormalize\0");
        let (c_norm2, r_norm2) = libs.pair::<Normalize2Fn>(b"VectorNormalize2\0");
        for zero in [true, false] {
            for _ in 0..CASES {
                let input = if zero {
                    [0.0; 3]
                } else {
                    [rng.finite(), rng.finite(), rng.positive()]
                };
                let (mut c, mut rust) = (input, input);
                assert_float(
                    c_norm(c.as_mut_ptr()),
                    r_norm(rust.as_mut_ptr()),
                    "VectorNormalize return",
                );
                assert_floats(&c, &rust, "VectorNormalize output");
                let (mut c, mut rust) = ([f32::NAN; 3], [f32::NAN; 3]);
                assert_float(
                    c_norm2(input.as_ptr(), c.as_mut_ptr()),
                    r_norm2(input.as_ptr(), rust.as_mut_ptr()),
                    "VectorNormalize2 return",
                );
                assert_floats(&c, &rust, "VectorNormalize2 output");
            }
        }

        type VectorMaFn = unsafe extern "C" fn(*const f32, f32, *const f32, *mut f32);
        type DotFn = unsafe extern "C" fn(*const f32, *const f32) -> f32;
        type BinaryVectorFn = unsafe extern "C" fn(*const f32, *const f32, *mut f32);
        type CopyFn = unsafe extern "C" fn(*const f32, *mut f32);
        type ScaleFn = unsafe extern "C" fn(*const f32, f32, *mut f32);
        let (c_ma, r_ma) = libs.pair::<VectorMaFn>(b"_VectorMA\0");
        let (c_dot, r_dot) = libs.pair::<DotFn>(b"_DotProduct\0");
        let (c_sub, r_sub) = libs.pair::<BinaryVectorFn>(b"_VectorSubtract\0");
        let (c_add_vec, r_add_vec) = libs.pair::<BinaryVectorFn>(b"_VectorAdd\0");
        let (c_copy, r_copy) = libs.pair::<CopyFn>(b"_VectorCopy\0");
        let (c_scale, r_scale) = libs.pair::<ScaleFn>(b"_VectorScale\0");
        let (c_scale4, r_scale4) = libs.pair::<ScaleFn>(b"Vector4Scale\0");
        for _ in 0..CASES {
            let a = [rng.finite(), rng.finite(), rng.finite()];
            let b = [rng.finite(), rng.finite(), rng.finite()];
            let scale = rng.finite();
            let (mut c, mut rust) = ([0.0; 4], [0.0; 4]);

            c_ma(a.as_ptr(), scale, b.as_ptr(), c.as_mut_ptr());
            r_ma(a.as_ptr(), scale, b.as_ptr(), rust.as_mut_ptr());
            assert_floats(&c[..3], &rust[..3], "_VectorMA");
            assert_float(
                c_dot(a.as_ptr(), b.as_ptr()),
                r_dot(a.as_ptr(), b.as_ptr()),
                "_DotProduct",
            );
            c_sub(a.as_ptr(), b.as_ptr(), c.as_mut_ptr());
            r_sub(a.as_ptr(), b.as_ptr(), rust.as_mut_ptr());
            assert_floats(&c[..3], &rust[..3], "_VectorSubtract");
            c_add_vec(a.as_ptr(), b.as_ptr(), c.as_mut_ptr());
            r_add_vec(a.as_ptr(), b.as_ptr(), rust.as_mut_ptr());
            assert_floats(&c[..3], &rust[..3], "_VectorAdd");
            c_copy(a.as_ptr(), c.as_mut_ptr());
            r_copy(a.as_ptr(), rust.as_mut_ptr());
            assert_floats(&c[..3], &rust[..3], "_VectorCopy");
            c_scale(a.as_ptr(), scale, c.as_mut_ptr());
            r_scale(a.as_ptr(), scale, rust.as_mut_ptr());
            assert_floats(&c[..3], &rust[..3], "_VectorScale");
            let input4 = [a[0], a[1], a[2], rng.finite()];
            c_scale4(input4.as_ptr(), scale, c.as_mut_ptr());
            r_scale4(input4.as_ptr(), scale, rust.as_mut_ptr());
            assert_floats(&c, &rust, "Vector4Scale");
        }

        type Log2Fn = unsafe extern "C" fn(i32) -> i32;
        let (c_log2, r_log2) = libs.pair::<Log2Fn>(b"Q_log2\0");
        for value in [0, 1] {
            assert_eq!(c_log2(value), r_log2(value));
        }
        for _ in 0..CASES {
            let value = (rng.u32() & 0x7fff_ffff).max(2) as i32;
            assert_eq!(c_log2(value), r_log2(value), "Q_log2({value})");
        }
    }
}

#[test]
fn matrices_optional_outputs_and_perpendicular_config_rows_134_to_146() {
    let _guard = serial_guard();
    unsafe {
        let libs = Libraries::open();
        let mut rng = Rng::new(0x1357_9bdf);

        type MatrixFn = unsafe extern "C" fn(*const [f32; 3], *const [f32; 3], *mut [f32; 3]);
        let (c_matrix, r_matrix) = libs.pair::<MatrixFn>(b"MatrixMultiply\0");
        for _ in 0..CASES {
            let mut a = [[0.0; 3]; 3];
            let mut b = [[0.0; 3]; 3];
            for value in a.as_flattened_mut().iter_mut() {
                *value = rng.finite();
            }
            for value in b.as_flattened_mut().iter_mut() {
                *value = rng.finite();
            }
            let (mut c, mut rust) = ([[0.0; 3]; 3], [[0.0; 3]; 3]);
            c_matrix(a.as_ptr(), b.as_ptr(), c.as_mut_ptr());
            r_matrix(a.as_ptr(), b.as_ptr(), rust.as_mut_ptr());
            assert_floats(c.as_flattened(), rust.as_flattened(), "MatrixMultiply");
        }

        type AngleVectorsFn = unsafe extern "C" fn(*const f32, *mut f32, *mut f32, *mut f32);
        let (c_vectors, r_vectors) = libs.pair::<AngleVectorsFn>(b"AngleVectors\0");
        for mask in 0_u8..8 {
            for _ in 0..CASES {
                let angles = [rng.finite(), rng.finite(), rng.finite()];
                let (mut cf, mut cr, mut cu) = ([f32::NAN; 3], [f32::NAN; 3], [f32::NAN; 3]);
                let (mut rf, mut rr, mut ru) = (cf, cr, cu);
                c_vectors(
                    angles.as_ptr(),
                    if mask & 1 != 0 {
                        cf.as_mut_ptr()
                    } else {
                        ptr::null_mut()
                    },
                    if mask & 2 != 0 {
                        cr.as_mut_ptr()
                    } else {
                        ptr::null_mut()
                    },
                    if mask & 4 != 0 {
                        cu.as_mut_ptr()
                    } else {
                        ptr::null_mut()
                    },
                );
                r_vectors(
                    angles.as_ptr(),
                    if mask & 1 != 0 {
                        rf.as_mut_ptr()
                    } else {
                        ptr::null_mut()
                    },
                    if mask & 2 != 0 {
                        rr.as_mut_ptr()
                    } else {
                        ptr::null_mut()
                    },
                    if mask & 4 != 0 {
                        ru.as_mut_ptr()
                    } else {
                        ptr::null_mut()
                    },
                );
                assert_floats(&cf, &rf, "AngleVectors forward");
                assert_floats(&cr, &rr, "AngleVectors right");
                assert_floats(&cu, &ru, "AngleVectors up");
            }
        }

        type PerpendicularFn = unsafe extern "C" fn(*mut f32, *const f32);
        let (c_perp, r_perp) = libs.pair::<PerpendicularFn>(b"PerpendicularVector\0");
        let raw_shapes = [
            [0.1_f32, 0.6, 0.8],
            [0.6, 0.1, 0.8],
            [0.6, 0.8, 0.1],
            [0.5, 0.5, 0.70710677],
        ];
        for raw in raw_shapes {
            for _ in 0..CASES {
                let perturb = 1.0 + rng.channel() / 1000.0;
                let mut src = [raw[0] * perturb, raw[1] * perturb, raw[2] * perturb];
                let length =
                    ((src[0] * src[0] + src[1] * src[1] + src[2] * src[2]) as f64).sqrt() as f32;
                for value in &mut src {
                    *value /= length;
                }
                let (mut c, mut rust) = ([0.0; 3], [0.0; 3]);
                c_perp(c.as_mut_ptr(), src.as_ptr());
                r_perp(rust.as_mut_ptr(), src.as_ptr());
                assert_floats(&c, &rust, "PerpendicularVector");
            }
        }
    }
}

#[test]
fn explicit_error_surface_rows_1_to_7() {
    let _guard = serial_guard();
    unsafe {
        let libs = Libraries::open();

        type ClampCharFn = unsafe extern "C" fn(i32) -> i8;
        type ClampShortFn = unsafe extern "C" fn(i32) -> i16;
        let (c_char, r_char) = libs.pair::<ClampCharFn>(b"ClampChar\0");
        let (c_short, r_short) = libs.pair::<ClampShortFn>(b"ClampShort\0");
        for value in [-129, -130, i32::MIN] {
            let c = c_char(value);
            assert_eq!(c, -128, "C sentinel for ERRORS row 1");
            assert_eq!(c, r_char(value), "ERRORS row 1");
        }
        for value in [128, 129, i32::MAX] {
            let c = c_char(value);
            assert_eq!(c, 127, "C sentinel for ERRORS row 2");
            assert_eq!(c, r_char(value), "ERRORS row 2");
        }
        for value in [-32769, -32770, i32::MIN] {
            let c = c_short(value);
            assert_eq!(c, -32768, "C sentinel for ERRORS row 3");
            assert_eq!(c, r_short(value), "ERRORS row 3");
        }
        for value in [32768, 32769, i32::MAX] {
            let c = c_short(value);
            assert_eq!(c, 32767, "C sentinel for ERRORS row 4");
            assert_eq!(c, r_short(value), "ERRORS row 4");
        }

        type DirToByteFn = unsafe extern "C" fn(*const f32) -> i32;
        let (c_dir, r_dir) = libs.pair::<DirToByteFn>(b"DirToByte\0");
        let c = c_dir(ptr::null());
        assert_eq!(c, 0, "C sentinel for ERRORS row 5");
        assert_eq!(c, r_dir(ptr::null()), "ERRORS row 5");

        type ByteToDirFn = unsafe extern "C" fn(i32, *mut f32);
        let (c_byte, r_byte) = libs.pair::<ByteToDirFn>(b"ByteToDir\0");
        for value in [-1, 162, i32::MIN, i32::MAX] {
            let (mut c, mut rust) = ([f32::NAN; 3], [f32::NAN; 3]);
            c_byte(value, c.as_mut_ptr());
            r_byte(value, rust.as_mut_ptr());
            assert_eq!(c, [0.0; 3], "C sentinel for ERRORS row 6");
            assert_floats(&c, &rust, "ERRORS row 6");
        }

        type PlaneFn = unsafe extern "C" fn(*mut f32, *const f32, *const f32, *const f32) -> i32;
        let (c_plane, r_plane) = libs.pair::<PlaneFn>(b"PlaneFromPoints\0");
        let mut rng = Rng::new(0xdead_beef);
        for _ in 0..CASES {
            let a = [rng.finite(), rng.finite(), rng.finite()];
            let b = a;
            let c = [rng.finite(), rng.finite(), rng.finite()];
            let (mut cp, mut rp) = ([0.0; 4], [0.0; 4]);
            let c_result = c_plane(cp.as_mut_ptr(), a.as_ptr(), b.as_ptr(), c.as_ptr());
            let rust_result = r_plane(rp.as_mut_ptr(), a.as_ptr(), b.as_ptr(), c.as_ptr());
            assert_eq!(c_result, 0, "C sentinel for ERRORS row 7");
            assert_eq!(c_result, rust_result, "ERRORS row 7");
            assert_floats(&cp[..3], &rp[..3], "degenerate plane output");
        }
    }
}

#[test]
fn generic_null_pointer_boundaries_match() {
    let _guard = serial_guard();
    unsafe {
        let libs = Libraries::open();
        macro_rules! compare {
            ($name:literal, $ty:ty, $invoke:expr) => {{
                let (c, rust) = libs.pair::<$ty>(concat!($name, "\0").as_bytes());
                let c_status = child_status(|| {
                    ($invoke)(c);
                });
                let rust_status = child_status(|| {
                    ($invoke)(rust);
                });
                assert_eq!(c_status, rust_status, "{} null boundary", $name);
            }};
        }

        type SeedInt = unsafe extern "C" fn(*mut i32) -> i32;
        type SeedFloat = unsafe extern "C" fn(*mut i32) -> f32;
        compare!("Q_rand", SeedInt, |f: SeedInt| f(ptr::null_mut()));
        compare!("Q_random", SeedFloat, |f: SeedFloat| f(ptr::null_mut()));
        compare!("Q_crandom", SeedFloat, |f: SeedFloat| f(ptr::null_mut()));

        type ByteToDir = unsafe extern "C" fn(i32, *mut f32);
        type TwoVec = unsafe extern "C" fn(*const f32, *mut f32);
        type ThreeVec = unsafe extern "C" fn(*const f32, *const f32, *mut f32);
        type NormalizeColor = unsafe extern "C" fn(*const f32, *mut f32) -> f32;
        compare!("ByteToDir", ByteToDir, |f: ByteToDir| f(0, ptr::null_mut()));
        compare!("NormalizeColor", NormalizeColor, |f: NormalizeColor| {
            f(ptr::null(), ptr::null_mut())
        });

        type Plane = unsafe extern "C" fn(*mut f32, *const f32, *const f32, *const f32) -> i32;
        type Rotate = unsafe extern "C" fn(*mut f32, *const f32, *const f32, f32);
        type RotateDirection = unsafe extern "C" fn(*mut [f32; 3], f32);
        compare!("PlaneFromPoints", Plane, |f: Plane| {
            f(ptr::null_mut(), ptr::null(), ptr::null(), ptr::null())
        });
        compare!("RotatePointAroundVector", Rotate, |f: Rotate| {
            f(ptr::null_mut(), ptr::null(), ptr::null(), 0.0)
        });
        compare!(
            "RotateAroundDirection",
            RotateDirection,
            |f: RotateDirection| { f(ptr::null_mut(), 0.0) }
        );

        compare!("vectoangles", TwoVec, |f: TwoVec| {
            f(ptr::null(), ptr::null_mut())
        });
        type AnglesToAxis = unsafe extern "C" fn(*const f32, *mut [f32; 3]);
        type AxisClear = unsafe extern "C" fn(*mut [f32; 3]);
        type AxisCopy = unsafe extern "C" fn(*const [f32; 3], *mut [f32; 3]);
        compare!("AnglesToAxis", AnglesToAxis, |f: AnglesToAxis| {
            f(ptr::null(), ptr::null_mut())
        });
        compare!("AxisClear", AxisClear, |f: AxisClear| f(ptr::null_mut()));
        compare!("AxisCopy", AxisCopy, |f: AxisCopy| {
            f(ptr::null(), ptr::null_mut())
        });

        type Project = unsafe extern "C" fn(*mut f32, *const f32, *const f32);
        type MakeNormal = unsafe extern "C" fn(*const f32, *mut f32, *mut f32);
        type VectorRotate = unsafe extern "C" fn(*const f32, *const [f32; 3], *mut f32);
        compare!("ProjectPointOnPlane", Project, |f: Project| {
            f(ptr::null_mut(), ptr::null(), ptr::null())
        });
        compare!("MakeNormalVectors", MakeNormal, |f: MakeNormal| {
            f(ptr::null(), ptr::null_mut(), ptr::null_mut())
        });
        compare!("VectorRotate", VectorRotate, |f: VectorRotate| {
            f(ptr::null(), ptr::null(), ptr::null_mut())
        });

        compare!("AnglesSubtract", ThreeVec, |f: ThreeVec| {
            f(ptr::null(), ptr::null(), ptr::null_mut())
        });
        type SetSignbits = unsafe extern "C" fn(*mut CPlane);
        type BoxSide = unsafe extern "C" fn(*const f32, *const f32, *const CPlane) -> i32;
        compare!("SetPlaneSignbits", SetSignbits, |f: SetSignbits| {
            f(ptr::null_mut())
        });
        compare!("BoxOnPlaneSide", BoxSide, |f: BoxSide| {
            f(ptr::null(), ptr::null(), ptr::null())
        });

        type Radius = unsafe extern "C" fn(*const f32, *const f32) -> f32;
        type Clear = unsafe extern "C" fn(*mut f32, *mut f32);
        type Add = unsafe extern "C" fn(*const f32, *mut f32, *mut f32);
        compare!("RadiusFromBounds", Radius, |f: Radius| {
            f(ptr::null(), ptr::null())
        });
        compare!("ClearBounds", Clear, |f: Clear| {
            f(ptr::null_mut(), ptr::null_mut())
        });
        compare!("AddPointToBounds", Add, |f: Add| {
            f(ptr::null(), ptr::null_mut(), ptr::null_mut())
        });

        type Normalize = unsafe extern "C" fn(*mut f32) -> f32;
        type Normalize2 = unsafe extern "C" fn(*const f32, *mut f32) -> f32;
        compare!("VectorNormalize", Normalize, |f: Normalize| {
            f(ptr::null_mut())
        });
        compare!("VectorNormalize2", Normalize2, |f: Normalize2| {
            f(ptr::null(), ptr::null_mut())
        });

        type VectorMa = unsafe extern "C" fn(*const f32, f32, *const f32, *mut f32);
        type Dot = unsafe extern "C" fn(*const f32, *const f32) -> f32;
        type Copy = unsafe extern "C" fn(*const f32, *mut f32);
        type Scale = unsafe extern "C" fn(*const f32, f32, *mut f32);
        compare!("_VectorMA", VectorMa, |f: VectorMa| {
            f(ptr::null(), 0.0, ptr::null(), ptr::null_mut())
        });
        compare!("_DotProduct", Dot, |f: Dot| f(ptr::null(), ptr::null()));
        compare!("_VectorSubtract", ThreeVec, |f: ThreeVec| {
            f(ptr::null(), ptr::null(), ptr::null_mut())
        });
        compare!("_VectorAdd", ThreeVec, |f: ThreeVec| {
            f(ptr::null(), ptr::null(), ptr::null_mut())
        });
        compare!("_VectorCopy", Copy, |f: Copy| {
            f(ptr::null(), ptr::null_mut())
        });
        compare!("_VectorScale", Scale, |f: Scale| {
            f(ptr::null(), 0.0, ptr::null_mut())
        });
        compare!("Vector4Scale", Scale, |f: Scale| {
            f(ptr::null(), 0.0, ptr::null_mut())
        });

        type Matrix = unsafe extern "C" fn(*const [f32; 3], *const [f32; 3], *mut [f32; 3]);
        type AngleVectors = unsafe extern "C" fn(*const f32, *mut f32, *mut f32, *mut f32);
        compare!("MatrixMultiply", Matrix, |f: Matrix| {
            f(ptr::null(), ptr::null(), ptr::null_mut())
        });
        compare!("AngleVectors", AngleVectors, |f: AngleVectors| {
            f(
                ptr::null(),
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
            )
        });
        compare!("PerpendicularVector", TwoVec, |f: TwoVec| {
            f(ptr::null(), ptr::null_mut())
        });
    }
}
