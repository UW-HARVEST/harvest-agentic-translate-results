use libloading::Library;
use std::ffi::{CString, c_char, c_int, c_void};
use std::fs::File;
use std::io::Read;
use std::os::fd::FromRawFd;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{LazyLock, Mutex};

type FmaArray = unsafe extern "C" fn(*mut c_int, *const c_int, *const c_int, *const c_int, c_int);
type CallFma = unsafe extern "C" fn(*const c_int, c_int) -> c_int;
type Driver = unsafe extern "C" fn(*const c_char);

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(pipe_fds: *mut c_int) -> c_int;
}

const STDOUT_FILENO: c_int = 1;
static STDOUT_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

struct Api {
    _library: Library,
    fma_array: FmaArray,
    call_fma: CallFma,
    driver: Driver,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let fma_array = unsafe { *library.get::<FmaArray>(b"fma_array\0").unwrap() };
        let call_fma = unsafe { *library.get::<CallFma>(b"call_fma\0").unwrap() };
        let driver = unsafe { *library.get::<Driver>(b"driver\0").unwrap() };
        Self {
            _library: library,
            fma_array,
            call_fma,
            driver,
        }
    }
}

struct Libraries {
    c: Api,
    rust: Api,
}

impl Libraries {
    fn load() -> Self {
        let c_path = c_library_path();
        let rust_path = rust_library_path();
        assert!(
            c_path.is_file(),
            "C library does not exist at {}; build it first",
            c_path.display()
        );
        assert!(
            rust_path.is_file(),
            "Rust library does not exist at {}",
            rust_path.display()
        );
        unsafe {
            Self {
                c: Api::load(&c_path),
                rust: Api::load(&rust_path),
            }
        }
    }
}

fn c_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../c_src/build/libdriver.so")
        .canonicalize()
        .unwrap()
}

fn rust_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target/release")
        .join("libdriver.so")
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u32(&mut self) -> u32 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value as u32
    }

    fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }

    fn length(&mut self, minimum: usize, maximum: usize) -> usize {
        minimum + self.next_u32() as usize % (maximum - minimum + 1)
    }
}

fn random_ints(rng: &mut Rng, len: usize) -> Vec<c_int> {
    (0..len).map(|_| rng.next_i32()).collect()
}

fn compare_fma(libraries: &Libraries, mul1: &[c_int], mul2: &[c_int], add: &[c_int]) {
    assert_eq!(mul1.len(), mul2.len());
    assert_eq!(mul1.len(), add.len());
    let mut c_out = vec![0x1357_2468; mul1.len()];
    let mut rust_out = c_out.clone();
    unsafe {
        (libraries.c.fma_array)(
            c_out.as_mut_ptr(),
            mul1.as_ptr(),
            mul2.as_ptr(),
            add.as_ptr(),
            mul1.len() as c_int,
        );
        (libraries.rust.fma_array)(
            rust_out.as_mut_ptr(),
            mul1.as_ptr(),
            mul2.as_ptr(),
            add.as_ptr(),
            mul1.len() as c_int,
        );
    }
    assert_eq!(c_out, rust_out);
}

fn compare_call_fma(libraries: &Libraries, data: &[c_int]) {
    let c_result = unsafe { (libraries.c.call_fma)(data.as_ptr(), data.len() as c_int) };
    let rust_result = unsafe { (libraries.rust.call_fma)(data.as_ptr(), data.len() as c_int) };
    assert_eq!(c_result, rust_result);
}

