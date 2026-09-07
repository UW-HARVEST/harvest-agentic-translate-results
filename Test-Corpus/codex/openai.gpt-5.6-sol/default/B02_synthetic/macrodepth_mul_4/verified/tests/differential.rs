use libloading::{Library, Symbol};
use std::env;
use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::path::{Path, PathBuf};
use std::ptr;

type OpFn = unsafe extern "C" fn(c_int, c_int) -> c_int;
type UnaryFn = unsafe extern "C" fn(c_int) -> c_int;
type MainFn = unsafe extern "C" fn(c_int, *mut *mut c_char) -> c_int;

unsafe extern "C" {
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(fds: *mut c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn read(fd: c_int, buffer: *mut c_void, count: usize) -> isize;
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(status: c_int) -> !;
}

const STDOUT_FILENO: c_int = 1;
const STDERR_FILENO: c_int = 2;
const RANDOM_CASES: usize = 64;

struct Api {
    library: Library,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        Self {
            library: unsafe { Library::new(path) }
                .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display())),
        }
    }

    unsafe fn symbol<T: Copy>(&self, name: &[u8]) -> T {
        let symbol: Symbol<'_, T> = unsafe { self.library.get(name) }
            .unwrap_or_else(|error| panic!("missing {:?}: {error}", CStr::from_bytes_with_nul(name)));
        *symbol
    }

    unsafe fn global_op(&self) -> OpFn {
        let address = unsafe { self.symbol::<*mut OpFn>(b"G_OP\0") };
        unsafe { *address }
    }

    unsafe fn global_name(&self) -> Vec<u8> {
        let address = unsafe { self.symbol::<*mut *const c_char>(b"G_OP_NAME\0") };
        unsafe { CStr::from_ptr(*address) }.to_bytes_with_nul().to_vec()
    }
}

#[derive(Debug, PartialEq, Eq)]
struct Captured<T> {
    value: T,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

fn checked_pipe() -> (c_int, c_int) {
    let mut fds = [-1, -1];
    let result = unsafe { pipe(fds.as_mut_ptr()) };
    assert_eq!(result, 0, "pipe failed: {}", std::io::Error::last_os_error());
    (fds[0], fds[1])
}

fn read_all(fd: c_int) -> Vec<u8> {
    let mut output = Vec::new();
    let mut buffer = [0_u8; 4096];
    loop {
        let count = unsafe { read(fd, buffer.as_mut_ptr().cast(), buffer.len()) };
        if count == 0 {
            break;
        }
        assert!(
            count > 0,
            "read failed: {}",
            std::io::Error::last_os_error()
        );
        output.extend_from_slice(&buffer[..count as usize]);
    }
    assert_eq!(unsafe { close(fd) }, 0);
    output
}

fn capture<T>(call: impl FnOnce() -> T) -> Captured<T> {
    unsafe {
        fflush(ptr::null_mut());
    }

    let (stdout_read, stdout_write) = checked_pipe();
    let (stderr_read, stderr_write) = checked_pipe();
    let saved_stdout = unsafe { dup(STDOUT_FILENO) };
    let saved_stderr = unsafe { dup(STDERR_FILENO) };
    assert!(saved_stdout >= 0 && saved_stderr >= 0);
    assert_eq!(unsafe { dup2(stdout_write, STDOUT_FILENO) }, STDOUT_FILENO);
    assert_eq!(unsafe { dup2(stderr_write, STDERR_FILENO) }, STDERR_FILENO);
    assert_eq!(unsafe { close(stdout_write) }, 0);
    assert_eq!(unsafe { close(stderr_write) }, 0);

    let value = call();

    unsafe {
        fflush(ptr::null_mut());
    }
    assert_eq!(unsafe { dup2(saved_stdout, STDOUT_FILENO) }, STDOUT_FILENO);
    assert_eq!(unsafe { dup2(saved_stderr, STDERR_FILENO) }, STDERR_FILENO);
    assert_eq!(unsafe { close(saved_stdout) }, 0);
    assert_eq!(unsafe { close(saved_stderr) }, 0);

    Captured {
        value,
        stdout: read_all(stdout_read),
        stderr: read_all(stderr_read),
    }
}

fn effective_op() -> &'static str {
    if cfg!(feature = "mul") {
        "mul"
    } else if cfg!(feature = "sub") {
        "sub"
    } else {
        "add"
    }
}

