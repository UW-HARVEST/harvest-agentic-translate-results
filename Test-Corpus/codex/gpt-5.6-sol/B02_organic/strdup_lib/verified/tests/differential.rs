use libloading::Library;
use std::ffi::{CStr, c_char, c_void};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr;

type CustomStrdup = unsafe extern "C" fn(*const c_char) -> *mut c_char;
type Free = unsafe extern "C" fn(*mut c_void);

struct Driver {
    _library: Library,
    custom_strdup: CustomStrdup,
}

impl Driver {
    fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let custom_strdup = unsafe {
            *library
                .get::<CustomStrdup>(b"custom_strdup\0")
                .unwrap_or_else(|error| {
                    panic!(
                        "failed to load custom_strdup from {}: {error}",
                        path.display()
                    )
                })
        };
        Self {
            _library: library,
            custom_strdup,
        }
    }

    unsafe fn call_raw(&self, input: *const c_char) -> *mut c_char {
        unsafe { (self.custom_strdup)(input) }
    }
}

struct Allocator {
    _library: Library,
    free: Free,
}

impl Allocator {
    fn load() -> Self {
        let library = unsafe { Library::new("libc.so.6") }.expect("failed to load libc.so.6");
        let free = unsafe {
            *library
                .get::<Free>(b"free\0")
                .expect("failed to load free from libc.so.6")
        };
        Self {
            _library: library,
            free,
        }
    }

    unsafe fn free(&self, pointer: *mut c_char) {
        unsafe { (self.free)(pointer.cast()) };
    }
}

struct Apis {
    c: Driver,
    rust: Driver,
    allocator: Allocator,
}

impl Apis {
    fn load() -> Self {
        Self {
            c: Driver::load(&manifest_dir().join("c_src/build/libdriver.so")),
            rust: Driver::load(&rust_library_path()),
            allocator: Allocator::load(),
        }
    }

    unsafe fn duplicate_bytes(&self, driver: &Driver, input: *const c_char) -> Option<Vec<u8>> {
        let output = unsafe { driver.call_raw(input) };
        if output.is_null() {
            return None;
        }

        let bytes = unsafe { CStr::from_ptr(output) }
            .to_bytes_with_nul()
            .to_vec();
        unsafe { self.allocator.free(output) };
        Some(bytes)
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn rust_library_path() -> PathBuf {
    if let Some(path) = std::env::var_os("RUST_DRIVER_SO") {
        return PathBuf::from(path);
    }

    let target = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| manifest_dir().join("target"));
    let target = if target.is_absolute() {
        target
    } else {
        manifest_dir().join(target)
    };
    target.join("release").join("libdriver.so")
}

fn assert_case(apis: &Apis, storage: &[u8], expected: &[u8]) {
    assert_eq!(
        storage.last(),
        Some(&0),
        "test input must be NUL-terminated"
    );
    let input = storage.as_ptr().cast::<c_char>();
    let c = unsafe { apis.duplicate_bytes(&apis.c, input) };
    let rust = unsafe { apis.duplicate_bytes(&apis.rust, input) };
    assert_eq!(rust, c, "C and Rust output differ for input {storage:?}");
    assert_eq!(
        c.as_deref(),
        Some(expected),
        "C output did not match its input"
    );
}

struct FixedRng(u64);

impl FixedRng {
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

    fn length(&mut self, minimum: usize, maximum: usize) -> usize {
        minimum + self.next_u64() as usize % (maximum - minimum + 1)
    }

