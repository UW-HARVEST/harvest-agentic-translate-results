use libloading::{Library, Symbol};
use std::fs;
use std::path::{Path, PathBuf};

type Float2Half = unsafe extern "C" fn(f32) -> u16;

fn shared_library_in(directory: &Path, required_fragment: &str) -> PathBuf {
    let mut matches: Vec<_> = fs::read_dir(directory)
        .unwrap_or_else(|error| {
            panic!(
                "failed to read shared-library directory {}: {error}",
                directory.display()
            )
        })
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension().is_some_and(|extension| extension == "so")
                && path
                    .file_name()
                    .is_some_and(|name| name.to_string_lossy().contains(required_fragment))
        })
        .collect();
    matches.sort();

    assert_eq!(
        matches.len(),
        1,
        "expected exactly one *{required_fragment}*.so in {}, found {matches:?}",
        directory.display()
    );
    matches.pop().unwrap()
}

fn libraries() -> (Library, Library) {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_library = shared_library_in(&manifest_dir.join("../c_src/build"), "harvest-work-Rg2BZs");

    // `cargo test` builds an rlib test harness but does not place the cdylib in
    // target/debug. The verification commands explicitly build the cdylib in
    // release mode before running this integration test.
    let rust_library = shared_library_in(&manifest_dir.join("target/release"), "float2half_lib");

    // SAFETY: Both paths were discovered as shared objects produced by this
    // workspace's CMake and Cargo builds.
    unsafe {
        (
            Library::new(c_library).expect("failed to load C shared library"),
            Library::new(rust_library).expect("failed to load Rust shared library"),
        )
    }
}

fn next_random(state: &mut u64) -> u32 {
    let mut value = *state;
    value ^= value << 13;
    value ^= value >> 7;
    value ^= value << 17;
    *state = value;
    value as u32
}

fn compare_configuration(sign: u32, exponent_start: u32, exponent_end: u32, seed: u64) {
    let (c_library, rust_library) = libraries();

    // SAFETY: The symbol is specified by the public C header and both
    // libraries export it with the exact `float -> uint16_t` C ABI.
    let (c_float2half, rust_float2half): (Symbol<Float2Half>, Symbol<Float2Half>) = unsafe {
        (
            c_library
                .get(b"float2half\0")
                .expect("C library is missing float2half"),
            rust_library
                .get(b"float2half\0")
                .expect("Rust library is missing float2half"),
        )
    };

    let sign_bits = sign << 31;
    let mut state = seed;

    for exponent in exponent_start..=exponent_end {
        for mantissa in [0, 1, 0x003f_ffff, 0x0040_0000, 0x007f_fffe, 0x007f_ffff] {
            compare_one(
                &c_float2half,
                &rust_float2half,
                sign_bits | (exponent << 23) | mantissa,
            );
        }

        for _ in 0..128 {
            let mantissa = next_random(&mut state) & 0x007f_ffff;
            compare_one(
                &c_float2half,
                &rust_float2half,
                sign_bits | (exponent << 23) | mantissa,
            );
        }
    }
}

fn compare_one(c_float2half: &Float2Half, rust_float2half: &Float2Half, bits: u32) {
    let input = f32::from_bits(bits);

    // SAFETY: The loaded symbols have the declared scalar C ABI and accept
    // every binary32 object representation.
    let (c_result, rust_result) = unsafe { (c_float2half(input), rust_float2half(input)) };

    assert_eq!(
        rust_result, c_result,
        "mismatch for input bits 0x{bits:08x}: C=0x{c_result:04x}, Rust=0x{rust_result:04x}"
    );
}

macro_rules! configuration_test {
    ($name:ident, $sign:expr, $start:expr, $end:expr, $seed:expr) => {
        #[test]
        fn $name() {
            compare_configuration($sign, $start, $end, $seed);
        }
    };
}

configuration_test!(config_01_positive_exp_000_102, 0, 0, 102, 0x0101_0101);
configuration_test!(config_02_positive_exp_103, 0, 103, 103, 0x0202_0202);
configuration_test!(config_03_positive_exp_104, 0, 104, 104, 0x0303_0303);
configuration_test!(config_04_positive_exp_105, 0, 105, 105, 0x0404_0404);
configuration_test!(config_05_positive_exp_106, 0, 106, 106, 0x0505_0505);
configuration_test!(config_06_positive_exp_107, 0, 107, 107, 0x0606_0606);
configuration_test!(config_07_positive_exp_108, 0, 108, 108, 0x0707_0707);
configuration_test!(config_08_positive_exp_109, 0, 109, 109, 0x0808_0808);
configuration_test!(config_09_positive_exp_110, 0, 110, 110, 0x0909_0909);
configuration_test!(config_10_positive_exp_111, 0, 111, 111, 0x1010_1010);
configuration_test!(config_11_positive_exp_112, 0, 112, 112, 0x1111_1111);
configuration_test!(config_12_positive_exp_113_142, 0, 113, 142, 0x1212_1212);
configuration_test!(config_13_positive_exp_143_254, 0, 143, 254, 0x1313_1313);
configuration_test!(config_14_positive_exp_255, 0, 255, 255, 0x1414_1414);
configuration_test!(config_15_negative_exp_000_102, 1, 0, 102, 0x1515_1515);
configuration_test!(config_16_negative_exp_103, 1, 103, 103, 0x1616_1616);
configuration_test!(config_17_negative_exp_104, 1, 104, 104, 0x1717_1717);
configuration_test!(config_18_negative_exp_105, 1, 105, 105, 0x1818_1818);
configuration_test!(config_19_negative_exp_106, 1, 106, 106, 0x1919_1919);
configuration_test!(config_20_negative_exp_107, 1, 107, 107, 0x2020_2020);
configuration_test!(config_21_negative_exp_108, 1, 108, 108, 0x2121_2121);
configuration_test!(config_22_negative_exp_109, 1, 109, 109, 0x2222_2222);
configuration_test!(config_23_negative_exp_110, 1, 110, 110, 0x2323_2323);
configuration_test!(config_24_negative_exp_111, 1, 111, 111, 0x2424_2424);
configuration_test!(config_25_negative_exp_112, 1, 112, 112, 0x2525_2525);
configuration_test!(config_26_negative_exp_113_142, 1, 113, 142, 0x2626_2626);
configuration_test!(config_27_negative_exp_143_254, 1, 143, 254, 0x2727_2727);
configuration_test!(config_28_negative_exp_255, 1, 255, 255, 0x2828_2828);
