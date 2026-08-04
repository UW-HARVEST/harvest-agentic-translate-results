use std::os::raw::c_int;

#[unsafe(no_mangle)]
pub extern "C" fn sieve(start: c_int) {
    let mut val = start;
    loop {
        println!("{}", val);
        if val % 10 == 9 {
            break;
        }
        val += 1;
    }
}