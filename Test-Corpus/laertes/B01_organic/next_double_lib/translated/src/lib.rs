pub type __uint64_t = u64;
pub type uint64_t = u64;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct cn_rnd_t {
    pub state: [u64; 2],
}
impl std::default::Default for cn_rnd_t {
    fn default() -> Self {
        cn_rnd_t {
        state: [u64::default(); 2]
        }
    }
}

unsafe extern "C" fn cn_rnd_next<'a1>(mut rnd: Option<&'a1 mut crate::src::lib::cn_rnd_t>) -> u64 {
    let mut x: uint64_t = (*borrow_mut(&mut rnd).unwrap()).state[0 as libc::c_int as usize];
    let mut y: uint64_t = (*borrow_mut(&mut rnd).unwrap()).state[1 as libc::c_int as usize];
    (*borrow_mut(&mut rnd).unwrap()).state[0 as libc::c_int as usize] = y;
    x = (x as libc::c_ulong ^ (x << 23 as libc::c_int) as libc::c_ulong)
        as uint64_t;
    x = (x as libc::c_ulong ^ (x >> 17 as libc::c_int) as libc::c_ulong)
        as uint64_t;
    x = (x as libc::c_ulong ^ (y ^ y >> 26 as libc::c_int) as libc::c_ulong)
        as uint64_t;
    (*borrow_mut(&mut rnd).unwrap()).state[1 as libc::c_int as usize] = x;
    return x.wrapping_add(y);
}
#[no_mangle]
pub unsafe extern "C" fn next_double<'a1>(mut rnd: Option<&'a1 mut crate::src::lib::cn_rnd_t>) -> libc::c_double {
    let mut value: uint64_t = cn_rnd_next(borrow_mut(&mut rnd));
    let mut exponent: uint64_t = 1023 as uint64_t;
    let mut mantissa: uint64_t = value >> 12 as libc::c_int;
    let mut result: uint64_t = exponent << 52 as libc::c_int | mantissa;
    return *(&raw mut result as *mut libc::c_double) - 1.0f64;
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

