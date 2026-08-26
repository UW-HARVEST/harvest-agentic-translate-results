use libloading::Library;
use std::env;
use std::ffi::{CString, c_char, c_int, c_void};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

type PrintLine = unsafe extern "C" fn(*const c_char);
type PrintIntLine = unsafe extern "C" fn(c_int);
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

static STDOUT_LOCK: Mutex<()> = Mutex::new(());
const STDOUT_FILENO: c_int = 1;

struct Libraries {
    c: Library,
    rust: Library,
}

impl Libraries {
    unsafe fn load() -> Self {
        let c_path = library_path("DRIVER_C_SO", "c_src/build/libdriver.so");
        let rust_path = library_path("DRIVER_RUST_SO", "target/release/libdriver.so");
        assert!(
            c_path.is_file(),
            "C shared library does not exist: {}",
            c_path.display()
        );
        assert!(
            rust_path.is_file(),
            "Rust shared library does not exist: {}",
            rust_path.display()
        );

        Self {
            c: unsafe { Library::new(&c_path) }
                .unwrap_or_else(|error| panic!("failed to load {}: {error}", c_path.display())),
            rust: unsafe { Library::new(&rust_path) }
                .unwrap_or_else(|error| panic!("failed to load {}: {error}", rust_path.display())),
        }
    }
}

fn library_path(variable: &str, relative_default: &str) -> PathBuf {
    env::var_os(variable)
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join(relative_default))
}

fn capture_stdout(call: impl FnOnce()) -> Vec<u8> {
    let _guard = STDOUT_LOCK.lock().unwrap();
    let mut pipe_fds = [-1; 2];

    unsafe {
        assert_eq!(
            fflush(std::ptr::null_mut()),
            0,
            "fflush before capture failed"
        );
        assert_eq!(pipe(pipe_fds.as_mut_ptr()), 0, "pipe failed");
    }

    let saved_stdout = unsafe { dup(STDOUT_FILENO) };
    assert!(saved_stdout >= 0, "dup failed");
    assert_eq!(
        unsafe { dup2(pipe_fds[1], STDOUT_FILENO) },
        STDOUT_FILENO,
        "redirecting stdout failed"
    );
    assert_eq!(unsafe { close(pipe_fds[1]) }, 0, "closing write fd failed");

    call();

    unsafe {
        assert_eq!(fflush(std::ptr::null_mut()), 0, "fflush after call failed");
        assert_eq!(
            dup2(saved_stdout, STDOUT_FILENO),
            STDOUT_FILENO,
            "restoring stdout failed"
        );
        assert_eq!(close(saved_stdout), 0, "closing saved stdout failed");
    }

    let mut output = Vec::new();
    let mut buffer = [0_u8; 4096];
    loop {
        let count = unsafe { read(pipe_fds[0], buffer.as_mut_ptr().cast(), buffer.len()) };
        assert!(count >= 0, "read failed");
        if count == 0 {
            break;
        }
        output.extend_from_slice(&buffer[..count as usize]);
    }
    assert_eq!(unsafe { close(pipe_fds[0]) }, 0, "closing read fd failed");
    output
}

fn assert_same_output(name: &str, case: &str, c_call: impl FnOnce(), rust_call: impl FnOnce()) {
    let c_output = capture_stdout(c_call);
    let rust_output = capture_stdout(rust_call);
    assert_eq!(
        c_output, rust_output,
        "{name} output differs for case {case}"
    );
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

    fn next_i32(&mut self) -> i32 {
        self.next_u64() as i32
    }
}

fn config_1_print_line_non_null_randomized() {
    let libraries = unsafe { Libraries::load() };
    let c_function = unsafe { libraries.c.get::<PrintLine>(b"printLine\0").unwrap() };
    let rust_function = unsafe { libraries.rust.get::<PrintLine>(b"printLine\0").unwrap() };
    let mut rng = Rng::new(0x42f0_e1eb_a9ea_3693);

    let mut cases = vec![CString::new("").unwrap(), CString::new("line").unwrap()];
    for _ in 0..256 {
        let length = (rng.next_u64() % 513) as usize;
        let bytes = (0..length)
            .map(|_| {
                let value = (rng.next_u64() % 127) as u8;
                if value == 0 { 1 } else { value }
            })
            .collect::<Vec<_>>();
        cases.push(CString::new(bytes).unwrap());
    }

    for (index, value) in cases.iter().enumerate() {
        assert_same_output(
            "printLine",
            &index.to_string(),
            || unsafe { c_function(value.as_ptr()) },
            || unsafe { rust_function(value.as_ptr()) },
        );
    }
}

