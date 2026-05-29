pub const RANDOM_64BITWORDS_NEEDED_FOR_CLHASH: usize = 133;
pub const RANDOM_BYTES_NEEDED_FOR_CLHASH: usize = 133 * 8;

// ---------- 128-bit value helpers ----------

#[derive(Copy, Clone, Default)]
struct M128 {
    lo: u64,
    hi: u64,
}

impl M128 {
    fn zero() -> Self {
        M128 { lo: 0, hi: 0 }
    }
    fn xor(self, other: M128) -> M128 {
        M128 {
            lo: self.lo ^ other.lo,
            hi: self.hi ^ other.hi,
        }
    }
}

fn m128_load(bytes: &[u8]) -> M128 {
    let mut lo_buf = [0u8; 8];
    let mut hi_buf = [0u8; 8];
    lo_buf.copy_from_slice(&bytes[0..8]);
    hi_buf.copy_from_slice(&bytes[8..16]);
    M128 {
        lo: u64::from_le_bytes(lo_buf),
        hi: u64::from_le_bytes(hi_buf),
    }
}

fn read_u64_le(bytes: &[u8]) -> u64 {
    let mut buf = [0u8; 8];
    buf.copy_from_slice(&bytes[0..8]);
    u64::from_le_bytes(buf)
}

// Carry-less multiplication of two 64-bit values: returns (lo, hi) as a 128-bit value.
fn clmul64(a: u64, b: u64) -> (u64, u64) {
    let mut lo: u64 = 0;
    let mut hi: u64 = 0;
    for i in 0..64u32 {
        if (b >> i) & 1 == 1 {
            if i == 0 {
                lo ^= a;
            } else {
                lo ^= a << i;
                hi ^= a >> (64 - i);
            }
        }
    }
    (lo, hi)
}

// _mm_clmulepi64_si128: imm bit 0 picks low(0)/high(1) of a, bit 4 picks low(0)/high(1) of b.
fn clmul_pi64_si128(a: M128, b: M128, imm: u8) -> M128 {
    let a_part = if imm & 0x01 != 0 { a.hi } else { a.lo };
    let b_part = if imm & 0x10 != 0 { b.hi } else { b.lo };
    let (lo, hi) = clmul64(a_part, b_part);
    M128 { lo, hi }
}

// Shift each 64-bit lane left by x.
fn slli_epi64(a: M128, x: u32) -> M128 {
    M128 {
        lo: a.lo << x,
        hi: a.hi << x,
    }
}

// Shift each 64-bit lane right by x (logical).
fn srli_epi64(a: M128, x: u32) -> M128 {
    M128 {
        lo: a.lo >> x,
        hi: a.hi >> x,
    }
}

// Shift the 128-bit value left by n bytes (zero-fill).
fn slli_si128(a: M128, n: u32) -> M128 {
    if n >= 16 {
        return M128::zero();
    }
    if n == 0 {
        return a;
    }
    if n < 8 {
        let bits = n * 8;
        M128 {
            lo: a.lo << bits,
            hi: (a.hi << bits) | (a.lo >> (64 - bits)),
        }
    } else if n == 8 {
        M128 { lo: 0, hi: a.lo }
    } else {
        let bits = (n - 8) * 8;
        M128 {
            lo: 0,
            hi: a.lo << bits,
        }
    }
}

// Shift the 128-bit value right by n bytes (zero-fill).
fn srli_si128(a: M128, n: u32) -> M128 {
    if n >= 16 {
        return M128::zero();
    }
    if n == 0 {
        return a;
    }
    if n < 8 {
        let bits = n * 8;
        M128 {
            lo: (a.lo >> bits) | (a.hi << (64 - bits)),
            hi: a.hi >> bits,
        }
    } else if n == 8 {
        M128 { lo: a.hi, hi: 0 }
    } else {
        let bits = (n - 8) * 8;
        M128 {
            lo: a.hi >> bits,
            hi: 0,
        }
    }
}

