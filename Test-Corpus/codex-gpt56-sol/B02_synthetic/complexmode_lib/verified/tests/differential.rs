use libloading::Library;
use std::env;
use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::fs::{self, OpenOptions};
use std::io::{Read, Seek, SeekFrom};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

type CreateResultString = unsafe extern "C" fn(*const c_char, c_int) -> *mut c_char;
type CheckPermissions = unsafe extern "C" fn(c_int, c_int) -> c_int;
type SafeAdd = unsafe extern "C" fn(c_int, c_int, c_int) -> c_int;
type MultiplyWithLog = unsafe extern "C" fn(c_int, c_int, *mut *mut c_char) -> c_int;
type CopyAndSum = unsafe extern "C" fn(*mut c_int, c_int) -> c_int;
type CompareOperations = unsafe extern "C" fn(*const c_char, *const c_char) -> c_int;
type Complexmode = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

struct Api {
    _library: Library,
    create_result_string: CreateResultString,
    check_permissions: CheckPermissions,
    safe_add: SafeAdd,
    multiply_with_log: MultiplyWithLog,
    copy_and_sum: CopyAndSum,
    compare_operations: CompareOperations,
    complexmode: Complexmode,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let create_result_string = unsafe {
            *library
                .get::<CreateResultString>(b"create_result_string\0")
                .unwrap()
        };
        let check_permissions = unsafe {
            *library
                .get::<CheckPermissions>(b"check_permissions\0")
                .unwrap()
        };
        let safe_add = unsafe { *library.get::<SafeAdd>(b"safe_add\0").unwrap() };
        let multiply_with_log = unsafe {
            *library
                .get::<MultiplyWithLog>(b"multiply_with_log\0")
                .unwrap()
        };
        let copy_and_sum = unsafe { *library.get::<CopyAndSum>(b"copy_and_sum\0").unwrap() };
        let compare_operations = unsafe {
            *library
                .get::<CompareOperations>(b"compare_operations\0")
                .unwrap()
        };
        let complexmode = unsafe { *library.get::<Complexmode>(b"complexmode\0").unwrap() };

        Self {
            _library: library,
            create_result_string,
            check_permissions,
            safe_add,
            multiply_with_log,
            copy_and_sum,
            compare_operations,
            complexmode,
        }
    }
}

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn free(pointer: *mut c_void);
}

const STDOUT_FILENO: c_int = 1;
static STDOUT_LOCK: Mutex<()> = Mutex::new(());
static CAPTURE_ID: AtomicU64 = AtomicU64::new(0);

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    manifest_dir().join("c_src/build/libtranslated_rust.so")
}

fn rust_library_path() -> PathBuf {
    let target = env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| manifest_dir().join("target"));
    let target = if target.is_absolute() {
        target
    } else {
        manifest_dir().join(target)
    };
    let preferred_profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    for profile in [preferred_profile, "release", "debug"] {
        let direct = target.join(profile).join("libcomplexmode_lib.so");
        if direct.exists() {
            return direct;
        }

        let deps = target.join(profile).join("deps");
        if let Ok(entries) = fs::read_dir(deps) {
            let mut candidates: Vec<_> = entries
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .filter(|path| {
                    path.file_name()
                        .and_then(|name| name.to_str())
                        .is_some_and(|name| {
                            name.starts_with("libcomplexmode_lib") && name.ends_with(".so")
                        })
                })
                .collect();
            candidates.sort();
            if let Some(candidate) = candidates.pop() {
                return candidate;
            }
        }
    }
    panic!(
        "Rust cdylib was not produced under {}; run cargo build first",
        target.display()
    );
}

fn load_pair() -> (Api, Api) {
    assert!(
        c_library_path().exists(),
        "build the C shared object before running tests"
    );
    unsafe {
        (
            Api::load(&c_library_path()),
            Api::load(&rust_library_path()),
        )
    }
}

