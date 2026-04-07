use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libtranslated_rust.so")
}

fn rust_lib_path() -> PathBuf {
    // The cdylib is built in target/debug/ (or release/)
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target/debug/libcharinbuf_lib.so");
    p
}

struct Libs {
    c: Library,
    rs: Library,
}

impl Libs {
    fn load() -> Self {
        unsafe {
            Libs {
                c: Library::new(c_lib_path()).expect("load C .so"),
                rs: Library::new(rust_lib_path()).expect("load Rust .so"),
            }
        }
    }
}

// ── validate_uint16_range ──

#[test]
fn test_validate_uint16_range() {
    let libs = Libs::load();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(c_int) -> c_int> =
            libs.c.get(b"validate_uint16_range").unwrap();
        let rs_fn: Symbol<unsafe extern "C" fn(c_int) -> c_int> =
            libs.rs.get(b"validate_uint16_range").unwrap();

        for v in [-1, 0, 1, 100, 65535, 65536, i32::MAX, i32::MIN] {
            let c_r = c_fn(v);
            let rs_r = rs_fn(v);
            assert_eq!(c_r, rs_r, "validate_uint16_range({v}): C={c_r} Rust={rs_r}");
        }
    }
}

// ── is_string_empty ──

#[test]
fn test_is_string_empty() {
    let libs = Libs::load();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(*const u8) -> c_int> =
            libs.c.get(b"is_string_empty").unwrap();
        let rs_fn: Symbol<unsafe extern "C" fn(*const u8) -> c_int> =
            libs.rs.get(b"is_string_empty").unwrap();

        let cases: &[&[u8]] = &[b"\0", b"hello\0", b"X\0"];
        for s in cases {
            let c_r = c_fn(s.as_ptr());
            let rs_r = rs_fn(s.as_ptr());
            assert_eq!(c_r, rs_r, "is_string_empty({:?}): C={c_r} Rust={rs_r}", s);
        }
        // NULL case
        let c_r = c_fn(std::ptr::null());
        let rs_r = rs_fn(std::ptr::null());
        assert_eq!(c_r, rs_r, "is_string_empty(NULL)");
    }
}

// ── counter functions (reset, increment, decrement, multiply) ──
// These use a static variable inside each .so, so we test sequences.

#[test]
fn test_counter_functions() {
    let libs = Libs::load();
    unsafe {
        let c_reset: Symbol<unsafe extern "C" fn(c_int) -> c_int> =
            libs.c.get(b"reset_counter").unwrap();
        let c_inc: Symbol<unsafe extern "C" fn(c_int) -> c_int> =
            libs.c.get(b"increment_counter").unwrap();
        let c_dec: Symbol<unsafe extern "C" fn(c_int) -> c_int> =
            libs.c.get(b"decrement_counter").unwrap();
        let c_mul: Symbol<unsafe extern "C" fn(c_int) -> c_int> =
            libs.c.get(b"multiply_counter").unwrap();

        let rs_reset: Symbol<unsafe extern "C" fn(c_int) -> c_int> =
            libs.rs.get(b"reset_counter").unwrap();
        let rs_inc: Symbol<unsafe extern "C" fn(c_int) -> c_int> =
            libs.rs.get(b"increment_counter").unwrap();
        let rs_dec: Symbol<unsafe extern "C" fn(c_int) -> c_int> =
            libs.rs.get(b"decrement_counter").unwrap();
        let rs_mul: Symbol<unsafe extern "C" fn(c_int) -> c_int> =
            libs.rs.get(b"multiply_counter").unwrap();

        // Reset both to 10
        assert_eq!(c_reset(10), rs_reset(10));
        // Increment by 5 -> 15
        assert_eq!(c_inc(5), rs_inc(5));
        // Multiply by 3 -> 45
        assert_eq!(c_mul(3), rs_mul(3));
        // Decrement by 20 -> 25
        assert_eq!(c_dec(20), rs_dec(20));
        // Reset to 0
        assert_eq!(c_reset(0), rs_reset(0));
        // Decrement by 1 -> -1
        assert_eq!(c_dec(1), rs_dec(1));
    }
}

// ── create_buffer ──

