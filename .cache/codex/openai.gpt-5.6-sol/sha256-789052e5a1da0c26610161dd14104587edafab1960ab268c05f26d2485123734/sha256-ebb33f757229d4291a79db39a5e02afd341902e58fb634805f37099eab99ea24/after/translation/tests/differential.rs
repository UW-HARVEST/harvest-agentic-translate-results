use libloading::Library;
use std::ffi::{c_char, c_int};
use std::fs;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

type BinaryFn = unsafe extern "C" fn(c_int, c_int) -> c_int;
type StringFn = unsafe extern "C" fn(*mut c_char, c_int);
type ValidateFn = unsafe extern "C" fn(c_int) -> c_int;
type FindrepFn = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

#[derive(Clone, Copy)]
struct Exports {
    add: BinaryFn,
    multiply: BinaryFn,
    subtract: BinaryFn,
    divide: BinaryFn,
    process_octal: StringFn,
    replace: StringFn,
    validate: ValidateFn,
    findrep: FindrepFn,
}

impl Exports {
    unsafe fn load(lib: &Library) -> Self {
        unsafe {
            Self {
                add: *lib.get(b"add_to_accumulator\0").unwrap(),
                multiply: *lib.get(b"multiply_with_multiplier\0").unwrap(),
                subtract: *lib.get(b"subtract_from_accumulator\0").unwrap(),
                divide: *lib.get(b"divide_multiplier\0").unwrap(),
                process_octal: *lib.get(b"process_octal_string\0").unwrap(),
                replace: *lib.get(b"find_and_replace_char\0").unwrap(),
                validate: *lib.get(b"validate_and_normalize\0").unwrap(),
                findrep: *lib.get(b"findrep\0").unwrap(),
            }
        }
    }
}

static NEXT_LIBRARY_ID: AtomicU64 = AtomicU64::new(0);

fn source_libraries() -> (PathBuf, PathBuf) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    (
        root.join("../c_src/build/libharvest-work-xr5AfX.so"),
        root.join("target/release/libfindrep_lib.so"),
    )
}

struct Pair {
    c: Exports,
    rust: Exports,
    _c_library: Library,
    _rust_library: Library,
    directory: PathBuf,
}

impl Pair {
    fn new() -> Self {
        let id = NEXT_LIBRARY_ID.fetch_add(1, Ordering::Relaxed);
        let directory =
            std::env::temp_dir().join(format!("findrep-differential-{}-{id}", std::process::id()));
        fs::create_dir(&directory).unwrap();

        let (c_source, rust_source) = source_libraries();
        assert!(
            c_source.is_file(),
            "missing C library: {}",
            c_source.display()
        );
        assert!(
            rust_source.is_file(),
            "missing Rust library: {}",
            rust_source.display()
        );

        let c_copy = directory.join(format!("libc-{id}.so"));
        let rust_copy = directory.join(format!("librust-{id}.so"));
        fs::copy(c_source, &c_copy).unwrap();
        fs::copy(rust_source, &rust_copy).unwrap();

        unsafe {
            let c_library = Library::new(c_copy).unwrap();
            let rust_library = Library::new(rust_copy).unwrap();
            let c = Exports::load(&c_library);
            let rust = Exports::load(&rust_library);
            Self {
                c,
                rust,
                _c_library: c_library,
                _rust_library: rust_library,
                directory,
            }
        }
    }

    fn binary(&self, c_fn: BinaryFn, rust_fn: BinaryFn, a: i32, b: i32) -> i32 {
        unsafe {
            let c_result = c_fn(a, b);
            let rust_result = rust_fn(a, b);
            assert_eq!(rust_result, c_result, "binary call diverged for ({a}, {b})");
            c_result
        }
    }

    fn validate(&self, value: i32) -> i32 {
        unsafe {
            let c_result = (self.c.validate)(value);
            let rust_result = (self.rust.validate)(value);
            assert_eq!(rust_result, c_result, "validate diverged for {value}");
            c_result
        }
    }

    fn findrep(&self, values: [i32; 4]) -> i32 {
        unsafe {
            let c_result = (self.c.findrep)(values[0], values[1], values[2], values[3]);
            let rust_result = (self.rust.findrep)(values[0], values[1], values[2], values[3]);
            assert_eq!(rust_result, c_result, "findrep diverged for {values:?}");
            c_result
        }
    }

