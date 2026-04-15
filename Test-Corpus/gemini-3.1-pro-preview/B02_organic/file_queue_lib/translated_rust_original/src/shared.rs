use libc::{c_char, c_void};

pub unsafe fn os_calloc(num: usize, size: usize) -> *mut c_void {
    let out = libc::calloc(num, size);
    if out.is_null() {
        libc::fprintf(libc::stderr, b"Memory allocation failed in os_calloc\0".as_ptr() as *const c_char);
        libc::exit(libc::EXIT_FAILURE);
    }
    out
}

pub unsafe fn os_realloc(ptr: *mut c_void, new_size: usize) -> *mut c_void {
    let out = libc::realloc(ptr, new_size);
    if out.is_null() {
        libc::fprintf(libc::stderr, b"Memory allocation failed in os_realloc\0".as_ptr() as *const c_char);
        libc::exit(libc::EXIT_FAILURE);
    }
    out
}

pub unsafe fn os_strdup(str: *const c_char) -> *mut c_char {
    if str.is_null() {
        libc::fprintf(libc::stderr, b"NULL string passed to os_strdup\0".as_ptr() as *const c_char);
        libc::exit(libc::EXIT_FAILURE);
    }
    let dup = libc::strdup(str);
    if dup.is_null() {
        libc::fprintf(libc::stderr, b"Memory allocation failed in os_strdup\0".as_ptr() as *const c_char);
        libc::exit(libc::EXIT_FAILURE);
    }
    dup
}

#[macro_export]
macro_rules! os_free {
    ($x:expr) => {
        if !$x.is_null() {
            libc::free($x as *mut libc::c_void);
            $x = std::ptr::null_mut();
        }
    };
}

#[macro_export]
macro_rules! os_clearnl {
    ($x:expr, $p:expr) => {
        $p = libc::strrchr($x as *const libc::c_char, b'\n' as libc::c_int);
        if !$p.is_null() {
            *$p = 0;
        }
    };
}
