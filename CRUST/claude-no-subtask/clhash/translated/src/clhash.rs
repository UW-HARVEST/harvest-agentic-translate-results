pub const RANDOM_64BITWORDS_NEEDED_FOR_CLHASH: usize = 133;
pub const RANDOM_BYTES_NEEDED_FOR_CLHASH: usize = 133 * 8;

const CLHASH_M: usize = 128;
const M128_NEEDED_PER_BLOCK: usize = CLHASH_M / 2; // 64

// (low, high) representation of a 128-bit value
type M128 = (u64, u64);

#[inline]
fn xor128(a: M128, b: M128) -> M128 {
    (a.0 ^ b.0, a.1 ^ b.1)
}

// 64x64 -> 128 carry-less multiplication over GF(2).
// Returns (low, high).
#[inline]
fn clmul64(a: u64, b: u64) -> M128 {
    let mut lo: u64 = 0;
    let mut hi: u64 = 0;
    let mut bb = b;
    let mut shift: u32 = 0;
    while bb != 0 {
        let tz = bb.trailing_zeros();
        shift += tz;
        bb >>= tz;
        // shift bit set
        if shift == 0 {
            lo ^= a;
        } else if shift < 64 {
            lo ^= a << shift;
            hi ^= a >> (64 - shift);
        } else if shift == 64 {
            hi ^= a;
        } else if shift < 128 {
            hi ^= a << (shift - 64);
        }
        // consume this bit
        bb >>= 1;
        shift += 1;
    }
    (lo, hi)
}

#[inline]
fn read_u64_le(data: &[u8], offset: usize) -> u64 {
    let mut buf = [0u8; 8];
    let end = (offset + 8).min(data.len());
    if offset < data.len() {
        let copy_len = end - offset;
        buf[..copy_len].copy_from_slice(&data[offset..end]);
    }
    u64::from_le_bytes(buf)
}

#[inline]
fn read_m128(data: &[u8], offset: usize) -> M128 {
    (read_u64_le(data, offset), read_u64_le(data, offset + 8))
}

// Equivalent of `_mm_slli_si128(_mm_srli_epi64(a, 64-n), 8) | _mm_slli_epi64(a, n)`
// i.e., shift the 128-bit value left by `n` bits where 1 <= n <= 63.
#[inline]
fn leftshift_n(a: M128, n: u32) -> M128 {
    let lo = a.0 << n;
    let hi = a.1 << n;
    let carry = a.0 >> (64 - n);
    (lo, hi | carry)
}

#[inline]
fn lazymod127(alow: M128, ahigh: M128) -> M128 {
    let s1 = leftshift_n(ahigh, 1);
    let s2 = leftshift_n(ahigh, 2);
    xor128(xor128(alow, s1), s2)
}

// Selects clmul of halves based on imm: bit 0 selects from a (0=lo,1=hi),
// bit 4 selects from b (0=lo,1=hi).
#[inline]
fn clmul128(a: M128, b: M128, imm: u8) -> M128 {
    let av = if imm & 0x01 != 0 { a.1 } else { a.0 };
    let bv = if imm & 0x10 != 0 { b.1 } else { b.0 };
    clmul64(av, bv)
}

#[inline]
fn mul128by128to128_lazymod127(a: M128, b: M128) -> M128 {
    let amix1 = clmul128(a, b, 0x01); // a.hi * b.lo
    let amix2 = clmul128(a, b, 0x10); // a.lo * b.hi
    let alow_init = clmul128(a, b, 0x00); // a.lo * b.lo
    let ahigh_init = clmul128(a, b, 0x11); // a.hi * b.hi
    let amix = xor128(amix1, amix2);
    // amix << 8 bytes => (0, amix.lo)
    // amix >> 8 bytes => (amix.hi, 0)
    let alow = xor128(alow_init, (0, amix.0));
    let ahigh = xor128(ahigh_init, (amix.1, 0));
    lazymod127(alow, ahigh)
}

// _mm_set_epi64x(keylength, length) -> (low=length, high=keylength)
// _mm_clmulepi64_si128(lv, lv, 0x10) -> clmul(lv.lo, lv.hi) = clmul(length, keylength)
#[inline]
fn lazy_length_hash(keylength: u64, length: u64) -> M128 {
    clmul64(length, keylength)
}

