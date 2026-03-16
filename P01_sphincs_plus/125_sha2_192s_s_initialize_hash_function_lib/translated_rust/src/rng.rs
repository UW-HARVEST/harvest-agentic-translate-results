use crate::context::{AesXofStruct, Aes256CtrDrbgStruct};
use crate::params::*;
use aes::cipher::{BlockEncrypt, KeyInit};
use aes::cipher::generic_array::GenericArray;
use aes::Aes256;

fn aes256_ecb(key: &[u8], ctr: &[u8], buffer: &mut [u8]) {
    let cipher = Aes256::new(GenericArray::from_slice(&key[..32]));
    let mut block = *GenericArray::from_slice(&ctr[..16]);
    cipher.encrypt_block(&mut block);
    buffer[..16].copy_from_slice(&block);
}

static mut DRBG_CTX: Aes256CtrDrbgStruct = Aes256CtrDrbgStruct {
    key: [0u8; 32],
    v: [0u8; 16],
    reseed_counter: 0,
};

pub fn aes256_ctr_drbg_update_internal(
    provided_data: *const u8,
    key: &mut [u8; 32],
    v: &mut [u8; 16],
) {
    let mut temp = [0u8; 48];
    for i in 0..3 {
        for j in (0..16).rev() {
            if v[j] == 0xff {
                v[j] = 0x00;
            } else {
                v[j] += 1;
                break;
            }
        }
        aes256_ecb(key, v, &mut temp[16 * i..]);
    }
    if !provided_data.is_null() {
        let pd = unsafe { core::slice::from_raw_parts(provided_data, 48) };
        for i in 0..48 {
            temp[i] ^= pd[i];
        }
    }
    key.copy_from_slice(&temp[..32]);
    v.copy_from_slice(&temp[32..48]);
}

pub fn seedexpander_init_internal(
    ctx: &mut AesXofStruct,
    seed: &[u8],
    diversifier: &[u8],
    mut maxlen: u64,
) -> i32 {
    if maxlen >= 0x100000000 {
        return RNG_BAD_MAXLEN;
    }
    ctx.length_remaining = maxlen;
    ctx.key.copy_from_slice(&seed[..32]);
    ctx.ctr[..8].copy_from_slice(&diversifier[..8]);
    ctx.ctr[11] = (maxlen % 256) as u8; maxlen >>= 8;
    ctx.ctr[10] = (maxlen % 256) as u8; maxlen >>= 8;
    ctx.ctr[9] = (maxlen % 256) as u8; maxlen >>= 8;
    ctx.ctr[8] = (maxlen % 256) as u8;
    ctx.ctr[12..16].fill(0);
    ctx.buffer_pos = 16;
    ctx.buffer.fill(0);
    RNG_SUCCESS
}

pub fn seedexpander_internal(ctx: &mut AesXofStruct, x: &mut [u8], mut xlen: u64) -> i32 {
    if x.is_empty() {
        return RNG_BAD_OUTBUF;
    }
    if xlen >= ctx.length_remaining {
        return RNG_BAD_REQ_LEN;
    }
    ctx.length_remaining -= xlen;
    let mut offset = 0usize;
    while xlen > 0 {
        let bp = ctx.buffer_pos as usize;
        if xlen <= (16 - bp) as u64 {
            let xl = xlen as usize;
            x[offset..offset + xl].copy_from_slice(&ctx.buffer[bp..bp + xl]);
            ctx.buffer_pos += xlen;
            return RNG_SUCCESS;
        }
        let take = 16 - bp;
        x[offset..offset + take].copy_from_slice(&ctx.buffer[bp..16]);
        xlen -= take as u64;
        offset += take;

        let ctr_copy = ctx.ctr;
        aes256_ecb(&ctx.key, &ctr_copy, &mut ctx.buffer);
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

pub fn randombytes_init_internal(entropy_input: &[u8], personalization_string: *const u8) {
    let mut seed_material = [0u8; 48];
    seed_material.copy_from_slice(&entropy_input[..48]);
    if !personalization_string.is_null() {
        let ps = unsafe { core::slice::from_raw_parts(personalization_string, 48) };
        for i in 0..48 {
            seed_material[i] ^= ps[i];
        }
    }
    unsafe {
        DRBG_CTX.key.fill(0);
        DRBG_CTX.v.fill(0);
        aes256_ctr_drbg_update_internal(
            seed_material.as_ptr(),
            &mut DRBG_CTX.key,
            &mut DRBG_CTX.v,
        );
        DRBG_CTX.reseed_counter = 1;
    }
}

pub fn rng_randombytes_internal(x: &mut [u8], mut xlen: u64) -> i32 {
    let mut block = [0u8; 16];
    let mut i = 0usize;
    unsafe {
        while xlen > 0 {
            for j in (0..16).rev() {
                if DRBG_CTX.v[j] == 0xff {
                    DRBG_CTX.v[j] = 0x00;
                } else {
                    DRBG_CTX.v[j] += 1;
                    break;
                }
            }
            let v_copy = DRBG_CTX.v;
            aes256_ecb(&DRBG_CTX.key, &v_copy, &mut block);
            if xlen > 15 {
                x[i..i + 16].copy_from_slice(&block);
                i += 16;
                xlen -= 16;
            } else {
                let xl = xlen as usize;
                x[i..i + xl].copy_from_slice(&block[..xl]);
                xlen = 0;
            }
        }
        aes256_ctr_drbg_update_internal(
            core::ptr::null(),
            &mut DRBG_CTX.key,
            &mut DRBG_CTX.v,
        );
        DRBG_CTX.reseed_counter += 1;
    }
    RNG_SUCCESS
}

pub fn randombytes_urandom(x: &mut [u8], mut xlen: u64) {
    use std::fs::File;
    use std::io::Read;
    let mut f = File::open("/dev/urandom").expect("Failed to open /dev/urandom");
    let mut offset = 0usize;
    while xlen > 0 {
        let to_read = if xlen < 1048576 { xlen as usize } else { 1048576 };
        match f.read(&mut x[offset..offset + to_read]) {
            Ok(n) if n > 0 => {
                offset += n;
                xlen -= n as u64;
            }
            _ => {
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
        }
    }
}
