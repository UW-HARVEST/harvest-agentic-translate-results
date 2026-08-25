use libloading::Library;
use std::env;
use std::ffi::{c_double, c_int};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};

type SpectralContrast = unsafe extern "C" fn(*mut c_double, *mut c_double, c_int) -> c_double;
type Match = unsafe extern "C" fn(*mut c_double, *mut c_double, c_int, c_double) -> c_int;

struct Api {
    _library: Library,
    spectral_contrast: SpectralContrast,
    matcher: Match,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let spectral_contrast = unsafe {
            *library
                .get::<SpectralContrast>(b"spectral_contrast\0")
                .unwrap()
        };
        let matcher = unsafe { *library.get::<Match>(b"match\0").unwrap() };
        Self {
            _library: library,
            spectral_contrast,
            matcher,
        }
    }
}

struct Pair {
    c: Api,
    rust: Api,
}

impl Pair {
    fn load() -> Self {
        unsafe {
            Self {
                c: Api::load(&c_library_path()),
                rust: Api::load(&rust_library_path()),
            }
        }
    }
}

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    crate_root().join("c_src/build/libtranslated_rust.so")
}

fn rust_library_path() -> PathBuf {
    let root = crate_root();
    [
        root.join("target/debug/deps/libtranslated_rust.so"),
        root.join("target/debug/libtranslated_rust.so"),
        root.join("target/release/libtranslated_rust.so"),
    ]
    .into_iter()
    .find(|path| path.is_file())
    .expect("Rust cdylib was not built")
}

fn bits(values: &[f64]) -> Vec<u64> {
    values.iter().map(|value| value.to_bits()).collect()
}

fn packed_f32(values: &[f32]) -> Vec<f64> {
    let mut words = vec![0x0123_4567_89ab_cdef_u64; values.len().max(1)];
    for (index, value) in values.iter().enumerate() {
        let shift = (index % 2) * 32;
        let mask = 0xffff_ffff_u64 << shift;
        words[index / 2] = (words[index / 2] & !mask) | ((value.to_bits() as u64) << shift);
    }
    words.into_iter().map(f64::from_bits).collect()
}

fn assert_spectral_disjoint(pair: &Pair, a: &[f64], b: &[f64], length: i32) -> f64 {
    let mut c_a = a.to_vec();
    let mut c_b = b.to_vec();
    let mut rust_a = a.to_vec();
    let mut rust_b = b.to_vec();
    let c_result =
        unsafe { (pair.c.spectral_contrast)(c_a.as_mut_ptr(), c_b.as_mut_ptr(), length) };
    let rust_result =
        unsafe { (pair.rust.spectral_contrast)(rust_a.as_mut_ptr(), rust_b.as_mut_ptr(), length) };
    assert_eq!(
        c_result.to_bits(),
        rust_result.to_bits(),
        "return value for length {length}, a={:x?}, b={:x?}",
        bits(a),
        bits(b)
    );
    assert_eq!(bits(&c_a), bits(&rust_a), "first output buffer");
    assert_eq!(bits(&c_b), bits(&rust_b), "second output buffer");
    c_result
}

fn assert_spectral_alias(pair: &Pair, input: &[f64], length: i32) -> f64 {
    let mut c_input = input.to_vec();
    let mut rust_input = input.to_vec();
    let c_result =
        unsafe { (pair.c.spectral_contrast)(c_input.as_mut_ptr(), c_input.as_mut_ptr(), length) };
    let rust_result = unsafe {
        (pair.rust.spectral_contrast)(rust_input.as_mut_ptr(), rust_input.as_mut_ptr(), length)
    };
    assert_eq!(c_result.to_bits(), rust_result.to_bits(), "return value");
    assert_eq!(bits(&c_input), bits(&rust_input), "aliased output buffer");
    c_result
}

