use libloading::Library;
use std::ffi::{CString, c_char, c_int, c_void};
use std::fs::OpenOptions;
use std::io::{Read, Seek, SeekFrom};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

type PrintLine = unsafe extern "C" fn(*const c_char);
type PrintIntLine = unsafe extern "C" fn(c_int);
type Unary = unsafe extern "C" fn(c_int);
type Driver = unsafe extern "C" fn(c_int, c_int);

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

struct Api {
    _library: Library,
    print_line: PrintLine,
    print_int_line: PrintIntLine,
    bad: Unary,
    good: Unary,
    driver: Driver,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let print_line = unsafe { *library.get(b"printLine\0").unwrap() };
        let print_int_line = unsafe { *library.get(b"printIntLine\0").unwrap() };
        let bad = unsafe { *library.get(b"bad\0").unwrap() };
        let good = unsafe { *library.get(b"good\0").unwrap() };
        let driver = unsafe { *library.get(b"driver\0").unwrap() };

        Self {
            _library: library,
            print_line,
            print_int_line,
            bad,
            good,
            driver,
        }
    }
}

fn library_paths() -> (PathBuf, PathBuf) {
    let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    (
        crate_root.join("../c_src/build/libdriver.so"),
        crate_root.join("target/release/libdriver.so"),
    )
}

fn capture_stdout(call: impl FnOnce()) -> Vec<u8> {
    static NEXT_FILE: AtomicU64 = AtomicU64::new(0);

    let path = std::env::temp_dir().join(format!(
        "driver-differential-{}-{}",
        std::process::id(),
        NEXT_FILE.fetch_add(1, Ordering::Relaxed)
    ));
    let mut output = OpenOptions::new()
        .create_new(true)
        .read(true)
        .write(true)
        .open(&path)
        .unwrap();

    unsafe {
        assert_eq!(fflush(std::ptr::null_mut()), 0);
        let saved_stdout = dup(1);
        assert!(saved_stdout >= 0);
        assert_eq!(dup2(output.as_raw_fd(), 1), 1);

        call();

        assert_eq!(fflush(std::ptr::null_mut()), 0);
        assert_eq!(dup2(saved_stdout, 1), 1);
        assert_eq!(close(saved_stdout), 0);
    }

    output.seek(SeekFrom::Start(0)).unwrap();
    let mut bytes = Vec::new();
    output.read_to_end(&mut bytes).unwrap();
    drop(output);
    std::fs::remove_file(path).unwrap();
    bytes
}

fn compare(label: &str, c_call: impl FnOnce(), rust_call: impl FnOnce()) -> Vec<u8> {
    let c_output = capture_stdout(c_call);
    let rust_output = capture_stdout(rust_call);
    assert_eq!(rust_output, c_output, "{label}");
    c_output
}

fn one_hot(index: i32) -> Vec<u8> {
    let mut output = Vec::new();
    for position in 0..10 {
        output.extend_from_slice(if position == index { b"1\n" } else { b"0\n" });
    }
    output
}

fn expected_good(data: i32) -> Vec<u8> {
    let mut output = one_hot(7);
    if (0..10).contains(&data) {
        output.extend(one_hot(data));
    } else {
        output.extend_from_slice(b"ERROR: Array index is out-of-bounds\n");
    }
    output
}

fn expected_bad(data: i32) -> Vec<u8> {
    if data < 0 {
        b"ERROR: Array index is negative.\n".to_vec()
    } else {
        one_hot(data)
    }
}

fn expected_driver(good_data: i32, bad_data: i32) -> Vec<u8> {
    let mut output = b"Calling good()...\n".to_vec();
    output.extend(expected_good(good_data));
    output.extend_from_slice(b"Finished good()\nCalling bad()...\n");
    output.extend(expected_bad(bad_data));
    output.extend_from_slice(b"Finished bad()\n");
    output
}

struct Rng(u64);

impl Rng {
    fn next_u32(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0 as u32
    }

    fn i32(&mut self) -> i32 {
        self.next_u32() as i32
    }

    fn usize_below(&mut self, upper: usize) -> usize {
        self.next_u32() as usize % upper
    }
}

