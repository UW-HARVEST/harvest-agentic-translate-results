// Test helper binary: dlopen()s a shared library, calls confusion(a,b,c,d),
// and writes its stdout output followed by `__RET__:<ret>\n`.

use std::ffi::{c_char, c_int, c_void, CString};
use std::io::Write;

#[link(name = "dl")]
extern "C" {
    fn dlopen(filename: *const c_char, flags: c_int) -> *mut c_void;
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
    fn dlerror() -> *const c_char;
}

extern "C" {
    fn fflush(stream: *mut c_void) -> c_int;
    static mut stdout: *mut c_void;
}

const RTLD_NOW: c_int = 0x002;

type ConfusionFn = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 6 {
        eprintln!("usage: confusion_runner <libpath> a b c d");
        std::process::exit(2);
    }
    let lib_path = CString::new(args[1].clone()).unwrap();
    let a: c_int = args[2].parse().unwrap();
    let b: c_int = args[3].parse().unwrap();
    let c: c_int = args[4].parse().unwrap();
    let d: c_int = args[5].parse().unwrap();

    unsafe {
        let h = dlopen(lib_path.as_ptr(), RTLD_NOW);
        if h.is_null() {
            let err = dlerror();
            let s = if err.is_null() {
                "<null>".to_string()
            } else {
                std::ffi::CStr::from_ptr(err).to_string_lossy().into_owned()
            };
            eprintln!("dlopen failed: {s}");
            std::process::exit(3);
        }
        let sym_name = CString::new("confusion").unwrap();
        let p = dlsym(h, sym_name.as_ptr());
        if p.is_null() {
            eprintln!("dlsym failed: confusion");
            std::process::exit(4);
        }
        let f: ConfusionFn = std::mem::transmute(p);

        fflush(stdout);
        let ret = f(a, b, c, d);
        fflush(stdout);

        let mut out = std::io::stdout().lock();
        out.write_all(format!("__RET__:{}\n", ret).as_bytes()).unwrap();
        out.flush().unwrap();
    }
}
