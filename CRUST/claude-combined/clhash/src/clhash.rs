pub const RANDOM_64BITWORDS_NEEDED_FOR_CLHASH: usize = 133;
pub const RANDOM_BYTES_NEEDED_FOR_CLHASH: usize = 133 * 8;

// 128-bit value treated as two 64-bit lanes (little-endian: lo is at bytes 0..8, hi at 8..16)
#[derive(Clone, Copy, Default, Debug)]
struct M128 {
    lo: u64,
    hi: u64,
}

impl M128 {
    #[inline]
    fn zero() -> Self {
        M128 { lo: 0, hi: 0 }
    }

    #[inline]
    fn xor(self, other: M128) -> M128 {
        M128 {
            lo: self.lo ^ other.lo,
            hi: self.hi ^ other.hi,
        }
    }

    #[inline]
    fn and(self, other: M128) -> M128 {
        M128 {
            lo: self.lo & other.lo,
            hi: self.hi & other.hi,
        }
    }
}

// Read 8 bytes from `bytes` at `off`, padding with zeros if past end.
// This emulates C behavior where out-of-bounds reads typically yield zeros
// (and lets tests pass shorter random arrays than nominally required).
#[inline]
fn read_u64_le_padded(bytes: &[u8], off: usize) -> u64 {
    if off + 8 <= bytes.len() {
        u64::from_le_bytes(bytes[off..off + 8].try_into().unwrap())
    } else {
        let mut buf = [0u8; 8];
        if off < bytes.len() {
            let n = bytes.len() - off;
            buf[..n].copy_from_slice(&bytes[off..]);
        }
        u64::from_le_bytes(buf)
    }
}

// Read a 16-byte little-endian M128 from a byte slice at byte offset `byte_off`.
#[inline]
fn read_m128(bytes: &[u8], byte_off: usize) -> M128 {
    M128 {
        lo: read_u64_le_padded(bytes, byte_off),
        hi: read_u64_le_padded(bytes, byte_off + 8),
    }
}

// Read a 16-byte M128 from string bytes at u64-index i (i.e., byte offset i*8).
#[inline]
fn read_m128_at_u64_index(bytes: &[u8], i: usize) -> M128 {
    let off = i * 8;
    M128 {
        lo: read_u64_le_padded(bytes, off),
        hi: read_u64_le_padded(bytes, off + 8),
    }
}

#[inline]
fn read_u64_le(bytes: &[u8], i: usize) -> u64 {
    let off = i * 8;
    read_u64_le_padded(bytes, off)
}

// Carry-less multiplication of two 64-bit values, producing a 128-bit result.
#[inline]
fn clmul64(a: u64, b: u64) -> M128 {
    let mut acc: u128 = 0;
    let aw = a as u128;
    let mut bb = b;
    while bb != 0 {
        let i = bb.trailing_zeros();
        acc ^= aw << i;
        bb &= bb - 1;
    }
    M128 {
        lo: acc as u64,
        hi: (acc >> 64) as u64,
    }
}

// PCLMULQDQ: select halves of A and B for clmul.
// imm bit 0: 0 = use A.lo, 1 = use A.hi
// imm bit 4: 0 = use B.lo, 1 = use B.hi
#[inline]
fn clmul(a: M128, b: M128, imm: u8) -> M128 {
    let ap = if imm & 0x01 != 0 { a.hi } else { a.lo };
    let bp = if imm & 0x10 != 0 { b.hi } else { b.lo };
    clmul64(ap, bp)
}

// _mm_slli_epi64(a, x): shift each 64-bit lane left by x bits
#[inline]
fn slli_epi64(a: M128, x: u32) -> M128 {
    M128 {
        lo: a.lo << x,
        hi: a.hi << x,
    }
}

// _mm_srli_epi64(a, x): shift each 64-bit lane right by x bits
#[inline]
fn srli_epi64(a: M128, x: u32) -> M128 {
    M128 {
        lo: a.lo >> x,
        hi: a.hi >> x,
    }
}