fn capture_stdout<T>(operation: impl FnOnce() -> T) -> (T, Vec<u8>) {
    let _lock = STDOUT_LOCK.lock().unwrap();
    let id = CAPTURE_ID.fetch_add(1, Ordering::Relaxed);
    let path = env::temp_dir().join(format!(
        "translated-rust-capture-{}-{id}",
        std::process::id()
    ));
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .read(true)
        .write(true)
        .open(&path)
        .unwrap();

    unsafe {
        assert_eq!(fflush(ptr::null_mut()), 0);
    }
    let saved_stdout = unsafe { dup(STDOUT_FILENO) };
    assert!(saved_stdout >= 0);
    assert_eq!(
        unsafe { dup2(file.as_raw_fd(), STDOUT_FILENO) },
        STDOUT_FILENO
    );

    let result = operation();

    unsafe {
        assert_eq!(fflush(ptr::null_mut()), 0);
        assert_eq!(dup2(saved_stdout, STDOUT_FILENO), STDOUT_FILENO);
        assert_eq!(close(saved_stdout), 0);
    }
    file.seek(SeekFrom::Start(0)).unwrap();
    let mut output = Vec::new();
    file.read_to_end(&mut output).unwrap();
    drop(file);
    fs::remove_file(path).unwrap();
    (result, output)
}

unsafe fn take_c_string(pointer: *mut c_char) -> Option<Vec<u8>> {
    if pointer.is_null() {
        return None;
    }
    let bytes = unsafe { CStr::from_ptr(pointer) }
        .to_bytes_with_nul()
        .to_vec();
    unsafe {
        free(pointer.cast());
    }
    Some(bytes)
}

#[derive(Clone)]
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
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

    fn ascii_string(&mut self, length: usize) -> CString {
        let bytes: Vec<u8> = (0..length)
            .map(|_| b'!' + (self.next_u64() % 94) as u8)
            .collect();
        CString::new(bytes).unwrap()
    }
}

fn invoke_created(api: &Api, operation: *const c_char, value: i32) -> Option<Vec<u8>> {
    unsafe { take_c_string((api.create_result_string)(operation, value)) }
}

fn invoke_multiply(api: &Api, a: i32, b: i32) -> (i32, Option<Vec<u8>>) {
    let mut log = ptr::null_mut();
    let result = unsafe { (api.multiply_with_log)(a, b, &mut log) };
    (result, unsafe { take_c_string(log) })
}

#[test]
fn symbols_and_configs_01_03_create_result_string() {
    let (c, rust) = load_pair();
    let mut rng = Rng::new(0x8c3c_0103_d15c_a11e);
    let boundaries = [i32::MIN, -1, 0, 1, i32::MAX];

    let empty = CString::new("").unwrap();
    for value in boundaries
        .into_iter()
        .chain((0..128).map(|_| rng.next_i32()))
    {
        assert_eq!(
            invoke_created(&c, empty.as_ptr(), value),
            invoke_created(&rust, empty.as_ptr(), value)
        );
    }

    for _ in 0..128 {
        let length = 1 + (rng.next_u64() % 20) as usize;
        let operation = rng.ascii_string(length);
        let value = rng.next_i32();
        assert_eq!(
            invoke_created(&c, operation.as_ptr(), value),
            invoke_created(&rust, operation.as_ptr(), value)
        );
    }

    for _ in 0..128 {
        let length = 64 + (rng.next_u64() % 128) as usize;
        let operation = rng.ascii_string(length);
        let value = rng.next_i32();
        let c_value = invoke_created(&c, operation.as_ptr(), value).unwrap();
        let rust_value = invoke_created(&rust, operation.as_ptr(), value).unwrap();
        assert_eq!(c_value, rust_value);
        assert_eq!(c_value.len(), 64);
    }
}

