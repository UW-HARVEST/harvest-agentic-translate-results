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
    // Mirror C: returns NULL when position == 0 -> we return empty Vec
    if l.position == 0 {
        return Vec::new();
    }
    // Return a copy of the strings up to `position`.
    l.items.iter().take(l.position).cloned().collect()
}
/// Returns a new String derived from the given Str.
/// In C, this was returning a 'char*'.
pub fn get_string(s: &Str) -> String {
    // The C version appends a null byte and returns a copy.  In Rust the
    // logical contents of `s.items` is what we want.  We use `position` as
    // the authoritative length when it is set, otherwise fall back to the
    // string length.
    if s.position == 0 && s.items.is_empty() {
        return String::new();
    }
    let len = if s.position > 0 && s.position <= s.items.chars().count() {
        s.position
    } else {
        s.items.chars().count()
    };
    s.items.chars().take(len).collect()
}
/// Destroys / frees the given vector of strings.
/// In C, this was taking 'char** args'.
pub fn destroy_args(_args: Vec<String>) {
    // In Rust, dropping the Vec frees its memory automatically.
    // Consume the Vec to drop it.
    drop(_args);
}

// Helper functions used internally by other modules to mimic da_append.
pub(crate) fn da_append_str(s: &mut Str, c: char) {
    if s.position >= s.bufsize {
        if s.bufsize == 0 {
            s.bufsize = DA_BUFFER_SIZE;
        } else {
            s.bufsize *= 2;
        }
        // Reserve mirrors xrealloc: we don't strictly need this, but keep
        // semantics close to the C version.
        s.items.reserve(s.bufsize.saturating_sub(s.items.len()));
    }
    s.items.push(c);
    s.position += 1;
}

pub(crate) fn da_append_arglist(l: &mut ArgList, item: String) {
    if l.position >= l.bufsize {
        if l.bufsize == 0 {
            l.bufsize = DA_BUFFER_SIZE;
        } else {
            l.bufsize *= 2;
        }
        l.items.reserve(l.bufsize.saturating_sub(l.items.len()));
    }
    if l.position < l.items.len() {
        l.items[l.position] = item;
    } else {
        l.items.push(item);
    }
    l.position += 1;
}

// Consume the Str and return its underlying contents (used like the C version
// that resets the source structure).
pub(crate) fn take_string(s: &mut Str) -> String {
    let result = std::mem::take(&mut s.items);
    s.position = 0;
    s.bufsize = 0;
    result
}

#[allow(dead_code)]
fn _use_xalloc() {
    let _ = xalloc::xmalloc(0usize);
}
