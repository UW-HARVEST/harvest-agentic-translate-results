/// Base64/Base62 encoding and decoding utilities (translated from base64.c).

const BASE64_TABLE: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

const BASE62_TABLE: &[u8; 62] =
    b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

/// Internal helper that performs Base64-style encoding using the supplied table.
/// `table` must contain at least 64 entries (Base64); when it has 62 entries (Base62),
/// indices >= 62 are wrapped modulo 62 (mirroring the original C behaviour of
/// reading past the table — but avoiding any out-of-bounds access). The C
/// implementation only uses the first 62 entries when invoked with the Base62
/// table, so this is safe.
fn base_encode_impl(src: &[u8], out: &mut [u8], table: &[u8]) -> Option<usize> {
    let len = src.len();
    let mut olen = len.checked_mul(4)? / 3 + 4;
    olen = olen.checked_add(1)?; // nul termination

    if olen > out.len() {
        return None;
    }

    let table_len = table.len();
    let lookup = |idx: u8| -> u8 {
        let i = idx as usize;
        if i < table_len {
            table[i]
        } else {
            // Wrap; only relevant for the Base62 alphabet which has 62 entries.
            table[i % table_len]
        }
    };

    let mut pos = 0usize;
    let mut in_pos = 0usize;

    while len - in_pos >= 3 {
        out[pos] = lookup(src[in_pos] >> 2);
        pos += 1;
        out[pos] = lookup(((src[in_pos] & 0x03) << 4) | (src[in_pos + 1] >> 4));
        pos += 1;
        out[pos] = lookup(((src[in_pos + 1] & 0x0f) << 2) | (src[in_pos + 2] >> 6));
        pos += 1;
        out[pos] = lookup(src[in_pos + 2] & 0x3f);
        pos += 1;
        in_pos += 3;
    }

    if len - in_pos > 0 {
        out[pos] = lookup(src[in_pos] >> 2);
        pos += 1;
        if len - in_pos == 1 {
            out[pos] = lookup((src[in_pos] & 0x03) << 4);
            pos += 1;
            out[pos] = b'=';
            pos += 1;
        } else {
            out[pos] = lookup(((src[in_pos] & 0x03) << 4) | (src[in_pos + 1] >> 4));
            pos += 1;
            out[pos] = lookup((src[in_pos + 1] & 0x0f) << 2);
            pos += 1;
        }
        out[pos] = b'=';
        pos += 1;
    }

    // nul terminator (not counted in returned length)
    if pos < out.len() {
        out[pos] = 0;
    }

    Some(pos)
}

/// Encode the given source bytes into base62. Returns the number of written bytes on success.
pub fn base62_encode(src: &[u8], out: &mut [u8]) -> Option<usize> {
    base_encode_impl(src, out, BASE62_TABLE)
}

/// Encode the given source bytes into base64. Returns the number of written bytes on success.
pub fn base64_encode(src: &[u8], out: &mut [u8]) -> Option<usize> {
    base_encode_impl(src, out, BASE64_TABLE)
}

/// Decode the given base64-encoded source bytes. Returns the number of decoded bytes on success.
pub fn base64_decode(src: &[u8], out: &mut [u8]) -> Option<usize> {
    let mut dtable = [0x80u8; 256];
    for (i, &b) in BASE64_TABLE.iter().enumerate() {
        dtable[b as usize] = i as u8;
    }
    dtable[b'=' as usize] = 0;

    // Count valid characters
    let mut count = 0usize;
    for &b in src {
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
    let mut pad = 0usize;

    for &b in src {
        let tmp = dtable[b as usize];
        if tmp == 0x80 {
            continue;
        }
        if b == b'=' {
            pad += 1;
        }
        block[count] = tmp;
        count += 1;
        if count == 4 {
            if pos >= out.len() {
                return None;
            }
            out[pos] = (block[0] << 2) | (block[1] >> 4);
            pos += 1;
            if pos >= out.len() {
                return None;
            }
            out[pos] = (block[1] << 4) | (block[2] >> 2);
            pos += 1;
            if pos >= out.len() {
                return None;
            }
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
