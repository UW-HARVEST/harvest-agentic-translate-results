use libloading::{Library, Symbol};
use std::env;
use std::ffi::{c_float, c_int};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use std::ptr;
use std::sync::OnceLock;

type Normalize = unsafe extern "C" fn(*mut c_float, *const c_float, c_int);

const RANDOM_CASES: usize = 128;

struct Api {
    _library: Library,
    normalize: Normalize,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let normalize = {
            let symbol: Symbol<Normalize> =
                unsafe { library.get(b"normalize\0") }.unwrap_or_else(|error| {
                    panic!("normalize missing from {}: {error}", path.display())
                });
            *symbol
        };
        Self {
            _library: library,
            normalize,
        }
    }

    unsafe fn call(&self, dest: *mut u32, src: *const u32, size: c_int) {
        unsafe { (self.normalize)(dest.cast::<c_float>(), src.cast::<c_float>(), size) };
    }
}

#[derive(Clone)]
struct Rng(u64);

impl Rng {
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

    fn next_u32(&mut self) -> u32 {
        self.next_u64() as u32
    }

    fn len(&mut self, minimum: usize, maximum_inclusive: usize) -> usize {
        minimum + self.next_u64() as usize % (maximum_inclusive - minimum + 1)
    }

    fn moderate_finite_bits(&mut self) -> u32 {
        let sign = self.next_u32() & 0x8000_0000;
        let exponent = (120 + self.next_u32() % 12) << 23;
        let fraction = self.next_u32() & 0x007f_ffff;
        sign | exponent | fraction
    }

    fn arbitrary_words(&mut self, count: usize) -> Vec<u32> {
        (0..count).map(|_| self.next_u32()).collect()
    }
}