// _mm_shuffle_epi8: byte-wise permutation; if index byte's high bit is set, output is 0.
fn shuffle_epi8(table: M128, indices: M128) -> M128 {
    let mut tbytes = [0u8; 16];
    tbytes[0..8].copy_from_slice(&table.lo.to_le_bytes());
    tbytes[8..16].copy_from_slice(&table.hi.to_le_bytes());
    let mut ibytes = [0u8; 16];
    ibytes[0..8].copy_from_slice(&indices.lo.to_le_bytes());
    ibytes[8..16].copy_from_slice(&indices.hi.to_le_bytes());
    let mut out = [0u8; 16];
    for i in 0..16 {
        if ibytes[i] & 0x80 != 0 {
            out[i] = 0;
        } else {
            out[i] = tbytes[(ibytes[i] & 0x0F) as usize];
        }
    }
    let mut lo_buf = [0u8; 8];
    let mut hi_buf = [0u8; 8];
    lo_buf.copy_from_slice(&out[0..8]);
    hi_buf.copy_from_slice(&out[8..16]);
    M128 {
        lo: u64::from_le_bytes(lo_buf),
        hi: u64::from_le_bytes(hi_buf),
    }
}

// ---------- CLHash core operations (translated from clhash.c) ----------

fn leftshift1(a: M128) -> M128 {
    let u64shift = slli_epi64(a, 1);
    let topbits = slli_si128(srli_epi64(a, 63), 8);
    u64shift.xor(topbits)
}

fn leftshift2(a: M128) -> M128 {
    let u64shift = slli_epi64(a, 2);
    let topbits = slli_si128(srli_epi64(a, 62), 8);
    u64shift.xor(topbits)
}

// "lazy" modulo with 2^127 + 2 + 1; preconditions: top 2 bits of ahigh are zero.
fn lazymod127(alow: M128, ahigh: M128) -> M128 {
    let s1 = leftshift1(ahigh);
    let s2 = leftshift2(ahigh);
    alow.xor(s1).xor(s2)
}

// 128 x 128 -> 128 multiplication with lazy reduction.
fn mul128by128to128_lazymod127(a: M128, b: M128) -> M128 {
    let amix1 = clmul_pi64_si128(a, b, 0x01);
    let amix2 = clmul_pi64_si128(a, b, 0x10);
    let alow = clmul_pi64_si128(a, b, 0x00);
    let ahigh = clmul_pi64_si128(a, b, 0x11);
    let amix = amix1.xor(amix2);
    let amix1s = slli_si128(amix, 8);
    let amix2s = srli_si128(amix, 8);
    let alow2 = alow.xor(amix1s);
    let ahigh2 = ahigh.xor(amix2s);
    lazymod127(alow2, ahigh2)
}

// _mm_set_epi64x(keylength, length) gives high=keylength, low=length.
// Then clmul with imm=0x10 multiplies low(a) * high(b) = length * keylength.
fn lazy_length_hash(keylength: u64, length: u64) -> M128 {
    let lengthvector = M128 {
        lo: length,
        hi: keylength,
    };
    clmul_pi64_si128(lengthvector, lengthvector, 0x10)
}

// Reduce 128-bit polynomial value modulo (x^64 + x^4 + x^3 + x + 1).
// Returns the reduced 64-bit value in the low 64 bits; high bits contain garbage (matches C).
fn precomp_reduction64_si128(a: M128) -> M128 {
    // C constant: low = (1<<4)+(1<<3)+(1<<1)+(1<<0) = 0x1B
    let c = M128 { lo: 0x1B, hi: 0 };
    let q2 = clmul_pi64_si128(a, c, 0x01);
    // Lookup table from the C code (16 bytes).
    let table_bytes: [u8; 16] = [
        0, 27, 54, 45, 108, 119, 90, 65, 0xD8, 0xC3, 0xEE, 0xF5, 0xB4, 0xAF, 0x82, 0x99,
    ];
    let mut lo_buf = [0u8; 8];
    let mut hi_buf = [0u8; 8];
    lo_buf.copy_from_slice(&table_bytes[0..8]);
    hi_buf.copy_from_slice(&table_bytes[8..16]);
    let table = M128 {
        lo: u64::from_le_bytes(lo_buf),
        hi: u64::from_le_bytes(hi_buf),
    };
    let q3 = shuffle_epi8(table, srli_si128(q2, 8));
    let q4 = q2.xor(a);
    q3.xor(q4)
}

