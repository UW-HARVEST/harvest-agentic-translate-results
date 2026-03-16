// Ortho, interleave, shift_rows, mix_columns operations

fn br_aes_ct_ortho(q: &mut [u32; 8]) {
    macro_rules! swapn32 { ($cl:expr,$ch:expr,$s:expr,$x:expr,$y:expr) => {
        let a=$x; let b=$y; $x=(a& $cl)|((b& $cl)<<$s); $y=((a& $ch)>>$s)|(b& $ch);
    }}
    swapn32!(0x55555555u32,0xAAAAAAAAu32,1,q[0],q[1]);
    swapn32!(0x55555555u32,0xAAAAAAAAu32,1,q[2],q[3]);
    swapn32!(0x55555555u32,0xAAAAAAAAu32,1,q[4],q[5]);
    swapn32!(0x55555555u32,0xAAAAAAAAu32,1,q[6],q[7]);
    swapn32!(0x33333333u32,0xCCCCCCCCu32,2,q[0],q[2]);
    swapn32!(0x33333333u32,0xCCCCCCCCu32,2,q[1],q[3]);
    swapn32!(0x33333333u32,0xCCCCCCCCu32,2,q[4],q[6]);
    swapn32!(0x33333333u32,0xCCCCCCCCu32,2,q[5],q[7]);
    swapn32!(0x0F0F0F0Fu32,0xF0F0F0F0u32,4,q[0],q[4]);
    swapn32!(0x0F0F0F0Fu32,0xF0F0F0F0u32,4,q[1],q[5]);
    swapn32!(0x0F0F0F0Fu32,0xF0F0F0F0u32,4,q[2],q[6]);
    swapn32!(0x0F0F0F0Fu32,0xF0F0F0F0u32,4,q[3],q[7]);
}

fn br_aes_ct64_ortho(q: &mut [u64; 8]) {
    macro_rules! swapn { ($cl:expr,$ch:expr,$s:expr,$x:expr,$y:expr) => {
        let a=$x; let b=$y; $x=(a& $cl)|((b& $cl)<<$s); $y=((a& $ch)>>$s)|(b& $ch);
    }}
    swapn!(0x5555555555555555u64,0xAAAAAAAAAAAAAAAAu64,1,q[0],q[1]);
    swapn!(0x5555555555555555u64,0xAAAAAAAAAAAAAAAAu64,1,q[2],q[3]);
    swapn!(0x5555555555555555u64,0xAAAAAAAAAAAAAAAAu64,1,q[4],q[5]);
    swapn!(0x5555555555555555u64,0xAAAAAAAAAAAAAAAAu64,1,q[6],q[7]);
    swapn!(0x3333333333333333u64,0xCCCCCCCCCCCCCCCCu64,2,q[0],q[2]);
    swapn!(0x3333333333333333u64,0xCCCCCCCCCCCCCCCCu64,2,q[1],q[3]);
    swapn!(0x3333333333333333u64,0xCCCCCCCCCCCCCCCCu64,2,q[4],q[6]);
    swapn!(0x3333333333333333u64,0xCCCCCCCCCCCCCCCCu64,2,q[5],q[7]);
    swapn!(0x0F0F0F0F0F0F0F0Fu64,0xF0F0F0F0F0F0F0F0u64,4,q[0],q[4]);
    swapn!(0x0F0F0F0F0F0F0F0Fu64,0xF0F0F0F0F0F0F0F0u64,4,q[1],q[5]);
    swapn!(0x0F0F0F0F0F0F0F0Fu64,0xF0F0F0F0F0F0F0F0u64,4,q[2],q[6]);
    swapn!(0x0F0F0F0F0F0F0F0Fu64,0xF0F0F0F0F0F0F0F0u64,4,q[3],q[7]);
}

fn br_aes_ct64_interleave_in(q0: &mut u64, q1: &mut u64, w: &[u32]) {
    let (mut x0,mut x1,mut x2,mut x3) = (w[0] as u64, w[1] as u64, w[2] as u64, w[3] as u64);
    x0 |= x0 << 16; x1 |= x1 << 16; x2 |= x2 << 16; x3 |= x3 << 16;
    x0 &= 0x0000FFFF0000FFFF; x1 &= 0x0000FFFF0000FFFF; x2 &= 0x0000FFFF0000FFFF; x3 &= 0x0000FFFF0000FFFF;
    x0 |= x0 << 8; x1 |= x1 << 8; x2 |= x2 << 8; x3 |= x3 << 8;
    x0 &= 0x00FF00FF00FF00FF; x1 &= 0x00FF00FF00FF00FF; x2 &= 0x00FF00FF00FF00FF; x3 &= 0x00FF00FF00FF00FF;
    *q0 = x0 | (x2 << 8);
    *q1 = x1 | (x3 << 8);
}