fn assert_spectral_overlap(pair: &Pair, input: &[f64], length: i32) -> f64 {
    let mut c_input = input.to_vec();
    let mut rust_input = input.to_vec();
    let c_result = unsafe {
        let first = c_input.as_mut_ptr();
        (pair.c.spectral_contrast)(first, first.add(1), length)
    };
    let rust_result = unsafe {
        let first = rust_input.as_mut_ptr();
        (pair.rust.spectral_contrast)(first, first.add(1), length)
    };
    assert_eq!(c_result.to_bits(), rust_result.to_bits(), "return value");
    assert_eq!(
        bits(&c_input),
        bits(&rust_input),
        "overlapping output buffer"
    );
    c_result
}

fn assert_match(pair: &Pair, test: &[f64], reference: &[f64], threshold: f64) -> i32 {
    let mut c_test = test.to_vec();
    let mut c_reference = reference.to_vec();
    let mut rust_test = test.to_vec();
    let mut rust_reference = reference.to_vec();
    let bins = test.len() as i32;
    let c_result = unsafe {
        (pair.c.matcher)(
            c_test.as_mut_ptr(),
            c_reference.as_mut_ptr(),
            bins,
            threshold,
        )
    };
    let rust_result = unsafe {
        (pair.rust.matcher)(
            rust_test.as_mut_ptr(),
            rust_reference.as_mut_ptr(),
            bins,
            threshold,
        )
    };
    assert_eq!(c_result, rust_result, "return value");
    assert_eq!(bits(&c_test), bits(&rust_test), "test buffer");
    assert_eq!(
        bits(&c_reference),
        bits(&rust_reference),
        "reference buffer"
    );
    c_result
}

fn assert_match_alias(pair: &Pair, input: &[f64], threshold: f64) -> i32 {
    let mut c_input = input.to_vec();
    let mut rust_input = input.to_vec();
    let bins = input.len() as i32;
    let c_result =
        unsafe { (pair.c.matcher)(c_input.as_mut_ptr(), c_input.as_mut_ptr(), bins, threshold) };
    let rust_result = unsafe {
        (pair.rust.matcher)(
            rust_input.as_mut_ptr(),
            rust_input.as_mut_ptr(),
            bins,
            threshold,
        )
    };
    assert_eq!(c_result, rust_result, "return value");
    assert_eq!(bits(&c_input), bits(&rust_input), "aliased input buffer");
    c_result
}

struct Rng(u64);

impl Rng {
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

    fn finite_f32(&mut self) -> f32 {
        let magnitude = ((self.next_u64() % 20_000) as f32 + 1.0) / 997.0;
        if self.next_u64() & 1 == 0 {
            magnitude
        } else {
            -magnitude
        }
    }

    fn positive_f64(&mut self, base: f64) -> f64 {
        let low_word = (self.finite_f32().abs() + 0.25).to_bits() as u64;
        f64::from_bits((base.to_bits() & 0xffff_ffff_0000_0000) | low_word)
    }

    fn positive_vec(&mut self, length: usize, base: f64) -> Vec<f64> {
        (0..length)
            .map(|index| self.positive_f64(base + (index % 5) as f64 * 0.125))
            .collect()
    }
}

#[test]
fn config_01_spectral_empty() {
    let pair = Pair::load();
    for _ in 0..64 {
        let c =
            unsafe { (pair.c.spectral_contrast)(std::ptr::null_mut(), std::ptr::null_mut(), 0) };
        let rust =
            unsafe { (pair.rust.spectral_contrast)(std::ptr::null_mut(), std::ptr::null_mut(), 0) };
        assert_eq!(c.to_bits(), rust.to_bits());
    }
}

#[test]
fn config_02_spectral_single() {
    let pair = Pair::load();
    let mut rng = Rng::new(0x0202_0202_0202_0202);
    for _ in 0..128 {
        let a = packed_f32(&[rng.finite_f32()]);
        let b = packed_f32(&[rng.finite_f32()]);
        assert_spectral_disjoint(&pair, &a, &b, 1);
    }
}

