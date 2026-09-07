use libloading::Library;
use std::ffi::{c_char, c_double, c_int, c_void};
use std::fs::File;
use std::io::Read;
use std::os::fd::FromRawFd;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

type ConvertDoubleToInt = unsafe extern "C" fn(c_double) -> c_int;
type FindValueInBuffer = unsafe extern "C" fn(*const c_char, usize, c_int) -> c_int;
type ProcessNegation = unsafe extern "C" fn(c_int) -> c_int;
type CreateNumericBuffer = unsafe extern "C" fn(*mut c_char, c_int, c_int);
type CalculateWithDoubles = unsafe extern "C" fn(c_int, c_int, c_int) -> c_double;
type Doubleneg = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

struct Api {
    _library: Library,
    convert_double_to_int: ConvertDoubleToInt,
    find_value_in_buffer: FindValueInBuffer,
    process_negation: ProcessNegation,
    create_numeric_buffer: CreateNumericBuffer,
    calculate_with_doubles: CalculateWithDoubles,
    doubleneg: Doubleneg,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let convert_double_to_int = unsafe {
            *library
                .get::<ConvertDoubleToInt>(b"convert_double_to_int\0")
                .unwrap()
        };
        let find_value_in_buffer = unsafe {
            *library
                .get::<FindValueInBuffer>(b"find_value_in_buffer\0")
                .unwrap()
        };
        let process_negation = unsafe {
            *library
                .get::<ProcessNegation>(b"process_negation\0")
                .unwrap()
        };
        let create_numeric_buffer = unsafe {
            *library
                .get::<CreateNumericBuffer>(b"create_numeric_buffer\0")
                .unwrap()
        };
        let calculate_with_doubles = unsafe {
            *library
                .get::<CalculateWithDoubles>(b"calculate_with_doubles\0")
                .unwrap()
        };
        let doubleneg = unsafe { *library.get::<Doubleneg>(b"doubleneg\0").unwrap() };

        Self {
            _library: library,
            convert_double_to_int,
            find_value_in_buffer,
            process_negation,
            create_numeric_buffer,
            calculate_with_doubles,
            doubleneg,
        }
    }
}

fn c_library_path() -> PathBuf {
    let build = Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/build");
    let mut candidates: Vec<_> = std::fs::read_dir(&build)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", build.display()))
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "so"))
        .collect();
    candidates.sort();
    assert_eq!(
        candidates.len(),
        1,
        "expected exactly one C shared library in {}",
        build.display()
    );
    candidates.pop().unwrap()
}

fn rust_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("target/release/libdoubleneg_lib.so")
}

fn apis() -> (Api, Api) {
    let c_path = c_library_path();
    let rust_path = rust_library_path();
    assert!(
        rust_path.is_file(),
        "build the Rust cdylib first: cargo build --release"
    );
    unsafe { (Api::load(&c_path), Api::load(&rust_path)) }
}

#[derive(Clone)]
struct FixedRng(u64);

impl FixedRng {
    fn new() -> Self {
        Self(0x8f3c_6a21_d947_b5e1)
    }

    fn next_u64(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value
    }

    fn next_i32(&mut self) -> i32 {
        self.next_u64() as i32
    }

    fn bounded_i32(&mut self, magnitude: i32) -> i32 {
        (self.next_u64() % (2 * magnitude as u64 + 1)) as i32 - magnitude
    }

    fn nonzero_bounded_i32(&mut self, magnitude: i32) -> i32 {
        loop {
            let value = self.bounded_i32(magnitude);
            if value != 0 {
                return value;
            }
        }
    }
}

fn assert_same_double(c_value: f64, rust_value: f64, context: &str) {
    assert_eq!(
        c_value.to_bits(),
        rust_value.to_bits(),
        "{context}: C={c_value:?} Rust={rust_value:?}"
    );
}

