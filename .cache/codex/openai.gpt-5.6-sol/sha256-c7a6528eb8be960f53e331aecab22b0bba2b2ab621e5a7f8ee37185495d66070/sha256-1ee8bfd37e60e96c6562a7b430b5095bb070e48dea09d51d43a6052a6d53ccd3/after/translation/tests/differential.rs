use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr;
use std::sync::OnceLock;

const CASES_PER_ROW: usize = 128;

#[repr(C)]
#[derive(Clone, Copy)]
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

#[derive(Clone, Copy)]
enum WidthShape {
    Zero,
    One,
    Many,
}

#[derive(Clone, Copy)]
enum HeightShape {
    Zero,
    One,
    PositiveEven,
    OddAtLeastThree,
}

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

    fn range_inclusive(&mut self, min: usize, max: usize) -> usize {
        min + (self.next_u64() as usize % (max - min + 1))
    }

    fn pixel(&mut self) -> CpPixel {
        let bytes = self.next_u64().to_ne_bytes();
        CpPixel {
            r: bytes[0],
            g: bytes[1],
            b: bytes[2],
            a: bytes[3],
        }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    let build_dir = manifest_dir().join("../c_src/build");
    let mut libraries: Vec<_> = std::fs::read_dir(&build_dir)
        .unwrap_or_else(|error| {
            panic!(
                "failed to read C build directory {}: {error}",
                build_dir.display()
            )
        })
        .map(|entry| entry.expect("failed to read C build entry").path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "so"))
        .collect();
    libraries.sort();
    assert_eq!(
        libraries.len(),
        1,
        "expected exactly one C shared object in {}",
        build_dir.display()
    );
    libraries.pop().unwrap()
}

fn rust_library_path() -> PathBuf {
    static BUILD: OnceLock<()> = OnceLock::new();
    BUILD.get_or_init(|| {
        let status = Command::new(env!("CARGO"))
            .args(["build", "--release", "--lib"])
            .current_dir(manifest_dir())
            .status()
            .expect("failed to launch cargo build for the Rust cdylib");
        assert!(status.success(), "cargo build for the Rust cdylib failed");
    });

    let target_dir = if let Some(configured) = std::env::var_os("CARGO_TARGET_DIR") {
        let configured = PathBuf::from(configured);
        if configured.is_absolute() {
            configured
        } else {
            manifest_dir().join(configured)
        }
    } else {
        manifest_dir().join("target")
    };
    target_dir.join("release").join("libflip_horizontal_lib.so")
}

fn load_library(path: &Path) -> Library {
    assert!(
        path.is_file(),
        "shared object not found: {}",
        path.display()
    );
    unsafe { Library::new(path) }
        .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()))
}

fn dimensions(rng: &mut Rng, width: WidthShape, height: HeightShape) -> (usize, usize) {
    let width = match width {
        WidthShape::Zero => 0,
        WidthShape::One => 1,
        WidthShape::Many => rng.range_inclusive(2, 64),
    };
    let height = match height {
        HeightShape::Zero => 0,
        HeightShape::One => 1,
        HeightShape::PositiveEven => rng.range_inclusive(1, 32) * 2,
        HeightShape::OddAtLeastThree => rng.range_inclusive(1, 32) * 2 + 1,
    };
    (width, height)
}

fn bytes(pixels: &[CpPixel]) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(pixels.as_ptr().cast::<u8>(), std::mem::size_of_val(pixels))
    }
}

