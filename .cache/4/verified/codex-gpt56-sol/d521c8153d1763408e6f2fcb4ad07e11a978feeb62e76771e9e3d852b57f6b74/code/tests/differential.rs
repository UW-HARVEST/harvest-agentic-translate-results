use libloading::{Library, Symbol};
use std::ffi::{CString, c_char, c_double, c_int, c_long, c_void};
use std::fs::{File, OpenOptions};
use std::os::fd::AsRawFd;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::ptr;
use std::sync::Mutex;

type ClassifyMode = unsafe extern "C" fn(*const c_char) -> c_int;
type ApplyMultiplier = unsafe extern "C" fn(c_int, c_int) -> c_int;
type ConvertDouble = unsafe extern "C" fn(c_double) -> c_int;
type GetModifiedTime = unsafe extern "C" fn(c_int, c_int) -> c_long;
type HashTimeValue = unsafe extern "C" fn(c_long) -> c_int;
type Modeselect = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

static STDOUT_LOCK: Mutex<()> = Mutex::new(());
const CASES_PER_SHAPE: usize = 64;
const CASES_PER_MODESELECT_ROW: usize = 16;

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

struct Libraries {
    c: Library,
    rust: Library,
}

impl Libraries {
    unsafe fn load() -> Self {
        Self {
            c: unsafe { Library::new(c_library_path()) }.expect("load C shared library"),
            rust: unsafe { Library::new(rust_library_path()) }.expect("load Rust shared library"),
        }
    }

    unsafe fn symbols<T>(&self, name: &[u8]) -> (Symbol<'_, T>, Symbol<'_, T>) {
        (
            unsafe { self.c.get(name) }.expect("load symbol from C shared library"),
            unsafe { self.rust.get(name) }.expect("load symbol from Rust shared library"),
        )
    }
}

struct StdoutSilencer {
    saved_stdout: c_int,
    _dev_null: File,
}

impl StdoutSilencer {
    fn new() -> Self {
        let dev_null = OpenOptions::new()
            .write(true)
            .open("/dev/null")
            .expect("open /dev/null");
        unsafe {
            fflush(ptr::null_mut());
        }
        let saved_stdout = unsafe { dup(1) };
        assert!(saved_stdout >= 0, "dup(stdout) failed");
        assert_eq!(
            unsafe { dup2(dev_null.as_raw_fd(), 1) },
            1,
            "redirect stdout failed"
        );
        Self {
            saved_stdout,
            _dev_null: dev_null,
        }
    }
}

impl Drop for StdoutSilencer {
    fn drop(&mut self) {
        unsafe {
            fflush(ptr::null_mut());
            dup2(self.saved_stdout, 1);
            close(self.saved_stdout);
        }
    }
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

    fn unit_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / ((1_u64 << 53) as f64)
    }
}

fn c_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("c_src/build/libtranslated_rust.so")
}

fn rust_library_path() -> PathBuf {
    std::env::current_exe()
        .expect("current test executable")
        .parent()
        .and_then(Path::parent)
        .expect("Cargo target profile directory")
        .join("libmodeselect_lib.so")
}

fn assert_i32_bytes(c: i32, rust: i32, context: impl std::fmt::Display) {
    assert_eq!(
        c.to_ne_bytes(),
        rust.to_ne_bytes(),
        "byte mismatch for {context}: C={c} Rust={rust}"
    );
}

fn assert_i64_bytes(c: i64, rust: i64, context: impl std::fmt::Display) {
    assert_eq!(
        c.to_ne_bytes(),
        rust.to_ne_bytes(),
        "byte mismatch for {context}: C={c} Rust={rust}"
    );
}