#[test]
fn phase_b_convert_double_to_int_rows_1_to_4() {
    let (c, rust) = apis();
    let mut values = vec![
        i32::MIN as f64,
        -1.0,
        -0.0,
        0.0,
        1.0,
        i32::MAX as f64,
        -12345.875,
        -0.999_999,
        0.999_999,
        12345.875,
        i32::MAX as f64 + 1.0,
        i32::MIN as f64 - 1.0,
        1.0e40,
        -1.0e40,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::from_bits(0x7ff8_0000_0000_0001),
        f64::from_bits(0xfff8_0000_0000_0001),
    ];
    let mut rng = FixedRng::new();
    for _ in 0..20_000 {
        let integer = rng.next_i32() as f64;
        let fraction = (rng.next_u64() % 1_000_000) as f64 / 1_000_001.0;
        values.push(if rng.next_u64() & 1 == 0 {
            integer + fraction
        } else {
            integer - fraction
        });
    }

    for value in values {
        let c_result = unsafe { (c.convert_double_to_int)(value) };
        let rust_result = unsafe { (rust.convert_double_to_int)(value) };
        assert_eq!(c_result, rust_result, "value={value:?}");
    }
}

#[test]
fn phase_b_find_value_in_buffer_rows_5_to_10() {
    let (c, rust) = apis();

    let cases: Vec<(Vec<i8>, usize, i32)> = vec![
        (vec![], 0, 0),
        (vec![42], 1, 42),
        (vec![42], 1, 7),
        (vec![1, 2, 3, 4], 4, 1),
        (vec![1, 2, 3, 4], 4, 3),
        (vec![1, 2, 3, 4], 4, 4),
        (vec![1, 2, 3, 4], 4, 9),
        (vec![-1, 0, 1], 3, 255),
        (vec![-1, 0, 1], 3, -1),
        (vec![-1, 0, 1], 3, 511),
    ];
    for (buffer, size, search) in cases {
        let c_result = unsafe { (c.find_value_in_buffer)(buffer.as_ptr(), size, search) };
        let rust_result = unsafe { (rust.find_value_in_buffer)(buffer.as_ptr(), size, search) };
        assert_eq!(c_result, rust_result, "size={size}, search={search}");
    }

    let mut rng = FixedRng::new();
    for _ in 0..10_000 {
        let length = (rng.next_u64() % 513) as usize;
        let mut buffer = Vec::with_capacity(length);
        for _ in 0..length {
            buffer.push(rng.next_u64() as i8);
        }
        let prefix = (rng.next_u64() % (length as u64 + 1)) as usize;
        let search = rng.next_i32();
        let c_result = unsafe { (c.find_value_in_buffer)(buffer.as_ptr(), prefix, search) };
        let rust_result = unsafe { (rust.find_value_in_buffer)(buffer.as_ptr(), prefix, search) };
        assert_eq!(
            c_result, rust_result,
            "length={length}, prefix={prefix}, search={search}"
        );
    }
}

#[test]
fn phase_b_process_negation_rows_11_to_13() {
    let (c, rust) = apis();
    let mut rng = FixedRng::new();
    let mut values = vec![i32::MIN, -1, 0, 1, i32::MAX];
    values.extend((0..20_000).map(|_| rng.next_i32()));
    for value in values {
        let c_result = unsafe { (c.process_negation)(value) };
        let rust_result = unsafe { (rust.process_negation)(value) };
        assert_eq!(c_result, rust_result, "value={value}");
    }
}

#[test]
fn phase_b_create_numeric_buffer_rows_14_to_17() {
    let (c, rust) = apis();

    for size in [-100, -1, 0] {
        let mut c_buffer = vec![0x5a_i8; 32];
        let mut rust_buffer = c_buffer.clone();
        unsafe {
            (c.create_numeric_buffer)(c_buffer.as_mut_ptr().add(8), size, 123);
            (rust.create_numeric_buffer)(rust_buffer.as_mut_ptr().add(8), size, 123);
        }
        assert_eq!(c_buffer, rust_buffer, "size={size}");
    }

    let mut rng = FixedRng::new();
    for _ in 0..10_000 {
        let size = 1 + (rng.next_u64() % 512) as i32;
        let seed = rng.bounded_i32(1_000_000);
        let mut c_buffer = vec![0x5a_i8; size as usize + 16];
        let mut rust_buffer = c_buffer.clone();
        unsafe {
            (c.create_numeric_buffer)(c_buffer.as_mut_ptr().add(8), size, seed);
            (rust.create_numeric_buffer)(rust_buffer.as_mut_ptr().add(8), size, seed);
        }
        assert_eq!(c_buffer, rust_buffer, "size={size}, seed={seed}");
    }
}