fn precomp_reduction64(a: M128) -> u64 {
    precomp_reduction64_si128(a).lo
}

fn simple128to64hashwithlength(value: M128, key: M128, keylength: u64, length: u64) -> u64 {
    let add = value.xor(key);
    let clprod1 = clmul_pi64_si128(add, add, 0x10);
    let total = clprod1.xor(lazy_length_hash(keylength, length));
    precomp_reduction64(total)
}

// ---------- inner-loop helpers (translated from C) ----------

// Process exactly `length_words` u64 words; expects length divisible by 4.
// `s_offset_bytes` is the byte offset into `s` where the chunk starts.
// `rs` is always the random source starting from offset 0.
fn clmul_half_scalar_product_without_reduction(
    rs: &[u8],
    s: &[u8],
    s_offset_bytes: usize,
    length_words: usize,
) -> M128 {
    let mut acc = M128::zero();
    let mut t: usize = 0;
    while t + 4 <= length_words {
        let rs_off = t * 8;
        let s_off = s_offset_bytes + t * 8;

        let temp1 = m128_load(&rs[rs_off..rs_off + 16]);
        let temp2 = m128_load(&s[s_off..s_off + 16]);
        let add1 = temp1.xor(temp2);
        let clprod1 = clmul_pi64_si128(add1, add1, 0x10);
        acc = acc.xor(clprod1);

        let temp12 = m128_load(&rs[rs_off + 16..rs_off + 32]);
        let temp22 = m128_load(&s[s_off + 16..s_off + 32]);
        let add12 = temp12.xor(temp22);
        let clprod12 = clmul_pi64_si128(add12, add12, 0x10);
        acc = acc.xor(clprod12);

        t += 4;
    }
    acc
}

// Same as above but handles 0..3 trailing u64 words.
fn clmul_half_scalar_product_with_tail(
    rs: &[u8],
    s: &[u8],
    s_offset_bytes: usize,
    length_words: usize,
) -> M128 {
    let mut acc = M128::zero();
    let mut t: usize = 0;
    while t + 4 <= length_words {
        let rs_off = t * 8;
        let s_off = s_offset_bytes + t * 8;

        let temp1 = m128_load(&rs[rs_off..rs_off + 16]);
        let temp2 = m128_load(&s[s_off..s_off + 16]);
        let add1 = temp1.xor(temp2);
        let clprod1 = clmul_pi64_si128(add1, add1, 0x10);
        acc = acc.xor(clprod1);

        let temp12 = m128_load(&rs[rs_off + 16..rs_off + 32]);
        let temp22 = m128_load(&s[s_off + 16..s_off + 32]);
        let add12 = temp12.xor(temp22);
        let clprod12 = clmul_pi64_si128(add12, add12, 0x10);
        acc = acc.xor(clprod12);

        t += 4;
    }
    if t + 2 <= length_words {
        let rs_off = t * 8;
        let s_off = s_offset_bytes + t * 8;
        let temp1 = m128_load(&rs[rs_off..rs_off + 16]);
        let temp2 = m128_load(&s[s_off..s_off + 16]);
        let add1 = temp1.xor(temp2);
        let clprod1 = clmul_pi64_si128(add1, add1, 0x10);
        acc = acc.xor(clprod1);
        t += 2;
    }
    if t < length_words {
        let rs_off = t * 8;
        let s_off = s_offset_bytes + t * 8;
        let temp1 = m128_load(&rs[rs_off..rs_off + 16]);
        let lo = read_u64_le(&s[s_off..s_off + 8]);
        let temp2 = M128 { lo, hi: 0 };
        let add1 = temp1.xor(temp2);
        let clprod1 = clmul_pi64_si128(add1, add1, 0x10);
        acc = acc.xor(clprod1);
    }
    acc
}

