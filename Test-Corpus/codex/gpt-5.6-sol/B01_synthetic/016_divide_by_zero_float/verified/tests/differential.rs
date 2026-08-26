use libc::{c_char, c_int};
use libloading::{Library, Symbol};
use std::ffi::CString;
use std::fs::OpenOptions;
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

type PrintLine = unsafe extern "C" fn(*const c_char);
type PrintIntLine = unsafe extern "C" fn(c_int);
type NoArg = unsafe extern "C" fn();
type Main = unsafe extern "C" fn(c_int, *mut *mut c_char) -> c_int;

unsafe extern "C" {
    #[link_name = "stdin"]
    static mut C_STDIN: *mut libc::FILE;
    #[link_name = "stdout"]
    static mut C_STDOUT: *mut libc::FILE;
    fn __fpurge(stream: *mut libc::FILE);
}

static TEMP_ID: AtomicU64 = AtomicU64::new(0);

struct Api {
    library: Library,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        Self {
            library: Library::new(path)
                .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display())),
        }
    }

    fn print_line(&self, value: Option<&CString>) {
        unsafe {
            let function: Symbol<PrintLine> = self.library.get(b"printLine").unwrap();
            function(value.map_or(std::ptr::null(), |value| value.as_ptr()));
        }
    }

    fn print_int_line(&self, value: c_int) {
        unsafe {
            let function: Symbol<PrintIntLine> = self.library.get(b"printIntLine").unwrap();
            function(value);
        }
    }

    fn bad(&self) {
        unsafe {
            let function: Symbol<NoArg> = self.library.get(b"bad").unwrap();
            function();
        }
    }

    fn good(&self) {
        unsafe {
            let function: Symbol<NoArg> = self.library.get(b"good").unwrap();
            function();
        }
    }

    fn main(&self, argc: c_int, with_argv: bool) -> c_int {
        unsafe {
            let function: Symbol<Main> = self.library.get(b"main").unwrap();
            if with_argv {
                let arg0 = CString::new("driver").unwrap();
                let arg1 = CString::new("ignored").unwrap();
                let mut argv = vec![
                    arg0.as_ptr().cast_mut(),
                    arg1.as_ptr().cast_mut(),
                    std::ptr::null_mut(),
                ];
                function(argc, argv.as_mut_ptr())
            } else {
                function(argc, std::ptr::null_mut())
            }
        }
    }
}

#[derive(Clone, Copy)]
enum InputMode<'a> {
    Bytes(&'a [u8]),
    ReadError,
}

struct Capture {
    output: Vec<u8>,
    result: c_int,
}

fn temp_path(label: &str) -> PathBuf {
    let id = TEMP_ID.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "driver-differential-{}-{}-{label}",
        std::process::id(),
        id
    ))
}

fn capture(mode: InputMode<'_>, call: impl FnOnce() -> c_int) -> Capture {
    let input_path = temp_path("input");
    let output_path = temp_path("output");

    match mode {
        InputMode::Bytes(bytes) => {
            let mut input = OpenOptions::new()
                .read(true)
                .write(true)
                .create_new(true)
                .open(&input_path)
                .unwrap();
            input.write_all(bytes).unwrap();
            input.seek(SeekFrom::Start(0)).unwrap();
            drop(input);
        }
        InputMode::ReadError => {
            drop(
                OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&input_path)
                    .unwrap(),
            );
        }
    }

    let input = match mode {
        InputMode::Bytes(_) => OpenOptions::new().read(true).open(&input_path).unwrap(),
        InputMode::ReadError => OpenOptions::new().write(true).open(&input_path).unwrap(),
    };
    let mut output = OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(&output_path)
        .unwrap();

    unsafe {
        libc::fflush(std::ptr::null_mut());
        __fpurge(C_STDIN);
        libc::clearerr(C_STDIN);

        let saved_stdin = libc::dup(libc::STDIN_FILENO);
        let saved_stdout = libc::dup(libc::STDOUT_FILENO);
        assert!(saved_stdin >= 0 && saved_stdout >= 0);
        assert_eq!(libc::dup2(input.as_raw_fd(), libc::STDIN_FILENO), 0);
        assert_eq!(libc::dup2(output.as_raw_fd(), libc::STDOUT_FILENO), 1);
        libc::clearerr(C_STDIN);
        libc::clearerr(C_STDOUT);

        let result = call();
        libc::fflush(std::ptr::null_mut());

        assert_eq!(libc::dup2(saved_stdin, libc::STDIN_FILENO), 0);
        assert_eq!(libc::dup2(saved_stdout, libc::STDOUT_FILENO), 1);
        libc::close(saved_stdin);
        libc::close(saved_stdout);
        __fpurge(C_STDIN);
        libc::clearerr(C_STDIN);
        libc::clearerr(C_STDOUT);

        output.seek(SeekFrom::Start(0)).unwrap();
        let mut bytes = Vec::new();
        output.read_to_end(&mut bytes).unwrap();
        drop(input);
        drop(output);
        let _ = std::fs::remove_file(input_path);
        let _ = std::fs::remove_file(output_path);

        Capture {
            output: bytes,
            result,
        }
    }
}

