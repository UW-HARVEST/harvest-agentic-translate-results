use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};
use std::ptr;

type Crc16 = unsafe extern "C" fn(*const u8, u32, u16) -> u16;

struct Implementations {
    c: Library,
    rust: Library,
}

impl Implementations {
    fn load() -> Self {
        let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_path = crate_dir.join("../c_src/build/libharvest-work-9JQpsO.so");
        let rust_path = crate_dir.join("target/release/libcrc16_lib.so");

        assert_library_exists(&c_path);
        assert_library_exists(&rust_path);

        Self {
            c: unsafe { Library::new(&c_path) }
                .unwrap_or_else(|error| panic!("failed to load {}: {error}", c_path.display())),
            rust: unsafe { Library::new(&rust_path) }
                .unwrap_or_else(|error| panic!("failed to load {}: {error}", rust_path.display())),
        }
    }

    fn compare(&self, data: &[u8], initial_crc: u16) {
        let length = u32::try_from(data.len()).expect("test input length fits u32");
        self.compare_raw(data.as_ptr(), length, initial_crc);
    }

    fn compare_raw(&self, data: *const u8, length: u32, initial_crc: u16) {
        let c_crc16: Symbol<'_, Crc16> = unsafe { self.c.get(b"crc16\0") }.expect("C crc16 export");
        let rust_crc16: Symbol<'_, Crc16> =
            unsafe { self.rust.get(b"crc16\0") }.expect("Rust crc16 export");

        let c_result = unsafe { c_crc16(data, length, initial_crc) };
        let rust_result = unsafe { rust_crc16(data, length, initial_crc) };

        assert_eq!(
            rust_result, c_result,
            "FFI result mismatch for length={length}, initial_crc={initial_crc:#06x}"
        );
    }
}

fn assert_library_exists(path: &Path) {
    assert!(
        path.is_file(),
        "required shared library does not exist: {}; build both libraries first",
        path.display()
    );
}

#[derive(Clone)]
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

    fn next_u16(&mut self) -> u16 {
        self.next_u64() as u16
    }

    fn fill(&mut self, bytes: &mut [u8]) {
        for chunk in bytes.chunks_mut(8) {
            let random = self.next_u64().to_le_bytes();
            chunk.copy_from_slice(&random[..chunk.len()]);
        }
    }
}

#[test]
fn config_1_empty_input() {
    let implementations = Implementations::load();
    let mut random = SplitMix64::new(0x6a09_e667_f3bc_c909);
    let non_null = [0xa5_u8];

    for iteration in 0..4096 {
        let pointer = if iteration % 2 == 0 {
            ptr::null()
        } else {
            non_null.as_ptr()
        };
        implementations.compare_raw(pointer, 0, random.next_u16());
    }
}

#[test]
fn config_2_tail_only_lengths_1_through_7() {
    let implementations = Implementations::load();
    let mut random = SplitMix64::new(0xbb67_ae85_84ca_a73b);

    for length in 1..=7 {
        for _ in 0..2048 {
            let mut data = vec![0_u8; length];
            random.fill(&mut data);
            implementations.compare(&data, random.next_u16());
        }
    }
}

#[test]
fn config_3_exactly_one_eight_byte_slice() {
    let implementations = Implementations::load();
    let mut random = SplitMix64::new(0x3c6e_f372_fe94_f82b);

    for _ in 0..16_384 {
        let mut data = [0_u8; 8];
        random.fill(&mut data);
        implementations.compare(&data, random.next_u16());
    }
}

#[test]
fn config_4_one_slice_plus_tail_lengths_9_through_15() {
    let implementations = Implementations::load();
    let mut random = SplitMix64::new(0xa54f_f53a_5f1d_36f1);

    for length in 9..=15 {
        for _ in 0..2048 {
            let mut data = vec![0_u8; length];
            random.fill(&mut data);
            implementations.compare(&data, random.next_u16());
        }
    }
}

#[test]
fn config_5_multiple_slices_with_and_without_tail() {
    let implementations = Implementations::load();
    let mut random = SplitMix64::new(0x510e_527f_ade6_82d1);

    for remainder in 0..8 {
        for slices in [2_usize, 3, 4, 7, 8, 15, 16, 31, 64, 127] {
            let length = slices * 8 + remainder;
            for _ in 0..64 {
                let mut data = vec![0_u8; length];
                random.fill(&mut data);
                implementations.compare(&data, random.next_u16());
            }
        }
    }

    for _ in 0..4096 {
        let length = 16 + (random.next_u64() as usize % (1024 * 1024 - 15));
        let mut data = vec![0_u8; length];
        random.fill(&mut data);
        implementations.compare(&data, random.next_u16());
    }
}

#[test]
fn generic_boundary_zero_length_preserves_all_crc_extremes() {
    let implementations = Implementations::load();
    let byte = [0x5a_u8];

    for initial_crc in [0_u16, 1, 0x7fff, 0x8000, 0xfffe, 0xffff] {
        implementations.compare_raw(ptr::null(), 0, initial_crc);
        implementations.compare_raw(byte.as_ptr(), 0, initial_crc);
    }
}

#[test]
fn generic_boundary_large_valid_length() {
    let implementations = Implementations::load();
    let mut random = SplitMix64::new(0x1f83_d9ab_fb41_bd6b);
    let mut data = vec![0_u8; 16 * 1024 * 1024 + 7];
    random.fill(&mut data);

    for initial_crc in [0_u16, 0x1234, 0xffff] {
        implementations.compare(&data, initial_crc);
    }
}
