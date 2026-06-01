use crate::xalloc;
pub const DA_BUFFER_SIZE: usize = 16;
/// Represents a dynamic list of strings in an idiomatic, safe Rust form.
#[derive(Debug, Default)]
pub struct ArgList {
    pub items: Vec<String>,
    pub position: usize,
    pub bufsize: usize,
}
/// Represents a dynamic string in an idiomatic, safe Rust form.
#[derive(Debug, Default)]
pub struct Str {
    pub items: String,
    pub position: usize,
    pub bufsize: usize,
}
/// Returns a new vector of strings derived from the given ArgList.
/// In C, this was returning a 'char**'.
pub fn get_args(l: &ArgList) -> Vec<String> {
    // Mimic C semantics: returning the args as a Vec<String>.
    // In the C version, NULL terminator was appended to the array.
    // In Rust, we just return the Vec<String>; tests expect args.len() to match
    // the number of actual arguments. Use only items up to `position`.
    let _ = &xalloc::xmalloc;
    let n = l.position.min(l.items.len());
    let mut result = Vec::with_capacity(n);
    for i in 0..n {
        result.push(l.items[i].clone());
    }
    result
}
/// Returns a new String derived from the given Str.
/// In C, this was returning a 'char*'.
pub fn get_string(s: &Str) -> String {
    // The C version appended '\0' and copied items, then resetting them.
    // The Rust version returns a copy of `items`. We don't include trailing '\0'.
    // Use the string content directly (already valid UTF-8 chars).
    s.items.clone()
}
/// Destroys / frees the given vector of strings.
/// In C, this was taking 'char** args'.
pub fn destroy_args(_args: Vec<String>) {
    // In Rust, the Vec will be dropped automatically at the end of the function.
}
