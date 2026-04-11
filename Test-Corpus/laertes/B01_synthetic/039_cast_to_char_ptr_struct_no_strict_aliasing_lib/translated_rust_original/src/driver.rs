extern "C" {
    fn printf(__format: *const libc::c_char, ...) -> libc::c_int;
    fn memcpy(
        __dest: *mut libc::c_void,
        __src: *const libc::c_void,
        __n: size_t,
    ) -> *mut libc::c_void;
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct house_t {
    pub floors: libc::c_int,
    pub bedrooms: libc::c_int,
    pub bathrooms: libc::c_double,
}
pub type size_t = usize;
unsafe extern "C" fn print_hex(mut p: *mut libc::c_uchar, mut len: libc::c_int) {
    let mut i: libc::c_int = 0 as libc::c_int;
    while i < len {
        printf(
            b"%02x\0" as *const u8 as *const libc::c_char,
            *p.offset(i as isize) as libc::c_int,
        );
        i += 1;
    }
    printf(b"\n\0" as *const u8 as *const libc::c_char);
}
#[no_mangle]
pub unsafe extern "C" fn driver(mut floors: libc::c_int) {
    let mut house: house_t = house_t {
        floors: 0 as libc::c_int,
        bedrooms: 0,
        bathrooms: 0.,
    };
    house.floors = floors;
    house.bedrooms = 3 as libc::c_int;
    house.bathrooms = 2.0f64;
    let mut raw: [libc::c_char; 16] = [0; 16];
    memcpy(
        &raw mut raw as *mut libc::c_char as *mut libc::c_void,
        &raw mut house as *const libc::c_void,
        std::mem::size_of::<house_t>() as size_t,
    );
    print_hex(
        &raw mut raw as *mut libc::c_uchar,
        std::mem::size_of::<[libc::c_char; 16]>() as libc::c_int,
    );
}
