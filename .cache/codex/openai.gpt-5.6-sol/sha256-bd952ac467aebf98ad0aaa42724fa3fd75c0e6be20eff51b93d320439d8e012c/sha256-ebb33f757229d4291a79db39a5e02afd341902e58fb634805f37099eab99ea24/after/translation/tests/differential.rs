use libloading::Library;
use std::ffi::{CString, c_char, c_int};
use std::path::{Path, PathBuf};
use std::process::Command;

type Memchra2 = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

struct Libraries {
    _c_library: Library,
    _rust_library: Library,
    c_memchra2: Memchra2,
    rust_memchra2: Memchra2,
}

impl Libraries {
    fn load() -> Self {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_path = manifest.join("../c_src/build/libharvest-work-6IJQh0.so");
        let rust_path = manifest.join("target/release/libmemchra2_lib.so");

        assert!(
            c_path.is_file(),
            "missing C shared library {}; build it before testing",
            c_path.display()
        );
        assert!(
            rust_path.is_file(),
            "missing Rust shared library {}; run cargo build --release before testing",
            rust_path.display()
        );

        unsafe {
            let c_library = Library::new(&c_path)
                .unwrap_or_else(|error| panic!("load {}: {error}", c_path.display()));
            let rust_library = Library::new(&rust_path)
                .unwrap_or_else(|error| panic!("load {}: {error}", rust_path.display()));
            let c_memchra2 = *c_library
                .get::<Memchra2>(b"memchra2\0")
                .expect("C memchra2 export");
            let rust_memchra2 = *rust_library
                .get::<Memchra2>(b"memchra2\0")
                .expect("Rust memchra2 export");

            Self {
                _c_library: c_library,
                _rust_library: rust_library,
                c_memchra2,
                rust_memchra2,
            }
        }
    }

    fn compare(&self, row: &str, case: usize, args: [i32; 4]) {
        let c_result = unsafe { (self.c_memchra2)(args[0], args[1], args[2], args[3]) };
        let rust_result = unsafe { (self.rust_memchra2)(args[0], args[1], args[2], args[3]) };
        assert_eq!(
            c_result, rust_result,
            "{row} case {case}: args={args:?}, C={c_result} ({:#010x}), Rust={rust_result} ({:#010x})",
            c_result as u32, rust_result as u32
        );
    }
}

#[derive(Clone, Copy)]
enum FloatClass {
    PositiveConditionFalse,
    PositiveBelowOne,
    PositiveOneToThousand,
    SignBitSet,
}

impl FloatClass {
    fn row_base(self) -> usize {
        match self {
            Self::PositiveConditionFalse => 1,
            Self::PositiveBelowOne => 9,
            Self::PositiveOneToThousand => 17,
            Self::SignBitSet => 25,
        }
    }

    fn a_value(self, random: u32, case: usize) -> i32 {
        match self {
            Self::PositiveConditionFalse => {
                const CORPUS: [u32; 8] = [
                    0x0000_0000, // +0.0
                    0x447a_0000, // 1000.0
                    0x447a_0001, // immediately above 1000.0
                    0x7f7f_ffff, // largest finite float
                    0x7f80_0000, // +infinity
                    0x7f80_0001, // signaling NaN payload
                    0x7fc0_0000, // quiet NaN
                    0x7fff_ffff, // NaN with maximum positive payload
                ];
                if case % 3 == 0 {
                    CORPUS[(case / 3) % CORPUS.len()] as i32
                } else {
                    let span = 0x8000_0000_u64 - 0x447a_0000_u64;
                    (0x447a_0000_u64 + u64::from(random) % span) as u32 as i32
                }
            }
            Self::PositiveBelowOne => {
                const CORPUS: [u32; 8] = [
                    0x0000_0001, // smallest subnormal
                    0x007f_ffff, // largest subnormal
                    0x0080_0000, // smallest normal
                    0x3e80_0000, // 0.25
                    0x3f00_0000, // 0.5
                    0x3f7f_fffe,
                    0x3f7f_ffff, // immediately below 1.0
                    0x0000_00ff,
                ];
                if case % 3 == 0 {
                    CORPUS[(case / 3) % CORPUS.len()] as i32
                } else {
                    (1 + random % 0x3f7f_ffff) as i32
                }
            }
            Self::PositiveOneToThousand => {
                const CORPUS: [u32; 8] = [
                    0x3f80_0000, // 1.0
                    0x3fc0_0000, // 1.5
                    0x4000_0000, // 2.0
                    0x42f6_0000, // 123.0
                    0x43fa_0000, // 500.0
                    0x4479_fffe,
                    0x4479_ffff, // immediately below 1000.0
                    0x4120_0000, // 10.0
                ];
                if case % 3 == 0 {
                    CORPUS[(case / 3) % CORPUS.len()] as i32
                } else {
                    (0x3f80_0000 + random % (0x447a_0000 - 0x3f80_0000)) as i32
                }
            }
            Self::SignBitSet => {
                const CORPUS: [u32; 8] = [
                    0x8000_0000, // -0.0 / INT_MIN
                    0x8000_0001, // negative subnormal
                    0xbf80_0000, // -1.0
                    0xc47a_0000, // -1000.0
                    0xff7f_ffff, // largest-magnitude finite negative float
                    0xff80_0000, // -infinity
                    0xffc0_0000, // negative quiet NaN
                    0xffff_ffff, // -1 as integer, negative NaN as float
                ];
                if case % 3 == 0 {
                    CORPUS[(case / 3) % CORPUS.len()] as i32
                } else {
                    (random | 0x8000_0000) as i32
                }
            }
        }
    }
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u32(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 ^ (self.0 >> 32)) as u32
    }
}

