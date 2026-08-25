use libloading::{Library, Symbol};
use std::ffi::{CString, c_char, c_int, c_uchar};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr;
use std::sync::OnceLock;

type Memchra2 = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

static ORACLES: OnceLock<(PathBuf, PathBuf)> = OnceLock::new();
static RUST_LIBRARY: OnceLock<PathBuf> = OnceLock::new();

fn crate_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn rust_library() -> &'static Path {
    RUST_LIBRARY
        .get_or_init(|| {
            let output_dir = crate_root().join("target/differential");
            fs::create_dir_all(&output_dir).expect("create differential output directory");
            let output = output_dir.join("libmemchra2_lib.so");
            let status = Command::new("rustc")
                .args([
                    "--crate-name",
                    "memchra2_lib",
                    "--edition=2024",
                    "--crate-type=cdylib",
                    "-C",
                    "panic=abort",
                    "-O",
                ])
                .arg(crate_root().join("src/lib.rs"))
                .arg("-o")
                .arg(&output)
                .status()
                .expect("run rustc");
            assert!(status.success(), "failed to compile Rust shared object");
            output
        })
        .as_path()
}

fn compile_shared(output: &Path, source: &Path) {
    let status = Command::new("cc")
        .args(["-shared", "-fPIC", "-O2"])
        .arg(source)
        .arg("-o")
        .arg(output)
        .status()
        .expect("run C compiler");
    assert!(status.success(), "failed to compile {}", source.display());
}

fn c_libraries() -> &'static (PathBuf, PathBuf) {
    ORACLES.get_or_init(|| {
        let output_dir = crate_root().join("target/differential");
        fs::create_dir_all(&output_dir).expect("create differential output directory");

        let public = output_dir.join("libtranslated_rust.so");
        let internal = output_dir.join("libtranslated_rust_internal.so");
        compile_shared(&public, &crate_root().join("c_src/src/lib.c"));
        compile_shared(&internal, &crate_root().join("tests/c_internal_oracle.c"));
        (public, internal)
    })
}

#[derive(Clone, Copy)]
enum FloatClass {
    PositiveZero,
    BelowOne,
    OneToThousand,
    ThousandOrMore,
    PositiveInfinity,
    PositiveNan,
    SignBitSet,
}

impl FloatClass {
    const ALL: [Self; 7] = [
        Self::PositiveZero,
        Self::BelowOne,
        Self::OneToThousand,
        Self::ThousandOrMore,
        Self::PositiveInfinity,
        Self::PositiveNan,
        Self::SignBitSet,
    ];

    fn value(self, rng: &mut XorShift64, sample: usize) -> c_int {
        let bits = match self {
            Self::PositiveZero => 0,
            Self::BelowOne => ranged_bits(rng, sample, 0x0000_0001, 0x3f7f_ffff),
            Self::OneToThousand => ranged_bits(rng, sample, 0x3f80_0000, 0x4479_ffff),
            Self::ThousandOrMore => ranged_bits(rng, sample, 0x447a_0000, 0x7f7f_ffff),
            Self::PositiveInfinity => 0x7f80_0000,
            Self::PositiveNan => ranged_bits(rng, sample, 0x7f80_0001, 0x7fff_ffff),
            Self::SignBitSet => match sample {
                0 => 0x8000_0000,
                1 => 0xffff_ffff,
                _ => rng.next_u32() | 0x8000_0000,
            },
        };
        bits as c_int
    }
}

fn ranged_bits(rng: &mut XorShift64, sample: usize, low: u32, high: u32) -> u32 {
    match sample {
        0 => low,
        1 => high,
        _ => low + rng.next_u32() % (high - low + 1),
    }
}

struct XorShift64(u64);

impl XorShift64 {
    fn next_u32(&mut self) -> u32 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value as u32
    }
}

