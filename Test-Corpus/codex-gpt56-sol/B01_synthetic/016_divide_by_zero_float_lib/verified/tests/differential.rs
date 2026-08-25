use libloading::Library;
use std::ffi::{CString, c_char, c_int, c_void};
use std::fs::File;
use std::io::Read;
use std::os::fd::FromRawFd;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, OnceLock};

type PrintLine = unsafe extern "C" fn(*const c_char);
type PrintIntLine = unsafe extern "C" fn(c_int);
type FloatFn = unsafe extern "C" fn(f32);
type DriverFn = unsafe extern "C" fn(f32, f32);

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(fds: *mut c_int) -> c_int;
}

struct Api {
    _library: Library,
    print_line: PrintLine,
    print_int_line: PrintIntLine,
    bad: FloatFn,
    good: FloatFn,
    driver: DriverFn,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let print_line = unsafe { *library.get::<PrintLine>(b"printLine\0").unwrap() };
        let print_int_line = unsafe { *library.get::<PrintIntLine>(b"printIntLine\0").unwrap() };
        let bad = unsafe { *library.get::<FloatFn>(b"bad\0").unwrap() };
        let good = unsafe { *library.get::<FloatFn>(b"good\0").unwrap() };
        let driver = unsafe { *library.get::<DriverFn>(b"driver\0").unwrap() };

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

struct XorShift64(u64);

impl XorShift64 {
    fn next_u32(&mut self) -> u32 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value as u32
    }

