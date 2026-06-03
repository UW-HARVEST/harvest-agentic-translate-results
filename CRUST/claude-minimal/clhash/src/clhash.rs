use core::arch::x86_64::*;
use core::ptr;

pub const RANDOM_64BITWORDS_NEEDED_FOR_CLHASH: usize = 133;
pub const RANDOM_BYTES_NEEDED_FOR_CLHASH: usize = 133 * 8;

const M: usize = 128;
const M128_NEEDED_PER_BLOCK: usize = M / 2; // 64

#[inline]
#[target_feature(enable = "sse2,ssse3,sse4.1,pclmulqdq")]
unsafe fn leftshift1(a: __m128i) -> __m128i {
    let u64shift = _mm_slli_epi64(a, 1);
    let topbits = _mm_slli_si128::<8>(_mm_srli_epi64(a, 63));
    _mm_or_si128(u64shift, topbits)
}

#[inline]
#[target_feature(enable = "sse2,ssse3,sse4.1,pclmulqdq")]
unsafe fn leftshift2(a: __m128i) -> __m128i {
    let u64shift = _mm_slli_epi64(a, 2);
    let topbits = _mm_slli_si128::<8>(_mm_srli_epi64(a, 62));
    _mm_or_si128(u64shift, topbits)
}

#[inline]
#[target_feature(enable = "sse2,ssse3,sse4.1,pclmulqdq")]
unsafe fn lazymod127(alow: __m128i, ahigh: __m128i) -> __m128i {
    let shift1 = leftshift1(ahigh);
    let shift2 = leftshift2(ahigh);
    _mm_xor_si128(_mm_xor_si128(alow, shift1), shift2)
}

#[inline]
#[target_feature(enable = "sse2,ssse3,sse4.1,pclmulqdq")]
unsafe fn mul128by128to128_lazymod127(a: __m128i, b: __m128i) -> __m128i {
    let amix1 = _mm_clmulepi64_si128::<0x01>(a, b);
    let amix2 = _mm_clmulepi64_si128::<0x10>(a, b);
    let mut alow = _mm_clmulepi64_si128::<0x00>(a, b);
    let mut ahigh = _mm_clmulepi64_si128::<0x11>(a, b);
    let amix = _mm_xor_si128(amix1, amix2);
    alow = _mm_xor_si128(alow, _mm_slli_si128::<8>(amix));
    ahigh = _mm_xor_si128(ahigh, _mm_srli_si128::<8>(amix));
    lazymod127(alow, ahigh)
}

#[inline]
#[target_feature(enable = "sse2,ssse3,sse4.1,pclmulqdq")]
unsafe fn lazy_length_hash(keylength: u64, length: u64) -> __m128i {
    let lengthvector = _mm_set_epi64x(keylength as i64, length as i64);
    _mm_clmulepi64_si128::<0x10>(lengthvector, lengthvector)
}

#[inline]
#[target_feature(enable = "sse2,ssse3,sse4.1,pclmulqdq")]
unsafe fn precomp_reduction64_si128(a: __m128i) -> __m128i {
    let c = _mm_cvtsi64_si128(((1u64 << 4) | (1u64 << 3) | (1u64 << 1) | (1u64 << 0)) as i64);
    let q2 = _mm_clmulepi64_si128::<0x01>(a, c);
    let table = _mm_setr_epi8(
        0,
        27,
        54,
        45,
        108,
        119,
        90,
        65,
        216u8 as i8,
        195u8 as i8,
        238u8 as i8,
        245u8 as i8,
        180u8 as i8,
        175u8 as i8,
        130u8 as i8,
        153u8 as i8,
    );
    let q3 = _mm_shuffle_epi8(table, _mm_srli_si128::<8>(q2));
    let q4 = _mm_xor_si128(q2, a);
    _mm_xor_si128(q3, q4)
}

#[inline]
#[target_feature(enable = "sse2,ssse3,sse4.1,pclmulqdq")]
unsafe fn precomp_reduction64(a: __m128i) -> u64 {
    _mm_cvtsi128_si64(precomp_reduction64_si128(a)) as u64
}

#[inline]
#[target_feature(enable = "sse2,ssse3,sse4.1,pclmulqdq")]
unsafe fn simple128to64hashwithlength(
    value: __m128i,
    key: __m128i,
    keylength: u64,
    length: u64,
) -> u64 {
    let add = _mm_xor_si128(value, key);
    let clprod1 = _mm_clmulepi64_si128::<0x10>(add, add);
    let total = _mm_xor_si128(clprod1, lazy_length_hash(keylength, length));
    precomp_reduction64(total)
}

