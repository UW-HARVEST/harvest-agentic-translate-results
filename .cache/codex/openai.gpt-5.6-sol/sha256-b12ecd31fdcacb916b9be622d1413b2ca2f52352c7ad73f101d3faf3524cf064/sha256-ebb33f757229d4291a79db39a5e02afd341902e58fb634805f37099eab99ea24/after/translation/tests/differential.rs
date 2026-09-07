use libloading::{Library, Symbol};
use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};

type MatchFn = unsafe extern "C" fn(*mut f64, *mut f64, i32, f64) -> i32;
type SpectralFn = unsafe extern "C" fn(*mut f64, *mut f64, i32) -> f64;

const RANDOM_CASES: usize = 64;
const SIGSEGV: i32 = 11;

fn c_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/build/libharvest-work-LTHfSI.so")
}

fn rust_library_path() -> PathBuf {
    env::current_exe()
        .expect("current test executable")
        .parent()
        .expect("deps directory")
        .parent()
        .expect("profile directory")
        .join("libunderhanded_c_nuke_lib.so")
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn index(&mut self, length: usize) -> usize {
        (self.next_u64() as usize) % length
    }

    fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / ((1_u64 << 53) as f64)
    }

    fn positive_f64(&mut self) -> f64 {
        0.25 + self.unit() * 9.75
    }

    fn nonzero_f32(&mut self) -> f32 {
        let value = (self.unit() * 20.0 - 10.0) as f32;
        if value.abs() < 0.01 { 1.0 } else { value }
    }
}

fn doubles_containing_floats(values: &[f32]) -> Vec<f64> {
    let mut result = vec![f64::from_bits(0x3fd5_5555_5555_5555); values.len().max(1)];
    for (index, value) in values.iter().enumerate() {
        let double_index = index / 2;
        let byte_offset = (index % 2) * 4;
        let mut bytes = result[double_index].to_ne_bytes();
        bytes[byte_offset..byte_offset + 4].copy_from_slice(&value.to_ne_bytes());
        result[double_index] = f64::from_ne_bytes(bytes);
    }
    result
}

fn assert_f64_buffers_equal(c: &[f64], rust: &[f64], context: &str) {
    let c_bits: Vec<u64> = c.iter().map(|value| value.to_bits()).collect();
    let rust_bits: Vec<u64> = rust.iter().map(|value| value.to_bits()).collect();
    assert_eq!(c_bits, rust_bits, "{context}");
}

fn run_spectral_configuration(
    row: usize,
    lengths: &[usize],
    zero_a: bool,
    zero_b: bool,
    seed: u64,
) {
    let c_library = unsafe { Library::new(c_library_path()) }.expect("load C library");
    let rust_library = unsafe { Library::new(rust_library_path()) }.expect("load Rust library");
    let c_fn: Symbol<SpectralFn> =
        unsafe { c_library.get(b"spectral_contrast\0") }.expect("C spectral_contrast");
    let rust_fn: Symbol<SpectralFn> =
        unsafe { rust_library.get(b"spectral_contrast\0") }.expect("Rust spectral_contrast");
    let mut rng = Rng::new(seed);

    for case in 0..RANDOM_CASES {
        let length = lengths[rng.index(lengths.len())];
        let mut a_floats: Vec<f32> = (0..length).map(|_| rng.nonzero_f32()).collect();
        let mut b_floats: Vec<f32> = (0..length).map(|_| rng.nonzero_f32()).collect();
        if zero_a {
            a_floats.fill(0.0);
        }
        if zero_b {
            b_floats.fill(0.0);
        }

        let a = doubles_containing_floats(&a_floats);
        let b = doubles_containing_floats(&b_floats);
        let mut c_a = a.clone();
        let mut c_b = b.clone();
        let mut rust_a = a;
        let mut rust_b = b;
        let c_result = unsafe { c_fn(c_a.as_mut_ptr(), c_b.as_mut_ptr(), length as i32) };
        let rust_result =
            unsafe { rust_fn(rust_a.as_mut_ptr(), rust_b.as_mut_ptr(), length as i32) };
        let context = format!("CONFIGS row {row}, randomized case {case}, length {length}");

        assert_eq!(
            c_result.to_bits(),
            rust_result.to_bits(),
            "{context}: return value"
        );
        assert_f64_buffers_equal(&c_a, &rust_a, &format!("{context}: a buffer"));
        assert_f64_buffers_equal(&c_b, &rust_b, &format!("{context}: b buffer"));
    }
}

#[derive(Clone, Copy)]
enum MatchOutcome {
    EarlyReject,
    FullFalse,
    FullTrue,
}

