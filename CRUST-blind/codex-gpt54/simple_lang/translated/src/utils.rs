pub const SIMPLE_LANG_UTILS_H: bool = true;
// No external crate references needed
/// Replicates the C signature: char* strndup(const char* s, size_t n);
/// Returns a newly allocated substring of length n from s in a safe, idiomatic way.
pub fn strndup(s: &str, n: usize) -> String {
    s.as_bytes()
        .get(..n.min(s.len()))
        .map(|slice| String::from_utf8_lossy(slice).into_owned())
        .unwrap_or_default()
}
