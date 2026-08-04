pub const RANDOM_64BITWORDS_NEEDED_FOR_CLHASH: usize = 133;
pub const RANDOM_BYTES_NEEDED_FOR_CLHASH: usize = 133 * 8;

// --- xorshift128plus PRNG ---

struct Xorshift128PlusKey {
    part1: u64,
    part2: u64,
}

fn xorshift128plus(key: &mut Xorshift128PlusKey) -> u64 {
    let mut s1 = key.part1;
    let s0 = key.part2;
    key.part1 = s0;
    s1 ^= s1 << 23;
    key.part2 = s1 ^ s0 ^ (s1 >> 18) ^ (s0 >> 5);
    key.part2.wrapping_add(s0)
}

// --- 128-bit value as (lo, hi) pair of u64 ---

/// Carry-less multiply two 64-bit values, producing a 128-bit result (lo, hi).
fn clmul64(a: u64, b: u64) -> (u64, u64) {
    let mut lo: u64 = 0;
    let mut hi: u64 = 0;
    for i in 0..64 {
        if (b >> i) & 1 == 1 {
            lo ^= a << i;
            if i > 0 {
                hi ^= a >> (64 - i);
            }
        }
    }
    (lo, hi)
}

/// _mm_clmulepi64_si128 equivalent.
/// `a` = (a_lo, a_hi), `b` = (b_lo, b_hi), `imm` selects which 64-bit halves to multiply.
/// imm=0x00: a_lo * b_lo, imm=0x01: a_hi * b_lo, imm=0x10: a_lo * b_hi, imm=0x11: a_hi * b_hi
fn clmulepi64(a: (u64, u64), b: (u64, u64), imm: u8) -> (u64, u64) {
    let av = if imm & 0x01 != 0 { a.1 } else { a.0 };
    let bv = if imm & 0x10 != 0 { b.1 } else { b.0 };
    clmul64(av, bv)
}

// 128-bit left shift by 1
fn leftshift1(a: (u64, u64)) -> (u64, u64) {
    let lo = a.0 << 1;
    let hi = (a.1 << 1) | (a.0 >> 63);
    (lo, hi)
}

// 128-bit left shift by 2
fn leftshift2(a: (u64, u64)) -> (u64, u64) {
    let lo = a.0 << 2;
    let hi = (a.1 << 2) | (a.0 >> 62);
    (lo, hi)
}

fn xor128(a: (u64, u64), b: (u64, u64)) -> (u64, u64) {
    (a.0 ^ b.0, a.1 ^ b.1)
}

/// Lazy modular reduction with 2^127 + 2 + 1 (actually 2*(2^127+2+1))
fn lazymod127(alow: (u64, u64), ahigh: (u64, u64)) -> (u64, u64) {
    let shift1 = leftshift1(ahigh);
    let shift2 = leftshift2(ahigh);
    xor128(xor128(alow, shift1), shift2)
}

/// Multiplication with lazy reduction mod 2^127+2+1
fn mul128by128to128_lazymod127(a: (u64, u64), b: (u64, u64)) -> (u64, u64) {
    let amix1 = clmulepi64(a, b, 0x01);
    let amix2 = clmulepi64(a, b, 0x10);
    let alow = clmulepi64(a, b, 0x00);
    let ahigh = clmulepi64(a, b, 0x11);
    let amix = xor128(amix1, amix2);
    // _mm_slli_si128(amix, 8) shifts left by 8 bytes = moves lo to hi, lo becomes 0
    let amix1_shifted = (0u64, amix.0);
    // _mm_srli_si128(amix, 8) shifts right by 8 bytes = moves hi to lo, hi becomes 0
    let amix2_shifted = (amix.1, 0u64);
    let alow = xor128(alow, amix1_shifted);
    let ahigh = xor128(ahigh, amix2_shifted);
    lazymod127(alow, ahigh)
}

/// Lazy length hash: clmul of (length, keylength) with imm=0x10
fn lazy_length_hash(keylength: u64, length: u64) -> (u64, u64) {
    let lengthvector = (length, keylength);
    clmulepi64(lengthvector, lengthvector, 0x10)
}

