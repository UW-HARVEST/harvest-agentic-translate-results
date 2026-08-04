use libc::{c_char, c_int, c_uint};
use crate::shared::{os_calloc, os_realloc, os_strdup};
use crate::{os_free, os_clearnl};

pub const ALERTS_DAILY: &[u8] = b"alerts.log\0";

pub const CRALERT_MAIL_SET: c_int = 0x001;
pub const CRALERT_EXEC_SET: c_int = 0x002;
pub const CRALERT_READ_ALL: c_int = 0x004;
pub const CRALERT_READ_FAILED: c_int = 0x008;
pub const CRALERT_FP_SET: c_int = 0x010;

#[repr(C)]
pub struct alert_data {
    pub rule: c_uint,
    pub level: c_uint,
    pub alertid: *mut c_char,
    pub date: *mut c_char,
    pub location: *mut c_char,
    pub comment: *mut c_char,
    pub group: *mut c_char,
    pub srcip: *mut c_char,
    pub srcport: c_int,
    pub dstip: *mut c_char,
    pub dstport: c_int,
    pub user: *mut c_char,
    pub filename: *mut c_char,
}

#[unsafe(no_mangle)]
pub extern "C" fn FreeAlertData(al_data: *mut alert_data) {
    unsafe {
        if al_data.is_null() { return; }
        os_free!((*al_data).alertid);
        os_free!((*al_data).date);
        os_free!((*al_data).location);
        os_free!((*al_data).comment);
        os_free!((*al_data).group);
        os_free!((*al_data).srcip);
        os_free!((*al_data).dstip);
        os_free!((*al_data).user);
        os_free!((*al_data).filename);
        libc::free(al_data as *mut libc::c_void);
    }
}

unsafe fn goto_l_error(al_data: *mut alert_data, fp: *mut libc::FILE) {
    FreeAlertData(al_data);
    libc::clearerr(fp);
}