#[test]
fn configs_04_07_permissions_and_safe_add() {
    let (c, rust) = load_pair();
    let mut rng = Rng::new(0x4304_0700_51de_cafe);

    for perms in [i32::MIN, -1, 0, 1, i32::MAX]
        .into_iter()
        .chain((0..128).map(|_| rng.next_i32()))
    {
        assert_eq!(unsafe { (c.check_permissions)(perms, 0) }, 1);
        assert_eq!(unsafe { (c.check_permissions)(perms, 0) }, unsafe {
            (rust.check_permissions)(perms, 0)
        });
    }

    for _ in 0..256 {
        let required = rng.next_i32();
        let perms = required | rng.next_i32();
        assert_eq!(unsafe { (c.check_permissions)(perms, required) }, unsafe {
            (rust.check_permissions)(perms, required)
        });

        let required = rng.next_i32() | 1;
        let missing_bit = 1_i32.wrapping_shl(required.trailing_zeros());
        let perms = required & !missing_bit;
        assert_eq!(unsafe { (c.check_permissions)(perms, required) }, unsafe {
            (rust.check_permissions)(perms, required)
        });
        assert_eq!(unsafe { (c.check_permissions)(perms, required) }, 0);
    }

    let mut cases = vec![
        (i32::MIN, -1, 0o600),
        (i32::MAX, 1, 0o600),
        (i32::MIN, i32::MIN, -1),
        (i32::MAX, i32::MAX, 0o1600),
    ];
    cases.extend((0..256).map(|_| (rng.next_i32(), rng.next_i32(), 0o600 | rng.next_i32())));
    for (a, b, perms) in cases {
        assert_eq!(unsafe { (c.safe_add)(a, b, perms) }, unsafe {
            (rust.safe_add)(a, b, perms)
        });
    }
}

#[test]
fn configs_08_09_multiply_with_log() {
    let (c, rust) = load_pair();
    let mut rng = Rng::new(0xbad5_eed0_0809_f00d);

    for _ in 0..256 {
        let a = (rng.next_u64() % 60_001) as i32 - 30_000;
        let b = (rng.next_u64() % 60_001) as i32 - 30_000;
        assert_eq!(invoke_multiply(&c, a, b), invoke_multiply(&rust, a, b));
    }

    let mut overflow_cases = vec![
        (i32::MAX, 2),
        (i32::MIN, -1),
        (i32::MIN, 2),
        (i32::MAX, i32::MAX),
    ];
    while overflow_cases.len() < 132 {
        let a = rng.next_i32();
        let b = rng.next_i32();
        let product = i64::from(a) * i64::from(b);
        if product < i64::from(i32::MIN) || product > i64::from(i32::MAX) {
            overflow_cases.push((a, b));
        }
    }
    for (a, b) in overflow_cases {
        assert_eq!(invoke_multiply(&c, a, b), invoke_multiply(&rust, a, b));
    }
}

#[test]
fn configs_10_13_copy_and_sum() {
    let (c, rust) = load_pair();
    let mut rng = Rng::new(0xc0de_1013_5a5a_1234);

    let mut empty_storage = [123];
    assert_eq!(
        unsafe { (c.copy_and_sum)(empty_storage.as_mut_ptr(), 0) },
        unsafe { (rust.copy_and_sum)(empty_storage.as_mut_ptr(), 0) }
    );

    for value in [i32::MIN, -1, 0, 1, i32::MAX]
        .into_iter()
        .chain((0..128).map(|_| rng.next_i32()))
    {
        let mut values = [value];
        assert_eq!(
            unsafe { (c.copy_and_sum)(values.as_mut_ptr(), 1) },
            unsafe { (rust.copy_and_sum)(values.as_mut_ptr(), 1) }
        );
    }

    for _ in 0..128 {
        let length = 2 + (rng.next_u64() % 31) as usize;
        let mut values: Vec<i32> = (0..length)
            .map(|_| (rng.next_u64() % 20_001) as i32 - 10_000)
            .collect();
        assert_eq!(
            unsafe { (c.copy_and_sum)(values.as_mut_ptr(), length as i32) },
            unsafe { (rust.copy_and_sum)(values.as_mut_ptr(), length as i32) }
        );
    }

    let mut overflow_cases = vec![
        vec![i32::MAX, 1],
        vec![i32::MIN, -1],
        vec![i32::MAX, i32::MAX, i32::MAX],
    ];
    while overflow_cases.len() < 131 {
        let mut values = vec![rng.next_i32(), rng.next_i32(), rng.next_i32()];
        let sum: i64 = values.iter().map(|&value| i64::from(value)).sum();
        if sum < i64::from(i32::MIN) || sum > i64::from(i32::MAX) {
            overflow_cases.push(std::mem::take(&mut values));
        }
    }
    for mut values in overflow_cases {
        let count = values.len() as i32;
        assert_eq!(
            unsafe { (c.copy_and_sum)(values.as_mut_ptr(), count) },
            unsafe { (rust.copy_and_sum)(values.as_mut_ptr(), count) }
        );
    }
}

