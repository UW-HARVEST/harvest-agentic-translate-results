//! Translation of `app/src/rng.c` (the deterministic AES-256-CTR-DRBG used by
//! the KAT driver). The OpenSSL AES-256-ECB call is replaced by a self-contained
//! pure-Rust AES-256 block encryption (byte-identical to AES-256-ECB).

pub const RNG_SUCCESS: i32 = 0;
pub const RNG_BAD_MAXLEN: i32 = -1;
pub const RNG_BAD_OUTBUF: i32 = -2;
pub const RNG_BAD_REQ_LEN: i32 = -3;

// ---------------- AES-256 (single block, ECB) ----------------

#[rustfmt::skip]
const SBOX: [u8; 256] = [
    0x63,0x7c,0x77,0x7b,0xf2,0x6b,0x6f,0xc5,0x30,0x01,0x67,0x2b,0xfe,0xd7,0xab,0x76,
    0xca,0x82,0xc9,0x7d,0xfa,0x59,0x47,0xf0,0xad,0xd4,0xa2,0xaf,0x9c,0xa4,0x72,0xc0,
    0xb7,0xfd,0x93,0x26,0x36,0x3f,0xf7,0xcc,0x34,0xa5,0xe5,0xf1,0x71,0xd8,0x31,0x15,
    0x04,0xc7,0x23,0xc3,0x18,0x96,0x05,0x9a,0x07,0x12,0x80,0xe2,0xeb,0x27,0xb2,0x75,
    0x09,0x83,0x2c,0x1a,0x1b,0x6e,0x5a,0xa0,0x52,0x3b,0xd6,0xb3,0x29,0xe3,0x2f,0x84,
    0x53,0xd1,0x00,0xed,0x20,0xfc,0xb1,0x5b,0x6a,0xcb,0xbe,0x39,0x4a,0x4c,0x58,0xcf,
    0xd0,0xef,0xaa,0xfb,0x43,0x4d,0x33,0x85,0x45,0xf9,0x02,0x7f,0x50,0x3c,0x9f,0xa8,
    0x51,0xa3,0x40,0x8f,0x92,0x9d,0x38,0xf5,0xbc,0xb6,0xda,0x21,0x10,0xff,0xf3,0xd2,
    0xcd,0x0c,0x13,0xec,0x5f,0x97,0x44,0x17,0xc4,0xa7,0x7e,0x3d,0x64,0x5d,0x19,0x73,
    0x60,0x81,0x4f,0xdc,0x22,0x2a,0x90,0x88,0x46,0xee,0xb8,0x14,0xde,0x5e,0x0b,0xdb,
    0xe0,0x32,0x3a,0x0a,0x49,0x06,0x24,0x5c,0xc2,0xd3,0xac,0x62,0x91,0x95,0xe4,0x79,
    0xe7,0xc8,0x37,0x6d,0x8d,0xd5,0x4e,0xa9,0x6c,0x56,0xf4,0xea,0x65,0x7a,0xae,0x08,
    0xba,0x78,0x25,0x2e,0x1c,0xa6,0xb4,0xc6,0xe8,0xdd,0x74,0x1f,0x4b,0xbd,0x8b,0x8a,
    0x70,0x3e,0xb5,0x66,0x48,0x03,0xf6,0x0e,0x61,0x35,0x57,0xb9,0x86,0xc1,0x1d,0x9e,
    0xe1,0xf8,0x98,0x11,0x69,0xd9,0x8e,0x94,0x9b,0x1e,0x87,0xe9,0xce,0x55,0x28,0xdf,
    0x8c,0xa1,0x89,0x0d,0xbf,0xe6,0x42,0x68,0x41,0x99,0x2d,0x0f,0xb0,0x54,0xbb,0x16,
];

const RCON: [u8; 8] = [0x00, 0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40];

