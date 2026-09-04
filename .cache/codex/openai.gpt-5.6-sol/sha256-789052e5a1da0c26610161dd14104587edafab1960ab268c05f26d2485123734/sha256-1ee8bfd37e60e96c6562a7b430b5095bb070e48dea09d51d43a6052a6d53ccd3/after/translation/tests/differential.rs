use libloading::Library;
use std::ffi::{c_char, c_int};
use std::fs;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use std::sync::atomic::{AtomicU64, Ordering};

type BinaryFn = unsafe extern "C" fn(c_int, c_int) -> c_int;
type StringFn = unsafe extern "C" fn(*mut c_char, c_int);
type UnaryFn = unsafe extern "C" fn(c_int) -> c_int;
type FindrepFn = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

static COPY_ID: AtomicU64 = AtomicU64::new(0);

struct Api {
    _library: Library,
    add: BinaryFn,
    multiply: BinaryFn,
    subtract: BinaryFn,
    divide: BinaryFn,
    process_octal: StringFn,
    replace: StringFn,
    normalize: UnaryFn,
    findrep: FindrepFn,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }.unwrap();
        let add = unsafe { *library.get(b"add_to_accumulator\0").unwrap() };
        let multiply = unsafe { *library.get(b"multiply_with_multiplier\0").unwrap() };
        let subtract = unsafe { *library.get(b"subtract_from_accumulator\0").unwrap() };
        let divide = unsafe { *library.get(b"divide_multiplier\0").unwrap() };
        let process_octal = unsafe { *library.get(b"process_octal_string\0").unwrap() };
        let replace = unsafe { *library.get(b"find_and_replace_char\0").unwrap() };
        let normalize = unsafe { *library.get(b"validate_and_normalize\0").unwrap() };
        let findrep = unsafe { *library.get(b"findrep\0").unwrap() };
        Self {
            _library: library,
            add,
            multiply,
            subtract,
            divide,
            process_octal,
            replace,
            normalize,
            findrep,
        }
    }
}

struct Pair {
    c: Api,
    rust: Api,
    copy_dir: PathBuf,
}

impl Pair {
    fn fresh(label: &str) -> Self {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_source = manifest.join("../c_src/build/libharvest-work-L8zxXO.so");
        let rust_source = manifest.join("target/release/libfindrep_lib.so");
        assert!(
            c_source.is_file(),
            "missing C shared library: {}",
            c_source.display()
        );
        assert!(
            rust_source.is_file(),
            "missing release Rust shared library: {}",
            rust_source.display()
        );

        let id = COPY_ID.fetch_add(1, Ordering::Relaxed);
        let copy_dir = std::env::temp_dir().join(format!(
            "findrep-differential-{}-{id}-{label}",
            std::process::id()
        ));
        fs::create_dir_all(&copy_dir).unwrap();
        let c_copy = copy_dir.join("libground_truth.so");
        let rust_copy = copy_dir.join("libtranslation.so");
        fs::copy(c_source, &c_copy).unwrap();
        fs::copy(rust_source, &rust_copy).unwrap();

        let c = unsafe { Api::load(&c_copy) };
        let rust = unsafe { Api::load(&rust_copy) };
        Self { c, rust, copy_dir }
    }

    fn binary(&self, row: u32, c_fn: BinaryFn, rust_fn: BinaryFn, a: i32, b: i32) -> i32 {
        let c = unsafe { c_fn(a, b) };
        let rust = unsafe { rust_fn(a, b) };
        assert_eq!(rust, c, "CONFIGS.md row {row}: inputs ({a}, {b})");
        c
    }

    fn add(&self, row: u32, a: i32, b: i32) -> i32 {
        self.binary(row, self.c.add, self.rust.add, a, b)
    }

    fn multiply(&self, row: u32, a: i32, b: i32) -> i32 {
        self.binary(row, self.c.multiply, self.rust.multiply, a, b)
    }

    fn subtract(&self, row: u32, a: i32, b: i32) -> i32 {
        self.binary(row, self.c.subtract, self.rust.subtract, a, b)
    }

    fn divide(&self, row: u32, a: i32, b: i32) -> i32 {
        self.binary(row, self.c.divide, self.rust.divide, a, b)
    }

    fn normalize(&self, row: u32, value: i32) -> i32 {
        let c = unsafe { (self.c.normalize)(value) };
        let rust = unsafe { (self.rust.normalize)(value) };
        assert_eq!(rust, c, "CONFIGS.md row {row}: input {value}");
        c
    }

    fn findrep(&self, row: u32, values: [i32; 4]) -> i32 {
        let [a, b, c, d] = values;
        let c_result = unsafe { (self.c.findrep)(a, b, c, d) };
        let rust_result = unsafe { (self.rust.findrep)(a, b, c, d) };
        assert_eq!(
            rust_result, c_result,
            "CONFIGS.md row {row}: inputs {values:?}"
        );
        c_result
    }

