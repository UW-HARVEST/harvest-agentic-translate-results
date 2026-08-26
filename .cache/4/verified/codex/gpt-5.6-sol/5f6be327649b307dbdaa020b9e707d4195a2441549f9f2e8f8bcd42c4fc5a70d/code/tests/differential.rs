use libloading::Library;
use std::ffi::c_int;
use std::path::{Path, PathBuf};

type Jumpnode = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

struct Libraries {
    _c_library: Library,
    _rust_library: Library,
    c_jumpnode: Jumpnode,
    rust_jumpnode: Jumpnode,
}

impl Libraries {
    fn load() -> Self {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let c_path = manifest_dir.join("c_src/build/libtranslated_rust.so");
        let rust_path = rust_library_path(manifest_dir);

        assert!(
            c_path.is_file(),
            "C shared library not found at {}",
            c_path.display()
        );
        assert!(
            rust_path.is_file(),
            "Rust shared library not found at {}",
            rust_path.display()
        );

        unsafe {
            let c_library = Library::new(&c_path)
                .unwrap_or_else(|error| panic!("failed to load {}: {error}", c_path.display()));
            let rust_library = Library::new(&rust_path)
                .unwrap_or_else(|error| panic!("failed to load {}: {error}", rust_path.display()));

            let c_jumpnode = *c_library
                .get::<Jumpnode>(b"jumpnode\0")
                .expect("C library does not export jumpnode");
            let rust_jumpnode = *rust_library
                .get::<Jumpnode>(b"jumpnode\0")
                .expect("Rust library does not export jumpnode");

            Self {
                _c_library: c_library,
                _rust_library: rust_library,
                c_jumpnode,
                rust_jumpnode,
            }
        }
    }

    fn compare(&self, mode: c_int, node_id: c_int, depth: c_int, flags: c_int) {
        let c_result = unsafe { (self.c_jumpnode)(mode, node_id, depth, flags) };
        let rust_result = unsafe { (self.rust_jumpnode)(mode, node_id, depth, flags) };
        assert_eq!(
            rust_result, c_result,
            "mode={mode}, node_id={node_id}, depth={depth}, flags={flags}"
        );
    }

    fn compare_expected(
        &self,
        mode: c_int,
        node_id: c_int,
        depth: c_int,
        flags: c_int,
        expected: c_int,
    ) {
        let c_result = unsafe { (self.c_jumpnode)(mode, node_id, depth, flags) };
        let rust_result = unsafe { (self.rust_jumpnode)(mode, node_id, depth, flags) };
        assert_eq!(
            c_result, expected,
            "unexpected C result for mode={mode}, node_id={node_id}, depth={depth}, flags={flags}"
        );
        assert_eq!(
            rust_result, expected,
            "unexpected Rust result for mode={mode}, node_id={node_id}, depth={depth}, flags={flags}"
        );
    }
}

fn rust_library_path(manifest_dir: &Path) -> PathBuf {
    let target_dir = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| manifest_dir.join("target"));
    target_dir.join("debug/libjumpnode_lib.so")
}

struct FixedRng(u64);

impl FixedRng {
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

    fn next_i32(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }

    fn inclusive_u64(&mut self, lower: u64, upper: u64) -> u64 {
        lower + self.next_u64() % (upper - lower + 1)
    }
}

fn random_with_decimal_len(rng: &mut FixedRng, len: usize) -> i32 {
    assert!((1..=11).contains(&len));

    let positive_range = match len {
        1 => Some((0, 9)),
        2..=9 => Some((10_u64.pow((len - 1) as u32), 10_u64.pow(len as u32) - 1)),
        10 => Some((1_000_000_000, i32::MAX as u64)),
        11 => None,
        _ => unreachable!(),
    };
    let negative_magnitude_range = match len {
        1 => None,
        2..=10 => Some((
            10_u64.pow((len - 2) as u32),
            10_u64.pow((len - 1) as u32) - 1,
        )),
        11 => Some((1_000_000_000, 2_147_483_648)),
        _ => unreachable!(),
    };

    let use_negative =
        negative_magnitude_range.is_some() && (positive_range.is_none() || rng.next_u64() & 1 == 1);
    if use_negative {
        let (lower, upper) = negative_magnitude_range.unwrap();
        let magnitude = rng.inclusive_u64(lower, upper);
        if magnitude == 2_147_483_648 {
            i32::MIN
        } else {
            -(magnitude as i32)
        }
    } else {
        let (lower, upper) = positive_range.unwrap();
        rng.inclusive_u64(lower, upper) as i32
    }
}

fn random_arguments(rng: &mut FixedRng) -> (i32, i32, i32) {
    (rng.next_i32(), rng.next_i32(), rng.next_i32())
}

#[test]
fn config_c1_mode_one_empty_store() {
    let libraries = Libraries::load();
    let mut rng = FixedRng::new(0xc100_37e1_93d4_108b);
    for _ in 0..4096 {
        let (node_id, depth, flags) = random_arguments(&mut rng);
        libraries.compare(0o1, node_id, depth, flags);
    }
}