fn signed_value(rng: &mut Rng, negative: bool, case: usize) -> i32 {
    const NONNEGATIVE: [i32; 16] = [
        0,
        1,
        9,
        10,
        99,
        100,
        255,
        256,
        999,
        1000,
        32_767,
        65_535,
        999_999,
        1_000_000,
        1_000_000_000,
        i32::MAX,
    ];
    const NEGATIVE: [i32; 16] = [
        -1,
        -9,
        -10,
        -99,
        -100,
        -255,
        -256,
        -999,
        -1000,
        -32_768,
        -65_535,
        -65_536,
        -999_999,
        -1_000_000,
        -1_000_000_000,
        i32::MIN,
    ];

    if case % 3 == 0 {
        if negative {
            NEGATIVE[(case / 3) % NEGATIVE.len()]
        } else {
            NONNEGATIVE[(case / 3) % NONNEGATIVE.len()]
        }
    } else if negative {
        (rng.next_u32() | 0x8000_0000) as i32
    } else {
        (rng.next_u32() & 0x7fff_ffff) as i32
    }
}

#[test]
fn phase_b_all_configuration_rows_match() {
    let libraries = Libraries::load();
    let classes = [
        FloatClass::PositiveConditionFalse,
        FloatClass::PositiveBelowOne,
        FloatClass::PositiveOneToThousand,
        FloatClass::SignBitSet,
    ];
    let mut rng = Rng::new(0x6d65_6d63_6872_6132);

    for class in classes {
        for sign_mask in 0..8_usize {
            let row = format!("C{:02}", class.row_base() + sign_mask);
            for case in 0..256 {
                let a = class.a_value(rng.next_u32(), case);
                let b = signed_value(&mut rng, sign_mask & 1 != 0, case);
                let c = signed_value(&mut rng, sign_mask & 2 != 0, case + 5);
                let d = signed_value(&mut rng, sign_mask & 4 != 0, case + 11);
                libraries.compare(&row, case, [a, b, c, d]);
            }
        }
    }

    let exact_a_boundaries = [
        ("C33", 0x0000_0000_u32 as i32),
        ("C34", 0x0000_0001_u32 as i32),
        ("C35", 0x3f7f_ffff_u32 as i32),
        ("C36", 0x3f80_0000_u32 as i32),
        ("C37", 0x4479_ffff_u32 as i32),
        ("C38", 0x447a_0000_u32 as i32),
    ];
    for (row, a) in exact_a_boundaries {
        for case in 0..256 {
            let b = rng.next_u32() as i32;
            let c = rng.next_u32() as i32;
            let d = rng.next_u32() as i32;
            libraries.compare(row, case, [a, b, c, d]);
        }
    }

    let extrema = [i32::MIN, i32::MAX];
    let mut case = 0;
    for &a in &extrema {
        for &b in &extrema {
            for &c in &extrema {
                for &d in &extrema {
                    libraries.compare("C39", case, [a, b, c, d]);
                    case += 1;
                }
            }
        }
    }
    for randomized_case in 0..256 {
        let args = std::array::from_fn(|_| {
            if rng.next_u32() & 1 == 0 {
                i32::MIN
            } else {
                i32::MAX
            }
        });
        libraries.compare("C39", case + randomized_case, args);
    }
}

