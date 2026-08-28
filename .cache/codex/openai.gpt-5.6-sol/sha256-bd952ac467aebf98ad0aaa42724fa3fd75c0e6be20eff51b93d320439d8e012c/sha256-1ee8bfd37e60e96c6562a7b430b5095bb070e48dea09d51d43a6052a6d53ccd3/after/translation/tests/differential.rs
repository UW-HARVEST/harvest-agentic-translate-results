use libloading::Library;
use std::ffi::{CString, c_char, c_int};
use std::path::{Path, PathBuf};
use std::process::Command;

type Memchra2 = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

struct LoadedLibraries {
    _c_library: Library,
    _rust_library: Library,
    c_memchra2: Memchra2,
    rust_memchra2: Memchra2,
}

impl LoadedLibraries {
    fn load() -> Self {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let c_path = find_c_library(&manifest_dir.join("../c_src/build"));
        let rust_path = manifest_dir.join("target/release/libmemchra2_lib.so");

        assert!(
            rust_path.is_file(),
            "missing {}; run `cargo build --release` first",
            rust_path.display()
        );

        // The function pointers remain valid because both libraries are retained
        // in this structure for at least as long as the pointers.
        unsafe {
            let c_library = Library::new(&c_path)
                .unwrap_or_else(|error| panic!("failed to load {}: {error}", c_path.display()));
            let rust_library = Library::new(&rust_path)
                .unwrap_or_else(|error| panic!("failed to load {}: {error}", rust_path.display()));
            let c_memchra2 = *c_library
                .get::<Memchra2>(b"memchra2\0")
                .unwrap_or_else(|error| {
                    panic!("failed to load memchra2 from {}: {error}", c_path.display())
                });
            let rust_memchra2 =
                *rust_library
                    .get::<Memchra2>(b"memchra2\0")
                    .unwrap_or_else(|error| {
                        panic!(
                            "failed to load memchra2 from {}: {error}",
                            rust_path.display()
                        )
                    });

            Self {
                _c_library: c_library,
                _rust_library: rust_library,
                c_memchra2,
                rust_memchra2,
            }
        }
    }

    fn assert_match(&self, row: usize, input: [c_int; 4]) {
        let [a, b, c, d] = input;
        unsafe {
            let c_result = (self.c_memchra2)(a, b, c, d);
            let rust_result = (self.rust_memchra2)(a, b, c, d);
            assert_eq!(
                c_result.to_ne_bytes(),
                rust_result.to_ne_bytes(),
                "CONFIGS.md row {row}, input {input:?}: C={c_result}, Rust={rust_result}"
            );
        }
    }
}

fn find_c_library(build_dir: &Path) -> PathBuf {
    let mut libraries: Vec<_> = std::fs::read_dir(build_dir)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", build_dir.display()))
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_file()
                && path.extension().is_some_and(|extension| extension == "so")
                && path
                    .file_name()
                    .is_some_and(|name| name.to_string_lossy().starts_with("lib"))
        })
        .collect();
    libraries.sort();
    assert_eq!(
        libraries.len(),
        1,
        "expected exactly one C shared library in {}, found {libraries:?}",
        build_dir.display()
    );
    libraries.remove(0)
}

#[derive(Clone, Copy)]
enum AClass {
    NegativeSign,
    PositiveZero,
    LessThanOne,
    OneToThousand,
    ThousandToInfinity,
    PositiveNan,
}

impl AClass {
    const ALL: [Self; 6] = [
        Self::NegativeSign,
        Self::PositiveZero,
        Self::LessThanOne,
        Self::OneToThousand,
        Self::ThousandToInfinity,
        Self::PositiveNan,
    ];

    fn random_value(self, random: &mut XorShift64) -> c_int {
        match self {
            Self::NegativeSign => (random.next_u32() | 0x8000_0000) as c_int,
            Self::PositiveZero => 0,
            Self::LessThanOne => random.inclusive(0x0000_0001, 0x3f7f_ffff) as c_int,
            Self::OneToThousand => random.inclusive(0x3f80_0000, 0x4479_ffff) as c_int,
            Self::ThousandToInfinity => random.inclusive(0x447a_0000, 0x7f80_0000) as c_int,
            Self::PositiveNan => random.inclusive(0x7f80_0001, 0x7fff_ffff) as c_int,
        }
    }

    fn boundary_values(self) -> &'static [c_int] {
        match self {
            Self::NegativeSign => &[c_int::MIN, -1, 0x8000_0001_u32 as c_int, -1_082_130_432],
            Self::PositiveZero => &[0],
            Self::LessThanOne => &[1, 0x007f_ffff, 0x0080_0000, 0x3f7f_ffff],
            Self::OneToThousand => &[0x3f80_0000, 0x3f80_0001, 0x4479_ffff],
            Self::ThousandToInfinity => &[0x447a_0000, 0x447a_0001, 0x7f7f_ffff, 0x7f80_0000],
            Self::PositiveNan => &[0x7f80_0001, 0x7fc0_0000, c_int::MAX],
        }
    }
}

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

    fn inclusive(&mut self, low: u32, high: u32) -> u32 {
        let width = u64::from(high) - u64::from(low) + 1;
        (u64::from(low) + u64::from(self.next_u32()) % width) as u32
    }
}

