use libloading::Library;
use std::env;
use std::ffi::c_int;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;

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

type Premultiply = unsafe extern "C" fn(*mut Image);

struct Api {
    _library: Library,
    premultiply: Premultiply,
}

impl Api {
    fn load(path: &Path) -> Self {
        assert!(
            path.is_file(),
            "shared library does not exist: {}",
            path.display()
        );
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

    unsafe fn call(&self, image: *mut Image) {
        unsafe { (self.premultiply)(image) };
    }
}

struct Libraries {
    c: Api,
    rust: Api,
}

impl Libraries {
    fn load() -> Self {
        Self {
            c: Api::load(&c_library_path()),
            rust: Api::load(&rust_library_path()),
        }
    }
}

fn c_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("c_src/build/libtranslated_rust.so")
}

fn rust_library_path() -> PathBuf {
    if let Some(path) = env::var_os("RUST_PREMULTIPLY_SO") {
        return path.into();
    }

    Path::new(env!("CARGO_MANIFEST_DIR")).join("target/release/libpremultiply_lib.so")
}

fn next_random(state: &mut u64) -> u8 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    (*state >> 24) as u8
}

fn randomized_pixels(count: usize, iteration: usize, state: &mut u64) -> Vec<Pixel> {
    let mut pixels = Vec::with_capacity(count);
    for index in 0..count {
        let mut pixel = Pixel {
            r: next_random(state),
            g: next_random(state),
            b: next_random(state),
            a: next_random(state),
        };

        if index == 0 {
            pixel.r = match iteration % 3 {
                0 => 0,
                1 => u8::MAX,
                _ => next_random(state),
            };
            pixel.g = match iteration % 3 {
                0 => u8::MAX,
                1 => 0,
                _ => next_random(state),
            };
            pixel.a = match iteration % 3 {
                0 => 0,
                1 => u8::MAX,
                _ => 1 + next_random(state) % 254,
            };
        }
        pixels.push(pixel);
    }
    pixels
}

fn as_bytes(pixels: &[Pixel]) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(pixels.as_ptr().cast::<u8>(), std::mem::size_of_val(pixels))
    }
}

fn differential_dimensions(row: usize, w: c_int, h: c_int, active_pixels: usize) {
    assert_eq!(std::mem::size_of::<Pixel>(), 4);
    let libraries = Libraries::load();
    let storage_pixels = active_pixels.max(16) + 4;
    let mut state = 0xd1ff_3a5e_91c0_4b27_u64 ^ row as u64;

    for iteration in 0..256 {
        let input = randomized_pixels(storage_pixels, iteration, &mut state);
        let mut c_pixels = input.clone();
        let mut rust_pixels = input.clone();
        let mut c_image = Image {
            w,
            h,
            pix: c_pixels.as_mut_ptr(),
        };
        let mut rust_image = Image {
            w,
            h,
            pix: rust_pixels.as_mut_ptr(),
        };

        unsafe {
            libraries.c.call(&mut c_image);
            libraries.rust.call(&mut rust_image);
        }

        assert_eq!(
            as_bytes(&rust_pixels),
            as_bytes(&c_pixels),
            "CONFIGS.md row {row}, iteration {iteration}, dimensions {w}x{h}"
        );
        assert_eq!(
            &c_pixels[active_pixels..],
            &input[active_pixels..],
            "C canary changed for CONFIGS.md row {row}, iteration {iteration}"
        );
        assert_eq!(
            &rust_pixels[active_pixels..],
            &input[active_pixels..],
            "Rust canary changed for CONFIGS.md row {row}, iteration {iteration}"
        );
    }
}

macro_rules! dimension_test {
    ($name:ident, $row:literal, $w:expr, $h:expr, $active:expr) => {
        #[test]
        fn $name() {
            differential_dimensions($row, $w, $h, $active);
        }
    };
}

dimension_test!(config_01_zero_by_zero, 1, 0, 0, 0);
dimension_test!(config_02_zero_by_positive, 2, 0, 7, 0);
dimension_test!(config_03_zero_by_negative, 3, 0, -7, 0);
dimension_test!(config_04_positive_by_zero, 4, 7, 0, 0);
dimension_test!(config_05_negative_by_zero, 5, -7, 0, 0);
dimension_test!(config_06_positive_by_negative, 6, 7, -5, 0);
dimension_test!(config_07_negative_by_positive, 7, -7, 5, 0);
dimension_test!(config_08_one_by_one, 8, 1, 1, 1);
dimension_test!(config_09_many_by_one, 9, 11, 1, 11);
dimension_test!(config_10_one_by_many, 10, 1, 13, 13);
dimension_test!(config_11_many_by_many, 11, 7, 9, 63);
dimension_test!(config_12_negative_one_by_negative_one, 12, -1, -1, 1);
dimension_test!(config_13_negative_many_by_negative_one, 13, -11, -1, 11);
dimension_test!(config_14_negative_one_by_negative_many, 14, -1, -13, 13);
dimension_test!(config_15_negative_many_by_negative_many, 15, -7, -9, 63);

