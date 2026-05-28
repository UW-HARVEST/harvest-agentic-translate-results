// Pure-Rust SHA-256 / SHA-512 incremental + one-shot API matching the C
// `sha256_inc_*` / `sha512_inc_*` interface used by the SPHINCS+ reference.

use sha2::Digest;

#[inline]
fn store_be64(out: &mut [u8], v: u64) {
    out[..8].copy_from_slice(&v.to_be_bytes());
}

#[inline]
fn load_be64(input: &[u8]) -> u64 {
    let mut buf = [0u8; 8];
    buf.copy_from_slice(&input[..8]);
    u64::from_be_bytes(buf)
}

const IV_256: [u8; 32] = [
    0x6a, 0x09, 0xe6, 0x67, 0xbb, 0x67, 0xae, 0x85, 0x3c, 0x6e, 0xf3, 0x72, 0xa5, 0x4f, 0xf5, 0x3a,
    0x51, 0x0e, 0x52, 0x7f, 0x9b, 0x05, 0x68, 0x8c, 0x1f, 0x83, 0xd9, 0xab, 0x5b, 0xe0, 0xcd, 0x19,
];

const IV_512: [u8; 64] = [
    0x6a, 0x09, 0xe6, 0x67, 0xf3, 0xbc, 0xc9, 0x08, 0xbb, 0x67, 0xae, 0x85, 0x84, 0xca, 0xa7, 0x3b,
    0x3c, 0x6e, 0xf3, 0x72, 0xfe, 0x94, 0xf8, 0x2b, 0xa5, 0x4f, 0xf5, 0x3a, 0x5f, 0x1d, 0x36, 0xf1,
    0x51, 0x0e, 0x52, 0x7f, 0xad, 0xe6, 0x82, 0xd1, 0x9b, 0x05, 0x68, 0x8c, 0x2b, 0x3e, 0x6c, 0x1f,
    0x1f, 0x83, 0xd9, 0xab, 0xfb, 0x41, 0xbd, 0x6b, 0x5b, 0xe0, 0xcd, 0x19, 0x13, 0x7e, 0x21, 0x79,
];

pub fn sha256_inc_init(state: &mut [u8]) {
    state[..32].copy_from_slice(&IV_256);
    for b in state[32..40].iter_mut() {
        *b = 0;
    }
}

pub fn sha512_inc_init(state: &mut [u8]) {
    state[..64].copy_from_slice(&IV_512);
    for b in state[64..72].iter_mut() {
        *b = 0;
    }
}

// Pull in the SHA-256 / SHA-512 compression machinery from the `sha2` crate.
// We need to do block-by-block compression that updates an externally held
// 32-byte (or 64-byte) state. The `sha2` crate exposes the internal
// `compress256`/`compress512` low-level functions.
use sha2::compress256;
use sha2::compress512;

fn crypto_hashblocks_sha256(state: &mut [u8], input: &[u8]) {
    let mut s = [0u32; 8];
    for i in 0..8 {
        let mut buf = [0u8; 4];
        buf.copy_from_slice(&state[i * 4..i * 4 + 4]);
        s[i] = u32::from_be_bytes(buf);
    }

    let nblocks = input.len() / 64;
    if nblocks > 0 {
        // sha2 compress256 expects &[GenericArray<u8, U64>]
        // The sha2 crate uses generic-array. We can transmute.
        let blocks: &[[u8; 64]] = unsafe {
            core::slice::from_raw_parts(input.as_ptr() as *const [u8; 64], nblocks)
        };
        // GenericArray<u8, U64> has same layout as [u8; 64].
        let blocks_ga: &[sha2::digest::generic_array::GenericArray<
            u8,
            sha2::digest::generic_array::typenum::U64,
        >] = unsafe { core::mem::transmute(blocks) };
        compress256(&mut s, blocks_ga);
    }

    for i in 0..8 {
        state[i * 4..i * 4 + 4].copy_from_slice(&s[i].to_be_bytes());
    }
}

