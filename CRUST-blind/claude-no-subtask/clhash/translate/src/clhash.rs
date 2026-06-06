pub const RANDOM_64BITWORDS_NEEDED_FOR_CLHASH: usize = 133;
pub const RANDOM_BYTES_NEEDED_FOR_CLHASH: usize = 133 * 8;

// ----------------------------------------------------------------------------
// Software emulation of the SSE2/PCLMUL intrinsics used by the C reference.
// We model an __m128i as a pair of u64 lanes (lo = bits 0..63, hi = bits 64..127).
// ----------------------------------------------------------------------------

#[derive(Copy, Clone, Debug, Default)]
struct M128 {
    lo: u64,
    hi: u64,
}

impl M128 {
    const fn zero() -> Self {
        M128 { lo: 0, hi: 0 }
    }
}

#[inline]
fn xor128(a: M128, b: M128) -> M128 {
    M128 { lo: a.lo ^ b.lo, hi: a.hi ^ b.hi }
}

#[inline]
fn and128(a: M128, b: M128) -> M128 {
    M128 { lo: a.lo & b.lo, hi: a.hi & b.hi }
}

#[inline]
fn or128(a: M128, b: M128) -> M128 {
    M128 { lo: a.lo | b.lo, hi: a.hi | b.hi }
}

#[inline]
fn slli_epi64(a: M128, count: u32) -> M128 {
    if count >= 64 {
        M128::zero()
    } else {
        M128 { lo: a.lo << count, hi: a.hi << count }
    }
}

#[inline]
fn srli_epi64(a: M128, count: u32) -> M128 {
    if count >= 64 {
        M128::zero()
    } else {
        M128 { lo: a.lo >> count, hi: a.hi >> count }
    }
}

/// Shift the entire 128-bit register left by `imm` bytes (zero-fill).
#[inline]
fn slli_si128(a: M128, imm: u32) -> M128 {
    if imm >= 16 {
        return M128::zero();
    }
    let bits = imm * 8;
    if bits == 0 {
        a
    } else if bits == 64 {
        M128 { lo: 0, hi: a.lo }
    } else if bits > 64 {
        let s = bits - 64;
        M128 { lo: 0, hi: a.lo << s }
    } else {
        M128 {
            lo: a.lo << bits,
            hi: (a.hi << bits) | (a.lo >> (64 - bits)),
        }
    }
}

/// Shift the entire 128-bit register right by `imm` bytes (zero-fill).
#[inline]
fn srli_si128(a: M128, imm: u32) -> M128 {
    if imm >= 16 {
        return M128::zero();
    }
    let bits = imm * 8;
    if bits == 0 {
        a
    } else if bits == 64 {
        M128 { lo: a.hi, hi: 0 }
    } else if bits > 64 {
        let s = bits - 64;
        M128 { lo: a.hi >> s, hi: 0 }
    } else {
        M128 {
            lo: (a.lo >> bits) | (a.hi << (64 - bits)),
            hi: a.hi >> bits,
        }
    }
}

/// 64-bit by 64-bit carry-less multiply -> 128-bit result.
#[inline]
fn clmul64(a: u64, b: u64) -> M128 {
    let mut lo: u64 = 0;
    let mut hi: u64 = 0;
    let mut bb = b;
    let mut i: u32 = 0;
    while bb != 0 {
        if bb & 1 == 1 {
            if i == 0 {
                lo ^= a;
            } else {
                lo ^= a.wrapping_shl(i);
                hi ^= a >> (64 - i);
            }
        }
        bb >>= 1;
        i += 1;
    }
    M128 { lo, hi }
}

/// _mm_clmulepi64_si128: imm bit 0 picks lane from `a`, bit 4 picks lane from `b`.
#[inline]
fn clmulepi64_si128(a: M128, b: M128, imm: u8) -> M128 {
    let a_lane = if (imm & 0x01) != 0 { a.hi } else { a.lo };
    let b_lane = if (imm & 0x10) != 0 { b.hi } else { b.lo };
    clmul64(a_lane, b_lane)
}

#[inline]
fn m128_to_bytes(a: M128) -> [u8; 16] {
    let mut result = [0u8; 16];
    result[0..8].copy_from_slice(&a.lo.to_le_bytes());
    result[8..16].copy_from_slice(&a.hi.to_le_bytes());
    result
}