#[test]
fn config_03_spectral_many_finite() {
    let pair = Pair::load();
    let mut rng = Rng::new(0x0303_0303_0303_0303);
    for length in 2..34 {
        for _ in 0..8 {
            let a_values = (0..length).map(|_| rng.finite_f32()).collect::<Vec<_>>();
            let b_values = (0..length).map(|_| rng.finite_f32()).collect::<Vec<_>>();
            assert_spectral_disjoint(
                &pair,
                &packed_f32(&a_values),
                &packed_f32(&b_values),
                length,
            );
        }
    }
}

#[test]
fn config_04_spectral_exact_alias() {
    let pair = Pair::load();
    let mut rng = Rng::new(0x0404_0404_0404_0404);
    for length in 2..66 {
        let values = (0..length).map(|_| rng.finite_f32()).collect::<Vec<_>>();
        assert_spectral_alias(&pair, &packed_f32(&values), length);
    }
}

#[test]
fn config_05_spectral_partial_overlap() {
    let pair = Pair::load();
    let mut rng = Rng::new(0x0505_0505_0505_0505);
    for length in 3..67 {
        let values = (0..length + 2)
            .map(|_| rng.finite_f32())
            .collect::<Vec<_>>();
        let mut input = packed_f32(&values);
        input.resize(length as usize + 2, f64::from_bits(0xfeed_face_cafe_beef));
        assert_spectral_overlap(&pair, &input, length);
    }
}

#[test]
fn config_06_spectral_zero_magnitude() {
    let pair = Pair::load();
    let mut rng = Rng::new(0x0606_0606_0606_0606);
    for length in 1..65 {
        let zeros = vec![0.0_f32; length];
        let finite = (0..length).map(|_| rng.finite_f32()).collect::<Vec<_>>();
        assert_spectral_disjoint(
            &pair,
            &packed_f32(&zeros),
            &packed_f32(&finite),
            length as i32,
        );
        assert_spectral_disjoint(
            &pair,
            &packed_f32(&zeros),
            &packed_f32(&zeros),
            length as i32,
        );
    }
}

#[test]
fn config_07_spectral_nonfinite() {
    let pair = Pair::load();
    let mut rng = Rng::new(0x0707_0707_0707_0707);
    let specials = [f32::NAN, f32::INFINITY, f32::NEG_INFINITY];
    for case in 0..96 {
        let length = 2 + case % 31;
        let mut a = (0..length).map(|_| rng.finite_f32()).collect::<Vec<_>>();
        let mut b = (0..length).map(|_| rng.finite_f32()).collect::<Vec<_>>();
        a[case % length] = specials[case % specials.len()];
        b[(case * 7) % length] = specials[(case / 3) % specials.len()];
        assert_spectral_disjoint(&pair, &packed_f32(&a), &packed_f32(&b), length as i32);
    }
}

#[test]
fn config_08_match_single_early_rejection() {
    let pair = Pair::load();
    let mut rng = Rng::new(0x0808_0808_0808_0808);
    for _ in 0..128 {
        let reference = vec![rng.positive_f64(4.0)];
        let test = vec![rng.positive_f64(1.0)];
        assert_eq!(assert_match(&pair, &test, &reference, 0.75), 0);
    }
}

#[test]
fn config_09_match_single_full_pipeline() {
    let pair = Pair::load();
    let mut rng = Rng::new(0x0909_0909_0909_0909);
    for _ in 0..128 {
        let reference = vec![rng.positive_f64(1.0)];
        let test = vec![rng.positive_f64(2.0)];
        assert_eq!(assert_match(&pair, &test, &reference, 0.5), 0);
    }
}

#[test]
fn config_10_match_short_early_rejection() {
    let pair = Pair::load();
    let mut rng = Rng::new(0x1010_1010_1010_1010);
    for case in 0..128 {
        let length = 2 + case % 14;
        let reference = rng.positive_vec(length, 4.0);
        let test = rng.positive_vec(length, 1.0);
        assert_eq!(assert_match(&pair, &test, &reference, 0.75), 0);
    }
}

#[test]
fn config_11_match_short_full_true() {
    let pair = Pair::load();
    let mut rng = Rng::new(0x1111_1111_1111_1111);
    let mut true_cases = 0;
    for case in 0..512 {
        let length = 2 + case % 14;
        let input = rng.positive_vec(length, 1.0);
        if assert_match(&pair, &input, &input, 0.5) == 1 {
            true_cases += 1;
            if true_cases == 64 {
                break;
            }
        }
    }
    assert_eq!(true_cases, 64);
}

