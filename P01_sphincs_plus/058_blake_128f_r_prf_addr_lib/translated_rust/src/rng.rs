use aes::Aes256;
use aes::cipher::{BlockEncrypt, KeyInit, generic_array::GenericArray};

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

fn aes256_ecb(key: &[u8; 32], ctr: &[u8; 16], buffer: &mut [u8; 16]) {
    let cipher = Aes256::new(GenericArray::from_slice(key));
    let mut block = GenericArray::clone_from_slice(ctr);
    cipher.encrypt_block(&mut block);
    buffer.copy_from_slice(&block);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn AES256_ECB(key: *mut u8, ctr: *mut u8, buffer: *mut u8) {
    let k = &*(key as *const [u8; 32]);
    let c = &*(ctr as *const [u8; 16]);
    let b = &mut *(buffer as *mut [u8; 16]);
    aes256_ecb(k, c, b);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn seedexpander_init(
    ctx: *mut AesXofStruct,
    seed: *const u8,
    diversifier: *const u8,
    maxlen: u64,
) -> i32 {
    if maxlen >= 0x100000000 {
        return RNG_BAD_MAXLEN;
    }

    let ctx = &mut *ctx;
    ctx.length_remaining = maxlen;

    core::ptr::copy_nonoverlapping(seed, ctx.key.as_mut_ptr(), 32);
    core::ptr::copy_nonoverlapping(diversifier, ctx.ctr.as_mut_ptr(), 8);

    let mut ml = maxlen;
    ctx.ctr[11] = (ml % 256) as u8;
    ml >>= 8;
    ctx.ctr[10] = (ml % 256) as u8;
    ml >>= 8;
    ctx.ctr[9] = (ml % 256) as u8;
    ml >>= 8;
    ctx.ctr[8] = (ml % 256) as u8;
    ctx.ctr[12..16].fill(0);

    ctx.buffer_pos = 16;
    ctx.buffer.fill(0);

    RNG_SUCCESS
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn seedexpander(ctx: *mut AesXofStruct, x: *mut u8, mut xlen: u64) -> i32 {
    if x.is_null() {
        return RNG_BAD_OUTBUF;
    }
    let ctx = &mut *ctx;
    if xlen >= ctx.length_remaining {
        return RNG_BAD_REQ_LEN;
    }

    ctx.length_remaining -= xlen;
    let mut offset: u64 = 0;

    while xlen > 0 {
        if xlen <= 16 - ctx.buffer_pos {
            core::ptr::copy_nonoverlapping(
                ctx.buffer.as_ptr().add(ctx.buffer_pos as usize),
                x.add(offset as usize),
                xlen as usize,
            );
            ctx.buffer_pos += xlen;
            return RNG_SUCCESS;
        }

        let avail = 16 - ctx.buffer_pos;
        core::ptr::copy_nonoverlapping(
            ctx.buffer.as_ptr().add(ctx.buffer_pos as usize),
            x.add(offset as usize),
            avail as usize,
        );
        xlen -= avail;
        offset += avail;

        aes256_ecb(&ctx.key, &ctx.ctr, &mut ctx.buffer);
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
pub unsafe extern "C" fn AES256_CTR_DRBG_Update(
    provided_data: *const u8,
    key: *mut u8,
    v: *mut u8,
) {
    let mut temp = [0u8; 48];
    let key_arr = &*(key as *const [u8; 32]);
    let v_arr = &mut *(v as *mut [u8; 16]);

    for i in 0..3 {
        // increment V
        for j in (0..16).rev() {
            if v_arr[j] == 0xff {
                v_arr[j] = 0x00;
            } else {
                v_arr[j] += 1;
                break;
            }
        }
        let mut block = [0u8; 16];
        aes256_ecb(key_arr, v_arr, &mut block);
        temp[16 * i..16 * (i + 1)].copy_from_slice(&block);
    }

    if !provided_data.is_null() {
        for i in 0..48 {
            temp[i] ^= *provided_data.add(i);
        }
    }

    core::ptr::copy_nonoverlapping(temp.as_ptr(), key, 32);
    core::ptr::copy_nonoverlapping(temp.as_ptr().add(32), v, 16);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn randombytes_init(
    entropy_input: *const u8,
    personalization_string: *const u8,
) {
    let mut seed_material = [0u8; 48];
    core::ptr::copy_nonoverlapping(entropy_input, seed_material.as_mut_ptr(), 48);

    if !personalization_string.is_null() {
        for i in 0..48 {
            seed_material[i] ^= *personalization_string.add(i);
        }
    }

    DRBG_CTX.key.fill(0);
    DRBG_CTX.v.fill(0);
    AES256_CTR_DRBG_Update(
        seed_material.as_ptr(),
        DRBG_CTX.key.as_mut_ptr(),
        DRBG_CTX.v.as_mut_ptr(),
    );
    DRBG_CTX.reseed_counter = 1;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rng_randombytes(x: *mut u8, mut xlen: u64) -> i32 {
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
        aes256_ecb(&DRBG_CTX.key, &DRBG_CTX.v, &mut block);
        if xlen > 15 {
            core::ptr::copy_nonoverlapping(block.as_ptr(), x.add(i), 16);
            i += 16;
            xlen -= 16;
        } else {
            core::ptr::copy_nonoverlapping(block.as_ptr(), x.add(i), xlen as usize);
            xlen = 0;
        }
    }
    AES256_CTR_DRBG_Update(
        core::ptr::null(),
        DRBG_CTX.key.as_mut_ptr(),
        DRBG_CTX.v.as_mut_ptr(),
    );
    DRBG_CTX.reseed_counter += 1;

    RNG_SUCCESS
}