#[inline]
#[target_feature(enable = "sse2,ssse3,sse4.1,pclmulqdq")]
unsafe fn clmul_half_scalar_product_without_reduction(
    randomsource: *const __m128i,
    string: *const u64,
    length: usize,
) -> __m128i {
    let mut rs = randomsource;
    let mut s = string;
    let endstring = string.add(length);
    let mut acc = _mm_setzero_si128();
    while s.add(3) < endstring {
        let temp1 = _mm_loadu_si128(rs);
        let temp2 = _mm_loadu_si128(s as *const __m128i);
        let add1 = _mm_xor_si128(temp1, temp2);
        let clprod1 = _mm_clmulepi64_si128::<0x10>(add1, add1);
        acc = _mm_xor_si128(clprod1, acc);

        let temp12 = _mm_loadu_si128(rs.add(1));
        let temp22 = _mm_loadu_si128(s.add(2) as *const __m128i);
        let add12 = _mm_xor_si128(temp12, temp22);
        let clprod12 = _mm_clmulepi64_si128::<0x10>(add12, add12);
        acc = _mm_xor_si128(clprod12, acc);

        rs = rs.add(2);
        s = s.add(4);
    }
    acc
}

#[inline]
#[target_feature(enable = "sse2,ssse3,sse4.1,pclmulqdq")]
unsafe fn clmul_half_scalar_product_with_tail_without_reduction(
    randomsource: *const __m128i,
    string: *const u64,
    length: usize,
) -> __m128i {
    let mut rs = randomsource;
    let mut s = string;
    let endstring = string.add(length);
    let mut acc = _mm_setzero_si128();
    while s.add(3) < endstring {
        let temp1 = _mm_loadu_si128(rs);
        let temp2 = _mm_loadu_si128(s as *const __m128i);
        let add1 = _mm_xor_si128(temp1, temp2);
        let clprod1 = _mm_clmulepi64_si128::<0x10>(add1, add1);
        acc = _mm_xor_si128(clprod1, acc);

        let temp12 = _mm_loadu_si128(rs.add(1));
        let temp22 = _mm_loadu_si128(s.add(2) as *const __m128i);
        let add12 = _mm_xor_si128(temp12, temp22);
        let clprod12 = _mm_clmulepi64_si128::<0x10>(add12, add12);
        acc = _mm_xor_si128(clprod12, acc);

        rs = rs.add(2);
        s = s.add(4);
    }
    if s.add(1) < endstring {
        let temp1 = _mm_loadu_si128(rs);
        let temp2 = _mm_loadu_si128(s as *const __m128i);
        let add1 = _mm_xor_si128(temp1, temp2);
        let clprod1 = _mm_clmulepi64_si128::<0x10>(add1, add1);
        acc = _mm_xor_si128(clprod1, acc);
        rs = rs.add(1);
        s = s.add(2);
    }
    if s < endstring {
        let temp1 = _mm_loadu_si128(rs);
        let temp2 = _mm_loadl_epi64(s as *const __m128i);
        let add1 = _mm_xor_si128(temp1, temp2);
        let clprod1 = _mm_clmulepi64_si128::<0x10>(add1, add1);
        acc = _mm_xor_si128(clprod1, acc);
    }
    acc
}