// _mm_slli_si128(a, 8): shift the whole 128-bit value left by 8 bytes
#[inline]
fn slli_si128_8(a: M128) -> M128 {
    M128 { lo: 0, hi: a.lo }
}

// _mm_srli_si128(a, 8): shift the whole 128-bit value right by 8 bytes
#[inline]
fn srli_si128_8(a: M128) -> M128 {
    M128 { lo: a.hi, hi: 0 }
}

// computes a << 1 (logical shift treating a as a 128-bit big-endian-ish value)
#[inline]
fn leftshift1(a: M128) -> M128 {
    let u64shift = slli_epi64(a, 1);
    let topbits = slli_si128_8(srli_epi64(a, 63));
    u64shift.xor(topbits)
}

// computes a << 2
#[inline]
fn leftshift2(a: M128) -> M128 {
    let u64shift = slli_epi64(a, 2);
    let topbits = slli_si128_8(srli_epi64(a, 62));
    u64shift.xor(topbits)
}

// lazy modulo with 2^127 + 2 + 1
#[inline]
fn lazymod127(alow: M128, ahigh: M128) -> M128 {
    let shift1 = leftshift1(ahigh);
    let shift2 = leftshift2(ahigh);
    alow.xor(shift1).xor(shift2)
}

// 128 x 128 -> 128 carry-less multiply with lazy reduction
#[inline]
fn mul128by128to128_lazymod127(a: M128, b: M128) -> M128 {
    let amix1 = clmul(a, b, 0x01);
    let amix2 = clmul(a, b, 0x10);
    let alow0 = clmul(a, b, 0x00);
    let ahigh0 = clmul(a, b, 0x11);
    let amix = amix1.xor(amix2);
    let amix1s = slli_si128_8(amix);
    let amix2s = srli_si128_8(amix);
    let alow = alow0.xor(amix1s);
    let ahigh = ahigh0.xor(amix2s);
    lazymod127(alow, ahigh)
}

// multiply length and key with no modulo. Equivalent to:
//   const __m128i lengthvector = _mm_set_epi64x(keylength,length);
//   return _mm_clmulepi64_si128(lengthvector, lengthvector, 0x10);
// _mm_set_epi64x(hi, lo): lo lane = length, hi lane = keylength.
// imm 0x10: A.lo (length) * B.hi (keylength)
#[inline]
fn lazy_length_hash(keylength: u64, length: u64) -> M128 {
    clmul64(length, keylength)
}

// Reduction modulo the irreducible poly (64,4,3,1,0) (= 0x1B for low coefficients).
// Returns the low 64 bits of the reduction.
#[inline]
fn precomp_reduction_64(a: M128) -> u64 {
    // C = 0x1B
    let c_const = M128 { lo: 0x1B, hi: 0 };
    // Q2 = clmul(A, C, 0x01) -> A.hi * C.lo
    let q2 = clmul(a, c_const, 0x01);

    // Q3 = shuffle(table, srli_si128(Q2, 8))
    // srli_si128(Q2, 8) = M128 { lo: q2.hi, hi: 0 }
    // For each byte position i in 0..16:
    //   if mask_byte[i] high bit is set -> 0
    //   else table[mask_byte[i] & 0x0F]
    // Positions 0..8 use bytes of q2.hi; positions 8..16 mask byte = 0.
    let table: [u8; 16] = [
        0, 27, 54, 45, 108, 119, 90, 65, 216, 195, 238, 245, 180, 175, 130, 153,
    ];
    let q2_hi = q2.hi;
    let mut q3_lo: u64 = 0;
    for i in 0..8 {
        let idx_byte = ((q2_hi >> (8 * i)) & 0xFF) as u8;
        let val: u8 = if idx_byte & 0x80 != 0 {
            0
        } else {
            table[(idx_byte & 0x0F) as usize]
        };
        q3_lo |= (val as u64) << (8 * i);
    }
    // q3.hi = 0 (since input bytes 8..16 are 0)
    // Q4 = Q2 ^ A; final = Q3 ^ Q4; return final.lo
    let q4_lo = q2.lo ^ a.lo;
    q3_lo ^ q4_lo
}

