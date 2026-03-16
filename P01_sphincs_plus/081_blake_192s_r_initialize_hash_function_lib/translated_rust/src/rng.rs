use crate::context::SpxCtx;
use crate::params::*;

pub fn randombytes(x: &mut [u8], xlen: u64) {
    use std::fs::File;
    use std::io::Read;
    let mut f = File::open("/dev/urandom").expect("Failed to open /dev/urandom");
    let mut remaining = xlen as usize;
    let mut offset = 0;
    while remaining > 0 {
        let chunk = std::cmp::min(remaining, 1048576);
        let n = f.read(&mut x[offset..offset + chunk]).unwrap_or(0);
        if n < 1 {
            std::thread::sleep(std::time::Duration::from_secs(1));
            continue;
        }
        offset += n;
        remaining -= n;
    }
}

// ---- RNG (deterministic, OpenSSL AES) ----

use openssl::symm::{Cipher, Crypter, Mode};

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

fn aes256_ecb(key: &[u8], ctr: &[u8], buffer: &mut [u8]) {
    let cipher = Cipher::aes_256_ecb();
    let mut crypter = Crypter::new(cipher, Mode::Encrypt, key, None).unwrap();
    crypter.pad(false);
    let count = crypter.update(ctr, buffer).unwrap();
    let _ = crypter.finalize(&mut buffer[count..]);
}

pub fn aes256_ctr_drbg_update(provided_data: Option<&[u8]>, key: &mut [u8], v: &mut [u8]) {
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
        aes256_ecb(key, v, &mut temp[16 * i..16 * i + 16]);
    }
    if let Some(pd) = provided_data {
        for i in 0..48 {
            temp[i] ^= pd[i];
        }
    }
    key[..32].copy_from_slice(&temp[..32]);
    v[..16].copy_from_slice(&temp[32..48]);
}

pub fn seedexpander_init(
    ctx: &mut AesXofStruct,
    seed: &[u8],
    diversifier: &[u8],
    maxlen: u64,
) -> i32 {
    if maxlen >= 0x100000000 {
        return -1; // RNG_BAD_MAXLEN
    }
    ctx.length_remaining = maxlen;
    ctx.key[..32].copy_from_slice(&seed[..32]);
    ctx.ctr[..8].copy_from_slice(&diversifier[..8]);
    let mut ml = maxlen;
    ctx.ctr[11] = (ml % 256) as u8; ml >>= 8;
    ctx.ctr[10] = (ml % 256) as u8; ml >>= 8;
    ctx.ctr[9] = (ml % 256) as u8; ml >>= 8;
    ctx.ctr[8] = (ml % 256) as u8;
    ctx.ctr[12..16].fill(0);
    ctx.buffer_pos = 16;
    ctx.buffer.fill(0);
    0
}

pub fn seedexpander(ctx: &mut AesXofStruct, x: &mut [u8], xlen: u64) -> i32 {
    if x.is_empty() {
        return -2; // RNG_BAD_OUTBUF
    }
    let mut xlen = xlen as u64;
    if xlen >= ctx.length_remaining {
        return -3; // RNG_BAD_REQ_LEN
    }
    ctx.length_remaining -= xlen;
    let mut offset: usize = 0;
    while xlen > 0 {
        let avail = 16 - ctx.buffer_pos;
        if xlen <= avail {
            x[offset..offset + xlen as usize]
                .copy_from_slice(&ctx.buffer[ctx.buffer_pos as usize..(ctx.buffer_pos + xlen) as usize]);
            ctx.buffer_pos += xlen;
            return 0;
        }
        x[offset..offset + avail as usize]
            .copy_from_slice(&ctx.buffer[ctx.buffer_pos as usize..16]);
        xlen -= avail;
        offset += avail as usize;

        aes256_ecb(&ctx.key, &ctx.ctr.clone(), &mut ctx.buffer);
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
    0
}

pub fn randombytes_init(entropy_input: &[u8], personalization_string: Option<&[u8]>) {
    let mut seed_material = [0u8; 48];
    seed_material.copy_from_slice(&entropy_input[..48]);
    if let Some(ps) = personalization_string {
        for i in 0..48 {
            seed_material[i] ^= ps[i];
        }
    }
    unsafe {
        DRBG_CTX.key.fill(0);
        DRBG_CTX.v.fill(0);
        aes256_ctr_drbg_update(Some(&seed_material), &mut DRBG_CTX.key, &mut DRBG_CTX.v);
        DRBG_CTX.reseed_counter = 1;
    }
}

pub fn randombytes_det(x: &mut [u8], xlen: u64) -> i32 {
    let mut block = [0u8; 16];
    let mut i: usize = 0;
    let mut remaining = xlen;

    unsafe {
        while remaining > 0 {
            // increment V
            for j in (0..16).rev() {
                if DRBG_CTX.v[j] == 0xff {
                    DRBG_CTX.v[j] = 0x00;
                } else {
                    DRBG_CTX.v[j] += 1;
                    break;
                }
            }
            aes256_ecb(&DRBG_CTX.key.clone(), &DRBG_CTX.v.clone(), &mut block);
            if remaining > 15 {
                x[i..i + 16].copy_from_slice(&block);
                i += 16;
                remaining -= 16;
            } else {
                x[i..i + remaining as usize].copy_from_slice(&block[..remaining as usize]);
                remaining = 0;
            }
        }
        aes256_ctr_drbg_update(None, &mut DRBG_CTX.key, &mut DRBG_CTX.v);
        DRBG_CTX.reseed_counter += 1;
    }
    0
}
