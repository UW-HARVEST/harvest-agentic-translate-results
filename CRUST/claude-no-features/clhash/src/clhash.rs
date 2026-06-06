pub const RANDOM_64BITWORDS_NEEDED_FOR_CLHASH: usize = 133;
pub const RANDOM_BYTES_NEEDED_FOR_CLHASH: usize = 133 * 8;

// ---------------------------------------------------------------------------
// Internal SIMD-like 128-bit value type and primitives.
// ---------------------------------------------------------------------------

#[derive(Copy, Clone, Default, Debug, PartialEq, Eq)]
struct M128i {
    lo: u64,
    hi: u64,
}

impl M128i {
    #[inline]
    fn zero() -> Self {
        Self { lo: 0, hi: 0 }
    }
    #[inline]
    fn xor(self, other: Self) -> Self {
        Self { lo: self.lo ^ other.lo, hi: self.hi ^ other.hi }
    }
    #[inline]
    fn and(self, other: Self) -> Self {
        Self { lo: self.lo & other.lo, hi: self.hi & other.hi }
    }
    // _mm_slli_epi64: shift each 64-bit lane left by x
    #[inline]
    fn slli_epi64(self, x: u32) -> Self {
        if x == 0 {
            self
        } else if x >= 64 {
            Self::zero()
        } else {
            Self { lo: self.lo << x, hi: self.hi << x }
        }
    }
    // _mm_srli_epi64
    #[inline]
    fn srli_epi64(self, x: u32) -> Self {
        if x == 0 {
            self
        } else if x >= 64 {
            Self::zero()
        } else {
            Self { lo: self.lo >> x, hi: self.hi >> x }
        }
    }
    // _mm_slli_si128: shift the entire 128-bit value left by n bytes
    #[inline]
    fn slli_si128(self, n: u32) -> Self {
        if n >= 16 {
            Self::zero()
        } else if n == 0 {
            self
        } else if n == 8 {
            Self { lo: 0, hi: self.lo }
        } else if n > 8 {
            Self { lo: 0, hi: self.lo << ((n - 8) * 8) }
        } else {
            let s = n * 8;
            Self {
                lo: self.lo << s,
                hi: (self.hi << s) | (self.lo >> (64 - s)),
            }
        }
    }
    // _mm_srli_si128
    #[inline]
    fn srli_si128(self, n: u32) -> Self {
        if n >= 16 {
            Self::zero()
        } else if n == 0 {
            self
        } else if n == 8 {
            Self { lo: self.hi, hi: 0 }
        } else if n > 8 {
            Self { lo: self.hi >> ((n - 8) * 8), hi: 0 }
        } else {
            let s = n * 8;
            Self {
                lo: (self.lo >> s) | (self.hi << (64 - s)),
                hi: self.hi >> s,
            }
        }
    }
}

// Read a u64 little-endian from a byte slice at given offset, padding with zeros
// if the slice is shorter than offset+8.
#[inline]
fn read_u64_le(buf: &[u8], offset: usize) -> u64 {
    let mut tmp = [0u8; 8];
    if offset < buf.len() {
        let end = (offset + 8).min(buf.len());
        let len = end - offset;
        tmp[..len].copy_from_slice(&buf[offset..end]);
    }
    u64::from_le_bytes(tmp)
}

#[inline]
fn read_m128i(buf: &[u8], offset: usize) -> M128i {
    M128i {
        lo: read_u64_le(buf, offset),
        hi: read_u64_le(buf, offset + 8),
    }
}

// Carry-less multiplication of two 64-bit values, producing a 128-bit result.
#[inline]
fn clmul64(a: u64, b: u64) -> (u64, u64) {
    // Returns (lo, hi).
    let mut lo: u64 = 0;
    let mut hi: u64 = 0;
    let mut bb = b;
    let mut i: u32 = 0;
    while bb != 0 {
        if bb & 1 == 1 {
            lo ^= a << i;
            if i > 0 {
                hi ^= a >> (64 - i);
            }
        }
        bb >>= 1;
        i += 1;
    }
    (lo, hi)
}