    fn process_octal(&self, row: u32, value: i32) {
        let mut c_buffer = [0x5a_u8; 128];
        let mut rust_buffer = [0x5a_u8; 128];
        unsafe {
            (self.c.process_octal)(c_buffer.as_mut_ptr().cast(), value);
            (self.rust.process_octal)(rust_buffer.as_mut_ptr().cast(), value);
        }
        assert_eq!(rust_buffer, c_buffer, "CONFIGS.md row {row}: input {value}");
    }

    fn replace(&self, row: u32, input: &[u8], search: i32) {
        assert_eq!(input.last(), Some(&0));
        assert!(!input[..input.len() - 1].contains(&0));
        let mut c_buffer = input.to_vec();
        let mut rust_buffer = input.to_vec();
        unsafe {
            (self.c.replace)(c_buffer.as_mut_ptr().cast(), search);
            (self.rust.replace)(rust_buffer.as_mut_ptr().cast(), search);
        }
        assert_eq!(
            rust_buffer,
            c_buffer,
            "CONFIGS.md row {row}: search {search}, input length {}",
            input.len()
        );
    }
}

impl Drop for Pair {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.copy_dir);
    }
}

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

    fn range(&mut self, min: i32, max: i32) -> i32 {
        assert!(min <= max);
        min + (self.next_u32() % ((max - min + 1) as u32)) as i32
    }

    fn choose<T: Copy>(&mut self, values: &[T]) -> T {
        values[(self.next_u32() as usize) % values.len()]
    }
}

#[test]
fn configs_01_08_low_level_integer_mutators() {
    let pair = Pair::fresh("rows-01-08");
    assert_eq!(pair.add(1, 0, 0), 0);

    let mut rng = Rng::new(0x0102_0304_0506_0708);
    for _ in 0..128 {
        let a = rng.range(-100, 100);
        let b = rng.range(-100, 100);
        pair.add(2, a, b);
    }

    let pair = Pair::fresh("row-03");
    for _ in 0..64 {
        let nonzero = rng.choose(&[-7, -3, -1, 1, 2, 9]);
        if rng.next_u32() & 1 == 0 {
            pair.multiply(3, 0, nonzero);
        } else {
            pair.multiply(3, nonzero, 0);
        }
    }

    for case in 0..64 {
        let pair = Pair::fresh(&format!("row-04-{case}"));
        let a = rng.choose(&[-4, -3, -2, -1, 1, 2, 3, 4]);
        let b = rng.choose(&[-4, -3, -2, -1, 1, 2, 3, 4]);
        pair.multiply(4, a, b);
        for _ in 0..8 {
            pair.multiply(4, rng.choose(&[-1, 1]), rng.choose(&[-1, 1]));
        }
    }

    let pair = Pair::fresh("rows-05-07");
    for _ in 0..64 {
        let value = rng.range(-1000, 1000);
        pair.subtract(5, value, value);
        pair.subtract(6, rng.range(-100, 100), rng.range(-100, 100));
        pair.divide(7, rng.range(-100, 100), 0);
    }

    for case in 0..128 {
        let pair = Pair::fresh(&format!("row-08-{case}"));
        let a = rng.choose(&[-20, -13, -7, -3, 1, 2, 9, 17]);
        let b = rng.choose(&[-11, -5, -2, 1, 3, 7, 10, 19]);
        pair.multiply(8, a, b);
        let divisor = rng.choose(&[-9, -7, -4, -3, -2, -1, 1, 2, 3, 5, 8]);
        pair.divide(8, rng.range(-100, 100), divisor);
    }
}

