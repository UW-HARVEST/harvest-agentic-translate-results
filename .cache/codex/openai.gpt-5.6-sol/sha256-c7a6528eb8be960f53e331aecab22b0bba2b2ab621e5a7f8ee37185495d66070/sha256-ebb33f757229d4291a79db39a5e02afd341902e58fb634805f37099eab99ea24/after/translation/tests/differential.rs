use libloading::{Library, Symbol};
use std::env;
use std::ffi::c_int;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct CpPixel {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

#[repr(C)]
struct CpImage {
    w: c_int,
    h: c_int,
    pix: *mut CpPixel,
}

type FlipHorizontal = unsafe extern "C" fn(*mut CpImage);

fn c_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/build/libharvest-work-dDNg7n.so")
}

fn rust_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("target/release/libflip_horizontal_lib.so")
}

unsafe fn call_library(path: &Path, image: *mut CpImage) {
    let library = unsafe { Library::new(path) }
        .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
    let flip: Symbol<FlipHorizontal> = unsafe { library.get(b"flip_horizontal\0") }
        .unwrap_or_else(|error| panic!("failed to load flip_horizontal: {error}"));
    unsafe { flip(image) };
}

#[derive(Clone)]
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value
    }

    fn next_u8(&mut self) -> u8 {
        self.next_u64() as u8
    }

    fn range(&mut self, start: usize, end_exclusive: usize) -> usize {
        start + (self.next_u64() as usize % (end_exclusive - start))
    }
}

fn random_pixels(rng: &mut Rng, count: usize) -> Vec<CpPixel> {
    (0..count)
        .map(|_| CpPixel {
            r: rng.next_u8(),
            g: rng.next_u8(),
            b: rng.next_u8(),
            a: rng.next_u8(),
        })
        .collect()
}

fn pixel_bytes(pixels: &[CpPixel]) -> &[u8] {
    assert_eq!(std::mem::size_of::<CpPixel>(), 4);
    unsafe {
        std::slice::from_raw_parts(pixels.as_ptr().cast::<u8>(), std::mem::size_of_val(pixels))
    }
}

fn compare_valid_case(w: usize, h: usize, rng: &mut Rng) {
    let logical_len = w.checked_mul(h).expect("test dimensions overflowed usize");
    // A real allocation keeps pix valid even for zero-pixel image shapes.
    let allocation_len = logical_len.max(1);
    let input = random_pixels(rng, allocation_len);
    let mut c_pixels = input.clone();
    let mut rust_pixels = input;

    let mut c_image = CpImage {
        w: c_int::try_from(w).unwrap(),
        h: c_int::try_from(h).unwrap(),
        pix: c_pixels.as_mut_ptr(),
    };
    let mut rust_image = CpImage {
        w: c_int::try_from(w).unwrap(),
        h: c_int::try_from(h).unwrap(),
        pix: rust_pixels.as_mut_ptr(),
    };

    unsafe {
        call_library(&c_library_path(), &mut c_image);
        call_library(&rust_library_path(), &mut rust_image);
    }

    assert_eq!(
        pixel_bytes(&rust_pixels),
        pixel_bytes(&c_pixels),
        "byte output mismatch for w={w}, h={h}"
    );
}

#[test]
fn config_01_empty_image() {
    let mut rng = Rng::new(0x0101_5eed_cafe_beef);
    for _ in 0..128 {
        compare_valid_case(0, 0, &mut rng);
    }
}

#[test]
fn config_02_one_row() {
    let mut rng = Rng::new(0x0202_5eed_cafe_beef);
    for _ in 0..128 {
        let w = rng.range(1, 65);
        compare_valid_case(w, 1, &mut rng);
    }
}

#[test]
fn config_03_one_pair_zero_columns() {
    let mut rng = Rng::new(0x0303_5eed_cafe_beef);
    for _ in 0..128 {
        compare_valid_case(0, 2, &mut rng);
    }
}

#[test]
fn config_04_one_pair_one_column() {
    let mut rng = Rng::new(0x0404_5eed_cafe_beef);
    for _ in 0..128 {
        compare_valid_case(1, 2, &mut rng);
    }
}

#[test]
fn config_05_one_pair_many_columns() {
    let mut rng = Rng::new(0x0505_5eed_cafe_beef);
    for _ in 0..128 {
        let w = rng.range(2, 65);
        compare_valid_case(w, 2, &mut rng);
    }
}

