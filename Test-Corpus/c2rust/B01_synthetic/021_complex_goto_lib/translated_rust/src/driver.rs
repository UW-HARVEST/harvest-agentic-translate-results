extern "C" {
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn driver(mut x: ::core::ffi::c_int, mut y: ::core::ffi::c_int) {
    let mut current_block_6: u64;
    while x > 0 as ::core::ffi::c_int || y > 0 as ::core::ffi::c_int {
        printf(b"loop\n\0" as *const u8 as *const ::core::ffi::c_char);
        if x == 1 as ::core::ffi::c_int && y == 4 as ::core::ffi::c_int {
            current_block_6 = 13277901459238179029;
        } else {
            current_block_6 = 861247850213060928;
        }
        loop {
            match current_block_6 {
                861247850213060928 => {
                    if x > 0 as ::core::ffi::c_int {
                        printf(b"x\n\0" as *const u8 as *const ::core::ffi::c_char);
                        x -= 1;
                    }
                    current_block_6 = 13277901459238179029;
                }
                _ => {
                    if y == 0 as ::core::ffi::c_int {
                        break;
                    }
                    printf(b"y\n\0" as *const u8 as *const ::core::ffi::c_char);
                    y -= 1;
                    if x < 3 as ::core::ffi::c_int {
                        current_block_6 = 861247850213060928;
                    } else {
                        break;
                    }
                }
            }
        }
    }
}
