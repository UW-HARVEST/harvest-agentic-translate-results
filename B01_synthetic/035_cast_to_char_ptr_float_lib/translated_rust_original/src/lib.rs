use std::ffi::c_int;

fn print_hex(p: *const u8, len: c_int) {
    for i in 0..len {
        print!("{:02x}", unsafe { *p.offset(i as isize) });
    }
    println!();
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: f32) {
    let p = &x as *const f32 as *const u8;
    print_hex(p, std::mem::size_of::<f32>() as c_int);
}
