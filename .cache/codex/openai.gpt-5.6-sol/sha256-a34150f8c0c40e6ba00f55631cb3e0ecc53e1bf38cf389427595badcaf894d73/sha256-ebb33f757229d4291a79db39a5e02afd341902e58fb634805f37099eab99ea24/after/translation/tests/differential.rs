use libloading::Library;
use std::ffi::{CString, c_char, c_float, c_int, c_void};
use std::fs::{OpenOptions, remove_file};
use std::io::{Read, Seek, SeekFrom};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard};

type PrintLine = unsafe extern "C" fn(*const c_char);
type PrintIntLine = unsafe extern "C" fn(c_int);
type Bad = unsafe extern "C" fn(c_float);
type Good = unsafe extern "C" fn(c_float);
type Driver = unsafe extern "C" fn(c_float, c_float);

unsafe extern "C" {
    fn fflush(stream: *mut c_void) -> c_int;
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
}

struct Api {
    _library: Library,
    print_line: PrintLine,
    print_int_line: PrintIntLine,
    bad: Bad,
    good: Good,
    driver: Driver,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let print_line = unsafe { *library.get::<PrintLine>(b"printLine\0").unwrap() };
        let print_int_line = unsafe { *library.get::<PrintIntLine>(b"printIntLine\0").unwrap() };
        let bad = unsafe { *library.get::<Bad>(b"bad\0").unwrap() };
        let good = unsafe { *library.get::<Good>(b"good\0").unwrap() };
        let driver = unsafe { *library.get::<Driver>(b"driver\0").unwrap() };
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

static STDOUT_LOCK: Mutex<()> = Mutex::new(());
static CAPTURE_ID: AtomicU64 = AtomicU64::new(0);

struct Harness {
    _guard: MutexGuard<'static, ()>,
    c: Api,
    rust: Api,
}

impl Harness {
    fn new() -> Self {
        let guard = STDOUT_LOCK
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_path = root.join("../c_src/build/libdriver.so");
        let rust_path = std::env::var_os("RUST_DRIVER_SO")
            .map(PathBuf::from)
            .unwrap_or_else(|| root.join("target/release/libdriver.so"));
        assert!(
            c_path.is_file(),
            "missing C shared library: {}",
            c_path.display()
        );
        assert!(
            rust_path.is_file(),
            "missing Rust shared library: {}; run cargo build --release first",
            rust_path.display()
        );
        Self {
            _guard: guard,
            c: unsafe { Api::load(&c_path) },
            rust: unsafe { Api::load(&rust_path) },
        }
    }

    fn compare<F>(&self, row: &str, calls: F)
    where
        F: Fn(&Api),
    {
        let c_output = capture_stdout(|| calls(&self.c));
        let rust_output = capture_stdout(|| calls(&self.rust));
        assert_eq!(
            c_output,
            rust_output,
            "{row} output mismatch\nC: {:?}\nRust: {:?}",
            String::from_utf8_lossy(&c_output),
            String::from_utf8_lossy(&rust_output)
        );
    }
}

fn capture_stdout<F>(call: F) -> Vec<u8>
where
    F: FnOnce(),
{
    let id = CAPTURE_ID.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "driver-differential-{}-{id}.out",
        std::process::id()
    ));
    let mut file = OpenOptions::new()
        .create_new(true)
        .read(true)
        .write(true)
        .open(&path)
        .unwrap();
    let saved_stdout;
    unsafe {
        assert_eq!(fflush(std::ptr::null_mut()), 0);
        saved_stdout = dup(1);
        assert!(saved_stdout >= 0);
        assert_eq!(dup2(file.as_raw_fd(), 1), 1);
    }

    call();

    unsafe {
        assert_eq!(fflush(std::ptr::null_mut()), 0);
        assert_eq!(dup2(saved_stdout, 1), 1);
        assert_eq!(close(saved_stdout), 0);
    }
    file.seek(SeekFrom::Start(0)).unwrap();
    let mut output = Vec::new();
    file.read_to_end(&mut output).unwrap();
    drop(file);
    remove_file(path).unwrap();
    output
}

#[derive(Clone)]
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn next_u32(&mut self) -> u32 {
        self.next_u64() as u32
    }
}

fn random_i32s(seed: u64, count: usize, predicate: impl Fn(i32) -> bool) -> Vec<i32> {
    let mut rng = Rng::new(seed);
    let mut values = Vec::with_capacity(count);
    while values.len() < count {
        let value = rng.next_u32() as i32;
        if predicate(value) {
            values.push(value);
        }
    }
    values
}

fn random_f32s(seed: u64, count: usize, predicate: impl Fn(f32) -> bool) -> Vec<f32> {
    let mut rng = Rng::new(seed);
    let mut values = Vec::with_capacity(count);
    while values.len() < count {
        let value = f32::from_bits(rng.next_u32());
        if predicate(value) {
            values.push(value);
        }
    }
    values
}

fn repeat<T: Copy>(values: &[T], count: usize) -> Vec<T> {
    (0..count)
        .map(|index| values[index % values.len()])
        .collect()
}