fn library_paths() -> (PathBuf, PathBuf) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c = root.join("c_src/build/libdriver_c.so");
    let profile = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let rust = profile.join("libdriver.so");
    assert!(c.is_file(), "missing C shared library: {}", c.display());
    assert!(
        rust.is_file(),
        "missing Rust shared library: {}",
        rust.display()
    );
    (c, rust)
}

fn apis() -> (Api, Api) {
    let (c, rust) = library_paths();
    unsafe { (Api::load(&c), Api::load(&rust)) }
}

fn assert_same(
    label: &str,
    mode: InputMode<'_>,
    c: &Api,
    rust: &Api,
    call: impl Fn(&Api) -> c_int,
) -> Capture {
    let c_result = capture(mode, || call(c));
    let rust_result = capture(mode, || call(rust));
    assert_eq!(
        c_result.result, rust_result.result,
        "{label}: return value differs"
    );
    assert_eq!(
        c_result.output,
        rust_result.output,
        "{label}: output differs\nC: {:?}\nRust: {:?}",
        String::from_utf8_lossy(&c_result.output),
        String::from_utf8_lossy(&rust_result.output)
    );
    c_result
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

    fn finite(&mut self, negative: bool) -> f64 {
        let whole = 1 + self.next_u32() % 50_000;
        let fraction = self.next_u32() % 10_000;
        let value = f64::from(whole) + f64::from(fraction) / 10_000.0;
        if negative {
            -value
        } else {
            value
        }
    }
}

#[test]
fn config_rows_1_to_3_print_helpers() {
    let (c, rust) = apis();
    let mut rng = Lcg::new(0xC011_0001);

    // CONFIGS row 1.
    let empty = CString::new("").unwrap();
    assert_same(
        "printLine empty string",
        InputMode::Bytes(b""),
        &c,
        &rust,
        |api| {
            api.print_line(Some(&empty));
            0
        },
    );

    // CONFIGS row 2: many ordinary and embedded-newline strings.
    for case in 0..64 {
        let len = 1 + (rng.next_u32() % 48) as usize;
        let mut value = String::with_capacity(len + 8);
        for index in 0..len {
            if case & 1 == 1 && index == len / 2 {
                value.push('\n');
            } else {
                value.push(char::from(b'a' + (rng.next_u32() % 26) as u8));
            }
        }
        let value = CString::new(value).unwrap();
        assert_same(
            "printLine valid string",
            InputMode::Bytes(b""),
            &c,
            &rust,
            |api| {
                api.print_line(Some(&value));
                0
            },
        );
    }

    // CONFIGS row 3: explicit boundaries plus randomized bit patterns.
    let mut values = vec![c_int::MIN, -1_000_000, -1, 0, 1, 1_000_000, c_int::MAX];
    for _ in 0..64 {
        values.push(rng.next_u32() as c_int);
    }
    for value in values {
        assert_same(
            "printIntLine signed boundary",
            InputMode::Bytes(b""),
            &c,
            &rust,
            |api| {
                api.print_int_line(value);
                0
            },
        );
    }
}

