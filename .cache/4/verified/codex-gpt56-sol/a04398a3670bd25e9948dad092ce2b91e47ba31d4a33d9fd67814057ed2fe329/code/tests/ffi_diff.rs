use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};
use std::fs::{self, File};
use std::os::fd::AsRawFd;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use std::sync::atomic::{AtomicU64, Ordering};

type Driver = unsafe extern "C" fn(c_int, c_int);

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

const STDOUT_FILENO: c_int = 1;
const CASES_ENV: &str = "DRIVER_DIFF_CASES";
const LIB_ENV: &str = "DRIVER_DIFF_LIB";
const OUTPUT_ENV: &str = "DRIVER_DIFF_OUTPUT";
static OUTPUT_ID: AtomicU64 = AtomicU64::new(0);

struct Run {
    status: ExitStatus,
    bytes: Vec<u8>,
}

fn c_library() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("c_src/build/libdriver.so")
}

fn rust_library() -> PathBuf {
    std::env::current_exe()
        .expect("test executable path")
        .parent()
        .and_then(Path::parent)
        .expect("target profile directory")
        .join("libdriver.so")
}

fn encode_cases(cases: &[(i32, i32)]) -> String {
    cases
        .iter()
        .map(|(x, y)| format!("{x}:{y}"))
        .collect::<Vec<_>>()
        .join(",")
}

fn decode_cases(encoded: &str) -> Vec<(i32, i32)> {
    encoded
        .split(',')
        .filter(|item| !item.is_empty())
        .map(|item| {
            let (x, y) = item.split_once(':').expect("encoded x:y pair");
            (
                x.parse().expect("encoded i32 numerator"),
                y.parse().expect("encoded i32 denominator"),
            )
        })
        .collect()
}

fn run_library(library: &Path, cases: &[(i32, i32)]) -> Run {
    assert!(library.is_file(), "missing library: {}", library.display());

    let id = OUTPUT_ID.fetch_add(1, Ordering::Relaxed);
    let output_path =
        std::env::temp_dir().join(format!("driver-ffi-diff-{}-{id}.out", std::process::id()));
    let status = Command::new(std::env::current_exe().expect("test executable path"))
        .arg("--exact")
        .arg("ffi_child")
        .arg("--ignored")
        .arg("--nocapture")
        .env(LIB_ENV, library)
        .env(CASES_ENV, encode_cases(cases))
        .env(OUTPUT_ENV, &output_path)
        .status()
        .expect("run isolated FFI child");
    let bytes = fs::read(&output_path).unwrap_or_default();
    let _ = fs::remove_file(output_path);
    Run { status, bytes }
}

fn assert_matching_success(label: &str, cases: &[(i32, i32)]) {
    let c = run_library(&c_library(), cases);
    let rust = run_library(&rust_library(), cases);

    assert!(c.status.success(), "{label}: C failed with {:?}", c.status);
    assert!(
        rust.status.success(),
        "{label}: Rust failed with {:?}",
        rust.status
    );
    assert_eq!(rust.bytes, c.bytes, "{label}: stdout differs");
}

fn assert_matching_signal(label: &str, input: (i32, i32)) {
    let c = run_library(&c_library(), &[input]);
    let rust = run_library(&rust_library(), &[input]);

    assert_eq!(c.status.signal(), Some(8), "{label}: C signal changed");
    assert_eq!(
        rust.status.signal(),
        c.status.signal(),
        "{label}: termination signal differs"
    );
    assert_eq!(rust.bytes, c.bytes, "{label}: pre-signal stdout differs");
}

#[derive(Clone, Copy)]
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x as u32
    }

    fn positive_bounded(&mut self, max: i32) -> i32 {
        (self.next_u32() % max as u32 + 1) as i32
    }
}

fn zero_cases(rng: &mut Rng, negative_denominator: bool) -> Vec<(i32, i32)> {
    (0..128)
        .map(|_| {
            let magnitude = rng.positive_bounded(i32::MAX);
            (
                0,
                if negative_denominator {
                    -magnitude
                } else {
                    magnitude
                },
            )
        })
        .collect()
}

fn exact_cases(rng: &mut Rng, x_negative: bool, y_negative: bool) -> Vec<(i32, i32)> {
    (0..128)
        .map(|_| {
            let y_magnitude = rng.positive_bounded(32_000);
            let quotient_magnitude = rng.positive_bounded(32_000);
            let x_magnitude = y_magnitude * quotient_magnitude;
            (
                if x_negative {
                    -x_magnitude
                } else {
                    x_magnitude
                },
                if y_negative {
                    -y_magnitude
                } else {
                    y_magnitude
                },
            )
        })
        .collect()
}

