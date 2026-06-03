pub const RANDOM_64BITWORDS_NEEDED_FOR_CLHASH: usize = 133;
pub const RANDOM_BYTES_NEEDED_FOR_CLHASH: usize = 133 * 8;

// ---------------------------------------------------------------------------
// Internal "pseudo __m128i" representation: two little-endian u64 lanes.
// ---------------------------------------------------------------------------

#[derive(Copy, Clone, Debug, Default)]
struct M128i {
    low: u64,
    high: u64,
}

impl M128i {
    const fn zero() -> Self {
        M128i { low: 0, high: 0 }
    }
}

// Carry-less multiplication of two 64-bit values, producing 128 bits.
fn clmul64(a: u64, b: u64) -> M128i {
    let mut result: u128 = 0;
    let aa = a as u128;
    let mut bb = b;
    let mut i: u32 = 0;
    while bb != 0 {
        if (bb & 1) == 1 {
            result ^= aa << i;
        }
        bb >>= 1;
        i += 1;
    }
    M128i {
        low: result as u64,
        high: (result >> 64) as u64,
    }
}

fn xor(a: M128i, b: M128i) -> M128i {
    M128i {
        low: a.low ^ b.low,
        high: a.high ^ b.high,
    }
}

fn and(a: M128i, b: M128i) -> M128i {
    M128i {
        low: a.low & b.low,
        high: a.high & b.high,
    }
}

fn or(a: M128i, b: M128i) -> M128i {
    M128i {
        low: a.low | b.low,
        high: a.high | b.high,
    }
}

// Shift each 64-bit lane left by n bits (n in 0..64).
fn slli_epi64(a: M128i, n: u32) -> M128i {
    M128i {
        low: a.low << n,
        high: a.high << n,
    }
}

// Shift each 64-bit lane right by n bits (n in 0..64).
fn srli_epi64(a: M128i, n: u32) -> M128i {
    M128i {
        low: a.low >> n,
        high: a.high >> n,
    }
}

// Shift the entire 128-bit value left by n bytes.
fn slli_si128(a: M128i, n_bytes: u32) -> M128i {
    if n_bytes >= 16 {
        return M128i::zero();
    }
    let v: u128 = (a.low as u128) | ((a.high as u128) << 64);
    let shifted = v << (n_bytes * 8);
    M128i {
        low: shifted as u64,
        high: (shifted >> 64) as u64,
    }
}

// Shift the entire 128-bit value right by n bytes.
fn srli_si128(a: M128i, n_bytes: u32) -> M128i {
    if n_bytes >= 16 {
        return M128i::zero();
    }
    let v: u128 = (a.low as u128) | ((a.high as u128) << 64);
    let shifted = v >> (n_bytes * 8);
    M128i {
        low: shifted as u64,
        high: (shifted >> 64) as u64,
    }
}

// _mm_setr_epi8: byte 0 is at the lowest position.
fn setr_epi8(b: [u8; 16]) -> M128i {
    let mut low: u64 = 0;
    let mut high: u64 = 0;
    for i in 0..8 {
        low |= (b[i] as u64) << (i * 8);
    }
    for i in 0..8 {
        high |= (b[i + 8] as u64) << (i * 8);
    }
    M128i { low, high }
}

// _mm_setr_epi32: 32-bit lane 0 is at the lowest position.
fn setr_epi32(a: u32, b: u32, c: u32, d: u32) -> M128i {
    M128i {
        low: (a as u64) | ((b as u64) << 32),
        high: (c as u64) | ((d as u64) << 32),
    }
}

// _mm_shuffle_epi8: byte-wise permute. If the mask byte's high bit is set,
// the result byte is 0; otherwise the low 4 bits of the mask byte are used as
// an index into `a`.
fn shuffle_epi8(a: M128i, mask: M128i) -> M128i {
    let a_bytes = m128i_to_bytes(a);
    let m_bytes = m128i_to_bytes(mask);
    let mut result = [0u8; 16];
    for i in 0..16 {
        let m = m_bytes[i];
        if m & 0x80 != 0 {
            result[i] = 0;
        } else {
            result[i] = a_bytes[(m & 0x0F) as usize];
        }
    }
    setr_epi8(result)
}

fn m128i_to_bytes(a: M128i) -> [u8; 16] {
    let mut b = [0u8; 16];
    b[0..8].copy_from_slice(&a.low.to_le_bytes());
    b[8..16].copy_from_slice(&a.high.to_le_bytes());
    b
}

// ---------------------------------------------------------------------------
// Memory loads. Inputs are byte slices, treated as little-endian (matches the
// way x86 reads them through __m128i*/uint64_t* casts).
// ---------------------------------------------------------------------------

