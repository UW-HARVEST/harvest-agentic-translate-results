use libloading::Library;
use std::ffi::{CString, c_char, c_float, c_int, c_void};
use std::fs::File;
use std::io::Read;
use std::os::fd::FromRawFd;
use std::path::PathBuf;
use std::ptr;
use std::sync::Mutex;

type PrintLine = unsafe extern "C" fn(*const c_char);
type PrintIntLine = unsafe extern "C" fn(c_int);
type FloatFunction = unsafe extern "C" fn(c_float);
type Driver = unsafe extern "C" fn(c_float, c_float);

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(pipe_fds: *mut c_int) -> c_int;
}

static STDOUT_LOCK: Mutex<()> = Mutex::new(());

struct Libraries {
    c: Library,
    rust: Library,
}

impl Libraries {
    fn load() -> Self {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_path = manifest.join("../c_src/build/libdriver.so");
        let rust_path = std::env::var_os("RUST_DRIVER_SO")
            .map(PathBuf::from)
            .unwrap_or_else(|| manifest.join("target/release/libdriver.so"));

        assert!(
            c_path.is_file(),
            "missing C shared library: {}",
            c_path.display()
        );
        assert!(
            rust_path.is_file(),
            "missing Rust shared library: {}",
            rust_path.display()
        );

        unsafe {
            Self {
                c: Library::new(c_path).expect("load C shared library"),
                rust: Library::new(rust_path).expect("load Rust shared library"),
            }
        }
    }

    fn print_line(&self, line: *const c_char) -> (Vec<u8>, Vec<u8>) {
        unsafe {
            let c_fn = *self.c.get::<PrintLine>(b"printLine\0").unwrap();
            let rust_fn = *self.rust.get::<PrintLine>(b"printLine\0").unwrap();
            (
                capture_stdout(|| c_fn(line)),
                capture_stdout(|| rust_fn(line)),
            )
        }
    }

    fn print_int_line(&self, value: c_int) -> (Vec<u8>, Vec<u8>) {
        unsafe {
            let c_fn = *self.c.get::<PrintIntLine>(b"printIntLine\0").unwrap();
            let rust_fn = *self.rust.get::<PrintIntLine>(b"printIntLine\0").unwrap();
            (
                capture_stdout(|| c_fn(value)),
                capture_stdout(|| rust_fn(value)),
            )
        }
    }

    fn bad(&self, data: c_float) -> (Vec<u8>, Vec<u8>) {
        unsafe {
            let c_fn = *self.c.get::<FloatFunction>(b"bad\0").unwrap();
            let rust_fn = *self.rust.get::<FloatFunction>(b"bad\0").unwrap();
            (
                capture_stdout(|| c_fn(data)),
                capture_stdout(|| rust_fn(data)),
            )
        }
    }

    fn good(&self, data: c_float) -> (Vec<u8>, Vec<u8>) {
        unsafe {
            let c_fn = *self.c.get::<FloatFunction>(b"good\0").unwrap();
            let rust_fn = *self.rust.get::<FloatFunction>(b"good\0").unwrap();
            (
                capture_stdout(|| c_fn(data)),
                capture_stdout(|| rust_fn(data)),
            )
        }
    }

    fn driver(&self, good_data: c_float, bad_data: c_float) -> (Vec<u8>, Vec<u8>) {
        unsafe {
            let c_fn = *self.c.get::<Driver>(b"driver\0").unwrap();
            let rust_fn = *self.rust.get::<Driver>(b"driver\0").unwrap();
            (
                capture_stdout(|| c_fn(good_data, bad_data)),
                capture_stdout(|| rust_fn(good_data, bad_data)),
            )
        }
    }
}

unsafe fn capture_stdout(call: impl FnOnce()) -> Vec<u8> {
    let mut pipe_fds = [-1; 2];
    assert_eq!(
        unsafe { fflush(ptr::null_mut()) },
        0,
        "flush before capture"
    );
    assert_eq!(
        unsafe { pipe(pipe_fds.as_mut_ptr()) },
        0,
        "create stdout pipe"
    );
    let saved_stdout = unsafe { dup(1) };
    assert!(saved_stdout >= 0, "duplicate stdout");
    assert_eq!(unsafe { dup2(pipe_fds[1], 1) }, 1, "redirect stdout");
    assert_eq!(unsafe { close(pipe_fds[1]) }, 0, "close duplicate writer");

    call();

    assert_eq!(
        unsafe { fflush(ptr::null_mut()) },
        0,
        "flush captured output"
    );
    assert_eq!(unsafe { dup2(saved_stdout, 1) }, 1, "restore stdout");
    assert_eq!(unsafe { close(saved_stdout) }, 0, "close saved stdout");

    let mut output = Vec::new();
    let mut reader = unsafe { File::from_raw_fd(pipe_fds[0]) };
    reader
        .read_to_end(&mut output)
        .expect("read captured stdout");
    output
}