fn crypto_hashblocks_sha512(state: &mut [u8], input: &[u8]) {
    let mut s = [0u64; 8];
    for i in 0..8 {
        let mut buf = [0u8; 8];
        buf.copy_from_slice(&state[i * 8..i * 8 + 8]);
        s[i] = u64::from_be_bytes(buf);
    }

    let nblocks = input.len() / 128;
    if nblocks > 0 {
        let blocks: &[[u8; 128]] = unsafe {
            core::slice::from_raw_parts(input.as_ptr() as *const [u8; 128], nblocks)
        };
        let blocks_ga: &[sha2::digest::generic_array::GenericArray<
            u8,
            sha2::digest::generic_array::typenum::U128,
        >] = unsafe { core::mem::transmute(blocks) };
        compress512(&mut s, blocks_ga);
    }

    for i in 0..8 {
        state[i * 8..i * 8 + 8].copy_from_slice(&s[i].to_be_bytes());
    }
}

pub fn sha256_inc_blocks(state: &mut [u8], input: &[u8], inblocks: usize) {
    let mut bytes = load_be64(&state[32..40]);
    crypto_hashblocks_sha256(&mut state[..32], &input[..64 * inblocks]);
    bytes += 64 * inblocks as u64;
    store_be64(&mut state[32..40], bytes);
}

pub fn sha512_inc_blocks(state: &mut [u8], input: &[u8], inblocks: usize) {
    let mut bytes = load_be64(&state[64..72]);
    crypto_hashblocks_sha512(&mut state[..64], &input[..128 * inblocks]);
    bytes += 128 * inblocks as u64;
    store_be64(&mut state[64..72], bytes);
}

pub fn sha256_inc_finalize(out: &mut [u8], state: &mut [u8], input: &[u8], inlen: usize) {
    let mut padded = [0u8; 128];
    let bytes = load_be64(&state[32..40]) + inlen as u64;

    // Process full blocks first
    let full_block_bytes = inlen & !63;
    if full_block_bytes > 0 {
        crypto_hashblocks_sha256(&mut state[..32], &input[..full_block_bytes]);
    }
    let remaining_inlen = inlen - full_block_bytes;
    let in_remainder = &input[full_block_bytes..inlen];

    for i in 0..remaining_inlen {
        padded[i] = in_remainder[i];
    }
    padded[remaining_inlen] = 0x80;

    if remaining_inlen < 56 {
        for i in remaining_inlen + 1..56 {
            padded[i] = 0;
        }
        padded[56] = (bytes >> 53) as u8;
        padded[57] = (bytes >> 45) as u8;
        padded[58] = (bytes >> 37) as u8;
        padded[59] = (bytes >> 29) as u8;
        padded[60] = (bytes >> 21) as u8;
        padded[61] = (bytes >> 13) as u8;
        padded[62] = (bytes >> 5) as u8;
        padded[63] = (bytes << 3) as u8;
        crypto_hashblocks_sha256(&mut state[..32], &padded[..64]);
    } else {
        for i in remaining_inlen + 1..120 {
            padded[i] = 0;
        }
        padded[120] = (bytes >> 53) as u8;
        padded[121] = (bytes >> 45) as u8;
        padded[122] = (bytes >> 37) as u8;
        padded[123] = (bytes >> 29) as u8;
        padded[124] = (bytes >> 21) as u8;
        padded[125] = (bytes >> 13) as u8;
        padded[126] = (bytes >> 5) as u8;
        padded[127] = (bytes << 3) as u8;
        crypto_hashblocks_sha256(&mut state[..32], &padded[..128]);
    }

    out[..32].copy_from_slice(&state[..32]);
}

