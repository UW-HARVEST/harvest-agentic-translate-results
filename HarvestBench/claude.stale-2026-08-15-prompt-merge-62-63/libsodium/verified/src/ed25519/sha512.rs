//! SHA-512 streaming (crypto_hash_sha512_init/update/final), translated from
//! crypto_hash/sha512/cp/hash_sha512_cp.c. The one-shot crypto_hash_sha512 is
//! owned by P1; the streaming API symbols are assigned to P2.
use core::ffi::{c_int, c_void};

extern "C" {
    fn sodium_memzero(pnt: *mut c_void, len: usize);
}

#[repr(C)]
pub struct crypto_hash_sha512_state {
    pub state: [u64; 8],
    pub count: [u64; 2],
    pub buf: [u8; 128],
}

static KRND: [u64; 80] = [
    0x428a2f98d728ae22, 0x7137449123ef65cd, 0xb5c0fbcfec4d3b2f, 0xe9b5dba58189dbbc,
    0x3956c25bf348b538, 0x59f111f1b605d019, 0x923f82a4af194f9b, 0xab1c5ed5da6d8118,
    0xd807aa98a3030242, 0x12835b0145706fbe, 0x243185be4ee4b28c, 0x550c7dc3d5ffb4e2,
    0x72be5d74f27b896f, 0x80deb1fe3b1696b1, 0x9bdc06a725c71235, 0xc19bf174cf692694,
    0xe49b69c19ef14ad2, 0xefbe4786384f25e3, 0x0fc19dc68b8cd5b5, 0x240ca1cc77ac9c65,
    0x2de92c6f592b0275, 0x4a7484aa6ea6e483, 0x5cb0a9dcbd41fbd4, 0x76f988da831153b5,
    0x983e5152ee66dfab, 0xa831c66d2db43210, 0xb00327c898fb213f, 0xbf597fc7beef0ee4,
    0xc6e00bf33da88fc2, 0xd5a79147930aa725, 0x06ca6351e003826f, 0x142929670a0e6e70,
    0x27b70a8546d22ffc, 0x2e1b21385c26c926, 0x4d2c6dfc5ac42aed, 0x53380d139d95b3df,
    0x650a73548baf63de, 0x766a0abb3c77b2a8, 0x81c2c92e47edaee6, 0x92722c851482353b,
    0xa2bfe8a14cf10364, 0xa81a664bbc423001, 0xc24b8b70d0f89791, 0xc76c51a30654be30,
    0xd192e819d6ef5218, 0xd69906245565a910, 0xf40e35855771202a, 0x106aa07032bbd1b8,
    0x19a4c116b8d2d0c8, 0x1e376c085141ab53, 0x2748774cdf8eeb99, 0x34b0bcb5e19b48a8,
    0x391c0cb3c5c95a63, 0x4ed8aa4ae3418acb, 0x5b9cca4f7763e373, 0x682e6ff3d6b2b8a3,
    0x748f82ee5defb2fc, 0x78a5636f43172f60, 0x84c87814a1f0ab72, 0x8cc702081a6439ec,
    0x90befffa23631e28, 0xa4506cebde82bde9, 0xbef9a3f7b2c67915, 0xc67178f2e372532b,
    0xca273eceea26619c, 0xd186b8c721c0c207, 0xeada7dd6cde0eb1e, 0xf57d4f7fee6ed178,
    0x06f067aa72176fba, 0x0a637dc5a2c898a6, 0x113f9804bef90dae, 0x1b710b35131c471b,
    0x28db77f523047d84, 0x32caab7b40c72493, 0x3c9ebe0a15c9bebc, 0x431d67c49c100d4c,
    0x4cc5d4becb3e42b6, 0x597f299cfc657e2a, 0x5fcb6fab3ad6faec, 0x6c44198c4a475817,
];

