// tests/print_compare.rs
//
// Tests for the *_print and scene_list_shapes functions. These functions write
// to stdout via printf/fputs. We capture stdout by redirecting fd 1 to a temp
// file, calling the function, then restoring fd 1, and finally reading the
// captured bytes.
//
// Because the print output for scene_list_shapes/scene_print includes raw
// pointer values (%p) that *will* differ between the C and Rust singleton
// instances, the comparison strips/normalises those parts before checking
// byte-equality.

mod common;

use common::*;
use std::ffi::CString;
use std::io::Read;
use std::os::raw::c_int;
use std::os::unix::io::FromRawFd;

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut std::ffi::c_void) -> c_int;
    static stdout: *mut std::ffi::c_void;
}

/// Capture the bytes written to fd 1 (stdout) during execution of `f`.
fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    unsafe {
        // Flush libc stdout so any prior output is committed before redirection.
        fflush(stdout);

        // Save the current fd 1.
        let saved = dup(1);
        assert!(saved >= 0, "dup failed");

        // Create a temp file and redirect fd 1 to it.
        let tmp_path = std::env::temp_dir().join(format!(
            "stdout_capture_{}_{}.txt",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let tmp_file = std::fs::File::create(&tmp_path).expect("create tmp");
        let tmp_fd = std::os::unix::io::AsRawFd::as_raw_fd(&tmp_file);

        let r = dup2(tmp_fd, 1);
        assert!(r >= 0, "dup2 failed");

        // Run the function.
        f();

        // Flush libc stdout (which is now writing to tmp file).
        fflush(stdout);

        // Restore fd 1.
        dup2(saved, 1);
        close(saved);
        drop(tmp_file);

        // Read the captured output.
        let bytes = std::fs::read(&tmp_path).expect("read tmp");
        let _ = std::fs::remove_file(&tmp_path);
        bytes
    }
}

/// Replace any "0x[0-9a-fA-F]+" or "(nil)" sequences with a placeholder so we can
/// compare output that contains pointer addresses.
fn normalize_pointers(s: &[u8]) -> Vec<u8> {
    let text = String::from_utf8_lossy(s);
    let re = regex::Regex::new(r"(0x[0-9a-fA-F]+|\(nil\))").unwrap();
    re.replace_all(&text, "<PTR>").into_owned().into_bytes()
}

// We don't want to add the `regex` crate just for one test. Use a manual normalizer.
mod regex {
    pub struct Regex<'a> {
        // Just store the pattern, we'll do a custom replacement.
        _phantom: std::marker::PhantomData<&'a ()>,
    }
    impl Regex<'_> {
        pub fn new(_pattern: &str) -> Result<Self, ()> {
            Ok(Self {
                _phantom: std::marker::PhantomData,
            })
        }
        pub fn replace_all<'a>(&self, s: &'a str, _rep: &str) -> std::borrow::Cow<'a, str> {
            // Strip "0x" + hex digits, and "(nil)"
            let mut out = String::with_capacity(s.len());
            let mut chars = s.chars().peekable();
            while let Some(c) = chars.next() {
                // Detect "0x"
                if c == '0' && chars.peek() == Some(&'x') {
                    chars.next(); // consume 'x'
                    // skip hex digits
                    while let Some(&p) = chars.peek() {
                        if p.is_ascii_hexdigit() {
                            chars.next();
                        } else {
                            break;
                        }
                    }
                    out.push_str("<PTR>");
                } else if c == '(' {
                    // Detect "(nil)"
                    let mut buf = String::new();
                    buf.push(c);
                    let mut clone = chars.clone();
                    let target = "nil)";
                    let mut matched = true;
                    for tc in target.chars() {
                        if clone.next() != Some(tc) {
                            matched = false;
                            break;
                        }
                    }
                    if matched {
                        // consume the matched chars
                        for _ in 0..4 {
                            chars.next();
                        }
                        out.push_str("<PTR>");
                    } else {
                        out.push(c);
                    }
                } else {
                    out.push(c);
                }
            }
            std::borrow::Cow::Owned(out)
        }
    }
}

// ----------------------------------------------------------------
// shape_print
// ----------------------------------------------------------------

