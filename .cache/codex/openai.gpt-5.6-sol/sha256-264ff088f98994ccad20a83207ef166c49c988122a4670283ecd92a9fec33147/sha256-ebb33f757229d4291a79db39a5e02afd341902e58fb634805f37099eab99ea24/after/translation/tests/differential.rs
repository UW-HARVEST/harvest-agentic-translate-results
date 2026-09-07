use libloading::{Library, Symbol};
use std::fs;
use std::path::{Path, PathBuf};

type Jumpnode = unsafe extern "C" fn(i32, i32, i32, i32) -> i32;

struct Libraries {
    c: Library,
    rust: Library,
}

impl Libraries {
    fn load() -> Self {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_build = manifest.join("../c_src/build");
        let c_path = find_single_so(&c_build, "C shared library");

        let rust_path = manifest.join("target/release/libjumpnode_lib.so");
        assert!(
            rust_path.is_file(),
            "Rust cdylib does not exist at {}; run `cargo build --release` before testing",
            rust_path.display()
        );

        // SAFETY: Both paths are build artifacts controlled by this test.
        let c = unsafe { Library::new(&c_path) }
            .unwrap_or_else(|error| panic!("load C library {}: {error}", c_path.display()));
        // SAFETY: Both paths are build artifacts controlled by this test.
        let rust = unsafe { Library::new(&rust_path) }
            .unwrap_or_else(|error| panic!("load Rust library {}: {error}", rust_path.display()));

        Self { c, rust }
    }

    fn compare(&self, operation_mode: i32, node_id: i32, depth: i32, flags: i32) -> i32 {
        // SAFETY: The public C header and Rust export both declare this exact ABI.
        let c_jumpnode: Symbol<'_, Jumpnode> =
            unsafe { self.c.get(b"jumpnode\0") }.expect("load C jumpnode symbol");
        // SAFETY: The public C header and Rust export both declare this exact ABI.
        let rust_jumpnode: Symbol<'_, Jumpnode> =
            unsafe { self.rust.get(b"jumpnode\0") }.expect("load Rust jumpnode symbol");

        // SAFETY: All parameters are by-value i32 values, so every bit pattern is valid.
        let c_result = unsafe { c_jumpnode(operation_mode, node_id, depth, flags) };
        // SAFETY: All parameters are by-value i32 values, so every bit pattern is valid.
        let rust_result = unsafe { rust_jumpnode(operation_mode, node_id, depth, flags) };

        assert_eq!(
            rust_result.to_ne_bytes(),
            c_result.to_ne_bytes(),
            "ABI result mismatch for mode={operation_mode}, node_id={node_id}, \
             depth={depth}, flags={flags}: C={c_result}, Rust={rust_result}"
        );
        c_result
    }
}

fn find_single_so(directory: &Path, description: &str) -> PathBuf {
    let mut candidates: Vec<_> = fs::read_dir(directory)
        .unwrap_or_else(|error| panic!("read {} {}: {error}", description, directory.display()))
        .map(|entry| entry.expect("read shared-library directory entry").path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "so"))
        .collect();
    candidates.sort();
    assert_eq!(
        candidates.len(),
        1,
        "expected exactly one {description} in {}, found {candidates:?}",
        directory.display()
    );
    candidates.remove(0)
}

#[derive(Clone, Copy)]
struct XorShift64(u64);

impl XorShift64 {
    fn new(seed: u64) -> Self {
        assert_ne!(seed, 0);
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value
    }

    fn next_i32(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }
}

const INT_SHAPES: &[i32] = &[
    i32::MIN,
    -1_000_000_000,
    -10_000,
    -100,
    -10,
    -9,
    -1,
    0,
    1,
    9,
    10,
    100,
    10_000,
    1_000_000_000,
    i32::MAX,
];

fn compare_boundary_cross_product(libraries: &Libraries, flags: i32) {
    for &node_id in INT_SHAPES {
        for &depth in INT_SHAPES {
            libraries.compare(0o3, node_id, depth, flags);
        }
    }
}

#[test]
fn config_c1_zero_flags() {
    let libraries = Libraries::load();
    compare_boundary_cross_product(&libraries, 0);

    let mut rng = XorShift64::new(0x434f_4e46_4947_4331);
    for _ in 0..10_000 {
        libraries.compare(0o3, rng.next_i32(), rng.next_i32(), 0);
    }
}

