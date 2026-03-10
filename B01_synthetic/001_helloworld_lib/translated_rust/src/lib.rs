use std::os::raw::c_int;

#[unsafe(no_mangle)]
pub extern "C" fn helloworld() -> c_int {
    print!("Hello World!\n");
    0
}
