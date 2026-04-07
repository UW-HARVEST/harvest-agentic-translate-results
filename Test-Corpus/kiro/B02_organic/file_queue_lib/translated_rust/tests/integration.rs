use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_uint, c_void, CStr, CString};
use std::io::Write;
use std::ptr;

// Mirror of the C alert_data struct
#[repr(C)]
struct AlertData {
    rule: c_uint,
    level: c_uint,
    alertid: *mut c_char,
    date: *mut c_char,
    location: *mut c_char,
    comment: *mut c_char,
    group: *mut c_char,
    srcip: *mut c_char,
    srcport: c_int,
    dstip: *mut c_char,
    dstport: c_int,
    user: *mut c_char,
    filename: *mut c_char,
}

#[repr(C)]
struct FileQueue {
    last_change: i64,
    year: c_int,
    day: c_int,
    flags: c_int,
    mon: [c_char; 4],
    file_name: [c_char; 257],
    fp: *mut c_void,
    f_status: [u8; 144], // struct stat
}

#[repr(C)]
struct Tm {
    tm_sec: c_int,
    tm_min: c_int,
    tm_hour: c_int,
    tm_mday: c_int,
    tm_mon: c_int,
    tm_year: c_int,
    tm_wday: c_int,
    tm_yday: c_int,
    tm_isdst: c_int,
    tm_gmtoff: i64,
    tm_zone: *const c_char,
}

extern "C" {
    fn fopen(path: *const c_char, mode: *const c_char) -> *mut c_void;
    fn fclose(fp: *mut c_void) -> c_int;
    fn free(ptr: *mut c_void);
}

type GetAlertDataFn = unsafe extern "C" fn(c_int, *mut c_void) -> *mut AlertData;
type FreeAlertDataFn = unsafe extern "C" fn(*mut AlertData);
type InitFileQueueFn = unsafe extern "C" fn(*mut FileQueue, *const Tm, c_int) -> c_int;
type DriverFn = unsafe extern "C" fn(c_int, c_int, c_int, c_uint, c_int) -> *mut AlertData;

fn c_lib_path() -> String {
    std::fs::canonicalize("c_src/build/libdriver.so")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string()
}

fn rust_lib_path() -> String {
    std::fs::canonicalize("target/debug/libdriver.so")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string()
}

unsafe fn str_field(p: *mut c_char) -> Option<String> {
    if p.is_null() {
        None
    } else {
        Some(CStr::from_ptr(p).to_string_lossy().into_owned())
    }
}

fn write_temp_alert(content: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir();
    let path = dir.join(format!("alert_test_{}.log", std::process::id()));
    let mut f = std::fs::File::create(&path).unwrap();
    f.write_all(content.as_bytes()).unwrap();
    path
}

unsafe fn compare_alert(c_al: *mut AlertData, r_al: *mut AlertData, label: &str) {
    assert_eq!(
        c_al.is_null(),
        r_al.is_null(),
        "{label}: null mismatch c={} r={}",
        c_al.is_null(),
        r_al.is_null()
    );
    if c_al.is_null() {
        return;
    }
    let c = &*c_al;
    let r = &*r_al;
    assert_eq!(c.rule, r.rule, "{label}: rule mismatch");
    assert_eq!(c.level, r.level, "{label}: level mismatch");
    assert_eq!(str_field(c.alertid), str_field(r.alertid), "{label}: alertid");
    assert_eq!(str_field(c.date), str_field(r.date), "{label}: date");
    assert_eq!(str_field(c.location), str_field(r.location), "{label}: location");
    assert_eq!(str_field(c.comment), str_field(r.comment), "{label}: comment");
    assert_eq!(str_field(c.group), str_field(r.group), "{label}: group");
    assert_eq!(str_field(c.srcip), str_field(r.srcip), "{label}: srcip");
    assert_eq!(c.srcport, r.srcport, "{label}: srcport");
    assert_eq!(str_field(c.dstip), str_field(r.dstip), "{label}: dstip");
    assert_eq!(c.dstport, r.dstport, "{label}: dstport");
    assert_eq!(str_field(c.user), str_field(r.user), "{label}: user");
    assert_eq!(str_field(c.filename), str_field(r.filename), "{label}: filename");
}

