use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};

type Pow43 = unsafe extern "C" fn(i32) -> f32;

struct Implementations {
    c: Library,
    rust: Library,
}

impl Implementations {
    fn load() -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_path = root.join("c_src/build/libtranslated_rust.so");
        let rust_path = rust_library_path(&root);

        assert!(
            c_path.is_file(),
            "C shared library is missing: {}",
            c_path.display()
        );
        assert!(
            rust_path.is_file(),
            "Rust shared library is missing: {}",
            rust_path.display()
        );

        unsafe {
            Self {
                c: Library::new(c_path).expect("load C shared library"),
                rust: Library::new(rust_path).expect("load Rust shared library"),
            }
        }
    }

    fn compare(&self, x: i32) {
        unsafe {
            let c_pow43: Symbol<'_, Pow43> = self.c.get(b"pow43").expect("load C pow43");
            let rust_pow43: Symbol<'_, Pow43> = self.rust.get(b"pow43").expect("load Rust pow43");
            let c_result = c_pow43(x);
            let rust_result = rust_pow43(x);

            assert_eq!(
                c_result.to_bits(),
                rust_result.to_bits(),
                "pow43({x}): C={c_result:?} ({:#010x}), Rust={rust_result:?} ({:#010x})",
                c_result.to_bits(),
                rust_result.to_bits()
            );
        }
    }
}

fn rust_library_path(root: &Path) -> PathBuf {
    let direct = root.join("target/debug/libpow43_lib.so");
    if direct.is_file() {
        return direct;
    }

    let deps = root.join("target/debug/deps");
    let mut candidates: Vec<_> = std::fs::read_dir(&deps)
        .unwrap_or_else(|error| panic!("read {}: {error}", deps.display()))
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("libpow43_lib") && name.ends_with(".so"))
        })
        .collect();
    candidates.sort();
    candidates
        .pop()
        .unwrap_or_else(|| panic!("Rust shared library is missing under {}", deps.display()))
}

fn generated_values(
    seed: u64,
    count: usize,
    minimum: i32,
    maximum: i32,
    predicate: impl Fn(i32) -> bool,
) -> Vec<i32> {
    let mut state = seed;
    let width = (i64::from(maximum) - i64::from(minimum) + 1) as u64;
    let mut values = Vec::with_capacity(count + 2);
    values.extend([minimum, maximum].into_iter().filter(|x| predicate(*x)));

    while values.len() < count + 2 {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let x = minimum + (state % width) as i32;
        if predicate(x) {
            values.push(x);
        }
    }
    values
}

fn scaled_sign(x: i32) -> i32 {
    (2 * (x << 3)) & 64
}

fn unscaled_sign(x: i32) -> i32 {
    (2 * x) & 64
}

#[test]
fn config_1_direct_table_lookup() {
    let implementations = Implementations::load();
    for x in generated_values(0x0101_5eed, 4_096, -16, 128, |_| true) {
        implementations.compare(x);
    }
}

#[test]
fn config_2_scaled_interpolation_sign_zero() {
    let implementations = Implementations::load();
    for x in generated_values(0x0202_5eed, 8_192, 129, 1023, |x| scaled_sign(x) == 0) {
        implementations.compare(x);
    }
}

#[test]
fn config_3_scaled_interpolation_sign_64() {
    let implementations = Implementations::load();
    for x in generated_values(0x0303_5eed, 8_192, 129, 1023, |x| scaled_sign(x) == 64) {
        implementations.compare(x);
    }
}

#[test]
fn config_4_unscaled_interpolation_sign_zero() {
    let implementations = Implementations::load();
    for x in generated_values(0x0404_5eed, 16_384, 1024, 8223, |x| unscaled_sign(x) == 0) {
        implementations.compare(x);
    }
}

#[test]
fn config_5_unscaled_interpolation_sign_64() {
    let implementations = Implementations::load();
    for x in generated_values(0x0505_5eed, 16_384, 1024, 8223, |x| unscaled_sign(x) == 64) {
        implementations.compare(x);
    }
}
