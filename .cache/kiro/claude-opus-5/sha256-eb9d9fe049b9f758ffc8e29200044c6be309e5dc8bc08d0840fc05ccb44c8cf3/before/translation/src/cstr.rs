//! Helpers reproducing C string semantics for the fixed-size `char[]` fields
//! used by `item_t` and `order_t`.

/// `strncpy(dst, src, N - 1); dst[N - 1] = '\0';`
///
/// Copies at most `N - 1` bytes, zero-fills the remainder, and guarantees the
/// final byte is NUL. Source bytes past the limit are silently dropped, exactly
/// like the C original.
pub fn cstr_from<const N: usize>(src: &str) -> [u8; N] {
    let mut buf = [0u8; N];
    let bytes = src.as_bytes();
    let n = if bytes.len() < N - 1 {
        bytes.len()
    } else {
        N - 1
    };
    buf[..n].copy_from_slice(&bytes[..n]);
    buf
}

/// Length up to the first NUL, i.e. `strlen`.
fn cstr_len(buf: &[u8]) -> usize {
    match buf.iter().position(|&b| b == 0) {
        Some(i) => i,
        None => buf.len(),
    }
}

/// The bytes `printf("%s", buf)` would emit: everything before the first NUL.
pub fn cstr_bytes(buf: &[u8]) -> &[u8] {
    &buf[..cstr_len(buf)]
}

/// `%s` rendering of a fixed-size buffer.
///
/// All strings in this program are ASCII literals baked into the source, so the
/// contents are always valid UTF-8; the lossy fallback keeps this total.
pub fn cstr_str(buf: &[u8]) -> std::borrow::Cow<'_, str> {
    String::from_utf8_lossy(cstr_bytes(buf))
}

/// `strcmp(buf, other) == 0`
pub fn cstr_eq(buf: &[u8], other: &str) -> bool {
    cstr_bytes(buf) == other.as_bytes()
}
