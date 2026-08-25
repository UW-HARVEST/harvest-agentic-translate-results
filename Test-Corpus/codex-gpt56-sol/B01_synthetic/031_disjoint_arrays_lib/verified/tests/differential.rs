use libloading::Library;
use std::ffi::{CString, c_char, c_int, c_void};
use std::fs::File;
use std::io::Read;
use std::os::fd::FromRawFd;
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::Mutex;

type FmaArray = unsafe extern "C" fn(*mut c_int, *const c_int, *const c_int, *const c_int, c_int);
type CallFma = unsafe extern "C" fn(*const c_int, c_int) -> c_int;
type Driver = unsafe extern "C" fn(*const c_char);

static STDOUT_LOCK: Mutex<()> = Mutex::new(());

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(pipefd: *mut c_int) -> c_int;
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
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_path = root.join("c_src/build/libdriver.so");
        let rust_path = root.join("target/debug/libdriver.so");
        assert!(c_path.is_file(), "missing C library: {}", c_path.display());
        assert!(
            rust_path.is_file(),
            "missing Rust library: {}",
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

#[derive(Clone)]
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u32(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0 as u32
    }

    fn usize_in(&mut self, start: usize, end: usize) -> usize {
        start + self.next_u32() as usize % (end - start)
    }

    fn bounded_i32(&mut self) -> i32 {
        (self.next_u32() % 20_001) as i32 - 10_000
    }
}

fn compare_fma(apis: &Apis, mul1: &[i32], mul2: &[i32], add: &[i32], initial: &[i32], len: i32) {
    let mut c_out = initial.to_vec();
    let mut rust_out = initial.to_vec();
    unsafe {
        (apis.c.fma_array)(
            c_out.as_mut_ptr(),
            mul1.as_ptr(),
            mul2.as_ptr(),
            add.as_ptr(),
            len,
        );
        (apis.rust.fma_array)(
            rust_out.as_mut_ptr(),
            mul1.as_ptr(),
            mul2.as_ptr(),
            add.as_ptr(),
            len,
        );
    }
    assert_eq!(c_out, rust_out);
}

fn compare_call_fma(apis: &Apis, data: &[i32]) {
    let c_result = unsafe { (apis.c.call_fma)(data.as_ptr(), data.len() as i32) };
    let rust_result = unsafe { (apis.rust.call_fma)(data.as_ptr(), data.len() as i32) };
    assert_eq!(c_result, rust_result, "data length {}", data.len());
}

fn capture_stdout(call: impl FnOnce()) -> Vec<u8> {
    const STDOUT_FILENO: c_int = 1;
    let mut pipe_fds = [-1; 2];
    unsafe {
        assert_eq!(fflush(ptr::null_mut()), 0);
        assert_eq!(pipe(pipe_fds.as_mut_ptr()), 0);
        let saved_stdout = dup(STDOUT_FILENO);
        assert!(saved_stdout >= 0);
        assert_eq!(dup2(pipe_fds[1], STDOUT_FILENO), STDOUT_FILENO);
        assert_eq!(close(pipe_fds[1]), 0);

        call();

        assert_eq!(fflush(ptr::null_mut()), 0);
        assert_eq!(dup2(saved_stdout, STDOUT_FILENO), STDOUT_FILENO);
        assert_eq!(close(saved_stdout), 0);
    }

    let mut output = Vec::new();
    unsafe { File::from_raw_fd(pipe_fds[0]) }
        .read_to_end(&mut output)
        .unwrap();
    output
}

fn driver_output(driver: Driver, input: &str) -> Vec<u8> {
    let input = CString::new(input).unwrap();
    capture_stdout(|| unsafe { driver(input.as_ptr()) })
}

fn compare_driver(apis: &Apis, input: &str) {
    let c_output = driver_output(apis.c.driver, input);
    let rust_output = driver_output(apis.rust.driver, input);
    assert_eq!(
        c_output, rust_output,
        "driver output differs for input {input:?}"
    );
}

fn integer_input(values: &[i32], rng: &mut Rng) -> String {
    const SEPARATORS: [&str; 6] = [" ", "  ", "\t", "\n", " \t", "\n  "];
    let mut input = String::new();
    if rng.next_u32().is_multiple_of(2) {
        input.push_str(SEPARATORS[rng.usize_in(0, SEPARATORS.len())]);
    }
    for (index, value) in values.iter().enumerate() {
        if index != 0 {
            input.push_str(SEPARATORS[rng.usize_in(0, SEPARATORS.len())]);
        }
        if *value >= 0 && rng.next_u32().is_multiple_of(2) {
            input.push('+');
        }
        input.push_str(&value.to_string());
    }
    input
}