fn read_u64_le(bytes: &[u8], byte_idx: usize) -> u64 {
    let mut buf = [0u8; 8];
    buf.copy_from_slice(&bytes[byte_idx..byte_idx + 8]);
    u64::from_le_bytes(buf)
}

fn read_m128i_le(bytes: &[u8], byte_idx: usize) -> M128i {
    M128i {
        low: read_u64_le(bytes, byte_idx),
        high: read_u64_le(bytes, byte_idx + 8),
    }
}

// ---------------------------------------------------------------------------
// Hash building blocks (mirroring clhash.c).
// ---------------------------------------------------------------------------

// computes a << 1 across the 128-bit value
fn leftshift1(a: M128i) -> M128i {
    let u64shift = slli_epi64(a, 1);
    let topbits = slli_si128(srli_epi64(a, 63), 8);
    or(u64shift, topbits)
}

// computes a << 2 across the 128-bit value
fn leftshift2(a: M128i) -> M128i {
    let u64shift = slli_epi64(a, 2);
    let topbits = slli_si128(srli_epi64(a, 62), 8);
    or(u64shift, topbits)
}

fn lazymod127(alow: M128i, ahigh: M128i) -> M128i {
    let shift1 = leftshift1(ahigh);
    let shift2 = leftshift2(ahigh);
    xor(xor(alow, shift1), shift2)
}

fn mul128by128to128_lazymod127(a: M128i, b: M128i) -> M128i {
    let amix1 = clmul64(a.high, b.low); // imm 0x01
    let amix2 = clmul64(a.low, b.high); // imm 0x10
    let alow0 = clmul64(a.low, b.low); // imm 0x00
    let ahigh0 = clmul64(a.high, b.high); // imm 0x11
    let amix = xor(amix1, amix2);
    let amix1s = slli_si128(amix, 8);
    let amix2s = srli_si128(amix, 8);
    let alow = xor(alow0, amix1s);
    let ahigh = xor(ahigh0, amix2s);
    lazymod127(alow, ahigh)
}

fn lazy_length_hash(keylength: u64, length: u64) -> M128i {
    // _mm_set_epi64x(keylength, length): low=length, high=keylength
    // _mm_clmulepi64_si128(v, v, 0x10) = clmul(v.low, v.high)
    clmul64(length, keylength)
}

fn precomp_reduction64_si128(a: M128i) -> M128i {
    // C is _mm_cvtsi64_si128((1U<<4)+(1U<<3)+(1U<<1)+(1U<<0)) = 27 in low lane
    // Q2 = clmul(A, C, 0x01) = clmul(A.high, C.low) = clmul(A.high, 27)
    let q2 = clmul64(a.high, 27);
    let table = setr_epi8([
        0, 27, 54, 45, 108, 119, 90, 65, 216, 195, 238, 245, 180, 175, 130, 153,
    ]);
    let q3 = shuffle_epi8(table, srli_si128(q2, 8));
    let q4 = xor(q2, a);
    xor(q3, q4)
}

fn precomp_reduction64(a: M128i) -> u64 {
    precomp_reduction64_si128(a).low
}

fn simple128to64_hash_with_length(value: M128i, key: M128i, keylength: u64, length: u64) -> u64 {
    let add = xor(value, key);
    // _mm_clmulepi64_si128(add, add, 0x10) = clmul(add.low, add.high)
    let clprod1 = clmul64(add.low, add.high);
    let total = xor(clprod1, lazy_length_hash(keylength, length));
    precomp_reduction64(total)
}

// ---------------------------------------------------------------------------
// Inner products. `string_offset` is given in u64 words; `length` is also in
// u64 words. The random source is always indexed from byte 0.
// ---------------------------------------------------------------------------

fn clmul_half_scalar_product_no_red(
    random: &[u8],
    stringbyte: &[u8],
    string_offset: usize,
    length: usize,
) -> M128i {
    let mut acc = M128i::zero();
    let mut rs_idx: usize = 0;
    let mut s_idx: usize = string_offset;
    let endstring = string_offset + length;
    while s_idx + 4 <= endstring {
        let temp1 = read_m128i_le(random, rs_idx * 16);
        let temp2 = read_m128i_le(stringbyte, s_idx * 8);
        let add1 = xor(temp1, temp2);
        let clprod1 = clmul64(add1.low, add1.high);
        acc = xor(clprod1, acc);

        let temp12 = read_m128i_le(random, (rs_idx + 1) * 16);
        let temp22 = read_m128i_le(stringbyte, (s_idx + 2) * 8);
        let add12 = xor(temp12, temp22);
        let clprod12 = clmul64(add12.low, add12.high);
        acc = xor(clprod12, acc);

        rs_idx += 2;
        s_idx += 4;
    }
    acc
}