// _mm_clmulepi64_si128: imm8 selects which 64-bit half of each operand.
//   bit 0 selects A's half (0=lo, 1=hi)
//   bit 4 selects B's half (0=lo, 1=hi)
#[inline]
fn clmulepi64(a: M128i, b: M128i, imm8: u8) -> M128i {
    let aval = if imm8 & 0x01 != 0 { a.hi } else { a.lo };
    let bval = if imm8 & 0x10 != 0 { b.hi } else { b.lo };
    let (lo, hi) = clmul64(aval, bval);
    M128i { lo, hi }
}

// _mm_shuffle_epi8(table, control)
#[inline]
fn shuffle_epi8(table: M128i, control: M128i) -> M128i {
    let table_u128: u128 = ((table.hi as u128) << 64) | (table.lo as u128);
    let table_b: [u8; 16] = table_u128.to_le_bytes();
    let ctrl_u128: u128 = ((control.hi as u128) << 64) | (control.lo as u128);
    let ctrl_b: [u8; 16] = ctrl_u128.to_le_bytes();
    let mut result = [0u8; 16];
    for i in 0..16 {
        let c = ctrl_b[i];
        if c & 0x80 != 0 {
            result[i] = 0;
        } else {
            result[i] = table_b[(c & 0x0f) as usize];
        }
    }
    let result_u128 = u128::from_le_bytes(result);
    M128i { lo: result_u128 as u64, hi: (result_u128 >> 64) as u64 }
}

// computes a << 1 (over 128 bits)
#[inline]
fn leftshift1(a: M128i) -> M128i {
    let u64shift = a.slli_epi64(1);
    let topbits = a.srli_epi64(63).slli_si128(8);
    u64shift.xor(topbits)
}

// computes a << 2 (over 128 bits)
#[inline]
fn leftshift2(a: M128i) -> M128i {
    let u64shift = a.slli_epi64(2);
    let topbits = a.srli_epi64(62).slli_si128(8);
    u64shift.xor(topbits)
}

#[inline]
fn lazymod127(alow: M128i, ahigh: M128i) -> M128i {
    let s1 = leftshift1(ahigh);
    let s2 = leftshift2(ahigh);
    alow.xor(s1).xor(s2)
}

#[inline]
fn mul128by128_lazymod127(a: M128i, b: M128i) -> M128i {
    let amix1 = clmulepi64(a, b, 0x01);
    let amix2 = clmulepi64(a, b, 0x10);
    let alow = clmulepi64(a, b, 0x00);
    let ahigh = clmulepi64(a, b, 0x11);
    let amix = amix1.xor(amix2);
    let amix1s = amix.slli_si128(8);
    let amix2s = amix.srli_si128(8);
    let alow = alow.xor(amix1s);
    let ahigh = ahigh.xor(amix2s);
    lazymod127(alow, ahigh)
}

#[inline]
fn lazy_length_hash(keylength: u64, length: u64) -> M128i {
    // _mm_set_epi64x(keylength, length): low=length, high=keylength
    let v = M128i { lo: length, hi: keylength };
    // _mm_clmulepi64_si128(v, v, 0x10) -> v.lo (B's lo with imm8 bit4=1 means B.hi) — wait
    // imm8=0x10: bit0=0 -> A.lo; bit4=1 -> B.hi. Both are v. So v.lo * v.hi = length * keylength.
    clmulepi64(v, v, 0x10)
}

#[inline]
fn precomp_reduction64_si128(a: M128i) -> M128i {
    // C = irreducible polynomial constant: low = (1<<4)+(1<<3)+(1<<1)+(1<<0) = 27, high = 0
    let c = M128i { lo: 27, hi: 0 };
    // Q2 = clmul(A, C, 0x01) = A.hi * C.lo
    let q2 = clmulepi64(a, c, 0x01);
    let lut = M128i {
        lo: u64::from_le_bytes([0, 27, 54, 45, 108, 119, 90, 65]),
        hi: u64::from_le_bytes([216, 195, 238, 245, 180, 175, 130, 153]),
    };
    let q3 = shuffle_epi8(lut, q2.srli_si128(8));
    let q4 = q2.xor(a);
    q3.xor(q4)
}

