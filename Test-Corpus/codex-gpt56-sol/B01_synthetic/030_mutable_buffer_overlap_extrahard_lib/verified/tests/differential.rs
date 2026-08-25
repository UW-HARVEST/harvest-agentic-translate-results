use libloading::Library;
use std::ffi::{c_int, c_void};
use std::fs::File;
use std::io::Read;
use std::os::fd::FromRawFd;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

type FmaArray = unsafe extern "C" fn(
    out: *mut c_int,
    mul1: *const c_int,
    mul2: *const c_int,
    add: *const c_int,
    len: c_int,
);
type Driver = unsafe extern "C" fn(data: *const c_int, len: c_int);

struct Api {
    _library: Library,
    fma_array: FmaArray,
    driver: Driver,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let fma_array = unsafe { *library.get::<FmaArray>(b"fma_array\0").unwrap() };
        let driver = unsafe { *library.get::<Driver>(b"driver\0").unwrap() };
        Self {
            _library: library,
            fma_array,
            driver,
        }
    }
}

fn c_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("c_src")
        .join("build")
        .join("libdriver.so")
}

fn rust_library_path() -> PathBuf {
    let executable = std::env::current_exe().unwrap();
    let profile_dir = executable
        .parent()
        .and_then(Path::parent)
        .expect("integration test must run under target/<profile>/deps");
    let direct = profile_dir.join("libdriver.so");
    if direct.is_file() {
        return direct;
    }

    let deps = profile_dir.join("deps");
    let mut candidates: Vec<_> = std::fs::read_dir(&deps)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("libdriver") && name.ends_with(".so"))
        })
        .collect();
    candidates.sort();
    candidates.pop().unwrap_or_else(|| {
        panic!(
            "Rust cdylib not found in {} or {}",
            direct.display(),
            deps.display()
        )
    })
}

fn apis() -> (Api, Api) {
    let c_path = c_library_path();
    assert!(
        c_path.is_file(),
        "build the C reference library first: {}",
        c_path.display()
    );
    let rust_path = rust_library_path();
    unsafe { (Api::load(&c_path), Api::load(&rust_path)) }
}

#[derive(Clone, Copy, Debug)]
enum AliasCase {
    Disjoint,
    OutMul1,
    OutMul2,
    OutAdd,
    Full,
    SharedMultiplicands,
    OutAfterInputs,
    InputsAfterOut,
}

#[derive(Clone)]
struct Buffers {
    out: Vec<i32>,
    mul1: Vec<i32>,
    mul2: Vec<i32>,
    add: Vec<i32>,
}

fn invoke_fma(api: &Api, case: AliasCase, mut buffers: Buffers, len: usize) -> Vec<Vec<i32>> {
    unsafe {
        match case {
            AliasCase::Disjoint => (api.fma_array)(
                buffers.out.as_mut_ptr(),
                buffers.mul1.as_ptr(),
                buffers.mul2.as_ptr(),
                buffers.add.as_ptr(),
                len as c_int,
            ),
            AliasCase::OutMul1 => {
                let shared = buffers.mul1.as_mut_ptr();
                (api.fma_array)(
                    shared,
                    shared,
                    buffers.mul2.as_ptr(),
                    buffers.add.as_ptr(),
                    len as c_int,
                );
            }
            AliasCase::OutMul2 => {
                let shared = buffers.mul2.as_mut_ptr();
                (api.fma_array)(
                    shared,
                    buffers.mul1.as_ptr(),
                    shared,
                    buffers.add.as_ptr(),
                    len as c_int,
                );
            }
            AliasCase::OutAdd => {
                let shared = buffers.add.as_mut_ptr();
                (api.fma_array)(
                    shared,
                    buffers.mul1.as_ptr(),
                    buffers.mul2.as_ptr(),
                    shared,
                    len as c_int,
                );
            }
            AliasCase::Full => {
                let shared = buffers.out.as_mut_ptr();
                (api.fma_array)(shared, shared, shared, shared, len as c_int);
            }
            AliasCase::SharedMultiplicands => {
                let shared = buffers.mul1.as_ptr();
                (api.fma_array)(
                    buffers.out.as_mut_ptr(),
                    shared,
                    shared,
                    buffers.add.as_ptr(),
                    len as c_int,
                );
            }
            AliasCase::OutAfterInputs => {
                let base = buffers.out.as_mut_ptr();
                (api.fma_array)(base.add(1), base, base, base, len as c_int);
            }
            AliasCase::InputsAfterOut => {
                let base = buffers.out.as_mut_ptr();
                (api.fma_array)(base, base.add(1), base.add(1), base.add(1), len as c_int);
            }
        }
    }
    vec![buffers.out, buffers.mul1, buffers.mul2, buffers.add]
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_i32(&mut self) -> i32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0 as i32
    }

    fn usize_in(&mut self, range: std::ops::Range<usize>) -> usize {
        range.start + (self.next_i32() as u32 as usize % (range.end - range.start))
    }
}

fn random_buffers(rng: &mut Rng, len: usize, partial_alias: bool) -> Buffers {
    let allocated = len + usize::from(partial_alias);
    let mut values = || (0..allocated).map(|_| rng.next_i32()).collect();
    Buffers {
        out: values(),
        mul1: values(),
        mul2: values(),
        add: values(),
    }
}

