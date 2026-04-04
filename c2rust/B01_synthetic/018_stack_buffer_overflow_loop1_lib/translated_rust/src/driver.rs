extern "C" {
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
}
pub type size_t = usize;
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
#[no_mangle]
pub unsafe extern "C" fn printLine(mut line: *const ::core::ffi::c_char) {
    if !line.is_null() {
        printf(b"%s\n\0" as *const u8 as *const ::core::ffi::c_char, line);
    }
}
#[no_mangle]
pub unsafe extern "C" fn printIntLine(mut intNumber: ::core::ffi::c_int) {
    printf(
        b"%d\n\0" as *const u8 as *const ::core::ffi::c_char,
        intNumber,
    );
}
#[no_mangle]
pub unsafe extern "C" fn bad() {
    let mut alloca_allocations: Vec<Vec<u8>> = Vec::new();
    let mut data: *mut ::core::ffi::c_int = ::core::ptr::null_mut::<::core::ffi::c_int>();
    alloca_allocations.push(::std::vec::from_elem(
        0,
        10 as ::core::ffi::c_int as ::core::ffi::c_ulong as usize,
    ));
    data = alloca_allocations.last_mut().unwrap().as_mut_ptr() as *mut ::core::ffi::c_int;
    let mut source: [::core::ffi::c_int; 10] = [0 as ::core::ffi::c_int; 10];
    let mut i: size_t = 0;
    i = 0 as size_t;
    while i < 10 as size_t {
        *data.offset(i as isize) = source[i as usize];
        i = i.wrapping_add(1);
    }
    printIntLine(*data.offset(0 as ::core::ffi::c_int as isize));
}
#[no_mangle]
pub unsafe extern "C" fn good() {
    let mut alloca_allocations: Vec<Vec<u8>> = Vec::new();
    let mut data: *mut ::core::ffi::c_int = ::core::ptr::null_mut::<::core::ffi::c_int>();
    data = ::core::ptr::null_mut::<::core::ffi::c_int>();
    alloca_allocations.push(::std::vec::from_elem(
        0,
        (10 as usize).wrapping_mul(::core::mem::size_of::<::core::ffi::c_int>() as usize) as usize,
    ));
    data = alloca_allocations.last_mut().unwrap().as_mut_ptr() as *mut ::core::ffi::c_int;
    let mut source: [::core::ffi::c_int; 10] = [0 as ::core::ffi::c_int; 10];
    let mut i: size_t = 0;
    i = 0 as size_t;
    while i < 10 as size_t {
        *data.offset(i as isize) = source[i as usize];
        i = i.wrapping_add(1);
    }
    printIntLine(*data.offset(0 as ::core::ffi::c_int as isize));
}
#[no_mangle]
pub unsafe extern "C" fn driver(mut useGood: ::core::ffi::c_int) {
    if useGood != 0 {
        good();
    } else {
        bad();
    };
}
