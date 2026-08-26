use std::ffi::{c_char, c_int};
use std::io::{self, Write};

unsafe extern "C" {
    fn scanf(format: *const c_char, ...) -> c_int;
}

#[repr(C)]
struct House {
    floors: c_int,
    bedrooms: c_int,
    bathrooms: f64,
}

#[no_mangle]
pub extern "C" fn driver(floors: c_int) {
    let house = House {
        floors,
        bedrooms: 3,
        bathrooms: 2.0,
    };
    let bytes = unsafe {
        std::slice::from_raw_parts(
            (&house as *const House).cast::<u8>(),
            std::mem::size_of::<House>(),
        )
    };

    let mut output = [0_u8; std::mem::size_of::<House>() * 2 + 1];
    const HEX: &[u8; 16] = b"0123456789abcdef";
    for (index, byte) in bytes.iter().copied().enumerate() {
        output[index * 2] = HEX[usize::from(byte >> 4)];
        output[index * 2 + 1] = HEX[usize::from(byte & 0x0f)];
    }
    output[output.len() - 1] = b'\n';

    let _ = io::stdout().write_all(&output);
}

#[cfg_attr(not(test), no_mangle)]
pub extern "C" fn main() -> c_int {
    let mut floors = 0;
    unsafe {
        scanf(b"%d\0".as_ptr().cast(), &mut floors);
    }
    driver(floors);
    0
}
