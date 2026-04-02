//! Integration tests comparing C vs Rust implementations.
//! Both libraries are loaded via libloading to compare outputs.

use libloading::{Library, Symbol};
use std::ffi::{CStr, CString};
use std::io::Write;
use std::ptr;

#[repr(C)]
struct AlertData {
    rule: u32,
    level: u32,
    alertid: *mut i8,
    date: *mut i8,
    location: *mut i8,
    comment: *mut i8,
    group: *mut i8,
    srcip: *mut i8,
    srcport: i32,
    dstip: *mut i8,
    dstport: i32,
    user: *mut i8,
    filename: *mut i8,
}

#[repr(C)]
struct FileQueue {
    last_change: libc::time_t,
    year: i32,
    day: i32,
    flags: i32,
    mon: [i8; 4],
    file_name: [i8; 257],
    fp: *mut libc::FILE,
    f_status: libc::stat,
}

#[repr(C)]
struct Tm {
    tm_sec: i32,
    tm_min: i32,
    tm_hour: i32,
    tm_mday: i32,
    tm_mon: i32,
    tm_year: i32,
    tm_wday: i32,
    tm_yday: i32,
    tm_isdst: i32,
    tm_gmtoff: libc::c_long,
    tm_zone: *const i8,
}

fn c_lib_path() -> String {
    format!("{}/c_src/build/libdriver.so", env!("CARGO_MANIFEST_DIR"))
}

fn rust_lib_path() -> String {
    // The cdylib is built as libdriver.so in target/debug/
    let manifest = env!("CARGO_MANIFEST_DIR");
    format!("{}/target/debug/libdriver.so", manifest)
}

unsafe fn str_from_ptr(p: *const i8) -> Option<String> {
    if p.is_null() { None } else { Some(CStr::from_ptr(p).to_string_lossy().into_owned()) }
}

fn make_temp_file(content: &str) -> (tempfile::NamedTempFile, CString) {
    let mut f = tempfile::NamedTempFile::new().unwrap();
    f.write_all(content.as_bytes()).unwrap();
    f.flush().unwrap();
    let path = CString::new(f.path().to_str().unwrap()).unwrap();
    (f, path)
}

struct Libs {
    c: Library,
    r: Library,
}

impl Libs {
    fn load() -> Self {
        unsafe {
            Libs {
                c: Library::new(c_lib_path()).expect("Failed to load C lib"),
                r: Library::new(rust_lib_path()).expect("Failed to load Rust lib"),
            }
        }
    }
}

// ── Test os_calloc ──
#[test]
fn test_os_calloc() {
    let libs = Libs::load();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(usize, usize) -> *mut libc::c_void> =
            libs.c.get(b"os_calloc").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(usize, usize) -> *mut libc::c_void> =
            libs.r.get(b"os_calloc").unwrap();

        let c_ptr = c_fn(10, 1);
        let r_ptr = r_fn(10, 1);
        assert!(!c_ptr.is_null() && !r_ptr.is_null());

        let c_s = std::slice::from_raw_parts(c_ptr as *const u8, 10);
        let r_s = std::slice::from_raw_parts(r_ptr as *const u8, 10);
        assert_eq!(c_s, r_s, "os_calloc: memory content differs");

        libc::free(c_ptr);
        libc::free(r_ptr);
    }
}

// ── Test os_realloc ──
#[test]
fn test_os_realloc() {
    let libs = Libs::load();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(*mut libc::c_void, usize) -> *mut libc::c_void> =
            libs.c.get(b"os_realloc").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(*mut libc::c_void, usize) -> *mut libc::c_void> =
            libs.r.get(b"os_realloc").unwrap();

        let c_ptr = libc::calloc(5, 1);
        let r_ptr = libc::calloc(5, 1);
        let c2 = c_fn(c_ptr, 20);
        let r2 = r_fn(r_ptr, 20);
        assert!(!c2.is_null() && !r2.is_null());
        libc::free(c2);
        libc::free(r2);
    }
}