fn assert_same(context: &str, outputs: (Vec<u8>, Vec<u8>)) -> Vec<u8> {
    assert_eq!(outputs.0, outputs.1, "{context}");
    outputs.0
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u32(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0 as u32
    }

    fn float_with_exponent(&mut self, min: u32, max: u32, negative: bool) -> f32 {
        let exponent = min + self.next_u32() % (max - min + 1);
        let sign = if negative { 1_u32 << 31 } else { 0 };
        f32::from_bits(sign | (exponent << 23) | (self.next_u32() & 0x7f_ffff))
    }

    fn quiet_nan(&mut self) -> f32 {
        f32::from_bits(0x7fc0_0000 | (self.next_u32() & 0x003f_ffff))
    }
}

#[test]
fn valid_print_configs_rows_1_to_6() {
    let _lock = STDOUT_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x51a7_1001);

    let empty = CString::new("").unwrap();
    assert_same(
        "CONFIGS row 1: empty string",
        libraries.print_line(empty.as_ptr()),
    );

    for case in 0..64 {
        let length = 1 + (rng.next_u32() % 96) as usize;
        let bytes: Vec<u8> = (0..length)
            .map(|_| b' ' + (rng.next_u32() % 95) as u8)
            .collect();
        let string = CString::new(bytes).unwrap();
        assert_same(
            &format!("CONFIGS row 2: non-empty string case {case}"),
            libraries.print_line(string.as_ptr()),
        );
    }

    for case in 0..64 {
        let negative = -((rng.next_u32() % 2_147_483_647) as i32 + 1);
        assert_same(
            &format!("CONFIGS row 3: negative integer case {case}"),
            libraries.print_int_line(negative),
        );
    }
    assert_same("CONFIGS row 4: zero", libraries.print_int_line(0));
    for case in 0..64 {
        let positive = (rng.next_u32() % 2_147_483_647) as i32 + 1;
        assert_same(
            &format!("CONFIGS row 5: positive integer case {case}"),
            libraries.print_int_line(positive),
        );
    }
    for value in [i32::MIN, i32::MAX] {
        assert_same(
            "CONFIGS row 6: integer boundary",
            libraries.print_int_line(value),
        );
    }
}

#[test]
fn valid_bad_configs_rows_7_to_11() {
    let _lock = STDOUT_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x51a7_2002);

    for case in 0..64 {
        assert_same(
            &format!("CONFIGS row 7: positive in-range quotient case {case}"),
            libraries.bad(rng.float_with_exponent(103, 254, false)),
        );
        assert_same(
            &format!("CONFIGS row 8: negative in-range quotient case {case}"),
            libraries.bad(rng.float_with_exponent(103, 254, true)),
        );
    }
    for value in [0.0_f32, -0.0_f32] {
        assert_same("CONFIGS row 9: signed zero", libraries.bad(value));
    }
    for case in 0..64 {
        let negative = rng.next_u32() & 1 != 0;
        assert_same(
            &format!("CONFIGS row 10: quotient overflow case {case}"),
            libraries.bad(rng.float_with_exponent(0, 101, negative)),
        );
    }
    for value in [f32::INFINITY, f32::NEG_INFINITY] {
        assert_same("CONFIGS row 11: infinity", libraries.bad(value));
    }
    for case in 0..64 {
        let value = rng.quiet_nan();
        assert_same(
            &format!("CONFIGS row 11: NaN payload case {case}"),
            libraries.bad(value),
        );
    }
}

#[test]
fn valid_good_configs_rows_12_to_16() {
    let _lock = STDOUT_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x51a7_3003);

    for case in 0..64 {
        assert_same(
            &format!("CONFIGS row 12: accepted positive case {case}"),
            libraries.good(rng.float_with_exponent(108, 254, false)),
        );
        assert_same(
            &format!("CONFIGS row 13: accepted negative case {case}"),
            libraries.good(rng.float_with_exponent(108, 254, true)),
        );
    }

    let threshold = 0.000001_f32;
    for value in [
        0.0,
        -0.0,
        threshold,
        -threshold,
        f32::from_bits(threshold.to_bits() - 1),
        -f32::from_bits(threshold.to_bits() - 1),
    ] {
        assert_same(
            "CONFIGS row 14: rejected finite boundary",
            libraries.good(value),
        );
    }
    for case in 0..64 {
        let negative = rng.next_u32() & 1 != 0;
        assert_same(
            &format!("CONFIGS row 14: rejected finite case {case}"),
            libraries.good(rng.float_with_exponent(0, 106, negative)),
        );
    }
    for case in 0..64 {
        assert_same(
            &format!("CONFIGS row 15: NaN payload case {case}"),
            libraries.good(rng.quiet_nan()),
        );
    }
    for value in [f32::INFINITY, f32::NEG_INFINITY] {
        assert_same("CONFIGS row 16: infinity", libraries.good(value));
    }
}

