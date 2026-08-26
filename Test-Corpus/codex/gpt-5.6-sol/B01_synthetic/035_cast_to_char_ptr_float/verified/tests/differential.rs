use libloading::Library;
use std::env;
use std::ffi::{c_int, c_void};
use std::fs;
use std::io;
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;

type DriverFn = unsafe extern "C" fn(f32);
type MainFn = unsafe extern "C" fn() -> c_int;

static C_LIBRARY: OnceLock<PathBuf> = OnceLock::new();
static RUST_LIBRARY: OnceLock<PathBuf> = OnceLock::new();
static INVOCATION_ID: AtomicU64 = AtomicU64::new(0);

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

#[derive(Debug, Eq, PartialEq)]
struct CallResult {
    status: c_int,
    stdout: Vec<u8>,
}

struct XorShift32(u32);

impl XorShift32 {
    fn new(seed: u32) -> Self {
        Self(seed)
    }

    fn next(&mut self) -> u32 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 17;
        value ^= value << 5;
        self.0 = value;
        value
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library() -> &'static Path {
    C_LIBRARY
        .get_or_init(|| {
            let output_dir = manifest_dir().join("target/differential");
            fs::create_dir_all(&output_dir).expect("create C shared-library output directory");
            let library = output_dir.join("libdriver_c.so");
            let output = Command::new("cc")
                .args(["-shared", "-fPIC"])
                .arg(manifest_dir().join("c_src/src/main.c"))
                .arg("-o")
                .arg(&library)
                .output()
                .expect("run C compiler");
            assert_command_succeeded("compile C shared library", &output);
            library
        })
        .as_path()
}

fn rust_library() -> &'static Path {
    RUST_LIBRARY
        .get_or_init(|| {
            let output_dir = manifest_dir().join("target/differential");
            fs::create_dir_all(&output_dir).expect("create Rust shared-library output directory");
            let library = output_dir.join("libdriver_rust.so");
            let output = Command::new("rustc")
                .args([
                    "--edition=2021",
                    "--crate-type=cdylib",
                    "--crate-name=driver",
                ])
                .arg(manifest_dir().join("src/lib.rs"))
                .arg("-o")
                .arg(&library)
                .output()
                .expect("run Rust compiler");
            assert_command_succeeded("compile Rust shared library", &output);
            library
        })
        .as_path()
}