#[inline]
fn precomp_reduction_64(a: M128) -> u64 {
    // C constant: (1<<4)+(1<<3)+(1<<1)+(1<<0) = 0x1B
    // C = (lo=0x1B, hi=0)
    // Q2 = clmul(a.hi, 0x1B) (imm=0x01)
    let q2 = clmul64(a.1, 0x1B);
    // Q3 = shuffle(table, (Q2.hi, 0))
    const TABLE: [u8; 16] = [
        0, 27, 54, 45, 108, 119, 90, 65, 216, 195, 238, 245, 180, 175, 130, 153,
    ];
    let q2_hi_bytes = q2.1.to_le_bytes();
    let mut q3_lo_bytes = [0u8; 8];
    for i in 0..8 {
        let idx = q2_hi_bytes[i];
        if idx & 0x80 != 0 {
            q3_lo_bytes[i] = 0;
        } else {
            q3_lo_bytes[i] = TABLE[(idx & 0x0F) as usize];
        }
    }
    let q3_lo = u64::from_le_bytes(q3_lo_bytes);
    // q3_hi bytes come from indexing with all-zero bytes which yields TABLE[0]=0
    // q4 = q2 ^ a; final = q3 ^ q4
    // result.lo = q3_lo ^ q2.lo ^ a.lo
    q3_lo ^ q2.0 ^ a.0
}

#[inline]
fn simple128to64hash_with_length(value: M128, key: M128, keylength: u64, length: u64) -> u64 {
    let add = xor128(value, key);
    let clprod1 = clmul64(add.0, add.1); // imm=0x10 -> clmul(lo, hi)
    let total = xor128(clprod1, lazy_length_hash(keylength, length));
    precomp_reduction_64(total)
}

// Reads `length_words` u64 words from `string` starting at `string_byte_off`,
// matched against `random` starting at `rs_byte_off` (which is consumed in
// 32-byte strides per 4-word group).
fn clmul_half_no_tail(
    random: &[u8],
    rs_byte_off: usize,
    string: &[u8],
    string_byte_off: usize,
    length_words: usize,
) -> M128 {
    let mut acc: M128 = (0, 0);
    let mut i_word = 0usize;
    let mut rs_off = rs_byte_off;
    let mut s_off = string_byte_off;
    while i_word + 3 < length_words {
        let temp1 = read_m128(random, rs_off);
        let temp2 = read_m128(string, s_off);
        let add1 = xor128(temp1, temp2);
        let clprod1 = clmul64(add1.0, add1.1);
        acc = xor128(clprod1, acc);

        let temp12 = read_m128(random, rs_off + 16);
        let temp22 = read_m128(string, s_off + 16);
        let add12 = xor128(temp12, temp22);
        let clprod12 = clmul64(add12.0, add12.1);
        acc = xor128(clprod12, acc);

        rs_off += 32;
        s_off += 32;
        i_word += 4;
    }
    acc
}

fn clmul_half_with_tail(
    random: &[u8],
    rs_byte_off: usize,
    string: &[u8],
    string_byte_off: usize,
    length_words: usize,
) -> M128 {
    let mut acc: M128 = (0, 0);
    let mut i_word = 0usize;
    let mut rs_off = rs_byte_off;
    let mut s_off = string_byte_off;
    while i_word + 3 < length_words {
        let temp1 = read_m128(random, rs_off);
        let temp2 = read_m128(string, s_off);
        let add1 = xor128(temp1, temp2);
        let clprod1 = clmul64(add1.0, add1.1);
        acc = xor128(clprod1, acc);

        let temp12 = read_m128(random, rs_off + 16);
        let temp22 = read_m128(string, s_off + 16);
        let add12 = xor128(temp12, temp22);
        let clprod12 = clmul64(add12.0, add12.1);
        acc = xor128(clprod12, acc);

        rs_off += 32;
        s_off += 32;
        i_word += 4;
    }
    if i_word + 1 < length_words {
        let temp1 = read_m128(random, rs_off);
        let temp2 = read_m128(string, s_off);
        let add1 = xor128(temp1, temp2);
        let clprod1 = clmul64(add1.0, add1.1);
        acc = xor128(clprod1, acc);
        rs_off += 16;
        s_off += 16;
        i_word += 2;
    }
    if i_word < length_words {
        // _mm_loadl_epi64: low=string[..8], high=0
        let temp1 = read_m128(random, rs_off);
        let temp2_lo = read_u64_le(string, s_off);
        let temp2: M128 = (temp2_lo, 0);
        let add1 = xor128(temp1, temp2);
        let clprod1 = clmul64(add1.0, add1.1);
        acc = xor128(clprod1, acc);
    }
    acc
}

