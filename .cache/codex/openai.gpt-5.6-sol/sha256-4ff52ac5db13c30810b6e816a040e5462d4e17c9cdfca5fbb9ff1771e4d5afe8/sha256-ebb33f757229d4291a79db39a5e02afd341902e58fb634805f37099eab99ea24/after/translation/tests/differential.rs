use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::path::{Path, PathBuf};

type CallPredict = unsafe extern "C" fn(c_int) -> c_int;

struct LoadedLibraries {
    _c: Library,
    _rust: Library,
    c_call_predict: CallPredict,
    rust_call_predict: CallPredict,
}

impl LoadedLibraries {
    fn load() -> Self {
        let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_path = crate_root.join("../c_src/build/libharvest-work-uQuYXX.so");
        let rust_path = rust_library_path(&crate_root);

        assert!(
            c_path.is_file(),
            "C shared library is missing at {}; build it with CMake first",
            c_path.display()
        );
        assert!(
            rust_path.is_file(),
            "Rust shared library is missing at {}; build it with cargo build first",
            rust_path.display()
        );

        unsafe {
            let c = Library::new(&c_path)
                .unwrap_or_else(|error| panic!("failed to load {}: {error}", c_path.display()));
            let rust = Library::new(&rust_path)
                .unwrap_or_else(|error| panic!("failed to load {}: {error}", rust_path.display()));

            let c_symbol: Symbol<CallPredict> = c
                .get(b"call_predict\0")
                .unwrap_or_else(|error| panic!("C call_predict export is missing: {error}"));
            let rust_symbol: Symbol<CallPredict> = rust
                .get(b"call_predict\0")
                .unwrap_or_else(|error| panic!("Rust call_predict export is missing: {error}"));
            let c_call_predict = *c_symbol;
            let rust_call_predict = *rust_symbol;

            Self {
                _c: c,
                _rust: rust,
                c_call_predict,
                rust_call_predict,
            }
        }
    }

    fn compare(&self, pfcn: c_int) -> (c_int, c_int) {
        unsafe { ((self.c_call_predict)(pfcn), (self.rust_call_predict)(pfcn)) }
    }
}

fn rust_library_path(crate_root: &Path) -> PathBuf {
    let test_executable =
        std::env::current_exe().expect("failed to locate the integration test executable");
    let profile_dir = test_executable
        .parent()
        .and_then(Path::parent)
        .expect("integration test executable was not under target/<profile>/deps");
    let profile_library = profile_dir.join("libcall_predict_lib.so");
    if profile_library.is_file() {
        return profile_library;
    }

    let release_library = crate_root.join("target/release/libcall_predict_lib.so");
    if release_library.is_file() {
        return release_library;
    }

    profile_library
}

fn next_u64(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

#[test]
fn all_valid_configuration_rows_match_via_shared_objects() {
    let libraries = LoadedLibraries::load();

    for pfcn in 0..=11 {
        let (c_result, rust_result) = libraries.compare(pfcn);
        assert_eq!(c_result, 1, "C result for valid pfcn={pfcn}");
        assert_eq!(
            rust_result,
            c_result,
            "shared objects diverged for CONFIGS.md row {} (pfcn={pfcn})",
            pfcn + 1
        );
    }

    let mut seed = 0x4d59_5df4_d0f3_3173_u64;
    let mut hits = [0_u32; 12];
    for _ in 0..12_000 {
        let pfcn = (next_u64(&mut seed) % 12) as c_int;
        hits[pfcn as usize] += 1;
        let (c_result, rust_result) = libraries.compare(pfcn);
        assert_eq!(
            rust_result, c_result,
            "randomized valid call diverged for pfcn={pfcn}"
        );
    }

    assert!(
        hits.iter().all(|count| *count >= 900),
        "fixed-seed run did not exercise every configuration many times: {hits:?}"
    );
}

#[test]
fn unsupported_selectors_match_exact_rejection_value() {
    let libraries = LoadedLibraries::load();
    let boundaries = [
        c_int::MIN,
        c_int::MIN + 1,
        -65_536,
        -13,
        -2,
        -1,
        12,
        13,
        65_536,
        c_int::MAX - 1,
        c_int::MAX,
    ];

    for pfcn in boundaries {
        let (c_result, rust_result) = libraries.compare(pfcn);
        assert_eq!(c_result, 0, "C rejection value for pfcn={pfcn}");
        assert_eq!(
            rust_result, c_result,
            "shared objects diverged for ERRORS.md row 1 at pfcn={pfcn}"
        );
    }

    let mut seed = 0xa076_1d64_78bd_642f_u64;
    for _ in 0..20_000 {
        let mut pfcn = next_u64(&mut seed) as c_int;
        if (0..=11).contains(&pfcn) {
            pfcn = pfcn.wrapping_add(12);
        }
        let (c_result, rust_result) = libraries.compare(pfcn);
        assert_eq!(c_result, 0, "C rejection value for pfcn={pfcn}");
        assert_eq!(
            rust_result, c_result,
            "randomized invalid call diverged for pfcn={pfcn}"
        );
    }
}
