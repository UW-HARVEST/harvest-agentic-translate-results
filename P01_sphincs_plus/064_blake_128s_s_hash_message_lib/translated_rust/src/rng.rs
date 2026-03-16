use openssl::symm::{Cipher, Crypter, Mode};
use std::sync::Mutex;

static DRBG_CTX: Mutex<Aes256CtrDrbg> = Mutex::new(Aes256CtrDrbg::new());

struct Aes256CtrDrbg {
    key: [u8; 32],
    v: [u8; 16],
    reseed_counter: i32,
}

impl Aes256CtrDrbg {
    const fn new() -> Self {
        Self { key: [0u8; 32], v: [0u8; 16], reseed_counter: 0 }
    }

    fn update(&mut self, provided_data: Option<&[u8]>) {
        let mut temp = [0u8; 48];
        for i in 0..3 {
            for j in (0..16).rev() {
                if self.v[j] == 0xff {
                    self.v[j] = 0x00;
                } else {
                    self.v[j] += 1;
                    break;
                }
            }
            aes256_ecb(&self.key, &self.v, &mut temp[16 * i..]);
        }
        if let Some(data) = provided_data {
            for i in 0..48 {
                temp[i] ^= data[i];
            }
        }
        self.key.copy_from_slice(&temp[..32]);
        self.v.copy_from_slice(&temp[32..48]);
    }
}

fn aes256_ecb(key: &[u8], ctr: &[u8], buffer: &mut [u8]) {
    let cipher = Cipher::aes_256_ecb();
    let mut c = Crypter::new(cipher, Mode::Encrypt, key, None).unwrap();
    c.pad(false);
    let mut out = [0u8; 32];
    let _ = c.update(ctr, &mut out).unwrap();
    buffer[..16].copy_from_slice(&out[..16]);
}

pub fn randombytes_init(entropy_input: &[u8], personalization_string: Option<&[u8]>) {
    let mut seed_material = [0u8; 48];
    seed_material.copy_from_slice(&entropy_input[..48]);
    if let Some(ps) = personalization_string {
        for i in 0..48 {
            seed_material[i] ^= ps[i];
        }
    }
    let mut ctx = DRBG_CTX.lock().unwrap();
    ctx.key = [0u8; 32];
    ctx.v = [0u8; 16];
    ctx.update(Some(&seed_material));
    ctx.reseed_counter = 1;
}

pub fn randombytes(x: &mut [u8], mut xlen: u64) {
    let mut block = [0u8; 16];
    let mut i: usize = 0;
    let mut ctx = DRBG_CTX.lock().unwrap();

    while xlen > 0 {
        for j in (0..16).rev() {
            if ctx.v[j] == 0xff {
                ctx.v[j] = 0x00;
            } else {
                ctx.v[j] += 1;
                break;
            }
        }
        aes256_ecb(&ctx.key, &ctx.v, &mut block);
        if xlen > 15 {
            x[i..i + 16].copy_from_slice(&block);
            i += 16;
            xlen -= 16;
        } else {
            x[i..i + xlen as usize].copy_from_slice(&block[..xlen as usize]);
            xlen = 0;
        }
    }
    ctx.update(None);
    ctx.reseed_counter += 1;
}
