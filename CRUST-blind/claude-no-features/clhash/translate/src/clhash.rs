pub const RANDOM_64BITWORDS_NEEDED_FOR_CLHASH: usize = 133;
pub const RANDOM_BYTES_NEEDED_FOR_CLHASH: usize = 133 * 8;

// A 128-bit value modeled as two u64 halves, mimicking __m128i (lo = bytes 0..8, hi = bytes 8..16)
#[derive(Clone, Copy, Debug)]
struct M128i {
    lo: u64,
    hi: u64,
}

impl M128i {
    const fn zero() -> Self {
        Self { lo: 0, hi: 0 }
    }
    fn xor(self, other: Self) -> Self {
        Self {
            lo: self.lo ^ other.lo,
            hi: self.hi ^ other.hi,
        }
    }
}

// Carry-less multiplication of two 64-bit values producing a 128-bit result
fn clmul64(a: u64, b: u64) -> M128i {
    let a128 = a as u128;
    let mut result: u128 = 0;
    for i in 0..64u32 {
        if (b >> i) & 1 == 1 {
            result ^= a128 << i;
        }
    }
    M128i {
        lo: result as u64,
        hi: (result >> 64) as u64,
    }
}

// Mimics _mm_clmulepi64_si128(a, b, imm)
// imm bit 0: select hi of a if 1, else lo
// imm bit 4: select hi of b if 1, else lo
fn clmul_imm(a: M128i, b: M128i, imm: u32) -> M128i {
    let av = if imm & 0x01 != 0 { a.hi } else { a.lo };
    let bv = if imm & 0x10 != 0 { b.hi } else { b.lo };
    clmul64(av, bv)
}

// Read M128i (16 bytes) from random data at m128_idx index
fn load_random_m128i(random: &[u8], m128_idx: usize) -> M128i {
    let off = m128_idx * 16;
    M128i {
        lo: u64::from_le_bytes(random[off..off + 8].try_into().unwrap()),
        hi: u64::from_le_bytes(random[off + 8..off + 16].try_into().unwrap()),
    }
}

// Read a single u64 word from random at u64 index
fn load_random_u64(random: &[u8], idx: usize) -> u64 {
    let off = idx * 8;
    u64::from_le_bytes(random[off..off + 8].try_into().unwrap())
}

// Read a u64 word from string at word index
fn load_string_word(stringbyte: &[u8], word_idx: usize) -> u64 {
    let off = word_idx * 8;
    u64::from_le_bytes(stringbyte[off..off + 8].try_into().unwrap())
}

// Read a 16-byte M128i from string at the given word index (= 8*word_idx bytes)
fn load_string_m128i(stringbyte: &[u8], word_idx: usize) -> M128i {
    M128i {
        lo: load_string_word(stringbyte, word_idx),
        hi: load_string_word(stringbyte, word_idx + 1),
    }
}

// Compute a << 1 over 128 bits (assuming top bit is 0)
fn leftshift1(a: M128i) -> M128i {
    M128i {
        lo: a.lo << 1,
        hi: (a.hi << 1) | (a.lo >> 63),
    }
}

// Compute a << 2 over 128 bits (assuming top 2 bits are 0)
fn leftshift2(a: M128i) -> M128i {
    M128i {
        lo: a.lo << 2,
        hi: (a.hi << 2) | (a.lo >> 62),
    }
}

// Lazy modulo by 2^127 + 2 + 1 (assuming top 2 bits of ahigh are 0)
fn lazymod127(alow: M128i, ahigh: M128i) -> M128i {
    let shift1 = leftshift1(ahigh);
    let shift2 = leftshift2(ahigh);
    alow.xor(shift1).xor(shift2)
}

// 128 x 128 -> 128 carry-less multiply with lazy reduction
fn mul128by128to128_lazymod127(a: M128i, b: M128i) -> M128i {
    let amix1 = clmul_imm(a, b, 0x01);
    let amix2 = clmul_imm(a, b, 0x10);
    let alow = clmul_imm(a, b, 0x00);
    let ahigh = clmul_imm(a, b, 0x11);
    let amix = amix1.xor(amix2);
    // _mm_slli_si128(amix, 8): hi = amix.lo, lo = 0
    let amix1_shifted = M128i {
        lo: 0,
        hi: amix.lo,
    };
    // _mm_srli_si128(amix, 8): lo = amix.hi, hi = 0
    let amix2_shifted = M128i {
        lo: amix.hi,
        hi: 0,
    };
    let alow = alow.xor(amix1_shifted);
    let ahigh = ahigh.xor(amix2_shifted);
    lazymod127(alow, ahigh)
}

