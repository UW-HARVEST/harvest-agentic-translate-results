use crate::utils::u32_to_bytes;

pub const SPX_BLAKE512_OUTPUT_BYTES: usize = 64;

#[repr(C)]
pub struct Blakestate512 {
    pub h: [u64; 8],
    pub s: [u64; 4],
    pub t: [u64; 2],
    pub buflen: i32,
    pub nullt: i32,
    pub buf: [u8; 128],
}

fn u8to32(p: *const u8) -> u32 {
    unsafe {
        ((*p.add(0) as u32) << 24)
            | ((*p.add(1) as u32) << 16)
            | ((*p.add(2) as u32) << 8)
            | (*p.add(3) as u32)
    }
}

fn u8to64(p: *const u8) -> u64 {
    ((u8to32(p) as u64) << 32) | (u8to32(unsafe { p.add(4) }) as u64)
}

fn u64to8(p: *mut u8, v: u64) {
    unsafe {
        let hi = (v >> 32) as u32;
        let lo = v as u32;
        *p.add(0) = (hi >> 24) as u8;
        *p.add(1) = (hi >> 16) as u8;
        *p.add(2) = (hi >> 8) as u8;
        *p.add(3) = hi as u8;
        *p.add(4) = (lo >> 24) as u8;
        *p.add(5) = (lo >> 16) as u8;
        *p.add(6) = (lo >> 8) as u8;
        *p.add(7) = lo as u8;
    }
}

static CST: [u64; 16] = [
    0x243F6A8885A308D3, 0x13198A2E03707344, 0xA4093822299F31D0, 0x082EFA98EC4E6C89,
    0x452821E638D01377, 0xBE5466CF34E90C6C, 0xC0AC29B7C97C50DD, 0x3F84D5B5B5470917,
    0x9216D5D98979FB1B, 0xD1310BA698DFB5AC, 0x2FFD72DBD01ADFB7, 0xB8E1AFED6A267E96,
    0xBA7C9045F12C7F99, 0x24A19947B3916CF7, 0x0801F2E2858EFC16, 0x636920D871574E69,
];

static PADDING: [u8; 129] = [
    0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0,
];

#[inline(always)]
fn blake512_rot(x: u64, n: u32) -> u64 {
    (x << (64 - n)) | (x >> n)
}

// BLAKE-512 sigma permutation (16 rounds)
static SIGMA: [[usize; 16]; 16] = [
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
    [14, 10, 4, 8, 9, 15, 13, 6, 1, 12, 0, 2, 11, 7, 5, 3],
    [11, 8, 12, 0, 5, 2, 15, 13, 10, 14, 3, 6, 7, 1, 9, 4],
    [7, 9, 3, 1, 13, 12, 11, 14, 2, 6, 5, 10, 4, 0, 15, 8],
    [9, 0, 5, 7, 2, 4, 10, 15, 14, 1, 11, 12, 6, 8, 3, 13],
    [2, 12, 6, 10, 0, 11, 8, 3, 4, 13, 7, 5, 15, 14, 1, 9],
    [12, 5, 1, 15, 14, 13, 4, 10, 0, 7, 6, 3, 9, 2, 8, 11],
    [13, 11, 7, 14, 12, 1, 3, 9, 5, 0, 15, 4, 8, 6, 2, 10],
    [6, 15, 14, 9, 11, 3, 0, 8, 12, 2, 13, 7, 1, 4, 10, 5],
    [10, 2, 8, 4, 7, 6, 1, 5, 15, 11, 9, 14, 3, 12, 13, 0],
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
    [14, 10, 4, 8, 9, 15, 13, 6, 1, 12, 0, 2, 11, 7, 5, 3],
    [11, 8, 12, 0, 5, 2, 15, 13, 10, 14, 3, 6, 7, 1, 9, 4],
    [7, 9, 3, 1, 13, 12, 11, 14, 2, 6, 5, 10, 4, 0, 15, 8],
    [9, 0, 5, 7, 2, 4, 10, 15, 14, 1, 11, 12, 6, 8, 3, 13],
    [2, 12, 6, 10, 0, 11, 8, 3, 4, 13, 7, 5, 15, 14, 1, 9],
];

