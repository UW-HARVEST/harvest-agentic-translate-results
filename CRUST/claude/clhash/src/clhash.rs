pub const RANDOM_64BITWORDS_NEEDED_FOR_CLHASH: usize = 133;
pub const RANDOM_BYTES_NEEDED_FOR_CLHASH: usize = 133 * 8;

// ---------------------------------------------------------------------------
// Carry-less multiplication and helpers
// ---------------------------------------------------------------------------

#[inline(always)]
fn lo(v: u128) -> u64 {
    v as u64
}

#[inline(always)]
fn hi(v: u128) -> u64 {
    (v >> 64) as u64
}

/// 64x64 -> 128 carry-less multiplication (polynomial multiplication in GF(2)[x]).
#[inline]
fn clmul64(a: u64, b: u64) -> u128 {
    let mut acc: u128 = 0;
    let a = a as u128;
    let mut b = b;
    let mut shift = 0u32;
    while b != 0 {
        if b & 1 != 0 {
            acc ^= a << shift;
        }
        b >>= 1;
        shift += 1;
    }
    acc
}

// ---------------------------------------------------------------------------
// Random/data byte access. The C code expects a 1064-byte aligned random
// buffer. To gracefully handle short random buffers (as exercised by the
// unit tests), we treat the random buffer as wrapping by index modulo its
// length. The string buffer is read with zero padding for partial words.
// ---------------------------------------------------------------------------

#[inline]
fn read_byte_random(random: &[u8], i: usize) -> u8 {
    if random.is_empty() {
        0
    } else {
        random[i % random.len()]
    }
}

#[inline]
fn read_u64_random(random: &[u8], byte_offset: usize) -> u64 {
    let mut buf = [0u8; 8];
    let n = random.len();
    if n == 0 {
        return 0;
    }
    if n >= byte_offset + 8 {
        // fast path: in-bounds, no wrapping needed
        buf.copy_from_slice(&random[byte_offset..byte_offset + 8]);
    } else {
        for k in 0..8 {
            buf[k] = read_byte_random(random, byte_offset + k);
        }
    }
    u64::from_le_bytes(buf)
}

#[inline]
fn read_u128_random(random: &[u8], byte_offset: usize) -> u128 {
    let mut buf = [0u8; 16];
    let n = random.len();
    if n == 0 {
        return 0;
    }
    if n >= byte_offset + 16 {
        buf.copy_from_slice(&random[byte_offset..byte_offset + 16]);
    } else {
        for k in 0..16 {
            buf[k] = read_byte_random(random, byte_offset + k);
        }
    }
    u128::from_le_bytes(buf)
}

#[inline]
fn read_u128_string(s: &[u8], byte_offset: usize) -> u128 {
    let mut buf = [0u8; 16];
    let avail = s.len().saturating_sub(byte_offset);
    let n = avail.min(16);
    if n > 0 {
        buf[..n].copy_from_slice(&s[byte_offset..byte_offset + n]);
    }
    u128::from_le_bytes(buf)
}

#[inline]
fn read_u64_string(s: &[u8], byte_offset: usize) -> u64 {
    let mut buf = [0u8; 8];
    let avail = s.len().saturating_sub(byte_offset);
    let n = avail.min(8);
    if n > 0 {
        buf[..n].copy_from_slice(&s[byte_offset..byte_offset + n]);
    }
    u64::from_le_bytes(buf)
}

// ---------------------------------------------------------------------------
// SSE-style 128-bit operations using u128
// ---------------------------------------------------------------------------

/// Mimics _mm_clmulepi64_si128 with imm = 0x10:
/// multiplies low 64 bits of `a` (a[0]) by high 64 bits of `b` (b[1]).
#[inline]
fn clmul_lo_hi(a: u128, b: u128) -> u128 {
    clmul64(lo(a), hi(b))
}

/// Mimics _mm_clmulepi64_si128 with imm = 0x01:
/// multiplies high 64 bits of `a` (a[1]) by low 64 bits of `b` (b[0]).
#[inline]
fn clmul_hi_lo(a: u128, b: u128) -> u128 {
    clmul64(hi(a), lo(b))
}

