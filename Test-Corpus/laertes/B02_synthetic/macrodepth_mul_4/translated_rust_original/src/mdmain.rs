extern "C" {
    static mut stderr: *mut _IO_FILE;
    fn fprintf(
        __stream: *mut FILE,
        __format: *const libc::c_char,
        ...
    ) -> libc::c_int;
    fn printf(__format: *const libc::c_char, ...) -> libc::c_int;
    fn atoi(__nptr: *const libc::c_char) -> libc::c_int;
    fn op_add(a: libc::c_int, b: libc::c_int) -> libc::c_int;
    static mut G_OP:
        Option<unsafe extern "C" fn(libc::c_int, libc::c_int) -> libc::c_int>;
    static mut G_OP_NAME: *const libc::c_char;
    fn helper_call(a: libc::c_int, b: libc::c_int) -> libc::c_int;
    fn helper_ptr(a: libc::c_int, b: libc::c_int) -> libc::c_int;
    fn use_generated(n: libc::c_int) -> libc::c_int;
}
pub type size_t = usize;
pub type __off_t = libc::c_long;
pub type __off64_t = libc::c_long;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _IO_FILE {
    pub _flags: libc::c_int,
    pub _IO_read_ptr: *mut libc::c_char,
    pub _IO_read_end: *mut libc::c_char,
    pub _IO_read_base: *mut libc::c_char,
    pub _IO_write_base: *mut libc::c_char,
    pub _IO_write_ptr: *mut libc::c_char,
    pub _IO_write_end: *mut libc::c_char,
    pub _IO_buf_base: *mut libc::c_char,
    pub _IO_buf_end: *mut libc::c_char,
    pub _IO_save_base: *mut libc::c_char,
    pub _IO_backup_base: *mut libc::c_char,
    pub _IO_save_end: *mut libc::c_char,
    pub _markers: *mut _IO_marker,
    pub _chain: *mut _IO_FILE,
    pub _fileno: libc::c_int,
    pub _flags2: libc::c_int,
    pub _old_offset: __off_t,
    pub _cur_column: libc::c_ushort,
    pub _vtable_offset: libc::c_schar,
    pub _shortbuf: [libc::c_char; 1],
    pub _lock: *mut libc::c_void,
    pub _offset: __off64_t,
    pub __pad1: *mut libc::c_void,
    pub __pad2: *mut libc::c_void,
    pub __pad3: *mut libc::c_void,
    pub __pad4: *mut libc::c_void,
    pub __pad5: size_t,
    pub _mode: libc::c_int,
    pub _unused2: [libc::c_char; 20],
}
pub type _IO_lock_t = ();
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _IO_marker {
    pub _next: *mut _IO_marker,
    pub _sbuf: *mut _IO_FILE,
    pub _pos: libc::c_int,
}
pub type FILE = _IO_FILE;
pub const INIT_add: libc::c_int = 0 as libc::c_int;
unsafe fn main_0(
    mut argc: libc::c_int,
    mut argv: *mut *mut libc::c_char,
) -> libc::c_int {
    if argc < 3 as libc::c_int {
        fprintf(
            stderr as *mut FILE,
            b"usage: %s A B\n\0" as *const u8 as *const libc::c_char,
            *argv.offset(0 as libc::c_int as isize),
        );
        return 2 as libc::c_int;
    }
    let mut a: libc::c_int = atoi(*argv.offset(1 as libc::c_int as isize));
    let mut b: libc::c_int = atoi(*argv.offset(2 as libc::c_int as isize));
    let mut r_call: libc::c_int = op_add(a, b);
    let mut acc: libc::c_int = INIT_add;
    acc += 0 as libc::c_int;
    acc += 1 as libc::c_int;
    acc += 2 as libc::c_int;
    acc += 3 as libc::c_int;
    acc += 4 as libc::c_int;
    let mut x1: libc::c_int = helper_call(a, b);
    let mut x2: libc::c_int = helper_ptr(a, b);
    let mut x3: libc::c_int = use_generated(REPEAT);
    let mut g: libc::c_int = G_OP.expect("non-null function pointer")(a, b);
    printf(
        b"op=%s call=%d acc=%d g.call=%d\n\0" as *const u8 as *const libc::c_char,
        G_OP_NAME,
        r_call,
        acc,
        g,
    );
    printf(
        b"summary=%d\n\0" as *const u8 as *const libc::c_char,
        r_call + acc + x1 + x2 + x3 + g,
    );
    return 0 as libc::c_int;
}
pub const REPEAT: libc::c_int = 5 as libc::c_int;
pub fn main() {
    let mut args_strings: Vec<Vec<u8>> = ::std::env::args()
        .map(|arg| {
            ::std::ffi::CString::new(arg)
                .expect("Failed to convert argument into CString.")
                .into_bytes_with_nul()
        })
        .collect();
    let mut args_ptrs: Vec<*mut libc::c_char> = args_strings
        .iter_mut()
        .map(|arg| arg.as_mut_ptr() as *mut libc::c_char)
        .chain(::core::iter::once(std::ptr::null_mut()))
        .collect();
    unsafe {
        ::std::process::exit(main_0(
            (args_ptrs.len() - 1) as libc::c_int,
            args_ptrs.as_mut_ptr() as *mut *mut libc::c_char,
        ) as i32)
    }
}
