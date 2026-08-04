pub const RANDOM_64BITWORDS_NEEDED_FOR_CLHASH: usize = 133;
pub const RANDOM_BYTES_NEEDED_FOR_CLHASH: usize = 133 * 8;

// ---- Helpers for emulating SSE 128-bit operations using pure Rust ----

#[inline]
fn read_u64(s: &[u8], off: usize) -> u64 {
    u64::from_le_bytes(s[off..off + 8].try_into().unwrap())
}

#[inline]
fn read_m128(s: &[u8], off: usize) -> u128 {
    u128::from_le_bytes(s[off..off + 16].try_into().unwrap())
}

/// Carry-less multiplication of two 64-bit values, producing a 128-bit result.
#[inline]
fn clmul64(a: u64, b: u64) -> u128 {
    let mut result: u128 = 0;
    let a128 = a as u128;
    let mut bb = b;
    let mut shift: u32 = 0;
    while bb != 0 {
        if bb & 1 != 0 {
            result ^= a128 << shift;
        }
        bb >>= 1;
        shift += 1;
    }
    result
}

/// Emulates `_mm_clmulepi64_si128(a, b, imm)`.
///
/// imm bit 0 selects the 64-bit lane from `a` (0 = low, 1 = high).
/// imm bit 4 selects the 64-bit lane from `b`.
#[inline]
fn clmul_imm(a: u128, b: u128, imm: u8) -> u128 {
    let av = if imm & 0x01 != 0 { (a >> 64) as u64 } else { a as u64 };
    let bv = if imm & 0x10 != 0 { (b >> 64) as u64 } else { b as u64 };
    clmul64(av, bv)
}

/// Compute the "lazy" modulo with 2^127 + 2 + 1.
///
/// Equivalent to `Alow ^ (Ahigh << 1) ^ (Ahigh << 2)` in u128 arithmetic.
#[inline]
fn lazymod127(alow: u128, ahigh: u128) -> u128 {
    alow ^ (ahigh.wrapping_shl(1)) ^ (ahigh.wrapping_shl(2))
}

/// Multiply two 128-bit polynomials, returning a lazy reduction modulo 2^127+2+1.
#[inline]
fn mul128by128_lazymod127(a: u128, b: u128) -> u128 {
    let amix1 = clmul_imm(a, b, 0x01);
    let amix2 = clmul_imm(a, b, 0x10);
    let alow = clmul_imm(a, b, 0x00);
    let ahigh = clmul_imm(a, b, 0x11);
    let amix = amix1 ^ amix2;
    let amix_lo_part = amix.wrapping_shl(64);
    let amix_hi_part = amix.wrapping_shr(64);
    let alow = alow ^ amix_lo_part;
    let ahigh = ahigh ^ amix_hi_part;
    lazymod127(alow, ahigh)
}

#[inline]
fn lazy_length_hash(keylength: u64, length: u64) -> u128 {
    // _mm_set_epi64x(keylength, length) gives high=keylength, low=length.
    // _mm_clmulepi64_si128(.., .., 0x10) = clmul(low, high) of that vector.
    clmul64(length, keylength)
}

const SHUFFLE_TABLE: [u8; 16] = [
    0, 27, 54, 45, 108, 119, 90, 65, 216, 195, 238, 245, 180, 175, 130, 153,
];

#[inline]
fn shuffle_epi8(indices: u128) -> u128 {
    let idx_bytes = indices.to_le_bytes();
    let mut out = [0u8; 16];
    for i in 0..16 {
        let idx = idx_bytes[i];
        if idx & 0x80 != 0 {
            out[i] = 0;
        } else {
            out[i] = SHUFFLE_TABLE[(idx & 0x0F) as usize];
        }
    }
    u128::from_le_bytes(out)
}

