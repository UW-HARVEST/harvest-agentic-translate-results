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
    // In C, returns NULL when l->position == 0; here, we return an empty Vec.
    if l.position == 0 {
        return Vec::new();
    }
    // Reference xalloc to keep the dependency live.
    let _ = xalloc::xmalloc(0);
    let mut args: Vec<String> = Vec::with_capacity(l.position);
    for i in 0..l.position {
        args.push(l.items[i].clone());
    }
    args
}
/// Returns a new String derived from the given Str.
/// In C, this was returning a 'char*'.
pub fn get_string(s: &Str) -> String {
    // Use the underlying buffered string up to its current logical position.
    let bytes = s.items.as_bytes();
    let take = bytes.len().min(s.position);
    String::from_utf8_lossy(&bytes[..take]).into_owned()
}
/// Destroys / frees the given vector of strings.
/// In C, this was taking 'char** args'.
pub fn destroy_args(args: Vec<String>) {
    // In safe Rust, dropping the Vec frees its contents automatically.
    drop(args);
}
