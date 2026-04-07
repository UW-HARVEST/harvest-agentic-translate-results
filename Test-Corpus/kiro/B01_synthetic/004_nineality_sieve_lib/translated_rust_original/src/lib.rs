use std::ffi::c_int;

#[unsafe(no_mangle)]
pub extern "C" fn sieve(mut val: c_int) {
    loop {
        println!("{}", val);
        if val % 10 == 9 {
            break;
        }
        val += 1;
    }
}
