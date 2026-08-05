//! Translated from ed25519_ref10_fe_25_5.h, fe_25_5/fe.h, fe_25_5/constants.h,
//! and the fe25519_invert/pow22523/sqrt helpers in ed25519_ref10.c.
//! fe25519 is the 2^25.5 representation: [i32; 10].

pub type Fe = [i32; 10];

#[inline(always)]
pub fn load_3(inp: &[u8]) -> u64 {
    (inp[0] as u64) | ((inp[1] as u64) << 8) | ((inp[2] as u64) << 16)
}

#[inline(always)]
pub fn load_4(inp: &[u8]) -> u64 {
    (inp[0] as u64)
        | ((inp[1] as u64) << 8)
        | ((inp[2] as u64) << 16)
        | ((inp[3] as u64) << 24)
}

/* ---- constants (fe_25_5/constants.h) ---- */

pub const FE25519_SQRTM1: Fe = [
    -32595792, -7943725, 9377950, 3500415, 12389472, -272473, -25146209, -2005654, 326686, 11406482,
];
pub const ED25519_SQRTAM2: Fe = [
    -12222970, -8312128, -11511410, 9067497, -15300785, -241793, 25456130, 14121551, -12187136,
    3972024,
];
pub const ED25519_D: Fe = [
    -10913610, 13857413, -15372611, 6949391, 114729, -8787816, -6275908, -3247719, -18696448,
    -12055116,
];
pub const ED25519_D2: Fe = [
    -21827239, -5839606, -30745221, 13898782, 229458, 15978800, -12551817, -6495438, 29715968,
    9444199,
];
pub const ED25519_A_32: u32 = 486662;
pub const ED25519_A: Fe = [486662, 0, 0, 0, 0, 0, 0, 0, 0, 0];
pub const ED25519_SQRTADM1: Fe = [
    24849947, -153582, -23613485, 6347715, -21072328, -667138, -25271143, -15367704, -870347,
    14525639,
];
pub const ED25519_INVSQRTAMD: Fe = [
    6111485, 4156064, -27798727, 12243468, -25904040, 120897, 20826367, -7060776, 6093568, -1986012,
];
pub const ED25519_ONEMSQD: Fe = [
    6275446, -16617371, -22938544, -3773710, 11667077, 7397348, -27922721, 1766195, -24433858,
    672203,
];
pub const ED25519_SQDMONE: Fe = [
    15551795, -11097455, -13425098, -10125071, -11896535, 10178284, -26634327, 4729244, -5282110,
    -10116402,
];

/* ---- basic ops ---- */

#[inline(always)]
pub fn fe_0() -> Fe {
    [0i32; 10]
}

#[inline(always)]
pub fn fe_1() -> Fe {
    let mut h = [0i32; 10];
    h[0] = 1;
    h
}

#[inline(always)]
pub fn fe_add(f: Fe, g: Fe) -> Fe {
    let mut h = [0i32; 10];
    for i in 0..10 {
        h[i] = f[i].wrapping_add(g[i]);
    }
    h
}

#[inline(always)]
pub fn fe_sub(f: Fe, g: Fe) -> Fe {
    let mut h = [0i32; 10];
    for i in 0..10 {
        h[i] = f[i].wrapping_sub(g[i]);
    }
    h
}

#[inline(always)]
pub fn fe_neg(f: Fe) -> Fe {
    let mut h = [0i32; 10];
    for i in 0..10 {
        h[i] = f[i].wrapping_neg();
    }
    h
}

#[inline(always)]
pub fn fe_cmov(f: &mut Fe, g: &Fe, b: u32) {
    let mask: u32 = (b as i32).wrapping_neg() as u32;
    for i in 0..10 {
        let x = ((f[i] ^ g[i]) as u32 & mask) as i32;
        f[i] ^= x;
    }
}

#[inline(always)]
pub fn fe_cswap(f: &mut Fe, g: &mut Fe, b: u32) {
    let mask: u32 = ((b as i64).wrapping_neg()) as u32;
    for i in 0..10 {
        let x = ((f[i] ^ g[i]) as u32 & mask) as i32;
        f[i] ^= x;
        g[i] ^= x;
    }
}

#[inline(always)]
pub fn fe_isnegative(f: &Fe) -> i32 {
    let s = fe_tobytes(*f);
    (s[0] & 1) as i32
}

#[inline(always)]
pub fn fe_iszero(f: &Fe) -> i32 {
    let s = fe_tobytes(*f);
    crate::ed25519::is_zero(&s) as i32
}