fn config_2_and_error_1_print_line_null() {
    let libraries = unsafe { Libraries::load() };
    let c_function = unsafe { libraries.c.get::<PrintLine>(b"printLine\0").unwrap() };
    let rust_function = unsafe { libraries.rust.get::<PrintLine>(b"printLine\0").unwrap() };

    for index in 0..64 {
        assert_same_output(
            "printLine",
            &format!("null-{index}"),
            || unsafe { c_function(std::ptr::null()) },
            || unsafe { rust_function(std::ptr::null()) },
        );
    }
}

fn config_3_print_int_line_randomized() {
    let libraries = unsafe { Libraries::load() };
    let c_function = unsafe { libraries.c.get::<PrintIntLine>(b"printIntLine\0").unwrap() };
    let rust_function = unsafe {
        libraries
            .rust
            .get::<PrintIntLine>(b"printIntLine\0")
            .unwrap()
    };
    let mut rng = Rng::new(0xa409_3822_299f_31d0);
    let mut cases = vec![i32::MIN, -1, 0, 1, i32::MAX];
    cases.extend((0..512).map(|_| rng.next_i32()));

    for value in cases {
        assert_same_output(
            "printIntLine",
            &value.to_string(),
            || unsafe { c_function(value) },
            || unsafe { rust_function(value) },
        );
    }
}

fn config_4_bad_direct() {
    let libraries = unsafe { Libraries::load() };
    let c_function = unsafe { libraries.c.get::<NoArgs>(b"bad\0").unwrap() };
    let rust_function = unsafe { libraries.rust.get::<NoArgs>(b"bad\0").unwrap() };

    for index in 0..64 {
        assert_same_output(
            "bad",
            &index.to_string(),
            || unsafe { c_function() },
            || unsafe { rust_function() },
        );
    }
}

fn config_5_good_direct() {
    let libraries = unsafe { Libraries::load() };
    let c_function = unsafe { libraries.c.get::<NoArgs>(b"good\0").unwrap() };
    let rust_function = unsafe { libraries.rust.get::<NoArgs>(b"good\0").unwrap() };

    for index in 0..64 {
        assert_same_output(
            "good",
            &index.to_string(),
            || unsafe { c_function() },
            || unsafe { rust_function() },
        );
    }
}

fn config_6_driver_zero_dispatches_to_bad() {
    let libraries = unsafe { Libraries::load() };
    let c_function = unsafe { libraries.c.get::<Driver>(b"driver\0").unwrap() };
    let rust_function = unsafe { libraries.rust.get::<Driver>(b"driver\0").unwrap() };

    for index in 0..64 {
        assert_same_output(
            "driver",
            &format!("zero-{index}"),
            || unsafe { c_function(0) },
            || unsafe { rust_function(0) },
        );
    }
}

fn config_7_driver_nonzero_dispatches_to_good() {
    let libraries = unsafe { Libraries::load() };
    let c_function = unsafe { libraries.c.get::<Driver>(b"driver\0").unwrap() };
    let rust_function = unsafe { libraries.rust.get::<Driver>(b"driver\0").unwrap() };
    let mut rng = Rng::new(0x1319_8a2e_0370_7344);
    let mut cases = vec![i32::MIN, -1, 1, i32::MAX];
    cases.extend((0..256).map(|_| {
        let value = rng.next_i32();
        if value == 0 { 1 } else { value }
    }));

    for value in cases {
        assert_same_output(
            "driver",
            &value.to_string(),
            || unsafe { c_function(value) },
            || unsafe { rust_function(value) },
        );
    }
}

#[test]
fn differential_surface() {
    config_1_print_line_non_null_randomized();
    config_2_and_error_1_print_line_null();
    config_3_print_int_line_randomized();
    config_4_bad_direct();
    config_5_good_direct();
    config_6_driver_zero_dispatches_to_bad();
    config_7_driver_nonzero_dispatches_to_good();
}
