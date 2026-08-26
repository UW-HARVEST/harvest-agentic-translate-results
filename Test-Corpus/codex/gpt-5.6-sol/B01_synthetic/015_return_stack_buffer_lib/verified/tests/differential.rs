use libloading::Library;
use std::ffi::{c_char, c_int, c_void};
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::{Mutex, MutexGuard};

type PrintLine = unsafe extern "C" fn(*const c_char);
type NoArgs = unsafe extern "C" fn();
type Driver = unsafe extern "C" fn(c_int);

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(fds: *mut c_int) -> c_int;
    fn read(fd: c_int, buffer: *mut c_void, count: usize) -> isize;
}

struct Api {
    _library: Library,
    print_line: PrintLine,
    bad: NoArgs,
    good: NoArgs,
    driver: Driver,
}

impl Api {
    fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let print_line = unsafe {
            *library
                .get::<PrintLine>(b"printLine\0")
                .expect("missing printLine")
        };
        let bad = unsafe { *library.get::<NoArgs>(b"bad\0").expect("missing bad") };
        let good = unsafe { *library.get::<NoArgs>(b"good\0").expect("missing good") };
        let driver = unsafe { *library.get::<Driver>(b"driver\0").expect("missing driver") };

        Self {
            _library: library,
            print_line,
            bad,
            good,
            driver,
        }
    }
}

struct Pair {
    c: Api,
    rust: Api,
}

impl Pair {
    fn load() -> Self {
        Self {
            c: Api::load(&c_library_path()),
            rust: Api::load(&rust_library_path()),
        }
    }

    fn compare(&self, context: &str, c_call: impl FnOnce(), rust_call: impl FnOnce()) {
        let c_output = capture_stdout(c_call);
        let rust_output = capture_stdout(rust_call);
        assert_eq!(c_output, rust_output, "stdout mismatch for {context}");
    }
}

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

    fn non_nul_byte(&mut self) -> u8 {
        (self.next_u64() % 255 + 1) as u8
    }

    fn bytes(&mut self, length: usize) -> Vec<u8> {
        (0..length).map(|_| self.non_nul_byte()).collect()
    }
}

static STDOUT_LOCK: Mutex<()> = Mutex::new(());

fn setup() -> (MutexGuard<'static, ()>, Pair) {
    let guard = STDOUT_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    (guard, Pair::load())
}

fn c_library_path() -> PathBuf {
    std::env::var_os("C_DRIVER_SO").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("c_src/build/libdriver.so"),
        PathBuf::from,
    )
}

fn rust_library_path() -> PathBuf {
    if let Some(path) = std::env::var_os("RUST_DRIVER_SO") {
        return PathBuf::from(path);
    }

    let executable = std::env::current_exe().expect("failed to locate test executable");
    let deps = executable
        .parent()
        .expect("test executable has no parent directory");
    let profile = deps
        .parent()
        .expect("target profile has no parent directory");
    let target = profile
        .parent()
        .expect("target directory has no parent directory");
    let candidates = [
        profile.join("libdriver.so"),
        deps.join("libdriver.so"),
        target.join("release/libdriver.so"),
    ];

    candidates
        .into_iter()
        .find(|path| path.is_file())
        .unwrap_or_else(|| {
            panic!(
                "Rust cdylib not found next to test artifacts under {}",
                profile.display()
            )
        })
}

fn capture_stdout(call: impl FnOnce()) -> Vec<u8> {
    const STDOUT_FILENO: c_int = 1;

    unsafe {
        assert_eq!(fflush(ptr::null_mut()), 0, "fflush before capture failed");

        let mut pipe_fds = [-1; 2];
        assert_eq!(pipe(pipe_fds.as_mut_ptr()), 0, "pipe failed");

        let saved_stdout = dup(STDOUT_FILENO);
        assert!(saved_stdout >= 0, "dup failed");
        assert_eq!(
            dup2(pipe_fds[1], STDOUT_FILENO),
            STDOUT_FILENO,
            "dup2 capture failed"
        );
        assert_eq!(close(pipe_fds[1]), 0, "close write descriptor failed");

        call();

        assert_eq!(fflush(ptr::null_mut()), 0, "fflush after call failed");
        assert_eq!(
            dup2(saved_stdout, STDOUT_FILENO),
            STDOUT_FILENO,
            "dup2 restore failed"
        );
        assert_eq!(close(saved_stdout), 0, "close saved descriptor failed");

        let mut output = Vec::new();
        let mut buffer = [0_u8; 8192];
        loop {
            let count = read(pipe_fds[0], buffer.as_mut_ptr().cast(), buffer.len());
            assert!(count >= 0, "read failed");
            if count == 0 {
                break;
            }
            output.extend_from_slice(&buffer[..count as usize]);
        }
        assert_eq!(close(pipe_fds[0]), 0, "close read descriptor failed");
        output
    }
}