pub fn fe_mul(f: Fe, g: Fe) -> Fe {
    let f0 = f[0] as i64;
    let f1 = f[1] as i64;
    let f2 = f[2] as i64;
    let f3 = f[3] as i64;
    let f4 = f[4] as i64;
    let f5 = f[5] as i64;
    let f6 = f[6] as i64;
    let f7 = f[7] as i64;
    let f8 = f[8] as i64;
    let f9 = f[9] as i64;

    let g0 = g[0] as i64;
    let g1 = g[1] as i64;
    let g2 = g[2] as i64;
    let g3 = g[3] as i64;
    let g4 = g[4] as i64;
    let g5 = g[5] as i64;
    let g6 = g[6] as i64;
    let g7 = g[7] as i64;
    let g8 = g[8] as i64;
    let g9 = g[9] as i64;

    let g1_19 = 19 * g1;
    let g2_19 = 19 * g2;
    let g3_19 = 19 * g3;
    let g4_19 = 19 * g4;
    let g5_19 = 19 * g5;
    let g6_19 = 19 * g6;
    let g7_19 = 19 * g7;
    let g8_19 = 19 * g8;
    let g9_19 = 19 * g9;
    let f1_2 = 2 * f1;
    let f3_2 = 2 * f3;
    let f5_2 = 2 * f5;
    let f7_2 = 2 * f7;
    let f9_2 = 2 * f9;

    let f0g0 = f0 * g0;
    let f0g1 = f0 * g1;
    let f0g2 = f0 * g2;
    let f0g3 = f0 * g3;
    let f0g4 = f0 * g4;
    let f0g5 = f0 * g5;
    let f0g6 = f0 * g6;
    let f0g7 = f0 * g7;
    let f0g8 = f0 * g8;
    let f0g9 = f0 * g9;
    let f1g0 = f1 * g0;
    let f1g1_2 = f1_2 * g1;
    let f1g2 = f1 * g2;
    let f1g3_2 = f1_2 * g3;
    let f1g4 = f1 * g4;
    let f1g5_2 = f1_2 * g5;
    let f1g6 = f1 * g6;
    let f1g7_2 = f1_2 * g7;
    let f1g8 = f1 * g8;
    let f1g9_38 = f1_2 * g9_19;
    let f2g0 = f2 * g0;
    let f2g1 = f2 * g1;
    let f2g2 = f2 * g2;
    let f2g3 = f2 * g3;
    let f2g4 = f2 * g4;
    let f2g5 = f2 * g5;
    let f2g6 = f2 * g6;
    let f2g7 = f2 * g7;
    let f2g8_19 = f2 * g8_19;
    let f2g9_19 = f2 * g9_19;
    let f3g0 = f3 * g0;
    let f3g1_2 = f3_2 * g1;
    let f3g2 = f3 * g2;
    let f3g3_2 = f3_2 * g3;
    let f3g4 = f3 * g4;
    let f3g5_2 = f3_2 * g5;
    let f3g6 = f3 * g6;
    let f3g7_38 = f3_2 * g7_19;
    let f3g8_19 = f3 * g8_19;
    let f3g9_38 = f3_2 * g9_19;
    let f4g0 = f4 * g0;
    let f4g1 = f4 * g1;
    let f4g2 = f4 * g2;
    let f4g3 = f4 * g3;
    let f4g4 = f4 * g4;
    let f4g5 = f4 * g5;
    let f4g6_19 = f4 * g6_19;
    let f4g7_19 = f4 * g7_19;
    let f4g8_19 = f4 * g8_19;
    let f4g9_19 = f4 * g9_19;
    let f5g0 = f5 * g0;
    let f5g1_2 = f5_2 * g1;
    let f5g2 = f5 * g2;
    let f5g3_2 = f5_2 * g3;
    let f5g4 = f5 * g4;
    let f5g5_38 = f5_2 * g5_19;
    let f5g6_19 = f5 * g6_19;
    let f5g7_38 = f5_2 * g7_19;
    let f5g8_19 = f5 * g8_19;
    let f5g9_38 = f5_2 * g9_19;
    let f6g0 = f6 * g0;
    let f6g1 = f6 * g1;
    let f6g2 = f6 * g2;
    let f6g3 = f6 * g3;
    let f6g4_19 = f6 * g4_19;
    let f6g5_19 = f6 * g5_19;
    let f6g6_19 = f6 * g6_19;
    let f6g7_19 = f6 * g7_19;
    let f6g8_19 = f6 * g8_19;
    let f6g9_19 = f6 * g9_19;
    let f7g0 = f7 * g0;
    let f7g1_2 = f7_2 * g1;
    let f7g2 = f7 * g2;
    let f7g3_38 = f7_2 * g3_19;
    let f7g4_19 = f7 * g4_19;
    let f7g5_38 = f7_2 * g5_19;
    let f7g6_19 = f7 * g6_19;
    let f7g7_38 = f7_2 * g7_19;
    let f7g8_19 = f7 * g8_19;
    let f7g9_38 = f7_2 * g9_19;
    let f8g0 = f8 * g0;
    let f8g1 = f8 * g1;
    let f8g2_19 = f8 * g2_19;
    let f8g3_19 = f8 * g3_19;
    let f8g4_19 = f8 * g4_19;
    let f8g5_19 = f8 * g5_19;
    let f8g6_19 = f8 * g6_19;
    let f8g7_19 = f8 * g7_19;
    let f8g8_19 = f8 * g8_19;
    let f8g9_19 = f8 * g9_19;
    let f9g0 = f9 * g0;
    let f9g1_38 = f9_2 * g1_19;
    let f9g2_19 = f9 * g2_19;
    let f9g3_38 = f9_2 * g3_19;
    let f9g4_19 = f9 * g4_19;
    let f9g5_38 = f9_2 * g5_19;
    let f9g6_19 = f9 * g6_19;
    let f9g7_38 = f9_2 * g7_19;
    let f9g8_19 = f9 * g8_19;
    let f9g9_38 = f9_2 * g9_19;

    let mut h0 = f0g0 + f1g9_38 + f2g8_19 + f3g7_38 + f4g6_19 + f5g5_38 + f6g4_19 + f7g3_38
        + f8g2_19
        + f9g1_38;
    let mut h1 = f0g1 + f1g0 + f2g9_19 + f3g8_19 + f4g7_19 + f5g6_19 + f6g5_19 + f7g4_19
        + f8g3_19
        + f9g2_19;
    let mut h2 = f0g2 + f1g1_2 + f2g0 + f3g9_38 + f4g8_19 + f5g7_38 + f6g6_19 + f7g5_38
        + f8g4_19
        + f9g3_38;
    let mut h3 =
        f0g3 + f1g2 + f2g1 + f3g0 + f4g9_19 + f5g8_19 + f6g7_19 + f7g6_19 + f8g5_19 + f9g4_19;
    let mut h4 = f0g4 + f1g3_2 + f2g2 + f3g1_2 + f4g0 + f5g9_38 + f6g8_19 + f7g7_38 + f8g6_19
        + f9g5_38;
    let mut h5 =
        f0g5 + f1g4 + f2g3 + f3g2 + f4g1 + f5g0 + f6g9_19 + f7g8_19 + f8g7_19 + f9g6_19;
    let mut h6 = f0g6 + f1g5_2 + f2g4 + f3g3_2 + f4g2 + f5g1_2 + f6g0 + f7g9_38 + f8g8_19
        + f9g7_38;
    let mut h7 =
        f0g7 + f1g6 + f2g5 + f3g4 + f4g3 + f5g2 + f6g1 + f7g0 + f8g9_19 + f9g8_19;
    let mut h8 = f0g8 + f1g7_2 + f2g6 + f3g5_2 + f4g4 + f5g3_2 + f6g2 + f7g1_2 + f8g0 + f9g9_38;
    let mut h9 = f0g9 + f1g8 + f2g7 + f3g6 + f4g5 + f5g4 + f6g3 + f7g2 + f8g1 + f9g0;

    let mut carry0;
    let carry1;
    let carry2;
    let carry3;
    let mut carry4;
    let carry5;
    let carry6;
    let carry7;
    let carry8;
    let carry9;

    carry0 = (h0 + (1i64 << 25)) >> 26;
    h1 += carry0;
    h0 -= carry0 * (1i64 << 26);
    carry4 = (h4 + (1i64 << 25)) >> 26;
    h5 += carry4;
    h4 -= carry4 * (1i64 << 26);

    carry1 = (h1 + (1i64 << 24)) >> 25;
    h2 += carry1;
    h1 -= carry1 * (1i64 << 25);
    carry5 = (h5 + (1i64 << 24)) >> 25;
    h6 += carry5;
    h5 -= carry5 * (1i64 << 25);

    carry2 = (h2 + (1i64 << 25)) >> 26;
    h3 += carry2;
    h2 -= carry2 * (1i64 << 26);
    carry6 = (h6 + (1i64 << 25)) >> 26;
    h7 += carry6;
    h6 -= carry6 * (1i64 << 26);

    carry3 = (h3 + (1i64 << 24)) >> 25;
    h4 += carry3;
    h3 -= carry3 * (1i64 << 25);
    carry7 = (h7 + (1i64 << 24)) >> 25;
    h8 += carry7;
    h7 -= carry7 * (1i64 << 25);

    carry4 = (h4 + (1i64 << 25)) >> 26;
    h5 += carry4;
    h4 -= carry4 * (1i64 << 26);
    carry8 = (h8 + (1i64 << 25)) >> 26;
    h9 += carry8;
    h8 -= carry8 * (1i64 << 26);

    carry9 = (h9 + (1i64 << 24)) >> 25;
    h0 += carry9 * 19;
    h9 -= carry9 * (1i64 << 25);

    carry0 = (h0 + (1i64 << 25)) >> 26;
    h1 += carry0;
    h0 -= carry0 * (1i64 << 26);

    [
        h0 as i32, h1 as i32, h2 as i32, h3 as i32, h4 as i32, h5 as i32, h6 as i32, h7 as i32,
        h8 as i32, h9 as i32,
    ]
}