#[inline]
fn bytes_to_m128(b: [u8; 16]) -> M128 {
    let lo = u64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]);
    let hi = u64::from_le_bytes([b[8], b[9], b[10], b[11], b[12], b[13], b[14], b[15]]);
    M128 { lo, hi }
}

/// _mm_shuffle_epi8(source, mask): per-byte shuffle. If mask byte high bit is set
/// the corresponding output byte is zero, otherwise low 4 bits select source byte.
#[inline]
fn shuffle_epi8(source: M128, mask: M128) -> M128 {
    let src_bytes = m128_to_bytes(source);
    let mask_bytes = m128_to_bytes(mask);
    let mut result = [0u8; 16];
    for i in 0..16 {
        let m = mask_bytes[i];
        if m & 0x80 != 0 {
            result[i] = 0;
        } else {
            result[i] = src_bytes[(m & 0x0F) as usize];
        }
    }
    bytes_to_m128(result)
}

/// Load 16 bytes from `data` starting at byte `offset` as a little-endian M128.
#[inline]
fn load_m128_bytes(data: &[u8], offset: usize) -> M128 {
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&data[offset..offset + 16]);
    bytes_to_m128(bytes)
}

/// Read a u64 in little-endian order from `data` at byte `offset`.
#[inline]
fn read_u64_le(data: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
        data[offset + 4],
        data[offset + 5],
        data[offset + 6],
        data[offset + 7],
    ])
}

/// Read up to 8 bytes (less than 8) into the low part of a u64.
#[inline]
fn read_partial_u64_le(data: &[u8], offset: usize, nbytes: usize) -> u64 {
    let mut bytes = [0u8; 8];
    bytes[..nbytes].copy_from_slice(&data[offset..offset + nbytes]);
    u64::from_le_bytes(bytes)
}

// ---- Helpers from C reference ----

#[inline]
fn leftshift1(a: M128) -> M128 {
    let u64shift = slli_epi64(a, 1);
    let topbits = slli_si128(srli_epi64(a, 63), 8);
    or128(u64shift, topbits)
}

#[inline]
fn leftshift2(a: M128) -> M128 {
    let u64shift = slli_epi64(a, 2);
    let topbits = slli_si128(srli_epi64(a, 62), 8);
    or128(u64shift, topbits)
}

#[inline]
fn lazymod127(a_low: M128, a_high: M128) -> M128 {
    let s1 = leftshift1(a_high);
    let s2 = leftshift2(a_high);
    xor128(xor128(a_low, s1), s2)
}

#[inline]
fn mul128by128to128_lazymod127(a: M128, b: M128) -> M128 {
    let amix1 = clmulepi64_si128(a, b, 0x01);
    let amix2 = clmulepi64_si128(a, b, 0x10);
    let mut alow = clmulepi64_si128(a, b, 0x00);
    let mut ahigh = clmulepi64_si128(a, b, 0x11);
    let amix = xor128(amix1, amix2);
    let amix_lo = slli_si128(amix, 8);
    let amix_hi = srli_si128(amix, 8);
    alow = xor128(alow, amix_lo);
    ahigh = xor128(ahigh, amix_hi);
    lazymod127(alow, ahigh)
}

#[inline]
fn lazy_length_hash(keylength: u64, length: u64) -> M128 {
    // _mm_set_epi64x(keylength, length): lo=length, hi=keylength
    // _mm_clmulepi64_si128(v, v, 0x10): a_lane=v.lo=length, b_lane=v.hi=keylength
    clmul64(length, keylength)
}

/// Polynomial reduction modulo x^64 + x^4 + x^3 + x + 1.
/// Returns the 64-bit reduced value (low 64 bits of the M128 result).
#[inline]
fn precomp_reduction64_si128(a: M128) -> M128 {
    // C = _mm_cvtsi64_si128((1<<4)+(1<<3)+(1<<1)+(1<<0)) = 0x1B
    let c = M128 { lo: 0x1B, hi: 0 };
    let q2 = clmulepi64_si128(a, c, 0x01);
    // shuffle table from the C source
    let table = bytes_to_m128([
        0u8, 27u8, 54u8, 45u8, 108u8, 119u8, 90u8, 65u8,
        216u8, 195u8, 238u8, 245u8, 180u8, 175u8, 130u8, 153u8,
    ]);
    let q3 = shuffle_epi8(table, srli_si128(q2, 8));
    let q4 = xor128(q2, a);
    xor128(q3, q4)
}