#[test]
fn configs_09_18_string_operations() {
    let pair = Pair::fresh("rows-09-18");
    for _ in 0..64 {
        pair.process_octal(9, 0);
    }

    let mut rng = Rng::new(0x1817_1615_1413_1211);
    let positive_edges = [1, 63, 64, 65, 510, 511, 512, i32::MAX];
    let negative_edges = [-1, -63, -64, -511, -512, i32::MIN];
    for _ in 0..128 {
        pair.process_octal(10, rng.choose(&positive_edges));
        pair.process_octal(11, rng.choose(&negative_edges));
    }

    for _ in 0..64 {
        pair.replace(12, b"\0", rng.range(-1024, 1024));

        let absent_len = rng.range(1, 96) as usize;
        let mut absent = vec![b'a'; absent_len];
        absent.push(0);
        pair.replace(13, &absent, b'z' as i32);

        let present_len = rng.range(1, 96) as usize;
        let index = rng.range(0, present_len as i32 - 1) as usize;
        let mut present = vec![b'a'; present_len];
        present[index] = b'q';
        present.push(0);
        pair.replace(14, &present, b'q' as i32);

        let mut multiple = vec![b'a'; 96];
        let first = rng.range(0, 30) as usize;
        let second = rng.range(31, 95) as usize;
        multiple[first] = b'p';
        multiple[second] = b'p';
        multiple.push(0);
        pair.replace(15, &multiple, b'p' as i32);

        let byte = rng.range(b'A' as i32, b'Z' as i32);
        let high_multiple = rng.choose(&[-4, -3, -2, -1, 1, 2, 3, 4]) * 256;
        let out_of_range_search = byte + high_multiple;
        let mut converted_match = b"converted byte: ?\0".to_vec();
        converted_match[16] = byte as u8;
        pair.replace(16, &converted_match, out_of_range_search);
        pair.replace(16, b"lowercase only\0", out_of_range_search);
        pair.replace(17, b"terminator is excluded\0", 0);
    }

    let mut long = vec![b'a'; 65_536];
    long[65_535] = b'z';
    long.push(0);
    for _ in 0..32 {
        pair.replace(18, &long, b'z' as i32);
        long[65_535] = b'z';
    }
}

#[test]
fn configs_19_25_normalization_partitions() {
    let pair = Pair::fresh("rows-19-25");
    let mut rng = Rng::new(0x2524_2322_2120_1918);
    for _ in 0..256 {
        pair.normalize(
            19,
            rng.choose(&[i32::MIN, -1, -2, -63, -64, -511, -100_000]),
        );
        pair.normalize(20, 0);
        pair.normalize(21, rng.range(1, 63));
        pair.normalize(22, 64);
        pair.normalize(23, rng.range(65, 510));
        pair.normalize(24, 511);
        pair.normalize(25, rng.choose(&[512, 513, 1000, 65_535, i32::MAX]));
    }
}

#[test]
fn configs_26_35_findrep_branch_matrix() {
    let mut rng = Rng::new(0x3534_3332_3130_2928);

    let pair = Pair::fresh("row-26");
    for _ in 0..128 {
        pair.findrep(26, [0, 0, 0, 0]);
    }

    let pair = Pair::fresh("row-27-p1");
    for _ in 0..64 {
        pair.findrep(27, [rng.range(-100, -1), 0, 0, 0]);
    }
    let pair = Pair::fresh("row-27-p2");
    for _ in 0..64 {
        pair.findrep(27, [0, rng.range(-100, -1), 0, 0]);
    }

    let pair = Pair::fresh("row-28");
    for _ in 0..128 {
        if rng.next_u32() & 1 == 0 {
            pair.findrep(28, [0, 0, rng.range(-100, -1), 0]);
        } else {
            pair.findrep(28, [0, 0, 0, rng.range(-100, -1)]);
        }
    }

    let pair = Pair::fresh("row-29");
    for _ in 0..128 {
        pair.findrep(29, [rng.range(-100, -1), 0, rng.range(-100, -1), 0]);
    }

    let pair = Pair::fresh("row-30");
    for _ in 0..128 {
        pair.findrep(30, [rng.range(-20, -1), 0, -1, -1]);
    }

    let pair = Pair::fresh("row-31");
    for iteration in 0..128 {
        if iteration != 0 {
            pair.divide(31, 0, 40);
        }
        pair.multiply(31, 9, 9);
        pair.findrep(31, [rng.range(-20, -1), 0, -1, -1]);
    }

    let pair = Pair::fresh("row-32");
    for _ in 0..128 {
        pair.findrep(32, [rng.range(1, 63), rng.range(1, 63), 0, 0]);
        let accumulator = pair.add(32, 0, 0);
        pair.subtract(32, accumulator, 0);
    }

    let pair = Pair::fresh("row-33");
    for _ in 0..128 {
        pair.findrep(33, [rng.range(-20, -1), 0, -1, -1]);
    }

    let pair = Pair::fresh("row-34-accumulator-zero");
    for _ in 0..64 {
        pair.findrep(34, [0, 0, rng.range(-20, -1), 0]);
    }
    let pair = Pair::fresh("row-34-multiplier-zero");
    pair.multiply(34, 0, 1);
    for _ in 0..64 {
        pair.findrep(34, [rng.range(-20, -1), 0, -1, -1]);
    }

    let classes: [&[i32]; 5] = [
        &[i32::MIN, -1000, -64, -1],
        &[0],
        &[1, 2, 32, 63],
        &[64, 65, 100, 510, 511],
        &[512, 513, 1000, i32::MAX],
    ];
    for position in 0..4 {
        for (class_index, class) in classes.iter().enumerate() {
            let pair = Pair::fresh(&format!("row-35-{position}-{class_index}"));
            for _ in 0..32 {
                let mut values = [-1, -1, -1, -1];
                values[position] = rng.choose(class);
                pair.findrep(35, values);
            }
        }
    }
}