#[test]
fn config_06_odd_rows_zero_columns() {
    let mut rng = Rng::new(0x0606_5eed_cafe_beef);
    for _ in 0..128 {
        let h = 3 + 2 * rng.range(0, 16);
        compare_valid_case(0, h, &mut rng);
    }
}

#[test]
fn config_07_odd_rows_one_column() {
    let mut rng = Rng::new(0x0707_5eed_cafe_beef);
    for _ in 0..128 {
        let h = 3 + 2 * rng.range(0, 16);
        compare_valid_case(1, h, &mut rng);
    }
}

#[test]
fn config_08_odd_rows_many_columns() {
    let mut rng = Rng::new(0x0808_5eed_cafe_beef);
    for _ in 0..128 {
        let h = 3 + 2 * rng.range(0, 16);
        let w = rng.range(2, 65);
        compare_valid_case(w, h, &mut rng);
    }
}

#[test]
fn config_09_even_rows_zero_columns() {
    let mut rng = Rng::new(0x0909_5eed_cafe_beef);
    for _ in 0..128 {
        let h = 4 + 2 * rng.range(0, 16);
        compare_valid_case(0, h, &mut rng);
    }
}

#[test]
fn config_10_even_rows_one_column() {
    let mut rng = Rng::new(0x1010_5eed_cafe_beef);
    for _ in 0..128 {
        let h = 4 + 2 * rng.range(0, 16);
        compare_valid_case(1, h, &mut rng);
    }
}

#[test]
fn config_11_even_rows_many_columns() {
    let mut rng = Rng::new(0x1111_5eed_cafe_beef);
    for _ in 0..128 {
        let h = 4 + 2 * rng.range(0, 16);
        let w = rng.range(2, 65);
        compare_valid_case(w, h, &mut rng);
    }
}

fn call_with_null_pixels(path: &Path, w: c_int, h: c_int) {
    let mut image = CpImage {
        w,
        h,
        pix: std::ptr::null_mut(),
    };
    unsafe { call_library(path, &mut image) };
}

fn assert_both_return_with_null_pixels(w: c_int, h: c_int) {
    call_with_null_pixels(&c_library_path(), w, h);
    call_with_null_pixels(&rust_library_path(), w, h);
}

#[test]
fn error_03_zero_dimensions() {
    assert_both_return_with_null_pixels(0, 0);
}

#[test]
fn error_04_zero_width_positive_height() {
    assert_both_return_with_null_pixels(0, 7);
}

#[test]
fn error_05_negative_height() {
    assert_both_return_with_null_pixels(1, -1);
}

#[test]
fn error_06_negative_width_without_outer_iterations() {
    assert_both_return_with_null_pixels(-1, 1);
}

#[test]
fn error_07_oversized_width_without_outer_iterations() {
    assert_both_return_with_null_pixels(c_int::MAX, 1);
}

#[test]
fn error_08_extreme_negative_height() {
    assert_both_return_with_null_pixels(1, c_int::MIN);
}

#[test]
fn ffi_crash_probe() {
    let Some(path) = env::var_os("DIFF_CRASH_LIBRARY") else {
        return;
    };
    let case = env::var("DIFF_CRASH_CASE").expect("DIFF_CRASH_CASE is required");

    match case.as_str() {
        "null_image" => unsafe {
            call_library(Path::new(&path), std::ptr::null_mut());
        },
        "null_pixels" => {
            call_with_null_pixels(Path::new(&path), 1, 2);
        }
        other => panic!("unknown crash probe case: {other}"),
    }
}

fn run_crash_probe(path: &Path, case: &str) -> ExitStatus {
    Command::new(env::current_exe().expect("current test executable"))
        .args(["--exact", "ffi_crash_probe", "--nocapture"])
        .env("DIFF_CRASH_LIBRARY", path)
        .env("DIFF_CRASH_CASE", case)
        .status()
        .expect("failed to run crash probe")
}

#[cfg(unix)]
fn assert_matching_sigsegv(case: &str) {
    let c_status = run_crash_probe(&c_library_path(), case);
    let rust_status = run_crash_probe(&rust_library_path(), case);
    assert_eq!(c_status.signal(), Some(11), "C status was {c_status:?}");
    assert_eq!(
        rust_status.signal(),
        c_status.signal(),
        "Rust status {rust_status:?} differed from C status {c_status:?}"
    );
}

#[test]
#[cfg(unix)]
fn error_01_null_image_pointer() {
    assert_matching_sigsegv("null_image");
}

#[test]
#[cfg(unix)]
fn error_02_null_pixel_pointer_when_accessed() {
    assert_matching_sigsegv("null_pixels");
}
