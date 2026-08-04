// Base64/Base62 encoding/decoding, transpiled from c_src/base64.c.

const BASE64_TABLE: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

const BASE62_TABLE: &[u8; 62] =
    b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

/// Internal helper that performs base-N encoding using the given table.
/// Returns the number of bytes written to `out` (excluding the trailing NUL).
fn base_encode(src: &[u8], out: &mut [u8], table: &[u8]) -> Option<usize> {
    let len = src.len();
    // 3-byte blocks to 4-byte blocks, plus a NUL terminator
    let olen = len
        .checked_mul(4)?
        .checked_div(3)
        .and_then(|v| v.checked_add(4))?
        .checked_add(1)?;
    if olen > out.len() {
        return None;
    }

    let mut pos = 0usize;
    let mut i = 0usize;

    while len - i >= 3 {
        out[pos] = table[(src[i] >> 2) as usize];
        pos += 1;
        out[pos] = table[(((src[i] & 0x03) << 4) | (src[i + 1] >> 4)) as usize];
        pos += 1;
        out[pos] = table[(((src[i + 1] & 0x0f) << 2) | (src[i + 2] >> 6)) as usize];
        pos += 1;
        out[pos] = table[(src[i + 2] & 0x3f) as usize];
        pos += 1;
        i += 3;
    }

    let remaining = len - i;
    if remaining > 0 {
        out[pos] = table[(src[i] >> 2) as usize];
        pos += 1;
        if remaining == 1 {
            out[pos] = table[((src[i] & 0x03) << 4) as usize];
            pos += 1;
            out[pos] = b'=';
            pos += 1;
        } else {
            out[pos] = table[(((src[i] & 0x03) << 4) | (src[i + 1] >> 4)) as usize];
            pos += 1;
            out[pos] = table[((src[i + 1] & 0x0f) << 2) as usize];
            pos += 1;
        }
        out[pos] = b'=';
        pos += 1;
    }

    // NUL terminate
    if pos < out.len() {
        out[pos] = 0;
    }
    Some(pos)
}

/// Encode the given source bytes into base62. Returns the number of written bytes on success.
pub fn base62_encode(src: &[u8], out: &mut [u8]) -> Option<usize> {
    base_encode(src, out, BASE62_TABLE)
}

/// Encode the given source bytes into base64. Returns the number of written bytes on success.
pub fn base64_encode(src: &[u8], out: &mut [u8]) -> Option<usize> {
    base_encode(src, out, BASE64_TABLE)
}

/// Decode the given base64-encoded source bytes. Returns the number of decoded bytes on success.
pub fn base64_decode(src: &[u8], out: &mut [u8]) -> Option<usize> {
    let mut dtable = [0x80u8; 256];
    for (i, &c) in BASE64_TABLE.iter().enumerate() {
        dtable[c as usize] = i as u8;
    }
    dtable[b'=' as usize] = 0;

    // Count valid characters
    let mut count = 0usize;
    for &b in src.iter() {
        if dtable[b as usize] != 0x80 {
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

    let mut pos = 0usize;
    let mut block = [0u8; 4];
    let mut bcount = 0usize;
    let mut pad = 0usize;

    for &b in src.iter() {
        let tmp = dtable[b as usize];
        if tmp == 0x80 {
            continue;
        }
        if b == b'=' {
            pad += 1;
        }
        block[bcount] = tmp;
        bcount += 1;
        if bcount == 4 {
            out[pos] = (block[0] << 2) | (block[1] >> 4);
            pos += 1;
            out[pos] = (block[1] << 4) | (block[2] >> 2);
            pos += 1;
            out[pos] = (block[2] << 6) | block[3];
            pos += 1;
            bcount = 0;
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
