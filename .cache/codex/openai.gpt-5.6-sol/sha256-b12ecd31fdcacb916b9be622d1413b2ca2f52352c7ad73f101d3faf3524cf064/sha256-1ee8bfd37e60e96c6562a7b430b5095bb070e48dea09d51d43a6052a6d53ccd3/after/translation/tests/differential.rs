use libloading::Library;
use std::ffi::{c_double, c_int};
use std::path::{Path, PathBuf};
use std::process::Command;

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

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    if let Some(path) = std::env::var_os("C_SHARED_LIBRARY") {
        return path.into();
    }
    let build = manifest_dir().join("../c_src/build");
    let mut libraries = std::fs::read_dir(&build)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "so"))
        .collect::<Vec<_>>();
    libraries.sort();
    assert_eq!(
        libraries.len(),
        1,
        "expected exactly one C shared library in {}",
        build.display()
    );
    libraries.pop().unwrap()
}

fn rust_library_path() -> PathBuf {
    std::env::var_os("RUST_SHARED_LIBRARY")
        .map(PathBuf::from)
        .unwrap_or_else(|| manifest_dir().join("target/release/libunderhanded_c_nuke_lib.so"))
}

fn load_apis() -> (Api, Api) {
    unsafe {
        (
            Api::load(&c_library_path()),
            Api::load(&rust_library_path()),
        )
    }
}

fn bytes(values: &[f64]) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(values.as_ptr().cast::<u8>(), std::mem::size_of_val(values))
    }
}

fn assert_f64_bits_eq(c: f64, rust: f64, context: &str) {
    assert_eq!(
        c.to_bits(),
        rust.to_bits(),
        "{context}: C={c:?} ({:#018x}), Rust={rust:?} ({:#018x})",
        c.to_bits(),
        rust.to_bits()
    );
}

fn compare_spectral(a: &[f64], b: &[f64], length: i32) -> f64 {
    let (c_api, rust_api) = load_apis();
    let mut c_a = a.to_vec();
    let mut c_b = b.to_vec();
    let mut rust_a = a.to_vec();
    let mut rust_b = b.to_vec();
    let c_result = unsafe { (c_api.spectral_contrast)(c_a.as_mut_ptr(), c_b.as_mut_ptr(), length) };
    let rust_result =
        unsafe { (rust_api.spectral_contrast)(rust_a.as_mut_ptr(), rust_b.as_mut_ptr(), length) };
    assert_f64_bits_eq(c_result, rust_result, &format!("spectral length={length}"));
    assert_eq!(bytes(&c_a), bytes(&rust_a), "mutated a, length={length}");
    assert_eq!(bytes(&c_b), bytes(&rust_b), "mutated b, length={length}");
    c_result
}

fn compare_spectral_alias(values: &[f64], length: i32) -> f64 {
    let (c_api, rust_api) = load_apis();
    let mut c_values = values.to_vec();
    let mut rust_values = values.to_vec();
    let c_result =
        unsafe { (c_api.spectral_contrast)(c_values.as_mut_ptr(), c_values.as_mut_ptr(), length) };
    let rust_result = unsafe {
        (rust_api.spectral_contrast)(rust_values.as_mut_ptr(), rust_values.as_mut_ptr(), length)
    };
    assert_f64_bits_eq(
        c_result,
        rust_result,
        &format!("aliased spectral length={length}"),
    );
    assert_eq!(
        bytes(&c_values),
        bytes(&rust_values),
        "aliased mutation, length={length}"
    );
    c_result
}

fn compare_match(test: &[f64], reference: &[f64], threshold: f64) -> i32 {
    let (c_api, rust_api) = load_apis();
    let mut c_test = test.to_vec();
    let mut c_reference = reference.to_vec();
    let mut rust_test = test.to_vec();
    let mut rust_reference = reference.to_vec();
    let bins = test.len() as i32;
    assert_eq!(test.len(), reference.len());
    let c_result = unsafe {
        (c_api.matcher)(
            c_test.as_mut_ptr(),
            c_reference.as_mut_ptr(),
            bins,
            threshold,
        )
    };
    let rust_result = unsafe {
        (rust_api.matcher)(
            rust_test.as_mut_ptr(),
            rust_reference.as_mut_ptr(),
            bins,
            threshold,
        )
    };
    assert_eq!(
        c_result, rust_result,
        "match bins={bins}, threshold={threshold:?}"
    );
    assert_eq!(bytes(&c_test), bytes(&rust_test), "test input mutation");
    assert_eq!(
        bytes(&c_reference),
        bytes(&rust_reference),
        "reference input mutation"
    );
    c_result
}

fn compare_match_alias(values: &[f64], threshold: f64) -> i32 {
    let (c_api, rust_api) = load_apis();
    let mut c_values = values.to_vec();
    let mut rust_values = values.to_vec();
    let bins = values.len() as i32;
    let c_result = unsafe {
        (c_api.matcher)(
            c_values.as_mut_ptr(),
            c_values.as_mut_ptr(),
            bins,
            threshold,
        )
    };
    let rust_result = unsafe {
        (rust_api.matcher)(
            rust_values.as_mut_ptr(),
            rust_values.as_mut_ptr(),
            bins,
            threshold,
        )
    };
    assert_eq!(c_result, rust_result, "aliased match bins={bins}");
    assert_eq!(
        bytes(&c_values),
        bytes(&rust_values),
        "aliased input mutation"
    );
    c_result
}