#[test]
fn valid_low_level_configuration_rows_match() {
    let _lock = STDOUT_LOCK.lock().expect("stdout lock");
    let libraries = unsafe { Libraries::load() };
    let mut rng = Rng::new(0x5EED_C0DE_D15C_A11E);

    unsafe {
        let (c_classify, rust_classify) = libraries.symbols::<ClassifyMode>(b"classify_mode\0");
        for mode in ["standard", "enhanced", "turbo", "extreme"] {
            for iteration in 0..CASES_PER_SHAPE {
                let mut bytes = vec![b'x'; iteration % 17];
                bytes.extend_from_slice(mode.as_bytes());
                bytes.push(0);
                let mode_ptr = bytes[(iteration % 17)..].as_ptr().cast();
                assert_i32_bytes(
                    c_classify(mode_ptr),
                    rust_classify(mode_ptr),
                    format_args!("classify_mode({mode:?}), iteration {iteration}"),
                );
            }
        }

        let (c_apply, rust_apply) = libraries.symbols::<ApplyMultiplier>(b"apply_multiplier\0");
        for level in 0..=4 {
            for iteration in 0..CASES_PER_SHAPE {
                let base = rng.next_i32();
                assert_i32_bytes(
                    c_apply(base, level),
                    rust_apply(base, level),
                    format_args!("apply_multiplier({base}, {level}), iteration {iteration}"),
                );
            }
        }

        let (c_convert_time, rust_convert_time) =
            libraries.symbols::<ConvertDouble>(b"convert_time_factor\0");
        compare_double_cases(&c_convert_time, &rust_convert_time, &mut rng, 1e12);

        let (c_convert_negative, rust_convert_negative) =
            libraries.symbols::<ConvertDouble>(b"convert_negative_overflow\0");
        compare_double_cases(&c_convert_negative, &rust_convert_negative, &mut rng, -1e15);

        let (c_modified_time, rust_modified_time) =
            libraries.symbols::<GetModifiedTime>(b"get_modified_time\0");
        let fixed_offsets = [
            (0, 0),
            (1, 1),
            (-1, -1),
            (1, -1),
            (-1, 1),
            (i32::MIN, i32::MIN),
            (i32::MAX, i32::MAX),
        ];
        for &(days, hours) in &fixed_offsets {
            assert_i64_bytes(
                c_modified_time(days, hours),
                rust_modified_time(days, hours),
                format_args!("get_modified_time({days}, {hours})"),
            );
        }
        for iteration in 0..(CASES_PER_SHAPE * 5) {
            let days = rng.next_i32();
            let hours = rng.next_i32();
            assert_i64_bytes(
                c_modified_time(days, hours),
                rust_modified_time(days, hours),
                format_args!("random get_modified_time iteration {iteration}"),
            );
        }

        let (c_hash, rust_hash) = libraries.symbols::<HashTimeValue>(b"hash_time_value\0");
        let fixed_times = [0, 1, 0x7f, 0x0102_0304_0506_0708, -1, i64::MIN, i64::MAX];
        for &time in &fixed_times {
            assert_i32_bytes(
                c_hash(time),
                rust_hash(time),
                format_args!("hash_time_value({time})"),
            );
        }
        for iteration in 0..(CASES_PER_SHAPE * 5) {
            let time = rng.next_u64() as i64;
            assert_i32_bytes(
                c_hash(time),
                rust_hash(time),
                format_args!("random hash_time_value iteration {iteration}"),
            );
        }
    }
}

unsafe fn compare_double_cases(
    c_function: &ConvertDouble,
    rust_function: &ConvertDouble,
    rng: &mut Rng,
    scale: f64,
) {
    let threshold = f64::from(i32::MAX) / scale.abs();
    let mut cases = vec![
        0.0,
        -0.0,
        f64::from(i32::MAX) / scale,
        f64::from(i32::MIN) / scale,
        threshold * 1.000_001,
        -threshold * 1.000_001,
        f64::MAX,
        -f64::MAX,
        f64::INFINITY,
        f64::NEG_INFINITY,
    ];
    for payload in 1..=CASES_PER_SHAPE as u64 {
        cases.push(f64::from_bits(0x7ff8_0000_0000_0000 | payload));
    }
    for _ in 0..CASES_PER_SHAPE {
        let magnitude = rng.unit_f64() * threshold * 0.99;
        cases.push(magnitude);
        cases.push(-magnitude);
        cases.push(threshold * (1.01 + rng.unit_f64() * 1e6));
        cases.push(-threshold * (1.01 + rng.unit_f64() * 1e6));
    }

    for (iteration, value) in cases.into_iter().enumerate() {
        assert_i32_bytes(
            unsafe { c_function(value) },
            unsafe { rust_function(value) },
            format_args!("double conversion scale {scale:e}, value {value:?}, case {iteration}"),
        );
    }
}

