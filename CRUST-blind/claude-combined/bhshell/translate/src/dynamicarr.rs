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

/// Append a character to a Str, mirroring the C `da_append` macro.
pub fn da_append_char(s: &mut Str, c: char) {
    if s.position >= s.bufsize {
        if s.bufsize == 0 {
            s.bufsize = DA_BUFFER_SIZE;
        } else {
            s.bufsize *= 2;
        }
        // Grow the buffer to bufsize bytes.
        // We use a Vec<u8> internally then convert to String at the end.
        // For simplicity, we just push.
    }
    s.items.push(c);
    s.position += 1;
}

/// Append a String to an ArgList, mirroring the C `da_append` macro.
pub fn da_append_arg(l: &mut ArgList, item: String) {
    if l.position >= l.bufsize {
        if l.bufsize == 0 {
            l.bufsize = DA_BUFFER_SIZE;
        } else {
            l.bufsize *= 2;
        }
    }
    if l.position < l.items.len() {
        l.items[l.position] = item;
    } else {
        l.items.push(item);
    }
    l.position += 1;
}

/// Returns a new vector of strings derived from the given ArgList.
/// In C, this was returning a 'char**'. The C version NULL-terminated.
/// In safe Rust we return a Vec<String> WITHOUT a NULL marker — length encodes that.
pub fn get_args(l: &ArgList) -> Vec<String> {
    if l.position == 0 {
        return Vec::new();
    }
    let take = l.position.min(l.items.len());
    l.items[..take].to_vec()
}
/// Returns a new String derived from the given Str.
/// In C, this was returning a 'char*'. The C version included a trailing '\0'
/// in its memcpy length, but the resulting C string is just `s.items`.
pub fn get_string(s: &Str) -> String {
    // The Str struct stores the running text in `items`. Just clone what's there.
    let bytes = s.items.as_bytes();
    let take = bytes.len().min(s.position);
    match std::str::from_utf8(&bytes[..take]) {
        Ok(t) => t.to_string(),
        Err(_) => String::from_utf8_lossy(&bytes[..take]).into_owned(),
    }
}
/// Destroys / frees the given vector of strings.
/// In C, this was taking 'char** args'.
pub fn destroy_args(_args: Vec<String>) {
    // In Rust, dropping is automatic when args goes out of scope here.
    // Reference xalloc to keep the import meaningful.
    let _ = xalloc::xmalloc(0);
}
