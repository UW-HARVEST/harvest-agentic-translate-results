use std::ffi::{c_float, c_int};

#[unsafe(no_mangle)]
pub extern "C" fn ldexp_q2(mut y: c_float, mut exp_q2: c_int) -> c_float {
    static G_EXPFRAC: [c_float; 4] = [
        9.31322575e-10_f32,
        7.83145814e-10_f32,
        6.58544508e-10_f32,
        5.53767716e-10_f32,
    ];

    loop {
        let e: c_int = if 30 * 4 > exp_q2 { exp_q2 } else { 30 * 4 };
        y *= G_EXPFRAC[(e & 3) as usize] * ((1_i32 << 30) >> (e >> 2)) as c_float;
        exp_q2 -= e;
        if exp_q2 <= 0 {
            break;
        }
    }

    y
}
