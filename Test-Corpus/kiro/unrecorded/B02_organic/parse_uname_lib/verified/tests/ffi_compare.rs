use libloading::{Library, Symbol};
use std::ffi::{CStr, CString};
use std::ptr;

const C_LIB: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/libdriver.so");
const RUST_LIB: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/target/debug/libdriver.so");

#[repr(C)]
#[derive(Clone)]
struct regmatch_t {
    rm_so: i32,
    rm_eo: i32,
}

#[repr(C)]
struct OsData {
    os_name: *mut i8,
    os_version: *mut i8,
    os_major: *mut i8,
    os_minor: *mut i8,
    os_codename: *mut i8,
    os_platform: *mut i8,
    os_build: *mut i8,
    os_uname: *mut i8,
    os_arch: *mut i8,
}

impl OsData {
    fn zeroed() -> Self {
        Self {
            os_name: ptr::null_mut(),
            os_version: ptr::null_mut(),
            os_major: ptr::null_mut(),
            os_minor: ptr::null_mut(),
            os_codename: ptr::null_mut(),
            os_platform: ptr::null_mut(),
            os_build: ptr::null_mut(),
            os_uname: ptr::null_mut(),
            os_arch: ptr::null_mut(),
        }
    }
}

unsafe fn ptr_to_opt_string(p: *mut i8) -> Option<String> {
    if p.is_null() {
        None
    } else {
        Some(CStr::from_ptr(p).to_string_lossy().into_owned())
    }
}

unsafe fn free_os_data(osd: &mut OsData) {
    for p in [
        &mut osd.os_name,
        &mut osd.os_version,
        &mut osd.os_major,
        &mut osd.os_minor,
        &mut osd.os_codename,
        &mut osd.os_platform,
        &mut osd.os_build,
        &mut osd.os_uname,
        &mut osd.os_arch,
    ] {
        if !(*p).is_null() {
            libc::free(*p as *mut _);
            *p = ptr::null_mut();
        }
    }
}

// ==================== get_os_arch tests ====================

fn compare_get_os_arch(input: &str) {
    let c_lib = unsafe { Library::new(C_LIB).expect("load C lib") };
    let rust_lib = unsafe { Library::new(RUST_LIB).expect("load Rust lib") };

    let cs = CString::new(input).unwrap();

    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(*mut i8) -> *mut i8> =
            c_lib.get(b"get_os_arch").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(*mut i8) -> *mut i8> =
            rust_lib.get(b"get_os_arch").unwrap();

        // Need separate mutable copies since the function takes *mut
        let mut c_buf = cs.as_bytes_with_nul().to_vec();
        let mut r_buf = cs.as_bytes_with_nul().to_vec();

        let c_res = c_fn(c_buf.as_mut_ptr() as *mut i8);
        let r_res = r_fn(r_buf.as_mut_ptr() as *mut i8);

        let c_str = ptr_to_opt_string(c_res);
        let r_str = ptr_to_opt_string(r_res);

        assert_eq!(c_str, r_str, "get_os_arch mismatch for input: {:?}", input);

        if !c_res.is_null() { libc::free(c_res as *mut _); }
        if !r_res.is_null() { libc::free(r_res as *mut _); }
    }
}

#[test]
fn test_get_os_arch_x86_64() { compare_get_os_arch("Linux 5.4.0 x86_64"); }
#[test]
fn test_get_os_arch_i386() { compare_get_os_arch("Linux 5.4.0 i386"); }
#[test]
fn test_get_os_arch_i686() { compare_get_os_arch("Linux 5.4.0 i686"); }
#[test]
fn test_get_os_arch_aarch64() { compare_get_os_arch("Linux 5.4.0 aarch64"); }
#[test]
fn test_get_os_arch_arm64() { compare_get_os_arch("Linux 5.4.0 arm64"); }
#[test]
fn test_get_os_arch_amd64() { compare_get_os_arch("FreeBSD 13.0 amd64"); }
#[test]
fn test_get_os_arch_sparc() { compare_get_os_arch("SunOS 5.11 sparc"); }
#[test]
fn test_get_os_arch_ia64() { compare_get_os_arch("HP-UX ia64"); }
#[test]
fn test_get_os_arch_aix() { compare_get_os_arch("AIX 7.2"); }
#[test]
fn test_get_os_arch_armv6() { compare_get_os_arch("Linux armv6l"); }
#[test]
fn test_get_os_arch_armv7() { compare_get_os_arch("Linux armv7l"); }
#[test]
fn test_get_os_arch_i86pc() { compare_get_os_arch("SunOS 5.11 i86pc"); }
#[test]
fn test_get_os_arch_none() { compare_get_os_arch("Linux 5.4.0 unknown"); }
#[test]
fn test_get_os_arch_empty() { compare_get_os_arch(""); }
#[test]
fn test_get_os_arch_priority() { compare_get_os_arch("x86_64 aarch64"); }

// ==================== w_regexec tests ====================

