use std::ffi::c_int;

#[unsafe(no_mangle)]
pub extern "C" fn ldexp_q2(mut y: f32, mut exp_q2: c_int) -> f32 {
    const G_EXPFRAC: [f32; 4] = [9.31322575e-10, 7.83145814e-10, 6.58544508e-10, 5.53767716e-10];
    loop {
        let e = if (30 * 4) > exp_q2 { exp_q2 } else { 30 * 4 };
        y *= G_EXPFRAC[(e & 3) as usize] * ((1_i32 << 30 >> (e >> 2)) as f32);
        exp_q2 -= e;
        if exp_q2 <= 0 {
            break;
        }
    }
    y
}
