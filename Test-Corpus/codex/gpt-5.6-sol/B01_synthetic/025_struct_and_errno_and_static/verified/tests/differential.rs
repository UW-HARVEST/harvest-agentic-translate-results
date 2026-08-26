use libloading::Library;
use std::ffi::{c_int, c_void};
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

type MainFn = unsafe extern "C" fn() -> c_int;
type RunFn = unsafe extern "C" fn(c_int);

static TEST_LOCK: Mutex<()> = Mutex::new(());
static TEMP_ID: AtomicU64 = AtomicU64::new(0);

unsafe extern "C" {
    static mut stdin: *mut c_void;

    fn __fpurge(stream: *mut c_void);
    fn clearerr(stream: *mut c_void);
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

struct Api {
    _library: Library,
    main: MainFn,
    run: RunFn,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let main = unsafe {
            *library
                .get::<MainFn>(b"main\0")
                .expect("missing main export")
        };
        let run = unsafe { *library.get::<RunFn>(b"run\0").expect("missing run export") };
        Self {
            _library: library,
            main,
            run,
        }
    }
}

struct Harness {
    c: Api,
    rust: Api,
}

impl Harness {
    unsafe fn load() -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let target = std::env::var_os("CARGO_TARGET_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| root.join("target"));
        Self {
            c: unsafe { Api::load(&root.join("c_src/build/libdriver_c.so")) },
            rust: unsafe { Api::load(&target.join("debug/libdriver.so")) },
        }
    }

    fn compare_run(&self, value: i32, context: &str) {
        let (_, c_output) = capture_stdio(&[], || unsafe { (self.c.run)(value) });
        let (_, rust_output) = capture_stdio(&[], || unsafe { (self.rust.run)(value) });
        assert_eq!(rust_output, c_output, "{context}; run({value})");
    }

    fn compare_main(&self, input: &[u8], context: &str) {
        let (c_result, c_output) = capture_stdio(input, || unsafe { (self.c.main)() });
        let (rust_result, rust_output) = capture_stdio(input, || unsafe { (self.rust.main)() });
        assert_eq!(rust_result, c_result, "{context}; input={input:?}");
        assert_eq!(rust_output, c_output, "{context}; input={input:?}");
    }
}

struct FdRestore {
    stdin_fd: c_int,
    stdout_fd: c_int,
}

impl Drop for FdRestore {
    fn drop(&mut self) {
        unsafe {
            fflush(ptr::null_mut());
            dup2(self.stdin_fd, 0);
            dup2(self.stdout_fd, 1);
            close(self.stdin_fd);
            close(self.stdout_fd);
            __fpurge(stdin);
            clearerr(stdin);
        }
    }
}

fn temp_file(label: &str) -> (PathBuf, File) {
    let id = TEMP_ID.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "driver-differential-{}-{id}-{label}",
        std::process::id()
    ));
    let file = OpenOptions::new()
        .create_new(true)
        .read(true)
        .write(true)
        .open(&path)
        .unwrap();
    (path, file)
}

fn capture_stdio<T>(input: &[u8], call: impl FnOnce() -> T) -> (T, Vec<u8>) {
    let (input_path, mut input_file) = temp_file("stdin");
    let (output_path, mut output_file) = temp_file("stdout");
    input_file.write_all(input).unwrap();
    input_file.seek(SeekFrom::Start(0)).unwrap();

    unsafe {
        assert_eq!(fflush(ptr::null_mut()), 0);
        let saved_stdin = dup(0);
        let saved_stdout = dup(1);
        assert!(saved_stdin >= 0 && saved_stdout >= 0);
        assert_eq!(dup2(input_file.as_raw_fd(), 0), 0);
        assert_eq!(dup2(output_file.as_raw_fd(), 1), 1);
        __fpurge(stdin);
        clearerr(stdin);
        let restore = FdRestore {
            stdin_fd: saved_stdin,
            stdout_fd: saved_stdout,
        };

        let result = call();
        assert_eq!(fflush(ptr::null_mut()), 0);
        drop(restore);

        output_file.seek(SeekFrom::Start(0)).unwrap();
        let mut output = Vec::new();
        output_file.read_to_end(&mut output).unwrap();
        drop(input_file);
        drop(output_file);
        std::fs::remove_file(input_path).unwrap();
        std::fs::remove_file(output_path).unwrap();
        (result, output)
    }
}

struct Lcg(u64);

impl Lcg {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u32(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 32) as u32
    }

    fn range(&mut self, upper: u32) -> u32 {
        self.next_u32() % upper
    }
}