// length-component hash (carry-less product of length and keylength)
fn lazy_length_hash(keylength: u64, length: u64) -> M128i {
    // _mm_set_epi64x(keylength, length) => lo=length, hi=keylength
    let lengthvector = M128i {
        lo: length,
        hi: keylength,
    };
    // 0x10: lo * hi -> length * keylength
    clmul_imm(lengthvector, lengthvector, 0x10)
}

// Convert M128i to/from 16-byte little-endian array
fn m128i_to_bytes(m: M128i) -> [u8; 16] {
    let mut b = [0u8; 16];
    b[..8].copy_from_slice(&m.lo.to_le_bytes());
    b[8..].copy_from_slice(&m.hi.to_le_bytes());
    b
}

fn bytes_to_m128i(b: [u8; 16]) -> M128i {
    M128i {
        lo: u64::from_le_bytes(b[..8].try_into().unwrap()),
        hi: u64::from_le_bytes(b[8..].try_into().unwrap()),
    }
}

// _mm_shuffle_epi8: byte-wise shuffle. If MSB of index byte is set, output 0.
fn shuffle_epi8(table: M128i, indices: M128i) -> M128i {
    let table_bytes = m128i_to_bytes(table);
    let idx_bytes = m128i_to_bytes(indices);
    let mut result_bytes = [0u8; 16];
    for i in 0..16 {
        let idx = idx_bytes[i];
        if idx & 0x80 == 0 {
            result_bytes[i] = table_bytes[(idx & 0x0F) as usize];
        }
    }
    bytes_to_m128i(result_bytes)
}

// Modulo reduction to 64-bit value (high 64 bits contain garbage)
fn precomp_reduction_64_si128(a: M128i) -> M128i {
    // C = irreducible polynomial constant: (1<<4)+(1<<3)+(1<<1)+(1<<0) = 27
    let c = M128i { lo: 27, hi: 0 };
    let q2 = clmul_imm(a, c, 0x01); // a.hi clmul 27
    // Build the lookup table
    let table_bytes: [u8; 16] = [
        0, 27, 54, 45, 108, 119, 90, 65, 216, 195, 238, 245, 180, 175, 130, 153,
    ];
    let table = bytes_to_m128i(table_bytes);
    // _mm_srli_si128(q2, 8): lo = q2.hi, hi = 0
    let indices = M128i { lo: q2.hi, hi: 0 };
    let q3 = shuffle_epi8(table, indices);
    let q4 = q2.xor(a);
    q3.xor(q4)
}

fn precomp_reduction_64(a: M128i) -> u64 {
    precomp_reduction_64_si128(a).lo
}

// Combine value with key, multiply, and add length-hash
fn simple_128_to_64_hash_with_length(
    value: M128i,
    key: M128i,
    keylength: u64,
    length: u64,
) -> u64 {
    let add = value.xor(key);
    let clprod1 = clmul_imm(add, add, 0x10);
    let total = clprod1.xor(lazy_length_hash(keylength, length));
    precomp_reduction_64(total)
}

// Process complete blocks of `length` words (length expected to be divisible by 4)
fn clmul_half_scalar_product_without_reduction(
    random: &[u8],
    stringbyte: &[u8],
    string_word_start: usize,
    length: usize,
) -> M128i {
    let mut acc = M128i::zero();
    let mut rs_m128_idx = 0usize;
    let mut s_word_idx = string_word_start;
    let endstring = string_word_start + length;
    while s_word_idx + 3 < endstring {
        let temp1 = load_random_m128i(random, rs_m128_idx);
        let temp2 = load_string_m128i(stringbyte, s_word_idx);
        let add1 = temp1.xor(temp2);
        let clprod1 = clmul_imm(add1, add1, 0x10);
        acc = clprod1.xor(acc);

        let temp12 = load_random_m128i(random, rs_m128_idx + 1);
        let temp22 = load_string_m128i(stringbyte, s_word_idx + 2);
        let add12 = temp12.xor(temp22);
        let clprod12 = clmul_imm(add12, add12, 0x10);
        acc = clprod12.xor(acc);

        rs_m128_idx += 2;
        s_word_idx += 4;
    }
    acc
}