    fn regular_float(&mut self) -> f32 {
        let magnitude = 0.001 + (self.next_u32() % 10_000_000) as f32 / 1_000.0;
        if self.next_u32() & 1 == 0 {
            magnitude
        } else {
            -magnitude
        }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn rust_library_path() -> PathBuf {
    static RUST_LIBRARY: OnceLock<PathBuf> = OnceLock::new();

    RUST_LIBRARY
        .get_or_init(|| {
            let target = std::env::var_os("CARGO_TARGET_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|| manifest_dir().join("target"));
            let target = if target.is_absolute() {
                target
            } else {
                manifest_dir().join(target)
            };
            let output = Command::new(env!("CARGO"))
                .args(["build", "--lib", "--no-default-features", "--target-dir"])
                .arg(&target)
                .current_dir(manifest_dir())
                .output()
                .expect("run cargo build for Rust cdylib");
            assert!(
                output.status.success(),
                "cargo build failed:\nstdout:\n{}\nstderr:\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            target.join("debug").join("libdriver.so")
        })
        .clone()
}

fn capture_stdout(call: impl FnOnce()) -> Vec<u8> {
    const STDOUT_FILENO: c_int = 1;
    let mut fds = [-1; 2];

    let saved_stdout = unsafe {
        assert_eq!(fflush(std::ptr::null_mut()), 0);
        assert_eq!(pipe(fds.as_mut_ptr()), 0);
        let saved_stdout = dup(STDOUT_FILENO);
        assert!(saved_stdout >= 0);
        assert_eq!(dup2(fds[1], STDOUT_FILENO), STDOUT_FILENO);
        assert_eq!(close(fds[1]), 0);
        saved_stdout
    };

    let read_fd = fds[0];
    let reader = std::thread::spawn(move || {
        let mut output = Vec::new();
        unsafe {
            File::from_raw_fd(read_fd)
                .read_to_end(&mut output)
                .expect("read captured stdout");
        }
        output
    });

    call();

    unsafe {
        assert_eq!(fflush(std::ptr::null_mut()), 0);
        assert_eq!(dup2(saved_stdout, STDOUT_FILENO), STDOUT_FILENO);
        assert_eq!(close(saved_stdout), 0);
    }

    reader.join().expect("stdout reader thread")
}

fn compare(c_api: &Api, rust_api: &Api, label: &str, call: impl Fn(&Api)) {
    static STDOUT_LOCK: Mutex<()> = Mutex::new(());
    let _guard = STDOUT_LOCK.lock().expect("stdout capture lock");
    let c_output = capture_stdout(|| call(c_api));
    let rust_output = capture_stdout(|| call(rust_api));
    assert_eq!(rust_output, c_output, "{label}");
}

fn load_apis() -> (Api, Api) {
    let c_path = manifest_dir().join("c_src/build/libdriver.so");
    let rust_path = rust_library_path();
    assert!(c_path.is_file(), "missing C library: {}", c_path.display());
    assert!(
        rust_path.is_file(),
        "missing Rust library: {}",
        rust_path.display()
    );
    unsafe { (Api::load(&c_path), Api::load(&rust_path)) }
}

#[test]
fn differential_surface_matches() {
    let (c_api, rust_api) = load_apis();

    // CONFIGS.md row 1: all strings are passed through the dynamic symbol.
    let mut strings = vec![
        CString::new("").unwrap(),
        CString::new("ordinary text").unwrap(),
        CString::new("percent signs %s %d are data").unwrap(),
        CString::new(vec![b'x'; 4096]).unwrap(),
    ];
    let mut rng = XorShift64(0x4d59_5df4_d0f3_3173);
    for _ in 0..128 {
        let len = (rng.next_u32() % 96) as usize;
        let bytes: Vec<u8> = (0..len)
            .map(|_| b' ' + (rng.next_u32() % 95) as u8)
            .collect();
        strings.push(CString::new(bytes).unwrap());
    }
    let embedded_nul = b"visible\0ignored\0";
    compare(&c_api, &rust_api, "printLine valid strings", |api| unsafe {
        for string in &strings {
            (api.print_line)(string.as_ptr());
        }
        (api.print_line)(embedded_nul.as_ptr().cast());
    });

    // CONFIGS.md row 2.
    let mut integers = vec![0, 1, -1, c_int::MIN, c_int::MAX];
    for _ in 0..512 {
        integers.push(rng.next_u32() as c_int);
    }
    compare(
        &c_api,
        &rust_api,
        "printIntLine integer surface",
        |api| unsafe {
            for &value in &integers {
                (api.print_int_line)(value);
            }
        },
    );

    // CONFIGS.md row 3.
    let mut regular_floats = vec![0.001, -0.001, 1.0, -1.0, 100.0, -100.0];
    for _ in 0..512 {
        regular_floats.push(rng.regular_float());
    }
    compare(&c_api, &rust_api, "bad regular floats", |api| unsafe {
        for &value in &regular_floats {
            (api.bad)(value);
        }
    });

    // CONFIGS.md row 4.
    let conversion_boundary = (100.0_f64 / 2_147_483_648.0) as f32;
    let exceptional_bad = [
        0.0,
        -0.0,
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        f32::from_bits(1),
        f32::from_bits(0x8000_0001),
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        f32::from_bits(0xffc0_1234),
        conversion_boundary,
        f32::from_bits(conversion_boundary.to_bits() - 1),
        f32::from_bits(conversion_boundary.to_bits() + 1),
        -conversion_boundary,
        f32::from_bits((-conversion_boundary).to_bits() - 1),
        f32::from_bits((-conversion_boundary).to_bits() + 1),
    ];
    compare(&c_api, &rust_api, "bad exceptional floats", |api| unsafe {
        for &value in &exceptional_bad {
            (api.bad)(value);
        }
    });

    // CONFIGS.md row 5.
    let threshold = 0.000001_f32;
    let above_threshold = f32::from_bits(threshold.to_bits() + 1);
    let mut valid_good = vec![
        above_threshold,
        -above_threshold,
        0.001,
        -0.001,
        1.0,
        -1.0,
        f32::MAX,
        f32::MIN,
    ];
    for _ in 0..512 {
        valid_good.push(rng.regular_float());
    }
    compare(
        &c_api,
        &rust_api,
        "good finite division branch",
        |api| unsafe {
            for &value in &valid_good {
                (api.good)(value);
            }
        },
    );

    // CONFIGS.md row 6.
    compare(&c_api, &rust_api, "good infinities", |api| unsafe {
        (api.good)(f32::INFINITY);
        (api.good)(f32::NEG_INFINITY);
    });

    // CONFIGS.md row 7.
    let regular_pairs: Vec<(f32, f32)> = (0..256)
        .map(|_| (rng.regular_float(), rng.regular_float()))
        .collect();
    compare(
        &c_api,
        &rust_api,
        "driver regular cross-product",
        |api| unsafe {
            for &(good_data, bad_data) in &regular_pairs {
                (api.driver)(good_data, bad_data);
            }
        },
    );

    // CONFIGS.md row 8.
    compare(
        &c_api,
        &rust_api,
        "driver exceptional badData cross-product",
        |api| unsafe {
            for (index, &bad_data) in exceptional_bad.iter().enumerate() {
                let good_data = if index & 1 == 0 { 2.0 } else { -2.0 };
                (api.driver)(good_data, bad_data);
            }
        },
    );

    // CONFIGS.md row 9.
    compare(
        &c_api,
        &rust_api,
        "driver infinite goodData cross-product",
        |api| unsafe {
            for (index, &bad_data) in exceptional_bad.iter().enumerate() {
                let good_data = if index & 1 == 0 {
                    f32::INFINITY
                } else {
                    f32::NEG_INFINITY
                };
                (api.driver)(good_data, bad_data);
            }
            (api.driver)(f32::INFINITY, 2.0);
            (api.driver)(f32::NEG_INFINITY, -2.0);
        },
    );

    rejection_surface_matches(&c_api, &rust_api);
}

fn rejection_surface_matches(c_api: &Api, rust_api: &Api) {
    // ERRORS.md row 1 and the generic pointer boundary.
    compare(c_api, rust_api, "printLine null pointer", |api| unsafe {
        (api.print_line)(std::ptr::null());
    });

    // ERRORS.md row 2 through the low-level entry point.
    let threshold = 0.000001_f32;
    let mut rejected = vec![
        0.0,
        -0.0,
        threshold,
        -threshold,
        f32::from_bits(threshold.to_bits() - 1),
        -f32::from_bits(threshold.to_bits() - 1),
        f32::from_bits(1),
        f32::from_bits(0x8000_0001),
        f32::NAN,
        f32::from_bits(0xffc0_1234),
    ];
    let mut rng = XorShift64(0xa09e_667f_3bcc_908b);
    for _ in 0..512 {
        let magnitude = (rng.next_u32() as f64 / u32::MAX as f64 * 0.000001) as f32;
        rejected.push(if rng.next_u32() & 1 == 0 {
            magnitude
        } else {
            -magnitude
        });
    }
    compare(c_api, rust_api, "good guarded inputs", |api| unsafe {
        for &value in &rejected {
            (api.good)(value);
        }
    });

    // ERRORS.md row 2 through the composed public entry point.
    compare(c_api, rust_api, "driver guarded goodData", |api| unsafe {
        for (index, &good_data) in rejected.iter().enumerate() {
            let bad_data = if index & 1 == 0 { 2.0 } else { -2.0 };
            (api.driver)(good_data, bad_data);
        }
    });
}