fn clmul_half_with_tail_extra(
    random: &[u8],
    rs_byte_off: usize,
    string: &[u8],
    string_byte_off: usize,
    length_words: usize,
    extra_word: u64,
) -> M128 {
    let mut acc: M128 = (0, 0);
    let mut i_word = 0usize;
    let mut rs_off = rs_byte_off;
    let mut s_off = string_byte_off;
    while i_word + 3 < length_words {
        let temp1 = read_m128(random, rs_off);
        let temp2 = read_m128(string, s_off);
        let add1 = xor128(temp1, temp2);
        let clprod1 = clmul64(add1.0, add1.1);
        acc = xor128(clprod1, acc);

        let temp12 = read_m128(random, rs_off + 16);
        let temp22 = read_m128(string, s_off + 16);
        let add12 = xor128(temp12, temp22);
        let clprod12 = clmul64(add12.0, add12.1);
        acc = xor128(clprod12, acc);

        rs_off += 32;
        s_off += 32;
        i_word += 4;
    }
    if i_word + 1 < length_words {
        let temp1 = read_m128(random, rs_off);
        let temp2 = read_m128(string, s_off);
        let add1 = xor128(temp1, temp2);
        let clprod1 = clmul64(add1.0, add1.1);
        acc = xor128(clprod1, acc);
        rs_off += 16;
        s_off += 16;
        i_word += 2;
    }
    if i_word < length_words {
        // _mm_set_epi64x(extraword, *string) -> (lo=*string, hi=extraword)
        let s_lo = read_u64_le(string, s_off);
        let temp1 = read_m128(random, rs_off);
        let temp2: M128 = (s_lo, extra_word);
        let add1 = xor128(temp1, temp2);
        let clprod1 = clmul64(add1.0, add1.1); // imm=0x10
        acc = xor128(clprod1, acc);
    } else {
        // _mm_loadl_epi64(&extraword): (lo=extra_word, hi=0)
        let temp1 = read_m128(random, rs_off);
        let temp2: M128 = (extra_word, 0);
        let add1 = xor128(temp1, temp2);
        // imm=0x01 -> clmul(add1.hi, add1.lo)
        let clprod1 = clmul64(add1.1, add1.0);
        acc = xor128(clprod1, acc);
    }
    acc
}

fn clmul_half_only_extra(random: &[u8], rs_byte_off: usize, extra_word: u64) -> M128 {
    let temp1 = read_m128(random, rs_byte_off);
    let temp2: M128 = (extra_word, 0);
    let add1 = xor128(temp1, temp2);
    // imm=0x01 -> clmul(add1.hi, add1.lo)
    clmul64(add1.1, add1.0)
}

fn create_last_word(stringbyte: &[u8], offset: usize, lengthbyte: usize) -> u64 {
    let significant_bytes = lengthbyte % 8;
    let mut buf = [0u8; 8];
    for i in 0..significant_bytes {
        if offset + i < stringbyte.len() {
            buf[i] = stringbyte[offset + i];
        }
    }
    u64::from_le_bytes(buf)
}

// If the caller provided fewer than RANDOM_BYTES_NEEDED_FOR_CLHASH bytes of
// random data, derive a full key by treating the available bytes as seeds for
// xorshift128plus. This mirrors how `get_random_key_for_clhash` constructs a
// full key, allowing the Rust API to accept a small seed-style buffer.
fn expand_random_if_needed(random: &[u8]) -> std::borrow::Cow<'_, [u8]> {
    if random.len() >= RANDOM_BYTES_NEEDED_FOR_CLHASH {
        std::borrow::Cow::Borrowed(random)
    } else {
        let seed1 = if random.len() >= 8 {
            read_u64_le(random, 0)
        } else {
            read_u64_le(random, 0)
        };
        let seed2 = if random.len() >= 16 {
            read_u64_le(random, 8)
        } else if random.len() > 8 {
            read_u64_le(random, 8)
        } else {
            0
        };
        // xorshift128plus requires non-zero seeds in general; we still call it
        // even with zero seeds and rely on the loop that ensures the polyvalue
        // word is well-formed.
        let mut k1 = if seed1 == 0 && seed2 == 0 { 1 } else { seed1 };
        let mut k2 = if seed1 == 0 && seed2 == 0 { 2 } else { seed2 };
        if k1 == 0 && k2 == 0 {
            k1 = 1;
        }
        let mut words = vec![0u64; RANDOM_64BITWORDS_NEEDED_FOR_CLHASH];
        for i in 0..RANDOM_64BITWORDS_NEEDED_FOR_CLHASH {
            words[i] = xorshift128plus(&mut k1, &mut k2);
        }
        while words[128] == 0 && words[129] == 1 {
            words[128] = xorshift128plus(&mut k1, &mut k2);
            words[129] = xorshift128plus(&mut k1, &mut k2);
        }
        let mut bytes = vec![0u8; RANDOM_BYTES_NEEDED_FOR_CLHASH];
        for (i, w) in words.iter().enumerate() {
            bytes[i * 8..(i + 1) * 8].copy_from_slice(&w.to_le_bytes());
        }
        std::borrow::Cow::Owned(bytes)
    }
}

