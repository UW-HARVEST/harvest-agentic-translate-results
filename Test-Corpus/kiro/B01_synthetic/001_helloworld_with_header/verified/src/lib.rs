#[no_mangle]
pub extern "C" fn helloworld() -> i32 {
    print!("Hello World!\n");
    0
}