pub unsafe fn blake512_compress(s: &mut Blakestate512, block: *const u8) {
    let m: [u64; 16] = [
        u8to64(block), u8to64(block.add(8)), u8to64(block.add(16)), u8to64(block.add(24)),
        u8to64(block.add(32)), u8to64(block.add(40)), u8to64(block.add(48)), u8to64(block.add(56)),
        u8to64(block.add(64)), u8to64(block.add(72)), u8to64(block.add(80)), u8to64(block.add(88)),
        u8to64(block.add(96)), u8to64(block.add(104)), u8to64(block.add(112)), u8to64(block.add(120)),
    ];

    let mut v: [u64; 16] = [
        s.h[0], s.h[1], s.h[2], s.h[3],
        s.h[4], s.h[5], s.h[6], s.h[7],
        s.s[0] ^ 0x243F6A8885A308D3, s.s[1] ^ 0x13198A2E03707344,
        s.s[2] ^ 0xA4093822299F31D0, s.s[3] ^ 0x082EFA98EC4E6C89,
        0x452821E638D01377, 0xBE5466CF34E90C6C,
        0xC0AC29B7C97C50DD, 0x3F84D5B5B5470917,
    ];

    if s.nullt == 0 {
        v[12] ^= s.t[0];
        v[13] ^= s.t[0];
        v[14] ^= s.t[1];
        v[15] ^= s.t[1];
    }

    for r in 0..16 {
        let sg = &SIGMA[r];
        // Column step
        v[0] = v[0].wrapping_add(m[sg[0]] ^ CST[sg[1]]).wrapping_add(v[4]);
        v[12] ^= v[0]; v[12] = blake512_rot(v[12], 32);
        v[8] = v[8].wrapping_add(v[12]);
        v[4] ^= v[8]; v[4] = blake512_rot(v[4], 25);
        v[0] = v[0].wrapping_add(m[sg[1]] ^ CST[sg[0]]).wrapping_add(v[4]);
        v[12] ^= v[0]; v[12] = blake512_rot(v[12], 16);
        v[8] = v[8].wrapping_add(v[12]);
        v[4] ^= v[8]; v[4] = blake512_rot(v[4], 11);

        v[1] = v[1].wrapping_add(m[sg[2]] ^ CST[sg[3]]).wrapping_add(v[5]);
        v[13] ^= v[1]; v[13] = blake512_rot(v[13], 32);
        v[9] = v[9].wrapping_add(v[13]);
        v[5] ^= v[9]; v[5] = blake512_rot(v[5], 25);
        v[1] = v[1].wrapping_add(m[sg[3]] ^ CST[sg[2]]).wrapping_add(v[5]);
        v[13] ^= v[1]; v[13] = blake512_rot(v[13], 16);
        v[9] = v[9].wrapping_add(v[13]);
        v[5] ^= v[9]; v[5] = blake512_rot(v[5], 11);

        v[2] = v[2].wrapping_add(m[sg[4]] ^ CST[sg[5]]).wrapping_add(v[6]);
        v[14] ^= v[2]; v[14] = blake512_rot(v[14], 32);
        v[10] = v[10].wrapping_add(v[14]);
        v[6] ^= v[10]; v[6] = blake512_rot(v[6], 25);
        v[2] = v[2].wrapping_add(m[sg[5]] ^ CST[sg[4]]).wrapping_add(v[6]);
        v[14] ^= v[2]; v[14] = blake512_rot(v[14], 16);
        v[10] = v[10].wrapping_add(v[14]);
        v[6] ^= v[10]; v[6] = blake512_rot(v[6], 11);

        v[3] = v[3].wrapping_add(m[sg[6]] ^ CST[sg[7]]).wrapping_add(v[7]);
        v[15] ^= v[3]; v[15] = blake512_rot(v[15], 32);
        v[11] = v[11].wrapping_add(v[15]);
        v[7] ^= v[11]; v[7] = blake512_rot(v[7], 25);
        v[3] = v[3].wrapping_add(m[sg[7]] ^ CST[sg[6]]).wrapping_add(v[7]);
        v[15] ^= v[3]; v[15] = blake512_rot(v[15], 16);
        v[11] = v[11].wrapping_add(v[15]);
        v[7] ^= v[11]; v[7] = blake512_rot(v[7], 11);

        // Diagonal step
        v[0] = v[0].wrapping_add(m[sg[8]] ^ CST[sg[9]]).wrapping_add(v[5]);
        v[15] ^= v[0]; v[15] = blake512_rot(v[15], 32);
        v[10] = v[10].wrapping_add(v[15]);
        v[5] ^= v[10]; v[5] = blake512_rot(v[5], 25);
        v[0] = v[0].wrapping_add(m[sg[9]] ^ CST[sg[8]]).wrapping_add(v[5]);
        v[15] ^= v[0]; v[15] = blake512_rot(v[15], 16);
        v[10] = v[10].wrapping_add(v[15]);
        v[5] ^= v[10]; v[5] = blake512_rot(v[5], 11);

        v[1] = v[1].wrapping_add(m[sg[10]] ^ CST[sg[11]]).wrapping_add(v[6]);
        v[12] ^= v[1]; v[12] = blake512_rot(v[12], 32);
        v[11] = v[11].wrapping_add(v[12]);
        v[6] ^= v[11]; v[6] = blake512_rot(v[6], 25);
        v[1] = v[1].wrapping_add(m[sg[11]] ^ CST[sg[10]]).wrapping_add(v[6]);
        v[12] ^= v[1]; v[12] = blake512_rot(v[12], 16);
        v[11] = v[11].wrapping_add(v[12]);
        v[6] ^= v[11]; v[6] = blake512_rot(v[6], 11);

        v[2] = v[2].wrapping_add(m[sg[12]] ^ CST[sg[13]]).wrapping_add(v[7]);
        v[13] ^= v[2]; v[13] = blake512_rot(v[13], 32);
        v[8] = v[8].wrapping_add(v[13]);
        v[7] ^= v[8]; v[7] = blake512_rot(v[7], 25);
        v[2] = v[2].wrapping_add(m[sg[13]] ^ CST[sg[12]]).wrapping_add(v[7]);
        v[13] ^= v[2]; v[13] = blake512_rot(v[13], 16);
        v[8] = v[8].wrapping_add(v[13]);
        v[7] ^= v[8]; v[7] = blake512_rot(v[7], 11);

        v[3] = v[3].wrapping_add(m[sg[14]] ^ CST[sg[15]]).wrapping_add(v[4]);
        v[14] ^= v[3]; v[14] = blake512_rot(v[14], 32);
        v[9] = v[9].wrapping_add(v[14]);
        v[4] ^= v[9]; v[4] = blake512_rot(v[4], 25);
        v[3] = v[3].wrapping_add(m[sg[15]] ^ CST[sg[14]]).wrapping_add(v[4]);
        v[14] ^= v[3]; v[14] = blake512_rot(v[14], 16);
        v[9] = v[9].wrapping_add(v[14]);
        v[4] ^= v[9]; v[4] = blake512_rot(v[4], 11);
    }

    for i in 0..8 {
        v[i] ^= v[i + 8];
    }
    for i in 0..4 {
        v[i] ^= s.s[i];
        v[i + 4] ^= s.s[i];
    }
    for i in 0..8 {
        s.h[i] ^= v[i];
    }
}