pub fn fe_sq(f: Fe) -> Fe {
    fe_sq_impl(f, false)
}

pub fn fe_sq2(f: Fe) -> Fe {
    fe_sq_impl(f, true)
}

fn fe_sq_impl(f: Fe, dbl: bool) -> Fe {
    let f0 = f[0] as i64;
    let f1 = f[1] as i64;
    let f2 = f[2] as i64;
    let f3 = f[3] as i64;
    let f4 = f[4] as i64;
    let f5 = f[5] as i64;
    let f6 = f[6] as i64;
    let f7 = f[7] as i64;
    let f8 = f[8] as i64;
    let f9 = f[9] as i64;

    let f0_2 = 2 * f0;
    let f1_2 = 2 * f1;
    let f2_2 = 2 * f2;
    let f3_2 = 2 * f3;
    let f4_2 = 2 * f4;
    let f5_2 = 2 * f5;
    let f6_2 = 2 * f6;
    let f7_2 = 2 * f7;
    let f5_38 = 38 * f5;
    let f6_19 = 19 * f6;
    let f7_38 = 38 * f7;
    let f8_19 = 19 * f8;
    let f9_38 = 38 * f9;

    let f0f0 = f0 * f0;
    let f0f1_2 = f0_2 * f1;
    let f0f2_2 = f0_2 * f2;
    let f0f3_2 = f0_2 * f3;
    let f0f4_2 = f0_2 * f4;
    let f0f5_2 = f0_2 * f5;
    let f0f6_2 = f0_2 * f6;
    let f0f7_2 = f0_2 * f7;
    let f0f8_2 = f0_2 * f8;
    let f0f9_2 = f0_2 * f9;
    let f1f1_2 = f1_2 * f1;
    let f1f2_2 = f1_2 * f2;
    let f1f3_4 = f1_2 * f3_2;
    let f1f4_2 = f1_2 * f4;
    let f1f5_4 = f1_2 * f5_2;
    let f1f6_2 = f1_2 * f6;
    let f1f7_4 = f1_2 * f7_2;
    let f1f8_2 = f1_2 * f8;
    let f1f9_76 = f1_2 * f9_38;
    let f2f2 = f2 * f2;
    let f2f3_2 = f2_2 * f3;
    let f2f4_2 = f2_2 * f4;
    let f2f5_2 = f2_2 * f5;
    let f2f6_2 = f2_2 * f6;
    let f2f7_2 = f2_2 * f7;
    let f2f8_38 = f2_2 * f8_19;
    let f2f9_38 = f2 * f9_38;
    let f3f3_2 = f3_2 * f3;
    let f3f4_2 = f3_2 * f4;
    let f3f5_4 = f3_2 * f5_2;
    let f3f6_2 = f3_2 * f6;
    let f3f7_76 = f3_2 * f7_38;
    let f3f8_38 = f3_2 * f8_19;
    let f3f9_76 = f3_2 * f9_38;
    let f4f4 = f4 * f4;
    let f4f5_2 = f4_2 * f5;
    let f4f6_38 = f4_2 * f6_19;
    let f4f7_38 = f4 * f7_38;
    let f4f8_38 = f4_2 * f8_19;
    let f4f9_38 = f4 * f9_38;
    let f5f5_38 = f5 * f5_38;
    let f5f6_38 = f5_2 * f6_19;
    let f5f7_76 = f5_2 * f7_38;
    let f5f8_38 = f5_2 * f8_19;
    let f5f9_76 = f5_2 * f9_38;
    let f6f6_19 = f6 * f6_19;
    let f6f7_38 = f6 * f7_38;
    let f6f8_38 = f6_2 * f8_19;
    let f6f9_38 = f6 * f9_38;
    let f7f7_38 = f7 * f7_38;
    let f7f8_38 = f7_2 * f8_19;
    let f7f9_76 = f7_2 * f9_38;
    let f8f8_19 = f8 * f8_19;
    let f8f9_38 = f8 * f9_38;
    let f9f9_38 = f9 * f9_38;

    let mut h0 = f0f0 + f1f9_76 + f2f8_38 + f3f7_76 + f4f6_38 + f5f5_38;
    let mut h1 = f0f1_2 + f2f9_38 + f3f8_38 + f4f7_38 + f5f6_38;
    let mut h2 = f0f2_2 + f1f1_2 + f3f9_76 + f4f8_38 + f5f7_76 + f6f6_19;
    let mut h3 = f0f3_2 + f1f2_2 + f4f9_38 + f5f8_38 + f6f7_38;
    let mut h4 = f0f4_2 + f1f3_4 + f2f2 + f5f9_76 + f6f8_38 + f7f7_38;
    let mut h5 = f0f5_2 + f1f4_2 + f2f3_2 + f6f9_38 + f7f8_38;
    let mut h6 = f0f6_2 + f1f5_4 + f2f4_2 + f3f3_2 + f7f9_76 + f8f8_19;
    let mut h7 = f0f7_2 + f1f6_2 + f2f5_2 + f3f4_2 + f8f9_38;
    let mut h8 = f0f8_2 + f1f7_4 + f2f6_2 + f3f5_4 + f4f4 + f9f9_38;
    let mut h9 = f0f9_2 + f1f8_2 + f2f7_2 + f3f6_2 + f4f5_2;

    let mut carry0;
    let carry1;
    let carry2;
    let carry3;
    let mut carry4;
    let carry5;
    let carry6;
    let carry7;
    let carry8;
    let carry9;

    if dbl {
        h0 += h0;
        h1 += h1;
        h2 += h2;
        h3 += h3;
        h4 += h4;
        h5 += h5;
        h6 += h6;
        h7 += h7;
        h8 += h8;
        h9 += h9;
    }

    carry0 = (h0 + (1i64 << 25)) >> 26;
    h1 += carry0;
    h0 -= carry0 * (1i64 << 26);
    carry4 = (h4 + (1i64 << 25)) >> 26;
    h5 += carry4;
    h4 -= carry4 * (1i64 << 26);

    carry1 = (h1 + (1i64 << 24)) >> 25;
    h2 += carry1;
    h1 -= carry1 * (1i64 << 25);
    carry5 = (h5 + (1i64 << 24)) >> 25;
    h6 += carry5;
    h5 -= carry5 * (1i64 << 25);

    carry2 = (h2 + (1i64 << 25)) >> 26;
    h3 += carry2;
    h2 -= carry2 * (1i64 << 26);
    carry6 = (h6 + (1i64 << 25)) >> 26;
    h7 += carry6;
    h6 -= carry6 * (1i64 << 26);

    carry3 = (h3 + (1i64 << 24)) >> 25;
    h4 += carry3;
    h3 -= carry3 * (1i64 << 25);
    carry7 = (h7 + (1i64 << 24)) >> 25;
    h8 += carry7;
    h7 -= carry7 * (1i64 << 25);

    carry4 = (h4 + (1i64 << 25)) >> 26;
    h5 += carry4;
    h4 -= carry4 * (1i64 << 26);
    carry8 = (h8 + (1i64 << 25)) >> 26;
    h9 += carry8;
    h8 -= carry8 * (1i64 << 26);

    carry9 = (h9 + (1i64 << 24)) >> 25;
    h0 += carry9 * 19;
    h9 -= carry9 * (1i64 << 25);

    carry0 = (h0 + (1i64 << 25)) >> 26;
    h1 += carry0;
    h0 -= carry0 * (1i64 << 26);

    [
        h0 as i32, h1 as i32, h2 as i32, h3 as i32, h4 as i32, h5 as i32, h6 as i32, h7 as i32,
        h8 as i32, h9 as i32,
    ]
}