#[inline]
fn precomp_reduction64(a: M128) -> u64 {
    precomp_reduction64_si128(a).lo
}

#[inline]
fn simple128to64hashwithlength(value: M128, key: M128, keylength: u64, length: u64) -> u64 {
    let add = xor128(value, key);
    let clprod1 = clmulepi64_si128(add, add, 0x10);
    let total = xor128(clprod1, lazy_length_hash(keylength, length));
    precomp_reduction64(total)
}

// --- Variants of __clmulhalfscalarproduct... ---

/// `length` is in u64 words. Random source is read 16 bytes at a time from
/// `random_bytes` starting at `rand_byte_offset`. String is read 8 bytes at a
/// time from `string_bytes` starting at `string_byte_offset`.
/// Used when length is divisible by 4 (in u64 words).
fn clmul_halfscalarproduct_without_reduction(
    random_bytes: &[u8],
    rand_byte_offset: usize,
    string_bytes: &[u8],
    string_byte_offset: usize,
    length_words: usize,
) -> M128 {
    let mut acc = M128::zero();
    let mut r_off = rand_byte_offset;
    let mut s_off = string_byte_offset;
    let end_s_off = string_byte_offset + length_words * 8;
    // Process 4 u64 words (= 2 __m128i of random) per iteration.
    while s_off + 32 <= end_s_off {
        let temp1 = load_m128_bytes(random_bytes, r_off);
        let temp2 = load_m128_bytes(string_bytes, s_off);
        let add1 = xor128(temp1, temp2);
        let clprod1 = clmulepi64_si128(add1, add1, 0x10);
        acc = xor128(clprod1, acc);

        let temp12 = load_m128_bytes(random_bytes, r_off + 16);
        let temp22 = load_m128_bytes(string_bytes, s_off + 16);
        let add12 = xor128(temp12, temp22);
        let clprod12 = clmulepi64_si128(add12, add12, 0x10);
        acc = xor128(clprod12, acc);

        r_off += 32;
        s_off += 32;
    }
    acc
}

/// Variant that may have 1, 2 or 3 trailing u64 words after the 4-word loop.
/// Reads complete u64 words only (used when lengthbyte is divisible by 8).
fn clmul_halfscalarproduct_with_tail_without_reduction(
    random_bytes: &[u8],
    rand_byte_offset: usize,
    string_bytes: &[u8],
    string_byte_offset: usize,
    length_words: usize,
) -> M128 {
    let mut acc = M128::zero();
    let mut r_off = rand_byte_offset;
    let mut s_off = string_byte_offset;
    let end_s_off = string_byte_offset + length_words * 8;

    while s_off + 32 <= end_s_off {
        let temp1 = load_m128_bytes(random_bytes, r_off);
        let temp2 = load_m128_bytes(string_bytes, s_off);
        let add1 = xor128(temp1, temp2);
        let clprod1 = clmulepi64_si128(add1, add1, 0x10);
        acc = xor128(clprod1, acc);

        let temp12 = load_m128_bytes(random_bytes, r_off + 16);
        let temp22 = load_m128_bytes(string_bytes, s_off + 16);
        let add12 = xor128(temp12, temp22);
        let clprod12 = clmulepi64_si128(add12, add12, 0x10);
        acc = xor128(clprod12, acc);

        r_off += 32;
        s_off += 32;
    }

    // string + 1 < endstring -> at least 2 u64 left
    if s_off + 16 <= end_s_off {
        let temp1 = load_m128_bytes(random_bytes, r_off);
        let temp2 = load_m128_bytes(string_bytes, s_off);
        let add1 = xor128(temp1, temp2);
        let clprod1 = clmulepi64_si128(add1, add1, 0x10);
        acc = xor128(clprod1, acc);
        r_off += 16;
        s_off += 16;
    }
    // string < endstring -> at least 1 u64 left
    if s_off < end_s_off {
        let temp1 = load_m128_bytes(random_bytes, r_off);
        // _mm_loadl_epi64: load 8 bytes into lo, zero hi
        let last = read_u64_le(string_bytes, s_off);
        let temp2 = M128 { lo: last, hi: 0 };
        let add1 = xor128(temp1, temp2);
        let clprod1 = clmulepi64_si128(add1, add1, 0x10);
        acc = xor128(clprod1, acc);
    }
    acc
}

