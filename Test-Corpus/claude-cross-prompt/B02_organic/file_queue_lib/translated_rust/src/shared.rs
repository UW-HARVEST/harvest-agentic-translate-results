// Mirrors c_src/include/shared.h
//
// The C macros os_free / os_clearnl and helpers os_calloc / os_realloc /
// os_strdup are abstracted away in the Rust port using owned types
// (Option<String>, Vec<u8>) and slicing. They have no separate Rust API.

pub const OS_MAXSTR: usize = 1024;

/// Equivalent to: `if ((p = strrchr(x, '\n'))) *p = '\0';`
/// Removes a trailing newline, if any (only the last one, like C strrchr).
pub fn os_clearnl(s: &mut String) {
    if let Some(idx) = s.rfind('\n') {
        s.truncate(idx);
    }
}
