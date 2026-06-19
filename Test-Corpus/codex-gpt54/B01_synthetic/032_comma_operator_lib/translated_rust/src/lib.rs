use libc::{c_char, c_int, printf};

static DRIVER_FORMAT: &[u8] = b"%d %d\n\0";

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int) {
    let mut i: c_int = 0;
    let mut j: c_int = 0;

    while i < x {
        unsafe {
            printf(
                DRIVER_FORMAT.as_ptr().cast::<c_char>(),
                i,
                j,
            );
        }
        i = i.wrapping_add(1);
        j = j.wrapping_add(2);
    }
}
