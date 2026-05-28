use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::os::raw::c_uint;
use std::os::unix::io::AsRawFd;
use std::sync::Mutex;

static STDOUT_LOCK: Mutex<()> = Mutex::new(());

const C_LIB: &str = "c_src/build/libtranslated_rust.so";
const RUST_LIB: &str = "target/debug/libcharinbuf_lib.so";

unsafe extern "C" {
    fn dup(fd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

// libc stdout is not directly accessible. We use fflush(NULL) which flushes all
// streams, and we redirect fd 1 (stdout) at the OS level.

/// Capture stdout produced during a closure that calls FFI functions writing to libc stdout.
fn capture_stdout<F: FnOnce() -> R, R>(f: F) -> (R, Vec<u8>) {
    let _guard = STDOUT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    unsafe {
        // Flush any pending stdio output before redirect
        fflush(std::ptr::null_mut());

        let tmp = tempfile().expect("create temp file");
        let tmp_fd = tmp.as_raw_fd();

        // Save original stdout
        let saved = dup(1);
        assert!(saved >= 0, "dup failed");

        // Redirect stdout to tmp file
        let r = dup2(tmp_fd, 1);
        assert!(r >= 0, "dup2 failed");

        let result = f();

        // Flush again after FFI calls
        fflush(std::ptr::null_mut());

        // Restore stdout
        dup2(saved, 1);
        close(saved);

        // Read the captured contents
        let mut tmp = tmp;
        tmp.seek(SeekFrom::Start(0)).expect("seek");
        let mut buf = Vec::new();
        tmp.read_to_end(&mut buf).expect("read");

        (result, buf)
    }
}

fn tempfile() -> std::io::Result<std::fs::File> {
    // Use a unique tmp file
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    let path = format!("/tmp/charinbuf_test_{}_{}.tmp", pid, n);
    let file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)?;
    // unlink the file path (still accessible by fd)
    let _ = fs::remove_file(&path);
    Ok(file)
}

fn load_lib(path: &str) -> Library {
    unsafe { Library::new(path).unwrap_or_else(|e| panic!("failed to load {}: {}", path, e)) }
}

// ---------- Tests ----------

#[test]
fn test_charinbuf_all_modes() {
    let c_lib = load_lib(C_LIB);
    let rust_lib = load_lib(RUST_LIB);

    let inputs: &[(c_int, c_int, c_int, c_int)] = &[
        (0, 100, 0, 0),
        (0, -1, 0, 0),
        (0, 65535, 0, 0),
        (0, 65536, 0, 0),
        (0, 0, 0, 0),
        (1, 0, 0, 0),
        (2, 0, 0, 0),
        (3, 10, 5, 3),
        (3, 0, 1, 1),
        (3, 100, 50, 2),
        (4, 0, 0, 0),
        (5, 0, 0, 0),
        (-1, 0, 0, 0),
        (99, 1, 2, 3),
    ];

    unsafe {
        let c_func: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
            c_lib.get(b"charinbuf").unwrap();
        let rust_func: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
            rust_lib.get(b"charinbuf").unwrap();

        for &(m, v, o1, o2) in inputs {
            let (c_ret, c_out) = capture_stdout(|| c_func(m, v, o1, o2));
            let (r_ret, r_out) = capture_stdout(|| rust_func(m, v, o1, o2));
            assert_eq!(
                c_ret, r_ret,
                "return mismatch for ({},{},{},{}): C={} Rust={}",
                m, v, o1, o2, c_ret, r_ret
            );
            assert_eq!(
                c_out,
                r_out,
                "stdout mismatch for ({},{},{},{}):\nC:    {:?}\nRust: {:?}",
                m,
                v,
                o1,
                o2,
                String::from_utf8_lossy(&c_out),
                String::from_utf8_lossy(&r_out)
            );
        }
    }
}

#[test]
fn test_validate_uint16_range() {
    let c_lib = load_lib(C_LIB);
    let rust_lib = load_lib(RUST_LIB);
    unsafe {
        let cf: Symbol<unsafe extern "C" fn(c_int) -> c_int> =
            c_lib.get(b"validate_uint16_range").unwrap();
        let rf: Symbol<unsafe extern "C" fn(c_int) -> c_int> =
            rust_lib.get(b"validate_uint16_range").unwrap();
        for v in [
            i32::MIN, -100, -1, 0, 1, 100, 65534, 65535, 65536, 100000, i32::MAX,
        ] {
            assert_eq!(cf(v), rf(v), "validate_uint16_range({})", v);
        }
    }
}

#[test]
fn test_is_string_empty() {
    let c_lib = load_lib(C_LIB);
    let rust_lib = load_lib(RUST_LIB);
    unsafe {
        let cf: Symbol<unsafe extern "C" fn(*const c_char) -> c_int> =
            c_lib.get(b"is_string_empty").unwrap();
        let rf: Symbol<unsafe extern "C" fn(*const c_char) -> c_int> =
            rust_lib.get(b"is_string_empty").unwrap();

        // null
        assert_eq!(cf(std::ptr::null()), rf(std::ptr::null()));

        // empty
        let empty = b"\0";
        assert_eq!(cf(empty.as_ptr() as *const c_char), rf(empty.as_ptr() as *const c_char));

        // non-empty
        let s = b"hello\0";
        assert_eq!(cf(s.as_ptr() as *const c_char), rf(s.as_ptr() as *const c_char));

        // single char
        let s2 = b"a\0";
        assert_eq!(cf(s2.as_ptr() as *const c_char), rf(s2.as_ptr() as *const c_char));
    }
}

#[test]
fn test_counter_funcs() {
    let c_lib = load_lib(C_LIB);
    let rust_lib = load_lib(RUST_LIB);
    unsafe {
        let c_reset: Symbol<unsafe extern "C" fn(c_int) -> c_int> =
            c_lib.get(b"reset_counter").unwrap();
        let r_reset: Symbol<unsafe extern "C" fn(c_int) -> c_int> =
            rust_lib.get(b"reset_counter").unwrap();
        let c_inc: Symbol<unsafe extern "C" fn(c_int) -> c_int> =
            c_lib.get(b"increment_counter").unwrap();
        let r_inc: Symbol<unsafe extern "C" fn(c_int) -> c_int> =
            rust_lib.get(b"increment_counter").unwrap();
        let c_dec: Symbol<unsafe extern "C" fn(c_int) -> c_int> =
            c_lib.get(b"decrement_counter").unwrap();
        let r_dec: Symbol<unsafe extern "C" fn(c_int) -> c_int> =
            rust_lib.get(b"decrement_counter").unwrap();
        let c_mul: Symbol<unsafe extern "C" fn(c_int) -> c_int> =
            c_lib.get(b"multiply_counter").unwrap();
        let r_mul: Symbol<unsafe extern "C" fn(c_int) -> c_int> =
            rust_lib.get(b"multiply_counter").unwrap();

        // reset both to 10
        assert_eq!(c_reset(10), r_reset(10));
        // inc by 5
        assert_eq!(c_inc(5), r_inc(5));
        // mul by 3
        assert_eq!(c_mul(3), r_mul(3));
        // dec by 7
        assert_eq!(c_dec(7), r_dec(7));
        // reset to 0
        assert_eq!(c_reset(0), r_reset(0));
        // inc by -100
        assert_eq!(c_inc(-100), r_inc(-100));
    }
}

#[test]
fn test_find_char_in_buffer() {
    let c_lib = load_lib(C_LIB);
    let rust_lib = load_lib(RUST_LIB);
    unsafe {
        let cf: Symbol<unsafe extern "C" fn(*const c_char, usize, c_char) -> *mut c_char> =
            c_lib.get(b"find_char_in_buffer").unwrap();
        let rf: Symbol<unsafe extern "C" fn(*const c_char, usize, c_char) -> *mut c_char> =
            rust_lib.get(b"find_char_in_buffer").unwrap();

        // null buffer
        let cn = cf(std::ptr::null(), 0, b'x' as c_char);
        let rn = rf(std::ptr::null(), 0, b'x' as c_char);
        assert!(cn.is_null());
        assert!(rn.is_null());

        // search for char that exists
        let buf = b"abcdefghij";
        let p = buf.as_ptr() as *const c_char;
        let cp = cf(p, buf.len(), b'e' as c_char);
        let rp = rf(p, buf.len(), b'e' as c_char);
        assert_eq!(
            cp.is_null(),
            rp.is_null(),
            "null mismatch"
        );
        assert!(!cp.is_null());
        let coff = (cp as isize) - (p as isize);
        let roff = (rp as isize) - (p as isize);
        assert_eq!(coff, roff);

        // not found
        let cp2 = cf(p, buf.len(), b'z' as c_char);
        let rp2 = rf(p, buf.len(), b'z' as c_char);
        assert!(cp2.is_null());
        assert!(rp2.is_null());

        // size 0
        let cp3 = cf(p, 0, b'a' as c_char);
        let rp3 = rf(p, 0, b'a' as c_char);
        assert!(cp3.is_null());
        assert!(rp3.is_null());
    }
}

#[test]
fn test_create_buffer() {
    let c_lib = load_lib(C_LIB);
    let rust_lib = load_lib(RUST_LIB);
    unsafe {
        let cb: Symbol<unsafe extern "C" fn(*const c_char) -> *mut c_char> =
            c_lib.get(b"create_buffer").unwrap();
        let rb: Symbol<unsafe extern "C" fn(*const c_char) -> *mut c_char> =
            rust_lib.get(b"create_buffer").unwrap();
        // Load free from libc directly
        let libc = Library::new("libc.so.6").expect("load libc");
        let free_sym: Symbol<unsafe extern "C" fn(*mut c_void)> =
            libc.get(b"free").expect("libc free");

        // null
        assert!(cb(std::ptr::null()).is_null());
        assert!(rb(std::ptr::null()).is_null());

        // string
        let s = b"hello world\0";
        let cp = cb(s.as_ptr() as *const c_char);
        let rp = rb(s.as_ptr() as *const c_char);
        assert!(!cp.is_null());
        assert!(!rp.is_null());
        // contents must match the input
        let n = 12; // "hello world" + nul
        let cs = std::slice::from_raw_parts(cp as *const u8, n);
        let rs = std::slice::from_raw_parts(rp as *const u8, n);
        assert_eq!(cs, &s[..]);
        assert_eq!(rs, &s[..]);
        free_sym(cp as *mut c_void);
        free_sym(rp as *mut c_void);
    }
}

#[test]
fn test_apply_operation() {
    let c_lib = load_lib(C_LIB);
    let rust_lib = load_lib(RUST_LIB);
    unsafe {
        let ca: Symbol<
            unsafe extern "C" fn(Option<unsafe extern "C" fn(c_int) -> c_int>, c_int) -> c_int,
        > = c_lib.get(b"apply_operation").unwrap();
        let ra: Symbol<
            unsafe extern "C" fn(Option<unsafe extern "C" fn(c_int) -> c_int>, c_int) -> c_int,
        > = rust_lib.get(b"apply_operation").unwrap();

        // null op -> -1
        assert_eq!(ca(None, 5), -1);
        assert_eq!(ra(None, 5), -1);

        let c_reset: Symbol<unsafe extern "C" fn(c_int) -> c_int> =
            c_lib.get(b"reset_counter").unwrap();
        let r_reset: Symbol<unsafe extern "C" fn(c_int) -> c_int> =
            rust_lib.get(b"reset_counter").unwrap();

        let cr = ca(Some(*c_reset), 42);
        let rr = ra(Some(*r_reset), 42);
        assert_eq!(cr, rr);
        assert_eq!(cr, 42);
    }
}

// Suppress unused warnings
#[allow(dead_code)]
fn _suppress_warns(_: c_uint) {}