#[inline(always)]
fn rotr64(x: u64, b: u32) -> u64 {
    x.rotate_right(b)
}
#[inline(always)]
fn ch(x: u64, y: u64, z: u64) -> u64 {
    (x & (y ^ z)) ^ z
}
#[inline(always)]
fn maj(x: u64, y: u64, z: u64) -> u64 {
    (x & (y | z)) | (y & z)
}
#[inline(always)]
fn big_s0(x: u64) -> u64 {
    rotr64(x, 28) ^ rotr64(x, 34) ^ rotr64(x, 39)
}
#[inline(always)]
fn big_s1(x: u64) -> u64 {
    rotr64(x, 14) ^ rotr64(x, 18) ^ rotr64(x, 41)
}
#[inline(always)]
fn sm0(x: u64) -> u64 {
    rotr64(x, 1) ^ rotr64(x, 8) ^ (x >> 7)
}
#[inline(always)]
fn sm1(x: u64) -> u64 {
    rotr64(x, 19) ^ rotr64(x, 61) ^ (x >> 6)
}

#[inline(always)]
fn load64_be(p: *const u8) -> u64 {
    unsafe {
        let mut r = 0u64;
        for i in 0..8 {
            r = (r << 8) | (*p.add(i) as u64);
        }
        r
    }
}
#[inline(always)]
fn store64_be(p: *mut u8, x: u64) {
    unsafe {
        for i in 0..8 {
            *p.add(i) = (x >> (56 - 8 * i)) as u8;
        }
    }
}

unsafe fn be64dec_vect(dst: &mut [u64], src: *const u8, len: usize) {
    for i in 0..(len / 8) {
        dst[i] = load64_be(src.add(i * 8));
    }
}
unsafe fn be64enc_vect(dst: *mut u8, src: &[u64], len: usize) {
    for i in 0..(len / 8) {
        store64_be(dst.add(i * 8), src[i]);
    }
}

fn sha512_transform(state: &mut [u64; 8], block: &[u8], w: &mut [u64; 80], s: &mut [u64; 8]) {
    unsafe {
        be64dec_vect(w, block.as_ptr(), 128);
    }
    s.copy_from_slice(state);

    macro_rules! rnd {
        ($a:expr,$b:expr,$c:expr,$d:expr,$e:expr,$f:expr,$g:expr,$h:expr,$k:expr) => {{
            s[$h] = s[$h]
                .wrapping_add(big_s1(s[$e]))
                .wrapping_add(ch(s[$e], s[$f], s[$g]))
                .wrapping_add($k);
            s[$d] = s[$d].wrapping_add(s[$h]);
            s[$h] = s[$h]
                .wrapping_add(big_s0(s[$a]))
                .wrapping_add(maj(s[$a], s[$b], s[$c]));
        }};
    }
    macro_rules! rndr {
        ($i:expr,$ii:expr) => {
            rnd!(
                (80 - $i) % 8,
                (81 - $i) % 8,
                (82 - $i) % 8,
                (83 - $i) % 8,
                (84 - $i) % 8,
                (85 - $i) % 8,
                (86 - $i) % 8,
                (87 - $i) % 8,
                w[$i + $ii].wrapping_add(KRND[$i + $ii])
            )
        };
    }
    macro_rules! msch {
        ($ii:expr,$i:expr) => {
            w[$i + $ii + 16] = sm1(w[$i + $ii + 14])
                .wrapping_add(w[$i + $ii + 9])
                .wrapping_add(sm0(w[$i + $ii + 1]))
                .wrapping_add(w[$i + $ii])
        };
    }

    let mut i = 0usize;
    loop {
        rndr!(0, i);
        rndr!(1, i);
        rndr!(2, i);
        rndr!(3, i);
        rndr!(4, i);
        rndr!(5, i);
        rndr!(6, i);
        rndr!(7, i);
        rndr!(8, i);
        rndr!(9, i);
        rndr!(10, i);
        rndr!(11, i);
        rndr!(12, i);
        rndr!(13, i);
        rndr!(14, i);
        rndr!(15, i);
        if i == 64 {
            break;
        }
        msch!(0, i);
        msch!(1, i);
        msch!(2, i);
        msch!(3, i);
        msch!(4, i);
        msch!(5, i);
        msch!(6, i);
        msch!(7, i);
        msch!(8, i);
        msch!(9, i);
        msch!(10, i);
        msch!(11, i);
        msch!(12, i);
        msch!(13, i);
        msch!(14, i);
        msch!(15, i);
        i += 16;
    }
    for j in 0..8 {
        state[j] = state[j].wrapping_add(s[j]);
    }
}

