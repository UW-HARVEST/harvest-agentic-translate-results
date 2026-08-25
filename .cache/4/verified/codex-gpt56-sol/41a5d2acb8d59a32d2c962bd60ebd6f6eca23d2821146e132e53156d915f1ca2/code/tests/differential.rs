use libloading::Library;
use std::ffi::{CString, c_char, c_int, c_void};
use std::fs::{self, OpenOptions};
use std::io::{Read, Seek, SeekFrom};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

type MainFn = unsafe extern "C" fn(c_int, *mut *mut c_char) -> c_int;

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

static STDOUT_LOCK: Mutex<()> = Mutex::new(());
static NEXT_CAPTURE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Eq, PartialEq)]
struct Outcome {
    code: c_int,
    stdout: Vec<u8>,
}

struct Implementations {
    c: Library,
    rust: Library,
}

impl Implementations {
    fn load() -> Self {
        let c = unsafe { Library::new(c_library_path()) }.expect("load C reference shared library");
        let rust = unsafe { Library::new(rust_library_path()) }
            .expect("load Rust translated shared library");
        Self { c, rust }
    }

    fn compare_input(&self, input: &str) -> Outcome {
        let c = invoke_with_input(&self.c, input, false);
        let rust = invoke_with_input(&self.rust, input, false);
        assert_eq!(rust, c, "input {input:?}");
        c
    }

    fn compare_argc(&self, argc: c_int) -> Outcome {
        let c = invoke_with_null_argv(&self.c, argc);
        let rust = invoke_with_null_argv(&self.rust, argc);
        assert_eq!(rust, c, "argc {argc}");
        c
    }
}

struct Lcg(u64);

impl Lcg {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0
    }

    fn below(&mut self, upper: u64) -> u64 {
        self.next() % upper
    }
}

fn c_library_path() -> PathBuf {
    std::env::var_os("C_REFERENCE_SO").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("c_src/build/libnineality.so"),
        PathBuf::from,
    )
}

fn rust_library_path() -> PathBuf {
    std::env::var_os("RUST_TRANSLATED_SO").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("target/debug/libdriver.so"),
        PathBuf::from,
    )
}

fn main_symbol(library: &Library) -> MainFn {
    unsafe {
        *library
            .get::<MainFn>(b"main\0")
            .expect("resolve exported main")
    }
}

fn invoke_with_input(library: &Library, input: &str, null_argv0: bool) -> Outcome {
    let main = main_symbol(library);
    let program = CString::new("driver").unwrap();
    let input = CString::new(input).unwrap();
    let mut argv = [
        if null_argv0 {
            ptr::null_mut()
        } else {
            program.as_ptr().cast_mut()
        },
        input.as_ptr().cast_mut(),
        ptr::null_mut(),
    ];
    capture_stdout(|| unsafe { main(2, argv.as_mut_ptr()) })
}

fn invoke_with_null_argv(library: &Library, argc: c_int) -> Outcome {
    let main = main_symbol(library);
    capture_stdout(|| unsafe { main(argc, ptr::null_mut()) })
}

fn capture_stdout(call: impl FnOnce() -> c_int) -> Outcome {
    let _lock = STDOUT_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let id = NEXT_CAPTURE.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "nineality-differential-{}-{id}.out",
        std::process::id()
    ));
    let mut output = OpenOptions::new()
        .create_new(true)
        .read(true)
        .write(true)
        .open(&path)
        .expect("create stdout capture");

    unsafe {
        assert_eq!(fflush(ptr::null_mut()), 0, "flush stdout before capture");
    }
    let saved_stdout = unsafe { dup(1) };
    assert!(saved_stdout >= 0, "duplicate stdout");
    assert_eq!(unsafe { dup2(output.as_raw_fd(), 1) }, 1, "redirect stdout");

    let code = call();

    unsafe {
        assert_eq!(fflush(ptr::null_mut()), 0, "flush captured stdout");
    }
    assert_eq!(unsafe { dup2(saved_stdout, 1) }, 1, "restore stdout");
    assert_eq!(unsafe { close(saved_stdout) }, 0, "close saved stdout");

    output.seek(SeekFrom::Start(0)).unwrap();
    let mut stdout = Vec::new();
    output.read_to_end(&mut stdout).unwrap();
    drop(output);
    fs::remove_file(path).unwrap();

    Outcome { code, stdout }
}

#[test]
fn config_1_initial_remainder_nine() {
    let libraries = Implementations::load();
    let mut random = Lcg::new(0x80cb_94b1_57a2_e0d9);

    for _ in 0..64 {
        let value = random.below(100_000) * 10 + 9;
        let outcome = libraries.compare_input(&value.to_string());
        assert_eq!(outcome.code, 0);
        assert_eq!(outcome.stdout, format!("{value}\n").as_bytes());
    }
}

