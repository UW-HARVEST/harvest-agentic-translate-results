use libloading::Library;
use std::collections::BTreeSet;
use std::ffi::{c_char, c_int, c_void, CString};
use std::fs;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

type FmaArray = unsafe extern "C" fn(*mut c_int, *const c_int, *const c_int, *const c_int, c_int);
type Driver = unsafe extern "C" fn(*mut c_int, c_int);
type Main = unsafe extern "C" fn() -> c_int;

unsafe extern "C" {
    static mut stdin: *mut c_void;
    static mut stdout: *mut c_void;

    fn clearerr(stream: *mut c_void);
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn freopen(pathname: *const c_char, mode: *const c_char, stream: *mut c_void) -> *mut c_void;
}

static STDIO_LOCK: Mutex<()> = Mutex::new(());
static TEMP_ID: AtomicU64 = AtomicU64::new(0);

struct Api {
    _library: Library,
    fma_array: FmaArray,
    driver: Driver,
    main: Main,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }.unwrap();
        let fma_array = unsafe { *library.get::<FmaArray>(b"fma_array\0").unwrap() };
        let driver = unsafe { *library.get::<Driver>(b"driver\0").unwrap() };
        let main = unsafe { *library.get::<Main>(b"main\0").unwrap() };
        Self {
            _library: library,
            fma_array,
            driver,
            main,
        }
    }
}

#[derive(Clone)]
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

    fn i32(&mut self) -> i32 {
        self.next_u64() as i32
    }

    fn usize(&mut self, start: usize, end: usize) -> usize {
        start + (self.next_u64() as usize % (end - start))
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    manifest_dir().join("c_src/build/libdriver_c.so")
}

fn rust_library_path() -> PathBuf {
    let test_exe = std::env::current_exe().unwrap();
    test_exe.parent().unwrap().join("libdriver.so")
}

fn load_apis() -> (Api, Api) {
    let c_path = c_library_path();
    let rust_path = rust_library_path();
    assert!(
        c_path.is_file(),
        "missing C shared object: {}",
        c_path.display()
    );
    assert!(
        rust_path.is_file(),
        "missing Rust shared object: {}",
        rust_path.display()
    );
    unsafe { (Api::load(&c_path), Api::load(&rust_path)) }
}

fn temp_path(kind: &str) -> PathBuf {
    let id = TEMP_ID.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("driver-diff-{}-{id}-{kind}", std::process::id()))
}

fn with_stdio<T>(input: &[u8], call: impl FnOnce() -> T) -> (T, Vec<u8>) {
    let input_path = temp_path("in");
    let output_path = temp_path("out");
    fs::write(&input_path, input).unwrap();

    let input_c = CString::new(input_path.as_os_str().as_encoded_bytes()).unwrap();
    let output_c = CString::new(output_path.as_os_str().as_encoded_bytes()).unwrap();
    let read_mode = c"r";
    let write_mode = c"w";

    let (saved_stdin, saved_stdout);
    unsafe {
        fflush(std::ptr::null_mut());
        saved_stdin = dup(0);
        saved_stdout = dup(1);
        assert!(saved_stdin >= 0 && saved_stdout >= 0);
        assert!(!freopen(input_c.as_ptr(), read_mode.as_ptr(), stdin).is_null());
        assert!(!freopen(output_c.as_ptr(), write_mode.as_ptr(), stdout).is_null());
    }

    let result = call();

    unsafe {
        assert_eq!(fflush(std::ptr::null_mut()), 0);
        assert_eq!(dup2(saved_stdin, 0), 0);
        assert_eq!(dup2(saved_stdout, 1), 1);
        close(saved_stdin);
        close(saved_stdout);
        clearerr(stdin);
        clearerr(stdout);
    }

    let output = fs::read(&output_path).unwrap();
    fs::remove_file(input_path).unwrap();
    fs::remove_file(output_path).unwrap();
    (result, output)
}

fn call_main(api: &Api, input: &[u8]) -> (c_int, Vec<u8>) {
    with_stdio(input, || unsafe { (api.main)() })
}

fn call_driver(api: &Api, values: &mut [i32]) -> Vec<u8> {
    let (_, output) = with_stdio(&[], || unsafe {
        (api.driver)(values.as_mut_ptr(), values.len() as c_int);
    });
    output
}

fn call_driver_null(api: &Api, len: c_int) -> Vec<u8> {
    let (_, output) = with_stdio(&[], || unsafe {
        (api.driver)(std::ptr::null_mut(), len);
    });
    output
}

