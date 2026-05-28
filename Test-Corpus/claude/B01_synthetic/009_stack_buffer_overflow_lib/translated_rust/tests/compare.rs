use libloading::{Library, Symbol};
use std::ffi::CString;
use std::fs::File;
use std::io::Read;
use std::os::raw::{c_char, c_int};
use std::os::unix::io::AsRawFd;
use std::sync::Mutex;

// libc bindings we need for stdout capture
extern "C" {
    fn fflush(stream: *mut libc::FILE) -> c_int;
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
}

// Serialize stdout-redirection to avoid races between tests
static REDIRECT_LOCK: Mutex<()> = Mutex::new(());

const C_SO: &str = "c_src/build/libdriver.so";
const RUST_SO: &str = "target/debug/libdriver.so";

fn stdout_fd() -> c_int {
    // file descriptor 1 is stdout
    1
}

/// Run `f` while capturing everything written to the C stdout (fd 1) and
/// return the captured bytes.
fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    let _guard = REDIRECT_LOCK.lock().unwrap();

    // Flush any pending output on stdout (the libc-level FILE*).
    unsafe {
        fflush(libc_stdout());
    }

    // Save the current stdout fd
    let saved = unsafe { dup(stdout_fd()) };
    assert!(saved >= 0, "dup() failed");

    // Create a tempfile and redirect stdout to it
    let tmp_path = std::env::temp_dir().join(format!(
        "harvest-stdout-{}-{}.txt",
        std::process::id(),
        rand_hex()
    ));
    let tmp_file = File::create(&tmp_path).expect("create tmpfile");
    let tmp_fd = tmp_file.as_raw_fd();

    unsafe {
        let r = dup2(tmp_fd, stdout_fd());
        assert!(r >= 0, "dup2 to tempfile failed");
    }

    f();

    // Flush libc stdout so all output is in our tempfile
    unsafe {
        fflush(libc_stdout());
    }

    // Restore stdout
    unsafe {
        let r = dup2(saved, stdout_fd());
        assert!(r >= 0, "dup2 restore failed");
        close(saved);
    }

    drop(tmp_file);

    let mut data = Vec::new();
    let mut f = File::open(&tmp_path).expect("reopen tmpfile");
    f.read_to_end(&mut data).expect("read tmpfile");
    let _ = std::fs::remove_file(&tmp_path);
    data
}

fn rand_hex() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    format!("{:x}", nanos)
}

// stdout FILE* lookup. On Linux glibc, "stdout" is an extern symbol pointing
// to a FILE*.
extern "C" {
    static stdout: *mut libc::FILE;
}
unsafe fn libc_stdout() -> *mut libc::FILE {
    stdout
}

fn load_libs() -> (Library, Library) {
    unsafe {
        let c = Library::new(C_SO).expect("load C .so");
        let r = Library::new(RUST_SO).expect("load Rust .so");
        (c, r)
    }
}

fn run_print_line(lib: &Library, msg: Option<&str>) -> Vec<u8> {
    unsafe {
        let f: Symbol<unsafe extern "C" fn(*const c_char)> =
            lib.get(b"printLine").expect("symbol printLine");
        match msg {
            Some(s) => {
                let cs = CString::new(s).unwrap();
                capture_stdout(|| f(cs.as_ptr()))
            }
            None => capture_stdout(|| f(std::ptr::null())),
        }
    }
}

fn run_print_int_line(lib: &Library, n: c_int) -> Vec<u8> {
    unsafe {
        let f: Symbol<unsafe extern "C" fn(c_int)> =
            lib.get(b"printIntLine").expect("symbol printIntLine");
        capture_stdout(|| f(n))
    }
}

fn run_bad(lib: &Library, n: c_int) -> Vec<u8> {
    unsafe {
        let f: Symbol<unsafe extern "C" fn(c_int)> = lib.get(b"bad").expect("symbol bad");
        capture_stdout(|| f(n))
    }
}

fn run_good(lib: &Library, n: c_int) -> Vec<u8> {
    unsafe {
        let f: Symbol<unsafe extern "C" fn(c_int)> = lib.get(b"good").expect("symbol good");
        capture_stdout(|| f(n))
    }
}

fn run_driver(lib: &Library, g: c_int, b: c_int) -> Vec<u8> {
    unsafe {
        let f: Symbol<unsafe extern "C" fn(c_int, c_int)> =
            lib.get(b"driver").expect("symbol driver");
        capture_stdout(|| f(g, b))
    }
}

#[test]
fn test_print_line_basic() {
    let (c, r) = load_libs();
    for s in &["hello", "", "with spaces", "lots of \t tabs"] {
        let co = run_print_line(&c, Some(s));
        let ro = run_print_line(&r, Some(s));
        assert_eq!(co, ro, "printLine mismatch for {:?}", s);
    }
}

#[test]
fn test_print_line_null() {
    let (c, r) = load_libs();
    let co = run_print_line(&c, None);
    let ro = run_print_line(&r, None);
    assert_eq!(co, ro, "printLine(NULL) mismatch");
}

#[test]
fn test_print_int_line() {
    let (c, r) = load_libs();
    for n in &[0i32, 1, -1, 42, i32::MIN, i32::MAX, -100, 1000] {
        let co = run_print_int_line(&c, *n);
        let ro = run_print_int_line(&r, *n);
        assert_eq!(co, ro, "printIntLine mismatch for {}", n);
    }
}

#[test]
fn test_bad_in_bounds() {
    let (c, r) = load_libs();
    for n in 0..10 {
        let co = run_bad(&c, n);
        let ro = run_bad(&r, n);
        assert_eq!(co, ro, "bad({}) mismatch", n);
    }
}

#[test]
fn test_bad_negative() {
    let (c, r) = load_libs();
    for n in &[-1, -100, i32::MIN] {
        let co = run_bad(&c, *n);
        let ro = run_bad(&r, *n);
        assert_eq!(co, ro, "bad({}) mismatch", n);
    }
}

#[test]
fn test_good_valid() {
    let (c, r) = load_libs();
    for n in 0..10 {
        let co = run_good(&c, n);
        let ro = run_good(&r, n);
        assert_eq!(co, ro, "good({}) mismatch", n);
    }
}

#[test]
fn test_good_out_of_bounds() {
    let (c, r) = load_libs();
    for n in &[-1i32, -100, 10, 100, i32::MAX] {
        let co = run_good(&c, *n);
        let ro = run_good(&r, *n);
        assert_eq!(co, ro, "good({}) mismatch", n);
    }
}

#[test]
fn test_driver_combinations() {
    let (c, r) = load_libs();
    let cases = [
        (0, 0),
        (5, 5),
        (9, 9),
        (3, -1),
        (-1, 3),
        (10, 10),
        (-100, -100),
    ];
    for (g, b) in cases {
        let co = run_driver(&c, g, b);
        let ro = run_driver(&r, g, b);
        assert_eq!(co, ro, "driver({},{}) mismatch", g, b);
    }
}
