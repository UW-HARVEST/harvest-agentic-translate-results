use libloading::Library;
use std::path::{Path, PathBuf};
use std::process::Command;

type Crc16 = unsafe extern "C" fn(*const u8, u32, u16) -> u16;

struct Implementations {
    _c_library: Library,
    _rust_library: Library,
    c_crc16: Crc16,
    rust_crc16: Crc16,
}

impl Implementations {
    fn load() -> Self {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_path = manifest_dir.join("../c_src/build/libharvest-work-9jqAfK.so");
        let target_dir = std::env::var_os("CARGO_TARGET_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| manifest_dir.join("target"));
        let rust_path = target_dir.join("release").join("libcrc16_lib.so");

        assert_library_exists(&c_path);
        assert_library_exists(&rust_path);

        unsafe {
            let c_library = Library::new(&c_path)
                .unwrap_or_else(|error| panic!("failed to load {}: {error}", c_path.display()));
            let rust_library = Library::new(&rust_path)
                .unwrap_or_else(|error| panic!("failed to load {}: {error}", rust_path.display()));
            let c_crc16 = *c_library
                .get::<Crc16>(b"crc16\0")
                .expect("C library does not export crc16");
            let rust_crc16 = *rust_library
                .get::<Crc16>(b"crc16\0")
                .expect("Rust library does not export crc16");

            Self {
                _c_library: c_library,
                _rust_library: rust_library,
                c_crc16,
                rust_crc16,
            }
        }
    }

    fn compare(&self, bytes: &[u8], initial_crc: u16) {
        let len = u32::try_from(bytes.len()).expect("test input exceeds the C length type");
        let c_result = unsafe { (self.c_crc16)(bytes.as_ptr(), len, initial_crc) };
        let rust_result = unsafe { (self.rust_crc16)(bytes.as_ptr(), len, initial_crc) };
        assert_eq!(
            rust_result, c_result,
            "CRC mismatch for len={len}, initial_crc={initial_crc:#06x}, bytes={bytes:02x?}"
        );
    }
}

fn assert_library_exists(path: &Path) {
    assert!(
        path.is_file(),
        "shared library not found at {}; build both libraries first",
        path.display()
    );
}

#[derive(Clone, Copy)]
struct XorShift64(u64);

impl XorShift64 {
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

    fn next_u16(&mut self) -> u16 {
        self.next_u64() as u16
    }

    fn fill(&mut self, bytes: &mut [u8]) {
        for byte in bytes {
            *byte = self.next_u64() as u8;
        }
    }
}

fn compare_randomized_lengths(lengths: &[usize], cases_per_length: usize, seed: u64) {
    let implementations = Implementations::load();
    let mut random = XorShift64::new(seed);

    for &len in lengths {
        for case in 0..cases_per_length {
            let mut bytes = vec![0; len];
            random.fill(&mut bytes);
            let initial_crc = match case {
                0 => 0,
                1 => u16::MAX,
                _ => random.next_u16(),
            };
            implementations.compare(&bytes, initial_crc);
        }
    }
}

#[test]
fn config_1_zero_length_null_and_non_null() {
    let implementations = Implementations::load();
    let mut random = XorShift64::new(0x8f3c_17a5_21d9_b604);

    for case in 0..1_024 {
        let initial_crc = match case {
            0 => 0,
            1 => u16::MAX,
            _ => random.next_u16(),
        };
        let c_null = unsafe { (implementations.c_crc16)(std::ptr::null(), 0, initial_crc) };
        let rust_null = unsafe { (implementations.rust_crc16)(std::ptr::null(), 0, initial_crc) };
        assert_eq!(rust_null, c_null);
        assert_eq!(rust_null, initial_crc);
        implementations.compare(&[], initial_crc);
    }
}

#[test]
fn config_2_tail_only_lengths_1_through_7() {
    compare_randomized_lengths(&(1..=7).collect::<Vec<_>>(), 512, 0x9ed6_51c3_f424_0a77);
}

#[test]
fn config_3_exactly_one_eight_byte_slice() {
    compare_randomized_lengths(&[8], 2_048, 0x75e2_a189_6bc4_d03f);
}

#[test]
fn config_4_one_slice_with_tail() {
    compare_randomized_lengths(&(9..=15).collect::<Vec<_>>(), 512, 0xc391_04de_872b_65fa);
}

#[test]
fn config_5_multiple_slices_without_tail() {
    compare_randomized_lengths(&[16, 24, 32, 64, 256, 1_024], 512, 0x2da8_f710_5c96_b34e);
}

#[test]
fn config_6_multiple_slices_with_tail() {
    compare_randomized_lengths(
        &[17, 18, 23, 25, 31, 33, 63, 255, 1_025],
        512,
        0xb467_3ac0_1e95_f82d,
    );
}

#[test]
fn large_representable_length_matches() {
    compare_randomized_lengths(&[1_048_576, 1_048_583], 4, 0xe025_9b7c_4613_ad8f);
}

#[test]
fn null_nonzero_and_oversized_boundaries_match_process_outcome() {
    for len in [1_u32, 8, u32::MAX] {
        let c_status = run_null_probe("c", len);
        let rust_status = run_null_probe("rust", len);
        assert_eq!(
            rust_status, c_status,
            "process outcome differs for null pointer and len={len}"
        );
        assert!(
            !c_status.success(),
            "C unexpectedly accepted null with len={len}"
        );
    }
}

fn run_null_probe(implementation: &str, len: u32) -> std::process::ExitStatus {
    Command::new(std::env::current_exe().expect("current test executable"))
        .args(["--exact", "null_pointer_probe_child", "--nocapture"])
        .env("CRC16_NULL_PROBE", implementation)
        .env("CRC16_NULL_PROBE_LEN", len.to_string())
        .status()
        .expect("failed to run null-pointer probe")
}

#[test]
fn null_pointer_probe_child() {
    let Ok(implementation) = std::env::var("CRC16_NULL_PROBE") else {
        return;
    };
    let len = std::env::var("CRC16_NULL_PROBE_LEN")
        .expect("probe length")
        .parse::<u32>()
        .expect("valid probe length");
    let implementations = Implementations::load();
    let function = match implementation.as_str() {
        "c" => implementations.c_crc16,
        "rust" => implementations.rust_crc16,
        other => panic!("unknown probe implementation {other}"),
    };

    let result = unsafe { function(std::ptr::null(), len, 0x5a5a) };
    panic!("null-pointer probe unexpectedly returned {result:#06x}");
}
