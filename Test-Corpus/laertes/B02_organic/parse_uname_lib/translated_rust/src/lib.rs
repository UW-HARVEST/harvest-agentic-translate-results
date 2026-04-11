use ::c2rust_bitfields;
extern "C" {
    pub type re_dfa_t;
    fn malloc(__size: size_t) -> *mut libc::c_void;
    fn free(__ptr: *mut libc::c_void);
    fn strdup(__s: *const libc::c_char) -> *mut libc::c_char;
    fn strstr(
        __haystack: *const libc::c_char,
        __needle: *const libc::c_char,
    ) -> *mut libc::c_char;
    fn strlen(__s: *const libc::c_char) -> size_t;
    fn regcomp(
        __preg: *mut regex_t,
        __pattern: *const libc::c_char,
        __cflags: libc::c_int,
    ) -> libc::c_int;
    fn regexec(
        __preg: *const regex_t,
        __String: *const libc::c_char,
        __nmatch: size_t,
        __pmatch: *mut regmatch_t,
        __eflags: libc::c_int,
    ) -> libc::c_int;
    fn regfree(__preg: *mut regex_t);
    static mut stderr: *mut _IO_FILE;
    fn fprintf(
        __stream: *mut FILE,
        __format: *const libc::c_char,
        ...
    ) -> libc::c_int;
    fn snprintf(
        __s: *mut libc::c_char,
        __maxlen: size_t,
        __format: *const libc::c_char,
        ...
    ) -> libc::c_int;
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct os_data {
    pub os_name: *mut libc::c_char,
    pub os_version: *mut libc::c_char,
    pub os_major: *mut libc::c_char,
    pub os_minor: *mut libc::c_char,
    pub os_codename: *mut libc::c_char,
    pub os_platform: *mut libc::c_char,
    pub os_build: *mut libc::c_char,
    pub os_uname: *mut libc::c_char,
    pub os_arch: *mut libc::c_char,
}
pub type size_t = usize;
pub type regoff_t = libc::c_int;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct regmatch_t {
    pub rm_so: regoff_t,
    pub rm_eo: regoff_t,
}
pub type regex_t = re_pattern_buffer;
#[derive(Copy, Clone, BitfieldStruct)]
#[repr(C)]
pub struct re_pattern_buffer {
    pub __buffer: *mut re_dfa_t,
    pub __allocated: __re_long_size_t,
    pub __used: __re_long_size_t,
    pub __syntax: reg_syntax_t,
    pub __fastmap: *mut libc::c_char,
    pub __translate: *mut libc::c_uchar,
    pub re_nsub: size_t,
    #[bitfield(name = "__can_be_null", ty = "libc::c_uint", bits = "0..=0")]
    #[bitfield(name = "__regs_allocated", ty = "libc::c_uint", bits = "1..=2")]
    #[bitfield(
        name = "__fastmap_accurate",
        ty = "libc::c_uint",
        bits = "3..=3"
    )]
    #[bitfield(name = "__no_sub", ty = "libc::c_uint", bits = "4..=4")]
    #[bitfield(name = "__not_bol", ty = "libc::c_uint", bits = "5..=5")]
    #[bitfield(name = "__not_eol", ty = "libc::c_uint", bits = "6..=6")]
    #[bitfield(name = "__newline_anchor", ty = "libc::c_uint", bits = "7..=7")]
    pub __can_be_null___regs_allocated___fastmap_accurate___no_sub___not_bol___not_eol___newline_anchor:
        [u8; 1],
    #[bitfield(padding)]
    pub c2rust_padding: [u8; 7],
}
pub type reg_syntax_t = libc::c_ulong;
pub type __re_long_size_t = libc::c_ulong;
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
pub type __off64_t = libc::c_long;
pub type _IO_lock_t = ();
pub type __off_t = libc::c_long;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _IO_marker {
    pub _next: *mut _IO_marker,
    pub _sbuf: *mut _IO_FILE,
    pub _pos: libc::c_int,
}
pub type FILE = _IO_FILE;
pub const NULL: *mut libc::c_void = std::ptr::null_mut::<libc::c_void>();
pub const REG_EXTENDED: libc::c_int = 1 as libc::c_int;
#[no_mangle]
pub unsafe extern "C" fn get_os_arch(
    mut os_header: *mut libc::c_char,
) -> *mut libc::c_char {
    let mut ARCHS: [*const libc::c_char; 13] = [
        b"x86_64\0" as *const u8 as *const libc::c_char,
        b"i386\0" as *const u8 as *const libc::c_char,
        b"i686\0" as *const u8 as *const libc::c_char,
        b"sparc\0" as *const u8 as *const libc::c_char,
        b"amd64\0" as *const u8 as *const libc::c_char,
        b"i86pc\0" as *const u8 as *const libc::c_char,
        b"ia64\0" as *const u8 as *const libc::c_char,
        b"AIX\0" as *const u8 as *const libc::c_char,
        b"armv6\0" as *const u8 as *const libc::c_char,
        b"armv7\0" as *const u8 as *const libc::c_char,
        b"aarch64\0" as *const u8 as *const libc::c_char,
        b"arm64\0" as *const u8 as *const libc::c_char,
        std::ptr::null::<libc::c_char>(),
    ];
    let mut os_arch: *mut libc::c_char = std::ptr::null_mut::<libc::c_char>();
    let mut i: libc::c_int = 0;
    i = 0 as libc::c_int;
    while !ARCHS[i as usize].is_null() {
        if !strstr(os_header, ARCHS[i as usize]).is_null() {
            os_arch = strdup(ARCHS[i as usize]);
            break;
        } else {
            i += 1;
        }
    }
    return os_arch;
}
#[no_mangle]
pub unsafe extern "C" fn w_regexec(
    mut pattern: *const libc::c_char,
    mut string: *const libc::c_char,
    mut nmatch: size_t,
    mut pmatch: *mut regmatch_t,
) -> libc::c_int {
    let mut regex: regex_t = regex_t {
        __buffer: std::ptr::null_mut::<re_dfa_t>(),
        __allocated: 0,
        __used: 0,
        __syntax: 0,
        __fastmap: std::ptr::null_mut::<libc::c_char>(),
        __translate: std::ptr::null_mut::<libc::c_uchar>(),
        re_nsub: 0,
        __can_be_null___regs_allocated___fastmap_accurate___no_sub___not_bol___not_eol___newline_anchor: [0; 1],
        c2rust_padding: [0; 7],
    };
    let mut result: libc::c_int = 0;
    if !(!pattern.is_null() && !string.is_null()) {
        return 0 as libc::c_int;
    }
    if regcomp(&raw mut regex, pattern, REG_EXTENDED) != 0 {
        fprintf(
            stderr as *mut FILE,
            b"Couldn't compile regular expression '%s'\n\0" as *const u8
                as *const libc::c_char,
            pattern,
        );
        return 0 as libc::c_int;
    }
    result = regexec(
        &raw mut regex,
        string,
        nmatch,
        pmatch as *mut regmatch_t,
        0 as libc::c_int,
    );
    regfree(&raw mut regex);
    return (result == 0) as libc::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn parse_uname_string(
    mut uname: *mut libc::c_char,
    mut osd: *mut os_data,
) {
    let mut str_tmp: *mut libc::c_char = std::ptr::null_mut::<libc::c_char>();
    let mut match_0: [regmatch_t; 2] = [
        regmatch_t {
            rm_so: 0 as regoff_t,
            rm_eo: 0,
        },
        regmatch_t { rm_so: 0, rm_eo: 0 },
    ];
    let mut match_size: libc::c_int = 0 as libc::c_int;
    if osd.is_null() {
        return;
    }
    str_tmp = strstr(
        uname,
        b" [Ver: \0" as *const u8 as *const libc::c_char,
    );
    if !str_tmp.is_null() {
        *str_tmp = '\0' as i32 as libc::c_char;
        str_tmp = str_tmp.offset(7 as libc::c_int as isize);
        (*osd).os_name = strdup(uname);
        *str_tmp
            .offset(strlen(str_tmp) as isize)
            .offset(-(1 as libc::c_int as isize)) = '\0' as i32 as libc::c_char;
        if w_regexec(
            b"^([0-9]+)\\.*\0" as *const u8 as *const libc::c_char,
            str_tmp,
            2 as size_t,
            &raw mut match_0 as *mut regmatch_t,
        ) != 0
        {
            match_size = (match_0[1 as libc::c_int as usize].rm_eo
                - match_0[1 as libc::c_int as usize].rm_so)
                as libc::c_int;
            (*osd).os_major = malloc((match_size + 1 as libc::c_int) as size_t)
                as *mut libc::c_char;
            snprintf(
                (*osd).os_major,
                (match_size + 1 as libc::c_int) as size_t,
                b"%.*s\0" as *const u8 as *const libc::c_char,
                match_size,
                str_tmp.offset(match_0[1 as libc::c_int as usize].rm_so as isize),
            );
        }
        if w_regexec(
            b"^[0-9]+\\.([0-9]+)\\.*\0" as *const u8 as *const libc::c_char,
            str_tmp,
            2 as size_t,
            &raw mut match_0 as *mut regmatch_t,
        ) != 0
        {
            match_size = (match_0[1 as libc::c_int as usize].rm_eo
                - match_0[1 as libc::c_int as usize].rm_so)
                as libc::c_int;
            (*osd).os_minor = malloc((match_size + 1 as libc::c_int) as size_t)
                as *mut libc::c_char;
            snprintf(
                (*osd).os_minor,
                (match_size + 1 as libc::c_int) as size_t,
                b"%.*s\0" as *const u8 as *const libc::c_char,
                match_size,
                str_tmp.offset(match_0[1 as libc::c_int as usize].rm_so as isize),
            );
        }
        if w_regexec(
            b"^[0-9]+\\.[0-9]+\\.([0-9]+(\\.[0-9]+)*)\\.*\0" as *const u8
                as *const libc::c_char,
            str_tmp,
            2 as size_t,
            &raw mut match_0 as *mut regmatch_t,
        ) != 0
        {
            match_size = (match_0[1 as libc::c_int as usize].rm_eo
                - match_0[1 as libc::c_int as usize].rm_so)
                as libc::c_int;
            (*osd).os_build = malloc((match_size + 1 as libc::c_int) as size_t)
                as *mut libc::c_char;
            snprintf(
                (*osd).os_build,
                (match_size + 1 as libc::c_int) as size_t,
                b"%.*s\0" as *const u8 as *const libc::c_char,
                match_size,
                str_tmp.offset(match_0[1 as libc::c_int as usize].rm_so as isize),
            );
        }
        (*osd).os_version = strdup(str_tmp);
        (*osd).os_platform = strdup(b"windows\0" as *const u8 as *const libc::c_char);
    } else {
        str_tmp = strstr(uname, b" [\0" as *const u8 as *const libc::c_char);
        if !str_tmp.is_null() {
            *str_tmp = '\0' as i32 as libc::c_char;
            str_tmp = str_tmp.offset(2 as libc::c_int as isize);
            (*osd).os_name = strdup(str_tmp);
            str_tmp = strstr(
                (*osd).os_name,
                b": \0" as *const u8 as *const libc::c_char,
            );
            if !str_tmp.is_null() {
                *str_tmp = '\0' as i32 as libc::c_char;
                str_tmp = str_tmp.offset(2 as libc::c_int as isize);
                (*osd).os_version = strdup(str_tmp);
                *(*osd)
                    .os_version
                    .offset(strlen((*osd).os_version) as isize)
                    .offset(-(1 as libc::c_int as isize)) =
                    '\0' as i32 as libc::c_char;
                str_tmp = strstr(
                    (*osd).os_version,
                    b" (\0" as *const u8 as *const libc::c_char,
                );
                if !str_tmp.is_null() {
                    *str_tmp = '\0' as i32 as libc::c_char;
                    str_tmp = str_tmp.offset(2 as libc::c_int as isize);
                    (*osd).os_codename = strdup(str_tmp);
                    *(*osd)
                        .os_codename
                        .offset(strlen((*osd).os_codename) as isize)
                        .offset(-(1 as libc::c_int as isize)) =
                        '\0' as i32 as libc::c_char;
                }
                if w_regexec(
                    b"^([0-9]+)\\.*\0" as *const u8 as *const libc::c_char,
                    (*osd).os_version,
                    2 as size_t,
                    &raw mut match_0 as *mut regmatch_t,
                ) != 0
                {
                    match_size = (match_0[1 as libc::c_int as usize].rm_eo
                        - match_0[1 as libc::c_int as usize].rm_so)
                        as libc::c_int;
                    (*osd).os_major = malloc((match_size + 1 as libc::c_int) as size_t)
                        as *mut libc::c_char;
                    snprintf(
                        (*osd).os_major,
                        (match_size + 1 as libc::c_int) as size_t,
                        b"%.*s\0" as *const u8 as *const libc::c_char,
                        match_size,
                        (*osd)
                            .os_version
                            .offset(match_0[1 as libc::c_int as usize].rm_so as isize),
                    );
                }
                if w_regexec(
                    b"^[0-9]+\\.([0-9]+)\\.*\0" as *const u8 as *const libc::c_char,
                    (*osd).os_version,
                    2 as size_t,
                    &raw mut match_0 as *mut regmatch_t,
                ) != 0
                {
                    match_size = (match_0[1 as libc::c_int as usize].rm_eo
                        - match_0[1 as libc::c_int as usize].rm_so)
                        as libc::c_int;
                    (*osd).os_minor = malloc((match_size + 1 as libc::c_int) as size_t)
                        as *mut libc::c_char;
                    snprintf(
                        (*osd).os_minor,
                        (match_size + 1 as libc::c_int) as size_t,
                        b"%.*s\0" as *const u8 as *const libc::c_char,
                        match_size,
                        (*osd)
                            .os_version
                            .offset(match_0[1 as libc::c_int as usize].rm_so as isize),
                    );
                }
            } else {
                *(*osd)
                    .os_name
                    .offset(strlen((*osd).os_name) as isize)
                    .offset(-(1 as libc::c_int as isize)) =
                    '\0' as i32 as libc::c_char;
            }
            str_tmp = strstr(
                (*osd).os_name,
                b"|\0" as *const u8 as *const libc::c_char,
            );
            if !str_tmp.is_null() {
                *str_tmp = '\0' as i32 as libc::c_char;
                str_tmp = str_tmp.offset(1);
                (*osd).os_platform = strdup(str_tmp);
            }
        }
        str_tmp = get_os_arch(uname);
        if !str_tmp.is_null() {
            (*osd).os_arch = strdup(str_tmp);
            free(str_tmp as *mut libc::c_void);
        }
    };
}