#[test]
fn config_11_all_channel_alpha_pairs() {
    let libraries = Libraries::load();
    let mut input = Vec::with_capacity(256 * 256);
    for alpha in 0..=u8::MAX {
        for channel in 0..=u8::MAX {
            input.push(Pixel {
                r: channel,
                g: u8::MAX - channel,
                b: channel.wrapping_mul(73),
                a: alpha,
            });
        }
    }

    let mut c_pixels = input.clone();
    let mut rust_pixels = input;
    let mut c_image = Image {
        w: 256,
        h: 256,
        pix: c_pixels.as_mut_ptr(),
    };
    let mut rust_image = Image {
        w: 256,
        h: 256,
        pix: rust_pixels.as_mut_ptr(),
    };
    unsafe {
        libraries.c.call(&mut c_image);
        libraries.rust.call(&mut rust_image);
    }
    assert_eq!(as_bytes(&rust_pixels), as_bytes(&c_pixels));
}

#[test]
fn error_g3_null_pixels_on_inactive_paths() {
    let libraries = Libraries::load();
    for (w, h) in [(0, 9), (9, 0), (-9, 7), (9, -7)] {
        let mut c_image = Image {
            w,
            h,
            pix: std::ptr::null_mut(),
        };
        let mut rust_image = Image {
            w,
            h,
            pix: std::ptr::null_mut(),
        };
        unsafe {
            libraries.c.call(&mut c_image);
            libraries.rust.call(&mut rust_image);
        }
    }
}

#[test]
fn error_g4_zero_dimensions_preserve_storage() {
    differential_dimensions(1, 0, 0, 0);
    differential_dimensions(2, 0, 17, 0);
    differential_dimensions(4, 17, 0, 0);
}

#[test]
fn error_g5_extreme_dimensions_match_c() {
    let libraries = Libraries::load();
    let dimensions = [
        (c_int::MIN, 1),
        (c_int::MIN, -1),
        (c_int::MAX, 0),
        (c_int::MAX, 1),
        (c_int::MAX, -1),
    ];

    for (w, h) in dimensions {
        let mut c_pixels = [Pixel {
            r: 17,
            g: 83,
            b: 241,
            a: 129,
        }];
        let mut rust_pixels = c_pixels;
        let mut c_image = Image {
            w,
            h,
            pix: c_pixels.as_mut_ptr(),
        };
        let mut rust_image = Image {
            w,
            h,
            pix: rust_pixels.as_mut_ptr(),
        };
        unsafe {
            libraries.c.call(&mut c_image);
            libraries.rust.call(&mut rust_image);
        }
        assert_eq!(rust_pixels, c_pixels, "extreme dimensions {w}x{h}");
    }
}

fn run_crash_probe(library: &str, kind: &str) -> ExitStatus {
    Command::new(env::current_exe().expect("current test executable"))
        .args(["--exact", "ffi_crash_probe", "--test-threads=1"])
        .env("DIFF_CRASH_LIBRARY", library)
        .env("DIFF_CRASH_KIND", kind)
        .status()
        .unwrap_or_else(|error| panic!("failed to run {library}/{kind} crash probe: {error}"))
}

fn assert_matching_crash(kind: &str) {
    let c_status = run_crash_probe("c", kind);
    let rust_status = run_crash_probe("rust", kind);
    assert!(!c_status.success(), "C {kind} probe unexpectedly succeeded");
    assert!(
        !rust_status.success(),
        "Rust {kind} probe unexpectedly succeeded"
    );

    #[cfg(unix)]
    assert_eq!(
        rust_status.signal(),
        c_status.signal(),
        "different termination signals for {kind}: C={c_status:?}, Rust={rust_status:?}"
    );

    #[cfg(not(unix))]
    assert_eq!(
        rust_status.code(),
        c_status.code(),
        "different exit codes for {kind}: C={c_status:?}, Rust={rust_status:?}"
    );
}

#[test]
fn error_g1_null_image_matches_c_termination() {
    assert_matching_crash("null_image");
}

#[test]
fn error_g2_null_pixels_matches_c_termination() {
    assert_matching_crash("null_pixels");
}

#[test]
fn ffi_crash_probe() {
    let Ok(library) = env::var("DIFF_CRASH_LIBRARY") else {
        return;
    };
    let kind = env::var("DIFF_CRASH_KIND").expect("DIFF_CRASH_KIND");
    let api = match library.as_str() {
        "c" => Api::load(&c_library_path()),
        "rust" => Api::load(&rust_library_path()),
        _ => panic!("unknown crash-probe library: {library}"),
    };

    unsafe {
        match kind.as_str() {
            "null_image" => api.call(std::ptr::null_mut()),
            "null_pixels" => {
                let mut image = Image {
                    w: 1,
                    h: 1,
                    pix: std::ptr::null_mut(),
                };
                api.call(&mut image);
            }
            _ => panic!("unknown crash-probe kind: {kind}"),
        }
    }
    panic!("{library}/{kind} crash probe unexpectedly returned");
}
