
#[no_mangle]
pub fn ldexp_q2(mut y: f32, mut exp_q2: i32) -> f32 {
    const G_EXPFRAC: [f32; 4] = [
        9.31322575e-10_f32,
        7.83145814e-10_f32,
        6.58544508e-10_f32,
        5.53767716e-10_f32,
    ];

    while exp_q2 > 0 {
        let e = exp_q2.min(30 * 4);
        let scale = G_EXPFRAC[(e & 3) as usize] * ((1_i32 << 30) >> (e >> 2)) as f32;
        y *= scale;
        exp_q2 -= e;
    }

    y
}