fn effective_repeat() -> c_int {
    for repeat in (0..=7).rev() {
        let enabled = match repeat {
            0 => cfg!(feature = "0"),
            1 => cfg!(feature = "1"),
            2 => cfg!(feature = "2"),
            3 => cfg!(feature = "3"),
            4 => cfg!(feature = "4"),
            5 => cfg!(feature = "5"),
            6 => cfg!(feature = "6"),
            7 => cfg!(feature = "7"),
            _ => unreachable!(),
        };
        if enabled {
            return repeat;
        }
    }
    5
}

fn c_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
        "../c_src/build/libmd_driver_{}_{}.so",
        effective_op(),
        effective_repeat()
    ))
}

fn rust_library_path() -> PathBuf {
    env::var_os("MD_RUST_SO").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("target/release/libmd_driver.so"),
        PathBuf::from,
    )
}

fn assert_same<T: std::fmt::Debug + PartialEq>(
    label: &str,
    c_result: Captured<T>,
    rust_result: Captured<T>,
) {
    assert_eq!(c_result, rust_result, "{label}");
}

struct FixedRng(u64);

impl FixedRng {
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

    fn range(&mut self, minimum: c_int, maximum: c_int) -> c_int {
        let width = (i64::from(maximum) - i64::from(minimum) + 1) as u32;
        minimum + (self.next_u32() % width) as c_int
    }

    fn operands(&mut self, operation: &str) -> (c_int, c_int) {
        let bound = if operation == "mul" { 1_000 } else { 1_000_000 };
        (self.range(-bound, bound), self.range(-bound, bound))
    }
}

fn call_binary_batch(
    api: &Api,
    symbol: &'static [u8],
    cases: &[(c_int, c_int)],
) -> Captured<Vec<c_int>> {
    let function = unsafe { api.symbol::<OpFn>(symbol) };
    capture(|| {
        cases
            .iter()
            .map(|&(a, b)| unsafe { function(a, b) })
            .collect()
    })
}

fn call_main_batch(
    api: &Api,
    cases: &[(Vec<CString>, c_int)],
) -> Captured<Vec<c_int>> {
    let function = unsafe { api.symbol::<MainFn>(b"main\0") };
    capture(|| {
        cases
            .iter()
            .map(|(arguments, argc)| {
                let mut argv: Vec<*mut c_char> = arguments
                    .iter()
                    .map(|argument| argument.as_ptr().cast_mut())
                    .collect();
                argv.push(ptr::null_mut());
                unsafe { function(*argc, argv.as_mut_ptr()) }
            })
            .collect()
    })
}

fn null_argv_status(api: &Api) -> c_int {
    let function = unsafe { api.symbol::<MainFn>(b"main\0") };
    let pid = unsafe { fork() };
    assert!(pid >= 0, "fork failed: {}", std::io::Error::last_os_error());
    if pid == 0 {
        unsafe {
            function(0, ptr::null_mut());
            _exit(0);
        }
    }

    let mut status = 0;
    assert_eq!(unsafe { waitpid(pid, &mut status, 0) }, pid);
    status
}

fn terminating_signal(status: c_int) -> Option<c_int> {
    let signal = status & 0x7f;
    (signal != 0 && signal != 0x7f).then_some(signal)
}