#[test]
fn config_12_match_short_full_false() {
    let pair = Pair::load();
    let mut rng = Rng::new(0x1212_1212_1212_1212);
    let mut false_cases = 0;
    for case in 0..1024 {
        let length = 3 + case % 13;
        let reference = rng.positive_vec(length, 1.0);
        let test = reference.iter().copied().rev().collect::<Vec<_>>();
        if assert_match(&pair, &test, &reference, 0.95) == 0 {
            false_cases += 1;
            if false_cases == 64 {
                break;
            }
        }
    }
    assert_eq!(false_cases, 64);
}

#[test]
fn config_13_match_exact_kernel_width() {
    let pair = Pair::load();
    let mut rng = Rng::new(0x1313_1313_1313_1313);
    for _ in 0..128 {
        let reference = rng.positive_vec(16, 1.0);
        assert_match(&pair, &reference, &reference, 0.5);
        let test = rng.positive_vec(16, 2.0);
        assert_match(&pair, &test, &reference, 1.1);
    }
}

#[test]
fn config_14_match_longer_than_kernel() {
    let pair = Pair::load();
    let mut rng = Rng::new(0x1414_1414_1414_1414);
    for case in 0..128 {
        let length = 17 + case % 80;
        let reference = rng.positive_vec(length, 1.0);
        let test = rng.positive_vec(length, 2.0);
        assert_match(&pair, &reference, &reference, 0.5);
        assert_match(&pair, &test, &reference, 1.1);
    }
}

#[test]
fn config_15_match_exact_alias() {
    let pair = Pair::load();
    let mut rng = Rng::new(0x1515_1515_1515_1515);
    for case in 0..128 {
        let length = 1 + case % 64;
        let input = rng.positive_vec(length, 1.0);
        assert_match_alias(&pair, &input, 0.5);
    }
}

#[test]
fn config_16_match_nonpositive_threshold() {
    let pair = Pair::load();
    let mut rng = Rng::new(0x1616_1616_1616_1616);
    for case in 0..128 {
        let length = 2 + case % 47;
        let test = rng.positive_vec(length, 1.0);
        let reference = rng.positive_vec(length, 2.0);
        assert_match(&pair, &test, &reference, 0.0);
        assert_match(&pair, &test, &reference, -1.0);
    }
}

#[test]
fn config_17_match_threshold_above_one() {
    let pair = Pair::load();
    let mut rng = Rng::new(0x1717_1717_1717_1717);
    for case in 0..128 {
        let length = 2 + case % 47;
        let test = rng.positive_vec(length, 4.0);
        let reference = rng.positive_vec(length, 1.0);
        assert_eq!(assert_match(&pair, &test, &reference, 1.25), 0);
    }
}

#[test]
fn config_18_match_nonfinite_threshold() {
    let pair = Pair::load();
    let mut rng = Rng::new(0x1818_1818_1818_1818);
    for case in 0..96 {
        let length = 2 + case % 47;
        let test = rng.positive_vec(length, 2.0);
        let reference = rng.positive_vec(length, 1.0);
        assert_match(&pair, &test, &reference, f64::NAN);
        assert_match(&pair, &test, &reference, f64::INFINITY);
        assert_match(&pair, &test, &reference, f64::NEG_INFINITY);
    }
}

#[test]
fn error_01_explicit_early_rejection() {
    let pair = Pair::load();
    for index in 1..129 {
        let test = vec![index as f64];
        let reference = vec![(index * 4) as f64];
        assert_eq!(assert_match(&pair, &test, &reference, 0.75), 0);
    }
}

#[test]
fn error_02_spectral_zero_length_null() {
    let pair = Pair::load();
    let c = unsafe { (pair.c.spectral_contrast)(std::ptr::null_mut(), std::ptr::null_mut(), 0) };
    let rust =
        unsafe { (pair.rust.spectral_contrast)(std::ptr::null_mut(), std::ptr::null_mut(), 0) };
    assert_eq!(c.to_bits(), 0);
    assert_eq!(c.to_bits(), rust.to_bits());
}