#[test]
fn config_2_nonnegative_incrementing_counts() {
    let libraries = Implementations::load();
    let mut random = Lcg::new(0xb6f2_f92a_849d_2931);

    for _ in 0..64 {
        let mut value = random.below(100_000);
        if value % 10 == 9 {
            value -= 1;
        }
        let outcome = libraries.compare_input(&value.to_string());
        assert_eq!(outcome.code, 0);
        assert!(outcome.stdout.starts_with(format!("{value}\n").as_bytes()));
        assert!(outcome.stdout.ends_with(b"9\n"));
    }
}

#[test]
fn config_3_small_negative_counts_cross_zero() {
    let libraries = Implementations::load();
    let mut random = Lcg::new(0xb2e5_2f3a_a15c_6d77);

    for _ in 0..64 {
        let value = -1 - random.below(128) as i64;
        let outcome = libraries.compare_input(&value.to_string());
        assert_eq!(outcome.code, 0);
        assert!(outcome.stdout.starts_with(format!("{value}\n").as_bytes()));
        assert!(outcome.stdout.ends_with(b"9\n"));
    }
}

#[test]
fn config_4_whitespace_and_signs() {
    let libraries = Implementations::load();
    let mut random = Lcg::new(0xd1b6_0fb7_4c6e_e163);
    let whitespace = [" ", "\t", "\n", " \t"];

    for index in 0..64 {
        let magnitude = random.below(128);
        let sign = if index % 2 == 0 { "+" } else { "-" };
        let prefix = whitespace[random.below(whitespace.len() as u64) as usize];
        let input = format!("{prefix}{sign}{magnitude}");
        assert_eq!(libraries.compare_input(&input).code, 0);
    }
}

#[test]
fn config_5_trailing_nonnumeric_bytes() {
    let libraries = Implementations::load();
    let mut random = Lcg::new(0xc4d7_811a_b931_6b7d);
    let suffixes = ["x", "xyz", "_tail", "  trailing"];

    for _ in 0..64 {
        let value = random.below(1_000);
        let suffix = suffixes[random.below(suffixes.len() as u64) as usize];
        let input = format!("{value}{suffix}");
        assert_eq!(libraries.compare_input(&input).code, 0);
    }
}

#[test]
fn config_6_long_range_overflow() {
    let libraries = Implementations::load();
    let mut random = Lcg::new(0xee30_2f55_02a7_643f);

    for index in 0..64 {
        let extra_digits = 1 + random.below(12);
        let mut digits = String::from("922337203685477580");
        digits.push(if index % 2 == 0 { '8' } else { '9' });
        for _ in 0..extra_digits {
            digits.push(char::from(b'0' + random.below(10) as u8));
        }
        if index % 2 == 1 {
            digits.insert(0, '-');
        }
        assert_eq!(libraries.compare_input(&digits).code, 0);
    }
}

#[test]
fn error_1_rejects_every_argc_other_than_two() {
    let libraries = Implementations::load();
    let expected = Outcome {
        code: 1,
        stdout: b"Error: should only be a single (integer) argument!\n".to_vec(),
    };

    for argc in [c_int::MIN, -1, 0, 1, 3, 4, 127, c_int::MAX] {
        assert_eq!(libraries.compare_argc(argc), expected);
    }
}

#[test]
fn error_2_rejects_inputs_with_no_conversion() {
    let libraries = Implementations::load();
    let expected = Outcome {
        code: 1,
        stdout: b"Error: first argument must be an integer!\n".to_vec(),
    };
    let fixed = ["", " ", "\t\n", "+", "-", "x", " x", "--1", "+-1"];

    for input in fixed {
        assert_eq!(libraries.compare_input(input), expected, "input {input:?}");
    }

    let mut random = Lcg::new(0xb1aa_551d_ea13_9807);
    for _ in 0..64 {
        let prefix = ["", " ", "\t"][random.below(3) as usize];
        let first = char::from(b'a' + random.below(26) as u8);
        let input = format!("{prefix}{first}{}", random.next());
        assert_eq!(libraries.compare_input(&input), expected);
    }
}

#[test]
fn generic_null_unused_argv_element_is_accepted() {
    let libraries = Implementations::load();
    let c = invoke_with_input(&libraries.c, "9", true);
    let rust = invoke_with_input(&libraries.rust, "9", true);

    assert_eq!(rust, c);
    assert_eq!(
        c,
        Outcome {
            code: 0,
            stdout: b"9\n".to_vec(),
        }
    );
}