/// Variant where the trailing partial word is provided as `extraword`.
fn clmul_halfscalarproduct_with_tail_extra(
    random_bytes: &[u8],
    rand_byte_offset: usize,
    string_bytes: &[u8],
    string_byte_offset: usize,
    length_words: usize,
    extraword: u64,
) -> M128 {
    let mut acc = M128::zero();
    let mut r_off = rand_byte_offset;
    let mut s_off = string_byte_offset;
    let end_s_off = string_byte_offset + length_words * 8;

    while s_off + 32 <= end_s_off {
        let temp1 = load_m128_bytes(random_bytes, r_off);
        let temp2 = load_m128_bytes(string_bytes, s_off);
        let add1 = xor128(temp1, temp2);
        let clprod1 = clmulepi64_si128(add1, add1, 0x10);
        acc = xor128(clprod1, acc);

        let temp12 = load_m128_bytes(random_bytes, r_off + 16);
        let temp22 = load_m128_bytes(string_bytes, s_off + 16);
        let add12 = xor128(temp12, temp22);
        let clprod12 = clmulepi64_si128(add12, add12, 0x10);
        acc = xor128(clprod12, acc);

        r_off += 32;
        s_off += 32;
    }

    if s_off + 16 <= end_s_off {
        let temp1 = load_m128_bytes(random_bytes, r_off);
        let temp2 = load_m128_bytes(string_bytes, s_off);
        let add1 = xor128(temp1, temp2);
        let clprod1 = clmulepi64_si128(add1, add1, 0x10);
        acc = xor128(clprod1, acc);
        r_off += 16;
        s_off += 16;
    }

    if s_off < end_s_off {
        // 1 full u64 word remaining + extraword (the partial word)
        let temp1 = load_m128_bytes(random_bytes, r_off);
        let last = read_u64_le(string_bytes, s_off);
        // _mm_set_epi64x(extraword, *string): lo = *string, hi = extraword
        let temp2 = M128 { lo: last, hi: extraword };
        let add1 = xor128(temp1, temp2);
        let clprod1 = clmulepi64_si128(add1, add1, 0x10);
        acc = xor128(clprod1, acc);
    } else {
        // No full word remaining, only the extraword.
        let temp1 = load_m128_bytes(random_bytes, r_off);
        let temp2 = M128 { lo: extraword, hi: 0 };
        let add1 = xor128(temp1, temp2);
        // imm = 0x01 -> a_lane = a.hi, b_lane = b.lo
        let clprod1 = clmulepi64_si128(add1, add1, 0x01);
        acc = xor128(clprod1, acc);
    }
    acc
}

fn clmul_halfscalarproduct_only_extra(
    random_bytes: &[u8],
    rand_byte_offset: usize,
    extraword: u64,
) -> M128 {
    let temp1 = load_m128_bytes(random_bytes, rand_byte_offset);
    let temp2 = M128 { lo: extraword, hi: 0 };
    let add1 = xor128(temp1, temp2);
    clmulepi64_si128(add1, add1, 0x01)
}

/// Build the partial last word (1..7 significant bytes in little-endian order).
#[inline]
fn create_last_word(string_bytes: &[u8], full_words: usize, lengthbyte: usize) -> u64 {
    let significant = lengthbyte % 8;
    if significant == 0 {
        0
    } else {
        read_partial_u64_le(string_bytes, full_words * 8, significant)
    }
}

// ----------------------------------------------------------------------------
// Public API
// ----------------------------------------------------------------------------