#[test]
fn configuration_surface_matches() {
    let _guard = TEST_LOCK.lock().unwrap();
    let harness = unsafe { Harness::load() };
    let mut rng = Lcg::new(0x5eed_c0de_d15c_a11);

    // C1: zero is a singleton input; vary the preceding state.
    for _ in 0..32 {
        harness.compare_run((rng.range(2_000_001) as i32) - 1_000_000, "C1 setup");
        harness.compare_run(0, "C1 zero");
    }

    // C2-C4: sign partitions and both integer boundaries.
    for _ in 0..64 {
        harness.compare_run((rng.range(i32::MAX as u32) + 1) as i32, "C2 positive");
    }
    for _ in 0..64 {
        harness.compare_run(-((rng.range(i32::MAX as u32) + 1) as i32), "C3 negative");
    }
    for index in 0..64 {
        let offset = rng.range(1024) as i32;
        let value = if index % 2 == 0 {
            i32::MIN.wrapping_add(offset)
        } else {
            i32::MAX.wrapping_sub(offset)
        };
        harness.compare_run(value, "C4 integer boundary");
    }

    // C5: a randomized stateful call sequence.
    for _ in 0..128 {
        harness.compare_run(rng.next_u32() as i32, "C5 accumulated state");
    }

    for _ in 0..32 {
        let value = rng.range(1_000_000);
        harness.compare_main(format!("{value}\n").as_bytes(), "C6 digits plus newline");
    }

    let whitespace = [b' ', b'\t', 0x0b, 0x0c, b'\r'];
    for _ in 0..32 {
        let count = rng.range(12) as usize + 1;
        let mut input = Vec::with_capacity(count + 16);
        for _ in 0..count {
            input.push(whitespace[rng.range(whitespace.len() as u32) as usize]);
        }
        input.extend_from_slice(rng.range(1_000_000).to_string().as_bytes());
        input.push(b'\n');
        harness.compare_main(&input, "C7 leading whitespace");
    }

    for _ in 0..32 {
        let value = rng.range(i32::MAX as u32);
        harness.compare_main(format!("+{value}\n").as_bytes(), "C8 plus sign");
    }

    for _ in 0..32 {
        let value = rng.range(i32::MAX as u32) + 1;
        harness.compare_main(format!("-{value}\n").as_bytes(), "C9 minus sign");
    }

    for _ in 0..32 {
        let zeroes = rng.range(80) as usize + 1;
        let input = format!("{}\n", "0".repeat(zeroes));
        harness.compare_main(input.as_bytes(), "C10 leading zeroes");
    }

    for _ in 0..32 {
        let value = rng.range(1_000_000);
        let suffix_len = rng.range(24) as usize + 1;
        let suffix: String = (0..suffix_len)
            .map(|_| (b'a' + rng.range(26) as u8) as char)
            .collect();
        harness.compare_main(
            format!("{value}{suffix}\n").as_bytes(),
            "C11 trailing non-digits",
        );
    }

    for _ in 0..32 {
        let value = rng.range(1_000_000);
        let mut input = value.to_string().into_bytes();
        input.push(0);
        input.extend((0..16).map(|_| b'a' + rng.range(26) as u8));
        input.push(b'\n');
        harness.compare_main(&input, "C12 embedded NUL");
    }

    for _ in 0..32 {
        let value = rng.range(1_000_000);
        harness.compare_main(value.to_string().as_bytes(), "C13 EOF without newline");
    }

    for _ in 0..32 {
        let digits = rng.range(9) as usize + 1;
        let first = (b'1' + rng.range(8) as u8) as char;
        let mut number = String::from(first);
        number.extend((1..digits).map(|_| (b'0' + rng.range(10) as u8) as char));
        let input = format!("{}{}\n", " ".repeat(99 - digits), number);
        harness.compare_main(input.as_bytes(), "C14 fgets payload limit");
    }

    for index in 0..64 {
        let offset = rng.range(1024) as i32;
        let value = if index % 2 == 0 {
            i32::MIN.wrapping_add(offset)
        } else {
            i32::MAX.wrapping_sub(offset)
        };
        harness.compare_main(format!("{value}\n").as_bytes(), "C15 inclusive int bounds");
    }
}

#[test]
fn error_surface_matches() {
    let _guard = TEST_LOCK.lock().unwrap();
    let harness = unsafe { Harness::load() };
    let mut rng = Lcg::new(0xbad5_eed0_0dd5_5eed);

    let fixed_no_conversion: &[&[u8]] = &[b"", b"\n", b" \t\r\n", b"+\n", b"-\n", b"x123\n"];
    for input in fixed_no_conversion {
        harness.compare_main(input, "E1 no conversion");
    }
    for _ in 0..64 {
        let mut input = vec![b' '; rng.range(16) as usize];
        if rng.range(2) == 1 {
            input.push(if rng.range(2) == 0 { b'+' } else { b'-' });
        }
        input.push(b'a' + rng.range(26) as u8);
        input.push(b'\n');
        harness.compare_main(&input, "E1 randomized no conversion");
    }

    harness.compare_main(b"9223372036854775808\n", "E2 LONG_MAX plus one");
    harness.compare_main(b"-9223372036854775809\n", "E2 LONG_MIN minus one");
    for _ in 0..64 {
        let count = rng.range(70) as usize + 30;
        let mut input = Vec::with_capacity(count + 2);
        if rng.range(2) == 1 {
            input.push(b'-');
        }
        input.extend((0..count).map(|_| b'8' + rng.range(2) as u8));
        input.push(b'\n');
        harness.compare_main(&input, "E2 strtol ERANGE");
    }

    harness.compare_main(b"-2147483649\n", "E3 INT_MIN minus one");
    for _ in 0..64 {
        let magnitude = i64::from(i32::MAX) + 2 + i64::from(rng.range(1_000_000));
        harness.compare_main(format!("-{magnitude}\n").as_bytes(), "E3 below INT_MIN");
    }

    harness.compare_main(b"2147483648\n", "E4 INT_MAX plus one");
    for _ in 0..64 {
        let value = i64::from(i32::MAX) + 1 + i64::from(rng.range(1_000_000));
        harness.compare_main(format!("{value}\n").as_bytes(), "E4 above INT_MAX");
    }
}