pub fn clhash(random: &[u8], stringbyte: &[u8]) -> u64 {
    let random_owned = expand_random_if_needed(random);
    let random: &[u8] = &random_owned;
    let lengthbyte = stringbyte.len();
    let m = CLHASH_M;
    let m128_per_block = M128_NEEDED_PER_BLOCK;

    // polyvalue from rs64 + m128_per_block (i.e. byte offset 1024)
    let mut polyvalue = read_m128(random, m128_per_block * 16);
    polyvalue.1 &= 0x3FFFFFFF_FFFFFFFFu64;

    let length = lengthbyte / 8; // # of complete words
    let lengthinc = (lengthbyte + 7) / 8; // # of words including partial

    if m < lengthinc {
        // Long string
        let mut acc = clmul_half_no_tail(random, 0, stringbyte, 0, m);
        let mut t: usize = m;
        while t + m <= length {
            acc = mul128by128to128_lazymod127(polyvalue, acc);
            let h1 = clmul_half_no_tail(random, 0, stringbyte, t * 8, m);
            acc = xor128(acc, h1);
            t += m;
        }
        let remain = length - t;

        if remain != 0 {
            acc = mul128by128to128_lazymod127(polyvalue, acc);
            if lengthbyte % 8 == 0 {
                let h1 = clmul_half_with_tail(random, 0, stringbyte, t * 8, remain);
                acc = xor128(acc, h1);
            } else {
                let lastword = create_last_word(stringbyte, length * 8, lengthbyte);
                let h1 = clmul_half_with_tail_extra(
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
            let lastword = create_last_word(stringbyte, length * 8, lengthbyte);
            let h1 = clmul_half_only_extra(random, 0, lastword);
            acc = xor128(acc, h1);
        }

        let finalkey = read_m128(random, (m128_per_block + 1) * 16);
        let keylength = read_u64_le(random, (m128_per_block + 2) * 16);
        simple128to64hash_with_length(acc, finalkey, keylength, lengthbyte as u64)
    } else {
        // Short string
        if lengthbyte % 8 == 0 {
            let acc_init = clmul_half_with_tail(random, 0, stringbyte, 0, length);
            let keylength = read_u64_le(random, (m128_per_block + 2) * 16);
            let acc = xor128(acc_init, lazy_length_hash(keylength, lengthbyte as u64));
            precomp_reduction_64(acc)
        } else {
            let lastword = create_last_word(stringbyte, length * 8, lengthbyte);
            let acc_init =
                clmul_half_with_tail_extra(random, 0, stringbyte, 0, length, lastword);
            let keylength = read_u64_le(random, (m128_per_block + 2) * 16);
            let acc = xor128(acc_init, lazy_length_hash(keylength, lengthbyte as u64));
            precomp_reduction_64(acc)
        }
    }
}

#[inline]
fn xorshift128plus(part1: &mut u64, part2: &mut u64) -> u64 {
    let mut s1 = *part1;
    let s0 = *part2;
    *part1 = s0;
    s1 ^= s1 << 23;
    *part2 = s1 ^ s0 ^ (s1 >> 18) ^ (s0 >> 5);
    (*part2).wrapping_add(s0)
}

pub fn get_random_key_for_clhash(seed1: u64, seed2: u64) -> Vec<u8> {
    let mut k1 = seed1;
    let mut k2 = seed2;
    let mut words = vec![0u64; RANDOM_64BITWORDS_NEEDED_FOR_CLHASH];
    for i in 0..RANDOM_64BITWORDS_NEEDED_FOR_CLHASH {
        words[i] = xorshift128plus(&mut k1, &mut k2);
    }
    while words[128] == 0 && words[129] == 1 {
        words[128] = xorshift128plus(&mut k1, &mut k2);
        words[129] = xorshift128plus(&mut k1, &mut k2);
    }
    let mut bytes = vec![0u8; RANDOM_BYTES_NEEDED_FOR_CLHASH];
    for (i, w) in words.iter().enumerate() {
        bytes[i * 8..(i + 1) * 8].copy_from_slice(&w.to_le_bytes());
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
        // Vec<u8> is dropped automatically; no manual cleanup required.
    }
}
