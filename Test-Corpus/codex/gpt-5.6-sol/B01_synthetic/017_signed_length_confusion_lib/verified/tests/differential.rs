use libloading::Library;
use std::ffi::{CString, c_char, c_int, c_void};
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::Mutex;

type Driver = unsafe extern "C" fn(c_int);
type PrintLine = unsafe extern "C" fn(*const c_char);

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(pipe_fds: *mut c_int) -> c_int;
    fn read(fd: c_int, buffer: *mut c_void, count: usize) -> isize;
}

const STDOUT_FILENO: c_int = 1;
const CASES_PER_ROW: usize = 128;
static STDOUT_LOCK: Mutex<()> = Mutex::new(());

struct Implementations {
    c: Library,
    rust: Library,
}

impl Implementations {
    unsafe fn load() -> Self {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_path = manifest.join("c_src/build/libdriver.so");
        let rust_path = rust_library_path(&manifest);

        assert!(
            c_path.is_file(),
            "C shared library is missing at {}; build it with CMake first",
            c_path.display()
        );
        assert!(
            rust_path.is_file(),
            "Rust shared library is missing at {}",
            rust_path.display()
        );

        Self {
            c: unsafe { Library::new(&c_path) }
                .unwrap_or_else(|error| panic!("failed to load {}: {error}", c_path.display())),
            rust: unsafe { Library::new(&rust_path) }
                .unwrap_or_else(|error| panic!("failed to load {}: {error}", rust_path.display())),
        }
    }

    unsafe fn compare_driver(&self, data: c_int) -> (Vec<u8>, Vec<u8>) {
        let c_driver: Driver = *unsafe { self.c.get(b"driver\0") }.expect("C driver symbol");
        let rust_driver: Driver =
            *unsafe { self.rust.get(b"driver\0") }.expect("Rust driver symbol");

        let c_output = capture_stdout(|| unsafe { c_driver(data) });
        let rust_output = capture_stdout(|| unsafe { rust_driver(data) });
        assert_eq!(rust_output, c_output, "driver({data})");
        (c_output, rust_output)
    }

    unsafe fn compare_print_line(&self, line: *const c_char, description: &str) -> Vec<u8> {
        let c_print_line: PrintLine =
            *unsafe { self.c.get(b"printLine\0") }.expect("C printLine symbol");
        let rust_print_line: PrintLine =
            *unsafe { self.rust.get(b"printLine\0") }.expect("Rust printLine symbol");

        let c_output = capture_stdout(|| unsafe { c_print_line(line) });
        let rust_output = capture_stdout(|| unsafe { rust_print_line(line) });
        assert_eq!(rust_output, c_output, "printLine({description})");
        c_output
    }
}

fn rust_library_path(manifest: &Path) -> PathBuf {
    let test_executable = std::env::current_exe().expect("current test executable");
    let deps = test_executable
        .parent()
        .expect("test executable must have a parent directory");
    let profile = deps
        .parent()
        .expect("Cargo deps directory must have a profile parent");
    let candidates = [
        profile.join("libdriver.so"),
        deps.join("libdriver.so"),
        manifest.join("target/release/libdriver.so"),
    ];

    candidates
        .into_iter()
        .find(|path| path.is_file())
        .unwrap_or_else(|| profile.join("libdriver.so"))
}

fn capture_stdout(call: impl FnOnce()) -> Vec<u8> {
    let _guard = STDOUT_LOCK.lock().expect("stdout capture lock");
    let mut pipe_fds = [-1; 2];

    unsafe {
        assert_eq!(fflush(ptr::null_mut()), 0, "flush stdout before capture");
        assert_eq!(pipe(pipe_fds.as_mut_ptr()), 0, "create stdout pipe");

        let saved_stdout = dup(STDOUT_FILENO);
        assert!(saved_stdout >= 0, "duplicate stdout");
        assert_eq!(
            dup2(pipe_fds[1], STDOUT_FILENO),
            STDOUT_FILENO,
            "redirect stdout"
        );
        assert_eq!(close(pipe_fds[1]), 0, "close original pipe writer");

        call();

        assert_eq!(fflush(ptr::null_mut()), 0, "flush captured stdout");
        assert_eq!(
            dup2(saved_stdout, STDOUT_FILENO),
            STDOUT_FILENO,
            "restore stdout"
        );
        assert_eq!(close(saved_stdout), 0, "close saved stdout");

        let mut output = Vec::new();
        let mut buffer = [0_u8; 1024];
        loop {
            let bytes_read = read(
                pipe_fds[0],
                buffer.as_mut_ptr().cast::<c_void>(),
                buffer.len(),
            );
            assert!(bytes_read >= 0, "read captured stdout");
            if bytes_read == 0 {
                break;
            }
            output.extend_from_slice(&buffer[..bytes_read as usize]);
        }
        assert_eq!(close(pipe_fds[0]), 0, "close pipe reader");
        output
    }
}

