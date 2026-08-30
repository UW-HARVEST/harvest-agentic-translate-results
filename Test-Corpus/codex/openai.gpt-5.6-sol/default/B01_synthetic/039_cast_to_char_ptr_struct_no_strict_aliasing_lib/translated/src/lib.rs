use std::ffi::{c_char, c_int};
use std::mem::{MaybeUninit, size_of};

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
    let mut house = MaybeUninit::<House>::zeroed();

    // The zeroed representation is valid for all fields and preserves the
    // source's initialization of any C struct padding.
    let house = unsafe {
        let house_ptr = house.as_mut_ptr();
        (*house_ptr).floors = floors;
        (*house_ptr).bedrooms = 3;
        (*house_ptr).bathrooms = 2.0;
        house.assume_init()
    };

    let raw = unsafe {
        std::slice::from_raw_parts((&house as *const House).cast::<u8>(), size_of::<House>())
    };

    for byte in raw {
        unsafe {
            printf(c"%02x".as_ptr(), c_int::from(*byte));
        }
    }
    unsafe {
        printf(c"\n".as_ptr());
    }
}
