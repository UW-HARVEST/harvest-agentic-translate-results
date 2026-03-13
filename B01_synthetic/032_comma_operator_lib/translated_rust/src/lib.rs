use std::ffi::c_int;

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int) {
    let mut j: c_int = 0;
    for i in 0..x {
        println!("{} {}", i, j);
        j += 2;
    }
}
