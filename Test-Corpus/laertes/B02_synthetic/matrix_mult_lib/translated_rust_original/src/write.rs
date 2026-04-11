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
    fn __errno_location() -> *mut libc::c_int;
    fn strerror(__errnum: libc::c_int) -> *mut libc::c_char;
}
pub use crate::src::matrix::size_t;
pub use crate::src::matrix::__off_t;
pub use crate::src::matrix::__off64_t;
// #[derive(Copy, Clone)]

pub use crate::src::matrix::_IO_FILE;
pub use crate::src::matrix::_IO_lock_t;
// #[derive(Copy, Clone)]

pub use crate::src::matrix::_IO_marker;
pub use crate::src::matrix::FILE;
pub const NULL: *mut libc::c_void = std::ptr::null_mut::<libc::c_void>();
pub const EINVAL: libc::c_int = 22 as libc::c_int;
#[no_mangle]
pub unsafe extern "C" fn write_to_file(
    mut filename: *const libc::c_char,
    mut content: *const libc::c_char,
) -> libc::c_int {
    if content.is_null() {
        fprintf(
            stderr as *mut FILE,
            b"Error: Content is NULL.\n\0" as *const u8 as *const libc::c_char,
        );
        return EINVAL;
    }
    let mut file: *mut FILE = fopen(filename, b"w\0" as *const u8 as *const libc::c_char);
    if file.is_null() {
        fprintf(
            stderr as *mut FILE,
            b"Error opening file '%s': %s\n\0" as *const u8 as *const libc::c_char,
            filename,
            strerror(*__errno_location()),
        );
        return *__errno_location();
    }
    if fprintf(
        file,
        b"%s\0" as *const u8 as *const libc::c_char,
        content,
    ) < 0 as libc::c_int
    {
        fprintf(
            stderr as *mut FILE,
            b"Error writing to file '%s': %s\n\0" as *const u8 as *const libc::c_char,
            filename,
            strerror(*__errno_location()),
        );
        fclose(file);
        return *__errno_location();
    }
    if fclose(file) != 0 as libc::c_int {
        fprintf(
            stderr as *mut FILE,
            b"Error closing file '%s': %s\n\0" as *const u8 as *const libc::c_char,
            filename,
            strerror(*__errno_location()),
        );
        return *__errno_location();
    }
    return 0 as libc::c_int;
}
