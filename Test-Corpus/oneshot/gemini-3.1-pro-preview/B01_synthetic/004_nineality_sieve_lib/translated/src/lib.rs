use std::os::raw::c_int;

#[unsafe(no_mangle)]
pub extern "C" fn sieve(mut val: c_int) {
    loop {
        println!("{}", val);
        if val % 10 == 9 {
            break;
        }
        val = val.wrapping_add(1);
    }
}