    fn process_octal(&self, value: i32) -> Vec<u8> {
        let mut c_buffer = vec![0x55_u8; 128];
        let mut rust_buffer = c_buffer.clone();
        unsafe {
            (self.c.process_octal)(c_buffer.as_mut_ptr().cast(), value);
            (self.rust.process_octal)(rust_buffer.as_mut_ptr().cast(), value);
        }
        assert_eq!(
            rust_buffer, c_buffer,
            "octal formatting diverged for {value}"
        );
        c_buffer
    }

    fn replace(&self, bytes: &[u8], search: i32) -> Vec<u8> {
        assert!(!bytes.contains(&0));
        let mut c_buffer = bytes.to_vec();
        c_buffer.push(0);
        c_buffer.extend_from_slice(&[0x55; 8]);
        let mut rust_buffer = c_buffer.clone();
        unsafe {
            (self.c.replace)(c_buffer.as_mut_ptr().cast(), search);
            (self.rust.replace)(rust_buffer.as_mut_ptr().cast(), search);
        }
        assert_eq!(
            rust_buffer, c_buffer,
            "character replacement diverged for {bytes:?}, search {search}"
        );
        c_buffer
    }
}

impl Drop for Pair {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.directory);
    }
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        (x.wrapping_mul(0x2545_f491_4f6c_dd1d) >> 32) as u32
    }

    fn between(&mut self, min: i32, max: i32) -> i32 {
        assert!(min <= max);
        let span = (i64::from(max) - i64::from(min) + 1) as u64;
        (i64::from(min) + i64::try_from(u64::from(self.next_u32()) % span).unwrap()) as i32
    }

    fn nonzero_between(&mut self, min: i32, max: i32) -> i32 {
        loop {
            let value = self.between(min, max);
            if value != 0 {
                return value;
            }
        }
    }

    fn any_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
}

#[test]
fn config_validate_shapes() {
    let pair = Pair::new();
    let mut rng = Rng::new(0x5eed_0001);

    assert_eq!(pair.validate(i32::MIN), i32::MIN);
    for _ in 0..128 {
        let value = rng.between(-1_000_000, -1);
        assert_eq!(pair.validate(value), value);
    }

    assert_eq!(pair.validate(0), 0);

    for _ in 0..128 {
        let value = rng.between(1, 63);
        assert_eq!(pair.validate(value), 64);
    }

    for value in [64, 65, 510, 511] {
        assert_eq!(pair.validate(value), value);
    }
    for _ in 0..128 {
        let value = rng.between(64, 511);
        assert_eq!(pair.validate(value), value);
    }

    assert_eq!(pair.validate(i32::MAX), 511);
    for _ in 0..128 {
        let value = rng.between(512, 1_000_000);
        assert_eq!(pair.validate(value), 511);
    }
}

#[test]
fn config_accumulator_operations() {
    let mut rng = Rng::new(0x5eed_0002);

    for _ in 0..96 {
        let pair = Pair::new();
        let a = rng.between(-10_000, 10_000);
        let b = rng.between(-10_000, 10_000);
        pair.binary(pair.c.add, pair.rust.add, a, b);
    }

    for _ in 0..48 {
        let pair = Pair::new();
        for _ in 0..24 {
            let a = rng.between(-100, 100);
            let b = rng.between(-100, 100);
            pair.binary(pair.c.add, pair.rust.add, a, b);
        }
    }

    for values in [(9, 2), (5, 5), (2, 9), (-4, -9)] {
        let pair = Pair::new();
        pair.binary(pair.c.subtract, pair.rust.subtract, values.0, values.1);
    }
    for _ in 0..96 {
        let pair = Pair::new();
        let a = rng.between(-10_000, 10_000);
        let b = rng.between(-10_000, 10_000);
        pair.binary(pair.c.subtract, pair.rust.subtract, a, b);
    }

    for _ in 0..48 {
        let pair = Pair::new();
        for step in 0..24 {
            let a = rng.between(-100, 100);
            let b = rng.between(-100, 100);
            if step % 2 == 0 {
                pair.binary(pair.c.add, pair.rust.add, a, b);
            } else {
                pair.binary(pair.c.subtract, pair.rust.subtract, a, b);
            }
        }
    }
}

