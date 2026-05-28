use core::ffi::{c_char, c_int};
use core::ptr;
use std::sync::atomic::{AtomicPtr, Ordering};

// Global FILE* used by all logger functions. Mirrors the static FILE *log_file
// in the original C source. We use AtomicPtr to allow safe interior mutability
// from extern "C" functions.
static LOG_FILE: AtomicPtr<libc::FILE> = AtomicPtr::new(ptr::null_mut());

fn get_log_file() -> *mut libc::FILE {
    LOG_FILE.load(Ordering::SeqCst)
}

fn set_log_file(f: *mut libc::FILE) {
    LOG_FILE.store(f, Ordering::SeqCst);
}

#[unsafe(no_mangle)]
pub extern "C" fn initialize_logger() -> c_int {
    unsafe {
        let log_file_env_name = b"LOG_FILE\0".as_ptr() as *const c_char;
        let log_file_env = libc::getenv(log_file_env_name);

        let default_path = b"default.log\0".as_ptr() as *const c_char;
        let log_file_path: *const c_char = if log_file_env.is_null() {
            default_path
        } else {
            log_file_env
        };

        let mode = b"a\0".as_ptr() as *const c_char;
        let f = libc::fopen(log_file_path, mode);
        if f.is_null() {
            let fmt = b"Failed to open log file: %s\n\0".as_ptr() as *const c_char;
            libc::fprintf(libc_stderr(), fmt, log_file_path);
            return -1;
        }

        set_log_file(f);

        let msg = b"Logger initialized.\0".as_ptr() as *const c_char;
        log_info(msg);
        0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn log_info(message: *const c_char) {
    unsafe {
        let f = get_log_file();
        if !f.is_null() {
            let fmt = b"[INFO] %s\n\0".as_ptr() as *const c_char;
            libc::fprintf(f, fmt, message);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn log_warning(message: *const c_char) {
    unsafe {
        let f = get_log_file();
        if !f.is_null() {
            let fmt = b"[WARNING] %s\n\0".as_ptr() as *const c_char;
            libc::fprintf(f, fmt, message);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn log_error(message: *const c_char) {
    unsafe {
        let f = get_log_file();
        if !f.is_null() {
            let fmt = b"[ERROR] %s\n\0".as_ptr() as *const c_char;
            libc::fprintf(f, fmt, message);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn finalize_logger() {
    unsafe {
        let f = get_log_file();
        if !f.is_null() {
            let msg = b"Logger finalized.\0".as_ptr() as *const c_char;
            log_info(msg);
            libc::fclose(f);
        }
    }
}

// Helper: portable way to get stderr FILE*. libc exposes it via an extern,
// but the symbol name differs across platforms; we use libc::dup of stderr
// fileno isn't ideal -- instead, rely on libc's exposed functions.
fn libc_stderr() -> *mut libc::FILE {
    // On most Unix platforms, stderr is exposed via the variable __stderrp or
    // simply through libc::fdopen of fd 2. The simplest portable approach is
    // to use libc's `stderr` via the dedicated function-like macro through
    // the `libc` crate.
    extern "C" {
        // Provided by the C runtime; on glibc/musl this is a function-like
        // wrapper. The libc crate exposes it as a static.
    }
    // The libc crate provides `libc::stderr` via a function-like helper on
    // some targets. To remain portable, use fdopen on fd 2.
    // However fdopen would create a new FILE*, which is undesirable. Instead,
    // use the libc crate's exposed symbol via a fallback.
    #[cfg(any(target_os = "linux", target_os = "android"))]
    unsafe {
        extern "C" {
            static mut stderr: *mut libc::FILE;
        }
        return stderr;
    }
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    unsafe {
        extern "C" {
            static mut __stderrp: *mut libc::FILE;
        }
        return __stderrp;
    }
    #[cfg(not(any(
        target_os = "linux",
        target_os = "android",
        target_os = "macos",
        target_os = "ios"
    )))]
    unsafe {
        // Fallback: open fd 2 as a stream. This may behave slightly differently
        // (separate FILE buffer), but is a safe last resort.
        let mode = b"w\0".as_ptr() as *const c_char;
        libc::fdopen(2, mode)
    }
}
