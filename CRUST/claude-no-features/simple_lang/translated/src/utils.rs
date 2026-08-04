pub const SIMPLE_LANG_UTILS_H: bool = true;
// No external crate references needed
/// Replicates the C signature: char* strndup(const char* s, size_t n);
/// Returns a newly allocated substring of length n from s in a safe, idiomatic way.
pub fn strndup(s: &str, n: usize) -> String {
    let bytes = s.as_bytes();
    let len = std::cmp::min(bytes.len(), n);
    // Use byte slicing; the C semantics treat this as raw bytes.
    // If the input is valid UTF-8 and we slice on a char boundary, this is safe.
    // For safety with non-ASCII, fall back to lossy conversion.
    match std::str::from_utf8(&bytes[..len]) {
        Ok(s) => s.to_string(),
        Err(_) => String::from_utf8_lossy(&bytes[..len]).into_owned(),
    }
}
