pub const SIMPLE_LANG_UTILS_H: bool = true;
// No external crate references needed
/// Replicates the C signature: char* strndup(const char* s, size_t n);
/// Returns a newly allocated substring of length n from s in a safe, idiomatic way.
pub fn strndup(s: &str, n: usize) -> String {
    let bytes = s.as_bytes();
    let len = if bytes.len() < n { bytes.len() } else { n };
    // Use the byte slice; assume input is ASCII (matches C strndup semantics).
    String::from_utf8_lossy(&bytes[..len]).into_owned()
}
