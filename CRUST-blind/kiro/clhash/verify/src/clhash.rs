pub const RANDOM_64BITWORDS_NEEDED_FOR_CLHASH: usize = 133;
pub const RANDOM_BYTES_NEEDED_FOR_CLHASH: usize = 133 * 8;

// --- 128-bit helpers (replacing __m128i) ---

type M128 = [u64; 2]; // [lo, hi]

fn m128_zero() -> M128 { [0, 0] }
fn m128_xor(a: M128, b: M128) -> M128 { [a[0] ^ b[0], a[1] ^ b[1]] }

// Carry-less (polynomial) multiplication of two 64-bit values -> 128-bit result
fn clmul64(a: u64, b: u64) -> M128 {
    let mut lo: u64 = 0;
    let mut hi: u64 = 0;
    for i in 0..64u32 {
        if (a >> i) & 1 != 0 {
            if i == 0 {
                lo ^= b;
            } else {
                lo ^= b << i;
                hi ^= b >> (64 - i);
            }
        }
    }
    [lo, hi]
}

// _mm_clmulepi64_si128(a, b, imm8) equivalent
// imm8: 0x00 = a.lo * b.lo, 0x01 = a.hi * b.lo, 0x10 = a.lo * b.hi, 0x11 = a.hi * b.hi
fn clmulepi64(a: M128, b: M128, imm8: u8) -> M128 {
    let a_val = if imm8 & 0x01 != 0 { a[1] } else { a[0] };
    let b_val = if imm8 & 0x10 != 0 { b[1] } else { b[0] };
    clmul64(a_val, b_val)
}

// a << 1 across 128 bits
fn leftshift1(a: M128) -> M128 {
    let lo = a[0] << 1;
    let hi = (a[1] << 1) | (a[0] >> 63);
    [lo, hi]
}

// a << 2 across 128 bits
fn leftshift2(a: M128) -> M128 {
    let lo = a[0] << 2;
    let hi = (a[1] << 2) | (a[0] >> 62);
    [lo, hi]
}

// lazy modulo with 2^127 + 2 + 1 (actually mod 2*(2^127+2+1))
fn lazymod127(a_low: M128, a_high: M128) -> M128 {
    let shift1 = leftshift1(a_high);
    let shift2 = leftshift2(a_high);
    m128_xor(m128_xor(a_low, shift1), shift2)
}

// multiplication with lazy reduction mod 2^127+2+1
fn mul128by128to128_lazymod127(a: M128, b: M128) -> M128 {
    let amix1 = clmulepi64(a, b, 0x01);
    let amix2 = clmulepi64(a, b, 0x10);
    let a_low = clmulepi64(a, b, 0x00);
    let a_high = clmulepi64(a, b, 0x11);
    let amix = m128_xor(amix1, amix2);
    // amix1 = amix << 64 (slli_si128 by 8 bytes)
    let amix_lo = [0, amix[0]];
    // amix2 = amix >> 64 (srli_si128 by 8 bytes)
    let amix_hi = [amix[1], 0];
    let a_low = m128_xor(a_low, amix_lo);
    let a_high = m128_xor(a_high, amix_hi);
    lazymod127(a_low, a_high)
}

// multiply length and key, no modulo
fn lazy_length_hash(keylength: u64, length: u64) -> M128 {
    // _mm_set_epi64x(keylength, length) -> [length, keylength]
    // _mm_clmulepi64_si128(v, v, 0x10) -> v.lo * v.hi = length * keylength
    clmul64(length, keylength)
}