#[test]
fn configs_14_17_compare_operations() {
    let (c, rust) = load_pair();
    let mut rng = Rng::new(0xc011_a7e0_1417_beef);
    let empty = CString::new("").unwrap();
    assert_eq!(
        unsafe { (c.compare_operations)(empty.as_ptr(), empty.as_ptr()) },
        unsafe { (rust.compare_operations)(empty.as_ptr(), empty.as_ptr()) }
    );

    for _ in 0..128 {
        let length = 1 + (rng.next_u64() % 64) as usize;
        let value = rng.ascii_string(length);
        assert_eq!(
            unsafe { (c.compare_operations)(value.as_ptr(), value.as_ptr()) },
            unsafe { (rust.compare_operations)(value.as_ptr(), value.as_ptr()) }
        );
    }

    for _ in 0..128 {
        let length = (rng.next_u64() % 32) as usize;
        let prefix = rng.ascii_string(length);
        let mut low = prefix.as_bytes().to_vec();
        let mut high = prefix.as_bytes().to_vec();
        low.push(b'A');
        high.push(b'Z');
        let low = CString::new(low).unwrap();
        let high = CString::new(high).unwrap();
        assert_eq!(
            unsafe { (c.compare_operations)(low.as_ptr(), high.as_ptr()) },
            unsafe { (rust.compare_operations)(low.as_ptr(), high.as_ptr()) }
        );
        assert_eq!(
            unsafe { (c.compare_operations)(high.as_ptr(), low.as_ptr()) },
            unsafe { (rust.compare_operations)(high.as_ptr(), low.as_ptr()) }
        );
    }
}

fn invoke_complex_batch(api: &Api, cases: &[(i32, i32, i32, i32)]) -> (Vec<i32>, Vec<u8>) {
    capture_stdout(|| {
        cases
            .iter()
            .map(|&(mode, a, b, c)| unsafe { (api.complexmode)(mode, a, b, c) })
            .collect()
    })
}

