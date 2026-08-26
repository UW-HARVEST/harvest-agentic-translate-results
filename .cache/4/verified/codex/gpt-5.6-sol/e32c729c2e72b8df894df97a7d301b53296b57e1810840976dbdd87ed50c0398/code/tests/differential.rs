use libloading::Library;
use std::ffi::{CString, c_char, c_int, c_void};
use std::fs;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct House {
    floors: c_int,
    bedrooms: c_int,
    bathrooms: f64,
}

type DriverFn = unsafe extern "C" fn(*const c_char);
type RunFn = unsafe extern "C" fn(*mut House, c_int);

struct Api {
    _library: Library,
    driver: DriverFn,
    run: RunFn,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let driver = unsafe { *library.get::<DriverFn>(b"driver\0").unwrap() };
        let run = unsafe { *library.get::<RunFn>(b"run\0").unwrap() };
        Self {
            _library: library,
            driver,
            run,
        }
    }
}

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(pipe_fds: *mut c_int) -> c_int;
    fn read(fd: c_int, buffer: *mut c_void, count: usize) -> isize;
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    manifest_dir().join("c_src/build/libdriver.so")
}

fn rust_library_path() -> PathBuf {
    if let Some(path) = std::env::var_os("DRIVER_RUST_SO") {
        return PathBuf::from(path);
    }

    let executable = std::env::current_exe().expect("current test executable");
    let profile_dir = executable
        .parent()
        .and_then(Path::parent)
        .expect("target profile directory");
    let profile_library = profile_dir.join("libdriver.so");
    if profile_library.is_file() {
        profile_library
    } else {
        profile_dir
            .parent()
            .expect("Cargo target directory")
            .join("release/libdriver.so")
    }
}

fn assert_libraries_exist() {
    for path in [c_library_path(), rust_library_path()] {
        assert!(
            path.is_file(),
            "shared library is missing: {}",
            path.display()
        );
    }
}

fn capture_stdout(call: impl FnOnce()) -> Vec<u8> {
    unsafe {
        assert_eq!(fflush(ptr::null_mut()), 0);

        let saved_stdout = dup(1);
        assert!(saved_stdout >= 0);

        let mut pipe_fds = [-1; 2];
        assert_eq!(pipe(pipe_fds.as_mut_ptr()), 0);
        assert_eq!(dup2(pipe_fds[1], 1), 1);
        assert_eq!(close(pipe_fds[1]), 0);

        call();

        assert_eq!(fflush(ptr::null_mut()), 0);
        assert_eq!(dup2(saved_stdout, 1), 1);
        assert_eq!(close(saved_stdout), 0);

        let mut output = Vec::new();
        let mut chunk = [0_u8; 4096];
        loop {
            let count = read(pipe_fds[0], chunk.as_mut_ptr().cast(), chunk.len());
            assert!(count >= 0);
            if count == 0 {
                break;
            }
            output.extend_from_slice(&chunk[..count as usize]);
        }
        assert_eq!(close(pipe_fds[0]), 0);
        output
    }
}

fn call_driver(api: &Api, input: &str) -> Vec<u8> {
    let input = CString::new(input).expect("test input has no interior NUL");
    capture_stdout(|| unsafe { (api.driver)(input.as_ptr()) })
}

fn call_run(api: &Api, initial: House, extra_bedrooms: c_int) -> (Vec<u8>, House) {
    let mut house = initial;
    let output = capture_stdout(|| unsafe { (api.run)(&mut house, extra_bedrooms) });
    (output, house)
}

fn house_bytes(house: &House) -> &[u8] {
    unsafe { std::slice::from_raw_parts(ptr::from_ref(house).cast(), std::mem::size_of::<House>()) }
}

fn compare_driver(c: &Api, rust: &Api, input: &str, row: &str) -> Vec<u8> {
    let c_output = call_driver(c, input);
    let rust_output = call_driver(rust, input);
    assert_eq!(
        rust_output, c_output,
        "{row}: output differs for input {input:?}"
    );
    c_output
}

