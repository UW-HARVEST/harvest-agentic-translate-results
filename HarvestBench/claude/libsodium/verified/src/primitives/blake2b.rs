//! Translated from crypto_generichash/blake2b/ref/{blake2b-ref.c, blake2b-compress-ref.c}
use crate::primitives::cutil::*;
use core::ffi::c_void;

pub const BLAKE2B_BLOCKBYTES: usize = 128;
pub const BLAKE2B_OUTBYTES: usize = 64;
pub const BLAKE2B_KEYBYTES: usize = 64;
pub const BLAKE2B_SALTBYTES: usize = 16;
pub const BLAKE2B_PERSONALBYTES: usize = 16;

#[repr(C, packed)]
pub struct blake2b_param {
    pub digest_length: u8,
    pub key_length: u8,
    pub fanout: u8,
    pub depth: u8,
    pub leaf_length: [u8; 4],
    pub node_offset: [u8; 8],
    pub node_depth: u8,
    pub inner_length: u8,
    pub reserved: [u8; 14],
    pub salt: [u8; BLAKE2B_SALTBYTES],
    pub personal: [u8; BLAKE2B_PERSONALBYTES],
}

#[repr(C, packed)]
pub struct blake2b_state {
    pub h: [u64; 8],
    pub t: [u64; 2],
    pub f: [u64; 2],
    pub buf: [u8; 2 * 128],
    pub buflen: usize,
    pub last_node: u8,
}

static BLAKE2B_IV: [u64; 8] = [
    0x6a09e667f3bcc908, 0xbb67ae8584caa73b, 0x3c6ef372fe94f82b, 0xa54ff53a5f1d36f1,
    0x510e527fade682d1, 0x9b05688c2b3e6c1f, 0x1f83d9abfb41bd6b, 0x5be0cd19137e2179,
];

static BLAKE2B_SIGMA: [[u8; 16]; 12] = [
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
];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_blake2b_compress_ref(
    s: *mut blake2b_state,
    block: *const u8,
) -> i32 {
    let s = &mut *s;
    let mut m = [0u64; 16];
    let mut v = [0u64; 16];
    for i in 0..16 {
        m[i] = load64_le(block.add(i * 8));
    }
    for i in 0..8 {
        v[i] = s.h[i];
    }
    v[8] = BLAKE2B_IV[0];
    v[9] = BLAKE2B_IV[1];
    v[10] = BLAKE2B_IV[2];
    v[11] = BLAKE2B_IV[3];
    v[12] = s.t[0] ^ BLAKE2B_IV[4];
    v[13] = s.t[1] ^ BLAKE2B_IV[5];
    v[14] = s.f[0] ^ BLAKE2B_IV[6];
    v[15] = s.f[1] ^ BLAKE2B_IV[7];

    macro_rules! g {
        ($r:expr,$i:expr,$a:expr,$b:expr,$c:expr,$d:expr) => {{
            v[$a] = v[$a]
                .wrapping_add(v[$b])
                .wrapping_add(m[BLAKE2B_SIGMA[$r][2 * $i + 0] as usize]);
            v[$d] = rotr64(v[$d] ^ v[$a], 32);
            v[$c] = v[$c].wrapping_add(v[$d]);
            v[$b] = rotr64(v[$b] ^ v[$c], 24);
            v[$a] = v[$a]
                .wrapping_add(v[$b])
                .wrapping_add(m[BLAKE2B_SIGMA[$r][2 * $i + 1] as usize]);
            v[$d] = rotr64(v[$d] ^ v[$a], 16);
            v[$c] = v[$c].wrapping_add(v[$d]);
            v[$b] = rotr64(v[$b] ^ v[$c], 63);
        }};
    }
    macro_rules! round {
        ($r:expr) => {{
            g!($r, 0, 0, 4, 8, 12);
            g!($r, 1, 1, 5, 9, 13);
            g!($r, 2, 2, 6, 10, 14);
            g!($r, 3, 3, 7, 11, 15);
            g!($r, 4, 0, 5, 10, 15);
            g!($r, 5, 1, 6, 11, 12);
            g!($r, 6, 2, 7, 8, 13);
            g!($r, 7, 3, 4, 9, 14);
        }};
    }
    round!(0);
    round!(1);
    round!(2);
    round!(3);
    round!(4);
    round!(5);
    round!(6);
    round!(7);
    round!(8);
    round!(9);
    round!(10);
    round!(11);

    for i in 0..8 {
        s.h[i] = s.h[i] ^ v[i] ^ v[i + 8];
    }
    0
}