fn command_succeeded(command: &mut Command, description: &str) {
    let output = command
        .output()
        .unwrap_or_else(|error| panic!("failed to run {description}: {error}"));
    assert!(
        output.status.success(),
        "{description} failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn library_paths() -> &'static (PathBuf, PathBuf) {
    static PATHS: OnceLock<(PathBuf, PathBuf)> = OnceLock::new();
    PATHS.get_or_init(|| {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_source = root.join("c_src");
        let c_build = c_source.join("build");
        std::fs::create_dir_all(&c_build).expect("create C build directory");
        command_succeeded(
            Command::new("cmake")
                .arg("..")
                .arg("-DCMAKE_POSITION_INDEPENDENT_CODE=ON")
                .current_dir(&c_build),
            "CMake configuration",
        );
        command_succeeded(
            Command::new("cmake")
                .args(["--build", "."])
                .current_dir(&c_build),
            "C shared-library build",
        );

        let rust_target = root.join("target/differential");
        let cargo = env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
        command_succeeded(
            Command::new(cargo)
                .args([
                    "build",
                    "--release",
                    "--no-default-features",
                    "--features",
                    "",
                    "--target-dir",
                ])
                .arg(&rust_target)
                .current_dir(&root),
            "Rust shared-library build",
        );

        let c_library = c_build.join("libtranslated_rust.so");
        let rust_library = rust_target.join("release/libnormalize_lib.so");
        assert!(c_library.is_file(), "{} is missing", c_library.display());
        assert!(
            rust_library.is_file(),
            "{} is missing",
            rust_library.display()
        );
        (c_library, rust_library)
    })
}

fn apis() -> (Api, Api) {
    let (c_path, rust_path) = library_paths();
    unsafe { (Api::load(c_path), Api::load(rust_path)) }
}

fn compare_separate(row: &str, source: &[u32], destination: &[u32]) {
    assert_eq!(source.len(), destination.len());
    let (c_api, rust_api) = apis();
    let c_source = source.to_vec();
    let rust_source = source.to_vec();
    let mut c_destination = destination.to_vec();
    let mut rust_destination = destination.to_vec();
    unsafe {
        c_api.call(
            c_destination.as_mut_ptr(),
            c_source.as_ptr(),
            source.len() as c_int,
        );
        rust_api.call(
            rust_destination.as_mut_ptr(),
            rust_source.as_ptr(),
            source.len() as c_int,
        );
    }
    assert_eq!(c_destination, rust_destination, "{row}: destination");
    assert_eq!(c_source, rust_source, "{row}: source");
}

fn compare_in_place(row: &str, input: &[u32]) {
    let (c_api, rust_api) = apis();
    let mut c_buffer = input.to_vec();
    let mut rust_buffer = input.to_vec();
    unsafe {
        c_api.call(
            c_buffer.as_mut_ptr(),
            c_buffer.as_ptr(),
            input.len() as c_int,
        );
        rust_api.call(
            rust_buffer.as_mut_ptr(),
            rust_buffer.as_ptr(),
            input.len() as c_int,
        );
    }
    assert_eq!(c_buffer, rust_buffer, "{row}");
}

fn compare_overlap(
    row: &str,
    initial: &[u32],
    destination_offset: usize,
    source_offset: usize,
    size: usize,
) {
    assert!(destination_offset + size <= initial.len());
    assert!(source_offset + size <= initial.len());
    let (c_api, rust_api) = apis();
    let mut c_buffer = initial.to_vec();
    let mut rust_buffer = initial.to_vec();
    unsafe {
        c_api.call(
            c_buffer.as_mut_ptr().add(destination_offset),
            c_buffer.as_ptr().add(source_offset),
            size as c_int,
        );
        rust_api.call(
            rust_buffer.as_mut_ptr().add(destination_offset),
            rust_buffer.as_ptr().add(source_offset),
            size as c_int,
        );
    }
    assert_eq!(c_buffer, rust_buffer, "{row}");
}

#[test]
fn config_c01_zero_size_distinct() {
    let mut rng = Rng::new(0xc01);
    for _ in 0..RANDOM_CASES {
        let source = [rng.next_u32()];
        let destination = [rng.next_u32()];
        let (c_api, rust_api) = apis();
        let mut c_destination = destination;
        let mut rust_destination = destination;
        unsafe {
            c_api.call(c_destination.as_mut_ptr(), source.as_ptr(), 0);
            rust_api.call(rust_destination.as_mut_ptr(), source.as_ptr(), 0);
        }
        assert_eq!(c_destination, rust_destination, "C1");
    }
}

#[test]
fn config_c02_zero_size_in_place() {
    let mut rng = Rng::new(0xc02);
    for _ in 0..RANDOM_CASES {
        let input = [rng.next_u32()];
        let (c_api, rust_api) = apis();
        let mut c_buffer = input;
        let mut rust_buffer = input;
        unsafe {
            c_api.call(c_buffer.as_mut_ptr(), c_buffer.as_ptr(), 0);
            rust_api.call(rust_buffer.as_mut_ptr(), rust_buffer.as_ptr(), 0);
        }
        assert_eq!(c_buffer, rust_buffer, "C2");
    }
}

#[test]
fn config_c03_single_finite_value() {
    let mut rng = Rng::new(0xc03);
    for _ in 0..RANDOM_CASES {
        let source = [rng.moderate_finite_bits()];
        compare_separate("C3", &source, &[rng.next_u32()]);
    }
}

#[test]
fn config_c04_many_finite_values_separate() {
    let mut rng = Rng::new(0xc04);
    for _ in 0..RANDOM_CASES {
        let size = rng.len(2, 64);
        let source: Vec<_> = (0..size).map(|_| rng.moderate_finite_bits()).collect();
        let destination = rng.arbitrary_words(size);
        compare_separate("C4", &source, &destination);
    }
}

#[test]
fn config_c05_many_finite_values_in_place() {
    let mut rng = Rng::new(0xc05);
    for _ in 0..RANDOM_CASES {
        let size = rng.len(2, 64);
        let input: Vec<_> = (0..size).map(|_| rng.moderate_finite_bits()).collect();
        compare_in_place("C5", &input);
    }
}

#[test]
fn config_c06_forward_overlap() {
    let mut rng = Rng::new(0xc06);
    for _ in 0..RANDOM_CASES {
        let size = rng.len(2, 64);
        let mut buffer = rng.arbitrary_words(size + 1);
        for word in &mut buffer[..size] {
            *word = rng.moderate_finite_bits();
        }
        compare_overlap("C6", &buffer, 1, 0, size);
    }
}

#[test]
fn config_c07_backward_overlap() {
    let mut rng = Rng::new(0xc07);
    for _ in 0..RANDOM_CASES {
        let size = rng.len(2, 64);
        let mut buffer = rng.arbitrary_words(size + 1);
        for word in &mut buffer[1..] {
            *word = rng.moderate_finite_bits();
        }
        compare_overlap("C7", &buffer, 0, 1, size);
    }
}

#[test]
fn config_c08_signed_zeros_separate() {
    let mut rng = Rng::new(0xc08);
    for _ in 0..RANDOM_CASES {
        let size = rng.len(2, 64);
        let source: Vec<_> = (0..size).map(|_| rng.next_u32() & 0x8000_0000).collect();
        compare_separate("C8", &source, &rng.arbitrary_words(size));
    }
}

#[test]
fn config_c09_signed_zeros_in_place() {
    let mut rng = Rng::new(0xc09);
    for _ in 0..RANDOM_CASES {
        let size = rng.len(2, 64);
        let input: Vec<_> = (0..size).map(|_| rng.next_u32() & 0x8000_0000).collect();
        compare_in_place("C9", &input);
    }
}

#[test]
fn config_c10_underflow_to_zero_separate() {
    let mut rng = Rng::new(0xc10);
    for _ in 0..RANDOM_CASES {
        let size = rng.len(2, 64);
        let source: Vec<_> = (0..size)
            .map(|_| (rng.next_u32() & 0x807f_ffff) | 1)
            .collect();
        compare_separate("C10", &source, &rng.arbitrary_words(size));
    }
}

#[test]
fn config_c11_underflow_to_zero_in_place() {
    let mut rng = Rng::new(0xc11);
    for _ in 0..RANDOM_CASES {
        let size = rng.len(2, 64);
        let input: Vec<_> = (0..size)
            .map(|_| (rng.next_u32() & 0x807f_ffff) | 1)
            .collect();
        compare_in_place("C11", &input);
    }
}

fn nan_vector(rng: &mut Rng, size: usize) -> Vec<u32> {
    let mut input: Vec<_> = (0..size).map(|_| rng.moderate_finite_bits()).collect();
    let index = rng.next_u64() as usize % size;
    input[index] = (rng.next_u32() & 0x807f_ffff) | 0x7fc0_0000;
    input
}

#[test]
fn config_c12_nan_separate() {
    let mut rng = Rng::new(0xc12);
    for _ in 0..RANDOM_CASES {
        let size = rng.len(2, 64);
        compare_separate(
            "C12",
            &nan_vector(&mut rng, size),
            &rng.arbitrary_words(size),
        );
    }
}

#[test]
fn config_c13_nan_in_place() {
    let mut rng = Rng::new(0xc13);
    for _ in 0..RANDOM_CASES {
        let size = rng.len(2, 64);
        compare_in_place("C13", &nan_vector(&mut rng, size));
    }
}

#[test]
fn config_c14_finite_sum_overflow() {
    let mut rng = Rng::new(0xc14);
    for _ in 0..RANDOM_CASES {
        let size = rng.len(2, 64);
        let source: Vec<_> = (0..size)
            .map(|_| {
                let sign = rng.next_u32() & 0x8000_0000;
                sign | 0x7e00_0000 | (rng.next_u32() & 0x007f_ffff)
            })
            .collect();
        compare_separate("C14", &source, &rng.arbitrary_words(size));
    }
}

#[test]
fn config_c15_infinity_input() {
    let mut rng = Rng::new(0xc15);
    for _ in 0..RANDOM_CASES {
        let size = rng.len(2, 64);
        let mut source: Vec<_> = (0..size).map(|_| rng.moderate_finite_bits()).collect();
        let index = rng.next_u64() as usize % size;
        source[index] = 0x7f80_0000 | (rng.next_u32() & 0x8000_0000);
        compare_separate("C15", &source, &rng.arbitrary_words(size));
    }
}

#[test]
fn config_c16_nonpositive_sum_partial_overlap() {
    let mut rng = Rng::new(0xc16);
    for case in 0..RANDOM_CASES {
        let size = rng.len(2, 64);
        let mut buffer = rng.arbitrary_words(size + 1);
        for word in &mut buffer[..size] {
            *word = rng.next_u32() & 0x8000_0000;
        }
        if case % 2 == 1 {
            buffer[rng.next_u64() as usize % size] = (rng.next_u32() & 0x807f_ffff) | 0x7fc0_0000;
        }
        compare_overlap("C16", &buffer, 1, 0, size);
    }
}

#[test]
fn config_c17_large_valid_vectors() {
    let mut rng = Rng::new(0xc17);
    for _ in 0..32 {
        let source: Vec<_> = (0..4096).map(|_| rng.moderate_finite_bits()).collect();
        compare_separate("C17", &source, &rng.arbitrary_words(4096));
    }
}

#[test]
fn config_c18_mixed_finite_values() {
    let mut rng = Rng::new(0xc18);
    for _ in 0..RANDOM_CASES {
        let size = rng.len(2, 128);
        let mut source = Vec::with_capacity(size);
        for _ in 0..size {
            let sign = rng.next_u32() & 0x8000_0000;
            let exponent = (100 + rng.next_u32() % 45) << 23;
            source.push(sign | exponent | (rng.next_u32() & 0x007f_ffff));
        }
        compare_separate("C18", &source, &rng.arbitrary_words(size));
    }
}

#[test]
fn config_c19_positive_sum_arbitrary_destination() {
    let mut rng = Rng::new(0xc19);
    for _ in 0..RANDOM_CASES {
        let size = rng.len(1, 128);
        let source: Vec<_> = (0..size).map(|_| rng.moderate_finite_bits()).collect();
        compare_separate("C19", &source, &rng.arbitrary_words(size));
    }
}

#[test]
fn config_c20_zero_or_nan_sum_arbitrary_destination() {
    let mut rng = Rng::new(0xc20);
    for case in 0..RANDOM_CASES {
        let size = rng.len(1, 128);
        let mut source: Vec<_> = (0..size).map(|_| rng.next_u32() & 0x8000_0000).collect();
        if case % 2 == 1 {
            source[rng.next_u64() as usize % size] = (rng.next_u32() & 0x807f_ffff) | 0x7fc0_0000;
        }
        compare_separate("C20", &source, &rng.arbitrary_words(size));
    }
}

#[test]
fn boundary_b01_null_zero_size() {
    let (c_api, rust_api) = apis();
    unsafe {
        c_api.call(ptr::null_mut(), ptr::null(), 0);
        rust_api.call(ptr::null_mut(), ptr::null(), 0);
    }
}

#[test]
fn boundary_b02_negative_one_in_place() {
    let (c_api, rust_api) = apis();
    let mut c_word = 0xdead_beef;
    let mut rust_word = c_word;
    unsafe {
        c_api.call(&mut c_word, &c_word, -1);
        rust_api.call(&mut rust_word, &rust_word, -1);
    }
    assert_eq!(c_word, rust_word);
}

#[test]
fn boundary_b03_int_min_in_place() {
    let (c_api, rust_api) = apis();
    let mut c_word = 0xdead_beef;
    let mut rust_word = c_word;
    unsafe {
        c_api.call(&mut c_word, &c_word, c_int::MIN);
        rust_api.call(&mut rust_word, &rust_word, c_int::MIN);
    }
    assert_eq!(c_word, rust_word);
}

#[derive(Debug, PartialEq, Eq)]
enum ProcessOutcome {
    Exit(Option<i32>),
    Signal(i32),
}

#[cfg(unix)]
fn process_outcome(status: ExitStatus) -> ProcessOutcome {
    use std::os::unix::process::ExitStatusExt;
    match status.signal() {
        Some(signal) => ProcessOutcome::Signal(signal),
        None => ProcessOutcome::Exit(status.code()),
    }
}

fn run_fault_probe(library: &Path, case: &str) -> ProcessOutcome {
    let status = Command::new(env::current_exe().expect("current test executable"))
        .args([
            "--exact",
            "crash_probe_worker",
            "--ignored",
            "--test-threads=1",
        ])
        .env("NORMALIZE_PROBE_LIBRARY", library)
        .env("NORMALIZE_PROBE_CASE", case)
        .status()
        .expect("run crash probe");
    process_outcome(status)
}

fn assert_fault_parity(row: &str, case: &str) {
    let (c_library, rust_library) = library_paths();
    let c_outcome = run_fault_probe(c_library, case);
    let rust_outcome = run_fault_probe(rust_library, case);
    assert!(
        matches!(c_outcome, ProcessOutcome::Signal(_)),
        "{row}: C unexpectedly returned: {c_outcome:?}"
    );
    assert_eq!(c_outcome, rust_outcome, "{row}");
}

#[test]
fn boundary_b04_null_source_positive_size() {
    assert_fault_parity("B4", "null_source");
}

#[test]
fn boundary_b05_null_destination_positive_size() {
    assert_fault_parity("B5", "null_destination");
}

#[test]
fn boundary_b06_negative_size_distinct_buffers() {
    assert_fault_parity("B6", "negative_distinct");
}

#[test]
fn boundary_b07_int_max_null_buffers() {
    assert_fault_parity("B7", "int_max_null");
}

#[test]
#[ignore = "launched in a subprocess by the boundary fault-parity tests"]
fn crash_probe_worker() {
    let Some(path) = env::var_os("NORMALIZE_PROBE_LIBRARY") else {
        return;
    };
    let case = env::var("NORMALIZE_PROBE_CASE").expect("NORMALIZE_PROBE_CASE");
    let api = unsafe { Api::load(Path::new(&path)) };
    let mut destination = 0_u32;
    let source = 1.0_f32.to_bits();
    unsafe {
        match case.as_str() {
            "null_source" => api.call(&mut destination, ptr::null(), 1),
            "null_destination" => api.call(ptr::null_mut(), &source, 1),
            "negative_distinct" => api.call(&mut destination, &source, -1),
            "int_max_null" => api.call(ptr::null_mut(), ptr::null(), c_int::MAX),
            _ => panic!("unknown probe case: {case}"),
        }
    }
}