#[inline]
#[target_feature(enable = "sse2,ssse3,sse4.1,pclmulqdq")]
unsafe fn clmul_half_scalar_product_with_tail_without_reduction_with_extra_word(
    randomsource: *const __m128i,
    string: *const u64,
    length: usize,
    extraword: u64,
) -> __m128i {
    let mut rs = randomsource;
    let mut s = string;
    let endstring = string.add(length);
    let mut acc = _mm_setzero_si128();
    while s.add(3) < endstring {
        let temp1 = _mm_loadu_si128(rs);
        let temp2 = _mm_loadu_si128(s as *const __m128i);
        let add1 = _mm_xor_si128(temp1, temp2);
        let clprod1 = _mm_clmulepi64_si128::<0x10>(add1, add1);
        acc = _mm_xor_si128(clprod1, acc);

        let temp12 = _mm_loadu_si128(rs.add(1));
        let temp22 = _mm_loadu_si128(s.add(2) as *const __m128i);
        let add12 = _mm_xor_si128(temp12, temp22);
        let clprod12 = _mm_clmulepi64_si128::<0x10>(add12, add12);
        acc = _mm_xor_si128(clprod12, acc);

        rs = rs.add(2);
        s = s.add(4);
    }
    if s.add(1) < endstring {
        let temp1 = _mm_loadu_si128(rs);
        let temp2 = _mm_loadu_si128(s as *const __m128i);
        let add1 = _mm_xor_si128(temp1, temp2);
        let clprod1 = _mm_clmulepi64_si128::<0x10>(add1, add1);
        acc = _mm_xor_si128(clprod1, acc);
        rs = rs.add(1);
        s = s.add(2);
    }
    if s < endstring {
        let temp1 = _mm_loadu_si128(rs);
        let star = ptr::read_unaligned(s);
        let temp2 = _mm_set_epi64x(extraword as i64, star as i64);
        let add1 = _mm_xor_si128(temp1, temp2);
        let clprod1 = _mm_clmulepi64_si128::<0x10>(add1, add1);
        acc = _mm_xor_si128(clprod1, acc);
    } else {
        let temp1 = _mm_loadu_si128(rs);
        let temp2 = _mm_loadl_epi64(&extraword as *const u64 as *const __m128i);
        let add1 = _mm_xor_si128(temp1, temp2);
        let clprod1 = _mm_clmulepi64_si128::<0x01>(add1, add1);
        acc = _mm_xor_si128(clprod1, acc);
    }
    acc
}

#[inline]
#[target_feature(enable = "sse2,ssse3,sse4.1,pclmulqdq")]
unsafe fn clmul_half_scalar_product_only_extra_word(
    randomsource: *const __m128i,
    extraword: u64,
) -> __m128i {
    let temp1 = _mm_loadu_si128(randomsource);
    let temp2 = _mm_loadl_epi64(&extraword as *const u64 as *const __m128i);
    let add1 = _mm_xor_si128(temp1, temp2);
    _mm_clmulepi64_si128::<0x01>(add1, add1)
}

#[inline]
unsafe fn create_last_word(lengthbyte: usize, lastw: *const u64) -> u64 {
    let significantbytes = lengthbyte % core::mem::size_of::<u64>();
    let mut lastword: u64 = 0;
    let dst = &mut lastword as *mut u64 as *mut u8;
    let src = lastw as *const u8;
    ptr::copy_nonoverlapping(src, dst, significantbytes);
    lastword
}

fn expand_random_to_full(input: &[u8]) -> Vec<u8> {
    if input.len() >= RANDOM_BYTES_NEEDED_FOR_CLHASH {
        return input[..RANDOM_BYTES_NEEDED_FOR_CLHASH].to_vec();
    }
    // Treat the input as seed material for xorshift128plus and
    // expand it to the required size. This lets callers pass small
    // (e.g. 16-byte) keys directly without precomputing the full key.
    let mut seed_buf = [0u8; 16];
    let copy_len = input.len().min(16);
    seed_buf[..copy_len].copy_from_slice(&input[..copy_len]);
    let seed1 = u64::from_le_bytes(seed_buf[0..8].try_into().unwrap());
    let seed2 = u64::from_le_bytes(seed_buf[8..16].try_into().unwrap());
    get_random_key_for_clhash(seed1, seed2)
}