#[inline(always)]
unsafe fn blake2b_compress(s: *mut blake2b_state, block: *const u8) -> i32 {
    _sodium_blake2b_compress_ref(s, block)
}

unsafe fn blake2b_set_lastnode(s: &mut blake2b_state) {
    s.f[1] = u64::MAX;
}
unsafe fn blake2b_is_lastblock(s: &blake2b_state) -> bool {
    s.f[0] != 0
}
unsafe fn blake2b_set_lastblock(s: &mut blake2b_state) {
    if s.last_node != 0 {
        blake2b_set_lastnode(s);
    }
    s.f[0] = u64::MAX;
}
unsafe fn blake2b_increment_counter(s: &mut blake2b_state, inc: u64) {
    s.t[0] = s.t[0].wrapping_add(inc);
    s.t[1] = s.t[1].wrapping_add((s.t[0] < inc) as u64);
}

unsafe fn blake2b_init0(s: &mut blake2b_state) {
    for i in 0..8 {
        s.h[i] = BLAKE2B_IV[i];
    }
    // zero everything between .t and .last_node inclusive
    s.t = [0; 2];
    s.f = [0; 2];
    s.buf = [0; 256];
    s.buflen = 0;
    s.last_node = 0;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_blake2b_init_param(
    s: *mut blake2b_state,
    p: *const blake2b_param,
) -> i32 {
    let st = &mut *s;
    blake2b_init0(st);
    let pbytes = p as *const u8;
    for i in 0..8 {
        st.h[i] ^= load64_le(pbytes.add(8 * i));
    }
    0
}

unsafe fn fill_param_base(p: *mut blake2b_param, outlen: u8, keylen: u8) {
    let p = &mut *p;
    p.digest_length = outlen;
    p.key_length = keylen;
    p.fanout = 1;
    p.depth = 1;
    store32_le(p.leaf_length.as_mut_ptr(), 0);
    store64_le(p.node_offset.as_mut_ptr(), 0);
    p.node_depth = 0;
    p.inner_length = 0;
    p.reserved = [0; 14];
}

unsafe fn new_param() -> blake2b_param {
    blake2b_param {
        digest_length: 0,
        key_length: 0,
        fanout: 0,
        depth: 0,
        leaf_length: [0; 4],
        node_offset: [0; 8],
        node_depth: 0,
        inner_length: 0,
        reserved: [0; 14],
        salt: [0; 16],
        personal: [0; 16],
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_blake2b_init(s: *mut blake2b_state, outlen: u8) -> i32 {
    let mut p = new_param();
    if outlen == 0 || outlen as usize > BLAKE2B_OUTBYTES {
        sodium_misuse();
    }
    fill_param_base(&mut p, outlen, 0);
    p.salt = [0; 16];
    p.personal = [0; 16];
    _sodium_blake2b_init_param(s, &p)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_blake2b_init_salt_personal(
    s: *mut blake2b_state,
    outlen: u8,
    salt: *const c_void,
    personal: *const c_void,
) -> i32 {
    let mut p = new_param();
    if outlen == 0 || outlen as usize > BLAKE2B_OUTBYTES {
        sodium_misuse();
    }
    fill_param_base(&mut p, outlen, 0);
    if !salt.is_null() {
        core::ptr::copy_nonoverlapping(salt as *const u8, p.salt.as_mut_ptr(), BLAKE2B_SALTBYTES);
    } else {
        p.salt = [0; 16];
    }
    if !personal.is_null() {
        core::ptr::copy_nonoverlapping(
            personal as *const u8,
            p.personal.as_mut_ptr(),
            BLAKE2B_PERSONALBYTES,
        );
    } else {
        p.personal = [0; 16];
    }
    _sodium_blake2b_init_param(s, &p)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_blake2b_init_key(
    s: *mut blake2b_state,
    outlen: u8,
    key: *const c_void,
    keylen: u8,
) -> i32 {
    let mut p = new_param();
    if outlen == 0 || outlen as usize > BLAKE2B_OUTBYTES {
        sodium_misuse();
    }
    if key.is_null() || keylen == 0 || keylen as usize > BLAKE2B_KEYBYTES {
        sodium_misuse();
    }
    fill_param_base(&mut p, outlen, keylen);
    p.salt = [0; 16];
    p.personal = [0; 16];
    if _sodium_blake2b_init_param(s, &p) < 0 {
        sodium_misuse();
    }
    {
        let mut block = [0u8; BLAKE2B_BLOCKBYTES];
        core::ptr::copy_nonoverlapping(key as *const u8, block.as_mut_ptr(), keylen as usize);
        _sodium_blake2b_update(s, block.as_ptr(), BLAKE2B_BLOCKBYTES as u64);
        sodium_memzero(block.as_mut_ptr() as *mut c_void, BLAKE2B_BLOCKBYTES);
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_blake2b_init_key_salt_personal(
    s: *mut blake2b_state,
    outlen: u8,
    key: *const c_void,
    keylen: u8,
    salt: *const c_void,
    personal: *const c_void,
) -> i32 {
    let mut p = new_param();
    if outlen == 0 || outlen as usize > BLAKE2B_OUTBYTES {
        sodium_misuse();
    }
    if key.is_null() || keylen == 0 || keylen as usize > BLAKE2B_KEYBYTES {
        sodium_misuse();
    }
    fill_param_base(&mut p, outlen, keylen);
    if !salt.is_null() {
        core::ptr::copy_nonoverlapping(salt as *const u8, p.salt.as_mut_ptr(), BLAKE2B_SALTBYTES);
    } else {
        p.salt = [0; 16];
    }
    if !personal.is_null() {
        core::ptr::copy_nonoverlapping(
            personal as *const u8,
            p.personal.as_mut_ptr(),
            BLAKE2B_PERSONALBYTES,
        );
    } else {
        p.personal = [0; 16];
    }
    if _sodium_blake2b_init_param(s, &p) < 0 {
        sodium_misuse();
    }
    {
        let mut block = [0u8; BLAKE2B_BLOCKBYTES];
        core::ptr::copy_nonoverlapping(key as *const u8, block.as_mut_ptr(), keylen as usize);
        _sodium_blake2b_update(s, block.as_ptr(), BLAKE2B_BLOCKBYTES as u64);
        sodium_memzero(block.as_mut_ptr() as *mut c_void, BLAKE2B_BLOCKBYTES);
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_blake2b_update(
    s: *mut blake2b_state,
    mut input: *const u8,
    mut inlen: u64,
) -> i32 {
    let bufp = (*s).buf.as_mut_ptr();
    while inlen > 0 {
        let left = (*s).buflen;
        let fill = 2 * BLAKE2B_BLOCKBYTES - left;
        if inlen as usize > fill {
            core::ptr::copy_nonoverlapping(input, bufp.add(left), fill);
            (*s).buflen += fill;
            increment_counter_ptr(s, BLAKE2B_BLOCKBYTES as u64);
            blake2b_compress(s, bufp);
            core::ptr::copy(bufp.add(BLAKE2B_BLOCKBYTES), bufp, BLAKE2B_BLOCKBYTES);
            (*s).buflen -= BLAKE2B_BLOCKBYTES;
            input = input.add(fill);
            inlen -= fill as u64;
        } else {
            core::ptr::copy_nonoverlapping(input, bufp.add(left), inlen as usize);
            (*s).buflen += inlen as usize;
            input = input.add(inlen as usize);
            inlen -= inlen;
        }
    }
    0
}

#[inline(always)]
unsafe fn increment_counter_ptr(s: *mut blake2b_state, inc: u64) {
    let t0 = (*s).t[0].wrapping_add(inc);
    (*s).t[1] = (*s).t[1].wrapping_add((t0 < inc) as u64);
    (*s).t[0] = t0;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_blake2b_final(
    s: *mut blake2b_state,
    out: *mut u8,
    outlen: u8,
) -> i32 {
    let mut buffer = [0u8; BLAKE2B_OUTBYTES];
    if outlen == 0 || outlen as usize > BLAKE2B_OUTBYTES {
        sodium_misuse();
    }
    if blake2b_is_lastblock(&*s) {
        return -1;
    }
    let bufp = (*s).buf.as_mut_ptr();
    if (*s).buflen > BLAKE2B_BLOCKBYTES {
        increment_counter_ptr(s, BLAKE2B_BLOCKBYTES as u64);
        blake2b_compress(s, bufp);
        (*s).buflen -= BLAKE2B_BLOCKBYTES;
        let buflen = (*s).buflen;
        core::ptr::copy(bufp.add(BLAKE2B_BLOCKBYTES), bufp, buflen);
    }
    let buflen = (*s).buflen;
    increment_counter_ptr(s, buflen as u64);
    {
        let st = &mut *s;
        blake2b_set_lastblock(st);
    }
    core::ptr::write_bytes(bufp.add(buflen), 0, 2 * BLAKE2B_BLOCKBYTES - buflen);
    blake2b_compress(s, bufp);

    for i in 0..8 {
        store64_le(buffer.as_mut_ptr().add(8 * i), (*s).h[i]);
    }
    core::ptr::copy_nonoverlapping(buffer.as_ptr(), out, outlen as usize);
    let hptr = core::ptr::addr_of_mut!((*s).h) as *mut c_void;
    sodium_memzero(hptr, core::mem::size_of::<[u64; 8]>());
    let bufptr = core::ptr::addr_of_mut!((*s).buf) as *mut c_void;
    sodium_memzero(bufptr, core::mem::size_of::<[u8; 256]>());
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_blake2b(
    out: *mut u8,
    input: *const c_void,
    key: *const c_void,
    outlen: u8,
    inlen: u64,
    keylen: u8,
) -> i32 {
    let mut s = new_state();
    if input.is_null() && inlen > 0 {
        sodium_misuse();
    }
    if out.is_null() {
        sodium_misuse();
    }
    if outlen == 0 || outlen as usize > BLAKE2B_OUTBYTES {
        sodium_misuse();
    }
    if key.is_null() && keylen > 0 {
        sodium_misuse();
    }
    if keylen as usize > BLAKE2B_KEYBYTES {
        sodium_misuse();
    }
    if keylen > 0 {
        if _sodium_blake2b_init_key(&mut s, outlen, key, keylen) < 0 {
            sodium_misuse();
        }
    } else if _sodium_blake2b_init(&mut s, outlen) < 0 {
        sodium_misuse();
    }
    _sodium_blake2b_update(&mut s, input as *const u8, inlen);
    _sodium_blake2b_final(&mut s, out, outlen);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_blake2b_salt_personal(
    out: *mut u8,
    input: *const c_void,
    key: *const c_void,
    outlen: u8,
    inlen: u64,
    keylen: u8,
    salt: *const c_void,
    personal: *const c_void,
) -> i32 {
    let mut s = new_state();
    if input.is_null() && inlen > 0 {
        sodium_misuse();
    }
    if out.is_null() {
        sodium_misuse();
    }
    if outlen == 0 || outlen as usize > BLAKE2B_OUTBYTES {
        sodium_misuse();
    }
    if key.is_null() && keylen > 0 {
        sodium_misuse();
    }
    if keylen as usize > BLAKE2B_KEYBYTES {
        sodium_misuse();
    }
    if keylen > 0 {
        if _sodium_blake2b_init_key_salt_personal(&mut s, outlen, key, keylen, salt, personal) < 0 {
            sodium_misuse();
        }
    } else if _sodium_blake2b_init_salt_personal(&mut s, outlen, salt, personal) < 0 {
        sodium_misuse();
    }
    _sodium_blake2b_update(&mut s, input as *const u8, inlen);
    _sodium_blake2b_final(&mut s, out, outlen);
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn _sodium_blake2b_pick_best_implementation() -> i32 {
    0
}

pub(crate) unsafe fn new_state() -> blake2b_state {
    blake2b_state {
        h: [0; 8],
        t: [0; 2],
        f: [0; 2],
        buf: [0; 256],
        buflen: 0,
        last_node: 0,
    }
}
