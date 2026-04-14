use std::os::raw::c_int;

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int) {
    let mut j: c_int = 0;
    let mut i: c_int = 0;
    while i < x {
        println!("{} {}", i, j);
        i += 1;
        j += 2;
    }
}