fn key_expansion(key: &[u8; 32]) -> [u8; 240] {
    let mut w = [0u8; 240];
    w[..32].copy_from_slice(key);
    let nk = 8usize;
    let total_words = 60usize;
    for i in nk..total_words {
        let mut temp = [w[4 * (i - 1)], w[4 * (i - 1) + 1], w[4 * (i - 1) + 2], w[4 * (i - 1) + 3]];
        if i % nk == 0 {
            // RotWord
            let t = temp[0];
            temp[0] = temp[1];
            temp[1] = temp[2];
            temp[2] = temp[3];
            temp[3] = t;
            // SubWord
            for b in temp.iter_mut() {
                *b = SBOX[*b as usize];
            }
            temp[0] ^= RCON[i / nk];
        } else if i % nk == 4 {
            for b in temp.iter_mut() {
                *b = SBOX[*b as usize];
            }
        }
        for j in 0..4 {
            w[4 * i + j] = w[4 * (i - nk) + j] ^ temp[j];
        }
    }
    w
}

fn xtime(x: u8) -> u8 {
    let hi = (x >> 7) & 1;
    (x << 1) ^ (0x1b * hi)
}

fn aes256_encrypt_block(rk: &[u8; 240], input: &[u8; 16]) -> [u8; 16] {
    let mut state = *input;

    // Initial round key addition (round 0).
    for i in 0..16 {
        state[i] ^= rk[i];
    }

    for round in 1..14 {
        // SubBytes
        for b in state.iter_mut() {
            *b = SBOX[*b as usize];
        }
        // ShiftRows (column-major index r + 4c)
        let old = state;
        for c in 0..4 {
            for r in 0..4 {
                state[r + 4 * c] = old[r + 4 * ((c + r) % 4)];
            }
        }
        // MixColumns
        for c in 0..4 {
            let s0 = state[4 * c];
            let s1 = state[4 * c + 1];
            let s2 = state[4 * c + 2];
            let s3 = state[4 * c + 3];
            let t = s0 ^ s1 ^ s2 ^ s3;
            state[4 * c] ^= t ^ xtime(s0 ^ s1);
            state[4 * c + 1] ^= t ^ xtime(s1 ^ s2);
            state[4 * c + 2] ^= t ^ xtime(s2 ^ s3);
            state[4 * c + 3] ^= t ^ xtime(s3 ^ s0);
        }
        // AddRoundKey
        for i in 0..16 {
            state[i] ^= rk[16 * round + i];
        }
    }

    // Final round (no MixColumns).
    for b in state.iter_mut() {
        *b = SBOX[*b as usize];
    }
    let old = state;
    for c in 0..4 {
        for r in 0..4 {
            state[r + 4 * c] = old[r + 4 * ((c + r) % 4)];
        }
    }
    for i in 0..16 {
        state[i] ^= rk[16 * 14 + i];
    }

    state
}

/// AES-256-ECB single-block encryption: buffer = AES_256(key, ctr).
fn aes256_ecb(key: &[u8; 32], ctr: &[u8; 16], buffer: &mut [u8; 16]) {
    let rk = key_expansion(key);
    *buffer = aes256_encrypt_block(&rk, ctr);
}

// ---------------- CTR-DRBG ----------------

/// C: `typedef struct { unsigned char Key[32]; unsigned char V[16]; int reseed_counter; }
/// AES256_CTR_DRBG_struct;`
#[allow(non_camel_case_types)]
#[allow(non_snake_case)]
#[repr(C)]
pub struct AES256_CTR_DRBG_struct {
    pub Key: [u8; 32],
    pub V: [u8; 16],
    pub reseed_counter: core::ffi::c_int,
}

const _: () = assert!(core::mem::size_of::<AES256_CTR_DRBG_struct>() == 52);
const _: () = assert!(core::mem::align_of::<AES256_CTR_DRBG_struct>() == 4);

/// C: `AES256_CTR_DRBG_struct DRBG_ctx;` -- the single, exported, zero-initialised
/// global DRBG state. This *is* the state; the Rust helpers below operate on it
/// directly (C's DRBG is not thread safe either).
#[allow(non_upper_case_globals)]
#[no_mangle]
pub static mut DRBG_ctx: AES256_CTR_DRBG_struct = AES256_CTR_DRBG_struct {
    Key: [0u8; 32],
    V: [0u8; 16],
    reseed_counter: 0,
};