fn clmul_half_scalar_product_with_tail_no_red(
    random: &[u8],
    stringbyte: &[u8],
    string_offset: usize,
    length: usize,
) -> M128i {
    let mut acc = M128i::zero();
    let mut rs_idx: usize = 0;
    let mut s_idx: usize = string_offset;
    let endstring = string_offset + length;
    while s_idx + 4 <= endstring {
        let temp1 = read_m128i_le(random, rs_idx * 16);
        let temp2 = read_m128i_le(stringbyte, s_idx * 8);
        let add1 = xor(temp1, temp2);
        let clprod1 = clmul64(add1.low, add1.high);
        acc = xor(clprod1, acc);

        let temp12 = read_m128i_le(random, (rs_idx + 1) * 16);
        let temp22 = read_m128i_le(stringbyte, (s_idx + 2) * 8);
        let add12 = xor(temp12, temp22);
        let clprod12 = clmul64(add12.low, add12.high);
        acc = xor(clprod12, acc);

        rs_idx += 2;
        s_idx += 4;
    }
    if s_idx + 2 <= endstring {
        let temp1 = read_m128i_le(random, rs_idx * 16);
        let temp2 = read_m128i_le(stringbyte, s_idx * 8);
        let add1 = xor(temp1, temp2);
        let clprod1 = clmul64(add1.low, add1.high);
        acc = xor(clprod1, acc);
        rs_idx += 1;
        s_idx += 2;
    }
    if s_idx < endstring {
        let temp1 = read_m128i_le(random, rs_idx * 16);
        // _mm_loadl_epi64: low = next u64, high = 0
        let temp2 = M128i {
            low: read_u64_le(stringbyte, s_idx * 8),
            high: 0,
        };
        let add1 = xor(temp1, temp2);
        let clprod1 = clmul64(add1.low, add1.high);
        acc = xor(clprod1, acc);
    }
    acc
}

fn clmul_half_scalar_product_with_tail_no_red_with_extra_word(
    random: &[u8],
    stringbyte: &[u8],
    string_offset: usize,
    length: usize,
    extraword: u64,
) -> M128i {
    let mut acc = M128i::zero();
    let mut rs_idx: usize = 0;
    let mut s_idx: usize = string_offset;
    let endstring = string_offset + length;
    while s_idx + 4 <= endstring {
        let temp1 = read_m128i_le(random, rs_idx * 16);
        let temp2 = read_m128i_le(stringbyte, s_idx * 8);
        let add1 = xor(temp1, temp2);
        let clprod1 = clmul64(add1.low, add1.high);
        acc = xor(clprod1, acc);

        let temp12 = read_m128i_le(random, (rs_idx + 1) * 16);
        let temp22 = read_m128i_le(stringbyte, (s_idx + 2) * 8);
        let add12 = xor(temp12, temp22);
        let clprod12 = clmul64(add12.low, add12.high);
        acc = xor(clprod12, acc);

        rs_idx += 2;
        s_idx += 4;
    }
    if s_idx + 2 <= endstring {
        let temp1 = read_m128i_le(random, rs_idx * 16);
        let temp2 = read_m128i_le(stringbyte, s_idx * 8);
        let add1 = xor(temp1, temp2);
        let clprod1 = clmul64(add1.low, add1.high);
        acc = xor(clprod1, acc);
        rs_idx += 1;
        s_idx += 2;
    }
    if s_idx < endstring {
        // temp2 = _mm_set_epi64x(extraword, *string): low=*string, high=extraword
        let temp1 = read_m128i_le(random, rs_idx * 16);
        let temp2 = M128i {
            low: read_u64_le(stringbyte, s_idx * 8),
            high: extraword,
        };
        let add1 = xor(temp1, temp2);
        let clprod1 = clmul64(add1.low, add1.high);
        acc = xor(clprod1, acc);
    } else {
        let temp1 = read_m128i_le(random, rs_idx * 16);
        // _mm_loadl_epi64(&extraword): low=extraword, high=0
        let temp2 = M128i {
            low: extraword,
            high: 0,
        };
        let add1 = xor(temp1, temp2);
        // _mm_clmulepi64_si128(add1, add1, 0x01) = clmul(add1.high, add1.low)
        let clprod1 = clmul64(add1.high, add1.low);
        acc = xor(clprod1, acc);
    }
    acc
}

