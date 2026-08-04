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

    l.items.iter().take(l.position).cloned().collect()
}
/// Returns a new String derived from the given Str.
/// In C, this was returning a 'char*'.
pub fn get_string(s: &Str) -> String {
    s.items.chars().take(s.position).collect()
}
/// Destroys / frees the given vector of strings.
/// In C, this was taking 'char** args'.
pub fn destroy_args(args: Vec<String>) {
    drop(args);
}