fn aes256_ctr_drbg_update(provided_data: Option<&[u8; 48]>, key: &mut [u8; 32], v: &mut [u8; 16]) {
    let mut temp = [0u8; 48];
    for i in 0..3 {
        // increment V (128-bit, big-endian)
        for j in (0..16).rev() {
            if v[j] == 0xff {
                v[j] = 0x00;
            } else {
                v[j] += 1;
                break;
            }
        }
        let mut block = [0u8; 16];
        aes256_ecb(key, v, &mut block);
        temp[16 * i..16 * i + 16].copy_from_slice(&block);
    }
    if let Some(pd) = provided_data {
        for i in 0..48 {
            temp[i] ^= pd[i];
        }
    }
    key.copy_from_slice(&temp[..32]);
    v.copy_from_slice(&temp[32..48]);
}

pub fn randombytes_init(entropy_input: &[u8; 48], personalization_string: Option<&[u8; 48]>) {
    let mut seed_material = *entropy_input;
    if let Some(ps) = personalization_string {
        for i in 0..48 {
            seed_material[i] ^= ps[i];
        }
    }
    unsafe {
        let ctx = core::ptr::addr_of_mut!(DRBG_ctx);
        (*ctx).Key = [0u8; 32];
        (*ctx).V = [0u8; 16];
        let key = core::ptr::addr_of_mut!((*ctx).Key);
        let v = core::ptr::addr_of_mut!((*ctx).V);
        aes256_ctr_drbg_update(Some(&seed_material), &mut *key, &mut *v);
        (*ctx).reseed_counter = 1;
    }
}

pub fn randombytes(x: &mut [u8]) -> i32 {
    // `as_mut_ptr()` on an empty slice is non-null (dangling), so the raw
    // implementation never sees a NULL pointer from this path.
    unsafe { randombytes_raw(x.as_mut_ptr(), x.len() as u64) }
}

/// Raw-pointer core of `randombytes`, faithful to the C body. A NULL `x` with
/// `xlen == 0` is well defined: no bytes are written, but the DRBG is still
/// updated and `reseed_counter` still incremented (exactly as in C).
unsafe fn randombytes_raw(x: *mut u8, mut xlen: u64) -> i32 {
    let ctx = core::ptr::addr_of_mut!(DRBG_ctx);
    let mut i = 0usize;
    while xlen > 0 {
        // increment V
        for j in (0..16usize).rev() {
            if (*ctx).V[j] == 0xff {
                (*ctx).V[j] = 0x00;
            } else {
                (*ctx).V[j] += 1;
                break;
            }
        }
        let key = *core::ptr::addr_of!((*ctx).Key);
        let v = *core::ptr::addr_of!((*ctx).V);
        let mut block = [0u8; 16];
        aes256_ecb(&key, &v, &mut block);
        if xlen > 15 {
            core::ptr::copy_nonoverlapping(block.as_ptr(), x.add(i), 16);
            i += 16;
            xlen -= 16;
        } else {
            core::ptr::copy_nonoverlapping(block.as_ptr(), x.add(i), xlen as usize);
            xlen = 0;
        }
    }
    let key = core::ptr::addr_of_mut!((*ctx).Key);
    let v = core::ptr::addr_of_mut!((*ctx).V);
    aes256_ctr_drbg_update(None, &mut *key, &mut *v);
    (*ctx).reseed_counter = (*ctx).reseed_counter.wrapping_add(1);
    RNG_SUCCESS
}

// ---------------- seedexpander (not used by the driver, kept for parity) ----------------

/// C: `AES_XOF_struct`. `unsigned long` is 64-bit on x86-64 Linux, hence `u64`.
#[repr(C)]
pub struct AesXofStruct {
    pub buffer: [u8; 16],
    pub buffer_pos: u64,
    pub length_remaining: u64,
    pub key: [u8; 32],
    pub ctr: [u8; 16],
}

