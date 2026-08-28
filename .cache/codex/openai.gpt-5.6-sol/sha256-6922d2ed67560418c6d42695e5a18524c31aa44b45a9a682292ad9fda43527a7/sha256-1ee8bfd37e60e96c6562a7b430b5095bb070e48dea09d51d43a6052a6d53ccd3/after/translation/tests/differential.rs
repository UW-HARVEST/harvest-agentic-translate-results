use libloading::Library;
use std::collections::BTreeSet;
use std::ffi::{c_char, c_int, c_void};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

type BinaryFn = unsafe extern "C" fn(c_int, c_int) -> c_int;
type ProcessFn = unsafe extern "C" fn(*mut c_char, c_int);
type NormalizeFn = unsafe extern "C" fn(c_int) -> c_int;
type FindrepFn = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

const CASES: usize = 64;
static COPY_ID: AtomicU64 = AtomicU64::new(0);

struct Api {
    _library: Library,
    add: BinaryFn,
    multiply: BinaryFn,
    subtract: BinaryFn,
    divide: BinaryFn,
    process_octal: ProcessFn,
    replace_char: ProcessFn,
    normalize: NormalizeFn,
    findrep: FindrepFn,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }.expect("load shared library");
        unsafe {
            Self {
                add: *library.get(b"add_to_accumulator\0").unwrap(),
                multiply: *library.get(b"multiply_with_multiplier\0").unwrap(),
                subtract: *library.get(b"subtract_from_accumulator\0").unwrap(),
                divide: *library.get(b"divide_multiplier\0").unwrap(),
                process_octal: *library.get(b"process_octal_string\0").unwrap(),
                replace_char: *library.get(b"find_and_replace_char\0").unwrap(),
                normalize: *library.get(b"validate_and_normalize\0").unwrap(),
                findrep: *library.get(b"findrep\0").unwrap(),
                _library: library,
            }
        }
    }
}

struct Pair {
    c: Api,
    rust: Api,
    c_copy: PathBuf,
    rust_copy: PathBuf,
}