pub fn fe_mul32(f: Fe, n: u32) -> Fe {
    let sn = n as i64;
    let mut h0 = f[0] as i64 * sn;
    let mut h1 = f[1] as i64 * sn;
    let mut h2 = f[2] as i64 * sn;
    let mut h3 = f[3] as i64 * sn;
    let mut h4 = f[4] as i64 * sn;
    let mut h5 = f[5] as i64 * sn;
    let mut h6 = f[6] as i64 * sn;
    let mut h7 = f[7] as i64 * sn;
    let mut h8 = f[8] as i64 * sn;
    let mut h9 = f[9] as i64 * sn;

    let carry9 = (h9 + (1i64 << 24)) >> 25;
    h0 += carry9 * 19;
    h9 -= carry9 * (1i64 << 25);
    let carry1 = (h1 + (1i64 << 24)) >> 25;
    h2 += carry1;
    h1 -= carry1 * (1i64 << 25);
    let carry3 = (h3 + (1i64 << 24)) >> 25;
    h4 += carry3;
    h3 -= carry3 * (1i64 << 25);
    let carry5 = (h5 + (1i64 << 24)) >> 25;
    h6 += carry5;
    h5 -= carry5 * (1i64 << 25);
    let carry7 = (h7 + (1i64 << 24)) >> 25;
    h8 += carry7;
    h7 -= carry7 * (1i64 << 25);

    let carry0 = (h0 + (1i64 << 25)) >> 26;
    h1 += carry0;
    h0 -= carry0 * (1i64 << 26);
    let carry2 = (h2 + (1i64 << 25)) >> 26;
    h3 += carry2;
    h2 -= carry2 * (1i64 << 26);
    let carry4 = (h4 + (1i64 << 25)) >> 26;
    h5 += carry4;
    h4 -= carry4 * (1i64 << 26);
    let carry6 = (h6 + (1i64 << 25)) >> 26;
    h7 += carry6;
    h6 -= carry6 * (1i64 << 26);
    let carry8 = (h8 + (1i64 << 25)) >> 26;
    h9 += carry8;
    h8 -= carry8 * (1i64 << 26);

    [
        h0 as i32, h1 as i32, h2 as i32, h3 as i32, h4 as i32, h5 as i32, h6 as i32, h7 as i32,
        h8 as i32, h9 as i32,
    ]
}

