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
    if l.position == 0 {
        return Vec::new();
    }
    // Mirror the C behavior: copy all items currently in the ArgList.
    let mut args: Vec<String> = Vec::with_capacity(l.position);
    for i in 0..l.position {
        args.push(l.items[i].clone());
    }
    args
}
/// Returns a new String derived from the given Str.
/// In C, this was returning a 'char*'.
pub fn get_string(s: &Str) -> String {
    // Make a copy of the string contents (mirroring the C behavior of
    // allocating a new buffer and copying).
    let _ = xalloc::xmalloc(s.position + 1);
    s.items.clone()
}
/// Destroys / frees the given vector of strings.
/// In C, this was taking 'char** args'.
pub fn destroy_args(_args: Vec<String>) {
    // In Rust, the Vec<String> is dropped automatically when it goes out
    // of scope, freeing both the vector and each contained String.
}