fn capture_stdout(call: impl FnOnce()) -> Vec<u8> {
    let _guard = STDOUT_LOCK.lock().unwrap();
    let mut pipe_fds = [-1; 2];
    unsafe {
        assert_eq!(fflush(std::ptr::null_mut()), 0);
        assert_eq!(pipe(pipe_fds.as_mut_ptr()), 0);
        let saved_stdout = dup(STDOUT_FILENO);
        assert!(saved_stdout >= 0);
        assert_eq!(dup2(pipe_fds[1], STDOUT_FILENO), STDOUT_FILENO);
        assert_eq!(close(pipe_fds[1]), 0);

        call();

        assert_eq!(fflush(std::ptr::null_mut()), 0);
        assert_eq!(dup2(saved_stdout, STDOUT_FILENO), STDOUT_FILENO);
        assert_eq!(close(saved_stdout), 0);
    }

    let mut output = Vec::new();
    unsafe { File::from_raw_fd(pipe_fds[0]) }
        .read_to_end(&mut output)
        .unwrap();
    output
}

fn call_driver(driver: Driver, input: &str) -> Vec<u8> {
    let input = CString::new(input).unwrap();
    capture_stdout(|| unsafe { driver(input.as_ptr()) })
}

fn compare_driver(libraries: &Libraries, input: &str) {
    let c_output = call_driver(libraries.c.driver, input);
    let rust_output = call_driver(libraries.rust.driver, input);
    assert_eq!(c_output, rust_output, "input: {input:?}");
}

fn render_values(values: &[c_int], style: usize) -> String {
    let mut output = String::new();
    if style == 1 {
        output.push_str(" \t");
    }
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            match style {
                0 => output.push(' '),
                1 => output.push_str(if index % 2 == 0 { "\n" } else { "\t" }),
                2 => {}
                _ => unreachable!(),
            }
        }
        if style == 2 && index > 0 && *value >= 0 {
            output.push('+');
        }
        output.push_str(&value.to_string());
    }
    output
}

#[test]
fn config_01_fma_zero_length() {
    let libraries = Libraries::load();
    unsafe {
        (libraries.c.fma_array)(
            std::ptr::null_mut(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            0,
        );
        (libraries.rust.fma_array)(
            std::ptr::null_mut(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            0,
        );
    }
}

#[test]
fn config_02_fma_one_element_randomized() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x02ca_feba_be12_3456);
    for _ in 0..256 {
        compare_fma(
            &libraries,
            &[rng.next_i32()],
            &[rng.next_i32()],
            &[rng.next_i32()],
        );
    }
}

#[test]
fn config_03_fma_many_elements_randomized() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x03ca_feba_be12_3456);
    for _ in 0..128 {
        let len = rng.length(2, 257);
        let mul1 = random_ints(&mut rng, len);
        let mul2 = random_ints(&mut rng, len);
        let add = random_ints(&mut rng, len);
        compare_fma(&libraries, &mul1, &mul2, &add);
    }
}

#[test]
fn config_04_call_fma_zero_length() {
    let libraries = Libraries::load();
    let c_result = unsafe { (libraries.c.call_fma)(std::ptr::null(), 0) };
    let rust_result = unsafe { (libraries.rust.call_fma)(std::ptr::null(), 0) };
    assert_eq!(c_result, 0);
    assert_eq!(c_result, rust_result);
}

#[test]
fn config_05_call_fma_one_element_randomized() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x05ca_feba_be12_3456);
    for _ in 0..256 {
        compare_call_fma(&libraries, &[rng.next_i32()]);
    }
}

#[test]
fn config_06_call_fma_many_elements_randomized() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x06ca_feba_be12_3456);
    for _ in 0..128 {
        let len = rng.length(2, 257);
        let data = random_ints(&mut rng, len);
        compare_call_fma(&libraries, &data);
    }
}

#[test]
fn config_07_call_fma_large_length_randomized() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x07ca_feba_be12_3456);
    for _ in 0..16 {
        let data = random_ints(&mut rng, 4096);
        compare_call_fma(&libraries, &data);
    }
}

#[test]
fn config_08_driver_one_integer_randomized() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x08ca_feba_be12_3456);
    for index in 0..128 {
        let value = rng.next_i32();
        let input = match index % 3 {
            0 => value.to_string(),
            1 => format!(" \t{value}"),
            2 if value >= 0 => format!("+{value}"),
            2 => value.to_string(),
            _ => unreachable!(),
        };
        compare_driver(&libraries, &input);
    }
}