// Same as with_tail but appends `extraword` as a final partial word.
fn clmul_half_scalar_product_with_tail_extra(
    rs: &[u8],
    s: &[u8],
    s_offset_bytes: usize,
    length_words: usize,
    extraword: u64,
) -> M128 {
    let mut acc = M128::zero();
    let mut t: usize = 0;
    while t + 4 <= length_words {
        let rs_off = t * 8;
        let s_off = s_offset_bytes + t * 8;

        let temp1 = m128_load(&rs[rs_off..rs_off + 16]);
        let temp2 = m128_load(&s[s_off..s_off + 16]);
        let add1 = temp1.xor(temp2);
        let clprod1 = clmul_pi64_si128(add1, add1, 0x10);
        acc = acc.xor(clprod1);

        let temp12 = m128_load(&rs[rs_off + 16..rs_off + 32]);
        let temp22 = m128_load(&s[s_off + 16..s_off + 32]);
        let add12 = temp12.xor(temp22);
        let clprod12 = clmul_pi64_si128(add12, add12, 0x10);
        acc = acc.xor(clprod12);

        t += 4;
    }
    if t + 2 <= length_words {
        let rs_off = t * 8;
        let s_off = s_offset_bytes + t * 8;
        let temp1 = m128_load(&rs[rs_off..rs_off + 16]);
        let temp2 = m128_load(&s[s_off..s_off + 16]);
        let add1 = temp1.xor(temp2);
        let clprod1 = clmul_pi64_si128(add1, add1, 0x10);
        acc = acc.xor(clprod1);
        t += 2;
    }
    if t < length_words {
        // 1 more word + extraword
        let rs_off = t * 8;
        let s_off = s_offset_bytes + t * 8;
        let temp1 = m128_load(&rs[rs_off..rs_off + 16]);
        let lo = read_u64_le(&s[s_off..s_off + 8]);
        let temp2 = M128 { lo, hi: extraword };
        let add1 = temp1.xor(temp2);
        let clprod1 = clmul_pi64_si128(add1, add1, 0x10);
        acc = acc.xor(clprod1);
    } else {
        // only extraword
        let rs_off = t * 8;
        let temp1 = m128_load(&rs[rs_off..rs_off + 16]);
        let temp2 = M128 {
            lo: extraword,
            hi: 0,
        };
        let add1 = temp1.xor(temp2);
        let clprod1 = clmul_pi64_si128(add1, add1, 0x01);
        acc = acc.xor(clprod1);
    }
    acc
}

fn clmul_only_extra(rs: &[u8], rs_offset_bytes: usize, extraword: u64) -> M128 {
    let temp1 = m128_load(&rs[rs_offset_bytes..rs_offset_bytes + 16]);
    let temp2 = M128 {
        lo: extraword,
        hi: 0,
    };
    let add1 = temp1.xor(temp2);
    clmul_pi64_si128(add1, add1, 0x01)
}

// Build the final partial word by zero-padding the last `lengthbyte % 8` bytes.
fn create_last_word(lengthbyte: usize, lastw: &[u8]) -> u64 {
    let significantbytes = lengthbyte % 8;
    let mut bytes = [0u8; 8];
    bytes[..significantbytes].copy_from_slice(&lastw[..significantbytes]);
    u64::from_le_bytes(bytes)
}

// ---------- public API ----------

