// Importing required modules
use crate::std;
// Constants
pub const TOTP_EXPORT: &str = "__attribute__((visibility(\"default\")))";

const TOTP_OK: i32 = 0;
const TOTP_EBOUNDS: i32 = 1;

// Function Declarations
pub fn hotp(key: &[u8; 64], counter: u64) -> i32 {
    let mut data: [u8; 8] = [0; 8];
    let mut hash: [u8; 20] = [0; 20];

    unpack64(counter, &mut data);
    hmac_sha1(key, &data, 8, &mut hash);

    let offset = (hash[19] & 0xF) as usize;
    let four: [u8; 4] = [
        hash[offset],
        hash[offset + 1],
        hash[offset + 2],
        hash[offset + 3],
    ];
    let trunc = pack32(&four) & 0x7FFFFFFF;

    (trunc % 1_000_000) as i32
}

pub fn sha1(buf: &mut [u8], len: usize, cap: usize, hash: &mut [u8; 20]) -> i32 {
    // 4.2.1, use k[t/20]
    let k: [u32; 4] = [0x5A827999, 0x6ED9EBA1, 0x8F1BBCDC, 0xCA62C1D6];

    // 5.1.1 (padding)
    // add 1 byte for stop bit, 8 for length, pad to 64 bytes
    if len > std::SIZE_MAX - 9 - 63 {
        return TOTP_EBOUNDS;
    }
    let new_len = (len + 9 + 63) / 64 * 64; // ceil len+9 to 64 multiple
    if new_len > cap {
        return TOTP_EBOUNDS;
    }

    // memset(buf+len, 0, new_len-len)
    for i in len..new_len {
        buf[i] = 0;
    }
    buf[len] = 1u8 << 7;

    // unpack64(len*8, &buf[new_len-8])
    let len_bits = (len as u64).wrapping_mul(8);
    let pos = new_len - 8;
    let mut tmp8: [u8; 8] = [0; 8];
    unpack64(len_bits, &mut tmp8);
    for j in 0..8 {
        buf[pos + j] = tmp8[j];
    }

    // 5.3.1
    let mut h: [u32; 5] = [
        0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476, 0xC3D2E1F0,
    ];

    let mut w: [u32; 80] = [0; 80];

    // 6.1.2
    let blocks = new_len / 64;
    for i in 0..blocks {
        for t in 0..16 {
            let base = i * 64 + t * 4;
            let four: [u8; 4] = [buf[base], buf[base + 1], buf[base + 2], buf[base + 3]];
            w[t] = pack32(&four);
        }
        for t in 16..80 {
            w[t] = rotl(w[t - 3] ^ w[t - 8] ^ w[t - 14] ^ w[t - 16], 1);
        }

        let mut a = h[0];
        let mut b = h[1];
        let mut c = h[2];
        let mut d = h[3];
        let mut e = h[4];

        for t in 0..80 {
            // 4.1.1 (f function)
            let f = if t < 20 {
                (b & c) ^ ((!b) & d)
            } else if t < 40 {
                b ^ c ^ d
            } else if t < 60 {
                (b & c) ^ (b & d) ^ (c & d)
            } else {
                b ^ c ^ d
            };

            let big_t = rotl(a, 5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k[t / 20])
                .wrapping_add(w[t]);
            e = d;
            d = c;
            c = rotl(b, 30);
            b = a;
            a = big_t;
        }

        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
    }

    for i in 0..5 {
        let mut tmp4: [u8; 4] = [0; 4];
        unpack32(h[i], &mut tmp4);
        hash[i * 4] = tmp4[0];
        hash[i * 4 + 1] = tmp4[1];
        hash[i * 4 + 2] = tmp4[2];
        hash[i * 4 + 3] = tmp4[3];
    }

    TOTP_OK
}

