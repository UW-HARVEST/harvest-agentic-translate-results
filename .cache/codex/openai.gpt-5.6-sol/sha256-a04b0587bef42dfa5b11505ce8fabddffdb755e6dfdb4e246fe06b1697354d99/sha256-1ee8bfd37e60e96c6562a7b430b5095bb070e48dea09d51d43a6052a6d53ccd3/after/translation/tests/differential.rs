use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::path::{Path, PathBuf};

type DivEuclid = unsafe extern "C" fn(c_int, c_int) -> c_int;

const RANDOM_CASES: usize = 2_048;

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x as u32
    }

    fn positive(&mut self) -> i32 {
        (self.next_u32() % i32::MAX as u32 + 1) as i32
    }

    fn nonnegative(&mut self) -> i32 {
        (self.next_u32() & i32::MAX as u32) as i32
    }

    fn index(&mut self, len: usize) -> usize {
        self.next_u32() as usize % len
    }
}

fn c_library_path() -> PathBuf {
    let build = Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/build");
    let libraries: Vec<_> = std::fs::read_dir(&build)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", build.display()))
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension().is_some_and(|extension| extension == "so")
                && path
                    .file_name()
                    .is_some_and(|name| name.to_string_lossy().starts_with("libharvest-work-"))
        })
        .collect();
    assert_eq!(
        libraries.len(),
        1,
        "expected exactly one C shared library in {}, found {libraries:?}",
        build.display()
    );
    libraries.into_iter().next().unwrap()
}

fn rust_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("release")
        .join("libdiv_euclid_lib.so")
}

fn compare_cases(name: &str, cases: impl IntoIterator<Item = (i32, i32)>) {
    let c_path = c_library_path();
    let rust_path = rust_library_path();
    assert!(
        rust_path.is_file(),
        "Rust shared library does not exist at {}",
        rust_path.display()
    );

    unsafe {
        let c_library = Library::new(&c_path)
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", c_path.display()));
        let rust_library = Library::new(&rust_path)
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", rust_path.display()));
        let c_div: Symbol<DivEuclid> = c_library
            .get(b"div_euclid")
            .expect("C div_euclid export is missing");
        let rust_div: Symbol<DivEuclid> = rust_library
            .get(b"div_euclid")
            .expect("Rust div_euclid export is missing");

        for (case, (v1, v2)) in cases.into_iter().enumerate() {
            let c_result = c_div(v1, v2);
            let rust_result = rust_div(v1, v2);
            assert_eq!(
                c_result.to_ne_bytes(),
                rust_result.to_ne_bytes(),
                "{name} case {case}: div_euclid({v1}, {v2}) differed: \
                 C={c_result}, Rust={rust_result}"
            );
        }
    }
}

fn random_exact_cases(seed: u64, negative_divisor: bool) -> Vec<(i32, i32)> {
    let mut rng = Rng::new(seed);
    let mut cases = Vec::with_capacity(RANDOM_CASES);
    for _ in 0..RANDOM_CASES {
        let divisor = rng.positive();
        let max_multiplier = i32::MAX / divisor;
        let multiplier = (rng.next_u32() % max_multiplier as u32 + 1) as i32;
        let magnitude = divisor * multiplier;
        cases.push((
            -magnitude,
            if negative_divisor { -divisor } else { divisor },
        ));
    }
    cases
}

fn random_nonexact_cases(seed: u64, negative_divisor: bool) -> Vec<(i32, i32)> {
    let mut rng = Rng::new(seed);
    let mut cases = Vec::with_capacity(RANDOM_CASES);
    for _ in 0..RANDOM_CASES {
        let divisor = (rng.next_u32() % 999_999 + 2) as i32;
        let remainder = (rng.next_u32() % (divisor - 1) as u32 + 1) as i32;
        let max_quotient = (i32::MAX - (divisor - 1)) / divisor;
        let quotient = (rng.next_u32() % (max_quotient as u32 + 1)) as i32;
        let magnitude = quotient * divisor + remainder;
        cases.push((
            -magnitude,
            if negative_divisor { -divisor } else { divisor },
        ));
    }
    cases
}

#[test]
fn config_01_nonnegative_positive() {
    let mut rng = Rng::new(0x0101_0101);
    let mut cases = vec![(0, 1), (0, i32::MAX), (i32::MAX, 1), (i32::MAX, i32::MAX)];
    cases.extend((0..RANDOM_CASES).map(|_| (rng.nonnegative(), rng.positive())));
    compare_cases("config 1", cases);
}

#[test]
fn config_02_nonnegative_negative_non_min() {
    let mut rng = Rng::new(0x0202_0202);
    let mut cases = vec![(0, -1), (i32::MAX, -1), (i32::MAX, -i32::MAX)];
    cases.extend((0..RANDOM_CASES).map(|_| (rng.nonnegative(), -rng.positive())));
    compare_cases("config 2", cases);
}

