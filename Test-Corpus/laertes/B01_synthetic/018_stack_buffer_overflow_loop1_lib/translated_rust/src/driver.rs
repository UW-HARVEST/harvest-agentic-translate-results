extern "C" {
    fn printf(__format: *const libc::c_char, ...) -> libc::c_int;
}
pub type size_t = usize;
pub const NULL: *mut libc::c_void = std::ptr::null_mut::<libc::c_void>();
#[no_mangle]
pub unsafe extern "C" fn printLine(mut line: *const libc::c_char) {
    if !line.is_null() {
        printf(b"%s\n\0" as *const u8 as *const libc::c_char, line);
    }
}
#[no_mangle]
pub unsafe extern "C" fn printIntLine(mut intNumber: libc::c_int) {
    printf(
        b"%d\n\0" as *const u8 as *const libc::c_char,
        intNumber,
    );
}
#[no_mangle]
pub unsafe extern "C" fn bad() {
    let mut alloca_allocations: Vec<Vec<u8>> = Vec::new();
    let mut data: *mut libc::c_int = std::ptr::null_mut::<libc::c_int>();
    alloca_allocations.push(::std::vec::from_elem(
        0,
        10 as libc::c_int as libc::c_ulong as usize,
    ));
    data = alloca_allocations.last_mut().unwrap().as_mut_ptr() as *mut libc::c_int;
    let mut source: [libc::c_int; 10] = [0 as libc::c_int; 10];
    let mut i: size_t = 0;
    i = 0 as size_t;
    while i < 10 as size_t {
        *data.offset(i as isize) = source[i as usize];
        i = i.wrapping_add(1);
    }
    printIntLine(*data.offset(0 as libc::c_int as isize));
}
#[no_mangle]
pub unsafe extern "C" fn good() {
    let mut alloca_allocations: Vec<Vec<u8>> = Vec::new();
    let mut data: *mut libc::c_int = std::ptr::null_mut::<libc::c_int>();
    data = std::ptr::null_mut::<libc::c_int>();
    alloca_allocations.push(::std::vec::from_elem(
        0,
        (10 as usize).wrapping_mul(std::mem::size_of::<libc::c_int>() as usize) as usize,
    ));
    data = alloca_allocations.last_mut().unwrap().as_mut_ptr() as *mut libc::c_int;
    let mut source: [libc::c_int; 10] = [0 as libc::c_int; 10];
    let mut i: size_t = 0;
    i = 0 as size_t;
    while i < 10 as size_t {
        *data.offset(i as isize) = source[i as usize];
        i = i.wrapping_add(1);
    }
    printIntLine(*data.offset(0 as libc::c_int as isize));
}
#[no_mangle]
pub unsafe extern "C" fn driver(mut useGood: libc::c_int) {
    if useGood != 0 {
        good();
    } else {
        bad();
    };
}
pub fn borrow<'a, 'b: 'a, T>(p: &'a Option<&'b mut T>) -> Option<&'a T> {
    p.as_ref().map(|x| &**x)
}

pub fn borrow_mut<'a, 'b : 'a, T>(p: &'a mut Option<&'b mut T>) -> Option<&'a mut T> {
    p.as_mut().map(|x| &mut **x)
}

pub fn owned_as_ref<'a, T>(p: &'a Option<Box<T>>) -> Option<&'a T> {
    p.as_ref().map(|x| x.as_ref())
}

pub fn owned_as_mut<'a, T>(p: &'a mut Option<Box<T>>) -> Option<&'a mut T> {
    p.as_mut().map(|x| x.as_mut())
}

pub fn option_to_raw<T>(p: Option<&T>) -> * const T {
    p.map_or(core::ptr::null(), |p| p as * const T)
}

pub fn _ref_eq<T>(p: Option<&T>, q: Option<&T>) -> bool {
    option_to_raw(p) == option_to_raw(q)
}

pub fn _ref_ne<T>(p: Option<&T>, q: Option<&T>) -> bool {
    option_to_raw(p) != option_to_raw(q)
}

