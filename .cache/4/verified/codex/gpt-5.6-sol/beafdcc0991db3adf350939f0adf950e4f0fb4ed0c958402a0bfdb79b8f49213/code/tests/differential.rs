use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Pixel {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

#[repr(C)]
struct Image {
    w: c_int,
    h: c_int,
    pix: *mut Pixel,
}

type FlipHorizontal = unsafe extern "C" fn(*mut Image);

const CHILD_LIBRARY_ENV: &str = "FLIP_DIFFERENTIAL_CHILD_LIBRARY";
const CHILD_CASE_ENV: &str = "FLIP_DIFFERENTIAL_CHILD_CASE";
const RANDOM_TRIALS_PER_ROW: usize = 128;

#[derive(Clone, Copy)]
enum WidthShape {
    Empty,
    One,
    Many,
}

#[derive(Clone, Copy)]
enum HeightShape {
    Empty,
    One,
    Two,
    OddMany,
    EvenMany,
}

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

    fn pixel(&mut self) -> Pixel {
        let bytes = self.next_u64().to_ne_bytes();
        Pixel {
            r: bytes[0],
            g: bytes[1],
            b: bytes[2],
            a: bytes[3],
        }
    }
}

fn c_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("c_src")
        .join("build")
        .join("libtranslated_rust.so")
}

fn rust_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("debug")
        .join("libflip_horizontal_lib.so")
}

unsafe fn call_library(path: &Path, image: *mut Image) {
    let library = unsafe { Library::new(path) }
        .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
    let flip: Symbol<FlipHorizontal> = unsafe { library.get(b"flip_horizontal") }
        .unwrap_or_else(|error| panic!("failed to load flip_horizontal: {error}"));
    unsafe { flip(image) };
}

fn dimensions(width: WidthShape, height: HeightShape, rng: &mut Rng) -> (i32, i32) {
    let w = match width {
        WidthShape::Empty => 0,
        WidthShape::One => 1,
        WidthShape::Many => 2 + (rng.next_u64() % 31) as i32,
    };
    let h = match height {
        HeightShape::Empty => 0,
        HeightShape::One => 1,
        HeightShape::Two => 2,
        HeightShape::OddMany => 3 + 2 * (rng.next_u64() % 15) as i32,
        HeightShape::EvenMany => 4 + 2 * (rng.next_u64() % 15) as i32,
    };
    (w, h)
}

fn compare_case(row: usize, w: i32, h: i32, rng: &mut Rng) {
    let logical_len = usize::try_from(w)
        .unwrap()
        .checked_mul(usize::try_from(h).unwrap())
        .unwrap();
    let initial: Vec<Pixel> = (0..logical_len + 2).map(|_| rng.pixel()).collect();
    let mut c_pixels = initial.clone();
    let mut rust_pixels = initial;

    let mut c_image = Image {
        w,
        h,
        // Keep one randomized guard pixel on each side of the logical image.
        pix: unsafe { c_pixels.as_mut_ptr().add(1) },
    };
    let mut rust_image = Image {
        w,
        h,
        pix: unsafe { rust_pixels.as_mut_ptr().add(1) },
    };

    unsafe {
        call_library(&c_library_path(), &mut c_image);
        call_library(&rust_library_path(), &mut rust_image);
    }

    assert_eq!(
        c_pixels, rust_pixels,
        "CONFIGS.md row {row} diverged for width {w}, height {h}"
    );
}

#[test]
fn every_configuration_row_matches_for_randomized_inputs() {
    let widths = [WidthShape::Empty, WidthShape::One, WidthShape::Many];
    let heights = [
        HeightShape::Empty,
        HeightShape::One,
        HeightShape::Two,
        HeightShape::OddMany,
        HeightShape::EvenMany,
    ];
    let mut rng = Rng::new(0x6a09_e667_f3bc_c909);

    for (width_index, width) in widths.into_iter().enumerate() {
        for (height_index, height) in heights.into_iter().enumerate() {
            let row = width_index * heights.len() + height_index + 1;
            for _ in 0..RANDOM_TRIALS_PER_ROW {
                let (w, h) = dimensions(width, height, &mut rng);
                compare_case(row, w, h, &mut rng);
            }
        }
    }
}

fn compare_no_access_boundary(w: i32, h: i32, pix: *mut Pixel) {
    let mut c_image = Image { w, h, pix };
    let mut rust_image = Image { w, h, pix };
    unsafe {
        call_library(&c_library_path(), &mut c_image);
        call_library(&rust_library_path(), &mut rust_image);
    }
}

#[test]
fn zero_work_accepts_null_pixels_and_extreme_dimensions() {
    for (w, h) in [
        (0, 0),
        (1, 0),
        (i32::MIN, 0),
        (i32::MAX, 0),
        (0, 1),
        (1, 1),
        (i32::MIN, 1),
        (i32::MAX, 1),
        (0, i32::MIN),
    ] {
        compare_no_access_boundary(w, h, std::ptr::null_mut());
    }
}

fn run_child(path: &Path, case: &str) -> ExitStatus {
    Command::new(std::env::current_exe().expect("current test executable"))
        .arg("--exact")
        .arg("ffi_boundary_child")
        .arg("--nocapture")
        .env(CHILD_LIBRARY_ENV, path)
        .env(CHILD_CASE_ENV, case)
        .status()
        .expect("run FFI boundary child")
}

#[cfg(unix)]
fn assert_same_signal(case: &str) {
    use std::os::unix::process::ExitStatusExt;

    let c_status = run_child(&c_library_path(), case);
    let rust_status = run_child(&rust_library_path(), case);
    assert!(
        !c_status.success() && !rust_status.success(),
        "{case}: expected both implementations to terminate"
    );
    assert_eq!(
        c_status.signal(),
        rust_status.signal(),
        "{case}: C and Rust terminated with different signals"
    );
}

#[test]
#[cfg(unix)]
fn null_pointer_boundary_behavior_matches() {
    assert_same_signal("null_image");
    assert_same_signal("null_pixels");
}

#[test]
fn ffi_boundary_child() {
    let Ok(library) = std::env::var(CHILD_LIBRARY_ENV) else {
        return;
    };
    let case = std::env::var(CHILD_CASE_ENV).expect("child case");

    match case.as_str() {
        "null_image" => unsafe {
            call_library(Path::new(&library), std::ptr::null_mut());
        },
        "null_pixels" => {
            let mut image = Image {
                w: 1,
                h: 2,
                pix: std::ptr::null_mut(),
            };
            unsafe {
                call_library(Path::new(&library), &mut image);
            }
        }
        _ => panic!("unknown child case: {case}"),
    }
}
