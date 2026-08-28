//! Translation of `c_src/src/logger.c` / `c_src/include/logger.h`.

use crate::cstdio::print_stderr;
use crate::cutil::{c_str_bytes, getenv_bytes};
use crate::stdio_stream::StdioStream;
use std::ffi::{c_char, c_int};
use std::fs::OpenOptions;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

/// `static FILE *log_file = NULL;`
///
/// `StdioStream` stands in for the `FILE *`: writes are buffered and flushed
/// when the stream is closed, matching stdio's behaviour.
static LOG_FILE: Mutex<Option<StdioStream>> = Mutex::new(None);

/// Streams that `initialize_logger()` overwrote without closing.  The C code
/// leaks the previous `FILE *`; glibc still flushes it at process exit, after
/// every stream that was closed explicitly.  New streams are prepended to
/// glibc's stream list, so exit-time flushing runs newest first.
static LEAKED: Mutex<Vec<StdioStream>> = Mutex::new(Vec::new());

/// glibc flushes every still-open stdio stream when the process exits (see
/// `_IO_cleanup`).  The original code can leave `log_file` open — e.g.
/// `driver()` returns `EXIT_FAILURE` without calling `finalize_logger()` when
/// `create_task_manager()` fails — and the buffered text still reaches the
/// file.  Registering an `atexit` hook reproduces that.
static EXIT_HOOK_REGISTERED: AtomicBool = AtomicBool::new(false);

extern "C" {
    fn atexit(cb: extern "C" fn()) -> c_int;
}

extern "C" fn flush_at_exit() {
    {
        let mut guard = LOG_FILE.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(writer) = guard.as_mut() {
            writer.flush();
        }
    }
    let mut leaked = LEAKED.lock().unwrap_or_else(|e| e.into_inner());
    for writer in leaked.iter_mut().rev() {
        writer.flush();
    }
}

fn register_exit_hook() {
    if !EXIT_HOOK_REGISTERED.swap(true, Ordering::SeqCst) {
        unsafe { atexit(flush_at_exit) };
    }
}

fn open_append(path_bytes: &[u8]) -> Option<std::fs::File> {
    #[cfg(unix)]
    let path: std::path::PathBuf = {
        use std::os::unix::ffi::OsStrExt;
        std::path::PathBuf::from(std::ffi::OsStr::from_bytes(path_bytes))
    };
    #[cfg(not(unix))]
    let path: std::path::PathBuf =
        std::path::PathBuf::from(String::from_utf8_lossy(path_bytes).into_owned());

    // fopen(path, "a") == O_WRONLY | O_CREAT | O_APPEND
    OpenOptions::new().append(true).create(true).open(path).ok()
}

/// `fprintf(log_file, "<prefix> %s\n", message)` guarded by `if (log_file)`.
unsafe fn write_entry(prefix: &[u8], message: *const c_char) {
    let mut guard = LOG_FILE.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(writer) = guard.as_mut() {
        let mut line = Vec::with_capacity(prefix.len() + 16);
        line.extend_from_slice(prefix);
        line.extend_from_slice(&c_str_bytes(message));
        line.push(b'\n');
        writer.write(&line);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn initialize_logger() -> c_int {
    let log_file_env = getenv_bytes("LOG_FILE");
    let log_file_path: Vec<u8> = match log_file_env {
        Some(v) => v,
        None => b"default.log".to_vec(),
    };

    // log_file = fopen(log_file_path, "a");  (assigned unconditionally, so a
    // failed re-initialisation also clears any previously open stream)
    let opened = open_append(&log_file_path).map(StdioStream::from_file);
    let failed = opened.is_none();
    {
        let mut guard = LOG_FILE.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(previous) = guard.take() {
            // The old FILE * is neither flushed nor closed by the C code.
            LEAKED
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .push(previous);
        }
        *guard = opened;
    }

    register_exit_hook();

    if failed {
        let mut msg = b"Failed to open log file: ".to_vec();
        msg.extend_from_slice(&log_file_path);
        msg.push(b'\n');
        print_stderr(&msg);
        return -1;
    }

    unsafe { write_entry(b"[INFO] ", c"Logger initialized.".as_ptr()) };
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn log_info(message: *const c_char) {
    write_entry(b"[INFO] ", message);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn log_warning(message: *const c_char) {
    write_entry(b"[WARNING] ", message);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn log_error(message: *const c_char) {
    write_entry(b"[ERROR] ", message);
}

#[unsafe(no_mangle)]
pub extern "C" fn finalize_logger() {
    let is_open = {
        let guard = LOG_FILE.lock().unwrap_or_else(|e| e.into_inner());
        guard.is_some()
    };
    if is_open {
        unsafe { write_entry(b"[INFO] ", c"Logger finalized.".as_ptr()) };
        // fclose(log_file): flush, then close.  (The C code leaves the dangling
        // `log_file` pointer in place; reproducing that use-after-free is not
        // possible in Rust, so the handle is cleared instead.)
        let stream = {
            let mut guard = LOG_FILE.lock().unwrap_or_else(|e| e.into_inner());
            guard.take()
        };
        drop(stream);
    }
}

/// Internal helpers so the other modules can log without going through the
/// exported `extern "C"` symbols.
pub(crate) fn log_info_str(message: &std::ffi::CStr) {
    unsafe { write_entry(b"[INFO] ", message.as_ptr()) };
}

pub(crate) fn log_warning_str(message: &std::ffi::CStr) {
    unsafe { write_entry(b"[WARNING] ", message.as_ptr()) };
}

pub(crate) fn log_error_str(message: &std::ffi::CStr) {
    unsafe { write_entry(b"[ERROR] ", message.as_ptr()) };
}
