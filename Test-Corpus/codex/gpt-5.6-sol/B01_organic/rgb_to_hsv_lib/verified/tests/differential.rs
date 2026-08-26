use libloading::Library;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

type RgbToHsv = unsafe extern "C" fn(*mut f32, *const f32);

const CASES: usize = 512;
const DEST_INIT: [u32; 3] = [0x7fc0_1234, 0xdead_beef, 0x8000_0000];

struct Libraries {
    c: Library,
    rust: Library,
}

impl Libraries {
    fn load() -> Self {
        unsafe {
            Self {
                c: Library::new(c_library_path()).expect("load C shared object"),
                rust: Library::new(rust_library_path()).expect("load Rust shared object"),
            }
        }
    }

    fn functions(&self) -> (RgbToHsv, RgbToHsv) {
        unsafe {
            let c = *self
                .c
                .get::<RgbToHsv>(b"rgb_to_hsv\0")
                .expect("load C rgb_to_hsv");
            let rust = *self
                .rust
                .get::<RgbToHsv>(b"rgb_to_hsv\0")
                .expect("load Rust rgb_to_hsv");
            (c, rust)
        }
    }
}

#[derive(Clone, Copy)]
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

    fn bounded(&mut self, limit: u32) -> u32 {
        self.next_u32() % limit
    }

    fn finite(&mut self) -> f32 {
        (self.bounded(2_000_001) as i32 - 1_000_000) as f32 / 257.0
    }

    fn nonzero_finite(&mut self) -> f32 {
        loop {
            let value = f32::from_bits(self.next_u32());
            if value.is_finite() && value != 0.0 {
                return value;
            }
        }
    }

    fn quiet_nan(&mut self) -> f32 {
        let sign = self.next_u32() & 0x8000_0000;
        let payload = (self.next_u32() & 0x003f_ffff) | 0x0040_0000;
        f32::from_bits(sign | 0x7f80_0000 | payload)
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    manifest_dir().join("c_src/build/libtranslated_rust.so")
}

fn rust_library_path() -> PathBuf {
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    manifest_dir().join(format!("target/{profile}/librgb_to_hsv_lib.so"))
}

fn assert_case(functions: (RgbToHsv, RgbToHsv), src: [f32; 3]) {
    let mut c_dest = DEST_INIT.map(f32::from_bits);
    let mut rust_dest = DEST_INIT.map(f32::from_bits);
    unsafe {
        functions.0(c_dest.as_mut_ptr(), src.as_ptr());
        functions.1(rust_dest.as_mut_ptr(), src.as_ptr());
    }
    assert_float_bytes_eq(&c_dest, &rust_dest, &src);
}

fn assert_float_bytes_eq(c: &[f32], rust: &[f32], src: &[f32; 3]) {
    let c_bits: Vec<_> = c.iter().map(|value| value.to_bits()).collect();
    let rust_bits: Vec<_> = rust.iter().map(|value| value.to_bits()).collect();
    assert_eq!(
        c_bits,
        rust_bits,
        "input bits: {:08x?}",
        src.map(f32::to_bits)
    );
}

fn shuffled(mut values: [f32; 3], rng: &mut Rng) -> [f32; 3] {
    values.swap(0, rng.bounded(3) as usize);
    values.swap(1, 1 + rng.bounded(2) as usize);
    values
}

#[test]
fn configs_v1_finite_nonzero_grayscale() {
    let libraries = Libraries::load();
    let functions = libraries.functions();
    let mut rng = Rng::new(0x01d1_f5e5_0000_0001);
    for _ in 0..CASES {
        let value = rng.nonzero_finite();
        assert_case(functions, [value; 3]);
    }
}

#[test]
fn configs_v2_signed_zero() {
    let libraries = Libraries::load();
    let functions = libraries.functions();
    let mut rng = Rng::new(0x02d1_f5e5_0000_0002);
    for _ in 0..CASES {
        let mut src = [0.0; 3];
        for value in &mut src {
            *value = f32::from_bits(rng.next_u32() & 0x8000_0000);
        }
        assert_case(functions, src);
    }
}

#[test]
fn configs_v3_unequal_nonpositive_with_zero_maximum() {
    let libraries = Libraries::load();
    let functions = libraries.functions();
    let mut rng = Rng::new(0x03d1_f5e5_0000_0003);
    for _ in 0..CASES {
        let a = -((rng.bounded(10_000) + 1) as f32);
        let b = -((rng.bounded(10_000) + 1) as f32);
        assert_case(functions, shuffled([0.0, a, b], &mut rng));
    }
}

#[test]
fn configs_v4_unique_red_maximum_without_hue_adjustment() {
    let libraries = Libraries::load();
    let functions = libraries.functions();
    let mut rng = Rng::new(0x04d1_f5e5_0000_0004);
    for _ in 0..CASES {
        let b = rng.finite();
        let g = b + (rng.bounded(1_000) + 1) as f32;
        let r = g + (rng.bounded(1_000) + 1) as f32;
        assert_case(functions, [r, g, b]);
    }
}