fn nonexact_cases(rng: &mut Rng, x_negative: bool, y_negative: bool) -> Vec<(i32, i32)> {
    (0..128)
        .map(|_| {
            let y_magnitude = rng.positive_bounded(1_000_000).max(2);
            let quotient = rng.positive_bounded(1_000);
            let remainder = rng.positive_bounded(y_magnitude - 1);
            let x_magnitude = y_magnitude * quotient + remainder;
            (
                if x_negative {
                    -x_magnitude
                } else {
                    x_magnitude
                },
                if y_negative {
                    -y_magnitude
                } else {
                    y_magnitude
                },
            )
        })
        .collect()
}

#[test]
fn configuration_surface_matches() {
    let mut rng = Rng::new(0x5eed_c0de_d15c_a11e);

    assert_matching_success("config 1: zero/positive", &zero_cases(&mut rng, false));
    assert_matching_success("config 2: zero/negative", &zero_cases(&mut rng, true));
    assert_matching_success(
        "config 3: positive/positive exact",
        &exact_cases(&mut rng, false, false),
    );
    assert_matching_success(
        "config 4: positive/positive nonexact",
        &nonexact_cases(&mut rng, false, false),
    );
    assert_matching_success(
        "config 5: positive/negative exact",
        &exact_cases(&mut rng, false, true),
    );
    assert_matching_success(
        "config 6: positive/negative nonexact",
        &nonexact_cases(&mut rng, false, true),
    );
    assert_matching_success(
        "config 7: negative/positive exact",
        &exact_cases(&mut rng, true, false),
    );
    assert_matching_success(
        "config 8: negative/positive nonexact",
        &nonexact_cases(&mut rng, true, false),
    );

    let mut negative_negative = exact_cases(&mut rng, true, true);
    negative_negative.extend(nonexact_cases(&mut rng, true, true));
    assert_matching_success("config 9: negative/negative", &negative_negative);

    let mut boundaries = vec![
        (i32::MIN, 1),
        (i32::MIN, 2),
        (i32::MIN, i32::MAX),
        (i32::MIN, i32::MIN),
        (i32::MAX, 1),
        (i32::MAX, -1),
        (i32::MAX, i32::MAX),
        (i32::MAX, i32::MIN),
        (1, i32::MIN),
        (-1, i32::MIN),
    ];
    for _ in 0..128 {
        let mut denominator = rng.next_u32() as i32;
        if denominator == 0 || denominator == -1 {
            denominator = 1;
        }
        boundaries.push((i32::MIN, denominator));

        let mut denominator = rng.next_u32() as i32;
        if denominator == 0 {
            denominator = 1;
        }
        boundaries.push((i32::MAX, denominator));

        let numerator = rng.next_u32() as i32;
        let denominator = if rng.next_u32() & 1 == 0 {
            i32::MIN
        } else {
            i32::MAX
        };
        boundaries.push((numerator, denominator));
    }
    assert_matching_success("config 10: integer boundaries", &boundaries);
}

#[test]
fn error_surface_matches() {
    for x in [0, 1, -1, 17, i32::MIN, i32::MAX] {
        assert_matching_signal("error 1: zero denominator", (x, 0));
    }
    assert_matching_signal("error 2: signed division overflow", (i32::MIN, -1));
}

#[test]
#[ignore]
fn ffi_child() {
    let library = std::env::var_os(LIB_ENV).expect("child library path");
    let cases = decode_cases(&std::env::var(CASES_ENV).expect("child cases"));
    let output_path = std::env::var_os(OUTPUT_ENV).expect("child output path");
    let output = File::create(output_path).expect("create child output");

    unsafe {
        let library = Library::new(library).expect("load shared library");
        let driver: Symbol<Driver> = library.get(b"driver\0").expect("load driver symbol");

        fflush(std::ptr::null_mut());
        let saved_stdout = dup(STDOUT_FILENO);
        assert!(saved_stdout >= 0, "duplicate stdout");
        assert_eq!(dup2(output.as_raw_fd(), STDOUT_FILENO), STDOUT_FILENO);

        for (x, y) in cases {
            driver(x, y);
        }

        fflush(std::ptr::null_mut());
        assert_eq!(dup2(saved_stdout, STDOUT_FILENO), STDOUT_FILENO);
        assert_eq!(close(saved_stdout), 0);
    }
}
