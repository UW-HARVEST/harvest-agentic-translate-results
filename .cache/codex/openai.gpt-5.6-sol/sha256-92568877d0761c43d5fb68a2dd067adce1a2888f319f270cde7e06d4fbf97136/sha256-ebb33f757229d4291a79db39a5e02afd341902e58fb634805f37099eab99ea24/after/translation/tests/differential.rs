use libloading::Library;
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::mem::size_of;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct CpPixel {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

#[repr(C)]
struct CpImage {
    w: i32,
    h: i32,
    pix: *mut CpPixel,
}

type Premultiply = unsafe extern "C" fn(*mut CpImage);

struct Api {
    _library: Library,
    premultiply: Premultiply,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let premultiply = unsafe {
            *library
                .get::<Premultiply>(b"premultiply\0")
                .unwrap_or_else(|error| {
                    panic!(
                        "failed to load premultiply from {}: {error}",
                        path.display()
                    )
                })
        };
        Self {
            _library: library,
            premultiply,
        }
    }

    unsafe fn call(&self, image: *mut CpImage) {
        unsafe { (self.premultiply)(image) }
    }
}

struct Apis {
    c: Api,
    rust: Api,
}

impl Apis {
    fn load() -> Self {
        let c_path = c_library_path();
        let rust_path = rust_library_path();
        assert!(
            c_path.is_file(),
            "C shared library is missing: {}",
            c_path.display()
        );
        assert!(
            rust_path.is_file(),
            "Rust shared library is missing: {}",
            rust_path.display()
        );
        unsafe {
            Self {
                c: Api::load(&c_path),
                rust: Api::load(&rust_path),
            }
        }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    let build_dir = manifest_dir().join("../c_src/build");
    let mut candidates: Vec<_> = fs::read_dir(&build_dir)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", build_dir.display()))
        .map(|entry| entry.expect("failed to inspect C build entry").path())
        .filter(|path| {
            path.extension() == Some(OsStr::new("so"))
                && path
                    .file_name()
                    .is_some_and(|name| name.as_encoded_bytes().starts_with(b"lib"))
        })
        .collect();
    candidates.sort();
    assert_eq!(
        candidates.len(),
        1,
        "expected exactly one C shared library in {}, found {candidates:?}",
        build_dir.display()
    );
    candidates.remove(0)
}

fn rust_library_path() -> PathBuf {
    manifest_dir().join("target/release/libpremultiply_lib.so")
}

fn pixels_as_bytes(pixels: &[CpPixel]) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(pixels.as_ptr().cast::<u8>(), std::mem::size_of_val(pixels))
    }
}

fn compare_pixels(apis: &Apis, w: i32, h: i32, input: &[CpPixel]) {
    let mut c_pixels = input.to_vec();
    let mut rust_pixels = input.to_vec();
    let mut c_image = CpImage {
        w,
        h,
        pix: c_pixels.as_mut_ptr(),
    };
    let mut rust_image = CpImage {
        w,
        h,
        pix: rust_pixels.as_mut_ptr(),
    };

    unsafe {
        apis.c.call(&mut c_image);
        apis.rust.call(&mut rust_image);
    }

    assert_eq!(
        pixels_as_bytes(&rust_pixels),
        pixels_as_bytes(&c_pixels),
        "byte mismatch for w={w}, h={h}"
    );
}

fn compare_null_data_noop(apis: &Apis, w: i32, h: i32) {
    let mut c_image = CpImage {
        w,
        h,
        pix: std::ptr::null_mut(),
    };
    let mut rust_image = CpImage {
        w,
        h,
        pix: std::ptr::null_mut(),
    };
    unsafe {
        apis.c.call(&mut c_image);
        apis.rust.call(&mut rust_image);
    }
}

#[derive(Clone)]
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn u8(&mut self) -> u8 {
        self.next_u64() as u8
    }

    fn usize_inclusive(&mut self, min: usize, max: usize) -> usize {
        min + (self.next_u64() as usize % (max - min + 1))
    }
}

fn random_pixels(rng: &mut Rng, count: usize, case: usize) -> Vec<CpPixel> {
    let mut pixels = Vec::with_capacity(count);
    for index in 0..count {
        let a = match (case + index) % 8 {
            0 => 0,
            1 => 255,
            _ => rng.u8(),
        };
        pixels.push(CpPixel {
            r: rng.u8(),
            g: rng.u8(),
            b: rng.u8(),
            a,
        });
    }
    pixels
}

#[test]
fn config_c1_one_pixel() {
    let apis = Apis::load();
    let mut rng = Rng::new(0x91d3_0a6f_52c8_7be1);
    for case in 0..512 {
        compare_pixels(&apis, 1, 1, &random_pixels(&mut rng, 1, case));
    }
}

#[test]
fn config_c2_one_row() {
    let apis = Apis::load();
    let mut rng = Rng::new(0x02b4_9f71_ee06_d8c3);
    for case in 0..256 {
        let width = rng.usize_inclusive(2, 128);
        compare_pixels(
            &apis,
            width as i32,
            1,
            &random_pixels(&mut rng, width, case),
        );
    }
}