#[test]
fn config_rows_4_to_12_bad() {
    let (c, rust) = apis();
    let mut rng = Lcg::new(0xBAD0_0004);

    // CONFIGS rows 4-5: many positive and negative finite values.
    for negative in [false, true] {
        for case in 0..64 {
            let input = format!("{:.8}\n", rng.finite(negative));
            assert_same(
                &format!("bad finite case {case}"),
                InputMode::Bytes(input.as_bytes()),
                &c,
                &rust,
                |api| {
                    api.bad();
                    0
                },
            );
        }
    }

    // CONFIGS row 6: randomized EOF-terminated numbers without a newline.
    for case in 0..32 {
        let input = format!("{:.8}", rng.finite(case & 1 == 1));
        assert_same(
            "bad EOF-terminated",
            InputMode::Bytes(input.as_bytes()),
            &c,
            &rust,
            |api| {
                api.bad();
                0
            },
        );
    }

    // CONFIGS rows 7-8: randomized values at and beyond 19 payload bytes.
    for _ in 0..32 {
        let value = 1 + rng.next_u32() % 50_000;
        let exact = format!("{value:019}");
        assert_eq!(exact.len(), 19);
        assert_same(
            "bad exactly 19 bytes",
            InputMode::Bytes(exact.as_bytes()),
            &c,
            &rust,
            |api| {
                api.bad();
                0
            },
        );

        let overlong = format!("{exact}{}ignored\n", rng.next_u32());
        assert_same(
            "bad over 19 bytes",
            InputMode::Bytes(overlong.as_bytes()),
            &c,
            &rust,
            |api| {
                api.bad();
                0
            },
        );
    }

    // CONFIGS row 9: randomized atof syntax classes.
    for case in 0..64 {
        let whole = 1 + rng.next_u32() % 500;
        let fraction = rng.next_u32() % 100;
        let input = match case % 4 {
            0 => format!("   +{whole}.{fraction:02}\n"),
            1 => format!("-{whole}.{fraction:02}\n"),
            2 => format!("{}.{}e-1\n", whole, fraction),
            _ => format!("{whole}.{fraction:02}junk\n"),
        };
        assert_same(
            "bad atof syntax",
            InputMode::Bytes(input.as_bytes()),
            &c,
            &rust,
            |api| {
                api.bad();
                0
            },
        );
    }

    // CONFIGS row 10: randomized forms that parse as zero.
    for case in 0..32 {
        let input = match case % 4 {
            0 => format!("{}0\n", " ".repeat((rng.next_u32() % 4) as usize)),
            1 => "-0.0000\n".to_owned(),
            2 => "\n".to_owned(),
            _ => format!("word{}\n", rng.next_u32()),
        };
        assert_same(
            "bad parsed zero",
            InputMode::Bytes(input.as_bytes()),
            &c,
            &rust,
            |api| {
                api.bad();
                0
            },
        );
    }
    // CONFIGS row 11: randomized case/sign variants of nonfinite tokens.
    let nonfinite = [
        "nan", "NAN", "+nan", "-nan", "inf", "INFINITY", "+inf", "-inf",
    ];
    for _ in 0..32 {
        let token = nonfinite[(rng.next_u32() as usize) % nonfinite.len()];
        let input = format!("{}{token}\n", " ".repeat((rng.next_u32() % 3) as usize));
        assert_same(
            "bad nonfinite",
            InputMode::Bytes(input.as_bytes()),
            &c,
            &rust,
            |api| {
                api.bad();
                0
            },
        );
    }
    // CONFIGS row 12: randomized tiny finite divisors.
    for _ in 0..32 {
        let sign = if rng.next_u32() & 1 == 0 { "" } else { "-" };
        let exponent = 10 + rng.next_u32() % 26;
        let input = format!("{sign}1e-{exponent}\n");
        assert_same(
            "bad quotient outside int",
            InputMode::Bytes(input.as_bytes()),
            &c,
            &rust,
            |api| {
                api.bad();
                0
            },
        );
    }
}