fn value_with_sign(random: &mut XorShift64, negative: bool) -> c_int {
    let bits = random.next_u32();
    if negative {
        (bits | 0x8000_0000) as c_int
    } else {
        (bits & 0x7fff_ffff) as c_int
    }
}

#[test]
fn every_configuration_matches_for_seeded_random_inputs() {
    let libraries = LoadedLibraries::load();
    // This order is the row order in CONFIGS.md. Bits 0, 1, and 2 select
    // negative b, c, and d respectively.
    let sign_masks = [0b000, 0b001, 0b010, 0b100, 0b011, 0b101, 0b110, 0b111];

    for (class_index, a_class) in AClass::ALL.into_iter().enumerate() {
        for (mask_index, sign_mask) in sign_masks.into_iter().enumerate() {
            let row = class_index * sign_masks.len() + mask_index + 1;
            let mut random =
                XorShift64::new(0x6d65_6d63_6872_6132_u64 ^ (row as u64 * 0x9e37_79b9));

            for sample in 0..128 {
                let input = [
                    a_class.random_value(&mut random),
                    value_with_sign(&mut random, sign_mask & 0b001 != 0),
                    value_with_sign(&mut random, sign_mask & 0b010 != 0),
                    value_with_sign(&mut random, sign_mask & 0b100 != 0),
                ];
                libraries.assert_match(row, input);

                if sample < a_class.boundary_values().len() {
                    let nonnegative = [0, 1, 127, 128, 255, 256, c_int::MAX];
                    let negative = [c_int::MIN, -1, -127, -128, -255, -256, -65_536];
                    let pick = |is_negative: bool, offset: usize| {
                        let values = if is_negative {
                            &negative[..]
                        } else {
                            &nonnegative[..]
                        };
                        values[(sample + offset) % values.len()]
                    };
                    libraries.assert_match(
                        row,
                        [
                            a_class.boundary_values()[sample],
                            pick(sign_mask & 0b001 != 0, 0),
                            pick(sign_mask & 0b010 != 0, 1),
                            pick(sign_mask & 0b100 != 0, 2),
                        ],
                    );
                }
            }
        }
    }
}

#[test]
fn public_ffi_integer_boundaries_match() {
    let libraries = LoadedLibraries::load();
    let boundaries = [c_int::MIN, -1, 0, 1, c_int::MAX];

    for &a in &boundaries {
        for &b in &boundaries {
            for &c in &boundaries {
                for &d in &boundaries {
                    libraries.assert_match(0, [a, b, c, d]);
                }
            }
        }
    }
}

type ProcessBuffer = unsafe extern "C" fn(*mut c_char, usize) -> c_int;
type ProcessStrings = unsafe extern "C" fn(*mut *mut c_char, c_int, *const c_char) -> c_int;
type SafeSumArray = unsafe extern "C" fn(*mut c_int, usize) -> c_int;
type InterpretAsInt = unsafe extern "C" fn(*mut u8, usize) -> c_int;
type CountOccurrences = unsafe extern "C" fn(*const c_char, c_char) -> c_int;
type ComplexIteration = unsafe extern "C" fn(*mut c_int, usize) -> c_int;

struct InternalFunctions {
    _library: Library,
    process_buffer: ProcessBuffer,
    process_strings: ProcessStrings,
    safe_sum_array: SafeSumArray,
    interpret_as_int: InterpretAsInt,
    count_occurrences: CountOccurrences,
    complex_iteration: ComplexIteration,
}

impl InternalFunctions {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));

        macro_rules! load {
            ($name:literal, $kind:ty) => {
                *unsafe { library.get::<$kind>(concat!($name, "\0").as_bytes()) }.unwrap_or_else(
                    |error| panic!("failed to load {} from {}: {error}", $name, path.display()),
                )
            };
        }

        let process_buffer = load!("verify_process_buffer", ProcessBuffer);
        let process_strings = load!("verify_process_strings", ProcessStrings);
        let safe_sum_array = load!("verify_safe_sum_array", SafeSumArray);
        let interpret_as_int = load!("verify_interpret_as_int", InterpretAsInt);
        let count_occurrences = load!("verify_count_occurrences", CountOccurrences);
        let complex_iteration = load!("verify_complex_iteration", ComplexIteration);

        Self {
            _library: library,
            process_buffer,
            process_strings,
            safe_sum_array,
            interpret_as_int,
            count_occurrences,
            complex_iteration,
        }
    }
}