// ── Test os_strdup ──
#[test]
fn test_os_strdup() {
    let libs = Libs::load();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(*const i8) -> *mut i8> =
            libs.c.get(b"os_strdup").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(*const i8) -> *mut i8> =
            libs.r.get(b"os_strdup").unwrap();

        let s = CString::new("hello world").unwrap();
        let c_d = c_fn(s.as_ptr());
        let r_d = r_fn(s.as_ptr());
        assert_eq!(CStr::from_ptr(c_d).to_bytes(), CStr::from_ptr(r_d).to_bytes());
        libc::free(c_d as *mut libc::c_void);
        libc::free(r_d as *mut libc::c_void);
    }
}

// ── Test GetAlertData basic ──
#[test]
fn test_get_alert_data_basic() {
    let alert = "\
** Alert 1234567.890:abc - syscheck\n\
2024 Mar 15 10:30:45 /var/log/test\n\
Rule: 550 (level 7) -> 'Test alert rule'\n\
Src IP: 192.168.1.100\n\
Src Port: 8080\n\
Dst IP: 10.0.0.1\n\
Dst Port: 443\n\
User: testuser\n\
Integrity checksum changed for: '/etc/passwd'\n\
";

    let libs = Libs::load();
    unsafe {
        let c_get: Symbol<unsafe extern "C" fn(i32, *mut libc::FILE) -> *mut AlertData> =
            libs.c.get(b"GetAlertData").unwrap();
        let r_get: Symbol<unsafe extern "C" fn(i32, *mut libc::FILE) -> *mut AlertData> =
            libs.r.get(b"GetAlertData").unwrap();
        let c_free: Symbol<unsafe extern "C" fn(*mut AlertData)> =
            libs.c.get(b"FreeAlertData").unwrap();
        let r_free: Symbol<unsafe extern "C" fn(*mut AlertData)> =
            libs.r.get(b"FreeAlertData").unwrap();

        let (_tf1, p1) = make_temp_file(alert);
        let c_fp = libc::fopen(p1.as_ptr(), b"r\0".as_ptr() as *const i8);
        let c_res = c_get(0, c_fp);
        libc::fclose(c_fp);

        let (_tf2, p2) = make_temp_file(alert);
        let r_fp = libc::fopen(p2.as_ptr(), b"r\0".as_ptr() as *const i8);
        let r_res = r_get(0, r_fp);
        libc::fclose(r_fp);

        assert!(!c_res.is_null(), "C returned NULL");
        assert!(!r_res.is_null(), "Rust returned NULL");

        assert_eq!((*c_res).rule, (*r_res).rule, "rule: C={} R={}", (*c_res).rule, (*r_res).rule);
        assert_eq!((*c_res).level, (*r_res).level, "level: C={} R={}", (*c_res).level, (*r_res).level);
        assert_eq!(str_from_ptr((*c_res).alertid), str_from_ptr((*r_res).alertid), "alertid");
        assert_eq!(str_from_ptr((*c_res).date), str_from_ptr((*r_res).date), "date");
        assert_eq!(str_from_ptr((*c_res).location), str_from_ptr((*r_res).location), "location");
        assert_eq!(str_from_ptr((*c_res).comment), str_from_ptr((*r_res).comment), "comment");
        assert_eq!(str_from_ptr((*c_res).group), str_from_ptr((*r_res).group), "group");
        assert_eq!(str_from_ptr((*c_res).srcip), str_from_ptr((*r_res).srcip), "srcip");
        assert_eq!((*c_res).srcport, (*r_res).srcport, "srcport");
        assert_eq!(str_from_ptr((*c_res).dstip), str_from_ptr((*r_res).dstip), "dstip");
        assert_eq!((*c_res).dstport, (*r_res).dstport, "dstport");
        assert_eq!(str_from_ptr((*c_res).user), str_from_ptr((*r_res).user), "user");
        assert_eq!(str_from_ptr((*c_res).filename), str_from_ptr((*r_res).filename), "filename");

        c_free(c_res);
        r_free(r_res);
    }
}

