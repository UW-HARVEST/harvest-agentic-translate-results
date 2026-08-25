use libloading::Library;
use std::ffi::c_int;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr;
use std::sync::Once;

type HdrCompare = unsafe extern "C" fn(*const u8, *const u8) -> c_int;

const CASES_PER_ROW: usize = 1_024;
static BUILD_RUST_CDYLIB: Once = Once::new();

struct Libraries {
    _c: Library,
    _rust: Library,
    c_hdr_compare: HdrCompare,
    rust_hdr_compare: HdrCompare,
}

impl Libraries {
    fn load() -> Self {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let c_path = root.join("c_src/build/libtranslated_rust.so");
        let rust_path = rust_library_path(root);

        BUILD_RUST_CDYLIB.call_once(|| build_rust_cdylib(root));
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
            let c = Library::new(&c_path)
                .unwrap_or_else(|error| panic!("failed to load {}: {error}", c_path.display()));
            let rust = Library::new(&rust_path)
                .unwrap_or_else(|error| panic!("failed to load {}: {error}", rust_path.display()));
            let c_hdr_compare = *c
                .get::<HdrCompare>(b"hdr_compare\0")
                .expect("C library does not export hdr_compare");
            let rust_hdr_compare = *rust
                .get::<HdrCompare>(b"hdr_compare\0")
                .expect("Rust library does not export hdr_compare");

            Self {
                _c: c,
                _rust: rust,
                c_hdr_compare,
                rust_hdr_compare,
            }
        }
    }

    fn compare(&self, h1: Option<&[u8; 3]>, h2: &[u8; 3], expected: c_int) {
        let h1 = h1.map_or(ptr::null(), |header| header.as_ptr());
        unsafe {
            let c_result = (self.c_hdr_compare)(h1, h2.as_ptr());
            let rust_result = (self.rust_hdr_compare)(h1, h2.as_ptr());
            assert_eq!(c_result.to_ne_bytes(), rust_result.to_ne_bytes());
            assert_eq!(c_result, expected);
        }
    }
}

fn build_rust_cdylib(root: &Path) {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let mut command = Command::new(cargo);
    command
        .current_dir(root)
        .args(["build", "--no-default-features"]);
    if !cfg!(debug_assertions) {
        command.arg("--release");
    }

    let status = command.status().expect("failed to run cargo build");
    assert!(status.success(), "cargo build failed with {status}");
}

fn rust_library_path(root: &Path) -> PathBuf {
    let target = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("target"));
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    target.join(profile).join("libhdr_compare_lib.so")
}

struct Rng(u64);

impl Rng {
    fn new(row: u64) -> Self {
        Self(0x5eed_c0de_d15c_a11u64 ^ row.wrapping_mul(0x9e37_79b9_7f4a_7c15))
    }

    fn byte(&mut self) -> u8 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0 as u8
    }

    fn header(&mut self) -> [u8; 3] {
        [self.byte(), self.byte(), self.byte()]
    }

    fn choose(&mut self, values: &[u8]) -> u8 {
        values[usize::from(self.byte()) % values.len()]
    }
}

fn sync_accepted(byte: u8) -> bool {
    byte & 0xf0 == 0xf0 || byte & 0xfe == 0xe2
}

fn version_valid(byte: u8) -> bool {
    (byte >> 1) & 3 != 0
}

fn byte2_valid(byte: u8) -> bool {
    byte >> 4 != 15 && (byte >> 2) & 3 != 3
}

fn valid_sync_byte(rng: &mut Rng) -> u8 {
    loop {
        let byte = rng.byte();
        if sync_accepted(byte) && version_valid(byte) {
            return byte;
        }
    }
}

fn valid_byte2(rng: &mut Rng) -> u8 {
    loop {
        let byte = rng.byte();
        if byte2_valid(byte) {
            return byte;
        }
    }
}

fn valid_h2(rng: &mut Rng) -> [u8; 3] {
    [0xff, valid_sync_byte(rng), valid_byte2(rng)]
}

#[test]
fn config_01_invalid_first_sync_byte_short_circuits_h1() {
    let libs = Libraries::load();
    let mut rng = Rng::new(1);
    for case in 0..CASES_PER_ROW {
        let mut h2 = rng.header();
        while h2[0] == 0xff {
            h2[0] = rng.byte();
        }
        let h1 = rng.header();
        libs.compare(Some(&h1), &h2, 0);
        if case < 64 {
            libs.compare(None, &h2, 0);
        }
    }
}

#[test]
fn config_02_invalid_second_sync_byte() {
    let libs = Libraries::load();
    let mut rng = Rng::new(2);
    for _ in 0..CASES_PER_ROW {
        let mut h2 = rng.header();
        h2[0] = 0xff;
        while sync_accepted(h2[1]) {
            h2[1] = rng.byte();
        }
        libs.compare(Some(&rng.header()), &h2, 0);
    }
}

#[test]
fn config_03_f_sync_with_zero_version() {
    let libs = Libraries::load();
    let mut rng = Rng::new(3);
    for _ in 0..CASES_PER_ROW {
        let h2 = [0xff, rng.choose(&[0xf0, 0xf1, 0xf8, 0xf9]), rng.byte()];
        libs.compare(Some(&rng.header()), &h2, 0);
    }
}

