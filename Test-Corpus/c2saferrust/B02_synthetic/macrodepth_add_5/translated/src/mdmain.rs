extern "C" {
    static mut stderr: *mut _IO_FILE;
    fn fprintf(
        __stream: *mut FILE,
        __format: *const ::core::ffi::c_char,
        ...
    ) -> ::core::ffi::c_int;
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
    fn atoi(__nptr: *const ::core::ffi::c_char) -> ::core::ffi::c_int;
    fn op_add(a: ::core::ffi::c_int, b: ::core::ffi::c_int) -> ::core::ffi::c_int;
    static mut G_OP:
        Option<unsafe extern "C" fn(::core::ffi::c_int, ::core::ffi::c_int) -> ::core::ffi::c_int>;
    static mut G_OP_NAME: *const ::core::ffi::c_char;
    fn helper_call(a: ::core::ffi::c_int, b: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn helper_ptr(a: ::core::ffi::c_int, b: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn use_generated(n: ::core::ffi::c_int) -> ::core::ffi::c_int;
}
pub type size_t = usize;
pub type __off_t = ::core::ffi::c_long;
pub type __off64_t = ::core::ffi::c_long;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _IO_FILE {
    pub _flags: ::core::ffi::c_int,
    pub _IO_read_ptr: *mut ::core::ffi::c_char,
    pub _IO_read_end: *mut ::core::ffi::c_char,
    pub _IO_read_base: *mut ::core::ffi::c_char,
    pub _IO_write_base: *mut ::core::ffi::c_char,
    pub _IO_write_ptr: *mut ::core::ffi::c_char,
    pub _IO_write_end: *mut ::core::ffi::c_char,
    pub _IO_buf_base: *mut ::core::ffi::c_char,
    pub _IO_buf_end: *mut ::core::ffi::c_char,
    pub _IO_save_base: *mut ::core::ffi::c_char,
    pub _IO_backup_base: *mut ::core::ffi::c_char,
    pub _IO_save_end: *mut ::core::ffi::c_char,
    pub _markers: *mut _IO_marker,
    pub _chain: *mut _IO_FILE,
    pub _fileno: ::core::ffi::c_int,
    pub _flags2: ::core::ffi::c_int,
    pub _old_offset: __off_t,
    pub _cur_column: ::core::ffi::c_ushort,
    pub _vtable_offset: ::core::ffi::c_schar,
    pub _shortbuf: [::core::ffi::c_char; 1],
    pub _lock: *mut ::core::ffi::c_void,
    pub _offset: __off64_t,
    pub __pad1: *mut ::core::ffi::c_void,
    pub __pad2: *mut ::core::ffi::c_void,
    pub __pad3: *mut ::core::ffi::c_void,
    pub __pad4: *mut ::core::ffi::c_void,
    pub __pad5: size_t,
    pub _mode: ::core::ffi::c_int,
    pub _unused2: [::core::ffi::c_char; 20],
}
pub type _IO_lock_t = ();
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _IO_marker {
    pub _next: *mut _IO_marker,
    pub _sbuf: *mut _IO_FILE,
    pub _pos: ::core::ffi::c_int,
}
pub type FILE = _IO_FILE;
pub const INIT_add: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
unsafe fn main_0(
    mut argc: ::core::ffi::c_int,
    mut argv: *mut *mut ::core::ffi::c_char,
) -> ::core::ffi::c_int {
    if argc < 3 as ::core::ffi::c_int {
        fprintf(
            stderr as *mut FILE,
            b"usage: %s A B\n\0" as *const u8 as *const ::core::ffi::c_char,
            *argv.offset(0 as ::core::ffi::c_int as isize),
        );
        return 2 as ::core::ffi::c_int;
    }
    let mut a: ::core::ffi::c_int = atoi(*argv.offset(1 as ::core::ffi::c_int as isize));
    let mut b: ::core::ffi::c_int = atoi(*argv.offset(2 as ::core::ffi::c_int as isize));
    let mut r_call: ::core::ffi::c_int = op_add(a, b);
    let mut acc: ::core::ffi::c_int = INIT_add;
    acc += 0 as ::core::ffi::c_int;
    acc += 1 as ::core::ffi::c_int;
    acc += 2 as ::core::ffi::c_int;
    acc += 3 as ::core::ffi::c_int;
    acc += 4 as ::core::ffi::c_int;
    let mut x1: ::core::ffi::c_int = helper_call(a, b);
    let mut x2: ::core::ffi::c_int = helper_ptr(a, b);
    let mut x3: ::core::ffi::c_int = use_generated(REPEAT);
    let mut g: ::core::ffi::c_int = G_OP.expect("non-null function pointer")(a, b);
    printf(
        b"op=%s call=%d acc=%d g.call=%d\n\0" as *const u8 as *const ::core::ffi::c_char,
        G_OP_NAME,
        r_call,
        acc,
        g,
    );
    printf(
        b"summary=%d\n\0" as *const u8 as *const ::core::ffi::c_char,
        r_call + acc + x1 + x2 + x3 + g,
    );
    return 0 as ::core::ffi::c_int;
}
pub const REPEAT: ::core::ffi::c_int = 5 as ::core::ffi::c_int;
pub fn main() {
    let mut args_strings: Vec<Vec<u8>> = ::std::env::args()
        .map(|arg| {
            ::std::ffi::CString::new(arg)
                .expect("Failed to convert argument into CString.")
                .into_bytes_with_nul()
        })
        .collect();
    let mut args_ptrs: Vec<*mut ::core::ffi::c_char> = args_strings
        .iter_mut()
        .map(|arg| arg.as_mut_ptr() as *mut ::core::ffi::c_char)
        .chain(::core::iter::once(::core::ptr::null_mut()))
        .collect();
    unsafe {
        ::std::process::exit(main_0(
            (args_ptrs.len() - 1) as ::core::ffi::c_int,
            args_ptrs.as_mut_ptr() as *mut *mut ::core::ffi::c_char,
        ) as i32)
    }
}
