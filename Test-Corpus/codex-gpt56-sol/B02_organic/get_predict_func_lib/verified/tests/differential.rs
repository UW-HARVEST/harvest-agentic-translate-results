use libloading::Library;
use std::ffi::c_int;
use std::path::{Path, PathBuf};

type GetPredictFunc = unsafe extern "C" fn(c_int) -> c_int;

struct Apis {
    _c_library: Library,
    _rust_library: Library,
    c_get_predict_func: GetPredictFunc,
    rust_get_predict_func: GetPredictFunc,
}

impl Apis {
    fn load() -> Self {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_path = manifest_dir.join("c_src/build/libtranslated_rust.so");
        let rust_path = rust_library_path(&manifest_dir);

        assert!(
            c_path.is_file(),
            "C shared library is missing at {}; build it with CMake first",
            c_path.display()
        );
        assert!(
            rust_path.is_file(),
            "Rust shared library is missing at {}",
            rust_path.display()
        );

        unsafe {
            let c_library = Library::new(&c_path)
                .unwrap_or_else(|error| panic!("failed to load {}: {error}", c_path.display()));
            let rust_library = Library::new(&rust_path)
                .unwrap_or_else(|error| panic!("failed to load {}: {error}", rust_path.display()));

            let c_get_predict_func = *c_library
                .get::<GetPredictFunc>(b"get_predict_func\0")
                .expect("C library does not export get_predict_func");
            let rust_get_predict_func = *rust_library
                .get::<GetPredictFunc>(b"get_predict_func\0")
                .expect("Rust library does not export get_predict_func");

            Self {
                _c_library: c_library,
                _rust_library: rust_library,
                c_get_predict_func,
                rust_get_predict_func,
            }
        }
    }

    fn compare(&self, input: c_int, expected: c_int) {
        unsafe {
            let c_result = (self.c_get_predict_func)(input);
            let rust_result = (self.rust_get_predict_func)(input);

            assert_eq!(
                c_result.to_ne_bytes(),
                rust_result.to_ne_bytes(),
                "ABI result bytes differ for pfcn={input}: C={c_result}, Rust={rust_result}"
            );
            assert_eq!(
                c_result, expected,
                "C ground truth returned an unexpected result for pfcn={input}"
            );
        }
    }
}

fn rust_library_path(manifest_dir: &Path) -> PathBuf {
    let target_dir = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .map(|path| {
            if path.is_absolute() {
                path
            } else {
                manifest_dir.join(path)
            }
        })
        .unwrap_or_else(|| manifest_dir.join("target"));

    let deps_library = target_dir.join("debug/deps").join(format!(
        "libget_predict_func_lib.{}",
        std::env::consts::DLL_EXTENSION
    ));
    if deps_library.is_file() {
        deps_library
    } else {
        target_dir.join("debug").join(format!(
            "libget_predict_func_lib.{}",
            std::env::consts::DLL_EXTENSION
        ))
    }
}

macro_rules! exact_configuration_test {
    ($name:ident, $pfcn:expr) => {
        #[test]
        fn $name() {
            Apis::load().compare($pfcn, 1);
        }
    };
}

exact_configuration_test!(config_01_pfcn_0, 0);
exact_configuration_test!(config_02_pfcn_1, 1);
exact_configuration_test!(config_03_pfcn_2, 2);
exact_configuration_test!(config_04_pfcn_3, 3);
exact_configuration_test!(config_05_pfcn_4, 4);
exact_configuration_test!(config_06_pfcn_5, 5);
exact_configuration_test!(config_07_pfcn_6, 6);
exact_configuration_test!(config_08_pfcn_7, 7);
exact_configuration_test!(config_09_pfcn_8, 8);
exact_configuration_test!(config_10_pfcn_9, 9);
exact_configuration_test!(config_11_pfcn_10, 10);
exact_configuration_test!(config_12_pfcn_11, 11);

#[test]
fn config_13_default_integer_class() {
    let apis = Apis::load();

    for input in [c_int::MIN, -1, 12, c_int::MAX] {
        apis.compare(input, 0);
    }

    // Fixed-seed LCG makes broad default-arm coverage reproducible.
    let mut state = 0x8d26_4f13_a9bc_0571_u64;
    for _ in 0..10_000 {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let mut input = (state >> 32) as c_int;
        if (0..=11).contains(&input) {
            input = input.wrapping_add(12);
        }
        apis.compare(input, 0);
    }
}
