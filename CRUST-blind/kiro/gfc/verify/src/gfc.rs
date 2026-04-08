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
    m: u64,
    r: u64,
    a: u64,
    b: u64,
    speck_exp: Vec<u32>,
}

impl GFC {
    pub fn new(range: u64, rounds: u64, seed: u64) -> Self {
        let tmp = (range as f64).sqrt().ceil() as u64;
        let a = tmp;
        let b = tmp;
        let mut speck_exp = vec![0u32; rounds as usize * SPECK_ROUNDS];
        for i in 0..rounds as usize {
            // Reinterpret two u64s as four u32s (little-endian)
            let key_u64 = [seed, i as u64];
            let key = [
                key_u64[0] as u32,
                (key_u64[0] >> 32) as u32,
                key_u64[1] as u32,
                (key_u64[1] >> 32) as u32,
            ];
            speck_expand(&key, &mut speck_exp[i * SPECK_ROUNDS..(i + 1) * SPECK_ROUNDS]);
        }
        GFC { m: range, r: rounds, a, b, speck_exp }
    }

    pub fn encrypt(&self, m: u64) -> u64 {
        let mut c = self.fe(m);
        while c >= self.m {
            c = self.fe(c);
        }
        c
    }

    pub fn decrypt(&self, m: u64) -> u64 {
        let mut c = self.fe_inv(m);
        while c >= self.m {
            c = self.fe_inv(c);
        }
        c
    }

    fn f(&self, j: u64, r: u64) -> u64 {
        let key = &self.speck_exp[(j as usize - 1) * SPECK_ROUNDS..j as usize * SPECK_ROUNDS];
        let pt = [r as u32, (r >> 32) as u32];
        let mut ct = [0u32; 2];
        speck_encrypt(&pt, &mut ct, key);
        ct[0] as u64 | ((ct[1] as u64) << 32)
    }

    fn fe(&self, m: u64) -> u64 {
        let mut l = m % self.a;
        let mut r = m / self.a;
        for j in 1..=self.r {
            let tmp = if j & 1 != 0 {
                (l.wrapping_add(self.f(j, r))) % self.a
            } else {
                (l.wrapping_add(self.f(j, r))) % self.b
            };
            l = r;
            r = tmp;
        }
        if self.r & 1 != 0 {
            self.a * l + r
        } else {
            self.a * r + l
        }
    }

    fn fe_inv(&self, m: u64) -> u64 {
        let (mut l, mut r) = if self.r & 1 != 0 {
            (m / self.a, m % self.a)
        } else {
            (m % self.a, m / self.a)
        };

        let mut j = self.r;
        while j >= 1 {
            let tmp = if j & 1 != 0 {
                (r.wrapping_add(self.a).wrapping_sub(self.f(j, l) % self.a)) % self.a
            } else {
                (r.wrapping_add(self.b).wrapping_sub(self.f(j, l) % self.b)) % self.b
            };
            r = l;
            l = tmp;
            j -= 1;
        }
        self.a * r + l
    }
}

fn speck_expand(key: &[u32; SPECK_KEY_LEN], round_keys: &mut [u32]) {
    let mut b = key[0];
    let mut a = [key[1], key[2], key[3]];
    round_keys[0] = b;
    for i in 0..(SPECK_ROUNDS - 1) as u32 {
        round(&mut a[i as usize % (SPECK_KEY_LEN - 1)], &mut b, i);
        round_keys[i as usize + 1] = b;
    }
}

fn speck_encrypt(pt: &[u32; 2], ct: &mut [u32; 2], key: &[u32]) {
    let mut a = pt[1];
    let mut b = pt[0];
    for i in 0..SPECK_ROUNDS {
        round(&mut a, &mut b, key[i]);
    }
    ct[0] = b;
    ct[1] = a;
}