pub fn from_base32(s: &str, buf: &mut [u8], cap: usize) -> usize {
    let len = std::strlen(s);
    if len % 8 != 0 {
        return 0;
    }
    if cap < (len + 1) / 8 * 5 {
        return 0;
    }

    let bytes = s.as_bytes();
    let mut v: [u8; 8] = [0; 8];
    let n = len / 8;

    for i in 0..n {
        // s[i*8] is non-zero by construction (strlen stops at NUL)
        for j in 0..8 {
            let c = bytes[i * 8 + j];
            if c == b'=' {
                v[j] = 0;
            } else if c >= b'A' && c <= b'Z' {
                v[j] = c - b'A';
            } else if c >= b'a' && c <= b'z' {
                v[j] = c - b'a';
            } else if c >= b'2' && c <= b'7' {
                v[j] = c - b'2' + 26;
            } else {
                return 0;
            }
        }

        buf[i * 5] = (((v[0] as u32) << 3) | ((v[1] as u32) >> 2)) as u8;
        buf[i * 5 + 1] =
            (((v[1] as u32) << 6) | ((v[2] as u32) << 1) | ((v[3] as u32) >> 4)) as u8;
        buf[i * 5 + 2] = (((v[3] as u32) << 4) | ((v[4] as u32) >> 1)) as u8;
        buf[i * 5 + 3] =
            (((v[4] as u32) << 7) | ((v[5] as u32) << 2) | ((v[6] as u32) >> 3)) as u8;
        buf[i * 5 + 4] = (((v[6] as u32) << 5) | (v[7] as u32)) as u8;

        if bytes[i * 8 + 2] == b'=' {
            return i * 5 + 1;
        }
        if bytes[i * 8 + 4] == b'=' {
            return i * 5 + 2;
        }
        if bytes[i * 8 + 5] == b'=' {
            return i * 5 + 3;
        }
        if bytes[i * 8 + 7] == b'=' {
            return i * 5 + 4;
        }
    }

    n * 5
}

pub fn unpack64(x: u64, a: &mut [u8; 8]) {
    let hi = (x >> 32) as u32;
    let lo = x as u32;
    let mut tmp: [u8; 4] = [0; 4];
    unpack32(hi, &mut tmp);
    a[0] = tmp[0];
    a[1] = tmp[1];
    a[2] = tmp[2];
    a[3] = tmp[3];
    unpack32(lo, &mut tmp);
    a[4] = tmp[0];
    a[5] = tmp[1];
    a[6] = tmp[2];
    a[7] = tmp[3];
}

pub fn pack32(a: &[u8; 4]) -> u32 {
    ((a[0] as u32) << 24) | ((a[1] as u32) << 16) | ((a[2] as u32) << 8) | (a[3] as u32)
}

pub fn unpack32(x: u32, a: &mut [u8; 4]) {
    a[0] = (x >> 24) as u8;
    a[1] = (x >> 16) as u8;
    a[2] = (x >> 8) as u8;
    a[3] = x as u8;
}

pub fn totp(key: &[u8; 64], time: u64) -> i32 {
    hotp(key, time / 30)
}

pub fn hmac_sha1(key: &[u8; 64], data: &[u8], len: usize, hash: &mut [u8; 20]) -> i32 {
    if len > 64 {
        return TOTP_EBOUNDS;
    }

    let mut buf: [u8; 196] = [0; 196];

    for i in 0..64 {
        buf[i] = key[i] ^ 0x36;
    }
    for i in 0..len {
        buf[64 + i] = data[i];
    }
    sha1(&mut buf, 64 + len, 196, hash);

    for i in 0..64 {
        buf[i] = key[i] ^ 0x5C;
    }
    for i in 0..20 {
        buf[64 + i] = hash[i];
    }
    sha1(&mut buf, 64 + 20, 196, hash);

    TOTP_OK
}

pub fn rotl(x: u32, n: i32) -> u32 {
    // FIPS 180-3 2.2.2: x << n | x >> (32-n)
    // For n in 1..=31, this is equivalent to rotate_left.
    x.rotate_left(n as u32)
}
