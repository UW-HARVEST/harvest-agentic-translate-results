use aes::cipher::{BlockEncrypt, KeyInit};
use aes::Aes256;
use aes::cipher::generic_array::GenericArray;

pub struct Aes256CtrDrbgStruct {
    pub key: [u8; 32],
    pub v: [u8; 16],
    pub reseed_counter: i32,
}

pub fn aes256_ecb(key: &[u8; 32], ctr: &[u8; 16], buffer: &mut [u8; 16]) {
    let cipher = Aes256::new(GenericArray::from_slice(key));
    let mut block = GenericArray::clone_from_slice(ctr);
    cipher.encrypt_block(&mut block);
    buffer.copy_from_slice(&block);
}

pub fn aes256_ctr_drbg_update(provided_data: Option<&[u8; 48]>, key: &mut [u8; 32], v: &mut [u8; 16]) {
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
        let mut block = [0u8; 16];
        aes256_ecb(key, v, &mut block);
        temp[i * 16..(i + 1) * 16].copy_from_slice(&block);
    }
    if let Some(data) = provided_data {
        for i in 0..48 {
            temp[i] ^= data[i];
        }
    }
    key.copy_from_slice(&temp[..32]);
    v.copy_from_slice(&temp[32..48]);
}

pub fn randombytes_init(drbg: &mut Aes256CtrDrbgStruct, entropy_input: &[u8; 48], personalization_string: Option<&[u8; 48]>) {
    let mut seed_material = [0u8; 48];
    seed_material.copy_from_slice(entropy_input);
    if let Some(pers) = personalization_string {
        for i in 0..48 {
            seed_material[i] ^= pers[i];
        }
    }
    drbg.key.fill(0);
    drbg.v.fill(0);
    aes256_ctr_drbg_update(Some(&seed_material), &mut drbg.key, &mut drbg.v);
    drbg.reseed_counter = 1;
}

pub fn randombytes(drbg: &mut Aes256CtrDrbgStruct, x: &mut [u8]) {
    let mut xlen = x.len();
    let mut i = 0;
    while xlen > 0 {
        for j in (0..16).rev() {
            if drbg.v[j] == 0xff {
                drbg.v[j] = 0x00;
            } else {
                drbg.v[j] += 1;
                break;
            }
        }
        let mut block = [0u8; 16];
        aes256_ecb(&drbg.key, &drbg.v, &mut block);
        if xlen > 15 {
            x[i..i + 16].copy_from_slice(&block);
            i += 16;
            xlen -= 16;
        } else {
            x[i..i + xlen].copy_from_slice(&block[..xlen]);
            xlen = 0;
        }
    }
    aes256_ctr_drbg_update(None, &mut drbg.key, &mut drbg.v);
    drbg.reseed_counter += 1;
}
