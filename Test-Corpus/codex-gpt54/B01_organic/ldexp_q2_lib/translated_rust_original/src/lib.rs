use core::ffi::{c_float, c_int};

const G_EXPFRAC: [c_float; 4] = [
    c_float::from_bits(0x3080_0000),
    c_float::from_bits(0x3057_44fd),
    c_float::from_bits(0x3035_04f3),
    c_float::from_bits(0x3018_37f0),
];

#[unsafe(no_mangle)]
pub extern "C" fn ldexp_q2(mut y: c_float, mut exp_q2: c_int) -> c_float {
    let mut e: c_int;

    loop {
        e = if 30 * 4 > exp_q2 { exp_q2 } else { 30 * 4 };
        y *= G_EXPFRAC[(e & 3) as usize]
            * ((1_i32 << 30).wrapping_shr((e >> 2) as u32) as c_float);
        exp_q2 -= e;
        if exp_q2 <= 0 {
            break;
        }
    }

    y
}