// ---- Tests ----

#[test]
fn test_get_alert_data_empty_file() {
    let path = write_temp_alert("");
    let c_path = CString::new(path.to_str().unwrap()).unwrap();
    let mode = CString::new("r").unwrap();

    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();
        let c_get: Symbol<GetAlertDataFn> = c_lib.get(b"GetAlertData").unwrap();
        let r_get: Symbol<GetAlertDataFn> = r_lib.get(b"GetAlertData").unwrap();
        let c_free: Symbol<FreeAlertDataFn> = c_lib.get(b"FreeAlertData").unwrap();
        let r_free: Symbol<FreeAlertDataFn> = r_lib.get(b"FreeAlertData").unwrap();

        let fp_c = fopen(c_path.as_ptr(), mode.as_ptr());
        let fp_r = fopen(c_path.as_ptr(), mode.as_ptr());

        let c_al = c_get(0, fp_c);
        let r_al = r_get(0, fp_r);

        compare_alert(c_al, r_al, "empty_file");

        if !c_al.is_null() { c_free(c_al); }
        if !r_al.is_null() { r_free(r_al); }
        fclose(fp_c);
        fclose(fp_r);
    }
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_get_alert_data_single_alert() {
    let content = "\
** Alert 1234567890.123:abc - syscheck\n\
2024 Apr 13 16:15:17 /var/log/auth.log\n\
Rule: 550 (level 7) -> 'Test alert comment'\n\
Src IP: 192.168.1.1\n\
Src Port: 8080\n\
Dst IP: 10.0.0.1\n\
Dst Port: 443\n\
User: testuser\n\
Integrity checksum changed for: '/etc/passwd'\n\
";
    let path = write_temp_alert(content);
    let c_path = CString::new(path.to_str().unwrap()).unwrap();
    let mode = CString::new("r").unwrap();

    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();
        let c_get: Symbol<GetAlertDataFn> = c_lib.get(b"GetAlertData").unwrap();
        let r_get: Symbol<GetAlertDataFn> = r_lib.get(b"GetAlertData").unwrap();
        let c_free: Symbol<FreeAlertDataFn> = c_lib.get(b"FreeAlertData").unwrap();
        let r_free: Symbol<FreeAlertDataFn> = r_lib.get(b"FreeAlertData").unwrap();

        let fp_c = fopen(c_path.as_ptr(), mode.as_ptr());
        let fp_r = fopen(c_path.as_ptr(), mode.as_ptr());

        let c_al = c_get(0, fp_c);
        let r_al = r_get(0, fp_r);

        compare_alert(c_al, r_al, "single_alert");

        if !c_al.is_null() { c_free(c_al); }
        if !r_al.is_null() { r_free(r_al); }
        fclose(fp_c);
        fclose(fp_r);
    }
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_get_alert_data_two_alerts() {
    let content = "\
** Alert 111.1:first - group1\n\
2024 Jan 01 00:00:00 /var/log/syslog\n\
Rule: 100 (level 3) -> 'First alert'\n\
Src IP: 1.2.3.4\n\
** Alert 222.2:second - group2\n\
2024 Feb 02 12:30:00 /var/log/messages\n\
Rule: 200 (level 5) -> 'Second alert'\n\
User: admin\n\
";
    let path = write_temp_alert(content);
    let c_path = CString::new(path.to_str().unwrap()).unwrap();
    let mode = CString::new("r").unwrap();

    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();
        let c_get: Symbol<GetAlertDataFn> = c_lib.get(b"GetAlertData").unwrap();
        let r_get: Symbol<GetAlertDataFn> = r_lib.get(b"GetAlertData").unwrap();
        let c_free: Symbol<FreeAlertDataFn> = c_lib.get(b"FreeAlertData").unwrap();
        let r_free: Symbol<FreeAlertDataFn> = r_lib.get(b"FreeAlertData").unwrap();

        // First alert
        let fp_c = fopen(c_path.as_ptr(), mode.as_ptr());
        let fp_r = fopen(c_path.as_ptr(), mode.as_ptr());

        let c_al1 = c_get(0, fp_c);
        let r_al1 = r_get(0, fp_r);
        compare_alert(c_al1, r_al1, "two_alerts_first");

        // Second alert (continue reading from same fp)
        let c_al2 = c_get(0, fp_c);
        let r_al2 = r_get(0, fp_r);
        compare_alert(c_al2, r_al2, "two_alerts_second");

        if !c_al1.is_null() { c_free(c_al1); }
        if !r_al1.is_null() { r_free(r_al1); }
        if !c_al2.is_null() { c_free(c_al2); }
        if !r_al2.is_null() { r_free(r_al2); }
        fclose(fp_c);
        fclose(fp_r);
    }
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_get_alert_data_mail_flag() {
    // With CRALERT_MAIL_SET (0x001), only alerts with "mail" flag should be returned
    let content = "\
** Alert 111.1:first mail - group1\n\
2024 Jan 01 00:00:00 /var/log/syslog\n\
Rule: 100 (level 3) -> 'Mail alert'\n\
";
    let path = write_temp_alert(content);
    let c_path = CString::new(path.to_str().unwrap()).unwrap();
    let mode = CString::new("r").unwrap();

    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();
        let c_get: Symbol<GetAlertDataFn> = c_lib.get(b"GetAlertData").unwrap();
        let r_get: Symbol<GetAlertDataFn> = r_lib.get(b"GetAlertData").unwrap();
        let c_free: Symbol<FreeAlertDataFn> = c_lib.get(b"FreeAlertData").unwrap();
        let r_free: Symbol<FreeAlertDataFn> = r_lib.get(b"FreeAlertData").unwrap();

        let fp_c = fopen(c_path.as_ptr(), mode.as_ptr());
        let fp_r = fopen(c_path.as_ptr(), mode.as_ptr());

        let c_al = c_get(0x001, fp_c); // CRALERT_MAIL_SET
        let r_al = r_get(0x001, fp_r);
        compare_alert(c_al, r_al, "mail_flag");

        if !c_al.is_null() { c_free(c_al); }
        if !r_al.is_null() { r_free(r_al); }
        fclose(fp_c);
        fclose(fp_r);
    }
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_get_alert_data_no_mail_skips() {
    // CRALERT_MAIL_SET but alert doesn't have "mail" - should skip and return NULL
    let content = "\
** Alert 111.1:first nomatch - group1\n\
2024 Jan 01 00:00:00 /var/log/syslog\n\
Rule: 100 (level 3) -> 'No mail alert'\n\
";
    let path = write_temp_alert(content);
    let c_path = CString::new(path.to_str().unwrap()).unwrap();
    let mode = CString::new("r").unwrap();

    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();
        let c_get: Symbol<GetAlertDataFn> = c_lib.get(b"GetAlertData").unwrap();
        let r_get: Symbol<GetAlertDataFn> = r_lib.get(b"GetAlertData").unwrap();
        let c_free: Symbol<FreeAlertDataFn> = c_lib.get(b"FreeAlertData").unwrap();
        let r_free: Symbol<FreeAlertDataFn> = r_lib.get(b"FreeAlertData").unwrap();

        let fp_c = fopen(c_path.as_ptr(), mode.as_ptr());
        let fp_r = fopen(c_path.as_ptr(), mode.as_ptr());

        let c_al = c_get(0x001, fp_c);
        let r_al = r_get(0x001, fp_r);
        compare_alert(c_al, r_al, "no_mail_skips");

        if !c_al.is_null() { c_free(c_al); }
        if !r_al.is_null() { r_free(r_al); }
        fclose(fp_c);
        fclose(fp_r);
    }
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_get_alert_data_garbage_input() {
    let content = "this is not an alert\njust random text\n";
    let path = write_temp_alert(content);
    let c_path = CString::new(path.to_str().unwrap()).unwrap();
    let mode = CString::new("r").unwrap();

    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();
        let c_get: Symbol<GetAlertDataFn> = c_lib.get(b"GetAlertData").unwrap();
        let r_get: Symbol<GetAlertDataFn> = r_lib.get(b"GetAlertData").unwrap();

        let fp_c = fopen(c_path.as_ptr(), mode.as_ptr());
        let fp_r = fopen(c_path.as_ptr(), mode.as_ptr());

        let c_al = c_get(0, fp_c);
        let r_al = r_get(0, fp_r);
        compare_alert(c_al, r_al, "garbage_input");

        fclose(fp_c);
        fclose(fp_r);
    }
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_get_alert_data_syscheck_filename() {
    let content = "\
** Alert 999.9:check - syscheck\n\
2024 Mar 15 10:00:00 /var/log/syslog\n\
Rule: 550 (level 7) -> 'File changed'\n\
Integrity checksum changed for: '/etc/shadow'\n\
";
    let path = write_temp_alert(content);
    let c_path = CString::new(path.to_str().unwrap()).unwrap();
    let mode = CString::new("r").unwrap();

    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();
        let c_get: Symbol<GetAlertDataFn> = c_lib.get(b"GetAlertData").unwrap();
        let r_get: Symbol<GetAlertDataFn> = r_lib.get(b"GetAlertData").unwrap();
        let c_free: Symbol<FreeAlertDataFn> = c_lib.get(b"FreeAlertData").unwrap();
        let r_free: Symbol<FreeAlertDataFn> = r_lib.get(b"FreeAlertData").unwrap();

        let fp_c = fopen(c_path.as_ptr(), mode.as_ptr());
        let fp_r = fopen(c_path.as_ptr(), mode.as_ptr());

        let c_al = c_get(0, fp_c);
        let r_al = r_get(0, fp_r);
        compare_alert(c_al, r_al, "syscheck_filename");

        if !c_al.is_null() { c_free(c_al); }
        if !r_al.is_null() { r_free(r_al); }
        fclose(fp_c);
        fclose(fp_r);
    }
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_init_file_queue_nonexistent() {
    // Init with a file that doesn't exist - should succeed (Handle_Queue returns 0, not <0)
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();
        let c_init: Symbol<InitFileQueueFn> = c_lib.get(b"Init_FileQueue").unwrap();
        let r_init: Symbol<InitFileQueueFn> = r_lib.get(b"Init_FileQueue").unwrap();

        let tm = Tm {
            tm_sec: 0, tm_min: 0, tm_hour: 0,
            tm_mday: 15, tm_mon: 3, tm_year: 124,
            tm_wday: 0, tm_yday: 0, tm_isdst: 0,
            tm_gmtoff: 0, tm_zone: ptr::null(),
        };

        let mut c_fq: FileQueue = std::mem::zeroed();
        let mut r_fq: FileQueue = std::mem::zeroed();

        let c_ret = c_init(&mut c_fq, &tm, 0);
        let r_ret = r_init(&mut r_fq, &tm, 0);

        assert_eq!(c_ret, r_ret, "Init_FileQueue return value mismatch");
        assert_eq!(c_fq.day, r_fq.day, "day mismatch");
        assert_eq!(c_fq.year, r_fq.year, "year mismatch");
        assert_eq!(c_fq.flags, r_fq.flags, "flags mismatch");
        assert_eq!(c_fq.mon, r_fq.mon, "mon mismatch");
    }
}