pub fn clhash(random: &[u8], stringbyte: &[u8]) -> u64 {
    const M_WORDS: usize = 128; // we process the data in chunks of 128 64-bit words
    const M128_NEEDED_PER_BLOCK: usize = M_WORDS / 2; // = 64

    let poly_offset = M128_NEEDED_PER_BLOCK * 16; // 1024
    let mut polyvalue = m128_load(&random[poly_offset..poly_offset + 16]);
    // _mm_and_si128 with [0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0x3FFFFFFF] (epi32)
    // -> low = 0xFFFF_FFFF_FFFF_FFFF, high = 0x3FFF_FFFF_FFFF_FFFF
    polyvalue.hi &= 0x3FFF_FFFF_FFFF_FFFFu64;

    let lengthbyte = stringbyte.len();
    let length = lengthbyte / 8; // # of complete 64-bit words
    let lengthinc = (lengthbyte + 7) / 8; // # of words including a partial one

    let keylength_offset = (M128_NEEDED_PER_BLOCK + 2) * 16; // 1056
    let keylength = read_u64_le(&random[keylength_offset..keylength_offset + 8]);

    if M_WORDS < lengthinc {
        // ---- long string path ----
        let mut acc =
            clmul_half_scalar_product_without_reduction(random, stringbyte, 0, M_WORDS);
        let mut t: usize = M_WORDS;
        while t + M_WORDS <= length {
            acc = mul128by128to128_lazymod127(polyvalue, acc);
            let h1 = clmul_half_scalar_product_without_reduction(
                random,
                stringbyte,
                t * 8,
                M_WORDS,
            );
            acc = acc.xor(h1);
            t += M_WORDS;
        }
        let remain = length - t;

        if remain != 0 {
            acc = mul128by128to128_lazymod127(polyvalue, acc);
            if lengthbyte % 8 == 0 {
                let h1 = clmul_half_scalar_product_with_tail(
                    random,
                    stringbyte,
                    t * 8,
                    remain,
                );
                acc = acc.xor(h1);
            } else {
                let lastword = create_last_word(lengthbyte, &stringbyte[length * 8..]);
                let h1 = clmul_half_scalar_product_with_tail_extra(
                    random,
                    stringbyte,
                    t * 8,
                    remain,
                    lastword,
                );
                acc = acc.xor(h1);
            }
        } else if lengthbyte % 8 != 0 {
            // No completely filled words remaining, but a partial one exists.
            acc = mul128by128to128_lazymod127(polyvalue, acc);
            let lastword = create_last_word(lengthbyte, &stringbyte[length * 8..]);
            let h1 = clmul_only_extra(random, 0, lastword);
            acc = acc.xor(h1);
        }

        let finalkey_offset = (M128_NEEDED_PER_BLOCK + 1) * 16; // 1040
        let finalkey = m128_load(&random[finalkey_offset..finalkey_offset + 16]);
        simple128to64hashwithlength(acc, finalkey, keylength, lengthbyte as u64)
    } else {
        // ---- short string path ----
        let mut acc = if lengthbyte % 8 == 0 {
            clmul_half_scalar_product_with_tail(random, stringbyte, 0, length)
        } else {
            let lastword = create_last_word(lengthbyte, &stringbyte[length * 8..]);
            clmul_half_scalar_product_with_tail_extra(
                random,
                stringbyte,
                0,
                length,
                lastword,
            )
        };
        acc = acc.xor(lazy_length_hash(keylength, lengthbyte as u64));
        precomp_reduction64(acc)
    }
}

// xorshift128plus PRNG used for generating the random key.
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
    let mut words = [0u64; RANDOM_64BITWORDS_NEEDED_FOR_CLHASH];
    for w in words.iter_mut() {
        *w = xorshift128plus(&mut p1, &mut p2);
    }
    while words[128] == 0 && words[129] == 1 {
        words[128] = xorshift128plus(&mut p1, &mut p2);
        words[129] = xorshift128plus(&mut p1, &mut p2);
    }
    let mut bytes = Vec::with_capacity(RANDOM_BYTES_NEEDED_FOR_CLHASH);
    for w in &words {
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
        // The Vec frees its allocation automatically; nothing extra to do here.
    }
}
