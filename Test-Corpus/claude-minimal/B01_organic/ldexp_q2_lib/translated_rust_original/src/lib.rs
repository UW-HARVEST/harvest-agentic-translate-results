#[no_mangle]
pub extern "C" fn ldexp_q2(mut y: f32, mut exp_q2: i32) -> f32 {
    const G_EXPFRAC: [f32; 4] = [
        9.31322575e-10f32,
        7.83145814e-10f32,
        6.58544508e-10f32,
        5.53767716e-10f32,
    ];
    loop {
        let e: i32 = if (30 * 4) > exp_q2 { exp_q2 } else { 30 * 4 };
        let shift: i32 = (1i32 << 30) >> (e >> 2);
        y *= G_EXPFRAC[(e & 3) as usize] * (shift as f32);
        exp_q2 -= e;
        if exp_q2 <= 0 {
            break;
        }
    }
    y
}