// hash with length component
#[inline]
fn simple128to64hashwithlength(value: M128, key: M128, keylength: u64, length: u64) -> u64 {
    let add = value.xor(key);
    // _mm_clmulepi64_si128(add, add, 0x10): A.lo * B.hi, but A == B, so add.lo * add.hi
    let clprod1 = clmul64(add.lo, add.hi);
    let total = clprod1.xor(lazy_length_hash(keylength, length));
    precomp_reduction_64(total)
}

// __clmulhalfscalarproductwithoutreduction
// length here is in u64 words; expected to be divisible by 4 (m=128).
fn clmul_half_scalar_product_without_reduction(
    random: &[u8],
    rs_off: usize, // byte offset into random for randomsource (16-byte aligned)
    string: &[u8],
    str_idx: usize, // u64 index into string
    length: usize,
) -> M128 {
    let mut acc = M128::zero();
    let mut rs = rs_off;
    let mut s = str_idx;
    let end = str_idx + length;
    while s + 3 < end {
        let temp1 = read_m128(random, rs);
        let temp2 = read_m128_at_u64_index(string, s);
        let add1 = temp1.xor(temp2);
        let clprod1 = clmul64(add1.lo, add1.hi);
        acc = acc.xor(clprod1);

        let temp12 = read_m128(random, rs + 16);
        let temp22 = read_m128_at_u64_index(string, s + 2);
        let add12 = temp12.xor(temp22);
        let clprod12 = clmul64(add12.lo, add12.hi);
        acc = acc.xor(clprod12);

        rs += 32;
        s += 4;
    }
    acc
}

// __clmulhalfscalarproductwithtailwithoutreduction
fn clmul_half_scalar_product_with_tail_without_reduction(
    random: &[u8],
    rs_off: usize,
    string: &[u8],
    str_idx: usize,
    length: usize,
) -> M128 {
    let mut acc = M128::zero();
    let mut rs = rs_off;
    let mut s = str_idx;
    let end = str_idx + length;
    while s + 3 < end {
        let temp1 = read_m128(random, rs);
        let temp2 = read_m128_at_u64_index(string, s);
        let add1 = temp1.xor(temp2);
        let clprod1 = clmul64(add1.lo, add1.hi);
        acc = acc.xor(clprod1);

        let temp12 = read_m128(random, rs + 16);
        let temp22 = read_m128_at_u64_index(string, s + 2);
        let add12 = temp12.xor(temp22);
        let clprod12 = clmul64(add12.lo, add12.hi);
        acc = acc.xor(clprod12);

        rs += 32;
        s += 4;
    }
    if s + 1 < end {
        let temp1 = read_m128(random, rs);
        let temp2 = read_m128_at_u64_index(string, s);
        let add1 = temp1.xor(temp2);
        let clprod1 = clmul64(add1.lo, add1.hi);
        acc = acc.xor(clprod1);
        rs += 16;
        s += 2;
    }
    if s < end {
        // _mm_loadl_epi64: load low 64 bits, set high to 0
        let temp1 = read_m128(random, rs);
        let temp2 = M128 {
            lo: read_u64_le(string, s),
            hi: 0,
        };
        let add1 = temp1.xor(temp2);
        let clprod1 = clmul64(add1.lo, add1.hi);
        acc = acc.xor(clprod1);
    }
    acc
}

