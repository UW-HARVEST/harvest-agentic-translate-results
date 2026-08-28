use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::path::{Path, PathBuf};

type Pow43 = unsafe extern "C" fn(c_int) -> f32;

fn profile_dir() -> PathBuf {
    std::env::current_exe()
        .expect("test executable path")
        .parent()
        .and_then(Path::parent)
        .expect("Cargo profile directory")
        .to_path_buf()
}

fn compare_inputs(inputs: &[c_int]) {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let c_path = manifest_dir.join("../c_src/build/libharvest-work-BDhd95.so");
    let profile_dir = profile_dir();
    let top_level_rust_path = profile_dir.join("libpow43_lib.so");
    let deps_rust_path = profile_dir.join("deps/libpow43_lib.so");
    let rust_path = if top_level_rust_path.is_file() {
        top_level_rust_path
    } else {
        deps_rust_path
    };

    assert!(
        c_path.is_file(),
        "missing C shared library: {}",
        c_path.display()
    );
    assert!(
        rust_path.is_file(),
        "missing Rust shared library: {}",
        rust_path.display()
    );

    unsafe {
        let c_library = Library::new(&c_path).expect("load C shared library");
        let rust_library = Library::new(&rust_path).expect("load Rust shared library");
        let c_pow43: Symbol<Pow43> = c_library.get(b"pow43").expect("load C pow43");
        let rust_pow43: Symbol<Pow43> = rust_library.get(b"pow43").expect("load Rust pow43");

        for &x in inputs {
            let expected = c_pow43(x);
            let actual = rust_pow43(x);
            assert_eq!(
                actual.to_bits(),
                expected.to_bits(),
                "pow43({x}): C={expected:?} (0x{:08x}), Rust={actual:?} (0x{:08x})",
                expected.to_bits(),
                actual.to_bits()
            );
        }
    }
}

fn shuffled_range(
    start: c_int,
    end: c_int,
    seed: u64,
    include: impl Fn(c_int) -> bool,
) -> Vec<c_int> {
    let mut values: Vec<_> = (start..=end).filter(|&x| include(x)).collect();
    let mut state = seed;

    for index in (1..values.len()).rev() {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        values.swap(index, (state as usize) % (index + 1));
    }

    values
}

#[test]
fn config_1_direct_table_path() {
    let values = shuffled_range(-16, 128, 0x4d59_5df4_d0f3_3173, |_| true);
    assert_eq!(values.len(), 145);
    compare_inputs(&values);
}

#[test]
fn config_2_scaled_path_sign_zero() {
    let values = shuffled_range(129, 1023, 0x70c8_7ec1_8e4d_2f21, |x| ((x << 3) & 32) == 0);
    assert!(values.len() > 400);
    compare_inputs(&values);
}

#[test]
fn config_3_scaled_path_sign_64() {
    let values = shuffled_range(129, 1023, 0x23b7_2189_4738_0fa1, |x| ((x << 3) & 32) != 0);
    assert!(values.len() > 400);
    compare_inputs(&values);
}

#[test]
fn config_4_unscaled_path_sign_zero() {
    let values = shuffled_range(1024, 8223, 0x9e37_79b9_7f4a_7c15, |x| (x & 32) == 0);
    assert_eq!(values.len(), 3616);
    compare_inputs(&values);
}

#[test]
fn config_5_unscaled_path_sign_64() {
    let values = shuffled_range(1024, 8223, 0xd1b5_4a32_d192_ed03, |x| (x & 32) != 0);
    assert_eq!(values.len(), 3584);
    compare_inputs(&values);
}