#[inline]
fn precomp_reduction64_si128(a: u128) -> u128 {
    // C polynomial constant: (1<<4)+(1<<3)+(1<<1)+(1<<0) = 0x1B in low 64 bits.
    let c: u128 = 0x1B;
    let q2 = clmul_imm(a, c, 0x01);
    // Q3 = shuffle(table, srli_si128(Q2, 8))
    let indices = q2 >> 64; // shift right by 8 bytes; high 8 bytes become 0.
    let q3 = shuffle_epi8(indices);
    let q4 = q2 ^ a;
    q3 ^ q4
}

#[inline]
fn precomp_reduction64(a: u128) -> u64 {
    precomp_reduction64_si128(a) as u64
}

#[inline]
fn simple128to64hash_with_length(value: u128, key: u128, keylength: u64, length: u64) -> u64 {
    let add = value ^ key;
    let clprod1 = clmul_imm(add, add, 0x10);
    let total = clprod1 ^ lazy_length_hash(keylength, length);
    precomp_reduction64(total)
}

/// Process exactly `length` 64-bit words (must be divisible by 4) without reduction.
fn clmul_half_scalar_product_no_reduction(
    random: &[u8],
    rs_off: usize,
    string: &[u8],
    str_off: usize,
    length_words: usize,
) -> u128 {
    let mut acc: u128 = 0;
    let mut rs = rs_off;
    let mut s = str_off;
    let s_end = str_off + length_words * 8;
    while s + 32 <= s_end {
        let temp1 = read_m128(random, rs);
        let temp2 = read_m128(string, s);
        let add1 = temp1 ^ temp2;
        let clprod1 = clmul_imm(add1, add1, 0x10);
        acc ^= clprod1;

        let temp12 = read_m128(random, rs + 16);
        let temp22 = read_m128(string, s + 16);
        let add12 = temp12 ^ temp22;
        let clprod12 = clmul_imm(add12, add12, 0x10);
        acc ^= clprod12;

        rs += 32;
        s += 32;
    }
    acc
}

/// Process up to `length` 64-bit words (length not necessarily divisible by 4).
fn clmul_half_scalar_product_with_tail_no_reduction(
    random: &[u8],
    rs_off: usize,
    string: &[u8],
    str_off: usize,
    length_words: usize,
) -> u128 {
    let mut acc: u128 = 0;
    let mut rs = rs_off;
    let mut s = str_off;
    let s_end = str_off + length_words * 8;
    while s + 32 <= s_end {
        let temp1 = read_m128(random, rs);
        let temp2 = read_m128(string, s);
        let add1 = temp1 ^ temp2;
        let clprod1 = clmul_imm(add1, add1, 0x10);
        acc ^= clprod1;

        let temp12 = read_m128(random, rs + 16);
        let temp22 = read_m128(string, s + 16);
        let add12 = temp12 ^ temp22;
        let clprod12 = clmul_imm(add12, add12, 0x10);
        acc ^= clprod12;

        rs += 32;
        s += 32;
    }
    if s + 16 <= s_end {
        let temp1 = read_m128(random, rs);
        let temp2 = read_m128(string, s);
        let add1 = temp1 ^ temp2;
        let clprod1 = clmul_imm(add1, add1, 0x10);
        acc ^= clprod1;
        rs += 16;
        s += 16;
    }
    if s + 8 <= s_end {
        let temp1 = read_m128(random, rs);
        let temp2 = read_u64(string, s) as u128; // _mm_loadl_epi64: high 64 = 0
        let add1 = temp1 ^ temp2;
        let clprod1 = clmul_imm(add1, add1, 0x10);
        acc ^= clprod1;
    }
    acc
}