pub fn fe_frombytes(s: &[u8]) -> Fe {
    let mut h0 = load_4(&s[0..]) as i64;
    let mut h1 = (load_3(&s[4..]) << 6) as i64;
    let mut h2 = (load_3(&s[7..]) << 5) as i64;
    let mut h3 = (load_3(&s[10..]) << 3) as i64;
    let mut h4 = (load_3(&s[13..]) << 2) as i64;
    let mut h5 = load_4(&s[16..]) as i64;
    let mut h6 = (load_3(&s[20..]) << 7) as i64;
    let mut h7 = (load_3(&s[23..]) << 5) as i64;
    let mut h8 = (load_3(&s[26..]) << 4) as i64;
    let mut h9 = ((load_3(&s[29..]) & 8388607) << 2) as i64;

    let carry9 = (h9 + (1i64 << 24)) >> 25;
    h0 += carry9 * 19;
    h9 -= carry9 * (1i64 << 25);
    let carry1 = (h1 + (1i64 << 24)) >> 25;
    h2 += carry1;
    h1 -= carry1 * (1i64 << 25);
    let carry3 = (h3 + (1i64 << 24)) >> 25;
    h4 += carry3;
    h3 -= carry3 * (1i64 << 25);
    let carry5 = (h5 + (1i64 << 24)) >> 25;
    h6 += carry5;
    h5 -= carry5 * (1i64 << 25);
    let carry7 = (h7 + (1i64 << 24)) >> 25;
    h8 += carry7;
    h7 -= carry7 * (1i64 << 25);

    let carry0 = (h0 + (1i64 << 25)) >> 26;
    h1 += carry0;
    h0 -= carry0 * (1i64 << 26);
    let carry2 = (h2 + (1i64 << 25)) >> 26;
    h3 += carry2;
    h2 -= carry2 * (1i64 << 26);
    let carry4 = (h4 + (1i64 << 25)) >> 26;
    h5 += carry4;
    h4 -= carry4 * (1i64 << 26);
    let carry6 = (h6 + (1i64 << 25)) >> 26;
    h7 += carry6;
    h6 -= carry6 * (1i64 << 26);
    let carry8 = (h8 + (1i64 << 25)) >> 26;
    h9 += carry8;
    h8 -= carry8 * (1i64 << 26);

    [
        h0 as i32, h1 as i32, h2 as i32, h3 as i32, h4 as i32, h5 as i32, h6 as i32, h7 as i32,
        h8 as i32, h9 as i32,
    ]
}

