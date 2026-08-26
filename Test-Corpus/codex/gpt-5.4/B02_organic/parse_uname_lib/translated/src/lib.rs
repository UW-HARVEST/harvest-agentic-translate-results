use libc::{c_void, malloc};
use std::ffi::c_char;
use std::ptr;
use std::slice;

#[repr(C)]
#[allow(non_camel_case_types)]
pub struct os_data {
    pub os_name: *mut c_char,
    pub os_version: *mut c_char,
    pub os_major: *mut c_char,
    pub os_minor: *mut c_char,
    pub os_codename: *mut c_char,
    pub os_platform: *mut c_char,
    pub os_build: *mut c_char,
    pub os_uname: *mut c_char,
    pub os_arch: *mut c_char,
}

const ARCHS: [&[u8]; 12] = [
    b"x86_64",
    b"i386",
    b"i686",
    b"sparc",
    b"amd64",
    b"i86pc",
    b"ia64",
    b"AIX",
    b"armv6",
    b"armv7",
    b"aarch64",
    b"arm64",
];

unsafe fn c_string_len(ptr: *const c_char) -> usize {
    unsafe { libc::strlen(ptr) }
}

unsafe fn bytes_from_c_str<'a>(ptr: *const c_char) -> &'a [u8] {
    let len = unsafe { c_string_len(ptr) };
    unsafe { slice::from_raw_parts(ptr.cast::<u8>(), len) }
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|window| window == needle)
}

unsafe fn duplicate_bytes(bytes: &[u8]) -> *mut c_char {
    unsafe {
        let dest = malloc(bytes.len() + 1).cast::<u8>();
        if dest.is_null() {
            return ptr::null_mut();
        }

        ptr::copy_nonoverlapping(bytes.as_ptr(), dest, bytes.len());
        *dest.add(bytes.len()) = 0;
        dest.cast::<c_char>()
    }
}

unsafe fn duplicate_ptr(ptr: *const c_char) -> *mut c_char {
    unsafe { duplicate_bytes(bytes_from_c_str(ptr)) }
}

unsafe fn set_last_byte_to_nul(ptr: *mut c_char) {
    unsafe {
        let len = c_string_len(ptr);
        *ptr.cast::<u8>().add(len.wrapping_sub(1)) = 0;
    }
}

fn capture_major(bytes: &[u8]) -> Option<&[u8]> {
    let len = bytes.iter().take_while(|b| b.is_ascii_digit()).count();
    if len == 0 { None } else { Some(&bytes[..len]) }
}

fn capture_minor(bytes: &[u8]) -> Option<&[u8]> {
    let major_len = bytes.iter().take_while(|b| b.is_ascii_digit()).count();
    if major_len == 0 || bytes.get(major_len) != Some(&b'.') {
        return None;
    }

    let start = major_len + 1;
    let minor_len = bytes[start..]
        .iter()
        .take_while(|b| b.is_ascii_digit())
        .count();
    if minor_len == 0 {
        None
    } else {
        Some(&bytes[start..start + minor_len])
    }
}

fn capture_build(bytes: &[u8]) -> Option<&[u8]> {
    let major_len = bytes.iter().take_while(|b| b.is_ascii_digit()).count();
    if major_len == 0 || bytes.get(major_len) != Some(&b'.') {
        return None;
    }

    let minor_start = major_len + 1;
    let minor_len = bytes[minor_start..]
        .iter()
        .take_while(|b| b.is_ascii_digit())
        .count();
    if minor_len == 0 {
        return None;
    }

    let mut idx = minor_start + minor_len;
    if bytes.get(idx) != Some(&b'.') {
        return None;
    }
    idx += 1;

    let build_start = idx;
    let first_len = bytes[idx..]
        .iter()
        .take_while(|b| b.is_ascii_digit())
        .count();
    if first_len == 0 {
        return None;
    }
    idx += first_len;

    loop {
        if bytes.get(idx) != Some(&b'.') {
            break;
        }

        let next_start = idx + 1;
        let next_len = bytes
            .get(next_start..)
            .unwrap_or(&[])
            .iter()
            .take_while(|b| b.is_ascii_digit())
            .count();
        if next_len == 0 {
            break;
        }
        idx = next_start + next_len;
    }

    Some(&bytes[build_start..idx])
}

