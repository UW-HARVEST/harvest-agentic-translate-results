use libloading::{Library, Symbol};
use std::os::raw::{c_char, c_int};
use std::path::PathBuf;
use std::sync::OnceLock;

type FmaArrayFn = unsafe extern "C" fn(
    *mut c_int,
    *const c_int,
    *const c_int,
    *const c_int,
    c_int,
);
type CallFmaFn = unsafe extern "C" fn(*const c_int, c_int) -> c_int;
type DriverFn = unsafe extern "C" fn(*const c_char);

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src")
        .join("build")
        .join("libdriver.so")
}

fn rust_lib_path() -> PathBuf {
    let target = std::env::var("CARGO_TARGET_DIR")
        .unwrap_or_else(|_| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("target")
                .to_string_lossy()
                .into_owned()
        });
    let mut p = PathBuf::from(target);
    if !p.is_absolute() {
        p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(p);
    }
    // Try both debug and release
    let dbg = p.join("debug").join("libdriver.so");
    if dbg.exists() {
        return dbg;
    }
    p.join("release").join("libdriver.so")
}

struct Libs {
    c: Library,
    rs: Library,
}

fn load_libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| unsafe {
        let c = Library::new(c_lib_path()).expect("load c lib");
        let rs = Library::new(rust_lib_path()).expect("load rust lib");
        Libs { c, rs }
    })
}

fn get<'a, T>(lib: &'a Library, name: &[u8]) -> Symbol<'a, T> {
    unsafe { lib.get(name).expect("symbol") }
}

#[test]
fn test_fma_array_basic() {
    let libs = load_libs();
    let c_fma: Symbol<FmaArrayFn> = get(&libs.c, b"fma_array");
    let r_fma: Symbol<FmaArrayFn> = get(&libs.rs, b"fma_array");

    let mul1: Vec<c_int> = vec![1, 2, 3, 4, 5];
    let mul2: Vec<c_int> = vec![10, 20, 30, 40, 50];
    let add: Vec<c_int> = vec![100, 200, 300, 400, 500];
    let len = mul1.len() as c_int;

    let mut out_c: Vec<c_int> = vec![0; mul1.len()];
    let mut out_r: Vec<c_int> = vec![0; mul1.len()];

    unsafe {
        c_fma(out_c.as_mut_ptr(), mul1.as_ptr(), mul2.as_ptr(), add.as_ptr(), len);
        r_fma(out_r.as_mut_ptr(), mul1.as_ptr(), mul2.as_ptr(), add.as_ptr(), len);
    }
    assert_eq!(out_c, out_r);
}

#[test]
fn test_fma_array_negatives_and_overflow() {
    let libs = load_libs();
    let c_fma: Symbol<FmaArrayFn> = get(&libs.c, b"fma_array");
    let r_fma: Symbol<FmaArrayFn> = get(&libs.rs, b"fma_array");

    let mul1: Vec<c_int> = vec![-1, i32::MAX, i32::MIN, 0, 7];
    let mul2: Vec<c_int> = vec![2, 2, -1, 999, -7];
    let add: Vec<c_int> = vec![5, -5, 1, 0, i32::MAX];
    let len = mul1.len() as c_int;

    let mut out_c: Vec<c_int> = vec![0; mul1.len()];
    let mut out_r: Vec<c_int> = vec![0; mul1.len()];

    unsafe {
        c_fma(out_c.as_mut_ptr(), mul1.as_ptr(), mul2.as_ptr(), add.as_ptr(), len);
        r_fma(out_r.as_mut_ptr(), mul1.as_ptr(), mul2.as_ptr(), add.as_ptr(), len);
    }
    assert_eq!(out_c, out_r);
}

#[test]
fn test_call_fma_empty() {
    let libs = load_libs();
    let c_cf: Symbol<CallFmaFn> = get(&libs.c, b"call_fma");
    let r_cf: Symbol<CallFmaFn> = get(&libs.rs, b"call_fma");

    let data: Vec<c_int> = vec![];
    unsafe {
        let cv = c_cf(data.as_ptr(), 0);
        let rv = r_cf(data.as_ptr(), 0);
        assert_eq!(cv, rv);
        assert_eq!(cv, 0);
    }
}

#[test]
fn test_call_fma_various() {
    let libs = load_libs();
    let c_cf: Symbol<CallFmaFn> = get(&libs.c, b"call_fma");
    let r_cf: Symbol<CallFmaFn> = get(&libs.rs, b"call_fma");

    let cases: Vec<Vec<c_int>> = vec![
        vec![42],
        vec![1, 2, 3, 4, 5],
        vec![-1, -2, -3, i32::MAX, i32::MIN],
        vec![0; 50],
        (0..100).collect(),
    ];

    for case in cases {
        let len = case.len() as c_int;
        unsafe {
            let cv = c_cf(case.as_ptr(), len);
            let rv = r_cf(case.as_ptr(), len);
            assert_eq!(cv, rv, "mismatch for case len={}", case.len());
            // Per C: out[len-1] = ones[len-1] * data[len-1] + zeros[len-1] = data[len-1]
            assert_eq!(cv, *case.last().unwrap());
        }
    }
}

#[test]
fn test_driver_compare_via_redirect() {
    use std::ffi::CString;
    use std::fs::File;
    use std::io::{Read, Seek, SeekFrom};
    use std::os::unix::io::{AsRawFd, FromRawFd};

    fn capture_stdout<F: FnOnce()>(f: F) -> String {
        unsafe {
            extern "C" {
                fn fflush(stream: *mut std::ffi::c_void) -> i32;
            }

            let saved = libc_dup(1);
            // Create unique tmp file
            let pid = std::process::id();
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!("driver_capture_{}_{}", pid, nanos));
            let tmp = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(true)
                .open(&path)
                .unwrap();
            let fd = tmp.as_raw_fd();
            libc_dup2(fd, 1);
            f();
            fflush(std::ptr::null_mut());
            libc_dup2(saved, 1);
            libc_close(saved);
            let mut t = tmp;
            t.seek(SeekFrom::Start(0)).unwrap();
            let mut s = String::new();
            t.read_to_string(&mut s).unwrap();
            let _ = std::fs::remove_file(&path);
            s
        }
    }

    // Use raw libc via libloading-style? Just use libc crate functions through extern.
    extern "C" {
        fn dup(oldfd: i32) -> i32;
        fn dup2(oldfd: i32, newfd: i32) -> i32;
        fn close(fd: i32) -> i32;
    }
    unsafe fn libc_dup(fd: i32) -> i32 { dup(fd) }
    unsafe fn libc_dup2(a: i32, b: i32) -> i32 { dup2(a, b) }
    unsafe fn libc_close(fd: i32) -> i32 { close(fd) }

    let libs = load_libs();
    let c_drv: Symbol<DriverFn> = get(&libs.c, b"driver");
    let r_drv: Symbol<DriverFn> = get(&libs.rs, b"driver");

    let inputs = [
        "1 2 3 4 5",
        "",
        "100",
        "-1 -2 -3",
        "10 20 30 40 50 60 70 80 90 100",
        "  7   8   9  ",
        "1 abc 2 3", // sscanf will stop at "abc"
        "0 0 0 0 0",
    ];

    for s in inputs {
        let cs = CString::new(s).unwrap();
        let c_out = capture_stdout(|| unsafe { c_drv(cs.as_ptr()); });
        let r_out = capture_stdout(|| unsafe { r_drv(cs.as_ptr()); });
        assert_eq!(c_out, r_out, "driver mismatch for input {:?}", s);
    }
}