fn shaped_int(rng: &mut XorShift64, negative: bool, sample: usize) -> c_int {
    const POSITIVE: [c_int; 20] = [
        0,
        1,
        9,
        10,
        99,
        100,
        127,
        128,
        255,
        256,
        999,
        1000,
        9_999,
        10_000,
        0x7f00,
        0x7fff,
        0x7f80,
        0x7fff_ff00,
        0x7fff_ffff,
        0x00ff_ffff,
    ];
    const NEGATIVE: [c_int; 20] = [
        -1,
        -2,
        -9,
        -10,
        -99,
        -100,
        -127,
        -128,
        -129,
        -255,
        -256,
        -999,
        -1000,
        -9_999,
        -10_000,
        -0x7f00,
        -0x7fff,
        -0x7f80,
        c_int::MIN,
        -0x00ff_ffff,
    ];

    if sample < POSITIVE.len() {
        return if negative {
            NEGATIVE[sample]
        } else {
            POSITIVE[sample]
        };
    }

    let bits = rng.next_u32();
    if negative {
        (bits | 0x8000_0000) as c_int
    } else {
        (bits & 0x7fff_ffff) as c_int
    }
}

#[test]
fn all_configuration_rows_match_byte_for_byte() {
    let (c_path, _) = c_libraries();
    let rust_path = rust_library();
    assert!(rust_path.is_file(), "missing {}", rust_path.display());

    unsafe {
        let c_library = Library::new(c_path).expect("load C shared object");
        let rust_library = Library::new(rust_path).expect("load Rust shared object");
        let c_fn: Symbol<Memchra2> = c_library.get(b"memchra2").expect("C memchra2");
        let rust_fn: Symbol<Memchra2> = rust_library.get(b"memchra2").expect("Rust memchra2");

        let mut rng = XorShift64(0xd1ff_e2e5_9a17_c3b7);
        for (class_index, class) in FloatClass::ALL.iter().copied().enumerate() {
            for sign_mask in 0_u8..8 {
                let row = class_index * 8 + sign_mask as usize + 1;
                for sample in 0..256 {
                    let a = class.value(&mut rng, sample);
                    let b = shaped_int(&mut rng, sign_mask & 0b100 != 0, sample);
                    let c = shaped_int(&mut rng, sign_mask & 0b010 != 0, sample + 3);
                    let d = shaped_int(&mut rng, sign_mask & 0b001 != 0, sample + 7);
                    let expected = c_fn(a, b, c, d);
                    let actual = rust_fn(a, b, c, d);
                    assert_eq!(
                        actual.to_ne_bytes(),
                        expected.to_ne_bytes(),
                        "CONFIGS.md row {row}, sample {sample}: ({a}, {b}, {c}, {d})"
                    );
                }
            }
        }
    }
}