#[test]
fn configs_18_24_complexmode() {
    let (c, rust) = load_pair();
    let mut rng = Rng::new(0xc0de_1824_cafe_5eed);

    let mut mode_1 = vec![
        (1, i32::MIN, -1, 0),
        (1, i32::MAX, 1, 0),
        (1, i32::MAX, i32::MAX, 0),
    ];
    mode_1.extend((0..128).map(|_| (1, rng.next_i32(), rng.next_i32(), rng.next_i32())));
    assert_eq!(
        invoke_complex_batch(&c, &mode_1),
        invoke_complex_batch(&rust, &mode_1)
    );

    let mode_2_regular: Vec<_> = (0..128)
        .map(|_| {
            (
                2,
                (rng.next_u64() % 60_001) as i32 - 30_000,
                (rng.next_u64() % 60_001) as i32 - 30_000,
                rng.next_i32(),
            )
        })
        .collect();
    assert_eq!(
        invoke_complex_batch(&c, &mode_2_regular),
        invoke_complex_batch(&rust, &mode_2_regular)
    );

    let mut mode_2_overflow = vec![
        (2, i32::MAX, 2, 0),
        (2, i32::MIN, -1, 0),
        (2, i32::MIN, 2, 0),
    ];
    while mode_2_overflow.len() < 131 {
        let a = rng.next_i32();
        let b = rng.next_i32();
        let product = i64::from(a) * i64::from(b);
        if product < i64::from(i32::MIN) || product > i64::from(i32::MAX) {
            mode_2_overflow.push((2, a, b, rng.next_i32()));
        }
    }
    assert_eq!(
        invoke_complex_batch(&c, &mode_2_overflow),
        invoke_complex_batch(&rust, &mode_2_overflow)
    );

    let mode_3_regular: Vec<_> = (0..128)
        .map(|_| {
            (
                3,
                (rng.next_u64() % 20_001) as i32 - 10_000,
                (rng.next_u64() % 20_001) as i32 - 10_000,
                (rng.next_u64() % 20_001) as i32 - 10_000,
            )
        })
        .collect();
    assert_eq!(
        invoke_complex_batch(&c, &mode_3_regular),
        invoke_complex_batch(&rust, &mode_3_regular)
    );

    let mut mode_3_overflow = vec![
        (3, i32::MAX, 1, 0),
        (3, i32::MIN, -1, 0),
        (3, i32::MAX, i32::MAX, i32::MAX),
    ];
    while mode_3_overflow.len() < 131 {
        let values = [rng.next_i32(), rng.next_i32(), rng.next_i32()];
        let sum: i64 = values.iter().map(|&value| i64::from(value)).sum();
        if sum < i64::from(i32::MIN) || sum > i64::from(i32::MAX) {
            mode_3_overflow.push((3, values[0], values[1], values[2]));
        }
    }
    assert_eq!(
        invoke_complex_batch(&c, &mode_3_overflow),
        invoke_complex_batch(&rust, &mode_3_overflow)
    );

    let mode_4_regular: Vec<_> = (0..128)
        .map(|_| {
            (
                4,
                (rng.next_u64() % 20_001) as i32 - 10_000,
                (rng.next_u64() % 20_001) as i32 - 10_000,
                (rng.next_u64() % 20_001) as i32 - 10_000,
            )
        })
        .collect();
    assert_eq!(
        invoke_complex_batch(&c, &mode_4_regular),
        invoke_complex_batch(&rust, &mode_4_regular)
    );

    let mut mode_4_overflow = vec![
        (4, i32::MAX, 1, 0),
        (4, i32::MIN, -1, 0),
        (4, i32::MAX, i32::MAX, i32::MAX),
    ];
    while mode_4_overflow.len() < 131 {
        let values = [rng.next_i32(), rng.next_i32(), rng.next_i32()];
        let sum: i64 = values.iter().map(|&value| i64::from(value)).sum();
        if sum < i64::from(i32::MIN) || sum > i64::from(i32::MAX) {
            mode_4_overflow.push((4, values[0], values[1], values[2]));
        }
    }
    assert_eq!(
        invoke_complex_batch(&c, &mode_4_overflow),
        invoke_complex_batch(&rust, &mode_4_overflow)
    );
}