#[test]
fn c01_print_line_empty() {
    let h = Harness::new();
    let empty = CString::new("").unwrap();
    h.compare("C1", |api| {
        for _ in 0..128 {
            unsafe { (api.print_line)(empty.as_ptr()) };
        }
    });
}

#[test]
fn c02_print_line_random_nonempty() {
    let h = Harness::new();
    let mut rng = Rng::new(0xC02);
    let strings: Vec<CString> = (0..256)
        .map(|_| {
            let length = 1 + (rng.next_u32() as usize % 96);
            let bytes: Vec<u8> = (0..length)
                .map(|_| 0x20 + (rng.next_u32() % 0x5f) as u8)
                .collect();
            CString::new(bytes).unwrap()
        })
        .collect();
    h.compare("C2", |api| {
        for value in &strings {
            unsafe { (api.print_line)(value.as_ptr()) };
        }
    });
}

#[test]
fn c03_print_int_negative() {
    let h = Harness::new();
    let mut values = random_i32s(0xC03, 255, |value| value < 0);
    values.push(i32::MIN);
    h.compare("C3", |api| {
        for &value in &values {
            unsafe { (api.print_int_line)(value) };
        }
    });
}

#[test]
fn c04_print_int_zero() {
    let h = Harness::new();
    h.compare("C4", |api| {
        for _ in 0..128 {
            unsafe { (api.print_int_line)(0) };
        }
    });
}

#[test]
fn c05_print_int_positive() {
    let h = Harness::new();
    let mut values = random_i32s(0xC05, 255, |value| value > 0);
    values.push(i32::MAX);
    h.compare("C5", |api| {
        for &value in &values {
            unsafe { (api.print_int_line)(value) };
        }
    });
}

#[test]
fn c06_bad_positive_in_range() {
    let h = Harness::new();
    let values = random_f32s(0xC06, 512, |value| {
        if !(value.is_finite() && value > 0.0) {
            return false;
        }
        let quotient = 100.0 / f64::from(value);
        (1.0..=f64::from(i32::MAX)).contains(&quotient)
    });
    h.compare("C6", |api| {
        for &value in &values {
            unsafe { (api.bad)(value) };
        }
    });
}

#[test]
fn c07_bad_negative_in_range() {
    let h = Harness::new();
    let values = random_f32s(0xC07, 512, |value| {
        if !(value.is_finite() && value < 0.0) {
            return false;
        }
        let quotient = 100.0 / f64::from(value);
        (f64::from(i32::MIN)..=-1.0).contains(&quotient)
    });
    h.compare("C7", |api| {
        for &value in &values {
            unsafe { (api.bad)(value) };
        }
    });
}

#[test]
fn c08_bad_finite_conversion_overflow() {
    let h = Harness::new();
    let mut values = Vec::new();
    for bits in 1..=256_u32 {
        values.push(f32::from_bits(bits));
        values.push(f32::from_bits(bits | 0x8000_0000));
    }
    values.extend([
        4.656_612_5e-8_f32,
        -4.656_612_5e-8_f32,
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
    ]);
    h.compare("C8", |api| {
        for &value in &values {
            unsafe { (api.bad)(value) };
        }
    });
}

#[test]
fn c09_bad_signed_zero() {
    let h = Harness::new();
    let values = repeat(&[0.0_f32, -0.0_f32], 256);
    h.compare("C9", |api| {
        for &value in &values {
            unsafe { (api.bad)(value) };
        }
    });
}

#[test]
fn c10_bad_nonfinite() {
    let h = Harness::new();
    let mut values = repeat(&[f32::INFINITY, f32::NEG_INFINITY], 128);
    for payload in 1..=256_u32 {
        values.push(f32::from_bits(0x7f80_0000 | payload));
        values.push(f32::from_bits(0xff80_0000 | payload));
    }
    h.compare("C10", |api| {
        for &value in &values {
            unsafe { (api.bad)(value) };
        }
    });
}

#[test]
fn c11_good_positive_accepted() {
    let h = Harness::new();
    let values = random_f32s(0xC11, 512, |value| value.is_finite() && value > 0.000001);
    h.compare("C11", |api| {
        for &value in &values {
            unsafe { (api.good)(value) };
        }
    });
}

#[test]
fn c12_good_negative_accepted() {
    let h = Harness::new();
    let values = random_f32s(0xC12, 512, |value| value.is_finite() && value < -0.000001);
    h.compare("C12", |api| {
        for &value in &values {
            unsafe { (api.good)(value) };
        }
    });
}

#[test]
fn c13_good_threshold_adjacent_accepted() {
    let h = Harness::new();
    let positive = f32::from_bits(0.000001_f32.to_bits() + 1);
    let negative = -positive;
    let values = repeat(&[positive, negative], 256);
    h.compare("C13", |api| {
        for &value in &values {
            unsafe { (api.good)(value) };
        }
    });
}

#[test]
fn c14_good_infinite() {
    let h = Harness::new();
    let values = repeat(&[f32::INFINITY, f32::NEG_INFINITY], 256);
    h.compare("C14", |api| {
        for &value in &values {
            unsafe { (api.good)(value) };
        }
    });
}