fn run_match_configuration(row: usize, bins_choices: &[usize], outcome: MatchOutcome, seed: u64) {
    let c_library = unsafe { Library::new(c_library_path()) }.expect("load C library");
    let rust_library = unsafe { Library::new(rust_library_path()) }.expect("load Rust library");
    let c_fn: Symbol<MatchFn> = unsafe { c_library.get(b"match\0") }.expect("C match");
    let rust_fn: Symbol<MatchFn> = unsafe { rust_library.get(b"match\0") }.expect("Rust match");
    let mut rng = Rng::new(seed);
    let mut accepted = 0;
    let mut attempts = 0;

    while accepted < RANDOM_CASES && attempts < 200_000 {
        attempts += 1;
        let bins = bins_choices[rng.index(bins_choices.len())];
        let reference: Vec<f64> = (0..bins).map(|_| rng.positive_f64()).collect();
        let (mut test, threshold, expected) = match outcome {
            MatchOutcome::EarlyReject => (reference.clone(), 1.5, 0),
            MatchOutcome::FullTrue => (reference.clone(), 0.0, 1),
            MatchOutcome::FullFalse => ((0..bins).map(|_| rng.positive_f64()).collect(), 0.5, 0),
        };

        let reference_total: f64 = reference.iter().copied().sum();
        let mut test_total: f64 = test.iter().copied().sum();
        if matches!(outcome, MatchOutcome::FullFalse) && test_total < threshold * reference_total {
            let scale = threshold * reference_total / test_total + 0.25;
            for value in &mut test {
                *value *= scale;
            }
            test_total = test.iter().copied().sum();
        }

        match outcome {
            MatchOutcome::EarlyReject => {
                assert!(test_total < threshold * reference_total);
            }
            MatchOutcome::FullFalse | MatchOutcome::FullTrue => {
                assert!(test_total >= threshold * reference_total);
            }
        }

        let mut c_test = test.clone();
        let mut c_reference = reference.clone();
        let c_result = unsafe {
            c_fn(
                c_test.as_mut_ptr(),
                c_reference.as_mut_ptr(),
                bins as i32,
                threshold,
            )
        };
        if c_result != expected {
            continue;
        }

        let mut rust_test = test;
        let mut rust_reference = reference;
        let rust_result = unsafe {
            rust_fn(
                rust_test.as_mut_ptr(),
                rust_reference.as_mut_ptr(),
                bins as i32,
                threshold,
            )
        };
        assert_eq!(
            c_result, rust_result,
            "CONFIGS row {row}, accepted case {accepted}, bins {bins}"
        );
        assert_f64_buffers_equal(
            &c_test,
            &rust_test,
            &format!("CONFIGS row {row}: caller test buffer"),
        );
        assert_f64_buffers_equal(
            &c_reference,
            &rust_reference,
            &format!("CONFIGS row {row}: caller reference buffer"),
        );
        accepted += 1;
    }

    assert_eq!(
        accepted, RANDOM_CASES,
        "CONFIGS row {row}: insufficient matching randomized cases after {attempts} attempts"
    );
}

#[test]
fn config_01_spectral_length_one() {
    run_spectral_configuration(1, &[1], false, false, 0x1001);
}

#[test]
fn config_02_spectral_even_many() {
    run_spectral_configuration(2, &[2, 4, 8, 16, 32], false, false, 0x1002);
}

#[test]
fn config_03_spectral_odd_many() {
    run_spectral_configuration(3, &[3, 5, 9, 17, 31], false, false, 0x1003);
}

#[test]
fn config_04_spectral_zero_a() {
    run_spectral_configuration(4, &[1, 2, 7, 16, 31], true, false, 0x1004);
}

#[test]
fn config_05_spectral_zero_b() {
    run_spectral_configuration(5, &[1, 2, 7, 16, 31], false, true, 0x1005);
}

#[test]
fn config_06_match_one_early() {
    run_match_configuration(6, &[1], MatchOutcome::EarlyReject, 0x2006);
}

#[test]
fn config_07_match_one_false() {
    run_match_configuration(7, &[1], MatchOutcome::FullFalse, 0x2007);
}

#[test]
fn config_08_match_small_even_early() {
    run_match_configuration(
        8,
        &[2, 4, 6, 8, 10, 12, 14],
        MatchOutcome::EarlyReject,
        0x2008,
    );
}

#[test]
fn config_09_match_small_even_false() {
    run_match_configuration(
        9,
        &[2, 4, 6, 8, 10, 12, 14],
        MatchOutcome::FullFalse,
        0x2009,
    );
}

