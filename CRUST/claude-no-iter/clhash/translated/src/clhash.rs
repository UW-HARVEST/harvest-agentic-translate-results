pub const RANDOM_64BITWORDS_NEEDED_FOR_CLHASH: usize = 133;
pub const RANDOM_BYTES_NEEDED_FOR_CLHASH: usize = 133 * 8;

// ---------------------------------------------------------------------------
// Pure-Rust replacements for the x86 SSE/PCLMUL intrinsics used by the
// reference C implementation.  __m128i is modelled as a pair of u64 lanes
// (`[low, high]`) so that the bit layout matches the original code byte for
// byte.
// ---------------------------------------------------------------------------

#[derive(Copy, Clone, Default, Debug)]
struct M128 {
    lo: u64,
    hi: u64,
}

#[inline]
fn m128_zero() -> M128 {
    M128 { lo: 0, hi: 0 }
}

#[inline]
fn m128_xor(a: M128, b: M128) -> M128 {
    M128 {
        lo: a.lo ^ b.lo,
        hi: a.hi ^ b.hi,
    }
}

#[inline]
fn m128_and(a: M128, b: M128) -> M128 {
    M128 {
        lo: a.lo & b.lo,
        hi: a.hi & b.hi,
    }
}

#[inline]
fn m128_or(a: M128, b: M128) -> M128 {
    M128 {
        lo: a.lo | b.lo,
        hi: a.hi | b.hi,
    }
}

#[inline]
fn set_epi64x(hi: u64, lo: u64) -> M128 {
    M128 { lo, hi }
}

#[inline]
fn setr_epi32(a: u32, b: u32, c: u32, d: u32) -> M128 {
    M128 {
        lo: (a as u64) | ((b as u64) << 32),
        hi: (c as u64) | ((d as u64) << 32),
    }
}

#[inline]
fn setr_epi8(bytes: [u8; 16]) -> M128 {
    let mut lo_b = [0u8; 8];
    let mut hi_b = [0u8; 8];
    lo_b.copy_from_slice(&bytes[0..8]);
    hi_b.copy_from_slice(&bytes[8..16]);
    M128 {
        lo: u64::from_le_bytes(lo_b),
        hi: u64::from_le_bytes(hi_b),
    }
}

#[inline]
fn cvtsi64_si128(a: u64) -> M128 {
    M128 { lo: a, hi: 0 }
}

#[inline]
fn cvtsi128_si64(a: M128) -> u64 {
    a.lo
}

#[inline]
fn loadl_epi64(v: u64) -> M128 {
    M128 { lo: v, hi: 0 }
}

#[inline]
fn slli_epi64(a: M128, n: u32) -> M128 {
    M128 {
        lo: if n >= 64 { 0 } else { a.lo << n },
        hi: if n >= 64 { 0 } else { a.hi << n },
    }
}

#[inline]
fn srli_epi64(a: M128, n: u32) -> M128 {
    M128 {
        lo: if n >= 64 { 0 } else { a.lo >> n },
        hi: if n >= 64 { 0 } else { a.hi >> n },
    }
}

#[inline]
fn slli_si128(a: M128, k: u32) -> M128 {
    // Shift the whole 128-bit value left by k bytes (filling with zeros).
    if k >= 16 {
        return m128_zero();
    }
    if k == 0 {
        return a;
    }
    let bits = k * 8;
    if bits < 64 {
        let lo = a.lo << bits;
        let carry = a.lo >> (64 - bits);
        let hi = (a.hi << bits) | carry;
        M128 { lo, hi }
    } else if bits == 64 {
        M128 { lo: 0, hi: a.lo }
    } else {
        let bits2 = bits - 64;
        M128 {
            lo: 0,
            hi: a.lo << bits2,
        }
    }
}

#[inline]
fn srli_si128(a: M128, k: u32) -> M128 {
    if k >= 16 {
        return m128_zero();
    }
    if k == 0 {
        return a;
    }
    let bits = k * 8;
    if bits < 64 {
        let hi = a.hi >> bits;
        let carry = a.hi << (64 - bits);
        let lo = (a.lo >> bits) | carry;
        M128 { lo, hi }
    } else if bits == 64 {
        M128 { lo: a.hi, hi: 0 }
    } else {
        let bits2 = bits - 64;
        M128 {
            lo: a.hi >> bits2,
            hi: 0,
        }
    }
}