#[test]
fn config_c2_low_bits_only() {
    let libraries = Libraries::load();
    for flags in 1..=0o177 {
        compare_boundary_cross_product(&libraries, flags);
    }

    let mut rng = XorShift64::new(0x434f_4e46_4947_4332);
    for _ in 0..10_000 {
        let flags = ((rng.next_u64() % 0o177) + 1) as i32;
        libraries.compare(0o3, rng.next_i32(), rng.next_i32(), flags);
    }
}

#[test]
fn config_c3_high_bits_with_zero_low_bits() {
    let libraries = Libraries::load();
    for flags in [0o200, 0o400, i32::MAX & !0o177, i32::MIN, -0o200] {
        assert_eq!(flags & 0o177, 0);
        compare_boundary_cross_product(&libraries, flags);
    }

    let mut rng = XorShift64::new(0x434f_4e46_4947_4333);
    for index in 0..10_000 {
        let mut flags = rng.next_i32() & !0o177;
        if flags == 0 {
            flags = 0o200;
        }
        if index % 2 == 0 {
            flags |= i32::MIN;
        }
        assert_ne!(flags, 0);
        assert_eq!(flags & 0o177, 0);
        libraries.compare(0o3, rng.next_i32(), rng.next_i32(), flags);
    }
}

#[test]
fn config_c4_high_and_low_bits() {
    let libraries = Libraries::load();
    for flags in [0o201, 0o577, i32::MAX, i32::MIN | 1, -1] {
        assert_ne!(flags & !0o177, 0);
        assert_ne!(flags & 0o177, 0);
        compare_boundary_cross_product(&libraries, flags);
    }

    let mut rng = XorShift64::new(0x434f_4e46_4947_4334);
    for index in 0..10_000 {
        let low = ((rng.next_u64() % 0o177) + 1) as i32;
        let mut high = rng.next_i32() & !0o177;
        if high == 0 {
            high = 0o200;
        }
        if index % 2 == 0 {
            high |= i32::MIN;
        }
        let flags = high | low;
        assert_ne!(flags & !0o177, 0);
        assert_ne!(flags & 0o177, 0);
        libraries.compare(0o3, rng.next_i32(), rng.next_i32(), flags);
    }
}

fn exercise_missing_node_error(mode: i32, expected: i32, seed: u64) {
    let libraries = Libraries::load();
    let mut rng = XorShift64::new(seed);

    for &node_id in INT_SHAPES {
        for &depth in INT_SHAPES {
            for flags in [i32::MIN, -1, 0, 1, i32::MAX] {
                assert_eq!(libraries.compare(mode, node_id, depth, flags), expected);
            }
        }
    }

    for _ in 0..10_000 {
        assert_eq!(
            libraries.compare(mode, rng.next_i32(), rng.next_i32(), rng.next_i32()),
            expected
        );
    }
}

#[test]
fn error_e1_mode_1_missing_node() {
    exercise_missing_node_error(0o1, 0o22, 0x4552_524f_525f_4531);
}

#[test]
fn error_e2_mode_2_missing_node() {
    exercise_missing_node_error(0o2, 0o42, 0x4552_524f_525f_4532);
}

#[test]
fn error_e3_mode_4_missing_node() {
    exercise_missing_node_error(0o4, 0o102, 0x4552_524f_525f_4533);
}

#[test]
fn error_e4_unknown_operation_values() {
    let libraries = Libraries::load();
    let invalid_boundary_modes = [i32::MIN, -1_000_000, -1, 0, 0o5, 0o6, 0o177, i32::MAX];

    for operation_mode in invalid_boundary_modes {
        for &node_id in INT_SHAPES {
            assert_eq!(
                libraries.compare(operation_mode, node_id, i32::MIN, i32::MAX),
                0o202
            );
        }
    }

    let mut rng = XorShift64::new(0x4552_524f_525f_4534);
    let mut tested = 0;
    while tested < 10_000 {
        let operation_mode = rng.next_i32();
        if !matches!(operation_mode, 0o1..=0o4) {
            assert_eq!(
                libraries.compare(
                    operation_mode,
                    rng.next_i32(),
                    rng.next_i32(),
                    rng.next_i32()
                ),
                0o202
            );
            tested += 1;
        }
    }
}
