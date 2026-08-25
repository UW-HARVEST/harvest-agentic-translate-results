use libloading::Library;
use std::ffi::c_int;
use std::path::{Path, PathBuf};

type DivEuclid = unsafe extern "C" fn(c_int, c_int) -> c_int;

const CASES_PER_ROW: usize = 4_096;

struct Apis {
    _c_library: Library,
    _rust_library: Library,
    c_div_euclid: DivEuclid,
    rust_div_euclid: DivEuclid,
}

impl Apis {
    fn load() -> Self {
        let c_path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("c_src/build/libtranslated_rust.so");
        let rust_path = rust_library_path();

        assert!(
            c_path.is_file(),
            "C shared library is missing at {}; build it with CMake first",
            c_path.display()
        );
        assert!(
            rust_path.is_file(),
            "Rust shared library is missing at {}",
            rust_path.display()
        );

        unsafe {
            let c_library = Library::new(&c_path)
                .unwrap_or_else(|error| panic!("failed to load {}: {error}", c_path.display()));
            let rust_library = Library::new(&rust_path)
                .unwrap_or_else(|error| panic!("failed to load {}: {error}", rust_path.display()));
            let c_div_euclid = *c_library
                .get::<DivEuclid>(b"div_euclid\0")
                .expect("C library does not export div_euclid");
            let rust_div_euclid = *rust_library
                .get::<DivEuclid>(b"div_euclid\0")
                .expect("Rust library does not export div_euclid");

            Self {
                _c_library: c_library,
                _rust_library: rust_library,
                c_div_euclid,
                rust_div_euclid,
            }
        }
    }

    fn compare(&self, v1: c_int, v2: c_int) {
        unsafe {
            let c_result = (self.c_div_euclid)(v1, v2);
            let rust_result = (self.rust_div_euclid)(v1, v2);
            assert_eq!(
                c_result.to_ne_bytes(),
                rust_result.to_ne_bytes(),
                "div_euclid({v1}, {v2}): C returned {c_result}, Rust returned {rust_result}"
            );
        }
    }
}

fn rust_library_path() -> PathBuf {
    let test_executable = std::env::current_exe().expect("cannot locate test executable");
    let deps_directory = test_executable
        .parent()
        .expect("test executable has no parent directory");
    let deps_library = deps_directory.join("libdiv_euclid_lib.so");
    if deps_library.is_file() {
        return deps_library;
    }

    deps_directory
        .parent()
        .expect("deps directory has no parent")
        .join("libdiv_euclid_lib.so")
}

struct Rng(u64);

impl Rng {
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

    fn nonnegative(&mut self) -> c_int {
        (self.next_u32() & c_int::MAX as u32) as c_int
    }

    fn positive(&mut self) -> c_int {
        self.nonnegative().max(1)
    }

    fn negative_non_min(&mut self) -> c_int {
        -self.positive()
    }
}

fn compare_cases(seed: u64, mut make_case: impl FnMut(&mut Rng) -> (c_int, c_int)) {
    let apis = Apis::load();
    let mut rng = Rng::new(seed);
    for _ in 0..CASES_PER_ROW {
        let (v1, v2) = make_case(&mut rng);
        apis.compare(v1, v2);
    }
}

fn exact_negative_dividend(rng: &mut Rng) -> (c_int, c_int) {
    let divisor = (rng.next_u32() % 65_535 + 1) as c_int;
    let max_quotient = c_int::MAX / divisor;
    let quotient = (rng.next_u32() % max_quotient as u32 + 1) as c_int;
    (-divisor * quotient, divisor)
}

#[test]
fn config_01_nonnegative_dividend_positive_divisor() {
    compare_cases(0x0101_0101, |rng| (rng.nonnegative(), rng.positive()));
}

#[test]
fn config_02_nonnegative_dividend_negative_divisor() {
    compare_cases(0x0202_0202, |rng| {
        (rng.nonnegative(), rng.negative_non_min())
    });
}

#[test]
fn config_03_nonnegative_dividend_min_divisor() {
    compare_cases(0x0303_0303, |rng| (rng.nonnegative(), c_int::MIN));
}

#[test]
fn config_04_negative_dividend_positive_divisor_exact() {
    compare_cases(0x0404_0404, exact_negative_dividend);
}

#[test]
fn config_05_negative_dividend_positive_divisor_with_remainder() {
    compare_cases(0x0505_0505, |rng| {
        loop {
            let pair = (rng.negative_non_min(), rng.positive());
            if pair.0 % pair.1 != 0 {
                break pair;
            }
        }
    });
}

#[test]
fn config_06_negative_dividend_negative_divisor_exact() {
    compare_cases(0x0606_0606, |rng| {
        let (v1, positive_divisor) = exact_negative_dividend(rng);
        (v1, -positive_divisor)
    });
}

#[test]
fn config_07_negative_dividend_negative_divisor_with_remainder() {
    compare_cases(0x0707_0707, |rng| {
        loop {
            let pair = (rng.negative_non_min(), rng.negative_non_min());
            if pair.0 % pair.1 != 0 {
                break pair;
            }
        }
    });
}

#[test]
fn config_08_negative_dividend_min_divisor() {
    compare_cases(0x0808_0808, |rng| (rng.negative_non_min(), c_int::MIN));
}

#[test]
fn config_09_min_dividend_positive_divisor_exact() {
    compare_cases(0x0909_0909, |rng| {
        let exponent = rng.next_u32() % 31;
        (c_int::MIN, 1 << exponent)
    });
}

#[test]
fn config_10_min_dividend_positive_divisor_with_remainder() {
    compare_cases(0x1010_1010, |rng| {
        loop {
            let divisor = rng.positive();
            if !divisor.unsigned_abs().is_power_of_two() {
                break (c_int::MIN, divisor);
            }
        }
    });
}

#[test]
fn config_11_min_dividend_negative_divisor_exact() {
    compare_cases(0x1111_1111, |rng| {
        let exponent = rng.next_u32() % 31;
        (c_int::MIN, -(1 << exponent))
    });
}

#[test]
fn config_12_min_dividend_negative_divisor_with_remainder() {
    compare_cases(0x1212_1212, |rng| {
        loop {
            let divisor = rng.negative_non_min();
            if !divisor.unsigned_abs().is_power_of_two() {
                break (c_int::MIN, divisor);
            }
        }
    });
}

#[test]
fn config_13_min_dividend_min_divisor() {
    Apis::load().compare(c_int::MIN, c_int::MIN);
}

#[test]
fn error_01_zero_divisor() {
    compare_cases(0xe101_e101, |rng| (rng.next_u32() as c_int, 0));
}

#[test]
fn boundary_values() {
    let apis = Apis::load();
    let values = [
        c_int::MIN,
        c_int::MIN + 1,
        -1,
        0,
        1,
        c_int::MAX - 1,
        c_int::MAX,
    ];
    for v1 in values {
        for v2 in values {
            apis.compare(v1, v2);
        }
    }
}

#[test]
fn randomized_full_domain() {
    compare_cases(0xd1ff_e2e1, |rng| {
        (rng.next_u32() as c_int, rng.next_u32() as c_int)
    });
}
