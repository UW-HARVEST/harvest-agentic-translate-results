use std::io::{self, Write};
use std::mem::{align_of, size_of, MaybeUninit};
use std::os::raw::{c_char, c_int};
use std::ptr;

#[repr(C)]
struct House {
    floors: c_int,
    bedrooms: c_int,
    bathrooms: f64,
}

extern "C" {
    fn scanf(format: *const c_char, ...) -> c_int;
}

fn field_offsets() -> (usize, usize, usize) {
    let uninit = MaybeUninit::<House>::uninit();
    let base = uninit.as_ptr();

    unsafe {
        let floors = ptr::addr_of!((*base).floors) as usize - base as usize;
        let bedrooms = ptr::addr_of!((*base).bedrooms) as usize - base as usize;
        let bathrooms = ptr::addr_of!((*base).bathrooms) as usize - base as usize;
        (floors, bedrooms, bathrooms)
    }
}

fn print_hex(bytes: &[u8]) {
    let mut output = String::with_capacity(bytes.len() * 2 + 1);
    for byte in bytes {
        use std::fmt::Write as _;
        write!(&mut output, "{:02x}", byte).unwrap();
    }
    output.push('\n');

    io::stdout().write_all(output.as_bytes()).unwrap();
}

fn driver(floors: c_int) {
    let (floors_offset, bedrooms_offset, bathrooms_offset) = field_offsets();
    let mut raw = vec![0u8; size_of::<House>()];

    raw[floors_offset..floors_offset + size_of::<c_int>()].copy_from_slice(&floors.to_ne_bytes());
    raw[bedrooms_offset..bedrooms_offset + size_of::<c_int>()]
        .copy_from_slice(&(3 as c_int).to_ne_bytes());
    raw[bathrooms_offset..bathrooms_offset + size_of::<f64>()]
        .copy_from_slice(&2.0f64.to_ne_bytes());

    print_hex(&raw);
}

fn main() {
    assert_eq!(size_of::<c_int>(), 4);
    assert_eq!(align_of::<House>(), align_of::<f64>());

    let mut x: c_int = 0;
    unsafe {
        scanf(b"%d\0".as_ptr().cast::<c_char>(), &mut x);
    }
    driver(x);
}