#[test]
fn differential_surface() {
    let c_path = c_library_path();
    let rust_path = rust_library_path();
    assert!(c_path.is_file(), "missing {}", c_path.display());
    assert!(rust_path.is_file(), "missing {}", rust_path.display());

    let c_api = unsafe { Api::load(&c_path) };
    let rust_api = unsafe { Api::load(&rust_path) };
    let mut rng = FixedRng::new(0x5eed_d1ff_2026_0904);

    for (symbol, operation) in [
        (&b"op_add\0"[..], "add"),
        (&b"op_sub\0"[..], "sub"),
        (&b"op_mul\0"[..], "mul"),
    ] {
        let cases: Vec<_> = (0..RANDOM_CASES)
            .map(|_| rng.operands(operation))
            .collect();
        assert_same(
            &format!("{operation} direct randomized batch"),
            call_binary_batch(&c_api, symbol, &cases),
            call_binary_batch(&rust_api, symbol, &cases),
        );
    }

    let helper_cases: Vec<_> = (0..RANDOM_CASES)
        .map(|_| rng.operands(effective_op()))
        .collect();
    assert_same(
        "helper_ptr randomized batch",
        call_binary_batch(&c_api, b"helper_ptr\0", &helper_cases),
        call_binary_batch(&rust_api, b"helper_ptr\0", &helper_cases),
    );

    let c_global_op = unsafe { c_api.global_op() };
    let rust_global_op = unsafe { rust_api.global_op() };
    let global_cases: Vec<_> = (0..RANDOM_CASES)
        .map(|_| rng.operands(effective_op()))
        .collect();
    assert_same(
        "G_OP randomized batch",
        capture(|| {
            global_cases
                .iter()
                .map(|&(a, b)| unsafe { c_global_op(a, b) })
                .collect::<Vec<_>>()
        }),
        capture(|| {
            global_cases
                .iter()
                .map(|&(a, b)| unsafe { rust_global_op(a, b) })
                .collect::<Vec<_>>()
        }),
    );
    assert_eq!(
        unsafe { c_api.global_name() },
        unsafe { rust_api.global_name() },
        "G_OP_NAME"
    );

    let helper_call_cases: Vec<_> = (0..RANDOM_CASES)
        .map(|_| rng.operands(effective_op()))
        .collect();
    assert_same(
        "helper_call randomized batch",
        call_binary_batch(&c_api, b"helper_call\0", &helper_call_cases),
        call_binary_batch(&rust_api, b"helper_call\0", &helper_call_cases),
    );

    let c_generated = unsafe { c_api.symbol::<UnaryFn>(b"use_generated\0") };
    let rust_generated = unsafe { rust_api.symbol::<UnaryFn>(b"use_generated\0") };
    let mut generated_cases = Vec::new();
    for n in 0..=6 {
        for _ in 0..16 {
            generated_cases.push(n);
        }
    }
    for case in 0..RANDOM_CASES {
        let n = if case % 2 == 0 {
            rng.range(c_int::MIN, -1)
        } else {
            rng.range(7, c_int::MAX)
        };
        generated_cases.push(n);
    }
    assert_same(
        "use_generated switch/default batch",
        capture(|| {
            generated_cases
                .iter()
                .map(|&n| unsafe { c_generated(n) })
                .collect::<Vec<_>>()
        }),
        capture(|| {
            generated_cases
                .iter()
                .map(|&n| unsafe { rust_generated(n) })
                .collect::<Vec<_>>()
        }),
    );

    let mut main_cases = Vec::new();
    for case in 0..32 {
        let (a, b) = rng.operands(effective_op());
        let mut arguments = vec![
            CString::new("driver").unwrap(),
            CString::new(a.to_string()).unwrap(),
            CString::new(b.to_string()).unwrap(),
        ];
        if case % 2 == 1 {
            arguments.push(CString::new("ignored").unwrap());
        }
        let argc = arguments.len() as c_int;
        main_cases.push((arguments, argc));
    }

    main_cases.push((
        vec![
            CString::new("driver").unwrap(),
            CString::new("17").unwrap(),
            CString::new("-9").unwrap(),
        ],
        c_int::MAX,
    ));
    assert_same(
        "main valid/oversized argc batch",
        call_main_batch(&c_api, &main_cases),
        call_main_batch(&rust_api, &main_cases),
    );

    let mut error_cases = Vec::new();
    for argc in [c_int::MIN, -1, 0, 1, 2] {
        error_cases.push((vec![CString::new("driver").unwrap()], argc));
    }
    let c_errors = call_main_batch(&c_api, &error_cases);
    let rust_errors = call_main_batch(&rust_api, &error_cases);
    assert!(c_errors.value.iter().all(|&result| result == 2));
    assert!(rust_errors.value.iter().all(|&result| result == 2));
    assert_same("main argc < 3 batch", c_errors, rust_errors);

    if env::var_os("MD_SKIP_NULL").is_none() {
        let c_status = null_argv_status(&c_api);
        let rust_status = null_argv_status(&rust_api);
        assert_eq!(
            terminating_signal(c_status),
            terminating_signal(rust_status),
            "null argv signal mismatch: C={c_status:#x}, Rust={rust_status:#x}"
        );
        assert!(
            terminating_signal(c_status).is_some() && terminating_signal(rust_status).is_some(),
            "null argv unexpectedly returned: C={c_status:#x}, Rust={rust_status:#x}"
        );
    }
}