#[test]
fn errors_02_04_06_10_and_generic_boundaries() {
    let (c, rust) = load_pair();
    let mut rng = Rng::new(0xe220_4061_0bad_f00d);

    let denied: Vec<_> = (0..128)
        .map(|_| {
            let perms = rng.next_i32() & !0o200;
            (rng.next_i32(), rng.next_i32(), perms)
        })
        .collect();
    let c_denied = capture_stdout(|| {
        denied
            .iter()
            .map(|&(a, b, perms)| unsafe { (c.safe_add)(a, b, perms) })
            .collect::<Vec<_>>()
    });
    let rust_denied = capture_stdout(|| {
        denied
            .iter()
            .map(|&(a, b, perms)| unsafe { (rust.safe_add)(a, b, perms) })
            .collect::<Vec<_>>()
    });
    assert_eq!(c_denied, rust_denied);
    assert!(c_denied.0.iter().all(|&result| result == 0));

    let null_counts = [i32::MIN, -1, 0, 1, i32::MAX];
    let c_null_source = capture_stdout(|| {
        null_counts
            .iter()
            .map(|&count| unsafe { (c.copy_and_sum)(ptr::null_mut(), count) })
            .collect::<Vec<_>>()
    });
    let rust_null_source = capture_stdout(|| {
        null_counts
            .iter()
            .map(|&count| unsafe { (rust.copy_and_sum)(ptr::null_mut(), count) })
            .collect::<Vec<_>>()
    });
    assert_eq!(c_null_source, rust_null_source);
    assert!(c_null_source.0.iter().all(|&result| result == -1));

    let valid = CString::new("operation").unwrap();
    let null_pairs = [
        (ptr::null(), valid.as_ptr()),
        (valid.as_ptr(), ptr::null()),
        (ptr::null(), ptr::null()),
    ];
    let c_null_operations = capture_stdout(|| {
        null_pairs
            .iter()
            .map(|&(left, right)| unsafe { (c.compare_operations)(left, right) })
            .collect::<Vec<_>>()
    });
    let rust_null_operations = capture_stdout(|| {
        null_pairs
            .iter()
            .map(|&(left, right)| unsafe { (rust.compare_operations)(left, right) })
            .collect::<Vec<_>>()
    });
    assert_eq!(c_null_operations, rust_null_operations);
    assert!(c_null_operations.0.iter().all(|&result| result == -1));

    let mut invalid_modes = vec![i32::MIN, -1, 0, 5, i32::MAX];
    while invalid_modes.len() < 133 {
        let mode = rng.next_i32();
        if !(1..=4).contains(&mode) {
            invalid_modes.push(mode);
        }
    }
    let invalid_cases: Vec<_> = invalid_modes
        .into_iter()
        .map(|mode| (mode, rng.next_i32(), rng.next_i32(), rng.next_i32()))
        .collect();
    let c_invalid = invoke_complex_batch(&c, &invalid_cases);
    let rust_invalid = invoke_complex_batch(&rust, &invalid_cases);
    assert_eq!(c_invalid, rust_invalid);
    assert!(c_invalid.0.iter().all(|&result| result == -1));

    let c_null_string = invoke_created(&c, ptr::null(), i32::MIN);
    let rust_null_string = invoke_created(&rust, ptr::null(), i32::MIN);
    assert_eq!(c_null_string, rust_null_string);

    let mut source = [1_i32];
    for count in [i32::MIN, -1] {
        let c_oversized =
            capture_stdout(|| unsafe { (c.copy_and_sum)(source.as_mut_ptr(), count) });
        let rust_oversized =
            capture_stdout(|| unsafe { (rust.copy_and_sum)(source.as_mut_ptr(), count) });
        assert_eq!(c_oversized, rust_oversized);
        assert_eq!(c_oversized.0, -1);
    }
}

fn target_dir() -> PathBuf {
    let target = env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| manifest_dir().join("target"));
    if target.is_absolute() {
        target
    } else {
        manifest_dir().join(target)
    }
}

fn build_fault_shim() -> PathBuf {
    let support = target_dir().join("test-support");
    fs::create_dir_all(&support).unwrap();
    let output = support.join("libfault_inject.so");
    let status = Command::new("cc")
        .args(["-shared", "-fPIC"])
        .arg(manifest_dir().join("tests/fault_inject.c"))
        .args(["-o"])
        .arg(&output)
        .arg("-ldl")
        .status()
        .unwrap();
    assert!(status.success(), "failed to compile allocator fault shim");
    assert!(output.exists());
    output
}

fn hex(bytes: &[u8]) -> String {
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write;
        write!(&mut encoded, "{byte:02x}").unwrap();
    }
    encoded
}

fn run_fault_child(case: &str, target: &str, shim: &Path) -> std::process::Output {
    let preload = match env::var_os("LD_PRELOAD") {
        Some(existing) if !existing.is_empty() => {
            let mut value = shim.as_os_str().to_os_string();
            value.push(":");
            value.push(existing);
            value
        }
        _ => shim.as_os_str().to_os_string(),
    };
    Command::new(env::current_exe().unwrap())
        .arg("errors_01_03_05_07_09_fault_injection_and_generic_g2")
        .args(["--exact", "--nocapture", "--test-threads=1"])
        .env("LD_PRELOAD", preload)
        .env("DIFF_CHILD_CASE", case)
        .env("DIFF_CHILD_TARGET", target)
        .env("DIFF_SHIM", shim)
        .output()
        .unwrap()
}