#[test]
fn configs_v5_unique_red_maximum_with_hue_adjustment() {
    let libraries = Libraries::load();
    let functions = libraries.functions();
    let mut rng = Rng::new(0x05d1_f5e5_0000_0005);
    for _ in 0..CASES {
        let g = rng.finite();
        let b = g + (rng.bounded(1_000) + 1) as f32;
        let r = b + (rng.bounded(1_000) + 1) as f32;
        assert_case(functions, [r, g, b]);
    }
}

#[test]
fn configs_v6_unique_green_maximum() {
    let libraries = Libraries::load();
    let functions = libraries.functions();
    let mut rng = Rng::new(0x06d1_f5e5_0000_0006);
    for _ in 0..CASES {
        let r = rng.finite();
        let b = rng.finite();
        let g = r.max(b) + (rng.bounded(1_000) + 1) as f32;
        assert_case(functions, [r, g, b]);
    }
}

#[test]
fn configs_v7_unique_blue_maximum() {
    let libraries = Libraries::load();
    let functions = libraries.functions();
    let mut rng = Rng::new(0x07d1_f5e5_0000_0007);
    for _ in 0..CASES {
        let r = rng.finite();
        let g = rng.finite();
        let b = r.max(g) + (rng.bounded(1_000) + 1) as f32;
        assert_case(functions, [r, g, b]);
    }
}

#[test]
fn configs_v8_red_green_tied_maximum() {
    let libraries = Libraries::load();
    let functions = libraries.functions();
    let mut rng = Rng::new(0x08d1_f5e5_0000_0008);
    for _ in 0..CASES {
        let low = rng.finite();
        let high = low + (rng.bounded(1_000) + 1) as f32;
        assert_case(functions, [high, high, low]);
    }
}

#[test]
fn configs_v9_red_blue_tied_maximum() {
    let libraries = Libraries::load();
    let functions = libraries.functions();
    let mut rng = Rng::new(0x09d1_f5e5_0000_0009);
    for _ in 0..CASES {
        let low = rng.finite();
        let high = low + (rng.bounded(1_000) + 1) as f32;
        assert_case(functions, [high, low, high]);
    }
}

#[test]
fn configs_v10_green_blue_tied_maximum() {
    let libraries = Libraries::load();
    let functions = libraries.functions();
    let mut rng = Rng::new(0x10d1_f5e5_0000_0010);
    for _ in 0..CASES {
        let low = rng.finite();
        let high = low + (rng.bounded(1_000) + 1) as f32;
        assert_case(functions, [low, high, high]);
    }
}

#[test]
fn configs_v11_finite_negative_unequal() {
    let libraries = Libraries::load();
    let functions = libraries.functions();
    let mut rng = Rng::new(0x11d1_f5e5_0000_0011);
    for _ in 0..CASES {
        let high = -((rng.bounded(10_000) + 1) as f32);
        let middle = high - (rng.bounded(1_000) + 1) as f32;
        let low = middle - (rng.bounded(1_000) + 1) as f32;
        assert_case(functions, shuffled([high, middle, low], &mut rng));
    }
}

#[test]
fn configs_v12_subnormal_and_extreme_finite_values() {
    let libraries = Libraries::load();
    let functions = libraries.functions();
    let mut rng = Rng::new(0x12d1_f5e5_0000_0012);
    let templates = [
        [f32::from_bits(1), f32::from_bits(2), f32::from_bits(3)],
        [f32::MAX, -f32::MAX, 0.0],
        [f32::MAX, f32::MIN_POSITIVE, -f32::MAX],
        [-f32::MAX, -f32::MIN_POSITIVE, -f32::from_bits(1)],
    ];
    for index in 0..CASES {
        let mut src = templates[index % templates.len()];
        src = shuffled(src, &mut rng);
        if rng.next_u32() & 1 != 0 {
            src[rng.bounded(3) as usize] = -src[rng.bounded(3) as usize];
        }
        assert_case(functions, src);
    }
}

#[test]
fn configs_v13_nan_in_red() {
    let libraries = Libraries::load();
    let functions = libraries.functions();
    let mut rng = Rng::new(0x13d1_f5e5_0000_0013);
    for _ in 0..CASES {
        assert_case(functions, [rng.quiet_nan(), rng.finite(), rng.finite()]);
    }
}

#[test]
fn configs_v14_nan_in_green() {
    let libraries = Libraries::load();
    let functions = libraries.functions();
    let mut rng = Rng::new(0x14d1_f5e5_0000_0014);
    for _ in 0..CASES {
        assert_case(functions, [rng.finite(), rng.quiet_nan(), rng.finite()]);
    }
}