#[test]
fn test_create_buffer() {
    let libs = Libs::load();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(*const u8) -> *mut u8> =
            libs.c.get(b"create_buffer").unwrap();
        let rs_fn: Symbol<unsafe extern "C" fn(*const u8) -> *mut u8> =
            libs.rs.get(b"create_buffer").unwrap();

        // NULL input
        let c_r = c_fn(std::ptr::null());
        let rs_r = rs_fn(std::ptr::null());
        assert!(c_r.is_null() && rs_r.is_null(), "create_buffer(NULL) should return NULL");

        // Normal string
        let input = b"test string\0";
        let c_buf = c_fn(input.as_ptr());
        let rs_buf = rs_fn(input.as_ptr());
        assert!(!c_buf.is_null());
        assert!(!rs_buf.is_null());

        // Compare contents byte-by-byte
        let len = input.len() - 1; // exclude our trailing \0 for length, but include it in comparison
        for i in 0..=len {
            assert_eq!(
                *c_buf.add(i), *rs_buf.add(i),
                "create_buffer content mismatch at byte {i}"
            );
        }

        // Free both
        libc::free(c_buf as *mut libc::c_void);
        libc::free(rs_buf as *mut libc::c_void);
    }
}

// ── find_char_in_buffer ──

#[test]
fn test_find_char_in_buffer() {
    let libs = Libs::load();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(*const u8, usize, u8) -> *mut u8> =
            libs.c.get(b"find_char_in_buffer").unwrap();
        let rs_fn: Symbol<unsafe extern "C" fn(*const u8, usize, u8) -> *mut u8> =
            libs.rs.get(b"find_char_in_buffer").unwrap();

        // NULL buffer
        let c_r = c_fn(std::ptr::null(), 10, b'a');
        let rs_r = rs_fn(std::ptr::null(), 10, b'a');
        assert!(c_r.is_null() && rs_r.is_null());

        // Find existing char - compare offsets
        let buf = b"Hello World";
        let c_r = c_fn(buf.as_ptr(), buf.len(), b'W');
        let rs_r = rs_fn(buf.as_ptr(), buf.len(), b'W');
        let c_off = c_r as usize - buf.as_ptr() as usize;
        let rs_off = rs_r as usize - buf.as_ptr() as usize;
        assert_eq!(c_off, rs_off, "find_char_in_buffer offset mismatch for 'W'");

        // Char not found
        let c_r = c_fn(buf.as_ptr(), buf.len(), b'Z');
        let rs_r = rs_fn(buf.as_ptr(), buf.len(), b'Z');
        assert!(c_r.is_null() && rs_r.is_null(), "should be NULL for missing char");
    }
}

// ── apply_operation ──

#[test]
fn test_apply_operation() {
    let libs = Libs::load();
    unsafe {
        type OpFunc = unsafe extern "C" fn(
            Option<unsafe extern "C" fn(c_int) -> c_int>,
            c_int,
        ) -> c_int;
        let c_fn: Symbol<OpFunc> = libs.c.get(b"apply_operation").unwrap();
        let rs_fn: Symbol<OpFunc> = libs.rs.get(b"apply_operation").unwrap();

        // NULL function pointer
        let c_r = c_fn(None, 42);
        let rs_r = rs_fn(None, 42);
        assert_eq!(c_r, rs_r, "apply_operation(NULL, 42)");
        assert_eq!(c_r, -1);

        // With a real function pointer - use reset_counter from each lib
        // We need to get the function pointers from each respective library
        let c_reset: Symbol<unsafe extern "C" fn(c_int) -> c_int> =
            libs.c.get(b"reset_counter").unwrap();
        let rs_reset: Symbol<unsafe extern "C" fn(c_int) -> c_int> =
            libs.rs.get(b"reset_counter").unwrap();

        let c_reset_ptr: unsafe extern "C" fn(c_int) -> c_int = *c_reset;
        let rs_reset_ptr: unsafe extern "C" fn(c_int) -> c_int = *rs_reset;

        let c_r = c_fn(Some(c_reset_ptr), 99);
        let rs_r = rs_fn(Some(rs_reset_ptr), 99);
        assert_eq!(c_r, rs_r, "apply_operation(reset_counter, 99)");
    }
}