#[test]
fn configs_36_37_stateful_composition() {
    let mut rng = Rng::new(0x3736_3534_3332_3130);

    for case in 0..64 {
        let pair = Pair::fresh(&format!("row-36-{case}"));
        match case % 3 {
            0 => {
                pair.add(36, rng.range(110, 200), rng.range(0, 20));
                pair.multiply(36, 9, 9);
            }
            1 => {
                pair.add(36, rng.range(-200, -110), rng.range(-20, 0));
                pair.multiply(36, -3, 7);
            }
            _ => {
                pair.add(36, 0, 0);
                pair.multiply(36, 0, rng.range(-20, 20));
            }
        }
        for _ in 0..16 {
            pair.findrep(
                36,
                [
                    rng.range(-20, 20),
                    rng.range(-20, 20),
                    rng.range(-3, -1),
                    rng.range(-3, -1),
                ],
            );
        }
    }

    let pair = Pair::fresh("row-37");
    for _ in 0..2048 {
        match rng.next_u32() % 5 {
            0 => {
                pair.add(37, rng.range(-10, 10), rng.range(-10, 10));
            }
            1 => {
                pair.multiply(37, rng.choose(&[-1, 0, 1]), rng.choose(&[-1, 1]));
            }
            2 => {
                pair.subtract(37, rng.range(-10, 10), rng.range(-10, 10));
            }
            3 => {
                pair.divide(37, rng.range(-10, 10), rng.choose(&[-2, -1, 0, 1, 2]));
            }
            _ => {
                pair.findrep(
                    37,
                    [
                        rng.range(-5, 5),
                        rng.range(-5, 5),
                        rng.range(-1, 0),
                        rng.range(-1, 0),
                    ],
                );
            }
        }
    }
}

fn null_child_status(library: &str, operation: &str) -> ExitStatus {
    Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "null_pointer_child", "--nocapture"])
        .env("FINDREP_NULL_CHILD_LIBRARY", library)
        .env("FINDREP_NULL_CHILD_OPERATION", operation)
        .status()
        .unwrap()
}

#[test]
fn null_pointer_child() {
    let Ok(library) = std::env::var("FINDREP_NULL_CHILD_LIBRARY") else {
        return;
    };
    let operation = std::env::var("FINDREP_NULL_CHILD_OPERATION").unwrap();
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let path = match library.as_str() {
        "c" => manifest.join("../c_src/build/libharvest-work-L8zxXO.so"),
        "rust" => manifest.join("target/release/libfindrep_lib.so"),
        _ => panic!("unknown library"),
    };
    let api = unsafe { Api::load(&path) };
    unsafe {
        match operation.as_str() {
            "process_octal_string" => (api.process_octal)(std::ptr::null_mut(), 123),
            "find_and_replace_char" => (api.replace)(std::ptr::null_mut(), b'x' as i32),
            _ => panic!("unknown operation"),
        }
    }
}

#[test]
fn generic_null_boundaries_match_process_termination() {
    for operation in ["process_octal_string", "find_and_replace_char"] {
        let c = null_child_status("c", operation);
        let rust = null_child_status("rust", operation);
        assert!(!c.success(), "C unexpectedly accepted null for {operation}");
        assert_eq!(
            rust.signal(),
            c.signal(),
            "null boundary termination mismatch for {operation}: C={c:?}, Rust={rust:?}"
        );
        assert_eq!(
            rust.code(),
            c.code(),
            "null boundary exit-code mismatch for {operation}: C={c:?}, Rust={rust:?}"
        );
    }
}

fn defined_dynamic_symbols(path: &Path) -> Vec<String> {
    let output = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(path)
        .output()
        .unwrap();
    assert!(output.status.success());
    let mut symbols: Vec<String> = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .filter_map(|line| line.split_whitespace().last())
        .map(str::to_owned)
        .collect();
    symbols.sort();
    symbols
}

#[test]
fn phase_d_dynamic_symbol_parity() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_path = manifest.join("../c_src/build/libharvest-work-L8zxXO.so");
    let rust_path = manifest.join("target/release/libfindrep_lib.so");
    let c_symbols = defined_dynamic_symbols(&c_path);
    let rust_symbols = defined_dynamic_symbols(&rust_path);
    let missing: Vec<_> = c_symbols
        .iter()
        .filter(|symbol| rust_symbols.binary_search(symbol).is_err())
        .collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing C symbols: {missing:?}"
    );
    assert_eq!(c_symbols.len(), 8, "unexpected C symbol-surface change");

    let _c_api = unsafe { Api::load(&c_path) };
    let _rust_api = unsafe { Api::load(&rust_path) };
}
