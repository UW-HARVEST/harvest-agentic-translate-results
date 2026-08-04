pub const RANDOM_64BITWORDS_NEEDED_FOR_CLHASH: usize = 133;
pub const RANDOM_BYTES_NEEDED_FOR_CLHASH: usize = 133 * 8;

#[derive(Clone, Copy, Default)]
struct Block128 {
    low: u64,
    high: u64,
}

impl Block128 {
    fn xor(self, other: Self) -> Self {
        Self {
            low: self.low ^ other.low,
            high: self.high ^ other.high,
        }
    }

    fn shift_left(self, amount: u32) -> Self {
        match amount {
            0 => self,
            1..=63 => Self {
                low: self.low << amount,
                high: (self.high << amount) | (self.low >> (64 - amount)),
            },
            64 => Self {
                low: 0,
                high: self.low,
            },
            65..=127 => Self {
                low: 0,
                high: self.low << (amount - 64),
            },
            _ => Self::default(),
        }
    }

    fn to_u128(self) -> u128 {
        self.low as u128 | ((self.high as u128) << 64)
    }

    fn from_u128(value: u128) -> Self {
        Self {
            low: value as u64,
            high: (value >> 64) as u64,
        }
    }
}

fn read_u64_le(bytes: &[u8]) -> u64 {
    let mut word = [0u8; 8];
    word.copy_from_slice(bytes);
    u64::from_le_bytes(word)
}

fn parse_words(bytes: &[u8]) -> [u64; RANDOM_64BITWORDS_NEEDED_FOR_CLHASH] {
    let mut words = [0u64; RANDOM_64BITWORDS_NEEDED_FOR_CLHASH];
    let full_words = (bytes.len() / 8).min(RANDOM_64BITWORDS_NEEDED_FOR_CLHASH);
    for (idx, chunk) in bytes.chunks_exact(8).take(full_words).enumerate() {
        words[idx] = read_u64_le(chunk);
    }
    words
}

fn xorshift128plus(state: &mut (u64, u64)) -> u64 {
    let mut s1 = state.0;
    let s0 = state.1;
    state.0 = s0;
    s1 ^= s1 << 23;
    state.1 = s1 ^ s0 ^ (s1 >> 18) ^ (s0 >> 5);
    state.1.wrapping_add(s0)
}

fn generate_random_words(seed1: u64, seed2: u64) -> [u64; RANDOM_64BITWORDS_NEEDED_FOR_CLHASH] {
    let mut state = (seed1, seed2);
    let mut words = [0u64; RANDOM_64BITWORDS_NEEDED_FOR_CLHASH];
    for word in &mut words {
        *word = xorshift128plus(&mut state);
    }
    while words[128] == 0 && words[129] == 1 {
        words[128] = xorshift128plus(&mut state);
        words[129] = xorshift128plus(&mut state);
    }
    words
}

fn normalized_random_words(random: &[u8]) -> [u64; RANDOM_64BITWORDS_NEEDED_FOR_CLHASH] {
    if random.len() >= RANDOM_BYTES_NEEDED_FOR_CLHASH {
        return parse_words(&random[..RANDOM_BYTES_NEEDED_FOR_CLHASH]);
    }

    let mut seed_bytes = [0u8; 16];
    let seed_len = random.len().min(seed_bytes.len());
    seed_bytes[..seed_len].copy_from_slice(&random[..seed_len]);
    let seed1 = read_u64_le(&seed_bytes[..8]);
    let seed2 = read_u64_le(&seed_bytes[8..]);
    generate_random_words(seed1, seed2)
}

fn block_from_words(words: &[u64], index: usize) -> Block128 {
    Block128 {
        low: words[index * 2],
        high: words[index * 2 + 1],
    }
}

fn clmul64(a: u64, b: u64) -> Block128 {
    let mut result = 0u128;
    let mut bits = b;
    let mut shift = 0u32;
    while bits != 0 {
        if bits & 1 != 0 {
            result ^= (a as u128) << shift;
        }
        bits >>= 1;
        shift += 1;
    }
    Block128::from_u128(result)
}

fn clmul_select(a: Block128, b: Block128, imm: u8) -> Block128 {
    let lhs = if imm & 0x01 == 0 { a.low } else { a.high };
    let rhs = if imm & 0x10 == 0 { b.low } else { b.high };
    clmul64(lhs, rhs)
}

