// Base64/Base62 encoding and base64 decoding implementations.
// Translated from C base64.c.

const BASE64_TABLE: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const BASE62_TABLE: &[u8; 62] =
    b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

fn base_encode(src: &[u8], out: &mut [u8], table: &[u8]) -> Option<usize> {
    let len = src.len();
    // olen = len * 4 / 3 + 4 + 1 (nul)
    let olen = len.checked_mul(4)? / 3 + 4 + 1;
    if olen < len {
        return None; // integer overflow (cannot really happen in usize but matches C)
    }
    if olen > out.len() {
        return None;
    }

    let mut pos: usize = 0;
    let mut i: usize = 0;

    while len - i >= 3 {
        let b0 = src[i];
        let b1 = src[i + 1];
        let b2 = src[i + 2];
        out[pos] = table[(b0 >> 2) as usize];
        pos += 1;
        out[pos] = table[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize];
        pos += 1;
        out[pos] = table[(((b1 & 0x0f) << 2) | (b2 >> 6)) as usize];
        pos += 1;
        out[pos] = table[(b2 & 0x3f) as usize];
        pos += 1;
        i += 3;
    }

    if len - i > 0 {
        let b0 = src[i];
        out[pos] = table[(b0 >> 2) as usize];
        pos += 1;
        if len - i == 1 {
            out[pos] = table[((b0 & 0x03) << 4) as usize];
            pos += 1;
            out[pos] = b'=';
            pos += 1;
        } else {
            let b1 = src[i + 1];
            out[pos] = table[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize];
            pos += 1;
            out[pos] = table[((b1 & 0x0f) << 2) as usize];
            pos += 1;
        }
        out[pos] = b'=';
        pos += 1;
    }

    // nul terminator
    if pos < out.len() {
        out[pos] = 0;
    }

    Some(pos)
}

pub fn base62_encode(src: &[u8], out: &mut [u8]) -> Option<usize> {
    base_encode(src, out, BASE62_TABLE)
}

pub fn base64_encode(src: &[u8], out: &mut [u8]) -> Option<usize> {
    base_encode(src, out, BASE64_TABLE)
}

pub fn base64_decode(src: &[u8], out: &mut [u8]) -> Option<usize> {
    let mut dtable = [0x80u8; 256];
    for (i, &c) in BASE64_TABLE.iter().enumerate() {
        dtable[c as usize] = i as u8;
    }
    dtable[b'=' as usize] = 0;

    let mut count: usize = 0;
    for &c in src.iter() {
        if dtable[c as usize] != 0x80 {
            count += 1;
        }
    }

    if count == 0 || count % 4 != 0 {
        return None;
    }

    let olen = count / 4 * 3;
    if out.len() < olen {
        return None;
    }

    let mut pos: usize = 0;
    let mut block = [0u8; 4];
    let mut block_count: usize = 0;
    let mut pad: usize = 0;

    for &c in src.iter() {
        let tmp = dtable[c as usize];
        if tmp == 0x80 {
            continue;
        }

        if c == b'=' {
            pad += 1;
        }
        block[block_count] = tmp;
        block_count += 1;

        if block_count == 4 {
            out[pos] = (block[0] << 2) | (block[1] >> 4);
            pos += 1;
            out[pos] = (block[1] << 4) | (block[2] >> 2);
            pos += 1;
            out[pos] = (block[2] << 6) | block[3];
            pos += 1;
            block_count = 0;
            if pad > 0 {
                if pad == 1 {
                    pos -= 1;
                } else if pad == 2 {
                    pos -= 2;
                } else {
                    return None;
                }
                break;
            }
        }
    }

    Some(pos)
}