fn assert_command_succeeded(operation: &str, output: &Output) {
    assert!(
        output.status.success(),
        "{operation} failed with {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn temporary_path(label: &str) -> PathBuf {
    let id = INVOCATION_ID.fetch_add(1, Ordering::Relaxed);
    env::temp_dir().join(format!(
        "driver-differential-{}-{id}-{label}",
        std::process::id()
    ))
}

fn invoke(library: &Path, mode: &str, input: &[u8], driver_bits: &[u32]) -> CallResult {
    let input_path = temporary_path("stdin");
    let output_path = temporary_path("stdout");
    let status_path = temporary_path("status");
    fs::write(&input_path, input).expect("write child stdin");

    let bits = driver_bits
        .iter()
        .map(|value| format!("{value:08x}"))
        .collect::<Vec<_>>()
        .join(",");

    let output = Command::new(env::current_exe().expect("locate integration test executable"))
        .args(["--exact", "ffi_child_entry", "--nocapture"])
        .env("DRIVER_FFI_CHILD", "1")
        .env("DRIVER_FFI_LIBRARY", library)
        .env("DRIVER_FFI_MODE", mode)
        .env("DRIVER_FFI_INPUT", &input_path)
        .env("DRIVER_FFI_OUTPUT", &output_path)
        .env("DRIVER_FFI_STATUS", &status_path)
        .env("DRIVER_FFI_BITS", bits)
        .output()
        .expect("run isolated FFI helper");
    assert_command_succeeded("isolated FFI helper", &output);

    let result = CallResult {
        status: fs::read_to_string(&status_path)
            .expect("read child return status")
            .parse()
            .expect("parse child return status"),
        stdout: fs::read(&output_path).expect("read child stdout"),
    };

    for path in [input_path, output_path, status_path] {
        let _ = fs::remove_file(path);
    }
    result
}

fn compare_main(input: &[u8]) {
    let c = invoke(c_library(), "main", input, &[]);
    let rust = invoke(rust_library(), "main", input, &[]);
    assert_eq!(c, rust, "input bytes: {input:?}");
}

fn redirect_fd(file: &fs::File, target: c_int) -> c_int {
    let saved = unsafe { dup(target) };
    assert!(
        saved >= 0,
        "dup({target}) failed: {}",
        io::Error::last_os_error()
    );
    let result = unsafe { dup2(file.as_raw_fd(), target) };
    assert!(
        result >= 0,
        "dup2 to {target} failed: {}",
        io::Error::last_os_error()
    );
    saved
}

fn restore_fd(saved: c_int, target: c_int) {
    let result = unsafe { dup2(saved, target) };
    assert!(
        result >= 0,
        "restore fd {target} failed: {}",
        io::Error::last_os_error()
    );
    unsafe {
        close(saved);
    }
}

#[test]
fn ffi_child_entry() {
    if env::var_os("DRIVER_FFI_CHILD").is_none() {
        return;
    }

    let input =
        fs::File::open(env::var_os("DRIVER_FFI_INPUT").unwrap()).expect("open redirected stdin");
    let output = fs::File::create(env::var_os("DRIVER_FFI_OUTPUT").unwrap())
        .expect("open redirected stdout");
    let saved_stdin = redirect_fd(&input, 0);
    let saved_stdout = redirect_fd(&output, 1);

    let library = unsafe {
        Library::new(env::var_os("DRIVER_FFI_LIBRARY").unwrap()).expect("load shared library")
    };
    let status = match env::var("DRIVER_FFI_MODE").unwrap().as_str() {
        "driver" => {
            let driver = unsafe {
                *library
                    .get::<DriverFn>(b"driver\0")
                    .expect("load driver symbol")
            };
            let bits = env::var("DRIVER_FFI_BITS").unwrap();
            for bits in bits.split(',').filter(|value| !value.is_empty()) {
                let value = f32::from_bits(u32::from_str_radix(bits, 16).unwrap());
                unsafe {
                    driver(value);
                }
            }
            0
        }
        "main" => {
            let entry = unsafe { *library.get::<MainFn>(b"main\0").expect("load main symbol") };
            unsafe { entry() }
        }
        mode => panic!("unknown child mode {mode}"),
    };

    unsafe {
        fflush(std::ptr::null_mut());
    }
    restore_fd(saved_stdout, 1);
    restore_fd(saved_stdin, 0);
    fs::write(
        env::var_os("DRIVER_FFI_STATUS").unwrap(),
        status.to_string(),
    )
    .expect("write child return status");
}

#[test]
fn config_1_driver_arbitrary_float_bit_patterns() {
    let mut bits = vec![
        0x0000_0000,
        0x8000_0000,
        0x0000_0001,
        0x007f_ffff,
        0x0080_0000,
        0x3f80_0000,
        0x7f7f_ffff,
        0x7f80_0000,
        0xff80_0000,
        0x7fc0_0000,
        0x7f80_0001,
        0xffff_ffff,
    ];
    let mut random = XorShift32::new(0x5eed_1234);
    bits.extend((0..1_024).map(|_| random.next()));

    let c = invoke(c_library(), "driver", &[], &bits);
    let rust = invoke(rust_library(), "driver", &[], &bits);
    assert_eq!(c, rust);
    assert_eq!(c.status, 0);
    assert_eq!(c.stdout.len(), bits.len() * 9);
}

#[test]
fn config_2_main_valid_float_tokens() {
    let mut random = XorShift32::new(0xc001_cafe);
    let fixed = ["0\n", "-0\n", "inf\n", "-infinity\n", "nan\n", "NAN\n"];
    for input in fixed {
        compare_main(input.as_bytes());
    }
    for _ in 0..32 {
        let sign = if random.next() & 1 == 0 { "" } else { "-" };
        let whole = random.next() % 1_000_000;
        let fraction = random.next() % 1_000_000;
        let exponent = (random.next() % 41) as i32 - 20;
        compare_main(format!("{sign}{whole}.{fraction:06}e{exponent}\n").as_bytes());
    }
}

#[test]
fn config_3_main_valid_token_with_trailing_bytes() {
    let mut random = XorShift32::new(0x1234_abcd);
    for _ in 0..32 {
        let whole = random.next() % 1_000_000;
        let fraction = random.next() % 1_000_000;
        let suffix_len = (random.next() % 24 + 1) as usize;
        let suffix: String = (0..suffix_len)
            .map(|_| (b'a' + (random.next() % 26) as u8) as char)
            .collect();
        compare_main(format!("{whole}.{fraction:06}{suffix}\n").as_bytes());
    }
}

#[test]
fn config_4_main_failed_float_conversion() {
    let mut random = XorShift32::new(0x0bad_f00d);
    for _ in 0..32 {
        let whitespace = " ".repeat((random.next() % 8) as usize);
        let invalid_len = (random.next() % 24 + 1) as usize;
        let invalid: String = (0..invalid_len)
            .map(|_| (b'g' + (random.next() % 20) as u8) as char)
            .collect();
        compare_main(format!("{whitespace}{invalid}\n").as_bytes());
    }
}

#[test]
fn config_5_main_eof_before_conversion() {
    let mut random = XorShift32::new(0xfeed_face);
    compare_main(&[]);
    for _ in 0..31 {
        let length = (random.next() % 32 + 1) as usize;
        let input: Vec<u8> = (0..length)
            .map(|_| match random.next() % 3 {
                0 => b' ',
                1 => b'\t',
                _ => b'\n',
            })
            .collect();
        compare_main(&input);
    }
}

#[test]
fn dynamic_symbol_surface_matches() {
    let c_output = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(c_library())
        .output()
        .expect("inspect C symbols");
    assert_command_succeeded("inspect C symbols", &c_output);
    let rust_output = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(rust_library())
        .output()
        .expect("inspect Rust symbols");
    assert_command_succeeded("inspect Rust symbols", &rust_output);

    let symbols = |bytes: &[u8]| {
        String::from_utf8_lossy(bytes)
            .lines()
            .filter_map(|line| line.split_whitespace().nth(2))
            .map(str::to_owned)
            .collect::<Vec<_>>()
    };
    assert_eq!(symbols(&c_output.stdout), symbols(&rust_output.stdout));
}
