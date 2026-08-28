use libloading::Library;
use std::ffi::{c_float, c_int};
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr;

type Colourblind = unsafe extern "C" fn(c_int, *mut c_float, *mut c_float, *mut c_float);

struct DifferentialLibraries {
    _c_library: Library,
    _rust_library: Library,
    c_colourblind: Colourblind,
    rust_colourblind: Colourblind,
}

impl DifferentialLibraries {
    fn load() -> Self {
        let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let c_path = library_path(
            "COLOURBLIND_C_LIB",
            crate_dir.join("../c_src/build/libharvest-work-UnWJLG.so"),
        );
        let rust_path = library_path(
            "COLOURBLIND_RUST_LIB",
            crate_dir.join("target/release/libcolourblind_lib.so"),
        );

        unsafe {
            let c_library = Library::new(&c_path)
                .unwrap_or_else(|error| panic!("failed to load {}: {error}", c_path.display()));
            let rust_library = Library::new(&rust_path)
                .unwrap_or_else(|error| panic!("failed to load {}: {error}", rust_path.display()));
            let c_colourblind = *c_library
                .get::<Colourblind>(b"colourblind\0")
                .expect("C library does not export colourblind");
            let rust_colourblind = *rust_library
                .get::<Colourblind>(b"colourblind\0")
                .expect("Rust library does not export colourblind");

            Self {
                _c_library: c_library,
                _rust_library: rust_library,
                c_colourblind,
                rust_colourblind,
            }
        }
    }
}

fn library_path(variable: &str, default: PathBuf) -> PathBuf {
    std::env::var_os(variable)
        .map(PathBuf::from)
        .unwrap_or(default)
}

#[derive(Clone, Copy, Debug)]
enum AliasShape {
    Distinct,
    RedGreen,
    RedBlue,
    GreenBlue,
    All,
}

impl AliasShape {
    fn indices(self) -> [usize; 3] {
        match self {
            Self::Distinct => [0, 1, 2],
            Self::RedGreen => [0, 0, 1],
            Self::RedBlue => [0, 1, 0],
            Self::GreenBlue => [0, 1, 1],
            Self::All => [0, 0, 0],
        }
    }
}

fn invoke(function: Colourblind, impairment: c_int, values: &mut [f32; 3], shape: AliasShape) {
    let [red, green, blue] = shape.indices();
    unsafe {
        function(
            impairment,
            values.as_mut_ptr().add(red),
            values.as_mut_ptr().add(green),
            values.as_mut_ptr().add(blue),
        );
    }
}

fn assert_configuration(impairment: c_int, shape: AliasShape) {
    const EDGE_CASES: [[u32; 3]; 12] = [
        [0, 0, 0],
        [0x8000_0000, 0, 0x8000_0000],
        [1, 0x007f_ffff, 0x0080_0000],
        [0x8000_0001, 0x807f_ffff, 0x8080_0000],
        [1.0f32.to_bits(), (-1.0f32).to_bits(), 0.5f32.to_bits()],
        [
            f32::MAX.to_bits(),
            (-f32::MAX).to_bits(),
            f32::MIN_POSITIVE.to_bits(),
        ],
        [f32::INFINITY.to_bits(), 1.0f32.to_bits(), 0],
        [f32::NEG_INFINITY.to_bits(), f32::INFINITY.to_bits(), 0],
        [0x7fc0_0000, 1.0f32.to_bits(), 2.0f32.to_bits()],
        [0x7fc1_2345, 0xffc5_4321, 0x7f80_0001],
        [0xffc0_0000, 0x7f80_0001, 0xff80_0001],
        [0x7fff_ffff, 0xffff_ffff, 0x7fa0_0000],
    ];

    let libraries = DifferentialLibraries::load();
    for bits in EDGE_CASES
        .into_iter()
        .chain(RandomTriples::new(0xd1ff_e2e3_4a5b_6c7d, 10_000))
    {
        let initial = bits.map(f32::from_bits);
        let mut c_values = initial;
        let mut rust_values = initial;

        invoke(libraries.c_colourblind, impairment, &mut c_values, shape);
        invoke(
            libraries.rust_colourblind,
            impairment,
            &mut rust_values,
            shape,
        );

        assert_eq!(
            c_values.map(f32::to_bits),
            rust_values.map(f32::to_bits),
            "mismatch for impairment {impairment}, shape {shape:?}, input {bits:08x?}"
        );
    }
}

struct RandomTriples {
    state: u64,
    remaining: usize,
}

impl RandomTriples {
    fn new(seed: u64, count: usize) -> Self {
        Self {
            state: seed,
            remaining: count,
        }
    }

    fn next_u32(&mut self) -> u32 {
        self.state = self.state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        (value ^ (value >> 31)) as u32
    }
}

impl Iterator for RandomTriples {
    type Item = [u32; 3];

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }
        self.remaining -= 1;
        Some([self.next_u32(), self.next_u32(), self.next_u32()])
    }
}

