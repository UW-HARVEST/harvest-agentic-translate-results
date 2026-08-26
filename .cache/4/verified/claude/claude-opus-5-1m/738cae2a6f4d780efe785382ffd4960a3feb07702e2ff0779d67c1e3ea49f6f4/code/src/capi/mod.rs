//! The C ABI surface of the library.
//!
//! `c_src` is built (see `build_c_lib.sh`) into `libcdriver.so` from exactly the
//! source list of the CMake target (`src/main.c src/scene.c src/shape.c`), so
//! the shared object exports the public functions of all three translation
//! units - `shape.c`, `scene.c` and the application level functions of
//! `main.c`, `main` included.
//!
//! Every one of those symbols is re-created here with `#[no_mangle]` /
//! `extern "C"` so that the Rust `cdylib` is a drop in replacement for the C
//! shared object: identical struct layouts, identical return values, identical
//! bytes on `stdout` / `stderr` and identical file contents.

pub mod app;
pub mod scene;
pub mod shape;

/// The three standard streams of the C library.  They are plain data symbols in
/// glibc, but the `libc` crate does not declare them, so they are declared here.
pub(crate) mod cstdio {
    use libc::FILE;

    extern "C" {
        static stdin: *mut FILE;
        static stdout: *mut FILE;
        static stderr: *mut FILE;
    }

    #[inline]
    pub(crate) unsafe fn c_stdin() -> *mut FILE {
        stdin
    }

    #[inline]
    #[allow(dead_code)]
    pub(crate) unsafe fn c_stdout() -> *mut FILE {
        stdout
    }

    #[inline]
    pub(crate) unsafe fn c_stderr() -> *mut FILE {
        stderr
    }
}