fn driver_values(seed: u64, count: usize, predicate: impl Fn(f32) -> bool) -> Vec<f32> {
    random_f32s(seed, count, predicate)
}

#[test]
fn c15_driver_positive_in_range() {
    let h = Harness::new();
    let good = driver_values(0xC150, 256, |value| value.is_finite() && value > 0.000001);
    let bad = driver_values(0xC151, 256, |value| {
        value.is_finite()
            && value > 0.0
            && (1.0..=f64::from(i32::MAX)).contains(&(100.0 / f64::from(value)))
    });
    h.compare("C15", |api| {
        for (&good, &bad) in good.iter().zip(&bad) {
            unsafe { (api.driver)(good, bad) };
        }
    });
}

#[test]
fn c16_driver_negative_in_range() {
    let h = Harness::new();
    let good = driver_values(0xC160, 256, |value| value.is_finite() && value < -0.000001);
    let bad = driver_values(0xC161, 256, |value| {
        value.is_finite()
            && value < 0.0
            && (f64::from(i32::MIN)..=-1.0).contains(&(100.0 / f64::from(value)))
    });
    h.compare("C16", |api| {
        for (&good, &bad) in good.iter().zip(&bad) {
            unsafe { (api.driver)(good, bad) };
        }
    });
}

#[test]
fn c17_driver_threshold_and_overflow() {
    let h = Harness::new();
    let positive = f32::from_bits(0.000001_f32.to_bits() + 1);
    let negative = -positive;
    let good = repeat(&[positive, negative], 256);
    let bad: Vec<f32> = (1..=256_u32)
        .map(|bits| {
            let value = f32::from_bits(bits);
            if bits % 2 == 0 { value } else { -value }
        })
        .collect();
    h.compare("C17", |api| {
        for (&good, &bad) in good.iter().zip(&bad) {
            unsafe { (api.driver)(good, bad) };
        }
    });
}

#[test]
fn c18_driver_nonfinite() {
    let h = Harness::new();
    let good = repeat(&[f32::INFINITY, f32::NEG_INFINITY], 256);
    let bad: Vec<f32> = (1..=256_u32)
        .map(|payload| f32::from_bits(0x7f80_0000 | payload))
        .collect();
    h.compare("C18", |api| {
        for (&good, &bad) in good.iter().zip(&bad) {
            unsafe { (api.driver)(good, bad) };
        }
    });
}

#[test]
fn c19_driver_rejected_near_zero_continues() {
    let h = Harness::new();
    let mut good = random_f32s(0xC19, 254, |value| {
        value.is_finite() && f64::from(value).abs() <= 0.000001
    });
    good.extend([0.000001_f32, -0.000001_f32]);
    let bad = random_f32s(0xC191, 256, |value| {
        if !(value.is_finite() && value != 0.0) {
            return false;
        }
        let quotient = 100.0 / f64::from(value);
        (f64::from(i32::MIN)..=f64::from(i32::MAX)).contains(&quotient)
    });
    h.compare("C19", |api| {
        for (&good, &bad) in good.iter().zip(&bad) {
            unsafe { (api.driver)(good, bad) };
        }
    });
}

#[test]
fn c20_driver_nan_and_signed_zero() {
    let h = Harness::new();
    let good: Vec<f32> = (1..=256_u32)
        .map(|payload| f32::from_bits(0x7f80_0000 | payload))
        .collect();
    let bad = repeat(&[0.0_f32, -0.0_f32], 256);
    h.compare("C20", |api| {
        for (&good, &bad) in good.iter().zip(&bad) {
            unsafe { (api.driver)(good, bad) };
        }
    });
}

#[test]
fn e01_print_line_null() {
    let h = Harness::new();
    h.compare("E1", |api| {
        for _ in 0..128 {
            unsafe { (api.print_line)(std::ptr::null()) };
        }
    });
}

#[test]
fn e02_good_rejection_branch() {
    let h = Harness::new();
    let mut values = random_f32s(0xE02, 512, |value| {
        value.is_finite() && f64::from(value).abs() <= 0.000001
    });
    values.extend([
        0.0,
        -0.0,
        0.000001_f32,
        -0.000001_f32,
        f32::from_bits(0x7fc0_0001),
        f32::from_bits(0xffc0_0001),
    ]);
    h.compare("E2", |api| {
        for &value in &values {
            unsafe { (api.good)(value) };
        }
    });
}

#[test]
fn e03_driver_rejection_branch() {
    let h = Harness::new();
    let mut good = random_f32s(0xE03, 512, |value| {
        value.is_finite() && f64::from(value).abs() <= 0.000001
    });
    good.extend([
        0.0,
        -0.0,
        0.000001_f32,
        -0.000001_f32,
        f32::from_bits(0x7fc0_0001),
        f32::from_bits(0xffc0_0001),
    ]);
    let bad = random_f32s(0xE031, good.len(), |value| {
        value.is_finite() && value != 0.0
    });
    h.compare("E3", |api| {
        for (&good, &bad) in good.iter().zip(&bad) {
            unsafe { (api.driver)(good, bad) };
        }
    });
}

#[test]
fn exported_symbols_load_from_both_libraries() {
    let _h = Harness::new();
}
