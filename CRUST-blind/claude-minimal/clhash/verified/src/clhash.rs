use core::arch::x86_64::*;

pub const RANDOM_64BITWORDS_NEEDED_FOR_CLHASH: usize = 133;
pub const RANDOM_BYTES_NEEDED_FOR_CLHASH: usize = 133 * 8;

// computes a << 1
#[inline]
#[target_feature(enable = "sse2")]
unsafe fn leftshift1(a: __m128i) -> __m128i {
    let u64shift = _mm_slli_epi64::<1>(a);
    let topbits = _mm_slli_si128::<8>(_mm_srli_epi64::<63>(a));
    _mm_or_si128(u64shift, topbits)
}

// computes a << 2
#[inline]
#[target_feature(enable = "sse2")]
unsafe fn leftshift2(a: __m128i) -> __m128i {
    let u64shift = _mm_slli_epi64::<2>(a);
    let topbits = _mm_slli_si128::<8>(_mm_srli_epi64::<62>(a));
    _mm_or_si128(u64shift, topbits)
}

#[inline]
#[target_feature(enable = "sse2")]
unsafe fn lazymod127(alow: __m128i, ahigh: __m128i) -> __m128i {
    let shift1 = leftshift1(ahigh);
    let shift2 = leftshift2(ahigh);
    _mm_xor_si128(_mm_xor_si128(alow, shift1), shift2)
}

// multiplication with lazy reduction
#[inline]
#[target_feature(enable = "sse2,pclmulqdq")]
unsafe fn mul128by128to128_lazymod127(a: __m128i, b: __m128i) -> __m128i {
    let amix1 = _mm_clmulepi64_si128::<0x01>(a, b);
    let amix2 = _mm_clmulepi64_si128::<0x10>(a, b);
    let alow = _mm_clmulepi64_si128::<0x00>(a, b);
    let ahigh = _mm_clmulepi64_si128::<0x11>(a, b);
    let amix = _mm_xor_si128(amix1, amix2);
    let amix1_l = _mm_slli_si128::<8>(amix);
    let amix2_l = _mm_srli_si128::<8>(amix);
    let alow = _mm_xor_si128(alow, amix1_l);
    let ahigh = _mm_xor_si128(ahigh, amix2_l);
    lazymod127(alow, ahigh)
}

// multiply the length and some key, no modulo
#[inline]
#[target_feature(enable = "sse2,pclmulqdq")]
unsafe fn lazy_length_hash(keylength: u64, length: u64) -> __m128i {
    let lengthvector = _mm_set_epi64x(keylength as i64, length as i64);
    _mm_clmulepi64_si128::<0x10>(lengthvector, lengthvector)
}

// modulo reduction to 64-bit value. The high 64 bits contain garbage.
#[inline]
#[target_feature(enable = "sse2,ssse3,pclmulqdq")]
unsafe fn precomp_reduction64_si128(a: __m128i) -> __m128i {
    let c = _mm_cvtsi64_si128(((1u32 << 4) + (1u32 << 3) + (1u32 << 1) + (1u32 << 0)) as i64);
    let q2 = _mm_clmulepi64_si128::<0x01>(a, c);
    let q3 = _mm_shuffle_epi8(
        _mm_setr_epi8(
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
        ),
        _mm_srli_si128::<8>(q2),
    );
    let q4 = _mm_xor_si128(q2, a);
    _mm_xor_si128(q3, q4)
}

#[inline]
#[target_feature(enable = "sse2,ssse3,pclmulqdq")]
unsafe fn precomp_reduction64(a: __m128i) -> u64 {
    _mm_cvtsi128_si64(precomp_reduction64_si128(a)) as u64
}

#[inline]
#[target_feature(enable = "sse2,ssse3,pclmulqdq")]
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

// For use with CLHASH
// length is # of u64 words; expected to be 128 (divisible by 4)
#[inline]
#[target_feature(enable = "sse2,sse3,pclmulqdq")]
unsafe fn clmul_half_scalar_product_without_reduction(
    randomsource: *const u8,
    string: *const u64,
    length: usize,
) -> __m128i {
    let mut acc = _mm_setzero_si128();
    let endstring = string.add(length);
    let mut rs = randomsource as *const __m128i;
    let mut s = string;
    while s.add(3) < endstring {
        let temp1 = _mm_loadu_si128(rs);
        let temp2 = _mm_lddqu_si128(s as *const __m128i);
        let add1 = _mm_xor_si128(temp1, temp2);
        let clprod1 = _mm_clmulepi64_si128::<0x10>(add1, add1);
        acc = _mm_xor_si128(clprod1, acc);
        let temp12 = _mm_loadu_si128(rs.add(1));
        let temp22 = _mm_lddqu_si128(s.add(2) as *const __m128i);
        let add12 = _mm_xor_si128(temp12, temp22);
        let clprod12 = _mm_clmulepi64_si128::<0x10>(add12, add12);
        acc = _mm_xor_si128(clprod12, acc);
        rs = rs.add(2);
        s = s.add(4);
    }
    acc
}

