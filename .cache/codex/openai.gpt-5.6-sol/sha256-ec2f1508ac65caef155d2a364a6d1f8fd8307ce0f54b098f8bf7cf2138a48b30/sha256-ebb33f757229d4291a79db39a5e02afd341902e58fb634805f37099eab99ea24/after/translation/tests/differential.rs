use libloading::Library;
use std::fs;
use std::path::{Path, PathBuf};

type MaxSizeFrame = unsafe extern "C" fn(u32, u32, u32) -> u32;

struct DifferentialApis {
    _c_library: Library,
    _rust_library: Library,
    c_max_size_frame: MaxSizeFrame,
    rust_max_size_frame: MaxSizeFrame,
}

impl DifferentialApis {
    fn load() -> Self {
        let c_path = c_library_path();
        let rust_path = rust_library_path();

        // SAFETY: Both paths identify shared libraries built by this workspace.
        let c_library = unsafe { Library::new(&c_path) }.unwrap_or_else(|error| {
            panic!("failed to load C library {}: {error}", c_path.display())
        });
        // SAFETY: Both paths identify shared libraries built by this workspace.
        let rust_library = unsafe { Library::new(&rust_path) }.unwrap_or_else(|error| {
            panic!(
                "failed to load Rust library {}: {error}",
                rust_path.display()
            )
        });

        // SAFETY: The public C header declares this exact symbol and signature.
        let c_max_size_frame = unsafe {
            *c_library
                .get::<MaxSizeFrame>(b"max_size_frame\0")
                .expect("C library does not export max_size_frame")
        };
        // SAFETY: The translation must expose the same C ABI and signature.
        let rust_max_size_frame = unsafe {
            *rust_library
                .get::<MaxSizeFrame>(b"max_size_frame\0")
                .expect("Rust library does not export max_size_frame")
        };

        Self {
            _c_library: c_library,
            _rust_library: rust_library,
            c_max_size_frame,
            rust_max_size_frame,
        }
    }

    fn assert_match(&self, blocksize: u32, channels: u32, bitdepth: u32) {
        // SAFETY: This function has only by-value u32 parameters and both pointers
        // were resolved with the signature declared by the public C header.
        let c_result = unsafe { (self.c_max_size_frame)(blocksize, channels, bitdepth) };
        // SAFETY: Same reasoning as the C call above.
        let rust_result = unsafe { (self.rust_max_size_frame)(blocksize, channels, bitdepth) };

        assert_eq!(
            c_result.to_ne_bytes(),
            rust_result.to_ne_bytes(),
            "byte mismatch for blocksize={blocksize}, channels={channels}, bitdepth={bitdepth}; \
             C={c_result}, Rust={rust_result}"
        );
    }
}

fn c_library_path() -> PathBuf {
    let build_dir = manifest_dir()
        .parent()
        .expect("translation crate must have a parent")
        .join("c_src")
        .join("build");
    let mut candidates: Vec<PathBuf> = fs::read_dir(&build_dir)
        .unwrap_or_else(|error| {
            panic!(
                "failed to read C build directory {}: {error}; build the C library first",
                build_dir.display()
            )
        })
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "so"))
        .collect();
    candidates.sort();
    assert_eq!(
        candidates.len(),
        1,
        "expected exactly one C shared library in {}, found {candidates:?}",
        build_dir.display()
    );
    candidates.remove(0)
}

fn rust_library_path() -> PathBuf {
    let path = manifest_dir()
        .join("target")
        .join("release")
        .join("libmax_size_frame_lib.so");
    assert!(
        path.is_file(),
        "Rust shared library {} is missing; run cargo build --release first",
        path.display()
    );
    path
}

fn manifest_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

struct FixedRng(u64);

impl FixedRng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u32(&mut self) -> u32 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value as u32
    }
}

fn except(value: u32, excluded: u32) -> u32 {
    if value == excluded {
        value.wrapping_add(1)
    } else {
        value
    }
}

#[test]
fn config_1_stereo_bitdepth_32() {
    let apis = DifferentialApis::load();
    for blocksize in [0, 1, 2, 3, 0x07ff_ffff, 0x0800_0000, u32::MAX - 1, u32::MAX] {
        apis.assert_match(blocksize, 2, 32);
    }

    let mut rng = FixedRng::new(0x58d2_72f2_a4bf_3c91);
    for _ in 0..25_000 {
        apis.assert_match(rng.next_u32(), 2, 32);
    }
}

#[test]
fn config_2_stereo_bitdepth_not_32() {
    let apis = DifferentialApis::load();
    for blocksize in [0, 1, 2, 0x0800_0000, u32::MAX] {
        for bitdepth in [0, 1, 2, 31, 33, u32::MAX - 1, u32::MAX] {
            apis.assert_match(blocksize, 2, bitdepth);
        }
    }

    let mut rng = FixedRng::new(0xe4a9_a921_a286_e47b);
    for _ in 0..25_000 {
        let blocksize = rng.next_u32();
        let bitdepth = except(rng.next_u32(), 32);
        apis.assert_match(blocksize, 2, bitdepth);
    }
}

#[test]
fn config_3_non_stereo_bitdepth_32() {
    let apis = DifferentialApis::load();
    for blocksize in [0, 1, 2, 0x0800_0000, u32::MAX] {
        for channels in [0, 1, 3, 4, u32::MAX - 1, u32::MAX] {
            apis.assert_match(blocksize, channels, 32);
        }
    }

    let mut rng = FixedRng::new(0x80dd_f26f_5425_a9d1);
    for _ in 0..25_000 {
        let blocksize = rng.next_u32();
        let channels = except(rng.next_u32(), 2);
        apis.assert_match(blocksize, channels, 32);
    }
}

#[test]
fn config_4_non_stereo_bitdepth_not_32() {
    let apis = DifferentialApis::load();
    for blocksize in [0, 1, 2, 0x0800_0000, u32::MAX] {
        for channels in [0, 1, 3, u32::MAX] {
            for bitdepth in [0, 1, 31, 33, u32::MAX] {
                apis.assert_match(blocksize, channels, bitdepth);
            }
        }
    }

    let mut rng = FixedRng::new(0xa51e_9b4c_2d03_771f);
    for _ in 0..25_000 {
        let blocksize = rng.next_u32();
        let channels = except(rng.next_u32(), 2);
        let bitdepth = except(rng.next_u32(), 32);
        apis.assert_match(blocksize, channels, bitdepth);
    }
}

#[test]
fn phase_c_generic_scalar_boundaries_are_valid_and_match() {
    let apis = DifferentialApis::load();
    let boundaries = [0, 1, 2, 3, 31, 32, 33, u32::MAX - 1, u32::MAX];

    for blocksize in boundaries {
        for channels in boundaries {
            for bitdepth in boundaries {
                apis.assert_match(blocksize, channels, bitdepth);
            }
        }
    }
}
