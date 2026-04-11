extern "C" {
    fn printf(__format: *const libc::c_char, ...) -> libc::c_int;
    fn rand() -> libc::c_int;
    fn srand(__seed: libc::c_uint);
}
pub type size_t = usize;
pub const ARRAY_SIZE: libc::c_int = 256 as libc::c_int * 1024 as libc::c_int;
pub const ITERATIONS: libc::c_int = 2000 as libc::c_int;
#[no_mangle]
pub static mut array: [libc::c_int; 262144] = [0; 262144];
#[no_mangle]
pub unsafe extern "C" fn perform_expensive_operations() {
    let mut i: size_t = 0 as size_t;
    while i < ARRAY_SIZE as size_t {
        let mut x: libc::c_int = array[i as usize];
        let mut j: libc::c_int = 0 as libc::c_int;
        while j < 100 as libc::c_int {
            x = x * 3 as libc::c_int + 7 as libc::c_int;
            x = x ^ x >> 3 as libc::c_int;
            x = x - (x << 1 as libc::c_int);
            x = x / 2 as libc::c_int + x % 7 as libc::c_int;
            j += 1;
        }
        array[i as usize] = x;
        i = i.wrapping_add(1);
    }
}
#[no_mangle]
pub unsafe extern "C" fn long_exec(mut seed: libc::c_uint) {
    srand(seed);
    let mut i: size_t = 0 as size_t;
    while i < ARRAY_SIZE as size_t {
        array[i as usize] = rand();
        i = i.wrapping_add(1);
    }
    let mut i_0: libc::c_int = 0 as libc::c_int;
    while i_0 < ITERATIONS {
        perform_expensive_operations();
        i_0 += 1;
    }
    let mut xor_result: libc::c_int = 0 as libc::c_int;
    let mut i_1: size_t = 0 as size_t;
    while i_1 < ARRAY_SIZE as size_t {
        xor_result ^= array[i_1 as usize];
        i_1 = i_1.wrapping_add(1);
    }
    printf(
        b"%d\n\0" as *const u8 as *const libc::c_char,
        xor_result,
    );
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

