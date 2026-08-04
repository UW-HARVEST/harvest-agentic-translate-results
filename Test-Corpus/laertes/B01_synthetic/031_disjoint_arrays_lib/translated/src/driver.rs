extern "C" {
    fn printf(__format: *const libc::c_char, ...) -> libc::c_int;
    fn sscanf(
        __s: *const libc::c_char,
        __format: *const libc::c_char,
        ...
    ) -> libc::c_int;
}
pub type size_t = usize;
#[no_mangle]
pub unsafe extern "C" fn fma_array(
    mut out: *mut libc::c_int,
    mut mul1: *const libc::c_int,
    mut mul2: *const libc::c_int,
    mut add: *const libc::c_int,
    mut len: libc::c_int,
) {
    let mut i: libc::c_int = 0 as libc::c_int;
    while i < len {
        *out.offset(i as isize) =
            *mul1.offset(i as isize) * *mul2.offset(i as isize) + *add.offset(i as isize);
        i += 1;
    }
}
#[no_mangle]
pub unsafe extern "C" fn call_fma(
    mut data: *const libc::c_int,
    mut len: libc::c_int,
) -> libc::c_int {
    if len == 0 as libc::c_int {
        return 0 as libc::c_int;
    }
    let vla = len as usize;
    let mut out: Vec<libc::c_int> = ::std::vec::from_elem(0, vla);
    let vla_0 = len as usize;
    let mut ones: Vec<libc::c_int> = ::std::vec::from_elem(0, vla_0);
    let vla_1 = len as usize;
    let mut zeros: Vec<libc::c_int> = ::std::vec::from_elem(0, vla_1);
    *out.as_mut_ptr().offset(0 as libc::c_int as isize) = 0 as libc::c_int;
    let mut i: libc::c_int = 0 as libc::c_int;
    while i < len {
        *ones.as_mut_ptr().offset(i as isize) = 1 as libc::c_int;
        *zeros.as_mut_ptr().offset(i as isize) = 0 as libc::c_int;
        i += 1;
    }
    fma_array(
        out.as_mut_ptr(),
        ones.as_mut_ptr(),
        data,
        zeros.as_mut_ptr(),
        len,
    );
    return *out
        .as_mut_ptr()
        .offset((len - 1 as libc::c_int) as isize);
}
#[no_mangle]
pub unsafe extern "C" fn driver(mut in_0: *const libc::c_char) {
    let mut data: [libc::c_int; 100] = [0; 100];
    let mut i: libc::c_int = 0;
    i = 0 as libc::c_int;
    while i < 100 as libc::c_int {
        let mut nb: size_t = 0;
        if sscanf(
            in_0,
            b"%d%zn\0" as *const u8 as *const libc::c_char,
            (&raw mut data as *mut libc::c_int).offset(i as isize)
                as *mut libc::c_int,
            &raw mut nb,
        ) != 1 as libc::c_int
        {
            break;
        }
        in_0 = in_0.offset(nb as isize);
        i += 1;
    }
    let mut result: libc::c_int = call_fma(&raw mut data as *mut libc::c_int, i);
    printf(b"%d\n\0" as *const u8 as *const libc::c_char, result);
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