#[test]
fn phase_c_public_boundaries_and_private_error_surface_match() {
    let libraries = Libraries::load();

    let boundary_cases = [
        [0, 0, 0, 0],
        [i32::MIN, 0, 0, 0],
        [i32::MAX, 0, 0, 0],
        [0, i32::MIN, i32::MAX, -1],
        [i32::MIN, i32::MIN, i32::MIN, i32::MIN],
        [i32::MAX, i32::MAX, i32::MAX, i32::MAX],
    ];
    for (case, args) in boundary_cases.into_iter().enumerate() {
        libraries.compare("Phase C public boundary", case, args);
    }

    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_path = manifest.join("../c_src/build/libharvest-work-6IJQh0.so");
    let rust_path = manifest.join("target/release/libmemchra2_lib.so");
    assert_private_helpers_not_exported(&c_path);
    assert_private_helpers_not_exported(&rust_path);
}

#[test]
fn phase_c_every_private_rejection_row_matches_exactly() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let output_directory = manifest.join("target/release");
    let c_adapter = output_directory.join("libverification_c_adapter.so");
    let rust_adapter = output_directory.join("libverification_rust_adapter.so");

    let c_status = Command::new("cc")
        .current_dir(&manifest)
        .args(["-shared", "-fPIC", "-O0"])
        .arg("tests/support/c_adapter.c")
        .arg("-o")
        .arg(&c_adapter)
        .status()
        .expect("run C compiler for private-helper adapter");
    assert!(c_status.success(), "C private-helper adapter build failed");

    let rust_status = Command::new("rustc")
        .current_dir(&manifest)
        .args(["--edition=2024", "--crate-type=cdylib", "-C", "opt-level=3"])
        .arg("tests/support/rust_adapter.rs")
        .arg("-o")
        .arg(&rust_adapter)
        .status()
        .expect("run rustc for private-helper adapter");
    assert!(
        rust_status.success(),
        "Rust private-helper adapter build failed"
    );

    unsafe {
        let c = Library::new(&c_adapter).expect("load C private-helper adapter");
        let rust = Library::new(&rust_adapter).expect("load Rust private-helper adapter");

        type ProcessBuffer = unsafe extern "C" fn(*mut c_char, usize) -> c_int;
        type ProcessStrings = unsafe extern "C" fn(*mut *mut c_char, c_int, *const c_char) -> c_int;
        type SafeSumArray = unsafe extern "C" fn(*mut c_int, usize) -> c_int;
        type InterpretAsInt = unsafe extern "C" fn(*mut u8, usize) -> c_int;
        type CountOccurrences = unsafe extern "C" fn(*const c_char, c_char) -> c_int;
        type ComplexIteration = unsafe extern "C" fn(*mut c_int, usize) -> c_int;

        let c_process_buffer = *c
            .get::<ProcessBuffer>(b"verification_process_buffer\0")
            .unwrap();
        let r_process_buffer = *rust
            .get::<ProcessBuffer>(b"verification_process_buffer\0")
            .unwrap();
        let c_process_strings = *c
            .get::<ProcessStrings>(b"verification_process_strings\0")
            .unwrap();
        let r_process_strings = *rust
            .get::<ProcessStrings>(b"verification_process_strings\0")
            .unwrap();
        let c_safe_sum = *c
            .get::<SafeSumArray>(b"verification_safe_sum_array\0")
            .unwrap();
        let r_safe_sum = *rust
            .get::<SafeSumArray>(b"verification_safe_sum_array\0")
            .unwrap();
        let c_interpret = *c
            .get::<InterpretAsInt>(b"verification_interpret_as_int\0")
            .unwrap();
        let r_interpret = *rust
            .get::<InterpretAsInt>(b"verification_interpret_as_int\0")
            .unwrap();
        let c_count = *c
            .get::<CountOccurrences>(b"verification_count_occurrences\0")
            .unwrap();
        let r_count = *rust
            .get::<CountOccurrences>(b"verification_count_occurrences\0")
            .unwrap();
        let c_complex = *c
            .get::<ComplexIteration>(b"verification_complex_iteration\0")
            .unwrap();
        let r_complex = *rust
            .get::<ComplexIteration>(b"verification_complex_iteration\0")
            .unwrap();

        assert_same(
            "E01",
            c_process_buffer(std::ptr::null_mut(), 4),
            r_process_buffer(std::ptr::null_mut(), 4),
        );
        let mut empty = [0_i8];
        assert_same(
            "E02",
            c_process_buffer(empty.as_mut_ptr(), 1),
            r_process_buffer(empty.as_mut_ptr(), 1),
        );

        let target = CString::new("test").unwrap();
        assert_same(
            "E03",
            c_process_strings(std::ptr::null_mut(), 1, target.as_ptr()),
            r_process_strings(std::ptr::null_mut(), 1, target.as_ptr()),
        );
        let mut placeholder = [std::ptr::null_mut()];
        assert_same(
            "E04-zero",
            c_process_strings(placeholder.as_mut_ptr(), 0, target.as_ptr()),
            r_process_strings(placeholder.as_mut_ptr(), 0, target.as_ptr()),
        );
        assert_same(
            "E04-negative",
            c_process_strings(placeholder.as_mut_ptr(), -1, target.as_ptr()),
            r_process_strings(placeholder.as_mut_ptr(), -1, target.as_ptr()),
        );

        let valid = CString::new("test-value").unwrap();
        let mut with_null = [std::ptr::null_mut(), valid.as_ptr().cast_mut()];
        assert_same(
            "E05",
            c_process_strings(with_null.as_mut_ptr(), 2, target.as_ptr()),
            r_process_strings(with_null.as_mut_ptr(), 2, target.as_ptr()),
        );
        let empty_string = CString::new("").unwrap();
        let mut with_empty = [empty_string.as_ptr().cast_mut(), valid.as_ptr().cast_mut()];
        assert_same(
            "E06",
            c_process_strings(with_empty.as_mut_ptr(), 2, target.as_ptr()),
            r_process_strings(with_empty.as_mut_ptr(), 2, target.as_ptr()),
        );

        assert_same(
            "E07",
            c_safe_sum(std::ptr::null_mut(), 4),
            r_safe_sum(std::ptr::null_mut(), 4),
        );
        let mut integers = [1, 2, 3, 4];
        assert_same(
            "E08",
            c_safe_sum(integers.as_mut_ptr(), 0),
            r_safe_sum(integers.as_mut_ptr(), 0),
        );

        assert_same(
            "E09",
            c_interpret(std::ptr::null_mut(), 4),
            r_interpret(std::ptr::null_mut(), 4),
        );
        let mut short_bytes = [1_u8, 2, 3];
        assert_same(
            "E10",
            c_interpret(short_bytes.as_mut_ptr(), 3),
            r_interpret(short_bytes.as_mut_ptr(), 3),
        );

        assert_same(
            "E11",
            c_count(std::ptr::null(), b'x' as c_char),
            r_count(std::ptr::null(), b'x' as c_char),
        );
        assert_same(
            "E12",
            c_count(empty_string.as_ptr(), b'x' as c_char),
            r_count(empty_string.as_ptr(), b'x' as c_char),
        );

        assert_same(
            "E13",
            c_complex(std::ptr::null_mut(), 4),
            r_complex(std::ptr::null_mut(), 4),
        );
        assert_same(
            "E14",
            c_complex(integers.as_mut_ptr(), 0),
            r_complex(integers.as_mut_ptr(), 0),
        );
    }
}

fn assert_same(row: &str, c_result: c_int, rust_result: c_int) {
    assert_eq!(
        c_result, rust_result,
        "{row}: C={c_result}, Rust={rust_result}"
    );
}

fn assert_private_helpers_not_exported(path: &Path) {
    const PRIVATE_HELPERS: [&[u8]; 7] = [
        b"process_buffer\0",
        b"process_strings\0",
        b"safe_sum_array\0",
        b"interpret_as_int\0",
        b"count_occurrences\0",
        b"complex_iteration\0",
        b"memchra\0",
    ];

    unsafe {
        let library =
            Library::new(path).unwrap_or_else(|error| panic!("load {}: {error}", path.display()));
        for &name in &PRIVATE_HELPERS {
            assert!(
                library.get::<*const ()>(name).is_err(),
                "{} unexpectedly exports {}",
                path.display(),
                String::from_utf8_lossy(&name[..name.len() - 1])
            );
        }
    }
}