static PAD: [u8; 128] = {
    let mut a = [0u8; 128];
    a[0] = 0x80;
    a
};

unsafe fn sha512_pad(state: &mut crypto_hash_sha512_state) {
    let r = ((state.count[1] >> 3) & 0x7f) as usize;
    if r < 112 {
        for i in 0..(112 - r) {
            state.buf[r + i] = PAD[i];
        }
    } else {
        for i in 0..(128 - r) {
            state.buf[r + i] = PAD[i];
        }
        let mut w = [0u64; 80];
        let mut s = [0u64; 8];
        let buf = state.buf;
        sha512_transform(&mut state.state, &buf, &mut w, &mut s);
        for i in 0..112 {
            state.buf[i] = 0;
        }
    }
    let count = state.count;
    be64enc_vect(state.buf.as_mut_ptr().add(112), &count, 16);
    let mut w = [0u64; 80];
    let mut s = [0u64; 8];
    let buf = state.buf;
    sha512_transform(&mut state.state, &buf, &mut w, &mut s);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_hash_sha512_init(state: *mut crypto_hash_sha512_state) -> c_int {
    static INIT: [u64; 8] = [
        0x6a09e667f3bcc908, 0xbb67ae8584caa73b, 0x3c6ef372fe94f82b, 0xa54ff53a5f1d36f1,
        0x510e527fade682d1, 0x9b05688c2b3e6c1f, 0x1f83d9abfb41bd6b, 0x5be0cd19137e2179,
    ];
    let state = &mut *state;
    state.count[0] = 0;
    state.count[1] = 0;
    state.state = INIT;
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_hash_sha512_update(
    state: *mut crypto_hash_sha512_state,
    mut input: *const u8,
    mut inlen: u64,
) -> c_int {
    let state = &mut *state;
    if inlen == 0 {
        return 0;
    }
    let r = ((state.count[1] >> 3) & 0x7f) as u64;
    let bitlen1 = (inlen as u64) << 3;
    let bitlen0 = (inlen as u64) >> 61;
    let newc1 = state.count[1].wrapping_add(bitlen1);
    if newc1 < bitlen1 {
        state.count[0] = state.count[0].wrapping_add(1);
    }
    state.count[1] = newc1;
    state.count[0] = state.count[0].wrapping_add(bitlen0);
    if inlen < 128 - r {
        for i in 0..inlen {
            state.buf[(r + i) as usize] = *input.add(i as usize);
        }
        return 0;
    }
    for i in 0..(128 - r) {
        state.buf[(r + i) as usize] = *input.add(i as usize);
    }
    {
        let mut w = [0u64; 80];
        let mut s = [0u64; 8];
        let buf = state.buf;
        sha512_transform(&mut state.state, &buf, &mut w, &mut s);
    }
    input = input.add((128 - r) as usize);
    inlen -= 128 - r;
    while inlen >= 128 {
        let block = core::slice::from_raw_parts(input, 128);
        let mut w = [0u64; 80];
        let mut s = [0u64; 8];
        sha512_transform(&mut state.state, block, &mut w, &mut s);
        input = input.add(128);
        inlen -= 128;
    }
    inlen &= 127;
    for i in 0..inlen {
        state.buf[i as usize] = *input.add(i as usize);
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_hash_sha512_final(
    state: *mut crypto_hash_sha512_state,
    out: *mut u8,
) -> c_int {
    let st = &mut *state;
    sha512_pad(st);
    be64enc_vect(out, &st.state, 64);
    sodium_memzero(
        state as *mut c_void,
        core::mem::size_of::<crypto_hash_sha512_state>(),
    );
    0
}