macro_rules! configuration_test {
    ($name:ident, $impairment:expr, $shape:expr) => {
        #[test]
        fn $name() {
            assert_configuration($impairment, $shape);
        }
    };
}

configuration_test!(config_01_protanopia_distinct, 0, AliasShape::Distinct);
configuration_test!(config_02_protanopia_red_green, 0, AliasShape::RedGreen);
configuration_test!(config_03_protanopia_red_blue, 0, AliasShape::RedBlue);
configuration_test!(config_04_protanopia_green_blue, 0, AliasShape::GreenBlue);
configuration_test!(config_05_protanopia_all, 0, AliasShape::All);
configuration_test!(config_06_deuteranopia_distinct, 1, AliasShape::Distinct);
configuration_test!(config_07_deuteranopia_red_green, 1, AliasShape::RedGreen);
configuration_test!(config_08_deuteranopia_red_blue, 1, AliasShape::RedBlue);
configuration_test!(config_09_deuteranopia_green_blue, 1, AliasShape::GreenBlue);
configuration_test!(config_10_deuteranopia_all, 1, AliasShape::All);
configuration_test!(config_11_tritanopia_distinct, 2, AliasShape::Distinct);
configuration_test!(config_12_tritanopia_red_green, 2, AliasShape::RedGreen);
configuration_test!(config_13_tritanopia_red_blue, 2, AliasShape::RedBlue);
configuration_test!(config_14_tritanopia_green_blue, 2, AliasShape::GreenBlue);
configuration_test!(config_15_tritanopia_all, 2, AliasShape::All);

#[test]
fn error_01_unsupported_impairments_are_exact_no_ops() {
    let libraries = DifferentialLibraries::load();
    let fixed_invalid = [c_int::MIN, -1, 3, 4, c_int::MAX];

    for impairment in fixed_invalid.into_iter().chain(
        RandomTriples::new(0xfeed_face_cafe_beef, 10_000)
            .map(|bits| bits[0] as c_int)
            .filter(|value| !matches!(value, 0..=2)),
    ) {
        let bits = [
            impairment as u32 ^ 0x7fc0_0000,
            impairment as u32,
            !(impairment as u32),
        ];
        let initial = bits.map(f32::from_bits);
        let mut c_values = initial;
        let mut rust_values = initial;

        invoke(
            libraries.c_colourblind,
            impairment,
            &mut c_values,
            AliasShape::Distinct,
        );
        invoke(
            libraries.rust_colourblind,
            impairment,
            &mut rust_values,
            AliasShape::Distinct,
        );

        assert_eq!(c_values.map(f32::to_bits), bits);
        assert_eq!(rust_values.map(f32::to_bits), bits);
    }

    for impairment in fixed_invalid {
        unsafe {
            (libraries.c_colourblind)(
                impairment,
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
            );
            (libraries.rust_colourblind)(
                impairment,
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
            );
        }
    }
}

#[test]
fn generic_boundary_valid_impairments_with_null_pointers_match_termination() {
    let executable = std::env::current_exe().expect("failed to locate differential test binary");

    for impairment in 0..=2 {
        for null_argument in 0..3 {
            let run_probe = |library: &str| {
                Command::new(&executable)
                    .arg("--exact")
                    .arg("null_pointer_probe_child")
                    .arg("--nocapture")
                    .env("NULL_PROBE_LIBRARY", library)
                    .env("NULL_PROBE_IMPAIRMENT", impairment.to_string())
                    .env("NULL_PROBE_ARGUMENT", null_argument.to_string())
                    .status()
                    .expect("failed to run null-pointer probe")
            };

            let c_status = run_probe("c");
            let rust_status = run_probe("rust");
            assert!(
                !c_status.success(),
                "C unexpectedly accepted mode {impairment} with argument {null_argument} null"
            );
            assert_eq!(
                c_status.signal(),
                rust_status.signal(),
                "termination mismatch for mode {impairment} with argument {null_argument} null"
            );
        }
    }
}

#[test]
fn null_pointer_probe_child() {
    let Ok(library) = std::env::var("NULL_PROBE_LIBRARY") else {
        return;
    };
    let impairment = std::env::var("NULL_PROBE_IMPAIRMENT")
        .expect("missing probe impairment")
        .parse::<c_int>()
        .expect("invalid probe impairment");
    let null_argument = std::env::var("NULL_PROBE_ARGUMENT")
        .expect("missing probe argument")
        .parse::<usize>()
        .expect("invalid probe argument");
    let libraries = DifferentialLibraries::load();
    let function = match library.as_str() {
        "c" => libraries.c_colourblind,
        "rust" => libraries.rust_colourblind,
        _ => panic!("invalid probe library {library}"),
    };
    let mut values: [f32; 3] = [0.25, 0.5, 0.75];
    let base = values.as_mut_ptr();
    let mut pointers = [base, unsafe { base.add(1) }, unsafe { base.add(2) }];
    pointers[null_argument] = ptr::null_mut();

    unsafe {
        function(impairment, pointers[0], pointers[1], pointers[2]);
    }
}
