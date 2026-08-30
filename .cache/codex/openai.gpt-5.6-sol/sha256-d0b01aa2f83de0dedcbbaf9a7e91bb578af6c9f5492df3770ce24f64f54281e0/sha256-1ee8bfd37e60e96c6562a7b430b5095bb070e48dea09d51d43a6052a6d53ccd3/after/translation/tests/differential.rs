use libloading::Library;
use std::env;
use std::ffi::{CString, c_char, c_int, c_void};
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::Mutex;

type PrintLine = unsafe extern "C" fn(*const c_char);
type VoidFn = unsafe extern "C" fn();
type Driver = unsafe extern "C" fn(c_int);

static STDOUT_LOCK: Mutex<()> = Mutex::new(());

fn stdout_guard() -> std::sync::MutexGuard<'static, ()> {
    STDOUT_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(pipe_fds: *mut c_int) -> c_int;
    fn read(fd: c_int, buffer: *mut c_void, count: usize) -> isize;
}

struct Api {
    _library: Library,
    print_line: PrintLine,
    bad: VoidFn,
    good: VoidFn,
    driver: Driver,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library: Library = unsafe {
            libloading::os::unix::Library::open(
                Some(path),
                libloading::os::unix::RTLD_NOW | libloading::os::unix::RTLD_LOCAL,
            )
        }
        .map(Into::into)
        .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));

        let print_line = unsafe { *library.get::<PrintLine>(b"printLine\0").unwrap() };
        let bad = unsafe { *library.get::<VoidFn>(b"bad\0").unwrap() };
        let good = unsafe { *library.get::<VoidFn>(b"good\0").unwrap() };
        let driver = unsafe { *library.get::<Driver>(b"driver\0").unwrap() };

        Self {
            _library: library,
            print_line,
            bad,
            good,
            driver,
        }
    }
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn library_path(variable: &str, fallback: PathBuf) -> PathBuf {
    env::var_os(variable).map(PathBuf::from).unwrap_or(fallback)
}

unsafe fn load_apis() -> (Api, Api) {
    let root = workspace_root();
    let c_path = library_path("DRIVER_C_SO", root.join("c_src/build/libdriver.so"));
    let rust_path = library_path(
        "DRIVER_RUST_SO",
        root.join("translation/target/release/libdriver.so"),
    );

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

    (unsafe { Api::load(&c_path) }, unsafe {
        Api::load(&rust_path)
    })
}

unsafe fn capture_stdout(call: impl FnOnce()) -> Vec<u8> {
    const STDOUT_FILENO: c_int = 1;

    let mut pipe_fds = [-1; 2];
    assert_eq!(unsafe { fflush(ptr::null_mut()) }, 0);
    assert_eq!(unsafe { pipe(pipe_fds.as_mut_ptr()) }, 0);

    let saved_stdout = unsafe { dup(STDOUT_FILENO) };
    assert!(saved_stdout >= 0);
    assert_eq!(unsafe { dup2(pipe_fds[1], STDOUT_FILENO) }, STDOUT_FILENO);
    assert_eq!(unsafe { close(pipe_fds[1]) }, 0);

    call();

    assert_eq!(unsafe { fflush(ptr::null_mut()) }, 0);
    assert_eq!(unsafe { dup2(saved_stdout, STDOUT_FILENO) }, STDOUT_FILENO);
    assert_eq!(unsafe { close(saved_stdout) }, 0);

    let mut output = Vec::new();
    let mut chunk = [0_u8; 4096];
    loop {
        let count = unsafe {
            read(
                pipe_fds[0],
                chunk.as_mut_ptr().cast::<c_void>(),
                chunk.len(),
            )
        };
        assert!(count >= 0, "read from stdout capture pipe failed");
        if count == 0 {
            break;
        }
        output.extend_from_slice(&chunk[..count as usize]);
    }
    assert_eq!(unsafe { close(pipe_fds[0]) }, 0);
    output
}

fn assert_same_output(label: &str, c_call: impl FnOnce(), rust_call: impl FnOnce()) {
    let c_output = unsafe { capture_stdout(c_call) };
    let rust_output = unsafe { capture_stdout(rust_call) };
    assert_eq!(c_output, rust_output, "{label}");
}

#[cfg(target_arch = "x86_64")]
unsafe fn call_bad_with_zeroed_uninitialized_slot(function: VoidFn) {
    unsafe { call_bad_with_seeded_uninitialized_slot(function, ptr::null()) };
}

#[cfg(target_arch = "x86_64")]
unsafe fn call_bad_with_seeded_uninitialized_slot(function: VoidFn, seed: *const c_char) {
    unsafe {
        std::arch::asm!(
            "mov qword ptr [rsp - 24], {seed}",
            "call {function}",
            seed = in(reg) seed,
            function = in(reg) function,
            clobber_abi("C"),
        );
    }
}

#[cfg(not(target_arch = "x86_64"))]
unsafe fn call_bad_with_zeroed_uninitialized_slot(function: VoidFn) {
    unsafe { function() };
}

#[cfg(not(target_arch = "x86_64"))]
unsafe fn call_bad_with_seeded_uninitialized_slot(function: VoidFn, _seed: *const c_char) {
    unsafe { function() };
}

#[cfg(target_arch = "x86_64")]
unsafe fn call_driver_zero_with_zeroed_uninitialized_slot(function: Driver) {
    unsafe { call_driver_zero_with_seeded_uninitialized_slot(function, ptr::null()) };
}

#[cfg(target_arch = "x86_64")]
unsafe fn call_driver_zero_with_seeded_uninitialized_slot(function: Driver, seed: *const c_char) {
    unsafe {
        std::arch::asm!(
            "mov qword ptr [rsp - 56], {seed}",
            "call {function}",
            seed = in(reg) seed,
            function = in(reg) function,
            in("edi") 0_i32,
            clobber_abi("C"),
        );
    }
}