#[test]
fn all_defined_configuration_and_error_rows_match() {
    let (c_path, rust_path) = library_paths();
    assert!(c_path.is_file(), "missing {}", c_path.display());
    assert!(
        rust_path.is_file(),
        "missing {}; run cargo build --release first",
        rust_path.display()
    );

    let c = unsafe { Api::load(&c_path) };
    let rust = unsafe { Api::load(&rust_path) };
    let mut rng = Rng(0x6a09_e667_f3bc_c909);

    // C1: non-null strings, including empty and non-empty values.
    for iteration in 0..128 {
        let length = if iteration == 0 {
            0
        } else {
            rng.usize_below(65)
        };
        let bytes: Vec<u8> = (0..length)
            .map(|_| 1 + rng.usize_below(126) as u8)
            .collect();
        let text = CString::new(bytes).unwrap();
        let output = compare(
            &format!("C1 iteration {iteration}"),
            || unsafe { (c.print_line)(text.as_ptr()) },
            || unsafe { (rust.print_line)(text.as_ptr()) },
        );
        assert_eq!(output.last(), Some(&b'\n'));
    }

    // C2: arbitrary C ints and both integer boundaries.
    let mut integers = vec![i32::MIN, -1, 0, 1, i32::MAX];
    integers.extend((0..128).map(|_| rng.i32()));
    for value in integers {
        let output = compare(
            &format!("C2 value {value}"),
            || unsafe { (c.print_int_line)(value) },
            || unsafe { (rust.print_int_line)(value) },
        );
        assert_eq!(output, format!("{value}\n").as_bytes());
    }

    // C3 and C4: all safe indices, repeatedly sampled with a fixed seed.
    for iteration in 0..128 {
        let index = if iteration < 10 {
            iteration
        } else {
            rng.usize_below(10)
        } as i32;
        let bad_output = compare(
            &format!("C3 iteration {iteration}, index {index}"),
            || unsafe { (c.bad)(index) },
            || unsafe { (rust.bad)(index) },
        );
        assert_eq!(bad_output, one_hot(index));

        let good_output = compare(
            &format!("C4 iteration {iteration}, index {index}"),
            || unsafe { (c.good)(index) },
            || unsafe { (rust.good)(index) },
        );
        assert_eq!(good_output, expected_good(index));
    }

    // C5: full valid cross-product, plus repeated fixed-seed samples.
    let mut driver_inputs: Vec<(i32, i32)> = (0..10)
        .flat_map(|good_data| (0..10).map(move |bad_data| (good_data, bad_data)))
        .collect();
    driver_inputs
        .extend((0..128).map(|_| (rng.usize_below(10) as i32, rng.usize_below(10) as i32)));
    for (good_data, bad_data) in driver_inputs {
        let output = compare(
            &format!("C5 goodData={good_data}, badData={bad_data}"),
            || unsafe { (c.driver)(good_data, bad_data) },
            || unsafe { (rust.driver)(good_data, bad_data) },
        );
        assert_eq!(output, expected_driver(good_data, bad_data));
    }

    // E1: the explicit null-pointer no-op.
    let output = compare(
        "E1 printLine(NULL)",
        || unsafe { (c.print_line)(std::ptr::null()) },
        || unsafe { (rust.print_line)(std::ptr::null()) },
    );
    assert!(output.is_empty());

    // E2: every negative value takes bad's exact rejection branch.
    let mut negatives = vec![i32::MIN, -1];
    negatives.extend((0..64).map(|_| rng.i32() | i32::MIN));
    for data in negatives {
        let output = compare(
            &format!("E2 data={data}"),
            || unsafe { (c.bad)(data) },
            || unsafe { (rust.bad)(data) },
        );
        assert_eq!(output, expected_bad(data));
    }

    // E3 and E4: each false term of goodB2G's compound range check.
    let mut low_indices = vec![i32::MIN, -1];
    low_indices.extend((0..64).map(|_| rng.i32() | i32::MIN));
    for data in low_indices {
        let output = compare(
            &format!("E3 data={data}"),
            || unsafe { (c.good)(data) },
            || unsafe { (rust.good)(data) },
        );
        assert_eq!(output, expected_good(data));
    }
    let mut high_indices = vec![10, 11, i32::MAX];
    high_indices.extend((0..64).map(|_| 10 + rng.usize_below(10_000) as i32));
    for data in high_indices {
        let output = compare(
            &format!("E4 data={data}"),
            || unsafe { (c.good)(data) },
            || unsafe { (rust.good)(data) },
        );
        assert_eq!(output, expected_good(data));
    }

    // E5-E7: rejection behavior through the composed public entry point.
    for iteration in 0..64 {
        let bad_data = rng.usize_below(10) as i32;
        let negative_good = if iteration == 0 {
            i32::MIN
        } else {
            rng.i32() | i32::MIN
        };
        let output = compare(
            &format!("E5 goodData={negative_good}, badData={bad_data}"),
            || unsafe { (c.driver)(negative_good, bad_data) },
            || unsafe { (rust.driver)(negative_good, bad_data) },
        );
        assert_eq!(output, expected_driver(negative_good, bad_data));

        let high_good = if iteration == 0 {
            i32::MAX
        } else {
            10 + rng.usize_below(10_000) as i32
        };
        let output = compare(
            &format!("E6 goodData={high_good}, badData={bad_data}"),
            || unsafe { (c.driver)(high_good, bad_data) },
            || unsafe { (rust.driver)(high_good, bad_data) },
        );
        assert_eq!(output, expected_driver(high_good, bad_data));

        let good_data = rng.usize_below(10) as i32;
        let negative_bad = if iteration == 0 {
            i32::MIN
        } else {
            rng.i32() | i32::MIN
        };
        let output = compare(
            &format!("E7 goodData={good_data}, badData={negative_bad}"),
            || unsafe { (c.driver)(good_data, negative_bad) },
            || unsafe { (rust.driver)(good_data, negative_bad) },
        );
        assert_eq!(output, expected_driver(good_data, negative_bad));

        let output = compare(
            &format!("combined errors goodData={negative_good}, badData={negative_bad}"),
            || unsafe { (c.driver)(negative_good, negative_bad) },
            || unsafe { (rust.driver)(negative_good, negative_bad) },
        );
        assert_eq!(output, expected_driver(negative_good, negative_bad));
    }
}

#[test]
fn one_past_bad_boundary_matches_in_isolated_processes() {
    let (c_path, rust_path) = library_paths();
    let executable = std::env::current_exe().unwrap();

    let run = |path: &Path| {
        Command::new(&executable)
            .args(["--exact", "boundary_child", "--nocapture"])
            .env("DRIVER_BOUNDARY_LIBRARY", path)
            .output()
            .unwrap()
    };

    let c = run(&c_path);
    let rust = run(&rust_path);
    assert_eq!(rust.status, c.status);
    assert_eq!(rust.stdout, c.stdout);
    assert_eq!(rust.stderr, c.stderr);
}

#[test]
fn boundary_child() {
    let Some(path) = std::env::var_os("DRIVER_BOUNDARY_LIBRARY") else {
        return;
    };
    let api = unsafe { Api::load(Path::new(&path)) };
    unsafe {
        (api.bad)(10);
        fflush(std::ptr::null_mut());
    }
}