fn lazymod127(low: Block128, high: Block128) -> Block128 {
    low.xor(high.shift_left(1)).xor(high.shift_left(2))
}

fn mul128by128to128_lazymod127(a: Block128, b: Block128) -> Block128 {
    let amix1 = clmul_select(a, b, 0x01);
    let amix2 = clmul_select(a, b, 0x10);
    let mut low = clmul_select(a, b, 0x00);
    let mut high = clmul_select(a, b, 0x11);
    let amix = amix1.xor(amix2);
    low = low.xor(Block128 {
        low: 0,
        high: amix.low,
    });
    high = high.xor(Block128 {
        low: amix.high,
        high: 0,
    });
    lazymod127(low, high)
}

fn lazy_length_hash(keylength: u64, length: u64) -> Block128 {
    clmul64(length, keylength)
}

fn precomp_reduction64(a: Block128) -> u64 {
    let mut value = a.to_u128();
    let modulus = (1u128 << 64) | (1u128 << 4) | (1u128 << 3) | (1u128 << 1) | 1u128;
    for bit in (64..128).rev() {
        if (value >> bit) & 1 != 0 {
            value ^= modulus << (bit - 64);
        }
    }
    value as u64
}

fn simple128to64hashwithlength(value: Block128, key: Block128, keylength: u64, length: u64) -> u64 {
    let add = value.xor(key);
    let total = clmul_select(add, add, 0x10).xor(lazy_length_hash(keylength, length));
    precomp_reduction64(total)
}

fn complete_words(bytes: &[u8]) -> Vec<u64> {
    bytes.chunks_exact(8).map(read_u64_le).collect()
}

fn clmul_pair(key: Block128, low: u64, high: u64) -> Block128 {
    let add = key.xor(Block128 { low, high });
    clmul_select(add, add, 0x10)
}

fn clmulhalfscalarproductwithoutreduction(
    randomsource: &[u64; RANDOM_64BITWORDS_NEEDED_FOR_CLHASH],
    string: &[u64],
    length: usize,
) -> Block128 {
    let mut acc = Block128::default();
    let mut rs_idx = 0usize;
    let mut s_idx = 0usize;
    while s_idx + 3 < length {
        acc = acc.xor(clmul_pair(
            block_from_words(randomsource, rs_idx),
            string[s_idx],
            string[s_idx + 1],
        ));
        acc = acc.xor(clmul_pair(
            block_from_words(randomsource, rs_idx + 1),
            string[s_idx + 2],
            string[s_idx + 3],
        ));
        rs_idx += 2;
        s_idx += 4;
    }
    acc
}

fn clmulhalfscalarproductwithtailwithoutreduction(
    randomsource: &[u64; RANDOM_64BITWORDS_NEEDED_FOR_CLHASH],
    string: &[u64],
    length: usize,
) -> Block128 {
    let mut acc = Block128::default();
    let mut rs_idx = 0usize;
    let mut s_idx = 0usize;
    while s_idx + 3 < length {
        acc = acc.xor(clmul_pair(
            block_from_words(randomsource, rs_idx),
            string[s_idx],
            string[s_idx + 1],
        ));
        acc = acc.xor(clmul_pair(
            block_from_words(randomsource, rs_idx + 1),
            string[s_idx + 2],
            string[s_idx + 3],
        ));
        rs_idx += 2;
        s_idx += 4;
    }
    if s_idx + 1 < length {
        acc = acc.xor(clmul_pair(
            block_from_words(randomsource, rs_idx),
            string[s_idx],
            string[s_idx + 1],
        ));
        rs_idx += 1;
        s_idx += 2;
    }
    if s_idx < length {
        acc = acc.xor(clmul_pair(
            block_from_words(randomsource, rs_idx),
            string[s_idx],
            0,
        ));
    }
    acc
}