// Reduction to 64-bit using the irreducible polynomial (64,4,3,1,0)
fn precomp_reduction64(a: M128) -> u64 {
    // C = (1<<4)+(1<<3)+(1<<1)+(1<<0) = 27, stored as [27, 0]
    let c: M128 = [27, 0];
    // Q2 = clmulepi64(A, C, 0x01) means A[1] * C[0] = a[1] * 27
    let q2 = clmulepi64(a, c, 0x01);

    // Q3 = shuffle table lookup on high 64 bits of Q2
    // The table maps each nibble through a specific lookup
    let table: [u8; 16] = [0, 27, 54, 45, 108, 119, 90, 65, 216, 195, 238, 245, 180, 175, 130, 153];
    // _mm_srli_si128(Q2, 8) -> [q2[1], 0]
    // _mm_shuffle_epi8 with the table: each byte of the index selects from the table
    let idx_bytes = q2[1].to_le_bytes();
    let mut q3_bytes = [0u8; 16];
    for i in 0..8 {
        let idx = idx_bytes[i];
        if idx & 0x80 != 0 {
            q3_bytes[i] = 0;
        } else {
            q3_bytes[i] = table[(idx & 0x0F) as usize];
        }
    }
    // high 8 bytes are 0 (from the zero-extended index)
    let q3_lo = u64::from_le_bytes(q3_bytes[0..8].try_into().unwrap());
    let q3: M128 = [q3_lo, 0];

    // Q4 = Q2 XOR A
    let q4 = m128_xor(q2, a);
    let result = m128_xor(q3, q4);
    result[0] // low 64 bits only; high 64 bits are garbage
}

// (a xor k1) * (b xor k2) mod p with length component
fn simple128to64hashwithlength(value: M128, key: M128, keylength: u64, length: u64) -> u64 {
    let add = m128_xor(value, key);
    let clprod1 = clmulepi64(add, add, 0x10);
    let total = m128_xor(clprod1, lazy_length_hash(keylength, length));
    precomp_reduction64(total)
}

// Read an M128 from random source (pair of u64s at m128 index)
fn read_m128(random: &[u64], m128_idx: usize) -> M128 {
    [random[m128_idx * 2], random[m128_idx * 2 + 1]]
}

// Read an M128 from string data (pair of u64s)
fn read_string_m128(string: &[u64], offset: usize) -> M128 {
    [string[offset], string[offset + 1]]
}

fn clmulhalfscalarproductwithoutreduction(rs: &[u64], string: &[u64], length: usize) -> M128 {
    let mut acc = m128_zero();
    let mut rs_idx: usize = 0; // m128 index into random source
    let mut s_idx: usize = 0;
    while s_idx + 3 < length {
        let temp1 = read_m128(rs, rs_idx);
        let temp2 = read_string_m128(string, s_idx);
        let add1 = m128_xor(temp1, temp2);
        let clprod1 = clmulepi64(add1, add1, 0x10);
        acc = m128_xor(clprod1, acc);

        let temp12 = read_m128(rs, rs_idx + 1);
        let temp22 = read_string_m128(string, s_idx + 2);
        let add12 = m128_xor(temp12, temp22);
        let clprod12 = clmulepi64(add12, add12, 0x10);
        acc = m128_xor(clprod12, acc);

        rs_idx += 2;
        s_idx += 4;
    }
    acc
}

fn clmulhalfscalarproductwithtailwithoutreduction(rs: &[u64], string: &[u64], length: usize) -> M128 {
    let mut acc = m128_zero();
    let mut rs_idx: usize = 0;
    let mut s_idx: usize = 0;
    while s_idx + 3 < length {
        let temp1 = read_m128(rs, rs_idx);
        let temp2 = read_string_m128(string, s_idx);
        let add1 = m128_xor(temp1, temp2);
        let clprod1 = clmulepi64(add1, add1, 0x10);
        acc = m128_xor(clprod1, acc);

        let temp12 = read_m128(rs, rs_idx + 1);
        let temp22 = read_string_m128(string, s_idx + 2);
        let add12 = m128_xor(temp12, temp22);
        let clprod12 = clmulepi64(add12, add12, 0x10);
        acc = m128_xor(clprod12, acc);

        rs_idx += 2;
        s_idx += 4;
    }
    if s_idx + 1 < length {
        let temp1 = read_m128(rs, rs_idx);
        let temp2 = read_string_m128(string, s_idx);
        let add1 = m128_xor(temp1, temp2);
        let clprod1 = clmulepi64(add1, add1, 0x10);
        acc = m128_xor(clprod1, acc);
        rs_idx += 1;
        s_idx += 2;
    }
    if s_idx < length {
        let temp1 = read_m128(rs, rs_idx);
        let temp2: M128 = [string[s_idx], 0]; // loadl_epi64
        let add1 = m128_xor(temp1, temp2);
        let clprod1 = clmulepi64(add1, add1, 0x10);
        acc = m128_xor(clprod1, acc);
    }
    acc
}

