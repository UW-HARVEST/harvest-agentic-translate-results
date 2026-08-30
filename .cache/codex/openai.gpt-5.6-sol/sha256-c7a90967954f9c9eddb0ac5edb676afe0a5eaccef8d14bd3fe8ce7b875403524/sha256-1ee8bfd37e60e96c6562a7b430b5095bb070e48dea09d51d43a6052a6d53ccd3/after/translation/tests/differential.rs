use libloading::{Library, Symbol};
use std::ffi::{CString, c_char, c_int, c_void};
use std::io;
use std::os::fd::RawFd;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct House {
    floors: c_int,
    bedrooms: c_int,
    bathrooms: f64,
}

type DriverFn = unsafe extern "C" fn(*const c_char);
type RunFn = unsafe extern "C" fn(*mut House, c_int);

static STDOUT_LOCK: Mutex<()> = Mutex::new(());

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(fds: *mut c_int) -> c_int;
    fn read(fd: c_int, buffer: *mut c_void, count: usize) -> isize;
}

#[derive(Clone, Copy)]
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

    fn range_u32(&mut self, start: u32, end: u32) -> u32 {
        start + (self.next_u64() % u64::from(end - start)) as u32
    }

    fn ascii_letters(&mut self, length: usize) -> String {
        (0..length)
            .map(|_| (b'a' + self.range_u32(0, 26) as u8) as char)
            .collect()
    }
}

fn c_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/build/libdriver.so")
}

fn rust_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("target/release/libdriver.so")
}

fn check_syscall(result: c_int, operation: &str) -> c_int {
    if result == -1 {
        panic!("{operation} failed: {}", io::Error::last_os_error());
    }
    result
}

fn capture_stdout(call: impl FnOnce()) -> Vec<u8> {
    let _guard = STDOUT_LOCK.lock().expect("stdout capture lock");
    let mut descriptors: [RawFd; 2] = [-1, -1];

    unsafe {
        assert_eq!(fflush(std::ptr::null_mut()), 0, "fflush before capture");
        check_syscall(pipe(descriptors.as_mut_ptr()), "pipe");
        let saved_stdout = check_syscall(dup(1), "dup");
        check_syscall(dup2(descriptors[1], 1), "dup2 capture");
        check_syscall(close(descriptors[1]), "close pipe writer");

        call();

        assert_eq!(fflush(std::ptr::null_mut()), 0, "fflush after call");
        check_syscall(dup2(saved_stdout, 1), "dup2 restore");
        check_syscall(close(saved_stdout), "close saved stdout");

        let mut output = Vec::new();
        let mut buffer = [0_u8; 4096];
        loop {
            let count = read(descriptors[0], buffer.as_mut_ptr().cast(), buffer.len());
            if count == 0 {
                break;
            }
            if count < 0 {
                panic!("read failed: {}", io::Error::last_os_error());
            }
            output.extend_from_slice(&buffer[..count as usize]);
        }
        check_syscall(close(descriptors[0]), "close pipe reader");
        output
    }
}

unsafe fn load_driver(library: &Library) -> Symbol<'_, DriverFn> {
    unsafe { library.get(b"driver\0").expect("load driver symbol") }
}

unsafe fn load_run(library: &Library) -> Symbol<'_, RunFn> {
    unsafe { library.get(b"run\0").expect("load run symbol") }
}

fn compare_driver(c_driver: DriverFn, rust_driver: DriverFn, input: &str) {
    let input = CString::new(input).expect("test input contains no NUL");
    let c_output = capture_stdout(|| unsafe { c_driver(input.as_ptr()) });
    let rust_output = capture_stdout(|| unsafe { rust_driver(input.as_ptr()) });
    assert_eq!(
        rust_output, c_output,
        "driver stdout differs for input {input:?}"
    );
}

fn compare_run(c_run: RunFn, rust_run: RunFn, initial: House, extra_bedrooms: i32) {
    let mut c_house = initial;
    let c_output = capture_stdout(|| unsafe { c_run(&mut c_house, extra_bedrooms) });
    let mut rust_house = initial;
    let rust_output = capture_stdout(|| unsafe { rust_run(&mut rust_house, extra_bedrooms) });

    assert_eq!(
        rust_output, c_output,
        "run stdout differs for {initial:?}, extra={extra_bedrooms}"
    );
    assert_eq!(rust_house.floors, c_house.floors, "floors differ");
    assert_eq!(rust_house.bedrooms, c_house.bedrooms, "bedrooms differ");
    assert_eq!(
        rust_house.bathrooms.to_bits(),
        c_house.bathrooms.to_bits(),
        "bathroom bits differ"
    );
}