/// Reduction to 64 bits using the irreducible polynomial (64,4,3,1,0)
fn precomp_reduction64(a: (u64, u64)) -> u64 {
    let c: u64 = (1 << 4) + (1 << 3) + (1 << 1) + (1 << 0); // 0x1B
    // Q2 = clmulepi64(A, C_vec, 0x01) where C_vec = (c, 0)
    // imm=0x01 means: a.hi * c_vec.lo = a.1 * c
    let q2 = clmul64(a.1, c);

    // The shuffle table for the high 64 bits of Q2
    let table: [u8; 16] = [0, 27, 54, 45, 108, 119, 90, 65, 216, 195, 238, 245, 180, 175, 130, 153];
    // Q3 = shuffle using high 64 bits of Q2 as indices
    // _mm_srli_si128(Q2, 8) gives (q2.1, 0) — the high word moved to low
    // Then shuffle: each byte of the index selects from the table
    let idx_bytes = q2.1.to_le_bytes();
    let mut q3_bytes = [0u8; 16];
    for i in 0..8 {
        let idx = (idx_bytes[i] & 0x0F) as usize;
        // _mm_shuffle_epi8 zeros output byte if high bit of index is set
        if idx_bytes[i] & 0x80 != 0 {
            q3_bytes[i] = 0;
        } else {
            q3_bytes[i] = table[idx];
        }
    }
    // high 8 bytes are zero since the index bytes (from srli_si128 Q2,8) high half is 0
    // but we only need the low 64 bits anyway
    let q3_lo = u64::from_le_bytes([q3_bytes[0], q3_bytes[1], q3_bytes[2], q3_bytes[3],
                                     q3_bytes[4], q3_bytes[5], q3_bytes[6], q3_bytes[7]]);

    // Q4 = Q2 XOR A
    let q4_lo = q2.0 ^ a.0;
    // final = Q3 XOR Q4 (low 64 bits only)
    q3_lo ^ q4_lo
}

/// Hash 128-bit value with key and length
fn simple128to64hashwithlength(value: (u64, u64), key: (u64, u64), keylength: u64, length: u64) -> u64 {
    let add = xor128(value, key);
    let clprod1 = clmulepi64(add, add, 0x10);
    let total = xor128(clprod1, lazy_length_hash(keylength, length));
    precomp_reduction64(total)
}

// --- Helper to read u64 from a byte slice (little-endian) ---

fn read_u64_le(data: &[u8], offset: usize) -> u64 {
    let end = (offset + 8).min(data.len());
    let mut buf = [0u8; 8];
    let len = end.saturating_sub(offset);
    buf[..len].copy_from_slice(&data[offset..offset + len]);
    u64::from_le_bytes(buf)
}

/// Read the random source as pairs of u64 (representing __m128i values)
fn read_rs128(random: &[u8], index: usize) -> (u64, u64) {
    let off = index * 16;
    (read_u64_le(random, off), read_u64_le(random, off + 8))
}

fn create_last_word(lengthbyte: usize, string_bytes: &[u8], offset: usize) -> u64 {
    let significant = lengthbyte % 8;
    let mut buf = [0u8; 8];
    let avail = string_bytes.len().saturating_sub(offset);
    let to_copy = significant.min(avail);
    buf[..to_copy].copy_from_slice(&string_bytes[offset..offset + to_copy]);
    u64::from_le_bytes(buf)
}

// --- Scalar product functions ---

/// __clmulhalfscalarproductwithoutreduction
/// length must be divisible by 4
fn clmul_half_scalar_product_without_reduction(
    rs: &[u8], string: &[u64], str_offset: usize, length: usize,
) -> (u64, u64) {
    let mut acc: (u64, u64) = (0, 0);
    let mut rs_idx: usize = 0; // index into 128-bit random words
    let mut s = 0usize;
    while s + 3 < length {
        let temp1 = read_rs128(rs, rs_idx);
        let temp2 = (string[str_offset + s], string[str_offset + s + 1]);
        let add1 = xor128(temp1, temp2);
        let clprod1 = clmulepi64(add1, add1, 0x10);
        acc = xor128(clprod1, acc);

        let temp12 = read_rs128(rs, rs_idx + 1);
        let temp22 = (string[str_offset + s + 2], string[str_offset + s + 3]);
        let add12 = xor128(temp12, temp22);
        let clprod12 = clmulepi64(add12, add12, 0x10);
        acc = xor128(clprod12, acc);

        rs_idx += 2;
        s += 4;
    }
    acc
}