#[test]
fn error_03_spectral_negative_length_null() {
    let pair = Pair::load();
    let c = unsafe { (pair.c.spectral_contrast)(std::ptr::null_mut(), std::ptr::null_mut(), -1) };
    let rust =
        unsafe { (pair.rust.spectral_contrast)(std::ptr::null_mut(), std::ptr::null_mut(), -1) };
    assert_eq!(c.to_bits(), 0);
    assert_eq!(c.to_bits(), rust.to_bits());
}

#[repr(C)]
struct RLimit {
    current: u64,
    maximum: u64,
}

fn disable_core_dumps() {
    unsafe extern "C" {
        fn setrlimit(resource: c_int, limit: *const RLimit) -> c_int;
    }
    const RLIMIT_CORE: c_int = 4;
    let limit = RLimit {
        current: 0,
        maximum: 0,
    };
    unsafe {
        setrlimit(RLIMIT_CORE, &limit);
    }
}

fn run_boundary_child(library: &Path, case: &str) -> ExitStatus {
    Command::new(env::current_exe().unwrap())
        .arg("--exact")
        .arg("boundary_child")
        .arg("--nocapture")
        .env("DIFF_CHILD_LIBRARY", library)
        .env("DIFF_CHILD_CASE", case)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap()
}

fn assert_same_sigsegv(case: &str) {
    use std::os::unix::process::ExitStatusExt;

    let c = run_boundary_child(&c_library_path(), case);
    let rust = run_boundary_child(&rust_library_path(), case);
    assert_eq!(c.signal(), Some(11), "C status: {c:?}");
    assert_eq!(rust.signal(), c.signal(), "Rust status: {rust:?}");
}

#[test]
fn boundary_child() {
    let Ok(case) = env::var("DIFF_CHILD_CASE") else {
        return;
    };
    disable_core_dumps();
    let library = PathBuf::from(env::var_os("DIFF_CHILD_LIBRARY").unwrap());
    let api = unsafe { Api::load(&library) };
    let mut values = [1.0_f64, 2.0];
    let null = std::ptr::null_mut();
    unsafe {
        match case.as_str() {
            "spectral_null_a" => {
                (api.spectral_contrast)(null, values.as_mut_ptr(), 1);
            }
            "spectral_null_b" => {
                (api.spectral_contrast)(values.as_mut_ptr(), null, 1);
            }
            "match_null_test" => {
                (api.matcher)(null, values.as_mut_ptr(), 1, 0.5);
            }
            "match_null_reference" => {
                (api.matcher)(values.as_mut_ptr(), null, 1, 0.5);
            }
            "match_zero" => {
                (api.matcher)(values.as_mut_ptr(), values.as_mut_ptr(), 0, 0.5);
            }
            "match_negative" => {
                (api.matcher)(values.as_mut_ptr(), values.as_mut_ptr(), -1, 0.5);
            }
            "match_oversized" => {
                (api.matcher)(values.as_mut_ptr(), values.as_mut_ptr(), c_int::MAX, 0.5);
            }
            _ => panic!("unknown boundary case {case}"),
        }
    }
}

#[test]
fn error_04_spectral_null_first() {
    assert_same_sigsegv("spectral_null_a");
}

#[test]
fn error_05_spectral_null_second() {
    assert_same_sigsegv("spectral_null_b");
}

#[test]
fn error_06_match_null_test() {
    assert_same_sigsegv("match_null_test");
}

#[test]
fn error_07_match_null_reference() {
    assert_same_sigsegv("match_null_reference");
}

#[test]
fn error_08_match_zero_length() {
    assert_same_sigsegv("match_zero");
}

#[test]
fn error_09_match_negative_length() {
    assert_same_sigsegv("match_negative");
}

#[test]
fn error_10_match_oversized_length() {
    assert_same_sigsegv("match_oversized");
}