fn fe_reduce(f: Fe) -> Fe {
    let mut h0 = f[0];
    let mut h1 = f[1];
    let mut h2 = f[2];
    let mut h3 = f[3];
    let mut h4 = f[4];
    let mut h5 = f[5];
    let mut h6 = f[6];
    let mut h7 = f[7];
    let mut h8 = f[8];
    let mut h9 = f[9];

    let mut q: i32;
    // C: q = (19*h9 + (uint32_t)1<<24) >> 25; the constant makes this unsigned.
    q = (((19i32.wrapping_mul(h9)) as u32).wrapping_add(1u32 << 24) >> 25) as i32;
    q = (h0.wrapping_add(q)) >> 26;
    q = (h1.wrapping_add(q)) >> 25;
    q = (h2.wrapping_add(q)) >> 26;
    q = (h3.wrapping_add(q)) >> 25;
    q = (h4.wrapping_add(q)) >> 26;
    q = (h5.wrapping_add(q)) >> 25;
    q = (h6.wrapping_add(q)) >> 26;
    q = (h7.wrapping_add(q)) >> 25;
    q = (h8.wrapping_add(q)) >> 26;
    q = (h9.wrapping_add(q)) >> 25;

    h0 = h0.wrapping_add(19i32.wrapping_mul(q));

    let mut carry0 = h0 >> 26;
    h1 += carry0;
    h0 -= carry0 * (1i32 << 26);
    let carry1 = h1 >> 25;
    h2 += carry1;
    h1 -= carry1 * (1i32 << 25);
    let carry2 = h2 >> 26;
    h3 += carry2;
    h2 -= carry2 * (1i32 << 26);
    let carry3 = h3 >> 25;
    h4 += carry3;
    h3 -= carry3 * (1i32 << 25);
    let carry4 = h4 >> 26;
    h5 += carry4;
    h4 -= carry4 * (1i32 << 26);
    let carry5 = h5 >> 25;
    h6 += carry5;
    h5 -= carry5 * (1i32 << 25);
    let carry6 = h6 >> 26;
    h7 += carry6;
    h6 -= carry6 * (1i32 << 26);
    let carry7 = h7 >> 25;
    h8 += carry7;
    h7 -= carry7 * (1i32 << 25);
    let carry8 = h8 >> 26;
    h9 += carry8;
    h8 -= carry8 * (1i32 << 26);
    let carry9 = h9 >> 25;
    h9 -= carry9 * (1i32 << 25);
    let _ = &mut carry0;

    [h0, h1, h2, h3, h4, h5, h6, h7, h8, h9]
}

