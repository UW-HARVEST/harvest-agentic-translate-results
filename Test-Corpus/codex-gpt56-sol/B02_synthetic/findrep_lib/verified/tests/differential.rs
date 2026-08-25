use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

type BinaryFn = unsafe extern "C" fn(c_int, c_int) -> c_int;
type StringFn = unsafe extern "C" fn(*mut c_char, c_int);
type UnaryFn = unsafe extern "C" fn(c_int) -> c_int;
type FindrepFn = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

const CASES: usize = 32;
static FIXTURE_ID: AtomicU64 = AtomicU64::new(0);

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

    fn i32(&mut self) -> i32 {
        self.next_u32() as i32
    }

    fn range(&mut self, min: i32, max: i32) -> i32 {
        assert!(min <= max);
        let width = i64::from(max) - i64::from(min) + 1;
        (i64::from(min) + i64::from(self.next_u32()) % width) as i32
    }

    fn nonzero(&mut self, min: i32, max: i32) -> i32 {
        loop {
            let value = self.range(min, max);
            if value != 0 {
                return value;
            }
        }
    }
}

struct Fixture {
    dir: PathBuf,
    c_path: PathBuf,
    rust_path: PathBuf,
}

impl Fixture {
    fn new(label: &str) -> Self {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let profile = if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        };
        let target = std::env::var_os("CARGO_TARGET_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| manifest.join("target"));
        let target = if target.is_absolute() {
            target
        } else {
            manifest.join(target)
        };
        let rust_source = target.join(profile).join("libfindrep_lib.so");
        let c_source = manifest
            .join("c_src")
            .join("build")
            .join("libtranslated_rust.so");
        assert!(
            c_source.is_file(),
            "missing C library {}; build it with CMake first",
            c_source.display()
        );
        assert!(
            rust_source.is_file(),
            "missing Rust cdylib {}; build it before testing",
            rust_source.display()
        );

        let id = FIXTURE_ID.fetch_add(1, Ordering::Relaxed);
        let dir = target
            .join("differential-libs")
            .join(format!("{}-{label}-{id}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let c_path = dir.join("c.so");
        let rust_path = dir.join("rust.so");
        fs::copy(c_source, &c_path).unwrap();
        fs::copy(rust_source, &rust_path).unwrap();
        Self {
            dir,
            c_path,
            rust_path,
        }
    }

    unsafe fn pair(&self) -> Pair {
        Pair {
            c: unsafe { Library::new(&self.c_path) }.unwrap(),
            rust: unsafe { Library::new(&self.rust_path) }.unwrap(),
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

struct Pair {
    c: Library,
    rust: Library,
}

impl Pair {
    unsafe fn binary(&self, row: usize, name: &[u8], a: i32, b: i32) -> i32 {
        let c: Symbol<BinaryFn> = unsafe { self.c.get(name) }.unwrap();
        let rust: Symbol<BinaryFn> = unsafe { self.rust.get(name) }.unwrap();
        let expected = unsafe { c(a, b) };
        let actual = unsafe { rust(a, b) };
        assert_eq!(actual, expected, "CONFIGS.md row {row}: ({a}, {b})");
        actual
    }

    unsafe fn string(&self, row: usize, name: &[u8], bytes: &mut [u8], arg: i32) {
        let c: Symbol<StringFn> = unsafe { self.c.get(name) }.unwrap();
        let rust: Symbol<StringFn> = unsafe { self.rust.get(name) }.unwrap();
        let mut expected = bytes.to_vec();
        unsafe { c(expected.as_mut_ptr().cast(), arg) };
        unsafe { rust(bytes.as_mut_ptr().cast(), arg) };
        assert_eq!(bytes, expected, "CONFIGS.md row {row}: byte argument {arg}");
    }

    unsafe fn unary(&self, row: usize, name: &[u8], value: i32) -> i32 {
        let c: Symbol<UnaryFn> = unsafe { self.c.get(name) }.unwrap();
        let rust: Symbol<UnaryFn> = unsafe { self.rust.get(name) }.unwrap();
        let expected = unsafe { c(value) };
        let actual = unsafe { rust(value) };
        assert_eq!(actual, expected, "CONFIGS.md row {row}: value {value}");
        actual
    }

    unsafe fn findrep(&self, row: usize, values: [i32; 4]) -> i32 {
        let c: Symbol<FindrepFn> = unsafe { self.c.get(b"findrep\0") }.unwrap();
        let rust: Symbol<FindrepFn> = unsafe { self.rust.get(b"findrep\0") }.unwrap();
        let expected = unsafe { c(values[0], values[1], values[2], values[3]) };
        let actual = unsafe { rust(values[0], values[1], values[2], values[3]) };
        assert_eq!(
            actual, expected,
            "CONFIGS.md row {row}: parameters {values:?}"
        );
        actual
    }
}

fn with_fresh_pairs(label: &str, mut check: impl FnMut(&Pair, usize, &mut Rng)) {
    let fixture = Fixture::new(label);
    let mut rng = Rng::new(0x6a09_e667_f3bc_c909);
    for case in 0..CASES {
        let pair = unsafe { fixture.pair() };
        check(&pair, case, &mut rng);
        drop(pair);
    }
}

#[test]
fn arithmetic_rows_1_through_8() {
    with_fresh_pairs("add-fresh", |pair, _, rng| unsafe {
        pair.binary(1, b"add_to_accumulator\0", rng.i32(), rng.i32());
    });

    let fixture = Fixture::new("add-repeated");
    let pair = unsafe { fixture.pair() };
    let mut rng = Rng::new(0xbb67_ae85_84ca_a73b);
    for _ in 0..CASES {
        unsafe {
            pair.binary(2, b"add_to_accumulator\0", rng.i32(), rng.i32());
        }
    }

    with_fresh_pairs("multiply-fresh", |pair, _, rng| unsafe {
        pair.binary(
            3,
            b"multiply_with_multiplier\0",
            rng.range(-46_340, 46_340),
            rng.range(-46_340, 46_340),
        );
    });

    let fixture = Fixture::new("multiply-repeated");
    let pair = unsafe { fixture.pair() };
    let mut rng = Rng::new(0x3c6e_f372_fe94_f82b);
    let odd = [-7, -5, -3, -1, 1, 3, 5, 7];
    for _ in 0..CASES {
        let a = odd[rng.next_u32() as usize % odd.len()];
        let b = odd[rng.next_u32() as usize % odd.len()];
        unsafe {
            pair.binary(4, b"multiply_with_multiplier\0", a, b);
        }
    }

    with_fresh_pairs("subtract-fresh", |pair, _, rng| unsafe {
        pair.binary(5, b"subtract_from_accumulator\0", rng.i32(), rng.i32());
    });

    let fixture = Fixture::new("subtract-repeated");
    let pair = unsafe { fixture.pair() };
    let mut rng = Rng::new(0xa54f_f53a_5f1d_36f1);
    for _ in 0..CASES {
        unsafe {
            pair.binary(6, b"subtract_from_accumulator\0", rng.i32(), rng.i32());
        }
    }

    with_fresh_pairs("divide-zero", |pair, _, rng| unsafe {
        pair.binary(
            7,
            b"multiply_with_multiplier\0",
            rng.nonzero(-100_000, 100_000),
            1,
        );
        pair.binary(7, b"divide_multiplier\0", rng.i32(), 0);
    });

    with_fresh_pairs("divide-nonzero", |pair, _, rng| unsafe {
        pair.binary(
            8,
            b"multiply_with_multiplier\0",
            rng.nonzero(-100_000, 100_000),
            1,
        );
        pair.binary(8, b"divide_multiplier\0", rng.i32(), rng.nonzero(-31, 31));
    });
}

#[test]
fn string_rows_9_through_17() {
    let fixture = Fixture::new("strings");
    let pair = unsafe { fixture.pair() };
    let mut rng = Rng::new(0x510e_527f_ade6_82d1);

    for _ in 0..CASES {
        let mut dest = [0xa5; 64];
        unsafe { pair.string(9, b"process_octal_string\0", &mut dest, 0) };
    }
    for case in 0..CASES {
        let value = if case == 0 {
            i32::MAX
        } else {
            rng.range(1, i32::MAX)
        };
        let mut dest = [0xa5; 64];
        unsafe { pair.string(10, b"process_octal_string\0", &mut dest, value) };
    }
    for case in 0..CASES {
        let value = if case == 0 {
            i32::MIN
        } else {
            rng.range(i32::MIN, -1)
        };
        let mut dest = [0xa5; 64];
        unsafe { pair.string(11, b"process_octal_string\0", &mut dest, value) };
    }

    for _ in 0..CASES {
        let mut bytes = vec![0];
        unsafe {
            pair.string(
                12,
                b"find_and_replace_char\0",
                &mut bytes,
                rng.range(1, 255),
            )
        };
    }
    for _ in 0..CASES {
        let mut bytes = b"abcdef012345\0".to_vec();
        unsafe { pair.string(13, b"find_and_replace_char\0", &mut bytes, b'z'.into()) };
    }
    for _ in 0..CASES {
        let search = rng.range(b'a'.into(), b'z'.into()) as u8;
        let mut bytes = vec![search, b'q', b'r', 0];
        unsafe { pair.string(14, b"find_and_replace_char\0", &mut bytes, search.into()) };
    }
    for _ in 0..CASES {
        let search = rng.range(b'a'.into(), b'z'.into()) as u8;
        let mut bytes = vec![b'0', b'1', search, b'2', search, 0];
        unsafe { pair.string(15, b"find_and_replace_char\0", &mut bytes, search.into()) };
    }
    for _ in 0..CASES {
        let mut bytes = b"before\0target\0".to_vec();
        unsafe { pair.string(16, b"find_and_replace_char\0", &mut bytes, b't'.into()) };
    }
    for _ in 0..CASES {
        let search = rng.range(b'a'.into(), b'z'.into());
        let mut bytes = vec![b'0', search as u8, b'1', 0];
        unsafe {
            pair.string(
                17,
                b"find_and_replace_char\0",
                &mut bytes,
                search + 256 * rng.range(1, 1024),
            )
        };
    }
}

#[test]
fn normalization_rows_18_through_21_and_error_rows() {
    let fixture = Fixture::new("normalize");
    let pair = unsafe { fixture.pair() };
    let mut rng = Rng::new(0x9b05_688c_2b3e_6c1f);

    for case in 0..CASES {
        let value = if case == 0 {
            i32::MIN
        } else {
            rng.range(i32::MIN, 0)
        };
        assert_eq!(
            unsafe { pair.unary(18, b"validate_and_normalize\0", value) },
            value
        );
    }
    for _ in 0..CASES {
        let value = rng.range(1, 63);
        assert_eq!(
            unsafe { pair.unary(19, b"validate_and_normalize\0", value) },
            64,
            "ERRORS.md row 1"
        );
    }
    for case in 0..CASES {
        let value = match case {
            0 => 64,
            1 => 511,
            _ => rng.range(64, 511),
        };
        assert_eq!(
            unsafe { pair.unary(20, b"validate_and_normalize\0", value) },
            value
        );
    }
    for case in 0..CASES {
        let value = if case == 0 {
            i32::MAX
        } else {
            rng.range(512, i32::MAX)
        };
        assert_eq!(
            unsafe { pair.unary(21, b"validate_and_normalize\0", value) },
            511,
            "ERRORS.md row 2"
        );
    }
}

#[test]
fn findrep_rows_22_through_39() {
    with_fresh_pairs("findrep-22", |pair, _, _| unsafe {
        pair.findrep(22, [0, 0, 0, 0]);
    });
    with_fresh_pairs("findrep-23", |pair, case, rng| unsafe {
        let mut values = [0; 4];
        values[case % 2] = rng.range(1, 104);
        pair.findrep(23, values);
    });
    with_fresh_pairs("findrep-24", |pair, case, rng| unsafe {
        let mut values = [0; 4];
        values[case % 2] = rng.range(105, 511);
        pair.findrep(24, values);
    });
    with_fresh_pairs("findrep-25", |pair, case, rng| unsafe {
        let mut values = [0; 4];
        values[2 + case % 2] = rng.nonzero(-511, 511);
        pair.findrep(25, values);
    });
    with_fresh_pairs("findrep-26", |pair, _, rng| unsafe {
        pair.findrep(26, [rng.range(1, 104), rng.range(1, 104), 0, 0]);
    });
    with_fresh_pairs("findrep-27", |pair, _, rng| unsafe {
        pair.findrep(27, [0, 0, rng.range(1, 511), rng.range(1, 511)]);
    });
    with_fresh_pairs("findrep-28", |pair, case, rng| unsafe {
        let mut values = [0; 4];
        values[case % 2] = rng.range(1, 104);
        values[2 + (case / 2) % 2] = rng.nonzero(-511, 511);
        pair.findrep(28, values);
    });
    with_fresh_pairs("findrep-29", |pair, case, rng| unsafe {
        let mut values = [0; 4];
        values[case % 2] = rng.range(105, 511);
        values[2 + (case / 2) % 2] = rng.nonzero(-511, 511);
        pair.findrep(29, values);
    });
    with_fresh_pairs("findrep-30", |pair, case, rng| unsafe {
        let mut values = [
            rng.range(1, 104),
            rng.range(1, 104),
            rng.range(1, 511),
            rng.range(1, 511),
        ];
        values[case % 2] = 0;
        pair.findrep(30, values);
    });
    with_fresh_pairs("findrep-31", |pair, case, rng| unsafe {
        let mut values = [
            rng.range(105, 511),
            rng.range(105, 511),
            rng.range(1, 511),
            rng.range(1, 511),
        ];
        values[case % 2] = 0;
        pair.findrep(31, values);
    });
    with_fresh_pairs("findrep-32", |pair, case, rng| unsafe {
        let mut values = [
            rng.range(1, 511),
            rng.range(1, 511),
            rng.range(1, 511),
            rng.range(1, 511),
        ];
        values[2 + case % 2] = 0;
        pair.findrep(32, values);
    });
    with_fresh_pairs("findrep-33", |pair, _, rng| unsafe {
        pair.findrep(
            33,
            [
                rng.range(1, 511),
                rng.range(1, 511),
                rng.range(1, 511),
                rng.range(1, 511),
            ],
        );
    });
    with_fresh_pairs("findrep-34", |pair, _, rng| unsafe {
        pair.binary(34, b"add_to_accumulator\0", rng.range(105, 500), 0);
        pair.findrep(34, [0; 4]);
    });
    with_fresh_pairs("findrep-35", |pair, _, rng| unsafe {
        pair.binary(35, b"multiply_with_multiplier\0", rng.range(65, 500), 1);
        pair.findrep(35, [0; 4]);
    });
    with_fresh_pairs("findrep-36", |pair, _, rng| unsafe {
        pair.binary(36, b"add_to_accumulator\0", rng.nonzero(-100, 100), 0);
        pair.findrep(36, [0; 4]);
    });
    with_fresh_pairs("findrep-37", |pair, _, rng| unsafe {
        pair.binary(37, b"add_to_accumulator\0", rng.nonzero(-100, 100), 0);
        pair.binary(37, b"multiply_with_multiplier\0", 0, rng.i32());
        pair.findrep(37, [0; 4]);
    });
    with_fresh_pairs("findrep-38", |pair, _, rng| unsafe {
        let a = rng.range(-10_000, 10_000);
        pair.binary(38, b"add_to_accumulator\0", a, -18 - a);
        assert_eq!(pair.findrep(38, [0; 4]), 0o777);
    });

    let fixture = Fixture::new("findrep-39");
    let pair = unsafe { fixture.pair() };
    let mut rng = Rng::new(0x1f83_d9ab_fb41_bd6b);
    for _ in 0..CASES {
        unsafe {
            pair.findrep(
                39,
                [
                    rng.range(-120, 120),
                    rng.range(-120, 120),
                    rng.range(-120, 120),
                    rng.range(-120, 120),
                ],
            );
        }
    }
}

fn exported_symbols(path: &Path) -> Vec<String> {
    let output = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(path)
        .output()
        .unwrap();
    assert!(output.status.success());
    let mut symbols: Vec<_> = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let _address = fields.next()?;
            let kind = fields.next()?;
            let name = fields.next()?;
            (kind == "T").then(|| name.to_owned())
        })
        .collect();
    symbols.sort();
    symbols
}

#[test]
fn dynamic_symbol_surface_matches() {
    let fixture = Fixture::new("symbols");
    let expected = exported_symbols(&fixture.c_path);
    let actual = exported_symbols(&fixture.rust_path);
    assert_eq!(actual, expected, "SYMBOLS.md dynamic export mismatch");
}

#[test]
fn null_pointer_behavior_matches_in_subprocesses() {
    use std::os::unix::process::ExitStatusExt;

    for function in ["process_octal_string", "find_and_replace_char"] {
        let run = |implementation: &str| {
            Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "null_pointer_child_entry"])
                .env("DIFF_NULL_FUNCTION", function)
                .env("DIFF_NULL_IMPLEMENTATION", implementation)
                .status()
                .unwrap()
        };
        let c = run("c");
        let rust = run("rust");
        assert_eq!(
            rust.signal(),
            c.signal(),
            "null-pointer signal differs for {function}"
        );
        assert_eq!(c.signal(), Some(11), "C did not terminate with SIGSEGV");
    }
}

#[test]
fn null_pointer_child_entry() {
    let Ok(function) = std::env::var("DIFF_NULL_FUNCTION") else {
        return;
    };
    let implementation = std::env::var("DIFF_NULL_IMPLEMENTATION").unwrap();
    let fixture = Fixture::new("null-child");
    let path = if implementation == "c" {
        &fixture.c_path
    } else {
        &fixture.rust_path
    };
    let library = unsafe { Library::new(path) }.unwrap();
    let symbol: Symbol<StringFn> =
        unsafe { library.get(format!("{function}\0").as_bytes()) }.unwrap();
    unsafe { symbol(std::ptr::null_mut(), 1) };
    panic!("null-pointer call unexpectedly returned");
}