#[test]
fn config_rows_13_to_20_good() {
    let (c, rust) = apis();
    let mut rng = Lcg::new(0x600D_0013);

    // CONFIGS rows 13-14: many values on each side of zero.
    for negative in [false, true] {
        for case in 0..64 {
            let input = format!("{:.8}\n", rng.finite(negative));
            assert_same(
                &format!("good finite case {case}"),
                InputMode::Bytes(input.as_bytes()),
                &c,
                &rust,
                |api| {
                    api.good();
                    0
                },
            );
        }
    }

    // CONFIGS row 15: repeat the nearest practical decimal inputs above
    // epsilon with randomized sign, whitespace, and accepted trailing bytes.
    for case in 0..32 {
        let sign = if rng.next_u32() & 1 == 0 { "" } else { "-" };
        let leading = if rng.next_u32() & 1 == 0 { "" } else { " " };
        let trailing = if case & 1 == 0 { "" } else { "x" };
        let input = format!("{leading}{sign}0.0000010000001{trailing}\n");
        assert_same(
            "good epsilon exterior",
            InputMode::Bytes(input.as_bytes()),
            &c,
            &rust,
            |api| {
                api.good();
                0
            },
        );
    }

    // CONFIGS row 16: randomized EOF-terminated division inputs.
    for case in 0..32 {
        let input = format!("{:.8}", rng.finite(case & 1 == 1));
        assert_same(
            "good EOF-terminated",
            InputMode::Bytes(input.as_bytes()),
            &c,
            &rust,
            |api| {
                api.good();
                0
            },
        );
    }

    // CONFIGS rows 17-18: randomized values at and beyond 19 payload bytes.
    for _ in 0..32 {
        let value = 1 + rng.next_u32() % 50_000;
        let exact = format!("+{value:018}");
        assert_eq!(exact.len(), 19);
        assert_same(
            "good exactly 19 bytes",
            InputMode::Bytes(exact.as_bytes()),
            &c,
            &rust,
            |api| {
                api.good();
                0
            },
        );

        let overlong = format!("{exact}{}ignored\n", rng.next_u32());
        assert_same(
            "good over 19 bytes",
            InputMode::Bytes(overlong.as_bytes()),
            &c,
            &rust,
            |api| {
                api.good();
                0
            },
        );
    }

    // CONFIGS row 19: randomized atof syntax classes in the division branch.
    for case in 0..64 {
        let whole = 1 + rng.next_u32() % 500;
        let fraction = rng.next_u32() % 100;
        let input = match case % 4 {
            0 => format!("   +{whole}.{fraction:02}\n"),
            1 => format!("-{whole}.{fraction:02}\n"),
            2 => format!("{}.{}e-1\n", whole, fraction),
            _ => format!("{whole}.{fraction:02}junk\n"),
        };
        assert_same(
            "good atof syntax",
            InputMode::Bytes(input.as_bytes()),
            &c,
            &rust,
            |api| {
                api.good();
                0
            },
        );
    }

    // CONFIGS row 20: randomized infinity spellings and signs.
    let infinities = ["inf", "INF", "infinity", "INFINITY", "+inf", "-inf"];
    for _ in 0..32 {
        let token = infinities[(rng.next_u32() as usize) % infinities.len()];
        let input = format!("{}{token}\n", " ".repeat((rng.next_u32() % 3) as usize));
        assert_same(
            "good infinity",
            InputMode::Bytes(input.as_bytes()),
            &c,
            &rust,
            |api| {
                api.good();
                0
            },
        );
    }
}