const _: () = assert!(core::mem::size_of::<AesXofStruct>() == 80);
const _: () = assert!(core::mem::align_of::<AesXofStruct>() == 8);
const _: () = assert!(core::mem::offset_of!(AesXofStruct, buffer) == 0);
const _: () = assert!(core::mem::offset_of!(AesXofStruct, buffer_pos) == 16);
const _: () = assert!(core::mem::offset_of!(AesXofStruct, length_remaining) == 24);
const _: () = assert!(core::mem::offset_of!(AesXofStruct, key) == 32);
const _: () = assert!(core::mem::offset_of!(AesXofStruct, ctr) == 64);

pub fn seedexpander_init(
    ctx: &mut AesXofStruct,
    seed: &[u8; 32],
    diversifier: &[u8; 8],
    mut maxlen: u64,
) -> i32 {
    if maxlen >= 0x100000000 {
        return RNG_BAD_MAXLEN;
    }
    ctx.length_remaining = maxlen;
    ctx.key.copy_from_slice(seed);
    ctx.ctr[..8].copy_from_slice(diversifier);
    ctx.ctr[11] = (maxlen % 256) as u8;
    maxlen >>= 8;
    ctx.ctr[10] = (maxlen % 256) as u8;
    maxlen >>= 8;
    ctx.ctr[9] = (maxlen % 256) as u8;
    maxlen >>= 8;
    ctx.ctr[8] = (maxlen % 256) as u8;
    for b in ctx.ctr[12..16].iter_mut() {
        *b = 0;
    }
    ctx.buffer_pos = 16;
    ctx.buffer = [0u8; 16];
    RNG_SUCCESS
}

pub fn seedexpander(ctx: &mut AesXofStruct, x: &mut [u8]) -> i32 {
    // `as_mut_ptr()` is never NULL (dangling for an empty slice), so the NULL
    // check inside `seedexpander_raw` never fires through this path.
    unsafe { seedexpander_raw(ctx as *mut AesXofStruct, x.as_mut_ptr(), x.len() as u64) }
}

/// Raw-pointer core of `seedexpander`, faithful to the C body: the NULL output
/// buffer check happens *before* the requested-length check.
unsafe fn seedexpander_raw(ctx: *mut AesXofStruct, x: *mut u8, mut xlen: u64) -> i32 {
    if x.is_null() {
        return RNG_BAD_OUTBUF;
    }
    if xlen >= (*ctx).length_remaining {
        return RNG_BAD_REQ_LEN;
    }
    (*ctx).length_remaining -= xlen;

    let mut offset = 0u64;
    while xlen > 0 {
        let bp = (*ctx).buffer_pos;
        let avail = 16u64.wrapping_sub(bp);
        if xlen <= avail {
            // buffer has what we need
            core::ptr::copy_nonoverlapping(
                (*ctx).buffer.as_ptr().add(bp as usize),
                x.add(offset as usize),
                xlen as usize,
            );
            (*ctx).buffer_pos = bp.wrapping_add(xlen);
            return RNG_SUCCESS;
        }

        // take what's in the buffer
        core::ptr::copy_nonoverlapping(
            (*ctx).buffer.as_ptr().add(bp as usize),
            x.add(offset as usize),
            avail as usize,
        );
        xlen -= avail;
        offset += avail;

        let key = *core::ptr::addr_of!((*ctx).key);
        let ctr = *core::ptr::addr_of!((*ctx).ctr);
        aes256_ecb(&key, &ctr, &mut *core::ptr::addr_of_mut!((*ctx).buffer));
        (*ctx).buffer_pos = 0;

        // increment the counter
        for i in (12..16usize).rev() {
            if (*ctx).ctr[i] == 0xff {
                (*ctx).ctr[i] = 0x00;
            } else {
                (*ctx).ctr[i] += 1;
                break;
            }
        }
    }
    RNG_SUCCESS
}

// ------------------------------------------------------------------
// Exported C ABI wrappers.
// ------------------------------------------------------------------

#[export_name = "randombytes_init"]
pub unsafe extern "C" fn c_randombytes_init(
    entropy_input: *mut u8,
    personalization_string: *mut u8,
) {
    let ent = &*(entropy_input as *const [u8; 48]);
    if personalization_string.is_null() {
        randombytes_init(ent, None);
    } else {
        let ps = &*(personalization_string as *const [u8; 48]);
        randombytes_init(ent, Some(ps));
    }
}