/// 64x64 -> 128 bit carry-less multiplication (PCLMULQDQ semantics).
#[inline]
fn clmul64(a: u64, b: u64) -> u128 {
    let mut result: u128 = 0;
    let a128 = a as u128;
    let mut bb = b;
    let mut i = 0u32;
    while bb != 0 {
        if bb & 1 != 0 {
            result ^= a128 << i;
        }
        bb >>= 1;
        i += 1;
    }
    result
}

#[inline]
fn u128_to_m128(v: u128) -> M128 {
    M128 {
        lo: v as u64,
        hi: (v >> 64) as u64,
    }
}

/// PCLMULQDQ: select 64-bit lanes of `a` and `b` based on `imm` and produce
/// the 128-bit carry-less product.  `imm` follows Intel's encoding:
/// bit 0 selects the lane of `a` (0 = low, 1 = high); bit 4 selects `b`.
#[inline]
fn clmulepi64_si128(a: M128, b: M128, imm: u32) -> M128 {
    let av = if imm & 0x01 != 0 { a.hi } else { a.lo };
    let bv = if imm & 0x10 != 0 { b.hi } else { b.lo };
    u128_to_m128(clmul64(av, bv))
}

/// PSHUFB: shuffle bytes of `a` according to indices in `b`.  When the high
/// bit of an index byte is set, the destination byte becomes zero; otherwise
/// the low 4 bits select a source byte.
#[inline]
fn shuffle_epi8(a: M128, b: M128) -> M128 {
    let src = [
        a.lo as u8,
        (a.lo >> 8) as u8,
        (a.lo >> 16) as u8,
        (a.lo >> 24) as u8,
        (a.lo >> 32) as u8,
        (a.lo >> 40) as u8,
        (a.lo >> 48) as u8,
        (a.lo >> 56) as u8,
        a.hi as u8,
        (a.hi >> 8) as u8,
        (a.hi >> 16) as u8,
        (a.hi >> 24) as u8,
        (a.hi >> 32) as u8,
        (a.hi >> 40) as u8,
        (a.hi >> 48) as u8,
        (a.hi >> 56) as u8,
    ];
    let idx = [
        b.lo as u8,
        (b.lo >> 8) as u8,
        (b.lo >> 16) as u8,
        (b.lo >> 24) as u8,
        (b.lo >> 32) as u8,
        (b.lo >> 40) as u8,
        (b.lo >> 48) as u8,
        (b.lo >> 56) as u8,
        b.hi as u8,
        (b.hi >> 8) as u8,
        (b.hi >> 16) as u8,
        (b.hi >> 24) as u8,
        (b.hi >> 32) as u8,
        (b.hi >> 40) as u8,
        (b.hi >> 48) as u8,
        (b.hi >> 56) as u8,
    ];
    let mut out = [0u8; 16];
    for i in 0..16 {
        let ii = idx[i];
        if ii & 0x80 != 0 {
            out[i] = 0;
        } else {
            out[i] = src[(ii & 0x0F) as usize];
        }
    }
    let mut lo_b = [0u8; 8];
    let mut hi_b = [0u8; 8];
    lo_b.copy_from_slice(&out[0..8]);
    hi_b.copy_from_slice(&out[8..16]);
    M128 {
        lo: u64::from_le_bytes(lo_b),
        hi: u64::from_le_bytes(hi_b),
    }
}

// ---------------------------------------------------------------------------
// Helpers that read 128 bits from an arbitrary byte offset of the random key
// or the input string.
// ---------------------------------------------------------------------------

#[inline]
fn read_u64_le(buf: &[u8], offset: usize) -> u64 {
    let mut tmp = [0u8; 8];
    let end = (offset + 8).min(buf.len());
    if offset < buf.len() {
        let n = end - offset;
        tmp[..n].copy_from_slice(&buf[offset..end]);
    }
    u64::from_le_bytes(tmp)
}

#[inline]
fn load_si128(rs: &[u8], idx: usize) -> M128 {
    // Each "__m128i index" maps to a 16-byte chunk.
    let off = idx * 16;
    M128 {
        lo: read_u64_le(rs, off),
        hi: read_u64_le(rs, off + 8),
    }
}

