use libloading::Library;
use std::env;
use std::ffi::{CString, c_char, c_int, c_void};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use std::ptr;
use std::sync::{Mutex, MutexGuard};

type FmaArray = unsafe extern "C" fn(
    out: *mut c_int,
    mul1: *const c_int,
    mul2: *const c_int,
    add: *const c_int,
    len: c_int,
);
type CallFma = unsafe extern "C" fn(data: *const c_int, len: c_int) -> c_int;
type Driver = unsafe extern "C" fn(input: *const c_char);

unsafe extern "C" {
    fn pipe(fds: *mut c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn read(fd: c_int, buf: *mut c_void, count: usize) -> isize;
    fn fflush(stream: *mut c_void) -> c_int;
}

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

struct Apis {
    c: Api,
    rust: Api,
}

impl Apis {
    fn load() -> Self {
        unsafe {
            Self {
                c: Api::load(&c_library_path()),
                rust: Api::load(&rust_library_path()),
            }
        }
    }
}

static TEST_LOCK: Mutex<()> = Mutex::new(());

fn serial() -> MutexGuard<'static, ()> {
    TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    env::var_os("DRIVER_C_SO")
        .map(PathBuf::from)
        .unwrap_or_else(|| manifest_dir().join("../c_src/build/libdriver.so"))
}

fn rust_library_path() -> PathBuf {
    env::var_os("DRIVER_RUST_SO")
        .map(PathBuf::from)
        .unwrap_or_else(|| manifest_dir().join("target/release/libdriver.so"))
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

    fn next_i32(&mut self) -> i32 {
        self.next_u64() as i32
    }

    fn length(&mut self, minimum: usize, maximum: usize) -> usize {
        minimum + (self.next_u64() as usize % (maximum - minimum + 1))
    }
}

fn random_vec(rng: &mut Rng, len: usize) -> Vec<c_int> {
    (0..len).map(|_| rng.next_i32()).collect()
}

fn run_fma(api: &Api, mul1: &[c_int], mul2: &[c_int], add: &[c_int]) -> Vec<c_int> {
    assert_eq!(mul1.len(), mul2.len());
    assert_eq!(mul1.len(), add.len());
    let mut out = vec![0x5a5a_5a5a; mul1.len()];
    unsafe {
        (api.fma_array)(
            out.as_mut_ptr(),
            mul1.as_ptr(),
            mul2.as_ptr(),
            add.as_ptr(),
            mul1.len() as c_int,
        );
    }
    out
}

fn assert_fma_matches(apis: &Apis, mul1: &[c_int], mul2: &[c_int], add: &[c_int]) {
    assert_eq!(
        run_fma(&apis.c, mul1, mul2, add),
        run_fma(&apis.rust, mul1, mul2, add)
    );
}

fn call_fma(api: &Api, data: &[c_int]) -> c_int {
    unsafe { (api.call_fma)(data.as_ptr(), data.len() as c_int) }
}

fn capture_stdout(call: impl FnOnce()) -> Vec<u8> {
    const STDOUT_FILENO: c_int = 1;
    unsafe {
        assert_eq!(fflush(ptr::null_mut()), 0);
        let mut fds = [-1; 2];
        assert_eq!(pipe(fds.as_mut_ptr()), 0);
        let saved_stdout = dup(STDOUT_FILENO);
        assert!(saved_stdout >= 0);
        assert_eq!(dup2(fds[1], STDOUT_FILENO), STDOUT_FILENO);
        assert_eq!(close(fds[1]), 0);

        call();

        assert_eq!(fflush(ptr::null_mut()), 0);
        assert_eq!(dup2(saved_stdout, STDOUT_FILENO), STDOUT_FILENO);
        assert_eq!(close(saved_stdout), 0);

        let mut output = Vec::new();
        let mut buffer = [0_u8; 256];
        loop {
            let count = read(fds[0], buffer.as_mut_ptr().cast(), buffer.len());
            assert!(count >= 0);
            if count == 0 {
                break;
            }
            output.extend_from_slice(&buffer[..count as usize]);
        }
        assert_eq!(close(fds[0]), 0);
        output
    }
}

