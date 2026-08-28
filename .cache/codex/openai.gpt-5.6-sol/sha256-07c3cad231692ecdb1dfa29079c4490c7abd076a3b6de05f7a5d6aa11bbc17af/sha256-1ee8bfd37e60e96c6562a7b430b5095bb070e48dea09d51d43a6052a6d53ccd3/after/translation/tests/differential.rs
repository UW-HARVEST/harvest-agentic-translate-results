use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct CbRgb255 {
    r: u8,
    g: u8,
    b: u8,
}

type Tritanopia = unsafe extern "C" fn(CbRgb255) -> CbRgb255;

const LOW: bool = false;
const HIGH: bool = true;
const RANDOM_CASES: usize = 8_192;

fn library_paths() -> (PathBuf, PathBuf) {
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    (
        crate_dir.join("../c_src/build/libharvest-work-haOvWm.so"),
        crate_dir.join("target/release/libtritanopia_lib.so"),
    )
}

fn next_random(state: &mut u64) -> u64 {
    let mut value = *state;
    value ^= value << 13;
    value ^= value >> 7;
    value ^= value << 17;
    *state = value;
    value
}

fn channel_value(high: bool, state: &mut u64) -> u8 {
    let value = next_random(state);
    if high {
        11 + (value % 245) as u8
    } else {
        (value % 11) as u8
    }
}

fn assert_configuration(branches: [bool; 3], seed: u64) {
    let (c_path, rust_path) = library_paths();
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
        let c_function: Symbol<'_, Tritanopia> =
            c_library.get(b"tritanopia\0").expect("load C tritanopia");
        let rust_function: Symbol<'_, Tritanopia> = rust_library
            .get(b"tritanopia\0")
            .expect("load Rust tritanopia");

        let lows = [0, 10];
        let highs = [11, 255];
        for &r in if branches[0] { &highs } else { &lows } {
            for &g in if branches[1] { &highs } else { &lows } {
                for &b in if branches[2] { &highs } else { &lows } {
                    assert_equal(&c_function, &rust_function, CbRgb255 { r, g, b });
                }
            }
        }

        let mut state = seed;
        for _ in 0..RANDOM_CASES {
            let input = CbRgb255 {
                r: channel_value(branches[0], &mut state),
                g: channel_value(branches[1], &mut state),
                b: channel_value(branches[2], &mut state),
            };
            assert_equal(&c_function, &rust_function, input);
        }
    }
}

unsafe fn assert_equal(
    c_function: &Symbol<'_, Tritanopia>,
    rust_function: &Symbol<'_, Tritanopia>,
    input: CbRgb255,
) {
    let c_output = unsafe { c_function(input) };
    let rust_output = unsafe { rust_function(input) };
    assert_eq!(rust_output, c_output, "input: {input:?}");
}

#[test]
fn row_1_all_channels_low() {
    assert_configuration([LOW, LOW, LOW], 0x1580_44ad_8bd3_8e55);
}

#[test]
fn row_2_only_blue_high() {
    assert_configuration([LOW, LOW, HIGH], 0xd0f3_3173_13cb_23c0);
}

#[test]
fn row_3_only_green_high() {
    assert_configuration([LOW, HIGH, LOW], 0x9e37_79b9_7f4a_7c15);
}

#[test]
fn row_4_green_and_blue_high() {
    assert_configuration([LOW, HIGH, HIGH], 0x243f_6a88_85a3_08d3);
}

#[test]
fn row_5_only_red_high() {
    assert_configuration([HIGH, LOW, LOW], 0x1319_8a2e_0370_7344);
}

#[test]
fn row_6_red_and_blue_high() {
    assert_configuration([HIGH, LOW, HIGH], 0xa409_3822_299f_31d0);
}

#[test]
fn row_7_red_and_green_high() {
    assert_configuration([HIGH, HIGH, LOW], 0x082e_fa98_ec4e_6c89);
}

#[test]
fn row_8_all_channels_high() {
    assert_configuration([HIGH, HIGH, HIGH], 0x4528_21e6_38d0_1377);
}

#[test]
fn exhaustive_byte_domain() {
    let (c_path, rust_path) = library_paths();
    unsafe {
        let c_library = Library::new(c_path).expect("load C shared library");
        let rust_library = Library::new(rust_path).expect("load Rust shared library");
        let c_function: Symbol<'_, Tritanopia> =
            c_library.get(b"tritanopia\0").expect("load C tritanopia");
        let rust_function: Symbol<'_, Tritanopia> = rust_library
            .get(b"tritanopia\0")
            .expect("load Rust tritanopia");

        for r in u8::MIN..=u8::MAX {
            for g in u8::MIN..=u8::MAX {
                for b in u8::MIN..=u8::MAX {
                    assert_equal(&c_function, &rust_function, CbRgb255 { r, g, b });
                }
            }
        }
    }
}
