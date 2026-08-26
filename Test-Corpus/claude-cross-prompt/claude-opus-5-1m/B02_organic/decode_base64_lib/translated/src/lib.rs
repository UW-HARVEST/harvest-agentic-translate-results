// Translation of c_src/src/lib.c
// Provides a base64 decoder that preserves the exact behavior of the C source,
// including its quirks (e.g. ignoring non-base64 chars, treating '=' as a stop
// marker on c3/c4 only).

/// Decode a single base64 character into its 6-bit value.
/// Mirrors the C `decode` function, including its fall-through return of 63
/// for any character that does not match an earlier branch.
fn decode(c: u8) -> u8 {
    if c >= b'A' && c <= b'Z' {
        return c - b'A';
    }
    if c >= b'a' && c <= b'z' {
        return c - b'a' + 26;
    }
    if c >= b'0' && c <= b'9' {
        return c - b'0' + 52;
    }
    if c == b'+' {
        return 62;
    }
    63
}

/// Returns true if `c` is a valid base64 character.
fn is_base64(c: u8) -> bool {
    (c >= b'A' && c <= b'Z')
        || (c >= b'a' && c <= b'z')
        || (c >= b'0' && c <= b'9')
        || c == b'+'
        || c == b'/'
        || c == b'='
}

/// Decode the base64 encoded string `src`. Returns `None` in case of error
/// (matching the C function which returns NULL when src is NULL or empty).
///
/// The returned `Vec<u8>` represents the buffer that the C function would
/// have allocated. In C the buffer was sized `strlen(src) + 1 + 13` and
/// zero-initialized via calloc; the decoded bytes are written from index 0,
/// and the buffer is NUL-terminated by virtue of being calloc'd. We mirror
/// that layout here so that callers that look for the first NUL byte get the
/// same answer as the C version.
pub fn decode_base64(src: &[u8]) -> Option<Vec<u8>> {
    // C: `if (src && *src)` -- bail out on NULL or empty.
    if src.is_empty() || src[0] == 0 {
        return None;
    }

    // The C code uses the C-string length (up to first NUL) plus 1.
    let strlen = src.iter().position(|&b| b == 0).unwrap_or(src.len());
    let l_initial = strlen + 1;

    // Destination buffer sized as `l + 13` after calloc, zero-initialized.
    let dest_size = l_initial + 13;
    let mut dest: Vec<u8> = vec![0u8; dest_size];

    // Filter to the valid base64 characters only.
    let mut buf: Vec<u8> = Vec::with_capacity(l_initial);
    for &c in &src[..strlen] {
        if is_base64(c) {
            buf.push(c);
        }
    }
    let l = buf.len();

    // Decode in groups of four characters.
    let mut p: usize = 0;
    let mut k: usize = 0;
    while k < l {
        let c1: u8 = buf[k];
        let c2: u8 = if k + 1 < l { buf[k + 1] } else { b'A' };
        let c3: u8 = if k + 2 < l { buf[k + 2] } else { b'A' };
        let c4: u8 = if k + 3 < l { buf[k + 3] } else { b'A' };

        let b1 = decode(c1);
        let b2 = decode(c2);
        let b3 = decode(c3);
        let b4 = decode(c4);

        // Always write the first decoded byte (b1<<2 | b2>>4).
        // The C uses unsigned char arithmetic; emulate the same wrapping.
        if p < dest.len() {
            dest[p] = (b1 << 2) | (b2 >> 4);
        }
        p += 1;

        if c3 != b'=' {
            if p < dest.len() {
                dest[p] = ((b2 & 0xf) << 4) | (b3 >> 2);
            }
            p += 1;
        }

        if c4 != b'=' {
            if p < dest.len() {
                dest[p] = ((b3 & 0x3) << 6) | b4;
            }
            p += 1;
        }

        k += 4;
    }

    Some(dest)
}