#[test]
fn config_04_e_sync_branch_is_valid() {
    let libs = Libraries::load();
    let mut rng = Rng::new(4);
    for _ in 0..CASES_PER_ROW {
        let h2 = [0xff, rng.choose(&[0xe2, 0xe3]), valid_byte2(&mut rng)];
        let h1 = [rng.byte(), h2[1] ^ (rng.byte() & 1), h2[2]];
        libs.compare(Some(&h1), &h2, 1);
    }
}

#[test]
fn config_05_reserved_bitrate_nibble() {
    let libs = Libraries::load();
    let mut rng = Rng::new(5);
    for _ in 0..CASES_PER_ROW {
        let h2 = [0xff, valid_sync_byte(&mut rng), 0xf0 | (rng.byte() & 0x0f)];
        libs.compare(Some(&rng.header()), &h2, 0);
    }
}

#[test]
fn config_06_reserved_layer_bits() {
    let libs = Libraries::load();
    let mut rng = Rng::new(6);
    for _ in 0..CASES_PER_ROW {
        let h2 = [
            0xff,
            valid_sync_byte(&mut rng),
            ((rng.byte() % 15) << 4) | 0x0c | (rng.byte() & 3),
        ];
        libs.compare(Some(&rng.header()), &h2, 0);
    }
}

#[test]
fn config_07_masked_byte_one_mismatch() {
    let libs = Libraries::load();
    let mut rng = Rng::new(7);
    for _ in 0..CASES_PER_ROW {
        let h2 = valid_h2(&mut rng);
        let mut h1 = rng.header();
        while (h1[1] ^ h2[1]) & 0xfe == 0 {
            h1[1] = rng.byte();
        }
        libs.compare(Some(&h1), &h2, 0);
    }
}

#[test]
fn config_08_byte_one_lsb_is_ignored() {
    let libs = Libraries::load();
    let mut rng = Rng::new(8);
    for _ in 0..CASES_PER_ROW {
        let h2 = valid_h2(&mut rng);
        let h1 = [rng.byte(), h2[1] ^ 1, h2[2]];
        libs.compare(Some(&h1), &h2, 1);
    }
}

#[test]
fn config_09_layer_mismatch() {
    let libs = Libraries::load();
    let mut rng = Rng::new(9);
    for _ in 0..CASES_PER_ROW {
        let h2 = valid_h2(&mut rng);
        let different_layer = ((h2[2] >> 2) + 1) & 3;
        let h1 = [
            rng.byte(),
            h2[1] ^ (rng.byte() & 1),
            (rng.byte() & !0x0c) | (different_layer << 2),
        ];
        libs.compare(Some(&h1), &h2, 0);
    }
}

#[test]
fn config_10_both_bitrate_nibbles_nonzero() {
    let libs = Libraries::load();
    let mut rng = Rng::new(10);
    for _ in 0..CASES_PER_ROW {
        let mut h2 = valid_h2(&mut rng);
        h2[2] = ((rng.byte() % 14) + 1) << 4 | (h2[2] & 0x0f);
        let h1_bitrate = (h2[2] >> 4) % 14 + 1;
        let h1 = [
            rng.byte(),
            h2[1] ^ (rng.byte() & 1),
            (h1_bitrate << 4) | (rng.byte() & 0x03) | (h2[2] & 0x0c),
        ];
        libs.compare(Some(&h1), &h2, 1);
    }
}

#[test]
fn config_11_h1_zero_h2_nonzero_bitrate() {
    let libs = Libraries::load();
    let mut rng = Rng::new(11);
    for _ in 0..CASES_PER_ROW {
        let mut h2 = valid_h2(&mut rng);
        h2[2] = ((rng.byte() % 14) + 1) << 4 | (h2[2] & 0x0f);
        let h1 = [
            rng.byte(),
            h2[1] ^ (rng.byte() & 1),
            (rng.byte() & 0x03) | (h2[2] & 0x0c),
        ];
        libs.compare(Some(&h1), &h2, 0);
    }
}

#[test]
fn config_12_h1_nonzero_h2_zero_bitrate() {
    let libs = Libraries::load();
    let mut rng = Rng::new(12);
    for _ in 0..CASES_PER_ROW {
        let mut h2 = valid_h2(&mut rng);
        h2[2] &= 0x0f;
        let h1 = [
            rng.byte(),
            h2[1] ^ (rng.byte() & 1),
            ((rng.byte() % 15) + 1) << 4 | (rng.byte() & 0x03) | (h2[2] & 0x0c),
        ];
        libs.compare(Some(&h1), &h2, 0);
    }
}

#[test]
fn config_13_both_bitrate_nibbles_zero() {
    let libs = Libraries::load();
    let mut rng = Rng::new(13);
    for _ in 0..CASES_PER_ROW {
        let mut h2 = valid_h2(&mut rng);
        h2[2] &= 0x0f;
        let h1 = [
            rng.byte(),
            h2[1] ^ (rng.byte() & 1),
            (rng.byte() & 0x03) | (h2[2] & 0x0c),
        ];
        libs.compare(Some(&h1), &h2, 1);
    }
}