#[inline]
fn precomp_reduction64(a: M128i) -> u64 {
    precomp_reduction64_si128(a).lo
}

#[inline]
fn simple128to64hashwithlength(value: M128i, key: M128i, keylength: u64, length: u64) -> u64 {
    let add = value.xor(key);
    let clprod1 = clmulepi64(add, add, 0x10);
    let total = clprod1.xor(lazy_length_hash(keylength, length));
    precomp_reduction64(total)
}

// For use with CLHASH. We expect length_words to be divisible by 4 (length=128 in calls).
fn clmul_half_scalar_product_without_reduction(
    random: &[u8],
    string: &[u8],
    length_words: usize,
) -> M128i {
    let total_bytes = length_words * 8;
    let mut acc = M128i::zero();
    let mut s = 0usize;
    let mut r = 0usize;
    while s + 32 <= total_bytes {
        let temp1 = read_m128i(random, r);
        let temp2 = read_m128i(string, s);
        let add1 = temp1.xor(temp2);
        let clprod1 = clmulepi64(add1, add1, 0x10);
        acc = clprod1.xor(acc);

        let temp12 = read_m128i(random, r + 16);
        let temp22 = read_m128i(string, s + 16);
        let add12 = temp12.xor(temp22);
        let clprod12 = clmulepi64(add12, add12, 0x10);
        acc = clprod12.xor(acc);

        s += 32;
        r += 32;
    }
    acc
}

fn clmul_half_scalar_product_with_tail_without_reduction(
    random: &[u8],
    string: &[u8],
    length_words: usize,
) -> M128i {
    let total_bytes = length_words * 8;
    let mut acc = M128i::zero();
    let mut s = 0usize;
    let mut r = 0usize;
    while s + 32 <= total_bytes {
        let temp1 = read_m128i(random, r);
        let temp2 = read_m128i(string, s);
        let add1 = temp1.xor(temp2);
        let clprod1 = clmulepi64(add1, add1, 0x10);
        acc = clprod1.xor(acc);

        let temp12 = read_m128i(random, r + 16);
        let temp22 = read_m128i(string, s + 16);
        let add12 = temp12.xor(temp22);
        let clprod12 = clmulepi64(add12, add12, 0x10);
        acc = clprod12.xor(acc);

        s += 32;
        r += 32;
    }
    if s + 16 <= total_bytes {
        let temp1 = read_m128i(random, r);
        let temp2 = read_m128i(string, s);
        let add1 = temp1.xor(temp2);
        let clprod1 = clmulepi64(add1, add1, 0x10);
        acc = clprod1.xor(acc);
        s += 16;
        r += 16;
    }
    if s < total_bytes {
        let temp1 = read_m128i(random, r);
        // _mm_loadl_epi64: low 64 bits from string, high = 0
        let temp2 = M128i { lo: read_u64_le(string, s), hi: 0 };
        let add1 = temp1.xor(temp2);
        let clprod1 = clmulepi64(add1, add1, 0x10);
        acc = clprod1.xor(acc);
    }
    acc
}