#[target_feature(enable = "sse2,ssse3,sse4.1,pclmulqdq")]
unsafe fn clhash_inner(random: &[u8], stringbyte: &[u8]) -> u64 {
    let full_random = expand_random_to_full(random);
    // Make sure the storage is u64-aligned by copying into a Vec<u64>.
    let mut padded_random: Vec<u64> = vec![0u64; RANDOM_64BITWORDS_NEEDED_FOR_CLHASH];
    {
        let dst = padded_random.as_mut_ptr() as *mut u8;
        ptr::copy_nonoverlapping(full_random.as_ptr(), dst, RANDOM_BYTES_NEEDED_FOR_CLHASH);
    }

    let lengthbyte = stringbyte.len();
    let rs64 = padded_random.as_ptr() as *const __m128i;

    let mut polyvalue = _mm_loadu_si128(rs64.add(M128_NEEDED_PER_BLOCK));
    polyvalue = _mm_and_si128(
        polyvalue,
        _mm_setr_epi32(
            0xFFFFFFFFu32 as i32,
            0xFFFFFFFFu32 as i32,
            0xFFFFFFFFu32 as i32,
            0x3FFFFFFFu32 as i32,
        ),
    );

    let length = lengthbyte / core::mem::size_of::<u64>(); // # of complete words
    let lengthinc = (lengthbyte + core::mem::size_of::<u64>() - 1) / core::mem::size_of::<u64>(); // # incl. partials

    let string = stringbyte.as_ptr() as *const u64;

    if M < lengthinc {
        // long strings
        let mut acc = clmul_half_scalar_product_without_reduction(rs64, string, M);
        let mut t = M;
        while t + M <= length {
            acc = mul128by128to128_lazymod127(polyvalue, acc);
            let h1 = clmul_half_scalar_product_without_reduction(rs64, string.add(t), M);
            acc = _mm_xor_si128(acc, h1);
            t += M;
        }
        let remain = length - t; // number of completely filled words

        if remain != 0 {
            acc = mul128by128to128_lazymod127(polyvalue, acc);
            if lengthbyte % core::mem::size_of::<u64>() == 0 {
                let h1 = clmul_half_scalar_product_with_tail_without_reduction(
                    rs64,
                    string.add(t),
                    remain,
                );
                acc = _mm_xor_si128(acc, h1);
            } else {
                let lastword = create_last_word(lengthbyte, string.add(length));
                let h1 = clmul_half_scalar_product_with_tail_without_reduction_with_extra_word(
                    rs64,
                    string.add(t),
                    remain,
                    lastword,
                );
                acc = _mm_xor_si128(acc, h1);
            }
        } else if lengthbyte % core::mem::size_of::<u64>() != 0 {
            // there are no completely filled words left, but there is one partial word.
            acc = mul128by128to128_lazymod127(polyvalue, acc);
            let lastword = create_last_word(lengthbyte, string.add(length));
            let h1 = clmul_half_scalar_product_only_extra_word(rs64, lastword);
            acc = _mm_xor_si128(acc, h1);
        }

        let finalkey = _mm_loadu_si128(rs64.add(M128_NEEDED_PER_BLOCK + 1));
        let keylength =
            ptr::read_unaligned(rs64.add(M128_NEEDED_PER_BLOCK + 2) as *const u64);
        simple128to64hashwithlength(acc, finalkey, keylength, lengthbyte as u64)
    } else {
        // short strings
        if lengthbyte % core::mem::size_of::<u64>() == 0 {
            let mut acc =
                clmul_half_scalar_product_with_tail_without_reduction(rs64, string, length);
            let keylength =
                ptr::read_unaligned(rs64.add(M128_NEEDED_PER_BLOCK + 2) as *const u64);
            acc = _mm_xor_si128(acc, lazy_length_hash(keylength, lengthbyte as u64));
            precomp_reduction64(acc)
        } else {
            let lastword = create_last_word(lengthbyte, string.add(length));
            let mut acc = clmul_half_scalar_product_with_tail_without_reduction_with_extra_word(
                rs64, string, length, lastword,
            );
            let keylength =
                ptr::read_unaligned(rs64.add(M128_NEEDED_PER_BLOCK + 2) as *const u64);
            acc = _mm_xor_si128(acc, lazy_length_hash(keylength, lengthbyte as u64));
            precomp_reduction64(acc)
        }
    }
}

pub fn clhash(random: &[u8], stringbyte: &[u8]) -> u64 {
    unsafe { clhash_inner(random, stringbyte) }
}

#[inline]
fn xorshift128plus(part1: &mut u64, part2: &mut u64) -> u64 {
    let mut s1 = *part1;
    let s0 = *part2;
    *part1 = s0;
    s1 ^= s1 << 23; // a
    *part2 = s1 ^ s0 ^ (s1 >> 18) ^ (s0 >> 5); // b, c
    (*part2).wrapping_add(s0)
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
    let mut bytes = vec![0u8; RANDOM_BYTES_NEEDED_FOR_CLHASH];
    unsafe {
        ptr::copy_nonoverlapping(
            a64.as_ptr() as *const u8,
            bytes.as_mut_ptr(),
            RANDOM_BYTES_NEEDED_FOR_CLHASH,
        );
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
        // Vec<u8> handles its own deallocation; nothing to do here.
    }
}