#[test]
fn config_03_nonnegative_min_divisor() {
    let mut rng = Rng::new(0x0303_0303);
    let mut cases = vec![(0, i32::MIN), (i32::MAX, i32::MIN)];
    cases.extend((0..RANDOM_CASES).map(|_| (rng.nonnegative(), i32::MIN)));
    compare_cases("config 3", cases);
}

#[test]
fn config_04_negative_exact_positive() {
    let mut cases = vec![(-1, 1), (-i32::MAX, 1), (-i32::MAX, i32::MAX)];
    cases.extend(random_exact_cases(0x0404_0404, false));
    compare_cases("config 4", cases);
}

#[test]
fn config_05_negative_nonexact_positive() {
    let mut cases = vec![(-1, 2), (-i32::MAX, 2), (-i32::MAX, i32::MAX)];
    cases.extend(random_nonexact_cases(0x0505_0505, false));
    compare_cases("config 5", cases);
}

#[test]
fn config_06_negative_exact_negative() {
    let mut cases = vec![(-1, -1), (-i32::MAX, -1), (-i32::MAX, -i32::MAX)];
    cases.extend(random_exact_cases(0x0606_0606, true));
    compare_cases("config 6", cases);
}

#[test]
fn config_07_negative_nonexact_negative() {
    let mut cases = vec![(-1, -2), (-i32::MAX, -2), (-i32::MAX, -i32::MAX)];
    cases.extend(random_nonexact_cases(0x0707_0707, true));
    compare_cases("config 7", cases);
}

#[test]
fn config_08_negative_non_min_min_divisor() {
    let mut rng = Rng::new(0x0808_0808);
    let mut cases = vec![(-1, i32::MIN), (-i32::MAX, i32::MIN)];
    cases.extend((0..RANDOM_CASES).map(|_| (-rng.positive(), i32::MIN)));
    compare_cases("config 8", cases);
}

#[test]
fn config_09_min_dividend_exact_positive() {
    let divisors: Vec<i32> = (0..=30).map(|shift| 1_i32 << shift).collect();
    let mut rng = Rng::new(0x0909_0909);
    let mut cases = vec![(i32::MIN, 1), (i32::MIN, 1 << 30)];
    cases.extend((0..RANDOM_CASES).map(|_| (i32::MIN, divisors[rng.index(divisors.len())])));
    compare_cases("config 9", cases);
}

#[test]
fn config_10_min_dividend_nonexact_positive() {
    let mut rng = Rng::new(0x1010_1010);
    let mut cases = vec![(i32::MIN, 3), (i32::MIN, i32::MAX)];
    cases.extend((0..RANDOM_CASES).map(|_| {
        let mut divisor = rng.positive();
        if (divisor as u32).is_power_of_two() {
            divisor = 3;
        }
        (i32::MIN, divisor)
    }));
    compare_cases("config 10", cases);
}

#[test]
fn config_11_min_dividend_exact_negative() {
    let divisors: Vec<i32> = (0..=30).map(|shift| 1_i32 << shift).collect();
    let mut rng = Rng::new(0x1111_1111);
    let mut cases = vec![(i32::MIN, -1), (i32::MIN, -(1 << 30))];
    cases.extend((0..RANDOM_CASES).map(|_| (i32::MIN, -divisors[rng.index(divisors.len())])));
    compare_cases("config 11", cases);
}

#[test]
fn config_12_min_dividend_nonexact_negative() {
    let mut rng = Rng::new(0x1212_1212);
    let mut cases = vec![(i32::MIN, -3), (i32::MIN, -i32::MAX)];
    cases.extend((0..RANDOM_CASES).map(|_| {
        let mut divisor = rng.positive();
        if (divisor as u32).is_power_of_two() {
            divisor = 3;
        }
        (i32::MIN, -divisor)
    }));
    compare_cases("config 12", cases);
}

#[test]
fn config_13_double_minimum() {
    compare_cases("config 13", [(i32::MIN, i32::MIN)]);
}

#[test]
fn error_01_zero_divisor() {
    let mut rng = Rng::new(0xe001_e001);
    let mut cases = vec![(i32::MIN, 0), (-1, 0), (0, 0), (1, 0), (i32::MAX, 0)];
    cases.extend((0..RANDOM_CASES).map(|_| (rng.next_u32() as i32, 0)));
    compare_cases("error 1", cases);
}

#[test]
fn generic_scalar_boundaries() {
    let values = [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX];
    compare_cases(
        "generic scalar boundaries",
        values
            .into_iter()
            .flat_map(|v1| values.into_iter().map(move |v2| (v1, v2))),
    );
}