#[test]
fn every_error_surface_row_matches() {
    let (_, c_path) = c_libraries();
    let rust_path = rust_library();

    unsafe {
        let c_library = Library::new(c_path).expect("load internal C shared object");
        let rust_library = Library::new(rust_path).expect("load Rust shared object");

        type ProcessBuffer = unsafe extern "C" fn(*mut c_char, usize) -> c_int;
        let c_process_buffer: Symbol<ProcessBuffer> =
            c_library.get(b"process_buffer").expect("C process_buffer");
        let r_process_buffer: Symbol<ProcessBuffer> = rust_library
            .get(b"process_buffer")
            .expect("Rust process_buffer");
        assert_same(
            1,
            c_process_buffer(ptr::null_mut(), 1),
            r_process_buffer(ptr::null_mut(), 1),
        );
        let mut empty = [0 as c_char];
        assert_same(
            2,
            c_process_buffer(empty.as_mut_ptr(), 1),
            r_process_buffer(empty.as_mut_ptr(), 1),
        );

        type ProcessStrings = unsafe extern "C" fn(*mut *mut c_char, c_int, *const c_char) -> c_int;
        let c_process_strings: Symbol<ProcessStrings> = c_library
            .get(b"process_strings")
            .expect("C process_strings");
        let r_process_strings: Symbol<ProcessStrings> = rust_library
            .get(b"process_strings")
            .expect("Rust process_strings");
        let target = CString::new("test").unwrap();
        assert_same(
            3,
            c_process_strings(ptr::null_mut(), 1, target.as_ptr()),
            r_process_strings(ptr::null_mut(), 1, target.as_ptr()),
        );
        let mut placeholder = ptr::null_mut();
        assert_same(
            4,
            c_process_strings(&mut placeholder, -1, target.as_ptr()),
            r_process_strings(&mut placeholder, -1, target.as_ptr()),
        );
        let mut valid = CString::new("testing").unwrap().into_bytes_with_nul();
        let mut items = [ptr::null_mut(), valid.as_mut_ptr().cast()];
        assert_same(
            5,
            c_process_strings(items.as_mut_ptr(), 2, target.as_ptr()),
            r_process_strings(items.as_mut_ptr(), 2, target.as_ptr()),
        );
        let mut empty_string = [0 as c_char];
        items[0] = empty_string.as_mut_ptr();
        assert_same(
            6,
            c_process_strings(items.as_mut_ptr(), 2, target.as_ptr()),
            r_process_strings(items.as_mut_ptr(), 2, target.as_ptr()),
        );

        type SafeSum = unsafe extern "C" fn(*mut c_int, usize) -> c_int;
        let c_safe_sum: Symbol<SafeSum> =
            c_library.get(b"safe_sum_array").expect("C safe_sum_array");
        let r_safe_sum: Symbol<SafeSum> = rust_library
            .get(b"safe_sum_array")
            .expect("Rust safe_sum_array");
        assert_same(
            7,
            c_safe_sum(ptr::null_mut(), 1),
            r_safe_sum(ptr::null_mut(), 1),
        );
        let mut value = 42;
        assert_same(8, c_safe_sum(&mut value, 0), r_safe_sum(&mut value, 0));

        type Interpret = unsafe extern "C" fn(*mut c_uchar, usize) -> c_int;
        let c_interpret: Symbol<Interpret> = c_library
            .get(b"interpret_as_int")
            .expect("C interpret_as_int");
        let r_interpret: Symbol<Interpret> = rust_library
            .get(b"interpret_as_int")
            .expect("Rust interpret_as_int");
        assert_same(
            9,
            c_interpret(ptr::null_mut(), 4),
            r_interpret(ptr::null_mut(), 4),
        );
        let mut bytes = [0x12_u8, 0x34, 0x56, 0x78];
        assert_same(
            10,
            c_interpret(bytes.as_mut_ptr(), 3),
            r_interpret(bytes.as_mut_ptr(), 3),
        );

        type CountOccurrences = unsafe extern "C" fn(*const c_char, c_char) -> c_int;
        let c_count: Symbol<CountOccurrences> = c_library
            .get(b"count_occurrences")
            .expect("C count_occurrences");
        let r_count: Symbol<CountOccurrences> = rust_library
            .get(b"count_occurrences")
            .expect("Rust count_occurrences");
        assert_same(
            11,
            c_count(ptr::null(), b'x' as c_char),
            r_count(ptr::null(), b'x' as c_char),
        );
        assert_same(
            12,
            c_count(empty.as_ptr(), b'x' as c_char),
            r_count(empty.as_ptr(), b'x' as c_char),
        );

        type Complex = unsafe extern "C" fn(*mut c_int, usize) -> c_int;
        let c_complex: Symbol<Complex> = c_library
            .get(b"complex_iteration")
            .expect("C complex_iteration");
        let r_complex: Symbol<Complex> = rust_library
            .get(b"complex_iteration")
            .expect("Rust complex_iteration");
        assert_same(
            13,
            c_complex(ptr::null_mut(), 1),
            r_complex(ptr::null_mut(), 1),
        );
        assert_same(14, c_complex(&mut value, 0), r_complex(&mut value, 0));
    }
}

fn assert_same(row: usize, expected: c_int, actual: c_int) {
    assert_eq!(
        actual.to_ne_bytes(),
        expected.to_ne_bytes(),
        "ERRORS.md row {row}"
    );
}
