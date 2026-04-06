extern "C" {
    fn fprintf(
        __stream: *mut FILE,
        __format: *const ::core::ffi::c_char,
        ...
    ) -> ::core::ffi::c_int;
    fn fputc(__c: ::core::ffi::c_int, __stream: *mut FILE) -> ::core::ffi::c_int;
    fn realloc(__ptr: *mut ::core::ffi::c_void, __size: size_t) -> *mut ::core::ffi::c_void;
    fn free(__ptr: *mut ::core::ffi::c_void);
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
#[derive(Copy, Clone)]
#[repr(C)]
pub struct IntVec {
    pub data: *mut ::core::ffi::c_int,
    pub len: size_t,
    pub cap: size_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct Program {
    pub code: *const ::core::ffi::c_int,
    pub n: size_t,
    pub ip: size_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct VM {
    pub stack: IntVec,
    pub trace: IntVec,
    pub steps: ::core::ffi::c_int,
}
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const true_0: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const false_0: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const SIZE_MAX: ::core::ffi::c_ulong = 18446744073709551615 as ::core::ffi::c_ulong;
#[no_mangle]
pub unsafe extern "C" fn iv_init(mut v: *mut IntVec) {
    (*v).data = ::core::ptr::null_mut::<::core::ffi::c_int>();
    (*v).cap = 0 as size_t;
    (*v).len = (*v).cap;
}
#[no_mangle]
pub unsafe extern "C" fn iv_free(mut v: *mut IntVec) {
    free((*v).data as *mut ::core::ffi::c_void);
    (*v).data = ::core::ptr::null_mut::<::core::ffi::c_int>();
    (*v).cap = 0 as size_t;
    (*v).len = (*v).cap;
}
#[no_mangle]
pub unsafe extern "C" fn iv_reserve(mut v: *mut IntVec, mut need: size_t) -> bool {
    if need <= (*v).cap {
        return true_0 != 0;
    }
    let mut nc: size_t = if (*v).cap != 0 { (*v).cap } else { 8 as size_t };
    while nc < need {
        if nc > (SIZE_MAX as size_t).wrapping_div(2 as size_t) {
            return false_0 != 0;
        }
        nc = (nc as ::core::ffi::c_ulong).wrapping_mul(2 as ::core::ffi::c_ulong) as size_t
            as size_t;
    }
    let mut p: *mut ::core::ffi::c_int = realloc(
        (*v).data as *mut ::core::ffi::c_void,
        nc.wrapping_mul(::core::mem::size_of::<::core::ffi::c_int>() as size_t),
    ) as *mut ::core::ffi::c_int;
    if p.is_null() {
        return false_0 != 0;
    }
    (*v).data = p;
    (*v).cap = nc;
    return true_0 != 0;
}
#[no_mangle]
pub unsafe extern "C" fn iv_push(mut v: *mut IntVec, mut x: ::core::ffi::c_int) -> bool {
    if (*v).len == (*v).cap
        && !iv_reserve(
            v,
            (if (*v).cap != 0 {
                (*v).cap.wrapping_mul(2 as size_t)
            } else {
                8 as size_t
            }),
        )
    {
        return false_0 != 0;
    }
    let fresh0 = (*v).len;
    (*v).len = (*v).len.wrapping_add(1);
    *(*v).data.offset(fresh0 as isize) = x;
    return true_0 != 0;
}
#[no_mangle]
pub unsafe extern "C" fn iv_pop(mut v: *mut IntVec, mut out: *mut ::core::ffi::c_int) -> bool {
    if (*v).len == 0 {
        return false_0 != 0;
    }
    if !out.is_null() {
        *out = *(*v)
            .data
            .offset((*v).len.wrapping_sub(1 as size_t) as isize);
    }
    (*v).len = (*v).len.wrapping_sub(1);
    return true_0 != 0;
}
#[no_mangle]
pub unsafe extern "C" fn iv_peek(
    mut v: *const IntVec,
    mut def: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    return if (*v).len != 0 {
        *(*v)
            .data
            .offset((*v).len.wrapping_sub(1 as size_t) as isize)
    } else {
        def
    };
}
#[no_mangle]
pub unsafe extern "C" fn prog_init(
    mut p: *mut Program,
    mut code: *const ::core::ffi::c_int,
    mut n: size_t,
) {
    (*p).code = code;
    (*p).n = n;
    (*p).ip = 0 as size_t;
}
#[no_mangle]
pub unsafe extern "C" fn prog_fetch(mut p: *mut Program, mut out: *mut ::core::ffi::c_int) -> bool {
    if (*p).ip >= (*p).n {
        return false_0 != 0;
    }
    let fresh1 = (*p).ip;
    (*p).ip = (*p).ip.wrapping_add(1);
    *out = *(*p).code.offset(fresh1 as isize);
    return true_0 != 0;
}
#[no_mangle]
pub unsafe extern "C" fn vm_init(mut vm: *mut VM) {
    iv_init(&raw mut (*vm).stack);
    iv_init(&raw mut (*vm).trace);
    (*vm).steps = 0 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn vm_free(mut vm: *mut VM) {
    iv_free(&raw mut (*vm).stack);
    iv_free(&raw mut (*vm).trace);
    (*vm).steps = 0 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn vm_trace(mut vm: *mut VM, mut t: ::core::ffi::c_int) {
    iv_push(&raw mut (*vm).trace, t);
}
#[no_mangle]
pub unsafe extern "C" fn vm_print(
    mut fp: *mut FILE,
    mut label: *const ::core::ffi::c_char,
    mut vm: *const VM,
) {
    fprintf(
        fp,
        b"%sSTACK_TOP=%d STEPS=%d TRACE=\0" as *const u8 as *const ::core::ffi::c_char,
        label,
        iv_peek(&raw const (*vm).stack, -(777 as ::core::ffi::c_int)),
        (*vm).steps,
    );
    let mut i: size_t = 0 as size_t;
    while i < (*vm).trace.len {
        fputc(
            ::core::mem::transmute::<[u8; 27], [::core::ffi::c_char; 27]>(
                *b"abcdefghijklmnopqrstuvwxyz\0",
            )[(*(*vm).trace.data.offset(i as isize) & 25 as ::core::ffi::c_int) as usize]
                as ::core::ffi::c_int,
            fp,
        );
        i = i.wrapping_add(1);
    }
    fputc('\n' as i32, fp);
}