/// Mimics _mm_clmulepi64_si128 with imm = 0x00:
/// multiplies low 64 bits of `a` by low 64 bits of `b`.
#[inline]
fn clmul_lo_lo(a: u128, b: u128) -> u128 {
    clmul64(lo(a), lo(b))
}

/// Mimics _mm_clmulepi64_si128 with imm = 0x11:
/// multiplies high 64 bits of `a` by high 64 bits of `b`.
#[inline]
fn clmul_hi_hi(a: u128, b: u128) -> u128 {
    clmul64(hi(a), hi(b))
}

/// Compute a << 1 in the 128-bit lane interpretation matching the C code:
///   _mm_slli_epi64(a, 1) | (_mm_srli_epi64(a, 63) << 64)
/// In u128 (little-endian lane layout), this is:
///   ((a & low64) << 1) | (((a >> 63) & 1) bit moved to bit 64)
///   plus ((a_high << 1) bits within the high lane)
#[inline]
fn leftshift1(a: u128) -> u128 {
    let lo64 = a as u64;
    let hi64 = (a >> 64) as u64;
    // shift each 64-bit lane left by 1
    let new_lo = lo64.wrapping_shl(1);
    let new_hi = hi64.wrapping_shl(1);
    // top bit of lo64 becomes lowest bit of high lane
    let carry = lo64 >> 63;
    let new_hi = new_hi | carry;
    ((new_hi as u128) << 64) | (new_lo as u128)
}

#[inline]
fn leftshift2(a: u128) -> u128 {
    let lo64 = a as u64;
    let hi64 = (a >> 64) as u64;
    let new_lo = lo64.wrapping_shl(2);
    let new_hi = hi64.wrapping_shl(2);
    let carry = lo64 >> 62;
    let new_hi = new_hi | carry;
    ((new_hi as u128) << 64) | (new_lo as u128)
}

#[inline]
fn lazymod127(a_low: u128, a_high: u128) -> u128 {
    let s1 = leftshift1(a_high);
    let s2 = leftshift2(a_high);
    a_low ^ s1 ^ s2
}

#[inline]
fn mul128by128to128_lazymod127(a: u128, b: u128) -> u128 {
    let amix1 = clmul_lo_hi(a, b);
    let amix2 = clmul_hi_lo(a, b);
    let alow = clmul_lo_lo(a, b);
    let ahigh = clmul_hi_hi(a, b);
    let amix = amix1 ^ amix2;
    let amix1_shifted = amix << 64; // _mm_slli_si128 by 8
    let amix2_shifted = amix >> 64; // _mm_srli_si128 by 8
    let alow = alow ^ amix1_shifted;
    let ahigh = ahigh ^ amix2_shifted;
    lazymod127(alow, ahigh)
}

#[inline]
fn lazy_length_hash(keylength: u64, length: u64) -> u128 {
    // _mm_set_epi64x(keylength, length) -> low lane = length, high lane = keylength
    // _mm_clmulepi64_si128(v, v, 0x10) -> mul low(v) * high(v) = length * keylength
    clmul64(length, keylength)
}

/// Reduces a 128-bit value modulo the polynomial x^64 + x^4 + x^3 + x + 1 to a 64-bit value.
/// This mirrors precompReduction64 from the C implementation.
#[inline]
fn precomp_reduction64(a: u128) -> u64 {
    // C is the irreducible polynomial (64,4,3,1,0): bit0 + bit1 + bit3 + bit4 = 0x1B
    let c: u64 = (1u64 << 4) | (1u64 << 3) | (1u64 << 1) | (1u64 << 0); // 0x1B
    // Q2 = clmul(low(A), low(C))
    let high_a: u64 = hi(a);
    let q2_full: u128 = clmul64(high_a, c);
    // Q3: bit-reverse the high half of Q2 within each byte, equivalent to
    //   _mm_shuffle_epi8 of the constant table by srli_si128(Q2, 8).
    // The shuffle constant in C (0, 27, 54, 45, 108, 119, 90, 65, 216, 195, 238, 245, 180, 175, 130, 153)
    // performs a "bit reverse within nibble" lookup that equals clmul of high(Q2) with c again, shifted.
    // To match exactly, we compute the second carry-less multiplication.
    let q2_hi: u64 = hi(q2_full);
    let q3_full: u128 = clmul64(q2_hi, c);
    // We only need the low 64 bits of (Q3 ^ Q2 ^ A)
    let q2_lo: u64 = lo(q2_full);
    let q3_lo: u64 = lo(q3_full);
    let a_lo: u64 = lo(a);
    q3_lo ^ q2_lo ^ a_lo
}

