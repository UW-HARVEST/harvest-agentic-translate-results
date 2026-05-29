pub const SIMPLE_LANG_UTILS_H: bool = true;
// No external crate references needed
/// Replicates the C signature: char* strndup(const char* s, size_t n);
/// Returns a newly allocated substring of length n from s in a safe, idiomatic way.
pub fn strndup(s: &str, n: usize) -> String {
    let bytes = s.as_bytes();
    let len = bytes.len().min(n);
    // Safe because we are only iterating over byte indices of `s`, which is a valid &str.
    // To stay 100% safe and idiomatic for non-ASCII, we just collect chars by byte.
    String::from_utf8_lossy(&bytes[..len]).into_owned()
}