fn run_driver(api: &Api, input: &str) -> Vec<u8> {
    let input = CString::new(input).unwrap();
    capture_stdout(|| unsafe { (api.driver)(input.as_ptr()) })
}

fn assert_driver_matches(apis: &Apis, input: &str) {
    let c = run_driver(&apis.c, input);
    let rust = run_driver(&apis.rust, input);
    assert_eq!(c, rust, "input: {input:?}");
}

fn decimal_input(values: &[c_int]) -> String {
    values
        .iter()
        .map(c_int::to_string)
        .collect::<Vec<_>>()
        .join(" ")
}

#[test]
fn config_01_fma_zero_length() {
    let _guard = serial();
    let apis = Apis::load();
    unsafe {
        (apis.c.fma_array)(ptr::null_mut(), ptr::null(), ptr::null(), ptr::null(), 0);
        (apis.rust.fma_array)(ptr::null_mut(), ptr::null(), ptr::null(), ptr::null(), 0);
    }
}

#[test]
fn config_02_fma_one_element_randomized() {
    let _guard = serial();
    let apis = Apis::load();
    let mut rng = Rng::new(0x2e65_721f_90ab_11cd);
    for _ in 0..1_000 {
        assert_fma_matches(
            &apis,
            &[rng.next_i32()],
            &[rng.next_i32()],
            &[rng.next_i32()],
        );
    }
}

#[test]
fn config_03_fma_many_elements_randomized() {
    let _guard = serial();
    let apis = Apis::load();
    let mut rng = Rng::new(0x73c2_1777_0bf3_6621);
    for _ in 0..250 {
        let len = rng.length(2, 256);
        let mul1 = random_vec(&mut rng, len);
        let mul2 = random_vec(&mut rng, len);
        let add = random_vec(&mut rng, len);
        assert_fma_matches(&apis, &mul1, &mul2, &add);
    }
}

#[test]
fn config_04_fma_large_valid_length() {
    let _guard = serial();
    let apis = Apis::load();
    let mut rng = Rng::new(0xbc44_9d12_544a_002f);
    let len = 16_384;
    let mul1 = random_vec(&mut rng, len);
    let mul2 = random_vec(&mut rng, len);
    let add = random_vec(&mut rng, len);
    assert_fma_matches(&apis, &mul1, &mul2, &add);
}

#[test]
fn config_05_call_fma_zero_length() {
    let _guard = serial();
    let apis = Apis::load();
    unsafe {
        assert_eq!((apis.c.call_fma)(ptr::null(), 0), 0);
        assert_eq!((apis.rust.call_fma)(ptr::null(), 0), 0);
    }
}

#[test]
fn config_06_call_fma_one_element_randomized() {
    let _guard = serial();
    let apis = Apis::load();
    let mut rng = Rng::new(0xdd3f_0a29_d640_aab1);
    for _ in 0..1_000 {
        let data = [rng.next_i32()];
        assert_eq!(call_fma(&apis.c, &data), call_fma(&apis.rust, &data));
    }
}

#[test]
fn config_07_call_fma_many_elements_randomized() {
    let _guard = serial();
    let apis = Apis::load();
    let mut rng = Rng::new(0x8a92_3184_c95e_bf40);
    for _ in 0..250 {
        let len = rng.length(2, 512);
        let data = random_vec(&mut rng, len);
        assert_eq!(call_fma(&apis.c, &data), call_fma(&apis.rust, &data));
    }
}

#[test]
fn config_08_call_fma_large_valid_length() {
    let _guard = serial();
    let apis = Apis::load();
    let mut rng = Rng::new(0xcdfa_d974_9a4e_b52d);
    let data = random_vec(&mut rng, 16_384);
    assert_eq!(call_fma(&apis.c, &data), call_fma(&apis.rust, &data));
}

#[test]
fn config_09_driver_zero_scans() {
    let _guard = serial();
    let apis = Apis::load();
    for input in ["", "x", "   ", "+", "--1", "\t\nx 1"] {
        assert_driver_matches(&apis, input);
        assert_eq!(run_driver(&apis.c, input), b"0\n");
    }
}

