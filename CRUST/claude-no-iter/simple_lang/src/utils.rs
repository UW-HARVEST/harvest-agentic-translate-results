pub const SIMPLE_LANG_UTILS_H: bool = true;
// No external crate references needed
/// Replicates the C signature: char* strndup(const char* s, size_t n);
/// Returns a newly allocated substring of length n from s in a safe, idiomatic way.
pub fn strndup(s: &str, n: usize) -> String {
    let bytes = s.as_bytes();
    let len = bytes.len().min(n);
    // Safety: this assumes ASCII / valid UTF-8 boundary at `len`. The C source
    // operates on bytes, and our usage in the lexer always slices on ASCII
    // boundaries, so this is safe in practice.
    String::from_utf8_lossy(&bytes[..len]).into_owned()
}