fn clmulhalfscalarproductwithtailwithoutreduction_with_extra_word(
    rs: &[u64], string: &[u64], length: usize, extraword: u64,
) -> M128 {
    let mut acc = m128_zero();
    let mut rs_idx: usize = 0;
    let mut s_idx: usize = 0;
    while s_idx + 3 < length {
        let temp1 = read_m128(rs, rs_idx);
        let temp2 = read_string_m128(string, s_idx);
        let add1 = m128_xor(temp1, temp2);
        let clprod1 = clmulepi64(add1, add1, 0x10);
        acc = m128_xor(clprod1, acc);

        let temp12 = read_m128(rs, rs_idx + 1);
        let temp22 = read_string_m128(string, s_idx + 2);
        let add12 = m128_xor(temp12, temp22);
        let clprod12 = clmulepi64(add12, add12, 0x10);
        acc = m128_xor(clprod12, acc);

        rs_idx += 2;
        s_idx += 4;
    }
    if s_idx + 1 < length {
        let temp1 = read_m128(rs, rs_idx);
        let temp2 = read_string_m128(string, s_idx);
        let add1 = m128_xor(temp1, temp2);
        let clprod1 = clmulepi64(add1, add1, 0x10);
        acc = m128_xor(clprod1, acc);
        rs_idx += 1;
        s_idx += 2;
    }
    if s_idx < length {
        // _mm_set_epi64x(extraword, *string) -> [*string, extraword]
        let temp1 = read_m128(rs, rs_idx);
        let temp2: M128 = [string[s_idx], extraword];
        let add1 = m128_xor(temp1, temp2);
        let clprod1 = clmulepi64(add1, add1, 0x10);
        acc = m128_xor(clprod1, acc);
    } else {
        // _mm_loadl_epi64(&extraword) -> [extraword, 0]
        let temp1 = read_m128(rs, rs_idx);
        let temp2: M128 = [extraword, 0];
        let add1 = m128_xor(temp1, temp2);
        // Note: 0x01 here, not 0x10!
        let clprod1 = clmulepi64(add1, add1, 0x01);
        acc = m128_xor(clprod1, acc);
    }
    acc
}

fn clmulhalfscalarproduct_only_extra_word(rs: &[u64], extraword: u64) -> M128 {
    let temp1: M128 = [rs[0], rs[1]];
    let temp2: M128 = [extraword, 0];
    let add1 = m128_xor(temp1, temp2);
    clmulepi64(add1, add1, 0x01)
}

fn create_last_word(lengthbyte: usize, data: &[u8]) -> u64 {
    let significant = lengthbyte % 8;
    let mut lastword = 0u64;
    let bytes = lastword.to_le_bytes();
    let mut buf = bytes;
    for i in 0..significant {
        buf[i] = data[i];
    }
    lastword = u64::from_le_bytes(buf);
    lastword
}

// Convert byte slice to u64 slice (little-endian)
fn bytes_to_u64s(bytes: &[u8]) -> Vec<u64> {
    let count = (bytes.len() + 7) / 8;
    let mut result = Vec::with_capacity(count);
    for i in 0..count {
        let start = i * 8;
        let end = (start + 8).min(bytes.len());
        let mut buf = [0u8; 8];
        buf[..end - start].copy_from_slice(&bytes[start..end]);
        result.push(u64::from_le_bytes(buf));
    }
    result
}

