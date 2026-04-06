#![allow(
    dead_code,
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    unused_assignments,
    unused_mut
)]
#![feature(raw_ref_op)]
#[allow(unused_imports)]
use ::driver;
extern "C" {
    static mut stdin: *mut _IO_FILE;
    static mut stdout: *mut _IO_FILE;
    static mut stderr: *mut _IO_FILE;
    fn fprintf(
        __stream: *mut FILE,
        __format: *const ::core::ffi::c_char,
        ...
    ) -> ::core::ffi::c_int;
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
    fn fgets(
        __s: *mut ::core::ffi::c_char,
        __n: ::core::ffi::c_int,
        __stream: *mut FILE,
    ) -> *mut ::core::ffi::c_char;
    fn strtol(
        __nptr: *const ::core::ffi::c_char,
        __endptr: *mut *mut ::core::ffi::c_char,
        __base: ::core::ffi::c_int,
    ) -> ::core::ffi::c_long;
    fn strcmp(
        __s1: *const ::core::ffi::c_char,
        __s2: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
    fn iv_init(v: *mut IntVec);
    fn iv_free(v: *mut IntVec);
    fn iv_push(v: *mut IntVec, x: ::core::ffi::c_int) -> bool;
    fn vm_init(vm: *mut VM);
    fn vm_free(vm: *mut VM);
    fn vm_print(fp: *mut FILE, label: *const ::core::ffi::c_char, vm: *const VM);
    fn run_engine(
        impl_id: ::core::ffi::c_int,
        code: *const ::core::ffi::c_int,
        n: size_t,
        vm: *mut VM,
    ) -> ::core::ffi::c_int;
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
pub struct VM {
    pub stack: IntVec,
    pub trace: IntVec,
    pub steps: ::core::ffi::c_int,
}
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
unsafe extern "C" fn usage(mut p: *const ::core::ffi::c_char) {
    fprintf(
        stderr as *mut FILE,
        b"Usage: %s [--stdin] [bytecodes...]\nBytecodes are integers forming a small VM program.\n\0"
            as *const u8 as *const ::core::ffi::c_char,
        p,
    );
}
unsafe extern "C" fn read_stdin(mut v: *mut IntVec) -> size_t {
    let mut buf: [::core::ffi::c_char; 4096] = [0; 4096];
    let mut count: size_t = 0 as size_t;
    while !fgets(
        &raw mut buf as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>() as ::core::ffi::c_int,
        stdin as *mut FILE,
    )
    .is_null()
    {
        let mut p: *mut ::core::ffi::c_char = &raw mut buf as *mut ::core::ffi::c_char;
        while *p != 0 {
            let mut q: *mut ::core::ffi::c_char = p;
            while *q as ::core::ffi::c_int != 0
                && *q as ::core::ffi::c_int != ' ' as i32
                && *q as ::core::ffi::c_int != '\t' as i32
                && *q as ::core::ffi::c_int != '\n' as i32
                && *q as ::core::ffi::c_int != '\r' as i32
            {
                q = q.offset(1);
            }
            let mut save: ::core::ffi::c_char = *q;
            *q = '\0' as i32 as ::core::ffi::c_char;
            if *p != 0 {
                let mut e: *mut ::core::ffi::c_char =
                    ::core::ptr::null_mut::<::core::ffi::c_char>();
                let mut t: ::core::ffi::c_long = strtol(p, &raw mut e, 10 as ::core::ffi::c_int);
                if !e.is_null() && *e as ::core::ffi::c_int == '\0' as i32 {
                    iv_push(v, t as ::core::ffi::c_int);
                    count = count.wrapping_add(1);
                }
            }
            *q = save;
            p = if *q as ::core::ffi::c_int != 0 {
                q.offset(1 as ::core::ffi::c_int as isize)
            } else {
                q
            };
        }
    }
    return count;
}
unsafe fn main_0(
    mut argc: ::core::ffi::c_int,
    mut argv: *mut *mut ::core::ffi::c_char,
) -> ::core::ffi::c_int {
    let mut use_stdin: bool = false_0 != 0;
    let mut code: IntVec = IntVec {
        data: ::core::ptr::null_mut::<::core::ffi::c_int>(),
        len: 0,
        cap: 0,
    };
    iv_init(&raw mut code);
    let mut i: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
    while i < argc {
        if strcmp(
            *argv.offset(i as isize),
            b"--help\0" as *const u8 as *const ::core::ffi::c_char,
        ) == 0
        {
            usage(*argv.offset(0 as ::core::ffi::c_int as isize));
            iv_free(&raw mut code);
            return 0 as ::core::ffi::c_int;
        } else if strcmp(
            *argv.offset(i as isize),
            b"--stdin\0" as *const u8 as *const ::core::ffi::c_char,
        ) == 0
        {
            use_stdin = true_0 != 0;
        } else {
            let mut e: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<::core::ffi::c_char>();
            let mut t: ::core::ffi::c_long = strtol(
                *argv.offset(i as isize),
                &raw mut e,
                10 as ::core::ffi::c_int,
            );
            if !e.is_null() && *e as ::core::ffi::c_int == '\0' as i32 {
                iv_push(&raw mut code, t as ::core::ffi::c_int);
            } else {
                fprintf(
                    stderr as *mut FILE,
                    b"skip '%s'\n\0" as *const u8 as *const ::core::ffi::c_char,
                    *argv.offset(i as isize),
                );
            }
        }
        i += 1;
    }
    if use_stdin {
        read_stdin(&raw mut code);
    }
    if code.len == 0 as size_t {
        fprintf(
            stderr as *mut FILE,
            b"no program\n\0" as *const u8 as *const ::core::ffi::c_char,
        );
        iv_free(&raw mut code);
        return 2 as ::core::ffi::c_int;
    }
    let mut vmA: VM = VM {
        stack: IntVec {
            data: ::core::ptr::null_mut::<::core::ffi::c_int>(),
            len: 0,
            cap: 0,
        },
        trace: IntVec {
            data: ::core::ptr::null_mut::<::core::ffi::c_int>(),
            len: 0,
            cap: 0,
        },
        steps: 0,
    };
    let mut vmB: VM = VM {
        stack: IntVec {
            data: ::core::ptr::null_mut::<::core::ffi::c_int>(),
            len: 0,
            cap: 0,
        },
        trace: IntVec {
            data: ::core::ptr::null_mut::<::core::ffi::c_int>(),
            len: 0,
            cap: 0,
        },
        steps: 0,
    };
    let mut vmE: VM = VM {
        stack: IntVec {
            data: ::core::ptr::null_mut::<::core::ffi::c_int>(),
            len: 0,
            cap: 0,
        },
        trace: IntVec {
            data: ::core::ptr::null_mut::<::core::ffi::c_int>(),
            len: 0,
            cap: 0,
        },
        steps: 0,
    };
    vm_init(&raw mut vmA);
    vm_init(&raw mut vmB);
    vm_init(&raw mut vmE);
    let mut rcA: ::core::ffi::c_int =
        run_engine(0 as ::core::ffi::c_int, code.data, code.len, &raw mut vmA);
    let mut rcB: ::core::ffi::c_int =
        run_engine(1 as ::core::ffi::c_int, code.data, code.len, &raw mut vmB);
    let mut rcE: ::core::ffi::c_int =
        run_engine(2 as ::core::ffi::c_int, code.data, code.len, &raw mut vmE);
    printf(
        b"RC:A=%d B=%d EXT=%d\n\0" as *const u8 as *const ::core::ffi::c_char,
        rcA,
        rcB,
        rcE,
    );
    vm_print(
        stdout as *mut FILE,
        b"A:\0" as *const u8 as *const ::core::ffi::c_char,
        &raw mut vmA,
    );
    vm_print(
        stdout as *mut FILE,
        b"B:\0" as *const u8 as *const ::core::ffi::c_char,
        &raw mut vmB,
    );
    vm_print(
        stdout as *mut FILE,
        b"EXT:\0" as *const u8 as *const ::core::ffi::c_char,
        &raw mut vmE,
    );
    vm_free(&raw mut vmA);
    vm_free(&raw mut vmB);
    vm_free(&raw mut vmE);
    iv_free(&raw mut code);
    return 0 as ::core::ffi::c_int;
}
pub const true_0: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const false_0: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
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