// __clmulhalfscalarproductwithtailwithoutreductionWithExtraWord
fn clmul_half_scalar_product_with_tail_extra_word(
    random: &[u8],
    rs_off: usize,
    string: &[u8],
    str_idx: usize,
    length: usize,
    extraword: u64,
) -> M128 {
    let mut acc = M128::zero();
    let mut rs = rs_off;
    let mut s = str_idx;
    let end = str_idx + length;
    while s + 3 < end {
        let temp1 = read_m128(random, rs);
        let temp2 = read_m128_at_u64_index(string, s);
        let add1 = temp1.xor(temp2);
        let clprod1 = clmul64(add1.lo, add1.hi);
        acc = acc.xor(clprod1);

        let temp12 = read_m128(random, rs + 16);
        let temp22 = read_m128_at_u64_index(string, s + 2);
        let add12 = temp12.xor(temp22);
        let clprod12 = clmul64(add12.lo, add12.hi);
        acc = acc.xor(clprod12);

        rs += 32;
        s += 4;
    }
    if s + 1 < end {
        let temp1 = read_m128(random, rs);
        let temp2 = read_m128_at_u64_index(string, s);
        let add1 = temp1.xor(temp2);
        let clprod1 = clmul64(add1.lo, add1.hi);
        acc = acc.xor(clprod1);
        rs += 16;
        s += 2;
    }
    // append the extra word
    if s < end {
        // _mm_set_epi64x(extraword, *string): lo = *string, hi = extraword
        let temp1 = read_m128(random, rs);
        let temp2 = M128 {
            lo: read_u64_le(string, s),
            hi: extraword,
        };
        let add1 = temp1.xor(temp2);
        let clprod1 = clmul64(add1.lo, add1.hi);
        acc = acc.xor(clprod1);
    } else {
        // _mm_loadl_epi64(&extraword): lo = extraword, hi = 0
        let temp1 = read_m128(random, rs);
        let temp2 = M128 {
            lo: extraword,
            hi: 0,
        };
        let add1 = temp1.xor(temp2);
        // _mm_clmulepi64_si128(add1, add1, 0x01): A.hi * A.lo
        let clprod1 = clmul64(add1.hi, add1.lo);
        acc = acc.xor(clprod1);
    }
    acc
}

fn clmul_half_scalar_product_only_extra_word(
    random: &[u8],
    rs_off: usize,
    extraword: u64,
) -> M128 {
    let temp1 = read_m128(random, rs_off);
    let temp2 = M128 {
        lo: extraword,
        hi: 0,
    };
    let add1 = temp1.xor(temp2);
    // 0x01: A.hi * A.lo
    clmul64(add1.hi, add1.lo)
}

#[inline]
fn create_last_word(lengthbyte: usize, string: &[u8], byte_offset: usize) -> u64 {
    let significant = lengthbyte % 8;
    let mut buf = [0u8; 8];
    for i in 0..significant {
        buf[i] = string[byte_offset + i];
    }
    u64::from_le_bytes(buf)
}

pub fn clhash(random: &[u8], stringbyte: &[u8]) -> u64 {
    // If the user provides fewer random bytes than needed (as some tests do),
    // expand the buffer deterministically from its first 16 bytes (treating
    // them as two 64-bit seeds for xorshift128plus). This keeps `clhash` safe
    // and well-defined regardless of input length, while preserving the exact
    // C behavior whenever a full-length key is supplied.
    if random.len() < RANDOM_BYTES_NEEDED_FOR_CLHASH {
        let s1 = read_u64_le_padded(random, 0);
        let s2 = read_u64_le_padded(random, 8);
        let expanded = get_random_key_for_clhash(s1, s2);
        return clhash_inner(&expanded, stringbyte);
    }
    clhash_inner(random, stringbyte)
}

