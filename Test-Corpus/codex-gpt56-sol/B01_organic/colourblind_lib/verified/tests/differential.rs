use libloading::Library;
use std::ffi::c_int;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

type Colourblind = unsafe extern "C" fn(c_int, *mut f32, *mut f32, *mut f32);

struct Libraries {
    _c_library: Library,
    _rust_library: Library,
    c_colourblind: Colourblind,
    rust_colourblind: Colourblind,
}

impl Libraries {
    fn load() -> Self {
        let c_library =
            unsafe { Library::new(c_library_path()) }.expect("load C reference shared library");
        let rust_library =
            unsafe { Library::new(rust_library_path()) }.expect("load Rust shared library");
        let c_colourblind = unsafe {
            *c_library
                .get::<Colourblind>(b"colourblind\0")
                .expect("resolve C colourblind")
        };
        let rust_colourblind = unsafe {
            *rust_library
                .get::<Colourblind>(b"colourblind\0")
                .expect("resolve Rust colourblind")
        };

        Self {
            _c_library: c_library,
            _rust_library: rust_library,
            c_colourblind,
            rust_colourblind,
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum PointerShape {
    Distinct,
    RedGreen,
    RedBlue,
    GreenBlue,
    All,
}

impl PointerShape {
    fn indices(self) -> [usize; 3] {
        match self {
            Self::Distinct => [0, 1, 2],
            Self::RedGreen => [0, 0, 2],
            Self::RedBlue => [0, 1, 0],
            Self::GreenBlue => [0, 1, 1],
            Self::All => [0, 0, 0],
        }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    manifest_dir()
        .join("c_src")
        .join("build")
        .join("libtranslated_rust.so")
}

fn rust_library_path() -> PathBuf {
    let target = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| manifest_dir().join("target"));
    let target = if target.is_absolute() {
        target
    } else {
        manifest_dir().join(target)
    };
    target.join("debug").join("libcolourblind_lib.so")
}

fn invoke(
    function: Colourblind,
    impairment: c_int,
    shape: PointerShape,
    bits: [u32; 3],
) -> [u32; 3] {
    let mut slots = bits.map(f32::from_bits);
    let indices = shape.indices();
    let base = slots.as_mut_ptr();

    unsafe {
        function(
            impairment,
            base.add(indices[0]),
            base.add(indices[1]),
            base.add(indices[2]),
        );
    }

    slots.map(f32::to_bits)
}

fn random_bits(state: &mut u64) -> u32 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    (*state >> 16) as u32
}

fn verify_configuration(impairment: c_int, shape: PointerShape) {
    const EDGES: [u32; 20] = [
        0x0000_0000,
        0x8000_0000,
        0x0000_0001,
        0x8000_0001,
        0x007f_ffff,
        0x807f_ffff,
        0x0080_0000,
        0x8080_0000,
        0x3f80_0000,
        0xbf80_0000,
        0x7f7f_ffff,
        0xff7f_ffff,
        0x7f80_0000,
        0xff80_0000,
        0x7fc0_0000,
        0xffc0_0000,
        0x7fc1_2345,
        0xffc1_2345,
        0x7f81_2345,
        0xff81_2345,
    ];

    let libraries = Libraries::load();
    for index in 0..EDGES.len() {
        let bits = [
            EDGES[index],
            EDGES[(index * 7 + 3) % EDGES.len()],
            EDGES[(index * 13 + 5) % EDGES.len()],
        ];
        assert_outputs_equal(&libraries, impairment, shape, bits, index);
    }

    let mut state = 0xd1ff_3a5e_9b07_4c21_u64
        ^ (impairment as u32 as u64).wrapping_mul(0x9e37_79b9)
        ^ (shape.indices()[1] as u64) << 40
        ^ (shape.indices()[2] as u64) << 48;
    for index in 0..4096 {
        let bits = [
            random_bits(&mut state),
            random_bits(&mut state),
            random_bits(&mut state),
        ];
        assert_outputs_equal(&libraries, impairment, shape, bits, EDGES.len() + index);
    }
}

fn assert_outputs_equal(
    libraries: &Libraries,
    impairment: c_int,
    shape: PointerShape,
    bits: [u32; 3],
    case: usize,
) {
    let c_output = invoke(libraries.c_colourblind, impairment, shape, bits);
    let rust_output = invoke(libraries.rust_colourblind, impairment, shape, bits);
    assert_eq!(
        rust_output, c_output,
        "mode={impairment}, shape={shape:?}, case={case}, input={bits:08x?}"
    );
}

macro_rules! configuration_test {
    ($name:ident, $impairment:expr, $shape:expr) => {
        #[test]
        fn $name() {
            verify_configuration($impairment, $shape);
        }
    };
}

configuration_test!(config_01_protanopia_distinct, 0, PointerShape::Distinct);
configuration_test!(config_02_protanopia_red_green, 0, PointerShape::RedGreen);
configuration_test!(config_03_protanopia_red_blue, 0, PointerShape::RedBlue);
configuration_test!(config_04_protanopia_green_blue, 0, PointerShape::GreenBlue);
configuration_test!(config_05_protanopia_all, 0, PointerShape::All);
configuration_test!(config_06_deuteranopia_distinct, 1, PointerShape::Distinct);
configuration_test!(config_07_deuteranopia_red_green, 1, PointerShape::RedGreen);
configuration_test!(config_08_deuteranopia_red_blue, 1, PointerShape::RedBlue);
configuration_test!(
    config_09_deuteranopia_green_blue,
    1,
    PointerShape::GreenBlue
);
configuration_test!(config_10_deuteranopia_all, 1, PointerShape::All);
configuration_test!(config_11_tritanopia_distinct, 2, PointerShape::Distinct);
configuration_test!(config_12_tritanopia_red_green, 2, PointerShape::RedGreen);
configuration_test!(config_13_tritanopia_red_blue, 2, PointerShape::RedBlue);
configuration_test!(config_14_tritanopia_green_blue, 2, PointerShape::GreenBlue);
configuration_test!(config_15_tritanopia_all, 2, PointerShape::All);

#[test]
fn boundary_out_of_range_impairments_are_no_ops() {
    let libraries = Libraries::load();
    let mut state = 0xa632_f9d4_718c_05eb_u64;

    for impairment in [-1, 3, c_int::MIN, c_int::MAX] {
        for shape in [
            PointerShape::Distinct,
            PointerShape::RedGreen,
            PointerShape::RedBlue,
            PointerShape::GreenBlue,
            PointerShape::All,
        ] {
            for _ in 0..1024 {
                let bits = [
                    random_bits(&mut state),
                    random_bits(&mut state),
                    random_bits(&mut state),
                ];
                let c_output = invoke(libraries.c_colourblind, impairment, shape, bits);
                let rust_output = invoke(libraries.rust_colourblind, impairment, shape, bits);
                assert_eq!(c_output, bits);
                assert_eq!(rust_output, c_output);
            }
        }
    }
}

#[test]
fn boundary_invalid_impairments_accept_null_pointers() {
    let libraries = Libraries::load();
    for impairment in [-1, 3, c_int::MIN, c_int::MAX] {
        unsafe {
            (libraries.c_colourblind)(
                impairment,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );
            (libraries.rust_colourblind)(
                impairment,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );
        }
    }
}

#[cfg(unix)]
fn child_signal(status: ExitStatus) -> Option<i32> {
    use std::os::unix::process::ExitStatusExt;
    status.signal()
}

#[cfg(unix)]
fn run_null_child(library: &Path, impairment: c_int, null_slot: usize) -> ExitStatus {
    Command::new(std::env::current_exe().expect("current test executable"))
        .arg("--exact")
        .arg("boundary_null_pointer_child")
        .arg("--nocapture")
        .env("COLOURBLIND_NULL_CHILD_LIBRARY", library)
        .env("COLOURBLIND_NULL_CHILD_MODE", impairment.to_string())
        .env("COLOURBLIND_NULL_CHILD_SLOT", null_slot.to_string())
        .status()
        .expect("run null-pointer child")
}

#[test]
#[cfg(unix)]
fn boundary_valid_impairments_have_null_pointer_crash_parity() {
    for impairment in 0..=2 {
        for null_slot in 0..3 {
            let c_status = run_null_child(&c_library_path(), impairment, null_slot);
            let rust_status = run_null_child(&rust_library_path(), impairment, null_slot);
            assert_eq!(
                child_signal(rust_status),
                child_signal(c_status),
                "mode={impairment}, null pointer slot={null_slot}"
            );
            assert_eq!(child_signal(c_status), Some(11));
        }
    }
}

#[test]
#[cfg(unix)]
fn boundary_null_pointer_child() {
    let Some(library_path) = std::env::var_os("COLOURBLIND_NULL_CHILD_LIBRARY") else {
        return;
    };
    let impairment = std::env::var("COLOURBLIND_NULL_CHILD_MODE")
        .expect("child mode")
        .parse::<c_int>()
        .expect("integer child mode");
    let null_slot = std::env::var("COLOURBLIND_NULL_CHILD_SLOT")
        .expect("child null slot")
        .parse::<usize>()
        .expect("integer child null slot");

    let library = unsafe { Library::new(library_path) }.expect("load child shared library");
    let colourblind = unsafe {
        *library
            .get::<Colourblind>(b"colourblind\0")
            .expect("resolve child colourblind")
    };
    let mut values = [1.0_f32, 2.0, 3.0];
    let mut pointers = [
        &mut values[0] as *mut f32,
        &mut values[1] as *mut f32,
        &mut values[2] as *mut f32,
    ];
    pointers[null_slot] = std::ptr::null_mut();

    unsafe {
        colourblind(impairment, pointers[0], pointers[1], pointers[2]);
    }
    panic!("valid-mode null pointer unexpectedly returned");
}
