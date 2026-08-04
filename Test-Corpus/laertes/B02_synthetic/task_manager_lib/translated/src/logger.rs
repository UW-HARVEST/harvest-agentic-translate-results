extern "C" {
    static mut stderr: *mut _IO_FILE;
    fn fclose(__stream: *mut FILE) -> libc::c_int;
    fn fopen(
        __filename: *const libc::c_char,
        __modes: *const libc::c_char,
    ) -> *mut FILE;
    fn fprintf(
        __stream: *mut FILE,
        __format: *const libc::c_char,
        ...
    ) -> libc::c_int;
    fn getenv(__name: *const libc::c_char) -> *mut libc::c_char;
}
pub use crate::src::driver::size_t;
pub use crate::src::driver::__off_t;
pub use crate::src::driver::__off64_t;
// #[derive(Copy, Clone)]

pub use crate::src::driver::_IO_FILE;
pub use crate::src::driver::_IO_lock_t;
// #[derive(Copy, Clone)]

pub use crate::src::driver::_IO_marker;
pub use crate::src::driver::FILE;
pub const NULL: *mut libc::c_void = std::ptr::null_mut::<libc::c_void>();
static mut log_file: *mut FILE = std::ptr::null::<FILE>() as *mut FILE;
#[no_mangle]
pub unsafe extern "C" fn initialize_logger() -> libc::c_int {
    let mut log_file_env: *const libc::c_char =
        getenv(b"LOG_FILE\0" as *const u8 as *const libc::c_char);
    let mut log_file_path: *const libc::c_char = if !log_file_env.is_null() {
        log_file_env
    } else {
        b"default.log\0" as *const u8 as *const libc::c_char
    };
    log_file = fopen(
        log_file_path,
        b"a\0" as *const u8 as *const libc::c_char,
    );
    if log_file.is_null() {
        fprintf(
            stderr as *mut FILE,
            b"Failed to open log file: %s\n\0" as *const u8 as *const libc::c_char,
            log_file_path,
        );
        return -(1 as libc::c_int);
    }
    log_info(b"Logger initialized.\0" as *const u8 as *const libc::c_char);
    return 0 as libc::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn log_info(mut message: *const libc::c_char) {
    if !log_file.is_null() {
        fprintf(
            log_file,
            b"[INFO] %s\n\0" as *const u8 as *const libc::c_char,
            message,
        );
    }
}
#[no_mangle]
pub unsafe extern "C" fn log_warning(mut message: *const libc::c_char) {
    if !log_file.is_null() {
        fprintf(
            log_file,
            b"[WARNING] %s\n\0" as *const u8 as *const libc::c_char,
            message,
        );
    }
}
#[no_mangle]
pub unsafe extern "C" fn log_error(mut message: *const libc::c_char) {
    if !log_file.is_null() {
        fprintf(
            log_file,
            b"[ERROR] %s\n\0" as *const u8 as *const libc::c_char,
            message,
        );
    }
}
#[no_mangle]
pub unsafe extern "C" fn finalize_logger() {
    if !log_file.is_null() {
        log_info(b"Logger finalized.\0" as *const u8 as *const libc::c_char);
        fclose(log_file);
    }
}
