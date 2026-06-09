// Translation of c_src/src/lib.c. The original C source is a library that
// exposes a single function (`encode_base64`) and has no `main`, so the
// produced executable does nothing observable on stdout/stderr — matching
// the C build, which only emits a shared library.

fn encode(u: u8) -> u8 {
    if u < 26 {
        b'A' + u
    } else if u < 52 {
        b'a' + (u - 26)
    } else if u < 62 {
        b'0' + (u - 52)
    } else if u == 62 {
        b'+'
    } else {
        b'/'
    }
}

/// Base64 encode `size` bytes of `src`. Returns the encoded string,
/// or `None` mirroring the C function returning NULL.
///
/// Mirrors the original C implementation's quirks:
///   * If `size` is 0, the length of `src` is used (computed as a C string,
///     i.e. up to the first NUL byte).
///   * The output buffer is sized `size * 4 / 3 + 4` bytes (zero filled),
///     matching the C `calloc` allocation.
#[allow(dead_code)]
pub fn encode_base64(mut size: i32, src: Option<&[u8]>) -> Option<Vec<u8>> {
    let src = src?;

    if size == 0 {
        // Equivalent to strlen((char *)src) — count up to first NUL byte.
        size = src.iter().position(|&b| b == 0).unwrap_or(src.len()) as i32;
    }

    let cap: usize = (size as usize) * 4 / 3 + 4;
    let mut out: Vec<u8> = vec![0u8; cap];
    let mut pos: usize = 0;

    let size_usize = size as usize;
    let mut i: usize = 0;
    while i < size_usize {
        let b1: u8;
        let mut b2: u8 = 0;
        let mut b3: u8 = 0;

        b1 = src[i];

        if i + 1 < size_usize {
            b2 = src[i + 1];
        }

        if i + 2 < size_usize {
            b3 = src[i + 2];
        }

        let b4: u8 = b1 >> 2;
        let b5: u8 = ((b1 & 0x3) << 4) | (b2 >> 4);
        let b6: u8 = ((b2 & 0xf) << 2) | (b3 >> 6);
        let b7: u8 = b3 & 0x3f;

        out[pos] = encode(b4);
        pos += 1;
        out[pos] = encode(b5);
        pos += 1;

        if i + 1 < size_usize {
            out[pos] = encode(b6);
            pos += 1;
        } else {
            out[pos] = b'=';
            pos += 1;
        }

        if i + 2 < size_usize {
            out[pos] = encode(b7);
            pos += 1;
        } else {
            out[pos] = b'=';
            pos += 1;
        }

        i += 3;
    }

    let _ = pos; // silence unused warning if optimized
    Some(out)
}

fn main() {
    // The C source has no `main` — it is compiled as a shared library only.
    // Produce no output to remain byte-identical.
}
