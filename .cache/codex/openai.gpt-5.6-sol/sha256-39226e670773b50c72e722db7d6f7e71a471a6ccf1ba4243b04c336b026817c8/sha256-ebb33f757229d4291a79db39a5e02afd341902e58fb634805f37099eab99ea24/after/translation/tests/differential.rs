use libloading::Library;
use std::env;
use std::ffi::c_int;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

type Normalize = unsafe extern "C" fn(*mut f32, *const f32, c_int);

struct Loaded {
    _library: Library,
    normalize: Normalize,
}

impl Loaded {
    unsafe fn open(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let normalize = {
            let symbol = unsafe { library.get::<Normalize>(b"normalize\0") }
                .unwrap_or_else(|error| panic!("failed to load normalize: {error}"));
            *symbol
        };
        Self {
            _library: library,
            normalize,
        }
    }
}

struct Libraries {
    c: Loaded,
    rust: Loaded,
}

impl Libraries {
    fn open() -> Self {
        let c_path = c_library_path();
        let rust_path = rust_library_path();
        assert!(c_path.is_file(), "missing C library: {}", c_path.display());
        assert!(
            rust_path.is_file(),
            "missing Rust library: {} (run cargo build --release first)",
            rust_path.display()
        );
        unsafe {
            Self {
                c: Loaded::open(&c_path),
                rust: Loaded::open(&rust_path),
            }
        }
    }
}

#[derive(Clone, Copy)]
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u32(&mut self) -> u32 {
        let mut value = self.0;
        value ^= value >> 12;
        value ^= value << 25;
        value ^= value >> 27;
        self.0 = value;
        (value.wrapping_mul(0x2545_f491_4f6c_dd1d) >> 32) as u32
    }

    fn usize(&mut self, low: usize, high_exclusive: usize) -> usize {
        low + (self.next_u32() as usize % (high_exclusive - low))
    }

    fn finite_nonzero_bits(&mut self) -> u32 {
        let magnitude = 1 + self.next_u32() % 1_000_000;
        let value = magnitude as f32 / 257.0;
        if self.next_u32() & 1 == 0 {
            value.to_bits()
        } else {
            (-value).to_bits()
        }
    }

    fn arbitrary_finite_bits(&mut self) -> u32 {
        if self.next_u32() % 13 == 0 {
            (self.next_u32() & 1) << 31
        } else {
            self.finite_nonzero_bits()
        }
    }

    fn destination_bits(&mut self) -> u32 {
        match self.next_u32() % 8 {
            0 => 0,
            1 => 0x8000_0000,
            2 => f32::INFINITY.to_bits(),
            3 => f32::NEG_INFINITY.to_bits(),
            4 => 0x7fc0_0000 | (self.next_u32() & 0x003f_ffff),
            _ => self.next_u32(),
        }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    if let Some(path) = env::var_os("C_NORMALIZE_LIB") {
        return PathBuf::from(path);
    }
    let build = manifest_dir().join("../c_src/build");
    let mut libraries: Vec<_> = fs::read_dir(&build)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", build.display()))
        .map(|entry| entry.expect("invalid C build directory entry").path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("lib") && name.ends_with(".so"))
        })
        .collect();
    libraries.sort();
    assert_eq!(
        libraries.len(),
        1,
        "expected exactly one C shared library in {}",
        build.display()
    );
    libraries.remove(0)
}

fn rust_library_path() -> PathBuf {
    env::var_os("RUST_NORMALIZE_LIB")
        .map(PathBuf::from)
        .unwrap_or_else(|| manifest_dir().join("target/release/libnormalize_lib.so"))
}

fn as_floats(bits: &[u32]) -> Vec<f32> {
    bits.iter().copied().map(f32::from_bits).collect()
}

fn as_bits(values: &[f32]) -> Vec<u32> {
    values.iter().map(|value| value.to_bits()).collect()
}

fn call_disjoint(
    normalize: Normalize,
    source_bits: &[u32],
    destination_bits: &[u32],
    size: c_int,
) -> (Vec<u32>, Vec<u32>) {
    let source = as_floats(source_bits);
    let mut destination = as_floats(destination_bits);
    unsafe {
        normalize(destination.as_mut_ptr(), source.as_ptr(), size);
    }
    (as_bits(&destination), as_bits(&source))
}