fn clmul_half_scalar_product_with_tail_without_reduction_with_extra_word(
    random: &[u8],
    string: &[u8],
    length_words: usize,
    extraword: u64,
) -> M128i {
    let total_bytes = length_words * 8;
    let mut acc = M128i::zero();
    let mut s = 0usize;
    let mut r = 0usize;
    while s + 32 <= total_bytes {
        let temp1 = read_m128i(random, r);
        let temp2 = read_m128i(string, s);
        let add1 = temp1.xor(temp2);
        let clprod1 = clmulepi64(add1, add1, 0x10);
        acc = clprod1.xor(acc);

        let temp12 = read_m128i(random, r + 16);
        let temp22 = read_m128i(string, s + 16);
        let add12 = temp12.xor(temp22);
        let clprod12 = clmulepi64(add12, add12, 0x10);
        acc = clprod12.xor(acc);

        s += 32;
        r += 32;
    }
    if s + 16 <= total_bytes {
        let temp1 = read_m128i(random, r);
        let temp2 = read_m128i(string, s);
        let add1 = temp1.xor(temp2);
        let clprod1 = clmulepi64(add1, add1, 0x10);
        acc = clprod1.xor(acc);
        s += 16;
        r += 16;
    }
    if s < total_bytes {
        // We have 1 u64 left; append extraword as the high 64 bits.
        let temp1 = read_m128i(random, r);
        let temp2 = M128i { lo: read_u64_le(string, s), hi: extraword };
        let add1 = temp1.xor(temp2);
        let clprod1 = clmulepi64(add1, add1, 0x10);
        acc = clprod1.xor(acc);
    } else {
        let temp1 = read_m128i(random, r);
        let temp2 = M128i { lo: extraword, hi: 0 };
        let add1 = temp1.xor(temp2);
        // imm8 = 0x01: A.hi * B.lo
        let clprod1 = clmulepi64(add1, add1, 0x01);
        acc = clprod1.xor(acc);
    }
    acc
}

fn clmul_half_scalar_product_only_extra_word(random: &[u8], extraword: u64) -> M128i {
    let temp1 = read_m128i(random, 0);
    let temp2 = M128i { lo: extraword, hi: 0 };
    let add1 = temp1.xor(temp2);
    clmulepi64(add1, add1, 0x01)
}

// There always remains an incomplete word that has 1..7 used bytes; pad with zeros.
fn create_last_word(stringbyte: &[u8]) -> u64 {
    let lengthbyte = stringbyte.len();
    let significant = lengthbyte % 8;
    let length = lengthbyte / 8;
    let mut tmp = [0u8; 8];
    let start = length * 8;
    if significant > 0 {
        tmp[..significant].copy_from_slice(&stringbyte[start..start + significant]);
    }
    u64::from_le_bytes(tmp)
}

pub fn clhash(random: &[u8], stringbyte: &[u8]) -> u64 {
    // If the random buffer is shorter than the required key, expand it.
    // Treat the first 16 bytes as two u64 seeds for xorshift128+.
    if random.len() < RANDOM_BYTES_NEEDED_FOR_CLHASH {
        let mut seed_bytes = [0u8; 16];
        let copy_len = random.len().min(16);
        seed_bytes[..copy_len].copy_from_slice(&random[..copy_len]);
        let seed1 = u64::from_le_bytes(seed_bytes[0..8].try_into().unwrap());
        let seed2 = u64::from_le_bytes(seed_bytes[8..16].try_into().unwrap());
        let expanded = get_random_key_for_clhash(seed1, seed2);
        return clhash_inner(&expanded, stringbyte);
    }
    clhash_inner(random, stringbyte)
}