#[test]
fn valid_configuration_surface_matches() {
    let c_library = unsafe { Library::new(c_library_path()).expect("load C library") };
    let rust_library = unsafe { Library::new(rust_library_path()).expect("load Rust library") };
    let c_driver = unsafe { *load_driver(&c_library) };
    let rust_driver = unsafe { *load_driver(&rust_library) };
    let c_run = unsafe { *load_run(&c_library) };
    let rust_run = unsafe { *load_run(&rust_library) };
    let mut rng = Rng::new(0xd1ff_e2e5_2025_0001);

    // CONFIGS.md row 1: randomized direct low-level calls.
    for iteration in 0..256 {
        let extra = match iteration % 3 {
            0 => -(rng.range_u32(1, 1_000_001) as i32),
            1 => 0,
            _ => rng.range_u32(1, 1_000_001) as i32,
        };
        let house = House {
            floors: rng.range_u32(0, i32::MAX as u32) as i32 - 1_000_000_000,
            bedrooms: rng.range_u32(0, 2_000_001) as i32 - 1_000_000,
            bathrooms: f64::from(rng.next_i32()) / 10.0,
        };
        compare_run(c_run, rust_run, house, extra);
    }

    // CONFIGS.md row 2: defined integer edges and floating-point shapes.
    let boundary_cases = [
        (i32::MIN, i32::MIN, 0, -0.0_f64),
        (i32::MAX - 1, i32::MAX, 0, 0.0),
        (0, i32::MIN, i32::MAX, 0.05),
        (0, i32::MAX, i32::MIN, -0.05),
        (1, 0, i32::MAX, f64::INFINITY),
        (-1, 0, i32::MIN, f64::NEG_INFINITY),
        (100, -100, 100, f64::NAN),
        (-100, 100, -100, f64::from_bits(0x7ff8_1234_5678_9abc)),
    ];
    for &(floors, bedrooms, extra, bathrooms) in &boundary_cases {
        compare_run(
            c_run,
            rust_run,
            House {
                floors,
                bedrooms,
                bathrooms,
            },
            extra,
        );
    }
    for _ in 0..128 {
        let (floors, bedrooms, extra, _) =
            boundary_cases[rng.range_u32(0, boundary_cases.len() as u32) as usize];
        let bathrooms = match rng.range_u32(0, 5) {
            0 => f64::from_bits(0x7ff8_0000_0000_0000 | (rng.next_u64() >> 13)),
            1 => f64::INFINITY,
            2 => f64::NEG_INFINITY,
            3 => -0.0,
            _ => f64::from(rng.next_i32()) / 16.0,
        };
        compare_run(
            c_run,
            rust_run,
            House {
                floors,
                bedrooms,
                bathrooms,
            },
            extra,
        );
    }

    // CONFIGS.md rows 3, 6, and 7: randomized forms of exact parsed values.
    for _ in 0..96 {
        let zeroes = "0".repeat(rng.range_u32(1, 20) as usize);
        let zero_sign = ["", "+", "-"][rng.range_u32(0, 3) as usize];
        compare_driver(c_driver, rust_driver, &format!("{zero_sign}{zeroes}"));

        let padding = "0".repeat(rng.range_u32(0, 12) as usize);
        let suffix_length = rng.range_u32(1, 8) as usize;
        let suffix = rng.ascii_letters(suffix_length);
        compare_driver(
            c_driver,
            rust_driver,
            &format!("-{padding}2147483648{suffix}"),
        );

        let padding = "0".repeat(rng.range_u32(0, 12) as usize);
        let suffix_length = rng.range_u32(1, 8) as usize;
        let suffix = rng.ascii_letters(suffix_length);
        compare_driver(
            c_driver,
            rust_driver,
            &format!("+{padding}2147483647{suffix}"),
        );
    }

    // CONFIGS.md rows 4 and 5: randomized positive and negative decimals.
    for _ in 0..128 {
        let positive = rng.range_u32(1, i32::MAX as u32) as i32;
        let negative = -(rng.range_u32(1, i32::MAX as u32) as i32);
        compare_driver(c_driver, rust_driver, &positive.to_string());
        compare_driver(c_driver, rust_driver, &negative.to_string());
    }

    // CONFIGS.md row 8: all C-locale whitespace forms before valid decimals.
    let whitespace = [" ", "\t", "\n", "\r", "\x0b", "\x0c"];
    for _ in 0..96 {
        let value = rng.next_i32();
        let prefix = whitespace[rng.range_u32(0, whitespace.len() as u32) as usize]
            .repeat(rng.range_u32(1, 5) as usize);
        compare_driver(c_driver, rust_driver, &format!("{prefix}{value}"));
    }

    // CONFIGS.md row 9: explicit signs and randomized leading zero counts.
    for _ in 0..96 {
        let value = rng.range_u32(0, i32::MAX as u32);
        let zeroes = "0".repeat(rng.range_u32(1, 12) as usize);
        compare_driver(c_driver, rust_driver, &format!("+{zeroes}{value}"));
    }

    // CONFIGS.md row 10: trailing data is accepted after a valid prefix.
    for _ in 0..128 {
        let value = rng.next_i32();
        let suffix_length = rng.range_u32(1, 33) as usize;
        let suffix = rng.ascii_letters(suffix_length);
        compare_driver(c_driver, rust_driver, &format!("{value}{suffix}"));
    }
}

