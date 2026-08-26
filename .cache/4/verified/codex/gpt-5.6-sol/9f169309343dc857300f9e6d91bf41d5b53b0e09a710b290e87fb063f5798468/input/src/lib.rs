use std::ffi::{c_char, c_int};
use std::mem::size_of;
use std::ptr::{addr_of_mut, write_unaligned};

#[repr(C)]
struct House {
    floors: c_int,
    bedrooms: c_int,
    bathrooms: f64,
}

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(floors: c_int) {
    let mut raw = [0_u8; size_of::<House>()];
    let house = raw.as_mut_ptr().cast::<House>();

    // Write in place so the zeroed C struct's padding remains zeroed.
    unsafe {
        write_unaligned(addr_of_mut!((*house).floors), floors);
        write_unaligned(addr_of_mut!((*house).bedrooms), 3);
        write_unaligned(addr_of_mut!((*house).bathrooms), 2.0);

        for byte in raw {
            printf(c"%02x".as_ptr(), c_int::from(byte));
        }
        printf(c"\n".as_ptr());
    }
}