pub fn sha512_inc_finalize(out: &mut [u8], state: &mut [u8], input: &[u8], inlen: usize) {
    let mut padded = [0u8; 256];
    let bytes = load_be64(&state[64..72]) + inlen as u64;

    let full_block_bytes = inlen & !127;
    if full_block_bytes > 0 {
        crypto_hashblocks_sha512(&mut state[..64], &input[..full_block_bytes]);
    }
    let remaining_inlen = inlen - full_block_bytes;
    let in_remainder = &input[full_block_bytes..inlen];

    for i in 0..remaining_inlen {
        padded[i] = in_remainder[i];
    }
    padded[remaining_inlen] = 0x80;

    if remaining_inlen < 112 {
        for i in remaining_inlen + 1..119 {
            padded[i] = 0;
        }
        padded[119] = (bytes >> 61) as u8;
        padded[120] = (bytes >> 53) as u8;
        padded[121] = (bytes >> 45) as u8;
        padded[122] = (bytes >> 37) as u8;
        padded[123] = (bytes >> 29) as u8;
        padded[124] = (bytes >> 21) as u8;
        padded[125] = (bytes >> 13) as u8;
        padded[126] = (bytes >> 5) as u8;
        padded[127] = (bytes << 3) as u8;
        crypto_hashblocks_sha512(&mut state[..64], &padded[..128]);
    } else {
        for i in remaining_inlen + 1..247 {
            padded[i] = 0;
        }
        padded[247] = (bytes >> 61) as u8;
        padded[248] = (bytes >> 53) as u8;
        padded[249] = (bytes >> 45) as u8;
        padded[250] = (bytes >> 37) as u8;
        padded[251] = (bytes >> 29) as u8;
        padded[252] = (bytes >> 21) as u8;
        padded[253] = (bytes >> 13) as u8;
        padded[254] = (bytes >> 5) as u8;
        padded[255] = (bytes << 3) as u8;
        crypto_hashblocks_sha512(&mut state[..64], &padded[..256]);
    }

    out[..64].copy_from_slice(&state[..64]);
}

pub fn sha256(out: &mut [u8], input: &[u8]) {
    let mut state = [0u8; 40];
    sha256_inc_init(&mut state);
    sha256_inc_finalize(out, &mut state, input, input.len());
}

pub fn sha512(out: &mut [u8], input: &[u8]) {
    let mut state = [0u8; 72];
    sha512_inc_init(&mut state);
    sha512_inc_finalize(out, &mut state, input, input.len());
}

pub const SPX_SHA256_OUTPUT_BYTES: usize = 32;
pub const SPX_SHA512_OUTPUT_BYTES: usize = 64;

pub fn mgf1_256(out: &mut [u8], outlen: usize, input: &[u8], inlen: usize) {
    let mut inbuf = vec![0u8; inlen + 4];
    inbuf[..inlen].copy_from_slice(&input[..inlen]);
    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];
    let mut i: u64 = 0;
    let mut out_pos = 0usize;
    while ((i + 1) as usize) * SPX_SHA256_OUTPUT_BYTES <= outlen {
        crate::utils::u32_to_bytes(&mut inbuf[inlen..inlen + 4], i as u32);
        sha256(&mut out[out_pos..out_pos + SPX_SHA256_OUTPUT_BYTES], &inbuf);
        out_pos += SPX_SHA256_OUTPUT_BYTES;
        i += 1;
    }
    if outlen > (i as usize) * SPX_SHA256_OUTPUT_BYTES {
        crate::utils::u32_to_bytes(&mut inbuf[inlen..inlen + 4], i as u32);
        sha256(&mut outbuf, &inbuf);
        let leftover = outlen - (i as usize) * SPX_SHA256_OUTPUT_BYTES;
        out[out_pos..out_pos + leftover].copy_from_slice(&outbuf[..leftover]);
    }
}

pub fn mgf1_512(out: &mut [u8], outlen: usize, input: &[u8], inlen: usize) {
    let mut inbuf = vec![0u8; inlen + 4];
    inbuf[..inlen].copy_from_slice(&input[..inlen]);
    let mut outbuf = [0u8; SPX_SHA512_OUTPUT_BYTES];
    let mut i: u64 = 0;
    let mut out_pos = 0usize;
    while ((i + 1) as usize) * SPX_SHA512_OUTPUT_BYTES <= outlen {
        crate::utils::u32_to_bytes(&mut inbuf[inlen..inlen + 4], i as u32);
        sha512(&mut out[out_pos..out_pos + SPX_SHA512_OUTPUT_BYTES], &inbuf);
        out_pos += SPX_SHA512_OUTPUT_BYTES;
        i += 1;
    }
    if outlen > (i as usize) * SPX_SHA512_OUTPUT_BYTES {
        crate::utils::u32_to_bytes(&mut inbuf[inlen..inlen + 4], i as u32);
        sha512(&mut outbuf, &inbuf);
        let leftover = outlen - (i as usize) * SPX_SHA512_OUTPUT_BYTES;
        out[out_pos..out_pos + leftover].copy_from_slice(&outbuf[..leftover]);
    }
}