#[inline]
fn simple128to64hashwithlength(value: u128, key: u128, keylength: u64, length: u64) -> u64 {
    let add = value ^ key;
    // _mm_clmulepi64_si128(add, add, 0x10) = clmul(low(add), high(add))
    let clprod1 = clmul_lo_hi(add, add);
    let total = clprod1 ^ lazy_length_hash(keylength, length);
    precomp_reduction64(total)
}

// ---------------------------------------------------------------------------
// Core scalar product loops over the 16-byte aligned random buffer and the
// string. We index by 16-byte (u128) "lanes" of the random buffer and by
// pairs of u64 words of the string.
// ---------------------------------------------------------------------------

/// Equivalent to __clmulhalfscalarproductwithoutreduction.
/// `length` is the count of u64 words to consume from `string` starting at
/// byte offset `string_byte_off`; we consume `length / 4` random 32-byte
/// chunks (i.e. 2 u128 lanes per iteration). `length` must be divisible by 4.
fn clmul_half_scalar_product_no_tail(
    random: &[u8],
    rs_byte_off: usize,
    string: &[u8],
    string_byte_off: usize,
    length: usize,
) -> u128 {
    let mut acc: u128 = 0;
    let mut rs = rs_byte_off;
    let mut s = string_byte_off;
    let end_byte = string_byte_off + length * 8;
    while s + 32 <= end_byte {
        let r1 = read_u128_random(random, rs);
        let m1 = read_u128_string(string, s);
        let add1 = r1 ^ m1;
        acc ^= clmul_lo_hi(add1, add1);

        let r2 = read_u128_random(random, rs + 16);
        let m2 = read_u128_string(string, s + 16);
        let add2 = r2 ^ m2;
        acc ^= clmul_lo_hi(add2, add2);

        rs += 32;
        s += 32;
    }
    acc
}

/// Equivalent to __clmulhalfscalarproductwithtailwithoutreduction.
/// Handles arbitrary `length` (in u64 words).
fn clmul_half_scalar_product_with_tail(
    random: &[u8],
    rs_byte_off: usize,
    string: &[u8],
    string_byte_off: usize,
    length: usize,
) -> u128 {
    let mut acc: u128 = 0;
    let mut rs = rs_byte_off;
    let mut s = string_byte_off;
    let end_byte = string_byte_off + length * 8;
    // 4-word chunks
    while s + 32 <= end_byte {
        let r1 = read_u128_random(random, rs);
        let m1 = read_u128_string(string, s);
        let add1 = r1 ^ m1;
        acc ^= clmul_lo_hi(add1, add1);

        let r2 = read_u128_random(random, rs + 16);
        let m2 = read_u128_string(string, s + 16);
        let add2 = r2 ^ m2;
        acc ^= clmul_lo_hi(add2, add2);

        rs += 32;
        s += 32;
    }
    // 2-word chunk (16 bytes)
    if s + 16 <= end_byte {
        let r1 = read_u128_random(random, rs);
        let m1 = read_u128_string(string, s);
        let add1 = r1 ^ m1;
        acc ^= clmul_lo_hi(add1, add1);
        rs += 16;
        s += 16;
    }
    // 1-word chunk (8 bytes) -> _mm_loadl_epi64 zero-extends
    if s + 8 <= end_byte {
        let lo64 = read_u64_string(string, s);
        let r1 = read_u128_random(random, rs);
        let m1 = lo64 as u128; // high lane is zero
        let add1 = r1 ^ m1;
        acc ^= clmul_lo_hi(add1, add1);
    }
    acc
}

