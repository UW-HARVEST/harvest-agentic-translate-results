const BASE64_TABLE: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const BASE62_TABLE: &[u8; 62] =
    b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

fn base_encode(src: &[u8], out: &mut [u8], table: &[u8]) -> Option<usize> {
    let olen = src.len().checked_mul(4)?.checked_div(3)?.checked_add(5)?;
    if olen < src.len() || olen > out.len() {
        return None;
    }

    let mut in_pos = 0usize;
    let mut out_pos = 0usize;

    while src.len().saturating_sub(in_pos) >= 3 {
        out[out_pos] = table[(src[in_pos] >> 2) as usize];
        out[out_pos + 1] = table[(((src[in_pos] & 0x03) << 4) | (src[in_pos + 1] >> 4)) as usize];
        out[out_pos + 2] =
            table[(((src[in_pos + 1] & 0x0f) << 2) | (src[in_pos + 2] >> 6)) as usize];
        out[out_pos + 3] = table[(src[in_pos + 2] & 0x3f) as usize];
        in_pos += 3;
        out_pos += 4;
    }

    match src.len().saturating_sub(in_pos) {
        0 => {}
        1 => {
            out[out_pos] = table[(src[in_pos] >> 2) as usize];
            out[out_pos + 1] = table[((src[in_pos] & 0x03) << 4) as usize];
            out[out_pos + 2] = b'=';
            out[out_pos + 3] = b'=';
            out_pos += 4;
        }
        2 => {
            out[out_pos] = table[(src[in_pos] >> 2) as usize];
            out[out_pos + 1] = table[(((src[in_pos] & 0x03) << 4) | (src[in_pos + 1] >> 4)) as usize];
            out[out_pos + 2] = table[((src[in_pos + 1] & 0x0f) << 2) as usize];
            out[out_pos + 3] = b'=';
            out_pos += 4;
        }
        _ => return None,
    }

    out[out_pos] = 0;
    Some(out_pos)
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
    for (i, ch) in BASE64_TABLE.iter().enumerate() {
        dtable[*ch as usize] = i as u8;
    }
    dtable[b'=' as usize] = 0;

    let count = src
        .iter()
        .filter(|&&ch| dtable[ch as usize] != 0x80)
        .count();

    if count == 0 || count % 4 != 0 {
        return None;
    }

    let olen = count / 4 * 3;
    if out.len() < olen {
        return None;
    }

    let mut block = [0u8; 4];
    let mut block_len = 0usize;
    let mut out_pos = 0usize;
    let mut pad = 0usize;

    for &ch in src {
        let tmp = dtable[ch as usize];
        if tmp == 0x80 {
            continue;
        }

        if ch == b'=' {
            pad += 1;
        }

        block[block_len] = tmp;
        block_len += 1;

        if block_len == 4 {
            out[out_pos] = (block[0] << 2) | (block[1] >> 4);
            out[out_pos + 1] = (block[1] << 4) | (block[2] >> 2);
            out[out_pos + 2] = (block[2] << 6) | block[3];
            out_pos += 3;
            block_len = 0;

            if pad != 0 {
                match pad {
                    1 => out_pos -= 1,
                    2 => out_pos -= 2,
                    _ => return None,
                }
                break;
            }
        }
    }

    Some(out_pos)
}