#[cfg(not(target_arch = "x86_64"))]
unsafe fn call_driver_zero_with_zeroed_uninitialized_slot(function: Driver) {
    unsafe { function(0) };
}

#[cfg(not(target_arch = "x86_64"))]
unsafe fn call_driver_zero_with_seeded_uninitialized_slot(function: Driver, _seed: *const c_char) {
    unsafe { function(0) };
}

struct Lcg(u64);

impl Lcg {
    fn next_u32(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 32) as u32
    }

    fn nonzero_byte(&mut self) -> u8 {
        (self.next_u32() % 255 + 1) as u8
    }
}

#[test]
fn config_1_print_line_matches_for_randomized_c_strings() {
    let _guard = stdout_guard();
    let (c, rust) = unsafe { load_apis() };
    let mut rng = Lcg(0x5eed_c0de_d15c_a11e);

    for case in 0..256 {
        let length = match case {
            0 => 0,
            1 => 1,
            _ => (rng.next_u32() as usize % 1024) + 2,
        };
        let bytes: Vec<u8> = (0..length).map(|_| rng.nonzero_byte()).collect();
        let input = CString::new(bytes).unwrap();

        assert_same_output(
            &format!("CONFIGS.md row 1, randomized case {case}"),
            || unsafe { (c.print_line)(input.as_ptr()) },
            || unsafe { (rust.print_line)(input.as_ptr()) },
        );
    }
}

#[test]
fn config_2_bad_matches_when_c_uninitialized_slot_is_null() {
    let _guard = stdout_guard();
    let (c, rust) = unsafe { load_apis() };

    for case in 0..64 {
        assert_same_output(
            &format!("CONFIGS.md row 2, repetition {case}"),
            || unsafe { call_bad_with_zeroed_uninitialized_slot(c.bad) },
            || unsafe { call_bad_with_zeroed_uninitialized_slot(rust.bad) },
        );
    }
}

#[test]
fn config_3_bad_matches_for_randomized_nonnull_stack_values() {
    let _guard = stdout_guard();
    let (c, rust) = unsafe { load_apis() };
    let mut rng = Lcg(0xbad0_cafe_5eed_0003);

    for case in 0..256 {
        let length = (rng.next_u32() as usize % 1024) + 1;
        let bytes: Vec<u8> = (0..length).map(|_| rng.nonzero_byte()).collect();
        let input = CString::new(bytes).unwrap();

        assert_same_output(
            &format!("CONFIGS.md row 3, randomized case {case}"),
            || unsafe { call_bad_with_seeded_uninitialized_slot(c.bad, input.as_ptr()) },
            || unsafe { call_bad_with_seeded_uninitialized_slot(rust.bad, input.as_ptr()) },
        );
    }
}

#[test]
fn config_4_good_matches() {
    let _guard = stdout_guard();
    let (c, rust) = unsafe { load_apis() };

    for case in 0..64 {
        assert_same_output(
            &format!("CONFIGS.md row 4, repetition {case}"),
            || unsafe { (c.good)() },
            || unsafe { (rust.good)() },
        );
    }
}

#[test]
fn config_5_driver_zero_matches_when_c_uninitialized_slot_is_null() {
    let _guard = stdout_guard();
    let (c, rust) = unsafe { load_apis() };

    for case in 0..64 {
        assert_same_output(
            &format!("CONFIGS.md row 5, repetition {case}"),
            || unsafe { call_driver_zero_with_zeroed_uninitialized_slot(c.driver) },
            || unsafe { call_driver_zero_with_zeroed_uninitialized_slot(rust.driver) },
        );
    }
}

#[test]
fn config_6_driver_zero_matches_for_randomized_nonnull_stack_values() {
    let _guard = stdout_guard();
    let (c, rust) = unsafe { load_apis() };
    let mut rng = Lcg(0xd012_e700_5eed_0006);

    for case in 0..256 {
        let length = (rng.next_u32() as usize % 1024) + 1;
        let bytes: Vec<u8> = (0..length).map(|_| rng.nonzero_byte()).collect();
        let input = CString::new(bytes).unwrap();

        assert_same_output(
            &format!("CONFIGS.md row 6, randomized case {case}"),
            || unsafe { call_driver_zero_with_seeded_uninitialized_slot(c.driver, input.as_ptr()) },
            || unsafe {
                call_driver_zero_with_seeded_uninitialized_slot(rust.driver, input.as_ptr())
            },
        );
    }
}

#[test]
fn config_7_driver_nonzero_matches_for_randomized_ints() {
    let _guard = stdout_guard();
    let (c, rust) = unsafe { load_apis() };
    let mut rng = Lcg(0xd1ff_3e11_71a1_5eed);

    for case in 0..256 {
        let mut use_good = rng.next_u32() as i32;
        if use_good == 0 {
            use_good = if case % 2 == 0 { 1 } else { -1 };
        }

        assert_same_output(
            &format!("CONFIGS.md row 7, randomized input {use_good}"),
            || unsafe { (c.driver)(use_good) },
            || unsafe { (rust.driver)(use_good) },
        );
    }
}

#[test]
fn error_1_print_line_null_matches_exactly() {
    let _guard = stdout_guard();
    let (c, rust) = unsafe { load_apis() };

    for case in 0..64 {
        assert_same_output(
            &format!("ERRORS.md row 1, repetition {case}"),
            || unsafe { (c.print_line)(ptr::null()) },
            || unsafe { (rust.print_line)(ptr::null()) },
        );
    }
}