unsafe fn assign_capture(slot: *mut *mut c_char, capture: Option<&[u8]>) {
    if let Some(bytes) = capture {
        unsafe {
            *slot = duplicate_bytes(bytes);
        }
    }
}

unsafe fn get_os_arch(os_header: *mut c_char) -> *mut c_char {
    unsafe {
        let header = bytes_from_c_str(os_header);
        for arch in ARCHS {
            if find_bytes(header, arch).is_some() {
                return duplicate_bytes(arch);
            }
        }
        ptr::null_mut()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn parse_uname_string(uname: *mut c_char, osd: *mut os_data) {
    unsafe {
        let mut str_tmp: *mut c_char = ptr::null_mut();

        if osd.is_null() {
            return;
        }

        if let Some(idx) = find_bytes(bytes_from_c_str(uname), b" [Ver: ") {
            str_tmp = uname.add(idx);
        }

        if !str_tmp.is_null() {
            *str_tmp = 0;
            str_tmp = str_tmp.add(7);
            (*osd).os_name = duplicate_ptr(uname);
            set_last_byte_to_nul(str_tmp);

            let version_bytes = bytes_from_c_str(str_tmp);
            assign_capture(ptr::addr_of_mut!((*osd).os_major), capture_major(version_bytes));
            assign_capture(ptr::addr_of_mut!((*osd).os_minor), capture_minor(version_bytes));
            assign_capture(ptr::addr_of_mut!((*osd).os_build), capture_build(version_bytes));

            (*osd).os_version = duplicate_ptr(str_tmp);
            (*osd).os_platform = duplicate_bytes(b"windows");
        } else {
            if let Some(idx) = find_bytes(bytes_from_c_str(uname), b" [") {
                str_tmp = uname.add(idx);
            }

            if !str_tmp.is_null() {
                *str_tmp = 0;
                str_tmp = str_tmp.add(2);
                (*osd).os_name = duplicate_ptr(str_tmp);

                let mut inner_tmp: *mut c_char = ptr::null_mut();
                if let Some(idx) = find_bytes(bytes_from_c_str((*osd).os_name), b": ") {
                    inner_tmp = (*osd).os_name.add(idx);
                }

                if !inner_tmp.is_null() {
                    *inner_tmp = 0;
                    inner_tmp = inner_tmp.add(2);
                    (*osd).os_version = duplicate_ptr(inner_tmp);
                    set_last_byte_to_nul((*osd).os_version);

                    let mut codename_tmp: *mut c_char = ptr::null_mut();
                    if let Some(idx) = find_bytes(bytes_from_c_str((*osd).os_version), b" (") {
                        codename_tmp = (*osd).os_version.add(idx);
                    }

                    if !codename_tmp.is_null() {
                        *codename_tmp = 0;
                        codename_tmp = codename_tmp.add(2);
                        (*osd).os_codename = duplicate_ptr(codename_tmp);
                        set_last_byte_to_nul((*osd).os_codename);
                    }

                    let version_bytes = bytes_from_c_str((*osd).os_version);
                    assign_capture(ptr::addr_of_mut!((*osd).os_major), capture_major(version_bytes));
                    assign_capture(ptr::addr_of_mut!((*osd).os_minor), capture_minor(version_bytes));
                } else {
                    set_last_byte_to_nul((*osd).os_name);
                }

                let mut platform_tmp: *mut c_char = ptr::null_mut();
                if let Some(idx) = find_bytes(bytes_from_c_str((*osd).os_name), b"|") {
                    platform_tmp = (*osd).os_name.add(idx);
                }

                if !platform_tmp.is_null() {
                    *platform_tmp = 0;
                    platform_tmp = platform_tmp.add(1);
                    (*osd).os_platform = duplicate_ptr(platform_tmp);
                }
            }

            str_tmp = get_os_arch(uname);
            if !str_tmp.is_null() {
                (*osd).os_arch = duplicate_ptr(str_tmp);
                libc::free(str_tmp.cast::<c_void>());
            }
        }
    }
}