/// __clmulhalfscalarproductwithtailwithoutreduction
fn clmul_half_scalar_product_with_tail_without_reduction(
    rs: &[u8], string: &[u64], str_offset: usize, length: usize,
) -> (u64, u64) {
    let mut acc: (u64, u64) = (0, 0);
    let mut rs_idx: usize = 0;
    let mut s = 0usize;
    while s + 3 < length {
        let temp1 = read_rs128(rs, rs_idx);
        let temp2 = (string[str_offset + s], string[str_offset + s + 1]);
        let add1 = xor128(temp1, temp2);
        let clprod1 = clmulepi64(add1, add1, 0x10);
        acc = xor128(clprod1, acc);

        let temp12 = read_rs128(rs, rs_idx + 1);
        let temp22 = (string[str_offset + s + 2], string[str_offset + s + 3]);
        let add12 = xor128(temp12, temp22);
        let clprod12 = clmulepi64(add12, add12, 0x10);
        acc = xor128(clprod12, acc);

        rs_idx += 2;
        s += 4;
    }
    if s + 1 < length {
        let temp1 = read_rs128(rs, rs_idx);
        let temp2 = (string[str_offset + s], string[str_offset + s + 1]);
        let add1 = xor128(temp1, temp2);
        let clprod1 = clmulepi64(add1, add1, 0x10);
        acc = xor128(clprod1, acc);
        rs_idx += 1;
        s += 2;
    }
    if s < length {
        let temp1 = read_rs128(rs, rs_idx);
        let temp2 = (string[str_offset + s], 0u64);
        let add1 = xor128(temp1, temp2);
        let clprod1 = clmulepi64(add1, add1, 0x10);
        acc = xor128(clprod1, acc);
    }
    acc
}

/// __clmulhalfscalarproductwithtailwithoutreductionWithExtraWord
fn clmul_half_scalar_product_with_tail_without_reduction_with_extra_word(
    rs: &[u8], string: &[u64], str_offset: usize, length: usize, extraword: u64,
) -> (u64, u64) {
    let mut acc: (u64, u64) = (0, 0);
    let mut rs_idx: usize = 0;
    let mut s = 0usize;
    while s + 3 < length {
        let temp1 = read_rs128(rs, rs_idx);
        let temp2 = (string[str_offset + s], string[str_offset + s + 1]);
        let add1 = xor128(temp1, temp2);
        let clprod1 = clmulepi64(add1, add1, 0x10);
        acc = xor128(clprod1, acc);

        let temp12 = read_rs128(rs, rs_idx + 1);
        let temp22 = (string[str_offset + s + 2], string[str_offset + s + 3]);
        let add12 = xor128(temp12, temp22);
        let clprod12 = clmulepi64(add12, add12, 0x10);
        acc = xor128(clprod12, acc);

        rs_idx += 2;
        s += 4;
    }
    if s + 1 < length {
        let temp1 = read_rs128(rs, rs_idx);
        let temp2 = (string[str_offset + s], string[str_offset + s + 1]);
        let add1 = xor128(temp1, temp2);
        let clprod1 = clmulepi64(add1, add1, 0x10);
        acc = xor128(clprod1, acc);
        rs_idx += 1;
        s += 2;
    }
    if s < length {
        let temp1 = read_rs128(rs, rs_idx);
        let temp2 = (string[str_offset + s], extraword);
        let add1 = xor128(temp1, temp2);
        let clprod1 = clmulepi64(add1, add1, 0x10);
        acc = xor128(clprod1, acc);
    } else {
        let temp1 = read_rs128(rs, rs_idx);
        let temp2 = (extraword, 0u64);
        let add1 = xor128(temp1, temp2);
        // Note: imm=0x01 here (not 0x10) — matches the C code
        let clprod1 = clmulepi64(add1, add1, 0x01);
        acc = xor128(clprod1, acc);
    }
    acc
}

/// __clmulhalfscalarproductOnlyExtraWord
fn clmul_half_scalar_product_only_extra_word(rs: &[u8], extraword: u64) -> (u64, u64) {
    let temp1 = read_rs128(rs, 0);
    let temp2 = (extraword, 0u64);
    let add1 = xor128(temp1, temp2);
    clmulepi64(add1, add1, 0x01)
}

// --- Main clhash function ---