// Process up to `length` words; handles partial trailing words (1, 2, or 3)
fn clmul_half_scalar_product_with_tail_without_reduction(
    random: &[u8],
    stringbyte: &[u8],
    string_word_start: usize,
    length: usize,
) -> M128i {
    let mut acc = M128i::zero();
    let mut rs_m128_idx = 0usize;
    let mut s_word_idx = string_word_start;
    let endstring = string_word_start + length;
    while s_word_idx + 3 < endstring {
        let temp1 = load_random_m128i(random, rs_m128_idx);
        let temp2 = load_string_m128i(stringbyte, s_word_idx);
        let add1 = temp1.xor(temp2);
        let clprod1 = clmul_imm(add1, add1, 0x10);
        acc = clprod1.xor(acc);

        let temp12 = load_random_m128i(random, rs_m128_idx + 1);
        let temp22 = load_string_m128i(stringbyte, s_word_idx + 2);
        let add12 = temp12.xor(temp22);
        let clprod12 = clmul_imm(add12, add12, 0x10);
        acc = clprod12.xor(acc);

        rs_m128_idx += 2;
        s_word_idx += 4;
    }
    if s_word_idx + 1 < endstring {
        let temp1 = load_random_m128i(random, rs_m128_idx);
        let temp2 = load_string_m128i(stringbyte, s_word_idx);
        let add1 = temp1.xor(temp2);
        let clprod1 = clmul_imm(add1, add1, 0x10);
        acc = clprod1.xor(acc);
        rs_m128_idx += 1;
        s_word_idx += 2;
    }
    if s_word_idx < endstring {
        let temp1 = load_random_m128i(random, rs_m128_idx);
        // _mm_loadl_epi64: load 8 bytes into lo, hi = 0
        let temp2 = M128i {
            lo: load_string_word(stringbyte, s_word_idx),
            hi: 0,
        };
        let add1 = temp1.xor(temp2);
        let clprod1 = clmul_imm(add1, add1, 0x10);
        acc = clprod1.xor(acc);
    }
    acc
}

// Same as above but appends an extra word at the end (for the partial-byte tail)
fn clmul_half_scalar_product_with_tail_with_extra_word(
    random: &[u8],
    stringbyte: &[u8],
    string_word_start: usize,
    length: usize,
    extraword: u64,
) -> M128i {
    let mut acc = M128i::zero();
    let mut rs_m128_idx = 0usize;
    let mut s_word_idx = string_word_start;
    let endstring = string_word_start + length;
    while s_word_idx + 3 < endstring {
        let temp1 = load_random_m128i(random, rs_m128_idx);
        let temp2 = load_string_m128i(stringbyte, s_word_idx);
        let add1 = temp1.xor(temp2);
        let clprod1 = clmul_imm(add1, add1, 0x10);
        acc = clprod1.xor(acc);

        let temp12 = load_random_m128i(random, rs_m128_idx + 1);
        let temp22 = load_string_m128i(stringbyte, s_word_idx + 2);
        let add12 = temp12.xor(temp22);
        let clprod12 = clmul_imm(add12, add12, 0x10);
        acc = clprod12.xor(acc);

        rs_m128_idx += 2;
        s_word_idx += 4;
    }
    if s_word_idx + 1 < endstring {
        let temp1 = load_random_m128i(random, rs_m128_idx);
        let temp2 = load_string_m128i(stringbyte, s_word_idx);
        let add1 = temp1.xor(temp2);
        let clprod1 = clmul_imm(add1, add1, 0x10);
        acc = clprod1.xor(acc);
        rs_m128_idx += 1;
        s_word_idx += 2;
    }
    if s_word_idx < endstring {
        // Append: lo = *string, hi = extraword
        let temp1 = load_random_m128i(random, rs_m128_idx);
        let temp2 = M128i {
            lo: load_string_word(stringbyte, s_word_idx),
            hi: extraword,
        };
        let add1 = temp1.xor(temp2);
        let clprod1 = clmul_imm(add1, add1, 0x10);
        acc = clprod1.xor(acc);
    } else {
        // Only extra word, lo = extraword, hi = 0
        let temp1 = load_random_m128i(random, rs_m128_idx);
        let temp2 = M128i {
            lo: extraword,
            hi: 0,
        };
        let add1 = temp1.xor(temp2);
        // 0x01 selects: a.hi * b.lo (here add1.hi clmul add1.lo)
        let clprod1 = clmul_imm(add1, add1, 0x01);
        acc = clprod1.xor(acc);
    }
    acc
}

fn clmul_half_scalar_product_only_extra_word(random: &[u8], extraword: u64) -> M128i {
    let temp1 = load_random_m128i(random, 0);
    let temp2 = M128i {
        lo: extraword,
        hi: 0,
    };
    let add1 = temp1.xor(temp2);
    clmul_imm(add1, add1, 0x01)
}

// Read partial trailing bytes from string into a u64 (zero-padded high bytes)
fn create_last_word(stringbyte: &[u8], length_words: usize) -> u64 {
    let lengthbyte = stringbyte.len();
    let off = length_words * 8;
    let mut bytes = [0u8; 8];
    let take = lengthbyte - off;
    bytes[..take].copy_from_slice(&stringbyte[off..lengthbyte]);
    u64::from_le_bytes(bytes)
}