fn clmul_half_scalar_product_only_extra_word(random: &[u8], extraword: u64) -> M128i {
    let temp1 = read_m128i_le(random, 0);
    let temp2 = M128i {
        low: extraword,
        high: 0,
    };
    let add1 = xor(temp1, temp2);
    // imm = 0x01: clmul(add1.high, add1.low)
    clmul64(add1.high, add1.low)
}

// Construct the trailing partial word from the last (lengthbyte % 8) bytes,
// zero-padded to a u64 (little-endian).
fn create_last_word(stringbyte: &[u8], lengthbyte: usize) -> u64 {
    let significantbytes = lengthbyte % 8;
    if significantbytes == 0 {
        return 0;
    }
    let length = lengthbyte / 8;
    let start = length * 8;
    let mut buf = [0u8; 8];
    buf[..significantbytes].copy_from_slice(&stringbyte[start..start + significantbytes]);
    u64::from_le_bytes(buf)
}

// ---------------------------------------------------------------------------
// Public hash function.
// ---------------------------------------------------------------------------

pub fn clhash(random: &[u8], stringbyte: &[u8]) -> u64 {
    assert!(
        random.len() >= RANDOM_BYTES_NEEDED_FOR_CLHASH,
        "random key too small"
    );
    let lengthbyte = stringbyte.len();
    let m: usize = 128; // process data in chunks of 16 cache lines
    let m128_per_block: usize = m / 2; // 64

    // polyvalue = random[1024..1040] AND mask (clear top 2 bits)
    let mut polyvalue = read_m128i_le(random, m128_per_block * 16);
    polyvalue = and(
        polyvalue,
        setr_epi32(0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0x3FFFFFFF),
    );

    let length = lengthbyte / 8; // # of complete u64 words
    let lengthinc = (lengthbyte + 7) / 8; // # of words including partial

    if m < lengthinc {
        // long strings
        let mut acc = clmul_half_scalar_product_no_red(random, stringbyte, 0, m);
        let mut t = m;
        while t + m <= length {
            acc = mul128by128to128_lazymod127(polyvalue, acc);
            let h1 = clmul_half_scalar_product_no_red(random, stringbyte, t, m);
            acc = xor(acc, h1);
            t += m;
        }
        let remain = length - t; // # of completely filled words left
        if remain != 0 {
            acc = mul128by128to128_lazymod127(polyvalue, acc);
            if lengthbyte % 8 == 0 {
                let h1 =
                    clmul_half_scalar_product_with_tail_no_red(random, stringbyte, t, remain);
                acc = xor(acc, h1);
            } else {
                let lastword = create_last_word(stringbyte, lengthbyte);
                let h1 = clmul_half_scalar_product_with_tail_no_red_with_extra_word(
                    random, stringbyte, t, remain, lastword,
                );
                acc = xor(acc, h1);
            }
        } else if lengthbyte % 8 != 0 {
            acc = mul128by128to128_lazymod127(polyvalue, acc);
            let lastword = create_last_word(stringbyte, lengthbyte);
            let h1 = clmul_half_scalar_product_only_extra_word(random, lastword);
            acc = xor(acc, h1);
        }

        let finalkey = read_m128i_le(random, (m128_per_block + 1) * 16);
        let keylength = read_u64_le(random, (m128_per_block + 2) * 16);
        simple128to64_hash_with_length(acc, finalkey, keylength, lengthbyte as u64)
    } else {
        // short strings
        let keylength = read_u64_le(random, (m128_per_block + 2) * 16);
        if lengthbyte % 8 == 0 {
            let mut acc =
                clmul_half_scalar_product_with_tail_no_red(random, stringbyte, 0, length);
            acc = xor(acc, lazy_length_hash(keylength, lengthbyte as u64));
            precomp_reduction64(acc)
        } else {
            let lastword = create_last_word(stringbyte, lengthbyte);
            let mut acc = clmul_half_scalar_product_with_tail_no_red_with_extra_word(
                random, stringbyte, 0, length, lastword,
            );
            acc = xor(acc, lazy_length_hash(keylength, lengthbyte as u64));
            precomp_reduction64(acc)
        }
    }
}

// ---------------------------------------------------------------------------
// xorshift128+ PRNG to seed the random key.
// ---------------------------------------------------------------------------

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

    let mut bytes = Vec::with_capacity(RANDOM_BYTES_NEEDED_FOR_CLHASH);
    for w in &a64 {
        bytes.extend_from_slice(&w.to_le_bytes());
    }
    bytes
}

// ---------------------------------------------------------------------------
// ClHasher: convenience type wrapping a random key.
// ---------------------------------------------------------------------------

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
        // The Vec owns its allocation and frees it automatically.
    }
}