fn run_configuration_row(row: u64, width_shape: WidthShape, height_shape: HeightShape) {
    assert_eq!(std::mem::size_of::<CpPixel>(), 4);
    let c_library = load_library(&c_library_path());
    let rust_library = load_library(&rust_library_path());
    let c_flip: Symbol<FlipHorizontal> =
        unsafe { c_library.get(b"flip_horizontal\0") }.expect("C symbol missing");
    let rust_flip: Symbol<FlipHorizontal> =
        unsafe { rust_library.get(b"flip_horizontal\0") }.expect("Rust symbol missing");
    let mut rng = Rng::new(0x6a09_e667_f3bc_c909 ^ row);

    for case in 0..CASES_PER_ROW {
        let (width, height) = dimensions(&mut rng, width_shape, height_shape);
        let logical_len = width.checked_mul(height).unwrap();
        // Keep valid backing storage even when one dimension is zero so the C
        // implementation can perform its zero-offset pointer arithmetic.
        let backing_len = logical_len.max(8);
        let original: Vec<_> = (0..backing_len).map(|_| rng.pixel()).collect();
        let mut c_pixels = original.clone();
        let mut rust_pixels = original;
        let mut c_image = CpImage {
            w: width as c_int,
            h: height as c_int,
            pix: c_pixels.as_mut_ptr(),
        };
        let mut rust_image = CpImage {
            w: width as c_int,
            h: height as c_int,
            pix: rust_pixels.as_mut_ptr(),
        };

        unsafe {
            c_flip(&mut c_image);
            rust_flip(&mut rust_image);
        }

        assert_eq!(
            bytes(&c_pixels),
            bytes(&rust_pixels),
            "configuration row {row}, randomized case {case}, {width}x{height}"
        );
    }
}

macro_rules! configuration_test {
    ($name:ident, $row:literal, $width:expr, $height:expr) => {
        #[test]
        fn $name() {
            run_configuration_row($row, $width, $height);
        }
    };
}

configuration_test!(
    config_01_width_zero_height_zero,
    1,
    WidthShape::Zero,
    HeightShape::Zero
);
configuration_test!(
    config_02_width_zero_height_one,
    2,
    WidthShape::Zero,
    HeightShape::One
);
configuration_test!(
    config_03_width_zero_height_even,
    3,
    WidthShape::Zero,
    HeightShape::PositiveEven
);
configuration_test!(
    config_04_width_zero_height_odd,
    4,
    WidthShape::Zero,
    HeightShape::OddAtLeastThree
);
configuration_test!(
    config_05_width_one_height_zero,
    5,
    WidthShape::One,
    HeightShape::Zero
);
configuration_test!(
    config_06_width_one_height_one,
    6,
    WidthShape::One,
    HeightShape::One
);
configuration_test!(
    config_07_width_one_height_even,
    7,
    WidthShape::One,
    HeightShape::PositiveEven
);
configuration_test!(
    config_08_width_one_height_odd,
    8,
    WidthShape::One,
    HeightShape::OddAtLeastThree
);
configuration_test!(
    config_09_width_many_height_zero,
    9,
    WidthShape::Many,
    HeightShape::Zero
);
configuration_test!(
    config_10_width_many_height_one,
    10,
    WidthShape::Many,
    HeightShape::One
);
configuration_test!(
    config_11_width_many_height_even,
    11,
    WidthShape::Many,
    HeightShape::PositiveEven
);
configuration_test!(
    config_12_width_many_height_odd,
    12,
    WidthShape::Many,
    HeightShape::OddAtLeastThree
);

#[test]
fn accepted_boundary_inputs_match() {
    let c_library = load_library(&c_library_path());
    let rust_library = load_library(&rust_library_path());
    let c_flip: Symbol<FlipHorizontal> =
        unsafe { c_library.get(b"flip_horizontal\0") }.expect("C symbol missing");
    let rust_flip: Symbol<FlipHorizontal> =
        unsafe { rust_library.get(b"flip_horizontal\0") }.expect("Rust symbol missing");

    // These inputs all suppress the C loops, so a null pixel pointer is read
    // from the image structure but is never dereferenced or offset.
    let dimensions = [
        (0, 0),
        (c_int::MAX, 0),
        (c_int::MIN, 0),
        (-1, -1),
        (0, c_int::MIN),
        (c_int::MAX, 1),
    ];
    for (width, height) in dimensions {
        let mut c_image = CpImage {
            w: width,
            h: height,
            pix: ptr::null_mut(),
        };
        let mut rust_image = CpImage {
            w: width,
            h: height,
            pix: ptr::null_mut(),
        };
        unsafe {
            c_flip(&mut c_image);
            rust_flip(&mut rust_image);
        }
        assert_eq!((c_image.w, c_image.h), (rust_image.w, rust_image.h));
        assert_eq!(c_image.pix, rust_image.pix);
    }
}