// ── charinbuf (top-level) ──
// Compare return values for all modes. Stdout output is a side effect;
// we focus on return value correctness.

#[test]
fn test_charinbuf_mode0() {
    let libs = Libs::load();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
            libs.c.get(b"charinbuf").unwrap();
        let rs_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
            libs.rs.get(b"charinbuf").unwrap();

        // Valid uint16 values
        for v in [0, 1, 100, 65535] {
            let c_r = c_fn(0, v, 0, 0);
            let rs_r = rs_fn(0, v, 0, 0);
            assert_eq!(c_r, rs_r, "charinbuf(0, {v}, 0, 0)");
        }
        // Out of range
        for v in [-1, 65536, i32::MAX] {
            let c_r = c_fn(0, v, 0, 0);
            let rs_r = rs_fn(0, v, 0, 0);
            assert_eq!(c_r, rs_r, "charinbuf(0, {v}, 0, 0)");
        }
    }
}

#[test]
fn test_charinbuf_mode1() {
    let libs = Libs::load();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
            libs.c.get(b"charinbuf").unwrap();
        let rs_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
            libs.rs.get(b"charinbuf").unwrap();

        let c_r = c_fn(1, 0, 0, 0);
        let rs_r = rs_fn(1, 0, 0, 0);
        assert_eq!(c_r, rs_r, "charinbuf mode 1");
    }
}

#[test]
fn test_charinbuf_mode2() {
    let libs = Libs::load();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
            libs.c.get(b"charinbuf").unwrap();
        let rs_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
            libs.rs.get(b"charinbuf").unwrap();

        let c_r = c_fn(2, 0, 0, 0);
        let rs_r = rs_fn(2, 0, 0, 0);
        assert_eq!(c_r, rs_r, "charinbuf mode 2");
    }
}

#[test]
fn test_charinbuf_mode3() {
    let libs = Libs::load();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
            libs.c.get(b"charinbuf").unwrap();
        let rs_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
            libs.rs.get(b"charinbuf").unwrap();

        // mode=3, value=10, opt1=3, opt2=2 => reset(10), inc(3)=13, mul(2)=26, dec(5)=21
        let c_r = c_fn(3, 10, 3, 2);
        let rs_r = rs_fn(3, 10, 3, 2);
        assert_eq!(c_r, rs_r, "charinbuf(3, 10, 3, 2)");

        // Another set of values
        let c_r = c_fn(3, 0, 0, 0);
        let rs_r = rs_fn(3, 0, 0, 0);
        assert_eq!(c_r, rs_r, "charinbuf(3, 0, 0, 0)");
    }
}

#[test]
fn test_charinbuf_mode4() {
    let libs = Libs::load();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
            libs.c.get(b"charinbuf").unwrap();
        let rs_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
            libs.rs.get(b"charinbuf").unwrap();

        let c_r = c_fn(4, 0, 0, 0);
        let rs_r = rs_fn(4, 0, 0, 0);
        assert_eq!(c_r, rs_r, "charinbuf mode 4");
    }
}

#[test]
fn test_charinbuf_default() {
    let libs = Libs::load();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
            libs.c.get(b"charinbuf").unwrap();
        let rs_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
            libs.rs.get(b"charinbuf").unwrap();

        for mode in [5, -1, 100] {
            let c_r = c_fn(mode, 0, 0, 0);
            let rs_r = rs_fn(mode, 0, 0, 0);
            assert_eq!(c_r, rs_r, "charinbuf({mode}, 0, 0, 0) default case");
        }
    }
}

// ── stdout comparison for charinbuf ──
// Capture stdout from both libraries and compare output strings.

#[cfg(target_os = "linux")]
mod stdout_capture {
    use super::*;
    use std::io::Read;
    use std::os::unix::io::FromRawFd;

