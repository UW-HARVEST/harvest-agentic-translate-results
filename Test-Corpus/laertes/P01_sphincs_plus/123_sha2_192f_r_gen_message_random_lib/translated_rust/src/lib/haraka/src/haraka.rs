extern "C" {
    fn memcpy(
        __dest: *mut libc::c_void,
        __src: *const libc::c_void,
        __n: size_t,
    ) -> *mut libc::c_void;
}
pub type size_t = usize;
pub type __uint8_t = u8;
pub type __uint32_t = u32;
pub type __uint64_t = u64;
pub type uint8_t = __uint8_t;
pub type uint32_t = __uint32_t;
pub type uint64_t = __uint64_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct spx_ctx {
    pub pub_seed: [uint8_t; 16],
    pub sk_seed: [uint8_t; 16],
    pub tweaked512_rc64: [[uint64_t; 8]; 10],
    pub tweaked256_rc32: [[uint32_t; 8]; 10],
}
pub const SPX_N: libc::c_int = 16 as libc::c_int;
pub const HARAKAS_RATE: libc::c_int = 32 as libc::c_int;
static mut haraka512_rc64: [[uint64_t; 8]; 10] = [
    [
        0x24cf0ab9086f628b as libc::c_long as uint64_t,
        0xbdd6eeecc83b8382 as libc::c_ulong,
        0xd96fb0306cdad0a7 as libc::c_ulong,
        0xaace082ac8f95f89 as libc::c_ulong,
        0x449d8e8870d7041f as libc::c_long as uint64_t,
        0x49bb2f80b2b3e2f8 as libc::c_long as uint64_t,
        0x569ae98d93bb258 as libc::c_long as uint64_t,
        0x23dc9691e7d6a4b1 as libc::c_long as uint64_t,
    ],
    [
        0xd8ba10ede0fe5b6e as libc::c_ulong,
        0x7ecf7dbe424c7b8e as libc::c_long as uint64_t,
        0x6ea9949c6df62a31 as libc::c_long as uint64_t,
        0xbf3f3c97ec9c313e as libc::c_ulong,
        0x241d03a196a1861e as libc::c_long as uint64_t,
        0xead3a51116e5a2ea as libc::c_ulong,
        0x77d479fcad9574e3 as libc::c_long as uint64_t,
        0x18657a1af894b7a0 as libc::c_long as uint64_t,
    ],
    [
        0x10671e1a7f595522 as libc::c_long as uint64_t,
        0xd9a00ff675d28c7b as libc::c_ulong,
        0x2f1edf0d2b9ba661 as libc::c_long as uint64_t,
        0xb8ff58b8e3de45f9 as libc::c_ulong,
        0xee29261da9865c02 as libc::c_ulong,
        0xd1532aa4b50bdf43 as libc::c_ulong,
        0x8bf858159b231bb1 as libc::c_ulong,
        0xdf17439d22d4f599 as libc::c_ulong,
    ],
    [
        0xdd4b2f0870b918c0 as libc::c_ulong,
        0x757a81f3b39b1bb6 as libc::c_long as uint64_t,
        0x7a5c556898952e3f as libc::c_long as uint64_t,
        0x7dd70a16d915d87a as libc::c_long as uint64_t,
        0x3ae61971982b8301 as libc::c_long as uint64_t,
        0xc3ab319e030412be as libc::c_ulong,
        0x17c0033ac094a8cb as libc::c_long as uint64_t,
        0x5a0630fc1a8dc4ef as libc::c_long as uint64_t,
    ],
    [
        0x17708988c1632f73 as libc::c_long as uint64_t,
        0xf92ddae090b44f4f as libc::c_ulong,
        0x11ac0285c43aa314 as libc::c_long as uint64_t,
        0x509059941936b8ba as libc::c_long as uint64_t,
        0xd03e152fa2ce9b69 as libc::c_ulong,
        0x3fbcbcb63a32998b as libc::c_long as uint64_t,
        0x6204696d692254f7 as libc::c_long as uint64_t,
        0x915542ed93ec59b4 as libc::c_ulong,
    ],
    [
        0xf4ed94aa8879236e as libc::c_ulong,
        0xff6cb41cd38e03c0 as libc::c_ulong,
        0x69b38602368aeab as libc::c_long as uint64_t,
        0x669495b820f0ddba as libc::c_long as uint64_t,
        0xf42013b1b8bf9e3d as libc::c_ulong,
        0xcf935efe6439734d as libc::c_ulong,
        0xbc1dcf42ca29e3f8 as libc::c_ulong,
        0x7e6d3ed29f78ad67 as libc::c_long as uint64_t,
    ],
    [
        0xf3b0f6837ffcddaa as libc::c_ulong,
        0x3a76faef934ddf41 as libc::c_long as uint64_t,
        0xcec7ae583a9c8e35 as libc::c_ulong,
        0xe4dd18c68f0260af as libc::c_ulong,
        0x2c0e5df1ad398eaa as libc::c_long as uint64_t,
        0x478df5236ae22e8c as libc::c_long as uint64_t,
        0xfb944c46fe865f39 as libc::c_ulong,
        0xaa48f82f028132ba as libc::c_ulong,
    ],
    [
        0x231b9ae2b76aca77 as libc::c_long as uint64_t,
        0x292a76a712db0b40 as libc::c_long as uint64_t,
        0x5850625dc8134491 as libc::c_long as uint64_t,
        0x73137dd469810fb5 as libc::c_long as uint64_t,
        0x8a12a6a202a474fd as libc::c_ulong,
        0xd36fd9daa78bdb80 as libc::c_ulong,
        0xb34c5e733505706f as libc::c_ulong,
        0xbaf1cdca818d9d96 as libc::c_ulong,
    ],
    [
        0x2e99781335e8c641 as libc::c_long as uint64_t,
        0xbddfe5cce47d560e as libc::c_ulong,
        0xf74e9bf32e5e040c as libc::c_ulong,
        0x1d7a709d65996be9 as libc::c_long as uint64_t,
        0x670df36a9cf66cdd as libc::c_long as uint64_t,
        0xd05ef84a176a2875 as libc::c_ulong,
        0xf888e828cb1c44e as libc::c_long as uint64_t,
        0x1a79e9c9727b052c as libc::c_long as uint64_t,
    ],
    [
        0x83497348628d84de as libc::c_ulong,
        0x2e9387d51f22a754 as libc::c_long as uint64_t,
        0xb000068da2f852d6 as libc::c_ulong,
        0x378c9e1190fd6fe5 as libc::c_long as uint64_t,
        0x870027c316de7293 as libc::c_ulong,
        0xe51a9d4462e047bb as libc::c_ulong,
        0x90ecf7f8c6251195 as libc::c_ulong,
        0x655953bfbed90a9c as libc::c_long as uint64_t,
    ],
];
#[inline]
unsafe extern "C" fn br_dec32le(mut src: *const libc::c_uchar) -> uint32_t {
    return *src.offset(0 as libc::c_int as isize) as uint32_t
        | (*src.offset(1 as libc::c_int as isize) as uint32_t) << 8 as libc::c_int
        | (*src.offset(2 as libc::c_int as isize) as uint32_t) << 16 as libc::c_int
        | (*src.offset(3 as libc::c_int as isize) as uint32_t) << 24 as libc::c_int;
}
unsafe extern "C" fn br_range_dec32le(
    mut v: *mut uint32_t,
    mut num: size_t,
    mut src: *const libc::c_uchar,
) {
    loop {
        let fresh0 = num;
        num = num.wrapping_sub(1);
        if !(fresh0 > 0 as size_t) {
            break;
        }
        let fresh1 = v;
        v = v.offset(1);
        *fresh1 = br_dec32le(src);
        src = src.offset(4 as libc::c_int as isize);
    }
}
#[inline]
unsafe extern "C" fn br_enc32le(mut dst: *mut libc::c_uchar, mut x: uint32_t) {
    *dst.offset(0 as libc::c_int as isize) = x as libc::c_uchar;
    *dst.offset(1 as libc::c_int as isize) =
        (x >> 8 as libc::c_int) as libc::c_uchar;
    *dst.offset(2 as libc::c_int as isize) =
        (x >> 16 as libc::c_int) as libc::c_uchar;
    *dst.offset(3 as libc::c_int as isize) =
        (x >> 24 as libc::c_int) as libc::c_uchar;
}
unsafe extern "C" fn br_range_enc32le(
    mut dst: *mut libc::c_uchar,
    mut v: *const uint32_t,
    mut num: size_t,
) {
    loop {
        let fresh2 = num;
        num = num.wrapping_sub(1);
        if !(fresh2 > 0 as size_t) {
            break;
        }
        let fresh3 = v;
        v = v.offset(1);
        br_enc32le(dst, *fresh3);
        dst = dst.offset(4 as libc::c_int as isize);
    }
}
unsafe extern "C" fn br_aes_ct64_bitslice_Sbox(mut q: *mut uint64_t) {
    let mut x0: uint64_t = 0;
    let mut x1: uint64_t = 0;
    let mut x2: uint64_t = 0;
    let mut x3: uint64_t = 0;
    let mut x4: uint64_t = 0;
    let mut x5: uint64_t = 0;
    let mut x6: uint64_t = 0;
    let mut x7: uint64_t = 0;
    let mut y1: uint64_t = 0;
    let mut y2: uint64_t = 0;
    let mut y3: uint64_t = 0;
    let mut y4: uint64_t = 0;
    let mut y5: uint64_t = 0;
    let mut y6: uint64_t = 0;
    let mut y7: uint64_t = 0;
    let mut y8: uint64_t = 0;
    let mut y9: uint64_t = 0;
    let mut y10: uint64_t = 0;
    let mut y11: uint64_t = 0;
    let mut y12: uint64_t = 0;
    let mut y13: uint64_t = 0;
    let mut y14: uint64_t = 0;
    let mut y15: uint64_t = 0;
    let mut y16: uint64_t = 0;
    let mut y17: uint64_t = 0;
    let mut y18: uint64_t = 0;
    let mut y19: uint64_t = 0;
    let mut y20: uint64_t = 0;
    let mut y21: uint64_t = 0;
    let mut z0: uint64_t = 0;
    let mut z1: uint64_t = 0;
    let mut z2: uint64_t = 0;
    let mut z3: uint64_t = 0;
    let mut z4: uint64_t = 0;
    let mut z5: uint64_t = 0;
    let mut z6: uint64_t = 0;
    let mut z7: uint64_t = 0;
    let mut z8: uint64_t = 0;
    let mut z9: uint64_t = 0;
    let mut z10: uint64_t = 0;
    let mut z11: uint64_t = 0;
    let mut z12: uint64_t = 0;
    let mut z13: uint64_t = 0;
    let mut z14: uint64_t = 0;
    let mut z15: uint64_t = 0;
    let mut z16: uint64_t = 0;
    let mut z17: uint64_t = 0;
    let mut t0: uint64_t = 0;
    let mut t1: uint64_t = 0;
    let mut t2: uint64_t = 0;
    let mut t3: uint64_t = 0;
    let mut t4: uint64_t = 0;
    let mut t5: uint64_t = 0;
    let mut t6: uint64_t = 0;
    let mut t7: uint64_t = 0;
    let mut t8: uint64_t = 0;
    let mut t9: uint64_t = 0;
    let mut t10: uint64_t = 0;
    let mut t11: uint64_t = 0;
    let mut t12: uint64_t = 0;
    let mut t13: uint64_t = 0;
    let mut t14: uint64_t = 0;
    let mut t15: uint64_t = 0;
    let mut t16: uint64_t = 0;
    let mut t17: uint64_t = 0;
    let mut t18: uint64_t = 0;
    let mut t19: uint64_t = 0;
    let mut t20: uint64_t = 0;
    let mut t21: uint64_t = 0;
    let mut t22: uint64_t = 0;
    let mut t23: uint64_t = 0;
    let mut t24: uint64_t = 0;
    let mut t25: uint64_t = 0;
    let mut t26: uint64_t = 0;
    let mut t27: uint64_t = 0;
    let mut t28: uint64_t = 0;
    let mut t29: uint64_t = 0;
    let mut t30: uint64_t = 0;
    let mut t31: uint64_t = 0;
    let mut t32: uint64_t = 0;
    let mut t33: uint64_t = 0;
    let mut t34: uint64_t = 0;
    let mut t35: uint64_t = 0;
    let mut t36: uint64_t = 0;
    let mut t37: uint64_t = 0;
    let mut t38: uint64_t = 0;
    let mut t39: uint64_t = 0;
    let mut t40: uint64_t = 0;
    let mut t41: uint64_t = 0;
    let mut t42: uint64_t = 0;
    let mut t43: uint64_t = 0;
    let mut t44: uint64_t = 0;
    let mut t45: uint64_t = 0;
    let mut t46: uint64_t = 0;
    let mut t47: uint64_t = 0;
    let mut t48: uint64_t = 0;
    let mut t49: uint64_t = 0;
    let mut t50: uint64_t = 0;
    let mut t51: uint64_t = 0;
    let mut t52: uint64_t = 0;
    let mut t53: uint64_t = 0;
    let mut t54: uint64_t = 0;
    let mut t55: uint64_t = 0;
    let mut t56: uint64_t = 0;
    let mut t57: uint64_t = 0;
    let mut t58: uint64_t = 0;
    let mut t59: uint64_t = 0;
    let mut t60: uint64_t = 0;
    let mut t61: uint64_t = 0;
    let mut t62: uint64_t = 0;
    let mut t63: uint64_t = 0;
    let mut t64: uint64_t = 0;
    let mut t65: uint64_t = 0;
    let mut t66: uint64_t = 0;
    let mut t67: uint64_t = 0;
    let mut s0: uint64_t = 0;
    let mut s1: uint64_t = 0;
    let mut s2: uint64_t = 0;
    let mut s3: uint64_t = 0;
    let mut s4: uint64_t = 0;
    let mut s5: uint64_t = 0;
    let mut s6: uint64_t = 0;
    let mut s7: uint64_t = 0;
    x0 = *q.offset(7 as libc::c_int as isize);
    x1 = *q.offset(6 as libc::c_int as isize);
    x2 = *q.offset(5 as libc::c_int as isize);
    x3 = *q.offset(4 as libc::c_int as isize);
    x4 = *q.offset(3 as libc::c_int as isize);
    x5 = *q.offset(2 as libc::c_int as isize);
    x6 = *q.offset(1 as libc::c_int as isize);
    x7 = *q.offset(0 as libc::c_int as isize);
    y14 = x3 ^ x5;
    y13 = x0 ^ x6;
    y9 = x0 ^ x3;
    y8 = x0 ^ x5;
    t0 = x1 ^ x2;
    y1 = t0 ^ x7;
    y4 = y1 ^ x3;
    y12 = y13 ^ y14;
    y2 = y1 ^ x0;
    y5 = y1 ^ x6;
    y3 = y5 ^ y8;
    t1 = x4 ^ y12;
    y15 = t1 ^ x5;
    y20 = t1 ^ x1;
    y6 = y15 ^ x7;
    y10 = y15 ^ t0;
    y11 = y20 ^ y9;
    y7 = x7 ^ y11;
    y17 = y10 ^ y11;
    y19 = y10 ^ y8;
    y16 = t0 ^ y11;
    y21 = y13 ^ y16;
    y18 = x0 ^ y16;
    t2 = y12 & y15;
    t3 = y3 & y6;
    t4 = t3 ^ t2;
    t5 = y4 & x7;
    t6 = t5 ^ t2;
    t7 = y13 & y16;
    t8 = y5 & y1;
    t9 = t8 ^ t7;
    t10 = y2 & y7;
    t11 = t10 ^ t7;
    t12 = y9 & y11;
    t13 = y14 & y17;
    t14 = t13 ^ t12;
    t15 = y8 & y10;
    t16 = t15 ^ t12;
    t17 = t4 ^ t14;
    t18 = t6 ^ t16;
    t19 = t9 ^ t14;
    t20 = t11 ^ t16;
    t21 = t17 ^ y20;
    t22 = t18 ^ y19;
    t23 = t19 ^ y21;
    t24 = t20 ^ y18;
    t25 = t21 ^ t22;
    t26 = t21 & t23;
    t27 = t24 ^ t26;
    t28 = t25 & t27;
    t29 = t28 ^ t22;
    t30 = t23 ^ t24;
    t31 = t22 ^ t26;
    t32 = t31 & t30;
    t33 = t32 ^ t24;
    t34 = t23 ^ t33;
    t35 = t27 ^ t33;
    t36 = t24 & t35;
    t37 = t36 ^ t34;
    t38 = t27 ^ t36;
    t39 = t29 & t38;
    t40 = t25 ^ t39;
    t41 = t40 ^ t37;
    t42 = t29 ^ t33;
    t43 = t29 ^ t40;
    t44 = t33 ^ t37;
    t45 = t42 ^ t41;
    z0 = t44 & y15;
    z1 = t37 & y6;
    z2 = t33 & x7;
    z3 = t43 & y16;
    z4 = t40 & y1;
    z5 = t29 & y7;
    z6 = t42 & y11;
    z7 = t45 & y17;
    z8 = t41 & y10;
    z9 = t44 & y12;
    z10 = t37 & y3;
    z11 = t33 & y4;
    z12 = t43 & y13;
    z13 = t40 & y5;
    z14 = t29 & y2;
    z15 = t42 & y9;
    z16 = t45 & y14;
    z17 = t41 & y8;
    t46 = z15 ^ z16;
    t47 = z10 ^ z11;
    t48 = z5 ^ z13;
    t49 = z9 ^ z10;
    t50 = z2 ^ z12;
    t51 = z2 ^ z5;
    t52 = z7 ^ z8;
    t53 = z0 ^ z3;
    t54 = z6 ^ z7;
    t55 = z16 ^ z17;
    t56 = z12 ^ t48;
    t57 = t50 ^ t53;
    t58 = z4 ^ t46;
    t59 = z3 ^ t54;
    t60 = t46 ^ t57;
    t61 = z14 ^ t57;
    t62 = t52 ^ t58;
    t63 = t49 ^ t58;
    t64 = z4 ^ t59;
    t65 = t61 ^ t62;
    t66 = z1 ^ t63;
    s0 = t59 ^ t63;
    s6 = t56 ^ !t62;
    s7 = t48 ^ !t60;
    t67 = t64 ^ t65;
    s3 = t53 ^ t66;
    s4 = t51 ^ t66;
    s5 = t47 ^ t65;
    s1 = t64 ^ !s3;
    s2 = t55 ^ !t67;
    *q.offset(7 as libc::c_int as isize) = s0;
    *q.offset(6 as libc::c_int as isize) = s1;
    *q.offset(5 as libc::c_int as isize) = s2;
    *q.offset(4 as libc::c_int as isize) = s3;
    *q.offset(3 as libc::c_int as isize) = s4;
    *q.offset(2 as libc::c_int as isize) = s5;
    *q.offset(1 as libc::c_int as isize) = s6;
    *q.offset(0 as libc::c_int as isize) = s7;
}
unsafe extern "C" fn br_aes_ct_bitslice_Sbox(mut q: *mut uint32_t) {
    let mut x0: uint32_t = 0;
    let mut x1: uint32_t = 0;
    let mut x2: uint32_t = 0;
    let mut x3: uint32_t = 0;
    let mut x4: uint32_t = 0;
    let mut x5: uint32_t = 0;
    let mut x6: uint32_t = 0;
    let mut x7: uint32_t = 0;
    let mut y1: uint32_t = 0;
    let mut y2: uint32_t = 0;
    let mut y3: uint32_t = 0;
    let mut y4: uint32_t = 0;
    let mut y5: uint32_t = 0;
    let mut y6: uint32_t = 0;
    let mut y7: uint32_t = 0;
    let mut y8: uint32_t = 0;
    let mut y9: uint32_t = 0;
    let mut y10: uint32_t = 0;
    let mut y11: uint32_t = 0;
    let mut y12: uint32_t = 0;
    let mut y13: uint32_t = 0;
    let mut y14: uint32_t = 0;
    let mut y15: uint32_t = 0;
    let mut y16: uint32_t = 0;
    let mut y17: uint32_t = 0;
    let mut y18: uint32_t = 0;
    let mut y19: uint32_t = 0;
    let mut y20: uint32_t = 0;
    let mut y21: uint32_t = 0;
    let mut z0: uint32_t = 0;
    let mut z1: uint32_t = 0;
    let mut z2: uint32_t = 0;
    let mut z3: uint32_t = 0;
    let mut z4: uint32_t = 0;
    let mut z5: uint32_t = 0;
    let mut z6: uint32_t = 0;
    let mut z7: uint32_t = 0;
    let mut z8: uint32_t = 0;
    let mut z9: uint32_t = 0;
    let mut z10: uint32_t = 0;
    let mut z11: uint32_t = 0;
    let mut z12: uint32_t = 0;
    let mut z13: uint32_t = 0;
    let mut z14: uint32_t = 0;
    let mut z15: uint32_t = 0;
    let mut z16: uint32_t = 0;
    let mut z17: uint32_t = 0;
    let mut t0: uint32_t = 0;
    let mut t1: uint32_t = 0;
    let mut t2: uint32_t = 0;
    let mut t3: uint32_t = 0;
    let mut t4: uint32_t = 0;
    let mut t5: uint32_t = 0;
    let mut t6: uint32_t = 0;
    let mut t7: uint32_t = 0;
    let mut t8: uint32_t = 0;
    let mut t9: uint32_t = 0;
    let mut t10: uint32_t = 0;
    let mut t11: uint32_t = 0;
    let mut t12: uint32_t = 0;
    let mut t13: uint32_t = 0;
    let mut t14: uint32_t = 0;
    let mut t15: uint32_t = 0;
    let mut t16: uint32_t = 0;
    let mut t17: uint32_t = 0;
    let mut t18: uint32_t = 0;
    let mut t19: uint32_t = 0;
    let mut t20: uint32_t = 0;
    let mut t21: uint32_t = 0;
    let mut t22: uint32_t = 0;
    let mut t23: uint32_t = 0;
    let mut t24: uint32_t = 0;
    let mut t25: uint32_t = 0;
    let mut t26: uint32_t = 0;
    let mut t27: uint32_t = 0;
    let mut t28: uint32_t = 0;
    let mut t29: uint32_t = 0;
    let mut t30: uint32_t = 0;
    let mut t31: uint32_t = 0;
    let mut t32: uint32_t = 0;
    let mut t33: uint32_t = 0;
    let mut t34: uint32_t = 0;
    let mut t35: uint32_t = 0;
    let mut t36: uint32_t = 0;
    let mut t37: uint32_t = 0;
    let mut t38: uint32_t = 0;
    let mut t39: uint32_t = 0;
    let mut t40: uint32_t = 0;
    let mut t41: uint32_t = 0;
    let mut t42: uint32_t = 0;
    let mut t43: uint32_t = 0;
    let mut t44: uint32_t = 0;
    let mut t45: uint32_t = 0;
    let mut t46: uint32_t = 0;
    let mut t47: uint32_t = 0;
    let mut t48: uint32_t = 0;
    let mut t49: uint32_t = 0;
    let mut t50: uint32_t = 0;
    let mut t51: uint32_t = 0;
    let mut t52: uint32_t = 0;
    let mut t53: uint32_t = 0;
    let mut t54: uint32_t = 0;
    let mut t55: uint32_t = 0;
    let mut t56: uint32_t = 0;
    let mut t57: uint32_t = 0;
    let mut t58: uint32_t = 0;
    let mut t59: uint32_t = 0;
    let mut t60: uint32_t = 0;
    let mut t61: uint32_t = 0;
    let mut t62: uint32_t = 0;
    let mut t63: uint32_t = 0;
    let mut t64: uint32_t = 0;
    let mut t65: uint32_t = 0;
    let mut t66: uint32_t = 0;
    let mut t67: uint32_t = 0;
    let mut s0: uint32_t = 0;
    let mut s1: uint32_t = 0;
    let mut s2: uint32_t = 0;
    let mut s3: uint32_t = 0;
    let mut s4: uint32_t = 0;
    let mut s5: uint32_t = 0;
    let mut s6: uint32_t = 0;
    let mut s7: uint32_t = 0;
    x0 = *q.offset(7 as libc::c_int as isize);
    x1 = *q.offset(6 as libc::c_int as isize);
    x2 = *q.offset(5 as libc::c_int as isize);
    x3 = *q.offset(4 as libc::c_int as isize);
    x4 = *q.offset(3 as libc::c_int as isize);
    x5 = *q.offset(2 as libc::c_int as isize);
    x6 = *q.offset(1 as libc::c_int as isize);
    x7 = *q.offset(0 as libc::c_int as isize);
    y14 = x3 ^ x5;
    y13 = x0 ^ x6;
    y9 = x0 ^ x3;
    y8 = x0 ^ x5;
    t0 = x1 ^ x2;
    y1 = t0 ^ x7;
    y4 = y1 ^ x3;
    y12 = y13 ^ y14;
    y2 = y1 ^ x0;
    y5 = y1 ^ x6;
    y3 = y5 ^ y8;
    t1 = x4 ^ y12;
    y15 = t1 ^ x5;
    y20 = t1 ^ x1;
    y6 = y15 ^ x7;
    y10 = y15 ^ t0;
    y11 = y20 ^ y9;
    y7 = x7 ^ y11;
    y17 = y10 ^ y11;
    y19 = y10 ^ y8;
    y16 = t0 ^ y11;
    y21 = y13 ^ y16;
    y18 = x0 ^ y16;
    t2 = y12 & y15;
    t3 = y3 & y6;
    t4 = t3 ^ t2;
    t5 = y4 & x7;
    t6 = t5 ^ t2;
    t7 = y13 & y16;
    t8 = y5 & y1;
    t9 = t8 ^ t7;
    t10 = y2 & y7;
    t11 = t10 ^ t7;
    t12 = y9 & y11;
    t13 = y14 & y17;
    t14 = t13 ^ t12;
    t15 = y8 & y10;
    t16 = t15 ^ t12;
    t17 = t4 ^ t14;
    t18 = t6 ^ t16;
    t19 = t9 ^ t14;
    t20 = t11 ^ t16;
    t21 = t17 ^ y20;
    t22 = t18 ^ y19;
    t23 = t19 ^ y21;
    t24 = t20 ^ y18;
    t25 = t21 ^ t22;
    t26 = t21 & t23;
    t27 = t24 ^ t26;
    t28 = t25 & t27;
    t29 = t28 ^ t22;
    t30 = t23 ^ t24;
    t31 = t22 ^ t26;
    t32 = t31 & t30;
    t33 = t32 ^ t24;
    t34 = t23 ^ t33;
    t35 = t27 ^ t33;
    t36 = t24 & t35;
    t37 = t36 ^ t34;
    t38 = t27 ^ t36;
    t39 = t29 & t38;
    t40 = t25 ^ t39;
    t41 = t40 ^ t37;
    t42 = t29 ^ t33;
    t43 = t29 ^ t40;
    t44 = t33 ^ t37;
    t45 = t42 ^ t41;
    z0 = t44 & y15;
    z1 = t37 & y6;
    z2 = t33 & x7;
    z3 = t43 & y16;
    z4 = t40 & y1;
    z5 = t29 & y7;
    z6 = t42 & y11;
    z7 = t45 & y17;
    z8 = t41 & y10;
    z9 = t44 & y12;
    z10 = t37 & y3;
    z11 = t33 & y4;
    z12 = t43 & y13;
    z13 = t40 & y5;
    z14 = t29 & y2;
    z15 = t42 & y9;
    z16 = t45 & y14;
    z17 = t41 & y8;
    t46 = z15 ^ z16;
    t47 = z10 ^ z11;
    t48 = z5 ^ z13;
    t49 = z9 ^ z10;
    t50 = z2 ^ z12;
    t51 = z2 ^ z5;
    t52 = z7 ^ z8;
    t53 = z0 ^ z3;
    t54 = z6 ^ z7;
    t55 = z16 ^ z17;
    t56 = z12 ^ t48;
    t57 = t50 ^ t53;
    t58 = z4 ^ t46;
    t59 = z3 ^ t54;
    t60 = t46 ^ t57;
    t61 = z14 ^ t57;
    t62 = t52 ^ t58;
    t63 = t49 ^ t58;
    t64 = z4 ^ t59;
    t65 = t61 ^ t62;
    t66 = z1 ^ t63;
    s0 = t59 ^ t63;
    s6 = t56 ^ !t62;
    s7 = t48 ^ !t60;
    t67 = t64 ^ t65;
    s3 = t53 ^ t66;
    s4 = t51 ^ t66;
    s5 = t47 ^ t65;
    s1 = t64 ^ !s3;
    s2 = t55 ^ !t67;
    *q.offset(7 as libc::c_int as isize) = s0;
    *q.offset(6 as libc::c_int as isize) = s1;
    *q.offset(5 as libc::c_int as isize) = s2;
    *q.offset(4 as libc::c_int as isize) = s3;
    *q.offset(3 as libc::c_int as isize) = s4;
    *q.offset(2 as libc::c_int as isize) = s5;
    *q.offset(1 as libc::c_int as isize) = s6;
    *q.offset(0 as libc::c_int as isize) = s7;
}
unsafe extern "C" fn br_aes_ct_ortho(mut q: *mut uint32_t) {
    let mut a: uint32_t = 0;
    let mut b: uint32_t = 0;
    a = *q.offset(0 as libc::c_int as isize);
    b = *q.offset(1 as libc::c_int as isize);
    *q.offset(0 as libc::c_int as isize) = a & 0x55555555 as libc::c_int as uint32_t
        | (b & 0x55555555 as libc::c_int as uint32_t) << 1 as libc::c_int;
    *q.offset(1 as libc::c_int as isize) =
        (a & 0xaaaaaaaa as libc::c_uint as uint32_t) >> 1 as libc::c_int
            | b & 0xaaaaaaaa as libc::c_uint as uint32_t;
    let mut a_0: uint32_t = 0;
    let mut b_0: uint32_t = 0;
    a_0 = *q.offset(2 as libc::c_int as isize);
    b_0 = *q.offset(3 as libc::c_int as isize);
    *q.offset(2 as libc::c_int as isize) = a_0
        & 0x55555555 as libc::c_int as uint32_t
        | (b_0 & 0x55555555 as libc::c_int as uint32_t) << 1 as libc::c_int;
    *q.offset(3 as libc::c_int as isize) =
        (a_0 & 0xaaaaaaaa as libc::c_uint as uint32_t) >> 1 as libc::c_int
            | b_0 & 0xaaaaaaaa as libc::c_uint as uint32_t;
    let mut a_1: uint32_t = 0;
    let mut b_1: uint32_t = 0;
    a_1 = *q.offset(4 as libc::c_int as isize);
    b_1 = *q.offset(5 as libc::c_int as isize);
    *q.offset(4 as libc::c_int as isize) = a_1
        & 0x55555555 as libc::c_int as uint32_t
        | (b_1 & 0x55555555 as libc::c_int as uint32_t) << 1 as libc::c_int;
    *q.offset(5 as libc::c_int as isize) =
        (a_1 & 0xaaaaaaaa as libc::c_uint as uint32_t) >> 1 as libc::c_int
            | b_1 & 0xaaaaaaaa as libc::c_uint as uint32_t;
    let mut a_2: uint32_t = 0;
    let mut b_2: uint32_t = 0;
    a_2 = *q.offset(6 as libc::c_int as isize);
    b_2 = *q.offset(7 as libc::c_int as isize);
    *q.offset(6 as libc::c_int as isize) = a_2
        & 0x55555555 as libc::c_int as uint32_t
        | (b_2 & 0x55555555 as libc::c_int as uint32_t) << 1 as libc::c_int;
    *q.offset(7 as libc::c_int as isize) =
        (a_2 & 0xaaaaaaaa as libc::c_uint as uint32_t) >> 1 as libc::c_int
            | b_2 & 0xaaaaaaaa as libc::c_uint as uint32_t;
    let mut a_3: uint32_t = 0;
    let mut b_3: uint32_t = 0;
    a_3 = *q.offset(0 as libc::c_int as isize);
    b_3 = *q.offset(2 as libc::c_int as isize);
    *q.offset(0 as libc::c_int as isize) = a_3
        & 0x33333333 as libc::c_int as uint32_t
        | (b_3 & 0x33333333 as libc::c_int as uint32_t) << 2 as libc::c_int;
    *q.offset(2 as libc::c_int as isize) =
        (a_3 & 0xcccccccc as libc::c_uint as uint32_t) >> 2 as libc::c_int
            | b_3 & 0xcccccccc as libc::c_uint as uint32_t;
    let mut a_4: uint32_t = 0;
    let mut b_4: uint32_t = 0;
    a_4 = *q.offset(1 as libc::c_int as isize);
    b_4 = *q.offset(3 as libc::c_int as isize);
    *q.offset(1 as libc::c_int as isize) = a_4
        & 0x33333333 as libc::c_int as uint32_t
        | (b_4 & 0x33333333 as libc::c_int as uint32_t) << 2 as libc::c_int;
    *q.offset(3 as libc::c_int as isize) =
        (a_4 & 0xcccccccc as libc::c_uint as uint32_t) >> 2 as libc::c_int
            | b_4 & 0xcccccccc as libc::c_uint as uint32_t;
    let mut a_5: uint32_t = 0;
    let mut b_5: uint32_t = 0;
    a_5 = *q.offset(4 as libc::c_int as isize);
    b_5 = *q.offset(6 as libc::c_int as isize);
    *q.offset(4 as libc::c_int as isize) = a_5
        & 0x33333333 as libc::c_int as uint32_t
        | (b_5 & 0x33333333 as libc::c_int as uint32_t) << 2 as libc::c_int;
    *q.offset(6 as libc::c_int as isize) =
        (a_5 & 0xcccccccc as libc::c_uint as uint32_t) >> 2 as libc::c_int
            | b_5 & 0xcccccccc as libc::c_uint as uint32_t;
    let mut a_6: uint32_t = 0;
    let mut b_6: uint32_t = 0;
    a_6 = *q.offset(5 as libc::c_int as isize);
    b_6 = *q.offset(7 as libc::c_int as isize);
    *q.offset(5 as libc::c_int as isize) = a_6
        & 0x33333333 as libc::c_int as uint32_t
        | (b_6 & 0x33333333 as libc::c_int as uint32_t) << 2 as libc::c_int;
    *q.offset(7 as libc::c_int as isize) =
        (a_6 & 0xcccccccc as libc::c_uint as uint32_t) >> 2 as libc::c_int
            | b_6 & 0xcccccccc as libc::c_uint as uint32_t;
    let mut a_7: uint32_t = 0;
    let mut b_7: uint32_t = 0;
    a_7 = *q.offset(0 as libc::c_int as isize);
    b_7 = *q.offset(4 as libc::c_int as isize);
    *q.offset(0 as libc::c_int as isize) = a_7 & 0xf0f0f0f as libc::c_int as uint32_t
        | (b_7 & 0xf0f0f0f as libc::c_int as uint32_t) << 4 as libc::c_int;
    *q.offset(4 as libc::c_int as isize) =
        (a_7 & 0xf0f0f0f0 as libc::c_uint as uint32_t) >> 4 as libc::c_int
            | b_7 & 0xf0f0f0f0 as libc::c_uint as uint32_t;
    let mut a_8: uint32_t = 0;
    let mut b_8: uint32_t = 0;
    a_8 = *q.offset(1 as libc::c_int as isize);
    b_8 = *q.offset(5 as libc::c_int as isize);
    *q.offset(1 as libc::c_int as isize) = a_8 & 0xf0f0f0f as libc::c_int as uint32_t
        | (b_8 & 0xf0f0f0f as libc::c_int as uint32_t) << 4 as libc::c_int;
    *q.offset(5 as libc::c_int as isize) =
        (a_8 & 0xf0f0f0f0 as libc::c_uint as uint32_t) >> 4 as libc::c_int
            | b_8 & 0xf0f0f0f0 as libc::c_uint as uint32_t;
    let mut a_9: uint32_t = 0;
    let mut b_9: uint32_t = 0;
    a_9 = *q.offset(2 as libc::c_int as isize);
    b_9 = *q.offset(6 as libc::c_int as isize);
    *q.offset(2 as libc::c_int as isize) = a_9 & 0xf0f0f0f as libc::c_int as uint32_t
        | (b_9 & 0xf0f0f0f as libc::c_int as uint32_t) << 4 as libc::c_int;
    *q.offset(6 as libc::c_int as isize) =
        (a_9 & 0xf0f0f0f0 as libc::c_uint as uint32_t) >> 4 as libc::c_int
            | b_9 & 0xf0f0f0f0 as libc::c_uint as uint32_t;
    let mut a_10: uint32_t = 0;
    let mut b_10: uint32_t = 0;
    a_10 = *q.offset(3 as libc::c_int as isize);
    b_10 = *q.offset(7 as libc::c_int as isize);
    *q.offset(3 as libc::c_int as isize) = a_10
        & 0xf0f0f0f as libc::c_int as uint32_t
        | (b_10 & 0xf0f0f0f as libc::c_int as uint32_t) << 4 as libc::c_int;
    *q.offset(7 as libc::c_int as isize) =
        (a_10 & 0xf0f0f0f0 as libc::c_uint as uint32_t) >> 4 as libc::c_int
            | b_10 & 0xf0f0f0f0 as libc::c_uint as uint32_t;
}
#[inline]
unsafe extern "C" fn add_round_key32(mut q: *mut uint32_t, mut sk: *const uint32_t) {
    let ref mut fresh22 = *q.offset(0 as libc::c_int as isize);
    *fresh22 = (*fresh22 as libc::c_uint
        ^ *sk.offset(0 as libc::c_int as isize) as libc::c_uint)
        as uint32_t;
    let ref mut fresh23 = *q.offset(1 as libc::c_int as isize);
    *fresh23 = (*fresh23 as libc::c_uint
        ^ *sk.offset(1 as libc::c_int as isize) as libc::c_uint)
        as uint32_t;
    let ref mut fresh24 = *q.offset(2 as libc::c_int as isize);
    *fresh24 = (*fresh24 as libc::c_uint
        ^ *sk.offset(2 as libc::c_int as isize) as libc::c_uint)
        as uint32_t;
    let ref mut fresh25 = *q.offset(3 as libc::c_int as isize);
    *fresh25 = (*fresh25 as libc::c_uint
        ^ *sk.offset(3 as libc::c_int as isize) as libc::c_uint)
        as uint32_t;
    let ref mut fresh26 = *q.offset(4 as libc::c_int as isize);
    *fresh26 = (*fresh26 as libc::c_uint
        ^ *sk.offset(4 as libc::c_int as isize) as libc::c_uint)
        as uint32_t;
    let ref mut fresh27 = *q.offset(5 as libc::c_int as isize);
    *fresh27 = (*fresh27 as libc::c_uint
        ^ *sk.offset(5 as libc::c_int as isize) as libc::c_uint)
        as uint32_t;
    let ref mut fresh28 = *q.offset(6 as libc::c_int as isize);
    *fresh28 = (*fresh28 as libc::c_uint
        ^ *sk.offset(6 as libc::c_int as isize) as libc::c_uint)
        as uint32_t;
    let ref mut fresh29 = *q.offset(7 as libc::c_int as isize);
    *fresh29 = (*fresh29 as libc::c_uint
        ^ *sk.offset(7 as libc::c_int as isize) as libc::c_uint)
        as uint32_t;
}
#[inline]
unsafe extern "C" fn shift_rows32(mut q: *mut uint32_t) {
    let mut i: libc::c_int = 0;
    i = 0 as libc::c_int;
    while i < 8 as libc::c_int {
        let mut x: uint32_t = 0;
        x = *q.offset(i as isize);
        *q.offset(i as isize) = x & 0xff as uint32_t
            | (x & 0xfc00 as uint32_t) >> 2 as libc::c_int
            | (x & 0x300 as uint32_t) << 6 as libc::c_int
            | (x & 0xf00000 as uint32_t) >> 4 as libc::c_int
            | (x & 0xf0000 as uint32_t) << 4 as libc::c_int
            | (x & 0xc0000000 as uint32_t) >> 6 as libc::c_int
            | (x & 0x3f000000 as uint32_t) << 2 as libc::c_int;
        i += 1;
    }
}
#[inline]
unsafe extern "C" fn rotr16(mut x: uint32_t) -> uint32_t {
    return x << 16 as libc::c_int | x >> 16 as libc::c_int;
}
#[inline]
unsafe extern "C" fn mix_columns32(mut q: *mut uint32_t) {
    let mut q0: uint32_t = 0;
    let mut q1: uint32_t = 0;
    let mut q2: uint32_t = 0;
    let mut q3: uint32_t = 0;
    let mut q4: uint32_t = 0;
    let mut q5: uint32_t = 0;
    let mut q6: uint32_t = 0;
    let mut q7: uint32_t = 0;
    let mut r0: uint32_t = 0;
    let mut r1: uint32_t = 0;
    let mut r2: uint32_t = 0;
    let mut r3: uint32_t = 0;
    let mut r4: uint32_t = 0;
    let mut r5: uint32_t = 0;
    let mut r6: uint32_t = 0;
    let mut r7: uint32_t = 0;
    q0 = *q.offset(0 as libc::c_int as isize);
    q1 = *q.offset(1 as libc::c_int as isize);
    q2 = *q.offset(2 as libc::c_int as isize);
    q3 = *q.offset(3 as libc::c_int as isize);
    q4 = *q.offset(4 as libc::c_int as isize);
    q5 = *q.offset(5 as libc::c_int as isize);
    q6 = *q.offset(6 as libc::c_int as isize);
    q7 = *q.offset(7 as libc::c_int as isize);
    r0 = q0 >> 8 as libc::c_int | q0 << 24 as libc::c_int;
    r1 = q1 >> 8 as libc::c_int | q1 << 24 as libc::c_int;
    r2 = q2 >> 8 as libc::c_int | q2 << 24 as libc::c_int;
    r3 = q3 >> 8 as libc::c_int | q3 << 24 as libc::c_int;
    r4 = q4 >> 8 as libc::c_int | q4 << 24 as libc::c_int;
    r5 = q5 >> 8 as libc::c_int | q5 << 24 as libc::c_int;
    r6 = q6 >> 8 as libc::c_int | q6 << 24 as libc::c_int;
    r7 = q7 >> 8 as libc::c_int | q7 << 24 as libc::c_int;
    *q.offset(0 as libc::c_int as isize) = q7 ^ r7 ^ r0 ^ rotr16(q0 ^ r0);
    *q.offset(1 as libc::c_int as isize) = q0 ^ r0 ^ q7 ^ r7 ^ r1 ^ rotr16(q1 ^ r1);
    *q.offset(2 as libc::c_int as isize) = q1 ^ r1 ^ r2 ^ rotr16(q2 ^ r2);
    *q.offset(3 as libc::c_int as isize) = q2 ^ r2 ^ q7 ^ r7 ^ r3 ^ rotr16(q3 ^ r3);
    *q.offset(4 as libc::c_int as isize) = q3 ^ r3 ^ q7 ^ r7 ^ r4 ^ rotr16(q4 ^ r4);
    *q.offset(5 as libc::c_int as isize) = q4 ^ r4 ^ r5 ^ rotr16(q5 ^ r5);
    *q.offset(6 as libc::c_int as isize) = q5 ^ r5 ^ r6 ^ rotr16(q6 ^ r6);
    *q.offset(7 as libc::c_int as isize) = q6 ^ r6 ^ r7 ^ rotr16(q7 ^ r7);
}
unsafe extern "C" fn br_aes_ct64_ortho(mut q: *mut uint64_t) {
    let mut a: uint64_t = 0;
    let mut b: uint64_t = 0;
    a = *q.offset(0 as libc::c_int as isize);
    b = *q.offset(1 as libc::c_int as isize);
    *q.offset(0 as libc::c_int as isize) = a & 0x5555555555555555 as libc::c_long
        as uint64_t
        | (b & 0x5555555555555555 as libc::c_long as uint64_t) << 1 as libc::c_int;
    *q.offset(1 as libc::c_int as isize) =
        (a & 0xaaaaaaaaaaaaaaaa as libc::c_ulong as uint64_t) >> 1 as libc::c_int
            | b & 0xaaaaaaaaaaaaaaaa as libc::c_ulong as uint64_t;
    let mut a_0: uint64_t = 0;
    let mut b_0: uint64_t = 0;
    a_0 = *q.offset(2 as libc::c_int as isize);
    b_0 = *q.offset(3 as libc::c_int as isize);
    *q.offset(2 as libc::c_int as isize) = a_0
        & 0x5555555555555555 as libc::c_long as uint64_t
        | (b_0 & 0x5555555555555555 as libc::c_long as uint64_t) << 1 as libc::c_int;
    *q.offset(3 as libc::c_int as isize) =
        (a_0 & 0xaaaaaaaaaaaaaaaa as libc::c_ulong as uint64_t) >> 1 as libc::c_int
            | b_0 & 0xaaaaaaaaaaaaaaaa as libc::c_ulong as uint64_t;
    let mut a_1: uint64_t = 0;
    let mut b_1: uint64_t = 0;
    a_1 = *q.offset(4 as libc::c_int as isize);
    b_1 = *q.offset(5 as libc::c_int as isize);
    *q.offset(4 as libc::c_int as isize) = a_1
        & 0x5555555555555555 as libc::c_long as uint64_t
        | (b_1 & 0x5555555555555555 as libc::c_long as uint64_t) << 1 as libc::c_int;
    *q.offset(5 as libc::c_int as isize) =
        (a_1 & 0xaaaaaaaaaaaaaaaa as libc::c_ulong as uint64_t) >> 1 as libc::c_int
            | b_1 & 0xaaaaaaaaaaaaaaaa as libc::c_ulong as uint64_t;
    let mut a_2: uint64_t = 0;
    let mut b_2: uint64_t = 0;
    a_2 = *q.offset(6 as libc::c_int as isize);
    b_2 = *q.offset(7 as libc::c_int as isize);
    *q.offset(6 as libc::c_int as isize) = a_2
        & 0x5555555555555555 as libc::c_long as uint64_t
        | (b_2 & 0x5555555555555555 as libc::c_long as uint64_t) << 1 as libc::c_int;
    *q.offset(7 as libc::c_int as isize) =
        (a_2 & 0xaaaaaaaaaaaaaaaa as libc::c_ulong as uint64_t) >> 1 as libc::c_int
            | b_2 & 0xaaaaaaaaaaaaaaaa as libc::c_ulong as uint64_t;
    let mut a_3: uint64_t = 0;
    let mut b_3: uint64_t = 0;
    a_3 = *q.offset(0 as libc::c_int as isize);
    b_3 = *q.offset(2 as libc::c_int as isize);
    *q.offset(0 as libc::c_int as isize) = a_3
        & 0x3333333333333333 as libc::c_long as uint64_t
        | (b_3 & 0x3333333333333333 as libc::c_long as uint64_t) << 2 as libc::c_int;
    *q.offset(2 as libc::c_int as isize) =
        (a_3 & 0xcccccccccccccccc as libc::c_ulong as uint64_t) >> 2 as libc::c_int
            | b_3 & 0xcccccccccccccccc as libc::c_ulong as uint64_t;
    let mut a_4: uint64_t = 0;
    let mut b_4: uint64_t = 0;
    a_4 = *q.offset(1 as libc::c_int as isize);
    b_4 = *q.offset(3 as libc::c_int as isize);
    *q.offset(1 as libc::c_int as isize) = a_4
        & 0x3333333333333333 as libc::c_long as uint64_t
        | (b_4 & 0x3333333333333333 as libc::c_long as uint64_t) << 2 as libc::c_int;
    *q.offset(3 as libc::c_int as isize) =
        (a_4 & 0xcccccccccccccccc as libc::c_ulong as uint64_t) >> 2 as libc::c_int
            | b_4 & 0xcccccccccccccccc as libc::c_ulong as uint64_t;
    let mut a_5: uint64_t = 0;
    let mut b_5: uint64_t = 0;
    a_5 = *q.offset(4 as libc::c_int as isize);
    b_5 = *q.offset(6 as libc::c_int as isize);
    *q.offset(4 as libc::c_int as isize) = a_5
        & 0x3333333333333333 as libc::c_long as uint64_t
        | (b_5 & 0x3333333333333333 as libc::c_long as uint64_t) << 2 as libc::c_int;
    *q.offset(6 as libc::c_int as isize) =
        (a_5 & 0xcccccccccccccccc as libc::c_ulong as uint64_t) >> 2 as libc::c_int
            | b_5 & 0xcccccccccccccccc as libc::c_ulong as uint64_t;
    let mut a_6: uint64_t = 0;
    let mut b_6: uint64_t = 0;
    a_6 = *q.offset(5 as libc::c_int as isize);
    b_6 = *q.offset(7 as libc::c_int as isize);
    *q.offset(5 as libc::c_int as isize) = a_6
        & 0x3333333333333333 as libc::c_long as uint64_t
        | (b_6 & 0x3333333333333333 as libc::c_long as uint64_t) << 2 as libc::c_int;
    *q.offset(7 as libc::c_int as isize) =
        (a_6 & 0xcccccccccccccccc as libc::c_ulong as uint64_t) >> 2 as libc::c_int
            | b_6 & 0xcccccccccccccccc as libc::c_ulong as uint64_t;
    let mut a_7: uint64_t = 0;
    let mut b_7: uint64_t = 0;
    a_7 = *q.offset(0 as libc::c_int as isize);
    b_7 = *q.offset(4 as libc::c_int as isize);
    *q.offset(0 as libc::c_int as isize) = a_7
        & 0xf0f0f0f0f0f0f0f as libc::c_long as uint64_t
        | (b_7 & 0xf0f0f0f0f0f0f0f as libc::c_long as uint64_t) << 4 as libc::c_int;
    *q.offset(4 as libc::c_int as isize) =
        (a_7 & 0xf0f0f0f0f0f0f0f0 as libc::c_ulong as uint64_t) >> 4 as libc::c_int
            | b_7 & 0xf0f0f0f0f0f0f0f0 as libc::c_ulong as uint64_t;
    let mut a_8: uint64_t = 0;
    let mut b_8: uint64_t = 0;
    a_8 = *q.offset(1 as libc::c_int as isize);
    b_8 = *q.offset(5 as libc::c_int as isize);
    *q.offset(1 as libc::c_int as isize) = a_8
        & 0xf0f0f0f0f0f0f0f as libc::c_long as uint64_t
        | (b_8 & 0xf0f0f0f0f0f0f0f as libc::c_long as uint64_t) << 4 as libc::c_int;
    *q.offset(5 as libc::c_int as isize) =
        (a_8 & 0xf0f0f0f0f0f0f0f0 as libc::c_ulong as uint64_t) >> 4 as libc::c_int
            | b_8 & 0xf0f0f0f0f0f0f0f0 as libc::c_ulong as uint64_t;
    let mut a_9: uint64_t = 0;
    let mut b_9: uint64_t = 0;
    a_9 = *q.offset(2 as libc::c_int as isize);
    b_9 = *q.offset(6 as libc::c_int as isize);
    *q.offset(2 as libc::c_int as isize) = a_9
        & 0xf0f0f0f0f0f0f0f as libc::c_long as uint64_t
        | (b_9 & 0xf0f0f0f0f0f0f0f as libc::c_long as uint64_t) << 4 as libc::c_int;
    *q.offset(6 as libc::c_int as isize) =
        (a_9 & 0xf0f0f0f0f0f0f0f0 as libc::c_ulong as uint64_t) >> 4 as libc::c_int
            | b_9 & 0xf0f0f0f0f0f0f0f0 as libc::c_ulong as uint64_t;
    let mut a_10: uint64_t = 0;
    let mut b_10: uint64_t = 0;
    a_10 = *q.offset(3 as libc::c_int as isize);
    b_10 = *q.offset(7 as libc::c_int as isize);
    *q.offset(3 as libc::c_int as isize) = a_10
        & 0xf0f0f0f0f0f0f0f as libc::c_long as uint64_t
        | (b_10 & 0xf0f0f0f0f0f0f0f as libc::c_long as uint64_t) << 4 as libc::c_int;
    *q.offset(7 as libc::c_int as isize) =
        (a_10 & 0xf0f0f0f0f0f0f0f0 as libc::c_ulong as uint64_t) >> 4 as libc::c_int
            | b_10 & 0xf0f0f0f0f0f0f0f0 as libc::c_ulong as uint64_t;
}
unsafe extern "C" fn br_aes_ct64_interleave_in(
    mut q0: *mut uint64_t,
    mut q1: *mut uint64_t,
    mut w: *const uint32_t,
) {
    let mut x0: uint64_t = 0;
    let mut x1: uint64_t = 0;
    let mut x2: uint64_t = 0;
    let mut x3: uint64_t = 0;
    x0 = *w.offset(0 as libc::c_int as isize) as uint64_t;
    x1 = *w.offset(1 as libc::c_int as isize) as uint64_t;
    x2 = *w.offset(2 as libc::c_int as isize) as uint64_t;
    x3 = *w.offset(3 as libc::c_int as isize) as uint64_t;
    x0 = (x0 as libc::c_ulong | (x0 << 16 as libc::c_int) as libc::c_ulong)
        as uint64_t;
    x1 = (x1 as libc::c_ulong | (x1 << 16 as libc::c_int) as libc::c_ulong)
        as uint64_t;
    x2 = (x2 as libc::c_ulong | (x2 << 16 as libc::c_int) as libc::c_ulong)
        as uint64_t;
    x3 = (x3 as libc::c_ulong | (x3 << 16 as libc::c_int) as libc::c_ulong)
        as uint64_t;
    x0 = (x0 as libc::c_ulong
        & 0xffff0000ffff as libc::c_long as uint64_t as libc::c_ulong)
        as uint64_t;
    x1 = (x1 as libc::c_ulong
        & 0xffff0000ffff as libc::c_long as uint64_t as libc::c_ulong)
        as uint64_t;
    x2 = (x2 as libc::c_ulong
        & 0xffff0000ffff as libc::c_long as uint64_t as libc::c_ulong)
        as uint64_t;
    x3 = (x3 as libc::c_ulong
        & 0xffff0000ffff as libc::c_long as uint64_t as libc::c_ulong)
        as uint64_t;
    x0 = (x0 as libc::c_ulong | (x0 << 8 as libc::c_int) as libc::c_ulong)
        as uint64_t;
    x1 = (x1 as libc::c_ulong | (x1 << 8 as libc::c_int) as libc::c_ulong)
        as uint64_t;
    x2 = (x2 as libc::c_ulong | (x2 << 8 as libc::c_int) as libc::c_ulong)
        as uint64_t;
    x3 = (x3 as libc::c_ulong | (x3 << 8 as libc::c_int) as libc::c_ulong)
        as uint64_t;
    x0 = (x0 as libc::c_ulong
        & 0xff00ff00ff00ff as libc::c_long as uint64_t as libc::c_ulong)
        as uint64_t;
    x1 = (x1 as libc::c_ulong
        & 0xff00ff00ff00ff as libc::c_long as uint64_t as libc::c_ulong)
        as uint64_t;
    x2 = (x2 as libc::c_ulong
        & 0xff00ff00ff00ff as libc::c_long as uint64_t as libc::c_ulong)
        as uint64_t;
    x3 = (x3 as libc::c_ulong
        & 0xff00ff00ff00ff as libc::c_long as uint64_t as libc::c_ulong)
        as uint64_t;
    *q0 = x0 | x2 << 8 as libc::c_int;
    *q1 = x1 | x3 << 8 as libc::c_int;
}
unsafe extern "C" fn br_aes_ct64_interleave_out(
    mut w: *mut uint32_t,
    mut q0: uint64_t,
    mut q1: uint64_t,
) {
    let mut x0: uint64_t = 0;
    let mut x1: uint64_t = 0;
    let mut x2: uint64_t = 0;
    let mut x3: uint64_t = 0;
    x0 = q0 & 0xff00ff00ff00ff as libc::c_long as uint64_t;
    x1 = q1 & 0xff00ff00ff00ff as libc::c_long as uint64_t;
    x2 = q0 >> 8 as libc::c_int & 0xff00ff00ff00ff as libc::c_long as uint64_t;
    x3 = q1 >> 8 as libc::c_int & 0xff00ff00ff00ff as libc::c_long as uint64_t;
    x0 = (x0 as libc::c_ulong | (x0 >> 8 as libc::c_int) as libc::c_ulong)
        as uint64_t;
    x1 = (x1 as libc::c_ulong | (x1 >> 8 as libc::c_int) as libc::c_ulong)
        as uint64_t;
    x2 = (x2 as libc::c_ulong | (x2 >> 8 as libc::c_int) as libc::c_ulong)
        as uint64_t;
    x3 = (x3 as libc::c_ulong | (x3 >> 8 as libc::c_int) as libc::c_ulong)
        as uint64_t;
    x0 = (x0 as libc::c_ulong
        & 0xffff0000ffff as libc::c_long as uint64_t as libc::c_ulong)
        as uint64_t;
    x1 = (x1 as libc::c_ulong
        & 0xffff0000ffff as libc::c_long as uint64_t as libc::c_ulong)
        as uint64_t;
    x2 = (x2 as libc::c_ulong
        & 0xffff0000ffff as libc::c_long as uint64_t as libc::c_ulong)
        as uint64_t;
    x3 = (x3 as libc::c_ulong
        & 0xffff0000ffff as libc::c_long as uint64_t as libc::c_ulong)
        as uint64_t;
    *w.offset(0 as libc::c_int as isize) =
        x0 as uint32_t | (x0 >> 16 as libc::c_int) as uint32_t;
    *w.offset(1 as libc::c_int as isize) =
        x1 as uint32_t | (x1 >> 16 as libc::c_int) as uint32_t;
    *w.offset(2 as libc::c_int as isize) =
        x2 as uint32_t | (x2 >> 16 as libc::c_int) as uint32_t;
    *w.offset(3 as libc::c_int as isize) =
        x3 as uint32_t | (x3 >> 16 as libc::c_int) as uint32_t;
}
#[inline]
unsafe extern "C" fn add_round_key(mut q: *mut uint64_t, mut sk: *const uint64_t) {
    let ref mut fresh4 = *q.offset(0 as libc::c_int as isize);
    *fresh4 = (*fresh4 as libc::c_ulong
        ^ *sk.offset(0 as libc::c_int as isize) as libc::c_ulong)
        as uint64_t;
    let ref mut fresh5 = *q.offset(1 as libc::c_int as isize);
    *fresh5 = (*fresh5 as libc::c_ulong
        ^ *sk.offset(1 as libc::c_int as isize) as libc::c_ulong)
        as uint64_t;
    let ref mut fresh6 = *q.offset(2 as libc::c_int as isize);
    *fresh6 = (*fresh6 as libc::c_ulong
        ^ *sk.offset(2 as libc::c_int as isize) as libc::c_ulong)
        as uint64_t;
    let ref mut fresh7 = *q.offset(3 as libc::c_int as isize);
    *fresh7 = (*fresh7 as libc::c_ulong
        ^ *sk.offset(3 as libc::c_int as isize) as libc::c_ulong)
        as uint64_t;
    let ref mut fresh8 = *q.offset(4 as libc::c_int as isize);
    *fresh8 = (*fresh8 as libc::c_ulong
        ^ *sk.offset(4 as libc::c_int as isize) as libc::c_ulong)
        as uint64_t;
    let ref mut fresh9 = *q.offset(5 as libc::c_int as isize);
    *fresh9 = (*fresh9 as libc::c_ulong
        ^ *sk.offset(5 as libc::c_int as isize) as libc::c_ulong)
        as uint64_t;
    let ref mut fresh10 = *q.offset(6 as libc::c_int as isize);
    *fresh10 = (*fresh10 as libc::c_ulong
        ^ *sk.offset(6 as libc::c_int as isize) as libc::c_ulong)
        as uint64_t;
    let ref mut fresh11 = *q.offset(7 as libc::c_int as isize);
    *fresh11 = (*fresh11 as libc::c_ulong
        ^ *sk.offset(7 as libc::c_int as isize) as libc::c_ulong)
        as uint64_t;
}
#[inline]
unsafe extern "C" fn shift_rows(mut q: *mut uint64_t) {
    let mut i: libc::c_int = 0;
    i = 0 as libc::c_int;
    while i < 8 as libc::c_int {
        let mut x: uint64_t = 0;
        x = *q.offset(i as isize);
        *q.offset(i as isize) = x & 0xffff as libc::c_int as uint64_t
            | (x & 0xfff00000 as libc::c_uint as uint64_t) >> 4 as libc::c_int
            | (x & 0xf0000 as libc::c_int as uint64_t) << 12 as libc::c_int
            | (x & 0xff0000000000 as libc::c_long as uint64_t) >> 8 as libc::c_int
            | (x & 0xff00000000 as libc::c_long as uint64_t) << 8 as libc::c_int
            | (x & 0xf000000000000000 as libc::c_ulong as uint64_t)
                >> 12 as libc::c_int
            | (x & 0xfff000000000000 as libc::c_long as uint64_t) << 4 as libc::c_int;
        i += 1;
    }
}
#[inline]
unsafe extern "C" fn rotr32(mut x: uint64_t) -> uint64_t {
    return x << 32 as libc::c_int | x >> 32 as libc::c_int;
}
#[inline]
unsafe extern "C" fn mix_columns(mut q: *mut uint64_t) {
    let mut q0: uint64_t = 0;
    let mut q1: uint64_t = 0;
    let mut q2: uint64_t = 0;
    let mut q3: uint64_t = 0;
    let mut q4: uint64_t = 0;
    let mut q5: uint64_t = 0;
    let mut q6: uint64_t = 0;
    let mut q7: uint64_t = 0;
    let mut r0: uint64_t = 0;
    let mut r1: uint64_t = 0;
    let mut r2: uint64_t = 0;
    let mut r3: uint64_t = 0;
    let mut r4: uint64_t = 0;
    let mut r5: uint64_t = 0;
    let mut r6: uint64_t = 0;
    let mut r7: uint64_t = 0;
    q0 = *q.offset(0 as libc::c_int as isize);
    q1 = *q.offset(1 as libc::c_int as isize);
    q2 = *q.offset(2 as libc::c_int as isize);
    q3 = *q.offset(3 as libc::c_int as isize);
    q4 = *q.offset(4 as libc::c_int as isize);
    q5 = *q.offset(5 as libc::c_int as isize);
    q6 = *q.offset(6 as libc::c_int as isize);
    q7 = *q.offset(7 as libc::c_int as isize);
    r0 = q0 >> 16 as libc::c_int | q0 << 48 as libc::c_int;
    r1 = q1 >> 16 as libc::c_int | q1 << 48 as libc::c_int;
    r2 = q2 >> 16 as libc::c_int | q2 << 48 as libc::c_int;
    r3 = q3 >> 16 as libc::c_int | q3 << 48 as libc::c_int;
    r4 = q4 >> 16 as libc::c_int | q4 << 48 as libc::c_int;
    r5 = q5 >> 16 as libc::c_int | q5 << 48 as libc::c_int;
    r6 = q6 >> 16 as libc::c_int | q6 << 48 as libc::c_int;
    r7 = q7 >> 16 as libc::c_int | q7 << 48 as libc::c_int;
    *q.offset(0 as libc::c_int as isize) = q7 ^ r7 ^ r0 ^ rotr32(q0 ^ r0);
    *q.offset(1 as libc::c_int as isize) = q0 ^ r0 ^ q7 ^ r7 ^ r1 ^ rotr32(q1 ^ r1);
    *q.offset(2 as libc::c_int as isize) = q1 ^ r1 ^ r2 ^ rotr32(q2 ^ r2);
    *q.offset(3 as libc::c_int as isize) = q2 ^ r2 ^ q7 ^ r7 ^ r3 ^ rotr32(q3 ^ r3);
    *q.offset(4 as libc::c_int as isize) = q3 ^ r3 ^ q7 ^ r7 ^ r4 ^ rotr32(q4 ^ r4);
    *q.offset(5 as libc::c_int as isize) = q4 ^ r4 ^ r5 ^ rotr32(q5 ^ r5);
    *q.offset(6 as libc::c_int as isize) = q5 ^ r5 ^ r6 ^ rotr32(q6 ^ r6);
    *q.offset(7 as libc::c_int as isize) = q6 ^ r6 ^ r7 ^ rotr32(q7 ^ r7);
}
unsafe extern "C" fn interleave_constant(
    mut out: *mut uint64_t,
    mut in_0: *const libc::c_uchar,
) {
    let mut tmp_32_constant: [uint32_t; 16] = [0; 16];
    let mut i: libc::c_int = 0;
    br_range_dec32le(
        &raw mut tmp_32_constant as *mut uint32_t,
        16 as size_t,
        in_0,
    );
    i = 0 as libc::c_int;
    while i < 4 as libc::c_int {
        br_aes_ct64_interleave_in(
            out.offset(i as isize) as *mut uint64_t,
            out.offset((i + 4 as libc::c_int) as isize) as *mut uint64_t,
            (&raw mut tmp_32_constant as *mut uint32_t)
                .offset((i << 2 as libc::c_int) as isize),
        );
        i += 1;
    }
    br_aes_ct64_ortho(out);
}
unsafe extern "C" fn interleave_constant32(
    mut out: *mut uint32_t,
    mut in_0: *const libc::c_uchar,
) {
    let mut i: libc::c_int = 0;
    i = 0 as libc::c_int;
    while i < 4 as libc::c_int {
        *out.offset((2 as libc::c_int * i) as isize) =
            br_dec32le(in_0.offset((4 as libc::c_int * i) as isize));
        *out.offset((2 as libc::c_int * i + 1 as libc::c_int) as isize) = br_dec32le(
            in_0.offset((4 as libc::c_int * i) as isize)
                .offset(16 as libc::c_int as isize),
        );
        i += 1;
    }
    br_aes_ct_ortho(out);
}
#[no_mangle]
pub unsafe extern "C" fn SPX_tweak_constants(mut ctx: *mut spx_ctx) {
    let mut buf: [libc::c_uchar; 640] = [0; 640];
    let mut i: libc::c_int = 0;
    memcpy(
        &raw mut (*ctx).tweaked512_rc64 as *mut [uint64_t; 8] as *mut uint8_t
            as *mut libc::c_void,
        &raw const haraka512_rc64 as *const [uint64_t; 8] as *mut uint8_t
            as *const libc::c_void,
        (40 as libc::c_int * 16 as libc::c_int) as size_t,
    );
    SPX_haraka_S(
        &raw mut buf as *mut libc::c_uchar,
        (40 as libc::c_int * 16 as libc::c_int) as libc::c_ulonglong,
        &raw mut (*ctx).pub_seed as *mut uint8_t,
        SPX_N as libc::c_ulonglong,
        ctx,
    );
    i = 0 as libc::c_int;
    while i < 10 as libc::c_int {
        interleave_constant32(
            &raw mut *(&raw mut (*ctx).tweaked256_rc32 as *mut [uint32_t; 8]).offset(i as isize)
                as *mut uint32_t,
            (&raw mut buf as *mut libc::c_uchar)
                .offset((32 as libc::c_int * i) as isize),
        );
        interleave_constant(
            &raw mut *(&raw mut (*ctx).tweaked512_rc64 as *mut [uint64_t; 8]).offset(i as isize)
                as *mut uint64_t,
            (&raw mut buf as *mut libc::c_uchar)
                .offset((64 as libc::c_int * i) as isize),
        );
        i += 1;
    }
}
unsafe extern "C" fn haraka_S_absorb(
    mut s: *mut libc::c_uchar,
    mut r: libc::c_uint,
    mut m: *const libc::c_uchar,
    mut mlen: libc::c_ulonglong,
    mut p: libc::c_uchar,
    mut ctx: *const spx_ctx,
) {
    let mut i: libc::c_ulonglong = 0;
    let vla = r as usize;
    let mut t: Vec<uint8_t> = ::std::vec::from_elem(0, vla);
    while mlen >= r as libc::c_ulonglong {
        i = 0 as libc::c_ulonglong;
        while i < r as libc::c_ulonglong {
            let ref mut fresh12 = *s.offset(i as isize);
            *fresh12 = (*fresh12 as libc::c_int
                ^ *m.offset(i as isize) as libc::c_int)
                as libc::c_uchar;
            i = i.wrapping_add(1);
        }
        SPX_haraka512_perm(s, s, ctx);
        mlen = mlen.wrapping_sub(r as libc::c_ulonglong);
        m = m.offset(r as isize);
    }
    i = 0 as libc::c_ulonglong;
    while i < r as libc::c_ulonglong {
        *t.as_mut_ptr().offset(i as isize) = 0 as uint8_t;
        i = i.wrapping_add(1);
    }
    i = 0 as libc::c_ulonglong;
    while i < mlen {
        *t.as_mut_ptr().offset(i as isize) = *m.offset(i as isize) as uint8_t;
        i = i.wrapping_add(1);
    }
    *t.as_mut_ptr().offset(i as isize) = p as uint8_t;
    let ref mut fresh13 = *t
        .as_mut_ptr()
        .offset(r.wrapping_sub(1 as libc::c_uint) as isize);
    *fresh13 = (*fresh13 as libc::c_int | 128 as libc::c_int) as uint8_t;
    i = 0 as libc::c_ulonglong;
    while i < r as libc::c_ulonglong {
        let ref mut fresh14 = *s.offset(i as isize);
        *fresh14 = (*fresh14 as libc::c_int
            ^ *t.as_mut_ptr().offset(i as isize) as libc::c_int)
            as libc::c_uchar;
        i = i.wrapping_add(1);
    }
}
unsafe extern "C" fn haraka_S_squeezeblocks(
    mut h: *mut libc::c_uchar,
    mut nblocks: libc::c_ulonglong,
    mut s: *mut libc::c_uchar,
    mut r: libc::c_uint,
    mut ctx: *const spx_ctx,
) {
    while nblocks > 0 as libc::c_ulonglong {
        SPX_haraka512_perm(s, s, ctx);
        memcpy(
            h as *mut libc::c_void,
            s as *const libc::c_void,
            HARAKAS_RATE as size_t,
        );
        h = h.offset(r as isize);
        nblocks = nblocks.wrapping_sub(1);
    }
}
#[no_mangle]
pub unsafe extern "C" fn SPX_haraka_S_inc_init(mut s_inc: *mut uint8_t) {
    let mut i: size_t = 0;
    i = 0 as size_t;
    while i < 64 as size_t {
        *s_inc.offset(i as isize) = 0 as uint8_t;
        i = i.wrapping_add(1);
    }
    *s_inc.offset(64 as libc::c_int as isize) = 0 as uint8_t;
}
#[no_mangle]
pub unsafe extern "C" fn SPX_haraka_S_inc_absorb(
    mut s_inc: *mut uint8_t,
    mut m: *const uint8_t,
    mut mlen: size_t,
    mut ctx: *const spx_ctx,
) {
    let mut i: size_t = 0;
    while mlen.wrapping_add(*s_inc.offset(64 as libc::c_int as isize) as size_t)
        >= HARAKAS_RATE as size_t
    {
        i = 0 as size_t;
        while i
            < (HARAKAS_RATE
                - *s_inc.offset(64 as libc::c_int as isize) as libc::c_int)
                as size_t
        {
            let ref mut fresh15 = *s_inc.offset(
                (*s_inc.offset(64 as libc::c_int as isize) as size_t).wrapping_add(i)
                    as isize,
            );
            *fresh15 = (*fresh15 as libc::c_int
                ^ *m.offset(i as isize) as libc::c_int) as uint8_t;
            i = i.wrapping_add(1);
        }
        mlen = (mlen as libc::c_ulong).wrapping_sub(
            (HARAKAS_RATE - *s_inc.offset(64 as libc::c_int as isize) as libc::c_int)
                as size_t as libc::c_ulong,
        ) as size_t as size_t;
        m = m.offset(
            (HARAKAS_RATE - *s_inc.offset(64 as libc::c_int as isize) as libc::c_int)
                as isize,
        );
        *s_inc.offset(64 as libc::c_int as isize) = 0 as uint8_t;
        SPX_haraka512_perm(s_inc as *mut libc::c_uchar, s_inc, ctx);
    }
    i = 0 as size_t;
    while i < mlen {
        let ref mut fresh16 = *s_inc.offset(
            (*s_inc.offset(64 as libc::c_int as isize) as size_t).wrapping_add(i) as isize,
        );
        *fresh16 = (*fresh16 as libc::c_int ^ *m.offset(i as isize) as libc::c_int)
            as uint8_t;
        i = i.wrapping_add(1);
    }
    let ref mut fresh17 = *s_inc.offset(64 as libc::c_int as isize);
    *fresh17 = (*fresh17 as libc::c_int + mlen as uint8_t as libc::c_int) as uint8_t;
}
#[no_mangle]
pub unsafe extern "C" fn SPX_haraka_S_inc_finalize(mut s_inc: *mut uint8_t) {
    let ref mut fresh18 = *s_inc.offset(*s_inc.offset(64 as libc::c_int as isize) as isize);
    *fresh18 = (*fresh18 as libc::c_int ^ 0x1f as libc::c_int) as uint8_t;
    let ref mut fresh19 = *s_inc.offset((HARAKAS_RATE - 1 as libc::c_int) as isize);
    *fresh19 = (*fresh19 as libc::c_int ^ 128 as libc::c_int) as uint8_t;
    *s_inc.offset(64 as libc::c_int as isize) = 0 as uint8_t;
}
#[no_mangle]
pub unsafe extern "C" fn SPX_haraka_S_inc_squeeze(
    mut out: *mut uint8_t,
    mut outlen: size_t,
    mut s_inc: *mut uint8_t,
    mut ctx: *const spx_ctx,
) {
    let mut i: size_t = 0;
    i = 0 as size_t;
    while i < outlen && i < *s_inc.offset(64 as libc::c_int as isize) as size_t {
        *out.offset(i as isize) = *s_inc.offset(
            ((HARAKAS_RATE - *s_inc.offset(64 as libc::c_int as isize) as libc::c_int)
                as size_t)
                .wrapping_add(i) as isize,
        );
        i = i.wrapping_add(1);
    }
    out = out.offset(i as isize);
    outlen = (outlen as libc::c_ulong).wrapping_sub(i as libc::c_ulong) as size_t
        as size_t;
    let ref mut fresh20 = *s_inc.offset(64 as libc::c_int as isize);
    *fresh20 = (*fresh20 as libc::c_int - i as uint8_t as libc::c_int) as uint8_t;
    while outlen > 0 as size_t {
        SPX_haraka512_perm(s_inc as *mut libc::c_uchar, s_inc, ctx);
        i = 0 as size_t;
        while i < outlen && i < HARAKAS_RATE as size_t {
            *out.offset(i as isize) = *s_inc.offset(i as isize);
            i = i.wrapping_add(1);
        }
        out = out.offset(i as isize);
        outlen = (outlen as libc::c_ulong).wrapping_sub(i as libc::c_ulong) as size_t
            as size_t;
        *s_inc.offset(64 as libc::c_int as isize) =
            (HARAKAS_RATE as size_t).wrapping_sub(i) as uint8_t;
    }
}
#[no_mangle]
pub unsafe extern "C" fn SPX_haraka_S(
    mut out: *mut libc::c_uchar,
    mut outlen: libc::c_ulonglong,
    mut in_0: *const libc::c_uchar,
    mut inlen: libc::c_ulonglong,
    mut ctx: *const spx_ctx,
) {
    let mut i: libc::c_ulonglong = 0;
    let mut s: [libc::c_uchar; 64] = [0; 64];
    let mut d: [libc::c_uchar; 32] = [0; 32];
    i = 0 as libc::c_ulonglong;
    while i < 64 as libc::c_ulonglong {
        s[i as usize] = 0 as libc::c_uchar;
        i = i.wrapping_add(1);
    }
    haraka_S_absorb(
        &raw mut s as *mut libc::c_uchar,
        32 as libc::c_uint,
        in_0,
        inlen,
        0x1f as libc::c_uchar,
        ctx,
    );
    haraka_S_squeezeblocks(
        out,
        outlen.wrapping_div(32 as libc::c_ulonglong),
        &raw mut s as *mut libc::c_uchar,
        32 as libc::c_uint,
        ctx,
    );
    out = out.offset(
        outlen
            .wrapping_div(32 as libc::c_ulonglong)
            .wrapping_mul(32 as libc::c_ulonglong) as isize,
    );
    if outlen.wrapping_rem(32 as libc::c_ulonglong) != 0 {
        haraka_S_squeezeblocks(
            &raw mut d as *mut libc::c_uchar,
            1 as libc::c_ulonglong,
            &raw mut s as *mut libc::c_uchar,
            32 as libc::c_uint,
            ctx,
        );
        i = 0 as libc::c_ulonglong;
        while i < outlen.wrapping_rem(32 as libc::c_ulonglong) {
            *out.offset(i as isize) = d[i as usize];
            i = i.wrapping_add(1);
        }
    }
}
#[no_mangle]
pub unsafe extern "C" fn SPX_haraka512_perm(
    mut out: *mut libc::c_uchar,
    mut in_0: *const libc::c_uchar,
    mut ctx: *const spx_ctx,
) {
    let mut w: [uint32_t; 16] = [0; 16];
    let mut q: [uint64_t; 8] = [0; 8];
    let mut tmp_q: uint64_t = 0;
    let mut i: libc::c_uint = 0;
    let mut j: libc::c_uint = 0;
    br_range_dec32le(&raw mut w as *mut uint32_t, 16 as size_t, in_0);
    i = 0 as libc::c_uint;
    while i < 4 as libc::c_uint {
        br_aes_ct64_interleave_in(
            (&raw mut q as *mut uint64_t).offset(i as isize) as *mut uint64_t,
            (&raw mut q as *mut uint64_t).offset(i.wrapping_add(4 as libc::c_uint) as isize)
                as *mut uint64_t,
            (&raw mut w as *mut uint32_t).offset((i << 2 as libc::c_int) as isize),
        );
        i = i.wrapping_add(1);
    }
    br_aes_ct64_ortho(&raw mut q as *mut uint64_t);
    i = 0 as libc::c_uint;
    while i < 5 as libc::c_uint {
        j = 0 as libc::c_uint;
        while j < 2 as libc::c_uint {
            br_aes_ct64_bitslice_Sbox(&raw mut q as *mut uint64_t);
            shift_rows(&raw mut q as *mut uint64_t);
            mix_columns(&raw mut q as *mut uint64_t);
            add_round_key(
                &raw mut q as *mut uint64_t,
                &raw const *(&raw const (*ctx).tweaked512_rc64 as *const [uint64_t; 8])
                    .offset((2 as libc::c_uint).wrapping_mul(i).wrapping_add(j) as isize)
                    as *const uint64_t,
            );
            j = j.wrapping_add(1);
        }
        j = 0 as libc::c_uint;
        while j < 8 as libc::c_uint {
            tmp_q = q[j as usize];
            q[j as usize] = (tmp_q & 0x1000100010001 as uint64_t) << 5 as libc::c_int
                | (tmp_q & 0x2000200020002 as uint64_t) << 12 as libc::c_int
                | (tmp_q & 0x4000400040004 as uint64_t) >> 1 as libc::c_int
                | (tmp_q & 0x8000800080008 as uint64_t) << 6 as libc::c_int
                | (tmp_q & 0x20002000200020 as uint64_t) << 9 as libc::c_int
                | (tmp_q & 0x40004000400040 as uint64_t) >> 4 as libc::c_int
                | (tmp_q & 0x80008000800080 as uint64_t) << 3 as libc::c_int
                | (tmp_q & 0x2100210021002100 as uint64_t) >> 5 as libc::c_int
                | (tmp_q & 0x210021002100210 as uint64_t) << 2 as libc::c_int
                | (tmp_q & 0x800080008000800 as uint64_t) << 4 as libc::c_int
                | (tmp_q & 0x1000100010001000 as uint64_t) >> 12 as libc::c_int
                | (tmp_q & 0x4000400040004000 as uint64_t) >> 10 as libc::c_int
                | (tmp_q & 0x8400840084008400 as uint64_t) >> 3 as libc::c_int;
            j = j.wrapping_add(1);
        }
        i = i.wrapping_add(1);
    }
    br_aes_ct64_ortho(&raw mut q as *mut uint64_t);
    i = 0 as libc::c_uint;
    while i < 4 as libc::c_uint {
        br_aes_ct64_interleave_out(
            (&raw mut w as *mut uint32_t).offset((i << 2 as libc::c_int) as isize),
            q[i as usize],
            q[i.wrapping_add(4 as libc::c_uint) as usize],
        );
        i = i.wrapping_add(1);
    }
    br_range_enc32le(out, &raw mut w as *mut uint32_t, 16 as size_t);
}
#[no_mangle]
pub unsafe extern "C" fn SPX_haraka512(
    mut out: *mut libc::c_uchar,
    mut in_0: *const libc::c_uchar,
    mut ctx: *const spx_ctx,
) {
    let mut i: libc::c_int = 0;
    let mut buf: [libc::c_uchar; 64] = [0; 64];
    SPX_haraka512_perm(&raw mut buf as *mut libc::c_uchar, in_0, ctx);
    i = 0 as libc::c_int;
    while i < 64 as libc::c_int {
        buf[i as usize] = (buf[i as usize] as libc::c_int
            ^ *in_0.offset(i as isize) as libc::c_int)
            as libc::c_uchar;
        i += 1;
    }
    memcpy(
        out as *mut libc::c_void,
        (&raw mut buf as *mut libc::c_uchar).offset(8 as libc::c_int as isize)
            as *const libc::c_void,
        8 as size_t,
    );
    memcpy(
        out.offset(8 as libc::c_int as isize) as *mut libc::c_void,
        (&raw mut buf as *mut libc::c_uchar).offset(24 as libc::c_int as isize)
            as *const libc::c_void,
        8 as size_t,
    );
    memcpy(
        out.offset(16 as libc::c_int as isize) as *mut libc::c_void,
        (&raw mut buf as *mut libc::c_uchar).offset(32 as libc::c_int as isize)
            as *const libc::c_void,
        8 as size_t,
    );
    memcpy(
        out.offset(24 as libc::c_int as isize) as *mut libc::c_void,
        (&raw mut buf as *mut libc::c_uchar).offset(48 as libc::c_int as isize)
            as *const libc::c_void,
        8 as size_t,
    );
}
#[no_mangle]
pub unsafe extern "C" fn SPX_haraka256(
    mut out: *mut libc::c_uchar,
    mut in_0: *const libc::c_uchar,
    mut ctx: *const spx_ctx,
) {
    let mut q: [uint32_t; 8] = [0; 8];
    let mut tmp_q: uint32_t = 0;
    let mut i: libc::c_int = 0;
    let mut j: libc::c_int = 0;
    i = 0 as libc::c_int;
    while i < 4 as libc::c_int {
        q[(2 as libc::c_int * i) as usize] =
            br_dec32le(in_0.offset((4 as libc::c_int * i) as isize));
        q[(2 as libc::c_int * i + 1 as libc::c_int) as usize] = br_dec32le(
            in_0.offset((4 as libc::c_int * i) as isize)
                .offset(16 as libc::c_int as isize),
        );
        i += 1;
    }
    br_aes_ct_ortho(&raw mut q as *mut uint32_t);
    i = 0 as libc::c_int;
    while i < 5 as libc::c_int {
        j = 0 as libc::c_int;
        while j < 2 as libc::c_int {
            br_aes_ct_bitslice_Sbox(&raw mut q as *mut uint32_t);
            shift_rows32(&raw mut q as *mut uint32_t);
            mix_columns32(&raw mut q as *mut uint32_t);
            add_round_key32(
                &raw mut q as *mut uint32_t,
                &raw const *(&raw const (*ctx).tweaked256_rc32 as *const [uint32_t; 8])
                    .offset((2 as libc::c_int * i + j) as isize)
                    as *const uint32_t,
            );
            j += 1;
        }
        j = 0 as libc::c_int;
        while j < 8 as libc::c_int {
            tmp_q = q[j as usize];
            q[j as usize] = tmp_q & 0x81818181 as uint32_t
                | (tmp_q & 0x2020202 as uint32_t) << 1 as libc::c_int
                | (tmp_q & 0x4040404 as uint32_t) << 2 as libc::c_int
                | (tmp_q & 0x8080808 as uint32_t) << 3 as libc::c_int
                | (tmp_q & 0x10101010 as uint32_t) >> 3 as libc::c_int
                | (tmp_q & 0x20202020 as uint32_t) >> 2 as libc::c_int
                | (tmp_q & 0x40404040 as uint32_t) >> 1 as libc::c_int;
            j += 1;
        }
        i += 1;
    }
    br_aes_ct_ortho(&raw mut q as *mut uint32_t);
    i = 0 as libc::c_int;
    while i < 4 as libc::c_int {
        br_enc32le(
            out.offset((4 as libc::c_int * i) as isize),
            q[(2 as libc::c_int * i) as usize],
        );
        br_enc32le(
            out.offset((4 as libc::c_int * i) as isize)
                .offset(16 as libc::c_int as isize),
            q[(2 as libc::c_int * i + 1 as libc::c_int) as usize],
        );
        i += 1;
    }
    i = 0 as libc::c_int;
    while i < 32 as libc::c_int {
        let ref mut fresh21 = *out.offset(i as isize);
        *fresh21 = (*fresh21 as libc::c_int ^ *in_0.offset(i as isize) as libc::c_int)
            as libc::c_uchar;
        i += 1;
    }
}
