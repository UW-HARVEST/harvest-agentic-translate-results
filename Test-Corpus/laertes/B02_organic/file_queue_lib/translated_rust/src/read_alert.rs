extern "C" {
    static mut stderr: *mut _IO_FILE;
    fn fprintf(
        __stream: *mut FILE,
        __format: *const libc::c_char,
        ...
    ) -> libc::c_int;
    fn fgets(
        __s: *mut libc::c_char,
        __n: libc::c_int,
        __stream: *mut FILE,
    ) -> *mut libc::c_char;
    fn fseek(
        __stream: *mut FILE,
        __off: libc::c_long,
        __whence: libc::c_int,
    ) -> libc::c_int;
    fn clearerr(__stream: *mut FILE);
    fn feof(__stream: *mut FILE) -> libc::c_int;
    fn perror(__s: *const libc::c_char);
    fn atoi(__nptr: *const libc::c_char) -> libc::c_int;
    fn calloc(__nmemb: size_t, __size: size_t) -> *mut libc::c_void;
    fn realloc(__ptr: *mut libc::c_void, __size: size_t) -> *mut libc::c_void;
    fn free(__ptr: *mut libc::c_void);
    fn exit(__status: libc::c_int) -> !;
    fn strncpy(
        __dest: *mut libc::c_char,
        __src: *const libc::c_char,
        __n: size_t,
    ) -> *mut libc::c_char;
    fn strncmp(
        __s1: *const libc::c_char,
        __s2: *const libc::c_char,
        __n: size_t,
    ) -> libc::c_int;
    fn strdup(__s: *const libc::c_char) -> *mut libc::c_char;
    fn strchr(__s: *const libc::c_char, __c: libc::c_int)
        -> *mut libc::c_char;
    fn strrchr(
        __s: *const libc::c_char,
        __c: libc::c_int,
    ) -> *mut libc::c_char;
    fn strstr(
        __haystack: *const libc::c_char,
        __needle: *const libc::c_char,
    ) -> *mut libc::c_char;
    fn strlen(__s: *const libc::c_char) -> size_t;
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
// #[derive(Copy, Clone)]

pub use crate::src::driver::alert_data;
pub const NULL: *mut libc::c_void = std::ptr::null_mut::<libc::c_void>();
pub const SEEK_CUR: libc::c_int = 1 as libc::c_int;
pub const CRALERT_MAIL_SET: libc::c_int = 0x1 as libc::c_int;
pub const EXIT_FAILURE: libc::c_int = 1 as libc::c_int;
pub const OS_MAXSTR: libc::c_int = 1024 as libc::c_int;
#[no_mangle]
pub unsafe extern "C" fn os_calloc(mut num: size_t, mut size: size_t) -> *mut libc::c_void {
    let mut out: *mut libc::c_void = calloc(num, size);
    if out.is_null() {
        fprintf(
            stderr as *mut FILE,
            b"Memory allocation failed in os_calloc\0" as *const u8 as *const libc::c_char,
        );
        exit(EXIT_FAILURE);
    }
    return out;
}
#[no_mangle]
pub unsafe extern "C" fn os_realloc(
    mut ptr: *mut libc::c_void,
    mut new_size: size_t,
) -> *mut libc::c_void {
    let mut out: *mut libc::c_void = realloc(ptr, new_size);
    if out.is_null() {
        fprintf(
            stderr as *mut FILE,
            b"Memory allocation failed in os_realloc\0" as *const u8 as *const libc::c_char,
        );
        exit(EXIT_FAILURE);
    }
    return out;
}
#[no_mangle]
pub unsafe extern "C" fn os_strdup(
    mut str: *const libc::c_char,
) -> *mut libc::c_char {
    if str.is_null() {
        fprintf(
            stderr as *mut FILE,
            b"NULL string passed to os_strdup\0" as *const u8 as *const libc::c_char,
        );
        exit(EXIT_FAILURE);
    }
    let mut dup: *mut libc::c_char = strdup(str);
    if dup.is_null() {
        fprintf(
            stderr as *mut FILE,
            b"Memory allocation failed in os_strdup\0" as *const u8 as *const libc::c_char,
        );
        exit(EXIT_FAILURE);
    }
    return dup;
}
pub const ALERT_BEGIN: [libc::c_char; 9] =
    unsafe { std::mem::transmute::<[u8; 9], [libc::c_char; 9]>(*b"** Alert\0") };
pub const ALERT_BEGIN_SZ: libc::c_int = 8 as libc::c_int;
pub const RULE_BEGIN: [libc::c_char; 7] =
    unsafe { std::mem::transmute::<[u8; 7], [libc::c_char; 7]>(*b"Rule: \0") };
pub const RULE_BEGIN_SZ: libc::c_int = 6 as libc::c_int;
pub const SRCIP_BEGIN: [libc::c_char; 9] =
    unsafe { std::mem::transmute::<[u8; 9], [libc::c_char; 9]>(*b"Src IP: \0") };
pub const SRCIP_BEGIN_SZ: libc::c_int = 8 as libc::c_int;
pub const SRCPORT_BEGIN: [libc::c_char; 11] =
    unsafe { std::mem::transmute::<[u8; 11], [libc::c_char; 11]>(*b"Src Port: \0") };
pub const SRCPORT_BEGIN_SZ: libc::c_int = 10 as libc::c_int;
pub const DSTIP_BEGIN: [libc::c_char; 9] =
    unsafe { std::mem::transmute::<[u8; 9], [libc::c_char; 9]>(*b"Dst IP: \0") };
pub const DSTIP_BEGIN_SZ: libc::c_int = 8 as libc::c_int;
pub const DSTPORT_BEGIN: [libc::c_char; 11] =
    unsafe { std::mem::transmute::<[u8; 11], [libc::c_char; 11]>(*b"Dst Port: \0") };
pub const DSTPORT_BEGIN_SZ: libc::c_int = 10 as libc::c_int;
pub const USER_BEGIN: [libc::c_char; 7] =
    unsafe { std::mem::transmute::<[u8; 7], [libc::c_char; 7]>(*b"User: \0") };
pub const USER_BEGIN_SZ: libc::c_int = 6 as libc::c_int;
pub const ALERT_MAIL: [libc::c_char; 5] =
    unsafe { std::mem::transmute::<[u8; 5], [libc::c_char; 5]>(*b"mail\0") };
pub const ALERT_MAIL_SZ: libc::c_int = 4 as libc::c_int;
pub const LOG_LIMIT: libc::c_int = 100 as libc::c_int;
#[no_mangle]
pub unsafe extern "C" fn FreeAlertData(mut al_data: *mut alert_data) {
    let mut p: *mut *mut libc::c_char = std::ptr::null_mut::<*mut libc::c_char>();
    if !(*al_data).alertid.is_null() {
        free((*al_data).alertid as *mut libc::c_void);
        (*al_data).alertid = std::ptr::null_mut::<libc::c_char>();
    }
    if !(*al_data).date.is_null() {
        free((*al_data).date as *mut libc::c_void);
        (*al_data).date = std::ptr::null_mut::<libc::c_char>();
    }
    if !(*al_data).location.is_null() {
        free((*al_data).location as *mut libc::c_void);
        (*al_data).location = std::ptr::null_mut::<libc::c_char>();
    }
    if !(*al_data).comment.is_null() {
        free((*al_data).comment as *mut libc::c_void);
        (*al_data).comment = std::ptr::null_mut::<libc::c_char>();
    }
    if !(*al_data).group.is_null() {
        free((*al_data).group as *mut libc::c_void);
        (*al_data).group = std::ptr::null_mut::<libc::c_char>();
    }
    if !(*al_data).srcip.is_null() {
        free((*al_data).srcip as *mut libc::c_void);
        (*al_data).srcip = std::ptr::null_mut::<libc::c_char>();
    }
    if !(*al_data).dstip.is_null() {
        free((*al_data).dstip as *mut libc::c_void);
        (*al_data).dstip = std::ptr::null_mut::<libc::c_char>();
    }
    if !(*al_data).user.is_null() {
        free((*al_data).user as *mut libc::c_void);
        (*al_data).user = std::ptr::null_mut::<libc::c_char>();
    }
    if !(*al_data).filename.is_null() {
        free((*al_data).filename as *mut libc::c_void);
        (*al_data).filename = std::ptr::null_mut::<libc::c_char>();
    }
    free(al_data as *mut libc::c_void);
    al_data = std::ptr::null_mut::<alert_data>();
}
#[no_mangle]
pub unsafe extern "C" fn GetAlertData(
    mut flag: libc::c_int,
    mut fp: *mut FILE,
) -> *mut alert_data {
    let mut current_block: u64;
    let mut al_data: *mut alert_data = std::ptr::null_mut::<alert_data>();
    al_data =
        os_calloc(1 as size_t, std::mem::size_of::<alert_data>() as size_t) as *mut alert_data;
    let mut _r: libc::c_int = 0 as libc::c_int;
    let mut issyscheck: libc::c_int = 0 as libc::c_int;
    let mut log_size: size_t = 0 as size_t;
    let mut p: *mut libc::c_char = std::ptr::null_mut::<libc::c_char>();
    let mut str: [libc::c_char; 1025] = [0; 1025];
    str[OS_MAXSTR as usize] = '\0' as i32 as libc::c_char;
    loop {
        if fgets(&raw mut str as *mut libc::c_char, OS_MAXSTR, fp).is_null() {
            current_block = 3567897568976182940;
            break;
        }
        if strncmp(
            ALERT_BEGIN.as_ptr(),
            &raw mut str as *mut libc::c_char,
            ALERT_BEGIN_SZ as size_t,
        ) == 0 as libc::c_int
        {
            let mut m: *mut libc::c_char = std::ptr::null_mut::<libc::c_char>();
            let mut z: size_t = 0 as size_t;
            if _r == 2 as libc::c_int {
                if !(fseek(
                    fp,
                    strlen(&raw mut str as *mut libc::c_char).wrapping_neg()
                        as libc::c_long,
                    SEEK_CUR,
                ) != -(1 as libc::c_int))
                {
                    current_block = 4190919457040831865;
                    break;
                }
                return al_data;
            } else {
                p = (&raw mut str as *mut libc::c_char)
                    .offset(ALERT_BEGIN_SZ as isize)
                    .offset(1 as libc::c_int as isize);
                m = strstr(p, b":\0" as *const u8 as *const libc::c_char);
                if m.is_null() {
                    continue;
                }
                z = strlen(p).wrapping_sub(strlen(m));
                (*al_data).alertid = os_realloc(
                    (*al_data).alertid as *mut libc::c_void,
                    z.wrapping_add(1 as size_t)
                        .wrapping_mul(std::mem::size_of::<libc::c_char>() as size_t),
                ) as *mut libc::c_char;
                strncpy((*al_data).alertid, p, z);
                *(*al_data).alertid.offset(z as isize) = '\0' as i32 as libc::c_char;
                p = strchr(p, ' ' as i32);
                if p.is_null() {
                    continue;
                }
                p = p.offset(1);
                if flag & CRALERT_MAIL_SET != 0
                    && strncmp(ALERT_MAIL.as_ptr(), p, ALERT_MAIL_SZ as size_t)
                        != 0 as libc::c_int
                {
                    continue;
                }
                p = strchr(p, '-' as i32);
                if !p.is_null() {
                    p = p.offset(1);
                    while *p as libc::c_int == ' ' as i32 {
                        p = p.offset(1);
                    }
                    if !(*al_data).group.is_null() {
                        free((*al_data).group as *mut libc::c_void);
                        (*al_data).group = std::ptr::null_mut::<libc::c_char>();
                    }
                    (*al_data).group = os_strdup(p);
                    p = strrchr((*al_data).group, '\n' as i32);
                    if !p.is_null() {
                        *p = '\0' as i32 as libc::c_char;
                    }
                    if !(*al_data).group.is_null()
                        && !strstr(
                            (*al_data).group,
                            b"syscheck\0" as *const u8 as *const libc::c_char,
                        )
                        .is_null()
                    {
                        issyscheck = 1 as libc::c_int;
                    }
                }
                _r = 1 as libc::c_int;
            }
        } else {
            if _r < 1 as libc::c_int {
                continue;
            }
            if _r == 1 as libc::c_int {
                p = strrchr(&raw mut str as *mut libc::c_char, '\n' as i32);
                if !p.is_null() {
                    *p = '\0' as i32 as libc::c_char;
                }
                p = strchr(&raw mut str as *mut libc::c_char, ':' as i32);
                if !p.is_null() {
                    p = strchr(p, ' ' as i32);
                    if !p.is_null() {
                        *p = '\0' as i32 as libc::c_char;
                        p = p.offset(1);
                    } else {
                        perror(
                            b"date of location not NULL\0" as *const u8
                                as *const libc::c_char,
                        );
                        current_block = 4190919457040831865;
                        break;
                    }
                }
                if !(*al_data).date.is_null() || !(*al_data).location.is_null() || p.is_null() {
                    perror(
                        b"date or location not NULL or p is NULL\0" as *const u8
                            as *const libc::c_char,
                    );
                    current_block = 4190919457040831865;
                    break;
                } else {
                    (*al_data).date = os_strdup(&raw mut str as *mut libc::c_char);
                    (*al_data).location = os_strdup(p);
                    _r = 2 as libc::c_int;
                    log_size = 0 as size_t;
                }
            } else {
                if !(_r == 2 as libc::c_int) {
                    continue;
                }
                if strncmp(
                    RULE_BEGIN.as_ptr(),
                    &raw mut str as *mut libc::c_char,
                    RULE_BEGIN_SZ as size_t,
                ) == 0 as libc::c_int
                {
                    p = strrchr(&raw mut str as *mut libc::c_char, '\n' as i32);
                    if !p.is_null() {
                        *p = '\0' as i32 as libc::c_char;
                    }
                    p = (&raw mut str as *mut libc::c_char).offset(RULE_BEGIN_SZ as isize);
                    (*al_data).rule = atoi(p) as libc::c_uint;
                    p = strchr(p, ' ' as i32);
                    if !p.is_null() {
                        p = p.offset(1);
                        p = strchr(p, ' ' as i32);
                        if !p.is_null() {
                            p = p.offset(1);
                        }
                    }
                    if p.is_null() {
                        current_block = 4190919457040831865;
                        break;
                    }
                    (*al_data).level = atoi(p) as libc::c_uint;
                    p = strchr(p, '\'' as i32);
                    if p.is_null() {
                        current_block = 4190919457040831865;
                        break;
                    }
                    p = p.offset(1);
                    if !(*al_data).comment.is_null() {
                        free((*al_data).comment as *mut libc::c_void);
                        (*al_data).comment = std::ptr::null_mut::<libc::c_char>();
                    }
                    (*al_data).comment = os_strdup(p);
                    p = strrchr((*al_data).comment, '\'' as i32);
                    if p.is_null() {
                        current_block = 4190919457040831865;
                        break;
                    }
                    *p = '\0' as i32 as libc::c_char;
                } else if strncmp(
                    SRCIP_BEGIN.as_ptr(),
                    &raw mut str as *mut libc::c_char,
                    SRCIP_BEGIN_SZ as size_t,
                ) == 0 as libc::c_int
                {
                    p = strrchr(&raw mut str as *mut libc::c_char, '\n' as i32);
                    if !p.is_null() {
                        *p = '\0' as i32 as libc::c_char;
                    }
                    p = (&raw mut str as *mut libc::c_char).offset(SRCIP_BEGIN_SZ as isize);
                    if !(*al_data).srcip.is_null() {
                        free((*al_data).srcip as *mut libc::c_void);
                        (*al_data).srcip = std::ptr::null_mut::<libc::c_char>();
                    }
                    (*al_data).srcip = os_strdup(p);
                } else if strncmp(
                    SRCPORT_BEGIN.as_ptr(),
                    &raw mut str as *mut libc::c_char,
                    SRCPORT_BEGIN_SZ as size_t,
                ) == 0 as libc::c_int
                {
                    p = strrchr(&raw mut str as *mut libc::c_char, '\n' as i32);
                    if !p.is_null() {
                        *p = '\0' as i32 as libc::c_char;
                    }
                    p = (&raw mut str as *mut libc::c_char)
                        .offset(SRCPORT_BEGIN_SZ as isize);
                    (*al_data).srcport = atoi(p);
                } else if strncmp(
                    DSTIP_BEGIN.as_ptr(),
                    &raw mut str as *mut libc::c_char,
                    DSTIP_BEGIN_SZ as size_t,
                ) == 0 as libc::c_int
                {
                    p = strrchr(&raw mut str as *mut libc::c_char, '\n' as i32);
                    if !p.is_null() {
                        *p = '\0' as i32 as libc::c_char;
                    }
                    p = (&raw mut str as *mut libc::c_char).offset(DSTIP_BEGIN_SZ as isize);
                    if !(*al_data).dstip.is_null() {
                        free((*al_data).dstip as *mut libc::c_void);
                        (*al_data).dstip = std::ptr::null_mut::<libc::c_char>();
                    }
                    (*al_data).dstip = os_strdup(p);
                } else if strncmp(
                    DSTPORT_BEGIN.as_ptr(),
                    &raw mut str as *mut libc::c_char,
                    DSTPORT_BEGIN_SZ as size_t,
                ) == 0 as libc::c_int
                {
                    p = strrchr(&raw mut str as *mut libc::c_char, '\n' as i32);
                    if !p.is_null() {
                        *p = '\0' as i32 as libc::c_char;
                    }
                    p = (&raw mut str as *mut libc::c_char)
                        .offset(DSTPORT_BEGIN_SZ as isize);
                    (*al_data).dstport = atoi(p);
                } else if strncmp(
                    USER_BEGIN.as_ptr(),
                    &raw mut str as *mut libc::c_char,
                    USER_BEGIN_SZ as size_t,
                ) == 0 as libc::c_int
                {
                    p = strrchr(&raw mut str as *mut libc::c_char, '\n' as i32);
                    if !p.is_null() {
                        *p = '\0' as i32 as libc::c_char;
                    }
                    p = (&raw mut str as *mut libc::c_char).offset(USER_BEGIN_SZ as isize);
                    if !(*al_data).user.is_null() {
                        free((*al_data).user as *mut libc::c_void);
                        (*al_data).user = std::ptr::null_mut::<libc::c_char>();
                    }
                    (*al_data).user = os_strdup(p);
                } else if log_size < LOG_LIMIT as size_t {
                    p = strrchr(&raw mut str as *mut libc::c_char, '\n' as i32);
                    if !p.is_null() {
                        *p = '\0' as i32 as libc::c_char;
                    }
                    if issyscheck == 1 as libc::c_int {
                        if strncmp(
                            &raw mut str as *mut libc::c_char,
                            b"Integrity checksum changed for: '\0" as *const u8
                                as *const libc::c_char,
                            33 as size_t,
                        ) == 0 as libc::c_int
                        {
                            (*al_data).filename = strdup(
                                (&raw mut str as *mut libc::c_char)
                                    .offset(33 as libc::c_int as isize),
                            );
                            if !(*al_data).filename.is_null() {
                                *(*al_data).filename.offset(
                                    strlen((*al_data).filename).wrapping_sub(1 as size_t) as isize,
                                ) = '\0' as i32 as libc::c_char;
                            }
                        }
                        issyscheck = 0 as libc::c_int;
                    }
                }
            }
        }
    }
    match current_block {
        3567897568976182940 => {
            if feof(fp) != 0 && _r == 2 as libc::c_int {
                return al_data;
            }
        }
        _ => {}
    }
    FreeAlertData(al_data);
    clearerr(fp);
    return std::ptr::null_mut::<alert_data>();
}