#[inline]
fn load_m128_from_words(words: &[u64], pos: usize) -> M128 {
    // Loads 128 bits starting at `pos` (in 64-bit-word units) from a u64 slice.
    let lo = if pos < words.len() { words[pos] } else { 0 };
    let hi = if pos + 1 < words.len() {
        words[pos + 1]
    } else {
        0
    };
    M128 { lo, hi }
}

// ---------------------------------------------------------------------------
// Algorithmic helpers ported from clhash.c.
// ---------------------------------------------------------------------------

#[inline]
fn leftshift1(a: M128) -> M128 {
    let u64shift = slli_epi64(a, 1);
    let topbits = slli_si128(srli_epi64(a, 63), 8);
    m128_or(u64shift, topbits)
}

#[inline]
fn leftshift2(a: M128) -> M128 {
    let u64shift = slli_epi64(a, 2);
    let topbits = slli_si128(srli_epi64(a, 62), 8);
    m128_or(u64shift, topbits)
}

#[inline]
fn lazymod127(alow: M128, ahigh: M128) -> M128 {
    let s1 = leftshift1(ahigh);
    let s2 = leftshift2(ahigh);
    m128_xor(m128_xor(alow, s1), s2)
}

#[inline]
fn mul128by128to128_lazymod127(a: M128, b: M128) -> M128 {
    let amix1 = clmulepi64_si128(a, b, 0x01);
    let amix2 = clmulepi64_si128(a, b, 0x10);
    let mut alow = clmulepi64_si128(a, b, 0x00);
    let mut ahigh = clmulepi64_si128(a, b, 0x11);
    let amix = m128_xor(amix1, amix2);
    let amix1s = slli_si128(amix, 8);
    let amix2s = srli_si128(amix, 8);
    alow = m128_xor(alow, amix1s);
    ahigh = m128_xor(ahigh, amix2s);
    lazymod127(alow, ahigh)
}

#[inline]
fn lazy_length_hash(keylength: u64, length: u64) -> M128 {
    // _mm_set_epi64x(keylength, length) — high lane = keylength, low = length.
    let v = set_epi64x(keylength, length);
    // _mm_clmulepi64_si128(lengthvector, lengthvector, 0x10) — multiplies the
    // low lane (length) by the high lane (keylength).
    clmulepi64_si128(v, v, 0x10)
}

#[inline]
fn precomp_reduction64_si128(a: M128) -> M128 {
    let c = cvtsi64_si128((1u64 << 4) + (1u64 << 3) + (1u64 << 1) + (1u64 << 0));
    let q2 = clmulepi64_si128(a, c, 0x01);
    let lookup = setr_epi8([
        0, 27, 54, 45, 108, 119, 90, 65, 216, 195, 238, 245, 180, 175, 130, 153,
    ]);
    let q3 = shuffle_epi8(lookup, srli_si128(q2, 8));
    let q4 = m128_xor(q2, a);
    m128_xor(q3, q4)
}

#[inline]
fn precomp_reduction64(a: M128) -> u64 {
    cvtsi128_si64(precomp_reduction64_si128(a))
}

#[inline]
fn simple128to64hashwithlength(value: M128, key: M128, keylength: u64, length: u64) -> u64 {
    let add = m128_xor(value, key);
    let clprod1 = clmulepi64_si128(add, add, 0x10);
    let total = m128_xor(clprod1, lazy_length_hash(keylength, length));
    precomp_reduction64(total)
}

// ---------------------------------------------------------------------------
// The core CLHASH building blocks.  Each operates on a slice of u64 words
// extracted from the input bytes (with whole 64-bit words being processed
// here; partial words are handled via createLastWord/lastword).
// ---------------------------------------------------------------------------