#[test]
fn config_rows_21_to_26_main() {
    let (c, rust) = apis();
    let mut rng = Lcg::new(0xA11C_0021);

    // CONFIGS row 21: null argv and two successful records.
    for case in 0..32 {
        let input = format!("{:.6}\n{:.6}\n", rng.finite(false), rng.finite(true));
        assert_same(
            &format!("main null argv case {case}"),
            InputMode::Bytes(input.as_bytes()),
            &c,
            &rust,
            |api| api.main(0, false),
        );
    }

    // CONFIGS row 22: randomized argc values and records; argv is ignored.
    for _ in 0..32 {
        let argc = rng.next_u32() as c_int;
        let input = format!("{:.6}\n{:.6}\n", rng.finite(false), rng.finite(true));
        assert_same(
            "main non-null argv",
            InputMode::Bytes(input.as_bytes()),
            &c,
            &rust,
            |api| api.main(argc, true),
        );
    }

    // CONFIGS row 23: randomized values inside epsilon followed by bad input.
    for _ in 0..32 {
        let tiny = f64::from(rng.next_u32() % 1_000_001) / 1.0e12;
        let second_is_negative = rng.next_u32() & 1 == 1;
        let second = rng.finite(second_is_negative);
        let input = format!("{tiny:.12}\n{second:.6}\n");
        assert_same(
            "main epsilon then bad",
            InputMode::Bytes(input.as_bytes()),
            &c,
            &rust,
            |api| api.main(0, false),
        );
    }

    // CONFIGS row 24: randomized values in consecutive chunks of one line.
    for _ in 0..32 {
        let first = 1 + rng.next_u32() % 50_000;
        let second = 1 + rng.next_u32() % 50_000;
        let first_chunk = format!("{first:019}");
        assert_eq!(first_chunk.len(), 19);
        let input = format!("{first_chunk}{second}\n");
        assert_same(
            "main one long logical line",
            InputMode::Bytes(input.as_bytes()),
            &c,
            &rust,
            |api| api.main(0, false),
        );
    }

    // CONFIGS row 25: randomized first record followed immediately by EOF.
    for case in 0..32 {
        let input = format!("{:.6}\n", rng.finite(case & 1 == 1));
        assert_same(
            "main EOF after good",
            InputMode::Bytes(input.as_bytes()),
            &c,
            &rust,
            |api| api.main(0, false),
        );
    }

    // CONFIGS row 26: immediate EOF with randomized ignored arguments.
    for _ in 0..32 {
        let argc = rng.next_u32() as c_int;
        let with_argv = rng.next_u32() & 1 == 1;
        assert_same(
            "main immediate EOF",
            InputMode::Bytes(b""),
            &c,
            &rust,
            |api| api.main(argc, with_argv),
        );
    }
}

#[test]
fn error_rows_1_to_4() {
    let (c, rust) = apis();

    // ERRORS row 1 and the generic nullable-pointer boundary.
    let result = assert_same("printLine null", InputMode::Bytes(b""), &c, &rust, |api| {
        api.print_line(None);
        0
    });
    assert_eq!(result.output, b"");

    // ERRORS row 2: both EOF and an actual stream read error.
    for mode in [InputMode::Bytes(b""), InputMode::ReadError] {
        let result = assert_same("bad fgets failure", mode, &c, &rust, |api| {
            api.bad();
            0
        });
        assert_eq!(result.output, b"fgets() failed.\n-2147483648\n");
    }

    // ERRORS row 3: both EOF and an actual stream read error.
    for mode in [InputMode::Bytes(b""), InputMode::ReadError] {
        let result = assert_same("good fgets failure", mode, &c, &rust, |api| {
            api.good();
            0
        });
        assert_eq!(
            result.output,
            b"50\nfgets() failed.\nThis would result in a divide by zero\n"
        );
    }

    // ERRORS row 4: fixed boundaries, NaN, and randomized finite values inside
    // the inclusive epsilon rejection interval.
    let mut inputs = vec![
        "0\n".to_owned(),
        "-0\n".to_owned(),
        "0.000001\n".to_owned(),
        "-0.000001\n".to_owned(),
        "nan\n".to_owned(),
    ];
    let mut rng = Lcg::new(0xE220_0004);
    for _ in 0..64 {
        let magnitude = f64::from(rng.next_u32() % 1_000_001) / 1.0e12;
        let sign = if rng.next_u32() & 1 == 0 { 1.0 } else { -1.0 };
        inputs.push(format!("{:.12}\n", sign * magnitude));
    }
    for input in inputs {
        let result = assert_same(
            "good epsilon rejection",
            InputMode::Bytes(input.as_bytes()),
            &c,
            &rust,
            |api| {
                api.good();
                0
            },
        );
        assert_eq!(
            result.output,
            b"50\nThis would result in a divide by zero\n"
        );
    }
}