#[test]
fn config_multiplier_and_division() {
    let mut rng = Rng::new(0x5eed_0003);

    for _ in 0..96 {
        let pair = Pair::new();
        let a = rng.between(1, 100);
        let b = rng.between(1, 100);
        pair.binary(pair.c.multiply, pair.rust.multiply, a, b);
    }

    for &(a, b) in &[(0, 7), (7, 0), (0, 0)] {
        let pair = Pair::new();
        assert_eq!(pair.binary(pair.c.multiply, pair.rust.multiply, a, b), 0);
    }

    for _ in 0..64 {
        let pair = Pair::new();
        for _ in 0..8 {
            let a = rng.nonzero_between(-3, 3);
            let b = rng.nonzero_between(-3, 3);
            pair.binary(pair.c.multiply, pair.rust.multiply, a, b);
        }
    }

    for _ in 0..96 {
        let pair = Pair::new();
        let a = rng.between(1, 100);
        let b = rng.between(1, 100);
        pair.binary(pair.c.multiply, pair.rust.multiply, a, b);
        let divisor = rng.between(1, 100);
        pair.binary(
            pair.c.divide,
            pair.rust.divide,
            rng.between(-50, 50),
            divisor,
        );
    }

    for _ in 0..96 {
        let pair = Pair::new();
        let a = rng.nonzero_between(-100, 100);
        let b = rng.nonzero_between(-100, 100);
        pair.binary(pair.c.multiply, pair.rust.multiply, a, b);
        let divisor = rng.nonzero_between(-100, -1);
        pair.binary(
            pair.c.divide,
            pair.rust.divide,
            rng.between(-50, 50),
            divisor,
        );
    }

    for _ in 0..96 {
        let pair = Pair::new();
        let before = pair.binary(
            pair.c.multiply,
            pair.rust.multiply,
            rng.between(-100, 100),
            rng.between(-100, 100),
        );
        let after = pair.binary(pair.c.divide, pair.rust.divide, rng.between(-100, 100), 0);
        assert_eq!(after, before);
    }
}

#[test]
fn config_signed_integer_boundaries() {
    let pair = Pair::new();
    pair.binary(pair.c.add, pair.rust.add, i32::MAX, 1);

    let pair = Pair::new();
    pair.binary(pair.c.add, pair.rust.add, i32::MIN, -1);

    let pair = Pair::new();
    pair.binary(pair.c.subtract, pair.rust.subtract, i32::MIN, 1);

    let pair = Pair::new();
    pair.binary(pair.c.subtract, pair.rust.subtract, i32::MAX, -1);

    let pair = Pair::new();
    pair.binary(pair.c.multiply, pair.rust.multiply, i32::MAX, 2);

    let pair = Pair::new();
    pair.binary(pair.c.multiply, pair.rust.multiply, i32::MIN, -1);
}

#[test]
fn config_full_i32_domain_randomized() {
    let mut rng = Rng::new(0x5eed_0009);

    for _ in 0..256 {
        let a = rng.any_i32();
        let b = rng.any_i32();

        let pair = Pair::new();
        pair.binary(pair.c.add, pair.rust.add, a, b);

        let pair = Pair::new();
        pair.binary(pair.c.subtract, pair.rust.subtract, a, b);

        let pair = Pair::new();
        pair.binary(pair.c.multiply, pair.rust.multiply, a, b);

        let pair = Pair::new();
        let divisor = loop {
            let candidate = rng.any_i32();
            if candidate != 0 && !(a == i32::MIN && candidate == -1) {
                break candidate;
            }
        };
        pair.binary(pair.c.multiply, pair.rust.multiply, a, 1);
        pair.binary(pair.c.divide, pair.rust.divide, b, divisor);

        let pair = Pair::new();
        pair.validate(a);
        pair.process_octal(b);
        pair.findrep([a, b, rng.any_i32(), rng.any_i32()]);
    }

    let pair = Pair::new();
    for _ in 0..256 {
        let len = rng.between(0, 96) as usize;
        let bytes: Vec<u8> = (0..len)
            .map(|_| {
                loop {
                    let byte = (rng.next_u32() & 0xff) as u8;
                    if byte != 0 {
                        break byte;
                    }
                }
            })
            .collect();
        pair.replace(&bytes, rng.any_i32());
    }
}

#[test]
fn config_octal_strings() {
    let pair = Pair::new();
    assert!(
        pair.process_octal(0)
            .starts_with(b"Octal: 00, Decimal: 0\0")
    );

    let mut rng = Rng::new(0x5eed_0004);
    for value in [1, 7, 8, 63, 64, 511, 512, i32::MAX] {
        pair.process_octal(value);
    }
    for _ in 0..128 {
        pair.process_octal(rng.between(1, i32::MAX));
    }

    pair.process_octal(i32::MIN);
    for _ in 0..128 {
        pair.process_octal(rng.between(-1_000_000, -1));
    }
}

