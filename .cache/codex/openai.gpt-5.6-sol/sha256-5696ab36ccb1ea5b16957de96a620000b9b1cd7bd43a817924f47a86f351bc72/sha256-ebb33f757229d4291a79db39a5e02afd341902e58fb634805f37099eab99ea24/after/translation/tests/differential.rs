use libloading::Library;
use std::ffi::c_int;
use std::path::{Path, PathBuf};

type Pow43 = unsafe extern "C" fn(c_int) -> f32;

struct Implementations {
    _c_library: Library,
    _rust_library: Library,
    c_pow43: Pow43,
    rust_pow43: Pow43,
}

impl Implementations {
    fn load() -> Self {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_path = manifest_dir
            .join("../c_src/build/libharvest-work-3NSYMq.so")
            .canonicalize()
            .expect("C shared library must be built before running tests");
        let rust_path = rust_shared_library_path();

        // SAFETY: Both paths name shared libraries built by this workspace.
        let c_library = unsafe { Library::new(&c_path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", c_path.display()));
        // SAFETY: Both paths name shared libraries built by this workspace.
        let rust_library = unsafe { Library::new(&rust_path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", rust_path.display()));

        // Copy the function pointers while retaining both libraries for the
        // entire lifetime of this holder.
        // SAFETY: The C header and Rust export both define `float pow43(int)`.
        let c_pow43 = unsafe {
            *c_library
                .get::<Pow43>(b"pow43\0")
                .expect("C library must export pow43")
        };
        // SAFETY: The C header and Rust export both define `float pow43(int)`.
        let rust_pow43 = unsafe {
            *rust_library
                .get::<Pow43>(b"pow43\0")
                .expect("Rust library must export pow43")
        };

        Self {
            _c_library: c_library,
            _rust_library: rust_library,
            c_pow43,
            rust_pow43,
        }
    }

    fn assert_equal(&self, x: i32) {
        // SAFETY: The function pointers have the signature declared by the C
        // header. Test inputs stay within the C table's in-bounds domain.
        let c_value = unsafe { (self.c_pow43)(x) };
        // SAFETY: Same ABI and domain as the C call above.
        let rust_value = unsafe { (self.rust_pow43)(x) };

        assert_eq!(
            c_value.to_bits(),
            rust_value.to_bits(),
            "pow43({x}) differs: C={c_value:?} ({:#010x}), Rust={rust_value:?} ({:#010x})",
            c_value.to_bits(),
            rust_value.to_bits(),
        );
    }
}

fn rust_shared_library_path() -> PathBuf {
    let test_executable = std::env::current_exe().expect("test executable path");
    let deps_dir = test_executable
        .parent()
        .expect("integration test executable must have a parent");
    let path = deps_dir.join("libpow43_lib.so");
    assert!(
        path.is_file(),
        "Rust shared library was not produced at {}",
        path.display()
    );
    path
}

fn sign_after_scaling(x: i32) -> i32 {
    (x.wrapping_shl(3).wrapping_mul(2)) & 64
}

fn sign_without_scaling(x: i32) -> i32 {
    x.wrapping_mul(2) & 64
}

fn inputs_for_row(
    min: i32,
    max: i32,
    expected_sign: Option<i32>,
    sign: fn(i32) -> i32,
    mut seed: u64,
) -> Vec<i32> {
    let matches = |x: i32| expected_sign.is_none_or(|wanted| sign(x) == wanted);
    let mut inputs: Vec<i32> = (min..=max).filter(|&x| matches(x)).collect();
    assert!(!inputs.is_empty());

    let first = *inputs.first().unwrap();
    let last = *inputs.last().unwrap();
    inputs.extend([first, last]);

    let span = (i64::from(max) - i64::from(min) + 1) as u64;
    let mut random_count = 0;
    while random_count < 4096 {
        seed = seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let x = i64::from(min) + (seed % span) as i64;
        let x = i32::try_from(x).unwrap();
        if matches(x) {
            inputs.push(x);
            random_count += 1;
        }
    }

    inputs
}

fn run_row(inputs: impl IntoIterator<Item = i32>) {
    let implementations = Implementations::load();
    for x in inputs {
        implementations.assert_equal(x);
    }
}

#[test]
fn config_01_direct_table_lookup() {
    run_row(inputs_for_row(-16, 128, None, |_| 0, 0x01c0_ffee));
}

#[test]
fn config_02_scaled_interpolation_sign_zero() {
    run_row(inputs_for_row(
        129,
        1023,
        Some(0),
        sign_after_scaling,
        0x02c0_ffee,
    ));
}

#[test]
fn config_03_scaled_interpolation_sign_64() {
    run_row(inputs_for_row(
        129,
        1023,
        Some(64),
        sign_after_scaling,
        0x03c0_ffee,
    ));
}

#[test]
fn config_04_unscaled_interpolation_sign_zero() {
    run_row(inputs_for_row(
        1024,
        8223,
        Some(0),
        sign_without_scaling,
        0x04c0_ffee,
    ));
}

#[test]
fn config_05_unscaled_interpolation_sign_64() {
    run_row(inputs_for_row(
        1024,
        8223,
        Some(64),
        sign_without_scaling,
        0x05c0_ffee,
    ));
}

#[test]
fn exported_symbol_is_resolvable_from_both_shared_libraries() {
    let implementations = Implementations::load();
    implementations.assert_equal(0);
}

#[test]
fn all_defined_integer_inputs_match_exhaustively() {
    run_row(-16..=8223);
}

#[test]
fn shared_library_paths_are_external_artifacts() {
    let rust_path = rust_shared_library_path();
    assert_eq!(
        rust_path.file_name(),
        Some(Path::new("libpow43_lib.so").as_os_str())
    );
}
