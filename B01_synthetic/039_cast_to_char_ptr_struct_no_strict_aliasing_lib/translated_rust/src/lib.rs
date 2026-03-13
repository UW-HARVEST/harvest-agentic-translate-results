use std::ffi::c_int;
use std::mem;

extern "C" {
    fn printf(fmt: *const u8, ...) -> c_int;
}

#[repr(C)]
struct HouseT {
    floors: c_int,
    bedrooms: c_int,
    bathrooms: f64,
}

unsafe fn print_hex(p: *const u8, len: c_int) {
    for i in 0..len as usize {
        printf(b"%02x\0".as_ptr(), *p.add(i) as c_int);
    }
    printf(b"\n\0".as_ptr());
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(floors: c_int) {
    unsafe {
        let mut house: HouseT = mem::zeroed();
        house.floors = floors;
        house.bedrooms = 3;
        house.bathrooms = 2.0;
        let mut raw = [0u8; mem::size_of::<HouseT>()];
        std::ptr::copy_nonoverlapping(
            &house as *const HouseT as *const u8,
            raw.as_mut_ptr(),
            mem::size_of::<HouseT>(),
        );
        print_hex(raw.as_ptr(), raw.len() as c_int);
    }
}
