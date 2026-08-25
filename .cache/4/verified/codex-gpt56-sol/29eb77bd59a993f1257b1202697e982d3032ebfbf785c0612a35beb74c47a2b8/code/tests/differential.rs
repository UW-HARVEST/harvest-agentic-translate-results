use libloading::Library;
use std::path::{Path, PathBuf};
use std::ptr;

type Crc16 = unsafe extern "C" fn(*const u8, u32, u16) -> u16;

struct Implementations {
    _c_library: Library,
    _rust_library: Library,
    c_crc16: Crc16,
    rust_crc16: Crc16,
}

impl Implementations {
    fn load() -> Self {
        let c_path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("c_src/build/libtranslated_rust.so");
        let rust_path = rust_library_path();

        assert!(
            c_path.is_file(),
            "C shared library is missing at {}; build c_src first",
            c_path.display()
        );
        assert!(
            rust_path.is_file(),
            "Rust shared library is missing at {}",
            rust_path.display()
        );

        // SAFETY: Both paths name libraries built from this repository. Copying
        // the function pointers is valid while the libraries remain in Self.
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

    fn assert_match(&self, data: &[u8], initial_crc: u16) {
        self.assert_pointer_match(data.as_ptr(), data.len() as u32, initial_crc);
    }

    fn assert_pointer_match(&self, data: *const u8, len: u32, initial_crc: u16) {
        // SAFETY: Tests pass either a slice with at least len readable bytes or
        // a null pointer with len zero, matching the C API's memory contract.
        let (c_result, rust_result) = unsafe {
            (
                (self.c_crc16)(data, len, initial_crc),
                (self.rust_crc16)(data, len, initial_crc),
            )
        };

        assert_eq!(
            c_result.to_ne_bytes(),
            rust_result.to_ne_bytes(),
            "CRC mismatch for len={len}, initial_crc={initial_crc:#06x}"
        );
    }
}

fn rust_library_path() -> PathBuf {
    if let Some(path) = std::env::var_os("CRC16_RUST_LIB") {
        return path.into();
    }

    let test_executable = std::env::current_exe().expect("cannot locate test executable");
    test_executable
        .parent()
        .and_then(Path::parent)
        .expect("test executable is not under target/<profile>/deps")
        .join("libcrc16_lib.so")
}

struct DeterministicRng(u64);

impl DeterministicRng {
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

fn exercise_lengths(seed: u64, lengths: impl Iterator<Item = usize>) {
    let implementations = Implementations::load();
    let mut rng = DeterministicRng::new(seed);

    for len in lengths {
        let mut data = vec![0; len];
        rng.fill(&mut data);
        implementations.assert_match(&data, rng.next_u16());
    }
}

#[test]
fn config_1_empty_input() {
    let implementations = Implementations::load();
    let mut rng = DeterministicRng::new(0x1357_9bdf_2468_ace1);

    for _ in 0..256 {
        let initial_crc = rng.next_u16();
        implementations.assert_pointer_match(ptr::null(), 0, initial_crc);

        let unused_data = [rng.next_u64() as u8];
        implementations.assert_pointer_match(unused_data.as_ptr(), 0, initial_crc);
    }
}

#[test]
fn config_2_remainder_only_lengths() {
    exercise_lengths(
        0x52a4_f83d_19c7_06eb,
        (0..512).map(|iteration| iteration % 7 + 1),
    );
}

#[test]
fn config_3_exactly_one_eight_byte_slice() {
    exercise_lengths(0xa941_70cb_3e62_d85f, std::iter::repeat_n(8, 512));
}

#[test]
fn config_4_one_slice_with_remainder() {
    exercise_lengths(
        0x7042_cabd_e195_386f,
        (0..512).map(|iteration| iteration % 7 + 9),
    );
}

#[test]
fn config_5_multiple_slices_without_remainder() {
    exercise_lengths(
        0x3e81_b4d7_605a_29cf,
        (0..512).map(|iteration| (iteration % 64 + 2) * 8),
    );
}

#[test]
fn config_6_multiple_slices_with_remainder() {
    exercise_lengths(
        0xc790_52e8_1b64_af3d,
        (0..512).map(|iteration| {
            let slice_count = iteration % 64 + 2;
            let remainder = iteration % 7 + 1;
            slice_count * 8 + remainder
        }),
    );
}

#[test]
fn boundary_zero_length_with_null_and_non_null_pointers() {
    let implementations = Implementations::load();
    for initial_crc in [0, 1, 0x7fff, 0x8000, 0xfffe, 0xffff] {
        implementations.assert_pointer_match(ptr::null(), 0, initial_crc);
        implementations.assert_pointer_match([0xa5].as_ptr(), 0, initial_crc);
    }
}

#[test]
fn boundary_large_valid_length() {
    const LARGE_LENGTH: usize = 1024 * 1024 + 7;
    exercise_lengths(0xfedc_ba98_7654_3210, std::iter::once(LARGE_LENGTH));
}