impl Pair {
    fn fresh() -> Self {
        let id = COPY_ID.fetch_add(1, Ordering::Relaxed);
        let directory =
            std::env::temp_dir().join(format!("findrep-differential-{}-{id}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let c_copy = directory.join("libground_truth.so");
        let rust_copy = directory.join("libtranslation.so");
        std::fs::copy(c_library_path(), &c_copy).unwrap();
        std::fs::copy(rust_library_path(), &rust_copy).unwrap();

        unsafe {
            Self {
                c: Api::load(&c_copy),
                rust: Api::load(&rust_copy),
                c_copy,
                rust_copy,
            }
        }
    }

    fn binary(&self, name: &str, c_fn: BinaryFn, rust_fn: BinaryFn, a: i32, b: i32) {
        let (c_result, rust_result) = unsafe { (c_fn(a, b), rust_fn(a, b)) };
        assert_eq!(c_result, rust_result, "{name}({a}, {b})");
    }

    fn add(&self, a: i32, b: i32) {
        self.binary("add_to_accumulator", self.c.add, self.rust.add, a, b);
    }

    fn multiply(&self, a: i32, b: i32) {
        self.binary(
            "multiply_with_multiplier",
            self.c.multiply,
            self.rust.multiply,
            a,
            b,
        );
    }

    fn subtract(&self, a: i32, b: i32) {
        self.binary(
            "subtract_from_accumulator",
            self.c.subtract,
            self.rust.subtract,
            a,
            b,
        );
    }

    fn divide(&self, a: i32, b: i32) {
        self.binary("divide_multiplier", self.c.divide, self.rust.divide, a, b);
    }

    fn normalize(&self, value: i32) {
        let (c_result, rust_result) =
            unsafe { ((self.c.normalize)(value), (self.rust.normalize)(value)) };
        assert_eq!(c_result, rust_result, "validate_and_normalize({value})");
    }

    fn process_octal(&self, value: i32, fill: u8) {
        let mut c_buffer = [fill as c_char; 128];
        let mut rust_buffer = c_buffer;
        unsafe {
            (self.c.process_octal)(c_buffer.as_mut_ptr(), value);
            (self.rust.process_octal)(rust_buffer.as_mut_ptr(), value);
        }
        assert_eq!(
            bytes(&c_buffer),
            bytes(&rust_buffer),
            "process_octal_string(_, {value})"
        );
    }

    fn replace_char(&self, input: &[u8], search: i32) {
        assert!(!input.contains(&0));
        let mut c_buffer = input
            .iter()
            .copied()
            .chain([0])
            .map(|byte| byte as c_char)
            .collect::<Vec<_>>();
        let mut rust_buffer = c_buffer.clone();
        unsafe {
            (self.c.replace_char)(c_buffer.as_mut_ptr(), search);
            (self.rust.replace_char)(rust_buffer.as_mut_ptr(), search);
        }
        assert_eq!(
            bytes(&c_buffer),
            bytes(&rust_buffer),
            "find_and_replace_char({input:?}, {search})"
        );
    }

    fn findrep(&self, params: [i32; 4]) {
        let [a, b, c, d] = params;
        let (c_result, rust_result) = unsafe {
            (
                (self.c.findrep)(a, b, c, d),
                (self.rust.findrep)(a, b, c, d),
            )
        };
        assert_eq!(c_result, rust_result, "findrep({a}, {b}, {c}, {d})");
    }
}

impl Drop for Pair {
    fn drop(&mut self) {
        // The loaded handles are dropped after this method; remove the names now
        // and let the OS release the anonymous inodes when both handles close.
        let directory = self.c_copy.parent().unwrap().to_path_buf();
        let _ = std::fs::remove_file(&self.c_copy);
        let _ = std::fs::remove_file(&self.rust_copy);
        let _ = std::fs::remove_dir(directory);
    }
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
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

    fn range(&mut self, low: i32, high: i32) -> i32 {
        assert!(low <= high);
        let width = (high as i64 - low as i64 + 1) as u64;
        (low as i64 + (self.next_u32() as u64 % width) as i64) as i32
    }

    fn choose<T: Copy>(&mut self, values: &[T]) -> T {
        values[self.next_u32() as usize % values.len()]
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    manifest_dir().join("../c_src/build/libharvest-work-IP5DS8.so")
}

fn rust_library_path() -> PathBuf {
    manifest_dir().join("target/release/libfindrep_lib.so")
}

fn bytes(values: &[c_char]) -> Vec<u8> {
    values.iter().map(|value| *value as u8).collect()
}

fn random_nonzero(rng: &mut Rng) -> i32 {
    loop {
        let value = rng.range(-700, 700);
        if value != 0 {
            return value;
        }
    }
}

#[test]
fn every_configuration_matches() {
    let mut rng = Rng::new(0x5eed_1234_abcd_9876);

    // C01-C02: add, fresh and cumulative state.
    for _ in 0..CASES {
        let pair = Pair::fresh();
        pair.add(rng.range(-10_000, 10_000), rng.range(-10_000, 10_000));
    }
    for _ in 0..CASES {
        let pair = Pair::fresh();
        for _ in 0..24 {
            pair.add(rng.range(-100, 100), rng.range(-100, 100));
        }
        pair.findrep([0, 0, 0, 0]);
    }

    // C03-C04: multiply, fresh and cumulative state.
    for _ in 0..CASES {
        let pair = Pair::fresh();
        pair.multiply(rng.range(-1_000, 1_000), rng.range(-1_000, 1_000));
    }
    for _ in 0..CASES {
        let pair = Pair::fresh();
        for _ in 0..8 {
            pair.multiply(rng.choose(&[-2, -1, 0, 1, 2]), rng.choose(&[-1, 0, 1]));
        }
        pair.findrep([0, 0, 0, 0]);
    }

    // C05-C06: subtract, fresh and cumulative state.
    for _ in 0..CASES {
        let pair = Pair::fresh();
        pair.subtract(rng.range(-10_000, 10_000), rng.range(-10_000, 10_000));
    }
    for _ in 0..CASES {
        let pair = Pair::fresh();
        for _ in 0..24 {
            pair.subtract(rng.range(-100, 100), rng.range(-100, 100));
        }
        pair.findrep([0, 0, 0, 0]);
    }

    // C07-C08: zero and nonzero divisors.
    for _ in 0..CASES {
        let pair = Pair::fresh();
        pair.multiply(rng.range(-100, 100), rng.range(-100, 100));
        pair.divide(rng.range(i32::MIN, i32::MAX), 0);
        pair.findrep([0, 0, 0, 0]);
    }
    for _ in 0..CASES {
        let pair = Pair::fresh();
        pair.multiply(rng.range(-1_000, 1_000), rng.range(-1_000, 1_000));
        pair.divide(rng.range(i32::MIN, i32::MAX), random_nonzero(&mut rng));
        pair.findrep([0, 0, 0, 0]);
    }

    // C09-C11: octal formatting for zero, positive, and negative integers.
    for _ in 0..CASES {
        Pair::fresh().process_octal(0, rng.next_u32() as u8);
        let random_positive = rng.range(1, i32::MAX);
        Pair::fresh().process_octal(
            rng.choose(&[1, 63, 64, 511, 512, i32::MAX, random_positive]),
            rng.next_u32() as u8,
        );
        let random_negative = rng.range(i32::MIN, -1);
        Pair::fresh().process_octal(
            rng.choose(&[-1, -64, -511, i32::MIN, random_negative]),
            rng.next_u32() as u8,
        );
    }

    // C12-C17: every normalization branch and both exact thresholds.
    for _ in 0..CASES {
        Pair::fresh().normalize(rng.range(-10_000, 0));
        Pair::fresh().normalize(rng.range(1, 63));
        Pair::fresh().normalize(64);
        Pair::fresh().normalize(rng.range(65, 510));
        Pair::fresh().normalize(511);
        Pair::fresh().normalize(rng.range(512, 20_000));
    }

    // C18-C22: empty/absent/single/multiple/out-of-byte-range searches.
    for _ in 0..CASES {
        Pair::fresh().replace_char(b"", rng.range(-1_000, 1_000));
        Pair::fresh().replace_char(b"abcdef", rng.choose(&[0, b'z' as i32]));
        Pair::fresh().replace_char(b"abcdef", rng.choose(b"abcdef") as i32);
        Pair::fresh().replace_char(b"abracadabra", b'a' as i32);
        let search = rng.choose(&[-511, -255, 256 + b'a' as i32, 512 + b'b' as i32]);
        Pair::fresh().replace_char(b"alphabet", search);
    }

    // C23: no active parameters.
    for _ in 0..CASES {
        Pair::fresh().findrep([0, 0, 0, 0]);
    }

    // C24: one active parameter in every position.
    for _ in 0..CASES {
        for position in 0..4 {
            let mut params = [0; 4];
            params[position] = random_nonzero(&mut rng);
            Pair::fresh().findrep(params);
        }
    }

    // C25: two active parameters in every pair of positions.
    for _ in 0..CASES {
        for first in 0..4 {
            for second in (first + 1)..4 {
                let mut params = [0; 4];
                params[first] = random_nonzero(&mut rng);
                params[second] = random_nonzero(&mut rng);
                Pair::fresh().findrep(params);
            }
        }
    }

    // C26-C27: three and four active parameters.
    for _ in 0..CASES {
        let zero_position = rng.range(0, 3) as usize;
        let mut three = std::array::from_fn(|_| random_nonzero(&mut rng));
        three[zero_position] = 0;
        Pair::fresh().findrep(three);
        Pair::fresh().findrep(std::array::from_fn(|_| random_nonzero(&mut rng)));
    }

    // C28: one value from every normalization class.
    for _ in 0..CASES {
        let mut params = [
            rng.range(-700, -1),
            rng.range(1, 63),
            rng.range(64, 511),
            rng.range(512, 700),
        ];
        for index in (1..params.len()).rev() {
            let other = rng.range(0, index as i32) as usize;
            params.swap(index, other);
        }
        Pair::fresh().findrep(params);
    }

    // C29-C34: each static-state branch around its exact octal threshold.
    for _ in 0..CASES {
        Pair::fresh().findrep([
            rng.range(-60, -1),
            rng.range(-40, -1),
            rng.choose(&[-1, 0]),
            rng.choose(&[-1, 0]),
        ]);
        Pair::fresh().findrep([rng.range(1, 63), rng.range(1, 63), 0, 0]);
        Pair::fresh().findrep([rng.range(1, 63), rng.range(1, 63), 0, 0]);
        Pair::fresh().findrep([-1, -1, -1, -1]);
        Pair::fresh().findrep([-1, -1, rng.range(-8, -1), rng.range(-8, -1)]);
        Pair::fresh().findrep([1, 1, rng.range(1, 63), rng.range(1, 63)]);
    }

    // C35: low-level calls precondition all hidden state before findrep.
    for _ in 0..CASES {
        let pair = Pair::fresh();
        pair.add(rng.range(-50, 50), rng.range(-50, 50));
        pair.multiply(rng.range(-8, 8), rng.range(-8, 8));
        pair.subtract(rng.range(-50, 50), rng.range(-50, 50));
        pair.divide(rng.range(-100, 100), rng.choose(&[-4, -2, -1, 0, 1, 2, 4]));
        pair.findrep(std::array::from_fn(|_| rng.range(-8, 8)));
    }

    // C36: repeated mixed public calls exercise persistence end to end.
    for _ in 0..CASES {
        let pair = Pair::fresh();
        for _ in 0..4 {
            pair.findrep([
                rng.choose(&[-2, -1, 0]),
                rng.choose(&[-2, -1, 0]),
                rng.choose(&[-1, 0]),
                rng.choose(&[-1, 0]),
            ]);
            pair.add(rng.range(-5, 5), rng.range(-5, 5));
            pair.subtract(rng.range(-5, 5), rng.range(-5, 5));
        }
    }
}

#[test]
fn defined_dynamic_symbols_match() {
    fn symbols(path: &Path) -> BTreeSet<String> {
        let output = Command::new("nm")
            .args(["-D", "--defined-only"])
            .arg(path)
            .output()
            .unwrap();
        assert!(output.status.success(), "nm failed for {}", path.display());
        String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .filter_map(|line| line.split_whitespace().last())
            .map(str::to_owned)
            .collect()
    }

    let c_symbols = symbols(&c_library_path());
    let rust_symbols = symbols(&rust_library_path());
    assert_eq!(
        c_symbols, rust_symbols,
        "defined dynamic symbol sets differ"
    );
}

#[test]
fn generic_null_pointer_boundaries_match() {
    use std::os::unix::process::ExitStatusExt;

    for function in ["process_octal_string", "find_and_replace_char"] {
        let mut signals = Vec::new();
        for implementation in ["c", "rust"] {
            let status = Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "ffi_null_probe", "--ignored"])
                .env("NULL_PROBE_IMPLEMENTATION", implementation)
                .env("NULL_PROBE_FUNCTION", function)
                .status()
                .unwrap();
            assert!(
                !status.success(),
                "{implementation} {function}(NULL, _) unexpectedly returned"
            );
            signals.push(status.signal());
        }
        assert_eq!(
            signals[0], signals[1],
            "C and Rust fail differently for {function}(NULL, _)"
        );
        assert!(
            signals[0].is_some(),
            "{function} did not terminate by signal"
        );
    }
}

#[test]
#[ignore = "subprocess target for generic_null_pointer_boundaries_match"]
fn ffi_null_probe() {
    let implementation = std::env::var("NULL_PROBE_IMPLEMENTATION")
        .expect("NULL_PROBE_IMPLEMENTATION is set by parent");
    let function =
        std::env::var("NULL_PROBE_FUNCTION").expect("NULL_PROBE_FUNCTION is set by parent");
    let path = match implementation.as_str() {
        "c" => c_library_path(),
        "rust" => rust_library_path(),
        _ => panic!("unknown implementation"),
    };
    let library = unsafe { Library::new(path) }.unwrap();
    let call: libloading::Symbol<'_, ProcessFn> =
        unsafe { library.get(format!("{function}\0").as_bytes()) }.unwrap();
    unsafe { call(std::ptr::null_mut(), 7) };
}

// Make the pointer ABI explicit in this test crate as well.
const _: Option<*mut c_void> = None;
