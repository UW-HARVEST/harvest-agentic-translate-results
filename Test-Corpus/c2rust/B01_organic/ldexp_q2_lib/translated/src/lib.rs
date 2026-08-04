#[no_mangle]
pub unsafe extern "C" fn ldexp_q2(
    mut y: ::core::ffi::c_float,
    mut exp_q2: ::core::ffi::c_int,
) -> ::core::ffi::c_float {
    static mut g_expfrac: [::core::ffi::c_float; 4] = [
        9.31322575e-10f32,
        7.83145814e-10f32,
        6.58544508e-10f32,
        5.53767716e-10f32,
    ];
    let mut e: ::core::ffi::c_int = 0;
    loop {
        e = if 30 as ::core::ffi::c_int * 4 as ::core::ffi::c_int > exp_q2 {
            exp_q2
        } else {
            30 as ::core::ffi::c_int * 4 as ::core::ffi::c_int
        };
        y *= g_expfrac[(e & 3 as ::core::ffi::c_int) as usize]
            * ((1 as ::core::ffi::c_int) << 30 as ::core::ffi::c_int
                >> (e >> 2 as ::core::ffi::c_int)) as ::core::ffi::c_float;
        exp_q2 -= e;
        if !(exp_q2 > 0 as ::core::ffi::c_int) {
            break;
        }
    }
    return y;
}
