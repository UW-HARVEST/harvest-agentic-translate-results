// Shared helpers for the FFI conformance tests.
//
// Tests load BOTH the original C shared library and the Rust cdylib via
// `libloading`, then drive them through their public C ABI and assert that
// observable behavior is identical (return values + stdout + struct memory).

#![allow(dead_code)]
#![allow(non_camel_case_types)]

use libloading::{Library, Symbol};
use std::os::raw::{c_char, c_double, c_int};
use std::path::PathBuf;
use std::sync::Mutex;

pub const MAX_NAME_LENGTH: usize = 64;
pub const MAX_CATEGORY_LENGTH: usize = 32;

pub type size_t = libc::size_t;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct item_t {
    pub id: c_int,
    pub name: [c_char; MAX_NAME_LENGTH],
    pub category: [c_char; MAX_CATEGORY_LENGTH],
    pub price: c_double,
    pub quantity: c_int,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct order_t {
    pub customer_id: c_int,
    pub customer_name: [c_char; MAX_NAME_LENGTH],
    pub total_amount: c_double,
}

#[repr(C)]
pub struct array_int_t {
    pub data: *mut c_int,
    pub size: size_t,
    pub capacity: size_t,
}
#[repr(C)]
pub struct array_double_t {
    pub data: *mut c_double,
    pub size: size_t,
    pub capacity: size_t,
}
#[repr(C)]
pub struct array_item_t_t {
    pub data: *mut item_t,
    pub size: size_t,
    pub capacity: size_t,
}
#[repr(C)]
pub struct array_order_t_t {
    pub data: *mut order_t,
    pub size: size_t,
    pub capacity: size_t,
}

#[repr(C)]
pub struct list_node_int_t {
    pub data: c_int,
    pub next: *mut list_node_int_t,
}
#[repr(C)]
pub struct list_int_t {
    pub head: *mut list_node_int_t,
    pub tail: *mut list_node_int_t,
    pub size: size_t,
}

#[repr(C)]
pub struct list_node_double_t {
    pub data: c_double,
    pub next: *mut list_node_double_t,
}
#[repr(C)]
pub struct list_double_t {
    pub head: *mut list_node_double_t,
    pub tail: *mut list_node_double_t,
    pub size: size_t,
}

#[repr(C)]
pub struct list_node_item_t_t {
    pub data: item_t,
    pub next: *mut list_node_item_t_t,
}
#[repr(C)]
pub struct list_item_t_t {
    pub head: *mut list_node_item_t_t,
    pub tail: *mut list_node_item_t_t,
    pub size: size_t,
}

#[repr(C)]
pub struct list_node_order_t_t {
    pub data: order_t,
    pub next: *mut list_node_order_t_t,
}
#[repr(C)]
pub struct list_order_t_t {
    pub head: *mut list_node_order_t_t,
    pub tail: *mut list_node_order_t_t,
    pub size: size_t,
}

// Library loaders ------------------------------------------------------------

pub fn c_lib_path() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest.join("c_src").join("build").join("libdriver.so")
}

pub fn rust_lib_path() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let candidates = [
        manifest.join("target").join("release").join("libdriver.so"),
        manifest.join("target").join("debug").join("libdriver.so"),
    ];
    for c in &candidates {
        if c.exists() {
            return c.clone();
        }
    }
    candidates[1].clone()
}

pub fn load_c() -> Library {
    let p = c_lib_path();
    unsafe { Library::new(&p) }
        .unwrap_or_else(|e| panic!("failed to load C lib at {:?}: {}", p, e))
}

pub fn load_rust() -> Library {
    let p = rust_lib_path();
    unsafe { Library::new(&p) }
        .unwrap_or_else(|e| panic!("failed to load Rust lib at {:?}: {}", p, e))
}

pub fn sym<'a, T>(lib: &'a Library, name: &[u8]) -> Symbol<'a, T> {
    unsafe { lib.get(name) }
        .unwrap_or_else(|e| panic!("symbol {} missing: {}", String::from_utf8_lossy(name), e))
}

// stdout capture -------------------------------------------------------------
//
// Both implementations call libc::printf, which writes to the C runtime's
// stdout (file descriptor 1). To capture, we:
//   1. Flush stdout
//   2. Redirect fd 1 to the write end of a pipe via dup2()
//   3. Run the closure
//   4. Flush again, restore fd 1, read from the pipe
//
// We serialize captures with a global mutex so that parallel tests don't
// scramble each other's output.

static STDOUT_LOCK: Mutex<()> = Mutex::new(());

pub fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    let _guard = STDOUT_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    // The Rust stdio layer maintains its own buffer in front of fd 1; libtest
    // status output ("test foo ...", "ok\n", "FAILED\n") flows through it.
    // We must flush that buffer before redirecting fd 1, otherwise pending
    // bytes get sent into our pipe instead of the user's terminal.
    use std::io::Write as _;
    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();

    unsafe {
        libc::fflush(std::ptr::null_mut());

        let mut pipe_fds = [0i32; 2];
        if libc::pipe(pipe_fds.as_mut_ptr()) != 0 {
            panic!("pipe() failed");
        }
        let read_fd = pipe_fds[0];
        let write_fd = pipe_fds[1];

        let saved = libc::dup(1);
        if saved < 0 {
            libc::close(read_fd);
            libc::close(write_fd);
            panic!("dup(1) failed");
        }
        if libc::dup2(write_fd, 1) < 0 {
            libc::close(read_fd);
            libc::close(write_fd);
            libc::close(saved);
            panic!("dup2 failed");
        }
        libc::close(write_fd);

        // Make read end non-blocking so a final read() after closing the
        // write end doesn't hang waiting for more data.
        let flags = libc::fcntl(read_fd, libc::F_GETFL, 0);
        libc::fcntl(read_fd, libc::F_SETFL, flags | libc::O_NONBLOCK);

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));

        libc::fflush(std::ptr::null_mut());

        // Drain whatever has been written so far.
        let mut out = Vec::new();
        let mut buf = [0u8; 4096];
        loop {
            let n = libc::read(read_fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len());
            if n <= 0 {
                break;
            }
            out.extend_from_slice(&buf[..n as usize]);
        }

        libc::dup2(saved, 1);
        libc::close(saved);
        libc::close(read_fd);

        if let Err(p) = result {
            std::panic::resume_unwind(p);
        }
        out
    }
}

// Make a [c_char; N] buffer from a Rust &str (NUL-terminated, fixed length).
pub fn cstr_buf<const N: usize>(s: &str) -> [c_char; N] {
    let mut buf = [0 as c_char; N];
    let bytes = s.as_bytes();
    let copy = bytes.len().min(N - 1);
    for i in 0..copy {
        buf[i] = bytes[i] as c_char;
    }
    buf
}

/// Convert a NUL-terminated C buffer into a slice up to (excluding) the NUL.
pub fn cstr_slice(buf: &[c_char]) -> &[u8] {
    let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    // SAFETY: c_char is i8 on most platforms; reinterpret as u8 bytes.
    unsafe { std::slice::from_raw_parts(buf.as_ptr() as *const u8, end) }
}