pub fn clhash(random: &[u8], stringbyte: &[u8]) -> u64 {
    let m: usize = 128;
    let m128neededperblock = m / 2; // 64

    // polyvalue = rs64[m128neededperblock] with top 2 bits of high word zeroed
    let mut polyvalue = read_rs128(random, m128neededperblock);
    polyvalue.1 &= 0x3FFFFFFFFFFFFFFF;

    let lengthbyte = stringbyte.len();
    let length = lengthbyte / 8; // complete u64 words
    let lengthinc = (lengthbyte + 7) / 8; // including partial

    // Convert string bytes to u64 array (only complete words needed for indexing)
    let string: Vec<u64> = {
        let n = lengthinc.max(1);
        let mut v = Vec::with_capacity(n);
        for i in 0..length {
            v.push(read_u64_le(stringbyte, i * 8));
        }
        // partial word if any — we handle it separately via create_last_word
        if lengthinc > length {
            // push a zero placeholder; we use create_last_word for the actual partial word
            v.push(0);
        }
        v
    };

    if m < lengthinc {
        // Long strings
        let mut acc = clmul_half_scalar_product_without_reduction(random, &string, 0, m);
        let mut t = m;
        while t + m <= length {
            acc = mul128by128to128_lazymod127(polyvalue, acc);
            let h1 = clmul_half_scalar_product_without_reduction(random, &string, t, m);
            acc = xor128(acc, h1);
            t += m;
        }
        let remain = length - t;
        if remain != 0 {
            acc = mul128by128to128_lazymod127(polyvalue, acc);
            if lengthbyte % 8 == 0 {
                let h1 = clmul_half_scalar_product_with_tail_without_reduction(random, &string, t, remain);
                acc = xor128(acc, h1);
            } else {
                let lastword = create_last_word(lengthbyte, stringbyte, length * 8);
                let h1 = clmul_half_scalar_product_with_tail_without_reduction_with_extra_word(
                    random, &string, t, remain, lastword,
                );
                acc = xor128(acc, h1);
            }
        } else if lengthbyte % 8 != 0 {
            acc = mul128by128to128_lazymod127(polyvalue, acc);
            let lastword = create_last_word(lengthbyte, stringbyte, length * 8);
            let h1 = clmul_half_scalar_product_only_extra_word(random, lastword);
            acc = xor128(acc, h1);
        }

        let finalkey = read_rs128(random, m128neededperblock + 1);
        let keylength = read_u64_le(random, (m128neededperblock + 2) * 16);
        simple128to64hashwithlength(acc, finalkey, keylength, lengthbyte as u64)
    } else {
        // Short strings
        if lengthbyte % 8 == 0 {
            let mut acc = clmul_half_scalar_product_with_tail_without_reduction(random, &string, 0, length);
            let keylength = read_u64_le(random, (m128neededperblock + 2) * 16);
            acc = xor128(acc, lazy_length_hash(keylength, lengthbyte as u64));
            precomp_reduction64(acc)
        } else {
            let lastword = create_last_word(lengthbyte, stringbyte, length * 8);
            let mut acc = clmul_half_scalar_product_with_tail_without_reduction_with_extra_word(
                random, &string, 0, length, lastword,
            );
            let keylength = read_u64_le(random, (m128neededperblock + 2) * 16);
            acc = xor128(acc, lazy_length_hash(keylength, lengthbyte as u64));
            precomp_reduction64(acc)
        }
    }
}

pub fn get_random_key_for_clhash(seed1: u64, seed2: u64) -> Vec<u8> {
    let mut k = Xorshift128PlusKey { part1: seed1, part2: seed2 };
    let mut a64 = vec![0u64; RANDOM_64BITWORDS_NEEDED_FOR_CLHASH];
    for i in 0..RANDOM_64BITWORDS_NEEDED_FOR_CLHASH {
        a64[i] = xorshift128plus(&mut k);
    }
    while a64[128] == 0 && a64[129] == 1 {
        a64[128] = xorshift128plus(&mut k);
        a64[129] = xorshift128plus(&mut k);
    }
    // Convert to bytes (little-endian)
    let mut result = vec![0u8; RANDOM_BYTES_NEEDED_FOR_CLHASH];
    for (i, &val) in a64.iter().enumerate() {
        let bytes = val.to_le_bytes();
        result[i * 8..i * 8 + 8].copy_from_slice(&bytes);
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
        // In Rust, Vec is automatically freed. Nothing extra needed.
        // Clear sensitive data for safety.
        for b in self.random_data.iter_mut() {
            *b = 0;
        }
    }
}