#[test]
fn config_10_match_small_even_true() {
    run_match_configuration(
        10,
        &[2, 4, 6, 8, 10, 12, 14],
        MatchOutcome::FullTrue,
        0x2010,
    );
}

#[test]
fn config_11_match_small_odd_early() {
    run_match_configuration(
        11,
        &[3, 5, 7, 9, 11, 13, 15],
        MatchOutcome::EarlyReject,
        0x2011,
    );
}

#[test]
fn config_12_match_small_odd_false() {
    run_match_configuration(
        12,
        &[3, 5, 7, 9, 11, 13, 15],
        MatchOutcome::FullFalse,
        0x2012,
    );
}

#[test]
fn config_13_match_small_odd_true() {
    run_match_configuration(
        13,
        &[3, 5, 7, 9, 11, 13, 15],
        MatchOutcome::FullTrue,
        0x2013,
    );
}

#[test]
fn config_14_match_sixteen_early() {
    run_match_configuration(14, &[16], MatchOutcome::EarlyReject, 0x2014);
}

#[test]
fn config_15_match_sixteen_false() {
    run_match_configuration(15, &[16], MatchOutcome::FullFalse, 0x2015);
}

#[test]
fn config_16_match_sixteen_true() {
    run_match_configuration(16, &[16], MatchOutcome::FullTrue, 0x2016);
}

#[test]
fn config_17_match_large_even_early() {
    run_match_configuration(
        17,
        &[18, 20, 22, 24, 30, 32],
        MatchOutcome::EarlyReject,
        0x2017,
    );
}

#[test]
fn config_18_match_large_even_false() {
    run_match_configuration(
        18,
        &[18, 20, 22, 24, 30, 32],
        MatchOutcome::FullFalse,
        0x2018,
    );
}

#[test]
fn config_19_match_large_even_true() {
    run_match_configuration(
        19,
        &[18, 20, 22, 24, 30, 32],
        MatchOutcome::FullTrue,
        0x2019,
    );
}

#[test]
fn config_20_match_large_odd_early() {
    run_match_configuration(
        20,
        &[17, 19, 21, 23, 29, 31],
        MatchOutcome::EarlyReject,
        0x2020,
    );
}

#[test]
fn config_21_match_large_odd_false() {
    run_match_configuration(
        21,
        &[17, 19, 21, 23, 29, 31],
        MatchOutcome::FullFalse,
        0x2021,
    );
}

#[test]
fn config_22_match_large_odd_true() {
    run_match_configuration(
        22,
        &[17, 19, 21, 23, 29, 31],
        MatchOutcome::FullTrue,
        0x2022,
    );
}

fn load_match(library: &Library) -> Symbol<'_, MatchFn> {
    unsafe { library.get(b"match\0") }.expect("load match")
}

fn load_spectral(library: &Library) -> Symbol<'_, SpectralFn> {
    unsafe { library.get(b"spectral_contrast\0") }.expect("load spectral_contrast")
}

#[test]
fn error_01_energy_gate_rejection() {
    let c_library = unsafe { Library::new(c_library_path()) }.expect("load C library");
    let rust_library = unsafe { Library::new(rust_library_path()) }.expect("load Rust library");
    let c_fn = load_match(&c_library);
    let rust_fn = load_match(&rust_library);
    let mut c_test = [1.0, 2.0, 3.0];
    let mut c_reference = [1.0, 2.0, 3.0];
    let mut rust_test = c_test;
    let mut rust_reference = c_reference;
    let c_result = unsafe { c_fn(c_test.as_mut_ptr(), c_reference.as_mut_ptr(), 3, 1.5) };
    let rust_result =
        unsafe { rust_fn(rust_test.as_mut_ptr(), rust_reference.as_mut_ptr(), 3, 1.5) };
    assert_eq!(c_result, 0);
    assert_eq!(rust_result, c_result);
    assert_f64_buffers_equal(&c_test, &rust_test, "ERRORS row 1 test buffer");
    assert_f64_buffers_equal(
        &c_reference,
        &rust_reference,
        "ERRORS row 1 reference buffer",
    );
}

fn assert_spectral_nonpositive_length(length: i32, row: usize) {
    let c_library = unsafe { Library::new(c_library_path()) }.expect("load C library");
    let rust_library = unsafe { Library::new(rust_library_path()) }.expect("load Rust library");
    let c_fn = load_spectral(&c_library);
    let rust_fn = load_spectral(&rust_library);
    let c_result = unsafe { c_fn(std::ptr::null_mut(), std::ptr::null_mut(), length) };
    let rust_result = unsafe { rust_fn(std::ptr::null_mut(), std::ptr::null_mut(), length) };
    assert_eq!(c_result.to_bits(), 0, "ERRORS row {row}: C result");
    assert_eq!(
        rust_result.to_bits(),
        c_result.to_bits(),
        "ERRORS row {row}: Rust result"
    );
}

