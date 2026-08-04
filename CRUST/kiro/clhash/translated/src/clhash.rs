pub const RANDOM_64BITWORDS_NEEDED_FOR_CLHASH: usize = 133;
pub const RANDOM_BYTES_NEEDED_FOR_CLHASH: usize = 133 * 8;

/// Carry-less multiply two 64-bit values, producing a 128-bit result.
fn clmul64(a: u64, b: u64) -> u128 {
    let mut result: u128 = 0;
    let b128 = b as u128;
    for i in 0..64 {
        if (a >> i) & 1 != 0 {
            result ^= b128 << i;
        }
    }
    result
}

/// _mm_clmulepi64_si128 equivalent: carry-less multiply selected 64-bit halves.
/// imm selects which halves: bit 0 selects half of a, bit 4 selects half of b.
fn clmulepi64(a: u128, b: u128, imm: u8) -> u128 {
    let a_half = if imm & 1 == 0 { a as u64 } else { (a >> 64) as u64 };
    let b_half = if imm & 0x10 == 0 { b as u64 } else { (b >> 64) as u64 };
    clmul64(a_half, b_half)
}

fn lo64(v: u128) -> u64 { v as u64 }
fn hi64(v: u128) -> u64 { (v >> 64) as u64 }
fn from_halves(lo: u64, hi: u64) -> u128 { (lo as u128) | ((hi as u128) << 64) }

fn leftshift1(a: u128) -> u128 {
    let lo = lo64(a);
    let hi = hi64(a);
    let new_lo = lo << 1;
    let new_hi = (hi << 1) | (lo >> 63);
    from_halves(new_lo, new_hi)
}

fn leftshift2(a: u128) -> u128 {
    let lo = lo64(a);
    let hi = hi64(a);
    let new_lo = lo << 2;
    let new_hi = (hi << 2) | (lo >> 62);
    from_halves(new_lo, new_hi)
}

fn lazymod127(alow: u128, ahigh: u128) -> u128 {
    leftshift1(ahigh) ^ leftshift2(ahigh) ^ alow
}

fn mul128by128to128_lazymod127(a: u128, b: u128) -> u128 {
    let amix1 = clmulepi64(a, b, 0x01);
    let amix2 = clmulepi64(a, b, 0x10);
    let alow = clmulepi64(a, b, 0x00);
    let ahigh = clmulepi64(a, b, 0x11);
    let amix = amix1 ^ amix2;
    // slli_si128(amix, 8) shifts left by 8 bytes = moves lo64 to hi64, lo becomes 0
    let amix1_shifted = from_halves(0, lo64(amix));
    // srli_si128(amix, 8) shifts right by 8 bytes = moves hi64 to lo64, hi becomes 0
    let amix2_shifted = hi64(amix) as u128;
    let alow = alow ^ amix1_shifted;
    let ahigh = ahigh ^ amix2_shifted;
    lazymod127(alow, ahigh)
}

fn lazy_length_hash(keylength: u64, length: u64) -> u128 {
    // _mm_set_epi64x(keylength, length) => hi=keylength, lo=length
    // clmulepi64 with imm=0x10 => clmul(hi_of_a, lo_of_b) = clmul(keylength, length)
    let v = from_halves(length, keylength);
    clmulepi64(v, v, 0x10)
}

fn precomp_reduction64(a: u128) -> u64 {
    let c: u128 = ((1u32 << 4) + (1u32 << 3) + (1u32 << 1) + 1u32) as u128;
    let q2 = clmulepi64(a, c, 0x01);
    let table: [u8; 16] = [0, 27, 54, 45, 108, 119, 90, 65, 216, 195, 238, 245, 180, 175, 130, 153];
    // srli_si128(q2, 8): full 16 bytes with lo=hi64(q2), hi=0
    let shifted = hi64(q2) as u128; // lo=hi64(q2), hi=0
    let idx_bytes = shifted.to_le_bytes();
    let mut q3_bytes = [0u8; 16];
    for i in 0..16 {
        let idx = idx_bytes[i];
        if idx & 0x80 != 0 {
            q3_bytes[i] = 0;
        } else {
            q3_bytes[i] = table[(idx & 0x0f) as usize];
        }
    }
    let q3 = u128::from_le_bytes(q3_bytes);
    let q4 = q2 ^ a;
    lo64(q3 ^ q4)
}

fn simple128to64hashwithlength(value: u128, key: u128, keylength: u64, length: u64) -> u64 {
    let add = value ^ key;
    let clprod1 = clmulepi64(add, add, 0x10);
    let total = clprod1 ^ lazy_length_hash(keylength, length);
    precomp_reduction64(total)
}

fn read_u64_le(data: &[u8], offset: usize) -> u64 {
    if offset >= data.len() {
        return 0;
    }
    let mut buf = [0u8; 8];
    let end = (offset + 8).min(data.len());
    buf[..end - offset].copy_from_slice(&data[offset..end]);
    u64::from_le_bytes(buf)
}

