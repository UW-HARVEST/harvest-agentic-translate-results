use libloading::{Library, Symbol};
use std::ffi::{CStr, CString};
use std::ptr;

#[repr(C)]
struct OsData {
    os_name: *mut libc::c_char,
    os_version: *mut libc::c_char,
    os_major: *mut libc::c_char,
    os_minor: *mut libc::c_char,
    os_codename: *mut libc::c_char,
    os_platform: *mut libc::c_char,
    os_build: *mut libc::c_char,
    os_uname: *mut libc::c_char,
    os_arch: *mut libc::c_char,
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

    unsafe fn field_str(&self, p: *mut libc::c_char) -> Option<String> {
        if p.is_null() {
            None
        } else {
            Some(CStr::from_ptr(p).to_string_lossy().into_owned())
        }
    }

    unsafe fn free_fields(&mut self) {
        let ptrs = [
            self.os_name,
            self.os_version,
            self.os_major,
            self.os_minor,
            self.os_codename,
            self.os_platform,
            self.os_build,
            self.os_uname,
            self.os_arch,
        ];
        for p in ptrs {
            if !p.is_null() {
                libc::free(p as *mut libc::c_void);
            }
        }
    }
}

fn c_lib_path() -> String {
    let manifest = env!("CARGO_MANIFEST_DIR");
    format!("{}/c_src/build/libdriver.so", manifest)
}

// ============ Level 1: get_os_arch ============

#[test]
fn test_get_os_arch() {
    let lib = unsafe { Library::new(c_lib_path()).expect("load C lib") };
    let c_get_os_arch: Symbol<unsafe extern "C" fn(*mut libc::c_char) -> *mut libc::c_char> =
        unsafe { lib.get(b"get_os_arch").expect("get_os_arch symbol") };

    let cases = [
        ("Linux 5.4.0-42-generic x86_64", Some("x86_64")),
        ("SunOS 5.11 i86pc", Some("i86pc")),
        ("Darwin 20.6.0 arm64", Some("arm64")),
        ("Linux 5.10.0 aarch64", Some("aarch64")),
        ("Linux 5.10.0 armv7l", Some("armv7")),
        ("AIX 7.2", Some("AIX")),
        ("Linux 5.4.0 unknown", None),
        ("", None),
    ];

    for (input, expected) in &cases {
        let c_input = CString::new(*input).unwrap();
        let c_result = unsafe { (c_get_os_arch)(c_input.into_raw()) };
        let c_str = if c_result.is_null() {
            None
        } else {
            let s = unsafe { CStr::from_ptr(c_result).to_string_lossy().into_owned() };
            unsafe { libc::free(c_result as *mut libc::c_void) };
            Some(s)
        };
        // Note: CString::into_raw leaked, but this is a test

        // Now call Rust's internal get_os_arch via parse_uname_string indirectly
        // We test get_os_arch directly from C only here
        assert_eq!(
            c_str.as_deref(),
            *expected,
            "C get_os_arch mismatch for input: {:?}",
            input
        );
    }
}

// ============ Level 2: w_regexec ============

#[test]
fn test_w_regexec() {
    let lib = unsafe { Library::new(c_lib_path()).expect("load C lib") };
    let c_w_regexec: Symbol<
        unsafe extern "C" fn(
            *const libc::c_char,
            *const libc::c_char,
            libc::size_t,
            *mut libc::regmatch_t,
        ) -> libc::c_int,
    > = unsafe { lib.get(b"w_regexec").expect("w_regexec symbol") };

    // Test: matching pattern
    let pattern = CString::new("^([0-9]+)\\.*").unwrap();
    let input = CString::new("10.2.3").unwrap();
    let mut matches: [libc::regmatch_t; 2] = unsafe { std::mem::zeroed() };
    let result =
        unsafe { (c_w_regexec)(pattern.as_ptr(), input.as_ptr(), 2, matches.as_mut_ptr()) };
    assert_eq!(result, 1, "w_regexec should return 1 for match");
    assert_eq!(matches[1].rm_so, 0);
    assert_eq!(matches[1].rm_eo, 2); // "10"

    // Test: non-matching
    let pattern2 = CString::new("^([0-9]+)\\.*").unwrap();
    let input2 = CString::new("abc").unwrap();
    let mut matches2: [libc::regmatch_t; 2] = unsafe { std::mem::zeroed() };
    let result2 =
        unsafe { (c_w_regexec)(pattern2.as_ptr(), input2.as_ptr(), 2, matches2.as_mut_ptr()) };
    assert_eq!(result2, 0, "w_regexec should return 0 for no match");

    // Test: null pattern
    let result3 =
        unsafe { (c_w_regexec)(ptr::null(), input.as_ptr(), 2, matches.as_mut_ptr()) };
    assert_eq!(result3, 0, "w_regexec should return 0 for null pattern");

    // Test: null string
    let result4 =
        unsafe { (c_w_regexec)(pattern.as_ptr(), ptr::null(), 2, matches.as_mut_ptr()) };
    assert_eq!(result4, 0, "w_regexec should return 0 for null string");
}

// ============ Level 3: parse_uname_string (C vs Rust) ============

