use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void, CString};
use std::fs::File;
use std::io::Read;
use std::os::fd::FromRawFd;
use std::path::{Path, PathBuf};
use std::ptr;

type VoidFn = unsafe extern "C" fn();
type IntFn = unsafe extern "C" fn(c_int);
type StringFn = unsafe extern "C" fn(*const c_char);
type MainFn = unsafe extern "C" fn() -> c_int;

extern "C" {
    static mut stdin: *mut c_void;
    static mut stdout: *mut c_void;

    fn clearerr(stream: *mut c_void);
    fn close(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn fork() -> c_int;
    fn pipe(pipe_fds: *mut c_int) -> c_int;
    fn setvbuf(stream: *mut c_void, buffer: *mut c_char, mode: c_int, size: usize) -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn write(fd: c_int, buffer: *const c_void, count: usize) -> isize;
    fn _exit(status: c_int) -> !;
}

const STDIN_FILENO: c_int = 0;
const STDOUT_FILENO: c_int = 1;
const IONBF: c_int = 2;

#[derive(Debug, Eq, PartialEq)]
struct Outcome {
    output: Vec<u8>,
    return_value: Option<c_int>,
    wait_status: c_int,
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
}

fn make_pipe() -> [c_int; 2] {
    let mut fds = [-1; 2];
    let result = unsafe { pipe(fds.as_mut_ptr()) };
    assert_eq!(
        result,
        0,
        "pipe failed: {}",
        std::io::Error::last_os_error()
    );
    fds
}

fn close_fd(fd: c_int) {
    if fd >= 0 {
        let result = unsafe { close(fd) };
        assert_eq!(
            result,
            0,
            "close({fd}) failed: {}",
            std::io::Error::last_os_error()
        );
    }
}

fn write_all_fd(fd: c_int, mut bytes: &[u8]) {
    while !bytes.is_empty() {
        let written = unsafe { write(fd, bytes.as_ptr().cast(), bytes.len()) };
        assert!(
            written > 0,
            "write failed: {}",
            std::io::Error::last_os_error()
        );
        bytes = &bytes[written as usize..];
    }
}

fn read_all_fd(fd: c_int) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut file = unsafe { File::from_raw_fd(fd) };
    file.read_to_end(&mut bytes).expect("read from child pipe");
    bytes
}

fn capture<F>(input: Option<&[u8]>, call: F) -> Outcome
where
    F: FnOnce() -> c_int,
{
    let output_pipe = make_pipe();
    let result_pipe = make_pipe();
    let input_pipe = input.map(|bytes| {
        let fds = make_pipe();
        write_all_fd(fds[1], bytes);
        close_fd(fds[1]);
        fds
    });

    unsafe {
        fflush(ptr::null_mut());
    }

    let pid = unsafe { fork() };
    assert!(pid >= 0, "fork failed: {}", std::io::Error::last_os_error());

    if pid == 0 {
        unsafe {
            close(output_pipe[0]);
            close(result_pipe[0]);

            if let Some(fds) = input_pipe {
                if dup2(fds[0], STDIN_FILENO) < 0 {
                    _exit(120);
                }
                close(fds[0]);
                clearerr(stdin);
            }

            if dup2(output_pipe[1], STDOUT_FILENO) < 0 {
                _exit(121);
            }
            close(output_pipe[1]);
            setvbuf(stdout, ptr::null_mut(), IONBF, 0);

            let return_value = call();
            fflush(ptr::null_mut());
            write_all_fd(result_pipe[1], &return_value.to_ne_bytes());
            close(result_pipe[1]);
            _exit(0);
        }
    }

    close_fd(output_pipe[1]);
    close_fd(result_pipe[1]);
    if let Some(fds) = input_pipe {
        close_fd(fds[0]);
    }

    let output = read_all_fd(output_pipe[0]);
    let result = read_all_fd(result_pipe[0]);
    let mut wait_status = 0;
    let waited = unsafe { waitpid(pid, &mut wait_status, 0) };
    assert_eq!(waited, pid, "waitpid failed");

    let return_value = if result.len() == std::mem::size_of::<c_int>() {
        Some(c_int::from_ne_bytes(result.try_into().unwrap()))
    } else {
        assert!(
            result.is_empty(),
            "child returned a partial result: {result:?}"
        );
        None
    };

    Outcome {
        output,
        return_value,
        wait_status,
    }
}

unsafe fn call_void(library: &Library, symbol: &[u8]) -> Outcome {
    let function: Symbol<VoidFn> = library.get(symbol).expect("load void symbol");
    capture(None, || {
        function();
        0
    })
}

unsafe fn call_int(library: &Library, value: c_int) -> Outcome {
    let function: Symbol<IntFn> = library.get(b"printIntLine").expect("load printIntLine");
    capture(None, || {
        function(value);
        0
    })
}

unsafe fn call_string(library: &Library, value: *const c_char) -> Outcome {
    let function: Symbol<StringFn> = library.get(b"printLine").expect("load printLine");
    capture(None, || {
        function(value);
        0
    })
}

unsafe fn call_main(library: &Library, input: &[u8]) -> Outcome {
    let function: Symbol<MainFn> = library.get(b"main").expect("load main");
    capture(Some(input), || function())
}

fn assert_same(context: &str, c: Outcome, rust: Outcome) {
    assert_eq!(c, rust, "C/Rust mismatch for {context}");
}