#[test]
fn error_surface_matches() {
    let c_library = unsafe { Library::new(c_library_path()).expect("load C library") };
    let rust_library = unsafe { Library::new(rust_library_path()).expect("load Rust library") };
    let c_driver = unsafe { *load_driver(&c_library) };
    let rust_driver = unsafe { *load_driver(&rust_library) };
    let mut rng = Rng::new(0xe220_5eed_0000_0002);

    // ERRORS.md row 1: no conversion.
    for fixed in ["", " ", "\t\r\n", "+", "-", "  +", "  -"] {
        compare_driver(c_driver, rust_driver, fixed);
    }
    for _ in 0..128 {
        let prefix = if rng.next_u64() & 1 == 0 { "" } else { " +" };
        let letter_count = rng.range_u32(1, 65) as usize;
        let letters = rng.ascii_letters(letter_count);
        compare_driver(c_driver, rust_driver, &format!("{prefix}{letters}"));
    }

    // ERRORS.md row 2: strtol sets ERANGE for magnitudes outside long.
    for fixed in ["9223372036854775808", "-9223372036854775809"] {
        compare_driver(c_driver, rust_driver, fixed);
    }
    for _ in 0..128 {
        let sign = if rng.next_u64() & 1 == 0 { "" } else { "-" };
        let length = rng.range_u32(32, 257) as usize;
        let first = char::from(b'1' + rng.range_u32(0, 9) as u8);
        let tail: String = (1..length)
            .map(|_| char::from(b'0' + rng.range_u32(0, 10) as u8))
            .collect();
        compare_driver(c_driver, rust_driver, &format!("{sign}{first}{tail}"));
    }

    // ERRORS.md row 3: converted long is below INT_MIN without ERANGE.
    compare_driver(c_driver, rust_driver, "-2147483649");
    for _ in 0..128 {
        let distance = rng.range_u32(1, u32::MAX) as i64;
        let value = i64::from(i32::MIN) - distance;
        compare_driver(c_driver, rust_driver, &value.to_string());
    }

    // ERRORS.md row 4: converted long is above INT_MAX without ERANGE.
    compare_driver(c_driver, rust_driver, "2147483648");
    for _ in 0..128 {
        let distance = rng.range_u32(1, u32::MAX) as i64;
        let value = i64::from(i32::MAX) + distance;
        compare_driver(c_driver, rust_driver, &value.to_string());
    }
}

fn crash_status(library: &str, function: &str) -> std::process::ExitStatus {
    Command::new(std::env::current_exe().expect("current test executable"))
        .args(["--exact", "ffi_crash_probe", "--nocapture"])
        .env("DRIVER_CRASH_PROBE", format!("{library}:{function}"))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("run crash probe")
}

#[test]
fn generic_null_pointer_boundaries_match() {
    for function in ["driver", "run"] {
        let c_status = crash_status("c", function);
        let rust_status = crash_status("rust", function);
        assert!(
            !c_status.success(),
            "C {function}(NULL) unexpectedly returned"
        );
        assert_eq!(
            rust_status.signal(),
            c_status.signal(),
            "{function}(NULL) terminated with different signals"
        );
    }
}

#[test]
fn ffi_crash_probe() {
    let Ok(probe) = std::env::var("DRIVER_CRASH_PROBE") else {
        return;
    };
    let (implementation, function) = probe.split_once(':').expect("probe format");
    let path = match implementation {
        "c" => c_library_path(),
        "rust" => rust_library_path(),
        _ => panic!("unknown implementation"),
    };
    let library = unsafe { Library::new(path).expect("load probe library") };

    match function {
        "driver" => unsafe {
            let driver = load_driver(&library);
            driver(std::ptr::null());
        },
        "run" => unsafe {
            let run = load_run(&library);
            run(std::ptr::null_mut(), 0);
        },
        _ => panic!("unknown probe function"),
    }
}
