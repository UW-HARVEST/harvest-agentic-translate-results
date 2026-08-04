// Base64 / Base62 encoding/decoding (RFC1341-style).

static BASE64_TABLE: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

static BASE62_TABLE: &[u8; 62] =
    b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

/// Common encoder used by both base64 and base62 encoders. Returns Some(written)
/// where `written` is the number of bytes written (excluding any nul terminator),
/// or None on overflow / insufficient capacity.
fn base_encode(src: &[u8], out: &mut [u8], table: &[u8]) -> Option<usize> {
    let len = src.len();
    let out_capacity = out.len();

    // 3-byte blocks to 4-byte
    let olen_data = len
        .checked_mul(4)?
        .checked_div(3)
        .unwrap_or(0)
        .checked_add(4)?;
    // nul termination
    let olen = olen_data.checked_add(1)?;
    if olen < len {
        return None;
    }
    if olen > out_capacity {
        return None;
    }

    let mut pos: usize = 0;
    let mut i: usize = 0;
    while len.saturating_sub(i) >= 3 {
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

    let remaining = len - i;
    if remaining > 0 {
        let b0 = src[i];
        out[pos] = table[(b0 >> 2) as usize];
        pos += 1;
        if remaining == 1 {
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

    // nul terminator (caller has space for it because olen included it)
    if pos < out_capacity {
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

    let len = src.len();
    let out_capacity = out.len();

    let mut count: usize = 0;
    for i in 0..len {
        if dtable[src[i] as usize] != 0x80 {
            count += 1;
        }
    }

    if count == 0 || count % 4 != 0 {
        return None;
    }

    let olen = count / 4 * 3;
    if out_capacity < olen {
        return None;
    }

    let mut pos: usize = 0;
    let mut count: usize = 0;
    let mut block = [0u8; 4];
    let mut pad: i32 = 0;

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
            out[pos] = (block[0] << 2) | (block[1] >> 4);
            pos += 1;
            out[pos] = (block[1] << 4) | (block[2] >> 2);
            pos += 1;
            out[pos] = (block[2] << 6) | block[3];
            pos += 1;
            count = 0;
            if pad != 0 {
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
