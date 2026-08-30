use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};
use std::path::{Path, PathBuf};
use std::ptr;
use std::thread;
use std::time::{Duration, Instant};

type Driver = unsafe extern "C" fn(c_int, c_int);

const STDOUT_FILENO: c_int = 1;
const F_SETFL: c_int = 4;
const O_NONBLOCK: c_int = 0o4000;
const WNOHANG: c_int = 1;
const SIGKILL: c_int = 9;
const _IONBF: c_int = 2;

unsafe extern "C" {
    static mut stdout: *mut c_void;

    fn pipe(pipefd: *mut c_int) -> c_int;
    fn fork() -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fcntl(fd: c_int, cmd: c_int, arg: c_int) -> c_int;
    fn read(fd: c_int, buffer: *mut c_void, count: usize) -> isize;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn kill(pid: c_int, signal: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn setvbuf(stream: *mut c_void, buffer: *mut i8, mode: c_int, size: usize) -> c_int;
    fn _exit(status: c_int) -> !;
}

#[derive(Debug)]
struct Run {
    output: Vec<u8>,
    completed: bool,
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

    fn range(&mut self, start: i32, end_inclusive: i32) -> i32 {
        let width = (i64::from(end_inclusive) - i64::from(start) + 1) as u32;
        start + (self.next_u32() % width) as i32
    }
}

fn c_library() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/build/libdriver.so")
}

fn rust_library() -> PathBuf {
    std::env::var_os("RUST_DRIVER_SO")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            Path::new(env!("CARGO_MANIFEST_DIR")).join("target/release/libdriver.so")
        })
}

fn run(path: &Path, x: i32, y: i32, byte_limit: usize) -> Run {
    assert!(
        path.is_file(),
        "shared library is missing: {}",
        path.display()
    );

    // Keep the handle alive until the forked call and child cleanup are complete.
    let library = unsafe { Library::new(path) }
        .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
    let driver: Symbol<'_, Driver> = unsafe { library.get(b"driver\0") }
        .unwrap_or_else(|error| panic!("failed to resolve driver in {}: {error}", path.display()));
    let driver = *driver;

    unsafe {
        fflush(ptr::null_mut());
    }
    let mut fds = [-1; 2];
    assert_eq!(unsafe { pipe(fds.as_mut_ptr()) }, 0, "pipe failed");
    let pid = unsafe { fork() };
    assert!(pid >= 0, "fork failed");

    if pid == 0 {
        unsafe {
            close(fds[0]);
            if dup2(fds[1], STDOUT_FILENO) < 0 {
                _exit(120);
            }
            close(fds[1]);
            setvbuf(stdout, ptr::null_mut(), _IONBF, 0);
            driver(x, y);
            fflush(ptr::null_mut());
            _exit(0);
        }
    }

    unsafe {
        close(fds[1]);
        assert!(fcntl(fds[0], F_SETFL, O_NONBLOCK) >= 0, "fcntl failed");
    }

    let deadline = Instant::now() + Duration::from_secs(2);
    let mut output = Vec::with_capacity(byte_limit);
    let mut status = 0;
    let mut completed = false;

    loop {
        let remaining = byte_limit.saturating_sub(output.len());
        if remaining > 0 {
            let mut chunk = [0_u8; 4096];
            let requested = remaining.min(chunk.len());
            let count = unsafe { read(fds[0], chunk.as_mut_ptr().cast(), requested) };
            if count > 0 {
                output.extend_from_slice(&chunk[..count as usize]);
            }
        }

        let wait_result = unsafe { waitpid(pid, &mut status, WNOHANG) };
        if wait_result == pid {
            completed = status == 0;
            loop {
                let remaining = byte_limit.saturating_sub(output.len());
                if remaining == 0 {
                    break;
                }
                let mut chunk = [0_u8; 4096];
                let requested = remaining.min(chunk.len());
                let count = unsafe { read(fds[0], chunk.as_mut_ptr().cast(), requested) };
                if count <= 0 {
                    break;
                }
                output.extend_from_slice(&chunk[..count as usize]);
            }
            break;
        }

        if output.len() == byte_limit || Instant::now() >= deadline {
            unsafe {
                kill(pid, SIGKILL);
                waitpid(pid, &mut status, 0);
            }
            break;
        }

        thread::sleep(Duration::from_millis(1));
    }

    unsafe {
        close(fds[0]);
    }

    Run { output, completed }
}

fn assert_matching(x: i32, y: i32, byte_limit: usize, should_complete: bool) {
    let c = run(&c_library(), x, y, byte_limit);
    let rust = run(&rust_library(), x, y, byte_limit);

    assert_eq!(
        c.completed, should_complete,
        "unexpected C completion state for driver({x}, {y}): {c:?}"
    );
    assert_eq!(
        rust.completed, should_complete,
        "unexpected Rust completion state for driver({x}, {y}): {rust:?}"
    );
    assert_eq!(rust.output, c.output, "stdout differs for driver({x}, {y})");
}

#[test]
fn all_configuration_rows_match() {
    let mut rng = Rng::new(0x4d59_5df4_d0f3_3173);
    const FINITE_LIMIT: usize = 64 * 1024;
    const PREFIX_LIMIT: usize = 8 * 1024;

    // CONFIGS.md row 1: x <= 0, y <= 0, including both signed boundaries.
    for &(x, y) in &[(0, 0), (i32::MIN, 0), (0, i32::MIN), (i32::MIN, i32::MIN)] {
        assert_matching(x, y, FINITE_LIMIT, true);
    }
    for _ in 0..64 {
        assert_matching(rng.range(-1000, 0), rng.range(-1000, 0), FINITE_LIMIT, true);
    }

    // Row 2: x > 0, y == 0.
    for _ in 0..64 {
        assert_matching(rng.range(1, 64), 0, FINITE_LIMIT, true);
    }

    // Row 3: x <= 0, y > 0.
    for _ in 0..64 {
        assert_matching(rng.range(-1000, 0), rng.range(1, 64), FINITE_LIMIT, true);
    }

    // Row 4 is the singleton special-jump condition.
    assert_matching(1, 4, FINITE_LIMIT, true);

    // Row 5: positive x and y where the decremented x is below 3.
    for _ in 0..64 {
        let mut x = rng.range(1, 3);
        let mut y = rng.range(1, 64);
        if x == 1 && y == 4 {
            y = 5;
        }
        assert_matching(x, y, FINITE_LIMIT, true);
        x = 3;
        assert_matching(x, y, FINITE_LIMIT, true);
    }

    // Row 6: positive x and y where the decremented x remains at least 3.
    for _ in 0..64 {
        assert_matching(rng.range(4, 64), rng.range(1, 64), FINITE_LIMIT, true);
    }

    // Row 7: negative y eventually produces a nonterminating label cycle.
    for _ in 0..24 {
        assert_matching(rng.range(1, 64), rng.range(-64, -1), PREFIX_LIMIT, false);
    }
    assert_matching(1, i32::MIN, PREFIX_LIMIT, false);

    // Row 8: valid signed boundaries with impractically long output.
    assert_matching(i32::MAX, 0, PREFIX_LIMIT, false);
    assert_matching(0, i32::MAX, PREFIX_LIMIT, false);
    assert_matching(i32::MAX, i32::MIN, PREFIX_LIMIT, false);
}
