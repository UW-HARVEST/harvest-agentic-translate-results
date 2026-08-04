/// Constants for Speck encryption
const SPECK_ROUNDS: usize = 27;
const SPECK_KEY_LEN: usize = 4;
fn ror(x: u32, r: u32) -> u32 {
    x.rotate_right(r)
}
fn rol(x: u32, r: u32) -> u32 {
    x.rotate_left(r)
}
fn round(x: &mut u32, y: &mut u32, k: u32) {
    *x = ror(*x, 8);
    *x = x.wrapping_add(*y);
    *x ^= k;
    *y = rol(*y, 3);
    *y ^= *x;
}
#[derive(Debug)]
pub struct GFC {
    m: u64,       // Range
    r: u64,       // Number of Feistel rounds
    a: u64,
    b: u64,
    speck_exp: Vec<u32>,
}
impl GFC {
    /// Initialize the GFC structure
    pub fn new(range: u64, rounds: u64, seed: u64) -> Self {
        let tmp = (range as f64).sqrt().ceil() as u64;
        let mut speck_exp = vec![0; (rounds as usize) * SPECK_ROUNDS];

        for i in 0..rounds {
            let key = u64x2_to_u32x4_le(seed, i);
            let start = (i as usize) * SPECK_ROUNDS;
            speck_expand(&key, &mut speck_exp[start..start + SPECK_ROUNDS]);
        }

        Self {
            m: range,
            r: rounds,
            a: tmp,
            b: tmp,
            speck_exp,
        }
    }
    /// Encrypt using the Feistel cipher
    pub fn encrypt(&self, m: u64) -> u64 {
        let mut c = self.fe(m);
        while c >= self.m {
            c = self.fe(c);
        }
        c
    }
    /// Decrypt using the Feistel cipher
    pub fn decrypt(&self, m: u64) -> u64 {
        let mut c = self.fe_inv(m);
        while c >= self.m {
            c = self.fe_inv(c);
        }
        c
    }
    /// Function F for Feistel cipher using Speck encryption
    fn f(&self, j: u64, r: u64) -> u64 {
        let start = ((j - 1) as usize) * SPECK_ROUNDS;
        let key = &self.speck_exp[start..start + SPECK_ROUNDS];
        let pt = u64_to_u32x2_le(r);
        let mut ct = [0_u32; 2];
        speck_encrypt(&pt, &mut ct, key);
        u32x2_to_u64_le(ct)
    }
    /// Function fe[r, a, b] for encryption
    fn fe(&self, m: u64) -> u64 {
        let mut l = m % self.a;
        let mut r = m / self.a;

        for j in 1..=self.r {
            let modulus = if j & 1 == 1 { self.a } else { self.b };
            let tmp = l.wrapping_add(self.f(j, r)) % modulus;
            l = r;
            r = tmp;
        }

        if self.r & 1 == 1 {
            self.a.wrapping_mul(l).wrapping_add(r)
        } else {
            self.a.wrapping_mul(r).wrapping_add(l)
        }
    }
    /// Function fe^-1[r, a, b] for decryption
    fn fe_inv(&self, m: u64) -> u64 {
        let (mut l, mut r) = if self.r & 1 == 1 {
            (m / self.a, m % self.a)
        } else {
            (m % self.a, m / self.a)
        };

        let mut j = self.r;
        while j >= 1 {
            let modulus = if j & 1 == 1 { self.a } else { self.b };
            let tmp = r
                .wrapping_add(modulus)
                .wrapping_sub(self.f(j, l) % modulus)
                % modulus;
            r = l;
            l = tmp;
            j -= 1;
        }

        self.a.wrapping_mul(r).wrapping_add(l)
    }
}
/// Speck key expansion
fn speck_expand(key: &[u32; SPECK_KEY_LEN], round_keys: &mut [u32]) {
    let mut b = key[0];
    let mut a = [0_u32; SPECK_KEY_LEN - 1];
    a.copy_from_slice(&key[1..]);

    round_keys[0] = b;
    for i in 0..(SPECK_ROUNDS - 1) {
        let idx = i % (SPECK_KEY_LEN - 1);
        round(&mut a[idx], &mut b, i as u32);
        round_keys[i + 1] = b;
    }
}
/// Speck encryption function
fn speck_encrypt(pt: &[u32; 2], ct: &mut [u32; 2], key: &[u32]) {
    let mut x = pt[1];
    let mut y = pt[0];

    for &round_key in key.iter().take(SPECK_ROUNDS) {
        round(&mut x, &mut y, round_key);
    }

    ct[0] = y;
    ct[1] = x;
}

fn u64_to_u32x2_le(value: u64) -> [u32; 2] {
    [value as u32, (value >> 32) as u32]
}

fn u32x2_to_u64_le(words: [u32; 2]) -> u64 {
    u64::from(words[0]) | (u64::from(words[1]) << 32)
}

fn u64x2_to_u32x4_le(first: u64, second: u64) -> [u32; 4] {
    [
        first as u32,
        (first >> 32) as u32,
        second as u32,
        (second >> 32) as u32,
    ]
}
