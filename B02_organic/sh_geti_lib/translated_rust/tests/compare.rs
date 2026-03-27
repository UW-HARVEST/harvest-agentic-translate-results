use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void, CString};

const C_LIB_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/libtranslated_rust.so");

fn c_lib() -> Library {
    unsafe { Library::new(C_LIB_PATH).expect("Failed to load C .so") }
}

fn rust_lib() -> Library {
    // Find the Rust cdylib in target/debug/deps or target/debug
    let manifest = env!("CARGO_MANIFEST_DIR");
    let candidates = [
        format!("{}/target/debug/libsh_geti_lib.so", manifest),
        format!("{}/target/release/libsh_geti_lib.so", manifest),
    ];
    for p in &candidates {
        if std::path::Path::new(p).exists() {
            return unsafe { Library::new(p).expect("Failed to load Rust .so") };
        }
    }
    panic!("Rust .so not found. Build with `cargo build` first. Tried: {:?}", candidates);
}

// ── stbds_hash_string ──────────────────────────────────────────────────
#[test]
fn test_hash_string() {
    let clib = c_lib();
    let rlib = rust_lib();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(*mut c_char, usize) -> usize> =
            clib.get(b"stbds_hash_string").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(*mut c_char, usize) -> usize> =
            rlib.get(b"stbds_hash_string").unwrap();

        let strings = ["", "hello", "foo", "test_42", "a longer string"];
        let seeds = [0usize, 1, 42, 0xdeadbeef, 0x31415926];

        for s in &strings {
            for &sd in &seeds {
                let cs = CString::new(*s).unwrap();
                let c_r = c_fn(cs.as_ptr() as *mut c_char, sd);
                let r_r = r_fn(cs.as_ptr() as *mut c_char, sd);
                assert_eq!(c_r, r_r,
                    "hash_string({:?}, {:#x}): C={:#x} Rust={:#x}", s, sd, c_r, r_r);
            }
        }
    }
}

// ── stbds_hash_bytes ───────────────────────────────────────────────────
#[test]
fn test_hash_bytes() {
    let clib = c_lib();
    let rlib = rust_lib();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(*mut c_void, usize, usize) -> usize> =
            clib.get(b"stbds_hash_bytes").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(*mut c_void, usize, usize) -> usize> =
            rlib.get(b"stbds_hash_bytes").unwrap();

        let data: &[&[u8]] = &[
            b"", b"a", b"ab", b"abc", b"abcd", b"abcde", b"abcdef",
            b"abcdefg", b"abcdefgh", b"abcdefghi",
            b"hello world this is a longer test",
        ];
        let seeds = [0usize, 1, 42, 0xdeadbeef];

        for d in data {
            for &sd in &seeds {
                let c_r = c_fn(d.as_ptr() as *mut c_void, d.len(), sd);
                let r_r = r_fn(d.as_ptr() as *mut c_void, d.len(), sd);
                assert_eq!(c_r, r_r,
                    "hash_bytes({:?}, len={}, seed={:#x}): C={:#x} Rust={:#x}",
                    std::str::from_utf8(d).unwrap_or("<bin>"), d.len(), sd, c_r, r_r);
            }
        }
    }
}

// ── stbds_arrgrowf ─────────────────────────────────────────────────────
#[test]
fn test_arrgrowf() {
    let clib = c_lib();
    let rlib = rust_lib();
    unsafe {
        let c_grow: Symbol<unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void> =
            clib.get(b"stbds_arrgrowf").unwrap();
        let r_grow: Symbol<unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void> =
            rlib.get(b"stbds_arrgrowf").unwrap();
        let c_free: Symbol<unsafe extern "C" fn(*mut c_void)> =
            clib.get(b"stbds_arrfreef").unwrap();
        let r_free: Symbol<unsafe extern "C" fn(*mut c_void)> =
            rlib.get(b"stbds_arrfreef").unwrap();

        #[repr(C)]
        struct Hdr { length: usize, capacity: usize, hash_table: *mut c_void, temp: isize }

        let c_arr = c_grow(std::ptr::null_mut(), 4, 0, 10);
        let r_arr = r_grow(std::ptr::null_mut(), 4, 0, 10);
        assert!(!c_arr.is_null() && !r_arr.is_null());

        let ch = &*((c_arr as *mut Hdr).offset(-1));
        let rh = &*((r_arr as *mut Hdr).offset(-1));
        assert_eq!(ch.length, rh.length, "length mismatch");
        assert_eq!(ch.capacity, rh.capacity, "capacity mismatch");

        c_free(c_arr);
        r_free(r_arr);
    }
}

// ── sh_geti (capture stdout, compare byte-for-byte) ────────────────────
#[test]
fn test_sh_geti_output() {
    let clib = c_lib();
    let rlib = rust_lib();
    unsafe {
        let c_seed: Symbol<unsafe extern "C" fn(usize)> = clib.get(b"stbds_rand_seed").unwrap();
        let r_seed: Symbol<unsafe extern "C" fn(usize)> = rlib.get(b"stbds_rand_seed").unwrap();
        let c_sh: Symbol<unsafe extern "C" fn(c_int)> = clib.get(b"sh_geti").unwrap();
        let r_sh: Symbol<unsafe extern "C" fn(c_int)> = rlib.get(b"sh_geti").unwrap();

        for &num in &[0i32, 1, 4, 8, 10] {
            let c_out = capture_stdout(|| { c_seed(0x31415926); c_sh(num); });
            let r_out = capture_stdout(|| { r_seed(0x31415926); r_sh(num); });
            assert_eq!(c_out, r_out,
                "sh_geti({}) stdout mismatch.\nC:\n{}\nRust:\n{}", num, c_out, r_out);
        }
    }
}

unsafe fn capture_stdout<F: FnOnce()>(f: F) -> String {
    extern "C" {
        fn fflush(stream: *mut c_void) -> c_int;
        fn dup(fd: c_int) -> c_int;
        fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
        fn close(fd: c_int) -> c_int;
        fn pipe(pipefd: *mut c_int) -> c_int;
        fn read(fd: c_int, buf: *mut c_void, count: usize) -> isize;
        fn fcntl(fd: c_int, cmd: c_int, ...) -> c_int;
    }
    fflush(std::ptr::null_mut());
    let mut pipefd = [0i32; 2];
    assert_eq!(pipe(pipefd.as_mut_ptr()), 0);
    let saved = dup(1);
    dup2(pipefd[1], 1);
    f();
    fflush(std::ptr::null_mut());
    dup2(saved, 1);
    close(saved);
    close(pipefd[1]);
    // non-blocking read
    let flags = fcntl(pipefd[0], 3/*F_GETFL*/);
    fcntl(pipefd[0], 4/*F_SETFL*/, flags | 2048/*O_NONBLOCK*/);
    let mut out = Vec::new();
    let mut buf = [0u8; 4096];
    loop {
        let n = read(pipefd[0], buf.as_mut_ptr() as *mut c_void, buf.len());
        if n <= 0 { break; }
        out.extend_from_slice(&buf[..n as usize]);
    }
    close(pipefd[0]);
    String::from_utf8_lossy(&out).to_string()
}