#[unsafe(no_mangle)]
pub extern "C" fn GetAlertData(flag: c_int, fp: *mut libc::FILE) -> *mut alert_data {
    unsafe {
        let al_data = os_calloc(1, std::mem::size_of::<alert_data>()) as *mut alert_data;
        let mut _r = 0;
        let mut issyscheck = 0;
        let mut log_size = 0;
        let mut p: *mut c_char = std::ptr::null_mut();
        let mut str_buf: [c_char; 1025] = [0; 1025];

        while !libc::fgets(str_buf.as_mut_ptr(), 1024, fp).is_null() {
            if libc::strncmp(b"** Alert\0".as_ptr() as *const c_char, str_buf.as_ptr() as *const c_char, 8) == 0 {
                let mut z: usize = 0;
                if _r == 2 {
                    if libc::fseek(fp, -(libc::strlen(str_buf.as_ptr() as *const c_char) as libc::c_long), libc::SEEK_CUR) != -1 {
                        return al_data;
                    } else {
                        goto_l_error(al_data, fp);
                        return std::ptr::null_mut();
                    }
                }

                p = str_buf.as_mut_ptr().add(8 + 1);

                let m = libc::strstr(p as *const c_char, b":\0".as_ptr() as *const c_char);
                if m.is_null() {
                    continue;
                }

                z = libc::strlen(p as *const c_char) - libc::strlen(m as *const c_char);
                (*al_data).alertid = os_realloc((*al_data).alertid as *mut libc::c_void, z + 1) as *mut c_char;
                libc::strncpy((*al_data).alertid, p as *const c_char, z);
                *(*al_data).alertid.add(z) = 0;

                p = libc::strchr(p as *const c_char, b' ' as c_int);
                if p.is_null() {
                    continue;
                }
                p = p.add(1);

                if (flag & CRALERT_MAIL_SET) != 0 && libc::strncmp(b"mail\0".as_ptr() as *const c_char, p as *const c_char, 4) != 0 {
                    continue;
                }

                p = libc::strchr(p as *const c_char, b'-' as c_int);
                if !p.is_null() {
                    p = p.add(1);
                    while *p == b' ' as c_char {
                        p = p.add(1);
                    }
                    os_free!((*al_data).group);
                    (*al_data).group = os_strdup(p as *const c_char);

                    os_clearnl!((*al_data).group, p);
                    if !(*al_data).group.is_null() && !libc::strstr((*al_data).group as *const c_char, b"syscheck\0".as_ptr() as *const c_char).is_null() {
                        issyscheck = 1;
                    }
                }

                _r = 1;
                continue;
            }

            if _r < 1 {
                continue;
            }

            if _r == 1 {
                os_clearnl!(str_buf.as_mut_ptr(), p);

                p = libc::strchr(str_buf.as_mut_ptr() as *const c_char, b':' as c_int);
                if !p.is_null() {
                    p = libc::strchr(p as *const c_char, b' ' as c_int);
                    if !p.is_null() {
                        *p = 0;
                        p = p.add(1);
                    } else {
                        libc::perror(b"date of location not NULL\0".as_ptr() as *const c_char);
                        goto_l_error(al_data, fp);
                        return std::ptr::null_mut();
                    }
                }

                if !(*al_data).date.is_null() || !(*al_data).location.is_null() || p.is_null() {
                    libc::perror(b"date or location not NULL or p is NULL\0".as_ptr() as *const c_char);
                    goto_l_error(al_data, fp);
                    return std::ptr::null_mut();
                }

                (*al_data).date = os_strdup(str_buf.as_ptr() as *const c_char);
                (*al_data).location = os_strdup(p as *const c_char);
                _r = 2;
                log_size = 0;
                continue;
            } else if _r == 2 {
                if libc::strncmp(b"Rule: \0".as_ptr() as *const c_char, str_buf.as_ptr() as *const c_char, 6) == 0 {
                    os_clearnl!(str_buf.as_mut_ptr(), p);

                    p = str_buf.as_mut_ptr().add(6);
                    (*al_data).rule = libc::atoi(p as *const c_char) as c_uint;

                    p = libc::strchr(p as *const c_char, b' ' as c_int);
                    if !p.is_null() {
                        p = p.add(1);
                        p = libc::strchr(p as *const c_char, b' ' as c_int);
                        if !p.is_null() {
                            p = p.add(1);
                        }
                    }

                    if p.is_null() {
                        goto_l_error(al_data, fp);
                        return std::ptr::null_mut();
                    }

                    (*al_data).level = libc::atoi(p as *const c_char) as c_uint;

                    p = libc::strchr(p as *const c_char, b'\'' as c_int);
                    if p.is_null() {
                        goto_l_error(al_data, fp);
                        return std::ptr::null_mut();
                    }

                    p = p.add(1);
                    os_free!((*al_data).comment);
                    (*al_data).comment = os_strdup(p as *const c_char);

                    p = libc::strrchr((*al_data).comment as *const c_char, b'\'' as c_int);
                    if !p.is_null() {
                        *p = 0;
                    } else {
                        goto_l_error(al_data, fp);
                        return std::ptr::null_mut();
                    }
                } else if libc::strncmp(b"Src IP: \0".as_ptr() as *const c_char, str_buf.as_ptr() as *const c_char, 8) == 0 {
                    os_clearnl!(str_buf.as_mut_ptr(), p);
                    p = str_buf.as_mut_ptr().add(8);
                    os_free!((*al_data).srcip);
                    (*al_data).srcip = os_strdup(p as *const c_char);
                } else if libc::strncmp(b"Src Port: \0".as_ptr() as *const c_char, str_buf.as_ptr() as *const c_char, 10) == 0 {
                    os_clearnl!(str_buf.as_mut_ptr(), p);
                    p = str_buf.as_mut_ptr().add(10);
                    (*al_data).srcport = libc::atoi(p as *const c_char);
                } else if libc::strncmp(b"Dst IP: \0".as_ptr() as *const c_char, str_buf.as_ptr() as *const c_char, 8) == 0 {
                    os_clearnl!(str_buf.as_mut_ptr(), p);
                    p = str_buf.as_mut_ptr().add(8);
                    os_free!((*al_data).dstip);
                    (*al_data).dstip = os_strdup(p as *const c_char);
                } else if libc::strncmp(b"Dst Port: \0".as_ptr() as *const c_char, str_buf.as_ptr() as *const c_char, 10) == 0 {
                    os_clearnl!(str_buf.as_mut_ptr(), p);
                    p = str_buf.as_mut_ptr().add(10);
                    (*al_data).dstport = libc::atoi(p as *const c_char);
                } else if libc::strncmp(b"User: \0".as_ptr() as *const c_char, str_buf.as_ptr() as *const c_char, 6) == 0 {
                    os_clearnl!(str_buf.as_mut_ptr(), p);
                    p = str_buf.as_mut_ptr().add(6);
                    os_free!((*al_data).user);
                    (*al_data).user = os_strdup(p as *const c_char);
                } else if log_size < 100 {
                    os_clearnl!(str_buf.as_mut_ptr(), p);
                    if issyscheck == 1 {
                        if libc::strncmp(str_buf.as_ptr() as *const c_char, b"Integrity checksum changed for: '\0".as_ptr() as *const c_char, 33) == 0 {
                            (*al_data).filename = libc::strdup(str_buf.as_ptr().add(33) as *const c_char);
                            if !(*al_data).filename.is_null() {
                                let len = libc::strlen((*al_data).filename as *const c_char);
                                if len > 0 {
                                    *(*al_data).filename.add(len - 1) = 0;
                                }
                            }
                        }
                        issyscheck = 0;
                    }
                }
            }
        }

        if libc::feof(fp) != 0 && _r == 2 {
            return al_data;
        }

        goto_l_error(al_data, fp);
        std::ptr::null_mut()
    }
}