#[derive(Clone)]
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

    fn f64(&mut self, low: f64, high: f64) -> f64 {
        let unit = (self.next_u64() >> 11) as f64 / ((1_u64 << 53) as f64);
        low + unit * (high - low)
    }

    fn nonzero_f32(&mut self) -> f32 {
        let value = self.f64(-8.0, 8.0) as f32;
        if value.abs() < 0.125 { 0.125 } else { value }
    }
}

fn spectral_buffer(length: usize, rng: &mut Rng) -> Vec<f64> {
    let mut values = vec![0.0_f64; length.max(1)];
    let slots = values.as_mut_ptr().cast::<f32>();
    for index in 0..length {
        unsafe {
            *slots.add(index) = rng.nonzero_f32();
        }
    }
    values
}

fn positive_values(length: usize, rng: &mut Rng) -> Vec<f64> {
    (0..length).map(|_| rng.f64(0.25, 4.0)).collect()
}

fn exercise_match_path(lengths: &[usize], threshold: f64, scale: f64, expected: i32) {
    let mut rng = Rng::new(0x8f3d_91ab_c572_4e61);
    let mut accepted = 0;
    for iteration in 0..10_000 {
        let length = lengths[iteration % lengths.len()];
        let reference = positive_values(length, &mut rng);
        let test = reference
            .iter()
            .map(|value| value * scale)
            .collect::<Vec<_>>();
        if compare_match(&test, &reference, threshold) == expected {
            accepted += 1;
            if accepted == 96 {
                return;
            }
        }
    }
    panic!("found only {accepted} randomized candidates for expected result {expected}");
}

#[test]
fn config_01_spectral_zero_length_and_null() {
    let (c_api, rust_api) = load_apis();
    let c_result =
        unsafe { (c_api.spectral_contrast)(std::ptr::null_mut(), std::ptr::null_mut(), 0) };
    let rust_result =
        unsafe { (rust_api.spectral_contrast)(std::ptr::null_mut(), std::ptr::null_mut(), 0) };
    assert_f64_bits_eq(c_result, rust_result, "zero-length null spectral");
    assert_eq!(c_result.to_bits(), 0.0_f64.to_bits());
}

#[test]
fn config_02_spectral_one_element() {
    let mut rng = Rng::new(0xa618_67e2_d39c_b405);
    for _ in 0..128 {
        let a = spectral_buffer(1, &mut rng);
        let b = spectral_buffer(1, &mut rng);
        compare_spectral(&a, &b, 1);
    }
}

#[test]
fn config_03_spectral_many_distinct() {
    let mut rng = Rng::new(0x49c7_0ef1_238a_b65d);
    let lengths = [2, 3, 7, 16, 17, 63, 257];
    for iteration in 0..128 {
        let length = lengths[iteration % lengths.len()];
        let a = spectral_buffer(length, &mut rng);
        let b = spectral_buffer(length, &mut rng);
        compare_spectral(&a, &b, length as i32);
    }
}

#[test]
fn config_04_spectral_zero_magnitude() {
    let mut rng = Rng::new(0x9ad2_8307_f164_5bce);
    let lengths = [2, 3, 9, 16, 31, 128];
    for iteration in 0..96 {
        let length = lengths[iteration % lengths.len()];
        let zero = vec![0.0; length];
        let nonzero = spectral_buffer(length, &mut rng);
        if iteration % 2 == 0 {
            compare_spectral(&zero, &nonzero, length as i32);
        } else {
            compare_spectral(&nonzero, &zero, length as i32);
        }
    }
}

#[test]
fn config_05_spectral_aliased_buffers() {
    let mut rng = Rng::new(0x283b_7fd0_149c_e6a5);
    let lengths = [2, 3, 8, 16, 17, 65];
    for iteration in 0..96 {
        let length = lengths[iteration % lengths.len()];
        let values = spectral_buffer(length, &mut rng);
        compare_spectral_alias(&values, length as i32);
    }
}

#[test]
fn config_06_match_one_bin_total_rejects() {
    exercise_match_path(&[1], 0.8, 0.25, 0);
}

#[test]
fn config_07_match_one_bin_contrast_rejects() {
    exercise_match_path(&[1], 1.1, 2.0, 0);
}

#[test]
fn config_08_match_short_total_rejects() {
    exercise_match_path(&[2, 3, 7, 15], 0.8, 0.25, 0);
}

#[test]
fn config_09_match_short_contrast_rejects() {
    exercise_match_path(&[2, 3, 7, 15], 1.1, 2.0, 0);
}

#[test]
fn config_10_match_short_both_gates_pass() {
    exercise_match_path(&[2, 3, 7, 15], -1.1, 1.2, 1);
}

#[test]
fn config_11_match_sixteen_total_rejects() {
    exercise_match_path(&[16], 0.8, 0.25, 0);
}