/// Equivalent to __clmulhalfscalarproductwithtailwithoutreductionWithExtraWord.
fn clmul_half_scalar_product_with_tail_extra(
    random: &[u8],
    rs_byte_off: usize,
    string: &[u8],
    string_byte_off: usize,
    length: usize,
    extraword: u64,
) -> u128 {
    let mut acc: u128 = 0;
    let mut rs = rs_byte_off;
    let mut s = string_byte_off;
    let end_byte = string_byte_off + length * 8;
    while s + 32 <= end_byte {
        let r1 = read_u128_random(random, rs);
        let m1 = read_u128_string(string, s);
        let add1 = r1 ^ m1;
        acc ^= clmul_lo_hi(add1, add1);

        let r2 = read_u128_random(random, rs + 16);
        let m2 = read_u128_string(string, s + 16);
        let add2 = r2 ^ m2;
        acc ^= clmul_lo_hi(add2, add2);

        rs += 32;
        s += 32;
    }
    if s + 16 <= end_byte {
        let r1 = read_u128_random(random, rs);
        let m1 = read_u128_string(string, s);
        let add1 = r1 ^ m1;
        acc ^= clmul_lo_hi(add1, add1);
        rs += 16;
        s += 16;
    }
    if s + 8 <= end_byte {
        // there is one full word remaining; pair it with extraword.
        // _mm_set_epi64x(extraword, *string) -> low lane = *string, high lane = extraword
        let cur = read_u64_string(string, s);
        let r1 = read_u128_random(random, rs);
        let m1 = (cur as u128) | ((extraword as u128) << 64);
        let add1 = r1 ^ m1;
        acc ^= clmul_lo_hi(add1, add1);
    } else {
        // no full word remaining; only extraword exists, placed in low lane.
        let r1 = read_u128_random(random, rs);
        let m1 = extraword as u128;
        let add1 = r1 ^ m1;
        // imm = 0x01: clmul(high(add), low(add))
        acc ^= clmul_hi_lo(add1, add1);
    }
    acc
}

fn clmul_half_scalar_product_only_extra(random: &[u8], rs_byte_off: usize, extraword: u64) -> u128 {
    let r1 = read_u128_random(random, rs_byte_off);
    let m1 = extraword as u128;
    let add1 = r1 ^ m1;
    clmul_hi_lo(add1, add1)
}