#[test]
fn config_find_and_replace_shapes() {
    let pair = Pair::new();
    assert_eq!(pair.replace(b"", b'A' as i32)[0], 0);

    let mut rng = Rng::new(0x5eed_0005);
    for _ in 0..128 {
        let len = rng.between(1, 80) as usize;
        let bytes: Vec<u8> = (0..len)
            .map(|_| rng.between(b'a' as i32, b'y' as i32) as u8)
            .collect();
        pair.replace(&bytes, b'z' as i32);
    }

    for _ in 0..128 {
        let len = rng.between(1, 80) as usize;
        let mut bytes = vec![b'a'; len];
        bytes[0] = b'Q';
        assert_eq!(pair.replace(&bytes, b'Q' as i32)[0], b'X');
    }

    for _ in 0..128 {
        let len = rng.between(2, 80) as usize;
        let index = rng.between(1, (len - 1) as i32) as usize;
        let mut bytes = vec![b'a'; len];
        bytes[index] = b'Q';
        bytes[len - 1] = b'Q';
        let result = pair.replace(&bytes, b'Q' as i32);
        assert_eq!(result[index], b'X');
        if index != len - 1 {
            assert_eq!(result[len - 1], b'Q');
        }
    }

    for search in [b'A' as i32 + 256, b'A' as i32 - 256, b'A' as i32 + 512] {
        let result = pair.replace(b"zAz", search);
        assert_eq!(result[1], b'X');
    }

    let result = pair.replace(b"abc", 0);
    assert_eq!(&result[..4], b"abc\0");
}

#[test]
fn config_findrep_active_counts_and_normalization() {
    for _ in 0..64 {
        assert_eq!(Pair::new().findrep([0, 0, 0, 0]), 9);
    }

    let mut rng = Rng::new(0x5eed_0006);
    for position in 0..4 {
        for _ in 0..64 {
            let mut values = [0; 4];
            values[position] = rng.nonzero_between(-800, 800);
            Pair::new().findrep(values);
        }
    }

    let position_pairs = [(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)];
    for &(first, second) in &position_pairs {
        for _ in 0..32 {
            let mut values = [0; 4];
            values[first] = rng.nonzero_between(-800, 800);
            values[second] = rng.nonzero_between(-800, 800);
            Pair::new().findrep(values);
        }
    }

    for active in [3, 4] {
        for _ in 0..96 {
            let mut values = [0; 4];
            for value in values.iter_mut().take(active) {
                *value = rng.nonzero_between(-800, 800);
            }
            Pair::new().findrep(values);
        }
    }

    for _ in 0..128 {
        let values = [
            rng.between(-1_000, -1),
            rng.between(1, 63),
            rng.between(64, 511),
            rng.between(512, 2_000),
        ];
        Pair::new().findrep(values);
    }
}

#[test]
fn config_findrep_state_branches() {
    let mut rng = Rng::new(0x5eed_0007);

    for _ in 0..96 {
        let pair = Pair::new();
        let accumulator = rng.between(-200, 104);
        pair.binary(pair.c.add, pair.rust.add, accumulator, 0);
        pair.findrep([0, 0, 0, 0]);
    }

    for _ in 0..96 {
        let pair = Pair::new();
        let accumulator = rng.between(105, 2_000);
        pair.binary(pair.c.add, pair.rust.add, accumulator, 0);
        pair.findrep([0, 0, 0, 0]);
    }
    for _ in 0..96 {
        Pair::new().findrep([rng.between(53, 511), rng.between(53, 511), 0, 0]);
    }

    for _ in 0..96 {
        let pair = Pair::new();
        pair.binary(pair.c.add, pair.rust.add, rng.nonzero_between(-100, 100), 0);
        pair.binary(
            pair.c.multiply,
            pair.rust.multiply,
            0,
            rng.between(-100, 100),
        );
        pair.findrep([0, 0, 0, 0]);
    }

    for multiplier in [1, 2, 7, 32, 63, 64] {
        for _ in 0..24 {
            let pair = Pair::new();
            pair.binary(pair.c.add, pair.rust.add, rng.nonzero_between(-100, 100), 0);
            pair.binary(pair.c.multiply, pair.rust.multiply, multiplier, 1);
            pair.findrep([0, 0, 0, 0]);
        }
    }

    for _ in 0..128 {
        let pair = Pair::new();
        pair.binary(pair.c.add, pair.rust.add, rng.nonzero_between(-100, 100), 0);
        pair.binary(
            pair.c.multiply,
            pair.rust.multiply,
            rng.between(65, 1_000),
            1,
        );
        pair.findrep([0, 0, 0, 0]);
    }
}

