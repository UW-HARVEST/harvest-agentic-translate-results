//! Top-level test of sh_geti: redirect stdout to a temp file, run via the
//! libloading-loaded .so, then read back to compare both implementations.

mod common;

use common::{c_lib_path, ensure_libs_built, rust_lib_path};
use libloading::{Library, Symbol};
use std::os::raw::c_int;

unsafe extern "C" {
    fn fflush(stream: *mut libc::FILE) -> c_int;
}

fn run_sh_geti(lib_path: &std::path::Path, n: c_int) -> Vec<u8> {
    unsafe {
        // Flush libc stdout, save fd, redirect to /tmp pipe, restore.
        fflush(std::ptr::null_mut());

        let saved = libc::dup(libc::STDOUT_FILENO);
        assert!(saved >= 0);

        // Use a pipe but read with a thread.
        let mut fds = [0i32; 2];
        let r = libc::pipe(fds.as_mut_ptr());
        assert_eq!(r, 0);
        let read_fd = fds[0];
        let write_fd = fds[1];

        let dup_r = libc::dup2(write_fd, libc::STDOUT_FILENO);
        assert!(dup_r >= 0);
        libc::close(write_fd);

        // Read on a separate thread to avoid pipe full.
        let handle = std::thread::spawn(move || {
            let mut buf = Vec::new();
            let mut tmp = [0u8; 4096];
            loop {
                let n = libc::read(
                    read_fd,
                    tmp.as_mut_ptr() as *mut libc::c_void,
                    tmp.len(),
                );
                if n <= 0 {
                    break;
                }
                buf.extend_from_slice(&tmp[..n as usize]);
            }
            libc::close(read_fd);
            buf
        });

        // Load .so and call sh_geti.
        let lib = Library::new(lib_path).expect("load lib");
        let sym: Symbol<unsafe extern "C" fn(c_int)> = lib.get(b"sh_geti").unwrap();
        sym(n);
        fflush(std::ptr::null_mut());

        // Restore stdout — closing the dup target also closes the write end of
        // the pipe (the only remaining ref), letting the reader see EOF.
        libc::dup2(saved, libc::STDOUT_FILENO);
        libc::close(saved);

        handle.join().expect("reader thread")
    }
}

#[test]
fn test_sh_geti_matches_for_various_inputs() {
    ensure_libs_built();
    for &n in &[0, 1, 2, 3, 5, 6, 10, 16, 32, 100] {
        let c_out = run_sh_geti(&c_lib_path(), n);
        let r_out = run_sh_geti(&rust_lib_path(), n);
        assert_eq!(
            c_out,
            r_out,
            "sh_geti({}) differs:\nC:\n{}\nRust:\n{}",
            n,
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&r_out),
        );
    }
}
