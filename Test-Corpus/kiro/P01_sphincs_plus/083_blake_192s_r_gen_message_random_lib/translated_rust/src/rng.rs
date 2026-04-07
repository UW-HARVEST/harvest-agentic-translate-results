use aes::Aes256;
use aes::cipher::BlockEncrypt;
use aes::cipher::KeyInit;
use aes::cipher::generic_array::GenericArray;

pub const RNG_SUCCESS: i32 = 0;
pub const RNG_BAD_MAXLEN: i32 = -1;
pub const RNG_BAD_OUTBUF: i32 = -2;
pub const RNG_BAD_REQ_LEN: i32 = -3;

#[repr(C)]
pub struct AesXofStruct {
    pub buffer: [u8; 16],
    pub buffer_pos: u64,
    pub length_remaining: u64,
    pub key: [u8; 32],
    pub ctr: [u8; 16],
}

#[repr(C)]
pub struct Aes256CtrDrbgStruct {
    pub key: [u8; 32],
    pub v: [u8; 16],
    pub reseed_counter: i32,
}

static mut DRBG_CTX: Aes256CtrDrbgStruct = Aes256CtrDrbgStruct {
    key: [0u8; 32],
    v: [0u8; 16],
    reseed_counter: 0,
};

pub fn aes256_ecb_export(key: &[u8; 32], ctr: &[u8; 16], buffer: &mut [u8; 16]) {
    aes256_ecb(key, ctr, buffer);
}

fn aes256_ecb(key: &[u8; 32], ctr: &[u8; 16], buffer: &mut [u8; 16]) {
    let cipher = Aes256::new(GenericArray::from_slice(key));
    let mut block = *GenericArray::from_slice(ctr);
    cipher.encrypt_block(&mut block);
    buffer.copy_from_slice(&block);
}

#[unsafe(no_mangle)]
pub extern "C" fn seedexpander_init(
    ctx: *mut AesXofStruct,
    seed: *const u8,
    diversifier: *const u8,
    maxlen: u64,
) -> i32 {
    if maxlen >= 0x100000000 {
        return RNG_BAD_MAXLEN;
    }
    let ctx = unsafe { &mut *ctx };
    ctx.length_remaining = maxlen;

    unsafe { std::ptr::copy_nonoverlapping(seed, ctx.key.as_mut_ptr(), 32); }
    unsafe { std::ptr::copy_nonoverlapping(diversifier, ctx.ctr.as_mut_ptr(), 8); }

    let mut ml = maxlen;
    ctx.ctr[11] = (ml % 256) as u8; ml >>= 8;
    ctx.ctr[10] = (ml % 256) as u8; ml >>= 8;
    ctx.ctr[9] = (ml % 256) as u8; ml >>= 8;
    ctx.ctr[8] = (ml % 256) as u8;
    ctx.ctr[12..16].fill(0);

    ctx.buffer_pos = 16;
    ctx.buffer.fill(0);

    RNG_SUCCESS
}

#[unsafe(no_mangle)]
pub extern "C" fn seedexpander(
    ctx: *mut AesXofStruct,
    x: *mut u8,
    mut xlen: u64,
) -> i32 {
    let ctx = unsafe { &mut *ctx };
    if x.is_null() {
        return RNG_BAD_OUTBUF;
    }
    if xlen >= ctx.length_remaining {
        return RNG_BAD_REQ_LEN;
    }

    ctx.length_remaining -= xlen;
    let mut offset: u64 = 0;

    while xlen > 0 {
        let bp = ctx.buffer_pos as usize;
        if xlen <= (16 - bp) as u64 {
            unsafe {
                std::ptr::copy_nonoverlapping(
                    ctx.buffer.as_ptr().add(bp),
                    x.add(offset as usize),
                    xlen as usize,
                );
            }
            ctx.buffer_pos += xlen;
            return RNG_SUCCESS;
        }

        let take = 16 - bp;
        unsafe {
            std::ptr::copy_nonoverlapping(
                ctx.buffer.as_ptr().add(bp),
                x.add(offset as usize),
                take,
            );
        }
        xlen -= take as u64;
        offset += take as u64;

        let mut key = [0u8; 32];
        let mut ctr = [0u8; 16];
        key.copy_from_slice(&ctx.key);
        ctr.copy_from_slice(&ctx.ctr);
        let mut buf = [0u8; 16];
        aes256_ecb(&key, &ctr, &mut buf);
        ctx.buffer.copy_from_slice(&buf);
        ctx.buffer_pos = 0;

        for i in (12..=15).rev() {
            if ctx.ctr[i] == 0xff {
                ctx.ctr[i] = 0x00;
            } else {
                ctx.ctr[i] += 1;
                break;
            }
        }
    }

    RNG_SUCCESS
}

