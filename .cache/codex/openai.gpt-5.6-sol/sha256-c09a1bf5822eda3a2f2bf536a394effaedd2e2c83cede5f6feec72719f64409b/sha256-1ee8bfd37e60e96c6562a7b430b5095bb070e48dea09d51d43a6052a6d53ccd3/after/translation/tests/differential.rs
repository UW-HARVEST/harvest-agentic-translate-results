use libloading::Library;
use std::ffi::{c_float, c_int};
use std::path::{Path, PathBuf};

type LdexpQ2 = unsafe extern "C" fn(c_float, c_int) -> c_float;

struct Implementations {
    _c_library: Library,
    _rust_library: Library,
    c: LdexpQ2,
    rust: LdexpQ2,
}

impl Implementations {
    fn load() -> Self {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_path = find_c_library(&manifest.join("../c_src/build"));
        let target = std::env::var_os("CARGO_TARGET_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| manifest.join("target"));
        let rust_path = target.join("release/libldexp_q2_lib.so");

        assert!(
            rust_path.is_file(),
            "missing {}; run `cargo build --release` first",
            rust_path.display()
        );

        unsafe {
            let c_library = Library::new(&c_path)
                .unwrap_or_else(|error| panic!("load {}: {error}", c_path.display()));
            let rust_library = Library::new(&rust_path)
                .unwrap_or_else(|error| panic!("load {}: {error}", rust_path.display()));
            let c = *c_library
                .get::<LdexpQ2>(b"ldexp_q2\0")
                .expect("C ldexp_q2 export");
            let rust = *rust_library
                .get::<LdexpQ2>(b"ldexp_q2\0")
                .expect("Rust ldexp_q2 export");

            Self {
                _c_library: c_library,
                _rust_library: rust_library,
                c,
                rust,
            }
        }
    }

    fn assert_equal(&self, y_bits: u32, exp_q2: i32) {
        let y = f32::from_bits(y_bits);
        let c = unsafe { (self.c)(y, exp_q2) };
        let rust = unsafe { (self.rust)(y, exp_q2) };

        assert_eq!(
            rust.to_bits(),
            c.to_bits(),
            "y=0x{y_bits:08x} exp_q2={exp_q2}: C=0x{:08x}, Rust=0x{:08x}",
            c.to_bits(),
            rust.to_bits()
        );
    }
}

fn find_c_library(directory: &Path) -> PathBuf {
    let mut matches: Vec<_> = std::fs::read_dir(directory)
        .unwrap_or_else(|error| panic!("read {}: {error}", directory.display()))
        .map(|entry| entry.expect("C build directory entry").path())
        .filter(|path| {
            path.extension().and_then(|value| value.to_str()) == Some("so")
                && path
                    .file_name()
                    .and_then(|value| value.to_str())
                    .is_some_and(|name| name.starts_with("lib"))
        })
        .collect();
    matches.sort();
    assert_eq!(
        matches.len(),
        1,
        "expected one C shared object in {}",
        directory.display()
    );
    matches.pop().unwrap()
}

struct SplitMix64(u64);

impl SplitMix64 {
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut value = self.0;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        value ^ (value >> 31)
    }
}

#[derive(Clone, Copy)]
enum LoopShape {
    One,
    Two,
    Many,
}

fn exponent_for(rng: &mut SplitMix64, shape: LoopShape, residue: i32, sample: usize) -> i32 {
    match shape {
        LoopShape::One if sample % 3 == 0 => {
            let count = ((120 - residue) / 4 + 1) as u64;
            residue + 4 * (rng.next_u64() % count) as i32
        }
        LoopShape::One if sample % 3 == 1 => {
            let negative = (rng.next_u64() as i32) | i32::MIN;
            (negative & !3) | residue
        }
        LoopShape::One => {
            let negative = -1 - (rng.next_u64() % 16_384) as i32;
            (negative & !3) | residue
        }
        LoopShape::Two => {
            let first = 121 + (residue - (121 & 3) + 4) % 4;
            let count = ((240 - first) / 4 + 1) as u64;
            first + 4 * (rng.next_u64() % count) as i32
        }
        LoopShape::Many => {
            let first = 241 + (residue - (241 & 3) + 4) % 4;
            let count = ((20_000 - first) / 4 + 1) as u64;
            first + 4 * (rng.next_u64() % count) as i32
        }
    }
}

fn exercise_row(shape: LoopShape, residue: i32) {
    let implementations = Implementations::load();
    let mut rng = SplitMix64(
        0x5eed_c0de_d15c_a11u64
            ^ ((residue as u64) << 32)
            ^ match shape {
                LoopShape::One => 1,
                LoopShape::Two => 2,
                LoopShape::Many => 3,
            },
    );

    for sample in 0..2_048 {
        let y_bits = rng.next_u64() as u32;
        let exp_q2 = exponent_for(&mut rng, shape, residue, sample);
        assert_eq!(exp_q2 & 3, residue);
        implementations.assert_equal(y_bits, exp_q2);
    }
}

macro_rules! row_test {
    ($name:ident, $shape:expr, $residue:expr) => {
        #[test]
        fn $name() {
            exercise_row($shape, $residue);
        }
    };
}

row_test!(config_01_one_iteration_index_0, LoopShape::One, 0);
row_test!(config_02_one_iteration_index_1, LoopShape::One, 1);
row_test!(config_03_one_iteration_index_2, LoopShape::One, 2);
row_test!(config_04_one_iteration_index_3, LoopShape::One, 3);
row_test!(config_05_two_iterations_index_0, LoopShape::Two, 0);
row_test!(config_06_two_iterations_index_1, LoopShape::Two, 1);
row_test!(config_07_two_iterations_index_2, LoopShape::Two, 2);
row_test!(config_08_two_iterations_index_3, LoopShape::Two, 3);
row_test!(config_09_many_iterations_index_0, LoopShape::Many, 0);
row_test!(config_10_many_iterations_index_1, LoopShape::Many, 1);
row_test!(config_11_many_iterations_index_2, LoopShape::Many, 2);
row_test!(config_12_many_iterations_index_3, LoopShape::Many, 3);

#[test]
fn scalar_ffi_boundaries_match() {
    let implementations = Implementations::load();
    let y_bits = [
        0x0000_0000,
        0x8000_0000,
        0x0000_0001,
        0x007f_ffff,
        0x0080_0000,
        0x3f80_0000,
        0xbf80_0000,
        0x7f7f_ffff,
        0x7f80_0000,
        0xff80_0000,
        0x7fc1_2345,
        0x7f81_2345,
    ];
    let exponents = [
        i32::MIN,
        i32::MIN + 1,
        -1,
        0,
        1,
        119,
        120,
        121,
        239,
        240,
        241,
        i32::MAX - 3,
        i32::MAX - 2,
        i32::MAX - 1,
        i32::MAX,
    ];

    for (index, exp_q2) in exponents.into_iter().enumerate() {
        implementations.assert_equal(y_bits[index % y_bits.len()], exp_q2);
    }
}