pub fn clhash(random: &[u8], stringbyte: &[u8]) -> u64 {
    let lengthbyte = stringbyte.len();
    let m: usize = 128; // process data in chunks of 128 u64s
    let m128neededperblock = m / 2; // 64 m128 pairs

    // Convert random bytes to u64 slice
    let rs64: Vec<u64> = bytes_to_u64s(random);

    // polyvalue = rs64 at m128 index m128neededperblock, masked
    let mut polyvalue = read_m128(&rs64, m128neededperblock);
    polyvalue[1] &= 0x3fffffff_ffffffff; // clear two highest bits

    let length = lengthbyte / 8; // complete u64 words
    let lengthinc = (lengthbyte + 7) / 8; // including partial

    // Convert string to u64s
    let string = bytes_to_u64s(stringbyte);

    if m < lengthinc {
        // long strings
        let mut acc = clmulhalfscalarproductwithoutreduction(&rs64, &string, m);
        let mut t = m;
        while t + m <= length {
            acc = mul128by128to128_lazymod127(polyvalue, acc);
            let h1 = clmulhalfscalarproductwithoutreduction(&rs64, &string[t..], m);
            acc = m128_xor(acc, h1);
            t += m;
        }
        let remain = length - t;

        if remain != 0 {
            acc = mul128by128to128_lazymod127(polyvalue, acc);
            if lengthbyte % 8 == 0 {
                let h1 = clmulhalfscalarproductwithtailwithoutreduction(&rs64, &string[t..], remain);
                acc = m128_xor(acc, h1);
            } else {
                let last_word_offset = length * 8; // byte offset of partial word
                let lastword = create_last_word(lengthbyte, &stringbyte[last_word_offset..]);
                let h1 = clmulhalfscalarproductwithtailwithoutreduction_with_extra_word(
                    &rs64, &string[t..], remain, lastword,
                );
                acc = m128_xor(acc, h1);
            }
        } else if lengthbyte % 8 != 0 {
            acc = mul128by128to128_lazymod127(polyvalue, acc);
            let last_word_offset = length * 8;
            let lastword = create_last_word(lengthbyte, &stringbyte[last_word_offset..]);
            let h1 = clmulhalfscalarproduct_only_extra_word(&rs64, lastword);
            acc = m128_xor(acc, h1);
        }

        let finalkey = read_m128(&rs64, m128neededperblock + 1);
        let keylength = rs64[(m128neededperblock + 2) * 2];
        simple128to64hashwithlength(acc, finalkey, keylength, lengthbyte as u64)
    } else {
        // short strings
        if lengthbyte % 8 == 0 {
            let mut acc = clmulhalfscalarproductwithtailwithoutreduction(&rs64, &string, length);
            let keylength = rs64[(m128neededperblock + 2) * 2];
            acc = m128_xor(acc, lazy_length_hash(keylength, lengthbyte as u64));
            precomp_reduction64(acc)
        } else {
            let last_word_offset = length * 8;
            let lastword = create_last_word(lengthbyte, &stringbyte[last_word_offset..]);
            let mut acc = clmulhalfscalarproductwithtailwithoutreduction_with_extra_word(
                &rs64, &string, length, lastword,
            );
            let keylength = rs64[(m128neededperblock + 2) * 2];
            acc = m128_xor(acc, lazy_length_hash(keylength, lengthbyte as u64));
            precomp_reduction64(acc)
        }
    }
}

// xorshift128plus PRNG
fn xorshift128plus(part1: &mut u64, part2: &mut u64) -> u64 {
    let mut s1 = *part1;
    let s0 = *part2;
    *part1 = s0;
    s1 ^= s1 << 23;
    *part2 = s1 ^ s0 ^ (s1 >> 18) ^ (s0 >> 5);
    part2.wrapping_add(s0)
}

pub fn get_random_key_for_clhash(seed1: u64, seed2: u64) -> Vec<u8> {
    let mut part1 = seed1;
    let mut part2 = seed2;
    let mut a64 = vec![0u64; RANDOM_64BITWORDS_NEEDED_FOR_CLHASH];
    for i in 0..RANDOM_64BITWORDS_NEEDED_FOR_CLHASH {
        a64[i] = xorshift128plus(&mut part1, &mut part2);
    }
    while a64[128] == 0 && a64[129] == 1 {
        a64[128] = xorshift128plus(&mut part1, &mut part2);
        a64[129] = xorshift128plus(&mut part1, &mut part2);
    }
    // Convert to bytes (little-endian)
    let mut result = Vec::with_capacity(RANDOM_BYTES_NEEDED_FOR_CLHASH);
    for val in &a64 {
        result.extend_from_slice(&val.to_le_bytes());
    }
    result
}

pub struct ClHasher {
    random_data: Vec<u8>,
}

impl ClHasher {
    pub fn new(seed1: u64, seed2: u64) -> Self {
        ClHasher {
            random_data: get_random_key_for_clhash(seed1, seed2),
        }
    }
    pub fn hash<T: AsRef<[u8]>>(&self, data: T) -> u64 {
        clhash(&self.random_data, data.as_ref())
    }
}

impl Drop for ClHasher {
    fn drop(&mut self) {
        // Zero out the random data for security (like free in C)
        for b in self.random_data.iter_mut() {
            *b = 0;
        }
    }
}
