use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::path::{Path, PathBuf};

type GetPredictFunc = unsafe extern "C" fn(c_int) -> c_int;

struct SharedLibraries {
    c: Library,
    rust: Library,
}

impl SharedLibraries {
    fn load() -> Self {
        let c_path = c_library_path();
        let rust_path = rust_library_path();

        // Loading these fixed build artifacts is the behavior under test.
        let c = unsafe { Library::new(&c_path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", c_path.display()));
        let rust = unsafe { Library::new(&rust_path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", rust_path.display()));

        Self { c, rust }
    }

    fn compare(&self, pfcn: c_int) -> c_int {
        let c_result = call_export(&self.c, pfcn);
        let rust_result = call_export(&self.rust, pfcn);
        assert_eq!(
            c_result.to_ne_bytes(),
            rust_result.to_ne_bytes(),
            "byte mismatch for pfcn={pfcn}: C={c_result}, Rust={rust_result}"
        );
        c_result
    }
}

fn c_library_path() -> PathBuf {
    let build_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/build");
    let mut candidates: Vec<PathBuf> = std::fs::read_dir(&build_dir)
        .unwrap_or_else(|error| {
            panic!(
                "failed to read C build directory {}: {error}; build it with CMake first",
                build_dir.display()
            )
        })
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().is_some_and(|extension| extension == "so"))
        .collect();
    candidates.sort();

    match candidates.as_slice() {
        [path] => path.clone(),
        [] => panic!(
            "no C shared library found in {}; build it with CMake first",
            build_dir.display()
        ),
        _ => panic!(
            "expected one C shared library in {}, found: {candidates:?}",
            build_dir.display()
        ),
    }
}

fn rust_library_path() -> PathBuf {
    let test_executable =
        std::env::current_exe().expect("failed to locate the integration test executable");
    let profile_dir = test_executable
        .parent()
        .and_then(Path::parent)
        .expect("unexpected Cargo integration test path");
    let target_dir = profile_dir
        .parent()
        .expect("missing Cargo target directory");
    let candidates = [
        profile_dir.join("libget_predict_func_lib.so"),
        target_dir
            .join("release")
            .join("libget_predict_func_lib.so"),
    ];

    candidates
        .into_iter()
        .find(|candidate| candidate.is_file())
        .unwrap_or_else(|| {
            panic!("Rust cdylib is missing; run `cargo build --release` before `cargo test`")
        })
}

fn call_export(library: &Library, pfcn: c_int) -> c_int {
    // Resolve on every call so this exercises the exported ABI symbol itself.
    let function: Symbol<'_, GetPredictFunc> =
        unsafe { library.get(b"get_predict_func\0") }.expect("missing get_predict_func export");
    unsafe { function(pfcn) }
}

fn next_random(state: &mut u64) -> u32 {
    // Fixed-seed xorshift64* keeps property coverage reproducible.
    *state ^= *state >> 12;
    *state ^= *state << 25;
    *state ^= *state >> 27;
    state.wrapping_mul(0x2545_f491_4f6c_dd1d) as u32
}

#[test]
fn specialized_predictor_configurations_match() {
    let libraries = SharedLibraries::load();
    let mut seed = 0x6a09_e667_f3bc_c909;

    // Each explicit C switch case is a singleton input configuration. Exercise
    // every one repeatedly while randomizing invocation order.
    for _ in 0..512 {
        let mut cases: Vec<c_int> = (0..=11).collect();
        for index in (1..cases.len()).rev() {
            let swap_with = next_random(&mut seed) as usize % (index + 1);
            cases.swap(index, swap_with);
        }

        for pfcn in cases {
            assert_eq!(libraries.compare(pfcn), 1, "unexpected C result");
        }
    }
}

#[test]
fn generic_default_configuration_matches_randomized_inputs() {
    let libraries = SharedLibraries::load();

    for pfcn in [
        c_int::MIN,
        c_int::MIN + 1,
        -1,
        12,
        13,
        c_int::MAX - 1,
        c_int::MAX,
    ] {
        assert_eq!(libraries.compare(pfcn), 0, "unexpected C result");
    }

    let mut seed = 0xbb67_ae85_84ca_a73b;
    let mut compared = 0;
    while compared < 100_000 {
        let pfcn = next_random(&mut seed) as c_int;
        if (0..=11).contains(&pfcn) {
            continue;
        }
        assert_eq!(libraries.compare(pfcn), 0, "unexpected C result");
        compared += 1;
    }
}