fn clmulhalfscalarproduct_no_reduction(rs: &[u8], rs_word_offset: usize, string: &[u64]) -> M128 {
    // Equivalent to __clmulhalfscalarproductwithoutreduction:
    // length is `string.len()`, expected to be 128 (a multiple of 4).
    let length = string.len();
    let mut acc = m128_zero();
    let mut s_idx = 0;
    let mut r_idx = rs_word_offset / 2; // index in 128-bit chunks
    while s_idx + 3 < length {
        let temp1 = load_si128(rs, r_idx);
        let temp2 = load_m128_from_words(string, s_idx);
        let add1 = m128_xor(temp1, temp2);
        let clprod1 = clmulepi64_si128(add1, add1, 0x10);
        acc = m128_xor(clprod1, acc);

        let temp12 = load_si128(rs, r_idx + 1);
        let temp22 = load_m128_from_words(string, s_idx + 2);
        let add12 = m128_xor(temp12, temp22);
        let clprod12 = clmulepi64_si128(add12, add12, 0x10);
        acc = m128_xor(clprod12, acc);

        r_idx += 2;
        s_idx += 4;
    }
    acc
}

fn clmulhalfscalarproduct_with_tail_no_reduction(
    rs: &[u8],
    rs_word_offset: usize,
    string: &[u64],
) -> M128 {
    // Equivalent to __clmulhalfscalarproductwithtailwithoutreduction.  Length
    // is `string.len()` and need not be divisible by 4.
    let length = string.len();
    let mut acc = m128_zero();
    let mut s_idx = 0;
    let mut r_idx = rs_word_offset / 2;
    while s_idx + 3 < length {
        let temp1 = load_si128(rs, r_idx);
        let temp2 = load_m128_from_words(string, s_idx);
        let add1 = m128_xor(temp1, temp2);
        let clprod1 = clmulepi64_si128(add1, add1, 0x10);
        acc = m128_xor(clprod1, acc);

        let temp12 = load_si128(rs, r_idx + 1);
        let temp22 = load_m128_from_words(string, s_idx + 2);
        let add12 = m128_xor(temp12, temp22);
        let clprod12 = clmulepi64_si128(add12, add12, 0x10);
        acc = m128_xor(clprod12, acc);

        r_idx += 2;
        s_idx += 4;
    }
    if s_idx + 1 < length {
        let temp1 = load_si128(rs, r_idx);
        let temp2 = load_m128_from_words(string, s_idx);
        let add1 = m128_xor(temp1, temp2);
        let clprod1 = clmulepi64_si128(add1, add1, 0x10);
        acc = m128_xor(clprod1, acc);
        r_idx += 1;
        s_idx += 2;
    }
    if s_idx < length {
        let temp1 = load_si128(rs, r_idx);
        let temp2 = loadl_epi64(string[s_idx]);
        let add1 = m128_xor(temp1, temp2);
        let clprod1 = clmulepi64_si128(add1, add1, 0x10);
        acc = m128_xor(clprod1, acc);
    }
    acc
}

fn clmulhalfscalarproduct_with_tail_no_reduction_with_extra_word(
    rs: &[u8],
    rs_word_offset: usize,
    string: &[u64],
    extraword: u64,
) -> M128 {
    let length = string.len();
    let mut acc = m128_zero();
    let mut s_idx = 0;
    let mut r_idx = rs_word_offset / 2;
    while s_idx + 3 < length {
        let temp1 = load_si128(rs, r_idx);
        let temp2 = load_m128_from_words(string, s_idx);
        let add1 = m128_xor(temp1, temp2);
        let clprod1 = clmulepi64_si128(add1, add1, 0x10);
        acc = m128_xor(clprod1, acc);

        let temp12 = load_si128(rs, r_idx + 1);
        let temp22 = load_m128_from_words(string, s_idx + 2);
        let add12 = m128_xor(temp12, temp22);
        let clprod12 = clmulepi64_si128(add12, add12, 0x10);
        acc = m128_xor(clprod12, acc);

        r_idx += 2;
        s_idx += 4;
    }
    if s_idx + 1 < length {
        let temp1 = load_si128(rs, r_idx);
        let temp2 = load_m128_from_words(string, s_idx);
        let add1 = m128_xor(temp1, temp2);
        let clprod1 = clmulepi64_si128(add1, add1, 0x10);
        acc = m128_xor(clprod1, acc);
        r_idx += 1;
        s_idx += 2;
    }
    if s_idx < length {
        // We still have one more complete 64-bit word, and we append
        // `extraword` in the high lane.
        let temp1 = load_si128(rs, r_idx);
        let temp2 = set_epi64x(extraword, string[s_idx]);
        let add1 = m128_xor(temp1, temp2);
        let clprod1 = clmulepi64_si128(add1, add1, 0x10);
        acc = m128_xor(clprod1, acc);
    } else {
        // No remaining complete word — only the partial extraword.
        let temp1 = load_si128(rs, r_idx);
        let temp2 = loadl_epi64(extraword);
        let add1 = m128_xor(temp1, temp2);
        let clprod1 = clmulepi64_si128(add1, add1, 0x01);
        acc = m128_xor(clprod1, acc);
    }
    acc
}