pub fn blake512_init(s: &mut Blakestate512) {
    s.h[0] = 0x6A09E667F3BCC908;
    s.h[1] = 0xBB67AE8584CAA73B;
    s.h[2] = 0x3C6EF372FE94F82B;
    s.h[3] = 0xA54FF53A5F1D36F1;
    s.h[4] = 0x510E527FADE682D1;
    s.h[5] = 0x9B05688C2B3E6C1F;
    s.h[6] = 0x1F83D9ABFB41BD6B;
    s.h[7] = 0x5BE0CD19137E2179;
    s.t = [0; 2];
    s.buflen = 0;
    s.nullt = 0;
    s.s = [0; 4];
}

pub unsafe fn blake512_update(s: &mut Blakestate512, mut data: *const u8, mut datalen: u64) {
    let mut left = (s.buflen >> 3) as usize;
    let fill = 128 - left;

    if left != 0 && ((datalen >> 3) & 0x7F) as usize >= fill {
        core::ptr::copy_nonoverlapping(data, s.buf.as_mut_ptr().add(left), fill);
        s.t[0] = s.t[0].wrapping_add(1024);
        blake512_compress(s, s.buf.as_ptr());
        data = data.add(fill);
        datalen -= (fill as u64) << 3;
        left = 0;
    }

    while datalen >= 1024 {
        s.t[0] = s.t[0].wrapping_add(1024);
        blake512_compress(s, data);
        data = data.add(128);
        datalen -= 1024;
    }

    if datalen > 0 {
        core::ptr::copy_nonoverlapping(data, s.buf.as_mut_ptr().add(left), ((datalen >> 3) & 0x7F) as usize);
        s.buflen = ((left as u64) << 3).wrapping_add(datalen) as i32;
    } else {
        s.buflen = 0;
    }
}

