// Test helper: loads an arbitrary shared library with dlopen/dlsym and calls
// the requested exported symbol, so the C `.so` and the Rust `cdylib` are both
// driven exactly like an external caller would drive them.
//
// Running each call in its own process keeps the captured stdout bytes free of
// any output produced by the test harness itself.
//
// Usage:
//   parity_runner <so-path> print-line-null
//   parity_runner <so-path> print-line-hex <hex-bytes>
//   parity_runner <so-path> print-int <i32> [<i32> ...]
//   parity_runner <so-path> void <symbol> [<repeat-count>]
//   parity_runner <so-path> seq <symbol> [<symbol> ...]

use std::ffi::{c_char, c_int, c_void, CString};

unsafe extern "C" {
    fn dlopen(filename: *const c_char, flag: c_int) -> *mut c_void;
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
    fn dlerror() -> *const c_char;
}

// RTLD_NOW | RTLD_LOCAL: resolve eagerly, and keep the symbols out of the
// global namespace so nothing can interpose on the library's internal calls.
const RTLD_NOW: c_int = 2;
const RTLD_LOCAL: c_int = 0;

type PrintLineFn = unsafe extern "C" fn(*const c_char);
type PrintIntLineFn = unsafe extern "C" fn(c_int);
type VoidFn = unsafe extern "C" fn();

fn fail(msg: String) -> ! {
    eprintln!("parity_runner: {msg}");
    std::process::exit(2);
}

fn last_dlerror() -> String {
    unsafe {
        let err = dlerror();
        if err.is_null() {
            "unknown error".to_string()
        } else {
            std::ffi::CStr::from_ptr(err).to_string_lossy().into_owned()
        }
    }
}

fn load(path: &str) -> *mut c_void {
    let c_path = CString::new(path).unwrap_or_else(|_| fail("NUL in library path".into()));
    let handle = unsafe { dlopen(c_path.as_ptr(), RTLD_NOW | RTLD_LOCAL) };
    if handle.is_null() {
        fail(format!("dlopen({path}) failed: {}", last_dlerror()));
    }
    handle
}

fn symbol(handle: *mut c_void, name: &str) -> *mut c_void {
    let c_name = CString::new(name).unwrap_or_else(|_| fail("NUL in symbol name".into()));
    let sym = unsafe { dlsym(handle, c_name.as_ptr()) };
    if sym.is_null() {
        fail(format!("dlsym({name}) failed: {}", last_dlerror()));
    }
    sym
}

fn decode_hex(hex: &str) -> Vec<u8> {
    if !hex.len().is_multiple_of(2) {
        fail("hex payload must have an even length".into());
    }
    (0..hex.len() / 2)
        .map(|i| {
            u8::from_str_radix(&hex[2 * i..2 * i + 2], 16)
                .unwrap_or_else(|e| fail(format!("bad hex: {e}")))
        })
        .collect()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        fail(format!("usage: {} <so-path> <op> [operands...]", args[0]));
    }
    let handle = load(&args[1]);
    let op = args[2].as_str();

    match op {
        "print-line-null" => {
            let f: PrintLineFn = unsafe { std::mem::transmute(symbol(handle, "printLine")) };
            unsafe { f(std::ptr::null()) };
        }
        "print-line-hex" => {
            if args.len() != 4 {
                fail("print-line-hex needs exactly one hex operand".into());
            }
            let mut bytes = decode_hex(&args[3]);
            if bytes.contains(&0) {
                fail("payload must not contain an interior NUL".into());
            }
            bytes.push(0); // NUL terminator for the C string
            let f: PrintLineFn = unsafe { std::mem::transmute(symbol(handle, "printLine")) };
            unsafe { f(bytes.as_ptr() as *const c_char) };
        }
        "print-int" => {
            if args.len() < 4 {
                fail("print-int needs at least one integer operand".into());
            }
            let f: PrintIntLineFn = unsafe { std::mem::transmute(symbol(handle, "printIntLine")) };
            for raw in &args[3..] {
                let value: c_int = raw
                    .parse()
                    .unwrap_or_else(|e| fail(format!("bad integer {raw}: {e}")));
                unsafe { f(value) };
            }
        }
        "void" => {
            if args.len() < 4 {
                fail("void needs a symbol name".into());
            }
            let repeat: usize = if args.len() > 4 {
                args[4]
                    .parse()
                    .unwrap_or_else(|e| fail(format!("bad repeat count: {e}")))
            } else {
                1
            };
            let f: VoidFn = unsafe { std::mem::transmute(symbol(handle, &args[3])) };
            for _ in 0..repeat {
                unsafe { f() };
            }
        }
        "seq" => {
            if args.len() < 4 {
                fail("seq needs at least one symbol name".into());
            }
            for name in &args[3..] {
                let f: VoidFn = unsafe { std::mem::transmute(symbol(handle, name)) };
                unsafe { f() };
            }
        }
        other => fail(format!("unknown op: {other}")),
    }
    // Returning from main lets libc flush the stdout buffer, just as the C
    // program would on exit.
}
