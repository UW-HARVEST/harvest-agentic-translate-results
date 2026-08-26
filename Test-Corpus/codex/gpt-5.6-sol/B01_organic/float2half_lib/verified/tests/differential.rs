use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};

type Float2Half = unsafe extern "C" fn(f32) -> u16;

struct Implementations {
    _c_library: Library,
    _rust_library: Library,
    c_float2half: Float2Half,
    rust_float2half: Float2Half,
}

impl Implementations {
    fn load() -> Self {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let c_path = manifest_dir.join("c_src/build/libtranslated_rust.so");
        let rust_path = rust_library_path(manifest_dir);

        assert!(
            c_path.is_file(),
            "C shared library does not exist: {}",
            c_path.display()
        );
        assert!(
            rust_path.is_file(),
            "Rust shared library does not exist: {}",
            rust_path.display()
        );

        unsafe {
            let c_library = Library::new(&c_path)
                .unwrap_or_else(|error| panic!("failed to load {}: {error}", c_path.display()));
            let rust_library = Library::new(&rust_path)
                .unwrap_or_else(|error| panic!("failed to load {}: {error}", rust_path.display()));

            let c_float2half = load_float2half(&c_library, &c_path);
            let rust_float2half = load_float2half(&rust_library, &rust_path);

            Self {
                _c_library: c_library,
                _rust_library: rust_library,
                c_float2half,
                rust_float2half,
            }
        }
    }

    fn compare_bits(&self, input_bits: u32) {
        let input = f32::from_bits(input_bits);
        let c_output = unsafe { (self.c_float2half)(input) };
        let rust_output = unsafe { (self.rust_float2half)(input) };

        assert_eq!(
            rust_output.to_ne_bytes(),
            c_output.to_ne_bytes(),
            "mismatch for input bits {input_bits:#010x}: C={c_output:#06x}, Rust={rust_output:#06x}"
        );
    }
}

unsafe fn load_float2half(library: &Library, path: &Path) -> Float2Half {
    let symbol: Symbol<Float2Half> =
        unsafe { library.get(b"float2half\0") }.unwrap_or_else(|error| {
            panic!(
                "failed to resolve float2half in {}: {error}",
                path.display()
            )
        });
    *symbol
}

fn rust_library_path(manifest_dir: &Path) -> PathBuf {
    let profile = std::env::var("PROFILE").unwrap_or_else(|_| "debug".to_owned());
    for candidate_profile in [&profile[..], "release"] {
        let direct_path = manifest_dir
            .join("target")
            .join(candidate_profile)
            .join("libfloat2half_lib.so");
        if direct_path.is_file() {
            return direct_path;
        }
    }

    let deps_dir = manifest_dir.join("target").join(profile).join("deps");
    let candidates: Vec<_> = std::fs::read_dir(&deps_dir)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", deps_dir.display()))
        .map(|entry| entry.expect("failed to read target directory entry").path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("libfloat2half_lib") && name.ends_with(".so"))
        })
        .collect();

    assert_eq!(
        candidates.len(),
        1,
        "expected exactly one Rust shared library in {}, found {candidates:?}",
        deps_dir.display()
    );
    candidates.into_iter().next().unwrap()
}

fn compare_regime(sign: u32, minimum_exponent: u32, maximum_exponent: u32, seed: u64) {
    let implementations = Implementations::load();
    let boundary_mantissas = [0, 1, 0x003f_ffff, 0x007f_fffe, 0x007f_ffff];

    for exponent in minimum_exponent..=maximum_exponent {
        for mantissa in boundary_mantissas {
            implementations.compare_bits(sign | (exponent << 23) | mantissa);
        }
    }

    let mut state = seed;
    for _ in 0..4096 {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let exponent =
            minimum_exponent + ((state >> 32) as u32 % (maximum_exponent - minimum_exponent + 1));
        let mantissa = (state as u32) & 0x007f_ffff;
        implementations.compare_bits(sign | (exponent << 23) | mantissa);
    }
}

#[test]
fn positive_exponents_0_through_102() {
    compare_regime(0, 0, 102, 0x6ab3_23f4_7d91_0051);
}

#[test]
fn positive_exponents_103_through_112() {
    compare_regime(0, 103, 112, 0xd168_37a2_519c_0052);
}

#[test]
fn positive_exponents_113_through_142() {
    compare_regime(0, 113, 142, 0x2c4d_774e_b069_0053);
}

#[test]
fn positive_exponents_143_through_254() {
    compare_regime(0, 143, 254, 0xaf87_e5b9_76dc_0054);
}

#[test]
fn positive_exponent_255() {
    compare_regime(0, 255, 255, 0x3906_25a8_c417_0055);
}

#[test]
fn negative_exponents_0_through_102() {
    compare_regime(0x8000_0000, 0, 102, 0x7ec9_4a36_2db5_0056);
}

#[test]
fn negative_exponents_103_through_112() {
    compare_regime(0x8000_0000, 103, 112, 0xc2f1_695b_84e3_0057);
}

#[test]
fn negative_exponents_113_through_142() {
    compare_regime(0x8000_0000, 113, 142, 0x183d_be72_95a4_0058);
}

#[test]
fn negative_exponents_143_through_254() {
    compare_regime(0x8000_0000, 143, 254, 0xe495_32c1_067f_0059);
}

#[test]
fn negative_exponent_255() {
    compare_regime(0x8000_0000, 255, 255, 0x54b7_0f9c_d328_005a);
}
