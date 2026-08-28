use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::path::{Path, PathBuf};

type EncodeQuant = unsafe extern "C" fn(c_int, c_int, c_int, c_int, c_int, c_int) -> c_int;

const RANDOM_CASES_PER_ROW: usize = 4_096;

struct SplitMix64(u64);

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut value = self.0;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        value ^ (value >> 31)
    }

    fn next_i32(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    manifest_dir()
        .join("../c_src/build")
        .join("libharvest-work-6nhExu.so")
}

fn rust_library_path() -> PathBuf {
    manifest_dir()
        .join("target")
        .join("release")
        .join("libencode_quant_lib.so")
}

unsafe fn load_library(path: &Path) -> Library {
    assert!(
        path.is_file(),
        "shared library does not exist: {}",
        path.display()
    );
    // SAFETY: The tests keep the returned library loaded while using symbols.
    unsafe { Library::new(path) }.unwrap_or_else(|error| {
        panic!("failed to load {}: {error}", path.display());
    })
}

fn with_apis(test: impl FnOnce(EncodeQuant, EncodeQuant)) {
    // SAFETY: Both libraries export encode_quant with the signature in lib.h.
    unsafe {
        let c_library = load_library(&c_library_path());
        let rust_library = load_library(&rust_library_path());
        let c_symbol: Symbol<'_, EncodeQuant> = c_library
            .get(b"encode_quant\0")
            .expect("C library does not export encode_quant");
        let rust_symbol: Symbol<'_, EncodeQuant> = rust_library
            .get(b"encode_quant\0")
            .expect("Rust library does not export encode_quant");

        test(*c_symbol, *rust_symbol);
    }
}

fn assert_same(
    c_encode_quant: EncodeQuant,
    rust_encode_quant: EncodeQuant,
    input: [i32; 6],
    context: &str,
) {
    // SAFETY: All arguments and the return value are scalar C ints.
    let c_result =
        unsafe { c_encode_quant(input[0], input[1], input[2], input[3], input[4], input[5]) };
    // SAFETY: All arguments and the return value are scalar C ints.
    let rust_result =
        unsafe { rust_encode_quant(input[0], input[1], input[2], input[3], input[4], input[5]) };
    assert_eq!(
        rust_result, c_result,
        "{context}: input [uni={}, step={}, pred={}, tgt={}, tgt2={}, lsbit={}]",
        input[0], input[1], input[2], input[3], input[4], input[5]
    );
}

#[test]
fn all_configuration_rows_match_randomized_inputs() {
    const LSBIT_CLASSES: [&[i32]; 4] = [
        &[0],
        &[4],
        &[1, 3, -1, i32::MIN + 1, i32::MAX],
        &[2, 6, -2, i32::MIN, i32::MAX - 1],
    ];

    with_apis(|c_encode_quant, rust_encode_quant| {
        for (class_index, lsbit_values) in LSBIT_CLASSES.iter().enumerate() {
            for low_nibble in 0_u32..16 {
                let row = class_index * 16 + low_nibble as usize + 1;
                let mut random = SplitMix64::new(0xd1ff_e2e0_0000_0000_u64 ^ row as u64);

                for case_index in 0..RANDOM_CASES_PER_ROW {
                    let uni = ((random.next_u64() as u32 & !0xf) | low_nibble) as i32;
                    let input = [
                        uni,
                        random.next_i32(),
                        random.next_i32(),
                        random.next_i32(),
                        random.next_i32(),
                        lsbit_values[case_index % lsbit_values.len()],
                    ];
                    assert_same(
                        c_encode_quant,
                        rust_encode_quant,
                        input,
                        &format!("CONFIGS.md C{row:03}, randomized case {case_index}"),
                    );
                }
            }
        }
    });
}

#[test]
fn full_width_integer_boundaries_match() {
    const VALUES: [i32; 17] = [
        i32::MIN,
        i32::MIN + 1,
        -17,
        -16,
        -9,
        -8,
        -1,
        0,
        1,
        4,
        7,
        8,
        9,
        15,
        16,
        i32::MAX - 1,
        i32::MAX,
    ];
    const BASES: [[i32; 6]; 4] = [
        [0, 0, 0, 0, 0, 0],
        [7, 8, -1, 1, -1, 4],
        [8, -8, i32::MAX, i32::MIN, 0, -1],
        [-1, i32::MIN, i32::MIN, i32::MAX, i32::MIN, i32::MIN],
    ];

    with_apis(|c_encode_quant, rust_encode_quant| {
        for (base_index, base) in BASES.into_iter().enumerate() {
            assert_same(
                c_encode_quant,
                rust_encode_quant,
                base,
                &format!("boundary base {base_index}"),
            );
            for argument in 0..base.len() {
                for value in VALUES {
                    let mut input = base;
                    input[argument] = value;
                    assert_same(
                        c_encode_quant,
                        rust_encode_quant,
                        input,
                        &format!("boundary base {base_index}, argument {argument}, value {value}"),
                    );
                }
            }
        }
    });
}
