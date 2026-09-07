use libloading::Library;
use std::ffi::{c_int, c_void};
use std::fs::{self, File};
use std::io::Read;
use std::os::fd::FromRawFd;
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::atomic::{AtomicU64, Ordering};

type StaticAlias = unsafe extern "C" fn(*mut c_int) -> *mut c_int;
type Driver = unsafe extern "C" fn(c_int, c_int);

unsafe extern "C" {
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(pipefd: *mut c_int) -> c_int;
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(status: c_int) -> !;
}

static COPY_ID: AtomicU64 = AtomicU64::new(0);
const STDOUT_FILENO: c_int = 1;
const SIGSEGV: c_int = 11;
const RANDOM_CASES: usize = 64;

struct Loaded {
    _library: Library,
    static_alias: StaticAlias,
    driver: Driver,
}

impl Loaded {
    unsafe fn open(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let static_alias = *unsafe { library.get::<StaticAlias>(b"static_alias\0") }
            .unwrap_or_else(|error| panic!("missing static_alias in {}: {error}", path.display()));
        let driver = *unsafe { library.get::<Driver>(b"driver\0") }
            .unwrap_or_else(|error| panic!("missing driver in {}: {error}", path.display()));
        Self {
            _library: library,
            static_alias,
            driver,
        }
    }
}

struct TempLibraries {
    directory: PathBuf,
}

impl Drop for TempLibraries {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.directory);
    }
}

struct Pair {
    c: Loaded,
    rust: Loaded,
    _temp: TempLibraries,
}

impl Pair {
    fn fresh() -> Self {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_source = manifest.join("../c_src/build/libStaticAlias.so");
        let rust_source = manifest.join("target/release/libStaticAlias.so");
        assert!(
            c_source.is_file(),
            "missing C library {}; build it with CMake first",
            c_source.display()
        );
        assert!(
            rust_source.is_file(),
            "missing release Rust library {}; run cargo build --release first",
            rust_source.display()
        );

        let id = COPY_ID.fetch_add(1, Ordering::Relaxed);
        let directory =
            std::env::temp_dir().join(format!("staticalias-diff-{}-{id}", std::process::id()));
        fs::create_dir(&directory)
            .unwrap_or_else(|error| panic!("failed to create {}: {error}", directory.display()));
        let c_copy = directory.join("libStaticAlias_c.so");
        let rust_copy = directory.join("libStaticAlias_rust.so");
        fs::copy(&c_source, &c_copy)
            .unwrap_or_else(|error| panic!("failed to copy {}: {error}", c_source.display()));
        fs::copy(&rust_source, &rust_copy)
            .unwrap_or_else(|error| panic!("failed to copy {}: {error}", rust_source.display()));

        let temp = TempLibraries { directory };
        let c = unsafe { Loaded::open(&c_copy) };
        let rust = unsafe { Loaded::open(&rust_copy) };
        Self {
            c,
            rust,
            _temp: temp,
        }
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

    fn range(&mut self, start: c_int, end_exclusive: c_int) -> c_int {
        assert!(start < end_exclusive);
        let width = (end_exclusive as i64 - start as i64) as u32;
        start + (self.next_u32() % width) as c_int
    }
}

fn compare_stack_call(pair: &Pair, input: c_int, row: usize) -> (bool, c_int) {
    let mut c_value = input;
    let mut rust_value = input;
    let c_input = ptr::addr_of_mut!(c_value);
    let rust_input = ptr::addr_of_mut!(rust_value);
    let c_returned = unsafe { (pair.c.static_alias)(c_input) };
    let rust_returned = unsafe { (pair.rust.static_alias)(rust_input) };
    let c_returned_value = unsafe { c_returned.read() };
    let rust_returned_value = unsafe { rust_returned.read() };

    assert_eq!(c_value, rust_value, "CONFIGS.md row {row}: caller value");
    assert_eq!(
        c_returned_value, rust_returned_value,
        "CONFIGS.md row {row}: returned pointee"
    );
    assert_eq!(
        c_returned == c_input,
        rust_returned == rust_input,
        "CONFIGS.md row {row}: returned pointer ownership"
    );
    (c_returned == c_input, c_returned_value)
}

fn precondition_inner(pair: &Pair, seed_value: c_int) -> c_int {
    assert!(seed_value >= 1);
    let (returned_caller_storage, returned_value) = compare_stack_call(pair, seed_value, 0);
    assert!(!returned_caller_storage);
    returned_value
}

fn capture_driver(driver: Driver, initial_value: c_int, iterations: c_int) -> Vec<u8> {
    unsafe {
        assert_eq!(fflush(ptr::null_mut()), 0);
        let mut fds = [-1, -1];
        assert_eq!(pipe(fds.as_mut_ptr()), 0, "pipe failed");
        let saved_stdout = dup(STDOUT_FILENO);
        assert!(saved_stdout >= 0, "dup failed");
        assert_eq!(dup2(fds[1], STDOUT_FILENO), STDOUT_FILENO, "dup2 failed");
        assert_eq!(close(fds[1]), 0, "close write fd failed");

        driver(initial_value, iterations);

        assert_eq!(fflush(ptr::null_mut()), 0);
        assert_eq!(
            dup2(saved_stdout, STDOUT_FILENO),
            STDOUT_FILENO,
            "restore stdout failed"
        );
        assert_eq!(close(saved_stdout), 0, "close saved stdout failed");

        let mut output = Vec::new();
        let mut read_end = File::from_raw_fd(fds[0]);
        read_end.read_to_end(&mut output).expect("read pipe");
        output
    }
}

fn compare_driver(pair: &Pair, initial_value: c_int, iterations: c_int, row: usize) {
    let c_output = capture_driver(pair.c.driver, initial_value, iterations);
    let rust_output = capture_driver(pair.rust.driver, initial_value, iterations);
    assert_eq!(
        c_output, rust_output,
        "CONFIGS.md row {row}: initial={initial_value}, iterations={iterations}"
    );
}

fn child_status(function: StaticAlias) -> c_int {
    unsafe {
        let pid = fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            let _ = function(ptr::null_mut());
            _exit(0);
        }
        let mut status = 0;
        assert_eq!(waitpid(pid, ptr::addr_of_mut!(status), 0), pid);
        status
    }
}