fn compare_fma(c: &Api, rust: &Api, len: usize, cases: usize, seed: u64, aliased: bool) {
    let mut rng = Rng::new(seed);
    for case in 0..cases {
        if aliased {
            let initial: Vec<i32> = (0..len).map(|_| rng.i32()).collect();
            let mut c_values = initial.clone();
            let mut rust_values = initial;
            unsafe {
                (c.fma_array)(
                    c_values.as_mut_ptr(),
                    c_values.as_ptr(),
                    c_values.as_ptr(),
                    c_values.as_ptr(),
                    len as c_int,
                );
                (rust.fma_array)(
                    rust_values.as_mut_ptr(),
                    rust_values.as_ptr(),
                    rust_values.as_ptr(),
                    rust_values.as_ptr(),
                    len as c_int,
                );
            }
            assert_eq!(c_values, rust_values, "aliased fma case {case}, len {len}");
        } else {
            let mul1: Vec<i32> = (0..len).map(|_| rng.i32()).collect();
            let mul2: Vec<i32> = (0..len).map(|_| rng.i32()).collect();
            let add: Vec<i32> = (0..len).map(|_| rng.i32()).collect();
            let mut c_out = vec![0x1357_2468; len];
            let mut rust_out = c_out.clone();
            unsafe {
                (c.fma_array)(
                    c_out.as_mut_ptr(),
                    mul1.as_ptr(),
                    mul2.as_ptr(),
                    add.as_ptr(),
                    len as c_int,
                );
                (rust.fma_array)(
                    rust_out.as_mut_ptr(),
                    mul1.as_ptr(),
                    mul2.as_ptr(),
                    add.as_ptr(),
                    len as c_int,
                );
            }
            assert_eq!(c_out, rust_out, "separate fma case {case}, len {len}");
        }
    }
}

fn random_values(rng: &mut Rng, count: usize) -> Vec<i32> {
    (0..count).map(|_| rng.i32()).collect()
}

fn encode_input(values: &[i32], rng: &mut Rng) -> Vec<u8> {
    const SEPARATORS: [&str; 6] = [" ", "\t", "\n", "\r", "\x0b", "\x0c"];
    let mut text = String::new();
    for (index, value) in values.iter().enumerate() {
        text.push_str(SEPARATORS[rng.usize(0, SEPARATORS.len())]);
        if *value >= 0 && (rng.next_u64() & 1) == 0 {
            text.push('+');
        }
        text.push_str(&value.to_string());
        if index + 1 == values.len() && (rng.next_u64() & 1) == 0 {
            text.push_str(SEPARATORS[rng.usize(0, SEPARATORS.len())]);
        }
    }
    text.into_bytes()
}

fn compare_main(c: &Api, rust: &Api, input: &[u8]) -> Vec<u8> {
    let c_result = call_main(c, input);
    let rust_result = call_main(rust, input);
    assert_eq!(c_result.0, rust_result.0, "main return for {input:?}");
    assert_eq!(c_result.1, rust_result.1, "main output for {input:?}");
    c_result.1
}

fn line_count(output: &[u8]) -> usize {
    output.iter().filter(|byte| **byte == b'\n').count()
}

fn dynamic_symbols(path: &Path) -> BTreeSet<String> {
    let output = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "nm failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .filter_map(|line| line.split_whitespace().last())
        .map(str::to_owned)
        .collect()
}

fn null_probe_signal(library: &Path, mode: &str) -> Option<i32> {
    Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "null_pointer_probe_child",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("DRIVER_NULL_PROBE_LIBRARY", library)
        .env("DRIVER_NULL_PROBE_MODE", mode)
        .status()
        .unwrap()
        .signal()
}

#[test]
fn null_pointer_probe_child() {
    let Ok(library) = std::env::var("DRIVER_NULL_PROBE_LIBRARY") else {
        return;
    };
    let mode = std::env::var("DRIVER_NULL_PROBE_MODE").unwrap();
    let api = unsafe { Api::load(Path::new(&library)) };
    let mut out = [11_i32];
    let mul1 = [13_i32];
    let mul2 = [17_i32];
    let add = [19_i32];

    unsafe {
        match mode.as_str() {
            "fma-out" => (api.fma_array)(
                std::ptr::null_mut(),
                mul1.as_ptr(),
                mul2.as_ptr(),
                add.as_ptr(),
                1,
            ),
            "fma-mul1" => (api.fma_array)(
                out.as_mut_ptr(),
                std::ptr::null(),
                mul2.as_ptr(),
                add.as_ptr(),
                1,
            ),
            "fma-mul2" => (api.fma_array)(
                out.as_mut_ptr(),
                mul1.as_ptr(),
                std::ptr::null(),
                add.as_ptr(),
                1,
            ),
            "fma-add" => (api.fma_array)(
                out.as_mut_ptr(),
                mul1.as_ptr(),
                mul2.as_ptr(),
                std::ptr::null(),
                1,
            ),
            "driver" => (api.driver)(std::ptr::null_mut(), 1),
            _ => panic!("unknown null probe mode: {mode}"),
        }
    }
}

#[test]
fn symbol_surface_matches() {
    let c_symbols = dynamic_symbols(&c_library_path());
    let rust_symbols = dynamic_symbols(&rust_library_path());
    assert_eq!(
        c_symbols,
        BTreeSet::from(["driver".into(), "fma_array".into(), "main".into()])
    );
    assert_eq!(c_symbols.difference(&rust_symbols).count(), 0);
}

