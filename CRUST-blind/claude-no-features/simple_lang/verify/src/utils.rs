pub const SIMPLE_LANG_UTILS_H: bool = true;
// No external crate references needed
/// Replicates the C signature: char* strndup(const char* s, size_t n);
/// Returns a newly allocated substring of length n from s in a safe, idiomatic way.
pub fn strndup(s: &str, n: usize) -> String {
    let bytes = s.as_bytes();
    let len = if bytes.len() > n { n } else { bytes.len() };
    // Safely take up to n bytes; assume input is ASCII (matches C usage).
    String::from_utf8_lossy(&bytes[..len]).into_owned()
}