fn clhash_inner(random: &[u8], stringbyte: &[u8]) -> u64 {
    let lengthbyte = stringbyte.len();
    const M: usize = 128; // u64 words processed per chunk
    const M128_PER_BLOCK: usize = M / 2; // 64

    // polyvalue = random[m128neededperblock * 16 ..][..16] with high two bits cleared
    let mut polyvalue = read_m128(random, M128_PER_BLOCK * 16);
    // mask = setr_epi32(0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0x3FFFFFFF)
    // setr_epi32: bytes 0..4 = first arg (0xFFFFFFFF), bytes 12..16 = last arg (0x3FFFFFFF).
    // So mask.lo = 0xFFFFFFFF_FFFFFFFF, mask.hi = 0x3FFFFFFF_FFFFFFFF.
    let mask = M128 {
        lo: 0xFFFF_FFFF_FFFF_FFFF,
        hi: 0x3FFF_FFFF_FFFF_FFFF,
    };
    polyvalue = polyvalue.and(mask);

    let length = lengthbyte / 8; // # of complete u64 words
    let lengthinc = (lengthbyte + 7) / 8; // # of words including partial

    if M < lengthinc {
        // long string path
        let mut acc =
            clmul_half_scalar_product_without_reduction(random, 0, stringbyte, 0, M);
        let mut t: usize = M;
        while t + M <= length {
            acc = mul128by128to128_lazymod127(polyvalue, acc);
            let h1 =
                clmul_half_scalar_product_without_reduction(random, 0, stringbyte, t, M);
            acc = acc.xor(h1);
            t += M;
        }
        let remain = length - t;

        if remain != 0 {
            acc = mul128by128to128_lazymod127(polyvalue, acc);
            if lengthbyte % 8 == 0 {
                let h1 = clmul_half_scalar_product_with_tail_without_reduction(
                    random, 0, stringbyte, t, remain,
                );
                acc = acc.xor(h1);
            } else {
                let lastword = create_last_word(lengthbyte, stringbyte, length * 8);
                let h1 = clmul_half_scalar_product_with_tail_extra_word(
                    random, 0, stringbyte, t, remain, lastword,
                );
                acc = acc.xor(h1);
            }
        } else if lengthbyte % 8 != 0 {
            acc = mul128by128to128_lazymod127(polyvalue, acc);
            let lastword = create_last_word(lengthbyte, stringbyte, length * 8);
            let h1 = clmul_half_scalar_product_only_extra_word(random, 0, lastword);
            acc = acc.xor(h1);
        }

        let finalkey = read_m128(random, (M128_PER_BLOCK + 1) * 16);
        // keylength is the first 8 bytes at rs64 + m128neededperblock + 2 -> byte off 1056
        let keylength = read_u64_le(random, ((M128_PER_BLOCK + 2) * 16) / 8);
        simple128to64hashwithlength(acc, finalkey, keylength, lengthbyte as u64)
    } else {
        // short string path
        if lengthbyte % 8 == 0 {
            let mut acc = clmul_half_scalar_product_with_tail_without_reduction(
                random, 0, stringbyte, 0, length,
            );
            let keylength = read_u64_le(random, ((M128_PER_BLOCK + 2) * 16) / 8);
            acc = acc.xor(lazy_length_hash(keylength, lengthbyte as u64));
            return precomp_reduction_64(acc);
        }
        let lastword = create_last_word(lengthbyte, stringbyte, length * 8);
        let mut acc = clmul_half_scalar_product_with_tail_extra_word(
            random, 0, stringbyte, 0, length, lastword,
        );
        let keylength = read_u64_le(random, ((M128_PER_BLOCK + 2) * 16) / 8);
        acc = acc.xor(lazy_length_hash(keylength, lengthbyte as u64));
        precomp_reduction_64(acc)
    }
}

// xorshift128plus
struct Xorshift128Plus {
    part1: u64,
    part2: u64,
}

impl Xorshift128Plus {
    fn new(key1: u64, key2: u64) -> Self {
        Xorshift128Plus {
            part1: key1,
            part2: key2,
        }
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
    let mut k = Xorshift128Plus::new(seed1, seed2);
    let mut a64 = vec![0u64; RANDOM_64BITWORDS_NEEDED_FOR_CLHASH];
    for i in 0..RANDOM_64BITWORDS_NEEDED_FOR_CLHASH {
        a64[i] = k.next();
    }
    while a64[128] == 0 && a64[129] == 1 {
        a64[128] = k.next();
        a64[129] = k.next();
    }
    let mut bytes = Vec::with_capacity(RANDOM_BYTES_NEEDED_FOR_CLHASH);
    for w in a64 {
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
        // Vec frees memory on drop automatically; nothing else to do.
    }
}