fn compare_run(c: &Api, rust: &Api, house: House, extra: c_int, row: &str) {
    let (c_output, c_house) = call_run(c, house, extra);
    let (rust_output, rust_house) = call_run(rust, house, extra);
    assert_eq!(
        rust_output, c_output,
        "{row}: output differs for house {house:?}, extra {extra}"
    );
    assert_eq!(
        house_bytes(&rust_house),
        house_bytes(&c_house),
        "{row}: final struct bytes differ for house {house:?}, extra {extra}"
    );
}

#[derive(Clone, Copy)]
struct Rng(u64);

impl Rng {
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
}

#[test]
fn differential_valid_and_error_surfaces() {
    assert_libraries_exist();
    let c = unsafe { Api::load(&c_library_path()) };
    let rust = unsafe { Api::load(&rust_library_path()) };
    let mut rng = Rng(0x7c4a_91e2_d35b_680f);

    // C1: direct run calls with ordinary integers and finite doubles.
    for _ in 0..256 {
        let house = House {
            floors: (rng.next_i32() % 1_000_000).clamp(-1_000_000, 1_000_000),
            bedrooms: (rng.next_i32() % 1_000_000).clamp(-1_000_000, 1_000_000),
            bathrooms: (rng.next_i32() % 1_000_000) as f64 / 10.0,
        };
        let extra = (rng.next_i32() % 1_000_000).clamp(-1_000_000, 1_000_000);
        compare_run(&c, &rust, house, extra, "C1");
    }

    // C2: additions that cross signed int representation boundaries.
    for _ in 0..128 {
        let distance = (rng.next_u64() % 32) as i32;
        let high = rng.next_u64() & 1 == 0;
        let (floors, bedrooms, extra) = if high {
            (
                i32::MAX - distance,
                i32::MAX - distance,
                33 + (rng.next_u64() % 1024) as i32,
            )
        } else {
            (
                i32::MIN + distance,
                i32::MIN + distance,
                -33 - (rng.next_u64() % 1024) as i32,
            )
        };
        compare_run(
            &c,
            &rust,
            House {
                floors,
                bedrooms,
                bathrooms: (rng.next_i32() % 1000) as f64 / 10.0,
            },
            extra,
            "C2",
        );
    }

    // C3: libc floating formatting and addition for non-finite/signed-zero values.
    let special_values = [0.0, -0.0, f64::INFINITY, f64::NEG_INFINITY];
    for bathrooms in special_values {
        compare_run(
            &c,
            &rust,
            House {
                floors: rng.next_i32(),
                bedrooms: rng.next_i32(),
                bathrooms,
            },
            rng.next_i32(),
            "C3",
        );
    }
    for _ in 0..128 {
        let payload = rng.next_u64() & 0x0007_ffff_ffff_ffff;
        let sign = (rng.next_u64() & 1) << 63;
        let quiet_nan = f64::from_bits(sign | 0x7ff8_0000_0000_0000 | payload);
        compare_run(
            &c,
            &rust,
            House {
                floors: rng.next_i32(),
                bedrooms: rng.next_i32(),
                bathrooms: quiet_nan,
            },
            rng.next_i32(),
            "C3",
        );
    }

    // C4: canonical decimal strings spanning the complete i32 value space.
    for value in [i32::MIN, -1, 0, 1, i32::MAX] {
        compare_driver(&c, &rust, &value.to_string(), "C4");
    }
    for _ in 0..256 {
        let input = rng.next_i32().to_string();
        compare_driver(&c, &rust, &input, "C4");
    }

    // C5: leading whitespace/sign and ignored trailing nondigit bytes.
    let whitespace = ["", " ", "\t", "\n", "\r\n", " \x0b\x0c"];
    let suffixes = ["", "x", " trailing", ".75", "_suffix", "\tignored"];
    for index in 0..256 {
        let value = rng.next_i32();
        let digits = if value >= 0 && index % 2 == 0 {
            format!("+{value}")
        } else {
            value.to_string()
        };
        let input = format!(
            "{}{}{}",
            whitespace[index % whitespace.len()],
            digits,
            suffixes[(index / whitespace.len()) % suffixes.len()]
        );
        compare_driver(&c, &rust, &input, "C5");
    }

    // C6: accepted values at and near both i32 boundaries.
    for value in [i32::MIN, i32::MIN + 1, i32::MAX - 1, i32::MAX] {
        compare_driver(&c, &rust, &value.to_string(), "C6");
    }
    for _ in 0..256 {
        let distance = (rng.next_u64() % 4096) as i32;
        for value in [i32::MIN + distance, i32::MAX - distance] {
            compare_driver(&c, &rust, &value.to_string(), "C6");
        }
    }

    // E1: no decimal conversion.
    let fixed_no_conversion = ["", " ", "\t\n", "abc", "+", "-", ".12", "x123"];
    for input in fixed_no_conversion {
        let output = compare_driver(&c, &rust, input, "E1");
        assert_eq!(output, b"An error occurred\n");
    }
    for _ in 0..128 {
        let length = 1 + (rng.next_u64() % 64) as usize;
        let input: String = (0..length)
            .map(|_| (b'a' + (rng.next_u64() % 26) as u8) as char)
            .collect();
        let output = compare_driver(&c, &rust, &input, "E1");
        assert_eq!(output, b"An error occurred\n");
    }

    // E2: strtol range error, including oversized strings.
    for length in [20, 32, 64, 255, 1024, 4096] {
        for sign in ["", "-"] {
            let input = format!("{sign}{}", "9".repeat(length));
            let output = compare_driver(&c, &rust, &input, "E2");
            assert_eq!(output, b"An error occurred\n");
        }
    }

    // E3/E4: long can represent the value, but int cannot.
    for _ in 0..128 {
        let distance = 1 + (rng.next_u64() % 1_000_000) as i64;
        let below = i32::MIN as i64 - distance;
        let above = i32::MAX as i64 + distance;
        assert_eq!(
            compare_driver(&c, &rust, &below.to_string(), "E3"),
            b"An error occurred\n"
        );
        assert_eq!(
            compare_driver(&c, &rust, &above.to_string(), "E4"),
            b"An error occurred\n"
        );
    }

    compare_null_boundary("driver", "E5");
    compare_null_boundary("run", "E6");
}