fn br_aes_ct64_interleave_out(w: &mut [u32], q0: u64, q1: u64) {
    let (mut x0,mut x1,mut x2,mut x3) = (
        q0 & 0x00FF00FF00FF00FF, q1 & 0x00FF00FF00FF00FF,
        (q0 >> 8) & 0x00FF00FF00FF00FF, (q1 >> 8) & 0x00FF00FF00FF00FF);
    x0 |= x0 >> 8; x1 |= x1 >> 8; x2 |= x2 >> 8; x3 |= x3 >> 8;
    x0 &= 0x0000FFFF0000FFFF; x1 &= 0x0000FFFF0000FFFF; x2 &= 0x0000FFFF0000FFFF; x3 &= 0x0000FFFF0000FFFF;
    w[0] = (x0 as u32) | ((x0 >> 16) as u32);
    w[1] = (x1 as u32) | ((x1 >> 16) as u32);
    w[2] = (x2 as u32) | ((x2 >> 16) as u32);
    w[3] = (x3 as u32) | ((x3 >> 16) as u32);
}

fn add_round_key32(q: &mut [u32; 8], sk: &[u32; 8]) {
    for i in 0..8 { q[i] ^= sk[i]; }
}

fn shift_rows32(q: &mut [u32; 8]) {
    for i in 0..8 {
        let x = q[i];
        q[i] = (x & 0x000000FF)
            | ((x & 0x0000FC00) >> 2) | ((x & 0x00000300) << 6)
            | ((x & 0x00F00000) >> 4) | ((x & 0x000F0000) << 4)
            | ((x & 0xC0000000) >> 6) | ((x & 0x3F000000) << 2);
    }
}

fn mix_columns32(q: &mut [u32; 8]) {
    let (q0,q1,q2,q3,q4,q5,q6,q7) = (q[0],q[1],q[2],q[3],q[4],q[5],q[6],q[7]);
    let r0=q0.rotate_right(8); let r1=q1.rotate_right(8); let r2=q2.rotate_right(8); let r3=q3.rotate_right(8);
    let r4=q4.rotate_right(8); let r5=q5.rotate_right(8); let r6=q6.rotate_right(8); let r7=q7.rotate_right(8);
    q[0] = q7^r7^r0^(q0^r0).rotate_right(16);
    q[1] = q0^r0^q7^r7^r1^(q1^r1).rotate_right(16);
    q[2] = q1^r1^r2^(q2^r2).rotate_right(16);
    q[3] = q2^r2^q7^r7^r3^(q3^r3).rotate_right(16);
    q[4] = q3^r3^q7^r7^r4^(q4^r4).rotate_right(16);
    q[5] = q4^r4^r5^(q5^r5).rotate_right(16);
    q[6] = q5^r5^r6^(q6^r6).rotate_right(16);
    q[7] = q6^r6^r7^(q7^r7).rotate_right(16);
}

fn add_round_key(q: &mut [u64; 8], sk: &[u64; 8]) {
    for i in 0..8 { q[i] ^= sk[i]; }
}

fn shift_rows(q: &mut [u64; 8]) {
    for i in 0..8 {
        let x = q[i];
        q[i] = (x & 0x000000000000FFFF)
            | ((x & 0x00000000FFF00000) >> 4) | ((x & 0x00000000000F0000) << 12)
            | ((x & 0x0000FF0000000000) >> 8) | ((x & 0x000000FF00000000) << 8)
            | ((x & 0xF000000000000000) >> 12) | ((x & 0x0FFF000000000000) << 4);
    }
}

fn mix_columns(q: &mut [u64; 8]) {
    let (q0,q1,q2,q3,q4,q5,q6,q7) = (q[0],q[1],q[2],q[3],q[4],q[5],q[6],q[7]);
    let r0=q0.rotate_right(16); let r1=q1.rotate_right(16); let r2=q2.rotate_right(16); let r3=q3.rotate_right(16);
    let r4=q4.rotate_right(16); let r5=q5.rotate_right(16); let r6=q6.rotate_right(16); let r7=q7.rotate_right(16);
    q[0] = q7^r7^r0^(q0^r0).rotate_right(32);
    q[1] = q0^r0^q7^r7^r1^(q1^r1).rotate_right(32);
    q[2] = q1^r1^r2^(q2^r2).rotate_right(32);
    q[3] = q2^r2^q7^r7^r3^(q3^r3).rotate_right(32);
    q[4] = q3^r3^q7^r7^r4^(q4^r4).rotate_right(32);
    q[5] = q4^r4^r5^(q5^r5).rotate_right(32);
    q[6] = q5^r5^r6^(q6^r6).rotate_right(32);
    q[7] = q6^r6^r7^(q7^r7).rotate_right(32);
}

fn interleave_constant(out: &mut [u64; 8], inp: &[u8]) {
    let mut tmp = [0u32; 16];
    br_range_dec32le(&mut tmp, inp);
    for i in 0..4 {
        let mut a = 0u64;
        let mut b = 0u64;
        br_aes_ct64_interleave_in(&mut a, &mut b, &tmp[i * 4..]);
        out[i] = a;
        out[i + 4] = b;
    }
    br_aes_ct64_ortho(out);
}

fn interleave_constant32(out: &mut [u32; 8], inp: &[u8]) {
    for i in 0..4 {
        out[2 * i] = br_dec32le(&inp[4 * i..]);
        out[2 * i + 1] = br_dec32le(&inp[4 * i + 16..]);
    }
    br_aes_ct_ortho(out);
}