#[test]
fn composed_pipeline_configuration_cross_product_matches() {
    let _lock = STDOUT_LOCK.lock().expect("stdout lock");
    let _silencer = StdoutSilencer::new();
    let libraries = unsafe { Libraries::load() };
    let mut rng = Rng::new(0xC012_05ED_4805_EED5);

    unsafe {
        let (c_modeselect, rust_modeselect) = libraries.symbols::<Modeselect>(b"modeselect\0");
        for mode_residue in 0..4 {
            for complexity_residue in 0..5 {
                for hour_residue in 0..24 {
                    for iteration in 0..CASES_PER_MODESELECT_ROW {
                        let mode_selector =
                            mode_residue + 4 * (rng.next_u64() % 100_000_000) as i32;
                        let complexity =
                            complexity_residue + 5 * (rng.next_u64() % 100_000_000) as i32;
                        let seed = hour_residue + 24 * (rng.next_u64() % 80_000_000) as i32;
                        let time_offset = rng.next_i32();
                        assert_i32_bytes(
                            c_modeselect(mode_selector, time_offset, complexity, seed),
                            rust_modeselect(mode_selector, time_offset, complexity, seed),
                            format_args!(
                                "modeselect residues ({mode_residue}, \
                                 {complexity_residue}, {hour_residue}), iteration {iteration}"
                            ),
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn explicit_error_surface_matches() {
    let _lock = STDOUT_LOCK.lock().expect("stdout lock");
    let libraries = unsafe { Libraries::load() };
    let mut rng = Rng::new(0xE220_5EED_5E17_1E15);

    unsafe {
        let (c_classify, rust_classify) = libraries.symbols::<ClassifyMode>(b"classify_mode\0");
        for iteration in 0..CASES_PER_SHAPE {
            let unknown = CString::new(format!("unknown_{:016x}", rng.next_u64())).unwrap();
            let c_result = c_classify(unknown.as_ptr());
            let rust_result = rust_classify(unknown.as_ptr());
            assert_eq!(
                c_result, 0,
                "C unknown-mode sentinel, iteration {iteration}"
            );
            assert_i32_bytes(
                c_result,
                rust_result,
                format_args!("unknown classify_mode iteration {iteration}"),
            );
        }

        let (c_apply, rust_apply) = libraries.symbols::<ApplyMultiplier>(b"apply_multiplier\0");
        let mut invalid_levels = vec![-1, 5, i32::MIN, i32::MAX];
        while invalid_levels.len() < CASES_PER_SHAPE {
            let level = rng.next_i32();
            if !(0..=4).contains(&level) {
                invalid_levels.push(level);
            }
        }
        for (iteration, level) in invalid_levels.into_iter().enumerate() {
            let base = rng.next_i32();
            let c_result = c_apply(base, level);
            let rust_result = rust_apply(base, level);
            assert_eq!(
                c_result, 0xDEAD,
                "C invalid-level sentinel, iteration {iteration}"
            );
            assert_i32_bytes(
                c_result,
                rust_result,
                format_args!("invalid apply_multiplier level {level}, iteration {iteration}"),
            );
        }
    }
}

#[test]
fn null_pointer_boundary_matches() {
    if std::env::var_os("NULL_CLASSIFY_LIBRARY").is_some() {
        return;
    }

    let c_status = run_null_probe(&c_library_path());
    let rust_status = run_null_probe(&rust_library_path());
    assert_eq!(c_status.signal(), Some(11), "C null-pointer signal");
    assert_eq!(
        rust_status.signal(),
        c_status.signal(),
        "Rust null-pointer behavior differs from C"
    );
}

fn run_null_probe(library_path: &Path) -> std::process::ExitStatus {
    Command::new(std::env::current_exe().expect("current test executable"))
        .arg("--exact")
        .arg("null_classify_probe")
        .arg("--nocapture")
        .env("NULL_CLASSIFY_LIBRARY", library_path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("run null-pointer probe")
}

#[test]
fn null_classify_probe() {
    let Some(library_path) = std::env::var_os("NULL_CLASSIFY_LIBRARY") else {
        return;
    };
    unsafe {
        let library = Library::new(library_path).expect("load null-probe library");
        let classify: Symbol<'_, ClassifyMode> =
            library.get(b"classify_mode\0").expect("load classify_mode");
        classify(ptr::null());
    }
}
