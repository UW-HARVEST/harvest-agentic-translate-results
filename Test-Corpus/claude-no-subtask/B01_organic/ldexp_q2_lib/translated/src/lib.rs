use std::ffi::c_int;

#[unsafe(no_mangle)]
pub extern "C" fn ldexp_q2(mut y: f32, mut exp_q2: c_int) -> f32 {
    const G_EXPFRAC: [f32; 4] = [
        9.31322575e-10_f32,
        7.83145814e-10_f32,
        6.58544508e-10_f32,
        5.53767716e-10_f32,
    ];
    loop {
        let e: c_int = if (30 * 4) > exp_q2 { exp_q2 } else { 30 * 4 };
        // Mirror the C expression: g_expfrac[e & 3] * (1 << 30 >> (e >> 2))
        // In C, `1 << 30 >> (e >> 2)` is `(1 << 30) >> (e >> 2)` due to
        // left-to-right associativity. The integer result is then promoted
        // to float for the multiplication with g_expfrac[e & 3].
        let shift_amt: c_int = e >> 2;
        let int_factor: i32 = (1i32 << 30).wrapping_shr(shift_amt as u32);
        y *= G_EXPFRAC[(e & 3) as usize] * (int_factor as f32);
        exp_q2 -= e;
        if exp_q2 <= 0 {
            break;
        }
    }
    y
}