fn compare_w_regexec(pattern: &str, input: &str, nmatch: usize) {
    let c_lib = unsafe { Library::new(C_LIB).expect("load C lib") };
    let rust_lib = unsafe { Library::new(RUST_LIB).expect("load Rust lib") };

    let pat = CString::new(pattern).unwrap();
    let inp = CString::new(input).unwrap();

    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(*const i8, *const i8, usize, *mut regmatch_t) -> i32> =
            c_lib.get(b"w_regexec").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(*const i8, *const i8, usize, *mut regmatch_t) -> i32> =
            rust_lib.get(b"w_regexec").unwrap();

        let mut c_matches = vec![regmatch_t { rm_so: -1, rm_eo: -1 }; nmatch];
        let mut r_matches = vec![regmatch_t { rm_so: -1, rm_eo: -1 }; nmatch];

        let c_ret = c_fn(pat.as_ptr(), inp.as_ptr(), nmatch, c_matches.as_mut_ptr());
        let r_ret = r_fn(pat.as_ptr(), inp.as_ptr(), nmatch, r_matches.as_mut_ptr());

        assert_eq!(c_ret, r_ret, "w_regexec return mismatch for pattern={:?} input={:?}", pattern, input);

        if c_ret != 0 {
            for i in 0..nmatch {
                assert_eq!(
                    (c_matches[i].rm_so, c_matches[i].rm_eo),
                    (r_matches[i].rm_so, r_matches[i].rm_eo),
                    "w_regexec match[{}] mismatch for pattern={:?} input={:?}",
                    i, pattern, input
                );
            }
        }
    }
}

#[test]
fn test_w_regexec_major() { compare_w_regexec(r"^([0-9]+)\.*", "10.0.19041", 2); }
#[test]
fn test_w_regexec_minor() { compare_w_regexec(r"^[0-9]+\.([0-9]+)\.*", "10.0.19041", 2); }
#[test]
fn test_w_regexec_build() { compare_w_regexec(r"^[0-9]+\.[0-9]+\.([0-9]+(\.[0-9]+)*)\.*", "10.0.19041", 2); }
#[test]
fn test_w_regexec_no_match() { compare_w_regexec(r"^([0-9]+)\.*", "abc", 2); }
#[test]
fn test_w_regexec_null_pattern() {
    let c_lib = unsafe { Library::new(C_LIB).expect("load C lib") };
    let rust_lib = unsafe { Library::new(RUST_LIB).expect("load Rust lib") };
    let inp = CString::new("test").unwrap();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(*const i8, *const i8, usize, *mut regmatch_t) -> i32> =
            c_lib.get(b"w_regexec").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(*const i8, *const i8, usize, *mut regmatch_t) -> i32> =
            rust_lib.get(b"w_regexec").unwrap();
        let mut cm = [regmatch_t { rm_so: -1, rm_eo: -1 }];
        let mut rm = [regmatch_t { rm_so: -1, rm_eo: -1 }];
        let c_ret = c_fn(ptr::null(), inp.as_ptr(), 1, cm.as_mut_ptr());
        let r_ret = r_fn(ptr::null(), inp.as_ptr(), 1, rm.as_mut_ptr());
        assert_eq!(c_ret, r_ret, "null pattern mismatch");
    }
}
#[test]
fn test_w_regexec_null_string() {
    let c_lib = unsafe { Library::new(C_LIB).expect("load C lib") };
    let rust_lib = unsafe { Library::new(RUST_LIB).expect("load Rust lib") };
    let pat = CString::new("test").unwrap();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(*const i8, *const i8, usize, *mut regmatch_t) -> i32> =
            c_lib.get(b"w_regexec").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(*const i8, *const i8, usize, *mut regmatch_t) -> i32> =
            rust_lib.get(b"w_regexec").unwrap();
        let mut cm = [regmatch_t { rm_so: -1, rm_eo: -1 }];
        let mut rm = [regmatch_t { rm_so: -1, rm_eo: -1 }];
        let c_ret = c_fn(pat.as_ptr(), ptr::null(), 1, cm.as_mut_ptr());
        let r_ret = r_fn(pat.as_ptr(), ptr::null(), 1, rm.as_mut_ptr());
        assert_eq!(c_ret, r_ret, "null string mismatch");
    }
}
#[test]
fn test_w_regexec_version_parts() {
    compare_w_regexec(r"^([0-9]+)\.*", "6.3", 2);
    compare_w_regexec(r"^[0-9]+\.([0-9]+)\.*", "6.3", 2);
    compare_w_regexec(r"^[0-9]+\.[0-9]+\.([0-9]+(\.[0-9]+)*)\.*", "6.3", 2);
}
#[test]
fn test_w_regexec_multi_build() {
    compare_w_regexec(r"^[0-9]+\.[0-9]+\.([0-9]+(\.[0-9]+)*)\.*", "10.0.19041.1234", 2);
}

// ==================== parse_uname_string tests ====================