#[test]
fn phase_b_calculate_with_doubles_rows_18_to_37() {
    let (c, rust) = apis();

    for exponent_residue in -9..=9 {
        for c_value in [
            exponent_residue,
            exponent_residue + 100,
            exponent_residue - 100,
        ] {
            let c_result = unsafe { (c.calculate_with_doubles)(123, 0, c_value) };
            let rust_result = unsafe { (rust.calculate_with_doubles)(123, 0, c_value) };
            assert_same_double(c_result, rust_result, "b=0");
        }
    }

    let mut rng = FixedRng::new();
    for exponent_residue in -9..=9 {
        for _ in 0..2_000 {
            let a = rng.next_i32();
            let b = loop {
                let value = rng.next_i32();
                if value != 0 {
                    break value;
                }
            };
            let multiple = (rng.next_u64() % 100_000) as i32;
            let c_value = if exponent_residue < 0 {
                exponent_residue - 10 * multiple
            } else {
                exponent_residue + 10 * multiple
            };
            let c_result = unsafe { (c.calculate_with_doubles)(a, b, c_value) };
            let rust_result = unsafe { (rust.calculate_with_doubles)(a, b, c_value) };
            assert_same_double(c_result, rust_result, &format!("a={a}, b={b}, c={c_value}"));
        }
    }
}

unsafe extern "C" {
    #[link_name = "pipe"]
    fn c_pipe(fds: *mut c_int) -> c_int;
    #[link_name = "dup"]
    fn c_dup(fd: c_int) -> c_int;
    #[link_name = "dup2"]
    fn c_dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    #[link_name = "close"]
    fn c_close(fd: c_int) -> c_int;
    #[link_name = "fflush"]
    fn c_fflush(stream: *mut c_void) -> c_int;
    #[link_name = "fork"]
    fn c_fork() -> c_int;
    #[link_name = "waitpid"]
    fn c_waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    #[link_name = "_exit"]
    fn c_exit(status: c_int) -> !;
}

static STDOUT_LOCK: Mutex<()> = Mutex::new(());

fn capture_c_stdout<T>(operation: impl FnOnce() -> T) -> (T, Vec<u8>) {
    let _lock = STDOUT_LOCK.lock().unwrap();
    let mut fds = [-1, -1];
    assert_eq!(unsafe { c_pipe(fds.as_mut_ptr()) }, 0);
    assert_eq!(unsafe { c_fflush(std::ptr::null_mut()) }, 0);
    let saved_stdout = unsafe { c_dup(1) };
    assert!(saved_stdout >= 0);
    assert_eq!(unsafe { c_dup2(fds[1], 1) }, 1);
    assert_eq!(unsafe { c_close(fds[1]) }, 0);

    let reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let mut file = unsafe { File::from_raw_fd(fds[0]) };
        file.read_to_end(&mut bytes).unwrap();
        bytes
    });

    let result = operation();
    assert_eq!(unsafe { c_fflush(std::ptr::null_mut()) }, 0);
    assert_eq!(unsafe { c_dup2(saved_stdout, 1) }, 1);
    assert_eq!(unsafe { c_close(saved_stdout) }, 0);
    let bytes = reader.join().unwrap();
    (result, bytes)
}

fn nonzero_with_residue(rng: &mut FixedRng, residue: i32) -> i32 {
    let multiple = 1 + (rng.next_u64() % 1_000) as i32;
    match residue.cmp(&0) {
        std::cmp::Ordering::Less => residue - 10 * multiple,
        std::cmp::Ordering::Equal => {
            if rng.next_u64() & 1 == 0 {
                10 * multiple
            } else {
                -10 * multiple
            }
        }
        std::cmp::Ordering::Greater => residue + 10 * multiple,
    }
}

