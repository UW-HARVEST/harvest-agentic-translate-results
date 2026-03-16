use openssl::symm::{Cipher, Crypter, Mode};
use std::cell::RefCell;

pub struct Aes256CtrDrbgStruct {
    pub key: [u8; 32],
    pub v: [u8; 16],
    pub reseed_counter: i32,
}

thread_local! {
    static DRBG_CTX: RefCell<Aes256CtrDrbgStruct> = RefCell::new(Aes256CtrDrbgStruct {
        key: [0u8; 32],
        v: [0u8; 16],
        reseed_counter: 0,
    });
}

fn aes256_ecb(key: &[u8], ctr: &[u8], buffer: &mut [u8]) {
    let cipher = Cipher::aes_256_ecb();
    let mut crypter = Crypter::new(cipher, Mode::Encrypt, key, None).unwrap();
    crypter.pad(false);
    let count = crypter.update(ctr, buffer).unwrap();
    let _ = crypter.finalize(&mut buffer[count..]).unwrap();
}

fn ctr_drbg_update(provided_data: Option<&[u8]>, drbg: &mut Aes256CtrDrbgStruct) {
    let mut temp = [0u8; 48];
    for i in 0..3 {
        // increment V
        let mut j: i32 = 15;
        while j >= 0 {
            if drbg.v[j as usize] == 0xff {
                drbg.v[j as usize] = 0x00;
            } else {
                drbg.v[j as usize] += 1;
                break;
            }
            j -= 1;
        }
        aes256_ecb(&drbg.key, &drbg.v, &mut temp[16 * i..16 * i + 16]);
    }
    if let Some(pd) = provided_data {
        for i in 0..48 {
            temp[i] ^= pd[i];
        }
    }
    drbg.key.copy_from_slice(&temp[..32]);
    drbg.v.copy_from_slice(&temp[32..48]);
}

pub fn randombytes_init(entropy_input: &[u8], personalization_string: Option<&[u8]>) {
    let mut seed_material = [0u8; 48];
    seed_material.copy_from_slice(&entropy_input[..48]);
    if let Some(ps) = personalization_string {
        for i in 0..48 {
            seed_material[i] ^= ps[i];
        }
    }
    DRBG_CTX.with(|ctx| {
        let mut ctx = ctx.borrow_mut();
        ctx.key = [0u8; 32];
        ctx.v = [0u8; 16];
        ctr_drbg_update(Some(&seed_material), &mut ctx);
        ctx.reseed_counter = 1;
    });
}

pub fn randombytes(x: &mut [u8], xlen: u64) {
    DRBG_CTX.with(|ctx| {
        let mut ctx = ctx.borrow_mut();
        let mut block = [0u8; 16];
        let mut i: usize = 0;
        let mut remaining = xlen as usize;

        while remaining > 0 {
            // increment V
            let mut j: i32 = 15;
            while j >= 0 {
                if ctx.v[j as usize] == 0xff {
                    ctx.v[j as usize] = 0x00;
                } else {
                    ctx.v[j as usize] += 1;
                    break;
                }
                j -= 1;
            }
            aes256_ecb(&ctx.key, &ctx.v, &mut block);
            if remaining > 15 {
                x[i..i + 16].copy_from_slice(&block);
                i += 16;
                remaining -= 16;
            } else {
                x[i..i + remaining].copy_from_slice(&block[..remaining]);
                remaining = 0;
            }
        }
        ctr_drbg_update(None, &mut ctx);
        ctx.reseed_counter += 1;
    });
}