fn compare_print_line(pair: &Pair, context: &str, input: &[u8]) {
    let pointer = input.as_ptr().cast::<c_char>();
    pair.compare(
        context,
        || unsafe { (pair.c.print_line)(pointer) },
        || unsafe { (pair.rust.print_line)(pointer) },
    );
}

#[test]
fn config_1_print_line_empty() {
    let (_guard, pair) = setup();
    let mut rng = Rng::new(0x8105_78ec_3a91_042d);

    for case in 0..256 {
        let mut input = vec![0];
        let ignored_length = (rng.next_u64() % 64) as usize;
        input.extend(rng.bytes(ignored_length));
        compare_print_line(&pair, &format!("empty string case {case}"), &input);
    }
}

#[test]
fn config_2_print_line_one_visible_byte() {
    let (_guard, pair) = setup();
    let mut rng = Rng::new(0x50ba_67f3_8ca0_199e);

    for case in 0..512 {
        let mut input = vec![rng.non_nul_byte(), 0];
        let ignored_length = (rng.next_u64() % 32) as usize;
        input.extend(rng.bytes(ignored_length));
        compare_print_line(&pair, &format!("one-byte string case {case}"), &input);
    }
}

#[test]
fn config_3_print_line_many_visible_bytes() {
    let (_guard, pair) = setup();
    let mut rng = Rng::new(0xbb87_1f02_2d43_6aa1);
    let mut lengths = vec![2, 3, 15, 255, 4095, 4096];
    lengths.extend((0..122).map(|_| (rng.next_u64() % 4095 + 2) as usize));

    for (case, length) in lengths.into_iter().enumerate() {
        let mut input = rng.bytes(length);
        input.push(0);
        compare_print_line(&pair, &format!("{length}-byte string case {case}"), &input);
    }
}

#[test]
fn config_4_print_line_interior_nul() {
    let (_guard, pair) = setup();
    let mut rng = Rng::new(0xa9d0_e774_b6cc_351f);

    for case in 0..256 {
        let prefix_length = (rng.next_u64() % 128 + 1) as usize;
        let suffix_length = (rng.next_u64() % 128 + 1) as usize;
        let mut input = rng.bytes(prefix_length);
        input.push(0);
        input.extend(rng.bytes(suffix_length));
        input.push(0);
        compare_print_line(&pair, &format!("interior-NUL case {case}"), &input);
    }
}

#[test]
fn config_5_bad() {
    let (_guard, pair) = setup();

    for case in 0..64 {
        pair.compare(
            &format!("bad case {case}"),
            || unsafe { (pair.c.bad)() },
            || unsafe { (pair.rust.bad)() },
        );
    }
}

#[test]
fn config_6_good() {
    let (_guard, pair) = setup();

    for case in 0..64 {
        pair.compare(
            &format!("good case {case}"),
            || unsafe { (pair.c.good)() },
            || unsafe { (pair.rust.good)() },
        );
    }
}

#[test]
fn config_7_driver_zero() {
    let (_guard, pair) = setup();

    for case in 0..64 {
        pair.compare(
            &format!("driver zero case {case}"),
            || unsafe { (pair.c.driver)(0) },
            || unsafe { (pair.rust.driver)(0) },
        );
    }
}

#[test]
fn config_8_driver_nonzero() {
    let (_guard, pair) = setup();
    let mut rng = Rng::new(0x0751_dadc_b702_918e);
    let mut values = vec![c_int::MIN, -1, 1, c_int::MAX];

    while values.len() < 512 {
        let value = rng.next_u64() as c_int;
        if value != 0 {
            values.push(value);
        }
    }

    for (case, value) in values.into_iter().enumerate() {
        pair.compare(
            &format!("driver nonzero value {value}, case {case}"),
            || unsafe { (pair.c.driver)(value) },
            || unsafe { (pair.rust.driver)(value) },
        );
    }
}

#[test]
fn error_1_print_line_null() {
    let (_guard, pair) = setup();

    for case in 0..64 {
        let c_output = capture_stdout(|| unsafe { (pair.c.print_line)(ptr::null()) });
        let rust_output = capture_stdout(|| unsafe { (pair.rust.print_line)(ptr::null()) });
        assert_eq!(
            c_output, rust_output,
            "null rejection mismatch, case {case}"
        );
        assert!(c_output.is_empty(), "C wrote output for null, case {case}");
    }
}