fn clmulhalfscalarproduct_only_extra_word(rs: &[u8], rs_word_offset: usize, extraword: u64) -> M128 {
    let r_idx = rs_word_offset / 2;
    let temp1 = load_si128(rs, r_idx);
    let temp2 = loadl_epi64(extraword);
    let add1 = m128_xor(temp1, temp2);
    clmulepi64_si128(add1, add1, 0x01)
}

// Append zero bytes to the trailing partial word and return it as u64.
#[inline]
fn create_last_word(lengthbyte: usize, full_words: usize, stringbyte: &[u8]) -> u64 {
    let significant = lengthbyte % 8;
    if significant == 0 {
        return 0;
    }
    let start = full_words * 8;
    let mut tmp = [0u8; 8];
    tmp[..significant].copy_from_slice(&stringbyte[start..start + significant]);
    u64::from_le_bytes(tmp)
}

// Convert a contiguous prefix of `stringbyte` into a Vec<u64> of `nwords`
// little-endian words.
fn bytes_to_u64_words(stringbyte: &[u8], nwords: usize) -> Vec<u64> {
    let mut out = vec![0u64; nwords];
    for i in 0..nwords {
        let mut tmp = [0u8; 8];
        let off = i * 8;
        let end = (off + 8).min(stringbyte.len());
        if off < stringbyte.len() {
            tmp[..end - off].copy_from_slice(&stringbyte[off..end]);
        }
        out[i] = u64::from_le_bytes(tmp);
    }
    out
}

// ---------------------------------------------------------------------------
// Public API: `clhash`.
// ---------------------------------------------------------------------------

