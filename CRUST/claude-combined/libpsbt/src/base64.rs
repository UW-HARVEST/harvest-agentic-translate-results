/// Base64 encoding/decoding (RFC1341), translated from base64.c.
const BASE64_TABLE: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const BASE62_TABLE: &[u8; 62] =
    b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

fn base_encode(src: &[u8], out: &mut [u8], table: &[u8]) -> Option<usize> {
    let len = src.len();
    let olen = len.checked_mul(4)? / 3 + 4;
    let olen = olen + 1; // nul termination
    if olen < len {
        return None; // integer overflow
    }
    if olen > out.len() {
        return None; // buffer overflow
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

    if len - i > 0 {
        out[pos] = table[(src[i] >> 2) as usize];
        pos += 1;
        if len - i == 1 {
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

    if pos >= out.len() {
        return None;
    }
    out[pos] = 0;
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
    for (i, &b) in BASE64_TABLE.iter().enumerate() {
        dtable[b as usize] = i as u8;
    }
    dtable[b'=' as usize] = 0;

    let len = src.len();
    let mut count = 0usize;
    for &b in &src[..len] {
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
    let mut count = 0usize;
    let mut pad = 0i32;

    for i in 0..len {
        let tmp = dtable[src[i] as usize];
        if tmp == 0x80 {
            continue;
        }

        if src[i] == b'=' {
            pad += 1;
        }
        block[count] = tmp;
        count += 1;
        if count == 4 {
            if pos + 3 > out.len() {
                return None;
            }
            out[pos] = (block[0] << 2) | (block[1] >> 4);
            pos += 1;
            out[pos] = (block[1] << 4) | (block[2] >> 2);
            pos += 1;
            out[pos] = (block[2] << 6) | block[3];
            pos += 1;
            count = 0;
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
