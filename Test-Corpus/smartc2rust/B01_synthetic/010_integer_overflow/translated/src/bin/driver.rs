fn main() {
    let rc = unsafe { driver::main_c_main() };
    std::process::exit(rc as i32);
}
