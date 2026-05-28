//! Integration tests that compare the C-built shared library with the
//! Rust-built shared library through their `extern "C"` boundaries.

use libloading::{Library, Symbol};
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};

const C_SO: &str = "c_src/build/libdriver.so";
const RUST_SO: &str = "target/debug/libdriver.so";

#[repr(C)]
#[derive(Default, Debug)]
pub struct OsData {
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

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct Regmatch {
    pub rm_so: libc::regoff_t,
    pub rm_eo: libc::regoff_t,
}

fn load_libs() -> (Library, Library) {
    unsafe {
        let c = Library::new(C_SO).expect("load C .so");
        let r = Library::new(RUST_SO).expect("load Rust .so");
        (c, r)
    }
}

fn cstr_to_owned(p: *const c_char) -> Option<String> {
    if p.is_null() {
        None
    } else {
        Some(unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned())
    }
}

unsafe fn empty_osd() -> OsData {
    OsData {
        os_name: std::ptr::null_mut(),
        os_version: std::ptr::null_mut(),
        os_major: std::ptr::null_mut(),
        os_minor: std::ptr::null_mut(),
        os_codename: std::ptr::null_mut(),
        os_platform: std::ptr::null_mut(),
        os_build: std::ptr::null_mut(),
        os_uname: std::ptr::null_mut(),
        os_arch: std::ptr::null_mut(),
    }
}

fn snapshot(osd: &OsData) -> Vec<Option<String>> {
    vec![
        cstr_to_owned(osd.os_name),
        cstr_to_owned(osd.os_version),
        cstr_to_owned(osd.os_major),
        cstr_to_owned(osd.os_minor),
        cstr_to_owned(osd.os_codename),
        cstr_to_owned(osd.os_platform),
        cstr_to_owned(osd.os_build),
        cstr_to_owned(osd.os_uname),
        cstr_to_owned(osd.os_arch),
    ]
}

unsafe fn free_osd(osd: &mut OsData) {
    let ptrs = [
        osd.os_name,
        osd.os_version,
        osd.os_major,
        osd.os_minor,
        osd.os_codename,
        osd.os_platform,
        osd.os_build,
        osd.os_uname,
        osd.os_arch,
    ];
    for p in ptrs.iter() {
        if !p.is_null() {
            libc::free(*p as *mut libc::c_void);
        }
    }
    *osd = empty_osd();
}

#[test]
fn get_os_arch_matches() {
    let (c_lib, r_lib) = load_libs();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(*mut c_char) -> *mut c_char> =
            c_lib.get(b"get_os_arch").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(*mut c_char) -> *mut c_char> =
            r_lib.get(b"get_os_arch").unwrap();

        let inputs = vec![
            "Linux x86_64 something",
            "i386 abc",
            "i686 something",
            "sparc here",
            "something amd64",
            "i86pc",
            "ia64 system",
            "AIX system",
            "armv6 board",
            "armv7 board",
            "aarch64 box",
            "arm64 box",
            "no match here",
            "",
            "x86_64 i386 amd64", // first match wins
            "test i386 x86_64",   // first occurrence in array order (x86_64) should win
        ];

        for input in inputs {
            let cs = CString::new(input).unwrap();
            // We need a mutable copy each time since get_os_arch takes *mut.
            let mut buf1: Vec<u8> = cs.as_bytes_with_nul().to_vec();
            let mut buf2: Vec<u8> = cs.as_bytes_with_nul().to_vec();
            let p1 = c_fn(buf1.as_mut_ptr() as *mut c_char);
            let p2 = r_fn(buf2.as_mut_ptr() as *mut c_char);
            let s1 = cstr_to_owned(p1);
            let s2 = cstr_to_owned(p2);
            assert_eq!(s1, s2, "mismatch for input {:?}", input);
            if !p1.is_null() {
                libc::free(p1 as *mut libc::c_void);
            }
            if !p2.is_null() {
                libc::free(p2 as *mut libc::c_void);
            }
        }
    }
}