#[test]
fn config_12_match_sixteen_contrast_rejects() {
    exercise_match_path(&[16], 1.1, 2.0, 0);
}

#[test]
fn config_13_match_sixteen_both_gates_pass() {
    exercise_match_path(&[16], -1.1, 1.2, 1);
}

#[test]
fn config_14_match_long_total_rejects() {
    exercise_match_path(&[17, 33, 64, 257], 0.8, 0.25, 0);
}

#[test]
fn config_15_match_long_contrast_rejects() {
    exercise_match_path(&[17, 33, 64, 257], 1.1, 2.0, 0);
}

#[test]
fn config_16_match_long_both_gates_pass() {
    exercise_match_path(&[17, 33, 64, 257], -1.1, 1.2, 1);
}

#[test]
fn config_17_match_aliased_buffers() {
    let mut rng = Rng::new(0x57a9_c04e_316b_d82f);
    let lengths = [2, 7, 15, 16, 17, 33, 64, 257];
    for iteration in 0..128 {
        let length = lengths[iteration % lengths.len()];
        let values = positive_values(length, &mut rng);
        compare_match_alias(&values, 0.5);
    }
}

#[test]
fn config_18_match_nan_threshold() {
    let mut rng = Rng::new(0xd14f_68a3_95c2_07be);
    let lengths = [1, 2, 7, 16, 17, 64, 257];
    for iteration in 0..128 {
        let length = lengths[iteration % lengths.len()];
        let test = positive_values(length, &mut rng);
        let reference = positive_values(length, &mut rng);
        assert_eq!(compare_match(&test, &reference, f64::NAN), 0);
    }
}

#[test]
fn error_01_total_gate_rejection() {
    exercise_match_path(&[1, 2, 7, 16, 17, 64, 257], 0.75, 0.125, 0);
}

#[test]
fn error_02_final_contrast_rejection() {
    exercise_match_path(&[1, 2, 7, 16, 17, 64, 257], 1.25, 2.0, 0);
}

#[test]
fn boundary_spectral_negative_length_and_null() {
    let (c_api, rust_api) = load_apis();
    for length in [-1, i32::MIN] {
        let c_result = unsafe {
            (c_api.spectral_contrast)(std::ptr::null_mut(), std::ptr::null_mut(), length)
        };
        let rust_result = unsafe {
            (rust_api.spectral_contrast)(std::ptr::null_mut(), std::ptr::null_mut(), length)
        };
        assert_f64_bits_eq(c_result, rust_result, &format!("negative length={length}"));
        assert_eq!(c_result.to_bits(), 0.0_f64.to_bits());
    }
}

#[test]
fn ffi_crash_probe() {
    let Some(probe) = std::env::var_os("FFI_CRASH_PROBE") else {
        return;
    };
    let probe = probe.to_str().unwrap();
    let (side, case) = probe.split_once(':').unwrap();
    let library_path = match side {
        "c" => c_library_path(),
        "rust" => rust_library_path(),
        _ => panic!("unknown side {side}"),
    };
    let api = unsafe { Api::load(&library_path) };
    let mut value = 1.0_f64;
    unsafe {
        match case {
            "spectral-null-positive" => {
                (api.spectral_contrast)(std::ptr::null_mut(), std::ptr::null_mut(), 1);
            }
            "spectral-null-oversized" => {
                (api.spectral_contrast)(std::ptr::null_mut(), std::ptr::null_mut(), c_int::MAX);
            }
            "match-null-positive" => {
                (api.matcher)(std::ptr::null_mut(), std::ptr::null_mut(), 1, 0.5);
            }
            "match-zero" => {
                (api.matcher)(&mut value, &mut value, 0, 0.5);
            }
            "match-negative" => {
                (api.matcher)(&mut value, &mut value, -1, 0.5);
            }
            "match-null-oversized" => {
                (api.matcher)(std::ptr::null_mut(), std::ptr::null_mut(), c_int::MAX, 0.5);
            }
            _ => panic!("unknown probe case {case}"),
        }
    }
    panic!("probe unexpectedly returned: {probe}");
}

#[cfg(unix)]
fn probe_signal(side: &str, case: &str) -> Option<i32> {
    use std::os::unix::process::ExitStatusExt;

    Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "ffi_crash_probe"])
        .env("FFI_CRASH_PROBE", format!("{side}:{case}"))
        .output()
        .unwrap()
        .status
        .signal()
}

#[test]
#[cfg(unix)]
fn boundary_unchecked_inputs_match_process_signal() {
    let cases = [
        "spectral-null-positive",
        "spectral-null-oversized",
        "match-null-positive",
        "match-zero",
        "match-negative",
        "match-null-oversized",
    ];
    for case in cases {
        let c_signal = probe_signal("c", case);
        let rust_signal = probe_signal("rust", case);
        assert_eq!(
            c_signal, rust_signal,
            "different process signal for boundary case {case}: C={c_signal:?}, Rust={rust_signal:?}"
        );
        assert_eq!(
            c_signal,
            Some(11),
            "C boundary case did not SIGSEGV: {case}"
        );
    }
}