#[test]
fn config_09_driver_two_to_ninety_nine_integers_randomized() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x09ca_feba_be12_3456);
    for index in 0..96 {
        let len = rng.length(2, 99);
        let values = random_ints(&mut rng, len);
        compare_driver(&libraries, &render_values(&values, index % 3));
    }
}

#[test]
fn config_10_driver_exactly_one_hundred_integers_randomized() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x10ca_feba_be12_3456);
    for index in 0..32 {
        let values = random_ints(&mut rng, 100);
        compare_driver(&libraries, &render_values(&values, index % 3));
    }
}

#[test]
fn config_11_driver_ignores_values_after_one_hundred() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x11ca_feba_be12_3456);
    for index in 0..32 {
        let len = rng.length(101, 140);
        let values = random_ints(&mut rng, len);
        compare_driver(&libraries, &render_values(&values, index % 3));
    }
}

#[test]
fn error_01_driver_rejects_first_token() {
    let libraries = Libraries::load();
    for input in ["", " ", "\t\n", "x", "--1", "+"] {
        let c_output = call_driver(libraries.c.driver, input);
        let rust_output = call_driver(libraries.rust.driver, input);
        assert_eq!(c_output, b"0\n", "C input: {input:?}");
        assert_eq!(c_output, rust_output, "input: {input:?}");
    }
}

#[test]
fn error_02_driver_rejects_later_token() {
    let libraries = Libraries::load();
    for input in ["7x", "1 2 nope 3", "-8 +9 --10", "4,5"] {
        compare_driver(&libraries, input);
    }
}

#[test]
fn boundary_fma_negative_length_is_a_no_op() {
    let libraries = Libraries::load();
    let mut c_output = 123;
    let mut rust_output = 123;
    unsafe {
        (libraries.c.fma_array)(
            &mut c_output,
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            -1,
        );
        (libraries.rust.fma_array)(
            &mut rust_output,
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            -1,
        );
    }
    assert_eq!(c_output, rust_output);
    assert_eq!(c_output, 123);
}

#[test]
fn null_dereference_probe() {
    let Ok(probe) = std::env::var("DRIVER_NULL_PROBE") else {
        return;
    };
    let (implementation, function) = probe.split_once(':').unwrap();
    let path = match implementation {
        "c" => c_library_path(),
        "rust" => rust_library_path(),
        _ => panic!("unknown implementation"),
    };
    let api = unsafe { Api::load(&path) };
    unsafe {
        match function {
            "fma_array" => (api.fma_array)(
                std::ptr::null_mut(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                1,
            ),
            "call_fma" => {
                (api.call_fma)(std::ptr::null(), 1);
            }
            "driver" => (api.driver)(std::ptr::null()),
            _ => panic!("unknown function"),
        }
    }
}

#[cfg(unix)]
#[test]
fn boundary_null_dereferences_terminate_for_both_libraries() {
    use std::os::unix::process::ExitStatusExt;

    let executable = std::env::current_exe().unwrap();
    for function in ["fma_array", "call_fma", "driver"] {
        let mut statuses = Vec::new();
        for implementation in ["c", "rust"] {
            let status = Command::new(&executable)
                .args(["--exact", "null_dereference_probe", "--nocapture"])
                .env("DRIVER_NULL_PROBE", format!("{implementation}:{function}"))
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .unwrap();
            assert!(
                !status.success(),
                "{implementation} {function} unexpectedly accepted null"
            );
            statuses.push((status.code(), status.signal()));
        }
        assert!(
            statuses
                .iter()
                .all(|(code, signal)| code.is_some() || signal.is_some()),
            "missing termination status for {function}: {statuses:?}"
        );
        assert_eq!(
            statuses[0], statuses[1],
            "C and Rust termination differ for null {function}"
        );
    }
}
