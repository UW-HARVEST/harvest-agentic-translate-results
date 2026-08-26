use libloading::Library;
use std::path::{Path, PathBuf};

type MaxSizeFrame = unsafe extern "C" fn(u32, u32, u32) -> u32;

const RANDOM_CASES: usize = 4_096;

struct Implementations {
    _c_library: Library,
    _rust_library: Library,
    c: MaxSizeFrame,
    rust: MaxSizeFrame,
}

impl Implementations {
    fn load() -> Self {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let c_path = manifest_dir.join("c_src/build/libtranslated_rust.so");
        let rust_path = rust_library_path();

        assert!(
            c_path.is_file(),
            "C shared library missing: {}",
            c_path.display()
        );
        assert!(
            rust_path.is_file(),
            "Rust shared library missing: {}",
            rust_path.display()
        );

        unsafe {
            let c_library = Library::new(&c_path)
                .unwrap_or_else(|error| panic!("failed to load {}: {error}", c_path.display()));
            let rust_library = Library::new(&rust_path)
                .unwrap_or_else(|error| panic!("failed to load {}: {error}", rust_path.display()));
            let c = *c_library
                .get::<MaxSizeFrame>(b"max_size_frame\0")
                .expect("C symbol max_size_frame is missing");
            let rust = *rust_library
                .get::<MaxSizeFrame>(b"max_size_frame\0")
                .expect("Rust symbol max_size_frame is missing");

            Self {
                _c_library: c_library,
                _rust_library: rust_library,
                c,
                rust,
            }
        }
    }

    fn assert_match(&self, row: usize, blocksize: u32, channels: u32, bitdepth: u32) {
        unsafe {
            let c_result = (self.c)(blocksize, channels, bitdepth);
            let rust_result = (self.rust)(blocksize, channels, bitdepth);
            assert_eq!(
                rust_result, c_result,
                "CONFIGS.md row {row}: blocksize={blocksize}, channels={channels}, \
                 bitdepth={bitdepth}"
            );
        }
    }
}

fn rust_library_path() -> PathBuf {
    let test_executable = std::env::current_exe().expect("cannot locate test executable");
    let profile_dir = test_executable
        .parent()
        .and_then(Path::parent)
        .expect("test executable is not under target/<profile>/deps");
    profile_dir.join(format!(
        "{}max_size_frame_lib{}",
        std::env::consts::DLL_PREFIX,
        std::env::consts::DLL_SUFFIX
    ))
}

#[derive(Clone, Copy)]
struct XorShift64(u64);

impl XorShift64 {
    fn new(seed: u64) -> Self {
        assert_ne!(seed, 0);
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

#[test]
fn row_1_non_stereo_ordinary_arithmetic() {
    let implementations = Implementations::load();
    let channels = [0, 1, 3, 4, 8];
    let boundaries = [(0, 0, 0), (1, 1, 1), (4_095, 3, 32), (4_095, 8, 64)];
    for &(blocksize, channel_count, bitdepth) in &boundaries {
        implementations.assert_match(1, blocksize, channel_count, bitdepth);
    }

    let mut random = XorShift64::new(0x5a17_0a01_d15c_a001);
    for _ in 0..RANDOM_CASES {
        let blocksize = random.next_u32() % 4_096;
        let channel_count = channels[random.next_u32() as usize % channels.len()];
        let bitdepth = random.next_u32() % 65;
        implementations.assert_match(1, blocksize, channel_count, bitdepth);
    }
}

#[test]
fn row_2_non_stereo_wrapping_arithmetic() {
    let implementations = Implementations::load();
    let boundaries = [
        (u32::MAX, u32::MAX, u32::MAX),
        (u32::MAX, 1, u32::MAX),
        (u32::MAX - 1, 3, u32::MAX - 1),
        (1 << 31, 1 << 31, 1 << 31),
    ];
    for &(blocksize, channels, bitdepth) in &boundaries {
        implementations.assert_match(2, blocksize, channels, bitdepth);
    }

    let mut random = XorShift64::new(0x5a17_0a02_d15c_a002);
    for _ in 0..RANDOM_CASES {
        let blocksize = random.next_u32() | 0x8000_0000;
        let channels = random.next_u32() | 0x8000_0003;
        let bitdepth = random.next_u32() | 0x8000_0000;
        implementations.assert_match(2, blocksize, channels, bitdepth);
    }
}

#[test]
fn row_3_stereo_32_bit_ordinary_arithmetic() {
    let implementations = Implementations::load();
    for blocksize in [0, 1, 2, 65_535] {
        implementations.assert_match(3, blocksize, 2, 32);
    }

    let mut random = XorShift64::new(0x5a17_0a03_d15c_a003);
    for _ in 0..RANDOM_CASES {
        implementations.assert_match(3, random.next_u32() % 65_536, 2, 32);
    }
}

#[test]
fn row_4_stereo_32_bit_wrapping_arithmetic() {
    let implementations = Implementations::load();
    for blocksize in [67_108_864, 1 << 31, u32::MAX - 1, u32::MAX] {
        implementations.assert_match(4, blocksize, 2, 32);
    }

    let mut random = XorShift64::new(0x5a17_0a04_d15c_a004);
    for _ in 0..RANDOM_CASES {
        let blocksize = random.next_u32() | 0x1000_0000;
        implementations.assert_match(4, blocksize, 2, 32);
    }
}

#[test]
fn row_5_stereo_non_32_bit_ordinary_arithmetic() {
    let implementations = Implementations::load();
    let bitdepths = [0, 1, 2, 31, 33];
    for blocksize in [0, 1, 2, 65_535] {
        for bitdepth in bitdepths {
            implementations.assert_match(5, blocksize, 2, bitdepth);
        }
    }

    let mut random = XorShift64::new(0x5a17_0a05_d15c_a005);
    for _ in 0..RANDOM_CASES {
        let blocksize = random.next_u32() % 65_536;
        let bitdepth = bitdepths[random.next_u32() as usize % bitdepths.len()];
        implementations.assert_match(5, blocksize, 2, bitdepth);
    }
}

#[test]
fn row_6_stereo_non_32_bit_wrapping_arithmetic() {
    let implementations = Implementations::load();
    let boundaries = [
        (u32::MAX, u32::MAX),
        (u32::MAX, u32::MAX - 1),
        (u32::MAX - 1, u32::MAX),
        (1 << 31, 1 << 31),
    ];
    for &(blocksize, bitdepth) in &boundaries {
        implementations.assert_match(6, blocksize, 2, bitdepth);
    }

    let mut random = XorShift64::new(0x5a17_0a06_d15c_a006);
    for _ in 0..RANDOM_CASES {
        let blocksize = random.next_u32() | 0x8000_0000;
        let mut bitdepth = random.next_u32() | 0x8000_0000;
        if bitdepth == 32 {
            bitdepth = u32::MAX;
        }
        implementations.assert_match(6, blocksize, 2, bitdepth);
    }
}