fn child_result(output: &std::process::Output) -> String {
    assert!(
        output.status.success(),
        "fault child failed: status={:?}\nstdout={}\nstderr={}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .find_map(|line| {
            line.find("DIFF_RESULT:")
                .map(|index| &line[index + "DIFF_RESULT:".len()..])
        })
        .unwrap_or_else(|| {
            panic!(
                "fault child emitted no result:\n{}",
                String::from_utf8_lossy(&output.stdout)
            )
        })
        .to_owned()
}

fn run_fault_child_body(case: &str, target: &str) {
    type FailMalloc = unsafe extern "C" fn(usize);
    type EmptySnprintf = unsafe extern "C" fn();

    let shim = unsafe { Library::new(env::var_os("DIFF_SHIM").unwrap()) }.unwrap();
    let fail_malloc = unsafe {
        *shim
            .get::<FailMalloc>(b"ffi_fault_fail_next_malloc\0")
            .unwrap()
    };
    let empty_snprintf = unsafe {
        *shim
            .get::<EmptySnprintf>(b"ffi_fault_empty_next_snprintf\0")
            .unwrap()
    };
    let library_path = if target == "c" {
        c_library_path()
    } else {
        rust_library_path()
    };
    let api = unsafe { Api::load(&library_path) };

    if case == "null_log" {
        unsafe {
            (api.multiply_with_log)(7, 9, ptr::null_mut());
        }
        unreachable!("null output pointer unexpectedly returned");
    }

    let (result, output) = capture_stdout(|| match case {
        "create_alloc" => {
            unsafe { fail_malloc(64) };
            let operation = CString::new("x").unwrap();
            let pointer = unsafe { (api.create_result_string)(operation.as_ptr(), 7) };
            format!("null={}", pointer.is_null())
        }
        "multiply_alloc" => {
            unsafe { fail_malloc(64) };
            let mut log = 1_usize as *mut c_char;
            let value = unsafe { (api.multiply_with_log)(7, 9, &mut log) };
            format!("value={value},null={}", log.is_null())
        }
        "copy_alloc" => {
            unsafe { fail_malloc(28) };
            let mut values = [1, 2, 3, 4, 5, 6, 7];
            let value = unsafe { (api.copy_and_sum)(values.as_mut_ptr(), 7) };
            format!("value={value}")
        }
        "complex_tracker_alloc" => {
            unsafe { fail_malloc(40) };
            let value = unsafe { (api.complexmode)(1, 7, 9, 11) };
            format!("value={value}")
        }
        "complex_log_alloc" => {
            unsafe { fail_malloc(64) };
            let value = unsafe { (api.complexmode)(2, 7, 9, 11) };
            format!("value={value}")
        }
        "complex_empty_log" => {
            unsafe { empty_snprintf() };
            let value = unsafe { (api.complexmode)(2, 7, 9, 11) };
            format!("value={value}")
        }
        _ => panic!("unknown child case {case}"),
    });
    println!("DIFF_RESULT:{result}:stdout={}", hex(&output));
}

#[test]
fn errors_01_03_05_07_09_fault_injection_and_generic_g2() {
    if let (Ok(case), Ok(target)) = (env::var("DIFF_CHILD_CASE"), env::var("DIFF_CHILD_TARGET")) {
        run_fault_child_body(&case, &target);
        return;
    }

    let shim = build_fault_shim();
    for case in [
        "create_alloc",
        "multiply_alloc",
        "copy_alloc",
        "complex_tracker_alloc",
        "complex_log_alloc",
        "complex_empty_log",
    ] {
        let c = run_fault_child(case, "c", &shim);
        let rust = run_fault_child(case, "rust", &shim);
        assert_eq!(child_result(&c), child_result(&rust), "case {case}");
    }

    let c_crash = run_fault_child("null_log", "c", &shim);
    let rust_crash = run_fault_child("null_log", "rust", &shim);
    assert!(!c_crash.status.success());
    assert!(!rust_crash.status.success());
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        assert_eq!(c_crash.status.signal(), rust_crash.status.signal());
    }
}