#[test]
fn shape_print_all_shapes_match() {
    let _g = common::acquire_lock();
    let c = load_c();
    let r = load_rust();
    c.shape_manager_init();
    r.shape_manager_init();

    for t in 0..SHAPE_COUNT {
        let cp = c.shape_get(t);
        let rp = r.shape_get(t);

        let cb = capture_stdout(|| c.shape_print(cp));
        let rb = capture_stdout(|| r.shape_print(rp));
        assert_eq!(
            cb,
            rb,
            "shape_print({}) differs:\nC:   {:?}\nRust: {:?}",
            t,
            String::from_utf8_lossy(&cb),
            String::from_utf8_lossy(&rb)
        );
    }

    c.shape_manager_cleanup();
    r.shape_manager_cleanup();
}

#[test]
fn shape_print_null() {
    let _g = common::acquire_lock();
    let c = load_c();
    let r = load_rust();

    let cb = capture_stdout(|| c.shape_print(std::ptr::null()));
    let rb = capture_stdout(|| r.shape_print(std::ptr::null()));
    assert_eq!(cb, rb);
}

// ----------------------------------------------------------------
// scene_print
// ----------------------------------------------------------------

#[test]
fn scene_print_empty_and_populated() {
    let _g = common::acquire_lock();
    let c = load_c();
    let r = load_rust();
    c.shape_manager_init();
    r.shape_manager_init();

    let name = CString::new("Demo").unwrap();
    let cs = c.scene_create(name.as_ptr());
    let rs = r.scene_create(name.as_ptr());

    // Empty
    let cb = capture_stdout(|| c.scene_print(cs));
    let rb = capture_stdout(|| r.scene_print(rs));
    assert_eq!(cb, rb);

    // Populate
    for t in [1, 4, 7] {
        c.scene_add_shape(cs, c.shape_get(t));
        r.scene_add_shape(rs, r.shape_get(t));
    }
    let cb = capture_stdout(|| c.scene_print(cs));
    let rb = capture_stdout(|| r.scene_print(rs));
    assert_eq!(
        cb,
        rb,
        "scene_print populated differs:\nC:   {:?}\nRust: {:?}",
        String::from_utf8_lossy(&cb),
        String::from_utf8_lossy(&rb)
    );

    c.scene_destroy(cs);
    r.scene_destroy(rs);
    c.shape_manager_cleanup();
    r.shape_manager_cleanup();
}

#[test]
fn scene_print_null() {
    let _g = common::acquire_lock();
    let c = load_c();
    let r = load_rust();
    let cb = capture_stdout(|| c.scene_print(std::ptr::null()));
    let rb = capture_stdout(|| r.scene_print(std::ptr::null()));
    assert_eq!(cb, rb);
}

// ----------------------------------------------------------------
// scene_list_shapes (contains pointer addresses, normalize)
// ----------------------------------------------------------------

#[test]
fn scene_list_shapes_after_normalizing_ptrs() {
    let _g = common::acquire_lock();
    let c = load_c();
    let r = load_rust();
    c.shape_manager_init();
    r.shape_manager_init();

    let name = CString::new("List").unwrap();
    let cs = c.scene_create(name.as_ptr());
    let rs = r.scene_create(name.as_ptr());

    for t in [0, 2, 5] {
        c.scene_add_shape(cs, c.shape_get(t));
        r.scene_add_shape(rs, r.shape_get(t));
    }

    let cb = capture_stdout(|| c.scene_list_shapes(cs));
    let rb = capture_stdout(|| r.scene_list_shapes(rs));

    let cnorm = normalize_pointers(&cb);
    let rnorm = normalize_pointers(&rb);
    assert_eq!(
        cnorm,
        rnorm,
        "scene_list_shapes differs (after ptr normalization):\nC:   {:?}\nRust: {:?}",
        String::from_utf8_lossy(&cnorm),
        String::from_utf8_lossy(&rnorm)
    );

    c.scene_destroy(cs);
    r.scene_destroy(rs);
    c.shape_manager_cleanup();
    r.shape_manager_cleanup();
}

#[test]
fn scene_list_shapes_null() {
    let _g = common::acquire_lock();
    let c = load_c();
    let r = load_rust();
    let cb = capture_stdout(|| c.scene_list_shapes(std::ptr::null()));
    let rb = capture_stdout(|| r.scene_list_shapes(std::ptr::null()));
    assert_eq!(cb, rb);
}

// Suppress unused-import warning for std::io::Read + FromRawFd
#[allow(dead_code)]
fn _unused_imports() {
    let _: Option<&dyn Read> = None;
    let _ = unsafe { std::fs::File::from_raw_fd(0) };
}
