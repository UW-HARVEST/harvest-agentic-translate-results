use aes::cipher::{BlockEncrypt, KeyInit, generic_array::GenericArray};
use aes::Aes256;
use std::cell::RefCell;

struct Drbg {
    key: [u8; 32],
    v: [u8; 16],
    reseed_counter: i32,
}

thread_local! {
    static DRBG_CTX: RefCell<Drbg> = RefCell::new(Drbg {
        key: [0u8; 32],
        v: [0u8; 16],
        reseed_counter: 0,
    });
}

fn aes256_ecb(key: &[u8; 32], ctr: &[u8; 16], buffer: &mut [u8; 16]) {
    let cipher = Aes256::new(GenericArray::from_slice(key));
    let mut block = GenericArray::clone_from_slice(ctr);
    cipher.encrypt_block(&mut block);
    buffer.copy_from_slice(&block);
}

fn aes256_ctr_drbg_update(provided_data: Option<&[u8; 48]>, key: &mut [u8; 32], v: &mut [u8; 16]) {
    let mut temp = [0u8; 48];
    let mut key_copy = [0u8; 32];
    key_copy.copy_from_slice(key);
    for i in 0..3 {
        // increment V
        for j in (0..16).rev() {
            if v[j] == 0xff { v[j] = 0x00; } else { v[j] += 1; break; }
        }
        let mut block = [0u8; 16];
        aes256_ecb(&key_copy, v, &mut block);
        temp[16 * i..16 * i + 16].copy_from_slice(&block);
    }
    if let Some(pd) = provided_data {
        for i in 0..48 { temp[i] ^= pd[i]; }
    }
    key.copy_from_slice(&temp[..32]);
    v.copy_from_slice(&temp[32..48]);
}

pub fn randombytes_init(entropy_input: &[u8; 48], personalization_string: Option<&[u8; 48]>) {
    let mut seed_material = [0u8; 48];
    seed_material.copy_from_slice(entropy_input);
    if let Some(ps) = personalization_string {
        for i in 0..48 { seed_material[i] ^= ps[i]; }
    }
    DRBG_CTX.with(|ctx| {
        let mut ctx = ctx.borrow_mut();
        ctx.key = [0u8; 32];
        ctx.v = [0u8; 16];
        let mut key = ctx.key;
        let mut v = ctx.v;
        aes256_ctr_drbg_update(Some(&seed_material), &mut key, &mut v);
        ctx.key = key;
        ctx.v = v;
        ctx.reseed_counter = 1;
    });
}

pub fn randombytes(x: &mut [u8]) {
    DRBG_CTX.with(|ctx| {
        let mut ctx = ctx.borrow_mut();
        let mut xlen = x.len();
        let mut i = 0usize;
        while xlen > 0 {
            for j in (0..16).rev() {
                if ctx.v[j] == 0xff { ctx.v[j] = 0x00; } else { ctx.v[j] += 1; break; }
            }
            let mut block = [0u8; 16];
            aes256_ecb(&ctx.key, &ctx.v, &mut block);
            if xlen > 15 {
                x[i..i + 16].copy_from_slice(&block);
                i += 16;
                xlen -= 16;
            } else {
                x[i..i + xlen].copy_from_slice(&block[..xlen]);
                xlen = 0;
            }
        }
        let mut key = ctx.key;
        let mut v = ctx.v;
        aes256_ctr_drbg_update(None, &mut key, &mut v);
        ctx.key = key;
        ctx.v = v;
        ctx.reseed_counter += 1;
    });
}