#[test]
fn phase_b_doubleneg_rows_38_to_53_including_stdout() {
    let (c, rust) = apis();
    let mut rng = FixedRng::new();
    let mut cases = Vec::new();

    for mask in 0_u8..16 {
        let repetitions = if mask == 0 { 1 } else { 64 };
        for iteration in 0..repetitions {
            let residue = -9 + iteration as i32 % 19;
            let p1 = if mask & 0b1000 == 0 {
                0
            } else {
                rng.nonzero_bounded_i32(10_000)
            };
            let p2 = if mask & 0b0100 == 0 {
                0
            } else {
                rng.nonzero_bounded_i32(10_000)
            };
            let p3 = if mask & 0b0010 == 0 {
                0
            } else {
                nonzero_with_residue(&mut rng, residue)
            };
            let p4 = if mask & 0b0001 == 0 {
                0
            } else {
                rng.nonzero_bounded_i32(10_000)
            };
            cases.push((p1, p2, p3, p4));
        }
    }

    let (c_results, c_stdout) = capture_c_stdout(|| {
        cases
            .iter()
            .map(|&(a, b, c_value, d)| unsafe { (c.doubleneg)(a, b, c_value, d) })
            .collect::<Vec<_>>()
    });
    let (rust_results, rust_stdout) = capture_c_stdout(|| {
        cases
            .iter()
            .map(|&(a, b, c_value, d)| unsafe { (rust.doubleneg)(a, b, c_value, d) })
            .collect::<Vec<_>>()
    });

    assert_eq!(c_results, rust_results);
    assert_eq!(c_stdout, rust_stdout, "doubleneg stdout differs");
}

fn child_signal(operation: impl FnOnce()) -> c_int {
    let pid = unsafe { c_fork() };
    assert!(pid >= 0);
    if pid == 0 {
        operation();
        unsafe { c_exit(0) };
    }
    let mut status = 0;
    assert_eq!(unsafe { c_waitpid(pid, &mut status, 0) }, pid);
    status & 0x7f
}

#[test]
fn phase_c_error_row_1_and_generic_ffi_boundaries() {
    let (c, rust) = apis();

    let buffer = [1_i8, 2, 3, 4];
    let c_absent = unsafe { (c.find_value_in_buffer)(buffer.as_ptr(), buffer.len(), 99) };
    let rust_absent = unsafe { (rust.find_value_in_buffer)(buffer.as_ptr(), buffer.len(), 99) };
    assert_eq!(c_absent, -1);
    assert_eq!(c_absent, rust_absent);

    let c_null_zero = unsafe { (c.find_value_in_buffer)(std::ptr::null(), 0, 1) };
    let rust_null_zero = unsafe { (rust.find_value_in_buffer)(std::ptr::null(), 0, 1) };
    assert_eq!(c_null_zero, rust_null_zero);

    unsafe {
        (c.create_numeric_buffer)(std::ptr::null_mut(), 0, 1);
        (rust.create_numeric_buffer)(std::ptr::null_mut(), 0, 1);
        (c.create_numeric_buffer)(std::ptr::null_mut(), -1, 1);
        (rust.create_numeric_buffer)(std::ptr::null_mut(), -1, 1);
    }

    let c_find_signal = child_signal(|| unsafe {
        (c.find_value_in_buffer)(std::ptr::null(), 1, 1);
    });
    let rust_find_signal = child_signal(|| unsafe {
        (rust.find_value_in_buffer)(std::ptr::null(), 1, 1);
    });
    assert_ne!(c_find_signal, 0);
    assert_eq!(c_find_signal, rust_find_signal);

    let c_create_signal =
        child_signal(|| unsafe { (c.create_numeric_buffer)(std::ptr::null_mut(), 1, 1) });
    let rust_create_signal =
        child_signal(|| unsafe { (rust.create_numeric_buffer)(std::ptr::null_mut(), 1, 1) });
    assert_ne!(c_create_signal, 0);
    assert_eq!(c_create_signal, rust_create_signal);

    let oversized = 1_048_577_usize;
    let find_buffer = vec![0_i8; oversized];
    let c_large_find = unsafe { (c.find_value_in_buffer)(find_buffer.as_ptr(), oversized, 1) };
    let rust_large_find =
        unsafe { (rust.find_value_in_buffer)(find_buffer.as_ptr(), oversized, 1) };
    assert_eq!(c_large_find, rust_large_find);

    let mut c_large_create = vec![0_i8; oversized];
    let mut rust_large_create = c_large_create.clone();
    unsafe {
        (c.create_numeric_buffer)(c_large_create.as_mut_ptr(), oversized as i32, -12345);
        (rust.create_numeric_buffer)(rust_large_create.as_mut_ptr(), oversized as i32, -12345);
    }
    assert_eq!(c_large_create, rust_large_create);

    for value in [i32::MIN as f64 - 1.0, i32::MAX as f64 + 1.0] {
        let c_result = unsafe { (c.convert_double_to_int)(value) };
        let rust_result = unsafe { (rust.convert_double_to_int)(value) };
        assert_eq!(c_result, rust_result, "out-of-range value={value}");
    }
}
