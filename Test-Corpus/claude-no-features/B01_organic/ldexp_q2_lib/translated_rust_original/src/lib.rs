use std::ffi::c_int;

#[unsafe(no_mangle)]
pub extern "C" fn ldexp_q2(mut y: f32, mut exp_q2: c_int) -> f32 {
    const G_EXPFRAC: [f32; 4] = [
        9.31322575e-10f32,
        7.83145814e-10f32,
        6.58544508e-10f32,
        5.53767716e-10f32,
    ];
    let mut e: c_int;
    loop {
        e = if (30 * 4) > exp_q2 { exp_q2 } else { 30 * 4 };
        let shift_amt: i32 = e >> 2;
        let factor: i32 = (1i32 << 30) >> shift_amt;
        y *= G_EXPFRAC[(e & 3) as usize] * (factor as f32);
        exp_q2 -= e;
        if exp_q2 <= 0 {
            break;
        }
    }
    y
}
