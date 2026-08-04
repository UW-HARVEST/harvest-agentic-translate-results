use std::env;
use std::ffi::CString;
use std::os::unix::ffi::OsStrExt;

fn static_sum(sum: &mut i32, update: i32) -> i32 {
    *sum = sum.wrapping_add(update);
    *sum
}

fn main() {
    let args: Vec<_> = env::args_os().collect();
    if args.len() != 2 {
        print!("Error: should only be a single (integer) argument!\n");
        std::process::exit(1);
    }

    let arg_bytes = args[1].as_bytes();
    let arg = match CString::new(arg_bytes) {
        Ok(arg) => arg,
        Err(_) => {
            print!("Error: first argument must be an integer!\n");
            std::process::exit(1);
        }
    };

    let mut end = std::ptr::null_mut();
    let stride = unsafe {
        let start = arg.as_ptr() as *mut libc::c_char;
        let parsed = libc::strtol(arg.as_ptr(), &mut end, 10);
        if end == start {
            print!("Error: first argument must be an integer!\n");
            std::process::exit(1);
        }
        parsed as i32
    };

    let mut sum = 0_i32;
    for i in 0_i32..10 {
        println!("{}", static_sum(&mut sum, i.wrapping_mul(stride)));
    }
}
