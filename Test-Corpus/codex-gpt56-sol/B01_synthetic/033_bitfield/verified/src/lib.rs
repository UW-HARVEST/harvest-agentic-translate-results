use std::arch::asm;
use std::ffi::{c_char, c_int, c_uint, c_void};

extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;

    #[link_name = "__isoc99_scanf"]
    fn scanf(format: *const c_char, ...) -> c_int;
}

unsafe fn print_fields(bit_fields: c_uint, z: c_int) {
    let x = bit_fields & 0b11;
    let y = (bit_fields >> 2) & 0b111;
    let b = ((bit_fields >> 5) & 1) as c_int;

    printf(b"%u %u %d %d\n\0".as_ptr().cast(), x, y, b, z);
}

unsafe fn load_foo(foo: *const c_void) -> (c_uint, c_int) {
    let bit_fields: c_uint;
    let z: c_int;
    asm!(
        "mov {bit_fields:e}, dword ptr [{foo}]",
        "mov {z:e}, dword ptr [{foo} + 4]",
        foo = in(reg) foo,
        bit_fields = out(reg) bit_fields,
        z = out(reg) z,
        options(nostack, readonly, preserves_flags),
    );
    (bit_fields, z)
}

#[no_mangle]
pub unsafe extern "C" fn print_foo(foo: *const c_void) {
    let (bit_fields, z) = load_foo(foo);
    print_fields(bit_fields, z);
}

#[no_mangle]
pub unsafe extern "C" fn driver(x: c_uint, y: c_uint, b: bool, z: c_int) {
    let bit_fields = (x & 0b11) | ((y & 0b111) << 2) | ((b as c_uint) << 5);
    print_fields(bit_fields, z);
}

#[export_name = "main"]
pub unsafe extern "C" fn c_main() -> c_int {
    let mut x: c_uint = 0;
    let mut y: c_uint = 0;
    let mut b: c_int = 0;
    let mut z: c_int = 0;

    scanf(b"%u\0".as_ptr().cast(), &mut x);
    scanf(b"%u\0".as_ptr().cast(), &mut y);
    scanf(b"%d\0".as_ptr().cast(), &mut b);
    scanf(b"%d\0".as_ptr().cast(), &mut z);
    driver(x, y, b != 0, z);
    0
}