pub fn fe_tobytes(h: Fe) -> [u8; 32] {
    let t = fe_reduce(h);
    let mut s = [0u8; 32];
    s[0] = (t[0] >> 0) as u8;
    s[1] = (t[0] >> 8) as u8;
    s[2] = (t[0] >> 16) as u8;
    s[3] = ((t[0] >> 24) | (t[1].wrapping_mul(1 << 2))) as u8;
    s[4] = (t[1] >> 6) as u8;
    s[5] = (t[1] >> 14) as u8;
    s[6] = ((t[1] >> 22) | (t[2].wrapping_mul(1 << 3))) as u8;
    s[7] = (t[2] >> 5) as u8;
    s[8] = (t[2] >> 13) as u8;
    s[9] = ((t[2] >> 21) | (t[3].wrapping_mul(1 << 5))) as u8;
    s[10] = (t[3] >> 3) as u8;
    s[11] = (t[3] >> 11) as u8;
    s[12] = ((t[3] >> 19) | (t[4].wrapping_mul(1 << 6))) as u8;
    s[13] = (t[4] >> 2) as u8;
    s[14] = (t[4] >> 10) as u8;
    s[15] = (t[4] >> 18) as u8;
    s[16] = (t[5] >> 0) as u8;
    s[17] = (t[5] >> 8) as u8;
    s[18] = (t[5] >> 16) as u8;
    s[19] = ((t[5] >> 24) | (t[6].wrapping_mul(1 << 1))) as u8;
    s[20] = (t[6] >> 7) as u8;
    s[21] = (t[6] >> 15) as u8;
    s[22] = ((t[6] >> 23) | (t[7].wrapping_mul(1 << 3))) as u8;
    s[23] = (t[7] >> 5) as u8;
    s[24] = (t[7] >> 13) as u8;
    s[25] = ((t[7] >> 21) | (t[8].wrapping_mul(1 << 4))) as u8;
    s[26] = (t[8] >> 4) as u8;
    s[27] = (t[8] >> 12) as u8;
    s[28] = ((t[8] >> 20) | (t[9].wrapping_mul(1 << 6))) as u8;
    s[29] = (t[9] >> 2) as u8;
    s[30] = (t[9] >> 10) as u8;
    s[31] = (t[9] >> 18) as u8;
    s
}

fn fe_sqmul(mut s: Fe, n: i32, a: Fe) -> Fe {
    for _ in 0..n {
        s = fe_sq(s);
    }
    fe_mul(s, a)
}

pub fn fe_invert(z: Fe) -> Fe {
    let mut t0;
    let mut t1;
    let mut t2;
    let mut t3;

    t0 = fe_sq(z);
    t1 = fe_sq(t0);
    t1 = fe_sq(t1);
    t1 = fe_mul(z, t1);
    t0 = fe_mul(t0, t1);
    t2 = fe_sq(t0);
    t1 = fe_mul(t1, t2);
    t2 = fe_sq(t1);
    for _ in 1..5 {
        t2 = fe_sq(t2);
    }
    t1 = fe_mul(t2, t1);
    t2 = fe_sq(t1);
    for _ in 1..10 {
        t2 = fe_sq(t2);
    }
    t2 = fe_mul(t2, t1);
    t3 = fe_sq(t2);
    for _ in 1..20 {
        t3 = fe_sq(t3);
    }
    t2 = fe_mul(t3, t2);
    for _ in 1..11 {
        t2 = fe_sq(t2);
    }
    t1 = fe_mul(t2, t1);
    t2 = fe_sq(t1);
    for _ in 1..50 {
        t2 = fe_sq(t2);
    }
    t2 = fe_mul(t2, t1);
    t3 = fe_sq(t2);
    for _ in 1..100 {
        t3 = fe_sq(t3);
    }
    t2 = fe_mul(t3, t2);
    for _ in 1..51 {
        t2 = fe_sq(t2);
    }
    t1 = fe_mul(t2, t1);
    for _ in 1..6 {
        t1 = fe_sq(t1);
    }
    fe_mul(t1, t0)
}

