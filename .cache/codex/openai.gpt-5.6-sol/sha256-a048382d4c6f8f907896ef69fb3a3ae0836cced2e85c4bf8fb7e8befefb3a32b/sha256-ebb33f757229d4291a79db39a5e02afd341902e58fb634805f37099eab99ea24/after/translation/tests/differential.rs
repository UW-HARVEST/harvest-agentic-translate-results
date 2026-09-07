use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::path::{Path, PathBuf};

type GetPredictFunc = unsafe extern "C" fn(c_int) -> c_int;

fn c_library_path() -> PathBuf {
    let build_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/build");
    let mut libraries = std::fs::read_dir(&build_dir)
        .unwrap_or_else(|error| {
            panic!(
                "failed to read C build directory {}: {error}",
                build_dir.display()
            )
        })
        .map(|entry| entry.expect("failed to inspect C build entry").path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "so"))
        .collect::<Vec<_>>();
    libraries.sort();
    assert_eq!(
        libraries.len(),
        1,
        "expected exactly one C shared library in {}",
        build_dir.display()
    );
    libraries.remove(0)
}

fn rust_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("target/release/libget_predict_func_lib.so")
}

fn with_functions(test: impl FnOnce(GetPredictFunc, GetPredictFunc)) {
    let c_path = c_library_path();
    let rust_path = rust_library_path();

    unsafe {
        let c_library = Library::new(&c_path)
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", c_path.display()));
        let rust_library = Library::new(&rust_path)
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", rust_path.display()));
        let c_symbol: Symbol<GetPredictFunc> = c_library
            .get(b"get_predict_func\0")
            .expect("C library does not export get_predict_func");
        let rust_symbol: Symbol<GetPredictFunc> = rust_library
            .get(b"get_predict_func\0")
            .expect("Rust library does not export get_predict_func");
        test(*c_symbol, *rust_symbol);
    }
}

fn compare_call(
    c_function: GetPredictFunc,
    rust_function: GetPredictFunc,
    pfcn: c_int,
) -> (c_int, c_int) {
    unsafe {
        let c_result = c_function(pfcn);
        let rust_result = rust_function(pfcn);
        assert_eq!(
            c_result.to_ne_bytes(),
            rust_result.to_ne_bytes(),
            "byte mismatch for pfcn={pfcn}: C={c_result}, Rust={rust_result}"
        );
        (c_result, rust_result)
    }
}

fn next_random(state: &mut u64) -> u32 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    (*state >> 16) as u32
}

#[test]
fn all_valid_configuration_rows_match() {
    with_functions(|c_function, rust_function| {
        let mut seed = 0x6a09_e667_f3bc_c909_u64;
        let mut hits = [0_u32; 12];

        for _ in 0..49_152 {
            let pfcn = (next_random(&mut seed) % 12) as c_int;
            hits[pfcn as usize] += 1;
            let (c_result, rust_result) = compare_call(c_function, rust_function, pfcn);
            assert_eq!(c_result, 1, "C rejected valid pfcn={pfcn}");
            assert_eq!(rust_result, 1, "Rust rejected valid pfcn={pfcn}");
        }

        for (pfcn, count) in hits.into_iter().enumerate() {
            assert!(
                count >= 3_500,
                "randomized run under-sampled configuration pfcn={pfcn}: {count} calls"
            );
        }
    });
}

#[test]
fn invalid_selector_error_row_matches() {
    let boundary_values = [
        c_int::MIN,
        -65_536,
        -2,
        -1,
        12,
        13,
        14,
        15,
        16,
        65_536,
        c_int::MAX,
    ];

    with_functions(|c_function, rust_function| {
        for pfcn in boundary_values {
            let (c_result, rust_result) = compare_call(c_function, rust_function, pfcn);
            assert_eq!(c_result, 0, "C accepted invalid pfcn={pfcn}");
            assert_eq!(rust_result, 0, "Rust accepted invalid pfcn={pfcn}");
        }

        let mut seed = 0xbb67_ae85_84ca_a73b_u64;
        for _ in 0..49_152 {
            let mut pfcn = next_random(&mut seed) as c_int;
            if (0..=11).contains(&pfcn) {
                pfcn = pfcn.wrapping_add(12);
            }
            let (c_result, rust_result) = compare_call(c_function, rust_function, pfcn);
            assert_eq!(c_result, 0, "C accepted invalid pfcn={pfcn}");
            assert_eq!(rust_result, 0, "Rust accepted invalid pfcn={pfcn}");
        }
    });
}