fn compare_disjoint(
    libraries: &Libraries,
    source_bits: &[u32],
    destination_bits: &[u32],
    size: c_int,
    label: &str,
) {
    let c = call_disjoint(libraries.c.normalize, source_bits, destination_bits, size);
    let rust = call_disjoint(
        libraries.rust.normalize,
        source_bits,
        destination_bits,
        size,
    );
    assert_eq!(rust, c, "{label}");
}

fn call_backing(
    normalize: Normalize,
    initial_bits: &[u32],
    destination_offset: usize,
    source_offset: usize,
    size: c_int,
) -> Vec<u32> {
    let mut backing = as_floats(initial_bits);
    unsafe {
        normalize(
            backing.as_mut_ptr().add(destination_offset),
            backing.as_ptr().add(source_offset),
            size,
        );
    }
    as_bits(&backing)
}

fn compare_backing(
    libraries: &Libraries,
    initial_bits: &[u32],
    destination_offset: usize,
    source_offset: usize,
    size: c_int,
    label: &str,
) {
    let c = call_backing(
        libraries.c.normalize,
        initial_bits,
        destination_offset,
        source_offset,
        size,
    );
    let rust = call_backing(
        libraries.rust.normalize,
        initial_bits,
        destination_offset,
        source_offset,
        size,
    );
    assert_eq!(rust, c, "{label}");
}

fn random_destination(rng: &mut Rng, length: usize) -> Vec<u32> {
    (0..length).map(|_| rng.destination_bits()).collect()
}

fn finite_positive_source(rng: &mut Rng, length: usize) -> Vec<u32> {
    let mut source: Vec<_> = (0..length).map(|_| rng.arbitrary_finite_bits()).collect();
    source[rng.usize(0, length)] = rng.finite_nonzero_bits();
    source
}

fn zero_source(rng: &mut Rng, length: usize) -> Vec<u32> {
    (0..length)
        .map(|_| {
            if rng.next_u32() & 1 == 0 {
                0
            } else {
                0x8000_0000
            }
        })
        .collect()
}

fn nan_source(rng: &mut Rng, length: usize) -> Vec<u32> {
    let mut source = finite_positive_source(rng, length);
    let payload = 1 | (rng.next_u32() & 0x003f_ffff);
    source[rng.usize(0, length)] = 0x7fc0_0000 | payload;
    source
}

fn infinite_sum_source(rng: &mut Rng, length: usize) -> Vec<u32> {
    let mut source = finite_positive_source(rng, length);
    let index = rng.usize(0, length);
    source[index] = match rng.next_u32() % 4 {
        0 => f32::INFINITY.to_bits(),
        1 => f32::NEG_INFINITY.to_bits(),
        2 => f32::MAX.to_bits(),
        _ => (-f32::MAX).to_bits(),
    };
    source
}

fn subnormal_underflow_source(rng: &mut Rng, length: usize) -> Vec<u32> {
    (0..length)
        .map(|_| {
            let magnitude = 1 + rng.next_u32() % 1024;
            magnitude | ((rng.next_u32() & 1) << 31)
        })
        .collect()
}

fn overlapping_backing(
    rng: &mut Rng,
    source: &[u32],
    destination_offset: usize,
    source_offset: usize,
) -> Vec<u32> {
    let length = source.len() + destination_offset.max(source_offset);
    let mut backing = random_destination(rng, length);
    backing[source_offset..source_offset + source.len()].copy_from_slice(source);
    backing
}