struct XorShift64(u64);

impl XorShift64 {
    fn new(seed: u64) -> Self {
        assert_ne!(seed, 0);
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

    fn range(&mut self, start: usize, end_inclusive: usize) -> usize {
        start + self.next_u64() as usize % (end_inclusive - start + 1)
    }

    fn printable_bytes(&mut self, length: usize) -> Vec<u8> {
        (0..length)
            .map(|_| self.range(b' ' as usize, b'~' as usize) as u8)
            .collect()
    }
}

#[test]
fn phase_b_all_runtime_configurations_match() {
    let implementations = unsafe { Implementations::load() };
    let mut random = XorShift64::new(0x4d59_5df4_d0f3_3173);

    // C1: non-null empty C string.
    for _ in 0..CASES_PER_ROW {
        let line = CString::new(Vec::<u8>::new()).unwrap();
        let output =
            unsafe { implementations.compare_print_line(line.as_ptr(), "C1 empty string") };
        assert_eq!(output, b"\n");
    }

    // C2: one-byte C strings.
    for _ in 0..CASES_PER_ROW {
        let bytes = random.printable_bytes(1);
        let line = CString::new(bytes.clone()).unwrap();
        let output = unsafe { implementations.compare_print_line(line.as_ptr(), "C2 one byte") };
        assert_eq!(output, [bytes.as_slice(), b"\n"].concat());
    }

    // C3: many-byte C strings.
    for _ in 0..CASES_PER_ROW {
        let length = random.range(2, 256);
        let bytes = random.printable_bytes(length);
        let line = CString::new(bytes.clone()).unwrap();
        let output = unsafe { implementations.compare_print_line(line.as_ptr(), "C3 many bytes") };
        assert_eq!(output, [bytes.as_slice(), b"\n"].concat());
    }

    // C4: zero-length prefix.
    for _ in 0..CASES_PER_ROW {
        let (output, _) = unsafe { implementations.compare_driver(0) };
        assert_eq!(output, b"\n");
    }

    // C5: one-byte prefix.
    for _ in 0..CASES_PER_ROW {
        let (output, _) = unsafe { implementations.compare_driver(1) };
        assert_eq!(output, b"A\n");
    }

    // C6: interior prefix lengths.
    for _ in 0..CASES_PER_ROW {
        let data = random.range(2, 98) as c_int;
        let (output, _) = unsafe { implementations.compare_driver(data) };
        assert_eq!(output, [vec![b'A'; data as usize], b"\n".to_vec()].concat());
    }

    // C7: maximum source payload.
    for _ in 0..CASES_PER_ROW {
        let (output, _) = unsafe { implementations.compare_driver(99) };
        assert_eq!(output, [vec![b'A'; 99], b"\n".to_vec()].concat());
    }

    // C8: the copy branch is skipped at and above 100.
    let mut oversized = vec![100, 101, c_int::MAX];
    oversized.extend(
        (0..CASES_PER_ROW - oversized.len())
            .map(|_| 100 + (random.next_u64() % (c_int::MAX as u64 - 99)) as c_int),
    );
    for data in oversized {
        let (output, _) = unsafe { implementations.compare_driver(data) };
        assert_eq!(output, b"\n");
    }
}

#[test]
fn phase_c_all_explicit_rejections_match() {
    let implementations = unsafe { Implementations::load() };

    // E1 and the generic null-pointer boundary.
    let output = unsafe { implementations.compare_print_line(ptr::null(), "E1 null pointer") };
    assert!(output.is_empty());

    // E2, including the boundary and a value well beyond it.
    for data in [100, 101, c_int::MAX] {
        let (output, _) = unsafe { implementations.compare_driver(data) };
        assert_eq!(output, b"\n");
    }
}