fn build_internal_shims() -> (PathBuf, PathBuf) {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let output_dir = manifest_dir.join("target/differential-shims");
    std::fs::create_dir_all(&output_dir)
        .unwrap_or_else(|error| panic!("failed to create {}: {error}", output_dir.display()));
    let c_output = output_dir.join("libc_internal_shim.so");
    let rust_output = output_dir.join("librust_internal_shim.so");

    let c_status = Command::new("cc")
        .current_dir(manifest_dir)
        .args([
            "-shared",
            "-fPIC",
            "tests/support/c_internal_shim.c",
            "-o",
            c_output.to_str().unwrap(),
        ])
        .status()
        .expect("failed to execute cc for the internal C shim");
    assert!(c_status.success(), "internal C shim compilation failed");

    let rust_status = Command::new("rustc")
        .current_dir(manifest_dir)
        .args([
            "--edition=2024",
            "--crate-type=cdylib",
            "-C",
            "opt-level=3",
            "tests/support/rust_internal_shim.rs",
            "-o",
            rust_output.to_str().unwrap(),
        ])
        .status()
        .expect("failed to execute rustc for the internal Rust shim");
    assert!(
        rust_status.success(),
        "internal Rust shim compilation failed"
    );

    (c_output, rust_output)
}

#[test]
fn every_internal_rejection_guard_matches() {
    let (c_path, rust_path) = build_internal_shims();
    let c = unsafe { InternalFunctions::load(&c_path) };
    let rust = unsafe { InternalFunctions::load(&rust_path) };

    macro_rules! assert_guard {
        ($row:literal, $c_call:expr, $rust_call:expr) => {{
            let c_result = $c_call;
            let rust_result = $rust_call;
            assert_eq!(
                c_result, rust_result,
                "ERRORS.md row {}: C={}, Rust={}",
                $row, c_result, rust_result
            );
        }};
    }

    unsafe {
        assert_guard!(
            1,
            (c.process_buffer)(std::ptr::null_mut(), 1),
            (rust.process_buffer)(std::ptr::null_mut(), 1)
        );

        let mut empty = [0_i8];
        assert_guard!(
            2,
            (c.process_buffer)(empty.as_mut_ptr(), empty.len()),
            (rust.process_buffer)(empty.as_mut_ptr(), empty.len())
        );

        let target = CString::new("test").unwrap();
        assert_guard!(
            3,
            (c.process_strings)(std::ptr::null_mut(), 1, target.as_ptr()),
            (rust.process_strings)(std::ptr::null_mut(), 1, target.as_ptr())
        );

        let mut one_string = [target.as_ptr().cast_mut()];
        for invalid_count in [0, -1, c_int::MIN] {
            assert_guard!(
                4,
                (c.process_strings)(one_string.as_mut_ptr(), invalid_count, target.as_ptr()),
                (rust.process_strings)(one_string.as_mut_ptr(), invalid_count, target.as_ptr())
            );
        }

        let matching = CString::new("testing").unwrap();
        let mut null_element = [std::ptr::null_mut(), matching.as_ptr().cast_mut()];
        assert_guard!(
            5,
            (c.process_strings)(null_element.as_mut_ptr(), 2, target.as_ptr()),
            (rust.process_strings)(null_element.as_mut_ptr(), 2, target.as_ptr())
        );

        let mut empty_element = [empty.as_mut_ptr(), matching.as_ptr().cast_mut()];
        assert_guard!(
            6,
            (c.process_strings)(empty_element.as_mut_ptr(), 2, target.as_ptr()),
            (rust.process_strings)(empty_element.as_mut_ptr(), 2, target.as_ptr())
        );

        assert_guard!(
            7,
            (c.safe_sum_array)(std::ptr::null_mut(), 1),
            (rust.safe_sum_array)(std::ptr::null_mut(), 1)
        );

        let mut values = [17, -4];
        assert_guard!(
            8,
            (c.safe_sum_array)(values.as_mut_ptr(), 0),
            (rust.safe_sum_array)(values.as_mut_ptr(), 0)
        );

        assert_guard!(
            9,
            (c.interpret_as_int)(std::ptr::null_mut(), size_of::<c_int>()),
            (rust.interpret_as_int)(std::ptr::null_mut(), size_of::<c_int>())
        );

        let mut bytes = [0x12_u8, 0x34, 0x56, 0x78];
        for short_len in 0..size_of::<c_int>() {
            assert_guard!(
                10,
                (c.interpret_as_int)(bytes.as_mut_ptr(), short_len),
                (rust.interpret_as_int)(bytes.as_mut_ptr(), short_len)
            );
        }

        assert_guard!(
            11,
            (c.count_occurrences)(std::ptr::null(), b'x' as c_char),
            (rust.count_occurrences)(std::ptr::null(), b'x' as c_char)
        );
        assert_guard!(
            12,
            (c.count_occurrences)(empty.as_ptr(), b'x' as c_char),
            (rust.count_occurrences)(empty.as_ptr(), b'x' as c_char)
        );

        assert_guard!(
            13,
            (c.complex_iteration)(std::ptr::null_mut(), 1),
            (rust.complex_iteration)(std::ptr::null_mut(), 1)
        );
        assert_guard!(
            14,
            (c.complex_iteration)(values.as_mut_ptr(), 0),
            (rust.complex_iteration)(values.as_mut_ptr(), 0)
        );
    }
}