#[test]
fn all_valid_configuration_rows_match_byte_for_byte() {
    let libraries = Libraries::open();
    let mut rng = Rng::new(0x7a6f_4e12_c93b_580d);

    // CONFIGS.md rows 1-2: zero length, distinct and exact-alias pointers.
    for iteration in 0..256 {
        let source = random_destination(&mut rng, 1);
        let destination = random_destination(&mut rng, 1);
        compare_disjoint(
            &libraries,
            &source,
            &destination,
            0,
            &format!("row 1 iteration {iteration}"),
        );
        compare_backing(
            &libraries,
            &source,
            0,
            0,
            0,
            &format!("row 2 iteration {iteration}"),
        );
    }

    // CONFIGS.md rows 3-4: one finite nonzero value, disjoint and in-place.
    for iteration in 0..256 {
        let source = vec![rng.finite_nonzero_bits()];
        let destination = random_destination(&mut rng, 1);
        compare_disjoint(
            &libraries,
            &source,
            &destination,
            1,
            &format!("row 3 iteration {iteration}"),
        );
        compare_backing(
            &libraries,
            &source,
            0,
            0,
            1,
            &format!("row 4 iteration {iteration}"),
        );
    }

    // CONFIGS.md rows 5-8: many finite values across all alias layouts.
    for iteration in 0..256 {
        let length = rng.usize(2, 65);
        let source = finite_positive_source(&mut rng, length);
        let destination = random_destination(&mut rng, length);
        compare_disjoint(
            &libraries,
            &source,
            &destination,
            length as c_int,
            &format!("row 5 iteration {iteration}"),
        );
        compare_backing(
            &libraries,
            &source,
            0,
            0,
            length as c_int,
            &format!("row 6 iteration {iteration}"),
        );
        let before = overlapping_backing(&mut rng, &source, 0, 1);
        compare_backing(
            &libraries,
            &before,
            0,
            1,
            length as c_int,
            &format!("row 7 iteration {iteration}"),
        );
        let after = overlapping_backing(&mut rng, &source, 1, 0);
        compare_backing(
            &libraries,
            &after,
            1,
            0,
            length as c_int,
            &format!("row 8 iteration {iteration}"),
        );
    }

    // CONFIGS.md rows 9-10: one signed zero, disjoint and in-place.
    for iteration in 0..256 {
        let source = zero_source(&mut rng, 1);
        let destination = random_destination(&mut rng, 1);
        compare_disjoint(
            &libraries,
            &source,
            &destination,
            1,
            &format!("row 9 iteration {iteration}"),
        );
        compare_backing(
            &libraries,
            &source,
            0,
            0,
            1,
            &format!("row 10 iteration {iteration}"),
        );
    }

    // CONFIGS.md rows 11-14: many signed zeros across all alias layouts.
    for iteration in 0..256 {
        let length = rng.usize(2, 65);
        let source = zero_source(&mut rng, length);
        let destination = random_destination(&mut rng, length);
        compare_disjoint(
            &libraries,
            &source,
            &destination,
            length as c_int,
            &format!("row 11 iteration {iteration}"),
        );
        compare_backing(
            &libraries,
            &source,
            0,
            0,
            length as c_int,
            &format!("row 12 iteration {iteration}"),
        );
        let before = overlapping_backing(&mut rng, &source, 0, 1);
        compare_backing(
            &libraries,
            &before,
            0,
            1,
            length as c_int,
            &format!("row 13 iteration {iteration}"),
        );
        let after = overlapping_backing(&mut rng, &source, 1, 0);
        compare_backing(
            &libraries,
            &after,
            1,
            0,
            length as c_int,
            &format!("row 14 iteration {iteration}"),
        );
    }

    // CONFIGS.md rows 15-17: NaN sum, disjoint, exact alias, and both overlaps.
    for iteration in 0..256 {
        let length = rng.usize(1, 65);
        let source = nan_source(&mut rng, length);
        let destination = random_destination(&mut rng, length);
        compare_disjoint(
            &libraries,
            &source,
            &destination,
            length as c_int,
            &format!("row 15 iteration {iteration}"),
        );
        compare_backing(
            &libraries,
            &source,
            0,
            0,
            length as c_int,
            &format!("row 16 iteration {iteration}"),
        );
        if length > 1 {
            for (destination_offset, source_offset) in [(0, 1), (1, 0)] {
                let backing =
                    overlapping_backing(&mut rng, &source, destination_offset, source_offset);
                compare_backing(
                    &libraries,
                    &backing,
                    destination_offset,
                    source_offset,
                    length as c_int,
                    &format!(
                        "row 17 iteration {iteration} offsets {destination_offset}/{source_offset}"
                    ),
                );
            }
        }
    }

    // CONFIGS.md rows 18-21: infinite sum across all alias layouts.
    for iteration in 0..256 {
        let length = rng.usize(2, 65);
        let source = infinite_sum_source(&mut rng, length);
        let destination = random_destination(&mut rng, length);
        compare_disjoint(
            &libraries,
            &source,
            &destination,
            length as c_int,
            &format!("row 18 iteration {iteration}"),
        );
        compare_backing(
            &libraries,
            &source,
            0,
            0,
            length as c_int,
            &format!("row 19 iteration {iteration}"),
        );
        let before = overlapping_backing(&mut rng, &source, 0, 1);
        compare_backing(
            &libraries,
            &before,
            0,
            1,
            length as c_int,
            &format!("row 20 iteration {iteration}"),
        );
        let after = overlapping_backing(&mut rng, &source, 1, 0);
        compare_backing(
            &libraries,
            &after,
            1,
            0,
            length as c_int,
            &format!("row 21 iteration {iteration}"),
        );
    }

    // CONFIGS.md rows 22-23: nonzero subnormals whose squares underflow.
    for iteration in 0..256 {
        let length = rng.usize(1, 65);
        let source = subnormal_underflow_source(&mut rng, length);
        let destination = random_destination(&mut rng, length);
        compare_disjoint(
            &libraries,
            &source,
            &destination,
            length as c_int,
            &format!("row 22 iteration {iteration}"),
        );
        compare_backing(
            &libraries,
            &source,
            0,
            0,
            length as c_int,
            &format!("row 23 iteration {iteration}"),
        );
    }

    // CONFIGS.md row 24: negative size with exact alias, including null.
    for iteration in 0..256 {
        let size = -1 - (rng.next_u32() % (c_int::MAX as u32)) as c_int;
        let initial = random_destination(&mut rng, 4);
        compare_backing(
            &libraries,
            &initial,
            0,
            0,
            size,
            &format!("row 24 iteration {iteration}"),
        );
        unsafe {
            (libraries.c.normalize)(std::ptr::null_mut(), std::ptr::null(), size);
            (libraries.rust.normalize)(std::ptr::null_mut(), std::ptr::null(), size);
        }
    }
}

