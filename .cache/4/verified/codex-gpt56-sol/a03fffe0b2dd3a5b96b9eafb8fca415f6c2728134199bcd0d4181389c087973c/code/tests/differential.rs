use libloading::Library;
use std::ffi::{c_int, c_void};
use std::fs::File;
use std::io::{Read, Write};
use std::os::fd::{FromRawFd, RawFd};
use std::path::{Path, PathBuf};
use std::ptr;

type DriverFn = unsafe extern "C" fn(c_int, c_int);
type MainFn = unsafe extern "C" fn() -> c_int;

unsafe extern "C" {
    fn pipe(fds: *mut c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn clearerr(stream: *mut c_void);

    #[link_name = "stdin"]
    static mut C_STDIN: *mut c_void;
}

struct Api {
    _library: Library,
    driver: DriverFn,
    main: MainFn,
}

impl Api {
    fn load(path: &Path) -> Self {
        // SAFETY: The libraries remain owned by Api for the lifetime of the
        // copied function pointers, and both signatures come from the C source.
        unsafe {
            let library = Library::new(path)
                .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
            let driver = *library
                .get::<DriverFn>(b"driver\0")
                .unwrap_or_else(|error| panic!("missing driver in {}: {error}", path.display()));
            let main = *library
                .get::<MainFn>(b"main\0")
                .unwrap_or_else(|error| panic!("missing main in {}: {error}", path.display()));
            Self {
                _library: library,
                driver,
                main,
            }
        }
    }
}

struct XorShift64(u64);

impl XorShift64 {
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

fn pipe_pair() -> (RawFd, RawFd) {
    let mut fds = [-1, -1];
    // SAFETY: fds points to storage for the two descriptors written by pipe.
    let result = unsafe { pipe(fds.as_mut_ptr()) };
    assert_eq!(
        result,
        0,
        "pipe failed: {}",
        std::io::Error::last_os_error()
    );
    (fds[0], fds[1])
}

fn checked_dup(fd: RawFd) -> RawFd {
    // SAFETY: dup accepts any integer descriptor and reports invalid ones.
    let copy = unsafe { dup(fd) };
    assert!(
        copy >= 0,
        "dup({fd}) failed: {}",
        std::io::Error::last_os_error()
    );
    copy
}

fn checked_dup2(old_fd: RawFd, new_fd: RawFd) {
    // SAFETY: Both descriptors are owned or are the standard process streams.
    let result = unsafe { dup2(old_fd, new_fd) };
    assert!(
        result >= 0,
        "dup2({old_fd}, {new_fd}) failed: {}",
        std::io::Error::last_os_error()
    );
}

fn checked_close(fd: RawFd) {
    // SAFETY: Each descriptor passed here is live and owned by this function.
    let result = unsafe { close(fd) };
    assert_eq!(
        result,
        0,
        "close({fd}) failed: {}",
        std::io::Error::last_os_error()
    );
}

fn capture_stdio<T>(input: &[u8], invoke: impl FnOnce() -> T) -> (T, Vec<u8>) {
    const STDIN_FILENO: RawFd = 0;
    const STDOUT_FILENO: RawFd = 1;

    let (input_read, input_write) = pipe_pair();
    let (output_read, output_write) = pipe_pair();

    // SAFETY: input_write is uniquely owned and transferred to File.
    let mut input_file = unsafe { File::from_raw_fd(input_write) };
    input_file.write_all(input).expect("write test stdin");
    drop(input_file);

    // Flush output left by the test harness before replacing stdout.
    // SAFETY: A null stream asks libc to flush all output streams.
    assert_eq!(unsafe { fflush(ptr::null_mut()) }, 0);

    let saved_stdin = checked_dup(STDIN_FILENO);
    let saved_stdout = checked_dup(STDOUT_FILENO);
    checked_dup2(input_read, STDIN_FILENO);
    checked_dup2(output_write, STDOUT_FILENO);
    checked_close(input_read);
    checked_close(output_write);

    // scanf leaves EOF/error state on FILE even after fd 0 is replaced.
    // SAFETY: C_STDIN is libc's live stdin FILE pointer.
    unsafe { clearerr(C_STDIN) };
    let result = invoke();
    // SAFETY: A null stream asks libc to flush all output streams.
    assert_eq!(unsafe { fflush(ptr::null_mut()) }, 0);

    checked_dup2(saved_stdin, STDIN_FILENO);
    checked_dup2(saved_stdout, STDOUT_FILENO);
    checked_close(saved_stdin);
    checked_close(saved_stdout);
    // SAFETY: Clear EOF from this invocation before the next pipe is installed.
    unsafe { clearerr(C_STDIN) };

    // SAFETY: output_read is uniquely owned and transferred to File.
    let mut output_file = unsafe { File::from_raw_fd(output_read) };
    let mut output = Vec::new();
    output_file
        .read_to_end(&mut output)
        .expect("read captured stdout");
    (result, output)
}

fn call_driver(api: &Api, x: i32, y: i32) -> Vec<u8> {
    capture_stdio(&[], || {
        // SAFETY: The symbol has the verified driver(int, int) signature.
        unsafe { (api.driver)(x, y) }
    })
    .1
}

fn call_main(api: &Api, input: &[u8]) -> (i32, Vec<u8>) {
    capture_stdio(input, || {
        // SAFETY: The symbol has the verified main(void) signature.
        unsafe { (api.main)() }
    })
}

fn shared_object_paths() -> (PathBuf, PathBuf) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_library = root.join("c_src/build/libdriver.so");
    let test_executable = std::env::current_exe().expect("current test executable");
    let profile_dir = test_executable
        .parent()
        .and_then(Path::parent)
        .expect("target profile directory");
    let rust_library = profile_dir.join("libdriver.so");
    assert!(
        c_library.is_file(),
        "missing C shared library: {}",
        c_library.display()
    );
    assert!(
        rust_library.is_file(),
        "missing Rust shared library: {}",
        rust_library.display()
    );
    (c_library, rust_library)
}

#[test]
fn every_configuration_matches_through_dynamic_ffi() {
    let (c_path, rust_path) = shared_object_paths();
    let c = Api::load(&c_path);
    let rust = Api::load(&rust_path);
    let mut random = XorShift64::new(0x8f3d_9a72_c641_05be);

    // CONFIGS.md row 1: direct calls over boundary and randomized C ints.
    let mut pairs = vec![
        (i32::MIN, i32::MIN),
        (i32::MIN, i32::MAX),
        (i32::MAX, i32::MIN),
        (i32::MAX, i32::MAX),
        (-1, 0),
        (0, -1),
        (0, 0),
        (1, 1),
    ];
    pairs.extend((0..512).map(|_| (random.next_i32(), random.next_i32())));
    for (x, y) in pairs {
        let c_output = call_driver(&c, x, y);
        let rust_output = call_driver(&rust, x, y);
        let expected = format!("{}\n", x | !y).into_bytes();
        assert_eq!(c_output, expected, "C driver result for ({x}, {y})");
        assert_eq!(rust_output, c_output, "Rust driver result for ({x}, {y})");
    }

    // CONFIGS.md row 2: both decimal conversions succeed.
    for _ in 0..256 {
        let x = random.next_i32();
        let y = random.next_i32();
        let input = format!(" \n{x}\t{y}\n").into_bytes();
        let c_result = call_main(&c, &input);
        let rust_result = call_main(&rust, &input);
        assert_eq!(c_result.0, 0, "C main return for two valid ints");
        assert_eq!(rust_result, c_result, "main with input {input:?}");
    }

    // CONFIGS.md row 3: the first conversion fails, including immediate EOF.
    let mut invalid_first = vec![Vec::new(), b"x".to_vec(), b"? 12".to_vec()];
    invalid_first.extend((0..128).map(|_| {
        let suffix = random.next_u32();
        format!("invalid_{suffix:08x}").into_bytes()
    }));
    for input in invalid_first {
        let c_result = call_main(&c, &input);
        let rust_result = call_main(&rust, &input);
        assert_eq!(c_result, (0, b"-1\n".to_vec()), "C failed first scanf");
        assert_eq!(rust_result, c_result, "main with input {input:?}");
    }

    // CONFIGS.md row 4: x converts and the second conversion fails or sees EOF.
    for index in 0..256 {
        let x = random.next_i32();
        let input = if index % 2 == 0 {
            x.to_string().into_bytes()
        } else {
            format!("{x} invalid_{:08x}", random.next_u32()).into_bytes()
        };
        let c_result = call_main(&c, &input);
        let rust_result = call_main(&rust, &input);
        assert_eq!(c_result, (0, b"-1\n".to_vec()), "C failed second scanf");
        assert_eq!(rust_result, c_result, "main with input {input:?}");
    }
}