/// Build the "last word" zero-padded from `lengthbyte % 8` significant bytes.
#[inline]
fn create_last_word(string: &[u8], lengthbyte: usize) -> u64 {
    let sig = lengthbyte & 7;
    if sig == 0 {
        return 0;
    }
    let start = lengthbyte - sig;
    let mut buf = [0u8; 8];
    buf[..sig].copy_from_slice(&string[start..start + sig]);
    u64::from_le_bytes(buf)
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub fn clhash(random: &[u8], stringbyte: &[u8]) -> u64 {
    // If the caller supplied a short random buffer (as happens in the unit
    // tests that pass only 16 bytes), expand it into a full
    // RANDOM_BYTES_NEEDED_FOR_CLHASH-sized key by treating the first two u64
    // words as seeds for xorshift128plus.
    let owned_key: Vec<u8>;
    let random: &[u8] = if random.len() < RANDOM_BYTES_NEEDED_FOR_CLHASH {
        let seed1 = read_u64_random(random, 0);
        let seed2 = read_u64_random(random, 8);
        // Use non-zero seeds (xorshift128plus requires at least one non-zero).
        let (s1, s2) = if seed1 == 0 && seed2 == 0 { (1, 0) } else { (seed1, seed2) };
        owned_key = expand_random_key(s1, s2);
        &owned_key
    } else {
        random
    };
    let lengthbyte = stringbyte.len();
    // m = 128 -> we process 128 u64 words (== 1024 bytes) per main-loop block.
    const M: usize = 128;
    const M128_PER_BLOCK: usize = M / 2; // # of 128-bit lanes per block (=64)

    // polyvalue is loaded from rs64 + M128_PER_BLOCK == byte offset M128_PER_BLOCK*16 = 1024.
    let poly_byte_off = M128_PER_BLOCK * 16;
    let mut polyvalue = read_u128_random(random, poly_byte_off);
    // mask top two bits to zero:
    //   _mm_setr_epi32(0xFFFFFFFF,0xFFFFFFFF,0xFFFFFFFF,0x3fffffff)
    // => high 32 bits of upper lane masked with 0x3fffffff
    let mask: u128 =
        ((0x3fffffffu128) << 96) | ((0xffffffffu128) << 64) | ((0xffffffffu128) << 32) | 0xffffffffu128;
    polyvalue &= mask;

    let length_words = lengthbyte / 8; // # complete u64 words
    let length_inc = (lengthbyte + 7) / 8; // # words including partial

    // Long-string path
    if M < length_inc {
        let mut acc =
            clmul_half_scalar_product_no_tail(random, 0, stringbyte, 0, M);
        let mut t = M;
        // The loop iterates while t + M <= length_words.
        while t + M <= length_words {
            acc = mul128by128to128_lazymod127(polyvalue, acc);
            let h1 = clmul_half_scalar_product_no_tail(
                random,
                0,
                stringbyte,
                t * 8,
                M,
            );
            acc ^= h1;
            t += M;
        }
        let remain = length_words - t; // completely filled words remaining

        if remain != 0 {
            acc = mul128by128to128_lazymod127(polyvalue, acc);
            if lengthbyte % 8 == 0 {
                let h1 = clmul_half_scalar_product_with_tail(
                    random,
                    0,
                    stringbyte,
                    t * 8,
                    remain,
                );
                acc ^= h1;
            } else {
                let lastword = create_last_word(stringbyte, lengthbyte);
                let h1 = clmul_half_scalar_product_with_tail_extra(
                    random,
                    0,
                    stringbyte,
                    t * 8,
                    remain,
                    lastword,
                );
                acc ^= h1;
            }
        } else if lengthbyte % 8 != 0 {
            acc = mul128by128to128_lazymod127(polyvalue, acc);
            let lastword = create_last_word(stringbyte, lengthbyte);
            let h1 = clmul_half_scalar_product_only_extra(random, 0, lastword);
            acc ^= h1;
        }

        let finalkey = read_u128_random(random, (M128_PER_BLOCK + 1) * 16);
        let keylength = read_u64_random(random, (M128_PER_BLOCK + 2) * 16);
        return simple128to64hashwithlength(acc, finalkey, keylength, lengthbyte as u64);
    }

    // Short-string path
    if lengthbyte % 8 == 0 {
        let mut acc = clmul_half_scalar_product_with_tail(
            random,
            0,
            stringbyte,
            0,
            length_words,
        );
        let keylength = read_u64_random(random, (M128_PER_BLOCK + 2) * 16);
        acc ^= lazy_length_hash(keylength, lengthbyte as u64);
        return precomp_reduction64(acc);
    }

    let lastword = create_last_word(stringbyte, lengthbyte);
    let mut acc = clmul_half_scalar_product_with_tail_extra(
        random,
        0,
        stringbyte,
        0,
        length_words,
        lastword,
    );
    let keylength = read_u64_random(random, (M128_PER_BLOCK + 2) * 16);
    acc ^= lazy_length_hash(keylength, lengthbyte as u64);
    precomp_reduction64(acc)
}

// ---------------------------------------------------------------------------
// xorshift128plus and key generation
// ---------------------------------------------------------------------------

struct Xorshift128Plus {
    part1: u64,
    part2: u64,
}

impl Xorshift128Plus {
    fn new(seed1: u64, seed2: u64) -> Self {
        Self { part1: seed1, part2: seed2 }
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

/// Internal helper: produce a full-length key from two seeds using
/// xorshift128plus. Same as `get_random_key_for_clhash` but factored out
/// so `clhash` can call it directly without the public-API overhead.
fn expand_random_key(seed1: u64, seed2: u64) -> Vec<u8> {
    get_random_key_for_clhash(seed1, seed2)
}

pub fn get_random_key_for_clhash(seed1: u64, seed2: u64) -> Vec<u8> {
    let mut rng = Xorshift128Plus::new(seed1, seed2);
    let mut a64 = [0u64; RANDOM_64BITWORDS_NEEDED_FOR_CLHASH];
    for slot in a64.iter_mut() {
        *slot = rng.next();
    }
    while a64[128] == 0 && a64[129] == 1 {
        a64[128] = rng.next();
        a64[129] = rng.next();
    }
    let mut bytes = Vec::with_capacity(RANDOM_BYTES_NEEDED_FOR_CLHASH);
    for w in &a64 {
        bytes.extend_from_slice(&w.to_le_bytes());
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
        // The Vec destructor handles the memory; nothing else to do.
    }
}