// ── Test GetAlertData empty input ──
#[test]
fn test_get_alert_data_empty() {
    let content = "some random text\nno alerts here\n";
    let libs = Libs::load();
    unsafe {
        let c_get: Symbol<unsafe extern "C" fn(i32, *mut libc::FILE) -> *mut AlertData> =
            libs.c.get(b"GetAlertData").unwrap();
        let r_get: Symbol<unsafe extern "C" fn(i32, *mut libc::FILE) -> *mut AlertData> =
            libs.r.get(b"GetAlertData").unwrap();

        let (_tf1, p1) = make_temp_file(content);
        let c_fp = libc::fopen(p1.as_ptr(), b"r\0".as_ptr() as *const i8);
        let c_r = c_get(0, c_fp);
        libc::fclose(c_fp);

        let (_tf2, p2) = make_temp_file(content);
        let r_fp = libc::fopen(p2.as_ptr(), b"r\0".as_ptr() as *const i8);
        let r_r = r_get(0, r_fp);
        libc::fclose(r_fp);

        assert_eq!(c_r.is_null(), r_r.is_null(), "empty: C null={} R null={}", c_r.is_null(), r_r.is_null());
    }
}

// ── Test GetAlertData two alerts ──
#[test]
fn test_get_alert_data_two_alerts() {
    let content = "\
** Alert 111.222:first - group1\n\
2024 Jan 01 00:00:00 /var/log/a\n\
Rule: 100 (level 3) -> 'First alert'\n\
** Alert 333.444:second - group2\n\
2024 Feb 02 11:11:11 /var/log/b\n\
Rule: 200 (level 5) -> 'Second alert'\n\
";

    let libs = Libs::load();
    unsafe {
        let c_get: Symbol<unsafe extern "C" fn(i32, *mut libc::FILE) -> *mut AlertData> =
            libs.c.get(b"GetAlertData").unwrap();
        let r_get: Symbol<unsafe extern "C" fn(i32, *mut libc::FILE) -> *mut AlertData> =
            libs.r.get(b"GetAlertData").unwrap();
        let c_free: Symbol<unsafe extern "C" fn(*mut AlertData)> =
            libs.c.get(b"FreeAlertData").unwrap();
        let r_free: Symbol<unsafe extern "C" fn(*mut AlertData)> =
            libs.r.get(b"FreeAlertData").unwrap();

        let (_tf1, p1) = make_temp_file(content);
        let c_fp = libc::fopen(p1.as_ptr(), b"r\0".as_ptr() as *const i8);
        let c_a1 = c_get(0, c_fp);
        let c_a2 = c_get(0, c_fp);
        libc::fclose(c_fp);

        let (_tf2, p2) = make_temp_file(content);
        let r_fp = libc::fopen(p2.as_ptr(), b"r\0".as_ptr() as *const i8);
        let r_a1 = r_get(0, r_fp);
        let r_a2 = r_get(0, r_fp);
        libc::fclose(r_fp);

        // First alert
        assert!(!c_a1.is_null() && !r_a1.is_null(), "Alert1 null");
        assert_eq!((*c_a1).rule, (*r_a1).rule, "A1 rule");
        assert_eq!((*c_a1).level, (*r_a1).level, "A1 level");
        assert_eq!(str_from_ptr((*c_a1).alertid), str_from_ptr((*r_a1).alertid), "A1 alertid");
        assert_eq!(str_from_ptr((*c_a1).date), str_from_ptr((*r_a1).date), "A1 date");
        assert_eq!(str_from_ptr((*c_a1).comment), str_from_ptr((*r_a1).comment), "A1 comment");

        // Second alert
        assert!(!c_a2.is_null() && !r_a2.is_null(), "Alert2 null");
        assert_eq!((*c_a2).rule, (*r_a2).rule, "A2 rule");
        assert_eq!(str_from_ptr((*c_a2).alertid), str_from_ptr((*r_a2).alertid), "A2 alertid");
        assert_eq!(str_from_ptr((*c_a2).comment), str_from_ptr((*r_a2).comment), "A2 comment");

        c_free(c_a1); c_free(c_a2);
        r_free(r_a1); r_free(r_a2);
    }
}

