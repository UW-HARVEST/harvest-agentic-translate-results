pub type __int16_t = i16;
pub type int16_t = __int16_t;
pub type mp3d_sample_t = int16_t;
unsafe extern "C" fn mp3d_scale_pcm(mut sample: ::core::ffi::c_float) -> int16_t {
    if sample as ::core::ffi::c_double >= 32766.5f64 {
        return 32767 as ::core::ffi::c_int as int16_t;
    }
    if sample as ::core::ffi::c_double <= -32767.5f64 {
        return -(32768 as ::core::ffi::c_int) as int16_t;
    }
    let mut s: int16_t = (sample + 0.5f32) as int16_t;
    s = (s as ::core::ffi::c_int
        - ((s as ::core::ffi::c_int) < 0 as ::core::ffi::c_int) as ::core::ffi::c_int)
        as int16_t;
    return s;
}
#[no_mangle]
pub unsafe extern "C" fn synth_pair(
    mut pcm: *mut mp3d_sample_t,
    mut nch: ::core::ffi::c_int,
    mut z: *const ::core::ffi::c_float,
) {
    let mut a: ::core::ffi::c_float = 0.;
    a = (*z.offset((14 as ::core::ffi::c_int * 64 as ::core::ffi::c_int) as isize)
        - *z.offset(0 as ::core::ffi::c_int as isize))
        * 29 as ::core::ffi::c_int as ::core::ffi::c_float;
    a += (*z.offset((1 as ::core::ffi::c_int * 64 as ::core::ffi::c_int) as isize)
        + *z.offset((13 as ::core::ffi::c_int * 64 as ::core::ffi::c_int) as isize))
        * 213 as ::core::ffi::c_int as ::core::ffi::c_float;
    a += (*z.offset((12 as ::core::ffi::c_int * 64 as ::core::ffi::c_int) as isize)
        - *z.offset((2 as ::core::ffi::c_int * 64 as ::core::ffi::c_int) as isize))
        * 459 as ::core::ffi::c_int as ::core::ffi::c_float;
    a += (*z.offset((3 as ::core::ffi::c_int * 64 as ::core::ffi::c_int) as isize)
        + *z.offset((11 as ::core::ffi::c_int * 64 as ::core::ffi::c_int) as isize))
        * 2037 as ::core::ffi::c_int as ::core::ffi::c_float;
    a += (*z.offset((10 as ::core::ffi::c_int * 64 as ::core::ffi::c_int) as isize)
        - *z.offset((4 as ::core::ffi::c_int * 64 as ::core::ffi::c_int) as isize))
        * 5153 as ::core::ffi::c_int as ::core::ffi::c_float;
    a += (*z.offset((5 as ::core::ffi::c_int * 64 as ::core::ffi::c_int) as isize)
        + *z.offset((9 as ::core::ffi::c_int * 64 as ::core::ffi::c_int) as isize))
        * 6574 as ::core::ffi::c_int as ::core::ffi::c_float;
    a += (*z.offset((8 as ::core::ffi::c_int * 64 as ::core::ffi::c_int) as isize)
        - *z.offset((6 as ::core::ffi::c_int * 64 as ::core::ffi::c_int) as isize))
        * 37489 as ::core::ffi::c_int as ::core::ffi::c_float;
    a += *z.offset((7 as ::core::ffi::c_int * 64 as ::core::ffi::c_int) as isize)
        * 75038 as ::core::ffi::c_int as ::core::ffi::c_float;
    *pcm.offset(0 as ::core::ffi::c_int as isize) = mp3d_scale_pcm(a) as mp3d_sample_t;
    z = z.offset(2 as ::core::ffi::c_int as isize);
    a = *z.offset((14 as ::core::ffi::c_int * 64 as ::core::ffi::c_int) as isize)
        * 104 as ::core::ffi::c_int as ::core::ffi::c_float;
    a += *z.offset((12 as ::core::ffi::c_int * 64 as ::core::ffi::c_int) as isize)
        * 1567 as ::core::ffi::c_int as ::core::ffi::c_float;
    a += *z.offset((10 as ::core::ffi::c_int * 64 as ::core::ffi::c_int) as isize)
        * 9727 as ::core::ffi::c_int as ::core::ffi::c_float;
    a += *z.offset((8 as ::core::ffi::c_int * 64 as ::core::ffi::c_int) as isize)
        * 64019 as ::core::ffi::c_int as ::core::ffi::c_float;
    a += *z.offset((6 as ::core::ffi::c_int * 64 as ::core::ffi::c_int) as isize)
        * -(9975 as ::core::ffi::c_int) as ::core::ffi::c_float;
    a += *z.offset((4 as ::core::ffi::c_int * 64 as ::core::ffi::c_int) as isize)
        * -(45 as ::core::ffi::c_int) as ::core::ffi::c_float;
    a += *z.offset((2 as ::core::ffi::c_int * 64 as ::core::ffi::c_int) as isize)
        * 146 as ::core::ffi::c_int as ::core::ffi::c_float;
    a += *z.offset((0 as ::core::ffi::c_int * 64 as ::core::ffi::c_int) as isize)
        * -(5 as ::core::ffi::c_int) as ::core::ffi::c_float;
    *pcm.offset((16 as ::core::ffi::c_int * nch) as isize) = mp3d_scale_pcm(a) as mp3d_sample_t;
}