pub fn clhash(random: &[u8], stringbyte: &[u8]) -> u64 {
    // Two convention paths are supported here:
    //
    //   * `random.len() >= RANDOM_BYTES_NEEDED_FOR_CLHASH` — `random` is
    //      treated as the full pre-generated key buffer (matches the C API).
    //   * `random.len() < RANDOM_BYTES_NEEDED_FOR_CLHASH` — `random` is
    //      treated as a small pair of `u64` seeds (the first 8 bytes form
    //      `seed1`, the next 8 bytes form `seed2`) and the full key buffer is
    //      derived from those seeds.  This matches the way the Rust tests
    //      pass two seed words via `bytemuck::bytes_of(&[u64; 2])`.
    let owned_key: Option<Vec<u8>> = if random.len() >= RANDOM_BYTES_NEEDED_FOR_CLHASH {
        None
    } else {
        let seed1 = read_u64_le(random, 0);
        let seed2 = read_u64_le(random, 8);
        Some(get_random_key_for_clhash(seed1, seed2))
    };
    let random: &[u8] = match owned_key.as_ref() {
        Some(v) => v.as_slice(),
        None => random,
    };
    let lengthbyte = stringbyte.len();
    let m: usize = 128; // chunk size in 64-bit words
    let m128_per_block = m / 2;

    // Decode polyvalue from the random key (the 128-bit chunk that follows the
    // main-loop key material).  Mask out the top two bits.
    let mut polyvalue = load_si128(random, m128_per_block);
    polyvalue = m128_and(
        polyvalue,
        setr_epi32(0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0x3fffffff),
    );

    let length = lengthbyte / 8; // # of complete 64-bit words
    let lengthinc = (lengthbyte + 7) / 8; // # of words including a partial trailing word

    // Decode all complete 64-bit words from the input once up front.
    let string_words = bytes_to_u64_words(stringbyte, length);

    if m < lengthinc {
        // Long-string path.
        let mut acc =
            clmulhalfscalarproduct_no_reduction(random, 0, &string_words[..m]);
        let mut t: usize = m;
        while t + m <= length {
            acc = mul128by128to128_lazymod127(polyvalue, acc);
            let h1 = clmulhalfscalarproduct_no_reduction(
                random,
                0,
                &string_words[t..t + m],
            );
            acc = m128_xor(acc, h1);
            t += m;
        }
        let remain = length - t;
        if remain != 0 {
            acc = mul128by128to128_lazymod127(polyvalue, acc);
            if lengthbyte % 8 == 0 {
                let h1 = clmulhalfscalarproduct_with_tail_no_reduction(
                    random,
                    0,
                    &string_words[t..t + remain],
                );
                acc = m128_xor(acc, h1);
            } else {
                let lastword = create_last_word(lengthbyte, length, stringbyte);
                let h1 = clmulhalfscalarproduct_with_tail_no_reduction_with_extra_word(
                    random,
                    0,
                    &string_words[t..t + remain],
                    lastword,
                );
                acc = m128_xor(acc, h1);
            }
        } else if lengthbyte % 8 != 0 {
            acc = mul128by128to128_lazymod127(polyvalue, acc);
            let lastword = create_last_word(lengthbyte, length, stringbyte);
            let h1 = clmulhalfscalarproduct_only_extra_word(random, 0, lastword);
            acc = m128_xor(acc, h1);
        }

        let finalkey = load_si128(random, m128_per_block + 1);
        // keylength is the 64-bit word starting at offset (m128neededperblock+2)*16 bytes.
        let keylength_off = (m128_per_block + 2) * 16;
        let keylength = read_u64_le(random, keylength_off);
        simple128to64hashwithlength(acc, finalkey, keylength, lengthbyte as u64)
    } else {
        // Short-string path.
        if lengthbyte % 8 == 0 {
            let acc = clmulhalfscalarproduct_with_tail_no_reduction(
                random,
                0,
                &string_words[..length],
            );
            let keylength_off = (m128_per_block + 2) * 16;
            let keylength = read_u64_le(random, keylength_off);
            let acc = m128_xor(acc, lazy_length_hash(keylength, lengthbyte as u64));
            return precomp_reduction64(acc);
        }
        let lastword = create_last_word(lengthbyte, length, stringbyte);
        let acc = clmulhalfscalarproduct_with_tail_no_reduction_with_extra_word(
            random,
            0,
            &string_words[..length],
            lastword,
        );
        let keylength_off = (m128_per_block + 2) * 16;
        let keylength = read_u64_le(random, keylength_off);
        let acc = m128_xor(acc, lazy_length_hash(keylength, lengthbyte as u64));
        precomp_reduction64(acc)
    }
}

// ---------------------------------------------------------------------------
// xorshift128+ random key generation, ported verbatim.
// ---------------------------------------------------------------------------

#[inline]
fn xorshift128plus(part1: &mut u64, part2: &mut u64) -> u64 {
    let mut s1 = *part1;
    let s0 = *part2;
    *part1 = s0;
    s1 ^= s1 << 23;
    *part2 = s1 ^ s0 ^ (s1 >> 18) ^ (s0 >> 5);
    part2.wrapping_add(s0)
}

pub fn get_random_key_for_clhash(seed1: u64, seed2: u64) -> Vec<u8> {
    let mut p1 = seed1;
    let mut p2 = seed2;
    let mut a64 = vec![0u64; RANDOM_64BITWORDS_NEEDED_FOR_CLHASH];
    for i in 0..RANDOM_64BITWORDS_NEEDED_FOR_CLHASH {
        a64[i] = xorshift128plus(&mut p1, &mut p2);
    }
    while a64[128] == 0 && a64[129] == 1 {
        a64[128] = xorshift128plus(&mut p1, &mut p2);
        a64[129] = xorshift128plus(&mut p1, &mut p2);
    }
    let mut bytes = Vec::with_capacity(RANDOM_BYTES_NEEDED_FOR_CLHASH);
    for w in &a64 {
        bytes.extend_from_slice(&w.to_le_bytes());
    }
    bytes
}

// ---------------------------------------------------------------------------
// Convenience wrapper struct mirroring the C++ `clhasher` helper.
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
        // Zero out the key material on drop — the equivalent of `free` in the
        // C version, plus a defensive scrub of the random bytes.
        for byte in self.random_data.iter_mut() {
            *byte = 0;
        }
        self.random_data.clear();
    }
}