// ── Test GetAlertData with MAIL flag ──
#[test]
fn test_get_alert_data_mail_flag() {
    let no_mail = "\
** Alert 555.666:test - nomail_group\n\
2024 Apr 10 12:00:00 /var/log/c\n\
Rule: 300 (level 2) -> 'No mail alert'\n\
";
    let with_mail = "\
** Alert 777.888:test mail - mail_group\n\
2024 May 20 14:00:00 /var/log/d\n\
Rule: 400 (level 4) -> 'Mail alert'\n\
";

    let mail_flag = 0x001i32;
    let libs = Libs::load();
    unsafe {
        let c_get: Symbol<unsafe extern "C" fn(i32, *mut libc::FILE) -> *mut AlertData> =
            libs.c.get(b"GetAlertData").unwrap();
        let r_get: Symbol<unsafe extern "C" fn(i32, *mut libc::FILE) -> *mut AlertData> =
            libs.r.get(b"GetAlertData").unwrap();
        let c_free: Symbol<unsafe extern "C" fn(*mut AlertData)> =
            libs.c.get(b"FreeAlertData").unwrap();
        let r_free: Symbol<unsafe extern "C" fn(*mut AlertData)> =
            libs.r.get(b"FreeAlertData").unwrap();

        // No-mail with MAIL flag
        let (_tf1, p1) = make_temp_file(no_mail);
        let c_fp = libc::fopen(p1.as_ptr(), b"r\0".as_ptr() as *const i8);
        let c_r = c_get(mail_flag, c_fp);
        libc::fclose(c_fp);

        let (_tf2, p2) = make_temp_file(no_mail);
        let r_fp = libc::fopen(p2.as_ptr(), b"r\0".as_ptr() as *const i8);
        let r_r = r_get(mail_flag, r_fp);
        libc::fclose(r_fp);

        assert_eq!(c_r.is_null(), r_r.is_null(), "no-mail: C null={} R null={}", c_r.is_null(), r_r.is_null());

        // With mail
        let (_tf3, p3) = make_temp_file(with_mail);
        let c_fp2 = libc::fopen(p3.as_ptr(), b"r\0".as_ptr() as *const i8);
        let c_r2 = c_get(mail_flag, c_fp2);
        libc::fclose(c_fp2);

        let (_tf4, p4) = make_temp_file(with_mail);
        let r_fp2 = libc::fopen(p4.as_ptr(), b"r\0".as_ptr() as *const i8);
        let r_r2 = r_get(mail_flag, r_fp2);
        libc::fclose(r_fp2);

        assert_eq!(c_r2.is_null(), r_r2.is_null(), "mail: C null={} R null={}", c_r2.is_null(), r_r2.is_null());
        if !c_r2.is_null() && !r_r2.is_null() {
            assert_eq!((*c_r2).rule, (*r_r2).rule);
            c_free(c_r2);
            r_free(r_r2);
        }
    }
}

// ── Test Init_FileQueue ──
#[test]
fn test_init_file_queue() {
    let libs = Libs::load();
    unsafe {
        let c_init: Symbol<unsafe extern "C" fn(*mut FileQueue, *const Tm, i32) -> i32> =
            libs.c.get(b"Init_FileQueue").unwrap();
        let r_init: Symbol<unsafe extern "C" fn(*mut FileQueue, *const Tm, i32) -> i32> =
            libs.r.get(b"Init_FileQueue").unwrap();

        let tm = Tm {
            tm_sec: 0, tm_min: 0, tm_hour: 0,
            tm_mday: 15, tm_mon: 2, tm_year: 124,
            tm_wday: 0, tm_yday: 0, tm_isdst: 0,
            tm_gmtoff: 0, tm_zone: ptr::null(),
        };

        let mut c_fq: FileQueue = std::mem::zeroed();
        let mut r_fq: FileQueue = std::mem::zeroed();

        let c_ret = c_init(&mut c_fq, &tm, 0);
        let r_ret = r_init(&mut r_fq, &tm, 0);

        assert_eq!(c_ret, r_ret, "Init return: C={} R={}", c_ret, r_ret);
        assert_eq!(c_fq.day, r_fq.day, "day");
        assert_eq!(c_fq.year, r_fq.year, "year");
        assert_eq!(c_fq.mon, r_fq.mon, "mon");
        assert_eq!(c_fq.flags, r_fq.flags, "flags");
        assert_eq!(
            CStr::from_ptr(c_fq.file_name.as_ptr()),
            CStr::from_ptr(r_fq.file_name.as_ptr()),
            "file_name"
        );

        if !c_fq.fp.is_null() { libc::fclose(c_fq.fp); }
        if !r_fq.fp.is_null() { libc::fclose(r_fq.fp); }
    }
}