#[unsafe(no_mangle)]
pub extern "C" fn randombytes_init(
    entropy_input: *const u8,
    personalization_string: *const u8,
) {
    let mut seed_material = [0u8; 48];
    unsafe {
        std::ptr::copy_nonoverlapping(entropy_input, seed_material.as_mut_ptr(), 48);
    }
    if !personalization_string.is_null() {
        let ps = unsafe { std::slice::from_raw_parts(personalization_string, 48) };
        for i in 0..48 {
            seed_material[i] ^= ps[i];
        }
    }
    unsafe {
        DRBG_CTX.key.fill(0);
        DRBG_CTX.v.fill(0);
        aes256_ctr_drbg_update_internal(
            Some(&seed_material),
            &mut DRBG_CTX.key,
            &mut DRBG_CTX.v,
        );
        DRBG_CTX.reseed_counter = 1;
    }
}

fn aes256_ctr_drbg_update_internal(
    provided_data: Option<&[u8; 48]>,
    key: &mut [u8; 32],
    v: &mut [u8; 16],
) {
    let mut temp = [0u8; 48];
    for i in 0..3 {
        // increment V
        for j in (0..16).rev() {
            if v[j] == 0xff {
                v[j] = 0x00;
            } else {
                v[j] += 1;
                break;
            }
        }
        let mut buf = [0u8; 16];
        let k: [u8; 32] = *key;
        let ctr: [u8; 16] = *v;
        aes256_ecb(&k, &ctr, &mut buf);
        temp[16 * i..16 * i + 16].copy_from_slice(&buf);
    }
    if let Some(pd) = provided_data {
        for i in 0..48 {
            temp[i] ^= pd[i];
        }
    }
    key.copy_from_slice(&temp[..32]);
    v.copy_from_slice(&temp[32..48]);
}

#[unsafe(no_mangle)]
pub extern "C" fn randombytes(x: *mut u8, mut xlen: u64) -> i32 {
    unsafe { randombytes_internal(x, xlen) }
}

unsafe fn randombytes_internal(x: *mut u8, mut xlen: u64) -> i32 {
    let mut block = [0u8; 16];
    let mut i: usize = 0;

    while xlen > 0 {
        // increment V
        for j in (0..16).rev() {
            if DRBG_CTX.v[j] == 0xff {
                DRBG_CTX.v[j] = 0x00;
            } else {
                DRBG_CTX.v[j] += 1;
                break;
            }
        }
        let k: [u8; 32] = DRBG_CTX.key;
        let ctr: [u8; 16] = DRBG_CTX.v;
        aes256_ecb(&k, &ctr, &mut block);
        if xlen > 15 {
            std::ptr::copy_nonoverlapping(block.as_ptr(), x.add(i), 16);
            i += 16;
            xlen -= 16;
        } else {
            std::ptr::copy_nonoverlapping(block.as_ptr(), x.add(i), xlen as usize);
            xlen = 0;
        }
    }
    aes256_ctr_drbg_update_internal(None, &mut DRBG_CTX.key, &mut DRBG_CTX.v);
    DRBG_CTX.reseed_counter += 1;

    RNG_SUCCESS
}

/// Safe wrapper for calling randombytes from Rust code
pub fn randombytes_rust(buf: &mut [u8]) {
    unsafe {
        randombytes(buf.as_mut_ptr(), buf.len() as u64);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn AES256_CTR_DRBG_Update(
    provided_data: *const u8,
    key: *mut u8,
    v: *mut u8,
) {
    unsafe {
        let key_arr = &mut *(key as *mut [u8; 32]);
        let v_arr = &mut *(v as *mut [u8; 16]);
        if provided_data.is_null() {
            aes256_ctr_drbg_update_internal(None, key_arr, v_arr);
        } else {
            let pd = &*(provided_data as *const [u8; 48]);
            aes256_ctr_drbg_update_internal(Some(pd), key_arr, v_arr);
        }
    }
}
