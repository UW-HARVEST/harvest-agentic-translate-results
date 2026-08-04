pub const RANDOM_64BITWORDS_NEEDED_FOR_CLHASH: usize = 133;
pub const RANDOM_BYTES_NEEDED_FOR_CLHASH: usize = 133 * 8;

const WORD_BYTES: usize = core::mem::size_of::<u64>();
const BLOCK_WORDS: usize = 128;
const BLOCK_M128S: usize = BLOCK_WORDS / 2;

fn read_u64_le(bytes: &[u8]) -> u64 {
    let mut word = [0u8; WORD_BYTES];
    word[..bytes.len()].copy_from_slice(bytes);
    u64::from_le_bytes(word)
}

fn pack_u128(low: u64, high: u64) -> u128 {
    u128::from(low) | (u128::from(high) << 64)
}

fn low64(value: u128) -> u64 {
    value as u64
}

fn high64(value: u128) -> u64 {
    (value >> 64) as u64
}

fn clmul64(lhs: u64, mut rhs: u64) -> u128 {
    let mut product = 0u128;
    while rhs != 0 {
        let shift = rhs.trailing_zeros();
        product ^= u128::from(lhs) << shift;
        rhs &= rhs - 1;
    }
    product
}

fn clmul128(lhs: u128, rhs: u128) -> (u128, u128) {
    let low = clmul64(low64(lhs), low64(rhs));
    let high = clmul64(high64(lhs), high64(rhs));
    let mix = clmul64(low64(lhs), high64(rhs)) ^ clmul64(high64(lhs), low64(rhs));
    let low_part = low ^ (mix << 64);
    let high_part = high ^ (mix >> 64);
    (low_part, high_part)
}

fn lazymod127(low: u128, high: u128) -> u128 {
    low ^ (high << 1) ^ (high << 2)
}

fn mul128by128to128_lazymod127(lhs: u128, rhs: u128) -> u128 {
    let (low, high) = clmul128(lhs, rhs);
    lazymod127(low, high)
}

fn lazy_length_hash(keylength: u64, length: u64) -> u128 {
    clmul64(keylength, length)
}

fn precomp_reduction64(value: u128) -> u64 {
    let mut reduced = value;
    let modulus_low = (1u128 << 4) | (1u128 << 3) | (1u128 << 1) | 1;
    for bit in (64..128).rev() {
        if ((reduced >> bit) & 1) != 0 {
            reduced ^= 1u128 << bit;
            reduced ^= modulus_low << (bit - 64);
        }
    }
    reduced as u64
}

fn simple128to64hashwithlength(value: u128, key: u128, keylength: u64, length: u64) -> u64 {
    let mixed = value ^ key;
    let total = clmul64(high64(mixed), low64(mixed)) ^ lazy_length_hash(keylength, length);
    precomp_reduction64(total)
}

fn random_block(random: &[u8], index: usize) -> u128 {
    let offset = index * 16;
    pack_u128(
        read_u64_le(&random[offset..offset + 8]),
        read_u64_le(&random[offset + 8..offset + 16]),
    )
}

fn word_block(words: &[u64], start: usize) -> u128 {
    pack_u128(words[start], words[start + 1])
}

fn block_with_low_word(word: u64) -> u128 {
    pack_u128(word, 0)
}

fn block_with_extra(word: u64, extra: u64) -> u128 {
    pack_u128(word, extra)
}

fn halfscalarproduct_without_reduction(random: &[u8], words: &[u64], length: usize) -> u128 {
    let mut acc = 0u128;
    let mut random_index = 0usize;
    let mut word_index = 0usize;

    while word_index + 3 < length {
        let add1 = random_block(random, random_index) ^ word_block(words, word_index);
        acc ^= clmul64(high64(add1), low64(add1));

        let add2 = random_block(random, random_index + 1) ^ word_block(words, word_index + 2);
        acc ^= clmul64(high64(add2), low64(add2));

        random_index += 2;
        word_index += 4;
    }

    acc
}

fn halfscalarproduct_with_tail_without_reduction(
    random: &[u8],
    words: &[u64],
    length: usize,
) -> u128 {
    let mut acc = 0u128;
    let mut random_index = 0usize;
    let mut word_index = 0usize;

    while word_index + 3 < length {
        let add1 = random_block(random, random_index) ^ word_block(words, word_index);
        acc ^= clmul64(high64(add1), low64(add1));

        let add2 = random_block(random, random_index + 1) ^ word_block(words, word_index + 2);
        acc ^= clmul64(high64(add2), low64(add2));

        random_index += 2;
        word_index += 4;
    }

    if word_index + 1 < length {
        let add = random_block(random, random_index) ^ word_block(words, word_index);
        acc ^= clmul64(high64(add), low64(add));
        random_index += 1;
        word_index += 2;
    }

    if word_index < length {
        let add = random_block(random, random_index) ^ block_with_low_word(words[word_index]);
        acc ^= clmul64(high64(add), low64(add));
    }

    acc
}

