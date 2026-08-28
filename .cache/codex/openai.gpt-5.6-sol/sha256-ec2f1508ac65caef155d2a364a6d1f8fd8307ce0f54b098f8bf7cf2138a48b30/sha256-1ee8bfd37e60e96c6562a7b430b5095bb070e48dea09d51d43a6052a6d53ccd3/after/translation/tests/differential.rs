use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};

type MaxSizeFrame = unsafe extern "C" fn(u32, u32, u32) -> u32;

struct Implementations {
    c: Library,
    rust: Library,
}

impl Implementations {
    fn load() -> Self {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_path = manifest_dir.join("../c_src/build/libharvest-work-GY1cd5.so");
        let rust_path = target_dir(&manifest_dir)
            .join("release")
            .join("libmax_size_frame_lib.so");

        assert!(
            c_path.is_file(),
            "C shared library is missing: {}",
            c_path.display()
        );
        assert!(
            rust_path.is_file(),
            "Rust shared library is missing: {}",
            rust_path.display()
        );

        // SAFETY: Both paths are build outputs controlled by this test workspace.
        unsafe {
            Self {
                c: Library::new(c_path).expect("load C shared library"),
                rust: Library::new(rust_path).expect("load Rust shared library"),
            }
        }
    }

    fn compare(&self, blocksize: u32, channels: u32, bitdepth: u32) {
        // SAFETY: The public C header and Rust export define this exact signature.
        let (c_result, rust_result) = unsafe {
            let c_fn: Symbol<'_, MaxSizeFrame> = self
                .c
                .get(b"max_size_frame\0")
                .expect("resolve C max_size_frame");
            let rust_fn: Symbol<'_, MaxSizeFrame> = self
                .rust
                .get(b"max_size_frame\0")
                .expect("resolve Rust max_size_frame");
            (
                c_fn(blocksize, channels, bitdepth),
                rust_fn(blocksize, channels, bitdepth),
            )
        };

        assert_eq!(
            c_result.to_ne_bytes(),
            rust_result.to_ne_bytes(),
            "blocksize={blocksize}, channels={channels}, bitdepth={bitdepth}"
        );
    }
}

fn target_dir(manifest_dir: &Path) -> PathBuf {
    std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| manifest_dir.join("target"))
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

    fn next_except(&mut self, excluded: u32) -> u32 {
        loop {
            let value = self.next_u32();
            if value != excluded {
                return value;
            }
        }
    }
}

const BOUNDARIES: [u32; 4] = [0, 1, u32::MAX - 1, u32::MAX];
const NON_32_BITDEPTHS: [u32; 8] = [0, 1, 2, 31, 33, 255, u32::MAX - 1, u32::MAX];
const NON_STEREO_CHANNELS: [u32; 8] = [0, 1, 3, 4, 255, 256, u32::MAX - 1, u32::MAX];
const RANDOM_CASES: usize = 8_192;

#[test]
fn config_1_stereo_32_bit() {
    let implementations = Implementations::load();
    let mut rng = FixedRng::new(0x8aa8_0117_ba5e_ba11);

    for blocksize in BOUNDARIES {
        implementations.compare(blocksize, 2, 32);
    }
    for _ in 0..RANDOM_CASES {
        implementations.compare(rng.next_u32(), 2, 32);
    }
}

#[test]
fn config_2_stereo_non_32_bit() {
    let implementations = Implementations::load();
    let mut rng = FixedRng::new(0xe43d_ba86_2137_669a);

    for blocksize in BOUNDARIES {
        for bitdepth in NON_32_BITDEPTHS {
            implementations.compare(blocksize, 2, bitdepth);
        }
    }
    for _ in 0..RANDOM_CASES {
        implementations.compare(rng.next_u32(), 2, rng.next_except(32));
    }
}

#[test]
fn config_3_non_stereo_32_bit() {
    let implementations = Implementations::load();
    let mut rng = FixedRng::new(0x31fb_762c_7998_b353);

    for blocksize in BOUNDARIES {
        for channels in NON_STEREO_CHANNELS {
            implementations.compare(blocksize, channels, 32);
        }
    }
    for _ in 0..RANDOM_CASES {
        implementations.compare(rng.next_u32(), rng.next_except(2), 32);
    }
}

#[test]
fn config_4_non_stereo_non_32_bit() {
    let implementations = Implementations::load();
    let mut rng = FixedRng::new(0xdde0_2cc3_38ba_6b05);

    for blocksize in BOUNDARIES {
        for channels in NON_STEREO_CHANNELS {
            for bitdepth in NON_32_BITDEPTHS {
                implementations.compare(blocksize, channels, bitdepth);
            }
        }
    }
    for _ in 0..RANDOM_CASES {
        implementations.compare(rng.next_u32(), rng.next_except(2), rng.next_except(32));
    }
}