pub fn fe_pow22523(z: Fe) -> Fe {
    let mut t0;
    let mut t1;
    let mut t2;

    t0 = fe_sq(z);
    t1 = fe_sq(t0);
    t1 = fe_sq(t1);
    t1 = fe_mul(z, t1);
    t0 = fe_mul(t0, t1);
    t0 = fe_sq(t0);
    t0 = fe_mul(t1, t0);
    t1 = fe_sq(t0);
    for _ in 1..5 {
        t1 = fe_sq(t1);
    }
    t0 = fe_mul(t1, t0);
    t1 = fe_sq(t0);
    for _ in 1..10 {
        t1 = fe_sq(t1);
    }
    t1 = fe_mul(t1, t0);
    t2 = fe_sq(t1);
    for _ in 1..20 {
        t2 = fe_sq(t2);
    }
    t1 = fe_mul(t2, t1);
    for _ in 1..11 {
        t1 = fe_sq(t1);
    }
    t0 = fe_mul(t1, t0);
    t1 = fe_sq(t0);
    for _ in 1..50 {
        t1 = fe_sq(t1);
    }
    t1 = fe_mul(t1, t0);
    t2 = fe_sq(t1);
    for _ in 1..100 {
        t2 = fe_sq(t2);
    }
    t1 = fe_mul(t2, t1);
    for _ in 1..51 {
        t1 = fe_sq(t1);
    }
    t0 = fe_mul(t1, t0);
    t0 = fe_sq(t0);
    t0 = fe_sq(t0);
    fe_mul(t0, z)
}

pub fn fe_cneg(h: &mut Fe, b: u32) {
    let negf = fe_neg(*h);
    fe_cmov(h, &negf, b);
}

pub fn fe_abs(h: &mut Fe) {
    let b = fe_isnegative(h) as u32;
    fe_cneg(h, b);
}

fn fe_unchecked_sqrt(x2: Fe) -> Fe {
    let e = fe_pow22523(x2);
    let p_root = fe_mul(e, x2);
    let m_root = fe_mul(p_root, FE25519_SQRTM1);
    let m_root2 = fe_sq(m_root);
    let e2 = fe_sub(x2, m_root2);
    let mut x = p_root;
    fe_cmov(&mut x, &m_root, fe_iszero(&e2) as u32);
    x
}

pub fn fe_sqrt(x2: Fe) -> (Fe, i32) {
    let x2_copy = x2;
    let x = fe_unchecked_sqrt(x2);
    let mut check = fe_sq(x);
    check = fe_sub(check, x2_copy);
    (x, fe_iszero(&check) - 1)
}

pub fn fe_notsquare(x: Fe) -> i32 {
    let _10 = fe_mul(x, x);
    let _11 = fe_mul(x, _10);
    let mut _1100 = fe_sq(_11);
    _1100 = fe_sq(_1100);
    let _1111 = fe_mul(_11, _1100);
    let mut _11110000 = fe_sq(_1111);
    _11110000 = fe_sq(_11110000);
    _11110000 = fe_sq(_11110000);
    _11110000 = fe_sq(_11110000);
    let _11111111 = fe_mul(_1111, _11110000);
    let mut t = _11111111;
    t = fe_sqmul(t, 2, _11);
    let u = t;
    t = fe_sqmul(t, 10, u);
    t = fe_sqmul(t, 10, u);
    let mut v = t;
    t = fe_sqmul(t, 30, v);
    v = t;
    t = fe_sqmul(t, 60, v);
    v = t;
    t = fe_sqmul(t, 120, v);
    t = fe_sqmul(t, 10, u);
    t = fe_sqmul(t, 3, _11);
    t = fe_sq(t);

    let s = fe_tobytes(t);
    (s[1] & 1) as i32
}

/* fe25519_reduce64 helper used by ge25519_from_hash */
pub fn fe_reduce64(h: &[u8; 64], optblocker: u8) -> Fe {
    let mut fl = [0u8; 32];
    let mut gl = [0u8; 32];
    fl.copy_from_slice(&h[0..32]);
    gl.copy_from_slice(&h[32..64]);
    fl[31] &= 0x7f;
    gl[31] &= 0x7f;
    let mut fe_f = fe_frombytes(&fl);
    let fe_g = fe_frombytes(&gl);
    fe_f[0] = fe_f[0]
        .wrapping_add((((h[31] >> 5) ^ optblocker) >> 2) as i32 * 19)
        .wrapping_add((((h[63] >> 5) ^ optblocker) >> 2) as i32 * 722);
    for i in 0..10 {
        fe_f[i] = fe_f[i].wrapping_add(38i32.wrapping_mul(fe_g[i]));
    }
    fe_reduce(fe_f)
}

/* ---- exported C-ABI symbols ---- */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_fe25519_frombytes(h: *mut i32, s: *const u8) {
    let sl = core::slice::from_raw_parts(s, 32);
    let r = fe_frombytes(sl);
    core::ptr::copy_nonoverlapping(r.as_ptr(), h, 10);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_fe25519_tobytes(s: *mut u8, h: *const i32) {
    let mut f = [0i32; 10];
    core::ptr::copy_nonoverlapping(h, f.as_mut_ptr(), 10);
    let out = fe_tobytes(f);
    core::ptr::copy_nonoverlapping(out.as_ptr(), s, 32);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_fe25519_invert(out: *mut i32, z: *const i32) {
    let mut f = [0i32; 10];
    core::ptr::copy_nonoverlapping(z, f.as_mut_ptr(), 10);
    let r = fe_invert(f);
    core::ptr::copy_nonoverlapping(r.as_ptr(), out, 10);
}
