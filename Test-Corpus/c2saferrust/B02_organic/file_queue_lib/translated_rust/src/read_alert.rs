



use std::io::BufRead;

use std::os::raw::c_char;

use std::ffi::CString;

use std::alloc::{alloc_zeroed, handle_alloc_error, Layout};

extern "C" {
    static mut stderr: *mut _IO_FILE;
    fn fprintf(
        __stream: *mut FILE,
        __format: *const ::core::ffi::c_char,
        ...
    ) -> ::core::ffi::c_int;
    fn fgets(
        __s: *mut ::core::ffi::c_char,
        __n: ::core::ffi::c_int,
        __stream: *mut FILE,
    ) -> *mut ::core::ffi::c_char;
    fn fseek(
        __stream: *mut FILE,
        __off: ::core::ffi::c_long,
        __whence: ::core::ffi::c_int,
    ) -> ::core::ffi::c_int;
    fn clearerr(__stream: *mut FILE);
    fn feof(__stream: *mut FILE) -> ::core::ffi::c_int;
    fn perror(__s: *const ::core::ffi::c_char);
    fn atoi(__nptr: *const ::core::ffi::c_char) -> ::core::ffi::c_int;
    fn calloc(__nmemb: size_t, __size: size_t) -> *mut ::core::ffi::c_void;
    fn realloc(__ptr: *mut ::core::ffi::c_void, __size: size_t) -> *mut ::core::ffi::c_void;
    fn free(__ptr: *mut ::core::ffi::c_void);
    fn exit(__status: ::core::ffi::c_int) -> !;
    fn strncpy(
        __dest: *mut ::core::ffi::c_char,
        __src: *const ::core::ffi::c_char,
        __n: size_t,
    ) -> *mut ::core::ffi::c_char;
    fn strncmp(
        __s1: *const ::core::ffi::c_char,
        __s2: *const ::core::ffi::c_char,
        __n: size_t,
    ) -> ::core::ffi::c_int;
    fn strdup(__s: *const ::core::ffi::c_char) -> *mut ::core::ffi::c_char;
    fn strchr(__s: *const ::core::ffi::c_char, __c: ::core::ffi::c_int)
        -> *mut ::core::ffi::c_char;
    fn strrchr(
        __s: *const ::core::ffi::c_char,
        __c: ::core::ffi::c_int,
    ) -> *mut ::core::ffi::c_char;
    fn strstr(
        __haystack: *const ::core::ffi::c_char,
        __needle: *const ::core::ffi::c_char,
    ) -> *mut ::core::ffi::c_char;
    fn strlen(__s: *const ::core::ffi::c_char) -> size_t;
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
pub struct alert_data {
    pub rule: ::core::ffi::c_uint,
    pub level: ::core::ffi::c_uint,
    pub alertid: *mut ::core::ffi::c_char,
    pub date: *mut ::core::ffi::c_char,
    pub location: *mut ::core::ffi::c_char,
    pub comment: *mut ::core::ffi::c_char,
    pub group: *mut ::core::ffi::c_char,
    pub srcip: *mut ::core::ffi::c_char,
    pub srcport: ::core::ffi::c_int,
    pub dstip: *mut ::core::ffi::c_char,
    pub dstport: ::core::ffi::c_int,
    pub user: *mut ::core::ffi::c_char,
    pub filename: *mut ::core::ffi::c_char,
}
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const SEEK_CUR: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const CRALERT_MAIL_SET: ::core::ffi::c_int = 0x1 as ::core::ffi::c_int;
pub const EXIT_FAILURE: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const OS_MAXSTR: ::core::ffi::c_int = 1024 as ::core::ffi::c_int;
#[no_mangle]
pub fn os_calloc(num: usize, size: usize) -> *mut ::core::ffi::c_void {
    let total = num.checked_mul(size).unwrap_or_else(|| {
        eprintln!("Memory allocation failed in os_calloc");
        std::process::exit(EXIT_FAILURE);
    });

    let layout = Layout::from_size_align(total, 1).unwrap_or_else(|_| {
        eprintln!("Memory allocation failed in os_calloc");
        std::process::exit(EXIT_FAILURE);
    });

    let ptr = unsafe { alloc_zeroed(layout) };
    if ptr.is_null() {
        handle_alloc_error(layout);
    }

    ptr as *mut ::core::ffi::c_void
}

#[no_mangle]
pub fn os_realloc<T>(buf: *mut T, new_len: usize) -> *mut T {
    let new_buf = if buf.is_null() {
        let mut v = Vec::<T>::with_capacity(new_len);
        let ptr = v.as_mut_ptr();
        ::core::mem::forget(v);
        ptr
    } else {
        let mut v = unsafe { Vec::<T>::from_raw_parts(buf, 0, new_len) };
        if v.capacity() < new_len {
            v.reserve_exact(new_len - v.capacity());
        }
        let ptr = v.as_mut_ptr();
        ::core::mem::forget(v);
        ptr
    };

    if new_buf.is_null() {
        panic!("Memory allocation failed in os_realloc");
    }

    new_buf
}

#[no_mangle]
pub fn os_strdup(s: &str) -> String {
    s.to_owned()
}

pub const ALERT_BEGIN: [::core::ffi::c_char; 9] =
    unsafe { ::core::mem::transmute::<[u8; 9], [::core::ffi::c_char; 9]>(*b"** Alert\0") };
pub const ALERT_BEGIN_SZ: ::core::ffi::c_int = 8 as ::core::ffi::c_int;
pub const RULE_BEGIN: [::core::ffi::c_char; 7] =
    unsafe { ::core::mem::transmute::<[u8; 7], [::core::ffi::c_char; 7]>(*b"Rule: \0") };
pub const RULE_BEGIN_SZ: ::core::ffi::c_int = 6 as ::core::ffi::c_int;
pub const SRCIP_BEGIN: [::core::ffi::c_char; 9] =
    unsafe { ::core::mem::transmute::<[u8; 9], [::core::ffi::c_char; 9]>(*b"Src IP: \0") };
pub const SRCIP_BEGIN_SZ: ::core::ffi::c_int = 8 as ::core::ffi::c_int;
pub const SRCPORT_BEGIN: [::core::ffi::c_char; 11] =
    unsafe { ::core::mem::transmute::<[u8; 11], [::core::ffi::c_char; 11]>(*b"Src Port: \0") };
pub const SRCPORT_BEGIN_SZ: ::core::ffi::c_int = 10 as ::core::ffi::c_int;
pub const DSTIP_BEGIN: [::core::ffi::c_char; 9] =
    unsafe { ::core::mem::transmute::<[u8; 9], [::core::ffi::c_char; 9]>(*b"Dst IP: \0") };
pub const DSTIP_BEGIN_SZ: ::core::ffi::c_int = 8 as ::core::ffi::c_int;
pub const DSTPORT_BEGIN: [::core::ffi::c_char; 11] =
    unsafe { ::core::mem::transmute::<[u8; 11], [::core::ffi::c_char; 11]>(*b"Dst Port: \0") };
pub const DSTPORT_BEGIN_SZ: ::core::ffi::c_int = 10 as ::core::ffi::c_int;
pub const USER_BEGIN: [::core::ffi::c_char; 7] =
    unsafe { ::core::mem::transmute::<[u8; 7], [::core::ffi::c_char; 7]>(*b"User: \0") };
pub const USER_BEGIN_SZ: ::core::ffi::c_int = 6 as ::core::ffi::c_int;
pub const ALERT_MAIL: [::core::ffi::c_char; 5] =
    unsafe { ::core::mem::transmute::<[u8; 5], [::core::ffi::c_char; 5]>(*b"mail\0") };
pub const ALERT_MAIL_SZ: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const LOG_LIMIT: ::core::ffi::c_int = 100 as ::core::ffi::c_int;
#[no_mangle]
pub fn FreeAlertData(al_data: *mut alert_data) {
    let _ = al_data;
}

#[no_mangle]
pub unsafe extern "C" fn GetAlertData(
    mut flag: ::core::ffi::c_int,
    mut fp: *mut FILE,
) -> *mut alert_data {
    let mut current_block: u64;
    let mut al_data: *mut alert_data = ::core::ptr::null_mut::<alert_data>();
    al_data =
        os_calloc(1usize, ::core::mem::size_of::<alert_data>()) as *mut alert_data;
    let mut _r: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut issyscheck: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut log_size: size_t = 0 as size_t;
    let mut p: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<::core::ffi::c_char>();
    let mut str: [::core::ffi::c_char; 1025] = [0; 1025];
    str[OS_MAXSTR as usize] = '\0' as i32 as ::core::ffi::c_char;
    loop {
        {
    let mut line_buf = String::new();
    let bytes_read = std::io::BufRead::read_line(&mut fp, &mut line_buf).unwrap_or(0);
    if bytes_read == 0 {
        current_block = 3567897568976182940;
        break;
    }

    if line_buf.len() >= ALERT_BEGIN_SZ as usize
        && line_buf.as_bytes()[..ALERT_BEGIN_SZ as usize] == ALERT_BEGIN[..ALERT_BEGIN_SZ as usize].iter().map(|&c| c as u8).collect::<Vec<_>>()[..]
    {
            let mut m: Option<usize> = None;
let mut z: usize = 0;

if _r == 2 {
    let str_len = str.iter().position(|&c| c == 0).unwrap_or(str.len());
    if fseek(fp, -(str_len as ::core::ffi::c_long), SEEK_CUR) == -1 {
        current_block = 4190919457040831865;
        break;
    }
    return al_data;
} else {
    let str_len = str.iter().position(|&c| c == 0).unwrap_or(str.len());
    let line_bytes: Vec<u8> = str[..str_len].iter().map(|&c| c as u8).collect();
    let line = String::from_utf8_lossy(&line_bytes);

    let start = ALERT_BEGIN_SZ as usize + 1;
    if start >= line.len() {
        continue;
    }

    let p_slice = &line[start..];
    m = p_slice.find(':');
    if m.is_none() {
        continue;
    }

    z = m.unwrap();

    let alert_id = &p_slice[..z];
    if !(*al_data).alertid.is_null() {
        free((*al_data).alertid as *mut ::core::ffi::c_void);
    }
    match CString::new(alert_id) {
        Ok(s) => {
            (*al_data).alertid = s.into_raw();
        }
        Err(_) => {
            continue;
        }
    }

    let space_pos = match p_slice.find(' ') {
        Some(pos) => pos,
        None => continue,
    };
    let mut rest = &p_slice[space_pos + 1..];

    if flag & CRALERT_MAIL_SET != 0 {
        let alert_mail_len = ALERT_MAIL.iter().position(|&c| c == 0).unwrap_or(ALERT_MAIL.len());
        let alert_mail_bytes: Vec<u8> = ALERT_MAIL[..alert_mail_len].iter().map(|&c| c as u8).collect();
        let alert_mail = String::from_utf8_lossy(&alert_mail_bytes);
        if !rest.starts_with(alert_mail.as_ref()) {
            continue;
        }
    }

    if let Some(dash_pos) = rest.find('-') {
        rest = &rest[dash_pos + 1..];
        rest = rest.trim_start();

        if !(*al_data).group.is_null() {
            free((*al_data).group as *mut ::core::ffi::c_void);
            (*al_data).group = ::core::ptr::null_mut();
        }

        let group_text = rest.trim_end_matches('\n');
        match CString::new(group_text) {
            Ok(s) => {
                (*al_data).group = s.into_raw();
            }
            Err(_) => {
                continue;
            }
        }

        if group_text.contains("syscheck") {
            issyscheck = 1 as ::core::ffi::c_int;
        }
    }

    _r = 1 as ::core::ffi::c_int;
}


    } else {
        if _r < 1 as ::core::ffi::c_int {
            continue;
        }
        if _r == 1 as ::core::ffi::c_int {
                if let Some(newline_pos) = str.iter().position(|&c| c == '\n' as i8) {
    str[newline_pos] = 0;
}

let nul_pos = str.iter().position(|&c| c == 0).unwrap_or(str.len());
let line = String::from_utf8_lossy(
    &str[..nul_pos]
        .iter()
        .map(|&c| c as u8)
        .collect::<Vec<_>>(),
)
.into_owned();

let p = if let Some(colon_pos) = line.find(':') {
    if let Some(space_rel_pos) = line[colon_pos..].find(' ') {
        let split_pos = colon_pos + space_rel_pos;
        let date_part = &line[..split_pos];
        let location_part = &line[(split_pos + 1)..];

        if !(*al_data).date.is_null() || !(*al_data).location.is_null() || location_part.is_empty() {
            perror(b"date or location not NULL or p is NULL\0".as_ptr() as *const ::core::ffi::c_char);
            current_block = 4190919457040831865;
            break;
        }

        let date_owned = os_strdup(date_part);
        let location_owned = os_strdup(location_part);

        (*al_data).date = ::std::ffi::CString::new(date_owned)
            .unwrap()
            .into_raw();
        (*al_data).location = ::std::ffi::CString::new(location_owned)
            .unwrap()
            .into_raw();

        _r = 2 as ::core::ffi::c_int;
        log_size = 0 as usize;

        Some(location_part)
    } else {
        perror(b"date of location not NULL\0".as_ptr() as *const ::core::ffi::c_char);
        current_block = 4190919457040831865;
        break;
    }
} else {
    None
};

if p.is_none() {
    perror(b"date or location not NULL or p is NULL\0".as_ptr() as *const ::core::ffi::c_char);
    current_block = 4190919457040831865;
    break;
}


        } else {
                if _r != 2 {
    continue;
}

let line = {
    let nul_pos = str.iter().position(|&c| c == 0).unwrap_or(str.len());
    let bytes: Vec<u8> = str[..nul_pos].iter().map(|&c| c as u8).collect();
    String::from_utf8_lossy(&bytes).trim_end_matches('\n').to_string()
};

if line.starts_with("Rule: ") {
    let rest = &line[RULE_BEGIN_SZ as usize..];

    (*al_data).rule = rest
        .split_whitespace()
        .next()
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(0);

    let mut parts = rest.split_whitespace();
    parts.next();
    parts.next();
    let level_str = parts.next();

    if level_str.is_none() {
        current_block = 4190919457040831865;
        break;
    }

    (*al_data).level = level_str.unwrap().parse::<u32>().unwrap_or(0);

    let start_quote = line.find('\'');
    if start_quote.is_none() {
        current_block = 4190919457040831865;
        break;
    }

    let comment_part = &line[start_quote.unwrap() + 1..];
    let end_quote = comment_part.rfind('\'');
    if end_quote.is_none() {
        current_block = 4190919457040831865;
        break;
    }

    let comment = &comment_part[..end_quote.unwrap()];

    if !(*al_data).comment.is_null() {
        free((*al_data).comment as *mut ::core::ffi::c_void);
        (*al_data).comment = ::core::ptr::null_mut::<c_char>();
    }
    (*al_data).comment = strdup(CString::new(comment).unwrap().as_ptr());
} else if line.starts_with("Src IP: ") {
    let value = &line[SRCIP_BEGIN_SZ as usize..];
    if !(*al_data).srcip.is_null() {
        free((*al_data).srcip as *mut ::core::ffi::c_void);
        (*al_data).srcip = ::core::ptr::null_mut::<c_char>();
    }
    (*al_data).srcip = strdup(CString::new(value).unwrap().as_ptr());
} else if line.starts_with("Src Port: ") {
    let value = &line[SRCPORT_BEGIN_SZ as usize..];
    (*al_data).srcport = value.trim().parse().unwrap_or(0);
} else if line.starts_with("Dst IP: ") {
    let value = &line[DSTIP_BEGIN_SZ as usize..];
    if !(*al_data).dstip.is_null() {
        free((*al_data).dstip as *mut ::core::ffi::c_void);
        (*al_data).dstip = ::core::ptr::null_mut::<c_char>();
    }
    (*al_data).dstip = strdup(CString::new(value).unwrap().as_ptr());
} else if line.starts_with("Dst Port: ") {
    let value = &line[DSTPORT_BEGIN_SZ as usize..];
    (*al_data).dstport = value.trim().parse().unwrap_or(0);
} else if line.starts_with("User: ") {
    let value = &line[USER_BEGIN_SZ as usize..];
    if !(*al_data).user.is_null() {
        free((*al_data).user as *mut ::core::ffi::c_void);
        (*al_data).user = ::core::ptr::null_mut::<c_char>();
    }
    (*al_data).user = strdup(CString::new(value).unwrap().as_ptr());
} else if log_size < LOG_LIMIT as usize {
    if issyscheck == 1 {
        if let Some(filename) = line.strip_prefix("Integrity checksum changed for: '") {
            let filename = filename.strip_suffix('\'').unwrap_or(filename);
            if !(*al_data).filename.is_null() {
                free((*al_data).filename as *mut ::core::ffi::c_void);
                (*al_data).filename = ::core::ptr::null_mut::<c_char>();
            }
            (*al_data).filename = strdup(CString::new(filename).unwrap().as_ptr());
        }
        issyscheck = 0;
    }
}


        }
    }
}

    }
    match current_block {
        3567897568976182940 => {
            if feof(fp) != 0 && _r == 2 as ::core::ffi::c_int {
                return al_data;
            }
        }
        _ => {}
    }
    FreeAlertData(al_data);
    clearerr(fp);
    return ::core::ptr::null_mut::<alert_data>();
}