pub unsafe fn blake512_final(s: &mut Blakestate512, digest: *mut u8) {
    let mut msglen = [0u8; 16];
    let zo: u8 = 0x01;
    let oo: u8 = 0x81;
    let lo = s.t[0].wrapping_add(s.buflen as u64);
    let mut hi = s.t[1];
    if lo < s.buflen as u64 { hi = hi.wrapping_add(1); }
    u64to8(msglen.as_mut_ptr(), hi);
    u64to8(msglen.as_mut_ptr().add(8), lo);

    if s.buflen == 888 {
        s.t[0] = s.t[0].wrapping_sub(8);
        blake512_update(s, &oo, 8);
    } else {
        if s.buflen < 888 {
            if s.buflen == 0 { s.nullt = 1; }
            s.t[0] = s.t[0].wrapping_sub((888 - s.buflen) as u64);
            blake512_update(s, PADDING.as_ptr(), (888 - s.buflen) as u64);
        } else {
            s.t[0] = s.t[0].wrapping_sub((1024 - s.buflen) as u64);
            blake512_update(s, PADDING.as_ptr(), (1024 - s.buflen) as u64);
            s.t[0] = s.t[0].wrapping_sub(888);
            blake512_update(s, PADDING.as_ptr().add(1), 888);
            s.nullt = 1;
        }
        blake512_update(s, &zo, 8);
        s.t[0] = s.t[0].wrapping_sub(8);
    }
    s.t[0] = s.t[0].wrapping_sub(128);
    blake512_update(s, msglen.as_ptr(), 128);

    for i in 0..8 {
        u64to8(digest.add(i * 8), s.h[i]);
    }
}

pub unsafe fn blake512(out: *mut u8, in_: *const u8, inlen: u64) -> i32 {
    let mut s = core::mem::MaybeUninit::<Blakestate512>::uninit();
    let s = s.assume_init_mut();
    blake512_init(s);
    blake512_update(s, in_, inlen.wrapping_mul(8));
    blake512_final(s, out);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_blake512_mgf1(
    mut out: *mut u8, outlen: u64,
    in_: *const u8, inlen: u64,
) {
    let inlen = inlen as usize;
    let outlen = outlen as usize;
    let mut inbuf = vec![0u8; inlen + 4];
    let mut outbuf = [0u8; SPX_BLAKE512_OUTPUT_BYTES];

    core::ptr::copy_nonoverlapping(in_, inbuf.as_mut_ptr(), inlen);

    let mut i: usize = 0;
    while (i + 1) * SPX_BLAKE512_OUTPUT_BYTES <= outlen {
        u32_to_bytes(inbuf.as_mut_ptr().add(inlen), i as u32);
        blake512(out, inbuf.as_ptr(), (inlen + 4) as u64);
        out = out.add(SPX_BLAKE512_OUTPUT_BYTES);
        i += 1;
    }
    if outlen > i * SPX_BLAKE512_OUTPUT_BYTES {
        u32_to_bytes(inbuf.as_mut_ptr().add(inlen), i as u32);
        blake512(outbuf.as_mut_ptr(), inbuf.as_ptr(), (inlen + 4) as u64);
        core::ptr::copy_nonoverlapping(outbuf.as_ptr(), out, outlen - i * SPX_BLAKE512_OUTPUT_BYTES);
    }
}