fn read_u128_from_u64s(random: &[u8], idx: usize) -> u128 {
    // Read two u64s at byte offset idx*8
    let off = idx * 8;
    let lo = read_u64_le(random, off);
    let hi = read_u64_le(random, off + 8);
    from_halves(lo, hi)
}

fn clmul_half_scalar_product_without_reduction(rs: &[u8], string: &[u64], length: usize) -> u128 {
    let mut acc: u128 = 0;
    let mut rs_idx: usize = 0; // index in units of u64 pairs (each __m128i = 2 u64s)
    let mut s_idx: usize = 0;
    while s_idx + 3 < length {
        let temp1 = read_u128_from_u64s(rs, rs_idx * 2);
        let temp2 = from_halves(string[s_idx], string[s_idx + 1]);
        let add1 = temp1 ^ temp2;
        let clprod1 = clmulepi64(add1, add1, 0x10);
        acc ^= clprod1;

        let temp12 = read_u128_from_u64s(rs, rs_idx * 2 + 2);
        let temp22 = from_halves(string[s_idx + 2], string[s_idx + 3]);
        let add12 = temp12 ^ temp22;
        let clprod12 = clmulepi64(add12, add12, 0x10);
        acc ^= clprod12;

        rs_idx += 2;
        s_idx += 4;
    }
    acc
}

fn clmul_half_scalar_product_with_tail_without_reduction(rs: &[u8], string: &[u64], length: usize) -> u128 {
    let mut acc: u128 = 0;
    let mut rs_idx: usize = 0;
    let mut s_idx: usize = 0;
    while s_idx + 3 < length {
        let temp1 = read_u128_from_u64s(rs, rs_idx * 2);
        let temp2 = from_halves(string[s_idx], string[s_idx + 1]);
        let add1 = temp1 ^ temp2;
        acc ^= clmulepi64(add1, add1, 0x10);

        let temp12 = read_u128_from_u64s(rs, rs_idx * 2 + 2);
        let temp22 = from_halves(string[s_idx + 2], string[s_idx + 3]);
        let add12 = temp12 ^ temp22;
        acc ^= clmulepi64(add12, add12, 0x10);

        rs_idx += 2;
        s_idx += 4;
    }
    if s_idx + 1 < length {
        let temp1 = read_u128_from_u64s(rs, rs_idx * 2);
        let temp2 = from_halves(string[s_idx], string[s_idx + 1]);
        let add1 = temp1 ^ temp2;
        acc ^= clmulepi64(add1, add1, 0x10);
        rs_idx += 1;
        s_idx += 2;
    }
    if s_idx < length {
        let temp1 = read_u128_from_u64s(rs, rs_idx * 2);
        let temp2 = string[s_idx] as u128; // loadl_epi64: lo=string[s_idx], hi=0
        let add1 = temp1 ^ temp2;
        acc ^= clmulepi64(add1, add1, 0x10);
    }
    acc
}

fn clmul_half_scalar_product_with_tail_without_reduction_with_extra_word(
    rs: &[u8], string: &[u64], length: usize, extraword: u64,
) -> u128 {
    let mut acc: u128 = 0;
    let mut rs_idx: usize = 0;
    let mut s_idx: usize = 0;
    while s_idx + 3 < length {
        let temp1 = read_u128_from_u64s(rs, rs_idx * 2);
        let temp2 = from_halves(string[s_idx], string[s_idx + 1]);
        let add1 = temp1 ^ temp2;
        acc ^= clmulepi64(add1, add1, 0x10);

        let temp12 = read_u128_from_u64s(rs, rs_idx * 2 + 2);
        let temp22 = from_halves(string[s_idx + 2], string[s_idx + 3]);
        let add12 = temp12 ^ temp22;
        acc ^= clmulepi64(add12, add12, 0x10);

        rs_idx += 2;
        s_idx += 4;
    }
    if s_idx + 1 < length {
        let temp1 = read_u128_from_u64s(rs, rs_idx * 2);
        let temp2 = from_halves(string[s_idx], string[s_idx + 1]);
        let add1 = temp1 ^ temp2;
        acc ^= clmulepi64(add1, add1, 0x10);
        rs_idx += 1;
        s_idx += 2;
    }
    if s_idx < length {
        // _mm_set_epi64x(extraword, *string) => hi=extraword, lo=string[s_idx]
        let temp1 = read_u128_from_u64s(rs, rs_idx * 2);
        let temp2 = from_halves(string[s_idx], extraword);
        let add1 = temp1 ^ temp2;
        acc ^= clmulepi64(add1, add1, 0x10);
    } else {
        // _mm_loadl_epi64(&extraword) => lo=extraword, hi=0
        let temp1 = read_u128_from_u64s(rs, rs_idx * 2);
        let temp2 = extraword as u128;
        let add1 = temp1 ^ temp2;
        acc ^= clmulepi64(add1, add1, 0x01); // note: 0x01 here, not 0x10
    }
    acc
}