#[export_name = "randombytes"]
pub unsafe extern "C" fn c_randombytes(x: *mut u8, xlen: core::ffi::c_ulonglong) -> core::ffi::c_int {
    // NOTE: no slice is constructed, so `randombytes(NULL, 0)` behaves exactly
    // like C: no copy, but the DRBG is updated and reseed_counter incremented.
    randombytes_raw(x, xlen as u64)
}

/// C: `void AES256_ECB(unsigned char *key, unsigned char *ctr, unsigned char *buffer);`
#[export_name = "AES256_ECB"]
pub unsafe extern "C" fn c_aes256_ecb(key: *mut u8, ctr: *mut u8, buffer: *mut u8) {
    let mut k = [0u8; 32];
    core::ptr::copy_nonoverlapping(key, k.as_mut_ptr(), 32);
    let mut c = [0u8; 16];
    core::ptr::copy_nonoverlapping(ctr, c.as_mut_ptr(), 16);
    let mut out = [0u8; 16];
    aes256_ecb(&k, &c, &mut out);
    core::ptr::copy_nonoverlapping(out.as_ptr(), buffer, 16);
}

/// C: `void AES256_CTR_DRBG_Update(unsigned char *provided_data, unsigned char *Key,
/// unsigned char *V);` -- `provided_data` may be NULL.
#[export_name = "AES256_CTR_DRBG_Update"]
pub unsafe extern "C" fn c_aes256_ctr_drbg_update(
    provided_data: *mut u8,
    key: *mut u8,
    v: *mut u8,
) {
    let mut temp = [0u8; 48];

    for i in 0..3usize {
        // increment V
        for j in (0..16usize).rev() {
            let p = v.add(j);
            if *p == 0xff {
                *p = 0x00;
            } else {
                *p += 1;
                break;
            }
        }

        let mut k = [0u8; 32];
        core::ptr::copy_nonoverlapping(key, k.as_mut_ptr(), 32);
        let mut c = [0u8; 16];
        core::ptr::copy_nonoverlapping(v, c.as_mut_ptr(), 16);
        let mut block = [0u8; 16];
        aes256_ecb(&k, &c, &mut block);
        temp[16 * i..16 * i + 16].copy_from_slice(&block);
    }
    if !provided_data.is_null() {
        for i in 0..48usize {
            temp[i] ^= *provided_data.add(i);
        }
    }
    core::ptr::copy_nonoverlapping(temp.as_ptr(), key, 32);
    core::ptr::copy_nonoverlapping(temp.as_ptr().add(32), v, 16);
}

/// C: `int seedexpander_init(AES_XOF_struct *ctx, unsigned char *seed,
/// unsigned char *diversifier, unsigned long maxlen);`
#[export_name = "seedexpander_init"]
pub unsafe extern "C" fn c_seedexpander_init(
    ctx: *mut AesXofStruct,
    seed: *mut u8,
    diversifier: *mut u8,
    maxlen: core::ffi::c_ulong,
) -> core::ffi::c_int {
    // C checks maxlen before touching any pointer.
    if maxlen as u64 >= 0x1_0000_0000 {
        return RNG_BAD_MAXLEN;
    }
    let mut seed_arr = [0u8; 32];
    core::ptr::copy_nonoverlapping(seed, seed_arr.as_mut_ptr(), 32);
    let mut div = [0u8; 8];
    core::ptr::copy_nonoverlapping(diversifier, div.as_mut_ptr(), 8);
    seedexpander_init(&mut *ctx, &seed_arr, &div, maxlen as u64)
}

/// C: `int seedexpander(AES_XOF_struct *ctx, unsigned char *x, unsigned long xlen);`
#[export_name = "seedexpander"]
pub unsafe extern "C" fn c_seedexpander(
    ctx: *mut AesXofStruct,
    x: *mut u8,
    xlen: core::ffi::c_ulong,
) -> core::ffi::c_int {
    seedexpander_raw(ctx, x, xlen as u64)
}