    /// Redirect C-level stdout to a pipe, call `f`, restore stdout, return captured bytes.
    unsafe fn capture_c_stdout<F: FnOnce()>(f: F) -> String {
        // Flush Rust and C stdout
        libc::fflush(std::ptr::null_mut()); // fflush(NULL) flushes all

        let mut pipe_fds = [0i32; 2];
        assert_eq!(libc::pipe(pipe_fds.as_mut_ptr()), 0);

        let saved_fd = libc::dup(1);
        assert!(saved_fd >= 0);
        libc::dup2(pipe_fds[1], 1);
        libc::close(pipe_fds[1]);

        f();

        libc::fflush(std::ptr::null_mut());
        libc::dup2(saved_fd, 1);
        libc::close(saved_fd);

        let mut file = std::fs::File::from_raw_fd(pipe_fds[0]);
        // Set non-blocking to avoid hanging if nothing was written
        libc::fcntl(pipe_fds[0], libc::F_SETFL, libc::O_NONBLOCK);
        let mut buf = Vec::new();
        let _ = file.read_to_end(&mut buf);
        // Don't let File close the fd again — it's already consumed
        std::mem::forget(file);
        libc::close(pipe_fds[0]);
        String::from_utf8_lossy(&buf).into_owned()
    }

    #[test]
    fn test_charinbuf_stdout_mode0() {
        let libs = Libs::load();
        unsafe {
            let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
                libs.c.get(b"charinbuf").unwrap();
            let rs_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
                libs.rs.get(b"charinbuf").unwrap();

            let (c_out, c_ret) = {
                let mut ret = 0;
                let out = capture_c_stdout(|| ret = c_fn(0, 100, 0, 0));
                (out, ret)
            };
            let (rs_out, rs_ret) = {
                let mut ret = 0;
                let out = capture_c_stdout(|| ret = rs_fn(0, 100, 0, 0));
                (out, ret)
            };
            assert_eq!(c_ret, rs_ret, "mode0 return value");
            assert_eq!(c_out, rs_out, "mode0 stdout mismatch\nC:  {c_out:?}\nRs: {rs_out:?}");
        }
    }

    #[test]
    fn test_charinbuf_stdout_mode1() {
        let libs = Libs::load();
        unsafe {
            let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
                libs.c.get(b"charinbuf").unwrap();
            let rs_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
                libs.rs.get(b"charinbuf").unwrap();

            let (c_out, c_ret) = {
                let mut ret = 0;
                let out = capture_c_stdout(|| ret = c_fn(1, 0, 0, 0));
                (out, ret)
            };
            let (rs_out, rs_ret) = {
                let mut ret = 0;
                let out = capture_c_stdout(|| ret = rs_fn(1, 0, 0, 0));
                (out, ret)
            };
            assert_eq!(c_ret, rs_ret, "mode1 return value");
            assert_eq!(c_out, rs_out, "mode1 stdout mismatch\nC:  {c_out:?}\nRs: {rs_out:?}");
        }
    }

    #[test]
    fn test_charinbuf_stdout_mode3() {
        let libs = Libs::load();
        unsafe {
            let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
                libs.c.get(b"charinbuf").unwrap();
            let rs_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
                libs.rs.get(b"charinbuf").unwrap();

            let (c_out, c_ret) = {
                let mut ret = 0;
                let out = capture_c_stdout(|| ret = c_fn(3, 10, 3, 2));
                (out, ret)
            };
            let (rs_out, rs_ret) = {
                let mut ret = 0;
                let out = capture_c_stdout(|| ret = rs_fn(3, 10, 3, 2));
                (out, ret)
            };
            assert_eq!(c_ret, rs_ret, "mode3 return value");
            assert_eq!(c_out, rs_out, "mode3 stdout mismatch\nC:  {c_out:?}\nRs: {rs_out:?}");
        }
    }

    #[test]
    fn test_charinbuf_stdout_mode4() {
        let libs = Libs::load();
        unsafe {
            let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
                libs.c.get(b"charinbuf").unwrap();
            let rs_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
                libs.rs.get(b"charinbuf").unwrap();

            let (c_out, c_ret) = {
                let mut ret = 0;
                let out = capture_c_stdout(|| ret = c_fn(4, 0, 0, 0));
                (out, ret)
            };
            let (rs_out, rs_ret) = {
                let mut ret = 0;
                let out = capture_c_stdout(|| ret = rs_fn(4, 0, 0, 0));
                (out, ret)
            };
            assert_eq!(c_ret, rs_ret, "mode4 return value");
            assert_eq!(c_out, rs_out, "mode4 stdout mismatch\nC:  {c_out:?}\nRs: {rs_out:?}");
        }
    }
}