// length does not have to be divisible by 4
#[inline]
#[target_feature(enable = "sse2,sse3,pclmulqdq")]
unsafe fn clmul_half_scalar_product_with_tail_without_reduction(
    randomsource: *const u8,
    string: *const u64,
    length: usize,
) -> __m128i {
    let mut acc = _mm_setzero_si128();
    let endstring = string.add(length);
    let mut rs = randomsource as *const __m128i;
    let mut s = string;
    while s.add(3) < endstring {
        let temp1 = _mm_loadu_si128(rs);
        let temp2 = _mm_lddqu_si128(s as *const __m128i);
        let add1 = _mm_xor_si128(temp1, temp2);
        let clprod1 = _mm_clmulepi64_si128::<0x10>(add1, add1);
        acc = _mm_xor_si128(clprod1, acc);
        let temp12 = _mm_loadu_si128(rs.add(1));
        let temp22 = _mm_lddqu_si128(s.add(2) as *const __m128i);
        let add12 = _mm_xor_si128(temp12, temp22);
        let clprod12 = _mm_clmulepi64_si128::<0x10>(add12, add12);
        acc = _mm_xor_si128(clprod12, acc);
        rs = rs.add(2);
        s = s.add(4);
    }
    if s.add(1) < endstring {
        let temp1 = _mm_loadu_si128(rs);
        let temp2 = _mm_lddqu_si128(s as *const __m128i);
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

// length does not have to be divisible by 4, with extra word
#[inline]
#[target_feature(enable = "sse2,sse3,pclmulqdq")]
unsafe fn clmul_half_scalar_product_with_tail_without_reduction_with_extra_word(
    randomsource: *const u8,
    string: *const u64,
    length: usize,
    extraword: u64,
) -> __m128i {
    let mut acc = _mm_setzero_si128();
    let endstring = string.add(length);
    let mut rs = randomsource as *const __m128i;
    let mut s = string;
    while s.add(3) < endstring {
        let temp1 = _mm_loadu_si128(rs);
        let temp2 = _mm_lddqu_si128(s as *const __m128i);
        let add1 = _mm_xor_si128(temp1, temp2);
        let clprod1 = _mm_clmulepi64_si128::<0x10>(add1, add1);
        acc = _mm_xor_si128(clprod1, acc);
        let temp12 = _mm_loadu_si128(rs.add(1));
        let temp22 = _mm_lddqu_si128(s.add(2) as *const __m128i);
        let add12 = _mm_xor_si128(temp12, temp22);
        let clprod12 = _mm_clmulepi64_si128::<0x10>(add12, add12);
        acc = _mm_xor_si128(clprod12, acc);
        rs = rs.add(2);
        s = s.add(4);
    }
    if s.add(1) < endstring {
        let temp1 = _mm_loadu_si128(rs);
        let temp2 = _mm_lddqu_si128(s as *const __m128i);
        let add1 = _mm_xor_si128(temp1, temp2);
        let clprod1 = _mm_clmulepi64_si128::<0x10>(add1, add1);
        acc = _mm_xor_si128(clprod1, acc);
        rs = rs.add(1);
        s = s.add(2);
    }
    // append the extra word
    if s < endstring {
        let temp1 = _mm_loadu_si128(rs);
        let temp2 = _mm_set_epi64x(extraword as i64, *s as i64);
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
#[target_feature(enable = "sse2,pclmulqdq")]
unsafe fn clmul_half_scalar_product_only_extra_word(
    randomsource: *const u8,
    extraword: u64,
) -> __m128i {
    let temp1 = _mm_loadu_si128(randomsource as *const __m128i);
    let temp2 = _mm_loadl_epi64(&extraword as *const u64 as *const __m128i);
    let add1 = _mm_xor_si128(temp1, temp2);
    _mm_clmulepi64_si128::<0x01>(add1, add1)
}

// fill final partial word with 0s on the right
#[inline]
unsafe fn create_last_word(lengthbyte: usize, lastw: *const u64) -> u64 {
    let significantbytes = lengthbyte % core::mem::size_of::<u64>();
    let mut lastword: u64 = 0;
    core::ptr::copy_nonoverlapping(
        lastw as *const u8,
        &mut lastword as *mut u64 as *mut u8,
        significantbytes,
    );
    lastword
}

#[target_feature(enable = "sse2,sse3,ssse3,pclmulqdq")]
unsafe fn clhash_inner(random: *const u8, stringbyte: *const u8, lengthbyte: usize) -> u64 {
    const M: usize = 128; // process the data in chunks of 16 cache lines
    const M128_NEEDED_PER_BLOCK: usize = M / 2; // 64 __m128i words per block

    let rs64 = random;

    // load polyvalue from rs64 + M128_NEEDED_PER_BLOCK (in __m128i units)
    let polyvalue_ptr = rs64.add(M128_NEEDED_PER_BLOCK * 16) as *const __m128i;
    let polyvalue = _mm_loadu_si128(polyvalue_ptr);
    // setting two highest bits to zero
    let polyvalue = _mm_and_si128(
        polyvalue,
        _mm_setr_epi32(
            0xFFFFFFFFu32 as i32,
            0xFFFFFFFFu32 as i32,
            0xFFFFFFFFu32 as i32,
            0x3fffffff,
        ),
    );

    let length = lengthbyte / core::mem::size_of::<u64>(); // # of complete words
    let lengthinc =
        (lengthbyte + core::mem::size_of::<u64>() - 1) / core::mem::size_of::<u64>();

    let string = stringbyte as *const u64;

    if M < lengthinc {
        // long strings
        let mut acc = clmul_half_scalar_product_without_reduction(rs64, string, M);
        let mut t = M;
        while t + M <= length {
            // acc = polyvalue * acc + h1
            acc = mul128by128to128_lazymod127(polyvalue, acc);
            let h1 = clmul_half_scalar_product_without_reduction(rs64, string.add(t), M);
            acc = _mm_xor_si128(acc, h1);
            t += M;
        }
        let remain = length - t;

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
            // no completely filled words left, but one partial word remains
            acc = mul128by128to128_lazymod127(polyvalue, acc);
            let lastword = create_last_word(lengthbyte, string.add(length));
            let h1 = clmul_half_scalar_product_only_extra_word(rs64, lastword);
            acc = _mm_xor_si128(acc, h1);
        }

        let finalkey =
            _mm_loadu_si128(rs64.add((M128_NEEDED_PER_BLOCK + 1) * 16) as *const __m128i);
        let keylength = core::ptr::read_unaligned(
            rs64.add((M128_NEEDED_PER_BLOCK + 2) * 16) as *const u64,
        );
        simple128to64hashwithlength(acc, finalkey, keylength, lengthbyte as u64)
    } else {
        // short strings
        if lengthbyte % core::mem::size_of::<u64>() == 0 {
            let mut acc =
                clmul_half_scalar_product_with_tail_without_reduction(rs64, string, length);
            let keylength = core::ptr::read_unaligned(
                rs64.add((M128_NEEDED_PER_BLOCK + 2) * 16) as *const u64,
            );
            acc = _mm_xor_si128(acc, lazy_length_hash(keylength, lengthbyte as u64));
            return precomp_reduction64(acc);
        }
        let lastword = create_last_word(lengthbyte, string.add(length));
        let mut acc = clmul_half_scalar_product_with_tail_without_reduction_with_extra_word(
            rs64, string, length, lastword,
        );
        let keylength =
            core::ptr::read_unaligned(rs64.add((M128_NEEDED_PER_BLOCK + 2) * 16) as *const u64);
        acc = _mm_xor_si128(acc, lazy_length_hash(keylength, lengthbyte as u64));
        precomp_reduction64(acc)
    }
}

pub fn clhash(random: &[u8], stringbyte: &[u8]) -> u64 {
    assert!(random.len() >= RANDOM_BYTES_NEEDED_FOR_CLHASH);
    unsafe { clhash_inner(random.as_ptr(), stringbyte.as_ptr(), stringbyte.len()) }
}

// xorshift128plus PRNG used to generate the random key
struct Xorshift128PlusKey {
    part1: u64,
    part2: u64,
}

impl Xorshift128PlusKey {
    fn new(key1: u64, key2: u64) -> Self {
        Self {
            part1: key1,
            part2: key2,
        }
    }

    fn next(&mut self) -> u64 {
        let mut s1 = self.part1;
        let s0 = self.part2;
        self.part1 = s0;
        s1 ^= s1 << 23; // a
        self.part2 = s1 ^ s0 ^ (s1 >> 18) ^ (s0 >> 5); // b, c
        self.part2.wrapping_add(s0)
    }
}

pub fn get_random_key_for_clhash(seed1: u64, seed2: u64) -> Vec<u8> {
    let mut k = Xorshift128PlusKey::new(seed1, seed2);
    let mut a64 = vec![0u64; RANDOM_64BITWORDS_NEEDED_FOR_CLHASH];
    for i in 0..RANDOM_64BITWORDS_NEEDED_FOR_CLHASH {
        a64[i] = k.next();
    }
    while a64[128] == 0 && a64[129] == 1 {
        a64[128] = k.next();
        a64[129] = k.next();
    }
    let mut bytes = Vec::with_capacity(RANDOM_BYTES_NEEDED_FOR_CLHASH);
    for &word in &a64 {
        bytes.extend_from_slice(&word.to_le_bytes());
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
        // The Vec<u8> will be dropped automatically, freeing the random data.
    }
}
