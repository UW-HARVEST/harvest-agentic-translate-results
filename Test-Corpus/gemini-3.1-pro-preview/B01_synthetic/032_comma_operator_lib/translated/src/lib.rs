use std::os::raw::c_int;

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int) {
    let mut j = 0;
    for i in 0..x {
        println!("{} {}", i, j);
        j += 2;
    }
}
