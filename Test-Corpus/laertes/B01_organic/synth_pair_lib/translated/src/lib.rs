pub type __int16_t = i16;
pub type int16_t = i16;
pub type mp3d_sample_t = i16;
 extern "C" fn mp3d_scale_pcm(mut sample: libc::c_float) -> int16_t {
    if sample as libc::c_double >= 32766.5f64 {
        return 32767 as libc::c_int as int16_t;
    }
    if sample as libc::c_double <= -32767.5f64 {
        return -(32768 as libc::c_int) as int16_t;
    }
    let mut s: int16_t = (sample + 0.5f32) as int16_t;
    s = (s as libc::c_int
        - ((s as libc::c_int) < 0 as libc::c_int) as libc::c_int)
        as int16_t;
    return s;
}
#[no_mangle]
pub unsafe extern "C" fn synth_pair(
    mut pcm: *mut mp3d_sample_t,
    mut nch: libc::c_int,
    mut z: *const libc::c_float,
) {
    let mut a: libc::c_float = 0.;
    a = (*z.offset((14 as libc::c_int * 64 as libc::c_int) as isize)
        - *z.offset(0 as libc::c_int as isize))
        * 29 as libc::c_int as libc::c_float;
    a += (*z.offset((1 as libc::c_int * 64 as libc::c_int) as isize)
        + *z.offset((13 as libc::c_int * 64 as libc::c_int) as isize))
        * 213 as libc::c_int as libc::c_float;
    a += (*z.offset((12 as libc::c_int * 64 as libc::c_int) as isize)
        - *z.offset((2 as libc::c_int * 64 as libc::c_int) as isize))
        * 459 as libc::c_int as libc::c_float;
    a += (*z.offset((3 as libc::c_int * 64 as libc::c_int) as isize)
        + *z.offset((11 as libc::c_int * 64 as libc::c_int) as isize))
        * 2037 as libc::c_int as libc::c_float;
    a += (*z.offset((10 as libc::c_int * 64 as libc::c_int) as isize)
        - *z.offset((4 as libc::c_int * 64 as libc::c_int) as isize))
        * 5153 as libc::c_int as libc::c_float;
    a += (*z.offset((5 as libc::c_int * 64 as libc::c_int) as isize)
        + *z.offset((9 as libc::c_int * 64 as libc::c_int) as isize))
        * 6574 as libc::c_int as libc::c_float;
    a += (*z.offset((8 as libc::c_int * 64 as libc::c_int) as isize)
        - *z.offset((6 as libc::c_int * 64 as libc::c_int) as isize))
        * 37489 as libc::c_int as libc::c_float;
    a += *z.offset((7 as libc::c_int * 64 as libc::c_int) as isize)
        * 75038 as libc::c_int as libc::c_float;
    *pcm.offset(0 as libc::c_int as isize) = mp3d_scale_pcm(a) as mp3d_sample_t;
    z = z.offset(2 as libc::c_int as isize);
    a = *z.offset((14 as libc::c_int * 64 as libc::c_int) as isize)
        * 104 as libc::c_int as libc::c_float;
    a += *z.offset((12 as libc::c_int * 64 as libc::c_int) as isize)
        * 1567 as libc::c_int as libc::c_float;
    a += *z.offset((10 as libc::c_int * 64 as libc::c_int) as isize)
        * 9727 as libc::c_int as libc::c_float;
    a += *z.offset((8 as libc::c_int * 64 as libc::c_int) as isize)
        * 64019 as libc::c_int as libc::c_float;
    a += *z.offset((6 as libc::c_int * 64 as libc::c_int) as isize)
        * -(9975 as libc::c_int) as libc::c_float;
    a += *z.offset((4 as libc::c_int * 64 as libc::c_int) as isize)
        * -(45 as libc::c_int) as libc::c_float;
    a += *z.offset((2 as libc::c_int * 64 as libc::c_int) as isize)
        * 146 as libc::c_int as libc::c_float;
    a += *z.offset((0 as libc::c_int * 64 as libc::c_int) as isize)
        * -(5 as libc::c_int) as libc::c_float;
    *pcm.offset((16 as libc::c_int * nch) as isize) = mp3d_scale_pcm(a) as mp3d_sample_t;
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