fn halfscalarproduct_with_tail_without_reduction_with_extra_word(
    random: &[u8],
    words: &[u64],
    length: usize,
    extra_word: u64,
) -> u128 {
    let mut acc = 0u128;
    let mut random_index = 0usize;
    let mut word_index = 0usize;

    while word_index + 3 < length {
        let add1 = random_block(random, random_index) ^ word_block(words, word_index);
        acc ^= clmul64(high64(add1), low64(add1));

        let add2 = random_block(random, random_index + 1) ^ word_block(words, word_index + 2);
        acc ^= clmul64(high64(add2), low64(add2));

        random_index += 2;
        word_index += 4;
    }

    if word_index + 1 < length {
        let add = random_block(random, random_index) ^ word_block(words, word_index);
        acc ^= clmul64(high64(add), low64(add));
        random_index += 1;
        word_index += 2;
    }

    let add = if word_index < length {
        random_block(random, random_index) ^ block_with_extra(words[word_index], extra_word)
    } else {
        random_block(random, random_index) ^ block_with_low_word(extra_word)
    };
    acc ^ clmul64(high64(add), low64(add))
}

fn halfscalarproduct_only_extra_word(random: &[u8], extra_word: u64) -> u128 {
    let add = random_block(random, 0) ^ block_with_low_word(extra_word);
    clmul64(high64(add), low64(add))
}

fn create_last_word(input: &[u8]) -> u64 {
    read_u64_le(input)
}

pub fn clhash(random: &[u8], stringbyte: &[u8]) -> u64 {
    assert!(random.len() >= RANDOM_BYTES_NEEDED_FOR_CLHASH);

    let complete_word_count = stringbyte.len() / WORD_BYTES;
    let lengthinc = stringbyte.len().div_ceil(WORD_BYTES);
    let mut words = Vec::with_capacity(complete_word_count);
    for chunk in stringbyte.chunks_exact(WORD_BYTES) {
        words.push(read_u64_le(chunk));
    }
    let remainder = &stringbyte[complete_word_count * WORD_BYTES..];

    let mut polyvalue = random_block(random, BLOCK_M128S);
    polyvalue &= pack_u128(u64::MAX, 0x3fff_ffff_ffff_ffff);

    if BLOCK_WORDS < lengthinc {
        let mut acc = halfscalarproduct_without_reduction(random, &words, BLOCK_WORDS);
        let mut t = BLOCK_WORDS;

        while t + BLOCK_WORDS <= complete_word_count {
            acc = mul128by128to128_lazymod127(polyvalue, acc);
            let h1 = halfscalarproduct_without_reduction(
                random,
                &words[t..t + BLOCK_WORDS],
                BLOCK_WORDS,
            );
            acc ^= h1;
            t += BLOCK_WORDS;
        }

        let remain = complete_word_count - t;
        if remain != 0 {
            acc = mul128by128to128_lazymod127(polyvalue, acc);
            let h1 = if remainder.is_empty() {
                halfscalarproduct_with_tail_without_reduction(random, &words[t..], remain)
            } else {
                let lastword = create_last_word(remainder);
                halfscalarproduct_with_tail_without_reduction_with_extra_word(
                    random,
                    &words[t..],
                    remain,
                    lastword,
                )
            };
            acc ^= h1;
        } else if !remainder.is_empty() {
            acc = mul128by128to128_lazymod127(polyvalue, acc);
            let lastword = create_last_word(remainder);
            acc ^= halfscalarproduct_only_extra_word(random, lastword);
        }

        let finalkey = random_block(random, BLOCK_M128S + 1);
        let keylength = read_u64_le(&random[(BLOCK_M128S + 2) * 16..(BLOCK_M128S + 2) * 16 + 8]);
        simple128to64hashwithlength(acc, finalkey, keylength, stringbyte.len() as u64)
    } else {
        let keylength = read_u64_le(&random[(BLOCK_M128S + 2) * 16..(BLOCK_M128S + 2) * 16 + 8]);
        let acc = if remainder.is_empty() {
            halfscalarproduct_with_tail_without_reduction(random, &words, complete_word_count)
        } else {
            let lastword = create_last_word(remainder);
            halfscalarproduct_with_tail_without_reduction_with_extra_word(
                random,
                &words,
                complete_word_count,
                lastword,
            )
        };
        precomp_reduction64(acc ^ lazy_length_hash(keylength, stringbyte.len() as u64))
    }
}

#[derive(Clone, Copy)]
struct XorShift128PlusKey {
    part1: u64,
    part2: u64,
}

impl XorShift128PlusKey {
    fn new(part1: u64, part2: u64) -> Self {
        Self { part1, part2 }
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
    let mut key = XorShift128PlusKey::new(seed1, seed2);
    let mut words = [0u64; RANDOM_64BITWORDS_NEEDED_FOR_CLHASH];
    for word in &mut words {
        *word = key.next();
    }
    while words[128] == 0 && words[129] == 1 {
        words[128] = key.next();
        words[129] = key.next();
    }

    let mut bytes = Vec::with_capacity(RANDOM_BYTES_NEEDED_FOR_CLHASH);
    for word in words {
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
    fn drop(&mut self) {}
}