/// Like `with_tail` but appends an extra word at the end.
fn clmul_half_scalar_product_with_tail_no_reduction_with_extra_word(
    random: &[u8],
    rs_off: usize,
    string: &[u8],
    str_off: usize,
    length_words: usize,
    extraword: u64,
) -> u128 {
    let mut acc: u128 = 0;
    let mut rs = rs_off;
    let mut s = str_off;
    let s_end = str_off + length_words * 8;
    while s + 32 <= s_end {
        let temp1 = read_m128(random, rs);
        let temp2 = read_m128(string, s);
        let add1 = temp1 ^ temp2;
        let clprod1 = clmul_imm(add1, add1, 0x10);
        acc ^= clprod1;

        let temp12 = read_m128(random, rs + 16);
        let temp22 = read_m128(string, s + 16);
        let add12 = temp12 ^ temp22;
        let clprod12 = clmul_imm(add12, add12, 0x10);
        acc ^= clprod12;

        rs += 32;
        s += 32;
    }
    if s + 16 <= s_end {
        let temp1 = read_m128(random, rs);
        let temp2 = read_m128(string, s);
        let add1 = temp1 ^ temp2;
        let clprod1 = clmul_imm(add1, add1, 0x10);
        acc ^= clprod1;
        rs += 16;
        s += 16;
    }
    if s + 8 <= s_end {
        // One word remains from the string; pair it with extraword.
        // _mm_set_epi64x(extraword, *string) → high=extraword, low=*string
        let temp1 = read_m128(random, rs);
        let lo = read_u64(string, s) as u128;
        let hi = (extraword as u128) << 64;
        let temp2 = lo | hi;
        let add1 = temp1 ^ temp2;
        let clprod1 = clmul_imm(add1, add1, 0x10);
        acc ^= clprod1;
    } else {
        // No string words left; only extraword (in low 64 bits).
        let temp1 = read_m128(random, rs);
        let temp2 = extraword as u128; // high 64 = 0
        let add1 = temp1 ^ temp2;
        let clprod1 = clmul_imm(add1, add1, 0x01);
        acc ^= clprod1;
    }
    acc
}

fn clmul_half_scalar_product_only_extra_word(
    random: &[u8],
    rs_off: usize,
    extraword: u64,
) -> u128 {
    let temp1 = read_m128(random, rs_off);
    let temp2 = extraword as u128; // high 64 = 0
    let add1 = temp1 ^ temp2;
    clmul_imm(add1, add1, 0x01)
}

#[inline]
fn create_last_word(lengthbyte: usize, stringbyte: &[u8], offset: usize) -> u64 {
    let significant_bytes = lengthbyte % 8;
    let mut buf = [0u8; 8];
    for i in 0..significant_bytes {
        buf[i] = stringbyte[offset + i];
    }
    u64::from_le_bytes(buf)
}

