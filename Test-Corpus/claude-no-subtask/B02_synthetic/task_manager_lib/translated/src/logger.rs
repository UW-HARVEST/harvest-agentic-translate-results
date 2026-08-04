use std::ffi::c_char;
use std::ffi::c_int;
use std::ptr;
use std::sync::atomic::{AtomicPtr, Ordering};

// Resolve the platform's stderr FILE* at runtime.
extern "C" {
    // Linux/glibc + musl: extern FILE *stderr;
    static stderr: *mut libc::FILE;
}

fn stderr_ptr() -> *mut libc::FILE {
    unsafe { stderr }
}

// Use AtomicPtr to mirror C `static FILE *log_file = NULL;`
static LOG_FILE: AtomicPtr<libc::FILE> = AtomicPtr::new(ptr::null_mut());

fn log_file_ptr() -> *mut libc::FILE {
    LOG_FILE.load(Ordering::SeqCst)
}

#[unsafe(no_mangle)]
pub extern "C" fn initialize_logger() -> c_int {
    unsafe {
        let log_file_env = libc::getenv(b"LOG_FILE\0".as_ptr() as *const c_char);
        let log_file_path = if log_file_env.is_null() {
            b"default.log\0".as_ptr() as *const c_char
        } else {
            log_file_env as *const c_char
        };

        let mode = b"a\0".as_ptr() as *const c_char;
        let f = libc::fopen(log_file_path, mode);
        if f.is_null() {
            // fprintf(stderr, "Failed to open log file: %s\n", log_file_path);
            let stderr_file = stderr_ptr();
            libc::fprintf(
                stderr_file,
                b"Failed to open log file: %s\n\0".as_ptr() as *const c_char,
                log_file_path,
            );
            return -1;
        }

        LOG_FILE.store(f, Ordering::SeqCst);

        log_info(b"Logger initialized.\0".as_ptr() as *const c_char);
        0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn log_info(message: *const c_char) {
    unsafe {
        let f = log_file_ptr();
        if !f.is_null() {
            libc::fprintf(f, b"[INFO] %s\n\0".as_ptr() as *const c_char, message);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn log_warning(message: *const c_char) {
    unsafe {
        let f = log_file_ptr();
        if !f.is_null() {
            libc::fprintf(f, b"[WARNING] %s\n\0".as_ptr() as *const c_char, message);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn log_error(message: *const c_char) {
    unsafe {
        let f = log_file_ptr();
        if !f.is_null() {
            libc::fprintf(f, b"[ERROR] %s\n\0".as_ptr() as *const c_char, message);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn finalize_logger() {
    unsafe {
        let f = log_file_ptr();
        if !f.is_null() {
            log_info(b"Logger finalized.\0".as_ptr() as *const c_char);
            libc::fclose(f);
        }
    }
}