fn clmulhalfscalarproductwithtailwithoutreduction_with_extra_word(
    randomsource: &[u64; RANDOM_64BITWORDS_NEEDED_FOR_CLHASH],
    string: &[u64],
    length: usize,
    extraword: u64,
) -> Block128 {
    let mut acc = Block128::default();
    let mut rs_idx = 0usize;
    let mut s_idx = 0usize;
    while s_idx + 3 < length {
        acc = acc.xor(clmul_pair(
            block_from_words(randomsource, rs_idx),
            string[s_idx],
            string[s_idx + 1],
        ));
        acc = acc.xor(clmul_pair(
            block_from_words(randomsource, rs_idx + 1),
            string[s_idx + 2],
            string[s_idx + 3],
        ));
        rs_idx += 2;
        s_idx += 4;
    }
    if s_idx + 1 < length {
        acc = acc.xor(clmul_pair(
            block_from_words(randomsource, rs_idx),
            string[s_idx],
            string[s_idx + 1],
        ));
        rs_idx += 1;
        s_idx += 2;
    }
    if s_idx < length {
        acc = acc.xor(clmul_pair(
            block_from_words(randomsource, rs_idx),
            string[s_idx],
            extraword,
        ));
    } else {
        let key = block_from_words(randomsource, rs_idx);
        acc = acc.xor(clmul_select(
            key,
            Block128 {
                low: extraword,
                high: 0,
            },
            0x01,
        ));
    }
    acc
}

fn clmulhalfscalarproduct_only_extra_word(
    randomsource: &[u64; RANDOM_64BITWORDS_NEEDED_FOR_CLHASH],
    extraword: u64,
) -> Block128 {
    let key = block_from_words(randomsource, 0);
    clmul_select(
        key,
        Block128 {
            low: extraword,
            high: 0,
        },
        0x01,
    )
}

fn create_last_word(bytes: &[u8]) -> u64 {
    let mut lastword = [0u8; 8];
    lastword[..bytes.len()].copy_from_slice(bytes);
    u64::from_le_bytes(lastword)
}

pub fn clhash(random: &[u8], stringbyte: &[u8]) -> u64 {
    let m = 128usize;
    let m128neededperblock = m / 2;
    let random_words = normalized_random_words(random);
    let mut polyvalue = block_from_words(&random_words, m128neededperblock);
    polyvalue.high &= 0x3fff_ffff_ffff_ffff;

    let lengthbyte = stringbyte.len();
    let length = lengthbyte / 8;
    let lengthinc = lengthbyte.div_ceil(8);
    let string_words = complete_words(&stringbyte[..length * 8]);

    if m < lengthinc {
        let mut acc = clmulhalfscalarproductwithoutreduction(&random_words, &string_words, m);
        let mut t = m;
        while t + m <= length {
            acc = mul128by128to128_lazymod127(polyvalue, acc);
            let h1 = clmulhalfscalarproductwithoutreduction(&random_words, &string_words[t..], m);
            acc = acc.xor(h1);
            t += m;
        }

        let remain = length - t;
        if remain != 0 {
            acc = mul128by128to128_lazymod127(polyvalue, acc);
            let h1 = if lengthbyte % 8 == 0 {
                clmulhalfscalarproductwithtailwithoutreduction(
                    &random_words,
                    &string_words[t..],
                    remain,
                )
            } else {
                let lastword = create_last_word(&stringbyte[length * 8..]);
                clmulhalfscalarproductwithtailwithoutreduction_with_extra_word(
                    &random_words,
                    &string_words[t..],
                    remain,
                    lastword,
                )
            };
            acc = acc.xor(h1);
        } else if lengthbyte % 8 != 0 {
            acc = mul128by128to128_lazymod127(polyvalue, acc);
            let lastword = create_last_word(&stringbyte[length * 8..]);
            acc = acc.xor(clmulhalfscalarproduct_only_extra_word(
                &random_words,
                lastword,
            ));
        }

        let finalkey = block_from_words(&random_words, m128neededperblock + 1);
        let keylength = random_words[132];
        simple128to64hashwithlength(acc, finalkey, keylength, lengthbyte as u64)
    } else if lengthbyte % 8 == 0 {
        let keylength = random_words[132];
        let acc =
            clmulhalfscalarproductwithtailwithoutreduction(&random_words, &string_words, length)
                .xor(lazy_length_hash(keylength, lengthbyte as u64));
        precomp_reduction64(acc)
    } else {
        let lastword = create_last_word(&stringbyte[length * 8..]);
        let keylength = random_words[132];
        let acc = clmulhalfscalarproductwithtailwithoutreduction_with_extra_word(
            &random_words,
            &string_words,
            length,
            lastword,
        )
        .xor(lazy_length_hash(keylength, lengthbyte as u64));
        precomp_reduction64(acc)
    }
}
pub fn get_random_key_for_clhash(seed1: u64, seed2: u64) -> Vec<u8> {
    let words = generate_random_words(seed1, seed2);
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
    fn drop(&mut self) {
        self.random_data.fill(0);
    }
}