pub fn clhash(random: &[u8], stringbyte: &[u8]) -> u64 {
    let lengthbyte = stringbyte.len();
    let m: usize = 128; // u64 words per block
    let m128_needed_per_block = m / 2; // = 64

    // Load polyvalue from random[m128_needed_per_block]
    let mut polyvalue = load_random_m128i(random, m128_needed_per_block);
    // Mask: lo = 0xFFFF_FFFF_FFFF_FFFF, hi = 0x3FFF_FFFF_FFFF_FFFF (zero top 2 bits of hi)
    polyvalue.lo &= 0xFFFF_FFFF_FFFF_FFFFu64;
    polyvalue.hi &= 0x3FFF_FFFF_FFFF_FFFFu64;

    let length = lengthbyte / 8; // # complete words
    let lengthinc = (lengthbyte + 7) / 8; // # words including partial

    if m < lengthinc {
        // long strings
        let mut acc = clmul_half_scalar_product_without_reduction(random, stringbyte, 0, m);
        let mut t = m;
        while t + m <= length {
            acc = mul128by128to128_lazymod127(polyvalue, acc);
            let h1 = clmul_half_scalar_product_without_reduction(random, stringbyte, t, m);
            acc = acc.xor(h1);
            t += m;
        }
        let remain = length - t; // number of completely filled words

        if remain != 0 {
            acc = mul128by128to128_lazymod127(polyvalue, acc);
            if lengthbyte % 8 == 0 {
                let h1 = clmul_half_scalar_product_with_tail_without_reduction(
                    random, stringbyte, t, remain,
                );
                acc = acc.xor(h1);
            } else {
                let lastword = create_last_word(stringbyte, length);
                let h1 = clmul_half_scalar_product_with_tail_with_extra_word(
                    random, stringbyte, t, remain, lastword,
                );
                acc = acc.xor(h1);
            }
        } else if lengthbyte % 8 != 0 {
            // No complete words left, but a partial word remains
            acc = mul128by128to128_lazymod127(polyvalue, acc);
            let lastword = create_last_word(stringbyte, length);
            let h1 = clmul_half_scalar_product_only_extra_word(random, lastword);
            acc = acc.xor(h1);
        }

        let finalkey = load_random_m128i(random, m128_needed_per_block + 1);
        // keylength is at u64 word index: 2 * (m128_needed_per_block + 2) = 132
        let keylength = load_random_u64(random, 2 * (m128_needed_per_block + 2));
        simple_128_to_64_hash_with_length(acc, finalkey, keylength, lengthbyte as u64)
    } else {
        // short strings
        if lengthbyte % 8 == 0 {
            let mut acc = clmul_half_scalar_product_with_tail_without_reduction(
                random, stringbyte, 0, length,
            );
            let keylength = load_random_u64(random, 2 * (m128_needed_per_block + 2));
            acc = acc.xor(lazy_length_hash(keylength, lengthbyte as u64));
            return precomp_reduction_64(acc);
        }
        let lastword = create_last_word(stringbyte, length);
        let mut acc = clmul_half_scalar_product_with_tail_with_extra_word(
            random, stringbyte, 0, length, lastword,
        );
        let keylength = load_random_u64(random, 2 * (m128_needed_per_block + 2));
        acc = acc.xor(lazy_length_hash(keylength, lengthbyte as u64));
        precomp_reduction_64(acc)
    }
}

// xorshift128+ random number generator step
fn xorshift128plus(part1: &mut u64, part2: &mut u64) -> u64 {
    let mut s1 = *part1;
    let s0 = *part2;
    *part1 = s0;
    s1 ^= s1 << 23;
    let new_part2 = s1 ^ s0 ^ (s1 >> 18) ^ (s0 >> 5);
    *part2 = new_part2;
    new_part2.wrapping_add(s0)
}

pub fn get_random_key_for_clhash(seed1: u64, seed2: u64) -> Vec<u8> {
    let mut part1 = seed1;
    let mut part2 = seed2;
    let mut a64 = [0u64; RANDOM_64BITWORDS_NEEDED_FOR_CLHASH];
    for i in 0..RANDOM_64BITWORDS_NEEDED_FOR_CLHASH {
        a64[i] = xorshift128plus(&mut part1, &mut part2);
    }
    while a64[128] == 0 && a64[129] == 1 {
        a64[128] = xorshift128plus(&mut part1, &mut part2);
        a64[129] = xorshift128plus(&mut part1, &mut part2);
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
        // Vec<u8> automatically frees its allocation; no explicit cleanup needed.
    }
}