fn terminating_signal(status: c_int) -> Option<c_int> {
    let signal = status & 0x7f;
    (signal != 0 && signal != 0x7f).then_some(signal)
}

#[test]
fn all_configuration_and_error_rows_match() {
    let mut rng = Rng::new(0x5eed_c0de_d15c_a11a);

    // CONFIGS.md row 1: caller-owned pointer, value below current inner.
    for _ in 0..RANDOM_CASES {
        let pair = Pair::fresh();
        let current_inner = precondition_inner(&pair, rng.range(1, 10_000));
        let input = current_inner - rng.range(1, 20_000);
        let (returned_caller_storage, _) = compare_stack_call(&pair, input, 1);
        assert!(returned_caller_storage);
    }

    // CONFIGS.md row 2: caller-owned pointer exactly equal to randomized inner.
    for _ in 0..RANDOM_CASES {
        let pair = Pair::fresh();
        let seed_value = rng.range(1, 10_000);
        let current_inner = precondition_inner(&pair, seed_value);
        let (returned_caller_storage, _) = compare_stack_call(&pair, current_inner, 2);
        assert!(!returned_caller_storage);
    }

    // CONFIGS.md row 3: caller-owned pointer above current inner.
    for _ in 0..RANDOM_CASES {
        let pair = Pair::fresh();
        let current_inner = precondition_inner(&pair, rng.range(1, 10_000));
        let input = current_inner + rng.range(1, 20_000);
        let (returned_caller_storage, _) = compare_stack_call(&pair, input, 3);
        assert!(!returned_caller_storage);
    }

    // CONFIGS.md row 4: feed each library's returned static pointer back in.
    for _ in 0..RANDOM_CASES {
        let pair = Pair::fresh();
        let seed_value = rng.range(1, 10_000);
        let mut c_seed = seed_value;
        let mut rust_seed = seed_value;
        let c_inner = unsafe { (pair.c.static_alias)(ptr::addr_of_mut!(c_seed)) };
        let rust_inner = unsafe { (pair.rust.static_alias)(ptr::addr_of_mut!(rust_seed)) };
        let c_returned = unsafe { (pair.c.static_alias)(c_inner) };
        let rust_returned = unsafe { (pair.rust.static_alias)(rust_inner) };
        assert_eq!(c_returned, c_inner, "CONFIGS.md row 4: C alias");
        assert_eq!(rust_returned, rust_inner, "CONFIGS.md row 4: Rust alias");
        assert_eq!(
            unsafe { c_returned.read() },
            unsafe { rust_returned.read() },
            "CONFIGS.md row 4: aliased result"
        );
    }

    // CONFIGS.md row 5: zero and negative iteration counts.
    for case in 0..RANDOM_CASES {
        let pair = Pair::fresh();
        let current_inner = precondition_inner(&pair, rng.range(1, 100));
        let initial = rng.next_u32() as c_int;
        let iterations = match case % 4 {
            0 => 0,
            1 => c_int::MIN,
            _ => -rng.range(1, 10_000),
        };
        compare_driver(&pair, initial, iterations, 5);
        let (returned_caller_storage, returned_value) = compare_stack_call(&pair, current_inner, 5);
        assert!(
            !returned_caller_storage,
            "CONFIGS.md row 5: no-op driver changed inner"
        );
        assert_eq!(
            returned_value,
            current_inner * 2,
            "CONFIGS.md row 5: no-op driver changed inner"
        );
    }

    for row in 6..=8 {
        for _ in 0..RANDOM_CASES {
            let pair = Pair::fresh();
            let current_inner = precondition_inner(&pair, rng.range(1, 100));
            let initial = match row {
                6 => current_inner - rng.range(1, current_inner + 100),
                7 => current_inner,
                8 => current_inner + rng.range(1, 100),
                _ => unreachable!(),
            };
            compare_driver(&pair, initial, 1, row);
        }
    }

    // CONFIGS.md row 9: all iterations remain below inner.
    for _ in 0..RANDOM_CASES {
        let pair = Pair::fresh();
        let current_inner = precondition_inner(&pair, rng.range(1, 100));
        let iterations = rng.range(2, 7);
        let initial = (2 - iterations) * current_inner - rng.range(1, 100);
        compare_driver(&pair, initial, iterations, 9);
    }

    // CONFIGS.md row 10: multiple lower calls, then an upper call.
    for _ in 0..RANDOM_CASES {
        let pair = Pair::fresh();
        let current_inner = precondition_inner(&pair, rng.range(1, 100));
        let iterations = rng.range(3, 7);
        let lower_calls = rng.range(2, iterations);
        let lower_bound = (1 - lower_calls) * current_inner;
        let upper_bound = (2 - lower_calls) * current_inner;
        let initial = rng.range(lower_bound, upper_bound);
        compare_driver(&pair, initial, iterations, 10);
    }

    // CONFIGS.md row 11: one lower call, then upper calls.
    for _ in 0..RANDOM_CASES {
        let pair = Pair::fresh();
        let current_inner = precondition_inner(&pair, rng.range(1, 100));
        let initial = rng.range(0, current_inner);
        compare_driver(&pair, initial, rng.range(2, 7), 11);
    }

    for row in 12..=13 {
        for _ in 0..RANDOM_CASES {
            let pair = Pair::fresh();
            let current_inner = precondition_inner(&pair, rng.range(1, 100));
            let initial = if row == 12 {
                current_inner
            } else {
                current_inner + rng.range(1, 100)
            };
            compare_driver(&pair, initial, rng.range(2, 7), row);
        }
    }

    // ERRORS.md row 1: both exported functions must reject NULL identically.
    let pair = Pair::fresh();
    let c_status = child_status(pair.c.static_alias);
    let rust_status = child_status(pair.rust.static_alias);
    assert_eq!(
        terminating_signal(c_status),
        Some(SIGSEGV),
        "ERRORS.md row 1: C did not terminate with SIGSEGV"
    );
    assert_eq!(
        terminating_signal(rust_status),
        terminating_signal(c_status),
        "ERRORS.md row 1: C and Rust termination differ"
    );
}
