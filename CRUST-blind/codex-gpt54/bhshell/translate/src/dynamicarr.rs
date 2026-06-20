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
    let used = if l.position == 0 {
        l.items.len()
    } else {
        l.position.min(l.items.len())
    };
    l.items[..used].to_vec()
}
/// Returns a new String derived from the given Str.
/// In C, this was returning a 'char*'.
pub fn get_string(s: &Str) -> String {
    let used = if s.position == 0 {
        s.items.chars().count()
    } else {
        s.position.min(s.items.chars().count())
    };
    s.items.chars().take(used).collect()
}
/// Destroys / frees the given vector of strings.
/// In C, this was taking 'char** args'.
pub fn destroy_args(args: Vec<String>) {
    drop(args);
}