#[test]
fn test_init_file_queue_with_fp_set() {
    // With CRALERT_FP_SET flag, file_name should be "<stdin>"
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();
        let c_init: Symbol<InitFileQueueFn> = c_lib.get(b"Init_FileQueue").unwrap();
        let r_init: Symbol<InitFileQueueFn> = r_lib.get(b"Init_FileQueue").unwrap();

        let tm = Tm {
            tm_sec: 0, tm_min: 0, tm_hour: 0,
            tm_mday: 1, tm_mon: 0, tm_year: 124,
            tm_wday: 0, tm_yday: 0, tm_isdst: 0,
            tm_gmtoff: 0, tm_zone: ptr::null(),
        };

        // Need a real FILE* for FP_SET path - use /dev/null
        let devnull = CString::new("/dev/null").unwrap();
        let rmode = CString::new("r").unwrap();
        let fp1 = fopen(devnull.as_ptr(), rmode.as_ptr());
        let fp2 = fopen(devnull.as_ptr(), rmode.as_ptr());

        let mut c_fq: FileQueue = std::mem::zeroed();
        let mut r_fq: FileQueue = std::mem::zeroed();
        c_fq.fp = fp1;
        r_fq.fp = fp2;

        let c_ret = c_init(&mut c_fq, &tm, 0x010); // CRALERT_FP_SET
        let r_ret = r_init(&mut r_fq, &tm, 0x010);

        assert_eq!(c_ret, r_ret, "Init_FileQueue FP_SET return mismatch");
        assert_eq!(c_fq.flags, r_fq.flags, "flags mismatch");
        // Compare file_name
        assert_eq!(c_fq.file_name[..7], r_fq.file_name[..7], "file_name mismatch");

        fclose(fp1);
        fclose(fp2);
    }
}

