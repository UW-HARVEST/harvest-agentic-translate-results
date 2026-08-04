// Translation of c_src/app/src/rng.c — NIST AES256-CTR DRBG used by the KAT
// driver. This is a deterministic RNG. Uses the pure-Rust `aes` crate for
// AES-256-ECB (not OpenSSL).

use aes::cipher::{BlockEncrypt, KeyInit};
use aes::Aes256;

pub const RNG_SUCCESS: i32 = 0;

pub struct Aes256CtrDrbg {
    pub key: [u8; 32],
    pub v: [u8; 16],
    pub reseed_counter: i32,
}

static mut DRBG_CTX: Aes256CtrDrbg = Aes256CtrDrbg {
    key: [0u8; 32],
    v: [0u8; 16],
    reseed_counter: 0,
};

fn aes256_ecb(key: &[u8; 32], ctr: &[u8; 16], buffer: &mut [u8; 16]) {
    let cipher = Aes256::new_from_slice(key).expect("aes256 key");
    let mut block = aes::cipher::generic_array::GenericArray::clone_from_slice(ctr);
    cipher.encrypt_block(&mut block);
    buffer.copy_from_slice(block.as_slice());
}

fn aes256_ctr_drbg_update(provided_data: Option<&[u8; 48]>, key: &mut [u8; 32], v: &mut [u8; 16]) {
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
        let mut buffer = [0u8; 16];
        aes256_ecb(key, v, &mut buffer);
        temp[16 * i..16 * (i + 1)].copy_from_slice(&buffer);
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
    let mut seed_material = [0u8; 48];
    seed_material.copy_from_slice(entropy_input);
    if let Some(p) = personalization_string {
        for i in 0..48 {
            seed_material[i] ^= p[i];
        }
    }
    unsafe {
        DRBG_CTX.key = [0u8; 32];
        DRBG_CTX.v = [0u8; 16];
        aes256_ctr_drbg_update(Some(&seed_material), &mut DRBG_CTX.key, &mut DRBG_CTX.v);
        DRBG_CTX.reseed_counter = 1;
    }
}

pub fn randombytes(x: &mut [u8], xlen: u64) -> i32 {
    let mut block = [0u8; 16];
    let mut i: usize = 0;
    let mut remaining = xlen as usize;

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
            aes256_ecb(&DRBG_CTX.key, &DRBG_CTX.v, &mut block);
            if remaining > 15 {
                x[i..i + 16].copy_from_slice(&block);
                i += 16;
                remaining -= 16;
            } else {
                x[i..i + remaining].copy_from_slice(&block[..remaining]);
                remaining = 0;
            }
        }
        aes256_ctr_drbg_update(None, &mut DRBG_CTX.key, &mut DRBG_CTX.v);
        DRBG_CTX.reseed_counter += 1;
    }
    RNG_SUCCESS
}
