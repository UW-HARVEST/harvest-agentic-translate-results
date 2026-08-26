// The original C source contains only a library function (custom_strdup)
// with no main entry point. A program built from just this translation
// unit performs no I/O, so the Rust executable mirrors that behavior:
// it produces no output and exits successfully.

mod lib_impl {
    pub fn custom_strdup(s: Option<&str>) -> Option<String> {
        match s {
            None => None,
            Some(v) => Some(v.to_string()),
        }
    }
}

fn main() {
    // Reference the function so it isn't dead-code-eliminated as a warning.
    let _ = lib_impl::custom_strdup(None);
}