fn rust_library_path() -> PathBuf {
    let executable = std::env::current_exe().expect("current test executable");
    executable
        .parent()
        .and_then(Path::parent)
        .expect("target profile directory")
        .join("libdriver.so")
}

fn verify_symbols(c_library: &Library, rust_library: &Library) {
    for symbol in [
        b"bad".as_slice(),
        b"good".as_slice(),
        b"main".as_slice(),
        b"printIntLine".as_slice(),
        b"printLine".as_slice(),
    ] {
        unsafe {
            c_library
                .get::<*const c_void>(symbol)
                .unwrap_or_else(|error| panic!("C symbol {symbol:?}: {error}"));
            rust_library
                .get::<*const c_void>(symbol)
                .unwrap_or_else(|error| panic!("Rust symbol {symbol:?}: {error}"));
        }
    }
}

fn valid_path_tests(c_library: &Library, rust_library: &Library) {
    unsafe {
        // CONFIGS.md row 1: empty non-null string.
        let empty = CString::new(Vec::new()).unwrap();
        assert_same(
            "printLine(empty)",
            call_string(c_library, empty.as_ptr()),
            call_string(rust_library, empty.as_ptr()),
        );

        // CONFIGS.md row 2: randomized non-empty strings and an oversized one.
        let mut rng = Rng::new(0x62d7_4e3a_c901_5bf1);
        for case in 0..192 {
            let length = 1 + rng.next_u32() as usize % 256;
            let bytes: Vec<u8> = (0..length)
                .map(|_| 1 + (rng.next_u32() % 127) as u8)
                .collect();
            let value = CString::new(bytes).unwrap();
            assert_same(
                &format!("printLine randomized case {case}"),
                call_string(c_library, value.as_ptr()),
                call_string(rust_library, value.as_ptr()),
            );
        }
        let oversized = CString::new(vec![b'Z'; 65_537]).unwrap();
        assert_same(
            "printLine oversized string",
            call_string(c_library, oversized.as_ptr()),
            call_string(rust_library, oversized.as_ptr()),
        );

        // CONFIGS.md row 3: randomized int values and all generic boundaries.
        let boundaries = [
            c_int::MIN,
            c_int::MIN + 1,
            -1,
            0,
            1,
            c_int::MAX - 1,
            c_int::MAX,
        ];
        for value in boundaries {
            assert_same(
                &format!("printIntLine boundary {value}"),
                call_int(c_library, value),
                call_int(rust_library, value),
            );
        }
        for case in 0..384 {
            let value = rng.next_i32();
            assert_same(
                &format!("printIntLine randomized case {case}: {value}"),
                call_int(c_library, value),
                call_int(rust_library, value),
            );
        }

        // CONFIGS.md rows 4-5: fixed low-level operations.
        assert_same(
            "bad",
            call_void(c_library, b"bad"),
            call_void(rust_library, b"bad"),
        );
        assert_same(
            "good",
            call_void(c_library, b"good"),
            call_void(rust_library, b"good"),
        );

        // CONFIGS.md row 6: successful conversion to zero.
        let zero_inputs: [&[u8]; 8] = [
            b"0\n",
            b"+0\n",
            b"-0\n",
            b"000000\n",
            b" 0 ",
            b"\t+000\n",
            b"\n-000 ",
            b"0 trailing",
        ];
        for (case, input) in zero_inputs.into_iter().enumerate() {
            assert_same(
                &format!("main zero case {case}"),
                call_main(c_library, input),
                call_main(rust_library, input),
            );
        }

        // CONFIGS.md row 7: successful conversion to randomized nonzero values.
        for case in 0..128 {
            let mut value = rng.next_i32();
            if value == 0 {
                value = 1;
            }
            let input = format!(" \t{value}\n");
            assert_same(
                &format!("main nonzero case {case}: {value}"),
                call_main(c_library, input.as_bytes()),
                call_main(rust_library, input.as_bytes()),
            );
        }

        // CONFIGS.md row 8: failed conversion leaves x at zero.
        for case in 0..64 {
            let suffix = rng.next_u32();
            let input = format!("not-an-int-{suffix}\n");
            assert_same(
                &format!("main failed conversion case {case}"),
                call_main(c_library, input.as_bytes()),
                call_main(rust_library, input.as_bytes()),
            );
        }

        // CONFIGS.md row 9: EOF leaves x at zero.
        assert_same(
            "main EOF",
            call_main(c_library, b""),
            call_main(rust_library, b""),
        );
    }
}

fn error_path_tests(c_library: &Library, rust_library: &Library) {
    unsafe {
        // ERRORS.md row 1 and the only pointer-taking API's null boundary.
        assert_same(
            "printLine(NULL)",
            call_string(c_library, ptr::null()),
            call_string(rust_library, ptr::null()),
        );
    }
}

fn main() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let c_path = manifest.join("c_src/build/libdriver_c.so");
    let rust_path = rust_library_path();
    assert!(
        c_path.is_file(),
        "missing C shared library: {}",
        c_path.display()
    );
    assert!(
        rust_path.is_file(),
        "missing Rust shared library: {}",
        rust_path.display()
    );

    let c_library = unsafe { Library::new(&c_path).expect("load C shared library") };
    let rust_library = unsafe { Library::new(&rust_path).expect("load Rust shared library") };

    verify_symbols(&c_library, &rust_library);
    valid_path_tests(&c_library, &rust_library);
    error_path_tests(&c_library, &rust_library);
}