#[test]
fn valid_configuration_surface_matches() {
    let _guard = STDIO_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let (c, rust) = load_apis();

    // CONFIGS rows 1-2: non-positive lengths do not access any pointer.
    for len in [-1, 0] {
        unsafe {
            (c.fma_array)(
                std::ptr::null_mut(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                len,
            );
            (rust.fma_array)(
                std::ptr::null_mut(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                len,
            );
        }
    }

    // CONFIGS rows 3-6: one/many elements with separate and composed aliasing.
    compare_fma(&c, &rust, 1, 256, 0x6a09_e667_f3bc_c909, false);
    for len in [2, 3, 7, 31, 257] {
        compare_fma(
            &c,
            &rust,
            len,
            64,
            0xbb67_ae85_84ca_a73b ^ len as u64,
            false,
        );
    }
    compare_fma(&c, &rust, 1, 256, 0x3c6e_f372_fe94_f82b, true);
    for len in [2, 3, 7, 31, 257] {
        compare_fma(&c, &rust, len, 64, 0xa54f_f53a_5f1d_36f1 ^ len as u64, true);
    }

    // CONFIGS rows 7-8: driver emits nothing for non-positive lengths.
    for len in [-1, 0] {
        assert_eq!(call_driver_null(&c, len), call_driver_null(&rust, len));
    }

    // CONFIGS rows 9-10: one, many, and a length beyond main's fixed buffer.
    let mut rng = Rng::new(0x510e_527f_ade6_82d1);
    for _ in 0..128 {
        let initial = random_values(&mut rng, 1);
        let mut c_values = initial.clone();
        let mut rust_values = initial;
        assert_eq!(
            call_driver(&c, &mut c_values),
            call_driver(&rust, &mut rust_values)
        );
        assert_eq!(c_values, rust_values);
    }
    for len in [2, 3, 17, 64, 257] {
        for _ in 0..32 {
            let initial = random_values(&mut rng, len);
            let mut c_values = initial.clone();
            let mut rust_values = initial;
            assert_eq!(
                call_driver(&c, &mut c_values),
                call_driver(&rust, &mut rust_values)
            );
            assert_eq!(c_values, rust_values);
        }
    }

    // CONFIGS rows 11-15: 0, 1, 2..99, 100, and more than 100 integers.
    compare_main(&c, &rust, b"");
    for _ in 0..64 {
        let values = random_values(&mut rng, 1);
        let input = encode_input(&values, &mut rng);
        assert_eq!(line_count(&compare_main(&c, &rust, &input)), 1);
    }
    for _ in 0..48 {
        let count = rng.usize(2, 100);
        let values = random_values(&mut rng, count);
        let input = encode_input(&values, &mut rng);
        assert_eq!(line_count(&compare_main(&c, &rust, &input)), count);
    }
    for _ in 0..16 {
        let values = random_values(&mut rng, 100);
        let input = encode_input(&values, &mut rng);
        assert_eq!(line_count(&compare_main(&c, &rust, &input)), 100);
    }
    for _ in 0..16 {
        let count = rng.usize(101, 140);
        let values = random_values(&mut rng, count);
        let input = encode_input(&values, &mut rng);
        assert_eq!(line_count(&compare_main(&c, &rust, &input)), 100);
    }
}

#[test]
fn error_surface_and_generic_boundaries_match() {
    let _guard = STDIO_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let (c, rust) = load_apis();

    // ERRORS row 1: EOF before the first conversion.
    for input in [b"".as_slice(), b" \t\n\r\x0b\x0c".as_slice()] {
        assert!(compare_main(&c, &rust, input).is_empty());
    }

    // ERRORS row 2: matching failure before the first conversion.
    for input in [b"x".as_slice(), b" +x".as_slice(), b"--1".as_slice()] {
        assert!(compare_main(&c, &rust, input).is_empty());
    }

    // ERRORS row 3: EOF after a successfully converted prefix.
    for input in [
        b"1".as_slice(),
        b"1 -2 3".as_slice(),
        b"\t+4\n5 ".as_slice(),
    ] {
        assert!(!compare_main(&c, &rust, input).is_empty());
    }

    // ERRORS row 4: matching failure after a successfully converted prefix.
    for input in [
        b"1x".as_slice(),
        b"1 -2 3 nope".as_slice(),
        b"\t+4\n5 --6".as_slice(),
    ] {
        assert!(!compare_main(&c, &rust, input).is_empty());
    }

    // Generic FFI boundaries defined by C: null with non-positive lengths.
    for len in [c_int::MIN, -1, 0] {
        unsafe {
            (c.fma_array)(
                std::ptr::null_mut(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                len,
            );
            (rust.fma_array)(
                std::ptr::null_mut(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                len,
            );
        }
        assert_eq!(call_driver_null(&c, len), call_driver_null(&rust, len));
    }

    // Positive-length null pointers are C undefined behavior, but both shared
    // objects must still have the same externally observed process outcome.
    for mode in ["fma-out", "fma-mul1", "fma-mul2", "fma-add", "driver"] {
        let c_signal = null_probe_signal(&c_library_path(), mode);
        let rust_signal = null_probe_signal(&rust_library_path(), mode);
        assert!(
            c_signal.is_some(),
            "C null probe unexpectedly survived: {mode}"
        );
        assert_eq!(c_signal, rust_signal, "null probe signal for {mode}");
    }
}
