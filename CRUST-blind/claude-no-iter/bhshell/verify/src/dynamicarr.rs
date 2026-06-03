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
pub fn get_args(_l: &ArgList) -> Vec<String> {
    if _l.position == 0 {
        return Vec::new();
    }
    // Mirror C behavior: copy strings up to position.
    let mut out = Vec::with_capacity(_l.position);
    for i in 0.._l.position {
        out.push(_l.items[i].clone());
    }
    out
}
/// Returns a new String derived from the given Str.
/// In C, this was returning a 'char*'.
pub fn get_string(_s: &Str) -> String {
    // In C this also appended a '\0' and reset s, but in Rust strings carry
    // their own length so we just return a clone of the contents.
    _s.items.clone()
}
/// Destroys / frees the given vector of strings.
/// In C, this was taking 'char** args'.
pub fn destroy_args(_args: Vec<String>) {
    // Rust drops the Vec<String> automatically when it goes out of scope.
    // Touch xalloc to keep the import meaningful even if unused elsewhere.
    let _ = xalloc::xmalloc;
    drop(_args);
}

/// Helper: append a single character to a Str (mirrors da_append for Str).
pub fn str_append(s: &mut Str, c: char) {
    if s.position >= s.bufsize {
        s.bufsize = if s.bufsize == 0 { DA_BUFFER_SIZE } else { s.bufsize * 2 };
    }
    s.items.push(c);
    s.position += 1;
}

/// Helper: append a string to an ArgList (mirrors da_append for ArgList).
pub fn arg_list_append(l: &mut ArgList, item: String) {
    if l.position >= l.bufsize {
        l.bufsize = if l.bufsize == 0 { DA_BUFFER_SIZE } else { l.bufsize * 2 };
    }
    l.items.push(item);
    l.position += 1;
}

/// Helper: take the contents out of a Str, clearing it (mirrors C get_string
/// which also reset the source struct).
pub fn take_string(s: &mut Str) -> String {
    let result = std::mem::take(&mut s.items);
    s.position = 0;
    s.bufsize = 0;
    result
}