fn special_bad_value(rng: &mut Rng, case: u32) -> f32 {
    match case % 5 {
        0 => 0.0,
        1 => -0.0,
        2 => rng.float_with_exponent(0, 101, case & 1 != 0),
        3 => {
            if case & 1 == 0 {
                f32::INFINITY
            } else {
                f32::NEG_INFINITY
            }
        }
        _ => rng.quiet_nan(),
    }
}

#[test]
fn valid_driver_configs_rows_17_to_23() {
    let _lock = STDOUT_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x51a7_4004);

    for case in 0..64_u32 {
        let bad_negative = rng.next_u32() & 1 != 0;
        let ordinary_bad = rng.float_with_exponent(103, 254, bad_negative);
        assert_same(
            &format!("CONFIGS row 17: positive good case {case}"),
            libraries.driver(rng.float_with_exponent(108, 254, false), ordinary_bad),
        );
        assert_same(
            &format!("CONFIGS row 18: negative good case {case}"),
            libraries.driver(rng.float_with_exponent(108, 254, true), ordinary_bad),
        );
        assert_same(
            &format!("CONFIGS row 19: rejected finite good case {case}"),
            libraries.driver(rng.float_with_exponent(0, 106, case & 1 != 0), ordinary_bad),
        );
        assert_same(
            &format!("CONFIGS row 20: NaN good case {case}"),
            libraries.driver(rng.quiet_nan(), ordinary_bad),
        );
        let infinite_good = if case & 1 == 0 {
            f32::INFINITY
        } else {
            f32::NEG_INFINITY
        };
        assert_same(
            &format!("CONFIGS row 21: infinite good case {case}"),
            libraries.driver(infinite_good, ordinary_bad),
        );
        let special_bad = special_bad_value(&mut rng, case);
        assert_same(
            &format!("CONFIGS row 22: accepted good and special bad case {case}"),
            libraries.driver(
                rng.float_with_exponent(108, 254, case & 1 != 0),
                special_bad,
            ),
        );
        let rejected_good = if case & 1 == 0 {
            rng.quiet_nan()
        } else {
            rng.float_with_exponent(0, 106, case & 2 != 0)
        };
        assert_same(
            &format!("CONFIGS row 23: rejected good and special bad case {case}"),
            libraries.driver(rejected_good, special_bad),
        );
    }
}

#[test]
fn error_rows_and_generic_boundaries() {
    let _lock = STDOUT_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x51a7_5005);

    let output = assert_same(
        "ERRORS row 1: null printLine pointer",
        libraries.print_line(ptr::null()),
    );
    assert_eq!(output, b"");

    let threshold = 0.000001_f32;
    for value in [
        0.0,
        -0.0,
        threshold,
        -threshold,
        f32::from_bits(threshold.to_bits() - 1),
        -f32::from_bits(threshold.to_bits() - 1),
    ] {
        let output = assert_same("ERRORS row 2: finite rejection", libraries.good(value));
        assert_eq!(output, b"50\nThis would result in a divide by zero\n");
    }
    for case in 0..64 {
        let output = assert_same(
            &format!("ERRORS row 2: unordered NaN case {case}"),
            libraries.good(rng.quiet_nan()),
        );
        assert_eq!(output, b"50\nThis would result in a divide by zero\n");
    }

    for good_data in [0.0, -0.0, threshold, -threshold, rng.quiet_nan()] {
        let output = assert_same(
            "ERRORS row 3: rejected driver goodData",
            libraries.driver(good_data, 2.0),
        );
        assert_eq!(
            output,
            b"Calling good()...\n50\nThis would result in a divide by zero\n\
Finished good()\nCalling bad()...\n50\nFinished bad()\n"
        );
    }

    let next_above = f32::from_bits(threshold.to_bits() + 1);
    let next_below_negative = -next_above;
    for value in [next_above, next_below_negative] {
        assert_same(
            "generic one-step-past threshold boundary",
            libraries.good(value),
        );
    }
    for value in [i32::MIN, 0, i32::MAX] {
        assert_same("generic integer boundary", libraries.print_int_line(value));
    }
}