fn clhash_inner(random: &[u8], stringbyte: &[u8]) -> u64 {
    const M: usize = 128; // u64 words per block
    let m128_per_block = M / 2; // 64 m128i per block
    let lengthbyte = stringbyte.len();
    let length = lengthbyte / 8; // # of complete words
    let lengthinc = (lengthbyte + 7) / 8; // # words including any partial

    let polyvalue_offset = m128_per_block * 16; // 1024 bytes
    let mut polyvalue = read_m128i(random, polyvalue_offset);
    // Mask: low 64 = 0xFFFFFFFFFFFFFFFF, high 64 = 0x3FFFFFFFFFFFFFFF
    polyvalue = polyvalue.and(M128i {
        lo: 0xFFFF_FFFF_FFFF_FFFF,
        hi: 0x3FFF_FFFF_FFFF_FFFF,
    });

    if M < lengthinc {
        // Long strings.
        let mut acc = clmul_half_scalar_product_without_reduction(random, stringbyte, M);
        let mut t = M;
        while t + M <= length {
            acc = mul128by128_lazymod127(polyvalue, acc);
            let h1 = clmul_half_scalar_product_without_reduction(
                random,
                &stringbyte[t * 8..],
                M,
            );
            acc = acc.xor(h1);
            t += M;
        }
        let remain = length - t;

        if remain != 0 {
            acc = mul128by128_lazymod127(polyvalue, acc);
            if lengthbyte % 8 == 0 {
                let h1 = clmul_half_scalar_product_with_tail_without_reduction(
                    random,
                    &stringbyte[t * 8..],
                    remain,
                );
                acc = acc.xor(h1);
            } else {
                let lastword = create_last_word(stringbyte);
                let h1 = clmul_half_scalar_product_with_tail_without_reduction_with_extra_word(
                    random,
                    &stringbyte[t * 8..],
                    remain,
                    lastword,
                );
                acc = acc.xor(h1);
            }
        } else if lengthbyte % 8 != 0 {
            acc = mul128by128_lazymod127(polyvalue, acc);
            let lastword = create_last_word(stringbyte);
            let h1 = clmul_half_scalar_product_only_extra_word(random, lastword);
            acc = acc.xor(h1);
        }

        let finalkey = read_m128i(random, polyvalue_offset + 16);
        let keylength = read_u64_le(random, polyvalue_offset + 32);
        simple128to64hashwithlength(acc, finalkey, keylength, lengthbyte as u64)
    } else {
        // Short strings.
        if lengthbyte % 8 == 0 {
            let mut acc = clmul_half_scalar_product_with_tail_without_reduction(
                random, stringbyte, length,
            );
            let keylength = read_u64_le(random, polyvalue_offset + 32);
            acc = acc.xor(lazy_length_hash(keylength, lengthbyte as u64));
            return precomp_reduction64(acc);
        }
        let lastword = create_last_word(stringbyte);
        let mut acc = clmul_half_scalar_product_with_tail_without_reduction_with_extra_word(
            random, stringbyte, length, lastword,
        );
        let keylength = read_u64_le(random, polyvalue_offset + 32);
        acc = acc.xor(lazy_length_hash(keylength, lengthbyte as u64));
        precomp_reduction64(acc)
    }
}

// ---------------------------------------------------------------------------
// Random key generation: xorshift128+ PRNG.
// ---------------------------------------------------------------------------

struct Xorshift128PlusKey {
    part1: u64,
    part2: u64,
}

impl Xorshift128PlusKey {
    fn new(key1: u64, key2: u64) -> Self {
        Self { part1: key1, part2: key2 }
    }
    fn next(&mut self) -> u64 {
        let mut s1 = self.part1;
        let s0 = self.part2;
        self.part1 = s0;
        s1 ^= s1 << 23;
        self.part2 = s1 ^ s0 ^ (s1 >> 18) ^ (s0 >> 5);
        self.part2.wrapping_add(s0)
    }
}

pub fn get_random_key_for_clhash(seed1: u64, seed2: u64) -> Vec<u8> {
    let mut k = Xorshift128PlusKey::new(seed1, seed2);
    let mut a64 = vec![0u64; RANDOM_64BITWORDS_NEEDED_FOR_CLHASH];
    for i in 0..RANDOM_64BITWORDS_NEEDED_FOR_CLHASH {
        a64[i] = k.next();
    }
    while a64[128] == 0 && a64[129] == 1 {
        a64[128] = k.next();
        a64[129] = k.next();
    }
    let mut bytes = vec![0u8; RANDOM_BYTES_NEEDED_FOR_CLHASH];
    for i in 0..RANDOM_64BITWORDS_NEEDED_FOR_CLHASH {
        bytes[i * 8..i * 8 + 8].copy_from_slice(&a64[i].to_le_bytes());
    }
    bytes
}

pub struct ClHasher {
    random_data: Vec<u8>,
}

impl ClHasher {
    pub fn new(seed1: u64, seed2: u64) -> Self {
        Self { random_data: get_random_key_for_clhash(seed1, seed2) }
    }
    pub fn hash<T: AsRef<[u8]>>(&self, data: T) -> u64 {
        clhash(&self.random_data, data.as_ref())
    }
}

impl Drop for ClHasher {
    fn drop(&mut self) {
        // Vec drop frees memory automatically; nothing to do.
    }
}
