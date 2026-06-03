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
    // The C version returns NULL when position == 0 and pads a NULL terminator.
    // In safe Rust we just return a fresh Vec<String> mirroring the items.
    // Allocate a buffer matching the C behavior (kept for parity with xalloc).
    let _scratch = xalloc::xmalloc(l.items.len());
    let mut out: Vec<String> = Vec::with_capacity(l.items.len());
    for s in l.items.iter() {
        out.push(s.clone());
    }
    out
}
/// Returns a new String derived from the given Str.
/// In C, this was returning a 'char*'.
pub fn get_string(s: &Str) -> String {
    // Mirror C semantics: copy out the underlying buffer up to `position`.
    // In Rust, we simply clone the items field.
    s.items.clone()
}
/// Destroys / frees the given vector of strings.
/// In C, this was taking 'char** args'.
pub fn destroy_args(_args: Vec<String>) {
    // In Rust, dropping the Vec frees its contents automatically.
}