#[test]
fn config_10_driver_one_integer_lexical_variants() {
    let _guard = serial();
    let apis = Apis::load();
    for input in [
        "0",
        "-1",
        "+17",
        "  42",
        "\t-2147483648\n",
        "2147483647 trailing",
    ] {
        assert_driver_matches(&apis, input);
    }
}

#[test]
fn config_11_driver_two_to_ninety_nine_randomized() {
    let _guard = serial();
    let apis = Apis::load();
    let mut rng = Rng::new(0x25e0_c54a_d9e4_7f37);
    for _ in 0..200 {
        let len = rng.length(2, 99);
        let values = random_vec(&mut rng, len);
        assert_driver_matches(&apis, &decimal_input(&values));
    }
}

#[test]
fn config_12_driver_malformed_suffix_randomized() {
    let _guard = serial();
    let apis = Apis::load();
    let mut rng = Rng::new(0x309a_b31f_2291_c777);
    for _ in 0..200 {
        let len = rng.length(1, 99);
        let values = random_vec(&mut rng, len);
        let input = format!("{} x {}", decimal_input(&values), rng.next_i32());
        assert_driver_matches(&apis, &input);
    }
}

#[test]
fn config_13_driver_exactly_one_hundred() {
    let _guard = serial();
    let apis = Apis::load();
    let mut rng = Rng::new(0xcb3a_690d_a3bf_af76);
    for _ in 0..50 {
        let values = random_vec(&mut rng, 100);
        assert_driver_matches(&apis, &decimal_input(&values));
    }
}

#[test]
fn config_14_driver_more_than_one_hundred() {
    let _guard = serial();
    let apis = Apis::load();
    let mut rng = Rng::new(0x40f1_ef7b_cda2_6974);
    for _ in 0..50 {
        let len = rng.length(101, 180);
        let values = random_vec(&mut rng, len);
        assert_driver_matches(&apis, &decimal_input(&values));
    }
}

#[test]
fn error_g2_fma_negative_length_noop() {
    let _guard = serial();
    let apis = Apis::load();
    unsafe {
        (apis.c.fma_array)(ptr::null_mut(), ptr::null(), ptr::null(), ptr::null(), -1);
        (apis.rust.fma_array)(ptr::null_mut(), ptr::null(), ptr::null(), ptr::null(), -1);
    }
}

fn run_crash_probe(library: &str, case: &str) -> ExitStatus {
    Command::new(env::current_exe().unwrap())
        .arg("--exact")
        .arg("ffi_crash_probe")
        .arg("--nocapture")
        .env("DRIVER_PROBE_LIBRARY", library)
        .env("DRIVER_PROBE_CASE", case)
        .status()
        .unwrap()
}

#[cfg(unix)]
fn assert_same_signal(case: &str) {
    use std::os::unix::process::ExitStatusExt;

    let c = run_crash_probe("c", case);
    let rust = run_crash_probe("rust", case);
    assert!(!c.success(), "C unexpectedly succeeded for {case}");
    assert!(!rust.success(), "Rust unexpectedly succeeded for {case}");
    assert_eq!(c.signal(), rust.signal(), "different signals for {case}");
    assert!(c.signal().is_some(), "C was not terminated by a signal");
}

#[test]
fn error_g3_g5_g6_null_dereferences_match_process_result() {
    let _guard = serial();
    for case in ["fma_null", "call_fma_null", "driver_null"] {
        assert_same_signal(case);
    }
}

#[test]
fn ffi_crash_probe() {
    let Ok(library_name) = env::var("DRIVER_PROBE_LIBRARY") else {
        return;
    };
    let case = env::var("DRIVER_PROBE_CASE").unwrap();
    let path = match library_name.as_str() {
        "c" => c_library_path(),
        "rust" => rust_library_path(),
        other => panic!("unknown probe library {other}"),
    };
    let api = unsafe { Api::load(&path) };
    unsafe {
        match case.as_str() {
            "fma_null" => {
                (api.fma_array)(ptr::null_mut(), ptr::null(), ptr::null(), ptr::null(), 1);
            }
            "call_fma_null" => {
                (api.call_fma)(ptr::null(), 1);
            }
            "driver_null" => {
                (api.driver)(ptr::null());
            }
            other => panic!("unknown probe case {other}"),
        }
    }
}