#[test]
fn config_c2_mode_two_empty_store() {
    let libraries = Libraries::load();
    let mut rng = FixedRng::new(0xc200_f5ca_277a_a883);
    for _ in 0..4096 {
        let (node_id, depth, flags) = random_arguments(&mut rng);
        libraries.compare(0o2, node_id, depth, flags);
    }
}

#[test]
fn config_c3_mode_three_decimal_shapes_and_low_flags() {
    let libraries = Libraries::load();
    let mut rng = FixedRng::new(0xc300_a149_700e_12fb);

    for node_len in 1..=11 {
        for depth_len in 1..=11 {
            for low_flags in 0..=0o177 {
                let node_id = random_with_decimal_len(&mut rng, node_len);
                let depth = random_with_decimal_len(&mut rng, depth_len);
                assert_eq!(node_id.to_string().len(), node_len);
                assert_eq!(depth.to_string().len(), depth_len);
                libraries.compare(0o3, node_id, depth, low_flags);
            }
        }
    }
}

#[test]
fn config_c4_mode_three_masks_high_flag_bits() {
    let libraries = Libraries::load();
    let fixed_flags = [128, 256, 1024, -128, -256, i32::MIN, i32::MAX & !0o177];

    for flags in fixed_flags {
        libraries.compare(0o3, i32::MIN, i32::MAX, flags);
    }

    let mut rng = FixedRng::new(0xc400_6be8_2681_dd5d);
    for _ in 0..4096 {
        let flags = rng.next_i32() & !0o177;
        libraries.compare(0o3, rng.next_i32(), rng.next_i32(), flags);
    }
}

#[test]
fn config_c5_mode_three_random_full_domain() {
    let libraries = Libraries::load();
    let mut rng = FixedRng::new(0xc500_b0d7_5331_f6c9);
    for _ in 0..16_384 {
        let (node_id, depth, flags) = random_arguments(&mut rng);
        libraries.compare(0o3, node_id, depth, flags);
    }
}

#[test]
fn config_c6_mode_four_empty_store() {
    let libraries = Libraries::load();
    let mut rng = FixedRng::new(0xc600_c052_711a_4f3b);
    for _ in 0..4096 {
        let (node_id, depth, flags) = random_arguments(&mut rng);
        libraries.compare(0o4, node_id, depth, flags);
    }
}

#[test]
fn config_c7_default_arm_immediate_boundaries() {
    let libraries = Libraries::load();
    let values = [i32::MIN, -1, 0, 1, i32::MAX];
    for mode in [0, 5] {
        for node_id in values {
            for depth in values {
                for flags in values {
                    libraries.compare(mode, node_id, depth, flags);
                }
            }
        }
    }
}

#[test]
fn config_c8_default_arm_random_out_of_range_modes() {
    let libraries = Libraries::load();
    let mut rng = FixedRng::new(0xc800_93f1_335f_aed7);

    for mode in [i32::MIN, -1, 0, 5, i32::MAX] {
        libraries.compare(mode, rng.next_i32(), rng.next_i32(), rng.next_i32());
    }

    for _ in 0..8192 {
        let mut mode = rng.next_i32();
        while (1..=4).contains(&mode) {
            mode = rng.next_i32();
        }
        libraries.compare(mode, rng.next_i32(), rng.next_i32(), rng.next_i32());
    }
}

#[test]
fn error_e1_mode_one_missing_node_is_exact() {
    let libraries = Libraries::load();
    let mut rng = FixedRng::new(0xe100_dbca_c32c_010f);
    for _ in 0..4096 {
        let (node_id, depth, flags) = random_arguments(&mut rng);
        libraries.compare_expected(0o1, node_id, depth, flags, 0o22);
    }
}

#[test]
fn error_e2_mode_two_missing_node_is_exact() {
    let libraries = Libraries::load();
    let mut rng = FixedRng::new(0xe200_ef36_d406_52eb);
    for _ in 0..4096 {
        let (node_id, depth, flags) = random_arguments(&mut rng);
        libraries.compare_expected(0o2, node_id, depth, flags, 0o42);
    }
}

#[test]
fn error_e3_mode_four_missing_node_is_exact() {
    let libraries = Libraries::load();
    let mut rng = FixedRng::new(0xe300_4693_2ac4_80a7);
    for _ in 0..4096 {
        let (node_id, depth, flags) = random_arguments(&mut rng);
        libraries.compare_expected(0o4, node_id, depth, flags, 0o102);
    }
}

#[test]
fn error_e4_invalid_mode_is_exact() {
    let libraries = Libraries::load();
    let mut rng = FixedRng::new(0xe400_70ae_685c_7713);

    for mode in [i32::MIN, -1, 0, 5, i32::MAX] {
        libraries.compare_expected(mode, i32::MIN, 0, i32::MAX, 0o202);
    }

    for _ in 0..4096 {
        let mut mode = rng.next_i32();
        while (1..=4).contains(&mode) {
            mode = rng.next_i32();
        }
        libraries.compare_expected(mode, rng.next_i32(), rng.next_i32(), rng.next_i32(), 0o202);
    }
}