#[test]
fn fma_array_matches_for_c1_through_c3() {
    let apis = Apis::load();

    unsafe {
        (apis.c.fma_array)(ptr::null_mut(), ptr::null(), ptr::null(), ptr::null(), 0);
        (apis.rust.fma_array)(ptr::null_mut(), ptr::null(), ptr::null(), ptr::null(), 0);
        (apis.c.fma_array)(ptr::null_mut(), ptr::null(), ptr::null(), ptr::null(), -1);
        (apis.rust.fma_array)(ptr::null_mut(), ptr::null(), ptr::null(), ptr::null(), -1);
    }
    for len in [-1, 0] {
        compare_fma(&apis, &[], &[], &[], &[11, 22, 33], len);
    }

    let mut rng = Rng::new(0x6b17_4567_e2a9_4c31);
    for _ in 0..256 {
        let mul1 = [rng.bounded_i32()];
        let mul2 = [rng.bounded_i32()];
        let add = [rng.bounded_i32()];
        compare_fma(&apis, &mul1, &mul2, &add, &[rng.bounded_i32()], 1);
    }

    for iteration in 0..256 {
        let len = if iteration == 0 {
            4_096
        } else {
            rng.usize_in(2, 129)
        };
        let mul1: Vec<_> = (0..len).map(|_| rng.bounded_i32()).collect();
        let mul2: Vec<_> = (0..len).map(|_| rng.bounded_i32()).collect();
        let add: Vec<_> = (0..len).map(|_| rng.bounded_i32()).collect();
        let initial: Vec<_> = (0..len).map(|_| rng.bounded_i32()).collect();
        compare_fma(&apis, &mul1, &mul2, &add, &initial, len as i32);
    }
}

#[test]
fn call_fma_matches_for_c4_through_c6() {
    let apis = Apis::load();
    let c_result = unsafe { (apis.c.call_fma)(ptr::null(), 0) };
    let rust_result = unsafe { (apis.rust.call_fma)(ptr::null(), 0) };
    assert_eq!(c_result, rust_result);
    assert_eq!(c_result, 0);

    let mut rng = Rng::new(0x98c2_728d_f151_947a);
    for _ in 0..256 {
        compare_call_fma(&apis, &[rng.next_u32() as i32]);
    }
    for iteration in 0..256 {
        let len = if iteration == 0 {
            4_096
        } else {
            rng.usize_in(2, 257)
        };
        let data: Vec<_> = (0..len).map(|_| rng.next_u32() as i32).collect();
        compare_call_fma(&apis, &data);
    }
}

#[test]
fn driver_matches_for_c7_through_c10() {
    let _stdout_guard = STDOUT_LOCK.lock().unwrap();
    let apis = Apis::load();
    let mut rng = Rng::new(0xc03a_5a17_d2e4_090b);

    for input in ["", " ", "\t\n", "x", "no integer", "--1", "+"] {
        compare_driver(&apis, input);
    }
    for _ in 0..128 {
        let invalid = format!("x{:08x}", rng.next_u32());
        compare_driver(&apis, &invalid);
    }

    for _ in 0..256 {
        let values = [rng.next_u32() as i32];
        let mut input = integer_input(&values, &mut rng);
        if rng.next_u32().is_multiple_of(2) {
            input.push_str(" trailing");
        }
        compare_driver(&apis, &input);
    }

    for _ in 0..256 {
        let count = rng.usize_in(2, 100);
        let values: Vec<_> = (0..count).map(|_| rng.next_u32() as i32).collect();
        let mut input = integer_input(&values, &mut rng);
        if rng.next_u32().is_multiple_of(2) {
            input.push_str(" stop");
        }
        compare_driver(&apis, &input);
    }

    for _ in 0..256 {
        let count = rng.usize_in(100, 121);
        let values: Vec<_> = (0..count).map(|_| rng.next_u32() as i32).collect();
        let input = integer_input(&values, &mut rng);
        compare_driver(&apis, &input);
    }
}

#[test]
fn driver_matches_error_e1_exactly() {
    let _stdout_guard = STDOUT_LOCK.lock().unwrap();
    let apis = Apis::load();
    let mut rng = Rng::new(0x355f_a37b_1570_119d);

    for _ in 0..256 {
        let count = rng.usize_in(0, 100);
        let values: Vec<_> = (0..count).map(|_| rng.next_u32() as i32).collect();
        let mut input = integer_input(&values, &mut rng);
        input.push_str(" rejected");
        compare_driver(&apis, &input);
    }
}
