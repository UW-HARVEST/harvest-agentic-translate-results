use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use libloading::{Library, Symbol};

#[repr(C)]
#[allow(non_snake_case)]
#[derive(Clone, Copy, Debug)]
struct Rgb {
    R: u8,
    G: u8,
    B: u8,
}

type ContrastRatio = unsafe extern "C" fn(Rgb, Rgb) -> f32;

const CASES_PER_ROW: usize = 128;
const LOW_MAX: u8 = 10;
const HIGH_MIN: u8 = 11;

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
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

    fn byte_inclusive(&mut self, min: u8, max: u8) -> u8 {
        let width = u64::from(max) - u64::from(min) + 1;
        min + (self.next_u64() % width) as u8
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    let build_dir = manifest_dir().join("../c_src/build");
    let mut libraries = fs::read_dir(&build_dir)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", build_dir.display()))
        .map(|entry| entry.expect("invalid C build directory entry").path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "so"))
        .collect::<Vec<_>>();
    libraries.sort();
    assert_eq!(
        libraries.len(),
        1,
        "expected exactly one C shared library in {}",
        build_dir.display()
    );
    libraries.pop().unwrap()
}

fn rust_library_path() -> PathBuf {
    manifest_dir().join("target/release/libcontrast_ratio_lib.so")
}

fn with_apis(test: impl FnOnce(ContrastRatio, ContrastRatio)) {
    let c_path = c_library_path();
    let rust_path = rust_library_path();
    assert!(c_path.is_file(), "missing C library: {}", c_path.display());
    assert!(
        rust_path.is_file(),
        "missing Rust library: {}",
        rust_path.display()
    );

    unsafe {
        let c_library = Library::new(&c_path)
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", c_path.display()));
        let rust_library = Library::new(&rust_path)
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", rust_path.display()));
        let c_symbol: Symbol<ContrastRatio> = c_library
            .get(b"contrast_ratio\0")
            .expect("C contrast_ratio export is missing");
        let rust_symbol: Symbol<ContrastRatio> = rust_library
            .get(b"contrast_ratio\0")
            .expect("Rust contrast_ratio export is missing");
        test(*c_symbol, *rust_symbol);
    }
}

fn assert_same(
    c_contrast_ratio: ContrastRatio,
    rust_contrast_ratio: ContrastRatio,
    a: Rgb,
    b: Rgb,
    context: &str,
) {
    let c_result = unsafe { c_contrast_ratio(a, b) };
    let rust_result = unsafe { rust_contrast_ratio(a, b) };
    assert_eq!(
        c_result.to_ne_bytes(),
        rust_result.to_ne_bytes(),
        "{context}: A={a:?}, B={b:?}, C={c_result:?} ({:#010x}), Rust={rust_result:?} ({:#010x})",
        c_result.to_bits(),
        rust_result.to_bits(),
    );
}

fn linearize(channel: u8) -> f32 {
    let channel = f32::from(channel) / 255.0;
    if channel > 0.04045 {
        (((f64::from(channel) + 0.055) / 1.055).powf(2.4)) as f32
    } else {
        (f64::from(channel) / 12.92) as f32
    }
}

fn luminance(rgb: Rgb) -> f32 {
    0.2126 * linearize(rgb.R) + 0.7152 * linearize(rgb.G) + 0.0722 * linearize(rgb.B)
}

fn channel_for_mask(rng: &mut Rng, high: bool) -> u8 {
    if high {
        rng.byte_inclusive(HIGH_MIN, u8::MAX)
    } else {
        rng.byte_inclusive(0, LOW_MAX)
    }
}

fn random_pair_for_mask(rng: &mut Rng, mask: u8) -> (Rgb, Rgb) {
    let mut channels = [0_u8; 6];
    for (index, channel) in channels.iter_mut().enumerate() {
        let high = mask & (1 << (5 - index)) != 0;
        *channel = channel_for_mask(rng, high);
    }
    (
        Rgb {
            R: channels[0],
            G: channels[1],
            B: channels[2],
        },
        Rgb {
            R: channels[3],
            G: channels[4],
            B: channels[5],
        },
    )
}

fn endpoint_biased_pair(rng: &mut Rng, mask: u8, want_less: bool) -> (Rgb, Rgb) {
    let mut channels = [0_u8; 6];
    for (index, channel) in channels.iter_mut().enumerate() {
        let high = mask & (1 << (5 - index)) != 0;
        let (min, max) = if high {
            (HIGH_MIN, u8::MAX)
        } else {
            (0, LOW_MAX)
        };
        let belongs_to_a = index < 3;
        let toward_min = belongs_to_a == want_less;
        *channel = if toward_min {
            rng.byte_inclusive(min, min + 3)
        } else {
            rng.byte_inclusive(max - 3, max)
        };
    }
    (
        Rgb {
            R: channels[0],
            G: channels[1],
            B: channels[2],
        },
        Rgb {
            R: channels[3],
            G: channels[4],
            B: channels[5],
        },
    )
}

fn endpoint_pair(mask: u8, want_less: bool) -> (Rgb, Rgb) {
    let mut channels = [0_u8; 6];
    for (index, channel) in channels.iter_mut().enumerate() {
        let high = mask & (1 << (5 - index)) != 0;
        let (min, max) = if high {
            (HIGH_MIN, u8::MAX)
        } else {
            (0, LOW_MAX)
        };
        let belongs_to_a = index < 3;
        *channel = if belongs_to_a == want_less { min } else { max };
    }
    (
        Rgb {
            R: channels[0],
            G: channels[1],
            B: channels[2],
        },
        Rgb {
            R: channels[3],
            G: channels[4],
            B: channels[5],
        },
    )
}

