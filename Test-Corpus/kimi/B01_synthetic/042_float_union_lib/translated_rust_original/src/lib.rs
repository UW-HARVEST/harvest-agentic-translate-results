use std::ffi::c_double;

#[unsafe(no_mangle)]
pub extern "C" fn driver(f: c_double) {
    let x = f.to_bits();
    println!("{:x} {:e} {:.4}", x, f, f);
}