fn compare_null_boundary(function: &str, row: &str) {
    let current_exe = std::env::current_exe().expect("current test executable");
    let run_probe = |library: &Path| {
        Command::new(&current_exe)
            .args(["--ignored", "--exact", "ffi_null_pointer_probe"])
            .env("DIFFERENTIAL_NULL_LIBRARY", library)
            .env("DIFFERENTIAL_NULL_FUNCTION", function)
            .status()
            .expect("run null-pointer probe")
    };

    let c_status = run_probe(&c_library_path());
    let rust_status = run_probe(&rust_library_path());
    assert_eq!(
        rust_status.signal(),
        c_status.signal(),
        "{row}: null-pointer termination signal differs"
    );
    assert_eq!(
        c_status.signal(),
        Some(11),
        "{row}: C did not terminate with SIGSEGV"
    );
}

#[test]
#[ignore = "run only in a subprocess by differential_valid_and_error_surfaces"]
fn ffi_null_pointer_probe() {
    let library_path = PathBuf::from(std::env::var_os("DIFFERENTIAL_NULL_LIBRARY").unwrap());
    let function = std::env::var("DIFFERENTIAL_NULL_FUNCTION").unwrap();
    let api = unsafe { Api::load(&library_path) };
    match function.as_str() {
        "driver" => unsafe { (api.driver)(ptr::null()) },
        "run" => unsafe { (api.run)(ptr::null_mut(), 0) },
        _ => panic!("unknown null probe function: {function}"),
    }
}

#[test]
fn phase_a_artifacts_exist_and_cover_rows() {
    for (name, expected_rows) in [
        ("SYMBOLS.md", ["driver", "run"].as_slice()),
        ("ERRORS.md", ["E1", "E2", "E3", "E4", "E5", "E6"].as_slice()),
        (
            "CONFIGS.md",
            ["F1", "C1", "C2", "C3", "C4", "C5", "C6"].as_slice(),
        ),
    ] {
        let contents = fs::read_to_string(manifest_dir().join(name)).unwrap();
        for row in expected_rows {
            assert!(contents.contains(row), "{name} is missing {row}");
        }
    }
}
