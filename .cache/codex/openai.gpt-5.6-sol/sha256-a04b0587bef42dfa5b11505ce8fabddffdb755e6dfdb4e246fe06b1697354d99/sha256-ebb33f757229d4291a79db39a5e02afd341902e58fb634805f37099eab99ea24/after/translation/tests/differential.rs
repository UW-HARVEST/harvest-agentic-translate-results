use libloading::Library;
use std::ffi::c_int;
use std::path::Path;

type DivEuclid = unsafe extern "C" fn(c_int, c_int) -> c_int;

const RANDOM_CASES: usize = 2_048;

struct APIs {
    _c_library: Library,
    _rust_library: Library,
    c_div_euclid: DivEuclid,
    rust_div_euclid: DivEuclid,
}

impl APIs {
    fn load() -> Self {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let c_path = manifest_dir.join("../c_src/build/libharvest-work-qjBfdv.so");
        let rust_path = manifest_dir.join("target/release/libdiv_euclid_lib.so");

        assert_library_exists(&c_path, "C");
        assert_library_exists(&rust_path, "Rust");

        // SAFETY: Both paths name shared libraries built for this process'
        // target. The copied function pointers remain valid because the
        // libraries are retained in this struct for at least as long.
        unsafe {
            let c_library = Library::new(&c_path)
                .unwrap_or_else(|error| panic!("load {}: {error}", c_path.display()));
            let rust_library = Library::new(&rust_path)
                .unwrap_or_else(|error| panic!("load {}: {error}", rust_path.display()));
            let c_div_euclid = *c_library
                .get::<DivEuclid>(b"div_euclid\0")
                .expect("load C div_euclid export");
            let rust_div_euclid = *rust_library
                .get::<DivEuclid>(b"div_euclid\0")
                .expect("load Rust div_euclid export");

            Self {
                _c_library: c_library,
                _rust_library: rust_library,
                c_div_euclid,
                rust_div_euclid,
            }
        }
    }

    fn compare(&self, row: &str, v1: i32, v2: i32) {
        // SAFETY: The symbols have the declared C ABI and accept two scalar
        // C ints, so every i32 bit pattern is a valid FFI argument.
        let c_result = unsafe { (self.c_div_euclid)(v1, v2) };
        let rust_result = unsafe { (self.rust_div_euclid)(v1, v2) };

        assert_eq!(
            c_result.to_ne_bytes(),
            rust_result.to_ne_bytes(),
            "{row} diverged for div_euclid({v1}, {v2}): C={c_result}, Rust={rust_result}"
        );
    }
}