struct ParseResult {
    os_name: Option<String>,
    os_version: Option<String>,
    os_major: Option<String>,
    os_minor: Option<String>,
    os_codename: Option<String>,
    os_platform: Option<String>,
    os_build: Option<String>,
    os_uname: Option<String>,
    os_arch: Option<String>,
}

fn call_parse_uname(lib: &Library, input: &str) -> ParseResult {
    let mut buf = input.as_bytes().to_vec();
    buf.push(0);

    unsafe {
        let func: Symbol<unsafe extern "C" fn(*mut i8, *mut OsData)> =
            lib.get(b"parse_uname_string").unwrap();

        let mut osd = OsData::zeroed();
        func(buf.as_mut_ptr() as *mut i8, &mut osd);

        let result = ParseResult {
            os_name: ptr_to_opt_string(osd.os_name),
            os_version: ptr_to_opt_string(osd.os_version),
            os_major: ptr_to_opt_string(osd.os_major),
            os_minor: ptr_to_opt_string(osd.os_minor),
            os_codename: ptr_to_opt_string(osd.os_codename),
            os_platform: ptr_to_opt_string(osd.os_platform),
            os_build: ptr_to_opt_string(osd.os_build),
            os_uname: ptr_to_opt_string(osd.os_uname),
            os_arch: ptr_to_opt_string(osd.os_arch),
        };

        free_os_data(&mut osd);
        result
    }
}

fn compare_parse_uname(input: &str) {
    let c_lib = unsafe { Library::new(C_LIB).expect("load C lib") };
    let rust_lib = unsafe { Library::new(RUST_LIB).expect("load Rust lib") };

    let c = call_parse_uname(&c_lib, input);
    let r = call_parse_uname(&rust_lib, input);

    assert_eq!(c.os_name, r.os_name, "os_name mismatch for {:?}", input);
    assert_eq!(c.os_version, r.os_version, "os_version mismatch for {:?}", input);
    assert_eq!(c.os_major, r.os_major, "os_major mismatch for {:?}", input);
    assert_eq!(c.os_minor, r.os_minor, "os_minor mismatch for {:?}", input);
    assert_eq!(c.os_codename, r.os_codename, "os_codename mismatch for {:?}", input);
    assert_eq!(c.os_platform, r.os_platform, "os_platform mismatch for {:?}", input);
    assert_eq!(c.os_build, r.os_build, "os_build mismatch for {:?}", input);
    assert_eq!(c.os_uname, r.os_uname, "os_uname mismatch for {:?}", input);
    assert_eq!(c.os_arch, r.os_arch, "os_arch mismatch for {:?}", input);
}

// Windows path tests
#[test]
fn test_parse_windows_basic() {
    compare_parse_uname("Microsoft Windows 10 [Ver: 10.0.19041]");
}

#[test]
fn test_parse_windows_server() {
    compare_parse_uname("Microsoft Windows Server 2019 [Ver: 10.0.17763]");
}

#[test]
fn test_parse_windows_multi_build() {
    compare_parse_uname("Windows 11 [Ver: 10.0.22000.1234]");
}

// Linux path tests
#[test]
fn test_parse_linux_ubuntu() {
    compare_parse_uname("Linux 5.4.0-42-generic x86_64 [Ubuntu|deb: 20.04.1 (focal)]");
}

#[test]
fn test_parse_linux_centos() {
    compare_parse_uname("Linux 3.10.0-1127.el7.x86_64 x86_64 [CentOS Linux|rpm: 7.8]");
}

#[test]
fn test_parse_linux_no_codename() {
    compare_parse_uname("Linux 5.4.0 x86_64 [Debian|deb: 10.9]");
}

#[test]
fn test_parse_linux_no_version() {
    compare_parse_uname("Linux 5.4.0 x86_64 [SomeOS]");
}

#[test]
fn test_parse_linux_aarch64() {
    compare_parse_uname("Linux 5.15.0 aarch64 [Ubuntu|deb: 22.04 (jammy)]");
}

// No bracket at all
#[test]
fn test_parse_no_bracket() {
    compare_parse_uname("Linux 5.4.0 x86_64");
}

// Null osd test
#[test]
fn test_parse_null_osd() {
    let c_lib = unsafe { Library::new(C_LIB).expect("load C lib") };
    let rust_lib = unsafe { Library::new(RUST_LIB).expect("load Rust lib") };
    let mut buf = b"test\0".to_vec();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(*mut i8, *mut OsData)> =
            c_lib.get(b"parse_uname_string").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(*mut i8, *mut OsData)> =
            rust_lib.get(b"parse_uname_string").unwrap();
        // Should not crash
        c_fn(buf.as_mut_ptr() as *mut i8, ptr::null_mut());
        r_fn(buf.as_mut_ptr() as *mut i8, ptr::null_mut());
    }
}

#[test]
fn test_parse_linux_with_pipe_platform() {
    compare_parse_uname("Linux 4.15.0 i686 [RedHat|rpm: 8.3 (Ootpa)]");
}

#[test]
fn test_parse_windows_simple_version() {
    compare_parse_uname("Windows XP [Ver: 5.1.2600]");
}
