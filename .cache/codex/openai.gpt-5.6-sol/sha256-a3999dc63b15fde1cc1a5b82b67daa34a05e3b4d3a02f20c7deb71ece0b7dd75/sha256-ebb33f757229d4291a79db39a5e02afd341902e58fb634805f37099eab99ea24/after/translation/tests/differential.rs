use libloading::Library;
use std::path::{Path, PathBuf};

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct CbRgb255 {
    r: u8,
    g: u8,
    b: u8,
}

type Tritanopia = unsafe extern "C" fn(CbRgb255) -> CbRgb255;

struct LoadedLibraries {
    _c_library: Library,
    _rust_library: Library,
    c_tritanopia: Tritanopia,
    rust_tritanopia: Tritanopia,
}

impl LoadedLibraries {
    fn load() -> Self {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let workspace_dir = manifest_dir
            .parent()
            .expect("translation crate must have a workspace parent");
        let c_library_path = find_c_library(&workspace_dir.join("c_src").join("build"));
        let rust_library_path = manifest_dir.join("target").join("release").join(format!(
            "{}tritanopia_lib{}",
            std::env::consts::DLL_PREFIX,
            std::env::consts::DLL_SUFFIX
        ));

        assert!(
            rust_library_path.is_file(),
            "Rust cdylib does not exist at {}; run cargo build --release first",
            rust_library_path.display()
        );

        unsafe {
            let c_library = Library::new(&c_library_path).unwrap_or_else(|error| {
                panic!(
                    "failed to load C library {}: {error}",
                    c_library_path.display()
                )
            });
            let rust_library = Library::new(&rust_library_path).unwrap_or_else(|error| {
                panic!(
                    "failed to load Rust library {}: {error}",
                    rust_library_path.display()
                )
            });
            let c_tritanopia = *c_library
                .get::<Tritanopia>(b"tritanopia\0")
                .expect("C library must export tritanopia");
            let rust_tritanopia = *rust_library
                .get::<Tritanopia>(b"tritanopia\0")
                .expect("Rust library must export tritanopia");

            Self {
                _c_library: c_library,
                _rust_library: rust_library,
                c_tritanopia,
                rust_tritanopia,
            }
        }
    }

    fn compare(&self, input: CbRgb255) {
        let c_output = unsafe { (self.c_tritanopia)(input) };
        let rust_output = unsafe { (self.rust_tritanopia)(input) };
        assert_eq!(
            rgb_bytes(c_output),
            rgb_bytes(rust_output),
            "differential mismatch for input {input:?}: C={c_output:?}, Rust={rust_output:?}"
        );
    }
}

fn find_c_library(build_dir: &Path) -> PathBuf {
    let mut candidates: Vec<PathBuf> = std::fs::read_dir(build_dir)
        .unwrap_or_else(|error| {
            panic!(
                "failed to read C build directory {}: {error}",
                build_dir.display()
            )
        })
        .map(|entry| entry.expect("failed to inspect C build entry").path())
        .filter(|path| {
            path.is_file()
                && path.extension().and_then(|extension| extension.to_str())
                    == Some(&std::env::consts::DLL_SUFFIX[1..])
        })
        .collect();
    candidates.sort();
    assert_eq!(
        candidates.len(),
        1,
        "expected exactly one C shared library in {}, found {candidates:?}",
        build_dir.display()
    );
    candidates.pop().unwrap()
}

fn rgb_bytes(rgb: CbRgb255) -> [u8; 3] {
    [rgb.r, rgb.g, rgb.b]
}

fn next_random(state: &mut u64) -> u64 {
    let mut value = *state;
    value ^= value << 13;
    value ^= value >> 7;
    value ^= value << 17;
    *state = value;
    value
}

fn sample_channel(state: &mut u64, power_gamma: bool) -> u8 {
    if power_gamma {
        11 + (next_random(state) % 245) as u8
    } else {
        (next_random(state) % 11) as u8
    }
}

fn branch_boundaries(power_gamma: bool) -> [u8; 2] {
    if power_gamma { [11, 255] } else { [0, 10] }
}

#[test]
fn every_configuration_row_matches_on_boundaries_and_randomized_inputs() {
    assert_eq!(
        std::mem::size_of::<CbRgb255>(),
        3,
        "the FFI struct must be byte-identical to three C unsigned chars"
    );
    let libraries = LoadedLibraries::load();
    let mut random_state = 0x4d59_5df4_d0f3_3173_u64;

    for configuration in 0_u8..8 {
        let r_power_gamma = configuration & 0b100 != 0;
        let g_power_gamma = configuration & 0b010 != 0;
        let b_power_gamma = configuration & 0b001 != 0;

        for r in branch_boundaries(r_power_gamma) {
            for g in branch_boundaries(g_power_gamma) {
                for b in branch_boundaries(b_power_gamma) {
                    libraries.compare(CbRgb255 { r, g, b });
                }
            }
        }

        for _ in 0..16_384 {
            libraries.compare(CbRgb255 {
                r: sample_channel(&mut random_state, r_power_gamma),
                g: sample_channel(&mut random_state, g_power_gamma),
                b: sample_channel(&mut random_state, b_power_gamma),
            });
        }
    }
}

#[test]
fn all_possible_inputs_match_byte_for_byte() {
    let libraries = LoadedLibraries::load();

    for r in 0_u16..=255 {
        for g in 0_u16..=255 {
            for b in 0_u16..=255 {
                libraries.compare(CbRgb255 {
                    r: r as u8,
                    g: g as u8,
                    b: b as u8,
                });
            }
        }
    }
}