fn assert_library_exists(path: &Path, implementation: &str) {
    assert!(
        path.is_file(),
        "{implementation} shared library is missing at {}; build it before running tests",
        path.display()
    );
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

fn nonnegative(rng: &mut XorShift64) -> i32 {
    (rng.next_u32() & i32::MAX as u32) as i32
}

fn positive(rng: &mut XorShift64) -> i32 {
    (rng.next_u32() % i32::MAX as u32 + 1) as i32
}

fn positive_at_least_two(rng: &mut XorShift64) -> i32 {
    (rng.next_u32() % (i32::MAX as u32 - 1) + 2) as i32
}

fn negative_non_min(rng: &mut XorShift64) -> i32 {
    -positive(rng)
}

fn exact_magnitudes(rng: &mut XorShift64) -> (i32, i32) {
    let divisor = positive(rng);
    let max_quotient = i32::MAX / divisor;
    let quotient = (rng.next_u32() % max_quotient as u32 + 1) as i32;
    (divisor * quotient, divisor)
}

fn nonexact_magnitudes(rng: &mut XorShift64) -> (i32, i32) {
    loop {
        let divisor = positive_at_least_two(rng);
        let dividend = positive(rng);
        if dividend % divisor != 0 {
            return (dividend, divisor);
        }
    }
}

fn min_magnitude_nondivisor(rng: &mut XorShift64) -> i32 {
    loop {
        let divisor = positive(rng);
        if (1_u64 << 31) % divisor as u64 != 0 {
            return divisor;
        }
    }
}

#[test]
fn config_01_nonnegative_positive_direct_division() {
    let apis = APIs::load();
    for &(v1, v2) in &[
        (0, 1),
        (0, i32::MAX),
        (1, 1),
        (i32::MAX, 1),
        (i32::MAX, i32::MAX),
    ] {
        apis.compare("CONFIGS row 1", v1, v2);
    }
    let mut rng = XorShift64::new(0x01d1_5ea5_e001);
    for _ in 0..RANDOM_CASES {
        apis.compare("CONFIGS row 1", nonnegative(&mut rng), positive(&mut rng));
    }
}

#[test]
fn config_02_nonnegative_negative_general() {
    let apis = APIs::load();
    for &(v1, v2) in &[
        (0, -1),
        (0, -i32::MAX),
        (1, -1),
        (i32::MAX, -1),
        (i32::MAX, -i32::MAX),
    ] {
        apis.compare("CONFIGS row 2", v1, v2);
    }
    let mut rng = XorShift64::new(0x02d1_5ea5_e002);
    for _ in 0..RANDOM_CASES {
        apis.compare(
            "CONFIGS row 2",
            nonnegative(&mut rng),
            negative_non_min(&mut rng),
        );
    }
}

#[test]
fn config_03_nonnegative_divisor_minimum() {
    let apis = APIs::load();
    for &v1 in &[0, 1, i32::MAX] {
        apis.compare("CONFIGS row 3", v1, i32::MIN);
    }
    let mut rng = XorShift64::new(0x03d1_5ea5_e003);
    for _ in 0..RANDOM_CASES {
        apis.compare("CONFIGS row 3", nonnegative(&mut rng), i32::MIN);
    }
}

#[test]
fn config_04_negative_positive_exact() {
    let apis = APIs::load();
    for &(v1, v2) in &[(-1, 1), (-i32::MAX, 1), (-i32::MAX, i32::MAX)] {
        apis.compare("CONFIGS row 4", v1, v2);
    }
    let mut rng = XorShift64::new(0x04d1_5ea5_e004);
    for _ in 0..RANDOM_CASES {
        let (magnitude, divisor) = exact_magnitudes(&mut rng);
        apis.compare("CONFIGS row 4", -magnitude, divisor);
    }
}

#[test]
fn config_05_negative_positive_nonexact() {
    let apis = APIs::load();
    for &(v1, v2) in &[(-1, 2), (-i32::MAX, 2), (-i32::MAX, i32::MAX - 1)] {
        apis.compare("CONFIGS row 5", v1, v2);
    }
    let mut rng = XorShift64::new(0x05d1_5ea5_e005);
    for _ in 0..RANDOM_CASES {
        let (magnitude, divisor) = nonexact_magnitudes(&mut rng);
        apis.compare("CONFIGS row 5", -magnitude, divisor);
    }
}

#[test]
fn config_06_negative_negative_exact() {
    let apis = APIs::load();
    for &(v1, v2) in &[(-1, -1), (-i32::MAX, -1), (-i32::MAX, -i32::MAX)] {
        apis.compare("CONFIGS row 6", v1, v2);
    }
    let mut rng = XorShift64::new(0x06d1_5ea5_e006);
    for _ in 0..RANDOM_CASES {
        let (magnitude, divisor) = exact_magnitudes(&mut rng);
        apis.compare("CONFIGS row 6", -magnitude, -divisor);
    }
}

#[test]
fn config_07_negative_negative_nonexact() {
    let apis = APIs::load();
    for &(v1, v2) in &[(-1, -2), (-i32::MAX, -2), (-i32::MAX, -(i32::MAX - 1))] {
        apis.compare("CONFIGS row 7", v1, v2);
    }
    let mut rng = XorShift64::new(0x07d1_5ea5_e007);
    for _ in 0..RANDOM_CASES {
        let (magnitude, divisor) = nonexact_magnitudes(&mut rng);
        apis.compare("CONFIGS row 7", -magnitude, -divisor);
    }
}

#[test]
fn config_08_negative_divisor_minimum() {
    let apis = APIs::load();
    for &v1 in &[-1, -2, -i32::MAX] {
        apis.compare("CONFIGS row 8", v1, i32::MIN);
    }
    let mut rng = XorShift64::new(0x08d1_5ea5_e008);
    for _ in 0..RANDOM_CASES {
        apis.compare("CONFIGS row 8", negative_non_min(&mut rng), i32::MIN);
    }
}

#[test]
fn config_09_dividend_minimum_positive_exact() {
    let apis = APIs::load();
    for exponent in 0..=30 {
        let divisor = 1_i32 << exponent;
        apis.compare("CONFIGS row 9", i32::MIN, divisor);
    }
    let mut rng = XorShift64::new(0x09d1_5ea5_e009);
    for _ in 0..RANDOM_CASES {
        let divisor = 1_i32 << (rng.next_u32() % 31);
        apis.compare("CONFIGS row 9", i32::MIN, divisor);
    }
}

#[test]
fn config_10_dividend_minimum_positive_nonexact() {
    let apis = APIs::load();
    for &v2 in &[3, 5, i32::MAX] {
        apis.compare("CONFIGS row 10", i32::MIN, v2);
    }
    let mut rng = XorShift64::new(0x0ad1_5ea5_e00a);
    for _ in 0..RANDOM_CASES {
        apis.compare(
            "CONFIGS row 10",
            i32::MIN,
            min_magnitude_nondivisor(&mut rng),
        );
    }
}

#[test]
fn config_11_dividend_minimum_negative_exact() {
    let apis = APIs::load();
    for exponent in 0..=30 {
        let divisor = -(1_i32 << exponent);
        apis.compare("CONFIGS row 11", i32::MIN, divisor);
    }
    let mut rng = XorShift64::new(0x0bd1_5ea5_e00b);
    for _ in 0..RANDOM_CASES {
        let divisor = -(1_i32 << (rng.next_u32() % 31));
        apis.compare("CONFIGS row 11", i32::MIN, divisor);
    }
}

#[test]
fn config_12_dividend_minimum_negative_nonexact() {
    let apis = APIs::load();
    for &v2 in &[-3, -5, -i32::MAX] {
        apis.compare("CONFIGS row 12", i32::MIN, v2);
    }
    let mut rng = XorShift64::new(0x0cd1_5ea5_e00c);
    for _ in 0..RANDOM_CASES {
        apis.compare(
            "CONFIGS row 12",
            i32::MIN,
            -min_magnitude_nondivisor(&mut rng),
        );
    }
}

#[test]
fn config_13_both_minimum() {
    let apis = APIs::load();
    apis.compare("CONFIGS row 13", i32::MIN, i32::MIN);
}

#[test]
fn error_01_zero_divisor_returns_zero() {
    let apis = APIs::load();
    for &v1 in &[i32::MIN, -1, 0, 1, i32::MAX] {
        apis.compare("ERRORS row 1", v1, 0);
    }
    let mut rng = XorShift64::new(0xe770_0001_5eed);
    for _ in 0..RANDOM_CASES {
        apis.compare("ERRORS row 1", rng.next_u32() as i32, 0);
    }
}