fn compare_fma_case(case: AliasCase, lengths: impl Fn(&mut Rng) -> usize, rounds: usize) {
    let (c, rust) = apis();
    let mut rng = Rng::new(0x5eed_4d41_91a5_77c3 ^ case as u64);
    for round in 0..rounds {
        let len = lengths(&mut rng);
        let partial = matches!(case, AliasCase::OutAfterInputs | AliasCase::InputsAfterOut);
        let buffers = random_buffers(&mut rng, len, partial);
        let c_result = invoke_fma(&c, case, buffers.clone(), len);
        let rust_result = invoke_fma(&rust, case, buffers, len);
        assert_eq!(
            c_result, rust_result,
            "{case:?} diverged at randomized round {round}, len={len}"
        );
    }
}

fn symbols_load_from_both_shared_libraries() {
    let _ = apis();
}

fn config_01_fma_zero_length() {
    compare_fma_case(AliasCase::Disjoint, |_| 0, 64);
}

fn config_02_fma_one_element() {
    compare_fma_case(AliasCase::Disjoint, |_| 1, 128);
}

fn config_03_fma_many_disjoint() {
    compare_fma_case(AliasCase::Disjoint, |rng| rng.usize_in(2..257), 128);
}

fn config_04_fma_out_aliases_mul1() {
    compare_fma_case(AliasCase::OutMul1, |rng| rng.usize_in(2..257), 128);
}

fn config_05_fma_out_aliases_mul2() {
    compare_fma_case(AliasCase::OutMul2, |rng| rng.usize_in(2..257), 128);
}

fn config_06_fma_out_aliases_add() {
    compare_fma_case(AliasCase::OutAdd, |rng| rng.usize_in(2..257), 128);
}

fn config_07_fma_full_alias() {
    compare_fma_case(AliasCase::Full, |rng| rng.usize_in(2..257), 128);
}

fn config_08_fma_input_only_alias() {
    compare_fma_case(
        AliasCase::SharedMultiplicands,
        |rng| rng.usize_in(2..257),
        128,
    );
}

fn config_09_fma_output_follows_inputs() {
    compare_fma_case(AliasCase::OutAfterInputs, |rng| rng.usize_in(2..257), 128);
}

fn config_10_fma_inputs_follow_output() {
    compare_fma_case(AliasCase::InputsAfterOut, |rng| rng.usize_in(2..257), 128);
}

unsafe extern "C" {
    fn pipe(pipefd: *mut c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

static STDOUT_LOCK: Mutex<()> = Mutex::new(());

fn capture_stdout(call: impl FnOnce()) -> Vec<u8> {
    let _guard = STDOUT_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let mut pipe_fds = [-1; 2];
    unsafe {
        assert_eq!(fflush(std::ptr::null_mut()), 0);
        assert_eq!(pipe(pipe_fds.as_mut_ptr()), 0);
        let saved_stdout = dup(1);
        assert!(saved_stdout >= 0);
        assert_eq!(dup2(pipe_fds[1], 1), 1);
        assert_eq!(close(pipe_fds[1]), 0);

        call();

        assert_eq!(fflush(std::ptr::null_mut()), 0);
        assert_eq!(dup2(saved_stdout, 1), 1);
        assert_eq!(close(saved_stdout), 0);
    }

    let mut output = Vec::new();
    unsafe { File::from_raw_fd(pipe_fds[0]) }
        .read_to_end(&mut output)
        .unwrap();
    output
}

fn compare_driver(lengths: impl Fn(&mut Rng) -> usize, rounds: usize) {
    let (c, rust) = apis();
    let mut rng = Rng::new(0xd81f_2a93_0527_4b6d);
    for round in 0..rounds {
        let len = lengths(&mut rng);
        let input: Vec<i32> = (0..len).map(|_| rng.next_i32()).collect();
        let c_output = capture_stdout(|| unsafe {
            (c.driver)(input.as_ptr(), len as c_int);
        });
        let rust_output = capture_stdout(|| unsafe {
            (rust.driver)(input.as_ptr(), len as c_int);
        });
        assert_eq!(
            c_output, rust_output,
            "driver diverged at randomized round {round}, len={len}"
        );
    }
}

fn config_11_driver_zero_length() {
    compare_driver(|_| 0, 64);
}

fn config_12_driver_one_element() {
    compare_driver(|_| 1, 128);
}

fn config_13_driver_many_elements() {
    compare_driver(|rng| rng.usize_in(2..129), 128);
}

fn config_14_driver_large_valid_input() {
    compare_driver(|_| 4096, 16);
}

fn error_boundaries_with_defined_c_behavior() {
    let (c, rust) = apis();
    for len in [0, -1, c_int::MIN] {
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

    let c_output = capture_stdout(|| unsafe {
        (c.driver)(std::ptr::null(), 0);
    });
    let rust_output = capture_stdout(|| unsafe {
        (rust.driver)(std::ptr::null(), 0);
    });
    assert_eq!(c_output, rust_output);
}

#[test]
fn all_config_and_error_rows_via_shared_libraries() {
    symbols_load_from_both_shared_libraries();
    config_01_fma_zero_length();
    config_02_fma_one_element();
    config_03_fma_many_disjoint();
    config_04_fma_out_aliases_mul1();
    config_05_fma_out_aliases_mul2();
    config_06_fma_out_aliases_add();
    config_07_fma_full_alias();
    config_08_fma_input_only_alias();
    config_09_fma_output_follows_inputs();
    config_10_fma_inputs_follow_output();
    config_11_driver_zero_length();
    config_12_driver_one_element();
    config_13_driver_many_elements();
    config_14_driver_large_valid_input();
    error_boundaries_with_defined_c_behavior();
}