#[test]
fn configs_v15_nan_in_blue() {
    let libraries = Libraries::load();
    let functions = libraries.functions();
    let mut rng = Rng::new(0x15d1_f5e5_0000_0015);
    for _ in 0..CASES {
        assert_case(functions, [rng.finite(), rng.finite(), rng.quiet_nan()]);
    }
}

#[test]
fn configs_v16_infinities() {
    let libraries = Libraries::load();
    let functions = libraries.functions();
    let mut rng = Rng::new(0x16d1_f5e5_0000_0016);
    for _ in 0..CASES {
        let infinity = if rng.next_u32() & 1 == 0 {
            f32::INFINITY
        } else {
            f32::NEG_INFINITY
        };
        let src = shuffled([infinity, rng.finite(), rng.finite()], &mut rng);
        assert_case(functions, src);
    }
}

#[test]
fn configs_v17_exact_in_place_operation() {
    let libraries = Libraries::load();
    let functions = libraries.functions();
    let mut rng = Rng::new(0x17d1_f5e5_0000_0017);
    for _ in 0..CASES {
        let input = [
            f32::from_bits(rng.next_u32()),
            f32::from_bits(rng.next_u32()),
            f32::from_bits(rng.next_u32()),
        ];
        let mut c = input;
        let mut rust = input;
        unsafe {
            functions.0(c.as_mut_ptr(), c.as_ptr());
            functions.1(rust.as_mut_ptr(), rust.as_ptr());
        }
        assert_float_bytes_eq(&c, &rust, &input);
    }
}

#[test]
fn configs_v18_partially_overlapping_regions() {
    let libraries = Libraries::load();
    let functions = libraries.functions();
    let mut rng = Rng::new(0x18d1_f5e5_0000_0018);
    for index in 0..CASES {
        let input = [
            f32::from_bits(rng.next_u32()),
            f32::from_bits(rng.next_u32()),
            f32::from_bits(rng.next_u32()),
            f32::from_bits(rng.next_u32()),
        ];
        let (src_offset, dest_offset) = if index & 1 == 0 { (0, 1) } else { (1, 0) };
        let mut c = input;
        let mut rust = input;
        unsafe {
            functions.0(c.as_mut_ptr().add(dest_offset), c.as_ptr().add(src_offset));
            functions.1(
                rust.as_mut_ptr().add(dest_offset),
                rust.as_ptr().add(src_offset),
            );
        }
        let src = [
            input[src_offset],
            input[src_offset + 1],
            input[src_offset + 2],
        ];
        assert_float_bytes_eq(&c, &rust, &src);
    }
}

#[test]
fn errors_required_null_pointer_probes() {
    if std::env::var_os("RGB_TO_HSV_NULL_PROBE").is_some() {
        return;
    }

    for boundary in ["dest", "src"] {
        let c_status = run_null_child(&c_library_path(), boundary);
        let rust_status = run_null_child(&rust_library_path(), boundary);
        assert_eq!(
            status_identity(c_status),
            status_identity(rust_status),
            "{boundary} null behavior differs: C={c_status:?}, Rust={rust_status:?}"
        );
        assert!(
            !c_status.success(),
            "{boundary} null unexpectedly succeeded"
        );
    }
}

#[test]
fn null_probe_child() {
    let Some(library_path) = std::env::var_os("RGB_TO_HSV_NULL_PROBE") else {
        return;
    };
    let boundary = std::env::var("RGB_TO_HSV_NULL_BOUNDARY").expect("null boundary");
    let library = unsafe { Library::new(library_path).expect("load probe library") };
    let function = unsafe {
        *library
            .get::<RgbToHsv>(b"rgb_to_hsv\0")
            .expect("load probe function")
    };
    let mut dest = [0.0f32; 3];
    let src = [0.25f32, 0.5, 0.75];
    unsafe {
        match boundary.as_str() {
            "dest" => function(std::ptr::null_mut(), src.as_ptr()),
            "src" => function(dest.as_mut_ptr(), std::ptr::null()),
            _ => panic!("unknown null boundary"),
        }
    }
}

fn run_null_child(library: &Path, boundary: &str) -> ExitStatus {
    Command::new(std::env::current_exe().expect("current test executable"))
        .arg("--exact")
        .arg("null_probe_child")
        .arg("--nocapture")
        .env("RGB_TO_HSV_NULL_PROBE", library)
        .env("RGB_TO_HSV_NULL_BOUNDARY", boundary)
        .status()
        .expect("run null probe child")
}

#[cfg(unix)]
fn status_identity(status: ExitStatus) -> (Option<i32>, Option<i32>) {
    use std::os::unix::process::ExitStatusExt;
    (status.code(), status.signal())
}

#[cfg(not(unix))]
fn status_identity(status: ExitStatus) -> (Option<i32>, Option<i32>) {
    (status.code(), None)
}