fn row_id(mask: u8, less: bool) -> String {
    format!("{mask:02}{}", if less { 'a' } else { 'b' })
}

fn expected_configuration_rows() -> BTreeSet<String> {
    let mut rows = BTreeSet::new();
    for mask in 0_u8..64 {
        for less in [true, false] {
            let (a, b) = endpoint_pair(mask, less);
            if (luminance(a) < luminance(b)) == less {
                rows.insert(row_id(mask, less));
            }
        }
    }
    rows
}

fn documented_configuration_rows() -> BTreeSet<String> {
    fs::read_to_string(manifest_dir().join("CONFIGS.md"))
        .expect("failed to read CONFIGS.md")
        .lines()
        .filter_map(|line| {
            let id = line.strip_prefix("| ")?.split_whitespace().next()?;
            (id.len() == 3
                && id.as_bytes()[0].is_ascii_digit()
                && id.as_bytes()[1].is_ascii_digit()
                && matches!(id.as_bytes()[2], b'a' | b'b'))
            .then(|| id.to_owned())
        })
        .collect()
}

#[test]
fn phase_b_all_configuration_rows_match() {
    let expected_rows = expected_configuration_rows();
    assert_eq!(expected_rows.len(), 124);
    assert_eq!(documented_configuration_rows(), expected_rows);

    with_apis(|c_api, rust_api| {
        for mask in 0_u8..64 {
            for less in [true, false] {
                let id = row_id(mask, less);
                if !expected_rows.contains(&id) {
                    continue;
                }

                let mut rng = Rng::new(0x6a09_e667_f3bc_c909 ^ u64::from(mask) ^ u64::from(less));
                let mut matched = 0;
                for attempt in 0..2_000_000 {
                    let (a, b) = if attempt < 100_000 {
                        random_pair_for_mask(&mut rng, mask)
                    } else {
                        endpoint_biased_pair(&mut rng, mask, less)
                    };
                    if (luminance(a) < luminance(b)) != less {
                        continue;
                    }
                    assert_same(
                        c_api,
                        rust_api,
                        a,
                        b,
                        &format!("CONFIGS.md row {id}, randomized case {matched}"),
                    );
                    matched += 1;
                    if matched == CASES_PER_ROW {
                        break;
                    }
                    assert_ne!(
                        attempt, 1_999_999,
                        "could not generate enough inputs for row {id}"
                    );
                }
                assert_eq!(matched, CASES_PER_ROW, "insufficient cases for row {id}");
            }
        }
    });
}

#[test]
fn phase_b_boundaries_and_ieee_results_match() {
    with_apis(|c_api, rust_api| {
        let boundary_values = [0, LOW_MAX, HIGH_MIN, u8::MAX];
        for &ar in &boundary_values {
            for &ag in &boundary_values {
                for &ab in &boundary_values {
                    for &br in &boundary_values {
                        for &bg in &boundary_values {
                            for &bb in &boundary_values {
                                assert_same(
                                    c_api,
                                    rust_api,
                                    Rgb {
                                        R: ar,
                                        G: ag,
                                        B: ab,
                                    },
                                    Rgb {
                                        R: br,
                                        G: bg,
                                        B: bb,
                                    },
                                    "threshold boundary cross-product",
                                );
                            }
                        }
                    }
                }
            }
        }

        let black = Rgb { R: 0, G: 0, B: 0 };
        assert_same(c_api, rust_api, black, black, "zero divided by zero");
        assert_same(
            c_api,
            rust_api,
            Rgb {
                R: 255,
                G: 255,
                B: 255,
            },
            black,
            "nonzero divided by zero",
        );

        let mut rng = Rng::new(0xbb67_ae85_84ca_a73b);
        for case in 0..1_024 {
            let color = Rgb {
                R: rng.byte_inclusive(0, u8::MAX),
                G: rng.byte_inclusive(0, u8::MAX),
                B: rng.byte_inclusive(0, u8::MAX),
            };
            assert_same(
                c_api,
                rust_api,
                color,
                color,
                &format!("equal color {case}"),
            );
        }
    });
}

#[test]
fn phase_c_all_ffi_bit_patterns_are_valid() {
    // The C API has no rejectable input category. Exercise every value of every
    // ABI field independently to guard that the Rust wrapper accepts the same
    // complete unsigned-char domain.
    with_apis(|c_api, rust_api| {
        for field in 0..6 {
            for value in u8::MIN..=u8::MAX {
                let mut channels = [0_u8; 6];
                channels[field] = value;
                assert_same(
                    c_api,
                    rust_api,
                    Rgb {
                        R: channels[0],
                        G: channels[1],
                        B: channels[2],
                    },
                    Rgb {
                        R: channels[3],
                        G: channels[4],
                        B: channels[5],
                    },
                    &format!("ABI field {field}, bit pattern {value}"),
                );
            }
        }
    });
}

fn defined_dynamic_symbols(library: &Path) -> BTreeSet<String> {
    let output = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(library)
        .output()
        .unwrap_or_else(|error| panic!("failed to execute nm for {}: {error}", library.display()));
    assert!(
        output.status.success(),
        "nm failed for {}: {}",
        library.display(),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .expect("nm output was not UTF-8")
        .lines()
        .filter_map(|line| line.split_whitespace().nth(2).map(str::to_owned))
        .collect()
}

#[test]
fn phase_d_c_symbols_are_exported_by_rust() {
    let c_symbols = defined_dynamic_symbols(&c_library_path());
    let rust_symbols = defined_dynamic_symbols(&rust_library_path());
    let missing = c_symbols.difference(&rust_symbols).collect::<Vec<_>>();
    assert!(missing.is_empty(), "Rust is missing C symbols: {missing:?}");
}