#[test]
fn config_c3_one_column() {
    let apis = Apis::load();
    let mut rng = Rng::new(0xd2ec_64bf_8d1a_a751);
    for case in 0..256 {
        let height = rng.usize_inclusive(2, 128);
        compare_pixels(
            &apis,
            1,
            height as i32,
            &random_pixels(&mut rng, height, case),
        );
    }
}

#[test]
fn config_c4_rectangle() {
    let apis = Apis::load();
    let mut rng = Rng::new(0x7f48_619a_30ed_b205);
    for case in 0..256 {
        let width = rng.usize_inclusive(2, 48);
        let height = rng.usize_inclusive(2, 48);
        compare_pixels(
            &apis,
            width as i32,
            height as i32,
            &random_pixels(&mut rng, width * height, case),
        );
    }
}

#[test]
fn config_c5_empty() {
    let apis = Apis::load();
    let mut rng = Rng::new(0x350a_7d19_bce4_826f);
    for _ in 0..256 {
        let positive = rng.usize_inclusive(1, 1_000_000) as i32;
        compare_null_data_noop(&apis, 0, positive);
        compare_null_data_noop(&apis, positive, 0);
        compare_null_data_noop(&apis, 0, 0);
    }
}

#[test]
fn config_c6_negative_bound() {
    let apis = Apis::load();
    let mut rng = Rng::new(0x84b9_1c0d_37ea_65f2);
    for _ in 0..256 {
        let width = rng.usize_inclusive(1, 16_000) as i32;
        let height = rng.usize_inclusive(1, 16_000) as i32;
        compare_null_data_noop(&apis, -width, height);
        compare_null_data_noop(&apis, width, -height);
    }
}

#[test]
fn config_c7_two_negative_dimensions() {
    let apis = Apis::load();
    let mut rng = Rng::new(0xe37a_d564_901f_2bc8);
    for case in 0..256 {
        let width = rng.usize_inclusive(1, 32);
        let height = rng.usize_inclusive(1, 32);
        compare_pixels(
            &apis,
            -(width as i32),
            -(height as i32),
            &random_pixels(&mut rng, width * height, case),
        );
    }
}

#[test]
fn all_channel_alpha_pairs_match() {
    let apis = Apis::load();
    let mut pixels = Vec::with_capacity(256 * 256);
    for alpha in 0..=u8::MAX {
        for channel in 0..=u8::MAX {
            pixels.push(CpPixel {
                r: channel,
                g: channel,
                b: channel,
                a: alpha,
            });
        }
    }
    compare_pixels(&apis, pixels.len() as i32, 1, &pixels);
}

#[test]
fn boundary_g3_g4_g5_non_dereferencing_shapes() {
    let apis = Apis::load();
    compare_null_data_noop(&apis, 0, 1);
    compare_null_data_noop(&apis, 1, 0);
    compare_null_data_noop(&apis, -1, 1);
    compare_null_data_noop(&apis, 1, -1);
    compare_null_data_noop(&apis, i32::MAX / size_of::<CpPixel>() as i32, 0);
}

fn run_crash_child(library: &Path, case: &str) -> ExitStatus {
    Command::new(env::current_exe().expect("failed to locate differential test binary"))
        .arg("--exact")
        .arg("ffi_crash_probe")
        .arg("--nocapture")
        .env("PREMULTIPLY_CRASH_LIBRARY", library)
        .env("PREMULTIPLY_CRASH_CASE", case)
        .status()
        .expect("failed to execute crash probe")
}

#[test]
fn boundary_g1_g2_null_pointer_behavior_matches() {
    for case in ["null_image", "null_pixels"] {
        let c_status = run_crash_child(&c_library_path(), case);
        let rust_status = run_crash_child(&rust_library_path(), case);
        assert!(!c_status.success(), "C unexpectedly accepted {case}");
        assert!(!rust_status.success(), "Rust unexpectedly accepted {case}");
        #[cfg(unix)]
        assert_eq!(
            rust_status.signal(),
            c_status.signal(),
            "termination signal differs for {case}: C={c_status:?}, Rust={rust_status:?}"
        );
        #[cfg(not(unix))]
        assert_eq!(
            rust_status.code(),
            c_status.code(),
            "termination status differs for {case}: C={c_status:?}, Rust={rust_status:?}"
        );
    }
}

#[test]
fn ffi_crash_probe() {
    let Some(library) = env::var_os("PREMULTIPLY_CRASH_LIBRARY") else {
        return;
    };
    let case = env::var("PREMULTIPLY_CRASH_CASE").expect("missing crash case");
    let api = unsafe { Api::load(Path::new(&library)) };
    match case.as_str() {
        "null_image" => unsafe { api.call(std::ptr::null_mut()) },
        "null_pixels" => {
            let mut image = CpImage {
                w: 1,
                h: 1,
                pix: std::ptr::null_mut(),
            };
            unsafe { api.call(&mut image) };
        }
        _ => panic!("unknown crash case: {case}"),
    }
}