pub fn clhash(random: &[u8], stringbyte: &[u8]) -> u64 {
    assert!(random.len() >= RANDOM_BYTES_NEEDED_FOR_CLHASH);
    let lengthbyte = stringbyte.len();
    const M: usize = 128; // we process the data in chunks of M 64-bit words
    const M128_PER_BLOCK: usize = M / 2; // 64

    // Polyvalue is loaded from the m128 right after the main random region.
    let poly_off = M128_PER_BLOCK * 16;
    let polyvalue_raw = read_m128(random, poly_off);
    let polyvalue_mask: u128 = (1u128 << 126) - 1; // clear top 2 bits
    let polyvalue = polyvalue_raw & polyvalue_mask;

    let length = lengthbyte / 8; // # of complete 64-bit words
    let lengthinc = (lengthbyte + 7) / 8; // # of 64-bit words including any partial

    let final_key_off = (M128_PER_BLOCK + 1) * 16;
    let keylength_off = (M128_PER_BLOCK + 2) * 16;

    if M < lengthinc {
        // long strings
        let mut acc =
            clmul_half_scalar_product_no_reduction(random, 0, stringbyte, 0, M);
        let mut t = M;
        while t + M <= length {
            acc = mul128by128_lazymod127(polyvalue, acc);
            let h1 = clmul_half_scalar_product_no_reduction(
                random, 0, stringbyte, t * 8, M,
            );
            acc ^= h1;
            t += M;
        }
        let remain = length - t; // # of completely filled words remaining

        if remain != 0 {
            acc = mul128by128_lazymod127(polyvalue, acc);
            if lengthbyte % 8 == 0 {
                let h1 = clmul_half_scalar_product_with_tail_no_reduction(
                    random, 0, stringbyte, t * 8, remain,
                );
                acc ^= h1;
            } else {
                let lastword = create_last_word(lengthbyte, stringbyte, length * 8);
                let h1 = clmul_half_scalar_product_with_tail_no_reduction_with_extra_word(
                    random, 0, stringbyte, t * 8, remain, lastword,
                );
                acc ^= h1;
            }
        } else if lengthbyte % 8 != 0 {
            // No completely-filled words remain, but there's a partial word.
            acc = mul128by128_lazymod127(polyvalue, acc);
            let lastword = create_last_word(lengthbyte, stringbyte, length * 8);
            let h1 = clmul_half_scalar_product_only_extra_word(random, 0, lastword);
            acc ^= h1;
        }

        let finalkey = read_m128(random, final_key_off);
        let keylength = read_u64(random, keylength_off);
        simple128to64hash_with_length(acc, finalkey, keylength, lengthbyte as u64)
    } else {
        // short strings
        if lengthbyte % 8 == 0 {
            let mut acc = clmul_half_scalar_product_with_tail_no_reduction(
                random, 0, stringbyte, 0, length,
            );
            let keylength = read_u64(random, keylength_off);
            acc ^= lazy_length_hash(keylength, lengthbyte as u64);
            return precomp_reduction64(acc);
        }
        let lastword = create_last_word(lengthbyte, stringbyte, length * 8);
        let mut acc = clmul_half_scalar_product_with_tail_no_reduction_with_extra_word(
            random, 0, stringbyte, 0, length, lastword,
        );
        let keylength = read_u64(random, keylength_off);
        acc ^= lazy_length_hash(keylength, lengthbyte as u64);
        precomp_reduction64(acc)
    }
}

// ---- Random key generation via xorshift128+ ----

#[inline]
fn xorshift128plus(state1: &mut u64, state2: &mut u64) -> u64 {
    let mut s1 = *state1;
    let s0 = *state2;
    *state1 = s0;
    s1 ^= s1 << 23;
    let new_s1 = s1 ^ s0 ^ (s1 >> 18) ^ (s0 >> 5);
    *state2 = new_s1;
    new_s1.wrapping_add(s0)
}

pub fn get_random_key_for_clhash(seed1: u64, seed2: u64) -> Vec<u8> {
    let mut k1 = seed1;
    let mut k2 = seed2;
    let mut a64 = [0u64; RANDOM_64BITWORDS_NEEDED_FOR_CLHASH];
    for i in 0..RANDOM_64BITWORDS_NEEDED_FOR_CLHASH {
        a64[i] = xorshift128plus(&mut k1, &mut k2);
    }
    while a64[128] == 0 && a64[129] == 1 {
        a64[128] = xorshift128plus(&mut k1, &mut k2);
        a64[129] = xorshift128plus(&mut k1, &mut k2);
    }
    let mut bytes = Vec::with_capacity(RANDOM_BYTES_NEEDED_FOR_CLHASH);
    for w in a64.iter() {
        bytes.extend_from_slice(&w.to_le_bytes());
    }
    bytes
}

pub struct ClHasher {
    random_data: Vec<u8>,
}

impl ClHasher {
    pub fn new(seed1: u64, seed2: u64) -> Self {
        Self {
            random_data: get_random_key_for_clhash(seed1, seed2),
        }
    }
    pub fn hash<T: AsRef<[u8]>>(&self, data: T) -> u64 {
        clhash(&self.random_data, data.as_ref())
    }
}

impl Drop for ClHasher {
    fn drop(&mut self) {
        // The Vec<u8> in `random_data` is freed automatically when dropped.
    }
}
