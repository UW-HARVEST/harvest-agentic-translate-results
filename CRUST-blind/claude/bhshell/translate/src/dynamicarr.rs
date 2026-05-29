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
    // Mirror the C behaviour: copy out the strings up to `position`.
    let mut args: Vec<String> = Vec::with_capacity(l.position);
    for i in 0..l.position {
        if i < l.items.len() {
            args.push(l.items[i].clone());
        }
    }
    args
}
/// Returns a new String derived from the given Str.
/// In C, this was returning a 'char*'.
pub fn get_string(s: &Str) -> String {
    // The C version appends a '\0' and then copies up to `position` bytes.
    // In Rust we don't need a NUL terminator, so we simply return a copy
    // of the contents accumulated so far.
    let mut out = String::with_capacity(s.position);
    let mut count = 0;
    for ch in s.items.chars() {
        if count >= s.position {
            break;
        }
        out.push(ch);
        count += 1;
    }
    out
}
/// Destroys / frees the given vector of strings.
/// In C, this was taking 'char** args'.
pub fn destroy_args(_args: Vec<String>) {
    // Rust automatically frees the Vec and its contents when dropped.
}

// Helper functions used internally to mirror the C `da_append` macro's behaviour.
impl Str {
    /// Append a single character to the dynamic string.
    pub fn append(&mut self, c: char) {
        if self.position >= self.bufsize {
            if self.bufsize == 0 {
                self.bufsize = DA_BUFFER_SIZE;
            } else {
                self.bufsize *= 2;
            }
        }
        // In Rust we only track a logical position; the underlying String
        // grows naturally on push.
        self.items.push(c);
        self.position += 1;
    }
}

impl ArgList {
    /// Append a string to the dynamic argument list.
    pub fn append(&mut self, s: String) {
        if self.position >= self.bufsize {
            if self.bufsize == 0 {
                self.bufsize = DA_BUFFER_SIZE;
            } else {
                self.bufsize *= 2;
            }
        }
        self.items.push(s);
        self.position += 1;
    }
}