fn clmul_half_scalar_product_only_extra_word(rs: &[u8], rs_idx: usize, extraword: u64) -> u128 {
    let temp1 = read_u128_from_u64s(rs, rs_idx * 2);
    let temp2 = extraword as u128;
    let add1 = temp1 ^ temp2;
    clmulepi64(add1, add1, 0x01)
}

fn create_last_word(lengthbyte: usize, data: &[u8], offset: usize) -> u64 {
    let significant = lengthbyte % 8;
    let mut lastword = [0u8; 8];
    let avail = data.len() - offset;
    let to_copy = significant.min(avail);
    lastword[..to_copy].copy_from_slice(&data[offset..offset + to_copy]);
    u64::from_le_bytes(lastword)
}

/// Convert byte slice to a vector of u64 (little-endian).
fn bytes_to_u64s(data: &[u8]) -> Vec<u64> {
    let n = (data.len() + 7) / 8;
    let mut result = Vec::with_capacity(n);
    for i in 0..n {
        let off = i * 8;
        let mut buf = [0u8; 8];
        let end = (off + 8).min(data.len());
        buf[..end - off].copy_from_slice(&data[off..end]);
        result.push(u64::from_le_bytes(buf));
    }
    result
}

pub fn clhash(random: &[u8], stringbyte: &[u8]) -> u64 {
    if random.len() < RANDOM_BYTES_NEEDED_FOR_CLHASH {
        // Generate full key from first two u64s as seeds
        let seed1 = read_u64_le(random, 0);
        let seed2 = read_u64_le(random, 8);
        let full_key = get_random_key_for_clhash(seed1, seed2);
        return clhash_inner(&full_key, stringbyte);
    }
    clhash_inner(random, stringbyte)
}

fn clhash_inner(random: &[u8], stringbyte: &[u8]) -> u64 {
    let lengthbyte = stringbyte.len();
    let m: usize = 128; // process data in chunks of 128 u64 words
    let m128neededperblock = m / 2; // 64 __m128i values = 128 u64 values

    // polyvalue = rs64[m128neededperblock] with top 2 bits of hi64 cleared
    let polyvalue = {
        let v = read_u128_from_u64s(random, m128neededperblock * 2);
        v & from_halves(0xFFFFFFFF_FFFFFFFF, 0x3FFFFFFF_FFFFFFFF)
    };

    let length = lengthbyte / 8; // complete u64 words
    let lengthinc = (lengthbyte + 7) / 8; // words including partial

    let string = bytes_to_u64s(stringbyte);

    if m < lengthinc {
        // long strings
        let mut acc = clmul_half_scalar_product_without_reduction(random, &string, m);
        let mut t = m;
        while t + m <= length {
            acc = mul128by128to128_lazymod127(polyvalue, acc);
            let h1 = clmul_half_scalar_product_without_reduction(random, &string[t..], m);
            acc ^= h1;
            t += m;
        }
        let remain = length - t;
        if remain != 0 {
            acc = mul128by128to128_lazymod127(polyvalue, acc);
            if lengthbyte % 8 == 0 {
                let h1 = clmul_half_scalar_product_with_tail_without_reduction(random, &string[t..], remain);
                acc ^= h1;
            } else {
                let lastword = create_last_word(lengthbyte, stringbyte, length * 8);
                let h1 = clmul_half_scalar_product_with_tail_without_reduction_with_extra_word(
                    random, &string[t..], remain, lastword,
                );
                acc ^= h1;
            }
        } else if lengthbyte % 8 != 0 {
            acc = mul128by128to128_lazymod127(polyvalue, acc);
            let lastword = create_last_word(lengthbyte, stringbyte, length * 8);
            let h1 = clmul_half_scalar_product_only_extra_word(random, 0, lastword);
            acc ^= h1;
        }

        let finalkey = read_u128_from_u64s(random, (m128neededperblock + 1) * 2);
        let keylength = read_u64_le(random, (m128neededperblock + 2) * 16);
        simple128to64hashwithlength(acc, finalkey, keylength, lengthbyte as u64)
    } else {
        // short strings
        if lengthbyte % 8 == 0 {
            let mut acc = clmul_half_scalar_product_with_tail_without_reduction(random, &string, length);
            let keylength = read_u64_le(random, (m128neededperblock + 2) * 16);
            acc ^= lazy_length_hash(keylength, lengthbyte as u64);
            precomp_reduction64(acc)
        } else {
            let lastword = create_last_word(lengthbyte, stringbyte, length * 8);
            let mut acc = clmul_half_scalar_product_with_tail_without_reduction_with_extra_word(
                random, &string, length, lastword,
            );
            let keylength = read_u64_le(random, (m128neededperblock + 2) * 16);
            acc ^= lazy_length_hash(keylength, lengthbyte as u64);
            precomp_reduction64(acc)
        }
    }
}

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
    // Convert to bytes (little-endian)
    let mut result = vec![0u8; RANDOM_BYTES_NEEDED_FOR_CLHASH];
    for (i, &val) in a64.iter().enumerate() {
        result[i * 8..(i + 1) * 8].copy_from_slice(&val.to_le_bytes());
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
        // Nothing to do in safe Rust - Vec drops automatically
    }
}
