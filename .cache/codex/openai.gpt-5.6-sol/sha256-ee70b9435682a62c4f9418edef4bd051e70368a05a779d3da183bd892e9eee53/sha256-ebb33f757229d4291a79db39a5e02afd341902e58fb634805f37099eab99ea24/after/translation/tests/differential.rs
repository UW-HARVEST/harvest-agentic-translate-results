use libloading::Library;
use std::path::{Path, PathBuf};

type Rev16 = unsafe extern "C" fn(u32) -> u32;

struct Implementations {
    _c_library: Library,
    _rust_library: Library,
    c_rev16: Rev16,
    rust_rev16: Rev16,
}

impl Implementations {
    fn load() -> Self {
        let c_path = c_library_path();
        let rust_path = rust_library_path();

        // SAFETY: Both paths name shared libraries built from this workspace.
        let c_library = unsafe {
            Library::new(&c_path).unwrap_or_else(|error| {
                panic!("failed to load C library {}: {error}", c_path.display())
            })
        };
        // SAFETY: Both paths name shared libraries built from this workspace.
        let rust_library = unsafe {
            Library::new(&rust_path).unwrap_or_else(|error| {
                panic!(
                    "failed to load Rust library {}: {error}",
                    rust_path.display()
                )
            })
        };

        // Copy the function pointers while retaining both libraries for the
        // lifetime of this structure.
        let c_rev16 = unsafe {
            *c_library
                .get::<Rev16>(b"rev16\0")
                .expect("C library does not export rev16")
        };
        let rust_rev16 = unsafe {
            *rust_library
                .get::<Rev16>(b"rev16\0")
                .expect("Rust library does not export rev16")
        };

        Self {
            _c_library: c_library,
            _rust_library: rust_library,
            c_rev16,
            rust_rev16,
        }
    }

    fn assert_match(&self, input: u32) {
        // SAFETY: rev16 takes and returns a uint32_t by value and has no other
        // preconditions in either implementation.
        let c_output = unsafe { (self.c_rev16)(input) };
        // SAFETY: Same ABI and argument contract as the C function.
        let rust_output = unsafe { (self.rust_rev16)(input) };
        assert_eq!(
            rust_output.to_ne_bytes(),
            c_output.to_ne_bytes(),
            "rev16 byte divergence for input {input:#010x}: Rust={rust_output:#010x}, C={c_output:#010x}"
        );
    }
}

fn c_library_path() -> PathBuf {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation crate must be inside the workspace");
    let build_dir = workspace.join("c_src").join("build");
    let mut candidates = std::fs::read_dir(&build_dir)
        .unwrap_or_else(|error| {
            panic!(
                "failed to read C build directory {}: {error}",
                build_dir.display()
            )
        })
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension().is_some_and(|extension| extension == "so")
                && path
                    .file_name()
                    .is_some_and(|name| name.to_string_lossy().starts_with("lib"))
        })
        .collect::<Vec<_>>();
    candidates.sort();
    assert_eq!(
        candidates.len(),
        1,
        "expected exactly one C shared library in {}, found {candidates:?}",
        build_dir.display()
    );
    candidates.pop().unwrap()
}

fn rust_library_path() -> PathBuf {
    let test_executable = std::env::current_exe().expect("test executable path is unavailable");
    let profile_dir = test_executable
        .parent()
        .and_then(Path::parent)
        .expect("integration test must run from target/<profile>/deps");
    let path = profile_dir.join("librev16_lib.so");
    assert!(
        path.is_file(),
        "Rust cdylib was not built at {}",
        path.display()
    );
    path
}

fn next_u32(state: &mut u32) -> u32 {
    let mut value = *state;
    value ^= value << 13;
    value ^= value >> 17;
    value ^= value << 5;
    *state = value;
    value
}

#[test]
fn config_1_low_16_bit_domain_matches() {
    let implementations = Implementations::load();

    for input in 0..=u16::MAX {
        implementations.assert_match(u32::from(input));
    }

    let mut state = 0x6d2b_79f5;
    for _ in 0..50_000 {
        implementations.assert_match(next_u32(&mut state) & 0x0000_ffff);
    }
}

#[test]
fn config_2_nonzero_upper_half_matches() {
    let implementations = Implementations::load();
    let mut state = 0xa341_316c;

    for _ in 0..100_000 {
        let random = next_u32(&mut state);
        let upper = ((random >> 16) | 1) << 16;
        let input = upper | (next_u32(&mut state) & 0x0000_ffff);
        implementations.assert_match(input);
    }
}

#[test]
fn config_3_boundary_focused_inputs_match() {
    let implementations = Implementations::load();
    let low_halves = [0_u32, 1, 0x7fff, 0x8000, 0xffff];
    let exact_boundaries = [
        0_u32,
        1,
        0x7fff,
        0x8000,
        0xffff,
        0x1_0000,
        0x1_0001,
        u32::MAX,
    ];

    for input in exact_boundaries {
        implementations.assert_match(input);
    }

    let mut state = 0xc801_3ea4;
    for _ in 0..20_000 {
        let upper = next_u32(&mut state) & 0xffff_0000;
        for low in low_halves {
            implementations.assert_match(upper | low);
        }
    }
}

#[test]
fn phase_c_scalar_boundaries_are_valid_and_match() {
    let implementations = Implementations::load();
    for input in [u32::MIN, 1, 0xffff, 0x1_0000, 0x1_0001, u32::MAX] {
        implementations.assert_match(input);
    }
}