#[test]
fn test_driver_nonexistent_file() {
    // driver() with default flags should return NULL since alerts.log doesn't exist
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();
        let c_driver: Symbol<DriverFn> = c_lib.get(b"driver").unwrap();
        let r_driver: Symbol<DriverFn> = r_lib.get(b"driver").unwrap();

        let c_ret = c_driver(15, 3, 124, 0, 0);
        let r_ret = r_driver(15, 3, 124, 0, 0);

        assert_eq!(c_ret.is_null(), r_ret.is_null(), "driver NULL mismatch");
    }
}

#[test]
fn test_get_alert_data_partial_alert() {
    // Alert header only, no date/location line - should return NULL
    let content = "** Alert 111.1:first - group1\n";
    let path = write_temp_alert(content);
    let c_path = CString::new(path.to_str().unwrap()).unwrap();
    let mode = CString::new("r").unwrap();

    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();
        let c_get: Symbol<GetAlertDataFn> = c_lib.get(b"GetAlertData").unwrap();
        let r_get: Symbol<GetAlertDataFn> = r_lib.get(b"GetAlertData").unwrap();

        let fp_c = fopen(c_path.as_ptr(), mode.as_ptr());
        let fp_r = fopen(c_path.as_ptr(), mode.as_ptr());

        let c_al = c_get(0, fp_c);
        let r_al = r_get(0, fp_r);
        compare_alert(c_al, r_al, "partial_alert");

        fclose(fp_c);
        fclose(fp_r);
    }
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_get_alert_data_dstip_dstport() {
    let content = "\
** Alert 555.5:net - network\n\
2024 Jun 20 08:30:00 /var/log/firewall\n\
Rule: 300 (level 10) -> 'Network alert'\n\
Src IP: 10.0.0.1\n\
Src Port: 12345\n\
Dst IP: 192.168.0.1\n\
Dst Port: 80\n\
User: root\n\
";
    let path = write_temp_alert(content);
    let c_path = CString::new(path.to_str().unwrap()).unwrap();
    let mode = CString::new("r").unwrap();

    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();
        let c_get: Symbol<GetAlertDataFn> = c_lib.get(b"GetAlertData").unwrap();
        let r_get: Symbol<GetAlertDataFn> = r_lib.get(b"GetAlertData").unwrap();
        let c_free: Symbol<FreeAlertDataFn> = c_lib.get(b"FreeAlertData").unwrap();
        let r_free: Symbol<FreeAlertDataFn> = r_lib.get(b"FreeAlertData").unwrap();

        let fp_c = fopen(c_path.as_ptr(), mode.as_ptr());
        let fp_r = fopen(c_path.as_ptr(), mode.as_ptr());

        let c_al = c_get(0, fp_c);
        let r_al = r_get(0, fp_r);
        compare_alert(c_al, r_al, "dstip_dstport");

        if !c_al.is_null() { c_free(c_al); }
        if !r_al.is_null() { r_free(r_al); }
        fclose(fp_c);
        fclose(fp_r);
    }
    std::fs::remove_file(&path).ok();
}