pub fn clhash(random: &[u8], stringbyte: &[u8]) -> u64 {
    assert!(random.len() >= RANDOM_BYTES_NEEDED_FOR_CLHASH);
    let lengthbyte = stringbyte.len();

    // m = 128 (in u64 words processed per main block).
    const M_WORDS: usize = 128;
    // m / 2 == 64 __m128i = 1024 random bytes consumed per block.
    const M128_PER_BLOCK: usize = 64;
    const RAND_BYTES_PER_BLOCK: usize = M128_PER_BLOCK * 16;

    // polyvalue = random[m128_per_block] with two highest bits cleared
    let polyvalue_raw = load_m128_bytes(random, RAND_BYTES_PER_BLOCK);
    let mask = M128 {
        lo: 0xFFFF_FFFF_FFFF_FFFFu64,
        hi: 0x3FFF_FFFF_FFFF_FFFFu64,
    };
    let polyvalue = and128(polyvalue_raw, mask);

    // length = number of complete u64 words
    let length = lengthbyte / 8;
    // lengthinc = number of u64 words covered, including any partial trailing word
    let lengthinc = (lengthbyte + 7) / 8;

    if M_WORDS < lengthinc {
        // long-string branch
        let mut acc =
            clmul_halfscalarproduct_without_reduction(random, 0, stringbyte, 0, M_WORDS);
        let mut t = M_WORDS;
        // for (; t + m <= length; t += m)
        while t + M_WORDS <= length {
            acc = mul128by128to128_lazymod127(polyvalue, acc);
            let h1 = clmul_halfscalarproduct_without_reduction(
                random,
                0,
                stringbyte,
                t * 8,
                M_WORDS,
            );
            acc = xor128(acc, h1);
            t += M_WORDS;
        }
        let remain = length - t; // number of completely filled words remaining

        if remain != 0 {
            acc = mul128by128to128_lazymod127(polyvalue, acc);
            if lengthbyte % 8 == 0 {
                let h1 = clmul_halfscalarproduct_with_tail_without_reduction(
                    random,
                    0,
                    stringbyte,
                    t * 8,
                    remain,
                );
                acc = xor128(acc, h1);
            } else {
                let lastword = create_last_word(stringbyte, length, lengthbyte);
                let h1 = clmul_halfscalarproduct_with_tail_extra(
                    random,
                    0,
                    stringbyte,
                    t * 8,
                    remain,
                    lastword,
                );
                acc = xor128(acc, h1);
            }
        } else if lengthbyte % 8 != 0 {
            acc = mul128by128to128_lazymod127(polyvalue, acc);
            let lastword = create_last_word(stringbyte, length, lengthbyte);
            let h1 = clmul_halfscalarproduct_only_extra(random, 0, lastword);
            acc = xor128(acc, h1);
        }

        let finalkey = load_m128_bytes(random, RAND_BYTES_PER_BLOCK + 16);
        // keylength is the first u64 of __m128i index (M128_PER_BLOCK + 2)
        let keylength = read_u64_le(random, RAND_BYTES_PER_BLOCK + 32);
        simple128to64hashwithlength(acc, finalkey, keylength, lengthbyte as u64)
    } else {
        // short-string branch
        if lengthbyte % 8 == 0 {
            let mut acc = clmul_halfscalarproduct_with_tail_without_reduction(
                random,
                0,
                stringbyte,
                0,
                length,
            );
            let keylength = read_u64_le(random, RAND_BYTES_PER_BLOCK + 32);
            acc = xor128(acc, lazy_length_hash(keylength, lengthbyte as u64));
            precomp_reduction64(acc)
        } else {
            let lastword = create_last_word(stringbyte, length, lengthbyte);
            let mut acc = clmul_halfscalarproduct_with_tail_extra(
                random,
                0,
                stringbyte,
                0,
                length,
                lastword,
            );
            let keylength = read_u64_le(random, RAND_BYTES_PER_BLOCK + 32);
            acc = xor128(acc, lazy_length_hash(keylength, lengthbyte as u64));
            precomp_reduction64(acc)
        }
    }
}

// ----------------------------------------------------------------------------
// xorshift128+ random key generator
// ----------------------------------------------------------------------------

struct Xorshift128Plus {
    part1: u64,
    part2: u64,
}

impl Xorshift128Plus {
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
    let mut rng = Xorshift128Plus::new(seed1, seed2);
    let mut words = [0u64; RANDOM_64BITWORDS_NEEDED_FOR_CLHASH];
    for w in words.iter_mut() {
        *w = rng.next();
    }
    // The C reference re-rolls indices 128 and 129 while (a64[128]==0 && a64[129]==1).
    while words[128] == 0 && words[129] == 1 {
        words[128] = rng.next();
        words[129] = rng.next();
    }
    let mut bytes = Vec::with_capacity(RANDOM_BYTES_NEEDED_FOR_CLHASH);
    for w in words.iter() {
        bytes.extend_from_slice(&w.to_le_bytes());
    }
    bytes
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
        // The Vec frees its own memory; nothing additional to do.
        // Zeroize the buffer to mirror "free" disposal of secret key material.
        for b in self.random_data.iter_mut() {
            *b = 0;
        }
    }
}