#[test]
fn config_findrep_repeated_and_interleaved_state() {
    let mut rng = Rng::new(0x5eed_0008);

    for _ in 0..32 {
        let pair = Pair::new();
        for _ in 0..64 {
            let values = [
                rng.between(-20, 20),
                rng.between(-20, 20),
                rng.between(-3, 3),
                rng.between(-3, 3),
            ];
            pair.findrep(values);
        }
    }

    for _ in 0..32 {
        let pair = Pair::new();
        for step in 0..64 {
            let a = rng.between(-10, 10);
            let b = rng.between(-10, 10);
            match step % 4 {
                0 => {
                    pair.binary(pair.c.add, pair.rust.add, a, b);
                }
                1 => {
                    pair.binary(pair.c.subtract, pair.rust.subtract, a, b);
                }
                2 => {
                    pair.binary(
                        pair.c.multiply,
                        pair.rust.multiply,
                        rng.between(-2, 2),
                        rng.between(-2, 2),
                    );
                }
                _ => {
                    pair.binary(
                        pair.c.divide,
                        pair.rust.divide,
                        a,
                        rng.nonzero_between(-3, 3),
                    );
                }
            }
            pair.findrep([
                rng.between(-10, 10),
                rng.between(-10, 10),
                rng.between(-2, 2),
                rng.between(-2, 2),
            ]);
        }
    }
}

#[test]
fn config_findrep_zero_result_sentinel() {
    for _ in 0..64 {
        let pair = Pair::new();
        assert_eq!(pair.binary(pair.c.add, pair.rust.add, -18, 0), -18);
        assert_eq!(pair.findrep([0, 0, 0, 0]), 0o777);
    }
}

#[test]
fn error_null_pointer_faults_match() {
    let executable = std::env::current_exe().unwrap();
    let (c_library, rust_library) = source_libraries();

    for operation in ["process_octal", "replace", "divide_overflow"] {
        let run = |library: &Path| {
            Command::new(&executable)
                .arg("--exact")
                .arg("ffi_null_pointer_probe_child")
                .arg("--nocapture")
                .env("FINDREP_NULL_PROBE_LIBRARY", library)
                .env("FINDREP_NULL_PROBE_OPERATION", operation)
                .status()
                .unwrap()
        };

        let c_status = run(&c_library);
        let rust_status = run(&rust_library);
        assert!(
            !c_status.success(),
            "C unexpectedly accepted NULL for {operation}"
        );
        assert!(
            !rust_status.success(),
            "Rust unexpectedly accepted NULL for {operation}"
        );
        assert_eq!(
            rust_status.signal(),
            c_status.signal(),
            "different native fault signal for {operation}"
        );
    }
}

#[test]
fn ffi_null_pointer_probe_child() {
    let Ok(library_path) = std::env::var("FINDREP_NULL_PROBE_LIBRARY") else {
        return;
    };
    let operation = std::env::var("FINDREP_NULL_PROBE_OPERATION").unwrap();
    unsafe {
        let library = Library::new(library_path).unwrap();
        match operation.as_str() {
            "process_octal" => {
                let function: libloading::Symbol<StringFn> =
                    library.get(b"process_octal_string\0").unwrap();
                function(std::ptr::null_mut(), 83);
            }
            "replace" => {
                let function: libloading::Symbol<StringFn> =
                    library.get(b"find_and_replace_char\0").unwrap();
                function(std::ptr::null_mut(), b'A' as i32);
            }
            "divide_overflow" => {
                let multiply: libloading::Symbol<BinaryFn> =
                    library.get(b"multiply_with_multiplier\0").unwrap();
                let divide: libloading::Symbol<BinaryFn> =
                    library.get(b"divide_multiplier\0").unwrap();
                multiply(i32::MIN, 1);
                divide(0, -1);
            }
            _ => panic!("unknown null probe operation: {operation}"),
        }
    }
}
