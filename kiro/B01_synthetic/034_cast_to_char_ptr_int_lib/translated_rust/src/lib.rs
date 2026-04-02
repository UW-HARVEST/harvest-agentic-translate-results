use std::ffi::c_int;

fn print_hex(p: *const u8, len: usize) {
    for i in 0..len {
        let byte = unsafe { *p.add(i) };
        print!("{:02x}", byte);
    }
    println!();
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int) {
    let p = &x as *const c_int as *const u8;
    print_hex(p, std::mem::size_of::<c_int>());
}