#[test]
fn generic_ffi_boundaries_match_process_outcomes() {
    let cases = [
        "zero_null_null",
        "zero_null_destination",
        "positive_null_source",
        "positive_null_destination_zero_source",
        "positive_null_destination_nonzero_source",
        "oversized_null_source",
        "negative_distinct_pointers",
        "minimum_size_null_alias",
    ];

    for case in cases {
        let c = run_boundary_child(&c_library_path(), case);
        let rust = run_boundary_child(&rust_library_path(), case);
        assert_eq!(
            exit_outcome(&rust),
            exit_outcome(&c),
            "boundary case {case}: C={c:?}, Rust={rust:?}"
        );
    }
}

#[test]
fn ffi_boundary_child() {
    let Some(case) = env::var_os("NORMALIZE_BOUNDARY_CASE") else {
        return;
    };
    let path = PathBuf::from(
        env::var_os("NORMALIZE_BOUNDARY_LIBRARY")
            .expect("NORMALIZE_BOUNDARY_LIBRARY must accompany NORMALIZE_BOUNDARY_CASE"),
    );
    let library = unsafe { Loaded::open(&path) };
    let normalize = library.normalize;
    let case = case.to_string_lossy();

    unsafe {
        match case.as_ref() {
            "zero_null_null" => normalize(std::ptr::null_mut(), std::ptr::null(), 0),
            "zero_null_destination" => {
                let source = 3.0_f32;
                normalize(std::ptr::null_mut(), &source, 0);
            }
            "positive_null_source" => {
                let mut destination = 7.0_f32;
                normalize(&mut destination, std::ptr::null(), 1);
            }
            "positive_null_destination_zero_source" => {
                let source = 0.0_f32;
                normalize(std::ptr::null_mut(), &source, 1);
            }
            "positive_null_destination_nonzero_source" => {
                let source = 1.0_f32;
                normalize(std::ptr::null_mut(), &source, 1);
            }
            "oversized_null_source" => {
                let mut destination = 0.0_f32;
                normalize(&mut destination, std::ptr::null(), c_int::MAX);
            }
            "negative_distinct_pointers" => {
                let source = 0.0_f32;
                let mut destination = 0.0_f32;
                normalize(&mut destination, &source, -1);
            }
            "minimum_size_null_alias" => {
                normalize(std::ptr::null_mut(), std::ptr::null(), c_int::MIN);
            }
            other => panic!("unknown boundary case {other}"),
        }
    }
}

fn run_boundary_child(library: &Path, case: &str) -> ExitStatus {
    Command::new(env::current_exe().expect("failed to locate test executable"))
        .arg("--exact")
        .arg("ffi_boundary_child")
        .arg("--nocapture")
        .env("NORMALIZE_BOUNDARY_LIBRARY", library)
        .env("NORMALIZE_BOUNDARY_CASE", case)
        .env("RUST_BACKTRACE", "0")
        .status()
        .unwrap_or_else(|error| panic!("failed to run boundary child for {case}: {error}"))
}

#[cfg(unix)]
fn exit_outcome(status: &ExitStatus) -> (Option<i32>, Option<i32>) {
    use std::os::unix::process::ExitStatusExt;
    (status.code(), status.signal())
}

#[cfg(not(unix))]
fn exit_outcome(status: &ExitStatus) -> (Option<i32>, Option<i32>) {
    (status.code(), None)
}
