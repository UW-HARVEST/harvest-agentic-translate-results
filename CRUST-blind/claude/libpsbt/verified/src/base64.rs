/// Base64 encoding/decoding tables and helpers, ported from the C
/// implementation in `c_src/base64.c` (RFC1341).

const BASE64_TABLE: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

const BASE62_TABLE: &[u8; 62] =
    b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

fn base_encode(src: &[u8], out: &mut [u8], table: &[u8]) -> Option<usize> {
    let len = src.len();

    // olen = len * 4 / 3 + 4 + 1 (extra byte for nul terminator), with overflow
    // checks matching the C implementation.
    let mul = len.checked_mul(4)?;
    let div = mul.checked_div(3)?;
    let olen = div.checked_add(5)?;

    if olen < len {
        return None;
    }
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

    let rem = len - i;
    if rem > 0 {
        out[pos] = table[(src[i] >> 2) as usize];
        pos += 1;
        if rem == 1 {
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
pub fn base62_encode(_src: &[u8], _out: &mut [u8]) -> Option<usize> {
    base_encode(_src, _out, BASE62_TABLE)
}

/// Encode the given source bytes into base64. Returns the number of written bytes on success.
pub fn base64_encode(_src: &[u8], _out: &mut [u8]) -> Option<usize> {
    base_encode(_src, _out, BASE64_TABLE)
}

/// Decode the given base64-encoded source bytes. Returns the number of decoded bytes on success.
pub fn base64_decode(_src: &[u8], _out: &mut [u8]) -> Option<usize> {
    let mut dtable = [0x80u8; 256];
    for (i, &c) in BASE64_TABLE.iter().enumerate() {
        dtable[c as usize] = i as u8;
    }
    dtable[b'=' as usize] = 0;

    // First pass: count valid base64 characters (and padding).
    let mut count = 0usize;
    for &c in _src {
        if dtable[c as usize] != 0x80 {
            count += 1;
        }
    }

    if count == 0 || count % 4 != 0 {
        return None;
    }

    let olen = count / 4 * 3;
    if _out.len() < olen {
        return None;
    }

    let mut pos = 0usize;
    let mut count = 0usize;
    let mut block = [0u8; 4];
    let mut pad = 0u32;

    for &c in _src {
        let tmp = dtable[c as usize];
        if tmp == 0x80 {
            continue;
        }

        if c == b'=' {
            pad += 1;
        }

        block[count] = tmp;
        count += 1;

        if count == 4 {
            _out[pos] = (block[0] << 2) | (block[1] >> 4);
            pos += 1;
            _out[pos] = (block[1] << 4) | (block[2] >> 2);
            pos += 1;
            _out[pos] = (block[2] << 6) | block[3];
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