type ParseUnameStringFn = unsafe extern "C" fn(*mut libc::c_char, *mut OsData);

fn load_c_parse() -> (Library, *const ()) {
    let lib = unsafe { Library::new(c_lib_path()).expect("load C lib") };
    let sym: Symbol<ParseUnameStringFn> =
        unsafe { lib.get(b"parse_uname_string").expect("parse_uname_string") };
    let ptr = *sym as *const ();
    (lib, ptr)
}

fn load_rust_parse() -> (Library, *const ()) {
    // Find the Rust .so in target/debug/deps or target/debug
    let manifest = env!("CARGO_MANIFEST_DIR");
    let search_dirs = [
        format!("{}/target/debug", manifest),
        format!("{}/target/debug/deps", manifest),
    ];
    for dir in &search_dirs {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with("libdriver") && name.ends_with(".so") {
                        if let Ok(lib) = unsafe { Library::new(path.to_str().unwrap()) } {
                            if let Ok(sym) =
                                unsafe { lib.get::<ParseUnameStringFn>(b"parse_uname_string") }
                            {
                                let ptr = *sym as *const ();
                                return (lib, ptr);
                            }
                        }
                    }
                }
            }
        }
    }
    panic!("Could not find Rust libdriver.so");
}

unsafe fn call_parse(
    func: ParseUnameStringFn,
    input: &str,
) -> (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
) {
    let c_input = CString::new(input).unwrap();
    let raw = c_input.into_raw();
    let mut osd = OsData::zeroed();
    func(raw, &mut osd);
    let result = (
        osd.field_str(osd.os_name),
        osd.field_str(osd.os_version),
        osd.field_str(osd.os_major),
        osd.field_str(osd.os_minor),
        osd.field_str(osd.os_codename),
        osd.field_str(osd.os_platform),
        osd.field_str(osd.os_build),
        osd.field_str(osd.os_uname),
        osd.field_str(osd.os_arch),
    );
    osd.free_fields();
    // Reclaim the CString to free it
    let _ = CString::from_raw(raw);
    result
}

fn compare_parse(input: &str) {
    let (c_lib, c_ptr) = load_c_parse();
    let (r_lib, r_ptr) = load_rust_parse();

    let c_fn: ParseUnameStringFn = unsafe { std::mem::transmute(c_ptr) };
    let r_fn: ParseUnameStringFn = unsafe { std::mem::transmute(r_ptr) };

    let c_result = unsafe { call_parse(c_fn, input) };
    let r_result = unsafe { call_parse(r_fn, input) };

    let fields = [
        "os_name",
        "os_version",
        "os_major",
        "os_minor",
        "os_codename",
        "os_platform",
        "os_build",
        "os_uname",
        "os_arch",
    ];

    let c_vals = [
        &c_result.0,
        &c_result.1,
        &c_result.2,
        &c_result.3,
        &c_result.4,
        &c_result.5,
        &c_result.6,
        &c_result.7,
        &c_result.8,
    ];
    let r_vals = [
        &r_result.0,
        &r_result.1,
        &r_result.2,
        &r_result.3,
        &r_result.4,
        &r_result.5,
        &r_result.6,
        &r_result.7,
        &r_result.8,
    ];

    for i in 0..9 {
        assert_eq!(
            c_vals[i], r_vals[i],
            "Field '{}' mismatch for input {:?}: C={:?}, Rust={:?}",
            fields[i], input, c_vals[i], r_vals[i]
        );
    }

    drop(c_lib);
    drop(r_lib);
}

#[test]
fn test_parse_windows() {
    compare_parse("Microsoft Windows 10 Pro [Ver: 10.0.19041.1234]");
}

#[test]
fn test_parse_linux() {
    compare_parse("Linux 5.4.0-42-generic x86_64 [Ubuntu|Linux: 20.04.1 (focal)]");
}

#[test]
fn test_parse_linux_no_codename() {
    compare_parse("Linux 5.4.0 x86_64 [CentOS|Linux: 7.9]");
}

#[test]
fn test_parse_linux_no_version() {
    compare_parse("Linux 5.4.0 x86_64 [Debian]");
}

#[test]
fn test_parse_linux_pipe_platform() {
    compare_parse("Linux 5.4.0 aarch64 [Amazon|Linux: 2]");
}

#[test]
fn test_parse_null_osd() {
    // Just ensure no crash
    let lib = unsafe { Library::new(c_lib_path()).expect("load C lib") };
    let c_fn: Symbol<ParseUnameStringFn> =
        unsafe { lib.get(b"parse_uname_string").expect("sym") };
    let input = CString::new("test").unwrap();
    unsafe { (c_fn)(input.into_raw(), ptr::null_mut()) };
    // If we get here, no crash
}

#[test]
fn test_parse_no_bracket() {
    compare_parse("Linux 5.4.0-42-generic x86_64");
}

#[test]
fn test_parse_windows_simple() {
    compare_parse("Windows 10 [Ver: 6.1.7601]");
}

#[test]
fn test_parse_aix() {
    compare_parse("AIX 7.2 [AIX: 7.2]");
}