#[test]
fn w_regexec_matches() {
    let (c_lib, r_lib) = load_libs();
    unsafe {
        type Fn = unsafe extern "C" fn(*const c_char, *const c_char, usize, *mut Regmatch) -> c_int;
        let c_fn: Symbol<Fn> = c_lib.get(b"w_regexec").unwrap();
        let r_fn: Symbol<Fn> = r_lib.get(b"w_regexec").unwrap();

        let cases = vec![
            ("^([0-9]+)\\.*", "10.0.19045.5247"),
            ("^[0-9]+\\.([0-9]+)\\.*", "10.0.19045.5247"),
            (
                "^[0-9]+\\.[0-9]+\\.([0-9]+(\\.[0-9]+)*)\\.*",
                "10.0.19045.5247",
            ),
            ("^([0-9]+)\\.*", "no digits here"),
            ("^([0-9]+)\\.*", "5"),
            ("^[a-z]+$", "abc"),
        ];

        for (pat, txt) in cases {
            let pat_c = CString::new(pat).unwrap();
            let txt_c = CString::new(txt).unwrap();
            let mut m1 = [Regmatch::default(); 2];
            let mut m2 = [Regmatch::default(); 2];
            let r1 = c_fn(pat_c.as_ptr(), txt_c.as_ptr(), 2, m1.as_mut_ptr());
            let r2 = r_fn(pat_c.as_ptr(), txt_c.as_ptr(), 2, m2.as_mut_ptr());
            assert_eq!(r1, r2, "rc mismatch for {:?}", (pat, txt));
            if r1 != 0 {
                for i in 0..2 {
                    assert_eq!(
                        m1[i].rm_so, m2[i].rm_so,
                        "rm_so[{}] mismatch for {:?}",
                        i,
                        (pat, txt)
                    );
                    assert_eq!(
                        m1[i].rm_eo, m2[i].rm_eo,
                        "rm_eo[{}] mismatch for {:?}",
                        i,
                        (pat, txt)
                    );
                }
            }
        }
    }
}

#[test]
fn w_regexec_null_args() {
    let (c_lib, r_lib) = load_libs();
    unsafe {
        type Fn = unsafe extern "C" fn(*const c_char, *const c_char, usize, *mut Regmatch) -> c_int;
        let c_fn: Symbol<Fn> = c_lib.get(b"w_regexec").unwrap();
        let r_fn: Symbol<Fn> = r_lib.get(b"w_regexec").unwrap();

        let txt_c = CString::new("hello").unwrap();
        let mut m = [Regmatch::default(); 2];
        let r1 = c_fn(std::ptr::null(), txt_c.as_ptr(), 2, m.as_mut_ptr());
        let r2 = r_fn(std::ptr::null(), txt_c.as_ptr(), 2, m.as_mut_ptr());
        assert_eq!(r1, r2);

        let pat_c = CString::new("^a").unwrap();
        let r1 = c_fn(pat_c.as_ptr(), std::ptr::null(), 2, m.as_mut_ptr());
        let r2 = r_fn(pat_c.as_ptr(), std::ptr::null(), 2, m.as_mut_ptr());
        assert_eq!(r1, r2);
    }
}

fn run_parse(lib: &Library, uname: &str) -> Vec<Option<String>> {
    unsafe {
        let f: Symbol<unsafe extern "C" fn(*mut c_char, *mut OsData)> =
            lib.get(b"parse_uname_string").unwrap();
        let mut osd = empty_osd();
        let cs = CString::new(uname).unwrap();
        let mut buf: Vec<u8> = cs.as_bytes_with_nul().to_vec();
        f(buf.as_mut_ptr() as *mut c_char, &mut osd as *mut OsData);
        let snap = snapshot(&osd);
        free_osd(&mut osd);
        snap
    }
}

#[test]
fn parse_uname_windows() {
    let (c_lib, r_lib) = load_libs();
    let inputs = vec![
        "Microsoft Windows 10 [Ver: 10.0.19045.5247]",
        "Microsoft Windows Server 2019 [Ver: 10.0.17763.1]",
        "Foo [Ver: 6.3.9600]",
    ];
    for input in inputs {
        let c_out = run_parse(&c_lib, input);
        let r_out = run_parse(&r_lib, input);
        assert_eq!(c_out, r_out, "mismatch for input {:?}", input);
    }
}

#[test]
fn parse_uname_linux_full() {
    let (c_lib, r_lib) = load_libs();
    let inputs = vec![
        "Linux foo 5.15.0-91-generic #101-Ubuntu SMP Tue Nov 14 13:30:08 UTC 2023 x86_64 [Ubuntu|ubuntu: 22.04.3 LTS (Jammy Jellyfish)]",
        "Linux host 4.18.0 #1 SMP Thu Jan 1 00:00:00 UTC 1970 amd64 [Debian|debian: 11 (bullseye)]",
        "Darwin host 22.0 i386 [macOS|macos: 13.0 (Ventura)]",
        "Linux host #1 SMP aarch64 [CentOS|centos: 7.9]",
        "Linux host arm64 [Alpine|alpine: 3.18]",
    ];
    for input in inputs {
        let c_out = run_parse(&c_lib, input);
        let r_out = run_parse(&r_lib, input);
        assert_eq!(c_out, r_out, "mismatch for input {:?}", input);
    }
}

#[test]
fn parse_uname_no_brackets() {
    let (c_lib, r_lib) = load_libs();
    let inputs = vec![
        "just a uname x86_64",
        "no markers anywhere",
        "Linux i386",
        "",
    ];
    for input in inputs {
        let c_out = run_parse(&c_lib, input);
        let r_out = run_parse(&r_lib, input);
        assert_eq!(c_out, r_out, "mismatch for input {:?}", input);
    }
}