    fn non_nul_byte(&mut self) -> u8 {
        (self.next_u64() % 255 + 1) as u8
    }
}

#[test]
fn configs_row_1_empty_string() {
    let apis = Apis::load();
    let mut rng = FixedRng::new(0x8d26_504c_9b7a_31e1);

    for _ in 0..512 {
        let trailing_length = rng.length(0, 256);
        let mut storage = Vec::with_capacity(trailing_length + 2);
        storage.push(0);
        for _ in 0..trailing_length {
            storage.push(rng.next_u64() as u8);
        }
        storage.push(0);
        assert_case(&apis, &storage, &[0]);
    }
}

#[test]
fn configs_row_2_one_byte_string() {
    let apis = Apis::load();
    let mut rng = FixedRng::new(0xd079_a223_5fcd_b8e6);

    for _ in 0..1024 {
        let byte = rng.non_nul_byte();
        assert_case(&apis, &[byte, 0], &[byte, 0]);
    }
}

#[test]
fn configs_row_3_many_byte_strings() {
    let apis = Apis::load();
    let mut rng = FixedRng::new(0x6a09_e667_f3bc_c909);

    for _ in 0..512 {
        let length = rng.length(2, 4096);
        let mut storage = Vec::with_capacity(length + 1);
        for _ in 0..length {
            storage.push(rng.non_nul_byte());
        }
        storage.push(0);
        assert_case(&apis, &storage, &storage);
    }
}

#[test]
fn configs_row_4_early_nul_with_trailing_bytes() {
    let apis = Apis::load();
    let mut rng = FixedRng::new(0xbb67_ae85_84ca_a73b);

    for _ in 0..512 {
        let visible_length = rng.length(1, 64);
        let trailing_length = rng.length(1, 256);
        let mut storage = Vec::with_capacity(visible_length + trailing_length + 2);
        for _ in 0..visible_length {
            storage.push(rng.non_nul_byte());
        }
        storage.push(0);
        let expected = storage.clone();
        for _ in 0..trailing_length {
            storage.push(rng.next_u64() as u8);
        }
        storage.push(0);
        assert_case(&apis, &storage, &expected);
    }
}

#[test]
fn errors_row_1_null_input() {
    let apis = Apis::load();
    let c = unsafe { apis.c.call_raw(ptr::null()) };
    let rust = unsafe { apis.rust.call_raw(ptr::null()) };
    assert!(c.is_null(), "C must return NULL for a NULL input");
    assert_eq!(rust, c, "Rust must return the same NULL sentinel as C");
}

#[repr(C)]
#[derive(Clone, Copy)]
struct RLimit {
    current: u64,
    maximum: u64,
}

unsafe extern "C" {
    fn getrlimit(resource: i32, limit: *mut RLimit) -> i32;
    fn setrlimit(resource: i32, limit: *const RLimit) -> i32;
}

const RLIMIT_AS: i32 = 9;
const MALLOC_FAILURE_CHILD: &str = "DRIVER_MALLOC_FAILURE_CHILD";

fn virtual_memory_bytes() -> u64 {
    let status =
        std::fs::read_to_string("/proc/self/status").expect("failed to read process status");
    let kibibytes = status
        .lines()
        .find_map(|line| line.strip_prefix("VmSize:"))
        .and_then(|value| value.split_ascii_whitespace().next())
        .and_then(|value| value.parse::<u64>().ok())
        .expect("VmSize was absent from /proc/self/status");
    kibibytes * 1024
}

fn run_malloc_failure_child() {
    let apis = Apis::load();

    // Resolve lazy PLT entries before restricting address-space growth.
    let empty = [0_u8];
    for driver in [&apis.c, &apis.rust] {
        let output = unsafe { driver.call_raw(empty.as_ptr().cast()) };
        assert!(!output.is_null(), "malloc preflight unexpectedly failed");
        unsafe { apis.allocator.free(output) };
    }

    let input_length = 64 * 1024 * 1024;
    let mut input = vec![b'x'; input_length + 1];
    input[input_length] = 0;

    let mut original = RLimit {
        current: 0,
        maximum: 0,
    };
    assert_eq!(
        unsafe { getrlimit(RLIMIT_AS, &mut original) },
        0,
        "getrlimit(RLIMIT_AS) failed"
    );
    let constrained = RLimit {
        current: virtual_memory_bytes(),
        maximum: original.maximum,
    };
    assert_eq!(
        unsafe { setrlimit(RLIMIT_AS, &constrained) },
        0,
        "setrlimit(RLIMIT_AS) failed"
    );

    let c = unsafe { apis.c.call_raw(input.as_ptr().cast()) };
    let rust = unsafe { apis.rust.call_raw(input.as_ptr().cast()) };

    let restore_result = unsafe { setrlimit(RLIMIT_AS, &original) };
    assert_eq!(restore_result, 0, "failed to restore RLIMIT_AS");
    assert!(c.is_null(), "C malloc-failure branch was not reached");
    assert_eq!(rust, c, "Rust must return the same NULL sentinel as C");
}

#[test]
fn errors_row_2_malloc_failure() {
    if std::env::var_os(MALLOC_FAILURE_CHILD).is_some() {
        run_malloc_failure_child();
        return;
    }

    let status = Command::new(std::env::current_exe().expect("failed to locate test executable"))
        .arg("--exact")
        .arg("errors_row_2_malloc_failure")
        .arg("--test-threads=1")
        .env(MALLOC_FAILURE_CHILD, "1")
        .status()
        .expect("failed to run isolated malloc-failure test");
    assert!(status.success(), "isolated malloc-failure test failed");
}