#[test]
fn error_02_spectral_zero_length_null() {
    assert_spectral_nonpositive_length(0, 2);
}

#[test]
fn error_03_spectral_negative_length_null() {
    for length in [-1, -2, i32::MIN] {
        assert_spectral_nonpositive_length(length, 3);
    }
}

fn run_boundary_child(library: &Path, case: &str) -> ExitStatus {
    Command::new(env::current_exe().expect("test executable"))
        .arg("--exact")
        .arg("boundary_child_dispatch")
        .arg("--nocapture")
        .env("DIFF_CHILD_LIBRARY", library)
        .env("DIFF_CHILD_CASE", case)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("run boundary child")
}

#[cfg(unix)]
fn assert_matching_sigsegv(case: &str, row: usize) {
    use std::os::unix::process::ExitStatusExt;

    let c_status = run_boundary_child(&c_library_path(), case);
    let rust_status = run_boundary_child(&rust_library_path(), case);
    assert_eq!(
        c_status.signal(),
        Some(SIGSEGV),
        "ERRORS row {row}: C status {c_status:?}"
    );
    assert_eq!(
        rust_status.signal(),
        c_status.signal(),
        "ERRORS row {row}: Rust status {rust_status:?}, C status {c_status:?}"
    );
}

#[test]
fn error_04_spectral_null_a() {
    assert_matching_sigsegv("spectral_null_a", 4);
}

#[test]
fn error_05_spectral_null_b() {
    assert_matching_sigsegv("spectral_null_b", 5);
}

#[test]
fn error_06_spectral_oversized_length() {
    assert_matching_sigsegv("spectral_oversized", 6);
}

#[test]
fn error_07_match_null_test() {
    assert_matching_sigsegv("match_null_test", 7);
}

#[test]
fn error_08_match_null_reference() {
    assert_matching_sigsegv("match_null_reference", 8);
}

#[test]
fn error_09_match_zero_bins() {
    assert_matching_sigsegv("match_zero", 9);
}

#[test]
fn error_10_match_negative_bins() {
    assert_matching_sigsegv("match_negative", 10);
}

#[test]
fn error_11_match_oversized_bins() {
    assert_matching_sigsegv("match_oversized", 11);
}

#[test]
fn boundary_child_dispatch() {
    let Ok(case) = env::var("DIFF_CHILD_CASE") else {
        return;
    };
    let library_path = env::var_os("DIFF_CHILD_LIBRARY").expect("child library path");
    let library = unsafe { Library::new(library_path) }.expect("load child library");

    match case.as_str() {
        "spectral_null_a" => {
            let function = load_spectral(&library);
            let mut b = f64::from_bits(1.0_f32.to_bits() as u64);
            unsafe {
                function(std::ptr::null_mut(), &mut b, 1);
            }
        }
        "spectral_null_b" => {
            let function = load_spectral(&library);
            let mut a = f64::from_bits(1.0_f32.to_bits() as u64);
            unsafe {
                function(&mut a, std::ptr::null_mut(), 1);
            }
        }
        "spectral_oversized" => {
            let function = load_spectral(&library);
            let mut a = f64::from_bits(1.0_f32.to_bits() as u64);
            let mut b = f64::from_bits(1.0_f32.to_bits() as u64);
            unsafe {
                function(&mut a, &mut b, i32::MAX);
            }
        }
        "match_null_test" => {
            let function = load_match(&library);
            let mut reference = 1.0;
            unsafe {
                function(std::ptr::null_mut(), &mut reference, 1, 0.0);
            }
        }
        "match_null_reference" => {
            let function = load_match(&library);
            let mut test = 1.0;
            unsafe {
                function(&mut test, std::ptr::null_mut(), 1, 0.0);
            }
        }
        "match_zero" => {
            let function = load_match(&library);
            unsafe {
                function(std::ptr::null_mut(), std::ptr::null_mut(), 0, 0.0);
            }
        }
        "match_negative" => {
            let function = load_match(&library);
            let mut test = 1.0;
            let mut reference = 1.0;
            unsafe {
                function(&mut test, &mut reference, -1, 0.0);
            }
        }
        "match_oversized" => {
            let function = load_match(&library);
            let mut test = 1.0;
            let mut reference = 1.0;
            unsafe {
                function(&mut test, &mut reference, i32::MAX, 0.0);
            }
        }
        other => panic!("unknown child case {other}"),
    }
}
