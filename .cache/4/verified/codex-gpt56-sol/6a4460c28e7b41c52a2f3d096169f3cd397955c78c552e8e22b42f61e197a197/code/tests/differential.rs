use libloading::{Library, Symbol};
use std::env;
use std::ffi::{c_int, c_void};
use std::io::Write;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

unsafe extern "C" {
    fn fflush(stream: *mut c_void) -> c_int;
}

type EntryPoint = unsafe extern "C" fn() -> c_int;

#[derive(Debug)]
struct Outcome {
    code: Option<i32>,
    signal: Option<i32>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

#[test]
fn ffi_child() {
    let Some(path) = env::var_os("DRIVER_FFI_LIBRARY") else {
        return;
    };

    unsafe {
        let library = Library::new(path).expect("load shared library");
        let main: Symbol<EntryPoint> = library.get(b"main\0").expect("load main");
        let code = main();
        fflush(std::ptr::null_mut());
        std::process::exit(code);
    }
}

fn c_library() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libdriver.so")
        .canonicalize()
        .expect("C shared library is missing; build it before running tests")
}

fn rust_library() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target/debug/libdriver.so")
        .canonicalize()
        .expect("Rust shared library is missing; run cargo build --lib")
}

fn run_library(library: &Path, input: &[u8]) -> Outcome {
    let mut child = Command::new(env::current_exe().expect("current test executable"))
        .args(["--exact", "ffi_child", "--nocapture", "--test-threads=1"])
        .env("DRIVER_FFI_LIBRARY", library)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn FFI child");

    child
        .stdin
        .take()
        .expect("child stdin")
        .write_all(input)
        .expect("write child input");

    let output = child.wait_with_output().expect("wait for FFI child");
    Outcome {
        code: output.status.code(),
        signal: output.status.signal(),
        stdout: output.stdout,
        stderr: output.stderr,
    }
}

fn assert_same(row: usize, input: &[u8]) -> Outcome {
    let c = run_library(&c_library(), input);
    let rust = run_library(&rust_library(), input);

    assert_eq!(
        (c.code, c.signal),
        (rust.code, rust.signal),
        "CONFIGS/ERRORS row {row}, status mismatch for input {:?}",
        String::from_utf8_lossy(input)
    );
    assert_eq!(
        c.stdout,
        rust.stdout,
        "CONFIGS/ERRORS row {row}, stdout mismatch for input {:?}",
        String::from_utf8_lossy(input)
    );
    assert_eq!(
        c.stderr,
        rust.stderr,
        "CONFIGS/ERRORS row {row}, stderr mismatch for input {:?}",
        String::from_utf8_lossy(input)
    );

    c
}

fn assert_success(row: usize, input: &[u8]) {
    let outcome = assert_same(row, input);
    assert_eq!(outcome.code, Some(0), "CONFIGS row {row}");
    assert_eq!(outcome.signal, None, "CONFIGS row {row}");
    assert!(
        outcome
            .stdout
            .windows(b"quotient: ".len())
            .any(|window| window == b"quotient: "),
        "CONFIGS row {row} did not exercise the program output"
    );
}

struct Lcg(u64);

impl Lcg {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u32(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 32) as u32
    }

    fn positive(&mut self) -> i32 {
        ((self.next_u32() & 0x7fff_ffff).max(1)) as i32
    }

    fn negative(&mut self) -> i32 {
        -self.positive()
    }
}

#[test]
fn all_configuration_rows_match() {
    let mut rng = Lcg::new(0x5eed_c0de_d15c_a11e);

    // Row 1: scanf reaches EOF before making an assignment.
    for _ in 0..16 {
        let whitespace = match rng.next_u32() % 4 {
            0 => "",
            1 => " ",
            2 => "\n\t",
            _ => " \r\n\t ",
        };
        assert_success(1, whitespace.as_bytes());
    }

    // Row 2: the first conversion is non-numeric.
    for _ in 0..16 {
        let input = format!("x{:08x}\n", rng.next_u32());
        assert_success(2, input.as_bytes());
    }

    // Row 3: one integer is assigned before EOF.
    for _ in 0..24 {
        let input = format!("{}", rng.next_u32() as i32);
        assert_success(3, input.as_bytes());
    }

    // Row 4: one integer is assigned and the second conversion fails.
    for _ in 0..24 {
        let input = format!("{} invalid\n", rng.next_u32() as i32);
        assert_success(4, input.as_bytes());
    }

    // Rows 5-8: all sign combinations for two nonzero integers.
    for _ in 0..24 {
        let pp = format!("{} {}\n", rng.positive(), rng.positive());
        assert_success(5, pp.as_bytes());

        let pn = format!("{} {}\n", rng.positive(), rng.negative());
        assert_success(6, pn.as_bytes());

        let np = format!("{} {}\n", rng.negative(), rng.positive());
        assert_success(7, np.as_bytes());

        let nn = format!("{} {}\n", rng.negative(), rng.negative());
        assert_success(8, nn.as_bytes());
    }

    // Row 9: a zero numerator with randomized nonzero divisors.
    for _ in 0..24 {
        let divisor = if rng.next_u32() & 1 == 0 {
            rng.positive()
        } else {
            rng.negative()
        };
        let input = format!("0 {divisor}\n");
        assert_success(9, input.as_bytes());
    }

    // Row 10: defined signed-integer boundary pairs.
    let boundaries = [
        (i32::MIN, 1),
        (i32::MIN, 2),
        (i32::MIN, i32::MAX),
        (i32::MAX, 1),
        (i32::MAX, -1),
        (i32::MAX, i32::MIN),
        (1, i32::MIN),
        (-1, i32::MIN),
    ];
    for &(numerator, divisor) in &boundaries {
        let input = format!("{numerator} {divisor}\n");
        assert_success(10, input.as_bytes());
    }

    // Row 11: scanf whitespace directives accept every C whitespace class.
    let separators = [" ", "\t", "\n", "\r", "\u{000b}", "\u{000c}", " \t\r\n "];
    for _ in 0..24 {
        let separator = separators[rng.next_u32() as usize % separators.len()];
        let input = format!(
            "{separator}{}{separator}{}{separator}",
            rng.next_u32() as i32,
            rng.positive()
        );
        assert_success(11, input.as_bytes());
    }

    // Row 12: scanf stops after two assignments and leaves trailing data unread.
    for _ in 0..24 {
        let input = format!(
            "{} {} trailing-{:08x} {}\n",
            rng.next_u32() as i32,
            rng.positive(),
            rng.next_u32(),
            rng.next_u32()
        );
        assert_success(12, input.as_bytes());
    }
}

#[test]
fn zero_divisor_error_row_matches() {
    let mut rng = Lcg::new(0xf1f0_0000_5eed_0001);

    for _ in 0..16 {
        let input = format!("{} 0\n", rng.next_u32() as i32);
        let outcome = assert_same(1, input.as_bytes());
        assert_eq!(outcome.code, None, "ERRORS row 1 unexpectedly returned");
        assert_eq!(outcome.signal, Some(8), "ERRORS row 1 must raise SIGFPE");
    }
}
