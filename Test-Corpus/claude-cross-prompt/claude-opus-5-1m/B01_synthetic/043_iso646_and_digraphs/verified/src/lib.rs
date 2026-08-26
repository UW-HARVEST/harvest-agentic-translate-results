use std::ffi::c_int;
use std::io::{self, Write};

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int, y: c_int) {
    // x bitor compl y  =>  x | ~y
    let result: c_int = x | !y;
    // printf("%d", result);
    print!("{}", result);
    // puts("");
    println!();
    let _ = io::stdout().flush();
}
