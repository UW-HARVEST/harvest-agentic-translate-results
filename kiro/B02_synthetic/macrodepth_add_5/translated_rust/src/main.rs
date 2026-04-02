use std::ffi::{c_int, CString};
use std::os::raw::c_char;

extern "C" {
    static stderr: *mut libc::FILE;
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        let prog = CString::new(args[0].as_str()).unwrap();
        unsafe {
            libc::fprintf(
                stderr,
                b"usage: %s A B\n\0".as_ptr() as *const c_char,
                prog.as_ptr(),
            );
        }
        std::process::exit(2);
    }
    let a: c_int = args[1].parse().unwrap_or(0);
    let b: c_int = args[2].parse().unwrap_or(0);

    macrodepth::init_globals();

    let r_call = macrodepth::selected_op(a, b);
    let mut acc = {
        #[cfg(feature = "add")] { 0 as c_int }
        #[cfg(feature = "sub")] { 0 as c_int }
        #[cfg(feature = "mul")] { 1 as c_int }
    };
    macrodepth::run_loop(&mut acc);

    let x1 = macrodepth::helper_call(a, b);
    let x2 = macrodepth::helper_ptr(a, b);
    let x3 = macrodepth::use_generated(macrodepth::REPEAT);
    let g = unsafe { (macrodepth::G_OP.unwrap())(a, b) };

    unsafe {
        libc::printf(
            b"op=%s call=%d acc=%d g.call=%d\n\0".as_ptr() as *const c_char,
            macrodepth::G_OP_NAME,
            r_call,
            acc,
            g,
        );
        libc::printf(
            b"summary=%d\n\0".as_ptr() as *const c_char,
            r_call + acc + x1 + x2 + x3 + g,
        );
    }
}
